# VERA20k System Map

Three hand-maintained JSON files that name the engine's systems and record
what has been established about them. They are a navigation aid for parity
work, not a completion ledger, and nothing in them certifies parity.

## Files

- `registry.v2.json` — the 336 canonical systems (`GSI-NN.NN`), grouped by
  family, each with a `baseline_status` (`native_evidence`,
  `rust_implementation`, `parity`, `basis`). Originally imported from
  `docs/research/GAMEMD_SYSTEM_INVENTORY_COVERAGE_MAP_GHIDRA_REPORT.md` and
  `GAMEMD_SYSTEM_STATUS_MATRIX_SYSTEM_MODEL_SYNTHESIS.md`; since 2026-09-06 it
  is edited directly.
- `topology.v2.json` — reviewed annotations on load-bearing systems (native
  anchors with addresses, current Rust surfaces, notes), typed edges between
  systems, the ordered stock player-visible loops (`LOOP-NNN-*`), coupled
  sets, and legacy slice aliases.
- `mechanisms.v1.json` — reviewed mechanism blocks (`MBLK-NNN-*`): one
  behavioural contract each, with its native anchors, Rust surfaces, ordered
  steps and typed handoffs to other blocks.
- `schemas/*.schema.json` — the shapes of the three files, kept as
  documentation. Nothing validates against them any more.

## Checking

```powershell
python -m tools.system_map check
```

Reports every cited repository path that does not exist and every
GSI/loop/block id that is not defined. That is the whole tool. The previous
importer, freshness model, renderer, source lock and query commands were
removed on 2026-09-06 (about 11,600 lines of Python) after an audit found the
data useful and the machinery unused; see the git history of
`tools/system_map/` if a piece is wanted back.

## Reading the data

- `parity` stays `UNCHECKED` or `DRIFT` until a gamemd-derived executable
  comparison demonstrates equivalence. `ANCHORED` native evidence means the
  bodies were read, not that Rust matches them.
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

## Maintenance

Update only the rows, surfaces, edges and blocks your change touched and
verified, then run the checker. Do not pause parity work to annotate the
whole registry, and do not pick work from missing fields: unannotated systems
are unmapped, not unimportant. Per-phase working state (owners per row,
corrections, residuals, open queue) belongs in
[`docs/plans/phase-briefs/`](../plans/phase-briefs/README.md), which cites
this map rather than duplicating it.
