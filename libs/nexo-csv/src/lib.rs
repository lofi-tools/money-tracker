use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum NexoCsvError {
    #[snafu(display("cannot open Nexo CSV {}", path.display()))]
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("cannot read Nexo CSV headers"))]
    Headers { source: csv::Error },
    #[snafu(display("cannot read Nexo CSV row {row}"))]
    Row { row: usize, source: csv::Error },
    #[snafu(display("cannot parse Nexo CSV row {row}"))]
    ParseRow { row: usize, source: csv::Error },
    #[snafu(display("Nexo CSV row is missing"))]
    MissingRow,
}

pub struct NexoCsv {}

/// Exact bytes of a Nexo export and their SHA-256 hash. The bytes are parsed
/// only after the caller checks whether the hash was imported already.
pub struct NexoCsvFile {
    bytes: Vec<u8>,
    pub sha256_hex: String,
}

impl NexoCsvFile {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, NexoCsvError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).context(OpenSnafu {
            path: path.to_path_buf(),
        })?;
        let sha256_hex = sha256_hex(&bytes);
        Ok(Self { bytes, sha256_hex })
    }

    pub fn parse(&self) -> Result<Vec<NexoTx>, NexoCsvError> {
        NexoCsv::from_reader(self.bytes.as_slice())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl NexoCsv {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Vec<NexoTx>, NexoCsvError> {
        let path = path.as_ref();
        let file_reader = std::fs::File::open(path).context(OpenSnafu {
            path: path.to_path_buf(),
        })?;
        Self::from_reader(file_reader)
    }
    pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Vec<NexoTx>, NexoCsvError> {
        let mut rdr = csv::Reader::from_reader(reader);

        let headers = rdr.headers().context(HeadersSnafu)?.clone();

        let nexo_txs = rdr
            .records()
            .enumerate()
            .map(|(index, row)| {
                let row = row.context(RowSnafu { row: index + 2 })?;
                let record: NexoTx = row
                    .deserialize(Some(&headers))
                    .context(ParseRowSnafu { row: index + 2 })?;
                Ok(record)
            })
            .collect::<Result<Vec<NexoTx>, NexoCsvError>>()?;

        Ok(nexo_txs)
    }
}

#[derive(Debug, Deserialize)]
pub struct NexoTx {
    #[serde(rename = "Transaction")]
    pub tx_id: String,
    #[serde(rename = "Type")]
    pub kind: TransactionType,
    #[serde(rename = "Input Currency")]
    pub input_currency: String,
    #[serde(rename = "Input Amount")]
    pub input_amount: f64, // TODO use decimal
    #[serde(rename = "Output Currency")]
    pub output_currency: String,
    #[serde(rename = "Output Amount")]
    pub output_amount: f64, // TODO use decimal
    #[serde(rename = "USD Equivalent")]
    pub usd_equivalent: String,
    #[serde(rename = "Details")]
    pub details: String,
    #[serde(
        rename = "Date / Time (UTC)",
        deserialize_with = "utils::time_utils::de_datetime"
    )]
    pub date_time_utc: DateTime<Utc>,
}
impl NexoTx {
    pub fn try_from_csv_row(csv_row: &str) -> Result<Self, NexoCsvError> {
        const HEADERS: &str = "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)";
        let with_headers = format!("{}\n{}", HEADERS, csv_row);
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true) // We are providing the headers explicitly
            .from_reader(with_headers.as_bytes());

        let record: NexoTx = rdr
            .deserialize()
            .next()
            .ok_or(NexoCsvError::MissingRow)?
            .context(ParseRowSnafu { row: 2usize })?;

        Ok(record)
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum TransactionType {
    Interest,
    #[serde(rename = "Locking Term Deposit")]
    LockTermDeposit,
    #[serde(rename = "Unlocking Term Deposit")]
    UnlockTermDeposit,
    #[serde(rename = "Fixed Term Interest")]
    FixedTermInterest,
    #[serde(rename = "Transfer From Pro Wallet")]
    TransferFromProWallet,
    #[serde(rename = "Transfer To Pro Wallet")]
    TransferToProWallet,
    #[serde(rename = "Exchange Deposited On")]
    ExchangeDepositedOn,
    #[serde(rename = "Deposit To Exchange")]
    DepositToExchange,
    #[serde(rename = "Withdraw Exchanged")]
    WithdrawExchanged,
    #[serde(rename = "Exchange To Withdraw")]
    ExchangeToWithdraw,
    #[serde(rename = "Top up Crypto")]
    TopUpCrypto,
    #[serde(rename = "Cashback")]
    Cashback,
    #[serde(rename = "Exchange Credit")]
    ExchangeCredit,
    #[serde(rename = "Nexo Card Transaction Fee")]
    NexoCardTransactionFee,
    #[serde(rename = "Credit Card Withdrawal Credit")]
    CreditCardWithdrawalCredit,
    #[serde(rename = "Transfer Out")]
    TransferOut,
    #[serde(rename = "Nexo Card Purchase")]
    NexoCardPurchase,
    #[serde(rename = "Credit Card Fiatx Exchange To Withdraw")]
    CreditCardFiatExchangeToWithdraw,
    #[serde(rename = "Exchange")]
    Exchange,
    #[serde(rename = "Exchange Cashback")]
    ExchangeCashback,
    #[serde(rename = "Withdrawal")]
    Withdrawal,
    #[serde(rename = "Referral Bonus")]
    ReferralBonus,
    #[serde(rename = "Dividend")]
    Dividend,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::{path::PathBuf, sync::LazyLock};

    static NEXO_CSV_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut path = std::env::current_dir().unwrap();
        path.push("../../.cache/nexo_transactions.csv");
        path
    });

    #[test]
    fn hashes_exact_bytes_with_sha256() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_ne!(sha256_hex(b"abc"), sha256_hex(b"abc\n"));
    }

    #[test]
    #[ignore = "Needs real data in .cache"]
    fn test_read_file() -> anyhow::Result<()> {
        let nexo_txs = NexoCsv::from_file(&*NEXO_CSV_PATH)?;
        println!("{}", nexo_txs.len());
        assert!(nexo_txs.len() > 3);
        Ok(())
    }

    #[test]
    fn test_read() -> anyhow::Result<()> {
        const EXAMPLE_CSV: LazyLock<String> = LazyLock::new(|| {
            utils::string_utils::unindent(
                r#"
            Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)
            NXT7YGjfSsQYl1XIWtcxdUcjx,Interest,NEXO,7.26938619,NEXO,7.26938619,$7.31,-,-,"approved / NEXO Interest Earned",2025-11-27 06:00:00
            NXT19G9w73JFFbODT59x1NJzh,Fixed Term Interest,NEXO,0.90361422,NEXO,0.90361422,$1.00,-,-,"approved / POL Term Deposit Interest in NEXO",2025-11-11 06:00:00
        "#,
            )
        });

        let nexo_txs = NexoCsv::from_reader(std::io::Cursor::new(&*EXAMPLE_CSV))?;

        assert_eq!(nexo_txs.len(), 2);
        assert_eq!(nexo_txs[0].tx_id, "NXT7YGjfSsQYl1XIWtcxdUcjx");
        assert_eq!(nexo_txs[1].tx_id, "NXT19G9w73JFFbODT59x1NJzh");
        assert_eq!(nexo_txs[0].kind, TransactionType::Interest);
        assert_eq!(nexo_txs[1].kind, TransactionType::FixedTermInterest);
        Ok(())
    }

    #[test]
    fn test_try_from_csv_row() -> anyhow::Result<()> {
        let nexo_tx = NexoTx::try_from_csv_row(
            r#"NXT7YGjfSsQYl1XIWtcxdUcjx,Interest,NEXO,7.26938619,NEXO,7.26938619,$7.31,-,-,"approved / NEXO Interest Earned",2025-11-27 06:00:00"#,
        )?;
        assert_eq!(nexo_tx.tx_id, "NXT7YGjfSsQYl1XIWtcxdUcjx");
        assert_eq!(nexo_tx.kind, TransactionType::Interest);
        assert_eq!(nexo_tx.input_currency, "NEXO");
        assert_eq!(nexo_tx.input_amount, 7.26938619);
        assert_eq!(nexo_tx.output_currency, "NEXO");
        assert_eq!(nexo_tx.output_amount, 7.26938619);
        assert_eq!(nexo_tx.usd_equivalent, "$7.31");
        assert_eq!(nexo_tx.details, "approved / NEXO Interest Earned");
        assert_eq!(
            nexo_tx.date_time_utc,
            DateTime::from_timestamp(1764223200, 0).unwrap(),
        );

        Ok(())
    }

    #[test]
    fn reports_bad_row_without_losing_its_line_number() {
        let error = NexoTx::try_from_csv_row(
            "id,Interest,ETH,1,ETH,1,$1,-,-,approved / interest,not-a-date",
        )
        .unwrap_err();
        assert!(matches!(error, NexoCsvError::ParseRow { row: 2, .. }));
    }

    #[test]
    fn reports_missing_file_with_its_path() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing-nexo-test.csv");
        let error = NexoCsv::from_file(&path).unwrap_err();
        assert!(matches!(error, NexoCsvError::Open { path: p, .. } if p == path));
    }
}
