# ObjectClass Get_Cell_Packed Building Anchor - Ghidra Research Report

**Address(es):** `0x0041BEA0` primary; cross-checks `0x00447AC0`, `0x005F65A0`, `0x005F6940`, `0x0043C2D0`, `0x004595C0`, `0x00447B20`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Exact conversion performed by `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` and whether the packed cell is the building origin/NW cell for standard placed refinery buildings.  
**Non-Scope:** Full refinery dock state machines, placement legality, render anchoring beyond the direct `GetCoords`/`Get_Cell_Packed` contrast, and runtime debugging.  
**Confidence:** High for the conversion and anchor result; Medium for "standard placed" coordinate initialization because this report cross-checks layout/docs rather than re-tracing every placement caller.  
**Active in YR:** Yes. Evidence: `BuildingClass__Receive_Radio @ 0x0043C2D0` and `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` call building vtable `+0x1B8`; BuildingClass vtable is documented at `0x007E3EBC` with `+0x1B8 -> 0x0041BEA0`; retail `GAREFN`/`NAREFN` are `Refinery=yes` in `ini/rulesmd.ini`.

## 1. Overview

`ObjectClass__Get_Cell_Packed` reads the raw object `Location.X` and `Location.Y` fields, converts each lepton coordinate to a signed 16-bit cell index, and packs them as `low16 = cell_x`, `high16 = cell_y`. For buildings, this is not the foundation center path: it does not call `BuildingClass__GetCoords`, does not read foundation width/height, and does not add any offset.

For standard placed refineries whose raw `Location` is the NW/foundation-origin cell center, the returned packed cell is the NW/foundation-origin cell. Example for a 4x3 `GAREFN`/`NAREFN` at origin cell `(10,10)`: raw `Location = (10*256+128, 10*256+128)`, so `Get_Cell_Packed` returns `(10,10)`, while `BuildingClass__GetCoords @ 0x00447AC0` returns the foundation center `(10*256+512, 10*256+384)`.

## 2. Class Layout / Key Offsets

| Offset | Field | Type | Purpose | Evidence | Active in YR |
|--------|-------|------|---------|----------|--------------|
| `+0x9C` | `Location.X` | signed int leptons | X source for packed cell | `0x0041BEA0`, `0x005F65A0`, `0x005F6940` | Yes - inherited ObjectClass field used by BuildingClass |
| `+0xA0` | `Location.Y` | signed int leptons | Y source for packed cell | `0x0041BEA0`, `0x005F65A0`, `0x005F6940` | Yes - inherited ObjectClass field used by BuildingClass |
| `+0xA4` | `Location.Z` | signed int leptons | ignored by packed cell | absent from `0x0041BEA0`; copied by `0x005F65A0` | Yes - active field, not part of this conversion |
| Building vtable `+0x48` | `GetCoords` | virtual call | foundation-center coordinate for buildings | `0x00447AC0`; docs vtable `0x007E3EBC + 0x48` | Yes - used by building/radio/dock helpers |
| Building vtable `+0x1B8` | `Get_Cell_Packed` | virtual call | raw-origin packed cell | `0x0043C2D0`, `0x004595C0`; docs vtable `0x007E3EBC + 0x1B8` | Yes - refinery radio/dock paths call it |

## 3. Core Logic

Verified pseudocode from `0x0041BEA0`:

```text
cell_x = (short)((Location.X + ((Location.X >> 31) & 0xFF)) >> 8)
cell_y = (short)((Location.Y + ((Location.Y >> 31) & 0xFF)) >> 8)
out = (cell_y << 16) | (cell_x & 0xFFFF)
```

Material details:

| Finding | Evidence | Confidence | Active in YR |
|---------|----------|------------|--------------|
| The function reads only raw `this+0x9C` and `this+0xA0`; it never calls a virtual `GetCoords` and never reads building type/foundation data. | `0x0041BEA0` decompile | High | Yes - called through BuildingClass vtable `+0x1B8` in active refinery paths |
| The conversion is not a plain arithmetic `sar 8`; for negative inputs it first adds `0xFF` via `(value >> 31) & 0xFF`. | `0x0041BEA0` decompile | High | Yes - same active code path; ordinary map cells are nonnegative |
| The negative-coordinate behavior is signed division by 256 with truncation toward zero, not floor. Examples: `-1 -> 0`, `-255 -> 0`, `-256 -> -1`, `-257 -> -1`. | formula at `0x0041BEA0` | High | Conditional - only matters if object Location can be negative |
| Each shifted result is narrowed to `short` before packing; there is no clamp or map-bound check in this helper. | explicit `(short)` casts in `0x0041BEA0` | High | Yes - helper always behaves this way; standard refinery cells are in range |
| `Location.Z` is ignored. The packed cell is 2D only. | `0x0041BEA0` reads `+0x9C/+0xA0` only; `0x005F65A0` shows `+0xA4` is Z | High | Yes |

Important correction against some prior wording: `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` describes this pattern as "floor-correct." That is wrong for negative values. The binary expression implements truncation toward zero for negative values. Existing `COORDINATE_SYSTEM_GAMEMD.md` only says "sign_corr" and "truncates +128 sub-cell offset," which is consistent with this report.

## 4. INI Keys

No INI key controls `ObjectClass__Get_Cell_Packed`.

| Key / section | Value | Effect on this helper | Evidence | Active in YR |
|---------------|-------|-----------------------|----------|--------------|
| `ini/artmd.ini [NAREFN] Foundation` | `4x3` | Cross-check only; `Get_Cell_Packed` ignores it, while `BuildingClass__GetCoords` uses foundation dimensions. | `ini/artmd.ini:1706`, `ini/artmd.ini:1709`; `0x00447AC0` | Yes |
| `ini/artmd.ini [GAREFN] Foundation` | `4x3` | Cross-check only; same as above. | `ini/artmd.ini:1763`, `ini/artmd.ini:1766`; `0x00447AC0` | Yes |
| `ini/artmd.ini [YAREFN] Foundation` | `2x2` | Cross-check only; same helper result remains raw origin/NW. | `ini/artmd.ini:1799`, `ini/artmd.ini:1804`; `0x00447AC0` | Yes |
| `ini/artmd.ini QueueingCell` | `4,1` on `NAREFN`/`GAREFN` | Not read by `Get_Cell_Packed`; active radio `CAN_DOCK` path uses hardcoded `packed + (3,1)`, not this key. | `ini/artmd.ini:1716`, `ini/artmd.ini:1773`, `0x0043C2D0` | Yes for art data; no effect on this helper |
| `ini/rulesmd.ini Refinery=yes` | set on `GAREFN`/`NAREFN` | Confirms stock placed Allied/Soviet refinery building types are standard refinery buildings in YR. | `ini/rulesmd.ini:11722`, `11727`, `12515`, `12520` | Yes |

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|-------------|-------------------|----------|--------------|
| `ObjectClass__GetCoords @ 0x005F65A0` | Base object coordinate getter returns raw `Location.X/Y/Z`. | direct decompile | Yes |
| `ObjectClass__Set_Raw_Coords @ 0x005F6940` | Base setter writes raw `Location.X/Y/Z` with no conversion. | direct decompile | Yes |
| `BuildingClass__GetCoords @ 0x00447AC0` | Building coordinate getter adds `width*0x80-0x80` to X and `height*0x80-0x80` to Y, then copies Z. | direct decompile | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x0E` | For refinery/resource-destination acceptance it calls vtable `+0x1B8`, then sends `packed.x + 3`, `packed.y + 1` as a `CellClass` destination. | direct decompile | Yes - active docking path for powered refinery/resource destination buildings |
| `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` conditional branch | Calls vtable `+0x48` for center-based `Force_Track` target, then vtable `+0x1B8` for separate passable-cell seed `packed.x - 1`, `packed.y + 1`. | direct decompile | Conditional - only when reciprocal dock link `+0x2E4` is non-null; prior reports note normal stock unload may not set that link |
| `BuildingClass__GetDockCoord @ 0x00447B20` | Some branches use `Get_Cell_Packed`; others use `GetCoords` plus offsets. It is not the helper under investigation. | direct decompile | Conditional by building type/dock branch |

## 6. Current Rust Implementation Status

This report did not modify Rust code.

| Rust area | Observed status | Evidence |
|-----------|-----------------|----------|
| Entity building position | The implementation stores building entity `position.rx/ry` and uses it as the foundation origin in many systems. | `src/sim/world/world_spawn.rs:551-555`, `src/app_entity_pick.rs:347-360` |
| Refinery `CAN_DOCK` queue seed | Current code has a dedicated `refinery_can_dock_queue_cell(rx, ry) -> (rx+3, ry+1)` with a comment citing gamemd. | `src/sim/miner/miner_dock_sequence.rs:105-109` |
| General refinery queue cell | Separate helper still uses `QueueingCell` or a geometry fallback for other purposes. | `src/sim/miner/miner_dock_sequence.rs:91-101` |
| Combat targeting | Current combat target coordinate path intentionally uses foundation center for structures, matching `BuildingClass__GetCoords`, not `Get_Cell_Packed`. | `src/sim/combat/mod.rs:297-317` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` | verified | direct decompile | none |
| Signed shift/correction behavior | verified | `0x0041BEA0` formula | none |
| Packed layout low16 X / high16 Y | verified | `CONCAT22(y_short, x_short)` in `0x0041BEA0` | none |
| Building center contrast | verified | `0x00447AC0` decompile | none |
| Base raw coordinate fields | verified | `0x005F65A0`, `0x005F6940` decompile | none |
| Active refinery `CAN_DOCK` consumer | verified | `0x0043C2D0` decompile; `ini/rulesmd.ini` refinery entries | none |
| Conditional dock-release consumer | verified | `0x004595C0` decompile | normal stock unload link lifecycle intentionally not re-investigated here |
| Full building placement call chain | touched-not-exhausted | coordinate docs plus raw field setters | exact constructor/unlimbo path for every placement source is outside this slice |
| TS legacy gates | verified for this helper | no flags/INI gates in `0x0041BEA0`; active YR consumers verified | none |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does `0x0041BEA0` use raw object coordinates or building foundation center? It uses raw `Location.X/Y` only. Evidence: `0x0041BEA0`; `0x00447AC0` contrast.

[RESOLVED] OQ-2 - What is the signed rounding behavior? Signed divide-by-256 truncating toward zero, implemented as `(v + ((v >> 31) & 0xFF)) >> 8`. Evidence: `0x0041BEA0`.

[RESOLVED] OQ-3 - What packed layout is returned? Low 16 bits are X; high 16 bits are Y. Evidence: `CONCAT22(y_short, x_short)` at `0x0041BEA0`.

[RESOLVED] OQ-4 - Is Z included? No. Evidence: `0x0041BEA0` reads `+0x9C/+0xA0` only; `0x005F65A0` shows `+0xA4` is the raw Z field.

[RESOLVED] OQ-5 - Does the result correspond to the NW/foundation origin for standard placed refineries? Yes for ordinary nonnegative placed cells: the raw building `Location` is the origin cell center, and the helper divides that raw coordinate by 256 without foundation offsets. Evidence: `0x0041BEA0`, `0x00447AC0`, `COORDINATE_SYSTEM_GAMEMD.md`, `ini/artmd.ini:1709/1766`.

[RESOLVED] OQ-6 - Is the code active in standard YR? Yes. Evidence: `0x0043C2D0` calls vtable `+0x1B8` in refinery/resource destination radio acceptance, `GAREFN` and `NAREFN` are `Refinery=yes` in `ini/rulesmd.ini:11727/12520`, and BuildingClass vtable mapping documents `+0x1B8 -> 0x0041BEA0`.

[DEFERRED] OQ-7 - Which exact placement/unlimbo function writes the first raw `Location` for every map/editor/build-queue path? Category: out-of-scope. Reason: not needed to resolve the helper conversion, and the current slice was constrained to `Get_Cell_Packed` plus refinery anchor. Next step: separate building placement coordinate initialization investigation if needed.

## Sources

- Ghidra decompiled/read: `0x0041BEA0`, `0x00447AC0`, `0x005F65A0`, `0x005F6940`, `0x0043C2D0`, `0x004595C0`, `0x00447B20`.
- Cross-check docs: `C:/Users/enok/Documents/ra2-rust-game-docs/COORDINATE_SYSTEM_GAMEMD.md`, `FOUNDATION_CENTER_INVESTIGATION.md`, `miner/CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`.
- INI data: `ini/artmd.ini:1706-1716`, `ini/artmd.ini:1763-1773`, `ini/artmd.ini:1799-1804`, `ini/rulesmd.ini:11722-11727`, `ini/rulesmd.ini:12515-12520`, `ini/rulesmd.ini:13234`.
- Rust status scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/combat/mod.rs`, `src/sim/world/world_spawn.rs`, `src/app_entity_pick.rs`.
