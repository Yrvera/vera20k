# UnitClass::Can_Enter_Cell Bridge/Tunnel Path -- Ghidra Research Report

**Primary address:** `0x0073F0A0` (`UnitClass::Can_Enter_Cell`, vtable `+0x1AC`)  
**Bridge sub-check:** `0x004D9C60` (`CheckBridgeTraversal`, vtable `+0x1B0`)  
**Low bridge/tube helpers:** `0x00484AB0`, `0x00484F20`, `0x007359F0`  
**Confidence:** High for the decompiled paths listed under Sources. Medium where noted for call sites summarized from prior full reports rather than re-decompiled in this pass.  
**Active in YR:** Yes for unit A* entry, drive movement probes, high bridge entry/exit, low-bridge tube cells, wall overlays, occupancy lists, and A* cost mapping. Fog/shroud rejection is conditional/legacy and normally not a standard YR skirmish pathing gate.

## Summary verdict

`UnitClass::Can_Enter_Cell @ 0x0073F0A0` is the live YR vehicle/unit cell-entry legality function used by pathfinding through vtable slot `+0x1AC`. The bridge-specific virtual at `+0x1B0` is not the A* entry; it is `CheckBridgeTraversal @ 0x004D9C60`, a hard 0/7 height and bridgehead validator that can update the caller's `path_height` and a bridge-list flag.

High bridge entry is a two-pass decision: `Can_Enter_Cell` first selects a bridge-vs-ground object-list byte from the candidate cell and incoming `path_height`, then `CheckBridgeTraversal` may update `path_height`, then `Can_Enter_Cell` may re-read bridge occupancy bits from `cell+0x128`. The object list and the occupancy bitfield can therefore legitimately come from different layers on bridgehead boundary ticks.

Low bridges are not just overlay IDs. The live binary low-bridge predicate is `CellClass::IsLowBridgeCell`: valid `cell+0x116` tube index plus `cell+0xEC == 10`. `CellClass::GetTubeAtCell` ignores land type and only returns the indexed `TubeClass*`; `UnitClass::Can_Enter_Cell` then uses tube existence, direction `8`, tube endpoint fields, and tube direction `+0x2C` for legality.

Current Rust has a substantial bridge-aware A* model, separate ground/bridge occupancy layers, and a `check_bridge_traversal` analogue. The remaining parity gaps are low-bridge/tube construction and entry, the exact two-pass object-list vs occupancy-bit split at runtime/A*, complete overlay/wall/building special cases, and stale/misleading return-code naming in `CellEntryResult`.

## Verified entry point and caller chain

### Binary facts

| Item | Verified fact | Evidence | Confidence | Active in YR |
|---|---|---|---|---|
| Unit A* entry | `UnitClass::Can_Enter_Cell` starts at `0x0073F0A0`; force-decompile succeeds under that label. | Ghidra `force_decompile 0073f0a0`; prior vtable report read UnitClass vtable slot `0x7F5E1C = 0x0073F0A0`. | High | Yes |
| Vtable slot | `+0x1AC` is the A* cell-entry slot. | `AStar_main_loop` assembly at `0x00429F54`: `CALL dword ptr [EDX + 0x1ac]`. | High | Yes |
| Bridge sub-check | Unit/Infantry/Foot `+0x1B0` resolves to `CheckBridgeTraversal @ 0x004D9C60`, not a parent `Can_Enter_Cell`. | Ghidra `force_decompile 004d9c60`; prior vtable memory report. | High | Yes |
| A* cost consumer | `AStar_compute_edge_cost @ 0x00429830` indexes `g_AStar_EdgeCost_BaseTable @ 0x0081870C` by the return code. | Ghidra `force_decompile 00429830`. | High | Yes |
| Movement consumer | `DriveLocomotionClass::Process_Movement @ 0x004B2630` probes the next path cell through vtable `+0x1AC` and dispatches on codes 0-7. | Prior full decompilation in `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`; not re-decompiled in full this pass. | Medium-high | Yes |

### Call shape

The A* call site near `0x00429F54` pushes the candidate cell, neighbor direction, current-node/path height, explicit current-node cell, and a flags byte, then calls `[EDX+0x1AC]`. `UnitClass::Can_Enter_Cell` forwards its explicit parent/current cell argument to `[vtable+0x1B0]`, so normal A* bridge traversal uses the directed parent edge, not an inferred current cell.

`CheckBridgeTraversal` has a fallback for callers that pass no parent cell: if `param_5 == 0`, it computes the predecessor as `candidate + DirectionOffset[(direction - 4) & 7]`. Direction `-1` is a separate height-seeding mode and skips directed diff/slope checks.

## Return-code table

The eight return codes are consumed both by pathfinding costs and by runtime movement dispatch.

| Code | Binary meaning in `UnitClass::Can_Enter_Cell` | A* base cost | Runtime effect | Hard/soft |
|---:|---|---:|---|---|
| 0 | Clear/allowed. Also returned for target building entry/capture special cases and valid tube entry. | 1.0 | Move/continue. | Pass |
| 1 | Special neutral/civilian/crushable obstruction state from object classification. | 1000.0 | Runtime marks/redraws and retries; A* strongly avoids. | Soft but very expensive |
| 2 | Moving friendly / temporary block, or crush/friendly infantry occupancy bit fallback. | 1.0 base, then dynamic | Runtime waits/repaths with `BlockedDelay`; A* runs blocker-path prediction unless urgency 2. | Soft |
| 3 | Scatter/garrison-related allied building obstruction. It is not produced by `CheckBridgeTraversal`. | 1.0 | Runtime crush/scatter path. | Soft |
| 4 | Friendly wall overlay / allied obstruction class. | 60.0 | Try scatter/crush or wait. | Soft-expensive |
| 5 | Enemy unit/building/wall/attackable obstruction. | 20.0 | Attack/wait/repath behavior. | Soft-moderate |
| 6 | Stationary allied non-building object. | 8.0 | Close-enough stop vs scatter handling. | Soft |
| 7 | Terrain/height/tube/shroud/hard object impassable. | 10000.0 | Stop/clear path/repath. | Hard blocker |

Important correction: older cost docs label code 3 as "BridgeRamp" and code 6 as "Cliff" because of pathfinding cost interpretation. In the verified `UnitClass::Can_Enter_Cell` body, bridge traversal itself returns only 0 or 7, and code 3 is assigned in building/allied-scatter handling.

For code 2, `AStar_compute_edge_cost @ 0x00429830` does more than read the base table:

- Normal mode (`Pathfinder+0x3C == 0`) walks the selected object list (`cell+0xE4` or `cell+0xE8`) and predicts the moving blocker's path for up to 10 cells.
- If the blocker chain clears, cost returns to baseline.
- If the chain remains jammed, cost becomes 4.0.
- Urgency 2 sets cost to 1000.0.
- The chosen object list is layer-sensitive, so bridge-vs-ground occupancy can change path choices even when coordinates are identical.

## High bridge decision path

### Pre-vtable layer snapshot in `UnitClass::Can_Enter_Cell`

At function start, `Can_Enter_Cell` computes a local bridge-list byte:

```text
 if candidate.Flags & 0x100 == 0 -> ground list
 else if path_height != -1 and abs(path_height - candidate.Level) < 2 -> ground list
 else -> bridge list
```

Details:

- `cell+0x140 & 0x100` is the bridge/deck structural flag used here.
- `cell+0x11B` is read as signed `Level`.
- The threshold is `< 2`, so height differences 0 or 1 use ground; 2+ select bridge at this early stage.
- The first occupancy snapshot always reads `cell+0x124` ground occupation bits into the local packed state.

### `CheckBridgeTraversal @ 0x004D9C60`

The bridge sub-check returns only `0` or `7`. It validates legal height-diff shapes:

| Shape | Legal condition | Side effect |
|---|---|---|
| Direction `-1` | If `path_height == -1` and candidate has `0x100`, set `path_height = candidate.Level + 4`; return 0. | Seeds deck height. |
| Parent bridge with unset height | If parent/predecessor has `0x100` and `path_height == -1`, set `path_height = parent.Level + 4`; candidate must have `0x200`, else return 7. | Prevents mid-span entry from a bridge parent unless candidate is bridgehead. |
| `abs(diff) == 0` | Allowed only if path height is unset/matches candidate level, or `{candidate bridge, candidate bridgehead, parent bridge}` are all true. | None. |
| `abs(diff) == 1` | Requires a nonzero `SlopeIndex` on the side determined by movement direction. | None. |
| `abs(diff) == 4`, high-to-low | Requires `path_height == candidate.Level` and parent/predecessor has bridge flag. | None. |
| `abs(diff) == 4`, low-to-high | Candidate must be bridge and bridgehead. | Writes `*bridge_entered = 1`, returns 0. |
| Anything else | Return 7. | Hard block. |

Tiny details verified from the decompile:

- The predecessor fallback rotates direction with `(direction - 4) & 7`.
- `Level` math is signed-byte math.
- `diff_abs == 2`, `3`, `5+` are hard-blocked, even though the caller's pre-layer decision uses `>= 2` as "bridge layer".
- The ascending diff-4 case is the only path that sets the bridge-entered/list flag.
- The descending diff-4 case does not set the bridge-entered/list flag.

### Post-vtable occupancy re-snapshot

After `CheckBridgeTraversal`, `Can_Enter_Cell` checks:

```text
if path_height != -1
   and candidate.Flags & 0x100 != 0
   and path_height == candidate.Level + 4:
    read cell+0x58 and cell+0x128 bridge occupancy bits
```

This re-read does not necessarily change the object list selected earlier. The object loop later picks `cell+0xE4` if the local bridge-list byte is zero, otherwise `cell+0xE8`. Therefore under-bridge and deck units are separated by list, but the bitfield fallback can be bridge-layer while the list selection remains ground-layer or vice versa at transition edges.

### Destroyed bridge deck

`Can_Enter_Cell` itself does not inspect the bridge damage state machine. It reacts to cell flags/walkability already maintained elsewhere. The relevant hard block is via `cell+0x140 & 0x100`, `cell+0x140 & 0x200`, `path_height`, and the later locomotor/passability checks. Destroyed high-bridge decks therefore matter to entry through the map/bridge state updating those flags and zone/path data, not through a direct "damage state" branch inside `UnitClass::Can_Enter_Cell`.

## Low bridge / tunnel / tube decision path

### `CellClass::IsLowBridgeCell @ 0x00484AB0`

Verified decompile:

```text
return (0 <= *(i16 *)(cell+0x116) < g_TubeArray.count)
    && (*(int *)(cell+0xEC) == 10)
```

This predicate does not read overlay ID. Low bridge pathing identity requires a valid tube index and `LandType == 10`.

### `CellClass::GetTubeAtCell @ 0x00484F20`

Verified decompile:

```text
tube_index = *(i16 *)(cell+0x116)
if 0 <= tube_index < g_TubeArray.count:
    return g_TubeArray[tube_index]
return NULL
```

This helper does not check `LandType`. `Can_Enter_Cell` calls it early and uses the pointer for both direction `8` tube entry and normal-direction tube exclusion.

### Tube checks in `UnitClass::Can_Enter_Cell`

The tube path runs before bridge traversal and before overlay/object-list scanning:

1. If `self.Type+0xDFC != -1`, the unit has a tunnel locomotor/tunnel type requirement.
2. If target `LandType == 10`, the code reads the isometric tile type at `cell+0x38` and checks `tile+0x2E4`/`tile+0x2E8` ramp orientation:
   - `(RampType == 5 or 4) && RampDir == 3` requires `cell+0x11A == 2`.
   - `RampType == 3 && (RampDir == 4 or 5)` requires `cell+0x11A == 6`.
   - Failure returns 7.
3. If `LandType` is neither the unit's tunnel type (`Type+0xDFC`) nor 10, the code may allow overlay indices `0xED..0xEE` unless the height/path value equals `cell.Level`; otherwise it returns 7. The exact overlay exception is verified in the decompile but its content semantics remain low-confidence without following the overlay definitions further.
4. If direction is exactly `8`, the function requires a non-null tube pointer and a nonzero endpoint at `tube+0x28`; otherwise return 7. If valid, return 0 immediately.
5. For normal directions, if a tube exists on the candidate cell, compute `abs(direction - tube.Direction_0x2C)`. If the result is `3`, `4`, or `5` and direction is not `-1`, return 7.
6. The function repeats the same exclusion for the adjacent/reverse direction using `(direction - 4) & 7`, `Pathfinding_update_continued`, and another `GetTubeAtCell`.

### `UnitClass::TubeMovement @ 0x007359F0`

`TubeMovement` is active movement execution after pathing has selected tube travel. Verified details:

- Unit field `+0x684`/`param_1[0x1A1]` is used as a signed/byte tube index into `g_TubeArray`.
- Unit byte `+0x685` is the current tube path-slot index.
- The tube stores path directions at `tube+0x30 + slot*4`, with `-1` as the end sentinel.
- For each path direction, only `dir & 7` is used for direction deltas.
- Tube start/end coordinates are read from `tube+0x24` and `tube+0x28`.
- On completion, the unit is placed at `tube+0x28`, tube index is reset to `0xFF`, facing is updated from `tube+0x2C`, and normal movement state resumes.
- If the exit cell's ground object list (`cell+0xE4`) is occupied, the function collects objects from that ground list and may shove/clear infantry/aircraft occupants before finishing.

### Low bridge construction

`CellClass::RecalcAttributes @ 0x0047D2B0` constructs tube records when:

- `cell.LandType == 10`;
- `cell+0x116` is invalid/out of range;
- `IsoTileTypeIndex` falls in one of four exact 4-tile low-bridge ranges;
- allocation of `0x1C4` bytes succeeds.

It calls `TubeClass::Constructor(cell_coord, DAT_0081CC20[offset])`. Existing low-bridge INI overlays in `rulesmd.ini` use `Land=Road` and usually `NoUseTileLandType=true`, so the display/INI overlay family is not by itself the live `IsLowBridgeCell` condition.

## Overlay/wall decision path

After bridge/tube/shroud/locomotor checks, `Can_Enter_Cell` reads `cell+0x44` overlay type index. If it is not `-1`, it resolves `g_OverlayTypeClass_Array[index]`.

Verified overlay fields:

| Offset | Meaning in this path | Effect |
|---:|---|---|
| `OverlayType+0x2AA` | special/crate-like overlay flag | If set, and the house/player-control check fails in single-player, return 7. |
| `OverlayType+0x2A8` | `Wall=` | Enters wall branch. |
| `OverlayType+0x22D` | crushable/passable wall-related flag | Lets the wall be bypassed/crushed before weapon checks. |
| `OverlayType+0x9C` | armor type | Used with warhead `Wood` capability for wood walls. |

Wall branch details:

- If the overlay is a wall and neither the overlay's passable/crushable flag nor the unit's crusher ability/weapon ability permits crushing, the unit must be able to fire (`vtable+0x2AC`) and must have a weapon whose warhead can damage walls (`Warhead+0x144`) or wood (`Warhead+0x147` with overlay armor type 6). Otherwise return 7.
- Enemy/non-allied walls upgrade the result to at least code 5.
- Allied/friendly walls upgrade the result to at least code 4.
- MovementZone value `0xC` has a special bypass in the wall branch; this is a type/zone prerequisite, not a bridge-specific rule.

Bridge overlays are not directly tested as "bridge overlay IDs" inside this branch. High bridge behavior arrives through cell flags and bridge records; low bridge behavior arrives through land type/tube records.

Gates are handled primarily as building objects, not overlay-only walls. In the object loop, `BuildingType+0x16BF` (`IsGate`) is passable when building mission/state at object `+0x618` is `0xC` or `8`; otherwise normal blocking logic applies.

## Occupancy/object-list decision path

### Cell fields

| Cell offset | Role |
|---:|---|
| `+0x54` | Ground-layer owner/aux value snapshotted before bridge check. |
| `+0x58` | Bridge-layer owner/aux value read when `path_height == Level+4`. |
| `+0xE4` | Ground object linked-list head. |
| `+0xE8` | Bridge object linked-list head. |
| `+0x124` | Ground occupancy bitfield. Low byte covers infantry subcells; bit 5 is vehicle/major occupant. |
| `+0x128` | Bridge occupancy bitfield with the same packed shape. |

The object loop traverses `Object+0x30` (`piVar15[0xC]`) until null. List order is gameplay-relevant: the function can return early for special mission/building cases, enemy unattackable cases, moving-ally deadlock, cargo/unload, and other blockers.

### Layer separation

Under-bridge units and deck units do not directly share object lists:

- Ground list: `cell+0xE4`.
- Bridge list: `cell+0xE8`.

However, the function has two independent layer concepts:

- object-list layer, chosen by the early bridge-list byte and possibly forced by ascending bridge entry;
- occupancy-bit layer, reselected after bridge traversal if `path_height == candidate.Level + 4`.

That split matters when a bridgehead/ramp transition updates `path_height` after the early list selection. Rust implementations that use one layer for both will diverge on edge ticks where deck-vs-ground occupants overlap at the same map cell.

### Object blocker outcomes

Verified outcomes include:

- `self == object`: clears the self vehicle bit and skips, so a unit does not block itself.
- Same transport destination: return 0.
- `IsTrain` on both mover and infantry blocker: return 0.
- Enter/capture mission with target building and matching `CanUnitEnter`/`Capturable`: return 0.
- Open gate: skip blocker.
- Active laser fence: return 7.
- Allied stationary building: return 7.
- Allied stationary non-building: upgrade to code 6.
- Allied moving blocker: may return 7 if same-facing and closer than `0x200` leptons, otherwise upgrade to code 2 if the blocker locomotor reports moving.
- Enemy attackable blocker: upgrade to code 5.
- Enemy unattackable/invisible building or unsuitable anim/terrain target: return 7.
- Crushable enemy/infantry fallback can turn into code 2 or 0 depending on occupancy bits and crush checks.

## Movement-zone / locomotor prerequisites

The cell-entry path uses several independent prerequisites:

- `TechnoType+0xDFC != -1` gates the tunnel land-type/ramp-orientation logic.
- `TechnoType+0x5B4` participates in wall special handling; prior verified docs identify this as `MovementZone`.
- `TechnoType+0x67C` is `SpeedType`; the post-object-loop terrain table uses `g_SpeedType_LandType_Table[cell.LandType * 9 + SpeedType]` and returns 7 when the multiplier is exactly 0.0. This post-loop speed table is skipped for the bridge-list path (`bridge-list byte != 0`).
- `TechnoType+0xD28` (`Crusher`) plus `TechnoClass::HasWeaponAbility(0x11)` affects wall crushing, enemy/crushable blockers, and occupancy-bit fallback.
- `TechnoType+0xC94` (jumpjet-like override in this path) allows some infantry/weapon fallback cases to avoid hard 7.
- Object `+0x674` locomotor COM pointer is used by `FootClass::LocomotorPassabilityCheck @ 0x004D9C10`, but only when its stack flag is nonzero; otherwise the helper returns 0.
- Shroud/fog rejection requires map editor mode off, cell not on screen/revealed to the relevant check, vtable `+0x320` false, and unit byte `+0x3D5 != 0`. This path exists in `gamemd.exe` but is conditional/legacy for standard YR.

## Current Rust comparison

### Matches or partially matches

- `src/sim/pathfinding/core.rs:242` implements a `check_bridge_traversal` analogue with the same legal diff classes 0/1/4, `(direction-4)&7` predecessor fallback, signed-level logic, and ascending `force_bridge_list` behavior.
- `src/sim/pathfinding/core.rs:330` implements split `CanEnterLayerContext` with independent object-list and occupancy-bit layers.
- `src/sim/movement/movement_occupancy.rs:38` resolves runtime can-enter layers and can produce bridge-list/ground-bit and bridge-list/bridge-bit splits, matching the known two-pass shape for some bridgehead cases.
- `src/sim/pathfinding/core.rs:51` through `:58` and `:1518` onward model the code-2 cost prediction constants and 10-hop chain.
- `src/sim/bridge_state/mod.rs:468` introduces `BridgeRecordKind::{High, Low}` and documents the verified high-vs-low record split.

### Diverges or remains incomplete

- `src/sim/pathfinding/cell_entry.rs:44` names code 3 as `BridgeRamp` and code 6 as `Cliff`; verified `UnitClass::Can_Enter_Cell` semantics are code 3 allied scatter/building case and code 6 stationary allied non-building. Even if internal dispatch works, the public model is stale and can mislead future cost/runtime mapping.
- `src/map/resolved_terrain.rs:303` and `:1095` still classify low bridges primarily from overlay effects, override them to Road, and set a bridge layer. The binary low-bridge predicate requires `LandType==10` plus a valid `cell+0x116` tube index. This is the largest confirmed low-bridge parity gap.
- `src/sim/movement/tunnel_movement.rs:1` implements subterranean burrow movement, not the low-bridge `TubeClass` cell-entry path with direction sentinel 8, tube endpoint `+0x28`, tube directions `+0x2C`, and `cell+0x116` indexing.
- Rust path grids do not appear to carry a `TubeClass`-equivalent per-cell tube index/path-slot model for low bridges. That means `IsLowBridgeCell`, `GetTubeAtCell`, and `UnitClass::TubeMovement` cannot yet be represented faithfully.
- Wall/overlay handling in Rust terrain resolution collapses many cases into `overlay_blocks`/`is_wall`. The binary `Can_Enter_Cell` wall path is dynamic: crusher, weapon, warhead wall/wood flags, ownership/alliance, and MovementZone can turn a wall from hard 7 into code 4 or 5.
- Binary object-list order is observable for early returns and first-moving-blocker cost prediction. Rust uses deterministic maps/sets and layer maps, but the report did not verify that insertion order reproduces `Object+0x30` list order for all relevant occupancy updates.

## Confirmed parity gaps

1. **Low bridge/tube foundation:** Rust low bridge passability is overlay/path-grid based; YR pathing is tube-index plus `LandType==10` based.
2. **Direction 8 tube entry:** Rust does not model `Can_Enter_Cell` direction sentinel 8 as "tube entry requires non-null tube and nonzero `tube+0x28`, then return 0."
3. **Tube direction exclusion:** Rust does not model the candidate and reverse-adjacent tube direction rejection where `abs(direction - tube.Direction)` in `3..=5` hard-blocks.
4. **Return-code taxonomy drift:** Rust enum names for codes 3 and 6 do not match verified UnitClass production sites.
5. **Dynamic wall codes:** Rust overlay blocking is coarser than the binary wall branch and may produce hard blocks where YR would return soft code 4/5 or vice versa.
6. **Object list ordering:** Needs explicit verification. YR linked-list order can affect early return and code-2 prediction; Rust may not preserve this at the same granularity.
7. **Full runtime two-pass exactness:** Rust approximates the split layer context, but the report did not prove every A*/runtime caller passes the same parent/current cell and path-height combination as YR.

## Implementation implications

- Treat `+0x1AC` and `+0x1B0` as separate concepts. The implementation should not encode bridge entry as a return code 3.
- Preserve independent object-list and occupancy-bit layer selection. A single "current movement layer" is not enough at bridgeheads.
- Low bridge pathing should be backed by a tube-index/tube-record model, not by overlay ID alone.
- Cell-entry return codes should be represented by numeric YR codes first, with semantic labels that reflect actual UnitClass production sites.
- Wall legality needs a dynamic object/weapon/owner-aware phase if Rust wants path choices to match YR, because wall overlays are not always hard terrain blockers.
- A* soft-block costs must use the selected object-list layer, not just coordinate occupancy.

## Remaining open questions

- The exact meaning of the overlay `0xED..0xEE` exception inside the tunnel-type branch remains unresolved. The branch was verified, but this pass did not trace those overlay definitions and map-use cases to completion.
- `FootClass::LocomotorPassabilityCheck @ 0x004D9C10` decompiled as a thin locomotor COM call gated by a stack byte. This pass did not resolve every concrete locomotor implementation behind vtable `+0x1C`.
- The exact source and lifecycle of unit byte `+0x3D5` for shroud/fog path rejection was not re-investigated; treat the branch as conditional/legacy until its standard YR defaults are independently verified.
- This pass did not fully re-decompile `DriveLocomotionClass::Process_Movement @ 0x004B2630`; runtime dispatch details are cited from the existing high-confidence collision report and should be re-opened only if a movement-tick implementation question depends on a specific sub-branch.
- Rust object insertion/list ordering versus YR `Object+0x30` ordering needs a focused audit.

## Sources

### Ghidra functions rechecked in this pass

- `0x0073F0A0` -- `UnitClass::Can_Enter_Cell`
- `0x004D9C60` -- `CheckBridgeTraversal`
- `0x004D9C10` -- `FootClass::LocomotorPassabilityCheck`
- `0x00484AB0` -- `CellClass::IsLowBridgeCell`
- `0x00484F20` -- `CellClass::GetTubeAtCell`
- `0x007359F0` -- `UnitClass::TubeMovement`
- `0x00429830` -- `AStar_compute_edge_cost`
- `0x0042ACF0` -- `PathfinderClass::UpdateBridgePassability`
- `0x0047D2B0` -- `CellClass::RecalcAttributes`
- `0x00429F54` -- assembly context for A* virtual `CALL [EDX+0x1AC]`

### Existing reports consulted and verified against

- `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`

### INI and Rust files checked

- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` low bridge overlay sections (`LOBRDG*`, `LOBRDGE*`, `LOBRDB*`, `LOBRDGB*`) and general movement keys.
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/core.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/cell_entry.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_occupancy.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_bridge.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/tunnel_movement.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/map/resolved_terrain.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs`
