//! Provider trait definitions for implementing external service adapters.

use crate::types::{Position, ProviderId, Transaction};

/// Provider trait with name method (object-safe)
pub trait Issuer2: std::fmt::Debug {
    fn name(&self) -> &'static str;
}

/// Provider trait with static name method (not object-safe)
pub trait Issuer3 {
    fn name() -> &'static str;
}

/// Core provider interface for fetching positions and transactions
#[async_trait::async_trait]
pub trait IsProvider {
    /// Returns the unique identifier for this provider
    fn provider_id(&self) -> ProviderId;

    /// Fetches all positions from the provider
    async fn fetch_positions(&self) -> anyhow::Result<Vec<Position>>;

    /// Fetches all transactions from the provider
    async fn fetch_transactions(&self) -> anyhow::Result<Vec<Transaction>>;
}
