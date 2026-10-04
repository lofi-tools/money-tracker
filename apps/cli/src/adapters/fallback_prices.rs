//! Public daily-price sources for symbols or periods absent from Binance Vision.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::{AssetId, history::AssetPricePoint};
use serde::Deserialize;
use snafu::{ResultExt, Snafu};

use super::binance_vision::month_range;

#[derive(Debug, Snafu)]
pub enum FallbackError {
    #[snafu(display("cannot initialize public price client"))]
    Client { source: reqwest::Error },
    #[snafu(display("requesting {provider} historical prices"))]
    Request {
        provider: &'static str,
        source: reqwest::Error,
    },
    #[snafu(display("{provider} returned HTTP {status}: {body}"))]
    Http {
        provider: &'static str,
        status: u16,
        body: String,
    },
    #[snafu(display("invalid KuCoin candle response"))]
    KucoinJson { source: reqwest::Error },
    #[snafu(display("KuCoin rejected historical price request: {message}"))]
    Kucoin { message: String },
    #[snafu(display("invalid KuCoin candle: {message}"))]
    Candle { message: String },
    #[snafu(display("invalid ECB historical FX CSV"))]
    EcbCsv { source: csv::Error },
    #[snafu(display("invalid ECB historical FX value: {message}"))]
    EcbValue { message: String },
}

pub struct FallbackPriceSvc {
    http: reqwest::Client,
}

impl FallbackPriceSvc {
    pub fn new() -> Result<Self, FallbackError> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .user_agent("crypto-tracker/0.1 (+https://github.com/nmrshll/crypto-tracker)")
            .build()
            .context(ClientSnafu)?;
        Ok(Self { http })
    }

    /// KuCoin is used only when Binance has no spot pair for the month.
    pub async fn kucoin_month(
        &self,
        asset: &AssetId,
        month: NaiveDate,
    ) -> Result<Option<Vec<AssetPricePoint>>, FallbackError> {
        let range = month_range(month).map_err(|e| FallbackError::Candle {
            message: e.to_string(),
        })?;
        let symbol = format!("{}-USDT", asset.0);
        let response = self
            .http
            .get("https://api.kucoin.com/api/v1/market/candles")
            .query(&[
                ("symbol", symbol.as_str()),
                ("type", "1day"),
                ("startAt", &range.from.timestamp().to_string()),
                ("endAt", &range.to.timestamp().to_string()),
            ])
            .send()
            .await
            .context(RequestSnafu { provider: "KuCoin" })?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .context(RequestSnafu { provider: "KuCoin" })?;
            return Err(FallbackError::Http {
                provider: "KuCoin",
                status: status.as_u16(),
                body,
            });
        }
        let payload: KucoinResponse = response.json().await.context(KucoinJsonSnafu)?;
        if payload.code == "400100" {
            return Ok(None);
        }
        if payload.code != "200000" {
            return Err(FallbackError::Kucoin {
                message: payload.msg.unwrap_or(payload.code),
            });
        }
        let rows = payload.data.unwrap_or_default();
        let points = parse_kucoin_rows(asset, &rows, range.from, range.to)?;
        Ok(if points.is_empty() {
            None
        } else {
            Some(points)
        })
    }

    /// ECB publishes EUR base reference rates. Derive GBP/USD as USD/EUR
    /// divided by GBP/EUR; EUR/USD is directly available.
    pub async fn ecb_month(
        &self,
        asset: &AssetId,
        month: NaiveDate,
    ) -> Result<Vec<AssetPricePoint>, FallbackError> {
        let range = month_range(month).map_err(|e| FallbackError::EcbValue {
            message: e.to_string(),
        })?;
        let start = (range.from - Duration::days(7)).date_naive().to_string();
        let end = (range.to - Duration::days(1)).date_naive().to_string();
        let response = self
            .http
            .get("https://data-api.ecb.europa.eu/service/data/EXR/D.USD+GBP.EUR.SP00.A")
            .query(&[
                ("startPeriod", start.as_str()),
                ("endPeriod", end.as_str()),
                ("format", "csvdata"),
            ])
            .send()
            .await
            .context(RequestSnafu { provider: "ECB" })?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .context(RequestSnafu { provider: "ECB" })?;
            return Err(FallbackError::Http {
                provider: "ECB",
                status: status.as_u16(),
                body,
            });
        }
        let bytes = response
            .bytes()
            .await
            .context(RequestSnafu { provider: "ECB" })?;
        parse_ecb_csv(asset, &bytes, range.from, range.to)
    }
}

#[derive(Deserialize)]
struct KucoinResponse {
    code: String,
    msg: Option<String>,
    data: Option<Vec<Vec<String>>>,
}

fn parse_kucoin_rows(
    asset: &AssetId,
    rows: &[Vec<String>],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<AssetPricePoint>, FallbackError> {
    let mut prices = Vec::new();
    for row in rows {
        let open = row
            .first()
            .ok_or_else(|| FallbackError::Candle {
                message: "missing open time".into(),
            })?
            .parse::<i64>()
            .map_err(|e| FallbackError::Candle {
                message: e.to_string(),
            })?;
        let close = row
            .get(2)
            .ok_or_else(|| FallbackError::Candle {
                message: "missing close price".into(),
            })?
            .parse::<f64>()
            .map_err(|e| FallbackError::Candle {
                message: e.to_string(),
            })?;
        let datetime =
            DateTime::from_timestamp(open + 86_400, 0).ok_or_else(|| FallbackError::Candle {
                message: "open time out of range".into(),
            })? - Duration::milliseconds(1);
        if !close.is_finite() || close <= 0.0 {
            return Err(FallbackError::Candle {
                message: format!("invalid close {close}"),
            });
        }
        if datetime >= from && datetime < to {
            prices.push(AssetPricePoint {
                datetime,
                asset_id: asset.clone(),
                vs_asset_id: AssetId::str("USD"),
                price: close,
            });
        }
    }
    prices.sort_by_key(|p| p.datetime);
    Ok(prices)
}

fn parse_ecb_csv(
    asset: &AssetId,
    bytes: &[u8],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<AssetPricePoint>, FallbackError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader.headers().context(EcbCsvSnafu)?.clone();
    let index = |name: &str| {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| FallbackError::EcbValue {
                message: format!("missing {name} column"),
            })
    };
    let currency_col = index("CURRENCY")?;
    let date_col = index("TIME_PERIOD")?;
    let value_col = index("OBS_VALUE")?;
    let mut rates: BTreeMap<NaiveDate, (Option<f64>, Option<f64>)> = BTreeMap::new();
    for row in reader.records() {
        let row = row.context(EcbCsvSnafu)?;
        let currency = row.get(currency_col).unwrap_or("");
        let day = NaiveDate::parse_from_str(row.get(date_col).unwrap_or(""), "%Y-%m-%d").map_err(
            |e| FallbackError::EcbValue {
                message: e.to_string(),
            },
        )?;
        let rate = row
            .get(value_col)
            .unwrap_or("")
            .parse::<f64>()
            .map_err(|e| FallbackError::EcbValue {
                message: e.to_string(),
            })?;
        if !rate.is_finite() || rate <= 0.0 {
            return Err(FallbackError::EcbValue {
                message: format!("invalid {currency} rate {rate}"),
            });
        }
        let entry = rates.entry(day).or_default();
        match currency {
            "USD" => entry.0 = Some(rate),
            "GBP" => entry.1 = Some(rate),
            _ => {}
        }
    }
    let rate_for = |usd: Option<f64>, gbp: Option<f64>| -> Option<f64> {
        match asset.0.as_str() {
            "EUR" | "EURX" => usd,
            "GBP" | "GBPX" => Some(usd? / gbp?),
            _ => None,
        }
    };
    let mut prices = BTreeMap::new();
    for (day, (usd, gbp)) in rates {
        let Some(price) = rate_for(usd, gbp) else {
            continue;
        };
        let day_start = DateTime::from_naive_utc_and_offset(day.and_hms_opt(0, 0, 0).unwrap(), Utc);
        let datetime = if day_start < from {
            from
        } else {
            day_start + Duration::hours(16)
        };
        if datetime >= from && datetime < to {
            prices.insert(
                datetime,
                AssetPricePoint {
                    datetime,
                    asset_id: asset.clone(),
                    vs_asset_id: AssetId::str("USD"),
                    price,
                },
            );
        }
    }
    // Multiple pre-month fixing days all map to `from`; keep only the latest.
    Ok(prices.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kucoin_close_is_known_only_at_day_end() -> Result<(), FallbackError> {
        let asset = AssetId::str("ETHW");
        let from = DateTime::from_timestamp(1663632000, 0).unwrap();
        let to = from + Duration::days(2);
        let rows = vec![vec!["1663632000".into(), "7".into(), "8".into()]];
        let prices = parse_kucoin_rows(&asset, &rows, from, to)?;
        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].price, 8.0);
        assert_eq!(
            prices[0].datetime,
            from + Duration::days(1) - Duration::milliseconds(1)
        );
        Ok(())
    }

    #[test]
    fn ecb_cross_rate_includes_prior_business_day_at_month_start() -> Result<(), FallbackError> {
        let csv = b"CURRENCY,TIME_PERIOD,OBS_VALUE\nUSD,2024-03-29,1.08\nGBP,2024-03-29,0.86\nUSD,2024-04-02,1.10\nGBP,2024-04-02,0.88\n";
        let from = DateTime::from_timestamp(1711929600, 0).unwrap();
        let to = from + Duration::days(30);
        let points = parse_ecb_csv(&AssetId::str("GBP"), csv, from, to)?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].datetime, from);
        assert!((points[0].price - 1.08 / 0.86).abs() < 1e-12);
        assert!((points[1].price - 1.10 / 0.88).abs() < 1e-12);
        Ok(())
    }
}
