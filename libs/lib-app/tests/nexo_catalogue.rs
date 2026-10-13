use anyhow::Result;
use chrono::NaiveDate;
use lib_app::{Application, adapters::nexo::rate_catalogue::*, exact::ExactRatio};
use lib_core::{
    Account, DEFAULT_PORTFOLIO, Store,
    ui_metadata::{AnnualRate, AnnualRateBasis},
};
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}
fn facts(asset: &str) -> Facts {
    Facts {
        at: date(13),
        asset: asset.into(),
        operation: Operation::EarnFlexible,
        months: None,
        wallet: Some(Wallet::Savings),
        payout: Some(Payout::InKind),
        quantity: Some("1000".parse().unwrap()),
        product_value_usd: Some("10000".parse().unwrap()),
        portfolio_usd: Some("11000".parse().unwrap()),
        nexo_usd: Some("1000".parse().unwrap()),
        complete_provider_valuation: true,
        platform_tier: None,
        loyalty_access: None,
        opt_in_valid_until: Some(date(25)),
        earnable: Some(true),
        minimum: Some(ConfirmedMinimum {
            asset: asset.into(),
            source_reference: "confirmed account minimum".into(),
            units: "500".parse().unwrap(),
            scope: MinimumScope::FlexibleBalance,
            effective_from: date(1),
            valid_until: date(15),
        }),
        active_term_fallback_confirmed: None,
        current_term_count: Some(0),
    }
}
fn evaluate_uk(c: &Catalogue, f: &Facts) -> Result<Evaluation> {
    evaluate(c, &"account:nexo".into(), Some("GB"), f)
}
fn documented(c: &mut Catalogue, asset: &str) {
    c.rates.push(RateRecord {
        id: "confirmed".into(),
        country: "GB".into(),
        asset: asset.into(),
        product: Product::Flexible,
        fraction: Some("0.05".parse().unwrap()),
        kind: RateKind::AccountApplicable,
        basis: Some(AnnualRateBasis::Apy),
        compounds_per_year: None,
        tier: Some(Tier::Platinum),
        payout: Some(Payout::InKind),
        months: None,
        bands: None,
        source: "flex".into(),
        effective_from: Some(date(1)),
    });
}
#[test]
fn bundled_rates_are_not_defaults_even_with_complete_eligibility() -> Result<()> {
    let c = Catalogue::bundled()?;
    for asset in ["ETH", "BNB", "NEAR", "NEXO", "USDT"] {
        let r = evaluate_uk(&c, &facts(asset))?;
        assert_eq!(r.eligibility, Eligibility::Eligible);
        assert_eq!(r.tier, Some(Tier::Platinum));
        assert!(r.annual_fraction.is_none() && r.known_annual_earnings_base.is_none());
        assert!(r.rate_issues.contains(&"applicable_rate_missing".into()));
    }
    assert!(
        c.rates
            .iter()
            .find(|r| r.asset == "NEAR" && r.product == Product::Flexible)
            .unwrap()
            .fraction
            .is_none()
    );
    assert!(
        c.rates
            .iter()
            .find(|r| r.id == "gb-flexible-nexo")
            .unwrap()
            .basis
            .is_none()
    );
    Ok(())
}
#[test]
fn tier_ratio_uses_rest_of_provider_and_exact_boundaries() -> Result<()> {
    let c = Catalogue::bundled()?;
    let mut f = facts("BNB");
    for (nexo, total, tier) in [
        ("1000", "11000", Tier::Platinum),
        ("500", "10500", Tier::Gold),
        ("100", "10100", Tier::Silver),
        ("99.99999999", "10099.99999999", Tier::Base),
    ] {
        f.nexo_usd = Some(nexo.parse()?);
        f.portfolio_usd = Some(total.parse()?);
        assert_eq!(evaluate_uk(&c, &f)?.tier, Some(tier));
    }
    f.complete_provider_valuation = false;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    f.platform_tier = Some(Tier::Gold);
    f.loyalty_access = Some(true);
    assert_eq!(evaluate_uk(&c, &f)?.tier, Some(Tier::Gold));
    f.platform_tier = None;
    f.complete_provider_valuation = true;
    f.nexo_usd = f.portfolio_usd.clone();
    assert_eq!(evaluate_uk(&c, &f)?.tier, None);
    Ok(())
}
#[test]
fn country_and_date_never_pick_other_programme_or_historical_snapshot() -> Result<()> {
    let c = Catalogue::bundled()?;
    let mut f = facts("ETH");
    for country in [None, Some("US"), Some("DE")] {
        let r = evaluate(&c, &"a".into(), country, &f)?;
        assert_eq!(r.eligibility, Eligibility::Unknown);
        assert!(r.annual_fraction.is_none());
    }
    for at in [date(12), date(27)] {
        f.at = at;
        assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    }
    Ok(())
}
#[test]
fn collateral_cooldown_minimum_and_opt_in_are_independent() -> Result<()> {
    let c = Catalogue::bundled()?;
    let mut f = facts("ETH");
    f.wallet = Some(Wallet::Credit);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    f.asset = "NEXO".into();
    f.minimum.as_mut().unwrap().asset = "NEXO".into();
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Eligible);
    f.earnable = None;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    f.earnable = Some(false);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    f.earnable = Some(true);
    f.quantity = Some("499.99999999".parse()?);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    f.quantity = Some("500".parse()?);
    f.minimum.as_mut().unwrap().valid_until = date(13);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    f.minimum = None;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    f.opt_in_valid_until = Some(date(13));
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    Ok(())
}
#[test]
fn grace_and_active_fixed_fallback_do_not_share_a_guessed_transition() -> Result<()> {
    let c = Catalogue::bundled()?;
    let mut f = facts("BNB");
    f.portfolio_usd = Some("4000".parse()?);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    f.loyalty_access = Some(true);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Eligible);
    f.loyalty_access = Some(false);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    f.operation = Operation::ExistingFixed;
    f.months = Some(1);
    f.minimum = None;
    f.opt_in_valid_until = None;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    f.active_term_fallback_confirmed = Some(true);
    let r = evaluate_uk(&c, &f)?;
    assert_eq!(r.eligibility, Eligibility::Eligible);
    assert_eq!(r.tier, Some(Tier::Base));
    assert!(r.annual_fraction.is_none());
    f.operation = Operation::RenewFixed;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    Ok(())
}
#[test]
fn fixed_duration_count_and_minimum_grouping_are_verified() -> Result<()> {
    let c = Catalogue::bundled()?;
    let mut f = facts("BNB");
    f.operation = Operation::CreateFixed;
    f.months = Some(1);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    f.minimum.as_mut().unwrap().scope = MinimumScope::IndividualTerm;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Eligible);
    f.months = Some(3);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    f.asset = "NEXO".into();
    f.minimum.as_mut().unwrap().asset = "NEXO".into();
    f.months = Some(12);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Eligible);
    f.current_term_count = Some(50);
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    f.current_term_count = Some(0);
    f.asset = "DOGE".into();
    f.minimum.as_mut().unwrap().asset = "DOGE".into();
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    Ok(())
}
#[test]
fn a_complete_applicable_rate_works_without_bonuses_or_maxima() -> Result<()> {
    let mut c = Catalogue::bundled()?;
    documented(&mut c, "BNB");
    let r = evaluate_uk(&c, &facts("BNB"))?;
    assert_eq!(
        r.annual_fraction,
        ExactRatio::integer(1).div(&ExactRatio::integer(20))
    );
    assert_eq!(r.known_annual_earnings_base, Some(ExactRatio::integer(50)));
    c.rates.last_mut().unwrap().basis = None;
    assert!(evaluate_uk(&c, &facts("BNB"))?.annual_fraction.is_none());
    c.rates.last_mut().unwrap().basis = Some(AnnualRateBasis::Apy);
    let mut duplicate = c.rates.last().unwrap().clone();
    duplicate.id = "conflict".into();
    c.rates.push(duplicate);
    assert!(
        evaluate_uk(&c, &facts("BNB"))?
            .rate_issues
            .contains(&"conflicting_rate_evidence".into())
    );
    Ok(())
}
fn band_rate(rate: &str) -> AnnualRate {
    AnnualRate {
        fraction: rate.parse().unwrap(),
        basis: AnnualRateBasis::Apy,
        compounds_per_year: None,
        reward_asset: None,
    }
}
#[test]
fn marginal_bands_preserve_partial_known_earnings_without_overall_apy() -> Result<()> {
    let mut c = Catalogue::bundled()?;
    documented(&mut c, "ETH");
    assert!(evaluate_uk(&c, &facts("ETH"))?.annual_fraction.is_none());
    c.rates.last_mut().unwrap().bands = Some(vec![
        RateBand {
            up_to_usd: Some("4000".parse()?),
            rate: Some(band_rate("0.05")),
        },
        RateBand {
            up_to_usd: None,
            rate: Some(band_rate("0.01")),
        },
    ]);
    let r = evaluate_uk(&c, &facts("ETH"))?;
    assert_eq!(
        r.annual_fraction,
        ExactRatio::integer(26).div(&ExactRatio::integer(1000))
    );
    assert_eq!(r.known_annual_earnings_base, Some(ExactRatio::integer(26)));
    c.rates.last_mut().unwrap().bands.as_mut().unwrap()[1].rate = None;
    let r = evaluate_uk(&c, &facts("ETH"))?;
    assert!(!r.rate_complete && r.annual_fraction.is_none());
    assert_eq!(r.known_annual_earnings_base, Some(ExactRatio::integer(20)));
    Ok(())
}
#[test]
fn malformed_rate_catalogues_and_unconfirmed_scopes_fail_closed() -> Result<()> {
    let mut c = Catalogue::bundled()?;
    let mut f = facts("BNB");
    f.quantity = Some("-1".parse()?);
    assert!(evaluate_uk(&c, &f).is_err());
    c.rates[0].source = "absent".into();
    assert!(c.validate().is_err());
    let mut c = Catalogue::bundled()?;
    c.rates[0].country = "US".into();
    assert!(c.validate().is_err());
    Ok(())
}
#[test]
fn application_reads_jurisdiction_from_the_specific_account() -> Result<()> {
    let store = Store::in_memory()?;
    for id in ["a", "b"] {
        store.save_account(&Account {
            id: id.into(),
            portfolio_id: DEFAULT_PORTFOLIO.into(),
            provider_id: "nexo".into(),
            label: id.into(),
            source_reference: None,
        })?;
    }
    store.set_account_jurisdiction(&"a".into(), "UK", "2026-10-13T00:00:00Z".parse()?)?;
    let app = Application::from_store(store);
    assert_eq!(
        app.nexo_rate_eligibility(&"a".into(), &facts("BNB"))?
            .eligibility,
        Eligibility::Eligible
    );
    assert_eq!(
        app.nexo_rate_eligibility(&"b".into(), &facts("BNB"))?
            .eligibility,
        Eligibility::Unknown
    );
    assert_eq!(
        app.ui_metadata(&DEFAULT_PORTFOLIO.into())?
            .accounts
            .iter()
            .find(|a| a.account.id.0 == "a")
            .unwrap()
            .account
            .jurisdiction
            .as_ref()
            .unwrap()
            .country_code,
        "GB"
    );
    Ok(())
}

#[test]
fn apr_compounding_is_explicit_and_headline_bonuses_are_never_added() -> Result<()> {
    let mut c = Catalogue::bundled()?;
    documented(&mut c, "BNB");
    c.rates.last_mut().unwrap().basis = Some(AnnualRateBasis::Apr);
    let r = evaluate_uk(&c, &facts("BNB"))?;
    assert!(r.annual_fraction.is_none());
    c.rates.last_mut().unwrap().compounds_per_year = Some(365);
    let r = evaluate_uk(&c, &facts("BNB"))?;
    let fraction = ExactRatio::integer(1)
        .div(&ExactRatio::integer(20))
        .unwrap();
    let expected = ExactRatio::integer(1)
        .add(&fraction.div(&ExactRatio::integer(365)).unwrap())
        .pow(365)
        .sub(&ExactRatio::integer(1));
    assert_eq!(r.annual_fraction, Some(expected));
    c.rates.last_mut().unwrap().compounds_per_year = Some(0);
    assert!(evaluate_uk(&c, &facts("BNB"))?.annual_fraction.is_none());
    c.rates.last_mut().unwrap().kind = RateKind::HeadlineMaximum;
    c.rates.last_mut().unwrap().basis = Some(AnnualRateBasis::Apy);
    assert!(evaluate_uk(&c, &facts("BNB"))?.annual_fraction.is_none());
    Ok(())
}

#[test]
fn renewal_replaces_a_term_slot_and_does_not_add_an_extra_one() -> Result<()> {
    let c = Catalogue::bundled()?;
    let mut f = facts("NEXO");
    f.operation = Operation::RenewFixed;
    f.months = Some(12);
    f.current_term_count = Some(50);
    f.minimum.as_mut().unwrap().scope = MinimumScope::IndividualTerm;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Eligible);
    f.operation = Operation::CreateFixed;
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Ineligible);
    f.current_term_count = Some(0);
    f.minimum.as_mut().unwrap().asset = "ETH".into();
    assert_eq!(evaluate_uk(&c, &f)?.eligibility, Eligibility::Unknown);
    Ok(())
}
