"""Command-line interface for importing, checking, and rendering the parity ledger."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from .corpus import import_source_set, load_tracked_corpus, write_import
from .errors import Diagnostic, ExitCode, FailureCode, LedgerError
from .jsonio import canonical_json_bytes
from .renderer import build_report, write_report
from .source_sets import SOURCE_SETS
from .workspace import find_repo_root, resolve_repo_path


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="python -m tools.parity_ledger")
    subcommands = parser.add_subparsers(dest="command", required=True)
    import_parser = subcommands.add_parser("import", help="import ignored source documents")
    import_parser.add_argument("--source-set", choices=sorted(SOURCE_SETS), required=True)
    check = subcommands.add_parser("check", help="validate tracked corpus and current facts")
    modes = check.add_mutually_exclusive_group()
    modes.add_argument("--ci", action="store_true")
    modes.add_argument("--require-sources", action="store_true")
    render = subcommands.add_parser("render", help="render machine and Markdown reports")
    render.add_argument("--output", required=True)
    return parser


def _emit_error(error: LedgerError) -> None:
    document = {
        "diagnostics": [item.to_document() for item in error.diagnostics],
        "exit_code": int(error.exit_code),
    }
    sys.stderr.write(canonical_json_bytes(document).decode("utf-8"))


def _resolve_render_output(root: Path, output_value: str) -> Path:
    textual_target = root / "target"
    resolved_target = textual_target.resolve(strict=False)
    if resolved_target != textual_target:
        raise LedgerError(
            ExitCode.INVALID_ARGUMENT,
            [
                Diagnostic(
                    FailureCode.UNSAFE_PATH.value,
                    field="output",
                    message="repository target/ must not be a symlink or junction",
                    fatal=True,
                )
            ],
        )
    try:
        output = resolve_repo_path(root, output_value)
        output.relative_to(resolved_target)
    except (LedgerError, ValueError) as error:
        diagnostics = (
            list(error.diagnostics)
            if isinstance(error, LedgerError)
            else [
                Diagnostic(
                    FailureCode.UNSAFE_PATH.value,
                    field="output",
                    message="resolved output escapes target/",
                    fatal=True,
                )
            ]
        )
        raise LedgerError(ExitCode.INVALID_ARGUMENT, diagnostics) from error
    return output


def main(argv: list[str] | None = None, *, repo: Path | None = None) -> int:
    arguments = _parser().parse_args(argv)
    try:
        root = find_repo_root(repo or Path.cwd())
        if arguments.command == "import":
            bundle = import_source_set(root, arguments.source_set)
            write_import(root, bundle)
            sys.stdout.write(
                canonical_json_bytes(
                    {
                        "corpus_digest": bundle.digest,
                        "diagnostics": [item.to_document() for item in bundle.diagnostics],
                        "obligations": len(bundle.obligations),
                        "source_set": bundle.source_set,
                    }
                ).decode("utf-8")
            )
            return int(ExitCode.OK)
        corpus = load_tracked_corpus(root)
        if arguments.command == "check":
            mode = "ci" if arguments.ci else "require" if arguments.require_sources else "default"
            if mode == "require":
                fresh = import_source_set(root, "bootstrap")
                if fresh.digest != corpus.source_lock.corpus_digest:
                    raise LedgerError(
                        ExitCode.REQUIRED_SOURCE_FAILED,
                        [
                            Diagnostic(
                                FailureCode.SOURCE_STALE.value,
                                record_id="bootstrap",
                                field="corpus_digest",
                                message=(
                                    "tracked normalized corpus differs from a fresh import: "
                                    f"tracked={corpus.source_lock.corpus_digest}, fresh={fresh.digest}"
                                ),
                                fatal=True,
                            )
                        ],
                    )
            report = build_report(root, corpus, source_mode=mode)
            sys.stdout.write(
                canonical_json_bytes(
                    {
                        "corpus_digest": report.corpus_digest,
                        "counts": report.counts,
                        "coverage_state": report.coverage_state,
                    }
                ).decode("utf-8")
            )
            return int(ExitCode.OK)
        if arguments.command == "render":
            output_value = arguments.output.replace("\\", "/")
            if output_value != "target" and not output_value.startswith("target/"):
                raise LedgerError(
                    ExitCode.INVALID_ARGUMENT,
                    [
                        Diagnostic(
                            FailureCode.UNSAFE_PATH.value,
                            field="output",
                            message="render output must be target/ or a child of target/",
                            fatal=True,
                        )
                    ],
                )
            output = _resolve_render_output(root, output_value)
            report = build_report(root, corpus, source_mode="default")
            write_report(output, report)
            sys.stdout.write(
                canonical_json_bytes(
                    {
                        "corpus_digest": report.corpus_digest,
                        "output": output_value,
                        "rows": len(report.rows),
                    }
                ).decode("utf-8")
            )
            return int(ExitCode.OK)
        raise AssertionError(arguments.command)
    except LedgerError as error:
        _emit_error(error)
        return int(error.exit_code)
    except Exception as exc:  # pragma: no cover - defensive process boundary
        error = LedgerError(
            ExitCode.INTERNAL_ERROR,
            [Diagnostic(FailureCode.INTERNAL_ERROR.value, message=str(exc), fatal=True)],
        )
        _emit_error(error)
        return int(error.exit_code)
