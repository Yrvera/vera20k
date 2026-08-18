# TeleportLocomotion::Phase0_SetWarpingOut — 0x007197d0

**Proposed Ghidra label:** TeleportLocomotion__Phase0_SetWarpingOut (existing name is authoritative — labeler skip rename, add plate comment only)

## Summary

Sets the "warping-out" state on both the owning TechnoClass and the TeleportLocomotionClass itself. Marks `TechnoClass+0x270` (BeingWarped flag = 1), stamps the current frame counter into locomotor `+0x38`, stores a passed-in tick value at `+0x3c`, initializes `+0x40` to `0x3c` (60 ticks — the WarpOut anim duration), then increments the state machine counter at `+0x34` to advance to phase 1.

This function is **effectively inlined** into `TeleportLocomotionClass__StateMachineTick` at the `state==0, HasPendingWarp` branch. No direct CALL xref was found (confirmed via `get_xrefs_to 0x007197d0`); the logic appears verbatim in StateMachineTick's decompile (confirmed via `decompile_function 0x007192f0`).

## Active in YR

**Yes.** The behavior of this function executes within `TeleportLocomotionClass__StateMachineTick` (0x007192f0), which is the locomotor's per-tick state driver. StateMachineTick is unambiguously YR-live; the `uVar3 == 0, piVar1[0x9f] != 0` branch triggers whenever a unit with a pending warp begins executing it. Verified via `decompile_function 0x007192f0` showing the inlined body.

## Decompilation Excerpt

Source: `decompile_function 0x007197d0`

```c
uint __thiscall TeleportLocomotion__Phase0_SetWarpingOut(int param_1)
{
  // param_1  = ECX = TechnoClass* (owning unit)
  // unaff_ESI = ESI = TeleportLocomotionClass* (the locomotor object)
  //   — ESI is an "unaffected" register, meaning it's set by the caller
  //     and this function reads it directly without receiving it as a named param.
  //     This is the Ghidra artifact of an inlined/optimized call.

  *(undefined1 *)(param_1 + 0x270) = 1;          // TechnoClass+0x270: BeingWarped = true
  *(undefined4 *)(unaff_ESI + 0x38) = g_CurrentFrameCounter;  // Locomotor+0x38: frame stamp
  *(undefined4 *)(unaff_ESI + 0x3c) = in_stack_00000024;      // Locomotor+0x3c: tick arg
  *(undefined4 *)(unaff_ESI + 0x40) = 0x3c;      // Locomotor+0x40: 60-tick WarpOut duration
  uVar1 = *(int *)(unaff_ESI + 0x34) + 1;        // read state counter at Locomotor+0x34
  *(uint *)(unaff_ESI + 0x34) = uVar1;            // write back incremented state
  return uVar1 & 0xffffff00;                      // return new state (high 3 bytes)
}
```

### Corresponding StateMachineTick inline (verified via `decompile_function 0x007192f0`)

In the `state==0, (char)piVar1[0x9f] != '\0'` branch of StateMachineTick:
```c
// piVar1 is int* pointing to TechnoClass; param_1 is int* pointing to locomotor
// Note: piVar1[0x9c] == byte offset 0x9c×4 = 0x270 on a byte-addressed machine
// piVar1 is int*, so (undefined1*)(piVar1 + 0x9c) = TechnoClass + 0x270
*(undefined1 *)(piVar1 + 0x9c) = 1;         // TechnoClass+0x270 = BeingWarped = 1
param_1[0xe] = g_CurrentFrameCounter;        // Locomotor+0x38 (param_1 is int*: 0xe×4)
param_1[0xf] = uStack_c;                     // Locomotor+0x3c
param_1[0x10] = 0x3c;                        // Locomotor+0x40 = 60 ticks
iVar6 = param_1[0xd];                        // Locomotor+0x34 (state counter)
param_1[0xd] = iVar6 + 1U;                   // state counter++
return iVar6 + 1U & 0xffffff00;
```

The field-by-field mapping is exact. This confirms the function at `0x007197d0` and the inlined body in StateMachineTick are the same logic.

**POINTER ARITHMETIC NOTE (CLAUDE.md):** In StateMachineTick, `param_1` is `int *` so `param_1[N]` = byte offset `N × 4`. In Phase0_SetWarpingOut, `param_1` is `int` (ECX value) and `unaff_ESI` is `int` (ESI value), so the offsets are direct byte offsets.

## Behavioral Analysis

### Trigger condition

Called (inlined) from StateMachineTick when:
1. `param_1[0xd]` (locomotor state) == 0
2. `(char)piVar1[0x9f]` (TechnoClass+`0x9c×4+3` = effectively `+0x27f` byte? — see YELLOW) != `\0`

This combination means: state is idle/ready (0), and a "pending warp" flag on the TechnoClass is set.

### Effects

1. **Sets BeingWarped** (`TechnoClass+0x270 = 1`): signals to the combat system, rendering, and other subsystems that this unit is mid-warp.
2. **Stamps frame counter** at locomotor `+0x38`: records when the warp-out began.
3. **Stores tick value at** `+0x3c`: the value comes from a stack argument in Phase0_SetWarpingOut (`in_stack_00000024`) or `uStack_c` in StateMachineTick — this is likely the per-tick elapsed time or a tick-count seed.
4. **Sets** `+0x40 = 0x3c` (60): this is the WarpOut animation duration in ticks. 60 ticks = 2 seconds at 30 Hz tick rate. Matches the observable WarpOut anim play time.
5. **Increments state counter** at `+0x34` from 0 → 1: advances the state machine to phase 1 (WarpOut anim playing phase).

### State machine role

Phase 0 is the "start-warp-out" phase. After `Phase0_SetWarpingOut` runs:
- State counter at locomotor `+0x34` is now 1
- Next tick, StateMachineTick will branch on `state == 1`
- State 1 calls `vtable+0x28` (likely `TimerCheck` or a delegate check)

## Struct Field Accesses

### TechnoClass fields (via `param_1` ECX / `piVar1` in StateMachineTick)

| TechnoClass Byte Offset | Access | Purpose |
|---|---|---|
| +0x270 | `*(undefined1 *)(param_1 + 0x270)` = 1 | BeingWarped flag: 1 = unit is currently in warp-out animation |
| +0x27f (YELLOW) | `(char)piVar1[0x9f]` trigger check | Pending-warp flag that gates entry (see Unverified) |

### TeleportLocomotionClass fields (via `unaff_ESI` / `param_1` in StateMachineTick)

`param_1` in StateMachineTick is `int *` — all indices × 4 = byte offset.

| Byte Offset | SMT Index | Written Value | Purpose |
|---|---|---|---|
| +0x34 | [0xd] | current_value + 1 | State machine counter: incremented from 0 → 1 |
| +0x38 | [0xe] | g_CurrentFrameCounter | Frame stamp: records warp-out start frame |
| +0x3c | [0xf] | uStack_c / in_stack_00000024 | Tick seed (likely per-tick elapsed or ChronoDelay tick count) |
| +0x40 | [0x10] | 0x3c (60) | WarpOut duration in ticks |

## Globals / Enums / INI Keys Referenced

| Symbol | Usage |
|---|---|
| `g_CurrentFrameCounter` | Read into locomotor `+0x38` to stamp warp-out start frame; same global used throughout locomotor |

## Out-of-Scope Refs

None — function calls no external functions (verified via `get_function_callees 0x007197d0` returning null, and confirmed from raw decompile which has only field accesses and the global read).

## Unverified (YELLOW)

- **Trigger flag at TechnoClass+0x27f**: StateMachineTick uses `(char)piVar1[0x9f]` where `piVar1` is `int *`. Byte offset = `0x9f × 4 = 0x27c`, and `(char)` picks the lowest byte = `+0x27c`. However the manifest notes `+0x27c` as `ChronoInTransit` and `+0x271` as "Warp anim gate". The precise byte within the 4-byte slot at `piVar1[0x9f]` needs verification — the cast to `char` reads byte 0 of the 4-byte word, which is offset `+0x27c` not `+0x27f`. Needs cross-check with struct-decode task.

- **`in_stack_00000024` / `uStack_c` value**: the tick seed stored at `+0x3c`. In StateMachineTick it is `uStack_c` (a local variable that has uncertain source in the decompile). In Phase0_SetWarpingOut it is `in_stack_00000024` (a stack argument at +0x24 from the call frame). The actual value in normal gameplay is not determined here — could be 0 or a pre-computed delay. Needs InitiateWarp cross-check.

- **`0x3c = 60` as WarpOut duration**: interpreted as 60 ticks = 2 seconds. This is consistent with the observable WarpOut anim but not independently verified against the WarpOut SHP frame count or a named rules key.
