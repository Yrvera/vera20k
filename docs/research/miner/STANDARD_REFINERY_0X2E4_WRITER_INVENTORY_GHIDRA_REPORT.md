# Standard Refinery +0x2E4 Writer Inventory - Ghidra Research Report

**Address(es):** `0x00458E50`, `0x004593A0`, `0x00459470`, `0x004595C0`, `0x0073D630`, `0x00739EC0`, `0x0043C2D0`, `0x006F4AB0`, `0x00707859`, `0x0070BF50`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Direct and docking-relevant indirect writers/clearers of `UnitClass/BuildingClass/TechnoClass +0x2E4` around stock `CMIN/HARV -> GAREFN/NAREFN` docking, Bunker entry, interrupts, destruction/temporal cleanup, and save/load pointer fixup.
**Non-Scope:** Exact runtime value/source of `DAT_0089F6A0`, full unload credit math, non-docking uses of unrelated classes whose layouts also contain a `+0x2E4` field.
**Confidence:** High for docking-relevant writer/clearer inventory; Medium for global no-writer proof because some non-docking class layouts also have `+0x2E4` fields.
**Active in YR:** Yes for the examined stock refinery and Bunker paths; conditional details are called out per row.

## 1. Overview

Stock `CMIN/HARV -> GAREFN/NAREFN` docking does **not** write a reciprocal `unit+0x2E4 <-> building+0x2E4` pair during the normal accepted refinery dock path. The standard unload FSM is reached through the zero-`unit+0x2E4` branch of `UnitClass::Mission_Deploy_Building @ 0x0073D630` and rediscovers the refinery by cell lookup using `DAT_0089F6A0`.

The only direct gameplay writer found that sets both sides of a reciprocal `+0x2E4` link is `FUN_00458E50` case 5, called from `BuildingClass::MissionRepairAndProduce` only when `BuildingType+0x16AB` (`Bunker=yes`) is set. Refinery exit/interrupt helpers clear `+0x2E4`, but the stock DockUnload handoff does not set it.

## 2. Class Layout / Key Offsets

| Field | Offset | Meaning in this slice | Active in YR |
|---|---:|---|---|
| `TechnoClass+0x2E4` / `UnitClass+0x2E4` / `BuildingClass+0x2E4` | `0x2E4` (`[0xB9]` in `int*` decompile) | Dock/garrison-style alternate link field used by Bunker and by release/cleanup helpers | Conditional |
| `BuildingClass+0x718` | `0x718` | Bunker/exit helper state; cleared with `+0x2E4` in teardown paths | Conditional |
| `UnitClass+0x418` | `0x418` | Radio `0x18` flag set by `TechnoClass::Receive_Radio`; not `+0x2E4` | Yes |
| `BuildingType+0x16AB` | `0x16AB` | `Bunker=yes`; gates `FUN_00458E50` writer path | Yes for `NABNKR`, not GAREFN/NAREFN |
| `BuildingType+0x16B3` | `0x16B3` | `DockUnload=yes`; stock refinery radio `0x15` handoff | Yes for GAREFN/NAREFN |
| `BuildingType+0x16BB` | `0x16BB` | `Refinery=yes`; stock unload FSM checks refinery identity | Yes for GAREFN/NAREFN |
| `DAT_0089F6A0/2` | global shorts | Cell offset used by zero-`unit+0x2E4` unload FSM to rediscover refinery | Yes |

## 3. Writer / Clearer Inventory

| Site | Operation | Classification | Active in YR | Evidence |
|---|---|---|---|---|
| `TechnoClass::Constructor @ 0x006F2D00` | initializes `param_1[0xB9] = 0` | construction/default | Yes for all Techno-derived objects | decompile shows `param_1[0xb9] = 0` |
| `FUN_00458E50` case 5 | `building+0x2E4 = unit`; `unit+0x2E4 = building`; sets unit mission `5,1` | **Bunker writer** | Conditional: only when caller building has `Type+0x16AB` | `0x00459301`, `0x0045930F`; caller `0x0044B7A3` gated by `param_1->Type[0x16ab]` |
| `BuildingClass::UndockUnit @ 0x004593A0` | clears `unit+0x2E4`, clears `building+0x2E4`, sends radio `3` | interrupt/sell/damage/temporal exit clearer | Conditional: only when `building+0x2E4 != 0` and linked object `WhatAmI()==1` | `0x00459450`, `0x0045945C`; callers include `BuildingClass::Sell`, `BuildingClass::ReceiveDamage`, `TemporalClass::Update` |
| `FUN_00459470` | clears `(*(building+0x2E4))+0x2E4`, clears `building+0x2E4`, clears `building+0x718`, sets building mission `5` | damage/temporal/super cleanup clearer | Conditional: linked building/unit destruction/teleport-like cleanup | `0x00459586`, `0x00459592`; xrefs from `SuperClass::Launch`, `TemporalClass::Update`, `UnitClass::ReceiveDamage` |
| `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` | reads `building+0x2E4`; if non-null clears `unit+0x2E4`, later clears `building+0x2E4` and `+0x718` | production/exit clearer | Conditional: only nonzero-link release branch; not the zero-link stock unload FSM | `0x004596B1` read, `0x004596E6` unit clear, `0x00459814` building clear |
| `TechnoClass::PointerExpired @ 0x00707859` | if expired pointer equals `this+0x2E4`, clear only when `g_MapEditorMode != 0` | pointer expiry/editor cleanup | Conditional: map editor/load cleanup; not normal skirmish | `if ((param_2 == param_1[0xb9]) && (g_MapEditorMode != 0)) param_1[0xb9]=0` |
| `FUN_0070BF50` | passes `param_1+0x2E4` to `FUN_006CF240` pointer fixup | save/load pointer fixup | Conditional: save/load/object remap, not gameplay writer | decompile calls `FUN_006cf240(&DAT_00b0c110,param_1 + 0x2e4)` |
| `0x0074B864..0x0074B91D` | pointer remap compares fields to old pointer and writes replacement | save/load pointer fixup | Conditional: pointer remapping only | assembly `LEA [ESI+0x2E4]`, compare/write sequence near `0x0074B907..0x0074B90F` |

## 4. Negative Inventory For Stock Refinery Path

| Function / branch | Result | Active in YR | Evidence |
|---|---|---|---|
| `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` | Does not write `+0x2E4`; accepted DockUnload path sends `0x12`, then `0x18`, then `0x16` | Yes for stock refineries | full decompile has no `+0x2E4`; hardcoded target `NW+(3,1)`; `DockUnload=yes` branch |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` | Does not write `+0x2E4`; if `DockUnload=yes`, sets sender mission `0x10` | Yes for stock refineries | case `0x15` calls sender vtable `+0x1E8(0x10,0)` |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x18` | Writes `byte +0x418 = 1`, not `+0x2E4` | Yes | decompile case `0x18`: `*(undefined1 *)&param_1[6].UniqueID = 1` |
| `UnitClass::PerCellProcess @ 0x00739EC0` pad arrival | Does not write `+0x2E4`; order is `FootClass::PerCellProcess(2)`, radio `0x15`, locomotor `+0x5C` | Yes | decompile arrival block around `0x0073A4D5..0x0073A52B` |
| `TechnoClass/UnitClass::Set_Destination @ 0x00741970` | Does not write `+0x2E4`; sends `0x0E` and manages destination/radio contacts | Yes | full decompile checked; no `+0x2E4` store in dock-accept branch |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` stock unload FSM | Does not set `+0x2E4`; zero-`unit+0x2E4` path uses `DAT_0089F6A0` cell lookup | Yes for CMIN/HARV | entry `CMP [ESI+0x2E4],0; JZ 0x0073D6E6`; lookup sites `0x0073E013`, `0x0073E2C8`, `0x0073E181` |

## 5. Core Logic

### 5.1 Stock refinery path

```text
Building Receive_Radio(0x0E):
  if DockUnload/refinery accepts:
    send MOVE_TO_CELL(0x12) to building NW+(3,1)
    send radio 0x18 to unit
    send radio 0x16 to unit

Unit PerCellProcess on pad:
  FootClass::PerCellProcess(2)
  send radio 0x15 to refinery
  locomotor slot +0x5C

Building Receive_Radio(0x15):
  if DockUnload=yes:
    unit.SetMission(0x10, 0)

Unit Mission_Deploy_Building:
  if unit+0x2E4 == 0:
    run harvester unload FSM
    rediscover refinery from current cell + DAT_0089F6A0/2
  else:
    call ReleaseDockedHarvester-style teardown branch
```

No step above writes reciprocal `+0x2E4` for stock GAREFN/NAREFN docking.

### 5.2 Bunker writer path

`BuildingClass::MissionRepairAndProduce @ 0x0044B7A3` calls `FUN_00458E50` only when `BuildingType+0x16AB` is nonzero. Inside `FUN_00458E50` case 5:

```text
building+0x2E4 = unit
unit+0x2E4 = building
unit+0x214 = -1
unit.vtable+0x150()
building+0x718 = 6
unit.SetMission(5, 1)
```

That is the reciprocal writer previously mistaken as a standard refinery writer. Stock GAREFN/NAREFN use `DockUnload=yes` (`+0x16B3`) and `Refinery=yes` (`+0x16BB`), not `Bunker=yes` (`+0x16AB`).

## 6. INI Keys

| INI key | Stock value | Effect | Active in YR |
|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | Allows CMIN to target stock refineries | Yes, `rulesmd.ini:7361` |
| `[CMIN] Harvester` | `yes` | Enables harvester unload FSM | Yes, `rulesmd.ini:7364` |
| `[HARV] Dock` | `NAREFN,GAREFN` | Allows HARV to target stock refineries | Yes, `rulesmd.ini:8225` |
| `[HARV] Harvester` | `yes` | Enables harvester unload FSM | Yes, `rulesmd.ini:8228` |
| `[GAREFN] DockUnload` | `yes` | Radio `0x15` sets mission `0x10` | Yes, `rulesmd.ini:11726` |
| `[GAREFN] Refinery` | `yes` | Unload FSM refinery check | Yes, `rulesmd.ini:11727` |
| `[NAREFN] DockUnload` | `yes` | Radio `0x15` sets mission `0x10` | Yes, `rulesmd.ini:12519` |
| `[NAREFN] Refinery` | `yes` | Unload FSM refinery check | Yes, `rulesmd.ini:12520` |
| `[NABNKR] Bunker` | `yes` | Activates `FUN_00458E50` reciprocal writer path | Yes, `rulesmd.ini:13732` |

## 7. Integration Points

- `BuildingClass::Receive_Radio` owns stock refinery accept/handoff messages but never sets `+0x2E4`.
- `UnitClass::PerCellProcess` owns physical pad-arrival radio `0x15` and locomotor stop/update but never sets `+0x2E4`.
- `TechnoClass::Receive_Radio` owns radio `0x18` flag `+0x418`; this is not a dock link.
- `UnitClass::Mission_Deploy_Building` is the stock unload handler. Its zero-`+0x2E4` path is the active stock unload FSM; its nonzero path is separate release/teardown.
- `BuildingClass::UndockUnit`, `FUN_00459470`, and `ReleaseDockedHarvester` are clearers/exit helpers, not stock reservation writers.

## 8. Current Rust Implementation Status

No Rust files were modified. Current Rust uses explicit miner/refinery IDs and dock-phase state rather than emulating `+0x2E4` directly. For stock refinery parity, the evidence supports keeping the standard unload link independent from a reciprocal `+0x2E4` pair; if a Bunker/garrison equivalent is implemented later, that is the path that needs reciprocal link semantics.

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct stores to docking-relevant `+0x2E4` | verified | byte/disassembly search for stores at `0x00459301`, `0x0045930F`, `0x00459450`, `0x0045945C`, `0x00459586`, `0x00459592`, `0x004596E6`, `0x00459814`, `0x00707859` | none for docking scope |
| `FUN_00458E50` reciprocal writer | verified | decompile `0x00458E50`; caller `0x0044B7A3` | none |
| Bunker gate | verified | `BuildingClass::MissionRepairAndProduce` checks `Type+0x16AB`; `rulesmd.ini:13732` | none |
| `BuildingClass::Receive_Radio` stock cases | verified | decompile `0x0043C2D0` | none |
| `TechnoClass::Receive_Radio` case `0x18` | verified | decompile `0x006F4AB0` | none |
| `UnitClass::PerCellProcess` pad arrival | verified | decompile `0x00739EC0` | none |
| `UnitClass::Mission_Deploy_Building` entry split | verified | decompile `0x0073D630`; sibling report `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md` | none for writer question |
| `BuildingClass::UndockUnit` clearer | verified | decompile `0x004593A0` | none |
| `FUN_00459470` clearer | verified | decompile `0x00459470`; xrefs | none |
| `BuildingClass::ReleaseDockedHarvester` clearer | verified | decompile `0x004595C0` | exact runtime path frequency belongs to sibling slot |
| Save/load pointer fixup | verified | decompile `0x0070BF50`; assembly `0x0074B864..0x0074B90F` | no gameplay impact expected |
| Unrelated class `+0x2E4` fields | touched-not-exhausted | examples: `AnimTypeClass`, `BulletTypeClass`, `VoxelAnimTypeClass` decompiles | out of scope; not Techno/Unit/Building docking |

## 10. Open Questions - Final State

[RESOLVED] OQ-1 - Does stock CMIN/HARV refinery docking write reciprocal `unit/building +0x2E4`? No direct or indirect writer was found in the stock path. The active unload FSM is the zero-`unit+0x2E4` branch and uses `DAT_0089F6A0` cell lookup. Evidence: `0x0073D630`, `0x0043C2D0`, `0x00739EC0`, `0x006F4AB0`, `0x00741970`.

[RESOLVED] OQ-2 - Where is the reciprocal writer that previous docs saw? `FUN_00458E50` case 5 writes both sides, but it is gated by `BuildingType+0x16AB` (`Bunker=yes`) from `BuildingClass::MissionRepairAndProduce`. Evidence: `0x00459301`, `0x0045930F`, caller `0x0044B7A3`.

[RESOLVED] OQ-3 - Which functions clear `+0x2E4` around docking? `UndockUnit`, `FUN_00459470`, `ReleaseDockedHarvester`, and map-editor `TechnoClass::PointerExpired` clear it under their respective conditions. Evidence: `0x004593A0`, `0x00459470`, `0x004595C0`, `0x00707859`.

[RESOLVED] OQ-4 - Is radio `0x18` the missing writer? No. It writes `+0x418`, then forwards radio `0x18`; it does not touch `+0x2E4`. Evidence: `TechnoClass::Receive_Radio @ 0x006F4AB0`.

[RESOLVED] OQ-5 - Is save/load a gameplay writer? No. `FUN_0070BF50` and the `0x0074B864` pointer-remap block include `+0x2E4`, but they are pointer fixup/rehydration surfaces rather than live docking behavior. Evidence: calls to `FUN_006CF240(&DAT_00B0C110, this+0x2E4)` and compare/replace assembly.

## Sources

- Ghidra `search_byte_patterns "E4 02 00 00"` and focused store searches for `C7/89/8D ... +0x2E4`.
- Ghidra `decompile_function 0x00458E50` - Bunker link helper.
- Ghidra `get_function_xrefs 0x00458E50` - caller `BuildingClass::MissionRepairAndProduce @ 0x0044B7A3`.
- Ghidra `decompile_function 0x0044B7A3` - `Type+0x16AB` gate.
- Ghidra `decompile_function 0x004593A0` - `BuildingClass::UndockUnit`.
- Ghidra `decompile_function 0x00459470` - cleanup clearer.
- Ghidra `get_function_xrefs 0x00459470` - `SuperClass::Launch`, `TemporalClass::Update`, `UnitClass::ReceiveDamage`.
- Ghidra `decompile_function 0x004595C0` - `BuildingClass::ReleaseDockedHarvester`.
- Ghidra `decompile_function 0x0043C2D0` - `BuildingClass::Receive_Radio`.
- Ghidra `decompile_function 0x006F4AB0` - `TechnoClass::Receive_Radio`.
- Ghidra `decompile_function 0x00739EC0` - `UnitClass::PerCellProcess`.
- Ghidra `decompile_function 0x0073D630` - `UnitClass::Mission_Deploy_Building`.
- Ghidra `decompile_function 0x00741970` - `Unit/Techno Set_Destination` dock flow.
- Ghidra `decompile_function 0x00707859` - `TechnoClass::PointerExpired`.
- Ghidra `decompile_function 0x0070BF50` - save/load pointer fixup.
- Prior docs checked: `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`, `CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md`, `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini`.
