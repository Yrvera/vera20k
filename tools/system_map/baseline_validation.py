"""Validation for imported registry, source lock, and live source freshness."""

from __future__ import annotations

from pathlib import Path
import re

from .jsonio import (
    canonical_json_bytes,
    sha256_bytes,
    sha256_file,
    validate_relative_path,
)
from .model import (
    ACTIVITY_VALUES,
    COMMIT_RE,
    Diagnostic,
    INVENTORY_VALUES,
    NATIVE_VALUES,
    PARITY_VALUES,
    REGISTRY_PATH,
    RUST_VALUES,
    SCHEMA_VERSION,
    SOURCE_LOCK_PATH,
    SystemMapError,
    canonical_system_id,
    family_for,
)
from .registry import build_registry


def validate_registry(registry: dict, diagnostics: list[Diagnostic]) -> None:
    if registry.get("schema_version") != SCHEMA_VERSION:
        _error(
            diagnostics,
            "REGISTRY_SCHEMA_VERSION",
            f"registry schema_version must be {SCHEMA_VERSION}",
            path=REGISTRY_PATH.as_posix(),
        )
    snapshot = registry.get("baseline_rust_snapshot")
    if not isinstance(snapshot, str) or not COMMIT_RE.fullmatch(snapshot):
        _error(
            diagnostics,
            "INVALID_BASELINE_COMMIT",
            "baseline_rust_snapshot must be a Git commit ID",
            path=REGISTRY_PATH.as_posix(),
        )
    systems = registry.get("systems")
    families = registry.get("families")
    services = registry.get("service_catalog")
    if not isinstance(systems, dict) or not systems:
        _error(
            diagnostics,
            "INVALID_REGISTRY_SYSTEMS",
            "registry systems must be a non-empty object",
            path=REGISTRY_PATH.as_posix(),
        )
        return
    if not isinstance(families, dict) or not families:
        _error(
            diagnostics,
            "INVALID_REGISTRY_FAMILIES",
            "registry families must be a non-empty object",
            path=REGISTRY_PATH.as_posix(),
        )
        families = {}
    if (
        not isinstance(services, list)
        or not all(isinstance(item, str) and item for item in services)
        or len(services) != len(set(services))
    ):
        _error(
            diagnostics,
            "INVALID_SERVICE_CATALOG",
            "service_catalog must be a unique string array",
            path=REGISTRY_PATH.as_posix(),
        )

    for system_id, record in sorted(systems.items()):
        if canonical_system_id(system_id) is None:
            _error(
                diagnostics,
                "INVALID_SYSTEM_ID",
                "registry key is not a canonical GSI ID",
                record_id=system_id,
            )
            continue
        if not isinstance(record, dict):
            _error(
                diagnostics,
                "INVALID_SYSTEM_RECORD",
                "registry system record must be an object",
                record_id=system_id,
            )
            continue
        for field in ("name", "family", "family_name", "discovery_scope"):
            if not isinstance(record.get(field), str) or not record[field]:
                _error(
                    diagnostics,
                    "INVALID_SYSTEM_FIELD",
                    f"{field} must be a non-empty string",
                    record_id=system_id,
                    field=field,
                )
        if record.get("family") != family_for(system_id):
            _error(
                diagnostics,
                "REGISTRY_FAMILY_MISMATCH",
                f"record family must be {family_for(system_id)}",
                record_id=system_id,
                field="family",
            )
        baseline = record.get("baseline_status")
        if not isinstance(baseline, dict):
            _error(
                diagnostics,
                "MISSING_BASELINE_STATUS",
                "baseline_status must be an object",
                record_id=system_id,
            )
            continue
        allowed_fields = {
            "activity": ACTIVITY_VALUES,
            "inventory_evidence": INVENTORY_VALUES,
            "native_evidence": NATIVE_VALUES,
            "rust_implementation": RUST_VALUES,
            "parity": PARITY_VALUES,
        }
        for field, allowed in allowed_fields.items():
            if baseline.get(field) not in allowed:
                _error(
                    diagnostics,
                    "INVALID_BASELINE_STATUS",
                    f"{field} has invalid value {baseline.get(field)!r}",
                    record_id=system_id,
                    field=f"baseline_status.{field}",
                )
        if not isinstance(baseline.get("basis"), str) or not baseline["basis"]:
            _error(
                diagnostics,
                "INVALID_BASELINE_BASIS",
                "baseline_status.basis must be non-empty",
                record_id=system_id,
            )
        is_group = baseline.get("activity") == "GROUP_NODE"
        if is_group and (
            baseline.get("inventory_evidence") != "GROUP_NODE"
            or baseline.get("native_evidence") != "N/A"
            or baseline.get("rust_implementation") != "N/A"
            or baseline.get("parity") != "N/A"
        ):
            _error(
                diagnostics,
                "INVALID_GROUP_BASELINE",
                "group baseline axes must be GROUP_NODE/N/A/N/A/N/A",
                record_id=system_id,
            )

    for family_id, family in sorted(families.items()):
        if not isinstance(family, dict) or not isinstance(
            family.get("systems"), list
        ):
            _error(
                diagnostics,
                "INVALID_FAMILY_RECORD",
                "family record must contain a systems array",
                record_id=family_id,
            )
            continue
        expected = sorted(
            system_id
            for system_id in systems
            if family_for(system_id) == family_id
        )
        if family["systems"] != expected:
            _error(
                diagnostics,
                "FAMILY_MEMBERSHIP_MISMATCH",
                "family systems do not match canonical registry rows",
                record_id=family_id,
            )


def validate_source_lock(
    registry: dict, source_lock: dict, diagnostics: list[Diagnostic]
) -> None:
    if source_lock.get("schema_version") != SCHEMA_VERSION:
        _error(
            diagnostics,
            "SOURCE_LOCK_SCHEMA_VERSION",
            f"source lock schema_version must be {SCHEMA_VERSION}",
            path=SOURCE_LOCK_PATH.as_posix(),
        )
    expected_digest = sha256_bytes(canonical_json_bytes(registry))
    if source_lock.get("registry_sha256") != expected_digest:
        _error(
            diagnostics,
            "REGISTRY_DIGEST_MISMATCH",
            "source lock registry_sha256 does not match canonical registry",
            path=SOURCE_LOCK_PATH.as_posix(),
        )
    if source_lock.get("baseline_rust_snapshot") != registry.get(
        "baseline_rust_snapshot"
    ):
        _error(
            diagnostics,
            "BASELINE_SNAPSHOT_MISMATCH",
            "registry and source lock baseline commits differ",
            path=SOURCE_LOCK_PATH.as_posix(),
        )
    sources = source_lock.get("sources")
    if not isinstance(sources, dict):
        _error(
            diagnostics,
            "INVALID_SOURCE_LOCK_SOURCES",
            "source lock sources must be an object",
            path=SOURCE_LOCK_PATH.as_posix(),
        )
        return
    for role in ("inventory", "status_matrix", "core_services"):
        source = sources.get(role)
        if not isinstance(source, dict):
            _error(
                diagnostics,
                "MISSING_SOURCE_LOCK_ENTRY",
                f"source lock lacks {role}",
                path=SOURCE_LOCK_PATH.as_posix(),
            )
            continue
        if validate_relative_path(source.get("path")) is None:
            _error(
                diagnostics,
                "UNSAFE_SOURCE_PATH",
                f"{role} path is not portable and relative",
                field=f"sources.{role}.path",
            )
        digest = source.get("sha256")
        if not isinstance(digest, str) or not re.fullmatch(
            r"[0-9a-f]{64}", digest
        ):
            _error(
                diagnostics,
                "INVALID_SOURCE_DIGEST",
                f"{role} sha256 is invalid",
                field=f"sources.{role}.sha256",
            )


def validate_live_sources(
    repo: Path,
    registry: dict,
    source_lock: dict,
    diagnostics: list[Diagnostic],
) -> None:
    try:
        current_registry, current_lock = build_registry(repo)
    except SystemMapError as exc:
        diagnostics.extend(exc.diagnostics)
        return
    if canonical_json_bytes(current_registry) != canonical_json_bytes(registry):
        _error(
            diagnostics,
            "REGISTRY_SOURCE_DRIFT",
            "normalized live inventory/status differs from registry.v2.json",
            path=REGISTRY_PATH.as_posix(),
        )
    if canonical_json_bytes(current_lock) != canonical_json_bytes(source_lock):
        _error(
            diagnostics,
            "SOURCE_LOCK_DRIFT",
            "fresh import source lock differs in path, hash, or provenance",
            path=SOURCE_LOCK_PATH.as_posix(),
        )
    sources = source_lock.get("sources", {})
    for role, locked in sorted(sources.items()):
        if not isinstance(locked, dict):
            continue
        normalized = validate_relative_path(locked.get("path"))
        if normalized is None:
            continue
        absolute = repo / normalized
        if not absolute.exists():
            _error(
                diagnostics,
                "MISSING_LOCKED_SOURCE",
                f"locked {role} source is missing: {normalized}",
                path=normalized,
            )
        elif sha256_file(absolute) != locked.get("sha256"):
            _error(
                diagnostics,
                "SOURCE_HASH_MISMATCH",
                f"locked {role} source has changed: {normalized}",
                path=normalized,
            )
    if current_lock.get("registry_sha256") != source_lock.get("registry_sha256"):
        _error(
            diagnostics,
            "REGISTRY_LOCK_DRIFT",
            "fresh import registry digest differs from source lock",
            path=SOURCE_LOCK_PATH.as_posix(),
        )


def _error(
    diagnostics: list[Diagnostic],
    code: str,
    message: str,
    *,
    record_id: str = "",
    field: str = "",
    path: str = "",
) -> None:
    diagnostics.append(
        Diagnostic(
            "error",
            code,
            message,
            record_id=record_id,
            field=field,
            path=path,
        )
    )
