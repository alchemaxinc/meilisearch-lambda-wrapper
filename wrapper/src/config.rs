//! Configuration for the Meilisearch Lambda proxy.
//!
//! All customizable values are exposed as environment variables with sensible defaults.
//! Constants that don't change between environments are plain `const` values.

/// Hop-by-hop headers to strip when copying headers between the client and
/// Meilisearch, in either direction. The proxy buffers the full request or
/// response body before it forwards the body, so any header that describes
/// the original framing is stale and must not pass through:
///
/// - `transfer-encoding`: the original side may chunk, but we always send a
///   single complete body
/// - `content-length`: the original value may not match the buffered body,
///   for example after the client's body is truncated at the size limit
/// - `connection`: hop-by-hop header per the HTTP spec, not meant for
///   end-to-end forwarding
pub const HEADERS_TO_SKIP: &[&str] = &["transfer-encoding", "content-length", "connection"];

/// Port the proxy listens on for incoming HTTP requests. This is the port that
/// AWS Lambda Web Adapter (LWA) forwards traffic to, and must match `AWS_LWA_PORT`.
pub const PROXY_LISTEN_PORT: u16 = 8080;

/// Internal Meilisearch address. Meilisearch runs as a child process within the
/// same container, listening on its default port 7700.
pub const MEILISEARCH_HOST: &str = "http://localhost:7700";

/// Maximum time to wait for an async Meilisearch task (e.g. document indexing) to
/// complete before returning an error. Derived from the Lambda's configured timeout
/// minus 1 second of headroom for cleanup.
///
/// Env: `AWS_LAMBDA_TIMEOUT_SECONDS` (default: 300)
pub static MAX_WAIT_TIME: std::sync::LazyLock<std::time::Duration> = std::sync::LazyLock::new(|| {
    let timeout: u64 = std::env::var("AWS_LAMBDA_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| return "300".to_string())
        .parse()
        .expect("AWS_LAMBDA_TIMEOUT_SECONDS must be a number");
    return std::time::Duration::from_secs(timeout - 1);
});

/// How often to poll Meilisearch's `/tasks` endpoint when waiting for an async
/// operation to complete. Lower values give faster responses but more CPU usage.
///
/// Env: `MEILISEARCH_POLL_INTERVAL_MS` (default: 100)
pub static POLL_INTERVAL: std::sync::LazyLock<std::time::Duration> = std::sync::LazyLock::new(|| {
    let ms: u64 = std::env::var("MEILISEARCH_POLL_INTERVAL_MS")
        .unwrap_or_else(|_| return "100".to_string())
        .parse()
        .expect("MEILISEARCH_POLL_INTERVAL_MS must be a number");
    return std::time::Duration::from_millis(ms);
});

/// Maximum allowed size of an incoming request body. Prevents a large payload from
/// exhausting Lambda memory. Should be set to a fraction of the Lambda's configured
/// memory, leaving room for the proxy, Meilisearch, and the response buffer.
///
/// Env: `MAX_REQUEST_BODY_SIZE_MB` (default: 100)
pub static MAX_REQUEST_BODY_SIZE: std::sync::LazyLock<usize> = std::sync::LazyLock::new(|| {
    let mb: usize = std::env::var("MAX_REQUEST_BODY_SIZE_MB")
        .unwrap_or_else(|_| return "100".to_string())
        .parse()
        .expect("MAX_REQUEST_BODY_SIZE_MB must be a number");
    return mb * 1024 * 1024;
});

/// How often the supervisor thread checks whether the Meilisearch child
/// process is still alive. If the process has exited, the wrapper exits
/// too, so the container orchestrator (Lambda/ECS) can restart it instead
/// of serving errors forever from a dead Meilisearch instance.
///
/// Env: `MEILISEARCH_SUPERVISION_INTERVAL_MS` (default: 1000)
pub static SUPERVISION_INTERVAL: std::sync::LazyLock<std::time::Duration> = std::sync::LazyLock::new(|| {
    let ms: u64 = std::env::var("MEILISEARCH_SUPERVISION_INTERVAL_MS")
        .unwrap_or_else(|_| return "1000".to_string())
        .parse()
        .expect("MEILISEARCH_SUPERVISION_INTERVAL_MS must be a number");
    return std::time::Duration::from_millis(ms);
});

/// Maximum time to wait for a single upstream request to Meilisearch (the
/// initial forwarded request, or one task-status poll) to complete. This is
/// separate from [`MAX_WAIT_TIME`], which bounds the whole task-polling
/// loop; this bounds each individual HTTP call within it, so a stalled
/// connection cannot hang the proxy forever.
///
/// Capped at [`MAX_WAIT_TIME`]: a per-request timeout longer than the whole
/// polling budget can never fire before the Lambda invocation itself times
/// out, silently defeating the point of this setting.
///
/// Env: `UPSTREAM_REQUEST_TIMEOUT_SECONDS` (default: 30, capped at `MAX_WAIT_TIME`)
pub static UPSTREAM_REQUEST_TIMEOUT: std::sync::LazyLock<std::time::Duration> = std::sync::LazyLock::new(|| {
    let secs: u64 = std::env::var("UPSTREAM_REQUEST_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| return "30".to_string())
        .parse()
        .expect("UPSTREAM_REQUEST_TIMEOUT_SECONDS must be a number");
    return std::cmp::min(std::time::Duration::from_secs(secs), *MAX_WAIT_TIME);
});
