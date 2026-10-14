//! Explicit held Ethereum metadata refresh; takes a selected DB and portfolio ID.
use anyhow::{Result, ensure};
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2,
        "usage: refresh_token_metadata DB PORTFOLIO_ID (back up DB first)"
    );
    let store = lib_core::Store::open(&args[0])?;
    let count = lib_app::token_metadata::refresh_held_ethereum(
        &store,
        &lib_core::PortfolioId(args[1].clone()),
    )
    .await?;
    println!("Cached metadata for {count} held Ethereum tokens; ledger unchanged.");
    Ok(())
}
