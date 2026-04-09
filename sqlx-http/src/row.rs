use std::sync::Arc;

use sqlx_core::column::ColumnIndex;
use sqlx_core::error::Error;
use sqlx_core::row::Row;

use crate::column::HttpColumn;
use crate::db::HttpDb;
use crate::type_info::HttpTypeInfo;
use crate::value::HttpValueRef;

pub struct HttpRow {
    pub(crate) columns: Arc<Vec<HttpColumn>>,
    pub(crate) values: Vec<serde_json::Value>,
}

impl Row for HttpRow {
    type Database = HttpDb;

    fn columns(&self) -> &[HttpColumn] {
        &self.columns
    }

    fn try_get_raw<I: ColumnIndex<Self>>(&self, index: I) -> Result<HttpValueRef<'_>, Error> {
        let index = index.index(self)?;
        let value = &self.values[index];
        let type_info = HttpTypeInfo::from_json(value);
        Ok(HttpValueRef { value, type_info })
    }
}
