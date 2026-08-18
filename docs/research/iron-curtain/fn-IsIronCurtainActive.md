# TechnoClass__IsIronCurtainActive — decode

**Address:** `0x0041bf40`  
**Class:** TechnoClass (virtual method, inherited by all Techno subclasses)  
**Runbook:** function-decode-v1  
**Decoded:** 2026-05-24

---

## Summary

Returns `true` when the Iron Curtain (or Force Shield) protection on this
object is currently active. This is the gate consulted before any damage
application that respects IC invulnerability.

Logic (plain): the unit has an "apply frame" timestamp (`+0x18c`). If that
timestamp is `-1`, the IC was never applied (return false). Otherwise,
compute elapsed = `g_CurrentFrameCounter − apply_frame`. If elapsed is less
than the stored `duration` (`+0x194`) AND `remaining = duration − elapsed`
is positive, return `true`. Otherwise return `false`.

**Active in YR: Yes.** Iron Curtain is a standard Soviet super weapon in all
YR/RA2 skirmish configurations. No TS-legacy flag gates this path.

---

## Decompilation excerpt

```c
// from decompile_function 0x0041bf40
undefined4 __fastcall TechnoClass__IsIronCurtainActive(int param_1)
{
  int iVar1;   // duration (from +0x194)
  int iVar2;   // elapsed frames

  iVar1 = *(int *)(param_1 + 0x194);           // read duration
  if (*(int *)(param_1 + 0x18c) != -1) {       // apply_frame != NEVER_APPLIED
    iVar2 = g_CurrentFrameCounter - *(int *)(param_1 + 0x18c); // elapsed
    if (iVar2 < iVar1) {                        // elapsed < duration
      iVar1 = iVar1 - iVar2;                    // remaining = duration - elapsed
      return CONCAT31((int3)((uint)iVar1 >> 8), 0 < iVar1);  // return remaining > 0
    }
    iVar1 = 0;
  }
  // apply_frame == -1, or elapsed >= duration: return false (iVar1 == 0)
  return CONCAT31((int3)((uint)iVar1 >> 8), 0 < iVar1);
}
```

`param_1` is `int` (direct byte offsets). Verified: `+0x18c` and `+0x194`
are direct byte offsets from the `this` pointer.

---

## Behavioral analysis

### Return value semantics

The return is a 4-byte value where the low byte is `0` (false) or `1` (true)
(`0 < remaining`), and the upper 3 bytes carry the remaining frame count
right-shifted by 8. In practice callers only test the low byte (bool), but
the remaining-frames information is encoded in the high bytes of the return
value and could be read by a caller that casts to `int`.

### Edge cases

| Condition | Result |
|-----------|--------|
| `apply_frame == -1` (sentinel: IC never applied) | false |
| `elapsed >= duration` (IC expired) | false |
| `duration == 0` | false (0 < 0 is false) |
| `elapsed == 0`, `duration > 0` | true |
| `remaining == 0` (boundary: duration ticked to zero exactly) | false (`0 < 0`) |

The check `elapsed < duration` is redundant with `remaining > 0` but both
must pass. The boundary case (`elapsed == duration`, i.e., `remaining == 0`)
returns false, meaning the IC effect ends at the frame where
`g_CurrentFrameCounter − apply_frame == duration`, not one frame later.

### Encoding of "never applied"

`-1` (`0xFFFFFFFF`) in `+0x18c` is the sentinel. The signed comparison
`elapsed < iVar1` would fail safely because if `apply_frame == 0xFFFFFFFF`
the outer `if` blocks the inner path. Explicit guard is necessary to avoid
wrapping arithmetic on large `g_CurrentFrameCounter` values.

### No side effects

This function is pure: it only reads struct fields and the global frame
counter. No writes, no allocations. Safe to call from any context.

---

## Struct field accesses

All fields are on the `TechnoClass` base (frame: direct byte offset from
`this` pointer; `param_1` is `int`, so offsets are direct).

| Offset | Size | Access | Semantic |
|--------|------|--------|----------|
| `+0x18c` | 4 bytes (i32) | Read | `ic_apply_frame` — game frame when IC was applied. Sentinel `-1` = never applied. |
| `+0x194` | 4 bytes (i32) | Read | `ic_duration` — total IC duration in frames, loaded from `RulesClass::IronCurtainDuration`. |

Both offsets verified via `decompile_function 0x0041bf40`.

---

## Globals referenced

| Symbol | Address | Role |
|--------|---------|------|
| `g_CurrentFrameCounter` | Unknown (referenced by name; decode-global-CurrentFrameCounter task will resolve) | Engine-wide frame counter. Compared against `ic_apply_frame` to compute elapsed. |

`g_CurrentFrameCounter` is read directly (not via a pointer indirection in
this function) — it is a global `int` incremented once per game tick.

---

## Callers

**Direct callers:** none — `get_function_callers 0x0041bf40` returned null,
consistent with vtable dispatch. Verified via `get_xrefs_to 0x0041bf40`.

**Vtable entries** (from `get_xrefs_to 0x0041bf40` — all DATA refs):

| Vtable address | Vtable owner (inferred from surrounding named slots) |
|----------------|------------------------------------------------------|
| `0x007e2404` | TechnoClass hierarchy (contains `TechnoClass__Select` at -20 bytes) |
| `0x007e401c` | TechnoClass hierarchy (same surrounding pattern) |
| `0x007e8df4` | TechnoClass hierarchy |
| `0x007eb1b8` | TechnoClass hierarchy |
| `0x007f4ac0` | TechnoClass hierarchy |
| `0x007f5dd0` | TechnoClass hierarchy |

All 6 vtables show the identical 16-byte pattern at the slot:
`[0x0070e300][0x0070e300][0x0041bf40][0x006f7970]`
(two `FUN_0070e300` entries immediately precede `IsIronCurtainActive`).
Verified via `read_memory` at each address.

The 6 vtable slots span at least InfantryClass, VehicleClass, AircraftClass,
BuildingClass, and potentially unit subclasses. Exact class-per-vtable binding
is **unverified** (class names not resolved from RTTI in this session); this
is best confirmed by the struct-decode tasks (#10, #11).

**Vtable slot index:** `IsIronCurtainActive` is at byte offset `0x404` from
`0x007e2000` (first observed vtable start candidate), giving slot index
`0x101` (257). However, the true vtable base requires RTTI confirmation
and is **unverified**. The slot offset relative to named peers is confirmed:
`IsIronCurtainActive` sits immediately after two consecutive `FUN_0070e300`
entries. Verified via `read_memory 0x007e23f0`.

---

## Callees

None (`get_function_callees 0x0041bf40` returned no callees). Reads only:
struct fields and global `g_CurrentFrameCounter`. Confirmed by decompilation.

---

## INI keys

None read in this function. Duration value originates from `RulesClass`
(decoded in decode-string-IronCurtainDuration / decode-struct-RulesClass-IC-config).

---

## Out-of-scope refs

| Symbol | Address | Reason out of scope |
|--------|---------|---------------------|
| `FUN_0070e300` | `0x0070e300` | Appears twice in vtable immediately before IsIronCurtainActive; writes to `+0x1c0`, `+0x1a8`, `+0x1ac`, `+0x1b0`. Likely IC-related state setter. Scope-explorer should evaluate. |
| `FUN_006f7970` | `0x006f7970` | Appears in vtable immediately after IsIronCurtainActive. Unknown role. |

---

## Unverified (YELLOW)

- **Exact vtable slot index** (257 / `0x101`): computed from `0x007e2404 - 0x007e2000` but the vtable base `0x007e2000` is inferred, not confirmed via RTTI. Do not rely on the absolute slot number; use the function address `0x0041bf40` as the authoritative identifier.
- **Vtable-to-class mapping:** the 6 vtable addresses (`0x007e2404`, `0x007e401c`, `0x007e8df4`, `0x007eb1b8`, `0x007f4ac0`, `0x007f5dd0`) are confirmed as vtable slots holding `0x0041bf40`, but which specific class (InfantryClass, UnitClass, etc.) owns each vtable was not resolved in this session via RTTI.
- **"Remaining frames in high bytes"** return value: the `CONCAT31` encoding suggests callers could read remaining frame count from the high bytes of the return. No callers that do this were identified. Mark unverified until a caller trace session resolves it.
