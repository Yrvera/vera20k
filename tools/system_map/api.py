"""Public read-only API for validated System Map navigation.

The CLI remains a presentation boundary.  Other repository tools should use
this module instead of importing private CLI loaders or reading canonical JSON
directly.
"""

from __future__ import annotations

from collections import Counter
from copy import deepcopy
from pathlib import Path
import re

from .mechanism_validation import load_mechanisms
from .model import (
    CANONICAL_ID_RE,
    LOOP_ID_RE,
    MECHANISM_ID_RE,
    Diagnostic,
    SystemMapError,
)
from .registry import load_registry, load_source_lock
from .report import build_report, show_mechanism, show_system
from .validation import load_topology, raise_for_errors, validate_all


_TOKEN_RE = re.compile(r"[a-z0-9]+")
_STOP_TERMS = frozenset(
    {
        "a",
        "an",
        "and",
        "for",
        "from",
        "handoff",
        "implementation",
        "index",
        "map",
        "mechanism",
        "block",
        "mcp",
        "navigator",
        "of",
        "or",
        "research",
        "rust",
        "system",
        "the",
        "to",
        "tool",
        "tools",
        "v2",
        "with",
    }
)


def load_report(
    repo: Path,
    *,
    require_sources: bool = True,
    ci: bool = False,
) -> dict:
    """Load, validate, and merge the canonical map with live Git freshness."""

    resolved_repo = repo.resolve()
    registry = load_registry(resolved_repo)
    source_lock = load_source_lock(resolved_repo)
    topology = load_topology(resolved_repo)
    mechanisms = load_mechanisms(resolved_repo)
    diagnostics = validate_all(
        resolved_repo,
        registry,
        source_lock,
        topology,
        mechanisms=mechanisms,
        require_sources=require_sources,
        ci=ci,
    )
    raise_for_errors(diagnostics)
    return build_report(
        resolved_repo,
        registry,
        source_lock,
        topology,
        diagnostics,
        mechanisms=mechanisms,
    )


def require_system(report: dict, system_id: str) -> dict:
    """Return one exact canonical system view or raise a structured error."""

    normalized = system_id.strip().upper()
    if not CANONICAL_ID_RE.fullmatch(normalized):
        raise _not_found("system", system_id)
    result = show_system(report, normalized)
    if result is None or "system" not in result:
        raise _not_found("system", normalized)
    return result


def require_loop(report: dict, loop_id: str) -> dict:
    """Return one exact ordered loop or raise a structured error."""

    normalized = loop_id.strip().upper()
    if not LOOP_ID_RE.fullmatch(normalized):
        raise _not_found("loop", loop_id)
    result = report.get("loops", {}).get(normalized)
    if not isinstance(result, dict):
        raise _not_found("loop", normalized)
    return deepcopy(result)


def require_mechanism(report: dict, mechanism_id: str) -> dict:
    """Return one exact semantic mechanism or raise a structured error."""

    normalized = mechanism_id.strip().upper()
    if not MECHANISM_ID_RE.fullmatch(normalized):
        raise _not_found("mechanism", mechanism_id)
    result = show_mechanism(report, normalized)
    if result is None:
        raise _not_found("mechanism", normalized)
    return result


def find_candidates(report: dict, query: str, limit: int = 8) -> dict:
    """Rank bounded System Map candidates without asserting ownership."""

    normalized_query = " ".join(query.split())
    if not normalized_query:
        raise ValueError("query must not be blank")
    if limit < 1:
        raise ValueError("limit must be at least 1")

    terms = _significant_terms(normalized_query)
    systems = _system_candidates(
        report,
        normalized_query,
        terms,
        limit,
    )
    loops = _loop_candidates(
        report,
        normalized_query,
        terms,
        limit,
    )
    mechanisms = _mechanism_candidates(
        report,
        normalized_query,
        terms,
        limit,
    )
    return {
        "matched": bool(systems or loops or mechanisms),
        "query": normalized_query,
        "query_terms": terms,
        "system_candidates": systems,
        "loop_candidates": loops,
        "mechanism_candidates": mechanisms,
    }


def report_summary(report: dict) -> dict:
    """Return compact live topology and freshness counts."""

    freshness = Counter()
    baseline_freshness = Counter()
    annotated = 0
    for system in report.get("systems", {}).values():
        if system.get("topology"):
            annotated += 1
        states = system.get("freshness", {})
        freshness[
            states.get("rust_mapping_freshness", {}).get("state", "UNKNOWN")
        ] += 1
        baseline_freshness[
            states.get("baseline_status_freshness", {}).get(
                "state", "UNKNOWN"
            )
        ] += 1

    diagnostics = report.get("diagnostics", [])
    repository = report.get("repository", {})
    dirty_paths = list(repository.get("dirty_paths", []))
    return {
        "annotated_systems": annotated,
        "baseline_freshness": dict(sorted(baseline_freshness.items())),
        "diagnostic_count": len(diagnostics),
        "error_count": sum(
            item.get("severity") == "error" for item in diagnostics
        ),
        "loop_count": len(report.get("loops", {})),
        "mechanism_count": len(report.get("mechanisms", {})),
        "mechanism_edge_count": len(report.get("mechanism_edges", [])),
        "mechanism_observed_at_commit": report.get(
            "mechanism_observed_at_commit"
        ),
        "mechanism_schema_version": report.get("mechanism_schema_version"),
        "mapping_freshness": dict(sorted(freshness.items())),
        "observed_at_commit": report.get("observed_at_commit"),
        "repository": {
            "branch": repository.get("branch"),
            "dirty_path_count": len(dirty_paths),
            "dirty_paths": dirty_paths[:20],
            "dirty_paths_omitted": max(0, len(dirty_paths) - 20),
            "head": repository.get("head"),
        },
        "schema_version": report.get("schema_version"),
        "service_count": len(report.get("services", {})),
        "system_count": len(report.get("systems", {})),
        "typed_edge_count": len(report.get("edges", [])),
        "warning_count": sum(
            item.get("severity") == "warning" for item in diagnostics
        ),
    }


def _system_candidates(
    report: dict,
    query: str,
    terms: list[str],
    limit: int,
) -> list[dict]:
    rows: list[dict] = []
    query_folded = query.casefold()
    query_system_ids = {
        item.upper()
        for item in re.findall(
            r"\bGSI-\d{2}\.\d{2}\b",
            query,
            flags=re.IGNORECASE,
        )
    }
    loop_memberships = _loop_memberships(report)
    service_memberships = _service_memberships(report)

    for system_id, system in report.get("systems", {}).items():
        fields: list[tuple[str, int, list[str]]] = [
            (
                "name",
                180,
                [system.get("name", ""), system.get("family_name", "")],
            )
        ]
        topology = system.get("topology", {})
        fields.append(
            (
                "native/rust surface",
                130,
                [
                    *(
                        f"{anchor.get('symbol', '')} "
                        f"{anchor.get('address', '')}"
                        for anchor in topology.get("native_anchors", [])
                        if isinstance(anchor, dict)
                    ),
                    *(
                        f"{surface.get('path', '')} "
                        f"{surface.get('symbol', '')}"
                        for surface in topology.get("rust_surfaces", [])
                        if isinstance(surface, dict)
                    ),
                    *(
                        str(note)
                        for note in topology.get("notes", [])
                        if isinstance(note, str)
                    ),
                ],
            )
        )

        services = service_memberships.get(system_id, [])
        fields.append(
            (
                "service",
                90,
                [
                    f"{slug} {service.get('detail', '')} "
                    f"{' '.join(service.get('roles', []))}"
                    for slug, service in services
                ],
            )
        )

        loops = loop_memberships.get(system_id, [])
        loop_texts = [_loop_search_text(loop_id, loop) for loop_id, loop in loops]
        loop_weight = 85 if any(
            loop.get("owner") == system_id for _, loop in loops
        ) else 55
        fields.append(("loop context", loop_weight, loop_texts))

        score, matched_terms, reasons = _score_fields(
            query_folded,
            terms,
            fields,
        )
        if system_id in query_system_ids:
            score += 10_000
            reasons.insert(0, f"canonical ID: {system_id}")
        if query_folded == system_id.casefold():
            score += 20_000
        if score <= 0 or not _enough_coverage(terms, matched_terms, reasons):
            continue

        view = show_system(report, system_id)
        freshness = system.get("freshness", {})
        rows.append(
            {
                "baseline_status": deepcopy(system.get("baseline_status", {})),
                "candidate_only": True,
                "family": system.get("family"),
                "family_name": system.get("family_name"),
                "freshness": {
                    "baseline_status": freshness.get(
                        "baseline_status_freshness", {}
                    ).get("state", "UNKNOWN"),
                    "rust_mapping": freshness.get(
                        "rust_mapping_freshness", {}
                    ).get("state", "UNKNOWN"),
                },
                "id": system_id,
                "loops": view.get("loops", []) if view else [],
                "match_reasons": reasons[:8],
                "matched_terms": sorted(matched_terms),
                "name": system.get("name"),
                "query_coverage": _coverage(terms, matched_terms),
                "routing_metrics": deepcopy(
                    system.get("routing_metrics", {})
                ),
                "score": score,
                "services": view.get("services", []) if view else [],
            }
        )

    rows.sort(
        key=lambda row: (
            -row["score"],
            -row["query_coverage"],
            row["id"],
            str(row["name"]),
        )
    )
    return rows[:limit]


def _loop_candidates(
    report: dict,
    query: str,
    terms: list[str],
    limit: int,
) -> list[dict]:
    rows: list[dict] = []
    query_folded = query.casefold()
    query_loop_ids = {
        item.upper()
        for item in re.findall(
            r"\bLOOP-\d{3}-[A-Z0-9-]+\b",
            query,
            flags=re.IGNORECASE,
        )
    }

    for loop_id, loop in report.get("loops", {}).items():
        score, matched_terms, reasons = _score_fields(
            query_folded,
            terms,
            [
                (
                    "loop",
                    180,
                    [
                        loop.get("name", ""),
                        loop.get("player_visible_result", ""),
                        loop.get("stock_fixture", ""),
                    ],
                ),
                (
                    "stage",
                    120,
                    [
                        f"{stage.get('action', '')} "
                        f"{stage.get('system', '')}"
                        for stage in loop.get("stages", [])
                        if isinstance(stage, dict)
                    ],
                ),
                (
                    "native/rust surface",
                    100,
                    [
                        *(
                            str(item)
                            for item in loop.get("native_entrypoints", [])
                        ),
                        *(
                            str(item.get("path", ""))
                            for item in loop.get("rust_touchpoints", [])
                            if isinstance(item, dict)
                        ),
                    ],
                ),
            ],
        )
        if loop_id in query_loop_ids:
            score += 10_000
            reasons.insert(0, f"canonical ID: {loop_id}")
        if query_folded == loop_id.casefold():
            score += 20_000
        if score <= 0 or not _enough_coverage(terms, matched_terms, reasons):
            continue

        ordered = list(loop.get("ordered_systems", []))
        rows.append(
            {
                "candidate_only": True,
                "id": loop_id,
                "match_reasons": reasons[:8],
                "matched_terms": sorted(matched_terms),
                "name": loop.get("name"),
                "oracle_status": loop.get("oracle", {}).get(
                    "status", "UNKNOWN"
                ),
                "ordered_systems": ordered,
                "owner": loop.get("owner"),
                "query_coverage": _coverage(terms, matched_terms),
                "score": score,
                "stage_count": len(loop.get("stages", [])),
            }
        )

    rows.sort(
        key=lambda row: (
            -row["score"],
            -row["query_coverage"],
            row["id"],
            str(row["name"]),
        )
    )
    return rows[:limit]


def _mechanism_candidates(
    report: dict,
    query: str,
    terms: list[str],
    limit: int,
) -> list[dict]:
    """Rank mechanism contracts independently of systems and loops."""

    rows: list[dict] = []
    query_folded = query.casefold()
    query_ids = {
        item.upper()
        for item in re.findall(
            r"\bMBLK-\d{3}-[A-Z0-9-]+\b",
            query,
            flags=re.IGNORECASE,
        )
    }
    for block_id, block in report.get("mechanisms", {}).items():
        score, matched_terms, reasons = _score_fields(
            query_folded,
            terms,
            [
                (
                    "mechanism",
                    180,
                    [
                        block.get("name", ""),
                        block.get("contract", ""),
                        block.get("research_query", ""),
                    ],
                ),
                (
                    "activation",
                    125,
                    [
                        str(block.get("activation", {}).get("trigger", "")),
                        str(
                            block.get("activation", {}).get(
                                "stock_fixture", ""
                            )
                        ),
                    ],
                ),
                (
                    "ordered step",
                    110,
                    [
                        f"{step.get('system', '')} "
                        f"{step.get('action', '')}"
                        for step in block.get("steps", [])
                        if isinstance(step, dict)
                    ],
                ),
                (
                    "semantic contract",
                    100,
                    [
                        str(item.get("detail", ""))
                        for item in block.get("critical_semantics", [])
                        if isinstance(item, dict)
                    ],
                ),
                (
                    "native/rust surface",
                    90,
                    [
                        *(
                            f"{anchor.get('symbol', '')} "
                            f"{anchor.get('address', '')}"
                            if isinstance(anchor, dict)
                            else str(anchor)
                            for anchor in block.get("native_anchors", [])
                        ),
                        *(
                            f"{surface.get('path', '')} "
                            f"{surface.get('symbol', '')}"
                            for surface in block.get("rust_surfaces", [])
                            if isinstance(surface, dict)
                        ),
                    ],
                ),
            ],
        )
        if block_id in query_ids:
            score += 10_000
            reasons.insert(0, f"canonical ID: {block_id}")
        if query_folded == block_id.casefold():
            score += 20_000
        if score <= 0 or not _enough_coverage(terms, matched_terms, reasons):
            continue
        rows.append(
            {
                "candidate_only": True,
                "freshness": block.get("freshness", {}).get(
                    "state", "UNMAPPED"
                ),
                "id": block_id,
                "loops": sorted(
                    {
                        item.get("loop")
                        for item in block.get("loop_memberships", [])
                        if isinstance(item, dict)
                        and isinstance(item.get("loop"), str)
                    }
                )[:8],
                "match_reasons": reasons[:8],
                "matched_terms": sorted(matched_terms),
                "name": _bounded_text(block.get("name"), 240),
                "owner": block.get("owner"),
                "participant_count": len(block.get("participants", [])),
                "participants": list(block.get("participants", []))[:12],
                "query_coverage": _coverage(terms, matched_terms),
                "score": score,
            }
        )
    rows.sort(
        key=lambda row: (
            -row["score"],
            -row["query_coverage"],
            row["id"],
            str(row["name"]),
        )
    )
    return rows[:limit]


def _score_fields(
    query_folded: str,
    terms: list[str],
    fields: list[tuple[str, int, list[str]]],
) -> tuple[int, set[str], list[str]]:
    score = 0
    matched: set[str] = set()
    reasons: list[str] = []
    for label, weight, values in fields:
        text = " ".join(str(value) for value in values if value)
        if not text:
            continue
        folded = " ".join(text.casefold().split())
        tokens = set(_TOKEN_RE.findall(folded))
        overlap = [term for term in terms if term in tokens]
        if overlap:
            matched.update(overlap)
            score += weight * len(overlap)
            reasons.append(f"{label}: {', '.join(overlap[:4])}")
        if query_folded and query_folded == folded:
            score += weight * 8
            reasons.append(f"exact {label}")
        elif (
            len(query_folded) >= 6
            and query_folded in folded
            and query_folded not in _STOP_TERMS
        ):
            score += weight * 3
            reasons.append(f"phrase in {label}")
    score += int(1_000 * _coverage(terms, matched))
    return score, matched, reasons


def _significant_terms(query: str) -> list[str]:
    without_ids = re.sub(
        r"\b(?:GSI-\d{2}\.\d{2}|LOOP-\d{3}-[A-Z0-9-]+|"
        r"MBLK-\d{3}-[A-Z0-9-]+)\b",
        " ",
        query,
        flags=re.IGNORECASE,
    )
    return sorted(
        {
            term
            for term in _TOKEN_RE.findall(without_ids.casefold())
            if len(term) >= 2 and term not in _STOP_TERMS
        }
    )


def _coverage(terms: list[str], matched: set[str]) -> float:
    if not terms:
        return 0.0
    return round(len(matched) / len(terms), 3)


def _enough_coverage(
    terms: list[str],
    matched: set[str],
    reasons: list[str],
) -> bool:
    if any(reason.startswith("canonical ID:") for reason in reasons):
        return True
    required = 1 if len(terms) <= 1 else 2
    return len(matched) >= required


def _loop_memberships(report: dict) -> dict[str, list[tuple[str, dict]]]:
    memberships: dict[str, list[tuple[str, dict]]] = {}
    for loop_id, loop in report.get("loops", {}).items():
        systems = set(loop.get("ordered_systems", []))
        owner = loop.get("owner")
        if isinstance(owner, str):
            systems.add(owner)
        for system_id in systems:
            if isinstance(system_id, str):
                memberships.setdefault(system_id, []).append((loop_id, loop))
    return memberships


def _service_memberships(
    report: dict,
) -> dict[str, list[tuple[str, dict]]]:
    memberships: dict[str, list[tuple[str, dict]]] = {}
    for slug, service in report.get("services", {}).items():
        systems = service.get("systems", service.get("gsi_ids", []))
        for system_id in systems:
            if isinstance(system_id, str):
                memberships.setdefault(system_id, []).append((slug, service))
    return memberships


def _loop_search_text(loop_id: str, loop: dict) -> str:
    stage_actions = " ".join(
        str(stage.get("action", ""))
        for stage in loop.get("stages", [])
        if isinstance(stage, dict)
    )
    return " ".join(
        (
            loop_id,
            str(loop.get("name", "")),
            str(loop.get("player_visible_result", "")),
            str(loop.get("stock_fixture", "")),
            stage_actions,
        )
    )


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


def _bounded_text(value: object, limit: int) -> str | None:
    if not isinstance(value, str):
        return None
    compact = " ".join(value.split())
    if len(compact) <= limit:
        return compact
    return compact[: limit - 3].rstrip() + "..."
