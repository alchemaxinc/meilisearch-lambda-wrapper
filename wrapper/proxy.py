import gzip
import json
from http.server import BaseHTTPRequestHandler, HTTPServer

import requests

from . import config
from .logging_config import get_logger
from .meilisearch import MeiliSearchWrapper

logger = get_logger(__name__)


class InterceptingProxy(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        """Override the default HTTP server logging to use our JSON logger."""
        logger.info(
            format % args,
            extra={
                "client_address": self.client_address[0],
                "client_port": self.client_address[1],
                "method": self.command,
                "path": self.path,
                "http_version": self.request_version,
            },
        )

    def log_request(self, code="?", size="-"):
        """Override to log HTTP requests through our logging framework."""
        logger.info(
            f"{self.command} {self.path}",
            extra={
                "client_address": self.client_address[0],
                "client_port": self.client_address[1],
                "method": self.command,
                "path": self.path,
                "http_version": self.request_version,
                "status_code": code,
                "response_size": size,
            },
        )

    def log_error(self, format, *args):
        """Override error logging to use our JSON logger."""
        logger.error(
            format % args,
            extra={
                "client_address": self.client_address[0],
                "client_port": self.client_address[1],
            },
        )

    def do_request(self, method):
        content_length = int(self.headers.get("Content-Length", 0))
        post_data = self.rfile.read(content_length) if content_length > 0 else None

        # Special synchronous handling: any POST to /indexes/
        if method == "POST" and self.path.startswith("/indexes/"):
            self.handle_index_post(method, post_data)
            return

        # Anything else that can be proxied as normal
        self.forward_request(method, post_data)

    def _strip_host_and_normalize_accept_encoding(self):
        """Copy incoming headers, strip Host, and normalize Accept-Encoding to avoid compressed payloads to proxy.
        We prefer identity from upstream for easier JSON handling.
        """
        headers = {k: v for k, v in self.headers.items() if k.lower() != "host"}
        headers["Accept-Encoding"] = "identity"
        return headers

    def _forward_request_and_get_body(self, method, body):
        url = f"{config.MEILISEARCH_HOST}{self.path}"
        headers = self._strip_host_and_normalize_accept_encoding()
        logger.debug(
            "headers being sent to meilisearch",
            extra={"headers": headers, "method": method, "path": self.path},
        )
        resp = requests.request(
            method=method,
            url=url,
            headers=headers,
            data=body,
            allow_redirects=False,
        )
        content_encoding = (resp.headers.get("Content-Encoding", "") or "").lower()
        if "gzip" in content_encoding:
            body_bytes = gzip.decompress(resp.content)
        else:
            body_bytes = resp.content
        return resp, body_bytes

    def _send_response_to_client(self, resp, body_bytes):
        self.send_response(resp.status_code)

        # Forward headers from upstream, skipping certain ones
        for k, v in resp.headers.items():
            if k.lower() in config.HEADERS_TO_SKIP:
                continue

            self.send_header(k, v)

        # Set correct content length
        self.send_header("Content-Length", str(len(body_bytes)))
        self.end_headers()

        logger.debug(
            "headers being sent to client",
            extra={
                "headers": {
                    k: v
                    for k, v in resp.headers.items()
                    if k.lower() not in config.HEADERS_TO_SKIP
                },
                "method": self.command,
                "path": self.path,
            },
        )
        self.wfile.write(body_bytes)

    def _send_error_to_client(self, status_code, message):
        """Send error response with CORS headers as JSON."""
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json")
        error_response = json.dumps({"error": message})
        body = error_response.encode("utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def forward_request(self, method, body):
        """Forward request to the real Lambda wrapper."""
        try:
            resp, body_bytes = self._forward_request_and_get_body(method, body)

            # Send the upstream response back to the client, but ensure no compression headers
            self._send_response_to_client(resp, body_bytes)

        except Exception as e:
            self._send_error_to_client(500, f"Proxy Error: {str(e)}")

    def handle_index_post(self, method, post_data):
        try:
            resp, body_bytes = self._forward_request_and_get_body(method, post_data)
            logger.debug(
                "Intercepted index creation POST",
                extra={"response_code": resp.status_code},
            )
            response_json = json.loads(body_bytes.decode("utf-8"))
            if "taskUid" not in response_json:
                logger.error("taskUid not found in response")
                self._send_response_to_client(resp, body_bytes)
                return

            task_uid = response_json["taskUid"]

            # Pass the headers from the original client request
            headers = self._strip_host_and_normalize_accept_encoding()
            resp, body_bytes = MeiliSearchWrapper.wait_for_task(
                task_uid, headers=headers
            )
            self._send_response_to_client(resp, body_bytes)

        except Exception as e:
            self._send_error_to_client(500, f"Proxy Error: {str(e)}")

    # Route all standard HTTP verbs to our handler
    def do_GET(self):
        self.do_request("GET")

    def do_POST(self):
        self.do_request("POST")

    def do_PUT(self):
        self.do_request("PUT")

    def do_DELETE(self):
        self.do_request("DELETE")

    def do_OPTIONS(self):
        """Handle CORS preflight requests."""
        self.send_response(200)
        self.send_header("Content-Length", "0")
        self.end_headers()


if __name__ == "__main__":
    server_address = ("", config.PROXY_LISTEN_PORT)
    httpd = HTTPServer(server_address, InterceptingProxy)
    logger.info(
        "Proxy running",
        extra={
            "port": config.PROXY_LISTEN_PORT,
            "forwarding to": config.MEILISEARCH_HOST,
        },
    )
    httpd.serve_forever()
