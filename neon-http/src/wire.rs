use serde::{Deserialize, Serialize};

// --- Request types ---

#[derive(Debug, Serialize)]
pub struct NeonQueryRequest {
    pub query: String,
    pub params: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct NeonTransactionRequest {
    pub queries: Vec<NeonQueryRequest>,
}

// --- Response types ---

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NeonQueryResponse {
    Ok(NeonQueryResult),
    Err(NeonErrorResponse),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NeonTransactionResponse {
    Ok(NeonTransactionResult),
    Err(NeonErrorResponse),
}

#[derive(Debug, Deserialize)]
pub struct NeonQueryResult {
    pub command: String,
    #[serde(default)]
    pub row_count: i64,
    pub rows: Vec<serde_json::Value>,
    pub fields: Vec<NeonField>,
    #[serde(default)]
    pub row_as_array: bool,
}

#[derive(Debug, Deserialize)]
pub struct NeonTransactionResult {
    pub results: Vec<NeonQueryResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NeonField {
    pub name: String,
    #[serde(default)]
    pub data_type_id: i64,
    #[serde(default)]
    pub table_id: i64,
    #[serde(default)]
    pub column_id: i64,
    #[serde(default)]
    pub data_type_size: i64,
    #[serde(default)]
    pub data_type_modifier: i64,
    #[serde(default)]
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct NeonErrorResponse {
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_query_response_ok() {
        let json = serde_json::json!({
            "command": "SELECT",
            "row_count": 2,
            "rows": [{"id": 1}, {"id": 2}],
            "fields": [{"name": "id", "data_type_id": 23}]
        });
        let resp: NeonQueryResponse = serde_json::from_value(json).unwrap();
        match resp {
            NeonQueryResponse::Ok(r) => {
                assert_eq!(r.command, "SELECT");
                assert_eq!(r.row_count, 2);
                assert_eq!(r.rows.len(), 2);
                assert_eq!(r.fields.len(), 1);
                assert_eq!(r.fields[0].name, "id");
                assert_eq!(r.fields[0].data_type_id, 23);
            }
            NeonQueryResponse::Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    }

    #[test]
    fn deserialize_query_response_error() {
        let json = serde_json::json!({
            "message": "relation does not exist",
            "code": "42P01"
        });
        let resp: NeonQueryResponse = serde_json::from_value(json).unwrap();
        match resp {
            NeonQueryResponse::Err(e) => {
                assert_eq!(e.message, "relation does not exist");
                assert_eq!(e.code, Some("42P01".to_string()));
            }
            NeonQueryResponse::Ok(_) => panic!("Expected Err, got Ok"),
        }
    }

    #[test]
    fn deserialize_transaction_response_ok() {
        let json = serde_json::json!({
            "results": [
                {
                    "command": "INSERT",
                    "row_count": 1,
                    "rows": [],
                    "fields": []
                },
                {
                    "command": "SELECT",
                    "row_count": 1,
                    "rows": [{"id": 1}],
                    "fields": [{"name": "id"}]
                }
            ]
        });
        let resp: NeonTransactionResponse = serde_json::from_value(json).unwrap();
        match resp {
            NeonTransactionResponse::Ok(r) => {
                assert_eq!(r.results.len(), 2);
                assert_eq!(r.results[0].command, "INSERT");
                assert_eq!(r.results[1].command, "SELECT");
                assert_eq!(r.results[1].rows.len(), 1);
            }
            NeonTransactionResponse::Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    }

    #[test]
    fn deserialize_transaction_response_error() {
        let json = serde_json::json!({
            "message": "transaction failed",
            "code": "25P02"
        });
        let resp: NeonTransactionResponse = serde_json::from_value(json).unwrap();
        match resp {
            NeonTransactionResponse::Err(e) => {
                assert_eq!(e.message, "transaction failed");
                assert_eq!(e.code, Some("25P02".to_string()));
            }
            NeonTransactionResponse::Ok(_) => panic!("Expected Err, got Ok"),
        }
    }

    #[test]
    fn deserialize_field_all_properties() {
        let json = serde_json::json!({
            "name": "id",
            "data_type_id": 23,
            "table_id": 16384,
            "column_id": 1,
            "data_type_size": 4,
            "data_type_modifier": -1,
            "format": "text"
        });
        let field: NeonField = serde_json::from_value(json).unwrap();
        assert_eq!(field.name, "id");
        assert_eq!(field.data_type_id, 23);
        assert_eq!(field.table_id, 16384);
        assert_eq!(field.column_id, 1);
        assert_eq!(field.data_type_size, 4);
        assert_eq!(field.data_type_modifier, -1);
        assert_eq!(field.format, "text");
    }

    #[test]
    fn deserialize_field_minimal() {
        let json = serde_json::json!({"name": "col"});
        let field: NeonField = serde_json::from_value(json).unwrap();
        assert_eq!(field.name, "col");
        assert_eq!(field.data_type_id, 0);
        assert_eq!(field.table_id, 0);
        assert_eq!(field.column_id, 0);
        assert_eq!(field.data_type_size, 0);
        assert_eq!(field.data_type_modifier, 0);
        assert_eq!(field.format, "");
    }

    #[test]
    fn serialize_query_request() {
        let req = NeonQueryRequest {
            query: "SELECT $1".to_string(),
            params: vec![serde_json::json!(42)],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["query"], "SELECT $1");
        assert_eq!(json["params"][0], 42);
    }

    #[test]
    fn serialize_transaction_request() {
        let req = NeonTransactionRequest {
            queries: vec![
                NeonQueryRequest {
                    query: "INSERT INTO t VALUES ($1)".to_string(),
                    params: vec![serde_json::json!("hello")],
                },
                NeonQueryRequest {
                    query: "SELECT 1".to_string(),
                    params: vec![],
                },
            ],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["queries"].as_array().unwrap().len(), 2);
        assert_eq!(json["queries"][0]["query"], "INSERT INTO t VALUES ($1)");
        assert_eq!(json["queries"][0]["params"][0], "hello");
        assert_eq!(json["queries"][1]["query"], "SELECT 1");
        assert!(json["queries"][1]["params"].as_array().unwrap().is_empty());
    }
}
