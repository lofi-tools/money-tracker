#![allow(async_fn_in_trait)]
use anyhow::anyhow;
use api_client_utils::prelude::*;
use reqwest::header::ACCEPT;
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use self::payloads::{CurrentPriceReq, CurrentPriceResp};

// lazy_static::lazy_static! {
//     pub static ref COINGECKO: ProviderId = ProviderId("coingecko".to_string());
//     pub static ref API_BASE: Url = Url::parse("https://api.coingecko.com/api/v3/").unwrap();
// }

const PROVIDER_ID_COINGECKO: &str = "coingecko";

// TODO move to own lib
pub struct CoingeckoClient {
    pub base_url: String,
    pub http_client: reqwest::Client,
}
impl JsonApiClient for CoingeckoClient {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}
impl CoingeckoClient {
    pub fn new() -> anyhow::Result<Self> {
        Ok(CoingeckoClient {
            base_url: "https://api.coingecko.com/api/v3/".to_string(),
            http_client: reqwest::Client::new(),
        })
    }

    // TODO move methods to here
    pub async fn fetch_current_prices(
        &self,
        args: CurrentPriceReq,
    ) -> anyhow::Result<CurrentPriceResp> {
        let req = self.get("/simple/price").query(&args);
        let resp = req
            .recv_json::<CurrentPriceResp, CoingeckoErrResp>()
            .await?;
        Ok(resp)
    }
}

#[derive(Deserialize, thiserror::Error, Debug)]
#[error("Coingecko api error response: {0:?}")]
#[serde(transparent)]
pub struct CoingeckoErrResp(serde_json::Value);

pub mod payloads {
    use self::utils::Or;
    use super::utils::ser_joined_str;
    use super::*;

    #[derive(Debug, Serialize)]
    pub struct CurrentPriceReq {
        #[serde(serialize_with = "ser_joined_str")]
        ids: Vec<String>,
        #[serde(serialize_with = "ser_joined_str")]
        vs_assets: Vec<String>,
    }
    impl Default for CurrentPriceReq {
        fn default() -> Self {
            CurrentPriceReq {
                ids: vec!["ethereum".to_string()], // TODO all coingecko assets
                vs_assets: vec!["usd".to_string()],
            }
        }
    }
    impl CurrentPriceReq {
        pub fn or_default(self) -> Self {
            Self {
                ids: self.ids.or(Self::default().ids),
                vs_assets: self.vs_assets.or(Self::default().vs_assets),
            }
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct CurrentPriceResp(pub HashMap<String, HashMap<String, f64>>);
}

pub mod utils {
    use serde::Serializer;

    pub fn ser_joined_str<S>(v: &[String], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let joined_str = v.join(",");
        serializer.serialize_str(&joined_str)
    }

    pub trait Or {
        fn or(self, default: Self) -> Self;
    }
    impl<T> Or for Vec<T> {
        fn or(self, default: Self) -> Self {
            if self.is_empty() {
                return default;
            }
            self
        }
    }
}

pub mod old {
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

    pub trait RequestBuilderExt {
        async fn get_json<T: DeserializeOwned>(self) -> Result<T, anyhow::Error>;
    }
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
}
