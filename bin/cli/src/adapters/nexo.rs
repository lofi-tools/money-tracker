use lib_core::traits::IsProvider;
use lib_core::{AccountId, AssetId, Position, ProviderId, Transaction, TxEffect};
use nexo_csv::{NexoCsv, NexoTx};

pub struct NexoSvc {
    // pub transactions_csv: NexoCsv,
}
impl NexoSvc {
    pub fn new() -> anyhow::Result<Self> {
        Ok(NexoSvc {
            // transactions_csv: NexoCsv {},
        })
    }

    pub fn fetch_transactions(&self) -> anyhow::Result<Vec<()>> {
        let mut transactions = NexoCsv::read_all()?;
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
        const PROVIDER_ID_NEXO: &str = "nexo";
        ProviderId::from(PROVIDER_ID_NEXO)
    }

    async fn fetch_positions(&self) -> anyhow::Result<Vec<Position>> {
        todo!()
    }

    async fn fetch_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        let mut nexo_transactions = NexoCsv::read_all()?;
        nexo_transactions.sort_by(|a, b| a.date_time_utc.cmp(&b.date_time_utc));

        let transactions = nexo_transactions
            .into_iter()
            .map(|ntx| transaction_from_nexo_tx(ntx))
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
            datetime: todo!(),
        }],
        nexo_csv::TransactionType::TopUpCrypto => vec![TxEffect {
            // asset: asset_id_from_nexo(&nexo_tx.output_currency),
            amount: to_u64(
                nexo_tx.output_amount,
                get_decimals(&nexo_tx.output_currency),
            ),
            account_id: todo!(),
            datetime: todo!(),
        }],
        nexo_csv::TransactionType::TermInterest => vec![TxEffect {
            // asset: asset_id_from_nexo(&nexo_tx.output_currency),
            amount: to_u64(
                nexo_tx.output_amount,
                get_decimals(&nexo_tx.output_currency),
            ),
            account_id: todo!(),
            datetime: todo!(),
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
