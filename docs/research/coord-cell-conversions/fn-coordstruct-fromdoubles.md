# CoordStruct__FromDoubles — Decode Doc

## Summary

`CoordStruct__FromDoubles` (0x004399a0) reads three consecutive x87 FPU stack
registers (ST0, ST1, ST2) via three calls to `Math__ftol`, converts each
`float10` to an integer via truncation/rounding, and writes them into a
CoordStruct at `param_1[0]` (X), `param_1[1]` (Y), `param_1[2]` (Z).
Returns `param_1`. This is a double→int boundary gate for animation-computed
coordinates, not a general-purpose function. It is called exclusively from
animation systems that pre-load the FPU stack with double-precision coordinates.
It is a determinism hazard: the FPU rounding mode affects the result.

Function body: 0x004399a0
(verified via `decompile_function 0x004399a0`)

## Active in YR

**Yes.** 3 callers, all animation systems active in YR:
- `AnimClass__AI` (0x00423ac0) — per-tick update for bounce/meteor animations;
  result fed to `Apply_area_damage` for landing impact damage
- `AnimClass__ProcessBounceResult` (0x00423930) — bounce physics result
  processing; result used to look up cell occupants and apply damage
- `VoxelAnimClass__AI` (0x00749f30) — voxel animation per-tick update

All callers fire during normal gameplay whenever a bounce-physics animation
plays (e.g., meteor impacts, bouncing projectiles).
(verified via `get_function_callers 0x004399a0`)

## Decompilation excerpt

```c
// verified via decompile_function 0x004399a0
undefined4 * CoordStruct__FromDoubles(undefined4 *param_1)
{
  undefined4 uVar1, uVar2, uVar3;

  // Math__ftol reads ST0/ST1/ST2 from x87 FPU stack (pre-loaded by caller)
  uVar1 = Math__ftol();  // ST2 → int (Z)
  uVar2 = Math__ftol();  // ST1 → int (Y)
  uVar3 = Math__ftol();  // ST0 → int (X)
  *param_1 = uVar1;      // param_1[0] = X leptons
  param_1[1] = uVar2;    // param_1[1] = Y leptons
  param_1[2] = uVar3;    // param_1[2] = Z leptons
  return param_1;
}
```

Note: Ghidra's decompilation may misordered the ST0/ST1/ST2 pop sequence.
The actual x87 stack pop order depends on whether the callee uses `fst`/`fstp`
or the caller pre-loads in a specific order. The observed callers pass
coordinates in standard X/Y/Z order (leptons) to the FPU stack before calling,
and the resulting CoordStruct is used in positional contexts consistent with
X at offset 0, Y at offset 4, Z at offset 8.

`param_1` is `undefined4 *` — offsets are × 4.

## Behavioral analysis

### Purpose

This function is NOT a general "convert doubles to CoordStruct" utility.
It is a very narrow x87-FPU-convention bridge: it assumes the caller has
pre-loaded three doubles onto the x87 FPU stack (via `fld` instructions),
and pops them off sequentially via `Math__ftol`. The function name
"FromDoubles" reflects the ABI — the inputs are 64-bit doubles on the FPU
stack, not function arguments.

### Rounding mode

Uses `Math__ftol` (0x007c5f00) — same as `CoordStruct__Distance3D`. The
`fistp` instruction is used internally, which applies the FPU rounding mode
(typically round-to-nearest in gamemd's default FPU state). This is distinct
from simple truncation (`(int)x`).
(verified via `get_function_callees 0x004399a0` and `decompile_function 0x007c5f00`)

### Units

Output: leptons (integer). The animation physics systems compute positions in
doubles (typically lepton-scale), and this function converts them to integer
leptons for use in CoordStruct-based APIs.

### Determinism analysis — HAZARD

Same hazard as `CoordStruct__Distance3D`: x87 float10/fistp is sensitive to
the FPU control word precision setting. The result can differ between clients
with different FPU states.

**Sim-side impact**: `AnimClass__AI` feeds the result directly to
`Apply_area_damage` — the animation landing position determines which units
take damage from meteor/bounce-anim impacts. If two clients compute different
integer leptons for the same bounce landing, they will include/exclude different
units from splash damage, breaking lockstep determinism.

**Rust implementation guidance**: Do not use `f64` or `f32` for the intermediate
animation position when computing damage origin coordinates. Convert from the
bounce physics (which may legitimately use float for rendering) using a
deterministic rounding convention (truncation or fixed-point) that matches
the observed gamemd behavior for the damage-application path.

### Callers' FPU stack loading pattern

In `AnimClass__AI` (verified via `decompile_function 0x00423ac0`):
```c
// Pattern before CoordStruct__FromDoubles calls:
uVar14 = *(undefined4 *)(param_1[0x32] + 0x330);  // warhead type
uVar23 = 0;
Math__ftol(0, uVar14, 1);          // pre-loads Z? onto FPU
CoordStruct__FromDoubles(&local_54);  // pops 3 FPU values into X/Y/Z
Apply_area_damage(uVar23, uVar14, uVar26);  // uses the coord
```
The exact FPU loading happens in the immediate precede; Ghidra's decompilation
abstracts the FPU state, so the exact double values are not visible in the
C pseudocode. The key point: the caller is responsible for loading doubles.

In `AnimClass__ProcessBounceResult` (verified via `decompile_function 0x00423930`):
```c
uVar7 = CoordStruct__FromDoubles(local_24);  // pops bounce endpoint coords
iVar4 = CellClass__Get_Cell_At(uVar7);  // look up cell at the coordinate
```
Here it's used purely for cell lookup (rendering/AI context) — no immediate
damage application.

### Observable vs internal

- **Observable**: The landing cell for bounce-animation damage. If the coordinate
  converts to cell (X, Y), the splash damage hits occupants of that cell.
  Player-visible: unit takes damage or not from meteor/bouncing-projectile hits.
- **Internal**: The double-to-int conversion itself. The player sees whether
  damage happens, not the exact lepton value.

## Struct field accesses

| Param offset | Byte offset | Size | Access | Semantics |
|---|---|---|---|---|
| `*param_1` | 0x00 | 4 (int) | write | X in leptons (from FPU ST) |
| `param_1[1]` | 0x04 | 4 (int) | write | Y in leptons (from FPU ST) |
| `param_1[2]` | 0x08 | 4 (int) | write | Z in leptons (from FPU ST) |

Param type is `undefined4 *` — offsets are × 4.
(verified via `decompile_function 0x004399a0`)

## Callees

| Callee | Address | Purpose |
|---|---|---|
| `Math__ftol` | 0x007c5f00 | x87 float10 → integer, called 3 times |

(verified via `get_function_callees 0x004399a0`)

## Callers / Lifecycle

| Caller | Address | Context | Sim-side? |
|---|---|---|---|
| `AnimClass__AI` | 0x00423ac0 | Bounce/meteor anim per-tick; feeds Apply_area_damage | YES (damage path) |
| `AnimClass__ProcessBounceResult` | 0x00423930 | Bounce physics result; cell lookup and damage | Partly |
| `VoxelAnimClass__AI` | 0x00749f30 | Voxel animation per-tick | Unknown (not decompiled) |

(verified via `get_function_callers 0x004399a0`)

## Out-of-scope refs

- `Apply_area_damage` (0x00489280) — splash damage system; out of scope
- `BounceClass__Update` — bounce physics engine; out of scope
- `VoxelAnimClass__AI` internals — not decompiled; context unknown
- Animation INI keys that control bounce behavior (`Bounces=`, `BounceAnim=`)

## Unverified claims (YELLOW)

**UNVERIFIED**: The exact FPU stack pop order (which Math__ftol call reads X vs
Y vs Z). Ghidra abstracts the FPU stack and the three `uVar` assignments may be
shown in a different order than the actual x86 `fistp` instructions. The caller
context (animation bounce coordinates fed to Apply_area_damage in X/Y/Z order)
implies the standard CoordStruct layout (X at 0, Y at 4, Z at 8), but the
exact mapping of the three `Math__ftol` results to the three writes is not
confirmed by disassembly inspection in this session — only by caller context.
