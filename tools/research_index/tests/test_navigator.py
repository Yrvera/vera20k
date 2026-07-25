"""Unified navigator composition and production CLI tests."""

from __future__ import annotations

import json
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile
import unittest


TOOL_ROOT = Path(__file__).resolve().parents[1]
REPO_ROOT = TOOL_ROOT.parents[1]
for path in (TOOL_ROOT, REPO_ROOT):
    if str(path) not in sys.path:
        sys.path.insert(0, str(path))

from research_index.lifecycle import refresh_index
from research_index.navigator import (
    NAVIGATOR_MAX_ANCHORS,
    NAVIGATOR_MAX_LIMIT,
    research_navigate,
)
from research_index.navigator_formatting import format_research_navigator
from tools.system_map.api import load_report
from tools.system_map.model import SystemMapError


class NavigatorCompositionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = load_report(REPO_ROOT, require_sources=True)

    def test_result_keeps_evidence_and_topology_separate(self) -> None:
        with _indexed_workspace() as (workspace, db_path):
            result = research_navigate(
                db_path,
                workspace,
                self.report,
                "power outage recovery",
                system_id="GSI-09.07",
                loop_id="LOOP-012-POWER-OUTAGE-RECOVERY",
                limit=3,
            )

        self.assertTrue(result["matched"])
        self.assertTrue(result["research_matched"])
        self.assertTrue(result["system_map"]["matched"])
        self.assertIn("validation", result["research"])
        self.assertEqual(
            result["system_map"]["selected_system"]["system"]["id"],
            "GSI-09.07",
        )
        self.assertEqual(
            result["system_map"]["selected_loop"]["id"],
            "LOOP-012-POWER-OUTAGE-RECOVERY",
        )
        self.assertTrue(
            all(
                candidate["candidate_only"]
                for candidate in result["system_map"]["system_candidates"]
            )
        )

        text = format_research_navigator(result)
        self.assertIn("candidate only; not verified ownership", text)
        self.assertIn("Selected player-visible loop:", text)
        self.assertIn("Pre-implementation brief:", text)

    def test_zero_matches_are_explicit_and_not_success_shaped(self) -> None:
        with _indexed_workspace() as (workspace, db_path):
            result = research_navigate(
                db_path,
                workspace,
                self.report,
                secrets.token_hex(16),
                limit=2,
            )

        self.assertFalse(result["matched"])
        self.assertFalse(result["research_matched"])
        self.assertFalse(result["system_map"]["matched"])
        text = format_research_navigator(result)
        self.assertIn("matched: False", text)
        self.assertIn("No systems matched.", text)
        self.assertIn("No loops matched.", text)

    def test_exact_query_selects_system_and_unknown_id_fails(self) -> None:
        with _indexed_workspace() as (workspace, db_path):
            selected = research_navigate(
                db_path,
                workspace,
                self.report,
                "gsi-07.15",
                limit=2,
            )
            with self.assertRaises(SystemMapError):
                research_navigate(
                    db_path,
                    workspace,
                    self.report,
                    "GSI-99.99",
                    limit=2,
                )

        self.assertEqual(
            selected["system_map"]["selected_system"]["system"]["id"],
            "GSI-07.15",
        )

    def test_input_bounds_prevent_oversized_handoffs(self) -> None:
        with _indexed_workspace() as (workspace, db_path):
            with self.assertRaisesRegex(ValueError, "limit"):
                research_navigate(
                    db_path,
                    workspace,
                    self.report,
                    "power",
                    limit=NAVIGATOR_MAX_LIMIT + 1,
                )
            with self.assertRaisesRegex(ValueError, "anchors"):
                research_navigate(
                    db_path,
                    workspace,
                    self.report,
                    "power",
                    anchors=[
                        f"anchor-{index}"
                        for index in range(NAVIGATOR_MAX_ANCHORS + 1)
                    ],
                )


class NavigatorCliTests(unittest.TestCase):
    def test_exact_selection_round_trips_as_json(self) -> None:
        completed = self._run("--json", "--limit", "2", "GSI-07.15")
        result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertTrue(result["matched"])
        self.assertEqual(
            result["system_map"]["selected_system"]["system"]["id"],
            "GSI-07.15",
        )

    def test_zero_match_is_structured_and_nonzero(self) -> None:
        completed = self._run(
            "--json",
            "--limit",
            "2",
            secrets.token_hex(16),
        )
        result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 1, completed.stderr)
        self.assertFalse(result["matched"])
        self.assertFalse(result["research_matched"])
        self.assertFalse(result["system_map"]["matched"])

    def test_unknown_exact_id_is_a_structured_input_error(self) -> None:
        completed = self._run("--json", "GSI-99.99")
        result = json.loads(completed.stderr)

        self.assertEqual(completed.returncode, 4)
        self.assertEqual(
            result["diagnostics"][0]["code"],
            "UNKNOWN_SYSTEM",
        )

    def _run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, "tools/research_index/navigate.py", *args],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
        )


class _IndexedWorkspace:
    def __init__(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()

    def __enter__(self) -> tuple[Path, Path]:
        workspace = Path(self._temporary.name)
        doc = workspace / "docs/research/POWER_GHIDRA_REPORT.md"
        doc.parent.mkdir(parents=True)
        doc.write_text(
            "\n".join(
                (
                    "# Power outage recovery",
                    "",
                    "Verified power outage recovery evidence.",
                    "",
                    "## Implementation Handoff",
                    "",
                    "Rust touchpoint: `src/sim/power_system.rs`.",
                )
            ),
            encoding="utf-8",
        )
        db_path = workspace / "research.db"
        refresh_index(db_path, workspace, roots=["docs/research"])
        return workspace, db_path

    def __exit__(self, *args: object) -> None:
        self._temporary.cleanup()


def _indexed_workspace() -> _IndexedWorkspace:
    return _IndexedWorkspace()


if __name__ == "__main__":
    unittest.main()
