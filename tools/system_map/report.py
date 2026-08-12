"""Merge, query, and deterministically render System Map v2."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path

from .freshness import (
    build_freshness,
    build_mechanism_freshness,
    repository_state,
)
from .jsonio import sha256_file
from .model import (
    Diagnostic,
    MECHANISMS_PATH,
    REGISTRY_PATH,
    SCHEMA_VERSION,
    SOURCE_LOCK_PATH,
    TOPOLOGY_PATH,
    loop_stage_ids,
)


GENERATOR_ID = "vera20k-system-map"
GENERATOR_VERSION = "1.1.0"


def build_report(
    repo: Path,
    registry: dict,
    source_lock: dict,
    topology: dict,
    diagnostics: list[Diagnostic],
    *,
    mechanisms: dict,
) -> dict:
    """Build the complete navigation report without mutating canonical inputs."""

    metrics = routing_metrics(registry, topology)
    freshness = build_freshness(repo, registry, topology)
    systems: dict[str, dict] = {}
    annotations = topology.get("systems", {})
    for system_id, baseline in sorted(registry.get("systems", {}).items()):
        systems[system_id] = {
            **deepcopy(baseline),
            "freshness": freshness.get(system_id, {}),
            "routing_metrics": metrics[system_id],
            "topology": deepcopy(annotations.get(system_id, {})),
        }

    mechanism_source = mechanisms
    mechanism_freshness = build_mechanism_freshness(repo, mechanism_source)
    mechanism_blocks = {
        block_id: {
            "id": block_id,
            **deepcopy(block),
            "freshness": mechanism_freshness.get(block_id, {}),
        }
        for block_id, block in sorted(
            mechanism_source.get("blocks", {}).items()
        )
        if isinstance(block, dict)
    }
    loops = {
        loop_id: normalize_loop(loop_id, loop, systems)
        for loop_id, loop in sorted(topology.get("loops", {}).items())
    }
    _attach_loop_mechanisms(loops, mechanism_blocks)
    edges = sorted(
        deepcopy(topology.get("edges", [])),
        key=lambda edge: (
            str(edge.get("id", "")),
            str(edge.get("plane", "")),
            str(edge.get("kind", "")),
        ),
    )
    mechanism_edges = sorted(
        deepcopy(mechanism_source.get("edges", [])),
        key=lambda edge: (
            str(edge.get("id", "")),
            str(edge.get("plane", "")),
            str(edge.get("kind", "")),
        ),
    )
    provenance_inputs = {
        "registry": {
            "path": REGISTRY_PATH.as_posix(),
            "sha256": sha256_file(repo / REGISTRY_PATH),
        },
        "source_lock": {
            "path": SOURCE_LOCK_PATH.as_posix(),
            "sha256": sha256_file(repo / SOURCE_LOCK_PATH),
        },
        "topology": {
            "path": TOPOLOGY_PATH.as_posix(),
            "sha256": sha256_file(repo / TOPOLOGY_PATH),
        },
    }
    provenance_inputs["mechanisms"] = {
        "path": MECHANISMS_PATH.as_posix(),
        "sha256": sha256_file(repo / MECHANISMS_PATH),
    }
    return {
        "coupled_sets": deepcopy(topology.get("coupled_sets", [])),
        "diagnostics": [item.to_document() for item in sorted(diagnostics)],
        "edges": edges,
        "id_policy": deepcopy(topology.get("id_policy")),
        "legacy_slice_aliases": deepcopy(
            topology.get("legacy_slice_aliases", [])
        ),
        "loops": loops,
        "mechanism_edges": mechanism_edges,
        "mechanism_observed_at_commit": mechanism_source.get(
            "observed_at_commit"
        ),
        "mechanism_schema_version": mechanism_source.get("schema_version"),
        "mechanisms": mechanism_blocks,
        "observed_at_commit": topology.get("observed_at_commit"),
        "provenance": {
            "generator": {
                "id": GENERATOR_ID,
                "version": GENERATOR_VERSION,
            },
            "inputs": provenance_inputs,
        },
        "repository": repository_state(repo),
        "schema_version": SCHEMA_VERSION,
        "services": {
            key: deepcopy(value)
            for key, value in sorted(topology.get("services", {}).items())
        },
        "source_baseline": {
            "baseline_rust_snapshot": registry.get(
                "baseline_rust_snapshot"
            ),
            "source_lock": deepcopy(source_lock),
        },
        "systems": systems,
    }


def routing_metrics(registry: dict, topology: dict) -> dict[str, dict]:
    """Compute mapped connectivity counts, never a parity/progress score."""

    metrics = {
        system_id: {
            "incoming_edges": 0,
            "loop_required_by": 0,
            "loop_requires": 0,
            "loop_memberships": 0,
            "outgoing_edges": 0,
            "owned_loops": 0,
            "required_by": 0,
            "requires": 0,
            "service_memberships": 0,
        }
        for system_id in registry.get("systems", {})
    }
    for edge in topology.get("edges", []):
        if not isinstance(edge, dict):
            continue
        source = edge.get("from")
        target = edge.get("to")
        if source in metrics:
            metrics[source]["outgoing_edges"] += 1
            if edge.get("kind") == "requires":
                metrics[source]["requires"] += 1
            elif edge.get("kind") == "loop_requires":
                metrics[source]["loop_requires"] += 1
        if target in metrics:
            metrics[target]["incoming_edges"] += 1
            if edge.get("kind") == "requires":
                metrics[target]["required_by"] += 1
            elif edge.get("kind") == "loop_requires":
                metrics[target]["loop_required_by"] += 1

    for loop in topology.get("loops", {}).values():
        if not isinstance(loop, dict):
            continue
        owner = loop.get("owner")
        if owner in metrics:
            metrics[owner]["owned_loops"] += 1
        stage_ids, _ = loop_stage_ids(loop)
        for system_id in set(stage_ids):
            if system_id in metrics:
                metrics[system_id]["loop_memberships"] += 1

    for service in topology.get("services", {}).values():
        if not isinstance(service, dict):
            continue
        mapped = service.get("systems", service.get("gsi_ids", []))
        if not isinstance(mapped, list):
            continue
        for system_id in set(mapped):
            if system_id in metrics:
                metrics[system_id]["service_memberships"] += 1

    return metrics


def normalize_loop(loop_id: str, loop: dict, systems: dict[str, dict]) -> dict:
    """Add a stable ordered route while preserving loop-specific evidence."""

    result = deepcopy(loop)
    stage_ids, _ = loop_stage_ids(loop)
    result["id"] = loop_id
    result["ordered_systems"] = stage_ids
    result["ordered_system_names"] = [
        systems.get(system_id, {}).get("name", "UNKNOWN")
        for system_id in stage_ids
    ]
    return result


def _attach_loop_mechanisms(
    loops: dict[str, dict],
    mechanisms: dict[str, dict],
) -> None:
    """Expose mapped and explicitly unmapped loop stages without scoring them."""

    mapped: dict[str, dict[int, list[str]]] = {}
    for block_id, block in mechanisms.items():
        for membership in block.get("loop_memberships", []):
            if not isinstance(membership, dict):
                continue
            loop_id = membership.get("loop")
            if loop_id not in loops:
                continue
            for order in membership.get("stage_orders", []):
                if isinstance(order, int):
                    mapped.setdefault(loop_id, {}).setdefault(order, []).append(
                        block_id
                    )
    for loop_id, loop in loops.items():
        stage_ids, orders = loop_stage_ids(loop)
        normalized_orders = [
            order if order is not None else index + 1
            for index, order in enumerate(orders)
        ]
        stage_map = mapped.get(loop_id, {})
        loop["mechanism_stage_map"] = [
            {
                "mechanisms": sorted(stage_map.get(order, [])),
                "order": order,
                "system": system_id,
            }
            for order, system_id in zip(
                normalized_orders, stage_ids, strict=True
            )
            if stage_map.get(order)
        ]
        mechanism_ids = {
            block_id
            for block_ids in stage_map.values()
            for block_id in block_ids
        }
        loop["mechanisms"] = sorted(
            mechanism_ids,
            key=lambda block_id: (
                min(
                    order
                    for order, block_ids in stage_map.items()
                    if block_id in block_ids
                ),
                block_id,
            ),
        )
        # Untouched loops are simply not mapped by this pilot. Enumerating every
        # stage there would turn navigation metadata into a missingness backlog.
        loop["unmapped_mechanism_stage_orders"] = (
            [order for order in normalized_orders if order not in stage_map]
            if stage_map
            else []
        )


def owner_rows(report: dict, limit: int | None = None) -> list[dict]:
    rows = []
    for system_id, system in report.get("systems", {}).items():
        metrics = system["routing_metrics"]
        baseline = system["baseline_status"]
        freshness = system["freshness"]
        rows.append(
            {
                "baseline_activity": baseline["activity"],
                "baseline_freshness": freshness[
                    "baseline_status_freshness"
                ]["state"],
                "baseline_native_evidence": baseline["native_evidence"],
                "baseline_parity": baseline["parity"],
                "id": system_id,
                "mapping_freshness": freshness["rust_mapping_freshness"][
                    "state"
                ],
                "name": system["name"],
                **metrics,
            }
        )
    activity_rank = {
        "STOCK_ACTIVE": 0,
        "MODE_ACTIVE": 1,
        "CONTENT_CONDITIONAL": 2,
        "COMPILED_INACTIVE": 3,
        "UNKNOWN": 4,
        "GROUP_NODE": 5,
    }
    rows.sort(
        key=lambda row: (
            -int(row["owned_loops"] > 0),
            activity_rank.get(row["baseline_activity"], 99),
            -row["owned_loops"],
            -row["requires"],
            -row["required_by"],
            -row["loop_requires"],
            -row["loop_required_by"],
            -row["loop_memberships"],
            row["id"],
        )
    )
    return rows if limit is None else rows[:limit]


def show_system(report: dict, system_id: str) -> dict | None:
    system = report.get("systems", {}).get(system_id)
    if system is None:
        for alias in report.get("legacy_slice_aliases", []):
            if system_id not in {
                alias.get("legacy_id"),
                alias.get("slice_id"),
            }:
                continue
            targets = alias.get("canonical_systems", [])
            return {
                "canonical_systems": [
                    {
                        "id": target,
                        "name": report.get("systems", {})
                        .get(target, {})
                        .get("name", "UNKNOWN"),
                    }
                    for target in targets
                ],
                "legacy_alias": deepcopy(alias),
            }
        return None
    incoming = [
        edge for edge in report["edges"] if edge.get("to") == system_id
    ]
    outgoing = [
        edge for edge in report["edges"] if edge.get("from") == system_id
    ]
    loops = [
        loop_id
        for loop_id, loop in report["loops"].items()
        if system_id in loop.get("ordered_systems", [])
        or loop.get("owner") == system_id
    ]
    services = [
        slug
        for slug, service in report["services"].items()
        if system_id
        in service.get("systems", service.get("gsi_ids", []))
    ]
    mechanisms = [
        block_id
        for block_id, block in report.get("mechanisms", {}).items()
        if system_id == block.get("owner")
        or system_id in block.get("participants", [])
    ]
    return {
        "incoming_edges": incoming,
        "loops": loops,
        "mechanisms": mechanisms,
        "outgoing_edges": outgoing,
        "services": services,
        "system": {"id": system_id, **deepcopy(system)},
    }


def show_mechanism(report: dict, block_id: str) -> dict | None:
    """Return one mechanism with its same-namespace relationships."""

    block = report.get("mechanisms", {}).get(block_id)
    if not isinstance(block, dict):
        return None
    edges = report.get("mechanism_edges", [])
    return {
        "incoming_edges": [
            deepcopy(edge) for edge in edges if edge.get("to") == block_id
        ],
        "loops": [
            membership.get("loop")
            for membership in block.get("loop_memberships", [])
            if isinstance(membership, dict)
        ],
        "mechanism": deepcopy(block),
        "outgoing_edges": [
            deepcopy(edge) for edge in edges if edge.get("from") == block_id
        ],
    }


def stale_rows(
    report: dict,
    system_id: str | None = None,
    *,
    include_unmapped: bool = False,
) -> list[dict]:
    rows: list[dict] = []
    selected = (
        {system_id: report["systems"].get(system_id)}
        if system_id is not None
        else report.get("systems", {})
    )
    for key, system in selected.items():
        if not isinstance(system, dict):
            continue
        mapping = system["freshness"]["rust_mapping_freshness"]
        baseline = system["freshness"]["baseline_status_freshness"]
        if system_id is None and (
            mapping["state"] == "FRESH"
            or (mapping["state"] == "UNMAPPED" and not include_unmapped)
        ):
            continue
        rows.append(
            {
                "baseline_reasons": baseline["reasons"],
                "baseline_state": baseline["state"],
                "changed_paths": sorted(
                    set(mapping["changed_paths"] + baseline["changed_paths"])
                ),
                "dirty_paths": sorted(
                    set(mapping["dirty_paths"] + baseline["dirty_paths"])
                ),
                "id": key,
                "mapping_reasons": mapping["reasons"],
                "mapping_state": mapping["state"],
                "missing_paths": sorted(
                    set(mapping["missing_paths"] + baseline["missing_paths"])
                ),
                "name": system["name"],
            }
        )
    return sorted(rows, key=lambda row: (row["mapping_state"], row["id"]))


def render_markdown(report: dict) -> str:
    """Render the full map as a generated navigation document."""

    provenance = report["provenance"]
    generator = provenance["generator"]
    inputs = provenance["inputs"]
    annotated_count = sum(
        1 for system in report["systems"].values() if system.get("topology")
    )
    rust_mapped_count = sum(
        1
        for system in report["systems"].values()
        if system.get("freshness", {})
        .get("rust_mapping_freshness", {})
        .get("state")
        != "UNMAPPED"
    )
    lines = [
        "# VERA20k System Map v2",
        "",
        "> GENERATED from `docs/system-map/registry.v2.json`, "
        "`docs/system-map/topology.v2.json`, and "
        "`docs/system-map/mechanisms.v1.json`. Do not hand-edit this output.",
        "> Baseline matrix fields are historical source statements, not current "
        "parity or completion claims.",
        "",
        "## Provenance",
        "",
        "| Field | Value |",
        "|---|---|",
        f"| Generator | `{_md(generator['id'])}` "
        f"`{_md(generator['version'])}` |",
        f"| Registry SHA-256 | `{_md(inputs['registry']['sha256'])}` |",
        f"| Source lock SHA-256 | "
        f"`{_md(inputs['source_lock']['sha256'])}` |",
        f"| Topology SHA-256 | `{_md(inputs['topology']['sha256'])}` |",
        f"| Mechanisms SHA-256 | "
        f"`{_md(inputs['mechanisms']['sha256'])}` |",
        f"| Repository head | `{_md(report['repository']['head'])}` |",
        f"| Branch | `{_md(report['repository']['branch'])}` |",
        f"| Topology observed at | `{_md(str(report['observed_at_commit']))}` |",
        f"| Mechanisms observed at | "
        f"`{_md(str(report['mechanism_observed_at_commit']))}` |",
        f"| Status-matrix Rust baseline | "
        f"`{_md(str(report['source_baseline']['baseline_rust_snapshot']))}` |",
        f"| Dirty paths | {len(report['repository']['dirty_paths'])} |",
        f"| Registry systems | {len(report['systems'])} |",
        f"| Annotated systems | {annotated_count} |",
        f"| Rust-mapped systems | {rust_mapped_count} |",
        f"| Typed edges | {len(report['edges'])} |",
        f"| Player loops | {len(report['loops'])} |",
        f"| Mechanism blocks | {len(report['mechanisms'])} |",
        f"| Mechanism edges | {len(report['mechanism_edges'])} |",
        "",
        "Mapped connectivity is a routing aid only. It is biased toward the "
        "currently annotated portion of the graph and is not work priority or "
        "parity progress.",
        "",
        "## Player-visible production loops",
        "",
        "| Loop | Owner | Route | Visible result | Oracle status |",
        "|---|---|---|---|---|",
    ]
    for loop_id, loop in report["loops"].items():
        owner = str(loop.get("owner", "UNKNOWN"))
        route = " → ".join(loop.get("ordered_systems", []))
        visible = loop.get(
            "player_visible_result",
            loop.get(
                "player_visible_outcome",
                loop.get("visible_outcome", loop.get("visible_assertions", "")),
            ),
        )
        oracle = loop.get(
            "oracle",
            loop.get(
                "oracle_status",
                loop.get(
                    "evidence_oracle_status", loop.get("evidence_level", "")
                ),
            ),
        )
        lines.append(
            f"| `{_md(loop_id)}` | `{_md(owner)}` | {_md(route)} | "
            f"{_md(_compact(visible))} | {_md(_compact(oracle))} |"
        )

    lines.extend(
        [
            "",
            "## Semantic mechanism blocks",
            "",
            "Mechanisms connect native evidence, canonical GSI systems, Rust "
            "surfaces, and selected loop stages. They are navigation contracts, "
            "not parity or completion claims.",
            "",
            "| Block | Owner | Participants | Loops/stages | Rust freshness | Contract |",
            "|---|---|---|---|---|---|",
        ]
    )
    for block_id, block in report["mechanisms"].items():
        memberships = "; ".join(
            f"{item.get('loop')}:"
            + ",".join(str(order) for order in item.get("stage_orders", []))
            for item in block.get("loop_memberships", [])
            if isinstance(item, dict)
        )
        lines.append(
            f"| `{_md(block_id)}` | `{_md(str(block.get('owner', '')) )}` | "
            f"{_md(', '.join(block.get('participants', [])))} | "
            f"{_md(memberships)} | "
            f"`{_md(str(block.get('freshness', {}).get('state', 'UNMAPPED')) )}` | "
            f"{_md(_compact(block.get('contract', '')))} |"
        )

    lines.extend(
        [
            "",
            "## Typed mechanism edges",
            "",
            "| ID | Plane | Kind | From | To | Loop | Detail |",
            "|---|---|---|---|---|---|---|",
        ]
    )
    for edge in report["mechanism_edges"]:
        lines.append(
            f"| `{_md(str(edge.get('id', '')) )}` | "
            f"`{_md(str(edge.get('plane', '')) )}` | "
            f"`{_md(str(edge.get('kind', '')) )}` | "
            f"`{_md(str(edge.get('from', '')) )}` | "
            f"`{_md(str(edge.get('to', '')) )}` | "
            f"`{_md(str(edge.get('loop', '')) )}` | "
            f"{_md(_compact(edge.get('detail', '')))} |"
        )

    lines.extend(
        [
            "",
            "## Mapped owner/connectivity view",
            "",
            "| ID | Activity | Native evidence | Baseline parity | Mapping "
            "freshness | Baseline freshness | Owns loops | Loop membership | "
            "In | Out | Requires | Required by | Loop requires | Loop required "
            "by | Services | System |",
            "|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|"
            "---:|---:|---|",
        ]
    )
    for row in owner_rows(report, limit=40):
        lines.append(
            f"| `{row['id']}` | `{row['baseline_activity']}` | "
            f"`{row['baseline_native_evidence']}` | "
            f"`{row['baseline_parity']}` | `{row['mapping_freshness']}` | "
            f"`{row['baseline_freshness']}` | {row['owned_loops']} | "
            f"{row['loop_memberships']} | {row['incoming_edges']} | "
            f"{row['outgoing_edges']} | {row['requires']} | "
            f"{row['required_by']} | {row['loop_requires']} | "
            f"{row['loop_required_by']} | {row['service_memberships']} | "
            f"{_md(row['name'])} |"
        )

    lines.extend(
        [
            "",
            "## Core-service crosswalk",
            "",
            "| Service | Canonical GSI systems |",
            "|---|---|",
        ]
    )
    for slug, service in report["services"].items():
        mapped = ", ".join(
            f"`{item}`"
            for item in service.get("systems", service.get("gsi_ids", []))
        )
        lines.append(f"| `{_md(slug)}` | {mapped} |")

    lines.extend(
        [
            "",
            "## Typed system edges",
            "",
            "| ID | Plane | Kind | From | To | Context/detail |",
            "|---|---|---|---|---|---|",
        ]
    )
    for edge in report["edges"]:
        detail = edge.get("context", edge.get("detail", edge.get("reason", "")))
        lines.append(
            f"| `{_md(str(edge.get('id', '')) )}` | "
            f"`{_md(str(edge.get('plane', '')) )}` | "
            f"`{_md(str(edge.get('kind', '')) )}` | "
            f"`{_md(str(edge.get('from', '')) )}` | "
            f"`{_md(str(edge.get('to', '')) )}` | "
            f"{_md(_compact(detail))} |"
        )

    aliases = report.get("legacy_slice_aliases", [])
    if aliases:
        lines.extend(
            [
                "",
                "## Legacy slice aliases",
                "",
                "| Historical pseudo-GSI | Slice | Canonical systems | Reason |",
                "|---|---|---|---|",
            ]
        )
        for alias in aliases:
            canonical = ", ".join(
                f"`{item}`" for item in alias.get("canonical_systems", [])
            )
            lines.append(
                f"| `{_md(str(alias.get('legacy_id', '')) )}` | "
                f"`{_md(str(alias.get('slice_id', '')) )}` | {canonical} | "
                f"{_md(_compact(alias.get('reason', '')))} |"
            )

    lines.extend(["", "## Full canonical registry"])
    current_family = None
    for system_id, system in report["systems"].items():
        family = system["family"]
        if family != current_family:
            current_family = family
            lines.extend(
                [
                    "",
                    f"### {family} — {_md(system['family_name'])}",
                    "",
                    "| ID | System | Baseline activity | Baseline native | "
                    "Baseline Rust | Baseline parity | Mapping freshness | Loops |",
                    "|---|---|---|---|---|---|---|---:|",
                ]
            )
        baseline = system["baseline_status"]
        freshness = system["freshness"]["rust_mapping_freshness"]["state"]
        loops = system["routing_metrics"]["loop_memberships"]
        lines.append(
            f"| `{system_id}` | {_md(system['name'])} | "
            f"`{baseline['activity']}` | `{baseline['native_evidence']}` | "
            f"`{baseline['rust_implementation']}` | `{baseline['parity']}` | "
            f"`{freshness}` | {loops} |"
        )

    if report["diagnostics"]:
        lines.extend(
            [
                "",
                "## Validation diagnostics",
                "",
                "| Severity | Code | Record | Field | Message |",
                "|---|---|---|---|---|",
            ]
        )
        for item in report["diagnostics"]:
            lines.append(
                f"| `{item['severity']}` | `{item['code']}` | "
                f"`{_md(item['record_id'])}` | `{_md(item['field'])}` | "
                f"{_md(item['message'])} |"
            )
    lines.append("")
    return "\n".join(lines)


def format_system_view(view: dict) -> str:
    if "legacy_alias" in view:
        alias = view["legacy_alias"]
        lines = [
            f"{alias.get('legacy_id')} — historical pseudo-GSI alias",
            f"slice: {alias.get('slice_id')}",
            "canonical systems: "
            + ", ".join(
                f"{item['id']} ({item['name']})"
                for item in view["canonical_systems"]
            ),
        ]
        if alias.get("reason"):
            lines.append("reason: " + _compact(alias["reason"]))
        _append_items(lines, "evidence", alias.get("evidence", []))
        return "\n".join(lines) + "\n"

    system = view["system"]
    baseline = system["baseline_status"]
    freshness = system["freshness"]["rust_mapping_freshness"]
    topology = system.get("topology", {})
    lines = [
        f"{system['id']} — {system['name']}",
        f"family: {system['family']} — {system['family_name']}",
        "baseline (historical): "
        f"activity={baseline['activity']} native={baseline['native_evidence']} "
        f"rust={baseline['rust_implementation']} parity={baseline['parity']}",
        f"Rust mapping freshness: {freshness['state']} — "
        + "; ".join(freshness["reasons"]),
        f"services: {', '.join(view['services']) or '(none)'}",
        f"loops: {', '.join(view['loops']) or '(none)'}",
        f"mechanisms: {', '.join(view.get('mechanisms', [])) or '(none)'}",
        "incoming edges:",
    ]
    lines.extend(_edge_text(edge) for edge in view["incoming_edges"])
    if not view["incoming_edges"]:
        lines.append("  (none)")
    lines.append("outgoing edges:")
    lines.extend(_edge_text(edge) for edge in view["outgoing_edges"])
    if not view["outgoing_edges"]:
        lines.append("  (none)")
    _append_items(
        lines, "state authorities", topology.get("state_authorities", [])
    )
    _append_items(lines, "native anchors", topology.get("native_anchors", []))
    _append_items(lines, "Rust surfaces", topology.get("rust_surfaces", []))
    _append_items(lines, "notes", topology.get("notes", []))
    return "\n".join(lines) + "\n"


def format_loop_view(loop: dict) -> str:
    lines = [
        f"{loop['id']} — {loop.get('name', loop.get('title', 'unnamed loop'))}",
        f"owner: {loop.get('owner', 'UNKNOWN')}",
        "route: " + " → ".join(loop.get("ordered_systems", [])),
    ]
    fixture = loop.get("stock_fixture", loop.get("fixture"))
    if fixture is not None:
        lines.append("fixture: " + _compact(fixture))
    visible = loop.get(
        "player_visible_result",
        loop.get(
            "player_visible_outcome",
            loop.get("visible_outcome", loop.get("visible_assertions")),
        ),
    )
    if visible is not None:
        lines.append("visible result: " + _compact(visible))
    ordering_note = loop.get("ordering_note")
    if ordering_note is not None:
        lines.append("ordering note: " + _compact(ordering_note))
    oracle = loop.get(
        "oracle",
        loop.get(
            "oracle_status",
            loop.get("evidence_oracle_status", loop.get("evidence_level")),
        ),
    )
    if oracle is not None:
        lines.append("oracle status: " + _compact(oracle))
    if loop.get("mechanisms"):
        lines.append("mechanisms: " + ", ".join(loop["mechanisms"]))
    stage_map = loop.get("mechanism_stage_map", [])
    if stage_map:
        lines.append("mechanism stage mappings:")
        for item in stage_map:
            lines.append(
                "  "
                + str(item.get("order", "?"))
                + ". "
                + str(item.get("system", "?"))
                + " -> "
                + ", ".join(item.get("mechanisms", []))
            )
    unmapped = loop.get("unmapped_mechanism_stage_orders", [])
    if unmapped:
        lines.append(
            "mechanism-unmapped stages: "
            + ", ".join(str(order) for order in unmapped)
        )
    stages = loop.get("ordered_stages", loop.get("stages", []))
    if isinstance(stages, list) and stages:
        lines.append("stages:")
        for index, stage in enumerate(stages, start=1):
            if isinstance(stage, str):
                lines.append(f"  {index}. {stage}")
                continue
            if isinstance(stage, dict):
                order = stage.get("order", index)
                system_id = stage.get(
                    "system", stage.get("system_id", stage.get("id", "?"))
                )
                action = _compact(stage.get("action", ""))
                suffix = f" — {action}" if action else ""
                lines.append(f"  {order}. {system_id}{suffix}")
    _append_items(
        lines,
        "native entrypoints",
        loop.get(
            "native_anchors",
            loop.get(
                "verified_native_anchors",
                loop.get("native_entrypoints", []),
            ),
        ),
    )
    _append_items(lines, "Rust touchpoints", loop.get("rust_touchpoints", []))
    _append_items(lines, "evidence", loop.get("evidence", []))
    return "\n".join(lines) + "\n"


def format_mechanism_view(view: dict) -> str:
    """Format one semantic mechanism without implying parity completion."""

    block = view["mechanism"]
    activation = block.get("activation", {})
    freshness = block.get("freshness", {})
    lines = [
        f"{block['id']} - {block.get('name', 'unnamed mechanism')}",
        f"owner: {block.get('owner', 'UNKNOWN')}",
        "participants: " + ", ".join(block.get("participants", [])),
        "contract: " + _compact(block.get("contract", "")),
        "activation: " + _compact(activation),
        "Rust mapping freshness: "
        + str(freshness.get("state", "UNMAPPED"))
        + " - "
        + "; ".join(freshness.get("reasons", [])),
        "loops: " + ", ".join(view.get("loops", [])),
        "research query: " + _compact(block.get("research_query", "")),
        "steps:",
    ]
    for step in block.get("steps", []):
        if isinstance(step, dict):
            lines.append(
                f"  {step.get('order', '?')}. {step.get('system', '?')} "
                f"- {_compact(step.get('action', ''))}"
            )
    _append_items(lines, "inputs", block.get("inputs", []))
    _append_items(lines, "outputs", block.get("outputs", []))
    _append_items(
        lines, "loop-stage memberships", block.get("loop_memberships", [])
    )
    lines.append("incoming mechanism edges:")
    lines.extend(_edge_text(edge) for edge in view["incoming_edges"])
    if not view["incoming_edges"]:
        lines.append("  (none)")
    lines.append("outgoing mechanism edges:")
    lines.extend(_edge_text(edge) for edge in view["outgoing_edges"])
    if not view["outgoing_edges"]:
        lines.append("  (none)")
    _append_items(lines, "authority", block.get("authority", []))
    _append_items(
        lines, "critical semantics", block.get("critical_semantics", [])
    )
    _append_items(lines, "native anchors", block.get("native_anchors", []))
    _append_items(lines, "Rust surfaces", block.get("rust_surfaces", []))
    _append_items(lines, "open questions", block.get("open_questions", []))
    _append_items(lines, "evidence", block.get("evidence", []))
    return "\n".join(lines) + "\n"


def format_owner_rows(rows: list[dict]) -> str:
    lines = [
        "Mapped owner/connectivity view (routing coverage, not priority/parity):",
        "ID        ACTIVITY            NATIVE         PARITY         "
        "MAP         BASE        OWN LOOP IN OUT REQ BY LREQ LBY SVC  SYSTEM",
    ]
    for row in rows:
        lines.append(
            f"{row['id']:<9} {row['baseline_activity']:<19} "
            f"{row['baseline_native_evidence']:<14} "
            f"{row['baseline_parity']:<14} "
            f"{row['mapping_freshness']:<11} "
            f"{row['baseline_freshness']:<11} "
            f"{row['owned_loops']:>3} "
            f"{row['loop_memberships']:>4} {row['incoming_edges']:>2} "
            f"{row['outgoing_edges']:>3} {row['requires']:>3} "
            f"{row['required_by']:>2} {row['loop_requires']:>4} "
            f"{row['loop_required_by']:>3} "
            f"{row['service_memberships']:>3} "
            f" {row['name']}"
        )
    return "\n".join(lines) + "\n"


def format_stale_rows(rows: list[dict]) -> str:
    if not rows:
        return "No selected Rust mapping is stale.\n"
    lines = []
    for row in rows:
        lines.append(
            f"{row['id']} {row['mapping_state']} "
            f"(baseline {row['baseline_state']}) — {row['name']}"
        )
        for reason in row["mapping_reasons"]:
            lines.append(f"  mapping: {reason}")
        for reason in row["baseline_reasons"]:
            lines.append(f"  baseline: {reason}")
        for key in ("changed_paths", "dirty_paths", "missing_paths"):
            for path in row[key]:
                lines.append(f"  {key[:-1]}: {path}")
    return "\n".join(lines) + "\n"


def _edge_text(edge: dict) -> str:
    return (
        f"  {edge.get('id', '?')}: [{edge.get('plane', '?')}/"
        f"{edge.get('kind', '?')}] {edge.get('from', '?')} -> "
        f"{edge.get('to', '?')}"
    )


def _append_items(lines: list[str], label: str, values: object) -> None:
    if not isinstance(values, list) or not values:
        return
    lines.append(label + ":")
    lines.extend(f"  - {_compact(value)}" for value in values)


def _compact(value: object) -> str:
    if isinstance(value, str):
        return " ".join(value.split())
    if isinstance(value, list):
        return "; ".join(_compact(item) for item in value)
    if isinstance(value, dict):
        return "; ".join(
            f"{key}={_compact(item)}" for key, item in sorted(value.items())
        )
    if value is None:
        return ""
    return str(value)


def _md(value: str) -> str:
    return value.replace("|", "\\|").replace("\r", " ").replace("\n", " ")
