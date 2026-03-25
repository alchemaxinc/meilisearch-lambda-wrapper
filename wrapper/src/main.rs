mod config;
mod http_server;

use tracing::{debug, error, info};

fn main() {
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

    info!(
        port = config::PROXY_LISTEN_PORT,
        "starting MeiliSearch wrapper proxy"
    );
}
