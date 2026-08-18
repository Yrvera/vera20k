# Mission_Deploy_Building DAT_0089F6A0 Refinery Lookup - Ghidra Research Report

**Address(es):** `0x0073D630` primary, lookup helper `0x0047C520`, release helper `0x004595C0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** How `UnitClass::Mission_Deploy_Building` uses `DAT_0089F6A0` / adjacent-cell lookup to recover the stock refinery during CMIN/HARV unload, and whether that replaces, depends on, or is separate from `+0x2E4`.  
**Non-Scope:** Exact `DAT_0089F6A0` initialization source/value, full ore-credit math, full dock arrival writer inventory, and non-refinery enter systems.  
**Confidence:** High for branch dependency and lookup use; Medium for the runtime origin of `DAT_0089F6A0` because debugger memory was unavailable.  
**Active in YR:** Yes for stock `[CMIN]`/`[HARV]` unloading at `[GAREFN]`/`[NAREFN]`.

## 1. Overview

`DAT_0089F6A0` is used inside the harvester unload FSM while the unit-side `+0x2E4` pointer is zero. The function adds the two signed 16-bit global components to the miner's current map cell, gets that `CellClass`, then calls `Look_up_building_in_cell` to find the refinery object in the adjacent/offset cell.

This lookup does not depend on `unit+0x2E4`. It is the active way states 1, 3, and 4 recover the refinery during the unload loop. The nonzero-`+0x2E4` branch is separate: it finds the building at the unit cell and calls `BuildingClass::ReleaseDockedHarvester`, whose callee reads/clears `building+0x2E4`.

## 2. Class Layout / Key Offsets

| Field / global | Offset | Purpose in this slice | Active in YR |
|---|---:|---|---|
| `UnitClass+0x2E4` / `param_1[0xB9]` | `0x2E4` | Entry dispatcher: zero enters harvester unload FSM; nonzero enters release/teardown branch | Yes, but only release branch uses nonzero |
| `UnitClass+0xBC` / `param_1[0x2F]` | `0xBC` | Mission substate; harvester path uses states 1, 3, 4 here | Yes |
| `UnitClass+0x6C4` / `param_1[0x1B1]` | `0x6C4` | `UnitTypeClass*`; `+0xE0E` gates harvester path, `+0xE0F` gates weeder variant | Yes |
| `UnitClass+0x6D1` | `0x6D1` | One-shot unload initialization flag; when zero, state init runs and then sets it to one | Yes |
| `DAT_0089F6A0` | `0x0089F6A0` | signed short X offset added to current cell before refinery lookup | Yes |
| `DAT_0089F6A2` | `0x0089F6A2` | signed short Y offset added to current cell before refinery lookup | Yes |
| `BuildingType+0x16B3` | `0x16B3` | `DockUnload=yes`; radio `0x15` sends the miner to mission `0x10` | Yes for GAREFN/NAREFN |
| `BuildingType+0x16BB` | `0x16BB` | `Refinery=yes`; checked in state 4 delay/close branch | Yes for GAREFN/NAREFN |
| `CellClass+0xE4` | `0xE4` | First object in cell linked list scanned by `Look_up_building_in_cell` | Yes |

## 3. Core Logic

### 3.1 Entry split

Assembly at the start of `UnitClass::Mission_Deploy_Building`:

```asm
0073d63b: CMP dword ptr [ESI + 0x2e4], EBX
0073d641: JZ  0x0073d6e6
```

`EBX` is zero. Therefore:

- `unit+0x2E4 == 0` jumps to `0x0073D6E6`, the main deploy/harvester dispatcher and, for `Harvester=yes`, the unload FSM.
- `unit+0x2E4 != 0` falls through to the release branch at `0x0073D647..0x0073D66D`.

This corrects the easy-to-misread older note that described the branch direction backward. The `DAT_0089F6A0` lookup is in the zero-`+0x2E4` path.

### 3.2 DAT_0089F6A0 lookup sites

The same lookup pattern appears three times in the active harvester path:

```text
current_cell = unit.vtable+0x1B8()
lookup_cell.x = current_cell.x + *(short*)0x0089F6A0
lookup_cell.y = current_cell.y + *(short*)0x0089F6A2
cell = MapClass::Get_CellClass(lookup_cell)
building = Look_up_building_in_cell(cell)
```

Verified sites:

| Site | State / use | Evidence | Active in YR |
|---|---|---|---|
| `0x0073E013..0x0073E05A` | first-entry/init: find refinery and set anim slot 7 if found | `MOV word ptr [0x0089f6a0]`, `ADD`, `MapClass__Get_CellClass`, `CALL 0x0047C520` | Yes |
| `0x0073E2C8..0x0073E306` | state 3 dump loop: find refinery each deposit tick | same pattern, then `this_00` drives credits/anims | Yes |
| `0x0073E181..0x0073E1C6` | state 4 wait/close branch: find refinery to test `Refinery=yes` and `+0x57C` | same pattern, then reads `Type+0x16BB` and `building+0x57C` | Yes |

The state 3 null-building branch is also active: if lookup returns null, the function checks `PathType__Has_Valid_Steps`; if valid it sends radio command `3`, then sets mission `10`/Harvest with queued flag `1`.

### 3.3 Lookup helper

`Look_up_building_in_cell @ 0x0047C520` scans the cell's object list:

```text
if g_GameActive:
    for obj = cell+0xE4; obj != null; obj = obj+0x30:
        if obj.WhatAmI() == 6:
            return obj
return null
```

It does not read `unit+0x2E4`, `building+0x2E4`, the unit's radio link, or target fields. Its only inputs are the `CellClass*` and the active object's linked list. Active in YR: Yes; this helper is generic and directly called by `Mission_Deploy_Building` at the lookup sites above.

### 3.4 Nonzero +0x2E4 branch

When `unit+0x2E4 != 0`, `Mission_Deploy_Building` does not run the `DAT_0089F6A0` lookup first. It:

1. calls unit vtable `+0x1BC`,
2. calls `Look_up_building_in_cell`,
3. if a building exists, repeats that lookup,
4. calls `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`.

The release callee then reads `building+0x2E4` as the docked unit pointer. If it is null, the building clears `+0x718`, sets mission `5`, and returns. If it points to a UnitClass object, it clears `unit+0x2E4`, powers on locomotion, performs the forced-track/exit destination sequence, then clears `building+0x2E4` and `building+0x718`.

This branch depends on `+0x2E4`; the unload loop does not.

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | CMIN can target both stock refineries | Yes (`rulesmd.ini:7361`) |
| `[CMIN] Harvester` | `yes` | enables harvester unload path | Yes (`rulesmd.ini:7364`) |
| `[CMIN] Teleporter` | `yes` | chrono movement elsewhere; not checked by `Mission_Deploy_Building` lookup | Yes (`rulesmd.ini:7396`) |
| `[HARV] Dock` | `NAREFN,GAREFN` | HARV can target both stock refineries | Yes (`rulesmd.ini:8225`) |
| `[HARV] Harvester` | `yes` | enables harvester unload path | Yes (`rulesmd.ini:8228`) |
| `[GAREFN] DockUnload` | `yes` | radio `0x15` sets sender mission `0x10` | Yes (`rulesmd.ini:11726`) |
| `[GAREFN] Refinery` | `yes` | state 4 branch checks `Type+0x16BB` | Yes (`rulesmd.ini:11727`) |
| `[NAREFN] DockUnload` | `yes` | radio `0x15` sets sender mission `0x10` | Yes (`rulesmd.ini:12519`) |
| `[NAREFN] Refinery` | `yes` | state 4 branch checks `Type+0x16BB` | Yes (`rulesmd.ini:12520`) |

## 5. Integration Points

- `UnitClass::PerCellProcess @ 0x00739EC0` sends radio `0x15` on pad arrival.
- `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` checks `DockUnload=yes` and calls sender vtable `+0x1E8` with mission `0x10`.
- `UnitClass::Mission_Deploy_Building @ 0x0073D630` is the mission `0x10` handler.
- `Mission_Deploy_Building` uses `DAT_0089F6A0` lookup in the zero-`+0x2E4` harvester FSM.
- `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` is called only from `Mission_Deploy_Building` and is the nonzero-`+0x2E4` teardown/exit helper.

## 6. Current Rust Implementation Status

Rust currently models this with explicit IDs rather than a cell-list rediscovery:

| Rust area | Evidence | Status vs this slice |
|---|---|---|
| Miner dock phase state | `src/sim/miner/mod.rs` `RefineryDockPhase` | Represents gamemd mission/radio flow with a custom sub-FSM |
| On-pad link equivalent | `src/sim/miner/miner_dock.rs` `RefineryDockContacts::on_pad` | Explicit building->miner map equivalent to a docked/on-pad link |
| Unload loop | `src/sim/miner/miner_dock_sequence.rs` `phase_unloading` | Uses `reserved_refinery` rather than `DAT_0089F6A0` cell lookup |
| Pad arrival transition | `src/sim/miner/miner_dock_sequence.rs` `phase_pivoting` | Seeds unloading when pad/pivot completes |

No Rust changes were made.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building` entry split | verified | `0x0073D63B CMP [ESI+0x2E4],0`; `0x0073D641 JZ 0x0073D6E6` | none |
| `DAT_0089F6A0` init lookup | verified | `0x0073E013..0x0073E05A` | exact global initialization source deferred |
| `DAT_0089F6A0` state 3 dump lookup | verified | `0x0073E2C8..0x0073E306` | none for branch dependency |
| `DAT_0089F6A0` state 4 wait lookup | verified | `0x0073E181..0x0073E1C6` | none for branch dependency |
| `Look_up_building_in_cell` helper | verified | decompile `0x0047C520` | none |
| nonzero `unit+0x2E4` release branch | verified | `0x0073D647..0x0073D66D`, `0x004595C0` | writer identity covered by sibling slots |
| exact `DAT_0089F6A0` value and load source | deferred | debugger read unavailable; prior doc marks source as likely docking offset | next narrow report if needed |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Is `DAT_0089F6A0` lookup on the zero or nonzero `unit+0x2E4` branch? It is on the zero branch. Evidence: `0x0073D63B CMP [ESI+0x2E4],0`; `0x0073D641 JZ 0x0073D6E6`; lookup sites are below `0x0073DEE0`.

[RESOLVED] OQ-2 - Does the lookup depend on `unit+0x2E4` or `building+0x2E4`? No. The lookup adds global shorts to the current cell and scans the resulting cell's object list. Evidence: `0x0073E013..0x0073E05A`, `0x0073E2C8..0x0073E306`, `0x0047C520`.

[RESOLVED] OQ-3 - What is the nonzero `unit+0x2E4` branch used for? It finds the building at the unit cell and calls `BuildingClass::ReleaseDockedHarvester`, which reads/clears `building+0x2E4`. Evidence: `0x0073D647..0x0073D66D`, `0x004595C0`.

[RESOLVED] OQ-4 - Is this active for stock CMIN/HARV? Yes. `[CMIN]` and `[HARV]` have `Dock=NAREFN,GAREFN` and `Harvester=yes`; `[GAREFN]`/`[NAREFN]` have `DockUnload=yes` and `Refinery=yes` in `rulesmd.ini`.

[DEFERRED] OQ-5 - What exact runtime value/source initializes `DAT_0089F6A0`? Category: out-of-scope. Static evidence proves use and branch dependency; debugger memory was unavailable in this session.

## Sources

- Ghidra `decompile_function 0x0073D630` - `UnitClass__Mission_Deploy_Building`
- Ghidra `get_assembly_context` for `0x0073D63B`, `0x0073E013`, `0x0073E2C8`, `0x0073E181`
- Ghidra `decompile_function 0x0047C520` - `Look_up_building_in_cell`
- Ghidra `decompile_function 0x004595C0` - `BuildingClass__ReleaseDockedHarvester`
- Ghidra `decompile_function 0x00739EC0` - `UnitClass__PerCellProcess`
- Ghidra `decompile_function 0x0043C2D0` - `BuildingClass__Receive_Radio`
- `ini/rulesmd.ini:7361`, `7364`, `7396`, `8225`, `8228`, `11726`, `11727`, `12519`, `12520`
- Prior docs: `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`, `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`
