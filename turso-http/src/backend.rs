use db_http_core::{
    Column, DatabaseBackend, Error, HttpRequest, Query, QueryResult, Transaction, http_post,
};

use crate::wire::{
    Batch, BatchCondition, BatchStep, ExecuteResult, PipelineOkResponse, PipelineRequest,
    PipelineRequestItem, PipelineResponse, PipelineResultItem, Statement, TypedValue,
    rewrite_placeholders,
};

pub struct TursoBackend {
    pub(crate) base_url: String,
    pub(crate) auth_token: String,
}

impl TursoBackend {
    pub fn new(host: &str, auth_token: &str) -> Result<Self, Error> {
        if host.is_empty() {
            return Err(Error::Config("host cannot be empty".to_string()));
        }
        if auth_token.is_empty() {
            return Err(Error::Config("auth_token cannot be empty".to_string()));
        }
        let base_url = if host.starts_with("http://") || host.starts_with("https://") {
            host.to_string()
        } else {
            format!("https://{host}")
        };
        Ok(Self {
            base_url,
            auth_token: auth_token.to_string(),
        })
    }

    fn url(&self) -> String {
        format!("{}/v2/pipeline", self.base_url)
    }

    fn headers(&self) -> Vec<(String, String)> {
        vec![(
            "Authorization".to_string(),
            format!("Bearer {}", self.auth_token),
        )]
    }

    fn to_statement(query: &Query) -> Statement {
        let sql = rewrite_placeholders(&query.sql);
        let args = if query.params.is_empty() {
            None
        } else {
            Some(query.params.iter().map(TypedValue::from_json).collect())
        };
        Statement { sql, args }
    }

    fn convert_result(result: ExecuteResult) -> QueryResult {
        let columns: Vec<Column> = result
            .cols
            .iter()
            .map(|c| Column {
                name: c.name.clone(),
            })
            .collect();

        let rows: Vec<serde_json::Value> = result
            .rows
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                for (i, val) in row.iter().enumerate() {
                    if let Some(col) = columns.get(i) {
                        obj.insert(col.name.clone(), val.to_json());
                    }
                }
                serde_json::Value::Object(obj)
            })
            .collect();

        QueryResult {
            columns,
            rows,
            affected_row_count: result.affected_row_count,
        }
    }

    async fn send_pipeline(
        &self,
        pipeline: PipelineRequest,
    ) -> Result<PipelineResponse, Error> {
        let body = serde_json::to_string(&pipeline)?;

        let request = HttpRequest {
            url: self.url(),
            headers: self.headers(),
            body,
        };

        let response_text = http_post(&request).await?;
        serde_json::from_str(&response_text).map_err(|e| {
            Error::Http(format!(
                "Failed to parse Turso response: {e}. Body: {response_text}"
            ))
        })
    }
}

impl DatabaseBackend for TursoBackend {
    async fn execute_query(&self, query: &Query) -> Result<QueryResult, Error> {
        let pipeline = PipelineRequest {
            baton: None,
            requests: vec![
                PipelineRequestItem::Execute {
                    stmt: Self::to_statement(query),
                },
                PipelineRequestItem::Close,
            ],
        };

        let response = self.send_pipeline(pipeline).await?;

        for item in response.results {
            match item {
                PipelineResultItem::Ok { response } => match response {
                    PipelineOkResponse::Execute { result } => {
                        return Ok(Self::convert_result(result));
                    }
                    PipelineOkResponse::Close => continue,
                    _ => continue,
                },
                PipelineResultItem::Error { error } => {
                    return Err(Error::Database(error.message));
                }
            }
        }

        Err(Error::Database(
            "No execute result in Turso response".to_string(),
        ))
    }

    async fn execute_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Vec<QueryResult>, Error> {
        // Build batch steps: BEGIN, then each query (conditioned on previous ok), COMMIT, ROLLBACK on error
        let mut steps = Vec::new();

        // Step 0: BEGIN
        steps.push(BatchStep {
            condition: None,
            stmt: Statement {
                sql: "BEGIN".to_string(),
                args: None,
            },
        });

        // Steps 1..N: user queries, each conditioned on the previous step succeeding
        for (i, query) in transaction.queries.iter().enumerate() {
            steps.push(BatchStep {
                condition: Some(BatchCondition {
                    condition_type: "ok".to_string(),
                    step: i, // previous step index
                }),
                stmt: Self::to_statement(query),
            });
        }

        let last_query_step = transaction.queries.len(); // index of last user query step

        // COMMIT: conditioned on last query succeeding
        steps.push(BatchStep {
            condition: Some(BatchCondition {
                condition_type: "ok".to_string(),
                step: last_query_step,
            }),
            stmt: Statement {
                sql: "COMMIT".to_string(),
                args: None,
            },
        });

        // ROLLBACK: conditioned on last query failing
        steps.push(BatchStep {
            condition: Some(BatchCondition {
                condition_type: "error".to_string(),
                step: last_query_step,
            }),
            stmt: Statement {
                sql: "ROLLBACK".to_string(),
                args: None,
            },
        });

        let pipeline = PipelineRequest {
            baton: None,
            requests: vec![
                PipelineRequestItem::Batch {
                    batch: Batch { steps },
                },
                PipelineRequestItem::Close,
            ],
        };

        let response = self.send_pipeline(pipeline).await?;

        for item in response.results {
            match item {
                PipelineResultItem::Ok { response } => match response {
                    PipelineOkResponse::Batch { result } => {
                        // Check for step errors first
                        for error in &result.step_errors {
                            if let Some(err) = error {
                                return Err(Error::Database(err.message.clone()));
                            }
                        }

                        // Extract results for user queries (skip BEGIN at index 0,
                        // skip COMMIT/ROLLBACK at the end)
                        let user_results: Vec<QueryResult> = result
                            .step_results
                            .into_iter()
                            .skip(1) // skip BEGIN
                            .take(transaction.queries.len())
                            .map(|opt| {
                                opt.map(Self::convert_result).unwrap_or(QueryResult {
                                    columns: vec![],
                                    rows: vec![],
                                    affected_row_count: 0,
                                })
                            })
                            .collect();

                        return Ok(user_results);
                    }
                    PipelineOkResponse::Close => continue,
                    _ => continue,
                },
                PipelineResultItem::Error { error } => {
                    return Err(Error::Database(error.message));
                }
            }
        }

        Err(Error::Database(
            "No batch result in Turso response".to_string(),
        ))
    }
}
