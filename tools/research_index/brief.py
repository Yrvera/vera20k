#!/usr/bin/env python3
"""Build a compact pre-implementation research brief."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from research_index.brief import research_brief
from research_index.database import DEFAULT_DB
from research_index.formatting import format_research_brief


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build a VERA20k pre-implementation research brief.")
    parser.add_argument("query", help="System topic, mechanism, function, or implementation question")
    parser.add_argument("--anchor", action="append", default=[], help="Exact symbol or address to include; repeatable")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="SQLite database path")
    parser.add_argument("--workspace", default=".", help="Workspace root")
    parser.add_argument("--system", help="Filter by inferred system")
    parser.add_argument("--source", help="Filter by source kind")
    parser.add_argument("--limit", type=int, default=8, help="Maximum rows per section")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    result = research_brief(
        Path(args.db),
        Path(args.workspace).resolve(),
        args.query,
        system=args.system,
        source_kind=args.source,
        anchors=args.anchor,
        limit=args.limit,
    )
    print(json.dumps(result, indent=2) if args.json else format_research_brief(result))
    return 0 if result["validation"]["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
