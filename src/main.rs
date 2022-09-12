#![feature(once_cell)]
mod actor_db;
pub mod binance;
pub mod models;
mod utils;

use binance::BinanceClient;
use data::MUT_PRODUCTS;
use tokio::sync::mpsc;

use crate::actor_db::DB;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;
    std::env::set_var("RUST_BACKTRACE", "1");

    // actor_db stuff
    {
        let db = DB::spawn();
    }

    let bc = BinanceClient::new();
    // bc.list_staking_positions().await?;
    data::fetch_assets();
    data::fetch_positions().await;
    // TODO update Products, UserProducts with binance data

    dbg!(MUT_PRODUCTS.read().unwrap());

    Ok(())
}

mod data {
    use std::sync::{Arc, RwLock};

    use crate::{
        binance::BinanceClient,
        models::{ListAssets, ListPositions, ListProducts, Product},
    };

    lazy_static::lazy_static! {
        pub static ref ASSETS: ListAssets = fetch_assets();
        pub static ref PRODUCTS: ListProducts = fetch_products();
        pub static ref POSITIONS: ListPositions = tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move {
                fetch_positions().await
            })
        });
        pub static ref MUT_PRODUCTS: Arc<RwLock<ListProducts>> = Arc::new(RwLock::new(ListProducts::new()));
    }

    pub fn fetch_assets() -> ListAssets {
        let mut assets = ListAssets::new();
        assets.insert("ETH");
        assets.insert("DOT");
        assets.insert("NEAR");
        assets.insert("CRO");
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

    pub async fn fetch_positions() -> ListPositions {
        let mut all_positions = ListPositions::new();
        // TODO fetch from Binance, transform into positions + products
        let bc = BinanceClient::new();
        let got_positions = bc.list_staking_positions().await.unwrap();
        let mut mut_products = MUT_PRODUCTS.write().unwrap();
        for binance_pos in got_positions.iter() {
            // dbg!(binance_pos);
            mut_products.insert(Product::from(binance_pos));
            // TODO insert position: grab ref to ListPosition and insert
        }
        // dbg!(&*PRODUCTS);

        all_positions
    }
}
