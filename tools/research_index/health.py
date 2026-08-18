#!/usr/bin/env python3
"""Inspect or refresh the research-index generation."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from research_index.database import DEFAULT_DB
from research_index.formatting import format_index_health
from research_index.lifecycle import (
    IndexLifecycleError,
    ensure_fresh,
    inspect_index,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Inspect or refresh VERA20k research-index freshness."
    )
    parser.add_argument("--db", default=str(DEFAULT_DB), help="SQLite database path")
    parser.add_argument("--workspace", default=".", help="Workspace root")
    parser.add_argument(
        "--root",
        action="append",
        dest="roots",
        help="Explicit index root; repeatable and persisted after refresh",
    )
    parser.add_argument(
        "--refresh",
        action="store_true",
        help="Synchronously rebuild when the generation is stale",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=40,
        help="Maximum changed-file rows per category",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    db_path = Path(args.db)
    workspace = Path(args.workspace).resolve()
    try:
        if args.refresh:
            result = ensure_fresh(
                db_path,
                workspace,
                roots=args.roots,
                limit=args.limit,
            )
        else:
            result = inspect_index(
                db_path,
                workspace,
                roots=args.roots,
                limit=args.limit,
            )
    except IndexLifecycleError as exc:
        print(f"research-index freshness failed: {exc}", file=sys.stderr)
        return 1

    print(
        json.dumps(result, indent=2)
        if args.json
        else format_index_health(result)
    )
    return 0 if result["fresh"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
