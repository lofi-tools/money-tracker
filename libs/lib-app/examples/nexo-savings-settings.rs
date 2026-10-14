//! Persist user-confirmed flexible eligibility/rate and fixed product duration.
use anyhow::{Result, ensure};
use lib_app::{Application, manual::ManualEdit};
use lib_core::{
    ProductId,
    ui_metadata::{AnnualRate, AnnualRateBasis, ProductTemplate, RateScope},
};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        (4..=5).contains(&a.len()),
        "usage: nexo-savings-settings EXISTING_DB ASSET FLEX_PERCENT FIXED_MONTHS [REWARD_ASSET]"
    );
    ensure!(std::path::Path::new(&a[0]).is_file(), "database must exist");
    let app = Application::open(&a[0])?;
    let account = app
        .store()
        .accounts(&lib_core::DEFAULT_PORTFOLIO.into())?
        .into_iter()
        .find(|a| a.id.0 == "legacy:nexo")
        .ok_or_else(|| anyhow::anyhow!("Nexo account missing"))?;
    let months: u32 = a[3].parse()?;
    ensure!(months > 0 && months <= 12, "invalid term duration");
    let percent: rust_decimal::Decimal = rust_decimal::Decimal::from_str_exact(&a[2])?;
    ensure!(
        percent >= rust_decimal::Decimal::ZERO && percent <= rust_decimal::Decimal::from(100),
        "invalid APY"
    );
    let fixed = ProductId::from(format!("nexo-fixed:{}:{}", account.id.0, a[1]));
    let mut template = app
        .store()
        .product_template(&fixed)?
        .ok_or_else(|| anyhow::anyhow!("fixed product missing"))?;
    template.term_months = Some(months);
    app.store().save_product_template(&template)?;
    let flex = ProductId::from(format!("nexo-flex:{}:{}", account.id.0, a[1]));
    app.store().save_product_template(&ProductTemplate {id:flex.clone(),provider_id:account.provider_id,
        source_reference:flex.0.clone(),label:format!("Nexo flexible savings · {}",a[1]),term_months:None,
        evidence:format!("User confirms non-fixed funds earn the flexible base rate; account-applicable {percent}% APY; payout currency only when explicitly supplied")})?;
    app.store()
        .save_subscription(&lib_core::ui_metadata::Subscription {
            id: format!("qualified-flex:{}", flex.0),
            account_id: account.id.clone(),
            product_id: flex.clone(),
            base_asset: lib_core::AssetId::str(&a[1]),
            source_reference: flex.0.clone(),
            label: "User-confirmed flexible savings".into(),
            opened_at: None,
            closed_at: None,
            lock_status: lib_core::ui_metadata::LockStatus::Flexible,
            maturity: None,
            auto_renew: None,
            positions: vec![],
        })?;
    app.apply_manual_edit(
        &account.portfolio_id,
        &ManualEdit::Rate {
            scope: RateScope::Product(flex),
            rate: Some(AnnualRate {
                fraction: (percent / rust_decimal::Decimal::from(100))
                    .normalize()
                    .to_string()
                    .parse()?,
                basis: AnnualRateBasis::Apy,
                compounds_per_year: None,
                reward_asset: a.get(4).map(|s| lib_core::AssetId::str(s)),
            }),
            effective_at: chrono::Utc::now(),
            pinned: false,
            clear: false,
        },
    )?;
    println!(
        "Saved account-confirmed flexible rate and fixed duration; financial holdings unchanged."
    );
    Ok(())
}
