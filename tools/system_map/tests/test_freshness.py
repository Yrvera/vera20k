"""Git-based Rust mapping freshness tests."""

from __future__ import annotations

from pathlib import Path
import subprocess
import tempfile
import unittest

from tools.system_map.freshness import (
    build_freshness,
    build_mechanism_freshness,
    compare_surfaces,
)
from tools.system_map.report import stale_rows


class FreshnessTests(unittest.TestCase):
    def test_changed_is_stale_but_representative_unchanged_is_unresolved(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            _git(repo, "config", "user.email", "system-map@example.invalid")
            _git(repo, "config", "user.name", "System Map Test")
            source = repo / "src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("// v1\n", encoding="utf-8")
            _git(repo, "add", "src/lib.rs")
            _git(repo, "commit", "-q", "-m", "initial")
            observed = _git(repo, "rev-parse", "HEAD").strip()

            representative = [
                {"coverage": "representative", "path": "src/lib.rs"}
            ]
            exhaustive = [{"coverage": "exhaustive", "path": "src/lib.rs"}]

            self.assertEqual(
                compare_surfaces(repo, representative, observed)["state"],
                "UNRESOLVED",
            )
            self.assertEqual(
                compare_surfaces(repo, exhaustive, observed)["state"], "FRESH"
            )

            source.write_text("// dirty\n", encoding="utf-8")
            self.assertEqual(
                compare_surfaces(repo, exhaustive, observed)["state"], "STALE"
            )
            _git(repo, "add", "src/lib.rs")
            _git(repo, "commit", "-q", "-m", "change")
            result = compare_surfaces(repo, exhaustive, observed)
            self.assertEqual(result["state"], "STALE")
            self.assertEqual(result["changed_paths"], ["src/lib.rs"])

    def test_missing_and_unmapped_are_distinct(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            _git(repo, "config", "user.email", "system-map@example.invalid")
            _git(repo, "config", "user.name", "System Map Test")
            marker = repo / "README.md"
            marker.write_text("fixture\n", encoding="utf-8")
            _git(repo, "add", "README.md")
            _git(repo, "commit", "-q", "-m", "initial")
            observed = _git(repo, "rev-parse", "HEAD").strip()

            self.assertEqual(
                compare_surfaces(repo, [], observed)["state"], "UNMAPPED"
            )
            self.assertEqual(
                compare_surfaces(
                    repo,
                    [{"coverage": "exhaustive", "path": "src/missing.rs"}],
                    observed,
                )["state"],
                "MISSING",
            )

    def test_stale_view_excludes_unmapped_rows_by_default(self) -> None:
        def freshness(state: str) -> dict:
            return {
                "changed_paths": [],
                "dirty_paths": [],
                "missing_paths": [],
                "reasons": [],
                "state": state,
            }

        systems = {}
        for system_id, state in (
            ("GSI-01.01", "UNMAPPED"),
            ("GSI-01.02", "STALE"),
            ("GSI-01.03", "FRESH"),
        ):
            systems[system_id] = {
                "freshness": {
                    "baseline_status_freshness": freshness("STALE"),
                    "rust_mapping_freshness": freshness(state),
                },
                "name": system_id,
            }
        report = {"systems": systems}

        self.assertEqual(
            [row["id"] for row in stale_rows(report)], ["GSI-01.02"]
        )
        self.assertEqual(
            [row["id"] for row in stale_rows(report, include_unmapped=True)],
            ["GSI-01.02", "GSI-01.01"],
        )
        self.assertEqual(
            [row["id"] for row in stale_rows(report, "GSI-01.01")],
            ["GSI-01.01"],
        )

    def test_surface_commit_never_overrides_baseline_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            _git(repo, "config", "user.email", "system-map@example.invalid")
            _git(repo, "config", "user.name", "System Map Test")
            source = repo / "src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("// baseline\n", encoding="utf-8")
            _git(repo, "add", "src/lib.rs")
            _git(repo, "commit", "-q", "-m", "baseline")
            baseline = _git(repo, "rev-parse", "HEAD").strip()
            source.write_text("// mapped\n", encoding="utf-8")
            _git(repo, "add", "src/lib.rs")
            _git(repo, "commit", "-q", "-m", "mapped")
            mapped = _git(repo, "rev-parse", "HEAD").strip()
            registry = {
                "baseline_rust_snapshot": baseline,
                "systems": {"GSI-01.01": {}},
            }
            topology = {
                "observed_at_commit": mapped,
                "systems": {
                    "GSI-01.01": {
                        "rust_surfaces": [
                            {
                                "coverage": "exhaustive",
                                "observed_at_commit": mapped,
                                "path": "src/lib.rs",
                            }
                        ]
                    }
                },
            }

            freshness = build_freshness(repo, registry, topology)[
                "GSI-01.01"
            ]

            self.assertEqual(
                freshness["baseline_status_freshness"][
                    "observed_at_commits"
                ],
                [baseline],
            )
            self.assertEqual(
                freshness["baseline_status_freshness"]["state"], "STALE"
            )
            self.assertEqual(
                freshness["rust_mapping_freshness"]["state"], "FRESH"
            )

    def test_topology_root_commit_never_refreshes_surface_mapping(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            _git(repo, "config", "user.email", "system-map@example.invalid")
            _git(repo, "config", "user.name", "System Map Test")
            source = repo / "src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("// observed\n", encoding="utf-8")
            _git(repo, "add", "src/lib.rs")
            _git(repo, "commit", "-q", "-m", "observed")
            observed = _git(repo, "rev-parse", "HEAD").strip()
            source.write_text("// changed\n", encoding="utf-8")
            _git(repo, "add", "src/lib.rs")
            _git(repo, "commit", "-q", "-m", "changed")
            topology_root = _git(repo, "rev-parse", "HEAD").strip()
            registry = {
                "baseline_rust_snapshot": observed,
                "systems": {"GSI-01.01": {}},
            }
            topology = {
                "observed_at_commit": topology_root,
                "systems": {
                    "GSI-01.01": {
                        "rust_surfaces": [
                            {
                                "coverage": "exhaustive",
                                "observed_at_commit": observed,
                                "path": "src/lib.rs",
                            }
                        ]
                    }
                },
            }

            mapping = build_freshness(repo, registry, topology)[
                "GSI-01.01"
            ]["rust_mapping_freshness"]

            self.assertEqual(mapping["observed_at_commits"], [observed])
            self.assertEqual(mapping["state"], "STALE")

    def test_rust_edge_evidence_participates_in_endpoint_freshness(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            _git(repo, "config", "user.email", "system-map@example.invalid")
            _git(repo, "config", "user.name", "System Map Test")
            source = repo / "src/edge.rs"
            source.parent.mkdir(parents=True)
            source.write_text("// observed relationship\n", encoding="utf-8")
            _git(repo, "add", "src/edge.rs")
            _git(repo, "commit", "-q", "-m", "observed")
            observed = _git(repo, "rev-parse", "HEAD").strip()
            registry = {
                "baseline_rust_snapshot": observed,
                "systems": {
                    "GSI-01.01": {},
                    "GSI-01.02": {},
                },
            }
            topology = {
                "edges": [
                    {
                        "evidence": ["src/edge.rs:1"],
                        "from": "GSI-01.01",
                        "observed_at_commit": observed,
                        "plane": "rust",
                        "to": "GSI-01.02",
                    }
                ],
                "loops": {},
                "systems": {},
            }

            freshness = build_freshness(repo, registry, topology)
            for system_id in ("GSI-01.01", "GSI-01.02"):
                mapping = freshness[system_id]["rust_mapping_freshness"]
                self.assertEqual(mapping["paths"], ["src/edge.rs"])
                self.assertEqual(mapping["state"], "UNRESOLVED")

            source.write_text("// dirty relationship\n", encoding="utf-8")
            freshness = build_freshness(repo, registry, topology)
            for system_id in ("GSI-01.01", "GSI-01.02"):
                self.assertEqual(
                    freshness[system_id]["rust_mapping_freshness"]["state"],
                    "STALE",
                )

    def test_rust_mechanism_edge_participates_in_both_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            _git(repo, "config", "user.email", "system-map@example.invalid")
            _git(repo, "config", "user.name", "System Map Test")
            source = repo / "src/handoff.rs"
            source.parent.mkdir(parents=True)
            source.write_text("// observed handoff\n", encoding="utf-8")
            _git(repo, "add", "src/handoff.rs")
            _git(repo, "commit", "-q", "-m", "observed")
            observed = _git(repo, "rev-parse", "HEAD").strip()
            mechanisms = {
                "blocks": {
                    "MBLK-001-A": {"rust_surfaces": []},
                    "MBLK-002-B": {"rust_surfaces": []},
                },
                "edges": [
                    {
                        "evidence": ["src/handoff.rs:1"],
                        "from": "MBLK-001-A",
                        "observed_at_commit": observed,
                        "plane": "rust",
                        "to": "MBLK-002-B",
                    }
                ],
                "observed_at_commit": observed,
            }

            freshness = build_mechanism_freshness(repo, mechanisms)
            for block_id in ("MBLK-001-A", "MBLK-002-B"):
                self.assertEqual(
                    freshness[block_id]["paths"], ["src/handoff.rs"]
                )
                self.assertEqual(
                    freshness[block_id]["state"], "UNRESOLVED"
                )

            source.write_text("// dirty handoff\n", encoding="utf-8")
            freshness = build_mechanism_freshness(repo, mechanisms)
            for block_id in ("MBLK-001-A", "MBLK-002-B"):
                self.assertEqual(freshness[block_id]["state"], "STALE")


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
