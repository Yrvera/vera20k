"""Research-index validity checks."""

from __future__ import annotations

from pathlib import Path

from .database import connect
from .lifecycle import (
    IndexLifecycleError,
    corpus_snapshot,
    effective_root_labels,
    normalize_roots,
)
from .metadata import checksum
from .system_map import document_filters


def validate_index(
    db_path: Path,
    workspace: Path,
    system: str | None = None,
    topic: str | None = None,
    source_kind: str | None = None,
    status: str | None = None,
    limit: int = 40,
) -> dict:
    conn = connect(db_path)
    try:
        docs = scoped_documents(conn, system, topic, source_kind, status)
        doc_ids = [doc["id"] for doc in docs]
        missing_files = []
        checksum_mismatches = []
        stale_or_unknown = []
        unindexed_files = []
        corpus_errors = []

        for doc in docs:
            path = workspace / doc["path"]
            if not path.exists():
                missing_files.append(public_doc(doc))
                continue

            text = path.read_text(encoding="utf-8", errors="replace")
            current_checksum = checksum(text)
            if current_checksum != doc["checksum"]:
                item = public_doc(doc)
                item["indexed_checksum"] = doc["checksum"]
                item["current_checksum"] = current_checksum
                checksum_mismatches.append(item)

            if doc["status"] == "stale" or doc["source_kind"] == "unknown" or doc["status"] == "unknown":
                stale_or_unknown.append(public_doc(doc))

        if not any((system, topic, source_kind, status)):
            try:
                _, _, discovered = normalize_roots(
                    workspace,
                    effective_root_labels(db_path, workspace),
                )
                current_paths = set(corpus_snapshot(workspace, discovered))
                indexed_paths = {doc["path"] for doc in docs}
                unindexed_files = sorted(current_paths - indexed_paths)
            except (IndexLifecycleError, OSError) as exc:
                corpus_errors.append(str(exc))

        missing_links = scoped_missing_links(conn, doc_ids, workspace)
        validity_errors = (
            len(missing_files)
            + len(checksum_mismatches)
            + len(missing_links)
            + len(unindexed_files)
            + len(corpus_errors)
        )
        scope_matched = bool(docs)

        return {
            "system": system,
            "topic": topic,
            "source_kind": source_kind,
            "status": status,
            "documents_checked": len(docs),
            "scope_matched": scope_matched,
            "valid": scope_matched and validity_errors == 0,
            "missing_files": missing_files[:limit],
            "checksum_mismatches": checksum_mismatches[:limit],
            "missing_links": missing_links[:limit],
            "stale_or_unknown": stale_or_unknown[:limit],
            "unindexed_files": unindexed_files[:limit],
            "corpus_errors": corpus_errors[:limit],
            "counts": {
                "missing_files": len(missing_files),
                "checksum_mismatches": len(checksum_mismatches),
                "missing_links": len(missing_links),
                "stale_or_unknown": len(stale_or_unknown),
                "unindexed_files": len(unindexed_files),
                "corpus_errors": len(corpus_errors),
            },
        }
    finally:
        conn.close()


def scoped_documents(conn, system: str | None, topic: str | None, source_kind: str | None, status: str | None) -> list[dict]:
    sql = """
        SELECT DISTINCT d.id, d.path, d.title, d.system, d.subsystem, d.source_kind, d.status, d.checksum
        FROM documents d
        LEFT JOIN chunks c ON c.document_id = d.id
    """
    where, params = document_filters(system, topic, source_kind, status)
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += " ORDER BY d.path"
    return [dict(row) for row in conn.execute(sql, params)]


def scoped_missing_links(
    conn,
    doc_ids: list[int],
    workspace: Path,
) -> list[dict]:
    if not doc_ids:
        return []
    placeholders = ",".join("?" for _ in doc_ids)
    rows = conn.execute(
        f"""
        SELECT d.path, d.title, d.source_kind, d.status, l.target
        FROM links l
        JOIN documents d ON d.id = l.document_id
        WHERE l.document_id IN ({placeholders})
          AND l.target NOT LIKE 'http://%'
          AND l.target NOT LIKE 'https://%'
          AND l.target NOT LIKE 'mailto:%'
        ORDER BY d.path, l.target
        """,
        doc_ids,
    )
    results = []
    for row in rows:
        target = Path(row["target"])
        if not target.is_absolute():
            target = (workspace / row["path"]).parent / target
        if target.exists():
            continue
        results.append(
            {
                "path": row["path"],
                "title": row["title"],
                "source_kind": row["source_kind"],
                "status": row["status"],
                "target": row["target"],
            }
        )
    return results


def public_doc(doc: dict) -> dict:
    return {
        "path": doc["path"],
        "title": doc["title"],
        "system": doc["system"],
        "subsystem": doc["subsystem"],
        "source_kind": doc["source_kind"],
        "status": doc["status"],
    }
