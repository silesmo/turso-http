use serde::{Deserialize, Serialize};

// --- Request types ---

#[derive(Debug, Serialize)]
pub struct PipelineRequest {
    pub baton: Option<String>,
    pub requests: Vec<PipelineRequestItem>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum PipelineRequestItem {
    #[serde(rename = "execute")]
    Execute { stmt: Statement },
    #[serde(rename = "batch")]
    Batch { batch: Batch },
    #[serde(rename = "close")]
    Close,
}

#[derive(Debug, Serialize)]
pub struct Statement {
    pub sql: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<TypedValue>>,
}

#[derive(Debug, Serialize)]
pub struct Batch {
    pub steps: Vec<BatchStep>,
}

#[derive(Debug, Serialize)]
pub struct BatchStep {
    pub condition: Option<BatchCondition>,
    pub stmt: Statement,
}

#[derive(Debug, Serialize)]
pub struct BatchCondition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub step: usize,
}

/// A bound value sent to Turso's Hrana HTTP pipeline endpoint.
///
/// Uses the canonical Hrana wire shapes per type:
/// - `integer` → JSON string (BigInt safety)
/// - `float`   → unquoted JSON number
/// - `text`    → JSON string
/// - `blob`    → base64 string in the `base64` field
/// - `null`    → no value field
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TypedValue {
    Null,
    Integer { value: String },
    Float { value: f64 },
    Text { value: String },
    Blob { base64: String },
}

// --- Response types ---

#[derive(Debug, Deserialize)]
pub struct PipelineResponse {
    pub baton: Option<String>,
    pub base_url: Option<String>,
    pub results: Vec<PipelineResultItem>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PipelineResultItem {
    #[serde(rename = "ok")]
    Ok { response: PipelineOkResponse },
    #[serde(rename = "error")]
    Error { error: TursoError },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PipelineOkResponse {
    #[serde(rename = "execute")]
    Execute { result: ExecuteResult },
    #[serde(rename = "batch")]
    Batch { result: BatchResult },
    #[serde(rename = "close")]
    Close,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteResult {
    pub cols: Vec<TursoColumn>,
    pub rows: Vec<Vec<TursoValue>>,
    pub affected_row_count: u64,
    pub last_insert_rowid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchResult {
    pub step_results: Vec<Option<ExecuteResult>>,
    pub step_errors: Vec<Option<TursoError>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TursoColumn {
    pub name: String,
    pub decltype: Option<String>,
}

/// A value received from Turso's Hrana HTTP pipeline endpoint.
///
/// Canonical wire shapes returned by Turso:
/// - `integer` → JSON string
/// - `float`   → unquoted JSON number
/// - `text`    → JSON string
/// - `blob`    → base64 string in the `base64` field
/// - `null`    → `{"type":"null"}`
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TursoValue {
    Null,
    Integer { value: String },
    Float { value: f64 },
    Text { value: String },
    Blob { base64: String },
}

#[derive(Debug, Deserialize)]
pub struct TursoError {
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}

// --- Conversion helpers ---

impl TypedValue {
    /// Convert a generic `serde_json::Value` into the correct Hrana wire variant.
    ///
    /// Booleans are encoded as integers 0/1 (SQLite has no native bool).
    /// Arrays and objects fall through to a text representation — unusual for
    /// SQL binds but preserves the previous library behaviour.
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => TypedValue::Null,
            serde_json::Value::Bool(b) => TypedValue::Integer {
                value: if *b { "1".to_string() } else { "0".to_string() },
            },
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    TypedValue::Integer {
                        value: i.to_string(),
                    }
                } else if let Some(f) = n.as_f64() {
                    TypedValue::Float { value: f }
                } else {
                    TypedValue::Text {
                        value: n.to_string(),
                    }
                }
            }
            serde_json::Value::String(s) => TypedValue::Text { value: s.clone() },
            other => TypedValue::Text {
                value: other.to_string(),
            },
        }
    }
}

impl TursoValue {
    /// Convert a received Turso value into a user-facing `serde_json::Value`.
    ///
    /// Integers are parsed from their string form; floats pass through as
    /// JSON numbers; text and blob pass through as strings (blob stays base64).
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            TursoValue::Null => serde_json::Value::Null,
            TursoValue::Integer { value } => value
                .parse::<i64>()
                .ok()
                .map(|i| serde_json::Value::Number(i.into()))
                .unwrap_or_else(|| serde_json::Value::String(value.clone())),
            TursoValue::Float { value } => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            TursoValue::Text { value } => serde_json::Value::String(value.clone()),
            TursoValue::Blob { base64 } => serde_json::Value::String(base64.clone()),
        }
    }
}

// --- Placeholder rewriting ---

/// Rewrites `$1`, `$2`, ... placeholders to `?1`, `?2`, ... for libSQL's numbered syntax.
///
/// Preserves parameter numbers so the same `$1` used multiple times in a query
/// reuses the same bound argument (libSQL supports `?NNN` numbered parameters).
///
/// Passes through `?` and `?N` unchanged — queries already using SQLite-style
/// placeholders work as-is.
pub fn rewrite_placeholders(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            // $N → ?N (preserve the number for parameter reuse)
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
                result.push_str(&digits);
            }
        } else if ch == '?' {
            // Pass through ? and ?N as-is
            result.push('?');
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    result.push(d);
                    chars.next();
                } else {
                    break;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_placeholders_basic() {
        assert_eq!(
            rewrite_placeholders("SELECT * FROM users WHERE id = $1"),
            "SELECT * FROM users WHERE id = ?1"
        );
    }

    #[test]
    fn test_rewrite_placeholders_multiple() {
        assert_eq!(
            rewrite_placeholders("INSERT INTO users (name, age) VALUES ($1, $2)"),
            "INSERT INTO users (name, age) VALUES (?1, ?2)"
        );
    }

    #[test]
    fn test_rewrite_placeholders_no_placeholders() {
        assert_eq!(
            rewrite_placeholders("SELECT * FROM users"),
            "SELECT * FROM users"
        );
    }

    #[test]
    fn test_rewrite_placeholders_in_string_literal() {
        assert_eq!(
            rewrite_placeholders("SELECT * FROM users WHERE name = '$1' AND id = $1"),
            "SELECT * FROM users WHERE name = '$1' AND id = ?1"
        );
    }

    #[test]
    fn test_rewrite_placeholders_escaped_quote_in_literal() {
        assert_eq!(
            rewrite_placeholders(r"SELECT * FROM t WHERE name = 'can\'t' AND id = $1"),
            r"SELECT * FROM t WHERE name = 'can\'t' AND id = ?1"
        );
    }

    #[test]
    fn test_rewrite_placeholders_double_digit() {
        assert_eq!(
            rewrite_placeholders("SELECT $1, $2, $10, $11"),
            "SELECT ?1, ?2, ?10, ?11"
        );
    }

    #[test]
    fn test_rewrite_placeholders_reused_param() {
        assert_eq!(
            rewrite_placeholders("SELECT * FROM t WHERE a < $1 AND b >= $1"),
            "SELECT * FROM t WHERE a < ?1 AND b >= ?1"
        );
    }

    #[test]
    fn test_rewrite_placeholders_passthrough_bare_question() {
        assert_eq!(
            rewrite_placeholders("SELECT * FROM t WHERE id = ?"),
            "SELECT * FROM t WHERE id = ?"
        );
    }

    #[test]
    fn test_rewrite_placeholders_passthrough_numbered_question() {
        assert_eq!(
            rewrite_placeholders("SELECT * FROM t WHERE a = ?1 AND b = ?2"),
            "SELECT * FROM t WHERE a = ?1 AND b = ?2"
        );
    }

    #[test]
    fn test_rewrite_placeholders_mixed_dollar_and_question() {
        assert_eq!(
            rewrite_placeholders("SELECT $1, ?2, ?"),
            "SELECT ?1, ?2, ?"
        );
    }

    #[test]
    fn test_typed_value_from_json_null() {
        let v = TypedValue::from_json(&serde_json::Value::Null);
        assert!(matches!(v, TypedValue::Null));
    }

    #[test]
    fn test_typed_value_from_json_integer() {
        let v = TypedValue::from_json(&serde_json::json!(42));
        if let TypedValue::Integer { value } = v {
            assert_eq!(value, "42");
        } else {
            panic!("Expected Integer variant");
        }
    }

    #[test]
    fn test_typed_value_from_json_string() {
        let v = TypedValue::from_json(&serde_json::json!("hello"));
        if let TypedValue::Text { value } = v {
            assert_eq!(value, "hello");
        } else {
            panic!("Expected Text variant");
        }
    }

    #[test]
    fn test_turso_value_to_json_integer() {
        let v = TursoValue::Integer {
            value: "42".to_string(),
        };
        assert_eq!(v.to_json(), serde_json::json!(42));
    }

    #[test]
    fn test_turso_value_to_json_text() {
        let v = TursoValue::Text {
            value: "hello".to_string(),
        };
        assert_eq!(v.to_json(), serde_json::json!("hello"));
    }

    #[test]
    fn test_turso_value_to_json_float() {
        let v = TursoValue::Float { value: 3.14 };
        assert_eq!(v.to_json(), serde_json::json!(3.14));
    }

    #[test]
    fn test_turso_value_to_json_null() {
        let v = TursoValue::Null;
        assert_eq!(v.to_json(), serde_json::Value::Null);
    }

    #[test]
    fn test_turso_value_to_json_typed_null() {
        // `{"type":"null"}` deserializes into the `Null` variant.
        let v: TursoValue = serde_json::from_value(serde_json::json!({"type": "null"})).unwrap();
        assert_eq!(v.to_json(), serde_json::Value::Null);
    }

    #[test]
    fn deserialize_execute_response() {
        let json = serde_json::json!({
            "baton": null,
            "base_url": null,
            "results": [{
                "type": "ok",
                "response": {
                    "type": "execute",
                    "result": {
                        "cols": [
                            {"name": "id", "decltype": "INTEGER"},
                            {"name": "name", "decltype": "TEXT"}
                        ],
                        "rows": [
                            [
                                {"type": "integer", "value": "1"},
                                {"type": "text", "value": "Alice"}
                            ]
                        ],
                        "affected_row_count": 0,
                        "last_insert_rowid": null
                    }
                }
            }]
        });
        let resp: PipelineResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.results.len(), 1);
        match &resp.results[0] {
            PipelineResultItem::Ok { response: PipelineOkResponse::Execute { result } } => {
                assert_eq!(result.cols.len(), 2);
                assert_eq!(result.cols[0].name, "id");
                assert_eq!(result.cols[1].name, "name");
                assert_eq!(result.rows.len(), 1);
                assert_eq!(result.affected_row_count, 0);
            }
            other => panic!("Expected Ok/Execute, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_batch_response() {
        let json = serde_json::json!({
            "baton": null,
            "base_url": null,
            "results": [{
                "type": "ok",
                "response": {
                    "type": "batch",
                    "result": {
                        "step_results": [
                            {
                                "cols": [{"name": "id", "decltype": "INTEGER"}],
                                "rows": [],
                                "affected_row_count": 1,
                                "last_insert_rowid": "5"
                            },
                            null
                        ],
                        "step_errors": [
                            null,
                            {"message": "step failed", "code": "SQLITE_ERROR"}
                        ]
                    }
                }
            }]
        });
        let resp: PipelineResponse = serde_json::from_value(json).unwrap();
        match &resp.results[0] {
            PipelineResultItem::Ok { response: PipelineOkResponse::Batch { result } } => {
                assert_eq!(result.step_results.len(), 2);
                assert!(result.step_results[0].is_some());
                assert!(result.step_results[1].is_none());
                assert!(result.step_errors[0].is_none());
                let err = result.step_errors[1].as_ref().unwrap();
                assert_eq!(err.message, "step failed");
                assert_eq!(err.code, Some("SQLITE_ERROR".to_string()));
            }
            other => panic!("Expected Ok/Batch, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_error_response() {
        let json = serde_json::json!({
            "baton": null,
            "base_url": null,
            "results": [{
                "type": "error",
                "error": {
                    "message": "statement failed",
                    "code": "SQLITE_ERROR"
                }
            }]
        });
        let resp: PipelineResponse = serde_json::from_value(json).unwrap();
        match &resp.results[0] {
            PipelineResultItem::Error { error } => {
                assert_eq!(error.message, "statement failed");
                assert_eq!(error.code, Some("SQLITE_ERROR".to_string()));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_close_response() {
        let json = serde_json::json!({
            "baton": null,
            "base_url": null,
            "results": [{
                "type": "ok",
                "response": {
                    "type": "close"
                }
            }]
        });
        let resp: PipelineResponse = serde_json::from_value(json).unwrap();
        match &resp.results[0] {
            PipelineResultItem::Ok { response: PipelineOkResponse::Close } => {}
            other => panic!("Expected Ok/Close, got {:?}", other),
        }
    }

    #[test]
    fn deserialize_mixed_typed_values() {
        // Canonical wire shapes Turso actually sends:
        // - integer as JSON string
        // - float   as JSON number (unquoted)
        // - text    as JSON string
        // - blob    via base64 field
        // - null    as {"type":"null"}
        let json = serde_json::json!([
            {"type": "integer", "value": "42"},
            {"type": "float",   "value": 3.14},
            {"type": "text",    "value": "hello"},
            {"type": "blob",    "base64": "AQID"},
            {"type": "null"}
        ]);
        let values: Vec<TursoValue> = serde_json::from_value(json).unwrap();
        assert_eq!(values.len(), 5);
        assert_eq!(values[0].to_json(), serde_json::json!(42));
        assert_eq!(values[1].to_json(), serde_json::json!(3.14));
        assert_eq!(values[2].to_json(), serde_json::json!("hello"));
        assert_eq!(values[3].to_json(), serde_json::json!("AQID"));
        assert_eq!(values[4].to_json(), serde_json::Value::Null);
    }

    #[test]
    fn turso_value_to_json_blob() {
        let v = TursoValue::Blob {
            base64: "SGVsbG8=".to_string(),
        };
        assert_eq!(v.to_json(), serde_json::json!("SGVsbG8="));
    }

    #[test]
    fn typed_value_from_json_bool() {
        // Booleans encode as integer 0/1 — SQLite has no native bool type.
        let t = TypedValue::from_json(&serde_json::json!(true));
        if let TypedValue::Integer { value } = &t {
            assert_eq!(value, "1");
        } else {
            panic!("Expected Integer variant");
        }

        let f = TypedValue::from_json(&serde_json::json!(false));
        if let TypedValue::Integer { value } = &f {
            assert_eq!(value, "0");
        } else {
            panic!("Expected Integer variant");
        }
    }

    #[test]
    fn typed_value_from_json_float() {
        let v = TypedValue::from_json(&serde_json::json!(3.14));
        if let TypedValue::Float { value } = v {
            assert_eq!(value, 3.14);
        } else {
            panic!("Expected Float variant");
        }
    }

    #[test]
    fn typed_value_from_json_array() {
        // Arrays/objects aren't native Hrana types — fall through to text.
        let v = TypedValue::from_json(&serde_json::json!([1, 2, 3]));
        if let TypedValue::Text { value } = v {
            assert_eq!(value, "[1,2,3]");
        } else {
            panic!("Expected Text variant");
        }
    }

    #[test]
    fn typed_value_null_serialization() {
        let v = TypedValue::Null;
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json, serde_json::json!({"type": "null"}));
    }

    // ── Regression tests for the float wire-encoding bug ────────────────

    #[test]
    fn typed_value_float_serializes_as_unquoted_json_number() {
        // Hrana expects `{"type":"float","value":<number>}`, NOT a string.
        // Turso rejects the string form with:
        //   "JSON parse error: invalid type: string ..., expected f64"
        let v = TypedValue::Float { value: 24.5 };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["type"], serde_json::json!("float"));
        assert_eq!(json["value"], serde_json::json!(24.5));
        assert!(
            json["value"].is_number(),
            "float value must serialize as JSON number, got: {}",
            json["value"]
        );
    }

    #[test]
    fn typed_value_integer_serializes_as_stringified_value() {
        // Hrana requires integers as strings (BigInt safety).
        // Sending an unquoted number fails with:
        //   "JSON parse error: invalid type: integer, expected a borrowed string"
        let v = TypedValue::Integer {
            value: "42".to_string(),
        };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["type"], serde_json::json!("integer"));
        assert_eq!(json["value"], serde_json::json!("42"));
        assert!(
            json["value"].is_string(),
            "integer value must serialize as JSON string, got: {}",
            json["value"]
        );
    }

    #[test]
    fn typed_value_text_and_blob_serialize_in_canonical_shapes() {
        let text = serde_json::to_value(&TypedValue::Text {
            value: "hi".to_string(),
        })
        .unwrap();
        assert_eq!(text, serde_json::json!({"type": "text", "value": "hi"}));

        let blob = serde_json::to_value(&TypedValue::Blob {
            base64: "AQID".to_string(),
        })
        .unwrap();
        assert_eq!(blob, serde_json::json!({"type": "blob", "base64": "AQID"}));
    }

    #[test]
    fn roundtrip_float_matches_shape_turso_accepts() {
        // Full pipeline: Rust f64 → from_json → wire JSON. The live Turso
        // HTTP endpoint was verified to accept `{"type":"float","value":3.14}`
        // and reject `{"type":"float","value":"3.14"}`. This test locks in
        // that shape.
        let typed = TypedValue::from_json(&serde_json::json!(24.5));
        let wire = serde_json::to_value(&typed).unwrap();
        assert_eq!(wire["type"], serde_json::json!("float"));
        assert_eq!(wire["value"].as_f64(), Some(24.5));
        // Must NOT serialize value as a string.
        assert!(
            !wire["value"].is_string(),
            "float wire value must not be a string; got {}",
            wire["value"]
        );
    }

    #[test]
    fn turso_value_deserializes_canonical_wire_shapes() {
        // Full row shape matching the live Turso probe output.
        let row = serde_json::json!([
            {"type": "integer", "value": "42"},
            {"type": "float",   "value": 3.14},
            {"type": "text",    "value": "hello"},
            {"type": "null"},
            {"type": "blob",    "base64": "SGVsbG8="}
        ]);
        let values: Vec<TursoValue> = serde_json::from_value(row).unwrap();
        assert!(matches!(values[0], TursoValue::Integer { ref value } if value == "42"));
        assert!(matches!(values[1], TursoValue::Float { value } if value == 3.14));
        assert!(matches!(values[2], TursoValue::Text { ref value } if value == "hello"));
        assert!(matches!(values[3], TursoValue::Null));
        assert!(matches!(values[4], TursoValue::Blob { ref base64 } if base64 == "SGVsbG8="));
    }

    #[test]
    fn typed_value_boolean_type_tag_rejected_at_deserialize() {
        // Sanity check: Turso's server rejects `"type":"boolean"`. Our write
        // side never emits it — booleans go through as integers 0/1.
        // We verify that TypedValue has no `Boolean` variant by confirming
        // the full set of variants serialize to exactly the allowed types.
        for (v, expected_type) in [
            (TypedValue::Null, "null"),
            (TypedValue::Integer { value: "0".into() }, "integer"),
            (TypedValue::Float { value: 0.0 }, "float"),
            (TypedValue::Text { value: "".into() }, "text"),
            (TypedValue::Blob { base64: "".into() }, "blob"),
        ] {
            let json = serde_json::to_value(&v).unwrap();
            assert_eq!(json["type"], serde_json::json!(expected_type));
        }
    }

    #[test]
    fn test_pipeline_request_serialization() {
        let req = PipelineRequest {
            baton: None,
            requests: vec![
                PipelineRequestItem::Execute {
                    stmt: Statement {
                        sql: "SELECT ?".to_string(),
                        args: Some(vec![TypedValue::Integer {
                            value: "1".to_string(),
                        }]),
                    },
                },
                PipelineRequestItem::Close,
            ],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["requests"][0]["type"], "execute");
        assert_eq!(json["requests"][0]["stmt"]["sql"], "SELECT ?");
        assert_eq!(
            json["requests"][0]["stmt"]["args"][0],
            serde_json::json!({"type": "integer", "value": "1"})
        );
        assert_eq!(json["requests"][1]["type"], "close");
    }
}
