#!/usr/bin/env python3
"""Map research docs for a system or topic."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from research_index.database import DEFAULT_DB
from research_index.formatting import format_system_map
from research_index.system_map import system_map


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="List and group VERA20k research docs for a system/topic.")
    parser.add_argument("topic", nargs="?", help="Optional topic phrase to match in paths, titles, headings, or chunks")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="SQLite database path")
    parser.add_argument("--system", help="Filter by inferred system")
    parser.add_argument("--source", help="Filter by source kind")
    parser.add_argument("--status", help="Filter by status")
    parser.add_argument("--limit", type=int, default=80, help="Maximum rows per section")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    result = system_map(
        Path(args.db),
        system=args.system,
        topic=args.topic,
        source_kind=args.source,
        status=args.status,
        limit=args.limit,
    )
    print(json.dumps(result, indent=2) if args.json else format_system_map(result))
    return 0 if result["matched"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
