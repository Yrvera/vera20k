#!/usr/bin/env python3
"""Build an implementation-oriented parity handoff from the research index."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from research_index.database import DEFAULT_DB
from research_index.formatting import format_parity_handoff
from research_index.handoff import parity_handoff


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build a VERA20k parity implementation handoff.")
    parser.add_argument("query", help="Mechanism, symbol, doc topic, or implementation question")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="SQLite database path")
    parser.add_argument("--workspace", default=".", help="Workspace root for Rust-path freshness checks")
    parser.add_argument("--limit", type=int, default=8, help="Maximum rows per section")
    parser.add_argument("--system", help="Filter evidence and handoff sections by inferred system")
    parser.add_argument("--source", help="Filter evidence and handoff sections by source kind")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    result = parity_handoff(
        Path(args.db),
        args.query,
        args.limit,
        system=args.system,
        source_kind=args.source,
        workspace=Path(args.workspace).resolve(),
    )
    print(json.dumps(result, indent=2) if args.json else format_parity_handoff(result))
    return 0 if result["matched"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
