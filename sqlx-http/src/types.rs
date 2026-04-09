use std::collections::HashMap;

use sqlx_core::types::Type;

use crate::db::HttpDb;
use crate::type_info::HttpTypeInfo;

impl Type<HttpDb> for bool {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Bool
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Bool | HttpTypeInfo::Integer | HttpTypeInfo::Text)
    }
}

macro_rules! impl_type_int {
    ($($t:ty),+) => {
        $(
            impl Type<HttpDb> for $t {
                fn type_info() -> HttpTypeInfo {
                    HttpTypeInfo::Integer
                }

                fn compatible(ty: &HttpTypeInfo) -> bool {
                    matches!(ty, HttpTypeInfo::Integer | HttpTypeInfo::Float | HttpTypeInfo::Text)
                }
            }
        )+
    };
}

impl_type_int!(i8, i16, i32, i64, u8, u16, u32, u64);

impl Type<HttpDb> for f32 {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Float
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Float | HttpTypeInfo::Integer | HttpTypeInfo::Text)
    }
}

impl Type<HttpDb> for f64 {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Float
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Float | HttpTypeInfo::Integer | HttpTypeInfo::Text)
    }
}

impl Type<HttpDb> for str {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        !matches!(ty, HttpTypeInfo::Null)
    }
}

impl Type<HttpDb> for String {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        !matches!(ty, HttpTypeInfo::Null)
    }
}

impl Type<HttpDb> for Vec<u8> {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Blob
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Blob | HttpTypeInfo::Text)
    }
}

#[cfg(not(feature = "json"))]
impl Type<HttpDb> for serde_json::Value {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Json
    }

    fn compatible(_ty: &HttpTypeInfo) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Box<str>, Box<[u8]>, Cow<str>
// ---------------------------------------------------------------------------

impl Type<HttpDb> for Box<str> {
    fn type_info() -> HttpTypeInfo {
        <str as Type<HttpDb>>::type_info()
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        <str as Type<HttpDb>>::compatible(ty)
    }
}

impl Type<HttpDb> for Box<[u8]> {
    fn type_info() -> HttpTypeInfo {
        <Vec<u8> as Type<HttpDb>>::type_info()
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        <Vec<u8> as Type<HttpDb>>::compatible(ty)
    }
}

impl Type<HttpDb> for std::borrow::Cow<'_, str> {
    fn type_info() -> HttpTypeInfo {
        <str as Type<HttpDb>>::type_info()
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        <str as Type<HttpDb>>::compatible(ty)
    }
}

// ---------------------------------------------------------------------------
// IpAddr, Ipv4Addr, Ipv6Addr
// ---------------------------------------------------------------------------

impl Type<HttpDb> for std::net::IpAddr {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        <str as Type<HttpDb>>::compatible(ty)
    }
}

impl Type<HttpDb> for std::net::Ipv4Addr {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        <str as Type<HttpDb>>::compatible(ty)
    }
}

impl Type<HttpDb> for std::net::Ipv6Addr {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        <str as Type<HttpDb>>::compatible(ty)
    }
}

// ---------------------------------------------------------------------------
// Json<T>
// ---------------------------------------------------------------------------

#[cfg(feature = "json")]
impl<T: serde::Serialize> Type<HttpDb> for sqlx_core::types::Json<T> {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Json
    }

    fn compatible(_ty: &HttpTypeInfo) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Text<T>
// ---------------------------------------------------------------------------

impl<T: std::fmt::Display> Type<HttpDb> for sqlx_core::types::Text<T> {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        <str as Type<HttpDb>>::compatible(ty)
    }
}

// ---------------------------------------------------------------------------
// chrono types
// ---------------------------------------------------------------------------

#[cfg(feature = "chrono")]
mod chrono_types {
    use super::*;

    impl Type<HttpDb> for chrono::NaiveDate {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }

    impl Type<HttpDb> for chrono::NaiveTime {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text)
        }
    }

    impl Type<HttpDb> for chrono::NaiveDateTime {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }

    impl Type<HttpDb> for chrono::DateTime<chrono::Utc> {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }

    impl Type<HttpDb> for chrono::DateTime<chrono::FixedOffset> {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }

    impl Type<HttpDb> for chrono::DateTime<chrono::Local> {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }
}

// ---------------------------------------------------------------------------
// time types
// ---------------------------------------------------------------------------

#[cfg(feature = "time")]
mod time_types {
    use super::*;

    impl Type<HttpDb> for time::Date {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }

    impl Type<HttpDb> for time::Time {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text)
        }
    }

    impl Type<HttpDb> for time::PrimitiveDateTime {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }

    impl Type<HttpDb> for time::OffsetDateTime {
        fn type_info() -> HttpTypeInfo {
            HttpTypeInfo::Text
        }

        fn compatible(ty: &HttpTypeInfo) -> bool {
            matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Integer)
        }
    }
}

// ---------------------------------------------------------------------------
// uuid
// ---------------------------------------------------------------------------

#[cfg(feature = "uuid")]
impl Type<HttpDb> for uuid::Uuid {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Text | HttpTypeInfo::Blob)
    }
}

// ---------------------------------------------------------------------------
// rust_decimal
// ---------------------------------------------------------------------------

#[cfg(feature = "rust_decimal")]
impl Type<HttpDb> for rust_decimal::Decimal {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(
            ty,
            HttpTypeInfo::Text | HttpTypeInfo::Float | HttpTypeInfo::Integer
        )
    }
}

// ---------------------------------------------------------------------------
// bigdecimal
// ---------------------------------------------------------------------------

#[cfg(feature = "bigdecimal")]
impl Type<HttpDb> for bigdecimal::BigDecimal {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Text
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(
            ty,
            HttpTypeInfo::Text | HttpTypeInfo::Float | HttpTypeInfo::Integer
        )
    }
}

// ---------------------------------------------------------------------------
// Vec<T> arrays (skip Vec<u8> which is blob/base64)
// ---------------------------------------------------------------------------

macro_rules! impl_type_array {
    ($($t:ty),+) => {
        $(
            impl Type<HttpDb> for Vec<$t> {
                fn type_info() -> HttpTypeInfo {
                    HttpTypeInfo::Json
                }

                fn compatible(ty: &HttpTypeInfo) -> bool {
                    matches!(ty, HttpTypeInfo::Json | HttpTypeInfo::Text)
                }
            }

            impl Type<HttpDb> for Vec<Option<$t>> {
                fn type_info() -> HttpTypeInfo {
                    HttpTypeInfo::Json
                }

                fn compatible(ty: &HttpTypeInfo) -> bool {
                    matches!(ty, HttpTypeInfo::Json | HttpTypeInfo::Text)
                }
            }
        )+
    };
}

impl_type_array!(bool, i8, i16, i32, i64, u16, u32, u64, f32, f64, String);

impl Type<HttpDb> for Vec<serde_json::Value> {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Json
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Json | HttpTypeInfo::Text)
    }
}

// ---------------------------------------------------------------------------
// JsonArray — marker trait for custom struct arrays
// ---------------------------------------------------------------------------

/// Marker trait for types that can be stored as JSON array elements.
///
/// Implement this for your own types to enable `Vec<T>` and `Vec<Option<T>>`
/// as query parameters and result columns:
///
/// ```rust,ignore
/// #[derive(serde::Serialize, serde::Deserialize, FromRow)]
/// struct Tag { name: String, value: String }
///
/// impl sqlx_http::JsonArray for Tag {}
///
/// // Now this works:
/// let tags: Vec<Tag> = row.get("tags");
/// ```
///
/// Built-in scalar types (`Vec<i32>`, `Vec<String>`, etc.) do NOT need this
/// trait — they have dedicated impls.
pub trait JsonArray: serde::Serialize + serde::de::DeserializeOwned {}

impl<T: JsonArray> JsonArray for Option<T> {}

impl<T: JsonArray> Type<HttpDb> for Vec<T> {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Json
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Json | HttpTypeInfo::Text)
    }
}

// ---------------------------------------------------------------------------
// HashMap<String, V>
// ---------------------------------------------------------------------------

impl<V: serde::Serialize + serde::de::DeserializeOwned> Type<HttpDb> for HashMap<String, V> {
    fn type_info() -> HttpTypeInfo {
        HttpTypeInfo::Json
    }

    fn compatible(ty: &HttpTypeInfo) -> bool {
        matches!(ty, HttpTypeInfo::Json | HttpTypeInfo::Text)
    }
}
