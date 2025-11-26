use self::traits::Issuer3;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod traits {
    use super::{Position, ProviderId, Transaction};

    pub trait Issuer2: std::fmt::Debug {
        fn name(&self) -> &'static str;
    }

    // not object-safe
    pub trait Issuer3 {
        fn name() -> &'static str;
    }

    #[async_trait::async_trait] // makes async trait also object-safe
    pub trait IsProvider {
        // TODO load assets, attach external_ids

        fn provider_id(&self) -> ProviderId;
        // fn list_products(&self) -> anyhow::Result<AllProducts>;
        async fn fetch_positions(&self) -> anyhow::Result<Vec<Position>>;

        async fn fetch_transactions(&self) -> anyhow::Result<Vec<Transaction>>;
    }
}

pub struct Db {
    pub assets: HashMap<AssetId, AssetId>,
    pub positions: HashMap<PositionId, Position>,
    pub products: HashMap<ProductId, Product>,
    pub transactions: HashMap<TransactionId, Transaction>,
    // TODO positionHistory
    // pub total_asset_hist: Vec<AssetOwnHistPoint>,
    // TODO AssetPriceHistory
    // pub asset_price_hist: Vec<AssetPrice>,
}
impl Db {
    pub fn new() -> Self {
        Db {
            assets: HashMap::new(),
            positions: HashMap::new(),
            products: HashMap::new(),
            transactions: HashMap::new(),
        }
    }
    fn upsert_position(&mut self, position: &Position) {
        // TODO match and upsert asset
        self.positions.insert(position.id.clone(), position.clone());
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProviderId(pub String);
impl From<&str> for ProviderId {
    fn from(name: &str) -> Self {
        ProviderId(name.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PositionId(pub String);
impl<S: AsRef<str>> From<S> for PositionId {
    fn from(name: S) -> Self {
        PositionId(name.as_ref().to_string())
    }
}

pub struct AllPositions {
    pub positions: HashMap<PositionId, Position>,
}

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct AssetId(pub String);
// impl<S: AsRef<str>> From<S> for AssetId {
//     fn from(name: S) -> Self {
//         AssetId(name.as_ref().to_string())
//     }
// }

#[derive(Debug, Clone)]
pub struct Asset {
    pub id: AssetId,
    pub chain_id: String,
    pub external_ids: HashMap<ProviderId, ExternalAssetId>,
}
impl Asset {
    // pub fn new(id: &str) -> Self {
    //     Asset {
    //         id: Asset::from(id),
    //         external_ids: HashMap::new(),
    //     }
    // }
    pub fn with_ext_id(mut self, ext_id: ExternalAssetId) -> Self {
        self.external_ids.insert(ext_id.issuer_id(), ext_id);
        self
    }
    pub fn merge(&mut self, other: &Self) -> &mut Self {
        self.external_ids.extend(other.external_ids.clone());
        self
    }
}

// TODO represent asset on different chains / providers
#[derive(Debug, Clone)]
pub enum AssetId {
    Eth,
    Unknown(String),
}
impl AssetId {
    pub fn unknown(name: &str) -> Self {
        println!("Unknown asset: {}", name);
        AssetId::Unknown(name.to_string())
    }
    // pub fn id(&self) -> AssetId {
    //     match self {
    //         Asset::Eth => AssetId("eth".to_string()),
    //         Asset::Unknown(name) => AssetId(format!("Unknown asset: {name}")),
    //     }
    // }
}

#[derive(Debug, Clone)]
pub struct ExternalAssetId {
    pub id: String,
    pub issuer_name: String,
    _asset: std::marker::PhantomData<AssetId>,
}
impl ExternalAssetId {
    pub fn new<Issuer: Issuer3>(id: &str) -> Self {
        ExternalAssetId {
            id: id.to_string(),
            issuer_name: Issuer::name().to_string(),
            _asset: std::marker::PhantomData,
        }
    }
    pub fn issuer_id(&self) -> ProviderId {
        ProviderId::from(self.issuer_name.as_str())
    }
}
impl PartialEq for ExternalAssetId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ExternalAssetId {}
impl std::hash::Hash for ExternalAssetId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.issuer_name.hash(state);
        self.id.hash(state);
    }
}

// pub struct AllAssets {
//     pub by_id: HashMap<AssetId, AssetId>,
//     // by_external_id: HashMap<ExternalAssetId, Asset>, // TODO use sqlite indexes instead of re-creating a database
// }
// impl AllAssets {
//     pub fn new() -> Self {
//         AllAssets {
//             by_id: HashMap::new(),
//             // by_external_id: HashMap::new(),
//         }
//     }
//     pub fn upsert(&mut self, new: AssetId) {
//         let merge_into_by_id = self
//             .by_id
//             .get_mut(&new.id)
//             .map(|existing| existing.merge(&new))
//             .cloned()
//             .unwrap_or(new.clone());
//         self.by_id.insert(new.id, merge_into_by_id);
//     }
//     pub fn get(&self, id: &AssetId) -> Option<&AssetId> {
//         self.by_id.get(id)
//     }
// }

#[derive(Debug, Clone)]
pub struct Product {
    pub id: ProductId,
    pub asset_id: AssetId,
    pub apy: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProductId(pub String);
impl<S: AsRef<str>> From<S> for ProductId {
    fn from(name: S) -> Self {
        ProductId(name.as_ref().to_string())
    }
}

pub struct AllProducts {
    pub products: HashMap<ProductId, Product>,
}
impl AllProducts {
    pub fn new() -> Self {
        AllProducts {
            products: HashMap::new(),
        }
    }
    pub fn insert(&mut self, product: Product) {
        self.products.insert(product.id.clone(), product);
    }
    pub fn get(&self, id: &ProductId) -> Option<&Product> {
        self.products.get(id)
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    pub id: PositionId,
    pub product_id: ProductId,
    pub amount: f64,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

pub mod history {
    use super::{AssetId, Position};
    use chrono::{DateTime, Utc};

    #[derive(Debug, Clone)]
    pub struct AssetPricePoint {
        pub datetime: DateTime<Utc>,
        pub asset_id: AssetId,
        pub vs_asset_id: AssetId,
        pub price: f64,
    }

    #[derive(Debug, Clone)]
    pub struct AssetPriceHistory {
        pub points: Vec<AssetPricePoint>,
    }

    #[derive(Debug, Clone)]
    pub struct PositionHistory {
        pub points: Vec<Position>,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TransactionId(pub String);
impl<S: AsRef<str>> From<S> for TransactionId {
    fn from(name: S) -> Self {
        TransactionId(name.as_ref().to_string())
    }
}
pub struct Transaction {
    pub id: TransactionId,
    pub datetime: DateTime<Utc>,
    pub inputs: Vec<TxInputOutput>,
    pub outputs: Vec<TxInputOutput>,
}

pub struct TxInputOutput {
    pub asset: AssetId,
    pub amount: f64, // TODO use accountable type (e.g. Decimal)
}

#[cfg(test)]
mod tests {
    use self::traits::Issuer3;
    use super::*;
    use crate::adapters::binance::BinanceSvc;

    pub struct Provider2 {}
    impl Issuer3 for Provider2 {
        fn name() -> &'static str {
            "provider_2"
        }
    }

    // #[test]
    // fn test_upsert() -> anyhow::Result<()> {
    //     let mut assets = AllAssets::new();
    //     assets.upsert(
    //         AssetId::new("asset_1")
    //             .with_ext_id(ExternalAssetId::new::<BinanceSvc>("binance:asset_1")),
    //     );
    //     assets.upsert(
    //         AssetId::new("asset_1").with_ext_id(ExternalAssetId::new::<Provider2>(
    //             "some_other_name_for_same_asset",
    //         )),
    //     );

    //     // TODO assert there's 1 asset with 2 ext ids

    //     let asset = assets.get(&AssetId::from("asset_1")).unwrap();
    //     assert_eq!(asset.external_ids.len(), 2);

    //     Ok(())
    // }
}
