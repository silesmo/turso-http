use crate::error::Error;
use crate::types::{Query, QueryResult, Transaction};
use std::future::Future;

pub trait DatabaseBackend: Send + Sync {
    fn execute_query(&self, query: &Query)
        -> impl Future<Output = Result<QueryResult, Error>> + Send;

    fn execute_transaction(&self, transaction: &Transaction)
        -> impl Future<Output = Result<Vec<QueryResult>, Error>> + Send;
}

impl<T: DatabaseBackend> DatabaseBackend for &T {
    fn execute_query(&self, query: &Query)
        -> impl Future<Output = Result<QueryResult, Error>> + Send {
        (*self).execute_query(query)
    }

    fn execute_transaction(&self, transaction: &Transaction)
        -> impl Future<Output = Result<Vec<QueryResult>, Error>> + Send {
        (*self).execute_transaction(transaction)
    }
}
