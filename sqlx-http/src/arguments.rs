use sqlx_core::arguments::{Arguments, IntoArguments};
use sqlx_core::encode::Encode;
use sqlx_core::error::BoxDynError;
use sqlx_core::types::Type;

use crate::db::HttpDb;

#[derive(Default)]
pub struct HttpArguments {
    pub(crate) values: Vec<serde_json::Value>,
}

impl<'q> Arguments<'q> for HttpArguments {
    type Database = HttpDb;

    fn reserve(&mut self, additional: usize, _size: usize) {
        self.values.reserve(additional);
    }

    fn add<T>(&mut self, value: T) -> Result<(), BoxDynError>
    where
        T: 'q + Encode<'q, HttpDb> + Type<HttpDb>,
    {
        let _ = value.encode(&mut self.values)?;
        Ok(())
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

impl<'q> IntoArguments<'q, HttpDb> for HttpArguments {
    fn into_arguments(self) -> HttpArguments {
        self
    }
}
