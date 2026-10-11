//! Version 1 JSON contracts. Financial quantities/rates are strings, never floats.
use anyhow::Result;
use chrono::{DateTime, Utc};
use lib_core::{AccountId, AssetId, PortfolioId, PositionId, PositionRole, ProviderId, Units};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

pub const CONTRACT_VERSION: u32 = 1;

/// Validated plain decimal, serialized as a canonical string. No exponents,
/// NaN, rounding or fixed-width integer/decimal range restriction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactDecimal(String);

impl ExactDecimal {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl FromStr for ExactDecimal {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        let scale = value.split_once('.').map_or(0, |(_, f)| f.len());
        let scale = u8::try_from(scale)?;
        let units = Units::from_decimal(value, scale)?;
        let mut canonical = units.decimal_string(scale);
        if scale > 0 {
            canonical = canonical
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_owned();
        }
        Ok(Self(canonical))
    }
}
impl fmt::Display for ExactDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Serialize for ExactDecimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for ExactDecimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

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
/// These are transport contracts, not persisted rates or an earnings engine.
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
