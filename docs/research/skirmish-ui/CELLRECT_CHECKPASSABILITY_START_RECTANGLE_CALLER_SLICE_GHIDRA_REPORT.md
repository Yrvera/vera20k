# CellRect CheckPassability Start Rectangle Caller Slice - Ghidra Research Report

**Address(es):** `0x00688380` deficient-start caller, `0x0056DC20` nearby-passable helper, `0x0056E7C0` `CellRect__CheckPassability`, `0x004834A0` `CellClass__CheckCellPassability`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** only the standard selected-Skirmish deficient-start fallback chain `0x00688380 -> 0x0056DC20 -> 0x0056E7C0 -> 0x004834A0`, plus the nearby helper branch values needed to answer what the `8x8` rectangle accepts.  
**Non-Scope:** full nearby-cell search ranking, MCV nearby placement fallback `0x00688ED0`, A* `Can_Enter_Cell`, AI building/site `CheckOccupancy`, all `CellClass` field writers, and broad movement/pathfinding passability.  
**Confidence:** High for caller arguments, full-rectangle dimensions, occupancy-rect disabled state, overlay/wall/bridge conditions reached by this caller, and YR liveness.  
**Active in YR:** Yes, conditional on standard selected Skirmish maps with fewer gathered authored starts than required active participants.

## 0. Swarm Slot Working Notes

- Target question: For deficient selected-Skirmish starts, what terrain/object/overlay/bridge checks are behind the `8x8` passability rectangle passed from `0x00688380` through `FootClass__Find_Nearby_Passable_Cell`?
- Non-goals: Do not audit all nearby-cell callers, all movement/pathfinding, MCV placement fallback, random map generation, or AI base placement.
- Evidence needed to mark COMPLETE: live selected-YR caller proof, exact `0x00688380` argument values, propagation into `CellRect__CheckPassability`, full-rectangle bounds, whether `CellRect__CheckOccupancy` runs, and overlay/object/bridge behavior for this caller.
- Stop conditions: stop once the deficient-start caller's values and all helper branches those values enable/disable are resolved, with unrelated helper branches listed only as negative facts or out-of-scope uncertainty.

## 1. Overview

Deficient selected-Skirmish starts use a full `8x8` rectangle passability check, not a single center tile. The rectangle is validated by `CellRect__CheckPassability` over every cell in the candidate footprint using SpeedType column `1` (`Track`), no required zone, no required height, no blanket overlay rejection, and bridge-aware flag `0`.

The most important correction is that this start fallback does **not** call `CellRect__CheckOccupancy`. The final nearby helper flag that gates `CheckOccupancy(rect, -1)` is passed as `0` by `0x00688380`, so the separate object-list, `Cell+0x4C`, `Cell+0x11C`, reservation, building lookup, and playfield-corner occupancy validator is inactive for this deficient-start slice. Objects can still matter only insofar as they have already affected the per-cell occupation bytes tested inside `CellClass__CheckCellPassability`.

## 2. Caller Argument Contract

The deficient fallback call in `ScenarioClass__Gather_Start_Positions @ 0x00688380` decompiles as:

```text
Find_Nearby_Passable_Cell(
    out,
    seed_cell,
    speed_type = 1,
    required_zone_id = -1,
    movement_zone = 0,
    bridge_aware_zone = 0,
    width = 8,
    height = 8,
    reject_any_overlay = 0,
    height_tolerance_check = 0,
    current_cell_obstacle_free_check = 0,
    allow_bridge_cells = 1,
    reference_cell = (0,0),
    single_direction_mode = 0,
    check_occupancy_rect = 0
)
```

Active in YR: Yes for deficient standard selected Skirmish. Evidence:

- `0x00688380` is already on the selected Skirmish start assignment path via `ScenarioClass__Full_Init`, selected-mode `+0x80`, and `ScenarioClass__AssignStartingPoints`.
- `0x00688572..0x006885B5` sets up the nearby helper call; `EDI` is set to `8` at `0x00688521`, `EBX` is zeroed at `0x0068852E`, `ECX` is set to `0x0087F7E8` at `0x006885A6`, and the final stack argument push for `check_occupancy_rect` is `PUSH EBX` at `0x00688572`.
- The decompiler's resolved call line at `0x00688380` matches these values: `(..., 1, 0xffffffff, 0, 0, 8, 8, 0, 0, 0, 1, &local_20, 0, 0)`.

## 3. Core Logic

### 3.1 `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`

For every candidate on the search rings, the helper first checks that the candidate cell exists/on-screen enough for `TechnoClass__IsOnScreen(cell, 1)`, then calls:

```text
CellRect__CheckPassability(candidate_top_left, 8, 8,
                           speed_type=1,
                           required_zone_id=-1,
                           movement_zone=0,
                           required_height=-1,
                           bridge_aware_zone=0,
                           reject_any_overlay=0)
```

Active in YR: Yes for this fallback. Evidence: four candidate-side calls to `0x0056E7C0` at `0x0056DE0E`, `0x0056E024`, `0x0056E265`, `0x0056E467`; each pushes the caller-provided width/height and flags.

The start caller disables three later candidate predicates:

- Height-near-start tolerance is off because `param_11 = 0`; branch at `0x0056DE1B..0x0056DE4C` is skipped.
- `TechnoClass__Is_Current_Cell_Obstacle_Free` is off because `param_12 = 0`; branch at `0x0056DE52..0x0056DE66` is skipped.
- `CellRect__CheckOccupancy(rect, -1)` is off because `param_16 = 0`; branch at `0x0056DE86..0x0056DEA4` is skipped.

The start caller also passes `param_13 = 1`, so the helper's own candidate-cell structural-bridge reject is disabled. Evidence: branch form `param_13 != 0 || ((cell->Flags & 0x100) == 0)` in decompile `0x0056DC20`.

### 3.2 `CellRect__CheckPassability @ 0x0056E7C0`

This validator is a true rectangle loop. It iterates every `x` offset `0..width-1` and every `y` offset `0..height-1`; for the start caller that is all 64 cells in the `8x8` candidate area. Any failing cell rejects the candidate.

Active in YR: Yes through the deficient-start nearby call. Evidence: decompile `0x0056E7C0`, loop body `0x0056E7CA..0x0056E87C`, and caller dimensions at `0x00688587..0x00688591`.

For this start caller:

- Bounds: the validator computes `index = y * 0x200 + x`; out-of-range or null cell pointer substitutes dummy cell `DAT_00ABDC50` and writes requested coord to `DAT_00ABDC74`. There is no final `MapClass__IsRectInPlayfield` call in `CheckPassability`.
- Overlay blanket reject: disabled because `reject_any_overlay = 0`. Non-wall overlays are not rejected merely for existing.
- Cell passability: every cell calls `CellClass__CheckCellPassability`.

### 3.3 `CellClass__CheckCellPassability @ 0x004834A0`

For the start caller's exact arguments:

- SpeedType is `1`, which the verified speed table maps to `Track`.
- Required zone is `-1`, so `MapClass__GetZoneID` is not called and zone connectivity is not required.
- Required height is `-1`, so exact level/height filtering is not required.
- Occupation mask modifiers are both `0`, so the selected occupation byte must be exactly `0`.
- Because required height is `-1`, a bridge-flagged cell uses alternate occupation byte `Cell+0x128`; non-bridge cells use ground occupation byte `Cell+0x124`.
- Wall overlays are still special even though blanket overlay rejection is disabled. If `Cell+0x44` points to an overlay type with wall flag `OverlayType+0x2A8`, movement zone `0` does not meet the accepted wall-zone set, so the wall overlay rejects the cell.
- Non-wall overlays are not inspected here after the blanket overlay check is skipped.
- Land legality comes from `g_SpeedType_LandType_Table[SpeedType + LandType*9]`; exact `0.0` rejects unless the bridge alternate-occupation path is active.

Active in YR: Yes through `0x0056E7C0`. Evidence: decompile `0x004834A0`; SpeedType table report maps column `1` to `Track`; wrapper pushes two zero occupation modifiers before `CellClass__CheckCellPassability`.

## 4. Direct Answers

| Question | Answer for deficient selected-Skirmish fallback | Evidence | Active in YR |
|---|---|---|---|
| Single center tile or full footprint? | Full `8x8` rectangle; all 64 cells must pass. | `0x00688587..0x00688591`, `0x0056E7C0` loops | Yes, conditional on deficiency |
| Terrain-aware? | Yes. Uses `Track` SpeedType vs each cell's LandType speed entry; `0.0` rejects. | `0x004835D5..0x004835F6`, SpeedType table column 1 | Yes |
| Zone-aware? | No for this caller. Required zone is `-1`; `MapClass__GetZoneID` branch skipped. | caller passes `0xffffffff`; `0x004834BF..0x004834D8` | Yes, disabled by caller |
| Object-free? | Not as a separate object-list/occupancy rectangle. `CheckOccupancy` and current-cell obstacle-free checks are disabled. Only `Cell+0x124/+0x128` occupation bytes can reject object-affected cells. | final arg `0` at `0x00688572`; `0x0056DE86..0x0056DEA4`; `0x00483527..0x00483572` | Yes |
| Overlay-aware? | Partly. Blanket "any overlay" reject is disabled; wall overlays still reject for MovementZone `0`; non-wall overlays do not reject just by existing. | `reject_any_overlay=0`; `0x00483583..0x004835D5` | Yes |
| Bridge-aware? | Partly. The nearby helper does not reject bridge-flagged candidate cells because `allow_bridge_cells=1`; `CheckCellPassability` uses alternate bridge occupation byte when `Cell+0x140 & 0x100` and height is unrestricted. | caller `param_13=1`; `0x00483527..0x0048354E` | Yes, content-dependent |
| Playfield rectangle checked? | Not by `CheckPassability`; out-of-range cells use dummy-cell behavior. Separate `CheckOccupancy` playfield-corner check is disabled for this caller. | full body `0x0056E7C0`; disabled `0x00586780` branch | Yes |

## 5. Current Rust Implementation Status

Current Rust has pieces of the necessary data model, but no start-fallback helper with this exact contract.

| Rust surface | Status |
|---|---|
| `src/app_skirmish.rs::assign_launch_starts` | Currently flags deficient starts as unsupported and does not generate native fallback cells. |
| `src/map/waypoints.rs` | Provides generic sorted multiplayer waypoints; does not build the native start vector plus fallback cells. |
| `src/rules/locomotor_type.rs::SpeedType` and `src/rules/terrain_rules.rs` | SpeedType and per-land costs exist; `Track` maps to the binary SpeedType column needed here. |
| `src/sim/pathfinding/passability.rs`, `terrain_cost.rs`, `core.rs` | Existing passability is useful but not a drop-in replacement because the start fallback needs an `8x8` all-cells check with caller-specific flags and no zone/occupancy rect. |
| `src/sim/occupancy.rs` | May provide occupation information, but must not be wired as `CheckOccupancy` for this specific fallback unless reproducing `Cell+0x124/+0x128` semantics inside `CheckCellPassability`. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-notes target/non-goals/evidence/stop conditions | verified | section 0 | none |
| Selected-Skirmish liveness for `0x00688380` deficient fallback | verified | prior selected path reports plus `0x00688380` call branch | none for this slice |
| Exact nearby helper args from deficient fallback | verified | decompile `0x00688380`; assembly `0x00688521..0x006885B5` | none |
| `CellRect__CheckPassability` dimensions and all-cells loop | verified | `0x0056E7C0`; caller width/height pushes | none |
| `CellClass__CheckCellPassability` branches used by this caller | verified | `0x004834A0` | full non-start callers out-of-scope |
| `CellRect__CheckOccupancy` non-use by this caller | verified | final helper arg `0`; branch `0x0056DE86..0x0056DEA4` skipped | none |
| Object-list fields in `CheckOccupancy` | touched-only-as-negative | broad validator report; `0x00586780` | not part of this caller |
| Exact dummy cell field values | deferred | `0x0056E7C0` fallback to `DAT_00ABDC50` | only needed for pathological out-of-map candidates |
| Runtime screenshot/fixture for a deficient map | deferred | static binary proof only | runtime validation could confirm visible result |

## 7. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this path active in standard YR selected Skirmish? -> Yes, conditional on deficient authored start count; `0x00688380` is reached by the selected start-assignment path and calls `0x0056DC20` in the deficiency branch.` (evidence: `0x00688380`; prior `SKIRMISH_GATHER_START_POSITIONS_DEFICIENT_WAYPOINT_FALLBACK_00688380_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-02 - What dimensions reach `CheckPassability`? -> Width `8`, height `8`.` (evidence: `0x00688587..0x00688591`; `0x0056DE0E` call shape)
- `[RESOLVED] OQ-03 - Is acceptance a single center tile? -> No, `0x0056E7C0` loops over every cell in the `width x height` rectangle.` (evidence: `0x0056E7CA..0x0056E87C`)
- `[RESOLVED] OQ-04 - Does this caller require a zone match? -> No, it passes required zone `-1`.` (evidence: `0x00688380` call line; `0x004834BF..0x004834D8`)
- `[RESOLVED] OQ-05 - Which SpeedType is used? -> SpeedType `1`, the `Track` column.` (evidence: caller arg `1`; `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` column table)
- `[RESOLVED] OQ-06 - Does this caller blanket-reject any overlay? -> No, `reject_any_overlay = 0`.` (evidence: caller arg sequence; `0x0056E832..0x0056E83E`)
- `[RESOLVED] OQ-07 - Do wall overlays still matter? -> Yes, wall overlays are checked inside `CellClass__CheckCellPassability`; MovementZone `0` is not in the accepted wall movement-zone set.` (evidence: `0x00483583..0x004835D5`)
- `[RESOLVED] OQ-08 - Does this caller run `TechnoClass__Is_Current_Cell_Obstacle_Free`? -> No, `param_12 = 0`.` (evidence: caller arg sequence; branch `0x0056DE52..0x0056DE66`)
- `[RESOLVED] OQ-09 - Does this caller run `CellRect__CheckOccupancy`? -> No, final flag `param_16 = 0`.` (evidence: `PUSH EBX` at `0x00688572`; branch `0x0056DE86..0x0056DEA4`)
- `[RESOLVED] OQ-10 - Are object lists checked by this start rectangle? -> Not through `CheckOccupancy`; only per-cell occupation bytes in `CheckCellPassability` can reject object-affected cells.` (evidence: `0x0056DE86..0x0056DEA4`; `0x00483527..0x00483572`)
- `[RESOLVED] OQ-11 - Does this caller reject bridge cells in the nearby helper? -> No, it passes the flag that bypasses the helper's bridge-flag rejection.` (evidence: caller arg `1`; helper condition around `param_13`)
- `[RESOLVED] OQ-12 - Does bridge occupation use a distinct byte? -> Yes, when bridge flag `0x100` is present and height is unrestricted, `Cell+0x128` is used instead of `Cell+0x124`.` (evidence: `0x00483527..0x0048354E`)
- `[RESOLVED] OQ-13 - Does `CheckPassability` perform final rectangle playfield containment? -> No, that belongs to `CheckOccupancy`, which this caller disables.` (evidence: full `0x0056E7C0`; skipped `0x00586780`)
- `[RESOLVED] OQ-14 - Can current Rust reuse placement preview logic directly? -> No evidence supports direct reuse; the native start fallback is Track-SpeedType full-rectangle passability with no Buildable/placement predicate and no `CheckOccupancy` call.` (evidence: `0x004834A0`; Rust scan)
- `[DEFERRED] OQ-15 - What are exact dummy-cell values for out-of-map rectangle cells?` (category: bounded-cost-too-high; reason: not needed for ordinary valid-map fallback contract; next-step-if-pursued: read `DAT_00ABDC50` initialized fields)
- `[DEFERRED] OQ-16 - Which writers set `Cell+0x124/+0x128` for every object class?` (category: requires-different-system-context; reason: this slice verifies the read contract, not all occupation-byte writers; next-step-if-pursued: occupation-byte writer audit)
- `[DEFERRED] OQ-17 - Runtime result for a pathological map with no possible valid `8x8` Track rectangle.` (category: needs-runtime-debugger; reason: static gather loop can retry indefinitely; visible load UX needs fixture/debug observation)

Adversarial corner cases answered:

- A single clear center with water or blocked terrain inside the `8x8` area fails because all 64 cells are checked.
- A non-wall overlay in the area does not fail merely for being an overlay because blanket overlay reject is disabled.
- A wall overlay in the area fails for MovementZone `0`.
- A dynamic object only blocks if the relevant cell occupation byte is nonzero; the `CheckOccupancy` object-list scan is not called.
- A bridge-flagged cell is not rejected by the nearby helper and uses alternate bridge occupation flags in `CheckCellPassability`.

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Deficient-start fallback candidate acceptance is full `8x8` all-cells Track-SpeedType passability, not center-cell passability. | `0x00688587..0x00688591`; `0x0056E7C0`; `0x004834A0`; SpeedType table column `1` | missing; Rust currently has no deficient-start fallback and no exact start-rect validator | `src/app_skirmish.rs::assign_launch_starts`, future map/pathfinding helper using `src/rules/terrain_rules.rs` and `src/sim/pathfinding/*` | Generate fallback starts only at cells whose `8x8` rectangle passes Track-SpeedType land/occupation rules | A map with one authored start and a second seed near water/rock produces a fallback whose entire `8x8` area is Track-passable, not a cell whose center is passable but footprint overlaps blocked terrain. Proposed test: `skirmish_deficient_start_requires_full_8x8_track_passability` | Do not accept a single passable tile or reuse a center-only path grid query. |
| This caller disables `CellRect__CheckOccupancy`, current-cell obstacle-free, required-zone, and blanket overlay rejection. | caller args at `0x00688380`; final `PUSH EBX` at `0x00688572`; skipped branches `0x0056DE52..0x0056DEA4` | potential future risk: Rust may over-block by reusing placement or occupancy-preview logic | `src/app_skirmish.rs`, `src/sim/occupancy.rs`, building placement helpers if considered | Keep the start fallback helper distinct from building placement and occupancy-rect validation; include only occupation-byte-equivalent blocking in the passability step | A candidate with a harmless non-wall overlay and no occupation-byte blocker remains eligible; a wall overlay rejects. Proposed test: `skirmish_deficient_start_overlay_rules_match_track_passability_not_placement` | Do not call a generic placement/occupancy rectangle validator that rejects all overlays, slopes, object lists, or playfield corners beyond this binary path. |
| Bridge-flagged candidate cells are allowed by the nearby helper; passability then uses bridge alternate occupation flags when the bridge flag is present and height is unrestricted. | caller `param_13=1`; helper bridge condition; `0x00483527..0x0048354E` | unchecked; Rust passability/bridge surfaces exist but no start-fallback bridge case | `src/sim/pathfinding/passability.rs`, `src/map/resolved_terrain.rs`, bridge terrain facts used by any fallback helper | Preserve the distinction between bridge rejection and bridge alternate occupation in start fallback | A deficient-start fallback over intact bridge/deck terrain should be judged by Track-SpeedType land speed and bridge occupation byte, not rejected merely because the cell has bridge flag `0x100`. Proposed test: `skirmish_deficient_start_allows_bridge_flag_with_bridge_occupation_rules` | Do not blanket-reject bridge cells in the deficient-start nearby helper. |

### Negative Facts / Do Not Do

- Do not implement the `8x8` requirement as a single center-tile passability check. Active in YR: Yes. Evidence: `0x0056E7C0` loops over width and height; caller passes `8,8` at `0x00688587..0x00688591`.
- Do not run `CellRect__CheckOccupancy` for deficient-start fallback. Active in YR: Yes, disabled by caller. Evidence: final helper arg is `0` (`PUSH EBX`) at `0x00688572`; occupancy branch at `0x0056DE86..0x0056DEA4` is gated by that flag.
- Do not reject every overlay in the fallback rectangle. Active in YR: Yes. Evidence: `reject_any_overlay = 0`; only wall-overlay logic remains active inside `0x004834A0`.
- Do not require same-zone connectivity for deficient-start fallback. Active in YR: Yes. Evidence: caller passes required zone `-1`; `0x004834BF..0x004834D8` only runs when the required zone is not `-1`.
- Do not use building placement/Buildable rules as the start fallback rectangle predicate. Active in YR: Yes. Evidence: `0x004834A0` uses SpeedType/LandType speed and occupation bytes; no building placement predicate or `Buildable` reader is called in this chain.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_GATHER_START_POSITIONS_DEFICIENT_WAYPOINT_FALLBACK_00688380_GHIDRA_REPORT.md`: replace "fallback calls `FootClass__Find_Nearby_Passable_Cell` with `8,8` rectangle dimensions that flow into `CellRect__CheckPassability`; lower-level passability semantics remain separate" with "`fallback calls `FootClass__Find_Nearby_Passable_Cell` with `8,8` dimensions that flow into full-rectangle `CellRect__CheckPassability`: all 64 cells must pass Track-SpeedType land/occupation/wall-overlay rules, required-zone and required-height are disabled, blanket overlay rejection is disabled, bridge cells are not rejected by the helper, and `CellRect__CheckOccupancy` is not called for this start fallback.`"
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`: replace vague "8x8 clearance" wording with "`8x8` full-rectangle Track-SpeedType passability only; not center-tile passability and not the separate `CellRect__CheckOccupancy` object/placement rectangle."
- `C:/Users/enok/Documents/ra2-rust-game-docs/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`: no contradiction; add a caller-specific note if patched later: "For `ScenarioClass__Gather_Start_Positions` deficient fallback, `param_16=0`, so the `Find_Nearby` calls to `CheckOccupancy(rect,-1)` are not reached."

## 9. Remaining Uncertainty

- Exact initialized field values on dummy cell `DAT_00ABDC50` were not dumped; ordinary valid-map behavior is unaffected by this deferral.
- Full writer taxonomy for `Cell+0x124/+0x128` occupation bytes remains outside this slice.
- Runtime UX for pathological maps with no possible `8x8` Track-passable rectangle needs a fixture/debugger observation.

## Sources

- Ghidra read-only decompile: `0x00688380`, `0x0056DC20`, `0x0056E7C0`, `0x004834A0`, `0x00586780`.
- Ghidra assembly context: `0x00688521..0x006885B5`, `0x0056DE0E`, `0x0056DE52..0x0056DEA4`.
- Prior docs: `skirmish-ui/SKIRMISH_GATHER_START_POSITIONS_DEFICIENT_WAYPOINT_FALLBACK_00688380_GHIDRA_REPORT.md`, `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`.
- Rust scan: `src/app_skirmish.rs`, `src/map/waypoints.rs`, `src/rules/locomotor_type.rs`, `src/rules/terrain_rules.rs`, `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/terrain_cost.rs`, `src/sim/occupancy.rs`.
