//! Exact plain decimals shared by persisted metadata and application DTOs.
use crate::Units;
use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactDecimal(String);
impl ExactDecimal {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn is_negative(&self) -> bool {
        self.0.starts_with('-')
    }
}
impl FromStr for ExactDecimal {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        let scale = u8::try_from(value.split_once('.').map_or(0, |(_, f)| f.len()))?;
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
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for ExactDecimal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
