use crate::payloads::{ListResp, StakingPositionResp};
use api_client_utils::prelude::*;
use payloads::{LockedEarnPos, StakingProduct};
use serde::Deserialize;
use signing::RequestBuilderExt;

// const API_BASE: OnceCell<Url> = OnceCell::new(|| Url::parse("https://api.binance.com").unwrap());
// const API_KEY: OnceCell<String> = OnceCell::new(|| std::env::var("BINANCE_API_KEY").unwrap());
// const API_SECRET: OnceCell<String> = OnceCell::new(|| std::env::var("BINANCE_SECRET_KEY").unwrap());

pub struct BinanceClient {
    http_client: reqwest::Client,
    base_url: String,
    api_key: String,
    api_secret: String,
}
impl JsonApiClient for BinanceClient {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}
impl BinanceClient {
    pub fn new() -> anyhow::Result<Self> {
        Ok(BinanceClient {
            http_client: reqwest::Client::new(),
            base_url: "https://api.binance.com/sapi/v1".to_string(),
            api_key: std::env::var("BINANCE_API_KEY")?,
            api_secret: std::env::var("BINANCE_SECRET_KEY")?,
        })
    }

    pub async fn list_staking_products(&self) -> anyhow::Result<Vec<StakingProduct>> {
        let req = self
            .get("/staking/productList")
            .query(&[("product", "STAKING")])
            .sign(self)?;
        let resp = req
            .recv_json::<Vec<StakingProduct>, BinanceApiErrResp>()
            .await?;
        Ok(resp)
    }
    pub async fn list_staking_positions(&self) -> anyhow::Result<Vec<StakingPositionResp>> {
        let req = self
            .get("/staking/position")
            .query(&[("product", "STAKING")])
            .sign(self)?;
        let resp: Vec<StakingPositionResp> = req
            .recv_json::<Vec<StakingPositionResp>, BinanceApiErrResp>()
            .await?;
        Ok(resp)
    }
    pub async fn list_locked_earn_positions(&self) -> anyhow::Result<Vec<LockedEarnPos>> {
        let req = self.get("/simple-earn/locked/position").sign(self)?;
        let resp = req
            .recv_json::<ListResp<LockedEarnPos>, BinanceApiErrResp>()
            .await?;

        // TODO parse timestamps into

        todo!()
    }
    pub async fn list_flexible_earn_pos(&self) -> anyhow::Result<Vec<()>> {
        todo!()
    }
}

#[derive(Debug, derive_more::Display, Deserialize)]
#[display(fmt = "msg: {msg}")]
pub struct BinanceApiErrResp {
    code: i32,
    msg: String,
}

pub mod payloads {
    use crate::utils::{de_from_str, de_str_to_datetime};
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::ops::Deref;

    #[derive(Deserialize, Debug)]
    pub struct ListResp<T> {
        pub rows: Vec<T>,
        pub total: u64,
    }

    #[derive(Deserialize, Debug)]
    pub struct StakingPositionResp {
        #[serde(rename = "positionId")]
        pub position_id: PositionId,
        #[serde(rename = "productId")]
        pub product_id: String,
        pub asset: String,
        pub amount: String,
        #[serde(rename = "purchaseTime")]
        pub purchase_time: u64,
        pub duration: u64,
        // pub accrualDays: String,
        #[serde(rename = "rewardAsset")]
        pub reward_asset: String,
        pub apy: String,
        // pub rewardAmt: String,
        // pub extraRewardAsset: String,
        // pub extraRewardAPY: String,
        // pub estExtraRewardAmt: String,
        // pub nextInterestPay: String,
        // pub nextInterestPayDate: String,
        // pub payInterestPeriod: String,
        // pub redeemAmountEarly: String,
        #[serde(rename = "interestEndDate")]
        pub interest_end_date: u64,
        // pub deliverDate: String,
        // pub redeemPeriod: String,
        // pub redeemingAmt: String,
        // pub canRedeemEarly: bool,
        // pub renewable: bool,
        // pub partialAmtDeliverDate: String,
        // pub status: String,
    }

    #[derive(Deserialize, Debug)]
    pub struct LockedEarnPos {
        #[serde(rename = "positionId")]
        pub position_id: String,
        #[serde(rename = "projectId")]
        pub project_id: String,
        #[serde(rename = "asset")]
        pub asset_id: String,
        #[serde(deserialize_with = "de_from_str")]
        pub amount: f64,
        #[serde(rename = "purchaseTime", deserialize_with = "de_str_to_datetime")]
        pub purchase_time: DateTime<Utc>,
        #[serde(rename = "duration", deserialize_with = "de_from_str")]
        pub duration_days: u64,
        #[serde(rename = "accrualDays", deserialize_with = "de_from_str")]
        pub accrual_days: u64,
        #[serde(rename = "rewardAsset")]
        pub reward_asset: String,
        #[serde(rename = "APY", deserialize_with = "de_from_str")]
        pub apy: f64,
        #[serde(rename = "isRenewable")]
        pub is_renewable: bool,
        #[serde(rename = "isAutoRenew")]
        pub is_auto_renew: bool,
        #[serde(rename = "redeemDate", deserialize_with = "de_str_to_datetime")]
        pub redeem_date: DateTime<Utc>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct PositionId {
        id: u64,
        _type: std::marker::PhantomData<StakingPositionResp>,
    }
    impl Deref for PositionId {
        type Target = u64;
        fn deref(&self) -> &Self::Target {
            &self.id
        }
    }

    #[derive(Deserialize, Debug)]
    pub struct StakingProduct {
        #[serde(rename = "projectId")]
        pub project_id: String,
        pub detail: StakingProductDetail,
        pub quota: serde_json::Value,
    }
    #[derive(Deserialize, Debug)]
    pub struct StakingProductDetail {
        pub asset: String, // Lock up asset
        #[serde(rename = "rewardAsset")]
        pub reward_asset: String, // Earn Asset
        pub duration: u32, // Lock period(days)
        pub renewable: bool, // Project supports renewal
        #[serde(deserialize_with = "crate::utils::de_from_str")]
        pub apy: f64, // APY in multiple_per_year,
    }
}

pub mod models {
    pub struct StakingPosition {
        // TODO
    }
}

pub mod signing {
    use crate::{utils::hex, BinanceClient};
    use api_client_utils::RequestClient;
    use hmac_sha256::HMAC;
    use reqwest::RequestBuilder;
    use std::time::SystemTime;

    pub trait RequestBuilderExt {
        fn sign(self, client: &BinanceClient) -> Result<RequestClient, anyhow::Error>;
    }
    impl RequestBuilderExt for RequestBuilder {
        fn sign(self, client: &BinanceClient) -> Result<RequestClient, anyhow::Error> {
            use api_client_utils::RequestBuilderExt;
            let mut req = self
                .header("X-MBX-APIKEY", &client.api_key)
                .try_build_split()?;
            let url = req.request.url_mut();

            // append timestamp
            {
                let timestamp = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_millis();
                let mut query_pairs = url.query_pairs_mut();
                query_pairs.append_pair("timestamp", &timestamp.to_string());
            }

            // then hmac query
            let query_str = url.query().unwrap(); // TODO err
            let signature = HMAC::mac(query_str, &client.api_secret);
            let sig_hex = hex(signature)?;

            // then append hmac signature
            {
                let mut query_pairs = url.query_pairs_mut();
                query_pairs.append_pair("signature", &sig_hex);
            }

            Ok(req)
        }
    }
}

pub mod utils {
    use chrono::{DateTime, Utc};
    use serde::{de, Deserialize, Deserializer};

    pub fn de_from_str<'de, D, Out>(deserializer: D) -> Result<Out, D::Error>
    where
        D: Deserializer<'de>,
        Out: std::str::FromStr,
        Out::Err: std::fmt::Display,
    {
        let s = String::deserialize(deserializer)?;
        Out::from_str(&s).map_err(de::Error::custom)
    }

    pub fn de_str_to_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp_str = String::deserialize(deserializer)?;
        let timestamp = timestamp_str.parse::<i64>().map_err(de::Error::custom)?;
        let datetime = DateTime::from_timestamp_millis(timestamp)
            .ok_or(de::Error::custom("out of range number of milliseconds"))?;
        Ok(datetime)
    }

    pub fn hex(_in: impl AsRef<[u8]>) -> Result<String, std::fmt::Error> {
        let mut s = String::new();
        for byte in _in.as_ref() {
            use std::fmt::Write;
            // println!("{:02x}", byte);
            write!(&mut s, "{:02x}", byte)?;
        }
        Ok(s)
    }
    #[cfg(test)]
    mod tests {
        use crate::utils::hex;
        use hmac_sha256::HMAC;

        #[test]
        fn external_test_hmac() -> Result<(), Box<dyn std::error::Error>> {
            let data = b"symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";
            let key = b"NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
            let signature = HMAC::mac(data, key);
            let hex = hex(signature)?;

            let expected = "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71";
            assert_eq!(hex, expected);
            Ok(())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::payloads::LockedEarnPos;

    #[tokio::test]
    async fn test_fetch_binance() -> anyhow::Result<()> {
        let client = BinanceClient::new()?;
        let products = client.list_staking_products().await?;
        dbg!(products);

        Ok(())
    }

    #[test]
    fn test_serde_responses() -> anyhow::Result<()> {
        let resp_list_earn_flexible = r#"{"positionId": "123123","projectId": "Axs*90","asset": "AXS","amount": "122.09202928","purchaseTime": "1646182276000","duration": "60","accrualDays": "4","rewardAsset": "AXS","APY": "0.23","isRenewable": true,"isAutoRenew": true,"redeemDate": "1732182276000"}"#;
        let deser = serde_json::from_str::<LockedEarnPos>(&resp_list_earn_flexible)?;
        dbg!(deser);
        Ok(())
    }
}
