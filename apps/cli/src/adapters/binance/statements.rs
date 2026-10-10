//! Binance statement CSVs and ZIPs -> exact ledger movements.
use super::*;
use binance_client::statement::StatementFile;
use lib_core::Store;
use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum StatementError {
    #[snafu(display("cannot load Binance statement {}", path.display()))]
    Load {
        path: PathBuf,
        source: binance_client::BinanceError,
    },
    #[snafu(display("cannot map Binance statement {}", path.display()))]
    Map {
        path: PathBuf,
        source: anyhow::Error,
    },
    #[snafu(display("cannot persist Binance statement"))]
    Store { source: anyhow::Error },
}

pub fn default_import_path() -> anyhow::Result<PathBuf> {
    let imports = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/imports");
    let mut paths = std::fs::read_dir(&imports)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|p| {
        p.file_name().is_some_and(|n| {
            n.to_string_lossy()
                .starts_with("Binance-Transaction-History")
        }) && p.extension().is_some_and(|e| e == "csv" || e == "zip")
    });
    paths.sort();
    // Prefer a ZIP so the ordinary test exercises decompression caching.
    paths
        .iter()
        .rev()
        .find(|p| p.extension().is_some_and(|e| e == "zip"))
        .or_else(|| paths.last())
        .cloned()
        .context("no Binance CSV or ZIP found in .cache/imports")
}

pub fn test_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/test-data/binance")
}

/// Import API movements using the same IDs as matching statement movements.
/// Matching requires the same UTC second, asset, amount and both accounts;
/// occurrence numbers preserve repeated identical movements within either source.
pub fn import_api_history(
    snapshot: &AccountSnapshot,
    store: &Store,
) -> Result<bool, StatementError> {
    let bytes = serde_json::to_vec(snapshot).map_err(|source| StatementError::Map {
        path: "API history".into(),
        source: source.into(),
    })?;
    let hash = binance_client::statement::fingerprint(&bytes);
    if snafu::ResultExt::context(
        store.has_imported_file("BINANCE-API-HISTORY-v1", &hash),
        StoreSnafu,
    )? {
        return Ok(false);
    }
    let mapped = (|| {
        let mut normalized = Vec::new();
        for event in events(snapshot)? {
            let datetime = DateTime::from_timestamp(event.datetime.timestamp(), 0)
                .context("API timestamp outside supported range")?;
            for movement in event.movements {
                normalized.push(Event {
                    datetime,
                    movements: vec![movement],
                });
            }
        }
        let scales = store
            .position_metadata()?
            .into_iter()
            .map(|(_, asset, scale, _)| (asset.0, scale))
            .collect();
        map_movements(normalized, vec![], snapshot.fetched_at, &scales)
    })()
    .map_err(|source| StatementError::Map {
        path: "API history".into(),
        source,
    })?;
    persist_mapped(store, &mapped, "BINANCE-API-HISTORY-v1", &hash)
}

pub fn import_statement(path: &Path, cache: &Path, store: &Store) -> Result<bool, StatementError> {
    let file = snafu::ResultExt::context(StatementFile::load(path, cache), LoadSnafu { path })?;
    if snafu::ResultExt::context(
        store.has_imported_file("BINANCE-STATEMENT-v1", &file.hash),
        StoreSnafu,
    )? {
        eprintln!("Binance statement already imported; reusing DB");
        return Ok(false);
    }
    let mapped = snafu::ResultExt::context(
        (|| {
            let rows = file.parse()?;
            eprintln!("Mapping {} Binance statement rows", rows.len());
            let mut events = Vec::new();
            let mut transfers =
                BTreeMap::<(DateTime<Utc>, String, String), Vec<(String, Decimal)>>::new();
            for row in &rows {
                let account = match row.account.as_str() {
                    "Spot" => "spot",
                    "Funding" => "funding",
                    _ => anyhow::bail!("unsupported statement account {}", row.account),
                };
                let n: Decimal = row.change.parse().context("invalid statement amount")?;
                if n.is_zero() {
                    continue;
                }
                let op = row.operation.as_str();
                if op == "Transfer Between Spot and Funding" {
                    transfers
                        .entry((
                            row.datetime,
                            row.coin.clone(),
                            n.abs().normalize().to_string(),
                        ))
                        .or_default()
                        .push((account.to_owned(), n));
                    continue;
                }
                let counterparty = match op {
                    "Simple Earn Flexible Subscription" | "Simple Earn Flexible Redemption" => {
                        "flexible"
                    }
                    "Staking Purchase"
                    | "Staking Redemption"
                    | "Simple Earn Locked Subscription"
                    | "Simple Earn Locked Redemption" => "locked",
                    "Deposit" => "external:deposit",
                    "Withdraw" => "external:withdrawal",
                    "Fee" | "Transaction Fee" => "external:fees",
                    "Buy"
                    | "Sell"
                    | "Transaction Buy"
                    | "Transaction Spend"
                    | "Transaction Revenue"
                    | "Transaction Sold"
                    | "Small Assets Exchange BNB"
                    | "Binance Convert"
                    | "Transaction Related" => "external:market",
                    "Spending" | "Merchant Payment" => "external:spending",
                    "Asset - Transfer"
                    | "Commission History"
                    | "Distribution"
                    | "Staking Rewards"
                    | "Simple Earn Flexible Interest"
                    | "Launchpool Interest"
                    | "Cash Voucher Distribution"
                    | "BNB Vault Rewards"
                    | "Simple Earn Locked Rewards"
                    | "Mission Reward Distribution"
                    | "Cashback"
                    | "Airdrop Assets"
                    | "Launchpool Airdrop - User Claim Distribution"
                    | "Launchpool Airdrop - System Distribution"
                    | "HODLer Airdrops Distribution" => "external:rewards",
                    _ => anyhow::bail!("unsupported statement operation {op}"),
                };
                if matches!(
                    op,
                    "Simple Earn Flexible Subscription"
                        | "Staking Purchase"
                        | "Simple Earn Locked Subscription"
                ) {
                    ensure!(
                        n.is_sign_negative(),
                        "subscription must debit source account"
                    );
                }
                if matches!(
                    op,
                    "Simple Earn Flexible Redemption"
                        | "Staking Redemption"
                        | "Simple Earn Locked Redemption"
                ) {
                    ensure!(
                        n.is_sign_positive(),
                        "redemption must credit destination account"
                    );
                }
                let (source, target) = if n.is_sign_negative() {
                    (account, counterparty)
                } else {
                    (counterparty, account)
                };
                events.push(Event {
                    datetime: row.datetime,
                    movements: vec![Movement {
                        source: source.into(),
                        target: target.into(),
                        asset: row.coin.clone(),
                        amount: n.abs(),
                    }],
                });
            }
            for ((datetime, asset, _), entries) in transfers {
                let mut debit: Vec<_> = entries
                    .iter()
                    .filter(|(_, n)| n.is_sign_negative())
                    .collect();
                let credit: Vec<_> = entries
                    .iter()
                    .filter(|(_, n)| n.is_sign_positive())
                    .collect();
                ensure!(
                    debit.len() == credit.len(),
                    "unpaired Spot/Funding transfer at {datetime}"
                );
                for incoming in credit {
                    let index = debit
                        .iter()
                        .position(|outgoing| outgoing.0 != incoming.0 && outgoing.1 == -incoming.1)
                        .context("mismatched Spot/Funding transfer")?;
                    let outgoing = debit.remove(index);
                    events.push(Event {
                        datetime,
                        movements: vec![Movement {
                            source: outgoing.0.clone(),
                            target: incoming.0.clone(),
                            asset: asset.clone(),
                            amount: incoming.1,
                        }],
                    });
                }
            }
            events.sort_by_key(|event| event.datetime);
            let end = rows
                .iter()
                .map(|r| r.datetime)
                .max()
                .context("empty Binance statement")?;
            let scales: BTreeMap<_, _> = store
                .position_metadata()?
                .into_iter()
                .map(|(_, asset, scale, _)| (asset.0, scale))
                .collect();
            let mapped = map_movements(events, vec![], end, &scales)?;
            mapped.reconcile().or_else(|error| match error {
                BinanceServiceError::Reconciliation { .. } => Ok(()),
                other => Err(other),
            })?;
            Ok::<_, anyhow::Error>(mapped)
        })(),
        MapSnafu { path },
    )?;
    persist_mapped(store, &mapped, "BINANCE-STATEMENT-v1", &file.hash)
}

fn persist_mapped(
    store: &Store,
    mapped: &BinanceAccountData,
    source: &str,
    hash: &str,
) -> Result<bool, StatementError> {
    let positions: Vec<_> = mapped
        .position_assets
        .iter()
        .map(|(position, asset)| {
            (
                position.clone(),
                asset.clone(),
                mapped
                    .data
                    .assets
                    .iter()
                    .find(|a| a.id == *asset)
                    .unwrap()
                    .decimals,
                mapped.owned.contains(position),
            )
        })
        .collect();
    let mut occurrences = HashMap::<String, usize>::new();
    let mut transactions = Vec::new();
    for transaction in &mapped.data.transactions {
        let encoded = serde_json::to_vec(transaction).map_err(|source| StatementError::Map {
            path: PathBuf::from("mapped ledger"),
            source: source.into(),
        })?;
        let hash = binance_client::statement::fingerprint(&encoded);
        let occurrence = occurrences.entry(hash.clone()).or_default();
        let id = format!("BINANCE-STATEMENT:{hash}:{occurrence}");
        *occurrence += 1;
        transactions.push((id, transaction.clone()));
    }
    snafu::ResultExt::context(
        store.save_file_import_with_progress(
            source,
            hash,
            &positions,
            &transactions,
            |done, total| eprintln!("Writing Binance statement transactions: {done}/{total}"),
        ),
        StoreSnafu,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn cached_positions() -> anyhow::Result<AccountSnapshot> {
        let path = test_data_dir().join("positions-v1.json");
        match std::fs::read(&path) {
            Ok(bytes) => return Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
        eprintln!("Fetching current Binance positions (no history backfill)");
        let snapshot = BinanceClient::new()?
            .with_backoff_progress(|endpoint, status, attempt, delay| {
                eprintln!(
                    "Binance {endpoint}: HTTP {status}, attempt {attempt}; cooldown {:.1}s",
                    delay.as_secs_f64()
                );
            })
            .fetch_current_positions()
            .await?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        serde_json::to_writer(file.as_file_mut(), &snapshot)?;
        file.as_file().sync_all()?;
        file.persist(&path)?;
        eprintln!("Cached positions at {}", snapshot.fetched_at);
        Ok(snapshot)
    }

    async fn load_supplement(store: &Store, positions: &AccountSnapshot) -> anyhow::Result<()> {
        let path = test_data_dir().join("earn-supplement-v1.json");
        let snapshot: AccountSnapshot = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let end = store
                    .latest_balances()?
                    .first()
                    .context("empty statement ledger")?
                    .datetime;
                eprintln!("Loading Earn interest and transfers omitted from the statement");
                let snapshot = BinanceClient::new()?
                    .with_history_cache(
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("../../.cache/api_responses/binance"),
                    )
                    .with_request_progress(|endpoint, cached| {
                        if !cached {
                            eprintln!("Loading {endpoint} from API");
                        }
                    })
                    .with_backoff_progress(|endpoint, status, attempt, delay| {
                        eprintln!(
                            "Binance {endpoint}: HTTP {status}, attempt {attempt}; cooldown {:.1}s",
                            delay.as_secs_f64()
                        );
                    })
                    .fetch_statement_supplement(end, positions)
                    .await?;
                let mut file = tempfile::NamedTempFile::new_in(test_data_dir())?;
                serde_json::to_writer(file.as_file_mut(), &snapshot)?;
                file.as_file().sync_all()?;
                file.persist(&path)?;
                snapshot
            }
            Err(error) => return Err(error.into()),
        };
        ensure!(
            snapshot.fetched_at == positions.fetched_at,
            "Earn cache and position snapshot have different cutoffs; refresh both together"
        );
        let hash = binance_client::statement::fingerprint(&serde_json::to_vec(&snapshot)?);
        if store.has_imported_file("BINANCE-EARN-SUPPLEMENT-v1", &hash)? {
            return Ok(());
        }
        import_api_history(&snapshot, store)?;
        // Separate fixed marker lets replay skip mapping the supplement entirely.
        store.save_file_import_with_progress(
            "BINANCE-EARN-SUPPLEMENT-v1",
            &hash,
            &[],
            &[],
            |_, _| {},
        )?;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "first run fetches private Binance positions; subsequent runs reuse .cache/test-data"]
    async fn fetch_cached_positions() -> anyhow::Result<()> {
        let snapshot = cached_positions().await?;
        let mapped = map_snapshot(&snapshot)?;
        for balance in mapped.current_balances {
            let asset = &mapped.position_assets[&balance.position_id];
            let scale = mapped
                .data
                .assets
                .iter()
                .find(|a| a.id == *asset)
                .unwrap()
                .decimals;
            if balance.amount != 0 {
                println!(
                    "{}: {}",
                    balance.position_id.0,
                    Decimal::try_from_i128_with_scale(balance.amount, u32::from(scale))?
                        .normalize()
                );
            }
        }
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires Binance CSV or ZIP in .cache/imports; API positions fetched once"]
    async fn latest_amount_after_cached_statement() -> anyhow::Result<()> {
        display_or_reconcile(false).await
    }

    #[tokio::test]
    #[ignore = "requires Binance statement; API positions and Earn supplement fetched once"]
    async fn cached_binance_statement_reconciles() -> anyhow::Result<()> {
        display_or_reconcile(true).await
    }

    async fn display_or_reconcile(assert_reconciliation: bool) -> anyhow::Result<()> {
        let path = std::env::var_os("BINANCE_STATEMENT")
            .map(PathBuf::from)
            .map(|path| {
                if path.is_relative() {
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../..")
                        .join(path)
                } else {
                    path
                }
            })
            .map(Ok)
            .unwrap_or_else(default_import_path)?;
        let data = test_data_dir();
        std::fs::create_dir_all(&data)?;
        let store = Store::open(data.join("test-binance-statement-v1.duckdb"))?;
        import_statement(&path, &data.join("statements"), &store)?;
        let positions = cached_positions().await?;
        load_supplement(&store, &positions).await?;
        let balances = store.latest_balances()?;
        let end = balances.first().context("empty statement ledger")?.datetime;
        ensure!(
            end <= positions.fetched_at,
            "ledger ends after cached API positions; refresh export and reference caches together"
        );
        println!("Binance transaction-derived asset balances at {end}:");
        for (asset, amount) in store.asset_units_at(end)? {
            if amount != 0 {
                println!(
                    "  {}: {}",
                    asset.0,
                    Decimal::try_from_i128_with_scale(
                        amount,
                        u32::from(store.asset_scale(&asset)?)
                    )?
                    .normalize()
                );
            }
        }
        println!(
            "Comparing statement plus recorded Earn supplement with API positions at {}",
            positions.fetched_at
        );
        let actual = map_snapshot(&positions)?;
        let scales: HashMap<_, _> = actual
            .data
            .assets
            .iter()
            .map(|a| (a.id.clone(), a.decimals))
            .collect();
        let mut observations = actual.current_balances;
        let metadata = store.position_metadata()?;
        for balance in &mut observations {
            let asset = &actual.position_assets[&balance.position_id];
            let scale = metadata
                .iter()
                .find(|(_, id, _, _)| id == asset)
                .map(|(_, _, scale, _)| *scale)
                .unwrap_or(scales[asset]);
            let value =
                Decimal::try_from_i128_with_scale(balance.amount, u32::from(scales[asset]))?;
            balance.amount = i128::from(units(value, scale)?);
        }
        let ledger = balances
            .into_iter()
            .map(|b| (b.position_id, b.amount))
            .collect();
        match reconcile_amounts(ledger, &observations) {
            Ok(()) => println!("All owned account balances match cached API positions"),
            Err(error) if assert_reconciliation => return Err(error.into()),
            Err(error) => eprintln!("{error}"),
        }
        Ok(())
    }

    #[test]
    fn imports_exact_splits_repeated_rows_and_overlapping_files_once() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("export(UTC).csv");
        let header = "UTC_Time,Account,Operation,Coin,Change,Remark\n";
        let rows = "2024-01-01 00:00:00,Spot,Deposit,BTC,1,\n2024-01-01 00:00:00,Spot,Deposit,BTC,1,\n2024-01-01 00:01:00,Spot,Transfer Between Spot and Funding,BTC,-0.5,\n2024-01-01 00:01:00,Funding,Transfer Between Spot and Funding,BTC,0.5,\n2024-01-01 00:02:00,Funding,Simple Earn Flexible Subscription,BTC,-0.2,\n2024-01-01 00:03:00,Spot,Fee,BTC,-0.00000001,\n";
        std::fs::write(&path, format!("{header}{rows}"))?;
        let store = Store::in_memory()?;
        assert!(import_statement(&path, dir.path(), &store)?);
        assert!(!import_statement(&path, dir.path(), &store)?);
        // A different file hash with overlapping rows must not double count.
        std::fs::write(
            &path,
            format!("{header}{rows}2024-01-01 00:04:00,Spot,Deposit,BTC,0.1,\n"),
        )?;
        assert!(import_statement(&path, dir.path(), &store)?);
        let balances: HashMap<_, _> = store
            .latest_balances()?
            .into_iter()
            .map(|b| (b.position_id.0, b.amount))
            .collect();
        assert_eq!(balances["BINANCE:spot:BTC"], 159999999);
        assert_eq!(balances["BINANCE:funding:BTC"], 30000000);
        assert_eq!(balances["BINANCE:flexible:BTC"], 20000000);
        assert!(!balances.keys().any(|key| key.contains("external")));
        Ok(())
    }

    #[test]
    fn unsupported_operations_and_unpaired_transfers_commit_nothing() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("export.csv");
        let store = Store::in_memory()?;
        for operation in ["Unknown Operation", "Transfer Between Spot and Funding"] {
            std::fs::write(
                &path,
                format!(
                    "UTC_Time,Account,Operation,Coin,Change,Remark\n2024-01-01 00:00:00,Spot,Deposit,BTC,1,\n2024-01-01 00:01:00,Spot,{operation},BTC,-0.5,\n"
                ),
            )?;
            assert!(matches!(
                import_statement(&path, dir.path(), &store),
                Err(StatementError::Map { .. })
            ));
            assert!(store.latest_balances()?.is_empty());
        }
        Ok(())
    }

    #[test]
    fn api_and_statement_deposits_deduplicate_in_either_order() -> anyhow::Result<()> {
        use serde_json::json;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("export.csv");
        std::fs::write(
            &path,
            "UTC_Time,Account,Operation,Coin,Change,Remark\n2024-01-01 00:00:00,Spot,Deposit,BTC,1,\n2024-01-01 00:00:00,Spot,Deposit,BTC,1,\n",
        )?;
        let datetime = "2024-01-01T00:00:00Z".parse::<DateTime<Utc>>()?;
        let snapshot = AccountSnapshot {
            version: 1,
            from: datetime,
            fetched_at: datetime,
            symbols: vec![],
            balances: BTreeMap::new(),
            history: BTreeMap::from([(
                "deposits".into(),
                vec![
                    json!({"id":"one", "coin":"BTC", "amount":"1", "status":1, "insertTime":datetime.timestamp_millis()+123}),
                    json!({"id":"two", "coin":"BTC", "amount":"1", "status":1, "insertTime":datetime.timestamp_millis()+456}),
                ],
            )]),
        };
        for api_first in [false, true] {
            let store = Store::in_memory()?;
            if api_first {
                import_api_history(&snapshot, &store)?;
            }
            import_statement(&path, dir.path(), &store)?;
            import_api_history(&snapshot, &store)?;
            assert_eq!(store.latest_balances()?[0].amount, 200000000);
        }
        Ok(())
    }

    #[test]
    fn earn_migration_and_funding_redemption_use_owned_counterparties() -> anyhow::Result<()> {
        use serde_json::json;
        let datetime = "2024-01-01T00:00:00Z".parse::<DateTime<Utc>>()?;
        let snapshot = AccountSnapshot {
            version: 1,
            from: datetime,
            fetched_at: datetime,
            symbols: vec![],
            balances: BTreeMap::new(),
            history: BTreeMap::from([
                (
                    "locked/redemptionRecord/".into(),
                    vec![
                        json!({"asset":"BNB", "type":"NEW_TRANSFERRED", "amount":"1", "originalAmount":"1", "lossAmount":"0", "status":"PAID", "isComplete":true, "time":datetime.timestamp_millis(), "deliverDate":datetime.timestamp_millis()}),
                    ],
                ),
                (
                    "flexible/redemptionRecord/".into(),
                    vec![
                        json!({"asset":"BNB", "destAccount":"FUNDING", "amount":"0.5", "status":"PAID", "time":datetime.timestamp_millis()}),
                    ],
                ),
            ]),
        };
        let store = Store::in_memory()?;
        import_api_history(&snapshot, &store)?;
        let balances: HashMap<_, _> = store
            .latest_balances()?
            .into_iter()
            .map(|b| (b.position_id.0, b.amount))
            .collect();
        assert_eq!(balances["BINANCE:locked:BNB"], -100000000);
        assert_eq!(balances["BINANCE:flexible:BNB"], 50000000);
        assert_eq!(balances["BINANCE:funding:BNB"], 50000000);
        assert!(!balances.contains_key("BINANCE:spot:BNB"));
        Ok(())
    }
}
