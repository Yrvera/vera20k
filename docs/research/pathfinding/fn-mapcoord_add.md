# MapCoord_Add — Decode Doc
**Proposed Ghidra label:** `MapCoord_Add` (already labeled)
**Address:** `0x0042D510`

## Summary

Adds two packed `(x, y)` short-pair coordinates together and stores the result into a
third output location. The function takes three short-pair pointers: `param_1` (ECX,
addend A), `param_3` (addend B), and `param_2` (output destination). It performs
16-bit signed addition on both the x and y components and writes the packed result.

No bounds checking, no overflow protection, no callee calls. Pure arithmetic utility.

**Active in YR: Yes.** 52 callers across the map, bridge, zone, and path systems.

---

## Signature

Source: `decompile_function 0x0042D510`

```c
void __thiscall MapCoord_Add(
    short      *param_1,     // ECX: addend A — packed (x, y) short pair
    undefined4 *param_2,     // destination for result (written as packed uint32)
    short      *param_3      // addend B — packed (x, y) short pair
)
```

`param_1` is passed via ECX (`__thiscall` convention). `param_2` and `param_3` are on
the stack.

---

## Decompilation

Source: `decompile_function 0x0042D510`

```c
void __thiscall MapCoord_Add(short *param_1, undefined4 *param_2, short *param_3)
{
    param_3 = (short *)CONCAT22(param_3[1] + param_1[1], *param_3 + *param_1);
    *param_2 = param_3;
    return;
}
```

`CONCAT22(hi, lo)` = Ghidra's notation for packing two 16-bit values into a 32-bit
word: high 16 bits = `param_3[1] + param_1[1]` (y + y), low 16 bits = `*param_3 + *param_1` (x + x).

---

## Assembly

Source: `disassemble_function 0x0042D510`

```
0042d510: MOV EAX, dword ptr [ESP+0x8]     ; EAX = &param_3
0042d514: MOV DX, word ptr [EAX]            ; DX = param_3.x
0042d517: MOV AX, word ptr [EAX+0x2]        ; AX = param_3.y
0042d51b: ADD AX, word ptr [ECX+0x2]        ; AX = param_3.y + param_1.y
0042d51f: ADD DX, word ptr [ECX]             ; DX = param_3.x + param_1.x
0042d522: MOV word ptr [ESP+0xa], AX        ; write result.y to stack temp
0042d527: MOV EAX, dword ptr [ESP+0x4]      ; EAX = &param_2 (destination)
0042d52b: MOV word ptr [ESP+0x8], DX        ; write result.x to stack temp
0042d530: MOV ECX, dword ptr [ESP+0x8]      ; ECX = packed (result.x, result.y)
0042d534: MOV dword ptr [EAX], ECX          ; *param_2 = packed result
0042d536: RET 0x8
```

The stack layout places `result.y` at `[ESP+0xa]` and `result.x` at `[ESP+0x8]`, which
form a contiguous 32-bit packed coordinate at `[ESP+0x8]`. This is then read as a
`dword` into ECX and written to the destination.

---

## Behavioral Analysis

### MapCoord packed format

A `MapCoord` (packed cell coordinate) is a `uint32` with:
- low `short` = x (cell column)
- high `short` = y (cell row)

So `&coord` can be read as `short[2]` where `[0]` = x and `[1]` = y.

### Operation

```
result.x = param_1.x + param_3.x
result.y = param_1.y + param_3.y
*param_2 = pack(result.x, result.y)
```

Arithmetic is 16-bit signed addition (`short`). Overflow wraps silently.

### Callee used by path smoothing

`Path_smooth_single_segment @ 0x0042B420` uses this to advance the current position
forward along a direction sequence when smoothing corners. The direction deltas come
from `g_DirectionOffsets @ 0x0089F688` (8-entry table, dx/dy shorts at stride 4).

---

## Struct Field Accesses

| Struct | Offset | Type | Meaning |
|---|---|---|---|
| `MapCoord` (packed) | `+0x00` | `short` | x (cell column) |
| `MapCoord` (packed) | `+0x02` | `short` | y (cell row) |

Confirmed from disassembly: `MOV DX, word ptr [EAX]` reads x; `MOV AX, word ptr [EAX+0x2]` reads y.

---

## Callers (52 total)

Selected callers verified via `get_function_callers 0x0042D510`:

| Caller | Address | Notes |
|---|---|---|
| `Path_smooth_single_segment` | `0x0042B420` | Advances map coord during path corner-smoothing |
| `MapClass__BridgePavementSpanWalker` | `0x00569760` | Bridge pavement cell traversal |
| `MapClass__AddBridgeZoneEdges` | `0x005851B0` | Zone edge computation for bridge cells |
| `MapClass__RemoveBridgeZoneEdges` | `0x00584E50` | Zone edge cleanup |
| `MapClass__Resize` | `0x00565C10` | Map resize cell iteration |
| `BuildingClass__Unlimbo` | `0x00440580` | Building placement coordinate adjustment |
| `SlaveManagerClass__AI_Update` | `0x006AF6C0` | Slave unit targeting |
| `HouseClass__AI_ScanBasePerimeter` | `0x005082C0` | AI perimeter scan |

Full list of 52 callers available via `get_function_callers 0x0042D510`.

---

## Callees

None. Pure arithmetic — no function calls. Verified via `get_function_callees 0x0042D510`
(returned "No callees found").

---

## INI Keys

None. Pure coordinate arithmetic utility.

---

## Self-Proof (3 Claims Verified This Session)

1. **No callees** — confirmed via `get_function_callees 0x0042D510` returning "No
   callees found for function: null". Pure arithmetic with no function calls.

2. **x at `+0x00`, y at `+0x02` (short pairs)** — confirmed from disassembly:
   `MOV DX, word ptr [EAX]` (x = offset 0) and `MOV AX, word ptr [EAX+0x2]` (y = offset 2).
   Verified via `disassemble_function 0x0042D510`.

3. **52 callers across map/bridge/zone/path systems** — confirmed via
   `get_function_callers 0x0042D510` returning 52 entries including
   `Path_smooth_single_segment`, multiple `MapClass__*`, `BuildingClass__Unlimbo`,
   `SlaveManagerClass__AI_Update`, and others.

---

## YELLOW (Unverified)

| Item | Why unverified | How to verify |
|---|---|---|
| Silent wrap on 16-bit overflow | Behavior when result exceeds `short` range — no clamp or error path visible in decompile | No action needed unless callers pass out-of-range deltas |
| `__thiscall` parameter mapping | ECX = param_1 per Ghidra; no independent runtime confirmation | Trace caller with debugger |
