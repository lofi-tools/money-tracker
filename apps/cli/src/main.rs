use adapters::binance_vision::{BinanceVisionSvc, month_range};
use adapters::coingecko::CoinGeckoSvc;
use adapters::nexo::{NexoError, NexoImportOutcome, NexoSvc};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use clap::Parser;
use cli::{Args, Config};
use lib_core::{AssetId, PriceRange, Store};
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;
use std::time::Instant;

pub mod adapters {
    pub mod binance;
    pub mod binance_vision;
    pub mod coingecko;
    pub mod fallback_prices;
    pub mod nexo;
}
mod cli;
mod models;

#[derive(Debug, Snafu)]
enum AppError {
    #[snafu(display("invalid UTC date '{input}' (expected YYYY-MM-DD)"))]
    Date {
        input: String,
        source: chrono::ParseError,
    },
    #[snafu(display("cannot create data directory {}", path.display()))]
    CreateDataDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("cannot open DuckDB database {}", path.display()))]
    OpenDatabase {
        path: PathBuf,
        source: anyhow::Error,
    },
    #[snafu(display("Nexo import failed"))]
    ImportNexo { source: NexoError },
    #[snafu(display("cannot initialize CoinGecko client"))]
    CoinGeckoClient { source: anyhow::Error },
    #[snafu(display("cannot initialize Binance public archive client"))]
    BinanceArchiveClient { source: anyhow::Error },
    #[snafu(display("cannot fetch and cache Binance archive prices for {asset} in {month}"))]
    BinanceArchivePrices {
        asset: String,
        month: String,
        source: anyhow::Error,
    },
    #[snafu(display("cannot derive NEXO prices from the local export"))]
    NexoImpliedPrices { source: NexoError },
    #[snafu(display("no historical USD price source covers {asset} in {month}"))]
    UnpricedMonth { asset: String, month: String },
    #[snafu(display("cannot fetch historical CoinGecko prices"))]
    FetchHistoricalPrices { source: anyhow::Error },
    #[snafu(display("cannot fetch current CoinGecko prices"))]
    FetchCurrentPrices { source: anyhow::Error },
    #[snafu(display("cannot save CoinGecko prices to DuckDB"))]
    SavePrices { source: anyhow::Error },
    #[snafu(display("cannot read owned asset history from DuckDB"))]
    ReadAssets { source: anyhow::Error },
    #[snafu(display("cannot plan uncached CoinGecko price ranges"))]
    PlanPrices { source: anyhow::Error },
    #[snafu(display("cannot calculate historical net worth"))]
    NetWorth { source: anyhow::Error },
}

fn parse_utc_day(s: &str) -> Result<DateTime<Utc>, AppError> {
    let day = NaiveDate::parse_from_str(s, "%Y-%m-%d").context(DateSnafu {
        input: s.to_string(),
    })?;
    Ok(DateTime::from_naive_utc_and_offset(
        day.and_hms_opt(0, 0, 0).unwrap(),
        Utc,
    ))
}

#[tokio::main]
async fn main() -> snafu::Report<AppError> {
    snafu::Report::from(run().await)
}

async fn run() -> Result<(), AppError> {
    let args = Args::parse();
    let historical_from = args
        .historical_from
        .as_deref()
        .map(parse_utc_day)
        .transpose()?;
    let historical_to = args
        .historical_to
        .as_deref()
        .map(parse_utc_day)
        .transpose()?
        .unwrap_or_else(Utc::now);
    let fetch_current = args.historical_to.is_none();
    let offline = args.offline;
    let config = Config::from_env(args);
    eprintln!(
        "Opening DuckDB at {}",
        config.data_dir.join("portfolio.duckdb").display()
    );
    std::fs::create_dir_all(&config.data_dir).context(CreateDataDirSnafu {
        path: config.data_dir.clone(),
    })?;
    let db_path = config.data_dir.join("portfolio.duckdb");
    let store = Store::open(&db_path).context(OpenDatabaseSnafu { path: db_path })?;
    eprintln!(
        "Checking Nexo transactions from {}...",
        config.nexo_csv_path.display()
    );
    let started = Instant::now();
    let outcome = NexoSvc::new(&config)
        .import_into_with_progress(&store, |done, total| {
            eprintln!("Writing Nexo transactions: {done}/{total}");
        })
        .context(ImportNexoSnafu)?;
    match outcome {
        NexoImportOutcome::Imported(imported) => eprintln!(
            "Imported {imported} Nexo transactions in {:.1}s",
            started.elapsed().as_secs_f64()
        ),
        NexoImportOutcome::AlreadyImported => {
            eprintln!("Nexo CSV already imported; skipped transaction writes");
        }
    }

    let asset_starts = store.owned_assets_first_seen().context(ReadAssetsSnafu)?;
    let assets: Vec<AssetId> = asset_starts
        .iter()
        .map(|(asset, _)| asset.clone())
        .collect();
    if !offline && !assets.is_empty() {
        eprintln!("Checking cached USD prices for {} assets...", assets.len());
        let mut coingecko = None;
        let binance_archive = BinanceVisionSvc::new().context(BinanceArchiveClientSnafu)?;
        let usd = AssetId::str("USD");
        // CoinGecko's public historical endpoint rejects queries older than
        // 365 days. Leave one day of margin for its rolling UTC boundary.
        let coingecko_from = Utc::now() - Duration::days(364);
        for (asset, first_seen) in &asset_starts {
            let lookback = first_seen
                .checked_sub_signed(Duration::days(1))
                .unwrap_or(*first_seen);
            let from = historical_from.map_or(lookback, |requested| requested.min(lookback));
            if from >= historical_to {
                continue;
            }
            let wanted = PriceRange::new(from, historical_to).context(PlanPricesSnafu)?;
            let missing_ranges = store
                .missing_price_ranges(asset, &usd, wanted)
                .context(PlanPricesSnafu)?;
            if missing_ranges.is_empty() {
                eprintln!("{}: historical USD prices already cached", asset.0);
            }
            for missing in missing_ranges
                .iter()
                .copied()
                .filter(|r| r.from < coingecko_from)
            {
                let mut month = missing.from.date_naive().with_day(1).unwrap();
                while month_range(month).context(PlanPricesSnafu)?.from
                    < missing.to.min(coingecko_from)
                {
                    let range = month_range(month).context(PlanPricesSnafu)?;
                    if !store
                        .missing_price_ranges(asset, &usd, range)
                        .context(PlanPricesSnafu)?
                        .is_empty()
                    {
                        eprintln!(
                            "{}: filling historical USD prices for {}...",
                            asset.0,
                            month.format("%Y-%m")
                        );
                        let started = Instant::now();
                        let fetched = binance_archive
                            .fill_usd_month(&store, asset, month)
                            .await
                            .context(BinanceArchivePricesSnafu {
                                asset: asset.0.clone(),
                                month: month.format("%Y-%m").to_string(),
                            })?;
                        let (count, source) = match fetched {
                            Some(fetch) => {
                                if asset.0 == "NEXO" {
                                    let stored = store
                                        .prices_in_range(asset, &usd, range)
                                        .context(PlanPricesSnafu)?;
                                    if let Some(first) = stored.first()
                                        && first.datetime > range.from + Duration::days(2)
                                    {
                                        let early: Vec<_> = NexoSvc::new(&config)
                                            .implied_nexo_prices(range)
                                            .context(NexoImpliedPricesSnafu)?
                                            .into_iter()
                                            .filter(|point| point.datetime < first.datetime)
                                            .collect();
                                        store.save_prices(&early).context(SavePricesSnafu)?;
                                        if !early.is_empty() {
                                            eprintln!(
                                                "NEXO: filled {} earlier prices from USD equivalents in the local CSV",
                                                early.len()
                                            );
                                        }
                                    }
                                }
                                (fetch.count, fetch.source)
                            }
                            None if asset.0 == "NEXO" => {
                                let points = NexoSvc::new(&config)
                                    .implied_nexo_prices(range)
                                    .context(NexoImpliedPricesSnafu)?;
                                if points.is_empty() {
                                    if range.to <= *first_seen {
                                        eprintln!(
                                            "{}: no preceding price before the first transaction in {}; skipping lookback",
                                            asset.0,
                                            month.format("%Y-%m")
                                        );
                                        month = range.to.date_naive();
                                        continue;
                                    }
                                    return Err(AppError::UnpricedMonth {
                                        asset: asset.0.clone(),
                                        month: month.format("%Y-%m").to_string(),
                                    });
                                }
                                store
                                    .save_historical_prices(asset, &usd, range, &points)
                                    .context(SavePricesSnafu)?;
                                (points.len(), "Nexo CSV USD Equivalent")
                            }
                            None => {
                                if range.to <= *first_seen {
                                    eprintln!(
                                        "{}: no preceding price before the first transaction in {}; skipping lookback",
                                        asset.0,
                                        month.format("%Y-%m")
                                    );
                                    month = range.to.date_naive();
                                    continue;
                                }
                                return Err(AppError::UnpricedMonth {
                                    asset: asset.0.clone(),
                                    month: month.format("%Y-%m").to_string(),
                                });
                            }
                        };
                        eprintln!(
                            "{}: cached {count} {source} observations in {:.1}s",
                            asset.0,
                            started.elapsed().as_secs_f64()
                        );
                    }
                    month = range.to.date_naive();
                }
            }
            let recent_wanted = if wanted.to > coingecko_from {
                Some(
                    PriceRange::new(wanted.from.max(coingecko_from), wanted.to)
                        .context(PlanPricesSnafu)?,
                )
            } else {
                None
            };
            let recent_missing = match recent_wanted {
                Some(range) => store
                    .missing_price_ranges(asset, &usd, range)
                    .context(PlanPricesSnafu)?,
                None => Vec::new(),
            };
            for missing in recent_missing {
                if coingecko.is_none() {
                    coingecko = Some(CoinGeckoSvc::new().context(CoinGeckoClientSnafu)?);
                }
                eprintln!(
                    "{}: fetching USD prices from {} to {}...",
                    asset.0, missing.from, missing.to
                );
                let started = Instant::now();
                let prices = coingecko
                    .as_ref()
                    .unwrap()
                    .fetch_historical_prices(std::slice::from_ref(asset), missing.from, missing.to)
                    .await
                    .context(FetchHistoricalPricesSnafu)?;
                store
                    .save_historical_prices(asset, &usd, missing, &prices)
                    .context(SavePricesSnafu)?;
                eprintln!(
                    "{}: cached {} price observations in {:.1}s",
                    asset.0,
                    prices.len(),
                    started.elapsed().as_secs_f64()
                );
            }
        }
        if fetch_current {
            if coingecko.is_none() {
                coingecko = Some(CoinGeckoSvc::new().context(CoinGeckoClientSnafu)?);
            }
            eprintln!("Fetching current USD prices for {} assets...", assets.len());
            let current_prices = coingecko
                .as_ref()
                .unwrap()
                .fetch_current_prices(&assets)
                .await
                .context(FetchCurrentPricesSnafu)?;
            store
                .save_prices(&current_prices)
                .context(SavePricesSnafu)?;
            eprintln!(
                "Cached {} current USD price observations",
                current_prices.len()
            );
        }
    } else if offline {
        eprintln!("Offline mode: using stored prices only");
    } else {
        eprintln!("No assets with owned transactions to price");
    }
    if historical_from.is_some() {
        eprintln!("Calculating historical net worth in USD...");
        let history = store
            .total_value_history(&AssetId::str("USD"))
            .context(NetWorthSnafu)?;
        println!("Historical net worth: {} points", history.len());
        if let Some((when, worth)) = history.last() {
            println!("{}: ${:.2}", when, worth);
        }
    }
    Ok(())
}
