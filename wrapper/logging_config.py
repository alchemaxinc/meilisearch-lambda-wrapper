"""Structured JSON logging configuration"""

import json
import logging
import sys

from .config import LOG_LEVEL, LOG_LEVELS


class JSONFormatter(logging.Formatter):
    """Custom formatter that outputs JSON structured logs"""

    # Known standard LogRecord attributes we should not treat as extras
    _standard_attrs = {
        "name",
        "msg",
        "args",
        "levelname",
        "levelno",
        "pathname",
        "filename",
        "module",
        "exc_info",
        "taskName",
        "exc_text",
        "stack_info",
        "lineno",
        "funcName",
        "created",
        "msecs",
        "relativeCreated",
        "thread",
        "threadName",
        "processName",
        "process",
        "message",
    }

    def format(self, record):
        log_entry = {
            "level": record.levelname,
            "message": record.getMessage(),
            "timestamp": self.formatTime(record, "%Y-%m-%dT%H:%M:%SZ"),
        }

        # Collect custom fields added via `extra={...}`
        for key, value in record.__dict__.items():
            if key in self._standard_attrs:
                continue

            if key.startswith("_"):
                continue

            log_entry[key] = value

        return json.dumps(log_entry)


def get_logger(name: str) -> logging.Logger:
    """Create and configure a logger with JSON formatting"""
    logger = logging.getLogger(name)
    logger.setLevel(LOG_LEVELS.get(LOG_LEVEL, logging.INFO))

    # Only add handler if not already configured
    if not logger.handlers:
        handler = logging.StreamHandler(sys.stderr)
        handler.setFormatter(JSONFormatter())
        logger.addHandler(handler)

    return logger
