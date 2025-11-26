use coingecko_client::{payloads::CurrentPriceReq, CoingeckoClient};
use serde::Serialize;

use crate::models::{history::AssetPricePoint, AssetId};

pub const PROVIDER_ID_COINGECKO: &str = "coingecko";

pub struct CoinGeckoSvc {
    pub api_client: CoingeckoClient,
}
impl CoinGeckoSvc {
    pub fn service_id() -> &'static str {
        PROVIDER_ID_COINGECKO
    }
    pub fn new() -> anyhow::Result<Self> {
        Ok(CoinGeckoSvc {
            api_client: CoingeckoClient::new()?,
        })
    }

    pub async fn fetch_current_prices(&self) -> anyhow::Result<Vec<AssetPricePoint>> {
        let resp = self
            .api_client
            .fetch_current_prices(CurrentPriceReq::default())
            .await?;

        let prices = resp
            .into_iter()
            .map(|p| AssetPricePoint {
                asset_id: AssetId::from_coingecko(&p.asset_id),
                vs_asset_id: AssetId::from_coingecko(&p.vs_asset_id),
                price: p.price,
                datetime: p.time,
            })
            .collect::<Vec<AssetPricePoint>>();

        Ok(prices)
    }
}

impl AssetId {
    fn from_coingecko(coingecko_asset: &str) -> Self {
        match coingecko_asset {
            "ETH" => AssetId::Eth,
            _ => AssetId::unknown(coingecko_asset),
        }
    }
}

pub mod old {
    // lazy_static::lazy_static! {
    //     pub static ref COINGECKO: ProviderId = ProviderId("coingecko".to_string());
    //     pub static ref API_BASE: Url = Url::parse("https://api.coingecko.com/api/v3/").unwrap();
    // }

    use super::*;

    #[derive(Debug, Serialize)]
    pub struct CurrentPriceReq {
        ids: String,
        vs_currencies: String,
    }
    impl CurrentPriceReq {
        // fn new() -> Self {
        //     CurrentPriceReq {
        //         ids: ASSETS
        //             .by_id
        //             .iter()
        //             .fold(Vec::new(), |mut acc, (_a_id, a)| {
        //                 if let Some(cg_id) = a.external_ids.get(&COINGECKO) {
        //                     acc.push(cg_id.to_string());
        //                 }
        //                 acc
        //             })
        //             .join(","),
        //         vs_currencies: "usd".to_string(),
        //     }
        // }
        // pub async fn send(self) -> Result<CurrentPriceResp, anyhow::Error> {
        //     let client = reqwest::Client::new();
        //     client
        //         .get(API_BASE.join("simple/price")?)
        //         .query(&self)
        //         .get_json::<CurrentPriceResp>()
        //         .await
        // }
        // pub async fn fetch_prices() -> Result<CurrentPriceResp, anyhow::Error> {
        //     let resp = CurrentPriceReq::new().send().await?;
        //     Ok(resp)
        // }
    }

    // pub async fn fetch_prices() -> Result<(), anyhow::Error> {
    //     // TODO ids from ASSETS
    //     let asset_cg_ids = ASSETS.by_id.iter().fold(Vec::new(), |mut acc, (a_id, a)| {
    //         if let Some(cg_id) = a.external_ids.get(&COINGECKO) {
    //             acc.push(cg_id.to_string());
    //         }
    //         acc
    //     });
    //     dbg!(asset_cg_ids);

    //     // let client = reqwest::Client::new();
    //     // let resp = client
    //     //     .get(API_BASE.join("simple/price")?)
    //     //     .query(&[["ids"], ["vs_currencies", "usd"]])
    //     //     .get_json::<CurrentPriceResp>()
    //     //     .await?;
    //     // let req = bld.try_clone().unwrap();
    //     // let req = req.build().unwrap();
    //     // let url = req.url().to_string();
    //     // dbg!(url);

    //     // bld.send().await?;

    //     // dbg!(resp);

    //     todo!()
    // }

    // #[derive(thiserror::Error, Debug)]
    // pub enum CoinGeckoErr {
    //     #[error("Url parse err: {0}")]
    //     UrlParseErr(#[from] String),
    // }

    // pub async fn get<T: DeserializeOwned>(bld: RequestBuilder) -> Result<T, anyhow::Error> {
    //     let resp = bld.send().await?;
    //     if !resp.status().is_success() {
    //         println!("ERR: {:?}", resp);
    //     } else {
    //         let parsed: T = resp.json().await?;
    //     }

    //     todo!()
    // }
}
