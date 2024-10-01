use api_client_utils::prelude::*;
use chrono::Utc;
use models::AssetPricePoint;
use payloads::{CurrentPriceReq, CurrentPriceResp};
use serde::Deserialize;

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

    pub async fn fetch_current_prices(
        &self,
        args: CurrentPriceReq,
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        let req = self.get("/simple/price").query(&args);
        let resp = req
            .recv_json::<CurrentPriceResp, CoingeckoErrResp>()
            .await?;

        let mut prices_out = Vec::new();
        for (asset_id, prices) in resp.0 {
            for (vs_asset_id, price) in prices {
                prices_out.push(AssetPricePoint {
                    asset_id: asset_id.clone(),
                    vs_asset_id,
                    price,
                    time: Utc::now(),
                })
            }
        }

        Ok(prices_out)
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
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize)]
    pub struct CurrentPriceReq {
        #[serde(serialize_with = "ser_joined_str")]
        ids: Vec<String>,
        #[serde(rename = "vs_currencies", serialize_with = "ser_joined_str")]
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

pub mod models {
    use chrono::{DateTime, Utc};

    pub struct AssetPricePoint {
        pub asset_id: String,
        pub vs_asset_id: String,
        pub price: f64,
        pub time: DateTime<Utc>,
    }
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
