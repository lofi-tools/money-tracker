//! OS locking and atomic wall-clock state for the shared limiter.
use super::*;
use crate::{BinanceError, error::*};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

/// Keep a stable lock inode separate from the atomically replaced JSON state.
/// Closing the file releases its OS lock, including on cancellation/process exit.
pub(crate) struct Permit {
    _lock: Option<std::fs::File>,
}

#[derive(Serialize, Deserialize)]
struct SavedBudget {
    ceiling: u32,
    effective: u32,
    calls: VecDeque<(i64, u32)>,
    total: u64,
    remote: Option<(i64, u32, u64)>,
    successes: usize,
}
#[derive(Serialize, Deserialize)]
struct SavedState {
    version: u32,
    next_request_ms: i64,
    interval_ms: u64,
    budgets: HashMap<String, SavedBudget>,
}
impl Pacer {
    pub async fn acquire(&mut self, path: &str) -> Result<Permit, BinanceError> {
        let Some(directory) = self.directory.clone() else {
            self.wait(path).await;
            return Ok(Permit { _lock: None });
        };
        std::fs::create_dir_all(&directory).context(LimiterIoSnafu {
            operation: "create directory for",
            path: &directory,
        })?;
        let lock_path = directory.join("state.lock");
        loop {
            let lock = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path)
                .context(LimiterIoSnafu {
                    operation: "open lock for",
                    path: &lock_path,
                })?;
            match lock.try_lock() {
                Ok(()) => (),
                Err(std::fs::TryLockError::WouldBlock) => {
                    drop(lock);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
                Err(std::fs::TryLockError::Error(source)) => {
                    return Err(BinanceError::LimiterIo {
                        operation: "lock",
                        path: lock_path,
                        source,
                    });
                }
            }
            let permit = Permit { _lock: Some(lock) };
            self.restore()?;
            let now = Instant::now();
            let until = self
                .budget(path)
                .ready_at(profile(path).0, now)
                .max(self.next_request);
            if until <= now {
                return Ok(permit);
            }
            // Do not hold the file lock during a budget/cooldown wait. Re-read
            // after waking because another process may extend the cooldown.
            drop(permit);
            tokio::time::sleep_until(until).await;
        }
    }
    fn restore(&mut self) -> Result<(), BinanceError> {
        let directory = self
            .directory
            .clone()
            .expect("persistent limiter has directory");
        let path = directory.join("state-v1.json");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                *self = Self::persistent(directory);
                return Ok(());
            }
            Err(source) => {
                return Err(BinanceError::LimiterIo {
                    operation: "read",
                    path,
                    source,
                });
            }
        };
        let saved: SavedState =
            serde_json::from_slice(&bytes).context(LimiterDecodeSnafu { path: &path })?;
        ensure!(
            saved.version == 1 && (250..=30000).contains(&saved.interval_ms),
            InvalidLimiterStateSnafu {
                reason: "unsupported version or pacing interval"
            }
        );
        let now = Instant::now();
        let wall = chrono::Utc::now().timestamp_millis();
        let decode = |stamp: i64| -> Result<Instant, BinanceError> {
            let delta = stamp
                .checked_sub(wall)
                .ok_or(BinanceError::InvalidLimiterState {
                    reason: "timestamp overflow",
                })?;
            let duration = Duration::from_millis(delta.unsigned_abs());
            if delta >= 0 {
                now.checked_add(duration)
            } else {
                now.checked_sub(duration)
            }
            .ok_or(BinanceError::InvalidLimiterState {
                reason: "timestamp outside supported timer range",
            })
        };
        let next_request = decode(saved.next_request_ms)?.max(now);
        let mut budgets = HashMap::new();
        for (key, value) in saved.budgets {
            ensure!(
                value.ceiling > 0 && value.effective > 0 && value.effective <= value.ceiling,
                InvalidLimiterStateSnafu {
                    reason: "invalid weighted budget"
                }
            );
            let mut calls = VecDeque::new();
            let mut last_stamp = None;
            for (stamp, cost) in value.calls {
                ensure!(
                    cost > 0 && last_stamp.is_none_or(|last| last <= stamp),
                    InvalidLimiterStateSnafu {
                        reason: "invalid call history"
                    }
                );
                last_stamp = Some(stamp);
                if wall.saturating_sub(stamp) < 60000 {
                    calls.push_back((decode(stamp)?, cost));
                }
            }
            let remote = match value.remote {
                Some((stamp, used, total)) if wall.saturating_sub(stamp) < 60000 => {
                    ensure!(
                        total <= value.total,
                        InvalidLimiterStateSnafu {
                            reason: "invalid server observation"
                        }
                    );
                    Some((decode(stamp)?, used, total))
                }
                _ => None,
            };
            budgets.insert(
                key,
                Budget {
                    ceiling: value.ceiling,
                    effective: value.effective,
                    calls,
                    total: value.total,
                    remote,
                    successes: value.successes,
                },
            );
        }
        self.next_request = next_request;
        self.interval = Duration::from_millis(saved.interval_ms);
        self.budgets = budgets;
        self.active = None;
        Ok(())
    }
    pub fn persist(&self, permit: &Permit) -> Result<(), BinanceError> {
        let Some(directory) = &self.directory else {
            return Ok(());
        };
        ensure!(
            permit._lock.is_some(),
            InvalidLimiterStateSnafu {
                reason: "state write without lock"
            }
        );
        let now = Instant::now();
        // Round wall time upward so serialization never shortens a cooldown.
        let wall = chrono::Utc::now().timestamp_millis().saturating_add(1);
        let encode = |at: Instant| -> Result<i64, BinanceError> {
            let (positive, duration) = if at >= now {
                (true, at.duration_since(now))
            } else {
                (false, now.duration_since(at))
            };
            let rounded = if positive {
                duration.as_nanos().div_ceil(1_000_000)
            } else {
                duration.as_millis()
            };
            let millis = i64::try_from(rounded).map_err(|_| BinanceError::InvalidLimiterState {
                reason: "timestamp overflow",
            })?;
            (if positive {
                wall.checked_add(millis)
            } else {
                wall.checked_sub(millis)
            })
            .ok_or(BinanceError::InvalidLimiterState {
                reason: "timestamp overflow",
            })
        };
        let mut budgets = HashMap::new();
        for (key, value) in &self.budgets {
            budgets.insert(
                key.clone(),
                SavedBudget {
                    ceiling: value.ceiling,
                    effective: value.effective,
                    total: value.total,
                    successes: value.successes,
                    calls: value
                        .calls
                        .iter()
                        .filter(|(at, _)| now.saturating_duration_since(*at) < WINDOW)
                        .map(|(at, cost)| Ok((encode(*at)?, *cost)))
                        .collect::<Result<_, BinanceError>>()?,
                    remote: value
                        .remote
                        .filter(|(at, _, _)| now.saturating_duration_since(*at) < WINDOW)
                        .map(|(at, used, total)| Ok((encode(at)?, used, total)))
                        .transpose()?,
                },
            );
        }
        let saved = SavedState {
            version: 1,
            next_request_ms: encode(self.next_request)?,
            interval_ms: u64::try_from(self.interval.as_millis()).unwrap_or(30000),
            budgets,
        };
        let path = directory.join("state-v1.json");
        let mut file = tempfile::NamedTempFile::new_in(directory).context(LimiterIoSnafu {
            operation: "create temporary",
            path: &path,
        })?;
        serde_json::to_writer(file.as_file_mut(), &saved).context(LimiterEncodeSnafu)?;
        file.as_file().sync_all().context(LimiterIoSnafu {
            operation: "sync",
            path: &path,
        })?;
        file.persist(&path)
            .map_err(|error| error.error)
            .context(LimiterIoSnafu {
                operation: "replace",
                path: &path,
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, Write};

    fn child(directory: &std::path::Path, mode: &str) -> std::process::Command {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "backpressure::persistence::tests::process_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("BINANCE_LIMITER_TEST_DIRECTORY", directory)
            .env("BINANCE_LIMITER_TEST_MODE", mode);
        command
    }
    fn saved(directory: &std::path::Path) -> SavedState {
        serde_json::from_slice(&std::fs::read(directory.join("state-v1.json")).unwrap()).unwrap()
    }

    #[test]
    #[ignore = "subprocess helper; requires BINANCE_LIMITER_TEST_DIRECTORY"]
    fn process_worker() -> anyhow::Result<()> {
        let Some(directory) = std::env::var_os("BINANCE_LIMITER_TEST_DIRECTORY") else {
            return Ok(());
        };
        let directory = PathBuf::from(directory);
        let mode = std::env::var("BINANCE_LIMITER_TEST_MODE")?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let mut pacer = Pacer::persistent(directory.clone());
                let permit = pacer.acquire("/api/v3/account").await?;
                pacer.start_request("/api/v3/account");
                if mode == "cooldown" || mode == "hold" {
                    pacer.cool_down(Duration::from_secs(2)).unwrap();
                }
                pacer.persist(&permit)?;
                writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(directory.join("sent.log"))?,
                    "{}",
                    chrono::Utc::now().timestamp_millis()
                )?;
                if mode == "hold" {
                    println!("LOCK_HELD");
                    std::io::stdout().flush()?;
                    tokio::time::sleep(Duration::from_secs(30)).await;
                } else {
                    // Simulate a response still in flight while another process starts.
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    pacer.success();
                    pacer.persist(&permit)?;
                }
                Ok::<_, anyhow::Error>(())
            })
    }

    #[test]
    fn parallel_processes_share_reservations_and_cooldown() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let mut first = child(directory.path(), "hold")
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        let mut lines = std::io::BufReader::new(first.stdout.take().unwrap()).lines();
        assert!(
            lines
                .by_ref()
                .any(|line| line.is_ok_and(|line| line == "LOCK_HELD"))
        );
        let mut second = child(directory.path(), "reserve")
            .stdout(std::process::Stdio::null())
            .spawn()?;
        // OS lock excludes another process while first's response is in flight.
        std::thread::sleep(Duration::from_millis(100));
        assert!(second.try_wait()?.is_none());
        assert_eq!(saved(directory.path()).budgets["spot"].calls.len(), 1);
        // Killing the owner releases the lock, but its saved cooldown and
        // reservation remain. The waiting process must honor both.
        first.kill()?;
        first.wait()?;
        assert!(second.wait()?.success());
        let state = saved(directory.path());
        assert_eq!(state.budgets["spot"].calls.len(), 2);
        assert_eq!(state.budgets["spot"].total, 40);
        assert_eq!(state.budgets["spot"].effective, 2400);
        let times: Vec<i64> = std::fs::read_to_string(directory.path().join("sent.log"))?
            .lines()
            .map(|line| line.parse().unwrap())
            .collect();
        assert_eq!(times.len(), 2);
        assert!(times[1] - times[0] >= 1900, "cooldown was lost: {times:?}");
        // Two more independent processes cannot overwrite each other's counters.
        let mut a = child(directory.path(), "reserve")
            .stdout(std::process::Stdio::null())
            .spawn()?;
        let mut b = child(directory.path(), "reserve")
            .stdout(std::process::Stdio::null())
            .spawn()?;
        assert!(a.wait()?.success());
        assert!(b.wait()?.success());
        assert_eq!(saved(directory.path()).budgets["spot"].total, 80);
        Ok(())
    }

    #[tokio::test]
    async fn corrupted_state_is_not_reset_and_expired_history_is_pruned() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("state-v1.json");
        std::fs::write(&path, "broken")?;
        let mut pacer = Pacer::persistent(directory.path().into());
        assert!(matches!(
            pacer.acquire("/api/v3/account").await,
            Err(BinanceError::LimiterDecode { .. })
        ));
        assert_eq!(std::fs::read_to_string(&path)?, "broken");
        std::fs::remove_file(&path)?;
        let permit = pacer.acquire("/api/v3/account").await?;
        pacer.start_request("/api/v3/account");
        pacer.persist(&permit)?;
        drop(permit);
        let mut state = saved(directory.path());
        state.next_request_ms = chrono::Utc::now().timestamp_millis() - 1;
        state.budgets.get_mut("spot").unwrap().calls[0].0 -= 120000;
        state.budgets.get_mut("spot").unwrap().remote =
            Some((chrono::Utc::now().timestamp_millis() - 120000, 6000, 20));
        std::fs::write(&path, serde_json::to_vec(&state)?)?;
        let _permit = pacer.acquire("/api/v3/account").await?;
        assert!(pacer.budget("/api/v3/account").calls.is_empty());
        assert!(pacer.budget("/api/v3/account").remote.is_none());
        Ok(())
    }
}
