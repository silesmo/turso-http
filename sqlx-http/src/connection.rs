use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use futures_core::future::BoxFuture;
use log::LevelFilter;
use sqlx_core::connection::{ConnectOptions, Connection};
use sqlx_core::error::Error;
use sqlx_core::transaction::Transaction;
use url::Url;

use crate::db::HttpDb;
use crate::pool::DynBackend;

pub struct HttpConnection {
    #[allow(dead_code)]
    pub(crate) backend: Arc<dyn DynBackend>,
    pub(crate) transaction_depth: usize,
}

impl Connection for HttpConnection {
    type Database = HttpDb;
    type Options = HttpConnectOptions;

    fn close(self) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async { Ok(()) })
    }

    fn close_hard(self) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async { Ok(()) })
    }

    fn ping(&mut self) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async { Ok(()) })
    }

    fn begin(&mut self) -> BoxFuture<'_, Result<Transaction<'_, HttpDb>, Error>>
    where
        Self: Sized,
    {
        Transaction::begin(self, None)
    }

    fn shrink_buffers(&mut self) {}

    fn flush(&mut self) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async { Ok(()) })
    }

    fn should_flush(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct HttpConnectOptions {
    #[allow(dead_code)]
    pub(crate) url: String,
}

impl FromStr for HttpConnectOptions {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            url: s.to_string(),
        })
    }
}

impl ConnectOptions for HttpConnectOptions {
    type Connection = HttpConnection;

    fn from_url(url: &Url) -> Result<Self, Error> {
        Ok(Self {
            url: url.to_string(),
        })
    }

    fn connect(&self) -> BoxFuture<'_, Result<HttpConnection, Error>>
    where
        HttpConnection: Sized,
    {
        Box::pin(async {
            Err(Error::Configuration(
                "use Pool::<Turso>::connect() or similar instead of Connection::connect".into(),
            ))
        })
    }

    fn log_statements(self, _level: LevelFilter) -> Self {
        self
    }

    fn log_slow_statements(self, _level: LevelFilter, _duration: Duration) -> Self {
        self
    }
}

impl fmt::Debug for HttpConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpConnection")
            .field("transaction_depth", &self.transaction_depth)
            .finish()
    }
}
