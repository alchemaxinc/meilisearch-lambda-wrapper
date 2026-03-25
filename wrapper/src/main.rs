mod config;
mod proxy;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        // Get log level from RUST_LOG or default to "INFO"
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .json()
        // Promote fields to top level, instead of nesting inside 'fields'
        .flatten_event(true)
        .init();

    let addr = format!("0.0.0.0:{}", config::PROXY_LISTEN_PORT);
    let app = proxy::Proxy::new().router();

    tracing::info!(
        port = config::PROXY_LISTEN_PORT,
        "starting MeiliSearch wrapper proxy"
    );

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
