use std::fmt;

use sqlx_core::type_info::TypeInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpTypeInfo {
    Null,
    Bool,
    Integer,
    Float,
    Text,
    Blob,
    Json,
}

impl fmt::Display for HttpTypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl TypeInfo for HttpTypeInfo {
    fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    fn name(&self) -> &str {
        match self {
            Self::Null => "NULL",
            Self::Bool => "BOOL",
            Self::Integer => "INTEGER",
            Self::Float => "FLOAT",
            Self::Text => "TEXT",
            Self::Blob => "BLOB",
            Self::Json => "JSON",
        }
    }

    fn type_compatible(&self, other: &Self) -> bool {
        if self == other {
            return true;
        }
        // Lenient: HTTP backends may represent numbers as strings, etc.
        !matches!((self, other), (Self::Null, _) | (_, Self::Null))
    }
}

impl HttpTypeInfo {
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(_) => Self::Bool,
            serde_json::Value::Number(n) => {
                if n.is_f64() && !n.is_i64() && !n.is_u64() {
                    Self::Float
                } else {
                    Self::Integer
                }
            }
            serde_json::Value::String(_) => Self::Text,
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => Self::Json,
        }
    }
}
