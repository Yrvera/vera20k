"""Compact pre-implementation research brief assembly."""

from __future__ import annotations

from pathlib import Path

from .graph import evidence_view, implementation_view
from .handoff import parity_handoff
from .system_map import system_map
from .validation import validate_index


def research_brief(
    db_path: Path,
    workspace: Path,
    query: str,
    system: str | None = None,
    source_kind: str | None = None,
    anchors: list[str] | None = None,
    limit: int = 8,
) -> dict:
    anchors = anchors or []
    return {
        "query": query,
        "system": system,
        "source_kind": source_kind,
        "validation": validate_index(db_path, workspace, system=system, topic=query, source_kind=source_kind, limit=limit),
        "map": system_map(db_path, system=system, topic=query, source_kind=source_kind, limit=limit),
        "handoff": parity_handoff(
            db_path,
            query,
            limit=limit,
            system=system,
            source_kind=source_kind,
            workspace=workspace,
        ),
        "anchors": [
            anchor_brief(db_path, anchor, limit, workspace=workspace)
            for anchor in anchors
        ],
    }


def anchor_brief(
    db_path: Path,
    anchor: str,
    limit: int,
    workspace: Path | None = None,
) -> dict:
    evidence = evidence_view(db_path, anchor, limit=limit)
    implementation = implementation_view(
        db_path,
        anchor,
        limit=limit,
        workspace=workspace,
    )
    return {
        "anchor": anchor,
        "evidence_documents": evidence["documents"],
        "implementation_documents": implementation["documents"],
        "rust_paths": implementation["rust_paths"],
    }
