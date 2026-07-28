"""Typed edge and closed-loop requirement validation."""

from __future__ import annotations

from pathlib import Path

from .evidence_validation import (
    validate_native_edge_evidence,
    validate_rust_edge_evidence,
)
from .model import (
    Diagnostic,
    EDGE_ID_RE,
    EDGE_KINDS,
    EDGE_PLANES,
    canonical_system_id,
    loop_stage_ids,
)

_EDGE_FIELDS = frozenset(
    {
        "context",
        "detail",
        "evidence",
        "from",
        "id",
        "kind",
        "loop",
        "observed_at_commit",
        "plane",
        "state",
        "to",
    }
)


def validate_edges(
    repo: Path,
    edges: object,
    loops: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
) -> list[dict]:
    """Validate edge identities, plane contracts, and system references."""

    if not isinstance(edges, list):
        _error(
            diagnostics, "INVALID_EDGES", "edges must be an array", field="edges"
        )
        return []
    seen: set[str] = set()
    seen_semantics: dict[tuple[object, ...], str] = {}
    valid: list[dict] = []
    for index, edge in enumerate(edges):
        field = f"edges[{index}]"
        if not isinstance(edge, dict):
            _error(
                diagnostics, "INVALID_EDGE", "edge must be an object", field=field
            )
            continue
        for key in sorted(set(edge) - _EDGE_FIELDS):
            _error(
                diagnostics,
                "UNKNOWN_EDGE_FIELD",
                f"unsupported edge field {key!r}",
                field=f"{field}.{key}",
            )
        edge_id = edge.get("id")
        if not isinstance(edge_id, str) or not EDGE_ID_RE.fullmatch(edge_id):
            _error(
                diagnostics,
                "INVALID_EDGE_ID",
                "edge id must use EDGE-NNNN-SLUG",
                field=field,
            )
        elif edge_id in seen:
            _error(
                diagnostics,
                "DUPLICATE_EDGE_ID",
                f"duplicate edge id {edge_id}",
                record_id=edge_id,
            )
        else:
            seen.add(edge_id)
        semantic_key = tuple(
            _freeze(edge.get(key))
            for key in (
                "plane",
                "kind",
                "from",
                "to",
                "context",
                "state",
                "loop",
            )
        )
        previous = seen_semantics.get(semantic_key)
        if previous is not None:
            _error(
                diagnostics,
                "DUPLICATE_EDGE_SEMANTICS",
                f"edge duplicates the relationship declared by {previous}",
                record_id=str(edge_id or ""),
            )
        else:
            seen_semantics[semantic_key] = str(edge_id or "")
        if edge.get("plane") not in EDGE_PLANES:
            _error(
                diagnostics,
                "INVALID_EDGE_PLANE",
                f"unsupported edge plane {edge.get('plane')!r}",
                record_id=str(edge_id or ""),
            )
        if edge.get("kind") not in EDGE_KINDS:
            _error(
                diagnostics,
                "INVALID_EDGE_KIND",
                f"unsupported edge kind {edge.get('kind')!r}",
                record_id=str(edge_id or ""),
            )
        _known_id(edge.get("from"), known_systems, diagnostics, field)
        _known_id(edge.get("to"), known_systems, diagnostics, field)
        if not _nonempty(edge.get("detail")):
            _error(
                diagnostics,
                "MISSING_EDGE_DETAIL",
                "edge requires non-empty detail",
                record_id=str(edge_id or ""),
                field="detail",
            )
        if edge.get("kind") == "ordered_before" and not _nonempty(
            edge.get("context")
        ):
            _error(
                diagnostics,
                "MISSING_EDGE_CONTEXT",
                "ordered_before edge requires a context",
                record_id=str(edge_id or ""),
                field="context",
            )
        if edge.get("kind") == "owns_state" and not _nonempty(
            edge.get("state")
        ):
            _error(
                diagnostics,
                "MISSING_EDGE_STATE",
                "owns_state edge requires a named state",
                record_id=str(edge_id or ""),
                field="state",
            )
        if edge.get("plane") == "native":
            validate_native_edge_evidence(
                edge,
                diagnostics,
                record_id=str(edge_id or ""),
            )
        if edge.get("plane") == "rust":
            validate_rust_edge_evidence(
                repo,
                edge,
                diagnostics,
                record_id=str(edge_id or ""),
            )
        if edge.get("kind") == "loop_requires":
            _validate_loop_requirement(edge, loops, diagnostics)
        valid.append(edge)
    return valid


def _validate_loop_requirement(
    edge: dict,
    loops: object,
    diagnostics: list[Diagnostic],
) -> None:
    edge_id = str(edge.get("id") or "")
    if edge.get("plane") != "routing":
        _error(
            diagnostics,
            "INVALID_LOOP_REQUIREMENT_PLANE",
            "loop_requires is restricted to the routing plane",
            record_id=edge_id,
            field="plane",
        )
    loop_id = edge.get("loop")
    if not isinstance(loop_id, str) or not isinstance(loops, dict):
        _error(
            diagnostics,
            "MISSING_LOOP_REQUIREMENT_LOOP",
            "loop_requires must name an existing loop",
            record_id=edge_id,
            field="loop",
        )
        return
    loop = loops.get(loop_id)
    if not isinstance(loop, dict):
        _error(
            diagnostics,
            "UNKNOWN_LOOP_REQUIREMENT_LOOP",
            f"loop_requires references unknown loop {loop_id!r}",
            record_id=edge_id,
            field="loop",
        )
        return
    if edge.get("from") != loop.get("owner"):
        _error(
            diagnostics,
            "LOOP_REQUIREMENT_OWNER_MISMATCH",
            "loop_requires source must equal the named loop owner",
            record_id=edge_id,
            field="from",
        )
    stage_ids, _ = loop_stage_ids(loop)
    if edge.get("to") not in stage_ids:
        _error(
            diagnostics,
            "LOOP_REQUIREMENT_STAGE_MISMATCH",
            "loop_requires target must appear in the named loop stages",
            record_id=edge_id,
            field="to",
        )


def _known_id(
    value: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
    field: str,
) -> None:
    if canonical_system_id(value) is None:
        _error(
            diagnostics,
            "INVALID_SYSTEM_REFERENCE",
            f"not a canonical GSI ID: {value!r}",
            field=field,
        )
    elif value not in known_systems:
        _error(
            diagnostics,
            "UNKNOWN_SYSTEM_REFERENCE",
            f"system is absent from registry: {value}",
            record_id=str(value),
            field=field,
        )


def _nonempty(value: object) -> bool:
    if isinstance(value, str):
        return bool(value.strip())
    if isinstance(value, (list, dict)):
        return bool(value)
    return value is not None


def _freeze(value: object) -> object:
    if isinstance(value, dict):
        return tuple(
            (key, _freeze(item)) for key, item in sorted(value.items())
        )
    if isinstance(value, list):
        return tuple(_freeze(item) for item in value)
    return value


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
