//! Explicit held Ethereum metadata refresh; takes a selected DB and portfolio ID.
use anyhow::{Result, ensure};
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        (2..=3).contains(&args.len()),
        "usage: refresh_token_metadata DB PORTFOLIO_ID [CHAIN_ID] (back up DB first)"
    );
    let store = lib_core::Store::open(&args[0])?;
    let count = lib_app::token_metadata::refresh_held_network(
        &store,
        &lib_core::PortfolioId(args[1].clone()),
        args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(1),
    )
    .await?;
    println!("Cached metadata for {count} held tokens; ledger unchanged.");
    Ok(())
}
