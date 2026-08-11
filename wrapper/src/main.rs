//! Meilisearch Lambda Wrapper — an HTTP proxy that wraps Meilisearch's async API
//! to run on AWS Lambda.
//! Starts Meilisearch as a child process and proxies all
//! requests, converting async write operations into synchronous ones by polling
//! until completion.

mod config;
mod meilisearch;
mod proxy;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        // Respect RUST_LOG if set; default to "info" only when it isn't.
        // Using `.add_directive()` on top of `from_default_env()` would
        // add a second global directive alongside whatever RUST_LOG set,
        // which can silently widen or narrow the effective level instead
        // of acting as a pure fallback.
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| return tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        // Promote fields to top level, instead of nesting inside 'fields'
        .flatten_event(true)
        .init();

    let meilisearch = match meilisearch::Meilisearch::start() {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, "failed to start meilisearch");
            std::process::exit(1);
        }
    };
    tracing::info!(pid = meilisearch.pid(), "meilisearch daemon is running");

    let addr = format!("0.0.0.0:{}", config::PROXY_LISTEN_PORT);
    let app = proxy::Proxy::new().router();

    tracing::info!(port = config::PROXY_LISTEN_PORT, "starting meilisearch wrapper proxy");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
