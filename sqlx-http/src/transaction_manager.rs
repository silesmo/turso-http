use std::borrow::Cow;

use futures_core::future::BoxFuture;
use sqlx_core::error::Error;
use sqlx_core::transaction::TransactionManager;

use crate::connection::HttpConnection;
use crate::db::HttpDb;

pub struct HttpTransactionManager;

impl TransactionManager for HttpTransactionManager {
    type Database = HttpDb;

    fn begin<'conn>(
        conn: &'conn mut HttpConnection,
        _statement: Option<Cow<'static, str>>,
    ) -> BoxFuture<'conn, Result<(), Error>> {
        Box::pin(async move {
            conn.transaction_depth += 1;
            Ok(())
        })
    }

    fn commit(conn: &mut HttpConnection) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            if conn.transaction_depth > 0 {
                conn.transaction_depth -= 1;
            }
            Ok(())
        })
    }

    fn rollback(conn: &mut HttpConnection) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            if conn.transaction_depth > 0 {
                conn.transaction_depth -= 1;
            }
            Ok(())
        })
    }

    fn start_rollback(conn: &mut HttpConnection) {
        if conn.transaction_depth > 0 {
            conn.transaction_depth -= 1;
        }
    }

    fn get_transaction_depth(conn: &HttpConnection) -> usize {
        conn.transaction_depth
    }
}
