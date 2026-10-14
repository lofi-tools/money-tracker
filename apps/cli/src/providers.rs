//! Cached provider replay and a portfolio-wide exact quantity report.
use anyhow::{Context, Result, ensure};
use chrono::Utc;
use lib_core::{Account, AssetId, PositionRole, Store, Units};
use std::{collections::BTreeMap, path::Path};

pub fn report(store: &Store) -> Result<String> {
    let cutoff = Utc::now();
    let metadata: std::collections::HashMap<_, _> = store
        .position_metadata()?
        .into_iter()
        .map(|(p, a, s, r)| (p, (a, s, r)))
        .collect();
    let mut text = format!("Transaction-derived provider balances at {cutoff}\n");
    let mut providers = BTreeMap::<String, Vec<Account>>::new();
    for account in store.accounts(&lib_core::DEFAULT_PORTFOLIO.into())? {
        providers
            .entry(account.provider_id.0.clone())
            .or_default()
            .push(account);
    }
    for (provider, accounts) in providers {
        text.push_str(&format!("\n{provider}\n"));
        let mut totals = BTreeMap::<(String, bool), Units>::new();
        let mut count = 0;
        for account in accounts {
            text.push_str(&format!(
                "  Account: {} ({})\n",
                account.label, account.id.0
            ));
            for balance in store.account_balances_at(&account.id, cutoff)? {
                if balance.amount == 0 {
                    continue;
                }
                let (asset, scale, role) = &metadata[&balance.position_id];
                let debt = *role == PositionRole::Liability;
                text.push_str(&format!(
                    "    {}: {} {}{}\n",
                    balance.position_id.0,
                    balance.amount.decimal_string(*scale),
                    asset.0,
                    if debt { " (liability)" } else { "" }
                ));
                *totals.entry((asset.0.clone(), debt)).or_default() += balance.amount;
                count += 1;
            }
        }
        text.push_str("  Asset totals (owned buckets counted once; liabilities separate):\n");
        for ((asset, debt), amount) in totals {
            text.push_str(&format!(
                "    {}: {}{}\n",
                asset,
                amount.decimal_string(store.asset_scale(&AssetId::str(&asset))?),
                if debt { " (liability)" } else { "" }
            ));
        }
        if count == 0 {
            text.push_str("    No nonzero imported holdings\n");
        }
    }
    Ok(text)
}

pub async fn run(
    store: &Store,
    config: &crate::cli::Config,
    no_import: bool,
    widen_scales: bool,
    offline: bool,
    statement: Option<&Path>,
    positions: Option<&Path>,
    supplement: Option<&Path>,
    kraken: Option<&Path>,
) -> Result<()> {
    let env_statement = std::env::var_os("BINANCE_STATEMENT").map(std::path::PathBuf::from);
    let statement = statement.or(env_statement.as_deref());
    if !no_import {
        if config.nexo_csv_path.try_exists()? {
            lib_app::adapters::nexo::NexoSvc::from_path(&config.nexo_csv_path)
                .import_into(store)?;
        } else {
            ensure!(!config.nexo_csv_explicit, "explicit Nexo CSV is missing");
            eprintln!("Nexo export absent; retaining any stored holdings");
        }
        let found = if let Some(path) = statement {
            Some(path.to_path_buf())
        } else if Path::new(".cache/imports").try_exists()? {
            lib_app::adapters::binance::statements::import_path_in(Path::new(".cache/imports"))?
        } else {
            None
        };
        if let Some(path) = found {
            let reference = positions
                .map(Path::to_path_buf)
                .unwrap_or_else(|| ".cache/test-data/binance/positions-v1.json".into());
            let earn = supplement
                .map(Path::to_path_buf)
                .unwrap_or_else(|| ".cache/test-data/binance/earn-supplement-v1.json".into());
            if positions.is_some()
                || supplement.is_some()
                || (reference.try_exists()? && earn.try_exists()?)
            {
                let cutoff = lib_app::adapters::binance::statements::replay_cached(
                    store,
                    &path,
                    &config.cache_dir.join("binance/statements"),
                    &reference,
                    &earn,
                )?;
                eprintln!("Binance statement + Earn supplement reconcile exactly at {cutoff}");
            } else {
                ensure!(
                    statement.is_none(),
                    "explicit Binance statement requires positions and Earn supplement caches"
                );
                eprintln!("Binance reference/supplement absent; retaining stored holdings");
            }
        } else {
            ensure!(
                positions.is_none() && supplement.is_none(),
                "Binance cache options require a statement"
            );
            eprintln!("Binance statement absent; retaining stored holdings");
        }
        let id = std::env::var("KRAKEN_ACCOUNT_ID").unwrap_or_else(|_| "kraken:primary".into());
        let default = std::path::PathBuf::from(".cache/test-data/kraken").join(format!(
            "{}.json",
            kraken_client::fingerprint(id.as_bytes())
        ));
        let path = kraken.unwrap_or(&default);
        if kraken.is_some() || path.try_exists()? {
            let snapshot: kraken_client::Snapshot =
                serde_json::from_slice(&std::fs::read(path)?).context("invalid Kraken snapshot")?;
            if widen_scales {
                let plan = lib_app::adapters::kraken::required_scales(&snapshot)?;
                let mut widened = std::collections::HashSet::new();
                let existing: std::collections::HashMap<_, _> = store
                    .position_metadata()?
                    .into_iter()
                    .map(|(_, asset, scale, _)| (asset, scale))
                    .collect();
                for (asset, scale) in plan {
                    if let Some(old) = existing.get(&asset) {
                        if *old < scale && widened.insert(asset.clone()) {
                            store.widen_asset_scale(&asset, scale)?;
                            eprintln!(
                                "Losslessly widened {} precision: {} → {} decimals",
                                asset.0, old, scale
                            );
                        }
                    }
                }
            }
            let account = Account {
                id: snapshot.account_id.clone().into(),
                portfolio_id: lib_core::DEFAULT_PORTFOLIO.into(),
                provider_id: "kraken".into(),
                label: "Kraken account".into(),
                source_reference: None,
            };
            let hash = lib_app::adapters::kraken::import_snapshot(store, &account, &snapshot)?;
            lib_app::adapters::kraken::reconcile(store, &account.id, &snapshot, &hash)?;
            eprintln!("Kraken reconciles exactly at {}", snapshot.observed_at);
        } else {
            eprintln!("Kraken snapshot absent; retaining stored holdings");
        }
    }
    if !no_import && !offline {
        let portfolio = lib_core::DEFAULT_PORTFOLIO.into();
        let quote = config.reporting_currency.clone();
        let mut missing = Vec::new();
        for account in store.accounts(&portfolio)? {
            for (_, asset, _, role) in store.account_position_metadata(&account.id)? {
                if role != PositionRole::ExternalCounterparty
                    && asset != quote
                    && !missing.contains(&asset)
                    && store
                        .selected_price(&portfolio, &asset, &quote, Utc::now())?
                        .is_none()
                {
                    missing.push(asset);
                }
            }
        }
        if !missing.is_empty() {
            let prices = lib_app::adapters::coingecko::CoinGeckoSvc::new()?
                .fetch_exact_current_prices(&missing)
                .await?;
            for price in &prices {
                store.save_price_evidence(price)?;
            }
            eprintln!(
                "Cached {} latest public price observations; unsupported assets remain unknown",
                prices.len()
            );
        }
    }
    print!("{}", report(store)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_report_and_projection_include_each_provider_without_counterparties() -> Result<()> {
        let store = Store::in_memory()?;
        let time = "2024-01-01T00:00:00Z".parse()?;
        for (provider, amount) in [
            ("nexo", "1"),
            ("binance", "2"),
            ("kraken", "3"),
            ("evm", "4"),
        ] {
            let account = Account {
                id: provider.into(),
                portfolio_id: lib_core::DEFAULT_PORTFOLIO.into(),
                provider_id: provider.into(),
                label: provider.into(),
                source_reference: None,
            };
            store.save_account(&account)?;
            let own = lib_core::TxnEffect {
                datetime: time,
                position_id: "spot:BTC".into(),
                amount: Units::from_decimal(amount, 8)?,
            };
            let external = lib_core::TxnEffect {
                datetime: time,
                position_id: "external:BTC".into(),
                amount: own.amount.clone(),
            };
            store.save_account_import(
                &account.id,
                "fixture",
                "snapshot",
                &[
                    (
                        "spot:BTC".into(),
                        AssetId::str("BTC"),
                        8,
                        PositionRole::Asset,
                    ),
                    (
                        "external:BTC".into(),
                        AssetId::str("BTC"),
                        8,
                        PositionRole::ExternalCounterparty,
                    ),
                ],
                &[(
                    "deposit".into(),
                    lib_core::Transaction {
                        datetime: time,
                        inputs: vec![external],
                        outputs: vec![own],
                    },
                )],
                &[],
            )?;
        }
        let text = report(&store)?;
        for provider in ["nexo", "binance", "kraken", "evm"] {
            assert!(text.contains(&format!("\n{provider}\n")));
        }
        assert!(!text.contains("external:BTC"));
        assert!(text.contains("BTC: 3.00000000"));
        let app = lib_app::Application::from_store(store);
        let projection = app.portfolio_projection(lib_app::portfolio::PortfolioRequest {
            context: lib_app::dto::ReadContext {
                portfolio_id: lib_core::DEFAULT_PORTFOLIO.into(),
                cutoff: Utc::now(),
            },
            reporting_asset: AssetId::str("USD"),
            dust_threshold: "1".parse()?,
        })?;
        for provider in ["binance", "kraken"] {
            assert!(
                projection.providers.iter().any(
                    |e| e.selection == lib_app::portfolio::Selection::Provider(provider.into())
                )
            );
        }
        Ok(())
    }
}
