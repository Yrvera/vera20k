"""SQLite storage and query functions for the research index."""

from __future__ import annotations

import os
import re
import sqlite3
import tempfile
from pathlib import Path

from .chunking import Chunk
from .metadata import DocumentMetadata, extract_terms
from .ranking import final_score, related_evidence_weight


TOOL_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DB = TOOL_ROOT / ".cache" / "research.db"
SCHEMA = TOOL_ROOT / "schema.sql"
QUERY_TOKEN_RE = re.compile(r"[A-Za-z0-9_:+./-]{2,}")
CORPUS_GENERIC_QUERY_TERMS = {
    "acceptance",
    "affected",
    "current",
    "doc",
    "docs",
    "document",
    "documents",
    "effect",
    "evidence",
    "handoff",
    "implementation",
    "query",
    "report",
    "research",
    "required",
    "rust",
    "scenario",
    "touchpoint",
    "touchpoints",
    "verified",
}


def connect(db_path: Path) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    return conn


def rebuild_database(db_path: Path, workspace: Path, documents: list[tuple[str, DocumentMetadata, list[Chunk]]]) -> None:
    db_path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, tmp_name = tempfile.mkstemp(
        prefix=f"{db_path.name}.",
        suffix=".tmp",
        dir=db_path.parent,
    )
    os.close(descriptor)
    tmp_path = Path(tmp_name)

    try:
        conn = connect(tmp_path)
        try:
            conn.executescript(SCHEMA.read_text(encoding="utf-8"))
            doc_ids: dict[str, int] = {}
            with conn:
                for relpath, meta, chunks in documents:
                    doc_id = insert_document(conn, relpath, meta)
                    doc_ids[relpath] = doc_id
                    for chunk in chunks:
                        chunk_id = insert_chunk(conn, doc_id, relpath, chunk)
                        insert_terms(conn, doc_id, chunk_id, chunk.text, Path(relpath).suffix)
                    insert_links(conn, doc_id, workspace, relpath, "\n".join(chunk.text for chunk in chunks), Path(relpath).suffix)
                insert_edges(conn, workspace, documents, doc_ids)
        finally:
            conn.close()
        os.replace(tmp_path, db_path)
    finally:
        if tmp_path.exists():
            tmp_path.unlink()


def insert_document(conn: sqlite3.Connection, relpath: str, meta: DocumentMetadata) -> int:
    cursor = conn.execute(
        """
        INSERT INTO documents(path, title, system, subsystem, source_kind, status, modified_time, checksum)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (relpath, meta.title, meta.system, meta.subsystem, meta.source_kind, meta.status, meta.modified_time, meta.checksum),
    )
    return int(cursor.lastrowid)


def insert_chunk(conn: sqlite3.Connection, doc_id: int, relpath: str, chunk: Chunk) -> int:
    cursor = conn.execute(
        """
        INSERT INTO chunks(document_id, heading_path, start_line, end_line, text)
        VALUES (?, ?, ?, ?, ?)
        """,
        (doc_id, chunk.heading_path, chunk.start_line, chunk.end_line, chunk.text),
    )
    chunk_id = int(cursor.lastrowid)
    conn.execute(
        "INSERT INTO chunks_fts(rowid, text, heading_path, path, chunk_id) VALUES (?, ?, ?, ?, ?)",
        (chunk_id, chunk.text, chunk.heading_path, relpath, chunk_id),
    )
    return chunk_id


def insert_terms(conn: sqlite3.Connection, doc_id: int, chunk_id: int, text: str, suffix: str) -> None:
    terms = extract_terms(text, suffix)
    conn.executemany("INSERT INTO addresses(document_id, chunk_id, address) VALUES (?, ?, ?)", ((doc_id, chunk_id, item) for item in terms.addresses))
    conn.executemany("INSERT INTO symbols(document_id, chunk_id, symbol) VALUES (?, ?, ?)", ((doc_id, chunk_id, item) for item in terms.symbols))
    conn.executemany("INSERT INTO ini_keys(document_id, chunk_id, key) VALUES (?, ?, ?)", ((doc_id, chunk_id, item) for item in terms.ini_keys))
    conn.executemany("INSERT INTO rust_paths(document_id, chunk_id, path) VALUES (?, ?, ?)", ((doc_id, chunk_id, item) for item in terms.rust_paths))


def insert_links(conn: sqlite3.Connection, doc_id: int, workspace: Path, relpath: str, text: str, suffix: str) -> None:
    terms = extract_terms(text, suffix)
    base = workspace / relpath
    parent = base.parent
    rows = []
    for target in terms.links:
        if target.startswith(("http://", "https://", "mailto:")):
            exists = 1
        else:
            exists = 1 if (parent / target).exists() else 0
        rows.append((doc_id, target, exists))
    conn.executemany("INSERT INTO links(document_id, target, exists_flag) VALUES (?, ?, ?)", rows)


def insert_edges(conn: sqlite3.Connection, workspace: Path, documents: list[tuple[str, DocumentMetadata, list[Chunk]]], doc_ids: dict[str, int]) -> None:
    rows = []
    for relpath, meta, chunks in documents:
        doc_id = doc_ids[relpath]
        suffix = Path(relpath).suffix

        rows.extend(metadata_edges(doc_id, meta))
        for chunk in chunks:
            terms = extract_terms(chunk.text, suffix)
            rows.extend(term_edges(doc_id, "mentions_symbol", terms.symbols, 1.0, chunk.start_line, chunk.end_line))
            rows.extend(term_edges(doc_id, "mentions_address", terms.addresses, 1.0, chunk.start_line, chunk.end_line))
            rows.extend(term_edges(doc_id, "mentions_ini_key", terms.ini_keys, 1.1, chunk.start_line, chunk.end_line))
            rows.extend(term_edges(doc_id, "mentions_rust_path", terms.rust_paths, 1.3, chunk.start_line, chunk.end_line))
            rows.extend(reference_edges(doc_id, workspace, relpath, terms.links, doc_ids, chunk.start_line, chunk.end_line))

    conn.executemany(
        """
        INSERT INTO edges(source_document_id, edge_kind, target, target_document_id, source_start_line, source_end_line, weight, evidence)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        rows,
    )


def metadata_edges(doc_id: int, meta: DocumentMetadata) -> list[tuple[int, str, str, int | None, int | None, int | None, float, str]]:
    return [
        (doc_id, "belongs_to_system", meta.system, None, None, None, 0.8, "path metadata"),
        (doc_id, "belongs_to_subsystem", meta.subsystem, None, None, None, 0.7, "path metadata"),
        (doc_id, "has_source_kind", meta.source_kind, None, None, None, 0.6, "filename metadata"),
        (doc_id, "has_status", meta.status, None, None, None, 0.6, "filename metadata"),
    ]


def term_edges(
    doc_id: int,
    edge_kind: str,
    terms: tuple[str, ...],
    weight: float,
    start_line: int,
    end_line: int,
) -> list[tuple[int, str, str, int | None, int, int, float, str]]:
    return [(doc_id, edge_kind, term, None, start_line, end_line, weight, "extracted term") for term in terms]


def reference_edges(
    doc_id: int,
    workspace: Path,
    relpath: str,
    links: tuple[str, ...],
    doc_ids: dict[str, int],
    start_line: int,
    end_line: int,
) -> list[tuple[int, str, str, int | None, int, int, float, str]]:
    parent = (workspace / relpath).parent
    rows = []
    for link in links:
        if link.startswith(("http://", "https://", "mailto:")):
            continue
        resolved = (parent / link).resolve()
        try:
            target = resolved.relative_to(workspace).as_posix()
        except ValueError:
            target = link
        target_doc_id = doc_ids.get(target)
        rows.append((doc_id, "references_doc", target, target_doc_id, start_line, end_line, 1.5 if target_doc_id else 0.4, link))
    return rows


def search(db_path: Path, query: str, limit: int = 10, system: str | None = None, source_kind: str | None = None) -> list[dict]:
    if not query.strip():
        return []

    conn = connect(db_path)
    try:
        fts_query = to_fts_query(query)
        rows = run_fts(conn, fts_query, max(100, limit * 12), system, source_kind)
        if not rows:
            rows = run_like(conn, query, max(40, limit * 8), system, source_kind)

        exact_terms = exact_term_set(query)
        informative_terms = informative_query_terms(query)
        results = []
        for row in rows:
            exact_boost = exact_match_boost(row["text"], exact_terms)
            matched_terms = matched_query_terms(row, informative_terms)
            coverage = len(matched_terms) / len(informative_terms) if informative_terms else 0.0
            coverage_boost = min(0.6, coverage * 0.4 + len(matched_terms) * 0.05)
            score = final_score(
                float(row["bm25"]),
                row["source_kind"],
                row["status"],
                exact_boost + coverage_boost,
            )
            item = dict(row)
            item["score"] = round(score, 4)
            item["snippet"] = snippet(row["text"], query)
            item["matched_terms"] = list(matched_terms)
            item["query_hit_count"] = len(matched_terms)
            item["query_coverage"] = round(coverage, 4)
            results.append(item)

        results.sort(key=lambda item: item["score"], reverse=True)
        return results[:limit]
    finally:
        conn.close()


def run_fts(conn: sqlite3.Connection, fts_query: str, limit: int, system: str | None, source_kind: str | None) -> list[sqlite3.Row]:
    sql = """
        SELECT d.id AS document_id, d.path, d.title, d.system, d.subsystem, d.source_kind, d.status,
               c.heading_path, c.start_line, c.end_line, c.text,
               bm25(chunks_fts) AS bm25
        FROM chunks_fts
        JOIN chunks c ON c.id = chunks_fts.chunk_id
        JOIN documents d ON d.id = c.document_id
        WHERE chunks_fts MATCH ?
    """
    params: list[object] = [fts_query]
    sql, params = add_filters(sql, params, system, source_kind)
    sql += " ORDER BY bm25(chunks_fts) LIMIT ?"
    params.append(limit)
    try:
        return list(conn.execute(sql, params))
    except sqlite3.OperationalError:
        return []


def run_like(conn: sqlite3.Connection, query: str, limit: int, system: str | None, source_kind: str | None) -> list[sqlite3.Row]:
    sql = """
        SELECT d.id AS document_id, d.path, d.title, d.system, d.subsystem, d.source_kind, d.status,
               c.heading_path, c.start_line, c.end_line, c.text,
               10.0 AS bm25
        FROM chunks c
        JOIN documents d ON d.id = c.document_id
        WHERE c.text LIKE ?
    """
    params: list[object] = [f"%{query}%"]
    sql, params = add_filters(sql, params, system, source_kind)
    sql += " LIMIT ?"
    params.append(limit)
    return list(conn.execute(sql, params))


def add_filters(sql: str, params: list[object], system: str | None, source_kind: str | None) -> tuple[str, list[object]]:
    if system:
        sql += " AND d.system = ?"
        params.append(system)
    if source_kind:
        sql += " AND d.source_kind = ?"
        params.append(source_kind)
    return sql, params


def related_by_document(db_path: Path, path: str, limit: int = 20) -> list[dict]:
    normalized = Path(path).as_posix()
    conn = connect(db_path)
    try:
        doc = conn.execute("SELECT id, path, system, subsystem FROM documents WHERE path = ? OR path LIKE ?", (normalized, f"%{normalized}")).fetchone()
        if not doc:
            return []

        source = {"id": int(doc["id"]), "system": doc["system"], "subsystem": doc["subsystem"]}
        terms = collect_document_terms(conn, source["id"])
        return related_for_terms(conn, terms, source, limit)
    finally:
        conn.close()


def related_by_term(db_path: Path, term: str, limit: int = 20) -> list[dict]:
    conn = connect(db_path)
    try:
        return related_for_terms(conn, {term}, None, limit)
    finally:
        conn.close()


def collect_document_terms(conn: sqlite3.Connection, doc_id: int) -> set[str]:
    terms: set[str] = set()
    for table, column in (("symbols", "symbol"), ("addresses", "address"), ("ini_keys", "key"), ("rust_paths", "path")):
        for row in conn.execute(f"SELECT DISTINCT {column} AS value FROM {table} WHERE document_id = ?", (doc_id,)):
            terms.add(row["value"])
    return terms


def related_for_terms(conn: sqlite3.Connection, terms: set[str], source: dict | None, limit: int) -> list[dict]:
    if not terms:
        return []

    scores: dict[int, dict] = {}
    lookups = (
        ("symbols", "symbol"),
        ("addresses", "address"),
        ("ini_keys", "key"),
        ("rust_paths", "path"),
    )

    for term in terms:
        for table, column in lookups:
            term_weight = term_idf(conn, table, column, term)
            if term_weight <= 0.0:
                continue
            rows = conn.execute(
                f"""
                SELECT d.id, d.path, d.title, d.system, d.subsystem, d.source_kind, d.status, ? AS matched_term, ? AS matched_kind
                FROM {table} t
                JOIN documents d ON d.id = t.document_id
                WHERE t.{column} = ?
                """,
                (term, table, term),
            )
            for row in rows:
                doc_id = int(row["id"])
                if source is not None and doc_id == source["id"]:
                    continue
                entry = scores.setdefault(doc_id, {key: row[key] for key in row.keys() if key != "id"})
                entry.setdefault("matches", [])
                entry["matches"].append({"kind": row["matched_kind"], "term": row["matched_term"], "weight": term_weight})

    results = list(scores.values())
    for item in results:
        unique = {(match["kind"], match["term"], match["weight"]) for match in item["matches"]}
        item["match_count"] = len(unique)
        item["matches"] = [{"kind": kind, "term": term, "weight": round(weight, 3)} for kind, term, weight in sorted(unique)]
        item["related_score"] = round(related_score(item, source), 4)
    results.sort(key=lambda item: item["related_score"], reverse=True)
    return results[:limit]


def term_idf(conn: sqlite3.Connection, table: str, column: str, term: str) -> float:
    total = conn.execute("SELECT COUNT(*) AS count FROM documents").fetchone()["count"]
    row = conn.execute(f"SELECT COUNT(DISTINCT document_id) AS count FROM {table} WHERE {column} = ?", (term,)).fetchone()
    frequency = max(int(row["count"]), 1)
    # Ignore ultra-common terms; they make everything related to everything else.
    if frequency > max(50, total * 0.08):
        return 0.0
    return 1.0 + (total / frequency) ** 0.5 / 4.0


def related_score(item: dict, source: dict | None) -> float:
    weighted_overlap = sum(match["weight"] for match in item["matches"])
    score = weighted_overlap + related_evidence_weight(item["source_kind"], item["status"])

    if source is not None:
        if item["system"] == source["system"]:
            score += 2.0
        if item["subsystem"] == source["subsystem"]:
            score += 1.0
        if item["system"] != source["system"]:
            score *= 0.55
    else:
        searchable = f"{item['path']} {item['title']}".lower()
        title_path_hits = sum(1 for match in item["matches"] if match["term"].lower() in searchable)
        score += min(2.0, title_path_hits * 0.75)
        if item["system"] == "bridges" and any(is_bridge_term(match["term"]) for match in item["matches"]):
            score += 0.9

    basename = Path(item["path"]).name.upper()
    if basename in {"_MANIFEST.YAML", "_MANIFEST.YML"}:
        score *= 0.25
    if basename in {"AUDIT_LOG.MD", "ADDRESS_MAP.MD", "LABEL_AUDIT_LOG.MD"}:
        score *= 0.20
    elif "AUDIT" in basename or "INDEX" in basename or "ADDRESS_MAP" in basename:
        score *= 0.55
    elif "COMPLETE_DECODE" in basename or "MASTER" in basename:
        score *= 0.65
    if item["source_kind"] == "plan":
        score *= 0.70

    # Reward focused documents over broad catch-alls once the raw overlap is high.
    score = score / (1.0 + max(item["match_count"] - 30, 0) * 0.015)
    return score


def is_bridge_term(term: str) -> bool:
    lowered = term.lower()
    return any(marker in lowered for marker in ("bridge", "cabhut", "onbridge", "repairhut", "tubeclass"))


def to_fts_query(query: str) -> str:
    tokens = re.findall(r"[A-Za-z0-9_:+.-]+", query)
    if not tokens:
        return query
    return " OR ".join(f'"{token}"' if any(ch in token for ch in "_:+.-") else token for token in tokens)


def exact_term_set(query: str) -> set[str]:
    return set(re.findall(r"[A-Za-z0-9_:+./-]{3,}", query))


def informative_query_terms(query: str) -> tuple[str, ...]:
    """Return stable query terms that carry topic meaning in this corpus.

    Generic workflow words are useful for broad FTS discovery but must not be
    enough to make an implementation handoff look relevant.
    """

    terms = []
    seen = set()
    for match in QUERY_TOKEN_RE.finditer(query):
        term = match.group(0).lower()
        if term in CORPUS_GENERIC_QUERY_TERMS:
            continue
        if len(term) < 3 and not any(ch.isdigit() for ch in term):
            continue
        if term not in seen:
            seen.add(term)
            terms.append(term)
    return tuple(terms)


def matched_query_terms(row: sqlite3.Row | dict, terms: tuple[str, ...]) -> tuple[str, ...]:
    haystack = " ".join(
        str(row[key])
        for key in ("path", "title", "heading_path", "text")
        if key in row.keys() and row[key] is not None
    ).lower()
    return tuple(
        term
        for term in terms
        if query_term_matches_text(term, haystack)
    )


def query_term_matches_text(term: str, text: str) -> bool:
    lowered_term = term.lower()
    lowered_text = text.lower()
    normalized = normalized_address(lowered_term)
    if normalized is not None:
        return any(
            normalized_address(match.group(0)) == normalized
            for match in re.finditer(r"\b0x[0-9a-f]{4,}\b", lowered_text)
        )
    if any(marker in lowered_term for marker in ("::", "_", ":", "+", ".", "/")):
        return lowered_term in lowered_text
    return (
        re.search(
            rf"(?<![a-z0-9]){re.escape(lowered_term)}(?![a-z0-9])",
            lowered_text,
        )
        is not None
    )


def required_query_hits(terms: tuple[str, ...]) -> int:
    if not terms:
        return 0
    return 1 if len(terms) == 1 else 2


def normalized_address(value: str) -> str | None:
    if re.fullmatch(r"0x[0-9a-f]+", value.lower()) is None:
        return None
    return value.lower().removeprefix("0x").lstrip("0") or "0"


def exact_match_boost(text: str, terms: set[str]) -> float:
    lowered = text.lower()
    return min(0.4, 0.08 * sum(1 for term in terms if term.lower() in lowered))


def snippet(text: str, query: str, width: int = 360) -> str:
    lowered = text.lower()
    positions = [lowered.find(term.lower()) for term in exact_term_set(query)]
    positions = [pos for pos in positions if pos >= 0]
    center = min(positions) if positions else 0
    start = max(0, center - width // 3)
    end = min(len(text), start + width)
    result = text[start:end].replace("\n", " ")
    if start > 0:
        result = "..." + result
    if end < len(text):
        result += "..."
    return " ".join(result.split())
