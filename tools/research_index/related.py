#!/usr/bin/env python3
"""Find research docs related by extracted evidence terms."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from research_index.database import DEFAULT_DB, related_by_document, related_by_term
from research_index.formatting import format_related_results


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Find related VERA20k research docs.")
    parser.add_argument("target", help="Document path, or term when --term is set")
    parser.add_argument("--term", action="store_true", help="Treat target as an exact extracted term")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="SQLite database path")
    parser.add_argument("--limit", type=int, default=20, help="Maximum results")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    db_path = Path(args.db)
    rows = related_by_term(db_path, args.target, args.limit) if args.term else related_by_document(db_path, args.target, args.limit)

    if args.json:
        print(json.dumps(rows, indent=2))
    else:
        print(format_related_results(rows))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
