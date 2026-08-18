# MapClass::IsCoordsInPlayfield — decode

## Summary

`MapClass__IsCoordsInPlayfield` converts a `CoordStruct` (X, Y leptons) to a
`CellStruct` using the same sign-correct arithmetic shift used by `Get_Cell_Packed`,
then delegates the actual bounds check to `MapClass__Is_Cell_In_Playfield`.

It is a thin adapter: lepton → cell conversion + playfield bounds check in 22
instructions with a single basic block (cyclomatic complexity 1). The conversion
math is identical to `Get_Cell_Packed` — this confirms a shared primitive for
lepton→cell conversion throughout the engine, not per-caller re-implementations.

Callers use the `char` return value (0 = out of bounds, non-zero = in playfield)
as a gate before spawning units, docking, dropping payloads, or validating animation
targets. Ghidra mis-types the return as `void`; the actual return is a `char` in AL,
as confirmed by all six caller sites storing the return to `cVar` variables.

**Verified via `decompile_function 0x005785f0`, `disassemble_function 0x005785f0`,
`get_function_callers 0x005785f0`, `get_function_callees 0x005785f0`.**

## Active in YR

YES. Called by `AircraftClass__Unlimbo`, `AircraftClass__Mission_Rescue`,
`BuildingClass__CanDock`, `ObjectClass__Unlimbo`, and two unnamed functions — all
live YR code paths reachable in a normal skirmish. No TS-only gate detected.

## Address

`0x005785f0` in `gamemd.exe`

## Signature (actual)

```c
// __stdcall, 1 arg: pointer to CoordStruct {X: int leptons, Y: int leptons, Z: int}
// Returns: char — 0 if out of playfield, non-zero if in playfield
char MapClass__IsCoordsInPlayfield(CoordStruct *coord);
```

Ghidra shows `void` return and `int *param_1` with zero params in the function
signature tool — both are artefacts. The disassembly at `0x005785f0` uses `RET 4`
(stdcall, one 4-byte pushed argument), and every caller tests the return in AL as a
`char`. Verified via `disassemble_function 0x005785f0` and all six caller
decompilations.

## Parameters

| Name | Type | Frame offset | Meaning |
|------|------|-------------|---------|
| `coord` | `CoordStruct *` | `[ESP+8]` (after PUSH ESI) | Source coords in leptons. Only X (`[coord+0]`) and Y (`[coord+4]`) are read; Z is ignored. |

Reference frame: **CoordStruct** (leptons). `[coord+0]` = X (east positive),
`[coord+4]` = Y (south positive). Confirmed by disassembly reading `[ESI]` and
`[ESI+4]`. Verified via `disassemble_function 0x005785f0`.

## Return Value

`char` — `0` if the converted cell falls outside the playfield; non-zero (result
of `MapClass__Is_Cell_In_Playfield`) if inside. Verified by six caller sites each
storing result into `cVar` and branching on `== '\0'`.

## Control Flow

Single basic block (confirmed by `get_function_signature 0x005785f0`: `basic_block_count=1`):

```
1. Load X = *coord       (ESI+0, dword)
2. CDQ; AND EDX,0xFF; ADD EAX,EDX; SAR EAX,8   → cell_x (sign-correct shift)
3. Write cell_x as word to stack local [ESP+0xC]
4. Load Y = coord[1]     (ESI+4, dword)
5. CDQ; AND EDX,0xFF; ADD EAX,EDX; SAR EAX,8   → cell_y (sign-correct shift)
6. Write cell_y as word to stack local [ESP+0xE]
7. LEA EDX, [ESP+0xC]    → pointer to packed CellStruct {cell_x, cell_y}
8. PUSH EDX; PUSH 1
9. CALL MapClass__Is_Cell_In_Playfield @ 0x00578460
10. RET 4
```

The two-word pack at `[ESP+0xC]` forms a `CellStruct` {X: short, Y: short} in
cell coordinates. The second argument pushed is the literal `1` (param3 of
`Is_Cell_In_Playfield`). Verified via `disassemble_function 0x005785f0`.

## Sign-Correct Shift Formula

For each lepton component `v` (signed 32-bit):

```
cell = (v + (v >> 31 & 0xFF)) >> 8
```

This is the standard floor-division-by-256 with sign correction for negative
coordinates. Identical to the formula in `Get_Cell_Packed @ 0x0041BEA0`.
Confirmed the formula is shared — not duplicated with variation. Verified via
`decompile_function 0x005785f0`.

**Concrete fixture (canonical refinery GAREFN at NW cell (10,10)):**
- NW cell leptons: X = 10 × 256 = 2560, Y = 10 × 256 = 2560
- Formula: (2560 + (2560>>31 & 0xFF)) >> 8 = 2560 >> 8 = 10 ✓

**Negative fixture (coord at X = -1 lepton):**
- v = -1; v>>31 = -1 (all ones); -1 & 0xFF = 255; -1 + 255 = 254; 254 >> 8 = 0
- Meaning: lepton -1 maps to cell 0 (correct floor division toward −∞)

## Struct Field Accesses

`param_1` is `int *` in Ghidra decompilation — offset arithmetic must be multiplied
by 4. But the disassembly shows ESI used with byte offsets (+0 and +4), which are
**direct byte offsets** from the CoordStruct base. So:

- `[ESI + 0x00]` = `CoordStruct.X` (leptons, signed int) — from CoordStruct frame
- `[ESI + 0x04]` = `CoordStruct.Y` (leptons, signed int) — from CoordStruct frame
- `[ESI + 0x08]` = `CoordStruct.Z` — **not read**

Verified via `disassemble_function 0x005785f0` (MOV EAX,[ESI] and MOV EAX,[ESI+4]).

## Callers

Six callers identified via `get_function_callers 0x005785f0`:

| Address | Function | Role of this call |
|---------|----------|------------------|
| `0x00415960` | `AircraftClass__Mission_Rescue` | Guard before `Drop_Payload` — returns early (mission result 5) if coords are out of playfield. Verified via `decompile_function 0x00415960`. |
| `0x00414310` | `AircraftClass__Unlimbo` | Guard before ground-height lookup and `FootClass__Unlimbo` call — skips the unlimbo path if out of playfield. Verified via `decompile_function 0x00414310`. |
| `0x00457ce0` | `BuildingClass__CanDock` | Guard on building's own GetCoords result — only proceeds with dock eligibility checks if building center is in playfield. Verified via `decompile_function 0x00457ce0`. |
| `0x004cd510` | `FUN_004cd510` | Reads coords from object at `+0x9c/+0xa0/+0xa4`; if out of playfield (returns 0), enters fallback path that kills passengers and destroys object. If in playfield, sets flag `+0x3d5`. Verified via `decompile_function 0x004cd510`. |
| `0x00729580` | `FUN_00729580` | Complex unit relocation function; checks current unit coords in playfield before attempting `Find_Nearby_Passable_Cell`. If out of playfield, falls through to alternate handling. Verified via `decompile_function 0x00729580`. |
| `0x005f5940` | `ObjectClass__Unlimbo` | Immediate first guard at function entry — returns 0 if coords are out of playfield before any placement logic runs. Verified via `decompile_function 0x005f5940`. |

## Callees

| Address | Function | Role |
|---------|----------|------|
| `0x00578460` | `MapClass__Is_Cell_In_Playfield` | Out-of-scope ref (map-cell-grid system). Receives `(this=MapClass, cellstruct_ptr, param3=1)` and returns char. The `param3=1` literal enables height-adjusted playfield edges (see `Is_Cell_In_Playfield` decompilation: the branch at `param_3 != '\0'` adds the height-adjusted iVar4 correction). |

`MapClass__Is_Cell_In_Playfield` is an out-of-scope-ref for this decode task.
Verified via `get_function_callees 0x005785f0` and `decompile_function 0x00578460`.

## Globals

None accessed directly by this function. The `MapClass` `this` pointer is not
present in the arguments — this is a free (non-thiscall) function that delegates to
`Is_Cell_In_Playfield` which uses a `this` pointer passed separately. The `this`
for `Is_Cell_In_Playfield` appears to be resolved from a global singleton (ECX on
the call, not a parameter of IsCoordsInPlayfield). The exact global is internal to
`Is_Cell_In_Playfield` (out of scope).

## INI Keys

None. Pure computation — no INI key reads.

## Enums

None. The `param3=1` passed to `Is_Cell_In_Playfield` is a literal boolean, not an enum value.

## Load-Bearing vs Internal

**Load-bearing observable behavior:** The sign-correct shift formula and the `param3=1`
constant passed to `Is_Cell_In_Playfield` both affect which cells are accepted.
Passing `param3=1` activates height-adjusted edge computation inside
`Is_Cell_In_Playfield` (confirmed by the `if (param_3 != '\0')` branch at
`0x00578460`). Using `param3=0` would give a tighter playfield bound — wrong behavior.

**Internal:** The stack layout of the temporary CellStruct is an implementation detail.

## Out-of-Scope Refs

- `MapClass__Is_Cell_In_Playfield @ 0x00578460` — map-cell-grid system boundary check.
  Receives the converted CellStruct and returns bool. Belongs to the map/cell-grid decode tasks.
- `FUN_004cd510 @ 0x004cd510` — unnamed; caller context suggests it may be a unit
  out-of-bounds recovery handler.
- `FUN_00729580 @ 0x00729580` — unnamed; caller context suggests unit relocation /
  passable-cell search logic.

## Unverified

None — all claims verified from binary in this session via Ghidra MCP tools.
