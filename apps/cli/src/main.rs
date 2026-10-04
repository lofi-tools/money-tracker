// #![feature(map_try_insert)] // for try_insert in models::AssetPrice
// use crate::adapters::binance::BinanceSvc;
// use crate::adapters::coingecko;
// use crate::adapters::nexo::NexoSvc;
use crate::adapters::nexo::NexoSvc;
use adapters::coingecko::CoinGeckoSvc;
use clap::Parser;
use cli::{Args, Config};
use lib_core::Db;
use lib_core::traits::IsProvider;

pub mod adapters {
    pub mod binance;
    pub mod coingecko;
    pub mod nexo;
}
mod cli;
mod models;

use std::cell::LazyCell;
use std::path::PathBuf;

const CACHE_DIR: LazyCell<PathBuf> = LazyCell::new(|| {
    if let Ok(p) = std::env::var("CRYPTODASH_CACHE_DIR") {
        return PathBuf::from(p);
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cryptodash")
});

const DATA_DIR: LazyCell<PathBuf> = LazyCell::new(|| {
    if let Ok(p) = std::env::var("CRYPTODASH_DATA_DIR") {
        return PathBuf::from(p);
    }
    // For now, use cache dir for data as requested
    PathBuf::from(&*CACHE_DIR)
});

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
    // dotenvy::dotenv_override().ok();
    let config = Config::from_env(Args::parse());
    // dbg!(args);
    std::process::exit(0);

    // std::env::set_var("RUST_BACKTRACE", "1");

    // let mut db = Db::new();

    let providers: Vec<Box<dyn IsProvider>> = vec![
        // Box::new(BinanceSvc::new()?)
        Box::new(NexoSvc::new(&config)?),
    ];
    for provider in providers {
        // let positions = provider.fetch_positions().await?;
        // for position in positions {
        //     dbg!(position);
        // }
        let txn_data = provider.fetch_all_txn_data().await?;
        dbg!(txn_data);
    }

    let coingecko_svc = CoinGeckoSvc::new()?;
    let prices = coingecko_svc.fetch_current_prices().await?;
    dbg!(prices);

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

    // data::positions_groupby_currency();

    // coingecko::CurrentPriceReq::fetch_prices().await?;

    Ok(())
}
