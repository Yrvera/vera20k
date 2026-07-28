"""Topology orchestration and structural validation for System Map v2."""

from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path

from .baseline_validation import (
    validate_live_sources,
    validate_registry,
    validate_source_lock,
)
from .evidence_validation import (
    validate_evidence_tree,
    validate_native_anchors,
    validate_observation_commits,
    validate_research_citations,
    validate_rust_surfaces,
    validate_service_citations,
)
from .edge_validation import validate_edges
from .graph_validation import validate_requires_cycles
from .jsonio import load_json_strict
from .loop_validation import validate_loops
from .mechanism_validation import validate_mechanisms
from .model import (
    COMMIT_RE,
    Diagnostic,
    LEGACY_PSEUDO_GSI_RE,
    RUST_COVERAGE_VALUES,
    SCHEMA_VERSION,
    SERVICE_ROLE_VALUES,
    SLICE_ID_RE,
    SystemMapError,
    TOPOLOGY_PATH,
    canonical_system_id,
)
from .shape_validation import (
    validate_alias_shape,
    validate_id_policy,
    validate_service_shape,
    validate_system_annotation_shape,
)


def load_topology(repo: Path) -> dict:
    value = load_json_strict(repo / TOPOLOGY_PATH)
    if not isinstance(value, dict):
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "TOPOLOGY_NOT_OBJECT",
                    "topology root must be an object",
                    path=TOPOLOGY_PATH.as_posix(),
                )
            ]
        )
    return value


def validate_all(
    repo: Path,
    registry: dict,
    source_lock: dict,
    topology: dict,
    *,
    mechanisms: dict | None = None,
    require_sources: bool = False,
    ci: bool = False,
) -> list[Diagnostic]:
    """Return sorted diagnostics for canonical data and optional live sources."""

    diagnostics: list[Diagnostic] = []
    validate_registry(registry, diagnostics)
    validate_source_lock(registry, source_lock, diagnostics)
    systems = registry.get("systems", {})
    known_systems = set(systems) if isinstance(systems, dict) else set()
    group_systems = {
        system_id
        for system_id, record in systems.items()
        if isinstance(record, dict)
        and isinstance(record.get("baseline_status"), dict)
        and record["baseline_status"].get("activity") == "GROUP_NODE"
    }
    _validate_topology(
        repo,
        topology,
        known_systems,
        group_systems,
        set(registry.get("service_catalog", [])),
        diagnostics,
        require_paths=require_sources and not ci,
    )
    if mechanisms is not None:
        validate_mechanisms(
            repo,
            mechanisms,
            known_systems,
            group_systems,
            topology,
            diagnostics,
            require_paths=require_sources and not ci,
        )
    if require_sources and not ci:
        validate_live_sources(repo, registry, source_lock, diagnostics)
    return sorted(diagnostics)


def errors(diagnostics: Iterable[Diagnostic]) -> list[Diagnostic]:
    return [item for item in diagnostics if item.severity == "error"]


def raise_for_errors(diagnostics: list[Diagnostic]) -> None:
    failures = errors(diagnostics)
    if failures:
        raise SystemMapError(failures)


def _validate_topology(
    repo: Path,
    topology: dict,
    known_systems: set[str],
    group_systems: set[str],
    expected_services: set[str],
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
) -> None:
    required_root = {
        "schema_version",
        "observed_at_commit",
        "id_policy",
        "legacy_slice_aliases",
        "systems",
        "services",
        "edges",
        "loops",
        "coupled_sets",
    }
    for field in sorted(required_root - set(topology)):
        _error(
            diagnostics,
            "MISSING_TOPOLOGY_FIELD",
            f"topology lacks required field {field}",
            field=field,
            path=TOPOLOGY_PATH.as_posix(),
        )
    unknown_root = sorted(set(topology) - required_root)
    for field in unknown_root:
        _error(
            diagnostics,
            "UNKNOWN_TOPOLOGY_FIELD",
            f"topology has unsupported root field {field}",
            field=field,
            path=TOPOLOGY_PATH.as_posix(),
        )
    if topology.get("schema_version") != SCHEMA_VERSION:
        _error(
            diagnostics,
            "TOPOLOGY_SCHEMA_VERSION",
            f"topology schema_version must be {SCHEMA_VERSION}",
            path=TOPOLOGY_PATH.as_posix(),
        )
    observed = topology.get("observed_at_commit")
    if not isinstance(observed, str) or not COMMIT_RE.fullmatch(observed):
        _error(
            diagnostics,
            "INVALID_OBSERVED_COMMIT",
            "observed_at_commit must be a Git commit ID",
            path=TOPOLOGY_PATH.as_posix(),
        )
    validate_id_policy(topology.get("id_policy"), diagnostics)

    _validate_system_annotations(
        repo, topology.get("systems"), known_systems, diagnostics
    )
    _validate_services(
        repo,
        topology.get("services"),
        known_systems,
        expected_services,
        diagnostics,
        require_paths=require_paths,
    )
    _validate_aliases(
        topology.get("legacy_slice_aliases"), known_systems, diagnostics
    )
    valid_edges = validate_edges(
        repo,
        topology.get("edges"),
        topology.get("loops"),
        known_systems,
        diagnostics,
    )
    validate_loops(
        repo,
        topology.get("loops"),
        known_systems,
        group_systems,
        diagnostics,
    )
    coupled = _validate_coupled_sets(
        repo,
        topology.get("coupled_sets"),
        known_systems,
        diagnostics,
        require_paths=require_paths,
    )
    validate_requires_cycles(
        valid_edges, known_systems, coupled, diagnostics
    )
    validate_evidence_tree(
        repo,
        topology,
        diagnostics,
        require_paths=require_paths,
    )
    validate_observation_commits(repo, topology, diagnostics)


def _validate_system_annotations(
    repo: Path,
    systems: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
) -> None:
    if not isinstance(systems, dict):
        _error(
            diagnostics,
            "INVALID_TOPOLOGY_SYSTEMS",
            "topology systems must be an object",
            field="systems",
        )
        return
    for system_id, annotation in sorted(systems.items()):
        _known_id(system_id, known_systems, diagnostics, "systems")
        if not isinstance(annotation, dict):
            _error(
                diagnostics,
                "INVALID_SYSTEM_ANNOTATION",
                "system annotation must be an object",
                record_id=system_id,
            )
            continue
        validate_system_annotation_shape(
            annotation, diagnostics, record_id=system_id
        )
        validate_rust_surfaces(
            repo,
            annotation.get("rust_surfaces", []),
            diagnostics,
            record_id=system_id,
            field="systems.rust_surfaces",
            require_paths=True,
            require_observation=True,
        )
        validate_native_anchors(
            annotation.get("native_anchors", []),
            diagnostics,
            record_id=system_id,
        )
        coverage = annotation.get("rust_surface_coverage")
        if coverage is not None and coverage not in RUST_COVERAGE_VALUES:
            _error(
                diagnostics,
                "INVALID_RUST_COVERAGE",
                f"unsupported rust_surface_coverage {coverage!r}",
                record_id=system_id,
            )


def _validate_services(
    repo: Path,
    services: object,
    known_systems: set[str],
    expected_services: set[str],
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
) -> None:
    if not isinstance(services, dict):
        _error(
            diagnostics,
            "INVALID_SERVICES",
            "services must be an object",
            field="services",
        )
        return
    actual_services = set(services)
    for slug in sorted(expected_services - actual_services):
        _error(
            diagnostics,
            "SERVICE_CROSSWALK_MISSING",
            "core service has no topology crosswalk",
            record_id=slug,
        )
    for slug in sorted(actual_services - expected_services):
        _error(
            diagnostics,
            "UNKNOWN_SERVICE_SLUG",
            "topology service is absent from the canonical service catalog",
            record_id=slug,
        )
    for slug, service in sorted(services.items()):
        if not isinstance(service, dict):
            _error(
                diagnostics,
                "INVALID_SERVICE_RECORD",
                "service crosswalk must be an object",
                record_id=slug,
            )
            continue
        validate_service_shape(service, diagnostics, record_id=slug)
        mapped = service.get("gsi_ids")
        validate_service_citations(
            repo,
            service.get("evidence"),
            diagnostics,
            service_slug=slug,
            require_paths=require_paths,
        )
        roles = service.get("roles")
        if not isinstance(roles, list) or not roles:
            _error(
                diagnostics,
                "MISSING_SERVICE_ROLES",
                "service crosswalk requires one or more typed roles",
                record_id=slug,
                field="roles",
            )
        else:
            role_names = [role for role in roles if isinstance(role, str)]
            if len(role_names) != len(set(role_names)):
                _error(
                    diagnostics,
                    "DUPLICATE_SERVICE_ROLE",
                    "service crosswalk contains a duplicate role",
                    record_id=slug,
                    field="roles",
                )
            for role in roles:
                if role not in SERVICE_ROLE_VALUES:
                    _error(
                        diagnostics,
                        "INVALID_SERVICE_ROLE",
                        f"unsupported service role {role!r}",
                        record_id=slug,
                        field="roles",
                    )
        if not isinstance(mapped, list) or not mapped:
            _error(
                diagnostics,
                "EMPTY_SERVICE_CROSSWALK",
                "service gsi_ids must be a non-empty array",
                record_id=slug,
            )
            continue
        for system_id in mapped:
            _known_id(system_id, known_systems, diagnostics, f"services.{slug}")
        mapped_ids = [item for item in mapped if isinstance(item, str)]
        if len(mapped_ids) != len(set(mapped_ids)):
            _error(
                diagnostics,
                "DUPLICATE_SERVICE_SYSTEM",
                "service crosswalk contains a duplicate GSI ID",
                record_id=slug,
            )


def _validate_aliases(
    aliases: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
) -> None:
    if not isinstance(aliases, list):
        _error(
            diagnostics,
            "INVALID_LEGACY_ALIASES",
            "legacy_slice_aliases must be an array",
            field="legacy_slice_aliases",
        )
        return
    seen_legacy: set[str] = set()
    seen_slices: set[str] = set()
    for index, alias in enumerate(aliases):
        field = f"legacy_slice_aliases[{index}]"
        if not isinstance(alias, dict):
            _error(
                diagnostics,
                "INVALID_LEGACY_ALIAS",
                "legacy alias must be an object",
                field=field,
            )
            continue
        name = alias.get("legacy_id")
        if not isinstance(name, str) or not LEGACY_PSEUDO_GSI_RE.fullmatch(name):
            _error(
                diagnostics,
                "INVALID_LEGACY_ALIAS_NAME",
                "legacy_id must be a suffixed pseudo-GSI such as GSI-04.03A",
                field=field,
            )
        elif name in seen_legacy:
            _error(
                diagnostics,
                "DUPLICATE_LEGACY_ALIAS",
                f"duplicate legacy alias {name}",
                record_id=name,
            )
        else:
            seen_legacy.add(name)
            if name[:-1] not in known_systems:
                _error(
                    diagnostics,
                    "UNKNOWN_LEGACY_BASE",
                    f"legacy pseudo-GSI base is absent from registry: {name[:-1]}",
                    record_id=name,
                    field=f"{field}.legacy_id",
                )
        slice_id = alias.get("slice_id")
        if (
            not isinstance(slice_id, str)
            or not SLICE_ID_RE.fullmatch(slice_id)
        ):
            _error(
                diagnostics,
                "INVALID_SLICE_ID",
                "slice_id must use SLICE-YYYYMMDD-SLUG",
                record_id=str(name or ""),
                field=f"{field}.slice_id",
            )
        elif slice_id in seen_slices:
            _error(
                diagnostics,
                "DUPLICATE_SLICE_ID",
                f"duplicate slice ID {slice_id}",
                record_id=slice_id,
                field=f"{field}.slice_id",
            )
        else:
            seen_slices.add(slice_id)
        validate_alias_shape(
            alias,
            diagnostics,
            record_id=str(name or ""),
            field=field,
        )
        targets = alias.get("canonical_systems")
        if isinstance(targets, list) and targets:
            for system_id in targets:
                _known_id(system_id, known_systems, diagnostics, field)
        else:
            _error(
                diagnostics,
                "MISSING_LEGACY_ALIAS_TARGETS",
                "legacy alias requires one or more canonical_systems",
                record_id=str(name or ""),
                field=f"{field}.canonical_systems",
            )


def _validate_coupled_sets(
    repo: Path,
    coupled_sets: object,
    known_systems: set[str],
    diagnostics: list[Diagnostic],
    *,
    require_paths: bool,
) -> list[set[str]]:
    if not isinstance(coupled_sets, list):
        _error(
            diagnostics,
            "INVALID_COUPLED_SETS",
            "coupled_sets must be an array",
            field="coupled_sets",
        )
        return []
    normalized: list[set[str]] = []
    seen: set[str] = set()
    for index, coupled in enumerate(coupled_sets):
        field = f"coupled_sets[{index}]"
        if not isinstance(coupled, dict):
            _error(
                diagnostics,
                "INVALID_COUPLED_SET",
                "coupled set must be an object",
                field=field,
            )
            continue
        coupled_id = coupled.get("id")
        if not isinstance(coupled_id, str) or not coupled_id:
            _error(
                diagnostics,
                "INVALID_COUPLED_SET_ID",
                "coupled set id must be non-empty",
                field=field,
            )
        elif coupled_id in seen:
            _error(
                diagnostics,
                "DUPLICATE_COUPLED_SET",
                f"duplicate coupled set {coupled_id}",
                record_id=coupled_id,
            )
        else:
            seen.add(coupled_id)
        if not _nonempty(coupled.get("reason")):
            _error(
                diagnostics,
                "MISSING_COUPLED_REASON",
                "coupled set requires a non-empty reason",
                record_id=str(coupled_id or ""),
                field=f"{field}.reason",
            )
        validate_research_citations(
            repo,
            coupled.get("evidence"),
            diagnostics,
            record_id=str(coupled_id or ""),
            field=f"{field}.evidence",
            require_paths=require_paths,
        )
        members = coupled.get("systems")
        if not isinstance(members, list) or len(members) < 2:
            _error(
                diagnostics,
                "INVALID_COUPLED_MEMBERS",
                "coupled set needs at least two systems",
                record_id=str(coupled_id or ""),
            )
            continue
        member_ids = [item for item in members if isinstance(item, str)]
        if len(member_ids) != len(set(member_ids)):
            _error(
                diagnostics,
                "DUPLICATE_COUPLED_MEMBER",
                "coupled set must not repeat a system",
                record_id=str(coupled_id or ""),
                field=f"{field}.systems",
            )
        member_set: set[str] = set()
        for system_id in members:
            _known_id(system_id, known_systems, diagnostics, field)
            if isinstance(system_id, str):
                member_set.add(system_id)
        if len(member_set) >= 2:
            normalized.append(member_set)
    return normalized


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
            "warning",
            code,
            message,
            record_id=record_id,
            field=field,
        )
    )
