//! Version 1 JSON contracts. Financial quantities/rates are strings, never floats.
use chrono::{DateTime, Utc};
use lib_core::{AccountId, AssetId, PortfolioId, PositionId, PositionRole, ProviderId, Units};
use serde::{Deserialize, Serialize};

pub const CONTRACT_VERSION: u32 = 1;

pub use lib_core::ExactDecimal;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadContext {
    pub portfolio_id: PortfolioId,
    pub cutoff: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantityDto {
    pub asset_id: AssetId,
    pub units: Units,
    pub scale: u8,
}
impl QuantityDto {
    pub fn decimal_string(&self) -> String {
        self.units.decimal_string(self.scale)
    }
}

/// An exact economic value in a specified denomination; no UI rounding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoneyDto {
    pub asset_id: AssetId,
    pub amount: ExactDecimal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntityScope {
    Portfolio { portfolio_id: PortfolioId },
    Account { account_id: AccountId },
    Position { position_id: PositionId },
    Asset { asset_id: AssetId },
    Provider { provider_id: ProviderId },
    Product { product_id: lib_core::ProductId },
    Subscription { subscription_id: String },
}

/// Evidence distinguishes computed ledger quantities from independent observations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceDto {
    LedgerReplay {
        cutoff: DateTime<Utc>,
    },
    Provider {
        source: String,
        reference: String,
        observed_at: DateTime<Utc>,
        effective_from: Option<DateTime<Utc>>,
    },
    Manual {
        action_id: String,
        updated_at: DateTime<Utc>,
        effective_from: DateTime<Utc>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WithheldFigure {
    HistoricalValue,
    HistoricalEarnings,
    CurrentValue,
    CurrentEarnings,
    OverallApy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueDto {
    pub code: String,
    pub scope: EntityScope,
    pub message: String,
    pub withheld: Vec<WithheldFigure>,
}

/// Partial is an explicitly incomplete known subtotal. Unknown has no value,
/// whereas known zero is represented by a real exact zero value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FigureDto<T> {
    Known {
        value: T,
        evidence: Vec<EvidenceDto>,
    },
    Partial {
        value: T,
        evidence: Vec<EvidenceDto>,
        issues: Vec<IssueDto>,
    },
    Unknown {
        issues: Vec<IssueDto>,
    },
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateBasis {
    Apr,
    Apy,
}

/// Annual fraction: 5% is "0.05". Compounding conventions remain explicit.
/// Legacy transport vocabulary; portfolio projections expose selected persisted evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateDto {
    pub annual_fraction: ExactDecimal,
    pub basis: RateBasis,
    pub compounding: Option<String>,
    pub reward_asset_id: Option<AssetId>,
    pub effective_from: DateTime<Utc>,
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VerificationDto {
    Unknown,
    VerifiedAtCutoff {
        cutoff: DateTime<Utc>,
        source: String,
        reference: String,
    },
    Mismatch {
        cutoff: DateTime<Utc>,
        issues: Vec<IssueDto>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountDto {
    pub id: AccountId,
    pub provider_id: ProviderId,
    pub label: String,
    pub source_reference: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerPositionDto {
    pub position_id: PositionId,
    pub account_id: AccountId,
    pub role: PositionRole,
    /// Signed quantities retain liability ledger signs. Not a positive debt magnitude.
    pub quantity: QuantityDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerBalancesDto {
    pub contract_version: u32,
    pub context: ReadContext,
    pub accounts: Vec<AccountDto>,
    pub positions: Vec<LedgerPositionDto>,
    pub evidence: EvidenceDto,
    pub verification: VerificationDto,
    pub issues: Vec<IssueDto>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn exact_wire_values_reject_numbers_and_lossy_formats() -> Result<()> {
        let value: ExactDecimal = "123456789012345678901234567890.000000000000000001".parse()?;
        let json = serde_json::to_string(&value)?;
        assert_eq!(
            json,
            "\"123456789012345678901234567890.000000000000000001\""
        );
        assert_eq!(serde_json::from_str::<ExactDecimal>(&json)?, value);
        for invalid in ["1e3", "NaN", "Infinity", "", ".5", " 1", "1.2.3"] {
            assert!(invalid.parse::<ExactDecimal>().is_err());
        }
        assert!(serde_json::from_str::<ExactDecimal>("0.05").is_err());
        assert!(serde_json::from_str::<Units>("9007199254740993").is_err());
        assert_eq!("-0.00".parse::<ExactDecimal>()?.as_str(), "0");
        assert_eq!("+001.2300".parse::<ExactDecimal>()?.as_str(), "1.23");
        let unknown: FigureDto<MoneyDto> = FigureDto::Unknown { issues: vec![] };
        let zero = FigureDto::Known {
            value: MoneyDto {
                asset_id: AssetId::str("USD"),
                amount: "0".parse()?,
            },
            evidence: vec![],
        };
        assert_ne!(serde_json::to_value(unknown)?, serde_json::to_value(zero)?);
        Ok(())
    }
}

/// Stage-3 metadata only; this is not the current-value/annual-earnings projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAccountDto {
    pub account: AccountDto,
    pub structure: lib_core::ui_metadata::AccountStructure,
    pub parent: Option<AccountId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMetadataDto {
    pub contract_version: u32,
    pub portfolio_id: PortfolioId,
    pub revisions: lib_core::ui_metadata::PortfolioRevisions,
    pub accounts: Vec<UiAccountDto>,
    pub providers: Vec<lib_core::ui_metadata::ProviderDisplay>,
    pub assets: Vec<lib_core::ui_metadata::AssetDisplay>,
    pub products: Vec<lib_core::ui_metadata::ProductTemplate>,
    pub subscriptions: Vec<lib_core::ui_metadata::Subscription>,
    pub local_state: Vec<(
        lib_core::ui_metadata::LocalTarget,
        lib_core::ui_metadata::LocalRecord,
    )>,
}
