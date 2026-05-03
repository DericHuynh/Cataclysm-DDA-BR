use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Weight measured in grams.
///
/// CDDA internally stores weights in grams.
///
/// # Deserialization
///
/// Accepts both a bare number (interpreted as grams) and CDDA-style
/// human-readable strings like `"100 g"`, `"1 kg"`, `"500g"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Weight(pub u64);

impl Weight {
    pub const ZERO: Weight = Weight(0);

    pub const fn from_grams(g: u64) -> Self {
        Weight(g)
    }

    pub const fn from_kilograms(kg: u64) -> Self {
        Weight(kg * 1000)
    }

    pub fn as_grams(&self) -> u64 {
        self.0
    }

    pub fn as_kilograms(&self) -> f64 {
        self.0 as f64 / 1000.0
    }
}

// ---- Custom Deserialize: accepts "100 g", "1 kg", 500, etc. ----

impl<'de> Deserialize<'de> for Weight {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct WeightVisitor;

        impl serde::de::Visitor<'_> for WeightVisitor {
            type Value = Weight;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CDDA weight: number (grams) or string like \"100 g\"")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Weight, E> {
                Ok(Weight::from_grams(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Weight, E> {
                if v < 0 {
                    Err(E::custom("weight cannot be negative"))
                } else {
                    Ok(Weight::from_grams(v as u64))
                }
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Weight, E> {
                if v < 0.0 {
                    Err(E::custom("weight cannot be negative"))
                } else {
                    Ok(Weight::from_grams(v as u64))
                }
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Weight, E> {
                parse_weight(s).ok_or_else(|| E::custom(format!("invalid weight: '{}'", s)))
            }
        }

        deserializer.deserialize_any(WeightVisitor)
    }
}

/// Parse a CDDA weight string like "100 g", "1 kg", "500g" into grams.
fn parse_weight(s: &str) -> Option<Weight> {
    let s = s.trim();
    let (value_str, unit_part) = split_value_unit(s)?;
    let value: f64 = value_str.parse().ok()?;
    let unit = unit_part.to_lowercase();

    match unit.as_str() {
        "g" | "gram" | "grams" => Some(Weight::from_grams(value as u64)),
        "kg" | "kilogram" | "kilograms" => Some(Weight::from_kilograms(value as u64)),
        "mg" | "milligram" | "milligrams" => Some(Weight::from_grams((value / 1000.0) as u64)),
        _ => None,
    }
}

/// Split a string like "100 g" into ("100", "g").
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

impl Add for Weight {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Weight(self.0 + rhs.0)
    }
}

impl Sub for Weight {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Weight(self.0.saturating_sub(rhs.0))
    }
}

impl PartialOrd for Weight {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Weight {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

// ---- Display ----

impl std::fmt::Display for Weight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 1000 {
            write!(f, "{} kg", self.0 as f64 / 1000.0)
        } else {
            write!(f, "{} g", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weight_arithmetic() {
        let w1 = Weight::from_kilograms(2);
        let w2 = Weight::from_grams(500);
        assert_eq!((w1 + w2).as_grams(), 2500);
    }

    #[test]
    fn test_parse_weight_strings() {
        assert_eq!(parse_weight("100 g"), Some(Weight(100)));
        assert_eq!(parse_weight("1 kg"), Some(Weight(1000)));
        assert_eq!(parse_weight("500g"), Some(Weight(500)));
        assert_eq!(parse_weight("2.5 kg"), Some(Weight(2500)));
    }

    #[test]
    fn test_deserialize_weight() {
        let w: Weight = serde_json::from_str("500").unwrap();
        assert_eq!(w, Weight(500));

        let w: Weight = serde_json::from_str(r#""100 g""#).unwrap();
        assert_eq!(w, Weight(100));

        let w: Weight = serde_json::from_str(r#""1 kg""#).unwrap();
        assert_eq!(w, Weight(1000));
    }
}
