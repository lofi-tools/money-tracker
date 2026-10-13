//! Refresh lifecycle shared by the CLI and desktop hosts: which sources are due,
//! what acquisition each source needs, and exactly what is persisted.
//!
//! Scheduling is a pure decision over stored [`RefreshState`] so hosts and tests
//! can inject a clock. Acquisition (network/file reads) never runs on the
//! database thread; hosts plan, acquire, then commit the typed outcomes.
use crate::adapters::{coingecko::CoinGeckoSvc, kraken as kraken_adapter, nexo::NexoSvc};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use lib_core::{
    Account, AccountId, AssetId, PortfolioId, Store,
    ui_metadata::{PriceEvidence, RefreshState},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PRICES_SOURCE: &str = "prices";

/// Why a run was requested. Launch and scheduled work coalesce into one run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshTrigger {
    Launch,
    Scheduled,
    Manual,
}

/// Why one source takes part in a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunReason {
    Manual,
    RefreshOnOpen,
    MissedSchedule,
    Scheduled,
}

/// Exact inputs one source needs; produced by planning, consumed by acquisition.
#[derive(Clone, Debug)]
pub enum Acquisition {
    Prices {
        assets: Vec<AssetId>,
        quote: AssetId,
    },
    /// One Kraken account: the cached snapshot path is account-specific.
    Kraken { account: Account, cache: PathBuf },
    /// The configured Nexo export. The app re-reads a file; it is never a live
    /// Nexo balance source.
    Nexo { path: PathBuf },
}

/// One configured refresh source with its stored state and prerequisite.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub source: String,
    pub label: String,
    pub state: RefreshState,
    /// Present when this app has an acquisition path for the source.
    pub acquisition: Option<Acquisition>,
    /// Present when the source cannot run here; a prerequisite, not a failure.
    pub prerequisite: Option<String>,
}

/// What one planned source will do. `Skip` means neither a run nor a schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanAction {
    Run(RunReason),
    /// Start the existing interval now without contacting anything: a source
    /// that was never scheduled begins one interval after it is first seen.
    StartSchedule,
    Skip,
}

#[derive(Clone, Debug)]
pub struct PlanItem {
    pub source: String,
    pub label: String,
    pub action: PlanAction,
    pub acquisition: Option<Acquisition>,
    /// Carried through so a run can report a scoped blocked reason.
    pub prerequisite: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RefreshPlan {
    pub portfolio: PortfolioId,
    pub at: DateTime<Utc>,
    pub trigger: RefreshTrigger,
    pub interval_seconds: u64,
    pub items: Vec<PlanItem>,
}

impl RefreshPlan {
    /// Sources this plan will actually contact.
    pub fn runs(&self) -> impl Iterator<Item = &PlanItem> {
        self.items
            .iter()
            .filter(|item| matches!(item.action, PlanAction::Run(_)))
    }
    /// True when the plan performs no work at all (nothing to run or schedule).
    pub fn is_idle(&self) -> bool {
        self.items
            .iter()
            .all(|item| matches!(item.action, PlanAction::Skip))
    }
}

/// Sources the app can refresh, derived from the open portfolio. Providers
/// without a documented acquisition path are not listed as refresh sources.
pub fn candidates(
    store: &Store,
    portfolio: &PortfolioId,
    reporting_asset: &AssetId,
    cache_dir: &Path,
    nexo_csv: Option<&Path>,
) -> Result<Vec<Candidate>> {
    let accounts = store.accounts(portfolio)?;
    let mut candidates = Vec::new();
    let mut assets = Vec::new();
    for account in &accounts {
        for (_, asset, _, _) in store.account_position_metadata(&account.id)? {
            if asset != *reporting_asset && !assets.contains(&asset) {
                assets.push(asset);
            }
        }
    }
    if !assets.is_empty() {
        candidates.push(Candidate {
            source: PRICES_SOURCE.into(),
            label: format!("Current prices ({})", reporting_asset.0),
            state: store.refresh_state(portfolio, PRICES_SOURCE)?,
            acquisition: Some(Acquisition::Prices {
                assets,
                quote: reporting_asset.clone(),
            }),
            prerequisite: None,
        });
    }
    let kraken_credentials = ["KRAKEN_API_KEY_ID", "KRAKEN_API_SECRET"]
        .iter()
        .all(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()));
    // The Nexo export is provider-wide: one file, one re-read. A configured
    // export is refreshable on its own, before any Nexo account exists.
    let usable = nexo_csv.filter(|path| path.is_file());
    if nexo_csv.is_some() || accounts.iter().any(|account| is_provider(account, "NEXO")) {
        let source = provider_source("NEXO");
        candidates.push(Candidate {
            label: "Nexo export".into(),
            state: store.refresh_state(portfolio, &source)?,
            acquisition: usable.map(|path| Acquisition::Nexo {
                path: path.to_path_buf(),
            }),
            prerequisite: usable.is_none().then(|| {
                "No readable Nexo export is configured; refreshing Nexo re-reads a CSV export and cannot fetch live balances (set the export path in the configuration file).".into()
            }),
            source,
        });
    }
    for account in &accounts {
        if is_provider(account, "kraken") {
            let source = format!("account:{}", account.id.0);
            candidates.push(Candidate {
                label: format!("Kraken · {}", account.label),
                source: source.clone(),
                state: store.refresh_state(portfolio, &source)?,
                acquisition: kraken_credentials.then(|| Acquisition::Kraken {
                    account: account.clone(),
                    cache: cache_dir.to_path_buf(),
                }),
                prerequisite: (!kraken_credentials).then(|| {
                    "Kraken API credentials are not available to this app; start it from an environment providing KRAKEN_API_KEY_ID and KRAKEN_API_SECRET (a desktop launch does not inherit a terminal's direnv environment).".into()
                }),
            });
        } else if is_provider(account, "binance") {
            candidates.push(Candidate {
                source: format!("account:{}", account.id.0),
                label: format!("Binance · {}", account.label),
                state: store.refresh_state(portfolio, &format!("account:{}", account.id.0))?,
                acquisition: None,
                prerequisite: Some(
                    "Binance refresh runs through the CLI statement/API import path, which this app does not run yet.".into(),
                ),
            });
        } else if is_provider(account, "evm") {
            candidates.push(Candidate {
                source: format!("account:{}", account.id.0),
                label: format!("EVM · {}", account.label),
                state: store.refresh_state(portfolio, &format!("account:{}", account.id.0))?,
                acquisition: None,
                prerequisite: Some(
                    "EVM refresh needs chain endpoints and a cache configured outside the app; run the CLI `evm` command.".into(),
                ),
            });
        }
    }
    Ok(candidates)
}

fn is_provider(account: &Account, provider: &str) -> bool {
    account.provider_id.0.eq_ignore_ascii_case(provider)
}

pub fn provider_source(provider: &str) -> String {
    format!("provider:{provider}")
}

/// The shared Kraken snapshot cache path for one local account.
pub fn kraken_cache_path(cache_dir: &Path, account: &AccountId) -> PathBuf {
    cache_dir.join("kraken").join(format!(
        "{}.json",
        kraken_client::fingerprint(account.0.as_bytes())
    ))
}

/// Decide what one trigger does with the stored schedule.
///
/// A source that was never scheduled starts its interval on the next launch or
/// scheduled pass instead of contacting anything, so opening the app does not
/// refresh unless the user enabled refresh-on-open. An intervening launch after
/// a missed interval performs exactly one catch-up run.
pub fn plan(
    portfolio: PortfolioId,
    trigger: RefreshTrigger,
    refresh_on_open: bool,
    interval_seconds: u64,
    candidates: Vec<Candidate>,
    now: DateTime<Utc>,
) -> RefreshPlan {
    let items = candidates
        .into_iter()
        .map(|candidate| {
            let Candidate {
                source,
                label,
                state,
                acquisition,
                prerequisite,
            } = candidate;
            // A prerequisite blocks both runs and schedules; the host reports it
            // as a scoped explanation instead of a failure.
            if let Some(reason) = prerequisite {
                return PlanItem {
                    source,
                    label,
                    action: PlanAction::Skip,
                    acquisition: None,
                    prerequisite: Some(reason),
                };
            }
            let due = state.next_due.is_some_and(|next| next <= now);
            let unscheduled = state.next_due.is_none();
            let action = match trigger {
                RefreshTrigger::Manual => PlanAction::Run(RunReason::Manual),
                RefreshTrigger::Launch if refresh_on_open => {
                    PlanAction::Run(RunReason::RefreshOnOpen)
                }
                RefreshTrigger::Launch if due => PlanAction::Run(RunReason::MissedSchedule),
                RefreshTrigger::Launch if unscheduled => PlanAction::StartSchedule,
                RefreshTrigger::Launch => PlanAction::Skip,
                RefreshTrigger::Scheduled if unscheduled => PlanAction::StartSchedule,
                RefreshTrigger::Scheduled if due => PlanAction::Run(RunReason::Scheduled),
                RefreshTrigger::Scheduled => PlanAction::Skip,
            };
            let acquisition = matches!(action, PlanAction::Run(_))
                .then_some(acquisition)
                .flatten();
            PlanItem {
                source,
                label,
                action,
                acquisition,
                prerequisite: None,
            }
        })
        .collect();
    RefreshPlan {
        portfolio,
        at: now,
        trigger,
        interval_seconds,
        items,
    }
}

/// A redacted failure reason. Provider text is never forwarded: it can contain
/// credentials or private amounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    AcquisitionFailed,
    ImportFailed,
    ReconciliationFailed,
}

impl FailureCode {
    pub fn id(&self) -> &'static str {
        match self {
            Self::AcquisitionFailed => "acquisition_failed",
            Self::ImportFailed => "import_failed",
            Self::ReconciliationFailed => "reconciliation_failed",
        }
    }
    pub fn message(&self) -> &'static str {
        match self {
            Self::AcquisitionFailed => {
                "the source could not be reached or refused the request from this app"
            }
            Self::ImportFailed => "the acquired data could not be imported into this portfolio",
            Self::ReconciliationFailed => {
                "independent balances did not reconcile with the imported ledger"
            }
        }
    }
    pub fn text(&self) -> String {
        format!("{}: {}", self.id(), self.message())
    }
}

/// Data acquired off the database thread, ready to commit.
#[derive(Clone, Debug)]
pub enum Acquired {
    Prices(Vec<PriceEvidence>),
    Kraken {
        account: Account,
        snapshot: Box<kraken_client::Snapshot>,
    },
    Nexo {
        path: PathBuf,
    },
}

pub struct SourceOutcome {
    pub source: String,
    pub result: std::result::Result<Acquired, FailureCode>,
}

/// Contact one planned source. Runs outside the database worker: no store access.
pub async fn acquire(item: &PlanItem) -> Option<SourceOutcome> {
    let acquisition = item.acquisition.clone()?;
    let source = item.source.clone();
    let result = match acquisition {
        Acquisition::Prices { assets, quote } => {
            match CoinGeckoSvc::new().map_err(|_| FailureCode::AcquisitionFailed) {
                Err(failure) => Err(failure),
                Ok(service) => match service.fetch_exact_current_prices(&assets).await {
                    Ok(prices) => Ok(Acquired::Prices(
                        prices.into_iter().filter(|p| p.quote == quote).collect(),
                    )),
                    Err(_) => Err(FailureCode::AcquisitionFailed),
                },
            }
        }
        Acquisition::Kraken { account, cache } => {
            let path = kraken_cache_path(&cache, &account.id);
            match kraken_client::cached_snapshot(&path, &account.id.0, false, true).await {
                Ok(snapshot) => Ok(Acquired::Kraken {
                    account,
                    snapshot: Box::new(snapshot),
                }),
                Err(_) => Err(FailureCode::AcquisitionFailed),
            }
        }
        Acquisition::Nexo { path } => Ok(Acquired::Nexo { path }),
    };
    Some(SourceOutcome { source, result })
}

/// The typed result of one source in a run, safe to show in the app.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RefreshOutcome {
    Updated { detail: String },
    Unchanged,
    Failed { code: String, message: String },
    Blocked { reason: String },
    Scheduled,
    NotDue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshEntry {
    pub source: String,
    pub label: String,
    pub outcome: RefreshOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshReport {
    pub at: DateTime<Utc>,
    pub trigger: RefreshTrigger,
    pub interval_seconds: u64,
    pub in_flight: bool,
    pub entries: Vec<RefreshEntry>,
}

impl RefreshReport {
    pub fn empty(at: DateTime<Utc>, trigger: RefreshTrigger, interval_seconds: u64) -> Self {
        Self {
            at,
            trigger,
            interval_seconds,
            in_flight: false,
            entries: Vec::new(),
        }
    }
    /// True when the run changed stored data or started a schedule.
    pub fn changed(&self) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                entry.outcome,
                RefreshOutcome::Updated { .. }
                    | RefreshOutcome::Unchanged
                    | RefreshOutcome::Scheduled
            )
        })
    }
    pub fn failures(&self) -> impl Iterator<Item = &RefreshEntry> {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.outcome, RefreshOutcome::Failed { .. }))
    }
}

/// Persist one run's outcomes and schedule. Every write is a dated refresh
/// state or exact evidence record; provider transactions are never rewritten.
pub fn commit(
    store: &Store,
    plan: &RefreshPlan,
    outcomes: &[SourceOutcome],
    now: DateTime<Utc>,
) -> Result<RefreshReport> {
    let interval = Duration::seconds(i64::try_from(plan.interval_seconds)?);
    let next_due = now + interval;
    let mut entries = Vec::new();
    for item in &plan.items {
        let state = store.refresh_state(&plan.portfolio, &item.source)?;
        let outcome = match item.action {
            PlanAction::Skip => match &item.prerequisite {
                Some(reason) => RefreshOutcome::Blocked {
                    reason: reason.clone(),
                },
                None => RefreshOutcome::NotDue,
            },
            PlanAction::StartSchedule => {
                store.save_refresh_state(
                    &plan.portfolio,
                    &item.source,
                    &RefreshState {
                        next_due: Some(next_due),
                        ..state
                    },
                )?;
                RefreshOutcome::Scheduled
            }
            PlanAction::Run(_) => {
                let outcome = outcomes.iter().find(|o| o.source == item.source);
                let result = match outcome.map(|o| &o.result) {
                    Some(Ok(acquired)) => apply(store, acquired),
                    Some(Err(failure)) => Err(*failure),
                    None => Err(FailureCode::AcquisitionFailed),
                };
                let (outcome, state) = match result {
                    Ok(outcome) => (
                        outcome,
                        RefreshState {
                            next_due: Some(next_due),
                            last_attempt: Some(now),
                            last_success: Some(now),
                            last_error: None,
                        },
                    ),
                    Err(failure) => (
                        RefreshOutcome::Failed {
                            code: failure.id().into(),
                            message: failure.text(),
                        },
                        RefreshState {
                            next_due: Some(next_due),
                            last_attempt: Some(now),
                            last_success: state.last_success,
                            last_error: Some(failure.text()),
                        },
                    ),
                };
                store.save_refresh_state(&plan.portfolio, &item.source, &state)?;
                outcome
            }
        };
        entries.push(RefreshEntry {
            source: item.source.clone(),
            label: item.label.clone(),
            outcome,
        });
    }
    Ok(RefreshReport {
        at: now,
        trigger: plan.trigger,
        interval_seconds: plan.interval_seconds,
        in_flight: false,
        entries,
    })
}

fn apply(store: &Store, acquired: &Acquired) -> std::result::Result<RefreshOutcome, FailureCode> {
    match acquired {
        Acquired::Prices(prices) => {
            if prices.is_empty() {
                // No reviewed price mapping covers the held assets; nothing is
                // fabricated and the source is not a failure.
                return Ok(RefreshOutcome::Unchanged);
            }
            for price in prices {
                store
                    .save_price_evidence(price)
                    .map_err(|_| FailureCode::ImportFailed)?;
            }
            Ok(RefreshOutcome::Updated {
                detail: format!("{} exact current prices", prices.len()),
            })
        }
        Acquired::Kraken { account, snapshot } => {
            let hash = kraken_adapter::import_snapshot(store, account, snapshot)
                .map_err(|_| FailureCode::ImportFailed)?;
            kraken_adapter::reconcile(store, &account.id, snapshot, &hash)
                .map_err(|_| FailureCode::ReconciliationFailed)?;
            Ok(RefreshOutcome::Updated {
                detail: format!("{} ledger entries reconciled", snapshot.ledger.len()),
            })
        }
        Acquired::Nexo { path } => match NexoSvc::from_path(path).import_into(store) {
            Ok(crate::adapters::nexo::NexoImportOutcome::Imported(imported)) => {
                Ok(RefreshOutcome::Updated {
                    detail: format!("{imported} transactions"),
                })
            }
            Ok(crate::adapters::nexo::NexoImportOutcome::AlreadyImported) => {
                Ok(RefreshOutcome::Unchanged)
            }
            Err(_) => Err(FailureCode::ImportFailed),
        },
    }
}

/// Display state for one configured source, including stale failures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshStatus {
    pub source: String,
    pub label: String,
    pub ready: bool,
    pub prerequisite: Option<String>,
    pub last_attempt: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub next_due: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

impl From<&Candidate> for RefreshStatus {
    fn from(candidate: &Candidate) -> Self {
        Self {
            source: candidate.source.clone(),
            label: candidate.label.clone(),
            ready: candidate.prerequisite.is_none(),
            prerequisite: candidate.prerequisite.clone(),
            last_attempt: candidate.state.last_attempt,
            last_success: candidate.state.last_success,
            next_due: candidate.state.next_due,
            last_error: candidate.state.last_error.clone(),
        }
    }
}

/// A refresh that could not start because an equivalent run was already active.
pub fn coalesced_report(
    trigger: RefreshTrigger,
    interval_seconds: u64,
    at: DateTime<Utc>,
) -> RefreshReport {
    RefreshReport {
        at,
        trigger,
        interval_seconds,
        in_flight: true,
        entries: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_core::DEFAULT_PORTFOLIO;

    const DAY: u64 = 86_400;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }
    fn source(source: &str, next_due: Option<DateTime<Utc>>) -> Candidate {
        Candidate {
            source: source.into(),
            label: source.into(),
            state: RefreshState {
                next_due,
                ..Default::default()
            },
            acquisition: Some(Acquisition::Nexo {
                path: "export.csv".into(),
            }),
            prerequisite: None,
        }
    }
    fn actions(plan: &RefreshPlan) -> Vec<(&str, PlanAction)> {
        plan.items
            .iter()
            .map(|item| (item.source.as_str(), item.action))
            .collect()
    }

    #[test]
    fn launch_starts_unscheduled_intervals_without_contacting_anything() {
        let now = at(1_700_000_000);
        let plan = plan_fn(
            RefreshTrigger::Launch,
            false,
            DAY,
            vec![source("a", None)],
            now,
        );
        assert_eq!(actions(&plan), vec![("a", PlanAction::StartSchedule)]);
        assert_eq!(plan.runs().count(), 0);
        assert!(!plan.is_idle());
    }

    #[test]
    fn launch_refreshes_once_when_an_interval_was_missed() {
        let now = at(1_700_000_000);
        let due = source("a", Some(now - Duration::hours(3)));
        let fresh = source("b", Some(now + Duration::hours(3)));
        let plan = plan_fn(RefreshTrigger::Launch, false, DAY, vec![due, fresh], now);
        assert_eq!(
            actions(&plan),
            vec![
                ("a", PlanAction::Run(RunReason::MissedSchedule)),
                ("b", PlanAction::Skip),
            ]
        );
        assert!(plan.runs().next().unwrap().acquisition.is_some());
    }

    #[test]
    fn refresh_on_open_runs_every_source_regardless_of_schedule() {
        let now = at(1_700_000_000);
        let plan = plan_fn(
            RefreshTrigger::Launch,
            true,
            DAY,
            vec![
                source("a", Some(now + Duration::hours(3))),
                source("b", None),
            ],
            now,
        );
        assert_eq!(
            actions(&plan),
            vec![
                ("a", PlanAction::Run(RunReason::RefreshOnOpen)),
                ("b", PlanAction::Run(RunReason::RefreshOnOpen)),
            ]
        );
    }

    #[test]
    fn scheduled_ticks_refresh_only_due_sources_and_manual_feeds_everything() {
        let now = at(1_700_000_000);
        let candidates = vec![
            source("due", Some(now)),
            source("later", Some(now + Duration::minutes(1))),
        ];
        assert_eq!(
            actions(&plan_fn(
                RefreshTrigger::Scheduled,
                false,
                DAY,
                candidates.clone(),
                now
            )),
            vec![
                ("due", PlanAction::Run(RunReason::Scheduled)),
                ("later", PlanAction::Skip),
            ]
        );
        assert_eq!(
            actions(&plan_fn(
                RefreshTrigger::Manual,
                false,
                DAY,
                candidates,
                now
            )),
            vec![
                ("due", PlanAction::Run(RunReason::Manual)),
                ("later", PlanAction::Run(RunReason::Manual)),
            ]
        );
    }

    #[test]
    fn a_prerequisite_blocks_runs_and_schedules_but_stays_visible() {
        let now = at(1_700_000_000);
        for trigger in [
            RefreshTrigger::Manual,
            RefreshTrigger::Launch,
            RefreshTrigger::Scheduled,
        ] {
            let mut blocked = source("kraken", None);
            blocked.acquisition = None;
            blocked.prerequisite = Some("missing credentials".into());
            let plan = plan_fn(
                trigger,
                true,
                DAY,
                vec![blocked, source("prices", None)],
                now,
            );
            assert_eq!(
                actions(&plan),
                vec![
                    ("kraken", PlanAction::Skip),
                    (
                        "prices",
                        if trigger == RefreshTrigger::Scheduled {
                            PlanAction::StartSchedule
                        } else {
                            PlanAction::Run(if trigger == RefreshTrigger::Manual {
                                RunReason::Manual
                            } else {
                                RunReason::RefreshOnOpen
                            })
                        }
                    ),
                ]
            );
            assert_eq!(
                plan.items[0].prerequisite.as_deref(),
                Some("missing credentials")
            );
            assert!(plan.items[0].acquisition.is_none());
        }
    }

    #[test]
    fn commit_records_evidence_schedule_and_redacted_failures() {
        let store = Store::in_memory().unwrap();
        let portfolio = PortfolioId::from(DEFAULT_PORTFOLIO);
        let now = at(1_700_000_000);
        let candidates = vec![source("ok", None), source("broken", None), {
            let mut blocked = source("blocked", None);
            blocked.acquisition = None;
            blocked.prerequisite = Some("needs credentials".into());
            blocked
        }];
        let plan = plan_fn(RefreshTrigger::Manual, false, DAY, candidates, now);
        // Empty exact quotes still prove the success path without any file or
        // network dependency; failures carry only redacted text.
        let outcomes = vec![
            SourceOutcome {
                source: "ok".into(),
                result: Ok(Acquired::Prices(Vec::new())),
            },
            SourceOutcome {
                source: "broken".into(),
                result: Err(FailureCode::AcquisitionFailed),
            },
        ];
        let report = commit(&store, &plan, &outcomes, now).unwrap();
        assert!(report.changed());
        assert_eq!(report.failures().count(), 1);
        assert_eq!(
            report
                .entries
                .iter()
                .map(|entry| &entry.outcome)
                .find_map(|outcome| match outcome {
                    RefreshOutcome::Failed { message, .. } => Some(message.clone()),
                    _ => None,
                })
                .as_deref(),
            Some(FailureCode::AcquisitionFailed.text().as_str())
        );
        // A successful source advances both timestamps; a failure keeps the last
        // success, records a redacted reason and schedules the next attempt.
        let saved = store.refresh_state(&portfolio, "ok").unwrap();
        assert_eq!(saved.last_attempt, Some(now));
        assert_eq!(saved.last_success, Some(now));
        assert_eq!(saved.last_error, None);
        assert_eq!(saved.next_due, Some(now + Duration::seconds(DAY as i64)));
        let failed = store.refresh_state(&portfolio, "broken").unwrap();
        assert_eq!(failed.last_attempt, Some(now));
        assert_eq!(failed.last_success, None);
        assert_eq!(
            failed.last_error.as_deref(),
            Some(
                "acquisition_failed: the source could not be reached or refused the request from this app"
            )
        );
        let blocked = store.refresh_state(&portfolio, "blocked").unwrap();
        assert_eq!(blocked, RefreshState::default());
        // Nothing is due immediately after a run.
        let again = plan_fn(
            RefreshTrigger::Scheduled,
            false,
            DAY,
            candidates_from(&store, &portfolio),
            now + Duration::minutes(1),
        );
        assert!(again.is_idle());
    }

    // Planning is pure; the wrappers keep the tests readable.
    fn plan_fn(
        trigger: RefreshTrigger,
        refresh_on_open: bool,
        interval: u64,
        candidates: Vec<Candidate>,
        now: DateTime<Utc>,
    ) -> RefreshPlan {
        plan(
            PortfolioId::from(DEFAULT_PORTFOLIO),
            trigger,
            refresh_on_open,
            interval,
            candidates,
            now,
        )
    }
    fn candidates_from(store: &Store, portfolio: &PortfolioId) -> Vec<Candidate> {
        ["ok", "broken"]
            .into_iter()
            .map(|source| Candidate {
                source: source.into(),
                label: source.into(),
                state: store.refresh_state(portfolio, source).unwrap(),
                acquisition: Some(Acquisition::Nexo {
                    path: "export.csv".into(),
                }),
                prerequisite: None,
            })
            .collect()
    }
}
