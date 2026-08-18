# NATBNK Bunker +0x2E4 Row Helper Path - Ghidra Research Report

**Address(es):** `0x00458A00` row helper, `0x00458E50` bunker link writer/state helper, `0x0073F76D` `UnitClass::Can_Enter_Cell` Bunker/UnitRepair callsite  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Bunker-specific fast-true branch in helper `0x00458A00`, the meaning of `BuildingClass+0x2E4`/Techno `+0x2E4` for that branch, and stock `NATBNK` candidate-cell consequence.  
**Non-Scope:** full bunker entry trigger/radio protocol, full eject/destruction lifecycle, all garrison systems, all `NumberImpassableRows` users.  
**Confidence:** High for helper branch, writer, and stock INI consequence; Medium for "while occupied" lifecycle end because clearing/eject writers were intentionally not traced.  
**Active in YR:** Conditional. Active for live YR `UnitClass::Can_Enter_Cell` when the encountered building is a `Bunker=yes` building and the candidate cell resolves back to that same building.

## 1. Overview

The Bunker branch in `0x00458A00` is an occupancy-state override for `NumberImpassableRows`. For stock `NATBNK`, `NumberImpassableRows=0`, so the normal strict west-row comparison does not block its own 2x2 foundation cells; once a tank has been installed into the bunker, `BuildingClass+0x2E4` becomes nonzero and the helper returns true before row math.

In the `UnitClass::Can_Enter_Cell` Bunker/UnitRepair callsite, helper false skips the current building occupant and continues scanning the cell object list. Helper true keeps the building in the normal blocking path.

## 2. Class Layout / Key Offsets

| Class / object | Offset | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingTypeClass` | `+0x16AB` | `Bunker=yes` type flag | `BuildingTypeClass_ReadINI_Water` reads `Bunker` to `+0x16AB`; `0x0073F74B..0x0073F755`; `0x00458A30` | Yes, for stock `NATBNK` (`rulesmd.ini:13732`) |
| `BuildingTypeClass` | `+0x1620` | `NumberImpassableRows` int | `BuildingTypeClass_ReadINI_Water` reads key to `+0x1620`; helper reads at `0x00458A20`, `0x00458A67` | Yes, `NATBNK` sets `0` (`rulesmd.ini:13751`) |
| `BuildingClass` | `+0x2E4` | Bunker contained-unit pointer used by helper fast-true | Read at `0x00458A3C`; state helper reads at `0x00458E5B`; writer at `0x00459301` | Conditional: nonzero after bunker install state 5 |
| `BuildingClass` | `+0x718` | Bunker state slot; state 5 performs install | switch source at `0x00458E99`; install writes state 6 at `0x00459327` | Conditional: only for `Bunker=yes` mission helper |
| `Techno`/unit object | `+0x2E4` (`[0xB9]`) | reciprocal back-reference from contained unit to bunker building | writer at `0x0045930F`; existing decompile shows `piVar5[0xB9] = building` | Conditional: same install event |
| `Techno`/unit object | `+0x214` (`[0x85]`) | cleared to `-1` during install | writer at `0x00459315` | Conditional: same install event |

## 3. Core Logic

### Helper `0x00458A00`

Pseudocode, with only the scoped branch:

```text
if Look_up_building_in_cell(candidate_cell) != building:
    return false

if building.Type.NumberImpassableRows == -1:
    return true

if building.Type.Bunker && building.field_0x2E4 != 0:
    return true

return candidate_cell.x < building.origin_cell.x + building.Type.NumberImpassableRows
```

Material details:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The Bunker fast-true branch runs only after the candidate cell's primary building lookup equals the current building. | `0x00458A08` call to `Look_up_building_in_cell`, compare at `0x00458A0F` | High | Conditional: same-building candidate cells only |
| `NumberImpassableRows == -1` returns true before the Bunker branch. | `0x00458A20..0x00458A2D` | High | Yes for buildings that retain the default sentinel |
| Bunker override requires both `BuildingType+0x16AB != 0` and `BuildingClass+0x2E4 != 0`. | `0x00458A30..0x00458A48` | High | Conditional: occupied/linked bunker only |
| If the Bunker override does not fire, the row comparison is strict `<`, not `<=`. | `0x00458A51..0x00458A72`; prior matrix identifies `SETG` equivalent | High | Yes |

### Bunker link writer/state helper `0x00458E50`

`BuildingClass::MissionRepairAndProduce` calls the bunker helper at `0x0044B7A3` when `param_1->Type[0x16AB]` is nonzero. The call happens before the ConstructionYard/Hospital/Armory/UnitRepair/UnitReload branches in that mission function.

The scoped lifecycle facts:

| State / branch | Finding | Evidence | Active in YR |
|---|---|---|---|
| Preflight | Reads `BuildingClass+0x2E4`; if zero, falls back to `FootClass__GetDestination(0)`. If no candidate or candidate `WhatAmI()!=1`, resets `+0x718=0` and sets building mission `5`. | `0x00458E5B..0x00458E8B`; decompile of `buildingclass_bunker_occupant_dock_link_writer` | Conditional: `Bunker=yes` mission tick |
| State 5 install | Writes `building+0x2E4 = unit`, `unit+0x2E4 = building`, `unit+0x214 = -1`, calls unit vtable `+0x150`, sets `building+0x718 = 6`, then calls linked unit mission setter with `(5,1)`. | `0x00459301..0x00459337` | Conditional: when entry state machine reaches install |
| Sound side effect | Plays `RulesClass+0x240` sound if not `-1`. | `0x0045933D..0x0045936F`; `rulesmd.ini:719` | Conditional: sound configured in stock YR |

This is enough to identify `BuildingClass+0x2E4` for the row helper: it is the bunker-contained unit pointer. It is nonzero while the building has a unit installed/linked by the bunker state helper. The exact later clear/eject paths are not claimed here.

## 4. INI Keys

| Section | Key | Stock YR value | Effect in this slice | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `[NATBNK]` | `Bunker` | `yes` | Sets `BuildingType+0x16AB`, enabling the Bunker gate at `0x0073F76D` and helper override at `0x00458A30`. | `rulesmd.ini:13732`; `0x0045FE50` read; `0x0073F755` | Yes |
| `[NATBNK]` | `NumberOfDocks` | `1` | Supports bunker entry/dock-style setup, but the callsite consequence here depends on `Bunker=yes` and `+0x2E4`. | `rulesmd.ini:13750` | Conditional; not traced beyond this slice |
| `[NATBNK]` | `NumberImpassableRows` | `0` | Makes normal row test false for same-foundation cells at `origin.x` and `origin.x+1`; occupied-bunker fast-true overrides this. | `rulesmd.ini:13751`; helper strict compare | Yes |
| `[NATBNK]` art | `Foundation` | `2x2` | Candidate building cells are the 2x2 foundation cells; their X values are not west of origin. | `artmd.ini:5022` | Yes |
| `[AudioVisual]` | `BunkerWallsUpSound` | `TankBunkerUp` | Played by install branch when `RulesClass+0x240 != -1`. | `rulesmd.ini:719`; `0x0045933D..0x0045936F` | Conditional: install branch |

## 5. Integration Points

`UnitClass::Can_Enter_Cell` has the scoped callsite at `0x0073F76D`. The gate is:

1. Current object in the candidate cell list is a building.
2. Building type has `UnitRepair=yes` (`+0x16A9`) or `Bunker=yes` (`+0x16AB`).
3. `Look_up_building_in_cell(candidate_cell) == current building`.
4. Call helper `0x00458A00`.
5. If helper returns false, jump to `0x0073FA87` and continue with the next cell object, so this building does not block this candidate cell.
6. If helper returns true, fall through to normal building blocker handling.

For stock `NATBNK` candidate foundation cells:

| Runtime bunker state | Helper result | Consequence at `0x0073F76D` | Evidence | Active in YR |
|---|---|---|---|---|
| Empty / no installed unit (`building+0x2E4 == 0`) | False, because `0 < origin.x + 0` is false for foundation X `origin.x` and `origin.x+1`. | `UnitClass::Can_Enter_Cell` skips the bunker object for that candidate cell; the bunker itself does not block via this branch. | `0x00458A30..0x00458A72`; `0x0073F76D..0x0073F774`; `rulesmd.ini:13751`; `artmd.ini:5022` | Conditional: same-building candidate cell |
| Occupied / installed unit linked (`building+0x2E4 != 0`) | True before row math. | The skip is suppressed; the bunker remains in the normal building blocker path for the candidate cell. | `0x00458A30..0x00458A48`; writer `0x00459301` | Conditional: after install state 5 |

The true path does not itself return a final passability code. It preserves the building for the later standard object ownership/visibility/crush/weapon checks in `UnitClass::Can_Enter_Cell`. For player-visible pathing, the material effect is that empty stock `NATBNK` foundation cells are row-exempt, while occupied stock `NATBNK` foundation cells are not row-exempt.

## 6. Current Rust Implementation Status

Rust currently parses and globally applies `NumberImpassableRows` without a scoped bunker contained-unit pointer equivalent in the movement grid path.

| Rust area | Status | Evidence | What matters |
|---|---|---|---|
| `number_impassable_rows` parsing | Present | `src/rules/object_type.rs:691`, `src/rules/object_type.rs:1090` | Stock `NATBNK` value `0` is representable. |
| Movement grid blocking | Present as grid-time building movement blocking | `src/app_init.rs:695..702`, `src/app_sim_tick.rs:813..814`, `src/sim/production/production_tech.rs:663..670`, `src/sim/pathfinding/core.rs:1474..1482` | Binary behavior is a conditional per-object skip inside `UnitClass::Can_Enter_Cell`, not an unconditional static grid rule. |
| Bunker `BuildingClass+0x2E4` equivalent | Not found in scoped scan | `rg` scan for bunker/number-impassable paths | Needed to reproduce occupied-vs-empty `NATBNK` passability distinction. |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Helper `0x00458A00` Bunker branch | verified | Ghidra decompile and assembly addresses `0x00458A30..0x00458A48` | none |
| `BuildingType+0x16AB` parser/source | verified | `BuildingTypeClass_ReadINI_Water`; `rulesmd.ini:13732` | none |
| `BuildingClass+0x2E4` install writer | verified | `buildingclass_bunker_occupant_dock_link_writer`; assembly `0x00459301..0x0045930F` | later clear/eject not traced |
| `Techno+0x2E4` reciprocal writer | verified | `0x0045930F`; decompile shows `piVar5[0xB9] = building` | later clear/eject not traced |
| `0x0073F76D` Bunker/UnitRepair callsite consequence | verified | assembly `0x0073F74B..0x0073F774`; decompile of `UnitClass__Can_Enter_Cell` | full downstream ownership return-code matrix out of scope |
| Stock `NATBNK` INI/art data | verified | `rulesmd.ini:13722..13751`, `artmd.ini:5019..5035` | none |
| Full bunker entry trigger | deferred | outside requested branch | separate bunker-entry/radio investigation |
| Full bunker clear/eject/destruction lifecycle | deferred | outside requested branch | separate bunker-exit/destruction investigation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - What does `BuildingType+0x16AB` mean? -> `Bunker=yes`.` Evidence: parser read in `BuildingTypeClass_ReadINI_Water`; stock `rulesmd.ini:13732`; callsite gate `0x0073F755`.
- `[RESOLVED] OQ-2 - What does `BuildingClass+0x2E4` mean in the helper? -> contained/linked unit pointer for bunker occupancy.` Evidence: helper read `0x00458A3C`; state-helper read `0x00458E5B`; install writer `0x00459301`.
- `[RESOLVED] OQ-3 - What is the unit-side `+0x2E4`? -> reciprocal back-reference to the containing bunker building.` Evidence: install writer `0x0045930F`.
- `[RESOLVED] OQ-4 - When does `BuildingClass+0x2E4` become nonzero? -> state 5 of bunker helper installs the unit and writes `building+0x2E4 = unit`.` Evidence: `0x00459301`; caller gate `0x0044B797..0x0044B7A3`.
- `[RESOLVED] OQ-5 - What happens for empty stock `NATBNK` candidate foundation cells? -> helper false, current bunker object skipped by `UnitClass::Can_Enter_Cell` at `0x0073F774`.` Evidence: `NumberImpassableRows=0` (`rulesmd.ini:13751`), `Foundation=2x2` (`artmd.ini:5022`), strict row compare `0x00458A51..0x00458A72`.
- `[RESOLVED] OQ-6 - What happens for occupied stock `NATBNK` candidate foundation cells? -> helper true before row math, so skip is suppressed and normal building blocker handling remains active.` Evidence: `0x00458A30..0x00458A48`; install writer `0x00459301`.
- `[DEFERRED] OQ-7 - Which exact function clears `BuildingClass+0x2E4` on unload/destruction/sell?` Category: out-of-scope; reason: not needed to decode the fast-true branch or occupied-vs-empty candidate-cell consequence; next step is a bunker exit/destruction lifecycle slice.

## Sources

- Ghidra decompile/assembly: `0x00458A00`, `0x00458A30..0x00458A72`, `0x00458E50`, `0x00458E5B`, `0x00458E99`, `0x00459301..0x0045936F`, `0x0044B780`, `0x0044B797..0x0044B7A3`, `0x0073F0A0`, `0x0073F74B..0x0073F774`, `0x0045FE50`.
- Existing docs checked: `BUNKER_SYSTEM_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_CALLSITE_MATRIX_GHIDRA_REPORT.md`, `.swarm-claims.md`.
- INI/art: `ini/rulesmd.ini:719`, `:13722..13751`; `ini/artmd.ini:5019..5035`.
- Rust scan only: `src/rules/object_type.rs`, `src/app_init.rs`, `src/app_sim_tick.rs`, `src/sim/production/production_tech.rs`, `src/sim/pathfinding/core.rs`.
