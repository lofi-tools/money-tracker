//! Annotate one uniquely identified fixed allocation, never a whole-asset default.
use anyhow::{Result, ensure};
use lib_app::{Application, adapters::nexo::allocations::open_allocations, manual::ManualEdit};
use lib_core::{
    AccountId, ProductId, Units,
    ui_metadata::{AnnualRate, AnnualRateBasis, ProductTemplate, RateScope},
};
use rust_decimal::Decimal;
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        (6..=8).contains(&a.len()),
        "usage: nexo-pot-rate EXISTING_DB CSV ASSET PRINCIPAL APY_PERCENT MONTHS [REWARD_ASSET] [FLEX_APY_PERCENT]"
    );
    ensure!(
        std::path::Path::new(&a[0]).is_file(),
        "database must already exist"
    );
    let percent: Decimal = a[4].parse()?;
    ensure!(
        percent >= Decimal::ZERO && percent <= Decimal::from(100),
        "invalid APY"
    );
    let months: u32 = a[5].parse()?;
    ensure!(months > 0 && months <= 12, "invalid term months");
    let rows = nexo_csv::NexoCsv::from_file(&a[1])?;
    let cutoff = rows
        .iter()
        .map(|r| r.date_time_utc)
        .max()
        .ok_or_else(|| anyhow::anyhow!("empty export"))?;
    let account = AccountId::from("legacy:nexo");
    let parts = open_allocations(&rows, &account, "user pot rate annotation", cutoff)?;
    let matches: Vec<_> = parts
        .iter()
        .filter(|p| {
            p.quantity.asset_id.0 == a[2]
                && Units::from_decimal(&a[3], p.quantity.scale).is_ok_and(|q| q == p.quantity.units)
        })
        .collect();
    ensure!(
        matches.len() == 1 && matches[0].kind == "fixed" && matches[0].count == 1,
        "principal must identify exactly one unambiguous current term"
    );
    let part = matches[0];
    let app = Application::open(&a[0])?;
    let owner = app
        .store()
        .portfolios()?
        .into_iter()
        .find_map(|p| {
            app.store()
                .accounts(&p.id)
                .ok()?
                .into_iter()
                .find(|v| v.id == account)
        })
        .ok_or_else(|| anyhow::anyhow!("account missing"))?;
    let product = ProductId::from(format!("nexo-fixed:{}:{}", account.0, a[2]));
    app.store().save_product_template(&ProductTemplate {term_months: Some(months),id:product.clone(),provider_id:owner.provider_id.clone(),source_reference:product.0.clone(),label:format!("Nexo {months}-month fixed · {}",a[2]),evidence:"User-reported individual pot rates, not a generic fixed-rate default. Scheduled maturity and the reason for differing rates remain unknown.".into()})?;
    if let Some(flex) = a.get(7) {
        let flex_value: Decimal = flex.parse()?;
        ensure!(
            flex_value >= Decimal::ZERO && flex_value <= Decimal::from(100),
            "invalid flexible APY"
        );
        let product = ProductId::from(format!("nexo-flex:{}:{}", account.0, a[2]));
        app.store().save_product_template(&ProductTemplate {term_months: None,id:product.clone(),provider_id:owner.provider_id.clone(),source_reference:product.0.clone(),label:format!("Nexo flexible savings · {}",a[2]),evidence:format!("User reports {flex}% APY paid in {}; reference only, residual collateral eligibility unknown.",a.get(6).map(String::as_str).unwrap_or("unspecified currency"))})?;
    }
    app.apply_manual_edit(
        &owner.portfolio_id,
        &ManualEdit::Rate {
            scope: RateScope::Subscription(part.id.clone()),
            rate: Some(AnnualRate {
                fraction: (percent / Decimal::from(100))
                    .normalize()
                    .to_string()
                    .parse()?,
                basis: AnnualRateBasis::Apy,
                compounds_per_year: None,
                reward_asset: a.get(6).map(|v| lib_core::AssetId::str(v)),
            }),
            effective_at: chrono::Utc::now(),
            pinned: false,
            clear: false,
        },
    )?;
    println!(
        "Saved user-reported {percent}% expected APY on one uniquely identified {} term; ledger unchanged.",
        a[2]
    );
    Ok(())
}
