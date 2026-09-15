# Movement: retail acceptance and increment ledger

Goal: VERA20k movement looks and works like retail Yuri's Revenge. This page
fixes what "like retail" means for movement, how each claim is evidenced, and
which increments have landed. It is a working ledger, not a parity certificate:
every status below cites the evidence it rests on and nothing more.

## Acceptance

A movement behaviour is accepted when an **ordinary, undisabled player order on
a retail map** produces the retail outcome, and the native mechanism behind it is
established from `gamemd.exe` (body, active caller, retail data). The bar is the
one frozen in [bridge-movement-matrix.md](bridge-movement-matrix.md) §0: synthetic
grids are regression ratchets, never parity; a gate whose own provenance says
"gamemd has no equivalent" is a divergence even while it produces no wrong pixel.

Player-visible criteria, in priority order (frequency × visibility):

| # | Criterion | Retail behaviour to match | Evidence instrument |
|---|---|---|---|
| A1 | Order admission | Every reachable ground destination is accepted; refusals only where retail refuses (`+0x1AC` code 7, zone reachability) | Retail-map `Command::Move` harness (`movement_bridge_retail_tests.rs`), Ghidra `+0x1AC` bodies |
| A2 | Route choice | A* over `Can_Enter_Cell` codes with the native cost table `0x0081870C`, hierarchy precheck, smoothing `0x0042B420` | `spatial_oracle.path_entry`, A* trace lanes, retail-map routes |
| A3 | Drive turning and tracks | Turn-table curves, ROT, residual budget, `Accelerates` ramp per `Process_Drive_Track 0x004B0F20` | `locomotor_track_*` oracles, `drive_track_tests.rs` |
| A4 | Column following | Followers chain onto an occupant in transit (`0x0073FA2C..FA7C`, code 2) instead of stopping | Retail-map convoy run; Ghidra |
| A5 | Blocked recovery | Retry counter, `CloseEnough`, scatter of blockers (`Scatter_Objects 0x00481670`, ten-entry cap) | Ghidra; retail-map blocked run |
| A6 | Infantry stepping | Sub-cell slots, `Walk` head/boundary/completion at `0x0075AEC0` family | `walk_*` oracles (consumed), retail-map runs |
| A7 | Crush | Crusher over `Crushable` infantry, sandbags and fences; victim lifecycle inside the crusher's slot | Ghidra; retail-map run |
| A8 | Height, slopes, bridges | Ground/deck height, ramp tilt, slope transitions; under-span and over-span routing | `ramp_height_oracle`, retail bridge matrix |
| A9 | Hover motion | Own XY integrator and vertical bob, not the Drive track | Ghidra `HoverLocomotionClass`; retail run |
| A10 | Scale | 20,000 units move without per-object whole-map work | `sim-phase-profiler`, hash-equal regression |

## Instruments

- **Retail-map harness:** `src/sim/movement/movement_bridge_retail_tests.rs`
  drives ordinary orders on `Hills.mmx`, `BayOPigs.mmx`, `Deadman.mmx` and records
  per-tick cell, z, on-bridge and occupancy. Ignored in CI; needs `config.toml`.
- **Native oracles:** `tools/spatial_oracle/*` (Unicorn, binary
  `1cdd1180…4298c`), see [tools/native_oracle.md](../../tools/native_oracle.md).
  No motion-over-time oracle exists; the executable capture programme is
  `BLOCKED` ([capture report](../research/GROUND_MOVEMENT_EXECUTABLE_NATIVE_ORACLE_CAPTURE_REPORT.md)).
- **Ghidra:** `gamemd.exe` in `testProsjekt`; UnitClass vtable `0x007F5C70`
  (`+0x1AC` at `0x007F5E1C` → `0x0073F0A0`), InfantryClass vtable `0x007EB058`
  (`+0x1AC` at `0x007EB204` → `0x0051BF90`).

## Increment ledger

Each row lands as its own reviewed PR. Status words: `LANDED` (merged, retail
harness green), `OPEN` (not started), `IN PROGRESS`.

| Increment | Criterion | Status | Evidence |
|---|---|---|---|
| I1 Ground-plane routing beneath an intact high span for Drive and Hover (matrix T1-13 / T1-15) | A1, A8 | IN PROGRESS | `UnitClass::Can_Enter_Cell 0x0073F0A0` never calls `CheckCellPassability 0x004834A0` (xref list: CellRect, threat scan, placement, paradrop, overlay Mark, Jumpjet touchdown); it defers height legality to `CheckBridgeTraversal 0x004D9C60` (equal-level arm admits) and selects the ground list `+0xE4` at `0x0073F51A` for a path height within one of the cell level (`0x0073F0B7..F0E8`), reading the land row at `0x0073FAB5` only on that branch. Rust: `cell_entry.rs` class arm extended from Infantry to every ground-layer mover; `TerrainCostGrid::ground_cost_at` supplies the terrain's own row beneath a deck. |
| I2 Movement pass whole-map setup hoisted from per-object to per-frame | A10 | OPEN | `movement_tick.rs:3692` rebuilds `blocker_neighbor_counts` (W×H scan) and per-owner block sets (all entities) once per live object per frame. |
| I3 Follower chains onto an occupant in transit | A4 | OPEN | Recorded DRIFT at `movement_tick.rs:1185`; native `0x0073FA2C..FA7C`. |
| I4 Hard-turn translation while braking | A3 | OPEN | Disclosed approximation at `movement_step.rs:309`. |
| I5 Path smoothing rejection terms | A2 | OPEN | `path_smooth.rs:18` ports `0x0042B420` without three rejection terms. |
| I6 Scatter model | A5 | OPEN | `scatter.rs` header retraction; `Scatter_Objects 0x00481670`. |
| I7 Hover locomotor phases 2/3 | A9 | OPEN | `hover.rs` header. |

## Residuals carried

- I1 consequence: beneath a collapsed high span the structural stamp is kept
  while the deck is no longer elevated, so Drive is admitted on the ground plane
  by the same rule as beneath an intact span. This follows the native reading
  (`0x004D9C60` and the land row decide; `0x100` only selects the plane) but no
  retail damaged-span crossing has been run for Drive.

- Attack-move characterizations `*_attack_moved_across_*_is_currently_dropped`
  in the retail harness now fail because the stall they pin is gone on `main`;
  they must be rewritten as positive crossings (observed 2026-09-15, 4 tests).
- `bridge_tile_retail_probe` and `bridge_restamp_retail_probe` need extracted
  campaign maps (`c3y03md.map`, `xmp34u4.map`) and a `.local/` directory; they
  fail for environment reasons on a fresh worktree, not behaviour.
