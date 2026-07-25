"""Command-line interface for importing, checking, querying, and rendering."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from .jsonio import (
    atomic_write_bytes,
    canonical_json_bytes,
    pretty_json,
    sha256_bytes,
    validate_relative_path,
)
from .model import Diagnostic, REGISTRY_PATH, SOURCE_LOCK_PATH, SystemMapError
from .registry import (
    build_registry,
    find_repo_root,
    load_registry,
    load_source_lock,
    write_import,
)
from .report import (
    build_report,
    format_loop_view,
    format_owner_rows,
    format_stale_rows,
    format_system_view,
    owner_rows,
    render_markdown,
    show_system,
    stale_rows,
)
from .validation import (
    load_topology,
    raise_for_errors,
    validate_all,
)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python -m tools.system_map",
        description=(
            "Build and query VERA20k's dependency-aware System Map v2. "
            "Baseline status fields remain historical and are never completion claims."
        ),
    )
    commands = parser.add_subparsers(dest="command", required=True)

    importer = commands.add_parser(
        "import", help="normalize ignored inventory/status sources"
    )
    importer.add_argument(
        "--output-dir",
        default="system_map",
        help="repository-relative canonical data directory",
    )

    check = commands.add_parser(
        "check", help="validate canonical map and optional live sources"
    )
    source_mode = check.add_mutually_exclusive_group()
    source_mode.add_argument(
        "--require-sources",
        action="store_true",
        help="reparse and hash-check ignored research sources",
    )
    source_mode.add_argument(
        "--ci",
        action="store_true",
        help="validate tracked canonical inputs without ignored sources",
    )

    render = commands.add_parser(
        "render", help="render deterministic merged JSON and Markdown"
    )
    render.add_argument(
        "--output",
        required=True,
        help="repository-relative target/ output directory",
    )
    render.add_argument(
        "--check",
        action="store_true",
        help=(
            "perform no writes and fail unless both rendered files exactly "
            "match the current deterministic output"
        ),
    )

    show = commands.add_parser(
        "show", help="show one GSI node with edges, loops, and freshness"
    )
    show.add_argument("system_id")
    show.add_argument("--json", action="store_true")

    loop = commands.add_parser(
        "loop", help="show one ordered player-visible production loop"
    )
    loop.add_argument("loop_id")
    loop.add_argument("--json", action="store_true")

    owners = commands.add_parser(
        "owners",
        help="rank mapped owner/connectivity coverage (not work priority)",
    )
    owners.add_argument("--limit", type=int, default=20)

    stale = commands.add_parser(
        "stale", help="show Git freshness for all or one mapped GSI node"
    )
    stale.add_argument("system_id", nargs="?")
    stale.add_argument(
        "--include-unmapped",
        action="store_true",
        help="include registry rows with no Rust mapping",
    )
    return parser


def main(
    argv: list[str] | None = None,
    *,
    repo: Path | None = None,
) -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    arguments = _parser().parse_args(argv)
    try:
        root = find_repo_root(repo or Path.cwd())
        if arguments.command == "import":
            output = validate_relative_path(arguments.output_dir)
            if output is None:
                raise SystemMapError(
                    [
                        Diagnostic(
                            "error",
                            "UNSAFE_OUTPUT_PATH",
                            "import output must be repository-relative",
                            field="output_dir",
                        )
                    ],
                    exit_code=1,
                )
            registry, source_lock = build_registry(root)
            write_import(
                root,
                registry,
                source_lock,
                output_dir=Path(output),
            )
            _stdout_json(
                {
                    "baseline_rust_snapshot": registry[
                        "baseline_rust_snapshot"
                    ],
                    "families": len(registry["families"]),
                    "output_registry": (
                        Path(output) / REGISTRY_PATH.name
                    ).as_posix(),
                    "output_source_lock": (
                        Path(output) / SOURCE_LOCK_PATH.name
                    ).as_posix(),
                    "services": len(registry["service_catalog"]),
                    "systems": len(registry["systems"]),
                }
            )
            return 0

        registry, source_lock, topology, diagnostics = _load(
            root,
            require_sources=(
                arguments.command == "check" and arguments.require_sources
            ),
            ci=arguments.command == "check" and arguments.ci,
        )
        raise_for_errors(diagnostics)

        if arguments.command == "check":
            _stdout_json(
                {
                    "edges": len(topology["edges"]),
                    "diagnostics": [
                        item.to_document() for item in diagnostics
                    ],
                    "errors": 0,
                    "families": len(registry["families"]),
                    "loops": len(topology["loops"]),
                    "services": len(topology["services"]),
                    "systems": len(registry["systems"]),
                    "warnings": sum(
                        1
                        for item in diagnostics
                        if item.severity == "warning"
                    ),
                }
            )
            return 0

        report = build_report(
            root, registry, source_lock, topology, diagnostics
        )
        if arguments.command == "render":
            output_value = validate_relative_path(arguments.output)
            if output_value is None or (
                output_value != "target"
                and not output_value.startswith("target/")
            ):
                raise SystemMapError(
                    [
                        Diagnostic(
                            "error",
                            "UNSAFE_OUTPUT_PATH",
                            "render output must be target/ or a child of target/",
                            field="output",
                        )
                    ],
                    exit_code=1,
                )
            output = root / output_value
            json_path = output / "system-map.v2.json"
            markdown_path = output / "SYSTEM_MAP_V2.md"
            json_payload = canonical_json_bytes(report)
            markdown_payload = render_markdown(report).encode("utf-8")
            if arguments.check:
                _check_render_outputs(
                    root,
                    (
                        ("json", json_path, json_payload),
                        ("markdown", markdown_path, markdown_payload),
                    ),
                )
                _stdout_json(
                    {
                        "checked": True,
                        "json": json_path.relative_to(root).as_posix(),
                        "markdown": markdown_path.relative_to(root).as_posix(),
                        "systems": len(report["systems"]),
                    }
                )
                return 0
            atomic_write_bytes(json_path, json_payload)
            atomic_write_bytes(markdown_path, markdown_payload)
            _stdout_json(
                {
                    "json": json_path.relative_to(root).as_posix(),
                    "markdown": markdown_path.relative_to(root).as_posix(),
                    "systems": len(report["systems"]),
                }
            )
            return 0

        if arguments.command == "show":
            view = show_system(report, arguments.system_id)
            if view is None:
                raise _not_found("system", arguments.system_id)
            sys.stdout.write(
                pretty_json(view) if arguments.json else format_system_view(view)
            )
            return 0

        if arguments.command == "loop":
            selected = report["loops"].get(arguments.loop_id)
            if selected is None:
                raise _not_found("loop", arguments.loop_id)
            sys.stdout.write(
                pretty_json(selected)
                if arguments.json
                else format_loop_view(selected)
            )
            return 0

        if arguments.command == "owners":
            if arguments.limit < 1:
                raise SystemMapError(
                    [
                        Diagnostic(
                            "error",
                            "INVALID_LIMIT",
                            "owners --limit must be positive",
                            field="limit",
                        )
                    ],
                    exit_code=1,
                )
            sys.stdout.write(
                format_owner_rows(owner_rows(report, arguments.limit))
            )
            return 0

        if arguments.command == "stale":
            if (
                arguments.system_id is not None
                and arguments.system_id not in report["systems"]
            ):
                raise _not_found("system", arguments.system_id)
            sys.stdout.write(
                format_stale_rows(
                    stale_rows(
                        report,
                        arguments.system_id,
                        include_unmapped=arguments.include_unmapped,
                    )
                )
            )
            return 0

        raise AssertionError(arguments.command)
    except SystemMapError as exc:
        _stderr_json(
            {
                "diagnostics": [
                    item.to_document() for item in sorted(exc.diagnostics)
                ],
                "exit_code": exc.exit_code,
            }
        )
        return exc.exit_code
    except Exception as exc:  # pragma: no cover - defensive CLI boundary
        _stderr_json(
            {
                "diagnostics": [
                    Diagnostic(
                        "error", "INTERNAL_ERROR", str(exc)
                    ).to_document()
                ],
                "exit_code": 3,
            }
        )
        return 3


def _load(
    repo: Path, *, require_sources: bool, ci: bool
) -> tuple[dict, dict, dict, list[Diagnostic]]:
    registry = load_registry(repo)
    source_lock = load_source_lock(repo)
    topology = load_topology(repo)
    diagnostics = validate_all(
        repo,
        registry,
        source_lock,
        topology,
        require_sources=require_sources,
        ci=ci,
    )
    return registry, source_lock, topology, diagnostics


def _not_found(kind: str, value: str) -> SystemMapError:
    return SystemMapError(
        [
            Diagnostic(
                "error",
                f"UNKNOWN_{kind.upper()}",
                f"{kind} not found: {value}",
                record_id=value,
            )
        ],
        exit_code=4,
    )


def _check_render_outputs(
    repo: Path,
    outputs: tuple[tuple[str, Path, bytes], ...],
) -> None:
    """Fail with deterministic diagnostics unless all outputs match exactly."""

    diagnostics: list[Diagnostic] = []
    for output_kind, path, expected in outputs:
        relative_path = path.relative_to(repo).as_posix()
        try:
            actual = path.read_bytes()
        except FileNotFoundError:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "MISSING_RENDER_OUTPUT",
                    "generated output is missing; run render without --check",
                    record_id=output_kind,
                    field="output",
                    path=relative_path,
                )
            )
            continue
        except OSError as exc:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "RENDER_OUTPUT_UNREADABLE",
                    f"generated output cannot be read: {exc}",
                    record_id=output_kind,
                    field="output",
                    path=relative_path,
                )
            )
            continue
        if actual != expected:
            diagnostics.append(
                Diagnostic(
                    "error",
                    "STALE_RENDER_OUTPUT",
                    (
                        "generated output does not match current inputs: "
                        f"expected sha256={sha256_bytes(expected)}, "
                        f"actual sha256={sha256_bytes(actual)}"
                    ),
                    record_id=output_kind,
                    field="output",
                    path=relative_path,
                )
            )
    if diagnostics:
        raise SystemMapError(diagnostics, exit_code=1)


def _stdout_json(value: object) -> None:
    sys.stdout.buffer.write(canonical_json_bytes(value))


def _stderr_json(value: object) -> None:
    sys.stderr.buffer.write(canonical_json_bytes(value))
