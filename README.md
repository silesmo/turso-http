# db-http

Lightweight Rust clients for serverless databases over HTTP.

## Crates

| Crate | Database |
|---|---|
| `turso-http` | [Turso](https://turso.tech) (libSQL) |
| `neon-http` | [Neon](https://neon.tech) Serverless Postgres |
| `planetscale-http` | [PlanetScale](https://planetscale.com) MySQL over HTTP |
| `db-http-core` | Shared core (query builder, deserializer, error types) |

> **Note:** The `neon-http` crate also works with
> [PlanetScale Postgres](https://planetscale.com/docs/postgres/connecting/neon-serverless-driver),
> which is compatible with the Neon serverless driver protocol.

## Usage

### Turso

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct User {
    id: i64,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), turso_http::Error> {
    let client = turso_http::Client::new_from_env()?;

    let users: Vec<User> = client
        .query("SELECT id, name FROM users WHERE id = $1")
        .bind(1)
        .fetch_all()
        .await?;

    println!("{:?}", users);
    Ok(())
}
```

Environment variables: `TURSO_DATABASE_URL`, `TURSO_AUTH_TOKEN`

### Neon / PlanetScale Postgres

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

    let users: Vec<User> = client
        .query("SELECT id, name FROM users WHERE id = $1")
        .bind(1)
        .fetch_all()
        .await?;

    println!("{:?}", users);
    Ok(())
}
```

Environment variable: `NEON_CONNECTION_STRING`

### PlanetScale MySQL

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

    let users: Vec<User> = client
        .query("SELECT id, name FROM users WHERE id = $1")
        .bind(1)
        .fetch_all()
        .await?;

    println!("{:?}", users);
    Ok(())
}
```

Environment variables: `PLANETSCALE_HOST`, `PLANETSCALE_USERNAME`, `PLANETSCALE_PASSWORD`
