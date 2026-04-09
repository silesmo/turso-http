pub mod backend;
pub mod wire;

use backend::TursoBackend;
use db_http_core::{Query, QueryBuilder, TransactionBuilder};

pub use db_http_core::{Column, Error, QueryResult, Transaction};

pub struct Client {
    backend: TursoBackend,
}

impl Client {
    /// Create a new client from a hostname and auth token.
    ///
    /// The `host` can be a bare hostname (e.g. `my-db.turso.io`) or a full
    /// URL with scheme (`https://my-db.turso.io`). If no scheme is provided,
    /// `https://` is prepended.
    pub fn new(host: &str, auth_token: &str) -> Result<Self, Error> {
        if host.is_empty() {
            return Err(Error::Config("host cannot be empty".to_string()));
        }
        if auth_token.is_empty() {
            return Err(Error::Config("auth_token cannot be empty".to_string()));
        }

        let base_url = if host.starts_with("http://") || host.starts_with("https://") {
            host.to_string()
        } else {
            format!("https://{host}")
        };

        Ok(Self {
            backend: TursoBackend {
                base_url,
                auth_token: auth_token.to_string(),
            },
        })
    }

    /// Create a new client from a `libsql://` URL.
    ///
    /// Parses URLs in the format:
    /// - `libsql://hostname.turso.io?authToken=TOKEN`
    /// - `libsql://hostname.turso.io` (requires separate auth_token)
    ///
    /// Also accepts `https://` URLs for convenience.
    pub fn new_from_url(url: &str, auth_token_override: Option<&str>) -> Result<Self, Error> {
        let (host, embedded_token) = parse_turso_url(url)?;
        let token = auth_token_override
            .or(embedded_token.as_deref())
            .ok_or_else(|| {
                Error::Config(
                    "No auth token: provide auth_token or use a URL with ?authToken=".to_string(),
                )
            })?;
        Self::new(&host, token)
    }

    /// Create a new client from environment variables.
    ///
    /// Reads `TURSO_DATABASE_URL` (supports `libsql://` URLs with embedded
    /// `?authToken=` or bare hostnames) and `TURSO_AUTH_TOKEN`. If
    /// `TURSO_AUTH_TOKEN` is set, it takes precedence over any token embedded
    /// in the URL.
    pub fn new_from_env() -> Result<Self, Error> {
        let url = std::env::var("TURSO_DATABASE_URL")
            .map_err(|_| Error::Config("TURSO_DATABASE_URL must be set".to_string()))?;
        let explicit_token = std::env::var("TURSO_AUTH_TOKEN").ok();
        Self::new_from_url(&url, explicit_token.as_deref())
    }

    pub fn query(&self, sql: &str) -> QueryBuilder<&TursoBackend> {
        QueryBuilder::new(&self.backend, sql)
    }

    pub fn transaction(&self) -> TransactionBuilder<&TursoBackend> {
        TransactionBuilder::new(&self.backend)
    }

    pub async fn execute(&self, query: Query) -> Result<QueryResult, Error> {
        use db_http_core::DatabaseBackend;
        self.backend.execute_query(&query).await
    }
}

/// Parse a Turso database URL into (hostname, optional embedded auth token).
///
/// Supported formats:
/// - `libsql://hostname.turso.io?authToken=TOKEN` → ("hostname.turso.io", Some("TOKEN"))
/// - `libsql://hostname.turso.io` → ("hostname.turso.io", None)
/// - `https://hostname.turso.io` → ("hostname.turso.io", None)
/// - `hostname.turso.io` → ("hostname.turso.io", None)
fn parse_turso_url(url: &str) -> Result<(String, Option<String>), Error> {
    // Strip scheme
    let without_scheme = url
        .strip_prefix("libsql://")
        .or_else(|| url.strip_prefix("https://"))
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Split on '?' to separate host from query params
    let (host, query) = match without_scheme.split_once('?') {
        Some((h, q)) => (h, Some(q)),
        None => (without_scheme, None),
    };

    // Strip trailing path (e.g. /v2/pipeline)
    let host = host.split('/').next().unwrap_or(host);

    if host.is_empty() {
        return Err(Error::Config(format!("Cannot parse host from URL: {}", url)));
    }

    // Extract authToken from query params if present
    let token = query.and_then(|q| {
        q.split('&')
            .find_map(|param| param.strip_prefix("authToken="))
            .map(|t| t.to_string())
    });

    Ok((host.to_string(), token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_libsql_url_with_token() {
        let (host, token) = parse_turso_url(
            "libsql://my-db-abc123.aws-eu-west-1.turso.io?authToken=eyJhbGciOiJFZERTQSJ9.test",
        )
        .unwrap();
        assert_eq!(host, "my-db-abc123.aws-eu-west-1.turso.io");
        assert_eq!(token.as_deref(), Some("eyJhbGciOiJFZERTQSJ9.test"));
    }

    #[test]
    fn test_parse_libsql_url_without_token() {
        let (host, token) = parse_turso_url("libsql://my-db.turso.io").unwrap();
        assert_eq!(host, "my-db.turso.io");
        assert!(token.is_none());
    }

    #[test]
    fn test_parse_https_url() {
        let (host, token) = parse_turso_url("https://my-db.turso.io").unwrap();
        assert_eq!(host, "my-db.turso.io");
        assert!(token.is_none());
    }

    #[test]
    fn test_parse_bare_hostname() {
        let (host, token) = parse_turso_url("my-db.turso.io").unwrap();
        assert_eq!(host, "my-db.turso.io");
        assert!(token.is_none());
    }

    #[test]
    fn test_parse_empty_url() {
        assert!(parse_turso_url("").is_err());
        assert!(parse_turso_url("libsql://").is_err());
    }

    #[test]
    fn test_new_from_url_with_embedded_token() {
        let client = Client::new_from_url(
            "libsql://my-db.turso.io?authToken=secret123",
            None,
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_new_from_url_override_token() {
        let client = Client::new_from_url(
            "libsql://my-db.turso.io?authToken=embedded",
            Some("override"),
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_new_from_url_no_token_fails() {
        let result = Client::new_from_url("libsql://my-db.turso.io", None);
        assert!(result.is_err());
    }
}
