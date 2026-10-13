//! Read-only historical candidates from a single account's Nexo CSV.
//! These are inferred links, never contractual maturity/current-rate evidence.
use crate::{dto::ExactDecimal, exact::ExactRatio};
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use nexo_csv::{NexoTx, TransactionType};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct TermCandidate {
    pub lock_id: String,
    pub asset: String,
    pub principal: Decimal,
    pub opened_at: DateTime<Utc>,
    pub close: Option<InferredClose>,
    pub ambiguous: bool,
}
#[derive(Debug)]
pub struct InferredClose {
    pub unlock_id: String,
    pub observed_at: DateTime<Utc>,
    pub elapsed_seconds: i64,
    pub reward: Option<InferredReward>,
}
#[derive(Debug)]
pub struct InferredReward {
    pub transaction_id: String,
    pub asset: String,
    pub amount: Decimal,
    pub timestamp_offset_seconds: i64,
    /// Exact return for this observed period, only for same-asset rewards.
    /// Conditional on this payout covering the whole inferred term; the CSV
    /// does not prove that there were no other payouts or adjustments.
    pub period_return: Option<ExactRatio>,
    /// Approximate annual equivalent: repeat/reinvest the same observed return
    /// over a fixed 365-day year. Not an exact value or contractual APY.
    pub equivalent_apy_estimate: Option<f64>,
}
#[derive(Debug, Default)]
pub struct TermInference {
    pub terms: Vec<TermCandidate>,
    pub unresolved_unlocks: Vec<String>,
    pub unresolved_rewards: Vec<String>,
}
#[derive(Default)]
struct Cohort {
    locks: Vec<usize>,
    remaining: usize,
    ambiguous: bool,
}

/// Scope must be one account's complete export. The explicit symmetric nearby
/// window (0 = same timestamp only) never grants permission to choose nearest.
/// Reject malformed movements/duplicate IDs instead of producing partial lies.
pub fn infer_terms(rows: &[NexoTx], reward_window_seconds: i64) -> Result<TermInference> {
    ensure!(reward_window_seconds >= 0, "negative reward window");
    let mut ids = BTreeSet::new();
    let mut times: BTreeMap<DateTime<Utc>, Vec<&NexoTx>> = BTreeMap::new();
    let mut unlocks = Vec::new();
    let mut rewards = Vec::new();
    for row in rows {
        ensure!(
            ids.insert(&row.tx_id),
            "duplicate Nexo transaction identity"
        );
        match row.kind {
            TransactionType::LockTermDeposit | TransactionType::UnlockTermDeposit => {
                let lock = row.kind == TransactionType::LockTermDeposit;
                ensure!(
                    row.input_currency == row.output_currency
                        && !row.input_currency.is_empty()
                        && row.input_amount != Decimal::ZERO
                        && row.input_amount.abs() == row.output_amount
                        && (row.input_amount < Decimal::ZERO) == lock,
                    "inconsistent term movement"
                );
                times.entry(row.date_time_utc).or_default().push(row);
                if !lock {
                    unlocks.push(row);
                }
            }
            TransactionType::FixedTermInterest => rewards.push(row),
            _ => {}
        }
    }
    unlocks.sort_by_key(|r| (r.date_time_utc, &r.tx_id));
    rewards.sort_by_key(|r| (r.date_time_utc, &r.tx_id));
    let mut report = TermInference::default();
    let mut active: BTreeMap<(String, Decimal), Cohort> = BTreeMap::new();
    for (time, mut events) in times {
        // Same-time locks cannot be openings for these unlocks. Deterministic
        // sort is presentation only; tied unlocks are processed as a group.
        events.sort_by_key(|r| &r.tx_id);
        let mut closing: BTreeMap<(String, Decimal), Vec<&NexoTx>> = BTreeMap::new();
        for row in &events {
            if row.kind == TransactionType::UnlockTermDeposit {
                closing
                    .entry((row.input_currency.clone(), row.input_amount))
                    .or_default()
                    .push(row);
            }
        }
        for (key, ends) in closing {
            let cohort = active.entry(key).or_default();
            if !cohort.ambiguous && cohort.remaining == 1 && ends.len() == 1 {
                let term = &mut report.terms[cohort.locks[0]];
                term.close = Some(InferredClose {
                    unlock_id: ends[0].tx_id.clone(),
                    observed_at: time,
                    elapsed_seconds: (time - term.opened_at).num_seconds(),
                    reward: None,
                });
            } else {
                report
                    .unresolved_unlocks
                    .extend(ends.iter().map(|r| r.tx_id.clone()));
                cohort.ambiguous = true;
                for &index in &cohort.locks {
                    report.terms[index].ambiguous = true;
                }
            }
            // No individual lock is consumed under ambiguity. Discard a cohort
            // only when all its possible principal has been released; never
            // let a later "one remaining" count repair an earlier arbitrary tie.
            cohort.remaining = cohort.remaining.saturating_sub(ends.len());
            if cohort.remaining == 0 {
                *cohort = Cohort::default();
            }
        }
        for row in events {
            if row.kind != TransactionType::LockTermDeposit {
                continue;
            }
            let cohort = active
                .entry((row.input_currency.clone(), row.input_amount.abs()))
                .or_default();
            let index = report.terms.len();
            report.terms.push(TermCandidate {
                lock_id: row.tx_id.clone(),
                asset: row.input_currency.clone(),
                principal: row.input_amount.abs(),
                opened_at: time,
                close: None,
                ambiguous: cohort.ambiguous || cohort.remaining > 0,
            });
            if cohort.remaining > 0 {
                cohort.ambiguous = true;
                for &other in &cohort.locks {
                    report.terms[other].ambiguous = true;
                }
            }
            cohort.locks.push(index);
            cohort.remaining += 1;
        }
    }
    // All unlocks compete, including ones whose principal match is unresolved.
    let candidates: Vec<Vec<usize>> = rewards
        .iter()
        .map(|reward| {
            if reward.input_amount < Decimal::ZERO
                || reward.output_amount <= Decimal::ZERO
                || reward.details.to_lowercase().contains("adjustment")
            {
                return Vec::new();
            }
            let base = explicit_reward_base(&reward.details);
            unlocks
                .iter()
                .enumerate()
                .filter_map(|(i, end)| {
                    let nearby = (reward.date_time_utc - end.date_time_utc)
                        .num_seconds()
                        .abs()
                        <= reward_window_seconds;
                    (nearby && base.is_none_or(|asset| asset == end.input_currency)).then_some(i)
                })
                .collect()
        })
        .collect();
    let mut competitors = vec![0; unlocks.len()];
    for list in &candidates {
        for &i in list {
            competitors[i] += 1;
        }
    }
    for (reward, list) in rewards.iter().zip(&candidates) {
        let term = if list.len() == 1 && competitors[list[0]] == 1 {
            let end = unlocks[list[0]];
            report.terms.iter_mut().find(|term| {
                term.close
                    .as_ref()
                    .is_some_and(|c| c.unlock_id == end.tx_id)
            })
        } else {
            None
        };
        let Some(term) = term else {
            report.unresolved_rewards.push(reward.tx_id.clone());
            continue;
        };
        let close = term.close.as_mut().expect("matched close");
        let period_return = if reward.output_currency == term.asset {
            let amount = ExactRatio::from_decimal(
                &reward.output_amount.to_string().parse::<ExactDecimal>()?,
            );
            let principal =
                ExactRatio::from_decimal(&term.principal.to_string().parse::<ExactDecimal>()?);
            amount.div(&principal)
        } else {
            None
        };
        let equivalent_apy_estimate = if period_return.is_some() && close.elapsed_seconds > 0 {
            let ratio = reward
                .output_amount
                .to_f64()
                .zip(term.principal.to_f64())
                .map(|(r, p)| r / p);
            ratio
                .map(|r| (r.ln_1p() * (365.0 * 86400.0 / close.elapsed_seconds as f64)).exp_m1())
                .filter(|v| v.is_finite())
        } else {
            None
        };
        close.reward = Some(InferredReward {
            transaction_id: reward.tx_id.clone(),
            asset: reward.output_currency.clone(),
            amount: reward.output_amount,
            timestamp_offset_seconds: (reward.date_time_utc - close.observed_at).num_seconds(),
            period_return,
            equivalent_apy_estimate,
        });
    }
    Ok(report)
}

// Only explicit source descriptions restrict earning asset. A generic reward
// paid in NEXO does not prove that the principal was NEXO.
fn explicit_reward_base(details: &str) -> Option<&str> {
    let text = details.strip_prefix("approved / ")?;
    let (asset, suffix) = text.split_once(" Term Deposit Interest ")?;
    matches!(suffix, "in NEXO" | "in kind").then_some(asset)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: &str, kind: &str, asset: &str, amount: &str, date: &str, detail: &str) -> NexoTx {
        let output = amount.trim_start_matches('-');
        NexoTx::try_from_csv_row(&format!(
            "{id},{kind},{asset},{amount},{asset},{output},$1,-,-,{detail},{date}"
        ))
        .unwrap()
    }
    fn lock(id: &str, amount: &str, date: &str) -> NexoTx {
        row(
            id,
            "Locking Term Deposit",
            "NEXO",
            amount,
            date,
            "approved / transfer",
        )
    }
    fn unlock(id: &str, amount: &str, date: &str) -> NexoTx {
        row(
            id,
            "Unlocking Term Deposit",
            "NEXO",
            amount,
            date,
            "approved / transfer",
        )
    }
    fn reward(id: &str, amount: &str, date: &str) -> NexoTx {
        row(
            id,
            "Fixed Term Interest",
            "NEXO",
            amount,
            date,
            "approved / Term Deposit Interest",
        )
    }
    const START: &str = "2025-01-01 00:00:00";
    const END: &str = "2026-01-01 00:00:00";
    #[test]
    fn unique_term_preserves_exact_return_and_labels_annual_equivalent() -> Result<()> {
        let rows = vec![
            reward("r", "10", END),
            unlock("u", "100", END),
            lock("l", "-100", START),
        ];
        let report = infer_terms(&rows, 0)?;
        let close = report.terms[0].close.as_ref().unwrap();
        assert_eq!(close.elapsed_seconds, 365 * 86400);
        let reward = close.reward.as_ref().unwrap();
        assert_eq!(
            reward.period_return,
            Some(
                ExactRatio::integer(1)
                    .div(&ExactRatio::integer(10))
                    .unwrap()
            )
        );
        assert!((reward.equivalent_apy_estimate.unwrap() - 0.1).abs() < 1e-12);
        assert!(report.unresolved_rewards.is_empty());
        Ok(())
    }
    #[test]
    fn overlapping_equal_principals_never_recover_via_fifo() -> Result<()> {
        let rows = vec![
            lock("a", "-100", START),
            lock("b", "-100", "2025-01-02 00:00:00"),
            unlock("u1", "100", "2025-02-01 00:00:00"),
            unlock("u2", "100", END),
            reward("r", "10", END),
        ];
        let report = infer_terms(&rows, 0)?;
        assert!(
            report
                .terms
                .iter()
                .all(|t| t.ambiguous && t.close.is_none())
        );
        assert_eq!(report.unresolved_unlocks.len(), 2);
        assert_eq!(report.unresolved_rewards.len(), 1);
        Ok(())
    }
    #[test]
    fn exact_large_principals_and_same_time_renewals_are_distinct() -> Result<()> {
        let rows = vec![
            lock("a", "-9007199254740993.00000001", START),
            lock("b", "-9007199254740993.00000002", START),
            unlock("u1", "9007199254740993.00000001", END),
            unlock("u2", "9007199254740993.00000002", END),
            lock("renewal", "-9007199254740993.00000001", END),
        ];
        let report = infer_terms(&rows, 0)?;
        assert_eq!(report.terms.iter().filter(|t| t.close.is_some()).count(), 2);
        assert!(!report.terms.iter().any(|t| t.ambiguous));
        assert!(
            report
                .terms
                .iter()
                .find(|t| t.lock_id == "renewal")
                .unwrap()
                .close
                .is_none()
        );
        Ok(())
    }
    #[test]
    fn competing_rewards_and_unmatched_unlocks_block_attribution() -> Result<()> {
        let report = infer_terms(
            &[
                lock("l", "-100", START),
                unlock("u", "100", END),
                reward("r1", "1", END),
                reward("r2", "2", END),
            ],
            0,
        )?;
        assert!(report.terms[0].close.as_ref().unwrap().reward.is_none());
        assert_eq!(report.unresolved_rewards.len(), 2);
        let report = infer_terms(
            &[
                lock("l", "-100", START),
                unlock("u", "100", END),
                unlock("missing", "200", END),
                reward("r", "1", END),
            ],
            0,
        )?;
        assert!(report.terms[0].close.as_ref().unwrap().reward.is_none());
        assert_eq!(report.unresolved_unlocks, ["missing"]);
        Ok(())
    }
    #[test]
    fn nearby_window_is_explicit_and_currency_conversion_is_not_invented() -> Result<()> {
        let mut l = lock("l", "-100", START);
        l.input_currency = "ETH".into();
        l.output_currency = "ETH".into();
        let mut u = unlock("u", "100", END);
        u.input_currency = "ETH".into();
        u.output_currency = "ETH".into();
        let mut r = reward("r", "10", "2026-01-01 00:00:30");
        r.details = "approved / ETH Term Deposit Interest in NEXO".into();
        let rows = [l, u, r];
        assert_eq!(infer_terms(&rows, 0)?.unresolved_rewards.len(), 1);
        let report = infer_terms(&rows, 30)?;
        let reward = report.terms[0]
            .close
            .as_ref()
            .unwrap()
            .reward
            .as_ref()
            .unwrap();
        assert_eq!(reward.timestamp_offset_seconds, 30);
        assert!(reward.period_return.is_none() && reward.equivalent_apy_estimate.is_none());
        Ok(())
    }
    #[test]
    fn explicit_base_disambiguates_assets_but_adjustments_and_credit_do_not() -> Result<()> {
        let mut l = lock("eth", "-100", START);
        l.input_currency = "ETH".into();
        l.output_currency = "ETH".into();
        let mut u = unlock("eth-end", "100", END);
        u.input_currency = "ETH".into();
        u.output_currency = "ETH".into();
        let mut r = reward("eth-reward", "10", END);
        r.details = "approved / ETH Term Deposit Interest in NEXO".into();
        let mut adjustment = reward("adjustment", "1", END);
        adjustment.details = "approved / Earn Adjustment Fixed Term".into();
        let report = infer_terms(
            &[
                l,
                u,
                lock("nexo", "-100", START),
                unlock("nexo-end", "100", END),
                r,
                adjustment,
                reward("credit", "-1", END),
            ],
            0,
        )?;
        assert!(
            report
                .terms
                .iter()
                .find(|t| t.asset == "ETH")
                .unwrap()
                .close
                .as_ref()
                .unwrap()
                .reward
                .is_some()
        );
        assert_eq!(report.unresolved_rewards, ["adjustment", "credit"]);
        Ok(())
    }
    #[test]
    fn duplicate_invalid_and_same_time_only_movements_cannot_fabricate_terms() {
        assert!(
            infer_terms(
                &[
                    lock("duplicate", "-100", START),
                    unlock("duplicate", "100", END)
                ],
                0
            )
            .is_err()
        );
        assert!(infer_terms(&[lock("bad", "100", START)], 0).is_err());
        assert!(infer_terms(&[], -1).is_err());
        let report =
            infer_terms(&[lock("l", "-100", START), unlock("u", "100", START)], 0).unwrap();
        assert_eq!(report.unresolved_unlocks.len(), 1);
        assert!(report.terms[0].close.is_none());
    }

    #[test]
    fn generic_nexo_payout_cannot_choose_nexo_principal_over_another_asset() -> Result<()> {
        let mut l = lock("eth", "-100", START);
        l.input_currency = "ETH".into();
        l.output_currency = "ETH".into();
        let mut u = unlock("eth-end", "100", END);
        u.input_currency = "ETH".into();
        u.output_currency = "ETH".into();
        let mut rows = vec![
            l,
            u,
            lock("nexo", "-100", START),
            unlock("nexo-end", "100", END),
            reward("generic", "1", END),
        ];
        let report = infer_terms(&rows, 0)?;
        assert_eq!(report.unresolved_rewards, ["generic"]);
        assert!(
            report
                .terms
                .iter()
                .all(|t| t.close.as_ref().unwrap().reward.is_none())
        );
        rows.reverse();
        let reversed = infer_terms(&rows, 0)?;
        assert_eq!(reversed.unresolved_rewards, report.unresolved_rewards);
        assert_eq!(
            reversed
                .terms
                .iter()
                .map(|t| &t.lock_id)
                .collect::<Vec<_>>(),
            report.terms.iter().map(|t| &t.lock_id).collect::<Vec<_>>()
        );
        Ok(())
    }
}
