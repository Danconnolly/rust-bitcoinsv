use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::iter::Sum;
use std::ops::{Add, Sub};

/// An Amount of BSV.
#[derive(Debug, Clone, PartialEq)]
pub struct Amount {
    pub satoshis: i64,
}

impl Amount {
    /// The zero amount.
    pub const ZERO: Amount = Amount::from_satoshis(0);
    /// Exactly one satoshi.
    pub const ONE_SAT: Amount = Amount::from_satoshis(1);
    /// Exactly one bitcoin.
    pub const ONE_BSV: Amount = Amount::from_satoshis(100_000_000);

    pub const fn from_satoshis(satoshis: i64) -> Self {
        Amount { satoshis }
    }

    /// Convert to a float, using 1BSV = 10^8 satoshis. Dont use this in calculations.
    pub fn as_bsv_f64(&self) -> f64 {
        self.satoshis as f64 / 100_000_000.0
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bsv = self.as_bsv_f64();
        let mut s = format!("{:.8}", bsv);
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.push('0');
        }
        f.write_str(&s)
    }
}

impl Serialize for Amount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        f64::serialize(&self.as_bsv_f64(), serializer)
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bsv = f64::deserialize(deserializer)?;
        Ok(Amount::from_satoshis((bsv * 100_000_000.0) as i64))
    }
}

impl Default for Amount {
    fn default() -> Self {
        Amount::ZERO
    }
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, other: Amount) -> Amount {
        Amount {
            satoshis: self.satoshis + other.satoshis,
        }
    }
}

impl Sub for Amount {
    type Output = Amount;

    fn sub(self, other: Amount) -> Amount {
        Amount {
            satoshis: self.satoshis - other.satoshis,
        }
    }
}

impl Sum for Amount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_serialize_amount() {
        let amount = Amount::from_satoshis(100_000_000);
        let json = serde_json::to_string(&amount).expect("Failed to serialize amount");
        assert_eq!(json, "1.0");
    }

    #[test]
    fn json_deserialize_amount() {
        let json = "1.0";
        let amount: Amount = serde_json::from_str(json).expect("Failed to deserialize amount");
        assert_eq!(amount, Amount::from_satoshis(100_000_000));
    }
}
