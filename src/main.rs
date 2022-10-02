#![feature(once_cell)]
#![feature(map_try_insert)] // for try_insert in models::AssetPrice
mod binance;
mod data;
mod models;
mod utils;
// mod actor_db;

use binance::BinanceClient;
use data::{POSITIONS, PRODUCTS};
mod coingecko;

// TODO - estimate value, epy
// TODO - binance: other assets
// TODO - AAX ??
// TODO - NEXO ??
// TODO -

// GOALS - total value estimate
// GOALS - earn per year estimate (per asset breakdown of epy)
// GOALS - idle assets
// GOALS - movable assets (how much or when ??)
// GOALS -
// GOALS -

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
    data::fetch_binance().await;

    println!("PRODUCTS:");
    for product in PRODUCTS.read().unwrap().map.iter() {
        // println!("{}", product.1);
    }
    println!("POSITIONS:");
    for pos in POSITIONS.read().unwrap().by_id.iter() {
        // println!("{}", pos.1);
    }

    data::positions_groupby_currency();

    coingecko::CurrentPriceReq::fetch_prices().await?;

    Ok(())
}
