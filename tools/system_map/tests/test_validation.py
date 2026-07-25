"""Canonical topology validation tests."""

from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest

from tools.system_map.tests.helpers import (
    registry_fixture,
    source_lock_fixture,
    topology_fixture,
)
from tools.system_map.validation import errors, validate_all


class ValidationTests(unittest.TestCase):
    def test_valid_minimal_topology_has_no_errors(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )

            self.assertEqual(errors(diagnostics), [])

    def test_unknown_suffix_id_and_dangling_edge_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["edges"][0]["to"] = "GSI-01.01A"

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            codes = {item.code for item in errors(diagnostics)}

            self.assertIn("INVALID_SYSTEM_REFERENCE", codes)

    def test_requires_cycle_needs_explicit_coupled_set(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["edges"] = [
                {
                    "from": "GSI-01.01",
                    "id": "EDGE-0001-A",
                    "kind": "requires",
                    "plane": "routing",
                    "to": "GSI-01.02",
                },
                {
                    "from": "GSI-01.02",
                    "id": "EDGE-0002-B",
                    "kind": "requires",
                    "plane": "routing",
                    "to": "GSI-01.01",
                },
            ]

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            self.assertIn(
                "UNACKNOWLEDGED_REQUIRES_CYCLE",
                {item.code for item in errors(diagnostics)},
            )

            topology["coupled_sets"] = [
                {
                    "evidence": ["docs/research/core_services.md:1"],
                    "id": "coupled:test",
                    "reason": "test-only mutual dependency",
                    "systems": ["GSI-01.01", "GSI-01.02"],
                }
            ]
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            self.assertNotIn(
                "UNACKNOWLEDGED_REQUIRES_CYCLE",
                {item.code for item in errors(diagnostics)},
            )
            topology["coupled_sets"][0].pop("evidence")
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            self.assertIn(
                "MISSING_RESEARCH_EVIDENCE",
                {item.code for item in errors(diagnostics)},
            )

    def test_group_node_cannot_own_loop(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["loops"]["LOOP-001-TEST"]["owner"] = "GSI-01.02"

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )

            self.assertIn(
                "GROUP_LOOP_OWNER",
                {item.code for item in errors(diagnostics)},
            )

    def test_namespaces_aliases_order_and_oracle_are_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["edges"][0]["id"] = "edge:test"
            loop = topology["loops"].pop("LOOP-001-TEST")
            loop["stages"] = [
                {"order": 1, "system": "GSI-01.01"},
                {"order": 3, "system": "GSI-01.01"},
            ]
            loop["oracle"] = {"status": "DONE", "gate": ""}
            topology["loops"]["loop:test"] = loop
            topology["legacy_slice_aliases"] = [
                {
                    "canonical_systems": ["GSI-01.01"],
                    "legacy_id": "GSI-01.01",
                    "slice_id": "slice:test",
                }
            ]

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            codes = {item.code for item in errors(diagnostics)}

            self.assertTrue(
                {
                    "INVALID_EDGE_ID",
                    "INVALID_LEGACY_ALIAS_NAME",
                    "INVALID_LOOP_ID",
                    "INVALID_LOOP_ORDER",
                    "INVALID_ORACLE_STATUS",
                    "INVALID_SLICE_ID",
                    "MISSING_LOOP_STAGE_ACTION",
                    "MISSING_ORACLE_GATE",
                }
                <= codes
            )

    def test_native_and_rust_edge_evidence_contracts_are_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["edges"] = [
                {
                    "context": "tick",
                    "evidence": ["trust me"],
                    "from": "GSI-01.01",
                    "id": "EDGE-0001-NATIVE",
                    "kind": "ordered_before",
                    "plane": "native",
                    "to": "GSI-01.02",
                },
                {
                    "context": "tick",
                    "evidence": ["src"],
                    "from": "GSI-01.01",
                    "id": "EDGE-0002-RUST",
                    "kind": "ordered_before",
                    "plane": "rust",
                    "to": "GSI-01.02",
                },
            ]

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            codes = {item.code for item in errors(diagnostics)}

            self.assertIn("INVALID_NATIVE_EDGE_CITATION", codes)
            self.assertIn("INVALID_RUST_EDGE_EVIDENCE", codes)
            self.assertIn("MISSING_RUST_EDGE_COMMIT", codes)

    def test_requires_cycles_are_checked_within_each_plane(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["edges"] = [
                {
                    "evidence": ["docs/research/test.md"],
                    "from": "GSI-01.01",
                    "id": "EDGE-0001-NATIVE",
                    "kind": "requires",
                    "plane": "native",
                    "to": "GSI-01.02",
                },
                {
                    "from": "GSI-01.02",
                    "id": "EDGE-0002-ROUTING",
                    "kind": "requires",
                    "plane": "routing",
                    "to": "GSI-01.01",
                },
            ]

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )

            self.assertNotIn(
                "UNACKNOWLEDGED_REQUIRES_CYCLE",
                {item.code for item in errors(diagnostics)},
            )

    def test_edge_semantics_and_stage_paths_are_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["edges"][0].pop("context")
            topology["edges"].append(
                {
                    "from": "GSI-01.01",
                    "id": "EDGE-0002-STATE",
                    "kind": "owns_state",
                    "plane": "routing",
                    "to": "GSI-01.02",
                }
            )
            topology["loops"]["LOOP-001-TEST"]["stages"] = [
                {
                    "order": 1,
                    "rust_surfaces": ["C:/outside.rs"],
                    "system": "GSI-01.01",
                },
                {"order": 2, "system": "GSI-01.01"},
            ]

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            codes = {item.code for item in errors(diagnostics)}

            self.assertIn("INVALID_RUST_PATH", codes)
            self.assertIn("MISSING_EDGE_CONTEXT", codes)
            self.assertIn("MISSING_EDGE_STATE", codes)
            self.assertIn("RUST_SURFACE_REQUIRES_OBJECT", codes)

    def test_loop_requirements_are_bound_to_owner_and_stages(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["edges"] = [
                {
                    "detail": "fixture loop requirement",
                    "from": "GSI-01.01",
                    "id": "EDGE-0001-LOOP-NEED",
                    "kind": "loop_requires",
                    "loop": "LOOP-001-TEST",
                    "plane": "routing",
                    "to": "GSI-01.01",
                }
            ]

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            self.assertEqual(errors(diagnostics), [])

            topology["edges"][0]["from"] = "GSI-01.02"
            topology["edges"][0]["to"] = "GSI-01.02"
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            codes = {item.code for item in errors(diagnostics)}
            self.assertIn("LOOP_REQUIREMENT_OWNER_MISMATCH", codes)
            self.assertIn("LOOP_REQUIREMENT_STAGE_MISMATCH", codes)

    def test_service_citation_must_point_to_its_slug_row(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            service_doc = repo / "docs/research/core_services.md"
            service_doc.write_text(
                "| Slug | Purpose |\n| `logicclass` | fixture |\n",
                encoding="utf-8",
            )
            topology["services"]["logicclass"]["evidence"] = [
                "docs/research/core_services.md:1"
            ]
            topology["services"]["logicclass"]["roles"] = ["TYPO"]

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )

            self.assertIn(
                "SERVICE_CITATION_MISMATCH",
                {item.code for item in errors(diagnostics)},
            )
            self.assertIn(
                "INVALID_SERVICE_ROLE",
                {item.code for item in errors(diagnostics)},
            )

    def test_verified_oracle_requires_reproducible_record(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["loops"]["LOOP-001-TEST"]["oracle"] = {
                "gate": "trust me",
                "status": "VERIFIED",
            }

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )

            self.assertIn(
                "MISSING_ORACLE_VERIFICATION",
                {item.code for item in errors(diagnostics)},
            )

            topology["loops"]["LOOP-001-TEST"]["oracle"]["verification"] = {
                "artifacts": ["trust me"],
                "command": "true",
                "method": "native_executable",
                "result": "pass",
            }
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            self.assertIn(
                "INVALID_ORACLE_ARTIFACT",
                {item.code for item in errors(diagnostics)},
            )

            artifact = repo / "tests/oracle-proof.json"
            artifact.parent.mkdir(parents=True, exist_ok=True)
            artifact.write_text('{"result":"matched"}\n', encoding="utf-8")
            digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
            topology["loops"]["LOOP-001-TEST"]["oracle"]["verification"][
                "artifacts"
            ] = [{"path": "tests/oracle-proof.json", "sha256": digest}]
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            self.assertNotIn(
                "INVALID_ORACLE_ARTIFACT",
                {item.code for item in errors(diagnostics)},
            )
            self.assertNotIn(
                "ORACLE_ARTIFACT_DIGEST_MISMATCH",
                {item.code for item in errors(diagnostics)},
            )

            topology["loops"]["LOOP-001-TEST"]["oracle"]["verification"][
                "artifacts"
            ][0]["sha256"] = "0" * 64
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            self.assertIn(
                "ORACLE_ARTIFACT_DIGEST_MISMATCH",
                {item.code for item in errors(diagnostics)},
            )

    def test_observation_commit_must_exist(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            _git(repo, "init", "-q")
            _git(repo, "config", "user.email", "system-map@example.invalid")
            _git(repo, "config", "user.name", "System Map Test")
            _git(repo, "add", ".")
            _git(repo, "commit", "-q", "-m", "fixture")
            head = _git(repo, "rev-parse", "HEAD").strip()
            topology["observed_at_commit"] = head
            for surface in topology["systems"]["GSI-01.01"][
                "rust_surfaces"
            ]:
                surface["observed_at_commit"] = head
            for surface in topology["loops"]["LOOP-001-TEST"][
                "rust_touchpoints"
            ]:
                surface["observed_at_commit"] = "f" * 40

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )

            self.assertIn(
                "UNKNOWN_OBSERVATION_COMMIT",
                {item.code for item in errors(diagnostics)},
            )

    def test_semantically_duplicate_edges_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            duplicate = dict(topology["edges"][0])
            duplicate["id"] = "EDGE-0002-DUPLICATE"
            topology["edges"].append(duplicate)

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )

            self.assertIn(
                "DUPLICATE_EDGE_SEMANTICS",
                {item.code for item in errors(diagnostics)},
            )

    def test_canonical_shape_owner_and_path_evidence_are_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            topology["systems"]["GSI-01.01"]["invented"] = True
            topology["systems"]["GSI-01.01"]["native_anchors"][0].pop(
                "symbol"
            )
            topology["loops"]["LOOP-001-TEST"]["owner"] = "GSI-01.02"
            topology["loops"]["LOOP-001-TEST"]["evidence"] = ["trust me"]
            topology["edges"][0].pop("detail")
            topology["observed_at_commit"] = "2" * 7

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
            )
            codes = {item.code for item in errors(diagnostics)}

            self.assertTrue(
                {
                    "INVALID_EVIDENCE_REFERENCE",
                    "INVALID_OBSERVED_COMMIT",
                    "LOOP_OWNER_NOT_IN_STAGES",
                    "MISSING_EDGE_DETAIL",
                    "MISSING_NATIVE_ANCHOR_SYMBOL",
                    "UNKNOWN_SYSTEM_ANNOTATION_FIELD",
                }
                <= codes
            )

    def test_ci_mode_does_not_require_ignored_research_sources(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            (repo / "docs/research/core_services.md").unlink()

            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
                ci=True,
            )

            self.assertEqual(errors(diagnostics), [])


def _git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return completed.stdout


if __name__ == "__main__":
    unittest.main()
