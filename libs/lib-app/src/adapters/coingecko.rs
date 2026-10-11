use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use coingecko_client::{CoingeckoClient, payloads::CurrentPriceReq};
use lib_core::{AssetId, history::AssetPricePoint};

pub struct CoinGeckoSvc {
    api_client: CoingeckoClient,
}

impl CoinGeckoSvc {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            api_client: CoingeckoClient::new()?,
        })
    }

    pub async fn fetch_current_prices(
        &self,
        assets: &[AssetId],
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        let mut ids = BTreeSet::new();
        let needs_gbp = assets
            .iter()
            .any(|a| matches!(a.0.as_str(), "GBP" | "GBPX"));
        let needs_eur = assets
            .iter()
            .any(|a| matches!(a.0.as_str(), "EUR" | "EURX"));
        let fiat = needs_gbp || needs_eur;
        if fiat {
            ids.insert("bitcoin".to_string()); // fiat exchange rates, quoted against BTC
        }
        for asset in assets {
            if let Some(id) = coin_id(&asset.0)? {
                ids.insert(id.to_string());
            }
        }
        let prices = if ids.is_empty() {
            Vec::new()
        } else {
            let mut quotes = vec!["usd".to_string()];
            if needs_gbp {
                quotes.push("gbp".to_string());
            }
            if needs_eur {
                quotes.push("eur".to_string());
            }
            self.api_client
                .fetch_current_prices(CurrentPriceReq::new(ids.into_iter().collect(), quotes))
                .await?
        };
        let mut by_id = BTreeMap::new();
        for price in prices {
            by_id.insert((price.asset_id, price.vs_asset_id), price.price);
        }
        let rate = |coin: &str, currency: &str| -> anyhow::Result<f64> {
            let value = *by_id
                .get(&(coin.to_string(), currency.to_string()))
                .with_context(|| format!("CoinGecko returned no {coin}/{currency} price"))?;
            anyhow::ensure!(
                value.is_finite() && value > 0.0,
                "invalid CoinGecko {coin}/{currency} price"
            );
            Ok(value)
        };
        let btc_usd = if fiat {
            Some(rate("bitcoin", "usd")?)
        } else {
            None
        };
        let now = Utc::now();
        assets
            .iter()
            .map(|asset| {
                let price = match asset.0.as_str() {
                    "USD" | "USDX" | "xUSD" => 1.0,
                    "GBP" | "GBPX" => {
                        usd_per_fiat_from_btc(btc_usd.unwrap(), rate("bitcoin", "gbp")?)?
                    }
                    "EUR" | "EURX" => {
                        usd_per_fiat_from_btc(btc_usd.unwrap(), rate("bitcoin", "eur")?)?
                    }
                    _ => {
                        let id = coin_id(&asset.0)?.context("missing CoinGecko coin ID")?;
                        rate(id, "usd")?
                    }
                };
                Ok(AssetPricePoint {
                    datetime: now,
                    asset_id: asset.clone(),
                    vs_asset_id: AssetId::str("USD"),
                    price,
                })
            })
            .collect()
    }

    pub async fn fetch_historical_prices(
        &self,
        assets: &[AssetId],
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        let quote = AssetId::str("USD");
        let mut out = Vec::new();
        let needs_gbp = assets
            .iter()
            .any(|a| matches!(a.0.as_str(), "GBP" | "GBPX"));
        let needs_eur = assets
            .iter()
            .any(|a| matches!(a.0.as_str(), "EUR" | "EURX"));
        let fx = if needs_gbp || needs_eur {
            let usd = self
                .api_client
                .fetch_historical_prices("bitcoin", "usd", from, to)
                .await?;
            let gbp = if needs_gbp {
                Some(
                    self.api_client
                        .fetch_historical_prices("bitcoin", "gbp", from, to)
                        .await?,
                )
            } else {
                None
            };
            let eur = if needs_eur {
                Some(
                    self.api_client
                        .fetch_historical_prices("bitcoin", "eur", from, to)
                        .await?,
                )
            } else {
                None
            };
            Some((usd, gbp, eur))
        } else {
            None
        };
        for asset in assets {
            match asset.0.as_str() {
                "USD" | "USDX" | "xUSD" => {
                    out.push(AssetPricePoint {
                        datetime: from,
                        asset_id: asset.clone(),
                        vs_asset_id: quote.clone(),
                        price: 1.0,
                    });
                }
                "GBP" | "GBPX" | "EUR" | "EURX" => {
                    let (usd, gbp, eur) = fx.as_ref().unwrap();
                    let other = if matches!(asset.0.as_str(), "GBP" | "GBPX") {
                        gbp.as_ref().unwrap()
                    } else {
                        eur.as_ref().unwrap()
                    };
                    let mut j = 0;
                    for u in usd {
                        while j + 1 < other.len() && other[j + 1].time <= u.time {
                            j += 1;
                        }
                        if other.is_empty() || other[j].time > u.time {
                            continue;
                        }
                        let price = usd_per_fiat_from_btc(u.price, other[j].price)?;
                        out.push(AssetPricePoint {
                            datetime: u.time,
                            asset_id: asset.clone(),
                            vs_asset_id: quote.clone(),
                            price,
                        });
                    }
                }
                _ => {
                    let id = coin_id(&asset.0)?.context("missing CoinGecko coin ID")?;
                    for point in self
                        .api_client
                        .fetch_historical_prices(id, "usd", from, to)
                        .await?
                    {
                        out.push(AssetPricePoint {
                            datetime: point.time,
                            asset_id: asset.clone(),
                            vs_asset_id: quote.clone(),
                            price: point.price,
                        });
                    }
                }
            }
        }
        Ok(out)
    }
}

fn usd_per_fiat_from_btc(btc_usd: f64, btc_fiat: f64) -> anyhow::Result<f64> {
    anyhow::ensure!(
        btc_usd.is_finite() && btc_usd > 0.0,
        "invalid BTC/USD price"
    );
    anyhow::ensure!(
        btc_fiat.is_finite() && btc_fiat > 0.0,
        "invalid BTC/fiat price"
    );
    Ok(btc_usd / btc_fiat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiat_price_routes_through_btc_when_direct_fiat_usd_is_unavailable() -> anyhow::Result<()> {
        assert_eq!(usd_per_fiat_from_btc(60_000.0, 50_000.0)?, 1.2);
        assert!(usd_per_fiat_from_btc(60_000.0, 0.0).is_err());
        Ok(())
    }
}

fn coin_id(ticker: &str) -> anyhow::Result<Option<&'static str>> {
    Ok(match ticker {
        "USD" | "USDX" | "xUSD" | "GBP" | "GBPX" | "EUR" | "EURX" => None,
        "BTC" => Some("bitcoin"),
        "ETH" => Some("ethereum"),
        "BNB" => Some("binancecoin"),
        "NEXO" => Some("nexo"),
        "NEAR" => Some("near"),
        "DOT" => Some("polkadot"),
        "POL" => Some("polygon-ecosystem-token"),
        "USDT" => Some("tether"),
        "USDC" => Some("usd-coin"),
        "ETHW" => Some("ethereum-pow-iou"),
        _ => bail!("no CoinGecko mapping for Nexo asset {ticker}"),
    })
}
