use sqlx_core::column::{Column, ColumnIndex};
use sqlx_core::error::Error;

use crate::db::HttpDb;
use crate::row::HttpRow;
use crate::statement::HttpStatement;
use crate::type_info::HttpTypeInfo;

#[derive(Debug, Clone)]
pub struct HttpColumn {
    pub(crate) name: String,
    pub(crate) ordinal: usize,
    pub(crate) type_info: HttpTypeInfo,
}

impl Column for HttpColumn {
    type Database = HttpDb;

    fn ordinal(&self) -> usize {
        self.ordinal
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn type_info(&self) -> &HttpTypeInfo {
        &self.type_info
    }
}

impl ColumnIndex<HttpRow> for usize {
    fn index(&self, row: &HttpRow) -> Result<usize, Error> {
        if *self < row.values.len() {
            Ok(*self)
        } else {
            Err(Error::ColumnIndexOutOfBounds {
                index: *self,
                len: row.values.len(),
            })
        }
    }
}

impl ColumnIndex<HttpRow> for &str {
    fn index(&self, row: &HttpRow) -> Result<usize, Error> {
        row.columns
            .iter()
            .position(|col| col.name == *self)
            .ok_or_else(|| Error::ColumnNotFound((*self).to_string()))
    }
}

impl<'q> ColumnIndex<HttpStatement<'q>> for usize {
    fn index(&self, stmt: &HttpStatement<'q>) -> Result<usize, Error> {
        if *self < stmt.columns.len() {
            Ok(*self)
        } else {
            Err(Error::ColumnIndexOutOfBounds {
                index: *self,
                len: stmt.columns.len(),
            })
        }
    }
}

impl<'q> ColumnIndex<HttpStatement<'q>> for &str {
    fn index(&self, stmt: &HttpStatement<'q>) -> Result<usize, Error> {
        stmt.columns
            .iter()
            .position(|col| col.name == *self)
            .ok_or_else(|| Error::ColumnNotFound((*self).to_string()))
    }
}
