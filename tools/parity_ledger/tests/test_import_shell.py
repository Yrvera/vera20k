"""Tests for exact shell active IDs, dispositions, and ownership overlays."""

from pathlib import Path
import unittest

from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.importers.shell import import_shell
from tools.parity_ledger.source_sets import BOOTSTRAP_SOURCES


def _active_ids() -> set[str]:
    return (
        {f"H{i}" for i in range(1, 20)}
        | {f"M{i}" for i in range(1, 40)}
        | ({f"L{i}" for i in range(1, 35)} - {"L3", "L25", "L28"})
    )


def _scan() -> bytes:
    lines = [
        "## Summary",
        "89 unique confirmed gaps: L1..L34, with L3/L25 merged upward into M13/L2 and L24 respectively, and L28 reclassified as a proven non-gap.",
        "A further ~15 CONFIRMED LOW items are folded into the per-service inventory rather than separately numbered.",
        "## ENGINE-SERVICE INVENTORY (framing 1)",
        "## Confirmed gaps — HIGH severity",
        *(f"**H{i}. High shell gap {i}.**\n- Rust: src/ui/h{i}.rs:1" for i in range(1, 20)),
        "## Confirmed gaps — MEDIUM severity",
        *(f"**M{i}. Medium shell gap {i}.**" for i in range(1, 40)),
        "## Confirmed gaps — LOW severity",
        *(f"**L{i}. Low shell gap {i}.**" for i in range(1, 35) if i not in {3, 25, 28}),
        "*(Additional LOW items folded into services above and not separately numbered: fixture.)*",
        "## Needs-verification queue (NV1..)",
    ]
    return ("\n\n".join(lines) + "\n").encode()


def _roadmap() -> bytes:
    active = _active_ids() - {"H1", "H19"}
    special = {"H5", "L4", "L10", "L14", "M16", "M21", "M25", "M26", "M27", "M35"}
    scopes: dict[str, set[str]] = {f"WS-{i}": set() for i in range(1, 14)}
    scopes["WS-1"] = active - special
    for local_id, owners in {
        "H5": ("WS-3",),
        "L4": ("WS-4", "WS-9"),
        "L10": ("WS-10", "WS-13"),
        "L14": ("WS-1", "WS-11"),
        "M16": ("WS-4", "WS-10"),
        "M21": ("WS-10", "WS-13"),
        "M25": ("WS-6", "WS-10"),
        "M26": ("WS-10", "WS-13"),
        "M27": ("WS-6", "WS-11"),
        "M35": ("WS-5", "WS-8"),
    }.items():
        for owner in owners:
            scopes[owner].add(local_id)
    sections = []
    for workstream in (f"WS-{i}" for i in range(1, 14)):
        targets = " ".join(sorted(scopes[workstream])) or "none"
        if workstream == "WS-5":
            targets = targets.replace("M35", "M35 (cameo mis-anchor — folded in M28)")
        metadata = "- **Dependencies:** unblocks H5.\n" if workstream == "WS-9" else ""
        sections.append(
            f"### {workstream} · Fixture\n\n- **Scope (closes):** {targets}.\n{metadata}"
        )
    quick = [
        "## Quick wins (trivial, existing test seam — MAY skip /brainstorm; still need a named test)",
        "- **QW-1 · M13 + L2** — fixture.",
        "- **QW-2 · M3** — fixture.",
        "- **QW-3 · M23** — fixture.",
        "- **QW-4 · L18** — fixture.",
        "- **QW-5 · L19** — fixture.",
        "- **QW-6 · M31** — fixture.",
        "- **QW-7 · L8** — fixture.",
        "- **QW-8 · M2** — fixture.",
        "- **QW-9 · L20** — fixture.",
        "## Research-first queue (resolve the named binary question BEFORE implementing the surface)",
        "| NV | Question | Target | Anchor |",
        "|---|---|---|---|",
        "| NV4 | Question (H7) | tool | anchor |",
        "| NV6 | Question (M4) | tool | anchor |",
        "| NV7 | Question (H18) | tool | anchor |",
        "| NV8 | Question (M7) | tool | anchor |",
        "| NV50/NV51 | Question (H9) | tool | anchor |",
        "| NV56 | Question (M29) | tool | anchor |",
        "| NV1 | Question (H15) | tool | anchor |",
        "| NV22 | Question (M22) | tool | anchor |",
        "| L1 (per-consumer) | Question | tool | anchor |",
        "## Suggested order (dependency-aware)",
        "**Deferred (blocked, not scheduled):** campaign (H10, M29, L9); network (M1, L27); pacing (L1).",
        "",
        "**TS-legacy / WOL — NOT gaps:** fixture.",
    ]
    return ("\n".join([*sections, *quick]) + "\n").encode()


class ShellImporterTests(unittest.TestCase):
    def test_full_synthetic_corpus(self) -> None:
        rows, dispositions, diagnostics = import_shell(
            _scan(),
            _roadmap(),
            BOOTSTRAP_SOURCES[4],
            BOOTSTRAP_SOURCES[5],
        )
        self.assertEqual(len(rows), 89)
        self.assertEqual({item.source_id for item in dispositions}, {"shell:L3", "shell:L25", "shell:L28"})
        self.assertEqual({item.id for item in rows if item.assignment.primary is None}, {"shell:H1", "shell:H19"})
        by_id = {item.id: item for item in rows}
        self.assertEqual(by_id["shell:M35"].assignment.primary.workstream, "shell:WS-8")
        self.assertFalse(
            any(item.workstream == "shell:WS-5" for item in by_id["shell:M35"].assignment.related)
        )
        self.assertFalse(
            any(item.workstream == "shell:WS-9" for item in by_id["shell:H5"].assignment.related)
        )
        self.assertEqual(by_id["shell:L4"].assignment.primary.workstream, "shell:WS-4")
        self.assertEqual(by_id["shell:M13"].assignment.primary.workstream, "shell:QW-1")
        self.assertEqual(by_id["shell:H1"].rust_anchors, ("src/ui/h1.rs",))
        self.assertEqual(
            [item.code for item in diagnostics],
            ["STALE_ROADMAP_REFERENCE", "UNNUMBERED_CONFIRMED_ITEMS"],
        )

    def test_fixture_documents_capture_multiline_and_overlay_shapes(self) -> None:
        fixture_dir = Path(__file__).parent / "fixtures"
        scan = (fixture_dir / "shell-scan.md").read_text(encoding="utf-8")
        roadmap = (fixture_dir / "shell-roadmap.md").read_text(encoding="utf-8")
        self.assertIn("A shell title wraps\nonto its second physical line", scan)
        self.assertIn("Deferred (blocked, not scheduled)", roadmap)

    def test_region_heading_quick_win_and_research_swaps_fail(self) -> None:
        scan = _scan()
        swapped = scan.replace(b"**H1.", b"**X999.").replace(b"**M1.", b"**H1.").replace(b"**X999.", b"**M1.")
        roadmap = _roadmap()
        duplicate_ws = roadmap.replace(
            "### WS-2 · Fixture".encode("utf-8"),
            "### WS-1 · Fixture".encode("utf-8"),
        )
        duplicate_qw = roadmap.replace(
            "QW-2 · M3".encode("utf-8"),
            "QW-1 · M3".encode("utf-8"),
        )
        swapped_gates = roadmap.replace(b"| NV4 |", b"| TMP |", 1).replace(b"| NV6 |", b"| NV4 |", 1).replace(b"| TMP |", b"| NV6 |", 1)
        for scan_raw, roadmap_raw in (
            (swapped, roadmap),
            (scan, duplicate_ws),
            (scan, duplicate_qw),
            (scan, swapped_gates),
        ):
            with self.subTest(roadmap=roadmap_raw[:100]), self.assertRaises(LedgerError):
                import_shell(scan_raw, roadmap_raw, BOOTSTRAP_SOURCES[4], BOOTSTRAP_SOURCES[5])


if __name__ == "__main__":
    unittest.main()
