# TeleportLocomotionClass__TimerCheck — 0x00719BF0

**Proposed Ghidra label:** TeleportLocomotionClass__TimerCheck (existing label is authoritative — plate comment only needed)

**Active in YR:** Yes — called from `TeleportLocomotionClass__StateMachineTick` @ 0x007192F0 (anchor function; states 1 and 6 per task description). StateMachineTick dispatched via ILocomotion vtable. No direct callers found (confirmed via get_function_callers 0x00719bf0) — dispatch is inline from StateMachineTick body.

---

## Summary

Per-tick timer check for the teleport locomotor's warp-delay wait states. Compares `g_CurrentFrameCounter` against `+0x3C` (timer start frame) and `+0x44` (duration). On expiry, clears `TechnoClass+0x271` (warp anim/shimmer gate), and if `TechnoClass+0x2B4 == 0` (no active targeting), calls `FUN_0070f770` (auto-fire wakeup, out-of-scope) and `TechnoClass__Passive_Target_Acquire` to re-engage weapons. If no target is acquired, calls `vtable+0x484(0, 1)` (Resume after warp). Also increments `+0x38` counter if it is positive.

---

## Caller chain (Active in YR: Yes)

```
TeleportLocomotionClass__StateMachineTick (0x007192F0)  [anchor; states 1 and 6]
  └─ TeleportLocomotionClass__TimerCheck (0x00719BF0)   [called per-tick during wait]
```
No function-call callers found (get_function_callers 0x00719bf0 returns null) — called inline from StateMachineTick body (not via its own vtable slot).

---

## Decompilation excerpt

Source: `decompile_function 0x00719bf0`

```c
void TeleportLocomotionClass__TimerCheck(int param_1)
{
  // param_1 = int (direct byte offsets) → TeleportLocomotionClass instance

  iVar2 = *(int *)(param_1 + 0x44);  // timer duration (frames)

  if (*(int *)(param_1 + 0x3c) != -1) {
    // Timer is active: compute elapsed frames
    iVar3 = g_CurrentFrameCounter - *(int *)(param_1 + 0x3c);  // elapsed since start
    if (iVar2 <= iVar3) goto LAB_00719c12;   // timer expired: elapsed >= duration
    iVar2 = iVar2 - iVar3;                   // remaining = duration - elapsed
  }
  if (iVar2 != 0) {
    return;  // timer still running, nothing to do
  }
  // iVar2 == 0 means duration was 0 (instant expiry) or expired via remaining==0

LAB_00719c12:  // TIMER EXPIRED
  // Clear warp anim/shimmer gate: TechnoClass+0x271 = 0
  *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x271) = 0;

  // If targeting is inactive (TechnoClass+0x2B4 == 0): re-engage weapons
  if (*(int *)(*(int *)(param_1 + 0xc) + 0x2b4) == 0) {
    FUN_0070f770();                                        // 0x0070F770: auto-fire wakeup
    cVar1 = TechnoClass__Passive_Target_Acquire();         // 0x00709480: passive target search
    if (cVar1 == '\0') {
      // No target acquired: resume locomotion
      (**(code **)(**(int **)(param_1 + 0xc) + 0x484))(0, 1);  // vtable+0x484 = Resume(0, 1)
    }
  }

  // Advance warp-count counter if positive
  if (0 < *(int *)(param_1 + 0x38)) {
    *(int *)(param_1 + 0x38) = *(int *)(param_1 + 0x38) + 1;
  }
  return;
}
```

---

## Behavioral analysis

### Timer formula

The timer uses two locomotor fields:
- `+0x3C` (timer start frame): set when the warp delay begins. Value -1 means "timer not started" — when -1, the function falls through to the `iVar2 != 0` check (duration only).
- `+0x44` (timer duration in frames): the total frames to wait.

Expiry condition:
- If `+0x3C != -1`: expired when `g_CurrentFrameCounter - +0x3C >= +0x44`.
- If `+0x3C == -1` AND `+0x44 == 0`: expired immediately (duration zero).
- If `+0x3C == -1` AND `+0x44 != 0`: timer runs on remaining duration only (no frame-start reference).

### On expiry: clear warp anim gate

`TechnoClass+0x271 = 0` — clears the warp shimmer/anim gate. Per memory note `[feedback_chrono_miner_no_arrival_shimmer]`: the WarpOut anim plays at the depart cell only; this field gates the arrival side. Clearing it signals the warp animation sequence is complete and the unit has fully materialized.

### Weapon re-engagement

If `TechnoClass+0x2B4 == 0` (targeting inactive/no current target):
1. Calls `FUN_0070f770` (0x0070F770) — out-of-scope per manifest (identity unknown, likely auto-fire wakeup or weapon reload signal).
2. Calls `TechnoClass__Passive_Target_Acquire` (0x00709480) — passive auto-target scan. Returns non-null if a target is found.
3. If no target acquired (returns null/'\0'): calls `vtable+0x484(0, 1)` — the Resume/unlock method. This re-arms the unit for normal locomotion after the warp-delay hold.

### Warp count counter (+0x38)

`+0x38` is incremented if it is positive. This is likely a teleport count or warp-cycle counter. It is initialized somewhere before the timer check runs (not visible in this function) and only advances on timer expiry — counting completed warp cycles.

### Observable consequence

From the player's perspective: after the warp delay completes, the unit becomes active again (weapons re-engage, locomotion resumes). The warp shimmer effect ends. If the unit successfully acquired a target during or after warp, it fires; otherwise it resumes movement.

---

## Struct field accesses

`param_1` is `int` (direct byte offsets, verified by `*(int *)(param_1 + N)` pattern in decompile_function 0x00719bf0).

| Field | Owner | Byte offset | Notes |
|---|---|---|---|
| `param_1 + 0x0C` | TeleportLocomotionClass | +0x0C | Owning TechnoClass pointer |
| `param_1 + 0x38` | TeleportLocomotionClass | +0x38 | Warp count / cycle counter (incremented on expiry if > 0) |
| `param_1 + 0x3C` | TeleportLocomotionClass | +0x3C | Timer start frame; -1 = not started |
| `param_1 + 0x44` | TeleportLocomotionClass | +0x44 | Timer duration in frames |
| TechnoClass+0x271 | TechnoClass | +0x271 | Warp anim/shimmer gate flag; cleared to 0 on timer expiry |
| TechnoClass+0x2B4 | TechnoClass | +0x2B4 | Targeting-active field; 0 = inactive → triggers weapon re-engage |

---

## Vtable slots resolved

| Offset | Called on | Resolved meaning | Evidence |
|---|---|---|---|
| vtable+0x484 | Self TechnoClass | `Resume()/Wake()` — re-arms unit after warp delay | Called only when no passive target found; args (0, 1) |

---

## Globals + INI keys

| Symbol | Address | Role |
|---|---|---|
| `g_CurrentFrameCounter` | (global) | Current game frame; used to compute elapsed time |
| `FUN_0070f770` | 0x0070F770 | Auto-fire wakeup (identity unconfirmed; out-of-scope per scope-explorer) |
| `TechnoClass__Passive_Target_Acquire` | 0x00709480 | Passive target scan; returns true if target acquired |

---

## Out-of-scope refs

| Symbol | Address | Reason |
|---|---|---|
| `FUN_0070f770` | 0x0070F770 | Decode task #68 (out of scope here; identity undetermined) |
| `TechnoClass__Passive_Target_Acquire` | 0x00709480 | Decode task #69 (out of scope here) |

---

## Unverified (YELLOW)

- **`+0x3C` = timer start frame, `+0x44` = timer duration**: Field names inferred from the formula `g_CurrentFrameCounter - +0x3C >= +0x44`. No independent struct layout confirmation from TeleportLocomotionClass struct decode (task #12).
- **`TechnoClass+0x2B4` = targeting-active field**: Role inferred from context (0 = fire/target re-engage). Exact field name not confirmed by TechnoClass struct decode.
- **`TechnoClass+0x271` = warp shimmer gate**: Named from memory note about WarpOut anim; confirmed cleared here but the flag's full lifecycle (set/clear across all states) not traced in this function.
- **`vtable+0x484` = Resume**: Identity inferred from call context (called when no target, after warp completes). Exact method name not directly decompiled in this session.
- **`+0x38` = warp count/cycle counter**: Incremented on expiry if > 0; its initialization site and upper-bound semantics not traced in this function.
