//! Versioned untrusted browser facts, conservative global matching, local ingestion.
use crate::{
    Application,
    adapters::nexo::{NEXO_LEDGER_SOURCE, allocations},
};
use anyhow::{Result, ensure};
use chrono::{DateTime, NaiveDate, Utc};
use lib_core::{
    AccountId, PortfolioId,
    ui_metadata::{AnnualRate, AnnualRateBasis, RateEvidence, RateScope, SourceEvidence},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_BYTES: usize = 2_000_000;
pub const ORIGINS: &[&str] = &["https://platform.nexo.com"];
pub fn sha256(bytes: &[u8]) -> String {
    hmac_sha256::Hash::hash(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Principal {
    pub asset: String,
    pub amount: String,
    pub precision: String,
    pub exactness: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DateFact {
    pub value: String,
    pub precision: String,
    pub timezone: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rate {
    pub percent: String,
    pub basis: String,
    pub raw_label: String,
    pub scope: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Payout {
    pub asset: Option<String>,
    pub scope: String,
    pub historical_continuity: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Term {
    pub observation_id: String,
    pub provider_term_id: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub status: String,
    pub principal: Option<Principal>,
    pub rate: Option<Rate>,
    pub payout: Option<Payout>,
    pub start: Option<DateFact>,
    pub maturity: Option<DateFact>,
    pub duration_months: Option<u32>,
    pub auto_renew: Option<bool>,
    pub renew_with_interest: Option<bool>,
    pub accrued_rewards: Vec<serde_json::Value>,
    pub evidence: serde_json::Value,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub status: String,
    pub reported_active_count: Option<usize>,
    pub views_captured: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub schema: String,
    pub version: u32,
    pub capture_id: String,
    pub capture_started_at: DateTime<Utc>,
    pub capture_finished_at: DateTime<Utc>,
    pub extractor_version: String,
    pub origin: String,
    pub page_path: String,
    pub locale: String,
    pub browser_timezone: String,
    pub account_hint: serde_json::Value,
    pub account_settings: serde_json::Value,
    pub coverage: Coverage,
    pub terms: Vec<Term>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Match {
    pub observation_id: String,
    pub subscription_id: Option<String>,
    pub candidates: Vec<String>,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub capture_id: String,
    pub content_hash: String,
    pub matched: usize,
    pub unresolved: usize,
    pub conflicts: usize,
    pub matches: Vec<Match>,
    pub duplicate: bool,
}
fn bounded(s: &str) -> bool {
    !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
}
fn decimal(s: &str) -> Result<rust_decimal::Decimal> {
    ensure!(
        s.len() <= 100 && !s.contains(['e', 'E', ',', '+']),
        "invalid decimal"
    );
    Ok(rust_decimal::Decimal::from_str_exact(s)?)
}
impl Capture {
    pub fn parse(payload: &str) -> Result<Self> {
        ensure!(payload.len() <= MAX_BYTES, "capture too large");
        let c: Self = serde_json::from_str(payload)?;
        ensure!(
            c.schema == "money-tracker.nexo-terms" && c.version == 1,
            "unsupported capture schema"
        );
        ensure!(
            ORIGINS.contains(&c.origin.as_str())
                && c.page_path.starts_with('/')
                && !c.page_path.contains(['?', '#']),
            "invalid capture origin/path"
        );
        ensure!(
            bounded(&c.capture_id) && bounded(&c.extractor_version) && c.terms.len() <= 1000,
            "invalid capture identity/count"
        );
        ensure!(
            c.capture_started_at <= c.capture_finished_at
                && c.capture_finished_at <= Utc::now() + chrono::Duration::minutes(5),
            "invalid capture time"
        );
        ensure!(
            ["partial", "complete"].contains(&c.coverage.status.as_str()),
            "invalid coverage"
        );
        if c.coverage.status == "complete" {
            ensure!(
                c.coverage.reported_active_count == Some(c.terms.len()),
                "complete capture count mismatch"
            );
        }
        let mut seen = BTreeSet::new();
        for t in &c.terms {
            ensure!(
                bounded(&t.observation_id) && seen.insert(&t.observation_id),
                "duplicate observation id"
            );
            ensure!(
                t.observed_at >= c.capture_started_at && t.observed_at <= c.capture_finished_at,
                "term time outside capture"
            );
            ensure!(
                t.status == "active" && t.provider_term_id.as_ref().is_none_or(|s| bounded(s)),
                "invalid term identity/status"
            );
            if let Some(p) = &t.principal {
                ensure!(
                    bounded(&p.asset)
                        && decimal(&p.amount)?.is_sign_positive()
                        && !decimal(&p.amount)?.is_zero(),
                    "invalid principal"
                );
                ensure!(
                    ["exact", "unknown"].contains(&p.exactness.as_str()),
                    "invalid precision"
                );
            }
            for date in [&t.start, &t.maturity].into_iter().flatten() {
                match date.precision.as_str() {
                    "date" => {
                        NaiveDate::parse_from_str(&date.value, "%Y-%m-%d")?;
                    }
                    "instant" => {
                        DateTime::parse_from_rfc3339(&date.value)?;
                    }
                    _ => anyhow::bail!("unsupported date precision"),
                }
            }
            if let Some(r) = &t.rate {
                let n = decimal(&r.percent)?;
                ensure!(
                    n >= rust_decimal::Decimal::ZERO
                        && n <= rust_decimal::Decimal::from(100)
                        && ["apy", "apr", "unknown"].contains(&r.basis.as_str())
                        && r.scope == "term",
                    "invalid rate"
                );
            }
        }
        Ok(c)
    }
}
fn same_start(date: &DateFact, start: DateTime<Utc>) -> bool {
    match date.precision.as_str() {
        "instant" => DateTime::parse_from_rfc3339(&date.value).is_ok_and(|d| d == start),
        "date" if date.timezone.as_deref() == Some("UTC") => {
            date.value == start.format("%Y-%m-%d").to_string()
        }
        _ => false,
    }
}
pub fn match_terms(c: &Capture, parts: &[allocations::TermAllocation]) -> Vec<Match> {
    let strong: Vec<_> = c
        .terms
        .iter()
        .map(|t| {
            t.principal.as_ref().is_some_and(|p| p.exactness == "exact")
                && t.start.as_ref().is_some_and(|d| {
                    d.precision == "instant"
                        || (d.precision == "date" && d.timezone.as_deref() == Some("UTC"))
                })
        })
        .collect();
    let mut result: Vec<Match> = c
        .terms
        .iter()
        .map(|t| {
            let mut reasons = vec![];
            if t.principal.is_none() {
                reasons.push("Original principal not displayed");
            }
            if t.principal.as_ref().is_some_and(|p| p.exactness != "exact") {
                reasons.push("Displayed principal precision is unverified");
            }
            if t.start.is_none() {
                reasons.push("Start date not displayed");
            }
            if t.start
                .as_ref()
                .is_some_and(|d| d.precision == "date" && d.timezone.as_deref() != Some("UTC"))
            {
                reasons.push("Start date timezone is unverified");
            }
            let candidates: Vec<_> = parts
                .iter()
                .filter(|p| p.kind == "fixed")
                .filter(|p| {
                    t.principal.as_ref().is_some_and(|v| {
                        v.asset == p.quantity.asset_id.0
                            && (v.exactness != "exact"
                                || decimal(&v.amount).ok()
                                    == decimal(&p.quantity.units.decimal_string(p.quantity.scale))
                                        .ok())
                    }) && t.start.as_ref().is_none_or(|d| {
                        if d.precision == "date" && d.timezone.as_deref() != Some("UTC") {
                            true
                        } else {
                            p.opened_at.is_some_and(|start| same_start(d, start))
                        }
                    })
                })
                .map(|p| p.id.clone())
                .collect();
            if candidates.is_empty() {
                reasons.push("No supported unique active CSV opening");
            }
            if reasons.is_empty() {
                reasons.push("Competing exact transaction candidates");
            }
            Match {
                observation_id: t.observation_id.clone(),
                subscription_id: None,
                candidates,
                reason: reasons.join("; "),
            }
        })
        .collect();
    let mut uses = BTreeMap::new();
    for m in &result {
        for id in &m.candidates {
            *uses.entry(id.clone()).or_insert(0) += 1;
        }
    }
    let mut provider_counts = BTreeMap::new();
    for t in &c.terms {
        if let Some(id) = &t.provider_term_id {
            *provider_counts.entry(id).or_insert(0) += 1;
        }
    }
    for ((m, t), strong) in result.iter_mut().zip(&c.terms).zip(strong) {
        if strong
            && m.candidates.len() == 1
            && uses[&m.candidates[0]] == 1
            && t.provider_term_id
                .as_ref()
                .is_none_or(|id| provider_counts[id] == 1)
        {
            m.subscription_id = Some(m.candidates[0].clone());
            m.reason = "Unique exact principal and source start; one-to-one".into();
        }
    }
    result
}

impl Application {
    pub fn ingest_nexo_capture(
        &self,
        portfolio: &PortfolioId,
        account: &AccountId,
        payload: &str,
    ) -> Result<Receipt> {
        let c = Capture::parse(payload)?;
        let hash = sha256(payload.as_bytes());
        ensure!(
            self.store()
                .accounts(portfolio)?
                .iter()
                .any(|a| &a.id == account && a.provider_id.0 == "nexo"),
            "paired Nexo account missing"
        );
        for (old_hash, old_payload, result) in self.store().nexo_captures(account)? {
            let old = Capture::parse(&old_payload)?;
            if old_hash == hash {
                let mut r: Receipt = serde_json::from_str(&result)?;
                r.duplicate = true;
                return Ok(r);
            }
            ensure!(
                old.capture_id != c.capture_id,
                "capture id reused with different payload"
            );
        }
        let mut sources = vec![];
        for (hash, bytes) in self
            .store()
            .legacy_source_snapshots(account, NEXO_LEDGER_SOURCE)?
        {
            let rows = nexo_csv::NexoCsv::from_reader(bytes.as_bytes())?;
            if let Some(at) = rows
                .iter()
                .map(|r| r.date_time_utc)
                .max()
                .filter(|at| *at <= c.capture_finished_at)
            {
                sources.push((at, hash, rows));
            }
        }
        let parts = if let Some((at, hash, rows)) = sources
            .into_iter()
            .max_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)))
        {
            allocations::open_allocations(&rows, account, &hash, at).unwrap_or_default()
        } else {
            vec![]
        };
        let mut matches = match_terms(&c, &parts);
        // Never rebind an external provider identity to another transaction silently.
        let mut old_bindings = BTreeMap::<String, String>::new();
        for (_, bytes, result) in self.store().nexo_captures(account)? {
            let old = Capture::parse(&bytes)?;
            let report: Receipt = serde_json::from_str(&result)?;
            for m in report.matches {
                if let Some(sub) = m.subscription_id {
                    if let Some(id) = old
                        .terms
                        .iter()
                        .find(|t| t.observation_id == m.observation_id)
                        .and_then(|t| t.provider_term_id.clone())
                    {
                        old_bindings.insert(id, sub);
                    }
                }
            }
        }
        let mut rates = vec![];
        let mut conflicts = 0;
        for (m, t) in matches.iter_mut().zip(&c.terms) {
            if let Some(sub) = &m.subscription_id {
                if t.provider_term_id
                    .as_ref()
                    .and_then(|id| old_bindings.get(id))
                    .is_some_and(|old| old != sub)
                {
                    m.subscription_id = None;
                    m.reason = "provider identity conflicts with an earlier binding".into();
                    conflicts += 1;
                    continue;
                }
                let Some(subscription) = self.store().subscription(sub)? else {
                    m.subscription_id = None;
                    m.reason = "local subscription metadata missing".into();
                    continue;
                };
                if let Some(r) = &t.rate {
                    if r.basis != "unknown" {
                        let asset = subscription.base_asset.clone();
                        let scopes = [
                            RateScope::Subscription(sub.clone()),
                            RateScope::Product(subscription.product_id),
                            RateScope::AccountAsset {
                                account: account.clone(),
                                asset: asset.clone(),
                            },
                            RateScope::Provider("nexo".into()),
                            RateScope::Asset(asset.clone()),
                            RateScope::ProviderAsset {
                                provider: "nexo".into(),
                                asset,
                            },
                        ];
                        let mut pinned = false;
                        for scope in scopes {
                            if self
                                .store()
                                .selected_rate(portfolio, &scope, t.observed_at)?
                                .is_some_and(|r| r.pinned)
                            {
                                pinned = true;
                            }
                        }
                        if pinned {
                            conflicts += 1;
                            m.reason
                                .push_str("; manual pin retained, captured rate not applied");
                            continue;
                        }
                        rates.push(RateEvidence {
                            id: format!("nexo-browser:{hash}:{}", t.observation_id),
                            portfolio_id: portfolio.clone(),
                            scope: RateScope::Subscription(sub.clone()),
                            effective_at: t.observed_at,
                            rate: Some(AnnualRate {
                                fraction: (decimal(&r.percent)? / rust_decimal::Decimal::from(100))
                                    .normalize()
                                    .to_string()
                                    .parse()?,
                                basis: if r.basis == "apy" {
                                    AnnualRateBasis::Apy
                                } else {
                                    AnnualRateBasis::Apr
                                },
                                compounds_per_year: None,
                                reward_asset: t
                                    .payout
                                    .as_ref()
                                    .and_then(|p| p.asset.as_deref())
                                    .map(lib_core::AssetId::str),
                            }),
                            evidence: SourceEvidence {
                                source: "nexo-browser".into(),
                                reference: format!("capture:{hash}:{}", t.observation_id),
                                observed_at: t.observed_at,
                                updated_at: t.observed_at,
                            },
                        });
                    }
                }
            }
        }
        let report = Receipt {
            capture_id: c.capture_id.clone(),
            content_hash: hash.clone(),
            matched: matches
                .iter()
                .filter(|m| m.subscription_id.is_some())
                .count(),
            unresolved: matches
                .iter()
                .filter(|m| m.subscription_id.is_none())
                .count(),
            conflicts,
            matches,
            duplicate: false,
        };
        ensure!(
            serde_json::to_vec(&report)?.len() < 900_000,
            "capture result exceeds acknowledgement limit"
        );
        self.store().commit_nexo_capture(
            portfolio,
            account,
            &hash,
            &c.capture_id,
            c.capture_finished_at,
            payload,
            &serde_json::to_string(&report)?,
            &rates,
        )?;
        Ok(report)
    }
    pub fn captured_nexo_term(
        &self,
        account: &AccountId,
        subscription: &str,
        cutoff: DateTime<Utc>,
    ) -> Result<Option<Term>> {
        let mut result = None;
        for (_, payload, receipt) in self.store().nexo_captures(account)? {
            let c = Capture::parse(&payload)?;
            if c.capture_finished_at > cutoff {
                continue;
            }
            let receipt: Receipt = serde_json::from_str(&receipt)?;
            for m in receipt.matches {
                if m.subscription_id.as_deref() == Some(subscription) {
                    if let Some(t) = c
                        .terms
                        .iter()
                        .find(|t| t.observation_id == m.observation_id)
                    {
                        result = Some(t.clone());
                    }
                }
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureView {
    pub account: AccountId,
    pub capture: Capture,
    pub receipt: Receipt,
}
impl Application {
    pub fn nexo_capture_views(&self, portfolio: &PortfolioId) -> Result<Vec<CaptureView>> {
        let mut out = vec![];
        for account in self
            .store()
            .accounts(portfolio)?
            .into_iter()
            .filter(|a| a.provider_id.0 == "nexo")
        {
            for (_, payload, result) in self.store().nexo_captures(&account.id)? {
                out.push(CaptureView {
                    account: account.id.clone(),
                    capture: Capture::parse(&payload)?,
                    receipt: serde_json::from_str(&result)?,
                });
            }
        }
        out.sort_by_key(|v| v.capture.capture_finished_at);
        out.reverse();
        out.truncate(20);
        Ok(out)
    }
}
