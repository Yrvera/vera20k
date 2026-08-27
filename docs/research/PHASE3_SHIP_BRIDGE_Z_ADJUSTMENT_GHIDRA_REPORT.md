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

## 12. Design-Review Repair Addendum (2026-08-27)

This addendum resolves the first design critic's load-bearing questions. It does not claim implementation or parity `PASS`; the GSI row remains open until implementation, validation, fresh criticism, and the phase-wide reverse audit complete.

### 12.1 Exact coordinate-to-cell semantics and destination authority

Live `MapClass__Get_CellClass_At_Coord @ 0x00565730` converts each signed coordinate component as:

```text
biased = coord + ((coord >> 31) & 0xFF)
cell   = biased >> 8
```

This is signed division by 256 truncated toward zero. It is not Euclidean/floor division: `-1..=-255` map to cell 0, `-256` maps to -1, `255` maps to 0, and `256` maps to 1. It then forms `cell_y * 0x200 + cell_x`; an invalid linear index/capacity/pointer returns the shared dummy cell.

The ordinary Ship reader copies the immediate destination triple from the Ship locomotor before this lookup. Therefore current Rust's exact authority for the structural destination cell is `ShipLocomotionRuntime.destination.x/y`, not `MovementTarget.final_goal` or `path.last()`. `final_goal`, then `path.last()`, may retain the existing neutral 2D fallback only when no active Ship destination exists. If an active destination exists but maps outside the valid Rust `PathGrid`, the defensive result is also neutral 2D; a stale final goal must not substitute a different structural cell.

### 12.2 Exact native 3D distance helper and discriminator

The Ship caller uses `CoordStruct__Distance3D @ 0x0041C380`. Current Rust already ports that routine as `util::native_x87::distance_3d_leptons` (`src/util/native_x87.rs`): wrapping signed dword deltas, x87-style square/sum, f32 `Sqrt_Approx`, and `Math__ftol` truncation. An exact integer square root is not equivalent.

The decisive regression is:

```text
distance_3d_leptons([0, 0, 0], [129, 0, 0]) == 128
SlowdownDistance = 129
128 < 129 -> brake
```

An exact integer square root would return 129 and fail the native strict comparison. The structural example remains planar 100 plus signed Z 416 -> native 3D result 427; with slowdown 500, `427 < 500` brakes.

### 12.3 Exact current/destination world Z reconstruction

The current representation's canonical object-Z precedence is demonstrated by `src/sim/combat/mod.rs::object_world_z_leptons`:

1. `Position.exact_z_leptons`, when present, is already absolute and wins.
2. Otherwise exact world X/Y are `rx * 256 + sub_x.to_num::<i32>()` and `ry * 256 + sub_y.to_num::<i32>()`.
3. `ground_height_leptons(cell.level, cell.slope_type, world_x, world_y)` supplies the signed sloped terrain surface. The helper sign-extends `level as i8`; for `Level=0xFF`, slope 0, the native-compatible result is -103 leptons because the signed `/ 256` conversion truncates toward zero.
4. Add the 416-lepton deck height only when the existing owner `on_bridge` authority is true. A flat-water Ship under a high bridge remains `on_bridge=false` at the water/ground Z.

`Position.z * 104` is only the existing mapless fallback and treats the byte as unsigned. It cannot be the exact valid-gameplay authority for signed levels, slopes, or layer state. The destination's ground Z is reconstructed at its exact stored X/Y from the destination `PathCell.ground_level` and `slope_type`; add 416 only for `PathCell::has_structural_bridge()`. The stored destination Z is not the braking target-Z authority because the native reader recomputes ground Z and the structural offset.

### 12.4 Production map-authority availability and defensive contract

Valid production gameplay publishes both required projections:

- `Scenario::post_map` calls `Simulation::rebuild_dynamic_navigation`; that routine requires `resolved_terrain` and constructs/publishes `PathGrid::from_resolved_terrain_with_bridges`.
- `Simulation::advance_app_frame` pins `path_grid_snapshot()` and passes it through the frame/movement pipeline while `resolved_terrain` remains simulation-owned.
- Headless launch rejects `!navigation_published` and `sim.path_grid().is_none()`.
- Snapshot map-authority restoration rejects missing `ResolvedTerrainGrid` and rebuild failure.

The app loader logs rather than panics on an initial publication failure, and test-only movement wrappers can pass absent grids. Accordingly, missing terrain, missing `PathGrid`, or an out-of-grid active destination is a VERA-internal defensive state: preserve the existing neutral 2D distance and apply no structural Z term. Tests must pin this behavior. It is not evidence that stock gameplay lacks destination/path-grid authority.

### 12.5 Complete `Set_Destination` guard mapping

`ShipLocomotionClass::Set_Destination @ 0x0069F450` checks these owner predicates, in this exact order, before installing the destination:

| Order | Native dispatch/body | Native authority | Current Rust authority | Required bounded delta |
|---:|---|---|---|---|
| 1 | owner vtable `+0x37C` -> `UnitClass__IsCrashing @ 0x00746C90` | `Unit+0x6D8 != -1` or `TechnoClass__IsUnderEMP` | `GameEntity.dying` is the current ordinary stock naval sinking/death command gate | Preserve `dying`. Retail EMP is TS-legacy dormant (`EMPLockRemaining` has no active writer), so no active-stock EMP field is required; mod-enabled EMP remains outside this stock row. |
| 2 | owner vtable `+0x380` -> timer predicate `0x004DE770` | `(current_frame - timer_start@+0x6A0) < timer_duration@+0x6A8`; repository protocol docs identify the ordinary use as `IsInRearmTimer` | `attack_target.cooldown_ticks > 0` is the existing ordinary rearm authority | Add this predicate to movement destination admission. |
| 3 | owner vtable `+0x1D4` -> `TechnoClass__IsWarpingOut @ 0x0070C5B0` | byte `Techno+0x270` | `teleport_state.warp_out_active()`; `TeleportPhase::Relocate` | Add this predicate to movement destination admission. |
| 4 | owner vtable `+0x1D8` -> `TechnoClass__IsBeingWarped @ 0x0070C5C0` | byte `Techno+0x271` | `teleport_state.warp_in_active()`; `ChronoDelay` with `being_warped_ticks > 0` | Add this predicate to movement destination admission. |

`movement_commands::can_accept_destination` already gates `dying` and is called by all three public destination-install paths in that module, but it lacks the existing rearm and teleport predicates. Internal arrival/replay calls consume previously admitted navigation rather than install an unrelated player retarget. The retail EMP corpus has only disabled `[EMPuls]`/`EMEffect=yes` content and no live `EMPLockRemaining` writer, so `IsUnderEMP` is evidence-excluded for active stock. No new state field is required for the four stock mappings.

The old setter-guard prose is stale in several documents. `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` and `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` misname `+0x37C/+0x380` as warp predicates; `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` also misnames `+0x380`; `FOOTCLASS_VTABLE_COMPLETE.md` calls `0x004DE770` `IsInGarrison`. These claims must be corrected to the four-body mapping above.

### 12.6 Exhaustive locomotor `Force_Track` caller result

`ShipLocomotionClass::Force_Track @ 0x006A0310` writes the supplied signed selector. A selector `>= 0x40` bypasses the active braking reader's signed `< 0x40` gate, so every locomotor-shaped call through vtable slot `+0x70` was recounted.

| Call site | Enclosing use | Selector |
|---|---|---:|
| `0x004591AF` | bunker/link installer `0x00458E50` | `0x43..0x46` |
| `0x0045943B` | `BuildingClass::UndockUnit` | `0x47` |
| `0x00459760` | `BuildingClass::ReleaseDockedHarvester` | `0x47` |
| `0x007101B3` | `TechnoClass::PerformDeploy` | `-1` |
| `0x006CCAA2` | `SuperClass::Launch` | `-1` |
| `0x0062AB24` | `FUN_0062A980` | `-1` |

All three positive-selector paths require the reciprocal building/unit pointer at `+0x2E4`. A live instruction write census found the only nonzero stores to that pointer at `0x00459301` and `0x0045930F`, inside the bunker installer; other writers only clear it. Retail `rulesmd.ini` has exactly one `Bunker=yes` type, `[NATBNK]`, a land Tank Bunker. Every stock Ship-CLSID type is a naval/water mover, so no valid stock Ship can acquire this link or a selector `>=0x40`. The similarly named shipyard undock functions do not establish `+0x2E4`; they merely consume a pre-existing link.

Therefore the stock Ship implementation needs no forced-track selector field or bypass gate. A modded/invalid Ship linked to a bunker is an explicit mod-support residual. The three `-1` calls remain signed `<0x40` and do not bypass braking.

The independent `Passive` gate is also stock-zero: the Ship type constructor default is false, all 13 stock Ship-CLSID sections are effectively false, and no inspected retail INI/map override assigns `Passive`. No stock field/gate is required.

### 12.7 Snapshot/hash consequence

No destination-Z cache, global, selector, or guard-state field is added; the existing destination, cooldown, dying, teleport, terrain, and bridge facts are already serialized/hashed where applicable. Payload layout and world-hash membership therefore remain unchanged. Repository compatibility policy still requires a behavior-epoch bump from snapshot version 112 to 113 and rejection of version 112. Scenario hashes may change because movement behavior changes, but the hash algorithm and membership list do not.

### 12.8 Repaired implementation handoff

- Put the pure Ship-only distance evaluation beside `ship_process_target_speed_fraction` / `update_ship_speed_fraction` in `drive_locomotion.rs`.
- Require `ShipLocomotionRuntime.destination` for structural lookup. Convert its signed X/Y to checked `u16` cells with native truncation-toward-zero semantics; use `PathCell::has_structural_bridge()` and that cell's signed ground/slope facts.
- Derive current world Z from `exact_z_leptons`, else resolved current terrain/slope plus 416 only for existing `on_bridge`; never infer valid-world Z from unsigned `Position.z` alone.
- Feed current/destination signed `[i32; 3]` coordinates to `util::native_x87::distance_3d_leptons`; keep the existing strict `< SlowdownDistance` owner unchanged.
- With no active destination, missing map facts, or an invalid active destination cell, retain deterministic neutral 2D behavior and no structural term. Never borrow a structural cell from `final_goal` while an active destination disagrees.
- Add rearm, warp-out, and warp-in predicates to `movement_commands::can_accept_destination`; preserve its ordinary `dying` gate. Add no new component state.
- Remove both generic `MovementZone=Water`/current-bridge duplicates so non-Ship water movers retain neutral 2D behavior.
- Bump snapshot version to 113, reject version 112, and leave payload/hash membership unchanged.

Required discriminators include destination-versus-current cell, immediate destination versus disagreeing final goal, active destination with `final_goal=None`, structural versus merely walkable, 100/427/500, `[0,0,0]` to `[129,0,0]` returning 128 with slowdown 129, retarget/cancel, all four setter guards, signed `Level=0xFF -> -103`, exact-Z precedence, slopes, `on_bridge`, missing-grid defense, non-Ship water ownership, and unchanged Drive/flat-under-bridge state.

### Addendum sources

- Live Ghidra MCP, active `gamemd.exe`: `MapClass__Get_CellClass_At_Coord @ 0x00565730`, `CoordStruct__Distance3D @ 0x0041C380`, `ShipLocomotionClass::Set_Destination @ 0x0069F450`, `ShipLocomotionClass::Force_Track @ 0x006A0310`, `UnitClass__IsCrashing @ 0x00746C90`, timer predicate `0x004DE770`, `TechnoClass__IsWarpingOut @ 0x0070C5B0`, `TechnoClass__IsBeingWarped @ 0x0070C5C0`, and the six call sites in Section 12.6.
- Retail data: `ini/rulesmd.ini` Ship sections, `[NATBNK]`, and disabled `[EMPuls]`; configured retail INI/map corpus scanned for `Passive` and stock Ship overrides. Dormancy cross-check: `docs/research/combat/systems/emp.md`.
- Current Rust: `src/util/native_x87.rs`, `src/util/lepton.rs`, `src/sim/components.rs`, `src/sim/combat/mod.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/teleport_movement.rs`, `src/sim/pathfinding/core.rs`, `src/sim/scenario_post_map.rs`, `src/sim/world/mod.rs`, `src/sim/snapshot.rs`, and `src/headless_scenario.rs`.

## 13. Design-Critic-2 Repair Addendum (2026-08-27)

This addendum supersedes the Section 12 handoff wherever the two differ. It closes the critic's three bounded evidence gaps: rejection ordering, fixed-point range, and the setter's stored Z. It still does not claim implementation or parity `PASS`.

### 13.1 Rejection is a Ship-only transaction boundary

Live `ShipLocomotionClass::Set_Destination @ 0x0069F450` calls all four owner predicates before its first destination write at `0x0069F489`. A rejected call therefore preserves the old immediate destination exactly. The setter also does not change the owner mission, order, attack target, NavCom, committed head, path, or speed state.

The current Rust placement is not equivalent if the extra predicates are merely added to `movement_commands::can_accept_destination`. That helper applies to every locomotor, and production `Command::Move`, `Command::AttackMove`, and `Command::RepairAtDepot` clear or replace mission/order/attack/dock state before `issue_move_command_with_layered` reaches it. In particular, clearing `attack_target` destroys the only current ordinary-rearm authority, `attack_target.cooldown_ticks`, before a late test can observe it.

The current caller census is:

| Rust entry/caller family | Can reach recognized Ship? | Mutation ordering result |
|---|---|---|
| `issue_move_command` | yes; forwards to layered entry | layered entry must preflight before path or state work |
| `issue_move_command_with_layered` | yes; player Move/AttackMove/repair, persistent orders, pursuit, scatter, production exit, and internal miner callers converge here | preflight at function entry protects direct/AI/internal calls; high-level commands which already mutate require an earlier identical check |
| `set_destination_for_teleporter_entity` | stock active Ship and active Teleport are mutually exclusive, but its fallback can call layered movement | run the same recognized-Ship-only preflight before piggyback/teleport mutation; non-Ship behavior is unchanged |
| `issue_direct_move` | generic entry; blocker scatter does not statically exclude a vehicle Ship | run preflight before its same-cell return and before `MovementTarget`/facing writes; this function is not currently a NavCom/Ship-destination writer |
| `navcom::set_destination_internal_cell` | yes only from the successful layered commit; its other production callers are explicitly Drive-only pending-arrival/queue paths | repeat the pure preflight before NavCom or Ship-runtime writes as defense in depth; return rejection without partial owner mutation |
| `Command::{Move, AttackMove, RepairAtDepot}` | yes without contradictory type predicates | call preflight before mission teardown/queueing and before attack/order/dock clears |
| `EnterTransport`, `PlantC4`, `CaptureBuilding`, `EnterBunker` movement branches | stock Ship excluded by passenger/C4/Engineer/Bunkerable rules | still place the cheap recognized-Ship-only preflight before their first side effect, so invalid/modded class combinations cannot bypass the setter contract |
| persistent AttackMove resume and combat pursuit | yes | they do not clear the attack/order before their layered call; entry preflight is early enough |
| miner/refinery, infantry damage scatter, passenger/garrison, and sell-ejection direct callers | no stock Ship by harvester/infantry/passenger role | entry preflight remains the defensive control; their existing non-Ship ordering is not changed |

The smallest fitting authority is one pure predicate owned beside the Ship setter adapter, callable read-only from `world_commands` and every movement entry. It first recognizes `LocomotorKind::Ship`; non-Ship returns admitted immediately. Only recognized Ship evaluates, in native order, `dying`, active `attack_target.cooldown_ticks`, `teleport_state.warp_out_active()`, and `teleport_state.warp_in_active()`. Existing generic `can_accept_destination` keeps its existing dying/build/unload policy and does not gain Ship rearm/warp semantics.

A command-level rejection must preserve at least mission state and timer, `order_intent`, the full attack target/cooldown/provenance, `movement_target`, owner NavCom/aux/queue, Ship destination/head/path/speed, facing target, dock/C4/capture/passenger/bunker state, and cell occupation. A Walk locomotor with the same positive weapon cooldown is the required control: it remains admitted under the old non-Ship policy and may be retasked normally.

### 13.2 Native distance remains `i32`; conversion is comparison-preserving

Native retains the `Math__ftol` result as a nonnegative signed whole-lepton `i32` and compares it directly with signed integer `SlowdownDistance` using strict `JL`. Rust's `distance_3d_leptons` already returns that `i32`; converting it eagerly with `SimFixed::from_num` is unsafe because `SimFixed = I16F16` represents only `-32768 .. 32767.9999847412109375`.

The exact Rust boundary adapter is:

- keep `distance_3d_leptons`' result as `i32` through the Ship helper;
- for `0..=32767`, convert the integer exactly to `SimFixed`;
- for `32768..=i32::MAX`, use `SimFixed::MAX`;
- do not wrap, panic, use a fallible unchecked conversion, or clamp the slowdown threshold.

This preserves the strict comparison for every nonnegative representable Rust slowdown `s`. When `d <= 32767`, integer conversion is exact, so `fixed(d) < s` is identical. When `d >= 32768`, native `d < s` is false because every representable `s` is less than 32768, and the saturated expression `SimFixed::MAX < s` is also false because `s <= SimFixed::MAX`. Fractional thresholds are covered by the same proof. Equality remains non-braking.

Required range tests pin 32767 as exact, 32768 as `SimFixed::MAX`, both against `SimFixed::MAX`, and a long-map 3D result above 32768. The long-map case must complete deterministically without panic or signed/fixed-point wrap. The 129 -> 128 and 100/427/500 discriminators remain required.

### 13.3 Stored destination Z is active state, not disposable derivation

The live setter stores the full incoming signed triple and then, for any non-null target whose X/Y/Z resolves to a structural cell, adds 416 to the stored Z. It does not replace incoming Z with cell ground. Consequently the caller and setter have separate responsibilities:

- a cell destination caller supplies the exact cell-center coordinate, including the exact signed/slope ground Z at that X/Y; the Ship setter adapter then adds 416 iff the resolved cell has native structural bit `0x100`;
- an entity/object destination caller supplies that target's incoming exact object coordinate Z; the Ship setter preserves it and applies the same structural-cell addition. It must not flatten an entity target to cell ground.

The native stored-coordinate census is:

| Consumer | Address | Stored-Z effect |
|---|---:|---|
| immediate `Destination` getter | `0x0069F3A0` | returns stored X/Y/Z verbatim |
| `Is_Moving` | `0x0069F290` | any non-NullCoord component, including Z, keeps the destination non-null |
| main `Process` | `0x0069FC10` | exact owner/destination arrival comparison reads all three components; later destination refresh calls slot 17 again |
| `Process_Drive_Track` braking | `0x006A05F0`, early block | copies stored X/Y/Z, uses X/Y to find the cell, then deliberately recomputes braking target Z as ground plus conditional 416 |
| `Process_Drive_Track` terminal arrival | `0x006A05F0`, terminal block | resolves NavCom only for target X/Y cell equality, then compares the **owner's current coordinate Z** (owner vtable `+0x4C`) to stored destination Z with `abs(delta) < 2 * g_ShipHeightStep` before clearing it |
| `Stop_Moving` | `0x0069F510` | clears all three components to Ship NullCoord |
| Ship IPersistStream load/save | load `0x0069EE90`, save `0x0069EF10` | common `0x0055AAC0/0x0055AA60` reads/writes the class's raw virtual size (`0x006A42A0` returns `0x70`), so the stored triple is inside persisted class state |

Thus recomputing target Z only for braking does not make the stored Z optional.

Current Rust has one load-bearing mismatch: `navcom::target_cell_coord` is shared by Drive and Ship and stores a coarse level number (`level` or `bridge_deck_level`) in `DriveCoord.z`, not signed world leptons. `DriveCoord` and `ShipLocomotionRuntime` already derive `Serialize`, `Deserialize`, `Eq`, and `Hash`; `snapshot.rs` already round-trips Ship destination, and `world_hash.rs` hashes the whole runtime. `ready_producer` consumes destination presence/X/Y and intentionally ignores Z. Stop/cancel already clears `destination` through `ship_stop_moving`.

The bounded repair must split construction without changing Drive:

1. Keep current Drive `target_cell_coord` semantics byte-for-byte.
2. For a successful Ship cell target, construct X/Y as the existing cell center (`rx * 256 + 128`, `ry * 256 + 128`). From the same `ResolvedTerrainCell`, evaluate `ground_height_leptons(cell.level, cell.slope_type, x, y)`, retaining signed `level as i8` and slope arithmetic; then add immutable 416 only when `cell.bridge_facts.has_structural_bridge()` is true. This is the MapClass-side structural authority; the braking read continues to use the corresponding `PathCell::has_structural_bridge()` projection.
3. If a Ship setter adapter is passed an already-exact entity/object `DriveCoord`, preserve its incoming Z and only add the structural 416. Current production has no Ship entity-target installer: `resolve_entity_nav_target_drive_coord` refreshes only `DriveLocomotionRuntime::head_to`; leave that Drive path unchanged rather than inventing entity pursuit scope. Unit-test the setter adapter's incoming-Z rule so a later native-shaped caller cannot flatten it.
4. A valid production cell install has `ResolvedTerrainGrid`. For the existing test/internal optional-terrain seam, use deterministic incoming/fallback Z 0 and no structural adjustment; do not panic or fabricate a bridge. This is VERA-internal defense, not stock behavior.

Retarget must replace the entire stored triple only after admission; a rejected retarget leaves the previous triple and every owner field unchanged. Null/cancel clears it. Snapshot round-trip must retain a nonzero adjusted Z, and changing only destination Z must change the existing world hash without changing hash membership.

### 13.4 Snapshot consequence, corrected

There is still no new payload field and no hash-membership change, but successful Ship cell destinations now write a different value into the existing serialized/hashed `DriveCoord.z`. The behavior epoch remains 113 and version 112 is rejected. Tests must cover both the version gate and nonzero destination-Z round-trip/hash sensitivity.

### 13.5 Superseding implementation handoff

- Add a pure recognized-Ship admission predicate beside `navcom`'s Ship setter. Call it from all movement issue entries and repeat it inside `set_destination_internal_cell`; call it in each high-level movement command before mission/order/attack/dock/radio mutation. Leave Walk and all other locomotors' rearm behavior unchanged.
- Split Ship cell-coordinate construction from Drive in `navcom.rs`. Store exact signed ground/slope leptons plus structural 416 for Ship; retain incoming exact Z plus structural 416 for the setter adapter's entity-coordinate form. Do not change Drive destination/head semantics.
- Keep the Ship braking helper's native distance as `i32`. Convert only at the existing `SimFixed` speed-update boundary using exact-through-32767/saturate-from-32768 semantics, then retain strict `<`.
- Continue recomputing braking target Z from destination X/Y, terrain, and structural bridge state; do not read stored Z as the braking ground authority.
- Bump snapshot version 112 -> 113, reject 112, keep payload/hash membership unchanged, and test the changed existing destination-Z value.

Required new critic-2 discriminators are command rejection with complete old-state preservation, Move and AttackMove pre-mutation order, direct/AI/internal entry rejection, Walk-with-rearm acceptance, exact Ship stored Z on flat/signed/slope/structural cells, Drive stored-Z control, entity incoming-Z preservation, getter/retarget/cancel, snapshot/hash, 32767/32768, and long-map distance. All prior discriminators remain in force.

### Addendum-2 sources

- Live Ghidra MCP, active `gamemd.exe`: `ShipLocomotionClass::Set_Destination @ 0x0069F450`, `Destination @ 0x0069F3A0`, `Is_Moving @ 0x0069F290`, `Process @ 0x0069FC10`, `Process_Drive_Track @ 0x006A05F0`, `Stop_Moving @ 0x0069F510`, Ship load/save bodies `0x0069EE90/0x0069EF10`, common raw persistence `0x0055AAC0/0x0055AA60`, and virtual size return `0x006A42A0`.
- Current Rust caller/consumer census: `src/sim/movement/{movement_commands,navcom,movement_tick,drive_locomotion}.rs`, `src/sim/world/{world_commands,world_orders,world_hash}.rs`, `src/sim/{components,snapshot}.rs`, and the direct/layered callers found under `src/sim/{combat,miner,passenger,production,movement}`.

## 14. Design-Critic-3 Repair Addendum (2026-08-27)

This addendum supersedes Sections 12 and 13 wherever they differ. In particular, it corrects the earlier identification of owner vtable `+0x380`, closes stock `UnitClass::Scatter` as a Ship destination installer, and makes the stored-Z terminal consumer implementation-relevant. It does not claim implementation or parity `PASS`.

### 14.1 Stock Ship scatter reaches the exact setter

Live `UnitClass::Scatter @ 0x00743A50` is the active scatter override for category `Unit`, which includes every stock naval unit using Ship locomotion. In both destination-producing branches it resolves a valid `CellClass*` and ends with owner vtable call `+0x480`; `TechnoClass::Set_Destination @ 0x00741970` / `FootClass::Set_Destination_Internal @ 0x004D94B0` resolves the target coordinate and calls the active locomotor's `Move_To`/setter slot `+0x44`. For a recognized Ship that final call is `ShipLocomotionClass::Set_Destination @ 0x0069F450`. The null-threat branch uses `Find_Nearby_Passable_Cell`; the directional branch checks playfield membership and `Can_Enter_Cell` before the same setter dispatch. A rejected Ship setter changes neither the prior locomotor destination nor the owner mission/NavCom.

Current Rust `bump_crush::scatter_blocker` does not exclude Ship. Its vehicle branch accepts a category-`Unit` blocker, selects an adjacent `PathGrid::is_walkable` and unoccupied cell, then calls generic `movement_commands::issue_direct_move`. That direct entry currently writes only `MovementTarget` and facing. It does **not** install owner NavCom or `ShipLocomotionRuntime.destination`, so a stock Ship scattered by a blocked mover can travel with a stale prior Ship destination or with none. The active callers are `movement_tick`, the three `movement_occupancy` paths, `tube_movement`, and `docking/bunker_install`; their production hosts already possess `ResolvedTerrainGrid` directly or through `Simulation`, so terrain can be threaded without inventing an authority. The other direct callers (infantry damage, miners, passengers, sell ejection, building entry) are stock non-Ship controls but must pass the same optional terrain argument.

The repair boundary is transactional. For a recognized Ship, `issue_direct_move` must first read-only preflight, validate the target cell in resolved terrain, compute the complete exact setter coordinate, and construct the complete direct `MovementTarget`. Only then may one mutable commit replace owner NavCom, Ship destination, movement target, and facing. Guard, out-of-grid, missing-terrain, or unsupported-slope failure returns false without changing stale prior navigation/destination/movement/facing; a first install and a replacement both use the exact same commit. Non-Ship direct callers retain their present behavior.

### 14.2 Stored Z controls terminal clear and retry

The relevant terminal block in `ShipLocomotionClass::Process_Drive_Track @ 0x006A05F0` runs after the committed head is nulled and the selector/cursor are reset. Its exact predicate/order is:

1. Require non-null owner `NavCom @ +0x5A4`.
2. Resolve `NavCom->vtable+0x4C` and convert its signed X/Y to cells with `(coord + ((coord >> 31) & 0xFF)) >> 8`.
3. Resolve the owner's current cell through owner vtable `+0x1B8` and require both cells equal.
4. Resolve the **owner's current world coordinate** through owner vtable `+0x4C`; this is not a second NavCom coordinate read.
5. Form signed wrapping `delta = owner.Z - Ship.destination.Z`, native absolute value `(delta ^ (delta >> 31)) - (delta >> 31)`, and require strict signed `< g_ShipHeightStep * 2`. Live `g_ShipHeightStep=104`, so the threshold is 208 and equality fails.
6. Only on that conjunction clear the stored destination triple and the committed-head triple. On failure the head/selector are already retired, but the immediate destination and owner NavCom remain; the next `Process @ 0x0069FC10` no-track path re-resolves NavCom and calls Ship setter slot `+0x44` before rebuilding movement.

For an ordinary nonstructural cell target, incoming and stored Z agree and the terminal delta is normally zero. For a stock Ship under a flat structural bridge, owner `OnBridge=false` leaves owner Z at water/ground while the setter stored `ground+416`; the delta is 416, so the strict `<208` predicate fails and native retains/retries the destination. For an on-deck owner whose current Z agrees with the adjusted destination, it can pass. This relationship is active and is why stored Z cannot be replaced by an unconditionally derived braking-only value.

Current Rust `navcom::finish_drive_navigation` unconditionally calls the Ship null path at every terminal movement, clears NavCom/destination/queue, and therefore erases the active 208-lepton distinction. The existing `navigation.pending_arrival_clear` process-entry retry path already rebuilds a path for a surviving cell NavCom, but its ownership/comments and helpers are Drive-only. The smallest repair generalizes that existing deferred seam to Drive/Ship: terminal Ship completion always retires head/path execution; it clears owner navigation only when the exact cell-plus-owner-Z predicate passes, otherwise preserves NavCom and stored destination and schedules the existing next-tick path rebuild. Current production installs only `NavTargetRef::Cell` for Ship; `Entity/Object/Building` variants have no production Ship installer. A non-cell defensive value therefore cannot prove arrival and must preserve/retry rather than fabricate a coordinate.

### 14.3 `+0x380` is not the ordinary weapon ROF timer

The earlier addenda inherited a stale name and wrongly mapped owner vtable `+0x380 -> 0x004DE770` to `attack_target.cooldown_ticks`. Live evidence separates two owner timers:

| Timer | Exact native fields/writers | Relevance to Ship setter |
|---|---|---|
| ordinary weapon ROF/rearm | Techno `+0x2EC` start and `+0x2F4` duration; written by `TechnoClassFireAtSpawnsBullet @ 0x006FE940` and read by the fire-error pipeline | **not read** by `0x004DE770` or Ship setter |
| post-warp/post-detach idle delay | Foot/Techno `+0x6A0` start and `+0x6A8` delay; read by `0x004DE770` as `remaining != 0` | exact setter guard 2 |

The complete direct-offset instruction census finds owner `+0x6A8` initialized to zero in `FootClass::Constructor @ 0x004D3402`, read by `0x004DE770`, and no ordinary-fire writer. The active indirect writer is `WarpAttachClass::Detach @ 0x0062A4A0`: on full detach it writes its **owner** (`WarpAttachClass+0x24`) `+0x6A0=current frame` and `+0x6A8=3 * value at the selected locomotor/weapon record +0xB0`; early/common cleanup writes the attached target's `+0x6A8=0`. `WarpAttachClass+0x24` is the warping/attacking owner and `+0x28` is its attached target, proven by constructor/use census and the existing WarpAttach report.

A valid object has one active locomotor identity. All stock Ship entries use the Ship CLSID, while an owner that reaches WarpAttach relocation/detach is the teleport/warp owner. No stock Ship type has a temporal/parasite WarpAttach owner path, and the attached Ship target is explicitly cleared to zero. Therefore guard 2 is evidence-excluded for stock recognized Ships. It must **not** reject a normally firing/rearming Ship, and this Phase-3 slice must not promote target-owned `AttackTarget.cooldown_ticks` into a new owner field merely to feed this setter. The known general combat mismatch remains real: Rust stores ordinary cooldown in `AttackTarget`, so target clear/death or a fresh command loses it even though native `+0x2EC/+0x2F4` is owner state. That belongs to combat-cadence closure, not this bridge-Z mechanism, because the native Ship setter never reads that timer. Retarget-in-place already preserves the Rust value; target-clear-to-Move loses it but must still be admitted here.

Required controls are consequently: a normally rearming Ship can retarget into pursuit and can accept a Move after target clear; a Walk rearm control remains unchanged; a synthetic positive post-warp-delay predicate fixture may reject only if the project later represents that exact owner field, but no new field is required for active stock closure. Stale documents naming `0x004DE770` `IsInRearmTimer`, `IsFiring`, or `IsInGarrison` must be corrected to the literal post-warp/post-detach remaining-delay predicate and its stock-Ship exclusion.

### 14.4 Fixed boundary correction

The comparison-preserving adapter remains exact-through-32767 and saturation-from-32768, but the earlier acceptance wording was wrong:

- distance 32767 converts exactly; against slowdown exactly 32767, equality is false;
- against any representable slowdown **greater than** 32767 (for example `SimFixed::MAX`), `32767 < slowdown` is true;
- distance 32768 saturates to `SimFixed::MAX`; against `SimFixed::MAX`, equality is false, matching native because no representable Rust slowdown exceeds 32768;
- a long-map distance above 32768 also saturates without panic/wrap and cannot satisfy the strict comparison.

### 14.5 Invalid target geometry rejects before mutation

Valid production paths do not need a fallback: layered movement resolves an in-grid goal before setter commit; Unit scatter checks playfield/passability; direct scatter selects an adjacent in-grid `PathGrid` cell; post-map production publishes matching `ResolvedTerrainGrid`/`PathGrid`; and retail TMP slope indices are within the 21 records supported by `ground_height_leptons`. Missing terrain, an out-of-grid target, or a slope outside `0..=20` is therefore a test/internal or corrupt-data seam.

Native `MapClass::Get_CellClass` has a dummy-cell facility, but this investigation found no evidence that the Ship setter's successful production callers intentionally install an invalid/dummy cell or that Rust should translate an unsupported slope to zero. The safe parity boundary is rejection: a recognized Ship cell install must return false before power, mission/order, NavCom, destination, path, movement, facing, or occupation mutation. The tick helper remains total for a pre-existing corrupt/restored invalid destination, but it must not invent bridge structure or clear the destination as arrived.

### 14.6 Superseding Rust handoff and tests

- Remove ordinary `attack_target.cooldown_ticks` from the recognized-Ship setter predicate. Preserve guard order among the stock-represented predicates: `dying`, evidence-excluded post-warp delay, warp-out, warp-in.
- Extend `issue_direct_move` with resolved-terrain authority. On recognized-Ship success, atomically install exact owner NavCom and exact Ship cell destination before/with movement; thread terrain through every direct caller and through every `scatter_blocker` caller. Add stale-prior, no-prior, flat, slope, and structural scatter tests.
- Generalize the terminal deferred-repath seam to Ship. Use NavCom only for target X/Y cell equality and use exact owner current world Z versus stored destination Z for strict `<208`; clear on pass, preserve/repath on failure. Add nonstructural delta 0, 207 pass, 208 fail, structural-under-bridge 416 fail, on-deck match pass, missing/non-cell NavCom preserve, and save/load continuation tests.
- Reject recognized-Ship missing-terrain, out-of-grid, and unsupported-slope installs transactionally. Do not retain the Section 13 zero-Z successful fallback.
- Correct the fixed boundary tests exactly as Section 14.4 states.
- Snapshot epoch remains 113/reject-112. No new active-stock timer field is added. Existing corrected `ShipLocomotionRuntime.destination.z` and generalized pending-repath state are already serialized/hashed; tests must prove structural destination and pending retry round-trip/hash behavior.

### Addendum-3 sources

- Live Ghidra MCP, active `gamemd.exe`: `UnitClass::Scatter @ 0x00743A50`, `TechnoClass::Set_Destination @ 0x00741970`, `FootClass::Set_Destination_Internal @ 0x004D94B0`, Ship `Set_Destination @ 0x0069F450`, `Process @ 0x0069FC10`, `Process_Drive_Track @ 0x006A05F0`, timer predicate `0x004DE770`, `FootClass::Constructor @ 0x004D31E0` (`0x004D3402` store), `WarpAttachClass::Detach @ 0x0062A4A0`, and `TechnoClassFireAtSpawnsBullet @ 0x006FE940`.
- Retail and class evidence: all stock naval entries use Ship locomotion; no stock Ship type is a WarpAttach/temporal owner; retail map/TMP slope and post-map grid publication evidence already cited above.
- Current Rust: `src/sim/movement/{bump_crush,movement_commands,movement_tick,movement_occupancy,tube_movement,navcom,teleport_movement}.rs`, `src/sim/docking/bunker_install.rs`, `src/sim/combat/mod.rs`, direct callers under miner/passenger/production/world, `src/sim/game_entity.rs`, `src/sim/snapshot.rs`, and `src/sim/world/world_hash.rs`.

## 15. Exact Active Parasite Prerequisite (critic-4 repair, 2026-08-27)

This section supersedes Sections 12.5, 12.7-12.8, 13.1, 14.3/14.6, and the former Section 15 wherever they describe owner vtable `+0x380`, `Foot+0x6A0/+0x6A8`, `0x006297F0`, or the stock writer schedule. The earlier detonation-time mapping was not an acceptable approximation: native permits a Ship destination install between attachment and the first timer write.

### 15.1 Timer predicate and corrected function identity

The predicate at `0x004DE770` reads signed `Foot+0x6A0` (start) and `Foot+0x6A8` (duration). `start == -1` returns false without treating `duration` as a paused remainder. Otherwise it performs signed/wrapping `elapsed = g_CurrentFrameCounter - start`; it returns true exactly when `elapsed < duration` and `duration - elapsed != 0`. Constructor store `0x004D3402` initializes duration to zero. This timer is not Techno's ordinary weapon cadence at `+0x2EC/+0x2F4` and is not locomotor speed.

`WarpAttachClass::UpdateAttack @ 0x00629FD0` first rejects a null manager victim. It then reads the manager owner type. When both `TechnoType+0xCCE Naval` and `TechnoType+0xD97 Organic` are true, it calls `0x006297F0` and returns before the generic attack-timer/ROF writer at `0x0062A074..0x0062A0A7`. `TechnoTypeClass::ReadINI @ 0x00715024..0x0071503F`, anchored by the literal `Organic`, proves `+0xD97`; retail `[SQD]` sets both flags. Therefore stock Giant Squid always uses `0x006297F0`.

The current Ghidra name `TemporalClass__AI` at `0x006297F0` is stale. Its body reads `WarpAttachClass+0x24` owner and `+0x28` victim, uses the owner's slot-0 weapon, plays the `SQDG*` grapple state machine, applies victim damage, and calls `WarpAttachClass::Detach`. It is the Naval+Organic Parasite/Giant-Squid update, not the Chrono Legionnaire weapon manager. The true chrono-erase manager is Techno `+0x274` and uses `TemporalClass::CanWarpTarget @ 0x0071AE50`, `TemporalClass::InitiateWarp @ 0x0071AF20`, and update body `0x0071A760`; none writes or clears `Foot+0x6A8`. No chrono-erase detonation-time timer write belongs in this mechanism.

### 15.2 Exact stock attach admission

`TechnoClass::Init_Managers @ 0x006F3F40` allocates a 0x58-byte `WarpAttachClass` at non-building owner `+0x69C` only when that owner's rookie primary weapon resolves to a warhead with `WarheadType+0x159 Parasite`. Retail SQD qualifies through both `SquidGrab` and its elite counterpart using `ParasitePlus`.

`BulletClass::DetonateAtCoord @ 0x004690B0`, branch `0x004693D3..0x0046941E`, requires `Parasite=yes`, a non-null source at `Bullet+0xB0`, and an object-class target from `Bullet+0x10C` (`AbstractFlags & 4`). Null, cell, and dummy-cell targets become null. It calls the source's manager `WarpAttachClass::Attach @ 0x0062A980`; the detonation body neither writes the victim timer nor receives an attach-success return.

`WarpAttachClass::CanAttach @ 0x0062A8E0` admits exactly when all of these are true:

1. victim is non-null;
2. victim `InLimbo @ +0x81` is false;
3. victim native-alive byte `+0x90` is nonzero;
4. victim health `+0x6C` is nonzero;
5. victim `+0x694` has no existing Parasite attacker;
6. victim type `+0xD38 Parasiteable` is true;
7. victim `+0x2E4` has no installed/contained-building reciprocal link;
8. only when the attacker type is Naval: victim cell from vslot `+0x1BC` exists and `CellClass::IsWaterSetTile @ 0x00485060` returns true.

There is no alliance, owner, mission, `ImmuneToPoison`, or ordinary Verses gate in `CanAttach`/`Attach`. `ImmuneToPoison @ TechnoType+0xD3B` is read by `TechnoClass::ReceiveDamage` only for a `Poison` warhead; `ParasitePlus` is not Poison. `UnitTypeClass::Constructor @ 0x007470D0`, store `0x00747297`, defaults `Parasiteable` true. Retail explicitly disables it on `DNOA`, `DNOB`, `DRON`, `SQD`, `SMIN`, and `ZEP` and explicitly enables it on `CAOS`.

On admission failure, `Attach` restores/reveals or removes the launched attacker through its nearby-cell fallback and installs no link or victim timer. On success it calls the attacker locomotor slot `+0x70` with selector `-1` and victim coordinates, then writes both directions: `victim+0x694 = attacker` and `manager+0x28 = victim`. It still does not arm `+0x6A8`.

Retail `[SQD]` has `Naval=yes`, `Organic=yes`, Ship locomotion, `Primary=SquidGrab`, `ElitePrimary=SquidGrabE`, and `NavalTargeting=3`. The retail enum defines 3 as organic-secondary, so ordinary non-organic water Units take the ParasitePlus primary while organic Dolphin/Squid targets take `SquidPunch` instead. Two Squids whose projectiles reach one legal victim in the same window are a stock-active discriminator: the first attaches; the second fails the now-non-null `victim+0x694` gate and must not refresh or replace anything.

### 15.3 Exact first write, refresh order, and stock timer value

At the head of every `0x006297F0` call, before its animation-state switch, the manager resolves the owner's current slot-0 weapon and its warhead. If the victim cell pointer is null **or** `IsWaterSetTile` is true, instructions `0x006298A4..0x006298AD` write victim start=current frame and duration=`WarheadType+0x170 Paralyzes`. If the cell exists and is not a water-set tile, `0x0062985F` calls `Detach` instead. `WarheadTypeClass::ReadINI @ 0x0075D3A0` reads `Paralyzes=` as a signed dword. Retail `ParasitePlus` is the only nonzero stock assignment and supplies 32767.

The SQD write is **not** ROF-gated. It runs on every victim-driven update before the state switch. The generic non-Naval+Organic branch of `UpdateAttack` has a separate `WeaponType+0xB0 ROF` manager timer, but SQD returned before that code. Retail even comments that `SquidGrab ROF=99` is ignored. For an attached stock Ship, `+0x6A0` is re-anchored to the current frame and `+0x6A8` is refreshed to 32767 on every qualifying victim Foot-AI tail.

`LogicClass::PerTickUpdate @ 0x0055AFB0` walks the live LogicVector forward and rereads the vector/count after each callback. The pre-existing victim precedes its later-created projectile in ordinary stock naval combat. `FootClass::AI @ 0x004DA530` runs mission work, destination setters, locomotor Process, idle scatter, and transport-entry work before its tail `0x004DAEE1..0x004DAEF3` follows victim `+0x694` to attacker `+0x69C` and invokes `UpdateAttack`. Thus a projectile that attaches after the victim's frame-N visit first writes on the victim's frame-N+1 tail. A Ship `Set_Destination` can occur in that interval and must see the timer inactive. Any Rust write at detonation is an observable false positive.

The state-4 culling path inside `0x006297F0` can detach and kill a red victim, or a yellow victim when the attacker is elite, when the warhead has `Culling=yes`; invalid state and lost-water paths also detach. Full grapple damage, animation, and culling are outside this GSI slice, but any later implementation of those producers must enter the same exact detach transaction before destroying the link.

### 15.4 Detach clear and caller reachability

`WarpAttachClass::Detach @ 0x0062A4A0` has two cleanup tails (`0x0062A862..0x0062A892` and `0x0062A89B..0x0062A8D1`). Both set victim rocking `+0x328=0`, write victim start=current frame/duration=0, clear victim `+0x694`, and clear manager `+0x28`. Naval SQD takes the remove/die-attached branch and skips the non-naval placement block whose owner-side delay is `3 * primary WeaponType.ROF`; a Ship victim never receives that `3*ROF` value.

The complete direct caller set and bounded stock-Ship relationship is:

| Caller | Exact detach admission relevant to an attached Ship | Boundary |
|---|---|---|
| Naval+Organic update `0x006297F0` | victim has left water; invalid grapple state; or `Culling=yes` reaches its red / elite-yellow terminal | water loss and lifecycle death are required; damage/culling production remains outside this slice until its upstream effect exists |
| `FootClass::ReceiveDamage @ 0x004D7330` | attached victim and Sonic warhead (`Warhead+0x14B`); separately, attached victim and negative incoming damage | wire the existing damage receiver before downstream damage/heal mutation |
| `TechnoClass::Receive_Radio @ 0x006F4AB0`, radio `0x1C` | a paid repair step is accepted and the repaired object has an attacker at `+0x694` | wire the active service-depot/naval-yard repair mutation |
| body `0x004DEAE4` (current `StartFidget` name is stale) | non-Organic target with an actual attachment before applying the invulnerability effect; Organic takes the damage branch and does not reach detach | wire the existing Iron Curtain application seam; do not clear from target identity alone |
| `SuperClass::Launch @ 0x006CC390`, Chronosphere case | an iterated source-area object has an actual attacker link, that attacker is Naval, and its manager exists; manager delay is set to 500 before detach | Chronosphere launch is currently unsupported in Rust, so the detach call becomes required with that upstream implementation, not an invented standalone clear |
| `TechnoClass::PerformDeploy @ 0x00710000` | its incoming target has an attacker link, that attacker has a manager, and attacker type is Naval; the call occurs only after Bullet's full IsLocomotor source/target/type/invulnerability/damage-threshold admission | an exact special-effect preflight may call detach; a resolved entity alone may not |
| Teleport locomotor state machine `0x007192F0` (the `0x00719400` symbol is a stale mid-function split) | warping owner has an actual attacker link | no stock recognized Ship owns Teleport locomotion; the Chronosphere case above is the stock Ship release |
| `UnitClass::PerCellProcess @ 0x00739EC0` | grinder/building-entry consumption finds an actual attacker link before deleting the entering Unit | stock recognized Ships cannot enter the land Grinder route; central pointer-expiry cleanup remains required |

`TechnoClass::Receive_Radio` is a repair-radio caller, not evidence for a generic transport clear. True chrono-erase `InitiateWarp` is not a Detach caller and must not clear this timer. Every row requires the actual two-way attachment; no resolved projectile target or broad superweapon target is sufficient authority.

### 15.5 Current Rust mismatch and exact prerequisite boundary

Current Rust already parses `WarheadType.parasite`, selects `SpecialDetonationAction::Parasite`, carries `ProjectileDetonation.source_id` plus `ProjectileTarget::Entity`, and then returns at an explicit unsupported special tail. It parses `ObjectType.naval` and owns exact `ObjectLifecycle::{in_limbo,object_alive}`, health, installed `BunkerLink`, resolved terrain `is_water`, stable IDs, deterministic live-object order, and central pointer-expiry/uninit. It does **not** parse `Paralyzes`, Warhead `Sonic`, `Organic`, or `Parasiteable`; retain a manager-presence fact; hold attacker/victim links; perform `CanAttach`; run a victim Foot-AI-tail update; or own the signed destination-delay timer. `AttackTarget.cooldown_ticks` is unrelated.

The smallest exact prerequisite is a bounded persistent attachment subset, not a full Parasite combat port:

- parse signed `WarheadType.paralyzes`, Warhead `Sonic`, and `ObjectType.organic`; parse `ObjectType.parasiteable` for the bounded recognized-Ship UnitType path with the live UnitType default true. Non-Ship Parasite admission remains on the explicit unsupported path rather than inventing other registry defaults;
- create per-attacker manager state at object initialization exactly when the non-building rookie primary warhead is Parasite; the state contains only `victim_id: Option<u64>` for this slice;
- add victim `parasite_attacker_id: Option<u64>` and owner `foot_destination_delay: CdTimer`, initialized raw inactive (`start=-1`, duration=0);
- at the Parasite special detonation branch, require a recognized Ship victim, the source entity and its manager, and `ProjectileTarget::Entity`; evaluate every Section 15.2 gate read-only, then install both links transactionally; do not arm the timer. Non-Ship targets remain explicitly unsupported in this GSI slice;
- after each victim's mission/destination and locomotor work, at the existing per-object Foot-AI tail seam, validate reciprocal links and resolve the attacker's live slot-0 weapon/warhead from current type and veterancy. For Naval+Organic attacker plus missing cell or water-set cell, write `CdTimer::started(current_frame, paralyzes)` every visit; for a known non-water cell, detach;
- centralize `detach_parasite(attacker_id, victim_id, current_frame)` so it validates the reciprocal pair, clears both links, and writes victim raw start=current/duration=0. Call it from every currently reachable release seam in Section 15.4 and from pointer expiry/uninit of either endpoint. Grinder and grapple-damage producers stay explicit while their upstream effects are absent;
- keep attack animations, periodic damage, culling damage, attacker placement/removal, and generic Terror Drone manager cadence explicit residuals. They are excluded only while their upstream producers remain unsupported; they may not bypass link cleanup once implemented.

Snapshot version remains 113/reject-112, but the payload and hash membership now change. Serialize and hash manager presence/victim ID, victim attacker ID, and raw timer start/duration in stable entity order. Load validation rejects one-sided, self, missing-endpoint, wrong-manager, and duplicate-victim links rather than silently repairing them. The prior claim that version 113 had no new payload is superseded.

### 15.6 Required discriminators

- Detonation frame: a legal SQD hit installs reciprocal links with timer still `start=-1,duration=0`; a Ship destination request in that interval is admitted. The next victim tail writes 32767 after locomotion, and every later qualifying tail re-anchors it.
- Admission failures (null source, non-entity target, limbo/dead/health-zero victim, existing attacker, `Parasiteable=no`, installed bunker, Naval attacker on missing/non-water cell) preserve both entities and timer byte-for-byte.
- Two-Squid race: first attach wins; second fails without timer restart or link replacement.
- Water loss, Sonic damage, negative heal, accepted repair-radio heal, non-Organic Iron Curtain, and endpoint uninit clear one exact reciprocal link and write victim duration zero. Organic invulnerability target, unattached targets, true chrono-erase, and a merely resolved IsLocomotor target do not clear.
- `start=-1,duration>0` remains false for the Ship setter predicate; start=current/duration=32767 is true; exact expiry is false.
- v113 round-trips manager-without-victim and active reciprocal attachment/timer states; v112 rejects; changing either link or either raw timer dword changes the world hash; malformed reciprocal snapshots reject.

The GSI row and design remain **OPEN** on two stock-active upstream release dependencies: Chronosphere source-area admission and IsLocomotor's full pre-`PerformDeploy` admission. A broad superweapon/locomotor target clear is forbidden; those exact upstream surfaces must be promoted before closure. The row also remains open until this prerequisite and the already-specified Ship destination/braking/arrival work are implemented, tested, and freshly criticized. This section does not claim full Giant Squid/Terror Drone combat parity.

### Addendum-4 sources

- Live Ghidra MCP, active `gamemd.exe`: `BulletClass::DetonateAtCoord @ 0x004690B0`; `TechnoClass::Init_Managers @ 0x006F3F40`; `WarpAttachClass::{CanAttach @ 0x0062A8E0, Attach @ 0x0062A980, UpdateAttack @ 0x00629FD0, Detach @ 0x0062A4A0}`; Naval+Organic update `0x006297F0`; `FootClass::AI @ 0x004DA530`; timer predicate `0x004DE770`; `FootClass::ReceiveDamage @ 0x004D7330`; `TechnoClass::Receive_Radio @ 0x006F4AB0`; invulnerability body `0x004DEAE4`; `SuperClass::Launch @ 0x006CC390`; `TechnoClass::PerformDeploy @ 0x00710000`; Teleport state machine `0x007192F0`; `UnitClass::PerCellProcess @ 0x00739EC0`; true chrono-erase manager `0x0071A760`, `0x0071AE50`, `0x0071AF20`.
- Retail `ini/rulesmd.ini`: `[SQD]`, `[SquidGrab]`, `[SquidGrabE]`, `[SquidPunch]`, `[ParasitePlus]`, explicit `Parasiteable=` overrides, and `NavalTargeting` enum comments.
- Current Rust: `src/rules/{object_type,warhead_type}.rs`; `src/sim/{projectile,game_entity,timer}.rs`; `src/sim/combat/{mod,combat_targeting}.rs`; `src/sim/world/{mod,lifecycle,world_hash}.rs`; `src/sim/{snapshot,docking/building_dock,superweapon/iron_curtain}.rs`; resolved terrain and live-object scheduler code cited above.
