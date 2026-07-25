"""Strict field-shape checks for canonical topology records."""

from __future__ import annotations

from .model import Diagnostic, ID_POLICY_PATTERNS

_ID_POLICY_FIELDS = frozenset((*ID_POLICY_PATTERNS, "rule"))
_SYSTEM_ANNOTATION_FIELDS = frozenset(
    {"native_anchors", "notes", "rust_surface_coverage", "rust_surfaces"}
)
_SERVICE_FIELDS = frozenset({"detail", "evidence", "gsi_ids", "roles"})
_ALIAS_FIELDS = frozenset(
    {"canonical_systems", "evidence", "legacy_id", "reason", "slice_id"}
)


def validate_id_policy(
    policy: object, diagnostics: list[Diagnostic]
) -> None:
    if not isinstance(policy, dict):
        _error(
            diagnostics,
            "INVALID_ID_POLICY",
            "id_policy must declare the canonical namespace patterns",
            field="id_policy",
        )
        return
    _reject_unknown(
        policy,
        _ID_POLICY_FIELDS,
        diagnostics,
        code="UNKNOWN_ID_POLICY_FIELD",
        field="id_policy",
    )
    if not _nonempty(policy.get("rule")):
        _error(
            diagnostics,
            "MISSING_ID_POLICY_RULE",
            "id_policy requires a non-empty rule",
            field="id_policy.rule",
        )
    for field, expected in ID_POLICY_PATTERNS.items():
        if policy.get(field) != expected:
            _error(
                diagnostics,
                "INVALID_ID_POLICY_PATTERN",
                f"{field} must be {expected!r}",
                field=f"id_policy.{field}",
            )


def validate_system_annotation_shape(
    annotation: dict,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
) -> None:
    _reject_unknown(
        annotation,
        _SYSTEM_ANNOTATION_FIELDS,
        diagnostics,
        code="UNKNOWN_SYSTEM_ANNOTATION_FIELD",
        field=f"systems.{record_id}",
        record_id=record_id,
    )


def validate_service_shape(
    service: dict,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
) -> None:
    _reject_unknown(
        service,
        _SERVICE_FIELDS,
        diagnostics,
        code="UNKNOWN_SERVICE_FIELD",
        field=f"services.{record_id}",
        record_id=record_id,
    )
    if not _nonempty(service.get("detail")):
        _error(
            diagnostics,
            "MISSING_SERVICE_DETAIL",
            "service crosswalk requires non-empty detail",
            record_id=record_id,
            field="detail",
        )


def validate_alias_shape(
    alias: dict,
    diagnostics: list[Diagnostic],
    *,
    record_id: str,
    field: str,
) -> None:
    _reject_unknown(
        alias,
        _ALIAS_FIELDS,
        diagnostics,
        code="UNKNOWN_LEGACY_ALIAS_FIELD",
        field=field,
        record_id=record_id,
    )
    if not _nonempty(alias.get("reason")):
        _error(
            diagnostics,
            "MISSING_LEGACY_ALIAS_REASON",
            "legacy alias requires a non-empty reason",
            record_id=record_id,
            field=f"{field}.reason",
        )
    evidence = alias.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        _error(
            diagnostics,
            "MISSING_LEGACY_ALIAS_EVIDENCE",
            "legacy alias requires path-backed evidence",
            record_id=record_id,
            field=f"{field}.evidence",
        )


def _reject_unknown(
    value: dict,
    allowed: frozenset[str],
    diagnostics: list[Diagnostic],
    *,
    code: str,
    field: str,
    record_id: str = "",
) -> None:
    for key in sorted(set(value) - allowed):
        _error(
            diagnostics,
            code,
            f"unsupported field {key!r}",
            record_id=record_id,
            field=f"{field}.{key}",
        )


def _nonempty(value: object) -> bool:
    if isinstance(value, str):
        return bool(value.strip())
    if isinstance(value, (list, dict)):
        return bool(value)
    return value is not None


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
