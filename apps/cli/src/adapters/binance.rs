//! Binance API records -> exact, asset-balanced ledger transactions.
pub mod statements;
use anyhow::{Context, ensure};
use binance_client::{BinanceClient, account::AccountSnapshot};
use chrono::{DateTime, NaiveDateTime, Utc};
use lib_core::traits::{IsProvider, Issuer3};
use lib_core::{
    Asset, AssetId, CollectTxnData, PositionBalance, PositionId, Product, ProductId, ProviderId,
    Transaction, TxnEffect, UserPosition,
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde_json::Value;
use snafu::Snafu;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Snafu)]
pub enum BinanceServiceError {
    #[snafu(display("cannot initialize Binance provider"))]
    Initialize {
        source: binance_client::BinanceError,
    },
    #[snafu(display("cannot fetch Binance account history and balances"))]
    FetchAccount {
        source: binance_client::BinanceError,
    },
    #[snafu(display("cannot map Binance snapshot to the ledger"))]
    MapSnapshot { source: anyhow::Error },
    #[snafu(display("cannot read BINANCE_ADDITIONAL_SYMBOLS from the environment"))]
    SymbolsEnvironment { source: std::env::VarError },
    #[snafu(display(
        "BINANCE_ADDITIONAL_SYMBOLS must be a JSON array of symbol/baseAsset/quoteAsset objects"
    ))]
    SymbolsConfig { source: serde_json::Error },
    #[snafu(display("missing Binance asset metadata for position {}", position.0))]
    MissingPosition { position: PositionId },
    #[snafu(display("Binance transaction at {datetime} is not balanced per asset"))]
    UnbalancedTransaction { datetime: DateTime<Utc> },
    #[snafu(display("Binance reconciliation failed (missing/retained history, unsupported activity, or activity during fetch):\n{}", differences.join("\n")))]
    Reconciliation { differences: Vec<String> },
}

pub struct BinanceSvc {
    pub client: BinanceClient,
}

impl BinanceSvc {
    pub fn new() -> Result<Self, BinanceServiceError> {
        Ok(Self {
            client: snafu::ResultExt::context(BinanceClient::new(), InitializeSnafu)?,
        })
    }

    pub async fn fetch_account_data(&self) -> Result<BinanceAccountData, BinanceServiceError> {
        let symbols = additional_symbols()?;
        let snapshot = snafu::ResultExt::context(
            self.client.fetch_account_snapshot(&symbols).await,
            FetchAccountSnafu,
        )?;
        snafu::ResultExt::context(map_snapshot(&snapshot), MapSnapshotSnafu)
    }

    pub async fn fetch_products(&self) -> Result<Vec<Product>, BinanceServiceError> {
        Ok(self.fetch_account_data().await?.data.products)
    }

    pub async fn fetch_positions(&self) -> Result<Vec<UserPosition>, BinanceServiceError> {
        let mapped = self.fetch_account_data().await?;
        Ok(mapped
            .data
            .positions
            .into_iter()
            .filter(|p| mapped.owned.contains(&p.id))
            .collect())
    }
}

fn additional_symbols() -> Result<Vec<binance_client::account::Symbol>, BinanceServiceError> {
    match std::env::var("BINANCE_ADDITIONAL_SYMBOLS") {
        Ok(value) => snafu::ResultExt::context(serde_json::from_str(&value), SymbolsConfigSnafu),
        Err(std::env::VarError::NotPresent) => Ok(Vec::new()),
        Err(source) => Err(BinanceServiceError::SymbolsEnvironment { source }),
    }
}

#[async_trait::async_trait]
impl IsProvider for BinanceSvc {
    fn provider_id(&self) -> ProviderId {
        ProviderId::from("binance")
    }
    async fn fetch_all_txn_data(&self) -> anyhow::Result<CollectTxnData> {
        Ok(self.fetch_account_data().await?.data)
    }
}
impl Issuer3 for BinanceSvc {
    fn name() -> ProviderId {
        ProviderId::from("binance")
    }
}

pub struct BinanceAccountData {
    pub data: CollectTxnData,
    pub current_balances: Vec<PositionBalance>,
    pub owned: HashSet<PositionId>,
    pub position_assets: HashMap<PositionId, AssetId>,
}

impl BinanceAccountData {
    /// Compare the union of historical and current accounts, including closed assets.
    pub fn reconcile(&self) -> Result<(), BinanceServiceError> {
        let mut ledger = HashMap::<PositionId, i128>::new();
        for tx in &self.data.transactions {
            let mut asset_totals = HashMap::<AssetId, i128>::new();
            for (effects, sign) in [(&tx.inputs, -1), (&tx.outputs, 1)] {
                for e in effects {
                    let asset = snafu::OptionExt::context(
                        self.position_assets.get(&e.position_id),
                        MissingPositionSnafu {
                            position: e.position_id.clone(),
                        },
                    )?;
                    *asset_totals.entry(asset.clone()).or_default() += sign * i128::from(e.amount);
                    *ledger.entry(e.position_id.clone()).or_default() +=
                        sign * i128::from(e.amount);
                }
            }
            snafu::ensure!(
                asset_totals.values().all(|n| *n == 0),
                UnbalancedTransactionSnafu {
                    datetime: tx.datetime
                }
            );
        }
        ledger.retain(|position, _| self.owned.contains(position));
        reconcile_amounts(ledger, &self.current_balances)
    }
}

fn reconcile_amounts(
    ledger: HashMap<PositionId, i128>,
    observations: &[PositionBalance],
) -> Result<(), BinanceServiceError> {
    let mut actual = HashMap::<PositionId, i128>::new();
    for balance in observations {
        *actual.entry(balance.position_id.clone()).or_default() += balance.amount;
    }
    let positions: std::collections::BTreeSet<_> = ledger
        .keys()
        .chain(actual.keys())
        .map(|p| p.0.as_str())
        .collect();
    let mut differences = Vec::new();
    for id in positions {
        let position = PositionId(id.to_owned());
        let derived = ledger.get(&position).copied().unwrap_or(0);
        let current = actual.get(&position).copied().unwrap_or(0);
        if derived != current {
            differences.push(format!(
                "{id}: ledger={derived}, API={current}, difference={} smallest units",
                current - derived
            ));
        }
    }
    snafu::ensure!(differences.is_empty(), ReconciliationSnafu { differences });
    Ok(())
}

struct Movement {
    source: String,
    target: String,
    asset: String,
    amount: Decimal,
}
struct Event {
    datetime: DateTime<Utc>,
    movements: Vec<Movement>,
}

fn text<'a>(row: &'a Value, field: &str) -> anyhow::Result<&'a str> {
    row[field]
        .as_str()
        .with_context(|| format!("Binance record: missing string {field}"))
}
fn amount(row: &Value, field: &str) -> anyhow::Result<Decimal> {
    let n = text(row, field)?
        .parse::<Decimal>()
        .with_context(|| format!("invalid Binance decimal {field}"))?;
    ensure!(!n.is_sign_negative(), "negative Binance amount {field}");
    Ok(n)
}
fn time(row: &Value, field: &str) -> anyhow::Result<DateTime<Utc>> {
    let value = &row[field];
    if let Some(n) = value
        .as_i64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
    {
        return DateTime::from_timestamp_millis(n).context("Binance timestamp out of range");
    }
    Ok(NaiveDateTime::parse_from_str(text(row, field)?, "%Y-%m-%d %H:%M:%S")?.and_utc())
}
fn wallet(row: &Value) -> anyhow::Result<&'static str> {
    match row.get("walletType").and_then(Value::as_u64).unwrap_or(0) {
        0 => Ok("spot"),
        1 => Ok("funding"),
        _ => anyhow::bail!("unknown Binance walletType"),
    }
}
fn movement(source: &str, target: &str, asset: &str, n: Decimal) -> Movement {
    Movement {
        source: source.into(),
        target: target.into(),
        asset: asset.into(),
        amount: n,
    }
}
fn exchange(
    row: &Value,
    source_asset: &str,
    source_amount: &str,
    target_asset: &str,
    target_amount: &str,
) -> anyhow::Result<Vec<Movement>> {
    Ok(vec![
        movement(
            "spot",
            "external:market",
            text(row, source_asset)?,
            amount(row, source_amount)?,
        ),
        movement(
            "external:market",
            "spot",
            text(row, target_asset)?,
            amount(row, target_amount)?,
        ),
    ])
}

fn events(snapshot: &AccountSnapshot) -> anyhow::Result<Vec<Event>> {
    let mut events = Vec::new();
    for (kind, rows) in &snapshot.history {
        // Raw JSON equality avoids accidental cross-endpoint deduplication (e.g. IDs
        // scoped to symbols). Repeated pages are rejected by the client.
        let mut seen = HashSet::new();
        for row in rows {
            if !seen.insert(serde_json::to_string(row)?) {
                continue;
            }
            let mapped = (|| -> anyhow::Result<Option<Event>> {
                let (datetime, movements) = match kind.as_str() {
                    "deposits" => {
                        if !matches!(row["status"].as_u64(), Some(1 | 6)) { return Ok(None); }
                        (time(row, "insertTime")?, vec![movement("external:deposit", wallet(row)?, text(row, "coin")?, amount(row, "amount")?)])
                    }
                    "withdrawals" => {
                        if !matches!(row["status"].as_u64(), Some(4 | 6)) { return Ok(None); }
                        let asset = text(row, "coin")?;
                        (time(row, "applyTime")?, vec![movement(wallet(row)?, "external:withdrawal", asset, amount(row, "amount")?),
                            movement(wallet(row)?, "external:fees", asset, amount(row, "transactionFee")?)])
                    }
                    "dividends" => {
                        let asset = text(row, "asset")?;
                        let (source, target) = if row["direction"].as_i64() == Some(0) { ("spot", "external:rewards") } else { ("external:rewards", "spot") };
                        (time(row, "divTime")?, vec![movement(source, target, asset, amount(row, "amount")?)])
                    }
                    "convert" => {
                        if text(row, "orderStatus")? != "SUCCESS" { return Ok(None); }
                        (time(row, "createTime")?, exchange(row, "fromAsset", "fromAmount", "toAsset", "toAmount")?)
                    }
                    "dust" => {
                        for detail in row["userAssetDribbletDetails"].as_array().context("missing dust details")? {
                            let target_asset = detail.get("targetAsset").and_then(Value::as_str).unwrap_or("BNB");
                            // transferedAmount is the net received amount; serviceChargeAmount
                            // describes the deducted portion, not an additional wallet debit.
                            events.push(Event { datetime: time(detail, "operateTime")?, movements: vec![
                                movement("spot", "external:market", text(detail, "fromAsset")?, amount(detail, "amount")?),
                                movement("external:market", "spot", target_asset, amount(detail, "transferedAmount")?),
                                movement("external:market", "external:fees", target_asset, amount(detail, "serviceChargeAmount")?),
                            ] });
                        }
                        return Ok(None);
                    }
                    _ if kind.starts_with("trades/") => {
                        let symbol = snapshot.symbols.iter().find(|s| s.symbol == kind[7..]).context("missing trade symbol metadata")?;
                        let buyer = row["isBuyer"].as_bool().context("missing isBuyer")?;
                        let (base_source, base_target, quote_source, quote_target) = if buyer {
                            ("external:market", "spot", "spot", "external:market")
                        } else { ("spot", "external:market", "external:market", "spot") };
                        (time(row, "time")?, vec![
                            movement(base_source, base_target, &symbol.base_asset, amount(row, "qty")?),
                            movement(quote_source, quote_target, &symbol.quote_asset, amount(row, "quoteQty")?),
                            movement("spot", "external:fees", text(row, "commissionAsset")?, amount(row, "commission")?),
                        ])
                    }
                    _ if kind.starts_with("flexible/") || kind.starts_with("locked/") => {
                        let parts: Vec<_> = kind.split('/').collect();
                        let account = parts[0];
                        let status = row.get("status").and_then(Value::as_str);
                        let asset = text(row, "asset")?;
                        let moves = match parts[1] {
                            "subscriptionRecord" => {
                                if status.is_some_and(|s| matches!(s, "FAILED" | "FAIL" | "PENDING")) { return Ok(None); }
                                ensure!(status.is_none_or(|s| matches!(s, "SUCCESS" | "PURCHASE_SUCCESS")), "unknown Earn subscription status");
                                if row.get("amtFromSpot").is_some() || row.get("amtFromFunding").is_some() {
                                    let spot = amount(row, "amtFromSpot")?;
                                    let funding = amount(row, "amtFromFunding")?;
                                    ensure!(spot.checked_add(funding) == Some(amount(row, "amount")?), "Earn source amounts do not match subscription");
                                    vec![movement("spot", account, asset, spot), movement("funding", account, asset, funding)]
                                } else {
                                    let source = match row.get("sourceAccount").and_then(Value::as_str).unwrap_or("SPOT") {
                                        "SPOT" => "spot", "FUND" | "FUNDING" => "funding",
                                        _ => anyhow::bail!("unsupported Earn source account"),
                                    };
                                    vec![movement(source, account, asset, amount(row, "amount")?)]
                                }
                            }
                            "redemptionRecord" => {
                                if status.is_some_and(|s| matches!(s, "FAILED" | "FAIL" | "PENDING" | "REDEEMING" | "WAIT_FOR_REDEMPTION")) { return Ok(None); }
                                ensure!(status.is_none_or(|s| matches!(s, "PAID" | "SUCCESS" | "REDEEMED")), "unknown Earn redemption status");
                                if row.get("isComplete").and_then(Value::as_bool) == Some(false) { return Ok(None); }
                                let destination = row.get("redeemTo").or_else(|| row.get("destAccount")).and_then(Value::as_str)
                                    .unwrap_or(if account == "locked" && row["type"] == "NEW_TRANSFERRED" { "FLEXIBLE" } else { "SPOT" });
                                let target = match destination {
                                    "SPOT" => "spot", "FLEXIBLE" => "flexible", "FUND" | "FUNDING" => "funding",
                                    _ => anyhow::bail!("unsupported Earn redemption destination"),
                                };
                                let received = amount(row, "amount")?;
                                let loss = if row.get("lossAmount").is_some() { amount(row, "lossAmount")? } else { Decimal::ZERO };
                                if row.get("originalAmount").is_some() {
                                    ensure!(received.checked_add(loss) == Some(amount(row, "originalAmount")?), "Earn redemption amount and loss differ from original principal");
                                }
                                let mut moves = vec![movement(account, target, asset, received), movement(account, "external:fees", asset, loss)];
                                // Legacy fixed-term Savings settlements include interest in
                                // the redeemed amount. The matching subscription is evidence
                                // of principal; current positions never determine this reward.
                                if account == "flexible" && row.get("productId").and_then(Value::as_str).is_some_and(|id| id.contains("DAYSS")) {
                                    let product = text(row, "productId")?;
                                    let subscriptions: Vec<_> = snapshot.history.get("flexible/subscriptionRecord/")
                                        .into_iter().flatten().filter(|s| s["productId"] == product && s["asset"] == asset && s["status"] == "SUCCESS" && s["time"].as_i64() < row["time"].as_i64()).collect();
                                    ensure!(subscriptions.len() == 1, "legacy Savings maturity requires one evidenced principal subscription");
                                    let settlements = snapshot.history.get(kind).into_iter().flatten()
                                        .filter(|r| r["productId"] == product && r["asset"] == asset && r["status"] == "PAID").count();
                                    ensure!(settlements == 1, "ambiguous repeated legacy Savings maturities");
                                    let principal = amount(subscriptions[0], "amount")?;
                                    ensure!(received >= principal && loss.is_zero(), "legacy Savings redemption is smaller than principal");
                                    moves.push(movement("external:rewards", account, asset, received - principal));
                                }
                                moves
                            }
                            "rewardsRecord" => {
                                // Flexible REALTIME rewards stay in Earn; bonus/airdrop and
                                // locked interest are paid to Spot.
                                let target = if account == "flexible" && parts.get(2) == Some(&"REALTIME") { "flexible" } else { "spot" };
                                let field = if account == "locked" { "amount" } else { "rewards" };
                                vec![movement("external:rewards", target, asset, amount(row, field)?)]
                            }
                            _ => anyhow::bail!("unsupported Binance Earn history {kind}"),
                        };
                        let datetime_field = if parts[1] == "redemptionRecord" && row.get("deliverDate").is_some() {
                            "deliverDate"
                        } else { "time" };
                        (time(row, datetime_field)?, moves)
                    }
                    "transfer/MAIN_FUNDING" | "transfer/FUNDING_MAIN" => {
                        if text(row, "status")? != "CONFIRMED" { return Ok(None); }
                        let (source, target) = if kind == "transfer/MAIN_FUNDING" { ("spot", "funding") } else { ("funding", "spot") };
                        (time(row, "timestamp")?, vec![movement(source, target, text(row, "asset")?, amount(row, "amount")?)])
                    }
                    _ if kind.starts_with("fiat/orders/") => {
                        if text(row, "status")? != "Successful" { return Ok(None); }
                        let asset = text(row, "fiatCurrency")?;
                        let n = amount(row, "indicatedAmount")?;
                        let fee = amount(row, "totalFee")?;
                        let moves = if kind.ends_with("/0") {
                            vec![movement("external:deposit", "spot", asset, n), movement("spot", "external:fees", asset, fee)]
                        } else {
                            vec![movement("spot", "external:withdrawal", asset, amount(row, "amount")?), movement("spot", "external:fees", asset, fee)]
                        };
                        (time(row, "createTime")?, moves)
                    }
                    _ if kind.starts_with("fiat/payments/") => {
                        if text(row, "status")? != "Completed" { return Ok(None); }
                        // Card purchases/sales settle externally; fiat never enters Spot.
                        let asset = text(row, "cryptoCurrency")?;
                        let (source, target) = if kind.ends_with("/0") { ("external:fiat-payment", "spot") } else { ("spot", "external:fiat-payment") };
                        let field = if kind.ends_with("/0") { "obtainAmount" } else { "sourceAmount" };
                        let mut moves = vec![movement(source, target, asset, amount(row, field)?)];
                        if row.get("paymentMethod").and_then(Value::as_str) == Some("Cash Balance") {
                            // Cash Balance buys debit the Binance fiat position instead
                            // of an external card or bank.
                            moves.push(movement("spot", "external:fiat-payment", text(row, "fiatCurrency")?, amount(row, "sourceAmount")?));
                        }
                        (time(row, "createTime")?, moves)
                    }
                    _ => anyhow::bail!("unsupported Binance history {kind}"),
                };
                Ok(Some(Event { datetime, movements }))
            })().with_context(|| format!("mapping Binance {kind}"))?;
            if let Some(event) = mapped {
                events.push(event);
            }
        }
    }
    events.sort_by_key(|e| e.datetime);
    Ok(events)
}

fn position(account: &str, asset: &str) -> PositionId {
    PositionId::from(format!("BINANCE:{account}:{asset}"))
}
fn units(n: Decimal, decimals: u8) -> anyhow::Result<u64> {
    let scaled = n
        .checked_mul(Decimal::from(
            10u64
                .checked_pow(u32::from(decimals))
                .context("Binance asset precision too large")?,
        ))
        .context("Binance amount overflow")?;
    ensure!(scaled.fract().is_zero(), "Binance amount loses precision");
    scaled
        .to_u64()
        .context("Binance amount exceeds ledger u64 range")
}

pub fn map_snapshot(snapshot: &AccountSnapshot) -> anyhow::Result<BinanceAccountData> {
    ensure!(
        snapshot.version == 1,
        "unsupported Binance snapshot version"
    );
    for account in ["spot", "funding", "flexible", "locked"] {
        ensure!(
            snapshot.balances.contains_key(account),
            "Binance snapshot is missing {account} balances"
        );
    }
    let events = events(snapshot)?;
    let mut current = Vec::new();
    for (account, rows) in &snapshot.balances {
        ensure!(
            matches!(account.as_str(), "spot" | "funding" | "flexible" | "locked"),
            "unsupported Binance balance account"
        );
        for row in rows {
            let asset = text(row, "asset")?.to_owned();
            let n = match account.as_str() {
                "spot" => amount(row, "free")?.checked_add(amount(row, "locked")?),
                "funding" => {
                    let mut sum = Decimal::ZERO;
                    for field in ["free", "locked", "freeze", "withdrawing"] {
                        sum = sum
                            .checked_add(amount(row, field)?)
                            .context("funding balance overflow")?;
                    }
                    Some(sum)
                }
                "flexible" => Some(amount(row, "totalAmount")?),
                "locked" => Some(amount(row, "amount")?),
                _ => unreachable!(),
            }
            .context("Binance balance overflow")?;
            current.push((account.clone(), asset, n));
        }
    }
    map_movements(events, current, snapshot.fetched_at, &BTreeMap::new())
}

fn map_movements(
    events: Vec<Event>,
    current: Vec<(String, String, Decimal)>,
    fetched_at: DateTime<Utc>,
    existing_scales: &BTreeMap<String, u8>,
) -> anyhow::Result<BinanceAccountData> {
    let mut precision = existing_scales.clone();
    for (asset, n) in events
        .iter()
        .flat_map(|e| e.movements.iter().map(|m| (&m.asset, m.amount)))
        .chain(current.iter().map(|(_, a, n)| (a, *n)))
    {
        let decimals = u8::try_from(n.normalize().scale())?.max(8);
        ensure!(
            existing_scales
                .get(asset)
                .is_none_or(|existing| *existing >= decimals),
            "existing ledger precision for {asset} is insufficient; rebuild the cached DB"
        );
        precision
            .entry(asset.clone())
            .and_modify(|d| *d = (*d).max(decimals))
            .or_insert(decimals);
    }
    let mut accounts = BTreeMap::<String, (String, bool)>::new();
    let mut transactions = Vec::new();
    for event in events {
        let mut tx = Transaction {
            datetime: event.datetime,
            inputs: vec![],
            outputs: vec![],
        };
        for m in event.movements {
            let n = units(m.amount, precision[&m.asset])?;
            if n == 0 {
                continue;
            }
            for (account, effects) in [(&m.source, &mut tx.inputs), (&m.target, &mut tx.outputs)] {
                let id = position(account, &m.asset);
                accounts.insert(
                    id.0.clone(),
                    (m.asset.clone(), !account.starts_with("external:")),
                );
                effects.push(TxnEffect {
                    position_id: id,
                    amount: n,
                    datetime: event.datetime,
                });
            }
        }
        if !tx.inputs.is_empty() {
            transactions.push(tx);
        }
    }
    let mut current_balances = Vec::new();
    for (account, asset, n) in current {
        let id = position(&account, &asset);
        accounts.insert(id.0.clone(), (asset.clone(), true));
        current_balances.push(PositionBalance {
            position_id: id,
            datetime: fetched_at,
            amount: i128::from(units(n, precision[&asset])?),
        });
    }
    let mut positions = Vec::new();
    let mut products = Vec::new();
    let mut owned = HashSet::new();
    let mut position_assets = HashMap::new();
    for (id, (asset, is_owned)) in accounts {
        let id = PositionId::from(id);
        let product_id = ProductId::from(&id.0);
        if is_owned {
            owned.insert(id.clone());
        }
        position_assets.insert(id.clone(), AssetId::str(&asset));
        products.push(Product {
            id: product_id.clone(),
            asset_id: AssetId::str(&asset),
            apy: 0.0,
        });
        positions.push(UserPosition {
            id,
            product_id,
            start_date: None,
            end_date: None,
        });
    }
    Ok(BinanceAccountData {
        data: CollectTxnData {
            transactions,
            positions,
            products,
            assets: precision
                .into_iter()
                .map(|(ticker, decimals)| Asset {
                    id: AssetId::str(&ticker),
                    decimals,
                    symbol: ticker.clone(),
                    ticker,
                })
                .collect(),
        },
        current_balances,
        owned,
        position_assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use binance_client::account::Symbol;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn sample() -> AccountSnapshot {
        let date = DateTime::from_timestamp_millis(1_700_000_000_000).unwrap();
        AccountSnapshot {
            version: 1,
            from: date,
            fetched_at: date,
            symbols: vec![Symbol {
                symbol: "BTCUSDT".into(),
                base_asset: "BTC".into(),
                quote_asset: "USDT".into(),
            }],
            history: BTreeMap::from([
                (
                    "deposits".into(),
                    vec![
                        json!({"id":"1", "coin":"BTC", "amount":"1", "status":1, "insertTime":1700000000000i64}),
                        json!({"id":"2", "coin":"BNB", "amount":"1", "status":6, "insertTime":1700000000000i64}),
                        json!({"id":"3", "coin":"BTC", "amount":"100", "status":0, "insertTime":1700000000000i64}),
                    ],
                ),
                (
                    "trades/BTCUSDT".into(),
                    vec![json!({"id":1, "time":1700000000001i64, "isBuyer":false,
                    "qty":"0.5", "quoteQty":"10000", "commission":"0.001", "commissionAsset":"BNB"})],
                ),
                (
                    "flexible/subscriptionRecord/".into(),
                    vec![
                        json!({"time":1700000000002i64, "asset":"USDT", "amount":"1000", "status":"SUCCESS"}),
                    ],
                ),
                (
                    "flexible/rewardsRecord/REALTIME".into(),
                    vec![json!({"time":1700000000003i64, "asset":"USDT", "rewards":"0.123456789"})],
                ),
                (
                    "locked/subscriptionRecord/".into(),
                    vec![
                        json!({"time":1700000000004i64, "asset":"BNB", "amount":"0.1", "status":"SUCCESS"}),
                    ],
                ),
                (
                    "locked/rewardsRecord/".into(),
                    vec![json!({"time":1700000000005i64, "asset":"BNB", "amount":"0.01"})],
                ),
                (
                    "transfer/MAIN_FUNDING".into(),
                    vec![
                        json!({"timestamp":1700000000006i64, "asset":"USDT", "amount":"500", "status":"CONFIRMED"}),
                    ],
                ),
                (
                    "withdrawals".into(),
                    vec![
                        json!({"id":"4", "coin":"BTC", "amount":"0.1", "transactionFee":"0.001", "status":6,
                    "applyTime":"2023-11-14 22:13:21"}),
                    ],
                ),
            ]),
            balances: BTreeMap::from([
                (
                    "spot".into(),
                    vec![
                        json!({"asset":"BTC", "free":"0.399", "locked":"0"}),
                        json!({"asset":"BNB", "free":"0.909", "locked":"0"}),
                        json!({"asset":"USDT", "free":"7500", "locked":"1000"}),
                    ],
                ),
                (
                    "flexible".into(),
                    vec![json!({"asset":"USDT", "totalAmount":"1000.123456789"})],
                ),
                (
                    "locked".into(),
                    vec![json!({"asset":"BNB", "amount":"0.1"})],
                ),
                (
                    "funding".into(),
                    vec![
                        json!({"asset":"USDT", "free":"500", "locked":"0", "freeze":"0", "withdrawing":"0"}),
                    ],
                ),
            ]),
        }
    }

    #[test]
    fn maps_owned_external_earn_funding_and_third_asset_fees() -> anyhow::Result<()> {
        let mapped = map_snapshot(&sample())?;
        mapped.reconcile()?;
        assert_eq!(mapped.data.transactions.len(), 9);
        assert!(mapped.owned.contains(&position("locked", "BNB")));
        assert!(!mapped.owned.contains(&position("external:fees", "BNB")));
        assert_eq!(
            mapped
                .data
                .assets
                .iter()
                .find(|a| a.ticker == "USDT")
                .unwrap()
                .decimals,
            9
        );
        let trade = &mapped.data.transactions[2];
        assert!(
            trade
                .inputs
                .iter()
                .any(|e| e.position_id == position("spot", "BNB") && e.amount == 100_000)
        );
        Ok(())
    }

    #[test]
    fn detects_missing_history_current_only_and_closed_accounts() -> anyhow::Result<()> {
        let mut snapshot = sample();
        snapshot.history.get_mut("deposits").unwrap().remove(0);
        let err = map_snapshot(&snapshot)?
            .reconcile()
            .unwrap_err()
            .to_string();
        assert!(err.contains("BINANCE:spot:BTC"));
        assert!(err.contains("difference=100000000"));
        snapshot = sample();
        snapshot
            .balances
            .get_mut("spot")
            .unwrap()
            .push(json!({"asset":"NEW", "free":"1", "locked":"0"}));
        assert!(
            map_snapshot(&snapshot)?
                .reconcile()
                .unwrap_err()
                .to_string()
                .contains("BINANCE:spot:NEW")
        );
        snapshot = sample();
        snapshot
            .balances
            .get_mut("spot")
            .unwrap()
            .retain(|r| r["asset"] != "BTC");
        assert!(
            map_snapshot(&snapshot)?
                .reconcile()
                .unwrap_err()
                .to_string()
                .contains("BINANCE:spot:BTC")
        );
        Ok(())
    }

    #[test]
    fn buy_sell_convert_dividends_dust_and_duplicate_records() -> anyhow::Result<()> {
        let mut snapshot = sample();
        snapshot.history.clear();
        snapshot.balances.clear();
        for account in ["spot", "funding", "flexible", "locked"] {
            snapshot.balances.insert(account.into(), vec![]);
        }
        snapshot.history.insert(
            "deposits".into(),
            vec![json!({"coin":"BTC", "amount":"1", "status":1, "insertTime":1700000000000i64})],
        );
        snapshot.history.insert("trades/BTCUSDT".into(), vec![
            json!({"id":1,"time":1700000000001i64,"isBuyer":false,"qty":"1","quoteQty":"100","commission":"1","commissionAsset":"USDT"}),
            json!({"id":2,"time":1700000000002i64,"isBuyer":true,"qty":"0.5","quoteQty":"50","commission":"0.01","commissionAsset":"BTC"}),
        ]);
        let conversion = json!({"orderStatus":"SUCCESS", "createTime":1700000000003i64, "fromAsset":"USDT", "fromAmount":"49", "toAsset":"BNB", "toAmount":"1"});
        snapshot
            .history
            .insert("convert".into(), vec![conversion.clone(), conversion]);
        snapshot.history.insert(
            "dividends".into(),
            vec![json!({"asset":"BNB","amount":"0.1","divTime":1700000000004i64})],
        );
        snapshot.history.insert("dust".into(), vec![json!({"userAssetDribbletDetails":[{"operateTime":1700000000005i64,
            "fromAsset":"BTC","amount":"0.49","transferedAmount":"0.05","serviceChargeAmount":"0.001"}]})]);
        snapshot.balances.insert(
            "spot".into(),
            vec![json!({"asset":"BNB","free":"1.15","locked":"0"})],
        );
        let mapped = map_snapshot(&snapshot)?;
        mapped.reconcile()?;
        assert_eq!(mapped.data.transactions.len(), 6);
        Ok(())
    }

    #[test]
    fn rejects_unknown_records_negative_and_overflow_amounts() {
        let mut snapshot = sample();
        snapshot.history.insert("unknown".into(), vec![json!({})]);
        assert!(map_snapshot(&snapshot).is_err());
        assert!(units(Decimal::MAX, 8).is_err());
        assert!(units("0.000000001".parse().unwrap(), 8).is_err());
        snapshot = sample();
        snapshot.history.get_mut("deposits").unwrap()[0]["amount"] = json!("-1");
        assert!(map_snapshot(&snapshot).is_err());
    }

    #[test]
    fn maps_split_funding_subscriptions_and_redemption_losses() -> anyhow::Result<()> {
        let mut snapshot = sample();
        snapshot.history.get_mut("deposits").unwrap().push(json!({
            "id":"fund", "coin":"USDT", "amount":"1000", "status":1, "walletType":1, "insertTime":1700000000000i64
        }));
        snapshot
            .history
            .get_mut("flexible/subscriptionRecord/")
            .unwrap()[0] = json!({
            "time":1700000000002i64, "asset":"USDT", "amount":"1000", "amtFromSpot":"300", "amtFromFunding":"700", "status":"SUCCESS"
        });
        snapshot.balances.get_mut("spot").unwrap()[2]["free"] = json!("8200");
        snapshot.balances.get_mut("funding").unwrap()[0]["free"] = json!("800");
        snapshot.history.insert("locked/redemptionRecord/".into(), vec![json!({
            "asset":"BNB", "time":1700000000007i64, "deliverDate":1700000000008i64,
            "amount":"0.09", "originalAmount":"0.1", "lossAmount":"0.01", "status":"PAID", "isComplete":true, "redeemTo":"FUNDING"
        })]);
        snapshot.balances.get_mut("locked").unwrap().clear();
        snapshot.balances.get_mut("funding").unwrap().push(
            json!({"asset":"BNB", "free":"0.09", "locked":"0", "freeze":"0", "withdrawing":"0"}),
        );
        Ok(map_snapshot(&snapshot)?.reconcile()?)
    }

    fn real_cache_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/binance/account-v1.json")
    }

    async fn cached_snapshot(path: &Path) -> anyhow::Result<AccountSnapshot> {
        match std::fs::read(path) {
            Ok(bytes) => {
                eprintln!("Loading Binance snapshot {}", path.display());
                return serde_json::from_slice(&bytes)
                    .context("invalid Binance cache; remove it explicitly to refetch");
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
        let client = BinanceClient::new().context(
            "first Binance cached test run requires BINANCE_API_KEY_ID and BINANCE_API_KEY_SECRET",
        )?;
        let parent = path.parent().context("cache path has no parent")?;
        eprintln!("Fetching Binance snapshot; reusing cached historical responses");
        let progress = std::sync::Mutex::new((0usize, String::new()));
        let snapshot = client
            .with_backoff_progress(|endpoint, status, attempt, delay| {
                eprintln!(
                    "Binance {endpoint}: HTTP {status}, attempt {attempt}; cooling down for {:.1}s",
                    delay.as_secs_f64()
                );
            })
            .with_request_progress(move |endpoint, cached| {
                let mut progress = progress.lock().unwrap_or_else(|e| e.into_inner());
                progress.0 += 1;
                if progress.0 == 1 || progress.0 % 100 == 0 || progress.1 != endpoint {
                    eprintln!(
                        "Loading Binance responses: {} completed; {endpoint} ({})",
                        progress.0,
                        if cached { "cache" } else { "API" }
                    );
                }
                progress.1 = endpoint.to_owned();
            })
            .with_history_cache(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/api_responses/binance"),
            )
            .fetch_account_snapshot(&additional_symbols()?)
            .await?;
        std::fs::create_dir_all(parent)?;
        // Atomic rename prevents partially written snapshots from becoming fixtures.
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(file.as_file_mut(), &snapshot)?;
        file.as_file().sync_all()?;
        file.persist(path)?;
        Ok(snapshot)
    }

    #[tokio::test]
    async fn replays_cached_raw_snapshot_without_constructing_client() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("account.json");
        std::fs::write(&path, serde_json::to_vec(&sample())?)?;
        map_snapshot(&cached_snapshot(&path).await?)?.reconcile()?;
        Ok(())
    }

    const CACHE_SOURCE: &str = "binance-account-test";
    // This dedicated DB is a frozen fixture; delete it to remap the raw snapshot.
    const CACHE_VERSION: &str = "account-v1";

    async fn load_cached_account(
        snapshot_path: &Path,
        db_path: &Path,
    ) -> anyhow::Result<lib_core::storage::Store> {
        use lib_core::storage::Store;
        std::fs::create_dir_all(db_path.parent().context("DB path has no parent")?)?;
        let store = Store::open(db_path)?;
        if !store.has_imported_file(CACHE_SOURCE, CACHE_VERSION)? {
            let snapshot = cached_snapshot(snapshot_path).await?;
            eprintln!("Mapping Binance snapshot transactions");
            let mapped = map_snapshot(&snapshot)?;
            // Structural failures prevent import; a balance discrepancy is preserved
            // in the fixture and must keep failing on subsequent runs.
            match mapped.reconcile() {
                Ok(()) | Err(BinanceServiceError::Reconciliation { .. }) => (),
                Err(error) => return Err(error.into()),
            }
            let scales: HashMap<_, _> = mapped
                .data
                .assets
                .iter()
                .map(|asset| (asset.id.clone(), asset.decimals))
                .collect();
            let positions: Vec<_> = mapped
                .position_assets
                .iter()
                .map(|(position, asset)| {
                    (
                        position.clone(),
                        asset.clone(),
                        scales[asset],
                        mapped.owned.contains(position),
                    )
                })
                .collect();
            let transactions: Vec<_> = mapped
                .data
                .transactions
                .into_iter()
                .enumerate()
                .map(|(index, transaction)| (format!("BINANCE-TEST:{index}"), transaction))
                .collect();
            store.save_file_import_with_observations(
                CACHE_SOURCE,
                CACHE_VERSION,
                &positions,
                &transactions,
                &mapped.current_balances,
                |done, total| eprintln!("Writing Binance transactions: {done}/{total}"),
            )?;
            eprintln!("Imported {} Binance transactions", transactions.len());
        } else {
            eprintln!("Reusing Binance test DB {}", db_path.display());
        }
        Ok(store)
    }

    async fn reconcile_cached_account(snapshot_path: &Path, db_path: &Path) -> anyhow::Result<()> {
        let store = load_cached_account(snapshot_path, db_path).await?;
        let ledger = store
            .latest_balances()?
            .into_iter()
            .map(|balance| (balance.position_id, balance.amount))
            .collect();
        Ok(reconcile_amounts(
            ledger,
            &store.balance_observations(CACHE_SOURCE, CACHE_VERSION)?,
        )?)
    }

    fn real_db_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache/test-data/test-binance-api-v1.duckdb")
    }

    #[tokio::test]
    async fn persistent_cache_reuses_import_and_rebuilds_from_raw_snapshot() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let snapshot = dir.path().join("account.json");
        let db = dir.path().join("account.duckdb");
        let bytes = serde_json::to_vec(&sample())?;
        std::fs::write(&snapshot, &bytes)?;
        reconcile_cached_account(&snapshot, &db).await?;
        // No snapshot, mapping, credentials or API are needed after the import.
        std::fs::remove_file(&snapshot)?;
        reconcile_cached_account(&snapshot, &db).await?;
        std::fs::write(&snapshot, &bytes)?;
        std::fs::remove_file(&db)?;
        reconcile_cached_account(&snapshot, &db).await?;
        Ok(())
    }

    #[tokio::test]
    async fn persistent_cache_preserves_independent_balance_discrepancy() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let snapshot_path = dir.path().join("account.json");
        let db = dir.path().join("account.duckdb");
        let mut snapshot = sample();
        snapshot.history.remove("deposits");
        std::fs::write(&snapshot_path, serde_json::to_vec(&snapshot)?)?;
        assert!(reconcile_cached_account(&snapshot_path, &db).await.is_err());
        std::fs::remove_file(&snapshot_path)?;
        let error = reconcile_cached_account(&snapshot_path, &db)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("ledger="));
        Ok(())
    }

    /// First run fetches real data; later runs use the persisted ledger and observations.
    #[tokio::test]
    async fn cached_binance_account_reconciles() -> anyhow::Result<()> {
        reconcile_cached_account(&real_cache_path(), &real_db_path()).await
    }

    #[tokio::test]
    #[ignore = "displays private Binance asset balances; first run needs API credentials"]
    async fn latest_amount_after_cached_account() -> anyhow::Result<()> {
        let store = load_cached_account(&real_cache_path(), &real_db_path()).await?;
        let balances = store.latest_balances()?;
        if let Some(balance) = balances.first() {
            println!(
                "Binance asset balances derived from cached transactions at {}:",
                balance.datetime
            );
            for (asset, amount) in store.asset_units_at(balance.datetime)? {
                let scale = store.asset_scale(&asset)?;
                let amount = Decimal::try_from_i128_with_scale(amount, u32::from(scale))?;
                println!("  {}: {}", asset.0, amount.normalize());
            }
        } else {
            println!("No Binance transactions in the cached ledger");
        }
        println!(
            "History may be incomplete; use the reconciliation test to check against API balances."
        );
        Ok(())
    }

    // Keep the existing command cache-first for compatibility.
    #[tokio::test]
    #[ignore = "first run fetches private Binance data; subsequent runs reuse .cache"]
    async fn live_binance_account_reconciles() -> anyhow::Result<()> {
        reconcile_cached_account(&real_cache_path(), &real_db_path()).await
    }

    #[tokio::test]
    #[ignore = "always fetches private Binance account data; requires BINANCE_API_KEY_ID and BINANCE_API_KEY_SECRET"]
    async fn uncached_binance_account_reconciles() -> anyhow::Result<()> {
        Ok(BinanceSvc::new()?.fetch_account_data().await?.reconcile()?)
    }
}
