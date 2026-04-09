// Internal implementation modules (prefixed to avoid name conflicts with public API modules)
mod arguments;
mod column;
mod connection;
mod convert;
#[path = "database.rs"]
mod db;
#[path = "decode.rs"]
mod decode_impl;
#[path = "encode.rs"]
mod encode_impl;
mod pool;
mod query_result;
mod row;
mod statement;
mod transaction;
mod transaction_manager;
mod type_info;
#[path = "types.rs"]
mod types_impl;
mod value;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Re-export sqlx-core traits at root (matches sqlx)
// ---------------------------------------------------------------------------

pub use sqlx_core::arguments::{Arguments, IntoArguments};
pub use sqlx_core::column::{Column, ColumnIndex};
pub use sqlx_core::connection::{ConnectOptions, Connection};
pub use sqlx_core::describe::Describe;
pub use sqlx_core::executor::{Execute, Executor};
pub use sqlx_core::from_row::FromRow;
pub use sqlx_core::raw_sql::{raw_sql, RawSql};
pub use sqlx_core::row::Row;
pub use sqlx_core::statement::Statement;
pub use sqlx_core::transaction::TransactionManager;
pub use sqlx_core::type_info::TypeInfo;
pub use sqlx_core::value::{Value, ValueRef};

// Re-export Either (used by Executor streams)
pub use either::Either;

// Our concrete types
pub use crate::arguments::HttpArguments;
pub use crate::column::HttpColumn;
pub use crate::db::HttpDb;
pub use crate::pool::{Neon, PlanetScale, Pool, Turso};
pub use crate::query_result::HttpQueryResult;
pub use crate::row::HttpRow;
pub use crate::statement::HttpStatement;
pub use crate::transaction::{
    FromTransactionResult, FromTransactionResults, HttpTransaction, TransactionResult,
};
pub use crate::type_info::HttpTypeInfo;
pub use crate::value::{HttpValue, HttpValueRef};

// Re-export derive macros at root (matching sqlx)
// Type, Encode, Decode derives come through the submodule re-exports below
pub use sqlx_http_macros::FromRow;

// ---------------------------------------------------------------------------
// Public submodules (matching sqlx's module structure)
// ---------------------------------------------------------------------------

/// Database trait and related types.
pub mod database {
    pub use sqlx_core::database::*;
}

/// Error types.
pub mod error {
    pub use sqlx_core::error::*;
}

/// Provides [`Encode`](sqlx_core::encode::Encode) for encoding values for the database.
pub mod encode {
    pub use sqlx_core::encode::{Encode, IsNull};

    pub use sqlx_http_macros::Encode;
}

/// Provides [`Decode`](sqlx_core::decode::Decode) for decoding values from the database.
pub mod decode {
    pub use sqlx_core::decode::Decode;

    pub use sqlx_http_macros::Decode;
}

// Root-level trait re-exports (matching sqlx)
pub use self::database::Database;
pub use self::encode::Encode;
pub use self::decode::Decode;
pub use self::error::{Error, Result};
pub use self::types::Type;

/// Types and traits for the `query` family of functions and macros.
pub mod query {
    pub use sqlx_core::query::{query, query_with, Map, Query};
    pub use sqlx_core::query_as::{query_as, query_as_with, QueryAs};
    pub use sqlx_core::query_scalar::{query_scalar, query_scalar_with, QueryScalar};
}

/// Query builder for dynamic queries.
pub mod query_builder {
    pub use sqlx_core::query_builder::*;
}

pub use sqlx_core::query::{query, query_with};
pub use sqlx_core::query_as::{query_as, query_as_with};
pub use sqlx_core::query_builder::QueryBuilder;
pub use sqlx_core::query_scalar::{query_scalar, query_scalar_with};

/// Conversions between Rust and SQL types.
pub mod types {
    pub use sqlx_core::types::{Text, Type};

    pub use crate::types_impl::JsonArray;

    #[cfg(feature = "json")]
    pub use sqlx_core::types::{Json, JsonRawValue, JsonValue};

    #[cfg(feature = "uuid")]
    #[doc(no_inline)]
    pub use uuid::{self, Uuid};

    #[cfg(feature = "chrono")]
    pub mod chrono {
        #[doc(no_inline)]
        pub use chrono::{
            DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc,
        };
    }

    #[cfg(feature = "time")]
    pub mod time {
        #[doc(no_inline)]
        pub use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
    }

    #[cfg(feature = "bigdecimal")]
    #[doc(no_inline)]
    pub use bigdecimal::BigDecimal;

    #[cfg(feature = "rust_decimal")]
    #[doc(no_inline)]
    pub use rust_decimal::Decimal;

    pub use sqlx_http_macros::Type;
}

// Re-export JsonArray at root for convenience
pub use crate::types_impl::JsonArray;

/// Convenience re-export of common traits.
pub mod prelude {
    pub use super::ConnectOptions;
    pub use super::Connection;
    pub use super::Decode;
    pub use super::Encode;
    pub use super::Executor;
    pub use super::FromRow;
    pub use super::IntoArguments;
    pub use super::Row;
    pub use super::Statement;
    pub use super::Type;
}

// Hidden module for derive macro internals
#[doc(hidden)]
pub mod __private {
    pub use serde_json;
    pub mod encode {
        pub use sqlx_core::encode::*;
    }
    pub mod error {
        pub use sqlx_core::error::*;
    }
}

// ---------------------------------------------------------------------------
// Convenience free functions (fixing DB = HttpDb)
// ---------------------------------------------------------------------------

/// Create a query with `DB = HttpDb`.
pub fn http_query(sql: &str) -> sqlx_core::query::Query<'_, HttpDb, HttpArguments> {
    sqlx_core::query::query(sql)
}

/// Create a query with explicit arguments.
pub fn http_query_with<'q, A>(
    sql: &'q str,
    arguments: A,
) -> sqlx_core::query::Query<'q, HttpDb, A>
where
    A: sqlx_core::arguments::IntoArguments<'q, HttpDb>,
{
    sqlx_core::query::query_with(sql, arguments)
}

/// Create a typed query with `DB = HttpDb`.
pub fn http_query_as<O>(
    sql: &str,
) -> sqlx_core::query_as::QueryAs<'_, HttpDb, O, HttpArguments>
where
    O: for<'r> FromRow<'r, HttpRow>,
{
    sqlx_core::query_as::query_as(sql)
}

/// Create a typed query with explicit arguments.
pub fn http_query_as_with<'q, O, A>(
    sql: &'q str,
    arguments: A,
) -> sqlx_core::query_as::QueryAs<'q, HttpDb, O, A>
where
    O: for<'r> FromRow<'r, HttpRow>,
    A: sqlx_core::arguments::IntoArguments<'q, HttpDb>,
{
    sqlx_core::query_as::query_as_with(sql, arguments)
}

/// Create a scalar query with `DB = HttpDb`.
pub fn http_query_scalar<O>(
    sql: &str,
) -> sqlx_core::query_scalar::QueryScalar<'_, HttpDb, O, HttpArguments>
where
    (O,): for<'r> FromRow<'r, HttpRow>,
{
    sqlx_core::query_scalar::query_scalar(sql)
}

/// Create a scalar query with explicit arguments.
pub fn http_query_scalar_with<'q, O, A>(
    sql: &'q str,
    arguments: A,
) -> sqlx_core::query_scalar::QueryScalar<'q, HttpDb, O, A>
where
    (O,): for<'r> FromRow<'r, HttpRow>,
    A: sqlx_core::arguments::IntoArguments<'q, HttpDb>,
{
    sqlx_core::query_scalar::query_scalar_with(sql, arguments)
}

// ---------------------------------------------------------------------------
// Query macros (no compile-time checking, same call syntax as sqlx)
// ---------------------------------------------------------------------------

/// Execute a SQL query. Same syntax as `sqlx::query!` but without compile-time checking.
///
/// Returns a `Query<HttpDb>` — use `.fetch_one()`, `.fetch_all()`, etc.
/// Results are `HttpRow` (use `.get()` / `.try_get()` to extract values).
#[macro_export]
macro_rules! query {
    ($sql:expr) => {
        $crate::http_query($sql)
    };
    ($sql:expr, $($args:expr),+ $(,)?) => {
        $crate::http_query($sql)$(.bind($args))+
    };
}

/// Same as [`query!`] (no compile-time checking is performed either way).
#[macro_export]
macro_rules! query_unchecked {
    ($($tt:tt)*) => { $crate::query!($($tt)*) };
}

/// Execute a SQL query and map results to a type implementing `FromRow`.
///
/// ```ignore
/// let users = query_as!(User, "SELECT * FROM users WHERE id = ?", id)
///     .fetch_all(&pool).await?;
/// ```
#[macro_export]
macro_rules! query_as {
    ($out:ty, $sql:expr) => {
        $crate::http_query_as::<$out>($sql)
    };
    ($out:ty, $sql:expr, $($args:expr),+ $(,)?) => {
        $crate::http_query_as::<$out>($sql)$(.bind($args))+
    };
}

/// Same as [`query_as!`] (no compile-time checking is performed either way).
#[macro_export]
macro_rules! query_as_unchecked {
    ($($tt:tt)*) => { $crate::query_as!($($tt)*) };
}

/// Execute a SQL query and return a single scalar value.
#[macro_export]
macro_rules! query_scalar {
    ($sql:expr) => {
        $crate::http_query_scalar::<_>($sql)
    };
    ($sql:expr, $($args:expr),+ $(,)?) => {
        $crate::http_query_scalar::<_>($sql)$(.bind($args))+
    };
}

/// Same as [`query_scalar!`] (no compile-time checking is performed either way).
#[macro_export]
macro_rules! query_scalar_unchecked {
    ($($tt:tt)*) => { $crate::query_scalar!($($tt)*) };
}

/// Execute a SQL query loaded from a file.
///
/// The file path is relative to the crate root (where `Cargo.toml` is).
#[macro_export]
macro_rules! query_file {
    ($path:expr) => {
        $crate::http_query(include_str!($path))
    };
    ($path:expr, $($args:expr),+ $(,)?) => {
        $crate::http_query(include_str!($path))$(.bind($args))+
    };
}

/// Same as [`query_file!`] (no compile-time checking is performed either way).
#[macro_export]
macro_rules! query_file_unchecked {
    ($($tt:tt)*) => { $crate::query_file!($($tt)*) };
}

/// Execute a SQL query loaded from a file and map results to a type.
#[macro_export]
macro_rules! query_file_as {
    ($out:ty, $path:expr) => {
        $crate::http_query_as::<$out>(include_str!($path))
    };
    ($out:ty, $path:expr, $($args:expr),+ $(,)?) => {
        $crate::http_query_as::<$out>(include_str!($path))$(.bind($args))+
    };
}

/// Same as [`query_file_as!`] (no compile-time checking is performed either way).
#[macro_export]
macro_rules! query_file_as_unchecked {
    ($($tt:tt)*) => { $crate::query_file_as!($($tt)*) };
}

/// Execute a SQL query loaded from a file and return a scalar.
#[macro_export]
macro_rules! query_file_scalar {
    ($path:expr) => {
        $crate::http_query_scalar::<_>(include_str!($path))
    };
    ($path:expr, $($args:expr),+ $(,)?) => {
        $crate::http_query_scalar::<_>(include_str!($path))$(.bind($args))+
    };
}

/// Same as [`query_file_scalar!`] (no compile-time checking is performed either way).
#[macro_export]
macro_rules! query_file_scalar_unchecked {
    ($($tt:tt)*) => { $crate::query_file_scalar!($($tt)*) };
}
