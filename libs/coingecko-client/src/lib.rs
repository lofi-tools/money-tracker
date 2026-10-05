use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use models::AssetPricePoint;
use payloads::{CurrentPriceReq, CurrentPriceResp};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use utils::prelude::*;

const USER_AGENT: &str = concat!(
    "money-tracker/",
    env!("CARGO_PKG_VERSION"),
    " (local portfolio tracker; https://github.com/lofi-tools/money-tracker)"
);

pub struct CoingeckoClient {
    pub base_url: String,
    pub http_client: reqwest::Client,
    rate: Arc<RateLimiter>,
}
impl IsApiClient for CoingeckoClient {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}
impl CoingeckoClient {
    pub fn new() -> anyhow::Result<Self> {
        Self::new_with_config(RateLimitConfig::default())
    }

    pub fn new_with_config(rate_config: RateLimitConfig) -> anyhow::Result<Self> {
        let (base_url, api_key) = if let Ok(key) = std::env::var("COINGECKO_PRO_API_KEY") {
            (
                "https://pro-api.coingecko.com/api/v3/",
                Some(("x-cg-pro-api-key", key)),
            )
        } else if let Ok(key) = std::env::var("COINGECKO_DEMO_API_KEY") {
            (
                "https://api.coingecko.com/api/v3/",
                Some(("x-cg-demo-api-key", key)),
            )
        } else {
            ("https://api.coingecko.com/api/v3/", None)
        };
        Ok(CoingeckoClient {
            base_url: base_url.to_string(),
            http_client: Self::build_http_client(api_key)?,
            rate: Arc::new(RateLimiter::new(rate_config)),
        })
    }

    #[cfg(test)]
    fn with_rate_for_tests(rate_config: RateLimitConfig) -> Self {
        CoingeckoClient {
            base_url: "https://api.coingecko.com/api/v3/".to_string(),
            http_client: reqwest::Client::new(),
            rate: Arc::new(RateLimiter::new(rate_config)),
        }
    }

    fn build_http_client(
        api_key: Option<(&'static str, String)>,
    ) -> anyhow::Result<reqwest::Client> {
        Ok(reqwest::Client::builder()
            .default_headers(Self::default_headers(api_key)?)
            .build()?)
    }

    fn default_headers(
        api_key: Option<(&'static str, String)>,
    ) -> anyhow::Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(USER_AGENT),
        );
        if let Some((name, key)) = api_key {
            headers.insert(name, reqwest::header::HeaderValue::from_str(&key)?);
        }
        Ok(headers)
    }

    pub async fn fetch_current_prices(
        &self,
        args: CurrentPriceReq,
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        let resp = self
            .get_json::<CurrentPriceResp>("/simple/price", &args)
            .await?;

        let mut prices_out = Vec::new();
        for (asset_id, prices) in resp.0 {
            for (vs_asset_id, price) in prices {
                prices_out.push(AssetPricePoint {
                    asset_id: asset_id.clone(),
                    vs_asset_id,
                    price,
                    time: Utc::now(),
                })
            }
        }

        Ok(prices_out)
    }

    /// CoinGecko's market chart range endpoint returns [Unix milliseconds, price].
    pub async fn fetch_historical_prices(
        &self,
        coin_id: &str,
        vs_currency: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        anyhow::ensure!(from < to, "price range must have from < to");
        let path = format!("/coins/{coin_id}/market_chart/range");
        #[derive(serde::Serialize)]
        struct RangeQuery {
            vs_currency: String,
            from: String,
            to: String,
        }
        let query = RangeQuery {
            vs_currency: vs_currency.to_string(),
            from: from.timestamp().to_string(),
            to: to.timestamp().to_string(),
        };
        let response = self
            .get_json::<payloads::HistoricalPriceResp>(&path, &query)
            .await?;
        response
            .prices
            .into_iter()
            .map(|(millis, price)| {
                anyhow::ensure!(price.is_finite() && price >= 0.0, "invalid CoinGecko price");
                let time = DateTime::from_timestamp_millis(millis)
                    .ok_or_else(|| anyhow::anyhow!("invalid CoinGecko timestamp {millis}"))?;
                Ok(AssetPricePoint {
                    asset_id: coin_id.to_string(),
                    vs_asset_id: vs_currency.to_string(),
                    price,
                    time,
                })
            })
            .collect()
    }

    /// GET + deserialize with client-side backpressure and 429/5xx retries.
    async fn get_json<D: DeserializeOwned>(
        &self,
        path: &str,
        query: &impl serde::Serialize,
    ) -> anyhow::Result<D> {
        let mut attempt: u32 = 0;
        loop {
            self.rate.acquire().await;
            let request = self
                .get(path)
                .query(query)
                .timeout(Duration::from_secs(30));
            let (client, req) = request.build_split();
            let req = req?;
            let method = req.method().clone();
            let url = req.url().clone();
            let resp = client.execute(req).await?;
            let status = resp.status();
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_retry_after);
            let body = resp.text().await?;

            if status.is_success() {
                let parsed: D = serde_json::from_str(&body).map_err(|e| {
                    anyhow::anyhow!("failed deserializing CoinGecko response from {url}: {e}\nbody: {body}")
                })?;
                return Ok(parsed);
            }

            let retryable = status.as_u16() == 429 || status.is_server_error();
            if retryable && attempt < self.rate.max_retries {
                let backoff = retry_after.unwrap_or_else(|| self.rate.backoff_for(attempt));
                self.rate.note_throttled();
                tokio::time::sleep(backoff).await;
                attempt += 1;
                continue;
            }
            anyhow::bail!("{method} {url} \nReceived {status} error response: {body}");
        }
    }
}

/// Client-side backpressure: sliding-window call tracker + min interval
/// between calls, plus exponential backoff when CoinGecko answers 429/5xx.
#[derive(Debug, Default)]
pub struct RateLimitConfig {
    pub min_interval: Option<Duration>,
    pub max_per_minute: Option<usize>,
    pub max_retries: Option<u32>,
    pub base_backoff: Option<Duration>,
}

struct RateLimiter {
    min_interval: Duration,
    max_per_minute: usize,
    max_retries: u32,
    base_backoff: Duration,
    state: Mutex<RateState>,
}

struct RateState {
    calls: VecDeque<Instant>,
    last_start: Option<Instant>,
    /// Extra delay added after a 429 so we slow down before retrying.
    penalty: Duration,
}

impl RateLimiter {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            min_interval: config.min_interval.unwrap_or(Duration::from_secs(8)),
            max_per_minute: config.max_per_minute.unwrap_or(7),
            max_retries: config.max_retries.unwrap_or(5),
            base_backoff: config.base_backoff.unwrap_or(Duration::from_secs(2)),
            state: Mutex::new(RateState {
                calls: VecDeque::new(),
                last_start: None,
                penalty: Duration::ZERO,
            }),
        }
    }

    async fn acquire(&self) {
        loop {
            let wait = {
                let mut state = self.state.lock().expect("rate state poisoned");
                let now = Instant::now();
                while state.calls.front().is_some_and(|t| now - *t > Duration::from_secs(60)) {
                    state.calls.pop_front();
                }
                let mut wait = Duration::ZERO;
                if state.calls.len() >= self.max_per_minute
                    && let Some(oldest) = state.calls.front()
                {
                    wait = wait.max(((*oldest + Duration::from_secs(60)) - now) + state.penalty);
                }
                if let Some(last) = state.last_start
                    && let Some(next) = last
                        .checked_add(self.min_interval)
                        .and_then(|t| t.checked_add(state.penalty))
                    && next > now
                {
                    wait = wait.max(next - now);
                }
                if wait.is_zero() {
                    state.calls.push_back(Instant::now());
                    state.last_start = Some(Instant::now());
                    // Decay penalty on successful admission so we speed back up.
                    state.penalty = state.penalty.checked_sub(Duration::from_millis(500)).unwrap_or(Duration::ZERO);
                    return;
                }
                wait
            };
            tokio::time::sleep(wait).await;
        }
    }

    fn note_throttled(&self) {
        let mut state = self.state.lock().expect("rate state poisoned");
        // Grow the spacing/window delay so subsequent acquire() calls
        // back off before we even hit the limit again (max +30s).
        state.penalty = (state.penalty + Duration::from_secs(5)).min(Duration::from_secs(30));
    }

    fn backoff_for(&self, attempt: u32) -> Duration {
        let exp = self.base_backoff.checked_mul(2_u32.pow(attempt)).unwrap_or(Duration::from_secs(60));
        (exp + Duration::from_millis((attempt as u64 * 137) % 500)).min(Duration::from_secs(60))
    }
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(secs) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(secs.min(120)));
    }
    // HTTP-date form: wait until that time.
    if let Ok(date) = chrono::DateTime::parse_from_rfc2822(value.trim())
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(value.trim()))
    {
        let delta = date.timestamp() - chrono::Utc::now().timestamp();
        if delta > 0 {
            return Some(Duration::from_secs(delta.min(120) as u64));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_is_configured_for_anonymous_demo_and_pro_clients() -> anyhow::Result<()> {
        let _ = CoingeckoClient::with_rate_for_tests(RateLimitConfig::default());
        for api_key in [
            None,
            Some(("x-cg-demo-api-key", "demo-key".to_string())),
            Some(("x-cg-pro-api-key", "pro-key".to_string())),
        ] {
            let headers = CoingeckoClient::default_headers(api_key.clone())?;
            assert_eq!(
                headers.get(reqwest::header::USER_AGENT),
                Some(&reqwest::header::HeaderValue::from_static(USER_AGENT))
            );
            if let Some((name, key)) = api_key {
                assert_eq!(headers.get(name).unwrap().to_str()?, key);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, thiserror::Error, Debug)]
#[error("Coingecko api error response: {0:?}")]
#[serde(transparent)]
pub struct CoingeckoErrResp(serde_json::Value);

pub mod payloads {
    use self::local_utils::Or;
    use super::local_utils::ser_joined_str;
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize)]
    pub struct CurrentPriceReq {
        #[serde(serialize_with = "ser_joined_str")]
        ids: Vec<String>,
        #[serde(rename = "vs_currencies", serialize_with = "ser_joined_str")]
        vs_assets: Vec<String>,
    }
    impl Default for CurrentPriceReq {
        fn default() -> Self {
            CurrentPriceReq {
                ids: vec!["ethereum".to_string()], // TODO all coingecko assets
                vs_assets: vec!["usd".to_string()],
            }
        }
    }
    impl CurrentPriceReq {
        pub fn new(ids: Vec<String>, vs_assets: Vec<String>) -> Self {
            Self { ids, vs_assets }
        }

        pub fn or_default(self) -> Self {
            Self {
                ids: self.ids.or(Self::default().ids),
                vs_assets: self.vs_assets.or(Self::default().vs_assets),
            }
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct CurrentPriceResp(pub HashMap<String, HashMap<String, f64>>);

    #[derive(Deserialize, Debug)]
    pub struct HistoricalPriceResp {
        pub prices: Vec<(i64, f64)>,
    }
}

pub mod models {
    use chrono::{DateTime, Utc};

    pub struct AssetPricePoint {
        pub asset_id: String,
        pub vs_asset_id: String,
        pub price: f64,
        pub time: DateTime<Utc>,
    }
}

pub mod local_utils {
    use serde::Serializer;

    pub fn ser_joined_str<S>(v: &[String], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let joined_str = v.join(",");
        serializer.serialize_str(&joined_str)
    }

    pub trait Or {
        fn or(self, default: Self) -> Self;
    }
    impl<T> Or for Vec<T> {
        fn or(self, default: Self) -> Self {
            if self.is_empty() {
                return default;
            }
            self
        }
    }
}
