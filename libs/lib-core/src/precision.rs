//! Explicit lossless scale upgrades; ordinary registrations remain immutable.
use crate::{AssetId, PortfolioId, Store, Units};
use anyhow::{Result, ensure};
use duckdb::params;
use std::str::FromStr;

impl Store {
    /// Widen every persisted integer quantity for an asset in one transaction.
    /// Refuse scale-dependent immutable UI evidence rather than rewriting its history.
    pub fn widen_asset_scale(&self, asset: &AssetId, scale: u8) -> Result<bool> {
        let tx = self.connection.unchecked_transaction()?;
        let old: u8 = tx.query_row(
            "SELECT decimals FROM asset_scales WHERE asset_id=?",
            [&asset.0],
            |r| r.get(0),
        )?;
        ensure!(scale >= old, "scale narrowing is forbidden");
        if old == scale {
            return Ok(false);
        }
        let evidence = tx
            .prepare("SELECT payload FROM ui_balance_evidence")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for payload in evidence {
            let value: crate::ui_metadata::BalanceEvidence = serde_json::from_str(&payload)?;
            ensure!(
                value.asset != *asset,
                "scale upgrade requires review of immutable balance evidence for {}",
                asset.0
            );
        }
        let verifications = tx
            .prepare("SELECT payload FROM ui_verifications")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for payload in verifications {
            let value: crate::ui_metadata::VerificationRecord = serde_json::from_str(&payload)?;
            for issue in value.issues {
                if issue.expected.is_some() || issue.actual.is_some() {
                    if let Some(position) = issue.position {
                        let same: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM positions WHERE position_id=? AND asset_id=?)", params![position.0,asset.0], |r|r.get(0))?;
                        ensure!(
                            !same,
                            "scale upgrade requires review of immutable quantity verification"
                        );
                    }
                }
            }
        }
        let effects = tx.prepare("SELECT transaction_id,e.position_id,amount,effect_order FROM transaction_effects e JOIN positions p USING(position_id) WHERE p.asset_id=?")?
            .query_map([&asset.0], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i32>(3)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        let observations = tx.prepare("SELECT source,content_hash,o.position_id,datetime,amount FROM balance_observations o JOIN positions p USING(position_id) WHERE p.asset_id=?")?
            .query_map([&asset.0], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,chrono::NaiveDateTime>(3)?,r.get::<_,String>(4)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        // Prepare all new values before any mutation; malformed quantities fail closed.
        let effects = effects
            .into_iter()
            .map(|(id, p, n, o)| {
                Ok((
                    id,
                    p,
                    Units::from_str(&n)?.rescale(old, scale)?.to_string(),
                    o,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let observations = observations
            .into_iter()
            .map(|(s, h, p, t, n)| {
                Ok((
                    s,
                    h,
                    p,
                    t,
                    Units::from_str(&n)?.rescale(old, scale)?.to_string(),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        tx.execute("DELETE FROM transaction_effects WHERE position_id IN (SELECT position_id FROM positions WHERE asset_id=?)", [&asset.0])?;
        for (id, p, n, o) in effects {
            tx.execute(
                "INSERT INTO transaction_effects VALUES (?,?,?,?)",
                params![id, p, n, o],
            )?;
        }
        tx.execute("DELETE FROM balance_observations WHERE position_id IN (SELECT position_id FROM positions WHERE asset_id=?)", [&asset.0])?;
        for (s, h, p, t, n) in observations {
            tx.execute(
                "INSERT INTO balance_observations VALUES (?,?,?,?,?)",
                params![s, h, p, t, n],
            )?;
        }
        tx.execute(
            "UPDATE asset_scales SET decimals=? WHERE asset_id=?",
            params![i32::from(scale), asset.0],
        )?;
        let portfolios = tx.prepare("SELECT DISTINCT a.portfolio_id FROM accounts a JOIN positions p ON a.id=p.account_id WHERE p.asset_id=?")?
            .query_map([&asset.0], |r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
        for portfolio in portfolios {
            crate::ui_metadata::bump_revision(&tx, &PortfolioId(portfolio), "ledger")?;
        }
        tx.commit()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PositionRole, Transaction, TxnEffect};
    #[test]
    fn widening_preserves_every_amount_observation_identity_and_other_asset() -> Result<()> {
        let store = Store::in_memory()?;
        let time = "2024-01-01T00:00:00Z".parse()?;
        let asset = AssetId::str("DOT");
        let mut positions = Vec::new();
        let mut outputs = Vec::new();
        let mut observations = Vec::new();
        for (id, role) in [
            ("NEXO:DOT", PositionRole::Asset),
            ("BINANCE:spot:DOT", PositionRole::Asset),
            ("NEXO-CREDIT:DOT", PositionRole::Liability),
            (
                "BINANCE:external:fees:DOT",
                PositionRole::ExternalCounterparty,
            ),
        ] {
            positions.push((id.into(), asset.clone(), 8, role));
            let amount = Units::from_decimal("123456789012345678901234567890.12345678", 8)?;
            outputs.push(TxnEffect {
                datetime: time,
                position_id: id.into(),
                amount: amount.clone(),
            });
            observations.push(crate::PositionBalance {
                position_id: id.into(),
                datetime: time,
                amount,
            });
        }
        positions.push((
            "NEXO:BTC".into(),
            AssetId::str("BTC"),
            8,
            PositionRole::Asset,
        ));
        outputs.push(TxnEffect {
            datetime: time,
            position_id: "NEXO:BTC".into(),
            amount: 5.into(),
        });
        store.save_file_import_with_observations(
            "fixture",
            "hash",
            &positions,
            &[(
                "original".into(),
                Transaction {
                    datetime: time,
                    inputs: vec![],
                    outputs,
                },
            )],
            &observations,
            |_, _| {},
        )?;
        let before = store.latest_balances()?;
        let source = store.balance_observations("fixture", "hash")?;
        let identities = store.position_metadata()?;
        assert!(store.widen_asset_scale(&asset, 10)?);
        for old in before {
            let new = store
                .latest_balances()?
                .into_iter()
                .find(|b| b.position_id == old.position_id)
                .unwrap();
            let is_dot = old.position_id.0 != "NEXO:BTC";
            assert_eq!(
                old.amount.decimal_string(8),
                new.amount
                    .rescale(if is_dot { 10 } else { 8 }, 8)?
                    .decimal_string(8)
            );
        }
        for (old, new) in source
            .iter()
            .zip(store.balance_observations("fixture", "hash")?)
        {
            assert_eq!(old.position_id, new.position_id);
            assert_eq!(old.datetime, new.datetime);
            assert_eq!(old.amount, new.amount.rescale(10, 8)?);
        }
        assert!(store.has_imported_file("fixture", "hash")?);
        assert_eq!(
            identities
                .iter()
                .map(|(p, a, _, r)| (p, a, r))
                .collect::<Vec<_>>(),
            store
                .position_metadata()?
                .iter()
                .map(|(p, a, _, r)| (p, a, r))
                .collect::<Vec<_>>()
        );
        assert!(!store.widen_asset_scale(&asset, 10)?);
        assert!(store.widen_asset_scale(&asset, 8).is_err());
        store.connection.execute(
            "UPDATE transaction_effects SET amount='malformed' WHERE position_id='NEXO:DOT'",
            [],
        )?;
        assert!(store.widen_asset_scale(&asset, 12).is_err());
        assert_eq!(store.asset_scale(&asset)?, 10);
        assert_eq!(
            store.balance_observations("fixture", "hash")?[0].amount,
            source[0].amount.rescale(8, 10)?
        );
        let original = source
            .iter()
            .find(|b| b.position_id.0 == "NEXO:DOT")
            .unwrap()
            .amount
            .rescale(8, 10)?;
        store.connection.execute(
            "UPDATE transaction_effects SET amount=? WHERE position_id='NEXO:DOT'",
            [original.to_string()],
        )?;
        let evidence = crate::ui_metadata::BalanceEvidence {
            id: "immutable".into(),
            account: "legacy:nexo".into(),
            position: Some("NEXO:DOT".into()),
            asset: asset.clone(),
            units: original,
            scale: 10,
            basis: crate::ui_metadata::BalanceBasis::Total,
            cutoff: time,
            complete: true,
            independent: true,
            evidence: crate::ui_metadata::SourceEvidence {
                source: "fixture".into(),
                reference: "capture".into(),
                observed_at: time,
                updated_at: time,
            },
        };
        store.save_balance_evidence(&evidence)?;
        let before_guard = store.latest_balances()?;
        assert!(store.widen_asset_scale(&asset, 12).is_err());
        assert_eq!(store.asset_scale(&asset)?, 10);
        assert_eq!(before_guard, store.latest_balances()?);
        assert_eq!(
            store.balance_evidence(&"legacy:nexo".into())?,
            vec![evidence]
        );
        Ok(())
    }
}
