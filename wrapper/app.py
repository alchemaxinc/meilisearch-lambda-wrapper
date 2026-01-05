from http.server import HTTPServer

from . import config
from .logging_config import get_logger
from .meilisearch import MeiliSearchWrapper
from .proxy import InterceptingProxy

logger = get_logger(__name__)


def main():
    logger.info(f"Starting MeiliSearch wrapper proxy")

    meilisearch = MeiliSearchWrapper()
    meilisearch.start()

    logger.info("MeiliSearch is ready!")

    # Start the proxy server
    server_address = ("", config.PROXY_LISTEN_PORT)
    httpd = HTTPServer(server_address, InterceptingProxy)
    logger.info(
        f"Proxy running on port {config.PROXY_LISTEN_PORT} forwarding to {config.MEILISEARCH_HOST}"
    )

    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        logger.info("Shutting down proxy server...")
        httpd.shutdown()


if __name__ == "__main__":
    main()
