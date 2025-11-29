use crate::cli::Config;
use lib_core::traits::IsProvider;
use lib_core::{
    AccountId, Asset, AssetId, Position, PositionId, Product, ProductId, ProviderId, Transaction,
    TxEffect,
};
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
            .iter()
            .map(|ntx| {
                let data = process_txn(ntx);
                dbg!(&data);
                data.transaction
            })
            .collect();

        Ok(transactions)
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

#[derive(Debug)]
pub struct TxData {
    pub assets: Vec<Asset>,
    pub products: Vec<Product>,
    pub positions: Vec<Position>,
    pub transaction: Transaction,
}

fn process_txn(tx: &NexoTx) -> TxData {
    match tx.kind {
        nexo_csv::TransactionType::Interest => process_interest(tx),
        nexo_csv::TransactionType::LockTermDeposit => process_lock_term_deposit(tx),
        nexo_csv::TransactionType::UnlockTermDeposit => process_unlock_term_deposit(tx),
        nexo_csv::TransactionType::TermInterest => process_term_interest(tx),
        nexo_csv::TransactionType::TransferFromProWallet => process_transfer_from_pro_wallet(tx),
        nexo_csv::TransactionType::TransferToProWallet => process_transfer_to_pro_wallet(tx),
        nexo_csv::TransactionType::ExchangeDepositedOn => process_exchange_deposited_on(tx),
        nexo_csv::TransactionType::DepositToExchange => process_deposit_to_exchange(tx),
        nexo_csv::TransactionType::WithdrawExchanged => process_withdraw_exchanged(tx),
        nexo_csv::TransactionType::ExchangeToWithdraw => process_exchange_to_withdraw(tx),
        nexo_csv::TransactionType::TopUpCrypto => process_top_up_crypto(tx),
    }
}

fn process_interest(tx: &NexoTx) -> TxData {
    let asset_id = asset_id_from_nexo(&tx.output_currency);
    let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    let effect = TxEffect {
        account_id,
        amount,
        datetime: tx.date_time_utc,
    };

    let asset = Asset {
        id: asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.output_currency),
        external_ids: Default::default(),
    };

    TxData {
        assets: vec![asset],
        products: vec![],
        positions: vec![],
        transaction: Transaction {
            inputs: vec![],
            outputs: vec![effect],
            datetime: tx.date_time_utc,
        },
    }
}

fn process_lock_term_deposit(tx: &NexoTx) -> TxData {
    let asset_id = asset_id_from_nexo(&tx.input_currency);
    let amount = to_u64(tx.input_amount, get_decimals(&tx.input_currency));
    let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    // Debit Savings
    let input_effect = TxEffect {
        account_id,
        amount,
        datetime: tx.date_time_utc,
    };

    // Create Position
    let position = Position {
        id: PositionId::from(tx.tx_id.clone()),
        product_id: ProductId::from("NEXO_TERM_DEPOSIT"),
        amount,
        start_date: tx.date_time_utc,
        end_date: tx.date_time_utc, // TODO: find end date
    };

    let asset = Asset {
        id: asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.input_currency),
        external_ids: Default::default(),
    };

    TxData {
        assets: vec![asset],
        products: vec![],
        positions: vec![position],
        transaction: Transaction {
            inputs: vec![input_effect],
            outputs: vec![], // TODO: represent flow to position?
            datetime: tx.date_time_utc,
        },
    }
}

fn process_unlock_term_deposit(tx: &NexoTx) -> TxData {
    let asset_id = asset_id_from_nexo(&tx.output_currency);
    let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    // Credit Savings
    let output_effect = TxEffect {
        account_id,
        amount,
        datetime: tx.date_time_utc,
    };

    let asset = Asset {
        id: asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.output_currency),
        external_ids: Default::default(),
    };

    TxData {
        assets: vec![asset],
        products: vec![],
        positions: vec![], // TODO: Close position?
        transaction: Transaction {
            inputs: vec![], // TODO: from position?
            outputs: vec![output_effect],
            datetime: tx.date_time_utc,
        },
    }
}

fn process_term_interest(tx: &NexoTx) -> TxData {
    process_interest(tx)
}

fn process_transfer_from_pro_wallet(tx: &NexoTx) -> TxData {
    TxData {
        assets: vec![],
        products: vec![],
        positions: vec![],
        transaction: Transaction {
            inputs: vec![],
            outputs: vec![],
            datetime: tx.date_time_utc,
        },
    }
}

fn process_transfer_to_pro_wallet(tx: &NexoTx) -> TxData {
    TxData {
        assets: vec![],
        products: vec![],
        positions: vec![],
        transaction: Transaction {
            inputs: vec![],
            outputs: vec![],
            datetime: tx.date_time_utc,
        },
    }
}

fn process_exchange_deposited_on(tx: &NexoTx) -> TxData {
    // Input side
    let in_asset_id = asset_id_from_nexo(&tx.input_currency);
    let in_amount = to_u64(tx.input_amount, get_decimals(&tx.input_currency));
    let in_account_id = AccountId::new(PROVIDER_ID.clone(), in_asset_id.clone());

    let in_effect = TxEffect {
        account_id: in_account_id,
        amount: in_amount,
        datetime: tx.date_time_utc,
    };

    let in_asset = Asset {
        id: in_asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.input_currency),
        external_ids: Default::default(),
    };

    // Output side
    let out_asset_id = asset_id_from_nexo(&tx.output_currency);
    let out_amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    let out_account_id = AccountId::new(PROVIDER_ID.clone(), out_asset_id.clone());

    let out_effect = TxEffect {
        account_id: out_account_id,
        amount: out_amount,
        datetime: tx.date_time_utc,
    };

    let out_asset = Asset {
        id: out_asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.output_currency),
        external_ids: Default::default(),
    };

    TxData {
        assets: vec![in_asset, out_asset],
        products: vec![],
        positions: vec![],
        transaction: Transaction {
            inputs: vec![in_effect],
            outputs: vec![out_effect],
            datetime: tx.date_time_utc,
        },
    }
}

fn process_deposit_to_exchange(tx: &NexoTx) -> TxData {
    // Treat as transfer? Or just ignore if it's internal bookkeeping?
    // "Deposit To Exchange" usually means moving from Savings to Pro/Exchange wallet.
    // Let's assume it's a transfer to Pro.
    // Input: Savings, Output: Pro?
    // If we track Pro as a separate provider or account, we should use that.
    // For now, let's treat it as a withdrawal from Savings (Input) and ignore destination (or maybe we should track it).

    let asset_id = asset_id_from_nexo(&tx.input_currency);
    let amount = to_u64(tx.input_amount, get_decimals(&tx.input_currency));
    let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    let input_effect = TxEffect {
        account_id,
        amount,
        datetime: tx.date_time_utc,
    };

    let asset = Asset {
        id: asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.input_currency),
        external_ids: Default::default(),
    };

    TxData {
        assets: vec![asset],
        products: vec![],
        positions: vec![],
        transaction: Transaction {
            inputs: vec![input_effect],
            outputs: vec![], // TODO: Destination?
            datetime: tx.date_time_utc,
        },
    }
}

fn process_withdraw_exchanged(tx: &NexoTx) -> TxData {
    // "Withdraw Exchanged" - likely moving back from Exchange to Savings?
    // Input: Exchange? Output: Savings.

    let asset_id = asset_id_from_nexo(&tx.output_currency);
    let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    let output_effect = TxEffect {
        account_id,
        amount,
        datetime: tx.date_time_utc,
    };

    let asset = Asset {
        id: asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.output_currency),
        external_ids: Default::default(),
    };

    TxData {
        assets: vec![asset],
        products: vec![],
        positions: vec![],
        transaction: Transaction {
            inputs: vec![], // From Exchange?
            outputs: vec![output_effect],
            datetime: tx.date_time_utc,
        },
    }
}

fn process_exchange_to_withdraw(tx: &NexoTx) -> TxData {
    // "Exchange To Withdraw" - maybe similar to Withdraw Exchanged?
    // Or maybe the trade itself?
    // Let's assume it's a trade.
    process_exchange_deposited_on(tx)
}

fn process_top_up_crypto(tx: &NexoTx) -> TxData {
    let asset_id = asset_id_from_nexo(&tx.output_currency);
    let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    let effect = TxEffect {
        account_id,
        amount,
        datetime: tx.date_time_utc,
    };

    let asset = Asset {
        id: asset_id.clone(),
        chain_id: "".to_string(),
        decimals: get_decimals(&tx.output_currency),
        external_ids: Default::default(),
    };

    TxData {
        assets: vec![asset],
        products: vec![],
        positions: vec![],
        transaction: Transaction {
            inputs: vec![],
            outputs: vec![effect],
            datetime: tx.date_time_utc,
        },
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
    fn test_process_txn() -> anyhow::Result<()> {
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
        let _data = process_txn(&nexo_tx);
        Ok(())
    }

    #[test]
    fn test_process_txn_top_up() -> anyhow::Result<()> {
        let nexo_tx = NexoTx {
            tx_id: "2".to_string(),
            kind: TransactionType::TopUpCrypto,
            input_currency: "".to_string(),
            input_amount: 0.0,
            output_currency: "BTC".to_string(),
            output_amount: 0.5,
            usd_equivalent: "20000.0".to_string(),
            details: "Top Up".to_string(),
            date_time_utc: Utc::now(),
        };
        let data = process_txn(&nexo_tx);
        assert_eq!(data.assets.len(), 1);
        assert_eq!(data.transaction.outputs.len(), 1);
        assert_eq!(data.transaction.outputs[0].amount, 50000000); // 0.5 BTC * 10^8
        Ok(())
    }

    #[test]
    fn test_process_txn_exchange() -> anyhow::Result<()> {
        let nexo_tx = NexoTx {
            tx_id: "3".to_string(),
            kind: TransactionType::ExchangeDepositedOn,
            input_currency: "USDT".to_string(),
            input_amount: 1000.0,
            output_currency: "ETH".to_string(),
            output_amount: 0.5,
            usd_equivalent: "1000.0".to_string(),
            details: "Exchange".to_string(),
            date_time_utc: Utc::now(),
        };
        let data = process_txn(&nexo_tx);
        assert_eq!(data.assets.len(), 2);
        assert_eq!(data.transaction.inputs.len(), 1);
        assert_eq!(data.transaction.outputs.len(), 1);
        assert_eq!(data.transaction.inputs[0].amount, 1000000000); // 1000 USDT * 10^6
        assert_eq!(data.transaction.outputs[0].amount, 500000000000000000); // 0.5 ETH * 10^18
        Ok(())
    }
}
