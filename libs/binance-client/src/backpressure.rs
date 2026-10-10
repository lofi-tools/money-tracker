//! One request budget across processes, independent of URL and credentials.
mod persistence;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::{sync::Mutex as AsyncMutex, time::Instant};

pub(crate) const MAX_ATTEMPTS: usize = 8;
const INITIAL_INTERVAL: Duration = Duration::from_millis(250);
const WINDOW: Duration = Duration::from_secs(60);
fn headroom(ceiling: u32) -> u32 {
    (u64::from(ceiling) * 4 / 5) as u32
}

// Official seeds and endpoint weights (checked 2026-10-10):
// https://developers.binance.com/en/docs/products/wallet/general-info
// https://developers.binance.com/en/docs/catalog/investment-and-services-fiat/api/rest-api/~
// https://developers.binance.com/en/docs/catalog/core-trading-convert/api/rest-api/trade
// https://developers.binance.com/en/docs/catalog/core-trading-wallet/api/rest-api/capital
// https://binance.github.io/binance-connector-js/classes/_binance_simple-earn.SimpleEarnRestAPI.FlexibleLockedApi.html
fn profile(path: &str) -> (u32, u32) {
    match path {
        "/api/v3/exchangeInfo" => (20, 6000),
        "/api/v3/account" | "/api/v3/myTrades" => (20, 6000),
        p if p.starts_with("/api/") => (20, 6000),
        "/sapi/v1/fiat/orders" => (45000, 180000),
        "/sapi/v1/capital/withdraw/history" => (18000, 180000),
        "/sapi/v1/convert/tradeFlow" => (3000, 180000),
        "/sapi/v1/capital/deposit/hisrec"
        | "/sapi/v1/asset/transfer"
        | "/sapi/v1/fiat/payments" => (1, 12000),
        "/sapi/v1/asset/assetDividend" => (10, 12000),
        // Earn history/position costs 150 IP weight. Unknown read endpoints use
        // this conservative estimate until observations refine the pacing.
        _ => (150, 12000),
    }
}
fn key(path: &str) -> &str {
    if path.starts_with("/api/") {
        "spot"
    } else {
        path
    }
}

struct Budget {
    ceiling: u32,
    effective: u32,
    calls: VecDeque<(Instant, u32)>,
    total: u64,
    remote: Option<(Instant, u32, u64)>,
    successes: usize,
}
impl Budget {
    fn new(ceiling: u32) -> Self {
        Self {
            ceiling,
            effective: headroom(ceiling),
            calls: VecDeque::new(),
            total: 0,
            remote: None,
            successes: 0,
        }
    }
    fn prune(&mut self, now: Instant) {
        while self
            .calls
            .front()
            .is_some_and(|(at, _)| now.duration_since(*at) >= WINDOW)
        {
            self.calls.pop_front();
        }
        if self
            .remote
            .is_some_and(|(at, _, _)| now.duration_since(at) >= WINDOW)
        {
            self.remote = None;
        }
    }
    fn usage(&self) -> u64 {
        let local: u64 = self.calls.iter().map(|(_, cost)| u64::from(*cost)).sum();
        let remote = self
            .remote
            .map(|(_, used, total)| {
                u64::from(used).saturating_add(self.total.saturating_sub(total))
            })
            .unwrap_or(0);
        local.max(remote)
    }
    fn ready_at(&mut self, cost: u32, now: Instant) -> Instant {
        self.prune(now);
        if self.usage().saturating_add(u64::from(cost)) <= u64::from(self.effective.max(cost)) {
            return now;
        }
        // Recheck at each expiry; a remote observation includes other callers
        // sharing the IP/account and remains conservative for a full minute.
        self.calls
            .front()
            .map(|(at, _)| *at + WINDOW)
            .into_iter()
            .chain(self.remote.map(|(at, _, _)| at + WINDOW))
            .min()
            .unwrap_or(now)
    }
}

pub(crate) struct Pacer {
    pub next_request: Instant,
    interval: Duration,
    budgets: HashMap<String, Budget>,
    active: Option<(String, u32)>,
    directory: Option<PathBuf>,
}

pub(crate) fn shared() -> Arc<AsyncMutex<Pacer>> {
    static PACER: OnceLock<Arc<AsyncMutex<Pacer>>> = OnceLock::new();
    PACER
        .get_or_init(|| {
            Arc::new(AsyncMutex::new(Pacer::persistent(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.cache/binance/limiter"),
            )))
        })
        .clone()
}
impl Pacer {
    pub(crate) fn new() -> Self {
        Self {
            next_request: Instant::now(),
            interval: INITIAL_INTERVAL,
            budgets: HashMap::new(),
            active: None,
            directory: None,
        }
    }
    pub(crate) fn persistent(directory: PathBuf) -> Self {
        Self {
            directory: Some(directory),
            ..Self::new()
        }
    }
    fn budget(&mut self, path: &str) -> &mut Budget {
        self.budgets
            .entry(key(path).to_owned())
            .or_insert_with(|| Budget::new(profile(path).1))
    }
    pub async fn wait(&mut self, path: &str) {
        loop {
            let now = Instant::now();
            let until = self
                .budget(path)
                .ready_at(profile(path).0, now)
                .max(self.next_request);
            if until <= now {
                return;
            }
            tokio::time::sleep_until(until).await;
        }
    }
    pub fn start_request(&mut self, path: &str) {
        let now = Instant::now();
        let cost = profile(path).0;
        let budget = self.budget(path);
        budget.calls.push_back((now, cost));
        budget.total = budget.total.saturating_add(u64::from(cost));
        let spacing = WINDOW.mul_f64(f64::from(cost) / f64::from(budget.effective.max(cost)));
        self.next_request = now + self.interval.max(spacing);
        self.active = Some((key(path).to_owned(), cost));
    }
    pub fn observe_headers(&mut self, path: &str, headers: &reqwest::header::HeaderMap) {
        for (header, bucket) in [
            ("x-mbx-used-weight-1m", "spot"),
            ("x-sapi-used-ip-weight-1m", key(path)),
            ("x-sapi-used-uid-weight-1m", key(path)),
        ] {
            if let Some(used) = headers
                .get(header)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
            {
                let budget = self.budgets.entry(bucket.to_owned()).or_insert_with(|| {
                    Budget::new(if bucket == "spot" {
                        6000
                    } else {
                        profile(path).1
                    })
                });
                budget.remote = Some((Instant::now(), used, budget.total));
            }
        }
    }
    pub fn observe_exchange_info(&mut self, value: &serde_json::Value) {
        if let Some(limits) = value["rateLimits"].as_array() {
            for limit in limits {
                if limit["rateLimitType"] == "REQUEST_WEIGHT"
                    && limit["interval"] == "MINUTE"
                    && limit["intervalNum"] == 1
                {
                    if let Some(ceiling) = limit["limit"]
                        .as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .filter(|v| *v >= 20)
                    {
                        let budget = self
                            .budgets
                            .entry("spot".into())
                            .or_insert_with(|| Budget::new(ceiling));
                        // Updating a documented ceiling must not erase a learned reduction.
                        budget.effective = ((u64::from(budget.effective) * u64::from(ceiling))
                            / u64::from(budget.ceiling))
                        .max(20)
                        .min(u64::from(ceiling)) as u32;
                        budget.ceiling = ceiling;
                    }
                }
            }
        }
    }
    pub fn cool_down(&mut self, delay: Duration) -> Option<()> {
        let until = Instant::now().checked_add(delay)?;
        self.next_request = self.next_request.max(until);
        self.interval = (self.interval * 2)
            .max(delay / 16)
            .min(Duration::from_secs(30));
        if let Some((key, cost)) = &self.active {
            let budget = self.budgets.get_mut(key)?;
            // Use the observed load at rejection as an upper bound when it is
            // informative; otherwise halve the previous estimated budget.
            let observed = u32::try_from(budget.usage()).unwrap_or(u32::MAX);
            let bound = if observed >= cost.saturating_mul(2) {
                observed.saturating_mul(4) / 5
            } else {
                budget.effective / 2
            };
            budget.effective = (budget.effective / 2).min(bound).max(*cost);
            budget.successes = 0;
        }
        Some(())
    }
    pub fn success(&mut self) {
        if let Some((key, _)) = &self.active {
            let budget = self
                .budgets
                .get_mut(key)
                .expect("active request has budget");
            budget.successes += 1;
            if budget.successes >= 32 {
                budget.effective = budget
                    .effective
                    .saturating_add((budget.ceiling / 100).max(1))
                    .min(headroom(budget.ceiling));
                self.interval = (self.interval.mul_f64(0.9)).max(INITIAL_INTERVAL);
                budget.successes = 0;
            }
        }
    }
}

pub(crate) fn retry_delay(
    header: Option<&str>,
    attempt: usize,
    now: chrono::DateTime<chrono::Utc>,
) -> Duration {
    // Never shorten a valid server cooldown. Support HTTP dates as well as seconds.
    let server = header.and_then(|value| {
        value
            .trim()
            .parse::<u64>()
            .ok()
            .map(Duration::from_secs)
            .or_else(|| {
                chrono::DateTime::parse_from_rfc2822(value)
                    .ok()
                    .map(|date| {
                        (date.with_timezone(&chrono::Utc) - now)
                            .to_std()
                            .unwrap_or(Duration::ZERO)
                    })
            })
    });
    server.filter(|delay| !delay.is_zero()).unwrap_or_else(|| {
        Duration::from_secs((30u64.saturating_mul(1u64 << attempt.min(4))).min(300))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_is_not_capped_and_fallback_grows() {
        let now = chrono::DateTime::from_timestamp(0, 0).unwrap();
        assert_eq!(retry_delay(Some("120"), 0, now), Duration::from_secs(120));
        assert_eq!(
            retry_delay(Some("Thu, 01 Jan 1970 00:02:00 GMT"), 0, now),
            Duration::from_secs(120)
        );
        assert_eq!(retry_delay(None, 0, now), Duration::from_secs(30));
        assert_eq!(
            retry_delay(Some("invalid"), 1, now),
            Duration::from_secs(60)
        );
        assert_eq!(retry_delay(None, 7, now), Duration::from_secs(300));
    }

    #[test]
    fn constructors_share_budget_across_urls_and_credentials() -> anyhow::Result<()> {
        let a = crate::BinanceClient::with_credentials(
            "https://api.binance.com",
            "one".into(),
            "secret-one".into(),
        )?;
        let b = crate::BinanceClient::with_credentials(
            "https://api1.binance.com/sapi/v1",
            "two".into(),
            "secret-two".into(),
        )?;
        assert!(Arc::ptr_eq(&a.pacer, &b.pacer));
        assert!(Arc::ptr_eq(&a.pacer, &shared()));
        Ok(())
    }

    #[tokio::test(start_paused = true)]
    async fn weighted_history_prevents_fiat_bursts_and_expires() {
        let mut pacer = Pacer::new();
        let path = "/sapi/v1/fiat/orders";
        for _ in 0..3 {
            pacer.wait(path).await;
            pacer.start_request(path);
            pacer.success();
        }
        // 45,000 weight each: the fourth exceeds our 144,000/min budget.
        let before = Instant::now();
        pacer.wait(path).await;
        assert!(Instant::now() - before >= Duration::from_millis(22500));
        assert_eq!(pacer.budget(path).calls.len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn server_usage_and_exchange_limits_constrain_next_request() {
        let mut pacer = Pacer::new();
        let path = "/api/v3/account";
        pacer.start_request(path);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-mbx-used-weight-1m", "4790".parse().unwrap());
        pacer.observe_headers(path, &headers);
        let before = Instant::now();
        pacer.wait(path).await;
        assert!(Instant::now() - before >= WINDOW);
        pacer.observe_exchange_info(&serde_json::json!({"rateLimits":[{"rateLimitType":"REQUEST_WEIGHT","interval":"MINUTE","intervalNum":1,"limit":1200}]}));
        assert_eq!(pacer.budget(path).effective, 960);
    }

    #[tokio::test(start_paused = true)]
    async fn rejection_learns_lower_budget_and_success_recovers_gradually() {
        let mut pacer = Pacer::new();
        let path = "/sapi/v1/simple-earn/flexible/history/rewardsRecord";
        pacer.start_request(path);
        let original = pacer.budget(path).effective;
        pacer.cool_down(Duration::from_secs(120)).unwrap();
        assert_eq!(pacer.budget(path).effective, original / 2);
        for _ in 0..32 {
            pacer.success();
        }
        assert!(pacer.budget(path).effective > original / 2);
        assert!(pacer.budget(path).effective < original);
        assert!(pacer.next_request >= Instant::now() + Duration::from_secs(120));
    }

    #[tokio::test(start_paused = true)]
    async fn shared_cooldown_survives_cancellation() {
        let a = Arc::new(AsyncMutex::new(Pacer::new()));
        a.lock().await.cool_down(Duration::from_secs(120)).unwrap();
        let b = a.clone();
        let task = tokio::spawn(async move {
            b.lock().await.wait("/api/v3/account").await;
        });
        tokio::time::advance(Duration::from_secs(119)).await;
        assert!(!task.is_finished());
        task.abort();
        assert!(a.lock().await.next_request > Instant::now());
    }
}
