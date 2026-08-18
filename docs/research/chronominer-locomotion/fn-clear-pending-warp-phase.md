# TeleportLocomotionClass__ClearPendingWarpPhase — function decode

**Address:** `0x00719790`
**Kind:** function
**Proposed Ghidra label:** TeleportLocomotionClass__ClearPendingWarpPhase (existing label is authoritative — plate comment update only)

---

## Summary

Tiny 2-operation function. Spawns an anim (AnimClass constructor with a caller-supplied
AnimType pointer) then clears `TechnoClass+0x280` (WarpState) to zero. Called as the
"no-destination" cleanup path in StateMachineTick state 0 when `GetCoords()` returns
null coordinates and no warp can be initiated. The anim argument is the pending warp-out
marker anim; clearing +0x280 marks the warp as cancelled.

Verified via `decompile_function 0x00719790`.

---

## Active in YR

**Yes — live.** Reachable through the StateMachineTick state-0 `uVar3 < 1` path when
the source-vs-cached-coord check finds no valid destination. No gating flag. Fires in
a standard YR game whenever a chrono unit's warp target is vacated between warp
initiation and departure (e.g., destination cell becomes invalid between ticks).

No direct CALL xrefs (`get_xrefs_to 0x00719790` returns empty). Called via register
dispatch from StateMachineTick state-0 branch (confirmed as `*param_1+0x480` pattern —
YELLOW, see below).

---

## Decompile (complete)

```c
uint __fastcall TeleportLocomotionClass__ClearPendingWarpPhase(undefined4 param_1, undefined4 param_2)
{
    uint uVar1;
    int unaff_ESI;  // TeleportLocomotionClass* (ESI convention)

    AnimClass__Constructor(param_2);       // spawn pending warp marker anim
    uVar1 = *(uint *)(unaff_ESI + 8);     // TeleportLocomotionClass+0x08 = TechnoClass ptr
    *(undefined4 *)(uVar1 + 0x280) = 0;   // TechnoClass+0x280 = WarpState = 0
    return uVar1 & 0xffffff00;
}
```
Source: `decompile_function 0x00719790`.

- `param_2` — AnimType pointer (passed by caller; the warp-marker anim to play on cancel)
- `unaff_ESI + 8` — `TeleportLocomotionClass+0x08` = TechnoClass pointer (direct byte offset;
  ESI = `TeleportLocomotionClass*`)
- `*(uVar1 + 0x280)` — `TechnoClass+0x280` (WarpState / pending-warp counter); cleared to 0

---

## Behavioral analysis

The function cancels a pending warp by:
1. Playing the warp-marker anim (`param_2`) — this is the visual feedback that the warp was
   cancelled (the WarpOut shimmer that never completes). The specific AnimType is caller-
   supplied; in StateMachineTick context it is `g_RulesClass_Instance+0x33c` (WarpOut AnimType).
2. Clearing `TechnoClass+0x280` — prevents the state machine from fast-forwarding to a
   stale warp state on the next tick.

Observable effect: when a warp is cancelled mid-setup (no valid destination found), the
player sees the WarpOut shimmer anim at the departure cell but the unit does not teleport.

---

## Struct fields accessed

| Field | Offset | Accessed via | Name | Role |
|---|---|---|---|---|
| `TeleportLocomotionClass+0x08` | `unaff_ESI+8` | ESI+8 direct | TechnoClass ptr | Resolve TechnoClass from locomotor |
| `TechnoClass+0x280` | `uVar1+0x280` | direct byte | WarpState | Cleared to 0 (cancel warp) |

---

## Globals / enums / INI keys

None directly. The AnimType pointer in `param_2` is supplied by the caller (expected to be
`g_RulesClass_Instance+0x33c` = WarpOut AnimType based on StateMachineTick context).

---

## Out-of-scope refs

- `AnimClass__Constructor` (`0x00421ea0`) — general animation infrastructure; not teleport-specific

---

## Unverified / YELLOW

- **Caller identity**: `get_xrefs_to 0x00719790` returns no results. The call is dispatched
  via register in StateMachineTick's no-destination branch (likely `(*vtable+0x480)()` pattern
  visible in the StateMachineTick decompile). The exact dispatch mechanism (vtable slot vs.
  direct register call) is not confirmed from a separate decompile of the call site. YELLOW.

- **`param_2` AnimType identity**: Expected to be `g_RulesClass_Instance+0x33c` (WarpOut AnimType)
  based on the StateMachineTick pattern, but not directly verified from the ClearPendingWarpPhase
  decompile itself — the function is `__fastcall` so `param_2` is whatever the caller puts in EDX.
  YELLOW.
