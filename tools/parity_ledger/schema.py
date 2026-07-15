"""Strict runtime decoders for every persisted parity-ledger document."""

from __future__ import annotations

from enum import Enum
from pathlib import Path
import re
from collections import Counter
from typing import Callable, TypeVar

from . import SCHEMA_VERSION
from .errors import Diagnostic, ExitCode, FailureCode, LedgerError
from .jsonio import canonical_json_bytes, load_json_strict, validate_relative_path
from .model import (
    ActivationClaim,
    ArtifactHashCheck,
    ArtifactRef,
    Assignment,
    AssignmentMention,
    AssignmentRole,
    AssignmentState,
    BridgeTraceCheck,
    Coverage,
    CoverageMode,
    DeterminismImpact,
    Disposition,
    DispositionKind,
    EvidenceDeclaration,
    EvidenceKind,
    EvidenceSetDocument,
    GitAncestorCheck,
    ImplementationState,
    LedgerReport,
    Obligation,
    ObligationKind,
    ObligationSetDocument,
    OracleState,
    ParityVerdict,
    PathExistsCheck,
    PlayerFrequency,
    Provenance,
    QueueState,
    ReducedRow,
    RegressionState,
    Relation,
    RelationKind,
    Severity,
    SourceClaims,
    SourceFileLock,
    SourceLockDocument,
    SourceRef,
    SourceRole,
    SourceState,
    TestDeclaredCheck,
    Tracking,
)


_FORBIDDEN_KEYS = {"status", "done", "complete", "landed", "pass", "verified"}
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_COMMIT_RE = re.compile(r"^[0-9a-f]{7,64}$")
_ID_RE = re.compile(r"^[a-z][a-z0-9_-]*:[A-Za-z0-9][A-Za-z0-9._:-]*$")
_SAFE_PATH_SEGMENT = (
    r"(?!(?:\.{1,2}|(?:[Cc][Oo][Nn]|[Pp][Rr][Nn]|[Aa][Uu][Xx]|[Nn][Uu][Ll]|"
    r"[Cc][Oo][Mm][1-9]|[Ll][Pp][Tt][1-9])[ .]*(?:\.[^/]*)?)(?:/|$))"
    r"(?=[^/]{1,255}(?:/|$))"
    r"[^/\\:\x00-\x1f\x7f]*[^/\\:\x00-\x1f\x7f .]"
)
_SAFE_PATH_PATTERN = rf"^(?:{_SAFE_PATH_SEGMENT})(?:/(?:{_SAFE_PATH_SEGMENT}))*$"
_T = TypeVar("_T")


def _fail(message: str, *, field: str = "", record_id: str = "") -> None:
    raise LedgerError(
        ExitCode.VALIDATION_FAILED,
        [
            Diagnostic(
                FailureCode.SCHEMA_INVALID.value,
                record_id=record_id,
                field=field,
                message=message,
                fatal=True,
            )
        ],
    )


def _reject_forbidden_keys(value: object, path: str = "$") -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            if key.casefold() in _FORBIDDEN_KEYS:
                _fail(f"forbidden result/status key {key!r}", field=f"{path}.{key}")
            _reject_forbidden_keys(item, f"{path}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            _reject_forbidden_keys(item, f"{path}[{index}]")


def _expect_object(value: object, path: str, keys: set[str]) -> dict[str, object]:
    if not isinstance(value, dict):
        _fail("expected object", field=path)
    actual = set(value)
    if actual != keys:
        missing = sorted(keys - actual)
        unknown = sorted(actual - keys)
        _fail(f"object keys differ; missing={missing}, unknown={unknown}", field=path)
    return value


def _expect_list(value: object, path: str) -> list[object]:
    if not isinstance(value, list):
        _fail("expected array", field=path)
    return value


def _expect_str(value: object, path: str, *, allow_empty: bool = False) -> str:
    if not isinstance(value, str) or (not allow_empty and not value):
        _fail("expected non-empty string" if not allow_empty else "expected string", field=path)
    return value


def _expect_int(value: object, path: str, *, minimum: int | None = None) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        _fail("expected integer", field=path)
    if minimum is not None and value < minimum:
        _fail(f"integer must be >= {minimum}", field=path)
    return value


def _expect_bool(value: object, path: str) -> bool:
    if not isinstance(value, bool):
        _fail("expected boolean", field=path)
    return value


def _expect_nullable(value: object, decoder: Callable[[object, str], _T], path: str) -> _T | None:
    return None if value is None else decoder(value, path)


def _expect_enum(value: object, enum_type: type[Enum], path: str):
    string = _expect_str(value, path)
    try:
        return enum_type(string)
    except ValueError:
        _fail(f"unknown {enum_type.__name__}: {string!r}", field=path)


def _expect_sha256(value: object, path: str) -> str:
    string = _expect_str(value, path)
    if not _SHA256_RE.fullmatch(string):
        _fail("expected lowercase 64-character SHA-256", field=path)
    return string


def _expect_path(value: object, path: str) -> str:
    return validate_relative_path(_expect_str(value, path), field=path)


def _expect_version(value: object, path: str) -> int:
    version = _expect_int(value, path)
    if version != SCHEMA_VERSION:
        raise LedgerError(
            ExitCode.VALIDATION_FAILED,
            [
                Diagnostic(
                    FailureCode.UNSUPPORTED_SCHEMA.value,
                    field=path,
                    message=f"unsupported schema version {version}",
                    fatal=True,
                )
            ],
        )
    return version


def _require_sorted_unique(items: list[_T], key: Callable[[_T], object], path: str) -> None:
    keys = [key(item) for item in items]
    if keys != sorted(keys) or len(keys) != len(set(keys)):
        _fail("array must be sorted and unique", field=path)


def _decode_diagnostic(value: object, path: str) -> Diagnostic:
    obj = _expect_object(value, path, {"code", "fatal", "field", "message", "record_id", "source_path"})
    source_path = _expect_str(obj["source_path"], f"{path}.source_path", allow_empty=True)
    if source_path:
        validate_relative_path(source_path, field=f"{path}.source_path")
    return Diagnostic(
        code=_expect_str(obj["code"], f"{path}.code"),
        fatal=_expect_bool(obj["fatal"], f"{path}.fatal"),
        field=_expect_str(obj["field"], f"{path}.field", allow_empty=True),
        message=_expect_str(obj["message"], f"{path}.message", allow_empty=True),
        record_id=_expect_str(obj["record_id"], f"{path}.record_id", allow_empty=True),
        source_path=source_path,
    )


def _decode_source_ref(value: object, path: str) -> SourceRef:
    obj = _expect_object(
        value,
        path,
        {"importer", "importer_version", "local_id", "path", "sha256", "source_key", "tracking"},
    )
    return SourceRef(
        path=_expect_path(obj["path"], f"{path}.path"),
        local_id=_expect_str(obj["local_id"], f"{path}.local_id"),
        source_key=_expect_str(obj["source_key"], f"{path}.source_key"),
        sha256=_expect_sha256(obj["sha256"], f"{path}.sha256"),
        tracking=_expect_enum(obj["tracking"], Tracking, f"{path}.tracking"),
        importer=_expect_str(obj["importer"], f"{path}.importer"),
        importer_version=_expect_int(obj["importer_version"], f"{path}.importer_version", minimum=1),
    )


def _decode_source_lock(value: object, path: str) -> SourceFileLock:
    obj = _expect_object(
        value,
        path,
        {"adapter", "declared_count", "path", "role", "sha256", "source_id", "system", "tracking"},
    )
    declared = obj["declared_count"]
    if declared is not None:
        declared = _expect_int(declared, f"{path}.declared_count", minimum=0)
    return SourceFileLock(
        source_id=_expect_str(obj["source_id"], f"{path}.source_id"),
        system=_expect_str(obj["system"], f"{path}.system"),
        role=_expect_enum(obj["role"], SourceRole, f"{path}.role"),
        path=_expect_path(obj["path"], f"{path}.path"),
        sha256=_expect_sha256(obj["sha256"], f"{path}.sha256"),
        tracking=_expect_enum(obj["tracking"], Tracking, f"{path}.tracking"),
        adapter=_expect_str(obj["adapter"], f"{path}.adapter"),
        declared_count=declared,
    )


def _decode_assignment_mention(value: object, path: str) -> AssignmentMention:
    obj = _expect_object(value, path, {"role", "source", "workstream"})
    workstream = _expect_str(obj["workstream"], f"{path}.workstream")
    if not _ID_RE.fullmatch(workstream):
        _fail("assignment workstream must be namespaced", field=f"{path}.workstream")
    return AssignmentMention(
        workstream=workstream,
        role=_expect_enum(obj["role"], AssignmentRole, f"{path}.role"),
        source=_decode_source_ref(obj["source"], f"{path}.source"),
    )


def _mention_key(item: AssignmentMention) -> tuple[str, str, str, str]:
    return (item.role.value, item.workstream, item.source.path, item.source.local_id)


def _decode_assignment(value: object, path: str) -> Assignment:
    obj = _expect_object(value, path, {"primary", "related"})
    primary = _expect_nullable(obj["primary"], _decode_assignment_mention, f"{path}.primary")
    if primary is not None and primary.role is not AssignmentRole.PRIMARY:
        _fail("primary assignment must use role 'primary'", field=f"{path}.primary.role")
    related = [
        _decode_assignment_mention(item, f"{path}.related[{index}]")
        for index, item in enumerate(_expect_list(obj["related"], f"{path}.related"))
    ]
    if any(item.role is AssignmentRole.PRIMARY for item in related):
        _fail("related assignments cannot use role 'primary'", field=f"{path}.related")
    _require_sorted_unique(related, _mention_key, f"{path}.related")
    return Assignment(primary, tuple(related))


def _decode_source_claims(value: object, path: str) -> SourceClaims:
    obj = _expect_object(value, path, {"activation", "determinism_impact", "player_frequency", "severity"})
    return SourceClaims(
        severity=_expect_enum(obj["severity"], Severity, f"{path}.severity"),
        activation=_expect_enum(obj["activation"], ActivationClaim, f"{path}.activation"),
        player_frequency=_expect_enum(obj["player_frequency"], PlayerFrequency, f"{path}.player_frequency"),
        determinism_impact=_expect_enum(obj["determinism_impact"], DeterminismImpact, f"{path}.determinism_impact"),
    )


def _decode_relation(value: object, path: str) -> Relation:
    obj = _expect_object(value, path, {"kind", "source", "target"})
    return Relation(
        kind=_expect_enum(obj["kind"], RelationKind, f"{path}.kind"),
        target=_expect_str(obj["target"], f"{path}.target"),
        source=_decode_source_ref(obj["source"], f"{path}.source"),
    )


def _decode_disposition(value: object, path: str) -> Disposition:
    obj = _expect_object(value, path, {"kind", "source", "source_id", "targets"})
    source_id = _expect_str(obj["source_id"], f"{path}.source_id")
    if not _ID_RE.fullmatch(source_id):
        _fail("disposition source ID must be namespaced", field=f"{path}.source_id")
    targets = [
        _expect_str(item, f"{path}.targets[{index}]")
        for index, item in enumerate(_expect_list(obj["targets"], f"{path}.targets"))
    ]
    _require_sorted_unique(targets, lambda item: item, f"{path}.targets")
    kind = _expect_enum(obj["kind"], DispositionKind, f"{path}.kind")
    if (kind is DispositionKind.MERGED) != bool(targets):
        _fail("merged requires targets and retired_non_gap forbids targets", field=f"{path}.targets")
    return Disposition(
        source_id=source_id,
        kind=kind,
        targets=tuple(targets),
        source=_decode_source_ref(obj["source"], f"{path}.source"),
    )


def _decode_obligation(value: object, path: str) -> Obligation:
    obj = _expect_object(
        value,
        path,
        {
            "assignment", "dependencies", "id", "kind", "relations", "rust_anchors",
            "schema_version", "source", "source_claims", "system", "title",
        },
    )
    _expect_version(obj["schema_version"], f"{path}.schema_version")
    obligation_id = _expect_str(obj["id"], f"{path}.id")
    system = _expect_str(obj["system"], f"{path}.system")
    if not _ID_RE.fullmatch(obligation_id) or obligation_id.split(":", 1)[0] != system:
        _fail("obligation ID must be namespaced by system", field=f"{path}.id", record_id=obligation_id)
    dependencies = [
        _expect_str(item, f"{path}.dependencies[{index}]")
        for index, item in enumerate(_expect_list(obj["dependencies"], f"{path}.dependencies"))
    ]
    anchors = [
        _expect_path(item, f"{path}.rust_anchors[{index}]")
        for index, item in enumerate(_expect_list(obj["rust_anchors"], f"{path}.rust_anchors"))
    ]
    relations = [
        _decode_relation(item, f"{path}.relations[{index}]")
        for index, item in enumerate(_expect_list(obj["relations"], f"{path}.relations"))
    ]
    _require_sorted_unique(dependencies, lambda item: item, f"{path}.dependencies")
    _require_sorted_unique(anchors, lambda item: item, f"{path}.rust_anchors")
    _require_sorted_unique(relations, lambda item: (item.kind.value, item.target), f"{path}.relations")
    return Obligation(
        id=obligation_id,
        system=system,
        kind=_expect_enum(obj["kind"], ObligationKind, f"{path}.kind"),
        title=_expect_str(obj["title"], f"{path}.title"),
        source=_decode_source_ref(obj["source"], f"{path}.source"),
        source_claims=_decode_source_claims(obj["source_claims"], f"{path}.source_claims"),
        assignment=_decode_assignment(obj["assignment"], f"{path}.assignment"),
        dependencies=tuple(dependencies),
        relations=tuple(relations),
        rust_anchors=tuple(anchors),
    )


def _decode_artifact(value: object, path: str) -> ArtifactRef:
    obj = _expect_object(value, path, {"path", "sha256"})
    return ArtifactRef(_expect_path(obj["path"], f"{path}.path"), _expect_sha256(obj["sha256"], f"{path}.sha256"))


def _artifact_key(item: ArtifactRef) -> tuple[str, str]:
    return (item.path, item.sha256)


def _decode_provenance(value: object, path: str) -> Provenance:
    obj = _expect_object(
        value,
        path,
        {"activation_proof", "executable_sha256", "inputs", "mode", "reference_runs", "scenario", "tool", "tool_version"},
    )
    inputs = [
        _decode_artifact(item, f"{path}.inputs[{index}]")
        for index, item in enumerate(_expect_list(obj["inputs"], f"{path}.inputs"))
    ]
    runs = [
        _decode_artifact(item, f"{path}.reference_runs[{index}]")
        for index, item in enumerate(_expect_list(obj["reference_runs"], f"{path}.reference_runs"))
    ]
    _require_sorted_unique(inputs, _artifact_key, f"{path}.inputs")
    _require_sorted_unique(runs, _artifact_key, f"{path}.reference_runs")
    return Provenance(
        executable_sha256=_expect_sha256(obj["executable_sha256"], f"{path}.executable_sha256"),
        tool=_expect_str(obj["tool"], f"{path}.tool"),
        tool_version=_expect_str(obj["tool_version"], f"{path}.tool_version"),
        scenario=_expect_str(obj["scenario"], f"{path}.scenario"),
        mode=_expect_str(obj["mode"], f"{path}.mode"),
        activation_proof=_expect_str(obj["activation_proof"], f"{path}.activation_proof"),
        inputs=tuple(inputs),
        reference_runs=tuple(runs),
    )


def _decode_coverage(value: object, path: str) -> Coverage:
    obj = _expect_object(value, path, {"domain", "mode"})
    return Coverage(
        _expect_enum(obj["mode"], CoverageMode, f"{path}.mode"),
        _expect_str(obj["domain"], f"{path}.domain"),
    )


def _decode_check(value: object, path: str):
    if not isinstance(value, dict) or "type" not in value:
        _fail("evidence check requires a type", field=path)
    check_type = _expect_str(value["type"], f"{path}.type")
    if check_type == "artifact_hash":
        _expect_object(value, path, {"type"})
        return ArtifactHashCheck()
    if check_type == "git_ancestor":
        obj = _expect_object(value, path, {"commit", "type"})
        commit = _expect_str(obj["commit"], f"{path}.commit")
        if not _COMMIT_RE.fullmatch(commit):
            _fail("invalid lowercase commit hash", field=f"{path}.commit")
        return GitAncestorCheck(commit)
    if check_type == "path_exists":
        obj = _expect_object(value, path, {"path", "type"})
        return PathExistsCheck(_expect_path(obj["path"], f"{path}.path"))
    if check_type == "test_declared":
        obj = _expect_object(value, path, {"commit", "path", "test_name", "type"})
        commit = _expect_str(obj["commit"], f"{path}.commit")
        if not _COMMIT_RE.fullmatch(commit):
            _fail("invalid lowercase commit hash", field=f"{path}.commit")
        return TestDeclaredCheck(
            _expect_path(obj["path"], f"{path}.path"),
            _expect_str(obj["test_name"], f"{path}.test_name"),
            commit,
        )
    if check_type == "bridge_trace":
        obj = _expect_object(value, path, {"left_trace", "right_trace", "type"})
        return BridgeTraceCheck(
            _decode_artifact(obj["left_trace"], f"{path}.left_trace"),
            _decode_artifact(obj["right_trace"], f"{path}.right_trace"),
        )
    _fail(f"unknown evidence check type {check_type!r}", field=f"{path}.type")


def _decode_evidence(value: object, path: str) -> EvidenceDeclaration:
    obj = _expect_object(
        value,
        path,
        {"artifact", "check", "coverage", "id", "kind", "obligations", "provenance", "schema_version"},
    )
    _expect_version(obj["schema_version"], f"{path}.schema_version")
    evidence_id = _expect_str(obj["id"], f"{path}.id")
    if not evidence_id.startswith("evidence:"):
        _fail("evidence ID must use evidence: namespace", field=f"{path}.id")
    obligations = [
        _expect_str(item, f"{path}.obligations[{index}]")
        for index, item in enumerate(_expect_list(obj["obligations"], f"{path}.obligations"))
    ]
    if not obligations:
        _fail("evidence must reference at least one obligation", field=f"{path}.obligations")
    _require_sorted_unique(obligations, lambda item: item, f"{path}.obligations")
    artifact = _expect_nullable(obj["artifact"], _decode_artifact, f"{path}.artifact")
    provenance = _expect_nullable(obj["provenance"], _decode_provenance, f"{path}.provenance")
    coverage = _expect_nullable(obj["coverage"], _decode_coverage, f"{path}.coverage")
    check = _decode_check(obj["check"], f"{path}.check")
    kind = _expect_enum(obj["kind"], EvidenceKind, f"{path}.kind")
    expected_checks = {
        EvidenceKind.IMPLEMENTATION_ANCHOR: PathExistsCheck,
        EvidenceKind.GIT_SCOPED: GitAncestorCheck,
        EvidenceKind.REGRESSION_DECLARATION: TestDeclaredCheck,
        EvidenceKind.GAMEMD_VECTOR: ArtifactHashCheck,
        EvidenceKind.BRIDGE_TRACE: BridgeTraceCheck,
    }
    if not isinstance(check, expected_checks[kind]):
        _fail(
            f"evidence kind {kind.value!r} is incompatible with check {check.type!r}",
            field=f"{path}.check",
        )
    if kind in {
        EvidenceKind.IMPLEMENTATION_ANCHOR,
        EvidenceKind.GIT_SCOPED,
        EvidenceKind.REGRESSION_DECLARATION,
    } and any(item is not None for item in (artifact, provenance, coverage)):
        _fail(
            f"evidence kind {kind.value!r} forbids artifact, provenance, and coverage",
            field=path,
        )
    if kind is EvidenceKind.BRIDGE_TRACE and artifact is not None:
        _fail("bridge_trace forbids a redundant top-level artifact", field=f"{path}.artifact")
    if isinstance(check, ArtifactHashCheck) and artifact is None:
        _fail("artifact_hash requires artifact", field=f"{path}.artifact")
    if isinstance(check, BridgeTraceCheck) and (provenance is None or coverage is None):
        _fail("bridge_trace requires provenance and coverage", field=path)
    return EvidenceDeclaration(
        evidence_id,
        tuple(obligations),
        kind,
        artifact,
        provenance,
        coverage,
        check,
    )


def decode_source_lock(value: object) -> SourceLockDocument:
    _reject_forbidden_keys(value)
    obj = _expect_object(value, "$", {"corpus_digest", "importer", "importer_version", "schema_version", "source_set", "sources"})
    _expect_version(obj["schema_version"], "$.schema_version")
    sources = [
        _decode_source_lock(item, f"$.sources[{index}]")
        for index, item in enumerate(_expect_list(obj["sources"], "$.sources"))
    ]
    _require_sorted_unique(sources, lambda item: item.source_id, "$.sources")
    return SourceLockDocument(
        _expect_str(obj["source_set"], "$.source_set"),
        _expect_str(obj["importer"], "$.importer"),
        _expect_int(obj["importer_version"], "$.importer_version", minimum=1),
        _expect_sha256(obj["corpus_digest"], "$.corpus_digest"),
        tuple(sources),
    )


def decode_obligation_set(value: object) -> ObligationSetDocument:
    _reject_forbidden_keys(value)
    obj = _expect_object(value, "$", {"corpus_digest", "diagnostics", "dispositions", "obligations", "schema_version", "source_set"})
    _expect_version(obj["schema_version"], "$.schema_version")
    obligations = [
        _decode_obligation(item, f"$.obligations[{index}]")
        for index, item in enumerate(_expect_list(obj["obligations"], "$.obligations"))
    ]
    dispositions = [
        _decode_disposition(item, f"$.dispositions[{index}]")
        for index, item in enumerate(_expect_list(obj["dispositions"], "$.dispositions"))
    ]
    diagnostics = [
        _decode_diagnostic(item, f"$.diagnostics[{index}]")
        for index, item in enumerate(_expect_list(obj["diagnostics"], "$.diagnostics"))
    ]
    _require_sorted_unique(obligations, lambda item: item.id, "$.obligations")
    _require_sorted_unique(dispositions, lambda item: item.source_id, "$.dispositions")
    if diagnostics != sorted(diagnostics):
        _fail("diagnostics must be sorted", field="$.diagnostics")
    return ObligationSetDocument(
        _expect_str(obj["source_set"], "$.source_set"),
        _expect_sha256(obj["corpus_digest"], "$.corpus_digest"),
        tuple(obligations),
        tuple(dispositions),
        tuple(diagnostics),
    )


def decode_evidence_set(value: object) -> EvidenceSetDocument:
    _reject_forbidden_keys(value)
    obj = _expect_object(value, "$", {"corpus_digest", "evidence", "schema_version", "source_set"})
    _expect_version(obj["schema_version"], "$.schema_version")
    evidence = [
        _decode_evidence(item, f"$.evidence[{index}]")
        for index, item in enumerate(_expect_list(obj["evidence"], "$.evidence"))
    ]
    _require_sorted_unique(evidence, lambda item: item.id, "$.evidence")
    return EvidenceSetDocument(
        _expect_str(obj["source_set"], "$.source_set"),
        _expect_sha256(obj["corpus_digest"], "$.corpus_digest"),
        tuple(evidence),
    )


def _decode_axis_counts(value: object, path: str, expected: set[str] | None) -> dict[str, int]:
    if not isinstance(value, dict):
        _fail("expected count object", field=path)
    if expected is not None and set(value) != expected:
        _fail(f"count keys differ; expected={sorted(expected)}", field=path)
    result = {key: _expect_int(item, f"{path}.{key}", minimum=0) for key, item in value.items()}
    return result


def decode_ledger(value: object) -> LedgerReport:
    obj = _expect_object(value, "$", {"corpus_digest", "counts", "coverage_state", "diagnostics", "dispositions", "rows", "schema_version"})
    _expect_version(obj["schema_version"], "$.schema_version")
    counts_obj = _expect_object(
        obj["counts"],
        "$.counts",
        {"assignment_state", "implementation_state", "oracle_state", "parity_verdict", "queue_state", "regression_state", "source_state", "system", "total"},
    )
    axes: dict[str, set[str] | None] = {
        "assignment_state": {item.value for item in AssignmentState},
        "implementation_state": {item.value for item in ImplementationState},
        "oracle_state": {item.value for item in OracleState},
        "parity_verdict": {item.value for item in ParityVerdict},
        "queue_state": {item.value for item in QueueState},
        "regression_state": {item.value for item in RegressionState},
        "source_state": {item.value for item in SourceState},
        "system": None,
    }
    counts: dict[str, object] = {
        axis: _decode_axis_counts(counts_obj[axis], f"$.counts.{axis}", expected)
        for axis, expected in axes.items()
    }
    counts["total"] = _expect_int(counts_obj["total"], "$.counts.total", minimum=0)
    rows = []
    for index, item in enumerate(_expect_list(obj["rows"], "$.rows")):
        path = f"$.rows[{index}]"
        row = _expect_object(
            item,
            path,
            {"assignment_state", "diagnostics", "implementation_state", "obligation", "oracle_state", "parity_verdict", "queue_state", "regression_state", "source_state"},
        )
        row_diagnostics = [
            _decode_diagnostic(value, f"{path}.diagnostics[{diagnostic_index}]")
            for diagnostic_index, value in enumerate(_expect_list(row["diagnostics"], f"{path}.diagnostics"))
        ]
        if row_diagnostics != sorted(row_diagnostics):
            _fail("diagnostics must be sorted", field=f"{path}.diagnostics")
        rows.append(
            ReducedRow(
                _decode_obligation(row["obligation"], f"{path}.obligation"),
                _expect_enum(row["source_state"], SourceState, f"{path}.source_state"),
                _expect_enum(row["assignment_state"], AssignmentState, f"{path}.assignment_state"),
                _expect_enum(row["implementation_state"], ImplementationState, f"{path}.implementation_state"),
                _expect_enum(row["regression_state"], RegressionState, f"{path}.regression_state"),
                _expect_enum(row["oracle_state"], OracleState, f"{path}.oracle_state"),
                _expect_enum(row["parity_verdict"], ParityVerdict, f"{path}.parity_verdict"),
                _expect_enum(row["queue_state"], QueueState, f"{path}.queue_state"),
                tuple(row_diagnostics),
            )
        )
    row_ids = [item.obligation.id for item in rows]
    if len(row_ids) != len(set(row_ids)):
        _fail("ledger rows must have unique obligation IDs", field="$.rows")
    dispositions = [
        _decode_disposition(item, f"$.dispositions[{index}]")
        for index, item in enumerate(_expect_list(obj["dispositions"], "$.dispositions"))
    ]
    _require_sorted_unique(dispositions, lambda item: item.source_id, "$.dispositions")
    diagnostics = [
        _decode_diagnostic(item, f"$.diagnostics[{index}]")
        for index, item in enumerate(_expect_list(obj["diagnostics"], "$.diagnostics"))
    ]
    if diagnostics != sorted(diagnostics):
        _fail("diagnostics must be sorted", field="$.diagnostics")
    expected_counts: dict[str, object] = {
        "assignment_state": {
            item.value: sum(row.assignment_state is item for row in rows)
            for item in AssignmentState
        },
        "implementation_state": {
            item.value: sum(row.implementation_state is item for row in rows)
            for item in ImplementationState
        },
        "oracle_state": {
            item.value: sum(row.oracle_state is item for row in rows)
            for item in OracleState
        },
        "parity_verdict": {
            item.value: sum(row.parity_verdict is item for row in rows)
            for item in ParityVerdict
        },
        "queue_state": {
            item.value: sum(row.queue_state is item for row in rows)
            for item in QueueState
        },
        "regression_state": {
            item.value: sum(row.regression_state is item for row in rows)
            for item in RegressionState
        },
        "source_state": {
            item.value: sum(row.source_state is item for row in rows)
            for item in SourceState
        },
        "system": dict(sorted(Counter(row.obligation.system for row in rows).items())),
        "total": len(rows),
    }
    if counts != expected_counts:
        _fail("ledger counts do not match rows", field="$.counts")
    coverage_state = _expect_str(obj["coverage_state"], "$.coverage_state")
    if coverage_state != "BOOTSTRAP_PROVISIONAL":
        _fail("ledger coverage_state must be BOOTSTRAP_PROVISIONAL", field="$.coverage_state")
    return LedgerReport(
        _expect_sha256(obj["corpus_digest"], "$.corpus_digest"),
        coverage_state,
        counts,
        tuple(rows),
        tuple(dispositions),
        tuple(diagnostics),
    )


def load_canonical(path: Path, decoder: Callable[[object], _T]) -> _T:
    raw = path.read_bytes()
    value = load_json_strict(raw)
    if canonical_json_bytes(value) != raw:
        raise LedgerError(
            ExitCode.VALIDATION_FAILED,
            [
                Diagnostic(
                    FailureCode.NONCANONICAL_JSON.value,
                    source_path=path.as_posix(),
                    message="JSON bytes are not canonical",
                    fatal=True,
                )
            ],
        )
    return decoder(value)


_SCHEMA_FILES = {
    "source-lock.v1.schema.json": {"corpus_digest", "importer", "importer_version", "schema_version", "source_set", "sources"},
    "obligation-set.v1.schema.json": {"corpus_digest", "diagnostics", "dispositions", "obligations", "schema_version", "source_set"},
    "evidence-set.v1.schema.json": {"corpus_digest", "evidence", "schema_version", "source_set"},
    "ledger.v1.schema.json": {"corpus_digest", "counts", "coverage_state", "diagnostics", "dispositions", "rows", "schema_version"},
}


def assert_schema_document_parity(schema_dir: Path) -> None:
    """Check the review schemas retain runtime top-level keys and version."""

    for filename, expected in _SCHEMA_FILES.items():
        value = load_json_strict((schema_dir / filename).read_bytes())
        obj = _expect_object(
            value,
            f"schema:{filename}",
            {"$defs", "$id", "$schema", "additionalProperties", "properties", "required", "title", "type"},
        )
        properties = obj["properties"]
        if not isinstance(properties, dict) or set(properties) != expected:
            _fail(f"schema top-level properties differ for {filename}")
        required = _expect_list(obj["required"], f"schema:{filename}.required")
        if required != sorted(expected):
            _fail(f"schema required keys differ for {filename}")
        version_schema = properties.get("schema_version")
        if version_schema != {"const": SCHEMA_VERSION}:
            _fail(f"schema version differs for {filename}")
        if obj["additionalProperties"] is not False:
            _fail(f"schema must reject additional properties for {filename}")
        definitions = obj["$defs"]
        if not isinstance(definitions, dict):
            _fail(f"schema definitions must be an object for {filename}")

        def assert_path_schema(path_schema: object, field: str, *, allow_empty: bool = False) -> None:
            expected = (
                {
                    "anyOf": [
                        {"const": ""},
                        {"pattern": _SAFE_PATH_PATTERN, "type": "string"},
                    ]
                }
                if allow_empty
                else {
                    "minLength": 1,
                    "pattern": _SAFE_PATH_PATTERN,
                    "type": "string",
                }
            )
            if path_schema != expected:
                _fail(f"stored {field} schema differs from runtime path contract in {filename}")

        def assert_path_contract(node: object) -> None:
            if isinstance(node, dict):
                properties_node = node.get("properties")
                if isinstance(properties_node, dict):
                    if "path" in properties_node:
                        assert_path_schema(properties_node["path"], "path")
                    if "source_path" in properties_node:
                        assert_path_schema(
                            properties_node["source_path"],
                            "source_path",
                            allow_empty=True,
                        )
                    if "rust_anchors" in properties_node:
                        anchors_schema = properties_node["rust_anchors"]
                        if not isinstance(anchors_schema, dict):
                            _fail(f"stored rust_anchors schema is not an object in {filename}")
                        assert_path_schema(anchors_schema.get("items"), "rust_anchors item")
                for child in node.values():
                    assert_path_contract(child)
            elif isinstance(node, list):
                for child in node:
                    assert_path_contract(child)

        assert_path_contract(definitions)

        def enum_at(*keys: str) -> set[str]:
            current: object = definitions
            for key in keys:
                if not isinstance(current, dict) or key not in current:
                    _fail(f"missing schema enum path {keys!r} for {filename}")
                current = current[key]
            if not isinstance(current, list) or not all(isinstance(item, str) for item in current):
                _fail(f"schema enum path {keys!r} is not a string array for {filename}")
            return set(current)

        if filename == "source-lock.v1.schema.json":
            if enum_at("source", "properties", "tracking", "enum") != {item.value for item in Tracking}:
                _fail("source-lock tracking enum differs from runtime")
            if enum_at("source", "properties", "role", "enum") != {item.value for item in SourceRole}:
                _fail("source-lock role enum differs from runtime")
        elif filename == "obligation-set.v1.schema.json":
            checks = (
                (("mention", "properties", "role", "enum"), {item.value for item in AssignmentRole}),
                (("obligation", "properties", "kind", "enum"), {item.value for item in ObligationKind}),
                (("relation", "properties", "kind", "enum"), {item.value for item in RelationKind}),
                (("disposition", "properties", "kind", "enum"), {item.value for item in DispositionKind}),
                (("claims", "properties", "severity", "enum"), {item.value for item in Severity}),
                (("claims", "properties", "activation", "enum"), {item.value for item in ActivationClaim}),
                (("claims", "properties", "player_frequency", "enum"), {item.value for item in PlayerFrequency}),
                (("claims", "properties", "determinism_impact", "enum"), {item.value for item in DeterminismImpact}),
            )
            for path, runtime_values in checks:
                if enum_at(*path) != runtime_values:
                    _fail(f"obligation schema enum differs at {path!r}")
        elif filename == "evidence-set.v1.schema.json":
            if enum_at("evidence", "properties", "kind", "enum") != {item.value for item in EvidenceKind}:
                _fail("evidence kind enum differs from runtime")
            if enum_at("coverage", "properties", "mode", "enum") != {item.value for item in CoverageMode}:
                _fail("coverage mode enum differs from runtime")
            check_variants = definitions.get("check", {}).get("oneOf", [])
            test_variant = next(
                (
                    item
                    for item in check_variants
                    if item.get("properties", {}).get("type")
                    == {"const": "test_declared"}
                ),
                None,
            )
            if test_variant is None or test_variant.get("required") != [
                "commit",
                "path",
                "test_name",
                "type",
            ]:
                _fail("test-declaration schema must require commit provenance")
            if test_variant["properties"].get("commit") != {
                "pattern": _COMMIT_RE.pattern,
                "type": "string",
            }:
                _fail("test-declaration commit schema differs from runtime")
        elif filename == "ledger.v1.schema.json":
            checks = (
                ("assignment_state", AssignmentState),
                ("implementation_state", ImplementationState),
                ("oracle_state", OracleState),
                ("parity_verdict", ParityVerdict),
                ("queue_state", QueueState),
                ("regression_state", RegressionState),
                ("source_state", SourceState),
            )
            for field, enum_type in checks:
                if enum_at("row", "properties", field, "enum") != {item.value for item in enum_type}:
                    _fail(f"ledger row enum differs for {field}")
