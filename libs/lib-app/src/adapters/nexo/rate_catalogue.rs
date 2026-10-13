//! Offline, evidence-qualified Nexo rules. Headline maxima never become defaults.
use crate::{dto::ExactDecimal, exact::ExactRatio};
use anyhow::{Result, ensure};
use chrono::NaiveDate;
use lib_core::{
    AccountId, Store,
    ui_metadata::{AnnualRate, AnnualRateBasis},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Base,
    Silver,
    Gold,
    Platinum,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Product {
    Flexible,
    Fixed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Payout {
    InKind,
    Nexo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RateKind {
    HeadlineMaximum,
    DocumentedRate,
    AccountApplicable,
    IllustrativeExample,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub url: String,
    pub scope: Option<String>,
    pub observed_on: NaiveDate,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RateBand {
    /// Ascending marginal upper boundary; final None means unbounded.
    pub up_to_usd: Option<ExactDecimal>,
    pub rate: Option<AnnualRate>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RateRecord {
    pub id: String,
    pub country: String,
    pub asset: String,
    pub product: Product,
    pub fraction: Option<ExactDecimal>,
    pub kind: RateKind,
    pub basis: Option<AnnualRateBasis>,
    pub compounds_per_year: Option<u32>,
    pub tier: Option<Tier>,
    pub payout: Option<Payout>,
    pub months: Option<u8>,
    pub bands: Option<Vec<RateBand>>,
    pub source: String,
    pub effective_from: Option<NaiveDate>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MinimumObservation {
    pub flexible: ExactDecimal,
    pub fixed: ExactDecimal,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub country: String,
    pub account_minimum_usd: ExactDecimal,
    pub grace_days: u32,
    pub opt_in_days: u32,
    pub tier_thresholds: BTreeMap<String, ExactDecimal>,
    pub tier_denominator: String,
    pub nexo_payout_bonus_range: [ExactDecimal; 2],
    pub fixed_months: BTreeMap<String, Vec<u8>>,
    pub fixed_unavailable: Vec<String>,
    pub banded_assets: Vec<String>,
    pub minimum_tiers: BTreeMap<String, Tier>,
    pub credit_wallet_earners: Vec<String>,
    pub eea_unavailable: Vec<String>,
    pub fixed_minimum_grouping: Option<String>,
    pub flexible_cooldown_hours: [u32; 2],
    pub flexible_snapshot_utc: String,
    pub flexible_payout_utc: [String; 2],
    pub fixed_payout_utc: [String; 2],
    pub fixed_max_terms_per_asset: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub version: u32,
    pub observed_on: NaiveDate,
    pub review_after: NaiveDate,
    pub minima_review_on: NaiveDate,
    pub sources: BTreeMap<String, Source>,
    pub rules: Rules,
    pub minimum_observations: BTreeMap<String, MinimumObservation>,
    pub rates: Vec<RateRecord>,
}
impl Catalogue {
    pub fn bundled() -> Result<Self> {
        let catalogue: Self = serde_json::from_str(include_str!("data/rates-v1.json"))?;
        catalogue.validate()?;
        Ok(catalogue)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1
                && self.rules.country == "GB"
                && self.rules.tier_denominator == "portfolio_excluding_nexo",
            "unsupported catalogue contract"
        );
        ensure!(
            self.observed_on < self.review_after && self.observed_on < self.minima_review_on,
            "invalid catalogue dates"
        );
        let mut ids = BTreeSet::new();
        for record in &self.rates {
            ensure!(
                ids.insert(&record.id) && !record.asset.is_empty(),
                "duplicate/empty rate identity"
            );
            let source = self
                .sources
                .get(&record.source)
                .ok_or_else(|| anyhow::anyhow!("unknown rate source"))?;
            ensure!(
                source
                    .scope
                    .as_ref()
                    .is_none_or(|scope| scope == &record.country),
                "regional source mismatch"
            );
            if let Some(fraction) = &record.fraction {
                ensure!(!fraction.is_negative(), "negative catalogue rate");
            }
            if let Some(bands) = &record.bands {
                ensure!(!bands.is_empty(), "empty bands");
                let mut previous = ExactRatio::zero();
                for (i, band) in bands.iter().enumerate() {
                    if let Some(limit) = &band.up_to_usd {
                        let limit = ratio(limit);
                        ensure!(limit > previous, "nonascending bands");
                        previous = limit;
                    } else {
                        ensure!(i == bands.len() - 1, "unbounded nonfinal band");
                    }
                    if let Some(rate) = &band.rate {
                        ensure!(!rate.fraction.is_negative(), "negative band rate");
                    }
                }
                ensure!(
                    bands.last().unwrap().up_to_usd.is_none(),
                    "incomplete band coverage"
                );
            }
        }
        Ok(())
    }
}
fn ratio(value: &ExactDecimal) -> ExactRatio {
    ExactRatio::from_decimal(value)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wallet {
    Savings,
    Credit,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    EarnFlexible,
    CreateFixed,
    RenewFixed,
    ExistingFixed,
}
/// Source-confirmed minimum for the quantity scope being evaluated. A fixed
/// per-term minimum cannot be supplied with an aggregate holding quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimumScope {
    FlexibleBalance,
    IndividualTerm,
}
#[derive(Clone, Debug)]
pub struct ConfirmedMinimum {
    pub asset: String,
    pub source_reference: String,
    pub units: ExactDecimal,
    pub scope: MinimumScope,
    pub effective_from: NaiveDate,
    pub valid_until: NaiveDate,
}
#[derive(Clone, Debug)]
pub struct Facts {
    pub at: NaiveDate,
    pub asset: String,
    pub operation: Operation,
    pub months: Option<u8>,
    pub wallet: Option<Wallet>,
    pub payout: Option<Payout>,
    pub quantity: Option<ExactDecimal>,
    pub product_value_usd: Option<ExactDecimal>,
    /// Nexo's provider-wide valuation, not app net worth; completeness is explicit.
    pub portfolio_usd: Option<ExactDecimal>,
    pub nexo_usd: Option<ExactDecimal>,
    pub complete_provider_valuation: bool,
    pub platform_tier: Option<Tier>,
    /// Platform confirmation of current grace/access status; not a guessed timer.
    pub loyalty_access: Option<bool>,
    pub opt_in_valid_until: Option<NaiveDate>,
    pub earnable: Option<bool>,
    pub minimum: Option<ConfirmedMinimum>,
    pub active_term_fallback_confirmed: Option<bool>,
    pub current_term_count: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Eligibility {
    Eligible,
    Ineligible,
    Unknown,
}
#[derive(Clone, Debug, Serialize)]
pub struct Evaluation {
    pub account_id: AccountId,
    pub country: Option<String>,
    pub tier: Option<Tier>,
    pub eligibility: Eligibility,
    pub qualification_issues: Vec<String>,
    pub rate_issues: Vec<String>,
    pub annual_fraction: Option<ExactRatio>,
    /// Known part of annual expected base-asset earnings, only when eligible.
    pub known_annual_earnings_base: Option<ExactRatio>,
    pub rate_complete: bool,
    pub headline_records: Vec<String>,
    pub selected_record: Option<String>,
    pub source_url: Option<String>,
}

/// Jurisdiction is read from this specific stored account; callers cannot replace
/// it with a UI-global default. No rates, ownership or overrides are written.
pub fn evaluate_account(store: &Store, account: &AccountId, facts: &Facts) -> Result<Evaluation> {
    let mut account_record = None;
    for portfolio in store.portfolios()? {
        if let Some(found) = store
            .accounts(&portfolio.id)?
            .into_iter()
            .find(|a| a.id == *account)
        {
            account_record = Some(found);
            break;
        }
    }
    let account_record = account_record.ok_or_else(|| anyhow::anyhow!("unknown account"))?;
    ensure!(
        account_record.provider_id.0.eq_ignore_ascii_case("nexo"),
        "account is not Nexo"
    );
    let jurisdiction = store.account_jurisdiction(account)?;
    let country = jurisdiction.as_ref().map(|j| j.country_code.as_str());
    let mut evaluation = evaluate(&Catalogue::bundled()?, account, country, facts)?;
    if jurisdiction.is_some_and(|j| facts.at < j.declared_at.date_naive()) {
        evaluation.eligibility = Eligibility::Unknown;
        evaluation
            .qualification_issues
            .push("jurisdiction_history_unknown".into());
        evaluation.annual_fraction = None;
        evaluation.known_annual_earnings_base = None;
        evaluation.rate_complete = false;
    }
    Ok(evaluation)
}
/// Pure rule evaluation, also usable for reviewed future snapshots in tests.
pub fn evaluate(
    catalogue: &Catalogue,
    account: &AccountId,
    country: Option<&str>,
    facts: &Facts,
) -> Result<Evaluation> {
    catalogue.validate()?;
    for value in [
        &facts.quantity,
        &facts.portfolio_usd,
        &facts.nexo_usd,
        &facts.product_value_usd,
    ]
    .into_iter()
    .flatten()
    {
        ensure!(!value.is_negative(), "negative qualification value");
    }
    if let (Some(total), Some(nexo)) = (&facts.portfolio_usd, &facts.nexo_usd) {
        ensure!(
            ratio(nexo) <= ratio(total),
            "NEXO value exceeds provider valuation"
        );
    }
    if let Some(minimum) = &facts.minimum {
        ensure!(
            !minimum.units.is_negative()
                && !ratio(&minimum.units).is_zero()
                && !minimum.source_reference.trim().is_empty()
                && minimum.valid_until > minimum.effective_from,
            "invalid minimum evidence"
        );
    }
    let product = if facts.operation == Operation::EarnFlexible {
        Product::Flexible
    } else {
        Product::Fixed
    };
    let mut result = Evaluation {
        account_id: account.clone(),
        country: country.map(str::to_owned),
        tier: None,
        eligibility: Eligibility::Unknown,
        qualification_issues: Vec::new(),
        rate_issues: Vec::new(),
        annual_fraction: None,
        known_annual_earnings_base: None,
        rate_complete: false,
        headline_records: Vec::new(),
        selected_record: None,
        source_url: None,
    };
    if country != Some(catalogue.rules.country.as_str()) {
        result.qualification_issues.push(
            if country.is_none() {
                "jurisdiction_unknown"
            } else {
                "programme_rules_unavailable"
            }
            .into(),
        );
        return Ok(result);
    }
    if facts.at < catalogue.observed_on || facts.at >= catalogue.review_after {
        result
            .qualification_issues
            .push("catalogue_outside_review_window".into());
        return Ok(result);
    }
    let mut denied = false;
    let mut missing = Vec::new();
    let minimum_account = ratio(&catalogue.rules.account_minimum_usd);
    let balance_ok = facts
        .complete_provider_valuation
        .then(|| {
            facts
                .portfolio_usd
                .as_ref()
                .map(|v| ratio(v) >= minimum_account)
        })
        .flatten();
    let access = facts.loyalty_access.or(balance_ok.filter(|ok| *ok));
    match access {
        Some(true) => {}
        Some(false) if facts.operation == Operation::ExistingFixed => {
            if facts.active_term_fallback_confirmed == Some(true) {
                result.tier = Some(Tier::Base);
            } else {
                missing.push("active_term_fallback_unconfirmed".into());
            }
        }
        Some(false) => {
            denied = true;
            missing.push("loyalty_access_suspended".into());
        }
        None => missing.push("account_balance_or_grace_status_unknown".into()),
    }
    if result.tier.is_none() {
        result.tier = facts.platform_tier.or_else(|| {
            if !facts.complete_provider_valuation {
                return None;
            }
            let total = ratio(facts.portfolio_usd.as_ref()?);
            let nexo = ratio(facts.nexo_usd.as_ref()?);
            let rest = total.sub(&nexo);
            if rest <= ExactRatio::zero() {
                return None;
            }
            let share = nexo.div(&rest)?;
            for (key, tier) in [
                ("platinum", Tier::Platinum),
                ("gold", Tier::Gold),
                ("silver", Tier::Silver),
            ] {
                if share >= ratio(catalogue.rules.tier_thresholds.get(key)?) {
                    return Some(tier);
                }
            }
            Some(Tier::Base)
        });
    }
    if result.tier.is_none() {
        missing.push("tier_unknown".into());
    }
    if let Some(required) = catalogue.rules.minimum_tiers.get(&facts.asset) {
        if result.tier.is_some_and(|tier| tier < *required) {
            denied = true;
            missing.push("asset_tier_ineligible".into());
        }
    }
    // Existing terms continue after opt-out. New/renewed terms and flexible
    // earnings require current agreement evidence in this UK profile.
    if facts.operation != Operation::ExistingFixed {
        match facts.opt_in_valid_until {
            Some(until) if until > facts.at => {}
            Some(_) => {
                denied = true;
                missing.push("opt_in_expired".into());
            }
            None => missing.push("opt_in_unknown".into()),
        }
    }
    if product == Product::Flexible {
        match facts.wallet {
            Some(Wallet::Savings) => {}
            Some(Wallet::Credit)
                if catalogue.rules.credit_wallet_earners.contains(&facts.asset) => {}
            Some(_) => {
                denied = true;
                missing.push("wallet_ineligible".into());
            }
            None => missing.push("wallet_unknown".into()),
        }
        match facts.earnable {
            Some(true) => {}
            Some(false) => {
                denied = true;
                missing.push("balance_not_yet_earnable".into());
            }
            None => missing.push("cooldown_or_snapshot_unknown".into()),
        }
    } else {
        if catalogue.rules.fixed_unavailable.contains(&facts.asset) {
            denied = true;
            missing.push("fixed_product_unavailable".into());
        }
        match (catalogue.rules.fixed_months.get(&facts.asset), facts.months) {
            (Some(months), Some(duration)) if months.contains(&duration) => {}
            (Some(_), Some(_)) => {
                denied = true;
                missing.push("duration_ineligible".into());
            }
            _ => missing.push("duration_or_product_unknown".into()),
        }
        if facts.operation != Operation::ExistingFixed {
            match facts.wallet {
                Some(Wallet::Savings) => {}
                Some(_) => {
                    denied = true;
                    missing.push("wallet_ineligible".into());
                }
                None => missing.push("wallet_unknown".into()),
            }
            match facts.current_term_count {
                Some(n)
                    if (facts.operation == Operation::CreateFixed
                        && n >= catalogue.rules.fixed_max_terms_per_asset)
                        || n > catalogue.rules.fixed_max_terms_per_asset =>
                {
                    denied = true;
                    missing.push("term_count_limit".into());
                }
                None => missing.push("term_count_unknown".into()),
                _ => {}
            }
        }
    }
    // Bundled minima are unscoped observations, never silently current rules.
    // Active legacy terms are not retroactively denied by new creation minima.
    if facts.operation != Operation::ExistingFixed {
        let expected_scope = if product == Product::Flexible {
            MinimumScope::FlexibleBalance
        } else {
            MinimumScope::IndividualTerm
        };
        match (&facts.minimum, &facts.quantity) {
            (Some(m), Some(q))
                if m.asset == facts.asset
                    && m.scope == expected_scope
                    && m.effective_from <= facts.at
                    && facts.at < m.valid_until =>
            {
                if ratio(q) < ratio(&m.units) {
                    denied = true;
                    missing.push("product_minimum_not_met".into());
                }
            }
            _ => missing.push("minimum_scope_or_freshness_unknown".into()),
        }
    }
    result.qualification_issues = missing;
    result.eligibility = if denied {
        Eligibility::Ineligible
    } else if result.qualification_issues.is_empty() {
        Eligibility::Eligible
    } else {
        Eligibility::Unknown
    };
    result.headline_records = catalogue
        .rates
        .iter()
        .filter(|r| {
            r.country == country.unwrap()
                && r.asset == facts.asset
                && r.product == product
                && r.kind == RateKind::HeadlineMaximum
        })
        .map(|r| r.id.clone())
        .collect();
    if facts.payout.is_none() {
        result.rate_issues.push("payout_unknown".into());
    }
    let compatible: Vec<_> = catalogue
        .rates
        .iter()
        .filter(|r| {
            r.country == country.unwrap()
                && r.asset == facts.asset
                && r.product == product
                && matches!(
                    r.kind,
                    RateKind::DocumentedRate | RateKind::AccountApplicable
                )
                && r.tier.is_some()
                && r.tier == result.tier
                && r.payout.is_some()
                && r.payout == facts.payout
                && (product == Product::Flexible || r.months.is_some() && r.months == facts.months)
                && r.effective_from.is_some_and(|date| date <= facts.at)
                && catalogue.sources[&r.source].observed_on <= facts.at
                && catalogue.sources[&r.source].scope.as_deref() == country
        })
        .collect();
    if compatible.len() != 1 {
        result.rate_issues.push(
            if compatible.is_empty() {
                "applicable_rate_missing"
            } else {
                "conflicting_rate_evidence"
            }
            .into(),
        );
        return Ok(result);
    }
    let record = compatible[0];
    result.selected_record = Some(record.id.clone());
    result.source_url = Some(catalogue.sources[&record.source].url.clone());
    if result.eligibility != Eligibility::Eligible {
        result
            .rate_issues
            .push("qualification_unresolved_or_ineligible".into());
        return Ok(result);
    }
    if access == Some(false) && facts.payout == Some(Payout::Nexo) {
        result
            .rate_issues
            .push("active_fallback_bonus_matrix_missing".into());
        return Ok(result);
    }
    if let Some(bands) = &record.bands {
        let Some(usd) = facts.product_value_usd.as_ref().map(ratio) else {
            result.rate_issues.push("band_valuation_unknown".into());
            return Ok(result);
        };
        if usd.is_zero() {
            result.rate_issues.push("zero_band_valuation".into());
            return Ok(result);
        }
        let mut remaining = usd.clone();
        let mut lower = ExactRatio::zero();
        let mut weighted = ExactRatio::zero();
        let mut complete = true;
        let mut any = false;
        for band in bands {
            if remaining.is_zero() {
                break;
            }
            let width = band
                .up_to_usd
                .as_ref()
                .map(|v| ratio(v).sub(&lower))
                .unwrap_or_else(|| remaining.clone());
            let allocated = if width < remaining {
                width
            } else {
                remaining.clone()
            };
            if let Some(rate) = band.rate.as_ref().and_then(|rate| {
                if facts.payout == Some(Payout::InKind)
                    && rate.basis == AnnualRateBasis::Apr
                    && rate.compounds_per_year.is_none()
                {
                    None
                } else {
                    effective_rate(rate)
                }
            }) {
                weighted = weighted.add(&allocated.mul(&rate));
                any = true;
            } else {
                complete = false;
            }
            remaining = remaining.sub(&allocated);
            if let Some(upper) = &band.up_to_usd {
                lower = ratio(upper);
            }
        }
        result.rate_complete = complete;
        let average = weighted.div(&usd).unwrap();
        if complete {
            result.annual_fraction = Some(average.clone());
        } else {
            result.rate_issues.push("rate_band_incomplete".into());
        }
        if any {
            result.known_annual_earnings_base =
                facts.quantity.as_ref().map(|q| ratio(q).mul(&average));
        }
    } else if catalogue.rules.banded_assets.contains(&facts.asset) {
        result.rate_issues.push("balance_bands_missing".into());
    } else if let (Some(fraction), Some(basis)) = (&record.fraction, &record.basis) {
        if facts.payout == Some(Payout::InKind)
            && *basis == AnnualRateBasis::Apr
            && record.compounds_per_year.is_none()
        {
            result
                .rate_issues
                .push("compounding_frequency_unknown".into());
            return Ok(result);
        }
        let rate = AnnualRate {
            fraction: fraction.clone(),
            basis: basis.clone(),
            compounds_per_year: record.compounds_per_year,
            reward_asset: None,
        };
        result.annual_fraction = effective_rate(&rate);
        result.rate_complete = result.annual_fraction.is_some();
        if !result.rate_complete {
            result.rate_issues.push("unsupported_compounding".into());
        }
        result.known_annual_earnings_base = result
            .annual_fraction
            .as_ref()
            .zip(facts.quantity.as_ref())
            .map(|(rate, q)| rate.mul(&ratio(q)));
    } else {
        result
            .rate_issues
            .push("annual_basis_or_rate_unknown".into());
    }
    Ok(result)
}
fn effective_rate(rate: &AnnualRate) -> Option<ExactRatio> {
    let fraction = ratio(&rate.fraction);
    match (&rate.basis, rate.compounds_per_year) {
        (AnnualRateBasis::Apy, _) | (AnnualRateBasis::Apr, None) => Some(fraction),
        (AnnualRateBasis::Apr, Some(n)) if n > 0 && n <= 366 => {
            let one = ExactRatio::integer(1);
            let divisor = ExactRatio::integer(n as i32);
            Some(one.add(&fraction.div(&divisor)?).pow(n).sub(&one))
        }
        _ => None,
    }
}
