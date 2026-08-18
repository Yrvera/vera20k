# TeleportLocomotionClass — State Machine States (enum decode)

**Kind:** enum (implicit — no Ghidra enum type defined)
**Symbol:** TeleportLocomotionClass_StateMachine_States
**Field:** TeleportLocomotionClass+0x34 (1-byte state counter)
**Range:** 0..7 (8 values)
**Active in YR:** Yes — state machine drives every teleport warp cycle

---

## Summary

The teleport locomotor's `StateMachineTick` (0x007192F0) is an 8-state machine stored
as a 1-byte counter at `TeleportLocomotionClass+0x34` (`param_1[0xd]` when `param_1`
is `int*`). States advance sequentially 0→1→2→3→4→5→6→7→0, with states 1 and 6 being
timer-wait loops and states 0 and 7 being the idle/reset states. The field is
initialized to 0 by the constructor.

Verified via `decompile_function 0x007192F0` (StateMachineTick) — each state arm
directly dispatches on `uVar3 = param_1[0xd]`. Self-proof re-verification: the
decompile shows `if (uVar3 == N)` branches for N = 0..7 with a final `LAB_00719be2`
fallthrough. No state value > 7 appears.

---

## State table

| State | Name | Entry condition | Side effects | Exit / next state |
|---|---|---|---|---|
| 0 | IDLE / CHECK_DEST | Default; reset to 0 from state 7 | See sub-branches below | → 1 (warp armed) or stays 0 |
| 1 | WARP_OUT_WAIT | Armed by state 0; timer running | Calls TimerCheck (loops per tick) | → 2 when timer expires (via TimerCheck) |
| 2 | WARP_RELOCATE | Entered from state 1 expiry | Spawns WarpOut anim at current Location; plays WarpOut sound; clears ChronoInTransit (+0x27C), BeingWarped (+0x270), bridge flag (+0x8C); calls Update_Position mode=1 (commit teleport) | → 3 (success) or → 4 (Update_Position returned non-null) |
| 3 | CHRONO_DELAY_LOAD | Entered from state 2 | Calls Update_Position; writes `Rules+0xBEC` (ChronoDelay) → TechnoClass+0x284 | → 4 (implicit after return) |
| 4 | OCCUPATION_CLEAR | Entered from state 3 | Calls Update_Position; Mark then Unmark occupation bits (`vtable+0x1B4`, `vtable+0x1CC`); calls `vtable+0x124` (unlock locomotion) | → 5 |
| 5 | POST_WARP_VALIDATE | Entered from state 4 | Checks IsInPlayfield; calls PostWarpValidation (water/impassable death check); if alive: clears kill-credit ptrs (+0x428/+0x42C), calls Resume, arms timer (g_CurrentFrameCounter, ChronoDelay from +0x284), spawns WarpIn anim at destination Location, plays WarpIn sound | → 6 (timer armed) |
| 6 | WARP_IN_WAIT | Armed by state 5; timer running | Calls TimerCheck (loops per tick); TimerCheck re-engages weapons on expiry | → 7 when timer expires (via TimerCheck resume path) |
| 7 | CLEANUP | Entered from state 6 expiry | Clears WarpAnimGate (+0x271); calls SetGhostCell; resumes locomotion (`vtable+0x480(0,0)`); clears dest_cache_1_z (+0x30); clears warp_state (TechnoClass+0x280=0); resets state to 0 | → 0 (reset) |

---

## Detailed state analysis

### State 0 — IDLE / CHECK_DEST

Verified via `decompile_function 0x007192F0` — state 0 is the default / reset state.

Three distinct sub-branches execute at state 0:

**Sub-branch A — warp_state copy** (highest priority guard):
```c
if ((uVar3 == 0) && (piVar1[0xa0] != 0)) {
    param_1[0xd] = piVar1[0xa0];  // copy TechnoClass+0x280 (warp_state) to locomotor state
    return 0;
}
```
If `TechnoClass+0x280` (warp_state) is non-zero, the locomotor adopts that value as its
state directly. This is the inter-state resume mechanism after an external interrupt.

**Sub-branch B — ChronoInTransit gate** (when TechnoClass+0x27C != 0):
```c
else if (uVar3 == 0) {  // i.e. if ChronoInTransit != 0 AND state == 0
    // arm timer, set state = 1
    *(undefined1*)(piVar1 + 0x9c) = 1;   // TechnoClass+0x270 = BeingWarped = 1
    param_1[0xe] = g_CurrentFrameCounter;  // +0x38 = timer_start = now
    param_1[0x10] = 0x3c;                  // +0x40 = timer_duration = 60 frames
    param_1[0xd] = state + 1;              // → state 1
}
```
When ChronoInTransit (`TechnoClass+0x27C`) is set, arms a 60-frame warp-out wait timer
and advances to state 1.

**Sub-branch C — normal warp dispatch** (ChronoInTransit == 0, state == 0):
Checks `Is_Moving()` and destination sentinel. If destination is valid and not equal to
current position → calls `InitiateWarp`, spawns WarpOut anim, arms timer via
InitiateWarp, transitions to state 7 (InitiateWarp side-effects then returns to tick
loop in state 0 which copies warp_state to state).

Note: the state machine manifest notes a "7-state machine (0..7)" but the decompile
shows 8 distinct arm values (0..7 inclusive). State 0 and state 7 are both active, so
total = 8 values.

### States 1 and 6 — Timer-wait loops (WARP_OUT_WAIT, WARP_IN_WAIT)

Both states call `(**(code**)(param_1[-1] + 0x28))()`. `param_1[-1]` is one `int*` unit
before the ILocomotion vtable pointer stored in `param_1[1]` — this is the IUnknown
vtable area. The slot `+0x28` on the IUnknown vtable resolves to the TimerCheck
callback. Verified: fn-timer-check.md documents 0x00719BF0 as the timer check function
called inline from states 1 and 6.

TimerCheck fires `vtable+0x484(0, 1)` (Resume) when the timer expires — this unblocks
the state machine to advance from 1→2 and 6→7 respectively.

### State 2 — WARP_RELOCATE

Key side effects (verified via decompile):
- Spawns `AnimClass` at TechnoClass+0x9C/+0xA0/+0xA4 (current Location) — WarpOut anim
  (`Rules+0x33C` anim type pointer)
- Calls `vtable+0x124()` — locomotor lock/suspend
- Plays WarpOut sound if `TechnoType+0x578 != -1` OR `Rules+0x21C != -1`
- Sets `TechnoClass+0x271 = 1` (WarpAnimGate)
- Clears `TechnoClass+0x27C = 0` (ChronoInTransit)
- Clears `TechnoClass+0x270 = 0` (BeingWarped/chrono_in_transit_gate)
- Clears `TechnoClass+0x8C = 0` (bridge flag)
- Calls `TeleportLocomotionClass__Update_Position(mode=1)` — commits teleport
- Advances: `state += 1` (→3), or if Update_Position returned non-null: `state += 2` (→4)

### State 3 — CHRONO_DELAY_LOAD

Key side effects:
- Calls `Update_Position` with current dest coords
- Writes `*(undefined4*)(TechnoClass+0x284) = *(undefined4*)(Rules+0xBEC)` — loads
  ChronoDelay (stock: 60 frames) into TechnoClass countdown field

### State 4 — OCCUPATION_CLEAR

Key side effects:
- Calls `Update_Position`
- Calls `vtable+0x1B4()` (Mark occupation bits) then `vtable+0x1CC()` (Unmark)
- Calls `vtable+0x124()` (locomotor unlock)
- Advances state to 5

### State 5 — POST_WARP_VALIDATE

Key side effects:
- Calls `vtable+0x1B8()` — Get_Cell_Packed, checks IsInPlayfield via `MapClass__Is_Cell_In_Playfield`
- If not in playfield: clears `TechnoClass+0x3D5 = 0` (YELLOW — field purpose unknown)
- If `TechnoClass+0x280 == 0`: calls `PostWarpValidation(dest_x, dest_y, dest_z)`
- If `TechnoClass+0x24 != 0` (IsDying/IsInAir flag YELLOW): arms timer and advances to 6:
  - Clears kill-credit ptrs: `TechnoClass+0x428 = 0`, `TechnoClass+0x42C = 0`
  - Calls `TechnoClass__SetGhostCell(0)`
  - Calls `vtable+0x480(0, 1)` — Resume
  - Arms timer: `param_1[0xe] = g_CurrentFrameCounter`, `param_1[0x10] = TechnoClass+0x284`
  - Spawns WarpIn anim at TechnoClass+0x9C/+0xA0/+0xA4 (Location) via `AnimClass__Constructor`
  - Advances state to 6

### State 7 — CLEANUP

Key side effects (verified via decompile):
```c
// state 7
*(undefined1*)((int)piVar1 + 0x271) = 0;  // clear WarpAnimGate
TechnoClass__SetGhostCell();
(**(code**)(*(int*)param_1[2] + 0x480))();  // vtable+0x480 = Resume (no args)
*(undefined1*)(param_1 + 0xc) = 0;          // clear dest_cache_1_z byte (+0x30)
*(undefined4*)(uVar3 + 0x280) = 0;          // TechnoClass+0x280 = 0 (warp_state)
param_1[0xd] = 0;                           // reset state to 0
```

---

## Proposed enum values

```
TeleportLocomotionClass_State_Idle              = 0
TeleportLocomotionClass_State_WarpOutWait       = 1
TeleportLocomotionClass_State_WarpRelocate      = 2
TeleportLocomotionClass_State_ChronoDelayLoad   = 3
TeleportLocomotionClass_State_OccupationClear   = 4
TeleportLocomotionClass_State_PostWarpValidate  = 5
TeleportLocomotionClass_State_WarpInWait        = 6
TeleportLocomotionClass_State_Cleanup           = 7
```

---

## Active in YR

**Yes — unconditionally.** StateMachineTick is dispatched via ILocomotion vtable slot 2
from the footclass locomotor update path. No gating flag. Every unit with
`Teleporter=yes` that initiates a warp runs through all 8 states per warp cycle.

---

## Struct field summary

| Field | Offset | Role in state machine |
|---|---|---|
| TeleportLocomotionClass+0x34 | direct byte | State value 0..7 — read and written as `param_1[0xd]` (int* arith, ×4) |
| TeleportLocomotionClass+0x38 | direct | timer_start_frame — set in states 0B, 5 |
| TeleportLocomotionClass+0x40 | direct | timer_duration_frames — set to 0x3c in state 0B, to ChronoDelay in state 5 |
| TechnoClass+0x270 | byte | BeingWarped gate — set 1 in state 0B, cleared 0 in state 2 |
| TechnoClass+0x271 | byte | WarpAnimGate — set 1 in state 2, cleared 0 in state 7 |
| TechnoClass+0x27C | byte | ChronoInTransit — checked in state 0, cleared in state 2 |
| TechnoClass+0x280 | direct | warp_state — copied to locomotor state in sub-branch A; cleared to 0 in state 7 |
| TechnoClass+0x284 | direct | ChronoDelay countdown — written from Rules+0xBEC in state 3; read as timer duration in state 5 |
| TechnoClass+0x428/+0x42C | direct | Kill-credit ptrs — cleared to 0 in state 5 |

---

## Globals consumed

| Global | Address | Value | Role |
|---|---|---|---|
| `g_CurrentFrameCounter` | (global) | current game tick | Timer arming in states 0B and 5 |
| `g_NullCoord_Teleport_X/Y/Z` | 0x00B0EBF8 | (0,0,0) | Sentinel for "no destination" check in state 0C (corrected from 0x00B0EBD8) |
| `g_RulesClass_Instance` | (global) | Rules singleton | ChronoDelay at +0xBEC; sound ids at +0x218/+0x21C; WarpOut anim at +0x33C |

---

## Out-of-scope refs

- `TeleportLocomotionClass__InitiateWarp` (0x00719400) — called in state 0C; documented in fn-initiate-warp.md
- `TeleportLocomotionClass__Update_Position` — called in states 2/3/4; documented in fn-update-position.md
- `TeleportLocomotionClass__PostWarpValidation` (0x007187A0) — called in state 5; documented in fn-post-warp-validation.md
- `TeleportLocomotionClass__TimerCheck` (0x00719BF0) — called in states 1 and 6; documented in fn-timer-check.md
- `AnimClass__Constructor`, `VocClass__PlayAt`, `CrateClass__PickupDispatch` — general infrastructure; out of scope

---

## Unverified / YELLOW

- **TechnoClass+0x270 exact name**: Written to 1 in state 0B, cleared in state 2. Called
  `BeingWarped` / `chrono_in_transit_gate` in struct decode. Field at `piVar1[0x9c]`
  accessed as `*(undefined1*)(piVar1 + 0x9c) = 1` — `piVar1` is `int*` so `piVar1 + 0x9c`
  = byte offset `0x9c × 4 = 0x270`. Confirmed offset, name YELLOW.

- **TechnoClass+0x3D5 field cleared in state 5**: `*(undefined1*)(iVar6 + 0x3d5) = 0` when
  IsInPlayfield returns false. Field name unknown. YELLOW.

- **TechnoClass+0x24 flag in state 5**: `cVar2 = (char)((int*)param_1[2])[0x24]` — `[0x24]`
  as int* = byte offset 0x90. But then cast to char and used as a gate for the "advance to
  state 6" path. This may be `IsDying`, `IsInAir`, or a chrono-complete flag. YELLOW.
  Wait — re-reading: `(int*)param_1[2]` = TechnoClass ptr (int*), `[0x24]` × 4 = 0x90.
  Field TechnoClass+0x90 purpose YELLOW.

- **`param_1[-1] + 0x28` dispatch in states 1 and 6**: Calls through IUnknown vtable
  area. The slot +0x28 resolves to TimerCheck per fn-timer-check.md caller chain
  analysis. Not independently confirmed via vtable read_memory in this session. YELLOW on
  the vtable slot mapping; HIGH on which function executes (TimerCheck observed in callee
  decompile).

- **State 3 → 4 transition**: The decompile shows state 3 returning without explicitly
  incrementing state. The increment from 3 to 4 must happen via the warp_state copy
  mechanism (sub-branch A at state 0) or via TimerCheck. Transition mechanism YELLOW.
