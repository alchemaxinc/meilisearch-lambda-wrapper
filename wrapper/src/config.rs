// Server configuration
pub const PROXY_LISTEN_PORT: u16 = 8080;
pub const MEILISEARCH_HOST: &str = "http://localhost:7700";

// Timeouts and polling
pub static MAX_WAIT_TIME: std::sync::LazyLock<std::time::Duration> =
    std::sync::LazyLock::new(|| {
        let timeout: u64 = std::env::var("AWS_LAMBDA_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| return "300".to_string())
            .parse()
            .expect("AWS_LAMBDA_TIMEOUT_SECONDS must be a number");
        return std::time::Duration::from_secs(timeout - 1);
    });

pub static POLL_INTERVAL: std::sync::LazyLock<std::time::Duration> =
    std::sync::LazyLock::new(|| {
        let ms: u64 = std::env::var("MEILISEARCH_POLL_INTERVAL_MS")
            .unwrap_or_else(|_| return "100".to_string())
            .parse()
            .expect("MEILISEARCH_POLL_INTERVAL_MS must be a number");
        return std::time::Duration::from_millis(ms);
    });
