//! Exact rational arithmetic for calculated money and non-terminating rates.
use crate::dto::ExactDecimal;
use anyhow::{Result, ensure};
use num_bigint::BigInt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{cmp::Ordering, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactRatio {
    n: BigInt,
    d: BigInt,
}
#[derive(Serialize, Deserialize)]
struct Wire {
    numerator: String,
    denominator: String,
}
impl ExactRatio {
    fn new(mut n: BigInt, mut d: BigInt) -> Self {
        assert!(d != BigInt::from(0));
        if d < BigInt::from(0) {
            n = -n;
            d = -d;
        }
        let mut a = if n < BigInt::from(0) {
            -n.clone()
        } else {
            n.clone()
        };
        let mut b = d.clone();
        while b != BigInt::from(0) {
            let r = &a % &b;
            a = b;
            b = r;
        }
        Self {
            n: n / &a,
            d: d / a,
        }
    }
    pub fn zero() -> Self {
        Self::integer(0)
    }
    pub fn integer(n: i32) -> Self {
        Self::new(n.into(), 1.into())
    }
    pub fn from_decimal(v: &ExactDecimal) -> Self {
        let s = v.as_str();
        let scale = s.split_once('.').map_or(0, |(_, f)| f.len());
        Self::new(
            BigInt::from_str(&s.replace('.', "")).expect("validated exact decimal"),
            BigInt::from(10).pow(scale as u32),
        )
    }
    pub fn quantity(v: &lib_core::Units, scale: u8) -> Self {
        Self::new(
            v.to_string().parse().expect("validated units"),
            BigInt::from(10).pow(u32::from(scale)),
        )
    }
    pub fn add(&self, b: &Self) -> Self {
        Self::new(&self.n * &b.d + &b.n * &self.d, &self.d * &b.d)
    }
    pub fn sub(&self, b: &Self) -> Self {
        self.add(&b.neg())
    }
    pub fn mul(&self, b: &Self) -> Self {
        Self::new(&self.n * &b.n, &self.d * &b.d)
    }
    pub fn div(&self, b: &Self) -> Option<Self> {
        (!b.is_zero()).then(|| Self::new(&self.n * &b.d, &self.d * &b.n))
    }
    pub fn neg(&self) -> Self {
        Self {
            n: -self.n.clone(),
            d: self.d.clone(),
        }
    }
    pub fn abs(&self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            self.clone()
        }
    }
    pub fn is_zero(&self) -> bool {
        self.n == BigInt::from(0)
    }
    pub fn is_negative(&self) -> bool {
        self.n < BigInt::from(0)
    }
    pub fn pow(&self, n: u32) -> Self {
        Self::new(self.n.pow(n), self.d.pow(n))
    }
    pub fn numerator(&self) -> String {
        self.n.to_string()
    }
    pub fn denominator(&self) -> String {
        self.d.to_string()
    }
}
impl Ord for ExactRatio {
    fn cmp(&self, b: &Self) -> Ordering {
        (&self.n * &b.d).cmp(&(&b.n * &self.d))
    }
}
impl PartialOrd for ExactRatio {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Serialize for ExactRatio {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        Wire {
            numerator: self.numerator(),
            denominator: self.denominator(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ExactRatio {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let w = Wire::deserialize(d)?;
        let parse = || -> Result<Self> {
            ensure!(
                w.numerator.len() <= 8_000_000 && w.denominator.len() <= 8_000_000,
                "ratio exceeds limit"
            );
            ensure!(
                w.numerator
                    .trim_start_matches('-')
                    .bytes()
                    .all(|b| b.is_ascii_digit())
                    && w.denominator.bytes().all(|b| b.is_ascii_digit()),
                "integer string required"
            );
            let n: BigInt = w.numerator.parse()?;
            let d: BigInt = w.denominator.parse()?;
            ensure!(d > BigInt::from(0), "denominator must be positive");
            Ok(Self::new(n, d))
        };
        parse().map_err(serde::de::Error::custom)
    }
}
