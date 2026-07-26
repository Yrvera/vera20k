#!/usr/bin/env python3
"""Navigate research evidence and System Map routes through one façade."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys


_TOOL_DIR = Path(__file__).resolve().parent
_REPO_ROOT = _TOOL_DIR.parents[1]
sys.path.insert(0, str(_TOOL_DIR))
sys.path.insert(0, str(_REPO_ROOT))

from research_index.database import DEFAULT_DB
from research_index.lifecycle import IndexLifecycleError, ensure_fresh
from research_index.navigator import research_navigate
from research_index.navigator_formatting import format_research_navigator
from tools.system_map.api import load_report
from tools.system_map.model import SystemMapError


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Combine cited research evidence with dependency-aware System "
            "Map candidates and ordered loops."
        )
    )
    parser.add_argument(
        "query",
        help="Natural-language topic or exact canonical GSI/LOOP/MBLK ID",
    )
    parser.add_argument(
        "--anchor",
        action="append",
        default=[],
        help="Exact symbol or address to include; repeatable, maximum eight",
    )
    parser.add_argument("--db", default=str(DEFAULT_DB))
    parser.add_argument("--workspace", default=".")
    parser.add_argument("--system", help="Research-index system filter")
    parser.add_argument("--source", help="Research source-kind filter")
    parser.add_argument(
        "--system-id",
        help="Exact canonical GSI-NN.NN selection",
    )
    parser.add_argument(
        "--loop-id",
        help="Exact canonical LOOP-NNN-SLUG selection",
    )
    parser.add_argument(
        "--mechanism-id",
        help="Exact canonical MBLK-NNN-SLUG selection",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=8,
        help="Rows per section, from 1 through 20",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON")
    return parser.parse_args()


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")

    args = parse_args()
    workspace = Path(args.workspace).resolve()
    db_path = Path(args.db)
    try:
        ensure_fresh(db_path, workspace)
        report = load_report(workspace, require_sources=True)
        result = research_navigate(
            db_path,
            workspace,
            report,
            args.query,
            system=args.system,
            source_kind=args.source,
            anchors=args.anchor,
            system_id=args.system_id,
            loop_id=args.loop_id,
            mechanism_id=args.mechanism_id,
            limit=args.limit,
        )
    except SystemMapError as exc:
        _print_error(
            args.json,
            str(exc),
            diagnostics=[
                diagnostic.to_document()
                for diagnostic in exc.diagnostics
            ],
        )
        return exc.exit_code
    except (IndexLifecycleError, ValueError) as exc:
        _print_error(args.json, str(exc))
        return 2

    print(
        json.dumps(result, indent=2)
        if args.json
        else format_research_navigator(result)
    )
    research_invalid = (
        result["research_matched"]
        and not result["research"]["validation"]["valid"]
    )
    return 0 if result["matched"] and not research_invalid else 1


def _print_error(
    as_json: bool,
    message: str,
    *,
    diagnostics: list[dict] | None = None,
) -> None:
    if as_json:
        print(
            json.dumps(
                {
                    "diagnostics": diagnostics or [],
                    "error": message,
                },
                indent=2,
            ),
            file=sys.stderr,
        )
    else:
        print(f"error: {message}", file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
