use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;

/// A type-safe identifier for game definitions.
///
/// `DefId<ItemDef>` and `DefId<MonsterDef>` are incompatible at compile time,
/// preventing bugs where the wrong ID type is passed to a lookup.
///
/// Internally stored as a plain String, but the type parameter ensures
/// compile-time safety.
#[derive(Debug, Clone, Serialize)]
pub struct DefId<T> {
    id: String,
    #[serde(skip)]
    _marker: PhantomData<T>,
}

// Manual JsonSchema: DefId serializes/deserializes as a plain string.
impl<T> JsonSchema for DefId<T> {
    fn schema_name() -> String {
        "DefId".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <String>::json_schema(_gen)
    }
}

// Manual PartialEq, Eq, Hash to avoid unnecessary bounds on T
impl<T> PartialEq for DefId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for DefId<T> {}

impl<T> std::hash::Hash for DefId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> DefId<T> {
    /// Create a new `DefId` from a string.
    pub fn new(id: impl Into<String>) -> Self {
        DefId {
            id: id.into(),
            _marker: PhantomData,
        }
    }

    /// Get the underlying string ID.
    pub fn as_str(&self) -> &str {
        &self.id
    }

    /// Convert into the underlying string.
    pub fn into_string(self) -> String {
        self.id
    }
}

impl<T> fmt::Display for DefId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

// Custom deserialize because PhantomData doesn't implement Deserialize
impl<'de, T> Deserialize<'de> for DefId<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        Ok(DefId {
            id,
            _marker: PhantomData,
        })
    }
}

impl<T> From<String> for DefId<T> {
    fn from(s: String) -> Self {
        DefId::new(s)
    }
}

impl<T> From<&str> for DefId<T> {
    fn from(s: &str) -> Self {
        DefId::new(s)
    }
}

impl<T> std::ops::Deref for DefId<T> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.id
    }
}

impl<T> PartialEq<str> for DefId<T> {
    fn eq(&self, other: &str) -> bool {
        self.id == other
    }
}
