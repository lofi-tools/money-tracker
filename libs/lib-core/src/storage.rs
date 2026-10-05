use std::{collections::HashMap, path::Path};

use crate::history::AssetPricePoint;
use crate::{
    AssetId, PositionBalance, PositionId, Product, ProductId, Transaction, TransactionId,
    UserPosition,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use duckdb::{Connection, params};

/// Local DuckDB persistence for transactions, position ownership/currency, and prices.
pub struct Store {
    connection: Connection,
}

/// A half-open interval of historical prices: [from, to).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PriceRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

impl PriceRange {
    pub fn new(from: DateTime<Utc>, to: DateTime<Utc>) -> anyhow::Result<Self> {
        anyhow::ensure!(from < to, "price range must have from < to");
        Ok(Self { from, to })
    }
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> anyhow::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> anyhow::Result<Self> {
        let store = Self { connection };
        store.init()?;
        Ok(store)
    }

    fn init(&self) -> anyhow::Result<()> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS transactions (id VARCHAR PRIMARY KEY, datetime TIMESTAMP NOT NULL);
             CREATE TABLE IF NOT EXISTS positions (position_id VARCHAR PRIMARY KEY, asset_id VARCHAR NOT NULL, owned BOOLEAN NOT NULL);
             CREATE TABLE IF NOT EXISTS asset_scales (asset_id VARCHAR PRIMARY KEY, decimals INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS transaction_effects (transaction_id VARCHAR NOT NULL, position_id VARCHAR NOT NULL, amount BIGINT NOT NULL, effect_order INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS asset_prices (datetime TIMESTAMP NOT NULL, asset_id VARCHAR NOT NULL, vs_asset_id VARCHAR NOT NULL, price DOUBLE NOT NULL, PRIMARY KEY(datetime, asset_id, vs_asset_id));
             CREATE TABLE IF NOT EXISTS price_coverage (asset_id VARCHAR NOT NULL, vs_asset_id VARCHAR NOT NULL, from_datetime TIMESTAMP NOT NULL, to_datetime TIMESTAMP NOT NULL, PRIMARY KEY(asset_id, vs_asset_id, from_datetime, to_datetime));
             CREATE TABLE IF NOT EXISTS unavailable_price_periods (provider VARCHAR NOT NULL, pair VARCHAR NOT NULL, from_datetime TIMESTAMP NOT NULL, to_datetime TIMESTAMP NOT NULL, PRIMARY KEY(provider, pair, from_datetime, to_datetime));
             CREATE TABLE IF NOT EXISTS imported_files (source VARCHAR NOT NULL, content_hash VARCHAR NOT NULL, transaction_count BIGINT NOT NULL, completed_at TIMESTAMP NOT NULL, PRIMARY KEY(source, content_hash));",
        )?;
        self.infer_legacy_price_coverage()?;
        Ok(())
    }

    /// Older databases contain observations but no request boundaries. Treat
    /// dense runs of observations as known history, leaving sparse spot quotes
    /// and the edges outside those runs available for a precise backfill.
    fn infer_legacy_price_coverage(&self) -> anyhow::Result<()> {
        let mut statement = self.connection.prepare(
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
        let tx = self.connection.unchecked_transaction()?;
        for (asset, quote, from, to) in runs {
            tx.execute(
                "INSERT OR IGNORE INTO price_coverage VALUES (?, ?, ?, ?)",
                params![asset, quote, from, to],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Register a position's currency and ownership. Counterparty positions are
    /// persisted with `owned = false` and excluded from net-worth valuation.
    pub fn save_position_asset(
        &self,
        position_id: &PositionId,
        asset_id: &AssetId,
        owned: bool,
    ) -> anyhow::Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO positions VALUES (?, ?, ?)",
            params![position_id.0, asset_id.0.as_str(), owned],
        )?;
        Ok(())
    }

    /// Record the ledger scale for an asset (the CSV importer uses 8 decimals).
    pub fn save_asset_scale(&self, asset_id: &AssetId, decimals: u8) -> anyhow::Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO asset_scales VALUES (?, ?)",
            params![asset_id.0.as_str(), i32::from(decimals)],
        )?;
        Ok(())
    }

    /// Persist a transaction and signed effects atomically. `id` should be the
    /// provider's stable transaction ID for idempotent imports.
    pub fn save_transaction(&self, id: &str, transaction: &Transaction) -> anyhow::Result<()> {
        self.save_transactions(&[(id.to_string(), transaction.clone())])
    }

    /// Save an import in one database transaction. Repeated IDs replace their effects.
    pub fn save_transactions(&self, transactions: &[(String, Transaction)]) -> anyhow::Result<()> {
        self.save_transactions_with_progress(transactions, |_, _| {})
    }

    /// Like `save_transactions`, with progress reported after each 1,000 rows
    /// and at completion. The whole import remains one database transaction.
    pub fn save_transactions_with_progress(
        &self,
        transactions: &[(String, Transaction)],
        mut on_progress: impl FnMut(usize, usize),
    ) -> anyhow::Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        Self::write_transactions(&tx, transactions, &mut on_progress)?;
        tx.commit()?;
        Ok(())
    }

    /// A completed file import is identified by its content hash. The file
    /// marker, asset registrations, and every transaction commit together.
    /// Returns false if this exact source/hash was already committed.
    pub fn save_file_import_with_progress(
        &self,
        source: &str,
        content_hash: &str,
        asset_positions: &[(PositionId, AssetId, u8, bool)],
        transactions: &[(String, Transaction)],
        mut on_progress: impl FnMut(usize, usize),
    ) -> anyhow::Result<bool> {
        let tx = self.connection.unchecked_transaction()?;
        let already_imported: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM imported_files WHERE source=? AND content_hash=?)",
            params![source, content_hash],
            |row| row.get(0),
        )?;
        if already_imported {
            return Ok(false);
        }
        for (position, asset, decimals, owned) in asset_positions {
            tx.execute(
                "INSERT OR REPLACE INTO asset_scales VALUES (?, ?)",
                params![asset.0.as_str(), i32::from(*decimals)],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO positions VALUES (?, ?, ?)",
                params![position.0.as_str(), asset.0.as_str(), owned],
            )?;
        }
        Self::write_transactions(&tx, transactions, &mut on_progress)?;
        tx.execute(
            "INSERT INTO imported_files VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
            params![source, content_hash, i64::try_from(transactions.len())?],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn has_imported_file(&self, source: &str, content_hash: &str) -> anyhow::Result<bool> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM imported_files WHERE source=? AND content_hash=?)",
            params![source, content_hash],
            |row| row.get(0),
        )?)
    }

    fn write_transactions(
        tx: &duckdb::Transaction<'_>,
        transactions: &[(String, Transaction)],
        on_progress: &mut impl FnMut(usize, usize),
    ) -> anyhow::Result<()> {
        for (index, (id, transaction)) in transactions.iter().enumerate() {
            tx.execute(
                "INSERT OR REPLACE INTO transactions VALUES (?, ?)",
                params![id, transaction.datetime.naive_utc()],
            )?;
            tx.execute(
                "DELETE FROM transaction_effects WHERE transaction_id = ?",
                [id],
            )?;
            for (order, (effect, sign)) in transaction
                .inputs
                .iter()
                .map(|e| (e, -1i64))
                .chain(transaction.outputs.iter().map(|e| (e, 1i64)))
                .enumerate()
            {
                let amount = i64::try_from(effect.amount)? * sign;
                tx.execute(
                    "INSERT INTO transaction_effects VALUES (?, ?, ?, ?)",
                    params![id, effect.position_id.0, amount, i32::try_from(order)?],
                )?;
            }
            let done = index + 1;
            if done % 1_000 == 0 || done == transactions.len() {
                on_progress(done, transactions.len());
            }
        }
        Ok(())
    }

    pub fn save_price(&self, point: &AssetPricePoint) -> anyhow::Result<()> {
        self.save_prices(std::slice::from_ref(point))
    }

    pub fn save_prices(&self, points: &[AssetPricePoint]) -> anyhow::Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        for point in points {
            Self::insert_price(&tx, point)?;
        }
        tx.commit()?;
        Ok(())
    }

    fn insert_price(tx: &duckdb::Transaction<'_>, point: &AssetPricePoint) -> anyhow::Result<()> {
        anyhow::ensure!(
            point.price.is_finite() && point.price > 0.0,
            "invalid asset price"
        );
        tx.execute(
            "INSERT OR REPLACE INTO asset_prices VALUES (?, ?, ?, ?)",
            params![
                point.datetime.naive_utc(),
                point.asset_id.0.as_str(),
                point.vs_asset_id.0.as_str(),
                point.price
            ],
        )?;
        Ok(())
    }

    /// Save observations and the successfully fetched interval together. Current
    /// spot prices use `save_prices` and do not mark any historical interval.
    pub fn save_historical_prices(
        &self,
        asset: &AssetId,
        quote: &AssetId,
        range: PriceRange,
        points: &[AssetPricePoint],
    ) -> anyhow::Result<()> {
        anyhow::ensure!(range.from < range.to, "price range must have from < to");
        anyhow::ensure!(!points.is_empty(), "historical price response was empty");
        let tx = self.connection.unchecked_transaction()?;
        for point in points {
            anyhow::ensure!(
                point.asset_id == *asset && point.vs_asset_id == *quote,
                "historical price asset or quote does not match requested pair"
            );
            anyhow::ensure!(
                range.from <= point.datetime && point.datetime <= range.to,
                "historical price timestamp lies outside requested range"
            );
            Self::insert_price(&tx, point)?;
        }
        tx.execute(
            "INSERT OR IGNORE INTO price_coverage VALUES (?, ?, ?, ?)",
            params![
                asset.0.as_str(),
                quote.0.as_str(),
                range.from.naive_utc(),
                range.to.naive_utc()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Return only the portions of `wanted` that have never been fetched for
    /// this asset/quote pair. Coverage records the request boundaries rather
    /// than inferring them from returned sample timestamps.
    pub fn missing_price_ranges(
        &self,
        asset: &AssetId,
        quote: &AssetId,
        wanted: PriceRange,
    ) -> anyhow::Result<Vec<PriceRange>> {
        anyhow::ensure!(wanted.from < wanted.to, "price range must have from < to");
        let mut statement = self.connection.prepare(
            "SELECT from_datetime, to_datetime FROM price_coverage \
             WHERE asset_id=? AND vs_asset_id=? AND to_datetime>? AND from_datetime<? \
             ORDER BY from_datetime, to_datetime",
        )?;
        let rows = statement.query_map(
            params![
                asset.0.as_str(),
                quote.0.as_str(),
                wanted.from.naive_utc(),
                wanted.to.naive_utc()
            ],
            |row| {
                Ok((
                    row.get::<_, NaiveDateTime>(0)?,
                    row.get::<_, NaiveDateTime>(1)?,
                ))
            },
        )?;
        let mut cursor = wanted.from;
        let mut missing = Vec::new();
        for row in rows {
            let (from, to) = row?;
            let from = DateTime::from_naive_utc_and_offset(from, Utc);
            let to = DateTime::from_naive_utc_and_offset(to, Utc);
            if cursor < from {
                missing.push(PriceRange {
                    from: cursor,
                    to: from.min(wanted.to),
                });
            }
            cursor = cursor.max(to);
            if cursor >= wanted.to {
                return Ok(missing);
            }
        }
        if cursor < wanted.to {
            missing.push(PriceRange {
                from: cursor,
                to: wanted.to,
            });
        }
        Ok(missing)
    }

    /// Read archived pair observations used when deriving a USD price. The
    /// archive fetcher stores each full calendar month as one coverage range.
    pub fn prices_in_range(
        &self,
        asset: &AssetId,
        quote: &AssetId,
        range: PriceRange,
    ) -> anyhow::Result<Vec<AssetPricePoint>> {
        let mut statement = self.connection.prepare(
            "SELECT datetime, price FROM asset_prices \
             WHERE asset_id=? AND vs_asset_id=? AND datetime>=? AND datetime<? \
             ORDER BY datetime",
        )?;
        let rows = statement.query_map(
            params![
                asset.0.as_str(),
                quote.0.as_str(),
                range.from.naive_utc(),
                range.to.naive_utc()
            ],
            |row| Ok((row.get::<_, NaiveDateTime>(0)?, row.get::<_, f64>(1)?)),
        )?;
        rows.map(|row| {
            let (datetime, price) = row?;
            Ok(AssetPricePoint {
                datetime: DateTime::from_naive_utc_and_offset(datetime, Utc),
                asset_id: asset.clone(),
                vs_asset_id: quote.clone(),
                price,
            })
        })
        .collect()
    }

    /// Cache definitive 404/unsupported results for immutable historical
    /// archive months separately from successful price coverage.
    pub fn has_unavailable_price_period(
        &self,
        provider: &str,
        pair: &str,
        range: PriceRange,
    ) -> anyhow::Result<bool> {
        let count: i64 = self.connection.query_row(
            "SELECT count(*) FROM unavailable_price_periods WHERE provider=? AND pair=? AND from_datetime=? AND to_datetime=?",
            params![provider, pair, range.from.naive_utc(), range.to.naive_utc()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn mark_unavailable_price_period(
        &self,
        provider: &str,
        pair: &str,
        range: PriceRange,
    ) -> anyhow::Result<()> {
        self.connection.execute(
            "INSERT OR IGNORE INTO unavailable_price_periods VALUES (?, ?, ?, ?)",
            params![provider, pair, range.from.naive_utc(), range.to.naive_utc()],
        )?;
        Ok(())
    }

    pub fn balances_at(&self, datetime: DateTime<Utc>) -> anyhow::Result<Vec<PositionBalance>> {
        let mut statement = self.connection.prepare(
            "SELECT e.position_id, SUM(e.amount)::BIGINT FROM transaction_effects e JOIN transactions t ON t.id=e.transaction_id JOIN positions p ON p.position_id=e.position_id WHERE t.datetime <= ? AND p.owned GROUP BY e.position_id ORDER BY e.position_id",
        )?;
        let rows = statement.query_map([datetime.naive_utc()], |row| {
            let position_id: String = row.get(0)?;
            let amount: i64 = row.get(1)?;
            Ok(PositionBalance {
                position_id: PositionId(position_id),
                datetime,
                amount: i128::from(amount),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Asset totals in human units at the specified instant, across owned positions.
    pub fn asset_amounts_at(&self, datetime: DateTime<Utc>) -> anyhow::Result<Vec<(AssetId, f64)>> {
        let mut statement = self.connection.prepare(
            "SELECT p.asset_id, SUM(e.amount)::DOUBLE / POWER(10, s.decimals) \
             FROM transaction_effects e \
             JOIN transactions t ON t.id=e.transaction_id \
             JOIN positions p ON p.position_id=e.position_id \
             JOIN asset_scales s ON s.asset_id=p.asset_id \
             WHERE p.owned AND t.datetime <= ? \
             GROUP BY p.asset_id, s.decimals ORDER BY p.asset_id",
        )?;
        let rows = statement.query_map([datetime.naive_utc()], |row| {
            let asset_name: String = row.get(0)?;
            let amount: f64 = row.get(1)?;
            let asset_id = AssetId::str(&asset_name);
            Ok((asset_id, amount))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Credit-line (non-owned) asset totals in human units at the specified
    /// instant. The Nexo adapter books loan-domain rows (card loan
    /// withdrawals, card spends, loan interest) on non-owned `NEXO-CREDIT:*`
    /// positions, so this derives the credit-line balance over time from the
    /// same transaction sums. Negative means net borrowed/spent.
    pub fn credit_amounts_at(&self, datetime: DateTime<Utc>) -> anyhow::Result<Vec<(AssetId, f64)>> {
        let mut statement = self.connection.prepare(
            "SELECT p.asset_id, SUM(e.amount)::DOUBLE / POWER(10, s.decimals) \
             FROM transaction_effects e \
             JOIN transactions t ON t.id=e.transaction_id \
             JOIN positions p ON p.position_id=e.position_id \
             JOIN asset_scales s ON s.asset_id=p.asset_id \
             WHERE NOT p.owned AND t.datetime <= ? \
             GROUP BY p.asset_id, s.decimals ORDER BY p.asset_id",
        )?;
        let rows = statement.query_map([datetime.naive_utc()], |row| {
            let asset_name: String = row.get(0)?;
            let amount: f64 = row.get(1)?;
            let asset_id = AssetId::str(&asset_name);
            Ok((asset_id, amount))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Asset totals in exact smallest units at the specified instant, across
    /// owned positions. No floating-point division: use this when dust
    /// matters (e.g. proving a remainder is real, not DOUBLE noise).
    pub fn asset_units_at(&self, datetime: DateTime<Utc>) -> anyhow::Result<Vec<(AssetId, i128)>> {
        self.summed_units_at(datetime, true)
    }

    /// Credit-line (non-owned) asset totals in exact smallest units.
    pub fn credit_units_at(&self, datetime: DateTime<Utc>) -> anyhow::Result<Vec<(AssetId, i128)>> {
        self.summed_units_at(datetime, false)
    }

    fn summed_units_at(
        &self,
        datetime: DateTime<Utc>,
        owned: bool,
    ) -> anyhow::Result<Vec<(AssetId, i128)>> {
        let mut statement = self.connection.prepare(
            "SELECT p.asset_id, SUM(e.amount)::BIGINT \
             FROM transaction_effects e \
             JOIN transactions t ON t.id=e.transaction_id \
             JOIN positions p ON p.position_id=e.position_id \
             WHERE p.owned = ? AND t.datetime <= ? \
             GROUP BY p.asset_id ORDER BY p.asset_id",
        )?;
        let rows = statement.query_map(params![owned, datetime.naive_utc()], |row| {
            let asset_name: String = row.get(0)?;
            let amount: i64 = row.get(1)?;
            Ok((AssetId::str(&asset_name), i128::from(amount)))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn owned_asset_ids(&self) -> anyhow::Result<Vec<AssetId>> {
        let mut statement = self
            .connection
            .prepare("SELECT DISTINCT asset_id FROM positions WHERE owned ORDER BY asset_id")?;
        let rows = statement.query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(AssetId::str(&name))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Assets with at least one effect on an owned position, and the earliest
    /// transaction that affected each one. Past holdings remain included.
    pub fn owned_assets_first_seen(&self) -> anyhow::Result<Vec<(AssetId, DateTime<Utc>)>> {
        let mut statement = self.connection.prepare(
            "SELECT p.asset_id, MIN(t.datetime) FROM transaction_effects e \
             JOIN transactions t ON t.id=e.transaction_id \
             JOIN positions p ON p.position_id=e.position_id \
             WHERE p.owned GROUP BY p.asset_id ORDER BY p.asset_id",
        )?;
        let rows = statement.query_map([], |row| {
            let asset: String = row.get(0)?;
            let first: NaiveDateTime = row.get(1)?;
            Ok((
                AssetId::str(&asset),
                DateTime::from_naive_utc_and_offset(first, Utc),
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Value owned assets at every transaction and quote-price observation time.
    /// Missing prices for nonzero holdings are an error, never silently omitted.
    pub fn total_value_history(
        &self,
        quote: &AssetId,
    ) -> anyhow::Result<Vec<(DateTime<Utc>, f64)>> {
        let quote = quote.0.as_str();
        let mut statement = self.connection.prepare(
            "WITH times AS (SELECT datetime FROM transactions UNION SELECT datetime FROM asset_prices WHERE vs_asset_id=?), \
             assets AS (SELECT DISTINCT asset_id FROM positions WHERE owned), \
             events AS (SELECT t.datetime, p.asset_id, SUM(e.amount) AS delta \
                 FROM transaction_effects e JOIN transactions t ON t.id=e.transaction_id \
                 JOIN positions p ON p.position_id=e.position_id WHERE p.owned \
                 GROUP BY t.datetime, p.asset_id), \
             holdings AS (SELECT times.datetime, assets.asset_id, \
                 SUM(COALESCE(events.delta, 0)) OVER \
                 (PARTITION BY assets.asset_id ORDER BY times.datetime) AS amount \
                 FROM times CROSS JOIN assets LEFT JOIN events \
                 ON events.datetime=times.datetime AND events.asset_id=assets.asset_id), \
             valued AS (SELECT h.datetime, h.amount / POWER(10, s.decimals) AS amount, \
                 CASE WHEN h.asset_id=? THEN 1.0 ELSE \
                   (SELECT ap.price FROM asset_prices ap WHERE ap.asset_id=h.asset_id \
                    AND ap.vs_asset_id=? AND ap.datetime<=h.datetime \
                    ORDER BY ap.datetime DESC LIMIT 1) END AS price \
                 FROM holdings h JOIN asset_scales s ON s.asset_id=h.asset_id) \
             SELECT datetime, SUM(CASE WHEN amount <> 0 THEN amount*price ELSE 0 END), \
                 SUM(CASE WHEN amount <> 0 AND price IS NULL THEN 1 ELSE 0 END) \
             FROM valued GROUP BY datetime ORDER BY datetime",
        )?;
        let rows = statement.query_map(params![quote, quote, quote], |row| {
            let datetime: NaiveDateTime = row.get(0)?;
            let value: Option<f64> = row.get(1)?;
            let missing: i64 = row.get(2)?;
            Ok((
                DateTime::from_naive_utc_and_offset(datetime, Utc),
                value.unwrap_or(0.0),
                missing,
            ))
        })?;
        let mut history = Vec::new();
        for row in rows {
            let (datetime, value, missing) = row?;
            anyhow::ensure!(
                missing == 0,
                "{missing} held asset(s) have no {} price at {datetime}",
                quote
            );
            history.push((datetime, value));
        }
        Ok(history)
    }
}

/// Database structure containing all domain entities
pub struct Db {
    pub assets: HashMap<AssetId, AssetId>,
    pub positions: HashMap<PositionId, UserPosition>,
    pub products: HashMap<ProductId, Product>,
    pub transactions: HashMap<TransactionId, Transaction>,
}

impl Db {
    pub fn new() -> Self {
        Db {
            assets: HashMap::new(),
            positions: HashMap::new(),
            products: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    #[allow(dead_code)] // Retained for the in-memory Db model.
    fn upsert_position(&mut self, position: &UserPosition) {
        self.positions.insert(position.id.clone(), position.clone());
    }
}

/// Collection of all products
pub struct AllProducts {
    pub products: HashMap<ProductId, Product>,
}

impl AllProducts {
    pub fn new() -> Self {
        AllProducts {
            products: HashMap::new(),
        }
    }

    pub fn insert(&mut self, product: Product) {
        self.products.insert(product.id.clone(), product);
    }

    pub fn get(&self, id: &ProductId) -> Option<&Product> {
        self.products.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TxnEffect;

    fn time(day: u32) -> DateTime<Utc> {
        chrono::TimeZone::with_ymd_and_hms(&Utc, 2025, 1, day, 0, 0, 0).unwrap()
    }

    fn transaction(
        at: DateTime<Utc>,
        position: &PositionId,
        input: u64,
        output: u64,
    ) -> Transaction {
        let effect = |amount| TxnEffect {
            position_id: position.clone(),
            amount,
            datetime: at,
        };
        Transaction {
            datetime: at,
            inputs: if input == 0 {
                vec![]
            } else {
                vec![effect(input)]
            },
            outputs: if output == 0 {
                vec![]
            } else {
                vec![effect(output)]
            },
        }
    }

    #[test]
    fn historical_amounts_and_net_worth_use_owned_positions_and_asset_scales() -> anyhow::Result<()>
    {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let usd = AssetId::str("USD");
        let owned = PositionId::from("owned ETH");
        let external = PositionId::from("external ETH");
        store.save_asset_scale(&eth, 8)?;
        store.save_asset_scale(&usd, 2)?;
        store.save_position_asset(&owned, &eth, true)?;
        store.save_position_asset(&external, &eth, false)?;
        let cash = PositionId::from("cash");
        store.save_position_asset(&cash, &usd, true)?;
        store.save_transaction("deposit", &transaction(time(1), &owned, 0, 200_000_000))?;
        store.save_transaction(
            "counterparty",
            &transaction(time(1), &external, 200_000_000, 0),
        )?;
        store.save_transaction("cash", &transaction(time(1), &cash, 0, 5_000))?;
        store.save_transaction("withdraw", &transaction(time(3), &owned, 50_000_000, 0))?;
        store.save_price(&AssetPricePoint {
            datetime: time(1),
            asset_id: eth.clone(),
            vs_asset_id: usd.clone(),
            price: 100.0,
        })?;
        store.save_price(&AssetPricePoint {
            datetime: time(2),
            asset_id: eth.clone(),
            vs_asset_id: usd.clone(),
            price: 120.0,
        })?;

        let amounts = store.asset_amounts_at(time(3))?;
        assert_eq!(amounts, vec![(eth.clone(), 1.5), (usd.clone(), 50.0)]);
        assert_eq!(
            store.total_value_history(&usd)?,
            vec![(time(1), 250.0), (time(2), 290.0), (time(3), 230.0),]
        );

        // Reimporting a corrected transaction replaces effects instead of doubling them.
        store.save_transaction("withdraw", &transaction(time(3), &owned, 100_000_000, 0))?;
        assert_eq!(store.asset_amounts_at(time(3))?[0], (eth, 1.0));
        Ok(())
    }

    #[test]
    fn missing_price_is_an_error() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let usd = AssetId::str("USD");
        let position = PositionId::from("eth");
        store.save_asset_scale(&eth, 8)?;
        store.save_position_asset(&position, &eth, true)?;
        store.save_transaction("deposit", &transaction(time(1), &position, 0, 100_000_000))?;
        assert!(store.total_value_history(&usd).is_err());
        Ok(())
    }

    #[test]
    fn price_coverage_fills_older_newer_and_interior_gaps_per_asset() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let btc = AssetId::str("BTC");
        let usd = AssetId::str("USD");
        let wanted = PriceRange::new(time(1), time(10))?;
        assert_eq!(
            store.missing_price_ranges(&eth, &usd, wanted)?,
            vec![wanted]
        );

        let middle = PriceRange::new(time(4), time(6))?;
        store.save_historical_prices(
            &eth,
            &usd,
            middle,
            &[AssetPricePoint {
                datetime: time(5),
                asset_id: eth.clone(),
                vs_asset_id: usd.clone(),
                price: 100.0,
            }],
        )?;
        assert_eq!(
            store.missing_price_ranges(&eth, &usd, wanted)?,
            vec![
                PriceRange::new(time(1), time(4))?,
                PriceRange::new(time(6), time(10))?
            ]
        );
        assert_eq!(
            store.missing_price_ranges(&btc, &usd, wanted)?,
            vec![wanted]
        );

        for range in [
            PriceRange::new(time(1), time(4))?,
            PriceRange::new(time(6), time(10))?,
        ] {
            store.save_historical_prices(
                &eth,
                &usd,
                range,
                &[AssetPricePoint {
                    datetime: range.from,
                    asset_id: eth.clone(),
                    vs_asset_id: usd.clone(),
                    price: 90.0,
                }],
            )?;
        }
        assert!(store.missing_price_ranges(&eth, &usd, wanted)?.is_empty());
        assert_eq!(
            store.missing_price_ranges(&eth, &usd, PriceRange::new(time(1), time(12))?)?,
            vec![PriceRange::new(time(10), time(12))?]
        );
        Ok(())
    }

    #[test]
    fn spot_prices_do_not_mark_historical_coverage_and_failed_import_rolls_back()
    -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let usd = AssetId::str("USD");
        let wanted = PriceRange::new(time(1), time(3))?;
        let spot = AssetPricePoint {
            datetime: time(2),
            asset_id: eth.clone(),
            vs_asset_id: usd.clone(),
            price: 100.0,
        };
        store.save_price(&spot)?;
        assert_eq!(
            store.missing_price_ranges(&eth, &usd, wanted)?,
            vec![wanted]
        );

        let invalid = AssetPricePoint {
            price: -1.0,
            ..spot
        };
        assert!(
            store
                .save_historical_prices(&eth, &usd, wanted, &[invalid])
                .is_err()
        );
        assert_eq!(
            store.missing_price_ranges(&eth, &usd, wanted)?,
            vec![wanted]
        );
        Ok(())
    }

    #[test]
    fn first_seen_uses_owned_transaction_effects_even_for_past_holdings() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let btc = AssetId::str("BTC");
        let unused = AssetId::str("UNUSED");
        let owned = PositionId::from("owned ETH");
        let external = PositionId::from("external BTC");
        store.save_position_asset(&owned, &eth, true)?;
        store.save_position_asset(&external, &btc, false)?;
        store.save_position_asset(&PositionId::from("unused"), &unused, true)?;
        store.save_transaction("buy", &transaction(time(2), &owned, 0, 100))?;
        store.save_transaction("sell", &transaction(time(3), &owned, 100, 0))?;
        store.save_transaction("other", &transaction(time(1), &external, 0, 100))?;
        assert_eq!(
            store.owned_assets_first_seen()?,
            vec![(eth.clone(), time(2))]
        );
        store.save_transaction("older import", &transaction(time(1), &owned, 0, 50))?;
        assert_eq!(store.owned_assets_first_seen()?, vec![(eth, time(1))]);
        Ok(())
    }

    #[test]
    fn legacy_dense_prices_seed_coverage_but_sparse_spot_prices_do_not() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let usd = AssetId::str("USD");
        for day in [2, 3, 4, 10] {
            store.save_price(&AssetPricePoint {
                datetime: time(day),
                asset_id: eth.clone(),
                vs_asset_id: usd.clone(),
                price: 100.0,
            })?;
        }
        store.infer_legacy_price_coverage()?;
        assert_eq!(
            store.missing_price_ranges(&eth, &usd, PriceRange::new(time(1), time(11))?)?,
            vec![
                PriceRange::new(time(1), time(2))?,
                PriceRange::new(time(4), time(11))?
            ]
        );
        Ok(())
    }

    #[test]
    fn fetched_ranges_survive_reopening_the_database() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("prices.duckdb");
        let eth = AssetId::str("ETH");
        let usd = AssetId::str("USD");
        let range = PriceRange::new(time(2), time(5))?;
        {
            let store = Store::open(&path)?;
            store.save_historical_prices(
                &eth,
                &usd,
                range,
                &[AssetPricePoint {
                    datetime: time(3),
                    asset_id: eth.clone(),
                    vs_asset_id: usd.clone(),
                    price: 100.0,
                }],
            )?;
        }
        let store = Store::open(&path)?;
        assert!(store.missing_price_ranges(&eth, &usd, range)?.is_empty());
        assert_eq!(
            store.missing_price_ranges(&eth, &usd, PriceRange::new(time(1), time(6))?)?,
            vec![
                PriceRange::new(time(1), time(2))?,
                PriceRange::new(time(5), time(6))?
            ]
        );
        Ok(())
    }

    #[test]
    fn unavailable_archive_pair_month_is_cached_across_reopen() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("prices.duckdb");
        let january = PriceRange::new(time(1), time(2))?;
        let february = PriceRange::new(time(2), time(3))?;
        {
            let store = Store::open(&path)?;
            store.mark_unavailable_price_period("binance-vision", "NEXOUSD", january)?;
            assert!(store.has_unavailable_price_period("binance-vision", "NEXOUSD", january)?);
            assert!(!store.has_unavailable_price_period("binance-vision", "NEXOUSD", february)?);
        }
        let store = Store::open(&path)?;
        assert!(store.has_unavailable_price_period("binance-vision", "NEXOUSD", january)?);
        assert!(!store.has_unavailable_price_period("kucoin", "NEXOUSD", january)?);
        Ok(())
    }

    #[test]
    fn file_hash_is_committed_only_after_all_rows_succeed() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let position = PositionId::from("NEXO:ETH");
        let assets = [(position.clone(), eth.clone(), 8, true)];
        let valid = ("first".to_string(), transaction(time(1), &position, 0, 100));
        let invalid = (
            "second".to_string(),
            transaction(time(2), &position, 0, u64::MAX),
        );
        assert!(
            store
                .save_file_import_with_progress(
                    "NEXO",
                    "same-file-hash",
                    &assets,
                    &[valid.clone(), invalid],
                    |_, _| {},
                )
                .is_err()
        );
        assert!(!store.has_imported_file("NEXO", "same-file-hash")?);
        assert!(store.owned_asset_ids()?.is_empty());
        assert!(store.balances_at(time(3))?.is_empty());

        assert!(store.save_file_import_with_progress(
            "NEXO",
            "same-file-hash",
            &assets,
            &[valid.clone()],
            |_, _| {},
        )?);
        assert!(store.has_imported_file("NEXO", "same-file-hash")?);
        assert!(!store.save_file_import_with_progress(
            "NEXO",
            "same-file-hash",
            &assets,
            &[valid],
            |_, _| {},
        )?);
        assert_eq!(store.balances_at(time(3))?.len(), 1);
        Ok(())
    }
}
