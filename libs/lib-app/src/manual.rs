//! Typed manual edits for host controls. Each write goes through the core
//! action log, so provider evidence and source labels are never overwritten.
//! Manual rate/price periods are dated: a backdated correction keeps later
//! periods, and pins protect the applicable manual period from newer evidence.
use crate::dto::ExactDecimal;
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use lib_core::{
    Account, AccountId, AssetId, PortfolioId, ProviderId, Store,
    ui_metadata::{
        AccountCustomization, AnnualRate, DustMode, LocalRecord, LocalTarget, LocalValue,
        ManualAction, ManualPrice, ManualRate, RateScope,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static GROUP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManualEdit {
    /// `rate: None` records an explicit unknown rate period; `clear` removes the
    /// dated manual period and falls back to source evidence when none remain.
    Rate {
        scope: RateScope,
        rate: Option<AnnualRate>,
        effective_at: DateTime<Utc>,
        pinned: bool,
        clear: bool,
    },
    RatePin {
        scope: RateScope,
        pinned: bool,
    },
    Price {
        asset: AssetId,
        quote: AssetId,
        price: Option<ExactDecimal>,
        effective_at: DateTime<Utc>,
        pinned: bool,
        clear: bool,
    },
    PricePin {
        asset: AssetId,
        quote: AssetId,
        pinned: bool,
    },
    AccountName {
        account: AccountId,
        name: Option<String>,
    },
    SubscriptionName {
        id: String,
        name: Option<String>,
    },
    Group {
        provider: ProviderId,
        name: String,
        parent: Option<AccountId>,
    },
    Move {
        account: AccountId,
        parent: Option<AccountId>,
        source_default: bool,
    },
    Dust {
        provider: ProviderId,
        asset: AssetId,
        mode: DustMode,
    },
    Spam {
        asset: AssetId,
        excluded: bool,
        reason: String,
    },
}

fn manual_rates(record: &LocalRecord) -> Vec<ManualRate> {
    match &record.value {
        Some(LocalValue::Rates(periods)) => periods.clone(),
        _ => vec![],
    }
}
fn manual_prices(record: &LocalRecord) -> Vec<ManualPrice> {
    match &record.value {
        Some(LocalValue::Prices(periods)) => periods.clone(),
        _ => vec![],
    }
}
fn customization(record: &LocalRecord) -> AccountCustomization {
    match &record.value {
        Some(LocalValue::Account(value)) => value.clone(),
        _ => AccountCustomization::default(),
    }
}
fn not_future(when: DateTime<Utc>) -> Result<()> {
    ensure!(
        when <= Utc::now(),
        "effective date cannot be in the future; it would never apply"
    );
    Ok(())
}
fn sorted(mut periods: Vec<ManualRate>) -> Vec<ManualRate> {
    periods.sort_by(|a, b| {
        a.effective_at
            .cmp(&b.effective_at)
            .then(a.updated_at.cmp(&b.updated_at))
    });
    periods
}
fn sorted_prices(mut periods: Vec<ManualPrice>) -> Vec<ManualPrice> {
    periods.sort_by(|a, b| {
        a.effective_at
            .cmp(&b.effective_at)
            .then(a.updated_at.cmp(&b.updated_at))
    });
    periods
}
impl crate::Application {
    /// Apply one typed edit inside the caller's serialized application job.
    /// `None` means the edit changed provider-independent structure without a
    /// local-state action (a local grouping node).
    pub fn apply_manual_edit(
        &self,
        portfolio: &PortfolioId,
        edit: &ManualEdit,
    ) -> Result<Option<ManualAction>> {
        let store = self.store();
        match edit {
            ManualEdit::Rate {
                scope,
                rate,
                effective_at,
                pinned,
                clear,
            } => {
                not_future(*effective_at)?;
                let target = LocalTarget::Rate {
                    scope: scope.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                let mut periods = manual_rates(&record);
                periods.retain(|p| p.effective_at != *effective_at);
                if !clear {
                    periods.push(ManualRate {
                        effective_at: *effective_at,
                        updated_at: Utc::now(),
                        rate: rate.clone(),
                        pinned: *pinned,
                    });
                    periods = sorted(periods);
                }
                let value = (!periods.is_empty()).then_some(LocalValue::Rates(periods));
                store
                    .apply_manual_change(portfolio, &target, record.revision, value)
                    .map(Some)
            }
            ManualEdit::RatePin { scope, pinned } => {
                let target = LocalTarget::Rate {
                    scope: scope.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                let mut periods = manual_rates(&record);
                let last = periods
                    .len()
                    .checked_sub(1)
                    .context("no manual rate override to pin")?;
                periods[last].pinned = *pinned;
                store
                    .apply_manual_change(
                        portfolio,
                        &target,
                        record.revision,
                        Some(LocalValue::Rates(periods)),
                    )
                    .map(Some)
            }
            ManualEdit::Price {
                asset,
                quote,
                price,
                effective_at,
                pinned,
                clear,
            } => {
                not_future(*effective_at)?;
                let target = LocalTarget::Price {
                    asset: asset.clone(),
                    quote: quote.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                let mut periods = manual_prices(&record);
                periods.retain(|p| p.effective_at != *effective_at);
                if !clear {
                    periods.push(ManualPrice {
                        effective_at: *effective_at,
                        updated_at: Utc::now(),
                        price: price.clone().context("manual price value required")?,
                        pinned: *pinned,
                    });
                    periods = sorted_prices(periods);
                }
                let value = (!periods.is_empty()).then_some(LocalValue::Prices(periods));
                store
                    .apply_manual_change(portfolio, &target, record.revision, value)
                    .map(Some)
            }
            ManualEdit::PricePin {
                asset,
                quote,
                pinned,
            } => {
                let target = LocalTarget::Price {
                    asset: asset.clone(),
                    quote: quote.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                let mut periods = manual_prices(&record);
                let last = periods
                    .len()
                    .checked_sub(1)
                    .context("no manual price override to pin")?;
                periods[last].pinned = *pinned;
                store
                    .apply_manual_change(
                        portfolio,
                        &target,
                        record.revision,
                        Some(LocalValue::Prices(periods)),
                    )
                    .map(Some)
            }
            ManualEdit::AccountName { account, name } => {
                let target = LocalTarget::Account {
                    id: account.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                let current = customization(&record);
                let value = Some(LocalValue::Account(AccountCustomization {
                    local_label: name.clone(),
                    parent_is_custom: current.parent_is_custom,
                    parent: if current.parent_is_custom {
                        current.parent.clone()
                    } else {
                        None
                    },
                }));
                store
                    .apply_manual_change(portfolio, &target, record.revision, value)
                    .map(Some)
            }
            ManualEdit::SubscriptionName { id, name } => {
                let target = LocalTarget::Subscription { id: id.clone() };
                let record = store.local_state(portfolio, &target)?;
                let value = name.clone().map(LocalValue::SubscriptionName);
                store
                    .apply_manual_change(portfolio, &target, record.revision, value)
                    .map(Some)
            }
            ManualEdit::Group {
                provider,
                name,
                parent,
            } => {
                ensure!(!name.trim().is_empty(), "group name is empty");
                ensure!(
                    store
                        .accounts(portfolio)?
                        .iter()
                        .any(|a| a.provider_id == *provider),
                    "provider is not in this portfolio"
                );
                let id = Self::new_group_id(store, provider)?;
                let account = Account {
                    id,
                    portfolio_id: portfolio.clone(),
                    provider_id: provider.clone(),
                    label: name.trim().to_owned(),
                    source_reference: Some("local group".into()),
                };
                store.save_group_account(&account, parent.clone(), 1)?;
                Ok(None)
            }
            ManualEdit::Move {
                account,
                parent,
                source_default,
            } => {
                let target = LocalTarget::Account {
                    id: account.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                let current = customization(&record);
                let value = Some(LocalValue::Account(AccountCustomization {
                    local_label: current.local_label.clone(),
                    parent_is_custom: !source_default,
                    parent: if *source_default {
                        None
                    } else {
                        parent.clone()
                    },
                }));
                store
                    .apply_manual_change(portfolio, &target, record.revision, value)
                    .map(Some)
            }
            ManualEdit::Dust {
                provider,
                asset,
                mode,
            } => {
                let target = LocalTarget::Dust {
                    provider: provider.clone(),
                    asset: asset.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                store
                    .apply_manual_change(
                        portfolio,
                        &target,
                        record.revision,
                        Some(LocalValue::Dust(mode.clone())),
                    )
                    .map(Some)
            }
            ManualEdit::Spam {
                asset,
                excluded,
                reason,
            } => {
                let target = LocalTarget::Spam {
                    asset: asset.clone(),
                };
                let record = store.local_state(portfolio, &target)?;
                let value = if *excluded {
                    Some(LocalValue::Spam {
                        excluded: true,
                        reason: reason.trim().to_owned(),
                    })
                } else {
                    // Restoring clears the portfolio-wide decision; account reviews stay.
                    None
                };
                store
                    .apply_manual_change(portfolio, &target, record.revision, value)
                    .map(Some)
            }
        }
    }
    /// Compensating action; source evidence conflicts still require review.
    pub fn undo_manual_edit(&self, portfolio: &PortfolioId, id: &str) -> Result<ManualAction> {
        self.store().undo_manual_action(portfolio, id)
    }
    fn new_group_id(store: &Store, provider: &ProviderId) -> Result<AccountId> {
        for attempt in 0..64u64 {
            let candidate = AccountId(format!(
                "local:group:{}:{}",
                provider.0,
                GROUP_SEQUENCE.fetch_add(1, Ordering::SeqCst) + attempt * 64
            ));
            if !store.account_exists(&candidate)? {
                return Ok(candidate);
            }
        }
        anyhow::bail!("unable to allocate a unique local group ID")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Application;
    use lib_core::{DEFAULT_PORTFOLIO, Store, ui_metadata::AnnualRateBasis};

    fn seed(store: &Store) -> Result<()> {
        store.save_account(&Account {
            id: "wallet".into(),
            portfolio_id: DEFAULT_PORTFOLIO.into(),
            provider_id: "nexo".into(),
            label: "My wallet".into(),
            source_reference: None,
        })?;
        store.save_asset_scale(&AssetId::str("USD"), 2)?;
        Ok(())
    }
    #[test]
    fn dated_manual_rates_keep_later_periods_and_pins() -> Result<()> {
        let app = Application::from_store(Store::in_memory()?);
        seed(app.store())?;
        let portfolio = DEFAULT_PORTFOLIO.into();
        let later = "2026-06-01T00:00:00Z".parse()?;
        let earlier = "2026-01-01T00:00:00Z".parse()?;
        let rate = |fraction: &str| -> Result<AnnualRate> {
            Ok(AnnualRate {
                fraction: fraction.parse()?,
                basis: AnnualRateBasis::Apy,
                compounds_per_year: None,
                reward_asset: None,
            })
        };
        let scope = RateScope::AccountAsset {
            account: "wallet".into(),
            asset: AssetId::str("USD"),
        };
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::Rate {
                scope: scope.clone(),
                rate: Some(rate("0.05")?),
                effective_at: later,
                pinned: false,
                clear: false,
            },
        )?;
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::Rate {
                scope: scope.clone(),
                rate: Some(rate("0.04")?),
                effective_at: earlier,
                pinned: false,
                clear: false,
            },
        )?;
        let selected = app.store().selected_rate(&portfolio, &scope, later)?;
        assert_eq!(selected.unwrap().rate.unwrap().fraction.as_str(), "0.05");
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::RatePin {
                scope: scope.clone(),
                pinned: true,
            },
        )?;
        assert!(
            app.store()
                .selected_rate(&portfolio, &scope, later)?
                .unwrap()
                .pinned
        );
        // A backdated correction does not erase the newer manual period.
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::Rate {
                scope: scope.clone(),
                rate: Some(rate("0.06")?),
                effective_at: earlier,
                pinned: false,
                clear: false,
            },
        )?;
        let after = app.store().selected_rate(&portfolio, &scope, later)?;
        assert_eq!(after.unwrap().rate.unwrap().fraction.as_str(), "0.05");
        // Clearing the newest period falls back to the earlier one.
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::Rate {
                scope: scope.clone(),
                rate: None,
                effective_at: later,
                pinned: false,
                clear: true,
            },
        )?;
        let fallback = app.store().selected_rate(&portfolio, &scope, later)?;
        assert_eq!(fallback.unwrap().rate.unwrap().fraction.as_str(), "0.06");
        Ok(())
    }
    #[test]
    fn future_effective_dates_and_pin_without_override_are_rejected() -> Result<()> {
        let app = Application::from_store(Store::in_memory()?);
        seed(app.store())?;
        let portfolio = DEFAULT_PORTFOLIO.into();
        let scope = RateScope::AccountAsset {
            account: "wallet".into(),
            asset: AssetId::str("USD"),
        };
        assert!(
            app.apply_manual_edit(
                &portfolio,
                &ManualEdit::Rate {
                    scope: scope.clone(),
                    rate: None,
                    effective_at: Utc::now() + chrono::Duration::days(1),
                    pinned: false,
                    clear: false,
                },
            )
            .is_err()
        );
        assert!(
            app.apply_manual_edit(
                &portfolio,
                &ManualEdit::RatePin {
                    scope,
                    pinned: true,
                },
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn group_and_move_stay_inside_one_provider_and_are_undoable() -> Result<()> {
        let app = Application::from_store(Store::in_memory()?);
        seed(app.store())?;
        app.store().save_account(&Account {
            id: "other".into(),
            portfolio_id: DEFAULT_PORTFOLIO.into(),
            provider_id: "binance".into(),
            label: "Other provider".into(),
            source_reference: None,
        })?;
        let portfolio = DEFAULT_PORTFOLIO.into();
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::Group {
                provider: ProviderId("nexo".into()),
                name: "Fixed terms".into(),
                parent: None,
            },
        )?;
        assert!(
            app.store()
                .accounts(&portfolio)?
                .iter()
                .any(|a| a.label == "Fixed terms"),
            "group accounts persist without a local action"
        );
        let group = app
            .store()
            .accounts(&portfolio)?
            .into_iter()
            .find(|a| a.label == "Fixed terms")
            .context("group account")?;
        assert!(app.store().account_parent(&group.id)?.is_none());
        let moved = app
            .apply_manual_edit(
                &portfolio,
                &ManualEdit::Move {
                    account: "wallet".into(),
                    parent: Some(group.id.clone()),
                    source_default: false,
                },
            )?
            .context("move records a local action")?;
        assert_eq!(
            app.store().account_parent(&"wallet".into())?,
            Some(group.id.clone())
        );
        // Cross-provider parents are rejected by the core validation.
        assert!(
            app.apply_manual_edit(
                &portfolio,
                &ManualEdit::Move {
                    account: "wallet".into(),
                    parent: Some("other".into()),
                    source_default: false,
                },
            )
            .is_err()
        );
        // Undo restores the provider default parent and cannot repeat.
        let restored = app.undo_manual_edit(&portfolio, &moved.id)?;
        assert_eq!(restored.undo_of, Some(moved.id.clone()));
        assert_eq!(app.store().account_parent(&"wallet".into())?, None);
        assert!(
            app.undo_manual_edit(&portfolio, &moved.id).is_err(),
            "an undone action cannot be undone again"
        );
        // A local name is independent of the hierarchy and keeps its source label.
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::AccountName {
                account: "wallet".into(),
                name: Some("Terms wallet".into()),
            },
        )?;
        assert_eq!(
            app.store()
                .local_state(
                    &portfolio,
                    &LocalTarget::Account {
                        id: "wallet".into()
                    },
                )?
                .value,
            Some(LocalValue::Account(AccountCustomization {
                local_label: Some("Terms wallet".into()),
                parent_is_custom: false,
                parent: None,
            }))
        );
        Ok(())
    }
    #[test]
    fn dust_and_spam_edits_are_scoped_and_reversible() -> Result<()> {
        let app = Application::from_store(Store::in_memory()?);
        seed(app.store())?;
        let portfolio = DEFAULT_PORTFOLIO.into();
        app.apply_manual_edit(
            &portfolio,
            &ManualEdit::Dust {
                provider: "nexo".into(),
                asset: AssetId::str("USD"),
                mode: DustMode::Significant,
            },
        )?;
        assert_eq!(
            app.store()
                .local_state(
                    &portfolio,
                    &LocalTarget::Dust {
                        provider: "nexo".into(),
                        asset: AssetId::str("USD"),
                    },
                )?
                .value,
            Some(LocalValue::Dust(DustMode::Significant))
        );
        assert!(
            app.apply_manual_edit(
                &portfolio,
                &ManualEdit::Dust {
                    provider: ProviderId("binance".into()),
                    asset: AssetId::str("USD"),
                    mode: DustMode::Significant,
                },
            )
            .is_err(),
            "dust requires the provider in this portfolio"
        );
        let excluded = app
            .apply_manual_edit(
                &portfolio,
                &ManualEdit::Spam {
                    asset: AssetId::str("SHIB"),
                    excluded: true,
                    reason: "airdrop spam".into(),
                },
            )?
            .context("spam records a local action")?;
        assert!(
            app.store()
                .asset_is_excluded(&"wallet".into(), &AssetId::str("SHIB"))?
        );
        app.undo_manual_edit(&portfolio, &excluded.id)?;
        assert!(
            !app.store()
                .asset_is_excluded(&"wallet".into(), &AssetId::str("SHIB"))?
        );
        Ok(())
    }
}
