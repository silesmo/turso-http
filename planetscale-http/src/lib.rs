pub mod backend;
pub mod wire;

use backend::PlanetScaleBackend;
use db_http_core::{Query, QueryBuilder, TransactionBuilder};

pub use db_http_core::{Column, Error, QueryResult, Transaction};

/// HTTP client for PlanetScale MySQL over HTTP.
pub struct Client {
    backend: PlanetScaleBackend,
}

impl Client {
    pub fn new(host: &str, username: &str, password: &str) -> Result<Self, Error> {
        if host.is_empty() {
            return Err(Error::Config("host cannot be empty".to_string()));
        }
        if username.is_empty() {
            return Err(Error::Config("username cannot be empty".to_string()));
        }
        if password.is_empty() {
            return Err(Error::Config("password cannot be empty".to_string()));
        }

        let host = host
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();

        Ok(Self {
            backend: PlanetScaleBackend {
                host,
                username: username.to_string(),
                password: password.to_string(),
            },
        })
    }

    pub fn new_from_env() -> Result<Self, Error> {
        let host = std::env::var("PLANETSCALE_HOST")
            .map_err(|_| Error::Config("PLANETSCALE_HOST must be set".to_string()))?;
        let username = std::env::var("PLANETSCALE_USERNAME")
            .map_err(|_| Error::Config("PLANETSCALE_USERNAME must be set".to_string()))?;
        let password = std::env::var("PLANETSCALE_PASSWORD")
            .map_err(|_| Error::Config("PLANETSCALE_PASSWORD must be set".to_string()))?;
        Self::new(&host, &username, &password)
    }

    pub fn query(&self, sql: &str) -> QueryBuilder<&PlanetScaleBackend> {
        QueryBuilder::new(&self.backend, sql)
    }

    pub fn transaction(&self) -> TransactionBuilder<&PlanetScaleBackend> {
        TransactionBuilder::new(&self.backend)
    }

    pub async fn execute(&self, query: Query) -> Result<QueryResult, Error> {
        use db_http_core::DatabaseBackend;
        self.backend.execute_query(&query).await
    }
}
