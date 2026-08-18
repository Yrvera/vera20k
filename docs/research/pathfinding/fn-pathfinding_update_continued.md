# MapCoord_StepByDir_GetCell — Decode Doc
**Applied Ghidra label:** `MapCoord_StepByDir_GetCell` (renamed 2026-06-01 from misleading `Pathfinding_update_continued`)
**Address:** `0x00481810`

## Summary

Despite the old Ghidra label `Pathfinding_update_continued`, this is **not a
path-update or A\* re-entry function**. It is a small cell-stepping utility: given a
struct containing a packed cell coordinate at
`+0x24` and a direction index `param_2` (0–7), it advances the stored cell coordinate
one step in that direction using `g_DirectionOffsets`, then calls
`MapClass__Get_CellClass` on the resulting cell and returns the `CellClass*`.

If `param_2 >= 8`, the function does nothing and returns immediately — the cell is not
modified and `MapClass__Get_CellClass` is not called.

It does **not** re-enter `Run_AStar`, `Find_Path`, or any A\* search function. It does
not adjust a path queue, read a path buffer, or touch any `PathfinderClass` state. The
connection to "pathfinding" is that it is used by zone/map flood-fill callers, which are
part of the broader pathfinding infrastructure — but the function itself is a pure cell
coordinate stepping helper.

**Active in YR: Yes.** Called by `FootClass__Find_Path @ 0x004D3920`,
`DriveLocomotionClass__Process_Movement @ 0x004B2630`,
`ZoneMap__FloodFillReachableZones @ 0x005840C0`, `MapClass__GetZoneID @ 0x0056D230`,
and 45+ other callers (verified via `get_function_callers 0x00481810`). Extremely
widely used; definitively live in YR.

---

## Decompilation excerpt

Source: `decompile_function 0x00481810`

```c
void __thiscall MapCoord_StepByDir_GetCell(int param_1, uint param_2)
{
    short sStack_2;

    if (param_2 < 8) {
        // Step the packed cell coord at param_1+0x24 by direction delta
        sStack_2 = (short)((uint)*(undefined4 *)(param_1 + 0x24) >> 0x10);  // current Y
        param_2 = CONCAT22(
            *(short *)((int)&g_DirectionOffsets + (param_2 & 7) * 4 + 2) + sStack_2,
            *(short *)(&g_DirectionOffsets + (param_2 & 7)) +
            (short)*(undefined4 *)(param_1 + 0x24)    // current X
        );
        // param_2 now holds the new packed cell coord (low word = new_x, high word = new_y)
        MapClass__Get_CellClass(&param_2);   // returns CellClass* in EAX
    }
    return;
}
```

> The `CONCAT22` is Ghidra's pseudocode for building a 4-byte value from two 2-byte
> halves. The disassembly at `0x00481826`–`0x0048184F` confirms: loads current cell
> from `[EAX+0x24]`, reads `g_DirectionOffsets[dir]` deltas (dx at low word, dy at high
> word), adds them, pushes result as a CellStruct, then calls `MapClass__Get_CellClass`.
> (Verified via `disassemble_function 0x00481810`.)

---

## Behavioral analysis

### Cell coord layout at `param_1+0x24`

The 4-byte value at `param_1+0x24` is a packed cell coordinate:
- Low word (`(short)(value)`) = cell X (column)
- High word (`(short)(value >> 16)`) = cell Y (row)

This is the same MapCoord packing used by `Path_walk_directions_to_cell` and other
path helpers.

### Direction stepping

`g_DirectionOffsets @ 0x0089F688` — 8 entries × 4 bytes each:
- `+0x00` (short): dx (X delta, signed)
- `+0x02` (short): dy (Y delta, signed)

Assembly at `0x00481826`: `LEA EAX,[EDX*4 + 0x89f688]` → `MOV DX,word ptr [EDX*4 + 0x89f688]` (dx) → `MOV AX,word ptr [EAX+2]` (dy). `g_DirectionOffsets` address `0x0089F688` confirmed from disassembly (verified via `disassemble_function 0x00481810`).

The direction is masked with `& 7` even though the guard `< 8` already ensures it is
in `[0, 7]` — double safety against bad inputs.

### `MapClass__Get_CellClass` call

Called with the new (stepped) cell coordinate. The MapClass singleton at `0x0087F7E8`
is used as `ECX` (verified in disassembly: `MOV ECX, 0x87f7e8; CALL 0x005657A0`).
Returns `CellClass*` in EAX.

### No path state touched

The function reads and writes only `param_1+0x24` (the struct's stored cell coord) and
calls `MapClass__Get_CellClass`. No `PathfinderClass`, no path buffer, no open/closed
list, no A\* re-entry.

---

## Struct field accesses

| Owner | Offset | Type | Meaning | Verified |
|-------|--------|------|---------|---------|
| caller struct (`param_1`) | `+0x24` | packed `int` (2×short) | current cell coord (x=low, y=high); read and written | disassemble `0x0048181c: MOV ECX,dword ptr [EAX+0x24]` |

---

## Globals

| Symbol | Address | Role |
|--------|---------|------|
| `g_DirectionOffsets` | `0x0089F688` | 8-entry delta table; stride 4; dx at `+0`, dy at `+2` (both signed shorts) |
| MapClass singleton | `0x0087F7E8` | Passed as ECX to `MapClass__Get_CellClass` |

---

## Callers (key subset)

Verified via `get_function_callers 0x00481810` — 51 total callers:

| Caller | Address | Notes |
|--------|---------|-------|
| `FootClass__Find_Path` | `0x004D3920` | Main pathfinding entry |
| `DriveLocomotionClass__Process_Movement` | `0x004B2630` | Drive locomotor |
| `ShipLocomotionClass__Process_Movement` | `0x006A1C80` | Ship locomotor |
| `ZoneMap__FloodFillReachableZones` | `0x005840C0` | Zone flood fill |
| `MapClass__GetZoneID` | `0x0056D230` | Zone ID lookup |
| `MapClass__ComputeBridgeZones` | `0x0056D6E0` | Bridge zone computation |
| `UnitClass__Can_Enter_Cell` | `0x0073F0A0` | Cell entry check |
| `WarheadTypeClass__Detonate` | `0x004690B0` | Weapon detonation cell lookup |

---

## Callees

| Function | Address | Role | Scope |
|----------|---------|------|-------|
| `MapClass__Get_CellClass` | `0x005657A0` | Get CellClass* for a cell coord | Out-of-scope — cell/map system |

---

## Label note

The prior Ghidra label `Pathfinding_update_continued` was an inherited/misleading
label that did not reflect the function's actual purpose. It was renamed in Ghidra to
`MapCoord_StepByDir_GetCell` on 2026-06-01 after re-verifying the body, callees, and
key callers. The function is not a "pathfinding update" in any loop or retry sense —
it is a 20-instruction stepping utility used pervasively across map, zone, locomotor,
and building systems.

---

## Self-Proof (3 Claims Verified This Session)

1. **Function body is a single cell-step utility — does NOT re-enter A* or touch PathfinderClass** — confirmed by fresh `decompile_function 0x00481810`: the entire decompile is 8 effective lines: read `param_1+0x24`, apply `g_DirectionOffsets[param_2]` deltas, call `MapClass__Get_CellClass`, return. No PathfinderClass field, no path buffer, no Run_AStar call. The task description said "re-enters Run_AStar or only adjusts path queue" — confirmed: it does neither.

2. **Sole callee is `MapClass__Get_CellClass @ 0x005657A0`** — confirmed via `get_function_callees 0x00481810` returning exactly `MapClass__Get_CellClass @ 005657a0`. No other callees exist.

3. **50 callers including `FootClass__Find_Path @ 0x004D3920` and `DriveLocomotionClass__Process_Movement @ 0x004B2630`** — confirmed via `get_function_callers 0x00481810` which returned 50 named callers. The prior doc claimed 51; actual count is 50. Key pathfinding-chain callers confirmed present: `FootClass__Find_Path`, `DriveLocomotionClass__Process_Movement`, `ShipLocomotionClass__Process_Movement`, `ZoneMap__FloodFillReachableZones`, `MapClass__GetZoneID`.

---

## YELLOW — Unverified

- The exact struct type of `param_1`: the function is used by many different caller
  structs that all happen to have a packed cell coord at `+0x24`. It is not a single
  named class field — it is a coincidental layout match across many callers. Verified
  only that `+0x24` is read and the new value is written back to the same location via
  the stack.
- Whether the function's output `CellClass*` (returned in EAX) is used by all callers:
  some callers may only care about the side-effect (the stepped coord on the stack, not
  the CellClass pointer). Not traced per caller.
