use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use lib_app::{Application, dto::*};
use lib_core::{
    Account, AssetId, Portfolio, PositionId, PositionRole, Store, Transaction, TxnEffect, Units,
};

fn cutoff() -> DateTime<Utc> {
    "2026-10-01T00:00:00Z".parse().unwrap()
}
fn seed(store: &Store, portfolio: &str, account: &str, amount: &str) -> Result<Account> {
    store.save_portfolio(&Portfolio {
        id: portfolio.into(),
        label: portfolio.into(),
        reporting_asset: AssetId::str("USD"),
    })?;
    let account = Account {
        id: account.into(),
        portfolio_id: portfolio.into(),
        provider_id: "kraken".into(),
        label: "Source account label".into(),
        source_reference: Some("remote-reference".into()),
    };
    store.save_account(&account)?;
    let asset: PositionId = "token".into();
    let debt: PositionId = "debt".into();
    let peer: PositionId = "counterparty".into();
    let spam: PositionId = "spam".into();
    store.save_account_import(
        &account.id,
        "fixture",
        "one",
        &[
            (
                asset.clone(),
                AssetId::str("TOKEN"),
                18,
                PositionRole::Asset,
            ),
            (
                debt.clone(),
                AssetId::str("USD"),
                2,
                PositionRole::Liability,
            ),
            (
                peer.clone(),
                AssetId::str("USD"),
                2,
                PositionRole::ExternalCounterparty,
            ),
            (spam.clone(), AssetId::str("SPAM"), 0, PositionRole::Asset),
        ],
        &[(
            "same-provider-record".into(),
            Transaction {
                datetime: cutoff(),
                inputs: vec![TxnEffect {
                    datetime: cutoff(),
                    position_id: debt,
                    amount: Units::from(2500),
                }],
                outputs: vec![
                    TxnEffect {
                        datetime: cutoff(),
                        position_id: asset,
                        amount: amount.parse()?,
                    },
                    TxnEffect {
                        datetime: cutoff(),
                        position_id: peer,
                        amount: Units::from(2500),
                    },
                    TxnEffect {
                        datetime: cutoff(),
                        position_id: spam,
                        amount: Units::from(99),
                    },
                ],
            },
        )],
        &[],
    )?;
    store.review_asset_tracking(
        &account.id,
        &AssetId::str("SPAM"),
        true,
        "spam",
        "fixture review",
    )?;
    Ok(account)
}

#[test]
fn exact_account_scoped_dtos_include_debt_exclude_counterparties_and_spam() -> Result<()> {
    let app = Application::from_store(Store::in_memory()?);
    let large = "123456789012345678901234567890000000000000000001";
    let a = seed(app.store(), "portfolio-a", "kraken-a", large)?;
    seed(app.store(), "portfolio-a", "kraken-b", "1")?;
    seed(app.store(), "portfolio-b", "kraken-c", "999")?;
    let before = app.store().account_audit_balances_at(&a.id, cutoff())?;
    let request = ReadContext {
        portfolio_id: "portfolio-a".into(),
        cutoff: cutoff(),
    };
    let response = app.ledger_balances(request.clone())?;
    assert_eq!(response.accounts.len(), 2);
    assert_eq!(response.positions.len(), 4);
    assert!(
        response
            .positions
            .iter()
            .all(|p| p.account_id.0 != "kraken-c")
    );
    assert!(
        response
            .positions
            .iter()
            .all(|p| p.quantity.asset_id.0 != "SPAM")
    );
    let token = response
        .positions
        .iter()
        .find(|p| p.account_id == a.id && p.role == PositionRole::Asset)
        .unwrap();
    assert_eq!(token.quantity.units.to_string(), large);
    assert_eq!(
        token.quantity.decimal_string(),
        "123456789012345678901234567890.000000000000000001"
    );
    let debt = response
        .positions
        .iter()
        .find(|p| p.account_id == a.id && p.role == PositionRole::Liability)
        .unwrap();
    assert_eq!(debt.quantity.decimal_string(), "-25.00");
    assert!(matches!(response.verification, VerificationDto::Unknown));
    assert!(matches!(
        response.evidence,
        EvidenceDto::LedgerReplay { .. }
    ));
    assert_eq!(response.issues[0].code, "ledger_history_unverified");
    let json = serde_json::to_value(&response)?;
    assert_eq!(
        json["positions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["account_id"] == "kraken-a" && p["role"] == "asset")
            .unwrap()["quantity"]["units"]
            .as_str(),
        Some(large)
    );
    assert_eq!(serde_json::from_value::<LedgerBalancesDto>(json)?, response);
    assert_eq!(
        app.store().account_audit_balances_at(&a.id, cutoff())?,
        before
    );
    let earlier = app.ledger_balances(ReadContext {
        cutoff: cutoff() - Duration::seconds(1),
        ..request
    })?;
    assert!(earlier.positions.is_empty());
    assert!(
        app.ledger_balances(ReadContext {
            portfolio_id: "does-not-exist".into(),
            cutoff: cutoff()
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn opening_existing_database_is_idempotent_and_keeps_legacy_provider_ids() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("portfolio.duckdb");
    let request = ReadContext {
        portfolio_id: lib_core::DEFAULT_PORTFOLIO.into(),
        cutoff: cutoff(),
    };
    let expected;
    {
        let app = Application::open(&path)?;
        for (position, role) in [
            ("NEXO:BTC", PositionRole::Asset),
            ("NEXO-CREDIT:USD", PositionRole::Liability),
            ("BINANCE:spot:BTC", PositionRole::Asset),
        ] {
            let asset = AssetId::str(if role == PositionRole::Liability {
                "USD"
            } else {
                "BTC"
            });
            app.store().save_asset_scale(
                &asset,
                if role == PositionRole::Liability {
                    2
                } else {
                    8
                },
            )?;
            app.store()
                .save_position_asset(&position.into(), &asset, role)?;
            app.store().save_transaction(
                position,
                &Transaction {
                    datetime: cutoff(),
                    inputs: if role == PositionRole::Liability {
                        vec![TxnEffect {
                            datetime: cutoff(),
                            position_id: position.into(),
                            amount: 100.into(),
                        }]
                    } else {
                        vec![]
                    },
                    outputs: if role == PositionRole::Asset {
                        vec![TxnEffect {
                            datetime: cutoff(),
                            position_id: position.into(),
                            amount: "900719925474099312345".parse()?,
                        }]
                    } else {
                        vec![]
                    },
                },
            )?;
        }
        expected = app.ledger_balances(request.clone())?;
        assert_eq!(expected.positions.len(), 3);
    }
    for _ in 0..2 {
        let app = Application::open(&path)?;
        assert_eq!(app.ledger_balances(request.clone())?, expected);
        assert!(
            expected
                .positions
                .iter()
                .any(|p| p.position_id.0 == "NEXO-CREDIT:USD")
        );
    }
    Ok(())
}
