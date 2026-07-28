"""Inventory/status parser tests."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from tools.system_map.baseline_validation import validate_live_sources
from tools.system_map.registry import (
    build_registry,
    parse_inventory,
    parse_status_matrix,
)
from tools.system_map.tests.helpers import registry_fixture, source_lock_fixture


class RegistryParserTests(unittest.TestCase):
    def test_live_corpus_has_expected_bootstrap_shape(self) -> None:
        repo = Path(__file__).resolve().parents[3]
        registry, source_lock = build_registry(repo)

        self.assertEqual(len(registry["systems"]), 336)
        self.assertEqual(len(registry["families"]), 18)
        self.assertEqual(len(registry["service_catalog"]), 41)
        self.assertEqual(
            sum(
                1
                for row in registry["systems"].values()
                if row["baseline_status"]["activity"] == "GROUP_NODE"
            ),
            77,
        )
        self.assertEqual(
            registry["baseline_rust_snapshot"],
            "a97ce88454d2ab938e6f8892dcac861845302c09",
        )
        self.assertEqual(
            source_lock["baseline_rust_snapshot"],
            registry["baseline_rust_snapshot"],
        )

    def test_parser_preserves_independent_baseline_axes(self) -> None:
        inventory = "\n".join(
            [
                "### GSI-01 — Runtime",
                "| ID | System | Discovery scope |",
                "|---|---|---|",
                "| GSI-01.01 | one `inline` system | stock |",
            ]
        )
        status = "\n".join(
            [
                f"**Rust snapshot:** `{'1' * 40}`",
                "### GSI-01 — Runtime",
                "| ID | System | Activity | Inventory | Native | Rust | Parity | Basis |",
                "|---|---|---|---|---|---|---|---|",
                "| GSI-01.01 | one `inline` system | STOCK_ACTIVE | "
                "EXHAUSTIVE_SLICE | NATIVE_ORACLE | COMPLETE_FOR_CONTRACT | "
                "VERIFIED | TEST |",
            ]
        )

        families, inventory_rows = parse_inventory(inventory, "inventory.md")
        snapshot, status_rows = parse_status_matrix(status, "status.md")

        self.assertEqual(snapshot, "1" * 40)
        self.assertEqual(families["GSI-01"]["systems"], ["GSI-01.01"])
        self.assertEqual(
            status_rows["GSI-01.01"]["native_evidence"], "NATIVE_ORACLE"
        )
        self.assertEqual(
            status_rows["GSI-01.01"]["rust_implementation"],
            "COMPLETE_FOR_CONTRACT",
        )
        self.assertEqual(
            inventory_rows["GSI-01.01"]["name"], "one `inline` system"
        )

    def test_live_validation_compares_the_complete_source_lock(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            registry = registry_fixture()
            locked = source_lock_fixture(registry)
            fresh = deepcopy(locked)
            fresh["sources"]["inventory"]["path"] = (
                "docs/research/canonical-inventory.md"
            )
            diagnostics = []

            with patch(
                "tools.system_map.baseline_validation.build_registry",
                return_value=(registry, fresh),
            ):
                validate_live_sources(repo, registry, locked, diagnostics)

            self.assertIn(
                "SOURCE_LOCK_DRIFT", {item.code for item in diagnostics}
            )


if __name__ == "__main__":
    unittest.main()
