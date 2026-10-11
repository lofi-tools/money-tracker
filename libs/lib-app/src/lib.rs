//! Shared application boundary for CLI and future desktop hosts.
//! Provider adapters retain their existing mapping/cache/reconciliation behavior.
//! Shared offline startup/settings and a managed DB worker serve future hosts.
//! Earnings projections and the frontend remain later stages.
pub mod adapters {
    pub mod binance;
    pub mod binance_vision;
    pub mod coingecko;
    pub mod evm;
    pub mod fallback_prices;
    pub mod kraken;
    pub mod nexo;
}
pub mod config;
pub mod dto;
mod service;
pub mod startup;
pub mod worker;
pub use service::Application;
