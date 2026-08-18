# BuildingClass GetCellLocation vtable +0x1B8 Anchor - Ghidra Research Report

**Address(es):** `0x0041BEA0` (`ObjectClass__Get_Cell_Packed`), BuildingClass vtable base `0x007E3EBC`, slot address `0x007E4074`, call sites `0x0045977E`, `0x0043CA7A`, `0x0073E01C`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Return semantics of BuildingClass/ObjectClass vtable slot `+0x1B8` when used as "GetCellLocation" in refinery/post-dump-adjacent contexts.
**Non-Scope:** Full refinery unload state machine, full `Find_Nearby_Passable_Cell` search behavior, rendering of exit movement, non-building overrides of nearby virtual slots.
**Confidence:** High
**Active in YR:** Yes for standard YR buildings; Yes for standard GAREFN/NAREFN refinery docking contexts; Conditional for the reciprocal-link `ReleaseDockedHarvester` post-dump branch, which requires `building+0x2E4` to be non-null.

## 1. Overview

For a building, vtable slot `+0x1B8` points to `ObjectClass__Get_Cell_Packed @ 0x0041BEA0`. It returns the building object's own location cell, computed only from `ObjectClass +0x9C/+0xA0` world X/Y. For standard buildings this is the foundation/NW origin cell, not the foundation center, not the refinery dock pad, and not the art `QueueingCell`.

The return is a packed 32-bit value written through the caller-provided out pointer: low 16 bits are cell X, high 16 bits are cell Y. Callers then add their own context-specific offsets such as refinery accepted dock `(x+3,y+1)` or reciprocal-link exit search `(x-1,y+1)`.

## 2. Class Layout / Key Offsets

| Item | Offset / address | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| BuildingClass vtable base | `0x007E3EBC` | Building virtual table used by standard buildings | `read_memory 0x007E3EBC` | Yes |
| vtable slot `+0x1B8` | `0x007E4074` -> bytes `a0 be 41 00` | Resolves to `0x0041BEA0` | `read_memory 0x007E4074` | Yes |
| Object location X | `this+0x9C` | lepton X used by slot `+0x1B8` | `0x0041BEA1` | Yes |
| Object location Y | `this+0xA0` | lepton Y used by slot `+0x1B8` | `0x0041BEB8` | Yes |
| Object location Z | `this+0xA4` | not read by slot `+0x1B8` | absence in `0x0041BEA0`; read by `GetCoords @ 0x00447AC0` | Yes |
| Building center virtual | vtable `+0x48` -> `0x00447AC0` | returns foundation-center leptons using width/height | decompile `0x00447AC0` | Yes |

## 3. Core Logic

`ObjectClass__Get_Cell_Packed @ 0x0041BEA0`:

```text
x = this.Location_X
y = this.Location_Y
out.low16  = (x + ((x >> 31) & 0xFF)) >> 8
out.high16 = (y + ((y >> 31) & 0xFF)) >> 8
return out_pointer
```

The `CDQ; AND EDX,0xFF; ADD EAX,EDX; SAR EAX,8` sequence is a sign-correct integer division by 256. Positive map coordinates are ordinary `x >> 8` and `y >> 8`. No foundation width, height, dock offset, queueing cell, or Z value participates.

**Active in YR: Yes.** The BuildingClass vtable slot is live and used by standard building code paths. Evidence: `read_memory 0x007E4074` points to `0x0041BEA0`; call sites below dispatch via `this->vtable+0x1B8`.

Contrast with `BuildingClass__GetCoords @ 0x00447AC0`, slot `+0x48`:

```text
center.x = Location_X + foundation_width  * 0x80 - 0x80
center.y = Location_Y + foundation_height * 0x80 - 0x80
center.z = Location_Z
```

This proves `+0x1B8` is not the center-cell/center-coordinate method. For a 4x3 refinery at NW `(10,10)`, `+0x1B8` returns `(10,10)`, while `+0x48` returns center leptons `(10*256+384, 10*256+256, z)`.

## 4. INI Keys

| File | Section | Key | Stock value | Relevance | Active in YR |
|---|---|---|---|---|---|
| `ini/rulesmd.ini:11722-11729` | `[GAREFN]` | `DockUnload`, `Refinery`, `NumberOfDocks` | `yes`, `yes`, `1` | selects standard refinery docking/unload contexts using this anchor | Yes |
| `ini/rulesmd.ini:12515-12521` | `[NAREFN]` | `DockUnload`, `Refinery`, `NumberOfDocks` | `yes`, `yes`, `1` | same for Soviet refinery | Yes |
| `ini/rulesmd.ini:7361,7364` | `[CMIN]` | `Dock`, `Harvester` | `NAREFN,GAREFN`, `yes` | standard chrono miner can reach refinery dock paths | Yes |
| `ini/rulesmd.ini:8225,8228` | `[HARV]` | `Dock`, `Harvester` | `NAREFN,GAREFN`, `yes` | standard war miner can reach refinery dock paths | Yes |
| `ini/artmd.ini:1709,1766` | `[NAREFN]`, `[GAREFN]` | `Foundation` | `4x3` | affects `GetCoords`, not `+0x1B8` | Yes |
| `ini/artmd.ini:1716,1773` | `[NAREFN]`, `[GAREFN]` | `QueueingCell` | `4,1` | not read by `+0x1B8`; not the returned anchor | Yes as data; No as input to this slot |
| `ini/artmd.ini:1760,1795` | `[NAREFN]`, `[GAREFN]` | `RemoveOccupy` | `3,1` pad opened | explains why accepted dock cell can be passable; not the slot return | Yes |

## 5. Integration Points

### Reciprocal-link post-dump/exit context

`BuildingClass__ReleaseDockedHarvester @ 0x004595C0` dispatches slot `+0x1B8` at `0x0045977E`. Assembly:

```text
0045977E  CALL dword ptr [EAX + 0x1b8]
00459784  MOV DX,word ptr [EAX]
00459787  MOV AX,word ptr [EAX + 0x2]
0045978B  DEC DX
0045978D  INC AX
```

The function therefore builds search anchor `(slot_x - 1, slot_y + 1)`. If a refinery's NW cell is `(10,10)`, this caller's anchor is `(9,11)`. The `(9,11)` is caller arithmetic; slot `+0x1B8` itself returned `(10,10)`.

**Active in YR: Conditional.** The function is in a live YR code path but its main branch requires `building+0x2E4 != 0` and the docked unit to report locomotion type `1`; current adjacent reports show stock zero-link CMIN/HARV DockUnload completion normally exits through `Mission_Deploy_Building` state 4 instead of this reciprocal-link branch.

### Accepted refinery CAN_DOCK context

`BuildingClass__Receive_Radio @ 0x0043C2D0`, case `0x0E`, dispatches slot `+0x1B8` at `0x0043CA7A`, then computes:

```text
move_cell.x = slot_x + 3
move_cell.y = slot_y + 1
payload = MapClass__Get_CellClass(move_cell)
```

For a refinery at NW `(10,10)`, accepted `MOVE_TO_CELL(0x12)` target is `(13,11)`. That is the hardcoded refinery pad offset from the NW anchor, not a different slot return.

Assembly:

```text
0043CA7A  CALL dword ptr [EAX + 0x1b8]
0043CA80  MOV DX,word ptr [EAX]
0043CA83  MOV AX,word ptr [EAX + 0x2]
0043CA87  ADD DX,0x3
0043CA8B  INC AX
0043CAA9  CALL 0x005657A0
```

**Active in YR: Yes.** GAREFN/NAREFN have `DockUnload=yes`; CMIN/HARV have `Dock=NAREFN,GAREFN` and `Harvester=yes`.

### Stock zero-link unload FSM context

`UnitClass__Mission_Deploy_Building @ 0x0073D630` calls the unit's own vtable `+0x1B8` at `0x0073E01C` and then adds the global adjacent lookup vector at `DAT_0089F6A0`/`DAT_0089F6A2` before `MapClass__Get_CellClass` and `Look_up_building_in_cell`.

**Active in YR: Yes.** This is the stock refinery unload rediscovery path. It is a unit-anchor use, not a building return, so it does not change the BuildingClass answer; it was checked only to avoid conflating the post-dump context.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| BuildingClass vtable slot `+0x1B8` binding | verified | `read_memory 0x007E4074` -> `a0be4100` | none |
| `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` return formula | verified | decompile and assembly `0x0041BEA1..0x0041BEDA` | none |
| Center-vs-origin distinction | verified | `BuildingClass__GetCoords @ 0x00447AC0` | none |
| `ReleaseDockedHarvester` slot use | verified | `0x0045977E..0x0045978D` | stock reachability is conditional, but formula is resolved |
| `Receive_Radio` accepted refinery slot use | verified | decompile `0x0043C2D0`; assembly `0x0043CA7A..0x0043CAA9` | none |
| Stock zero-link unload FSM confusion check | touched-not-exhausted | decompile `0x0073D630`, assembly `0x0073E01C..0x0073E053` | full FSM outside scope |

## 7. Open Questions - Final State

[RESOLVED] OQ-1 - What does BuildingClass vtable `+0x1B8` point to? It points to `ObjectClass__Get_Cell_Packed @ 0x0041BEA0`. Evidence: `read_memory 0x007E4074` returned `a0be4100`. Active in YR: Yes.

[RESOLVED] OQ-2 - Does the slot return NW/foundation origin, center, dock cell, or another anchor? For buildings it returns the object location cell from `+0x9C/+0xA0`, which is the foundation/NW origin cell for standard placed buildings. Evidence: `0x0041BEA0` reads only `+0x9C/+0xA0`; `0x00447AC0` separately computes the foundation center. Active in YR: Yes.

[RESOLVED] OQ-3 - Does the slot consume `QueueingCell`, `DockingOffset`, foundation dimensions, or `RemoveOccupy`? No. It reads no type fields and no INI-derived building type offsets. Evidence: `0x0041BEA0` only accesses object fields `+0x9C/+0xA0`. Active in YR: Yes.

[RESOLVED] OQ-4 - Why do refinery contexts produce `(NW+3,NW+1)` or `(NW-1,NW+1)`? Those are caller-side offsets after the slot returns NW. Evidence: `BuildingClass__Receive_Radio` assembly `0x0043CA7A..0x0043CAA9`; `ReleaseDockedHarvester` assembly `0x0045977E..0x0045978D`. Active in YR: Yes for `0x0E`; Conditional for `ReleaseDockedHarvester`.

[RESOLVED] OQ-5 - Is standard YR refinery data consistent with this interpretation? Yes. GAREFN/NAREFN are 4x3 DockUnload refineries; their `QueueingCell=4,1` and `RemoveOccupy=3,1` are separate art data, not slot inputs. Evidence: `rulesmd.ini:11722-11729`, `rulesmd.ini:12515-12521`, `artmd.ini:1709-1773`, `artmd.ini:1795`. Active in YR: Yes.

## Sources

- Ghidra `read_memory 0x007E4074`, `read_memory 0x007E3EBC`
- Ghidra `decompile_function 0x0041BEA0`
- Ghidra assembly context `0x0041BEA1..0x0041BEDA`
- Ghidra `decompile_function 0x00447AC0`
- Ghidra `decompile_function 0x004595C0`; assembly context `0x0045977E..0x0045978D`
- Ghidra `decompile_function 0x0043C2D0`; assembly context `0x0043CA7A..0x0043CAA9`
- Ghidra `decompile_function 0x0073D630`; assembly context `0x0073E01C..0x0073E053`
- `ini/rulesmd.ini`, `ini/artmd.ini`
- Prior docs used as leads and cross-checks: `miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md`, `miner/RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`
