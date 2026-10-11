//! Core domain types and traits for the crypto-tracker application.
//!
//! This library provides the foundational domain models and abstractions used
//! across all provider adapters and client libraries.

pub mod amount;
pub use amount::Units;
pub mod identity;
pub use identity::*;
pub mod history;
mod migrations;
pub mod storage;
pub mod traits;
pub mod types;

pub use history::*;
pub use storage::*;
pub use traits::*;
pub use types::*;
