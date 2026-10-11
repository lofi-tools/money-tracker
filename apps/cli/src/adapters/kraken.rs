//! Account-scoped Kraken ledger ingestion and independent balance reconciliation.
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use kraken_client::{Snapshot, fingerprint};
use lib_core::{
    Account, AccountId, AssetId, PositionBalance, PositionId, PositionRole, Store, Transaction,
    TxnEffect, Units,
};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

pub const SOURCE: &str = "kraken-ledger-v1";
type Registration = (PositionId, AssetId, u8, PositionRole);

struct Mapped {
    positions: Vec<Registration>,
    transactions: Vec<(String, Transaction)>,
    observations: Vec<PositionBalance>,
}

fn time(value: &str) -> Result<DateTime<Utc>> {
    let ns: i64 = Units::from_decimal(value, 9)?
        .to_string()
        .parse()
        .context("Kraken ledger time outside supported range")?;
    Ok(DateTime::from_timestamp_nanos(ns))
}

fn precision(value: &str) -> Result<u8> {
    let fraction = value
        .split_once('.')
        .map(|(_, f)| f.trim_end_matches('0'))
        .unwrap_or("");
    let scale =
        u8::try_from(fraction.len()).context("Kraken amount precision exceeds supported scale")?;
    Units::from_decimal(value, scale)?;
    Ok(scale)
}

/// Only explicitly supported provider aliases/suffixes; never strip X/Z generally.
fn resolve(snapshot: &Snapshot, code: &str) -> Result<(AssetId, String, u8)> {
    let (base, bucket) = match code.split_once('.') {
        None => (code, "spot"),
        Some((base, "S")) => (base, "staking"),
        Some((base, "M")) => (base, "opt-in"),
        Some((base, "B")) => (base, "earn"),
        Some((base, "F")) => (base, "rewards"),
        Some(_) => anyhow::bail!("unknown/tokenized Kraken asset suffix: {code}"),
    };
    let candidates: Vec<_> = snapshot
        .assets
        .iter()
        .filter(|(id, info)| id.as_str() == base || info.altname == base)
        .collect();
    ensure!(
        !candidates.is_empty(),
        "unknown Kraken asset {code}; asset metadata is required"
    );
    let info = candidates[0].1;
    ensure!(
        candidates
            .iter()
            .all(|(_, other)| other.altname == info.altname
                && other.decimals == info.decimals
                && other.aclass == info.aclass),
        "ambiguous Kraken asset {code}"
    );
    ensure!(
        info.aclass == "currency",
        "unsupported Kraken asset class for {code}"
    );
    let symbol = match info.altname.as_str() {
        "XBT" => "BTC",
        "XDG" => "DOGE",
        other => other,
    };
    ensure!(
        !symbol.is_empty() && !symbol.contains('.') && symbol != "KFEE",
        "unsupported Kraken asset {code}"
    );
    Ok((AssetId::str(symbol), bucket.into(), info.decimals))
}

fn owned(bucket: &str, asset: &AssetId) -> PositionId {
    PositionId::from(format!("{bucket}:{}", asset.0))
}
fn external(purpose: &str, asset: &AssetId) -> PositionId {
    PositionId::from(format!("external:{purpose}:{}", asset.0))
}

fn purpose(kind: &str, subtype: &str) -> Result<&'static str> {
    Ok(match kind {
        "deposit" => "deposit",
        "withdrawal" => "withdrawal",
        "trade" | "spend" | "receive" => "market",
        "staking" => "rewards",
        "earn" if subtype == "reward" || subtype == "rewards" => "rewards",
        "earn" | "transfer" => "transfer",
        "adjustment" => "adjustment",
        _ => anyhow::bail!("unsupported Kraken ledger type {kind}"),
    })
}

fn map(snapshot: &Snapshot, existing: &HashMap<AssetId, u8>) -> Result<Mapped> {
    ensure!(snapshot.version == 1, "unsupported Kraken snapshot version");
    ensure!(
        snapshot.history_cutoff <= snapshot.observed_at,
        "Kraken history cutoff is after observations"
    );
    let mut scales = HashMap::<AssetId, u8>::new();
    let mut register_scale = |code: &str, values: &[&str]| -> Result<()> {
        let (asset, _, metadata_scale) = resolve(snapshot, code)?;
        let required = values
            .iter()
            .map(|s| precision(s))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .max()
            .unwrap_or(0);
        let scale = if let Some(stored) = existing.get(&asset) {
            ensure!(
                *stored >= required,
                "Kraken amount exceeds protected scale for {}",
                asset.0
            );
            *stored
        } else {
            metadata_scale.max(required).max(8)
        };
        scales
            .entry(asset)
            .and_modify(|s| *s = (*s).max(scale))
            .or_insert(scale);
        Ok(())
    };
    for row in snapshot.ledger.values() {
        ensure!(
            row.aclass == "currency",
            "unsupported Kraken ledger asset class"
        );
        register_scale(&row.asset, &[&row.amount, &row.fee, &row.balance])?;
    }
    for (code, b) in &snapshot.balances {
        register_scale(
            code,
            &[&b.balance, &b.credit, &b.credit_used, &b.hold_trade],
        )?;
        ensure!(
            Units::from_decimal(&b.credit, precision(&b.credit)?)?.is_zero()
                && Units::from_decimal(&b.credit_used, precision(&b.credit_used)?)?.is_zero(),
            "Kraken credit balances require an unsupported liability integration"
        );
    }
    let mut positions = BTreeMap::<String, Registration>::new();
    let mut register = |position: &PositionId, asset: &AssetId, role| {
        positions.insert(
            position.0.clone(),
            (position.clone(), asset.clone(), scales[asset], role),
        );
    };
    let mut transactions = Vec::new();
    let mut rows: Vec<_> = snapshot.ledger.iter().collect();
    rows.sort_by_key(|(id, row)| (row.time.clone(), (*id).clone()));
    for (id, row) in rows {
        ensure!(!id.is_empty(), "empty Kraken ledger ID");
        let datetime = time(&row.time)?;
        ensure!(
            datetime <= snapshot.history_cutoff,
            "Kraken ledger entry after fixed cutoff"
        );
        let (asset, bucket, _) = resolve(snapshot, &row.asset)?;
        let amount = Units::from_decimal(&row.amount, scales[&asset])?;
        let fee = Units::from_decimal(&row.fee, scales[&asset])?;
        ensure!(!fee.is_negative(), "negative Kraken ledger fee");
        let counterparty = external(purpose(&row.kind, &row.subtype)?, &asset);
        let position = owned(&bucket, &asset);
        register(&position, &asset, PositionRole::Asset);
        register(&counterparty, &asset, PositionRole::ExternalCounterparty);
        let mut tx = Transaction {
            datetime,
            inputs: vec![],
            outputs: vec![],
        };
        let mut movement = |source: PositionId, target: PositionId, n: Units| {
            if n.is_zero() {
                return;
            }
            tx.inputs.push(TxnEffect {
                position_id: source,
                amount: n.clone(),
                datetime,
            });
            tx.outputs.push(TxnEffect {
                position_id: target,
                amount: n,
                datetime,
            });
        };
        if amount.is_negative() {
            movement(position.clone(), counterparty, amount.abs());
        } else {
            movement(counterparty, position.clone(), amount);
        }
        if !fee.is_zero() {
            let fees = external("fees", &asset);
            register(&fees, &asset, PositionRole::ExternalCounterparty);
            movement(position, fees, fee);
        }
        transactions.push((id.clone(), tx));
    }
    let mut observations = BTreeMap::<String, PositionBalance>::new();
    for (code, b) in &snapshot.balances {
        let (asset, bucket, _) = resolve(snapshot, code)?;
        let position = owned(&bucket, &asset);
        register(&position, &asset, PositionRole::Asset);
        let n = Units::from_decimal(&b.balance, scales[&asset])?;
        ensure!(
            !n.is_negative(),
            "negative Kraken asset balance requires review"
        );
        // Held amounts remain owned; they are not a second asset or a deduction.
        let held = Units::from_decimal(&b.hold_trade, scales[&asset])?;
        ensure!(!held.is_negative(), "negative Kraken hold amount");
        observations
            .entry(position.0.clone())
            .or_insert(PositionBalance {
                position_id: position,
                datetime: snapshot.observed_at,
                amount: 0.into(),
            })
            .amount += n;
    }
    Ok(Mapped {
        positions: positions.into_values().collect(),
        transactions,
        observations: observations.into_values().collect(),
    })
}

pub fn snapshot_hash(snapshot: &Snapshot) -> Result<String> {
    Ok(fingerprint(&serde_json::to_vec(snapshot)?))
}

pub fn import_snapshot(store: &Store, account: &Account, snapshot: &Snapshot) -> Result<String> {
    ensure!(
        account.provider_id.0 == "kraken" && account.id.0 == snapshot.account_id,
        "Kraken account identity/provider conflict"
    );
    let hash = snapshot_hash(snapshot)?;
    if let Some(existing) = store
        .accounts(&account.portfolio_id)?
        .into_iter()
        .find(|a| a.id == account.id)
    {
        ensure!(
            existing.provider_id == account.provider_id,
            "Kraken account provider conflict"
        );
    } else {
        store.save_account(account)?;
    }
    if !store.has_account_imported_file(&account.id, SOURCE, &hash)? {
        let existing = store
            .position_metadata()?
            .into_iter()
            .map(|(_, asset, scale, _)| (asset, scale))
            .collect();
        let mapped = map(snapshot, &existing)?;
        store.save_account_import(
            &account.id,
            SOURCE,
            &hash,
            &mapped.positions,
            &mapped.transactions,
            &mapped.observations,
        )?;
    }
    Ok(hash)
}

/// Independent quantities are evidence, never ledger entries or balance plugs.
pub fn reconcile(
    store: &Store,
    account: &AccountId,
    snapshot: &Snapshot,
    hash: &str,
) -> Result<()> {
    ensure!(
        account.0 == snapshot.account_id,
        "Kraken reconciliation account conflict"
    );
    ensure!(
        snapshot_hash(snapshot)? == hash,
        "Kraken reconciliation snapshot/hash conflict"
    );
    ensure!(
        store.has_account_imported_file(account, SOURCE, hash)?,
        "Kraken snapshot has not been imported"
    );
    let ledger = store.account_balances_at(account, snapshot.history_cutoff)?;
    let observations = store.account_balance_observations(account, SOURCE, hash)?;
    let mut sums = BTreeMap::<String, (Units, Units)>::new();
    for b in ledger {
        sums.entry(b.position_id.0).or_default().0 += b.amount;
    }
    for b in observations {
        sums.entry(b.position_id.0).or_default().1 += b.amount;
    }
    let differences: Vec<_> = sums
        .into_iter()
        .filter(|(_, (ledger, api))| ledger != api)
        .map(|(p, (ledger, api))| {
            format!(
                "{p}: ledger={ledger}, API={api}, difference={} smallest units",
                api.clone() - ledger.clone()
            )
        })
        .collect();
    ensure!(
        differences.is_empty(),
        "Kraken reconciliation failed:\n{}",
        differences.join("\n")
    );
    Ok(())
}

pub fn print_balances(store: &Store, account: &AccountId, cutoff: DateTime<Utc>) -> Result<String> {
    let metadata: BTreeMap<_, _> = store
        .position_metadata()?
        .into_iter()
        .map(|(p, a, s, r)| (p.0, (a, s, r)))
        .collect();
    let mut assets = BTreeMap::<String, Units>::new();
    let mut text = format!(
        "Kraken account {}: transaction balances at {cutoff}\n",
        account.0
    );
    for balance in store.account_balances_at(account, cutoff)? {
        let (asset, scale, role) = &metadata[&balance.position_id.0];
        ensure!(
            *role == PositionRole::Asset,
            "unexpected Kraken liability position"
        );
        text.push_str(&format!(
            "  {}: {} {}\n",
            balance.position_id.0,
            balance.amount.decimal_string(*scale),
            asset.0
        ));
        *assets.entry(asset.0.clone()).or_default() += balance.amount;
    }
    text.push_str("Asset totals (all owned Kraken buckets, counted once):\n");
    for (asset, amount) in assets {
        text.push_str(&format!(
            "  {}: {}\n",
            asset,
            amount.decimal_string(store.asset_scale(&AssetId::str(&asset))?)
        ));
    }
    text.push_str("No Kraken liability integration; credit balances are rejected.\n");
    Ok(text)
}

pub async fn run(
    store: &Store,
    account: &Account,
    path: &Path,
    offline: bool,
    refresh: bool,
) -> Result<()> {
    let snapshot = kraken_client::cached_snapshot(path, &account.id.0, offline, refresh).await?;
    let hash = import_snapshot(store, account, &snapshot)?;
    print!(
        "{}",
        print_balances(store, &account.id, snapshot.history_cutoff)?
    );
    println!(
        "Independent API balances observed at {} (history cutoff {})",
        snapshot.observed_at, snapshot.history_cutoff
    );
    reconcile(store, &account.id, &snapshot, &hash)?;
    println!("All owned Kraken positions reconcile exactly");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_core::{DEFAULT_PORTFOLIO, ProviderId};
    use serde_json::json;
    fn account(id: &str) -> Account {
        Account {
            id: id.into(),
            portfolio_id: DEFAULT_PORTFOLIO.into(),
            provider_id: ProviderId::from("kraken"),
            label: id.into(),
            source_reference: None,
        }
    }
    fn sample(id: &str) -> Snapshot {
        serde_json::from_value(json!({"version":1,"account_id":id,"credential_fingerprint":"synthetic", "history_cutoff":"2024-01-02T00:00:00Z","observed_at":"2024-01-02T00:00:01Z",
            "assets":{"XXBT":{"altname":"XBT","decimals":8,"aclass":"currency"},"ZUSD":{"altname":"USD","decimals":8,"aclass":"currency"}},
            "ledger":{
                "deposit":{"refid":"d","time":1704067200.1234,"type":"deposit","aclass":"currency","asset":"XXBT","amount":"1","fee":"0.001","balance":"0.999"},
                "trade":{"refid":"t","time":1704067300,"type":"trade","aclass":"currency","asset":"XXBT","amount":"-0.1","fee":"0.0001","balance":"0.8989"},
                "trade-usd":{"refid":"t","time":1704067300,"type":"trade","aclass":"currency","asset":"ZUSD","amount":"4000","fee":"1","balance":"3999"},
                "allocation":{"refid":"e","time":1704067400,"type":"earn","subtype":"allocation","aclass":"currency","asset":"XXBT","amount":"-0.2","fee":"0","balance":"0.6989"},
                "allocated":{"refid":"e","time":1704067400,"type":"earn","subtype":"allocation","aclass":"currency","asset":"XXBT.B","amount":"0.2","fee":"0","balance":"0.2"},
                "reward":{"refid":"r","time":1704067500,"type":"earn","subtype":"reward","aclass":"currency","asset":"XXBT.B","amount":"0.01","fee":"0","balance":"0.21"}},
            "balances":{"XXBT":{"balance":"0.6989","hold_trade":"0.1"},"XXBT.B":{"balance":"0.21"},"ZUSD":{"balance":"3999"}}})).unwrap()
    }
    #[test]
    fn maps_fees_earn_aliases_and_scopes_prints_in_mixed_database() -> Result<()> {
        let store = Store::in_memory()?;
        let a = account("alice");
        let b = account("bob");
        let snapshot = sample("alice");
        let mut configured = a.clone();
        configured.label = "My configured Kraken label".into();
        store.save_account(&configured)?;
        let hash = import_snapshot(&store, &a, &snapshot)?;
        assert_eq!(
            store
                .accounts(&a.portfolio_id)?
                .into_iter()
                .find(|account| account.id == a.id)
                .unwrap()
                .label,
            configured.label
        );
        reconcile(&store, &a.id, &snapshot, &hash)?;
        import_snapshot(&store, &b, &sample("bob"))?;
        // Other providers in the ordinary shared DB must never leak into this report.
        for (id, asset) in [("NEXO:BTC", "BTC"), ("BINANCE:spot:USD", "USD")] {
            let position = PositionId::from(id);
            store.save_position_asset(&position, &AssetId::str(asset), PositionRole::Asset)?;
            store.save_transaction(
                id,
                &Transaction {
                    datetime: snapshot.history_cutoff,
                    inputs: vec![],
                    outputs: vec![TxnEffect {
                        position_id: position,
                        amount: 100_000_000_000i64.into(),
                        datetime: snapshot.history_cutoff,
                    }],
                },
            )?;
        }
        let text = print_balances(&store, &a.id, snapshot.history_cutoff)?;
        assert!(text.contains("BTC: 0.90890000"));
        assert!(text.contains("USD: 3999.00000000"));
        assert!(
            !text.contains("bob")
                && !text.contains("external:")
                && !text.contains("NEXO")
                && !text.contains("BINANCE")
        );
        // Fee is separate and charged once; timestamp keeps exact subsecond digits.
        let mapped = map(&snapshot, &HashMap::new())?;
        let (_, tx) = mapped
            .transactions
            .iter()
            .find(|(id, _)| id == "deposit")
            .unwrap();
        assert_eq!(tx.datetime.timestamp_subsec_nanos(), 123400000);
        assert_eq!(tx.inputs.len(), 2);
        assert_eq!(tx.outputs.len(), 2);
        assert!(
            tx.outputs
                .iter()
                .any(|e| e.position_id.0 == "external:fees:BTC" && e.amount == 100000)
        );
        // Repeated exact snapshot and changed full snapshot do not double count IDs.
        assert_eq!(import_snapshot(&store, &a, &snapshot)?, hash);
        let mut changed = snapshot.clone();
        changed.observed_at += chrono::Duration::seconds(1);
        let new_hash = import_snapshot(&store, &a, &changed)?;
        reconcile(&store, &a.id, &changed, &new_hash)?;
        assert_eq!(
            print_balances(&store, &a.id, snapshot.history_cutoff)?,
            text
        );
        Ok(())
    }
    #[tokio::test]
    async fn cached_replay_reopen_and_mismatch_survive_without_credentials() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("snapshot.json");
        let db = dir.path().join("ledger.duckdb");
        let mut snapshot = sample("alice");
        snapshot.ledger.remove("trade-usd");
        kraken_client::write_json(&path, &snapshot)?;
        let loaded = kraken_client::cached_snapshot(&path, "alice", true, false).await?;
        let hash;
        {
            let store = Store::open(&db)?;
            hash = import_snapshot(&store, &account("alice"), &loaded)?;
            assert!(reconcile(&store, &account("alice").id, &loaded, &hash).is_err());
        }
        let store = Store::open(&db)?;
        assert!(
            reconcile(&store, &account("alice").id, &loaded, &hash)
                .unwrap_err()
                .to_string()
                .contains("API=399900000000")
        );
        assert!(
            kraken_client::cached_snapshot(&path, "bob", true, false)
                .await
                .is_err()
        );
        assert!(
            kraken_client::cached_snapshot(&dir.path().join("absent.json"), "alice", true, false)
                .await
                .is_err()
        );
        Ok(())
    }
    #[test]
    fn rejects_unknown_assets_types_credit_and_precision_atomically() -> Result<()> {
        for variant in 0..7 {
            let store = Store::in_memory()?;
            let a = account("alice");
            let mut snapshot = sample("alice");
            match variant {
                0 => snapshot.ledger.get_mut("deposit").unwrap().asset = "XXBT.T".into(),
                1 => snapshot.ledger.get_mut("deposit").unwrap().kind = "margin".into(),
                2 => snapshot.balances.get_mut("XXBT").unwrap().credit_used = "1".into(),
                3 => snapshot.ledger.get_mut("deposit").unwrap().fee = "-1".into(),
                4 => {
                    store.save_asset_scale(&AssetId::str("BTC"), 8)?;
                    snapshot.ledger.get_mut("deposit").unwrap().amount = "0.000000001".into();
                }
                5 => {
                    snapshot.assets.insert(
                        "ambiguous".into(),
                        kraken_client::AssetInfo {
                            altname: "XXBT".into(),
                            decimals: 8,
                            aclass: "currency".into(),
                        },
                    );
                }
                _ => snapshot.assets.clear(),
            }
            assert!(import_snapshot(&store, &a, &snapshot).is_err());
            assert!(
                store
                    .account_balances_at(&a.id, snapshot.history_cutoff)?
                    .is_empty()
            );
            assert!(!store.has_account_imported_file(&a.id, SOURCE, &snapshot_hash(&snapshot)?)?);
        }
        Ok(())
    }
    #[test]
    fn historical_closed_positions_and_large_exact_quantities() -> Result<()> {
        let store = Store::in_memory()?;
        let a = account("alice");
        let mut snapshot = sample("alice");
        snapshot.ledger.retain(|id, _| id == "deposit");
        snapshot.balances.clear();
        assert!(
            reconcile(
                &store,
                &a.id,
                &snapshot,
                &import_snapshot(&store, &a, &snapshot)?
            )
            .is_err()
        );
        let mut closed = snapshot.clone();
        let mut withdrawal = closed.ledger["deposit"].clone();
        withdrawal.time = "1704067300".into();
        withdrawal.kind = "withdrawal".into();
        withdrawal.amount = "-0.999".into();
        withdrawal.fee = "0".into();
        withdrawal.balance = "0".into();
        closed.ledger.insert("withdrawal".into(), withdrawal);
        let closed_store = Store::in_memory()?;
        let hash = import_snapshot(&closed_store, &a, &closed)?;
        reconcile(&closed_store, &a.id, &closed, &hash)?;
        let mut current_only = sample("alice");
        current_only.ledger.clear();
        let current_store = Store::in_memory()?;
        let hash = import_snapshot(&current_store, &a, &current_only)?;
        assert!(reconcile(&current_store, &a.id, &current_only, &hash).is_err());
        let n = "123456789012345678901234567890.123456789012345678";
        snapshot.ledger.get_mut("deposit").unwrap().amount = n.into();
        snapshot.ledger.get_mut("deposit").unwrap().fee = "0".into();
        snapshot.ledger.get_mut("deposit").unwrap().balance = n.into();
        snapshot
            .balances
            .insert("XXBT".into(), serde_json::from_value(json!({"balance":n}))?);
        let fresh = Store::in_memory()?;
        let hash = import_snapshot(&fresh, &a, &snapshot)?;
        reconcile(&fresh, &a.id, &snapshot, &hash)?;
        assert!(print_balances(&fresh, &a.id, snapshot.history_cutoff)?.contains(n));
        Ok(())
    }
    #[tokio::test]
    #[ignore = "private Kraken snapshot; first run requires read-only API credentials"]
    async fn latest_amount_after_cached_account() -> Result<()> {
        let id = std::env::var("KRAKEN_ACCOUNT_ID").unwrap_or_else(|_| "kraken:primary".into());
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/test-data/kraken");
        std::fs::create_dir_all(&dir)?;
        let key = fingerprint(id.as_bytes());
        let path = dir.join(format!("{key}.json"));
        let store = Store::open(dir.join("kraken.duckdb"))?;
        run(&store, &account(&id), &path, false, false).await
    }
}
