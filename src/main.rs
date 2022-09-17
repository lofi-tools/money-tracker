#![feature(once_cell)]
// mod actor_db;
pub mod binance;
pub mod models;
mod utils;

use binance::BinanceClient;
use data::MUT_PRODUCTS;

// use crate::actor_db::DB;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;
    std::env::set_var("RUST_BACKTRACE", "1");

    // // actor_db stuff
    // {
    //     let db = DB::spawn();
    // }

    let bc = BinanceClient::new();
    // bc.list_staking_positions().await?;
    data::fetch_assets();
    data::fetch_positions().await;
    // TODO update Products, UserProducts with binance data

    dbg!(MUT_PRODUCTS.read().unwrap());

    Ok(())
}

mod data;
