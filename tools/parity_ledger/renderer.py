"""Deterministic machine report and human Markdown projection."""

from __future__ import annotations

from collections import Counter
from pathlib import Path

from .corpus import Corpus
from .errors import Diagnostic, FailureCode
from .evidence import evaluate_evidence
from .graph import validate_graph
from .jsonio import canonical_json_bytes, stage_atomic_bytes
from .model import (
    AssignmentRole,
    AssignmentState,
    ImplementationState,
    LedgerReport,
    OracleState,
    ParityVerdict,
    PlayerFrequency,
    QueueState,
    RegressionState,
    SourceState,
)
from .reducer import reduce_rows
from .schema import decode_ledger
from .workspace import source_states


COVERAGE_STATE = "BOOTSTRAP_PROVISIONAL"

_QUEUE_RANK = {
    QueueState.NEEDS_ASSIGNMENT: 0,
    QueueState.NEEDS_SOURCE_REFRESH: 1,
    QueueState.DEPENDENCY_BLOCKED: 2,
    QueueState.NEEDS_RESEARCH: 3,
    QueueState.NEEDS_IMPLEMENTATION: 4,
    QueueState.NEEDS_REGRESSION: 5,
    QueueState.NEEDS_ORACLE: 6,
    QueueState.NOT_ACTIONABLE: 7,
    QueueState.NO_ACTION: 8,
}
_FREQUENCY_RANK = {
    PlayerFrequency.HIGH.value: 0,
    PlayerFrequency.MEDIUM.value: 1,
    PlayerFrequency.LOW.value: 2,
    PlayerFrequency.UNKNOWN.value: 3,
}
_DETERMINISM_RANK = {"sim_critical": 0, "output_only": 1, "unknown": 2}


def _all_counts(enum_type, values) -> dict[str, int]:
    counts = Counter(value.value for value in values)
    return {item.value: counts[item.value] for item in enum_type}


def _counts(rows) -> dict[str, object]:
    return {
        "assignment_state": _all_counts(AssignmentState, (item.assignment_state for item in rows)),
        "implementation_state": _all_counts(ImplementationState, (item.implementation_state for item in rows)),
        "oracle_state": _all_counts(OracleState, (item.oracle_state for item in rows)),
        "parity_verdict": _all_counts(ParityVerdict, (item.parity_verdict for item in rows)),
        "queue_state": _all_counts(QueueState, (item.queue_state for item in rows)),
        "regression_state": _all_counts(RegressionState, (item.regression_state for item in rows)),
        "source_state": _all_counts(SourceState, (item.source_state for item in rows)),
        "system": dict(sorted(Counter(item.obligation.system for item in rows).items())),
        "total": len(rows),
    }


def _row_sort_key(row) -> tuple[object, ...]:
    research = any(
        mention.role is AssignmentRole.RESEARCH_GATE
        for mention in row.obligation.assignment.related
    )
    return (
        _QUEUE_RANK[row.queue_state],
        _FREQUENCY_RANK[row.obligation.source_claims.player_frequency.value],
        _DETERMINISM_RANK[row.obligation.source_claims.determinism_impact.value],
        0 if research else 1,
        row.obligation.id,
    )


def build_report(repo: Path, corpus: Corpus, *, source_mode: str = "default") -> LedgerReport:
    validate_graph(corpus.obligation_set.obligations, corpus.obligation_set.dispositions)
    states = source_states(repo, corpus.source_lock, mode=source_mode)
    evaluated = evaluate_evidence(
        repo,
        corpus.evidence_set.evidence,
        corpus.obligation_set.obligations,
    )
    rows = reduce_rows(corpus.obligation_set.obligations, states, evaluated)
    ordered = tuple(sorted(rows, key=_row_sort_key))
    diagnostics = list(corpus.obligation_set.diagnostics)
    diagnostics.extend(evaluated.diagnostics)
    locks = {item.source_id: item for item in corpus.source_lock.sources}
    for source_id, state in sorted(states.items()):
        if state is SourceState.CURRENT:
            continue
        code = FailureCode.SOURCE_STALE if state is SourceState.STALE else FailureCode.SOURCE_UNAVAILABLE
        diagnostics.append(
            Diagnostic(
                code.value,
                source_path=locks[source_id].path,
                record_id=source_id,
                message=f"source is {state.value}",
                fatal=False,
            )
        )
    report = LedgerReport(
        corpus.source_lock.corpus_digest,
        COVERAGE_STATE,
        _counts(ordered),
        ordered,
        corpus.obligation_set.dispositions,
        tuple(sorted(set(diagnostics))),
    )
    decode_ledger(report.to_document())
    return report


def render_json(report: LedgerReport) -> bytes:
    return canonical_json_bytes(report.to_document())


def _id_lines(identifiers: list[str]) -> list[str]:
    return [f"- `{identifier}`" for identifier in sorted(identifiers)] or ["- None"]


def render_markdown(report: LedgerReport) -> bytes:
    rows = report.rows
    lines = [
        "# VERA20k Parity Ledger",
        "",
        "## Coverage State",
        "",
        f"`{report.coverage_state}`",
        "",
        "This bootstrap is a provisional obligation inventory, not a certified completion percentage.",
        "Declaration-only evidence cannot produce `PASS`, `FAIL`, `EXHAUSTIVE`, or `VERIFIED`.",
        "",
        "## Inventory Counts",
        "",
        f"- Active obligations: {len(rows)}",
    ]
    system_counts = report.counts["system"]
    assert isinstance(system_counts, dict)
    lines.extend(f"- {system}: {system_counts[system]}" for system in sorted(system_counts))
    lines.extend(["", "## Unassigned Obligations", ""])
    lines.extend(_id_lines([row.obligation.id for row in rows if row.assignment_state is AssignmentState.UNASSIGNED]))
    lines.extend(["", "## Stale Sources and Mappings", ""])
    stale = [
        row.obligation.id
        for row in rows
        if row.source_state is SourceState.STALE or row.implementation_state is ImplementationState.STALE_MAPPING
    ]
    lines.extend(_id_lines(stale))
    lines.extend(["", "## Parity Verdicts", ""])
    parity_counts = report.counts["parity_verdict"]
    assert isinstance(parity_counts, dict)
    lines.extend(f"- {verdict}: {parity_counts[verdict]}" for verdict in sorted(parity_counts))
    lines.extend(["", "## Next Queue", ""])
    queue_counts = report.counts["queue_state"]
    assert isinstance(queue_counts, dict)
    for queue in QueueState:
        identifiers = [row.obligation.id for row in rows if row.queue_state is queue]
        lines.append(f"### {queue.value} ({queue_counts[queue.value]})")
        lines.append("")
        lines.extend(_id_lines(identifiers))
        lines.append("")
    lines.extend(["## Source Diagnostics", ""])
    if report.diagnostics:
        for diagnostic in sorted(report.diagnostics):
            location = diagnostic.record_id or diagnostic.source_path or "corpus"
            lines.append(f"- `{diagnostic.code}` `{location}` — {diagnostic.message}")
    else:
        lines.append("- None")
    lines.extend(["", "## Dispositions", ""])
    if report.dispositions:
        for disposition in sorted(report.dispositions, key=lambda item: item.source_id):
            targets = ", ".join(f"`{target}`" for target in disposition.targets) or "none"
            lines.append(f"- `{disposition.source_id}`: {disposition.kind.value}; targets: {targets}")
    else:
        lines.append("- None")
    return ("\n".join(lines).rstrip() + "\n").encode("utf-8")


def write_report(output: Path, report: LedgerReport) -> None:
    staged = []
    try:
        for path, payload in (
            (output / "ledger.json", render_json(report)),
            (output / "summary.md", render_markdown(report)),
        ):
            staged.append(stage_atomic_bytes(path, payload))
        for item in staged:
            item.commit()
    finally:
        for item in staged:
            item.cleanup()
