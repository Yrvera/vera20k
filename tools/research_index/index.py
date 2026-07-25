#!/usr/bin/env python3
"""Build the local VERA20k research index (CLI shim over indexing.rebuild_index)."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from research_index.database import DEFAULT_DB
from research_index.indexing import DEFAULT_ROOTS
from research_index.lifecycle import IndexLifecycleError, refresh_index


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the VERA20k research SQLite FTS index.")
    parser.add_argument("roots", nargs="*", default=list(DEFAULT_ROOTS), help="Roots to index")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="Output SQLite database path")
    parser.add_argument("--workspace", default=".", help="Workspace root")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    workspace = Path(args.workspace).resolve()
    roots = [workspace / root for root in args.roots]
    db_path = Path(args.db)
    try:
        result = refresh_index(db_path, workspace, roots)
    except IndexLifecycleError as exc:
        print(f"research-index rebuild failed: {exc}", file=sys.stderr)
        return 1
    print(result["summary"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
