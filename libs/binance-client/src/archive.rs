//! Public spot candle archives from data.binance.vision. No Binance account is needed.

use std::io::{Cursor, Read};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use snafu::{ResultExt, Snafu};

const BASE_URL: &str = "https://data.binance.vision";

#[derive(Debug, Snafu)]
pub enum ArchiveError {
    #[snafu(display("invalid Binance symbol {symbol}"))]
    Symbol { symbol: String },
    #[snafu(display("requesting Binance archive {url}"))]
    Request { url: String, source: reqwest::Error },
    #[snafu(display("Binance archive {url} returned HTTP {status}: {body}"))]
    Http {
        url: String,
        status: u16,
        body: String,
    },
    #[snafu(display("reading Binance ZIP archive"))]
    Zip { source: zip::result::ZipError },
    #[snafu(display("reading Binance candle CSV"))]
    Io { source: std::io::Error },
    #[snafu(display("parsing Binance candle CSV"))]
    Csv { source: csv::Error },
    #[snafu(display("invalid Binance candle row: {reason}"))]
    Candle { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candle {
    /// The final traded price in the interval, known at `time`.
    pub time: DateTime<Utc>,
    pub close: f64,
}

pub struct ArchiveClient {
    http: reqwest::Client,
    base_url: String,
}

impl ArchiveClient {
    pub fn new() -> Result<Self, ArchiveError> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .user_agent("crypto-tracker/0.1 (+https://github.com/nmrshll/crypto-tracker)")
            .build()
            .context(RequestSnafu { url: BASE_URL })?;
        Ok(Self {
            http,
            base_url: BASE_URL.to_string(),
        })
    }

    /// Fetch a month of daily spot closes. `None` means this symbol/month is absent.
    pub async fn fetch_month(
        &self,
        symbol: &str,
        month: NaiveDate,
    ) -> Result<Option<Vec<Candle>>, ArchiveError> {
        let filename = month_filename(symbol, month)?;
        let url = format!(
            "{}/data/spot/monthly/klines/{symbol}/1d/{filename}",
            self.base_url
        );
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context(RequestSnafu { url: &url })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.context(RequestSnafu { url: &url })?;
            return Err(ArchiveError::Http {
                url,
                status: status.as_u16(),
                body,
            });
        }
        let bytes = response.bytes().await.context(RequestSnafu { url: &url })?;
        parse_zip(&bytes, &filename).map(Some)
    }
}

fn month_filename(symbol: &str, month: NaiveDate) -> Result<String, ArchiveError> {
    if symbol.is_empty()
        || !symbol
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return Err(ArchiveError::Symbol {
            symbol: symbol.to_string(),
        });
    }
    Ok(format!(
        "{symbol}-1d-{:04}-{:02}.zip",
        month.year(),
        month.month()
    ))
}

fn parse_zip(bytes: &[u8], filename: &str) -> Result<Vec<Candle>, ArchiveError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context(ZipSnafu)?;
    let csv_name = filename.replace(".zip", ".csv");
    let mut file = archive.by_name(&csv_name).context(ZipSnafu)?;
    let mut csv_bytes = Vec::new();
    file.read_to_end(&mut csv_bytes).context(IoSnafu)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv_bytes.as_slice());
    let mut candles = Vec::new();
    for row in reader.records() {
        let row = row.context(CsvSnafu)?;
        let timestamp = row
            .get(6)
            .ok_or_else(|| ArchiveError::Candle {
                reason: "missing close time".into(),
            })?
            .parse::<i64>()
            .map_err(|e| ArchiveError::Candle {
                reason: format!("close time: {e}"),
            })?;
        // Binance spot archives changed from milliseconds to microseconds in 2025.
        let time = if timestamp >= 100_000_000_000_000 {
            DateTime::from_timestamp_micros(timestamp)
        } else {
            DateTime::from_timestamp_millis(timestamp)
        }
        .ok_or_else(|| ArchiveError::Candle {
            reason: "close time out of range".into(),
        })?;
        let close = row
            .get(4)
            .ok_or_else(|| ArchiveError::Candle {
                reason: "missing close price".into(),
            })?
            .parse::<f64>()
            .map_err(|e| ArchiveError::Candle {
                reason: format!("close price: {e}"),
            })?;
        if !close.is_finite() || close <= 0.0 {
            return Err(ArchiveError::Candle {
                reason: format!("invalid close price {close}"),
            });
        }
        candles.push(Candle { time, close });
    }
    candles.sort_by_key(|c| c.time);
    Ok(candles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn month_url_filename_and_candles_with_both_timestamp_units()
    -> Result<(), Box<dyn std::error::Error>> {
        let name = month_filename("BTCUSDT", NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())?;
        assert_eq!(name, "BTCUSDT-1d-2025-01.zip");
        assert!(month_filename("btc/usdt", NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()).is_err());
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        writer.start_file(
            "BTCUSDT-1d-2025-01.csv",
            zip::write::SimpleFileOptions::default(),
        )?;
        writer.write_all(b"1735689600000,1,2,1,2.5,10,1735775999999,1,1,1,1,0\n1735776000000000,1,2,1,3.5,10,1735862399999000,1,1,1,1,0\n")?;
        let bytes = writer.finish()?.into_inner();
        let candles = parse_zip(&bytes, &name)?;
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].close, 2.5);
        assert_eq!(candles[1].close, 3.5);
        assert_eq!(candles[0].time.timestamp_millis(), 1735775999999);
        assert_eq!(candles[1].time.timestamp_micros(), 1735862399999000);
        Ok(())
    }
}
