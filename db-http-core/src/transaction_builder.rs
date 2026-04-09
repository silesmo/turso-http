use crate::backend::DatabaseBackend;
use crate::error::Error;
use crate::types::{Query, QueryResult, Transaction};

pub struct TransactionBuilder<B: DatabaseBackend> {
    backend: B,
    queries: Vec<Query>,
}

impl<B: DatabaseBackend> TransactionBuilder<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            queries: Vec::new(),
        }
    }

    pub fn add(mut self, query: Query) -> Self {
        self.queries.push(query);
        self
    }

    pub fn merge(mut self, transaction: Transaction) -> Self {
        self.queries.extend(transaction.queries);
        self
    }

    pub fn build(self) -> Transaction {
        Transaction {
            queries: self.queries,
        }
    }

    pub async fn execute(self) -> Result<Vec<QueryResult>, Error> {
        let transaction = Transaction {
            queries: self.queries,
        };
        self.backend.execute_transaction(&transaction).await
    }
}
