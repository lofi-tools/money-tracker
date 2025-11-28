use chrono::{DateTime, Utc};
use serde::Deserialize;

pub struct NexoCsv {}
impl NexoCsv {
    pub fn read_all(path: &str) -> anyhow::Result<Vec<NexoTx>> {
        let mut rdr = csv::Reader::from_path(path)?;

        let headers = rdr.headers()?.clone();

        let nexo_txs = rdr
            .records()
            .take(5)
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
    #[serde(rename = "Date / Time (UTC)", deserialize_with = "utils::de_datetime")]
    pub date_time_utc: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub enum TransactionType {
    Interest,
    #[serde(rename = "Locking Term Deposit")]
    LockTermDeposit,
    #[serde(rename = "Unlocking Term Deposit")]
    UnlockTermDeposit,
    #[serde(rename = "Fixed Term Interest")]
    TermInterest,
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
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_read() -> anyhow::Result<()> {
        let path = "../../.cache/nexo_transactions.csv";
        let nexo_txs = NexoCsv::read_all(path)?;
        dbg!(nexo_txs);

        Ok(())
    }
}

pub mod utils {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{Deserialize, Deserializer, de};

    const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn de_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let naive_dt = NaiveDateTime::parse_from_str(&s, DATE_FORMAT).map_err(de::Error::custom)?;
        let dt = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);
        Ok(dt.to_utc())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde::de::value::StringDeserializer;

        #[test]
        fn test_serde_datetime() -> anyhow::Result<()> {
            let dt = Utc::now();
            let _dt_str = dt.format(DATE_FORMAT).to_string();

            let s = "2024-04-19 05:00:00";
            let _dt = de_datetime(StringDeserializer::<csv::DeserializeError>::new(
                s.to_string(),
            ))?;

            Ok(())
        }
    }
}
