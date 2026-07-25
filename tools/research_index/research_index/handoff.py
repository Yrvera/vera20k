"""Implementation-oriented research handoff assembly."""

from __future__ import annotations

from pathlib import Path
import re

from .database import (
    connect,
    exact_term_set,
    informative_query_terms,
    normalized_address,
    required_query_hits,
    search,
)
from .ranking import authority_score
from .touchpoints import (
    direct_rust_touchpoints,
    implementation_term_graphs,
    is_specific_term,
    merge_rust_touchpoints,
    term_sort_key,
)


HANDOFF_MARKERS = (
    "implementation handoff",
    "current rust delta",
    "affected rust surface",
    "required implementation effect",
    "acceptance scenario",
)
AUTHORITY_EDGE_KINDS = ("mentions_address", "mentions_symbol", "mentions_ini_key", "mentions_rust_path")
RISK_TERMS = (
    "stale",
    "superseded",
    "wrong",
    "misleading",
    "unchecked",
    "deferred",
    "uncertainty",
    "unknown",
    "replacement wording",
    "correction",
    "corrected",
    "canbegarrisoned",
    "isgate",
)
INI_QUERY_RE = re.compile(r"\b([A-Za-z][A-Za-z0-9_.$-]{1,64})\s*=")


def parity_handoff(
    db_path: Path,
    query: str,
    limit: int = 8,
    system: str | None = None,
    source_kind: str | None = None,
    workspace: Path | None = None,
) -> dict:
    evidence = meaningful_search_results(
        search(
            db_path,
            query,
            limit=max(40, limit * 8),
            system=system,
            source_kind=source_kind,
        ),
        query,
    )[:limit]
    handoff_candidates = implementation_handoff_candidates(db_path, query, limit, system, source_kind)
    term_graphs = implementation_term_graphs(
        db_path,
        query,
        evidence,
        limit,
        workspace=workspace,
    )
    rust_touchpoints = merge_rust_touchpoints(
        term_graphs,
        direct_rust_touchpoints([*handoff_candidates, *evidence]),
        limit,
        workspace=workspace,
    )
    authority_clusters = authoritative_doc_clusters(
        db_path,
        query,
        evidence,
        handoff_candidates,
        rust_touchpoints,
        limit,
        system=system,
        source_kind=source_kind,
    )
    matched = bool(
        evidence
        or handoff_candidates
        or authority_clusters["trust_first"]
        or authority_clusters["supporting"]
        or authority_clusters["risky"]
    )
    warnings = handoff_warnings(
        evidence,
        handoff_candidates,
        rust_touchpoints,
        matched=matched,
    )

    return {
        "query": query,
        "system": system,
        "source_kind": source_kind,
        "matched": matched,
        "evidence": evidence,
        "handoff_candidates": handoff_candidates,
        "authority_clusters": authority_clusters,
        "implementation_terms": term_graphs,
        "rust_touchpoints": rust_touchpoints,
        "warnings": warnings,
    }


def authoritative_doc_clusters(
    db_path: Path,
    query: str,
    evidence: list[dict],
    handoff_candidates: list[dict],
    rust_touchpoints: list[dict],
    limit: int,
    system: str | None = None,
    source_kind: str | None = None,
) -> dict:
    anchors = query_anchors(query)
    candidates: dict[int, dict] = {}
    conn = connect(db_path)
    try:
        expanded_evidence = meaningful_search_results(
            search(
                db_path,
                query,
                limit=max(40, limit * 8),
                system=system,
                source_kind=source_kind,
            ),
            query,
        )
        for row in expanded_evidence:
            entry = doc_entry(conn, candidates, int(row["document_id"]))
            if entry is None:
                continue
            entry["lexical_hits"] += query_overlap_score(row, {term.lower() for term in exact_term_set(query)})
            entry["citations"].add(f"{row['path']}:{row['start_line']}-{row['end_line']}")
            entry["best_snippets"].append(row["snippet"])

        for row in evidence:
            entry = doc_entry(conn, candidates, int(row["document_id"]))
            if entry is None:
                continue
            entry["evidence_hits"] += 1
            entry["lexical_hits"] += query_overlap_score(row, {term.lower() for term in exact_term_set(query)})
            entry["citations"].add(f"{row['path']}:{row['start_line']}-{row['end_line']}")
            entry["best_snippets"].append(row["snippet"])

        for row in handoff_candidates:
            entry = doc_entry(conn, candidates, int(row["document_id"]))
            if entry is None:
                continue
            entry["has_handoff"] = True
            entry["handoff_hits"] += 1
            entry["citations"].add(f"{row['path']}:{row['start_line']}-{row['end_line']}")
            entry["best_snippets"].append(row["snippet"])

        for rust_path in rust_touchpoints:
            for citation in rust_path.get("citations", []):
                path = citation.split(":", 1)[0]
                doc_id = doc_id_for_path(conn, path)
                if doc_id is None:
                    continue
                entry = doc_entry(conn, candidates, doc_id)
                if entry is None:
                    continue
                entry["rust_paths"].add(rust_path["rust_path"])
                entry["citations"].add(citation)

        for hit in anchor_document_hits(conn, anchors, limit=max(80, limit * 16), system=system, source_kind=source_kind):
            entry = doc_entry(conn, candidates, hit["document_id"])
            if entry is None:
                continue
            anchor_label = f"{hit['edge_kind'].replace('mentions_', '')}:{hit['target']}"
            entry["anchors"].add(anchor_label)
            if hit["exact"]:
                entry["exact_anchor_hits"] += 1
            else:
                entry["partial_anchor_hits"] += 1
            if hit["source_start_line"] is not None and hit["source_end_line"] is not None:
                entry["citations"].add(f"{entry['path']}:{hit['source_start_line']}-{hit['source_end_line']}")

        docs = finalize_authority_docs(conn, list(candidates.values()), query)
    finally:
        conn.close()

    non_risky = [doc for doc in docs if not doc["hard_risk"]]
    high_confidence_count = sum(1 for doc in non_risky if doc["score"] >= 8.0)
    trust_count = min(len(non_risky), max(1, min(5, high_confidence_count)))
    trust_first = non_risky[:trust_count]
    supporting = non_risky[trust_count : trust_count + limit]
    risky = [doc for doc in docs if doc["hard_risk"]][:limit]

    matched_anchors = sorted(
        {
            anchor
            for doc in [*trust_first, *supporting, *risky]
            for anchor in doc.get("anchors", [])
        }
    )
    return {
        "anchors": anchors,
        "matched_anchors": matched_anchors,
        "trust_first": trust_first,
        "supporting": supporting,
        "risky": risky,
        "confidence_notes": confidence_notes(
            anchors,
            matched_anchors,
            trust_first,
            supporting,
            risky,
        ),
    }


def query_anchors(query: str) -> list[str]:
    anchors = set()
    anchors.update(match.group(1) for match in INI_QUERY_RE.finditer(query))
    for term in exact_term_set(query):
        if term.startswith("0x") or term.startswith("src/") or is_specific_term(term):
            anchors.add(term)
    return sorted(anchors, key=term_sort_key)


def meaningful_search_results(rows: list[dict], query: str) -> list[dict]:
    terms = informative_query_terms(query)
    required_hits = required_query_hits(terms)
    if required_hits == 0:
        return []
    return [
        row
        for row in rows
        if int(row.get("query_hit_count", 0)) >= required_hits
    ]


def anchor_document_hits(conn, anchors: list[str], limit: int, system: str | None, source_kind: str | None) -> list[dict]:
    if not anchors:
        return []
    placeholders = ",".join("?" for _ in AUTHORITY_EDGE_KINDS)
    filters = ""
    params_tail: list[object] = []
    if system:
        filters += " AND d.system = ?"
        params_tail.append(system)
    if source_kind:
        filters += " AND d.source_kind = ?"
        params_tail.append(source_kind)

    hits = []
    for anchor in anchors[:16]:
        address = normalized_address(anchor)
        address_clause = ""
        address_params: list[object] = []
        if address is not None:
            address_clause = (
                " OR (e.edge_kind = 'mentions_address' "
                "AND ltrim(substr(lower(e.target), 3), '0') = ?)"
            )
            address_params.append(address)
        rows = conn.execute(
            f"""
            SELECT d.id AS document_id, e.edge_kind, e.target, e.source_start_line, e.source_end_line,
                   CASE WHEN lower(e.target) = lower(?) THEN 1 ELSE 0 END AS exact
            FROM edges e
            JOIN documents d ON d.id = e.source_document_id
            WHERE e.edge_kind IN ({placeholders})
              AND (lower(e.target) = lower(?) OR e.target LIKE ? {address_clause})
              {filters}
            ORDER BY exact DESC, e.weight DESC, d.path
            LIMIT ?
            """,
            (
                anchor,
                *AUTHORITY_EDGE_KINDS,
                anchor,
                f"%{anchor}%",
                *address_params,
                *params_tail,
                limit,
            ),
        )
        hits.extend(dict(row) for row in rows)
    return hits


def doc_id_for_path(conn, path: str) -> int | None:
    row = conn.execute("SELECT id FROM documents WHERE path = ?", (path,)).fetchone()
    return None if row is None else int(row["id"])


def doc_entry(conn, candidates: dict[int, dict], doc_id: int) -> dict | None:
    if doc_id in candidates:
        return candidates[doc_id]
    row = conn.execute("SELECT * FROM documents WHERE id = ?", (doc_id,)).fetchone()
    if row is None:
        return None
    entry = {
        "document_id": doc_id,
        "path": row["path"],
        "title": row["title"],
        "system": row["system"],
        "subsystem": row["subsystem"],
        "source_kind": row["source_kind"],
        "status": row["status"],
        "modified_time": float(row["modified_time"]),
        "anchors": set(),
        "rust_paths": set(),
        "citations": set(),
        "best_snippets": [],
        "evidence_hits": 0,
        "handoff_hits": 0,
        "lexical_hits": 0,
        "exact_anchor_hits": 0,
        "partial_anchor_hits": 0,
        "has_handoff": False,
    }
    candidates[doc_id] = entry
    return entry


def finalize_authority_docs(conn, docs: list[dict], query: str) -> list[dict]:
    if not docs:
        return []
    newest = max(doc["modified_time"] for doc in docs)
    oldest = min(doc["modified_time"] for doc in docs)
    span = max(newest - oldest, 1.0)
    risk_texts = {doc["document_id"]: risk_signal_text(conn, doc["document_id"]) for doc in docs}
    correction_targets = correction_target_basenames(risk_texts.values())
    finalized = []
    for doc in docs:
        risk_text = risk_texts[doc["document_id"]]
        risk_flags = authority_risk_flags(doc, risk_text, Path(doc["path"]).name.upper() in correction_targets)
        correction_signal = has_correction_signal(risk_text)
        recency_bonus = 0.35 * ((doc["modified_time"] - oldest) / span)
        score = authority_score(
            doc["source_kind"],
            doc["status"],
            doc["exact_anchor_hits"],
            doc["partial_anchor_hits"],
            doc["lexical_hits"],
            doc["has_handoff"],
            correction_signal,
            risk_flags,
            recency_bonus,
        )
        hard_risk = "stale/superseded" in risk_flags or "legacy gate wording" in risk_flags
        finalized.append(
            {
                "path": doc["path"],
                "title": doc["title"],
                "source_kind": doc["source_kind"],
                "status": doc["status"],
                "score": round(score, 3),
                "anchors": sorted(doc["anchors"])[:8],
                "rust_paths": sorted(doc["rust_paths"])[:5],
                "citations": sorted(doc["citations"])[:5],
                "risk_flags": risk_flags,
                "hard_risk": hard_risk,
                "has_handoff": doc["has_handoff"],
                "notes": doc_confidence_notes(doc, risk_flags, correction_signal, query),
            }
        )
    finalized.sort(key=lambda row: (row["hard_risk"], -row["score"], row["path"]))
    return finalized


def risk_signal_text(conn, doc_id: int) -> str:
    terms = [f"%{term}%" for term in RISK_TERMS]
    clauses = " OR ".join("lower(c.text) LIKE ?" for _ in terms)
    rows = conn.execute(
        f"""
        SELECT c.heading_path, c.text
        FROM chunks c
        WHERE c.document_id = ? AND ({clauses})
        LIMIT 8
        """,
        (doc_id, *terms),
    )
    return "\n".join(f"{row['heading_path']}\n{row['text']}" for row in rows)


def correction_target_basenames(texts) -> set[str]:
    targets = set()
    for text in texts:
        for match in re.finditer(r"([A-Za-z0-9_./\\-]+\.md)[^.\n]{0,80}replacement wording", text, re.IGNORECASE):
            targets.add(Path(match.group(1).replace("\\", "/")).name.upper())
    return targets


def authority_risk_flags(doc: dict, text: str, is_correction_target: bool = False) -> list[str]:
    path_title_status = f"{doc['path']} {doc['title']} {doc['status']}".lower()
    context = f"{path_title_status} {text}".lower()
    flags = []
    if doc["status"] == "stale" or is_correction_target or "superseded" in path_title_status or "stale" in path_title_status:
        flags.append("stale/superseded")
    if "wrong" in path_title_status or "misleading" in path_title_status or (is_correction_target and ("wrong" in context or "misleading" in context)):
        flags.append("wrong/misleading wording")
    if any(term in context for term in ("unchecked", "deferred", "uncertainty", "not traced", "unknown")):
        flags.append("unchecked/deferred uncertainty")
    if "correction" in context or "corrected" in context or "replacement wording" in context:
        flags.append("correction notes")
    if "unit_can_enter_cell_ghidra_report" in path_title_status and ("canbegarrisoned" in context or "isgate=" in context):
        flags.append("legacy gate wording")
    return flags


def has_correction_signal(text: str) -> bool:
    lowered = text.lower()
    return "replacement wording" in lowered or "corrected" in lowered or "[resolved]" in lowered


def doc_confidence_notes(doc: dict, risk_flags: list[str], correction_signal: bool, query: str) -> list[str]:
    notes = []
    if doc["exact_anchor_hits"]:
        notes.append(f"{doc['exact_anchor_hits']} exact anchor hits")
    if doc["has_handoff"]:
        notes.append("has implementation handoff")
    if correction_signal:
        notes.append("contains correction/resolution notes")
    if risk_flags:
        notes.append("risk: " + ", ".join(risk_flags))
    if not notes:
        notes.append("matched query context")
    return notes


def confidence_notes(
    anchors: list[str],
    matched_anchors: list[str],
    trust_first: list[dict],
    supporting: list[dict],
    risky: list[dict],
) -> list[str]:
    notes = []
    if matched_anchors:
        notes.append("Strong anchors matched: " + ", ".join(matched_anchors[:8]))
    elif anchors:
        notes.append("Recognized query anchors had no indexed edge hits: " + ", ".join(anchors[:8]))
    if trust_first:
        verified = sum(1 for doc in trust_first if doc["status"] == "verified")
        notes.append(f"{verified}/{len(trust_first)} trust-first docs are verified-status evidence.")
    if risky:
        notes.append("Risky docs matched the query but carry stale, superseded, or legacy-wording signals.")
    if not trust_first and supporting:
        notes.append("No high-confidence primary cluster found; supporting docs need manual verification.")
    return notes


def implementation_handoff_candidates(
    db_path: Path,
    query: str,
    limit: int,
    system: str | None = None,
    source_kind: str | None = None,
) -> list[dict]:
    rows = meaningful_search_results(
        search(
            db_path,
            query,
            limit=max(40, limit * 8),
            system=system,
            source_kind=source_kind,
        ),
        query,
    )
    filtered = [
        row
        for row in rows
        if any(marker in row["text"].lower() for marker in HANDOFF_MARKERS)
    ]
    filtered.sort(
        key=lambda row: (
            int(row.get("query_hit_count", 0)),
            float(row["score"]),
        ),
        reverse=True,
    )
    return dedupe_rows(filtered)[:limit]


def query_overlap_score(row: dict, query_terms: set[str]) -> int:
    if not query_terms:
        return 1
    haystack = f"{row['path']} {row['title']} {row['heading_path']} {row['text']}".lower()
    return sum(1 for term in query_terms if term in haystack)


def handoff_warnings(
    evidence: list[dict],
    handoff_candidates: list[dict],
    rust_touchpoints: list[dict],
    matched: bool = True,
) -> list[str]:
    warnings = []
    if not matched:
        warnings.append("No relevant research scope matched the query; broaden it or remove filters.")
    if not evidence:
        warnings.append("No evidence chunks matched the query.")
    elif not any(row["status"] == "verified" for row in evidence[:5]):
        warnings.append("Top evidence is not verified; re-check binary or docs before implementation.")
    if not handoff_candidates:
        warnings.append("No explicit implementation handoff section found.")
    if not rust_touchpoints:
        warnings.append("No Rust touchpoints were found from extracted graph terms.")
    else:
        missing_count = sum(
            1
            for row in rust_touchpoints
            if row.get("exists") is False
        )
        if missing_count:
            warnings.append(
                f"{missing_count} Rust touchpoint(s) do not exist in the current "
                "workspace; they may be stale citations or planned paths."
            )
    return warnings


def dedupe_rows(rows: list[dict]) -> list[dict]:
    seen = set()
    deduped = []
    for row in rows:
        key = (row["path"], row["start_line"], row["end_line"])
        if key in seen:
            continue
        seen.add(key)
        deduped.append(row)
    return deduped
