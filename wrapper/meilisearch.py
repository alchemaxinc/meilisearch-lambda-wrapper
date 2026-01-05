import gzip
import json
import subprocess
import threading
import time

import requests

import config
from logging_config import get_logger

logger = get_logger(__name__)


class MeiliSearchWrapper:
    def __init__(self):
        self.host = config.MEILISEARCH_HOST
        self.process = None

    def _log_process_output(self, stream, stream_type):
        """Read and log output from the process stream."""
        try:
            for line in iter(stream.readline, ""):
                if not line:
                    continue

                line = line.rstrip("\n")
                if not line:
                    continue

                try:
                    # Parse the JSON to validate it, but print the original line
                    line_json = json.loads(line)
                    fields = {}
                    if "fields" in line_json:
                        fields = line_json.pop("fields")

                    msg = ""
                    if "message" in fields:
                        msg = fields["message"]

                    msg = msg.replace("\t", " ")

                    # Also clean tabs from the message in line_json if it exists
                    if "message" in line_json:
                        line_json["message"] = line_json["message"].replace("\t", " ")

                    logger.info(
                        msg,
                        extra={
                            "from_meili": True,
                            "meilisearch-stream-type": stream_type,
                            **line_json,
                        },
                    )
                except json.JSONDecodeError as e:
                    logger.info(
                        line,
                        extra={
                            "from_meili": True,
                            "meilisearch-stream-type": stream_type,
                        },
                    )
                continue

        except Exception as e:
            logger.error(f"Error reading {stream_type}: {str(e)}")
        finally:
            stream.close()

    def start(self):
        """Start the MeiliSearch binary in the background."""
        try:
            logger.info(f"Starting MeiliSearch on port {config.MEILISEARCH_PORT}...")

            self.process = subprocess.Popen(
                ["meilisearch"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            logger.info(f"MeiliSearch process started with PID {self.process.pid}")

            # Start threads to log stdout and stderr
            stdout_thread = threading.Thread(
                target=self._log_process_output,
                args=(self.process.stdout, "STDOUT"),
                daemon=True,
            )
            stderr_thread = threading.Thread(
                target=self._log_process_output,
                args=(self.process.stderr, "STDERR"),
                daemon=True,
            )
            stdout_thread.start()
            stderr_thread.start()

        except FileNotFoundError:
            logger.error("MeiliSearch binary not found. Is it installed and in PATH?")
            raise
        except Exception as e:
            logger.error(f"Failed to start MeiliSearch: {type(e).__name__}: {str(e)}")
            raise

    @classmethod
    def wait_for_task(cls, task_uid, *, headers):
        base = config.MEILISEARCH_HOST
        deadline = time.time() + config.MAX_WAIT_TIME

        logger.debug(
            "Polling task status",
            extra={"taskUid": task_uid, "host": base, "timeout": config.MAX_WAIT_TIME},
        )

        while time.time() < deadline:
            try:
                resp = requests.get(
                    f"{base}/tasks/{task_uid}",
                    headers=headers,
                    timeout=10,
                )
                if resp.status_code >= 400:
                    raise RuntimeError(
                        f"Error fetching task {task_uid}: {resp.status_code}"
                    )

                # Handle potential gzip encoding in response
                content_encoding = (
                    resp.headers.get("Content-Encoding", "") or ""
                ).lower()
                if "gzip" in content_encoding:
                    body_bytes = gzip.decompress(resp.content)
                else:
                    body_bytes = resp.content

                data = resp.json()
                status = data.get("status")
                logger.debug("Task poll", extra={"taskUid": task_uid, "status": status})

                if status == "succeeded":
                    logger.info("Task succeeded", extra={"taskUid": task_uid})
                    return resp, body_bytes

                if status in ("failed", "canceled"):
                    raise RuntimeError(f"Task {task_uid} terminal state: {status}")

            except requests.exceptions.RequestException as e:
                logger.debug(
                    "Task poll request failed",
                    extra={"taskUid": task_uid, "error": str(e)},
                )

            time.sleep(config.POLL_INTERVAL)

        raise TimeoutError(f"Timed out waiting for task {task_uid}")
