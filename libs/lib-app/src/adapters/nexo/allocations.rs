//! Read-only partitions of aggregate Nexo ownership. No synthetic ledger effects.
use super::term_inference::infer_terms;
use crate::{
    dto::{ExactDecimal, QuantityDto},
    exact::ExactRatio,
};
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use lib_core::{AccountId, AssetId, Units};
use nexo_csv::{NexoTx, TransactionType};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermAllocation {
    pub id: String,
    pub account: AccountId,
    pub kind: String,
    pub quantity: QuantityDto,
    pub opened_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub maturity: Option<DateTime<Utc>>,
    #[serde(default)]
    pub captured: Option<crate::nexo_capture::Term>,
    #[serde(default)]
    pub expected_maturity: Option<DateTime<Utc>>,
    pub as_of: DateTime<Utc>,
    pub source_hash: String,
    pub count: usize,
    pub observed_apy: Option<ExactDecimal>,
    pub observed_from: Option<String>,
    #[serde(default)]
    pub expected_apy: Option<ExactRatio>,
    #[serde(default)]
    pub expected_rate_note: Option<String>,
    #[serde(default)]
    pub expected_evidence: Vec<crate::dto::EvidenceDto>,
}
#[derive(Default)]
struct Cohort {
    openings: Vec<(String, DateTime<Utc>)>,
    remaining: usize,
    ambiguous: bool,
}

/// Only identify individual openings while the whole equal-principal cohort is
/// unambiguous. A released ambiguous cohort does not leave "open" ghost locks.
pub fn open_allocations(
    rows: &[NexoTx],
    account: &AccountId,
    hash: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<TermAllocation>> {
    let rows: Vec<_> = rows
        .iter()
        .filter(|r| r.date_time_utc <= cutoff)
        .cloned()
        .collect();
    let Some(as_of) = rows.iter().map(|r| r.date_time_utc).max() else {
        return Ok(vec![]);
    };
    // Export rewards are booked at 05:00, while associated unlock/renewal
    // movements can be a few minutes later. All events in this explicit
    // five-minute window compete; never pick the closest or force a match.
    let history = infer_terms(&rows, 300)?;
    let mut events: BTreeMap<DateTime<Utc>, Vec<&NexoTx>> = BTreeMap::new();
    for row in &rows {
        if matches!(
            row.kind,
            TransactionType::LockTermDeposit | TransactionType::UnlockTermDeposit
        ) {
            events.entry(row.date_time_utc).or_default().push(row);
        }
    }
    let mut active: BTreeMap<(String, Decimal), Cohort> = BTreeMap::new();
    for (time, events) in events {
        let mut closes: BTreeMap<(String, Decimal), usize> = BTreeMap::new();
        for row in &events {
            if row.kind == TransactionType::UnlockTermDeposit {
                *closes
                    .entry((row.input_currency.clone(), row.input_amount))
                    .or_default() += 1;
            }
        }
        for (key, count) in closes {
            let cohort = active.entry(key.clone()).or_default();
            ensure!(
                count <= cohort.remaining,
                "unlock exceeds evidenced locked principal"
            );
            if cohort.remaining > 1 || count > 1 {
                cohort.ambiguous = true;
            }
            cohort.remaining -= count;
            if cohort.remaining == 0 {
                active.remove(&key);
            }
        }
        for row in events {
            if row.kind == TransactionType::LockTermDeposit {
                let cohort = active
                    .entry((row.input_currency.clone(), row.input_amount.abs()))
                    .or_default();
                cohort.ambiguous |= cohort.remaining > 0;
                cohort.remaining += 1;
                cohort.openings.push((row.tx_id.clone(), time));
            }
        }
    }
    let mut out = vec![];
    for ((asset, principal), cohort) in active {
        let ambiguous = cohort.ambiguous;
        let opening = (!ambiguous).then(|| cohort.openings[0].clone());
        // A unique immediate predecessor supplies an observed return, not today's
        // contractual rate. Cross-currency payouts remain unknown without FX.
        let previous = opening.as_ref().and_then(|(_, time)| {
            let matches: Vec<_> = history
                .terms
                .iter()
                .filter(|t| {
                    t.asset == asset
                        && (t.principal == principal
                            || t.close
                                .as_ref()
                                .and_then(|c| c.reward.as_ref())
                                .is_some_and(|r| {
                                    r.asset == asset
                                        && t.principal.checked_add(r.amount) == Some(principal)
                                }))
                        && !t.ambiguous
                        && t.close.as_ref().is_some_and(|c| c.observed_at == *time)
                })
                .collect();
            (matches.len() == 1).then(|| matches[0]).and_then(|t| {
                t.close.as_ref()?.reward.as_ref().and_then(|r| {
                    r.equivalent_apy_estimate
                        .map(|apy| (r.transaction_id.clone(), apy))
                })
            })
        });
        let (observed_from, observed_apy) = match previous {
            Some((id, apy)) if apy >= 0.0 => (Some(id), apy.to_string().parse().ok()),
            _ => (None, None),
        };
        let quantity =
            &Units::from_decimal(&principal.to_string(), 8)? * i32::try_from(cohort.remaining)?;
        let identity = opening
            .as_ref()
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| format!("unresolved:{asset}:{principal}"));
        out.push(TermAllocation {
            id: format!("nexo-term:{}:{identity}", account.0),
            account: account.clone(),
            kind: if ambiguous {
                "unresolved_fixed"
            } else {
                "fixed"
            }
            .into(),
            quantity: QuantityDto {
                asset_id: AssetId::str(&asset),
                units: quantity,
                scale: 8,
            },
            opened_at: opening.map(|(_, t)| t),
            maturity: None,
            captured: None,
            expected_maturity: None,
            as_of,
            source_hash: hash.into(),
            count: cohort.remaining,
            observed_apy,
            observed_from,
            expected_apy: None,
            expected_rate_note: None,
            expected_evidence: vec![],
        });
    }
    Ok(out)
}

/// Partition only when locked <= aggregate quantity after exact scale normalization. Never
/// clamp or add an allocation to its original ownership a second time.
pub fn partition(quantity: &QuantityDto, parts: &[TermAllocation]) -> Result<Vec<TermAllocation>> {
    ensure!(
        !quantity.units.is_negative(),
        "negative aggregate ownership"
    );
    let mut out: Vec<_> = parts
        .iter()
        .filter(|p| p.quantity.asset_id == quantity.asset_id)
        .cloned()
        .collect();
    if out.is_empty() {
        return Ok(out);
    }
    let mut seen = BTreeSet::new();
    let mut locked = Units::from(0);
    for p in &mut out {
        ensure!(seen.insert(&p.id), "duplicate partition identity");
        p.quantity.units = p.quantity.units.rescale(p.quantity.scale, quantity.scale)?;
        p.quantity.scale = quantity.scale;
        locked += &p.quantity.units;
    }
    ensure!(
        locked <= quantity.units,
        "locked principal exceeds aggregate ownership"
    );
    let remaining = quantity.units.clone() - locked;
    if !remaining.is_zero() {
        let mut p = out[0].clone();
        p.id = format!("nexo-term:{}:residual:{}", p.account.0, quantity.asset_id.0);
        p.kind = "unclassified_residual".into();
        p.quantity.units = remaining;
        p.opened_at = None;
        p.maturity = None;
        p.captured = None;
        p.expected_maturity = None;
        p.count = 1;
        p.observed_apy = None;
        p.observed_from = None;
        p.expected_apy = None;
        p.expected_rate_note = None;
        p.expected_evidence.clear();
        out.push(p);
    }
    Ok(out)
}

pub fn observed_ratio(p: &TermAllocation) -> Option<ExactRatio> {
    p.observed_apy.as_ref().map(ExactRatio::from_decimal)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows(lines: &[&str]) -> Vec<NexoTx> {
        lines
            .iter()
            .map(|s| NexoTx::try_from_csv_row(s).unwrap())
            .collect()
    }
    fn at() -> DateTime<Utc> {
        "2026-03-01T00:00:00Z".parse().unwrap()
    }
    #[test]
    fn released_ambiguous_locks_do_not_become_ghost_current_terms() -> Result<()> {
        let data = rows(&[
            "a,Locking Term Deposit,ETH,-10,ETH,10,$0,-,-,lock,2026-01-01 00:00:00",
            "b,Locking Term Deposit,ETH,-10,ETH,10,$0,-,-,lock,2026-01-02 00:00:00",
            "u,Unlocking Term Deposit,ETH,10,ETH,10,$0,-,-,unlock,2026-01-03 00:00:00",
        ]);
        let parts = open_allocations(&data, &AccountId::from("legacy:nexo"), "hash", at())?;
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].count, 1);
        assert_eq!(parts[0].kind, "unresolved_fixed");
        assert!(parts[0].opened_at.is_none());
        let quantity = QuantityDto {
            asset_id: AssetId::str("ETH"),
            units: Units::from_decimal("12", 8)?,
            scale: 8,
        };
        let partition = partition(&quantity, &parts)?;
        assert_eq!(partition.len(), 2);
        assert_eq!(partition[1].quantity.units, Units::from_decimal("2", 8)?);
        let widened = QuantityDto {
            units: quantity.units.rescale(8, 10)?,
            scale: 10,
            ..quantity.clone()
        };
        let widened_parts = super::partition(&widened, &parts)?;
        assert_eq!(
            widened_parts[0].quantity.units,
            partition[0].quantity.units.rescale(8, 10)?
        );
        assert_eq!(
            widened_parts[1].quantity.units,
            partition[1].quantity.units.rescale(8, 10)?
        );
        assert!(
            super::partition(
                &QuantityDto {
                    units: Units::from_decimal("9", 8)?,
                    ..quantity
                },
                &parts
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn unique_reinvested_renewal_carries_labelled_observed_apy_only() -> Result<()> {
        let data = rows(&[
            "a,Locking Term Deposit,NEXO,-100,NEXO,100,$0,-,-,lock,2026-01-01 00:00:00",
            "u,Unlocking Term Deposit,NEXO,100,NEXO,100,$0,-,-,unlock,2026-02-01 00:00:00",
            "r,Fixed Term Interest,NEXO,1,NEXO,1,$0,-,-,approved / NEXO Term Deposit Interest in kind,2026-02-01 00:00:00",
            "b,Locking Term Deposit,NEXO,-101,NEXO,101,$0,-,-,lock,2026-02-01 00:00:00",
        ]);
        let parts = open_allocations(&data, &AccountId::from("legacy:nexo"), "hash", at())?;
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].kind, "fixed");
        assert!(parts[0].observed_apy.is_some());
        assert_eq!(parts[0].observed_from.as_deref(), Some("r"));
        let before = open_allocations(
            &data,
            &AccountId::from("legacy:nexo"),
            "hash",
            "2026-01-15T00:00:00Z".parse()?,
        )?;
        assert!(before[0].observed_apy.is_none());
        Ok(())
    }
    #[test]
    fn missing_opening_is_not_clamped_or_guessed() -> Result<()> {
        let data =
            rows(&["u,Unlocking Term Deposit,ETH,10,ETH,10,$0,-,-,unlock,2026-01-03 00:00:00"]);
        assert!(open_allocations(&data, &AccountId::from("legacy:nexo"), "hash", at()).is_err());
        Ok(())
    }
}
