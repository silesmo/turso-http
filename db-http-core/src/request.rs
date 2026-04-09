use crate::error::Error;

pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[cfg(target_os = "wasi")]
pub async fn http_post(request: &HttpRequest) -> Result<String, Error> {
    use wstd::http::{Client, HeaderValue, Request};

    let mut builder = Request::post(&request.url);
    builder = builder.header("Content-Type", HeaderValue::from_static("application/json"));
    for (key, value) in &request.headers {
        let value = HeaderValue::from_str(value)
            .map_err(|e| Error::Http(format!("Invalid header value for '{key}': {e}")))?;
        builder = builder.header(key.as_str(), value);
    }

    let req = builder
        .body(request.body.clone().into_bytes())
        .map_err(|e| Error::Http(format!("Failed to build request: {e}")))?;

    let client = Client::new();
    let mut response = client
        .send(req)
        .await
        .map_err(|e| Error::Http(format!("Request failed: {e}")))?;

    response
        .body_mut()
        .str_contents()
        .await
        .map_err(|e| Error::Http(format!("Failed to read response: {e}")))
        .map(|s| s.to_string())
}

#[cfg(not(target_os = "wasi"))]
pub async fn http_post(request: &HttpRequest) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let mut builder = client
        .post(&request.url)
        .header("Content-Type", "application/json");

    for (key, value) in &request.headers {
        builder = builder.header(key.as_str(), value.as_str());
    }

    let response = builder
        .body(request.body.clone())
        .send()
        .await
        .map_err(|e| Error::Http(format!("Request failed: {e}")))?;

    response
        .text()
        .await
        .map_err(|e| Error::Http(format!("Failed to read response: {e}")))
}
