"""Configuration and constants for the Meilisearch wrapper"""

import os
import logging

# Server configuration
MEILISEARCH_PORT = 7700
PROXY_LISTEN_PORT = 8080
MEILISEARCH_HOST = f"http://localhost:{MEILISEARCH_PORT}"

# Timeouts and polling
MAX_WAIT_TIME = int(os.environ.get("AWS_LAMBDA_TIMEOUT_SECONDS", "300")) - 1
POLL_INTERVAL = float(os.environ.get("MEILISEARCH_POLL_INTERVAL_MS", "100")) / 1000.0

# Logging
LOG_LEVEL = os.environ.get("LOG_LEVEL", "INFO").upper()
LOG_LEVELS = {
    "DEBUG": logging.DEBUG,
    "INFO": logging.INFO,
    "WARN": logging.WARN,
    "WARNING": logging.WARNING,
    "ERROR": logging.ERROR,
}

# Preserve content-encoding so clients can decode payloads properly.
HEADERS_TO_SKIP = ("transfer-encoding", "content-length")

# Meilisearch task states
TASK_TERMINAL_STATES = ("succeeded", "failed", "canceled")
