"""Deterministic docgraph queries."""

from __future__ import annotations

from pathlib import Path
import re
import sqlite3

from .database import connect, normalized_address, search
from .ranking import evidence_order_sql, evidence_weight


EDGE_LABELS = {
    "references_doc": "References",
    "mentions_symbol": "Symbols",
    "mentions_address": "Addresses",
    "mentions_ini_key": "INI keys",
    "mentions_rust_path": "Rust paths",
    "belongs_to_system": "System",
    "belongs_to_subsystem": "Subsystem",
    "has_source_kind": "Source kind",
    "has_status": "Status",
}

EVIDENCE_EDGE_KINDS = ("references_doc", "mentions_symbol", "mentions_address", "mentions_ini_key")
IMPLEMENTATION_EDGE_KINDS = ("mentions_rust_path", "mentions_symbol", "mentions_ini_key", "mentions_address")


def document_graph(db_path: Path, target: str, limit: int = 12) -> dict:
    conn = connect(db_path)
    try:
        doc = find_document(conn, target)
        if not doc:
            return {"target": target, "found": False, "outgoing": {}, "incoming": []}

        outgoing = {}
        for edge_kind in EDGE_LABELS:
            rows = list(
                conn.execute(
                    """
                    SELECT edge_kind, target, target_document_id, source_start_line, source_end_line, weight, evidence
                    FROM edges
                    WHERE source_document_id = ? AND edge_kind = ?
                    ORDER BY weight DESC, target, source_start_line
                    LIMIT ?
                    """,
                    (doc["id"], edge_kind, limit),
                )
            )
            if rows:
                outgoing[edge_kind] = [edge_row(row) for row in rows]

        incoming = list(
            conn.execute(
                """
                SELECT d.path, d.title, d.source_kind, d.status, e.edge_kind,
                       e.source_start_line, e.source_end_line, e.evidence, e.weight
                FROM edges e
                JOIN documents d ON d.id = e.source_document_id
                WHERE e.target_document_id = ?
                ORDER BY e.weight DESC, d.path, e.source_start_line
                LIMIT ?
                """,
                (doc["id"], limit),
            )
        )

        return {
            "target": target,
            "found": True,
            "document": document_row(doc),
            "outgoing": outgoing,
            "incoming": [incoming_row(row) for row in incoming],
        }
    finally:
        conn.close()


def backlinks(db_path: Path, target: str, limit: int = 20) -> dict:
    conn = connect(db_path)
    try:
        doc = find_document(conn, target)
        if not doc:
            return {"target": target, "found": False, "incoming": []}

        rows = conn.execute(
            """
            SELECT d.path, d.title, d.source_kind, d.status, e.edge_kind,
                   e.source_start_line, e.source_end_line, e.evidence, e.weight
            FROM edges e
            JOIN documents d ON d.id = e.source_document_id
            WHERE e.target_document_id = ?
            ORDER BY e.weight DESC, d.path, e.source_start_line
            LIMIT ?
            """,
            (doc["id"], limit),
        )
        return {"target": target, "found": True, "document": document_row(doc), "incoming": [incoming_row(row) for row in rows]}
    finally:
        conn.close()


def evidence_view(db_path: Path, target: str, limit: int = 12) -> dict:
    return term_view(db_path, target, EVIDENCE_EDGE_KINDS, "evidence", limit)


def implementation_view(
    db_path: Path,
    target: str,
    limit: int = 12,
    workspace: Path | None = None,
) -> dict:
    return term_view(
        db_path,
        target,
        IMPLEMENTATION_EDGE_KINDS,
        "implementation",
        limit,
        workspace=workspace,
    )


def term_view(
    db_path: Path,
    target: str,
    edge_kinds: tuple[str, ...],
    mode: str,
    limit: int,
    workspace: Path | None = None,
) -> dict:
    conn = connect(db_path)
    try:
        exact_edges = matching_edges(conn, target, edge_kinds, limit)
        related_docs = docs_for_term(conn, target, edge_kinds, limit)

        fallback_matches = []
        if not related_docs:
            fallback_matches = search(db_path, target, limit=limit)

        rust_paths = []
        if mode == "implementation":
            doc_ids = [row["document_id"] for row in related_docs]
            if not doc_ids:
                doc_ids = [row["document_id"] for row in fallback_matches if row.get("document_id") is not None]
            rust_paths = rust_paths_for_docs(conn, doc_ids, limit, workspace=workspace)

        return {
            "mode": mode,
            "target": target,
            "edges": [edge_doc_row(row) for row in exact_edges],
            "documents": [document_edge_row(row) for row in related_docs],
            "fallback_documents": [fallback_document_row(row) for row in fallback_matches],
            "rust_paths": rust_paths,
        }
    finally:
        conn.close()


def find_document(conn: sqlite3.Connection, target: str) -> sqlite3.Row | None:
    normalized = Path(target).as_posix()
    return conn.execute("SELECT * FROM documents WHERE path = ? OR path LIKE ?", (normalized, f"%{normalized}")).fetchone()


def matching_edges(conn: sqlite3.Connection, target: str, edge_kinds: tuple[str, ...], limit: int) -> list[sqlite3.Row]:
    placeholders = ",".join("?" for _ in edge_kinds)
    target_clause, target_params = edge_target_clause(target)
    query_limit = max(100, limit * 10)
    rows = list(
        conn.execute(
            f"""
            SELECT e.edge_kind, e.target, e.source_start_line, e.source_end_line, e.weight,
                   d.path, d.title, d.source_kind, d.status, d.id AS document_id
            FROM edges e
            JOIN documents d ON d.id = e.source_document_id
            WHERE e.edge_kind IN ({placeholders})
              AND {target_clause}
            ORDER BY e.weight DESC, {evidence_order_sql('d')}, d.path, e.source_start_line
            LIMIT ?
            """,
            (*edge_kinds, *target_params, query_limit),
        )
    )
    if rows:
        return rows[:limit]

    return list(
        conn.execute(
            f"""
            SELECT e.edge_kind, e.target, e.source_start_line, e.source_end_line, e.weight,
                   d.path, d.title, d.source_kind, d.status, d.id AS document_id
            FROM edges e
            JOIN documents d ON d.id = e.source_document_id
            WHERE e.edge_kind IN ({placeholders})
              AND e.target LIKE ?
            ORDER BY e.weight DESC, {evidence_order_sql('d')}, d.path, e.source_start_line
            LIMIT ?
            """,
            (*edge_kinds, f"%{target}%", query_limit),
        )
    )[:limit]


def docs_for_term(conn: sqlite3.Connection, target: str, edge_kinds: tuple[str, ...], limit: int) -> list[sqlite3.Row]:
    placeholders = ",".join("?" for _ in edge_kinds)
    target_clause, target_params = edge_target_clause(target)
    query_limit = max(100, limit * 10)
    rows = list(
        conn.execute(
            f"""
            SELECT d.id AS document_id, d.path, d.title, d.system, d.subsystem, d.source_kind, d.status,
                   COUNT(DISTINCT e.edge_kind || ':' || e.target) AS match_count,
                   GROUP_CONCAT(DISTINCT e.edge_kind || ':' || e.target) AS matches,
                   GROUP_CONCAT(DISTINCT e.source_start_line || '-' || e.source_end_line) AS line_ranges
            FROM edges e
            JOIN documents d ON d.id = e.source_document_id
            WHERE e.edge_kind IN ({placeholders})
              AND {target_clause}
            GROUP BY d.id
            ORDER BY match_count DESC, {evidence_order_sql('d')}, d.path
            LIMIT ?
            """,
            (*edge_kinds, *target_params, query_limit),
        )
    )
    if rows:
        return sorted(rows, key=lambda row: graph_document_score(row, target), reverse=True)[:limit]

    rows = list(
        conn.execute(
            f"""
            SELECT d.id AS document_id, d.path, d.title, d.system, d.subsystem, d.source_kind, d.status,
                   COUNT(DISTINCT e.edge_kind || ':' || e.target) AS match_count,
                   GROUP_CONCAT(DISTINCT e.edge_kind || ':' || e.target) AS matches,
                   GROUP_CONCAT(DISTINCT e.source_start_line || '-' || e.source_end_line) AS line_ranges
            FROM edges e
            JOIN documents d ON d.id = e.source_document_id
            WHERE e.edge_kind IN ({placeholders})
              AND e.target LIKE ?
            GROUP BY d.id
            ORDER BY match_count DESC, {evidence_order_sql('d')}, d.path
            LIMIT ?
            """,
            (*edge_kinds, f"%{target}%", query_limit),
        )
    )
    return sorted(rows, key=lambda row: graph_document_score(row, target), reverse=True)[:limit]


def edge_target_clause(target: str) -> tuple[str, list[object]]:
    address = normalized_address(target)
    if address is None:
        return "lower(e.target) = lower(?)", [target]
    return (
        "("
        "lower(e.target) = lower(?) "
        "OR (e.edge_kind = 'mentions_address' "
        "AND ltrim(substr(lower(e.target), 3), '0') = ?)"
        ")",
        [target, address],
    )


def graph_document_score(row: sqlite3.Row, target: str) -> float:
    score = float(row["match_count"]) + evidence_weight(row["source_kind"], row["status"])
    searchable = f"{row['path']} {row['title']}".lower()
    lowered_target = target.lower()
    context_bonus = 0.0

    if lowered_target in searchable:
        context_bonus += 0.6

    target_words = split_identifier(target)
    if target_words and all(word in searchable for word in target_words):
        context_bonus += 0.4
    aligned_words = sum(1 for word in target_words if word_matches_context(word, row))
    context_bonus += min(0.8, aligned_words * 0.35)
    score += min(1.2, context_bonus)

    basename = Path(row["path"]).name.upper()
    if basename in {"AUDIT_LOG.MD", "ADDRESS_MAP.MD", "LABEL_AUDIT_LOG.MD"}:
        score *= 0.20
    elif "AUDIT" in basename or "INDEX" in basename or "ADDRESS_MAP" in basename:
        score *= 0.55
    elif "COMPLETE_DECODE" in basename or "MASTER" in basename:
        score *= 0.65
    if row["source_kind"] == "plan":
        score *= 0.70

    return score


def split_identifier(value: str) -> list[str]:
    words = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", value).replace("_", " ").replace("-", " ")
    return [word.lower() for word in re.findall(r"[A-Za-z0-9]{3,}", words)]


def word_matches_context(word: str, row: sqlite3.Row) -> bool:
    context = f"{row['system']} {row['subsystem']} {row['path']} {row['title']}".lower()
    singular = word[:-1] if word.endswith("s") else word
    return word in context or singular in context


def rust_paths_for_docs(
    conn: sqlite3.Connection,
    doc_ids: list[int],
    limit: int,
    workspace: Path | None = None,
) -> list[dict]:
    if not doc_ids:
        return []
    placeholders = ",".join("?" for _ in doc_ids)
    rows = conn.execute(
        f"""
        SELECT e.target AS rust_path, COUNT(DISTINCT e.source_document_id) AS doc_count,
               GROUP_CONCAT(DISTINCT d.path) AS documents
        FROM edges e
        JOIN documents d ON d.id = e.source_document_id
        WHERE e.edge_kind = 'mentions_rust_path'
          AND e.source_document_id IN ({placeholders})
        GROUP BY e.target
        ORDER BY doc_count DESC, rust_path
        LIMIT ?
        """,
        (*doc_ids, limit),
    )
    results = []
    for row in rows:
        citations = conn.execute(
            f"""
            SELECT d.path, e.source_start_line, e.source_end_line
            FROM edges e
            JOIN documents d ON d.id = e.source_document_id
            WHERE e.edge_kind = 'mentions_rust_path'
              AND e.target = ?
              AND e.source_document_id IN ({placeholders})
            ORDER BY d.path, e.source_start_line
            LIMIT ?
            """,
            (row["rust_path"], *doc_ids, 8),
        )
        item = {
            "rust_path": row["rust_path"],
            "doc_count": row["doc_count"],
            "documents": split_concat(row["documents"]),
            "citations": [
                f"{citation['path']}:{citation['source_start_line']}-{citation['source_end_line']}"
                for citation in citations
            ],
        }
        if workspace is not None:
            item["exists"] = (workspace / row["rust_path"]).is_file()
        results.append(item)
    return results


def document_row(row: sqlite3.Row) -> dict:
    return {
        "path": row["path"],
        "title": row["title"],
        "system": row["system"],
        "subsystem": row["subsystem"],
        "source_kind": row["source_kind"],
        "status": row["status"],
    }


def edge_row(row: sqlite3.Row) -> dict:
    return {
        "edge_kind": row["edge_kind"],
        "target": row["target"],
        "target_document_id": row["target_document_id"],
        "source_start_line": row["source_start_line"],
        "source_end_line": row["source_end_line"],
        "weight": row["weight"],
        "evidence": row["evidence"],
    }


def incoming_row(row: sqlite3.Row) -> dict:
    return {
        "path": row["path"],
        "title": row["title"],
        "source_kind": row["source_kind"],
        "status": row["status"],
        "edge_kind": row["edge_kind"],
        "source_start_line": row["source_start_line"],
        "source_end_line": row["source_end_line"],
        "evidence": row["evidence"],
        "weight": row["weight"],
    }


def edge_doc_row(row: sqlite3.Row) -> dict:
    return {
        "edge_kind": row["edge_kind"],
        "target": row["target"],
        "weight": row["weight"],
        "path": row["path"],
        "title": row["title"],
        "source_kind": row["source_kind"],
        "status": row["status"],
        "source_start_line": row["source_start_line"],
        "source_end_line": row["source_end_line"],
    }


def document_edge_row(row: sqlite3.Row) -> dict:
    return {
        "path": row["path"],
        "title": row["title"],
        "system": row["system"],
        "subsystem": row["subsystem"],
        "source_kind": row["source_kind"],
        "status": row["status"],
        "match_count": row["match_count"],
        "matches": split_concat(row["matches"]),
        "line_ranges": split_concat(row["line_ranges"]),
    }


def fallback_document_row(row: dict) -> dict:
    return {
        "path": row["path"],
        "title": row["title"],
        "system": row["system"],
        "subsystem": row["subsystem"],
        "source_kind": row["source_kind"],
        "status": row["status"],
        "heading_path": row["heading_path"],
        "start_line": row["start_line"],
        "end_line": row["end_line"],
        "score": row["score"],
        "snippet": row["snippet"],
    }


def split_concat(value: str | None) -> list[str]:
    return [] if not value else value.split(",")
