import argparse
from typing import Callable, Iterable, List, Optional

from get_latest_meilisearch_version import handle_get_latest_meilisearch_version

HANDLERS: dict[str, Callable[[], int]] = {
    "get-latest-meilisearch-version": handle_get_latest_meilisearch_version,
}


def print_lines(lines: Iterable[str]) -> None:
    for line in lines:
        print(line)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Sync versions helpers")
    parser.add_argument("verb", choices=HANDLERS.keys())
    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return HANDLERS[args.verb]()


if __name__ == "__main__":
    raise SystemExit(main())
