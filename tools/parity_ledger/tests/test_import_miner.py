"""Tests for miner record shapes and collision-safe roadmap assignments."""

from pathlib import Path
import unittest

from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.importers.miner import import_miner
from tools.parity_ledger.source_sets import BOOTSTRAP_SOURCES


def _scan() -> bytes:
    lines = [
        "## Confirmed gaps — HIGH severity (visible in normal play within seconds)",
        *(f"**G{i}. High gap {i}**\n- Rust: src/sim/g{i}.rs:1" for i in range(1, 20)),
        "## Confirmed gaps — MEDIUM severity (specific situations / attentive players)",
        *(f"**M{i}. Medium gap {i}**" for i in range(1, 34)),
        "## Confirmed gaps — LOW severity (rare/boundary — still real; ranked for fix order, not parity)",
        *(f"- **L{i}.** Low gap {i}." for i in range(1, 68)),
        "## Slave miner & OREGATH additions (slave-war-render lane — 33 confirmed, 6 needs-verification)",
        "HIGH:",
        *(f"- **S{i}. High slave gap {i}.** Detail." for i in range(1, 4)),
        "MEDIUM:",
        *(f"- **S{i}. Medium slave gap {i}.** Detail." for i in range(4, 9)),
        "LOW (slave/anim details, mostly mod-visible or stock-inert):",
        *(f"- **S{i}.** Low slave gap {i}." for i in range(9, 21)),
        "## Needs verification (Rust state verified; gamemd side needs Ghidra or runtime capture)",
    ]
    return ("\n\n".join(lines) + "\n").encode()


def _roadmap() -> bytes:
    all_ids = (
        {f"G{i}" for i in range(1, 20)}
        | {f"M{i}" for i in range(1, 34)}
        | {f"L{i}" for i in range(1, 68)}
        | {f"S{i}" for i in range(1, 21)}
    )
    omitted = {"L7", "L34", "L35", "L43", "M32"}
    special = {"M1", "M5", "M30", "S5", "S12", "L66"}
    ordinary = sorted(all_ids - omitted - special)
    sections = [
        "# Fixture\n\n(140 confirmed gaps: synthetic)",
        "## W0 — Quick wins (no research gate, independent, do first)\n- " + " ".join(ordinary),
        "## W1 — Mission-cadence service\n- M1 and M5 half advanced.\n- S12 NOT in W1 — deferred to W11.",
        "## W2 — Dock radio protocol pass\n- M1 M5",
        "## W3 — Dock search",
        "## W4 — FNPC",
        "## W5 — Facing",
        "## W6 — Combat\n- object-AI S5 slice",
        "## W7 — Interrupts",
        "## W8 — Rendering",
        "## W9 — Warp\n- M30",
        "## W10 — Harvest",
        "## W11 — Slave\n- S5",
        "## W12 — Research\n- M30",
        "## W13 — Corrections",
        "## Deferred (correctly absent — needs other systems first)\n- L66",
        "## Suggested sequence",
        "## Status\n- M32",
    ]
    return ("\n\n".join(sections) + "\n").encode()


class MinerImporterTests(unittest.TestCase):
    def test_full_synthetic_corpus_and_assignment_exceptions(self) -> None:
        rows, diagnostics = import_miner(
            _scan(),
            _roadmap(),
            BOOTSTRAP_SOURCES[2],
            BOOTSTRAP_SOURCES[3],
        )
        self.assertEqual(len(rows), 139)
        by_id = {item.id: item for item in rows}
        self.assertEqual(
            {item.id for item in rows if item.assignment.primary is None},
            {"miner:L7", "miner:L34", "miner:L35", "miner:L43", "miner:M32"},
        )
        self.assertEqual(by_id["miner:M1"].assignment.primary.workstream, "miner:W2")
        self.assertEqual(by_id["miner:M30"].assignment.primary.workstream, "miner:W9")
        self.assertEqual(by_id["miner:S5"].assignment.primary.workstream, "miner:W11")
        self.assertEqual(by_id["miner:S12"].assignment.primary.workstream, "miner:W11")
        self.assertEqual(by_id["miner:L66"].assignment.primary.workstream, "miner:DEFERRED")
        self.assertEqual(by_id["miner:G1"].rust_anchors, ("src/sim/g1.rs",))
        self.assertEqual([item.code for item in diagnostics], ["DECLARED_COUNT_MISMATCH"])

    def test_fixture_documents_capture_record_and_collision_shapes(self) -> None:
        fixture_dir = Path(__file__).parent / "fixtures"
        scan = (fixture_dir / "miner-scan.md").read_text(encoding="utf-8")
        roadmap = (fixture_dir / "miner-roadmap.md").read_text(encoding="utf-8")
        self.assertIn("- **L1.** Marker-only title wraps", scan)
        self.assertIn("object-AI S5 slice", roadmap)
        self.assertIn("S12 NOT in W1", roadmap)

    def test_region_family_swaps_and_duplicate_workstreams_fail(self) -> None:
        scan = _scan()
        swapped = scan.replace(b"**G1.", b"**X999.").replace(b"**M1.", b"**G1.").replace(b"**X999.", b"**M1.")
        duplicate_workstream = _roadmap().replace(
            "## W2 — Dock radio protocol pass".encode("utf-8"),
            "## W1 — Dock radio protocol pass".encode("utf-8"),
        )
        with self.assertRaises(LedgerError):
            import_miner(swapped, _roadmap(), BOOTSTRAP_SOURCES[2], BOOTSTRAP_SOURCES[3])
        with self.assertRaises(LedgerError):
            import_miner(scan, duplicate_workstream, BOOTSTRAP_SOURCES[2], BOOTSTRAP_SOURCES[3])


if __name__ == "__main__":
    unittest.main()
