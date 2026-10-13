//! Explicit account-local setting; does not create a DB or infer a jurisdiction.
use anyhow::{Result, ensure};
use lib_core::{AccountId, Store};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: nexo-account-jurisdiction EXISTING_DB ACCOUNT_ID COUNTRY_CODE"
    );
    ensure!(
        std::path::Path::new(&args[0]).is_file(),
        "database must already exist"
    );
    let store = Store::open(&args[0])?;
    let id = AccountId::from(&args[1]);
    let mut found = false;
    for portfolio in store.portfolios()? {
        if let Some(account) = store
            .accounts(&portfolio.id)?
            .into_iter()
            .find(|a| a.id == id)
        {
            ensure!(
                account.provider_id.0.eq_ignore_ascii_case("nexo"),
                "account is not Nexo"
            );
            found = true;
            break;
        }
    }
    ensure!(found, "account does not exist");
    store.set_account_jurisdiction(&id, &args[2], chrono::Utc::now())?;
    println!("Nexo account jurisdiction saved; provider holdings and rates unchanged.");
    Ok(())
}
