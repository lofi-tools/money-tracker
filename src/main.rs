#![feature(map_try_insert)] // for try_insert in models::AssetPrice
#![feature(impl_trait_in_assoc_type)] //
use crate::adapters::binance2::{traits::IsProvider, BinanceSvc};
use crate::adapters::coingecko;
use crate::models::Db;
use data::{POSITIONS, PRODUCTS};

pub mod adapters {
    pub mod binance;
    pub mod binance2;
    pub mod coingecko;
}
mod data;
mod models;

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
    dotenvy::dotenv_override().ok();
    std::env::set_var("RUST_BACKTRACE", "1");

    let mut db = Db::new();

    let providers: Vec<Box<dyn IsProvider>> = vec![Box::new(BinanceSvc::new()?)];
    for provider in providers {
        let positions = provider.fetch_positions().await?;
        for position in positions {
            dbg!(position);
        }
    }

    // bc.list_staking_positions().await?;
    // data::fetch_assets();
    // data::fetch_binance().await?;

    // println!("PRODUCTS:");
    // for _product in PRODUCTS.read().unwrap().map.iter() {
    //     // println!("{}", product.1);
    // }
    // println!("POSITIONS:");
    // for _pos in POSITIONS.read().unwrap().by_id.iter() {
    //     // println!("{}", pos.1);
    // }

    data::positions_groupby_currency();

    coingecko::CurrentPriceReq::fetch_prices().await?;

    Ok(())
}
