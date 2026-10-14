//! Read-only audit of original rows after additive provider replay/precision widening.
use anyhow::{Result, ensure};
use lib_core::Units;
use std::str::FromStr;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2,
        "usage: check_provider_preservation AFTER_DB BEFORE_DB"
    );
    let c = duckdb::Connection::open(&args[0])?;
    c.execute_batch(&format!(
        "ATTACH '{}' AS baseline (READ_ONLY)",
        args[1].replace('\'', "''")
    ))?;
    let tables=c.prepare("SELECT table_name FROM information_schema.tables WHERE table_catalog='baseline' AND table_schema='main'")?
        .query_map([],|r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
    let mut checked = 0;
    for name in tables {
        if [
            "transaction_effects",
            "balance_observations",
            "asset_scales",
            "ui_portfolio_revisions",
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        let quoted = format!("\"{}\"", name.replace('"', "\"\""));
        let lost:i64=c.query_row(&format!("SELECT count(*) FROM (SELECT * FROM baseline.main.{quoted} EXCEPT ALL SELECT * FROM main.{quoted})"),[],|r|r.get(0))?;
        ensure!(lost == 0, "original rows changed in {name}");
        checked += 1;
    }
    let rows=c.prepare("SELECT b.amount,a.amount,bs.decimals,s.decimals FROM baseline.main.transaction_effects b JOIN baseline.main.positions bp USING(position_id) JOIN baseline.main.asset_scales bs ON bs.asset_id=bp.asset_id LEFT JOIN main.transaction_effects a ON a.transaction_id=b.transaction_id AND a.position_id=b.position_id AND a.effect_order=b.effect_order JOIN main.asset_scales s ON s.asset_id=bp.asset_id")?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,u8>(2)?,r.get::<_,u8>(3)?)))?.collect::<Result<Vec<_>,_>>()?;
    let effects = rows.len();
    for (before, after, old, new) in rows {
        ensure!(
            Units::from_str(&before)?.rescale(old, new)? == Units::from_str(&after)?,
            "original effect amount changed"
        );
    }
    let rows=c.prepare("SELECT b.amount,a.amount,bs.decimals,s.decimals FROM baseline.main.balance_observations b JOIN baseline.main.positions bp USING(position_id) JOIN baseline.main.asset_scales bs ON bs.asset_id=bp.asset_id LEFT JOIN main.balance_observations a ON a.source=b.source AND a.content_hash=b.content_hash AND a.position_id=b.position_id AND a.datetime=b.datetime JOIN main.asset_scales s ON s.asset_id=bp.asset_id")?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,u8>(2)?,r.get::<_,u8>(3)?)))?.collect::<Result<Vec<_>,_>>()?;
    let observations = rows.len();
    for (before, after, old, new) in rows {
        ensure!(
            Units::from_str(&before)?.rescale(old, new)? == Units::from_str(&after)?,
            "original observation amount changed"
        );
    }
    let lost:i64=c.query_row("SELECT count(*) FROM (SELECT portfolio_id,policy,metadata FROM baseline.main.ui_portfolio_revisions EXCEPT ALL SELECT portfolio_id,policy,metadata FROM main.ui_portfolio_revisions)",[],|r|r.get(0))?;
    ensure!(
        lost == 0,
        "original local metadata/policy revisions changed"
    );
    println!(
        "Preserved original rows in {checked} tables, {effects} effects and {observations} observations; quantities remain exact after scale widening."
    );
    Ok(())
}
