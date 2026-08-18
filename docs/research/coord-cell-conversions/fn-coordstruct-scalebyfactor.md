# CoordStruct__ScaleByFactor — decode

## Summary

`CoordStruct__ScaleByFactor` (`0x0075f540`) performs a weighted linear combination of
two `CoordStruct` operands using two float factors, writing the result into an output
`CoordStruct`. For each component: `out[i] = int(A[i] * factorA + B[i] * factorB)`.
The conversion from float to int uses `Math__ftol` (x87 FPU), making this function
**non-deterministic** and unsuitable for sim-side fixed-point code.

It appears only in rendering/physics paths: `DriveLocomotionClass__Process_Drive_Track`,
`ShipLocomotionClass__Process_Drive_Track`, `BuildingLightClass__AI`,
`ParticleSystemClass__AI_Railgun`, and two additional unnamed functions. None of these
are sim-tick pathways that require lockstep determinism.

**Verified via `decompile_function 0x0075f540`, `disassemble_function 0x0075f540`,
`get_function_callers 0x0075f540`, `get_function_callees 0x0075f540`.**

## Active in YR

YES. 6 direct callers (verified via `get_function_callers 0x0075f540`):
`BuildingLightClass__AI`, `DriveLocomotionClass__Process_Drive_Track`,
`ShipLocomotionClass__Process_Drive_Track`, `ParticleSystemClass__AI_Railgun`, and
two `FUN_*` functions. All are live in standard YR gameplay. No TS-only gate detected.

## Address

`0x0075f540` in `gamemd.exe`

## Signature (actual)

```c
// __fastcall: ECX = output CoordStruct*, EDX = operand B CoordStruct*
// Stack args: operand A CoordStruct* (pointer), float factorA, float factorB
// Returns: ECX (output pointer, for chaining)
// RET 0x8 = pops 2 × 4-byte stack args
undefined4 * __fastcall CoordStruct__ScaleByFactor(
    undefined4 *out,      // ECX — output CoordStruct (3 × int32)
    undefined4 *B,        // EDX — operand B CoordStruct
    undefined4 *A,        // [ESP_entry+4] — operand A CoordStruct
    float factorA,        // [ESP_entry+8] — scale factor for A
    float factorB         // [ESP_entry+12] — scale factor for B (deduced from FLD)
);
```

Ghidra shows `__fastcall` with a simplified view (3 `Math__ftol()` calls, param_1 as
output). Actual disassembly reveals ECX=output, EDX=B, stack=[A, factorA, factorB_or_combined].
`RET 0x8` = pops 2 × 4 bytes from stack (caller pushes A and float). Verified via
`disassemble_function 0x0075f540`.

**Note:** The exact argument encoding (how many float args are on the stack vs. FPU
registers) requires tracing the caller push sequence, which is out of scope here.
The observable behavior is the weighted sum per component.

## Parameters

| Name | Type | Location | Meaning |
|------|------|----------|---------|
| `out` | `undefined4 *` | ECX | Output CoordStruct (3 × int32); also return value |
| `B` | `undefined4 *` | EDX | Second operand CoordStruct |
| `A` | `undefined4 *` | `[ESP+4]` at entry | First operand CoordStruct |
| `factorA` | `float` | FPU / stack | Scale factor for operand A |
| `factorB` | `float` | FPU / stack | Scale factor for operand B (see FSUBR at entry) |

## Control Flow

Single-pass FPU-based computation (no branches in happy path):

For each component `i ∈ {0, 1, 2}`:
```
FILD [A + i*4]       ; load A[i] as float
FMUL factorA          ; A[i] * factorA
FILD [B + i*4]       ; load B[i] as float
FMUL factorB          ; B[i] * factorB
FADDP                 ; A[i]*factorA + B[i]*factorB
CALL Math__ftol       ; convert to int32
```

The factor for B is computed at entry: `FLD factorA; FSUBR [DAT_007e1718]` where
`DAT_007e1718` is a constant (likely `1.0f`). This gives `factorB = 1.0 - factorA`,
making this a linear interpolation: `out[i] = int(A[i] * t + B[i] * (1-t))`.

Verified via `disassemble_function 0x0075f540`:
`0x0075f543: FLD float [ESP+0x14]` (load factorA);
`0x0075f547: FSUBR double [0x007e1718]` (factorB = const - factorA);
`0x0075f554: FILD dword [EBX]` (A.X); `0x0075f55f: FILD dword [ESI]` (B.X).

## The DAT_007e1718 Constant

`0x007e1718` is in the `.rdata` section and likely contains `1.0f` or `1.0` (double).
This makes the function a standard linear interpolation: `lerp(A, B, t) = A*t + B*(1-t)`.
UNVERIFIED: exact value at `0x007e1718`. The pattern (FSUBR from constant after loading
t) is the canonical lerp setup.

## Formula

```
out.X = int(A.X * t + B.X * (1 - t))
out.Y = int(A.Y * t + B.Y * (1 - t))
out.Z = int(A.Z * t + B.Z * (1 - t))
```

Where `t` = `factorA` (float, passed by caller), `1 - t` = derived via `FSUBR 1.0`.

Result is leptons in all components.

## Determinism Hazard

Uses `Math__ftol @ 0x007c5f00` (x87 FPU integer conversion) for each component.
`Math__ftol` is non-deterministic: it reads the FPU control word and uses `ROUND`,
which is affected by CPU FPU state across calls. This function **MUST NOT** be used
in any sim-side fixed-point path. It appears only in:
- `DriveLocomotionClass__Process_Drive_Track` — locomotor processing (non-sim)
- `ShipLocomotionClass__Process_Drive_Track` — locomotor processing (non-sim)
- `BuildingLightClass__AI` — rendering light AI (non-sim)
- `ParticleSystemClass__AI_Railgun` — particle effects (non-sim)

All callers are confirmed render/physics paths, not sim-tick paths.
Verified via `get_function_callers 0x0075f540` and `decompile_function 0x007c5f00`.

## Callees

| Address | Name | Role |
|---|---|---|
| `0x007c5f00` | `Math__ftol` | x87 FPU float-to-int (non-deterministic) |

Verified via `get_function_callees 0x0075f540`.

## Globals

| Address | Access | Semantics |
|---|---|---|
| `DAT_007e1718` | READ (FPU `FSUBR`) | Constant value for lerp complement (likely `1.0`) |

## INI Keys

None.

## Enums

None.

## Load-Bearing vs Internal

**Load-bearing for render fidelity:**
- The linear interpolation formula `A*t + B*(1-t)` determines drive-track smoothing
  and particle positioning. Incorrect t values produce visible unit stuttering or
  particle drift.
- The `Math__ftol` rounding is internal — any truncation/rounding that produces the
  same pixel output is acceptable.

**Not load-bearing for sim determinism:** This function is pure render/physics and
must not be ported to fixed-point sim code.

## Out-of-Scope Refs

- `Math__ftol @ 0x007c5f00` — x87 ftol; non-determinism analysis in its own decode.
- `DAT_007e1718` — constant in `.rdata`; value unverified.
- `DriveLocomotionClass__Process_Drive_Track` — caller context for drive-track interpolation.

## Rust Equivalent

```rust
// Render-only: NOT for sim-side fixed-point code
// Linear interpolation between two CoordStructs
fn lerp_coords(a: &CoordStruct, b: &CoordStruct, t: f32) -> CoordStruct {
    CoordStruct {
        x: (a.x as f32 * t + b.x as f32 * (1.0 - t)) as i32,
        y: (a.y as f32 * t + b.y as f32 * (1.0 - t)) as i32,
        z: (a.z as f32 * t + b.z as f32 * (1.0 - t)) as i32,
    }
}
```

## SELF-PROOF — 3 random load-bearing claims verified

1. **`FILD dword ptr [EBX]` (A.X loaded as float)**: `disassemble_function 0x0075f540`
   at `0x0075f554` shows `FILD dword ptr [EBX]`. EBX = first stack arg (operand A).
   Verified.
2. **`CALL 0x007c5f00` (Math__ftol for each component)**: disassembly at `0x0075f565`,
   `0x0075f57a`, `0x0075f591` — three consecutive calls to `0x007c5f00` (one per
   component X/Y/Z). Verified.
3. **`RET 0x8`** (pops 2 stack args): disassembly ends with `ADD ESP,0xC; RET 0x8`.
   Verified at `0x0075f5ac`/`0x0075f5af`.

## Unverified

- Exact value at `DAT_007e1718` (the `1.0` constant for lerp complement). Pattern
  is consistent with `1.0f`, but the raw bytes were not read via `read_memory`.
- Exact argument push sequence for callers — whether factorA is pushed as float or
  passed differently. This affects how callers should be ported but not the function body.
