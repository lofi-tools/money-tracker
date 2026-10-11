//! Shared application boundary for CLI and future desktop hosts.
//! Provider adapters retain their existing mapping/cache/reconciliation behavior.
//! This crate does not yet implement UI startup policy or earnings projections.
pub mod adapters {
    pub mod binance;
    pub mod binance_vision;
    pub mod coingecko;
    pub mod evm;
    pub mod fallback_prices;
    pub mod kraken;
    pub mod nexo;
}
pub mod dto;
mod service;
pub use service::Application;
