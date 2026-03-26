pub struct Meilisearch {
    process: std::process::Child,
}

const MEILISEARCH_BINARY_NAME: &str = "meilisearch";

impl Meilisearch {
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!(
            host = crate::config::MEILISEARCH_HOST,
            "starting meilisearch"
        );

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
                    Ok(line) => tracing::info!(stream = "stdout", "{}", line),
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
                    Ok(line) => tracing::warn!(stream = "stderr", "{}", line),
                    Err(e) => {
                        tracing::error!(error = %e, "error reading meilisearch stderr");
                        break;
                    }
                }
            }
        });

        return Ok(Self { process: child });
    }

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
