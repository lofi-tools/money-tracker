//! Core domain types and traits for the crypto-tracker application.
//!
//! This library provides the foundational domain models and abstractions used
//! across all provider adapters and client libraries.

pub mod history;
pub mod traits;
pub mod types;

// Re-export commonly used types for convenience
pub use history::*;
pub use traits::*;
pub use types::*;
