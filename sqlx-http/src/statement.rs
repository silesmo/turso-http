use std::borrow::Cow;

use either::Either;
use sqlx_core::statement::Statement;

use crate::arguments::HttpArguments;
use crate::column::HttpColumn;
use crate::db::HttpDb;
use crate::type_info::HttpTypeInfo;

#[derive(Debug, Clone)]
pub struct HttpStatement<'q> {
    pub(crate) sql: Cow<'q, str>,
    pub(crate) columns: Vec<HttpColumn>,
}

impl<'q> Statement<'q> for HttpStatement<'q> {
    type Database = HttpDb;

    fn to_owned(&self) -> HttpStatement<'static> {
        HttpStatement {
            sql: Cow::Owned(self.sql.clone().into_owned()),
            columns: self.columns.clone(),
        }
    }

    fn sql(&self) -> &str {
        &self.sql
    }

    fn parameters(&self) -> Option<Either<&[HttpTypeInfo], usize>> {
        Some(Either::Right(0))
    }

    fn columns(&self) -> &[HttpColumn] {
        &self.columns
    }

    sqlx_core::impl_statement_query!(HttpArguments);
}
