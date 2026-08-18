# Canonical Direction Encoding - Ghidra Research Report

**Address(es):** `0x0089F688` direction table, `0x0049F2F0` initializer, `0x0042D490`, `0x00429780`, `0x00429A90`, `0x004D9C60`, `0x0047E040`, `0x0047E470`, `0x00480510`, `0x00452A40`, `0x00452DC0`, `0x004533A0`, `0x00480630`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** canonical 0..7 cell-neighbor direction encoding, the direction-8 tube exception, `(dir - 4) & 7` opposite behavior, and scoped pathfinding/bridge/wall consumers.
**Non-Scope:** 8-bit/16-bit facing conversion, turret/FLH axes, tactical screen inverse, full bridge state machine, full A* cost model, and all tube producer invariants.
**Confidence:** High for the encoding contract and scoped consumers; Medium for "no different convention anywhere" because this was a major-consumer slice, not a whole-binary census.
**Active in YR:** Yes. The table is initialized before `WinMain`; A*, bridges, walls, tubes, and map-coordinate stepping are live YR systems.

## 0. Investigation Notes

**Target question:** What exact 0..7 direction encoding does gamemd.exe use for cell-neighbor offsets, which live YR consumers depend on it, and where is direction `8` a special-case tube jump rather than a ninth compass direction?

**Non-goals:** 8-bit/16-bit facing-to-render conversion, tactical screen pixel inverse math, FLH/turret offset axes, full bridge damage state machine, full A* cost model, and all INI parser behavior outside keys that directly name direction/wall/bridge/tube behavior.

**Evidence needed to mark COMPLETE:** verify `g_DirectionOffsets @ 0x0089F688` initialization and table values; verify at least one A*/path consumer, one bridge traversal consumer using `(dir - 4) & 7`, one bridge SetBridgeDirection consumer, one wall/connection consumer, and the `dir == 8` tube exception; scan current Rust surfaces for convention mismatches; classify active-in-YR status and TS-legacy risk.

**Stop conditions:** no open material questions in this bounded slice; any unresolved consumer outside the named major surfaces is explicitly deferred; zero-add final Ghidra pass over the primary table initializer and scoped consumers. Final pass re-decompiled `0x0049F2F0`, `0x0042D490`, `0x004D9C60`, `0x0047E040`, and `0x00480510`; no new open questions were added.

## 1. Overview

The canonical RA2/YR cell-direction encoding is clockwise from map north:

| Direction id | Name | `(dx, dy)` |
|---:|---|---:|
| 0 | N | `(0, -1)` |
| 1 | NE | `(1, -1)` |
| 2 | E | `(1, 0)` |
| 3 | SE | `(1, 1)` |
| 4 | S | `(0, 1)` |
| 5 | SW | `(-1, 1)` |
| 6 | W | `(-1, 0)` |
| 7 | NW | `(-1, -1)` |

`dir == 8` is not a compass direction in the scoped path helpers. It is a tube jump: read the current cell's `CellClass+0x116` tube index, and if it is not `-1`, jump to `g_TubeArray[index]+0x28`; otherwise return packed coord `(0,0)`.

No verified scoped consumer used a different 0..7 compass order. Wall systems use only the cardinal subset `0,2,4,6`, bridge systems use the same table plus `(dir - 4) & 7` for the opposite cell, and A* passes the same direction id into `Can_Enter_Cell`.

## 2. Key Offsets / Tables

| Item | Type | Meaning | Active in YR | Evidence |
|---|---|---|---|---|
| `0x0089F688` | `short[8][2]` | global `(dx,dy)` table | Yes | `Foundation_direction_table_init @ 0x0049F2F0` |
| `CellClass+0x24/+0x26` | signed shorts | packed map cell X/Y | Yes | consumers add table shorts to these fields |
| `CellClass+0x116` | signed short | tube index; `-1` is no tube | Conditional | `0x0042D490`, `0x00429780`, `0x00429A90` |
| `g_TubeArray` | pointer table | tube pointer by `Cell+0x116` | Conditional | `0x0042D490`, `0x00429780`, `0x00429A90` |
| `TubeClass+0x28` | packed cell coord | direction-8 destination/exit | Conditional | same |
| `CellClass+0x140 & 0x100` | bridge structural bit | bridge traversal gate | Yes | `0x004D9C60`, `0x00429A90` |
| `CellClass+0x140 & 0x200` | bridgehead bit | required for upward bridge entry | Yes | `0x004D9C60` |
| `CellClass+0x140 & 0x40000` | temporary A* bridge marker | 4x cost multiplier marker | Yes | `0x00429A90`, prior `GDIRECTIONOFFSETS...` report |

## 3. Core Logic

### Table initialization

`Foundation_direction_table_init @ 0x0049F2F0` writes eight dwords into runtime storage:

| Address | Dword | Decoded signed shorts | Direction |
|---|---:|---:|---|
| `0x0089F688` | `0xFFFF0000` | `(0, -1)` | N |
| `0x0089F68C` | `0xFFFF0001` | `(1, -1)` | NE |
| `0x0089F690` | `0x00000001` | `(1, 0)` | E |
| `0x0089F694` | `0x00010001` | `(1, 1)` | SE |
| `0x0089F698` | `0x00010000` | `(0, 1)` | S |
| `0x0089F69C` | `0x0001FFFF` | `(-1, 1)` | SW |
| `0x0089F6A0` | `0x0000FFFF` | `(-1, 0)` | W |
| `0x0089F6A4` | `0xFFFFFFFF` | `(-1, -1)` | NW |

Active in YR: Yes. Prior report verified constructor-table reachability via data xref `0x00812BAC`; this pass re-decompiled the initializer and found the same writes. Static PE memory may read as zero because this is runtime-populated storage.

### Generic coordinate stepping

`MapCoord_Step_By_Direction @ 0x0042D490`:

- If `param_3 != 8`, it indexes `g_DirectionOffsets + param_3 * 4` for Y and `g_DirectionOffsets + param_3` for X, then writes `current + (dx,dy)`.
- If `param_3 == 8`, it gets the current `CellClass`, checks `Cell+0x116 != -1`, and returns `TubeClass+0x28`; if `Cell+0x116 == -1`, it returns `0`.
- It does not mask or bounds-check `param_3` in the non-8 branch.

Active in YR: Yes. This is a generic helper and matches `Path_walk_directions_to_cell @ 0x00429780`.

### Path direction replay

`Path_walk_directions_to_cell @ 0x00429780` repeats the same contract over a path buffer:

- `param_3 < 1` returns the start coord unchanged.
- For each path entry: `8` takes the tube branch; any other value indexes `g_DirectionOffsets` directly.
- Direction `8` with no tube index returns packed coord `0`.

`AStar_main_loop @ 0x00429A90` expands normal directions `0..7`, then treats loop index `8` as the tube case. For normal directions, it passes the same direction id to the object `Can_Enter_Cell` virtual (`vtable+0x1AC`). For `8`, it computes the tube destination and uses Chebyshev distance as the edge cost instead of the normal edge-cost table.

Active in YR: Yes. This is live pathfinding, not TS-only dead code.

### Opposite direction

The verified opposite operation is `(dir - 4) & 7`.

`CheckBridgeTraversal @ 0x004D9C60` uses it when `param_5 == 0`: it computes `uVar1 = param_2 - 4U & 7`, steps from `param_1` by that opposite offset, and gets the neighboring `CellClass`. This means:

- N `0` opposite is S `4`.
- NE `1` opposite is SW `5`.
- E `2` opposite is W `6`.
- SE `3` opposite is NW `7`.

`CellClass__SetBridgeDirection_NESW @ 0x0047E040` and `CellClass__SetBridgeDirection_NWSE @ 0x0047E470` also use the same opposite calculation for the fourth touched neighbor after three forward steps.

Active in YR: Yes. Bridge passability and bridge damage/repair use these paths in standard YR.

### Bridge SetBridgeDirection consumers

`SetBridgeDirection_NESW` and `SetBridgeDirection_NWSE` are functionally identical for direction indexing:

- `param_2 < 8` gates the three forward table steps.
- Each forward step reads `g_DirectionOffsets[(param_2 & 7)]`.
- The opposite slot is `uVar15 = param_2 - 4 & 7`.
- The `param_2 == 6` special case uses `DAT_0089F690`, which is the E offset `(1,0)`, not a different direction convention.

Active in YR: Yes. The scoped question is direction encoding; wider bridge state-byte and flag mutation details remain covered by bridge-specific reports.

### Wall consumers

Wall placement/connectivity uses the same compass ids, restricted to cardinals:

- `BuildingClass__ConnectWalls @ 0x00452A40` initializes `uVar8=0`, then advances `uVar8 = uVar8 + 2 & 7`, visiting `0,2,4,6`.
- `BuildingClass__RecalculateWallConnections @ 0x004533A0` uses the same `uStack_40 = uStack_40 + 2 & 7` cardinal loop and indexes `g_DirectionOffsets`.
- `CellClass__IsWallConnectableInDirection @ 0x00480510` treats `param_3 == 2 || 6` as one wall-building axis and `param_3 == 0 || 4` as the other.
- `BuildingClass__ExtendWallInDirection @ 0x00452DC0` walks repeatedly by `g_DirectionOffsets[param_2 & 7]`.
- `CellClass__PostDestructionWallCleanup @ 0x00480630` uses `DAT_0081CC70..` direction entries to visit self/cardinal neighbors and recompute wall connectivity; its neighbor stepping still uses `g_DirectionOffsets`.

Active in YR: Yes. `Wall=yes` overlays and wall buildings are standard YR content. This is not a separate 0..7 convention; it is the cardinal subset of the canonical table.

## 4. INI Keys

No INI key defines or overrides the canonical direction table.

| Key / data | Value observed | Effect in this slice | Active in YR |
|---|---|---|---|
| `[General] WindDirection` | `rulesmd.ini:399` comment says 0 is north | FacingType-related, not the cell-neighbor table | Yes, but non-scope |
| `DeployFacing` | examples comment `0 = N, 7 = NW` | Confirms authored 0..7 labels align with canonical order; facing behavior itself is separate | Conditional by object |
| `[Tubes]` entry direction/path steps | Rust parser validates `0..=7`; binary path helpers reserve `8` internally for tube jump | Map-authored tube path steps are normal directions; `8` is not accepted as a path-step direction in current Rust | Conditional on maps |
| `Wall=yes`, `BridgeStrength`, `DestroyableBridges` | standard YR rules | Enable wall/bridge systems that consume direction ids | Yes |

## 5. Integration Points

| Consumer | Direction behavior | Active in YR | Evidence |
|---|---|---|---|
| `MapCoord_Step_By_Direction @ 0x0042D490` | `0..7` direct table, `8` tube jump | Yes | decompile |
| `Path_walk_directions_to_cell @ 0x00429780` | path replay over same contract | Yes | decompile |
| `AStar_main_loop @ 0x00429A90` | expands directions `0..7`, then tube `8`; passes id to `Can_Enter_Cell` | Yes | decompile |
| `AStar_reconstruct_path @ 0x0042AA90` | writes `8` when parent/current delta is not adjacent; otherwise uses direction lookup table | Yes | decompile |
| `CheckBridgeTraversal @ 0x004D9C60` | `(dir - 4) & 7` opposite when previous cell not provided | Yes | decompile |
| `SetBridgeDirection_* @ 0x0047E040/0x0047E470` | forward `dir`, opposite `(dir - 4) & 7`, special W case | Yes | decompile |
| wall connection functions | cardinal subset `0,2,4,6` | Yes | decompile |

Tick-cycle placement was not exhaustively traced because this slice is a shared data contract. All listed consumers are live in standard YR gameplay when their systems run.

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta / note |
|---|---|---|
| `src/sim/bridge_state/mod.rs:199` `Direction` | `repr(u8)` N=0, NE=1, E=2, SE=3, S=4, SW=5, W=6, NW=7 | Matches binary contract. |
| `src/sim/bridge_state/mod.rs:227` `Direction::opposite` | explicit N<->S, NE<->SW, E<->W, SE<->NW | Matches `(dir - 4) & 7` semantically. |
| `src/sim/bridge_state/mod.rs:1859` `compute_anchor_span_cells` | slots 1..3 walk forward, slot 4 walks `opposite`, W special slot 5 | Matches scoped SetBridgeDirection shape. |
| `src/sim/pathfinding/core.rs` `NEIGHBORS` | N, NE, E, SE, S, SW, W, NW | Matches. |
| `src/sim/pathfinding/path_smooth.rs` `DIR_DELTAS` | same table | Matches. |
| `src/map/resolved_terrain.rs:298` `step_coord_by_direction` | `8` returns tube exit or `(0,0)` | Matches scoped direction-8 behavior for valid caller inputs. |
| `src/map/resolved_terrain.rs:1121` `direction_offset` | uses `direction & 7` | Potential mismatch for invalid `9..=255`: binary generic helper indexes directly in non-8 branch; Rust wraps. Existing parsers mostly validate, but future callers should avoid relying on wrap. |
| `src/app_commands.rs:346` wall fill | cardinal offsets N,E,S,W | Direction order matches cardinal subset, but it is hand-coded rather than using shared canonical table. |
| `src/map/tubes.rs` | validates authored tube path steps `0..=7` | Reasonable. Direction `8` is an internal path/replay sentinel, not map-authored normal direction. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `g_DirectionOffsets` table values | verified | `0x0049F2F0` decompile | none |
| table startup liveness | verified via prior report, spot-checked by initializer | `GDIRECTIONOFFSETS_0089F688...`; `0x0049F2F0` | live watchpoint optional |
| `MapCoord_Step_By_Direction` non-8 branch | verified | `0x0042D490` | none |
| `MapCoord_Step_By_Direction` dir-8 branch | verified | `0x0042D490` | tube producer invariants deferred |
| `Path_walk_directions_to_cell` | verified | `0x00429780` | none |
| `AStar_main_loop` direction loop | verified for direction contract | `0x00429A90` | full A* cost/order out of scope |
| `AStar_reconstruct_path` dir-8 output for non-adjacent deltas | verified | `0x0042AA90` | exact direction lookup table contents not separately dumped |
| `CheckBridgeTraversal` opposite formula | verified | `0x004D9C60` | none |
| `SetBridgeDirection_NESW/NWSE` forward/opposite stepping | verified | `0x0047E040`, `0x0047E470` | full bridge state machine out of scope |
| Wall cardinal consumers | verified | `0x00452A40`, `0x00452DC0`, `0x004533A0`, `0x00480510`, `0x00480630` | exact `DAT_0081CC70` memory values not dumped, but use sites show cardinal contract |
| Current Rust scan | verified enough for handoff | targeted reads + codegraph | whole-repo convention census deferred |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What is the canonical 0..7 table? -> N, NE, E, SE, S, SW, W, NW with signed offsets listed in Section 1.` (evidence: `0x0049F2F0`)
- `[RESOLVED] OQ-2 - Is the table active in YR? -> Yes, startup initializer writes it before gameplay; path/bridge/wall systems read it.` (evidence: `0x0049F2F0`; prior `GDIRECTIONOFFSETS_0089F688...`)
- `[RESOLVED] OQ-3 - Is direction 8 a compass direction? -> No. In scoped path helpers it is the tube branch.` (evidence: `0x0042D490`, `0x00429780`, `0x00429A90`)
- `[RESOLVED] OQ-4 - What happens for dir 8 with no tube? -> The packed coordinate becomes zero.` (evidence: `0x0042D490`, `0x00429780`)
- `[RESOLVED] OQ-5 - Is there a tube-index upper-bound check in these consumers? -> No; they check only `Cell+0x116 == -1` before indexing `g_TubeArray`.` (evidence: `0x0042D490`, `0x00429780`)
- `[RESOLVED] OQ-6 - What is the opposite formula? -> `(dir - 4) & 7`.` (evidence: `0x004D9C60`, `0x0047E040`, `0x0047E470`)
- `[RESOLVED] OQ-7 - Do bridges use a different 0..7 order? -> No verified difference; bridge functions index the same table and use the same opposite formula.` (evidence: `0x0047E040`, `0x0047E470`, `0x004D9C60`)
- `[RESOLVED] OQ-8 - Do walls use a different 0..7 order? -> No; wall code uses cardinal subset `0,2,4,6`.` (evidence: `0x00452A40`, `0x004533A0`, `0x00480510`)
- `[RESOLVED] OQ-9 - Does A* pass the same direction id to passability? -> Yes, normal loop passes `iStack_44` to `vtable+0x1AC`; `iStack_44==8` is the tube case.` (evidence: `0x00429A90`)
- `[RESOLVED] OQ-10 - Does Rust bridge direction enum match? -> Yes for discriminants, offsets, and opposite mapping.` (evidence: `src/sim/bridge_state/mod.rs`)
- `[RESOLVED] OQ-11 - Does Rust generic step helper match invalid direction behavior? -> For valid 0..8 inputs yes; for invalid `9..=255`, Rust wraps with `&7` while binary generic helper indexes directly.` (evidence: `src/map/resolved_terrain.rs:1121`; `0x0042D490`)
- `[DEFERRED] OQ-12 - Are there any obscure non-path/bridge/wall systems with a different 0..7 convention?` (category: `bounded-cost-too-high`; reason: this slot was scoped to major coordinate consumers, not a whole-binary census; next-step-if-pursued: xref all reads of `0x0089F688` and every 8-entry direction table)
- `[DEFERRED] OQ-13 - What are all producer invariants for positive `Cell+0x116` tube indices?` (category: `out-of-scope`; reason: this report verifies consumers only; next-step-if-pursued: TubeClass lifecycle and map-load audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Canonical direction ids are N, NE, E, SE, S, SW, W, NW | `0x0049F2F0` | none observed in main tables | `src/sim/bridge_state/mod.rs`, `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/path_smooth.rs` | Keep one shared contract for all cell-neighbor code | `canonical_direction_offsets_match_gamemd_table` | Do not introduce keypad/order-by-screen convention. |
| Opposite direction is `(dir - 4) & 7` | `0x004D9C60`, `0x0047E040` | none observed for bridge enum | `Direction::opposite`, bridge span helpers | Opposite of every direction must pair across 180 degrees | `canonical_direction_opposite_matches_wrapped_subtract_4` | Do not use `(dir + 4) % 8` only in code comments unless tested; it is equivalent but hides the binary expression. |
| Direction `8` is a tube jump to `Tube+0x28`, with no-tube fallback `(0,0)` | `0x0042D490`, `0x00429780` | implemented in `ResolvedTerrainGrid::step_coord_by_direction` | `src/map/resolved_terrain.rs`, `src/map/tubes.rs` | Preserve internal `8` behavior separately from map-authored path steps | `direction8_steps_to_tube_exit_and_missing_tube_returns_origin` | Do not add a ninth direction offset or accept `8` as authored normal tube path step. |
| Non-8 generic helper indexes directly; no valid-range guard in binary helper | `0x0042D490` | Rust wraps invalid `9..=255` via `direction & 7` | `src/map/resolved_terrain.rs::direction_offset` | Prefer rejecting invalid `>8` at API boundary or document sanitizer behavior | `invalid_direction_above_8_is_rejected_not_wrapped` | Do not rely on wrapping invalid direction values for parity. |
| Wall connectivity uses cardinal subset `0,2,4,6` | `0x00452A40`, `0x004533A0`, `0x00480510` | Rust wall fill hand-codes N,E,S,W | `src/app_commands.rs::fill_wall_between_endpoints` | If wall code is centralized, use canonical direction ids/cardinal subset | `wall_fill_scans_cardinals_in_gamemd_direction_order` | Do not scan diagonals for wall auto-fill/connectivity. |
| A* normal directions `0..7` and tube `8` are distinct cases | `0x00429A90`, `0x0042AA90` | pathfinding has separate tube edge; per-search marker overlay remains separate from this report's earlier pathgrid reports | `src/sim/pathfinding/core.rs` | Keep tube transitions outside the 8-neighbor array and after normal neighbor ids | `astar_expansion_keeps_direction8_out_of_neighbor_offsets` | Do not fold tube edges into `NEIGHBORS`. |

## Negative Facts / Do Not Do

- Do not treat direction `8` as an adjacent offset. It is a tube branch in the scoped helpers.
- Do not let invalid direction values silently wrap if the behavior is meant to mirror `MapCoord_Step_By_Direction`; binary valid callers may sanitize earlier, but this helper itself does not.
- Do not invent a bridge-specific direction order. Bridge code uses the same table.
- Do not invent a wall-specific direction order. Wall code uses the cardinal subset of the same table.
- Do not conflate 0..7 cell direction ids with 8-bit facing bytes. Facing is a different representation and belongs to slot 3.
- Do not assume `SetBridgeDirection_NESW` and `SetBridgeDirection_NWSE` differ in direction indexing; their distinction is caller/context, not a different compass table.

## Remaining Uncertainty

- This is not a whole-binary xref census of every possible direction-like table. It covers the requested major consumers: pathfinding, generic coordinate stepping, bridges, walls, and tubes.
- Live debugger memory was unavailable, so the report relies on constructor and consumer decompilation rather than a runtime memory dump.
- Tube producer invariants for invalid positive `Cell+0x116` values remain outside this report.

## Proposed Rust Test Names

- `canonical_direction_offsets_match_gamemd_table`
- `canonical_direction_opposite_matches_wrapped_subtract_4`
- `direction8_steps_to_tube_exit_and_missing_tube_returns_origin`
- `invalid_direction_above_8_is_rejected_not_wrapped`
- `wall_fill_scans_cardinals_in_gamemd_direction_order`
- `astar_expansion_keeps_direction8_out_of_neighbor_offsets`
- `bridge_set_direction_span_uses_forward3_and_opposite1`

## Sources

- Ghidra: `Foundation_direction_table_init @ 0x0049F2F0`
- Ghidra: `MapCoord_Step_By_Direction @ 0x0042D490`
- Ghidra: `Path_walk_directions_to_cell @ 0x00429780`
- Ghidra: `AStar_main_loop @ 0x00429A90`
- Ghidra: `AStar_reconstruct_path @ 0x0042AA90`
- Ghidra: `CheckBridgeTraversal @ 0x004D9C60`
- Ghidra: `CellClass__SetBridgeDirection_NESW @ 0x0047E040`
- Ghidra: `CellClass__SetBridgeDirection_NWSE @ 0x0047E470`
- Ghidra: `CellClass__IsWallConnectableInDirection @ 0x00480510`
- Ghidra: `BuildingClass__ConnectWalls @ 0x00452A40`
- Ghidra: `BuildingClass__ExtendWallInDirection @ 0x00452DC0`
- Ghidra: `BuildingClass__RecalculateWallConnections @ 0x004533A0`
- Ghidra: `CellClass__PostDestructionWallCleanup @ 0x00480630`
- Existing report: `docs/research/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`
- Rust scan: `src/sim/bridge_state/mod.rs`, `src/sim/bridge_specs.rs`, `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/path_smooth.rs`, `src/map/resolved_terrain.rs`, `src/map/tubes.rs`, `src/app_commands.rs`, `src/sim/ore_growth.rs`
- INI scan: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
