"""Bounded importer for miner findings and hierarchical roadmap assignments."""

from __future__ import annotations

from dataclasses import replace
import re

from .. import IMPORTER_VERSION
from ..errors import Diagnostic, FailureCode
from ..model import (
    Assignment,
    AssignmentMention,
    AssignmentRole,
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


_RECORD_RE = re.compile(r"^(?:- )?\*\*(?P<id>[GMLS][1-9]\d*)\.(?P<tail>.*)$")
_WORKSTREAM_RE = re.compile(r"^## (?P<id>W(?:[0-9]|1[0-3]))\b")
_FALSE_S5_RE = re.compile(r"object-AI S5 slice", re.IGNORECASE)


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


def _title_from_block(block: list[str], match: re.Match[str]) -> str:
    joined = "\n".join((match.group("tail"), *block[1:]))
    bold = joined.find("**")
    if bold >= 0 and joined[:bold].strip():
        return fold_markdown(joined[:bold]).rstrip()
    remainder = joined[bold + 2 :] if bold >= 0 else joined
    paragraph = re.split(r"\n\s*\n", remainder, maxsplit=1)[0]
    title = fold_markdown(paragraph)
    if not title:
        malformed(f"empty title for {match.group('id')}")
    return title


def _records(
    section: str,
    config: SourceConfig,
    digest: str,
    default_severity: Severity | None,
) -> list[Obligation]:
    lines = section.splitlines()
    starts: list[tuple[int, re.Match[str], Severity]] = []
    current_severity = default_severity
    for index, line in enumerate(lines):
        normalized = line.strip().rstrip(":").casefold()
        if default_severity is None and normalized in {"high", "medium", "low (slave/anim details, mostly mod-visible or stock-inert)"}:
            current_severity = {
                "high": Severity.HIGH,
                "medium": Severity.MEDIUM,
                "low (slave/anim details, mostly mod-visible or stock-inert)": Severity.LOW,
            }[normalized]
        match = _RECORD_RE.match(line)
        if match:
            if current_severity is None:
                malformed(f"severity not established for {match.group('id')}", source_path=config.path)
            starts.append((index, match, current_severity))
    output: list[Obligation] = []
    for position, (start, match, severity) in enumerate(starts):
        end = starts[position + 1][0] if position + 1 < len(starts) else len(lines)
        block = lines[start:end]
        local_id = match.group("id")
        output.append(
            Obligation(
                f"miner:{local_id}",
                "miner",
                ObligationKind.PARITY_GAP,
                _title_from_block(block, match),
                _source(config, digest, local_id),
                SourceClaims(severity=severity),
                Assignment(None),
                rust_anchors=extract_rust_paths("\n".join(block)),
            )
        )
    return output


def import_miner_findings(raw: bytes, config: SourceConfig) -> tuple[Obligation, ...]:
    text, digest = strict_text(raw, source_path=config.path)
    regions = (
        ("Confirmed gaps — HIGH severity (visible in normal play within seconds)", "Confirmed gaps — MEDIUM severity (specific situations / attentive players)", Severity.HIGH, "G", 19),
        ("Confirmed gaps — MEDIUM severity (specific situations / attentive players)", "Confirmed gaps — LOW severity (rare/boundary — still real; ranked for fix order, not parity)", Severity.MEDIUM, "M", 33),
        ("Confirmed gaps — LOW severity (rare/boundary — still real; ranked for fix order, not parity)", "Slave miner & OREGATH additions (slave-war-render lane — 33 confirmed, 6 needs-verification)", Severity.LOW, "L", 67),
        ("Slave miner & OREGATH additions (slave-war-render lane — 33 confirmed, 6 needs-verification)", "Needs verification (Rust state verified; gamemd side needs Ghidra or runtime capture)", None, "S", 20),
    )
    rows: list[Obligation] = []
    for start, end, severity, family, count in regions:
        region_rows = _records(bounded_section(text, start, end), config, digest, severity)
        ensure_exact_ids(
            (item.id.split(":", 1)[1] for item in region_rows),
            exact_id_set(family, 1, count),
            label=f"miner {family} region",
        )
        rows.extend(region_rows)
    by_family: dict[str, set[str]] = {family: set() for family in "GMLS"}
    for row in rows:
        local_id = row.id.split(":", 1)[1]
        by_family[local_id[0]].add(local_id)
    ensure_exact_ids(by_family["G"], exact_id_set("G", 1, 19), label="miner HIGH")
    ensure_exact_ids(by_family["M"], exact_id_set("M", 1, 33), label="miner MEDIUM")
    ensure_exact_ids(by_family["L"], exact_id_set("L", 1, 67), label="miner LOW")
    ensure_exact_ids(by_family["S"], exact_id_set("S", 1, 20), label="miner slave")
    if len(rows) != 139:
        malformed(f"miner finding total expected 139, found {len(rows)}", source_path=config.path)
    return tuple(sorted(rows, key=lambda item: (item.id.split(":")[1][0], id_sort_key(item.id.split(":")[1]))))


def _roadmap_sections(text: str) -> dict[str, str]:
    approved = bounded_section(text, "W0 — Quick wins (no research gate, independent, do first)", "Suggested sequence")
    lines = approved.splitlines()
    starts: list[tuple[int, str]] = [(0, "W0")]
    for index, line in enumerate(lines):
        match = _WORKSTREAM_RE.match(line)
        if match and match.group("id") != "W0":
            starts.append((index, match.group("id")))
        elif line.startswith("## Deferred "):
            starts.append((index, "DEFERRED"))
    starts.sort()
    expected_order = [*(f"W{number}" for number in range(14)), "DEFERRED"]
    actual_order = [name for _start, name in starts]
    if actual_order != expected_order:
        malformed(f"miner roadmap workstream order differs: {actual_order!r}")
    sections: dict[str, str] = {}
    for position, (start, name) in enumerate(starts):
        end = starts[position + 1][0] if position + 1 < len(starts) else len(lines)
        sections[name] = "\n".join(lines[start:end])
    expected = {f"W{number}" for number in range(14)} | {"DEFERRED"}
    if set(sections) != expected:
        malformed(f"miner roadmap workstreams differ; missing={sorted(expected - set(sections))}")
    return sections


def apply_miner_assignments(
    findings: tuple[Obligation, ...],
    roadmap_raw: bytes,
    roadmap_config: SourceConfig,
) -> tuple[tuple[Obligation, ...], tuple[Diagnostic, ...]]:
    text, digest = strict_text(roadmap_raw, source_path=roadmap_config.path)
    sections = _roadmap_sections(text)
    primary: dict[str, dict[str, AssignmentMention]] = {}
    related: dict[str, dict[tuple[str, str], AssignmentMention]] = {}

    def mention(local_id: str, workstream: str, role: AssignmentRole, source_local: str) -> AssignmentMention:
        return AssignmentMention(
            f"miner:{workstream}",
            role,
            _source(roadmap_config, digest, source_local),
        )

    def add_primary(local_id: str, workstream: str, source_local: str) -> None:
        item = mention(local_id, workstream, AssignmentRole.PRIMARY, source_local)
        primary.setdefault(local_id, {})[item.workstream] = item

    def add_related(local_id: str, workstream: str, role: AssignmentRole, source_local: str) -> None:
        item = mention(local_id, workstream, role, source_local)
        related.setdefault(local_id, {})[(item.workstream, item.role.value)] = item

    for workstream, body in sections.items():
        sanitized = _FALSE_S5_RE.sub("object-AI scheduler slice", body)
        identifiers = expand_ids(sanitized)
        for local_id in identifiers:
            if workstream == "W12":
                add_related(local_id, workstream, AssignmentRole.RESEARCH_GATE, workstream)
            elif workstream == "DEFERRED":
                add_primary(local_id, workstream, workstream)
                add_related(local_id, workstream, AssignmentRole.DEFERRED, workstream)
            elif workstream == "W1" and local_id in {"M1", "M5"}:
                add_related(local_id, workstream, AssignmentRole.HISTORICAL_PARTIAL, workstream)
            elif workstream == "W1" and local_id == "S12":
                add_primary(local_id, "W11", workstream)
                add_related(local_id, "W11", AssignmentRole.DEFERRED, workstream)
            else:
                add_primary(local_id, workstream, workstream)

    output: list[Obligation] = []
    known_ids = {item.id.split(":", 1)[1] for item in findings}
    for local_id in set(primary) | set(related):
        if local_id not in known_ids:
            malformed(f"miner roadmap references unknown finding {local_id}", source_path=roadmap_config.path)
    for finding in findings:
        local_id = finding.id.split(":", 1)[1]
        candidates = primary.get(local_id, {})
        if len(candidates) > 1:
            malformed(
                f"multiple primary assignments for {finding.id}: {sorted(candidates)}",
                source_path=roadmap_config.path,
                record_id=finding.id,
            )
        primary_item = next(iter(candidates.values()), None)
        related_items = tuple(related.get(local_id, {}).values())
        output.append(replace(finding, assignment=Assignment(primary_item, related_items)))

    by_id = {item.id: item for item in output}
    unassigned = {item.id for item in output if item.assignment.primary is None}
    expected_unassigned = {"miner:L7", "miner:L34", "miner:L35", "miner:L43", "miner:M32"}
    if unassigned != expected_unassigned:
        malformed(
            f"miner unassigned set differs; missing={sorted(expected_unassigned - unassigned)}, "
            f"extra={sorted(unassigned - expected_unassigned)}",
            source_path=roadmap_config.path,
        )
    pinned = {
        "miner:L5": "miner:W0",
        "miner:M1": "miner:W2",
        "miner:M5": "miner:W2",
        "miner:M30": "miner:W9",
        "miner:S5": "miner:W11",
        "miner:S12": "miner:W11",
        "miner:L66": "miner:DEFERRED",
    }
    for obligation_id, expected in pinned.items():
        actual = by_id[obligation_id].assignment.primary
        if actual is None or actual.workstream != expected:
            malformed(f"{obligation_id} expected primary {expected}", source_path=roadmap_config.path)

    declared_match = re.search(r"\((?P<count>\d+) confirmed gaps:", text)
    if declared_match is None:
        malformed("miner roadmap declared-count claim is missing", source_path=roadmap_config.path)
    declared = int(declared_match.group("count"))
    diagnostics: list[Diagnostic] = []
    if declared != len(findings):
        diagnostics.append(
            Diagnostic(
                FailureCode.DECLARED_COUNT_MISMATCH.value,
                source_path=roadmap_config.path,
                field="declared_count",
                message=f"roadmap declares {declared} findings; enumerated source has {len(findings)}",
                fatal=False,
            )
        )
    return tuple(sorted(output, key=lambda item: item.id)), tuple(sorted(diagnostics))


def import_miner(
    scan_raw: bytes,
    roadmap_raw: bytes,
    scan_config: SourceConfig,
    roadmap_config: SourceConfig,
) -> tuple[tuple[Obligation, ...], tuple[Diagnostic, ...]]:
    findings = import_miner_findings(scan_raw, scan_config)
    return apply_miner_assignments(findings, roadmap_raw, roadmap_config)
