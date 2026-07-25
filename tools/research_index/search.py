#!/usr/bin/env python3
"""Search the local VERA20k research index."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from research_index.database import DEFAULT_DB, search
from research_index.formatting import format_search_results


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Search the VERA20k research index.")
    parser.add_argument("query", help="Search query")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="SQLite database path")
    parser.add_argument("--limit", type=int, default=20, help="Maximum results")
    parser.add_argument("--system", help="Filter by inferred system")
    parser.add_argument("--source", help="Filter by source kind")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    rows = search(Path(args.db), args.query, limit=args.limit, system=args.system, source_kind=args.source)

    if args.json:
        print(json.dumps(rows, indent=2))
    else:
        print(format_search_results(rows))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
