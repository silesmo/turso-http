use crate::backend::DatabaseBackend;
use crate::deserializer::{deserialize_all, deserialize_one};
use crate::error::Error;
use crate::types::{Query, QueryResult};
use serde::de::DeserializeOwned;

pub struct QueryBuilder<B: DatabaseBackend> {
    backend: B,
    sql: String,
    params: Vec<serde_json::Value>,
}

impl<B: DatabaseBackend> QueryBuilder<B> {
    pub fn new(backend: B, sql: impl Into<String>) -> Self {
        Self {
            backend,
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    pub fn bind(mut self, value: impl Into<serde_json::Value>) -> Self {
        self.params.push(value.into());
        self
    }

    pub fn build(self) -> Query {
        Query {
            sql: self.sql,
            params: self.params,
        }
    }

    pub async fn fetch_one<T: DeserializeOwned>(self) -> Result<T, Error> {
        let query = Query {
            sql: self.sql,
            params: self.params,
        };
        let result = self.backend.execute_query(&query).await?;
        deserialize_one(result)
    }

    pub async fn fetch_all<T: DeserializeOwned>(self) -> Result<Vec<T>, Error> {
        let query = Query {
            sql: self.sql,
            params: self.params,
        };
        let result = self.backend.execute_query(&query).await?;
        deserialize_all(result)
    }

    pub async fn execute(self) -> Result<QueryResult, Error> {
        let query = Query {
            sql: self.sql,
            params: self.params,
        };
        self.backend.execute_query(&query).await
    }
}
