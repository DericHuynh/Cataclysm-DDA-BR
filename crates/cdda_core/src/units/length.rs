use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Length measured in millimeters.
///
/// Accepts both a bare number (mm) and CDDA-style strings like `"250 mm"`, `"10 cm"`, `"1 m"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(with = "String")]
pub struct Length(pub u32);

impl Length {
    pub const ZERO: Length = Length(0);

    pub const fn from_millimeters(mm: u32) -> Self {
        Length(mm)
    }

    pub const fn from_centimeters(cm: u32) -> Self {
        Length(cm * 10)
    }

    pub const fn from_meters(m: u32) -> Self {
        Length(m * 1000)
    }

    pub fn as_millimeters(&self) -> u32 {
        self.0
    }

    pub fn as_centimeters(&self) -> f64 {
        self.0 as f64 / 10.0
    }

    pub fn as_meters(&self) -> f64 {
        self.0 as f64 / 1000.0
    }
}

// ---- Custom Deserialize: accepts "250 mm", "10 cm", "1 m", 100, etc. ----

impl<'de> Deserialize<'de> for Length {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LengthVisitor;

        impl serde::de::Visitor<'_> for LengthVisitor {
            type Value = Length;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a CDDA length: number (mm) or string like \"250 mm\"")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Length, E> {
                let v32 = if v > u64::from(u32::MAX) {
                    return Err(E::custom("length value out of range"));
                } else {
                    v as u32
                };
                Ok(Length::from_millimeters(v32))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Length, E> {
                if v < 0 {
                    Err(E::custom("length cannot be negative"))
                } else {
                    self.visit_u64(v as u64)
                }
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Length, E> {
                if v < 0.0 {
                    Err(E::custom("length cannot be negative"))
                } else {
                    self.visit_u64(v as u64)
                }
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Length, E> {
                parse_length(s).ok_or_else(|| E::custom(format!("invalid length: '{}'", s)))
            }
        }

        deserializer.deserialize_any(LengthVisitor)
    }
}

/// Parse a CDDA length string like "250 mm", "10 cm", "1 m", "60 meter" into mm.
fn parse_length(s: &str) -> Option<Length> {
    let s = s.trim();
    let (value_str, unit_part) = split_value_unit(s)?;
    let value: f64 = value_str.parse().ok()?;
    let unit = unit_part.to_lowercase();

    match unit.as_str() {
        "mm" | "millimeter" | "millimeters" => Some(Length::from_millimeters(value as u32)),
        "cm" | "centimeter" | "centimeters" => {
            Some(Length::from_millimeters((value * 10.0) as u32))
        }
        "m" | "meter" | "meters" => Some(Length::from_millimeters((value * 1000.0) as u32)),
        "km" | "kilometer" | "kilometers" => {
            Some(Length::from_millimeters((value * 1_000_000.0) as u32))
        }
        _ => None,
    }
}

/// Split a string like "250 mm" into ("250", "mm").
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

impl Add for Length {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Length(self.0 + rhs.0)
    }
}

impl Sub for Length {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Length(self.0.saturating_sub(rhs.0))
    }
}

impl PartialOrd for Length {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Length {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

// ---- Display ----

impl std::fmt::Display for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 1000 {
            write!(f, "{} m", self.0 as f64 / 1000.0)
        } else if self.0 >= 10 {
            write!(f, "{} cm", self.0 as f64 / 10.0)
        } else {
            write!(f, "{} mm", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_length_strings() {
        assert_eq!(parse_length("250 mm"), Some(Length(250)));
        assert_eq!(parse_length("10 cm"), Some(Length(100)));
        assert_eq!(parse_length("1 m"), Some(Length(1000)));
        assert_eq!(parse_length("60 meter"), Some(Length(60000)));
    }

    #[test]
    fn test_deserialize_length() {
        let l: Length = serde_json::from_str("250").unwrap();
        assert_eq!(l, Length(250));

        let l: Length = serde_json::from_str(r#""250 mm""#).unwrap();
        assert_eq!(l, Length(250));

        let l: Length = serde_json::from_str(r#""10 cm""#).unwrap();
        assert_eq!(l, Length(100));

        let l: Length = serde_json::from_str(r#""1 m""#).unwrap();
        assert_eq!(l, Length(1000));
    }
}
