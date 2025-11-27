use crate::traits::Issuer3;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxEffect {
    /// The account affected by this effect (e.g. "Binance", "WalletA")
    pub account_id: AccountId,
    /// The change in balance. Positive for debit (increase?), Negative for credit (decrease?)
    /// OR: In accounting, Debit is usually positive (assets increase), Credit is negative (assets decrease).
    /// The design doc says: "Outflow: Negative amount", "Inflow: Positive amount".
    pub amount: Decimal,
    pub datetime: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// List of effects that represent money leaving an account (debits/outflows).
    pub inputs: Vec<TxEffect>,
    /// List of effects that represent money entering an account (credits/inflows).
    pub outputs: Vec<TxEffect>,
    /// The time the transaction occurred.
    pub datetime: DateTime<Utc>,
}

/// Database structure containing all domain entities
pub struct Db {
    pub assets: HashMap<AssetId, AssetId>,
    pub positions: HashMap<PositionId, Position>,
    pub products: HashMap<ProductId, Product>,
    pub transactions: HashMap<TransactionId, Transaction>,
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
        self.positions.insert(position.id.clone(), position.clone());
    }
}

/// Provider identifier
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProviderId(pub String);

impl From<&str> for ProviderId {
    fn from(name: &str) -> Self {
        ProviderId(name.to_string())
    }
}

/// Asset identifier - represents known and unknown assets
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetId {
    Eth,
    Unknown(String),
}

impl AssetId {
    pub fn unknown(name: &str) -> Self {
        println!("Unknown asset: {}", name);
        AssetId::Unknown(name.to_string())
    }
}

/// External asset ID from a specific provider
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

/// Asset with its cross-provider mappings
#[derive(Debug, Clone)]
pub struct Asset {
    pub id: AssetId,
    pub chain_id: String,
    pub external_ids: HashMap<ProviderId, ExternalAssetId>,
}

impl Asset {
    pub fn with_ext_id(mut self, ext_id: ExternalAssetId) -> Self {
        self.external_ids.insert(ext_id.issuer_id(), ext_id);
        self
    }

    pub fn merge(&mut self, other: &Self) -> &mut Self {
        self.external_ids.extend(other.external_ids.clone());
        self
    }
}

/// Account identifier combining provider and asset
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId {
    pub provider: ProviderId,
    pub asset: AssetId,
}

impl AccountId {
    pub fn new(provider: ProviderId, asset: AssetId) -> Self {
        AccountId { provider, asset }
    }
}

/// Position identifier
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PositionId(pub String);

impl<S: AsRef<str>> From<S> for PositionId {
    fn from(name: S) -> Self {
        PositionId(name.as_ref().to_string())
    }
}

/// Financial position representing a staked or invested amount
#[derive(Debug, Clone)]
pub struct Position {
    pub id: PositionId,
    pub product_id: ProductId,
    pub amount: f64,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Collection of all positions
pub struct AllPositions {
    pub positions: HashMap<PositionId, Position>,
}

/// Product identifier
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProductId(pub String);

impl<S: AsRef<str>> From<S> for ProductId {
    fn from(name: S) -> Self {
        ProductId(name.as_ref().to_string())
    }
}

/// Investment product offering
#[derive(Debug, Clone)]
pub struct Product {
    pub id: ProductId,
    pub asset_id: AssetId,
    pub apy: f64,
}

/// Collection of all products
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

/// Transaction identifier
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TransactionId(pub String);

impl<S: AsRef<str>> From<S> for TransactionId {
    fn from(name: S) -> Self {
        TransactionId(name.as_ref().to_string())
    }
}
