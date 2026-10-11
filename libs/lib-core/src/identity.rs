//! Stable portfolio/account identity, separate from display labels.
use crate::{AssetId, ProviderId};
use serde::{Deserialize, Serialize};

macro_rules! id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl<T: AsRef<str>> From<T> for $name {
            fn from(value: T) -> Self {
                Self(value.as_ref().to_owned())
            }
        }
    };
}
id!(PortfolioId);
id!(AccountId);

pub const DEFAULT_PORTFOLIO: &str = "local:default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Portfolio {
    pub id: PortfolioId,
    pub label: String,
    pub reporting_asset: AssetId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: AccountId,
    pub portfolio_id: PortfolioId,
    pub provider_id: ProviderId,
    pub label: String,
    pub source_reference: Option<String>,
}

/// Length-independent, collision-safe namespace; labels and credentials never enter IDs.
pub(crate) fn scoped_id(account: &AccountId, namespace: &str, reference: &str) -> String {
    format!(
        "account:{}",
        serde_json::to_string(&(&account.0, namespace, reference)).expect("strings serialize")
    )
}

pub(crate) fn legacy_account(position: &str) -> Account {
    let (id, provider, label) =
        if position.starts_with("NEXO:") || position.starts_with("NEXO-CREDIT:") {
            (
                "legacy:nexo".to_owned(),
                "nexo",
                "Legacy Nexo aggregate account".to_owned(),
            )
        } else if let Some(rest) = position.strip_prefix("BINANCE:") {
            let bucket = rest.split(':').next().unwrap_or("unknown");
            (
                format!("legacy:binance:{bucket}"),
                "binance",
                format!("Legacy Binance {bucket} account"),
            )
        } else {
            (
                "legacy:unassigned".to_owned(),
                "unknown",
                "Legacy account — ownership needs review".to_owned(),
            )
        };
    Account {
        id: AccountId(id),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        provider_id: ProviderId::from(provider),
        label,
        source_reference: None,
    }
}
