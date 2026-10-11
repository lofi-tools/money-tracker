use crate::dto::*;
use anyhow::{Context, Result, ensure};
use lib_core::{PositionRole, Store};
use std::{collections::BTreeMap, path::Path};

/// Host-independent application state. Hosts choose paths and serialize access.
/// Opening preserves Store's existing behavior. Desktop hosts use DatabaseWorker
/// for explicit startup policy and serialized access on a dedicated thread.
pub struct Application {
    store: Store,
}
impl Application {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::from_store(Store::open(path)?))
    }
    pub fn from_store(store: Store) -> Self {
        Self { store }
    }
    /// Compatibility access for existing import/price/reconciliation adapters.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Exact tracked ledger replay, explicitly unverified. This is not the stage-4
    /// current-observation/valuation/earnings projection. Include signed liabilities
    /// through account-scoped queries; never combine independent portfolios.
    pub fn ledger_balances(&self, context: ReadContext) -> Result<LedgerBalancesDto> {
        ensure!(
            self.store
                .portfolios()?
                .iter()
                .any(|p| p.id == context.portfolio_id),
            "unknown portfolio {}",
            context.portfolio_id.0
        );
        let mut positions = Vec::new();
        let mut accounts = Vec::new();
        for account in self.store.accounts(&context.portfolio_id)? {
            let metadata: BTreeMap<_, _> = self
                .store
                .account_position_metadata(&account.id)?
                .into_iter()
                .map(|(id, asset, scale, role)| (id.0, (asset, scale, role)))
                .collect();
            for balance in self
                .store
                .account_balances_at(&account.id, context.cutoff)?
            {
                let (asset_id, scale, role) = metadata
                    .get(&balance.position_id.0)
                    .context("ledger position has no registered asset/scale metadata")?;
                ensure!(
                    *role != PositionRole::ExternalCounterparty,
                    "counterparty in portfolio ledger"
                );
                positions.push(LedgerPositionDto {
                    position_id: balance.position_id,
                    account_id: account.id.clone(),
                    role: *role,
                    quantity: QuantityDto {
                        asset_id: asset_id.clone(),
                        units: balance.amount,
                        scale: *scale,
                    },
                });
            }
            accounts.push(AccountDto {
                id: account.id,
                provider_id: account.provider_id,
                label: account.label,
                source_reference: account.source_reference,
            });
        }
        let response = LedgerBalancesDto {
            contract_version: CONTRACT_VERSION,
            evidence: EvidenceDto::LedgerReplay { cutoff: context.cutoff },
            verification: VerificationDto::Unknown,
            issues: vec![IssueDto {
                code: "ledger_history_unverified".into(),
                scope: EntityScope::Portfolio { portfolio_id: context.portfolio_id.clone() },
                message: "Ledger replay does not establish independent balance verification or complete historical coverage".into(),
                withheld: vec![WithheldFigure::HistoricalValue, WithheldFigure::HistoricalEarnings],
            }],
            context, accounts, positions,
        };
        Ok(response)
    }
}

impl Application {
    pub fn ui_metadata(&self, portfolio: &lib_core::PortfolioId) -> Result<UiMetadataDto> {
        let revisions = self.store.revisions(portfolio)?;
        self.store.validate_account_tree(portfolio)?;
        let mut accounts = Vec::new();
        let mut provider_ids = std::collections::BTreeSet::new();
        let mut asset_ids = std::collections::BTreeSet::new();
        for account in self.store.accounts(portfolio)? {
            provider_ids.insert(account.provider_id.0.clone());
            for (_, asset, _, _) in self.store.account_position_metadata(&account.id)? {
                asset_ids.insert(asset.0);
            }
            let structure = self.store.account_structure(&account.id)?;
            let parent = self.store.account_parent(&account.id)?;
            accounts.push(UiAccountDto {
                account: AccountDto {
                    id: account.id,
                    provider_id: account.provider_id,
                    label: account.label,
                    source_reference: account.source_reference,
                },
                structure,
                parent,
            });
        }
        let subscriptions = self.store.subscriptions(portfolio)?;
        let product_ids: std::collections::BTreeSet<_> = subscriptions
            .iter()
            .map(|s| s.product_id.0.clone())
            .collect();
        let providers = provider_ids
            .into_iter()
            .map(|id| self.store.provider_display(&lib_core::ProviderId(id)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        let assets = asset_ids
            .into_iter()
            .map(|id| self.store.asset_display(&lib_core::AssetId::str(&id)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        let products = product_ids
            .into_iter()
            .map(|id| self.store.product_template(&id.into()))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(UiMetadataDto {
            contract_version: CONTRACT_VERSION,
            portfolio_id: portfolio.clone(),
            revisions,
            accounts,
            providers,
            assets,
            products,
            subscriptions,
            local_state: self.store.local_records(portfolio)?,
        })
    }
}
