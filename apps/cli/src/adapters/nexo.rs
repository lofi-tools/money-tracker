//! Map Nexo's parsed CSV transactions into the core ledger.

use std::collections::BTreeMap;
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
        let mut asset_positions: Vec<_> = Vec::with_capacity(assets.len() * 2);
        for (ticker, has_credit) in &assets {
            asset_positions.push((
                PositionId::from(format!("NEXO:{ticker}")),
                AssetId::str(ticker),
                CSV_DECIMALS,
                true,
            ));
            if *has_credit {
                asset_positions.push((
                    PositionId::from(format!("NEXO-CREDIT:{ticker}")),
                    AssetId::str(ticker),
                    CSV_DECIMALS,
                    false,
                ));
            }
        }
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
        let mut assets: BTreeMap<String, bool> = BTreeMap::new();
        let mut transactions = Vec::with_capacity(rows.len());
        for row in &rows {
            transactions.push(convert_row(row, &mut assets).context(MapTransactionSnafu {
                transaction_id: row.tx_id.clone(),
            })?);
        }
        Ok(CollectTxnData {
            transactions,
            assets: assets
                .into_keys()
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
    for (ticker, has_credit) in &assets {
        let asset = AssetId::str(ticker);
        store
            .save_asset_scale(&asset, CSV_DECIMALS)
            .context(StoreSnafu)?;
        store
            .save_position_asset(&PositionId::from(format!("NEXO:{ticker}")), &asset, true)
            .context(StoreSnafu)?;
        if *has_credit {
            store
                .save_position_asset(
                    &PositionId::from(format!("NEXO-CREDIT:{ticker}")),
                    &asset,
                    false,
                )
                .context(StoreSnafu)?;
        }
    }
    store
        .save_transactions_with_progress(&transactions, on_progress)
        .context(StoreSnafu)?;
    Ok(rows.len())
}

fn map_rows(
    rows: &[NexoTx],
) -> Result<(BTreeMap<String, bool>, Vec<(String, Transaction)>), NexoError> {
    // Ticker -> whether any row touched the credit-line wallet for it.
    let mut assets: BTreeMap<String, bool> = BTreeMap::new();
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

/// Rows that move funds within the credit-line (loan) domain rather than
/// owned savings. Card spending is funded by the credit line and converted
/// immediately to xUSD debt (1:1): each "Nexo Card Loan Withdrawal" appears
/// as up to four rows for the same money (collateral/loan-fx leg, drawdown
/// record, merchant spend, merchant-fx leg), so only the drawdown counts.
fn is_credit_domain(row: &NexoTx) -> bool {
    use TransactionType::*;
    match row.kind {
        ExchangeCredit | CreditCardWithdrawalCredit | NexoCardPurchase | NexoCardTransactionFee => {
            true
        }
        Interest | FixedTermInterest | Cashback | ExchangeCashback | ReferralBonus | Dividend
        | TopUpCrypto | TransferFromProWallet => row.input_amount < 0.0,
        _ => false,
    }
}

/// xUSD debt accrued by one loan-domain row (drawdowns, card fees and loan
/// interest all convert 1:1 to xUSD debt). Duplicate draw records, merchant
/// spends of already-drawn cash and off-ledger USD annotations accrue 0.
fn xusd_debt_accrued(row: &NexoTx) -> f64 {
    use TransactionType::*;
    let fiatx = ["USDX", "xUSD"];
    match row.kind {
        // The drawdown: collateral currency (USDX/xUSD) becomes xUSD debt.
        // GBPX legs are merchant-fx on already-drawn loan: no new debt.
        ExchangeCredit if fiatx.contains(&row.input_currency.as_str()) => {
            row.input_amount.abs()
        }
        // Card fees are charged to the credit line.
        NexoCardTransactionFee if fiatx.contains(&row.input_currency.as_str()) => {
            row.input_amount.abs()
        }
        // Loan interest accrues as debt (negative inputs, fiatx only).
        Interest | FixedTermInterest | Cashback | ExchangeCashback | ReferralBonus | Dividend
        | TopUpCrypto | TransferFromProWallet => {
            if row.input_amount < 0.0 {
                row.output_amount
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn convert_row(
    row: &NexoTx,
    assets: &mut BTreeMap<String, bool>,
) -> Result<Transaction, NexoMapError> {
    let credit = is_credit_domain(row);
    let mut changes: Vec<(&str, f64)> = Vec::new();
    use TransactionType::*;
    if credit {
        // Credit-line domain: only the xUSD debt accrual counts. Duplicate
        // draw records, spends of drawn cash and merchant-fx legs accrue 0.
        let debt = xusd_debt_accrued(row);
        if debt != 0.0 {
            changes.push(("xUSD", debt));
        }
    } else {
        match row.kind {
            Interest | FixedTermInterest | Cashback | ExchangeCashback | ReferralBonus | Dividend
            | TopUpCrypto | TransferFromProWallet => {
                // Negative inputs are loan-domain (see is_credit_domain) and
                // never reach this savings branch.
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
            // Void/legacy rows record 0 output for a same-value conversion
            // (e.g. 2023 fiat top-ups); credit 1:1 instead of nothing.
            let out = if row.output_amount == 0.0 {
                row.input_amount.abs()
            } else {
                row.output_amount
            };
            changes.push((&row.output_currency, out));
        }
        Withdrawal
        | WithdrawExchanged
        | NexoCardPurchase
        | NexoCardTransactionFee
        | CreditCardWithdrawalCredit
        | TransferToProWallet => {
            changes.push((&row.input_currency, -row.input_amount.abs()));
        }
        // Savings <-> term/credit-wallet moves remain owned by the user. The
        // export omits wallet IDs, so these have no effect on the aggregate
        // asset amount (verified: subtracting them breaks ETH/POL/USDT).
        LockTermDeposit
        | UnlockTermDeposit
        | TransferOut => {}
        }
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
        assets
            .entry(ticker.to_string())
            .and_modify(|has_credit| *has_credit |= credit)
            .or_insert(credit);
        let position = if credit {
            PositionId::from(format!("NEXO-CREDIT:{ticker}"))
        } else {
            PositionId::from(format!("NEXO:{ticker}"))
        };
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
    fn nexo_balances_sum_from_transactions_without_prices() -> anyhow::Result<()> {
        let rows = [
            "id1,Top up Crypto,ETH,2,ETH,2,$200,-,-,approved / deposit,2025-01-01 00:00:00",
            "id2,Top up Crypto,NEXO,100,NEXO,100,$50,-,-,approved / deposit,2025-01-02 00:00:00",
            "id3,Interest,NEXO,1,NEXO,1,$1,-,-,approved / interest,2025-01-03 00:00:00",
            "id4,Exchange,ETH,-0.5,NEXO,50,$100,-,-,approved / exchange,2025-01-04 00:00:00",
            "id5,Withdrawal,NEXO,-10,NEXO,0,$0,-,-,approved / withdraw,2025-01-05 00:00:00",
        ]
        .into_iter()
        .map(NexoTx::try_from_csv_row)
        .collect::<Result<Vec<_>, _>>()?;
        let store = Store::in_memory()?;
        import_nexo_rows(&store, &rows)?;
        let end = DateTime::<Utc>::from_timestamp(1736121600, 0).unwrap(); // 2025-01-06
        let mut amounts: BTreeMap<String, f64> = store
            .asset_amounts_at(end)?
            .into_iter()
            .map(|(asset, n)| (asset.0, n))
            .collect();
        // ETH: 2 - 0.5 = 1.5; NEXO: 100 + 1 + 50 - 10 = 141
        assert!((amounts.remove("ETH").unwrap_or(0.0) - 1.5).abs() < 1e-8);
        assert!((amounts.remove("NEXO").unwrap_or(0.0) - 141.0).abs() < 1e-8);
        assert!(amounts.is_empty(), "unexpected assets: {amounts:?}");
        Ok(())
    }

    #[test]
    fn file_import_then_latest_balances_match_csv_movements() -> anyhow::Result<()> {
        // End-to-end through the production path: hash-deduped file import
        // (`import_into`, the same call `main()` makes) followed by the same
        // DB balance query used for latest holdings (`asset_amounts_at`).
        // No prices involved — pure transaction sums.
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("nexo.csv");
        std::fs::write(
            &path,
            "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\n\
             id1,Top up Crypto,ETH,2,ETH,2,$200,-,-,approved / deposit,2025-01-01 00:00:00\n\
             id2,Top up Crypto,NEXO,100,NEXO,100,$50,-,-,approved / deposit,2025-01-02 00:00:00\n\
             id3,Interest,NEXO,1,NEXO,1,$1,-,-,approved / interest,2025-01-03 00:00:00\n\
             id4,Exchange,ETH,0.5,NEXO,50,$100,-,-,approved / exchange,2025-01-04 00:00:00\n\
             id5,Withdrawal,NEXO,10,NEXO,0,$0,-,-,approved / withdraw,2025-01-05 00:00:00\n",
        )?;
        let store = Store::in_memory()?;
        let service = NexoSvc::from_path(&path);
        assert_eq!(service.import_into(&store)?, NexoImportOutcome::Imported(5));
        // Re-importing the unchanged file must be a deduped no-op.
        assert_eq!(
            service.import_into(&store)?,
            NexoImportOutcome::AlreadyImported
        );

        let rows = service.read_rows()?;
        assert_eq!(rows.len(), 5);
        let end = rows.iter().map(|r| r.date_time_utc).max().unwrap();
        let mut amounts: BTreeMap<String, f64> = store
            .asset_amounts_at(end)?
            .into_iter()
            .map(|(asset, n)| (asset.0, n))
            .collect();
        // ETH: 2 - 0.5 = 1.5; NEXO: 100 + 1 + 50 - 10 = 141
        assert!((amounts.remove("ETH").unwrap_or(0.0) - 1.5).abs() < 1e-8);
        assert!((amounts.remove("NEXO").unwrap_or(0.0) - 141.0).abs() < 1e-8);
        assert!(amounts.is_empty(), "unexpected assets: {amounts:?}");
        Ok(())
    }

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
    #[ignore = "needs real export at .cache/nexo_transactions_05-10-2026_10-36-53.csv (not committed)"]
    fn latest_amount_after_cached_export() -> anyhow::Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache/nexo_transactions_05-10-2026_10-36-53.csv");
        let service = NexoSvc::from_path(&path);
        let rows = service.read_rows()?;
        // File-backed scratch DB next to the export: the hash-deduped import
        // below is a no-op on repeat runs, so the test skips re-inserting
        // thousands of transactions every time.
        let db_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache/test-nexo_05-10-2026.duckdb");
        let store = Store::open(&db_path)?;
        match service.import_into_with_progress(&store, |done, total| {
            eprintln!("Writing Nexo transactions: {done}/{total}");
        })? {
            NexoImportOutcome::Imported(n) => eprintln!("Imported {n} Nexo transactions"),
            NexoImportOutcome::AlreadyImported => {
                eprintln!("Nexo CSV already imported; reused {}", db_path.display())
            }
        }
        let end = rows.iter().map(|r| r.date_time_utc).max().unwrap();
        let amounts = store.asset_amounts_at(end)?;
        println!("latest balances at {end} ({} rows):", rows.len());
        for (asset, amount) in &amounts {
            println!("  {}: {amount}", asset.0);
        }
        println!("exact owned units (1e-8):");
        for (asset, units) in store.asset_units_at(end)? {
            println!("  {}: {units}", asset.0);
        }
        println!("exact credit units (1e-8):");
        for (asset, units) in store.credit_units_at(end)? {
            println!("  {}: {units}", asset.0);
        }
        let latest = |ticker: &str| {
            amounts
                .iter()
                .find(|(asset, _)| asset.0 == ticker)
                .map(|(_, n)| *n)
                .unwrap_or(0.0)
        };
        // Expected dashboard totals for this export (balances only; price/
        // credit-line fields are informational and not asserted here).
        let expected = [
            ("ETH", 214.10824555),
            ("NEXO", 271873.05097308),
            ("EURX", 83776.0),
            ("NEAR", 9540.24127999),
            ("BNB", 37.37181767),
            ("GBPX", 17763.0),
            ("DOT", 2247.74400873),
            ("POL", 9230.724),
            ("USDT", 4.521777),
            ("USDX", 1.56),
        ];
        for (ticker, balance) in expected {
            // 1e-4 tolerance: f64 accumulation dust over ~9k rows
            // (e.g. ~3.5e-5 on NEXO).
            assert!(
                (latest(ticker) - balance).abs() < 1e-4,
                "{ticker}: expected {balance}, got {}",
                latest(ticker)
            );
        }
        // Exact integer-unit expectations (1e-8). Integer sums prove what
        // is dust and what is real: USDC 45 units is float-division noise,
        // while GBP 7219002 units (£0.07219002) is a genuine remainder from
        // the 2023-11-10 convert-and-withdraw (GBP 990.71 -> £990.78219002,
        // withdrew £990.71). No arithmetic makes it zero.
        let owned_units: BTreeMap<String, i128> = store
            .asset_units_at(end)?
            .into_iter()
            .map(|(asset, n)| (asset.0, n))
            .collect();
        let expected_units: BTreeMap<String, i128> = [
            ("BNB", 3737181767),
            ("BTC", 0),
            ("DOT", 224774400873),
            ("ETH", 21410824548),
            ("ETHW", 0),
            ("EUR", 0),
            ("EURX", 8377600000000),
            ("GBP", 7219002),
            ("GBPX", 1776300000000),
            ("NEAR", 954024127999),
            ("NEXO", 27187305093806),
            ("POL", 923072400000),
            ("USDC", 45),
            ("USDT", 452178144),
            ("USDX", 156000000),
        ]
        .into_iter()
        .map(|(ticker, units)| (ticker.to_string(), units))
        .collect();
        assert_eq!(owned_units, expected_units);
        // Credit line: every card spend was converted immediately to xUSD
        // debt (1:1). Derived independently from the rows: drawdowns
        // (USDX/xUSD loan inputs) + card fees = principal 1357.92,
        // loan-interest outputs = 83.64, total draft 1441.56.
        let (mut principal, mut loan_interest) = (0.0, 0.0);
        for row in &rows {
            use TransactionType::*;
            let fiatx = ["USDX", "xUSD"];
            match row.kind {
                ExchangeCredit if fiatx.contains(&row.input_currency.as_str()) => {
                    principal += row.input_amount.abs();
                }
                NexoCardTransactionFee if fiatx.contains(&row.input_currency.as_str()) => {
                    principal += row.input_amount.abs();
                }
                Interest | FixedTermInterest | Cashback | ExchangeCashback | ReferralBonus
                | Dividend | TopUpCrypto | TransferFromProWallet
                    if row.input_amount < 0.0 =>
                {
                    loan_interest += row.output_amount;
                }
                _ => {}
            }
        }
        assert!((principal - 1357.92).abs() < 1e-6, "principal {principal}");
        assert!((loan_interest - 83.64).abs() < 1e-6, "interest {loan_interest}");
        let credit_units: BTreeMap<String, i128> = store
            .credit_units_at(end)?
            .into_iter()
            .map(|(asset, n)| (asset.0, n))
            .collect();
        let expected_credit: BTreeMap<String, i128> =
            [("xUSD", 144156000000)]
                .into_iter()
                .map(|(ticker, units)| (ticker.to_string(), units))
                .collect();
        assert_eq!(credit_units, expected_credit);
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
