//! Manages the Meilisearch child process lifecycle.
//!
//! Starts the `meilisearch` binary as a subprocess, parses its JSON log
//! output, and forwards each line at the correct log level.

/// Handle to the running Meilisearch child process. Killing the process is
/// handled automatically via the [`Drop`] implementation.
pub struct Meilisearch {
    process: std::process::Child,
}

const MEILISEARCH_BINARY_NAME: &str = "meilisearch";

#[derive(serde::Deserialize)]
struct MeilisearchLog {
    level: String,
    fields: MeilisearchLogFields,
    #[serde(default)]
    target: String,
}

#[derive(serde::Deserialize)]
struct MeilisearchLogFields {
    message: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

fn parse_log_level(level: &str) -> tracing::Level {
    return match level {
        "TRACE" => tracing::Level::TRACE,
        "DEBUG" => tracing::Level::DEBUG,
        "WARN" => tracing::Level::WARN,
        "ERROR" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
}

// Relies on MEILI_EXPERIMENTAL_LOGS_MODE=json, as regular tty-colored logs do not work well for
// forwarding. This is a requirement for this wrapper to run optimally and ensure proper
// debugging is possible.
fn forward_line(stream: &'static str, line: &str) {
    match serde_json::from_str::<MeilisearchLog>(line) {
        Ok(log) => {
            let level = parse_log_level(&log.level);
            let details = serde_json::to_string(&log.fields.extra).unwrap_or_default();
            match level {
                tracing::Level::TRACE => {
                    tracing::trace!(stream, meilisearch_target = %log.target, details = %details, "{}", log.fields.message)
                }
                tracing::Level::DEBUG => {
                    tracing::debug!(stream, meilisearch_target = %log.target, details = %details, "{}", log.fields.message)
                }
                tracing::Level::INFO => {
                    tracing::info!(stream, meilisearch_target = %log.target, details = %details, "{}", log.fields.message)
                }
                tracing::Level::WARN => {
                    tracing::warn!(stream, meilisearch_target = %log.target, details = %details, "{}", log.fields.message)
                }
                tracing::Level::ERROR => {
                    tracing::error!(stream, meilisearch_target = %log.target, details = %details, "{}", log.fields.message)
                }
            }
        }
        Err(_) => {
            panic!(
                "failed to parse meilisearch log line as JSON — \
                 is MEILI_EXPERIMENTAL_LOGS_MODE=json set? \
                 raw line: {line}"
            );
        }
    }
}

impl Meilisearch {
    /// Spawns the Meilisearch binary and wires up log forwarding for stdout/stderr.
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!(host = crate::config::MEILISEARCH_HOST, "starting meilisearch");

        let mut child = std::process::Command::new(MEILISEARCH_BINARY_NAME)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        tracing::info!(pid = child.id(), "meilisearch process started");
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in std::io::BufRead::lines(reader) {
                match line {
                    Ok(line) => forward_line("stdout", &line),
                    Err(e) => {
                        tracing::error!(error = %e, "error reading meilisearch stdout");
                        break;
                    }
                }
            }
        });

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in std::io::BufRead::lines(reader) {
                match line {
                    Ok(line) => forward_line("stderr", &line),
                    Err(e) => {
                        tracing::error!(error = %e, "error reading meilisearch stderr");
                        break;
                    }
                }
            }
        });

        return Ok(Self { process: child });
    }

    /// Returns the OS process ID of the running Meilisearch instance.
    pub fn pid(&self) -> u32 {
        return self.process.id();
    }
}

impl Drop for Meilisearch {
    fn drop(&mut self) {
        tracing::info!(pid = self.pid(), "stopping meilisearch");
        if let Err(e) = self.process.kill() {
            tracing::error!(error = %e, "failed to kill meilisearch process");
        }
    }
}
