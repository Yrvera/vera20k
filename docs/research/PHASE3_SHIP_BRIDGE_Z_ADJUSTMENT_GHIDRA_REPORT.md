# Phase 3 Ship Bridge Z Adjustment — Ghidra Research Report

**Address(es):** `g_BridgeZ_Offset @ 0x00B0782C`; initializer `0x0069EBB0`; direct consumers `0x0069F4F2`, `0x006A06B7`, `0x006A0F58`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the active-retail `ShipLocomotionClass` bridge-Z global, its complete static initialization and lifetime, every direct read/write, the stock naval types and map state that can reach each reader, the propagated movement effect, and the current Rust parity delta for Phase-3 GSI-04.03  
**Non-Scope:** the separate Drive/Walk height globals; the complete Ship locomotor; generic bridge rendering; non-stock mods that set `IsTrain=` or `Passive=`  
**Confidence:** High  
**Active in YR:** Yes. The destination-Z and braking-distance readers are active for ordinary stock Ship-locomotor movement when the stored destination cell has `Cell+0x140 & 0x100`. The other `Process_Drive_Track` reader is compiled and reachable for mods, but inactive for all inspected stock retail naval types and maps because it requires `IsTrain=true` and stock supplies none.

## 1. Overview

The process-global at `0x00B0782C` is a Ship-locomotor-specific vertical clearance in world leptons. It is zero in the cold PE image, is computed once during the static initialization table, becomes exactly **416**, and is never rewritten or serialized. Its ordinary stock effect is not a render-depth adjustment: it changes a Ship locomotor's stored immediate destination Z and, during the acceleration/braking branch, makes the distance-to-destination calculation genuinely three-dimensional when that destination is a structural bridge cell.

The current Rust constant already has the right magnitude. The active Rust mechanism does not match: it checks the mover's **current** cell, accepts any `bridge_walkable` cell, and adds 416 linearly to an already-computed planar distance. Native checks the **stored destination** cell's exact structural bit and places 416 inside the Z component of a 3D square root.

## 2. Class Layout / Key Offsets

All Ship-locomotor offsets below are from the complete object unless explicitly identified as ILocomotion-relative. `ShipLocomotionClass`'s ILocomotion subobject begins at complete-object `+0x04`, so `FUN_0069F450`'s displayed `this+0x30/+0x34/+0x38` writes complete-object `+0x34/+0x38/+0x3C`.

| Owner | Offset / address | Type | Verified role | Evidence | Active in YR? |
|---|---:|---|---|---|---|
| process | `0x00B0782C` | signed 32-bit integer | Ship bridge Z offset, exact post-init value 416 leptons | sole writer `0x0069EBD0`; four-xref census | Yes |
| process | `0x00B07838` | signed 32-bit integer | Ship projected height step, exact post-init value 104 leptons | writer `0x0069EB3B`; reader `0x0069EBB1` | Yes |
| Ship locomotor | `+0x34/+0x38/+0x3C` | three signed 32-bit world coordinates | immediate destination X/Y/Z; `+0x3C` receives the bridge bump | `0x0069F4A9..0x0069F4FD`; `0x006A068A..0x006A068D` | Yes |
| Ship locomotor | `+0x58` | signed 32-bit track index/state | braking block runs only while `< 0x40` | `0x006A0678..0x006A0684` | Conditional, normal active tracks |
| Unit/Foot owner | `+0x9C/+0xA0/+0xA4` | three signed 32-bit world coordinates | current X/Y/Z used by both 3D distance and layer selection | `0x006A06DF..0x006A0725`; `0x006A0F11..0x006A0F26` | Yes |
| Unit/Foot owner | `+0x8C` | byte bool | `OnBridge`; forces the second reader to choose the bridge/deck object list | `0x006A0F07..0x006A0F0F` | Yes as state; false for ordinary ships under a flat high bridge |
| Unit/Foot owner | `+0x6D0` | byte bool | suppresses the `IsTrain` object-list iteration when nonzero | `0x006A0EF6..0x006A0F01` | Conditional |
| Unit owner | `+0x6C4` | `UnitTypeClass*` | type pointer used by the `Passive` guard | `0x006A0662` | Yes |
| TechnoType | `+0x2F8` | signed 32-bit leptons | `SlowdownDistance`; compared using strict signed `<` after distance truncation | `0x006A076F..0x006A077D` | Yes; default 500 |
| TechnoType | `+0xC94` | byte bool | **`IsTrain`**, not `TooBigToFitUnderBridge` | parser `0x00712270..0x00712284`; ctor `0x007113B3` | No for stock retail data |
| TechnoType | `+0xDBD` | byte bool | `Accelerates`; false bypasses the whole distance/braking block | ctor `0x00711651`; read `0x006A0644` | Yes; true for all stock Ship-locomotor types |
| UnitType | `+0xE0C` | byte bool | `Passive`; true skips the distance/braking block for UnitClass owners | `0x006A0657..0x006A0676`; parser independently established in the cited Passive report | No stock assignments |
| UnitType | `+0xE16` | byte bool | `TooBigToFitUnderBridge`; unrelated to all three reads of `0x00B0782C` | string/parser `0x00747747..0x00747778` | Conditional elsewhere, not this mechanism |
| Cell | `+0x140`, bit `0x100` | structural flag | the exact predicate for both ordinary bridge-Z applications | `0x0069F4E7..0x0069F4F0`; `0x006A06B1..0x006A06D1` | Yes on intact structural high-bridge cells |
| Cell | `+0xE4` / `+0xE8` | object-list heads | ground/water list versus bridge/deck list selected by the `IsTrain` reader | `0x006A0F71`, `0x006A0F88` | Reader inactive for stock naval types |

### Vtable identity proof

The active setter is not identified from its Ghidra name. The Ship ILocomotion vtable is `0x007F2D8C`; `vtable-4 @ 0x007F2D88` points to CompleteObjectLocator `0x008093A0`; `COL+0x0C` points to TypeDescriptor `0x0083F880`, whose mangled name is `.?AVShipLocomotionClass@@`. Slot 17 (`+0x44`, table address `0x007F2DD0`) contains `0x0069F450`, and that body's receiver-relative writes and bridge-global read prove the setter role.

The four owner guards dispatch through the UnitClass vtable `0x007F5C70`; `vtable-4` points to COL `0x0080CC68`, whose TypeDescriptor is `0x00842D80` (`.?AVUnitClass@@`). The resolved slot bodies are:

1. `+0x37C -> 0x00746C90`: `UnitClass__IsCrashing`; nonzero aborts the coordinate write.
2. `+0x380 -> 0x004DE770`: returns whether the rearm timer still has time remaining; nonzero aborts.
3. `+0x1D4 -> 0x0070C5B0`: returns owner byte `+0x270` (`IsWarpingOut`); nonzero aborts.
4. `+0x1D8 -> 0x0070C5C0`: returns owner byte `+0x271` (`IsBeingWarped`); nonzero aborts.

This corrects old Ship reports that guessed deploy/undeploy or warp names from slot positions without resolving the UnitClass entries.

## 3. Core Logic

### 3.1 One-shot initialization and exact value

The cold image bytes at `0x00B0782C`, `0x00B07838`, `0x00B077C0`, `0x00B077E0`, and `0x00B077E8` are zero. That is BSS/static-image state, not the gameplay value.

The initialization table at `0x00814A40` contains, in this relevant order:

| Table address | Target | Effect used here |
|---:|---:|---|
| `0x00814A44` | `0x0069EA40` | compute projected 256-by-256 cell diagonal into `0x00B077E0` |
| `0x00814A54` | `0x0069EAD0` | compute 60 degrees (`pi/180 * 60`) into `0x00B077E8` |
| `0x00814A58` | `0x0069EAF0` | compute 90 degrees (`pi/180 * 90`) into `0x00B077C0` |
| `0x00814A5C` | `0x0069EB10` | compute `g_ShipHeightStep` |
| `0x00814A68` | `0x0069EBB0` | compute `g_BridgeZ_Offset` after the height step |

The exact static derivation is reproducible from active-binary bytes:

1. `0x0069EA40` computes `Sqrt_Approx(2 * pow(256.0, 2.0))`. The lookup-table entry at `0x0086D0BC` is `f3 04 35 00`; after the exponent bits are added by `Sqrt_Approx @ 0x004CAC40`, the returned float has bits `0x43B504F3` (approximately 362.0387), stored as a double.
2. `0x0069EAD0` and `0x0069EAF0` use the common `pi/180` literal at `0x007F1300` with double literals 60 and 90 at `0x007E1728/30`. Their difference at `0x0069EB10` is 30 degrees (`pi/6`).
3. `0x004CAD50` is a **tangent** lookup, not sine: it multiplies radians by the double at `0x007E8970` (`4096/(2*pi)`), converts toward zero, wraps to 12 bits, and loads a float from `0x0085D0A4 + index*4`. For `pi/6`, index 341 selects bytes `8f a0 13 3f` at `0x0085D5F8` (approximately 0.5766687).
4. `0x0069EB10` multiplies the diagonal, tangent-table result, and the double 0.5 at `0x007E1738`: approximately `104.3881797`. `Math__ftol @ 0x007C5F00` uses x87 control word `0x0E7F` from `0x00822D80`; RC=`11b`, so the signed conversion truncates toward zero. The store at `0x0069EB3B` is therefore exactly `g_ShipHeightStep = 104`.
5. `0x0069EBB0` loads that signed dword, forms the low 32-bit result of `height_step * 4` with `LEA`, `FILD`s it as signed, adds 0.5, and uses the same toward-zero conversion. The sole store at `0x0069EBD0` is exactly `g_BridgeZ_Offset = trunc(104*4 + 0.5) = 416`.

There is no clamp or saturation in either final conversion. The integer multiply is performed before `FILD`; overflow would use the wrapped 32-bit result, but the initialized value 104 is far from that boundary. The `+0.5` does not alter the final result here because `4 * 104` is already integral.

### 3.2 Exhaustive direct-xref census

`get_xrefs_to(0x00B0782C)` returns exactly four references:

| Instruction | Function | Access | Role |
|---:|---|---|---|
| `0x0069EBD0` | `ShipLocomotionClass__Compute_BridgeZOffset` | write | one-shot initialization |
| `0x0069F4F2` | `FUN_0069F450` / verified Ship `Set_Destination` | read | add 416 to stored immediate destination Z on a structural bridge cell |
| `0x006A06B7` | `ShipLocomotionClass__Process_Drive_Track` | read | select a 416-lepton destination Z component for 3D braking distance |
| `0x006A0F58` | `ShipLocomotionClass__Process_Drive_Track` | read | `IsTrain`-gated ground/deck list selection |

There are no render, save, load, theater, map-load, INI-read, damage, weapon, or non-Ship xrefs to this global.

### 3.3 `Set_Destination @ 0x0069F450`

The function short-circuits through the four owner guards in the exact order listed in Section 2. If all return zero, it stores all three incoming signed 32-bit coordinates before testing the bridge condition.

The remaining order is exact:

1. Compare the full X/Y/Z triple with the Ship NullCoord globals at `0x00B077F8/FC/800`.
2. If **all three** equal NullCoord, return without a map lookup or bridge adjustment.
3. Otherwise resolve the cell from the original coordinate triple.
4. Test only `Cell+0x140 & 0x100`. There is no `Cell.Level`, `OnBridge`, `Naval`, `MovementZone`, `TooBigToFitUnderBridge`, or elevation-height predicate here.
5. If set, add the signed 32-bit global to the just-stored immediate-destination Z at complete-object `+0x3C`. There is no clamp.

The bump is thus bound by the Ship vtable, not by an INI `MovementZone=Water` check. Its stored result is observable through the Ship immediate-destination getter and through later coordinate comparisons even though the braking block independently recomputes `ground_height + offset`.

### 3.4 Active braking-distance read at `0x006A06B7`

`ShipLocomotionClass__Process @ 0x0069FC10` calls `Process_Drive_Track @ 0x006A05F0` on the active track path and again after movement where the native control flow requires it. `get_function_callers(0x006A05F0)` returns this owner.

The bridge-aware distance block is reached only when:

- `TechnoType+0xDBD` (`Accelerates`) is nonzero. If false, the function directly applies the stored target speed fraction and skips all distance math.
- The owner is not a `UnitClass` whose `UnitType+0xE0C` (`Passive`) is set. The raw gate is owner virtual `+0x2C == 1` followed by `owner+0x6C4 -> type+0xE0C`; `Passive=true` suppresses this block.
- the signed track index at Ship locomotor `+0x58` is `< 0x40`.

All 13 active `rulesmd.ini` Ship-CLSID sections use `Accelerates=true` explicitly or by the constructor default at `0x00711651`, and neither extracted retail INIs nor the selected retail map archives contain an active `Passive=` assignment. The block is therefore active in ordinary stock naval movement.

Inside the block the operation order and units are:

1. Copy the stored immediate destination X/Y/Z from Ship-locomotor complete-object `+0x34/+0x38/+0x3C`.
2. Resolve the **destination cell**, not the owner's current cell (`0x006A068A..0x006A06B1`).
3. Load `g_BridgeZ_Offset` at `0x006A06B7` even for a non-bridge destination, then branchlessly form `z_offset = (Cell.Flags & 0x100) ? 416 : 0` with `AND 0x100; NEG; SBB; AND`.
4. Call `CellClass__GetGroundHeight(destination_coord) @ 0x00578080` and form `target_z = returned_ground_z + z_offset` as a signed dword.
5. Form signed dword deltas `dx = owner.X - destination.X`, `dy = owner.Y - destination.Y`, and `dz = owner.Z - target_z`.
6. Convert each delta to x87, compute `dx*dx + dy*dy + dz*dz`, call `Sqrt_Approx @ 0x004CAC40`, then `Math__ftol @ 0x007C5F00` (toward zero).
7. Compare the resulting signed integer distance to signed `TechnoType+0x2F8` with strict `JL` / `<` at `0x006A0777..0x006A077D`. Equality does **not** start braking.

For a flat-water ship whose current Z equals destination ground Z, the structural-cell distance is therefore `trunc(Sqrt_Approx(planar_squared + 416^2))`, not `planar_distance + 416`. A 100-lepton planar separation produces lookup result approximately 427.8504 and truncates to **427**. Against stock slowdown distance 500, native enters braking (`427 < 500`); the current Rust expression produces 516 and does not. This is an exact high-value discriminator.

The read is about approaching a stored destination that lies in a structural bridge cell. Merely passing through a bridge while the stored destination lies beyond it does not activate this Z term. Conversely, the term can be active before the mover itself enters the bridge cell.

### 3.5 `IsTrain`-gated list reader at `0x006A0F58`

After track-step coordinate application and the nearby `OnBridge` update, this block runs only if all of the following hold:

- owner type byte `TechnoType+0xC94` is nonzero;
- owner byte `+0x6D0` is zero.

If `OnBridge` at owner `+0x8C` is true, the block selects `Cell+0xE8`. If `OnBridge` is false, it obtains the owner's current coordinate and cell ground height, adds `g_BridgeZ_Offset`, and makes a **signed** `JL` comparison: current Z `< ground+416` selects `Cell+0xE4`; current Z `>= ground+416` selects `Cell+0xE8`. It then iterates that selected list and enters the collision/crush/damage sequence.

The field is conclusively `IsTrain`: string `IsTrain @ 0x008444BC` has its sole xref at `TechnoTypeClass__ReadINI 0x00712277`; the parser reads the previous/default byte at `+0xC94`, calls the bool reader, and stores AL back at `0x00712284`. The constructor writes zero at `0x007113B3`. No active `IsTrain=` occurs in the extracted retail INI corpus, the selected retail map archives, or loose retail maps. Therefore this direct reader has **no stock retail naval activation** and must not be used to justify a stock `TooBigToFitUnderBridge` implementation.

`TooBigToFitUnderBridge` is a different UnitType byte at `+0xE16`, read from string `0x00845DC8` by `UnitTypeClass__ReadINI @ 0x0074774E`. Several stock ships set it, but it does not gate this global's `0x006A0F58` reader.

### 3.6 Cell level, `OnBridge`, and under-versus-on relationship

The constant relationship is exact: the structural high-bridge level delta used by the Ship/Drive transition is four signed `Cell.Level` units, while the Ship projected step is 104 leptons, so this global is `4 * 104 = 416` leptons. The transition code sign-extends `Cell+0x11B`; it sets `OnBridge` only when the destination signed level equals source signed level minus four **and** the destination has structural bit `0x100`. It clears `OnBridge` when leaving structural cells.

An ordinary stock ship crossing flat water under an intact bridge sees equal water-ground levels. The `new_level == old_level - 4` entry predicate is false, while the structural destination prevents the leave predicate, so `OnBridge` remains false and the ship remains on the ground/water object list. The 416-lepton braking target is therefore an emergent representation of the deck above the water-surface ship; it does not move the ship onto the deck and does not write `OnBridge`.

No direct reader checks that the deck is actually elevated above ground. The exact gate is structural bit `0x100`. Low-bridge tube cells and generic `bridge_walkable` bridgeheads without that bit are excluded from this global's ordinary readers.

### 3.7 Lifetime, save/load, pause, and replay

- **Cold image:** zero, as expected for BSS.
- **Boot:** initialization-table entry `0x00814A68` runs after the height-step entry at `0x00814A5C` and writes 416 once.
- **Gameplay:** the global is immutable. The exhaustive xref census contains no later writer.
- **Save/load:** the global is not part of any save or load stream. Loading in the same process reuses the already initialized value; loading from a fresh process receives the same value before object deserialization. No snapshot field or swizzle registration is appropriate.
- **Pause:** pausing prevents normal locomotor tick execution; it does not alter the process-global.
- **Replay/determinism:** there is no RNG, time, theater, scenario, or map input to the value. Replays use the same immutable 416 and the same destination/cell predicates.

## 4. INI Keys and Retail Variant Set

`rulesmd.ini` is the YR authority for this class binding; the trailing alternate locomotor GUIDs on several lines are comments after `;` and are not active fallback values.

| Key / source | Type / default | Effect on this slice | Retail result |
|---|---|---|---|
| `Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` | CLSID | binds the Unit to the verified Ship vtable and all three readers | 13 active YR sections |
| `Accelerates=` / TechnoType `+0xDBD` | bool, default true at `0x00711651` | false bypasses the braking-distance read entirely | all 13 are true explicitly or by default |
| `Passive=` / UnitType `+0xE0C` | bool, default false | true suppresses the braking-distance branch for UnitClass | no active extracted-retail or selected-map assignment |
| `IsTrain=` / TechnoType `+0xC94` | bool, default false at `0x007113B3` | enables the second `Process_Drive_Track` read and object-list iteration | no active extracted-retail or selected-map assignment |
| `TooBigToFitUnderBridge=` / UnitType `+0xE16` | bool | unrelated to this global despite older docs | present on multiple ships, but no effect here |
| `Naval=yes`, `MovementZone=Water`, `SpeedType=Float` | per-type data | describes all stock Ship-bound sections; does not gate native global reads | all 13 sections |

| Section | `rulesmd.ini` section line | TechLevel | Stock availability | Notes for activation |
|---|---:|---:|---|---|
| `DEST` | 7075 | 4 | buildable | Ship, Water/Float, Accelerates default true |
| `DLPH` | 7130 | 5 | buildable | explicitly `Accelerates=true` |
| `AEGIS` | 7186 | 7 | buildable | Accelerates default true |
| `CARRIER` | 7249 | 7 | buildable | Accelerates default true |
| `HYD` | 7953 | 6 | buildable | Accelerates default true |
| `SUB` | 8001 | 2 | buildable | explicitly `Accelerates=true` |
| `SQD` | 8051 | 9 | buildable | Accelerates default true |
| `DRED` | 8119 | 6 | buildable | Accelerates default true |
| `BSUB` | 8936 | 2 | buildable | explicitly `Accelerates=true` |
| `VLAD` | 9120 | -1 | scenario/unbuildable | class path remains active when scenario-created |
| `CRUISE` | 10341 | -1 | scenario/unbuildable | class path remains active when scenario-created |
| `TUG` | 10395 | -1 | scenario/unbuildable | class path remains active when scenario-created |
| `CDEST` | 10448 | -1 | scenario/unbuildable | class path remains active when scenario-created |

Retail map override check: a raw literal scan found no active `IsTrain=` in `mapsmd03.mix`, `multimd.mix`, `expandmd01.mix`, `ra2md.mix`, `MAPS01.MIX`, `MAPS02.MIX`, `MULTI.MIX`, `ra2.mix`, or the loose `.map/.mpr/.yrm/.mmx/.yro` files in the configured retail root. The same bounded archive set contains no anchored `Passive=` assignment; `MultiplayPassive=` hits are a different key.

## 5. Integration Points

| Order | Owner / function | Exact integration |
|---:|---|---|
| 1 | process static initializer table | projection constants and lookup-derived diagonal are initialized, then `g_ShipHeightStep=104`, then `g_BridgeZ_Offset=416` |
| 2 | Ship ILocomotion slot 17, `0x0069F450` | accepts an immediate destination; owner-state guards run before any coordinate write; structural destination bit bumps stored Z |
| 3 | `ShipLocomotionClass__Process @ 0x0069FC10` | active Ship tick dispatches `Process_Drive_Track` |
| 4 | `Process_Drive_Track @ 0x006A05F0`, early speed branch | for Accelerates/non-Passive/track-index conditions, recomputes destination ground+conditional 416 and evaluates strict 3D slowdown distance |
| 5 | same function, after track coordinate and `OnBridge` transition | optional `IsTrain` branch selects the ground/water or bridge/deck object list; stock retail never enables it |

The global has no direct presentation integration. Deck rendering, voxel Z-buffer ordering, `ZFudgeBridge`, and Foot's on-deck height snap are separate consumers of other state/constants. Numerically equal 416 constants may represent the same physical clearance without sharing this global or its owner.

## 6. Current Rust Implementation Status

### Matches

- `src/sim/movement/movement_bridge.rs:136-154` defines `BRIDGE_Z_OFFSET` as `SimFixed::lit("416")`. The magnitude matches the exact active-binary derivation.
- `src/sim/map/bridge_topology.rs` supplies the corresponding four-level/416-lepton bridge-deck relationship, and `movement_bridge.rs:842-846` pins the equality.
- Rust's water-layer traversal and existing integration tests already preserve the ordinary under-bridge relationship (`on_bridge=false`, water/ground occupancy) for stock naval movers.
- No mutable snapshot field exists for this value, which matches its immutable process-global lifetime.

### Mismatches and gaps

1. **Wrong arithmetic (milestone-blocking):** `movement_tick.rs:2002-2009` computes 2D integer distance and then adds 416. Native puts the 416 in `dz`, sums three squares, calls its square-root lookup, and truncates. The existing `gsi_04_03b_water_mover_bridge_clearance_crosses_braking_boundary` test encodes the wrong native expectation: at planar 100 and slowdown 500, it expects 516/no braking, while native returns 427/braking.
2. **Wrong cell (milestone-blocking):** Rust tests `path_grid.cell(entity.position.rx, entity.position.ry)`. Native tests the Ship locomotor's stored immediate **destination** coordinate at `+0x34/+0x38/+0x3C`. A ship approaching a bridge destination must receive the Z term before its current cell is structural; a ship merely passing under a bridge toward a non-bridge destination must not receive it.
3. **Wrong flag projection (compounding):** Rust uses `bridge_deck_level_if_any().is_some()`, which is based on `PathCell.bridge_walkable`. Native tests the structural `Cell.Flags & 0x100` bit. `PathCell::has_structural_bridge()` is the matching existing Rust projection; walkable but nonstructural bridgeheads must not activate this term.
4. **Wrong owner gate (exactification/architecture):** Rust gates by `movement_zone.is_water_mover()`. Native ownership is the Ship ILocomotion vtable. All stock Ship types happen to be Water movers, so the positive stock set overlaps, but water-zone identity is not the native rule.
5. **Duplicate non-Ship use:** `movement_tick.rs:2142-2147` applies the same constant inside the generic accel/decel fallback for any water mover. This branch is unreachable for an entity recognized as Ship because the earlier Ship branch wins; it is not a port of any direct read in this slice and should not retain Ship-global semantics for another locomotor.
6. **Missing setter-equivalent evidence:** Rust stores a 2D `final_goal`/path endpoint rather than a bumped immediate-destination Z. That is acceptable only if the active Ship braking calculation reconstructs the exact target Z from the stored destination cell at the same point. Do not add mutable global or serialized destination-height state merely to mirror native storage.
7. **Stale comments/provenance:** `movement_bridge.rs` documents the Drive and Foot producers but not the Ship initializer and falsely summarizes the active effect as adding the value to braking distance "when a ship passes under a bridge cell." It is destination-cell 3D geometry.
8. **Stock-inactive branch:** Rust does not need an `IsTrain` bridge/crush branch for stock-retail GSI-04.03 closure. Treating `TooBigToFitUnderBridge` as that gate would add wrong behavior to common stock ships.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| cold/static state | verified | reads at `0x00B0782C/38`, `0x00B077C0/E0/E8` | none |
| initialization-table order | verified | pointer table `0x00814A40..0x00814A6B`; DATA xref `0x00814A68` | none |
| cell diagonal producer | verified | `0x0069EA40`; sqrt LUT byte `0x0086D0BC` | none |
| angle producers | verified | `0x0069EAD0`, `0x0069EAF0`; literals `0x007F1300`, `0x007E1728/30` | none |
| tangent lookup identity/index/value | verified | `0x004CAD50`; table bytes `0x0085D5F8` | none |
| conversion mode | verified | `Math__ftol @ 0x007C5F00`; control word `0x00822D80 = 0x0E7F` | none |
| `g_ShipHeightStep=104` | verified | `0x0069EB10..0x0069EB3B`; exact lookup operands | none |
| `g_BridgeZ_Offset=416` | verified | `0x0069EBB0..0x0069EBD0` | none |
| complete global xref census | verified | `get_xrefs_to 0x00B0782C` = three reads, one write | none |
| Ship setter vtable binding | verified | vtable `0x007F2D8C`, COL `0x008093A0`, TypeDescriptor `0x0083F880`, slot `0x007F2DD0` | none |
| setter guard order and bodies | verified | `0x0069F450`; Unit vtable/COL and `0x00746C90`, `0x004DE770`, `0x0070C5B0/C0` | none |
| setter NullCoord and structural-bit branches | verified | `0x0069F4B2..0x0069F4FD` | none |
| Process owner/call activity | verified | caller `0x0069FC10`; Ship vtable/type plus stock CLSID | none |
| Accelerates/Passive/track-index gates | verified | `0x006A0644..0x006A0684`; retail data scans | none |
| destination-cell branchless offset select | verified | `0x006A068A..0x006A06D1` | none |
| exact signed 3D arithmetic and truncation | verified | `0x006A06D3..0x006A0755` | none |
| strict slowdown comparison | verified | `0x006A076F..0x006A077D` | none |
| second reader and list-selection ordering | verified | `0x006A0EE2..0x006A0F8E` | none |
| `+0xC94 = IsTrain` and stock default/activity | verified | `0x007113B3`, `0x00712270..84`, string `0x008444BC`, retail scans | none |
| `TooBigToFitUnderBridge` exclusion | verified | `0x00747747..78`, string `0x00845DC8`; no global xref uses `+0xE16` | none |
| stock Ship variant set | verified | parsed active `rulesmd.ini` sections and values | none |
| selected stock map override set | verified | literal scans of configured retail map archives and loose maps | none within the stated retail corpus |
| under-versus-on bridge relation | verified | live `Process_Drive_Track` transition plus `BRIDGE_TRAVERSAL_STATE_GHIDRA_REPORT.md` | none |
| save/load/global lifetime | verified | complete xref census; init-table writer | none |
| current Rust consumers and tests | verified | `rg BRIDGE_Z_OFFSET`; `movement_tick.rs`, `movement_bridge.rs`, `pathfinding/core.rs` | none |
| visual composition | deferred | non-visual mechanism; no render xref | no visual ledger required |

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] OQ-01 — What are every direct reader and writer of 0x00B0782C? -> Exactly three reads and one write, enumerated in Section 3.2.` (evidence: `get_xrefs_to 0x00B0782C`)
- `[RESOLVED] OQ-02 — Is the cold zero the runtime value? -> No; it is BSS, replaced by the one-shot initializer.` (evidence: `0x00814A68 -> 0x0069EBB0`, store `0x0069EBD0`)
- `[RESOLVED] OQ-03 — Can the exact runtime value be established without a debugger? -> Yes; fixed init order, fixed lookup bytes, and fixed x87 conversion prove height step 104 and bridge offset 416.` (evidence: `0x0069EA40`, `0x0069EAD0`, `0x0069EAF0`, `0x0069EB10`, `0x0069EBB0`)
- `[RESOLVED] OQ-04 — Is helper 0x004CAD50 sine or tangent? -> Tangent; its 4096-angle table returns the pi/6 tangent entry.` (evidence: `0x004CAD50`; table `0x0085D0A4`, entry `0x0085D5F8`)
- `[RESOLVED] OQ-05 — What is the exact rounding rule? -> x87 signed conversion toward zero under control word 0x0E7F; the final +0.5 is retained but immaterial for integer 416.` (evidence: `0x007C5F00`, `0x00822D80`, `0x0069EBC1..CB`)
- `[RESOLVED] OQ-06 — Is 0x0069F450 truly the active Ship setter? -> Yes, Ship RTTI/COL and ILocomotion slot 17 bind it.` (evidence: `0x007F2D88 -> 0x008093A0 -> 0x0083F880`, slot `0x007F2DD0`)
- `[RESOLVED] OQ-07 — What suppresses the setter? -> UnitClass IsCrashing, active rearm timer, IsWarpingOut, and IsBeingWarped, in that exact order.` (evidence: `0x0069F450`; Unit COL/vtable and four resolved bodies)
- `[RESOLVED] OQ-08 — Does a partial NullCoord triple suppress adjustment? -> No; only exact equality of all X, Y, and Z skips lookup. Any differing component resolves the cell.` (evidence: `0x0069F4B2..0x0069F4CC`)
- `[RESOLVED] OQ-09 — Does the ordinary reader inspect current or destination cell? -> Destination cell copied from Ship-locomotor +0x34/+0x38/+0x3C.` (evidence: `0x006A068A..0x006A06B1`)
- `[RESOLVED] OQ-10 — Is native distance planar-plus-offset? -> No; offset forms target Z before signed dx/dy/dz square-sum, Sqrt_Approx, and truncation.` (evidence: `0x006A06D3..0x006A0755`)
- `[RESOLVED] OQ-11 — Is the slowdown boundary inclusive? -> No; signed strict distance < SlowdownDistance starts the branch.` (evidence: `0x006A0777..0x006A077D`)
- `[RESOLVED] OQ-12 — Which stock types reach the ordinary reader? -> All 13 active Ship-CLSID sections can; all are Accelerates=true and none is Passive.` (evidence: `rulesmd.ini` parsed table; ctor `0x00711651`; retail scans)
- `[RESOLVED] OQ-13 — What gates the second reader? -> TechnoType+0xC94 IsTrain, owner+0x6D0 clear; then OnBridge/current-Z selects E8 versus E4.` (evidence: `0x006A0EE2..0x006A0F8E`, parser `0x00712270..84`)
- `[RESOLVED] OQ-14 — Can stock retail ships reach the second reader? -> No inspected stock type/map sets IsTrain and its constructor default is false.` (evidence: `0x007113B3`; extracted retail INI, selected map-archive, and loose-map scans)
- `[RESOLVED] OQ-15 — Is +0xC94 TooBigToFitUnderBridge? -> No; TooBigToFitUnderBridge is UnitType+0xE16, while +0xC94 is IsTrain.` (evidence: strings/xrefs `0x008444BC -> 0x00712277` and `0x00845DC8 -> 0x0074774E`)
- `[RESOLVED] OQ-16 — Does this global directly render or z-sort ships? -> No; the complete xref set is confined to Ship initialization/setter/track processing.` (evidence: global xref census)
- `[RESOLVED] OQ-17 — How does under-water movement relate to OnBridge? -> Equal flat-water levels fail the signed -4 entry predicate; OnBridge remains false while structural destination still activates bridge-Z geometry.` (evidence: live `0x006A05F0` transition; `BRIDGE_TRAVERSAL_STATE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-18 — Is the global saved, loaded, or reset per map? -> No; one process-init writer, no serializer/map-load writer, immutable across save/load.` (evidence: global xref census; init table)
- `[RESOLVED] OQ-19 — Does pause/replay change the value? -> No; no time/RNG/pause/map input exists, and locomotor processing simply does not advance while paused.` (evidence: init chain and exhaustive xrefs)
- `[RESOLVED] OQ-20 — Does current Rust match? -> Magnitude and immutable representation match; arithmetic, source cell, structural predicate, owner gate, test expectation, and one generic duplicate consumer do not.` (evidence: `movement_bridge.rs:136-154,365-378,842-846`; `movement_tick.rs:1994-2052,2127-2149`; `pathfinding/core.rs:1761-1766`)

### Adversarial corner-case answers

1. **Ship is already under the bridge but destination is beyond it:** no native Z term, because the destination cell, not current cell, supplies bit `0x100`.
2. **Ship is outside the bridge but is ordered to a water cell under it:** native Z term is already active during approach; at planar 100 the distance truncates to 427, not 516.
3. **Destination is a walkable nonstructural bridgehead:** no native term; bit `0x100` is clear even though Rust `bridge_walkable` can be true.
4. **Destination triple is exact NullCoord:** setter performs no map lookup and no add; a partial sentinel still performs both.
5. **A mod sets `IsTrain=true`:** the later reader becomes active and can select the deck list even while `OnBridge=false` when current Z is at least `ground+416`; this is deliberately excluded from stock-row implementation.
6. **Save is loaded by a new executable process:** the process initializes 416 before loading the object graph; no save payload field is required.

The zero-add pass re-read `0x0069EBB0`, `0x0069F450`, both `0x006A05F0` reader regions, the complete global xref list, and the Rust consumers after the questions above were drained; it added no new material question.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ship height step is 104 and Ship bridge offset is immutable 416 | init table `0x00814A44..68`; `0x0069EA40`, `0x0069EB10`, `0x0069EBB0`; lookup bytes | magnitude matches | `src/sim/movement/movement_bridge.rs::BRIDGE_Z_OFFSET` and constant tests | preserve 416 as an immutable derived/fixed retail constant; update provenance to the Ship producer | exact constant test proves 104-to-416 derivation contract and equality with deck clearance | Do not restore stale 360 or add serialized/runtime-mutable global state |
| structural **destination** cell supplies target Z `ground+416` | `0x006A068A..0x006A06E5` | mismatch: current entity cell and `bridge_walkable` | `src/sim/movement/movement_tick.rs` active Ship speed-ramp distance path; `PathCell::has_structural_bridge` | use the Ship movement destination coordinate and exact structural projection; nonstructural bridgeheads yield zero | entity outside bridge, destination structural: adjustment active; entity on bridge, destination nonstructural: adjustment absent; walkable nonstructural bridgehead: absent | Do not key on current cell, `bridge_deck_level_if_any`, or water movement zone alone |
| bridge clearance is the signed Z component of truncated 3D distance | `0x006A06D3..0x006A0755`; strict compare `0x006A0777..7D`; lookup spot-check | mismatch: planar distance plus 416 | `distance_to_goal_leptons` call site used by Ship ramp, or a Ship-specific existing distance surface | reproduce deterministic `sqrt(dx²+dy²+dz²)`/toward-zero result before the strict slowdown comparison | planar 100, ground/current Z equal, structural destination, slowdown 500 -> distance 427 and braking true; nonstructural -> distance 100 | Do not add 416 after square root. Preserve strict `<`, not `<=` |
| Set_Destination stores the incoming triple before exact NullCoord/structural adjustment | `0x0069F450..0x0069F502`; Ship vtable proof | Rust has 2D goal state and no direct bumped-Z field | existing `MovementTarget.final_goal` / Ship movement-target handoff | no new persisted Z field is required if the active Ship calculation reconstructs native target Z from the same destination at the same ordering point | exact NullCoord-equivalent/cancel path does not query bridge or retain a bump; retarget structural-to-nonstructural drops the term immediately | Do not let an earlier target's 416 leak across retarget/cancel or serialize derived target Z unnecessarily |
| native ownership is Ship ILocomotion plus Accelerates/non-Passive/track gate | `0x007F2D8C` RTTI/slot proof; `0x006A0644..0x006A0684`; retail values | mismatch: generic water-mover gate and duplicate generic fallback | `movement_tick.rs:1994-2052` and `2127-2149` | keep the effect on the recognized Ship ramp path; remove Ship-global semantics from generic non-Ship fallback | Water-zone entity with non-Ship locomotor does not receive this adjustment; each listed stock Ship type can | Do not generalize from `MovementZone=Water`; class ownership is the native boundary |
| second reader is IsTrain-gated and stock-inactive | `0x006A0EE2..0x006A0F8E`; `0x007113B3`; `0x00712270..84`; retail scans | no stock mechanism required | no GSI-04.03 Rust behavior surface; document exclusion | leave this branch excluded from stock closure; record as mod-support residual only if mod parity scope expands | every stock Ship type has effective IsTrain=false, so no E4/E8 selection/damage iteration from this reader | Do not substitute parsed `TooBigToFitUnderBridge`; that is UnitType+0xE16 and would activate common stock ships incorrectly |
| ships under flat high bridges remain OnBridge=false while destination structural geometry can use 416 | transition inside `0x006A05F0`; signed Cell.Level relation; bridge traversal report | existing traversal tests broadly match | movement bridge transition and under-bridge integration tests | preserve water/ground occupancy and Z while correcting only the braking geometry | flat-water ship enters intact structural bridge cell: owner Z remains water height and OnBridge false; destination-based distance uses 416 only if that cell is the stored destination | Do not move the ship to deck height or infer a render-depth write from this global |

### Stale Docs / Follow-up Docs

- `docs/research/BRIDGE_BSS_RUNTIME_CONSTANT_SWEEP_GHIDRA_REPORT.md`: replace “exact Ship values require a live post-map debugger capture / Rust 360 remains unproven” with: **“The fixed static initializer and lookup bytes prove `g_ShipHeightStep=104` and `g_BridgeZ_Offset=416`; a debugger is unnecessary.”**
- `docs/research/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`: replace the `0x004CAD50` “Sin lookup” wording with **“4096-entry tangent lookup”**; replace guessed Set_Destination guard names with the four resolved UnitClass bodies in Section 2.
- `docs/research/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` and `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`: replace **“TechnoType+0xC94 TooBigToFitUnderBridge”** with **“TechnoType+0xC94 IsTrain; stock default false/no active retail assignments. TooBigToFitUnderBridge is UnitType+0xE16 and does not gate this reader.”**
- `docs/research/ZBUFFER_DEPTH_SYSTEM.md`: replace the broad render wording with: **“The Ship global has no direct render xref. Its stock-active readers alter immediate destination Z and destination-based 3D braking distance; an additional IsTrain-gated list selector is stock-inactive.”**
- `docs/research/ADDRESS_MAP.md`: replace initializer address `0x0069EBD0` with **function entry `0x0069EBB0` (store instruction `0x0069EBD0`)**, and record exact value 416.
- `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`: its “round-half-up” label is harmless for the positive integer result but incomplete. Exact wording is **“form signed int32 `4*height_step`, add 0.5 in x87, convert toward zero under control word 0x0E7F; result 416.”**
- `src/sim/movement/movement_bridge.rs`: replace the current “added to braking distance when a ship passes under a bridge cell” provenance with destination-structural 3D geometry and the Ship addresses in this report.

## 11. Ghidra Annotation Candidates

Read-only worker: no annotation was applied.

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x0069F450` | `FUN_0069f450`, undefined prototype | `ShipLocomotionClass__Set_Destination`; ILocomotion receiver plus Coord3D value args; comment exact guard and structural-Z order | rename/prototype/comment | Ship COL/TypeDescriptor, ILocomotion slot 17, body | worker-report-only |
| `0x00B0782C` | `g_BridgeZ_Offset`, `undefined4`, no plate comment | signed int32 `g_nShipBridgeZOffsetLeptons`; plate: boot-derived 416, three readers/one writer, not serialized/rendered | type/rename/comment | exact init chain and complete xref census | worker-report-only |
| `0x0069EBB0` | correctly named function, weak prototype | add comment: `trunc_toward_zero((int32)(g_ShipHeightStep*4)+0.5)`, sole store at `0x0069EBD0`, fixed result 416 | comment/prototype | full assembly and x87 control word | worker-report-only |

## Sources

- Live Ghidra MCP, active `gamemd.exe`: `0x004CAC40`, `0x004CAD50`, `0x0069EA40`, `0x0069EAD0`, `0x0069EAF0`, `0x0069EB10`, `0x0069EBB0`, `0x0069F450`, `0x0069FC10`, `0x006A05F0`, `0x0070C5B0`, `0x0070C5C0`, `0x007113B3`, `0x00711651`, `0x00712270`, `0x00746C90`, `0x0074774E`, `0x007C5F00`.
- Live static data/table reads: `0x007E1708`, `0x007E1728`, `0x007E1738`, `0x007E8970`, `0x007F1300`, `0x00814A40`, `0x00822D80`, `0x0085D5F8`, `0x0086D0BC`, `0x00870380`; Ship and Unit RTTI/vtables described in Section 2.
- Retail rules: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`; cross-check of extracted retail INIs under `ini/`.
- Retail maps: configured retail root `C:/Users/enok/Documents/Command and Conquer Red Alert II`; bounded archive and loose-map scans listed in Section 4.
- Current Rust: `src/sim/movement/movement_bridge.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/pathfinding/core.rs`, `src/sim/map/bridge_topology.rs`, relevant movement/world bridge tests.
- Prior-doc audit: `docs/research/ADDRESS_MAP.md`, `docs/research/ZBUFFER_DEPTH_SYSTEM.md`, `docs/research/BRIDGE_TRAVERSAL_STATE_GHIDRA_REPORT.md`, `docs/research/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, `docs/research/BRIDGE_BSS_RUNTIME_CONSTANT_SWEEP_GHIDRA_REPORT.md`, `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`, `docs/research/bridges/00-system-models/BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md`, `docs/research/UNIT_0X6C8_CONVOY_LINK_LIFECYCLE_RESWARM_20260528.md`.
