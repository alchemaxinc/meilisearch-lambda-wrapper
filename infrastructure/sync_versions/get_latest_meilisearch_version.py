from typing import Optional

import requests

TAGS_URL = "https://registry.hub.docker.com/v2/repositories/getmeili/meilisearch/tags"
DEFAULT_PARAMS = {
    "ordering": "last_updated",
    "page_size": 100,
}
TIMEOUT_SECONDS = 10


def fetch_versioned_tag_names(
    *,
    session: Optional[requests.Session] = None,
    url: str = TAGS_URL,
    params: Optional[dict] = None,
) -> str:
    """Return the latest versioned tag name from Docker Hub for the Meilisearch repository."""
    active_params = params or DEFAULT_PARAMS
    client = session or requests

    response = client.get(url, params=active_params, timeout=TIMEOUT_SECONDS)
    response.raise_for_status()

    payload = response.json()
    results = payload.get("results", [])

    names = [
        item["name"]
        for item in results
        if "name" in item
        if item["name"].startswith("v")
    ]

    return names[0]


def handle_get_latest_meilisearch_version() -> int:
    newest_tag = fetch_versioned_tag_names()
    print(newest_tag)
    return 0
