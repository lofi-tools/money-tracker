use hmac_sha256::HMAC;
use reqwest::Url;
use std::{lazy::Lazy, time::SystemTime};

use crate::utils::hex;

const API_BASE: &'static str = "https://api.binance.com";
const API_KEY: Lazy<String> = Lazy::new(|| std::env::var("BINANCE_API_KEY").unwrap());
const API_SECRET: Lazy<String> = Lazy::new(|| std::env::var("BINANCE_SECRET_KEY").unwrap());

pub async fn list_products() -> Result<(), Box<dyn std::error::Error>> {
    // let client = reqwest::Client::new();
    let url = Url::parse(&format!("{API_BASE}/sapi/v1/staking/productList"))?;
    req_signed(url).await?;
    // let resp = client
    //     .get(url)
    //     .header("X-MBX-APIKEY", &*API_KEY)
    //     .send()
    //     .await?;
    // let resp_text = resp.text().await?;

    // println!("{:#?}", resp_text);
    Ok(())
}

async fn req_signed(url: Url) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut req = client
        .get(url)
        .header("X-MBX-APIKEY", &*API_KEY)
        .query(&[("product", "STAKING")])
        .build()?;

    let mut url = req.url_mut();
    // append timestamp
    {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();
        dbg!(timestamp);
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

    // let client = reqwest::Client::new();
    // let req =
    let resp = client.execute(req).await?;
    let resp_text = resp.text().await?;

    println!("{:#?}", resp_text);

    todo!()
}

// TODO mv methods to this struct
pub struct BinanceClient {
    httpc: reqwest::Client,
}
impl BinanceClient {}

async fn list_swap_pools() {
    let url = format!("{API_BASE}/sapi/v1/bswap/pools");
}

// TODO not pub, rm
pub fn hmac() -> Result<(), Box<dyn std::error::Error>> {
    let mut url = Url::parse("https://example.net")?;
    let mut query_pairs = url.query_pairs_mut();
    // for pair in query.iter() {
    //     dbg!(pair);
    // }

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    query_pairs.append_pair("timestamp", &timestamp.to_string());
    let query_str = query_pairs.finish().query().unwrap();
    dbg!(query_str);

    // Calculate the signature from the data and key
    let signature = HMAC::mac(query_str, &*API_SECRET);
    dbg!(hex(signature)?);
    Ok(())
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
        // println!("signature={:02x?}", signature);

        let expected = "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71";
        assert_eq!(hex, expected);
        Ok(())
    }
}
