# neon-http

HTTP client for [Neon](https://neon.tech) Serverless Postgres.

> **Note:** This crate is also compatible with
> [PlanetScale Postgres](https://planetscale.com/docs/postgres/connecting/neon-serverless-driver),
> which uses the Neon serverless driver protocol.

## Install

```sh
cargo add neon-http
```

## Quick Start

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct User {
    id: i64,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), neon_http::Error> {
    let client = neon_http::Client::new_from_env()?;

    // Single query with $1-style placeholders
    let users: Vec<User> = client
        .query("SELECT id, name FROM users WHERE id = $1")
        .bind(1)
        .fetch_all()
        .await?;

    println!("{:?}", users);

    // Transaction
    let results = client
        .transaction()
        .query("INSERT INTO users (name) VALUES ($1)")
        .bind("alice")
        .query("INSERT INTO users (name) VALUES ($1)")
        .bind("bob")
        .execute()
        .await?;

    println!("Inserted {} rows", results.len());
    Ok(())
}
```

## API Reference

| Item | Kind | Description |
|---|---|---|
| `Client` | struct | Neon HTTP client |
| `Client::new(connection_string)` | fn | Create a client with a connection string |
| `Client::new_from_env()` | fn | Create a client from environment variables |
| `Client::query(sql)` | fn | Start building a query (returns `QueryBuilder`) |
| `Client::transaction()` | fn | Start building a transaction (returns `TransactionBuilder`) |
| `Client::execute(query)` | fn | Execute a pre-built `Query` directly |
| `Error` | enum | Error type (re-exported from `db-http-core`) |
| `QueryResult` | struct | Rows + columns returned by a query |
| `Column` | struct | Column name and type metadata |
| `Transaction` | struct | A set of queries to execute atomically |

### Placeholder style

Use `$1`, `$2`, ... for positional parameters (standard Postgres syntax).

## Environment Variables

| Variable | Description |
|---|---|
| `NEON_CONNECTION_STRING` | Postgres connection string (e.g. `postgres://user:pass@host/db`) |

## WASM/WASI Support

This crate compiles for both native and WASI targets. The underlying HTTP client (`db-http-core`) automatically selects the correct implementation.

## Contributing

1. Fork and clone the repository
2. Make your changes
3. Run `cargo fmt` and `cargo clippy`
4. Run tests: `cargo test`
5. Open a pull request

## License

MIT — see [LICENSE](../LICENSE).
