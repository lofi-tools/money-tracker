use crate::cli::Config;
use lib_core::traits::IsProvider;
use lib_core::{AccountId, AssetId, Position, ProviderId, Transaction, TxEffect};
use nexo_csv::{NexoCsv, NexoTx};
use std::path::PathBuf;
use std::sync::LazyLock;

const PROVIDER_ID: LazyLock<ProviderId> = LazyLock::new(|| ProviderId("NEXO".to_string()));

pub struct NexoSvc {
    // pub transactions_csv: NexoCsv,
    path_to_csv: PathBuf,
}
impl NexoSvc {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        Ok(NexoSvc {
            // transactions_csv: NexoCsv {},
            path_to_csv: config.data_dir.join("nexo_transactions.csv"),
        })
    }

    pub fn fetch_transactions(&self) -> anyhow::Result<Vec<()>> {
        let mut transactions = NexoCsv::read_all(&self.path_to_csv)?;
        transactions.sort_by(|a, b| a.date_time_utc.cmp(&b.date_time_utc));

        dbg!(&transactions);

        // TODO create DataFrame with only difference in ETH and timestamp
        // TODO cumsum for total ETH

        Ok(Vec::new())
    }
}
#[async_trait::async_trait]
impl IsProvider for NexoSvc {
    fn provider_id(&self) -> ProviderId {
        PROVIDER_ID.clone()
    }

    async fn fetch_positions(&self) -> anyhow::Result<Vec<Position>> {
        todo!()
    }

    async fn fetch_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let mut nexo_transactions = NexoCsv::read_all("nexo_transactions.csv")?;
        nexo_transactions.sort_by(|a, b| a.date_time_utc.cmp(&b.date_time_utc));

        let transactions = nexo_transactions
            .into_iter()
            .map(|ntx| transaction_from_nexo_tx(ntx))
            .collect();

        Ok(transactions)
    }
}

impl NexoTx {
    /// If there's only one input account, this returns it
    pub fn input_account(&self) -> AccountId {
        match &self.kind {
            nexo_csv::TransactionType::Interest => todo!(),
            nexo_csv::TransactionType::LockTermDeposit => todo!(),
            nexo_csv::TransactionType::UnlockTermDeposit => todo!(),
            nexo_csv::TransactionType::TermInterest => todo!(),
            nexo_csv::TransactionType::TransferFromProWallet => todo!(),
            nexo_csv::TransactionType::TransferToProWallet => todo!(),
            nexo_csv::TransactionType::ExchangeDepositedOn => todo!(),
            nexo_csv::TransactionType::DepositToExchange => todo!(),
            nexo_csv::TransactionType::WithdrawExchanged => todo!(),
            nexo_csv::TransactionType::ExchangeToWithdraw => todo!(),
            nexo_csv::TransactionType::TopUpCrypto => todo!(),
        }
    }
    pub fn inputs(&self) -> Vec<TxEffect> {
        match &self.kind {
            nexo_csv::TransactionType::Interest => todo!(),
            nexo_csv::TransactionType::LockTermDeposit => todo!(),
            nexo_csv::TransactionType::UnlockTermDeposit => todo!(),
            nexo_csv::TransactionType::TermInterest => todo!(),
            nexo_csv::TransactionType::TransferFromProWallet => todo!(),
            nexo_csv::TransactionType::TransferToProWallet => todo!(),
            nexo_csv::TransactionType::ExchangeDepositedOn => todo!(),
            nexo_csv::TransactionType::DepositToExchange => todo!(),
            nexo_csv::TransactionType::WithdrawExchanged => todo!(),
            nexo_csv::TransactionType::ExchangeToWithdraw => todo!(),
            nexo_csv::TransactionType::TopUpCrypto => vec![TxEffect {
                // asset: AssetId::from("ETH"),
                amount: to_u64(self.output_amount, get_decimals(&self.output_currency)),
                account_id: NexoSvc::mk_account_id(&format!("NEXO_TOPUP_{}", self.output_currency)),
                // asset: AssetId::from(&self.output_currency),
                // direction: TxDirection::Credit,
                // tx_type: TxType::Deposit,
                datetime: self.date_time_utc,
            }],
        }
    }
}

fn get_decimals(asset: &str) -> u8 {
    match asset {
        "ETH" => 18,
        "BTC" => 8,
        "USDT" => 6,
        _ => 18, // Default
    }
}

fn to_u64(amount: f64, decimals: u8) -> u64 {
    (amount * 10f64.powi(decimals as i32)).round() as u64
}

/// Convert a Nexo transaction to a Transaction
fn transaction_from_nexo_tx(nexo_tx: NexoTx) -> Transaction {
    let inputs = match &nexo_tx.kind {
        // TODO make transactionInputOutput use ProductId not assetId
        // TODO figure out content by logging
        nexo_csv::TransactionType::ExchangeDepositedOn => vec![],
        nexo_csv::TransactionType::DepositToExchange => vec![],
        nexo_csv::TransactionType::WithdrawExchanged => vec![],
        nexo_csv::TransactionType::ExchangeToWithdraw => vec![],
        // should have no impact on total -> no inputs/outputs // TODO same inputs / outputs
        nexo_csv::TransactionType::TransferFromProWallet => vec![],
        nexo_csv::TransactionType::TransferToProWallet => vec![],
        nexo_csv::TransactionType::LockTermDeposit => vec![], // TODO some(move between products)
        nexo_csv::TransactionType::UnlockTermDeposit => vec![], // TODO Some(move between products)
        // TODO inputs from other people / outside of Nexo
        nexo_csv::TransactionType::TopUpCrypto => vec![],
        nexo_csv::TransactionType::TermInterest => vec![],
        nexo_csv::TransactionType::Interest => vec![],
    };
    let outputs = match &nexo_tx.kind {
        nexo_csv::TransactionType::Interest => vec![TxEffect {
            // asset: asset_id_from_nexo(&nexo_tx.output_currency),
            amount: to_u64(
                nexo_tx.output_amount,
                get_decimals(&nexo_tx.output_currency),
            ),
            account_id: todo!(),
            datetime: nexo_tx.date_time_utc,
        }],
        nexo_csv::TransactionType::TopUpCrypto => vec![TxEffect {
            // asset: asset_id_from_nexo(&nexo_tx.output_currency),
            amount: to_u64(
                nexo_tx.output_amount,
                get_decimals(&nexo_tx.output_currency),
            ),
            account_id: todo!(),
            datetime: nexo_tx.date_time_utc,
        }],
        nexo_csv::TransactionType::TermInterest => vec![TxEffect {
            // asset: asset_id_from_nexo(&nexo_tx.output_currency),
            amount: to_u64(
                nexo_tx.output_amount,
                get_decimals(&nexo_tx.output_currency),
            ),
            account_id: todo!(),
            datetime: nexo_tx.date_time_utc,
        }],

        // TODO figure out content by logging
        nexo_csv::TransactionType::ExchangeDepositedOn => vec![],
        nexo_csv::TransactionType::DepositToExchange => vec![],
        nexo_csv::TransactionType::WithdrawExchanged => vec![],
        nexo_csv::TransactionType::ExchangeToWithdraw => vec![],
        // should have no impact on total -> no inputs/outputs // TODO same inputs / outputs
        nexo_csv::TransactionType::TransferFromProWallet => vec![],
        nexo_csv::TransactionType::TransferToProWallet => vec![],
        nexo_csv::TransactionType::LockTermDeposit => vec![], // TODO some(move between products)
        nexo_csv::TransactionType::UnlockTermDeposit => vec![], // TODO Some(move between products)
    };

    Transaction {
        // id: TransactionId::from(""), // TODO
        datetime: nexo_tx.date_time_utc,
        inputs,
        outputs,
    }
}

/// Convert a Nexo asset identifier to an AssetId
fn asset_id_from_nexo(nexo_asset: &str) -> AssetId {
    match nexo_asset {
        "ETH" => AssetId::Eth,
        _ => AssetId::unknown(nexo_asset),
    }
}

#[cfg(test)]
pub mod tests {
    use chrono::Utc;
    use nexo_csv::TransactionType;

    use super::*;

    #[test]
    fn test_transaction_from_nexo_tx() -> anyhow::Result<()> {
        let nexo_tx = NexoTx {
            tx_id: "1".to_string(),
            kind: TransactionType::Interest,
            input_currency: "ETH".to_string(),
            input_amount: 1.0,
            output_currency: "ETH".to_string(),
            output_amount: 1.0,
            usd_equivalent: "1.0".to_string(),
            details: "Interest".to_string(),
            date_time_utc: Utc::now(),
        };
        let _transaction = transaction_from_nexo_tx(nexo_tx);
        Ok(())
    }
}
