use crate::error::Error;
use crate::types::QueryResult;
use serde::de::DeserializeOwned;

pub fn deserialize_one<T: DeserializeOwned>(result: QueryResult) -> Result<T, Error> {
    let row = result.rows.into_iter().next().ok_or(Error::NoRows)?;
    serde_json::from_value(row).map_err(Error::Serialization)
}

pub fn deserialize_all<T: DeserializeOwned>(result: QueryResult) -> Result<Vec<T>, Error> {
    result
        .rows
        .into_iter()
        .map(|row| serde_json::from_value(row).map_err(Error::Serialization))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn make_result(rows: Vec<serde_json::Value>) -> QueryResult {
        QueryResult {
            columns: vec![],
            rows,
            affected_row_count: 0,
        }
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct User {
        id: i64,
        name: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct OptionalUser {
        id: i64,
        name: Option<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct NestedMeta {
        meta: serde_json::Value,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct MixedRow {
        int_val: i64,
        float_val: f64,
        str_val: String,
        bool_val: bool,
        null_val: Option<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct BoolStruct {
        active: bool,
    }

    #[test]
    fn deserialize_one_simple_struct() {
        let result = make_result(vec![serde_json::json!({"id": 1, "name": "Alice"})]);
        let user: User = deserialize_one(result).unwrap();
        assert_eq!(user, User { id: 1, name: "Alice".to_string() });
    }

    #[test]
    fn deserialize_all_multiple_rows() {
        let result = make_result(vec![
            serde_json::json!({"id": 1, "name": "Alice"}),
            serde_json::json!({"id": 2, "name": "Bob"}),
            serde_json::json!({"id": 3, "name": "Carol"}),
        ]);
        let users: Vec<User> = deserialize_all(result).unwrap();
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].name, "Alice");
        assert_eq!(users[1].name, "Bob");
        assert_eq!(users[2].name, "Carol");
    }

    #[test]
    fn deserialize_one_no_rows() {
        let result = make_result(vec![]);
        let err = deserialize_one::<User>(result).unwrap_err();
        assert!(matches!(err, Error::NoRows));
    }

    #[test]
    fn deserialize_all_empty() {
        let result = make_result(vec![]);
        let users: Vec<User> = deserialize_all(result).unwrap();
        assert!(users.is_empty());
    }

    #[test]
    fn deserialize_one_with_nulls() {
        let result = make_result(vec![serde_json::json!({"id": 1, "name": null})]);
        let user: OptionalUser = deserialize_one(result).unwrap();
        assert_eq!(user, OptionalUser { id: 1, name: None });
    }

    #[test]
    fn deserialize_one_type_mismatch() {
        let result = make_result(vec![serde_json::json!({"id": "not_a_number", "name": "Alice"})]);
        let err = deserialize_one::<User>(result).unwrap_err();
        assert!(matches!(err, Error::Serialization(_)));
    }

    #[test]
    fn deserialize_one_nested_json() {
        let result = make_result(vec![serde_json::json!({"meta": {"key": "val"}})]);
        let row: NestedMeta = deserialize_one(result).unwrap();
        assert_eq!(row.meta, serde_json::json!({"key": "val"}));
    }

    #[test]
    fn deserialize_all_mixed_types() {
        let result = make_result(vec![serde_json::json!({
            "int_val": 42,
            "float_val": 3.14,
            "str_val": "hello",
            "bool_val": true,
            "null_val": null
        })]);
        let rows: Vec<MixedRow> = deserialize_all(result).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], MixedRow {
            int_val: 42,
            float_val: 3.14,
            str_val: "hello".to_string(),
            bool_val: true,
            null_val: None,
        });
    }

    #[test]
    fn deserialize_one_extra_fields() {
        let result = make_result(vec![serde_json::json!({"id": 1, "name": "Alice", "extra": "ignored"})]);
        let user: User = deserialize_one(result).unwrap();
        assert_eq!(user, User { id: 1, name: "Alice".to_string() });
    }

    #[test]
    fn deserialize_one_bool_values() {
        let result = make_result(vec![serde_json::json!({"active": true})]);
        let row: BoolStruct = deserialize_one(result).unwrap();
        assert_eq!(row, BoolStruct { active: true });

        let result = make_result(vec![serde_json::json!({"active": false})]);
        let row: BoolStruct = deserialize_one(result).unwrap();
        assert_eq!(row, BoolStruct { active: false });
    }
}
