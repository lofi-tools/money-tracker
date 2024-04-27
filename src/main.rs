#![feature(map_try_insert)] // for try_insert in models::AssetPrice
mod data;
mod models;
mod utils;
// mod actor_db;

use binance_client::BinanceClient;
use data::{POSITIONS, PRODUCTS};

use crate::adapters::coingecko;
pub mod adapters {
    pub mod binance;
    pub mod coingecko;
}

// TODO - estimate value, epy
// TODO - binance: other assets
// TODO - AAX ??
// TODO - NEXO ??
// TODO -

// GOALS - total value estimate
// GOALS - earn per year estimate (per asset breakdown of epy)
// GOALS - idle assets
// GOALS - movable assets (how much and when ??)
// GOALS - plot principal, interest, income
// GOALS -

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;
    std::env::set_var("RUST_BACKTRACE", "1");

    // // actor_db stuff
    // {
    //     let db = DB::spawn();
    // }

    let _bc = BinanceClient::new();
    // bc.list_staking_positions().await?;
    data::fetch_assets();
    data::fetch_binance().await?;

    println!("PRODUCTS:");
    for _product in PRODUCTS.read().unwrap().map.iter() {
        // println!("{}", product.1);
    }
    println!("POSITIONS:");
    for _pos in POSITIONS.read().unwrap().by_id.iter() {
        // println!("{}", pos.1);
    }

    data::positions_groupby_currency();

    coingecko::CurrentPriceReq::fetch_prices().await?;

    Ok(())
}
