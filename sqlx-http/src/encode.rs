use std::collections::HashMap;

use base64::Engine;
use sqlx_core::encode::{Encode, IsNull};
use sqlx_core::error::BoxDynError;

use crate::db::HttpDb;
use crate::types_impl::JsonArray;

impl Encode<'_, HttpDb> for bool {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::Bool(*self));
        Ok(IsNull::No)
    }
}

macro_rules! impl_encode_int {
    ($($t:ty),+) => {
        $(
            impl Encode<'_, HttpDb> for $t {
                fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
                    buf.push(serde_json::json!(*self));
                    Ok(IsNull::No)
                }
            }
        )+
    };
}

impl_encode_int!(i8, i16, i32, i64, u8, u16, u32, u64);

impl Encode<'_, HttpDb> for f32 {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        let v = *self as f64;
        let n = serde_json::Number::from_f64(v)
            .ok_or_else(|| format!("cannot encode f32 {self}: NaN and Infinity are not valid JSON"))?;
        buf.push(serde_json::Value::Number(n));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for f64 {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        let n = serde_json::Number::from_f64(*self)
            .ok_or_else(|| format!("cannot encode f64 {self}: NaN and Infinity are not valid JSON"))?;
        buf.push(serde_json::Value::Number(n));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for &str {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String((*self).to_string()));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for String {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.clone()));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for Vec<u8> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(self);
        buf.push(serde_json::Value::String(encoded));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for Box<str> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        <&str as Encode<HttpDb>>::encode_by_ref(&self.as_ref(), buf)
    }
}

impl Encode<'_, HttpDb> for Box<[u8]> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(self.as_ref());
        buf.push(serde_json::Value::String(encoded));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for std::borrow::Cow<'_, str> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        <&str as Encode<HttpDb>>::encode_by_ref(&self.as_ref(), buf)
    }
}

// ---------------------------------------------------------------------------
// IpAddr, Ipv4Addr, Ipv6Addr
// ---------------------------------------------------------------------------

impl Encode<'_, HttpDb> for std::net::IpAddr {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.to_string()));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for std::net::Ipv4Addr {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.to_string()));
        Ok(IsNull::No)
    }
}

impl Encode<'_, HttpDb> for std::net::Ipv6Addr {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.to_string()));
        Ok(IsNull::No)
    }
}

// ---------------------------------------------------------------------------
// Json<T>
// ---------------------------------------------------------------------------

#[cfg(feature = "json")]
impl<T: serde::Serialize> Encode<'_, HttpDb> for sqlx_core::types::Json<T> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        let json = serde_json::to_value(&self.0)?;
        buf.push(json);
        Ok(IsNull::No)
    }
}

// ---------------------------------------------------------------------------
// Text<T>
// ---------------------------------------------------------------------------

impl<T: std::fmt::Display> Encode<'_, HttpDb> for sqlx_core::types::Text<T> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.0.to_string()));
        Ok(IsNull::No)
    }
}

#[cfg(not(feature = "json"))]
impl Encode<'_, HttpDb> for serde_json::Value {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(self.clone());
        Ok(if self.is_null() {
            IsNull::Yes
        } else {
            IsNull::No
        })
    }
}

// ---------------------------------------------------------------------------
// chrono types
// ---------------------------------------------------------------------------

#[cfg(feature = "chrono")]
mod chrono_encode {
    use super::*;

    impl Encode<'_, HttpDb> for chrono::NaiveDate {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            buf.push(serde_json::Value::String(self.format("%Y-%m-%d").to_string()));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for chrono::NaiveTime {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            buf.push(serde_json::Value::String(self.format("%H:%M:%S%.f").to_string()));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for chrono::NaiveDateTime {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            buf.push(serde_json::Value::String(self.format("%Y-%m-%d %H:%M:%S%.f").to_string()));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for chrono::DateTime<chrono::Utc> {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            buf.push(serde_json::Value::String(self.to_rfc3339()));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for chrono::DateTime<chrono::FixedOffset> {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            buf.push(serde_json::Value::String(self.to_rfc3339()));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for chrono::DateTime<chrono::Local> {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            buf.push(serde_json::Value::String(self.to_rfc3339()));
            Ok(IsNull::No)
        }
    }
}

// ---------------------------------------------------------------------------
// time types
// ---------------------------------------------------------------------------

#[cfg(feature = "time")]
mod time_encode {
    use super::*;
    use time::format_description::well_known::Rfc3339;

    impl Encode<'_, HttpDb> for time::Date {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            let fmt = time::format_description::parse("[year]-[month]-[day]")?;
            buf.push(serde_json::Value::String(self.format(&fmt)?));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for time::Time {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            let fmt = time::format_description::parse("[hour]:[minute]:[second].[subsecond]")?;
            buf.push(serde_json::Value::String(self.format(&fmt)?));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for time::PrimitiveDateTime {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            let fmt = time::format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
            )?;
            buf.push(serde_json::Value::String(self.format(&fmt)?));
            Ok(IsNull::No)
        }
    }

    impl Encode<'_, HttpDb> for time::OffsetDateTime {
        fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
            buf.push(serde_json::Value::String(self.format(&Rfc3339)?));
            Ok(IsNull::No)
        }
    }
}

// ---------------------------------------------------------------------------
// uuid
// ---------------------------------------------------------------------------

#[cfg(feature = "uuid")]
impl Encode<'_, HttpDb> for uuid::Uuid {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.to_string()));
        Ok(IsNull::No)
    }
}

// ---------------------------------------------------------------------------
// rust_decimal
// ---------------------------------------------------------------------------

#[cfg(feature = "rust_decimal")]
impl Encode<'_, HttpDb> for rust_decimal::Decimal {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.to_string()));
        Ok(IsNull::No)
    }
}

// ---------------------------------------------------------------------------
// bigdecimal
// ---------------------------------------------------------------------------

#[cfg(feature = "bigdecimal")]
impl Encode<'_, HttpDb> for bigdecimal::BigDecimal {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::String(self.to_string()));
        Ok(IsNull::No)
    }
}

// ---------------------------------------------------------------------------
// Vec<T> arrays (skip Vec<u8> which is blob/base64)
// ---------------------------------------------------------------------------

macro_rules! impl_encode_array {
    ($($t:ty),+) => {
        $(
            impl Encode<'_, HttpDb> for Vec<$t> {
                fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
                    buf.push(serde_json::to_value(self)?);
                    Ok(IsNull::No)
                }
            }

            impl Encode<'_, HttpDb> for Vec<Option<$t>> {
                fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
                    buf.push(serde_json::to_value(self)?);
                    Ok(IsNull::No)
                }
            }
        )+
    };
}

impl_encode_array!(bool, i8, i16, i32, i64, u16, u32, u64, String);

// f32/f64 arrays need special handling to reject NaN/Infinity
macro_rules! impl_encode_float_array {
    ($($t:ty),+) => {
        $(
            impl Encode<'_, HttpDb> for Vec<$t> {
                fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
                    let arr: Result<Vec<serde_json::Value>, BoxDynError> = self.iter().map(|v| {
                        let n = serde_json::Number::from_f64(*v as f64)
                            .ok_or_else(|| format!("cannot encode {}: NaN and Infinity are not valid JSON", v))?;
                        Ok(serde_json::Value::Number(n))
                    }).collect();
                    buf.push(serde_json::Value::Array(arr?));
                    Ok(IsNull::No)
                }
            }

            impl Encode<'_, HttpDb> for Vec<Option<$t>> {
                fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
                    let arr: Result<Vec<serde_json::Value>, BoxDynError> = self.iter().map(|v| {
                        match v {
                            Some(v) => {
                                let n = serde_json::Number::from_f64(*v as f64)
                                    .ok_or_else(|| format!("cannot encode {}: NaN and Infinity are not valid JSON", v))?;
                                Ok(serde_json::Value::Number(n))
                            }
                            None => Ok(serde_json::Value::Null),
                        }
                    }).collect();
                    buf.push(serde_json::Value::Array(arr?));
                    Ok(IsNull::No)
                }
            }
        )+
    };
}

impl_encode_float_array!(f32, f64);

impl Encode<'_, HttpDb> for Vec<serde_json::Value> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::Value::Array(self.clone()));
        Ok(IsNull::No)
    }
}

// Vec<T: JsonArray> — custom struct arrays
impl<T: JsonArray> Encode<'_, HttpDb> for Vec<T> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::to_value(self)?);
        Ok(IsNull::No)
    }
}

// ---------------------------------------------------------------------------
// HashMap<String, V>
// ---------------------------------------------------------------------------

impl<V: serde::Serialize> Encode<'_, HttpDb> for HashMap<String, V> {
    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        buf.push(serde_json::to_value(self)?);
        Ok(IsNull::No)
    }
}

impl<'q, T: Encode<'q, HttpDb>> Encode<'q, HttpDb> for Option<T> {
    fn encode(self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        if let Some(v) = self {
            v.encode(buf)
        } else {
            buf.push(serde_json::Value::Null);
            Ok(IsNull::Yes)
        }
    }

    fn encode_by_ref(&self, buf: &mut Vec<serde_json::Value>) -> Result<IsNull, BoxDynError> {
        if let Some(v) = self {
            v.encode_by_ref(buf)
        } else {
            buf.push(serde_json::Value::Null);
            Ok(IsNull::Yes)
        }
    }
}
