use anyhow::Result;
use chrono::{DateTime, Utc};
use lib_core::{ui_metadata::*, *};

fn time(day: u32) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&format!("2026-01-{day:02}T00:00:00Z"))
        .unwrap()
        .to_utc()
}
fn account(id: &str, portfolio: &str, provider: &str) -> Account {
    Account {
        id: id.into(),
        portfolio_id: portfolio.into(),
        provider_id: provider.into(),
        label: format!("Source {id}"),
        source_reference: Some(id.into()),
    }
}
fn fixture(store: &Store) -> Result<()> {
    store.save_portfolio(&Portfolio {
        id: "other".into(),
        label: "Other".into(),
        reporting_asset: AssetId::str("USD"),
    })?;
    for a in [
        account("a", DEFAULT_PORTFOLIO, "nexo"),
        account("b", DEFAULT_PORTFOLIO, "nexo"),
        account("c", DEFAULT_PORTFOLIO, "kraken"),
        account("other", "other", "nexo"),
    ] {
        store.save_account(&a)?;
    }
    store.save_asset_scale(&AssetId::str("BTC"), 8)?;
    store.save_asset_scale(&AssetId::str("USD"), 2)?;
    Ok(())
}
fn position(
    store: &Store,
    account: &str,
    id: &str,
    asset: &str,
    role: PositionRole,
    amount: Units,
) -> Result<PositionId> {
    let local: PositionId = id.into();
    let tx = Transaction {
        datetime: time(1),
        inputs: if role == PositionRole::Liability {
            vec![TxnEffect {
                position_id: local.clone(),
                amount: amount.clone(),
                datetime: time(1),
            }]
        } else {
            vec![]
        },
        outputs: if role == PositionRole::Asset {
            vec![TxnEffect {
                position_id: local.clone(),
                amount,
                datetime: time(1),
            }]
        } else {
            vec![]
        },
    };
    store.save_account_import(
        &account.into(),
        "fixture",
        id,
        &[(
            local.clone(),
            AssetId::str(asset),
            if asset == "USD" { 2 } else { 8 },
            role,
        )],
        &[(id.into(), tx)],
        &[],
    )?;
    Ok(Store::account_position_id(&account.into(), &local))
}
fn product() -> ProductTemplate {
    ProductTemplate {
        id: "term".into(),
        provider_id: "nexo".into(),
        source_reference: "term-source".into(),
        label: "Fixed term".into(),
        evidence: "synthetic provider metadata".into(),
    }
}
fn subscription(id: &str, position: PositionId) -> Subscription {
    Subscription {
        id: id.into(),
        account_id: "a".into(),
        product_id: "term".into(),
        base_asset: AssetId::str("BTC"),
        source_reference: id.into(),
        label: format!("Subscription {id}"),
        opened_at: Some(time(1)),
        closed_at: None,
        lock_status: LockStatus::Locked,
        maturity: Some(time(30)),
        auto_renew: Some(true),
        positions: vec![position],
    }
}
fn annual(fraction: &str) -> AnnualRate {
    AnnualRate {
        fraction: fraction.parse().unwrap(),
        basis: AnnualRateBasis::Apy,
        compounds_per_year: None,
        reward_asset: None,
    }
}
fn evidence(day: u32) -> SourceEvidence {
    SourceEvidence {
        source: "fixture".into(),
        reference: format!("snapshot-{day}"),
        observed_at: time(day),
        updated_at: time(day),
    }
}
fn rate(id: &str, scope: RateScope, day: u32, fraction: Option<&str>) -> RateEvidence {
    RateEvidence {
        id: id.into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        scope,
        effective_at: time(day),
        rate: fraction.map(annual),
        evidence: evidence(day),
    }
}

#[test]
fn hierarchy_preserves_source_and_customizations_and_rejects_invalid_parents() -> Result<()> {
    let store = Store::in_memory()?;
    fixture(&store)?;
    let group = account("group", DEFAULT_PORTFOLIO, "nexo");
    store.save_group_account(&group, None, 1)?;
    assert_eq!(store.account_structure(&group.id)?.kind, NodeKind::Group);
    store.save_account_structure(&AccountStructure {
        account_id: "a".into(),
        kind: NodeKind::Source,
        source_parent: Some(group.id.clone()),
        mapping_version: 1,
    })?;
    assert_eq!(store.account_parent(&"a".into())?, Some(group.id.clone()));
    let target = LocalTarget::Account { id: "a".into() };
    let action = store.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &target,
        0,
        Some(LocalValue::Account(AccountCustomization {
            local_label: Some("My account".into()),
            parent_is_custom: true,
            parent: None,
        })),
    )?;
    store.save_account_structure(&AccountStructure {
        account_id: "a".into(),
        kind: NodeKind::Source,
        source_parent: Some("b".into()),
        mapping_version: 2,
    })?;
    store.save_account(&Account {
        label: "Refreshed source name".into(),
        ..account("a", DEFAULT_PORTFOLIO, "nexo")
    })?;
    assert_eq!(store.account_parent(&"a".into())?, None);
    assert_eq!(
        store.local_state(&DEFAULT_PORTFOLIO.into(), &target)?.value,
        action.after.value
    );
    store.undo_manual_action(&DEFAULT_PORTFOLIO.into(), &action.id)?;
    assert_eq!(store.account_parent(&"a".into())?, Some("b".into()));
    let cyclic = AccountStructure {
        account_id: "b".into(),
        kind: NodeKind::Source,
        source_parent: Some("a".into()),
        mapping_version: 1,
    };
    assert!(store.save_account_structure(&cyclic).is_err());
    for parent in ["a", "c", "other", "missing"] {
        let value = LocalValue::Account(AccountCustomization {
            parent_is_custom: true,
            parent: Some(parent.into()),
            ..Default::default()
        });
        assert!(
            store
                .apply_manual_change(&DEFAULT_PORTFOLIO.into(), &target, 2, Some(value))
                .is_err()
        );
    }
    assert!(
        store
            .save_group_account(&account("a", DEFAULT_PORTFOLIO, "nexo"), None, 1)
            .is_err()
    );
    // Restoring a former source parent is revalidated against today's tree.
    let root_override = store.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &target,
        2,
        Some(LocalValue::Account(AccountCustomization {
            parent_is_custom: true,
            parent: None,
            ..Default::default()
        })),
    )?;
    store.save_account_structure(&AccountStructure {
        account_id: "b".into(),
        kind: NodeKind::Source,
        source_parent: Some("a".into()),
        mapping_version: 1,
    })?;
    assert!(
        store
            .undo_manual_action(&DEFAULT_PORTFOLIO.into(), &root_override.id)
            .is_err()
    );
    store.validate_account_tree(&DEFAULT_PORTFOLIO.into())?;

    Ok(())
}

#[test]
fn subscriptions_are_instances_and_positions_cannot_be_counted_twice() -> Result<()> {
    let store = Store::in_memory()?;
    fixture(&store)?;
    let a = position(&store, "a", "one", "BTC", PositionRole::Asset, 100.into())?;
    let b = position(&store, "a", "two", "BTC", PositionRole::Asset, 200.into())?;
    store.save_product_template(&product())?;
    store.save_subscription(&subscription("first", a.clone()))?;
    store.save_subscription(&subscription("second", b))?;
    assert!(
        store
            .save_subscription(&subscription("duplicate", a))
            .is_err()
    );
    assert_eq!(store.subscriptions(&DEFAULT_PORTFOLIO.into())?.len(), 2);
    assert!(store.subscriptions(&"other".into())?.is_empty());
    store.save_rate_evidence(&rate(
        "first-opening",
        RateScope::Subscription("first".into()),
        1,
        Some("0.05"),
    ))?;
    store.save_rate_evidence(&rate(
        "second-opening",
        RateScope::Subscription("second".into()),
        1,
        Some("0.06"),
    ))?;
    store.save_rate_evidence(&rate(
        "first-later",
        RateScope::Subscription("first".into()),
        2,
        Some("0.07"),
    ))?;
    let selected = store
        .selected_rate(
            &DEFAULT_PORTFOLIO.into(),
            &RateScope::Subscription("first".into()),
            time(3),
        )?
        .unwrap();
    assert_eq!(selected.rate.unwrap().fraction.as_str(), "0.07");
    assert_eq!(
        store
            .selected_rate(
                &DEFAULT_PORTFOLIO.into(),
                &RateScope::Subscription("second".into()),
                time(3)
            )?
            .unwrap()
            .rate
            .unwrap()
            .fraction
            .as_str(),
        "0.06"
    );
    let mut conflict = subscription(
        "first",
        position(&store, "b", "other", "BTC", PositionRole::Asset, 5.into())?,
    );
    conflict.account_id = "b".into();
    assert!(store.save_subscription(&conflict).is_err());
    assert!(
        store
            .selected_rate(
                &"other".into(),
                &RateScope::Subscription("first".into()),
                time(3)
            )
            .is_err()
    );
    let aggregate = RateScope::Product("term".into());
    store.save_rate_evidence(&rate("held-product", aggregate.clone(), 1, None))?;
    assert!(
        store
            .selected_rate(&DEFAULT_PORTFOLIO.into(), &aggregate, time(3))?
            .unwrap()
            .rate
            .is_none()
    );

    Ok(())
}

#[test]
fn dated_rates_pins_unknowns_cached_replay_and_finer_data_review() -> Result<()> {
    let store = Store::in_memory()?;
    fixture(&store)?;
    let portfolio: PortfolioId = DEFAULT_PORTFOLIO.into();
    let scope = RateScope::Provider("nexo".into());
    let target = LocalTarget::Rate {
        scope: scope.clone(),
    };
    assert!(store.selected_rate(&portfolio, &scope, time(1))?.is_none());
    let first = rate("initial", scope.clone(), 1, Some("0.05"));
    store.save_rate_evidence(&first)?;
    let revisions = store.revisions(&portfolio)?;
    store.save_rate_evidence(&first)?;
    assert_eq!(store.revisions(&portfolio)?, revisions);
    let mut conflict = first.clone();
    conflict.rate = Some(annual("0.09"));
    assert!(store.save_rate_evidence(&conflict).is_err());
    store.apply_manual_change(
        &portfolio,
        &target,
        0,
        Some(LocalValue::Rates(vec![ManualRate {
            effective_at: time(1),
            updated_at: time(2),
            rate: Some(annual("0.08")),
            pinned: false,
        }])),
    )?;
    assert_eq!(
        store
            .selected_rate(&portfolio, &scope, time(2))?
            .unwrap()
            .rate
            .unwrap()
            .fraction
            .as_str(),
        "0.08"
    );
    store.save_rate_evidence(&rate("new-period", scope.clone(), 3, Some("0.06")))?;
    assert_eq!(
        store
            .selected_rate(&portfolio, &scope, time(4))?
            .unwrap()
            .rate
            .unwrap()
            .fraction
            .as_str(),
        "0.06"
    );
    let pinned = store.apply_manual_change(
        &portfolio,
        &target,
        1,
        Some(LocalValue::Rates(vec![ManualRate {
            effective_at: time(1),
            updated_at: time(5),
            rate: Some(annual("0.10")),
            pinned: true,
        }])),
    )?;
    assert!(
        store
            .selected_rate(&portfolio, &scope, time(6))?
            .unwrap()
            .pinned
    );
    assert!(!store.override_requires_review(&portfolio, &target)?);
    store.save_rate_evidence(&rate(
        "partial-provider-asset",
        RateScope::ProviderAsset {
            provider: "nexo".into(),
            asset: AssetId::str("BTC"),
        },
        5,
        None,
    ))?;
    assert!(store.override_requires_review(&portfolio, &target)?);

    store.save_product_template(&product())?;
    store.save_subscription(&subscription(
        "partial-finer",
        position(
            &store,
            "a",
            "partial",
            "BTC",
            PositionRole::Asset,
            100.into(),
        )?,
    ))?;
    assert!(store.override_requires_review(&portfolio, &target)?);
    assert!(
        store
            .selected_rate(&portfolio, &scope, time(6))?
            .unwrap()
            .pinned
    );
    assert!(store.undo_manual_action(&portfolio, &pinned.id).is_err());
    // A backdated correction cannot erase an existing later manual period.
    store.apply_manual_change(
        &portfolio,
        &target,
        2,
        Some(LocalValue::Rates(vec![
            ManualRate {
                effective_at: time(1),
                updated_at: time(8),
                rate: Some(annual("0.11")),
                pinned: false,
            },
            ManualRate {
                effective_at: time(7),
                updated_at: time(7),
                rate: None,
                pinned: false,
            },
        ])),
    )?;
    assert!(
        store
            .selected_rate(&portfolio, &scope, time(9))?
            .unwrap()
            .rate
            .is_none()
    );
    assert_eq!(
        store
            .selected_rate(&portfolio, &scope, time(2))?
            .unwrap()
            .rate
            .unwrap()
            .fraction
            .as_str(),
        "0.11"
    );
    store.save_rate_evidence(&rate(
        "known-zero",
        RateScope::Asset(AssetId::str("USD")),
        1,
        Some("0"),
    ))?;
    assert_eq!(
        store
            .selected_rate(&portfolio, &RateScope::Asset(AssetId::str("USD")), time(9))?
            .unwrap()
            .rate
            .unwrap()
            .fraction
            .as_str(),
        "0"
    );
    Ok(())
}

#[test]
fn price_evidence_stays_exact_and_manual_prices_do_not_rewrite_history() -> Result<()> {
    let store = Store::in_memory()?;
    fixture(&store)?;
    let portfolio = DEFAULT_PORTFOLIO.into();
    let asset = AssetId::str("BTC");
    let quote = AssetId::str("USD");
    let p = PriceEvidence {
        id: "price1".into(),
        asset: asset.clone(),
        quote: quote.clone(),
        as_of: time(1),
        price: "12345678901234567890.000000000000000001".parse()?,
        evidence: evidence(1),
    };
    store.save_price_evidence(&p)?;
    store.save_price_evidence(&p)?;
    let target = LocalTarget::Price {
        asset: asset.clone(),
        quote: quote.clone(),
    };
    store.apply_manual_change(
        &portfolio,
        &target,
        0,
        Some(LocalValue::Prices(vec![ManualPrice {
            effective_at: time(2),
            updated_at: time(2),
            price: "0".parse()?,
            pinned: true,
        }])),
    )?;
    store.save_price_evidence(&PriceEvidence {
        id: "price3".into(),
        as_of: time(3),
        evidence: evidence(3),
        ..p.clone()
    })?;
    let selected = store
        .selected_price(&portfolio, &asset, &quote, time(4))?
        .unwrap();
    assert!(selected.pinned);
    assert_eq!(selected.price.as_str(), "0");
    assert_eq!(store.price_evidence(&asset, &quote)?[0], p);
    assert!(
        store
            .selected_price(&"other".into(), &asset, &quote, time(4))?
            .unwrap()
            .price
            .as_str()
            .starts_with("123456789")
    );
    assert_eq!(serde_json::to_value(&p)?["price"], p.price.as_str());
    Ok(())
}

#[test]
fn spam_follows_exact_asset_across_future_accounts_but_never_hides_debt_or_changes_audit()
-> Result<()> {
    let store = Store::in_memory()?;
    fixture(&store)?;
    position(&store, "a", "asset", "USD", PositionRole::Asset, 100.into())?;
    let debt = position(
        &store,
        "a",
        "debt",
        "USD",
        PositionRole::Liability,
        20.into(),
    )?;
    position(
        &store,
        "other",
        "asset",
        "USD",
        PositionRole::Asset,
        100.into(),
    )?;
    let before = store.account_audit_balances_at(&"a".into(), time(2))?;
    let portfolio = DEFAULT_PORTFOLIO.into();
    let target = LocalTarget::Spam {
        asset: AssetId::str("USD"),
    };
    let action = store.apply_manual_change(
        &portfolio,
        &target,
        0,
        Some(LocalValue::Spam {
            excluded: true,
            reason: "synthetic reviewed exclusion".into(),
        }),
    )?;
    let tracked = store.account_balances_at(&"a".into(), time(2))?;
    assert_eq!(tracked.len(), 1);
    assert_eq!(tracked[0].position_id, debt);
    assert_eq!(
        store.account_audit_balances_at(&"a".into(), time(2))?,
        before
    );
    store.save_account(&account("new", DEFAULT_PORTFOLIO, "kraken"))?;
    position(
        &store,
        "new",
        "asset",
        "USD",
        PositionRole::Asset,
        500.into(),
    )?;
    assert!(
        store
            .account_balances_at(&"new".into(), time(2))?
            .is_empty()
    );
    assert_eq!(
        store.account_balances_at(&"other".into(), time(2))?.len(),
        1
    );
    let dust = LocalTarget::Dust {
        provider: "nexo".into(),
        asset: AssetId::str("USD"),
    };
    store.apply_manual_change(&portfolio, &dust, 0, Some(LocalValue::Dust(DustMode::Dust)))?;
    assert_eq!(
        store.account_audit_balances_at(&"a".into(), time(2))?,
        before
    );
    store.undo_manual_action(&portfolio, &action.id)?;
    assert_eq!(store.account_balances_at(&"a".into(), time(2))?, before);
    assert_eq!(store.account_balances_at(&"new".into(), time(2))?.len(), 1);
    Ok(())
}

#[test]
fn action_log_and_undo_survive_restart_and_refuse_newer_local_edits() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("ui.duckdb");
    let portfolio: PortfolioId = DEFAULT_PORTFOLIO.into();
    let target = LocalTarget::Account { id: "a".into() };
    let action;
    {
        let store = Store::open(&path)?;
        fixture(&store)?;
        action = store.apply_manual_change(
            &portfolio,
            &target,
            0,
            Some(LocalValue::Account(AccountCustomization {
                local_label: Some("Mine".into()),
                ..Default::default()
            })),
        )?;
        assert!(
            store
                .apply_manual_change(&portfolio, &target, 0, None)
                .is_err()
        );
    }
    for _ in 0..2 {
        let store = Store::open(&path)?;
        assert_eq!(store.manual_actions(&portfolio)?, vec![action.clone()]);
    }
    let store = Store::open(&path)?;
    let undo = store.undo_manual_action(&portfolio, &action.id)?;
    assert_eq!(undo.undo_of, Some(action.id.clone()));
    assert!(undo.after.value.is_none());
    assert_eq!(store.local_state(&portfolio, &target)?.revision, 2);
    assert!(store.undo_manual_action(&portfolio, &action.id).is_err());
    assert!(store.undo_manual_action(&"other".into(), &undo.id).is_err());
    let older = store.apply_manual_change(
        &portfolio,
        &target,
        2,
        Some(LocalValue::Account(AccountCustomization {
            local_label: Some("Older".into()),
            ..Default::default()
        })),
    )?;
    store.apply_manual_change(
        &portfolio,
        &target,
        3,
        Some(LocalValue::Account(AccountCustomization {
            local_label: Some("Newer".into()),
            ..Default::default()
        })),
    )?;
    assert!(store.undo_manual_action(&portfolio, &older.id).is_err());
    assert_eq!(store.manual_actions(&portfolio)?.len(), 4);
    drop(store);
    assert_eq!(Store::open(&path)?.manual_actions(&portfolio)?.len(), 4);
    Ok(())
}

#[test]
fn alias_ambiguity_and_locators_do_not_merge_assets_or_change_scales() -> Result<()> {
    let store = Store::in_memory()?;
    for id in ["chain1:0x1", "chain2:0x1"] {
        store.save_asset_scale(&AssetId::str(id), 18)?;
        store.save_asset_display(&AssetDisplay {
            id: AssetId::str(id),
            name: None,
            ticker: Some("SAME".into()),
            chain: Some(id.into()),
            contract: Some("0x1".into()),
            unresolved_label: None,
            evidence: "synthetic locator".into(),
        })?;
        store.save_asset_alias(&AssetAlias {
            provider: "exchange".into(),
            namespace: "asset-code".into(),
            external_id: "SAME".into(),
            asset_id: AssetId::str(id),
            evidence: "candidate mapping".into(),
        })?;
    }
    assert_eq!(
        store
            .asset_aliases(&"exchange".into(), "asset-code", "SAME")?
            .len(),
        2
    );
    assert!(
        store
            .asset_aliases(&"other".into(), "asset-code", "SAME")?
            .is_empty()
    );
    let mut invalid = store.asset_display(&AssetId::str("chain1:0x1"))?.unwrap();
    invalid.chain = Some("different".into());
    assert!(store.save_asset_display(&invalid).is_err());
    assert!(
        store
            .save_asset_scale(&AssetId::str("chain1:0x1"), 8)
            .is_err()
    );
    Ok(())
}

#[test]
fn verification_is_exact_scoped_and_stale_after_ledger_or_policy_changes() -> Result<()> {
    let store = Store::in_memory()?;
    fixture(&store)?;
    let pos = position(
        &store,
        "a",
        "asset",
        "BTC",
        PositionRole::Asset,
        "900719925474099300000".parse()?,
    )?;
    let observation = BalanceEvidence {
        id: "balance".into(),
        account: "a".into(),
        position: Some(pos.clone()),
        asset: AssetId::str("BTC"),
        units: "900719925474099300000".parse()?,
        scale: 8,
        basis: BalanceBasis::Total,
        cutoff: time(2),
        complete: true,
        independent: true,
        evidence: evidence(2),
    };
    store.save_balance_evidence(&observation)?;
    let portfolio = DEFAULT_PORTFOLIO.into();
    let verification = VerificationRecord {
        id: "verify".into(),
        portfolio_id: portfolio,
        account: "a".into(),
        cutoff: time(2),
        observations: vec!["balance".into()],
        revisions: store.revisions(&DEFAULT_PORTFOLIO.into())?,
        outcome: VerificationOutcome::Verified,
        history_complete: false,
        issues: vec![],
    };
    store.save_verification(&verification)?;
    assert!(store.verification_is_current(&verification)?);
    let mut bad = observation.clone();
    bad.id = "bad-balance".into();
    bad.units = 1.into();
    store.save_balance_evidence(&bad)?;
    assert!(
        store
            .save_verification(&VerificationRecord {
                id: "bad-verify".into(),
                observations: vec![bad.id],
                ..verification.clone()
            })
            .is_err()
    );
    assert!(
        store
            .save_verification(&VerificationRecord {
                id: "overlap".into(),
                observations: vec!["balance".into(), "balance".into()],
                ..verification.clone()
            })
            .is_err()
    );
    store.apply_manual_change(
        &DEFAULT_PORTFOLIO.into(),
        &LocalTarget::Spam {
            asset: AssetId::str("BTC"),
        },
        0,
        Some(LocalValue::Spam {
            excluded: true,
            reason: "review".into(),
        }),
    )?;
    assert!(!store.verification_is_current(&verification)?);
    assert!(
        store
            .save_verification(&VerificationRecord {
                id: "stale".into(),
                ..verification.clone()
            })
            .is_err()
    );
    let after_policy = store.revisions(&DEFAULT_PORTFOLIO.into())?;
    store.save_transaction(
        "correction",
        &Transaction {
            datetime: time(3),
            inputs: vec![],
            outputs: vec![TxnEffect {
                position_id: pos,
                datetime: time(3),
                amount: 1.into(),
            }],
        },
    )?;
    assert!(store.revisions(&DEFAULT_PORTFOLIO.into())?.ledger > after_policy.ledger);
    let state = RefreshState {
        next_due: Some(time(4)),
        last_attempt: Some(time(3)),
        last_success: Some(time(2)),
    };
    store.save_refresh_state(&DEFAULT_PORTFOLIO.into(), "kraken", &state)?;
    assert_eq!(
        store.refresh_state(&DEFAULT_PORTFOLIO.into(), "kraken")?,
        state
    );
    assert_eq!(
        store.refresh_state(&DEFAULT_PORTFOLIO.into(), "other")?,
        RefreshState::default()
    );
    assert!(
        store
            .save_refresh_state(
                &DEFAULT_PORTFOLIO.into(),
                "kraken",
                &RefreshState {
                    last_success: Some(time(4)),
                    ..state
                }
            )
            .is_err()
    );
    Ok(())
}
