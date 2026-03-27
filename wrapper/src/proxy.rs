//! HTTP reverse proxy that sits in front of Meilisearch.
//!
//! - GET/DELETE/PATCH requests are forwarded as-is.
//! - POST requests to `/indexes/*` are intercepted: the proxy forwards the
//!   request, extracts the `taskUid` from the response, polls until the task
//!   completes, and returns the final result synchronously.
//! - OPTIONS requests return an empty 200 for CORS preflight.

use crate::config;

/// Reverse proxy state shared across all request handlers.
#[derive(Clone)]
pub struct Proxy {
    client: reqwest::Client,
}

/// Response shape for a newly enqueued Meilisearch task.
#[derive(serde::Deserialize)]
struct EnqueuedTask {
    #[serde(rename = "taskUid")]
    task_uid: u64,
}

/// Minimal task status response used during polling.
#[derive(serde::Deserialize)]
struct TaskStatus {
    status: String,
}

/// Copies incoming request headers into a new map for the upstream request.
fn sanitize_request_headers(headers: &axum::http::HeaderMap) -> reqwest::header::HeaderMap {
    let mut sanitized = reqwest::header::HeaderMap::new();
    for (key, value) in headers.iter() {
        sanitized.insert(key, value.clone());
    }
    return sanitized;
}

/// Builds an outgoing response, filtering out hop-by-hop headers listed in
/// [`config::HEADERS_TO_SKIP`].
fn build_response(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: bytes::Bytes,
) -> axum::response::Response {
    let mut response = axum::response::Response::builder().status(status.as_u16());
    for (key, value) in headers.iter() {
        if config::HEADERS_TO_SKIP.contains(&key.as_str()) {
            continue;
        }
        response = response.header(key, value);
    }
    return response.body(axum::body::Body::from(body)).unwrap();
}

impl Proxy {
    /// Creates a new proxy with a default HTTP client.
    pub fn new() -> Self {
        return Self {
            client: reqwest::Client::new(),
        };
    }

    /// Builds the axum router with all route handlers.
    pub fn router(self) -> axum::Router {
        return axum::Router::new()
            // CORS preflight
            .route("/{*path}", axum::routing::options(Self::options_handler))
            // Special synchronous handling: any POST to /indexes/
            .route(
                "/indexes/{*rest}",
                axum::routing::post(Self::index_post_handler).fallback(Self::proxy_handler),
            )
            // Anything else that can be proxied as normal
            .fallback(axum::routing::any(Self::proxy_handler))
            .with_state(self);
    }

    /// Handles CORS preflight requests with an empty 200 response.
    async fn options_handler() -> axum::response::Response {
        return axum::response::Response::builder()
            .status(200)
            .header("content-length", "0")
            .body(axum::body::Body::empty())
            .unwrap();
    }

    /// Polls Meilisearch's `/tasks/{uid}` endpoint until the task reaches a
    /// terminal state (`succeeded`, `failed`, or `canceled`) or the timeout
    /// expires. Returns the final task JSON body on success.
    async fn wait_for_task(
        &self,
        task_uid: u64,
        headers: &reqwest::header::HeaderMap,
    ) -> Result<bytes::Bytes, String> {
        let url = format!("{}/tasks/{}", config::MEILISEARCH_HOST, task_uid);
        let timeout_at = std::time::Instant::now() + *config::MAX_WAIT_TIME;
        let poll_interval = *config::POLL_INTERVAL;

        tracing::debug!(
            task_uid = task_uid,
            timeout = ?config::MAX_WAIT_TIME,
            "polling task status"
        );

        while std::time::Instant::now() < timeout_at {
            match self.client.get(&url).headers(headers.clone()).send().await {
                Ok(resp) => {
                    let status_code = resp.status();
                    let body = resp
                        .bytes()
                        .await
                        .map_err(|e| format!("failed to read task response: {}", e))?;

                    if status_code.is_client_error() || status_code.is_server_error() {
                        return Err(format!("error fetching task {}: {}", task_uid, status_code));
                    }

                    let task: TaskStatus = serde_json::from_slice(&body)
                        .map_err(|e| format!("failed to parse task response: {}", e))?;

                    tracing::debug!(task_uid = task_uid, status = %task.status, "task poll");
                    match task.status.as_str() {
                        "succeeded" => {
                            tracing::info!(task_uid = task_uid, "task succeeded");
                            return Ok(body);
                        }
                        "failed" | "canceled" => {
                            return Err(format!(
                                "task {} terminal state: {}",
                                task_uid, task.status
                            ));
                        }
                        _ => {} // still processing
                    }
                }
                Err(e) => {
                    tracing::debug!(task_uid = task_uid, error = %e, "task poll request failed");
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        return Err(format!("timed out waiting for task {}", task_uid));
    }

    /// Intercepts POST requests to `/indexes/*`. Forwards the request to
    /// Meilisearch, extracts the `taskUid` from the response, and polls until
    /// the task completes — turning the async operation into a synchronous one.
    /// If the response doesn't contain a `taskUid`, it's returned as-is.
    async fn index_post_handler(
        axum::extract::State(proxy): axum::extract::State<Self>,
        request: axum::extract::Request,
    ) -> axum::response::Response {
        let url = format!("{}{}", config::MEILISEARCH_HOST, request.uri());
        let headers = sanitize_request_headers(request.headers());

        tracing::info!(url = %url, "intercepted index POST");

        let body_bytes = axum::body::to_bytes(request.into_body(), *config::MAX_REQUEST_BODY_SIZE)
            .await
            .unwrap_or_default();

        // Forward the POST to MeiliSearch
        let upstream_response = match proxy
            .client
            .post(&url)
            .headers(headers.clone())
            .body(body_bytes)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(error = %e, "upstream POST failed");
                return axum::response::Response::builder()
                    .status(502)
                    .body(axum::body::Body::from(format!("proxy error: {}", e)))
                    .unwrap();
            }
        };

        let resp_status = upstream_response.status();
        let resp_body = match upstream_response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!(error = %e, "failed to read upstream response");
                return axum::response::Response::builder()
                    .status(502)
                    .body(axum::body::Body::from(format!("proxy error: {}", e)))
                    .unwrap();
            }
        };

        // Try to extract taskUid — if not present, return the original response
        let enqueued: EnqueuedTask = match serde_json::from_slice(&resp_body) {
            Ok(t) => t,
            Err(_) => {
                tracing::warn!(status = %resp_status, "no taskUid in response, returning as-is");
                return axum::response::Response::builder()
                    .status(resp_status)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(resp_body))
                    .unwrap();
            }
        };

        tracing::info!(task_uid = enqueued.task_uid, "waiting for task to complete");

        // Poll until the task completes
        match proxy.wait_for_task(enqueued.task_uid, &headers).await {
            Ok(task_body) => {
                return axum::response::Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(task_body))
                    .unwrap();
            }
            Err(e) => {
                tracing::error!(task_uid = enqueued.task_uid, error = %e, "task polling failed");
                return axum::response::Response::builder()
                    .status(500)
                    .body(axum::body::Body::from(format!("task polling error: {}", e)))
                    .unwrap();
            }
        }
    }

    /// Generic pass-through handler for all non-intercepted requests. Buffers
    /// the full request/response bodies and strips hop-by-hop headers.
    async fn proxy_handler(
        axum::extract::State(proxy): axum::extract::State<Self>,
        request: axum::extract::Request,
    ) -> axum::response::Response {
        let url = format!("{}{}", config::MEILISEARCH_HOST, request.uri());
        let method = request.method().clone();

        tracing::info!(method = %method, url = %url, "proxying request");

        let headers = sanitize_request_headers(request.headers());
        let body_bytes = axum::body::to_bytes(request.into_body(), *config::MAX_REQUEST_BODY_SIZE)
            .await
            .unwrap_or_default();

        let upstream_response = proxy
            .client
            .request(method, &url)
            .headers(headers)
            .body(body_bytes)
            .send()
            .await;

        match upstream_response {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let resp_body = resp.bytes().await.unwrap_or_default();
                return build_response(status, &headers, resp_body);
            }
            Err(e) => {
                tracing::error!(error = %e, "upstream request failed");
                return axum::response::Response::builder()
                    .status(502)
                    .body(axum::body::Body::from(format!("proxy error: {}", e)))
                    .unwrap();
            }
        }
    }
}
