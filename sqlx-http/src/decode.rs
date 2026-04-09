use std::collections::HashMap;

use sqlx_core::decode::Decode;
use sqlx_core::error::BoxDynError;

use crate::db::HttpDb;
use crate::types_impl::JsonArray;
use crate::value::HttpValueRef;

impl<'r> Decode<'r, HttpDb> for bool {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::Bool(b) => Ok(*b),
            serde_json::Value::Number(n) => Ok(n.as_i64().map(|v| v != 0).unwrap_or(false)),
            serde_json::Value::String(s) => match s.as_str() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(format!("cannot decode bool from string: {s}").into()),
            },
            _ => Err(format!("expected bool, got {:?}", value.value).into()),
        }
    }
}

macro_rules! impl_decode_int {
    ($($t:ty),+) => {
        $(
            impl<'r> Decode<'r, HttpDb> for $t {
                fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
                    match value.value {
                        serde_json::Value::Number(n) => {
                            if let Some(v) = n.as_i64() {
                                <$t>::try_from(v).map_err(|e| Box::new(e) as BoxDynError)
                            } else if let Some(v) = n.as_u64() {
                                <$t>::try_from(v).map_err(|e| Box::new(e) as BoxDynError)
                            } else if let Some(v) = n.as_f64() {
                                Ok(v as $t)
                            } else {
                                Err(format!("cannot decode {} from number: {n}", stringify!($t)).into())
                            }
                        }
                        serde_json::Value::String(s) => {
                            s.parse::<$t>().map_err(|e| Box::new(e) as BoxDynError)
                        }
                        _ => Err(format!("expected number, got {:?}", value.value).into()),
                    }
                }
            }
        )+
    };
}

impl_decode_int!(i8, i16, i32, i64, u8, u16, u32, u64);

impl<'r> Decode<'r, HttpDb> for f32 {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(|v| v as f32)
                .ok_or_else(|| format!("cannot decode f32 from {n}").into()),
            serde_json::Value::String(s) => {
                s.parse::<f32>().map_err(|e| Box::new(e) as BoxDynError)
            }
            _ => Err(format!("expected number, got {:?}", value.value).into()),
        }
    }
}

impl<'r> Decode<'r, HttpDb> for f64 {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .ok_or_else(|| format!("cannot decode f64 from {n}").into()),
            serde_json::Value::String(s) => {
                s.parse::<f64>().map_err(|e| Box::new(e) as BoxDynError)
            }
            _ => Err(format!("expected number, got {:?}", value.value).into()),
        }
    }
}

impl<'r> Decode<'r, HttpDb> for &'r str {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::String(s) => Ok(s.as_str()),
            _ => Err(format!("expected string, got {:?}", value.value).into()),
        }
    }
}

impl<'r> Decode<'r, HttpDb> for String {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Null => Err("unexpected null value for String".into()),
            other => Ok(other.to_string()),
        }
    }
}

impl<'r> Decode<'r, HttpDb> for Vec<u8> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        use base64::Engine;
        match value.value {
            serde_json::Value::String(s) => base64::engine::general_purpose::STANDARD
                .decode(s)
                .map_err(|e| Box::new(e) as BoxDynError),
            _ => Err(format!("expected string (base64), got {:?}", value.value).into()),
        }
    }
}

impl<'r> Decode<'r, HttpDb> for Box<str> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        <String as Decode<HttpDb>>::decode(value).map(String::into_boxed_str)
    }
}

impl<'r> Decode<'r, HttpDb> for Box<[u8]> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        <Vec<u8> as Decode<HttpDb>>::decode(value).map(Vec::into_boxed_slice)
    }
}

impl<'r> Decode<'r, HttpDb> for std::borrow::Cow<'r, str> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        <String as Decode<HttpDb>>::decode(value).map(std::borrow::Cow::Owned)
    }
}

// ---------------------------------------------------------------------------
// IpAddr, Ipv4Addr, Ipv6Addr
// ---------------------------------------------------------------------------

impl<'r> Decode<'r, HttpDb> for std::net::IpAddr {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::String(s) => Ok(s.parse()?),
            _ => Err(format!("expected string for IpAddr, got {:?}", value.value).into()),
        }
    }
}

impl<'r> Decode<'r, HttpDb> for std::net::Ipv4Addr {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::String(s) => Ok(s.parse()?),
            _ => Err(format!("expected string for Ipv4Addr, got {:?}", value.value).into()),
        }
    }
}

impl<'r> Decode<'r, HttpDb> for std::net::Ipv6Addr {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::String(s) => Ok(s.parse()?),
            _ => Err(format!("expected string for Ipv6Addr, got {:?}", value.value).into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Json<T>
// ---------------------------------------------------------------------------

#[cfg(feature = "json")]
impl<'r, T: serde::de::DeserializeOwned> Decode<'r, HttpDb> for sqlx_core::types::Json<T> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        let json = value.value.clone();
        // If the value is a string, try to parse it as JSON first
        let inner = match &json {
            serde_json::Value::String(s) => serde_json::from_str(s)?,
            other => serde_json::from_value(other.clone())?,
        };
        Ok(sqlx_core::types::Json(inner))
    }
}

// ---------------------------------------------------------------------------
// Text<T>
// ---------------------------------------------------------------------------

impl<'r, T> Decode<'r, HttpDb> for sqlx_core::types::Text<T>
where
    T: std::str::FromStr,
    BoxDynError: From<<T as std::str::FromStr>::Err>,
{
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as Decode<HttpDb>>::decode(value)?;
        Ok(sqlx_core::types::Text(s.parse()?))
    }
}

#[cfg(not(feature = "json"))]
impl<'r> Decode<'r, HttpDb> for serde_json::Value {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(value.value.clone())
    }
}

// ---------------------------------------------------------------------------
// chrono types
// ---------------------------------------------------------------------------

#[cfg(feature = "chrono")]
mod chrono_decode {
    use super::*;

    impl<'r> Decode<'r, HttpDb> for chrono::NaiveDate {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => Ok(s.parse()?),
                serde_json::Value::Number(n) => {
                    // Unix timestamp (days) — interpret as date
                    let ts = n.as_i64().ok_or("expected integer timestamp")?;
                    chrono::DateTime::from_timestamp(ts * 86400, 0)
                        .map(|dt| dt.date_naive())
                        .ok_or_else(|| format!("invalid timestamp for date: {ts}").into())
                }
                _ => Err(format!("expected string or number for NaiveDate, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for chrono::NaiveTime {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => Ok(s.parse()?),
                _ => Err(format!("expected string for NaiveTime, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for chrono::NaiveDateTime {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => {
                    // Try common formats
                    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
                        return Ok(dt);
                    }
                    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
                        return Ok(dt);
                    }
                    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                        return Ok(dt);
                    }
                    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                        return Ok(dt);
                    }
                    // Try parsing as RFC3339 and stripping timezone
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                        return Ok(dt.naive_utc());
                    }
                    Err(format!("cannot parse NaiveDateTime from: {s}").into())
                }
                serde_json::Value::Number(n) => {
                    let ts = n.as_i64().ok_or("expected integer timestamp")?;
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map(|dt| dt.naive_utc())
                        .ok_or_else(|| format!("invalid timestamp: {ts}").into())
                }
                _ => Err(format!("expected string or number for NaiveDateTime, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for chrono::DateTime<chrono::Utc> {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                        return Ok(dt.with_timezone(&chrono::Utc));
                    }
                    // Try without timezone, assume UTC
                    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
                        return Ok(ndt.and_utc());
                    }
                    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
                        return Ok(ndt.and_utc());
                    }
                    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                        return Ok(ndt.and_utc());
                    }
                    Err(format!("cannot parse DateTime<Utc> from: {s}").into())
                }
                serde_json::Value::Number(n) => {
                    let ts = n.as_i64().ok_or("expected integer timestamp")?;
                    chrono::DateTime::from_timestamp(ts, 0)
                        .ok_or_else(|| format!("invalid timestamp: {ts}").into())
                }
                _ => Err(format!("expected string or number for DateTime<Utc>, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for chrono::DateTime<chrono::FixedOffset> {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => {
                    Ok(chrono::DateTime::parse_from_rfc3339(s)?)
                }
                serde_json::Value::Number(n) => {
                    let ts = n.as_i64().ok_or("expected integer timestamp")?;
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map(|dt| dt.fixed_offset())
                        .ok_or_else(|| format!("invalid timestamp: {ts}").into())
                }
                _ => Err(format!("expected string or number for DateTime<FixedOffset>, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for chrono::DateTime<chrono::Local> {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            let utc = chrono::DateTime::<chrono::Utc>::decode(value)?;
            Ok(utc.with_timezone(&chrono::Local))
        }
    }
}

// ---------------------------------------------------------------------------
// time types
// ---------------------------------------------------------------------------

#[cfg(feature = "time")]
mod time_decode {
    use super::*;
    use time::format_description::well_known::Rfc3339;

    impl<'r> Decode<'r, HttpDb> for time::Date {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => {
                    let fmt = time::format_description::parse("[year]-[month]-[day]")?;
                    Ok(time::Date::parse(s, &fmt)?)
                }
                _ => Err(format!("expected string for Date, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for time::Time {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => {
                    let fmt = time::format_description::parse(
                        "[hour]:[minute]:[second][optional [.[subsecond]]]",
                    )?;
                    Ok(time::Time::parse(s, &fmt)?)
                }
                _ => Err(format!("expected string for Time, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for time::PrimitiveDateTime {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => {
                    // Try space-separated first, then T-separated
                    let fmt = time::format_description::parse(
                        "[year]-[month]-[day] [hour]:[minute]:[second][optional [.[subsecond]]]",
                    )?;
                    if let Ok(dt) = time::PrimitiveDateTime::parse(s, &fmt) {
                        return Ok(dt);
                    }
                    let fmt = time::format_description::parse(
                        "[year]-[month]-[day]T[hour]:[minute]:[second][optional [.[subsecond]]]",
                    )?;
                    Ok(time::PrimitiveDateTime::parse(s, &fmt)?)
                }
                serde_json::Value::Number(n) => {
                    let ts = n.as_i64().ok_or("expected integer timestamp")?;
                    let odt = time::OffsetDateTime::from_unix_timestamp(ts)?;
                    Ok(time::PrimitiveDateTime::new(odt.date(), odt.time()))
                }
                _ => Err(format!("expected string or number for PrimitiveDateTime, got {:?}", value.value).into()),
            }
        }
    }

    impl<'r> Decode<'r, HttpDb> for time::OffsetDateTime {
        fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
            match value.value {
                serde_json::Value::String(s) => {
                    Ok(time::OffsetDateTime::parse(s, &Rfc3339)?)
                }
                serde_json::Value::Number(n) => {
                    let ts = n.as_i64().ok_or("expected integer timestamp")?;
                    Ok(time::OffsetDateTime::from_unix_timestamp(ts)?)
                }
                _ => Err(format!("expected string or number for OffsetDateTime, got {:?}", value.value).into()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// uuid
// ---------------------------------------------------------------------------

#[cfg(feature = "uuid")]
impl<'r> Decode<'r, HttpDb> for uuid::Uuid {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::String(s) => Ok(s.parse()?),
            _ => Err(format!("expected string for Uuid, got {:?}", value.value).into()),
        }
    }
}

// ---------------------------------------------------------------------------
// rust_decimal
// ---------------------------------------------------------------------------

#[cfg(feature = "rust_decimal")]
impl<'r> Decode<'r, HttpDb> for rust_decimal::Decimal {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::String(s) => Ok(s.parse()?),
            serde_json::Value::Number(n) => {
                // Try from string representation of the number
                Ok(n.to_string().parse()?)
            }
            _ => Err(format!("expected string or number for Decimal, got {:?}", value.value).into()),
        }
    }
}

// ---------------------------------------------------------------------------
// bigdecimal
// ---------------------------------------------------------------------------

#[cfg(feature = "bigdecimal")]
impl<'r> Decode<'r, HttpDb> for bigdecimal::BigDecimal {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        use std::str::FromStr;
        match value.value {
            serde_json::Value::String(s) => Ok(bigdecimal::BigDecimal::from_str(s)?),
            serde_json::Value::Number(n) => {
                Ok(bigdecimal::BigDecimal::from_str(&n.to_string())?)
            }
            _ => Err(format!("expected string or number for BigDecimal, got {:?}", value.value).into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Vec<T> arrays (skip Vec<u8> which is blob/base64)
// ---------------------------------------------------------------------------

macro_rules! impl_decode_array {
    ($($t:ty),+) => {
        $(
            impl<'r> Decode<'r, HttpDb> for Vec<$t> {
                fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
                    match value.value {
                        serde_json::Value::Array(_) => {
                            serde_json::from_value(value.value.clone())
                                .map_err(|e| Box::new(e) as BoxDynError)
                        }
                        serde_json::Value::String(s) => {
                            // JSON-encoded array stored as text
                            serde_json::from_str(s)
                                .map_err(|e| Box::new(e) as BoxDynError)
                        }
                        serde_json::Value::Null => Err("unexpected null for array".into()),
                        _ => Err(format!("expected array, got {:?}", value.value).into()),
                    }
                }
            }

            impl<'r> Decode<'r, HttpDb> for Vec<Option<$t>> {
                fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
                    match value.value {
                        serde_json::Value::Array(_) => {
                            serde_json::from_value(value.value.clone())
                                .map_err(|e| Box::new(e) as BoxDynError)
                        }
                        serde_json::Value::String(s) => {
                            serde_json::from_str(s)
                                .map_err(|e| Box::new(e) as BoxDynError)
                        }
                        serde_json::Value::Null => Err("unexpected null for array".into()),
                        _ => Err(format!("expected array, got {:?}", value.value).into()),
                    }
                }
            }
        )+
    };
}

impl_decode_array!(bool, i8, i16, i32, i64, u16, u32, u64, f32, f64, String);

impl<'r> Decode<'r, HttpDb> for Vec<serde_json::Value> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::Array(arr) => Ok(arr.clone()),
            serde_json::Value::String(s) => {
                serde_json::from_str(s).map_err(|e| Box::new(e) as BoxDynError)
            }
            serde_json::Value::Null => Err("unexpected null for array".into()),
            _ => Err(format!("expected array, got {:?}", value.value).into()),
        }
    }
}

// Vec<T: JsonArray> — custom struct arrays
impl<'r, T: JsonArray> Decode<'r, HttpDb> for Vec<T> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::Array(_) => {
                serde_json::from_value(value.value.clone())
                    .map_err(|e| Box::new(e) as BoxDynError)
            }
            serde_json::Value::String(s) => {
                serde_json::from_str(s)
                    .map_err(|e| Box::new(e) as BoxDynError)
            }
            serde_json::Value::Null => Err("unexpected null for array".into()),
            _ => Err(format!("expected array, got {:?}", value.value).into()),
        }
    }
}

// ---------------------------------------------------------------------------
// HashMap<String, V>
// ---------------------------------------------------------------------------

impl<'r, V: serde::de::DeserializeOwned> Decode<'r, HttpDb> for HashMap<String, V> {
    fn decode(value: HttpValueRef<'r>) -> Result<Self, BoxDynError> {
        match value.value {
            serde_json::Value::Object(_) => {
                serde_json::from_value(value.value.clone())
                    .map_err(|e| Box::new(e) as BoxDynError)
            }
            serde_json::Value::String(s) => {
                serde_json::from_str(s)
                    .map_err(|e| Box::new(e) as BoxDynError)
            }
            serde_json::Value::Null => Err("unexpected null for HashMap".into()),
            _ => Err(format!("expected object, got {:?}", value.value).into()),
        }
    }
}
