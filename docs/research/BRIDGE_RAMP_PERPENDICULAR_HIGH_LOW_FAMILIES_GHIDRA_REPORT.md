# Bridge Ramp Perpendicular High/Low Families - Ghidra Research Report

**Address(es):** High `0x00572230`, `0x00572330`, `0x00572440`, `0x005727E0`, `0x00572B80`, `0x00572C90`, `0x00572DA0`, `0x00573170`; Low `0x0056ED40`, `0x0056EE40`, `0x0056EF50`, `0x0056F2F0`, `0x0056F690`, `0x0056F7A0`, `0x0056F8B0`, `0x0056FC80`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** ramp-perpendicular `UpdateRamp_*_{High,Low}` helper mutation behavior, direct caller liveness from `ProcessBridgeDamageStateMachine_{High,Low}`, and whether low helper bodies mirror high helper bodies.
**Non-Scope:** full bridge collapse walkers, full rim refresh algorithms, numeric recovery of runtime-initialized BSS tile constants, complete `SetBridgeDirection_*` semantics beyond direct collapse-call effects.
**Confidence:** High for helper body behavior and caller xrefs; Medium for semantic names of runtime tile buckets because the numeric values remain BSS globals.
**Active in YR:** Yes. The helpers are called from active bridge damage state machines reached by bridge damage.

## Target Question

Do the high `UpdateRamp_*_High` helpers mutate the perpendicular ramp/bridgehead cells as assumed, does the low `0x0056ED40...` family mirror them, and how should the `0x00572B80` high EW DamageA address be named relative to the four high addresses originally listed?

## Non-Goals

- Do not re-investigate the full bridge collapse walkers.
- Do not rewrite or verify `UpdateAdjacentBridges_High` / rim-refresh internals.
- Do not recover runtime numeric values for `DAT_00ABAD30`, `DAT_00AA1028`, pavement bucket globals, or tile base globals.
- Do not modify Rust, INI, or other research docs.

## Evidence Needed To Mark COMPLETE

- Decompile all 16 helper bodies, not only representative siblings.
- Verify direct callers/callees for high and low state machines.
- Resolve whether `0x00572B80` is a high EW DamageA sibling or a mislabeled low helper.
- Record every direct field write in the helpers and the helper-called write surfaces they invoke.
- State exactly where low mirrors high and where low live dispatch does not use low helpers.

## Stop Conditions

- Stop at direct `UpdateRamp_*` helper body behavior plus direct helper callees.
- Stop before full `BlowUpBridge`, full `SetBridgeDirection_*`, or rim-refresh algorithm analysis unless a direct helper write requires a bounded spot-check.
- Stop if Ghidra cannot decompile all siblings; this did not occur.

## 1. Overview

Each `UpdateRamp_*` call walks exactly one cell from the supplied coordinate by `g_DirectionOffsets[direction & 7]`, then mutates that target cell according to target flags, damage-step byte, and tile-class bucket. Damage variants perform one-step damage and tile-class progression. Collapse variants can recurse one perpendicular step, clear anchor overlay state, call direction setters, mark radar dirty, blow up a 3-cell footprint, and write final bridgehead tile classes.

`0x00572B80` is not an alternate name for one of the four NS high helpers. It is the missing high EW DamageA sibling, and the complete high family is eight helpers.

## 2. Key Offsets

| Offset | Type | Purpose | Evidence |
|---|---:|---|---|
| `Cell+0x24` | coord | Cell map coordinate used for dirties/caller args | all helper decompiles |
| `Cell+0x2C` | pointer | Back-pointer written by `SetBridgeDirection_*`, not directly by `UpdateRamp_*` | `0x47E040`, `0x47E470` spot-check |
| `Cell+0x38` | int | `IsoTileTypeIndex`; read by helpers, written by `SetOverlayAndPropagate` | `0x56E990`, `0x56EB80`, all helpers |
| `Cell+0x44` | int | Overlay field cleared to `0xFFFFFFFF` on helper collapse-final | collapse helper bodies |
| `Cell+0x11A` | byte | Tile sub-index/height byte; collapse footprint branch uses bit 0 for NS-family and `<5` for EW-family | collapse helper bodies |
| `Cell+0x11B` | byte | Level byte; final tile write passes `(char)+0x11B - 4` | collapse helper bodies |
| `Cell+0x11E` | byte | Damage-step byte | all helper bodies |
| `Cell+0x140` | flags | bit `0x80` gates damage-step writes | all helper bodies |

## 3. Address / Name Resolution

| Family | DamageA | DamageB | CollapseA | CollapseB |
|---|---|---|---|---|
| Low NS-label | `0x56ED40` | `0x56EE40` | `0x56EF50` | `0x56F2F0` |
| Low EW-label | `0x56F690` | `0x56F7A0` | `0x56F8B0` | `0x56FC80` |
| High NS-label | `0x572230` | `0x572330` | `0x572440` | `0x5727E0` |
| High EW-label | `0x572B80` | `0x572C90` | `0x572DA0` | `0x573170` |

The user's four high addresses are the high NS-label set. Existing docs that mention high `0x00572B80` for EW DamageA are correct: that address is the EW-label high DamageA helper, not a mismatch.

## 4. Common Helper Prologue

All 16 helpers:

1. Mask the caller direction with `& 7`.
2. Compute `target = input_coord + g_DirectionOffsets[direction & 7]`.
3. Compute flat index as `target_y * 0x200 + target_x`.
4. If the index is outside `0..=0x3FFFF` or the cell pointer is null, write `DAT_00ABDC74 = target_coord` and use scratch cell `DAT_00ABDC50`.
5. Continue executing against the target/scratch cell. There is no early return for off-map/null target.

This scratch fallback matters: an invalid perpendicular target can still consume the same write path, but writes hit the scratch cell instead of the map.

## 5. Damage-Step Writes

All damage-step writes are gated by `(Cell+0x140 & 0x80) != 0`. If the gate is false, the helper still executes the tile-class branch.

| Helper group | DamageA state write | DamageB state write | CollapseA state write | CollapseB state write |
|---|---|---|---|---|
| NS-label Low/High | `<4 -> 4`; `5 -> 6` | `<4 -> 5`; `4 -> 6` | `<7 -> 7`; `8 -> recursive collapse-final` | `<7 -> 8`; `7 -> recursive collapse-final` |
| EW-label Low/High | if `state > 8`: `<0xD -> 0xE`; `0xD -> 0xF` | if `state > 8`: `<0xD -> 0xD`; `0xE -> 0xF` | if `state > 8`: `<0x10 -> 0x11`; `0x10 -> recursive collapse-final` | if `state > 8`: `<0x10 -> 0x10`; `0x11 -> recursive collapse-final` |

Collapse-final state branch side effects:

- Recursively call the same collapse helper on the current target coordinate with the original direction.
- Call `SetBridgeDirection_NWSE(0,0)` for low NS-label or `SetBridgeDirection_NESW(0,0)` for high NS-label.
- Call `SetBridgeDirection_NWSE(6,0)` for low EW-label or `SetBridgeDirection_NESW(6,0)` for high EW-label.
- Write `Cell+0x11E = 0`.
- Write `Cell+0x44 = 0xFFFFFFFF`.
- Call `RadarClass__MarkTerrainDirty(Cell+0x24)`.

`SetBridgeDirection_NESW @ 0x47E040` and `SetBridgeDirection_NWSE @ 0x47E470` are byte-identical per existing plate comments and spot decompile; the high/low helpers still call different symbol addresses. With `param_3=0`, the setter path clears bridge direction state, calls `BlowUpBridge`, writes flags/`+0x11E` across the direction chain, and dirties terrain.

## 6. Tile-Class / Overlay Writes

All helpers compute `relative = (Cell+0x38 - tile_base) + 1`.

- Low helpers use tile base `DAT_00ABAD1C`.
- High helpers use tile base `DAT_00AA0E28`.
- Matching pavement buckets call `MapClass__ToggleBridgePavement(target, 1, 0)`.
- Bridgehead class buckets call `MapClass__SetOverlayAndPropagate(target, new_tile, 0xFFFFFFFF, level_arg, 0)`.

Tile-class progression:

| Family | DamageA tile branch | DamageB tile branch | CollapseA tile branch | CollapseB tile branch |
|---|---|---|---|---|
| NS-label | `ABAD30 -> ABAD30`; `ABAD30+2 -> ABAD30+2` | `ABAD30 -> ABAD30+1`; `ABAD30+1 -> ABAD30+2` | `ABAD30/ABAD30+2 -> ABAD30+2`; `ABAD30+3 -> recursive + footprint + ABAD30+3` | `ABAD30/ABAD30+1 -> ABAD30+2`; `ABAD30+3 -> recursive + footprint + ABAD30+3` |
| EW-label | `AA1028 -> AA1028`; `AA1028+2 -> AA1028+2` | `AA1028 -> AA1028+1`; `AA1028+1 -> AA1028+2` | `AA1028/AA1028+2 -> AA1028+2`; `AA1028+3 -> recursive + footprint + AA1028+3` | `AA1028/AA1028+1 -> AA1028+2`; `AA1028+3 -> recursive + footprint + AA1028+3` |

`SetOverlayAndPropagate @ 0x56EB80` writes `Cell+0x38 = new_tile`, calls `CellClass__RecalcAttributes`, marks radar dirty, and recursively propagates to 8 neighbors whose old `+0x38` matches the old tile. `ToggleBridgePavement @ 0x56E990` writes bit `0x2000` in `Cell+0x140`, marks radar dirty, and recursively propagates to 8 neighbors with matching `+0x38`.

## 7. Collapse Footprints Inside Helpers

The collapse helper's tile-class `+3` branch performs a recursive same-helper call first, then emits three `CellClass__BlowUpBridge` calls before the final `SetOverlayAndPropagate`.

NS-label footprint:

- If `(Cell+0x11A & 1) == 0`: `(x,y)`, `(x,y-1)`, `(x,y+1)` in that order.
- Else: `(x-1,y)`, `(x-1,y-1)`, `(x-1,y+1)` in that order.
- The final level argument uses `Cell(x,y).+0x11B - 4` in the first case, or `Cell(x-1,y).+0x11B - 4` in the second case.

EW-label footprint:

- Let `x0 = x - 1`.
- If `(byte)Cell+0x11A < 5`: `(x-1,y)`, `(x,y)`, `(x+1,y)` in that order; final level from `Cell(x,y).+0x11B - 4`.
- Else: `(x-1,y-1)`, `(x,y-1)`, `(x+1,y-1)` in that order; final level from `Cell(x,y-1).+0x11B - 4`.

Out-of-range/null cells in these explicit footprint lookups use the same scratch-cell fallback pattern except for a few `MapClass__Get_CellClass` calls that hide the fallback inside that helper.

## 8. Low Mirrors High?

**Helper bodies:** yes, with two concrete substitutions.

1. Low uses `DAT_00ABAD1C` as the tile base; high uses `DAT_00AA0E28`.
2. Low collapse-final calls `SetBridgeDirection_NWSE`; high collapse-final calls `SetBridgeDirection_NESW`.

The state-byte lattice, one-cell perpendicular walk, scratch fallback, tile-class branch structure, recursive collapse, three-cell `BlowUpBridge` footprints, `+0x44` clear, and terrain dirty calls match corresponding high helpers.

**Live low dispatcher usage:** not a pure mirror. `ProcessBridgeDamageStateMachine_Low @ 0x571490` calls low NS DamageA/B, low NS CollapseA/B, low EW CollapseA/B, and low EW DamageA/B in the structural/`+0x100` state-byte path. But in the low non-structural EW progressive bridgehead branch, xrefs show calls to high EW DamageA/B at `0x571AB0 -> 0x572B80` and `0x571AC5 -> 0x572C90`. That means a future implementation must not infer called helper height solely from the low/high owning dispatcher.

## 9. Integration Points

| Caller | Calls | Evidence |
|---|---|---|
| `ProcessBridgeDamageStateMachine_High @ 0x576BA0` | all eight high helpers | callee list and decompile |
| `ProcessBridgeDamageStateMachine_Low @ 0x571490` | all eight low helpers plus high EW DamageA/B in one non-structural branch | callee list and xrefs |
| `0x571AB0`, `0x571AC5` | low dispatcher direct branch calls high EW DamageA/B | xrefs to `0x572B80`, `0x572C90` |
| `0x57210C`, `0x572126` | low dispatcher structural path calls low EW DamageA/B | xrefs to `0x56F690`, `0x56F7A0` |

## 10. Current Rust Implementation Status

Relevant Rust surfaces:

- `src/sim/bridge_specs.rs::update_ramp_perpendicular`
- `src/sim/bridge_specs.rs::apply_anchor_class_transition`
- `src/sim/bridge_state/mod.rs` damage-state caller paths
- `src/sim/bridge_state/walker.rs` low/high classifier and collapse surfaces
- `src/sim/world/bridge_orchestrator.rs` side-effect orchestration

Rust already models the one-cell perpendicular direction selection and some state/tile-class progression. Current risks from the binary slice:

- `update_ramp_perpendicular` treats `is_high_bridge` as unused, but binary low vs high helper bodies differ in tile base and direction-setter address; even if state transitions match, tile-class routing must not be height-agnostic.
- Rust role gating via `BridgeCellRole::Anchor` / `Bridgehead` is narrower than the binary's raw `Cell+0x140 & 0x80` state gate plus independent tile-class branch.
- Rust must account for the low dispatcher's non-structural EW progressive branch calling high EW DamageA/B helpers.
- Rust should preserve scratch/no-early-return behavior at boundaries if those paths can be reached by map data; otherwise document a bounded-map invariant.

## 11. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| High NS DamageA/B | verified | `0x572230`, `0x572330` | none |
| High NS CollapseA/B | verified | `0x572440`, `0x5727E0` | no full setter expansion |
| High EW DamageA/B | verified | `0x572B80`, `0x572C90` | none |
| High EW CollapseA/B | verified | `0x572DA0`, `0x573170` | no full setter expansion |
| Low NS DamageA/B | verified | `0x56ED40`, `0x56EE40` | none |
| Low NS CollapseA/B | verified | `0x56EF50`, `0x56F2F0` | no full setter expansion |
| Low EW DamageA/B | verified | `0x56F690`, `0x56F7A0` | none |
| Low EW CollapseA/B | verified | `0x56F8B0`, `0x56FC80` | no full setter expansion |
| High/low helper mirroring | verified | body comparisons above | runtime numeric tile constants not recovered |
| Low dispatcher high-helper wrinkle | verified | xrefs `0x571AB0`, `0x571AC5` | reason for original compiler/source choice not needed |
| Full rim refresh | deferred | out-of-scope | BR-10 slot |

## 12. Open Questions - Final State

- `[RESOLVED] OQ1 - Are all high siblings known? -> Yes, high is 8 helpers; `0x572B80` is EW DamageA High.` (evidence: `0x572230..0x573170`)
- `[RESOLVED] OQ2 - Does each helper walk exactly one perpendicular cell before mutating? -> Yes, `target = input + g_DirectionOffsets[direction & 7]`.` (evidence: all 16 helper prologues)
- `[RESOLVED] OQ3 - Is off-map/null an early return? -> No, it writes `DAT_00ABDC74` and uses scratch `DAT_00ABDC50`.` (evidence: all 16 helper prologues)
- `[RESOLVED] OQ4 - What gates damage-step writes? -> Only `Cell+0x140 & 0x80`; tile-class branches still run without it.` (evidence: all 16 helper bodies)
- `[RESOLVED] OQ5 - Does low mirror high? -> Helper bodies mirror with tile-base and direction-setter substitutions; low dispatcher usage does not always mirror.` (evidence: helper decompiles, `0x571490` xrefs)
- `[RESOLVED] OQ6 - Which cells are blown up by helper collapse? -> Three cells in variant-specific order as listed in section 7.` (evidence: collapse helper bodies)
- `[RESOLVED] OQ7 - Which fields are directly written by helpers? -> `+0x11E`, `+0x44`, `DAT_00ABDC74`, plus helper-called `+0x38` / `+0x140` / direction-chain writes.` (evidence: helper bodies, `0x56EB80`, `0x56E990`, `0x47E040`, `0x47E470`)
- `[RESOLVED] OQ8 - Is this active in YR? -> Yes, called from active high/low bridge damage state machines.` (evidence: `0x576BA0`, `0x571490` callees)
- `[DEFERRED] OQ9 - What are the concrete numeric values of BSS tile constants?` (category: `needs-runtime-debugger`; reason: this slice verified control flow and globals, not runtime-init values; next-step-if-pursued: use the BSS constant sweep slot)

## 13. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `UpdateRamp_*` always walks one `direction&7` target, uses scratch fallback, and applies state writes only under `+0x140 & 0x80` while tile-class writes remain independent. | all 16 helper decompiles | partial/mismatch risk | `src/sim/bridge_specs.rs::update_ramp_perpendicular` | Separate raw target-cell flag gate from bridge-role abstraction and preserve no-early-return semantics or document invariant. | `bridge_update_ramp_perpendicular_flag_gate_tile_branch_runs_without_anchor_flag` | Do not gate all side effects on `BridgeCellRole::Anchor`. |
| Low helper bodies mirror high helper bodies except tile base and direction-setter address; live low dispatcher calls high EW DamageA/B in one non-structural EW progressive branch. | `0x571490`, xrefs `0x571AB0`, `0x571AC5`, helper bodies | mismatch risk | `src/sim/bridge_state/mod.rs`, `src/sim/bridge_specs.rs` | Route by exact dispatcher branch/helper target, not by owning low/high state machine alone. | `low_ew_bridgehead_progressive_damage_uses_high_ew_damage_helpers` | Do not treat `_is_high_bridge` as globally irrelevant for tile-class behavior. |
| Collapse helper `+3` branch recursively calls itself, blows up three ordered cells, writes final tile with level `+0x11B - 4`, and collapse-final state branch clears `+0x11E` and `+0x44` then dirties terrain. | `0x56EF50`, `0x56F2F0`, `0x56F8B0`, `0x56FC80`, `0x572440`, `0x5727E0`, `0x572DA0`, `0x573170` | partial/unchecked | `src/sim/bridge_specs.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/world/bridge_orchestrator.rs` | Preserve recursive ordering, three-cell order, final level source, overlay clear, and dirty marking. | `update_ramp_collapse_about_to_fall_emits_ordered_three_cell_footprint_and_clears_overlay` | Do not collapse only the target anchor or reorder recursive call after `BlowUpBridge`. |

## Negative Facts / Do Not Do

- Do not call the high family only four helpers; high EW siblings at `0x572B80..0x573170` are live.
- Do not say low "fully mirrors" high without the dispatcher caveat: low EW progressive bridgehead damage calls high EW DamageA/B.
- Do not gate tile-class writes on `Cell+0x140 & 0x80`; only `+0x11E` writes use that gate.
- Do not early-return on invalid target coordinates if exact scratch fallback behavior matters.
- Do not model CollapseA/B `+3` as a simple final-state write; it recurses and then emits an ordered three-cell `BlowUpBridge` footprint.

## Stale Docs / Follow-up Docs

- Replace "other `UpdateRamp_*` siblings are structurally equivalent per xrefs and prior state-machine report, but not all are restated here" with: "All 16 `UpdateRamp_*` siblings were decompiled in `BRIDGE_RAMP_PERPENDICULAR_HIGH_LOW_FAMILIES_GHIDRA_REPORT.md`; helper bodies are structurally equivalent within the exact state/tile matrices listed there, but low dispatcher live usage includes a non-mirroring EW progressive branch that calls high EW DamageA/B."
- Replace any wording that implies `0x00572B80` is an address/name mismatch with: "`0x00572B80` is `MapClass__UpdateRamp_EW_DamageA_High`, the first high EW sibling; the originally listed `0x00572230..0x005727E0` set covers only high NS-label helpers."

## Remaining Uncertainty

- Numeric values of runtime-initialized BSS tile bucket constants are not recovered here. The helper behavior is verified by global identity and branch use, but table-driven Rust or fixture generation still needs those values from the BSS constant sweep/debugger capture.
- Full `SetBridgeDirection_*` and rim-refresh field write matrices are intentionally not claimed by this report.

## Sources

- Ghidra decompile: `0x56ED40`, `0x56EE40`, `0x56EF50`, `0x56F2F0`, `0x56F690`, `0x56F7A0`, `0x56F8B0`, `0x56FC80`
- Ghidra decompile: `0x572230`, `0x572330`, `0x572440`, `0x5727E0`, `0x572B80`, `0x572C90`, `0x572DA0`, `0x573170`
- Ghidra decompile/callees: `ProcessBridgeDamageStateMachine_Low @ 0x571490`, `ProcessBridgeDamageStateMachine_High @ 0x576BA0`
- Ghidra spot-check: `SetBridgeDirection_NESW @ 0x47E040`, `SetBridgeDirection_NWSE @ 0x47E470`, `ToggleBridgePavement @ 0x56E990`, `SetOverlayAndPropagate @ 0x56EB80`
- Existing docs checked: `docs/research/MAPCLASS_COMPLETE_DECODE.md`, `docs/research/MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`, `docs/research/bridges/06-render-presentation-audio/BRIDGE_RENDERING_REMAINING_CASES_GHIDRA_REPORT.md`, `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGEHEAD_DIRECT_DAMAGE_SLOT3_COLLAPSE_GHIDRA_REPORT.md`

## Status

COMPLETE for the scoped ramp-perpendicular helper-family behavior. Deferred only for BSS numeric values and full rim/direction-setter algorithms, which are explicitly outside this slice.
