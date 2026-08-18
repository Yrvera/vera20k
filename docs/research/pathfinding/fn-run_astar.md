# FootClass__Run_AStar — Decode Doc
**Proposed Ghidra label:** FootClass__Run_AStar (already labeled)

## Summary

Bridge function called by `FootClass::Find_Path` that sets up A\* search state and dispatches into
`AStar_pathfind_search`. It: (1) queries the unit's destination in leptons via vtable slot `+0x4c`
(`FootClass__GetDestinationCoords`), (2) converts the lepton coords to a cell-packed goal, (3) calls
`Path_walk_directions_to_cell` to convert the existing path-direction buffer into waypoints (for
context or exclusion), then (4) calls `AStar_pathfind_search` with the global `PathfinderClass`
singleton and returns the result directly.

**Active in YR: Yes.** Single YR-reachable caller: `FootClass__Find_Path @ 0x004D3920`
(verified via `get_function_callers 0x004CBBA0`). Reachable from Drive/Walk/Ship locomotors via
every unit pathfind request. No TS-legacy gate.

---

## Decompilation excerpt

Source: `decompile_function 0x004CBBA0`

```c
undefined4 __thiscall
FootClass__Run_AStar(int *param_1,       // FootClass* this
                     undefined4 param_2, // (unused in body — stack artifact)
                     undefined4 param_3, // (unused in body — stack artifact)
                     undefined4 param_4, // goal cell (packed CellIndex — int)
                     undefined4 param_5) // extra arg forwarded to AStar
{
    undefined4 uVar1;
    undefined1 local_c[8];   // CoordStruct output buffer (x, y, z)
    undefined4 uStack_4;

    // (1) Get current destination in leptons via virtual dispatch
    (**(code **)(*param_1 + 0x4c))(local_c, 0);   // → FootClass__GetDestinationCoords

    // (2) Guard: if goal packed cell == 0, bail
    if (param_4 == 0) return 0;

    // (3) Convert existing path direction buffer into waypoints
    Path_walk_directions_to_cell(param_4, param_1 + 0x178);

    // (4) Run A* and return result
    uVar1 = AStar_pathfind_search(&stack_result, start_cell, param_1, goal_cell,
                                   0xffffffff, 0xffffffff, param_5);
    return uVar1;
}
```

> Note: The Ghidra pseudocode uses `unaff_retaddr` as an artifact of the non-standard calling
> convention. The disassembly at `0x004CBBA0` (verified via `disassemble_function 0x004CBBA0`)
> clarifies: the vtable result is stored in ECX (register return), then ECX fields are used
> directly as lepton X/Y.

---

## Behavioral analysis

### Step-by-step execution

1. **GetDestinationCoords via vtable+0x4c** (asm `004cbbb2: CALL dword ptr [EAX + 0x4c]`)
   - Calls `FootClass__GetDestinationCoords @ 0x004DBDF0` (verified via
     `read_memory 0x007e22f0` → `F0BD4D00` = `0x004DBDF0`; confirmed via
     `get_function_by_address 0x004DBDF0`).
   - Result: a `CoordStruct { x_lepton: i32, y_lepton: i32, z_lepton: i32 }` written into
     `local_c[8]` buffer; pointer returned in EAX/ECX.
   - `FootClass__GetDestinationCoords` handles three cases:
     - Unit in tube (`param_1[0x1a1] >= 0`): returns tube endpoint in leptons.
     - Locomotor has valid destination: returns locomotor destination.
     - Locomotor returns null/invalid: falls back to `vtable+0x48` (GetCoords = current pos).

2. **Lepton → cell conversion** (asm `004cbbbe`–`004cbbdd`)
   - X cell: `(x_lepton + (x_lepton >> 31 & 0xFF)) >> 8`
   - Y cell: `(y_lepton + (y_lepton >> 31 & 0xFF)) >> 8`
   - Stored as two consecutive `short` fields in `[ESP+0xc]` and `[ESP+0xe]`, forming a
     packed cell coordinate (cell_x as low short, cell_y as high short).
   - This is the standard sign-correct arithmetic-shift floor (matches CLAUDE.md lepton→cell
     formula exactly).

3. **Guard: goal == 0** (asm `004cbbe6: TEST EDI,EDI; 004cbbec: JNZ`)
   - `EDI = [ESP+0x24]` = `param_4` = the goal cell packed value passed by Find_Path.
   - If zero, returns 0 immediately (no path).

4. **Path_walk_directions_to_cell** (asm `004cbc0c: CALL 0x00429780`)
   - Called as `Path_walk_directions_to_cell(param_4, &this->path_direction_buf)`
   - `&this->path_direction_buf` = `ESI+0x5e0` = `param_1[0x178]` (offset 0x5e0 bytes /
     `int*` index 0x178; verified via asm `004cbbfc: LEA EAX,[ESI+0x5e0]`).
   - Called BEFORE A\* — converts any existing path direction buffer into cell waypoints so
     they can be passed as context (start node hint) to the new search.

5. **AStar_pathfind_search call** (asm `004cbc31: CALL 0x0042C900`)
   - ECX = `0x0087e8b8` = `g_PathfinderClass_Singleton` (global, read-only, zero at static
     init; runtime-allocated; verified via `read_memory 0x0087e8b8`).
   - Arguments pushed (right-to-left):
     - `&result_struct` — stack-allocated output struct for path result
     - `start_cell` — from `Path_walk_directions_to_cell` return (current pos cell)
     - `&walk_result` — struct from Path_walk result (holds waypoint list)
     - `ESI` = FootClass* this
     - `EDI` = goal_cell (converted from destination leptons → cells above)
     - `-1` (0xFFFFFFFF) = zone exclusion 1 (no exclusion)
     - `-1` (0xFFFFFFFF) = zone exclusion 2 (no exclusion)
     - `param_5` = extra arg forwarded from Find_Path caller

6. **Return** — returns the `AStar_pathfind_search` result directly (pointer to result struct,
   or 0 on failure).

---

## Struct field accesses (frame-annotated)

| Offset | Expression | Frame | Notes |
|--------|-----------|-------|-------|
| `+0x00` (vtable) | `*param_1` → `[EAX+0x4c]` | FootClass vtable | vtable slot 0x4c = `FootClass__GetDestinationCoords` |
| `+0x5e0` | `ESI+0x5e0` = `param_1[0x178]` | FootClass instance (NW-cell frame, leptons) | path direction buffer; `int*`-indexed as `[0x178]` (×4 = 0x5e0 bytes) |

> Frame note: `param_1` in Ghidra decompile has type `int*`; offset `0x178` means byte offset
> `0x178 × 4 = 0x5e0`. Per CLAUDE.md `int*` ×4 rule.

---

## Globals / Enums / INI

| Symbol | Address | Role |
|--------|---------|------|
| `g_PathfinderClass_Singleton` | `0x0087e8b8` | PathfinderClass* global; passed as ECX (this ptr) to `AStar_pathfind_search` (verified via `disassemble_function 0x004CBBA0` — `MOV ECX,0x87e8b8`) |

No INI keys read. No enum values. No TS-gated flags.

---

## Callees

| Function | Address | Role | Out-of-scope? |
|----------|---------|------|--------------|
| `FootClass__GetDestinationCoords` (via vtable+0x4c) | `0x004DBDF0` | Returns destination as CoordStruct (leptons) | No — referenced here only |
| `Path_walk_directions_to_cell` | `0x00429780` | Converts path direction buffer → cell waypoints | In-scope (task #15) |
| `AStar_pathfind_search` | `0x0042C900` | Runs A* search on PathfinderClass singleton | In-scope (task #3) |

---

## Out-of-scope refs

- `FootClass__GetDestinationCoords @ 0x004DBDF0` — vtable callee; locomotor/unit-state system, not
  A\* pathfinding core. Cited here for identification only.

---

## YELLOW — Unverified

- The exact struct layout of the `result_struct` passed to `AStar_pathfind_search` is unknown
  without decoding that function (task #3 in progress). The result is an 8-dword (32-byte) struct
  copied back into FootClass in Find_Path.
- The meaning of `param_5` (forwarded from Find_Path to AStar) is unverified without decoding
  AStar_pathfind_search.
- The exact semantics of `-1, -1` zone exclusion args to AStar are unverified (likely "no zone
  filter" but not confirmed from AStar decompile yet).
- Vtable base used for slot 0x4c verification: `0x007e22a4` (AircraftClass vtable). FootClass slot
  0x4c is inherited — same address should apply to Infantry/Unit/Aircraft. Not verified for all
  three vtables independently.
