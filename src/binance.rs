use crate::binance::staking::StakingPosition;
use crate::models::{ExternalId, ProductId};
use crate::utils::hex;
use derive_more::Display;
use hmac_sha256::HMAC;
use reqwest::{Request, RequestBuilder, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::lazy::Lazy;
use std::time::SystemTime;

const API_BASE: Lazy<Url> = Lazy::new(|| Url::parse("https://api.binance.com").unwrap());
const API_KEY: Lazy<String> = Lazy::new(|| std::env::var("BINANCE_API_KEY").unwrap());
const API_SECRET: Lazy<String> = Lazy::new(|| std::env::var("BINANCE_SECRET_KEY").unwrap());
const PROVIDER_ID_BINANCE: &str = "binance";

#[derive(Deserialize, Debug)]
pub struct StakingProduct {
    #[serde(rename = "projectId")]
    project_id: String,
    detail: StakingProductDetail,
    quota: serde_json::Value,
}
#[derive(Deserialize, Debug)]
pub struct StakingProductDetail {
    asset: String, // Lock up asset
    #[serde(rename = "rewardAsset")]
    reward_asset: String, // Earn Asset
    duration: u32, // Lock period(days)
    renewable: bool, // Project supports renewal
    #[serde(deserialize_with = "crate::utils::de_from_str")]
    apy: f64, // APY in multiple_per_year,
}

pub struct BinanceClient {
    httpc: reqwest::Client,
}
impl BinanceClient {
    pub fn new() -> Self {
        return BinanceClient {
            httpc: reqwest::Client::new(),
        };
    }

    // ROUTE METHODS
    pub async fn list_all_products(&self) -> Result<Vec<StakingProduct>, anyhow::Error> {
        let req = self
            .get("/sapi/v1/staking/productList")?
            .query(&[("product", "STAKING")])
            .sign()?;
        let resp: Vec<StakingProduct> = self.get_resp(req).await?;
        Ok(resp)
    }
    pub async fn list_staking_positions(&self) -> Result<Vec<StakingPosition>, anyhow::Error> {
        let req = self
            .get("/sapi/v1/staking/position")?
            .query(&[("product", "STAKING")])
            .sign()?;
        let resp: Vec<StakingPosition> = self.get_resp(req).await?;
        Ok(resp)
    }

    // UTILS
    pub fn get(&self, path: &str) -> Result<RequestBuilder, anyhow::Error> {
        Ok(self.httpc.get(API_BASE.join(path)?))
    }
    pub async fn get_resp<D: DeserializeOwned>(&self, req: Request) -> Result<D, BinanceErr> {
        let resp = self
            .httpc
            .execute(req)
            .await
            .map_err(BinanceErr::ReqwestErr)?;

        match resp.status() {
            s if s.is_success() => {
                // let txt = resp.text().await.unwrap();
                // dbg!(txt);
                // todo!()
                let resp_parsed: D = resp.json().await.map_err(BinanceErr::DeserResp)?;
                Ok(resp_parsed)
            }
            _status_err => {
                let err_parsed: BinanceApiErr = resp.json().await.map_err(BinanceErr::DeserResp)?;
                Err(BinanceErr::ApiErrResp(err_parsed))
            }
        }
    }
}

pub struct Binance;
impl Binance {
    fn product_id(id: &str) -> ProductId {
        ProductId(ExternalId::new(PROVIDER_ID_BINANCE, id))
    }
}

#[derive(Debug, Display, Deserialize)]
#[display(fmt = "msg: {msg}")]
pub struct BinanceApiErr {
    code: i32,
    msg: String,
}

mod staking {
    use super::Binance;
    use serde::Deserialize;

    #[allow(non_snake_case)]
    #[derive(Deserialize, Debug)]
    pub struct StakingPosition {
        #[serde(rename = "positionId")]
        pub positionId: u64,
        pub productId: String,
        pub asset: String,
        pub amount: String,
        pub purchaseTime: u64,
        pub duration: u64,
        // pub accrualDays: String,
        pub rewardAsset: String,
        pub apy: String,
        // pub rewardAmt: String,
        // pub extraRewardAsset: String,
        // pub extraRewardAPY: String,
        // pub estExtraRewardAmt: String,
        // pub nextInterestPay: String,
        // pub nextInterestPayDate: String,
        // pub payInterestPeriod: String,
        // pub redeemAmountEarly: String,
        pub interestEndDate: u64,
        // pub deliverDate: String,
        // pub redeemPeriod: String,
        // pub redeemingAmt: String,
        // pub canRedeemEarly: bool,
        // pub renewable: bool,
        // pub partialAmtDeliverDate: String,
        // pub status: String,
    }
    impl StakingPosition {
        // TODO fetch product if not exists in memDB
        pub async fn product(&self) {
            let product_id = Binance::product_id(&self.productId);
            // TODO use tokio message passing to get/update state
            // let found_product = PRODUCTS.
        }
    }
    // impl Into<models::Position> for StakingPosition {
    //     fn into(self) -> models::Position {
    //         todo!()
    //     }
    // }
    // impl Into<models::Product> for StakingPosition {

    // }
}

mod transform {
    use crate::binance::{self, PROVIDER_ID_BINANCE};
    use crate::data::ASSETS;
    use crate::models::{
        AssetId, ExternalId, ExternalIdVal, Position, PositionId, Product, ProductId,
    };

    fn product_id(id: impl Into<ExternalIdVal>) -> ProductId {
        ProductId(ExternalId::new(PROVIDER_ID_BINANCE, id)) // TODO parse resp to those types
    }
    fn position_id(id: impl Into<ExternalIdVal>) -> PositionId {
        PositionId(ExternalId::new(PROVIDER_ID_BINANCE, id)) // TODO parse biannce resp to this type
    }

    impl From<&binance::StakingPosition> for Position {
        fn from(pos: &binance::StakingPosition) -> Self {
            // TODO match product
            // let matched_product = MUT_PRODUCTS.read().
            // let product = Product::from(pos);

            Position {
                id: position_id(pos.positionId),
                product_id: Product::from(pos).id,
                amount: pos.amount.parse::<f64>().unwrap(),
                start_date: pos.purchaseTime,
                end_date: pos.interestEndDate,
            }
        }
    }
    impl From<&binance::StakingPosition> for Product {
        fn from(pos: &binance::StakingPosition) -> Self {
            let matched_asset = ASSETS.get(&AssetId(pos.asset.clone())).unwrap(); // TODO report errors somewhere

            Product {
                id: product_id(&pos.productId),
                asset_id: matched_asset.id.clone(),
                apy: (pos.apy.parse()).unwrap(),
            }
        }
    }
}

pub trait RequestBuilderExt {
    fn sign(self) -> Result<Request, anyhow::Error>;
}
impl RequestBuilderExt for RequestBuilder {
    fn sign(self) -> Result<Request, anyhow::Error> {
        let mut req = self.header("X-MBX-APIKEY", &*API_KEY).build()?;
        let url = req.url_mut();

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
        let signature = HMAC::mac(query_str, &*API_SECRET);
        let sig_hex = hex(signature)?;

        // then append hmac signature
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("signature", &sig_hex);
        }

        Ok(req)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BinanceErr {
    #[error("failed deserializing resp body: {0}")]
    DeserResp(reqwest::Error),
    #[error("API error response: {0}")]
    ApiErrResp(BinanceApiErr),
    #[error("reqwest err: {0}")]
    ReqwestErr(reqwest::Error),
}

#[cfg(test)]
mod tests {
    use hmac_sha256::HMAC;

    use crate::utils::hex;

    #[test]
    fn test_hmac() -> Result<(), Box<dyn std::error::Error>> {
        let data = b"symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";
        let key = b"NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
        let signature = HMAC::mac(data, key);
        let hex = hex(signature)?;

        let expected = "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71";
        assert_eq!(hex, expected);
        Ok(())
    }
}
