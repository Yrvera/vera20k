# BIB_ADJACENT_CELL_DIRECTION_SOURCE - Ghidra Research Report

**Address(es):** `0x0049F2F0` (`Foundation_direction_table_init`), `0x0073F0A0` (`UnitClass::Can_Enter_Cell`), data `0x0089F690`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** runtime source/value and initialization path for the adjacent-cell direction read by the `HasBib` branch in `UnitClass::Can_Enter_Cell`; resulting edge semantics for `Bib=yes` building blockers.
**Non-Scope:** `NumberImpassableRows`, `AddOccupy`/`RemoveOccupy` footprint production, refinery dock radio FSM, and full `UnitClass::Can_Enter_Cell` return-code tree.
**Confidence:** High for static binary initialization and consumer semantics; live-process watchpoint not available.
**Active in YR:** Yes for the initializer and `UnitClass::Can_Enter_Cell`; HasBib edge relaxation is Conditional on a checked occupant being a non-laser-fence building whose `BuildingTypeClass+0x1570` is nonzero.

## 1. Overview

The adjacent-cell direction previously referred to as `DAT_0089F690` is the east member of the global 8-neighbor cell-offset table. It is initialized before `WinMain` by a hardcoded static constructor, not by theater, map, INI, refinery dock, or a runtime parameter.

The effective initialized value at `0x0089F690` is dword `0x00000001`, interpreted by the HasBib branch as signed shorts `(dx=+1, dy=0)`. In `UnitClass::Can_Enter_Cell`, a `Bib=yes` building stops blocking the current cell only when the same building is not found one cell to the east.

## 2. Key Data / Offsets

| Item | Offset / address | Verified meaning | Active in YR |
|---|---:|---|---|
| East direction table member | `0x0089F690` / high word `0x0089F692` | Signed cell offset `(dx=+1, dy=0)` | Yes; constructor table initializes it before `WinMain` |
| HasBib flag | `BuildingTypeClass+0x1570` | Parsed `Bib=yes`; gates the UnitClass bib relaxation | Conditional; stock YR has `Bib=yes` structures such as `[GAREFN]` and `[NAREFN]` |
| Laser-fence exclusion | `BuildingTypeClass+0x16C0` | If nonzero, the HasBib offset probe is skipped and the building path follows laser-fence handling | Conditional; branch guard is present in the live UnitClass path |
| Current cell coordinates | `CellClass+0x24/+0x26` | The branch adds `0x0089F690/2` to these shorts before `MapClass::Get_CellClass` | Yes in `UnitClass::Can_Enter_Cell` |

## 3. Initialization Path

- **Verified binary finding:** the real initializer is `Foundation_direction_table_init @ 0x0049F2F0`, not a parameterized function starting at `0x0049F300`.
  - Evidence: Ghidra decompile of `0x0049F2F0`; assembly at `0x0049F2F0` begins the function, and `0x0049F300` is an internal `MOV EAX,[ESP]`.
  - **Active in YR:** Yes. `entry @ 0x007CD80F` calls `FUN_007CBDAF` before `WinMain`; `FUN_007CBDAF` iterates `0x00812000..0x00815DA4` through `FUN_007CBED3`; `get_function_xrefs(0x0049F2F0)` reports data xref `0x00812BAC`, which lies inside that constructor range.

- **Verified binary finding:** the constructor writes `0x0089F690 = 0x00000001`.
  - Evidence: decompile of `0x0049F2F0`; assembly context at `0x0049F336` writes `dword ptr [0x0089F690], ESI` after `EAX=1`, high word set from zeroed `EDX`, and low word set from `AX`.
  - **Active in YR:** Yes, through the constructor-table path above.

- **Verified binary finding:** the direction table around it is hardcoded as the 8 adjacent offsets: `0x0089F688=(0,-1)`, `0x0089F68C=(1,-1)`, `0x0089F690=(1,0)`, `0x0089F694=(1,1)`, `0x0089F698=(0,1)`, `0x0089F69C=(-1,1)`, `0x0089F6A0=(-1,0)`, `0x0089F6A4=(-1,-1)`.
  - Evidence: Ghidra decompile of `0x0049F2F0`; prior verified report `DAT_0089F6A0_RUNTIME_SOURCE_AND_VALUE_GHIDRA_REPORT.md` independently recorded the same constructor-table path and table values.
  - **Active in YR:** Yes. The constructor runs before normal game startup and multiple live helpers read this table.

## 4. Consumer Semantics In UnitClass::Can_Enter_Cell

- **Verified binary finding:** `UnitClass::Can_Enter_Cell @ 0x0073F0A0` reads `BuildingTypeClass+0x1570`; when nonzero and `+0x16C0` is zero, it adds `word [0x0089F690]` to the current cell X and `word [0x0089F692]` to current cell Y, then calls `MapClass::Get_CellClass` and `Look_up_building_in_cell`.
  - Evidence: decompile of `0x0073F0A0`; assembly context at `0x0073F7D3` (`HasBib` byte load), `0x0073F7E5` (`add cx, word ptr [0x0089F690]`), `0x0073F7EC` (`add dx, word ptr [0x0089F692]`), `0x0073F80F` (`MapClass::Get_CellClass`).
  - **Active in YR:** Conditional. This is inside the live UnitClass A* passability entry, but only executes for non-laser-fence BuildingClass occupants with `HasBib != 0`.

- **Verified binary finding:** if `Look_up_building_in_cell(current + (1,0))` returns a different building or null, the code jumps to the next occupant and does not let this building block the checked cell.
  - Evidence: decompile of `0x0073F0A0` around the HasBib branch; `Look_up_building_in_cell @ 0x0047C520` returns the first `WhatAmI()==6` object in `CellClass+0xE4`.
  - **Active in YR:** Conditional under the same HasBib branch conditions.

- **Verified binary finding:** `Look_up_building_in_cell @ 0x0047C520` is not bib-aware; it only scans a cell's ground object list and returns the first BuildingClass object.
  - Evidence: Ghidra decompile of `0x0047C520`.
  - **Active in YR:** Yes; directly called by the HasBib probe and other live systems.

## 5. Edge Semantics For Bib=yes

For a `Bib=yes` building occupying a candidate cell, the UnitClass branch asks: "is the same building also in the cell one east?" If yes, the current cell remains blocked by that building. If no, this building is skipped for this candidate cell.

Observable consequence: for UnitClass-style vehicle passability checks, HasBib relaxes the building blocker along the east edge of that building's occupied footprint, where "east edge" means cells whose `(x+1,y)` neighbor is not occupied by the same building. This applies to the actual occupant/footprint cells seen by the cell list; this report does not re-derive whether those cells came from the rectangular foundation or `AddOccupy`/`RemoveOccupy`.

This does not add bib cells to placement, ownership, or footprint production. It only changes how `UnitClass::Can_Enter_Cell` treats an already-present building occupant while evaluating a checked cell.

## 6. INI / Data Relevance

| INI path | Evidence | Effect on this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini [GAREFN] Bib=yes` | `ini/rulesmd.ini:11730` | Provides stock HasBib data for a live structure | Yes |
| `rulesmd.ini [NAREFN] Bib=yes` | `ini/rulesmd.ini:12523` | Provides stock HasBib data for a live structure | Yes |
| `artmd.ini [GAREFN] Foundation=4x3`, `AddOccupy1=-1,0`, `AddOccupy2=-1,-1`, `RemoveOccupy1=3,1` | `ini/artmd.ini:1766`, `1793..1795` | Context only; this slot did not re-investigate footprint production | Yes as stock data, non-scope for this report |
| Theater/map/init parameters | No reader found in the verified constructor path | The `0x0089F690` value is hardcoded, not theater/map-derived | No dependency found |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x0089F690` initialized value | verified | `0x0049F2F0` decompile; `0x0049F336` write | none |
| Constructor reachability before `WinMain` | verified | `0x007CD80F`, `0x007CBDAF`, `0x007CBED3`; data xref `0x00812BAC` | none |
| `0x0049F300 param_2` prior claim | verified refuted | `0x0049F300` is inside `0x0049F2F0`; no function parameter exists | none |
| `UnitClass::Can_Enter_Cell` HasBib consumer | verified | `0x0073F0A0`, `0x0073F7D3`, `0x0073F7E5`, `0x0073F7EC`, `0x0073F80F` | none |
| `Look_up_building_in_cell` helper contract | verified | `0x0047C520` decompile | none |
| Later live-memory overwrite after startup | touched-not-exhausted | debugger read failed: server not running at `127.0.0.1:8099`; direct PE immediate scan found only one decoded direct write to `0x0089F690` (`0x0049F336`) | optional runtime watchpoint if future work needs debugger-level proof |
| `NumberImpassableRows` interaction | deferred | user non-scope | separate slot/report |
| `AddOccupy`/`RemoveOccupy` footprint production | deferred | user non-scope | separate slot/report |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What is the effective offset used by the HasBib branch? `(dx=+1, dy=0)`, dword `0x00000001` at `0x0089F690`, high word `0` at `0x0089F692`. Evidence: `0x0049F2F0`, `0x0049F336`, `0x0073F7E5`, `0x0073F7EC`.

[RESOLVED] OQ-2 - Is the value sourced from theater, map, INI, or a parameter? No. The verified source is a no-argument static constructor in the CRT constructor table. Evidence: `0x0049F2F0`, `0x00812BAC`, `0x007CBDAF`, `0x007CBED3`, `0x007CD80F`.

[RESOLVED] OQ-3 - Does the older `FUN_0049f300 writes (1,param_2)` claim hold? No. `0x0049F300` is not a function entry; it is an instruction inside `0x0049F2F0`. Evidence: Ghidra decompile/function boundary and assembly context at `0x0049F2F0..0x0049F336`.

[RESOLVED] OQ-4 - What does this mean for `Bib=yes` building passability? The east-edge occupied cells of a HasBib building are skipped as blockers in `UnitClass::Can_Enter_Cell`, because the same building is not found at `(x+1,y)`. Evidence: `0x0073F0A0` HasBib branch and `0x0047C520` helper.

[DEFERRED] OQ-5 - Can a live runtime watchpoint prove no indirect post-startup overwrite? Deferred: needs-runtime-debugger. The debugger MCP endpoint was unavailable. Static evidence proves the active startup value and found no decoded direct write other than `0x0049F336`.

## Sources

- Ghidra `decompile_function 0049F2F0` - `Foundation_direction_table_init`.
- Ghidra `get_function_xrefs 0049F2F0` - data xref `0x00812BAC`.
- Ghidra `decompile_function 007CD80F`, `007CBDAF`, `007CBED3` - startup constructor iteration before `WinMain`.
- Ghidra `decompile_function 0073F0A0` and assembly context around `0x0073F7D3..0x0073F80F` - HasBib consumer branch.
- Ghidra `decompile_function 0047C520` - cell building lookup helper.
- Local read-only PE scan of `gamemd.exe` for direct references to `0x0089F690/0x0089F692`.
- Existing doc `docs/research/DAT_0089F6A0_RUNTIME_SOURCE_AND_VALUE_GHIDRA_REPORT.md`.
- INI context: `ini/rulesmd.ini:11730`, `ini/rulesmd.ini:12523`, `ini/artmd.ini:1766`, `ini/artmd.ini:1793..1795`.
