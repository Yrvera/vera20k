"""Closed-loop, stage, oracle, and content-addressed proof validation."""

from __future__ import annotations

import hashlib
from pathlib import Path

from .evidence_validation import (
    validate_native_anchors,
    validate_rust_surfaces,
)
from .jsonio import validate_relative_path
from .model import (
    Diagnostic,
    LOOP_ID_RE,
    ORACLE_STATUS_VALUES,
    ORACLE_VERIFICATION_METHOD_VALUES,
    canonical_system_id,
    loop_stage_ids,
)

_LOOP_FIELDS = frozenset(
    {
        "evidence",
        "name",
        "native_entrypoints",
        "oracle",
        "owner",
        "player_visible_result",
        "rust_touchpoints",
        "stages",
        "stock_fixture",
    }
)
_STAGE_FIELDS = frozenset({"action", "order", "rust_surfaces", "system"})
_ORACLE_FIELDS = frozenset({"gate", "status", "verification"})
_VERIFICATION_FIELDS = frozenset(
    {"artifacts", "command", "method", "result"}
)
_ARTIFACT_FIELDS = frozenset({"path", "sha256"})


def validate_loops(
    repo: Path,
    loops: object,
    known_systems: set[str],
    group_systems: set[str],
    diagnostics: list[Diagnostic],
) -> None:
    """Validate all player-visible production loops."""

    if not isinstance(loops, dict):
        _error(
            diagnostics, "INVALID_LOOPS", "loops must be an object", field="loops"
        )
        return
    for loop_id, loop in sorted(loops.items()):
        _validate_loop(
            repo,
            loop_id,
            loop,
            known_systems,
            group_systems,
            diagnostics,
        )


def _validate_loop(
    repo: Path,
    loop_id: str,
    loop: object,
    known_systems: set[str],
    group_systems: set[str],
    diagnostics: list[Diagnostic],
) -> None:
    if not isinstance(loop_id, str) or not LOOP_ID_RE.fullmatch(loop_id):
        _error(
            diagnostics,
            "INVALID_LOOP_ID",
            "loop ID must use LOOP-NNN-SLUG",
            record_id=str(loop_id or ""),
        )
    if not isinstance(loop, dict):
        _error(
            diagnostics,
            "INVALID_LOOP_RECORD",
            "loop must be an object",
            record_id=loop_id,
        )
        return
    _reject_unknown_fields(
        loop,
        _LOOP_FIELDS,
        diagnostics,
        code="UNKNOWN_LOOP_FIELD",
        record_id=loop_id,
        field=f"loops.{loop_id}",
    )
    if not _nonempty(loop.get("name")):
        _error(
            diagnostics,
            "MISSING_LOOP_NAME",
            "loop requires a non-empty name",
            record_id=loop_id,
            field="name",
        )
    owner = loop.get("owner")
    _known_id(owner, known_systems, diagnostics, f"loops.{loop_id}.owner")
    if owner in group_systems:
        _error(
            diagnostics,
            "GROUP_LOOP_OWNER",
            "a group node cannot own an executable loop",
            record_id=loop_id,
            field="owner",
        )
    stage_ids, orders = loop_stage_ids({"stages": loop.get("stages", [])})
    if len(stage_ids) < 2:
        _error(
            diagnostics,
            "LOOP_TOO_SHORT",
            "loop must contain at least two ordered stages",
            record_id=loop_id,
        )
    warned_groups: set[object] = set()
    for system_id in stage_ids:
        _known_id(system_id, known_systems, diagnostics, f"loops.{loop_id}.stages")
        if (
            isinstance(system_id, str)
            and system_id in group_systems
            and system_id not in warned_groups
        ):
            warned_groups.add(system_id)
            _warning(
                diagnostics,
                "GROUP_LOOP_STAGE",
                "loop contains a broad group stage; refine before using it as owner",
                record_id=loop_id,
                field=str(system_id),
            )
    if owner not in stage_ids:
        _error(
            diagnostics,
            "LOOP_OWNER_NOT_IN_STAGES",
            "loop owner must appear in the ordered stages",
            record_id=loop_id,
            field="owner",
        )
    explicit_orders = [order for order in orders if order is not None]
    if explicit_orders and explicit_orders != list(range(1, len(orders) + 1)):
        _error(
            diagnostics,
            "INVALID_LOOP_ORDER",
            "explicit loop stage orders must be contiguous 1..N",
            record_id=loop_id,
        )
    raw_stages = loop.get("stages", [])
    if isinstance(raw_stages, list):
        for index, stage in enumerate(raw_stages):
            if not isinstance(stage, dict):
                _error(
                    diagnostics,
                    "LOOP_STAGE_REQUIRES_OBJECT",
                    "loop stages require order, system, and action fields",
                    record_id=loop_id,
                    field=f"stages[{index}]",
                )
                continue
            _reject_unknown_fields(
                stage,
                _STAGE_FIELDS,
                diagnostics,
                code="UNKNOWN_LOOP_STAGE_FIELD",
                record_id=loop_id,
                field=f"stages[{index}]",
            )
            if not _nonempty(stage.get("action")):
                _error(
                    diagnostics,
                    "MISSING_LOOP_STAGE_ACTION",
                    "loop stage requires a non-empty action",
                    record_id=loop_id,
                    field=f"stages[{index}].action",
                )
            if "rust_surfaces" in stage:
                validate_rust_surfaces(
                    repo,
                    stage["rust_surfaces"],
                    diagnostics,
                    record_id=loop_id,
                    field=f"stages[{index}].rust_surfaces",
                    require_paths=True,
                    require_observation=True,
                )
    fixture = loop.get("stock_fixture")
    if not _nonempty(fixture):
        _error(
            diagnostics,
            "MISSING_LOOP_FIXTURE",
            "loop requires a stock fixture",
            record_id=loop_id,
        )
    visible = loop.get("player_visible_result")
    if not _nonempty(visible):
        _error(
            diagnostics,
            "MISSING_VISIBLE_RESULT",
            "loop requires a player-visible result/assertion",
            record_id=loop_id,
        )
    oracle = loop.get("oracle")
    if not isinstance(oracle, dict):
        _error(
            diagnostics,
            "MISSING_LOOP_ORACLE",
            "loop requires an oracle status and gate",
            record_id=loop_id,
            field="oracle",
        )
    else:
        _reject_unknown_fields(
            oracle,
            _ORACLE_FIELDS,
            diagnostics,
            code="UNKNOWN_ORACLE_FIELD",
            record_id=loop_id,
            field="oracle",
        )
        status = oracle.get("status")
        if status not in ORACLE_STATUS_VALUES:
            _error(
                diagnostics,
                "INVALID_ORACLE_STATUS",
                f"unsupported oracle status {status!r}",
                record_id=loop_id,
                field="oracle.status",
            )
        if not _nonempty(oracle.get("gate")):
            _error(
                diagnostics,
                "MISSING_ORACLE_GATE",
                "loop oracle requires a non-empty gate",
                record_id=loop_id,
                field="oracle.gate",
            )
        if status in {"TRACE_MATCHED", "VERIFIED"}:
            _validate_oracle_verification(
                repo,
                loop_id,
                status,
                oracle.get("verification"),
                diagnostics,
            )
    touchpoints = loop.get("rust_touchpoints")
    if not isinstance(touchpoints, list) or not touchpoints:
        _error(
            diagnostics,
            "MISSING_LOOP_RUST_TOUCHPOINTS",
            "loop requires one or more Rust touchpoints",
            record_id=loop_id,
            field="rust_touchpoints",
        )
    validate_rust_surfaces(
        repo,
        touchpoints,
        diagnostics,
        record_id=loop_id,
        field="rust_touchpoints",
        require_paths=True,
        require_observation=True,
    )
    anchors = loop.get("native_entrypoints")
    if not isinstance(anchors, list) or not anchors:
        _error(
            diagnostics,
            "MISSING_LOOP_NATIVE_ENTRYPOINTS",
            "loop requires one or more native entrypoints",
            record_id=loop_id,
            field="native_entrypoints",
        )
    validate_native_anchors(anchors, diagnostics, record_id=loop_id)
    evidence = loop.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        _error(
            diagnostics,
            "MISSING_LOOP_EVIDENCE",
            "loop requires one or more evidence citations",
            record_id=loop_id,
            field="evidence",
        )


def _validate_oracle_verification(
    repo: Path,
    loop_id: str,
    status: str,
    verification: object,
    diagnostics: list[Diagnostic],
) -> None:
    """Require reproducible, content-addressed proof for positive verdicts."""

    if not isinstance(verification, dict):
        _error(
            diagnostics,
            "MISSING_ORACLE_VERIFICATION",
            f"{status} requires a reproducible verification record",
            record_id=loop_id,
            field="oracle.verification",
        )
        return
    _reject_unknown_fields(
        verification,
        _VERIFICATION_FIELDS,
        diagnostics,
        code="UNKNOWN_ORACLE_VERIFICATION_FIELD",
        record_id=loop_id,
        field="oracle.verification",
    )
    method = verification.get("method")
    if method not in ORACLE_VERIFICATION_METHOD_VALUES:
        _error(
            diagnostics,
            "INVALID_ORACLE_METHOD",
            f"unsupported verification method {method!r}",
            record_id=loop_id,
            field="oracle.verification.method",
        )
    if status == "TRACE_MATCHED" and method != "native_executable":
        _error(
            diagnostics,
            "INVALID_TRACE_METHOD",
            "TRACE_MATCHED requires a native executable comparison",
            record_id=loop_id,
            field="oracle.verification.method",
        )
    for required in ("command", "result"):
        if not _nonempty(verification.get(required)):
            _error(
                diagnostics,
                "INCOMPLETE_ORACLE_VERIFICATION",
                f"oracle verification requires {required}",
                record_id=loop_id,
                field=f"oracle.verification.{required}",
            )
    artifacts = verification.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        _error(
            diagnostics,
            "INCOMPLETE_ORACLE_VERIFICATION",
            "oracle verification requires content-addressed artifacts",
            record_id=loop_id,
            field="oracle.verification.artifacts",
        )
        return
    for index, artifact in enumerate(artifacts):
        field = f"oracle.verification.artifacts[{index}]"
        if not isinstance(artifact, dict):
            _error(
                diagnostics,
                "INVALID_ORACLE_ARTIFACT",
                "oracle artifact must contain a repository-relative path and sha256",
                record_id=loop_id,
                field=field,
            )
            continue
        _reject_unknown_fields(
            artifact,
            _ARTIFACT_FIELDS,
            diagnostics,
            code="UNKNOWN_ORACLE_ARTIFACT_FIELD",
            record_id=loop_id,
            field=field,
        )
        path = validate_relative_path(artifact.get("path"))
        if path is None:
            _error(
                diagnostics,
                "INVALID_ORACLE_ARTIFACT_PATH",
                "oracle artifact path must be portable and repository-relative",
                record_id=loop_id,
                field=f"{field}.path",
            )
            continue
        expected = artifact.get("sha256")
        if (
            not isinstance(expected, str)
            or len(expected) != 64
            or any(
                character not in "0123456789abcdefABCDEF"
                for character in expected
            )
        ):
            _error(
                diagnostics,
                "INVALID_ORACLE_ARTIFACT_SHA256",
                "oracle artifact sha256 must be exactly 64 hexadecimal digits",
                record_id=loop_id,
                field=f"{field}.sha256",
            )
            continue
        absolute = repo / path
        if not absolute.is_file():
            _error(
                diagnostics,
                "MISSING_ORACLE_ARTIFACT",
                f"oracle artifact does not exist: {path}",
                record_id=loop_id,
                field=f"{field}.path",
            )
            continue
        actual = hashlib.sha256(absolute.read_bytes()).hexdigest()
        if actual != expected.lower():
            _error(
                diagnostics,
                "ORACLE_ARTIFACT_DIGEST_MISMATCH",
                f"oracle artifact digest does not match: {path}",
                record_id=loop_id,
                field=f"{field}.sha256",
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


def _reject_unknown_fields(
    value: dict,
    allowed: frozenset[str],
    diagnostics: list[Diagnostic],
    *,
    code: str,
    record_id: str,
    field: str,
) -> None:
    for key in sorted(set(value) - allowed):
        _error(
            diagnostics,
            code,
            f"unsupported field {key!r}",
            record_id=record_id,
            field=f"{field}.{key}",
        )


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
