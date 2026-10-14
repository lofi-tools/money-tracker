//! Persist an explicit wallet-software annotation through the manual action log.
use anyhow::{Result, ensure};
use lib_app::{Application, manual::ManualEdit};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: annotate-wallet EXISTING_DB PUBLIC_ADDRESS SOFTWARE"
    );
    ensure!(
        std::path::Path::new(&args[0]).is_file(),
        "database must already exist"
    );
    let app = Application::open(&args[0])?;
    let mut count = 0;
    for p in app.store().portfolios()? {
        for a in app.store().accounts(&p.id)? {
            if a.provider_id.0.eq_ignore_ascii_case("evm")
                && a.source_reference.as_ref().is_some_and(|r| {
                    r.split(':')
                        .last()
                        .is_some_and(|addr| addr.eq_ignore_ascii_case(&args[1]))
                })
            {
                app.apply_manual_edit(
                    &p.id,
                    &ManualEdit::WalletSoftware {
                        account: a.id,
                        software: Some(args[2].clone()),
                    },
                )?;
                count += 1;
            }
        }
    }
    ensure!(count > 0, "no matching registered wallet");
    println!("Annotated {count} chain accounts; provider identity and ledger unchanged.");
    Ok(())
}
