use crate::Issuer4;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxEffect {
    /// The account affected by this effect (e.g. "Binance", "WalletA")
    pub account_id: AccountId,
    /// The change in balance. Positive for debit (increase?), Negative for credit (decrease?)
    /// OR: In accounting, Debit is usually positive (assets increase), Credit is negative (assets decrease).
    /// The design doc says: "Outflow: Negative amount", "Inflow: Positive amount".
    pub amount: u64,
    pub datetime: DateTime<Utc>,
}

// inside a transaction, inputs and outputs must be balanced (sum of inputs == sum of outputs)
pub struct TxInput(pub TxEffect); // An input withdraws from an account
pub struct TxOutput(pub TxEffect); // An output deposits into an account

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// List of effects that represent money leaving an account (debits/outflows).
    pub inputs: Vec<TxEffect>,
    /// List of effects that represent money entering an account (credits/inflows).
    pub outputs: Vec<TxEffect>,
    /// The time the transaction occurred.
    pub datetime: DateTime<Utc>,
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
    pub issuer_name: ProviderId,
    _asset: std::marker::PhantomData<AssetId>,
}

impl ExternalAssetId {
    pub fn new<Issuer: Issuer4>(id: &str) -> Self {
        ExternalAssetId {
            id: id.to_string(),
            issuer_name: Issuer::NAME,
            _asset: std::marker::PhantomData,
        }
    }

    pub fn issuer_id(&self) -> ProviderId {
        self.issuer_name.clone()
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
    pub decimals: u8,
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
// impl From<String> for AccountId {
//     fn from(value: String) -> Self {
//         AccountId {
//             provider: todo!(),
//             asset: todo!(),
//         }
//     }
// }

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
    pub amount: u64,
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

/// Transaction identifier
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TransactionId(pub String);

impl<S: AsRef<str>> From<S> for TransactionId {
    fn from(name: S) -> Self {
        TransactionId(name.as_ref().to_string())
    }
}

/// Data collected from a provider
#[derive(Debug, Clone)]
pub struct CollectProviderData {
    pub provider_id: ProviderId,
    pub transactions: Vec<Transaction>,
    pub assets: Vec<Asset>,
    pub positions: Vec<Position>,
    pub products: Vec<Product>,
}
