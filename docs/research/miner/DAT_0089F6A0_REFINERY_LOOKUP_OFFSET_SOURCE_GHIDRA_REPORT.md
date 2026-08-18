# DAT_0089F6A0 Refinery Lookup Offset Source - Ghidra Research Report

**Address(es):** `0x0049F2F0` initializer, `0x007CBDAF` startup initializer dispatch, `0x007CBED3` initializer iterator, `0x0073D630` use site  
**Investigation Mode:** exhaustive-slice  
**Target question:** What exact runtime value initializes `DAT_0089F6A0` / `DAT_0089F6A2`, and what is the source of that value as used by `UnitClass::Mission_Deploy_Building` adjacent refinery lookup during stock CMIN/HARV zero-link unload?  
**Claimed Scope:** Only the `DAT_0089F6A0` adjacent-cell offset value, its write source, its startup liveness, and the narrow Mission_Deploy_Building use sites that consume it.  
**Non-goals:** Full unload FSM rediscovery, reciprocal `+0x2E4` writer inventory, ore-credit math, dock queue rules, pathfinding after unload, Rust implementation changes, or broad correction of older miner docs.  
**Confidence:** High.  
**Active in YR:** Yes. The initializer is called before `WinMain`, and the consuming Mission_Deploy_Building path is the stock CMIN/HARV zero-link unload path already verified by the parent reports.

## Evidence Needed To Mark COMPLETE

- Exact dword/short values written to `0x0089F6A0` and `0x0089F6A2`: complete, from decompile plus assembly at `0x0049F2F0..0x0049F39B`.
- Source/initialization mechanism: complete, from `entry -> FUN_007CBDAF -> FUN_007CBED3` and static initializer table entry `0x00812BAC -> 0x0049F2F0`.
- Proof there is not another known write in the static xref set: complete, `get_bulk_xrefs` reports one write to `0x0089F6A0`, at `0x0049F38E`, and no direct write to `0x0089F6A2`.
- Proof the Mission_Deploy_Building lookup reads the initialized shorts: complete, assembly reads at `0x0073E022/0x0073E030`, `0x0073E195/0x0073E19C`, and `0x0073E2D5/0x0073E2DC`.

## Stop Conditions

- Stop after the exact initializer/source/value/liveness chain is proven.
- Do not inspect the whole unload FSM except the already-known lookup read sites.
- Do not mutate Ghidra state, rename symbols, create functions, or patch Rust/docs other than this report.
- Do not treat debugger absence as blocking if static startup evidence proves the runtime write before gameplay.

## 1. Overview

`DAT_0089F6A0` is the low 16-bit component of a two-short cell offset packed into one dword by a startup initializer. At runtime after initialization, the packed dword at `0x0089F6A0` is `0x0000FFFF`, meaning:

| Address | Interpreted as | Runtime value | Decimal meaning | Active in YR |
|---|---:|---:|---:|---|
| `0x0089F6A0` | signed `short` X | `0xFFFF` | `-1` | Yes, consumed by stock harvester unload lookup |
| `0x0089F6A2` | signed `short` Y | `0x0000` | `0` | Yes, consumed by stock harvester unload lookup |
| `0x0089F6A0` | packed dword | `0x0000FFFF` | `(x=-1, y=0)` | Yes |

This is not read from INI and not derived from the refinery's art docking offset. It is one entry in a hardcoded direction/adjacent-cell global table initialized before `WinMain`.

## 2. Initialization Source And Exact Value

### 2.1 Static image value is not the runtime value

`read_memory 0x0089F6A0 length 16` returns sixteen zero bytes from the program image. That is the pre-initialized image/BSS state, not the value used during gameplay. The runtime value is written by the startup initializer below before game code reaches `WinMain`.

Active in YR: Yes. This distinction matters because the stock unload lookup runs after process startup initialization, not against the zeroed image.

### 2.2 The only known write sets the packed dword to `0x0000FFFF`

Ghidra xrefs to `0x0089F6A0` show one WRITE:

- `0x0049F38E`: `MOV dword ptr [0x0089f6a0],EDX`

The value in `EDX` is built immediately before the write:

```asm
0049f2f1: XOR EDX,EDX                 ; EDX = 0, so DX = 0x0000
0049f2f3: OR  ECX,0xffffffff          ; ECX = -1, so CX = 0xffff
...
0049f367: MOV word ptr [ESP + 0x4],CX ; low word = 0xffff
0049f36c: MOV word ptr [ESP + 0x6],DX ; high word = 0x0000
0049f371: MOV EDX,dword ptr [ESP + 0x4] ; EDX = 0x0000ffff
0049f38e: MOV dword ptr [0x0089f6a0],EDX
```

The decompiler for `0x0049F2F0` agrees:

```text
Foundation_direction_table_init:
  g_refinery_unload_adjacent_lookup_dx = 0xffff;
```

Interpreting the packed dword as two little-endian signed shorts gives X=`-1`, Y=`0`. `DAT_0089F6A2` has no separate direct write in the xref set because it is the high short of the dword write at `0x0049F38E`.

Active in YR: Yes. There is no rule flag or TS-only gate around this initializer.

### 2.3 The initializer is called before `WinMain`

The executable entry point `0x007CD80F` calls `FUN_007CBDAF` at `0x007CD8B4`, before the later call to `WinMain`.

`FUN_007CBDAF @ 0x007CBDAF`:

```text
if (PTR_FUN_0087BEB8 != 0) call it;
FUN_007CBED3(&DAT_00815DA8, &DAT_00815DBC);
FUN_007CBED3(&DAT_00812000, &DAT_00815DA4);
```

Assembly confirms the second initializer range:

```asm
007cbdc9: PUSH 0x815da4
007cbdce: PUSH 0x812000
007cbdd3: CALL 0x007cbed3
```

`FUN_007CBED3 @ 0x007CBED3` is the iterator:

```text
for (; param_1 < param_2; param_1++) {
  if (*param_1 != 0) (*(code *)*param_1)();
}
```

The static initializer table range includes `0x00812BAC`; `read_memory 0x00812B80 length 96` shows the dword at `0x00812BAC` is `0x0049F2F0`. `get_xrefs_to 0x0049F2F0` also reports `From 00812bac [DATA]`.

Therefore startup calls `0x0049F2F0` before `WinMain`, and that call writes `0x0000FFFF` to `0x0089F6A0`.

Active in YR: Yes. This is process startup initialization in `gamemd.exe`, not optional gameplay logic.

## 3. Mission_Deploy_Building Use Sites

`UnitClass::Mission_Deploy_Building @ 0x0073D630` consumes the two shorts as signed cell deltas added to the miner's current cell, then calls `MapClass::Get_CellClass` and `Look_up_building_in_cell`.

| Use site | Reads | Effective lookup cell | State/use | Active in YR |
|---|---|---|---|---|
| `0x0073E022`, `0x0073E030` | `word [0x0089F6A0]`, `word [0x0089F6A2]` | `(current.x - 1, current.y + 0)` | first unload init/refinery anim slot | Yes |
| `0x0073E195`, `0x0073E19C` | same | `(current.x - 1, current.y + 0)` | state 4 wait/close branch | Yes |
| `0x0073E2D5`, `0x0073E2DC` | same | `(current.x - 1, current.y + 0)` | state 3 deposit loop | Yes |

Representative assembly:

```asm
0073e2ce: MOV CX,word ptr [EAX]        ; current x
0073e2d1: MOV DX,word ptr [EAX + 0x2]  ; current y
0073e2d5: ADD CX,word ptr [0x0089f6a0] ; add -1
0073e2dc: ADD DX,word ptr [0x0089f6a2] ; add 0
0073e2ff: CALL 0x005657a0              ; MapClass::Get_CellClass
```

`Look_up_building_in_cell @ 0x0047C520` then scans `CellClass+0xE4` and returns the first object whose `WhatAmI()` returns `6`.

Active in YR: Yes. The parent investigation already verified this is reached by stock CMIN/HARV zero-link unload; this report only pins the offset source/value.

## 4. INI Keys

No INI key initializes or overrides `DAT_0089F6A0` in this slice. The runtime value comes from the hardcoded startup initializer at `0x0049F2F0`.

| Data source | Finding | Evidence | Active in YR |
|---|---|---|---|
| INI | No relevant reader found/needed for this global; value is hardcoded | only WRITE xref is `0x0049F38E`; startup chain reaches it | Yes |
| Static data table | initializer function pointer at `0x00812BAC` | table entry points to `0x0049F2F0` | Yes |
| Runtime debugger | unavailable | debugger server was not running; static startup proof is sufficient | N/A |

## 5. Integration Points

- `entry @ 0x007CD80F` calls `FUN_007CBDAF` before `WinMain`.
- `FUN_007CBDAF @ 0x007CBDAF` calls the initializer iterator over `0x00812000..0x00815DA4`.
- `FUN_007CBED3 @ 0x007CBED3` calls each non-null function pointer in that range.
- Table entry `0x00812BAC` points to `Foundation_direction_table_init @ 0x0049F2F0`.
- `Foundation_direction_table_init` writes `0x0000FFFF` to `0x0089F6A0`.
- `UnitClass::Mission_Deploy_Building @ 0x0073D630` later reads `0x0089F6A0` and `0x0089F6A2` in the zero-link harvester unload lookup.

## 6. Current Rust Implementation Status

No Rust changes were made.

Current Rust appears to keep the refinery identity explicitly rather than rediscovering the building by `(miner_cell.x - 1, miner_cell.y)`:

| Rust surface | Observed shape | Current Rust delta |
|---|---|---|
| `src/sim/miner/miner_dock_sequence.rs::phase_unloading` | unload path uses `reserved_refinery` passed into the phase | mismatch/unchecked for gamemd-style adjacent-cell rediscovery |
| `src/sim/miner/mod.rs::Miner::reserved_refinery` | stores the refinery ID across dock phases | implementation convenience; not a gamemd `DAT_0089F6A0` equivalent |
| `src/sim/miner/miner_dock.rs::RefineryDockContacts::on_pad` | tracks physical on-pad contact separately from reciprocal `+0x2E4` | can coexist, but must not replace the adjacent lookup where parity requires it |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `DAT_0089F6A0` exact runtime value | verified | write build-up and `MOV [0x0089F6A0],EDX` at `0x0049F367..0x0049F38E` | none |
| `DAT_0089F6A2` exact runtime value | verified | high word from same packed dword write at `0x0049F36C..0x0049F38E` | none |
| initializer function body | verified | decompile and assembly for `0x0049F2F0..0x0049F39B` | none |
| initializer table source | verified | `0x00812BAC` contains `0x0049F2F0`; xref `From 00812bac [DATA]` | none |
| startup liveness | verified | `entry @ 0x007CD80F` calls `FUN_007CBDAF`; `0x007CBDAF` calls iterator over `0x00812000..0x00815DA4`; iterator calls non-null pointers | none |
| Mission_Deploy_Building use sites | verified | assembly reads at `0x0073E022/30`, `0x0073E195/9C`, `0x0073E2D5/DC` | none |
| full unload FSM | not-touched | out of scope per parent target | use existing parent reports |
| runtime debugger memory snapshot | deferred | debugger server not running | not needed for COMPLETE because static startup chain proves runtime write |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-5A - What exact value is in DAT_0089F6A0 after initialization? -> Packed dword `0x0000FFFF`; low signed short X is `-1`.` (evidence: `0x0049F367..0x0049F38E`)
- `[RESOLVED] OQ-5B - What exact value is in DAT_0089F6A2 after initialization? -> High signed short Y is `0`.` (evidence: `0x0049F36C..0x0049F38E`)
- `[RESOLVED] OQ-5C - What initializes the global? -> `Foundation_direction_table_init @ 0x0049F2F0` writes the packed dword.` (evidence: decompile `0x0049F2F0`; xref write `0x0049F38E`)
- `[RESOLVED] OQ-5D - Is the initializer live in stock YR startup? -> Yes; entry calls `FUN_007CBDAF`, which iterates `0x00812000..0x00815DA4`, including table entry `0x00812BAC -> 0x0049F2F0`.` (evidence: `0x007CD8B4`, `0x007CBDC9..0x007CBDD3`, `0x007CBED3`, `0x00812BAC`)
- `[RESOLVED] OQ-5E - Is the value INI/art derived? -> No evidence of an INI/art reader; only known write is the hardcoded initializer.` (evidence: bulk xrefs to `0x0089F6A0/0x0089F6A2`)
- `[RESOLVED] OQ-5F - What cell does Mission_Deploy_Building look up with this value? -> The cell one tile west of the miner's current cell, `(current.x - 1, current.y)`.` (evidence: `0x0073E022/30`, `0x0073E195/9C`, `0x0073E2D5/DC`)
- `[RESOLVED] OQ-5G - Is this TS legacy or live YR behavior? -> Live YR behavior; startup initializer is unconditional and lookup use is on the stock zero-link CMIN/HARV unload path.` (evidence: `entry @ 0x007CD80F`; prior report `MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-5H - Can a live debugger snapshot show the post-init bytes directly?` (category: `needs-runtime-debugger`; reason: debugger server was not running; next-step-if-pursued: attach debugger and read ghidra address `0x0089F6A0` after process startup)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock unload refinery rediscovery uses `(miner_cell.x - 1, miner_cell.y + 0)` from the initialized `DAT_0089F6A0/A2` pair | init `0x0049F2F0..0x0049F38E`; uses `0x0073E022/30`, `0x0073E195/9C`, `0x0073E2D5/DC` | mismatch/unchecked: Rust unload uses `reserved_refinery` identity | `src/sim/miner/miner_dock_sequence.rs::phase_unloading` and state-4 handoff logic | Where parity depends on gamemd lookup, resolve the refinery from the cell immediately west of the miner's current accepted pad cell, not from a reciprocal link | `test_unload_refinery_lookup_uses_gamemd_adjacent_offset` | Do not use art `DockingOffset0`, building `+0x2E4`, or the original reserved target as a substitute for this lookup |
| The global value is a hardcoded startup table constant, not INI/art data | only write xref `0x0049F38E`; startup table `0x00812BAC -> 0x0049F2F0` | none needed if implemented as a named constant; unchecked if rules loader tries to source it | rules/assets integration should not own this value unless a later binary investigation proves a data source | deterministic test with GAREFN/NAREFN accepted dock cell verifies lookup at x-1 | Do not expose this as a modifiable rules key or derive it from foundation size |
| The lookup returns the first building object in the target cell's object list | `Look_up_building_in_cell @ 0x0047C520` scans `CellClass+0xE4`, `WhatAmI()==6` | unchecked: Rust direct ID lookup bypasses cell object ordering | world/entity cell occupancy and miner unload lookup surface | Reproduce building lookup semantics if multiple objects or stale reservation disagree | `test_unload_refinery_lookup_prefers_building_in_west_cell_over_reserved_refinery` | Do not assume `reserved_refinery` is authoritative once the zero-link unload FSM is running |

## Negative Facts / Do Not Do

- Do not leave OQ-5 as "exact value unknown": the runtime post-init pair is verified as `(-1, 0)`.
- Do not model `DAT_0089F6A0` as `0` because the image bytes are zero before startup initialization.
- Do not derive the value from `DockingOffset0`, refinery foundation, the accepted anchor `building NW+(3,1)`, or INI keys.
- Do not use reciprocal `unit/building +0x2E4` to find the refinery in the stock zero-link unload loop.
- Do not route normal stock zero-link post-unload exit through `ReleaseDockedHarvester` / `Force_Track(0x47)` based on this global.

## Stale Docs / Follow-up Docs

Replacement wording for any stale OQ-5 or "source unknown" text:

> `DAT_0089F6A0` is initialized before `WinMain` by `Foundation_direction_table_init @ 0x0049F2F0`, reached through the startup initializer table entry `0x00812BAC`. Its runtime packed dword is `0x0000FFFF`, so Mission_Deploy_Building reads X=`-1` from `0x0089F6A0` and Y=`0` from `0x0089F6A2`. The value is hardcoded startup data, not INI/art-derived and not a `+0x2E4` dock link.

## Remaining Uncertainty

None for the target slice. A live debugger byte read was unavailable, but static startup evidence proves the write before `WinMain` and there is no competing write xref in the analyzed program.

## Sources

- Ghidra `get_bulk_xrefs 0x0089F6A0,0x0089F6A2`
- Ghidra `read_memory 0x0089F6A0`
- Ghidra `decompile_function 0x0049F2F0`
- Ghidra `get_assembly_context 0x0049F2F0..0x0049F38E`
- Ghidra `read_memory 0x00812B80`
- Ghidra `get_xrefs_to 0x0049F2F0`
- Ghidra `decompile_function 0x007CD80F`
- Ghidra `decompile_function 0x007CBDAF`
- Ghidra `decompile_function 0x007CBED3`
- Ghidra `get_assembly_context 0x007CBDC9..0x007CBDD3`
- Ghidra `decompile_function 0x0073D630`
- Ghidra `get_assembly_context 0x0073E022,0x0073E030,0x0073E195,0x0073E19C,0x0073E2D5,0x0073E2DC`
- Ghidra `decompile_function 0x0047C520`
- Prior report: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_DEPLOY_BUILDING_DAT_0089F6A0_REFINERY_LOOKUP_GHIDRA_REPORT.md`
