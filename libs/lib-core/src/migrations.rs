//! Ordered, idempotent database migrations.
//!
//! Every step runs on every Store open, including in-memory databases. Each step
//! checks the current schema/data rather than relying on an applied-version table.
//! Keep existing steps in order and append new migrations to MIGRATIONS.

use anyhow::Context;
use chrono::NaiveDateTime;
use duckdb::{Connection, params};

struct Migration {
    name: &'static str,
    apply: fn(&Connection) -> anyhow::Result<()>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "001_create_tables",
        apply: create_tables,
    },
    Migration {
        name: "002_position_roles",
        apply: position_roles,
    },
    Migration {
        name: "003_legacy_price_coverage",
        apply: legacy_price_coverage,
    },
];

/// Run every migration in order before the Store becomes available. Each step
/// commits atomically; failure stops startup and identifies the failing step.
pub(crate) fn run(connection: &Connection) -> anyhow::Result<()> {
    for migration in MIGRATIONS {
        let tx = connection
            .unchecked_transaction()
            .with_context(|| format!("starting migration {}", migration.name))?;
        (migration.apply)(&tx).with_context(|| format!("applying migration {}", migration.name))?;
        tx.commit()
            .with_context(|| format!("committing migration {}", migration.name))?;
    }
    Ok(())
}

// 001: Baseline tables. Later schema changes belong in subsequent migrations.
fn create_tables(connection: &Connection) -> anyhow::Result<()> {
    connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS transactions (id VARCHAR PRIMARY KEY, datetime TIMESTAMP NOT NULL);
             CREATE TABLE IF NOT EXISTS positions (position_id VARCHAR PRIMARY KEY, asset_id VARCHAR NOT NULL, owned BOOLEAN NOT NULL);
             CREATE TABLE IF NOT EXISTS asset_scales (asset_id VARCHAR PRIMARY KEY, decimals INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS transaction_effects (transaction_id VARCHAR NOT NULL, position_id VARCHAR NOT NULL, amount BIGINT NOT NULL, effect_order INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS asset_prices (datetime TIMESTAMP NOT NULL, asset_id VARCHAR NOT NULL, vs_asset_id VARCHAR NOT NULL, price DOUBLE NOT NULL, PRIMARY KEY(datetime, asset_id, vs_asset_id));
             CREATE TABLE IF NOT EXISTS price_coverage (asset_id VARCHAR NOT NULL, vs_asset_id VARCHAR NOT NULL, from_datetime TIMESTAMP NOT NULL, to_datetime TIMESTAMP NOT NULL, PRIMARY KEY(asset_id, vs_asset_id, from_datetime, to_datetime));
             CREATE TABLE IF NOT EXISTS unavailable_price_periods (provider VARCHAR NOT NULL, pair VARCHAR NOT NULL, from_datetime TIMESTAMP NOT NULL, to_datetime TIMESTAMP NOT NULL, PRIMARY KEY(provider, pair, from_datetime, to_datetime));
             CREATE TABLE IF NOT EXISTS balance_observations (source VARCHAR NOT NULL, content_hash VARCHAR NOT NULL, position_id VARCHAR NOT NULL, datetime TIMESTAMP NOT NULL, amount VARCHAR NOT NULL);
             CREATE TABLE IF NOT EXISTS imported_files (source VARCHAR NOT NULL, content_hash VARCHAR NOT NULL, transaction_count BIGINT NOT NULL, completed_at TIMESTAMP NOT NULL, PRIMARY KEY(source, content_hash));",
        )?;
    Ok(())
}

// 002: Upgrade ownership-only positions; the legacy Nexo credit namespace
// denotes user debt. Existing explicit roles are never reclassified.
fn position_roles(connection: &Connection) -> anyhow::Result<()> {
    let has_role: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('positions') WHERE name='role')",
        [],
        |row| row.get(0),
    )?;
    if !has_role {
        connection.execute_batch(
            "CREATE TABLE positions_with_roles (
                     position_id VARCHAR PRIMARY KEY, asset_id VARCHAR NOT NULL,
                     owned BOOLEAN NOT NULL, role VARCHAR NOT NULL
                     CHECK (role IN ('asset', 'liability', 'external')));
                 INSERT INTO positions_with_roles
                     SELECT position_id, asset_id, owned, CASE
                         WHEN owned THEN 'asset'
                         WHEN starts_with(position_id, 'NEXO-CREDIT:') THEN 'liability'
                         ELSE 'external' END FROM positions;
                 DROP TABLE positions;
                 ALTER TABLE positions_with_roles RENAME TO positions;",
        )?;
    }
    Ok(())
}

// 003: Infer only dense legacy price runs without known request boundaries.
// Existing coverage and sparse observations remain unchanged on repeated runs.
fn legacy_price_coverage(connection: &Connection) -> anyhow::Result<()> {
    let mut statement = connection.prepare(
        "SELECT ap.asset_id, ap.vs_asset_id, ap.datetime FROM asset_prices ap \
             WHERE NOT EXISTS (SELECT 1 FROM price_coverage pc \
                 WHERE pc.asset_id=ap.asset_id AND pc.vs_asset_id=ap.vs_asset_id) \
             ORDER BY ap.asset_id, ap.vs_asset_id, ap.datetime",
    )?;
    let observations = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, NaiveDateTime>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut runs = Vec::new();
    let mut run: Option<(String, String, NaiveDateTime, NaiveDateTime)> = None;
    for (asset, quote, at) in observations {
        match &mut run {
            Some((current_asset, current_quote, _, end))
                if *current_asset == asset
                    && *current_quote == quote
                    && at.signed_duration_since(*end) <= chrono::Duration::hours(48) =>
            {
                *end = at;
            }
            _ => {
                if let Some(previous) = run.take() {
                    if previous.2 < previous.3 {
                        runs.push(previous);
                    }
                }
                run = Some((asset, quote, at, at));
            }
        }
    }
    if let Some(previous) = run {
        if previous.2 < previous.3 {
            runs.push(previous);
        }
    }
    for (asset, quote, from, to) in runs {
        connection.execute(
            "INSERT OR IGNORE INTO price_coverage VALUES (?, ?, ?, ?)",
            params![asset, quote, from, to],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    #[test]
    fn startup_reruns_migrations_and_preserves_existing_data() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("portfolio.duckdb");
        // An empty database requires the baseline before the role migration.
        drop(Store::open(&path)?);
        {
            let connection = Connection::open(&path)?;
            connection.execute_batch(
                "INSERT INTO positions VALUES ('NEXO-CREDIT:explicit', 'ETH', false, 'external');
                 INSERT INTO asset_prices VALUES ('2025-01-01', 'ETH', 'USD', 100), ('2025-01-02', 'ETH', 'USD', 101);
                 INSERT INTO imported_files VALUES ('NEXO', 'original', 3, '2025-01-01');",
            )?;
        }
        drop(Store::open(&path)?);
        {
            let connection = Connection::open(&path)?;
            let count: i64 =
                connection.query_row("SELECT count(*) FROM price_coverage", [], |r| r.get(0))?;
            assert_eq!(count, 1);
            // A later startup must run the backfill again, even though the
            // schema and earlier price coverage have already been migrated.
            connection.execute_batch(
                "INSERT INTO asset_prices VALUES ('2025-01-01', 'BTC', 'USD', 200), ('2025-01-02', 'BTC', 'USD', 201);",
            )?;
        }
        for _ in 0..3 {
            drop(Store::open(&path)?);
        }
        let connection = Connection::open(&path)?;
        let counts: (i64, i64, i64, i64) = connection.query_row(
            "SELECT (SELECT count(*) FROM positions), (SELECT count(*) FROM asset_prices),
                    (SELECT count(*) FROM price_coverage), (SELECT count(*) FROM imported_files)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        assert_eq!(counts, (1, 4, 2, 1));
        let role: String = connection.query_row("SELECT role FROM positions", [], |r| r.get(0))?;
        assert_eq!(role, "external");
        Ok(())
    }

    #[test]
    fn failed_migration_rolls_back_and_can_be_retried() -> anyhow::Result<()> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(
            "CREATE TABLE positions (position_id VARCHAR PRIMARY KEY, asset_id VARCHAR, owned BOOLEAN NOT NULL);
             INSERT INTO positions VALUES ('NEXO-CREDIT:xUSD', NULL, false);",
        )?;
        let error = run(&connection).unwrap_err();
        assert!(error.to_string().contains("002_position_roles"));
        let partial_tables: i64 = connection.query_row(
            "SELECT count(*) FROM information_schema.tables WHERE table_name='positions_with_roles'",
            [], |r| r.get(0),
        )?;
        assert_eq!(partial_tables, 0);
        connection.execute("UPDATE positions SET asset_id='xUSD'", [])?;
        run(&connection)?;
        run(&connection)?;
        let role: String = connection.query_row("SELECT role FROM positions", [], |r| r.get(0))?;
        assert_eq!(role, "liability");
        Ok(())
    }
}
