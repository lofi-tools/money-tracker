//! Minimum UI metadata. Source evidence and local state have separate lifecycles.
use crate::{
    AccountId, AssetId, ExactDecimal, PortfolioId, PositionId, ProductId, ProviderId, Store, Units,
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use duckdb::{Connection, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Source,
    Group,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountStructure {
    pub account_id: AccountId,
    pub kind: NodeKind,
    pub source_parent: Option<AccountId>,
    pub mapping_version: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCustomization {
    pub local_label: Option<String>,
    /// Distinguish source default from explicitly moving to the root.
    pub parent_is_custom: bool,
    pub parent: Option<AccountId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDisplay {
    pub id: ProviderId,
    pub name: Option<String>,
    pub evidence: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDisplay {
    pub id: AssetId,
    pub name: Option<String>,
    pub ticker: Option<String>,
    pub chain: Option<String>,
    pub contract: Option<String>,
    pub unresolved_label: Option<String>,
    pub evidence: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetAlias {
    pub provider: ProviderId,
    pub namespace: String,
    pub external_id: String,
    pub asset_id: AssetId,
    pub evidence: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductTemplate {
    pub id: ProductId,
    pub provider_id: ProviderId,
    pub source_reference: String,
    pub label: String,
    pub evidence: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    Unknown,
    Flexible,
    Locked,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub account_id: AccountId,
    pub product_id: ProductId,
    pub base_asset: AssetId,
    pub source_reference: String,
    pub label: String,
    pub opened_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub lock_status: LockStatus,
    pub maturity: Option<DateTime<Utc>>,
    pub auto_renew: Option<bool>,
    /// One subscription per ledger bucket. No duplicate principal allocation.
    pub positions: Vec<PositionId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum RateScope {
    Subscription(String),
    Product(ProductId),
    AccountAsset {
        account: AccountId,
        asset: AssetId,
    },
    Provider(ProviderId),
    Asset(AssetId),
    ProviderAsset {
        provider: ProviderId,
        asset: AssetId,
    },
    BorrowingPosition(PositionId),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnualRateBasis {
    Apr,
    Apy,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnualRate {
    pub fraction: ExactDecimal,
    pub basis: AnnualRateBasis,
    pub compounds_per_year: Option<u32>,
    pub reward_asset: Option<AssetId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEvidence {
    pub source: String,
    pub reference: String,
    pub observed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateEvidence {
    pub id: String,
    pub portfolio_id: PortfolioId,
    pub scope: RateScope,
    pub effective_at: DateTime<Utc>,
    pub rate: Option<AnnualRate>,
    pub evidence: SourceEvidence,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualRate {
    pub effective_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub rate: Option<AnnualRate>,
    pub pinned: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceEvidence {
    pub id: String,
    pub asset: AssetId,
    pub quote: AssetId,
    pub as_of: DateTime<Utc>,
    pub price: ExactDecimal,
    pub evidence: SourceEvidence,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualPrice {
    pub effective_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub price: ExactDecimal,
    pub pinned: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LocalTarget {
    Account {
        id: AccountId,
    },
    Subscription {
        id: String,
    },
    Rate {
        scope: RateScope,
    },
    Price {
        asset: AssetId,
        quote: AssetId,
    },
    Spam {
        asset: AssetId,
    },
    Dust {
        provider: ProviderId,
        asset: AssetId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DustMode {
    Auto,
    Dust,
    Significant,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum LocalValue {
    Account(AccountCustomization),
    SubscriptionName(String),
    Rates(Vec<ManualRate>),
    Prices(Vec<ManualPrice>),
    Spam { excluded: bool, reason: String },
    Dust(DustMode),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalRecord {
    pub revision: i64,
    pub value: Option<LocalValue>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualAction {
    pub id: String,
    pub sequence: i64,
    pub version: u32,
    pub portfolio_id: PortfolioId,
    pub target: LocalTarget,
    pub before: LocalRecord,
    pub after: LocalRecord,
    pub at: DateTime<Utc>,
    pub undo_of: Option<String>,
    pub evidence_fingerprint: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioRevisions {
    pub ledger: i64,
    pub policy: i64,
    pub metadata: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceEvidence {
    pub id: String,
    pub account: AccountId,
    pub position: Option<PositionId>,
    pub asset: AssetId,
    pub units: Units,
    pub scale: u8,
    pub basis: BalanceBasis,
    pub cutoff: DateTime<Utc>,
    pub complete: bool,
    pub independent: bool,
    pub evidence: SourceEvidence,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceBasis {
    Total,
    Available,
    Locked,
    Debt,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationOutcome {
    Unknown,
    Incomplete,
    Mismatch,
    Verified,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationIssue {
    pub code: String,
    pub position: Option<PositionId>,
    pub message: String,
    pub expected: Option<Units>,
    pub actual: Option<Units>,
    pub withheld: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub id: String,
    pub portfolio_id: PortfolioId,
    pub account: AccountId,
    pub cutoff: DateTime<Utc>,
    pub observations: Vec<String>,
    pub revisions: PortfolioRevisions,
    pub outcome: VerificationOutcome,
    pub history_complete: bool,
    pub issues: Vec<VerificationIssue>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshState {
    pub next_due: Option<DateTime<Utc>>,
    pub last_attempt: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
}

fn json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}
fn decode<T: DeserializeOwned>(value: String) -> Result<T> {
    Ok(serde_json::from_str(&value)?)
}
fn optional_payload<T: DeserializeOwned>(
    c: &Connection,
    table: &str,
    id: &str,
) -> Result<Option<T>> {
    // Table names come only from this module's literals, never request data.
    let rows = c
        .prepare(&format!("SELECT payload FROM {table} WHERE id=?"))?
        .query_map([id], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.into_iter().next().map(decode).transpose()
}
fn account_owner(c: &Connection, id: &AccountId) -> Result<(PortfolioId, ProviderId)> {
    let row: (String, String) = c
        .query_row(
            "SELECT portfolio_id,provider_id FROM accounts WHERE id=?",
            [&id.0],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .context("unknown metadata account")?;
    Ok((row.0.into(), ProviderId(row.1)))
}
fn portfolio_exists(c: &Connection, id: &PortfolioId) -> Result<()> {
    ensure!(
        c.query_row(
            "SELECT EXISTS(SELECT 1 FROM portfolios WHERE id=?)",
            [&id.0],
            |r| r.get::<_, bool>(0)
        )?,
        "unknown portfolio"
    );
    Ok(())
}
fn asset_exists(c: &Connection, id: &AssetId) -> Result<()> {
    ensure!(c.query_row("SELECT EXISTS(SELECT 1 FROM asset_scales WHERE asset_id=?) OR EXISTS(SELECT 1 FROM ui_asset_display WHERE id=?)", [&id.0,&id.0], |r| r.get::<_,bool>(0))?, "unknown metadata asset");
    Ok(())
}
fn immutable<T: Serialize + DeserializeOwned + PartialEq>(
    c: &Connection,
    table: &str,
    id: &str,
    payload: &T,
) -> Result<()> {
    ensure!(!id.is_empty(), "empty evidence ID");
    if let Some(previous) = optional_payload::<T>(c, table, id)? {
        ensure!(previous == *payload, "immutable evidence identity conflict");
        return Ok(());
    }
    c.execute(
        &format!("INSERT INTO {table} (id,payload) VALUES (?,?)"),
        params![id, json(payload)?],
    )?;
    Ok(())
}

fn local_record(
    c: &Connection,
    portfolio: &PortfolioId,
    target: &LocalTarget,
) -> Result<LocalRecord> {
    let rows = c
        .prepare("SELECT revision,payload FROM ui_local_state WHERE portfolio_id=? AND target=?")?
        .query_map(params![portfolio.0, json(target)?], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    match rows.into_iter().next() {
        Some((revision, value)) => Ok(LocalRecord {
            revision,
            value: value.map(decode).transpose()?,
        }),
        None => Ok(LocalRecord {
            revision: 0,
            value: None,
        }),
    }
}
fn structure(c: &Connection, id: &AccountId) -> Result<AccountStructure> {
    account_owner(c, id)?;
    Ok(
        optional_payload(c, "ui_account_structure", &id.0)?.unwrap_or(AccountStructure {
            account_id: id.clone(),
            kind: NodeKind::Source,
            source_parent: None,
            mapping_version: 0,
        }),
    )
}
fn effective_parent(c: &Connection, id: &AccountId) -> Result<Option<AccountId>> {
    let (portfolio, _) = account_owner(c, id)?;
    if let Some(LocalValue::Account(local)) =
        local_record(c, &portfolio, &LocalTarget::Account { id: id.clone() })?.value
    {
        if local.parent_is_custom {
            return Ok(local.parent);
        }
    }
    Ok(structure(c, id)?.source_parent)
}
fn validate_parent(c: &Connection, id: &AccountId, parent: Option<&AccountId>) -> Result<()> {
    let owner = account_owner(c, id)?;
    let mut next = parent.cloned();
    let mut visited = BTreeSet::from([id.0.clone()]);
    while let Some(node) = next {
        ensure!(visited.insert(node.0.clone()), "account hierarchy cycle");
        ensure!(
            account_owner(c, &node)? == owner,
            "parent must belong to the same portfolio and provider"
        );
        next = effective_parent(c, &node)?;
    }
    Ok(())
}
fn validate_rate(rate: &Option<AnnualRate>) -> Result<()> {
    if let Some(rate) = rate {
        ensure!(
            !rate.fraction.is_negative(),
            "annual rate must be nonnegative; borrowing cost is deducted by its role"
        );
        ensure!(
            rate.compounds_per_year.is_none_or(|n| n > 0),
            "compounding frequency must be positive"
        );
    }
    Ok(())
}
fn validate_evidence(e: &SourceEvidence) -> Result<()> {
    ensure!(
        !e.source.trim().is_empty() && !e.reference.trim().is_empty(),
        "evidence source/reference required"
    );
    ensure!(
        e.updated_at >= e.observed_at,
        "evidence update precedes observation"
    );
    Ok(())
}
fn validate_scope(c: &Connection, portfolio: &PortfolioId, scope: &RateScope) -> Result<()> {
    match scope {
        RateScope::Product(id) => {
            let product: ProductTemplate =
                optional_payload(c, "ui_products", &id.0)?.context("unknown product")?;
            let held = payloads::<Subscription>(c, "ui_subscriptions")?
                .into_iter()
                .any(|s| {
                    s.product_id == *id
                        && account_owner(c, &s.account_id).is_ok_and(|(p, provider)| {
                            p == *portfolio && provider == product.provider_id
                        })
                });
            ensure!(held, "product has no held subscriptions in this portfolio");
        }
        RateScope::Subscription(id) => {
            let sub: Subscription =
                optional_payload(c, "ui_subscriptions", id)?.context("unknown subscription")?;
            ensure!(
                account_owner(c, &sub.account_id)?.0 == *portfolio,
                "cross-portfolio subscription"
            );
        }
        RateScope::AccountAsset { account, asset } => {
            ensure!(
                account_owner(c, account)?.0 == *portfolio,
                "cross-portfolio account"
            );
            asset_exists(c, asset)?;
        }
        RateScope::BorrowingPosition(id) => {
            let (account, role): (String, String) = c.query_row(
                "SELECT account_id,role FROM positions WHERE position_id=?",
                [&id.0],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            ensure!(
                role == "liability" && account_owner(c, &account.into())?.0 == *portfolio,
                "rate target is not this portfolio's liability"
            );
        }
        RateScope::Asset(asset) => asset_exists(c, asset)?,
        RateScope::Provider(provider) | RateScope::ProviderAsset { provider, .. } => {
            ensure!(
                c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM accounts WHERE portfolio_id=? AND provider_id=?)",
                    params![portfolio.0, provider.0],
                    |r| r.get::<_, bool>(0)
                )?,
                "provider is not in portfolio"
            );
            if let RateScope::ProviderAsset { asset, .. } = scope {
                asset_exists(c, asset)?;
            }
        }
    }
    Ok(())
}
fn validate_local(
    c: &Connection,
    portfolio: &PortfolioId,
    target: &LocalTarget,
    value: &Option<LocalValue>,
) -> Result<()> {
    portfolio_exists(c, portfolio)?;
    match target {
        LocalTarget::Account { id } => ensure!(
            account_owner(c, id)?.0 == *portfolio,
            "cross-portfolio account"
        ),
        LocalTarget::Subscription { id } => {
            validate_scope(c, portfolio, &RateScope::Subscription(id.clone()))?
        }
        LocalTarget::Rate { scope } => validate_scope(c, portfolio, scope)?,
        LocalTarget::Price { asset, quote } => {
            asset_exists(c, asset)?;
            ensure!(
                !quote.0.is_empty() && asset != quote,
                "invalid price denomination"
            );
        }
        LocalTarget::Spam { asset } => ensure!(!asset.0.is_empty(), "empty spam asset"),
        LocalTarget::Dust { provider, asset } => validate_scope(
            c,
            portfolio,
            &RateScope::ProviderAsset {
                provider: provider.clone(),
                asset: asset.clone(),
            },
        )?,
    }
    match (target, value) {
        (_, None) => {}
        (LocalTarget::Account { id }, Some(LocalValue::Account(local))) => {
            ensure!(
                local
                    .local_label
                    .as_ref()
                    .is_none_or(|v| !v.trim().is_empty()),
                "local name is empty"
            );
            ensure!(
                local.parent_is_custom || local.parent.is_none(),
                "noncustom parent must be absent"
            );
            let parent = if local.parent_is_custom {
                local.parent.clone()
            } else {
                structure(c, id)?.source_parent
            };
            validate_parent(c, id, parent.as_ref())?;
        }
        (LocalTarget::Subscription { .. }, Some(LocalValue::SubscriptionName(name))) => {
            ensure!(!name.trim().is_empty(), "local name is empty")
        }
        (LocalTarget::Rate { .. }, Some(LocalValue::Rates(periods))) => {
            let mut dates = BTreeSet::new();
            for p in periods {
                ensure!(
                    dates.insert(p.effective_at),
                    "duplicate manual rate effective date"
                );
                validate_rate(&p.rate)?;
            }
        }
        (LocalTarget::Price { .. }, Some(LocalValue::Prices(periods))) => {
            let mut dates = BTreeSet::new();
            for p in periods {
                ensure!(
                    dates.insert(p.effective_at) && !p.price.is_negative(),
                    "duplicate price date or negative price"
                );
            }
        }
        (LocalTarget::Spam { .. }, Some(LocalValue::Spam { reason, .. })) => {
            ensure!(!reason.trim().is_empty(), "spam review needs a reason")
        }
        (LocalTarget::Dust { .. }, Some(LocalValue::Dust(_))) => {}
        _ => anyhow::bail!("local value does not match target type"),
    }
    Ok(())
}

pub(crate) fn bump_revision(c: &Connection, portfolio: &PortfolioId, column: &str) -> Result<()> {
    // Column is an internal literal, never supplied through an application command.
    c.execute(
        "INSERT OR IGNORE INTO ui_portfolio_revisions VALUES (?,0,0,0)",
        [&portfolio.0],
    )?;
    c.execute(
        &format!("UPDATE ui_portfolio_revisions SET {column}={column}+1 WHERE portfolio_id=?"),
        [&portfolio.0],
    )?;
    Ok(())
}
impl Store {
    pub fn account_structure(&self, id: &AccountId) -> Result<AccountStructure> {
        structure(&self.connection, id)
    }
    pub fn account_parent(&self, id: &AccountId) -> Result<Option<AccountId>> {
        effective_parent(&self.connection, id)
    }
    pub fn validate_account_tree(&self, portfolio: &PortfolioId) -> Result<()> {
        for account in self.accounts(portfolio)? {
            validate_parent(
                &self.connection,
                &account.id,
                effective_parent(&self.connection, &account.id)?.as_ref(),
            )?;
        }
        Ok(())
    }
    /// Provider defaults never mutate local preferences. A source mapping is versioned.
    pub fn save_account_structure(&self, value: &AccountStructure) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        let (portfolio, _) = account_owner(&tx, &value.account_id)?;
        let old = structure(&tx, &value.account_id)?;
        ensure!(
            old.mapping_version <= value.mapping_version && old.kind == value.kind,
            "structure version/kind conflict"
        );
        if old.mapping_version == value.mapping_version {
            ensure!(old == *value, "structure revision conflict");
            return Ok(());
        }
        validate_parent(&tx, &value.account_id, value.source_parent.as_ref())?;
        tx.execute("INSERT INTO ui_account_structure VALUES (?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![value.account_id.0,json(value)?])?;
        validate_parent(
            &tx,
            &value.account_id,
            effective_parent(&tx, &value.account_id)?.as_ref(),
        )?;
        bump_revision(&tx, &portfolio, "metadata")?;
        tx.commit()?;
        Ok(())
    }
    pub fn save_provider_display(&self, value: &ProviderDisplay) -> Result<()> {
        ensure!(
            !value.id.0.is_empty() && !value.evidence.trim().is_empty(),
            "provider ID/evidence required"
        );
        self.connection.execute("INSERT INTO ui_provider_display VALUES (?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![value.id.0,json(value)?])?;
        Ok(())
    }
    pub fn provider_display(&self, id: &ProviderId) -> Result<Option<ProviderDisplay>> {
        optional_payload(&self.connection, "ui_provider_display", &id.0)
    }
    pub fn save_asset_display(&self, value: &AssetDisplay) -> Result<()> {
        ensure!(
            !value.id.0.is_empty() && !value.evidence.trim().is_empty(),
            "asset ID/evidence required"
        );
        ensure!(
            value.contract.is_none() || value.chain.is_some(),
            "contract requires chain identity"
        );
        if let Some(previous) = self.asset_display(&value.id)? {
            if previous.chain.is_some() {
                ensure!(
                    previous.chain == value.chain && previous.contract == value.contract,
                    "asset locator identity conflict"
                );
            }
        }
        self.connection.execute("INSERT INTO ui_asset_display VALUES (?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![value.id.0,json(value)?])?;
        Ok(())
    }
    pub fn asset_display(&self, id: &AssetId) -> Result<Option<AssetDisplay>> {
        optional_payload(&self.connection, "ui_asset_display", &id.0)
    }
    pub fn save_asset_alias(&self, value: &AssetAlias) -> Result<()> {
        ensure!(
            !value.provider.0.is_empty()
                && !value.namespace.is_empty()
                && !value.external_id.is_empty()
                && !value.evidence.is_empty(),
            "alias namespace/reference/evidence required"
        );
        asset_exists(&self.connection, &value.asset_id)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO ui_asset_aliases VALUES (?,?,?,?,?)",
            params![
                value.provider.0,
                value.namespace,
                value.external_id,
                value.asset_id.0,
                json(value)?
            ],
        )?;
        Ok(())
    }
    /// Multiple results mean ambiguous; no automatic ticker-based ledger remapping.
    pub fn asset_aliases(
        &self,
        provider: &ProviderId,
        namespace: &str,
        external: &str,
    ) -> Result<Vec<AssetAlias>> {
        self.connection.prepare("SELECT payload FROM ui_asset_aliases WHERE provider=? AND namespace=? AND external_id=? ORDER BY asset_id")?
            .query_map(params![provider.0,namespace,external],|r| r.get::<_,String>(0))?.map(|r| decode(r?)).collect()
    }
    pub fn save_product_template(&self, value: &ProductTemplate) -> Result<()> {
        ensure!(
            !value.id.0.is_empty()
                && !value.provider_id.0.is_empty()
                && !value.source_reference.is_empty()
                && !value.evidence.is_empty(),
            "product identity/evidence required"
        );
        if let Some(old) = self.product_template(&value.id)? {
            ensure!(
                old.provider_id == value.provider_id
                    && old.source_reference == value.source_reference,
                "product identity conflict"
            );
        }
        self.connection.execute("INSERT INTO ui_products VALUES (?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![value.id.0,json(value)?])?;
        Ok(())
    }
    pub fn product_template(&self, id: &ProductId) -> Result<Option<ProductTemplate>> {
        optional_payload(&self.connection, "ui_products", &id.0)
    }
    pub fn subscription(&self, id: &str) -> Result<Option<Subscription>> {
        optional_payload(&self.connection, "ui_subscriptions", id)
    }
    pub fn subscriptions(&self, portfolio: &PortfolioId) -> Result<Vec<Subscription>> {
        portfolio_exists(&self.connection, portfolio)?;
        self.connection
            .prepare("SELECT payload FROM ui_subscriptions WHERE portfolio_id=? ORDER BY id")?
            .query_map([&portfolio.0], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn save_subscription(&self, value: &Subscription) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        let (portfolio, provider) = account_owner(&tx, &value.account_id)?;
        ensure!(
            !value.id.is_empty() && !value.source_reference.is_empty(),
            "subscription identity/reference required"
        );
        let product: ProductTemplate = optional_payload(&tx, "ui_products", &value.product_id.0)?
            .context("unknown subscription product")?;
        ensure!(
            product.provider_id == provider,
            "subscription provider conflict"
        );
        asset_exists(&tx, &value.base_asset)?;
        ensure!(
            value
                .opened_at
                .zip(value.closed_at)
                .is_none_or(|(a, b)| a <= b),
            "subscription closes before opening"
        );
        if let Some(old) = optional_payload::<Subscription>(&tx, "ui_subscriptions", &value.id)? {
            ensure!(
                old.account_id == value.account_id
                    && old.product_id == value.product_id
                    && old.base_asset == value.base_asset
                    && old.source_reference == value.source_reference,
                "subscription identity conflict"
            );
        }
        let mut seen = BTreeSet::new();
        for position in &value.positions {
            ensure!(seen.insert(&position.0), "duplicate subscription position");
            let (account, role): (Option<String>, String) = tx.query_row(
                "SELECT account_id,role FROM positions WHERE position_id=?",
                [&position.0],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            ensure!(
                account.as_ref() == Some(&value.account_id.0) && role != "external",
                "subscription position ownership conflict"
            );
            let other: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM ui_subscription_positions WHERE position_id=? AND subscription_id<>?)",params![position.0,value.id],|r| r.get(0))?;
            ensure!(
                !other,
                "position already attributed to another subscription"
            );
        }
        tx.execute(
            "DELETE FROM ui_subscription_positions WHERE subscription_id=?",
            [&value.id],
        )?;
        for position in &value.positions {
            tx.execute(
                "INSERT INTO ui_subscription_positions VALUES (?,?)",
                params![position.0, value.id],
            )?;
        }
        tx.execute("INSERT INTO ui_subscriptions VALUES (?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![value.id,portfolio.0,json(value)?])?;
        bump_revision(&tx, &portfolio, "metadata")?;
        tx.commit()?;
        Ok(())
    }
}

fn payloads<T: DeserializeOwned>(c: &Connection, table: &str) -> Result<Vec<T>> {
    c.prepare(&format!("SELECT payload FROM {table} ORDER BY id"))?
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|r| decode(r?))
        .collect()
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedRate {
    pub rate: Option<AnnualRate>,
    pub effective_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pinned: bool,
    pub evidence_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedPrice {
    pub price: ExactDecimal,
    pub as_of: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pinned: bool,
    pub evidence_id: Option<String>,
}
impl Store {
    pub fn save_rate_evidence(&self, value: &RateEvidence) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        portfolio_exists(&tx, &value.portfolio_id)?;
        validate_scope(&tx, &value.portfolio_id, &value.scope)?;
        validate_rate(&value.rate)?;
        validate_evidence(&value.evidence)?;
        let existed =
            optional_payload::<RateEvidence>(&tx, "ui_rate_evidence", &value.id)?.is_some();
        immutable(&tx, "ui_rate_evidence", &value.id, value)?;
        if !existed {
            bump_revision(&tx, &value.portfolio_id, "metadata")?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn rate_evidence(
        &self,
        portfolio: &PortfolioId,
        scope: &RateScope,
    ) -> Result<Vec<RateEvidence>> {
        portfolio_exists(&self.connection, portfolio)?;
        validate_scope(&self.connection, portfolio, scope)?;
        Ok(
            payloads::<RateEvidence>(&self.connection, "ui_rate_evidence")?
                .into_iter()
                .filter(|r| r.portfolio_id == *portfolio && r.scope == *scope)
                .collect(),
        )
    }
    /// Select effective periods before update times, so a backdated correction
    /// cannot erase a later rate period. Pins protect the applicable manual period.
    pub fn selected_rate(
        &self,
        portfolio: &PortfolioId,
        scope: &RateScope,
        cutoff: DateTime<Utc>,
    ) -> Result<Option<SelectedRate>> {
        let mut sourced: Vec<_> = self
            .rate_evidence(portfolio, scope)?
            .into_iter()
            .filter(|r| r.effective_at <= cutoff)
            .map(|r| SelectedRate {
                rate: r.rate,
                effective_at: r.effective_at,
                updated_at: r.evidence.updated_at,
                pinned: false,
                evidence_id: Some(r.id),
            })
            .collect();
        let state = self.local_state(
            portfolio,
            &LocalTarget::Rate {
                scope: scope.clone(),
            },
        )?;
        let manual = match state.value {
            Some(LocalValue::Rates(periods)) => periods
                .into_iter()
                .filter(|p| p.effective_at <= cutoff)
                .max_by_key(|p| (p.effective_at, p.updated_at)),
            _ => None,
        };
        if let Some(p) = manual {
            let manual = SelectedRate {
                rate: p.rate,
                effective_at: p.effective_at,
                updated_at: p.updated_at,
                pinned: p.pinned,
                evidence_id: None,
            };
            if manual.pinned {
                return Ok(Some(manual));
            }
            sourced.push(manual);
        }
        // Exact ties prefer manual; provider evidence IDs break source ties stably.
        Ok(sourced.into_iter().max_by(|a, b| {
            (
                a.effective_at,
                a.updated_at,
                a.evidence_id.is_none(),
                &a.evidence_id,
            )
                .cmp(&(
                    b.effective_at,
                    b.updated_at,
                    b.evidence_id.is_none(),
                    &b.evidence_id,
                ))
        }))
    }
    pub fn save_price_evidence(&self, value: &PriceEvidence) -> Result<()> {
        asset_exists(&self.connection, &value.asset)?;
        ensure!(
            !value.quote.0.is_empty() && value.asset != value.quote && !value.price.is_negative(),
            "invalid exact price"
        );
        validate_evidence(&value.evidence)?;
        immutable(&self.connection, "ui_price_evidence", &value.id, value)
    }
    pub fn price_evidence(&self, asset: &AssetId, quote: &AssetId) -> Result<Vec<PriceEvidence>> {
        Ok(
            payloads::<PriceEvidence>(&self.connection, "ui_price_evidence")?
                .into_iter()
                .filter(|p| p.asset == *asset && p.quote == *quote)
                .collect(),
        )
    }
    pub fn selected_price(
        &self,
        portfolio: &PortfolioId,
        asset: &AssetId,
        quote: &AssetId,
        cutoff: DateTime<Utc>,
    ) -> Result<Option<SelectedPrice>> {
        let mut candidates: Vec<_> = self
            .price_evidence(asset, quote)?
            .into_iter()
            .filter(|p| p.as_of <= cutoff)
            .map(|p| SelectedPrice {
                price: p.price,
                as_of: p.as_of,
                updated_at: p.evidence.updated_at,
                pinned: false,
                evidence_id: Some(p.id),
            })
            .collect();
        let state = self.local_state(
            portfolio,
            &LocalTarget::Price {
                asset: asset.clone(),
                quote: quote.clone(),
            },
        )?;
        if let Some(LocalValue::Prices(periods)) = state.value {
            if let Some(p) = periods
                .into_iter()
                .filter(|p| p.effective_at <= cutoff)
                .max_by_key(|p| (p.effective_at, p.updated_at))
            {
                let manual = SelectedPrice {
                    price: p.price,
                    as_of: p.effective_at,
                    updated_at: p.updated_at,
                    pinned: p.pinned,
                    evidence_id: None,
                };
                if manual.pinned {
                    return Ok(Some(manual));
                }
                candidates.push(manual);
            }
        }
        Ok(candidates.into_iter().max_by(|a, b| {
            (
                a.as_of,
                a.updated_at,
                a.evidence_id.is_none(),
                &a.evidence_id,
            )
                .cmp(&(
                    b.as_of,
                    b.updated_at,
                    b.evidence_id.is_none(),
                    &b.evidence_id,
                ))
        }))
    }
    pub fn revisions(&self, portfolio: &PortfolioId) -> Result<PortfolioRevisions> {
        portfolio_exists(&self.connection, portfolio)?;
        let rows = self
            .connection
            .prepare(
                "SELECT ledger,policy,metadata FROM ui_portfolio_revisions WHERE portfolio_id=?",
            )?
            .query_map([&portfolio.0], |r| {
                Ok(PortfolioRevisions {
                    ledger: r.get(0)?,
                    policy: r.get(1)?,
                    metadata: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().next().unwrap_or_default())
    }
    pub fn save_balance_evidence(&self, value: &BalanceEvidence) -> Result<()> {
        account_owner(&self.connection, &value.account)?;
        validate_evidence(&value.evidence)?;
        let scale: i32 = self.connection.query_row(
            "SELECT decimals FROM asset_scales WHERE asset_id=?",
            [&value.asset.0],
            |r| r.get(0),
        )?;
        ensure!(
            scale == i32::from(value.scale),
            "balance evidence scale conflict"
        );
        if let Some(position) = &value.position {
            let (account, asset): (Option<String>, String) = self.connection.query_row(
                "SELECT account_id,asset_id FROM positions WHERE position_id=?",
                [&position.0],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            ensure!(
                account.as_ref() == Some(&value.account.0) && asset == value.asset.0,
                "balance evidence position scope conflict"
            );
        }
        immutable(&self.connection, "ui_balance_evidence", &value.id, value)
    }
    pub fn balance_evidence(&self, account: &AccountId) -> Result<Vec<BalanceEvidence>> {
        account_owner(&self.connection, account)?;
        Ok(
            payloads::<BalanceEvidence>(&self.connection, "ui_balance_evidence")?
                .into_iter()
                .filter(|o| o.account == *account)
                .collect(),
        )
    }
    pub fn save_verification(&self, value: &VerificationRecord) -> Result<()> {
        ensure!(
            account_owner(&self.connection, &value.account)?.0 == value.portfolio_id,
            "verification portfolio conflict"
        );
        let current = self.revisions(&value.portfolio_id)?;
        ensure!(
            current.ledger == value.revisions.ledger && current.policy == value.revisions.policy,
            "verification revisions are stale"
        );
        for issue in &value.issues {
            ensure!(
                !issue.code.is_empty() && !issue.message.is_empty(),
                "issue code/message required"
            );
            ensure!(
                (issue.expected.is_none() && issue.actual.is_none()) || issue.position.is_some(),
                "quantity issue requires a position for exact asset/scale attribution"
            );
            if let Some(position) = &issue.position {
                ensure!(
                    self.position_account(position)?.as_ref() == Some(&value.account),
                    "issue position scope conflict"
                );
            }
        }
        ensure!(
            value.outcome != VerificationOutcome::Verified
                || (value.issues.is_empty() && !value.observations.is_empty()),
            "verified report needs observations and no issues"
        );
        for id in &value.observations {
            let observation: BalanceEvidence =
                optional_payload(&self.connection, "ui_balance_evidence", id)?
                    .context("unknown verification observation")?;
            ensure!(
                observation.account == value.account && observation.cutoff == value.cutoff,
                "observation account/cutoff conflict"
            );
            if value.outcome == VerificationOutcome::Verified {
                ensure!(
                    observation.complete && observation.independent,
                    "verified report needs complete independent evidence"
                );
            }
        }
        if value.outcome == VerificationOutcome::Verified {
            validate_verified_balances(self, value)?;
        }
        immutable(&self.connection, "ui_verifications", &value.id, value)
    }
    pub fn verifications(&self, portfolio: &PortfolioId) -> Result<Vec<VerificationRecord>> {
        portfolio_exists(&self.connection, portfolio)?;
        Ok(
            payloads::<VerificationRecord>(&self.connection, "ui_verifications")?
                .into_iter()
                .filter(|r| r.portfolio_id == *portfolio)
                .collect(),
        )
    }
    pub fn verification_is_current(&self, value: &VerificationRecord) -> Result<bool> {
        let revisions = self.revisions(&value.portfolio_id)?;
        Ok(
            revisions.ledger == value.revisions.ledger
                && revisions.policy == value.revisions.policy,
        )
    }
    pub fn refresh_state(&self, portfolio: &PortfolioId, source: &str) -> Result<RefreshState> {
        portfolio_exists(&self.connection, portfolio)?;
        let rows = self
            .connection
            .prepare("SELECT payload FROM ui_refresh_state WHERE portfolio_id=? AND source=?")?
            .query_map(params![portfolio.0, source], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .next()
            .map(decode)
            .transpose()
            .map(|v| v.unwrap_or_default())
    }
    pub fn save_refresh_state(
        &self,
        portfolio: &PortfolioId,
        source: &str,
        value: &RefreshState,
    ) -> Result<()> {
        portfolio_exists(&self.connection, portfolio)?;
        ensure!(!source.is_empty(), "refresh source required");
        ensure!(
            value
                .last_success
                .is_none_or(|s| value.last_attempt.is_some_and(|a| s <= a)),
            "refresh success exceeds last attempt"
        );
        self.connection.execute("INSERT INTO ui_refresh_state VALUES (?,?,?) ON CONFLICT(portfolio_id,source) DO UPDATE SET payload=excluded.payload",params![portfolio.0,source,json(value)?])?;
        Ok(())
    }
    pub fn local_state(
        &self,
        portfolio: &PortfolioId,
        target: &LocalTarget,
    ) -> Result<LocalRecord> {
        validate_local(&self.connection, portfolio, target, &None)?;
        local_record(&self.connection, portfolio, target)
    }
    pub fn manual_actions(&self, portfolio: &PortfolioId) -> Result<Vec<ManualAction>> {
        portfolio_exists(&self.connection, portfolio)?;
        self.connection
            .prepare(
                "SELECT payload FROM ui_manual_actions WHERE portfolio_id=? ORDER BY sequence",
            )?
            .query_map([&portfolio.0], |r| r.get::<_, String>(0))?
            .map(|r| decode(r?))
            .collect()
    }
    pub fn apply_manual_change(
        &self,
        portfolio: &PortfolioId,
        target: &LocalTarget,
        expected_revision: i64,
        value: Option<LocalValue>,
    ) -> Result<ManualAction> {
        self.commit_manual_change(portfolio, target, expected_revision, value, None)
    }
    pub fn undo_manual_action(&self, portfolio: &PortfolioId, id: &str) -> Result<ManualAction> {
        let action: ManualAction = optional_payload(&self.connection, "ui_manual_actions", id)?
            .context("unknown manual action")?;
        ensure!(
            action.portfolio_id == *portfolio && action.undo_of.is_none(),
            "action is not an original action in this portfolio"
        );
        ensure!(
            !self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM ui_manual_actions WHERE undo_of=?)",
                [id],
                |r| r.get::<_, bool>(0)
            )?,
            "action has already been undone"
        );
        ensure!(
            local_record(&self.connection, portfolio, &action.target)? == action.after,
            "undo conflicts with newer local state"
        );
        ensure!(
            action.evidence_fingerprint
                == source_fingerprint(&self.connection, portfolio, &action.target)?,
            "undo requires review after new source evidence"
        );
        self.commit_manual_change(
            portfolio,
            &action.target,
            action.after.revision,
            action.before.value,
            Some(id.into()),
        )
    }
    pub fn override_requires_review(
        &self,
        portfolio: &PortfolioId,
        target: &LocalTarget,
    ) -> Result<bool> {
        if !matches!(target, LocalTarget::Rate { .. } | LocalTarget::Price { .. }) {
            return Ok(false);
        }
        let state = self.local_state(portfolio, target)?;
        if state.value.is_none() {
            return Ok(false);
        }
        let latest = self
            .manual_actions(portfolio)?
            .into_iter()
            .rev()
            .find(|a| a.target == *target);
        match latest {
            Some(action) => Ok(source_fingerprint(&self.connection, portfolio, target)?
                != action.evidence_fingerprint),
            None => Ok(false),
        }
    }
    fn commit_manual_change(
        &self,
        portfolio: &PortfolioId,
        target: &LocalTarget,
        expected: i64,
        value: Option<LocalValue>,
        undo_of: Option<String>,
    ) -> Result<ManualAction> {
        let tx = self.connection.unchecked_transaction()?;
        validate_local(&tx, portfolio, target, &value)?;
        if value.is_none() {
            if let LocalTarget::Account { id } = target {
                validate_parent(&tx, id, structure(&tx, id)?.source_parent.as_ref())?;
            }
        }
        let before = local_record(&tx, portfolio, target)?;
        if let Some(id) = &undo_of {
            let original: ManualAction =
                optional_payload(&tx, "ui_manual_actions", id)?.context("unknown undo action")?;
            ensure!(
                original.portfolio_id == *portfolio
                    && original.target == *target
                    && original.undo_of.is_none()
                    && original.after == before,
                "undo scope/revision conflict"
            );
            ensure!(
                original.evidence_fingerprint == source_fingerprint(&tx, portfolio, target)?,
                "undo requires review after new source evidence"
            );
        }
        ensure!(before.revision == expected, "local revision conflict");
        ensure!(before.value != value, "no local change");
        let after = LocalRecord {
            revision: before
                .revision
                .checked_add(1)
                .context("local revision overflow")?,
            value,
        };
        let sequence: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence),0)+1 FROM ui_manual_actions",
            [],
            |r| r.get(0),
        )?;
        let action = ManualAction {
            id: format!("manual:{sequence}"),
            sequence,
            version: 1,
            portfolio_id: portfolio.clone(),
            target: target.clone(),
            before,
            after: after.clone(),
            at: Utc::now(),
            undo_of,
            evidence_fingerprint: source_fingerprint(&tx, portfolio, target)?,
        };
        tx.execute("INSERT INTO ui_local_state VALUES (?,?,?,?) ON CONFLICT(portfolio_id,target) DO UPDATE SET revision=excluded.revision,payload=excluded.payload",params![portfolio.0,json(target)?,after.revision,after.value.as_ref().map(json).transpose()?])?;
        if let LocalTarget::Spam { asset } = target {
            let excluded = matches!(after.value, Some(LocalValue::Spam { excluded: true, .. }));
            tx.execute("INSERT INTO ui_asset_spam VALUES (?,?,?) ON CONFLICT(portfolio_id,asset_id) DO UPDATE SET excluded=excluded.excluded",params![portfolio.0,asset.0,excluded])?;
            bump_revision(&tx, portfolio, "policy")?;
        } else {
            bump_revision(&tx, portfolio, "metadata")?;
        }
        tx.execute(
            "INSERT INTO ui_manual_actions VALUES (?,?,?,?,?,?)",
            params![
                action.id,
                sequence,
                portfolio.0,
                json(target)?,
                action.undo_of,
                json(&action)?
            ],
        )?;
        tx.commit()?;
        Ok(action)
    }
}

fn source_fingerprint(
    c: &Connection,
    portfolio: &PortfolioId,
    target: &LocalTarget,
) -> Result<String> {
    match target {
        LocalTarget::Rate { scope } => {
            // Aggregate overrides need review even when only partial finer data
            // arrives. Retain pins; this flag never invalidates a stored override.
            let rates: Vec<RateEvidence> = payloads(c, "ui_rate_evidence")?;
            let subscriptions: Vec<Subscription> = payloads(c, "ui_subscriptions")?;
            let related = |candidate: &RateScope| -> Result<bool> {
                if candidate == scope {
                    return Ok(true);
                }
                if let (RateScope::Product(product), RateScope::Subscription(id)) =
                    (scope, candidate)
                {
                    if let Some(sub) = subscriptions.iter().find(|s| &s.id == id) {
                        return Ok(sub.product_id == *product
                            && account_owner(c, &sub.account_id)?.0 == *portfolio);
                    }
                }

                if let RateScope::ProviderAsset { provider, asset } = candidate {
                    return Ok(match scope {
                        RateScope::Provider(id) => id == provider,
                        RateScope::Asset(id) => id == asset,
                        _ => false,
                    });
                }
                if let RateScope::Product(id) = candidate {
                    for sub in subscriptions.iter().filter(|s| &s.product_id == id) {
                        let (owner, provider) = account_owner(c, &sub.account_id)?;
                        if owner == *portfolio
                            && match scope {
                                RateScope::Provider(p) => *p == provider,
                                RateScope::Asset(a) => *a == sub.base_asset,
                                RateScope::ProviderAsset {
                                    provider: p,
                                    asset: a,
                                } => *p == provider && *a == sub.base_asset,
                                _ => false,
                            }
                        {
                            return Ok(true);
                        }
                    }
                }
                let account_asset = match candidate {
                    RateScope::Subscription(id) => subscriptions
                        .iter()
                        .find(|s| &s.id == id)
                        .map(|s| (s.account_id.clone(), s.base_asset.clone())),
                    RateScope::AccountAsset { account, asset } => {
                        Some((account.clone(), asset.clone()))
                    }
                    _ => None,
                };
                if let Some((account, asset)) = account_asset {
                    let (p, provider) = account_owner(c, &account)?;
                    if p != *portfolio {
                        return Ok(false);
                    }
                    return Ok(match scope {
                        RateScope::Provider(id) => *id == provider,
                        RateScope::Asset(id) => *id == asset,
                        RateScope::ProviderAsset {
                            provider: id,
                            asset: a,
                        } => *id == provider && *a == asset,
                        RateScope::AccountAsset {
                            account: id,
                            asset: a,
                        } => *id == account && *a == asset,
                        _ => false,
                    });
                }
                Ok(false)
            };
            let mut identities = Vec::new();
            for rate in rates {
                if rate.portfolio_id == *portfolio && related(&rate.scope)? {
                    identities.push(format!("rate:{}", rate.id));
                }
            }
            for sub in &subscriptions {
                if related(&RateScope::Subscription(sub.id.clone()))? {
                    identities.push(format!("subscription:{}:{}", sub.id, json(sub)?));
                }
            }
            identities.sort();
            json(&identities)
        }
        LocalTarget::Price { asset, quote } => {
            let identities: Vec<_> = payloads::<PriceEvidence>(c, "ui_price_evidence")?
                .into_iter()
                .filter(|p| p.asset == *asset && p.quote == *quote)
                .map(|p| p.id)
                .collect();
            json(&identities)
        }
        _ => Ok(String::new()),
    }
}

impl Store {
    /// Register a provider-specific grouping node without fabricating a connection,
    /// subscription or financial movement. Manual UI creation commands follow later.
    pub fn save_group_account(
        &self,
        account: &crate::Account,
        parent: Option<AccountId>,
        mapping_version: u32,
    ) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        portfolio_exists(&tx, &account.portfolio_id)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?)",
            [&account.id.0],
            |r| r.get(0),
        )?;
        if exists {
            ensure!(
                account_owner(&tx, &account.id)?
                    == (account.portfolio_id.clone(), account.provider_id.clone())
                    && structure(&tx, &account.id)?.kind == NodeKind::Group,
                "group identity conflict"
            );
        } else {
            ensure!(
                !account.id.0.is_empty() && !account.provider_id.0.is_empty(),
                "empty group identity"
            );
            tx.execute(
                "INSERT INTO accounts (id,portfolio_id,provider_id,label,source_reference) VALUES (?,?,?,?,?)",
                params![
                    account.id.0,
                    account.portfolio_id.0,
                    account.provider_id.0,
                    account.label,
                    account.source_reference
                ],
            )?;
        }
        let value = AccountStructure {
            account_id: account.id.clone(),
            kind: NodeKind::Group,
            source_parent: parent,
            mapping_version,
        };
        if exists {
            let previous = structure(&tx, &account.id)?;
            ensure!(
                previous.mapping_version <= mapping_version,
                "group mapping version conflict"
            );
            if previous.mapping_version == mapping_version {
                ensure!(previous == value, "group mapping revision conflict");
                return Ok(());
            }
        }
        validate_parent(&tx, &account.id, value.source_parent.as_ref())?;
        tx.execute("INSERT INTO ui_account_structure VALUES (?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![account.id.0,json(&value)?])?;
        bump_revision(&tx, &account.portfolio_id, "metadata")?;
        crate::migrations::seed_ui_metadata(&tx)?;
        tx.commit()?;
        Ok(())
    }
}

fn validate_verified_balances(store: &Store, value: &VerificationRecord) -> Result<()> {
    let metadata: BTreeMap<_, _> = store
        .account_position_metadata(&value.account)?
        .into_iter()
        .map(|(id, asset, _, role)| (id.0, (asset, role)))
        .collect();
    let ledger: BTreeMap<_, _> = store
        .account_balances_at(&value.account, value.cutoff)?
        .into_iter()
        .map(|p| (p.position_id.0, p.amount))
        .collect();
    let mut covered = BTreeSet::new();
    for id in &value.observations {
        let observation: BalanceEvidence =
            optional_payload(&store.connection, "ui_balance_evidence", id)?
                .context("missing observation")?;
        let role = match observation.basis {
            BalanceBasis::Total => crate::PositionRole::Asset,
            BalanceBasis::Debt => crate::PositionRole::Liability,
            _ => anyhow::bail!("available/locked evidence cannot verify total ledger balances"),
        };
        if role == crate::PositionRole::Asset
            && store.asset_is_excluded(&observation.account, &observation.asset)?
        {
            continue;
        }
        let positions: Vec<_> = metadata
            .iter()
            .filter(|(id, (asset, r))| {
                *asset == observation.asset
                    && *r == role
                    && observation.position.as_ref().is_none_or(|p| &p.0 == *id)
            })
            .map(|(id, _)| id.clone())
            .collect();
        ensure!(
            !positions.is_empty() || observation.units.is_zero(),
            "unregistered nonzero observation"
        );
        let mut expected = Units::default();
        for position in positions {
            ensure!(
                covered.insert(position.clone()),
                "verification has overlapping observations"
            );
            expected += ledger.get(&position).cloned().unwrap_or_default();
        }
        ensure!(
            expected == observation.units,
            "verified report has an exact balance mismatch"
        );
    }
    ensure!(
        ledger
            .iter()
            .all(|(id, amount)| amount.is_zero() || covered.contains(id)),
        "verified report omits nonzero tracked positions"
    );
    Ok(())
}

impl Store {
    pub fn local_records(
        &self,
        portfolio: &PortfolioId,
    ) -> Result<Vec<(LocalTarget, LocalRecord)>> {
        portfolio_exists(&self.connection, portfolio)?;
        self.connection.prepare("SELECT target,revision,payload FROM ui_local_state WHERE portfolio_id=? ORDER BY target")?
            .query_map([&portfolio.0],|r| Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,Option<String>>(2)?)))?
            .map(|row| {let (target,revision,payload)=row?;Ok((decode(target)?,LocalRecord {revision,value:payload.map(decode).transpose()?}))}).collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_action_insert_rolls_back_local_state_and_revision_then_retries() -> Result<()> {
        let store = Store::in_memory()?;
        let portfolio = crate::DEFAULT_PORTFOLIO.into();
        for id in ["a", "b"] {
            store.save_account(&crate::Account {
                id: id.into(),
                portfolio_id: crate::DEFAULT_PORTFOLIO.into(),
                provider_id: "nexo".into(),
                label: id.into(),
                source_reference: None,
            })?;
        }
        store.apply_manual_change(
            &portfolio,
            &LocalTarget::Account { id: "a".into() },
            0,
            Some(LocalValue::Account(AccountCustomization {
                local_label: Some("First".into()),
                ..Default::default()
            })),
        )?;
        store.connection.execute_batch(
            "CREATE UNIQUE INDEX injected_action_failure ON ui_manual_actions(portfolio_id)",
        )?;
        let target = LocalTarget::Account { id: "b".into() };
        let before = store.revisions(&portfolio)?;
        assert!(
            store
                .apply_manual_change(
                    &portfolio,
                    &target,
                    0,
                    Some(LocalValue::Account(AccountCustomization {
                        local_label: Some("Second".into()),
                        ..Default::default()
                    }))
                )
                .is_err()
        );
        assert_eq!(
            store.local_state(&portfolio, &target)?,
            LocalRecord {
                revision: 0,
                value: None
            }
        );
        assert_eq!(store.revisions(&portfolio)?, before);
        assert_eq!(store.manual_actions(&portfolio)?.len(), 1);
        store
            .connection
            .execute_batch("DROP INDEX injected_action_failure")?;
        let retried = store.apply_manual_change(
            &portfolio,
            &target,
            0,
            Some(LocalValue::Account(AccountCustomization {
                local_label: Some("Second".into()),
                ..Default::default()
            })),
        )?;
        assert_eq!(retried.sequence, 2);
        Ok(())
    }
}
