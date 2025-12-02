use crate::types::{ProviderId, Transaction, UserPosition};
use crate::{AssetId, CollectTxnData, PositionId};
use chrono::{DateTime, Utc};

/// Provider trait with name method (object-safe)
pub trait Issuer2: std::fmt::Debug {
    fn name(&self) -> &'static str;
}

/// Provider trait with static name method (not object-safe)
pub trait Issuer3 {
    fn name() -> ProviderId;
}

pub trait Issuer4 {
    const NAME: ProviderId;
}

/// Core provider interface for fetching positions and transactions
#[async_trait::async_trait]
pub trait IsProvider {
    /// Returns the unique identifier for this provider
    fn provider_id(&self) -> ProviderId; // &self for object-safety
    // const PROVIDER_ID: ProviderId;

    /// Fetches all positions from the provider
    async fn fetch_positions(&self) -> anyhow::Result<Vec<UserPosition>> {
        let data = self.fetch_all_txn_data().await?;
        Ok(data.positions)
    }

    /// Fetches all transactions from the provider
    async fn fetch_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let data = self.fetch_all_txn_data().await?;
        Ok(data.transactions)
    }

    async fn fetch_all_txn_data(&self) -> anyhow::Result<CollectTxnData>;
}

/// For types that have an associated account ID
pub trait HasPositionId {
    /// Returns the account ID associated with this type
    fn account_id(&self) -> PositionId;
}
pub trait HasDateTime {
    /// Returns the datetime associated with this type
    fn datetime(&self) -> DateTime<Utc>;
}
pub trait HasAssetId {
    /// Returns the asset ID associated with this type
    fn asset_id(&self) -> AssetId;
}
pub trait HasAmount {
    /// Returns the amount associated with this type
    fn amount(&self) -> f64;
}

pub trait IsTransaction: HasDateTime + HasAssetId + HasAmount {}
pub trait IsTxEffect: HasPositionId + HasDateTime + HasAssetId + HasAmount {}
