use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Game time measured in turns. 1 turn ≈ 1 second.
///
/// Accepts both bare numbers (turns) and CDDA-style strings like
/// `"1 h"`, `"30 m"`, `"26 h 12 m"`, `"744m"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Reflect)]
#[schemars(with = "String")]
pub struct Time(pub i64);

impl Time {
    pub const ZERO: Time = Time(0);

    pub const fn from_turns(turns: i64) -> Self {
        Time(turns)
    }

    pub fn as_turns(&self) -> i64 {
        self.0
    }
}

// ---- Custom Deserialize: accepts 3600, "1 h", "30 m", "26 h 12 m", "744m" ----

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TimeVisitor;

        impl serde::de::Visitor<'_> for TimeVisitor {
            type Value = Time;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("a CDDA time: number (turns) or string like \"1 h\", \"30 m\"")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Time, E> {
                Ok(Time::from_turns(v as i64))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Time, E> {
                Ok(Time::from_turns(v))
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Time, E> {
                Ok(Time::from_turns(v as i64))
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Time, E> {
                Ok(parse_time(s).unwrap_or(Time::ZERO))
            }
        }

        deserializer.deserialize_any(TimeVisitor)
    }
}

/// Parse a CDDA time string into turns (1 hour = 3600 turns, 1 minute = 60 turns).
///
/// Supports formats like:
/// - `"1 h"` / `"1 hour"`        → 3600 turns
/// - `"30 m"` / `"30 minute"`    → 1800 turns
/// - `"26 h 12 m"`               → 94320 turns
/// - `"744m"`                    → 44640 turns
/// - Bare number `"3600"`        → 3600 turns
fn parse_time(s: &str) -> Option<Time> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.split_whitespace().collect();

    // Single token: bare number or compact unit like "744m"
    if parts.len() == 1 {
        let token = parts[0];
        // Try compact format first (e.g. "744m")
        if let Some((val_str, unit)) = split_value_unit(token) {
            let val: i64 = val_str.parse().ok()?;
            return turns_from_value_unit(val, unit);
        }
        // Otherwise try bare number
        return token.parse::<i64>().ok().map(Time);
    }

    // Multiple tokens: process value-unit pairs like "26 h 12 m"
    let mut total_turns: i64 = 0;
    let mut i = 0;
    while i < parts.len() {
        if i + 1 < parts.len() {
            if let Ok(val) = parts[i].parse::<i64>() {
                let unit = parts[i + 1].to_lowercase();
                if let Some(t) = turns_from_value_unit(val, &unit) {
                    total_turns += t.0;
                    i += 2;
                    continue;
                }
            }
        }
        // Try compact format on remaining unpaired token
        if let Some((val_str, unit)) = split_value_unit(parts[i]) {
            if let Ok(val) = val_str.parse::<i64>() {
                if let Some(t) = turns_from_value_unit(val, unit) {
                    total_turns += t.0;
                    i += 1;
                    continue;
                }
            }
        }
        return None;
    }

    Some(Time(total_turns))
}

/// Convert a numeric value and a time-unit suffix into `Time` (turns).
fn turns_from_value_unit(val: i64, unit: &str) -> Option<Time> {
    match unit.to_lowercase().as_str() {
        "h" | "hour" | "hours" => Some(Time(val * 3600)),
        "m" | "minute" | "minutes" => Some(Time(val * 60)),
        "s" | "second" | "seconds" => Some(Time(val)),
        _ => None,
    }
}

/// Split a compact token like `"744m"` into `("744", "m")`.
///
/// Handles optional space between the numeric value and the unit suffix.
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

impl Add for Time {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Time(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Time(self.0 - rhs.0)
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

// ---- Display ----

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total = self.0;
        if total == 0 {
            return write!(f, "0 m");
        }

        let hours = total / 3600;
        let minutes = (total % 3600) / 60;
        let seconds = total % 60;

        let mut parts = Vec::new();
        if hours > 0 {
            parts.push(format!("{} h", hours));
        }
        if minutes > 0 {
            parts.push(format!("{} m", minutes));
        }
        if seconds > 0 || (hours == 0 && minutes == 0) {
            parts.push(format!("{} s", seconds));
        }

        write!(f, "{}", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_arithmetic() {
        let t1 = Time::from_turns(3600);
        let t2 = Time::from_turns(600);
        assert_eq!((t1 + t2).as_turns(), 4200);
        assert_eq!((t1 - t2).as_turns(), 3000);
    }

    #[test]
    fn test_parse_time_hours() {
        assert_eq!(parse_time("1 h"), Some(Time(3600)));
        assert_eq!(parse_time("1 hour"), Some(Time(3600)));
        assert_eq!(parse_time("2 hours"), Some(Time(7200)));
    }

    #[test]
    fn test_parse_time_minutes() {
        assert_eq!(parse_time("30 m"), Some(Time(1800)));
        assert_eq!(parse_time("30 minute"), Some(Time(1800)));
        assert_eq!(parse_time("45 minutes"), Some(Time(2700)));
    }

    #[test]
    fn test_parse_time_combined() {
        assert_eq!(parse_time("26 h 12 m"), Some(Time(26 * 3600 + 12 * 60)));
        assert_eq!(parse_time("10 h 12 m"), Some(Time(10 * 3600 + 12 * 60)));
        assert_eq!(parse_time("11 h 5 m"), Some(Time(11 * 3600 + 5 * 60)));
    }

    #[test]
    fn test_parse_time_compact() {
        assert_eq!(parse_time("744m"), Some(Time(744 * 60)));
        assert_eq!(parse_time("2h"), Some(Time(7200)));
    }

    #[test]
    fn test_parse_time_bare_number() {
        assert_eq!(parse_time("3600"), Some(Time(3600)));
        assert_eq!(parse_time("0"), Some(Time(0)));
    }

    #[test]
    fn test_parse_time_seconds() {
        assert_eq!(parse_time("30 s"), Some(Time(30)));
        assert_eq!(parse_time("30 second"), Some(Time(30)));
        assert_eq!(parse_time("60 seconds"), Some(Time(60)));
    }

    #[test]
    fn test_parse_time_empty_or_invalid() {
        assert_eq!(parse_time(""), None);
        assert_eq!(parse_time("   "), None);
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time("xyz h"), None);
    }

    #[test]
    fn test_deserialize_time_number() {
        let t: Time = serde_json::from_str("3600").unwrap();
        assert_eq!(t, Time(3600));

        let t: Time = serde_json::from_str("0").unwrap();
        assert_eq!(t, Time(0));
    }

    #[test]
    fn test_deserialize_time_string() {
        let t: Time = serde_json::from_str(r#""1 h""#).unwrap();
        assert_eq!(t, Time(3600));

        let t: Time = serde_json::from_str(r#""30 m""#).unwrap();
        assert_eq!(t, Time(1800));

        let t: Time = serde_json::from_str(r#""26 h 12 m""#).unwrap();
        assert_eq!(t, Time(26 * 3600 + 12 * 60));

        let t: Time = serde_json::from_str(r#""744m""#).unwrap();
        assert_eq!(t, Time(744 * 60));
    }

    #[test]
    fn test_deserialize_time_defaults_to_zero() {
        // Invalid strings default to Time(0)
        let t: Time = serde_json::from_str(r#""not a time""#).unwrap();
        assert_eq!(t, Time(0));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Time(3661)), "1 h 1 m 1 s");
        assert_eq!(format!("{}", Time(3600)), "1 h");
        assert_eq!(format!("{}", Time(1800)), "30 m");
        assert_eq!(format!("{}", Time(0)), "0 m");
        assert_eq!(format!("{}", Time(45)), "45 s");
    }
}
