# DAT_0089F6A0 Runtime Source and Value - Ghidra Research Report

**Investigation date:** 2026-05-21  
**Investigation mode:** exhaustive-slice for the `DAT_0089F6A0` value/source and its stock refinery DockUnload consumer semantics.  
**Primary functions:** `Foundation_direction_table_init @ 0x0049F2F0`, CRT initializer runner `0x007CBDAF/0x007CBED3`, `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `Look_up_building_in_cell @ 0x0047C520`.  
**Confidence:** HIGH for static init value/source and Mission_Deploy_Building consumer semantics; PARTIAL only for live memory observation because the debugger server was unavailable.  
**Active in YR:** Yes. The initializer is reached before `WinMain`, and the Mission `0x10` harvester unload FSM is active for stock `[CMIN]`/`[HARV]` unloading at `[GAREFN]`/`[NAREFN]`.

## Summary

`DAT_0089F6A0` is not loaded from refinery `DockingOffset%d`, `QueueingCell`, or any stock refinery INI key. It is a hardcoded member of the global 8-neighbor cell direction table initialized at process startup by `Foundation_direction_table_init @ 0x0049F2F0`.

The exact initialized dword at `0x0089F6A0` is `0x0000FFFF`, i.e. signed 16-bit `(dx=-1, dy=0)`. `DAT_0089F6A2` is the high word of the same dword and is therefore `0`. In the zero-link stock DockUnload path, `UnitClass::Mission_Deploy_Building` adds this offset to the miner's current map cell before calling `MapClass::Get_CellClass` and `Look_up_building_in_cell`. The semantic meaning is: while the miner is sitting on the accepted dock/pad cell, look one cell west for a refinery building cell.

This corrects the prior medium-confidence wording in `miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`, which described the global as a likely runtime-baked `[GAREFN] DockingOffset0=` value. The actual binary source is a hardcoded static constructor.

## Verified Init Path

### Startup dispatch

- **Verified binary finding:** `entry @ 0x007CD80F` calls `FUN_007CBDAF @ 0x007CBDAF` before `WinMain`.
  - Evidence: Ghidra decompile of `entry`, and assembly context at `0x007CD8B4 CALL 0x007CBDAF`.
  - **Active in YR:** Yes. This is the normal process entry path before game code reaches `WinMain`.

- **Verified binary finding:** `FUN_007CBDAF` calls `FUN_007CBED3(&0x00815DA8,&0x00815DBC)` and then `FUN_007CBED3(&0x00812000,&0x00815DA4)`.
  - Evidence: Ghidra decompile and assembly context at `0x007CBDBA..0x007CBDD3`.
  - **Active in YR:** Yes. This is standard MSVC-style constructor table iteration in the shipped executable.

- **Verified binary finding:** `FUN_007CBED3 @ 0x007CBED3` iterates dword function pointers from `param_1` up to `param_2` and calls every non-null pointer.
  - Evidence: decompile of `0x007CBED3`.
  - **Active in YR:** Yes, reached by `0x007CBDAF` before `WinMain`.

### Constructor table entry

- **Verified binary finding:** the constructor table contains pointer `0x0049F2F0` at `0x00812BAC`.
  - Evidence: PE/data scan of `gamemd.exe`: bytes around `0x00812BAC` are `f0 f2 49 00`; Ghidra `get_function_xrefs(0x0049F2F0)` reports data xref from `0x00812BAC`.
  - **Active in YR:** Yes. `0x00812BAC` lies inside the `0x00812000..0x00815DA4` range iterated by `FUN_007CBED3`.

### Value written

- **Verified binary finding:** `Foundation_direction_table_init @ 0x0049F2F0` writes `DAT_0089F6A0 = 0x0000FFFF`.
  - Evidence: Ghidra decompile of `0x0049F2F0`; local disassembly shows `0x0049F38E MOV dword ptr [0x0089F6A0], EDX`, where prior instructions built `EDX` from low word `0xFFFF` and high word `0x0000`.
  - **Active in YR:** Yes, via the constructor-table path above.

- **Verified binary finding:** the same constructor initializes the surrounding 8-neighbor table as signed 16-bit `(dx,dy)` dwords:
  - `0x0089F688 = 0xFFFF0000` -> `(0,-1)`
  - `0x0089F68C = 0xFFFF0001` -> `(1,-1)`
  - `0x0089F690 = 0x00000001` -> `(1,0)`
  - `0x0089F694 = 0x00010001` -> `(1,1)`
  - `0x0089F698 = 0x00010000` -> `(0,1)`
  - `0x0089F69C = 0x0001FFFF` -> `(-1,1)`
  - `0x0089F6A0 = 0x0000FFFF` -> `(-1,0)`
  - `0x0089F6A4 = 0xFFFFFFFF` -> `(-1,-1)`
  - Evidence: decompile/disassembly of `0x0049F2F0`.
  - **Active in YR:** Yes. The table is globally reused by multiple map/foundation helpers; the `0x0089F6A0` member is directly read by stock DockUnload.

- **Verified binary finding:** direct-immediate reference scan found one direct write to `0x0089F6A0`, at `0x0049F38E`; all stock DockUnload references at `0x0073E022/0x0073E195/0x0073E2D5` are reads.
  - Evidence: local PE scan over `gamemd.exe` for little-endian immediate `A0 F6 89 00`, with Capstone operand access classification.
  - **Active in YR:** Yes for the initializer write and the Mission_Deploy_Building reads. This does not exclude a debugger-only/runtime external write, but no second direct binary writer was found.

## Stock DockUnload Consumer Semantics

- **Verified binary finding:** `UnitClass::Mission_Deploy_Building` state-3 initialization reads the miner current cell, adds `(dx=-1,dy=0)`, gets that cell, then calls `Look_up_building_in_cell`.
  - Evidence: `0x0073E022` reads word `0x0089F6A0`, `0x0073E030` reads word `0x0089F6A2`, `0x0073E053` calls `MapClass::Get_CellClass`, `0x0073E05A` calls `0x0047C520`.
  - **Active in YR:** Yes, gated by `UnitTypeClass+0xE0E Harvester=yes`; stock `[CMIN]` and `[HARV]` have `Harvester=yes`.

- **Verified binary finding:** state 3, the per-dump loop, repeats the same west-cell lookup before each dump gate.
  - Evidence: `0x0073E2D5` adds word `0x0089F6A0`, `0x0073E2DC` adds word `0x0089F6A2`, `0x0073E2FF` calls `MapClass::Get_CellClass`, `0x0073E306` calls `Look_up_building_in_cell`.
  - **Active in YR:** Yes for every stock ore dump tick while the miner is in Mission `0x10`.

- **Verified binary finding:** state 4, the depart/guard state, also repeats the same west-cell lookup to find the refinery, test `Refinery=yes`, and inspect the slot-8 animation pointer.
  - Evidence: `0x0073E195` adds word `0x0089F6A0`, `0x0073E19C` adds word `0x0089F6A2`, `0x0073E1BF` calls `MapClass::Get_CellClass`, `0x0073E1C6` calls `Look_up_building_in_cell`.
  - **Active in YR:** Yes for stock completion of `[CMIN]`/`[HARV]` DockUnload; stock GAREFN/NAREFN normally have no active `ProductionAnim`, so the guard usually does not wait.

- **Verified binary finding:** the lookup helper itself does not know about refineries, docking offsets, or radio links. It scans `CellClass+0xE4` object list and returns the first object whose `WhatAmI()` is `6` (BuildingClass).
  - Evidence: decompile of `Look_up_building_in_cell @ 0x0047C520`.
  - **Active in YR:** Yes, directly called by the Mission_Deploy_Building sites above.

## INI Relevance

- **Verified binary finding:** stock YR reaches this path for normal miners because `[CMIN]` and `[HARV]` are harvesters with `Dock=NAREFN,GAREFN`, and `[GAREFN]`/`[NAREFN]` have `DockUnload=yes` and `Refinery=yes`.
  - Evidence: `ini/rulesmd.ini:7361`, `7364`, `8225`, `8228`, `11726`, `11727`, `12519`, `12520`.
  - **Active in YR:** Yes.

- **Verified binary finding:** the value `(-1,0)` is not sourced from stock refinery art keys. Stock `[GAREFN]` has `QueueingCell=4,1` and no active `DockingOffset0`; stock `[NAREFN]` has `QueueingCell=4,1` and a commented `;DockingOffset0=256,0,0`.
  - Evidence: `ini/artmd.ini:1716`, `1725`, `1773`.
  - **Active in YR:** Yes as data context; the binary source of `0x0089F6A0` is the hardcoded constructor, not these INI keys.

## Cross-Doc Correction

The prior report `miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md` was correct that the zero-`unit+0x2E4` stock unload FSM uses this global to rediscover the refinery. It was not correct where it called the value a "dock offset" likely baked from `[GAREFN] DockingOffset0=`. The precise correction is:

> `DAT_0089F6A0/2` is the west neighbor cell offset `(-1,0)` from the global 8-neighbor direction table, initialized by `Foundation_direction_table_init @ 0x0049F2F0` via the CRT constructor table. In the stock unload FSM it finds the building cell immediately west of the miner's dock cell.

## Open Questions - Final State

[RESOLVED] OQ-1 - What is the exact initialized value of `DAT_0089F6A0/2`? It is dword `0x0000FFFF`, signed shorts `dx=-1`, `dy=0`. Evidence: `0x0049F2F0`, `0x0049F38E`, constructor-table pointer `0x00812BAC`.

[RESOLVED] OQ-2 - Is the value initialized from refinery `DockingOffset%d` or `QueueingCell`? No. It is hardcoded by the global direction-table constructor. Evidence: `0x0049F2F0`; stock refinery art keys at `ini/artmd.ini:1716`, `1725`, `1773` do not supply `(-1,0)`.

[RESOLVED] OQ-3 - What does the zero-link Mission_Deploy_Building path do with the value? It adds `(-1,0)` to the miner's current cell, gets that cell, and scans for a BuildingClass object. Evidence: `0x0073E022..0x0073E05A`, `0x0073E195..0x0073E1C6`, `0x0073E2D5..0x0073E306`.

[RESOLVED] OQ-4 - Is this active in stock YR refinery DockUnload? Yes. The radio `0x15` DockUnload handoff queues Mission `0x10`; stock CMIN/HARV and GAREFN/NAREFN INI flags reach the harvester unload FSM.

[DEFERRED] OQ-5 - Can live process memory be read after game startup to prove the runtime memory cell still contains `0x0000FFFF` at the moment of a dock unload? Deferred: needs-runtime-debugger. The debugger MCP endpoint was not running (`127.0.0.1:8099` unavailable). Static evidence found the startup writer and no second direct binary writer.

## Sources

- Ghidra `decompile_function 0x0049F2F0` - `Foundation_direction_table_init`.
- Ghidra `decompile_function 0x007CD80F`, `0x007CBDAF`, `0x007CBED3` - startup/constructor-table path.
- Ghidra `decompile_function 0x0073D630` - `UnitClass::Mission_Deploy_Building`.
- Ghidra `get_assembly_context` for `0x0073E022`, `0x0073E195`, `0x0073E2D5`.
- Ghidra `decompile_function 0x0047C520` - `Look_up_building_in_cell`.
- Local read-only PE scan of `gamemd.exe` for direct references to `0x0089F6A0`/`0x0089F6A2`.
- `ini/rulesmd.ini:7361`, `7364`, `8225`, `8228`, `11726`, `11727`, `12519`, `12520`.
- `ini/artmd.ini:1716`, `1725`, `1773`.
