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

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TypedValue {
    Null {
        #[serde(rename = "type")]
        value_type: &'static str,
    },
    Value {
        #[serde(rename = "type")]
        value_type: String,
        value: String,
    },
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

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TursoValue {
    Null,
    Typed {
        #[serde(rename = "type")]
        value_type: String,
        value: Option<String>,
        #[serde(rename = "base64")]
        base64: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct TursoError {
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}

// --- Conversion helpers ---

impl TypedValue {
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => TypedValue::Null { value_type: "null" },
            serde_json::Value::Bool(b) => TypedValue::Value {
                value_type: "integer".to_string(),
                value: if *b { "1".to_string() } else { "0".to_string() },
            },
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    TypedValue::Value {
                        value_type: "integer".to_string(),
                        value: i.to_string(),
                    }
                } else if let Some(f) = n.as_f64() {
                    TypedValue::Value {
                        value_type: "float".to_string(),
                        value: f.to_string(),
                    }
                } else {
                    TypedValue::Value {
                        value_type: "text".to_string(),
                        value: n.to_string(),
                    }
                }
            }
            serde_json::Value::String(s) => TypedValue::Value {
                value_type: "text".to_string(),
                value: s.clone(),
            },
            other => TypedValue::Value {
                value_type: "text".to_string(),
                value: other.to_string(),
            },
        }
    }
}

impl TursoValue {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            TursoValue::Null => serde_json::Value::Null,
            TursoValue::Typed {
                value_type,
                value,
                base64,
            } => match value_type.as_str() {
                "null" => serde_json::Value::Null,
                "integer" => {
                    if let Some(v) = value {
                        if let Ok(i) = v.parse::<i64>() {
                            serde_json::Value::Number(i.into())
                        } else {
                            serde_json::Value::String(v.clone())
                        }
                    } else {
                        serde_json::Value::Null
                    }
                }
                "float" => {
                    if let Some(v) = value {
                        if let Ok(f) = v.parse::<f64>() {
                            serde_json::Number::from_f64(f)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::String(v.clone()))
                        } else {
                            serde_json::Value::String(v.clone())
                        }
                    } else {
                        serde_json::Value::Null
                    }
                }
                "text" => {
                    serde_json::Value::String(value.clone().unwrap_or_default())
                }
                "blob" => {
                    serde_json::Value::String(base64.clone().unwrap_or_default())
                }
                _ => {
                    serde_json::Value::String(value.clone().unwrap_or_default())
                }
            },
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
        assert!(matches!(v, TypedValue::Null { .. }));
    }

    #[test]
    fn test_typed_value_from_json_integer() {
        let v = TypedValue::from_json(&serde_json::json!(42));
        if let TypedValue::Value { value_type, value } = v {
            assert_eq!(value_type, "integer");
            assert_eq!(value, "42");
        } else {
            panic!("Expected Value variant");
        }
    }

    #[test]
    fn test_typed_value_from_json_string() {
        let v = TypedValue::from_json(&serde_json::json!("hello"));
        if let TypedValue::Value { value_type, value } = v {
            assert_eq!(value_type, "text");
            assert_eq!(value, "hello");
        } else {
            panic!("Expected Value variant");
        }
    }

    #[test]
    fn test_turso_value_to_json_integer() {
        let v = TursoValue::Typed {
            value_type: "integer".to_string(),
            value: Some("42".to_string()),
            base64: None,
        };
        assert_eq!(v.to_json(), serde_json::json!(42));
    }

    #[test]
    fn test_turso_value_to_json_text() {
        let v = TursoValue::Typed {
            value_type: "text".to_string(),
            value: Some("hello".to_string()),
            base64: None,
        };
        assert_eq!(v.to_json(), serde_json::json!("hello"));
    }

    #[test]
    fn test_turso_value_to_json_float() {
        let v = TursoValue::Typed {
            value_type: "float".to_string(),
            value: Some("3.14".to_string()),
            base64: None,
        };
        assert_eq!(v.to_json(), serde_json::json!(3.14));
    }

    #[test]
    fn test_turso_value_to_json_null() {
        let v = TursoValue::Null;
        assert_eq!(v.to_json(), serde_json::Value::Null);
    }

    #[test]
    fn test_turso_value_to_json_typed_null() {
        let v = TursoValue::Typed {
            value_type: "null".to_string(),
            value: None,
            base64: None,
        };
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
        let json = serde_json::json!([
            {"type": "integer", "value": "42"},
            {"type": "float", "value": "3.14"},
            {"type": "text", "value": "hello"},
            {"type": "blob", "base64": "AQID"},
            null
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
        let v = TursoValue::Typed {
            value_type: "blob".to_string(),
            value: None,
            base64: Some("SGVsbG8=".to_string()),
        };
        assert_eq!(v.to_json(), serde_json::json!("SGVsbG8="));
    }

    #[test]
    fn typed_value_from_json_bool() {
        let t = TypedValue::from_json(&serde_json::json!(true));
        if let TypedValue::Value { value_type, value } = &t {
            assert_eq!(value_type, "integer");
            assert_eq!(value, "1");
        } else {
            panic!("Expected Value variant");
        }

        let f = TypedValue::from_json(&serde_json::json!(false));
        if let TypedValue::Value { value_type, value } = &f {
            assert_eq!(value_type, "integer");
            assert_eq!(value, "0");
        } else {
            panic!("Expected Value variant");
        }
    }

    #[test]
    fn typed_value_from_json_float() {
        let v = TypedValue::from_json(&serde_json::json!(3.14));
        if let TypedValue::Value { value_type, value } = v {
            assert_eq!(value_type, "float");
            assert_eq!(value, "3.14");
        } else {
            panic!("Expected Value variant");
        }
    }

    #[test]
    fn typed_value_from_json_array() {
        let v = TypedValue::from_json(&serde_json::json!([1, 2, 3]));
        if let TypedValue::Value { value_type, value } = v {
            assert_eq!(value_type, "text");
            assert_eq!(value, "[1,2,3]");
        } else {
            panic!("Expected Value variant");
        }
    }

    #[test]
    fn typed_value_null_serialization() {
        let v = TypedValue::Null { value_type: "null" };
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json, serde_json::json!({"type": "null"}));
    }

    #[test]
    fn test_pipeline_request_serialization() {
        let req = PipelineRequest {
            baton: None,
            requests: vec![
                PipelineRequestItem::Execute {
                    stmt: Statement {
                        sql: "SELECT ?".to_string(),
                        args: Some(vec![TypedValue::Value {
                            value_type: "integer".to_string(),
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
        assert_eq!(json["requests"][1]["type"], "close");
    }
}
