"""Compose research evidence with dependency-aware System Map navigation."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import re

from .brief import research_brief


NAVIGATOR_MAX_ANCHORS = 8
NAVIGATOR_MAX_DIAGNOSTICS = 40
NAVIGATOR_MAX_LIMIT = 20
NAVIGATOR_MAX_QUERY_LENGTH = 1_000

_SYSTEM_ID_RE = re.compile(r"\bGSI-\d{2}\.\d{2}\b", re.IGNORECASE)
_LOOP_ID_RE = re.compile(
    r"\bLOOP-\d{3}-[A-Z0-9-]+\b",
    re.IGNORECASE,
)
_MECHANISM_ID_RE = re.compile(
    r"\bMBLK-\d{3}-[A-Z0-9-]+\b",
    re.IGNORECASE,
)


def research_navigate(
    db_path: Path,
    workspace: Path,
    report: dict,
    query: str,
    *,
    system: str | None = None,
    source_kind: str | None = None,
    anchors: list[str] | None = None,
    system_id: str | None = None,
    loop_id: str | None = None,
    mechanism_id: str | None = None,
    limit: int = 8,
) -> dict:
    """Build one honest, bounded evidence-and-routing bundle."""

    from tools.system_map.api import (
        find_candidates,
        report_summary,
        require_loop,
        require_mechanism,
        require_system,
    )

    normalized_query = " ".join(query.split())
    if not normalized_query:
        raise ValueError("query must not be blank")
    if len(normalized_query) > NAVIGATOR_MAX_QUERY_LENGTH:
        raise ValueError(
            f"query exceeds {NAVIGATOR_MAX_QUERY_LENGTH} characters"
        )
    if limit < 1 or limit > NAVIGATOR_MAX_LIMIT:
        raise ValueError(
            f"limit must be between 1 and {NAVIGATOR_MAX_LIMIT}"
        )

    anchor_list = [anchor.strip() for anchor in anchors or [] if anchor.strip()]
    if len(anchor_list) > NAVIGATOR_MAX_ANCHORS:
        raise ValueError(
            f"anchors are limited to {NAVIGATOR_MAX_ANCHORS}"
        )

    query_system_ids = _query_ids(_SYSTEM_ID_RE, normalized_query)
    query_loop_ids = _query_ids(_LOOP_ID_RE, normalized_query)
    query_mechanism_ids = _query_ids(_MECHANISM_ID_RE, normalized_query)
    _validate_query_ids(
        report,
        query_system_ids,
        query_loop_ids,
        query_mechanism_ids,
    )

    exact_query_system = _exact_id(_SYSTEM_ID_RE, normalized_query)
    exact_query_loop = _exact_id(_LOOP_ID_RE, normalized_query)
    exact_query_mechanism = _exact_id(_MECHANISM_ID_RE, normalized_query)
    resolved_system_id = _resolve_selector(
        "system",
        system_id,
        exact_query_system,
    )
    resolved_loop_id = _resolve_selector("loop", loop_id, exact_query_loop)
    resolved_mechanism_id = _resolve_selector(
        "mechanism",
        mechanism_id,
        exact_query_mechanism,
    )

    selected_system = (
        require_system(report, resolved_system_id)
        if resolved_system_id
        else None
    )
    selected_loop = (
        require_loop(report, resolved_loop_id) if resolved_loop_id else None
    )
    selected_mechanism_view = (
        require_mechanism(report, resolved_mechanism_id)
        if resolved_mechanism_id
        else None
    )
    selected_mechanism = (
        _mechanism_projection(selected_mechanism_view)
        if selected_mechanism_view
        else None
    )
    candidates = find_candidates(report, normalized_query, limit=limit)
    effective_query = normalized_query
    if exact_query_mechanism and selected_mechanism_view:
        effective_query = selected_mechanism_view["mechanism"].get(
            "research_query", normalized_query
        )
    if (
        not isinstance(effective_query, str)
        or not effective_query.strip()
        or len(effective_query) > NAVIGATOR_MAX_QUERY_LENGTH
    ):
        raise ValueError(
            "selected mechanism research_query is blank or exceeds "
            f"{NAVIGATOR_MAX_QUERY_LENGTH} characters"
        )
    effective_query = " ".join(effective_query.split())
    anchor_list, derived_anchors, anchors_omitted = _research_anchors(
        anchor_list,
        selected_mechanism_view,
    )
    research = research_brief(
        db_path,
        workspace,
        effective_query,
        system=system,
        source_kind=source_kind,
        anchors=anchor_list,
        limit=limit,
    )

    research_matched = _research_matched(research)
    topology_matched = bool(
        selected_system
        or selected_loop
        or selected_mechanism
        or candidates["system_candidates"]
        or candidates["loop_candidates"]
        or candidates["mechanism_candidates"]
    )
    warnings = _warnings(
        report,
        research,
        candidates,
        selected_system,
        selected_loop,
        selected_mechanism,
    )
    diagnostics = list(report.get("diagnostics", []))

    return {
        "anchors": anchor_list,
        "limit": limit,
        "matched": research_matched or topology_matched,
        "query": normalized_query,
        "research_seed": {
            "derived_anchors": derived_anchors,
            "effective_query": effective_query,
            "explicit_anchors": [
                anchor.strip()
                for anchor in anchors or []
                if anchor.strip()
            ],
            "mechanism_anchor_omissions": anchors_omitted,
            "query_substituted_from_mechanism": bool(exact_query_mechanism),
        },
        "research": research,
        "research_matched": research_matched,
        "source_kind": source_kind,
        "system": system,
        "system_map": {
            "diagnostics": deepcopy(
                diagnostics[:NAVIGATOR_MAX_DIAGNOSTICS]
            ),
            "diagnostics_omitted": max(
                0, len(diagnostics) - NAVIGATOR_MAX_DIAGNOSTICS
            ),
            "loop_candidates": candidates["loop_candidates"],
            "mechanism_candidates": candidates["mechanism_candidates"],
            "matched": topology_matched,
            "query_terms": candidates["query_terms"],
            "selected_loop": selected_loop,
            "selected_mechanism": selected_mechanism,
            "selected_system": selected_system,
            "summary": report_summary(report),
            "system_candidates": candidates["system_candidates"],
        },
        "warnings": warnings,
    }


def _mechanism_projection(view: dict) -> dict:
    """Bound an exact mechanism for navigator transport and display."""

    block = view["mechanism"]
    all_memberships = [
        {
            "loop": item.get("loop"),
            "stage_orders": list(item.get("stage_orders", [])),
        }
        for item in block.get("loop_memberships", [])
        if isinstance(item, dict)
    ]
    memberships = all_memberships[:12]
    participants = list(block.get("participants", []))
    incoming = [
        edge.get("id")
        for edge in view.get("incoming_edges", [])
        if isinstance(edge, dict)
    ]
    outgoing = [
        edge.get("id")
        for edge in view.get("outgoing_edges", [])
        if isinstance(edge, dict)
    ]
    return {
        "activation": {
            key: _bounded_text(
                block.get("activation", {}).get(key), 600
            )
            for key in ("mode", "stock_status", "trigger", "stock_fixture")
        },
        "contract": _bounded_text(block.get("contract"), 1_200),
        "critical_semantic_statuses": sorted(
            {
                str(item.get("status", "UNKNOWN"))
                for item in block.get("critical_semantics", [])
                if isinstance(item, dict)
            }
        ),
        "freshness": block.get("freshness", {}).get("state", "UNMAPPED"),
        "id": block.get("id"),
        "incoming_edge_count": len(incoming),
        "incoming_edge_ids": incoming[:20],
        "loop_membership_count": len(all_memberships),
        "loop_memberships": memberships,
        "name": _bounded_text(block.get("name"), 240),
        "open_question_count": len(block.get("open_questions", [])),
        "outgoing_edge_count": len(outgoing),
        "outgoing_edge_ids": outgoing[:20],
        "owner": block.get("owner"),
        "participant_count": len(participants),
        "participants": participants[:20],
        "research_query": _bounded_text(
            block.get("research_query"), NAVIGATOR_MAX_QUERY_LENGTH
        ),
    }


def _research_anchors(
    explicit: list[str],
    selected_view: dict | None,
) -> tuple[list[str], list[str], int]:
    """Merge explicit anchors with deterministic address-first mechanism seeds."""

    result: list[str] = []
    seen: set[str] = set()
    for anchor in explicit:
        folded = anchor.casefold()
        if folded not in seen:
            result.append(anchor)
            seen.add(folded)

    addresses: list[str] = []
    symbols: list[str] = []
    if selected_view:
        for anchor in selected_view["mechanism"].get("native_anchors", []):
            if isinstance(anchor, dict):
                address = anchor.get("address")
                symbol = anchor.get("symbol")
                if isinstance(address, str) and address.strip():
                    addresses.append(address.strip())
                elif isinstance(symbol, str) and symbol.strip():
                    symbols.append(symbol.strip())
            elif isinstance(anchor, str):
                match = re.search(r"\b0x[0-9A-Fa-f]{4,8}\b", anchor)
                if match:
                    addresses.append(match.group(0))
                else:
                    symbols.append(anchor.strip())
    derived: list[str] = []
    omitted = 0
    for anchor in [*addresses, *symbols]:
        folded = anchor.casefold()
        if folded in seen:
            continue
        if len(result) >= NAVIGATOR_MAX_ANCHORS:
            omitted += 1
            continue
        result.append(anchor)
        derived.append(anchor)
        seen.add(folded)
    return result, derived, omitted


def _research_matched(brief: dict) -> bool:
    if brief.get("map", {}).get("matched"):
        return True
    if brief.get("handoff", {}).get("matched"):
        return True
    return any(
        anchor.get("evidence_documents")
        or anchor.get("implementation_documents")
        or anchor.get("rust_paths")
        for anchor in brief.get("anchors", [])
    )


def _warnings(
    report: dict,
    research: dict,
    candidates: dict,
    selected_system: dict | None,
    selected_loop: dict | None,
    selected_mechanism: dict | None,
) -> list[str]:
    warnings: list[str] = []
    if not _research_matched(research):
        warnings.append(
            "Research index matched no documents or exact-anchor evidence."
        )
    if not (
        selected_system
        or selected_loop
        or selected_mechanism
        or candidates["system_candidates"]
        or candidates["loop_candidates"]
        or candidates.get("mechanism_candidates", [])
    ):
        warnings.append(
            "System Map matched no systems, loops, or mechanisms; broaden "
            "the query or provide an exact GSI/LOOP/MBLK ID."
        )
    if (
        candidates["system_candidates"]
        or candidates["loop_candidates"]
        or candidates.get("mechanism_candidates", [])
    ):
        warnings.append(
            "Natural-language System Map matches are navigation candidates, "
            "not verified owners, parity evidence, or completion claims."
        )
    if _ambiguous(candidates["system_candidates"]):
        warnings.append(
            "Multiple System Map systems have equally strong query coverage; "
            "inspect candidates or provide system_id."
        )
    if _ambiguous(candidates["loop_candidates"]):
        warnings.append(
            "Multiple player-visible loops have equally strong query coverage; "
            "inspect candidates or provide loop_id."
        )
    if _ambiguous(candidates.get("mechanism_candidates", [])):
        warnings.append(
            "Multiple mechanism blocks have equally strong query coverage; "
            "inspect candidates or provide mechanism_id."
        )

    if selected_system:
        system = selected_system["system"]
        state = (
            system.get("freshness", {})
            .get("rust_mapping_freshness", {})
            .get("state", "UNKNOWN")
        )
        if state != "FRESH":
            warnings.append(
                f"{system['id']} Rust mapping freshness is {state}; reread "
                "the mapped Rust surface before implementation."
            )
    if selected_loop:
        oracle = selected_loop.get("oracle", {}).get("status", "UNKNOWN")
        if oracle != "VERIFIED":
            warnings.append(
                f"{selected_loop['id']} executable oracle status is {oracle}; "
                "the loop is navigation, not parity proof."
            )
    if selected_mechanism:
        state = selected_mechanism.get("freshness", "UNKNOWN")
        if state != "FRESH":
            warnings.append(
                f"{selected_mechanism['id']} Rust mapping freshness is "
                f"{state}; reread its mapped Rust surfaces before implementation."
            )

    diagnostics = report.get("diagnostics", [])
    if diagnostics:
        warnings.append(
            f"System Map validation reported {len(diagnostics)} warning(s); "
            "inspect system_map.diagnostics for affected routes."
        )
    return warnings


def _ambiguous(rows: list[dict]) -> bool:
    if len(rows) < 2:
        return False
    first, second = rows[:2]
    return (
        first.get("query_coverage") == second.get("query_coverage")
        and first.get("score") == second.get("score")
    )


def _query_ids(pattern: re.Pattern[str], query: str) -> list[str]:
    return sorted({match.group(0).upper() for match in pattern.finditer(query)})


def _exact_id(pattern: re.Pattern[str], query: str) -> str | None:
    match = pattern.fullmatch(query)
    return match.group(0).upper() if match else None


def _validate_query_ids(
    report: dict,
    system_ids: list[str],
    loop_ids: list[str],
    mechanism_ids: list[str],
) -> None:
    from tools.system_map.api import (
        require_loop,
        require_mechanism,
        require_system,
    )

    for system_id in system_ids:
        require_system(report, system_id)
    for loop_id in loop_ids:
        require_loop(report, loop_id)
    for mechanism_id in mechanism_ids:
        require_mechanism(report, mechanism_id)


def _resolve_selector(
    kind: str,
    explicit: str | None,
    exact_query: str | None,
) -> str | None:
    if explicit is None:
        return exact_query
    normalized = explicit.strip().upper()
    if exact_query and normalized != exact_query:
        raise ValueError(
            f"{kind}_id {normalized} conflicts with exact query "
            f"{exact_query}"
        )
    return normalized


def _bounded_text(value: object, limit: int) -> str | None:
    if not isinstance(value, str):
        return None
    compact = " ".join(value.split())
    if len(compact) <= limit:
        return compact
    return compact[: limit - 3].rstrip() + "..."
