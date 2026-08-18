# Turret/Barrel Tilt State Machine — `FUN_00729B40` (States 2–7) — Ghidra Report

**Date:** 2026-05-19  
**Function:** `FUN_00729B40` (Ghidra label: `Turret_barrel_tilt`)  
**Address:** `0x00729B40`  
**Size:** 754 bytes  
**Active in standard YR:** **NO — TS-DEAD**  
**Confidence:** C=HIGH (direct decompile), I=HIGH (vtable confirmed), B=HIGH (TS-dormancy confirmed via INI grep + xref trace)

---

## 0. Critical Up-Front Finding: This Is TunnelLocomotionClass

`FUN_00729B40` sits at **slot 9** of the **TunnelLocomotionClass ILocomotion vtable** at `0x007F5A24`.

| Evidence | Value |
|---|---|
| Vtable address (contains `0x00729B40`) | `0x007F5A48` = vtable base `0x007F5A24` + slot 9 × 4 |
| Class | TunnelLocomotionClass |
| Class CLSID | `{4A582743-9839-11D1-B709-00A024DDAFD1}` |
| INI references to CLSID | **Zero** — never bound to any YR unit |
| Dormancy doc | `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md` §4 |
| TS legacy status | DEFERRED-TS (TS subterranean units; YR/RA2 has none) |

**Implication for the Rust port:** The entire tilt state machine documented here never fires in a standard YR skirmish. Do NOT implement states 2–7 of this function as aircraft-specific behavior — they belong to an uninstantiated TS locomotor. The `FUN_00729B40` code runs only if the TunnelLocomotionClass is instantiated, which requires a mod that sets `Locomotor={4A582743-...}` on some unit.

---

## 1. Calling Convention — param_1 Pointer Discipline

`FUN_00729B40` is called via the ILocomotion vtable thunk. MSVC adjusts `this` so the called method receives `param_1 = instance + 4` (pointing at the ILocomotion sub-object, NOT the base of the locomotor instance).

Consequence: when `FUN_00729B40` reads `*(int *)(param_1 + 0x14)`, this is:

```
param_1 + 0x14 = (instance + 4) + 0x14 = instance + 0x18
```

**The state byte is at `instance + 0x18`** (4-byte int, read as `int`).

State writers in the TunnelLocomotionClass write directly to `param_1 + 0x18` where their `param_1 = instance` (fastcall raw-instance convention, no thunk adjustment). Both conventions refer to the same memory location.

Confirmed by:
- `TunnelLocomotionClass::Constructor @ 0x00728A00`: `XOR EAX, EAX; MOV [ESI+0x18], EAX` (initializes state to 0).
- State writers `FUN_007291F0` @ `0x007292C8`: `C7 46 18 02 00 00 00` (writes 2 to `[ESI+0x18]`).
- State writers `FUN_007298F0` @ `0x00729A8C`: `C7 46 18 06 00 00 00` (writes 6 to `[ESI+0x18]`).
- `FUN_00729580` @ `0x007297F6`: `C7 46 18 05 00 00 00` (writes 5 to `[ESI+0x18]`).
- State 3 write @ `0x007293D3`: `C7 46 18 03 00 00 00` (writes 3 to `[ESI+0x18]`), inside the same function as the state-2 write.
- `FUN_007291D0` reads `*(int *)(param_1 + 0x14)` (via vtable thunk, equivalent to `instance+0x18`) and compares to 4.

---

## 2. State Field Layout at `instance+0x18`

The TunnelLocomotionClass instance struct fields relevant to the tilt computation:

| Offset | Type | Field | Notes |
|---|---|---|---|
| `+0x18` | int (4 bytes) | **State** | 0 = ground, 2 = takeoff, 3 = hovering, 4 = (landing-zone check), 5 = descending, 6 = landing, 7 = alt-landing; constructor zeroes this |
| `+0x1C` | int | Destination X (lepton) | Used by state handlers to compute delta to target |
| `+0x20` | int | Destination Y (lepton) | Same |
| `+0x24` | int | Timer start frame | Written as `g_CurrentFrameCounter` at state-2 entry (`0x007292CC`), state-5 entry, state-6 entry |
| `+0x28` | int | StartFrame2 / timer base | Written at state-2 entry alongside `+0x24` |
| `+0x2C` | int | **Remaining duration** (timer countdown) | Elapsed-time computation: `iVar4 = [param_1+0x24]; if([param_1+0x24] != -1) { iVar8 = g_CurrentFrameCounter - [param_1+0x24]; if(iVar8 < iVar4) local_12c = iVar4 - iVar8; else local_12c = 0; }` |
| `+0x30` | int | **Total duration** | Denominator of the tilt-progress formula |
| `+0x34` | int | (duplicate of `+0x30` at entry) | Set to same value as `+0x30` at state-entry; possibly "initial total" |
| `+0x38` | byte | "Tilt-anim played" flag | Cleared to 0 at state-2 entry and state-5 entry |

Offset verification: all offsets read from decompiled function bodies using `param_1` as the raw instance pointer (fastcall, no thunk adjustment). The `+0x30` / `+0x2C` fields are the "elapsed" and "total" timer pair used in the progress formulas for states 2, 6, 7.

---

## 3. The Tilt State Machine — Full Decompile Summary

`FUN_00729B40` dispatches on `*(int *)(param_1 + 0x14)` (= `instance+0x18`):

```c
switch (*(int *)(param_1 + 0x14)) {
    case 0:  // ground / idle → slope-based tilt (already documented)
    case 2:  // takeoff
    case 3:  // hovering
    default: // unhandled → tilt = 0.0
    case 5:  // descending
    case 6:  // landing
    case 7:  // alt landing
}
```

State 1 and state 4 are NOT handled in `FUN_00729B40` — they fall through to the `default` case (tilt = 0.0, identity rotation).

### State 0 (ground/idle) — Already Documented

Reads cell `slope_type` from `cell+0x11C`, looks up `DAT_00B45188[slope_type]` matrix. See `VOXEL_SLOPE_TILT_SYSTEM.md` for complete documentation.

---

## 4. States 2–7 — Tilt Formulas (Verified from Decompile)

### State 2 — Takeoff

**Entry condition:** Written by `FUN_007291F0` (at `0x007292C8`) when a CDTimer expires **and** the unit's current facing matches the heading to destination. Simultaneously writes:
- `[param_1+0x18] = 2`
- `[param_1+0x28] = g_CurrentFrameCounter`
- `[param_1+0x2C] = <tick_count_from_stack_local>` (CDTimer or similar step count)
- `[param_1+0x30] = <TypeClass.Speed>` (via `vtable[0x84]()` → `Math__ftol()`)
- `[param_1+0x34] = same`
- Plays a sound via `VocClass__PlayAt(0)` and spawns an anim via `AnimClass__Constructor(Rules+0x294, coord, ...)`.
- Clears `[param_1+0x38] = 0`.

**Tilt formula (from decompile of `FUN_00729B40` state-2 branch):**

```c
iVar4 = *(int *)(param_1 + 0x30);   // total duration
if (iVar4 == 0) {
    tilt = 1.0 * (π/2);              // instant max
} else {
    local_12c = *(int *)(param_1 + 0x2c);  // remaining
    if (*(int *)(param_1 + 0x24) != -1) {
        iVar8 = g_CurrentFrameCounter - *(int *)(param_1 + 0x24);
        local_12c = (iVar8 < local_12c) ? (local_12c - iVar8) : 0;
    }
    elapsed = iVar4 - local_12c;
    tilt = ((double)elapsed / (double)iVar4) * (π/2);
}
```

**Summary:** `tilt = (elapsed / total) × π/2`, clamped by remaining-frames logic.

`param_3` (cache-key pointer) is invalidated by writing `*param_3 = 0xFFFFFFFF` at entry.

**Fields used:**
- `+0x30` = total duration (int, frames)
- `+0x24` = start frame (int; `-1` = no frame stamp, use remaining directly)
- `+0x2C` = remaining duration (int, frames)

**π/2 constant:** `_LAB_007e281f_1` resolves to the double `0x3FF921FB54442D18` = π/2 ≈ 1.5707963267948966 stored at `0x007E2820`. Verified: `read_memory @ 0x007E2820` returns `18 2D 44 54 FB 21 F9 3F` = `0x3FF921FB54442D18` ✓

---

### State 3 — Hovering (fully tilted)

**Entry condition:** Written at `0x007293D3` (`C7 46 18 03 00 00 00`) inside the same function body as the state-2 write. The write occurs after a second branch in `FUN_007291F0` that handles the case where the CDTimer fires but the unit is already aligned — it skips the initial facing-check branch and goes directly to state 3 (already vertically oriented, no tilt-up transition needed). Also reached when state-2 tilt animation is complete (transition within the step handler).

**Tilt formula:**

```c
tilt = 1.5707963267948966;  // π/2 exactly (IEEE 754 double literal)
```

Constant used: `1.5707963267948966` (double literal embedded directly in Ghidra decompile — no indirection via global, just the FPU constant). Equivalent to `0x3FF921FB54442D18`.

`param_3` is **not** invalidated for state 3 (no `*param_3 = 0xFFFFFFFF` write). The cache key is left as-is, so the matrix is eligible for caching.

**Fields used:** None — tilt is a fixed constant.

---

### State 5 — Descending (inverted)

**Entry condition:** Written by `FUN_00729580` at `0x007297F6` (`C7 46 18 05 00 00 00`). This function is the descent/approach handler. State 5 is entered when `FUN_004C1B50()` (distance to destination squared or similar metric) returns `< 0x14` (decimal 20 — very close to target). Simultaneously:
- Tries to find a passable landing cell via `FootClass__Find_Nearby_Passable_Cell`.
- If cell found, calls `(**(ILoco_vtable+0x44))(param_1+4, cell_X, cell_Y, 0)` (Head_To_Coord to the landing cell).
- If no cell found: calls `linked_obj->vtable[0x124](1)` (scatter/abort).
- Clears `[param_1+0x38] = 0`.

**Tilt formula:**

```c
tilt = -1.5707963267948966;  // -π/2 exactly (IEEE 754 double literal)
```

`param_3` is **not** invalidated for state 5.

**Fields used:** None — fixed constant.

---

### State 6 — Landing (positive-to-horizontal transition)

**Entry condition:** Written by `FUN_007298F0` at `0x00729A8C` (`C7 46 18 06 00 00 00`). Entered when the unit's current altitude (`linked_obj+0xA4` Z coord) has reached or exceeded `CellClass__GetGroundHeight(current_XY)`. Simultaneously writes:
- `[param_1+0x28] = g_CurrentFrameCounter` (start frame)
- `[param_1+0x2C] = local_14` (ground altitude at landing site, used as coordinate; confusingly stored in timer field — Ghidra's decompile shows `*(int *)(param_1 + 0x2c) = local_14` where `local_14` is the Z coord at this point)
- `[param_1+0x30] = uVar5` (from `vtable[0x84]() → ftol()` = GetSpeed/TypeClass.Speed)
- `[param_1+0x34] = uVar5`
- Calls `linked_obj->vtable[0x120](0,0)` (likely Mark_All_Occupation_Bits or Unlimbo-prep)
- Calls `linked_obj->vtable[0x544](0,0)` (facing update — face destination)

**Tilt formula:**

```c
local_12c = *(int *)(param_1 + 0x30);   // total
dVar3 = 1.0;                              // default (fully tilted)
if (local_12c != 0) {
    iVar4 = *(int *)(param_1 + 0x2c);    // remaining
    if (*(int *)(param_1 + 0x24) != -1) {
        iVar8 = g_CurrentFrameCounter - *(int *)(param_1 + 0x24);
        iVar4 = (iVar8 < iVar4) ? (iVar4 - iVar8) : 0;
    }
    dVar3 = (double)(local_12c - iVar4) / (double)local_12c;  // progress [0..1]
}
tilt = (1.0 - dVar3) * (-π/2);
```

**Summary:** `tilt = (1 - progress) × (-π/2)`. At progress=0: tilt=-π/2. At progress=1: tilt=0.

`param_3` is invalidated by `*param_3 = 0xFFFFFFFF` at state-6 branch entry.

**-π/2 constant:** `_DAT_007f0c10` = bytes `18 2D 44 54 FB 21 F9 BF` = `0xBFF921FB54442D18` = -π/2. Verified: `read_memory @ 0x007F0C10` ✓

**Fields used:**
- `+0x30` = total frames (int)
- `+0x24` = start frame (int; -1 = use remaining directly)
- `+0x2C` = remaining frames (int)

---

### State 7 — Alt Landing (positive tilt, ascending-positive variant)

**Entry condition:** State 7 was NOT found written by a `MOV [REG+0x18], 7` immediate instruction in the binary. Exhaustive byte-pattern searches for `C7 46 18 07`, `C7 43 18 07`, `C7 40 18 07`, `C7 41 18 07`, `C7 47 18 07`, `C7 81 18 00 00 00 07` all returned no matches. State 7 is likely written via a register-computed value (e.g., `MOV [ESI+0x18], EAX` where EAX=7 from arithmetic), or may be a dead/unreachable state. **Confidence: LOW on entry path — writer not located.**

Functionally, per the decompile, state 7 is structurally identical to state 6 except it uses **+π/2** instead of **-π/2**:

```c
// Same timer logic as state 6 — same fields +0x24, +0x2C, +0x30
tilt = (1.0 - dVar3) * (π/2);
```

`param_3` is invalidated by `*param_3 = 0xFFFFFFFF` at state-7 branch entry (same as states 2 and 6).

**π/2 constant (state 7):** Uses `_LAB_007e281f_1` = π/2, same as state 2.

**Summary:** `tilt = (1 - progress) × π/2`. At progress=0: tilt=+π/2. At progress=1: tilt=0. This would represent a unit that starts with a positive vertical lean and tilts back to horizontal — the mirror image of state 6.

**Fields used:** Same as state 6: `+0x24`, `+0x2C`, `+0x30`.

---

## 5. Timer Fields — Elapsed/Total Summary

All animated states (2, 6, 7) use the same elapsed-time pattern:

```c
// From Turret_barrel_tilt FUN_00729B40, extracted from states 2 and 6:
total    = *(int *)(param_1 + 0x30);       // true instance+0x34 (total duration, frames)
start    = *(int *)(param_1 + 0x24);       // instance+0x28 (start frame via g_CurrentFrameCounter)
remaining= *(int *)(param_1 + 0x2c);       // instance+0x30 (countdown / remaining frames)

elapsed = total;  // default if no start frame
if (start != -1) {
    frames_since_start = g_CurrentFrameCounter - start;
    remaining = (frames_since_start < remaining) ? (remaining - frames_since_start) : 0;
}
progress_numerator = total - remaining;  // for states 2/7 this is elapsed
progress = progress_numerator / total;   // floating-point division
```

Note: the `+0x2C` in `FUN_00729b40`'s decompile (state 6) refers to `param_1 + 0x2c` where `param_1 = instance+4`, so the actual instance offset is `+0x30`. Similarly `+0x24` in decompile = instance `+0x28`.

---

## 6. State Machine Transition Map

From state-handler function analysis:

```
instance+0x18:
  0 → initial (Constructor zeroes this)
  0 or 1 → 2  (FUN_007291F0: CDTimer expires + facing aligned → takeoff)
  0 or 1 → 3  (FUN_007291F0: alternate branch → direct hover)
  3 (cruise) → 5  (FUN_00729580: distance < 0x14 to target)
  5 (hover) → 5  (continues until landing cell found)
  (descent) → 6  (FUN_007298F0: altitude >= ground_height at XY)
  6 (landing) → 0  (function near 0x00729A93: landing complete)
  state 4: read by FUN_007291D0 (In_Landing_Zone check, returns 4+param_2 if state==4)
  state 7: writer not located; formula matches state-6-mirror
```

States 1 and 4 are NOT dispatched in `FUN_00729B40` (fall to default, tilt=0).

---

## 7. Which Locomotors / Units

### TunnelLocomotionClass (DEFERRED-TS)

The state machine at `FUN_00729B40` is **TunnelLocomotionClass::Draw_Matrix** (slot 9 of vtable at `0x007F5A24`). TunnelLocomotionClass CLSID is `{4A582743-9839-11D1-B709-00A024DDAFD1}`. Zero INI references in `rules.ini` or `rulesmd.ini`. This locomotor is never instantiated in standard YR or RA2.

The state names (takeoff/hovering/descending/landing) are visually incongruous for a "tunnel" locomotor. They may represent a subterranean unit's surface-emergence animation (underground = descending, emerging = takeoff), or they may be a partial port of JumpjetLocomotionClass states that was never completed for YR.

### JumpjetLocomotionClass (ACTIVE)

JumpjetLocomotionClass uses states 0–6 at **`instance+0x50`** (not `+0x18`) and has its own ILocomotion vtable at `0x007ECD68` with different slot assignments. Its Draw_Matrix is at slot 9 = `0x0054DCC0` — a different function that handles JumpJet-specific lean. JumpjetLocomotionClass is active in YR (Rocketeer, Siege Chopper).

**The states 2–7 in `FUN_00729B40` are NOT active for Rocketeer/Siege Chopper.**

---

## 8. Key Constants

| Constant | Address | Value | Role |
|---|---|---|---|
| `π/2` double | `0x007E2820` | `0x3FF921FB54442D18` ≈ 1.5707963 | States 2, 3, 7 tilt target |
| `-π/2` double | `0x007F0C10` | `0xBFF921FB54442D18` ≈ -1.5707963 | States 5, 6 tilt target |
| `1.0` double | `_g_Const_1_0` | `0x3FF0000000000000` | Progress formula base |

---

## 9. Active in Standard YR?

**No.** `FUN_00729B40` is the Draw_Matrix (turret tilt) method of TunnelLocomotionClass, which is a DEFERRED-TS dormant locomotor with zero INI references in stock YR/RA2. States 2–7 are never entered during standard play. Full dormancy evidence in `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md` §4.

The tilt constants (π/2, -π/2) and the formula structure are correct per the decompile, but they are dead code paths for YR purposes.

---

## 10. Rust Port Implications

**Do NOT implement states 2–7 of this state machine** as part of the YR Rust port. Per `CLAUDE.md`: the parity bar applies to observable behavior; TunnelLocomotionClass is never instantiated in stock YR, so it produces zero observable behavior.

If a future mod-compatibility requirement arises for TunnelLocomotionClass, the tilt formulas are:

| State | Formula | Fields |
|---|---|---|
| 2 (takeoff) | `progress = (total - remaining) / total; tilt = progress × π/2` | `+0x24` start, `+0x2C` remaining, `+0x30` total |
| 3 (hover) | `tilt = π/2` (fixed) | none |
| 5 (descend) | `tilt = -π/2` (fixed) | none |
| 6 (landing) | `progress = (total - remaining) / total; tilt = (1 - progress) × (-π/2)` | same as state 2 |
| 7 (alt-landing) | `progress = same; tilt = (1 - progress) × π/2` | same as state 2 |

All tilt angles are applied via `Matrix_rotate_y_axis((float)tilt)` after building the facing rotation.

---

## Sources

- Ghidra decompile of `FUN_00729B40` @ `0x00729B40` (direct decompile, 2026-05-19)
- Ghidra decompile of state writers: `FUN_007291F0`, `FUN_007298F0`, `FUN_00729580`
- `read_memory @ 0x007F5A00` len 160 (vtable contents)
- `search_byte_patterns` for `C7 46 18 0X 00 00 00` patterns (state writers)
- `read_memory @ 0x007E2820` len 8 (π/2 constant verification)
- `read_memory @ 0x007F0C10` len 8 (-π/2 constant verification)
- `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md` §4 (TunnelLocomotionClass dormancy evidence)
- `VOXEL_SLOPE_TILT_SYSTEM.md` (state-0 path and calling context)
- `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` (confirmed JumpJet state is at `+0x50`, different function)
