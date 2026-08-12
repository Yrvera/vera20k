"""Semantic mechanism validation tests."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import tempfile
import unittest

from tools.system_map.tests.helpers import (
    mechanisms_fixture,
    registry_fixture,
    source_lock_fixture,
    topology_fixture,
)
from tools.system_map.validation import errors, validate_all


def _append_routing_edge(data: dict, **overrides: object) -> None:
    if "MBLK-002-TEST" not in data["blocks"]:
        second = deepcopy(data["blocks"]["MBLK-001-TEST"])
        second["name"] = "second test mechanism"
        data["blocks"]["MBLK-002-TEST"] = second
    edge = {
        "detail": "test routing edge",
        "from": "MBLK-001-TEST",
        "id": "MBEDGE-0001-TEST",
        "kind": "emits_to",
        "plane": "routing",
        "to": "MBLK-002-TEST",
    }
    edge.update(overrides)
    data["edges"].append(edge)


class MechanismValidationTests(unittest.TestCase):
    def test_valid_mechanism_contract_has_no_errors(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            registry = registry_fixture()
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology_fixture(repo),
                mechanisms=mechanisms_fixture(),
            )

        self.assertEqual(errors(diagnostics), [])

    def test_stage_system_must_be_a_participant(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            mechanisms = mechanisms_fixture()
            mechanisms["blocks"]["MBLK-001-TEST"]["participants"] = [
                "GSI-01.02"
            ]
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
                mechanisms=mechanisms,
            )

        codes = {item.code for item in errors(diagnostics)}
        self.assertIn("MECHANISM_OWNER_NOT_PARTICIPANT", codes)
        self.assertIn("MECHANISM_STAGE_NOT_PARTICIPANT", codes)

    def test_group_node_cannot_own_mechanism(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            mechanisms = mechanisms_fixture()
            block = mechanisms["blocks"]["MBLK-001-TEST"]
            block["owner"] = "GSI-01.02"
            block["participants"].append("GSI-01.02")
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
                mechanisms=mechanisms,
            )

        self.assertIn(
            "MECHANISM_GROUP_OWNER",
            {item.code for item in errors(diagnostics)},
        )

    def test_requires_cycles_are_checked_per_plane(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            registry = registry_fixture()
            topology = topology_fixture(repo)
            mechanisms = mechanisms_fixture()
            first = mechanisms["blocks"]["MBLK-001-TEST"]
            second = deepcopy(first)
            second["name"] = "second test mechanism"
            mechanisms["blocks"]["MBLK-002-TEST"] = second
            mechanisms["edges"] = [
                {
                    "detail": "native prerequisite",
                    "evidence": ["docs/research/core_services.md:1"],
                    "from": "MBLK-001-TEST",
                    "id": "MBEDGE-0001-NATIVE",
                    "kind": "requires",
                    "plane": "native",
                    "to": "MBLK-002-TEST",
                },
                {
                    "detail": "routing reverse navigation",
                    "from": "MBLK-002-TEST",
                    "id": "MBEDGE-0002-ROUTING",
                    "kind": "requires",
                    "plane": "routing",
                    "to": "MBLK-001-TEST",
                },
            ]
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
                mechanisms=mechanisms,
            )
            self.assertNotIn(
                "MECHANISM_REQUIRES_CYCLE",
                {item.code for item in errors(diagnostics)},
            )

            mechanisms["edges"][1]["plane"] = "native"
            mechanisms["edges"][1]["evidence"] = [
                "docs/research/core_services.md:1"
            ]
            diagnostics = validate_all(
                repo,
                registry,
                source_lock_fixture(registry),
                topology,
                mechanisms=mechanisms,
            )

        self.assertIn(
            "MECHANISM_REQUIRES_CYCLE",
            {item.code for item in errors(diagnostics)},
        )

    def test_malformed_scalar_types_report_errors_without_crashing(self) -> None:
        mutations = {
            "owner": (
                lambda data: data["blocks"]["MBLK-001-TEST"].__setitem__(
                    "owner", []
                ),
                "INVALID_MECHANISM_SYSTEM_REFERENCE",
            ),
            "activation mode": (
                lambda data: data["blocks"]["MBLK-001-TEST"][
                    "activation"
                ].__setitem__("mode", []),
                "INVALID_ACTIVATION_MODE",
            ),
            "semantic status": (
                lambda data: data["blocks"]["MBLK-001-TEST"][
                    "critical_semantics"
                ][0].__setitem__("status", []),
                "INVALID_SEMANTIC_STATUS",
            ),
            "edge plane": (
                lambda data: _append_routing_edge(data, plane=[]),
                "INVALID_MECHANISM_EDGE_PLANE",
            ),
        }
        for label, (mutate, expected_code) in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as temporary:
                repo = Path(temporary)
                registry = registry_fixture()
                mechanisms = mechanisms_fixture()
                mutate(mechanisms)
                diagnostics = validate_all(
                    repo,
                    registry,
                    source_lock_fixture(registry),
                    topology_fixture(repo),
                    mechanisms=mechanisms,
                )
                self.assertIn(
                    expected_code,
                    {item.code for item in errors(diagnostics)},
                )

    def test_runtime_validation_rejects_schema_drift_cases(self) -> None:
        mutations = {
            "boolean schema version": (
                lambda data: data.__setitem__("schema_version", True),
                "MECHANISM_SCHEMA_VERSION",
            ),
            "non-citation block evidence": (
                lambda data: data["blocks"]["MBLK-001-TEST"].__setitem__(
                    "evidence", [123]
                ),
                "INVALID_MECHANISM_CITATION",
            ),
            "blank optional edge context": (
                lambda data: _append_routing_edge(data, context=""),
                "INVALID_MECHANISM_EDGE_CONTEXT",
            ),
            "invalid optional edge commit": (
                lambda data: _append_routing_edge(
                    data, observed_at_commit=123
                ),
                "INVALID_MECHANISM_EDGE_COMMIT",
            ),
            "blank native anchor": (
                lambda data: data["blocks"]["MBLK-001-TEST"].__setitem__(
                    "native_anchors", [""]
                ),
                "EMPTY_NATIVE_ANCHOR",
            ),
        }
        try:
            import json
            from jsonschema import Draft202012Validator

            schema = json.loads(
                (
                    Path(__file__).resolve().parents[3]
                    / "docs/system-map/schemas/mechanisms.v1.schema.json"
                ).read_text(encoding="utf-8")
            )
            schema_validator = Draft202012Validator(schema)
        except ImportError:
            schema_validator = None
        for label, (mutate, expected_code) in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as temporary:
                repo = Path(temporary)
                registry = registry_fixture()
                mechanisms = mechanisms_fixture()
                mutate(mechanisms)
                diagnostics = validate_all(
                    repo,
                    registry,
                    source_lock_fixture(registry),
                    topology_fixture(repo),
                    mechanisms=mechanisms,
                )
                self.assertIn(
                    expected_code,
                    {item.code for item in errors(diagnostics)},
                )
                if schema_validator is not None:
                    self.assertTrue(
                        list(schema_validator.iter_errors(mechanisms)),
                        "manual/schema drift fixture must also fail JSON Schema",
                    )


if __name__ == "__main__":
    unittest.main()
