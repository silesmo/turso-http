use std::sync::Arc;

use sqlx_core::error::Error;
use sqlx_core::executor::Execute;
use sqlx_core::from_row::FromRow;

use crate::convert::convert_core_result;
use crate::db::HttpDb;
use crate::pool::DynBackend;
use crate::query_result::HttpQueryResult;
use crate::row::HttpRow;

// ---------------------------------------------------------------------------
// TransactionResult
// ---------------------------------------------------------------------------

/// The result of a single statement within a committed transaction.
///
/// Contains both the rows returned by the statement (if any) and the
/// affected row count.
pub struct TransactionResult {
    pub(crate) rows: Vec<HttpRow>,
    pub(crate) query_result: HttpQueryResult,
}

impl TransactionResult {
    pub fn rows_affected(&self) -> u64 {
        self.query_result.rows_affected()
    }

    pub fn rows(&self) -> &[HttpRow] {
        &self.rows
    }

    pub fn into_rows(self) -> Vec<HttpRow> {
        self.rows
    }

    /// Convert all rows into a typed collection via `FromRow`.
    pub fn into_typed<T: for<'r> FromRow<'r, HttpRow>>(self) -> Result<Vec<T>, Error> {
        self.rows
            .iter()
            .map(|row| T::from_row(row))
            .collect::<Result<Vec<T>, _>>()
    }

    /// Convert the first row into a typed value, or `None` if empty.
    pub fn first_typed<T: for<'r> FromRow<'r, HttpRow>>(&self) -> Result<Option<T>, Error> {
        self.rows.first().map(|row| T::from_row(row)).transpose()
    }
}

// ---------------------------------------------------------------------------
// FromTransactionResult — converts a single TransactionResult into a type
// ---------------------------------------------------------------------------

/// Trait for converting a single [`TransactionResult`] into a desired output type.
///
/// Built-in implementations:
/// - `TransactionResult` — passthrough (for INSERT/UPDATE/DELETE)
/// - `Vec<T: FromRow>` — convert all rows
/// - `Option<T: FromRow>` — first row or None
/// - `T: FromRow` — exactly one row (errors if not exactly one)
pub trait FromTransactionResult: Sized {
    fn from_transaction_result(result: TransactionResult) -> Result<Self, Error>;
}

impl FromTransactionResult for TransactionResult {
    fn from_transaction_result(result: TransactionResult) -> Result<Self, Error> {
        Ok(result)
    }
}

impl<T: for<'r> FromRow<'r, HttpRow>> FromTransactionResult for Vec<T> {
    fn from_transaction_result(result: TransactionResult) -> Result<Self, Error> {
        result.into_typed()
    }
}

impl<T: for<'r> FromRow<'r, HttpRow>> FromTransactionResult for Option<T> {
    fn from_transaction_result(result: TransactionResult) -> Result<Self, Error> {
        result.first_typed()
    }
}

// ---------------------------------------------------------------------------
// FromTransactionResults — converts Vec<TransactionResult> into a tuple
// ---------------------------------------------------------------------------

/// Trait for converting a `Vec<TransactionResult>` into a tuple of typed results.
///
/// Implemented for tuples of 1–12 elements where each element implements
/// [`FromTransactionResult`].
pub trait FromTransactionResults: Sized {
    const COUNT: usize;
    fn from_results(results: Vec<TransactionResult>) -> Result<Self, Error>;
}

macro_rules! impl_from_transaction_results {
    // Base: single element
    ($count:literal: $idx:tt => $T:ident) => {
        impl<$T: FromTransactionResult> FromTransactionResults for ($T,) {
            const COUNT: usize = $count;
            fn from_results(results: Vec<TransactionResult>) -> Result<Self, Error> {
                if results.len() != $count {
                    return Err(Error::Protocol(
                        format!(
                            "transaction result count mismatch: expected {}, got {}",
                            $count,
                            results.len()
                        )
                        .into(),
                    ));
                }
                let mut iter = results.into_iter();
                Ok((
                    $T::from_transaction_result(iter.next().unwrap())?,
                ))
            }
        }
    };
    // Recursive: multiple elements
    ($count:literal: $($idx:tt => $T:ident),+) => {
        impl<$($T: FromTransactionResult),+> FromTransactionResults for ($($T,)+) {
            const COUNT: usize = $count;
            fn from_results(results: Vec<TransactionResult>) -> Result<Self, Error> {
                if results.len() != $count {
                    return Err(Error::Protocol(
                        format!(
                            "transaction result count mismatch: expected {}, got {}",
                            $count,
                            results.len()
                        )
                        .into(),
                    ));
                }
                let mut iter = results.into_iter();
                Ok((
                    $($T::from_transaction_result(iter.next().unwrap())?,)+
                ))
            }
        }
    };
}

impl_from_transaction_results!(1: 0 => A);
impl_from_transaction_results!(2: 0 => A, 1 => B);
impl_from_transaction_results!(3: 0 => A, 1 => B, 2 => C);
impl_from_transaction_results!(4: 0 => A, 1 => B, 2 => C, 3 => D);
impl_from_transaction_results!(5: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E);
impl_from_transaction_results!(6: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F);
impl_from_transaction_results!(7: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G);
impl_from_transaction_results!(8: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H);
impl_from_transaction_results!(9: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I);
impl_from_transaction_results!(10: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J);
impl_from_transaction_results!(11: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K);
impl_from_transaction_results!(12: 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L);

// ---------------------------------------------------------------------------
// HttpTransaction
// ---------------------------------------------------------------------------

pub struct HttpTransaction {
    backend: Arc<dyn DynBackend>,
    queries: Vec<db_http_core::Query>,
}

impl HttpTransaction {
    pub(crate) fn new(backend: Arc<dyn DynBackend>) -> Self {
        Self {
            backend,
            queries: Vec::new(),
        }
    }

    /// Add a query to the transaction batch.
    pub fn add<'q, E: Execute<'q, HttpDb>>(&mut self, mut query: E) -> Result<(), Error> {
        let sql = query.sql().to_string();
        let args = query.take_arguments().map_err(Error::Encode)?;
        self.queries.push(crate::convert::to_core_query(&sql, args));
        Ok(())
    }

    /// Execute all queued queries atomically.
    pub async fn commit(self) -> Result<Vec<TransactionResult>, Error> {
        let tx = db_http_core::Transaction {
            queries: self.queries,
        };
        let results = self
            .backend
            .execute_transaction(&tx)
            .await
            .map_err(|e| Error::Protocol(e.to_string()))?;
        Ok(results
            .into_iter()
            .map(|r| {
                let (rows, query_result) = convert_core_result(r);
                TransactionResult { rows, query_result }
            })
            .collect())
    }

    /// Execute all queued queries atomically and destructure the results
    /// into a typed tuple.
    ///
    /// Each element in the tuple corresponds to one `add()` call, in order.
    /// Elements can be:
    /// - `TransactionResult` — raw result (for INSERT/UPDATE/DELETE)
    /// - `Vec<T: FromRow>` — all rows converted to `T`
    /// - `Option<T: FromRow>` — first row as `T`, or `None`
    ///
    /// # Errors
    ///
    /// Returns an error if the number of results doesn't match the tuple size,
    /// or if row conversion fails.
    pub async fn commit_as<T: FromTransactionResults>(self) -> Result<T, Error> {
        let results = self.commit().await?;
        T::from_results(results)
    }

    /// Discard all queued queries.
    pub fn rollback(self) {
        // Drop - queries are discarded
    }
}
