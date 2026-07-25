"""System and topic map queries for research docs."""

from __future__ import annotations

from pathlib import Path
import re

from .database import connect


SIGNAL_MARKERS = (
    "contradict",
    "correction",
    "corrects",
    "open question",
    "refuted",
    "superseded",
    "stale",
    "remaining uncertainty",
    "uncertainty",
)


def system_map(
    db_path: Path,
    system: str | None = None,
    topic: str | None = None,
    source_kind: str | None = None,
    status: str | None = None,
    limit: int = 80,
) -> dict:
    conn = connect(db_path)
    try:
        docs = matching_documents(conn, system, topic, source_kind, status, limit)
        doc_ids = [doc["id"] for doc in docs]
        document_count = count_matching_documents(
            conn,
            system,
            topic,
            source_kind,
            status,
        )
        return {
            "system": system,
            "topic": topic,
            "source_kind": source_kind,
            "status": status,
            "matched": document_count > 0,
            "document_count": document_count,
            "groups": matching_groups(conn, system, topic, source_kind, status),
            "documents": [public_document_row(doc) for doc in docs],
            "handoff_sections": matching_handoff_sections(conn, doc_ids, topic, limit),
            "signals": matching_signal_sections(conn, doc_ids, topic, limit),
        }
    finally:
        conn.close()


def matching_documents(conn, system: str | None, topic: str | None, source_kind: str | None, status: str | None, limit: int) -> list[dict]:
    sql = """
        SELECT d.id, d.path, d.title, d.system, d.subsystem, d.source_kind, d.status,
               COUNT(c.id) AS matching_chunks
        FROM documents d
        LEFT JOIN chunks c ON c.document_id = d.id
    """
    where, params = document_filters(system, topic, source_kind, status)
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += """
        GROUP BY d.id
        ORDER BY
          CASE d.source_kind
            WHEN 'ghidra' THEN 7
            WHEN 'trace' THEN 6
            WHEN 'contract' THEN 5
            WHEN 'synthesis' THEN 4
            WHEN 'audit' THEN 3
            WHEN 'ini' THEN 2
            WHEN 'plan' THEN 1
            ELSE 0
          END DESC,
          CASE d.status
            WHEN 'verified' THEN 4
            WHEN 'synthesis' THEN 3
            WHEN 'plan' THEN 2
            WHEN 'unknown' THEN 1
            ELSE 0
          END DESC,
          d.subsystem,
          d.path
        LIMIT ?
    """
    return [dict(row) for row in conn.execute(sql, (*params, limit))]


def count_matching_documents(conn, system: str | None, topic: str | None, source_kind: str | None, status: str | None) -> int:
    sql = "SELECT COUNT(DISTINCT d.id) AS count FROM documents d LEFT JOIN chunks c ON c.document_id = d.id"
    where, params = document_filters(system, topic, source_kind, status)
    if where:
        sql += " WHERE " + " AND ".join(where)
    return int(conn.execute(sql, params).fetchone()["count"])


def matching_groups(conn, system: str | None, topic: str | None, source_kind: str | None, status: str | None) -> list[dict]:
    sql = """
        SELECT d.subsystem, d.source_kind, d.status, COUNT(DISTINCT d.id) AS count
        FROM documents d
        LEFT JOIN chunks c ON c.document_id = d.id
    """
    where, params = document_filters(system, topic, source_kind, status)
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += """
        GROUP BY d.subsystem, d.source_kind, d.status
        ORDER BY d.subsystem, d.source_kind, d.status
    """
    return [
        {"subsystem": row["subsystem"], "source_kind": row["source_kind"], "status": row["status"], "count": row["count"]}
        for row in conn.execute(sql, params)
    ]


def document_filters(system: str | None, topic: str | None, source_kind: str | None, status: str | None) -> tuple[list[str], list[object]]:
    where = []
    params: list[object] = []
    if system:
        where.append("d.system = ?")
        params.append(system)
    if source_kind:
        where.append("d.source_kind = ?")
        params.append(source_kind)
    if status:
        where.append("d.status = ?")
        params.append(status)
    topic_terms = topic_tokens(topic)
    for term in topic_terms:
        where.append("(lower(d.path) LIKE ? OR lower(d.title) LIKE ? OR lower(c.heading_path) LIKE ? OR lower(c.text) LIKE ?)")
        like = f"%{term}%"
        params.extend([like, like, like, like])
    return where, params


def topic_tokens(topic: str | None) -> list[str]:
    if not topic:
        return []
    return [token.lower() for token in re.findall(r"[A-Za-z0-9_:+./-]{3,}", topic)]


def matching_handoff_sections(conn, doc_ids: list[int], topic: str | None, limit: int) -> list[dict]:
    return matching_sections(conn, doc_ids, topic, ("implementation handoff", "current rust delta", "affected rust surface"), limit)


def matching_signal_sections(conn, doc_ids: list[int], topic: str | None, limit: int) -> list[dict]:
    return matching_sections(conn, doc_ids, topic, SIGNAL_MARKERS, limit)


def matching_sections(conn, doc_ids: list[int], topic: str | None, markers: tuple[str, ...], limit: int) -> list[dict]:
    if not doc_ids:
        return []
    placeholders = ",".join("?" for _ in doc_ids)
    marker_clause = " OR ".join("lower(c.heading_path) LIKE ? OR lower(c.text) LIKE ?" for _ in markers)
    marker_params: list[object] = []
    for marker in markers:
        like = f"%{marker}%"
        marker_params.extend([like, like])

    topic_terms = topic_tokens(topic)
    topic_clause = ""
    topic_params: list[object] = []
    for term in topic_terms:
        topic_clause += " AND (lower(d.path) LIKE ? OR lower(d.title) LIKE ? OR lower(c.heading_path) LIKE ? OR lower(c.text) LIKE ?)"
        like = f"%{term}%"
        topic_params.extend([like, like, like, like])

    rows = conn.execute(
        f"""
        SELECT d.path, d.title, d.source_kind, d.status, c.heading_path, c.start_line, c.end_line, c.text
        FROM chunks c
        JOIN documents d ON d.id = c.document_id
        WHERE c.document_id IN ({placeholders})
          AND ({marker_clause})
          {topic_clause}
        ORDER BY
          CASE d.source_kind
            WHEN 'ghidra' THEN 7
            WHEN 'trace' THEN 6
            WHEN 'contract' THEN 5
            WHEN 'synthesis' THEN 4
            WHEN 'audit' THEN 3
            WHEN 'ini' THEN 2
            WHEN 'plan' THEN 1
            ELSE 0
          END DESC,
          d.path,
          c.start_line
        LIMIT ?
        """,
        (*doc_ids, *marker_params, *topic_params, limit),
    )
    return [section_row(row) for row in rows]


def section_row(row) -> dict:
    return {
        "path": row["path"],
        "title": row["title"],
        "source_kind": row["source_kind"],
        "status": row["status"],
        "heading_path": row["heading_path"],
        "start_line": row["start_line"],
        "end_line": row["end_line"],
        "snippet": snippet(row["text"]),
    }


def public_document_row(row: dict) -> dict:
    return {
        "path": row["path"],
        "title": row["title"],
        "system": row["system"],
        "subsystem": row["subsystem"],
        "source_kind": row["source_kind"],
        "status": row["status"],
        "matching_chunks": row["matching_chunks"],
    }


def snippet(text: str, width: int = 260) -> str:
    compact = " ".join(text.split())
    return compact if len(compact) <= width else compact[: width - 3].rstrip() + "..."
