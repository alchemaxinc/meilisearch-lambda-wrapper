use crate::config;

#[derive(Clone)]
pub struct Proxy {
    client: reqwest::Client,
}

#[derive(serde::Deserialize)]
struct EnqueuedTask {
    #[serde(rename = "taskUid")]
    task_uid: u64,
}

#[derive(serde::Deserialize)]
struct TaskStatus {
    status: String,
}

impl Proxy {
    pub fn new() -> Self {
        return Self {
            client: reqwest::Client::new(),
        };
    }

    pub fn router(self) -> axum::Router {
        return axum::Router::new()
            // Special synchronous handling: any POST to /indexes/
            .route(
                "/indexes/{*rest}",
                axum::routing::post(Self::index_post_handler).fallback(Self::proxy_handler),
            )
            // Anything else that can be proxied as normal
            .fallback(axum::routing::any(Self::proxy_handler))
            .with_state(self);
    }

    async fn wait_for_task(
        &self,
        task_uid: u64,
        headers: &reqwest::header::HeaderMap,
    ) -> Result<bytes::Bytes, String> {
        let url = format!("{}/tasks/{}", config::MEILISEARCH_HOST, task_uid);
        let deadline = std::time::Instant::now() + *config::MAX_WAIT_TIME;
        let poll_interval = *config::POLL_INTERVAL;

        tracing::debug!(
            task_uid = task_uid,
            timeout = ?config::MAX_WAIT_TIME,
            "polling task status"
        );

        while std::time::Instant::now() < deadline {
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

    async fn index_post_handler(
        axum::extract::State(proxy): axum::extract::State<Self>,
        request: axum::extract::Request,
    ) -> axum::response::Response {
        let url = format!("{}{}", config::MEILISEARCH_HOST, request.uri());
        let headers = request.headers().clone();

        tracing::info!(url = %url, "intercepted index POST");

        // TODO: Set the limit to something around the Lambda's memory limit
        let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
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

    async fn proxy_handler(
        axum::extract::State(proxy): axum::extract::State<Self>,
        request: axum::extract::Request,
    ) -> axum::response::Response {
        let url = format!("{}{}", config::MEILISEARCH_HOST, request.uri());
        let method = request.method().clone();

        tracing::info!(method = %method, url = %url, "proxying request");

        let headers = request.headers().clone();
        // TODO: Set the limit to something around the Lambda's memory limit
        let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
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

                let mut response = axum::response::Response::builder().status(status);
                for (key, value) in headers.iter() {
                    response = response.header(key, value);
                }
                return response.body(axum::body::Body::from(resp_body)).unwrap();
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
