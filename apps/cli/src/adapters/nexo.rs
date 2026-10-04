//! Map Nexo's parsed CSV transactions into the core ledger.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use lib_core::history::AssetPricePoint;
use lib_core::traits::IsProvider;
use lib_core::{
    Asset, AssetId, CollectTxnData, PositionId, PriceRange, ProviderId, Store, Transaction,
    TxnEffect, UserPosition,
};
use nexo_csv::{NexoCsv, NexoCsvError, NexoCsvFile, NexoTx, TransactionType};
use snafu::{ResultExt, Snafu};

use crate::cli::Config;

const CSV_DECIMALS: u8 = 8;

#[derive(Debug, Snafu)]
pub enum NexoError {
    #[snafu(display("cannot read Nexo export at {} (pass --nexo-csv PATH to select it)", path.display()))]
    ReadCsv { path: PathBuf, source: NexoCsvError },
    #[snafu(display("cannot map Nexo transaction {transaction_id}"))]
    MapTransaction {
        transaction_id: String,
        source: NexoMapError,
    },
    #[snafu(display("cannot save Nexo transactions to DuckDB"))]
    Store { source: anyhow::Error },
}

#[derive(Debug, PartialEq, Eq)]
pub enum NexoImportOutcome {
    Imported(usize),
    AlreadyImported,
}

#[derive(Debug, Snafu)]
pub enum NexoMapError {
    #[snafu(display("invalid Nexo asset amount {amount}"))]
    InvalidAmount { amount: f64 },
    #[snafu(display("Nexo asset amount {amount} exceeds the ledger range"))]
    AmountTooLarge { amount: f64 },
}

pub struct NexoSvc {
    path_to_csv: PathBuf,
}

impl NexoSvc {
    pub fn new(config: &Config) -> Self {
        Self::from_path(&config.nexo_csv_path)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self {
            path_to_csv: path.as_ref().to_path_buf(),
        }
    }

    pub fn read_rows(&self) -> Result<Vec<NexoTx>, NexoError> {
        NexoCsv::from_file(&self.path_to_csv).context(ReadCsvSnafu {
            path: self.path_to_csv.clone(),
        })
    }

    /// Fallback observations derived from the export's USD Equivalent field.
    /// A prior row is carried to the start of a month with no market listing.
    pub fn implied_nexo_prices(
        &self,
        range: PriceRange,
    ) -> Result<Vec<AssetPricePoint>, NexoError> {
        Ok(implied_nexo_prices_from_rows(&self.read_rows()?, range))
    }

    pub fn import_into(&self, store: &Store) -> Result<NexoImportOutcome, NexoError> {
        self.import_into_with_progress(store, |_, _| {})
    }

    pub fn import_into_with_progress(
        &self,
        store: &Store,
        on_progress: impl FnMut(usize, usize),
    ) -> Result<NexoImportOutcome, NexoError> {
        let file = NexoCsvFile::read(&self.path_to_csv).context(ReadCsvSnafu {
            path: self.path_to_csv.clone(),
        })?;
        let hash = &file.sha256_hex;
        if store.has_imported_file("NEXO", hash).context(StoreSnafu)? {
            return Ok(NexoImportOutcome::AlreadyImported);
        }
        let rows = file.parse().context(ReadCsvSnafu {
            path: self.path_to_csv.clone(),
        })?;
        let (assets, transactions) = map_rows(&rows)?;
        let asset_positions: Vec<_> = assets
            .into_iter()
            .map(|ticker| {
                (
                    PositionId::from(format!("NEXO:{ticker}")),
                    AssetId::str(&ticker),
                    CSV_DECIMALS,
                    true,
                )
            })
            .collect();
        let imported = store
            .save_file_import_with_progress(
                "NEXO",
                hash,
                &asset_positions,
                &transactions,
                on_progress,
            )
            .context(StoreSnafu)?;
        Ok(if imported {
            NexoImportOutcome::Imported(rows.len())
        } else {
            NexoImportOutcome::AlreadyImported
        })
    }
}

fn implied_nexo_prices_from_rows(rows: &[NexoTx], range: PriceRange) -> Vec<AssetPricePoint> {
    let mut by_time: BTreeMap<_, (f64, f64)> = BTreeMap::new();
    for row in rows {
        if row.output_currency != "NEXO"
            || row.output_amount <= 0.0
            || row.date_time_utc >= range.to
        {
            continue;
        }
        let Some(usd) = row
            .usd_equivalent
            .trim()
            .trim_start_matches('$')
            .replace(',', "")
            .parse::<f64>()
            .ok()
        else {
            continue;
        };
        if !usd.is_finite() || usd <= 0.0 {
            continue;
        }
        let entry = by_time.entry(row.date_time_utc).or_default();
        entry.0 += usd;
        entry.1 += row.output_amount;
    }
    let mut points = Vec::new();
    for (datetime, (usd, amount)) in by_time {
        let price = usd / amount;
        if !price.is_finite() || price <= 0.0 {
            continue;
        }
        let datetime = if datetime < range.from {
            range.from
        } else {
            datetime
        };
        let point = AssetPricePoint {
            datetime,
            asset_id: AssetId::str("NEXO"),
            vs_asset_id: AssetId::str("USD"),
            price,
        };
        if points
            .last()
            .is_some_and(|p: &AssetPricePoint| p.datetime == datetime)
        {
            *points.last_mut().unwrap() = point;
        } else {
            points.push(point);
        }
    }
    points
}

#[async_trait::async_trait]
impl IsProvider for NexoSvc {
    fn provider_id(&self) -> ProviderId {
        ProviderId::from("NEXO")
    }

    async fn fetch_positions(&self) -> anyhow::Result<Vec<UserPosition>> {
        anyhow::bail!("Nexo CSV does not identify individual wallet or product positions")
    }

    async fn fetch_all_txn_data(&self) -> anyhow::Result<CollectTxnData> {
        let rows = self.read_rows()?;
        let mut assets = BTreeSet::new();
        let mut transactions = Vec::with_capacity(rows.len());
        for row in &rows {
            transactions.push(convert_row(row, &mut assets).context(MapTransactionSnafu {
                transaction_id: row.tx_id.clone(),
            })?);
        }
        Ok(CollectTxnData {
            transactions,
            assets: assets
                .into_iter()
                .map(|ticker| Asset {
                    id: AssetId::str(&ticker),
                    decimals: CSV_DECIMALS,
                    symbol: ticker.clone(),
                    ticker,
                })
                .collect(),
            // The CSV does not identify individual wallets or current APYs.
            positions: Vec::new(),
            products: Vec::new(),
        })
    }
}

pub fn import_nexo_rows(store: &Store, rows: &[NexoTx]) -> Result<usize, NexoError> {
    import_nexo_rows_with_progress(store, rows, |_, _| {})
}

fn import_nexo_rows_with_progress(
    store: &Store,
    rows: &[NexoTx],
    on_progress: impl FnMut(usize, usize),
) -> Result<usize, NexoError> {
    let (assets, transactions) = map_rows(rows)?;
    for ticker in assets {
        let asset = AssetId::str(&ticker);
        store
            .save_asset_scale(&asset, CSV_DECIMALS)
            .context(StoreSnafu)?;
        store
            .save_position_asset(&PositionId::from(format!("NEXO:{ticker}")), &asset, true)
            .context(StoreSnafu)?;
    }
    store
        .save_transactions_with_progress(&transactions, on_progress)
        .context(StoreSnafu)?;
    Ok(rows.len())
}

fn map_rows(rows: &[NexoTx]) -> Result<(BTreeSet<String>, Vec<(String, Transaction)>), NexoError> {
    let mut assets = BTreeSet::new();
    let mut transactions = Vec::with_capacity(rows.len());
    for row in rows {
        let transaction = convert_row(row, &mut assets).context(MapTransactionSnafu {
            transaction_id: row.tx_id.clone(),
        })?;
        transactions.push((format!("NEXO:{}", row.tx_id), transaction));
    }
    Ok((assets, transactions))
}

fn units(amount: f64) -> Result<u64, NexoMapError> {
    if !amount.is_finite() || amount < 0.0 {
        return Err(NexoMapError::InvalidAmount { amount });
    }
    let scaled = (amount * 100_000_000.0).round();
    if scaled > i64::MAX as f64 {
        return Err(NexoMapError::AmountTooLarge { amount });
    }
    Ok(scaled as u64)
}

fn convert_row(row: &NexoTx, assets: &mut BTreeSet<String>) -> Result<Transaction, NexoMapError> {
    let mut changes: Vec<(&str, f64)> = Vec::new();
    use TransactionType::*;
    match row.kind {
        Interest | FixedTermInterest | Cashback | ExchangeCashback | ReferralBonus | Dividend
        | TopUpCrypto => {
            let amount = if row.output_amount == 0.0 {
                row.input_amount
            } else {
                row.output_amount
            };
            changes.push((&row.output_currency, amount));
        }
        DepositToExchange => changes.push((&row.input_currency, row.input_amount)),
        Exchange
        | ExchangeDepositedOn
        | ExchangeCredit
        | ExchangeToWithdraw
        | CreditCardFiatExchangeToWithdraw => {
            changes.push((&row.input_currency, -row.input_amount.abs()));
            changes.push((&row.output_currency, row.output_amount));
        }
        Withdrawal
        | WithdrawExchanged
        | NexoCardPurchase
        | NexoCardTransactionFee
        | CreditCardWithdrawalCredit => {
            changes.push((&row.input_currency, -row.input_amount.abs()));
        }
        // Both sides remain owned by the user. The export omits wallet IDs, so
        // these moves have no effect on the aggregate asset amount.
        LockTermDeposit
        | UnlockTermDeposit
        | TransferFromProWallet
        | TransferToProWallet
        | TransferOut => {}
    }

    let mut transaction = Transaction {
        datetime: row.date_time_utc,
        inputs: Vec::new(),
        outputs: Vec::new(),
    };
    for (ticker, change) in changes {
        if change == 0.0 {
            continue;
        }
        assets.insert(ticker.to_string());
        let position = PositionId::from(format!("NEXO:{ticker}"));
        let effect = TxnEffect {
            position_id: position,
            amount: units(change.abs())?,
            datetime: row.date_time_utc,
        };
        if change < 0.0 {
            transaction.inputs.push(effect);
        } else {
            transaction.outputs.push(effect);
        }
    }
    Ok(transaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[test]
    fn implied_nexo_rate_carries_latest_csv_value_across_empty_month() -> anyhow::Result<()> {
        let rows = vec![
            NexoTx::try_from_csv_row(
                "a,Top up Crypto,NEXO,10,NEXO,10,$20,-,-,approved,2020-08-04 12:00:00",
            )?,
            NexoTx::try_from_csv_row(
                "b,Top up Crypto,NEXO,10,NEXO,10,$30,-,-,approved,2020-08-27 12:00:00",
            )?,
        ];
        let range = PriceRange::new(
            DateTime::parse_from_rfc3339("2020-09-01T00:00:00Z")?.to_utc(),
            DateTime::parse_from_rfc3339("2020-10-01T00:00:00Z")?.to_utc(),
        )?;
        let points = implied_nexo_prices_from_rows(&rows, range);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].datetime, range.from);
        assert_eq!(points[0].price, 3.0);
        Ok(())
    }

    #[test]
    fn latest_amount_after_cached_export() -> anyhow::Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/nexo_transactions.csv");
        if !path.exists() {
            return Ok(());
        }
        let service = NexoSvc::from_path(&path);
        let rows = service.read_rows()?;
        let store = Store::in_memory()?;
        service.import_into(&store)?;
        let end = rows.iter().map(|r| r.date_time_utc).max().unwrap();
        let amounts = store.asset_amounts_at(end)?;
        let latest = |ticker: &str| {
            amounts
                .iter()
                .find(|(asset, _)| asset.0 == ticker)
                .map(|(_, n)| *n)
                .unwrap_or(0.0)
        };
        // Anchors from this export, independently summed from the CSV movements.
        assert!((latest("ETH") - 176.88934455).abs() < 1e-8);
        assert!((latest("NEXO") - 231832.90565071).abs() < 1e-8);
        Ok(())
    }

    #[test]
    fn internal_transfer_does_not_change_amount() -> anyhow::Result<()> {
        let row = NexoTx::try_from_csv_row(
            "id,Locking Term Deposit,DOT,-4,DOT,4,$10,-,-,approved / transfer,2025-01-01 00:00:00",
        )?;
        let store = Store::in_memory()?;
        import_nexo_rows(&store, &[row])?;
        let at = DateTime::<Utc>::from_timestamp(1735689600, 0).unwrap();
        assert!(store.asset_amounts_at(at)?.is_empty());
        Ok(())
    }

    #[test]
    fn reimport_replaces_effects() -> anyhow::Result<()> {
        let row = NexoTx::try_from_csv_row(
            "id,Top up Crypto,ETH,2,ETH,2,$200,-,-,approved / deposit,2025-01-01 00:00:00",
        )?;
        let store = Store::in_memory()?;
        import_nexo_rows(&store, &[row])?;
        let row = NexoTx::try_from_csv_row(
            "id,Top up Crypto,ETH,2,ETH,2,$200,-,-,approved / deposit,2025-01-01 00:00:00",
        )?;
        import_nexo_rows(&store, &[row])?;
        let at = DateTime::<Utc>::from_timestamp(1735689600, 0).unwrap();
        assert_eq!(
            store.asset_amounts_at(at)?,
            vec![(AssetId::str("ETH"), 2.0)]
        );
        Ok(())
    }

    #[test]
    fn missing_export_reports_the_path_and_override() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing-nexo-test.csv");
        let error = NexoSvc::from_path(&path).read_rows().unwrap_err();
        assert!(matches!(error, NexoError::ReadCsv { .. }));
        assert!(error.to_string().contains(&path.display().to_string()));
        assert!(error.to_string().contains("--nexo-csv"));
    }

    #[test]
    fn file_hash_skips_identical_exports_and_imports_changed_bytes() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("nexo.csv");
        let header = "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\n";
        let row = |amount| {
            format!(
                "id,Top up Crypto,ETH,{amount},ETH,{amount},$200,-,-,approved / deposit,2025-01-01 00:00:00\n"
            )
        };
        let bytes = format!("{header}{}", row(2));
        std::fs::write(&path, &bytes)?;
        let store = Store::in_memory()?;
        let service = NexoSvc::from_path(&path);
        assert_eq!(service.import_into(&store)?, NexoImportOutcome::Imported(1));
        let first_hash = NexoCsvFile::read(&path)?.sha256_hex;
        assert!(store.has_imported_file("NEXO", &first_hash)?);
        assert_eq!(
            service.import_into(&store)?,
            NexoImportOutcome::AlreadyImported
        );

        let copy = dir.path().join("same-content.csv");
        std::fs::write(&copy, &bytes)?;
        assert_eq!(
            NexoSvc::from_path(copy).import_into(&store)?,
            NexoImportOutcome::AlreadyImported
        );

        let changed = format!("{header}{}", row(3));
        std::fs::write(&path, &changed)?;
        assert_eq!(service.import_into(&store)?, NexoImportOutcome::Imported(1));
        let changed_hash = NexoCsvFile::read(&path)?.sha256_hex;
        assert!(store.has_imported_file("NEXO", &changed_hash)?);
        let at = DateTime::<Utc>::from_timestamp(1735689600, 0).unwrap();
        assert_eq!(
            store.asset_amounts_at(at)?,
            vec![(AssetId::str("ETH"), 3.0)]
        );
        Ok(())
    }

    #[test]
    fn invalid_export_does_not_store_its_hash() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("bad.csv");
        let bytes = b"Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\nid,Top up Crypto,ETH,nope,ETH,nope,$200,-,-,approved / deposit,2025-01-01 00:00:00\n";
        std::fs::write(&path, bytes)?;
        let hash = NexoCsvFile::read(&path)?.sha256_hex;
        let store = Store::in_memory()?;
        assert!(NexoSvc::from_path(path).import_into(&store).is_err());
        assert!(!store.has_imported_file("NEXO", &hash)?);
        Ok(())
    }

    #[test]
    fn imported_file_hash_survives_database_reopen() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let csv_path = dir.path().join("nexo.csv");
        let db_path = dir.path().join("portfolio.duckdb");
        std::fs::write(
            &csv_path,
            "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\nid,Top up Crypto,ETH,2,ETH,2,$200,-,-,approved / deposit,2025-01-01 00:00:00\n",
        )?;
        let service = NexoSvc::from_path(&csv_path);
        {
            let store = Store::open(&db_path)?;
            assert_eq!(service.import_into(&store)?, NexoImportOutcome::Imported(1));
        }
        let store = Store::open(&db_path)?;
        assert_eq!(
            service.import_into(&store)?,
            NexoImportOutcome::AlreadyImported
        );
        let at = DateTime::<Utc>::from_timestamp(1735689600, 0).unwrap();
        assert_eq!(
            store.asset_amounts_at(at)?,
            vec![(AssetId::str("ETH"), 2.0)]
        );
        Ok(())
    }
}
