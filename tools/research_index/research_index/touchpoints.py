"""Query-scoped Rust touchpoint discovery and freshness annotation.

This module keeps implementation-graph expansion and path merging separate
from handoff authority clustering. It depends on extracted document terms and
the graph query layer, but it never reads or changes Rust source contents.
"""

from __future__ import annotations

from pathlib import Path

from .database import (
    exact_term_set,
    informative_query_terms,
    normalized_address,
    query_term_matches_text,
)
from .graph import implementation_view
from .metadata import extract_terms


def implementation_term_graphs(
    db_path: Path,
    query: str,
    evidence: list[dict],
    limit: int,
    workspace: Path | None = None,
) -> list[dict]:
    terms = candidate_terms(query, evidence)
    graphs = []
    for term in terms[:12]:
        view = implementation_view(
            db_path,
            term,
            limit=limit,
            workspace=workspace,
        )
        if view["documents"] or view["rust_paths"]:
            graphs.append(
                {
                    "term": term,
                    "documents": view["documents"][:limit],
                    "rust_paths": view["rust_paths"][:limit],
                }
            )
    return graphs


def candidate_terms(query: str, evidence: list[dict]) -> list[str]:
    query_terms = informative_query_terms(query)
    terms = {
        term
        for term in exact_term_set(query)
        if is_specific_term(term)
    }
    for row in evidence[:5]:
        extracted = extract_terms(row["text"], Path(row["path"]).suffix)
        for term in (
            *extracted.addresses,
            *extracted.symbols,
            *extracted.ini_keys,
        ):
            if term_matches_query(term, query_terms):
                terms.add(term)
    return sorted(
        (term for term in terms if is_specific_term(term)),
        key=term_sort_key,
    )


def term_matches_query(term: str, query_terms: tuple[str, ...]) -> bool:
    address = normalized_address(term)
    if address is not None:
        return any(
            normalized_address(query_term) == address
            for query_term in query_terms
        )

    lowered = term.lower()
    return any(
        query_term_matches_text(query_term, lowered)
        or query_term_matches_text(lowered, query_term)
        for query_term in query_terms
    )


def is_specific_term(term: str) -> bool:
    if len(term) < 4:
        return False
    if term.startswith("0x"):
        return True
    if any(marker in term for marker in ("::", "__", "_", "/", ".")):
        return True
    return any(ch.isupper() for ch in term[1:])


def term_sort_key(term: str) -> tuple[int, str]:
    specificity = 0
    if term.startswith("0x"):
        specificity -= 3
    if "::" in term or "__" in term:
        specificity -= 2
    if "_" in term or "/" in term:
        specificity -= 1
    return (specificity, term.lower())


def direct_rust_touchpoints(rows: list[dict]) -> list[dict]:
    merged: dict[str, dict] = {}
    for row in rows:
        terms = extract_terms(row["text"], Path(row["path"]).suffix)
        citation = f"{row['path']}:{row['start_line']}-{row['end_line']}"
        for rust_path in terms.rust_paths:
            entry = merged.setdefault(
                rust_path,
                {
                    "rust_path": rust_path,
                    "terms": set(),
                    "documents": set(),
                    "citations": set(),
                },
            )
            entry["terms"].add("direct")
            entry["documents"].add(row["path"])
            entry["citations"].add(citation)

    return [
        {
            "rust_path": entry["rust_path"],
            "doc_count": len(entry["documents"]),
            "terms": sorted(entry["terms"]),
            "documents": sorted(entry["documents"]),
            "citations": sorted(entry["citations"]),
        }
        for entry in merged.values()
    ]


def merge_rust_touchpoints(
    term_graphs: list[dict],
    direct_rows: list[dict],
    limit: int,
    workspace: Path | None = None,
) -> list[dict]:
    merged: dict[str, dict] = {}
    for row in direct_rows:
        entry = touchpoint_entry(merged, row["rust_path"])
        entry["terms"].update(row.get("terms", []))
        entry["documents"].update(row.get("documents", []))
        entry["citations"].update(row.get("citations", []))

    for graph in term_graphs:
        for row in graph["rust_paths"]:
            entry = touchpoint_entry(merged, row["rust_path"])
            entry["terms"].add(graph["term"])
            entry["documents"].update(row.get("documents", []))
            entry["citations"].update(row.get("citations", []))

    results = []
    for entry in merged.values():
        item = {
            "rust_path": entry["rust_path"],
            "doc_count": len(entry["documents"]),
            "terms": sorted(entry["terms"]),
            "documents": sorted(entry["documents"]),
            "citations": sorted(entry["citations"]),
        }
        if workspace is not None:
            item["exists"] = (workspace / entry["rust_path"]).is_file()
        results.append(item)

    results.sort(
        key=lambda row: (
            row.get("exists") is False,
            -row["doc_count"],
            row["rust_path"],
        )
    )
    return results[:limit]


def touchpoint_entry(merged: dict[str, dict], rust_path: str) -> dict:
    return merged.setdefault(
        rust_path,
        {
            "rust_path": rust_path,
            "terms": set(),
            "documents": set(),
            "citations": set(),
        },
    )
