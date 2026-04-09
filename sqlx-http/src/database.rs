use crate::arguments::HttpArguments;
use crate::column::HttpColumn;
use crate::connection::HttpConnection;
use crate::query_result::HttpQueryResult;
use crate::row::HttpRow;
use crate::statement::HttpStatement;
use crate::transaction_manager::HttpTransactionManager;
use crate::type_info::HttpTypeInfo;
use crate::value::{HttpValue, HttpValueRef};

#[derive(Debug)]
pub struct HttpDb;

impl sqlx_core::database::Database for HttpDb {
    type Connection = HttpConnection;
    type TransactionManager = HttpTransactionManager;
    type Row = HttpRow;
    type QueryResult = HttpQueryResult;
    type Column = HttpColumn;
    type TypeInfo = HttpTypeInfo;
    type Value = HttpValue;
    type ValueRef<'r> = HttpValueRef<'r>;
    type Arguments<'q> = HttpArguments;
    type ArgumentBuffer<'q> = Vec<serde_json::Value>;
    type Statement<'q> = HttpStatement<'q>;

    const NAME: &'static str = "HTTP";
    const URL_SCHEMES: &'static [&'static str] = &["libsql", "neon", "planetscale"];
}
