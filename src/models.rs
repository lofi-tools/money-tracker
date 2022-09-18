use crate::data::{ASSETS, PRODUCTS};
use derive_more::{Display, From};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Asset {
    pub id: AssetId,
    pub external_ids: HashMap<ProviderId, ExternalId>,
}
impl Asset {
    pub fn new(id: &str) -> Self {
        Asset {
            id: AssetId::from(id),
            external_ids: HashMap::new(),
        }
    }
    pub fn with_ext_id(mut self, ext_id: ExternalId) -> Self {
        self.external_ids.insert(ext_id.provider_id.clone(), ext_id);
        self
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Display)]
pub struct AssetId(pub String);
impl From<&str> for AssetId {
    fn from(s: &str) -> Self {
        AssetId(s.to_string())
    }
}
pub struct ListAssets {
    by_id: HashMap<AssetId, Asset>,
    by_external_id: HashMap<ExternalId, Asset>,
}
impl ListAssets {
    pub fn new() -> Self {
        ListAssets {
            by_id: HashMap::new(),
            by_external_id: HashMap::new(),
        }
    }
    #[deprecated] // use insert instead
    pub fn insert_id(&mut self, asset_id: &str) {
        self.by_id.insert(
            AssetId(asset_id.to_string()),
            Asset {
                id: AssetId(asset_id.to_string()),
                external_ids: HashMap::new(),
            },
        );
        // TODO insert external, match to internal
    }
    pub fn insert(&mut self, asset: Asset) {
        // TODO NEXT TIME inset assets with external IDs
        self.by_id.insert(asset.id.clone(), asset.clone());
        for (_provider_id, ext_id) in asset.external_ids.iter() {
            self.by_external_id.insert(ext_id.clone(), asset.clone());
        }
    }
    pub fn get(&'static self, id: &AssetId) -> Option<&'static Asset> {
        self.by_id.get(id)
    }
}

//

#[derive(Debug, Clone, Display)]
#[display(fmt = "{id}")]
pub struct Product {
    pub id: ProductId,
    pub asset_id: AssetId,
    pub apy: f64,
}
impl Product {
    pub fn asset(&self) -> &'static Asset {
        let asset = ASSETS.get(&self.asset_id).unwrap();
        asset
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Display)]
#[display(fmt = "{}", self.0)]
pub struct ProductId(pub ExternalId);

#[derive(Debug)]
pub struct ListProducts {
    pub map: HashMap<ProductId, Product>,
}
impl ListProducts {
    pub fn new() -> Self {
        ListProducts {
            map: HashMap::new(),
        }
    }
    pub fn insert(&mut self, product: Product) {
        self.map.insert(product.id.clone(), product);
    }
    pub fn get(&self, id: &ProductId) -> Option<&Product> {
        self.map.get(id)
    }
}

//

#[derive(Debug, Clone, Display)]
#[display(fmt = "{}", id)]
pub struct Position {
    pub id: PositionId,
    // pub product: &'a Product<'a>,
    pub product_id: ProductId,
    pub amount: f64,
    pub start_date: u64, // UTC timestamp
    pub end_date: u64,   // UTC timestamp
}
impl Position {
    pub fn product(&self) -> Option<Product> {
        let lock = PRODUCTS.read().unwrap();
        let p = lock.get(&self.product_id);
        p.cloned()
    }
}
#[derive(PartialEq, Eq, Hash, Debug, Clone, Display)]
#[display(fmt = "{}", self.0)]
pub struct PositionId(pub ExternalId);

pub struct ListPositions {
    pub by_id: HashMap<PositionId, Position>,
}
impl ListPositions {
    pub fn new() -> Self {
        ListPositions {
            by_id: HashMap::new(),
        }
    }
    pub fn insert(&mut self, pos: Position) {
        self.by_id.insert(pos.id.clone(), pos);
    }
    pub fn get<'a>(&'a self, id: &PositionId) -> Option<&'a Position> {
        self.by_id.get(id)
    }
}

//

#[derive(PartialEq, Eq, Hash, Debug, Clone, Display)]
#[display(fmt = "{provider_id}:{val}")]
pub struct ExternalId {
    pub provider_id: ProviderId,
    pub val: ExternalIdVal,
}
impl ExternalId {
    pub fn new(provider_id: &str, id: impl Into<ExternalIdVal>) -> Self {
        ExternalId {
            provider_id: ProviderId::from(provider_id),
            val: id.into(),
        }
    }
}
#[derive(PartialEq, Eq, Hash, Debug, Clone, Display)]
#[display(fmt = "{}", self.0)]
pub enum ExternalIdVal {
    U64(u64),
    String(String),
}
impl From<&String> for ExternalIdVal {
    fn from(t: &String) -> Self {
        ExternalIdVal::String(t.to_string())
    }
}
impl From<&str> for ExternalIdVal {
    fn from(t: &str) -> Self {
        ExternalIdVal::String(t.to_string())
    }
}
impl From<u64> for ExternalIdVal {
    fn from(u: u64) -> Self {
        ExternalIdVal::U64(u)
    }
}

//

#[derive(PartialEq, Eq, Hash, Debug, Clone, Display)]
#[display(fmt = "{}", self.0)]
pub struct ProviderId(pub String);
impl<T: AsRef<str>> From<T> for ProviderId {
    fn from(t: T) -> Self {
        ProviderId(t.as_ref().to_string())
    }
}
// TODO provider
