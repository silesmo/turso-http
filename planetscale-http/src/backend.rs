use base64::Engine;
use db_http_core::{
    Column, DatabaseBackend, Error, HttpRequest, Query, QueryResult, Transaction, http_post,
};

use crate::wire::{
    PsQueryRequest, PsQueryResponse, PsQueryResult, cast_value, decode_row, format_query,
};

pub struct PlanetScaleBackend {
    pub(crate) host: String,
    pub(crate) username: String,
    pub(crate) password: String,
}

impl PlanetScaleBackend {
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
            host,
            username: username.to_string(),
            password: password.to_string(),
        })
    }

    fn url(&self) -> String {
        format!(
            "https://{}/psdb.v1alpha1.Database/Execute",
            self.host
        )
    }

    fn headers(&self) -> Vec<(String, String)> {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        vec![(
            "Authorization".to_string(),
            format!("Basic {}", encoded),
        )]
    }

    fn convert_result(result: PsQueryResult) -> Result<QueryResult, Error> {
        let fields = result.fields.unwrap_or_default();
        let raw_rows = result.rows.unwrap_or_default();

        let columns: Vec<Column> = fields
            .iter()
            .map(|f| Column {
                name: f.name.clone(),
            })
            .collect();

        let mut rows = Vec::with_capacity(raw_rows.len());
        for row in &raw_rows {
            let decoded = decode_row(row).map_err(Error::Database)?;
            let mut obj = serde_json::Map::new();
            for (i, val) in decoded.into_iter().enumerate() {
                if let Some(field) = fields.get(i) {
                    obj.insert(field.name.clone(), cast_value(field, val));
                }
            }
            rows.push(serde_json::Value::Object(obj));
        }

        let affected_row_count = result
            .rows_affected
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(QueryResult {
            columns,
            rows,
            affected_row_count,
        })
    }

    async fn send_query(
        &self,
        sql: &str,
        session: Option<serde_json::Value>,
    ) -> Result<PsQueryResponse, Error> {
        let wire = PsQueryRequest {
            query: sql.to_string(),
            session,
        };
        let body = serde_json::to_string(&wire)?;

        let request = HttpRequest {
            url: self.url(),
            headers: self.headers(),
            body,
        };

        let response_text = http_post(&request).await?;
        serde_json::from_str(&response_text).map_err(|e| {
            Error::Http(format!(
                "Failed to parse PlanetScale response: {e}. Body: {response_text}"
            ))
        })
    }
}

impl DatabaseBackend for PlanetScaleBackend {
    async fn execute_query(&self, query: &Query) -> Result<QueryResult, Error> {
        let sql = format_query(&query.sql, &query.params).map_err(Error::Database)?;
        let response = self.send_query(&sql, None).await?;

        if let Some(err) = response.error {
            return Err(Error::Database(err.message));
        }

        match response.result {
            Some(result) => Ok(Self::convert_result(result)?),
            None => Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_row_count: 0,
            }),
        }
    }

    async fn execute_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Vec<QueryResult>, Error> {
        // BEGIN — start with no session
        let begin_resp = self.send_query("BEGIN", None).await?;
        if let Some(err) = begin_resp.error {
            return Err(Error::Database(err.message));
        }
        let mut session = begin_resp.session;

        // Execute each query, forwarding the session
        let mut results = Vec::with_capacity(transaction.queries.len());
        for query in &transaction.queries {
            let sql = format_query(&query.sql, &query.params).map_err(Error::Database)?;
            let resp = self.send_query(&sql, session).await?;

            if let Some(err) = resp.error {
                // Attempt rollback, ignoring errors
                let _ = self.send_query("ROLLBACK", resp.session).await;
                return Err(Error::Database(err.message));
            }

            session = resp.session;
            match resp.result {
                Some(result) => results.push(Self::convert_result(result)?),
                None => results.push(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    affected_row_count: 0,
                }),
            }
        }

        // COMMIT
        let commit_resp = self.send_query("COMMIT", session).await?;
        if let Some(err) = commit_resp.error {
            return Err(Error::Database(err.message));
        }

        Ok(results)
    }
}
