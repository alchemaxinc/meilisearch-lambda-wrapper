//! HTTP reverse proxy that sits in front of Meilisearch.
//!
//! The proxy forwards every request to Meilisearch as-is. Meilisearch marks
//! an async write with a `taskUid` field in the response body. This is true
//! for every async operation: index, document, and settings writes, swaps,
//! dumps, snapshots, and task cancellation or deletion. Some of these
//! operations return `202 Accepted`. Others return `200 OK`. So the proxy
//! checks the response body for a `taskUid`, not the status code. When a
//! `taskUid` is present, the proxy polls `/tasks/{uid}` until the task
//! reaches a terminal state. On success, the proxy returns the final task
//! JSON with a 200 response. On failure, cancellation, or timeout, it
//! returns a 5xx error instead. If the caller's API key cannot poll task
//! status (it lacks the `tasks.get` action), the proxy cannot confirm the
//! outcome. In that case, it returns the original enqueued-task response
//! with no change, instead of a `500` that would misreport a write that
//! may have actually succeeded. A response without a `taskUid` (a search, a
//! read, or an error) passes through with no change.
//!
//! An OPTIONS request returns an empty 200 response for CORS preflight.

use std::error::Error;

use http_body_util::LengthLimitError;

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

/// Failure modes for [`Proxy::wait_for_task`]. Kept distinct from a plain
/// `String` so the caller can tell a permission problem (the write may have
/// succeeded; we just cannot confirm it) apart from an actual task failure
/// or a timeout.
enum WaitForTaskError {
    /// The caller's API key cannot poll `/tasks/{uid}` (Meilisearch returned
    /// `401` or `403`). The task itself may still be running or may have
    /// already succeeded — the proxy has no way to know.
    PermissionDenied(String),
    /// The task reached a terminal state of `failed` or `canceled`.
    TaskFailed(String),
    /// The task did not reach a terminal state before the timeout.
    TimedOut(String),
    /// Any other error while polling (network error, malformed response).
    Other(String),
}

impl std::fmt::Display for WaitForTaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            Self::PermissionDenied(msg) | Self::TaskFailed(msg) | Self::TimedOut(msg) | Self::Other(msg) => {
                write!(f, "{}", msg)
            }
        };
    }
}

/// Copies incoming request headers into a new map for the upstream request.
/// Skips the same hop-by-hop/framing headers as [`config::HEADERS_TO_SKIP`],
/// since the outgoing body may differ in length from what the client sent
/// (for example, when it was truncated at the size limit), so the upstream
/// HTTP client must compute the correct `content-length` itself.
fn sanitize_request_headers(headers: &axum::http::HeaderMap) -> reqwest::header::HeaderMap {
    let mut sanitized = reqwest::header::HeaderMap::new();
    for (key, value) in headers.iter() {
        if config::HEADERS_TO_SKIP.contains(&key.as_str()) {
            continue;
        }
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

/// Reads the full request body, up to [`config::MAX_REQUEST_BODY_SIZE`].
/// Returns `413 Payload Too Large` when the body exceeds the limit, or `400
/// Bad Request` if the body could not be read for any other reason (for
/// example, the client disconnected mid-upload). Earlier, any read error
/// here was silently replaced with an empty body, which forwarded a body
/// that no longer matched the request and could hang the upstream call.
async fn read_request_body(body: axum::body::Body) -> Result<bytes::Bytes, axum::response::Response> {
    return axum::body::to_bytes(body, *config::MAX_REQUEST_BODY_SIZE)
        .await
        .map_err(|e| {
            let is_too_large =
                std::error::Error::source(&e).is_some_and(|source| return source.is::<LengthLimitError>());
            if is_too_large {
                tracing::warn!(
                    limit = *config::MAX_REQUEST_BODY_SIZE,
                    "request body exceeded size limit"
                );
                return axum::response::Response::builder()
                    .status(413)
                    .body(axum::body::Body::from("request body too large"))
                    .unwrap();
            }
            tracing::error!(error = %e, "failed to read request body");
            return axum::response::Response::builder()
                .status(400)
                .body(axum::body::Body::from(format!("failed to read request body: {}", e)))
                .unwrap();
        });
}

impl Proxy {
    /// Creates a new proxy with an HTTP client configured with a timeout to
    /// stop a single upstream request from hanging forever.
    pub fn new() -> Self {
        return Self {
            client: reqwest::Client::builder()
                .timeout(*config::UPSTREAM_REQUEST_TIMEOUT)
                .build()
                .expect("failed to build upstream HTTP client"),
        };
    }

    /// Builds the axum router with all route handlers.
    pub fn router(self) -> axum::Router {
        return axum::Router::new()
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
    ) -> Result<bytes::Bytes, WaitForTaskError> {
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
                        .map_err(|e| return WaitForTaskError::Other(format!("failed to read task response: {}", e)))?;

                    // The caller's API key may not have the `tasks.get` action.
                    // The write itself may have already succeeded — the proxy
                    // just cannot check. Treat this differently from a real
                    // task failure so the caller isn't told the write failed.
                    if status_code == reqwest::StatusCode::UNAUTHORIZED || status_code == reqwest::StatusCode::FORBIDDEN
                    {
                        return Err(WaitForTaskError::PermissionDenied(format!(
                            "caller's API key cannot poll task {} status: {}",
                            task_uid, status_code
                        )));
                    }

                    if status_code.is_client_error() || status_code.is_server_error() {
                        return Err(WaitForTaskError::Other(format!(
                            "error fetching task {}: {}",
                            task_uid, status_code
                        )));
                    }

                    let task: TaskStatus = serde_json::from_slice(&body)
                        .map_err(|e| return WaitForTaskError::Other(format!("failed to parse task response: {}", e)))?;

                    tracing::debug!(task_uid = task_uid, status = %task.status, "task poll");
                    match task.status.as_str() {
                        "succeeded" => {
                            tracing::info!(task_uid = task_uid, "task succeeded");
                            return Ok(body);
                        }
                        "failed" | "canceled" => {
                            return Err(WaitForTaskError::TaskFailed(format!(
                                "task {} terminal state: {}",
                                task_uid, task.status
                            )));
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

        return Err(WaitForTaskError::TimedOut(format!(
            "timed out waiting for task {}",
            task_uid
        )));
    }

    /// Forwards a request to Meilisearch. If the response body has a
    /// `taskUid`, Meilisearch enqueued an async task. The proxy then polls
    /// the task until it completes and returns the final result. Otherwise
    /// the proxy returns the upstream response with no change. An OPTIONS
    /// request gets an empty 200 response for CORS preflight.
    async fn proxy_handler(
        axum::extract::State(proxy): axum::extract::State<Self>,
        request: axum::extract::Request,
    ) -> axum::response::Response {
        // Handle CORS preflight
        if request.method() == axum::http::Method::OPTIONS {
            return Self::options_handler().await;
        }

        let url = format!("{}{}", config::MEILISEARCH_HOST, request.uri());
        let method = request.method().clone();

        tracing::info!(method = %method, url = %url, "proxying request");

        let headers = sanitize_request_headers(request.headers());
        let body_bytes = match read_request_body(request.into_body()).await {
            Ok(bytes) => bytes,
            Err(rejection) => return rejection,
        };

        let upstream_response = match proxy
            .client
            .request(method, &url)
            .headers(headers.clone())
            .body(body_bytes)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    cause = ?e.source(),
                    "upstream request failed"
                );
                return axum::response::Response::builder()
                    .status(502)
                    .body(axum::body::Body::from(format!("proxy error: {}", e)))
                    .unwrap();
            }
        };

        let resp_status = upstream_response.status();
        let resp_headers = upstream_response.headers().clone();
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

        // A `taskUid` in the body means Meilisearch enqueued an async task.
        // This is true for a 202 response (most writes) and a 200 response
        // (for example, task cancellation or deletion). A search, a read,
        // or an error has no `taskUid`. The proxy passes these through with
        // no change, below.
        if let Ok(enqueued) = serde_json::from_slice::<EnqueuedTask>(&resp_body) {
            tracing::info!(task_uid = enqueued.task_uid, "waiting for task to complete");

            return match proxy.wait_for_task(enqueued.task_uid, &headers).await {
                Ok(task_body) => axum::response::Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(task_body))
                    .unwrap(),
                // The write itself was already accepted by Meilisearch (that is
                // how we got a `taskUid` at all). The caller's API key just
                // cannot poll `/tasks/{uid}` to confirm completion. Returning
                // 500 here would tell the caller the write failed, when it may
                // well have succeeded. Instead, pass through the original
                // enqueued-task response unchanged, so the caller sees exactly
                // what Meilisearch told us: the task was accepted.
                Err(WaitForTaskError::PermissionDenied(e)) => {
                    tracing::warn!(
                        task_uid = enqueued.task_uid,
                        error = %e,
                        "cannot confirm task completion due to insufficient permissions; returning the enqueued task response instead of a false error"
                    );
                    build_response(resp_status, &resp_headers, resp_body)
                }
                Err(e) => {
                    tracing::error!(task_uid = enqueued.task_uid, error = %e, "task polling failed");
                    axum::response::Response::builder()
                        .status(500)
                        .body(axum::body::Body::from(format!("task polling error: {}", e)))
                        .unwrap()
                }
            };
        }

        return build_response(resp_status, &resp_headers, resp_body);
    }
}
