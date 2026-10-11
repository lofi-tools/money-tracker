//! Chain-qualified EVM wallet effects; RPC observations never create movements.
use anyhow::{Context, Result, ensure};
use chrono::DateTime;
use evm_client::{Snapshot, TRANSFER, address, hex, integer, peer, text, topic_address};
use lib_core::{
    Account, AccountId, AssetId, PositionId, PositionRole, Store, Transaction, TxnEffect, Units,
};
use num_bigint::BigUint;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
pub const SOURCE: &str = "evm-wallet-v1";

/// Reviewed by the project against verified source and deployed runtime, not ticker heuristics.
pub const SAFE_PROMO: &str = "0x31a240648e2baf4f9f17225987f6f53fceb1699a";
pub fn exclude_safe_promo(store: &Store, account: &Account) -> Result<()> {
    ensure!(
        account.provider_id.0 == "evm" && account.id.0.starts_with("eip155:1:"),
        "SafePromo review applies only to Ethereum accounts"
    );
    store.review_asset_tracking(&account.id, &asset(1, Some(SAFE_PROMO)), true,
        "Reviewed spam: constant balanceOf=777 for every address; transfers disabled; no trustworthy decimals",
        "Project review, 2026-10-11; https://eth.blockscout.com/api/v2/smart-contracts/0x31a240648e2baf4f9f17225987f6f53fceb1699a; runtime SHA256 87b628c1cc1a0456d1ffb1b84ef5832ad1640dcb91f631a893cd6ac180be754f; source SHA256 278d2cb99679df3e3bdb3daad2a889a598259daed8898aa82a03dd25bbf6fc8c")
}

pub fn account(chain: u64, wallet: &str, portfolio: &str) -> Result<Account> {
    let wallet = address(wallet)?;
    evm_client::defaults(chain)?;
    Ok(Account {
        id: format!("eip155:{chain}:{wallet}").into(),
        portfolio_id: portfolio.into(),
        provider_id: "evm".into(),
        label: format!("EVM wallet (chain {chain})"),
        source_reference: Some(format!("eip155:{chain}:{wallet}")),
    })
}
pub fn register(store: &Store, a: &Account) -> Result<()> {
    let existing = store
        .accounts(&a.portfolio_id)?
        .into_iter()
        .find(|v| v.id == a.id);
    if let Some(v) = existing {
        ensure!(
            v.provider_id == a.provider_id && v.source_reference == a.source_reference,
            "EVM account binding conflict"
        );
    } else {
        store.save_account(a)?;
    }
    Ok(())
}
fn asset(chain: u64, contract: Option<&str>) -> AssetId {
    AssetId::str(&format!(
        "eip155:{chain}/{}",
        contract
            .map(|c| format!("erc20:{c}"))
            .unwrap_or_else(|| "native".into())
    ))
}
fn owned(asset: &AssetId) -> PositionId {
    format!("wallet:{}", asset.0).into()
}
fn movement(
    tx: &mut Transaction,
    asset: &AssetId,
    from: &str,
    to: &str,
    wallet: &str,
    n: Units,
    purpose: &str,
) {
    if n.is_zero() {
        return;
    }
    let effect = |position_id| TxnEffect {
        position_id,
        amount: n.clone(),
        datetime: tx.datetime,
    };
    if from == wallet {
        tx.inputs.push(effect(owned(asset)));
    } else if to == wallet {
        tx.inputs
            .push(effect(format!("external:{purpose}:{}", asset.0).into()));
    }
    if to == wallet {
        tx.outputs.push(effect(owned(asset)));
    } else if from == wallet {
        tx.outputs
            .push(effect(format!("external:{purpose}:{}", asset.0).into()));
    }
}
fn product(a: Units, b: Units) -> Result<Units> {
    let a =
        BigUint::parse_bytes(a.to_string().as_bytes(), 10).context("invalid fee multiplicand")?;
    let b =
        BigUint::parse_bytes(b.to_string().as_bytes(), 10).context("invalid fee multiplicand")?;
    (a * b).to_string().parse()
}
fn quantity(v: &serde_json::Value) -> Result<Units> {
    let s = v.as_str().context("missing exact EVM amount")?;
    let n: Units = if s.starts_with("0x") {
        hex(s)?
    } else {
        s.parse()?
    };
    ensure!(!n.is_negative(), "negative EVM source quantity");
    Ok(n)
}

fn receipt_status(s: &Snapshot, h: &str) -> Result<u64> {
    let receipt = &s.receipts[h];
    if !receipt["status"].is_null() {
        let status = integer(&receipt["status"])?;
        ensure!(status <= 1, "unknown receipt status");
        return Ok(status);
    }
    // Ethereum pre-Byzantium receipts contain a post-state root, not status.
    ensure!(
        s.chain == 1
            && integer(&receipt["blockNumber"])? < 4_370_000
            && receipt["root"].is_string(),
        "missing receipt status outside Ethereum pre-Byzantium history"
    );
    evm_client::hash(text(receipt, "root")?)?;
    Ok(u64::from(*s.transaction_outcomes.get(h).context(
        "pre-Byzantium transaction needs explicit indexer outcome evidence",
    )?))
}
pub fn import(store: &Store, a: &Account, s: &Snapshot) -> Result<String> {
    ensure!(
        s.version == 1 && account(s.chain, &s.address, &a.portfolio_id.0)?.id == a.id,
        "EVM snapshot/account mismatch"
    );
    evm_client::hash(&s.block_hash)?;
    register(store, a)?;
    let payload = serde_json::to_string(s)?;
    let digest = evm_client::fingerprint(payload.as_bytes());
    if store.has_account_imported_file(&a.id, SOURCE, &digest)? {
        store.save_account_source_snapshot(&a.id, SOURCE, &digest, &payload)?;
        return Ok(digest);
    }
    let native = asset(s.chain, None);
    let mut registrations = vec![(owned(&native), native.clone(), 18, PositionRole::Asset)];
    let mut observations = vec![lib_core::PositionBalance {
        position_id: owned(&native),
        datetime: s.timestamp,
        amount: s.native_balance.clone(),
    }];
    ensure!(
        !s.native_balance.is_negative(),
        "negative native observation"
    );
    for (contract, t) in &s.tokens {
        ensure!(
            address(contract)? == *contract && !t.balance.is_negative(),
            "invalid token identity/balance"
        );
        let id = asset(s.chain, Some(contract));
        registrations.push((owned(&id), id.clone(), t.scale, PositionRole::Asset));
        observations.push(lib_core::PositionBalance {
            position_id: owned(&id),
            datetime: s.timestamp,
            amount: t.balance.clone(),
        });
    }
    ensure!(
        s.transactions.len() == s.receipts.len(),
        "incomplete EVM receipts"
    );
    let mut transactions = BTreeMap::new();
    for (h, row) in &s.transactions {
        ensure!(
            evm_client::hash(h)? == *h && evm_client::hash(text(row, "hash")?)? == *h,
            "invalid EVM transaction identity"
        );
        let receipt = s.receipts.get(h).context("missing receipt")?;
        ensure!(
            evm_client::hash(text(receipt, "transactionHash")?)? == *h
                && row["blockHash"] == receipt["blockHash"],
            "receipt identity conflict"
        );
        ensure!(
            integer(&row["blockNumber"])? == integer(&receipt["blockNumber"])?
                && integer(&row["blockNumber"])? <= s.block,
            "EVM receipt beyond cutoff"
        );
        let dt = DateTime::from_timestamp(integer(&row["timestamp"])?.try_into()?, 0)
            .context("invalid transaction time")?;
        ensure!(dt <= s.timestamp, "transaction timestamp beyond cutoff");
        let mut tx = Transaction {
            datetime: dt,
            inputs: vec![],
            outputs: vec![],
        };
        let from = address(text(row, "from")?)?;
        let to = row["to"]
            .as_str()
            .map(address)
            .transpose()?
            .or_else(|| {
                receipt["contractAddress"]
                    .as_str()
                    .and_then(|v| address(v).ok())
            })
            .unwrap_or_default();
        let status = receipt_status(s, h)?;
        if status == 1 {
            movement(
                &mut tx,
                &native,
                &from,
                &to,
                &s.address,
                quantity(&row["value"])?,
                "transfer",
            );
        }
        if from == s.address {
            let mut fee = product(
                quantity(&receipt["gasUsed"])?,
                quantity(&receipt["effectiveGasPrice"])?,
            )?;
            if let Some(l1) = receipt.get("l1Fee").filter(|v| !v.is_null()) {
                fee += quantity(l1)?;
            }
            movement(&mut tx, &native, &s.address, "gas", &s.address, fee, "gas");
        }
        let mut indices = BTreeSet::new();
        for log in receipt["logs"].as_array().context("missing receipt logs")? {
            ensure!(
                !log["removed"].as_bool().unwrap_or(false),
                "removed EVM log"
            );
            ensure!(
                log["transactionHash"] == receipt["transactionHash"]
                    && log["blockHash"] == receipt["blockHash"],
                "log identity conflict"
            );
            ensure!(
                indices.insert(integer(&log["logIndex"])?),
                "duplicate receipt log index"
            );
            let topics = log["topics"].as_array().context("missing log topics")?;
            if status != 1 || topics.len() != 3 || topics[0] != TRANSFER {
                continue;
            }
            let from = topic_address(&topics[1])?;
            let to = topic_address(&topics[2])?;
            if from != s.address && to != s.address {
                continue;
            }
            let contract = address(text(log, "address")?)?;
            if s.unresolved_tokens.contains_key(&contract) {
                continue;
            }
            ensure!(
                s.tokens.contains_key(&contract),
                "undiscovered ERC-20 contract"
            );
            movement(
                &mut tx,
                &asset(s.chain, Some(&contract)),
                &from,
                &to,
                &s.address,
                hex(text(log, "data")?)?,
                "transfer",
            );
        }
        transactions.insert(h.clone(), tx);
    }
    let mut seen = BTreeSet::new();
    for row in &s.internals {
        let h = evm_client::hash(text(row, "transaction_hash")?)?;
        let index = row
            .get("index")
            .filter(|v| !v.is_null())
            .context("missing trace index")?
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| row["index"].to_string());
        ensure!(
            seen.insert((h.clone(), index.clone())),
            "duplicate internal trace identity"
        );
        let parent = s
            .transactions
            .get(&h)
            .context("internal trace missing parent")?;
        let receipt = &s.receipts[&h];
        if receipt_status(s, &h)? == 0
            || row.get("error").is_some_and(|v| !v.is_null() && v != "")
            || row["success"] == false
        {
            continue;
        }
        ensure!(
            integer(&row["block_number"])? == integer(&receipt["blockNumber"])?,
            "trace block mismatch"
        );
        let n = quantity(&row["value"])?;
        if n == 0 {
            continue; // zero-value traces do not require a valid recipient
        }
        let from = peer(&row["from"])?;
        let to = peer(&row["to"]).or_else(|_| peer(&row["created_contract"]))?;
        // Blockscout may include a root call identical to normal value movement.
        if index == "0"
            && from == address(text(parent, "from")?)?
            && parent["to"]
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case(&to))
            && n == quantity(&parent["value"])?
        {
            continue;
        }
        movement(
            transactions.get_mut(&h).unwrap(),
            &native,
            &from,
            &to,
            &s.address,
            n,
            "internal",
        );
    }
    // Register external purpose legs at the same immutable asset scale.
    let lookup = registrations
        .iter()
        .map(|(p, a, s, _)| (p.0.clone(), (a.clone(), *s)))
        .collect::<BTreeMap<_, _>>();
    let mut counterparts = BTreeMap::new();
    for tx in transactions.values() {
        for e in tx.inputs.iter().chain(&tx.outputs) {
            if e.position_id.0.starts_with("external:") {
                let id = e
                    .position_id
                    .0
                    .splitn(3, ':')
                    .nth(2)
                    .context("invalid external position")?;
                let a = AssetId::str(id);
                let (_, scale) = lookup.get(&owned(&a).0).context("unregistered asset")?;
                counterparts.insert(e.position_id.0.clone(), (a, *scale));
            }
        }
    }
    registrations.extend(
        counterparts
            .into_iter()
            .map(|(p, (a, s))| (p.into(), a, s, PositionRole::ExternalCounterparty)),
    );
    store.save_account_import(
        &a.id,
        SOURCE,
        &digest,
        &registrations,
        &transactions
            .into_iter()
            .map(|(h, t)| (format!("transaction:{h}"), t))
            .collect::<Vec<_>>(),
        &observations,
    )?;
    store.save_account_source_snapshot(&a.id, SOURCE, &digest, &payload)?;
    Ok(digest)
}
pub fn reconcile(store: &Store, a: &AccountId, s: &Snapshot, digest: &str) -> Result<String> {
    ensure!(
        digest == evm_client::fingerprint(&serde_json::to_vec(s)?)
            && store.has_account_imported_file(a, SOURCE, digest)?,
        "unimported/conflicting EVM snapshot"
    );
    ensure!(
        a.0 == format!("eip155:{}:{}", s.chain, s.address),
        "EVM account conflict"
    );
    let mut sums = BTreeMap::<String, (Units, Units)>::new();
    for b in store.account_balances_at(a, s.timestamp)? {
        sums.entry(b.position_id.0).or_default().0 += b.amount;
    }
    for b in store.account_balance_observations(a, SOURCE, digest)? {
        if !store.position_is_tracked(&b.position_id)? {
            continue;
        }
        sums.entry(b.position_id.0).or_default().1 += b.amount;
    }
    let mut report = format!(
        "EVM chain {} at block {} ({})\n",
        s.chain, s.block, s.block_hash
    );
    for contract in s.tokens.keys().chain(s.unresolved_tokens.keys()) {
        if store.asset_is_excluded(a, &asset(s.chain, Some(contract)))? {
            report.push_str(&format!("Excluded reviewed asset {contract}: raw history preserved; outside tracked reconciliation scope\n"));
        }
    }
    let mut mismatch = false;
    for (p, (ledger, rpc)) in sums {
        report.push_str(&format!(
            "{}: ledger={} RPC={} difference={} smallest units\n",
            p,
            ledger,
            rpc,
            rpc.clone() - ledger.clone()
        ));
        mismatch |= ledger != rpc;
    }
    ensure!(
        !mismatch,
        "EVM reconciliation failed; incomplete history, rebasing, unsupported effects or reorg:\n{report}"
    );
    ensure!(
        !(s.chain == 42161
            && s.transactions.iter().any(|(h, tx)| tx["from"]
                .as_str()
                .is_some_and(|from| from.eq_ignore_ascii_case(&s.address))
                && integer(&s.receipts[h]["blockNumber"]).is_ok_and(|block| block < 22_207_818))),
        "Arbitrum Classic feeStats mapping is required for exact fee coverage, even if aggregate quantities match:\n{report}"
    );
    let mut unresolved = 0;
    for contract in s.unresolved_tokens.keys() {
        if !store.asset_is_excluded(a, &asset(s.chain, Some(contract)))? {
            unresolved += 1;
        }
    }
    ensure!(
        unresolved == 0,
        "EVM coverage is partial: {} unresolved token contracts; verified position comparison follows:\n{report}",
        unresolved
    );
    Ok(report)
}
pub async fn run(
    store: &Store,
    a: &Account,
    path: &Path,
    rpc: Option<&str>,
    indexer: Option<&str>,
    confirmations: u64,
    offline: bool,
    refresh: bool,
) -> Result<()> {
    let identity = a
        .source_reference
        .as_deref()
        .context("missing EVM address")?;
    let parts = identity.split(':').collect::<Vec<_>>();
    ensure!(parts.len() == 3, "invalid EVM account reference");
    let chain = parts[1].parse()?;
    let wallet = parts[2];
    let snapshot = evm_client::cached(
        path,
        chain,
        wallet,
        rpc,
        indexer,
        confirmations,
        offline,
        refresh,
    )
    .await?;
    let digest = import(store, a, &snapshot)?;
    for (contract, reason) in &snapshot.unresolved_tokens {
        if store.asset_is_excluded(&a.id, &asset(snapshot.chain, Some(contract)))? {
            eprintln!(
                "Reviewed exclusion {contract}: financial mapping withheld; raw history preserved"
            );
        } else {
            eprintln!("Unresolved token {contract}: {reason}");
        }
    }
    for warning in &snapshot.warnings {
        eprintln!("Warning: {warning}");
    }
    print!("{}", reconcile(store, &a.id, &snapshot, &digest)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn wallet() -> String {
        format!("0x{}", "a".repeat(40))
    }
    fn fixture(chain: u64, w: &str) -> Snapshot {
        let peer = format!("0x{}", "b".repeat(40));
        let contract = format!("0x{}", "c".repeat(40));
        let block_hash = format!("0x{}", "d".repeat(64));
        let h = format!("0x{}", "1".repeat(64));
        let failed = format!("0x{}", "2".repeat(64));
        let topic = |a: &str| format!("0x{:0>64}", &a[2..]);
        let token_amount = hex(&format!("0x1{}", "0".repeat(40))).unwrap();
        let log = json!({"address":contract,"topics":[TRANSFER,topic(&peer),topic(w)],"data":format!("0x1{}","0".repeat(40)),"logIndex":"0x3","transactionHash":h,"blockHash":block_hash});
        let tx = |hash: &str, from: &str, to: &str, value: &str| json!({"hash":hash,"from":from,"to":to,"value":value,"blockNumber":"0xa","blockHash":block_hash,"timestamp":"0x6553f100"});
        let receipt = |hash: &str, status: &str, logs: serde_json::Value| json!({"transactionHash":hash,"blockHash":block_hash,"blockNumber":"0xa","gasUsed":"0x2","effectiveGasPrice":"0x3","status":status,"logs":logs});
        Snapshot {
            version: 1,
            chain,
            address: w.into(),
            block: 10,
            block_hash: block_hash.clone(),
            timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
            native_balance: 15.into(),
            tokens: BTreeMap::from([(
                contract,
                evm_client::Token {
                    symbol: "SAME".into(),
                    scale: 18,
                    balance: token_amount,
                },
            )]),
            transactions: BTreeMap::from([
                (h.clone(), tx(&h, &peer, w, "0x10")),
                (failed.clone(), tx(&failed, w, &peer, "0xffff")),
            ]),
            receipts: BTreeMap::from([
                (h.clone(), receipt(&h, "0x1", json!([log]))),
                (failed.clone(), receipt(&failed, "0x0", json!([]))),
            ]),
            transaction_outcomes: BTreeMap::new(),
            internals: vec![
                json!({"transaction_hash":h,"block_number":10,"index":0,"from":{"hash":peer},"to":{"hash":w},"value":"16","success":true}),
                json!({"transaction_hash":h,"block_number":10,"index":1,"from":{"hash":peer},"to":{"hash":w},"value":"5","success":true}),
                json!({"transaction_hash":failed,"block_number":10,"index":1,"from":{"hash":peer},"to":{"hash":w},"value":"99999","success":true}),
            ],
            warnings: vec![],
            unresolved_tokens: BTreeMap::new(),
            token_evidence: BTreeMap::new(),
        }
    }
    #[test]
    fn reviewed_unknown_spam_keeps_raw_logs_and_strict_tracked_reconciliation() -> Result<()> {
        let store = Store::in_memory()?;
        let w = wallet();
        let a = account(1, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let mut s = fixture(1, &w);
        let h = s.receipts.keys().next().unwrap().clone();
        let mut log = s.receipts[&h]["logs"][0].clone();
        log["address"] = serde_json::json!(SAFE_PROMO);
        log["logIndex"] = serde_json::json!("0x4");
        s.receipts.get_mut(&h).unwrap()["logs"]
            .as_array_mut()
            .unwrap()
            .push(log);
        s.unresolved_tokens
            .insert(SAFE_PROMO.into(), "no trustworthy scale".into());
        let d = import(&store, &a, &s)?;
        assert!(reconcile(&store, &a.id, &s, &d).is_err());
        exclude_safe_promo(&store, &a)?;
        assert!(reconcile(&store, &a.id, &s, &d)?.contains("Excluded reviewed asset"));
        assert!(
            store
                .account_source_snapshot(&a.id, SOURCE, &d)?
                .unwrap()
                .contains(SAFE_PROMO)
        );
        assert!(store.asset_scale(&asset(1, Some(SAFE_PROMO))).is_err());
        assert_eq!(import(&store, &a, &s)?, d);
        let mut bad = s.clone();
        bad.native_balance = 16.into();
        let db = import(&store, &a, &bad)?;
        assert!(
            reconcile(&store, &a.id, &bad, &db)
                .unwrap_err()
                .to_string()
                .contains("reconciliation failed")
        );
        store.review_asset_tracking(
            &a.id,
            &asset(1, Some(SAFE_PROMO)),
            false,
            "restore",
            "user review",
        )?;
        assert!(reconcile(&store, &a.id, &s, &d).is_err());
        Ok(())
    }

    #[test]
    fn resolved_exclusion_skips_both_sides_but_preserves_effects() -> Result<()> {
        let store = Store::in_memory()?;
        let w = wallet();
        let a = account(1, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let mut s = fixture(1, &w);
        let contract = s.tokens.keys().next().unwrap().clone();
        s.tokens.get_mut(&contract).unwrap().balance = 0.into();
        let d = import(&store, &a, &s)?;
        assert!(reconcile(&store, &a.id, &s, &d).is_err());
        store.review_asset_tracking(
            &a.id,
            &asset(1, Some(&contract)),
            true,
            "fixture review",
            "source",
        )?;
        reconcile(&store, &a.id, &s, &d)?;
        assert_eq!(store.account_balances_at(&a.id, s.timestamp)?.len(), 1);
        assert_eq!(
            store.account_audit_balances_at(&a.id, s.timestamp)?.len(),
            2
        );
        assert_eq!(
            store.account_balance_observations(&a.id, SOURCE, &d)?.len(),
            2
        );
        assert_eq!(store.account_balances_at(&a.id, s.timestamp)?[0].amount, 15);
        Ok(())
    }

    #[test]
    fn exact_receipt_logs_failed_gas_internal_dedup_and_mixed_accounts() -> Result<()> {
        let store = Store::in_memory()?;
        let w = wallet();
        let a = account(1, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let s = fixture(1, &w);
        let d = import(&store, &a, &s)?;
        let result = reconcile(&store, &a.id, &s, &d)?;
        assert!(result.contains("ledger=15 RPC=15"));
        assert_eq!(import(&store, &a, &s)?, d);
        reconcile(&store, &a.id, &s, &d)?;
        // Identical transaction hashes on another chain/wallet must never collide.
        let b = account(137, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let sb = fixture(137, &w);
        let db = import(&store, &b, &sb)?;
        reconcile(&store, &b.id, &sb, &db)?;
        let w2 = format!("0x{}", "e".repeat(40));
        let c = account(1, &w2, lib_core::DEFAULT_PORTFOLIO)?;
        let sc = fixture(1, &w2);
        let dc = import(&store, &c, &sc)?;
        reconcile(&store, &c.id, &sc, &dc)?;
        assert_eq!(store.account_balances_at(&a.id, s.timestamp)?.len(), 2);
        // Ticker equality never merges token locators across chains.
        assert_ne!(
            asset(1, s.tokens.keys().next().map(String::as_str)),
            asset(137, s.tokens.keys().next().map(String::as_str))
        );
        Ok(())
    }
    #[test]
    fn mismatch_is_persisted_and_bad_events_roll_back() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("evm.duckdb");
        let w = wallet();
        let a = account(1, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let mut s = fixture(1, &w);
        s.native_balance = 99.into();
        let d;
        {
            let store = Store::open(&path)?;
            d = import(&store, &a, &s)?;
            assert!(reconcile(&store, &a.id, &s, &d).is_err());
        }
        {
            let store = Store::open(&path)?;
            assert!(reconcile(&store, &a.id, &s, &d).is_err());
        }
        let store = Store::in_memory()?;
        let mut bad = fixture(1, &w);
        let receipt = bad
            .receipts
            .values_mut()
            .find(|r| r["status"] == "0x1")
            .unwrap();
        let log = receipt["logs"][0].clone();
        receipt["logs"].as_array_mut().unwrap().push(log);
        assert!(import(&store, &a, &bad).is_err());
        assert!(store.account_balances_at(&a.id, bad.timestamp)?.is_empty());
        assert!(!store.has_account_imported_file(
            &a.id,
            SOURCE,
            &evm_client::fingerprint(&serde_json::to_vec(&bad)?)
        )?);
        let good = fixture(1, &w);
        let d = import(&store, &a, &good)?;
        reconcile(&store, &a.id, &good, &d)?;
        let mut precision = good.clone();
        precision.tokens.values_mut().next().unwrap().scale = 8;
        assert!(import(&store, &a, &precision).is_err());
        reconcile(&store, &a.id, &good, &d)?;
        Ok(())
    }
    #[test]
    fn unresolved_contract_withholds_success_even_when_known_balances_match() -> Result<()> {
        let store = Store::in_memory()?;
        let w = wallet();
        let a = account(1, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let mut s = fixture(1, &w);
        let contract = s.tokens.keys().next().unwrap().clone();
        s.tokens.remove(&contract);
        s.unresolved_tokens
            .insert(contract, "missing verified decimals".into());
        let digest = import(&store, &a, &s)?;
        let error = reconcile(&store, &a.id, &s, &digest)
            .unwrap_err()
            .to_string();
        assert!(error.contains("coverage is partial"));
        assert!(error.contains("ledger=15 RPC=15"));
        Ok(())
    }
    #[test]
    fn classic_fee_coverage_is_not_accepted_just_because_balances_match() -> Result<()> {
        let store = Store::in_memory()?;
        let w = wallet();
        let a = account(42161, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let s = fixture(42161, &w);
        let digest = import(&store, &a, &s)?;
        let error = reconcile(&store, &a.id, &s, &digest)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Classic feeStats mapping"));
        assert!(error.contains("ledger=15 RPC=15"));
        Ok(())
    }
    #[test]
    fn absent_contract_code_does_not_create_a_balancing_writeoff() -> Result<()> {
        let store = Store::in_memory()?;
        let w = wallet();
        let a = account(1, &w, lib_core::DEFAULT_PORTFOLIO)?;
        let mut s = fixture(1, &w);
        let contract = s.tokens.keys().next().unwrap().clone();
        s.tokens.get_mut(&contract).unwrap().balance = 0.into();
        s.tokens.get_mut(&contract).unwrap().scale = 0;
        s.token_evidence.insert(
            contract,
            json!({"code_response":"0x","historical_decimals":{"block":10,"response":"0x0"}}),
        );
        let digest = import(&store, &a, &s)?;
        let error = reconcile(&store, &a.id, &s, &digest)
            .unwrap_err()
            .to_string();
        assert!(error.contains("reconciliation failed"));
        assert!(error.contains("RPC=0"));
        assert_eq!(store.account_balances_at(&a.id, s.timestamp)?.len(), 2);
        Ok(())
    }
    #[test]
    fn pre_byzantium_status_requires_evidenced_outcome() -> Result<()> {
        let mut s = fixture(1, &wallet());
        let h = s.receipts.keys().next().unwrap().clone();
        s.receipts
            .get_mut(&h)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("status");
        s.receipts.get_mut(&h).unwrap()["root"] = json!(format!("0x{}", "d".repeat(64)));
        assert!(receipt_status(&s, &h).is_err());
        s.transaction_outcomes.insert(h.clone(), false);
        assert_eq!(receipt_status(&s, &h)?, 0);
        s.transaction_outcomes.insert(h.clone(), true);
        assert_eq!(receipt_status(&s, &h)?, 1);
        s.chain = 137;
        assert!(receipt_status(&s, &h).is_err());
        Ok(())
    }
    #[test]
    fn saved_wallet_reference_and_label_survive_registration() -> Result<()> {
        let store = Store::in_memory()?;
        let a = account(1, &wallet(), lib_core::DEFAULT_PORTFOLIO)?;
        let mut custom = a.clone();
        custom.label = "Family wallet".into();
        store.save_account(&custom)?;
        register(&store, &a)?;
        let saved = store
            .accounts(&a.portfolio_id)?
            .into_iter()
            .find(|v| v.id == a.id)
            .unwrap();
        assert_eq!(saved.label, "Family wallet");
        assert_eq!(saved.source_reference, a.source_reference);
        Ok(())
    }
}

#[cfg(test)]
mod private_tests {
    use super::*;
    #[tokio::test]
    #[ignore = "uses a registered wallet from the local DB and fetches public chain data if not cached"]
    async fn latest_amount_after_cached_wallet() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let store = Store::open(root.join(".cache/data/portfolio.duckdb"))?;
        let id = std::env::var("EVM_ACCOUNT_ID").ok();
        let a = store
            .accounts(&lib_core::DEFAULT_PORTFOLIO.into())?
            .into_iter()
            .find(|a| {
                a.provider_id.0 == "evm"
                    && id
                        .as_ref()
                        .map(|id| &a.id.0 == id)
                        .unwrap_or_else(|| a.id.0.starts_with("eip155:1:"))
            })
            .context("register a wallet first with evm --address ADDRESS --register-only")?;
        let path = root.join(".cache/evm").join(format!(
            "{}.json",
            evm_client::fingerprint(a.id.0.as_bytes())
        ));
        run(
            &store,
            &a,
            &path,
            std::env::var("EVM_RPC_URL").ok().as_deref(),
            std::env::var("EVM_INDEXER_URL").ok().as_deref(),
            64,
            false,
            false,
        )
        .await
    }
}
