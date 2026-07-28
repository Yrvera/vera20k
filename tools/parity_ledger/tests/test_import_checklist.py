"""Tests for stable checklist importing without ignored workspace dependencies."""

from pathlib import Path
import unittest

from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.importers.checklist import import_core_checklist, import_scheduler_checklist
from tools.parity_ledger.importers.common import expand_ids
from tools.parity_ledger.source_sets import BOOTSTRAP_SOURCES


FIXTURES = Path(__file__).parent / "fixtures"


class ChecklistImporterTests(unittest.TestCase):
    def test_core_fixture_has_stable_32_rows(self) -> None:
        raw = (FIXTURES / "core-checklist.md").read_bytes()
        config = BOOTSTRAP_SOURCES[0]
        original = import_core_checklist(raw, config)
        checked = import_core_checklist(raw.replace(b"- [ ] One alpha", b"- [x] One alpha"), config)
        self.assertEqual(len(original), 32)
        self.assertEqual([item.id for item in original], [item.id for item in checked])
        self.assertNotEqual(original[0].source.sha256, checked[0].source.sha256)
        self.assertTrue(any("One alpha wraps onto another physical line." == item.title for item in original))

    def test_scheduler_fixture_excludes_status_and_nested_details(self) -> None:
        raw = (FIXTURES / "scheduler-checklist.md").read_bytes()
        rows = import_scheduler_checklist(raw, BOOTSTRAP_SOURCES[1])
        self.assertEqual(len(rows), 17)
        titles = {item.title for item in rows}
        self.assertIn("Contract one", titles)
        self.assertIn("Implementation one.", titles)
        self.assertNotIn("Excluded warning", titles)
        self.assertFalse(any("Nested detail" in title or "done" in title for title in titles))
        without_status = raw.replace(b". **done yesterday**", b".")
        clean_rows = import_scheduler_checklist(without_status, BOOTSTRAP_SOURCES[1])
        self.assertEqual(
            [(item.id, item.title) for item in rows],
            [(item.id, item.title) for item in clean_rows],
        )

    def test_heading_and_title_changes_change_identity(self) -> None:
        raw = (FIXTURES / "core-checklist.md").read_bytes()
        config = BOOTSTRAP_SOURCES[0]
        original = {item.title: item.id for item in import_core_checklist(raw, config)}
        changed = import_core_checklist(raw.replace(b"One beta", b"One beta changed"), config)
        changed_ids = {item.title: item.id for item in changed}
        self.assertNotEqual(original["One beta"], changed_ids["One beta changed"])

    def test_exact_core_structure_and_h2_boundaries_are_required(self) -> None:
        raw = (FIXTURES / "core-checklist.md").read_bytes()
        duplicate_heading = raw.replace(
            b"### 2. Two RNG streams",
            b"### 1. Native tick spine / LogicClass scheduler",
        )
        extended_boundary = raw.replace(
            b"## Suggested Next Work",
            b"## Suggested Next Work Extended",
        )
        for mutated in (duplicate_heading, extended_boundary):
            with self.subTest(mutated=mutated), self.assertRaises(LedgerError):
                import_core_checklist(mutated, BOOTSTRAP_SOURCES[0])

    def test_id_extraction_rejects_lowercase_suffixes(self) -> None:
        self.assertEqual(expand_ids("S4b G5foo xL7 H1"), ("H1",))
        self.assertEqual(expand_ids("xS4-S6y and M2a-M3b"), ())


if __name__ == "__main__":
    unittest.main()
