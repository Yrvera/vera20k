"""Validation for Rust surfaces, native anchors, and evidence citations."""

from __future__ import annotations

from pathlib import Path
import re
import subprocess

from .jsonio import validate_relative_path
from .model import (
    ADDRESS_RE,
    COMMIT_RE,
    Diagnostic,
    RUST_COVERAGE_VALUES,
)


_ADDRESS_IN_TEXT_RE = re.compile(r"\b0x[0-9A-Fa-f]{4,8}\b")
_CITATION_LINE_RE = re.compile(r"^(.*?):(\d+)(?:-(\d+))?$")


def validate_observation_commits(
    repo: Path,
    value: object,
    diagnostics: list[Diagnostic],
) -> None:
    """Verify every declared observation commit exists on current history."""

    probe = subprocess.run(
        ["git", "rev-parse", "--is-inside-work-tree"],
        cwd=repo,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if probe.returncode != 0:
        return
    locations: dict[str, list[str]] = {}

    def collect(item: object, location: str) -> None:
        if isinstance(item, dict):
            for key, child in item.items():
                child_location = f"{location}.{key}"
                if key == "observed_at_commit" and isinstance(child, str):
                    locations.setdefault(child.lower(), []).append(
                        child_location
                    )
                collect(child, child_location)
        elif isinstance(item, list):
            for index, child in enumerate(item):
                collect(child, f"{location}[{index}]")

    collect(value, "$")
    for commit, fields in sorted(locations.items()):
        if not COMMIT_RE.fullmatch(commit):
            continue
        exists = subprocess.run(
            ["git", "cat-file", "-e", f"{commit}^{{commit}}"],
            cwd=repo,
            check=False,
            capture_output=True,
        )
        if exists.returncode != 0:
            _error(
                diagnostics,
                "UNKNOWN_OBSERVATION_COMMIT",
                f"observation commit is unavailable: {commit}",
                field=", ".join(fields),
            )
            continue
        ancestor = subprocess.run(
            ["git", "merge-base", "--is-ancestor", commit, "HEAD"],
            cwd=repo,
            check=False,
            capture_output=True,
        )
        if ancestor.returncode != 0:
            _error(
                diagnostics,
                "DIVERGED_OBSERVATION_COMMIT",
                f"observation commit is not an ancestor of HEAD: {commit}",
                field=", ".join(fields),
            )


def validate_native_edge_evidence(
    edge: dict,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
) -> None:
    """Require native-plane edges to cite concrete research documents."""

    evidence = edge.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        _error(
            diagnostics,
            "NATIVE_EDGE_UNCITED",
            "native edge requires one or more research citations",
            record_id=record_id,
            field="evidence",
        )
        return
    for index, item in enumerate(evidence):
        path, normalized = _citation_path(item)
        if normalized is None or not normalized.startswith("docs/research/"):
            _error(
                diagnostics,
                "INVALID_NATIVE_EDGE_CITATION",
                f"native edge evidence must cite docs/research: {path!r}",
                record_id=record_id,
                field=f"evidence[{index}]",
            )


def validate_service_citations(
    repo: Path,
    evidence: object,
    diagnostics: list[Diagnostic],
    *,
    service_slug: str,
    require_paths: bool,
) -> None:
    """Require each core-service crosswalk citation to identify its slug row."""

    if not isinstance(evidence, list) or not evidence:
        _error(
            diagnostics,
            "MISSING_SERVICE_EVIDENCE",
            "service crosswalk requires a cited catalog row",
            record_id=service_slug,
            field="evidence",
        )
        return
    for index, item in enumerate(evidence):
        path, normalized, start, end = _citation_details(item)
        field = f"evidence[{index}]"
        if (
            normalized is None
            or not normalized.startswith("docs/research/")
            or not isinstance(start, int)
            or not isinstance(end, int)
        ):
            _error(
                diagnostics,
                "INVALID_SERVICE_CITATION",
                "service evidence must cite an exact docs/research line",
                record_id=service_slug,
                field=field,
            )
            continue
        absolute = repo / normalized
        if not absolute.is_file():
            if require_paths:
                _error(
                    diagnostics,
                    "MISSING_SERVICE_CITATION",
                    f"service citation does not exist: {path!r}",
                    record_id=service_slug,
                    field=field,
                )
            continue
        lines = absolute.read_text(encoding="utf-8").splitlines()
        if start < 1 or end < start or end > len(lines):
            _error(
                diagnostics,
                "INVALID_SERVICE_CITATION",
                f"service citation range {start}-{end} is invalid",
                record_id=service_slug,
                field=field,
            )
        elif service_slug not in "\n".join(lines[start - 1 : end]):
            _error(
                diagnostics,
                "SERVICE_CITATION_MISMATCH",
                f"cited row does not contain service slug {service_slug!r}",
                record_id=service_slug,
                field=field,
            )


def validate_research_citations(
    repo: Path,
    evidence: object,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    field: str,
    require_paths: bool,
) -> None:
    """Require non-empty, line-addressed citations into docs/research."""

    if not isinstance(evidence, list) or not evidence:
        _error(
            diagnostics,
            "MISSING_RESEARCH_EVIDENCE",
            "record requires one or more exact research citations",
            record_id=record_id,
            field=field,
        )
        return
    for index, item in enumerate(evidence):
        path, normalized, start, end = _citation_details(item)
        item_field = f"{field}[{index}]"
        if (
            normalized is None
            or not normalized.startswith("docs/research/")
            or not isinstance(start, int)
            or not isinstance(end, int)
        ):
            _error(
                diagnostics,
                "INVALID_RESEARCH_CITATION",
                "evidence must cite an exact docs/research line",
                record_id=record_id,
                field=item_field,
            )
            continue
        absolute = repo / normalized
        if not absolute.is_file():
            if require_paths:
                _error(
                    diagnostics,
                    "MISSING_RESEARCH_CITATION",
                    f"research citation does not exist: {path!r}",
                    record_id=record_id,
                    field=item_field,
                )
            continue
        line_count = len(absolute.read_bytes().splitlines())
        if start < 1 or end < start or end > line_count:
            _error(
                diagnostics,
                "INVALID_RESEARCH_CITATION",
                f"research citation range {start}-{end} is invalid",
                record_id=record_id,
                field=item_field,
            )


def validate_rust_surfaces(
    repo: Path,
    surfaces: object,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    field: str,
    require_paths: bool,
    require_observation: bool = False,
) -> None:
    if surfaces is None:
        return
    if not isinstance(surfaces, list):
        _error(
            diagnostics,
            "INVALID_RUST_SURFACES",
            "Rust surfaces/touchpoints must be an array",
            record_id=record_id,
            field=field,
        )
        return
    for index, surface in enumerate(surfaces):
        if isinstance(surface, str):
            path = surface
            coverage = None
            if require_observation:
                _error(
                    diagnostics,
                    "RUST_SURFACE_REQUIRES_OBJECT",
                    "canonical Rust surfaces require path and observed_at_commit",
                    record_id=record_id,
                    field=f"{field}[{index}]",
                )
        elif isinstance(surface, dict):
            path = surface.get("path")
            coverage = surface.get("coverage")
            observed = surface.get("observed_at_commit")
            if require_observation and observed is None:
                _error(
                    diagnostics,
                    "MISSING_SURFACE_COMMIT",
                    "canonical Rust surface requires observed_at_commit",
                    record_id=record_id,
                    field=f"{field}[{index}]",
                )
            elif observed is not None and (
                not isinstance(observed, str) or not COMMIT_RE.fullmatch(observed)
            ):
                _error(
                    diagnostics,
                    "INVALID_SURFACE_COMMIT",
                    "surface observed_at_commit is invalid",
                    record_id=record_id,
                    field=f"{field}[{index}]",
                )
        else:
            _error(
                diagnostics,
                "INVALID_RUST_SURFACE",
                "Rust surface must be a path string or object",
                record_id=record_id,
                field=f"{field}[{index}]",
            )
            continue
        normalized = validate_relative_path(path)
        if normalized is None or not normalized.startswith(("src/", "tests/")):
            _error(
                diagnostics,
                "INVALID_RUST_PATH",
                f"Rust path is not a portable src/tests path: {path!r}",
                record_id=record_id,
                field=f"{field}[{index}]",
            )
        elif require_paths and not (repo / normalized).exists():
            _error(
                diagnostics,
                "MISSING_RUST_PATH",
                f"mapped Rust path does not exist: {normalized}",
                record_id=record_id,
                field=f"{field}[{index}]",
            )
        if coverage is not None and coverage not in RUST_COVERAGE_VALUES:
            _error(
                diagnostics,
                "INVALID_RUST_COVERAGE",
                f"unsupported coverage {coverage!r}",
                record_id=record_id,
                field=f"{field}[{index}]",
            )


def validate_rust_edge_evidence(
    repo: Path,
    edge: dict,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
) -> None:
    """Require a source commit and concrete Rust files for a Rust-plane edge."""

    observed = edge.get("observed_at_commit")
    if not isinstance(observed, str) or not COMMIT_RE.fullmatch(observed):
        _error(
            diagnostics,
            "MISSING_RUST_EDGE_COMMIT",
            "Rust edge requires a valid observed_at_commit",
            record_id=record_id,
            field="observed_at_commit",
        )
    evidence = edge.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        _error(
            diagnostics,
            "MISSING_RUST_EDGE_EVIDENCE",
            "Rust edge requires one or more exact Rust source files",
            record_id=record_id,
            field="evidence",
        )
        return
    for index, item in enumerate(evidence):
        path = item.get("path") if isinstance(item, dict) else item
        if isinstance(path, str):
            match = _CITATION_LINE_RE.fullmatch(path.strip().replace("\\", "/"))
            if match:
                path = match.group(1)
        normalized = validate_relative_path(path)
        field = f"evidence[{index}]"
        if (
            normalized is None
            or not normalized.startswith(("src/", "tests/"))
            or not normalized.endswith(".rs")
        ):
            _error(
                diagnostics,
                "INVALID_RUST_EDGE_EVIDENCE",
                f"Rust edge evidence must name an exact .rs file: {path!r}",
                record_id=record_id,
                field=field,
            )
        elif not (repo / normalized).is_file():
            _error(
                diagnostics,
                "MISSING_RUST_EDGE_EVIDENCE",
                f"Rust edge evidence file does not exist: {normalized}",
                record_id=record_id,
                field=field,
            )


def validate_native_anchors(
    anchors: object, diagnostics: list[Diagnostic], *, record_id: str
) -> None:
    if anchors is None:
        return
    if not isinstance(anchors, list):
        _error(
            diagnostics,
            "INVALID_NATIVE_ANCHORS",
            "native anchors must be an array",
            record_id=record_id,
        )
        return
    for index, anchor in enumerate(anchors):
        if isinstance(anchor, str):
            if not _ADDRESS_IN_TEXT_RE.findall(anchor):
                _warning(
                    diagnostics,
                    "ANCHOR_WITHOUT_ADDRESS",
                    "native anchor string contains no address",
                    record_id=record_id,
                    field=f"native_anchors[{index}]",
                )
        elif isinstance(anchor, dict):
            address = anchor.get("address")
            if not isinstance(address, str) or not ADDRESS_RE.fullmatch(address):
                _error(
                    diagnostics,
                    "INVALID_NATIVE_ADDRESS",
                    f"invalid native anchor address {address!r}",
                    record_id=record_id,
                    field=f"native_anchors[{index}].address",
                )
            symbol = anchor.get("symbol")
            if not isinstance(symbol, str) or not symbol.strip():
                _error(
                    diagnostics,
                    "MISSING_NATIVE_ANCHOR_SYMBOL",
                    "native anchor requires a non-empty symbol",
                    record_id=record_id,
                    field=f"native_anchors[{index}].symbol",
                )
            evidence = anchor.get("evidence")
            path, normalized = _citation_path(evidence)
            if (
                normalized is None
                or not normalized.startswith("docs/research/")
            ):
                _error(
                    diagnostics,
                    "MISSING_NATIVE_ANCHOR_EVIDENCE",
                    f"native anchor requires a docs/research citation: {path!r}",
                    record_id=record_id,
                    field=f"native_anchors[{index}].evidence",
                )
        else:
            _error(
                diagnostics,
                "INVALID_NATIVE_ANCHOR",
                "native anchor must be a string or object",
                record_id=record_id,
                field=f"native_anchors[{index}]",
            )


def validate_evidence_tree(
    repo: Path,
    value: object,
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
    location: str = "$",
) -> None:
    if isinstance(value, dict):
        if "path" in value and (
            location.endswith("evidence")
            or ".evidence[" in location
            or location.endswith("citation")
            or ".citations[" in location
        ):
            _validate_citation(
                repo,
                value,
                diagnostics,
                require_paths=require_paths,
                location=location,
            )
        for key, item in value.items():
            validate_evidence_tree(
                repo,
                item,
                diagnostics,
                require_paths=require_paths,
                location=f"{location}.{key}",
            )
    elif isinstance(value, list):
        for index, item in enumerate(value):
            validate_evidence_tree(
                repo,
                item,
                diagnostics,
                require_paths=require_paths,
                location=f"{location}[{index}]",
            )
    elif isinstance(value, str) and (
        location.endswith("evidence")
        or ".evidence[" in location
        or location.endswith("citation")
        or ".citations[" in location
    ):
        _validate_citation_string(
            repo,
            value,
            diagnostics,
            require_paths=require_paths,
            location=location,
        )


def _validate_citation(
    repo: Path,
    citation: dict,
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
    location: str,
) -> None:
    path = citation.get("path")
    normalized = validate_relative_path(path)
    if normalized is None:
        _error(
            diagnostics,
            "INVALID_CITATION_PATH",
            f"citation path is not portable: {path!r}",
            field=location,
        )
        return
    if not normalized.startswith(("docs/", "src/", "tests/", "ini/")):
        _error(
            diagnostics,
            "INVALID_EVIDENCE_REFERENCE",
            f"evidence must reference a repository path: {normalized!r}",
            field=location,
        )
        return
    absolute = repo / normalized
    if require_paths and not absolute.exists():
        _error(
            diagnostics,
            "MISSING_CITATION",
            f"cited path does not exist: {normalized}",
            field=location,
        )
        return
    start = citation.get("start_line", citation.get("line"))
    end = citation.get("end_line", start)
    if start is not None and (not isinstance(start, int) or start < 1):
        _error(
            diagnostics,
            "INVALID_CITATION_LINE",
            "citation start line must be a positive integer",
            field=location,
        )
        return
    if end is not None and (
        not isinstance(end, int)
        or end < 1
        or (isinstance(start, int) and end < start)
    ):
        _error(
            diagnostics,
            "INVALID_CITATION_LINE",
            "citation end line must be positive and >= start",
            field=location,
        )
        return
    if absolute.is_file() and isinstance(end, int):
        line_count = len(absolute.read_bytes().splitlines())
        if end > line_count:
            _error(
                diagnostics,
                "CITATION_OUT_OF_BOUNDS",
                f"citation line {end} exceeds {normalized} ({line_count} lines)",
                field=location,
            )


def _validate_citation_string(
    repo: Path,
    value: str,
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
    location: str,
) -> None:
    if not value.strip():
        _error(
            diagnostics,
            "EMPTY_EVIDENCE",
            "evidence citation cannot be empty",
            field=location,
        )
        return
    candidate = value.strip().replace("\\", "/")
    line_match = _CITATION_LINE_RE.fullmatch(candidate)
    line_end: int | None = None
    if line_match:
        candidate = line_match.group(1)
        line_end = int(line_match.group(3) or line_match.group(2))
    if not candidate.startswith(("docs/", "src/", "tests/", "ini/")):
        _error(
            diagnostics,
            "INVALID_EVIDENCE_REFERENCE",
            f"evidence must reference a repository path: {value!r}",
            field=location,
        )
        return
    normalized = validate_relative_path(candidate)
    if normalized is None:
        _error(
            diagnostics,
            "INVALID_CITATION_PATH",
            f"citation path is not portable: {candidate!r}",
            field=location,
        )
        return
    absolute = repo / normalized
    if require_paths and not absolute.exists():
        _error(
            diagnostics,
            "MISSING_CITATION",
            f"cited path does not exist: {normalized}",
            field=location,
        )
    if absolute.is_file() and line_end is not None:
        line_count = len(absolute.read_bytes().splitlines())
        if line_end > line_count:
            _error(
                diagnostics,
                "CITATION_OUT_OF_BOUNDS",
                f"citation line {line_end} exceeds {normalized} ({line_count} lines)",
                field=location,
            )


def _citation_path(value: object) -> tuple[object, str | None]:
    path = value.get("path") if isinstance(value, dict) else value
    if isinstance(path, str):
        match = _CITATION_LINE_RE.fullmatch(path.strip().replace("\\", "/"))
        if match:
            path = match.group(1)
    return path, validate_relative_path(path)


def _citation_details(
    value: object,
) -> tuple[object, str | None, int | None, int | None]:
    if isinstance(value, dict):
        path = value.get("path")
        start = value.get("start_line", value.get("line"))
        end = value.get("end_line", start)
        return path, validate_relative_path(path), start, end
    path: object = value
    start = None
    end = None
    if isinstance(path, str):
        candidate = path.strip().replace("\\", "/")
        match = _CITATION_LINE_RE.fullmatch(candidate)
        if match:
            path = match.group(1)
            start = int(match.group(2))
            end = int(match.group(3) or match.group(2))
    return path, validate_relative_path(path), start, end


def _error(
    diagnostics: list[Diagnostic],
    code: str,
    message: str,
    *,
    record_id: str = "",
    field: str = "",
) -> None:
    diagnostics.append(
        Diagnostic(
            "error", code, message, record_id=record_id, field=field
        )
    )


def _warning(
    diagnostics: list[Diagnostic],
    code: str,
    message: str,
    *,
    record_id: str = "",
    field: str = "",
) -> None:
    diagnostics.append(
        Diagnostic(
            "warning", code, message, record_id=record_id, field=field
        )
    )
