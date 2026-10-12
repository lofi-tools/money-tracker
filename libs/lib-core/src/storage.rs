use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

use crate::history::AssetPricePoint;
use crate::{
    Account, AccountId, AssetId, Portfolio, PortfolioId, PositionBalance, PositionId, PositionRole,
    Product, ProductId, Transaction, TransactionId, Units, UserPosition,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use duckdb::{Connection, params};

/// Local DuckDB persistence for transactions, position ownership/currency, and prices.
pub struct Store {
    pub(crate) connection: Connection,
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
    /// A consistent read transaction; callers must not write or nest transactions.
    pub fn read_snapshot<T>(&self, read: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
        let transaction = self.connection.unchecked_transaction()?;
        let result = read()?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> anyhow::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> anyhow::Result<Self> {
        crate::migrations::run(&connection)?;
        Ok(Self { connection })
    }

    /// Append a reviewed decision. Current policy applies to all financial periods;
    /// restoring tracking appends a revision and never deletes the review trail.
    pub fn review_asset_tracking(
        &self,
        account: &AccountId,
        asset: &AssetId,
        excluded: bool,
        reason: &str,
        evidence: &str,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !asset.0.trim().is_empty() && !reason.trim().is_empty() && !evidence.trim().is_empty(),
            "review requires asset, reason and evidence"
        );
        let tx = self.connection.unchecked_transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?)",
            [&account.0],
            |r| r.get(0),
        )?;
        anyhow::ensure!(exists, "unknown review account");
        let changed = tx.execute("INSERT INTO asset_tracking_reviews
            SELECT ?, ?, COALESCE(MAX(revision),0)+1, ?, ?, ?, current_timestamp
            FROM asset_tracking_reviews WHERE account_id=? AND asset_id=?
            HAVING NOT EXISTS(SELECT 1 FROM asset_tracking_reviews r
                WHERE r.account_id=? AND r.asset_id=? AND r.excluded=? AND r.reason=? AND r.evidence=?
                AND r.revision=(SELECT MAX(revision) FROM asset_tracking_reviews WHERE account_id=? AND asset_id=?))",
            params![account.0, asset.0, excluded, reason, evidence, account.0, asset.0,
                account.0, asset.0, excluded, reason, evidence, account.0, asset.0])?;
        if changed > 0 {
            let portfolio: String = tx.query_row(
                "SELECT portfolio_id FROM accounts WHERE id=?",
                [&account.0],
                |r| r.get(0),
            )?;
            crate::ui_metadata::bump_revision(&tx, &portfolio.into(), "policy")?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn asset_is_excluded(&self, account: &AccountId, asset: &AssetId) -> anyhow::Result<bool> {
        Ok(self.connection.query_row("SELECT EXISTS(SELECT 1 FROM active_asset_exclusions WHERE account_id=? AND asset_id=?)",
            params![account.0, asset.0], |r| r.get(0))?)
    }

    /// Financial/earnings eligibility. Audit metadata and raw effects remain available.
    /// Liabilities are never hidden by a spam-token asset decision.
    pub fn position_is_tracked(&self, position: &PositionId) -> anyhow::Result<bool> {
        Ok(self.connection.query_row("SELECT p.role IN ('asset','liability') AND
            (p.role='liability' OR NOT EXISTS(SELECT 1 FROM active_asset_exclusions x
                WHERE x.account_id=p.account_id AND x.asset_id=p.asset_id)) FROM positions p WHERE position_id=?",
            [&position.0], |r| r.get(0))?)
    }

    /// Preserve provider history independently of normalized financial amounts/scales.
    /// Existing content-addressed evidence must never be silently replaced.
    pub fn save_account_source_snapshot(
        &self,
        account: &AccountId,
        source: &str,
        hash: &str,
        payload: &str,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.has_account_imported_file(account, source, hash)?,
            "snapshot must belong to completed import"
        );
        self.connection.execute(
            "INSERT OR IGNORE INTO account_source_snapshots VALUES (?, ?, ?, ?)",
            params![account.0, source, hash, payload],
        )?;
        anyhow::ensure!(
            self.account_source_snapshot(account, source, hash)?
                .as_deref()
                == Some(payload),
            "source evidence conflict"
        );
        Ok(())
    }

    pub fn account_source_snapshot(
        &self,
        account: &AccountId,
        source: &str,
        hash: &str,
    ) -> anyhow::Result<Option<String>> {
        let mut statement=self.connection.prepare("SELECT payload FROM account_source_snapshots WHERE account_id=? AND source=? AND content_hash=?")?;
        Ok(statement
            .query_map(params![account.0, source, hash], |r| r.get(0))?
            .next()
            .transpose()?)
    }

    /// Register a position's currency and economic role. The legacy ownership
    /// column is retained for database compatibility; queries use the role.
    pub fn save_position_asset(
        &self,
        position_id: &PositionId,
        asset_id: &AssetId,
        role: PositionRole,
    ) -> anyhow::Result<()> {
        Self::register_position(&self.connection, position_id, asset_id, role, None)
    }

    fn register_position(
        connection: &Connection,
        position: &PositionId,
        asset: &AssetId,
        role: PositionRole,
        account: Option<&AccountId>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !position.0.is_empty() && !asset.0.is_empty(),
            "empty position/asset ID"
        );
        let existing: Vec<(String, String, Option<String>)> = connection
            .prepare("SELECT asset_id, role, account_id FROM positions WHERE position_id=?")?
            .query_map([&position.0], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        if let Some((old_asset, old_role, old_account)) = existing.first() {
            anyhow::ensure!(
                old_asset == &asset.0 && old_role == role.as_str(),
                "position identity/role conflict: {}",
                position.0
            );
            if let Some(account) = account {
                anyhow::ensure!(
                    old_account.as_ref() == Some(&account.0),
                    "position account conflict: {}",
                    position.0
                );
            }
            return Ok(());
        }
        let account_id = if role == PositionRole::ExternalCounterparty {
            anyhow::ensure!(
                account.is_none(),
                "external counterparties cannot be owned accounts"
            );
            None
        } else if let Some(account) = account {
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?)",
                [&account.0],
                |r| r.get(0),
            )?;
            anyhow::ensure!(exists, "unknown account {}", account.0);
            Some(account.0.clone())
        } else {
            let account = crate::identity::legacy_account(&position.0);
            connection.execute("INSERT OR IGNORE INTO accounts (id, portfolio_id, provider_id, label, source_reference) VALUES (?, ?, ?, ?, ?)",
                params![account.id.0, account.portfolio_id.0, account.provider_id.0, account.label, account.source_reference])?;
            Some(account.id.0)
        };
        connection.execute("INSERT INTO positions (position_id, asset_id, owned, role, account_id) VALUES (?, ?, ?, ?, ?)",
            params![position.0, asset.0, role == PositionRole::Asset, role.as_str(), account_id])?;
        crate::migrations::seed_ui_metadata(connection)?;
        Ok(())
    }

    /// Repeat registrations are safe; changing scale requires a separate explicit conversion.
    pub fn save_asset_scale(&self, asset_id: &AssetId, decimals: u8) -> anyhow::Result<()> {
        Self::register_scale(&self.connection, asset_id, decimals)?;
        crate::migrations::seed_ui_metadata(&self.connection)
    }

    fn register_scale(connection: &Connection, asset: &AssetId, scale: u8) -> anyhow::Result<()> {
        connection.execute(
            "INSERT OR IGNORE INTO asset_scales (asset_id, decimals) VALUES (?, ?)",
            params![asset.0, i32::from(scale)],
        )?;
        let existing: i32 = connection.query_row(
            "SELECT decimals FROM asset_scales WHERE asset_id=?",
            [&asset.0],
            |r| r.get(0),
        )?;
        anyhow::ensure!(
            existing == i32::from(scale),
            "asset scale conflict for {}: stored {existing}, requested {scale}",
            asset.0
        );
        Ok(())
    }

    pub fn save_portfolio(&self, portfolio: &Portfolio) -> anyhow::Result<()> {
        anyhow::ensure!(!portfolio.id.0.is_empty(), "empty portfolio ID");
        self.connection.execute("INSERT INTO portfolios (id, label, reporting_asset_id) VALUES (?, ?, ?) ON CONFLICT(id) DO UPDATE SET label=excluded.label, reporting_asset_id=excluded.reporting_asset_id",
            params![portfolio.id.0, portfolio.label, portfolio.reporting_asset.0])?;
        Ok(())
    }

    pub fn portfolios(&self) -> anyhow::Result<Vec<Portfolio>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, label, reporting_asset_id FROM portfolios ORDER BY id")?;
        Ok(statement
            .query_map([], |r| {
                Ok(Portfolio {
                    id: PortfolioId(r.get(0)?),
                    label: r.get(1)?,
                    reporting_asset: AssetId(r.get(2)?),
                })
            })?
            .collect::<Result<_, _>>()?)
    }

    pub fn save_account(&self, account: &Account) -> anyhow::Result<()> {
        anyhow::ensure!(
            !account.id.0.is_empty() && !account.provider_id.0.is_empty(),
            "empty account/provider ID"
        );
        let tx = self.connection.unchecked_transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM portfolios WHERE id=?)",
            [&account.portfolio_id.0],
            |r| r.get(0),
        )?;
        anyhow::ensure!(exists, "unknown portfolio {}", account.portfolio_id.0);
        tx.execute("INSERT OR IGNORE INTO accounts (id, portfolio_id, provider_id, label, source_reference) VALUES (?, ?, ?, ?, ?)", params![account.id.0, account.portfolio_id.0, account.provider_id.0, account.label, account.source_reference])?;
        let (portfolio, provider): (String, String) = tx.query_row(
            "SELECT portfolio_id, provider_id FROM accounts WHERE id=?",
            [&account.id.0],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        anyhow::ensure!(
            portfolio == account.portfolio_id.0 && provider == account.provider_id.0,
            "account ownership/provider conflict"
        );
        tx.execute(
            "UPDATE accounts SET label=?, source_reference=? WHERE id=?",
            params![account.label, account.source_reference, account.id.0],
        )?;
        crate::migrations::seed_ui_metadata(&tx)?;
        tx.commit()?;
        Ok(())
    }

    pub fn accounts(&self, portfolio: &PortfolioId) -> anyhow::Result<Vec<Account>> {
        let mut statement = self.connection.prepare("SELECT id, provider_id, label, source_reference FROM accounts WHERE portfolio_id=? ORDER BY id")?;
        Ok(statement
            .query_map([&portfolio.0], |r| {
                Ok(Account {
                    id: AccountId(r.get(0)?),
                    portfolio_id: portfolio.clone(),
                    provider_id: crate::ProviderId(r.get(1)?),
                    label: r.get(2)?,
                    source_reference: r.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?)
    }

    /// Canonical position ID for an account-local source reference.
    pub fn account_position_id(account: &AccountId, source_id: &PositionId) -> PositionId {
        PositionId(crate::identity::scoped_id(
            account,
            "position",
            &source_id.0,
        ))
    }

    pub fn has_account_imported_file(
        &self,
        account: &AccountId,
        source: &str,
        hash: &str,
    ) -> anyhow::Result<bool> {
        self.has_imported_file(&crate::identity::scoped_id(account, "source", source), hash)
    }

    pub fn account_balance_observations(
        &self,
        account: &AccountId,
        source: &str,
        hash: &str,
    ) -> anyhow::Result<Vec<PositionBalance>> {
        self.balance_observations(&crate::identity::scoped_id(account, "source", source), hash)
    }

    /// Namespace positions, transaction IDs and file markers by stable account identity.
    /// Legacy provider-wide helpers retain their old IDs for backwards-compatible replay.
    pub fn save_account_import(
        &self,
        account: &AccountId,
        source: &str,
        hash: &str,
        positions: &[(PositionId, AssetId, u8, PositionRole)],
        transactions: &[(String, Transaction)],
        observations: &[PositionBalance],
    ) -> anyhow::Result<bool> {
        let position_id =
            |id: &PositionId| PositionId(crate::identity::scoped_id(account, "position", &id.0));
        let positions: Vec<_> = positions
            .iter()
            .map(|(p, a, s, r)| (position_id(p), a.clone(), *s, *r))
            .collect();
        let transactions: Vec<_> = transactions
            .iter()
            .map(|(id, t)| {
                let mut t = t.clone();
                for effect in t.inputs.iter_mut().chain(t.outputs.iter_mut()) {
                    effect.position_id = position_id(&effect.position_id);
                }
                (crate::identity::scoped_id(account, "transaction", id), t)
            })
            .collect();
        let observations: Vec<_> = observations
            .iter()
            .map(|o| PositionBalance {
                position_id: position_id(&o.position_id),
                datetime: o.datetime,
                amount: o.amount.clone(),
            })
            .collect();
        self.import_with_account(
            &crate::identity::scoped_id(account, "source", source),
            hash,
            &positions,
            &transactions,
            &observations,
            Some(account),
            |_, _| {},
        )
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
        asset_positions: &[(PositionId, AssetId, u8, PositionRole)],
        transactions: &[(String, Transaction)],
        on_progress: impl FnMut(usize, usize),
    ) -> anyhow::Result<bool> {
        self.save_file_import_with_observations(
            source,
            content_hash,
            asset_positions,
            transactions,
            &[],
            on_progress,
        )
    }

    /// Commit independent balance observations with the ledger and import marker.
    /// Observations are reference data; they never contribute transaction effects.
    pub fn save_file_import_with_observations(
        &self,
        source: &str,
        content_hash: &str,
        asset_positions: &[(PositionId, AssetId, u8, PositionRole)],
        transactions: &[(String, Transaction)],
        observations: &[PositionBalance],
        on_progress: impl FnMut(usize, usize),
    ) -> anyhow::Result<bool> {
        self.import_with_account(
            source,
            content_hash,
            asset_positions,
            transactions,
            observations,
            None,
            on_progress,
        )
    }

    fn import_with_account(
        &self,
        source: &str,
        content_hash: &str,
        asset_positions: &[(PositionId, AssetId, u8, PositionRole)],
        transactions: &[(String, Transaction)],
        observations: &[PositionBalance],
        account: Option<&AccountId>,
        mut on_progress: impl FnMut(usize, usize),
    ) -> anyhow::Result<bool> {
        let tx = self.connection.unchecked_transaction()?;
        if let Some(account) = account {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?)",
                [&account.0],
                |r| r.get(0),
            )?;
            anyhow::ensure!(exists, "unknown account {}", account.0);
        }
        let already_imported: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM imported_files WHERE source=? AND content_hash=?)",
            params![source, content_hash],
            |row| row.get(0),
        )?;
        if already_imported {
            return Ok(false);
        }
        for (position, asset, decimals, role) in asset_positions {
            Self::register_scale(&tx, asset, *decimals)?;
            Self::register_position(
                &tx,
                position,
                asset,
                *role,
                if *role == PositionRole::ExternalCounterparty {
                    None
                } else {
                    account
                },
            )?;
        }
        Self::write_transactions(&tx, transactions, &mut on_progress)?;
        for balance in observations {
            tx.execute(
                "INSERT INTO balance_observations (source, content_hash, position_id, datetime, amount) VALUES (?, ?, ?, ?, ?)",
                params![
                    source,
                    content_hash,
                    balance.position_id.0,
                    balance.datetime.naive_utc(),
                    balance.amount.to_string()
                ],
            )?;
        }
        tx.execute(
            "INSERT INTO imported_files (source, content_hash, transaction_count, completed_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
            params![source, content_hash, i64::try_from(transactions.len())?],
        )?;
        crate::migrations::seed_ui_metadata(&tx)?;
        tx.commit()?;
        Ok(true)
    }

    pub fn balance_observations(
        &self,
        source: &str,
        content_hash: &str,
    ) -> anyhow::Result<Vec<PositionBalance>> {
        let mut statement = self.connection.prepare(
            "SELECT position_id, datetime, amount FROM balance_observations WHERE source=? AND content_hash=?",
        )?;
        let rows = statement
            .query_map(params![source, content_hash], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, NaiveDateTime>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(position, datetime, amount)| {
                Ok(PositionBalance {
                    position_id: PositionId(position),
                    datetime: datetime.and_utc(),
                    amount: amount.parse()?,
                })
            })
            .collect()
    }

    /// Sum the complete persisted ledger, excluding external counterparties.
    pub fn latest_balances(&self) -> anyhow::Result<Vec<PositionBalance>> {
        let latest: Option<NaiveDateTime> =
            self.connection
                .query_row("SELECT MAX(datetime) FROM transactions", [], |row| {
                    row.get(0)
                })?;
        match latest {
            Some(datetime) => self.balances_at(datetime.and_utc()),
            None => Ok(Vec::new()),
        }
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
        let mut affected_portfolios = std::collections::BTreeSet::new();
        for (index, (id, transaction)) in transactions.iter().enumerate() {
            let old_portfolios = tx.prepare("SELECT DISTINCT a.portfolio_id FROM transaction_effects e JOIN positions p ON p.position_id=e.position_id JOIN accounts a ON a.id=p.account_id WHERE e.transaction_id=?")?
                .query_map([id],|r| r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
            affected_portfolios.extend(old_portfolios);
            tx.execute(
                "INSERT OR REPLACE INTO transactions (id, datetime) VALUES (?, ?)",
                params![id, transaction.datetime.naive_utc()],
            )?;
            tx.execute(
                "DELETE FROM transaction_effects WHERE transaction_id = ?",
                [id],
            )?;
            for (order, (effect, sign)) in transaction
                .inputs
                .iter()
                .map(|e| (e, -1i32))
                .chain(transaction.outputs.iter().map(|e| (e, 1i32)))
                .enumerate()
            {
                anyhow::ensure!(
                    !effect.amount.is_negative(),
                    "effect magnitude must be nonnegative"
                );
                let exists: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM positions WHERE position_id=?)",
                    [&effect.position_id.0],
                    |r| r.get(0),
                )?;
                anyhow::ensure!(exists, "unknown effect position {}", effect.position_id.0);
                let owner: Option<String> = tx.query_row("SELECT a.portfolio_id FROM positions p LEFT JOIN accounts a ON a.id=p.account_id WHERE p.position_id=?",[&effect.position_id.0],|r|r.get(0))?;
                if let Some(owner) = owner {
                    affected_portfolios.insert(owner);
                }
                let amount = (&effect.amount * sign).to_string();
                tx.execute(
                    "INSERT INTO transaction_effects (transaction_id, position_id, amount, effect_order) VALUES (?, ?, ?, ?)",
                    params![id, effect.position_id.0, amount, i32::try_from(order)?],
                )?;
            }
            let done = index + 1;
            if done % 1_000 == 0 || done == transactions.len() {
                on_progress(done, transactions.len());
            }
        }
        for portfolio in affected_portfolios {
            crate::ui_metadata::bump_revision(tx, &portfolio.into(), "ledger")?;
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
        self.position_balances_at(datetime, None, None, false)
    }

    pub fn portfolio_balances_at(
        &self,
        portfolio: &PortfolioId,
        datetime: DateTime<Utc>,
    ) -> anyhow::Result<Vec<PositionBalance>> {
        self.position_balances_at(datetime, Some(portfolio), None, false)
    }

    /// Signed asset and liability buckets for one account; counterparties are excluded.
    pub fn account_balances_at(
        &self,
        account: &AccountId,
        datetime: DateTime<Utc>,
    ) -> anyhow::Result<Vec<PositionBalance>> {
        self.position_balances_at(datetime, None, Some(account), false)
    }

    /// Unfiltered audit quantities, including excluded assets; never use for totals.
    pub fn account_audit_balances_at(
        &self,
        account: &AccountId,
        datetime: DateTime<Utc>,
    ) -> anyhow::Result<Vec<PositionBalance>> {
        self.position_balances_at(datetime, None, Some(account), true)
    }

    fn position_balances_at(
        &self,
        datetime: DateTime<Utc>,
        portfolio: Option<&PortfolioId>,
        account: Option<&AccountId>,
        include_excluded: bool,
    ) -> anyhow::Result<Vec<PositionBalance>> {
        let mut statement = self.connection.prepare("SELECT e.position_id, e.amount FROM transaction_effects e JOIN transactions t ON t.id=e.transaction_id JOIN positions p ON p.position_id=e.position_id LEFT JOIN accounts a ON a.id=p.account_id WHERE t.datetime<=? AND p.role IN ('asset','liability') AND (? IS NOT NULL OR p.role='asset') AND (? IS NULL OR a.portfolio_id=?) AND (? IS NULL OR p.account_id=?) AND (? OR (p.role='liability' OR NOT EXISTS(SELECT 1 FROM active_asset_exclusions x WHERE x.account_id=p.account_id AND x.asset_id=p.asset_id))) ORDER BY e.position_id")?;
        let filter = portfolio.map(|p| p.0.as_str());
        let account_filter = account.map(|a| a.0.as_str());
        let rows = statement.query_map(
            params![
                datetime.naive_utc(),
                account_filter,
                filter,
                filter,
                account_filter,
                account_filter,
                include_excluded
            ],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )?;
        let mut sums = BTreeMap::<String, Units>::new();
        for row in rows {
            let (id, amount) = row?;
            *sums.entry(id).or_default() += amount.parse::<Units>()?;
        }
        Ok(sums
            .into_iter()
            .map(|(id, amount)| PositionBalance {
                position_id: PositionId(id),
                datetime,
                amount,
            })
            .collect())
    }

    pub fn position_account(&self, position: &PositionId) -> anyhow::Result<Option<AccountId>> {
        Ok(self
            .connection
            .query_row(
                "SELECT account_id FROM positions WHERE position_id=?",
                [&position.0],
                |r| r.get::<_, Option<String>>(0),
            )?
            .map(AccountId))
    }

    pub fn position_metadata(
        &self,
    ) -> anyhow::Result<Vec<(PositionId, AssetId, u8, PositionRole)>> {
        self.filtered_position_metadata(None)
    }

    /// Account-scoped metadata for application DTOs. Existing global/audit API remains available.
    pub fn account_position_metadata(
        &self,
        account: &AccountId,
    ) -> anyhow::Result<Vec<(PositionId, AssetId, u8, PositionRole)>> {
        self.filtered_position_metadata(Some(account))
    }

    fn filtered_position_metadata(
        &self,
        account: Option<&AccountId>,
    ) -> anyhow::Result<Vec<(PositionId, AssetId, u8, PositionRole)>> {
        let filter = account.map(|a| a.0.as_str());
        let mut statement = self.connection.prepare(
            "SELECT p.position_id, p.asset_id, s.decimals, p.role FROM positions p JOIN asset_scales s ON s.asset_id=p.asset_id WHERE ? IS NULL OR p.account_id=? ORDER BY p.position_id",
        )?;
        let rows = statement
            .query_map(params![filter, filter], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(position, asset, scale, role)| {
                Ok((
                    PositionId(position),
                    AssetId(asset),
                    u8::try_from(scale)?,
                    PositionRole::try_from(role.as_str())?,
                ))
            })
            .collect()
    }

    /// Decimal precision used to encode an asset's ledger amounts.
    pub fn asset_scale(&self, asset: &AssetId) -> anyhow::Result<u8> {
        let decimals: i32 = self.connection.query_row(
            "SELECT decimals FROM asset_scales WHERE asset_id=?",
            [asset.0.as_str()],
            |row| row.get(0),
        )?;
        Ok(u8::try_from(decimals)?)
    }

    /// Asset totals in human units at the specified instant, across owned positions.
    pub fn asset_amounts_at(&self, datetime: DateTime<Utc>) -> anyhow::Result<Vec<(AssetId, f64)>> {
        self.approximate_amounts(datetime, PositionRole::Asset)
    }

    pub fn credit_amounts_at(
        &self,
        datetime: DateTime<Utc>,
    ) -> anyhow::Result<Vec<(AssetId, f64)>> {
        self.approximate_amounts(datetime, PositionRole::Liability)
    }

    fn approximate_amounts(
        &self,
        datetime: DateTime<Utc>,
        role: PositionRole,
    ) -> anyhow::Result<Vec<(AssetId, f64)>> {
        self.summed_units_at(datetime, role, None)?
            .into_iter()
            .map(|(asset, amount)| {
                let value = amount.approximate(self.asset_scale(&asset)?)?;
                Ok((asset, value))
            })
            .collect()
    }

    pub fn asset_units_at(&self, datetime: DateTime<Utc>) -> anyhow::Result<Vec<(AssetId, Units)>> {
        self.summed_units_at(datetime, PositionRole::Asset, None)
    }

    pub fn credit_units_at(
        &self,
        datetime: DateTime<Utc>,
    ) -> anyhow::Result<Vec<(AssetId, Units)>> {
        self.summed_units_at(datetime, PositionRole::Liability, None)
    }

    pub fn portfolio_units_at(
        &self,
        portfolio: &PortfolioId,
        datetime: DateTime<Utc>,
        role: PositionRole,
    ) -> anyhow::Result<Vec<(AssetId, Units)>> {
        anyhow::ensure!(
            role != PositionRole::ExternalCounterparty,
            "counterparties are not portfolio holdings"
        );
        self.summed_units_at(datetime, role, Some(portfolio))
    }

    fn summed_units_at(
        &self,
        datetime: DateTime<Utc>,
        role: PositionRole,
        portfolio: Option<&PortfolioId>,
    ) -> anyhow::Result<Vec<(AssetId, Units)>> {
        let mut statement = self.connection.prepare("SELECT p.asset_id, e.amount FROM transaction_effects e JOIN transactions t ON t.id=e.transaction_id JOIN positions p ON p.position_id=e.position_id LEFT JOIN accounts a ON a.id=p.account_id WHERE p.role=? AND t.datetime<=? AND (? IS NULL OR a.portfolio_id=?) AND (p.role='liability' OR NOT EXISTS(SELECT 1 FROM active_asset_exclusions x WHERE x.account_id=p.account_id AND x.asset_id=p.asset_id)) ORDER BY p.asset_id")?;
        let filter = portfolio.map(|p| p.0.as_str());
        let rows = statement.query_map(
            params![role.as_str(), datetime.naive_utc(), filter, filter],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )?;
        let mut sums = BTreeMap::<String, Units>::new();
        for row in rows {
            let (asset, amount) = row?;
            *sums.entry(asset).or_default() += amount.parse::<Units>()?;
        }
        Ok(sums
            .into_iter()
            .map(|(asset, amount)| (AssetId(asset), amount))
            .collect())
    }

    pub fn owned_asset_ids(&self) -> anyhow::Result<Vec<AssetId>> {
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT p.asset_id FROM positions p WHERE role = 'asset' AND (p.role='liability' OR NOT EXISTS(SELECT 1 FROM active_asset_exclusions x WHERE x.account_id=p.account_id AND x.asset_id=p.asset_id)) ORDER BY p.asset_id",
        )?;
        let rows = statement.query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(AssetId::str(&name))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Assets with at least one effect on an owned position, and the earliest
    /// transaction that affected each one. Past holdings remain included.
    pub fn owned_assets_first_seen(&self) -> anyhow::Result<Vec<(AssetId, DateTime<Utc>)>> {
        self.assets_first_seen("p.role = 'asset'")
    }

    /// Price eligibility for net worth, including liability-only currencies.
    pub fn portfolio_assets_first_seen(&self) -> anyhow::Result<Vec<(AssetId, DateTime<Utc>)>> {
        self.assets_first_seen("p.role IN ('asset', 'liability')")
    }

    fn assets_first_seen(&self, predicate: &str) -> anyhow::Result<Vec<(AssetId, DateTime<Utc>)>> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT p.asset_id, MIN(t.datetime) FROM transaction_effects e \
             JOIN transactions t ON t.id=e.transaction_id \
             JOIN positions p ON p.position_id=e.position_id \
             WHERE {predicate} AND (p.role='liability' OR NOT EXISTS(SELECT 1 FROM active_asset_exclusions x WHERE x.account_id=p.account_id AND x.asset_id=p.asset_id)) GROUP BY p.asset_id ORDER BY p.asset_id"
        ))?;
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

    /// Value user assets and signed liabilities, excluding external counterparties.
    /// Negative liability balances reduce net worth without reversing their sign.
    /// Missing prices for nonzero balances are an error, never silently omitted.
    pub fn total_value_history(
        &self,
        quote: &AssetId,
    ) -> anyhow::Result<Vec<(DateTime<Utc>, f64)>> {
        self.value_history(quote, None)
    }

    pub fn portfolio_value_history(
        &self,
        portfolio: &PortfolioId,
        quote: &AssetId,
    ) -> anyhow::Result<Vec<(DateTime<Utc>, f64)>> {
        self.value_history(quote, Some(portfolio))
    }

    fn value_history(
        &self,
        quote: &AssetId,
        portfolio: Option<&PortfolioId>,
    ) -> anyhow::Result<Vec<(DateTime<Utc>, f64)>> {
        let filter = portfolio.map(|p| p.0.as_str());
        let mut statement = self.connection.prepare("SELECT t.datetime, p.asset_id, e.amount FROM transactions t JOIN transaction_effects e ON e.transaction_id=t.id JOIN positions p ON p.position_id=e.position_id LEFT JOIN accounts a ON a.id=p.account_id WHERE p.role IN ('asset','liability') AND (? IS NULL OR a.portfolio_id=?) AND (p.role='liability' OR NOT EXISTS(SELECT 1 FROM active_asset_exclusions x WHERE x.account_id=p.account_id AND x.asset_id=p.asset_id)) ORDER BY t.datetime")?;
        let rows = statement.query_map(params![filter, filter], |r| {
            Ok((
                r.get::<_, NaiveDateTime>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut events = BTreeMap::<NaiveDateTime, Vec<(String, Units)>>::new();
        for row in rows {
            let (time, asset, amount) = row?;
            events
                .entry(time)
                .or_default()
                .push((asset, amount.parse()?));
        }
        // Preserve transaction-only cutoffs (including no-effect legacy records).
        let mut times = self.connection.prepare("SELECT t.datetime FROM transactions t WHERE ? IS NULL OR EXISTS(SELECT 1 FROM transaction_effects e JOIN positions p ON p.position_id=e.position_id JOIN accounts a ON a.id=p.account_id WHERE e.transaction_id=t.id AND a.portfolio_id=?) UNION SELECT datetime FROM asset_prices WHERE vs_asset_id=? ORDER BY datetime")?;
        for time in times.query_map(params![filter, filter, &quote.0], |r| {
            r.get::<_, NaiveDateTime>(0)
        })? {
            events.entry(time?).or_default();
        }
        let mut prices = self.connection.prepare("SELECT datetime, asset_id, price FROM asset_prices WHERE vs_asset_id=? ORDER BY datetime")?;
        let prices = prices
            .query_map([&quote.0], |r| {
                Ok((
                    r.get::<_, NaiveDateTime>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, f64>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut scale_query = self
            .connection
            .prepare("SELECT asset_id, decimals FROM asset_scales")?;
        let scales: HashMap<String, u8> = scale_query
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i32>(1)?)))?
            .map(|row| {
                let (asset, scale) = row?;
                Ok((asset, u8::try_from(scale)?))
            })
            .collect::<anyhow::Result<_>>()?;
        let mut price_index = 0;
        let mut current_prices = HashMap::<String, f64>::new();
        let mut balances = BTreeMap::<String, Units>::new();
        let mut history = Vec::new();
        for (time, effects) in events {
            while price_index < prices.len() && prices[price_index].0 <= time {
                current_prices.insert(prices[price_index].1.clone(), prices[price_index].2);
                price_index += 1;
            }
            for (asset, amount) in effects {
                *balances.entry(asset).or_default() += amount;
            }
            let mut value = 0.0;
            for (asset, amount) in &balances {
                if amount.is_zero() {
                    continue;
                }
                let price = if asset == &quote.0 {
                    1.0
                } else {
                    *current_prices.get(asset).ok_or_else(|| {
                        anyhow::anyhow!(
                            "portfolio asset or liability {asset} has no {} price at {time}",
                            quote.0
                        )
                    })?
                };
                value += amount.approximate(
                    *scales
                        .get(asset)
                        .ok_or_else(|| anyhow::anyhow!("missing asset scale for {asset}"))?,
                )? * price;
            }
            anyhow::ensure!(
                value.is_finite(),
                "net worth exceeds approximate valuation range"
            );
            history.push((time.and_utc(), value));
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
        let effect = |amount: u64| TxnEffect {
            position_id: position.clone(),
            amount: amount.into(),
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
    fn reviewed_exclusions_preserve_history_scope_debt_and_restore_on_reopen() -> anyhow::Result<()>
    {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("tracking.duckdb");
        let token = AssetId::str("USD");
        let position = PositionId::from("token");
        let debt = PositionId::from("debt");
        let account = Account {
            id: "review-a".into(),
            portfolio_id: crate::DEFAULT_PORTFOLIO.into(),
            provider_id: "test".into(),
            label: "A".into(),
            source_reference: None,
        };
        let mut other = account.clone();
        other.id = "review-b".into();
        {
            let store = Store::open(&path)?;
            for a in [&account, &other] {
                store.save_account(a)?;
                store.save_account_import(
                    &a.id,
                    "test",
                    "batch",
                    &[
                        (position.clone(), token.clone(), 2, PositionRole::Asset),
                        (debt.clone(), token.clone(), 2, PositionRole::Liability),
                    ],
                    &[
                        ("deposit".into(), transaction(time(1), &position, 0, 100)),
                        ("reward".into(), transaction(time(2), &position, 0, 50)),
                        ("loan".into(), transaction(time(1), &debt, 25, 0)),
                    ],
                    &[],
                )?;
            }
            store.save_account_source_snapshot(
                &account.id,
                "test",
                "batch",
                "raw reward history",
            )?;
            for _ in 0..3 {
                store.review_asset_tracking(
                    &account.id,
                    &token,
                    true,
                    "reviewed spam",
                    "verified source",
                )?;
            }
            assert_eq!(
                store.connection.query_row(
                    "SELECT count(*) FROM asset_tracking_reviews",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            assert_eq!(
                store.account_audit_balances_at(&account.id, time(2))?.len(),
                2
            );
            assert_eq!(store.account_balances_at(&account.id, time(2))?.len(), 1);
            assert!(
                !store.position_is_tracked(&Store::account_position_id(&account.id, &position))?
            );
            assert!(store.position_is_tracked(&Store::account_position_id(&account.id, &debt))?);
            assert_eq!(
                store.asset_units_at(time(2))?,
                vec![(token.clone(), 150.into())]
            );
            assert_eq!(
                store.credit_units_at(time(2))?,
                vec![(token.clone(), (-50).into())]
            );
            assert_eq!(
                store.total_value_history(&token)?,
                vec![(time(1), 0.5), (time(2), 1.0)]
            );
            // Unknown scale/identity needs no invented registration to be excluded.
            store.review_asset_tracking(
                &account.id,
                &AssetId::str("unknown-contract"),
                true,
                "spam",
                "review",
            )?;
            assert!(
                store
                    .asset_scale(&AssetId::str("unknown-contract"))
                    .is_err()
            );
            // Excluding the other account removes pricing/earnings asset eligibility too.
            store.review_asset_tracking(
                &other.id,
                &token,
                true,
                "reviewed spam",
                "verified source",
            )?;
            assert!(store.owned_asset_ids()?.is_empty());
            assert!(store.owned_assets_first_seen()?.is_empty());
            assert_eq!(
                store.portfolio_assets_first_seen()?,
                vec![(token.clone(), time(1))]
            ); // debt stays priced
            assert!(
                store
                    .portfolio_balances_at(&account.portfolio_id, time(2))?
                    .is_empty()
            ); // asset-only API
            assert_eq!(
                store.portfolio_units_at(&account.portfolio_id, time(2), PositionRole::Asset)?,
                vec![]
            );
            assert_eq!(
                store.total_value_history(&token)?,
                vec![(time(1), -0.5), (time(2), -0.5)]
            );
        }
        for _ in 0..3 {
            drop(Store::open(&path)?);
        }
        let store = Store::open(&path)?;
        assert!(store.asset_is_excluded(&account.id, &token)?);
        assert_eq!(
            store
                .account_source_snapshot(&account.id, "test", "batch")?
                .as_deref(),
            Some("raw reward history")
        );
        store.review_asset_tracking(
            &account.id,
            &token,
            false,
            "restored",
            "explicit user decision",
        )?;
        assert_eq!(store.asset_units_at(time(2))?, vec![(token, 150.into())]);
        assert_eq!(
            store
                .connection
                .query_row("SELECT count(*) FROM transaction_effects", [], |r| r
                    .get::<_, i64>(0))?,
            6
        );
        assert_eq!(
            store.connection.query_row(
                "SELECT count(*) FROM asset_tracking_reviews WHERE account_id=? AND asset_id='USD'",
                [&account.id.0],
                |r| r.get::<_, i64>(0)
            )?,
            2
        );
        Ok(())
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
        store.save_position_asset(&owned, &eth, PositionRole::Asset)?;
        store.save_position_asset(&external, &eth, PositionRole::ExternalCounterparty)?;
        let cash = PositionId::from("cash");
        store.save_position_asset(&cash, &usd, PositionRole::Asset)?;
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
    fn liabilities_reduce_net_worth_and_counterparties_never_become_debt() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let usd = AssetId::str("USD");
        let eur = AssetId::str("EUR");
        let unknown = AssetId::str("EXTERNAL_ONLY");
        let cash = PositionId::from("cash");
        let loan = PositionId::from("loan");
        let market = PositionId::from("BINANCE:external:market:EUR");
        let other = PositionId::from("external-only");
        for asset in [&usd, &eur, &unknown] {
            store.save_asset_scale(asset, 2)?;
        }
        store.save_position_asset(&cash, &usd, PositionRole::Asset)?;
        store.save_position_asset(&loan, &eur, PositionRole::Liability)?;
        store.save_position_asset(&market, &eur, PositionRole::ExternalCounterparty)?;
        store.save_position_asset(&other, &unknown, PositionRole::ExternalCounterparty)?;
        store.save_transaction("cash", &transaction(time(1), &cash, 0, 10_000))?;
        store.save_transaction("borrow", &transaction(time(1), &loan, 2_000, 0))?;
        store.save_transaction("market", &transaction(time(1), &market, 0, 50_000))?;
        store.save_transaction(
            "unpriced external",
            &transaction(time(1), &other, 10_000, 0),
        )?;
        for (at, price) in [(time(1), 2.0), (time(2), 3.0)] {
            store.save_price(&AssetPricePoint {
                datetime: at,
                asset_id: eur.clone(),
                vs_asset_id: usd.clone(),
                price,
            })?;
        }
        store.save_transaction("interest", &transaction(time(2), &loan, 100, 0))?;
        store.save_transaction("repay", &transaction(time(3), &loan, 0, 500))?;
        assert_eq!(store.asset_amounts_at(time(3))?, vec![(usd.clone(), 100.0)]);
        assert_eq!(
            store.credit_amounts_at(time(3))?,
            vec![(eur.clone(), -16.0)]
        );
        assert_eq!(
            store.credit_units_at(time(3))?,
            vec![(eur.clone(), (-1_600).into())]
        );
        assert_eq!(
            store.owned_assets_first_seen()?,
            vec![(usd.clone(), time(1))]
        );
        assert_eq!(
            store.portfolio_assets_first_seen()?,
            vec![(eur, time(1)), (usd.clone(), time(1))]
        );
        assert_eq!(
            store.total_value_history(&usd)?,
            vec![(time(1), 60.0), (time(2), 37.0), (time(3), 52.0)]
        );
        Ok(())
    }

    #[test]
    fn liability_only_portfolio_requires_prices_and_can_have_negative_net_worth()
    -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let usd = AssetId::str("USD");
        let debt_asset = AssetId::str("xUSD");
        let debt = PositionId::from("debt");
        store.save_asset_scale(&debt_asset, 2)?;
        store.save_position_asset(&debt, &debt_asset, PositionRole::Liability)?;
        store.save_transaction("borrow", &transaction(time(1), &debt, 1_000, 0))?;
        assert!(store.total_value_history(&usd).is_err());
        assert_eq!(
            store.portfolio_assets_first_seen()?,
            vec![(debt_asset.clone(), time(1))]
        );
        store.save_price(&AssetPricePoint {
            datetime: time(1),
            asset_id: debt_asset,
            vs_asset_id: usd.clone(),
            price: 1.0,
        })?;
        assert_eq!(store.total_value_history(&usd)?, vec![(time(1), -10.0)]);
        store.save_transaction("repaid", &transaction(time(2), &debt, 0, 1_000))?;
        assert_eq!(
            store.total_value_history(&usd)?,
            vec![(time(1), -10.0), (time(2), 0.0)]
        );
        Ok(())
    }

    #[test]
    fn legacy_roles_migrate_without_reimport_and_persist_explicit_classifications()
    -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("legacy.duckdb");
        {
            let connection = Connection::open(&path)?;
            connection.execute_batch(
                "CREATE TABLE positions (position_id VARCHAR PRIMARY KEY, asset_id VARCHAR NOT NULL, owned BOOLEAN NOT NULL);
                 INSERT INTO positions VALUES ('NEXO:USD', 'USD', true), ('NEXO-CREDIT:xUSD', 'xUSD', false), ('BINANCE:external:market:xUSD', 'xUSD', false);
                 CREATE TABLE transactions (id VARCHAR PRIMARY KEY, datetime TIMESTAMP NOT NULL);
                 INSERT INTO transactions VALUES ('cash', '2025-01-01'), ('debt', '2025-01-01'), ('external', '2025-01-01');
                 CREATE TABLE transaction_effects (transaction_id VARCHAR NOT NULL, position_id VARCHAR NOT NULL, amount BIGINT NOT NULL, effect_order INTEGER NOT NULL);
                 INSERT INTO transaction_effects VALUES ('cash', 'NEXO:USD', 10000, 0), ('debt', 'NEXO-CREDIT:xUSD', -2000, 0), ('external', 'BINANCE:external:market:xUSD', 50000, 0);
                 CREATE TABLE asset_scales (asset_id VARCHAR PRIMARY KEY, decimals INTEGER NOT NULL);
                 INSERT INTO asset_scales VALUES ('USD', 2), ('xUSD', 2);
                 CREATE TABLE imported_files (source VARCHAR NOT NULL, content_hash VARCHAR NOT NULL, transaction_count BIGINT NOT NULL, completed_at TIMESTAMP NOT NULL, PRIMARY KEY(source, content_hash));
                 INSERT INTO imported_files VALUES ('NEXO', 'already-imported', 3, '2025-01-01');",
            )?;
        }
        {
            let store = Store::open(&path)?;
            let usd = AssetId::str("USD");
            let xusd = AssetId::str("xUSD");
            assert!(!store.save_file_import_with_progress(
                "NEXO",
                "already-imported",
                &[],
                &[],
                |_, _| {},
            )?);
            store.save_price(&AssetPricePoint {
                datetime: time(1),
                asset_id: xusd,
                vs_asset_id: usd.clone(),
                price: 1.0,
            })?;
            assert_eq!(store.total_value_history(&usd)?, vec![(time(1), 80.0)]);
            // New classifications are explicit, even in the old Nexo namespace.
            store.save_position_asset(
                &PositionId::from("NEXO-CREDIT:explicit-external"),
                &usd,
                PositionRole::ExternalCounterparty,
            )?;
        }
        let store = Store::open(&path)?;
        assert!(store.has_imported_file("NEXO", "already-imported")?);
        assert_eq!(
            store.credit_units_at(time(1))?,
            vec![(AssetId::str("xUSD"), (-2_000).into())]
        );
        let roles: HashMap<_, _> = store
            .position_metadata()?
            .into_iter()
            .map(|(id, _, _, role)| (id.0, role))
            .collect();
        assert_eq!(roles["NEXO:USD"], PositionRole::Asset);
        assert_eq!(roles["NEXO-CREDIT:xUSD"], PositionRole::Liability);
        assert_eq!(
            roles["BINANCE:external:market:xUSD"],
            PositionRole::ExternalCounterparty
        );
        assert_eq!(
            roles["NEXO-CREDIT:explicit-external"],
            PositionRole::ExternalCounterparty
        );
        assert_eq!(
            store.total_value_history(&AssetId::str("USD"))?,
            vec![(time(1), 80.0)]
        );
        Ok(())
    }

    #[test]
    fn missing_price_is_an_error() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let usd = AssetId::str("USD");
        let position = PositionId::from("eth");
        store.save_asset_scale(&eth, 8)?;
        store.save_position_asset(&position, &eth, PositionRole::Asset)?;
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
        store.save_position_asset(&owned, &eth, PositionRole::Asset)?;
        store.save_position_asset(&external, &btc, PositionRole::ExternalCounterparty)?;
        store.save_position_asset(&PositionId::from("unused"), &unused, PositionRole::Asset)?;
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
        crate::migrations::run(&store.connection)?;
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
    fn observations_commit_with_import_and_remain_independent() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let position = PositionId::from("BINANCE:spot:ETH");
        let assets = [(
            position.clone(),
            AssetId::str("ETH"),
            8,
            PositionRole::Asset,
        )];
        let observations = [PositionBalance {
            position_id: position.clone(),
            datetime: time(2),
            amount: i128::MAX.into(),
        }];
        let valid = (
            "deposit".to_owned(),
            transaction(time(1), &position, 0, 100),
        );
        let invalid = (
            "overflow".to_owned(),
            Transaction {
                datetime: time(2),
                inputs: vec![],
                outputs: vec![crate::TxnEffect {
                    position_id: position.clone(),
                    datetime: time(2),
                    amount: (-1).into(),
                }],
            },
        );
        assert!(
            store
                .save_file_import_with_observations(
                    "BINANCE",
                    "fixture",
                    &assets,
                    &[valid.clone(), invalid],
                    &observations,
                    |_, _| {},
                )
                .is_err()
        );
        assert!(!store.has_imported_file("BINANCE", "fixture")?);
        assert!(store.balance_observations("BINANCE", "fixture")?.is_empty());
        assert!(store.latest_balances()?.is_empty());
        assert!(store.save_file_import_with_observations(
            "BINANCE",
            "fixture",
            &assets,
            &[valid],
            &observations,
            |_, _| {},
        )?);
        assert_eq!(
            store.balance_observations("BINANCE", "fixture")?,
            observations
        );
        assert_eq!(store.latest_balances()?[0].amount, 100);
        assert!(!store.save_file_import_with_observations(
            "BINANCE",
            "fixture",
            &[],
            &[],
            &[],
            |_, _| panic!("must skip imported rows"),
        )?);
        assert_eq!(
            store.balance_observations("BINANCE", "fixture")?,
            observations
        );
        Ok(())
    }

    #[test]
    fn file_hash_is_committed_only_after_all_rows_succeed() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let eth = AssetId::str("ETH");
        let position = PositionId::from("NEXO:ETH");
        let assets = [(position.clone(), eth.clone(), 8, PositionRole::Asset)];
        let valid = ("first".to_string(), transaction(time(1), &position, 0, 100));
        let invalid = (
            "second".to_string(),
            Transaction {
                datetime: time(2),
                inputs: vec![],
                outputs: vec![crate::TxnEffect {
                    position_id: position.clone(),
                    datetime: time(2),
                    amount: (-1).into(),
                }],
            },
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
    #[test]
    fn exact_balances_exceed_machine_ranges_and_scales_are_protected() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let asset = AssetId::str("chain:token");
        let position = PositionId::from("large");
        store.save_asset_scale(&asset, 18)?;
        store.save_position_asset(&position, &asset, PositionRole::Asset)?;
        let huge: Units = "100000000000000000000000000000000000000000000000001".parse()?;
        let tx = Transaction {
            datetime: time(1),
            inputs: vec![],
            outputs: vec![TxnEffect {
                position_id: position.clone(),
                amount: huge.clone(),
                datetime: time(1),
            }],
        };
        store.save_transaction("large", &tx)?;
        store.save_transaction("second", &transaction(time(1), &position, 0, u64::MAX))?;
        let mut expected = huge.clone();
        expected += Units::from(u64::MAX);
        assert_eq!(
            store.asset_units_at(time(1))?,
            vec![(asset.clone(), expected.clone())]
        );
        assert_eq!(store.balances_at(time(1))?[0].amount, expected);
        store.save_asset_scale(&asset, 18)?;
        assert!(store.save_asset_scale(&asset, 8).is_err());
        assert_eq!(store.asset_scale(&asset)?, 18);
        assert!(
            store
                .save_file_import_with_progress(
                    "test",
                    "conflicting",
                    &[(position.clone(), asset.clone(), 8, PositionRole::Asset)],
                    &[("large".into(), transaction(time(1), &position, 0, 1))],
                    |_, _| {}
                )
                .is_err()
        );
        assert!(!store.has_imported_file("test", "conflicting")?);
        assert_eq!(store.balances_at(time(1))?[0].amount, expected);
        store.save_price(&AssetPricePoint {
            datetime: time(1),
            asset_id: asset,
            vs_asset_id: AssetId::str("USD"),
            price: 1.0,
        })?;
        assert!(store.total_value_history(&AssetId::str("USD"))?[0].1 > 1e30);
        Ok(())
    }

    #[test]
    fn account_scoped_imports_isolate_ids_markers_and_portfolios() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let asset = AssetId::str("USD");
        let local_position = PositionId::from("spot:USD");
        let input = [(
            local_position.clone(),
            asset.clone(),
            2,
            PositionRole::Asset,
        )];
        for (portfolio, account, amount) in
            [("alice", "account-a", 100u64), ("bob", "account-b", 200u64)]
        {
            let portfolio = Portfolio {
                id: portfolio.into(),
                label: portfolio.into(),
                reporting_asset: asset.clone(),
            };
            store.save_portfolio(&portfolio)?;
            let account = Account {
                id: account.into(),
                portfolio_id: portfolio.id.clone(),
                provider_id: crate::ProviderId::from("nexo"),
                label: "Account".into(),
                source_reference: Some("provider-reference".into()),
            };
            store.save_account(&account)?;
            let txs = [(
                "same-provider-id".into(),
                transaction(time(1), &local_position, 0, amount),
            )];
            let observations = [PositionBalance {
                position_id: local_position.clone(),
                datetime: time(1),
                amount: amount.into(),
            }];
            assert!(store.save_account_import(
                &account.id,
                "export",
                "same-hash",
                &input,
                &txs,
                &observations
            )?);
            assert!(!store.save_account_import(
                &account.id,
                "export",
                "same-hash",
                &input,
                &txs,
                &observations
            )?);
            assert_eq!(
                store.portfolio_units_at(&portfolio.id, time(1), PositionRole::Asset)?,
                vec![(asset.clone(), amount.into())]
            );
            assert_eq!(
                store.portfolio_balances_at(&portfolio.id, time(1))?.len(),
                1
            );
            assert_eq!(
                store.portfolio_value_history(&portfolio.id, &asset)?,
                vec![(time(1), amount as f64 / 100.0)]
            );
            let mut conflict = account.clone();
            conflict.portfolio_id = crate::DEFAULT_PORTFOLIO.into();
            assert!(store.save_account(&conflict).is_err());
            assert_eq!(store.accounts(&portfolio.id)?[0], account);
            assert!(store.has_account_imported_file(&account.id, "export", "same-hash")?);
            assert_eq!(
                store.account_balance_observations(&account.id, "export", "same-hash")?[0].amount,
                amount
            );
            assert_eq!(
                store.account_balances_at(&account.id, time(1))?[0].amount,
                amount
            );
            assert_eq!(
                store
                    .position_account(&Store::account_position_id(&account.id, &local_position))?,
                Some(account.id)
            );
        }
        assert_eq!(
            store.asset_units_at(time(1))?,
            vec![(asset.clone(), 300.into())]
        );
        assert!(
            store
                .portfolio_units_at(
                    &crate::DEFAULT_PORTFOLIO.into(),
                    time(1),
                    PositionRole::Asset
                )?
                .is_empty()
        );
        let mut account = store.accounts(&PortfolioId::from("alice"))?.remove(0);
        account.label = "Renamed".into();
        store.save_account(&account)?;
        assert_eq!(
            store.portfolio_units_at(&account.portfolio_id, time(1), PositionRole::Asset)?,
            vec![(asset, 100.into())]
        );
        assert!(
            store
                .save_account_import(&AccountId::from("missing"), "export", "hash", &[], &[], &[])
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn legacy_nexo_binance_upgrade_preserves_data_and_user_metadata() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("legacy-both.duckdb");
        {
            let connection = Connection::open(&path)?;
            // Schema immediately before this change, including explicit roles and evidence.
            connection.execute_batch("CREATE TABLE positions (position_id VARCHAR PRIMARY KEY, asset_id VARCHAR NOT NULL, owned BOOLEAN NOT NULL, role VARCHAR NOT NULL);
                CREATE TABLE transaction_effects (transaction_id VARCHAR NOT NULL, position_id VARCHAR NOT NULL, amount BIGINT NOT NULL, effect_order INTEGER NOT NULL);
                CREATE TABLE transactions (id VARCHAR PRIMARY KEY, datetime TIMESTAMP NOT NULL);
                INSERT INTO positions VALUES ('NEXO:USD','USD',true,'asset'), ('NEXO-CREDIT:USD','USD',false,'liability'), ('BINANCE:spot:USD','USD',true,'asset'), ('BINANCE:external:fees:USD','USD',false,'external');
                INSERT INTO transactions VALUES ('NEXO:1','2025-01-01'), ('BINANCE:1','2025-01-01');
                INSERT INTO transaction_effects VALUES ('NEXO:1','NEXO:USD',9223372036854775807,0), ('NEXO:1','NEXO-CREDIT:USD',-2000,1), ('BINANCE:1','BINANCE:spot:USD',9223372036854775807,0), ('BINANCE:1','BINANCE:external:fees:USD',-100,1);")?;
            crate::migrations::run(&connection)?;
            connection.execute_batch("INSERT INTO asset_scales VALUES ('USD',2);
                INSERT INTO imported_files VALUES ('NEXO','nexo-hash',1,'2025-01-01'), ('BINANCE','binance-hash',1,'2025-01-01');
                INSERT INTO balance_observations VALUES ('BINANCE','binance-hash','BINANCE:spot:USD','2025-01-01','9223372036854775807');
                INSERT INTO asset_prices VALUES ('2025-01-01','USD','USD',1);
                INSERT INTO price_coverage VALUES ('USD','USD','2025-01-01','2025-01-02');
                UPDATE accounts SET label='My Nexo' WHERE id='legacy:nexo';
                UPDATE portfolios SET label='My portfolio', reporting_asset_id='EUR' WHERE id='local:default';")?;
        }
        for _ in 0..3 {
            let store = Store::open(&path)?;
            assert!(store.has_imported_file("NEXO", "nexo-hash")?);
            assert!(store.has_imported_file("BINANCE", "binance-hash")?);
            assert_eq!(
                store.asset_units_at(time(1))?[0].1.to_string(),
                "18446744073709551614"
            );
            assert_eq!(
                store.credit_units_at(time(1))?,
                vec![(AssetId::str("USD"), (-2000).into())]
            );
            assert_eq!(
                store.position_account(&PositionId::from("BINANCE:external:fees:USD"))?,
                None
            );
            assert_eq!(
                store.position_account(&PositionId::from("BINANCE:spot:USD"))?,
                Some(AccountId::from("legacy:binance:spot"))
            );
            assert_eq!(
                store.balance_observations("BINANCE", "binance-hash")?[0].amount,
                i64::MAX
            );
            assert_eq!(store.portfolios()?[0].label, "My portfolio");
            assert_eq!(store.portfolios()?[0].reporting_asset, AssetId::str("EUR"));
            assert_eq!(
                store
                    .accounts(&PortfolioId::from(crate::DEFAULT_PORTFOLIO))?
                    .iter()
                    .find(|a| a.id.0 == "legacy:nexo")
                    .unwrap()
                    .label,
                "My Nexo"
            );
            assert!(
                store
                    .missing_price_ranges(
                        &AssetId::str("USD"),
                        &AssetId::str("USD"),
                        PriceRange::new(time(1), time(2))?
                    )?
                    .is_empty()
            );
            let ordering: Vec<(String, i32)> = store.connection.prepare("SELECT transaction_id, effect_order FROM transaction_effects ORDER BY transaction_id, effect_order")?.query_map([], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<Result<_,_>>()?;
            assert_eq!(
                ordering,
                vec![
                    ("BINANCE:1".into(), 0),
                    ("BINANCE:1".into(), 1),
                    ("NEXO:1".into(), 0),
                    ("NEXO:1".into(), 1)
                ]
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires private cached Nexo/Binance DBs; copies them to temporary storage, no network"]
    fn cached_provider_databases_upgrade_without_changing_financial_records() -> anyhow::Result<()>
    {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let snapshot = |connection: &Connection| -> anyhow::Result<Vec<Option<Vec<String>>>> {
            let queries = [
                "SELECT position_id, asset_id, owned, role FROM positions ORDER BY position_id",
                "SELECT transaction_id, position_id, CAST(amount AS VARCHAR) AS amount, effect_order FROM transaction_effects ORDER BY transaction_id, effect_order",
                "SELECT * FROM transactions ORDER BY id",
                "SELECT * FROM asset_scales ORDER BY asset_id",
                "SELECT * FROM imported_files ORDER BY source, content_hash",
                "SELECT * FROM balance_observations ORDER BY source, content_hash, position_id, datetime, amount",
                "SELECT * FROM asset_prices ORDER BY datetime, asset_id, vs_asset_id",
                "SELECT * FROM price_coverage ORDER BY asset_id, vs_asset_id, from_datetime, to_datetime",
                "SELECT * FROM unavailable_price_periods ORDER BY provider, pair, from_datetime, to_datetime",
            ];
            queries.iter().map(|query| {
                let table = query.split(" FROM ").nth(1).unwrap().split_whitespace().next().unwrap();
                let exists: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name=?)", [table], |r| r.get(0))?;
                if !exists { return Ok(None); }
                let mut query = query.to_string();
                if table == "positions" {
                    let has_role: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('positions') WHERE name='role')", [], |r| r.get(0))?;
                    if !has_role { query = query.replace("owned, role", "owned, CASE WHEN owned THEN 'asset' WHEN starts_with(position_id, 'NEXO-CREDIT:') THEN 'liability' ELSE 'external' END AS role"); }
                }
                let mut statement = connection.prepare(&query)?;
                Ok(Some(statement.query_map([], |r| {
                    let columns = r.as_ref().column_count();
                    let values = (0..columns).map(|column| r.get::<_, duckdb::types::Value>(column)).collect::<Result<Vec<_>, _>>()?;
                    Ok(format!("{values:?}"))
                })?.collect::<Result<_, _>>()?))
            }).collect()
        };
        for relative in [
            ".cache/test-data/test-nexo_05-10-2026.duckdb",
            ".cache/test-data/test-binance-api-v1.duckdb",
            ".cache/test-data/binance/test-binance-statement-v1.duckdb",
        ] {
            let source = root.join(relative);
            anyhow::ensure!(
                !source.with_extension("duckdb.wal").exists(),
                "fixture has an active WAL; close its writer before copying"
            );
            let dir = tempfile::tempdir()?;
            let copy = dir.path().join("copy.duckdb");
            std::fs::copy(source, &copy)?;
            let before = snapshot(&Connection::open(&copy)?)?;
            for _ in 0..3 {
                let store = Store::open(&copy)?;
                let after = snapshot(&store.connection)?;
                for (index, original) in before.iter().enumerate() {
                    if let Some(original) = original {
                        let upgraded = after[index].as_ref().unwrap();
                        if index == 7 {
                            // Earlier coverage backfill may add conservative intervals.
                            assert!(original.iter().all(|row| upgraded.contains(row)));
                        } else {
                            assert_eq!(
                                upgraded, original,
                                "financial records changed for {relative}, table {index}"
                            );
                        }
                    }
                }
                store.latest_balances()?;
                store.asset_units_at(time(31))?;
            }
        }
        Ok(())
    }
    #[test]
    fn scoped_debt_and_external_counterparties_preserve_economic_roles() -> anyhow::Result<()> {
        let store = Store::in_memory()?;
        let account = Account {
            id: "provider:account".into(),
            portfolio_id: crate::DEFAULT_PORTFOLIO.into(),
            provider_id: crate::ProviderId::from("nexo"),
            label: "Loan account".into(),
            source_reference: None,
        };
        store.save_account(&account)?;
        let usd = AssetId::str("USD");
        let cash = PositionId::from("cash");
        let debt = PositionId::from("debt");
        let fees = PositionId::from("external:fees");
        let positions = [
            (cash.clone(), usd.clone(), 2, PositionRole::Asset),
            (debt.clone(), usd.clone(), 2, PositionRole::Liability),
            (
                fees.clone(),
                usd.clone(),
                2,
                PositionRole::ExternalCounterparty,
            ),
        ];
        let transactions = [
            ("cash".into(), transaction(time(1), &cash, 0, 10000)),
            ("debt".into(), transaction(time(1), &debt, 2000, 0)),
            ("external".into(), transaction(time(1), &fees, 999999, 0)),
        ];
        store.save_account_import(
            &account.id,
            "source",
            "hash",
            &positions,
            &transactions,
            &[],
        )?;
        let balances = store.account_balances_at(&account.id, time(1))?;
        assert_eq!(balances.len(), 2);
        assert_eq!(
            store.portfolio_units_at(&account.portfolio_id, time(1), PositionRole::Liability)?,
            vec![(usd.clone(), (-2000).into())]
        );
        assert_eq!(
            store.position_account(&Store::account_position_id(&account.id, &fees))?,
            None
        );
        assert_eq!(
            store.portfolio_value_history(&account.portfolio_id, &usd)?,
            vec![(time(1), 80.0)]
        );
        Ok(())
    }
}
