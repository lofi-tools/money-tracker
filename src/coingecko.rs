use std::collections::HashMap;

use crate::data::ASSETS;
use crate::models::ProviderId;
use anyhow::anyhow;
use reqwest::header::ACCEPT;
use reqwest::{RequestBuilder, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

lazy_static::lazy_static! {
    pub static ref COINGECKO: ProviderId = ProviderId("coingecko".to_string());
    pub static ref API_BASE: Url = Url::parse("https://api.coingecko.com/api/v3/").unwrap();
}

#[derive(Debug, Serialize)]
pub struct CurrentPriceReq {
    ids: String,
    vs_currencies: String,
}
impl CurrentPriceReq {
    fn new() -> Self {
        CurrentPriceReq {
            ids: ASSETS
                .by_id
                .iter()
                .fold(Vec::new(), |mut acc, (a_id, a)| {
                    if let Some(cg_id) = a.external_ids.get(&COINGECKO) {
                        acc.push(cg_id.to_string());
                    }
                    acc
                })
                .join(","),
            vs_currencies: "usd".to_string(),
        }
    }
    pub async fn send(self) -> Result<CurrentPriceResp, anyhow::Error> {
        let client = reqwest::Client::new();
        client
            .get(API_BASE.join("simple/price")?)
            .query(&self)
            .get_json::<CurrentPriceResp>()
            .await
    }
    pub async fn fetch_prices() -> Result<(), anyhow::Error> {
        let resp = CurrentPriceReq::new().send().await?;
        dbg!(resp);
        // TODO store prices

        todo!()
    }
}
#[derive(Deserialize, Debug)]
pub struct CurrentPriceResp(HashMap<String, HashMap<String, f64>>);

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
