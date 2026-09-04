---
name: re-swarm
description: "Coordinate parallel gamemd.exe research and reconcile implementation handoffs. After readers stop, the parent synchronizes certain Ghidra metadata unless opted out. No gameplay or research-input patches."
---

# Coordinate research

Use [shared coordination](../_shared/swarm.md) and
[re-investigate](../re-investigate/SKILL.md). Resolve targets into independent
mechanism questions; absent targets, select a bounded wave from implementation
blockers and current research gaps.

Assign each worker its question, evidence, Rust context and unique task-owned
`docs/research/` report. Workers may write only those reports.

Reconcile identities, formulas, active-YR conditions and Rust implications.
Independently inspect consequential handoffs; consolidate duplicates and necessary
prerequisites. Return report links, supported handoffs, stale-doc corrections,
remaining uncertainty and annotation outcomes.

The parent synchronizes certain metadata by default, serially after all readers stop,
under the [binary workflow](../../../docs/research/ghidra-workflow.md).
`--no-sync-ghidra-labels` or read-only requests disable it. Unavailable write tools
leave candidates unapplied without invalidating research.

Modifiers:

- `--area <area>`: bound selection.
- `--parity-blocker <area>`: prioritize the named gap.
- `--dry-run`: proposed assignments only; no dispatch or mutation.
- `--refresh-index`: refresh discovery from current sources.
- `--handoff-plan`: add a non-editing dependency/validation outline.
