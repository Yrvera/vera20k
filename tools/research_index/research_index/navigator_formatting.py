"""Bounded human-readable formatting for the unified navigator."""

from __future__ import annotations

from .formatting import append_omitted_count, format_research_brief


NAVIGATOR_CANDIDATE_LIMIT = 5
NAVIGATOR_ROUTE_LIMIT = 12
NAVIGATOR_WARNING_LIMIT = 8


def format_research_navigator(result: dict) -> str:
    """Render one bounded evidence-and-routing handoff."""

    lines = [
        f"Research navigator: {result['query']}",
        f"matched: {result['matched']}",
        (
            f"domains: research={result['research_matched']} "
            f"system_map={result['system_map']['matched']}"
        ),
    ]
    if result["warnings"]:
        lines.append("")
        lines.append("Warnings:")
        shown_warnings = result["warnings"][:NAVIGATOR_WARNING_LIMIT]
        lines.extend(f"  - {warning}" for warning in shown_warnings)
        append_omitted_count(
            lines,
            len(result["warnings"]),
            len(shown_warnings),
        )

    system_map = result["system_map"]
    summary = system_map["summary"]
    mapping = ", ".join(
        f"{state}={count}"
        for state, count in summary["mapping_freshness"].items()
    )
    lines.extend(
        [
            "",
            "System Map routing:",
            (
                f"  systems={summary['system_count']} "
                f"annotated={summary['annotated_systems']} "
                f"loops={summary['loop_count']} "
                f"edges={summary['typed_edge_count']} "
                f"warnings={summary['warning_count']}"
            ),
            f"  Rust mapping freshness: {mapping or '(none)'}",
        ]
    )

    selected_system = system_map.get("selected_system")
    if selected_system:
        system = selected_system["system"]
        freshness = system["freshness"]["rust_mapping_freshness"]["state"]
        lines.extend(
            [
                "",
                "Selected system:",
                f"  {system['id']} — {system['name']}",
                (
                    f"  mapping_freshness={freshness} "
                    f"baseline_parity={system['baseline_status']['parity']}"
                ),
                (
                    "  loops: "
                    + (", ".join(selected_system["loops"]) or "(none)")
                ),
                (
                    "  services: "
                    + (", ".join(selected_system["services"]) or "(none)")
                ),
            ]
        )

    selected_loop = system_map.get("selected_loop")
    if selected_loop:
        lines.extend(
            [
                "",
                "Selected player-visible loop:",
                f"  {selected_loop['id']} — {selected_loop['name']}",
                (
                    f"  owner={selected_loop['owner']} "
                    f"oracle={selected_loop['oracle']['status']}"
                ),
                (
                    "  route: "
                    + _format_navigator_route(
                        selected_loop.get("ordered_systems", [])
                    )
                ),
            ]
        )

    system_candidates = system_map["system_candidates"]
    lines.append("")
    lines.append(
        f"System candidates: {len(system_candidates)} "
        "(candidate only; not verified ownership)"
    )
    shown_systems = system_candidates[:NAVIGATOR_CANDIDATE_LIMIT]
    for index, candidate in enumerate(shown_systems, start=1):
        reasons = "; ".join(candidate["match_reasons"][:3])
        lines.append(
            f"{index}. {candidate['id']} — {candidate['name']} "
            f"score={candidate['score']} "
            f"coverage={candidate['query_coverage']:.3f}"
        )
        lines.append(
            f"   mapping={candidate['freshness']['rust_mapping']} "
            f"activity={candidate['baseline_status']['activity']}"
        )
        if reasons:
            lines.append(f"   why: {reasons}")
    if not shown_systems:
        lines.append("  No systems matched.")
    append_omitted_count(
        lines,
        len(system_candidates),
        len(shown_systems),
    )

    loop_candidates = system_map["loop_candidates"]
    lines.append("")
    lines.append(
        f"Loop candidates: {len(loop_candidates)} "
        "(candidate only; ordered route is navigation)"
    )
    shown_loops = loop_candidates[:NAVIGATOR_CANDIDATE_LIMIT]
    for index, candidate in enumerate(shown_loops, start=1):
        lines.append(
            f"{index}. {candidate['id']} — {candidate['name']} "
            f"score={candidate['score']} "
            f"coverage={candidate['query_coverage']:.3f}"
        )
        lines.append(
            f"   owner={candidate['owner']} "
            f"oracle={candidate['oracle_status']}"
        )
        lines.append(
            "   route: "
            + _format_navigator_route(candidate["ordered_systems"])
        )
    if not shown_loops:
        lines.append("  No loops matched.")
    append_omitted_count(lines, len(loop_candidates), len(shown_loops))

    lines.extend(["", format_research_brief(result["research"])])
    return "\n".join(lines).rstrip()


def _format_navigator_route(system_ids: list[str]) -> str:
    shown = system_ids[:NAVIGATOR_ROUTE_LIMIT]
    route = " -> ".join(shown) or "(none)"
    omitted = len(system_ids) - len(shown)
    if omitted:
        route += f" -> ... ({omitted} stages omitted; use JSON)"
    return route
