"""Conservative independent-axis and next-queue reduction for ledger rows."""

from __future__ import annotations

from dataclasses import dataclass

from .evidence import EvaluatedEvidence
from .model import (
    AssignmentRole,
    AssignmentState,
    ImplementationState,
    Obligation,
    ObligationKind,
    OracleState,
    ParityVerdict,
    QueueState,
    ReducedRow,
    RegressionState,
    SourceState,
)


@dataclass(frozen=True)
class RowFacts:
    source_state: SourceState
    assignment_state: AssignmentState
    implementation_state: ImplementationState
    regression_state: RegressionState
    oracle_state: OracleState
    dependency_blocked: bool = False
    research_required: bool = False
    oracle_attempted: bool = False


def derive_parity(obligation: Obligation, facts: RowFacts) -> ParityVerdict:
    if facts.source_state is SourceState.STALE:
        return ParityVerdict.UNCHECKED
    if facts.implementation_state is ImplementationState.STALE_MAPPING:
        return ParityVerdict.UNCHECKED
    if (
        obligation.kind is ObligationKind.PARITY_GAP
        and facts.implementation_state is not ImplementationState.LANDED
    ):
        return ParityVerdict.DRIFT
    if facts.implementation_state is ImplementationState.LANDED:
        if facts.oracle_state is OracleState.INCOMPLETE and facts.oracle_attempted:
            return ParityVerdict.UNCHECKED
        return ParityVerdict.UNVERIFIED
    return ParityVerdict.UNCHECKED


def derive_queue(
    obligation: Obligation,
    facts: RowFacts,
    parity: ParityVerdict,
) -> QueueState:
    if facts.source_state is SourceState.STALE:
        return QueueState.NEEDS_SOURCE_REFRESH
    if facts.assignment_state is AssignmentState.UNASSIGNED:
        return QueueState.NEEDS_ASSIGNMENT
    if facts.dependency_blocked:
        return QueueState.DEPENDENCY_BLOCKED
    if facts.research_required:
        return QueueState.NEEDS_RESEARCH
    if facts.regression_state is RegressionState.FAIL:
        return QueueState.NEEDS_IMPLEMENTATION
    if facts.implementation_state in {
        ImplementationState.NONE,
        ImplementationState.CANDIDATE,
        ImplementationState.STALE_MAPPING,
    }:
        return QueueState.NEEDS_IMPLEMENTATION
    if parity is ParityVerdict.DRIFT:
        return QueueState.NEEDS_IMPLEMENTATION
    if facts.implementation_state is ImplementationState.LANDED and facts.regression_state is not RegressionState.PASS:
        return QueueState.NEEDS_REGRESSION
    if facts.implementation_state is ImplementationState.LANDED and facts.oracle_state is not OracleState.EXHAUSTIVE:
        return QueueState.NEEDS_ORACLE
    if parity is ParityVerdict.VERIFIED:
        return QueueState.NO_ACTION
    return QueueState.NOT_ACTIONABLE


def reduce_rows(
    obligations: tuple[Obligation, ...],
    source_states: dict[str, SourceState],
    evidence: EvaluatedEvidence,
) -> tuple[ReducedRow, ...]:
    implementation = evidence.implementation
    output: list[ReducedRow] = []
    for obligation in obligations:
        assignment_state = (
            AssignmentState.ASSIGNED
            if obligation.assignment.primary is not None
            else AssignmentState.UNASSIGNED
        )
        dependency_blocked = any(
            implementation.get(dependency, ImplementationState.NONE) is not ImplementationState.LANDED
            for dependency in obligation.dependencies
        )
        research_required = any(
            mention.role is AssignmentRole.RESEARCH_GATE
            for mention in obligation.assignment.related
        )
        oracle_state = evidence.oracle.get(obligation.id, OracleState.NONE)
        facts = RowFacts(
            source_states[obligation.source.source_key],
            assignment_state,
            implementation.get(obligation.id, ImplementationState.NONE),
            evidence.regression.get(obligation.id, RegressionState.NONE),
            oracle_state,
            dependency_blocked,
            research_required,
            oracle_state is not OracleState.NONE,
        )
        parity = derive_parity(obligation, facts)
        queue = derive_queue(obligation, facts, parity)
        row_diagnostics = tuple(
            item for item in evidence.diagnostics if item.record_id == obligation.id
        )
        output.append(
            ReducedRow(
                obligation,
                facts.source_state,
                facts.assignment_state,
                facts.implementation_state,
                facts.regression_state,
                facts.oracle_state,
                parity,
                queue,
                row_diagnostics,
            )
        )
    return tuple(sorted(output, key=lambda item: item.obligation.id))
