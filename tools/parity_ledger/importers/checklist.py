"""Adapters for the core-engine and foundational-scheduler checklists."""

from __future__ import annotations

import hashlib

from .. import IMPORTER_VERSION
from ..model import (
    Assignment,
    AssignmentMention,
    AssignmentRole,
    Obligation,
    ObligationKind,
    SourceClaims,
    SourceRef,
)
from ..source_sets import SourceConfig
from .common import H3_RE, bounded_section, checklist_records, extract_rust_paths, malformed, slugify, strict_text


def _identity(namespace: str, heading_path: tuple[str, ...], title: str) -> str:
    payload = "\0".join((namespace, " / ".join(heading_path), title)).encode("utf-8")
    suffix = hashlib.sha256(payload).hexdigest()[:16]
    heading = slugify(heading_path[-1] if heading_path else "unsectioned")
    return f"{namespace}:{heading}:{suffix}"


def _source_ref(config: SourceConfig, digest: str, local_id: str) -> SourceRef:
    return SourceRef(
        config.path,
        local_id,
        config.source_id,
        digest,
        config.tracking,
        config.adapter,
        IMPORTER_VERSION,
    )


def _build(
    raw: bytes,
    config: SourceConfig,
    sections: tuple[tuple[str, str | None, ObligationKind, int], ...],
) -> tuple[Obligation, ...]:
    text, digest = strict_text(raw, source_path=config.path)
    output: list[Obligation] = []
    for start, end, kind, expected_count in sections:
        section = bounded_section(text, start, end)
        records = checklist_records(section)
        if len(records) != expected_count:
            malformed(
                f"section {start!r} expected {expected_count} records, found {len(records)}",
                source_path=config.path,
            )
        for heading_path, title in records:
            effective_path = heading_path or (start,)
            obligation_id = _identity(config.system, effective_path, title)
            source = _source_ref(config, digest, obligation_id.split(":", 1)[1])
            workstream = f"{config.system}:{slugify(effective_path[-1])}"
            primary = AssignmentMention(workstream, AssignmentRole.PRIMARY, source)
            output.append(
                Obligation(
                    obligation_id,
                    config.system,
                    kind,
                    title,
                    source,
                    SourceClaims(),
                    Assignment(primary),
                    rust_anchors=extract_rust_paths(title),
                )
            )
    identifiers = [item.id for item in output]
    if len(identifiers) != len(set(identifiers)):
        malformed("checklist generated duplicate semantic IDs", source_path=config.path)
    return tuple(sorted(output, key=lambda item: item.id))


def import_core_checklist(raw: bytes, config: SourceConfig) -> tuple[Obligation, ...]:
    text, _digest = strict_text(raw, source_path=config.path)
    section = bounded_section(text, "Big Missing Core Systems", "Suggested Next Work")
    expected_headings = [
        "1. Native tick spine / LogicClass scheduler",
        "2. Two RNG streams",
        "3. Object lifecycle and unregister discipline",
        "4. Frame/timing model",
        "5. Authoritative combat/projectile/warhead pipeline",
        "6. Target acquisition / order cadence",
        "7. Map/cell substrate",
        "8. Save/load/hash/MP lockstep substrate",
    ]
    headings = [match.group("title") for match in H3_RE.finditer(section)]
    if headings != expected_headings:
        malformed(f"core H3 headings differ: {headings!r}", source_path=config.path)
    counts = {heading: 0 for heading in expected_headings}
    for heading_path, _title in checklist_records(section):
        if len(heading_path) != 1 or heading_path[0] not in counts:
            malformed(f"core checklist item has invalid heading path {heading_path!r}", source_path=config.path)
        counts[heading_path[0]] += 1
    if any(count != 4 for count in counts.values()):
        malformed(f"core H3 checklist counts differ: {counts!r}", source_path=config.path)
    return _build(
        raw,
        config,
        (("Big Missing Core Systems", "Suggested Next Work", ObligationKind.CORE_OBLIGATION, 32),),
    )


def import_scheduler_checklist(raw: bytes, config: SourceConfig) -> tuple[Obligation, ...]:
    return _build(
        raw,
        config,
        (
            ("Contract Stack To Create", "Implementation Roadmap", ObligationKind.CONTRACT, 7),
            ("Implementation Roadmap", "Do Not Do", ObligationKind.IMPLEMENTATION, 5),
            ("Open Follow-Up Research", None, ObligationKind.RESEARCH, 5),
        ),
    )
