# Cell Reference Points - Ghidra Research Report

**Address(es):** `0x00486840`, `0x00486890`, `CellClass__Get_Center_Coords @ 0x00480A30`, `CellClass__GetGroundHeight @ 0x00578080`, `CellClass__Get_Cell_At @ 0x00565730`, `MapClass__Get_CellClass @ 0x005657A0`, `BulletClassAiHomingDetonationPath @ 0x004666E0`, `BulletClassBulletDetonationImpactDamage @ 0x00468D80`, `Apply_area_damage @ 0x00489280`
**Investigation Mode:** coverage-map
**Claimed Scope:** CellClass cell-origin vs cell-center semantics; CellClass coordinate virtual slots `+0x48` and `+0x58`; bridge-aware Z behavior on those slots; selected target/detonation/pathing consumers that prove which reference point is used.
**Non-Scope:** all BuildingClass/TechnoClass coordinate virtuals, complete building foundation targeting, complete FLH/fire-origin math, full tactical screen inverse, every caller of every coordinate helper.
**Confidence:** High for the two CellClass slots, signed lepton-to-cell conversion, bullet/detonation consumers, and AoE building/cell-center behavior. Medium for current Rust deltas because this report scanned only the relevant surfaces, not every caller.
**Active in YR:** Yes. These paths are core map cell, projectile, warhead, movement, and pathing behavior in normal YR gameplay.

## 0. Investigation Setup

**Target question:** What reference point does gamemd.exe use for a map cell when code asks for cell coordinates, target coordinates, detonation coordinates, or ground/bridge Z?

**Non-goals:** Do not re-investigate building foundation anchor semantics, tactical pixel inverse, FLH offsets, tube movement, or global direction encoding beyond their direct cell-coordinate consumers.

**Evidence needed to mark COMPLETE:** direct assembly/decompile evidence for `CellClass +0x48` and `+0x58`; at least one combat/homing consumer, one detonation/AoE consumer, and one pathing/movement/height consumer; clear Rust handoff and negative facts.

**Stop conditions:** stop at a coverage map once CellClass reference-point primitives and the highest-impact consumers are verified; defer whole-system consumer enumeration and non-CellClass coordinate virtuals to sibling swarm slots.

## 1. Overview

`CellClass` does not expose the northwest/origin corner as its normal coordinate. The active coordinate virtual at `+0x48` returns the center of the cell: `x = MapCoord_X * 256 + 128`, `y = MapCoord_Y * 256 + 128`, and `z = ground height at that center`. The target-coordinate virtual at `+0x58` delegates to `+0x48`; for cells marked as structural bridge cells (`CellClass+0x140 & 0x100`), it adds global bridge-height Z (`DAT_0089E7B4`) to the returned Z.

Combat and movement consumers mix these reference points intentionally. Homing target reads use target virtual `+0x58`; some final projectile snap paths use `+0x48` after a `+0x58` distance test. `Apply_area_damage` converts arbitrary impact coordinates to a containing cell using signed lepton-to-cell conversion, then uses that cell center for spread and building-in-neighbor-cell distance checks.

## 2. Class Layout / Key Offsets

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x24` | `MapCoord_X` signed 16-bit cell coordinate | `0x00486845` reads `word ptr [ECX+0x24]`; struct report | Yes |
| `CellClass+0x26` | `MapCoord_Y` signed 16-bit cell coordinate | `0x00486849` reads `word ptr [ECX+0x26]`; struct report | Yes |
| `CellClass+0x11B` | terrain level byte, read signed by effective-height code | `CellClass__Get_Effective_Height`; `CELLCLASS_STRUCT_GHIDRA_REPORT.md` | Yes |
| `CellClass+0x140 bit 0x80` | bridge overlay/effective height bit; adds `+4` levels in effective-height helper | `CellClass__Get_Effective_Height @ 0x005F5F00`, struct report | Yes |
| `CellClass+0x140 bit 0x100` | structural bridge cell; used by `+0x58` and AoE layer selection | `0x00486890`, `Apply_area_damage @ 0x00489280` | Yes |
| vtable `+0x48` | cell center / ground coordinate | assembly `0x00486840..0x00486886` | Yes |
| vtable `+0x58` | target coordinate, bridge-Z adjusted for `0x100` cells | assembly `0x00486890..0x004868F8` | Yes |
| `DAT_0089E7B4` | Z addend used by CellClass `+0x58` bridge branch | `0x004868A1`, `0x004868B8` | Yes |
| `DAT_0089E864 / DAT_0089E870` | bridge/Z thresholds used by AoE bridge and bridge-damage gates | `Apply_area_damage @ 0x00489280` | Yes |

## 3. Core Logic

### 3.1 `CellClass +0x48 @ 0x00486840`

Verified assembly, Active in YR: Yes.

`+0x48` reads the cell's stored map coordinates, shifts them by 8, then adds `0x80` to both axes. It passes a stack coordinate initialized with `x=0x80`, `y=0x80` into the ground-height helper, then writes `{center_x, center_y, ground_z}` to the caller-provided `CoordStruct`.

Important details:

- `MapCoord_X` and `MapCoord_Y` are read as signed words at `+0x24/+0x26`.
- The formula is exact: `(coord << 8) + 0x80`; no origin/NW return appears in this slot.
- The function returns via `RET 0x4`, so the caller supplies a destination `CoordStruct*` stack arg.
- Z comes from `CALL 0x0047B3A0` in the inline body at `0x0048686C`; the named wrapper `CellClass__GetGroundHeight @ 0x00578080` shows the same signed lepton-to-cell and ground-height helper path.

The separately named `CellClass__Get_Center_Coords @ 0x00480A30` has the same center formula and ground-height lookup. It is widely called directly by movement/render/logic functions, while `0x00486840` is the vtable slot reached by polymorphic AbstractClass coordinate calls.

### 3.2 `CellClass +0x58 @ 0x00486890`

Verified assembly, Active in YR: Yes.

The target-coordinate slot starts with `MOV EAX, [ECX+0x140]` and tests `AH & 1`, which is the dword bit `0x100`.

- If `CellClass+0x140 & 0x100` is set:
  - load `DAT_0089E7B4`;
  - call `this->vtable+0x48`;
  - copy X and Y unchanged;
  - write `Z = returned_Z + DAT_0089E7B4`.
- If the bit is clear:
  - call `this->vtable+0x48`;
  - copy X, Y, and Z unchanged.

This slot is a target coordinate, not a different XY anchor. The bridge branch only changes Z.

### 3.3 Lepton-to-cell conversion

Verified binary finding, Active in YR: Yes.

Both `CellClass__Get_Cell_At @ 0x00565730` and `Apply_area_damage @ 0x00489280` convert absolute leptons to a cell coordinate with signed bias:

`cell = (coord + (coord >> 31 & 0xFF)) >> 8`

For non-negative gameplay coordinates this is normal truncating division by 256. For negative coordinates, the `+0xFF` before shift avoids rounding down to the previous cell.

### 3.4 `MapClass__Get_CellClass @ 0x005657A0`

Verified decompile, Active in YR: Yes.

The cell lookup by packed cell coordinate uses `index = y * 0x200 + x`. If `index < 0`, `index > 0x3FFFF`, or the cell pointer is null, it stores the requested cell in `DAT_00ABDC74` and returns dummy cell `DAT_00ABDC50`. This matters because cell-target retargeting and off-map coordinate paths still receive a valid `CellClass` object, not a null.

### 3.5 `Apply_area_damage @ 0x00489280`

Verified decompile, Active in YR: Yes.

`Apply_area_damage` receives an arbitrary `CoordStruct* impactCoords`, converts it to `impactCell`, and uses the cell center as the blast reference:

- `cellX/cellY` from signed lepton-to-cell conversion.
- `centerX = cellX * 256 + 128`, `centerY = cellY * 256 + 128`, `centerZ = 0`.
- If `groundHeight < impactZ`, it separately scans airborne objects.
- If `impactCell->Flags & 0x100` and `groundHeight + DAT_0089E864 / 2 < impactZ`, it selects the bridge-layer object list (`CellClass+0xE8`) instead of the ground list (`CellClass+0xE4`) for each spread cell.
- For building-class objects (`WhatAmI()==6`) in non-center spread cells, distance is from the current spread cell's `+0x48` center to the impact coord. For a building in the impact cell, distance is forced to `0`.
- For non-building objects, distance is from object vtable `+0xA4` target/center coords to the impact coord.

This means blast spread is cell-centered even when the original projectile impact coordinate had sub-cell offset.

### 3.6 Bullet homing and detonation consumers

Verified decompile, Active in YR: Yes.

`BulletClassAiHomingDetonationPath @ 0x004666E0`:

- If `BulletClass+0x10C` target is null, the ROT>0 branch uses sentinel coords `DAT_0089DE30/34/38`.
- If target is non-null, it calls target vtable `+0x58` for homing target coordinates.
- If target has ObjectClass-like flag bit (`AbstractFlags & 2`), it may replace those coords with target vtable `+0xA4`; `CellClass` does not take that branch in the prior AAHeatSeeker2 report.
- Close-target detonation may copy the `+0x58` coordinate into the impact point unless the coordinate equals the sentinel and unless bullet type flags suppress snapping.
- Later proximity/final snap path calls target `+0x58` for a distance test, then calls target `+0x48` and writes the bullet coordinate via its own `+0x1B4`. For `CellClass`, this means final snap can drop from bridge-adjusted target Z back to ground-center Z.

`BulletClassBulletDetonationImpactDamage @ 0x00468D80`:

- Starts from bullet current `+0x9C/+0xA0/+0xA4`.
- For a non-null target and snap-enabled bullet type, computes distance to target `+0x48`; if distance `< 0x20` leptons and flags allow, it calls `+0x48` again for snap.
- Later generic close-target branch may call target `+0x58`; if target `WhatAmI()==6` and building art target offsets exist, it may replace that with target `+0xA4`.
- Calls `WarheadTypeClass__Detonate` with the prepared impact coordinate.

## 4. INI Keys

No INI key defines the cell center constants `256` or `128`; they are hardcoded coordinate-system facts in the binary. Relevant keys only affect consumers:

| File | Key | Effect in this scope |
|---|---|---|
| `rulesmd.ini` `CellSpread` | Determines integer spread radius used by `Apply_area_damage`; binary uses `ftol(CellSpread)` to index `CellSpreadTable`. |
| `rulesmd.ini` `BridgeStrength`, bridge warheads | Affect bridge damage after an impact cell is selected; not the cell reference point itself. |
| `rulesmd.ini` projectile flags such as `ROT`, `Inaccurate`, `Airburst` | Determine which bullet target-coordinate branches can snap to target coords. |
| `artmd.ini` building target/offset keys | Can redirect building target coordinate through object/building virtuals; out-of-scope for this CellClass slot except for the `WhatAmI()==6` branch noted above. |

## 5. Integration Points

| Consumer | Reference point used | Evidence | Active in YR |
|---|---|---|---|
| Polymorphic CellClass coordinate read | `+0x48`: cell center, ground Z | `0x00486840..0x00486886` | Yes |
| Polymorphic CellClass target coord | `+0x58`: center XY, bridge-adjusted Z on `0x100` | `0x00486890..0x004868F8` | Yes |
| Direct center coordinate calls | same center formula | `CellClass__Get_Center_Coords @ 0x00480A30` and callers | Yes |
| Homing missile target acquisition | target vtable `+0x58` unless null sentinel or object alternate target path | `0x004666E0` | Yes |
| Final bullet snap | can use `+0x58` for distance then `+0x48` for snap | `0x00467CA9..0x00467E4D` in prior report; decompile here | Yes |
| Bullet detonation preparation | target `+0x48`, later `+0x58`/`+0xA4` branches | `0x00468D80` | Yes |
| AoE spread | impact cell center for spread and building-neighbor distance | `0x00489280` | Yes |
| Bridge AoE object list | impact Z threshold selects ground list `+0xE4` or bridge list `+0xE8` | `0x00489280` | Yes |
| Movement/pathing direct center users | direct `CellClass__Get_Center_Coords` callers include Drive, Ship, Walk locomotion, Unit tube movement | caller list for `0x00480A30` | Yes |

## 6. Current Rust Implementation Status

Rust already has the basic cell-center convention:

- `src/util/lepton.rs` defines `LEPTONS_PER_CELL = 256` and `CELL_CENTER_LEPTON = 128`.
- `src/sim/combat/mod.rs::cell_center_coords` returns `(rx, ry, 128, 128)` for cell targets.
- `src/sim/combat/in_range.rs` uses `(rx*256+128, ry*256+128)` for `TargetKind::Cell`.
- `src/sim/combat/combat_aoe.rs` uses `CELL_CENTER_LEPTON` as the impact point for AoE distance.

Observed gaps/deltas:

- There is no single Rust abstraction matching `CellClass +0x48` vs `+0x58`. Current code has scattered `CELL_CENTER_LEPTON`, bridge-Z helpers, and per-call assumptions.
- `src/sim/movement/homing_movement.rs` stores `last_known_rx/ry` only and computes target vectors to whole-cell coordinates, effectively cell origins in cell-space, not explicit center leptons or bridge-aware target Z.
- `src/sim/combat/combat_aoe.rs` selects a single object-list layer from the impact cell and then applies that layer to all spread cells. Binary sets the bridge-layer decision from the impact cell too, but building-specific distance behavior and airborne scan are only partly modeled.
- Direct-hit and AoE building reference points are split in binary: direct object targeting may use structure/foundation target coords, while AoE building spread cells use current spread cell center or zero distance in the impact cell. Rust currently reuses `target_coords` foundation-center logic for many combat decisions and uses entity positions for AoE distances.
- Rust bridge Z constants are partially provisional: `BRIDGE_HEIGHT_DELTA_LEPTONS = 416` is documented as placeholder/default-derived, while binary consumers use globals such as `DAT_0089E7B4`, `DAT_0089E864`, and `DAT_0089E870`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / stop conditions | verified | Section 0 | none |
| CellClass vtable `+0x48 @ 0x00486840` | verified | assembly context `0x00486840..0x00486886` | none |
| CellClass vtable `+0x58 @ 0x00486890` | verified | assembly context `0x00486890..0x004868F8` | none |
| `CellClass__Get_Center_Coords @ 0x00480A30` | verified | decompile | none |
| `CellClass__GetGroundHeight @ 0x00578080` | touched-not-exhausted | decompile plus `FUN_0047B3A0` decompile | exact slope/table math inside `FUN_0047B3A0` is out-of-scope |
| Signed lepton-to-cell conversion | verified | `CellClass__Get_Cell_At @ 0x00565730`, `Apply_area_damage @ 0x00489280` | none |
| `MapClass__Get_CellClass @ 0x005657A0` dummy fallback | verified | decompile | none |
| Bullet homing `+0x58` consumer | verified | `BulletClassAiHomingDetonationPath @ 0x004666E0` | exact all-bullet flag names deferred |
| Bullet detonation `+0x48/+0x58/+0xA4` consumer | verified | `0x00468D80` | exact all-bullet flag names deferred |
| `Apply_area_damage` cell-center spread | verified | `0x00489280` | full special-warhead aftermath out-of-scope |
| `Apply_area_damage` bridge-layer selection | verified | `0x00489280` | exact numeric global values need runtime/global init audit |
| Movement/pathing direct center callers | touched-not-exhausted | caller list for `0x00480A30`, pathing docs | individual caller branch behavior out-of-scope |
| Current Rust coordinate constants | verified | `src/util/lepton.rs`, `src/sim/combat/mod.rs`, `src/sim/combat/in_range.rs` | none |
| Current Rust homing target reference point | touched-not-exhausted | `src/sim/movement/homing_movement.rs` scan | exact intended homing parity pending separate projectile pass |
| Current Rust AoE reference point | touched-not-exhausted | `src/sim/combat/combat_aoe.rs` scan | airborne splash and building-distance parity need implementation audit |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-CELLREF-001 - What is the target question? -> Verify CellClass cell-origin vs cell-center semantics, `+0x48/+0x58`, bridge Z, and selected consumers.` (evidence: user scope; this report Section 0)
- `[RESOLVED] OQ-CELLREF-002 - What is out of scope? -> Building anchor, tactical inverse, FLH, and all non-CellClass virtuals except direct consumer mentions.` (evidence: user scope; this report Section 0)
- `[RESOLVED] OQ-CELLREF-003 - Does CellClass `+0x48` return origin or center? -> Center: `(x<<8)+0x80`, `(y<<8)+0x80`, ground Z.` (evidence: `0x00486840..0x00486886`)
- `[RESOLVED] OQ-CELLREF-004 - Does CellClass `+0x58` change XY? -> No; it copies `+0x48` XY and only conditionally adds Z.` (evidence: `0x00486890..0x004868F8`)
- `[RESOLVED] OQ-CELLREF-005 - Which CellClass flag gates `+0x58` bridge Z? -> `CellClass+0x140 & 0x100` via `TEST AH,1`.` (evidence: `0x00486890..0x0048689D`)
- `[RESOLVED] OQ-CELLREF-006 - What Z addend does `+0x58` use? -> global `DAT_0089E7B4`.` (evidence: `0x004868A1`, `0x004868B8`)
- `[RESOLVED] OQ-CELLREF-007 - Is `+0x48` live through the vtable? -> Yes, vtable data xref `0x007E4F34 -> 0x00486840`.` (evidence: `get_xrefs_to 00486840`)
- `[RESOLVED] OQ-CELLREF-008 - Is `+0x58` live through the vtable? -> Yes, vtable data xref `0x007E4F44 -> 0x00486890`.` (evidence: `get_xrefs_to 00486890`)
- `[RESOLVED] OQ-CELLREF-009 - How are arbitrary lepton coords mapped to a cell? -> signed bias then arithmetic shift by 8.` (evidence: `CellClass__Get_Cell_At @ 0x00565730`, `Apply_area_damage @ 0x00489280`)
- `[RESOLVED] OQ-CELLREF-010 - Does `MapClass__Get_CellClass` return null off-map? -> No; it returns dummy cell and records requested coord.` (evidence: `0x005657A0`)
- `[RESOLVED] OQ-CELLREF-011 - Which coordinate does homing read from a target? -> vtable `+0x58` for non-null target, sentinel globals for null target.` (evidence: `0x004666E0`)
- `[RESOLVED] OQ-CELLREF-012 - Can final bullet snap use ground-center after bridge-aware target test? -> Yes, the final path can call `+0x58` for distance then target `+0x48` before writing bullet coords.` (evidence: `0x004666E0`; prior `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-CELLREF-013 - Does `BulletDetonation` use CellClass `+0x48`? -> Yes, generic target distance/snap calls target vtable `+0x48`.` (evidence: `0x00468D80`)
- `[RESOLVED] OQ-CELLREF-014 - Does AoE use impact sub-cell or cell center for spread reference? -> It converts to impact cell and constructs `cell*256+128` center for spread checks.` (evidence: `0x00489280`)
- `[RESOLVED] OQ-CELLREF-015 - How does AoE choose ground vs bridge object list? -> If impact cell has `0x100` and impact Z is above `ground + bridgeHeight/2`, use `+0xE8`; otherwise `+0xE4`.` (evidence: `0x00489280`)
- `[RESOLVED] OQ-CELLREF-016 - How does AoE treat building distance? -> Building in center cell distance is `0`; building in non-center spread cell uses that spread cell's `+0x48` center.` (evidence: `0x00489280`)
- `[RESOLVED] OQ-CELLREF-017 - Does Rust have cell-center constants? -> Yes: `LEPTONS_PER_CELL=256`, `CELL_CENTER_LEPTON=128`.` (evidence: `src/util/lepton.rs`)
- `[RESOLVED] OQ-CELLREF-018 - Does Rust cell target use center? -> Yes in combat target coords and in-range code.` (evidence: `src/sim/combat/mod.rs::cell_center_coords`; `src/sim/combat/in_range.rs`)
- `[RESOLVED] OQ-CELLREF-019 - Does Rust have a `+0x48/+0x58` semantic split? -> No single helper found; semantics are scattered.` (evidence: `rg CELL_CENTER_LEPTON/BRIDGE_HEIGHT_DELTA`, scanned files)
- `[RESOLVED] OQ-CELLREF-020 - Does Rust homing preserve target center/bridge Z? -> Current state stores only `last_known_rx/ry` and computes vectors to whole-cell values, no explicit `128,128` or bridge Z.` (evidence: `src/sim/movement/homing_movement.rs`)
- `[RESOLVED] OQ-CELLREF-021 - Tick-cycle integration: where do consumers run? -> Bullet AI/detonation and Apply_area_damage are active combat/movement tick paths; Rust equivalents run in combat tick and homing movement tick.` (evidence: `0x004666E0`, `0x00468D80`, `0x00489280`; Rust scans)
- `[RESOLVED] OQ-CELLREF-022 - TS legacy filter: are these CellClass slots YR-active? -> Yes, core CellClass vtable slots and projectile/AoE paths are stock YR active.` (evidence: vtable xrefs and active caller paths)
- `[DEFERRED] OQ-CELLREF-023 - What are exact runtime values/init sources for `DAT_0089E7B4`, `DAT_0089E864`, `DAT_0089E870`?` (category: `requires-different-system-context`; reason: needs global init/Rules bridge-height audit; next-step-if-pursued: investigate bridge-height global initialization)
- `[DEFERRED] OQ-CELLREF-024 - Do all movement/pathing callers of direct `CellClass__Get_Center_Coords` use center identically?` (category: `bounded-cost-too-high`; reason: caller list is broad and belongs to movement/pathing swarm slots; next-step-if-pursued: caller-by-caller movement coordinate audit)
- `[DEFERRED] OQ-CELLREF-025 - What are exact names for every BulletType flag around detonation snapping (`+0x294`, `+0x2A2`)?` (category: `out-of-scope`; reason: this report only needs branch effects; next-step-if-pursued: BulletType flag inventory)
- `[DEFERRED] OQ-CELLREF-026 - Complete building `+0xA4`/foundation target-coordinate semantics?` (category: `out-of-scope`; reason: slot 5 owns building/foundation anchor semantics; next-step-if-pursued: reconcile with BUILDING_FOUNDATION_ANCHOR_SEMANTICS)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Cell coordinate virtual `+0x48` is always cell center XY plus ground Z | `0x00486840..0x00486886` | mostly present, scattered | `src/util/lepton.rs`, `src/sim/combat/mod.rs`, future coordinate helper | Provide a canonical `cell_center_coord` / ground coord helper with exact `256/128` convention | `cellclass_get_coords_returns_center_not_origin` | Do not use NW/origin `(x*256,y*256)` for CellClass coordinate calls |
| Cell target virtual `+0x58` only adds bridge Z on `Cell+0x140 & 0x100` | `0x00486890..0x004868F8` | missing as single concept | `src/util/lepton.rs`, `src/map/resolved_terrain.rs`, combat/homing helpers | Add a distinct "cell target coord" path that returns center XY and bridge-adjusted Z only for structural bridge cells | `cellclass_target_coord_adds_bridge_z_without_xy_shift` | Do not conflate `has_bridge_deck`, `bridge_walkable`, and structural bridge `0x100` unless verified equivalent |
| Homing missiles read target `+0x58`; null targets use sentinel, not last cell origin | `0x004666E0` | mismatch risk observed: `last_known_rx/ry` only, no `128/128` or bridge Z | `src/sim/movement/homing_movement.rs` | Store/compute target as lepton center coordinate and preserve bridge-aware target Z where relevant | `homing_retargeted_cell_tracks_center_leptons` | Do not aim at integer cell coordinates as if they were centers |
| Final snap path can test `+0x58` then snap to `+0x48` | `0x004666E0`, `0x00468D80` | unchecked/missing in Rust homing detonation | `src/sim/movement/homing_movement.rs`, combat projectile detonation dispatch | Separate bridge-aware distance coordinate from ground-center snap coordinate for CellClass targets | `homing_cell_final_snap_uses_getcoords_after_targetcoord_distance` | Do not assume a single target coordinate is valid for all detonation branches |
| `Apply_area_damage` converts impact CoordStruct to cell and uses cell center as blast reference | `0x00489280` | partly present | `src/sim/combat/combat_aoe.rs` | Keep AoE blast origin at `(impact_cell,128,128)` even for sub-cell impact effects | `aoe_impact_subcell_uses_cell_center_for_distance` | Do not use projectile sub-cell impact as AoE center unless a separate binary branch proves it |
| AoE building distance uses center-cell full damage and neighbor-cell center distance | `0x00489280` | mismatch risk observed: entity/foundation positions are used in the scanned AoE distance path | `src/sim/combat/combat_aoe.rs` | Special-case structures in AoE per binary: center spread cell distance `0`, non-center uses spread cell center | `aoe_structure_center_cell_distance_zero` and `aoe_structure_neighbor_uses_cell_center` | Do not reuse direct-fire foundation target coords for splash building distance |
| AoE selects ground vs bridge object list from impact Z threshold | `0x00489280` | partial: layer selection exists, constants/globals unresolved | `src/sim/combat/combat_aoe.rs`, `src/util/lepton.rs` | Preserve impact-cell bridge-layer selector and audit constants against binary globals | `aoe_bridge_impact_selects_alt_object_list_above_half_bridge_height` | Do not derive bridge splash layer solely from target entity `on_bridge` |
| Signed lepton-to-cell conversion biases negatives before `>> 8` | `0x00565730`, `0x00489280` | unchecked | any future absolute-lepton conversion helper | Use signed-bias conversion for absolute CoordStruct to CellStruct paths | `signed_lepton_to_cell_negative_bias_matches_gamemd` | Do not use Rust `/ 256` or floor behavior blindly for negative coords |

**Stale Docs / Follow-up Docs**

- `WARHEAD_DETONATE_GHIDRA_REPORT.md` broadly matches the `Apply_area_damage` cell-center behavior, but future edits should tighten wording: "building in impact cell takes full damage (`dist=0`); building in non-center spread cells uses that spread cell's `CellClass +0x48` center."
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` remains useful for offsets, but its `+0x110` ZAdjust note is superseded by `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`; not directly part of this cell-reference slice.

## 10. Negative Facts / Do Not Do

- Do not model `CellClass +0x48` as the northwest/origin corner.
- Do not shift XY for `CellClass +0x58`; only Z changes, and only when `Cell+0x140 & 0x100` is set.
- Do not use direct-fire building/foundation target coordinates as a blanket replacement for AoE structure distance.
- Do not collapse ground, bridge deck, render ZAdjust, and projectile target Z into one `z` field. The binary uses separate globals/fields for these.
- Do not assume a destroyed target becoming a `CellClass*` is the same as null target; homing null uses sentinel globals, while retargeted cells return ordinary center coords.

## 11. Remaining Uncertainty

- Exact runtime/init values for `DAT_0089E7B4`, `DAT_0089E864`, and `DAT_0089E870` were not rederived in this slot.
- Full caller census for `CellClass__Get_Center_Coords @ 0x00480A30` is not exhausted; caller list proves broad active use but not each branch's local semantics.
- Building `+0xA4`/foundation target coordinate semantics are intentionally left to the building-anchor swarm slot.
- Rust deltas are a handoff audit, not a complete code review of every consumer.

## 12. Proposed Rust Test Names

- `cellclass_get_coords_returns_center_not_origin`
- `cellclass_target_coord_adds_bridge_z_without_xy_shift`
- `signed_lepton_to_cell_negative_bias_matches_gamemd`
- `homing_retargeted_cell_tracks_center_leptons`
- `homing_cell_final_snap_uses_getcoords_after_targetcoord_distance`
- `aoe_impact_subcell_uses_cell_center_for_distance`
- `aoe_structure_center_cell_distance_zero`
- `aoe_structure_neighbor_uses_cell_center`
- `aoe_bridge_impact_selects_alt_object_list_above_half_bridge_height`

## Sources

- Ghidra read-only:
  - assembly context `0x00486840..0x00486886`
  - assembly context `0x00486890..0x004868F8`
  - `CellClass__Get_Center_Coords @ 0x00480A30`
  - `CellClass__GetGroundHeight @ 0x00578080`
  - `CellClass__Get_Cell_At @ 0x00565730`
  - `MapClass__Get_CellClass @ 0x005657A0`
  - `BulletClassAiHomingDetonationPath @ 0x004666E0`
  - `BulletClassBulletDetonationImpactDamage @ 0x00468D80`
  - `Apply_area_damage @ 0x00489280`
- Prior research:
  - `docs/research/AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md`
  - `docs/research/CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`
  - `docs/research/CELLCLASS_STRUCT_GHIDRA_REPORT.md`
  - `docs/research/WARHEAD_DETONATE_GHIDRA_REPORT.md`
- Rust surfaces scanned:
  - `src/util/lepton.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/combat/combat_aoe.rs`
  - `src/sim/combat/in_range.rs`
  - `src/sim/movement/homing_movement.rs`
  - `src/map/resolved_terrain.rs`
