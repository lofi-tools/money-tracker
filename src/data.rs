use crate::binance::BinanceClient;
use crate::coingecko::COINGECKO;
use crate::models::{
    AllAssetPrices, Asset, ExternalId, ListAssets, ListPositions, ListProducts, Position, Product,
};
use itertools::Itertools;
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
    pub static ref ASSET_PRICES: Arc<RwLock<AllAssetPrices>> = Arc::new(RwLock::new(AllAssetPrices::new()));
}

pub fn fetch_assets() -> ListAssets {
    let mut assets = ListAssets::new();
    // TODO NEXT TIME fetch prices for each
    assets.insert(Asset::new("ETH").with_ext_id(ExternalId::new(&**COINGECKO, "ethereum")));
    assets.insert(Asset::new("DOT").with_ext_id(ExternalId::new(&**COINGECKO, "polkadot")));
    assets.insert(Asset::new("NEAR").with_ext_id(ExternalId::new(&**COINGECKO, "near")));
    assets.insert(Asset::new("CRO").with_ext_id(ExternalId::new(&**COINGECKO, "crypto-com-chain")));
    assets.insert(Asset::new("BNB").with_ext_id(ExternalId::new(&**COINGECKO, "binancecoin")));
    assets.insert(Asset::new("SOL").with_ext_id(ExternalId::new(&**COINGECKO, "solana")));
    assets.insert(Asset::new("ADA").with_ext_id(ExternalId::new(&**COINGECKO, "cardano")));
    assets.insert(Asset::new("MATIC").with_ext_id(ExternalId::new(&**COINGECKO, "matic-network")));
    assets.insert(Asset::new("AVAX").with_ext_id(ExternalId::new(&**COINGECKO, "avalanche-2")));
    assets.insert(Asset::new("ATOM").with_ext_id(ExternalId::new(&**COINGECKO, "cosmos")));
    assets.insert(Asset::new("UNI").with_ext_id(ExternalId::new(&**COINGECKO, "uniswap")));
    assets.insert(Asset::new("EGLD").with_ext_id(ExternalId::new(&**COINGECKO, "elrond-erd-2")));
    assets
}

#[deprecated]
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

// TODO move to positions Struct
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
