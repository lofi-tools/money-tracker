//! Record explicit account-applicable product rates; not advertised maxima.
use anyhow::{Result, ensure};
use lib_app::{Application, manual::ManualEdit};
use lib_core::{
    ProductId,
    ui_metadata::{AnnualRate, AnnualRateBasis, ProductTemplate, RateScope},
};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        (5..=6).contains(&args.len()),
        "usage: nexo-account-rates EXISTING_DB ASSET FLEX_PERCENT FIXED_PERCENT FIXED_MONTHS [REWARD_ASSET]"
    );
    ensure!(
        std::path::Path::new(&args[0]).is_file(),
        "database must already exist"
    );
    let app = Application::open(&args[0])?;
    let account = app
        .store()
        .portfolios()?
        .into_iter()
        .find_map(|p| {
            app.store()
                .accounts(&p.id)
                .ok()?
                .into_iter()
                .find(|a| a.id.0 == "legacy:nexo")
        })
        .ok_or_else(|| anyhow::anyhow!("Nexo account missing"))?;
    let months: u32 = args[4].parse()?;
    ensure!(months > 0 && months <= 12, "invalid fixed months");
    for (kind, percent, label) in [
        (
            "flex",
            &args[2],
            format!("Nexo flexible savings · {}", args[1]),
        ),
        (
            "fixed",
            &args[3],
            format!("Nexo {}-month fixed · {}", months, args[1]),
        ),
    ] {
        let product = ProductId::from(format!("nexo-{kind}:{}:{}", account.id.0, args[1]));
        app.store().save_product_template(&ProductTemplate {term_months: None,id:product.clone(),provider_id:account.provider_id.clone(),source_reference:product.0.clone(),label,evidence:format!("User supplied {percent}% APY for {kind}; fixed product term {months} months; payout currency as explicitly annotated. Flexible eligibility remains unallocated.")})?;
        let percent: rust_decimal::Decimal = percent.parse()?;
        ensure!(
            percent >= rust_decimal::Decimal::ZERO && percent <= rust_decimal::Decimal::from(100),
            "invalid annual percent"
        );
        let fraction = (percent / rust_decimal::Decimal::from(100))
            .normalize()
            .to_string()
            .parse()?;
        if kind == "flex" {
            continue;
        } // Reference fact, never an invented eligible holding.
        app.apply_manual_edit(
            &account.portfolio_id,
            &ManualEdit::Rate {
                scope: RateScope::Product(product),
                rate: Some(AnnualRate {
                    fraction,
                    basis: AnnualRateBasis::Apy,
                    compounds_per_year: None,
                    reward_asset: args.get(5).map(|a| lib_core::AssetId::str(a)),
                }),
                effective_at: chrono::Utc::now(),
                pinned: false,
                clear: false,
            },
        )?;
    }
    println!(
        "Explicit flexible/fixed product rates saved. Flexible rate does not classify unallocated collateral."
    );
    Ok(())
}
