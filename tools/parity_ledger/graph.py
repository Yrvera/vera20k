"""Deterministic dependency and relation validation for active obligations."""

from __future__ import annotations

from .errors import Diagnostic, ExitCode, FailureCode, LedgerError
from .model import Disposition, Obligation


def _canonical_cycle(cycle: list[str]) -> tuple[str, ...]:
    body = cycle[:-1]
    smallest = min(range(len(body)), key=lambda index: body[index])
    rotated = body[smallest:] + body[:smallest]
    return tuple((*rotated, rotated[0]))


def validate_graph(
    obligations: tuple[Obligation, ...],
    dispositions: tuple[Disposition, ...],
) -> None:
    active = {item.id: item for item in obligations}
    disposition_ids = {item.source_id for item in dispositions}
    diagnostics: list[Diagnostic] = []
    collisions = sorted(set(active) & disposition_ids)
    for identifier in collisions:
        diagnostics.append(
            Diagnostic(
                FailureCode.DUPLICATE_OBLIGATION.value,
                record_id=identifier,
                message="ID is both active and disposed",
                fatal=True,
            )
        )
    for obligation in obligations:
        seen_relations: set[tuple[str, str]] = set()
        for dependency in obligation.dependencies:
            if dependency == obligation.id or dependency not in active:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.UNRESOLVED_DEPENDENCY.value,
                        record_id=obligation.id,
                        field="dependencies",
                        message=f"invalid dependency {dependency}",
                        fatal=True,
                    )
                )
        for relation in obligation.relations:
            edge = (relation.kind.value, relation.target)
            if edge in seen_relations:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.UNRESOLVED_RELATION.value,
                        record_id=obligation.id,
                        field="relations",
                        message=f"duplicate relation {edge}",
                        fatal=True,
                    )
                )
            seen_relations.add(edge)
            if relation.target not in active and relation.target not in disposition_ids:
                diagnostics.append(
                    Diagnostic(
                        FailureCode.UNRESOLVED_RELATION.value,
                        record_id=obligation.id,
                        field="relations",
                        message=f"missing relation target {relation.target}",
                        fatal=True,
                    )
                )
    if diagnostics:
        raise LedgerError(ExitCode.VALIDATION_FAILED, diagnostics)

    colors: dict[str, int] = {identifier: 0 for identifier in active}
    stack: list[str] = []

    def visit(identifier: str) -> tuple[str, ...] | None:
        colors[identifier] = 1
        stack.append(identifier)
        for dependency in sorted(active[identifier].dependencies):
            if colors[dependency] == 0:
                cycle = visit(dependency)
                if cycle is not None:
                    return cycle
            elif colors[dependency] == 1:
                start = stack.index(dependency)
                return _canonical_cycle([*stack[start:], dependency])
        stack.pop()
        colors[identifier] = 2
        return None

    for identifier in sorted(active):
        if colors[identifier] == 0:
            cycle = visit(identifier)
            if cycle is not None:
                raise LedgerError(
                    ExitCode.VALIDATION_FAILED,
                    [
                        Diagnostic(
                            FailureCode.DEPENDENCY_CYCLE.value,
                            record_id=cycle[0],
                            field="dependencies",
                            message=" -> ".join(cycle),
                            fatal=True,
                        )
                    ],
                )
