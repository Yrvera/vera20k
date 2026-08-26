---
title: Disparity Scan - nonbridge Find Nearby Passable Cell
date: 2026-08-24
scope: MapClass Find Nearby Passable Cell 0x0056DC20 core, nonbridge rows, and represented Phase 3 callers
methodology: docs-first discovery, direct Rust verification, selective active-YR verification
---

# Disparity Scan - nonbridge Find Nearby Passable Cell

## Scope and evidence basis

This scan covers the active `0x0056DC20` nearby-passable-cell core, its
`bridge_aware_zone = 0` behavior, the deficient-start caller at `0x00688380`,
and the represented Rust callers that already claim this helper. Bridge-aware
zone lookup is retained as a separate GSI-04.03/04.04 prerequisite because the
current CellRect facade explicitly leaves that redirect unchecked. Higher-level
mission transitions are outside this bounded mechanism; their exact caller rows
remain visible below rather than being treated as absent.

Active evidence was read directly from `gamemd.exe` in the live `testProsjekt`
Ghidra instance. Fresh reads covered `0x0056DC20`, `0x00688380`, `0x0056BD40`,
`0x00486FF0`, and `0x006D6410`. A fresh xref census found 99 static calls to
`0x0056DC20`; assembly context for every one contains
`MOV ECX,0x0087F7E8` before the call. The receiver is therefore `g_Map`, and
`receiver+0xF4/+0xF8` are MapClass `Size.width/height`, not FootClass
Speed/Sight.

Documents read top to bottom included the current corrected deficient-start
report, both FNPC core/decode reports, the caller parameter matrix, the ring
resolution, the CellRect validator contract, the deficient-start gather report,
and the blocked-destination caller report. Current Rust was read directly in
`src/sim/find_nearby_cell.rs`, `src/sim/scenario_bootstrap.rs`,
`src/sim/cell_rect.rs`, `src/sim/crates.rs`,
`src/sim/production/production_spawn.rs`,
`src/sim/production/production_refinery.rs`, and
`src/sim/movement/movement_path.rs`.

## Summary

- 19 documented candidate behaviors inventoried
- 19 active-YR core/start claims freshly verified
- 10 verified gaps
- 2 caller-specific candidates awaiting exact caller-row closure
- 7 verified matches or false positives
- 3 deferred or prerequisite-blocked gaps/candidates

This report is a dated disparity snapshot, not a parity percentage or
completion certificate.

## Verified gaps

### HIGH priority

**G1. The shared Rust FNPC omits the mandatory candidate-anchor playfield gate.**

- **Active-YR evidence:** every candidate side in `0x0056DC20` calls
  `MapClass__Is_Cell_In_Playfield_CellClass(cell, 1) @ 0x00578540` before
  `CellRect__CheckPassability`, at `0x0056DDC0`, `0x0056DFD6`,
  `0x0056E217`, and `0x0056E419`.
- **Research pointer:**
  `docs/research/skirmish-ui/FOOTCLASS_FIND_NEARBY_PASSABLE_CELL_0056DC20_START_FALLBACK_GHIDRA_REPORT.md`
  sections 3.3 and 9.
- **Rust state:** `candidate_passes` in `src/sim/find_nearby_cell.rs:346`
  starts with rectangle passability and only reaches playfield logic through
  optional final occupancy. The local start helper independently performs the
  anchor gate at `src/sim/scenario_bootstrap.rs:194`, proving the shared core is
  not authoritative for this column.
- **Exact verdict:** DRIFT.
- **Priority rationale:** any represented no-final-occupancy caller, including
  crate placement and deficient starts, can admit a passable border/halo anchor
  that retail rejects. Border triggers are narrower than center-map calls, but
  the result changes placement and lockstep state whenever reached.

**G2. The shared query fixes every passability/occupancy footprint to `1x1`; the active start caller needs `8x8`.**

- **Active-YR evidence:** `0x00688380` passes width/height `8,8`; all four
  `0x0056DC20` candidate paths forward caller width/height to
  `CellRect__CheckPassability @ 0x0056E7C0`, and optional occupancy uses the same
  rectangle.
- **Research pointer:** corrected deficient-start report sections 2, 3.4, and 9;
  `CELLCLASS_SUBSTRATE_CELLRECT_VALIDATOR_CONTRACTS_GHIDRA_REPORT.md` section 3.
- **Rust state:** `src/sim/find_nearby_cell.rs:353` constructs
  `CellRect::new(cx, cy, 1, 1)`. Start gathering bypasses it through
  `find_nearby_start_rect` and `deficient_start_rect_track_passable` at
  `src/sim/scenario_bootstrap.rs:155` and `:564`.
- **Exact verdict:** DRIFT.
- **Priority rationale:** on a deficient map, one center-passable cell is not a
  legal replacement start; all 64 cells must pass. This can decide every
  participant's initial position and therefore the whole match.

**G3. Deficient-start Rust selects the first accepted cell instead of the native preferred pool and frame modulo.**

- **Active-YR evidence:** `0x0056DC20` stores up to 24 candidates, reclassifies
  each through `0x006D6410`, prefers the direct pool, and with the caller's zero
  reference selects `g_CurrentFrameCounter % pool_count`. Radius zero emits the
  seed twice because both north/south branches execute. The start caller passes
  a zero reference at `0x006885AB..0x006885B5`.
- **Research pointer:** corrected deficient-start report sections 3.2, 3.5, and
  9; `GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md` sections (a)-(d).
- **Rust state:** `src/sim/scenario_bootstrap.rs:167-175` suppresses the second
  radius-zero seed; `:204-206` returns `accepted.first()`. The helper receives
  no frame counter.
- **Exact verdict:** DRIFT.
- **Priority rationale:** the deficient-map trigger is conditional, but when it
  fires the selected start location changes deterministically and compounds
  through base placement, resource access, and opening movement.

**G4. The crate caller passes a real target `(0,0)` to Rust where native treats `(0,0)` as the null reference.**

- **Active-YR evidence:** fresh decompile of
  `MapClass__PlaceCrateAtRandomCell @ 0x0056BD40` shows a local reference
  initialized to zero and passed to `0x0056DC20`. The callee compares that value
  to `DAT_00ABD480` and uses frame-modulo selection, not nearest-to-origin.
- **Research pointer:** `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
  map/start/crate row.
- **Rust state:** `src/sim/crates.rs:269` sets `target_cell: Some((0, 0))`, which
  selects the nearest-distance branch at `src/sim/find_nearby_cell.rs:178`.
- **Exact verdict:** DRIFT.
- **Priority rationale:** every successful crate-generation attempt passes this
  row. With multiple surviving cells it commonly chooses a different cell,
  changing crate position and subsequent pickup outcomes.

**G5. Production spawn fallback truncates the native map-owned radius to 12.**

- **Active-YR evidence:** all 99 static `0x0056DC20` callsites load `g_Map` as
  receiver; the callee uses `Size.width + Size.height`, capped at 32. No caller
  supplies a search radius.
- **Research pointer:** corrected deficient-start report section 3.1; caller
  matrix Verified Contract Inputs.
- **Rust state:** `src/sim/production/production_spawn.rs:373-390` explicitly
  preserves a retired local radius of 12 in `SPAWN_FALLBACK_RADIUS`.
- **Exact verdict:** DRIFT.
- **Priority rationale:** only blocked/nonpreferred production exits reach the
  fallback, but factories and refineries exercise that condition routinely on
  congested bases; a valid cell at radius 12..31 is incorrectly missed.

### MEDIUM priority

**G6. The shared core has no native `param_15` half-ring mode.**

- **Active-YR evidence:** live `0x0056DC20` skips the north and west candidates
  when `param_15 != 0`, also skipping the first south corner, while the east
  column remains active.
- **Research pointer:** `GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md` section (b).
- **Rust state:** `NearbySearchOptions` at `src/sim/find_nearby_cell.rs:86`
  contains only overlay rejection; `ring_cells` at `:269` always emits the full
  ring.
- **Exact verdict:** DRIFT.
- **Priority rationale:** directional/wrapper callers use this to bias the
  candidate stream. It is narrower than the default row but changes cap and
  tie order whenever enabled.

**G7. The shared core has no native `param_12` current-cell-obstacle column.**

- **Active-YR evidence:** `0x0056DC20` conditionally calls
  `TechnoClass__Is_Current_Cell_Obstacle_Free @ 0x00486FF0` after height. Fresh
  decompile confirms the helper checks the anchor diamond, tile
  `AllowBurrowing` (`IsometricTileTypeClass+0x305`), slope/`Flags&0x500`, and
  ground-list Building/TerrainClass blockers.
- **Research pointer:** caller matrix command/path rows;
  `ALLOWTIBERIUM_THEATER_READER_AND_RUST_SURFACE_GHIDRA_REPORT.md` for the
  corrected `+0x305` identity.
- **Rust state:** `NearbyQuery` at `src/sim/find_nearby_cell.rs:96` exposes no
  obstacle-free flag or equivalent predicate. Rust also does not retain parsed
  `AllowBurrowing` on `ResolvedTerrainCell`.
- **Exact verdict:** DRIFT.
- **Priority rationale:** the gate is caller-dependent and not used by the
  deficient-start/crate rows, but active movement correction rows can accept
  anchors native rejects.

**G8. Deficient-start Rust adds a full-rectangle axis-aligned bounds gate that native does not have.**

- **Active-YR evidence:** native shape-tests only the top-left anchor through
  `0x00578540`. `CellRect__CheckPassability` performs fixed-index real/dummy
  lookups over the other 63 cells and has no rectangle-bounds or four-corner
  playfield call.
- **Research pointer:** corrected deficient-start report sections 3.3, 3.4, and
  Negative Facts.
- **Rust state:** `src/sim/scenario_bootstrap.rs:180-186` rejects unless the
  complete `8x8` rectangle lies within `NativeStartBounds` before either native
  predicate runs.
- **Exact verdict:** DRIFT.
- **Priority rationale:** edge-only but match-initialization-visible; it can
  reject a native candidate whose outlying footprint cells resolve through the
  shared dummy contract.

### LOW priority / exactification residuals

**G9. Direct/indirect projection omits the verified raw-`0x1000` bridge correction.**

- **Active-YR evidence:** fresh `FUN_006D6410 @ 0x006D6410` decompile reads the
  candidate's `CellClass+0x140 bit 0x1000`; when set, a probed structural-bridge
  cell contributes `+4` levels before projection.
- **Research pointer:** legacy ring report identifies `0x006D6410` but leaves
  the flag decode implicit.
- **Rust state:** `src/sim/find_nearby_cell.rs:320-327` explicitly records the
  rule as `UNCHECKED` and omits it. The preceding Phase 3 bridge slice now
  retains exact real/dummy `0x1180` flags, so the required input exists.
- **Exact verdict:** DRIFT.
- **Priority rationale:** requires a candidate with raw `0x1000` within six
  projected steps of a structural bridge, so it is uncommon but determinism-
  visible near affected bridges.

**G10. The height gate omits the seed-bridge `+4` adjustment when bridge-aware mode is enabled.**

- **Active-YR evidence:** fresh `0x0056DC20` decompile reads the seed cell once
  and adds four to its signed level when `param_7 != 0` and seed
  `Flags&0x100` is set.
- **Research pointer:** core report section 4d.
- **Rust state:** `src/sim/find_nearby_cell.rs:416-427` explicitly records this
  residual and reads the raw seed level.
- **Exact verdict:** DRIFT.
- **Priority rationale:** excluded from the nonbridge start/crate rows, but real
  for bridge-aware movement correction; rare boundary trigger, exact outcome
  drift when reached.

## Doc-derived candidates needing verification

These are not confirmed implementation instructions for a caller row.

**C1. Production spawn's post-selection land/water/subcell filter may reject the chosen result instead of filtering the native pool.**

- **Doc claim:** caller-matrix production/exit rows use caller-specific FNPC
  passability flags; `production_spawn.rs` comments admit its extra filter is not
  encoded by FNPC.
- **Rust state:** `src/sim/production/production_spawn.rs:323-334` selects first,
  then returns `None` if `cell_available_for_spawn` rejects that cell.
- **Missing proof:** exact stack decode for the corresponding
  `BuildingClass__ExitObject_Main` / production-completion call and whether the
  extra native rule is pre-FNPC, inside another native predicate, or after the
  chosen cell.
- **Potential impact:** congested factory exits can fail despite a later valid
  candidate.

**C2. Free-unit completion's two FNPC rows may have an additional occupancy or zone requirement.**

- **Doc claim:** the caller matrix marks the two
  `BuildingClass__OnConstructionComplete` rows as live but leaves semantic names
  for the retry flags open.
- **Rust state:** `src/sim/production/production_refinery.rs:330-380` explicitly
  drops same-zone and final-occupancy behavior as residuals.
- **Missing proof:** assembly push-order walk for both `0x00446CCD` and
  `0x00446E10`, plus the seed-zone source.
- **Potential impact:** every refinery free-unit fallback can choose a different
  landing cell when the primary cell is blocked.

## Deferred / blocked by prerequisites

- **Bridge-aware GetZoneID redirect** - ACTIVE-YR VERIFIED gap in
  `src/sim/cell_rect.rs:779`; blocked on the exact GSI-04.03/04.04 bridge-zone
  redirection contract. Nonbridge rows in this report pass `param_7 = 0` and do
  not depend on it.
- **`param_12` full implementation** - ACTIVE-YR VERIFIED gap; blocked on
  retaining theater `AllowBurrowing` metadata plus exact TerrainClass/building
  ground-list classification. The start and crate rows explicitly pass zero.
- **Complete caller migration** - ACTIVE-YR VERIFIED core with 99 callsites;
  callers whose owning gameplay systems are not represented remain recorded,
  not treated as correctly absent. Each represented caller still needs its own
  exact stack row and builder/critic closure.

## Doc errors discovered

- **`docs/research/pathfinding/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` sections 1-2** - calls the receiver FootClass and `+0xF4/+0xF8` Speed/Sight. Fresh assembly context for all 99 static calls proves every caller loads `g_Map`; the fields are MapClass Size width/height.
- **The same core report section 4b and `fn-find_nearby_passable_cell.md`** - use the stale `TechnoClass__IsOnScreen` label for `0x00578540`. The active function is `MapClass__Is_Cell_In_Playfield_CellClass` and applies the height-aware playfield diamond.
- **Legacy “diamond ring” prose** - the actual cell-coordinate equations enumerate a Chebyshev square perimeter: complete north/south rows followed by west/east interior columns. The fixed equations/order are authoritative; “diamond” is only presentation shorthand.
- **Core report section 4e** - identifies tile `+0x305` as `IsInsignificant`; verified IsometricTileType metadata identifies it as `AllowBurrowing`.

## Appendix - verified matches and false positives

| Preliminary claim | Evidence state | Actual Rust state |
|---|---|---|
| Full-ring order for `param_15=0` | ACTIVE-YR VERIFIED | Matches `ring_cells` at `src/sim/find_nearby_cell.rs:269`, including N/S then W/E order. |
| Radius-zero duplicate | ACTIVE-YR VERIFIED | Shared `ring_cells` matches; reduced start helper does not. |
| Candidate cap 24 | ACTIVE-YR VERIFIED | Matches `MAX_CANDIDATES` and mid-ring return. |
| Finish first direct ring | ACTIVE-YR VERIFIED | Matches `collect_candidates`; bridge-aware early-stop asymmetry is represented. |
| Direct pool preferred | ACTIVE-YR VERIFIED | Matches selection at `src/sim/find_nearby_cell.rs:163-171`. |
| No-target frame modulo | ACTIVE-YR VERIFIED | Shared core matches when caller uses `None`; crate caller maps the sentinel incorrectly. |
| Target nearest Euclidean, first tie | ACTIVE-YR VERIFIED | Integer squared distance is monotonic and stable-first, so the selected result matches. |

## Ghidra annotation candidates

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x0056DC20` | `FootClass__Find_Nearby_Passable_Cell` | `MapClass__Find_Nearby_Passable_Cell` plus plate note that radius is `min(Size.width+Size.height,32)` | label/comment | all 99 static callsites load `ECX=0x0087F7E8`; start/crate decompiles bind MapClass fields | deferred; read-only scan |

## Recommendations

First close one coherent mechanism: expand the shared query just enough to own
the exact deficient-start row (`8x8`, mandatory anchor gate, native duplicate/
pool/selection, actual frame input), migrate `find_nearby_start_rect` to it, and
remove the stricter full-footprint bounds precheck. Preserve the already-matching
shared selection/ring behavior. Give that diff and focused output to a fresh
critic before touching another caller.

Then close the crate null-reference mapping and production radius as separate
caller mechanisms. Resolve `param_15`, raw-`0x1000` projection, and the seed
bridge correction in the shared core before claiming nonbridge FNPC complete.
Do not promote C1/C2 until their exact caller stack rows are freshly decoded.
