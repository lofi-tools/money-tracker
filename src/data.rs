use crate::binance::BinanceClient;
use crate::models::{ListAssets, ListPositions, ListProducts, Position, Product};
use std::sync::{Arc, RwLock};

lazy_static::lazy_static! {
    pub static ref ASSETS: ListAssets = fetch_assets();
    // pub static ref PRODUCTS: ListProducts = fetch_products();
    // pub static ref POSITIONS: ListPositions = tokio::task::block_in_place(move || {
    //     tokio::runtime::Handle::current().block_on(async move {
    //         fetch_positions().await
    //     })
    // });
    // pub static ref MUT_ASSETS: Arc<RwLock<ListAssets>> = Arc::new(RwLock::new(ListAssets::new()));
    pub static ref PRODUCTS: Arc<RwLock<ListProducts>> = Arc::new(RwLock::new(ListProducts::new()));
    pub static ref POSITIONS: Arc<RwLock<ListPositions>> = Arc::new(RwLock::new(ListPositions::new()));
}

pub fn fetch_assets() -> ListAssets {
    let mut assets = ListAssets::new();
    assets.insert("ETH");
    assets.insert("DOT");
    assets.insert("NEAR");
    assets.insert("CRO");
    assets.insert("BNB");
    assets.insert("SOL");
    assets
}

pub fn fetch_products() -> ListProducts {
    let mut products = ListProducts::new();
    // TODO fetch from Binance, populate products

    // let product: Product<'static> = Product {
    //     id: "TODO".to_string(),
    //     asset: asset,
    // };
    // products.insert(product);
    products
}

pub async fn fetch_binance() -> ListPositions {
    let mut all_positions = ListPositions::new();
    // TODO fetch from Binance, transform into positions + products
    let bc = BinanceClient::new();
    let got_positions = bc.list_staking_positions().await.unwrap();
    let mut mut_products = PRODUCTS.write().unwrap();
    let mut mut_positions = POSITIONS.write().unwrap();
    for binance_pos in got_positions.iter() {
        // dbg!(binance_pos);
        mut_products.insert(Product::from(binance_pos));
        mut_positions.insert(Position::from(binance_pos));
        // TODO insert position: grab ref to ListPosition and insert
    }
    // dbg!(&*PRODUCTS);

    all_positions
}
