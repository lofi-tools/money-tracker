use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Deserialize;

pub struct NexoCsv {}
impl NexoCsv {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Vec<NexoTx>> {
        let file_reader = std::fs::File::open(path)?;
        let nexo_txs = Self::from_reader(file_reader)?;
        Ok(nexo_txs)
    }
    pub fn from_reader<R: std::io::Read>(reader: R) -> anyhow::Result<Vec<NexoTx>> {
        let mut rdr = csv::Reader::from_reader(reader);

        let headers = rdr.headers()?.clone();

        let nexo_txs = rdr
            .records()
            .map(|row| {
                let row = row?;
                let record: NexoTx = row
                    .deserialize(Some(&headers.to_owned()))
                    .map_err(|e| anyhow::Error::new(e).context(format!("{:#?}", row)))?;
                Ok(record)
            })
            .collect::<anyhow::Result<Vec<NexoTx>>>()?;

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
    pub fn try_from_csv_row(csv_row: &str) -> anyhow::Result<Self> {
        const HEADERS: &str = "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)";
        let with_headers = format!("{}\n{}", HEADERS, csv_row);
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true) // We are providing the headers explicitly
            .from_reader(with_headers.as_bytes());

        let record: NexoTx = rdr
            .deserialize()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No record found in CSV string"))?
            .map_err(|e| {
                anyhow::Error::new(e).context(format!("Failed to deserialize row: {}", csv_row))
            })?;

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
}
