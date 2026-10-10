//! Local Binance transaction-history exports. No browser credentials are used.
use crate::{BinanceError, error::*};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use snafu::{OptionExt, ResultExt, ensure};
use std::{io::Read, path::Path};

pub fn fingerprint(bytes: &[u8]) -> String {
    hmac_sha256::HMAC::mac(bytes, "binance-statement-v1")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub struct StatementFile {
    pub hash: String,
    parts: Vec<Vec<u8>>,
    offset: Option<FixedOffset>,
}

#[derive(Debug, Clone)]
pub struct StatementRow {
    pub datetime: DateTime<Utc>,
    pub account: String,
    pub operation: String,
    pub coin: String,
    pub change: String,
    pub remark: String,
}

#[derive(Deserialize)]
struct CsvRow {
    #[serde(rename = "Time", alias = "UTC_Time")]
    time: String,
    #[serde(rename = "Account")]
    account: String,
    #[serde(rename = "Operation")]
    operation: String,
    #[serde(rename = "Coin")]
    coin: String,
    #[serde(rename = "Change")]
    change: String,
    #[serde(rename = "Remark", default)]
    remark: String,
}

fn offset_from_name(path: &Path) -> Option<FixedOffset> {
    let name = path.file_name()?.to_str()?;
    let start = name.find("(UTC")? + 4;
    let end = name[start..].find(')')? + start;
    let zone = &name[start..end];
    if zone.is_empty() {
        return FixedOffset::east_opt(0);
    }
    let (sign, value) = if let Some(value) = zone.strip_prefix('+') {
        (1, value)
    } else {
        (-1, zone.strip_prefix('-')?)
    };
    let mut parts = value.split(':');
    let hours: i32 = parts.next()?.parse().ok()?;
    let minutes: i32 = parts
        .next()
        .map(|s| s.parse())
        .transpose()
        .ok()?
        .unwrap_or(0);
    if !(0..24).contains(&hours) || !(0..60).contains(&minutes) || parts.next().is_some() {
        return None;
    }
    FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
}

impl StatementFile {
    pub fn load(path: &Path, csv_cache: &Path) -> Result<Self, BinanceError> {
        let bytes = std::fs::read(path).context(CacheReadSnafu { path })?;
        let offset = offset_from_name(path);
        let parts = if path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("zip"))
        {
            let archive_hash = fingerprint(&bytes);
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
                .context(StatementZipSnafu { path })?;
            let mut members: Vec<_> = (0..archive.len())
                .map(|index| {
                    let member = archive
                        .by_index(index)
                        .context(StatementZipSnafu { path })?;
                    Ok((member.name().to_owned(), index))
                })
                .collect::<Result<Vec<_>, BinanceError>>()?;
            members.retain(|(name, _)| name.to_ascii_lowercase().ends_with(".csv"));
            members.sort();
            ensure!(
                !members.is_empty(),
                InvalidStatementSnafu {
                    reason: "ZIP contains no CSV files"
                }
            );
            std::fs::create_dir_all(csv_cache).context(CacheWriteSnafu { path: csv_cache })?;
            let mut parts = Vec::new();
            for (part, (_, index)) in members.iter().enumerate() {
                // Archive member names are never used as filesystem paths.
                let cached = csv_cache.join(format!("{archive_hash}-{part}.csv"));
                let bytes = match std::fs::read(&cached) {
                    Ok(bytes) => bytes,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        let mut member = archive
                            .by_index(*index)
                            .context(StatementZipSnafu { path })?;
                        let mut bytes = Vec::new();
                        member
                            .read_to_end(&mut bytes)
                            .context(CacheReadSnafu { path })?;
                        let mut temporary = tempfile::NamedTempFile::new_in(csv_cache)
                            .context(CacheWriteSnafu { path: &cached })?;
                        std::io::Write::write_all(temporary.as_file_mut(), &bytes)
                            .context(CacheWriteSnafu { path: &cached })?;
                        temporary
                            .as_file()
                            .sync_all()
                            .context(CacheWriteSnafu { path: &cached })?;
                        temporary
                            .persist(&cached)
                            .map_err(|e| e.error)
                            .context(CacheWriteSnafu { path: &cached })?;
                        bytes
                    }
                    Err(source) => {
                        return Err(BinanceError::CacheRead {
                            path: cached,
                            source,
                        });
                    }
                };
                parts.push(bytes);
            }
            parts
        } else {
            ensure!(
                path.extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("csv")),
                InvalidStatementSnafu {
                    reason: "expected a CSV or ZIP statement"
                }
            );
            vec![bytes]
        };
        // Framing includes part lengths, so multi-part identities are unambiguous.
        let mut identity = Vec::new();
        identity.extend(
            offset
                .map(|v| v.local_minus_utc())
                .unwrap_or(i32::MIN)
                .to_le_bytes(),
        );
        for part in &parts {
            identity.extend((part.len() as u64).to_le_bytes());
            identity.extend(part);
        }
        Ok(Self {
            hash: fingerprint(&identity),
            parts,
            offset,
        })
    }

    pub fn parse(&self) -> Result<Vec<StatementRow>, BinanceError> {
        let mut result = Vec::new();
        for bytes in &self.parts {
            let mut reader = csv::ReaderBuilder::new()
                .trim(csv::Trim::All)
                .from_reader(bytes.as_slice());
            let mut headers = reader.headers().context(StatementCsvSnafu)?.clone();
            if let Some(first) = headers.get(0) {
                if first.starts_with('\u{feff}') {
                    let cleaned: Vec<_> = headers
                        .iter()
                        .map(|s| s.trim_start_matches('\u{feff}'))
                        .collect();
                    headers = csv::StringRecord::from(cleaned);
                    reader.set_headers(headers.clone());
                }
            }
            let offset = if headers.iter().any(|s| s == "UTC_Time") {
                FixedOffset::east_opt(0)
            } else {
                self.offset
            }
            .context(InvalidStatementSnafu {
                reason: "Time column requires (UTC+/-offset) in the original export filename",
            })?;
            for row in reader.deserialize::<CsvRow>() {
                let row = row.context(StatementCsvSnafu)?;
                let datetime = NaiveDateTime::parse_from_str(&row.time, "%Y-%m-%d %H:%M:%S")
                    .context(StatementDateSnafu)?;
                let datetime = offset
                    .from_local_datetime(&datetime)
                    .single()
                    .context(InvalidStatementSnafu {
                        reason: "invalid statement timestamp",
                    })?
                    .with_timezone(&Utc);
                ensure!(
                    !row.coin.is_empty() && !row.operation.is_empty(),
                    InvalidStatementSnafu {
                        reason: "statement coin/operation is empty"
                    }
                );
                result.push(StatementRow {
                    datetime,
                    account: row.account,
                    operation: row.operation,
                    coin: row.coin,
                    change: row.change,
                    remark: row.remark,
                });
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const CSV: &str = "\u{feff}User ID,Time,Account,Operation,Coin,Change,Remark\nignored,2024-01-02 03:04:05,Spot,Deposit,BTC,0.12345678,\nignored,2024-01-02 03:04:05,Spot,Deposit,BTC,0.12345678,\n";

    #[test]
    fn zip_and_csv_share_identity_cache_and_keep_repeated_rows() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let csv = dir.path().join("export(UTC+1).csv");
        let zip = dir.path().join("export(UTC+1).zip");
        std::fs::write(&csv, CSV)?;
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip)?);
        writer.start_file("../../escape.csv", zip::write::SimpleFileOptions::default())?;
        writer.write_all(CSV.as_bytes())?;
        writer.finish()?;
        let cache = dir.path().join("cache");
        let direct = StatementFile::load(&csv, &cache)?;
        let archived = StatementFile::load(&zip, &cache)?;
        assert_eq!(direct.hash, archived.hash);
        let rows = archived.parse()?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].datetime.to_rfc3339(), "2024-01-02T02:04:05+00:00");
        assert_eq!(rows[0].change, "0.12345678");
        assert_eq!(std::fs::read_dir(&cache)?.count(), 1);
        assert!(!dir.path().join("escape.csv").exists());
        assert_eq!(StatementFile::load(&zip, &cache)?.hash, archived.hash);
        Ok(())
    }

    #[test]
    fn timezone_is_required_unless_header_explicitly_utc() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("renamed.csv");
        std::fs::write(&path, CSV)?;
        assert!(matches!(
            StatementFile::load(&path, dir.path())?.parse(),
            Err(BinanceError::InvalidStatement { .. })
        ));
        std::fs::write(&path, CSV.replace(",Time,", ",UTC_Time,"))?;
        assert_eq!(
            StatementFile::load(&path, dir.path())?.parse()?[0]
                .datetime
                .to_rfc3339(),
            "2024-01-02T03:04:05+00:00"
        );
        assert!(offset_from_name(Path::new("(UTC+2147483647).csv")).is_none());
        assert!(offset_from_name(Path::new("(UTC+1:99).csv")).is_none());
        Ok(())
    }
}
