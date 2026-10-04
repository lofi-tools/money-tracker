use chrono::{DateTime, Utc};
use models::AssetPricePoint;
use payloads::{CurrentPriceReq, CurrentPriceResp};
use serde::Deserialize;
use utils::prelude::*;

const USER_AGENT: &str = concat!(
    "money-tracker/",
    env!("CARGO_PKG_VERSION"),
    " (local portfolio tracker; https://github.com/lofi-tools/money-tracker)"
);

pub struct CoingeckoClient {
    pub base_url: String,
    pub http_client: reqwest::Client,
}
impl IsApiClient for CoingeckoClient {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}
impl CoingeckoClient {
    pub fn new() -> anyhow::Result<Self> {
        let (base_url, api_key) = if let Ok(key) = std::env::var("COINGECKO_PRO_API_KEY") {
            (
                "https://pro-api.coingecko.com/api/v3/",
                Some(("x-cg-pro-api-key", key)),
            )
        } else if let Ok(key) = std::env::var("COINGECKO_DEMO_API_KEY") {
            (
                "https://api.coingecko.com/api/v3/",
                Some(("x-cg-demo-api-key", key)),
            )
        } else {
            ("https://api.coingecko.com/api/v3/", None)
        };
        Ok(CoingeckoClient {
            base_url: base_url.to_string(),
            http_client: Self::build_http_client(api_key)?,
        })
    }

    fn build_http_client(
        api_key: Option<(&'static str, String)>,
    ) -> anyhow::Result<reqwest::Client> {
        Ok(reqwest::Client::builder()
            .default_headers(Self::default_headers(api_key)?)
            .build()?)
    }

    fn default_headers(
        api_key: Option<(&'static str, String)>,
    ) -> anyhow::Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(USER_AGENT),
        );
        if let Some((name, key)) = api_key {
            headers.insert(name, reqwest::header::HeaderValue::from_str(&key)?);
        }
        Ok(headers)
    }

    pub async fn fetch_current_prices(
        &self,
        args: CurrentPriceReq,
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        let req = self.get("/simple/price").query(&args);
        let resp = req.fetch_json::<CurrentPriceResp>().await?;

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

    /// CoinGecko's market chart range endpoint returns [Unix milliseconds, price].
    pub async fn fetch_historical_prices(
        &self,
        coin_id: &str,
        vs_currency: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        anyhow::ensure!(from < to, "price range must have from < to");
        let path = format!("/coins/{coin_id}/market_chart/range");
        let request = self
            .get(&path)
            .query(&[
                ("vs_currency", vs_currency.to_string()),
                ("from", from.timestamp().to_string()),
                ("to", to.timestamp().to_string()),
            ])
            .timeout(std::time::Duration::from_secs(30));
        let response = request
            .fetch_json::<payloads::HistoricalPriceResp>()
            .await?;
        response
            .prices
            .into_iter()
            .map(|(millis, price)| {
                anyhow::ensure!(price.is_finite() && price >= 0.0, "invalid CoinGecko price");
                let time = DateTime::from_timestamp_millis(millis)
                    .ok_or_else(|| anyhow::anyhow!("invalid CoinGecko timestamp {millis}"))?;
                Ok(AssetPricePoint {
                    asset_id: coin_id.to_string(),
                    vs_asset_id: vs_currency.to_string(),
                    price,
                    time,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_is_configured_for_anonymous_demo_and_pro_clients() -> anyhow::Result<()> {
        for api_key in [
            None,
            Some(("x-cg-demo-api-key", "demo-key".to_string())),
            Some(("x-cg-pro-api-key", "pro-key".to_string())),
        ] {
            let headers = CoingeckoClient::default_headers(api_key.clone())?;
            assert_eq!(
                headers.get(reqwest::header::USER_AGENT),
                Some(&reqwest::header::HeaderValue::from_static(USER_AGENT))
            );
            if let Some((name, key)) = api_key {
                assert_eq!(headers.get(name).unwrap().to_str()?, key);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, thiserror::Error, Debug)]
#[error("Coingecko api error response: {0:?}")]
#[serde(transparent)]
pub struct CoingeckoErrResp(serde_json::Value);

pub mod payloads {
    use self::local_utils::Or;
    use super::local_utils::ser_joined_str;
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
        pub fn new(ids: Vec<String>, vs_assets: Vec<String>) -> Self {
            Self { ids, vs_assets }
        }

        pub fn or_default(self) -> Self {
            Self {
                ids: self.ids.or(Self::default().ids),
                vs_assets: self.vs_assets.or(Self::default().vs_assets),
            }
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct CurrentPriceResp(pub HashMap<String, HashMap<String, f64>>);

    #[derive(Deserialize, Debug)]
    pub struct HistoricalPriceResp {
        pub prices: Vec<(i64, f64)>,
    }
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

pub mod local_utils {
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
