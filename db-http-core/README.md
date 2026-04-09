# db-http-core

Shared core library for the `db-http` workspace — provides the `DatabaseBackend` trait, query/transaction builders, error types, and HTTP abstraction.

## Install

```sh
cargo add db-http-core
```

## Quick Start

Implement `DatabaseBackend` for your own database:

```rust
use db_http_core::{DatabaseBackend, Query, QueryResult, Transaction, Error};

struct MyBackend;

impl DatabaseBackend for MyBackend {
    async fn execute_query(&self, query: &Query) -> Result<QueryResult, Error> {
        // Build and send your HTTP request here
        todo!()
    }

    async fn execute_transaction(&self, transaction: &Transaction) -> Result<Vec<QueryResult>, Error> {
        // Execute multiple queries in a transaction
        todo!()
    }
}
```

Build queries and transactions with the provided builders:

```rust
use db_http_core::{QueryBuilder, TransactionBuilder};

let users: Vec<User> = QueryBuilder::new(&backend, "SELECT * FROM users WHERE id = $1")
    .bind(1)
    .fetch_all()
    .await?;

let results = TransactionBuilder::new(&backend)
    .query("INSERT INTO users (name) VALUES ($1)")
    .bind("alice")
    .query("INSERT INTO users (name) VALUES ($1)")
    .bind("bob")
    .execute()
    .await?;
```

## API Reference

| Item | Kind | Description |
|---|---|---|
| `DatabaseBackend` | trait | Implement to add a new database backend |
| `QueryBuilder` | struct | Fluent builder for single queries |
| `TransactionBuilder` | struct | Fluent builder for multi-query transactions |
| `Query` | struct | A prepared query with SQL + parameters |
| `Transaction` | struct | A set of queries to execute atomically |
| `QueryResult` | struct | Rows + columns returned by a query |
| `Column` | struct | Column name and type metadata |
| `Error` | enum | Error type used across all crates |
| `HttpRequest` | struct | Minimal HTTP request abstraction |
| `http_post` | fn | Send an HTTP POST (uses `reqwest` or WASI) |
| `deserialize_one` | fn | Deserialize a single row from a `QueryResult` |
| `deserialize_all` | fn | Deserialize all rows from a `QueryResult` |

## WASM/WASI Support

`db-http-core` compiles for both native (`reqwest` + `tokio`) and WASI (`wstd`) targets. The `http_post` function automatically uses the correct HTTP client based on the compilation target.

## Contributing

1. Fork and clone the repository
2. Make your changes
3. Run `cargo fmt` and `cargo clippy`
4. Run tests: `cargo test`
5. Open a pull request

## License

MIT — see [LICENSE](../LICENSE).
