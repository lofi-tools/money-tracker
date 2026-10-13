use anyhow::Result;
use lib_core::*;
#[test]
fn declaration_is_account_scoped_idempotent_and_survives_reimport_reopen() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("portfolio.duckdb");
    let at = "2026-10-13T00:00:00Z".parse()?;
    let first = Account {
        id: "nexo:a".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        provider_id: "nexo".into(),
        label: "Nexo A".into(),
        source_reference: None,
    };
    let second = Account {
        id: "nexo:b".into(),
        ..first.clone()
    };
    let before;
    {
        let store = Store::open(&path)?;
        store.save_account(&first)?;
        store.save_account(&second)?;
        assert!(store.account_jurisdiction(&first.id)?.is_none());
        before = store.revisions(&first.portfolio_id)?.metadata;
        store.set_account_jurisdiction(&first.id, "uk", at)?;
        assert_eq!(
            store.account_jurisdiction(&first.id)?.unwrap().country_code,
            "GB"
        );
        assert!(store.account_jurisdiction(&second.id)?.is_none());
        assert_eq!(store.revisions(&first.portfolio_id)?.metadata, before + 1);
        store.set_account_jurisdiction(&first.id, "GB", at)?;
        assert_eq!(store.revisions(&first.portfolio_id)?.metadata, before + 1);
        store.save_account(&first)?;
        assert!(
            store
                .set_account_jurisdiction(&"missing".into(), "GB", at)
                .is_err()
        );
        assert!(
            store
                .set_account_jurisdiction(&first.id, "United Kingdom", at)
                .is_err()
        );
    }
    for _ in 0..3 {
        let store = Store::open(&path)?;
        assert_eq!(
            store.account_jurisdiction(&first.id)?.unwrap().declared_at,
            at
        );
        assert_eq!(store.revisions(&first.portfolio_id)?.metadata, before + 1);
        assert!(store.account_jurisdiction(&second.id)?.is_none());
        assert!(
            store
                .set_account_jurisdiction(&first.id, "US", "2026-10-12T00:00:00Z".parse()?)
                .is_err()
        );
    }
    Ok(())
}
