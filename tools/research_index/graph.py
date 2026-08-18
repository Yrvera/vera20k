#!/usr/bin/env python3
"""Navigate deterministic research document graph edges."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from research_index.database import DEFAULT_DB
from research_index.graph import backlinks, document_graph, evidence_view, implementation_view
from research_index.formatting import format_backlinks, format_document_graph, format_graph_view


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Navigate the VERA20k research docgraph.")
    parser.add_argument("mode", choices=("doc", "backlinks", "evidence", "implementation"), help="Graph view mode")
    parser.add_argument("target", help="Document path or exact term")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="SQLite database path")
    parser.add_argument("--workspace", default=".", help="Workspace root for Rust-path freshness checks")
    parser.add_argument("--limit", type=int, default=12, help="Maximum rows per section")
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    db_path = Path(args.db)

    if args.mode == "doc":
        result = document_graph(db_path, args.target, args.limit)
        text = format_document_graph(result)
    elif args.mode == "backlinks":
        result = backlinks(db_path, args.target, args.limit)
        text = format_backlinks(result)
    elif args.mode == "evidence":
        result = evidence_view(db_path, args.target, args.limit)
        text = format_graph_view(result)
    else:
        result = implementation_view(
            db_path,
            args.target,
            args.limit,
            workspace=Path(args.workspace).resolve(),
        )
        text = format_graph_view(result)

    print(json.dumps(result, indent=2) if args.json else text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
