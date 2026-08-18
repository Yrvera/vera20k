# Spatial Primitives Layer — gamemd.exe (Ghidra Research Report)

**Date:** 2026-05-07
**Confidence:** HIGH for all 12 categories below — every section cites verified
decompilation, prior research docs, or both. Numbered Ghidra addresses are functions /
data the layer's consumers reach for; in-text references in `[doc.md]` form point to the
authoritative deep-dives.
**Active in YR:** Yes — every category listed here runs on essentially every tick, frame,
input event, or fire decision.

---

## What this layer is

The **spatial primitives layer** is the foundational set of types, constants, and pure
functions that define *the language every other system speaks when it talks about "where"
and "how far"*. Three qualities make something belong here:

1. **It's a unit / convention, not a behavior** — leptons-per-cell, facing-byte direction,
   fixed-point scale, iso projection ratio. Pure definitions; no gameplay logic, no state.
2. **Everything reads it; it reads nothing.** Combat, pathfinding, vision, render, UI,
   audio panning all depend on it. Bottom of the stack.
3. **Bugs are silent and compounding.** A wrong lepton constant or facing offset doesn't
   crash, doesn't fail a test — it just makes every downstream system 2% off, and the game
   "feels wrong" with no single bug to point at.

The layer matters disproportionately for the 99%-parity bar: small drifts here multiply
across combat, movement, and rendering simultaneously.

---

## 1. Coordinate data types

| Type | Layout | Where stored | Source |
|------|--------|--------------|--------|
| **CoordStruct** | 12 bytes: 3× `int32` X/Y/Z in **leptons** | Every `ObjectClass` instance at `+0x9C / +0xA0 / +0xA4` | [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §4](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) |
| **CellStruct** | 2× `int16` X/Y, packed cell index | Used in cell-grid math | [CELLCLASS_STRUCT_GHIDRA_REPORT.md](CELLCLASS_STRUCT_GHIDRA_REPORT.md) |
| **Local index** | Skewed integer grid for `LocalSize` iteration | Internal — see §11 | [COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md §1](COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md) |

Key fact: `Coords.Z` at `ObjectClass+0xA4` is the **absolute Z in leptons**, not a level
count. Aircraft store altitude here too — there is no separate altitude field that
distance / fire / render code consults. Locomotor code is responsible for keeping `+0xA4`
updated as a unit climbs / drops.

---

## 2. Unit constants

| Constant | Value | Address | Verified in |
|----------|-------|---------|-------------|
| LEPTONS_PER_CELL | 256 (`0x100`) | — | universal |
| CELL_CENTER_LEPTON | 128 | — | [INFANTRY_SUBCELL_POSITIONING.md](INFANTRY_SUBCELL_POSITIONING.md) |
| TILE_WIDTH_PX | 60 (`0x3C`) | — | [COORDINATE_SYSTEM_GAMEMD.md](COORDINATE_SYSTEM_GAMEMD.md) |
| TILE_HEIGHT_PX | 30 (`0x1E`) | — | [COORDINATE_SYSTEM_GAMEMD.md](COORDINATE_SYSTEM_GAMEMD.md) |
| HEIGHT_STEP_PX | 15 (`0x0F`) | — | [COORDINATE_SYSTEM_GAMEMD.md](COORDINATE_SYSTEM_GAMEMD.md) |
| **LevelHeight** | 104 leptons (= cot60° × 256√2 × 0.5) | `0x89DDB8`, init at `0x45B080` | [COORDINATE_SYSTEM_GAMEMD.md:127](COORDINATE_SYSTEM_GAMEMD.md) |
| AdjustForZ multiplier | ≈ 0.14348 (= sin60° × pixel_per_lepton) | `0xB0CD48` | [COORDINATE_SYSTEM_GAMEMD.md](COORDINATE_SYSTEM_GAMEMD.md) |
| AdjustForZ +1px threshold | 728 leptons (≈ 7 levels) | hardcoded | [COORDINATE_SYSTEM_GAMEMD.md](COORDINATE_SYSTEM_GAMEMD.md) |
| BridgeHeightInLeptons | runtime-init (likely 4 × 104 = 416) | `DAT_00B0EB24`, also `DAT_00B0EC2C` | [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §6/§7](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) |
| HighFlightLevel | runtime-init from `Rules.FlightLevel=` | `DAT_00AC13C8` | [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §4.3](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) |
| **Rules.Gravity** | `[AudioVisual]/Gravity=` | `Rules+0x16B8` (parsed at `0x66B3D9`) | [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §0](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) |
| **Rules.ElevationIncrement** | `[ElevationModel]/ElevationIncrement=` | `Rules+0x1838` (parsed in `FUN_0066D150`) | [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §0](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) |

---

## 3. Forward projection (lepton → screen)

| Address | Function | Purpose |
|---------|----------|---------|
| `0x6D1EB0` | `Tactical::WorldToScreenSub` | Pure iso, no Z, no scroll |
| `0x6D1FE0` | `TacticalClass::CellToPixel` | Same — alias |
| `0x6D1F10` | `CoordsToClient` | Adds Z and viewport scroll |
| `0x6D2140` | `TacticalClass::CoordsToClient2` | + camera offset `+0xB0/+0xB4`, returns visibility |
| `0x6D20E0` | `AdjustForZ` | Z leptons → screen Y px |

Formula:
```
screen_x = (lepton_x − lepton_y) × 60/2 / 256 = (lx − ly) × 30 / 256
screen_y = (lepton_x + lepton_y) × 30/2 / 256 − AdjustForZ(Z)
AdjustForZ(z) = ftol(z × 0.14348 + (z >= 728 ? 1 : 0) + 0.5)
```
Full deep-dive: [COORDINATE_SYSTEM_GAMEMD.md](COORDINATE_SYSTEM_GAMEMD.md), [COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md](COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md).

---

## 4. Inverse projection (screen → cell)

`0x6D6590` is the real one — not a simple inverse:

1. Apply `Matrix3x4_TransformPoint` (camera matrix inverse).
2. Floor to cell (÷256).
3. Iterate up to **180 times** (`0xB3` cap): re-correct Y by resolved cell's height level,
   re-transform.
4. If resolved cell has bridge flag (`+0x140 & 0x100`), check 4 cardinal neighbors and
   shift along bridge direction (15px threshold).

Adds `g_RadarViewportOffsetX/Y` before transform — important near sidebar boundary.

---

## 5. Z-axis / height system

- **Height level**: signed byte at `CellClass+0x11B`, range typically 0-14.
- **Height in leptons**: `level × 104 + bridge_offset`.
- **Cell flags** at `+0x140`:
  - `0x100` = bridge present
  - `0x40000` = altered passability
- **Layered cell state** (ground / bridge):
  - Occupant list: `+0xE4` / `+0xE8`
  - Occupancy byte: `+0x124` / `+0x128`
  - Owner house ID: `+0x54` / `+0x58`
- `IsLowFlying` / `IsHighFlying` (`0x5F6B60` / `0x5F6B90`): split at `HighFlightLevel × 2`,
  both gated on `byte+0x74` (airborne flag).

---

## 6. Distance functions

| Variant | Formula | Used by |
|---------|---------|---------|
| **3D Euclidean (default)** | `(int)Sqrt_Approx(dx² + dy² + dz²)` | `InRange` Branch A2, `CoordStruct::Distance3D` (`0x41C380`) |
| 2D Euclidean | `(int)Sqrt_Approx(dx² + dy²)` | `InRange` Branch B (ballistic / AA arc), Branch A1 (dead in YR — see §10) |
| Cell distance | Chebyshev / Manhattan via cell deltas | Sight, sensor, "near" checks |

- `Sqrt_Approx @ 0x4CAC40` is **NOT precise** — float32-grade mantissa LUT at
  `DAT_008650BC`. Up to ±1 lepton drift at large distances.
- `Math::ftol @ 0x7C5F00` — x87 truncation toward zero.
- **Boundary semantics**: max-range inclusive (`<=`), min-range strict (`<`).
- **Sentinel**: `weapon.Range == -0x200` ⇒ always-in-range.

Full deep-dive: [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md).

---

## 7. Anchor / origin getters (the "where is this thing?" vtable layer)

| Class | Slot | Address | Returns |
|-------|------|---------|---------|
| `ObjectClass::GetCoords` | vtable+0x48 | `0x5F65A0` | Raw `+0x9C/+0xA0/+0xA4` |
| `BuildingClass::GetCoords` | vtable+0x48 | `0x447AC0` | Foundation **center**: `Location + ((fh-1)×128, (fw-1)×128, 0)` |
| `BuildingClass::GetTargetCoords` | vtable+0xA4 | `0x4500A0` | GetCoords + `TargetCoordOffset` (non-zero only on NAYARD/GAYARD/YAYARD) |
| `BuildingClass::GetCell` | vtable+0x1B8 | `0x41BEA0` | NW corner cell number from Location |
| `FootClass::GetDestinationCoords` | vtable+0x4C | — | Movement destination, separate slot |

Building Location is itself the **NW corner cell-center leptons** (`cell × 256 + 128`).

---

## 8. Direction / facing primitives

| Table | Address | Init | Contents |
|-------|---------|------|----------|
| 8-dir **cell** offsets | `0x89F688` | runtime (`0x49F2F0`) | N(0,-1), NE(1,-1), E(1,0), SE(1,1), S(0,1), SW(-1,1), W(-1,0), NW(-1,-1) |
| 8-dir **lepton** offsets | `0x89F6D8`, `0x89F6DC` | runtime | dx/dy in leptons |
| Drive **track** waypoints | `0x7E7A28` | static | dx, dy, heading per step (12B) |
| Drive **track** flags | `0x7E7B30` | static | bit1=swap XY/+0x40 heading; bit2=neg X; bit4=neg Y/-0x80 heading; bit8=cell trigger |
| Foundation 8-dir (placement extension) | `0x89F688` | runtime | Same as above (shared) |

- **Facing convention**: 8-bit byte, 0=N, increasing clockwise (64=E, 128=S, 192=W).
  Some paths use 16-bit `facing << 8`.
- `atan2` helper at `0x4CAE30` (generic, not Fly-specific).

Full deep-dive: [LOCOMOTION_MATH_AND_CONSTANTS.md](LOCOMOTION_MATH_AND_CONSTANTS.md).

---

## 9. Sub-cell positioning (infantry — 3 per cell)

| Table | Address | Init |
|-------|---------|------|
| Sub-cell lepton offsets (5×3 ints) | `0x89E9F0` | runtime (`0x48E480`) |
| Preference table (5×4 bytes) | `0x81CC84` | static |
| Random rotation table (4×4 bytes) | `0x81CC98` | static |
| Failure coords (0,0,0) | `0x89E778` | runtime (`0x47B300`) |

| Idx | Lepton (X,Y) | Screen offset | Status |
|-----|--------------|---------------|--------|
| 0 | (128,128) | (0,0) | Center, also "NW quadrant fallback" |
| 1 | (64,64) | (0,−7.5) | **Dead — never assigned** |
| 2 | (192,64) | (+15,0) | NE |
| 3 | (64,192) | (−15,0) | SW |
| 4 | (192,192) | (0,+7.5) | SE |

Quadrant-from-leptons: `0x4810A0`. Placement entry: `0x481180` (20 callers).
Mark/unmark occupancy via vtable+0xF0/+0xF4 (per-class). Occupancy byte bits 0-4 =
sub-cells, 0x20 = vehicle, 0x40 = building.

Full deep-dive: [INFANTRY_SUBCELL_POSITIONING.md](INFANTRY_SUBCELL_POSITIONING.md).

---

## 10. Range bonus chain (effective range)

Running sum applied inside `InRange` (`0x6F7220`):

1. `weapon.Range` (`+0xB4`)
2. (sentinel) `−0x200` ⇒ always-in-range
3. `+ AirRange` if target high-flying
4. **Replaces with** `(occupant_count + Rules.OccupyWeaponRange) × 256` if attacker garrisoned
5. `+ Rules.BunkerWeaponRangeBonus × 256` if bunkered
6. `+ Rules.OpenToppedRangeBonus × 256` if open-topped passenger
7. `+ height-fire bonus` if `weapon.Projectile.SubjectToElevation = yes` (`+0x297` on
   BulletTypeClass) — uses `(target_height − attacker_height) / Rules.ElevationIncrement`
   plus a ballistic distance term. **High-ground advantage. Active in YR.**
8. `+ (FoundationH + FoundationW) × 64` if target is a building (Branch A only)
9. `+ height-fire bonus (Branch B variant)` if `Projectile.SubjectToElevation` and
   `Projectile.Arcing = yes` — same height delta, no ballistic term.

Branch selection: `Projectile.Arcing` (`+0x29B`) gates Branch B (2D + ballistic-arc).
`Projectile.Floater` (`+0x295`) gates a TS-legacy alternate-gravity path inside Branch B
(no standard YR projectile sets `Floater=yes`).

**Branch A1 (`attacker.WhatAmI() == 3`, 2D-Euclidean) is dead code in YR.** Only
`*TypeClass` templates inherit `WhatAmI() == 3` from `AbstractTypeClass`; TechnoClass
instances on the map return 1 (Unit), 2 (Aircraft), 6 (Building), or 0xF (Infantry).
Branch A1 cannot be reached.

Full deep-dive: [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §3-§5](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md).

---

## 11. Local-grid / cell skews (NOT screen math)

`0x5654A0` / `0x565520` / `0x565660` — `local_index ↔ cell` skew transforms. Only callers:
`HouseClass::DetermineEdge` (start-edge calc) and `FUN_004AA440` (placement search). An
earlier doc mis-labeled these as world↔cell tactical math; they aren't — they're internal
helpers for iterating rectangles aligned to the map's `LocalSize` edge.

Source: [COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md §1](COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md).

---

## 12. Pathfinding-cost primitives

- Cliff cost factor: 4.0
- Bridge-aware mode cost: 1000.0
- Altered-passability multiplier: `DAT_007E37BC`
- Diagonal cost lookups: `DAT_007E3710`, `DAT_007E3730`, base costs at `DAT_0081872C`
- Direction offsets: `DAT_007E3774`
- Neighbor expansion: 8 compass + tunnel (dir 8) = 9
- Cliff threshold: 4 height-level diff = impassable

Full deep-dive: [LOCOMOTION_MATH_AND_CONSTANTS.md §9](LOCOMOTION_MATH_AND_CONSTANTS.md), [TERRAIN_COST_FACTSHEET.md](TERRAIN_COST_FACTSHEET.md), [ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md](ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md).

---

## What's outside this layer (intentionally)

These systems *consume* the spatial primitives but aren't part of them:

- Locomotor state machines, speed/accel ramping ([LOCOMOTION_MATH_AND_CONSTANTS.md](LOCOMOTION_MATH_AND_CONSTANTS.md))
- Pathfinding A* itself (consumer of §12)
- Weapon-fire targeting (consumer of §6, §10)
- Render order / depth (consumer of §3, §5)
- Bridge state machine, ramp transitions (consumer of §5)

If a "what's wrong here?" investigation lands at a coordinate / facing / distance bug,
this layer is where to look. If it lands at a per-system gameplay bug, the consumer is
where to look.

---

## Bridge-LOS gate (a consumer worth knowing about here)

`InRange` at `0x6F75FB-0x6F762F` enforces a **bridge LOS occlusion** rule that lives in
the spatial layer's vocabulary even though it's a fire-decision: if attacker is in a
bridge cell, `attacker.Z < bridge_top`, and `target.Z >= bridge_top`, the shot is
rejected. Plain meaning: a unit on the ground beneath a bridge cannot fire upward through
the deck. This fires in standard YR every match with a bridge.

Full deep-dive: [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §6](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md).

---

## Open questions (low priority, not load-bearing for the layer's structure)

- **Exact runtime values** of `DAT_00AC13C8` (HighFlightLevel), `DAT_00B0EB24`
  (BridgeHeightInLeptons), `DAT_00B0EB34` (ballistic-elevation scalar). Semantic meanings
  are clear; numeric values would need a debugger session or further static-trace work.
  Best derived in the Rust port from `rulesmd.ini` and known RA2/YR defaults
  (BridgeHeight = 4 × 104 = 416 leptons; HighFlightLevel maps to `Rules.FlightLevel=` at
  `Rules+0x7B4`).
- **`FootClass::IsHighFlying` actually-overrides?** Cosmetic doc question at `0x004DE620`.
- **Sqrt_Approx ±1-lepton drift threshold** — when does it become observable? Parity
  testing would resolve this; not a blocker.

---

## Sources

- **Ghidra decompilation (this session, 2026-05-07):**
  - All four addresses returning 3 from `WhatAmI()` (`0x0041CFB0`, `0x004369F0`,
    `0x0062D770`, `0x0074A960`) confirmed via byte-pattern search and vtable+0x2C xrefs.
  - `0x0041CFB0` identified as `AircraftTypeClass::WhatAmI` via RTTI chain at
    `0x007FB5B0` → TypeDescriptor `0x00817FB8` → string `.?AVAircraftTypeClass@@`.
  - `WeaponTypeClass+0xA0` confirmed = Projectile (BulletTypeClass*), per
    [WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md](WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md).
  - `BulletTypeClass+0x294-0x29F` flag offsets confirmed via
    [BULLETTYPECLASS_GHIDRA_REPORT.md](BULLETTYPECLASS_GHIDRA_REPORT.md).
  - `Rules+0x16B8 = Gravity` (`[AudioVisual]`) found at `RulesClass::ReadAudioVisual`
    `0x66B3D9` (string `0x83A34C`).
  - `Rules+0x1838 = ElevationIncrement` (`[ElevationModel]`) found in `FUN_0066D150`
    (string `0x83B370`).
  - `FUN_006F6F60` / `FUN_006F70E0` decompiled — both compute height-fire bonuses
    using `Rules.ElevationIncrement`.
  - Bridge-LOS gate re-disassembled at `0x6F75FB-0x6F762F`.

- **Existing docs incorporated** (in approximate dependency order):
  - [COORDINATE_SYSTEM_GAMEMD.md](COORDINATE_SYSTEM_GAMEMD.md) — CoordsToClient pipeline,
    AdjustForZ, HeightFactor.
  - [COORDINATE_ATOMS_AUDIT.md](COORDINATE_ATOMS_AUDIT.md) — atom-by-atom Rust↔gamemd
    comparison.
  - [COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md](COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md) —
    forward / inverse projection, local-index skews (§11).
  - [LOCOMOTION_MATH_AND_CONSTANTS.md](LOCOMOTION_MATH_AND_CONSTANTS.md) — direction
    tables, locomotor CLSIDs, pathfinding costs.
  - [INFANTRY_SUBCELL_POSITIONING.md](INFANTRY_SUBCELL_POSITIONING.md) — sub-cell tables,
    preference search, mark/unmark protocol.
  - [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md)
    — distance / range / bonus / LOS gate (with 2026-05-07 corrections).
  - [BULLETTYPECLASS_GHIDRA_REPORT.md](BULLETTYPECLASS_GHIDRA_REPORT.md) — projectile
    flag offsets.
  - [WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md](WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md) —
    weapon offsets (Projectile = +0xA0; Warhead = +0xAC).
  - [WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md](WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md) —
    warhead struct (relevant for cross-checks; warhead is NOT what gates the InRange
    branches despite an earlier claim to the contrary).
  - [CELLCLASS_STRUCT_GHIDRA_REPORT.md](CELLCLASS_STRUCT_GHIDRA_REPORT.md) — cell flags,
    height byte, occupant lists.
  - [BRIDGE_RENDERING_GHIDRA_REPORT.md](BRIDGE_RENDERING_GHIDRA_REPORT.md), [BRIDGE_SYSTEM.md](BRIDGE_SYSTEM.md) — bridge geometry
    (consumer of §5 + §11.5 LOS gate).
