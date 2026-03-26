use crate::config;

#[derive(Clone)]
pub struct Proxy {
    client: reqwest::blocking::Client,
}

impl Proxy {
    pub fn new() -> Self {
        return Self {
            client: reqwest::blocking::Client::new(),
        };
    }

    pub fn router(self) -> axum::Router {
        return axum::Router::new()
            .fallback(axum::routing::any(Self::handler))
            .with_state(self);
    }

    async fn handler(
        axum::extract::State(proxy): axum::extract::State<Self>,
        request: axum::extract::Request,
    ) -> axum::response::Response {
        let path = request.uri().path().to_string();
        let query = request
            .uri()
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();
        let method = request.method().clone();
        let url = format!("{}{}{}", config::MEILISEARCH_HOST, path, query);

        tracing::info!(method = %method, url = %url, "proxying request");
        let headers = request.headers().clone();
        // TODO: Set the usize::MAX limit to something around the Lambda's max memory, possibly a tad lower
        let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
            .await
            .unwrap_or_default();

        let upstream_response = proxy
            .client
            .request(method, &url)
            .headers(headers)
            .body(body_bytes)
            .send();

        match upstream_response {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let resp_body = resp.bytes().unwrap_or_default();

                let mut response = axum::response::Response::builder().status(status);
                for (key, value) in headers.iter() {
                    response = response.header(key, value);
                }
                return response.body(axum::body::Body::from(resp_body)).unwrap();
            }
            Err(e) => {
                tracing::error!(error = %e, "upstream request failed");
                return axum::response::Response::builder()
                    .status(500)
                    .body(axum::body::Body::from(format!("proxy error: {}", e)))
                    .unwrap();
            }
        }
    }
}
