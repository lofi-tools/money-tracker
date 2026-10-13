//! Compare cached Nexo balances to a fresh mapping without changing the ledger.
use lib_app::adapters::nexo::NexoSvc;
use lib_core::{AccountId, Store, Units, traits::IsProvider};
use nexo_csv::NexoCsv;
use std::collections::BTreeMap;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        (2..=3).contains(&args.len()),
        "usage: nexo-ledger-check DB CSV [BACKUP_DB]"
    );
    let store = Store::open(&args[0])?;
    if let Some(path) = args.get(2) {
        let backup = Store::open(path)?;
        let at = chrono::Utc::now();
        let mut unchanged = 0;
        for portfolio in backup.portfolios()? {
            for account in backup.accounts(&portfolio.id)? {
                anyhow::ensure!(
                    store.accounts(&portfolio.id)?.contains(&account),
                    "account metadata changed"
                );
                anyhow::ensure!(
                    store.account_jurisdiction(&account.id)?
                        == backup.account_jurisdiction(&account.id)?,
                    "jurisdiction changed"
                );
                if account.provider_id.0.eq_ignore_ascii_case("nexo") {
                    continue;
                }
                anyhow::ensure!(
                    store.account_position_metadata(&account.id)?
                        == backup.account_position_metadata(&account.id)?,
                    "unrelated position metadata changed"
                );
                anyhow::ensure!(
                    store.account_audit_balances_at(&account.id, at)?
                        == backup.account_audit_balances_at(&account.id, at)?,
                    "unrelated provider balance changed"
                );
                unchanged += 1;
            }
        }
        println!(
            "Preserved account metadata/jurisdictions and {unchanged} unrelated provider accounts."
        );
    }
    let rows = NexoCsv::from_file(&args[1])?;
    let cutoff = rows.iter().map(|r| r.date_time_utc).max().unwrap();
    let mapped = NexoSvc::from_path(&args[1]).fetch_all_txn_data().await?;
    let mut fresh: BTreeMap<String, Units> = BTreeMap::new();
    for tx in mapped.transactions {
        for e in tx.inputs {
            *fresh.entry(e.position_id.0).or_default() -= e.amount;
        }
        for e in tx.outputs {
            *fresh.entry(e.position_id.0).or_default() += e.amount;
        }
    }
    println!("CSV cutoff {cutoff}");
    let metadata = store.account_position_metadata(&AccountId::from("legacy:nexo"))?;
    let balances = store.account_balances_at(&AccountId::from("legacy:nexo"), cutoff)?;
    let mut cached: BTreeMap<String, Units> = balances
        .iter()
        .map(|b| (b.position_id.0.clone(), b.amount.clone()))
        .collect();
    for (id, _) in &fresh {
        cached.entry(id.clone()).or_insert_with(|| Units::from(0));
    }
    let mismatches = cached
        .iter()
        .filter(|(id, amount)| {
            **amount != fresh.get(*id).cloned().unwrap_or_else(|| Units::from(0))
        })
        .count();
    println!("Account position balance differences from fresh mapping: {mismatches}");
    for b in store.account_balances_at(&AccountId::from("legacy:nexo"), cutoff)? {
        let (_, a, scale, role) = metadata
            .iter()
            .find(|(id, _, _, _)| id == &b.position_id)
            .unwrap();
        if matches!(a.0.as_str(), "USD" | "USDX" | "xUSD" | "USDC" | "USDT") {
            println!(
                "cached {} {:?} {}",
                a.0,
                role,
                b.amount.decimal_string(*scale)
            );
        }
    }
    for (id, units) in fresh {
        if matches!(
            id.as_str(),
            "NEXO:USD" | "NEXO:USDX" | "NEXO:USDC" | "NEXO:USDT" | "NEXO:xUSD" | "NEXO-CREDIT:xUSD"
        ) {
            println!("fresh {id} {}", units.decimal_string(8));
        }
    }
    Ok(())
}
