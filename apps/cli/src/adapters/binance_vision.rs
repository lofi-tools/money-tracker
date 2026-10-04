use binance_client::archive::{ArchiveClient, Candle};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use lib_core::{AssetId, PriceRange, Store, history::AssetPricePoint};

use super::fallback_prices::FallbackPriceSvc;

/// Reads public Binance spot archives and persists each fetched pair/month.
pub struct BinanceVisionSvc {
    client: ArchiveClient,
    fallback: FallbackPriceSvc,
}

pub struct MonthFetch {
    pub count: usize,
    pub source: &'static str,
}

impl BinanceVisionSvc {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: ArchiveClient::new()?,
            fallback: FallbackPriceSvc::new()?,
        })
    }

    pub async fn fill_usd_month(
        &self,
        store: &Store,
        asset: &AssetId,
        month: NaiveDate,
    ) -> anyhow::Result<Option<MonthFetch>> {
        let range = month_range(month)?;
        let ticker = underlying(&asset.0);
        let (prices, source) = if ticker == "USD" || ticker == "USDT" {
            (
                vec![AssetPricePoint {
                    datetime: range.from,
                    asset_id: asset.clone(),
                    vs_asset_id: AssetId::str("USD"),
                    price: 1.0,
                }],
                "fixed USD proxy",
            )
        } else {
            let mut direct = None;
            for source in source_tickers(ticker) {
                direct = self.pair(store, source, "USD", month).await?;
                if direct.is_none() {
                    direct = self.pair(store, source, "USDT", month).await?;
                }
                if direct.is_some() {
                    break;
                }
            }
            if let Some(points) = direct {
                (
                    points
                        .into_iter()
                        .map(|c| AssetPricePoint {
                            datetime: c.time,
                            asset_id: asset.clone(),
                            vs_asset_id: AssetId::str("USD"),
                            price: c.close,
                        })
                        .collect(),
                    "Binance Vision",
                )
            } else if ticker == "GBP" || ticker == "EUR" {
                let btc_fiat = self.pair(store, "BTC", ticker, month).await?;
                if let Some(btc_fiat) = btc_fiat {
                    if let Some(btc_usdt) = self.pair(store, "BTC", "USDT", month).await? {
                        (
                            combine(&btc_fiat, &btc_usdt, |fiat_per_btc, usd_per_btc| {
                                usd_per_btc / fiat_per_btc
                            })
                            .into_iter()
                            .map(|(datetime, price)| AssetPricePoint {
                                datetime,
                                asset_id: asset.clone(),
                                vs_asset_id: AssetId::str("USD"),
                                price,
                            })
                            .collect(),
                            "Binance Vision BTC cross",
                        )
                    } else {
                        (self.fallback.ecb_month(asset, month).await?, "ECB")
                    }
                } else {
                    (self.fallback.ecb_month(asset, month).await?, "ECB")
                }
            } else {
                let mut asset_btc = None;
                for source in source_tickers(ticker) {
                    asset_btc = self.pair(store, source, "BTC", month).await?;
                    if asset_btc.is_some() {
                        break;
                    }
                }
                if let Some(asset_btc) = asset_btc {
                    if let Some(btc_usdt) = self.pair(store, "BTC", "USDT", month).await? {
                        (
                            combine(&asset_btc, &btc_usdt, |btc_per_asset, usd_per_btc| {
                                btc_per_asset * usd_per_btc
                            })
                            .into_iter()
                            .map(|(datetime, price)| AssetPricePoint {
                                datetime,
                                asset_id: asset.clone(),
                                vs_asset_id: AssetId::str("USD"),
                                price,
                            })
                            .collect(),
                            "Binance Vision BTC cross",
                        )
                    } else {
                        let Some(points) = self.kucoin(store, asset, month).await? else {
                            return Ok(None);
                        };
                        (points, "KuCoin")
                    }
                } else {
                    let Some(points) = self.kucoin(store, asset, month).await? else {
                        return Ok(None);
                    };
                    (points, "KuCoin")
                }
            }
        };
        anyhow::ensure!(
            !prices.is_empty(),
            "no usable historical USD prices for {} in {}",
            asset.0,
            month.format("%Y-%m")
        );
        store.save_historical_prices(asset, &AssetId::str("USD"), range, &prices)?;
        Ok(Some(MonthFetch {
            count: prices.len(),
            source,
        }))
    }

    async fn pair(
        &self,
        store: &Store,
        base: &str,
        quote: &str,
        month: NaiveDate,
    ) -> anyhow::Result<Option<Vec<Candle>>> {
        let range = month_range(month)?;
        let base_id = AssetId::str(base);
        let quote_id = AssetId::str(quote);
        if store
            .missing_price_ranges(&base_id, &quote_id, range)?
            .is_empty()
        {
            return Ok(Some(
                store
                    .prices_in_range(&base_id, &quote_id, range)?
                    .into_iter()
                    .map(|p| Candle {
                        time: p.datetime,
                        close: p.price,
                    })
                    .collect(),
            ));
        }
        let symbol = format!("{base}{quote}");
        if store.has_unavailable_price_period("binance-vision", &symbol, range)? {
            return Ok(None);
        }
        let Some(candles) = self.client.fetch_month(&symbol, month).await? else {
            store.mark_unavailable_price_period("binance-vision", &symbol, range)?;
            return Ok(None);
        };
        anyhow::ensure!(
            !candles.is_empty(),
            "Binance archive {symbol} is empty for {}",
            month.format("%Y-%m")
        );
        let points: Vec<_> = candles
            .iter()
            .map(|c| AssetPricePoint {
                datetime: c.time,
                asset_id: base_id.clone(),
                vs_asset_id: quote_id.clone(),
                price: c.close,
            })
            .collect();
        store.save_historical_prices(&base_id, &quote_id, range, &points)?;
        Ok(Some(candles))
    }

    async fn kucoin(
        &self,
        store: &Store,
        asset: &AssetId,
        month: NaiveDate,
    ) -> anyhow::Result<Option<Vec<AssetPricePoint>>> {
        let range = month_range(month)?;
        let symbol = format!("{}USDT", asset.0);
        if store.has_unavailable_price_period("kucoin", &symbol, range)? {
            return Ok(None);
        }
        let points = self.fallback.kucoin_month(asset, month).await?;
        if points.is_none() {
            store.mark_unavailable_price_period("kucoin", &symbol, range)?;
        }
        Ok(points)
    }
}

fn underlying(ticker: &str) -> &str {
    match ticker {
        "GBPX" => "GBP",
        "EURX" => "EUR",
        "USDX" | "xUSD" => "USD",
        other => other,
    }
}

fn source_tickers(ticker: &str) -> &'static [&'static str] {
    match ticker {
        "POL" => &["POL", "MATIC"],
        "BTC" => &["BTC"],
        "ETH" => &["ETH"],
        "BNB" => &["BNB"],
        "NEXO" => &["NEXO"],
        "NEAR" => &["NEAR"],
        "DOT" => &["DOT"],
        "USDC" => &["USDC"],
        "ETHW" => &["ETHW"],
        "GBP" => &["GBP"],
        "EUR" => &["EUR"],
        _ => &[],
    }
}

pub fn month_range(month: NaiveDate) -> anyhow::Result<PriceRange> {
    let first = month.with_day(1).unwrap();
    let next = if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1).unwrap()
    };
    PriceRange::new(day_start(first), day_start(next))
}

fn day_start(day: NaiveDate) -> DateTime<Utc> {
    DateTime::from_naive_utc_and_offset(day.and_hms_opt(0, 0, 0).unwrap(), Utc)
}

fn combine(a: &[Candle], b: &[Candle], f: impl Fn(f64, f64) -> f64) -> Vec<(DateTime<Utc>, f64)> {
    let mut j = 0;
    let mut out = Vec::new();
    for first in a {
        while j + 1 < b.len() && b[j + 1].time <= first.time {
            j += 1;
        }
        if b.is_empty() || b[j].time > first.time {
            continue;
        }
        let value = f(first.close, b[j].close);
        if value.is_finite() && value > 0.0 {
            out.push((first.time, value));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_rates_use_only_candles_known_at_the_time() -> anyhow::Result<()> {
        let t = day_start(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        let later = day_start(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        let a = vec![
            Candle {
                time: t,
                close: 0.01,
            },
            Candle {
                time: later,
                close: 0.02,
            },
        ];
        let b = vec![Candle {
            time: t,
            close: 50_000.0,
        }];
        assert_eq!(
            combine(&a, &b, |x, y| x * y),
            vec![(t, 500.0), (later, 1000.0)]
        );
        assert_eq!(
            combine(&a, &b, |x, y| y / x),
            vec![(t, 5_000_000.0), (later, 2_500_000.0)]
        );
        Ok(())
    }

    #[test]
    fn polygon_uses_matic_archive_when_pol_is_not_listed() {
        assert_eq!(source_tickers("POL"), &["POL", "MATIC"]);
    }

    #[tokio::test]
    async fn cached_pair_month_is_read_without_a_download() -> anyhow::Result<()> {
        let month = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let range = month_range(month)?;
        let store = Store::in_memory()?;
        let btc = AssetId::str("BTC");
        let usdt = AssetId::str("USDT");
        let point = AssetPricePoint {
            datetime: range.to - chrono::Duration::milliseconds(1),
            asset_id: btc.clone(),
            vs_asset_id: usdt.clone(),
            price: 42_000.0,
        };
        store.save_historical_prices(&btc, &usdt, range, &[point])?;
        let svc = BinanceVisionSvc::new()?;
        let cached = svc.pair(&store, "BTC", "USDT", month).await?.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].close, 42_000.0);
        Ok(())
    }

    #[tokio::test]
    async fn known_absent_pair_month_is_not_requested_again() -> anyhow::Result<()> {
        let month = NaiveDate::from_ymd_opt(2020, 8, 1).unwrap();
        let store = Store::in_memory()?;
        store.mark_unavailable_price_period("binance-vision", "NEXOUSD", month_range(month)?)?;
        let svc = BinanceVisionSvc::new()?;
        assert!(svc.pair(&store, "NEXO", "USD", month).await?.is_none());
        Ok(())
    }
}
