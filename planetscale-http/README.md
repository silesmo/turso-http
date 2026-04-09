# planetscale-http

HTTP client for [PlanetScale](https://planetscale.com) MySQL over HTTP.

## Install

```sh
cargo add planetscale-http
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
async fn main() -> Result<(), planetscale_http::Error> {
    let client = planetscale_http::Client::new_from_env()?;

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
| `Client` | struct | PlanetScale HTTP client |
| `Client::new(host, username, password)` | fn | Create a client with explicit credentials |
| `Client::new_from_env()` | fn | Create a client from environment variables |
| `Client::query(sql)` | fn | Start building a query (returns `QueryBuilder`) |
| `Client::transaction()` | fn | Start building a transaction (returns `TransactionBuilder`) |
| `Client::execute(query)` | fn | Execute a pre-built `Query` directly |
| `Error` | enum | Error type (re-exported from `db-http-core`) |
| `QueryResult` | struct | Rows + columns returned by a query |
| `Column` | struct | Column name and type metadata |
| `Transaction` | struct | A set of queries to execute atomically |

### Placeholder style

Both `$1, $2, ...` and MySQL-style `?` placeholders are supported. `$N` placeholders are automatically rewritten to `?` internally, providing a consistent API with the other `db-http` crates.

## Environment Variables

| Variable | Description |
|---|---|
| `PLANETSCALE_HOST` | PlanetScale database host |
| `PLANETSCALE_USERNAME` | Database username |
| `PLANETSCALE_PASSWORD` | Database password |

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
