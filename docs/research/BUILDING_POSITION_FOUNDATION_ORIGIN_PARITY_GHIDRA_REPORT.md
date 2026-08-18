# Building Position Foundation-Origin Parity - Ghidra Research Report

**Address(es):** `0x00440580`, `0x005F6940`, `0x00447AC0`, `0x0041BEA0`, `0x004500A0`, `0x007393C0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Building stored/live coordinate semantics relative to foundation origin/center, and how map/ready-placement and `DeploysInto=` placement feed that coordinate.  
**Non-Scope:** General placement validation, AddOccupy/RemoveOccupy shape correctness, visual draw offsets beyond coordinate getters, and full deploy/undeploy lifecycle.  
**Confidence:** High for stored coordinate, coordinate getters, and UnitClass deploy coordinate transformation; Medium for map loader equivalence because this pass verified the generic Unlimbo storage path and current Rust map parser, not every scenario-load caller.  
**Active in YR:** Yes.

## 1. Overview

Buildings store their live `ObjectClass` coordinate at byte offsets `+0x9C/+0xA0/+0xA4`. For buildings, that stored coordinate is the foundation origin cell center: `origin_cell * 256 + 128`, not the geometric foundation center. Some APIs intentionally project from that origin: `GetCell` returns the origin cell, while `BuildingClass::GetCoords` returns a foundation-center coordinate derived from the origin and foundation dimensions.

Map/ready placement reaches `BuildingClass::Unlimbo`, then `TechnoClass::Unlimbo`, then `ObjectClass::Reveal`, then vtable slot `+0x1B4`, which for `BuildingClass` resolves to `ObjectClass__Set_Raw_Coords @ 0x005F6940`. That setter copies the caller-provided coordinate directly into `+0x9C/+0xA0/+0xA4`.

## 2. Key Offsets / Slots

| Field / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Object `+0x9C/+0xA0/+0xA4` | Stored `CoordStruct` for all ObjectClass descendants; for buildings this is foundation-origin cell center | `ObjectClass__Set_Raw_Coords @ 0x005F6940` copies three dwords from input coord | Yes - called through `ObjectClass::Reveal` during Unlimbo |
| Building vtable `+0xD8` | `BuildingClass__Unlimbo` | vtable data `0x007E3F94 -> 0x00440580` | Yes - `UnitClass::Deploy @ 0x007393C0` calls new building vtable `+0xD8` |
| Building vtable `+0x48` | `BuildingClass__GetCoords`, foundation-center projection | vtable data xref `0x007E3F04 -> 0x00447AC0` | Yes - ordinary BuildingClass virtual getter |
| Building vtable `+0x1B8` | `ObjectClass__Get_Cell_Packed`, origin-cell getter | vtable data and forced decompile `0x0041BEA0` | Yes - used by `UnitClass::Deploy @ 0x0073945A` on the unit side and by building callers |
| BuildingType `+0xEF0` | Foundation table index | `BuildingTypeClass__GetFoundationWidth/Height`; INI `Foundation=` parser path | Yes - stock building art uses Foundation values |

## 3. Verified Binary Findings

### 3.1 Stored coordinate is the foundation origin

`ObjectClass__Set_Raw_Coords @ 0x005F6940` writes the input coordinate dwords directly to object `+0x9C`, `+0xA0`, and `+0xA4`. `BuildingClass__Unlimbo @ 0x00440580` calls `TechnoClass__Unlimbo(param_2, ...)`; `TechnoClass__Unlimbo` calls `ObjectClass__Reveal`; `ObjectClass__Reveal` calls vtable `+0x1B4`; `BuildingClass` vtable `+0x1B4` resolves to `ObjectClass__Set_Raw_Coords`.

Active in YR: Yes. Evidence: standard building placement/deploy path reaches `BuildingClass__Unlimbo`; `UnitClass::Deploy @ 0x0073970D..0x00739711` calls the new building's vtable `+0xD8`, which resolves to `0x00440580`.

### 3.2 `GetCell` returns the foundation origin cell

`ObjectClass__Get_Cell_Packed @ 0x0041BEA0` reads `+0x9C/+0xA0`, applies signed `+0xFF` correction for negative values, shifts both by 8, and packs `(x_cell, y_cell)`. Since placed building coordinates are cell centers (`cell*256+128`), this truncates back to the origin cell.

Active in YR: Yes. Evidence: `BuildingClass` vtable `+0x1B8`; `UnitClass::Deploy @ 0x0073945A` uses this getter on the deploying unit to seed the new building placement cell.

### 3.3 `BuildingClass::GetCoords` returns foundation center, not stored origin

`BuildingClass__GetCoords @ 0x00447AC0` computes:

```text
out.x = stored.x + (foundation_width * 128) - 128
out.y = stored.y + (foundation_height * 128) - 128
out.z = stored.z
```

This is `(width - 1)/2` and `(height - 1)/2` cells in 256-lepton units from the stored origin. It is exact for odd dimensions and half-cell centered for even dimensions. Example: a 4x4 building at origin `(rx,ry)` returns `(rx*256+512, ry*256+512)`, i.e. midway between the four center cells.

Active in YR: Yes. Evidence: `0x00447AC0` forced decompile; `BuildingTypeClass__GetFoundationWidth` and `GetFoundationHeight(0)` feed the formula.

### 3.4 `GetTargetCoords` layers TargetCoordOffset on top of foundation center

`BuildingClass__GetTargetCoords @ 0x004500A0` calls vtable `+0x48` first, then adds BuildingType offsets `+0xEBC/+0xEC0/+0xEC4`. This means weapon/target APIs using this function are center-plus-offset, not origin-based.

Active in YR: Yes. Evidence: forced decompile `0x004500A0`; stock naval-yard TargetCoordOffset values exist in prior coordinate report and rules/art data.

### 3.5 `DeploysInto=` uses unit cell plus a coarse NW adjustment, not full foundation-center math

`UnitClass__Deploy @ 0x007393C0` gets the deploying unit's cell through vtable `+0x1B8`, checks the target building's foundation width and height, and builds the new building Unlimbo coordinate from that cell. If target width <= 2 and height <= 2, it keeps the unit cell as the building origin. If either dimension is > 2, it adds `DAT_0089F6A4/6`, the NW direction offset `(-1,-1)`, before converting to cell-center leptons with `cell*256 + 128`.

Active in YR: Yes. Evidence: assembly around `0x00739460..0x007394D2` and `0x007396DF..0x00739711`; `DeploysInto=GACNST/NACNST/YACNST/YAREFN` in `rulesmd.ini`.

Implications for stock YR:

| Deploy target | Foundation | Binary origin relative to unit cell | INI evidence | Active in YR |
|---|---:|---:|---|---|
| GACNST/NACNST/YACNST | 4x4 | `unit_cell + (-1,-1)` | `rulesmd.ini:6977/7845/8834`; `artmd.ini:1601/1653/1626` | Yes |
| YAREFN (SMIN deploy) | 2x2 | `unit_cell` | `rulesmd.ini:9097`; `artmd.ini:1804` | Yes |

## 4. Current Rust Implementation Status

| Rust area | Status | Evidence | Comparison |
|---|---|---|---|
| Stored `GameEntity.position.rx/ry` for buildings | Uses placement/map/deploy cell as top-left/foundation origin | `src/sim/game_entity.rs:303`, `src/sim/world/world_spawn.rs:240`, `src/sim/world/world_spawn.rs:429` | MATCH for stored origin semantics |
| Map `[Structures]` parser | Reads fields 3/4 into `cell_x/cell_y`; `spawn_from_map` passes them directly to `GameEntity::new` | `src/map/entities.rs:190`, `src/map/entities.rs:249`, `src/sim/world/world_spawn.rs:118` | MATCH if map fields are origin cells; binary caller chain not fully enumerated |
| Ready building placement | Preview stores `rx/ry`, `place_ready_building` spawns object at same `rx/ry` | `src/sim/production/production_placement.rs:62`, `src/sim/production/production_placement.rs:180`, `src/sim/production/production_placement.rs:223` | MATCH for constructed/placed building origin semantics |
| Occupancy footprint | Structures occupy `building_footprint_cells(spawn_rx, spawn_ry, foundation, ...)` from stored position | `src/sim/world/world_spawn.rs:240`, `src/sim/world/world_spawn.rs:429` | MATCH for origin-based footprint seeding, excluding AddOccupy details |
| Generic MCV deploy | Computes `origin = unit_cell - (width/2, height/2)` | `src/sim/world/world_spawn.rs:503`, `src/sim/world/world_spawn.rs:514`, `src/sim/world/world_spawn.rs:680` | MISMATCH for stock ConYards: Rust 4x4 gives `(-2,-2)`, binary gives `(-1,-1)` |
| Slave Miner deploy path | Spawns YAREFN at the unit's current cell | `src/sim/slave_miner.rs:468`; stock YAREFN `Foundation=2x2` at `ini/artmd.ini:1804` | MATCH for YAREFN's binary `2x2 -> no offset` deploy origin |
| Building center API equivalent | No canonical sim helper found that reproduces `BuildingClass::GetCoords` as `(origin + (w-1)*128)` in lepton space | Codegraph + scoped grep | UNKNOWN / likely missing for APIs that need center-plus-target-offset semantics |

## 5. INI Keys

| Key | Location | Effect in this slice | Active in YR |
|---|---|---|---|
| `Foundation=` | `artmd.ini`, e.g. GACNST `1601`, NACNST `1653`, YACNST `1626`, YAREFN `1804`, GAREFN `1766`, NAREFN `1709` | Drives `BuildingTypeClass__GetFoundationWidth/Height`, center projection, footprint size, deploy adjustment threshold | Yes |
| `DeploysInto=` | `rulesmd.ini:6977`, `7845`, `8834`, `9097` | Selects building type passed to `BuildingClass::Constructor` in `UnitClass::Deploy` | Yes |
| `TargetCoordOffset` fields | BuildingType `+0xEBC/+0xEC0/+0xEC4`; stock nonzero naval yards per prior coordinate report | Added after `GetCoords`, not after stored origin | Conditional - only nonzero types shift target coord |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass__Set_Raw_Coords` | verified | `0x005F6940` | none |
| `ObjectClass__Reveal` placement store chain | verified | vtable `+0x1B4` call inside `ObjectClass__Reveal`; Building vtable `+0x1B4 -> 0x005F6940` | none |
| `BuildingClass__Unlimbo` entry and vtable binding | verified | `0x00440580`; vtable `0x007E3F94` | none for coordinate storage |
| `ObjectClass__Get_Cell_Packed` | verified | `0x0041BEA0` | none |
| `BuildingClass__GetCoords` | verified | `0x00447AC0` | none |
| `BuildingClass__GetTargetCoords` | verified | `0x004500A0` | none |
| `UnitClass__Deploy` `DeploysInto=` coordinate seed | verified | `0x007393C0`, assembly `0x00739460..0x00739711` | none for origin adjustment |
| Scenario map object caller inventory | touched-not-exhausted | generic Unlimbo storage chain verified; Rust parser read | Exact gamemd `[Structures]` scenario parser caller not enumerated |
| General AddOccupy/RemoveOccupy shape modifiers | deferred | out-of-scope by user instruction | sibling target likely owns foundation occupancy modifiers |

## 7. Open Questions - Final State

[RESOLVED] OQ-1 - Is a building's stored coordinate the foundation origin or center? Stored coordinate is foundation origin cell center. Evidence: `0x005F6940`, `0x00440580`, `ObjectClass::Reveal` vtable `+0x1B4` chain.  
[RESOLVED] OQ-2 - Does any common getter return a center coordinate? Yes, `BuildingClass__GetCoords @ 0x00447AC0` returns origin plus `(width-1,height-1)*128`.  
[RESOLVED] OQ-3 - Does `GetCell` expose origin or center cell? Origin cell. Evidence: `0x0041BEA0`.  
[RESOLVED] OQ-4 - How does standard `DeploysInto=` seed the building origin? Unit cell unchanged for <=2x<=2 target foundation; otherwise unit cell plus NW `(-1,-1)`. Evidence: `0x00739460..0x00739711`, `DAT_0089F6A4/6`.  
[RESOLVED] OQ-5 - Does current Rust store placed buildings at origin or center? Origin. Evidence: `src/sim/world/world_spawn.rs:240`, `src/sim/world/world_spawn.rs:429`.  
[RESOLVED] OQ-6 - Does current Rust MCV deploy match binary origin adjustment? No for 4x4 ConYards; Rust subtracts half foundation. Evidence: `src/sim/world/world_spawn.rs:680`, Ghidra `0x00739477..0x007394B1`.  
[DEFERRED] OQ-7 - Exact gamemd scenario `[Structures]` parser call site into `BuildingClass__Unlimbo`. Category: bounded-cost-too-high. Generic Unlimbo storage semantics are verified; this slot did not enumerate the full map-load object factory.

## Sources

- Ghidra: `0x00440580`, `0x005F6940`, `0x00447AC0`, `0x0041BEA0`, `0x004500A0`, `0x007393C0`; vtable data `0x007E3F04`, `0x007E3F94`, `0x007E4070`.
- Current Rust: `src/map/entities.rs`, `src/sim/world/world_spawn.rs`, `src/sim/production/production_placement.rs`, `src/sim/game_entity.rs`, `src/sim/slave_miner.rs`.
- INI: `ini/rulesmd.ini`, `ini/artmd.ini`.
