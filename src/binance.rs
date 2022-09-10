use hmac_sha256::HMAC;
use reqwest::{Request, RequestBuilder, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::lazy::Lazy;
use std::time::SystemTime;

use crate::binance::staking::StakingPosition;
use crate::utils::hex;

const API_BASE: Lazy<Url> = Lazy::new(|| Url::parse("https://api.binance.com").unwrap());
const API_KEY: Lazy<String> = Lazy::new(|| std::env::var("BINANCE_API_KEY").unwrap());
const API_SECRET: Lazy<String> = Lazy::new(|| std::env::var("BINANCE_SECRET_KEY").unwrap());
// const API_BASE: &'static str = "https://api.binance.com";
// const API_KEY: &'static str = "BINANCE_API_KEY";
// const API_SECRET: &'static str = "BINANCE_SECRET_KEY";

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
    pub fn get(&self, path: &str) -> Result<RequestBuilder, anyhow::Error> {
        Ok(self.httpc.get(API_BASE.join(path)?))
    }
    #[deprecated]
    fn signed(req_bld: RequestBuilder) -> Result<Request, anyhow::Error> {
        let mut req = req_bld.header("X-MBX-APIKEY", &*API_KEY).build()?;
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

    // ROUTE METHODS
    pub async fn list_all_products(&self) -> Result<(), Box<dyn std::error::Error>> {
        let url = API_BASE.join("/sapi/v1/staking/productList")?;
        let req_bld = self.httpc.get(url).query(&[("product", "STAKING")]);
        let req = Self::signed(req_bld)?;

        let resp = self.httpc.execute(req).await?;
        let resp_deser: Vec<StakingProduct> = resp.json().await?;

        // match resp_deser {
        //     serde_json::Value::Array(arr) => {
        //         dbg!(arr.get(0));
        //     }
        //     _ => todo!(),
        // }

        dbg!(resp_deser.get(0));

        todo!()
    }

    pub async fn list_user_products(&self) -> Result<(), anyhow::Error> {
        let req = self.get("/sapi/v1/staking/position")?.sign()?;
        let resp: Vec<StakingPosition> = self.get_resp(req).await?;

        todo!()
    }
    pub async fn get_resp<D: DeserializeOwned>(&self, req: Request) -> Result<D, anyhow::Error> {
        Ok(self.httpc.execute(req).await?.json().await?)
        // TODO deserialize into BinanceErr if err status, return typed err if fails
    }
}

struct BinanceApiErr {
    code: i32,
    msg: String,
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

mod staking {
    use serde::Deserialize;

    #[allow(non_snake_case)]
    #[derive(Deserialize)]
    pub struct StakingPosition {
        pub positionId: String,
        pub projectId: String,
        pub asset: String,
        pub amount: String,
        pub purchaseTime: String,
        pub duration: String,
        pub accrualDays: String,
        pub rewardAsset: String,
        pub APY: String,
        pub rewardAmt: String,
        pub extraRewardAsset: String,
        pub extraRewardAPY: String,
        pub estExtraRewardAmt: String,
        pub nextInterestPay: String,
        pub nextInterestPayDate: String,
        pub payInterestPeriod: String,
        pub redeemAmountEarly: String,
        pub interestEndDate: String,
        pub deliverDate: String,
        pub redeemPeriod: String,
        pub redeemingAmt: String,
        pub canRedeemEarly: bool,
        pub renewable: bool,
        pub partialAmtDeliverDate: String,
        pub status: String,
    }
}

mod deprecated {
    #[deprecated]
    pub async fn list_products() -> Result<(), Box<dyn std::error::Error>> {
        // // let client = reqwest::Client::new();
        // // let url = Url::parse(&format!("{API_BASE}/sapi/v1/staking/productList"))?;
        // let url = API_BASE.join("/sapi/v1/staking/productList")?;

        // req_signed(url).await?;
        // // let resp = client
        // //     .get(url)
        // //     .header("X-MBX-APIKEY", &*API_KEY)
        // //     .send()
        // //     .await?;
        // // let resp_text = resp.text().await?;

        // // println!("{:#?}", resp_text);
        todo!()
    }
    // #[deprecated]
    // async fn req_signed(url: Url) -> Result<(), Box<dyn std::error::Error>> {
    //     // let client = reqwest::Client::new();
    //     // // TODO mv staking out
    //     // let mut req = client
    //     //     .get(url)
    //     //     .header("X-MBX-APIKEY", &*API_KEY)
    //     //     .query(&[("product", "STAKING")])
    //     //     .build()?;

    //     // let mut url = req.url_mut();

    //     // // append timestamp
    //     // {
    //     //     let timestamp = SystemTime::now()
    //     //         .duration_since(SystemTime::UNIX_EPOCH)?
    //     //         .as_millis();
    //     //     dbg!(timestamp);
    //     //     let mut query_pairs = url.query_pairs_mut();
    //     //     query_pairs.append_pair("timestamp", &timestamp.to_string());
    //     // }

    //     // // then hmac query
    //     // let query_str = url.query().unwrap(); // TODO err
    //     // let signature = HMAC::mac(query_str, &*API_SECRET);
    //     // let sig_hex = hex(signature)?;

    //     // // then append hmac signature
    //     // {
    //     //     let mut query_pairs = url.query_pairs_mut();
    //     //     query_pairs.append_pair("signature", &sig_hex);
    //     // }

    //     // let resp = client.execute(req).await?;
    //     // let resp_text = resp.text().await?;
    //     // // TODO NEXT TIME deser into struct
    //     // // .json::<HashMap<String, String>>()

    //     // println!("{:#?}", resp_text);

    //     todo!()
    // }
    #[deprecated]
    async fn list_swap_pools() {
        // let url = format!("{API_BASE}/sapi/v1/bswap/pools");
    }

    #[deprecated]
    fn hmac() -> Result<(), Box<dyn std::error::Error>> {
        // let mut url = Url::parse("https://example.net")?;
        // let mut query_pairs = url.query_pairs_mut();
        // // for pair in query.iter() {
        // //     dbg!(pair);
        // // }

        // let timestamp = SystemTime::now()
        //     .duration_since(SystemTime::UNIX_EPOCH)?
        //     .as_secs();
        // query_pairs.append_pair("timestamp", &timestamp.to_string());
        // let query_str = query_pairs.finish().query().unwrap();
        // dbg!(query_str);

        // // Calculate the signature from the data and key
        // let signature = HMAC::mac(query_str, &*API_SECRET);
        // dbg!(hex(signature)?);
        todo!()
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
