"""Small canonical fixtures shared by System Map tests."""

from __future__ import annotations

from pathlib import Path

from tools.system_map.jsonio import canonical_json_bytes, sha256_bytes


def registry_fixture() -> dict:
    baseline_atomic = {
        "activity": "STOCK_ACTIVE",
        "basis": "TEST",
        "inventory_evidence": "BOUNDED",
        "native_evidence": "ANCHORED",
        "parity": "UNCHECKED",
        "rust_implementation": "PARTIAL",
    }
    baseline_group = {
        "activity": "GROUP_NODE",
        "basis": "TEST-GROUP",
        "inventory_evidence": "GROUP_NODE",
        "native_evidence": "N/A",
        "parity": "N/A",
        "rust_implementation": "N/A",
    }
    return {
        "baseline_rust_snapshot": "1" * 40,
        "families": {
            "GSI-01": {
                "name": "Runtime",
                "systems": ["GSI-01.01", "GSI-01.02"],
            }
        },
        "schema_version": 2,
        "service_catalog": ["logicclass"],
        "systems": {
            "GSI-01.01": {
                "baseline_status": baseline_atomic,
                "discovery_scope": "stock",
                "family": "GSI-01",
                "family_name": "Runtime",
                "name": "tick owner",
                "source_line": 10,
            },
            "GSI-01.02": {
                "baseline_status": baseline_group,
                "discovery_scope": "mixed",
                "family": "GSI-01",
                "family_name": "Runtime",
                "name": "group node",
                "source_line": 11,
            },
        },
    }


def source_lock_fixture(registry: dict) -> dict:
    return {
        "baseline_rust_snapshot": registry["baseline_rust_snapshot"],
        "registry_sha256": sha256_bytes(canonical_json_bytes(registry)),
        "schema_version": 2,
        "sources": {
            role: {
                "path": f"docs/research/{role}.md",
                "sha256": "0" * 64,
            }
            for role in ("core_services", "inventory", "status_matrix")
        },
    }


def topology_fixture(repo: Path) -> dict:
    source = repo / "src/lib.rs"
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text("// fixture\n", encoding="utf-8")
    service_doc = repo / "docs/research/core_services.md"
    service_doc.parent.mkdir(parents=True, exist_ok=True)
    service_doc.write_text("| `logicclass` | fixture |\n", encoding="utf-8")
    return {
        "coupled_sets": [],
        "edges": [
            {
                "context": "test tick",
                "detail": "test route",
                "from": "GSI-01.01",
                "id": "EDGE-0001-TEST",
                "kind": "ordered_before",
                "plane": "routing",
                "to": "GSI-01.02",
            }
        ],
        "id_policy": {
            "canonical_system_pattern": "^GSI-[0-9]{2}\\.[0-9]{2}$",
            "edge_pattern": "^EDGE-[0-9]{4}-[A-Z0-9-]+$",
            "loop_pattern": "^LOOP-[0-9]{3}-[A-Z0-9-]+$",
            "rule": "Implementation slices never create canonical GSI IDs.",
            "slice_pattern": "^SLICE-[0-9]{8}-[A-Z0-9-]+$",
        },
        "legacy_slice_aliases": [],
        "loops": {
            "LOOP-001-TEST": {
                "evidence": ["docs/research/core_services.md:1"],
                "name": "test loop",
                "native_entrypoints": ["LogicClass::AI 0x0055AFB0"],
                "oracle": {
                    "gate": "No native executable fixture is recorded.",
                    "status": "UNVERIFIED",
                },
                "stages": [
                    {
                        "action": "enter fixture",
                        "order": 1,
                        "system": "GSI-01.01",
                    },
                    {
                        "action": "complete fixture",
                        "order": 2,
                        "system": "GSI-01.01",
                    },
                ],
                "owner": "GSI-01.01",
                "player_visible_result": "a visible result",
                "rust_touchpoints": [
                    {
                        "coverage": "representative",
                        "observed_at_commit": "2" * 40,
                        "path": "src/lib.rs",
                    }
                ],
                "stock_fixture": "stock deterministic fixture",
            }
        },
        "observed_at_commit": "2" * 40,
        "schema_version": 2,
        "services": {
            "logicclass": {
                "detail": "fixture service",
                "evidence": ["docs/research/core_services.md:1"],
                "gsi_ids": ["GSI-01.01"],
                "roles": ["drives"],
            }
        },
        "systems": {
            "GSI-01.01": {
                "native_anchors": [
                    {
                        "address": "0x0055AFB0",
                        "evidence": "docs/research/logic.md",
                        "symbol": "LogicClass::AI",
                    }
                ],
                "rust_surface_coverage": "representative",
                "rust_surfaces": [
                    {
                        "coverage": "representative",
                        "observed_at_commit": "2" * 40,
                        "path": "src/lib.rs",
                    }
                ],
            }
        },
    }


def mechanisms_fixture() -> dict:
    return {
        "blocks": {
            "MBLK-001-TEST": {
                "activation": {
                    "guards": [],
                    "mode": "scheduled",
                    "stock_fixture": "stock test fixture",
                    "stock_status": "STOCK_ACTIVE",
                    "trigger": "test tick",
                },
                "authority": [
                    {
                        "aspect": "state",
                        "detail": "owns test state",
                        "owner": "GSI-01.01",
                    }
                ],
                "contract": "Commit the visible test result.",
                "critical_semantics": [
                    {
                        "basis": "native-binary",
                        "detail": "The test order is native-evidenced.",
                        "evidence": ["docs/research/core_services.md:1"],
                        "kind": "ordering",
                        "status": "VERIFIED",
                    }
                ],
                "evidence": ["docs/research/core_services.md:1"],
                "inputs": [
                    {
                        "detail": "the scheduled tick",
                        "external_source": "test scheduler",
                        "name": "tick",
                    }
                ],
                "loop_memberships": [
                    {
                        "loop": "LOOP-001-TEST",
                        "stage_orders": [1, 2],
                    }
                ],
                "name": "test mechanism",
                "native_anchors": [
                    {
                        "address": "0x0055AFB0",
                        "evidence": "docs/research/core_services.md:1",
                        "symbol": "LogicClass::AI",
                    }
                ],
                "open_questions": [],
                "outputs": [
                    {
                        "detail": "the player sees the test result",
                        "name": "visible result",
                        "player_visible": True,
                    }
                ],
                "owner": "GSI-01.01",
                "participants": ["GSI-01.01"],
                "research_query": "LogicClass AI test tick",
                "rust_surfaces": [
                    {
                        "coverage": "representative",
                        "observed_at_commit": "2" * 40,
                        "path": "src/lib.rs",
                    }
                ],
                "steps": [
                    {
                        "action": "enter fixture",
                        "order": 1,
                        "system": "GSI-01.01",
                    },
                    {
                        "action": "complete fixture",
                        "order": 2,
                        "system": "GSI-01.01",
                    },
                ],
            }
        },
        "edges": [],
        "id_policy": {
            "block_pattern": "^MBLK-[0-9]{3}-[A-Z0-9-]+$",
            "edge_pattern": "^MBEDGE-[0-9]{4}-[A-Z0-9-]+$",
            "rule": "Test mechanism IDs do not create GSI systems.",
        },
        "observed_at_commit": "2" * 40,
        "schema_version": 1,
    }
