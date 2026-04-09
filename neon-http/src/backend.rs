use db_http_core::{
    Column, DatabaseBackend, Error, HttpRequest, Query, QueryResult, Transaction, http_post,
};

use crate::wire::{
    NeonQueryRequest, NeonQueryResponse, NeonTransactionRequest, NeonTransactionResponse,
};

pub struct NeonBackend {
    pub(crate) host: String,
    pub(crate) connection_string: String,
}

impl NeonBackend {
    pub fn new(connection_string: &str) -> Result<Self, Error> {
        if connection_string.is_empty() {
            return Err(Error::Config(
                "connection_string cannot be empty".to_string(),
            ));
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
            host,
            connection_string: connection_string.to_string(),
        })
    }

    fn url(&self) -> String {
        format!("https://{}/sql", self.host)
    }

    fn headers(&self) -> Vec<(String, String)> {
        vec![
            (
                "Neon-Connection-String".to_string(),
                self.connection_string.clone(),
            ),
            ("Neon-Raw-Text-Output".to_string(), "true".to_string()),
            ("Neon-Array-Mode".to_string(), "false".to_string()),
        ]
    }

    fn to_wire_query(query: &Query) -> NeonQueryRequest {
        NeonQueryRequest {
            query: query.sql.clone(),
            params: query.params.clone(),
        }
    }

    fn convert_result(
        neon: crate::wire::NeonQueryResult,
    ) -> QueryResult {
        let columns = neon
            .fields
            .iter()
            .map(|f| Column {
                name: f.name.clone(),
            })
            .collect();
        QueryResult {
            columns,
            rows: neon.rows,
            affected_row_count: neon.row_count.max(0) as u64,
        }
    }
}

impl DatabaseBackend for NeonBackend {
    async fn execute_query(&self, query: &Query) -> Result<QueryResult, Error> {
        let wire = Self::to_wire_query(query);
        let body = serde_json::to_string(&wire)?;

        let request = HttpRequest {
            url: self.url(),
            headers: self.headers(),
            body,
        };

        let response_text = http_post(&request).await?;
        let response: NeonQueryResponse =
            serde_json::from_str(&response_text).map_err(Error::Serialization)?;

        match response {
            NeonQueryResponse::Ok(result) => Ok(Self::convert_result(result)),
            NeonQueryResponse::Err(err) => Err(Error::Database(err.message)),
        }
    }

    async fn execute_transaction(&self, transaction: &Transaction) -> Result<Vec<QueryResult>, Error> {
        let wire = NeonTransactionRequest {
            queries: transaction
                .queries
                .iter()
                .map(Self::to_wire_query)
                .collect(),
        };
        let body = serde_json::to_string(&wire)?;

        let request = HttpRequest {
            url: self.url(),
            headers: self.headers(),
            body,
        };

        let response_text = http_post(&request).await?;
        let response: NeonTransactionResponse =
            serde_json::from_str(&response_text).map_err(Error::Serialization)?;

        match response {
            NeonTransactionResponse::Ok(result) => {
                Ok(result.results.into_iter().map(Self::convert_result).collect())
            }
            NeonTransactionResponse::Err(err) => Err(Error::Database(err.message)),
        }
    }
}
