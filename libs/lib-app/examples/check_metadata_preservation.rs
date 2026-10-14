//! Compare every preexisting table to a backup after a metadata-only refresh.
use anyhow::{Result, ensure};
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        (2..=3).contains(&args.len()),
        "usage: check_metadata_preservation DB BACKUP_DB [--with-prices]"
    );
    let with_prices = args.get(2).is_some_and(|s| s == "--with-prices");
    ensure!(args.len() == 2 || with_prices, "unknown comparison option");
    let c = duckdb::Connection::open(&args[0])?;
    c.execute_batch(&format!(
        "ATTACH {} AS before_metadata (READ_ONLY)",
        quote(&args[1])
    ))?;
    let mut stmt = c.prepare("SELECT table_name FROM information_schema.tables WHERE table_catalog='before_metadata' AND table_schema='main' ORDER BY table_name")?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut checked = 0;
    for table in names {
        if table == "token_metadata" || (with_prices && table == "ui_price_evidence") {
            continue;
        }
        let table = identifier(&table);
        let n: i64 = c.query_row(&format!("SELECT count(*) FROM ((SELECT * FROM main.{table} EXCEPT ALL SELECT * FROM before_metadata.main.{table}) UNION ALL (SELECT * FROM before_metadata.main.{table} EXCEPT ALL SELECT * FROM main.{table}))"), [], |r| r.get(0))?;
        ensure!(n == 0, "existing table changed: {table}");
        checked += 1;
    }
    println!(
        "All {checked} protected tables match backup exactly; token metadata{} may differ.",
        if with_prices {
            " and price evidence"
        } else {
            ""
        }
    );
    if with_prices {
        let removed: i64 = c.query_row("SELECT count(*) FROM (SELECT * FROM before_metadata.main.ui_price_evidence EXCEPT ALL SELECT * FROM main.ui_price_evidence)", [], |r| r.get(0))?;
        ensure!(
            removed == 0,
            "existing price evidence was changed or removed"
        );
    }
    let mut stmt = c.prepare("SELECT payload FROM token_metadata ORDER BY id")?;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        let m: lib_core::token_metadata::TokenMetadata = serde_json::from_str(&row?)?;
        println!(
            "{}: name={}, ticker={}, decimals={:?}, RPC={:?}, icon={}, CoinGecko={}, warnings={}",
            m.contract,
            m.name.as_deref().unwrap_or("missing"),
            m.ticker.as_deref().unwrap_or("missing"),
            m.decimals,
            m.rpc_decimals,
            m.icon_data.is_some(),
            m.coingecko_id.as_deref().unwrap_or("unavailable"),
            m.warnings.join("; ")
        );
    }
    Ok(())
}
