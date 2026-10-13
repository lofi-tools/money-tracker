//! Fill missing public market quotes only; never contact account providers.
use anyhow::{Result, ensure};
use chrono::Utc;
use lib_app::adapters::coingecko::CoinGeckoSvc;
use lib_core::{AssetId, Store};
use std::collections::BTreeSet;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2,
        "usage: missing-prices EXISTING_DB REPORTING_CURRENCY"
    );
    ensure!(
        std::path::Path::new(&args[0]).is_file(),
        "database must already exist"
    );
    let store = Store::open(&args[0])?;
    let quote = AssetId::str(&args[1]);
    let mut missing = BTreeSet::new();
    for portfolio in store.portfolios()? {
        for account in store.accounts(&portfolio.id)? {
            for (_, asset, _, role) in store.account_position_metadata(&account.id)? {
                if !matches!(
                    role,
                    lib_core::PositionRole::Asset | lib_core::PositionRole::Liability
                ) {
                    continue;
                }
                if asset != quote
                    && (role == lib_core::PositionRole::Liability
                        || !store.asset_is_excluded(&account.id, &asset)?)
                    && store
                        .selected_price(&portfolio.id, &asset, &quote, Utc::now())?
                        .is_none()
                {
                    missing.insert(asset.0);
                }
            }
        }
    }
    if missing.is_empty() {
        println!("All held assets have stored quotes.");
        return Ok(());
    }
    let prices = CoinGeckoSvc::new()?
        .fetch_exact_current_prices(
            &missing
                .into_iter()
                .map(|a| AssetId::str(&a))
                .collect::<Vec<_>>(),
        )
        .await?;
    let mut saved = 0;
    for price in prices.into_iter().filter(|p| p.quote == quote) {
        store.save_price_evidence(&price)?;
        saved += 1;
    }
    println!(
        "Saved {saved} public quotes; unsupported mappings remain unknown. Provider transactions unchanged."
    );
    Ok(())
}
