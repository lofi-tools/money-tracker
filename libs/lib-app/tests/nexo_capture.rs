use anyhow::Result;
use chrono::{DateTime, Utc};
use lib_app::{
    Application,
    capture_bridge::*,
    config::*,
    nexo_capture::*,
    startup::OpenAction,
    worker::{DatabaseWorker, StartupOutcome},
};
use lib_core::{AccountId, DEFAULT_PORTFOLIO, Store, ui_metadata::*};
use serde_json::json;
fn at(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}
fn capture() -> Capture {
    serde_json::from_value(json!({
 "schema":"money-tracker.nexo-terms","version":1,"capture_id":"run-1","capture_started_at":"2026-01-10T00:00:00Z","capture_finished_at":"2026-01-10T00:01:00Z","extractor_version":"synthetic-1","origin":"https://platform.nexo.com","page_path":"/fixed-terms/eth","locale":"en","browser_timezone":"Europe/London","account_hint":{"source":"paired"},"account_settings":{},"coverage":{"status":"partial","reported_active_count":1,"views_captured":1},"warnings":[],"terms":[{
 "observation_id":"one","provider_term_id":"remote-term-1","observed_at":"2026-01-10T00:00:30Z","status":"active","principal":{"asset":"ETH","amount":"40.00000000","precision":"full","exactness":"exact"},"rate":{"percent":"6.25","basis":"apy","raw_label":"6.25% APY","scope":"term"},"payout":{"asset":"NEXO","scope":"term","historical_continuity":"unknown"},"start":{"value":"2026-01-02T00:00:00Z","precision":"instant","timezone":"UTC"},"maturity":{"value":"2026-02-02","precision":"date","timezone":null},"duration_months":1,"auto_renew":true,"renew_with_interest":false,"accrued_rewards":[],"evidence":{"view":"synthetic"},"warnings":[]
 }] })).unwrap()
}
fn setup() -> Result<(tempfile::TempDir, Application)> {
    let d = tempfile::tempdir()?;
    let csv = d.path().join("nexo.csv");
    std::fs::write(
        &csv,
        concat!(
            "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\n",
            "deposit,Top up Crypto,ETH,100,ETH,100,$1000,-,-,deposit,2026-01-01 00:00:00\n",
            "lock,Locking Term Deposit,ETH,-40,ETH,40,$400,-,-,lock,2026-01-02 00:00:00\n"
        ),
    )?;
    let app = Application::open(d.path().join("portfolio.duckdb"))?;
    lib_app::adapters::nexo::NexoSvc::from_path(csv).import_into(app.store())?;
    Ok((d, app))
}
fn ingest(app: &Application, c: &Capture) -> Result<Receipt> {
    app.ingest_nexo_capture(
        &DEFAULT_PORTFOLIO.into(),
        &"legacy:nexo".into(),
        &serde_json::to_string(c)?,
    )
}
#[test]
fn exact_capture_is_idempotent_keeps_principal_and_date_precision() -> Result<()> {
    let (_d, app) = setup()?;
    let account = AccountId::from("legacy:nexo");
    let before = app
        .store()
        .account_audit_balances_at(&account, at("2026-01-20T00:00:00Z"))?;
    let c = capture();
    let r = ingest(&app, &c)?;
    assert_eq!((r.matched, r.unresolved), (1, 0));
    let sub = r.matches[0].subscription_id.as_ref().unwrap();
    let rate = app
        .store()
        .selected_rate(
            &DEFAULT_PORTFOLIO.into(),
            &RateScope::Subscription(sub.clone()),
            Utc::now(),
        )?
        .unwrap();
    assert_eq!(rate.rate.unwrap().fraction.as_str(), "0.0625");
    assert!(app.store().subscription(sub)?.unwrap().maturity.is_none());
    assert_eq!(
        app.captured_nexo_term(&account, sub, Utc::now())?
            .unwrap()
            .maturity
            .unwrap()
            .precision,
        "date"
    );
    let rev = app.store().revisions(&DEFAULT_PORTFOLIO.into())?;
    assert!(ingest(&app, &c)?.duplicate);
    assert_eq!(app.store().revisions(&DEFAULT_PORTFOLIO.into())?, rev);
    assert_eq!(
        before,
        app.store()
            .account_audit_balances_at(&account, at("2026-01-20T00:00:00Z"))?
    );
    let mut changed = c;
    changed.terms[0].auto_renew = Some(false);
    assert!(ingest(&app, &changed).is_err());
    assert_eq!(app.store().nexo_captures(&account)?.len(), 1);
    Ok(())
}
#[test]
fn competing_captures_rounded_principal_unknown_start_and_empty_list_do_not_guess() -> Result<()> {
    for kind in 0..4 {
        let (_d, app) = setup()?;
        let mut c = capture();
        match kind {
            0 => {
                let mut t = c.terms[0].clone();
                t.observation_id = "two".into();
                t.provider_term_id = None;
                c.terms.push(t);
            }
            1 => c.terms[0].principal.as_mut().unwrap().exactness = "unknown".into(),
            2 => {
                let date = c.terms[0].start.as_mut().unwrap();
                date.precision = "date".into();
                date.value = "2026-01-02".into();
                date.timezone = None;
            }
            _ => c.terms.clear(),
        };
        let r = ingest(&app, &c)?;
        assert_eq!(r.matched, 0);
        assert!(
            app.store()
                .subscription("nexo-term:legacy:nexo:lock")?
                .unwrap()
                .closed_at
                .is_none()
        );
    }
    Ok(())
}
#[test]
fn pins_unknown_basis_and_provider_conflicts_are_preserved() -> Result<()> {
    let (_d, app) = setup()?;
    let mut c = capture();
    let subscription = "nexo-term:legacy:nexo:lock".to_string();
    app.apply_manual_edit(
        &DEFAULT_PORTFOLIO.into(),
        &lib_app::manual::ManualEdit::Rate {
            scope: RateScope::Product("nexo-fixed:legacy:nexo:ETH".into()),
            rate: Some(AnnualRate {
                fraction: "0.07".parse()?,
                basis: AnnualRateBasis::Apy,
                compounds_per_year: None,
                reward_asset: None,
            }),
            effective_at: at("2026-01-01T00:00:00Z"),
            pinned: true,
            clear: false,
        },
    )?;
    let r = ingest(&app, &c)?;
    assert_eq!(r.conflicts, 1);
    assert!(
        app.store()
            .rate_evidence(
                &DEFAULT_PORTFOLIO.into(),
                &RateScope::Subscription(subscription.clone())
            )?
            .is_empty()
    );
    c.capture_id = "run-2".into();
    c.terms[0].rate.as_mut().unwrap().basis = "unknown".into();
    assert_eq!(ingest(&app, &c)?.matched, 1);
    assert!(
        app.store()
            .rate_evidence(
                &DEFAULT_PORTFOLIO.into(),
                &RateScope::Subscription(subscription)
            )?
            .is_empty()
    );
    Ok(())
}
#[test]
fn malformed_or_wrong_account_capture_cannot_write() -> Result<()> {
    let (_d, app) = setup()?;
    let c = capture();
    for kind in 0..5 {
        let mut c = c.clone();
        match kind {
            0 => c.origin = "https://evil.example".into(),
            1 => c.terms[0].principal.as_mut().unwrap().amount = "NaN".into(),
            2 => c.coverage.status = "complete".into(),
            3 => c.terms[0].observed_at = at("2026-02-01T00:00:00Z"),
            _ => c.version = 2,
        };
        if kind == 2 {
            c.coverage.reported_active_count = Some(99);
        }
        assert!(ingest(&app, &c).is_err());
    }
    assert!(
        app.ingest_nexo_capture(
            &DEFAULT_PORTFOLIO.into(),
            &"not-my-account".into(),
            &serde_json::to_string(&c)?
        )
        .is_err()
    );
    assert!(app.store().nexo_captures(&"legacy:nexo".into())?.is_empty());
    Ok(())
}
#[test]
fn native_frames_bound_lengths_and_reject_truncation() -> Result<()> {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let mut bytes = vec![];
    write_frame(&mut bytes, &json!({"ok":true}))?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&read_frame(&mut bytes.as_slice())?.unwrap())?,
        json!({"ok":true})
    );
    assert!(read_frame(&mut ((MAX_BYTES + 4097) as u32).to_ne_bytes().as_slice()).is_err());
    assert!(read_frame(&mut [2, 0, 0, 0, 1].as_slice()).is_err());
    assert!(read_frame(&mut [].as_slice())?.is_none());
    Ok(())
}
#[tokio::test]
async fn native_inbox_uses_existing_db_owner_and_acknowledges_committed_retries() -> Result<()> {
    let (d, app) = setup()?;
    drop(app);
    let pair = Pairing {
        version: 1,
        connection_id: "a".repeat(64),
        extension_id: "b".repeat(32),
        database: d.path().join("portfolio.duckdb"),
        portfolio: DEFAULT_PORTFOLIO.into(),
        account: "legacy:nexo".into(),
        tracker_executable: None,
    };
    assert!(
        pair.authorize(&format!("chrome-extension://{}/", "c".repeat(32)))
            .is_err()
    );
    let dir = root(&pair.database)?;
    private_dir(&dir)?;
    atomic_private(&dir.join("pairing.json"), &serde_json::to_vec(&pair)?)?;
    let env = Envelope {
        version: 1,
        connection_id: pair.connection_id.clone(),
        payload: serde_json::to_string(&capture())?,
    };
    let id = pair.enqueue(&env)?;
    let loc = Locations::for_platform(d.path(), d.path(), d.path(), d.path(), d.path(), false);
    let mut resolved = ResolvedConfig::resolve(loc, None, None, None)?;
    resolved.database = pair.database.clone();
    let StartupOutcome::Ready(worker) =
        DatabaseWorker::start(resolved, OpenAction::OpenExisting).await?
    else {
        panic!("DB should open")
    };
    let receipts = worker.ingest_browser_inbox().await?;
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].matched, 1);
    let ack: Ack = serde_json::from_slice(&read_private(
        &dir.join(format!("{id}.result.json")),
        1_000_000,
    )?)?;
    assert!(matches!(ack, Ack::Committed { .. }));
    assert!(worker.ingest_browser_inbox().await?.is_empty());
    assert_eq!(worker.nexo_capture_views().await?.len(), 1);
    worker.close().await?;
    let reopened = Store::open(&pair.database)?;
    assert_eq!(reopened.nexo_captures(&pair.account.into())?.len(), 1);
    Ok(())
}
#[cfg(unix)]
#[test]
fn symlinks_and_world_readable_inboxes_are_rejected() -> Result<()> {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let d = tempfile::tempdir()?;
    let target = d.path().join("target");
    std::fs::write(&target, b"{}")?;
    let link = d.path().join("link");
    symlink(&target, &link)?;
    assert!(read_private(&link, 10).is_err());
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o755))?;
    assert!(private_dir(d.path()).is_err());
    Ok(())
}

#[test]
fn ambiguous_csv_cohort_and_changed_external_identity_never_rebind() -> Result<()> {
    for ambiguous in [true, false] {
        let (dir, app) = setup()?;
        if !ambiguous {
            assert_eq!(ingest(&app, &capture())?.matched, 1);
        }
        let csv = dir.path().join("nexo.csv");
        let mut source = std::fs::read_to_string(&csv)?;
        if ambiguous {
            source.push_str("lock2,Locking Term Deposit,ETH,-40,ETH,40,$400,-,-,lock,2026-01-03 00:00:00\nunlock,Unlocking Term Deposit,ETH,40,ETH,40,$400,-,-,unlock,2026-01-04 00:00:00\n");
        } else {
            source.push_str(
                "lock2,Locking Term Deposit,ETH,-10,ETH,10,$100,-,-,lock,2026-01-03 00:00:00\n",
            );
        }
        std::fs::write(&csv, source)?;
        lib_app::adapters::nexo::NexoSvc::from_path(&csv).import_into(app.store())?;
        let mut c = capture();
        c.capture_id = "run-new".into();
        if !ambiguous {
            c.terms[0].principal.as_mut().unwrap().amount = "10".into();
            c.terms[0].start.as_mut().unwrap().value = "2026-01-03T00:00:00Z".into();
        }
        let receipt = ingest(&app, &c)?;
        assert_eq!(receipt.matched, 0);
        assert_eq!(receipt.unresolved, 1);
        if !ambiguous {
            assert_eq!(receipt.conflicts, 1);
            assert!(receipt.matches[0].reason.contains("earlier binding"));
        }
    }
    Ok(())
}
