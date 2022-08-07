#![feature(once_cell)]

mod binance;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;

    // TODO load binance API key, query products/subs

    // let client = reqwest::Client::new();
    // let resp = client
    //     .get("https://api.binance.com/sapi/v1/staking/productList")
    //     .header(
    //         "X-MBX-APIKEY",
    //         "vmPUZE6mv9SD5VNHk4HlWFsOr6aKE2zvsw0MuIgwCIPy6utIco14y7Ju91duEh8A",
    //     )
    //     .send()
    //     .await?;
    // let resp_text = resp.text().await?;

    // let resp = reqwest::get("https://api.binance.com/sapi/v1/staking/productList")
    //     .await?
    //     // .json::<HashMap<String, String>>()
    //     .text()
    //     .await?;

    binance::list_products().await?;
    // binance::hmac()?;
    Ok(())
}
