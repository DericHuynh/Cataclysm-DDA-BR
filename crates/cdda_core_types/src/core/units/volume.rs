use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Volume measured in milliliters.
///
/// CDDA internally stores volumes in mL (1 mL = 0.001 L).
///
/// Accepts both a bare number (mL) and CDDA-style strings like `"250 ml"`, `"1 L"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Reflect)]
#[schemars(with = "String")]
pub struct Volume(pub u64);

impl Volume {
    pub const ZERO: Volume = Volume(0);

    pub const fn from_milliliters(m_l: u64) -> Self {
        Volume(m_l)
    }

    pub const fn from_liters(l: u64) -> Self {
        Volume(l * 1000)
    }

    pub fn as_milliliters(&self) -> u64 {
        self.0
    }

    pub fn as_liters(&self) -> f64 {
        self.0 as f64 / 1000.0
    }
}

// ---- Custom Deserialize: accepts "250 ml", "1 L", 250, etc. ----

impl<'de> Deserialize<'de> for Volume {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use a visitor that handles both strings and numbers
        struct VolumeVisitor;

        impl serde::de::Visitor<'_> for VolumeVisitor {
            type Value = Volume;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CDDA volume: number (mL) or string like \"250 ml\"")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Volume, E> {
                Ok(Volume::from_milliliters(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Volume, E> {
                if v < 0 {
                    Err(E::custom("volume cannot be negative"))
                } else {
                    Ok(Volume::from_milliliters(v as u64))
                }
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Volume, E> {
                if v < 0.0 {
                    Err(E::custom("volume cannot be negative"))
                } else {
                    Ok(Volume::from_milliliters(v as u64))
                }
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Volume, E> {
                parse_volume(s).ok_or_else(|| E::custom(format!("invalid volume: '{}'", s)))
            }
        }

        deserializer.deserialize_any(VolumeVisitor)
    }
}

/// Parse a CDDA volume string like "250 ml", "1 L", "500mL", "2.5L" into mL.
fn parse_volume(s: &str) -> Option<Volume> {
    let s = s.trim();

    // Split value and unit
    let (value_str, unit_part) = split_value_unit(s)?;
    let value: f64 = value_str.parse().ok()?;
    let unit = unit_part.to_lowercase();

    match unit.as_str() {
        "ml" | "milliliter" | "milliliters" => Some(Volume::from_milliliters(value as u64)),
        "l" | "liter" | "liters" => Some(Volume::from_milliliters((value * 1000.0) as u64)),
        _ => None,
    }
}

/// Split a string like "250 ml" into ("250", "ml").
/// Handles optional space between value and unit.
fn split_value_unit(s: &str) -> Option<(&str, &str)> {
    let s = s.trim();
    // Find where the numeric part ends
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

impl Add for Volume {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Volume(self.0 + rhs.0)
    }
}

impl Sub for Volume {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Volume(self.0.saturating_sub(rhs.0))
    }
}

impl PartialOrd for Volume {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Volume {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

// ---- Display ----

impl std::fmt::Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 1000 {
            write!(f, "{} L", self.0 as f64 / 1000.0)
        } else {
            write!(f, "{} ml", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_arithmetic() {
        let v1 = Volume::from_liters(1);
        let v2 = Volume::from_milliliters(500);
        assert_eq!((v1 + v2).as_milliliters(), 1500);
        assert_eq!((v1 - v2).as_milliliters(), 500);
    }

    #[test]
    fn test_volume_saturating_sub() {
        let small = Volume::from_milliliters(100);
        let large = Volume::from_liters(1);
        assert_eq!((small - large), Volume::ZERO);
    }

    #[test]
    fn test_parse_volume_strings() {
        assert_eq!(parse_volume("250 ml"), Some(Volume(250)));
        assert_eq!(parse_volume("1 L"), Some(Volume(1000)));
        assert_eq!(parse_volume("500mL"), Some(Volume(500)));
        assert_eq!(parse_volume("2.5 L"), Some(Volume(2500)));
    }

    #[test]
    fn test_deserialize_volume() {
        let v: Volume = serde_json::from_str("250").unwrap();
        assert_eq!(v, Volume(250));

        let v: Volume = serde_json::from_str(r#""250 ml""#).unwrap();
        assert_eq!(v, Volume(250));

        let v: Volume = serde_json::from_str(r#""1 L""#).unwrap();
        assert_eq!(v, Volume(1000));
    }

    #[test]
    fn test_split_value_unit() {
        assert_eq!(split_value_unit("250 ml"), Some(("250", "ml")));
        assert_eq!(split_value_unit("1 L"), Some(("1", "L")));
        assert_eq!(split_value_unit("500mL"), Some(("500", "mL")));
        assert_eq!(split_value_unit("2.5 L"), Some(("2.5", "L")));
    }
}
