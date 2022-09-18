use crate::binance::BinanceClient;
use crate::coingecko::COINGECKO;
use crate::models::{
    Asset, ExternalId, ListAssets, ListPositions, ListProducts, Position, Product,
};
use itertools::Itertools;
use std::collections::HashMap;
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
    assets.insert(Asset::new("ETH").with_ext_id(ExternalId::new(COINGECKO, "ethereum")));
    assets.insert(Asset::new("DOT")); // TODO NEXT TIME coingecko asset ids, then fetch prices for each
    assets.insert(Asset::new("NEAR"));
    assets.insert(Asset::new("CRO"));
    assets.insert(Asset::new("BNB"));
    assets.insert(Asset::new("SOL"));
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

pub async fn fetch_binance() {
    // fetch from Binance, transform into positions + products
    let bc = BinanceClient::new();
    let got_positions = bc.list_staking_positions().await.unwrap();

    let mut mut_products = PRODUCTS.write().unwrap();
    let mut mut_positions = POSITIONS.write().unwrap();
    for binance_pos in got_positions.iter() {
        mut_products.insert(Product::from(binance_pos));
        mut_positions.insert(Position::from(binance_pos));
    }
}

pub fn positions_groupby_currency() {
    let positions = POSITIONS.read().unwrap();
    // TODO NEXT TIME
    // let by_currency = pos_iter.fold(HashMap::new(), |mut map, (k, v)| {
    //     dbg!(k, v);
    //     todo!()
    // });
    let by_asset = positions
        .by_id
        .iter()
        .into_grouping_map_by(|(pos_k, pos)| pos.product().unwrap().asset().id.clone());
    // let sum = by_asset.aggregate(|sum,asset,(pos_id,pos)|{
    //     dbg!(pos.amount)
    // });
    let asset_sums = by_asset.fold(0_f64, |sum, asset, (_, pos)| sum + pos.amount);

    for (asset_id, sum) in asset_sums {
        println!("{asset_id}: {sum}");
    }
}
