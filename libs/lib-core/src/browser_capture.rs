//! Atomic evidence ingestion. Captures never change principal, movements or scales.
use crate::{
    AccountId, PortfolioId, Store,
    ui_metadata::{RateEvidence, RateScope, bump_revision},
};
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use duckdb::params;
impl Store {
    pub fn nexo_captures(&self, account: &AccountId) -> Result<Vec<(String, String, String)>> {
        Ok(self.connection.prepare("SELECT content_hash,payload,result FROM nexo_browser_captures WHERE account_id=? ORDER BY observed_at,content_hash")?
            .query_map([&account.0], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<std::result::Result<Vec<_>,_>>()?)
    }
    pub fn commit_nexo_capture(
        &self,
        portfolio: &PortfolioId,
        account: &AccountId,
        hash: &str,
        id: &str,
        observed_at: DateTime<Utc>,
        payload: &str,
        result: &str,
        rates: &[RateEvidence],
    ) -> Result<bool> {
        ensure!(
            self.accounts(portfolio)?
                .iter()
                .any(|a| &a.id == account && a.provider_id.0 == "nexo"),
            "Nexo account not in portfolio"
        );
        let tx = self.connection.unchecked_transaction()?;
        let mut stmt=tx.prepare("SELECT content_hash,payload FROM nexo_browser_captures WHERE account_id=? AND (content_hash=? OR capture_id=?)")?;
        let old = stmt
            .query_map(params![account.0, hash, id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        if !old.is_empty() {
            ensure!(
                old.iter().all(|(h, p)| h == hash && p == payload),
                "capture identity conflict"
            );
            return Ok(false);
        }
        for rate in rates {
            ensure!(&rate.portfolio_id == portfolio, "rate portfolio mismatch");
            let RateScope::Subscription(id) = &rate.scope else {
                anyhow::bail!("capture rate must be term-specific")
            };
            ensure!(
                self.subscription(id)?
                    .is_some_and(|s| &s.account_id == account),
                "capture subscription account mismatch"
            );
            crate::ui_metadata::validate_scope(&tx, portfolio, &rate.scope)?;
            crate::ui_metadata::validate_rate(&rate.rate)?;
            crate::ui_metadata::validate_evidence(&rate.evidence)?;
            let json = serde_json::to_string(rate)?;
            tx.execute(
                "INSERT INTO ui_rate_evidence VALUES (?,?)",
                params![rate.id, json],
            )?;
        }
        tx.execute(
            "INSERT INTO nexo_browser_captures VALUES (?,?,?,?,?,?)",
            params![account.0, hash, id, observed_at, payload, result],
        )?;
        bump_revision(&tx, portfolio, "metadata")?;
        tx.commit()?;
        Ok(true)
    }
}
