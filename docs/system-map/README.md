# VERA20k System Map — retired

**Retired from active maintenance on 2026-09-10.** The files remain in place
to preserve existing GSI, loop and mechanism references and historical evidence.
Do not update them for new engine work or require their checker for delivery.

Descriptions, status fields and cited research may be stale, contradictory or
incorrect. An archived citation or review is not proof of current accuracy.
Use current Rust and its nearby native references, focused research, and
reproducible native comparisons. Per-phase handoffs live in
[`docs/plans/phase-briefs/`](../plans/phase-briefs/README.md).

## Preserved files

- `registry.v2.json` — the 336 canonical systems (`GSI-NN.NN`), grouped by
  family, each with a `baseline_status` (`native_evidence`,
  `rust_implementation`, `parity`, `basis`). Originally imported from
  `docs/research/GAMEMD_SYSTEM_INVENTORY_COVERAGE_MAP_GHIDRA_REPORT.md` and
  `GAMEMD_SYSTEM_STATUS_MATRIX_SYSTEM_MODEL_SYNTHESIS.md`; edited directly from
  2026-09-06 until retirement. Status values are historical snapshots.
- `topology.v2.json` — reviewed annotations on load-bearing systems (native
  anchors with addresses, then-observed Rust surfaces, notes), typed edges between
  systems, the ordered stock player-visible loops (`LOOP-NNN-*`), coupled
  sets, and legacy slice aliases.
- `mechanisms.v1.json` — reviewed mechanism blocks (`MBLK-NNN-*`): one
  behavioural contract each, with its native anchors, Rust surfaces, ordered
  steps and typed handoffs to other blocks.
- `schemas/*.schema.json` — the shapes of the three files, kept as
  documentation. Nothing validates against them any more.

## Optional archive check

```powershell
python -m tools.system_map check
```

The retained compatibility tool reports missing cited repository paths and
undefined GSI/loop/block IDs. It does not check accuracy or freshness. The previous
importer, freshness model, renderer, source lock and query commands were
removed on 2026-09-06 (about 11,600 lines of Python) after an audit found the
data useful and the machinery unused; see the git history of
`tools/system_map/` if a piece is wanted back.

## Reading historical data

- `parity` and other statuses record past assessments, not current coverage.
  `ANCHORED` native evidence meant the bodies were read, not that Rust matched them.
- Native anchors, Rust surfaces and edges sit on separate planes. A native
  edge never implies Rust implements or orders the connection; a Rust surface
  marked `representative` names one place the mechanism lives, not all of
  them.
- Loop stage order is the player's journey through a loop, not proof that
  adjacent stages run in that order within a frame. Exact ordering claims live
  in `MBEDGE-*` mechanism edges with cited evidence.
- `observed_at_commit` on a Rust surface is the commit at which someone last
  looked. Later commits may have moved the code; the checker only catches
  paths that vanished, not paths whose contents drifted.

## Identity rules

- System ids are exactly `GSI-NN.NN` from the inventory. Never invent
  suffixed ids such as `GSI-04.03A`; historical pseudo-ids live only under
  `legacy_slice_aliases`.
- Loops are `LOOP-NNN-SLUG`, edges `EDGE-NNNN-SLUG`, mechanism blocks
  `MBLK-NNN-SLUG`, mechanism edges `MBEDGE-NNNN-SLUG`.

## Retirement boundary

The inventory IDs and existing links remain valid identifiers, not current
status claims. Neither missing annotations nor old status fields select the
next implementation task. Record current evidence beside the implementation
and in focused research; use a phase brief for the current working handoff.

The research-index `research_map` tool is separate: it searches and groups
research documents and is not retired with these hand-maintained JSON files.
