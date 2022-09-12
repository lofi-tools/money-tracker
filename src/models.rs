use crate::data::{ASSETS, POSITIONS, PRODUCTS};
use derive_more::From;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Asset {
    pub id: String,
}
#[derive(PartialEq, Eq, Hash, Debug)]
pub struct AssetId(pub String);
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
    pub fn insert(&mut self, asset_id: &str) {
        self.by_id.insert(
            AssetId(asset_id.to_string()),
            Asset {
                id: asset_id.to_string(),
            },
        );
        // TODO insert external, match to internal
    }
    pub fn get(&'static self, id: &AssetId) -> Option<&'static Asset> {
        self.by_id.get(id)
    }
}

#[derive(Debug)]
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
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ProductId(pub ExternalId);

#[derive(Debug)]
pub struct ListProducts {
    map: HashMap<ProductId, Product>,
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
    pub fn get<'a>(&'a self, id: &ProductId) -> Option<&'a Product> {
        self.map.get(id)
    }
}

#[derive(Debug)]
pub struct Position {
    pub id: PositionId,
    // pub product: &'a Product<'a>,
    pub product_id: ProductId,
    pub amount: f64,
    pub start_date: u64, // UTC timestamp
    pub end_date: u64,   // UTC timestamp
    pub external_id: ExternalId,
}
impl Position {
    pub fn product(&self) -> Option<&'static Product> {
        let p = PRODUCTS.get(&self.product_id);
        p
    }
}
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct PositionId(pub ExternalId);
pub struct ListPositions {
    by_id: HashMap<PositionId, Position>,
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

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
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
#[derive(PartialEq, Eq, Hash, Debug, Clone, From)]
pub enum ExternalIdVal {
    U64(u64),
    String(String),
}
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct ProviderId(pub String);
impl<T: AsRef<str>> From<T> for ProviderId {
    fn from(t: T) -> Self {
        ProviderId(t.as_ref().to_string())
    }
}
// pub struct ExternalIds {
//     by_provider: HashMap<ProviderId, ExternalId>,
// }
// impl ExternalIds {
//     pub fn new() -> Self {
//         ExternalIds {
//             by_provider: HashMap::new(),
//         }
//     }
//     pub fn insert(&mut self, ext_id: ExternalId) {
//         self.by_provider.insert(ext_id.provider_id, ext_id);
//     }
// }
