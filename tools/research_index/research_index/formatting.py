"""Human-readable output formatting."""

from __future__ import annotations


HANDOFF_TRUST_LIMIT = 3
HANDOFF_SUPPORTING_LIMIT = 2
HANDOFF_RISKY_LIMIT = 1
HANDOFF_CANDIDATE_LIMIT = 2
HANDOFF_RUST_LIMIT = 5
HANDOFF_EVIDENCE_LIMIT = 4
HANDOFF_TERM_LIMIT = 4
HANDOFF_SNIPPET_WIDTH = 200


def format_search_results(rows: list[dict]) -> str:
    if not rows:
        return "No results."

    lines: list[str] = []
    for index, row in enumerate(rows, start=1):
        lines.append(
            f"{index}. {row['path']}:{row['start_line']}-{row['end_line']} "
            f"[{row['source_kind']}/{row['status']}] score={row['score']}"
        )
        lines.append(f"   heading: {row['heading_path']}")
        lines.append(f"   {row['snippet']}")
        lines.append("")
    return "\n".join(lines).rstrip()


def format_related_results(rows: list[dict]) -> str:
    if not rows:
        return "No related documents."

    lines: list[str] = []
    for index, row in enumerate(rows, start=1):
        lines.append(
            f"{index}. {row['path']} [{row['source_kind']}/{row['status']}] "
            f"matches={row['match_count']} score={row.get('related_score', 0)}"
        )
        lines.append(f"   title: {row['title']}")
        preview = ", ".join(f"{match['kind']}:{match['term']}" for match in row["matches"][:8])
        if len(row["matches"]) > 8:
            preview += ", ..."
        lines.append(f"   {preview}")
        lines.append("")
    return "\n".join(lines).rstrip()


def format_document_graph(result: dict) -> str:
    if not result.get("found"):
        return f"Document not found: {result['target']}"

    doc = result["document"]
    lines = [
        f"Document: {doc['path']}",
        f"title: {doc['title']}",
        f"metadata: system={doc['system']} subsystem={doc['subsystem']} source={doc['source_kind']} status={doc['status']}",
        "",
        "Outgoing:",
    ]

    for edge_kind, rows in result["outgoing"].items():
        lines.append(f"  {edge_kind}:")
        for row in rows:
            target_doc = " doc" if row["target_document_id"] is not None else ""
            lines.append(f"    - {row['target']}{target_doc}{line_suffix(row)} weight={row['weight']}")

    lines.append("")
    lines.append("Incoming:")
    for row in result["incoming"]:
        lines.append(f"  - {row['path']}{line_suffix(row)} [{row['source_kind']}/{row['status']}] via {row['edge_kind']}")

    return "\n".join(lines).rstrip()


def format_backlinks(result: dict) -> str:
    if not result.get("found"):
        return f"Document not found: {result['target']}"

    lines = [f"Backlinks: {result['document']['path']}"]
    for row in result["incoming"]:
        lines.append(f"  - {row['path']}{line_suffix(row)} [{row['source_kind']}/{row['status']}] via {row['edge_kind']}")
    return "\n".join(lines).rstrip()


def format_graph_view(result: dict) -> str:
    lines = [f"{result['mode'].title()} graph: {result['target']}"]

    if result["documents"]:
        lines.append("")
        lines.append("Documents:")
        for index, row in enumerate(result["documents"], start=1):
            lines.append(
                f"{index}. {row['path']} [{row['source_kind']}/{row['status']}] "
                f"matches={row['match_count']}"
            )
            preview = ", ".join(row["matches"][:6])
            if len(row["matches"]) > 6:
                preview += ", ..."
            lines.append(f"   {preview}")
            if row.get("line_ranges"):
                ranges = ", ".join(row["line_ranges"][:6])
                if len(row["line_ranges"]) > 6:
                    ranges += ", ..."
                lines.append(f"   lines: {ranges}")

    if result["rust_paths"]:
        lines.append("")
        lines.append("Rust touchpoints:")
        for row in result["rust_paths"]:
            lines.append(
                f"  - {row['rust_path']} docs={row['doc_count']}"
                f"{existence_suffix(row)}"
            )
            if row.get("citations"):
                preview = ", ".join(row["citations"][:4])
                if len(row["citations"]) > 4:
                    preview += ", ..."
                lines.append(f"    citations: {preview}")

    if result.get("fallback_documents"):
        lines.append("")
        lines.append("Full-text fallback:")
        for index, row in enumerate(result["fallback_documents"], start=1):
            lines.append(
                f"{index}. {row['path']}:{row['start_line']}-{row['end_line']} "
                f"[{row['source_kind']}/{row['status']}] score={row['score']}"
            )
            lines.append(f"   heading: {row['heading_path']}")
            lines.append(f"   {row['snippet']}")

    if not result["documents"] and not result["edges"] and not result.get("fallback_documents"):
        lines.append("No graph matches.")

    return "\n".join(lines).rstrip()


def format_parity_handoff(result: dict) -> str:
    scope = []
    if result.get("system"):
        scope.append(f"system={result['system']}")
    if result.get("source_kind"):
        scope.append(f"source={result['source_kind']}")
    suffix = f" ({', '.join(scope)})" if scope else ""
    lines = [
        f"Parity handoff: {result['query']}{suffix}",
        f"matched: {result.get('matched', bool(result.get('evidence')))}",
    ]

    if result["warnings"]:
        lines.append("")
        lines.append("Warnings:")
        for warning in result["warnings"]:
            lines.append(f"  - {warning}")

    clusters = result.get("authority_clusters")
    if clusters:
        append_authority_section(
            lines,
            "Trust these first",
            clusters["trust_first"],
            HANDOFF_TRUST_LIMIT,
        )
        append_authority_section(
            lines,
            "Supporting docs",
            clusters["supporting"],
            HANDOFF_SUPPORTING_LIMIT,
        )
        append_authority_section(
            lines,
            "Risky / superseded docs",
            clusters["risky"],
            HANDOFF_RISKY_LIMIT,
        )
        if clusters["confidence_notes"]:
            lines.append("")
            lines.append("Confidence notes:")
            for note in clusters["confidence_notes"]:
                lines.append(f"  - {note}")

    if result["handoff_candidates"]:
        lines.append("")
        lines.append("Implementation handoff candidates:")
        shown = result["handoff_candidates"][:HANDOFF_CANDIDATE_LIMIT]
        for index, row in enumerate(shown, start=1):
            lines.append(
                f"{index}. {row['path']}:{row['start_line']}-{row['end_line']} "
                f"[{row['source_kind']}/{row['status']}] score={row['score']}"
            )
            lines.append(f"   heading: {row['heading_path']}")
            lines.append(f"   {shorten(row['snippet'], HANDOFF_SNIPPET_WIDTH)}")
        append_omitted_count(
            lines,
            len(result["handoff_candidates"]),
            len(shown),
        )

    if result["rust_touchpoints"]:
        lines.append("")
        lines.append("Rust touchpoints:")
        shown = result["rust_touchpoints"][:HANDOFF_RUST_LIMIT]
        for row in shown:
            terms = ", ".join(row["terms"][:4])
            lines.append(
                f"  - {row['rust_path']} docs={row['doc_count']} terms={terms}"
                f"{existence_suffix(row)}"
            )
            if row["citations"]:
                citations = ", ".join(row["citations"][:2])
                if len(row["citations"]) > 2:
                    citations += ", ..."
                lines.append(f"    citations: {citations}")
        append_omitted_count(
            lines,
            len(result["rust_touchpoints"]),
            len(shown),
        )

    if result["evidence"]:
        lines.append("")
        lines.append("Top evidence:")
        shown = result["evidence"][:HANDOFF_EVIDENCE_LIMIT]
        for index, row in enumerate(shown, start=1):
            lines.append(
                f"{index}. {row['path']}:{row['start_line']}-{row['end_line']} "
                f"[{row['source_kind']}/{row['status']}] score={row['score']}"
            )
            lines.append(f"   heading: {row['heading_path']}")
            lines.append(f"   {shorten(row['snippet'], HANDOFF_SNIPPET_WIDTH)}")
        append_omitted_count(lines, len(result["evidence"]), len(shown))

    if result["implementation_terms"]:
        lines.append("")
        lines.append("Implementation graph terms:")
        shown = result["implementation_terms"][:HANDOFF_TERM_LIMIT]
        for graph in shown:
            doc_count = len(graph["documents"])
            rust_count = len(graph["rust_paths"])
            lines.append(f"  - {graph['term']}: docs={doc_count} rust_paths={rust_count}")
        append_omitted_count(
            lines,
            len(result["implementation_terms"]),
            len(shown),
        )

    return "\n".join(lines).rstrip()


def append_authority_section(
    lines: list[str],
    title: str,
    rows: list[dict],
    limit: int,
) -> None:
    if not rows:
        return
    lines.append("")
    lines.append(f"{title}:")
    shown = rows[:limit]
    for index, row in enumerate(shown, start=1):
        lines.append(f"{index}. {row['path']} [{row['source_kind']}/{row['status']}] confidence={row['score']}")
        if row.get("anchors"):
            lines.append(f"   anchors: {', '.join(row['anchors'][:5])}")
        if row.get("notes"):
            lines.append(f"   notes: {'; '.join(row['notes'][:3])}")
        if row.get("citations"):
            citations = ", ".join(row["citations"][:2])
            if len(row["citations"]) > 2:
                citations += ", ..."
            lines.append(f"   citations: {citations}")
    append_omitted_count(lines, len(rows), len(shown))


def format_system_map(result: dict) -> str:
    scope = []
    if result.get("system"):
        scope.append(f"system={result['system']}")
    if result.get("topic"):
        scope.append(f"topic={result['topic']}")
    if result.get("source_kind"):
        scope.append(f"source={result['source_kind']}")
    if result.get("status"):
        scope.append(f"status={result['status']}")
    title = "Research map"
    if scope:
        title += f" ({', '.join(scope)})"
    lines = [
        title,
        f"documents: {result['document_count']}",
        f"matched: {result.get('matched', result['document_count'] > 0)}",
    ]

    if not result.get("matched", result["document_count"] > 0):
        lines.extend(
            [
                "",
                "No documents matched the requested scope. Broaden the topic "
                "or remove filters.",
            ]
        )
        return "\n".join(lines)

    if result["groups"]:
        lines.append("")
        lines.append("Groups:")
        for row in result["groups"]:
            lines.append(f"  - {row['subsystem']} [{row['source_kind']}/{row['status']}] docs={row['count']}")

    if result["documents"]:
        lines.append("")
        lines.append("Documents:")
        for index, row in enumerate(result["documents"], start=1):
            lines.append(
                f"{index}. {row['path']} [{row['source_kind']}/{row['status']}] "
                f"chunks={row['matching_chunks']}"
            )
            lines.append(f"   title: {row['title']}")

    if result["handoff_sections"]:
        lines.append("")
        lines.append("Implementation handoff sections:")
        for index, row in enumerate(result["handoff_sections"], start=1):
            lines.append(f"{index}. {row['path']}:{row['start_line']}-{row['end_line']} [{row['source_kind']}/{row['status']}]")
            lines.append(f"   heading: {row['heading_path']}")
            lines.append(f"   {row['snippet']}")

    if result["signals"]:
        lines.append("")
        lines.append("Contradiction / uncertainty signals:")
        for index, row in enumerate(result["signals"], start=1):
            lines.append(f"{index}. {row['path']}:{row['start_line']}-{row['end_line']} [{row['source_kind']}/{row['status']}]")
            lines.append(f"   heading: {row['heading_path']}")
            lines.append(f"   {row['snippet']}")

    return "\n".join(lines).rstrip()


def format_validation(result: dict) -> str:
    scope = []
    if result.get("system"):
        scope.append(f"system={result['system']}")
    if result.get("topic"):
        scope.append(f"topic={result['topic']}")
    suffix = f" ({', '.join(scope)})" if scope else ""
    lines = [
        f"Research index validation{suffix}",
        f"documents checked: {result['documents_checked']}",
        f"scope matched: {result.get('scope_matched', result['documents_checked'] > 0)}",
        f"valid: {result['valid']}",
    ]
    counts = result["counts"]
    lines.append(
        "issues: "
        f"missing_files={counts['missing_files']} "
        f"checksum_mismatches={counts['checksum_mismatches']} "
        f"missing_links={counts['missing_links']} "
        f"stale_or_unknown={counts['stale_or_unknown']}"
    )

    append_validation_section(lines, "Missing files", result["missing_files"], "path")
    append_validation_section(lines, "Checksum mismatches", result["checksum_mismatches"], "path")
    append_validation_section(lines, "Missing links", result["missing_links"], "target")
    append_validation_section(lines, "Stale or unknown docs", result["stale_or_unknown"], "path")
    return "\n".join(lines).rstrip()


def append_validation_section(lines: list[str], title: str, rows: list[dict], detail_key: str) -> None:
    if not rows:
        return
    lines.append("")
    lines.append(f"{title}:")
    for row in rows:
        detail = row[detail_key]
        if detail_key == "target":
            lines.append(f"  - {row['path']} -> {detail} [{row['source_kind']}/{row['status']}]")
        else:
            lines.append(f"  - {detail} [{row['source_kind']}/{row['status']}]")


def format_research_brief(result: dict) -> str:
    scope = []
    if result.get("system"):
        scope.append(f"system={result['system']}")
    if result.get("source_kind"):
        scope.append(f"source={result['source_kind']}")
    suffix = f" ({', '.join(scope)})" if scope else ""
    lines = [f"Pre-implementation brief: {result['query']}{suffix}"]

    validation = result["validation"]
    counts = validation["counts"]
    lines.append(
        f"validation: valid={validation['valid']} "
        f"scope_matched={validation.get('scope_matched', validation['documents_checked'] > 0)} "
        f"docs={validation['documents_checked']} "
        f"missing={counts['missing_files']} changed={counts['checksum_mismatches']} "
        f"links={counts['missing_links']} stale_or_unknown={counts['stale_or_unknown']}"
    )

    research_map = result["map"]
    lines.append(f"map: matching_docs={research_map['document_count']} groups={len(research_map['groups'])}")
    if research_map["signals"]:
        lines.append("")
        lines.append("Correction / uncertainty signals:")
        for row in research_map["signals"][:5]:
            lines.append(f"  - {row['path']}:{row['start_line']}-{row['end_line']} [{row['source_kind']}/{row['status']}]")
            lines.append(f"    {row['heading_path']}")

    handoff = result["handoff"]
    lines.append(f"handoff: matched={handoff.get('matched', bool(handoff['evidence']))}")
    if handoff["warnings"]:
        lines.append("")
        lines.append("Warnings:")
        for warning in handoff["warnings"]:
            lines.append(f"  - {warning}")

    if handoff["handoff_candidates"]:
        lines.append("")
        lines.append("Implementation handoff candidates:")
        for row in handoff["handoff_candidates"][:5]:
            lines.append(f"  - {row['path']}:{row['start_line']}-{row['end_line']} [{row['source_kind']}/{row['status']}]")
            lines.append(f"    {row['heading_path']}")

    if handoff["rust_touchpoints"]:
        lines.append("")
        lines.append("Rust touchpoints:")
        for row in handoff["rust_touchpoints"][:8]:
            terms = ", ".join(row["terms"][:3])
            lines.append(
                f"  - {row['rust_path']} docs={row['doc_count']} terms={terms}"
                f"{existence_suffix(row)}"
            )
            if row["citations"]:
                lines.append(f"    {row['citations'][0]}")

    if handoff["evidence"]:
        lines.append("")
        lines.append("Top evidence:")
        for row in handoff["evidence"][:5]:
            lines.append(f"  - {row['path']}:{row['start_line']}-{row['end_line']} [{row['source_kind']}/{row['status']}]")
            lines.append(f"    {row['heading_path']}")

    if result["anchors"]:
        lines.append("")
        lines.append("Exact anchors:")
        for anchor in result["anchors"]:
            lines.append(
                f"  - {anchor['anchor']}: evidence_docs={len(anchor['evidence_documents'])} "
                f"rust_paths={len(anchor['rust_paths'])}"
            )
            for doc in anchor["evidence_documents"][:3]:
                lines.append(f"    {doc['path']} [{doc['source_kind']}/{doc['status']}]")

    return "\n".join(lines).rstrip()


def line_suffix(row: dict) -> str:
    start = row.get("source_start_line")
    end = row.get("source_end_line")
    if start is None or end is None:
        return ""
    return f":{start}-{end}"


def existence_suffix(row: dict) -> str:
    if "exists" not in row:
        return ""
    return " exists=yes" if row["exists"] else " exists=no"


def shorten(value: str, width: int) -> str:
    compact = " ".join(value.split())
    if len(compact) <= width:
        return compact
    return compact[: width - 3].rstrip() + "..."


def append_omitted_count(lines: list[str], total: int, shown: int) -> None:
    omitted = total - shown
    if omitted > 0:
        lines.append(
            f"  ... {omitted} more omitted from text; use JSON or narrow the query."
        )
