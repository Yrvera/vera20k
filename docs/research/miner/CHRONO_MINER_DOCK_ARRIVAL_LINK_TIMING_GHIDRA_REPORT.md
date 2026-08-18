# Chrono Miner Dock Arrival Link Timing - Ghidra Research Report

**Address(es):** `0x00739EC0`, `0x00737430`, `0x0043C2D0`, `0x006F4AB0`, `0x0073D630`, `0x00458E50`, `0x0044B780`, `0x004B0EF0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact writer/tick context for refinery dock-arrival link fields and the ordering of radio `0x15`, locomotor stop, radio `0x16` timing sync, and mission transition for standard YR CMIN/HARV -> GAREFN/NAREFN.  
**Non-Scope:** far-return target selection, post-unload exit anchor pathing, full ore-drain economics, and Bunker occupant gameplay beyond identifying the `+0x2E4` writer.  
**Confidence:** High for the requested timing slice.  
**Active in YR:** Conditional overall. The radio/pad-arrival path is active for stock CMIN/HARV refineries; the `unit/building +0x2E4` reciprocal pointer writer is active in YR only for `Bunker=yes` buildings, not stock refineries.

## 1. Overview

The stock ore-refinery arrival path does not set `unit+0x2E4` or `building+0x2E4` as an alt dock pointer. The standard path uses radio and mission state: `BuildingClass::Receive_Radio(0x0E)` sends `0x18` and `0x16`, `TechnoClass::Receive_Radio(0x18)` sets `unit+0x418 = 1`, `UnitClass::Receive_Radio(0x16)` calls locomotor slot `+0x4C` with `0x4000` if needed, and physical pad arrival in `UnitClass::PerCellProcess` sends radio `0x15` before powering off the locomotor.

The exact reciprocal `+0x2E4` writer exists, but it is in `FUN_00458E50`, called from `BuildingClass::MissionRepairAndProduce` only when `BuildingType+0x16AB` (`Bunker=yes`) is set. Stock GAREFN/NAREFN have `DockUnload=yes` at `+0x16B3` and `Refinery=yes` at `+0x16BB`; they do not have `Bunker=yes`.

## 2. Key Offsets

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit/Techno `+0x418` | entered-dock flag set by radio `0x18`, cleared by `0x19` | `TechnoClass::Receive_Radio @ 0x006F4B72` writes byte `1`; `0x006F4BA6` writes byte `0` | Yes for refinery chain |
| Unit/Foot `+0x5A4` | dock/radio destination link used by pad-arrival equality check | `UnitClass::PerCellProcess @ 0x0073A4DF..0x0073A4F7`; `Mission_Deploy_Building @ 0x0073E539` | Yes |
| Unit `+0x674` | locomotor COM pointer | reads before `+0x4C`, `+0x5C`, movement checks | Yes |
| Unit byte `+0x6AF` | chrono/teleporting gate for `+0x4C(0x4000)` | `0x007376EE`, `0x0073DF7A` | Yes, conditionally false during dock drive-in |
| Unit byte `+0x6D1` | unload FSM initialized flag | set at `0x0073DFDA`; cleared at `0x0073DEF8`, `0x0073E1F6` | Yes |
| Unit/Building `+0x2E4` | reciprocal Bunker occupant link, not stock refinery dock-arrival link | `FUN_00458E50` case 5 writes both sides | Conditional: YR Bunker only |
| Building `+0x718` | Bunker dock/enter sub-state for `FUN_00458E50` | switch in `0x00458E50`; writer to 6 at `0x00459327` | Conditional: YR Bunker only |
| BuildingType `+0x16AB` | `Bunker=yes` | parser `BuildingTypeClass_ReadINI_Water`; `rulesmd.ini:[NATBNK] Bunker=yes` | Conditional, not GAREFN/NAREFN |
| BuildingType `+0x16B3` | `DockUnload=yes` | parser and GAREFN/NAREFN lines | Yes for stock refineries |
| BuildingType `+0x16BB` | `Refinery=yes` | parser and GAREFN/NAREFN lines | Yes for stock refineries |
| Locomotor slot `+0x4C` | DriveLocomotion `Do_Turn`, sets owner facing RateTimer target | `DriveLocomotionClass__Do_Turn @ 0x004B0EF0` calls `RateTimer__Set(&param_2)` | Yes |
| Locomotor slot `+0x5C` | pad-arrival stop/power-off call | `UnitClass::PerCellProcess @ 0x0073A521..0x0073A52B` | Yes |

## 3. Core Logic

### 3.1 Standard refinery admission before pad arrival

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` handles stock DockUnload acceptance. For GAREFN/NAREFN (`+0x16B3 != 0`), it computes the hardcoded queue cell from the building map cell plus `(3,1)`, sends radio `0x12` with that cell, and only after `0x12` returns `0x14` sends `0x18` then `0x16` to the unit.

**Active in YR:** Yes. `[CMIN]` and `[HARV]` have `Dock=NAREFN,GAREFN`; `[GAREFN]` and `[NAREFN]` have `DockUnload=yes` and `Refinery=yes` in `ini/rulesmd.ini`.

### 3.2 `0x18` writes `+0x418`, not `+0x2E4`

`TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x18` writes `byte [this+0x418] = 1` at `0x006F4B72`, then propagates radio `0x18` via vtable `+0x278`. Case `0x19` clears the same byte at `0x006F4BA6`, then propagates `0x19`.

This corrects the prior shorthand that called `+0x2E4` the radio `0x18` DockedIn flag. In the live disassembly, the radio flag is `+0x418`.

**Active in YR:** Yes for the standard refinery chain, because BuildingClass case `0x0E` sends `0x18` to CMIN/HARV.

### 3.3 Timing sync `0x16` happens before final pad-arrival `0x15`

`UnitClass::Receive_Radio @ 0x00737430` case `0x16` calls `FootClass::Receive_Radio` first, then:

1. If `unit+0x6AF == 0` and `RateTimer::Current() != 0x4000`, calls `locomotor+0x4C(unit+0x674, 0x4000)` and returns `1`.
2. Otherwise, if the locomotor reports not moving, the unit has a destination building, `unit+0x418 != 0`, and unit mission is `7`, sends directed radio `0x15` to that building via vtable `+0x278`.

`DriveLocomotionClass__Do_Turn @ 0x004B0EF0` is the Drive implementation of slot `+0x4C`; it calls `RateTimer__Set(&param_2)`. The value `0x4000` is a facing/timing RateTimer target, not a speed scalar and not a link-field write.

**Active in YR:** Yes. CMIN/HARV standard dock approach uses this while not actively teleporting (`+0x6AF == 0`).

### 3.4 Final pad-arrival order in `UnitClass::PerCellProcess`

When mission is `7` or `0x19`, the current destination is a building, the unit cell equals the building dock cell, and the locomotor piggyback CLSID test matches `CLSID_WalkLocomotion`, `UnitClass::PerCellProcess @ 0x00739EC0` performs:

1. Optional DockLink fill only for `BuildingType+0x16A9` UnitRepair; not standard refineries.
2. If `destination == unit+0x5A4`, call `FootClass::PerCellProcess(2)`.
3. Send radio `0x15` via vtable `+0x274`.
4. Reload `unit+0x674`; assert if null.
5. Call locomotor vtable `+0x5C` to stop/power off.

The order is `PerCellProcess(2)` -> radio `0x15` -> locomotor stop. The stop is not part of radio `0x15` or `0x16`.

**Active in YR:** Yes for stock CMIN/HARV refinery pad arrival.

### 3.5 Mission transition after radio `0x15`

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` checks receiver type flags. For `DockUnload=yes` (`+0x16B3`), it calls the sender unit's mission setter vtable `+0x1E8` with mission `0x10`, queued flag `0`, then returns `1`.

`UnitClass::Mission_Deploy_Building @ 0x0073D630` is the mission `0x10` handler for the harvester unload FSM. Its stock refinery path uses `+0x418`, `+0x5A4`, `+0x6D1`, the facing RateTimer at `+0x388`, and the `DAT_0089F6A0` dock offset to find the refinery from the unit cell; it does not require the Bunker reciprocal `+0x2E4` link.

**Active in YR:** Yes for stock GAREFN/NAREFN.

### 3.6 Exact `+0x2E4` reciprocal writer

The exact reciprocal writer is in `FUN_00458E50`, called from `BuildingClass::MissionRepairAndProduce @ 0x0044B780` only when `BuildingType+0x16AB != 0`. `BuildingTypeClass_ReadINI_Water` identifies `+0x16AB` as `Bunker=yes`, and stock `rulesmd.ini` sets it on `[NATBNK]` at line 13732.

In `FUN_00458E50` case `field_0x718 == 5`, the order is:

1. `building+0x2E4 = unit` at `0x00459301`
2. `unit+0x2E4 = building` at `0x0045930F`
3. `unit+0x214 = -1` at `0x00459315`
4. call unit vtable `+0x150`
5. `building+0x718 = 6` at `0x00459327`
6. set the unit mission to `5` queued (`PUSH 1`, `PUSH 5`, call vtable `+0x1E8`) at `0x00459331..0x00459337`

This is an active YR system, but it is not the stock chrono/war miner refinery pad-arrival writer.

**Active in YR:** Conditional. Yes for `Bunker=yes` NATBNK; No for GAREFN/NAREFN because they have `DockUnload=yes`/`Refinery=yes`, not `Bunker=yes`.

## 4. INI Keys

| INI path | Value | Effect for this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN]` | `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Teleporter=yes` | CMIN uses the harvester radio/refinery chain | Yes |
| `rulesmd.ini:[HARV]` | `Dock=NAREFN,GAREFN`, `Harvester=yes` | Regular miner uses same pad-arrival chain | Yes |
| `rulesmd.ini:[GAREFN]` | `DockUnload=yes`, `Refinery=yes` | Building receiver `0x0E`/`0x15` refinery path | Yes |
| `rulesmd.ini:[NAREFN]` | `DockUnload=yes`, `Refinery=yes` | Same as GAREFN | Yes |
| `rulesmd.ini:[NATBNK]` | `Bunker=yes` | Enables `FUN_00458E50` and reciprocal `+0x2E4` pointer writes | Conditional; not refinery docking |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | sends `0x12`, `0x18`, `0x16`; receives `0x15` and sets mission `0x10` | decompile case `0x0E`/`0x15` | Yes |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | writes `+0x418` for `0x18`/`0x19` | disassembly `0x006F4B72`, `0x006F4BA6` | Yes |
| `UnitClass::Receive_Radio @ 0x00737430` | handles timing sync `0x16`; may cascade `0x15` when already stopped | decompile case `0x16` | Yes |
| `UnitClass::PerCellProcess @ 0x00739EC0` | physical pad-cell arrival: `PerCellProcess(2)`, radio `0x15`, loco `+0x5C` | decompile/disassembly hot path | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | mission `0x10` unload FSM; RateTimer wait and `+0x6D1` init | disassembly `0x0073DF56..0x0073E09D` | Yes |
| `FUN_00458E50` | Bunker enter sub-FSM; exact reciprocal `+0x2E4` writer | decompile and disassembly `0x00459301..0x00459337` | Conditional |
| `BuildingClass::MissionRepairAndProduce @ 0x0044B780` | only caller of `FUN_00458E50`, gated by `+0x16AB` | xref from `0x0044B7A3` | Conditional |

## 6. Current Rust Implementation Status

Rust uses an explicit `RefineryDockPhase` FSM rather than the raw radio protocol. The local scan found the relevant surfaces in `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/miner/miner_system.rs`, and `src/sim/miner/miner_tests.rs`. Comments currently describe radio `0x16` as a RateTimer pivot and radio `0x15` as the transition into unloading; no Rust files were modified in this investigation.

Implementation caution: do not model stock refinery arrival as a reciprocal `unit/building +0x2E4` pointer link. Static binary evidence puts that reciprocal writer behind `Bunker=yes`, while standard refinery docking uses `+0x418`, `+0x5A4`, mission `0x10`, and the harvester unload FSM.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard `0x0E` DockUnload reply burst | verified | `BuildingClass::Receive_Radio @ 0x0043C2D0` | none |
| Radio `0x18` field write | verified | `TechnoClass::Receive_Radio @ 0x006F4B72` writes `+0x418` | none |
| Radio `0x16` ordering and `+0x4C` target | verified | `UnitClass::Receive_Radio @ 0x00737430`; `DriveLocomotionClass__Do_Turn @ 0x004B0EF0` | non-Drive variants out-of-scope |
| Pad-arrival `0x15` vs locomotor stop order | verified | `UnitClass::PerCellProcess @ 0x00739EC0` | none |
| `0x15` mission transition | verified | `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` | none |
| Harvester unload RateTimer wait and `+0x6D1` init | verified | `UnitClass::Mission_Deploy_Building @ 0x0073DF56..0x0073E09D` | full unload economics out-of-scope |
| Reciprocal `+0x2E4` writer | verified | `FUN_00458E50 @ 0x00459301..0x00459337` | none for writer identity |
| Whether reciprocal `+0x2E4` writer is standard refinery path | verified absent | caller gate `BuildingType+0x16AB` Bunker; GAREFN/NAREFN use `+0x16B3/+0x16BB` | none |
| Exact post-unload exit anchor for stock CMIN | deferred | out-of-scope slot 5 target | separate swarm slot |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What writes unit/building `+0x2E4` reciprocally? `FUN_00458E50` case 5 writes `building+0x2E4=unit` and `unit+0x2E4=building` at `0x00459301`/`0x0045930F`; active only under `Bunker=yes`.

[RESOLVED] OQ-2 - Does standard CMIN/HARV refinery pad arrival use that `+0x2E4` writer? No. GAREFN/NAREFN are `DockUnload=yes` and `Refinery=yes`; the only caller of `FUN_00458E50` is gated by `BuildingType+0x16AB` (`Bunker=yes`).

[RESOLVED] OQ-3 - What field does radio `0x18` set? `TechnoClass::Receive_Radio` writes byte `this+0x418 = 1`, not `+0x2E4`.

[RESOLVED] OQ-4 - What is the pad-arrival order? `FootClass::PerCellProcess(2)` -> radio `0x15` -> locomotor slot `+0x5C` stop/power-off in `UnitClass::PerCellProcess`.

[RESOLVED] OQ-5 - Is radio `0x16` before or after final pad-arrival `0x15`? It is sent in the `0x0E` admission reply burst before physical pad arrival; it can also cascade a directed `0x15` if the unit is already stopped, destination is a building, `+0x418 != 0`, and mission is `7`.

[DEFERRED] OQ-6 - Exact stock post-unload exit anchor and whether a separate Force_Track path fires for CMIN after ore drain. Category: out-of-scope; assigned to swarm slot 5.

## Sources

- Ghidra `decompile_function 739ec0` - `UnitClass::PerCellProcess`.
- Ghidra `decompile_function 737430` - `UnitClass::Receive_Radio`.
- Ghidra `decompile_function 43c2d0` - `BuildingClass::Receive_Radio`.
- Ghidra `decompile_function` and `disassemble_function 6f4ab0` - `TechnoClass::Receive_Radio`.
- Ghidra `decompile_function` and `disassemble_function 73d630` - `UnitClass::Mission_Deploy_Building`.
- Ghidra `decompile_function` and `disassemble_function 458e50` - Bunker enter sub-FSM and `+0x2E4` writer.
- Ghidra `get_function_xrefs address=458e50` - sole call from `BuildingClass::MissionRepairAndProduce`.
- Ghidra `decompile_function 44b7a3` / `0x0044B780` - caller gate on `BuildingType+0x16AB`.
- Ghidra `decompile_function 45fe50` - `BuildingTypeClass_ReadINI_Water` maps `Bunker` to `+0x16AB`, `DockUnload` to `+0x16B3`, `Refinery` to `+0x16BB`.
- Ghidra `decompile_function 4b0ef0` - DriveLocomotion slot `+0x4C` calls `RateTimer__Set`.
- `ini/rulesmd.ini` lines: CMIN/HARV dock/harvester keys, GAREFN/NAREFN `DockUnload`/`Refinery`, NATBNK `Bunker=yes`.
- Prior docs consulted: `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`, `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`, `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`.
