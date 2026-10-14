//! Fetched display evidence, separate from manual displays and protected ledger scales.
use crate::{AssetId, Store};
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use duckdb::params;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub asset_id: AssetId,
    pub chain_id: u64,
    pub contract: String,
    pub name: Option<String>,
    pub ticker: Option<String>,
    pub icon_url: Option<String>,
    pub icon_data: Option<String>,
    pub decimals: Option<u8>,
    pub rpc_decimals: Option<u8>,
    pub coingecko_id: Option<String>,
    pub fetched_at: Option<DateTime<Utc>>,
    /// Source URL -> bounded response evidence. Never contains credentials.
    pub sources: BTreeMap<String, serde_json::Value>,
    pub warnings: Vec<String>,
}
impl Store {
    pub fn token_metadata(&self, id: &AssetId) -> Result<Option<TokenMetadata>> {
        let mut stmt = self
            .connection
            .prepare("SELECT payload FROM token_metadata WHERE id=?")?;
        let mut rows = stmt.query([&id.0])?;
        rows.next()?
            .map(|r| -> Result<_> { Ok(serde_json::from_str(&r.get::<_, String>(0)?)?) })
            .transpose()
    }
    pub fn save_token_metadata(&self, value: &TokenMetadata) -> Result<()> {
        ensure!(
            matches!(value.chain_id, 1 | 137) && valid_contract(&value.contract),
            "unsupported or invalid token locator"
        );
        ensure!(
            value.asset_id.0 == format!("eip155:{}/erc20:{}", value.chain_id, value.contract),
            "token identity mismatch"
        );
        // Metadata is evidence only; it never registers or changes a scale.
        let _ = self.asset_scale(&value.asset_id)?;
        self.connection.execute("INSERT INTO token_metadata VALUES (?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload", params![value.asset_id.0, serde_json::to_string(value)?])?;
        Ok(())
    }
}
pub fn valid_contract(s: &str) -> bool {
    s.len() == 42
        && s.starts_with("0x")
        && s[2..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
