#![feature(once_cell)]

mod binance;
use binance::BinanceClient;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;

    let bc = BinanceClient::new();
    bc.list_all_products().await?;
    // TODO update Products, UserProducts with binance data

    Ok(())
}

mod models {
    use crate::data::{AssetId, ProductId, ASSETS, PRODUCTS};
    #[derive(Debug)]
    pub struct Asset {
        pub id: String,
    }

    #[derive(Debug)]
    pub struct Product {
        pub id: String,
        pub asset_id: AssetId,
    }
    impl Product {
        pub fn asset(&self) -> &'static Asset {
            let asset = ASSETS.get(&self.asset_id).unwrap();
            asset
        }
    }

    #[derive(Debug)]
    pub struct UserPosition {
        // pub product: &'a Product<'a>,
        pub product_id: ProductId,
        pub amount: f64,
    }
    impl UserPosition {
        pub fn product(&self) -> Option<&'static Product> {
            let p = PRODUCTS.get(&self.product_id);
            p
        }
    }
}

mod data {
    use crate::models::{Asset, Product};
    use std::collections::HashMap;

    lazy_static::lazy_static! {
        pub static ref ASSETS: ListAssets = fetch_assets();
        pub static ref PRODUCTS: ListProducts = fetch_products();
    }

    // pub const ASSETS: Lazy<ListAssets> = Lazy::new(|| fetch_assets());
    // pub const PRODUCTS: Lazy<ListProducts> = Lazy::new(|| fetch_products());

    fn fetch_assets() -> ListAssets {
        let mut assets = ListAssets::new();
        assets.insert("ETH");
        assets.insert("DOT");
        assets.insert("NEAR");
        assets.insert("CRO");
        assets
    }

    fn fetch_products() -> ListProducts {
        let mut products = ListProducts::new();
        // TODO fetch from Binance

        // let assets: &ListAssets = &*ASSETS;
        // let eth: &Asset = (*ASSETS).get("ETH").unwrap();
        // static asset: Asset = ASSETS.get("ETH").unwrap();
        // let product: Product<'static> = Product {
        //     id: "TODO".to_string(),
        //     asset: asset,
        // };
        // products.insert(product);
        products
    }

    #[derive(PartialEq, Eq, Hash, Debug)]
    pub struct AssetId(pub String);
    pub struct ListAssets {
        map: HashMap<AssetId, Asset>,
    }
    impl ListAssets {
        pub fn new() -> Self {
            ListAssets {
                map: HashMap::new(),
            }
        }
        pub fn insert(&mut self, asset_id: &str) {
            self.map.insert(
                AssetId(asset_id.to_string()),
                Asset {
                    id: asset_id.to_string(),
                },
            );
        }
        pub fn get<'a>(&'a self, id: &AssetId) -> Option<&'a Asset> {
            self.map.get(id)
        }
    }

    #[derive(PartialEq, Eq, Hash, Debug)]
    pub struct ProductId(pub String);
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
            self.map.insert(ProductId(product.id.clone()), product);
        }
        pub fn get<'a>(&'a self, id: &ProductId) -> Option<&'a Product> {
            self.map.get(id)
        }
    }
}
