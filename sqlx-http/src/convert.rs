use std::sync::Arc;

use crate::arguments::HttpArguments;
use crate::column::HttpColumn;
use crate::query_result::HttpQueryResult;
use crate::row::HttpRow;
use crate::type_info::HttpTypeInfo;

pub(crate) fn convert_core_result(
    result: db_http_core::QueryResult,
) -> (Vec<HttpRow>, HttpQueryResult) {
    let columns: Arc<Vec<HttpColumn>> = Arc::new(
        result
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| HttpColumn {
                name: c.name.clone(),
                ordinal: i,
                type_info: HttpTypeInfo::Text, // default; refined per-value in HttpValueRef
            })
            .collect(),
    );

    let rows = result
        .rows
        .into_iter()
        .map(|json_val| {
            let values = columns
                .iter()
                .map(|col| {
                    if let serde_json::Value::Object(ref obj) = json_val {
                        obj.get(&col.name)
                            .cloned()
                            .unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::Value::Null
                    }
                })
                .collect();
            HttpRow {
                columns: columns.clone(),
                values,
            }
        })
        .collect();

    let qr = HttpQueryResult {
        rows_affected: result.affected_row_count,
    };
    (rows, qr)
}

pub(crate) fn to_core_query(sql: &str, args: Option<HttpArguments>) -> db_http_core::Query {
    db_http_core::Query {
        sql: sql.to_owned(),
        params: args.map(|a| a.values).unwrap_or_default(),
    }
}
