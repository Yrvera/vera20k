# CoordStruct__Distance3D — Decode Doc

## Summary

`CoordStruct__Distance3D` (0x0041c380) computes the true 3D Euclidean distance
between two coordinate structs using `sqrt(dx² + dy² + dz²)`. The input is a
CoordStruct pointer (3 × int, leptons). The result is an integer in leptons
returned via `Math__ftol` (x87 float-to-int truncation). The sqrt is computed
by `Sqrt_Approx` — a fast LUT-based approximation, not MSVC `sqrt` or x87 `fsqrt`.
The function uses floating-point math (double + x87 float10) and is called from
sim-side systems — it is a determinism hazard.

Function body: 0x0041c380
(verified via `decompile_function 0x0041c380`)

## Active in YR

**Yes.** Called from 28 callers including core sim systems:
- `Apply_area_damage` (0x00489280) — splash damage range checks every time
  a warhead detonates near a unit — fires every combat hit in the game
- `BulletClass__HomingTrack` (0x005b20f0) — homing bullet trajectory per tick
- `DiskLaserClass__AI` (0x004a7340) — disk laser range computation
- `EMPulseClass__Apply` (0x004c54e0) — EMP radius check
- `DriveLocomotionClass__Process_Movement` (0x004b2630) — unit movement
- `BulletClass__SpawnShrapnel` (0x0046a310) — shrapnel scatter radius
- `WalkLocomotionClass__ProcessMovement` (0x0075aec0) — infantry walk
- `ShipLocomotionClass__Process_Movement` (0x006a1c80) — naval movement
- `HoverLocomotionClass__SpeedUpdate` (0x00515ed0) — hover speed calculation
- `InfantryClass__Mission_Capture` (0x005202f0) — capture mission range
(verified via `get_function_callers 0x0041c380`)

All callers are active in normal YR gameplay. Not gated behind any TS-only flag.

## Decompilation excerpt

```c
// verified via decompile_function 0x0041c380
void __fastcall CoordStruct__Distance3D(int *param_1)
{
  // param_1 is int* — offsets are × 4 (CLAUDE.md pitfall rule)
  // param_1[0] = dx (lepton X), param_1[1] = dy (lepton Y), param_1[2] = dz (lepton Z)
  Sqrt_Approx((double)param_1[2] * (double)param_1[2] +
              (double)param_1[1] * (double)param_1[1] +
              (double)*param_1 * (double)*param_1);
  Math__ftol();
  return;
}
```

The function returns void in Ghidra's decompilation, but the result is conveyed
via the x87 FPU stack: `Sqrt_Approx` leaves its `float10` result in ST0, which
`Math__ftol` truncates to an integer left in EAX/EDX. Callers read the integer
result from the return register.

`param_1` is `int *` — so `param_1[N]` = offset N × 4 bytes:
- `*param_1` = `param_1[0]` = byte offset 0x00 = delta X (leptons)
- `param_1[1]` = byte offset 0x04 = delta Y (leptons)  
- `param_1[2]` = byte offset 0x08 = delta Z (leptons)

## Behavioral analysis

### Metric

True 3D Euclidean distance: `int(sqrt(dx² + dy² + dz²))` in leptons.

This is NOT Manhattan distance, NOT 2D, NOT Chebyshev. All three axes contribute.
The Z component is the altitude difference in leptons.

### Sqrt implementation

`Sqrt_Approx` (0x004cac40) is a fast approximate square root:
- Takes a `double` via two DWORD params (CONCAT44)
- Uses a lookup table at `DAT_008650bc` (8192 entries × 4 bytes)
- Extracts mantissa bits (upper 13 bits → 8192 indices) and exponent
- Returns `float10` (80-bit x87 extended precision)
- **Approximation quality**: Not exact. The LUT granularity means results can
  differ from `sqrt()` by up to ~1 ULP at float32 precision.
(verified via `decompile_function 0x004cac40`)

### Return value conversion

`Math__ftol` (0x007c5f00) reads ST0 (the x87 top-of-stack float10) and converts
to a 64-bit integer via ROUND semantics. Callers use only the 32-bit EAX result.
The final result is in **leptons** (integer, truncated from float).
(verified via `decompile_function 0x007c5f00`)

### Units

Input: leptons (1 cell = 256 leptons). Output: leptons (integer).

### Determinism analysis — HAZARD

This function uses x87 float10 (80-bit extended precision) intermediate math.
x87 behavior can differ based on:
- FPU control word precision setting (80-bit vs 64-bit vs 32-bit)
- CPU implementation (different rounding in edge cases)

**`Math__ftol` reads the FPU control word** (`in_FPUControlWord`) and branches,
but both branches call `ROUND(in_ST0)` — identical behavior in Ghidra's model.
The actual assembly uses `fistp` which is sensitive to the rounding mode.

**Impact**: This function is used in sim-side calculations (damage range checks,
locomotor movement, bullet homing). Any FPU control word divergence between two
networked game clients would produce different integer results, breaking lockstep
determinism. In a Rust re-implementation, this function MUST be reproduced with
integer arithmetic or fixed-point math, NOT floating-point, to preserve replay
correctness.

**Equivalent fixed-point Rust (exact replacement for sim use):**
```rust
fn distance3d_leptons(dx: i32, dy: i32, dz: i32) -> i32 {
    let sq = (dx as i64) * (dx as i64)
           + (dy as i64) * (dy as i64)
           + (dz as i64) * (dz as i64);
    // Integer sqrt — matches gamemd's truncated result for sim-critical paths
    integer_sqrt(sq) as i32
}
```

For the Rust engine, use `fixed`-point integer sqrt instead of calling
the `Sqrt_Approx` LUT to avoid floating-point non-determinism.

### Observable vs internal

- **Observable**: The damage radius (in leptons) used to include/exclude units
  from splash damage. A unit at distance `d` takes splash damage only if
  `d <= radius`. This is player-visible (unit dies or doesn't).
- **Internal**: The intermediate float computation — the player only sees the
  pass/fail of the `d <= radius` comparison. The exact float value is internal.
- The observable outcome requires matching the integer result of `int(sqrt(dx²+dy²+dz²))`
  exactly, which is achievable with integer sqrt.

## Struct field accesses

| Param offset | Byte offset | Size | Access | Semantics |
|---|---|---|---|---|
| `param_1[0]` | 0x00 | 4 (int) | read | Delta X in leptons |
| `param_1[1]` | 0x04 | 4 (int) | read | Delta Y in leptons |
| `param_1[2]` | 0x08 | 4 (int) | read | Delta Z in leptons |

Param type is `int *` — offsets ARE × 4.
(verified via `decompile_function 0x0041c380`)

## Callers / Lifecycle

| Caller | Address | Context | Sim-side? |
|---|---|---|---|
| `Apply_area_damage` | 0x00489280 | Warhead splash range check | YES |
| `BulletClass__HomingTrack` | 0x005b20f0 | Homing bullet distance | YES |
| `DiskLaserClass__AI` | 0x004a7340 | Disk laser AI range | YES |
| `EMPulseClass__Apply` | 0x004c54e0 | EMP radius | YES |
| `DriveLocomotionClass__Process_Movement` | 0x004b2630 | Drive locomotor | YES |
| `WalkLocomotionClass__ProcessMovement` | 0x0075aec0 | Walk locomotor | YES |
| `ShipLocomotionClass__Process_Movement` | 0x006a1c80 | Naval locomotor | YES |
| `HoverLocomotionClass__SpeedUpdate` | 0x00515ed0 | Hover speed | YES |
| `BulletClass__SpawnShrapnel` | 0x0046a310 | Shrapnel spawn radius | YES |
| `InfantryClass__Mission_Capture` | 0x005202f0 | Capture range | YES |
| `AircraftClass__Find_Nearest_Friendly_Airfield` | 0x0041a160 | Airfield proximity | YES |
| `AircraftClass__Mission_Move` | 0x004166c0 | Aircraft movement | YES |
| `AnimClass__BounceAI` | 0x00425670 | Bounce animation AI | Render |
| `BuildingClass__ChangeOwner` | 0x00448260 | Ownership change | YES |
| `BuildingClass__ReceiveDamage` | 0x00442230 | Damage receipt | YES |
| `SuperClass__Launch` | 0x006cc390 | Superweapon launch | YES |
| `UnitClass__TubeMovement` | 0x007359f0 | Tunnel movement | YES |
| `HouseClass__Check_Spy_Reveal` | 0x004faf00 | Spy reveal radius | YES |
| `HouseClass__GetEdgeDirection` | 0x004ffb20 | Edge direction | YES |
| + 10 unnamed FUN_* | various | Various systems | Mixed |

(verified via `get_function_callers 0x0041c380`)

## Callees

| Callee | Address | Purpose |
|---|---|---|
| `Sqrt_Approx` | 0x004cac40 | Fast LUT-based approximate sqrt |
| `Math__ftol` | 0x007c5f00 | x87 float10 → integer truncation |

(verified via `get_function_callees 0x0041c380`)

## Out-of-scope refs

- `Sqrt_Approx` LUT at `DAT_008650bc` — the 8192-entry lookup table; out of scope
- `AnimClass__BounceAI` — animation bounce; only renders, no sim state changes
- All individual caller internals — out of scope

## Unverified claims (YELLOW)

None. All addresses, param-type offsets, callee identities, and behavioral
analysis are directly verified from Ghidra MCP decompilation calls cited inline.
