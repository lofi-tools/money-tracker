use anyhow::Result;
use chrono::{DateTime, Utc};
use lib_app::{Application, dto::*, exact::ExactRatio, portfolio::*};
use lib_core::{ui_metadata::*, *};
fn t(day: u32) -> DateTime<Utc> {
    format!("2026-01-{day:02}T00:00:00Z").parse().unwrap()
}
fn req() -> PortfolioRequest {
    PortfolioRequest {
        context: ReadContext {
            portfolio_id: DEFAULT_PORTFOLIO.into(),
            cutoff: t(20),
        },
        reporting_asset: AssetId::str("USD"),
        dust_threshold: "1".parse().unwrap(),
    }
}
fn account(s: &Store, id: &str, provider: &str) -> Result<()> {
    s.save_account(&Account {
        id: id.into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        provider_id: provider.into(),
        label: format!("Source {id}"),
        source_reference: Some(format!("remote:{id}")),
    })
}
fn pos(
    s: &Store,
    account: &str,
    id: &str,
    asset: &str,
    scale: u8,
    role: PositionRole,
    q: &str,
) -> Result<PositionId> {
    let before: Vec<_> = s
        .account_position_metadata(&account.into())?
        .into_iter()
        .map(|(id, _, _, _)| id)
        .collect();
    let local: PositionId = id.into();
    let effect = TxnEffect {
        datetime: t(1),
        position_id: local.clone(),
        amount: Units::from_decimal(q, scale)?,
    };
    s.save_account_import(
        &account.into(),
        "fixture",
        id,
        &[(local.clone(), AssetId::str(asset), scale, role)],
        &[(
            id.into(),
            Transaction {
                datetime: t(1),
                inputs: if role == PositionRole::Liability {
                    vec![effect.clone()]
                } else {
                    vec![]
                },
                outputs: if role != PositionRole::Liability {
                    vec![effect]
                } else {
                    vec![]
                },
            },
        )],
        &[],
    )?;
    if role == PositionRole::ExternalCounterparty {
        return Ok(local);
    }
    Ok(s.account_position_metadata(&account.into())?
        .into_iter()
        .map(|(id, _, _, _)| id)
        .find(|id| !before.contains(id))
        .expect("new fixture position"))
}
fn source(day: u32) -> SourceEvidence {
    SourceEvidence {
        source: "fixture API".into(),
        reference: format!("source:{day}"),
        observed_at: t(day),
        updated_at: t(day),
    }
}
fn rate(s: &Store, id: &str, scope: RateScope, value: Option<&str>) -> Result<()> {
    s.save_rate_evidence(&RateEvidence {
        id: id.into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        scope,
        effective_at: t(1),
        rate: value.map(|v| AnnualRate {
            fraction: v.parse().unwrap(),
            basis: AnnualRateBasis::Apy,
            compounds_per_year: None,
            reward_asset: None,
        }),
        evidence: source(2),
    })
}
fn price(s: &Store, asset: &str, v: &str) -> Result<()> {
    s.save_price_evidence(&PriceEvidence {
        id: format!("price:{asset}"),
        asset: AssetId::str(asset),
        quote: AssetId::str("USD"),
        as_of: t(1),
        price: v.parse()?,
        evidence: source(2),
    })
}
fn product(s: &Store, id: &str, provider: &str) -> Result<()> {
    s.save_product_template(&ProductTemplate {
        id: id.into(),
        provider_id: provider.into(),
        source_reference: id.into(),
        label: "Earn".into(),
        evidence: "fixture terms".into(),
    })
}
fn sub(
    s: &Store,
    id: &str,
    account: &str,
    product: &str,
    position: PositionId,
    asset: &str,
) -> Result<()> {
    s.save_subscription(&Subscription {
        id: id.into(),
        account_id: account.into(),
        product_id: product.into(),
        base_asset: AssetId::str(asset),
        source_reference: id.into(),
        label: format!("Subscription {id}"),
        opened_at: Some(t(1)),
        closed_at: None,
        lock_status: LockStatus::Locked,
        maturity: Some(t(25)),
        auto_renew: Some(true),
        positions: vec![position],
    })
}
fn amount(f: &FigureDto<CalculatedMoney>) -> ExactRatio {
    match f {
        FigureDto::Known { value, .. } | FigureDto::Partial { value, .. } => value.amount.clone(),
        _ => panic!("no value: {f:?}"),
    }
}
fn decimal(s: &str) -> ExactRatio {
    ExactRatio::from_decimal(&s.parse().unwrap())
}
fn known<T>(f: &FigureDto<T>) -> bool {
    matches!(f, FigureDto::Known { .. })
}

#[test]
fn weighted_subscriptions_idle_debt_and_account_tree_count_once() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "nexo")?;
    account(s, "b", "nexo")?;
    account(s, "c", "kraken")?;
    let one = pos(s, "a", "one", "ETH", 18, PositionRole::Asset, "50")?;
    let two = pos(s, "b", "two", "ETH", 18, PositionRole::Asset, "25")?;
    pos(s, "c", "idle", "ETH", 18, PositionRole::Asset, "25")?;
    pos(
        s,
        "c",
        "peer",
        "ETH",
        18,
        PositionRole::ExternalCounterparty,
        "999999",
    )?;
    price(s, "ETH", "10")?;
    product(s, "earn", "nexo")?;
    sub(s, "one", "a", "earn", one, "ETH")?;
    sub(s, "two", "b", "earn", two, "ETH")?;
    rate(s, "r1", RateScope::Subscription("one".into()), Some("0.05"))?;
    rate(s, "r2", RateScope::Subscription("two".into()), Some("0.06"))?;
    rate(
        s,
        "idle",
        RateScope::AccountAsset {
            account: "c".into(),
            asset: AssetId::str("ETH"),
        },
        Some("0"),
    )?;
    s.save_account_structure(&AccountStructure {
        account_id: "b".into(),
        kind: NodeKind::Source,
        source_parent: Some("a".into()),
        mapping_version: 1,
    })?;
    let before = s.revisions(&DEFAULT_PORTFOLIO.into())?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.gross_assets), decimal("1000"));
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("40"));
    assert!(
        matches!(&p.overview.current_apy,FigureDto::Known{value,..} if *value==decimal("0.04"))
    );
    assert_eq!(p.holdings.len(), 3);
    assert_eq!(
        p.assets[0].quantities[0].1.decimal_string(),
        "100.000000000000000000"
    );
    let a = p.details(AllocationTarget::Account("a".into()))?;
    assert_eq!(amount(&a.node.summary.current_value), decimal("750"));
    assert_eq!(a.holdings.len(), 2);
    // Stable synthetic wire fixture for future frontend consumers.
    let wire = serde_json::json!({"contract_version":p.contract_version,"context":p.request.context,"reporting_asset":p.request.reporting_asset,"overview":p.overview,"asset_quantity":p.assets[0].quantities[0].1,"calculation_basis":p.calculation_basis});
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/spec/fixtures/current-portfolio.json");
    if std::env::var_os("UPDATE_PORTFOLIO_FIXTURE").is_some() {
        std::fs::create_dir_all(fixture.parent().unwrap())?;
        std::fs::write(
            &fixture,
            format!("{}\n", serde_json::to_string_pretty(&wire)?),
        )?;
    }
    let expected: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(fixture)?)?;
    assert_eq!(wire, expected);
    assert_eq!(p.revisions, before);
    assert_eq!(s.revisions(&DEFAULT_PORTFOLIO.into())?, before);
    let debt = pos(s, "a", "debt", "USD", 2, PositionRole::Liability, "100")?;
    rate(s, "borrow", RateScope::BorrowingPosition(debt), Some("0.1"))?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.current_value), decimal("900"));
    assert_eq!(amount(&p.overview.annual_borrowing_costs), decimal("10"));
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("30"));
    assert!(
        matches!(&p.overview.current_apy,FigureDto::Known{value,..} if *value==decimal("0.03"))
    );
    let dt = p.details(AllocationTarget::Subscription("one".into()))?;
    assert_eq!(dt.subscription.unwrap().auto_renew, Some(true));
    Ok(())
}
#[test]
fn dust_views_include_all_values_manual_policy_and_unknown_prices() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "nexo")?;
    account(s, "b", "kraken")?;
    pos(s, "a", "big", "ETH", 18, PositionRole::Asset, "100")?;
    pos(s, "b", "tiny", "ETH", 18, PositionRole::Asset, "0.05")?;
    price(s, "ETH", "10")?;
    pos(s, "b", "unknown", "MYSTERY", 0, PositionRole::Asset, "1")?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.current_value), decimal("1000.5"));
    assert!(matches!(
        p.overview.current_value,
        FigureDto::Partial { .. }
    ));
    let eth = p
        .assets
        .iter()
        .find(|n| n.selection == Selection::Asset(AssetId::str("ETH")))
        .unwrap();
    assert_eq!(amount(&eth.summary.current_value), decimal("1000.5"));
    assert_eq!(amount(&p.dust.summary.current_value), decimal("0.5"));
    assert_eq!(p.holdings.iter().filter(|h| h.is_dust).count(), 1);
    let mystery = p
        .holdings
        .iter()
        .find(|h| h.quantity.asset_id == AssetId::str("MYSTERY"))
        .unwrap();
    assert!(!mystery.is_dust);
    assert!(matches!(mystery.value, FigureDto::Unknown { .. }));
    let target = LocalTarget::Dust {
        provider: "kraken".into(),
        asset: AssetId::str("ETH"),
    };
    s.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &target,
        0,
        Some(LocalValue::Dust(DustMode::Significant)),
    )?;
    assert!(
        !app.portfolio_projection(req())?
            .holdings
            .iter()
            .any(|h| h.is_dust)
    );
    let filtered = app.navigation(req(), true, "ETH")?;
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        amount(&filtered[0].summary.current_value),
        decimal("1000.5")
    );
    let mut exact = req();
    exact.dust_threshold = "0.5".parse()?;
    s.apply_manual_change(&DEFAULT_PORTFOLIO.into(), &target, 1, None)?;
    assert!(
        !app.portfolio_projection(exact)?
            .holdings
            .iter()
            .any(|h| h.is_dust)
    );
    Ok(())
}
#[test]
fn unknown_rates_prices_borrowing_and_zero_loss_remain_truthful() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "nexo")?;
    pos(s, "a", "known", "USD", 2, PositionRole::Asset, "100")?;
    pos(s, "a", "unknown", "ETH", 18, PositionRole::Asset, "1")?;
    rate(
        s,
        "usd",
        RateScope::AccountAsset {
            account: "a".into(),
            asset: AssetId::str("USD"),
        },
        Some("0.05"),
    )?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("5"));
    assert!(matches!(
        p.overview.net_annual_earnings,
        FigureDto::Partial { .. }
    ));
    assert!(matches!(p.overview.current_apy, FigureDto::Unknown { .. }));
    price(s, "ETH", "0")?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(p.holdings.len(), 2);
    assert!(known(&p.overview.current_value));
    assert!(matches!(p.overview.current_apy, FigureDto::Unknown { .. }));
    rate(
        s,
        "eth",
        RateScope::AccountAsset {
            account: "a".into(),
            asset: AssetId::str("ETH"),
        },
        Some("0"),
    )?;
    let p = app.portfolio_projection(req())?;
    assert!(known(&p.overview.current_apy));
    pos(s, "a", "debt", "USD", 2, PositionRole::Liability, "25")?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.current_value), decimal("75"));
    assert!(matches!(
        p.overview.annual_borrowing_costs,
        FigureDto::Unknown { .. }
    ));
    assert!(matches!(
        p.overview.net_annual_earnings,
        FigureDto::Partial { .. }
    ));
    assert!(matches!(p.overview.current_apy, FigureDto::Unknown { .. }));
    Ok(())
}
#[test]
fn aggregate_override_is_once_pinned_and_children_unattributed() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "nexo")?;
    let a = pos(s, "a", "one", "USD", 2, PositionRole::Asset, "100")?;
    let b = pos(s, "a", "two", "USD", 2, PositionRole::Asset, "100")?;
    product(s, "earn", "nexo")?;
    sub(s, "one", "a", "earn", a, "USD")?;
    sub(s, "two", "a", "earn", b, "USD")?;
    rate(
        s,
        "one",
        RateScope::Subscription("one".into()),
        Some("0.05"),
    )?;
    let scope = RateScope::ProviderAsset {
        provider: "nexo".into(),
        asset: AssetId::str("USD"),
    };
    rate(s, "aggregate", scope.clone(), Some("0.1"))?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("5"));
    assert!(matches!(
        p.overview.net_annual_earnings,
        FigureDto::Partial { .. }
    ));
    s.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &LocalTarget::Rate {
            scope: scope.clone(),
        },
        0,
        Some(LocalValue::Rates(vec![ManualRate {
            effective_at: t(1),
            updated_at: t(3),
            rate: Some(AnnualRate {
                fraction: "0.07".parse()?,
                basis: AnnualRateBasis::Apy,
                compounds_per_year: None,
                reward_asset: None,
            }),
            pinned: true,
        }])),
    )?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("14"));
    assert!(known(&p.overview.current_apy));
    assert!(
        p.holdings
            .iter()
            .all(|h| matches!(h.annual_earnings, FigureDto::Unknown { .. }))
    );
    assert_eq!(p.contributions.len(), 1);
    rate(
        s,
        "two",
        RateScope::Subscription("two".into()),
        Some("0.09"),
    )?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("14"));
    assert!(
        p.issues
            .iter()
            .any(|i| i.code == "aggregate_override_review")
    );
    Ok(())
}
#[test]
fn independent_balances_mismatch_scope_cutoffs_and_aggregate_no_splits() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "kraken")?;
    let one = pos(s, "a", "one", "USD", 2, PositionRole::Asset, "100")?;
    let two = pos(s, "a", "two", "USD", 2, PositionRole::Asset, "50")?;
    let obs = BalanceEvidence {
        id: "trusted".into(),
        account: "a".into(),
        position: Some(one.clone()),
        asset: AssetId::str("USD"),
        units: 12500.into(),
        scale: 2,
        basis: BalanceBasis::Total,
        cutoff: t(19),
        complete: true,
        independent: true,
        evidence: source(19),
    };
    s.save_balance_evidence(&obs)?;
    s.save_balance_evidence(&BalanceEvidence {
        id: "available".into(),
        position: Some(two.clone()),
        units: 999999.into(),
        basis: BalanceBasis::Available,
        ..obs.clone()
    })?;
    s.save_verification(&VerificationRecord {
        id: "mismatch".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        account: "a".into(),
        cutoff: t(19),
        observations: vec![obs.id.clone()],
        revisions: s.revisions(&DEFAULT_PORTFOLIO.into())?,
        outcome: VerificationOutcome::Mismatch,
        history_complete: false,
        issues: vec![VerificationIssue {
            code: "ledger_mismatch".into(),
            position: Some(one.clone()),
            message: "Imported history disagrees with independent total".into(),
            expected: Some(10000.into()),
            actual: Some(12500.into()),
            withheld: vec!["historical_value".into(), "historical_earnings".into()],
        }],
    })?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.current_value), decimal("175"));
    let observed = p
        .holdings
        .iter()
        .find(|h| h.position_ids.contains(&one))
        .unwrap();
    assert_eq!(observed.quantity.units, 12500);
    assert_eq!(observed.ledger_quantities[0].quantity.units, 10000);
    assert!(observed.issues.iter().any(|i| i.code == "ledger_mismatch"));
    assert!(observed.issues.iter().any(|i| i.code == "balance_as_of"));
    let other = p
        .holdings
        .iter()
        .find(|h| h.position_ids.contains(&two))
        .unwrap();
    assert!(known(&other.value));
    assert!(!other.issues.iter().any(|i| i.code == "ledger_mismatch"));
    s.save_balance_evidence(&BalanceEvidence {
        id: "total".into(),
        position: None,
        units: 20000.into(),
        cutoff: t(20),
        evidence: source(20),
        ..obs.clone()
    })?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(p.holdings.len(), 1);
    assert_eq!(amount(&p.overview.current_value), decimal("200"));
    assert_eq!(p.holdings[0].ledger_quantities.len(), 2);
    assert!(p.holdings[0].subscription.is_none());
    assert!(
        p.holdings[0]
            .issues
            .iter()
            .any(|i| i.code == "allocation_unattributed")
    );
    // Later partial/locked/available evidence must not replace complete totals.
    s.save_balance_evidence(&BalanceEvidence {
        id: "incomplete".into(),
        position: None,
        units: 999.into(),
        cutoff: t(20),
        complete: false,
        ..obs
    })?;
    let again = app.portfolio_projection(req())?;
    assert_eq!(again.holdings[0].id, p.holdings[0].id);
    assert_eq!(amount(&again.overview.current_value), decimal("200"));
    Ok(())
}
#[test]
fn verified_balance_does_not_imply_complete_history_and_stale_is_visible() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "kraken")?;
    let one = pos(s, "a", "one", "USD", 2, PositionRole::Asset, "100")?;
    s.save_balance_evidence(&BalanceEvidence {
        id: "obs".into(),
        account: "a".into(),
        position: Some(one),
        asset: AssetId::str("USD"),
        units: 10000.into(),
        scale: 2,
        basis: BalanceBasis::Total,
        cutoff: t(20),
        complete: true,
        independent: true,
        evidence: source(20),
    })?;
    s.save_verification(&VerificationRecord {
        id: "verified".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        account: "a".into(),
        cutoff: t(20),
        observations: vec!["obs".into()],
        revisions: s.revisions(&DEFAULT_PORTFOLIO.into())?,
        outcome: VerificationOutcome::Verified,
        history_complete: false,
        issues: vec![],
    })?;
    let p = app.portfolio_projection(req())?;
    assert!(p.holdings[0].verification_current);
    assert!(p.issues.iter().any(|i| i.code == "history_unverified"));
    pos(s, "a", "other", "USD", 2, PositionRole::Asset, "1")?;
    let p = app.portfolio_projection(req())?;
    assert!(p.issues.iter().any(|i| i.code == "verification_stale"));
    Ok(())
}
#[test]
fn exact_huge_values_compounded_apr_rational_apy_and_wire_roundtrip() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "nexo")?;
    let amount_text = "123456789012345678901234567890.000000000000000001";
    pos(
        s,
        "a",
        "huge",
        "TOKEN",
        18,
        PositionRole::Asset,
        amount_text,
    )?;
    price(s, "TOKEN", "0.000000000000000001")?;
    let scope = RateScope::AccountAsset {
        account: "a".into(),
        asset: AssetId::str("TOKEN"),
    };
    rate(s, "token", scope.clone(), Some("0.05"))?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(
        amount(&p.overview.current_value),
        decimal(amount_text).mul(&decimal("0.000000000000000001"))
    );
    let json = serde_json::to_string(&p)?;
    assert_eq!(serde_json::from_str::<PortfolioProjection>(&json)?, p);
    // 12% APR compounded twice => 12.36% exactly, not 12% or float noise.
    s.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &LocalTarget::Rate { scope },
        0,
        Some(LocalValue::Rates(vec![ManualRate {
            effective_at: t(3),
            updated_at: t(3),
            rate: Some(AnnualRate {
                fraction: "0.12".parse()?,
                basis: AnnualRateBasis::Apr,
                compounds_per_year: Some(2),
                reward_asset: None,
            }),
            pinned: true,
        }])),
    )?;
    let p = app.portfolio_projection(req())?;
    assert!(
        matches!(p.overview.current_apy,FigureDto::Known{ref value,..} if *value==decimal("0.1236"))
    );
    // Ratio 1/3 has no finite decimal; retain it exactly on the wire.
    let third = ExactRatio::integer(1).div(&ExactRatio::integer(3)).unwrap();
    assert_eq!(
        serde_json::to_string(&third)?,
        r#"{"numerator":"1","denominator":"3"}"#
    );
    for bad in [
        r#"{"numerator":1,"denominator":"3"}"#,
        r#"{"numerator":"1","denominator":"0"}"#,
        r#"{"numerator":"1","denominator":"-1"}"#,
        r#"{"numerator":"1e3","denominator":"1"}"#,
    ] {
        assert!(serde_json::from_str::<ExactRatio>(bad).is_err());
    }
    Ok(())
}
#[test]
fn isolation_spam_local_names_dates_and_no_closed_zero_subscription_loss() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "nexo")?;
    let one = pos(s, "a", "one", "USD", 2, PositionRole::Asset, "100")?;
    let zero = pos(s, "a", "zero", "USD", 2, PositionRole::Asset, "0")?;
    product(s, "earn", "nexo")?;
    sub(s, "one", "a", "earn", one, "USD")?;
    sub(s, "zero", "a", "earn", zero, "USD")?;
    rate(
        s,
        "old",
        RateScope::Subscription("one".into()),
        Some("0.05"),
    )?;
    s.save_rate_evidence(&RateEvidence {
        id: "new".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        scope: RateScope::Subscription("one".into()),
        effective_at: t(15),
        rate: Some(AnnualRate {
            fraction: "0.07".parse()?,
            basis: AnnualRateBasis::Apy,
            compounds_per_year: None,
            reward_asset: Some(AssetId::str("REWARD")),
        }),
        evidence: source(15),
    })?;
    s.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &LocalTarget::Account { id: "a".into() },
        0,
        Some(LocalValue::Account(AccountCustomization {
            local_label: Some("My savings".into()),
            ..Default::default()
        })),
    )?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(p.holdings.len(), 1);
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("7"));
    let dt = p.details(AllocationTarget::Account("a".into()))?;
    assert_eq!(dt.node.name, "My savings");
    assert_eq!(dt.original_account.unwrap().label, "Source a");
    let mut earlier = req();
    earlier.context.cutoff = t(10);
    assert_eq!(
        amount(&app.overview(earlier)?.net_annual_earnings),
        decimal("5")
    );
    let mut closed = s.subscription("one")?.unwrap();
    closed.closed_at = Some(t(19));
    s.save_subscription(&closed)?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(p.holdings.len(), 1);
    assert!(p.holdings[0].subscription.is_none());
    assert_eq!(amount(&p.overview.current_value), decimal("100"));
    s.save_portfolio(&Portfolio {
        id: "other".into(),
        label: "Other".into(),
        reporting_asset: AssetId::str("USD"),
    })?;
    let a = Account {
        id: "other-a".into(),
        portfolio_id: "other".into(),
        provider_id: "nexo".into(),
        label: "Other".into(),
        source_reference: None,
    };
    s.save_account(&a)?;
    pos(s, "other-a", "one", "USD", 2, PositionRole::Asset, "999")?;
    assert_eq!(amount(&app.overview(req())?.current_value), decimal("100"));
    s.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &LocalTarget::Spam {
            asset: AssetId::str("USD"),
        },
        0,
        Some(LocalValue::Spam {
            excluded: true,
            reason: "fixture exclusion".into(),
        }),
    )?;
    assert!(app.portfolio_projection(req())?.holdings.is_empty());
    let mut other = req();
    other.context.portfolio_id = "other".into();
    assert_eq!(amount(&app.overview(other)?.current_value), decimal("999"));
    assert_eq!(s.account_audit_balances_at(&"a".into(), t(20))?.len(), 2);
    Ok(())
}
#[test]
fn wholly_unknown_empty_zero_gross_and_mismatch_without_observation() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    let empty = app.portfolio_projection(req())?;
    assert_eq!(amount(&empty.overview.current_value), ExactRatio::zero());
    assert!(matches!(
        empty.overview.current_apy,
        FigureDto::NotApplicable
    ));
    assert!(empty.allocation.nodes.is_empty());
    account(s, "a", "nexo")?;
    let id = pos(s, "a", "one", "TOKEN", 18, PositionRole::Asset, "1")?;
    let p = app.portfolio_projection(req())?;
    assert!(matches!(
        p.overview.current_value,
        FigureDto::Unknown { .. }
    ));
    assert!(matches!(
        p.overview.net_annual_earnings,
        FigureDto::Unknown { .. }
    ));
    price(s, "TOKEN", "0")?;
    rate(
        s,
        "token",
        RateScope::AccountAsset {
            account: "a".into(),
            asset: AssetId::str("TOKEN"),
        },
        Some("0.05"),
    )?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(p.holdings.len(), 1);
    assert!(matches!(p.overview.current_apy, FigureDto::NotApplicable));
    s.save_verification(&VerificationRecord {
        id: "mismatch".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        account: "a".into(),
        cutoff: t(20),
        observations: vec![],
        revisions: s.revisions(&DEFAULT_PORTFOLIO.into())?,
        outcome: VerificationOutcome::Mismatch,
        history_complete: false,
        issues: vec![VerificationIssue {
            code: "missing_history".into(),
            position: Some(id),
            message: "No reconciled current quantity".into(),
            expected: None,
            actual: None,
            withheld: vec![],
        }],
    })?;
    let p = app.portfolio_projection(req())?;
    assert!(matches!(
        p.overview.current_value,
        FigureDto::Unknown { .. }
    ));
    assert!(matches!(
        p.overview.net_annual_earnings,
        FigureDto::Unknown { .. }
    ));
    assert!(
        p.issues
            .iter()
            .any(|i| i.code == "balance_mismatch_unobserved")
    );
    let mut invalid = req();
    invalid.dust_threshold = "-1".parse()?;
    assert!(app.portfolio_projection(invalid).is_err());
    let mut invalid = req();
    invalid.context.portfolio_id = "missing".into();
    assert!(app.portfolio_projection(invalid).is_err());
    // A failed read rolls back its transaction; the next read remains usable.
    assert!(app.portfolio_projection(req()).is_ok());
    Ok(())
}
#[test]
fn unsupported_compounding_and_aggregate_observation_identity_are_explicit() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "kraken")?;
    pos(s, "a", "one", "USD", 2, PositionRole::Asset, "100")?;
    pos(s, "a", "two", "USD", 2, PositionRole::Asset, "50")?;
    let scope = RateScope::AccountAsset {
        account: "a".into(),
        asset: AssetId::str("USD"),
    };
    s.save_rate_evidence(&RateEvidence {
        id: "hourly".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        scope,
        effective_at: t(1),
        rate: Some(AnnualRate {
            fraction: "0.12".parse()?,
            basis: AnnualRateBasis::Apr,
            compounds_per_year: Some(8760),
            reward_asset: None,
        }),
        evidence: source(2),
    })?;
    let p = app.portfolio_projection(req())?;
    assert!(matches!(
        p.overview.net_annual_earnings,
        FigureDto::Unknown { .. }
    ));
    assert!(p.issues.iter().any(|i| i.code == "compounding_unsupported"));
    let obs = BalanceEvidence {
        id: "total1".into(),
        account: "a".into(),
        position: None,
        asset: AssetId::str("USD"),
        units: 16000.into(),
        scale: 2,
        basis: BalanceBasis::Total,
        cutoff: t(19),
        complete: true,
        independent: true,
        evidence: source(19),
    };
    s.save_balance_evidence(&obs)?;
    let p = app.portfolio_projection(req())?;
    let id = p.holdings[0].id.clone();
    s.save_balance_evidence(&BalanceEvidence {
        id: "total2".into(),
        units: 17000.into(),
        cutoff: t(20),
        evidence: source(20),
        ..obs
    })?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(p.holdings[0].id, id);
    assert_eq!(amount(&p.overview.current_value), decimal("170"));
    Ok(())
}
#[test]
fn manual_quote_pin_reporting_currency_and_simple_apr_are_applied_exactly() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "nexo")?;
    pos(s, "a", "one", "TOKEN", 18, PositionRole::Asset, "2")?;
    let scope = RateScope::AccountAsset {
        account: "a".into(),
        asset: AssetId::str("TOKEN"),
    };
    s.save_rate_evidence(&RateEvidence {
        id: "simple-apr".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        scope,
        effective_at: t(1),
        rate: Some(AnnualRate {
            fraction: "0.05".parse()?,
            basis: AnnualRateBasis::Apr,
            compounds_per_year: None,
            reward_asset: None,
        }),
        evidence: source(2),
    })?;
    let target = LocalTarget::Price {
        asset: AssetId::str("TOKEN"),
        quote: AssetId::str("USD"),
    };
    let action = s.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &target,
        0,
        Some(LocalValue::Prices(vec![ManualPrice {
            effective_at: t(3),
            updated_at: t(3),
            price: "12".parse()?,
            pinned: true,
        }])),
    )?;
    let p = app.portfolio_projection(req())?;
    assert_eq!(amount(&p.overview.current_value), decimal("24"));
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("1.2"));
    assert!(!p.issues.iter().any(|i| i.code == "price_unknown"));
    assert!(
        matches!(&p.holdings[0].value,FigureDto::Known{evidence,..} if evidence.iter().any(|e|matches!(e,EvidenceDto::Manual{action_id,..} if *action_id==action.id)))
    );
    s.save_price_evidence(&PriceEvidence {
        id: "new-provider".into(),
        asset: AssetId::str("TOKEN"),
        quote: AssetId::str("USD"),
        as_of: t(19),
        price: "20".parse()?,
        evidence: source(19),
    })?;
    assert_eq!(amount(&app.overview(req())?.current_value), decimal("24"));
    s.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &target,
        1,
        Some(LocalValue::Prices(vec![ManualPrice {
            effective_at: t(3),
            updated_at: t(3),
            price: "12".parse()?,
            pinned: false,
        }])),
    )?;
    assert_eq!(amount(&app.overview(req())?.current_value), decimal("40"));
    let mut eur = req();
    eur.reporting_asset = AssetId::str("EUR");
    assert!(matches!(
        app.overview(eur.clone())?.current_value,
        FigureDto::Unknown { .. }
    ));
    s.save_price_evidence(&PriceEvidence {
        id: "eur".into(),
        asset: AssetId::str("TOKEN"),
        quote: AssetId::str("EUR"),
        as_of: t(19),
        price: "11".parse()?,
        evidence: source(19),
    })?;
    let p = app.portfolio_projection(eur)?;
    assert_eq!(amount(&p.overview.current_value), decimal("22"));
    assert_eq!(amount(&p.overview.net_annual_earnings), decimal("1.1"));
    // Float price rows never become exact quote evidence.
    s.save_prices(&[AssetPricePoint {
        datetime: t(19),
        asset_id: AssetId::str("TOKEN"),
        vs_asset_id: AssetId::str("GBP"),
        price: 9.0,
    }])?;
    let mut gbp = req();
    gbp.reporting_asset = AssetId::str("GBP");
    assert!(matches!(
        app.overview(gbp)?.current_value,
        FigureDto::Unknown { .. }
    ));
    Ok(())
}
#[test]
fn explicit_source_withheld_figures_are_honored_even_with_known_rates() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let s = app.store();
    account(s, "a", "kraken")?;
    let id = pos(s, "a", "one", "USD", 2, PositionRole::Asset, "100")?;
    rate(
        s,
        "usd",
        RateScope::AccountAsset {
            account: "a".into(),
            asset: AssetId::str("USD"),
        },
        Some("0.05"),
    )?;
    let r = VerificationRecord {
        id: "rates-pending".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        account: "a".into(),
        cutoff: t(20),
        observations: vec![],
        revisions: s.revisions(&DEFAULT_PORTFOLIO.into())?,
        outcome: VerificationOutcome::Incomplete,
        history_complete: false,
        issues: vec![VerificationIssue {
            code: "coverage_pending".into(),
            position: Some(id),
            message: "Provider has not established required coverage".into(),
            expected: None,
            actual: None,
            withheld: vec!["overall_apy".into()],
        }],
    };
    s.save_verification(&r)?;
    let p = app.portfolio_projection(req())?;
    assert!(known(&p.overview.current_value));
    assert!(known(&p.overview.net_annual_earnings));
    assert!(matches!(p.overview.current_apy, FigureDto::Unknown { .. }));
    let mut next = r;
    next.id = "value-pending".into();
    next.issues[0].withheld = vec!["current_value".into()];
    s.save_verification(&next)?;
    let p = app.portfolio_projection(req())?;
    assert!(matches!(
        p.overview.current_value,
        FigureDto::Unknown { .. }
    ));
    assert!(matches!(
        p.overview.net_annual_earnings,
        FigureDto::Unknown { .. }
    ));
    Ok(())
}
