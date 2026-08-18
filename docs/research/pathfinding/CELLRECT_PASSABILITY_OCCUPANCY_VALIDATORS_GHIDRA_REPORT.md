# CellRect Passability / Occupancy Validators - Ghidra Research Report

**Address(es):** `0x0056E7C0` (`CellRect__CheckPassability`), `0x00586780` (`CellRect__CheckOccupancy`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the two rectangle validators, their parameter contracts, helper calls, occupancy-layer treatment, and standard YR active callsites through nearby-passable-cell and AI building/site selection.  
**Non-Scope:** full building placement, full `Find_Nearby_Passable_Cell`, full A*, complete cell flag taxonomy, or retail asset placement effects beyond validator calls.  
**Confidence:** High for both validator bodies and direct callsites; Medium for human names of some cell fields imported from prior reports.  
**Active in YR:** Yes. `CellRect__CheckPassability` is called only by live `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`; `CellRect__CheckOccupancy` is called by that function and by live AI/production site helper `FUN_005060B0`, which is reached from `BuildingClass__ExitObject_Main` and `HouseClass__AI_ChooseNextProduction`.

## 1. Overview

These are small boolean rectangle validators. `CheckPassability` walks a `width x height` rectangle and asks each cell whether it is passable for the supplied SpeedType / zone / MovementZone / height-layer inputs. `CheckOccupancy` walks a rectangle and rejects occupied or reserved cells, then verifies all four rectangle corners are inside the playfield.

The important split is that passability and occupancy are independent. `Find_Nearby_Passable_Cell` always calls `CheckPassability`; it calls `CheckOccupancy` only when its final `check_occupancy_rect` argument is set. AI building/site selection calls `CheckOccupancy` directly without `CheckPassability`.

## 2. Class Layout / Key Offsets

| Offset / item | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `CellRect` / rect arg | four 32-bit fields: `x`, `y`, `width`, `height`; low 16 bits are used as signed cell coords when looking up `CellClass` | Yes | `0x005867AC..0x005867CE`, `0x00578390` |
| packed `CellStruct` arg | `short x`, `short y` top-left plus explicit width/height args in `CheckPassability` | Yes | `0x0056E7E8..0x0056E7FA` |
| `CellClass+0x24/+0x26` | map cell coord stored inside `CellClass`; passed to `MapClass__GetZoneID` | Yes | `0x004834C4..0x004834CF` |
| `CellClass+0x44` | overlay type index, `-1` if none | Yes | `0x0056E83A`, `0x00483583..0x0048359E` |
| `CellClass+0x4C` | nonzero object/overlay pointer-like blocker in `CheckOccupancy`; distinct from LandType | Yes | `0x00586833..0x00586838` |
| `CellClass+0xDC` | per-house/site reservation bitmask tested as `1 << (layer & 0x1F)`; skipped when layer is `-1` | Yes | `0x00586787..0x0058679D`, `0x0058681F..0x0058682B`; AI helper passes `HouseClass+0x30` at `0x005069C0..0x005069DB` |
| `CellClass+0xE4` | ground object list, scanned for RTTI `0x24` and `WhatAmI()==6` blockers | Yes | `FUN_0047C550 @ 0x0047C550`, `Look_up_building_in_cell @ 0x0047C520` |
| `CellClass+0x11B` | height/level byte used by passability helper and nearby-cell height checks | Yes | `0x004834EF..0x00483527`, `0x0056DE26..0x0056DE4C` |
| `CellClass+0x11C` | special/slope byte; any nonzero value blocks `CheckOccupancy` | Yes | `0x0058683A..0x00586842` |
| `CellClass+0x124` | ground occupation flags used by `CellClass__CheckCellPassability` | Yes | `0x00483550..0x00483572` |
| `CellClass+0x128` | alternate/bridge occupation flags used when required height is bridge layer | Conditional | `0x0048353C..0x0048354E`; active when bridge flag `0x100` and required height is level+4 or unrestricted |
| `CellClass+0x140 bit 0x100` | bridge structural flag used for height/layer handling and nearby bridge reject | Yes | `0x004834FA..0x00483527`, `0x0056DE77..0x0056DE80` |
| `CellClass+0xEC` | LandType row index for speed table lookup | Yes | `0x0048357D`, `0x004835D5..0x004835E5` |

## 3. Core Logic

### 3.1 `CellRect__CheckPassability @ 0x0056E7C0`

Signature reconstructed from `RET 0x24`, call assembly, and the helper stack:

```text
bool CheckPassability(
  CellStruct* top_left,
  int width,
  int height,
  int speed_type,
  int required_zone_id,
  int movement_zone,
  int required_height_or_level,
  bool bridge_aware_zone,
  bool reject_any_overlay
)
```

Active in YR: Yes. Evidence: four direct calls from `FootClass__Find_Nearby_Passable_Cell` at `0x0056DE0E`, `0x0056E024`, `0x0056E265`, `0x0056E467`.

Verified behavior:

| Behavior | Evidence | Active in YR |
|---|---|---|
| Iterates `x = top_left.x .. x+width-1` outer, `y = top_left.y .. y+height-1` inner; signed `i < width/height` loops mean `width <= 0` or `height <= 0` returns true without checking any cell. | `0x0056E7CA..0x0056E87C` | Yes |
| Cell lookup is hard-coded as `y * 0x200 + x` with valid index range `[0, 0x3FFFF]`; missing/out-of-range cells use dummy cell `DAT_00ABDC50` and write requested coord to `DAT_00ABDC74`. | `0x0056E7FF..0x0056E832` | Yes |
| If `reject_any_overlay != 0` and `CellClass+0x44 != -1`, the rectangle fails before calling `CellClass__CheckCellPassability`. | `0x0056E832..0x0056E83E` | Yes, conditional on caller flag |
| Calls `CellClass__CheckCellPassability` with `speed_type`, two zero occupation-mask modifiers, `required_zone_id`, `movement_zone`, `required_height_or_level`, and `bridge_aware_zone`. | `0x0056E840..0x0056E859`; callee `0x004834A0` | Yes |
| It does not call `MapClass__IsRectInPlayfield`; rectangle bounds are not enforced here except indirectly via dummy-cell behavior and caller checks. | full body `0x0056E7C0..0x0056E88B`; no xref to `0x00578390` | Yes |

`CellClass__CheckCellPassability @ 0x004834A0` details used by this wrapper:

| Behavior | Evidence | Active in YR |
|---|---|---|
| `speed_type == 4` immediately returns true, skipping zone, height, occupation flags, overlay wall, and land-speed table. This matches Winged/Fly special handling. | `0x004834A7`, `0x004835FF` | Yes |
| If `required_zone_id != -1`, `MapClass__GetZoneID(cell, movement_zone, bridge_aware_zone)` must equal it. | `0x004834BF..0x004834D8`; `MapClass__GetZoneID @ 0x0056D230` | Conditional: only when caller passes a real zone |
| If `required_height_or_level != -1`, exact level is required unless the cell is bridge-flagged and the required value equals `level + 4`. If exact level equals a bridge cell's base level and bridge-aware flag is false, it rejects. | `0x004834E1..0x00483527` | Conditional: `Find_Nearby_Passable_Cell` passes `-1`, but other callers can use this helper |
| Chooses normal occupation byte `+0x124` or bridge alternate byte `+0x128`; nonzero after two optional masks rejects. `CheckPassability` itself passes both mask flags as zero, so it requires the selected occupation byte to be zero. | `0x00483527..0x00483572`; wrapper pushes two zero args at `0x0056E854..0x0056E856` | Yes |
| Wall overlays (`OverlayType+0x2A8`) are passable only for movement zones `2`, `3`, `8`, `0x0C`, or zones `1`/`4` when overlay byte `+0x22D` is set; accepted wall overlays force `LandType = Clear` before the speed table lookup. | `0x00483583..0x004835D5` | Conditional on wall overlay |
| Land/speed table lookup is `g_SpeedType_LandType_Table[speed_type + LandType*9]`; exact `0.0` rejects unless the alternate bridge occupation path was selected. | `0x004835D5..0x004835F6`; speed table report | Yes |

### 3.2 `CellRect__CheckOccupancy @ 0x00586780`

Signature reconstructed from `RET 0x8`:

```text
bool CheckOccupancy(CellRect* rect, int reservation_layer_or_house_index)
```

Active in YR: Yes. Evidence: four calls from `FootClass__Find_Nearby_Passable_Cell` and one from `FUN_005060B0`.

Verified behavior:

| Behavior | Evidence | Active in YR |
|---|---|---|
| If layer is `-1`, the reservation mask is zero and `CellClass+0xDC` is ignored. Otherwise mask is `1 << (layer & 0x1F)`. | `0x00586787..0x0058679D` | Yes |
| Iterates signed `x < x + width` and `y < y + height`. Negative/zero dimensions skip blocker scan but still fall through to `MapClass__IsRectInPlayfield`. | `0x005867AC..0x0058686E` | Yes |
| Cell lookup uses signed low 16-bit x/y with fixed 512-wide index; out-of-range cells use dummy `DAT_00ABDC50`. Unlike passability, final playfield check rejects out-of-play rectangles. | `0x005867D4..0x00586812`, `0x00586874..0x0058687B` | Yes |
| Rejects if `FUN_0047C550` finds RTTI `0x24` in the ground object list. `FUN_0047C550` is `__thiscall`: the cell pointer is the implicit ECX receiver (`this`), and `param_2=0` selects the `+0xE4` ground-object list (corrected 2026-05-28: was written as `FUN_0047C550(cell, 0)` implying a 2-arg C call; binary shows `__thiscall` with cell as implicit `this` receiver via `decompile_function 0x0047C550` — ROOT_CAUSE: RTTI_LABEL_DRIFT/thiscall-receiver-collapsed). | `0x00586812..0x0058681D`; helper `0x0047C550` | Yes |
| Rejects if `(cell+0xDC & mask) != 0`; this is not dynamic unit occupancy and is skipped for `-1`. | `0x0058681F..0x0058682B` | Conditional: active in AI/site helper when house index is passed; skipped by nearby-cell calls |
| Rejects if `cell+0x44 != -1`, `cell+0x4C != 0`, or `cell+0x11C != 0`. | `0x0058682D..0x00586842` | Yes |
| Rejects if `Look_up_building_in_cell` finds a `WhatAmI()==6` object on `CellClass+0xE4`. | `0x00586844..0x0058684D`; helper `0x0047C520` | Yes |
| If no blocker is found, returns `MapClass__IsRectInPlayfield(rect, 1)`, which tests all four corners: NW, NE, SW, SE using `x+width-1` and `y+height-1`. | `0x00586874..0x00586880`; `0x00578390` | Yes |

## 4. INI Keys

The validators do not read INI keys directly. Their inputs are parsed elsewhere.

| Key / data | Effect reaching validators | Active in YR | Evidence |
|---|---|---|---|
| `SpeedType=` | supplies `speed_type` into passability helpers and speed-table column selection | Yes | `CellClass__CheckCellPassability @ 0x004834A0`; `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` |
| `MovementZone=` | supplies the zone-map/overlay movement category argument | Yes | `MapClass__GetZoneID @ 0x0056D230`; `Find_Nearby` call assembly `0x0056DDE2..0x0056DE0E` |
| LandType speed table sections (`Foot=`, `Track=`, etc.) | `CellClass__CheckCellPassability` rejects exact `0.0` for the passed SpeedType/LandType | Yes | `0x004835DE`; speed-table report |
| `Buildable=` | not read by either `CellRect` validator; building placement predicate `0x0047C620` reads it in a different path | No for this slice; Yes elsewhere | no `Buildable` table byte read in `0x0056E7C0` or `0x00586780`; `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS...` |
| `AIBaseSpacing` (`RulesClass+0x1460`) | expands the rectangle used by AI/site `CheckOccupancy`, not by `Find_Nearby_Passable_Cell` | Yes for AI placement/search | `FUN_005060B0 @ 0x00506694..0x005069DB` |

## 5. Integration Points

| Caller | Validator use | Active in YR | Evidence |
|---|---|---|---|
| `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` | Calls `CheckPassability` for every candidate; optionally calls `CheckOccupancy(rect, -1)` when `param_16 != 0`. | Yes | direct xrefs at `0x0056DE0E/0x0056E024/0x0056E265/0x0056E467` and `0x0056DE9D/0x0056E0B3/0x0056E2F6/0x0056E4F8` |
| `FUN_005060B0` AI building/site helper | Calls `CheckOccupancy(expanded_rect, HouseClass+0x30)`; also calls `Find_Nearby_Passable_Cell` in an alternate branch with width/height `foundation + 2`. | Yes | `0x005069C0..0x005069DB`, `0x0050616D..0x00506193`; callers `BuildingClass__ExitObject_Main`, `HouseClass__AI_ChooseNextProduction` |
| Direct callers of `Find_Nearby_Passable_Cell` | Placement/spawn/pathing adjacent systems reach validators indirectly: airfield search, rally points, production exit, chrono/warp, fly descent relocation, patrol/area guard, slave deploy, superweapon launch, etc. | Yes, mixed contexts | caller list from Ghidra includes `BuildingClass__ExitObject_Main`, `BuildingClass__OnConstructionComplete`, `FootClass__Find_Path`, `SuperClass__Launch`, `FlyLocomotionClass__Descent_Step`, `SlaveManagerClass__FindDeployCell` |
| A* main loop | Does not call these `CellRect` validators directly; it uses `Can_Enter_Cell` virtuals. | No direct use | `PATHFINDING_ASTAR_GHIDRA_REPORT.md`; no xrefs from A* to `0x0056E7C0`/`0x00586780` |

## 6. Current Rust Implementation Status

Current Rust has the relevant surfaces but not a unified binary-compatible `CellRect` validator pair.

| Rust area | Observed status | Evidence |
|---|---|---|
| `PathGrid` | stores boolean ground/bridge walkability; out-of-bounds returns false. This is coarser than `CheckPassability`, which threads SpeedType, zone, bridge-aware height, occupation bytes, overlay wall exceptions, and dummy-cell fallback. | `src/sim/pathfinding/core.rs:1094`, `:1128`, `:1136` |
| `OccupancyGrid` | tracks dynamic entity occupancy by movement layer and subcell. It does not represent `CellClass+0xDC` house/site reservation bits, `+0x44/+0x4C/+0x11C` blocker bytes, or the RTTI `0x24` object-list scan as separate cell fields. | `src/sim/occupancy.rs:92`, `:143` |
| Building placement | walks foundation cells and checks build-blocked terrain, overlay/terrain blockers, bridge deck, slope, and structure overlap. This is the separate building-placement predicate family, not these `CellRect` validators. | `src/sim/production/production_placement.rs:267`, `:300`, `:361` |
| Production spawn / nearby fallback | uses preferred exit offsets plus `PathGrid` and `OccupancyGrid`, then a ring fallback. It does not yet expose the binary `Find_Nearby` candidate flags or `CheckPassability`/`CheckOccupancy` contract. | `src/sim/production/production_spawn.rs:112`, `:128`, `:191` |
| Zone build | uses MovementZone passability and PathGrid blockers; useful for future `required_zone_id` parity, but not equivalent to `CheckPassability`'s per-call zone comparison. | `src/sim/pathfinding/zone_build.rs:424` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellRect__CheckPassability @ 0x0056E7C0` | verified | decompile + disassembly | none for scoped validator |
| `CellClass__CheckCellPassability @ 0x004834A0` | verified for arguments and branches used by `CheckPassability` | decompile + disassembly | full non-CellRect caller taxonomy out-of-scope |
| `CellRect__CheckOccupancy @ 0x00586780` | verified | decompile + disassembly | none for scoped validator |
| `FUN_0047C550` RTTI `0x24` scan | verified | decompile `0x0047C550` | exact class identity for RTTI `0x24` out-of-scope |
| `Look_up_building_in_cell @ 0x0047C520` | verified | decompile | none for scoped behavior |
| `MapClass__IsRectInPlayfield @ 0x00578390` | verified | decompile | `Is_Cell_In_Playfield` internals out-of-scope |
| `FootClass__Find_Nearby_Passable_Cell` call contract | verified only where it invokes validators | disassembly `0x0056DC20` | full search/selection already covered by prior report |
| `FUN_005060B0` occupancy callsite | verified only around `CheckOccupancy` calls | decompile/disassembly `0x005060B0` | full AI base placement algorithm out-of-scope |
| A* direct call relationship | verified absent direct xrefs | xrefs to validator addresses | no further A* exploration in this slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Are both named addresses real functions? -> Yes: `0x0056E7C0` and `0x00586780` decompile and disassemble as separate `RET 0x24` / `RET 0x8` validators.` Evidence: `0x0056E7C0`, `0x00586780`. Active in YR: Yes.
- `[RESOLVED] OQ-2 - What calls `CheckPassability`? -> Only `FootClass__Find_Nearby_Passable_Cell` in this binary xref set.` Evidence: xrefs `0x0056DE0E`, `0x0056E024`, `0x0056E265`, `0x0056E467`. Active in YR: Yes.
- `[RESOLVED] OQ-3 - What calls `CheckOccupancy`? -> `Find_Nearby_Passable_Cell` and `FUN_005060B0`.` Evidence: xrefs `0x0056DE9D`, `0x0056E0B3`, `0x0056E2F6`, `0x0056E4F8`, `0x005069DB`. Active in YR: Yes.
- `[RESOLVED] OQ-4 - Does `CheckPassability` include occupancy? -> It checks `CellClass+0x124/+0x128` occupation bytes through `CellClass__CheckCellPassability`, but it does not scan object lists or call `CheckOccupancy`.` Evidence: `0x00483527..0x00483572`, no call to `0x00586780`. Active in YR: Yes.
- `[RESOLVED] OQ-5 - Does `CheckOccupancy` include terrain passability? -> No SpeedType, MovementZone, LandType speed, or zone lookup; it checks object/reservation/blocker fields and playfield corners only.` Evidence: full body `0x00586780..0x00586893`. Active in YR: Yes.
- `[RESOLVED] OQ-6 - What does the second `CheckOccupancy` argument do? -> `-1` disables the `Cell+0xDC` test; otherwise it tests `1 << (arg & 0x1F)` against `Cell+0xDC`.` Evidence: `0x00586787..0x0058679D`, `0x0058681F..0x0058682B`. Active in YR: Yes.
- `[RESOLVED] OQ-7 - Is the `Cell+0xDC` layer used in standard YR? -> Yes in `FUN_005060B0` with `HouseClass+0x30`; `Find_Nearby` passes `-1` so it is skipped there.` Evidence: `0x005069C0..0x005069DB`, `0x0056DE98..0x0056DE9D`. Active in YR: Conditional.
- `[RESOLVED] OQ-8 - Are out-of-bounds rectangles treated the same by both validators? -> No. `CheckPassability` has dummy-cell fallback and no final rect bounds check; `CheckOccupancy` also uses dummy fallback during scan but returns `MapClass__IsRectInPlayfield(rect,1)` at the end.` Evidence: `0x0056E824..0x0056E85E`, `0x00586874..0x00586880`. Active in YR: Yes.
- `[RESOLVED] OQ-9 - Are zero-size rectangles possible edge cases? -> Both loops skip nonpositive dimensions; `CheckPassability` then returns true, while `CheckOccupancy` still performs corner playfield checks with `width-1` / `height-1`.` Evidence: `0x0056E7CA..0x0056E87F`, `0x005867B4..0x00586880`, `0x00578390`. Active in YR: Conditional on caller-supplied dimensions.
- `[RESOLVED] OQ-10 - Does Winged/Fly bypass `CheckCellPassability`? -> Yes, `speed_type == 4` jumps to success.` Evidence: `0x004834A7`, `0x004835FF`. Active in YR: Yes for callers passing speed type 4.
- `[RESOLVED] OQ-11 - Is building placement `0x0047C620` one of these validators? -> No. It is a separate per-cell building-placement predicate; these `CellRect` validators do not call it.` Evidence: validator callees; prior `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS...`. Active in YR: Yes for separate placement path.
- `[RESOLVED] OQ-12 - Does A* call these validators directly? -> No direct xrefs; A* uses `Can_Enter_Cell` virtuals.` Evidence: xrefs to `0x0056E7C0`/`0x00586780`; `PATHFINDING_ASTAR_GHIDRA_REPORT.md`. Active in YR: No direct use.
- `[RESOLVED] OQ-13 - Are relevant INI keys read inside these validators? -> No direct INI reads; all data is passed in or stored on CellClass from prior initialization.` Evidence: full validator bodies. Active in YR: Yes.
- `[RESOLVED] OQ-14 - Are TS legacy gates controlling these functions? -> No top-level TS/Fog gate was found; individual TS-era data such as Tunnel/bridge/wall rows can still feed normal logic when present.` Evidence: full bodies and callsites. Active in YR: Yes, with content-dependent branches.
- `[DEFERRED] OQ-15 - Exact semantic identity of RTTI `0x24` in `FUN_0047C550`.` Category: out-of-scope; reason: this slot only needs blocker effect inside `CheckOccupancy`; next-step-if-pursued: class RTTI enum audit.
- `[DEFERRED] OQ-16 - Full writer/clearer taxonomy for `CellClass+0xDC`.` Category: requires-different-system-context; reason: this slot verified read contract and AI callsite only; next-step-if-pursued: audit all `cell+0xDC` writes and reconcile GapGen/BaseSpacing naming.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `CheckPassability` needs per-call SpeedType, MovementZone, zone id, required height/layer, bridge-aware flag, overlay reject flag, and full-rect all-cells success. | `0x0056E7C0`, `0x004834A0` | missing/unchecked: Rust mostly uses `PathGrid::is_walkable` and zone helpers separately | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/zone_*`, future nearby-passable-cell helper | Add a rectangle passability validation surface for spawn/nearby-cell parity that preserves these inputs instead of collapsing them into one boolean walkable grid. | A blocked destination triggers nearby-cell search; a candidate on a different zone or with an overlay when `reject_any_overlay=true` is skipped even if `PathGrid` says walkable. Proposed test: `nearby_rect_passability_rejects_overlay_and_wrong_zone` | Do not replace this with static `PathGrid::is_walkable` alone. |
| `CheckOccupancy(rect, -1)` skips `Cell+0xDC` but still rejects RTTI `0x24`, `+0x44`, `+0x4C`, `+0x11C`, building occupants, and out-of-playfield rectangles. | `0x00586780`, `0x0047C550`, `0x0047C520`, `0x00578390` | missing/unchecked: `OccupancyGrid` models dynamic entities but not all cell blocker bytes or playfield corner behavior | `src/sim/occupancy.rs`, `src/sim/production/production_spawn.rs`, scatter/nearby helpers | Separate dynamic entity occupancy from cell-field blocker validation; preserve the `-1` mask skip used by `Find_Nearby`. | Nearby fallback with an otherwise empty sloped/special cell (`+0x11C != 0`) rejects under `check_occupancy_rect`; same rect with only `+0xDC` reservation passes when layer is `-1`. Proposed test: `check_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers` | Do not treat `CheckOccupancy` as "no other units only". |
| AI/site `CheckOccupancy` passes `HouseClass+0x30`, expands the footprint by `AIBaseSpacing`, and probes up to three shifted rectangles along each candidate direction. | `0x00506694..0x005069DB` | missing/unchecked: no `Cell+0xDC` house reservation surface apparent in current Rust scan | future AI/base placement surface; possibly production placement if AI build spacing is implemented | Model house reservation bits as a separate map or equivalent predicate for AI site selection, not as unit occupancy. | Two same-house AI placement candidates with overlapping expanded reservation masks are rejected for the second house while ordinary nearby-cell `-1` occupancy remains unaffected. Proposed test: `ai_base_spacing_reservation_blocks_same_house_expanded_rect` | Do not store `Cell+0xDC` in the entity `OccupancyGrid` as if it were a unit/building occupant. |

### Negative Facts / Do Not Do

- Do not document or implement `CellRect__CheckOccupancy` as a terrain passability check. It has no SpeedType, MovementZone, LandType, or zone read. Evidence: full body `0x00586780..0x00586893`. Active in YR: Yes.
- Do not document or implement `CellRect__CheckPassability` as a full occupancy/object-list check. It only consumes occupation bytes via `CellClass__CheckCellPassability`; object-list/terrain-object/building lookup belongs to `CheckOccupancy`. Evidence: callees of `0x0056E7C0` and `0x00586780`. Active in YR: Yes.
- Do not bake `Cell+0xDC` reservation bits into static `PathGrid` or dynamic `OccupancyGrid` without preserving the caller layer argument. `Find_Nearby` passes `-1` and deliberately skips it; AI site helper passes a house index. Evidence: `0x00586787..0x0058679D`, `0x0056DE98`, `0x005069D4..0x005069DB`. Active in YR: Conditional.
- Do not use `Buildable=` to emulate `CheckPassability`; this wrapper calls `CellClass__CheckCellPassability`, which uses `SpeedType + LandType*9` speed entries. `Buildable=` is a separate building-placement predicate concern. Evidence: `0x004835DE`; `0x0047C620` docs. Active in YR: Yes.
- Do not assume rectangle bounds are identical between validators. `CheckPassability` has no final `IsRectInPlayfield`; `CheckOccupancy` does. Evidence: `0x0056E7C0..0x0056E88B` versus `0x00586874..0x00586880`. Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: replace the `CheckOccupancy` summary "`Checks the entire foundation rectangle for object occupation`" with "`Checks the entire rectangle for cell-field blockers, object-list blockers, optional house/site reservation bits, and playfield containment; dynamic unit occupancy is only one adjacent concept and `Cell+0xDC` is skipped when the layer argument is `-1`.`"
- `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: replace "`Occupancy check: CellRect__CheckOccupancy (0x586780) -- no other units blocking`" with "`Optional rectangle occupancy/blocker check: `CellRect__CheckOccupancy @ 0x00586780`; it rejects `Cell+0x44/+0x4C/+0x11C`, RTTI `0x24`, building occupants, optional `Cell+0xDC` reservation bits, and out-of-playfield rectangles. In `Find_Nearby_Passable_Cell`, the reservation layer is `-1`, so `Cell+0xDC` is skipped.`"

## 10. Remaining Uncertainty

- Exact class name for RTTI `0x24` returned by `FUN_0047C550` was not resolved; blocker effect is verified.
- Exact global semantic name and writer taxonomy for `CellClass+0xDC` remains mixed in prior docs (`GapGenBitmask`, passability/base spacing mask). This report only claims the read contract and two validator callsite meanings.
- Dummy cell `DAT_00ABDC50` initial field values were not dumped; the validator behavior is still clear: both functions substitute it for out-of-range/null cell pointers, and only `CheckOccupancy` applies final rect playfield containment.

## Sources

- Ghidra read-only decompile/disassembly: `0x0056E7C0`, `0x00586780`, `0x004834A0`, `0x0047C550`, `0x0047C520`, `0x00578390`, `0x0056DC20`, `0x005060B0`, `0x0056D230`.
- Ghidra xrefs: validators, `Find_Nearby_Passable_Cell`, `FUN_005060B0`.
- Prior docs referenced: `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`, `PATHFINDING_ASTAR_GHIDRA_REPORT.md`, `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`, `BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`, `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`.
- Rust scan: `src/sim/pathfinding/core.rs`, `src/sim/occupancy.rs`, `src/sim/production/production_placement.rs`, `src/sim/production/production_spawn.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/movement/scatter.rs`.
