use std::collections::HashMap;

use crate::{AssetId, Position, PositionId, Product, ProductId, Transaction, TransactionId};

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
