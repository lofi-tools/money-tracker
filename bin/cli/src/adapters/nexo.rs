use crate::cli::Config;
use lib_core::traits::IsProvider;
use lib_core::{
    Asset, AssetId, CollectTxnData, CounterpartyPosition, PositionId, Product, ProductId,
    ProviderId, Transaction, TxEffect, UserPosition,
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

    async fn fetch_all_txn_data(&self) -> anyhow::Result<CollectTxnData> {
        let mut nexo_transactions = NexoCsv::read_all("nexo_transactions.csv")?;
        nexo_transactions.sort_by(|a, b| a.date_time_utc.cmp(&b.date_time_utc));

        let collect_txn_data =
            CollectTxnData::try_from_iter(nexo_transactions.iter().map(|ntx| process_txn(ntx)))?;

        Ok(collect_txn_data)
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

fn process_txn(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
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

fn process_interest(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    let asset_id = NexoAssets::try_from_str(&tx.output_currency)?.local_asset_id()?;
    let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));

    // let asset = Asset {
    //     id: asset_id.clone(),
    //     // chain_id: "multichain".to_string(),
    //     decimals: get_decimals(&tx.output_currency),
    // };

    // let product = Product {
    //     id: ProductId::from(format!("product:NEXO_REWARDS_{asset_id}")),
    //     asset_id,
    //     apy: todo!(),
    // };
    // let input_position = UserPosition {
    //     id: PositionId::from(format!("position:NEXO_REWARDS_{asset_id}")),
    //     product_id: product.id,
    //     // amount,
    //     start_date: None,
    //     end_date: None,
    //     // owner: PositionOwner::Provider,
    // };
    let input_position = CounterpartyPosition {
        id: PositionId::from(format!("counterparty_position:NEXO_REWARDS_{asset_id}")),
        asset_id: asset_id.clone(),
    };
    let input_effect = TxEffect {
        amount,
        datetime: tx.date_time_utc,
        position_id: input_position.id,
    };

    // let output_product = Product {
    //     id: ProductId::from(format!("product:NEXO_FLEXIBLE_{asset_id}")),
    //     asset_id,
    //     apy: todo!(),
    // };
    let output_product = NexoProducts::match_by_id(&format!("Nexo_{asset_id}_Flexible"))?;
    let output_position = UserPosition {
        id: PositionId::from(format!("position:NEXO_REWARDS_{asset_id}")),
        product_id: output_product.id()?,
        start_date: None,
        end_date: None,
    };
    let output_effect = TxEffect {
        amount,
        datetime: tx.date_time_utc,
        position_id: output_position.id.clone(),
    };

    let transaction = Transaction {
        inputs: vec![input_effect],
        outputs: vec![output_effect],
        datetime: tx.date_time_utc,
    };

    Ok(CollectTxnData {
        assets: vec![],   // assets are matched to hardcoded (unavailable via CSV / no API)
        products: vec![], // products are matched to hardcoded (unavailable via CSV / no API)
        positions: vec![output_position],
        transactions: vec![transaction],
    })
}

fn process_lock_term_deposit(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // let asset_id = asset_id_from_nexo(&tx.input_currency);
    // let amount = to_u64(tx.input_amount, get_decimals(&tx.input_currency));
    // let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    // // Debit Savings
    // let input_effect = TxEffect {
    //     account_id,
    //     amount,
    //     datetime: tx.date_time_utc,
    // };

    // // Create Position
    // let position = Position {
    //     id: PositionId::from(tx.tx_id.clone()),
    //     product_id: ProductId::from("NEXO_TERM_DEPOSIT"),
    //     amount,
    //     start_date: tx.date_time_utc,
    //     end_date: tx.date_time_utc, // TODO: find end date
    // };

    // let asset = Asset {
    //     id: asset_id.clone(),
    //     chain_id: "".to_string(),
    //     decimals: get_decimals(&tx.input_currency),
    //     external_ids: Default::default(),
    // };

    // TxData {
    //     assets: vec![asset],
    //     products: vec![],
    //     positions: vec![position],
    //     transaction: Transaction {
    //         inputs: vec![input_effect],
    //         outputs: vec![], // TODO: represent flow to position?
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

fn process_unlock_term_deposit(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // let asset_id = asset_id_from_nexo(&tx.output_currency);
    // let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    // let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    // // Credit Savings
    // let output_effect = TxEffect {
    //     account_id,
    //     amount,
    //     datetime: tx.date_time_utc,
    // };

    // let asset = Asset {
    //     id: asset_id.clone(),
    //     chain_id: "".to_string(),
    //     decimals: get_decimals(&tx.output_currency),
    //     external_ids: Default::default(),
    // };

    // TxData {
    //     assets: vec![asset],
    //     products: vec![],
    //     positions: vec![], // TODO: Close position?
    //     transaction: Transaction {
    //         inputs: vec![], // TODO: from position?
    //         outputs: vec![output_effect],
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

fn process_term_interest(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    process_interest(tx)
}

fn process_transfer_from_pro_wallet(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // TxData {
    //     assets: vec![],
    //     products: vec![],
    //     positions: vec![],
    //     transaction: Transaction {
    //         inputs: vec![],
    //         outputs: vec![],
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

fn process_transfer_to_pro_wallet(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // TxData {
    //     assets: vec![],
    //     products: vec![],
    //     positions: vec![],
    //     transaction: Transaction {
    //         inputs: vec![],
    //         outputs: vec![],
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

fn process_exchange_deposited_on(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // // Input side
    // let in_asset_id = asset_id_from_nexo(&tx.input_currency);
    // let in_amount = to_u64(tx.input_amount, get_decimals(&tx.input_currency));
    // let in_account_id = AccountId::new(PROVIDER_ID.clone(), in_asset_id.clone());

    // let in_effect = TxEffect {
    //     account_id: in_account_id,
    //     amount: in_amount,
    //     datetime: tx.date_time_utc,
    // };

    // let in_asset = Asset {
    //     id: in_asset_id.clone(),
    //     chain_id: "".to_string(),
    //     decimals: get_decimals(&tx.input_currency),
    //     external_ids: Default::default(),
    // };

    // // Output side
    // let out_asset_id = asset_id_from_nexo(&tx.output_currency);
    // let out_amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    // let out_account_id = AccountId::new(PROVIDER_ID.clone(), out_asset_id.clone());

    // let out_effect = TxEffect {
    //     account_id: out_account_id,
    //     amount: out_amount,
    //     datetime: tx.date_time_utc,
    // };

    // let out_asset = Asset {
    //     id: out_asset_id.clone(),
    //     chain_id: "".to_string(),
    //     decimals: get_decimals(&tx.output_currency),
    //     external_ids: Default::default(),
    // };

    // TxData {
    //     assets: vec![in_asset, out_asset],
    //     products: vec![],
    //     positions: vec![],
    //     transaction: Transaction {
    //         inputs: vec![in_effect],
    //         outputs: vec![out_effect],
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

fn process_deposit_to_exchange(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // Treat as transfer? Or just ignore if it's internal bookkeeping?
    // "Deposit To Exchange" usually means moving from Savings to Pro/Exchange wallet.
    // Let's assume it's a transfer to Pro.
    // Input: Savings, Output: Pro?
    // If we track Pro as a separate provider or account, we should use that.
    // For now, let's treat it as a withdrawal from Savings (Input) and ignore destination (or maybe we should track it).

    // let asset_id = asset_id_from_nexo(&tx.input_currency);
    // let amount = to_u64(tx.input_amount, get_decimals(&tx.input_currency));
    // let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    // let input_effect = TxEffect {
    //     account_id,
    //     amount,
    //     datetime: tx.date_time_utc,
    // };

    // let asset = Asset {
    //     id: asset_id.clone(),
    //     chain_id: "".to_string(),
    //     decimals: get_decimals(&tx.input_currency),
    //     external_ids: Default::default(),
    // };

    // TxData {
    //     assets: vec![asset],
    //     products: vec![],
    //     positions: vec![],
    //     transaction: Transaction {
    //         inputs: vec![input_effect],
    //         outputs: vec![], // TODO: Destination?
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

fn process_withdraw_exchanged(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // "Withdraw Exchanged" - likely moving back from Exchange to Savings?
    // Input: Exchange? Output: Savings.

    // let asset_id = asset_id_from_nexo(&tx.output_currency);
    // let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    // let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    // let output_effect = TxEffect {
    //     account_id,
    //     amount,
    //     datetime: tx.date_time_utc,
    // };

    // let asset = Asset {
    //     id: asset_id.clone(),
    //     chain_id: "".to_string(),
    //     decimals: get_decimals(&tx.output_currency),
    //     external_ids: Default::default(),
    // };

    // TxData {
    //     assets: vec![asset],
    //     products: vec![],
    //     positions: vec![],
    //     transaction: Transaction {
    //         inputs: vec![], // From Exchange?
    //         outputs: vec![output_effect],
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

fn process_exchange_to_withdraw(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // "Exchange To Withdraw" - maybe similar to Withdraw Exchanged?
    // Or maybe the trade itself?
    // Let's assume it's a trade.
    // process_exchange_deposited_on(tx)
    todo!()
}

fn process_top_up_crypto(tx: &NexoTx) -> anyhow::Result<CollectTxnData> {
    // let asset_id = asset_id_from_nexo(&tx.output_currency);
    // let amount = to_u64(tx.output_amount, get_decimals(&tx.output_currency));
    // let account_id = AccountId::new(PROVIDER_ID.clone(), asset_id.clone());

    // let effect = TxEffect {
    //     account_id,
    //     amount,
    //     datetime: tx.date_time_utc,
    // };

    // let asset = Asset {
    //     id: asset_id.clone(),
    //     chain_id: "".to_string(),
    //     decimals: get_decimals(&tx.output_currency),
    //     external_ids: Default::default(),
    // };

    // TxData {
    //     assets: vec![asset],
    //     products: vec![],
    //     positions: vec![],
    //     transaction: Transaction {
    //         inputs: vec![],
    //         outputs: vec![effect],
    //         datetime: tx.date_time_utc,
    //     },
    // }
    todo!()
}

// /// Convert a Nexo asset identifier to an AssetId
// fn asset_id_from_nexo(nexo_asset: &str) -> AssetId {
//     match nexo_asset {
//         "ETH" => AssetId::Eth,
//         _ => AssetId::unknown(nexo_asset),
//     }
// }

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

    // #[test]
    // fn test_process_txn_top_up() -> anyhow::Result<()> {
    //     let nexo_tx = NexoTx {
    //         tx_id: "2".to_string(),
    //         kind: TransactionType::TopUpCrypto,
    //         input_currency: "".to_string(),
    //         input_amount: 0.0,
    //         output_currency: "BTC".to_string(),
    //         output_amount: 0.5,
    //         usd_equivalent: "20000.0".to_string(),
    //         details: "Top Up".to_string(),
    //         date_time_utc: Utc::now(),
    //     };
    //     let data = process_txn(&nexo_tx);
    //     assert_eq!(data.assets.len(), 1);
    //     assert_eq!(data.transaction.outputs.len(), 1);
    //     assert_eq!(data.transaction.outputs[0].amount, 50000000); // 0.5 BTC * 10^8
    //     Ok(())
    // }

    // #[test]
    // fn test_process_txn_exchange() -> anyhow::Result<()> {
    //     let nexo_tx = NexoTx {
    //         tx_id: "3".to_string(),
    //         kind: TransactionType::ExchangeDepositedOn,
    //         input_currency: "USDT".to_string(),
    //         input_amount: 1000.0,
    //         output_currency: "ETH".to_string(),
    //         output_amount: 0.5,
    //         usd_equivalent: "1000.0".to_string(),
    //         details: "Exchange".to_string(),
    //         date_time_utc: Utc::now(),
    //     };
    //     let data = process_txn(&nexo_tx);
    //     assert_eq!(data.assets.len(), 2);
    //     assert_eq!(data.transaction.inputs.len(), 1);
    //     assert_eq!(data.transaction.outputs.len(), 1);
    //     assert_eq!(data.transaction.inputs[0].amount, 1000000000); // 1000 USDT * 10^6
    //     assert_eq!(data.transaction.outputs[0].amount, 500000000000000000); // 0.5 ETH * 10^18
    //     Ok(())
    // }
}

pub enum NexoAssets {
    Eth,
    Nexo,
    Eurx,
    Bnb,
    Gbpx,
    Near,
    Dot,
    Pol,
    Usdt,
    Usdx,
    Other(String),
}
impl NexoAssets {
    fn try_from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "ETH" => Ok(NexoAssets::Eth),
            "BNB" => Ok(NexoAssets::Bnb),
            "EURX" => Ok(NexoAssets::Eurx),
            "NEXO" => Ok(NexoAssets::Nexo),
            "GBPX" => Ok(NexoAssets::Gbpx),
            "NEAR" => Ok(NexoAssets::Near),
            "DOT" => Ok(NexoAssets::Dot),
            "POL" => Ok(NexoAssets::Pol),
            "USDT" => Ok(NexoAssets::Usdt),
            "USDX" => Ok(NexoAssets::Usdx),
            _ => Err(anyhow::anyhow!(format!("Unknown nexo asset: {}", s))),
        }
    }
    fn local_asset_id(&self) -> anyhow::Result<AssetId> {
        match self {
            NexoAssets::Other(s) => Err(anyhow::anyhow!(format!("Unknown nexo asset: {}", s))),
            NexoAssets::Eth => Ok(AssetId::str("ETH")),
            NexoAssets::Bnb => Ok(AssetId::str("BNB")),
            NexoAssets::Usdt => Ok(AssetId::str("USDT")),
            NexoAssets::Nexo => Ok(AssetId::str("NEXO")),
            NexoAssets::Eurx => Ok(AssetId::str("EURX")),
            NexoAssets::Gbpx => Ok(AssetId::str("GBP")),
            NexoAssets::Near => Ok(AssetId::str("NEAR")),
            NexoAssets::Dot => Ok(AssetId::str("DOT")),
            NexoAssets::Pol => Ok(AssetId::str("POL")),
            NexoAssets::Usdx => Ok(AssetId::str("USDX")),
        }
    }
}

pub enum NexoProducts {
    EthFlexible,
    Eth1mth,
    NexoFlexible,
    Nexo3mth,
    Nexo12mth,
    EurxFlexible,
    BnbFlexible,
    GbpxFlexible,
    NearFlexible,
    DotFlexible,
    PolFlexible,
    UsdtFlexible,
    UsdxFlexible,
}
impl NexoProducts {
    fn asset(&self) -> anyhow::Result<NexoAssets> {
        match self {
            NexoProducts::EthFlexible => Ok(NexoAssets::Eth),
            NexoProducts::UsdtFlexible => Ok(NexoAssets::Usdt),
            NexoProducts::NexoFlexible => Ok(NexoAssets::Other("NEXO".to_string())),
            NexoProducts::Eth1mth => Ok(NexoAssets::Eth),
            NexoProducts::Nexo3mth => Ok(NexoAssets::Other("NEXO".to_string())),
            NexoProducts::Nexo12mth => Ok(NexoAssets::Other("NEXO".to_string())),
            NexoProducts::EurxFlexible => Ok(NexoAssets::Other("EUR".to_string())),
            NexoProducts::BnbFlexible => Ok(NexoAssets::Bnb),
            NexoProducts::GbpxFlexible => Ok(NexoAssets::Other("GBP".to_string())),
            NexoProducts::NearFlexible => Ok(NexoAssets::Other("NEAR".to_string())),
            NexoProducts::DotFlexible => Ok(NexoAssets::Other("DOT".to_string())),
            NexoProducts::PolFlexible => Ok(NexoAssets::Other("POL".to_string())),
            NexoProducts::UsdxFlexible => Ok(NexoAssets::Other("USD".to_string())),
        }
    }
    fn lock_duration(&self) -> anyhow::Result<NexoLockDuration> {
        match self {
            NexoProducts::EthFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::UsdtFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::NexoFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::Eth1mth => Ok(NexoLockDuration::_1mth),
            NexoProducts::Nexo3mth => Ok(NexoLockDuration::_3mth),
            NexoProducts::Nexo12mth => Ok(NexoLockDuration::_12mth),
            NexoProducts::EurxFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::BnbFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::GbpxFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::NearFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::DotFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::PolFlexible => Ok(NexoLockDuration::Flexible),
            NexoProducts::UsdxFlexible => Ok(NexoLockDuration::Flexible),
        }
    }
    fn id(&self) -> anyhow::Result<ProductId> {
        Ok(ProductId::from(format!(
            "Nexo_{}_{}",
            self.asset()?.local_asset_id()?,
            self.lock_duration()?.fmt()
        )))
    }
    fn apy(&self) -> f64 {
        match self {
            NexoProducts::EthFlexible => 6.5,
            NexoProducts::Eth1mth => 7.5,
            NexoProducts::NexoFlexible => 3.0,
            NexoProducts::Nexo3mth => 6.0,
            NexoProducts::Nexo12mth => 9.0,
            NexoProducts::UsdtFlexible => todo!(),
            NexoProducts::EurxFlexible => todo!(),
            NexoProducts::BnbFlexible => todo!(),
            NexoProducts::GbpxFlexible => todo!(),
            NexoProducts::NearFlexible => todo!(),
            NexoProducts::DotFlexible => todo!(),
            NexoProducts::PolFlexible => todo!(),
            NexoProducts::UsdxFlexible => todo!(),
        }
    }
    fn match_by_id(id: &str) -> anyhow::Result<Self> {
        match id {
            "ETH" => Ok(NexoProducts::EthFlexible),
            "USDT" => Ok(NexoProducts::UsdtFlexible),
            _ => Err(anyhow::anyhow!(format!("Unknown nexo product: {id}"))),
        }
    }
}

pub enum NexoLockDuration {
    Flexible,
    _1mth,
    _3mth,
    _12mth,
}
impl NexoLockDuration {
    fn fmt(&self) -> &str {
        match self {
            NexoLockDuration::Flexible => "Flexible",
            NexoLockDuration::_1mth => "1mth",
            NexoLockDuration::_3mth => "3mth",
            NexoLockDuration::_12mth => "12mth",
        }
    }
}
