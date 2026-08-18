# RulesClass+0x850 in UnitClass::Mission_Unload - Ghidra Research Report

**Address(es):** `0x00740EF0` (`UnitClass::Mission_Unload`), `0x0066F362` (`RulesClass::ReadGeneral` RepairBay key), `0x00665650` (RulesClass constructor), `0x004DF040` (`FootClass::Find_Docking_Bay`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Identify `RulesClass+0x850`, how it is initialized/parsed, how `Mission_Unload` and nearby harvester flow pass it, and whether it affects standard YR chrono miner/refinery unload.
**Non-Scope:** Full repair-depot mission behavior, carryall unload behavior, Weeder/slave-miner flow beyond proving non-standard chrono-refinery impact.
**Confidence:** High
**Active in YR:** Conditional. The `RepairBay=` rules field and `Find_Docking_Bay` uses are live in YR; the `Mission_Unload` use is not part of standard chrono miner/refinery unload.

## 1. Overview

`RulesClass+0x850` is the start of the `[General] RepairBay=` building-type vector, not a scalar harvest/unload amount. The earlier phrasing "used as first transmit arg" is misleading: in `UnitClass::Mission_Unload`, the field is the first explicit argument to virtual slot `+0x528`, which UnitClass binds to `FootClass::Find_Docking_Bay @ 0x004DF040`; only the returned building pointer is later passed to radio `HELLO` through virtual slot `+0x278`.

For standard YR Allied chrono miner and Soviet war miner refinery unload, this does not drive the normal dump cycle. Standard ore return uses the unit type's own `Dock=` list (`param_1[0x1B1] + 1000`) in `Mission_Harvest` state 2 and the actual deposit runs through `Mission_Enter` / `Mission_Deploy_Building`, not `Mission_Unload`.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `RulesClass` | `+0x850` | dynamic vector object | `[General] RepairBay=` list, entries are `BuildingTypeClass*` | constructor `0x00665650` initializes vector at dword index `0x214`; `ReadGeneral` RepairBay block copies to `ESI+0x850`; parser calls `BuildingTypeClass::FindOrAllocate @ 0x004653C0` | Yes |
| `RulesClass` | `+0x854` | pointer inside vector | vector data pointer used by `Find_Docking_Bay` | `FootClass::Find_Docking_Bay @ 0x004DF040` reads `*(param_2 + 4)` | Yes |
| `RulesClass` | `+0x860` | int inside vector | active entry count; existing `RULESCLASS_FIELDS.csv` row points here | `FootClass::Find_Docking_Bay @ 0x004DF040` tests `*(param_2 + 0x10)` | Yes |
| `UnitClass` vtable | `+0x528` | method pointer | UnitClass binds this slot to `FootClass::Find_Docking_Bay @ 0x004DF040` | memory at `0x007F6198` = `40 F0 4D 00` | Yes |
| `UnitClass` vtable | `+0x278` | method pointer | radio `Transmit_Radio` wrapper, used after a dock building is found | `UnitClass::Mission_Unload @ 0x00740F5B..0x00740F60` calls slot `+0x278` with message `2` and returned building | Conditional |

## 3. Core Logic

`UnitClass::Mission_Unload @ 0x00740EF0`:

1. Calls vtable slot `+0x528` with `(Rules+0x850, 0, 0)`.
2. Clears unit byte `+0x6D2`.
3. If no building is found, retries `+0x528` with `(Rules+0x850, 0, 1)`.
4. If retry finds a building, calls vtable slot `+0x484` with `(0, 1)` to clear/reset path state.
5. If the first search found a building, sends radio `HELLO` (`2`) to that building through slot `+0x278`.
6. If `HELLO` returns `ROGER` (`1`), queues mission `7` (`Mission_Enter`) and returns `1`; otherwise falls through to the mission timer.

The load-bearing correction is step 1: `Rules+0x850` is not a radio payload. It is a building-type search list consumed by `Find_Docking_Bay`. Active in YR: Conditional. The code is present and can run when mission `10` is assigned, but this flow is not standard chrono miner/refinery unload.

`FootClass::Find_Docking_Bay @ 0x004DF040`:

- Takes the vector pointer as `param_2`.
- Iterates `i < *(param_2 + 0x10)`.
- Loads each building type pointer from `*(param_2 + 4) + i*4`.
- Calls vtable slot `+0x52C` helper for each candidate type and returns the best building.

Active in YR: Yes. The same helper is used for ordinary `Dock=` searches, but the `RepairBay=` vector is only one possible caller-provided list.

## 4. INI Keys

| INI key | Section | Binary field | Constructor default | Standard YR effective value | Evidence | Active in YR |
|---|---|---:|---|---|---|---|
| `RepairBay=` | `[General]` | `RulesClass+0x850` vector | empty vector, capacity initialized to 10, count 0 | `GADEPT,NADEPT,CAOUTP`; `YADEPT` is commented out | string `RepairBay` at `0x0083C818`, xref `0x0066F362`; `ini/rulesmd.ini:389`; `ini/rules.ini:299` | Yes |

Parser details:

- `RulesClass::ReadGeneral @ 0x0066F362` reads key string `RepairBay`.
- Present value: tokenizes the string and resolves each token through `BuildingTypeClass::FindOrAllocate @ 0x004653C0`, then copies the resulting vector into `Rules+0x850`.
- Absent value: copies the existing vector at `Rules+0x850`, so the constructor default remains an empty vector unless a prior/base rules layer already filled it.
- YR `rulesmd.ini` patches base RA2 `RepairBay=GADEPT,NADEPT` by adding `CAOUTP`; this is active for standard YR rules loading.

## 5. Integration Points

| Integration | Finding | Evidence | Active in YR |
|---|---|---|---|
| Rules creation | RulesClass object allocated as size `0x18C0`, then constructor `0x00665650` runs | `CCFileClass__Constructor @ 0x0052BAD8..0x0052BAFA` writes `g_RulesClass_Instance = FUN_00665650()` | Yes |
| Rules parsing | `[General] RepairBay=` read in `RulesClass::ReadGeneral` | `RepairBay` string xref from `0x0083C818` to `0x0066F362` | Yes |
| Mission_Unload search | Uses `Rules+0x850` as the dock-search type list | assembly `0x00740EF3..0x00740F08` adds `0x850`, pushes `(list,0,0)`, calls slot `+0x528` | Conditional |
| Mission_Unload radio | Radio receives returned building pointer, not `Rules+0x850` | assembly `0x00740F5B..0x00740F60` pushes returned `EAX`, pushes `2`, calls slot `+0x278` | Conditional |
| Standard chrono miner unload | Does not use `Rules+0x850` for normal refinery return/deposit | `Mission_Harvest @ 0x0073E5E0` state 2 calls slot `+0x528` with `UnitType+1000` (`Dock=` list); prior verified deposit report identifies `Mission_Deploy_Building @ 0x0073D630` as dump FSM | No for standard chrono-refinery unload |

## 6. Current Rust Implementation Status

The Rust rules model parses building `UnitRepair=yes` and uses it for command validation (`src/rules/object_type.rs:1042`, `src/sim/world/world_commands.rs:715`), but this scan found no parsed global `[General] RepairBay=` list in `src/rules` or `src/sim`. That is not a standard chrono miner/refinery unload gap; it matters for the no-ore repair fallback and any direct `Mission_Unload` parity work.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RulesClass+0x850` identity | verified | `ReadGeneral` xref `0x0066F362`; vector copy to `ESI+0x850`; `BuildingTypeClass::FindOrAllocate @ 0x004653C0` | none |
| Constructor default | verified | `RulesClass` ctor `0x00665650`: vector at dword index `0x214` (`+0x850`), count 0, capacity 10 | none |
| Retail INI value | verified | `ini/rulesmd.ini:389`, `ini/rules.ini:299` | none |
| `UnitClass::Mission_Unload @ 0x00740EF0` use | verified | decompile + assembly `0x00740EF3..0x00740F73` | none |
| UnitClass vtable slot `+0x528` binding | verified | memory `0x007F6198` = `0x004DF040` | none |
| `FootClass::Find_Docking_Bay @ 0x004DF040` vector layout | verified | decompile reads `param_2+4` and `param_2+0x10` | none |
| Standard chrono miner/refinery impact | verified | `Mission_Harvest @ 0x0073E5E0` uses unit `Dock=` list; radio/refinery unified report and deposit report place dump in `Mission_Deploy_Building` | none |
| Full service-depot repair mission behavior | deferred | out of scope | separate UnitRepair/Mission_Repair investigation if needed |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What is `RulesClass+0x850`? It is the start of the `[General] RepairBay=` `BuildingTypeClass*` vector. Evidence: `RepairBay` string xref `0x0066F362`, parser copy to `ESI+0x850`, `BuildingTypeClass::FindOrAllocate @ 0x004653C0`.

[RESOLVED] OQ-2 - Why did `RULESCLASS_FIELDS.csv` say `RepairBay` at `0x860`? `+0x860` is the vector's active count field (`+0x10` inside the vector object); the field object starts at `+0x850`. Evidence: `FootClass::Find_Docking_Bay @ 0x004DF040` tests `*(param_2+0x10)`.

[RESOLVED] OQ-3 - Is `Rules+0x850` a radio transmit payload in `Mission_Unload`? No. It is passed to vtable slot `+0x528` (`Find_Docking_Bay`); radio slot `+0x278` receives the returned building pointer. Evidence: `UnitClass` vtable memory `0x007F6198`, assembly `0x00740F05..0x00740F60`.

[RESOLVED] OQ-4 - What is the default? Constructor default is an empty vector with count 0/capacity 10; standard YR effective INI value is `GADEPT,NADEPT,CAOUTP` from `[General] RepairBay=`. Evidence: constructor `0x00665650`; `ini/rulesmd.ini:389`.

[RESOLVED] OQ-5 - Does this affect standard YR chrono miner/refinery unload? No. The standard refinery path searches the unit type `Dock=` list and deposits through `Mission_Enter` / `Mission_Deploy_Building`, not `Mission_Unload` with `RepairBay=`. Evidence: `Mission_Harvest @ 0x0073E5E0`; unified radio report OQ-9 parent context; `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`.

## Sources

- Ghidra `decompile_function 00740ef0`
- Ghidra `get_assembly_context 00740ef0`
- Ghidra `inspect_memory_content 007f6198`
- Ghidra `search_strings ^RepairBay$`
- Ghidra `get_xrefs_to 0083c818`
- Ghidra `get_assembly_context 0066f362` and `0066f3a7`
- Ghidra `decompile_function 004653c0`
- Ghidra `decompile_function 004df040`
- Ghidra `decompile_function 0073e5e0`
- Ghidra `decompile_function 00665650`
- `docs/research/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` OQ-9
- `ini/rulesmd.ini:389`
- `ini/rules.ini:299`
