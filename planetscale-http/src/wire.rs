use base64::Engine;
use serde::{Deserialize, Serialize};

// --- Request types ---

#[derive(Debug, Serialize)]
pub struct PsQueryRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<serde_json::Value>,
}

// --- Response types ---

#[derive(Debug, Deserialize)]
pub struct PsQueryResponse {
    #[serde(default)]
    pub session: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<PsQueryResult>,
    #[serde(default)]
    pub error: Option<PsError>,
    #[serde(default)]
    pub timing: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PsQueryResult {
    #[serde(default)]
    pub rows_affected: Option<String>,
    #[serde(default)]
    pub insert_id: Option<String>,
    #[serde(default)]
    pub fields: Option<Vec<PsField>>,
    #[serde(default)]
    pub rows: Option<Vec<PsRow>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PsField {
    pub name: String,
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PsRow {
    pub lengths: Vec<String>,
    #[serde(default)]
    pub values: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PsError {
    pub message: String,
    #[serde(default)]
    pub code: Option<u32>,
}

// --- Helpers ---

/// Decode a PlanetScale row into individual field values.
///
/// Each row has `lengths` (string-encoded ints, negative = null) and `values`
/// (base64-encoded concatenated field values). We split by lengths.
pub fn decode_row(row: &PsRow) -> Result<Vec<Option<String>>, String> {
    let raw = match &row.values {
        Some(v) => base64::engine::general_purpose::STANDARD
            .decode(v)
            .map_err(|e| format!("Failed to decode base64 row data: {e}"))?,
        None => Vec::new(),
    };

    let mut offset: usize = 0;
    let mut values = Vec::with_capacity(row.lengths.len());

    for (i, len_str) in row.lengths.iter().enumerate() {
        let len: i64 = len_str
            .parse()
            .map_err(|e| format!("Invalid length at index {i}: {e}"))?;
        if len < 0 {
            values.push(None);
        } else {
            let n = len as usize;
            if offset + n > raw.len() {
                return Err(format!(
                    "Row data too short: need {} bytes at offset {}, but only {} bytes available",
                    n,
                    offset,
                    raw.len()
                ));
            }
            let value = String::from_utf8_lossy(&raw[offset..offset + n]).to_string();
            offset += n;
            values.push(Some(value));
        }
    }

    Ok(values)
}

/// Cast a decoded string value to an appropriate JSON value based on the field type.
pub fn cast_value(field: &PsField, value: Option<String>) -> serde_json::Value {
    let val = match value {
        Some(v) => v,
        None => return serde_json::Value::Null,
    };

    let type_name = field.type_.as_deref().unwrap_or("");
    let upper = type_name.to_uppercase();

    match upper.as_str() {
        "INT8" | "INT16" | "INT24" | "INT32" | "INT64" | "UINT8" | "UINT16" | "UINT24"
        | "UINT32" | "UINT64" | "YEAR" => {
            if let Ok(n) = val.parse::<i64>() {
                serde_json::Value::Number(n.into())
            } else {
                serde_json::Value::String(val)
            }
        }
        "FLOAT32" | "FLOAT64" | "DECIMAL" => {
            if let Some(n) = serde_json::Number::from_f64(val.parse::<f64>().unwrap_or(0.0)) {
                serde_json::Value::Number(n)
            } else {
                serde_json::Value::String(val)
            }
        }
        "JSON" => serde_json::from_str(&val).unwrap_or(serde_json::Value::String(val)),
        _ => serde_json::Value::String(val),
    }
}

/// Rewrites `$1`, `$2`, ... placeholders to `?` for PlanetScale's positional syntax.
/// Skips `$N` inside single-quoted string literals.
fn rewrite_dollar_placeholders(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            // Check if followed by digits
            let mut digits = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    digits.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if digits.is_empty() {
                result.push('$');
            } else {
                result.push('?');
            }
        } else if ch == '\'' {
            // Skip string literals, handling backslash-escaped quotes
            result.push(ch);
            let mut escaped = false;
            for c in chars.by_ref() {
                result.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '\'' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Format a SQL query with inline parameters for PlanetScale.
///
/// Supports both `?` and `$1, $2, ...` placeholder styles.
/// `$N` placeholders are first rewritten to `?`, then `?` placeholders are
/// replaced with the corresponding parameter value:
/// - null → NULL
/// - number → as-is
/// - bool → 1 / 0
/// - string → single-quoted with basic escaping
pub fn format_query(sql: &str, params: &[serde_json::Value]) -> Result<String, String> {
    // First pass: rewrite $1, $2, ... to ?
    let sql = rewrite_dollar_placeholders(sql);

    let placeholder_count = sql.chars().filter(|&c| c == '?').count();
    if placeholder_count != params.len() {
        return Err(format!(
            "Parameter count mismatch: query has {} placeholders but {} parameters were supplied",
            placeholder_count,
            params.len()
        ));
    }

    // Second pass: inline ? with formatted params
    let mut result = String::with_capacity(sql.len());
    let mut param_idx = 0;

    for ch in sql.chars() {
        if ch == '?' {
            result.push_str(&format_param(&params[param_idx]));
            param_idx += 1;
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn format_param(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
            format!("'{}'", escaped)
        }
        other => {
            let s = other.to_string();
            let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
            format!("'{}'", escaped)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_row_basic() {
        // "helloworld" base64 = "aGVsbG93b3JsZA=="
        let row = PsRow {
            lengths: vec!["5".to_string(), "5".to_string()],
            values: Some(
                base64::engine::general_purpose::STANDARD.encode("helloworld"),
            ),
        };
        let values = decode_row(&row).unwrap();
        assert_eq!(values, vec![Some("hello".to_string()), Some("world".to_string())]);
    }

    #[test]
    fn decode_row_with_null() {
        // "hello" base64
        let row = PsRow {
            lengths: vec!["5".to_string(), "-1".to_string()],
            values: Some(
                base64::engine::general_purpose::STANDARD.encode("hello"),
            ),
        };
        let values = decode_row(&row).unwrap();
        assert_eq!(values, vec![Some("hello".to_string()), None]);
    }

    #[test]
    fn decode_row_empty() {
        let row = PsRow {
            lengths: vec!["-1".to_string()],
            values: None,
        };
        let values = decode_row(&row).unwrap();
        assert_eq!(values, vec![None]);
    }

    #[test]
    fn cast_value_int() {
        let field = PsField {
            name: "id".to_string(),
            type_: Some("INT64".to_string()),
            table: None,
        };
        assert_eq!(cast_value(&field, Some("42".to_string())), serde_json::json!(42));
    }

    #[test]
    fn cast_value_float() {
        let field = PsField {
            name: "price".to_string(),
            type_: Some("FLOAT64".to_string()),
            table: None,
        };
        assert_eq!(cast_value(&field, Some("3.14".to_string())), serde_json::json!(3.14));
    }

    #[test]
    fn cast_value_decimal() {
        let field = PsField {
            name: "amount".to_string(),
            type_: Some("DECIMAL".to_string()),
            table: None,
        };
        assert_eq!(cast_value(&field, Some("99.99".to_string())), serde_json::json!(99.99));
    }

    #[test]
    fn cast_value_json() {
        let field = PsField {
            name: "data".to_string(),
            type_: Some("JSON".to_string()),
            table: None,
        };
        let result = cast_value(&field, Some(r#"{"key":"val"}"#.to_string()));
        assert_eq!(result, serde_json::json!({"key": "val"}));
    }

    #[test]
    fn cast_value_string() {
        let field = PsField {
            name: "name".to_string(),
            type_: Some("VARCHAR".to_string()),
            table: None,
        };
        assert_eq!(
            cast_value(&field, Some("hello".to_string())),
            serde_json::json!("hello")
        );
    }

    #[test]
    fn cast_value_null() {
        let field = PsField {
            name: "x".to_string(),
            type_: Some("INT64".to_string()),
            table: None,
        };
        assert_eq!(cast_value(&field, None), serde_json::Value::Null);
    }

    #[test]
    fn format_query_basic() {
        let sql = "SELECT * FROM users WHERE id = ? AND name = ?";
        let params = vec![serde_json::json!(1), serde_json::json!("alice")];
        assert_eq!(
            format_query(sql, &params).unwrap(),
            "SELECT * FROM users WHERE id = 1 AND name = 'alice'"
        );
    }

    #[test]
    fn format_query_null() {
        assert_eq!(
            format_query("INSERT INTO t VALUES (?)", &[serde_json::Value::Null]).unwrap(),
            "INSERT INTO t VALUES (NULL)"
        );
    }

    #[test]
    fn format_query_bool() {
        assert_eq!(
            format_query("SELECT ?", &[serde_json::json!(true)]).unwrap(),
            "SELECT 1"
        );
        assert_eq!(
            format_query("SELECT ?", &[serde_json::json!(false)]).unwrap(),
            "SELECT 0"
        );
    }

    #[test]
    fn format_query_escapes_string() {
        assert_eq!(
            format_query("SELECT ?", &[serde_json::json!("it's a test")]).unwrap(),
            "SELECT 'it\\'s a test'"
        );
    }

    #[test]
    fn format_query_no_params() {
        assert_eq!(format_query("SELECT 1", &[]).unwrap(), "SELECT 1");
    }

    #[test]
    fn format_query_param_count_mismatch() {
        let err = format_query("SELECT ? AND ?", &[serde_json::json!(1)]).unwrap_err();
        assert!(err.contains("mismatch"));
    }

    #[test]
    fn decode_row_invalid_base64() {
        let row = PsRow {
            lengths: vec!["5".to_string()],
            values: Some("not-valid-base64!!!".to_string()),
        };
        let err = decode_row(&row).unwrap_err();
        assert!(err.contains("base64"));
    }

    #[test]
    fn decode_row_data_too_short() {
        let row = PsRow {
            lengths: vec!["100".to_string()],
            values: Some(
                base64::engine::general_purpose::STANDARD.encode("short"),
            ),
        };
        let err = decode_row(&row).unwrap_err();
        assert!(err.contains("too short"));
    }

    #[test]
    fn deserialize_response_with_result() {
        let json = serde_json::json!({
            "session": {"id": "abc"},
            "result": {
                "rowsAffected": "0",
                "insertId": "0",
                "fields": [
                    {"name": "id", "type": "INT64"},
                    {"name": "name", "type": "VARCHAR"}
                ],
                "rows": [
                    {
                        "lengths": ["1", "5"],
                        "values": base64::engine::general_purpose::STANDARD.encode("1alice")
                    }
                ]
            },
            "timing": 0.001
        });
        let resp: PsQueryResponse = serde_json::from_value(json).unwrap();
        assert!(resp.session.is_some());
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result.rows_affected, Some("0".to_string()));
        let fields = result.fields.unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "id");
        assert_eq!(fields[0].type_, Some("INT64".to_string()));
        let rows = result.rows.unwrap();
        assert_eq!(rows.len(), 1);
        let decoded = decode_row(&rows[0]).unwrap();
        assert_eq!(decoded, vec![Some("1".to_string()), Some("alice".to_string())]);
    }

    #[test]
    fn deserialize_response_with_error() {
        let json = serde_json::json!({
            "session": null,
            "error": {
                "message": "table not found",
                "code": 1146
            }
        });
        let resp: PsQueryResponse = serde_json::from_value(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.message, "table not found");
        assert_eq!(err.code, Some(1146));
    }

    #[test]
    fn deserialize_field_minimal() {
        let json = serde_json::json!({"name": "col"});
        let field: PsField = serde_json::from_value(json).unwrap();
        assert_eq!(field.name, "col");
        assert_eq!(field.type_, None);
        assert_eq!(field.table, None);
    }

    #[test]
    fn format_query_dollar_placeholders() {
        let sql = "SELECT * FROM users WHERE id = $1 AND name = $2";
        let params = vec![serde_json::json!(1), serde_json::json!("alice")];
        assert_eq!(
            format_query(sql, &params).unwrap(),
            "SELECT * FROM users WHERE id = 1 AND name = 'alice'"
        );
    }

    #[test]
    fn format_query_dollar_in_string_literal() {
        let sql = "SELECT * FROM users WHERE name = '$1' AND id = $1";
        let params = vec![serde_json::json!(1)];
        assert_eq!(
            format_query(sql, &params).unwrap(),
            "SELECT * FROM users WHERE name = '$1' AND id = 1"
        );
    }

    #[test]
    fn format_query_mixed_placeholders() {
        // ? style still works as before
        let sql = "SELECT * FROM users WHERE id = ? AND name = ?";
        let params = vec![serde_json::json!(1), serde_json::json!("alice")];
        assert_eq!(
            format_query(sql, &params).unwrap(),
            "SELECT * FROM users WHERE id = 1 AND name = 'alice'"
        );
    }

    #[test]
    fn format_query_escaped_quote_in_literal() {
        let sql = r"SELECT * FROM t WHERE name = 'can\'t' AND id = $1";
        let params = vec![serde_json::json!(1)];
        assert_eq!(
            format_query(sql, &params).unwrap(),
            r"SELECT * FROM t WHERE name = 'can\'t' AND id = 1"
        );
    }
}
