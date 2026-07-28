"""Closed v1 data model for source claims, declarations, and reductions."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Mapping, TypeAlias

from . import SCHEMA_VERSION
from .errors import Diagnostic


class Tracking(str, Enum):
    TRACKED = "tracked"
    IGNORED_LOCAL = "ignored-local"


class SourceRole(str, Enum):
    INVENTORY = "inventory"
    ASSIGNMENT = "assignment"


class AssignmentRole(str, Enum):
    PRIMARY = "primary"
    PARENT = "parent"
    QUICK_WIN = "quick_win"
    RESEARCH_GATE = "research_gate"
    DEFERRED = "deferred"
    HISTORICAL_PARTIAL = "historical_partial"


class DispositionKind(str, Enum):
    MERGED = "merged"
    RETIRED_NON_GAP = "retired_non_gap"


class Severity(str, Enum):
    CRITICAL = "critical"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"
    UNKNOWN = "unknown"


class ActivationClaim(str, Enum):
    ACTIVE = "active"
    CONDITIONAL = "conditional"
    ACTIVE_OR_CONDITIONAL = "active_or_conditional"
    UNKNOWN = "unknown"


class PlayerFrequency(str, Enum):
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"
    UNKNOWN = "unknown"


class DeterminismImpact(str, Enum):
    SIM_CRITICAL = "sim_critical"
    OUTPUT_ONLY = "output_only"
    UNKNOWN = "unknown"


class ObligationKind(str, Enum):
    CORE_OBLIGATION = "core_obligation"
    CONTRACT = "contract"
    IMPLEMENTATION = "implementation"
    RESEARCH = "research"
    PARITY_GAP = "parity_gap"


class RelationKind(str, Enum):
    RELATED = "related"
    OVERLAPS = "overlaps"
    SUPERSEDES = "supersedes"


class EvidenceKind(str, Enum):
    GIT_SCOPED = "git_scoped"
    IMPLEMENTATION_ANCHOR = "implementation_anchor"
    REGRESSION_DECLARATION = "regression_declaration"
    GAMEMD_VECTOR = "gamemd_vector"
    BRIDGE_TRACE = "bridge_trace"


class CoverageMode(str, Enum):
    INCOMPLETE = "incomplete"
    SAMPLED = "sampled"
    EXHAUSTIVE = "exhaustive"


class SourceState(str, Enum):
    CURRENT = "CURRENT"
    STALE = "STALE"
    UNAVAILABLE = "UNAVAILABLE"


class AssignmentState(str, Enum):
    ASSIGNED = "ASSIGNED"
    UNASSIGNED = "UNASSIGNED"
    ALIASED = "ALIASED"
    SUPERSEDED = "SUPERSEDED"


class ImplementationState(str, Enum):
    NONE = "NONE"
    CANDIDATE = "CANDIDATE"
    LANDED = "LANDED"
    STALE_MAPPING = "STALE_MAPPING"


class RegressionState(str, Enum):
    NONE = "NONE"
    DECLARED = "DECLARED"
    PASS = "PASS"
    FAIL = "FAIL"


class OracleState(str, Enum):
    NONE = "NONE"
    INCOMPLETE = "INCOMPLETE"
    SAMPLED = "SAMPLED"
    EXHAUSTIVE = "EXHAUSTIVE"


class ParityVerdict(str, Enum):
    DRIFT = "DRIFT"
    UNCHECKED = "UNCHECKED"
    UNVERIFIED = "UNVERIFIED"
    VERIFIED = "VERIFIED"


class QueueState(str, Enum):
    NOT_ACTIONABLE = "NOT_ACTIONABLE"
    NEEDS_SOURCE_REFRESH = "NEEDS_SOURCE_REFRESH"
    NEEDS_ASSIGNMENT = "NEEDS_ASSIGNMENT"
    DEPENDENCY_BLOCKED = "DEPENDENCY_BLOCKED"
    NEEDS_RESEARCH = "NEEDS_RESEARCH"
    NEEDS_IMPLEMENTATION = "NEEDS_IMPLEMENTATION"
    NEEDS_REGRESSION = "NEEDS_REGRESSION"
    NEEDS_ORACLE = "NEEDS_ORACLE"
    NO_ACTION = "NO_ACTION"


@dataclass(frozen=True)
class SourceRef:
    path: str
    local_id: str
    source_key: str
    sha256: str
    tracking: Tracking
    importer: str
    importer_version: int

    def to_document(self) -> dict[str, object]:
        return {
            "importer": self.importer,
            "importer_version": self.importer_version,
            "local_id": self.local_id,
            "path": self.path,
            "sha256": self.sha256,
            "source_key": self.source_key,
            "tracking": self.tracking.value,
        }


@dataclass(frozen=True)
class SourceFileLock:
    source_id: str
    system: str
    role: SourceRole
    path: str
    sha256: str
    tracking: Tracking
    adapter: str
    declared_count: int | None

    def to_document(self) -> dict[str, object]:
        return {
            "adapter": self.adapter,
            "declared_count": self.declared_count,
            "path": self.path,
            "role": self.role.value,
            "sha256": self.sha256,
            "source_id": self.source_id,
            "system": self.system,
            "tracking": self.tracking.value,
        }


@dataclass(frozen=True)
class AssignmentMention:
    workstream: str
    role: AssignmentRole
    source: SourceRef

    def to_document(self) -> dict[str, object]:
        return {
            "role": self.role.value,
            "source": self.source.to_document(),
            "workstream": self.workstream,
        }


def _mention_key(item: AssignmentMention) -> tuple[str, str, str, str]:
    return (item.role.value, item.workstream, item.source.path, item.source.local_id)


@dataclass(frozen=True)
class Assignment:
    primary: AssignmentMention | None
    related: tuple[AssignmentMention, ...] = ()

    def to_document(self) -> dict[str, object]:
        return {
            "primary": None if self.primary is None else self.primary.to_document(),
            "related": [item.to_document() for item in sorted(self.related, key=_mention_key)],
        }


@dataclass(frozen=True)
class SourceClaims:
    severity: Severity = Severity.UNKNOWN
    activation: ActivationClaim = ActivationClaim.UNKNOWN
    player_frequency: PlayerFrequency = PlayerFrequency.UNKNOWN
    determinism_impact: DeterminismImpact = DeterminismImpact.UNKNOWN

    def to_document(self) -> dict[str, object]:
        return {
            "activation": self.activation.value,
            "determinism_impact": self.determinism_impact.value,
            "player_frequency": self.player_frequency.value,
            "severity": self.severity.value,
        }


@dataclass(frozen=True)
class Relation:
    kind: RelationKind
    target: str
    source: SourceRef

    def to_document(self) -> dict[str, object]:
        return {"kind": self.kind.value, "source": self.source.to_document(), "target": self.target}


@dataclass(frozen=True)
class Disposition:
    source_id: str
    kind: DispositionKind
    targets: tuple[str, ...]
    source: SourceRef

    def to_document(self) -> dict[str, object]:
        return {
            "kind": self.kind.value,
            "source": self.source.to_document(),
            "source_id": self.source_id,
            "targets": sorted(self.targets),
        }


@dataclass(frozen=True)
class Obligation:
    id: str
    system: str
    kind: ObligationKind
    title: str
    source: SourceRef
    source_claims: SourceClaims
    assignment: Assignment
    dependencies: tuple[str, ...] = ()
    relations: tuple[Relation, ...] = ()
    rust_anchors: tuple[str, ...] = ()

    def to_document(self) -> dict[str, object]:
        return {
            "assignment": self.assignment.to_document(),
            "dependencies": sorted(self.dependencies),
            "id": self.id,
            "kind": self.kind.value,
            "relations": [
                item.to_document()
                for item in sorted(self.relations, key=lambda item: (item.kind.value, item.target))
            ],
            "rust_anchors": sorted(self.rust_anchors),
            "schema_version": SCHEMA_VERSION,
            "source": self.source.to_document(),
            "source_claims": self.source_claims.to_document(),
            "system": self.system,
            "title": self.title,
        }


@dataclass(frozen=True)
class ArtifactRef:
    path: str
    sha256: str

    def to_document(self) -> dict[str, object]:
        return {"path": self.path, "sha256": self.sha256}


@dataclass(frozen=True)
class Provenance:
    executable_sha256: str
    tool: str
    tool_version: str
    scenario: str
    mode: str
    activation_proof: str
    inputs: tuple[ArtifactRef, ...] = ()
    reference_runs: tuple[ArtifactRef, ...] = ()

    def to_document(self) -> dict[str, object]:
        key = lambda item: (item.path, item.sha256)
        return {
            "activation_proof": self.activation_proof,
            "executable_sha256": self.executable_sha256,
            "inputs": [item.to_document() for item in sorted(self.inputs, key=key)],
            "mode": self.mode,
            "reference_runs": [item.to_document() for item in sorted(self.reference_runs, key=key)],
            "scenario": self.scenario,
            "tool": self.tool,
            "tool_version": self.tool_version,
        }


@dataclass(frozen=True)
class Coverage:
    mode: CoverageMode
    domain: str

    def to_document(self) -> dict[str, object]:
        return {"domain": self.domain, "mode": self.mode.value}


@dataclass(frozen=True)
class ArtifactHashCheck:
    type: str = "artifact_hash"

    def to_document(self) -> dict[str, object]:
        return {"type": self.type}


@dataclass(frozen=True)
class GitAncestorCheck:
    commit: str
    type: str = "git_ancestor"

    def to_document(self) -> dict[str, object]:
        return {"commit": self.commit, "type": self.type}


@dataclass(frozen=True)
class PathExistsCheck:
    path: str
    type: str = "path_exists"

    def to_document(self) -> dict[str, object]:
        return {"path": self.path, "type": self.type}


@dataclass(frozen=True)
class TestDeclaredCheck:
    path: str
    test_name: str
    commit: str
    type: str = "test_declared"

    def to_document(self) -> dict[str, object]:
        return {
            "commit": self.commit,
            "path": self.path,
            "test_name": self.test_name,
            "type": self.type,
        }


@dataclass(frozen=True)
class BridgeTraceCheck:
    left_trace: ArtifactRef
    right_trace: ArtifactRef
    type: str = "bridge_trace"

    def to_document(self) -> dict[str, object]:
        return {
            "left_trace": self.left_trace.to_document(),
            "right_trace": self.right_trace.to_document(),
            "type": self.type,
        }


EvidenceCheck: TypeAlias = (
    ArtifactHashCheck | GitAncestorCheck | PathExistsCheck | TestDeclaredCheck | BridgeTraceCheck
)


@dataclass(frozen=True)
class EvidenceDeclaration:
    id: str
    obligations: tuple[str, ...]
    kind: EvidenceKind
    artifact: ArtifactRef | None
    provenance: Provenance | None
    coverage: Coverage | None
    check: EvidenceCheck

    def to_document(self) -> dict[str, object]:
        return {
            "artifact": None if self.artifact is None else self.artifact.to_document(),
            "check": self.check.to_document(),
            "coverage": None if self.coverage is None else self.coverage.to_document(),
            "id": self.id,
            "kind": self.kind.value,
            "obligations": sorted(self.obligations),
            "provenance": None if self.provenance is None else self.provenance.to_document(),
            "schema_version": SCHEMA_VERSION,
        }


@dataclass(frozen=True)
class SourceLockDocument:
    source_set: str
    importer: str
    importer_version: int
    corpus_digest: str
    sources: tuple[SourceFileLock, ...]

    def to_document(self) -> dict[str, object]:
        return {
            "corpus_digest": self.corpus_digest,
            "importer": self.importer,
            "importer_version": self.importer_version,
            "schema_version": SCHEMA_VERSION,
            "source_set": self.source_set,
            "sources": [item.to_document() for item in sorted(self.sources, key=lambda item: item.source_id)],
        }


@dataclass(frozen=True)
class ObligationSetDocument:
    source_set: str
    corpus_digest: str
    obligations: tuple[Obligation, ...]
    dispositions: tuple[Disposition, ...]
    diagnostics: tuple[Diagnostic, ...]

    def to_document(self) -> dict[str, object]:
        return {
            "corpus_digest": self.corpus_digest,
            "diagnostics": [item.to_document() for item in sorted(self.diagnostics)],
            "dispositions": [item.to_document() for item in sorted(self.dispositions, key=lambda item: item.source_id)],
            "obligations": [item.to_document() for item in sorted(self.obligations, key=lambda item: item.id)],
            "schema_version": SCHEMA_VERSION,
            "source_set": self.source_set,
        }


@dataclass(frozen=True)
class EvidenceSetDocument:
    source_set: str
    corpus_digest: str
    evidence: tuple[EvidenceDeclaration, ...]

    def to_document(self) -> dict[str, object]:
        return {
            "corpus_digest": self.corpus_digest,
            "evidence": [item.to_document() for item in sorted(self.evidence, key=lambda item: item.id)],
            "schema_version": SCHEMA_VERSION,
            "source_set": self.source_set,
        }


@dataclass(frozen=True)
class WorkspaceFacts:
    source_states: Mapping[str, SourceState]
    implementation_facts: Mapping[str, ImplementationState]
    regression_facts: Mapping[str, RegressionState]
    evidence_facts: Mapping[str, OracleState]


@dataclass(frozen=True)
class ReducedRow:
    obligation: Obligation
    source_state: SourceState
    assignment_state: AssignmentState
    implementation_state: ImplementationState
    regression_state: RegressionState
    oracle_state: OracleState
    parity_verdict: ParityVerdict
    queue_state: QueueState
    diagnostics: tuple[Diagnostic, ...] = ()

    def to_document(self) -> dict[str, object]:
        return {
            "assignment_state": self.assignment_state.value,
            "diagnostics": [item.to_document() for item in sorted(self.diagnostics)],
            "implementation_state": self.implementation_state.value,
            "obligation": self.obligation.to_document(),
            "oracle_state": self.oracle_state.value,
            "parity_verdict": self.parity_verdict.value,
            "queue_state": self.queue_state.value,
            "regression_state": self.regression_state.value,
            "source_state": self.source_state.value,
        }


@dataclass(frozen=True)
class LedgerReport:
    corpus_digest: str
    coverage_state: str
    counts: Mapping[str, object]
    rows: tuple[ReducedRow, ...]
    dispositions: tuple[Disposition, ...]
    diagnostics: tuple[Diagnostic, ...] = ()

    def to_document(self) -> dict[str, object]:
        counts: dict[str, object] = {}
        for axis, values in sorted(self.counts.items()):
            if axis == "total":
                counts[axis] = values
            else:
                assert isinstance(values, Mapping)
                counts[axis] = {key: values[key] for key in sorted(values)}
        return {
            "corpus_digest": self.corpus_digest,
            "counts": counts,
            "coverage_state": self.coverage_state,
            "diagnostics": [item.to_document() for item in sorted(self.diagnostics)],
            "dispositions": [item.to_document() for item in sorted(self.dispositions, key=lambda item: item.source_id)],
            "rows": [item.to_document() for item in self.rows],
            "schema_version": SCHEMA_VERSION,
        }
