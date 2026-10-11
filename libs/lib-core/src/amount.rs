//! Exact smallest-unit quantities. Floating point is only used at valuation boundaries.
use anyhow::{Result, ensure};
use num_bigint::BigInt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, ops, str::FromStr};

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Units(BigInt);

impl Units {
    pub fn is_zero(&self) -> bool {
        self.0 == BigInt::from(0)
    }
    pub fn is_negative(&self) -> bool {
        self.0 < BigInt::from(0)
    }
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            -self.clone()
        } else {
            self.clone()
        }
    }

    /// Parse a plain decimal without rounding, preserving the selected ledger scale.
    pub fn from_decimal(value: &str, scale: u8) -> Result<Self> {
        ensure!(value.len() <= 4096, "amount exceeds input length limit");
        let (negative, unsigned) = match value.strip_prefix('-') {
            Some(v) => (true, v),
            None => (false, value.strip_prefix('+').unwrap_or(value)),
        };
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        ensure!(
            !whole.is_empty()
                && whole.bytes().all(|b| b.is_ascii_digit())
                && fraction.bytes().all(|b| b.is_ascii_digit()),
            "invalid decimal amount {value}"
        );
        let scale = usize::from(scale);
        ensure!(
            fraction
                .get(scale..)
                .unwrap_or("")
                .bytes()
                .all(|b| b == b'0'),
            "amount loses precision at scale {scale}: {value}"
        );
        let fraction = &fraction[..fraction.len().min(scale)];
        let text = format!(
            "{}{}{}{}",
            if negative { "-" } else { "" },
            whole,
            fraction,
            "0".repeat(scale - fraction.len())
        );
        text.parse()
    }

    pub fn decimal_string(&self, scale: u8) -> String {
        let raw = self.abs().to_string();
        let scale = usize::from(scale);
        let digits = format!(
            "{}{}",
            "0".repeat((scale + 1).saturating_sub(raw.len())),
            raw
        );
        let sign = if self.is_negative() { "-" } else { "" };
        if scale == 0 {
            format!("{sign}{digits}")
        } else {
            let split = digits.len() - scale;
            format!("{sign}{}.{}", &digits[..split], &digits[split..])
        }
    }

    pub fn rescale(&self, old: u8, new: u8) -> Result<Self> {
        Self::from_decimal(&self.decimal_string(old), new)
    }

    pub fn approximate(&self, scale: u8) -> Result<f64> {
        let value: f64 = self.decimal_string(scale).parse()?;
        ensure!(
            value.is_finite(),
            "quantity exceeds approximate valuation range"
        );
        Ok(value)
    }
}

impl FromStr for Units {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        ensure!(value.len() <= 4096, "amount exceeds input length limit");
        let digits = value.strip_prefix('-').unwrap_or(value);
        ensure!(
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()),
            "invalid integer units"
        );
        Ok(Self(value.parse()?))
    }
}
impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Serialize for Units {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Units {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
macro_rules! integer {
    ($($t:ty),*) => {$(
        impl From<$t> for Units { fn from(v: $t) -> Self { Self(BigInt::from(v)) } }
        impl PartialEq<$t> for Units { fn eq(&self, v: &$t) -> bool { self.0 == BigInt::from(*v) } }
    )*};
}
integer!(i32, i64, i128, u32, u64, u128);
impl ops::AddAssign<&Units> for Units {
    fn add_assign(&mut self, rhs: &Units) {
        self.0 += &rhs.0;
    }
}
impl ops::AddAssign<Units> for Units {
    fn add_assign(&mut self, rhs: Units) {
        self.0 += rhs.0;
    }
}
impl ops::SubAssign<&Units> for Units {
    fn sub_assign(&mut self, rhs: &Units) {
        self.0 -= &rhs.0;
    }
}
impl ops::SubAssign<Units> for Units {
    fn sub_assign(&mut self, rhs: Units) {
        self.0 -= rhs.0;
    }
}
impl ops::Neg for Units {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}
impl ops::Sub for Units {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}
impl ops::Mul<i32> for &Units {
    type Output = Units;
    fn mul(self, rhs: i32) -> Units {
        Units(&self.0 * rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_decimal_conversion_and_rescaling() -> Result<()> {
        let n = Units::from_decimal("123456789012345678901234567890.000000000000000001", 18)?;
        assert_eq!(
            n.decimal_string(18),
            "123456789012345678901234567890.000000000000000001"
        );
        assert!(n.rescale(18, 8).is_err());
        assert!(Units::from_decimal("0.000000001", 8).is_err());
        assert_eq!(
            Units::from_decimal("-1.20", 8)?.decimal_string(2),
            "-1200000.00"
        );
        assert_eq!(Units::from_decimal("1.200", 2)?, 120);
        assert!(Units::from_decimal("NaN", 8).is_err());
        assert_eq!(
            serde_json::from_str::<Units>("\"123456789012345678901234567890\"")?.to_string(),
            "123456789012345678901234567890"
        );
        Ok(())
    }
}
