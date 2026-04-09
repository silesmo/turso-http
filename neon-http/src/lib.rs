pub mod backend;
pub mod wire;

use backend::NeonBackend;
use db_http_core::{Query, QueryBuilder, TransactionBuilder};

pub use db_http_core::{Column, Error, QueryResult, Transaction};

/// HTTP client for Neon Serverless Postgres.
///
/// Also compatible with [PlanetScale Postgres](https://planetscale.com/docs/postgres/connecting/neon-serverless-driver).
pub struct Client {
    backend: NeonBackend,
}

impl Client {
    pub fn new(connection_string: &str) -> Result<Self, Error> {
        if connection_string.is_empty() {
            return Err(Error::Config("connection_string cannot be empty".to_string()));
        }

        let host = connection_string
            .split('@')
            .nth(1)
            .and_then(|s| s.split('/').next())
            .filter(|h| !h.is_empty())
            .ok_or_else(|| {
                Error::Config(
                    "Invalid connection string: could not parse host (expected format: postgres://user:pass@host/db)".to_string(),
                )
            })?
            .to_string();

        Ok(Self {
            backend: NeonBackend {
                host,
                connection_string: connection_string.to_string(),
            },
        })
    }

    pub fn new_from_env() -> Result<Self, Error> {
        let connection_string = std::env::var("NEON_CONNECTION_STRING")
            .map_err(|_| Error::Config("NEON_CONNECTION_STRING must be set".to_string()))?;
        Self::new(&connection_string)
    }

    pub fn query(&self, sql: &str) -> QueryBuilder<&NeonBackend> {
        QueryBuilder::new(&self.backend, sql)
    }

    pub fn transaction(&self) -> TransactionBuilder<&NeonBackend> {
        TransactionBuilder::new(&self.backend)
    }

    pub async fn execute(&self, query: Query) -> Result<QueryResult, Error> {
        use db_http_core::DatabaseBackend;
        self.backend.execute_query(&query).await
    }
}
