use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Energy measured in Joules.
///
/// Used for bionics, vehicle power, and other energy systems.
///
/// # Deserialization
///
/// Accepts both a bare number (interpreted as Joules) and CDDA-style
/// human-readable strings like `"1 kJ"`, `"500 J"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(with = "String")]
pub struct Energy(pub u64);

impl Energy {
    pub const ZERO: Energy = Energy(0);

    pub const fn from_joules(j: u64) -> Self {
        Energy(j)
    }

    pub fn as_joules(&self) -> u64 {
        self.0
    }
}

// ---- Custom Deserialize: accepts "1 kJ", "500 J", 1000, etc. ----

impl<'de> Deserialize<'de> for Energy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnergyVisitor;

        impl serde::de::Visitor<'_> for EnergyVisitor {
            type Value = Energy;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CDDA energy: number (Joules) or string like \"1 kJ\"")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Energy, E> {
                Ok(Energy::from_joules(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Energy, E> {
                if v < 0 {
                    Err(E::custom("energy cannot be negative"))
                } else {
                    Ok(Energy::from_joules(v as u64))
                }
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Energy, E> {
                if v < 0.0 {
                    Err(E::custom("energy cannot be negative"))
                } else {
                    Ok(Energy::from_joules(v as u64))
                }
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Energy, E> {
                parse_energy(s).ok_or_else(|| E::custom(format!("invalid energy: '{}'", s)))
            }
        }

        deserializer.deserialize_any(EnergyVisitor)
    }
}

/// Parse a CDDA energy string like "1 kJ", "500 J", "1000J" into Joules.
fn parse_energy(s: &str) -> Option<Energy> {
    let s = s.trim();
    let (value_str, unit_part) = split_value_unit(s)?;
    let value: f64 = value_str.parse().ok()?;
    let unit = unit_part.to_lowercase();

    match unit.as_str() {
        "j" | "joule" | "joules" => Some(Energy::from_joules(value as u64)),
        "kj" | "kilojoule" | "kilojoules" => Some(Energy::from_joules((value * 1000.0) as u64)),
        _ => None,
    }
}

/// Split a string like "1 kJ" into ("1", "kJ").
fn split_value_unit(s: &str) -> Option<(&str, &str)> {
    let s = s.trim();
    let mut split_idx = s.len();
    for (i, c) in s.char_indices() {
        if c == ' ' {
            split_idx = i;
            break;
        }
        if !c.is_ascii_digit() && c != '.' && c != '-' && c != '+' {
            split_idx = i;
            break;
        }
    }

    if split_idx == 0 || split_idx == s.len() {
        return None;
    }

    let value_str = s[..split_idx].trim();
    let unit_str = s[split_idx..].trim();
    if value_str.is_empty() || unit_str.is_empty() {
        return None;
    }
    Some((value_str, unit_str))
}

// ---- Arithmetic ----

impl Add for Energy {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Energy(self.0 + rhs.0)
    }
}

impl Sub for Energy {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Energy(self.0.saturating_sub(rhs.0))
    }
}

impl PartialOrd for Energy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Energy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_energy_strings() {
        assert_eq!(parse_energy("500 J"), Some(Energy(500)));
        assert_eq!(parse_energy("1 kJ"), Some(Energy(1000)));
        assert_eq!(parse_energy("2.5 kJ"), Some(Energy(2500)));
    }

    #[test]
    fn test_deserialize_energy() {
        let e: Energy = serde_json::from_str("1000").unwrap();
        assert_eq!(e, Energy(1000));

        let e: Energy = serde_json::from_str(r#""1 kJ""#).unwrap();
        assert_eq!(e, Energy(1000));

        let e: Energy = serde_json::from_str(r#""500 J""#).unwrap();
        assert_eq!(e, Energy(500));
    }
}
