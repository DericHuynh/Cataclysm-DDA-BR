use serde::{Deserialize, Serialize};
use std::fmt;

/// A localized string that may contain translations for multiple languages.
///
/// In CDDA JSON, this appears as:
/// ```json
/// { "str": "name", "//~": "context" }
/// { "str_sp": "name" }           // same for singular & plural
/// { "str_pl": "names" }          // explicit plural
/// ```
/// Or as a plain string `"name"`.
///
/// For Stage 1 we store the raw English text. Full i18n support
/// (extracting translations) is deferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedString {
    /// A plain, non-localized string.
    Plain(String),
    /// A localized object with optional plural and context.
    Object {
        #[serde(rename = "str", default)]
        singular: Option<String>,
        #[serde(rename = "str_sp", skip_serializing_if = "Option::is_none")]
        same_plural: Option<String>,
        #[serde(rename = "str_pl", skip_serializing_if = "Option::is_none")]
        plural: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },
}

impl LocalizedString {
    /// Get the singular English string.
    pub fn singular(&self) -> &str {
        match self {
            LocalizedString::Plain(s) => s.as_str(),
            LocalizedString::Object {
                singular: Some(s), ..
            } => s.as_str(),
            LocalizedString::Object {
                same_plural: Some(sp),
                ..
            } => sp.as_str(),
            LocalizedString::Object { .. } => "",
        }
    }

    /// Get the plural English string (falls back to singular if not specified).
    pub fn plural(&self) -> &str {
        match self {
            LocalizedString::Plain(s) => s.as_str(),
            LocalizedString::Object {
                same_plural: Some(sp),
                ..
            } => sp.as_str(),
            LocalizedString::Object {
                plural: Some(p), ..
            } => p.as_str(),
            LocalizedString::Object {
                singular: Some(s), ..
            } => s.as_str(),
            LocalizedString::Object { .. } => "",
        }
    }
}

impl From<String> for LocalizedString {
    fn from(s: String) -> Self {
        LocalizedString::Plain(s)
    }
}

impl From<&str> for LocalizedString {
    fn from(s: &str) -> Self {
        LocalizedString::Plain(s.to_string())
    }
}

impl fmt::Display for LocalizedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.singular())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_string() {
        let s: LocalizedString = "hello".into();
        assert_eq!(s.singular(), "hello");
        assert_eq!(s.plural(), "hello");
    }

    #[test]
    fn test_object_string() {
        let json = r#"{"str": "item", "str_pl": "items"}"#;
        let s: LocalizedString = serde_json::from_str(json).unwrap();
        assert_eq!(s.singular(), "item");
        assert_eq!(s.plural(), "items");
    }

    #[test]
    fn test_object_same_plural() {
        let json = r#"{"str_sp": "sheep"}"#;
        let s: LocalizedString = serde_json::from_str(json).unwrap();
        assert_eq!(s.singular(), "sheep");
        assert_eq!(s.plural(), "sheep");
    }
}
