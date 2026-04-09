use std::fmt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use either::Either;
use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;
use futures_util::StreamExt;
use sqlx_core::describe::Describe;
use sqlx_core::error::Error;
use sqlx_core::executor::{Execute, Executor};

use crate::column::HttpColumn;
use crate::convert::{convert_core_result, to_core_query};
use crate::db::HttpDb;
use crate::query_result::HttpQueryResult;
use crate::row::HttpRow;
use crate::statement::HttpStatement;
use crate::transaction::HttpTransaction;
use crate::type_info::HttpTypeInfo;

// ---------------------------------------------------------------------------
// Marker types for backends
// ---------------------------------------------------------------------------

/// Marker type for the Turso/libSQL backend.
pub struct Turso;

/// Marker type for the Neon Postgres backend.
pub struct Neon;

/// Marker type for the PlanetScale MySQL backend.
pub struct PlanetScale;

// ---------------------------------------------------------------------------
// Object-safe wrapper for DatabaseBackend (which uses RPITIT)
// ---------------------------------------------------------------------------

pub(crate) trait DynBackend: Send + Sync {
    fn execute_query<'a>(
        &'a self,
        q: &'a db_http_core::Query,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<db_http_core::QueryResult, db_http_core::Error>,
                > + Send
                + 'a,
        >,
    >;

    fn execute_transaction<'a>(
        &'a self,
        t: &'a db_http_core::Transaction,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<db_http_core::QueryResult>, db_http_core::Error>,
                > + Send
                + 'a,
        >,
    >;
}

impl<B: db_http_core::DatabaseBackend + Send + Sync + 'static> DynBackend for B {
    fn execute_query<'a>(
        &'a self,
        q: &'a db_http_core::Query,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<db_http_core::QueryResult, db_http_core::Error>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(db_http_core::DatabaseBackend::execute_query(self, q))
    }

    fn execute_transaction<'a>(
        &'a self,
        t: &'a db_http_core::Transaction,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<db_http_core::QueryResult>, db_http_core::Error>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(db_http_core::DatabaseBackend::execute_transaction(self, t))
    }
}

// ---------------------------------------------------------------------------
// Pool<B>
// ---------------------------------------------------------------------------

/// An HTTP-backed database pool, parameterized by backend marker type.
///
/// Use [`Pool<Turso>`], [`Pool<Neon>`], or [`Pool<PlanetScale>`] depending on
/// your database provider. Each variant has its own `connect` and `from_env`
/// constructors.
///
/// ```rust,ignore
/// let pool = Pool::<Turso>::connect("mydb.turso.io", "my-token")?;
/// let pool = Pool::<Neon>::connect("postgres://...")?;
/// let pool = Pool::<PlanetScale>::connect("host", "user", "pass")?;
/// ```
pub struct Pool<B> {
    pub(crate) inner: Arc<dyn DynBackend>,
    _marker: PhantomData<B>,
}

impl<B> Clone for Pool<B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: PhantomData,
        }
    }
}

impl<B> fmt::Debug for Pool<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pool").finish()
    }
}

// ---------------------------------------------------------------------------
// Per-backend constructors
// ---------------------------------------------------------------------------

impl Pool<Turso> {
    pub fn connect(host: &str, auth_token: &str) -> Result<Self, Error> {
        let backend = turso_http::backend::TursoBackend::new(host, auth_token)
            .map_err(|e| Error::Configuration(e.to_string().into()))?;
        Ok(Self {
            inner: Arc::new(backend),
            _marker: PhantomData,
        })
    }

    /// Create a pool from environment variables.
    ///
    /// Reads `TURSO_DATABASE_URL` (supports `libsql://` URLs with embedded
    /// `?authToken=` or bare hostnames) and `TURSO_AUTH_TOKEN`. If
    /// `TURSO_AUTH_TOKEN` is set, it takes precedence over any token in the URL.
    pub fn from_env() -> Result<Self, Error> {
        let url = std::env::var("TURSO_DATABASE_URL")
            .map_err(|_| Error::Configuration("TURSO_DATABASE_URL must be set".into()))?;
        let explicit_token = std::env::var("TURSO_AUTH_TOKEN").ok();

        // Parse libsql:// or https:// URLs to extract hostname
        let without_scheme = url
            .strip_prefix("libsql://")
            .or_else(|| url.strip_prefix("https://"))
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(&url);

        let (host_part, query) = match without_scheme.split_once('?') {
            Some((h, q)) => (h, Some(q)),
            None => (without_scheme, None),
        };

        let host = host_part.split('/').next().unwrap_or(host_part);
        if host.is_empty() {
            return Err(Error::Configuration(
                format!("Cannot parse host from TURSO_DATABASE_URL: {}", url).into(),
            ));
        }

        // Extract authToken from query params if no explicit token
        let embedded_token = query.and_then(|q| {
            q.split('&')
                .find_map(|param| param.strip_prefix("authToken="))
                .map(|t| t.to_string())
        });

        let token = explicit_token
            .or(embedded_token)
            .ok_or_else(|| {
                Error::Configuration(
                    "TURSO_AUTH_TOKEN must be set or TURSO_DATABASE_URL must contain ?authToken=".into(),
                )
            })?;

        Self::connect(host, &token)
    }
}

impl Pool<Neon> {
    pub fn connect(connection_string: &str) -> Result<Self, Error> {
        let backend = neon_http::backend::NeonBackend::new(connection_string)
            .map_err(|e| Error::Configuration(e.to_string().into()))?;
        Ok(Self {
            inner: Arc::new(backend),
            _marker: PhantomData,
        })
    }

    pub fn from_env() -> Result<Self, Error> {
        let connection_string = std::env::var("NEON_CONNECTION_STRING")
            .map_err(|_| Error::Configuration("NEON_CONNECTION_STRING must be set".into()))?;
        Self::connect(&connection_string)
    }
}

impl Pool<PlanetScale> {
    pub fn connect(
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, Error> {
        let backend = planetscale_http::backend::PlanetScaleBackend::new(host, username, password)
            .map_err(|e| Error::Configuration(e.to_string().into()))?;
        Ok(Self {
            inner: Arc::new(backend),
            _marker: PhantomData,
        })
    }

    pub fn from_env() -> Result<Self, Error> {
        let host = std::env::var("PLANETSCALE_HOST")
            .map_err(|_| Error::Configuration("PLANETSCALE_HOST must be set".into()))?;
        let username = std::env::var("PLANETSCALE_USERNAME")
            .map_err(|_| Error::Configuration("PLANETSCALE_USERNAME must be set".into()))?;
        let password = std::env::var("PLANETSCALE_PASSWORD")
            .map_err(|_| Error::Configuration("PLANETSCALE_PASSWORD must be set".into()))?;
        Self::connect(&host, &username, &password)
    }
}

// ---------------------------------------------------------------------------
// Common methods for all Pool<B>
// ---------------------------------------------------------------------------

impl<B> Pool<B> {
    pub fn begin(&self) -> HttpTransaction {
        HttpTransaction::new(self.inner.clone())
    }
}

// ---------------------------------------------------------------------------
// Executor impl for all Pool<B>
// ---------------------------------------------------------------------------

impl<'c, B: Send + Sync + 'static> Executor<'c> for &Pool<B> {
    type Database = HttpDb;

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        mut query: E,
    ) -> BoxStream<'e, Result<Either<HttpQueryResult, HttpRow>, Error>>
    where
        'c: 'e,
        E: 'q + Execute<'q, HttpDb>,
    {
        let sql = query.sql().to_string();
        let args = match query.take_arguments() {
            Ok(args) => args,
            Err(e) => {
                return Box::pin(futures_util::stream::once(futures_util::future::ready(
                    Err(Error::Encode(e)),
                )));
            }
        };
        let pool = self.clone();

        Box::pin(
            futures_util::stream::once(async move {
                let core_query = to_core_query(&sql, args);
                match pool.inner.execute_query(&core_query).await {
                    Ok(result) => {
                        let (rows, query_result) = convert_core_result(result);
                        let mut items: Vec<Result<Either<HttpQueryResult, HttpRow>, Error>> =
                            Vec::with_capacity(1 + rows.len());
                        items.push(Ok(Either::Left(query_result)));
                        for row in rows {
                            items.push(Ok(Either::Right(row)));
                        }
                        futures_util::stream::iter(items).boxed()
                    }
                    Err(e) => futures_util::stream::once(futures_util::future::ready(Err(
                        Error::Protocol(e.to_string()),
                    )))
                    .boxed(),
                }
            })
            .flatten(),
        )
    }

    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        mut query: E,
    ) -> BoxFuture<'e, Result<Option<HttpRow>, Error>>
    where
        'c: 'e,
        E: 'q + Execute<'q, HttpDb>,
    {
        let sql = query.sql().to_string();
        let args = match query.take_arguments() {
            Ok(args) => args,
            Err(e) => return Box::pin(futures_util::future::ready(Err(Error::Encode(e)))),
        };
        let pool = self.clone();

        Box::pin(async move {
            let core_query = to_core_query(&sql, args);
            let result = pool
                .inner
                .execute_query(&core_query)
                .await
                .map_err(|e| Error::Protocol(e.to_string()))?;
            let (rows, _) = convert_core_result(result);
            Ok(rows.into_iter().next())
        })
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        _parameters: &'e [HttpTypeInfo],
    ) -> BoxFuture<'e, Result<HttpStatement<'q>, Error>>
    where
        'c: 'e,
    {
        Box::pin(async move {
            Ok(HttpStatement {
                sql: std::borrow::Cow::Borrowed(sql),
                columns: Vec::new(),
            })
        })
    }

    fn describe<'e, 'q: 'e>(
        self,
        _sql: &'q str,
    ) -> BoxFuture<'e, Result<Describe<HttpDb>, Error>>
    where
        'c: 'e,
    {
        Box::pin(async {
            Ok(Describe {
                columns: Vec::<HttpColumn>::new(),
                parameters: Some(Either::Right(0)),
                nullable: Vec::new(),
            })
        })
    }
}
