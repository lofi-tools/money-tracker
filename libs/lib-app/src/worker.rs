//! One dedicated thread owns DuckDB. Hosts await typed requests off the UI thread.
use crate::{
    Application,
    config::{ResolvedConfig, Settings},
    dto::{LedgerBalancesDto, ReadContext},
    startup::{self, MissingDatabase, OpenAction, Opened},
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use lib_core::PortfolioId;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use tokio::sync::{Mutex, oneshot};

type Job = Box<dyn FnOnce(&Application, &mut ResolvedConfig) + Send>;
enum Message {
    Run(Job),
    Close,
}
struct Inner {
    sender: mpsc::Sender<Message>,
    closing: Arc<AtomicBool>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.closing.store(true, Ordering::SeqCst);
        let _ = self.sender.send(Message::Close);
        // Dropping the last handle signals shutdown; explicit close() awaits it.
    }
}
#[derive(Clone)]
pub struct DatabaseWorker {
    inner: Arc<Inner>,
    portfolio_id: PortfolioId,
}
enum Initialized {
    Ready(PortfolioId),
    Missing(MissingDatabase),
}
pub enum StartupOutcome {
    Missing(MissingDatabase),
    Ready(DatabaseWorker),
}
impl DatabaseWorker {
    pub async fn start(resolved: ResolvedConfig, action: OpenAction) -> Result<StartupOutcome> {
        resolved.config.validate()?;
        let (sender, receiver) = mpsc::channel();
        let closing = Arc::new(AtomicBool::new(false));
        let closed = closing.clone();
        let (ready, result) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("money-tracker-db".into())
            .spawn(move || {
                let mut resolved = resolved;
                match startup::open(&resolved, action) {
                    Ok(Opened::Ready(app, portfolio)) => {
                        if ready.send(Ok(Initialized::Ready(portfolio))).is_err() {
                            return;
                        }
                        while let Ok(message) = receiver.recv() {
                            if closed.load(Ordering::SeqCst) {
                                break;
                            }
                            match message {
                                Message::Run(job) => job(&app, &mut resolved),
                                Message::Close => break,
                            }
                        }
                        // Application/Store close on their owning thread. Pending jobs
                        // are dropped; no detached refresh or provider task is spawned.
                    }
                    Ok(Opened::Missing(missing)) => {
                        let _ = ready.send(Ok(Initialized::Missing(missing)));
                    }
                    Err(e) => {
                        let _ = ready.send(Err(e));
                    }
                }
            })?;
        let inner = Arc::new(Inner {
            sender,
            closing,
            thread: Mutex::new(Some(thread)),
        });
        match result.await.context("database startup worker stopped")?? {
            Initialized::Ready(portfolio_id) => Ok(StartupOutcome::Ready(Self {
                inner,
                portfolio_id,
            })),
            Initialized::Missing(missing) => {
                if let Some(thread) = inner.thread.lock().await.take() {
                    tokio::task::spawn_blocking(move || thread.join())
                        .await?
                        .map_err(|_| anyhow::anyhow!("startup worker panicked"))?;
                }
                Ok(StartupOutcome::Missing(missing))
            }
        }
    }
    pub fn portfolio_id(&self) -> &PortfolioId {
        &self.portfolio_id
    }
    async fn request<R: Send + 'static>(
        &self,
        f: impl FnOnce(&Application, &mut ResolvedConfig) -> Result<R> + Send + 'static,
    ) -> Result<R> {
        ensure!(
            !self.inner.closing.load(Ordering::SeqCst),
            "database worker is closed"
        );
        let (reply, result) = oneshot::channel();
        self.inner
            .sender
            .send(Message::Run(Box::new(move |app, config| {
                // An abandoned queued request has no effects. In-flight DB work finishes
                // atomically; close awaits it rather than interrupting a migration/write.
                if !reply.is_closed() {
                    let _ = reply.send(f(app, config));
                }
            })))
            .map_err(|_| anyhow::anyhow!("database worker stopped"))?;
        result
            .await
            .context("database request cancelled during shutdown")?
    }
    pub async fn ledger_balances(&self, cutoff: DateTime<Utc>) -> Result<LedgerBalancesDto> {
        let portfolio_id = self.portfolio_id.clone();
        self.request(move |app, _| {
            app.ledger_balances(ReadContext {
                portfolio_id,
                cutoff,
            })
        })
        .await
    }
    /// Snapshot queries use the worker's selected portfolio and saved settings.
    pub async fn portfolio_projection(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<crate::portfolio::PortfolioProjection> {
        let portfolio_id = self.portfolio_id.clone();
        self.request(move |app, resolved| {
            app.portfolio_projection(crate::portfolio::PortfolioRequest {
                context: ReadContext {
                    portfolio_id,
                    cutoff,
                },
                reporting_asset: resolved.config.settings.reporting_currency.clone(),
                dust_threshold: resolved.config.settings.dust_threshold.clone(),
            })
        })
        .await
    }
    /// Explicit current-price refresh; provider ledger and reconciliation records are untouched.
    pub async fn refresh_prices(&self) -> Result<()> {
        let projection = self.portfolio_projection(Utc::now()).await?;
        ensure!(
            projection.request.reporting_asset.0 == "USD",
            "current price refresh currently supports USD only"
        );
        let assets: Vec<_> = projection
            .assets
            .iter()
            .filter_map(|entry| {
                if let crate::portfolio::Selection::Asset(id) = &entry.selection {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();
        let prices = crate::adapters::coingecko::CoinGeckoSvc::new()?
            .fetch_exact_current_prices(&assets)
            .await?;
        self.request(move |app, _| {
            for price in prices {
                app.store().save_price_evidence(&price)?;
            }
            Ok(())
        })
        .await
    }

    pub async fn ui_metadata(&self) -> Result<crate::dto::UiMetadataDto> {
        let portfolio = self.portfolio_id.clone();
        self.request(move |app, _| app.ui_metadata(&portfolio))
            .await
    }
    pub async fn manual_actions(&self) -> Result<Vec<lib_core::ui_metadata::ManualAction>> {
        let portfolio = self.portfolio_id.clone();
        self.request(move |app, _| app.store().manual_actions(&portfolio))
            .await
    }
    /// Low-level typed local mutation; host controls/commands belong to stage 7.
    pub async fn apply_manual_change(
        &self,
        target: lib_core::ui_metadata::LocalTarget,
        expected_revision: i64,
        value: Option<lib_core::ui_metadata::LocalValue>,
    ) -> Result<lib_core::ui_metadata::ManualAction> {
        let portfolio = self.portfolio_id.clone();
        self.request(move |app, _| {
            app.store()
                .apply_manual_change(&portfolio, &target, expected_revision, value)
        })
        .await
    }
    pub async fn undo_manual_action(
        &self,
        id: String,
    ) -> Result<lib_core::ui_metadata::ManualAction> {
        let portfolio = self.portfolio_id.clone();
        self.request(move |app, _| app.store().undo_manual_action(&portfolio, &id))
            .await
    }
    pub async fn settings(&self) -> Result<Settings> {
        self.request(|_, resolved| Ok(resolved.config.settings.clone()))
            .await
    }
    pub async fn save_settings(&self, settings: Settings) -> Result<()> {
        self.request(move |_, resolved| {
            settings.validate()?;
            let mut config = resolved.config.clone();
            config.settings = settings;
            config.save(&resolved.config_file)?;
            resolved.config = config;
            Ok(())
        })
        .await
    }
    /// Reject new requests, discard queued requests and await the current operation
    /// and database close. Call on explicit Quit/window close in the future host.
    pub async fn close(&self) -> Result<()> {
        self.inner.closing.store(true, Ordering::SeqCst);
        let _ = self.inner.sender.send(Message::Close);
        let mut lifecycle = self.inner.thread.lock().await;
        let thread = lifecycle.take();
        if let Some(thread) = thread {
            tokio::task::spawn_blocking(move || {
                thread
                    .join()
                    .map_err(|_| anyhow::anyhow!("database worker panicked"))
            })
            .await??;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Locations;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn cancellation_skips_queued_mutations_and_close_waits_for_inflight_work() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let resolved = ResolvedConfig::resolve(
            Locations::for_platform(dir.path(), dir.path(), dir.path(), dir.path(), false),
            None,
            None,
            None,
        )?;
        let StartupOutcome::Ready(worker) =
            DatabaseWorker::start(resolved, OpenAction::StartNew).await?
        else {
            panic!("expected ready");
        };
        let (started_tx, started) = oneshot::channel();
        let (release, hold) = mpsc::channel();
        let first_worker = worker.clone();
        let first = tokio::spawn(async move {
            first_worker
                .request(move |_, _| {
                    let _ = started_tx.send(());
                    hold.recv_timeout(std::time::Duration::from_secs(5))?;
                    Ok(())
                })
                .await
        });
        started.await?;
        let mutations = Arc::new(AtomicUsize::new(0));
        let count = mutations.clone();
        let mut cancelled = Box::pin(worker.request(move |_, _| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                cancelled.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        drop(cancelled);
        release.send(())?;
        first.await??;
        worker.settings().await?; // Barrier: the cancelled job has been consumed.
        assert_eq!(mutations.load(Ordering::SeqCst), 0);

        let (started_tx, started) = oneshot::channel();
        let (release, hold) = mpsc::channel();
        let first_worker = worker.clone();
        let first = tokio::spawn(async move {
            first_worker
                .request(move |_, _| {
                    let _ = started_tx.send(());
                    hold.recv_timeout(std::time::Duration::from_secs(5))?;
                    Ok(())
                })
                .await
        });
        started.await?;
        let count = mutations.clone();
        let mut queued = Box::pin(worker.request(move |_, _| {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                queued.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        let closing_worker = worker.clone();
        let closing = tokio::spawn(async move { closing_worker.close().await });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !worker.inner.closing.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        assert!(!closing.is_finished());
        release.send(())?;
        first.await??;
        closing.await??;
        assert!(queued.await.is_err());
        assert_eq!(mutations.load(Ordering::SeqCst), 0);
        assert!(worker.settings().await.is_err());
        Ok(())
    }
}
