//! Explicit user rates on an equal-principal unresolved cohort, without assigning identities.
use anyhow::{Result, ensure};
use lib_app::{Application, adapters::nexo::allocations::open_allocations, manual::ManualEdit};
use lib_core::{
    AccountId, ProductId,
    ui_metadata::{AnnualRate, AnnualRateBasis, ProductTemplate, RateScope},
};
use rust_decimal::Decimal;
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        a.len() == 5,
        "usage: nexo-unresolved-rates EXISTING_DB CSV ASSET FIRST_PERCENT SECOND_PERCENT"
    );
    ensure!(
        std::path::Path::new(&a[0]).is_file(),
        "database must already exist"
    );
    let app = Application::open(&a[0])?;
    let account = AccountId::from("legacy:nexo");
    let owner_account = app
        .store()
        .portfolios()?
        .into_iter()
        .find_map(|port| {
            app.store()
                .accounts(&port.id)
                .ok()?
                .into_iter()
                .find(|a| a.id == account)
        })
        .ok_or_else(|| anyhow::anyhow!("account missing"))?;
    let rows = nexo_csv::NexoCsv::from_file(&a[1])?;
    let cutoff = rows
        .iter()
        .map(|r| r.date_time_utc)
        .max()
        .ok_or_else(|| anyhow::anyhow!("empty CSV"))?;
    let parts = open_allocations(&rows, &account, "user rate annotation", cutoff)?;
    let cohorts: Vec<_> = parts
        .iter()
        .filter(|p| p.quantity.asset_id.0 == a[2] && p.kind == "unresolved_fixed" && p.count == 2)
        .collect();
    ensure!(
        cohorts.len() == 1,
        "rates require exactly one two-member equal-principal cohort"
    );
    let p = cohorts[0];
    let first: Decimal = a[3].parse()?;
    let second: Decimal = a[4].parse()?;
    ensure!(
        first >= Decimal::ZERO
            && first <= Decimal::from(100)
            && second >= Decimal::ZERO
            && second <= Decimal::from(100),
        "invalid APY"
    );
    let product = ProductId::from(format!("nexo-fixed-group:{}:{}", p.id, p.count));
    app.store().save_product_template(&ProductTemplate {term_months: Some(12),id:product.clone(),provider_id:owner_account.provider_id.clone(),source_reference:product.0.clone(),label:format!("Old 12-month {} pots · {}% / {}% · individual identity unresolved",a[2],first,second),evidence:"User confirmed grandfathered current rates. Equal principal permits an aggregate mean; no individual rate assignment or current offer inferred.".into()})?;
    app.store()
        .save_subscription(&lib_core::ui_metadata::Subscription {
            id: format!("qualified-group:{}", product.0),
            account_id: account.clone(),
            product_id: product.clone(),
            base_asset: p.quantity.asset_id.clone(),
            source_reference: product.0.clone(),
            label: "User-qualified equal-principal allocation group; individual identities unknown"
                .into(),
            opened_at: None,
            closed_at: None,
            lock_status: lib_core::ui_metadata::LockStatus::Locked,
            maturity: None,
            auto_renew: None,
            positions: vec![],
        })?;
    let owner = app
        .store()
        .portfolios()?
        .into_iter()
        .find(|port| {
            app.store()
                .accounts(&port.id)
                .is_ok_and(|accounts| accounts.iter().any(|v| v.id == account))
        })
        .ok_or_else(|| anyhow::anyhow!("account missing"))?;
    if a[2] == "EURX" {
        let flex = ProductId::from(format!("nexo-flex:{}:EURX", account.0));
        app.store().save_product_template(&ProductTemplate { term_months: None, id:flex.clone(), provider_id:owner_account.provider_id.clone(), source_reference:flex.0.clone(), label:"Nexo flexible savings · EURX".into(), evidence:"User supplied 6% base APY, +2 percentage points if paid in NEXO; reference only, unclassified residual may be collateral.".into() })?;
    }
    let fraction = ((first + second) / Decimal::from(200))
        .normalize()
        .to_string()
        .parse()?;
    app.apply_manual_edit(
        &owner.id,
        &ManualEdit::Rate {
            scope: RateScope::Product(product),
            rate: Some(AnnualRate {
                fraction,
                basis: AnnualRateBasis::Apy,
                compounds_per_year: None,
                reward_asset: None,
            }),
            effective_at: chrono::Utc::now(),
            pinned: false,
            clear: false,
        },
    )?;
    println!(
        "Aggregate expected rate saved for two equal-principal pots; individual assignments remain unresolved."
    );
    Ok(())
}
