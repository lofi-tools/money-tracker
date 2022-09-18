use std::collections::HashMap;

use anyhow::anyhow;
use reqwest::header::ACCEPT;
use reqwest::{RequestBuilder, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

pub const COINGECKO: &str = "coingecko";
lazy_static::lazy_static! {
    pub static ref API_BASE: Url = Url::parse("https://api.coingecko.com/api/v3/").unwrap();
}

// pub struct CoinGecko {
//     httpc: http::Client,
// }

#[derive(Debug)]
pub struct CurrentPriceReq {
    ids: String,
    vs_currencies: String,
}
#[derive(Deserialize, Debug)]
pub struct CurrentPriceResp(HashMap<String, HashMap<String, f64>>);

pub async fn fetch_prices() -> Result<(), anyhow::Error> {
    // let prices_resp: PricesResp = MarketChartAlltimeReq {
    //     currency: "bitcoin",
    //     vs_currency: "usd",
    // }
    // .get()
    // .await?;

    let client = reqwest::Client::new();
    let resp = client
        .get(API_BASE.join("simple/price")?)
        .query(&[["ids", "ethereum"], ["vs_currencies", "usd"]])
        .get_json::<CurrentPriceResp>()
        .await?;
    // let req = bld.try_clone().unwrap();
    // let req = req.build().unwrap();
    // let url = req.url().to_string();
    // dbg!(url);

    // bld.send().await?;

    dbg!(resp);

    todo!()
}

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

#[async_trait::async_trait]
pub trait RequestBuilderExt {
    async fn get_json<T: DeserializeOwned>(self) -> Result<T, anyhow::Error>;
}
#[async_trait::async_trait]
impl RequestBuilderExt for RequestBuilder {
    async fn get_json<T: DeserializeOwned>(self) -> Result<T, anyhow::Error> {
        let resp = self.header(ACCEPT, "application/json").send().await?;

        if !resp.status().is_success() {
            println!("ERR: {:?}", resp);
            Err(anyhow!("{}", resp.text().await?.to_string()))
        } else {
            let parsed: T = resp.json().await?;
            Ok(parsed)
        }
    }
}
