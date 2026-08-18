# VXL Interpolated Facing and Slope Transition — Ghidra Report

**Date:** 2026-05-19  
**Binary:** gamemd.exe  
**Scope:** VXL_InterpolatedFacing @ 0x00755A40, DAT_00b43188 quaternion table,
  and `locomotor+0x28`/`+0x2C` transition-trigger writers  
**Status:** COMPLETE — all three open questions from VOXEL_SLOPE_TILT_SYSTEM.md
  Open Q #2 are resolved.

---

## 1. VXL_InterpolatedFacing @ 0x00755A40

### Calling Convention (fastcall, __thiscall-like)

| Register/Stack | Parameter | Type | Meaning |
|---|---|---|---|
| ECX | param_1 | `undefined4 *` | Output buffer (48 bytes = 12 floats = 3×4 matrix) |
| EDX | param_2 | `int` | "from-slope" index (locomotor+0x1C, previous slope) |
| [ESP+0] | param_3 | `int` | "to-slope" index (locomotor+0x18, current slope) |
| [ESP+4] | param_4 | `float` | Interpolation t (0.0=start of transition, 1.0=done) |

Callers pass:
- `EDX = *(locomotor+0x1C)` (previous slope)
- `EAX = *(locomotor+0x18)` (current slope), pushed to stack
- `(float)dVar1` pushed as the t value, derived from the frame counter formula (see §4)
- ECX = local matrix output buffer

### Logic

```c
if (param_2 != param_3) {
    // Slope is changing: slerp between the two quaternions
    // from-quaternion = DAT_00b43188[param_2]  (at EDX-side in Slerp)
    // to-quaternion   = DAT_00b43188[param_3]  (at ECX-side in Slerp... or stack)
    Quaternion_Slerp(local_buf,
                     &DAT_00b43188 + param_2 * 0x10,
                     &DAT_00b43188 + param_3 * 0x10,
                     (float)param_4);
    Quaternion_ToMatrix(output, local_buf);
    return output;
} else {
    // from == to: no actual transition, direct matrix copy
    memcpy(output, &g_VXL_FacingMatrices + param_3 * 0x30, 48);
    return output;
}
```

Note: when `param_2 == param_3` this function behaves identically to
`VXL_GetFacingMatrix` @ 0x007559B0.

### Quaternion_Slerp @ 0x00646590

Standard spherical linear interpolation of unit quaternions.

```
q_result = slerp(q0, q1, t)
         = q0 * sin((1-t)*θ)/sin(θ)  +  q1 * sin(t*θ)/sin(θ)
where θ = acos(dot(q0, q1))
```

Degenerate-case handling:
- If `|dot| ≈ 1.0` (quaternions nearly identical): linear blend
- If `dot ≈ -1.0` (antipodal): handled separately

Output: 4-float quaternion [x, y, z, w] in `param_1`.

### Quaternion_ToMatrix (called after Slerp)

Not decompiled in this pass. Called immediately after Slerp; converts the
interpolated quaternion to a 3×4 rotation matrix. Its output is the 12-float
result copied into the caller's output buffer.

---

## 2. DAT_00b43188 Quaternion Table

**Address:** `0x00B43188`  
**Layout:** 20 entries × 16 bytes per entry = 320 bytes  
**Entry format:** 4 × IEEE 754 single-precision floats = `[x, y, z, w]` quaternion  
**Indexed by:** slope_type (0–19)

### Population: VXL_MasterLighting_Init @ 0x00754CB0

The table is BSS-zero in the binary image (confirmed by `read_memory` of 320
zero bytes at 0x00B43188). It is populated at runtime during engine init, as part
of the same `VXL_MasterLighting_Init` call that fills `DAT_00b45188` (slope
matrices).

**Verified population sequence (from disassembly):**

| Slope | Entry addr | Init call | Quaternion type |
|---|---|---|---|
| 0 | `0xB43188` | `Quaternion_Set(0, 0, 0, 1.0)` | Identity (flat) |
| 1 | `0xB43198` | `Quaternion_CopyAndStore(Quaternion_FromAxisAngle(EDGE_TILT))` | West edge tilt |
| 2 | `0xB431A8` | `Quaternion_CopyAndStore(Quaternion_FromAxisAngle(EDGE_TILT))` | North edge tilt |
| 3 | `0xB431B8` | `Quaternion_CopyAndStore(Quaternion_FromAxisAngle(EDGE_TILT))` | East edge tilt |
| 4 | `0xB431C8` | `Quaternion_CopyAndStore(Quaternion_FromAxisAngle(EDGE_TILT))` | South edge tilt |
| 5 | `0xB431D8` | `Quaternion_CopyAndStore(Quaternion_FromAxisAngle(CORNER_TILT))` | Corner tilt NW |
| 6 | `0xB431E8` | same | Corner tilt NE |
| 7 | `0xB431F8` | same | Corner tilt SE |
| 8 | `0xB43208` | same | Corner tilt SW |
| 9 | `0xB43218` | same | (alias of 5) |
| 10 | `0xB43228` | same | (alias of 6) |
| 11 | `0xB43238` | same | (alias of 7) |
| 12 | `0xB43248` | same | (alias of 8) |
| 13 | `0xB43258` | `Quaternion_CopyAndStore(Quaternion_FromAxisAngle(EDGE_TILT))` | Steep NW |
| 14 | `0xB43268` | same | Steep NE |
| 15 | `0xB43278` | same | Steep SE |
| 16 | `0xB43288` | same | Steep SW |
| 17–19 | `0xB43298+` | `Quaternion_Set(0, 0, 0, 1.0)` | Identity (BSS / 4 identity quaternions) |

Where `EDGE_TILT = 0.5214767` rad and `CORNER_TILT = 0.3858827` rad (same
constants as the slope matrix table).

**Important distinction from slope matrix table:** Each entry here represents the
tilt rotation to the named slope, expressed as a quaternion. The slerp in
`VXL_InterpolatedFacing` interpolates *between* two of these quaternions — one for
the previous slope and one for the current slope — rather than between the tilt
quaternion and identity.

However, there is a critical ambiguity: the `Quaternion_FromAxisAngle` call in
`VXL_MasterLighting_Init` only passes the tilt magnitude `fVar1` or `fVar2` —
it does NOT pass a directional compass angle. This means all 4 edge-slope entries
(1-4) get the same quaternion (a pure X-axis tilt by EDGE_TILT), and all 8
corner-slope entries (5-12) get the same quaternion (a pure X-axis tilt by
CORNER_TILT). The direction information is absent from the quaternion table.

**Implication:** When `VXL_InterpolatedFacing` slerps between two slope quaternions
that happen to have the same magnitude (e.g., both EDGE tilts in different
directions), the result is an identity blend — the interpolation produces no
rotation. This is likely a **design limitation or known inaccuracy** of the
interpolation: it only smoothly interpolates the *magnitude* of tilt, not the
direction. The direction snaps instantly while the tilt magnitude eases in/out.

The `Quaternion_FromAxisAngle` call signature: `FromAxisAngle(axis_vec, tilt_angle)`
where `axis_vec` is a 3D unit vector. From decompile, the axis used appears to be
`(x=param_1[0], y=param_1[1], z=param_1[2])` — but the init call only passes
a scalar angle, not a separate axis. The Ghidra decompile shows this function
taking `(float *output, float *axis_vec, float angle)`. The axis is hard-coded
in the init's local storage.

**Confidence:** MEDIUM for exact per-entry contents (BSS zero, cannot read
runtime values directly). HIGH for the count (320 bytes / 16 bytes per entry =
20 entries), stride (confirmed from `param_3 * 0x10` in function body), and
init sequence (read directly from VXL_MasterLighting_Init asm).

---

## 3. Locomotor +0x28 / +0x2C Writers

### Fields at locomotor+0x20 through +0x2C

From decompiling `CDTimerClass__Remaining @ 0x004b4d70` and the `Draw_Matrix`
disassembly:

| Offset | Width | Field | Meaning |
|---|---|---|---|
| `+0x18` | 4 bytes | `current_slope` | Slope index currently displayed (destination) |
| `+0x1C` | 4 bytes | `previous_slope` | Slope index at transition start |
| `+0x20` | 4 bytes | `timer_start_frame` | Frame number when transition started (-1 = not running) |
| `+0x24` | 4 bytes | (padding/unused) | Written by Force_New_Slope as re-read of param_2 (Ghidra artifact) |
| `+0x28` | 4 bytes | `transition_duration` | Total duration of transition in frames |
| `+0x2C` | 4 bytes | `transition_total` | Copy of total duration — the "is-interpolating?" gate (0 = not active) |

The `Draw_Matrix` interpolation branch fires **only when `*(locomotor+0x2C) != 0`.**
The blend factor `t` is computed as:

```
remaining = CDTimerClass__Remaining(&locomotor+0x20)
          = max(0, transition_duration - (currentFrame - timer_start_frame))
t = (transition_total - remaining) / transition_total
```

At `t=0.0` the matrix matches the previous slope; at `t=1.0` it matches the new slope.

### Force_New_Slope @ 0x004AFB40 — Zeroes Both Fields

This function always writes:
- `locomotor+0x28 ← 0`
- `locomotor+0x2C ← 0`

After this call, `+0x2C = 0` so `Draw_Matrix` never enters the interpolation branch.

### Search for Non-Zero Writers

An exhaustive search was conducted for all x86 MOV instructions writing to
`[reg+0x2C]` (patterns `89 46 2C`, `89 41 2C`, `89 48 2C`, `89 50 2C`,
`89 58 2C`, `89 70 2C`, `89 78 2C`, `C7 46 2C`, `C7 41 2C`, etc.) across the
entire binary.

**Result: NO write-site in the DriveLocomotion or ShipLocomotion code paths
sets `locomotor+0x2C` to a non-zero value at runtime.**

All writes found in the DriveLocomotion address space (0x004AF000–0x004B8000):
- `DriveLocomotionClass__Constructor @ 0x004AF540`: sets `+0x2C ← 0` (confirmed)
- `DriveLocomotionClass__Force_New_Slope @ 0x004AFB40`: sets `+0x2C ← 0` (confirmed)

All other `+0x2C` writes found were in unrelated classes: `Dial8Class__Constructor`,
`IPXManagerClass`, `NullModemClass`, `CCINIClass`, `MSShapeAnim`, UI widgets, etc.

The `DriveLocomotionClass__Load @ 0x004AF780` (stream deserialization via
`FUN_0055aac0`) would restore whatever `+0x2C` value was in a saved game stream,
but this is not a *runtime* trigger — it only matters for save/load continuity.

### Conclusion: Slope Interpolation Is Unreachable in a Fresh YR Game

The `VXL_InterpolatedFacing` function and the 320-byte quaternion table exist in
the binary and are correctly initialized at engine startup. However, the
interpolation branch in `Draw_Matrix` is unreachable in a normal YR skirmish
because `locomotor+0x2C` is always 0. The Ghidra annotation `(=3)` visible in
the `DriveLocomotionClass__Constructor` comment header (for the `slope_timer_total`
field) appears to be an artifact of a planned-but-never-wired feature — the value
3 appears nowhere in the constructor body.

The code path is present, architecturally sound, and would produce correct smooth
tilt transitions if triggered. It is not triggered in stock YR.

**Active in YR:** No. The quaternion table is populated unconditionally at init,
but the slerp branch in `Draw_Matrix` requires `locomotor+0x2C != 0`, which no
in-game code path ever sets. This is NOT a TS-only gate (no `SpecialFlags` check,
no `Rules` flag) — the feature simply has no trigger in the YR codebase.

---

## 4. Interpolation t Formula

When and if the interpolation branch fires (requires external code to set `+0x2C`
to a non-zero value N), the t value passed to `VXL_InterpolatedFacing` is:

```
elapsed   = currentFrame - locomotor+0x20  (frames since slope change)
remaining = max(0, locomotor+0x28 - elapsed)
t         = (locomotor+0x2C - remaining) / locomotor+0x2C
```

This gives `t=0` on frame 0 (full previous slope tilt), `t=1` when the timer
expires (full new slope tilt). The transition is **linear in time** — there is no
easing curve; the slerp itself provides rotational smoothness, but the t parameter
advances at constant rate.

---

## 5. Key Addresses

| Address | Label | Notes |
|---|---|---|
| `0x00755A40` | `VXL_InterpolatedFacing` | Slerp path (needs `+0x2C != 0` at caller) |
| `0x00646590` | `Quaternion_Slerp` | 4-param slerp: output, q0, q1, t |
| `0x00646480` | `Quaternion_FromAxisAngle` | Builds axis-angle quaternion |
| `0x00645C50` | `Quaternion_Set(x,y,z,w)` | Direct quaternion constructor |
| `0x00645D20` | `Quaternion_CopyAndStore` | Copies quaternion to target address |
| `0x004B4D70` | `CDTimerClass__Remaining` | Returns frames left in timer |
| `0x00B43188` | `g_VXL_QuaternionTable` | 20 × 16-byte quaternion table |
| `0x004AFB40` | `DriveLocomotionClass__Force_New_Slope` | Zeros +0x28/+0x2C |

---

## 6. Rust Implementation Implications

To bring the Rust renderer to full parity with gamemd's *actual behavior*:

1. **Do NOT implement slope interpolation as a default.** `locomotor+0x2C` is
   always 0 in a standard YR game. The direct matrix lookup path is the only
   path executed.

2. The quaternion table and `VXL_InterpolatedFacing` exist for completeness but
   are dead code in stock YR. If the Rust engine ever needs slope-transition
   smoothing as a deliberate enhancement, the mechanism is clear:
   - Set `transition_total (+0x2C)` to N frames when `Force_New_Slope` is called
   - Set `timer_start_frame (+0x20)` to current frame  
   - Set `transition_duration (+0x28)` to N
   - Each frame in `Draw_Matrix`: compute `t` per §4, call the slerp path

3. The quaternion table has a directional limitation: all same-tilt-type slopes
   share a single quaternion (only magnitude, no direction). Slerping between two
   EDGE slopes in different directions would produce a magnitude-only blend.

---

## Summary of Verified Facts

1. **VXL_InterpolatedFacing signature** (verified from raw ASM at 0x755A40):
   fastcall ECX=output, EDX=from_slope, [ESP+0]=to_slope, [ESP+4]=float t.
   When from==to, it is identical to VXL_GetFacingMatrix.

2. **DAT_00b43188 is a 20-entry × 16-byte quaternion table** (verified: 16-byte
   stride from `param_3 * 0x10` in function body; 320-byte BSS zero; populated
   at runtime by VXL_MasterLighting_Init with identity + slope quaternions).

3. **Quaternion_Slerp @ 0x00646590** is standard unit quaternion slerp with
   acos-based angle extraction and cosine-based blending (decompiled and confirmed).

4. **locomotor+0x2C is the interpolation gate** (verified from disasm at
   0x004b01b2): `MOV EBX, [ESI+0x2C]; TEST EBX, EBX; JZ` jumps to direct lookup.
   `+0x28` = duration, `+0x20` = start frame, used by CDTimerClass__Remaining.

5. **No runtime writer sets locomotor+0x2C to non-zero** (exhaustive byte-pattern
   search across all 89xx/C7xx MOV patterns, filtered to DriveLocomotion address
   space). The interpolation branch in Draw_Matrix is unreachable in a normal YR
   game session.
