#[cfg(test)]
mod encode_tests {
    use sqlx_core::encode::Encode;

    use crate::db::HttpDb;

    fn encode_value<'q, T: Encode<'q, HttpDb>>(value: T) -> serde_json::Value {
        let mut buf = Vec::new();
        let _ = value.encode(&mut buf).unwrap();
        buf.into_iter().next().unwrap()
    }

    #[test]
    fn encode_bool() {
        assert_eq!(encode_value(true), serde_json::json!(true));
        assert_eq!(encode_value(false), serde_json::json!(false));
    }

    #[test]
    fn encode_integers() {
        assert_eq!(encode_value(42_i32), serde_json::json!(42));
        assert_eq!(encode_value(-1_i64), serde_json::json!(-1));
        assert_eq!(encode_value(255_u8), serde_json::json!(255));
        assert_eq!(encode_value(0_u64), serde_json::json!(0));
    }

    #[test]
    fn encode_floats() {
        assert_eq!(encode_value(3.14_f64), serde_json::json!(3.14));
        // f32 is cast to f64
        let val = encode_value(1.5_f32);
        assert!(val.is_number());
    }

    #[test]
    fn encode_string() {
        assert_eq!(encode_value("hello"), serde_json::json!("hello"));
        assert_eq!(
            encode_value("world".to_string()),
            serde_json::json!("world")
        );
    }

    #[test]
    fn encode_blob() {
        let val = encode_value(vec![0u8, 1, 2, 3]);
        assert_eq!(val, serde_json::json!("AAECAw=="));
    }

    #[test]
    fn encode_json_value() {
        let json = serde_json::json!({"key": "value"});
        assert_eq!(encode_value(json.clone()), json);
    }

    #[test]
    fn encode_option_some() {
        assert_eq!(encode_value(Some(42_i32)), serde_json::json!(42));
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn encode_chrono_naive_date() {
        let d = chrono::NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        assert_eq!(encode_value(d), serde_json::json!("2024-03-15"));
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn encode_chrono_datetime_utc() {
        let dt = chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
            .unwrap()
            .and_hms_opt(12, 30, 0)
            .unwrap()
            .and_utc();
        let encoded = encode_value(dt);
        let s = encoded.as_str().unwrap();
        assert!(s.starts_with("2024-03-15T12:30:00"));
    }

    #[cfg(feature = "uuid")]
    #[test]
    fn encode_uuid() {
        let u = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(
            encode_value(u),
            serde_json::json!("550e8400-e29b-41d4-a716-446655440000")
        );
    }

    #[cfg(feature = "rust_decimal")]
    #[test]
    fn encode_rust_decimal() {
        let d: rust_decimal::Decimal = "123.456".parse().unwrap();
        assert_eq!(encode_value(d), serde_json::json!("123.456"));
    }

    #[cfg(feature = "bigdecimal")]
    #[test]
    fn encode_bigdecimal() {
        use std::str::FromStr;
        let d = bigdecimal::BigDecimal::from_str("99999.12345").unwrap();
        assert_eq!(encode_value(d), serde_json::json!("99999.12345"));
    }

    #[cfg(feature = "time")]
    #[test]
    fn encode_time_offset_datetime() {
        let odt = time::OffsetDateTime::from_unix_timestamp(1710504600).unwrap();
        let encoded = encode_value(odt);
        let s = encoded.as_str().unwrap();
        assert!(s.starts_with("2024-03-15T"));
    }

    #[test]
    fn encode_box_str() {
        let s: Box<str> = "hello".into();
        assert_eq!(encode_value(s), serde_json::json!("hello"));
    }

    #[test]
    fn encode_box_bytes() {
        let b: Box<[u8]> = vec![0u8, 1, 2, 3].into_boxed_slice();
        assert_eq!(encode_value(b), serde_json::json!("AAECAw=="));
    }

    #[test]
    fn encode_cow_str() {
        let s = std::borrow::Cow::Borrowed("world");
        assert_eq!(encode_value(s), serde_json::json!("world"));
    }

    #[test]
    fn encode_ipaddr() {
        let ip: std::net::IpAddr = "192.168.1.1".parse().unwrap();
        assert_eq!(encode_value(ip), serde_json::json!("192.168.1.1"));
    }

    #[test]
    fn encode_ipv4addr() {
        let ip: std::net::Ipv4Addr = "10.0.0.1".parse().unwrap();
        assert_eq!(encode_value(ip), serde_json::json!("10.0.0.1"));
    }

    #[test]
    fn encode_ipv6addr() {
        let ip: std::net::Ipv6Addr = "::1".parse().unwrap();
        assert_eq!(encode_value(ip), serde_json::json!("::1"));
    }

    #[cfg(feature = "json")]
    #[test]
    fn encode_json_wrapper() {
        use sqlx_core::types::Json;
        let val = Json(serde_json::json!({"key": "value"}));
        assert_eq!(encode_value(val), serde_json::json!({"key": "value"}));
    }

    #[test]
    fn encode_text_wrapper() {
        use sqlx_core::types::Text;
        let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
        assert_eq!(
            encode_value(Text(addr)),
            serde_json::json!("127.0.0.1:8080")
        );
    }

    #[test]
    fn encode_nonzero() {
        let n = std::num::NonZeroI32::new(42).unwrap();
        assert_eq!(encode_value(n), serde_json::json!(42));
    }

    #[test]
    fn encode_f64_nan_errors() {
        let mut buf = Vec::new();
        assert!(f64::NAN.encode(&mut buf).is_err());
    }

    #[test]
    fn encode_f64_infinity_errors() {
        let mut buf = Vec::new();
        assert!(f64::INFINITY.encode(&mut buf).is_err());
    }

    #[test]
    fn encode_f32_nan_errors() {
        let mut buf = Vec::new();
        assert!(f32::NAN.encode(&mut buf).is_err());
    }

    #[test]
    fn encode_option_none() {
        let mut buf = Vec::new();
        let result = Option::<i32>::None.encode(&mut buf).unwrap();
        assert!(result.is_null());
        assert_eq!(buf[0], serde_json::Value::Null);
    }

    // --- Array encode tests ---

    #[test]
    fn encode_vec_i32() {
        assert_eq!(encode_value(vec![1_i32, 2, 3]), serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn encode_vec_i64() {
        assert_eq!(encode_value(vec![10_i64, 20]), serde_json::json!([10, 20]));
    }

    #[test]
    fn encode_vec_f64() {
        assert_eq!(encode_value(vec![1.5_f64, 2.5]), serde_json::json!([1.5, 2.5]));
    }

    #[test]
    fn encode_vec_f64_nan_errors() {
        let mut buf = Vec::new();
        assert!(vec![1.0_f64, f64::NAN].encode(&mut buf).is_err());
    }

    #[test]
    fn encode_vec_bool() {
        assert_eq!(encode_value(vec![true, false, true]), serde_json::json!([true, false, true]));
    }

    #[test]
    fn encode_vec_string() {
        assert_eq!(
            encode_value(vec!["hello".to_string(), "world".to_string()]),
            serde_json::json!(["hello", "world"])
        );
    }

    #[test]
    fn encode_vec_empty() {
        assert_eq!(encode_value(Vec::<i32>::new()), serde_json::json!([]));
    }

    #[test]
    fn encode_vec_option_i32() {
        assert_eq!(
            encode_value(vec![Some(1_i32), None, Some(3)]),
            serde_json::json!([1, null, 3])
        );
    }

    #[test]
    fn encode_vec_option_string() {
        assert_eq!(
            encode_value(vec![Some("a".to_string()), None]),
            serde_json::json!(["a", null])
        );
    }

    #[test]
    fn encode_vec_json_value() {
        let arr = vec![serde_json::json!(1), serde_json::json!("two"), serde_json::json!(null)];
        assert_eq!(encode_value(arr), serde_json::json!([1, "two", null]));
    }
}

#[cfg(test)]
mod decode_tests {
    use sqlx_core::decode::Decode;

    use crate::type_info::HttpTypeInfo;
    use crate::value::HttpValueRef;

    fn make_ref(value: &serde_json::Value) -> HttpValueRef<'_> {
        HttpValueRef {
            value,
            type_info: HttpTypeInfo::from_json(value),
        }
    }

    #[test]
    fn decode_bool() {
        let val = serde_json::json!(true);
        assert!(bool::decode(make_ref(&val)).unwrap());

        let val = serde_json::json!(0);
        assert!(!bool::decode(make_ref(&val)).unwrap());

        let val = serde_json::json!("true");
        assert!(bool::decode(make_ref(&val)).unwrap());
    }

    #[test]
    fn decode_integers() {
        let val = serde_json::json!(42);
        assert_eq!(i32::decode(make_ref(&val)).unwrap(), 42);
        assert_eq!(i64::decode(make_ref(&val)).unwrap(), 42);
        assert_eq!(u32::decode(make_ref(&val)).unwrap(), 42);

        // From string
        let val = serde_json::json!("123");
        assert_eq!(i64::decode(make_ref(&val)).unwrap(), 123);
    }

    #[test]
    fn decode_floats() {
        let val = serde_json::json!(3.14);
        let f = f64::decode(make_ref(&val)).unwrap();
        assert!((f - 3.14).abs() < f64::EPSILON);

        // From string
        let val = serde_json::json!("2.718");
        let f = f64::decode(make_ref(&val)).unwrap();
        assert!((f - 2.718).abs() < f64::EPSILON);
    }

    #[test]
    fn decode_string() {
        let val = serde_json::json!("hello");
        assert_eq!(String::decode(make_ref(&val)).unwrap(), "hello");

        // &str borrowing
        assert_eq!(<&str>::decode(make_ref(&val)).unwrap(), "hello");
    }

    #[test]
    fn decode_blob() {
        let val = serde_json::json!("AAECAw==");
        assert_eq!(Vec::<u8>::decode(make_ref(&val)).unwrap(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn decode_json_value() {
        let val = serde_json::json!({"key": "value"});
        let decoded = serde_json::Value::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, val);
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn decode_chrono_naive_date() {
        let val = serde_json::json!("2024-03-15");
        let d = chrono::NaiveDate::decode(make_ref(&val)).unwrap();
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2024, 3, 15).unwrap());
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn decode_chrono_naive_datetime() {
        let val = serde_json::json!("2024-03-15 12:30:00");
        let dt = chrono::NaiveDateTime::decode(make_ref(&val)).unwrap();
        assert_eq!(
            dt,
            chrono::NaiveDate::from_ymd_opt(2024, 3, 15)
                .unwrap()
                .and_hms_opt(12, 30, 0)
                .unwrap()
        );
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn decode_chrono_datetime_utc() {
        let val = serde_json::json!("2024-03-15T12:30:00+00:00");
        let dt = chrono::DateTime::<chrono::Utc>::decode(make_ref(&val)).unwrap();
        assert_eq!(dt.date_naive(), chrono::NaiveDate::from_ymd_opt(2024, 3, 15).unwrap());
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn decode_chrono_datetime_utc_from_timestamp() {
        let val = serde_json::json!(1710504600);
        let dt = chrono::DateTime::<chrono::Utc>::decode(make_ref(&val)).unwrap();
        assert_eq!(dt.date_naive(), chrono::NaiveDate::from_ymd_opt(2024, 3, 15).unwrap());
    }

    #[cfg(feature = "uuid")]
    #[test]
    fn decode_uuid() {
        let val = serde_json::json!("550e8400-e29b-41d4-a716-446655440000");
        let u = uuid::Uuid::decode(make_ref(&val)).unwrap();
        assert_eq!(u.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[cfg(feature = "rust_decimal")]
    #[test]
    fn decode_rust_decimal_from_string() {
        let val = serde_json::json!("123.456");
        let d = rust_decimal::Decimal::decode(make_ref(&val)).unwrap();
        assert_eq!(d.to_string(), "123.456");
    }

    #[cfg(feature = "rust_decimal")]
    #[test]
    fn decode_rust_decimal_from_number() {
        let val = serde_json::json!(42.5);
        let d = rust_decimal::Decimal::decode(make_ref(&val)).unwrap();
        assert_eq!(d.to_string(), "42.5");
    }

    #[cfg(feature = "bigdecimal")]
    #[test]
    fn decode_bigdecimal_from_string() {
        use std::str::FromStr;
        let val = serde_json::json!("99999.12345");
        let d = bigdecimal::BigDecimal::decode(make_ref(&val)).unwrap();
        assert_eq!(d, bigdecimal::BigDecimal::from_str("99999.12345").unwrap());
    }

    #[cfg(feature = "time")]
    #[test]
    fn decode_time_offset_datetime() {
        let val = serde_json::json!("2024-03-15T12:30:00Z");
        let odt = time::OffsetDateTime::decode(make_ref(&val)).unwrap();
        assert_eq!(odt.year(), 2024);
        assert_eq!(odt.month(), time::Month::March);
        assert_eq!(odt.day(), 15);
    }

    #[cfg(feature = "time")]
    #[test]
    fn decode_time_offset_datetime_from_timestamp() {
        let val = serde_json::json!(1710504600);
        let odt = time::OffsetDateTime::decode(make_ref(&val)).unwrap();
        assert_eq!(odt.year(), 2024);
    }

    #[test]
    fn decode_box_str() {
        let val = serde_json::json!("hello");
        let decoded = Box::<str>::decode(make_ref(&val)).unwrap();
        assert_eq!(&*decoded, "hello");
    }

    #[test]
    fn decode_box_bytes() {
        let val = serde_json::json!("AAECAw==");
        let decoded = Box::<[u8]>::decode(make_ref(&val)).unwrap();
        assert_eq!(&*decoded, &[0u8, 1, 2, 3]);
    }

    #[test]
    fn decode_cow_str() {
        let val = serde_json::json!("world");
        let decoded = std::borrow::Cow::<str>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.as_ref(), "world");
    }

    #[test]
    fn decode_ipaddr() {
        let val = serde_json::json!("192.168.1.1");
        let ip = std::net::IpAddr::decode(make_ref(&val)).unwrap();
        assert_eq!(ip.to_string(), "192.168.1.1");
    }

    #[test]
    fn decode_ipv4addr() {
        let val = serde_json::json!("10.0.0.1");
        let ip = std::net::Ipv4Addr::decode(make_ref(&val)).unwrap();
        assert_eq!(ip.to_string(), "10.0.0.1");
    }

    #[test]
    fn decode_ipv6addr() {
        let val = serde_json::json!("::1");
        let ip = std::net::Ipv6Addr::decode(make_ref(&val)).unwrap();
        assert_eq!(ip.to_string(), "::1");
    }

    #[cfg(feature = "json")]
    #[test]
    fn decode_json_wrapper() {
        use sqlx_core::types::Json;
        use std::collections::HashMap;

        let val = serde_json::json!({"key": "value"});
        let decoded = Json::<HashMap<String, String>>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.0.get("key").unwrap(), "value");
    }

    #[cfg(feature = "json")]
    #[test]
    fn decode_json_wrapper_from_string() {
        use sqlx_core::types::Json;
        use std::collections::HashMap;

        // JSON stored as a string (common in SQLite)
        let val = serde_json::json!(r#"{"key": "value"}"#);
        let decoded = Json::<HashMap<String, String>>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.0.get("key").unwrap(), "value");
    }

    #[test]
    fn decode_text_wrapper() {
        use sqlx_core::types::Text;

        let val = serde_json::json!("127.0.0.1:8080");
        let decoded = Text::<std::net::SocketAddr>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.0.to_string(), "127.0.0.1:8080");
    }

    #[test]
    fn decode_nonzero() {
        let val = serde_json::json!(42);
        let n = std::num::NonZeroI32::decode(make_ref(&val)).unwrap();
        assert_eq!(n.get(), 42);
    }

    #[test]
    fn decode_nonzero_rejects_zero() {
        let val = serde_json::json!(0);
        assert!(std::num::NonZeroI32::decode(make_ref(&val)).is_err());
    }

    #[test]
    fn decode_option_null() {
        let val = serde_json::Value::Null;
        let decoded = Option::<i32>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, None);
    }

    #[test]
    fn decode_option_some() {
        let val = serde_json::json!(42);
        let decoded = Option::<i32>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, Some(42));
    }

    #[test]
    fn decode_string_null_errors() {
        let val = serde_json::Value::Null;
        assert!(String::decode(make_ref(&val)).is_err());
    }

    #[test]
    fn decode_string_from_number() {
        let val = serde_json::json!(42);
        assert_eq!(String::decode(make_ref(&val)).unwrap(), "42");
    }

    #[test]
    fn decode_string_from_bool() {
        let val = serde_json::json!(true);
        assert_eq!(String::decode(make_ref(&val)).unwrap(), "true");
    }

    // --- Array decode tests ---

    #[test]
    fn decode_vec_i32() {
        let val = serde_json::json!([1, 2, 3]);
        let decoded = Vec::<i32>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![1, 2, 3]);
    }

    #[test]
    fn decode_vec_i64() {
        let val = serde_json::json!([10, 20, 30]);
        let decoded = Vec::<i64>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![10_i64, 20, 30]);
    }

    #[test]
    fn decode_vec_f64() {
        let val = serde_json::json!([1.5, 2.5, 3.5]);
        let decoded = Vec::<f64>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![1.5, 2.5, 3.5]);
    }

    #[test]
    fn decode_vec_bool() {
        let val = serde_json::json!([true, false, true]);
        let decoded = Vec::<bool>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![true, false, true]);
    }

    #[test]
    fn decode_vec_string() {
        let val = serde_json::json!(["hello", "world"]);
        let decoded = Vec::<String>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec!["hello", "world"]);
    }

    #[test]
    fn decode_vec_empty() {
        let val = serde_json::json!([]);
        let decoded = Vec::<i32>::decode(make_ref(&val)).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn decode_vec_option_i32() {
        let val = serde_json::json!([1, null, 3]);
        let decoded = Vec::<Option<i32>>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![Some(1), None, Some(3)]);
    }

    #[test]
    fn decode_vec_option_string() {
        let val = serde_json::json!(["a", null, "c"]);
        let decoded = Vec::<Option<String>>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![Some("a".into()), None, Some("c".into())]);
    }

    #[test]
    fn decode_vec_json_value() {
        let val = serde_json::json!([1, "two", null, true]);
        let decoded = Vec::<serde_json::Value>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.len(), 4);
        assert_eq!(decoded[0], serde_json::json!(1));
        assert_eq!(decoded[1], serde_json::json!("two"));
        assert!(decoded[2].is_null());
    }

    #[test]
    fn decode_vec_from_json_string() {
        // Array stored as a JSON-encoded string (common in SQLite)
        let val = serde_json::json!("[1,2,3]");
        let decoded = Vec::<i32>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![1, 2, 3]);
    }

    #[test]
    fn decode_vec_null_errors() {
        let val = serde_json::Value::Null;
        assert!(Vec::<i32>::decode(make_ref(&val)).is_err());
    }

    #[test]
    fn decode_vec_type_mismatch_in_array() {
        // Array with wrong element types
        let val = serde_json::json!(["not", "numbers"]);
        assert!(Vec::<i32>::decode(make_ref(&val)).is_err());
    }

}

#[cfg(test)]
mod array_roundtrip_tests {
    use sqlx_core::decode::Decode;
    use sqlx_core::encode::Encode;

    use crate::db::HttpDb;
    use crate::type_info::HttpTypeInfo;
    use crate::value::HttpValueRef;

    fn encode_value<'q, T: Encode<'q, HttpDb>>(value: T) -> serde_json::Value {
        let mut buf = Vec::new();
        let _ = value.encode(&mut buf).unwrap();
        buf.into_iter().next().unwrap()
    }

    fn make_ref(value: &serde_json::Value) -> HttpValueRef<'_> {
        HttpValueRef {
            value,
            type_info: HttpTypeInfo::from_json(value),
        }
    }

    #[test]
    fn roundtrip_vec_i32() {
        let original = vec![1_i32, 2, 3];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<i32>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_vec_string() {
        let original = vec!["hello".to_string(), "world".to_string()];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<String>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_vec_option_i32() {
        let original = vec![Some(1_i32), None, Some(3)];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<Option<i32>>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_vec_f64() {
        let original = vec![1.1_f64, 2.2, 3.3];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<f64>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_vec_bool() {
        let original = vec![true, false, true, false];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<bool>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_vec_option_string() {
        let original = vec![Some("a".to_string()), None, Some("c".to_string())];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<Option<String>>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_vec_json_value() {
        let original = vec![serde_json::json!(1), serde_json::json!("two"), serde_json::json!(null)];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<serde_json::Value>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }
}

#[cfg(test)]
mod json_array_tests {
    use std::collections::HashMap;

    use sqlx_core::decode::Decode;
    use sqlx_core::encode::Encode;

    use crate::db::HttpDb;
    use crate::type_info::HttpTypeInfo;
    use crate::types_impl::JsonArray;
    use crate::value::HttpValueRef;

    fn encode_value<'q, T: Encode<'q, HttpDb>>(value: T) -> serde_json::Value {
        let mut buf = Vec::new();
        let _ = value.encode(&mut buf).unwrap();
        buf.into_iter().next().unwrap()
    }

    fn make_ref(value: &serde_json::Value) -> HttpValueRef<'_> {
        HttpValueRef {
            value,
            type_info: HttpTypeInfo::from_json(value),
        }
    }

    // -- Custom struct via JsonArray --

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Ingredient {
        name: String,
        amount: f64,
    }

    impl JsonArray for Ingredient {}

    #[test]
    fn encode_vec_custom_struct() {
        let items = vec![
            Ingredient { name: "flour".into(), amount: 2.5 },
            Ingredient { name: "sugar".into(), amount: 1.0 },
        ];
        let encoded = encode_value(items);
        assert_eq!(
            encoded,
            serde_json::json!([
                {"name": "flour", "amount": 2.5},
                {"name": "sugar", "amount": 1.0}
            ])
        );
    }

    #[test]
    fn decode_vec_custom_struct() {
        let val = serde_json::json!([
            {"name": "flour", "amount": 2.5},
            {"name": "sugar", "amount": 1.0}
        ]);
        let decoded = Vec::<Ingredient>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![
            Ingredient { name: "flour".into(), amount: 2.5 },
            Ingredient { name: "sugar".into(), amount: 1.0 },
        ]);
    }

    #[test]
    fn roundtrip_vec_custom_struct() {
        let original = vec![
            Ingredient { name: "flour".into(), amount: 2.5 },
            Ingredient { name: "sugar".into(), amount: 1.0 },
        ];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<Ingredient>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_vec_custom_struct_from_json_string() {
        let val = serde_json::json!(r#"[{"name":"flour","amount":2.5}]"#);
        let decoded = Vec::<Ingredient>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![Ingredient { name: "flour".into(), amount: 2.5 }]);
    }

    #[test]
    fn decode_vec_custom_struct_empty() {
        let val = serde_json::json!([]);
        let decoded = Vec::<Ingredient>::decode(make_ref(&val)).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn decode_vec_custom_struct_null_errors() {
        let val = serde_json::Value::Null;
        assert!(Vec::<Ingredient>::decode(make_ref(&val)).is_err());
    }

    // -- Vec<Option<CustomStruct>> --

    #[test]
    fn encode_vec_option_custom_struct() {
        let items = vec![
            Some(Ingredient { name: "flour".into(), amount: 2.5 }),
            None,
            Some(Ingredient { name: "sugar".into(), amount: 1.0 }),
        ];
        let encoded = encode_value(items);
        assert_eq!(
            encoded,
            serde_json::json!([
                {"name": "flour", "amount": 2.5},
                null,
                {"name": "sugar", "amount": 1.0}
            ])
        );
    }

    #[test]
    fn decode_vec_option_custom_struct() {
        let val = serde_json::json!([
            {"name": "flour", "amount": 2.5},
            null,
            {"name": "sugar", "amount": 1.0}
        ]);
        let decoded = Vec::<Option<Ingredient>>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded, vec![
            Some(Ingredient { name: "flour".into(), amount: 2.5 }),
            None,
            Some(Ingredient { name: "sugar".into(), amount: 1.0 }),
        ]);
    }

    #[test]
    fn roundtrip_vec_option_custom_struct() {
        let original = vec![
            Some(Ingredient { name: "flour".into(), amount: 2.5 }),
            None,
        ];
        let encoded = encode_value(original.clone());
        let decoded = Vec::<Option<Ingredient>>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    // -- HashMap<String, V> --

    #[test]
    fn encode_hashmap_string_value() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), serde_json::json!("value"));
        map.insert("num".to_string(), serde_json::json!(42));
        let encoded = encode_value(map.clone());
        assert!(encoded.is_object());
        assert_eq!(encoded["key"], serde_json::json!("value"));
        assert_eq!(encoded["num"], serde_json::json!(42));
    }

    #[test]
    fn decode_hashmap_string_value() {
        let val = serde_json::json!({"key": "value", "num": 42});
        let decoded = HashMap::<String, serde_json::Value>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.get("key").unwrap(), &serde_json::json!("value"));
        assert_eq!(decoded.get("num").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn roundtrip_hashmap_string_value() {
        let mut original = HashMap::new();
        original.insert("a".to_string(), serde_json::json!(1));
        original.insert("b".to_string(), serde_json::json!("two"));
        let encoded = encode_value(original.clone());
        let decoded = HashMap::<String, serde_json::Value>::decode(make_ref(&encoded)).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_hashmap_string_string() {
        let val = serde_json::json!({"name": "alice", "role": "admin"});
        let decoded = HashMap::<String, String>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.get("name").unwrap(), "alice");
        assert_eq!(decoded.get("role").unwrap(), "admin");
    }

    #[test]
    fn decode_hashmap_from_json_string() {
        let val = serde_json::json!(r#"{"key": "value"}"#);
        let decoded = HashMap::<String, serde_json::Value>::decode(make_ref(&val)).unwrap();
        assert_eq!(decoded.get("key").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn decode_hashmap_null_errors() {
        let val = serde_json::Value::Null;
        assert!(HashMap::<String, serde_json::Value>::decode(make_ref(&val)).is_err());
    }

    #[test]
    fn decode_hashmap_wrong_type_errors() {
        let val = serde_json::json!([1, 2, 3]);
        assert!(HashMap::<String, serde_json::Value>::decode(make_ref(&val)).is_err());
    }

    #[test]
    fn encode_hashmap_empty() {
        let map: HashMap<String, serde_json::Value> = HashMap::new();
        let encoded = encode_value(map);
        assert_eq!(encoded, serde_json::json!({}));
    }
}

#[cfg(test)]
mod type_tests {
    use sqlx_core::types::Type;

    use crate::type_info::HttpTypeInfo;

    #[test]
    fn type_compatible_integers() {
        assert!(i32::compatible(&HttpTypeInfo::Integer));
        assert!(i32::compatible(&HttpTypeInfo::Float));
        assert!(i32::compatible(&HttpTypeInfo::Text));
        assert!(!i32::compatible(&HttpTypeInfo::Null));
    }

    #[test]
    fn type_compatible_string() {
        assert!(String::compatible(&HttpTypeInfo::Text));
        assert!(String::compatible(&HttpTypeInfo::Integer));
        assert!(!String::compatible(&HttpTypeInfo::Null));
    }

    #[test]
    fn type_compatible_json() {
        assert!(serde_json::Value::compatible(&HttpTypeInfo::Json));
        assert!(serde_json::Value::compatible(&HttpTypeInfo::Text));
        assert!(serde_json::Value::compatible(&HttpTypeInfo::Null));
    }
}

#[cfg(test)]
mod row_tests {
    use std::sync::Arc;

    use sqlx_core::row::Row;

    use crate::column::HttpColumn;
    use crate::row::HttpRow;
    use crate::type_info::HttpTypeInfo;

    fn make_row() -> HttpRow {
        let columns = Arc::new(vec![
            HttpColumn {
                name: "id".to_string(),
                ordinal: 0,
                type_info: HttpTypeInfo::Integer,
            },
            HttpColumn {
                name: "name".to_string(),
                ordinal: 1,
                type_info: HttpTypeInfo::Text,
            },
            HttpColumn {
                name: "score".to_string(),
                ordinal: 2,
                type_info: HttpTypeInfo::Float,
            },
        ]);
        HttpRow {
            columns,
            values: vec![
                serde_json::json!(1),
                serde_json::json!("alice"),
                serde_json::json!(95.5),
            ],
        }
    }

    #[test]
    fn get_by_name() {
        let row = make_row();
        let id: i64 = row.get("id");
        assert_eq!(id, 1);
        let name: String = row.get("name");
        assert_eq!(name, "alice");
    }

    #[test]
    fn get_by_index() {
        let row = make_row();
        let id: i64 = row.get(0_usize);
        assert_eq!(id, 1);
        let name: String = row.get(1_usize);
        assert_eq!(name, "alice");
    }

    #[test]
    fn try_get_missing_column() {
        let row = make_row();
        let result = row.try_get::<i64, _>("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn try_get_out_of_bounds() {
        let row = make_row();
        let result = row.try_get::<i64, _>(10_usize);
        assert!(result.is_err());
    }

    #[test]
    fn get_float() {
        let row = make_row();
        let score: f64 = row.get("score");
        assert!((score - 95.5).abs() < f64::EPSILON);
    }

    #[test]
    fn row_columns() {
        use sqlx_core::column::Column;
        let row = make_row();
        assert_eq!(row.columns().len(), 3);
        assert_eq!(row.columns()[0].name(), "id");
        assert_eq!(row.columns()[1].name(), "name");
    }
}

#[cfg(test)]
mod arguments_tests {
    use sqlx_core::arguments::Arguments;

    use crate::arguments::HttpArguments;

    #[test]
    fn add_values() {
        let mut args = HttpArguments::default();
        args.add(42_i64).unwrap();
        args.add("hello").unwrap();
        args.add(true).unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args.values[0], serde_json::json!(42));
        assert_eq!(args.values[1], serde_json::json!("hello"));
        assert_eq!(args.values[2], serde_json::json!(true));
    }
}

#[cfg(test)]
mod convert_tests {
    use crate::convert::convert_core_result;

    use sqlx_core::row::Row;

    #[test]
    fn convert_empty_result() {
        let result = db_http_core::QueryResult {
            columns: vec![],
            rows: vec![],
            affected_row_count: 5,
        };
        let (rows, qr) = convert_core_result(result);
        assert!(rows.is_empty());
        assert_eq!(qr.rows_affected(), 5);
    }

    #[test]
    fn convert_with_data() {
        let result = db_http_core::QueryResult {
            columns: vec![
                db_http_core::Column {
                    name: "id".to_string(),
                },
                db_http_core::Column {
                    name: "name".to_string(),
                },
            ],
            rows: vec![
                serde_json::json!({"id": 1, "name": "alice"}),
                serde_json::json!({"id": 2, "name": "bob"}),
            ],
            affected_row_count: 0,
        };
        let (rows, _qr) = convert_core_result(result);
        assert_eq!(rows.len(), 2);

        let id: i64 = rows[0].get("id");
        assert_eq!(id, 1);
        let name: String = rows[0].get("name");
        assert_eq!(name, "alice");

        let id: i64 = rows[1].get("id");
        assert_eq!(id, 2);
    }
}

#[cfg(test)]
mod pool_tests {
    use crate::{Pool, Turso, Neon, PlanetScale};

    #[test]
    fn connect_turso_invalid() {
        let result = Pool::<Turso>::connect("", "token");
        assert!(result.is_err());
    }

    #[test]
    fn connect_neon_invalid() {
        let result = Pool::<Neon>::connect("");
        assert!(result.is_err());
    }

    #[test]
    fn connect_planetscale_invalid() {
        let result = Pool::<PlanetScale>::connect("", "user", "pass");
        assert!(result.is_err());
    }

    #[test]
    fn connect_turso_valid() {
        let pool = Pool::<Turso>::connect("mydb.turso.io", "my-token").unwrap();
        // Just verify it creates successfully
        let _ = format!("{pool:?}");
    }

    #[test]
    fn connect_neon_valid() {
        let pool =
            Pool::<Neon>::connect("postgres://user:pass@ep-cool-name.us-east-2.aws.neon.tech/db")
                .unwrap();
        let _ = format!("{pool:?}");
    }

    #[test]
    fn connect_planetscale_valid() {
        let pool =
            Pool::<PlanetScale>::connect("aws.connect.psdb.cloud", "user", "pass").unwrap();
        let _ = format!("{pool:?}");
    }
}

#[cfg(test)]
mod query_builder_tests {
    use crate::http_query;
    use sqlx_core::executor::Execute;

    #[test]
    fn query_with_binds() {
        let mut q = http_query("SELECT * FROM users WHERE id = ? AND name = ?")
            .bind(1_i64)
            .bind("alice".to_string());

        assert_eq!(q.sql(), "SELECT * FROM users WHERE id = ? AND name = ?");

        let args = q.take_arguments().unwrap().unwrap();
        assert_eq!(args.values.len(), 2);
        assert_eq!(args.values[0], serde_json::json!(1));
        assert_eq!(args.values[1], serde_json::json!("alice"));
    }
}

#[cfg(test)]
mod transaction_tests {
    use crate::{Pool, Turso, http_query};

    #[test]
    fn transaction_add_queries() {
        let pool =
            Pool::<Turso>::connect("mydb.turso.io", "token").unwrap();
        let mut tx = pool.begin();
        tx.add(http_query("INSERT INTO users (name) VALUES (?)").bind("alice")).unwrap();
        tx.add(http_query("INSERT INTO users (name) VALUES (?)").bind("bob")).unwrap();
        // We can't commit without a real backend, but we verified the queries were added
    }
}

#[cfg(test)]
mod transaction_result_tests {
    use std::sync::Arc;

    use sqlx_core::from_row::FromRow;
    use sqlx_core::row::Row;

    use crate::column::HttpColumn;
    use crate::query_result::HttpQueryResult;
    use crate::row::HttpRow;
    use crate::transaction::{
        FromTransactionResult, FromTransactionResults, TransactionResult,
    };
    use crate::type_info::HttpTypeInfo;

    fn make_insert_result(affected: u64) -> TransactionResult {
        TransactionResult {
            rows: vec![],
            query_result: HttpQueryResult {
                rows_affected: affected,
            },
        }
    }

    fn make_select_result(data: Vec<Vec<(&str, serde_json::Value)>>) -> TransactionResult {
        if data.is_empty() {
            return TransactionResult {
                rows: vec![],
                query_result: HttpQueryResult { rows_affected: 0 },
            };
        }
        let col_names: Vec<String> = data[0].iter().map(|(n, _)| n.to_string()).collect();
        let columns: Arc<Vec<HttpColumn>> = Arc::new(
            col_names
                .iter()
                .enumerate()
                .map(|(i, name)| HttpColumn {
                    name: name.clone(),
                    ordinal: i,
                    type_info: HttpTypeInfo::Text,
                })
                .collect(),
        );
        let rows = data
            .into_iter()
            .map(|pairs| {
                let values = pairs.into_iter().map(|(_, v)| v).collect();
                HttpRow {
                    columns: columns.clone(),
                    values,
                }
            })
            .collect();
        TransactionResult {
            rows,
            query_result: HttpQueryResult { rows_affected: 0 },
        }
    }

    #[derive(Debug, PartialEq)]
    struct User {
        id: i64,
        name: String,
    }

    impl<'r> FromRow<'r, HttpRow> for User {
        fn from_row(row: &'r HttpRow) -> Result<Self, sqlx_core::error::Error> {
            Ok(User {
                id: row.get("id"),
                name: row.get("name"),
            })
        }
    }

    #[derive(Debug, PartialEq)]
    struct Log {
        id: i64,
        action: String,
    }

    impl<'r> FromRow<'r, HttpRow> for Log {
        fn from_row(row: &'r HttpRow) -> Result<Self, sqlx_core::error::Error> {
            Ok(Log {
                id: row.get("id"),
                action: row.get("action"),
            })
        }
    }

    // Layer 1: into_typed / first_typed

    #[test]
    fn into_typed_converts_all_rows() {
        let result = make_select_result(vec![
            vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            vec![("id", serde_json::json!(2)), ("name", serde_json::json!("bob"))],
        ]);
        let users: Vec<User> = result.into_typed().unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0], User { id: 1, name: "alice".into() });
        assert_eq!(users[1], User { id: 2, name: "bob".into() });
    }

    #[test]
    fn into_typed_empty() {
        let result = make_select_result(vec![]);
        let users: Vec<User> = result.into_typed().unwrap();
        assert!(users.is_empty());
    }

    #[test]
    fn first_typed_returns_first() {
        let result = make_select_result(vec![
            vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            vec![("id", serde_json::json!(2)), ("name", serde_json::json!("bob"))],
        ]);
        let user: Option<User> = result.first_typed().unwrap();
        assert_eq!(user, Some(User { id: 1, name: "alice".into() }));
    }

    #[test]
    fn first_typed_returns_none_when_empty() {
        let result = make_select_result(vec![]);
        let user: Option<User> = result.first_typed().unwrap();
        assert_eq!(user, None);
    }

    // Layer 2: FromTransactionResult trait

    #[test]
    fn from_transaction_result_passthrough() {
        let result = make_insert_result(5);
        let r = TransactionResult::from_transaction_result(result).unwrap();
        assert_eq!(r.rows_affected(), 5);
    }

    #[test]
    fn from_transaction_result_vec() {
        let result = make_select_result(vec![
            vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
        ]);
        let users = Vec::<User>::from_transaction_result(result).unwrap();
        assert_eq!(users, vec![User { id: 1, name: "alice".into() }]);
    }

    #[test]
    fn from_transaction_result_option_some() {
        let result = make_select_result(vec![
            vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
        ]);
        let user = Option::<User>::from_transaction_result(result).unwrap();
        assert_eq!(user, Some(User { id: 1, name: "alice".into() }));
    }

    #[test]
    fn from_transaction_result_option_none() {
        let result = make_select_result(vec![]);
        let user = Option::<User>::from_transaction_result(result).unwrap();
        assert_eq!(user, None);
    }

    // Layer 2: FromTransactionResults — tuple destructuring

    #[test]
    fn tuple_destructure_two() {
        let results = vec![
            make_insert_result(1),
            make_select_result(vec![
                vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            ]),
        ];
        let (insert, users): (TransactionResult, Vec<User>) =
            FromTransactionResults::from_results(results).unwrap();
        assert_eq!(insert.rows_affected(), 1);
        assert_eq!(users, vec![User { id: 1, name: "alice".into() }]);
    }

    #[test]
    fn tuple_destructure_three_mixed() {
        let results = vec![
            make_insert_result(1),
            make_select_result(vec![
                vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            ]),
            make_select_result(vec![
                vec![("id", serde_json::json!(10)), ("action", serde_json::json!("login"))],
                vec![("id", serde_json::json!(11)), ("action", serde_json::json!("logout"))],
            ]),
        ];
        let (insert, users, logs): (TransactionResult, Vec<User>, Vec<Log>) =
            FromTransactionResults::from_results(results).unwrap();
        assert_eq!(insert.rows_affected(), 1);
        assert_eq!(users.len(), 1);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0], Log { id: 10, action: "login".into() });
    }

    #[test]
    fn tuple_destructure_with_option() {
        let results = vec![
            make_insert_result(1),
            make_select_result(vec![
                vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            ]),
        ];
        let (_, user): (TransactionResult, Option<User>) =
            FromTransactionResults::from_results(results).unwrap();
        assert_eq!(user, Some(User { id: 1, name: "alice".into() }));
    }

    #[test]
    fn tuple_destructure_count_mismatch_too_few() {
        let results = vec![make_insert_result(1)];
        let err = <(TransactionResult, Vec<User>)>::from_results(results);
        assert!(err.is_err());
    }

    #[test]
    fn tuple_destructure_count_mismatch_too_many() {
        let results = vec![make_insert_result(1), make_insert_result(2), make_insert_result(3)];
        let err = <(TransactionResult, TransactionResult)>::from_results(results);
        assert!(err.is_err());
    }

    // TransactionResult basic methods

    #[test]
    fn rows_affected_on_insert() {
        let result = make_insert_result(42);
        assert_eq!(result.rows_affected(), 42);
    }

    #[test]
    fn rows_returns_slice() {
        let result = make_select_result(vec![
            vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            vec![("id", serde_json::json!(2)), ("name", serde_json::json!("bob"))],
        ]);
        assert_eq!(result.rows().len(), 2);
        let id: i64 = result.rows()[0].get("id");
        assert_eq!(id, 1);
    }

    #[test]
    fn into_rows_returns_vec() {
        let result = make_select_result(vec![
            vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
        ]);
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1);
        let name: String = rows[0].get("name");
        assert_eq!(name, "alice");
    }

    // 1-tuple destructuring

    #[test]
    fn tuple_destructure_one() {
        let results = vec![
            make_select_result(vec![
                vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            ]),
        ];
        let (users,): (Vec<User>,) = FromTransactionResults::from_results(results).unwrap();
        assert_eq!(users, vec![User { id: 1, name: "alice".into() }]);
    }

    // All-TransactionResult tuple

    #[test]
    fn tuple_destructure_all_passthrough() {
        let results = vec![make_insert_result(1), make_insert_result(5)];
        let (r1, r2): (TransactionResult, TransactionResult) =
            FromTransactionResults::from_results(results).unwrap();
        assert_eq!(r1.rows_affected(), 1);
        assert_eq!(r2.rows_affected(), 5);
    }

    // Option with multiple rows returns first only

    #[test]
    fn option_with_multiple_rows_returns_first() {
        let result = make_select_result(vec![
            vec![("id", serde_json::json!(1)), ("name", serde_json::json!("alice"))],
            vec![("id", serde_json::json!(2)), ("name", serde_json::json!("bob"))],
        ]);
        let user = Option::<User>::from_transaction_result(result).unwrap();
        assert_eq!(user, Some(User { id: 1, name: "alice".into() }));
    }
}
