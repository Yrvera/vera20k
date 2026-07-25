#!/usr/bin/env python3
"""Build the local VERA20k research index (CLI shim over indexing.rebuild_index)."""

from __future__ import annotations

import argparse
from pathlib import Path

from research_index.database import DEFAULT_DB
from research_index.indexing import DEFAULT_ROOTS, rebuild_index


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the VERA20k research SQLite FTS index.")
    parser.add_argument("roots", nargs="*", default=list(DEFAULT_ROOTS), help="Roots to index")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="Output SQLite database path")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    workspace = Path.cwd()
    roots = [workspace / root for root in args.roots]
    db_path = Path(args.db)
    print(rebuild_index(workspace, roots, db_path))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
