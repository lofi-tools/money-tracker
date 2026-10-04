use std::{collections::HashMap, path::Path};

use crate::{AssetId, PositionBalance, PositionId, Product, ProductId, Transaction, TransactionId, UserPosition};
use crate::history::AssetPricePoint;
use chrono::{DateTime, NaiveDateTime, Utc};
use duckdb::{params, Connection};

/// Local DuckDB persistence for transactions, position ownership/currency, and prices.
pub struct Store {
    connection: Connection,
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
             CREATE TABLE IF NOT EXISTS positions (position_id VARCHAR PRIMARY KEY, asset_id JSON NOT NULL, owned BOOLEAN NOT NULL);
             CREATE TABLE IF NOT EXISTS transaction_effects (transaction_id VARCHAR NOT NULL, position_id VARCHAR NOT NULL, amount BIGINT NOT NULL, effect_order INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS asset_prices (datetime TIMESTAMP NOT NULL, asset_id JSON NOT NULL, vs_asset_id JSON NOT NULL, price DOUBLE NOT NULL, PRIMARY KEY(datetime, asset_id, vs_asset_id));",
        )?;
        Ok(())
    }

    /// Register a position's currency and ownership. Counterparty positions are
    /// persisted with `owned = false` and excluded from net-worth valuation.
    pub fn save_position_asset(&self, position_id: &PositionId, asset_id: &AssetId, owned: bool) -> anyhow::Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO positions VALUES (?, ?::JSON, ?)",
            params![position_id.0, serde_json::to_string(asset_id)?, owned],
        )?;
        Ok(())
    }

    /// Persist a transaction and signed effects atomically. `id` should be the
    /// provider's stable transaction ID for idempotent imports.
    pub fn save_transaction(&self, id: &str, transaction: &Transaction) -> anyhow::Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO transactions VALUES (?, ?)",
            params![id, transaction.datetime.naive_utc()],
        )?;
        tx.execute("DELETE FROM transaction_effects WHERE transaction_id = ?", [id])?;
        let mut order = 0i32;
        for (effect, sign) in transaction.inputs.iter().map(|e| (e, -1i64))
            .chain(transaction.outputs.iter().map(|e| (e, 1i64)))
        {
            let amount = i64::try_from(effect.amount)? * sign;
            tx.execute(
                "INSERT INTO transaction_effects VALUES (?, ?, ?, ?)",
                params![id, effect.position_id.0, amount, order],
            )?;
            order += 1;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn save_price(&self, point: &AssetPricePoint) -> anyhow::Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO asset_prices VALUES (?, ?::JSON, ?::JSON, ?)",
            params![point.datetime.naive_utc(), serde_json::to_string(&point.asset_id)?, serde_json::to_string(&point.vs_asset_id)?, point.price],
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
            Ok(PositionBalance { position_id: PositionId(position_id), datetime, amount: i128::from(amount) })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Value every owned asset at each ledger event time using the latest price
    /// at or before that time. External counterparty positions are excluded.
    pub fn total_value_history(&self, quote: &AssetId) -> anyhow::Result<Vec<(DateTime<Utc>, f64)>> {
        let quote = serde_json::to_string(quote)?;
        let mut statement = self.connection.prepare(
            "WITH times AS (SELECT DISTINCT datetime FROM transactions), assets AS (SELECT DISTINCT asset_id FROM positions WHERE owned), holdings AS (SELECT times.datetime, assets.asset_id, (SELECT COALESCE(SUM(e.amount), 0) FROM transaction_effects e JOIN transactions t ON t.id=e.transaction_id JOIN positions p ON p.position_id=e.position_id WHERE p.asset_id=assets.asset_id AND p.owned AND t.datetime<=times.datetime) AS amount FROM times CROSS JOIN assets), priced AS (SELECT h.datetime, h.amount, price.price FROM holdings h ASOF JOIN asset_prices price ON h.asset_id=price.asset_id AND h.datetime>=price.datetime WHERE price.vs_asset_id=?::JSON) SELECT datetime, SUM(amount*price) FROM priced GROUP BY datetime ORDER BY datetime",
        )?;
        let rows = statement.query_map([quote], |row| {
            let datetime: NaiveDateTime = row.get(0)?;
            let value: f64 = row.get(1)?;
            Ok((DateTime::from_naive_utc_and_offset(datetime, Utc), value))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
