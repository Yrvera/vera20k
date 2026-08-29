# Phase 3 Cell Ground Height 104 Domain and Consumer Census — Ghidra Report

**Date:** 2026-08-27

**Target:** active-retail Yuri's Revenge `gamemd.exe`, image base `0x00400000`, x86 32-bit

**Ghidra session:** project `testProsjekt`, program `/gamemd.exe`, connected read-only at `http://127.0.0.1:8089/`

**Primary functions:**

- `CellClass::GetGroundHeight @ 0x00578080` (world-coordinate wrapper)
- `CellClass::ComputeGroundHeightAtCoord @ 0x0047B3A0` (cell-owned slope evaluator)
- `CellClass::GetCoords @ 0x00486840` (ground-only cell-center coordinate)
- `CellClass::GetTargetCoords @ 0x00486890` (structural-bridge-aware target coordinate)
- Spark particle AI `FUN_0062C6E0`, ground call at `0x0062C7D4`

**Investigation mode:** exhaustive mechanism slice plus complete direct-xref census

**Active in YR:** yes. The functions have 185 direct wrapper calls and 32 direct inner-evaluator calls spanning map/cell, movement, placement, targeting, projectile, particle, damage, radar, and draw paths. Spark behavior 3 calls the same wrapper directly.

**Confidence:** HIGH for the scalar values, initializer formula, coordinate and slope domains, bridge offsets, named/global xrefs, the 185/32 direct-call census, and current-Rust consumer inventory. The malformed-slope and unavailable-cell boundaries remain intentionally classified rather than approximated.

**Implementation scope:** none. This report is a research handoff. It changes no Rust code and makes no Ghidra changes.

**Post-implementation Spark status (2026-08-27):** The Spark-specific
shared-dummy and invalid/unallocated-cell gap identified in this census is
closed. The current authority is the [Phase 3 Spark shared-dummy routing
contract](PHASE3_SPARK_SHARED_DUMMY_ROUTING_GHIDRA_REPORT.md). Implementation
and review repairs are `4c71b488`, `72bf8e15`, `96779c16`, and `0054549e`.
Spark-gap statements below are retained only as a historical pre-`4c71b488`
baseline; they do not describe current Rust. This addendum does not change the
status of other 104-domain consumers owned by separate rows.

## Verdict

The proposed split between a **90-lepton `CellClass::GetGroundHeight` domain** and a **104-lepton object/VXL/bridge domain is false in the active retail executable**.

Active runtime capture and static initializer disassembly agree:

- `g_nCellGroundLevelHeightLeptons @ 0x0089E7C0` is **104**, not 90.
- `CellClass::ComputeGroundHeightAtCoord @ 0x0047B3A0` multiplies the signed cell level and every slope record by that 104-lepton scalar.
- the literal `90.0 @ 0x007E1730` is an **angle in degrees** used with `60.0` to form `pi/6`; it is not a height result;
- `CellClass::GetTargetCoords` adds `g_nCellGroundStructuralDeckOffsetLeptons @ 0x0089E7B4 = 416` only when `CellClass+0x140 & 0x100` is set;
- Spark obtains ground by calling the same `CellClass::GetGroundHeight`, then adds its module-local `DAT_00AC4A0C = 416` bridge plane;
- independent Foot, Techno/InRange, area-damage, Anim, Bullet, Particle, Unit, and VXL initializer families also resolve to 104, and all examined four-level deck offsets resolve to 416.

Current Rust therefore has one high-impact regression and one important preservation rule:

1. `src/util/lepton.rs::CELLCLASS_GROUND_LEVEL_HEIGHT_LEPTONS = 90` and `cellclass_ground_height_leptons` are wrong. Their production callers produce incorrect terrain Z, target Z, placement Z, and test fixtures on every nonzero level and many slopes.
2. Spark's current use of `ground_height_leptons` (104) is correct and must be preserved. Changing Spark to 90 would introduce the exact bug this investigation disproves.

There is no validated active symbol named `MapClass::GetZPos` in this program. The load-bearing native distinction is not `MapClass 104` versus `CellClass 90`; it is:

```text
ground-only surface = CellClass::GetGroundHeight / GetCoords
bridge-aware target = ground-only surface + a caller-owned 416 offset when its bridge predicate selects deck
```

## 1. Scope and evidence order

### Included

1. Static derivation and live runtime value of the Cell ground-level scalar.
2. Exact Cell ground formula, coordinate units, signedness, slope-table domain, clamp order, and rounding.
3. World-coordinate lookup, flattened 512-wide indexing, and shared dummy behavior.
4. Ground-only versus bridge-aware coordinate APIs.
5. Spark constructor and behavior-3 height consumers.
6. Every direct xref to the ground wrapper and inner evaluator.
7. Direct readers of the named module-local 104/416 scalar pairs plus the Bullet, Particle, Unit, and VXL peers needed to close the domain hypothesis.
8. Every current Rust use of the 90-specific helper and the current 104 ground evaluator.
9. Evidence-backed exclusions: INI authority, nonexistent `MapClass::GetZPos` label, bridge non-inclusion in ground sampling, invalid slopes, and unavailable-cell policy.

### Not re-investigated here

- Spark's already-researched float integration, reflection matrices, RNG, color, or compositor arithmetic, except where their collision plane consumes ground/bridge Z.
- The complete behavior of every one of the 217 direct callers. This report exhaustively enumerates them and classifies their height role; caller-specific state machines remain owned by their system reports.
- Every writer to the shared dummy cell. The lookup and restamp behavior needed by this height mechanism is verified; complete global dummy lifecycle belongs to the map/cell substrate.
- Malformed executable state with slope bytes above 20. Native performs an unbounded table read; Rust must not emulate memory-unsafety.

### Evidence order

1. Fresh Ghidra disassembly/decompile and direct-xref queries against active `gamemd.exe`.
2. Read-only `ReadProcessMemory` capture of the running active-retail `gamemd.exe` process.
3. Current Rust source and git history.
4. Existing research/plans only as leads. Stale prose was not used to prove the scalar.

## 2. Opening questions log

| ID | Opening question |
|---|---|
| Q01 | What value is actually stored in the active Cell ground scalar? |
| Q02 | Does the static Cell initializer calculate 90 or 104? |
| Q03 | What does the literal 90.0 mean in that initializer? |
| Q04 | Is Cell ground numerically distinct from the VXL/object domain? |
| Q05 | What are the exact level, slope, XY, and return coordinate domains? |
| Q06 | What are the base-height rounding and negative-level semantics? |
| Q07 | What is the slope record layout, clamp order, and valid index range? |
| Q08 | How does the wrapper convert signed world coordinates to a cell? |
| Q09 | What happens when the flattened lookup misses? |
| Q10 | Does the ground sampler include bridge height? |
| Q11 | Which native method returns bridge-aware cell target coordinates? |
| Q12 | What bridge offset does that method add? |
| Q13 | What ground and bridge constants does Spark consume? |
| Q14 | Does Particle construction use the same ground wrapper? |
| Q15 | Which independent modules carry peer level/deck scalars? |
| Q16 | Are any of these values controlled by retail INI/map data? |
| Q17 | Is `MapClass::GetZPos` a verified active symbol? |
| Q18 | Which direct native consumers can observe the Cell ground result? |
| Q19 | Which current Rust production callers use the false 90 domain? |
| Q20 | Which current Rust callers already use the correct 104 domain? |
| Q21 | What exact correction and regression tests does the builder need? |

All questions are resolved in §12. No numeric claim in this slice remains approximate.

## 3. CellClass layout and coordinate domains

| Source | Native field/input | Width and interpretation | Height role |
|---|---|---|---|
| `CellClass+0x24` | packed cell X | signed `i16` | `GetCoords` forms `X * 256 + 128` |
| `CellClass+0x26` | packed cell Y | signed `i16` | `GetCoords` forms `Y * 256 + 128` |
| `CellClass+0x11B` | Level | signed `i8` at evaluation | base terrain height |
| `CellClass+0x11C` | slope index | unsigned `u8` | record 0 means flat; verified table indices 1..20 |
| `CellClass+0x140` | flags | `u32`; bit `0x100` | structural bridge predicate used by bridge-aware callers |
| ground wrapper input | world X/Y | signed `i32` leptons | divided by 256 with truncation toward zero |
| ground local X/Y | low coordinate bytes | unsigned `0..255` | slope interpolation within one cell |
| returned Z | world Z | signed `i32` leptons | terrain floor only |
| bridge offset | module-local integer | signed `i32` leptons | 416, added only by a caller that selected deck |

The coordinate frames are therefore compatible, not competing:

```text
1 cell axis step             = 256 world leptons
1 terrain Level step         = 104 world-Z leptons
1 structural deck offset     = 4 Levels = 416 world-Z leptons
cell-center local coordinate = (128, 128) leptons
```

## 4. The Cell scalar is 104, not 90

### 4.1 Static initializer

The Cell initializer region is a sequence of CRT init thunks. The load-bearing chain is:

```text
0x0047B1E0: radians60 = (pi / 180) * 60.0
0x0047B200: radians90 = (pi / 180) * 90.0
0x0047B220: angle = radians90 - radians60
0x0047B232: tan = Math__TanFromTable4096(angle)
0x0047B237: tan *= cell_diagonal
0x0047B240: tan *= 0.5
0x0047B246: Math__ftol
0x0047B24B: g_nCellGroundLevelHeightLeptons = EAX
```

The earlier thunks derive `cell_diagonal = sqrt(2 * 256^2)`. The live double at `0x0089E750` is `362.0386657714844`. The angle intermediates captured live are:

| Address | Runtime double | Meaning |
|---|---:|---|
| `0x0089E728` | `1.5707963267948966` | 90 degrees in radians |
| `0x0089E758` | `1.0471975511965976` | 60 degrees in radians |
| `0x0089E750` | `362.0386657714844` | cell diagonal |

Thus the static expression is:

```text
CellLevelHeight = ftol_chop(
    tan_table((90.0 - 60.0) * pi/180)
    * sqrt(2 * 256^2)
    * 0.5
) = 104
```

The old 90 claim stopped at the degree literal `0x007E1730` and misidentified an initializer input as its integer output.

### 4.2 Live runtime capture

The active process was `C:\Users\enok\Documents\Command and Conquer Red Alert II\gamemd.exe` (PID 9452 at capture time). Read-only `OpenProcess(PROCESS_VM_READ)` and `ReadProcessMemory` returned:

| Address | Runtime bytes | Signed integer | Role |
|---|---|---:|---|
| `0x0089E7C0` | `68 00 00 00` | 104 | Cell ground LevelHeight |
| `0x0089E7B4` | `A0 01 00 00` | 416 | Cell structural-deck target offset |
| `0x00AC13C8` | `68 00 00 00` | 104 | Foot/object LevelHeight |
| `0x00AC13BC` | `A0 01 00 00` | 416 | Foot OnBridge deck offset |
| `0x00B0EB34` | `68 00 00 00` | 104 | Techno/InRange LevelHeight |
| `0x00B0EB24` | `A0 01 00 00` | 416 | Techno/InRange structural-deck offset |
| `0x0089E870` | `68 00 00 00` | 104 | area-damage LevelHeight |
| `0x0089E864` | `A0 01 00 00` | 416 | area-damage structural-deck offset |
| `0x0089A1C0` | `68 00 00 00` | 104 | Anim LevelHeight |
| `0x0089A1B4` | `A0 01 00 00` | 416 | Anim structural-deck offset |
| `0x0089DE70` | `68 00 00 00` | 104 | Bullet module LevelHeight |
| `0x0089DE64` | `A0 01 00 00` | 416 | Bullet collision/deck plane offset |
| `0x00AC4A18` | `68 00 00 00` | 104 | Particle module LevelHeight |
| `0x00AC4A0C` | `A0 01 00 00` | 416 | Particle/Spark bridge plane offset |
| `0x00B1D0B8` | `68 00 00 00` | 104 | Unit module LevelHeight |
| `0x00B1D0AC` | `A0 01 00 00` | 416 | Unit bridge offset |
| `0x00B45578` | `68 00 00 00` | 104 | VXL LevelHeight |

These are separate globals and separately emitted initializer thunks, which matters for ownership and xrefs. They are not separate numeric height systems in the retail executable.

### 4.3 Derived deck value

The Cell deck thunk `0x0047B2C0..0x0047B2E5` reads `0x0089E7C0`, forms `4 * scalar`, adds `0.5`, chops with `Math__ftol`, and writes `0x0089E7B4`:

```text
CellDeckOffset = ftol_chop(4 * 104 + 0.5) = 416
```

Particle's peer thunk `0x0062B540..0x0062B566` reads `0x00AC4A18` and writes `0x00AC4A0C` with the same `4*x + 0.5` sequence. Foot does the same at `0x005F3860..0x005F3886`. The other module pairs have the same resolved values and their own xref sets.

## 5. Exact ground-height evaluator

### 5.1 Base height

`CellClass::ComputeGroundHeightAtCoord @ 0x0047B3A0` lazily converts the integer scalar to a double and caches `scalar / 256`. Assembly at `0x0047BA94..0x0047BAB5` then:

1. sign-extends `Cell+0x11B`;
2. multiplies by `g_nCellGroundLevelHeightLeptons`;
3. adds `0.5`;
4. calls `Math__ftol @ 0x007C5F00`, with the active control word chopping toward zero;
5. reads slope as the unsigned byte at `Cell+0x11C`.

For flat slope 0:

```text
base = ftol_chop(sign_extend_i8(Level) * 104 + 0.5)
```

Examples:

| Level byte | Signed level | Flat ground Z |
|---:|---:|---:|
| `0x00` | 0 | 0 |
| `0x01` | 1 | 104 |
| `0x02` | 2 | 208 |
| `0xFF` | -1 | -103 |

The negative result is intentionally asymmetric because `-104 + 0.5 = -103.5`, then chop toward zero yields `-103`.

### 5.2 Slope contribution

For nonzero slope `s`, native indexes record `s-1` in the 20-record table rooted at `0x0081C900`. Only `world_x & 0xFF` and `world_y & 0xFF` participate. With `G = 104`:

```text
raw = (local_y * coeff_y + local_x * coeff_x) * (G / 256)
    + bias_a
    + bias_b
slope_term = clamp(raw, 0, maximum)
height = ftol_chop(base + slope_term)
```

The clamp is applied to the slope term before base is added. The final conversion also chops toward zero.

| Slope | `coeff_x` | `coeff_y` | `bias_a` | `maximum` | `bias_b` |
|---:|---:|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 | 0 | 0 |
| 1 | 1 | 0 | 0 | G | 0 |
| 2 | 0 | 1 | 0 | G | 0 |
| 3 | -1 | 0 | G | G | 0 |
| 4 | 0 | -1 | G | G | 0 |
| 5 | 1 | 1 | -G | G | 0 |
| 6 | -1 | 1 | 0 | G | 0 |
| 7 | -1 | -1 | G | G | 0 |
| 8 | 1 | -1 | 0 | G | 0 |
| 9 | 1 | 1 | 0 | G | 0 |
| 10 | -1 | 1 | G | G | 0 |
| 11 | -1 | -1 | 2G | G | 0 |
| 12 | 1 | -1 | G | G | 0 |
| 13 | 1 | 1 | 0 | 2G | 0 |
| 14 | -1 | 1 | G | 2G | 0 |
| 15 | -1 | -1 | 2G | 2G | 0 |
| 16 | 1 | -1 | G | 2G | 0 |
| 17 | 0 | 0 | 0 | G/2 | G/2 |
| 18 | 0 | 0 | G | G/2 | -G/2 |
| 19 | 0 | 0 | 0 | G/2 | G/2 |
| 20 | 0 | 0 | G | G/2 | -G/2 |

At cell center `(128,128)`, the contributions for slopes 0..20 are:

```text
0, 52, 52, 52, 52,
0, 0, 0, 0,
104, 104, 104, 104, 104, 104, 104, 104,
52, 52, 52, 52
```

Native has no safe bounds check for a malformed slope above 20. The Rust `UnsupportedGroundSlope` result is the correct safe boundary, but it is not bit-for-bit behavior for corrupt executable state.

## 6. Lookup, dummy cell, and coordinate methods

### 6.1 Wrapper conversion and flattened lookup

`CellClass::GetGroundHeight @ 0x00578080` converts each signed world axis as:

```text
cell_axis = (world_axis + ((world_axis >> 31) & 0xFF)) >> 8
```

This equals signed division by 256 truncating toward zero. It then forms:

```text
linear = cell_y * 512 + cell_x
```

The validation is on the flattened index and pointer, not on X and Y independently. An individually out-of-range X can alias a valid linear slot when the flattened result is valid.

On invalid flattened index or null pointer, the wrapper uses the shared dummy `CellClass @ 0x00ABDC50`, stamps the requested packed coordinate at dummy `+0x24/+0x26` (`0x00ABDC74`), and calls the same inner evaluator. Dummy level/slope are live shared state, not an implicit immutable `(0,0)`.

Current Rust already has the correct generic substrate in `src/sim/cell_rect.rs::get_cellclass_fallback`: fixed 512-wide indexing, packed-coordinate truncation, real-or-dummy return, and shared dummy coordinate stamping. Callers that use `checked_cell_from_world` or return `None`/typed unavailable instead are not exact at this boundary.

### 6.2 Ground-only cell center

`CellClass::GetCoords @ 0x00486840`, vtable slot `+0x48`, forms:

```text
X = sign_extend_i16(Cell.X) * 256 + 128
Y = sign_extend_i16(Cell.Y) * 256 + 128
Z = CellClass::ComputeGroundHeightAtCoord(this, X, Y)
```

It does not add a bridge term. `CellClass::Get_Center_Coords @ 0x00480A30` is a direct-call sibling that passes the same local `(128,128)` point into the inner evaluator.

### 6.3 Bridge-aware target coordinate

`CellClass::GetTargetCoords @ 0x00486890`, vtable slot `+0x58`, is the bridge-aware cell target method:

```text
coord = this->GetCoords()
if (this->flags_0x140 & 0x100) != 0:
    coord.z += g_nCellGroundStructuralDeckOffsetLeptons  // 416
return coord
```

This is a caller composition over the same 104-lepton ground. It is not a second terrain evaluator.

### 6.4 `MapClass::GetZPos` exclusion

Fresh Ghidra symbol search for `GetZ` found `AnimClass::GetZAdjust @ 0x00425630` and `MapClass::GetZoneID @ 0x0056D230`; it found no `MapClass::GetZPos`. A repository-wide source/doc search also found no such verified symbol. Treat `MapClass::GetZPos` as an unverified shorthand, not an evidence label. The active functions proven above are the names and addresses implementation should cite.

## 7. Spark uses 104 ground plus 416 deck plane

### 7.1 Constructor floor

`ParticleClass::Constructor @ 0x0062B5E0` calls `CellClass::GetGroundHeight` at `0x0062B8B1`. If the input Z is at or below ground, it calls again at `0x0062B8C2` and replaces Z with that ground result before `Set_Raw_Coords`. There is no Spark-specific 90 scalar in this path.

### 7.2 Behavior-3 collision

Spark AI `FUN_0062C6E0` does:

```text
0x0062C7D4  ground = CellClass::GetGroundHeight(candidate_coord)
0x0062C7D9  deck_offset = DAT_00AC4A0C
0x0062C7E8  bridge_plane = ground + deck_offset
0x0062C7F4  candidate_cell = MapClass::Get_CellClass_At_Coord(candidate)
0x0062C81C  old_cell       = MapClass::Get_CellClass_At_Coord(old)
```

It tests `Cell+0x140 & 0x100` on candidate or old cell, then performs its asymmetric ascending/descending crossing checks around `bridge_plane`. Live runtime values are:

```text
Particle LevelHeight = 104
Spark bridge offset  = 416
Spark bridge plane   = candidate CellClass ground + 416
ascending snap       = bridge plane - 20 = ground + 396
```

The stale `ground + 360` / `ground + 340` interpretation is wrong.

### 7.3 Post-implementation Rust Spark verdict

`src/sim/particles/spark_world.rs` calls `ground_height_leptons` at lines 96 and 119. That evaluator uses 104 and is the correct scalar choice. `src/sim/particles/spark.rs` obtains `STRUCTURAL_BRIDGE_HEIGHT` from the 416-lepton bridge topology constant. These numeric choices match active retail.

The historical pre-`4c71b488` boundary distinction was lookup failure:

- Rust's Spark adapter reproduced signed truncation and fixed-stride flattened indexing.
- For a canonical cell missing from resolved terrain, it returned `UnavailableCell`; constructor ground converted that to `None`.
- Native uses the shared dummy and continues.

Current Rust now routes constructor ground and behavior-3 collision through the
same terrain-bound shared dummy with native lookup order, restamping, lazy fact
reads, collision continuation, and unconditional level/slope hash authority.
For Spark, the former safety/availability divergence and invalid-cell residual
are closed; other height consumers retain their own separately scoped routing
status.

## 8. Native scalar xrefs and ownership

### 8.1 Named Cell pair

`g_nCellGroundLevelHeightLeptons @ 0x0089E7C0`:

- reads: `CellClass::RecalcAttributes @ 0x0047D9C9`; `ComputeGroundHeightAtCoord @ 0x0047B3B5, 0x0047BA9B`; `CellClass::BlowUpBridge @ 0x0047DE83`; `CellOverlay_TileDraw @ 0x004804DC`; derivative thunks `0x0047B260, 0x0047B291, 0x0047B2C1`;
- write: `0x0047B24B`.

`g_nCellGroundStructuralDeckOffsetLeptons @ 0x0089E7B4`:

- reads: `FUN_00487720 @ 0x00487771`; `CellClass::BlowUpBridge @ 0x0047DEA0`; `CellClass::PlaceInfantryInCell @ 0x0048147B`; `CellClass::GetTargetCoords @ 0x004868A1`;
- write: `0x0047B2E0`.

### 8.2 Other named 104/416 pairs

| Owner | 104 scalar direct readers | 416 scalar direct readers |
|---|---|---|
| Foot/object `0x00AC13C8` / `0x00AC13BC` | `ObjectClass::IsHighFlying 0x005F6B9F`; `IsLowFlying 0x005F6B6F`; `ShouldBeOnBridge 0x005F6AEA, 0x005F6B20` | `FootClass::Set_Height_On_Bridge 0x005F5FB5`; `ObjectClass::GetHeight 0x005F5F86`; `Mark_Put 0x005F60B0`; `Mark_Remove 0x005F6130` |
| Techno/InRange `0x00B0EB34` / `0x00B0EB24` | railgun helper `0x0070C833`; `TechnoClass::InRange 0x006F74F8`; `FUN_006F6F60 0x006F7087`; `TechnoClass::GetFireError 0x006FCC9C` | `TechnoClass::InRange 0x006F7367, 0x006F761C`; `TechnoClass::CanFireAt 0x006F7887` |
| area damage `0x0089E870` / `0x0089E864` | `Apply_area_damage 0x0048979E, 0x0048982B, 0x00489F90, 0x00489FB1, 0x0048A114`; `Warhead::SelectExplosionAnim 0x0048A541` | area-damage bridge selectors; writer `0x00489120` derives `4*104` |
| Anim `0x0089A1C0` / `0x0089A1B4` | `AnimClass::BounceAI 0x004257A3, 0x00425830, 0x004259B8`; helper `0x0042610C` | `AnimClass::AI 0x00423CB6`; `BounceAI 0x0042585D, 0x004259E5, 0x00425A7B`; occupancy `0x0042629B, 0x00426328` |

### 8.3 Additional peer globals used by direct ground consumers

| Owner | 104 scalar | 416 scalar | Representative readers |
|---|---:|---:|---|
| Bullet | `0x0089DE70` | `0x0089DE64` | `BulletClass::AI`, detonation, rotating-SHP draw |
| Particle/Spark | `0x00AC4A18` | `0x00AC4A0C` | particle move behaviors, Spark AI |
| Unit | `0x00B1D0B8` | `0x00B1D0AC` | unit scatter, scenario unit read, action/occupation/draw |
| VXL | `0x00B45578` | not the Cell target offset | VXL slope tilt initialization |

Independent ownership explains why the binary has multiple globals. It does not justify numerically splitting Rust ground evaluators.

## 9. Complete native direct-call census

Fresh `get_xrefs_to` returned exactly **185** direct calls to wrapper `0x00578080`, grouped into **107** named/unnamed function owners below. Every returned callsite is included. This proves the ground wrapper is shared broadly; it does not claim every caller's entire state machine was re-audited here.

### 9.1 Damage, projectiles, particles, and effects

- `Apply_area_damage`: `0x004893B5`, `0x00489573`
- `Warhead__SelectExplosionAnim`: `0x0048A53C`
- `AnimClass__AI`: `0x00423CB1`
- `AnimClass__BounceAI`: `0x0042582B`, `0x00425847`, `0x00425858`, `0x004259B3`, `0x004259CF`, `0x004259E0`, `0x00425A76`
- `AnimClass__MarkCellOccupancy`: `0x00426296`
- `AnimClass__ClearCellOccupancy`: `0x00426323`
- `BounceClass__Update`: `0x00439BF3`
- `FUN_00439A10`: `0x00439A45`
- `BulletClass__AI`: `0x00467074`, `0x0046737B`, `0x0046749E`
- `BulletClass__HomingTrack`: `0x005B26DC`
- `BulletClassDrawItRotatingShpFrameDispatch`: `0x004682AB`
- `BulletClassFireRevealArmAndSubmit`: `0x0046891F`
- `ParticleClass__Constructor`: `0x0062B8B1`, `0x0062B8C2`
- `ParticleClass__Move_Dispatch`: `0x0062D68F`, `0x0062D6A0`
- particle behaviors `FUN_0062BD50`: `0x0062BF22`; Spark `FUN_0062C6E0`: `0x0062C7D4`; `FUN_0062D2A0`: `0x0062D356`, `0x0062D3AF`, `0x0062D3C9`; `FUN_0062D3F0`: `0x0062D4AF`
- `VoxelAnimClass__AI`: `0x0074A00B`
- `BuildingClass__ReceiveDamage`: `0x0044277C`
- effect/damage helpers `FUN_0043E940`: `0x0043EB6D`; `FUN_00487720`: `0x0048776C`; `FUN_004A9070`: `0x004A914E`

### 9.2 Targeting, range, actions, and superweapons

- `TechnoClass__InRange`: `0x006F7340`, `0x006F7617`
- railgun helper `FUN_0070C690`: `0x0070CA6E`
- `FootClass__Greatest_Threat_Scan`: `0x004D5F80`, `0x004D6172`
- `FootClass__What_Action_OnCell`: `0x004DDE32`
- `FootClass__What_Action_OnObject`: `0x004DDF13`
- `FootClass__ClickedAction_Cell`: `0x004D7EF5`
- `FootClass__ClickedAction_Object`: `0x004D7837`, `0x004D784C`, `0x004D79C7`
- `UnitClass__What_Action_OnCell`: `0x0074059E`
- `DisplayClass__SetCursorFromAction`: `0x004AAF87`
- `DisplayClass__DetermineAction`: `0x00692A5D`
- `SuperClass__Launch`: `0x006CDAC6`, `0x006CDD88`
- `House__LaunchNukeDown`: `0x006E3466`
- `BuildingClass__Mission_Missile`: `0x0044D001`
- `TriggerAction__Execute`: `0x006DD975`, `0x006DDA4E`, `0x006DEE46`, `0x006DF44D`, `0x006DF7DC`
- trigger/superweapon helpers `FUN_006E1CC0`: `0x006E1D0E`; `FUN_006E2290`: `0x006E22E1`; `FUN_006E2390`: `0x006E23E4`; `FUN_006E2520`: `0x006E2571`; `FUN_006E2C40`: `0x006E2C93`, `0x006E2D89`; `FUN_006E38F0`: `0x006E3946`; `FUN_00692300`: `0x0069244C`

### 9.3 Placement, cell occupancy, harvestability, and object lifecycle

- `CellClass__PlaceInfantryInCell`: `0x00481428`, `0x0048146E`
- `Try_Unlimbo_Object_At_Or_Near_Cell`: `0x00688F6A`, `0x0068924C`
- `AircraftClass__Unlimbo`: `0x00414361`, `0x00414378`
- `AircraftClass__Carryall_Pickup`: `0x00416B3C`
- `InfantryClass__Unlimbo`: `0x0051E002`
- `FootClass__Set_Height_On_Bridge`: `0x005F5FF6`, `0x005F603F`
- `ObjectClass__GetHeight`: `0x005F5F6E`
- `ObjectClass__ShouldBeOnBridge`: `0x005F6AAE`, `0x005F6AD9`
- `ObjectClass__Mark_Put`: `0x005F60AB`
- `ObjectClass__Mark_Remove`: `0x005F612B`
- `InfantryClass__MarkCellOccupancy`: `0x005217E6`
- `InfantryClass__UnmarkCellOccupancy`: `0x00521873`
- `UnitClass__MarkCellOccupationBit20`: `0x007441C9`
- `UnitClass__ClearCellOccupationBit20`: `0x00744229`
- `OverlayClass__Mark`: `0x005FD159`
- `FootClass__Is_Cell_Harvestable`: `0x004DCEE5`
- `FootClass__Is_Cell_Weedable`: `0x004DDA82`
- `VeinholeMonsterClass__Constructor`: `0x0074C8D3`
- `ScenarioClass__Read_Units_Section`: `0x00743505`
- `EventClass__Execute`: `0x004C6FFE`
- `TeamClass__Recruit_Or_Add`: `0x006E9AFA`
- `TechnoClass__Set_Destination`: `0x007428D1`
- lifecycle helpers `FUN_0049F550`: `0x0049F587`, `0x0049F59C`; `FUN_00513D20`: `0x00513D75`, `0x00513DC3`; `FUN_0051B350`: `0x0051B45E`, `0x0051B46F`, `0x0051B98A`; `FUN_0051FB00`: `0x0051FDC4`, `0x0051FDDB`; `FUN_0054D6D0`: `0x0054D746`, `0x0054D7CD`; `FUN_007298F0`: `0x0072991D`

### 9.4 Locomotion and spatial movement

- `DriveLocomotionClass__Process_Drive_Track`: `0x004B1003`, `0x004B1422`, `0x004B1469`, `0x004B18C7`
- `DriveLocomotionClass__Process_Movement`: `0x004B2BB2`, `0x004B3097`, `0x004B3CEC`, `0x004B3D1A`
- `ShipLocomotionClass__Process_Drive_Track`: `0x006A06D3`, `0x006A0AEA`, `0x006A0B31`, `0x006A0F53`
- `ShipLocomotionClass__Process_Movement`: `0x006A2202`, `0x006A26E7`, `0x006A333B`, `0x006A3369`
- `FlyLocomotionClass__Process`: `0x004CD6F8`
- `FlyLocomotionClass__Descent_Step`: `0x004CEAAC`, `0x004CEB41`, `0x004CECE7`
- `FlyLocomotionClass__Horizontal_Step`: `0x004CF3EC`
- `FlyLocomotionClass__Move_To_Coord`: `0x004CCE5B`, `0x004CCEE2`
- `FlyLocomotionClass__Emergency_Relocate`: `0x004CD1C4`
- `HoverLocomotionClass__Move`: `0x00514BF8`
- hover helpers `FUN_00514F70`: `0x0051525C`, `0x005152AC`, `0x00515395`; `FUN_005164D0`: `0x00516A23`
- `JumpjetLocomotionClass__Update_Coordinates_And_Altitude`: `0x0054D29A`, `0x0054D3EF`
- `JumpjetLocomotionClass__State4_Descend`: `0x0054C874`
- `JumpjetLocomotionClass__State5_Touchdown`: `0x0054CB48`
- `TeleportLocomotionClass__Process`: `0x00718C56`, `0x00719221`
- `TeleportLocomotionClass__Update_Position`: `0x007186D9`
- `WalkLocomotionClass__ProcessMovement`: `0x0075B4E6`, `0x0075B532`
- `WalkLocomotionClass__FindSubCellDest`: `0x0075C4FB`
- `UnitClass__TubeMovement`: `0x00735B05`, `0x00735B16`
- movement helpers `FUN_005B01C0`: `0x005B07D7`, `0x005B0823`, `0x005B0916`; `FUN_005B17B0`: `0x005B17F6`; `FUN_004DE1D0`: `0x004DE22F`, `0x004DE379`

### 9.5 Radar, tactical placement, and drawing

- `RadarClass__RenderCellPixel`: `0x00655D33`
- `TechnoClass__DrawRadarActionLines`: `0x004DC48D`
- `TechnoClass__DrawActionLines`: `0x004DC268`
- `BuildingPlacement_OverlayRenderer`: `0x006D535E`
- `BuildingPlacement_per_cell_draw`: `0x0047ED66`
- `AircraftClass__Draw_It`: `0x004146AD`
- `UnitClass__DrawVoxelBody`: `0x0073C310`
- `InfantryClass__DrawExtras`: `0x005193B0`
- draw/tactical helpers `FUN_00684C30`: `0x00684D2F`; `FUN_006DA9D0`: `0x006DAB52`; `FUN_006539D0`: `0x00653B0B`

### 9.6 Unnamed Ghidra owners retained verbatim

The following direct callsites currently have no owning function name in the active database and are retained so the census is lossless:

```text
0x00464A96 0x0046C515 0x0046C52C 0x004B609A
0x00514DF9 0x00516F26 0x005247F5 0x0052480C
0x00537392 0x00537482 0x00537572 0x00537662
0x0054B634 0x005FD9AF 0x0071E0F5 0x0071E10C
0x00729E95 0x00747ED5 0x00747EEC 0x00749BF6
0x0074D0A4
```

### 9.7 Direct inner-evaluator census

Fresh `get_xrefs_to 0x0047B3A0` returned exactly **32** direct calls:

- `CellClass__Get_Center_Coords`: `0x00480A60`
- `CellClass__GetGroundHeight`: `0x005780ED`
- `CellClass__GetCoords`: `0x0048686C`
- `CellClass__AddContent`: `0x0047E953`, `0x0047E9A4`
- `CellClass__IsShrouded_AtMapCoord`: `0x00487980`
- `CrateClass__PickupCloak`: `0x004828D0`
- `FUN_00484180`: `0x004842DD`
- `FUN_00487A10`: `0x00487AE4`
- `FUN_00486920`: `0x00486A0A`
- `FUN_00482900`: `0x00483334`
- `FUN_00485080`: `0x0048509E`
- `FUN_00580BC0`: `0x00580CE5`, `0x00580D91`, `0x00580E79`, `0x00580F75`
- unnamed owners: `0x00482002`, `0x00482258`, `0x004822E8`, `0x00482416`, `0x00482529`, `0x00482608`, `0x00482762`, `0x00482A02`, `0x00482B5C`, `0x00482BE5`, `0x00482DD8`, `0x00482F03`, `0x00482FBB`, `0x004830F2`, `0x004831A7`, `0x004832CE`

## 10. Retail data and INI authority

No `Rules`, `Art`, map, theater, or scenario INI read writes any scalar in this report. The values are initialized by executable geometry/trigonometry thunks before consumers run. Direct xref sets show one static writer per scalar family plus derived thunks; none is an INI parser.

The active install exposes only runtime/user INIs at the filesystem root; stock rules/art live in MIX archives. That packaging does not affect the conclusion because the executable writers do not consult INI storage. Existing reports that call these values `BridgeHeight` as a semantic label must not be read as evidence for a `[Rules] BridgeHeight=` key.

Evidence-backed exclusions:

- lighting `[Lighting] Level=` is unrelated visual/scenario state;
- `RadLevelDelay=90` and timing values of 90 are unrelated units;
- the `90.0` geometry literal is degrees;
- no retail INI can make Cell ground use 90 while VXL uses 104;
- TS-only branches in some consumers do not change the shared active evaluator or its scalar.

## 11. Current Rust comparison

### 11.1 Correct common evaluator and correct consumers

`src/util/lepton.rs::GROUND_LEVEL_HEIGHT_LEPTONS = 104` and `ground_height_leptons` reproduce the verified base, low-byte local XY, record coefficients, clamp, and signed chop for slopes 0..20. `src/sim/cell_kernel.rs::cell_floor_height` already routes through it.

Production/read-path users currently on the correct 104 evaluator include:

- `src/sim/particles/spark_world.rs:96,119` — Spark candidate and constructor ground;
- `src/sim/anim_class.rs:484` — Anim ground;
- `src/render/radar_visibility.rs:127` — radar height sample;
- `src/sim/combat/combat_aoe.rs:180,808,953,1049` — area-damage/impact geometry;
- `src/sim/combat/base_defense_response/admission.rs:105`;
- `src/sim/combat/mod.rs:1398`;
- `src/sim/combat/in_range.rs:41` — InRange terrain ground;
- `src/sim/miner/miner_system.rs:59`;
- `src/sim/overlay_grid.rs:395`;
- `src/sim/combat/smudge_dispatch.rs:404,750`;
- `src/sim/radiation.rs:100`;
- `src/sim/runtime.rs:559`;
- `src/sim/world/lifecycle.rs:392,1374`;
- `src/sim/movement/tube_movement.rs:659`;
- `src/sim/cell_kernel.rs:100`.

Direct multiplication users of `GROUND_LEVEL_HEIGHT_LEPTONS`/`LEPTONS_PER_LEVEL` also use 104. Their caller-specific omission status is outside this scalar correction, but their numeric domain must not be changed to 90.

### 11.2 False 90 helper and all production consumers

`src/util/lepton.rs` currently declares:

```rust
pub const CELLCLASS_GROUND_LEVEL_HEIGHT_LEPTONS: i32 = 90;
```

and constructs a second slope table plus `cellclass_ground_height_leptons`. This was introduced by commit `26a4da9e` (`Match gamemd slope-aware radar click target...`). The parent revision correctly stated that the Cell initializer independently resolved to 104. The change hardened the stale Spark report's false premise into code.

Every current production use of the wrong helper is:

| Rust consumer | Native role | Current mismatch |
|---|---|---|
| `src/render/minimap_interaction.rs:130` | `CellClass::Get_Center_Coords` for radar click | level/slope Z uses 90 instead of 104 |
| `src/sim/naval_base_placement.rs:174` | candidate Cell center distance point | candidate Z uses 90 instead of 104 |
| `src/sim/projectile.rs:241` | allocated real `CellClass::GetTargetCoords` | ground uses 90; conditional +416 is correct |
| `src/sim/projectile.rs:284` | shared dummy `CellClass::GetTargetCoords` | dummy ground uses 90; conditional +416 is correct |
| `src/sim/production/production_spawn.rs:637` | spawn Cell center evaluation | evaluator uses 90 instead of 104, but the current helper discards center Z and returns only the X/Y-derived Cell; the correction preserves output coordinates while removing the false domain and retaining the unsupported-slope gate |

Tests and prose that encode the same false values occur in:

- `src/util/lepton.rs:480..500`;
- `src/sim/projectile.rs:1298..1331`;
- `src/sim/bridge_specs.rs:1779`;
- `src/sim/combat/combat_tests.rs:7409`;
- `src/sim/snapshot.rs:5508,5564` and the expected-ground helper at `5376`;
- `src/sim/world/lifecycle_tests.rs:4349`;
- `src/sim/world/world_tests.rs:2777,2988`;
- `src/sim/world/bridge_orchestrator.rs:3128`;
- the recent naval-base design/fixtures that cite a 90-lepton candidate.

For flat level 2, current bad output is `180`; native is `208`. With a structural deck, current bad target is `596`; native is `624`. With Sonic's `+50`, bad tests expect `646`; native composition is `674`.

### 11.3 Mechanism matrix

| Mechanism | Native | Current Rust | Verdict |
|---|---|---|---|
| Cell Level scalar | independent global, value 104 | common 104 plus false duplicate 90 | FAIL where duplicate is used |
| Cell slope records | `G=104`, indices 0..20 | common table with 104 is exact; duplicate table uses 90 | preserve common, remove/fix duplicate |
| signed negative base | `ftol(level*104+0.5)` | common returns `-103` for -1 | PASS common; FAIL duplicate (`-89`) |
| local slope XY | unsigned low bytes | common and duplicate both use low bytes | PASS operation shape |
| ground bridge inclusion | none | common floor excludes bridge | PASS |
| bridge-aware Cell target | ground +416 iff raw structural bit | projectile composes helper +416 | predicate/offset PASS; ground FAIL |
| Spark ground | same Cell wrapper, 104 | common 104 | PASS |
| Spark plane | ground +416 | ground +416 | PASS numeric |
| signed world-to-cell | trunc toward zero | Spark uses shared trunc helper | PASS |
| flattened 512 lookup | flattened range/pointer; dummy fallback | Spark uses the shared fixed-stride fallback and preserves valid aliases | PASS for Spark |
| shared dummy ground | live dummy level/slope | Spark consumes the terrain-bound dummy; other height callers remain separately owned | PASS for Spark |
| INI override | none | constants | PASS conceptually |

## 12. Final questions log

| ID | Disposition |
|---|---|
| Q01 | **RESOLVED:** live `0x0089E7C0 = 104`. |
| Q02 | **RESOLVED:** `0x0047B220..0x0047B24B` calculates and writes 104. |
| Q03 | **RESOLVED:** `90.0 @ 0x007E1730` is degrees; it becomes radians and is subtracted from the 60-degree angle. |
| Q04 | **RESOLVED:** Cell and VXL use separate globals/initializers but both resolve to 104. There is no 90 numeric domain. |
| Q05 | **RESOLVED:** signed i8 Level, unsigned u8 slope, signed i32 world XY/Z leptons, unsigned low-byte local XY. |
| Q06 | **RESOLVED:** add 0.5 then chop; level -1 flat gives -103. |
| Q07 | **RESOLVED:** 20 records plus flat 0, clamp slope before base, then final chop; >20 is unsafe native malformed state. |
| Q08 | **RESOLVED:** biased arithmetic shift equals signed `/256` truncation toward zero. |
| Q09 | **RESOLVED:** flattened 512-wide validation; failure restamps and evaluates shared dummy. |
| Q10 | **RESOLVED:** ground sampler returns floor only. |
| Q11 | **RESOLVED:** `CellClass::GetTargetCoords @ 0x00486890`, not a proven `MapClass::GetZPos`. |
| Q12 | **RESOLVED:** Cell target offset `0x0089E7B4 = 416`, gated by raw flag `0x100`. |
| Q13 | **RESOLVED:** Spark calls Cell ground (104) and adds `0x00AC4A0C = 416`. |
| Q14 | **RESOLVED:** constructor calls wrapper twice around the `input_z <= ground` clamp. |
| Q15 | **RESOLVED:** named Cell/Foot/Techno/AoE/Anim peers and Bullet/Particle/Unit/VXL peers were captured; all level scalars are 104 and all examined four-level offsets are 416. |
| Q16 | **RESOLVED:** executable initializer constants, no INI writer. |
| Q17 | **RESOLVED:** no verified active `MapClass::GetZPos` symbol; use the proven Cell methods. |
| Q18 | **RESOLVED:** complete 185 wrapper and 32 inner direct-xref census is in §9. |
| Q19 | **RESOLVED:** five production callsites use the false helper, plus named test/prose fixtures. |
| Q20 | **RESOLVED:** Spark and the common sim consumers use the correct 104 evaluator; list in §11.1. |
| Q21 | **RESOLVED:** implementation handoff and acceptance tests are in §14. |

## 13. Adversarial review and zero-add pass

### Adversarial questions

1. **Could live memory be modified by a mod or trainer while static retail really says 90?** No. The active executable's own static initializer writes `0x0089E7C0`, and its FPU expression resolves to 104. Live memory independently agrees.
2. **Could Cell use 90 while the captured 104 belongs only to a later bridge helper?** No. `0x0047BA9B` inside the active inner evaluator directly multiplies Level by `0x0089E7C0`; slope setup derives from the same global.
3. **Could Spark use a hidden 90 table despite calling the wrapper?** No. `0x0062C7D4` calls `0x00578080`; that wrapper calls `0x0047B3A0`. Spark's only nearby height constant is its 416 bridge offset derived from Particle's 104.
4. **Could 90 be a map/theater override that the captured session happened not to use?** No. the writer is a fixed executable geometry initializer with no INI/map input, and its direct xref set contains no parser.
5. **Could independently named globals justify keeping separate Rust constants?** Separate names may document ownership, but giving one the value 90 is false. If separate aliases remain, every active-retail value and evaluator output must still be 104-identical.

### Zero-add cold reread

After drafting the findings, two load-bearing paths were reread without adding new hypotheses:

- static Cell chain `0x0047B1E0..0x0047B24B` plus live `0x0089E7C0/0x0089E7B4`;
- Spark `0x0062C7D4..0x0062C81C` plus live `0x00AC4A18/0x00AC4A0C`.

Both reproduced 104 and 416. The current Ghidra plate comments on `CellClass::GetCoords @ 0x00486840`, `GetTargetCoords @ 0x00486890`, and inner `0x0047B3A0` also state 104 and agree with the bytes. No new mechanism appeared in the cold reread.

## 14. Implementation handoff

### Required code delta

1. Remove the false 90-lepton semantic split in `src/util/lepton.rs`.
   - Prefer one 104 ground evaluator/table for all Cell ground users.
   - If ownership-specific aliases remain for documentation, assert they equal 104 and route both APIs through the same records/evaluator.
   - Correct comments that say Cell uses 90.
2. Correct all five production users in §11.2. A constant-only change may be sufficient numerically, but the builder should avoid retaining duplicate tables that can drift again.
3. Preserve Spark's current 104 `ground_height_leptons` call and 416 structural plane.
4. Rebaseline all 90-derived tests, snapshot hashes, wave target expectations, and recent design prose.
5. Spark-specific shared fixed-stride/dummy-cell routing is completed by
   `4c71b488` and its review repairs. Other height callers remain separately
   owned and must not infer parity from Spark's closure.
6. Keep floor and deck composition separate. Do not add 416 inside `ground_height_leptons`.

### Acceptance fixtures

At minimum:

- flat levels: 0 -> 0, 1 -> 104, 2 -> 208, `0xFF` -> -103;
- all slope 0..20 center contributions exactly match §5.2 with `G=104`;
- representative boundary coordinates use low unsigned bytes and preserve clamp-before-base/final chop;
- `GetCoords` equivalent at flat level 2 returns Z 208;
- `GetTargetCoords` equivalent at the same cell returns 208 without structural bit and 624 with bit `0x100`;
- dummy level `0xFF`, flat returns -103 and retains requested packed coordinate;
- Spark level-2 flat ground is 208, bridge plane 624, ascending snap 604; level-0 plane remains 416;
- Particle constructor clamps input `Z <= ground` to the 104-based ground;
- radar click, naval-base distance, production's intermediate Cell-center evaluation, projectile real target, and projectile dummy target all share the 104 evaluator;
- production's current X/Y-only returned spawn Cell remains unchanged because its intermediate center Z is discarded;
- Sonic target composed over level-2 structural cell is `208 + 416 + 50 = 674`;
- changing structural bridge state changes only the +416 selection, never the ground scalar;
- focused tests prove no production identifier or assertion still describes Cell terrain as 90 leptons.

### Do not implement

- Do not change Spark ground from 104 to 90.
- Do not use the degree literal as a height.
- Do not fold bridge height into the ground evaluator.
- Do not replace signed Level with unsigned arithmetic.
- Do not mathematically floor negative world axes; native truncates toward zero.
- Do not independently reject X/Y before the flattened 512-wide lookup where exact Cell lookup is required.
- Do not emulate native out-of-bounds slope memory reads.

### Expected player-visible triggers

Severity is high whenever nonzero elevation participates:

- radar clicks and action targets on raised terrain;
- projectile/cell target coordinates and Sonic/wave endpoints on raised or bridged cells;
- production's intermediate Cell-center evaluation on raised cells (the current Rust helper discards that Z before returning its X/Y-only spawn Cell, so this scalar correction alone does not move the unit);
- naval-base first-yard 3D distance checks near elevation changes;
- any test/snapshot derived from those values.

Flat level-0 maps hide the scalar regression because both domains return zero there. Slopes can expose it even at level 0.

## 15. Documentation corrections and annotation candidates

### Documents that contain load-bearing stale claims

`docs/research/PARTICLE_SPARK_LIVE_COLLISION_INPUTS_GHIDRA_REPORT.md`:

- replace every `Cell LevelHeight = 90` statement with `Cell LevelHeight = 104`;
- replace slope-table `L=90` and slopes 17..20 contribution 45 with `L=104` and contribution 52;
- replace Spark plane `G+360` and ascending `G+340` with `G+416` and `G+396`;
- delete the claim that VXL 104 and Cell ground 90 are different numeric domains; retain only that their globals/initializers are independently owned.
- replace its stale pre-implementation claim that Spark returns typed unavailable/off-array errors: the substrate exists in `cell_rect`, and Spark routing is implemented in `4c71b488` with review repairs through `0054549e`.

`docs/research/PARTICLE_SPARK_COLLISION_AND_PIXEL_COMPOSITOR_GHIDRA_REPORT.md`:

- mark its HIGH-confidence 90/360/340 conclusions superseded by this live runtime capture, or correct every affected table, trace, questions-log entry, and Rust handoff to 104/416/396;
- retain its independently verified float integration, collision inequalities, RNG, and compositor findings.

`docs/research/PHASE3_NAVAL_BASE_PLACEMENT_LIFECYCLE_GHIDRA_REPORT.md`:

- replace the claim that 104-lepton Cell ground is wrong with the verified 104 evaluator;
- replace its `cellclass_ground_height_leptons` Rust handoff with the common `ground_height_leptons` authority.

`docs/plans/2026-07-18-spark-native-float-and-point-compositor-design.md`:

- replace candidate ground `level*90` with the verified 104-lepton Cell evaluator;
- replace structural plane `G+360` / ascending commit `G+340` with `G+416` / `G+396`.

`docs/plans/2026-07-18-spark-live-collision-adapter-and-owner-design.md`:

- replace its retained `G+360` / ascending `G+340` adapter contract with the verified `G+416` / `G+396` composition;
- state explicitly that the Spark ground evaluator uses the correct 104 scalar and shared-dummy routing is implemented; use the current Spark routing contract for status.

`docs/plans/2026-08-26-naval-base-placement-design.md` and any plans/tests copied from it:

- replace the 90-lepton candidate Cell center with 104.

Older bridge plans that explicitly label 90 as approximate or unverified should be marked superseded by this live capture rather than used as implementation evidence.

### Ghidra annotation candidates

No Ghidra mutation was made. Current comments on `0x0047B3A0`, `0x00486840`, and `0x00486890` are already correct. Remaining stale particle-function comments, if any, should state:

```text
Spark calls CellClass::GetGroundHeight, whose active Cell scalar is 104.
DAT_00AC4A0C is the Particle module's independently derived four-level
structural bridge offset, 416. Bridge plane = ground + 416.
```

Candidate names for currently unnamed peer globals, subject to the project's annotation sync policy:

- `0x00AC4A18`: `g_nParticleLevelHeightLeptons`
- `0x00AC4A0C`: `g_nParticleStructuralDeckOffsetLeptons`
- `0x0089DE70`: `g_nBulletLevelHeightLeptons`
- `0x0089DE64`: `g_nBulletStructuralDeckOffsetLeptons`
- `0x00B1D0B8`: `g_nUnitLevelHeightLeptons`
- `0x00B1D0AC`: `g_nUnitStructuralDeckOffsetLeptons`

## 16. Sources

### Fresh active-binary operations

- `disassemble_bytes 0x0047B170..0x0047B310`
- `decompile_function 0x0047B3A0`
- `get_xrefs_to 0x0047B3A0` (32 results)
- `get_xrefs_to 0x00578080` (185 results)
- `decompile_function 0x00486840`
- `decompile_function 0x00486890`
- `decompile_function 0x0062B5E0`
- `decompile_function 0x0062C6E0`
- `disassemble_bytes 0x0062B520..0x0062B580`
- `disassemble_bytes 0x005F3830..0x005F38A0`
- `decompile_function 0x007549E0`
- direct-xref queries for every scalar address listed in §§4 and 8
- Ghidra symbol searches for `GetZ`, `LevelHeight`, and `Deck`

### Fresh active-runtime reads

Read-only process-memory capture of every address in §4.2 plus Cell initializer intermediates `0x0089E728`, `0x0089E758`, and `0x0089E750`.

### Current Rust sources

- `src/util/lepton.rs`
- `src/sim/cell_kernel.rs`
- `src/sim/cell_rect.rs`
- `src/sim/particles/spark_world.rs`
- `src/sim/particles/spark.rs`
- all `ground_height_leptons` and `cellclass_ground_height_leptons` callsites found by repository-wide `rg`
- git history for commit `26a4da9e`

### Existing leads inspected but not trusted without fresh verification

- `docs/research/PARTICLE_SPARK_LIVE_COLLISION_INPUTS_GHIDRA_REPORT.md`
- `docs/plans/2026-07-18-spark-native-float-and-point-compositor-design.md`
- bridge height and Cell target-coordinate reports referenced by the research index
