//! Historical data types for tracking price and position history.

use crate::types::{AssetId, Position};
use chrono::{DateTime, Utc};

/// A single point in asset price history
#[derive(Debug, Clone)]
pub struct AssetPricePoint {
    pub datetime: DateTime<Utc>,
    pub asset_id: AssetId,
    pub vs_asset_id: AssetId,
    pub price: f64,
}

/// Collection of asset price points over time
#[derive(Debug, Clone)]
pub struct AssetPriceHistory {
    pub points: Vec<AssetPricePoint>,
}

/// Collection of position snapshots over time
#[derive(Debug, Clone)]
pub struct PositionHistory {
    pub points: Vec<Position>,
}
