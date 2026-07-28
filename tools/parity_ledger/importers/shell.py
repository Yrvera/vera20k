"""Bounded importer for shell/UI gaps, dispositions, and roadmap ownership."""

from __future__ import annotations

from dataclasses import replace
import re

from .. import IMPORTER_VERSION
from ..errors import Diagnostic, FailureCode
from ..model import (
    Assignment,
    AssignmentMention,
    AssignmentRole,
    Disposition,
    DispositionKind,
    Obligation,
    ObligationKind,
    Severity,
    SourceClaims,
    SourceRef,
)
from ..source_sets import SourceConfig
from .common import (
    bounded_section,
    ensure_exact_ids,
    exact_id_set,
    expand_ids,
    extract_rust_paths,
    fold_markdown,
    id_sort_key,
    malformed,
    strict_text,
)


_RECORD_RE = re.compile(r"^\*\*(?P<id>[HML][1-9]\d*)\.(?P<tail>.*)$")
_WS_RE = re.compile(r"^### (?P<id>WS-(?:[1-9]|1[0-3]))\b", re.MULTILINE)
_QW_RE = re.compile(r"^- \*\*(?P<id>QW-[1-9])\s+·\s+(?P<targets>[^*]+)\*\*", re.MULTILINE)


def _source(config: SourceConfig, digest: str, local_id: str) -> SourceRef:
    return SourceRef(
        config.path,
        local_id,
        config.source_id,
        digest,
        config.tracking,
        config.adapter,
        IMPORTER_VERSION,
    )


def _title(block: list[str], match: re.Match[str]) -> str:
    joined = "\n".join([match.group("tail"), *block[1:]])
    close = joined.find("**")
    if close < 0:
        malformed(f"unterminated bold title for {match.group('id')}")
    title = fold_markdown(joined[:close])
    if not title:
        malformed(f"empty title for {match.group('id')}")
    return title


def _region_records(
    section: str,
    config: SourceConfig,
    digest: str,
    severity: Severity,
) -> list[Obligation]:
    lines = section.splitlines()
    starts: list[tuple[int, re.Match[str]]] = []
    for index, line in enumerate(lines):
        match = _RECORD_RE.match(line)
        if match:
            starts.append((index, match))
    output: list[Obligation] = []
    for position, (start, match) in enumerate(starts):
        end = starts[position + 1][0] if position + 1 < len(starts) else len(lines)
        block = lines[start:end]
        local_id = match.group("id")
        output.append(
            Obligation(
                f"shell:{local_id}",
                "shell",
                ObligationKind.PARITY_GAP,
                _title(block, match),
                _source(config, digest, local_id),
                SourceClaims(severity=severity),
                Assignment(None),
                rust_anchors=extract_rust_paths("\n".join(block)),
            )
        )
    return output


def _dispositions(
    text: str,
    config: SourceConfig,
    digest: str,
    active_ids: set[str],
) -> tuple[Disposition, ...]:
    summary = fold_markdown(bounded_section(text, "Summary", "ENGINE-SERVICE INVENTORY (framing 1)"))
    statement = re.compile(
        r"L1\.\.L34, with L3/L25 merged upward into M13/L2 and L24 respectively, "
        r"and L28 reclassified as a proven non-gap",
    )
    if not statement.search(summary):
        malformed("shell disposition statement changed or is missing", source_path=config.path)
    values = (
        Disposition("shell:L3", DispositionKind.MERGED, ("shell:L2", "shell:M13"), _source(config, digest, "L3")),
        Disposition("shell:L25", DispositionKind.MERGED, ("shell:L24",), _source(config, digest, "L25")),
        Disposition("shell:L28", DispositionKind.RETIRED_NON_GAP, (), _source(config, digest, "L28")),
    )
    for disposition in values:
        if disposition.source_id in active_ids:
            malformed(f"disposition {disposition.source_id} is also active", source_path=config.path)
        missing_targets = set(disposition.targets) - active_ids
        if missing_targets:
            malformed(
                f"disposition {disposition.source_id} targets missing obligations {sorted(missing_targets)}",
                source_path=config.path,
            )
    return values


def import_shell_findings(
    raw: bytes,
    config: SourceConfig,
) -> tuple[tuple[Obligation, ...], tuple[Disposition, ...], tuple[Diagnostic, ...]]:
    text, digest = strict_text(raw, source_path=config.path)
    regions = (
        ("Confirmed gaps — HIGH severity", "Confirmed gaps — MEDIUM severity", Severity.HIGH, "H", exact_id_set("H", 1, 19)),
        ("Confirmed gaps — MEDIUM severity", "Confirmed gaps — LOW severity", Severity.MEDIUM, "M", exact_id_set("M", 1, 39)),
        ("Confirmed gaps — LOW severity", "Needs-verification queue (NV1..)", Severity.LOW, "L", exact_id_set("L", 1, 34) - {"L3", "L25", "L28"}),
    )
    rows: list[Obligation] = []
    for start, end, severity, family, expected_ids in regions:
        region_rows = _region_records(bounded_section(text, start, end), config, digest, severity)
        ensure_exact_ids(
            (item.id.split(":", 1)[1] for item in region_rows),
            expected_ids,
            label=f"shell {family} region",
        )
        rows.extend(region_rows)
    local_ids = {item.id.split(":", 1)[1] for item in rows}
    expected_low = exact_id_set("L", 1, 34) - {"L3", "L25", "L28"}
    ensure_exact_ids((item for item in local_ids if item.startswith("H")), exact_id_set("H", 1, 19), label="shell HIGH")
    ensure_exact_ids((item for item in local_ids if item.startswith("M")), exact_id_set("M", 1, 39), label="shell MEDIUM")
    ensure_exact_ids((item for item in local_ids if item.startswith("L")), expected_low, label="shell LOW")
    if len(rows) != 89:
        malformed(f"shell finding total expected 89, found {len(rows)}", source_path=config.path)
    active_ids = {item.id for item in rows}
    dispositions = _dispositions(text, config, digest, active_ids)
    note = "A further ~15 CONFIRMED LOW items are folded into the per-service inventory"
    folded_detail = "Additional LOW items folded into services above and not separately numbered"
    normalized_text = fold_markdown(text)
    if note not in normalized_text or folded_detail not in normalized_text:
        malformed("shell unnumbered-confirmed-items note changed or is missing", source_path=config.path)
    diagnostics = [
        Diagnostic(
            FailureCode.UNNUMBERED_CONFIRMED_ITEMS.value,
            source_path=config.path,
            field="confirmed_low",
            message="approximately 15 confirmed LOW items remain folded into service prose rather than numbered obligations",
            fatal=False,
        )
    ]
    declared_match = re.search(r"(?P<count>\d+) unique confirmed gaps", normalized_text)
    if declared_match is None:
        malformed("shell declared-count claim is missing", source_path=config.path)
    declared = int(declared_match.group("count"))
    if declared != len(rows):
        diagnostics.append(
            Diagnostic(
                FailureCode.DECLARED_COUNT_MISMATCH.value,
                source_path=config.path,
                field="declared_count",
                message=f"scan declares {declared} findings; enumerated source has {len(rows)}",
                fatal=False,
            )
        )
    return tuple(sorted(rows, key=lambda item: item.id)), dispositions, tuple(sorted(diagnostics))


_OWNER_OVERRIDES = {
    "H5": "WS-3",
    "L4": "WS-4",
    "L10": "WS-10",
    "L14": "WS-1",
    "M16": "WS-4",
    "M21": "WS-10",
    "M25": "WS-6",
    "M26": "WS-10",
    "M27": "WS-6",
}

_EXPECTED_QW = {
    "QW-1": {"M13", "L2"},
    "QW-2": {"M3"},
    "QW-3": {"M23"},
    "QW-4": {"L18"},
    "QW-5": {"L19"},
    "QW-6": {"M31"},
    "QW-7": {"L8"},
    "QW-8": {"M2"},
    "QW-9": {"L20"},
}

_EXPECTED_RESEARCH = {
    "H7": "NV4",
    "M4": "NV6",
    "H18": "NV7",
    "M7": "NV8",
    "H9": "NV50-NV51",
    "M29": "NV56",
    "H15": "NV1",
    "M22": "NV22",
    "L1": "L1-per-consumer",
}
_EXPECTED_DEFERRED = {"H10", "M29", "L9", "M1", "L27", "L1"}


def _scope_candidates(
    text: str,
    active: set[str],
    roadmap_path: str,
) -> tuple[dict[str, set[str]], tuple[Diagnostic, ...]]:
    starts = list(_WS_RE.finditer(text))
    quick_start = text.find("## Quick wins")
    expected_order = [f"WS-{number}" for number in range(1, 14)]
    actual_order = [match.group("id") for match in starts]
    if actual_order != expected_order or quick_start < 0:
        malformed("shell roadmap must contain WS-1..WS-13 followed by Quick wins")
    candidates: dict[str, set[str]] = {}
    diagnostics: list[Diagnostic] = []
    for index, start in enumerate(starts):
        end = starts[index + 1].start() if index + 1 < len(starts) else quick_start
        section = text[start.start():end]
        scope = re.search(
            r"^- \*\*Scope \(closes\):\*\*(.*?)(?=^- \*\*|^### |\Z)",
            section,
            re.MULTILINE | re.DOTALL,
        )
        if scope is None:
            malformed(f"{start.group('id')} has no bounded Scope (closes) paragraph")
        scope_text = scope.group(1)
        stale_m35 = re.search(r"M35\s*\([^)]*folded in M28[^)]*\)", scope_text, re.IGNORECASE)
        if start.group("id") == "WS-5":
            if stale_m35 is None:
                malformed("WS-5 stale M35 folded-in-M28 qualifier changed or is missing")
            scope_text = scope_text[: stale_m35.start()] + scope_text[stale_m35.end() :]
            diagnostics.append(
                Diagnostic(
                    FailureCode.STALE_ROADMAP_REFERENCE.value,
                    source_path=roadmap_path,
                    record_id="shell:M35",
                    field="WS-5",
                    message="WS-5's M35 label is stale; active M35 is owned by WS-8",
                    fatal=False,
                )
            )
        identifiers = {item for item in expand_ids(scope_text) if item[:1] in {"H", "M", "L"}}
        unknown = {f"shell:{item}" for item in identifiers} - active
        if unknown:
            malformed(f"{start.group('id')} references non-active shell IDs {sorted(unknown)}")
        for local_id in identifiers:
            candidates.setdefault(local_id, set()).add(start.group("id"))
    return candidates, tuple(sorted(diagnostics))


def _quick_wins(text: str) -> dict[str, str]:
    quick = bounded_section(
        text,
        "Quick wins (trivial, existing test seam — MAY skip /brainstorm; still need a named test)",
        "Research-first queue (resolve the named binary question BEFORE implementing the surface)",
    )
    matches = list(_QW_RE.finditer(quick))
    actual_order = [match.group("id") for match in matches]
    if actual_order != list(_EXPECTED_QW):
        malformed(f"shell quick-win headings changed or duplicate: {actual_order!r}")
    parsed: dict[str, set[str]] = {}
    for match in matches:
        parsed[match.group("id")] = {item for item in expand_ids(match.group("targets")) if item[:1] in {"H", "M", "L"}}
    if parsed != _EXPECTED_QW:
        malformed(f"shell quick-win mapping changed: {parsed!r}")
    return {local_id: quick_win for quick_win, identifiers in parsed.items() for local_id in identifiers}


def _research_gates(text: str) -> dict[str, str]:
    section = bounded_section(
        text,
        "Research-first queue (resolve the named binary question BEFORE implementing the surface)",
        "Suggested order (dependency-aware)",
    )
    result: dict[str, str] = {}
    for line in section.splitlines():
        if not line.startswith("|") or line.startswith(("|---", "| NV |")):
            continue
        columns = [column.strip() for column in line.strip("|").split("|")]
        if not columns:
            continue
        gate = re.sub(r"[^A-Za-z0-9]+", "-", columns[0]).strip("-")
        for local_id in expand_ids(line):
            if local_id[:1] in {"H", "M", "L"}:
                if local_id in result:
                    malformed(f"duplicate research gate for {local_id}")
                result[local_id] = gate
    if result != _EXPECTED_RESEARCH:
        malformed(f"shell research-gate mapping changed: {result!r}")
    return result


def _deferred_ids(text: str) -> set[str]:
    match = re.search(
        r"^\*\*Deferred \(blocked, not scheduled\):\*\*(.*?)(?:\n\s*\n|\n\*\*TS-legacy)",
        text,
        re.MULTILINE | re.DOTALL,
    )
    if match is None:
        malformed("shell deferred paragraph missing")
    identifiers = {item for item in expand_ids(match.group(1)) if item[:1] in {"H", "M", "L"}}
    if identifiers != _EXPECTED_DEFERRED:
        malformed(f"shell deferred set changed: {sorted(identifiers)}")
    return identifiers


def apply_shell_assignments(
    findings: tuple[Obligation, ...],
    roadmap_raw: bytes,
    roadmap_config: SourceConfig,
) -> tuple[tuple[Obligation, ...], tuple[Diagnostic, ...]]:
    text, digest = strict_text(roadmap_raw, source_path=roadmap_config.path)
    active = {item.id for item in findings}
    scopes, scope_diagnostics = _scope_candidates(text, active, roadmap_config.path)
    quick_wins = _quick_wins(text)
    research = _research_gates(text)
    deferred = _deferred_ids(text)

    def mention(workstream: str, role: AssignmentRole, local_id: str) -> AssignmentMention:
        return AssignmentMention(
            f"shell:{workstream}",
            role,
            _source(roadmap_config, digest, local_id),
        )

    output: list[Obligation] = []
    for finding in findings:
        local_id = finding.id.split(":", 1)[1]
        workstreams = scopes.get(local_id, set())
        related: dict[tuple[str, str], AssignmentMention] = {}
        if local_id in quick_wins:
            chosen = quick_wins[local_id]
            primary = mention(chosen, AssignmentRole.PRIMARY, chosen)
            quick = mention(chosen, AssignmentRole.QUICK_WIN, chosen)
            related[(quick.workstream, quick.role.value)] = quick
            for workstream in workstreams:
                parent = mention(workstream, AssignmentRole.PARENT, workstream)
                related[(parent.workstream, parent.role.value)] = parent
        elif len(workstreams) == 0:
            primary = None
        elif len(workstreams) == 1:
            chosen = next(iter(workstreams))
            primary = mention(chosen, AssignmentRole.PRIMARY, chosen)
        else:
            chosen = _OWNER_OVERRIDES.get(local_id)
            if chosen is None or chosen not in workstreams:
                malformed(f"ambiguous shell primary for {local_id}: {sorted(workstreams)}")
            primary = mention(chosen, AssignmentRole.PRIMARY, chosen)
            for workstream in workstreams - {chosen}:
                parent = mention(workstream, AssignmentRole.PARENT, workstream)
                related[(parent.workstream, parent.role.value)] = parent
        if local_id in research:
            item = mention(research[local_id], AssignmentRole.RESEARCH_GATE, research[local_id])
            related[(item.workstream, item.role.value)] = item
        if local_id in deferred:
            item = mention("DEFERRED", AssignmentRole.DEFERRED, "DEFERRED")
            related[(item.workstream, item.role.value)] = item
        output.append(replace(finding, assignment=Assignment(primary, tuple(related.values()))))

    unassigned = {item.id for item in output if item.assignment.primary is None}
    if unassigned != {"shell:H1", "shell:H19"}:
        malformed(f"shell unassigned set changed: {sorted(unassigned)}", source_path=roadmap_config.path)
    by_id = {item.id: item for item in output}
    if by_id["shell:M35"].assignment.primary.workstream != "shell:WS-8":
        malformed("shell:M35 must remain owned by shell:WS-8", source_path=roadmap_config.path)
    if any(
        mention.workstream == "shell:WS-5"
        for mention in by_id["shell:M35"].assignment.related
    ):
        malformed("shell:M35 must not inherit the stale WS-5 label", source_path=roadmap_config.path)
    h5 = by_id["shell:H5"].assignment
    if h5.primary is None or h5.primary.workstream != "shell:WS-3" or any(
        mention.workstream == "shell:WS-9" for mention in h5.related
    ):
        malformed("shell:H5 scope must remain solely WS-3", source_path=roadmap_config.path)
    for quick_win, identifiers in _EXPECTED_QW.items():
        for local_id in identifiers:
            if by_id[f"shell:{local_id}"].assignment.primary.workstream != f"shell:{quick_win}":
                malformed(f"shell:{local_id} must be owned by shell:{quick_win}")
    return tuple(sorted(output, key=lambda item: item.id)), scope_diagnostics


def import_shell(
    scan_raw: bytes,
    roadmap_raw: bytes,
    scan_config: SourceConfig,
    roadmap_config: SourceConfig,
) -> tuple[tuple[Obligation, ...], tuple[Disposition, ...], tuple[Diagnostic, ...]]:
    findings, dispositions, diagnostics = import_shell_findings(scan_raw, scan_config)
    assigned, assignment_diagnostics = apply_shell_assignments(findings, roadmap_raw, roadmap_config)
    return assigned, dispositions, tuple(sorted((*diagnostics, *assignment_diagnostics)))
