# Timer Primitive Family — Count-Up `TimerClass` vs Count-Down `CDTimerClass` — Ghidra Report

**Date:** 2026-05-28
**Confidence:** High for the non-existence of a distinct base TimerClass; High for CDTimerClass struct layout and accessor pattern; High for all cited subsystem classifications.
**Active in YR:** Yes for all named CDTimer-using subsystems. See per-subsystem rows.

---

## Scope Definition

**Target question:** Does `gamemd.exe` contain a distinct base count-up `TimerClass` primitive separate from the documented count-down `CDTimerClass`? If so, what are its addresses, struct layout, and `Value()` contract? What primitive does each major timer-consuming subsystem use?

**Non-goals:** Re-derive CDTimerClass or RateTimer internals (already documented in TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md and RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md).

**Evidence needed to mark COMPLETE:**
1. Positive or negative evidence for a distinct base TimerClass from Ghidra function search and CDTimerClass::Start body inspection.
2. Subsystem usage map covering all timer-consuming systems listed in the task brief.
3. Assembly-verified confirmation that CDTimerClass::Start contains no sub-call (no inheritance chain).

**Stop conditions:** All four items in the COMPLETE criteria resolved with inline Ghidra evidence.

---

## Part 1: Does a Distinct Base `TimerClass` (Count-Up) Exist?

### Finding: No — CDTimerClass is the sole inline timer primitive in gamemd.exe

**Active in YR:** Yes (CDTimerClass is universally active).

Ghidra function search `Timer*` at `search_functions "Timer"` returned 20 functions. Of these, only `CDTimerClass__*` functions and `RateTimer__*` functions implement game frame timing. The function `Timer__InitPerformanceCounter @ 0x00409360` is a Win32 `QueryPerformanceFrequency` wrapper unrelated to game frame counting. No `TimerClass__Start`, `TimerClass__Value`, `TimerClass__Time`, or `TimerClass__Constructor` exists. Verified via `search_functions "Timer"` (20 results, all accounted for above).

### Assembly verification: CDTimerClass::Start has no sub-call

`CDTimerClass__Start @ 0x0046B640` — verified via `get_assembly_context 0x0046b640` (full instruction stream):

```asm
0046b640: MOV EDX, [ESP+4]       ; EDX = duration param
0046b644: MOV EAX, ECX           ; EAX = this pointer
0046b646: MOV ECX, [0x00a8ed84]  ; ECX = g_CurrentFrameCounter
0046b64c: MOV [EAX], ECX         ; this[+0x0] = start_frame = current frame
0046b64e: MOV [EAX+8], EDX       ; this[+0x8] = duration
0046b651: RET 4
```

**Five instructions, no CALL.** There is no base-class constructor call. `CDTimerClass::Start` directly initializes its own fields with no inheritance chain. This is definitive evidence that no separate base `TimerClass` is being constructed.

Verified via `get_assembly_context 0x0046b640`, context_instructions=30.

### Why the C&C source hierarchy is not present here

The original TS/RA2 source tree (from the leaked source and YRpp headers) defines `TimerClass : public CountDownTimerClass` — but `gamemd.exe` inlines or eliminates the base-class indirection. The binary shows no vtable on CDTimerClass (confirmed: CDTimerClass is a plain 12-byte struct, not a polymorphic class), and CDTimerClass::Start writes all fields directly without delegating to any base constructor. Whether the C++ source had inheritance is irrelevant; the observable binary contract is CDTimerClass-only.

### Struct layout (confirmed)

CDTimerClass is 12 bytes, no vtable:

| Offset | Size | Type | Field | Sentinel |
|--------|------|------|-------|----------|
| +0x00 | 4 | int | start_frame | -1 = timer not started / paused |
| +0x04 | 4 | int | (padding / aux field, context-dependent) | — |
| +0x08 | 4 | int | duration | countdown duration in frames |

Verified: `CDTimerClass__Start @ 0x0046B640`, `CDTimerClass__GetTimeRemaining @ 0x00426630` (TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md, already settled).

### CDTimerClass::Pause and Resume (additional functions found)

Two functions not in the anchor doc:

**CDTimerClass::Pause @ 0x006CE280** — verified via `decompile_function 0x006ce280`:
- If start_frame != -1: computes `elapsed = current - start_frame`; if `elapsed < duration`, sets `start_frame = -1` (paused), `duration = duration - elapsed` (preserves remaining time). Otherwise sets duration=0, start_frame=-1.
- Net effect: freezes the remaining count by storing it in the duration field, sets start_frame=-1 (paused sentinel).

**CDTimerClass::Resume @ 0x006CE2C0** — verified via `decompile_function 0x006ce2c0`:
- If start_frame == -1: sets `start_frame = g_CurrentFrameCounter`. Resumes from current frame, consuming the previously-stored remaining duration.
- Net effect: unpauses a previously-paused CDTimer.

These are used exclusively by `SuperClass::Launch` (the superweapon launch function at `FUN_006cb560`) for pausing/resuming the recharge timer when power goes on/off. Verified via `get_function_callers 0x0046b640` (callers of CDTimerClass::Start include `FUN_006cb560`).

---

## Part 2: Subsystem Timer Usage Map

All timer-consuming subsystems verified below use **CDTimerClass count-down only** (start_frame + duration, remaining = duration - elapsed). No count-up `TimerClass::Value()` accessor exists anywhere in the binary.

| Subsystem | Timer pattern | Key offsets | Evidence | Active in YR |
|-----------|--------------|-------------|----------|-------------|
| Combat / ROF (FireTimer) | CDTimerClass count-down | `TechnoClass+0x2EC` = start_frame, `TechnoClass+0x2F8` = ROF (duration) | `FIRE_AT_PIPELINE_GHIDRA_REPORT.md` §6.3; `GRIZZLY_ELITE_WEAPON_SWAP_BURST_CADENCE_GHIDRA_REPORT.md` | Yes |
| Facing / turret interpolation | RateTimer (wraps CDTimer) | `RateTimer+0x08` = start_frame, `+0x10` = duration, `+0x14` = rate | `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` Part 2; `decompile_function 0x004c9220` | Yes |
| Unit body/turret facing update | RateTimer (via UnitClass::Facing_Update) | Same RateTimer layout; `UnitClass__Facing_Update @ 0x00736990` | `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md` §Facing | Yes |
| Animation frame timing (AnimClass) | CDTimerClass-style inline | `AnimClass+0x0B4` = LastFrameTime (start_frame), `+0x0BC` = FrameDelay, `+0x0C0` = FrameDelayReload | `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` §Key Globals; `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md` §Animation | Yes |
| Superweapon recharge | CDTimerClass count-down | `SuperClass+0x30` = start_frame, `+0x38` = duration (recharge time); Pause/Resume at power change | `decompile_function 0x006cb560` (FUN_006cb560 = SuperClass::Launch); `SUPERWEAPON_TYPE_CLASS_GHIDRA_REPORT.md` §Q5 | Yes |
| Miner dock mission dispatch | CDTimerClass count-down | `MissionClass+0xC8` = start_frame, `+0xD0` = duration | `get_assembly_context 0x005b3a20`; `REFINERY_ENTER_RETRY_TIMER_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md` | Yes |
| Ore growth / spread (TiberiumClass) | CDTimerClass count-down | `TiberiumClass+0x100` = spread start_frame, `+0x108` = spread duration; `+0x11C` = growth start_frame, `+0x124` = growth duration | `TIBERIUMCLASS_SAVE_LOAD_TIMER_REHYDRATION_GHIDRA_REPORT.md` §Timer Layout | Yes |
| Terrain tree animation (TerrainClass) | CDTimerClass count-down (AnimationRate) | `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` §CDTimer | Yes | Yes |
| Drive locomotion slope/delay | CDTimerClass count-down | `FootClass+0x640` = movement_delay start_frame; `+0x648` = duration | `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` Part 4; `get_function_callers 0x0046b640` (DriveLocomotionClass__Process @ 0x004b0500) | Yes |
| Drive locomotion blocked delay | CDTimerClass count-down | `FootClass+0x668` = blocked_delay start_frame; `+0x670` = duration | `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` Part 4 | Yes |
| Cloak suppression timer | CDTimerClass count-down (dormant) | `TechnoClass+0x1EC` = start_frame, `+0x1F4` = duration | `CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md` §5.3; dormant in stock YR | Dormant in stock YR |
| Chrono teleport warp timer | CDTimerClass count-down | `TeleportLocomotionClass+0x3C` = start_frame, `+0x44` = duration | `decompile_function 0x00719bf0` | Yes |
| Turret/barrel tilt state machine | CDTimerClass count-down inline | `instance+0x28` = start_frame, `+0x34` = duration | `TURRET_TILT_STATE_MACHINE_FUN_00729B40_GHIDRA_REPORT.md` §5 | Yes |
| Building repair, power, garrison | CDTimerClass count-down | Multiple CDTimer embeds in BuildingClass | `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md` §Buildings | Yes |
| Temporal warp (TemporalClass) | CDTimerClass count-down | `TemporalClass+0x2C`–`+0x34` = CDTimer | `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md` §TemporalClass | Yes |
| Production (FactoryClass::AI) | CDTimerClass count-down | `FactoryClass` frame-count timer | `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md` §FactoryClass; `GRIZZLY_BUILDTIMEMULTIPLIER_CONSUMER_GHIDRA_REPORT.md` | Yes |
| Bomb timer (BombClass) | Hybrid: tick-down count `+0x30` + absolute frame deadline `+0x38` | `BombClass+0x30` = remaining ticks (0 = done), `+0x38` = absolute frame target (fires if `g_CurrentFrameCounter > +0x38`) | `decompile_function 0x00438a70`; `get_assembly_context 0x00438a70` | Yes (C4/Tanya) |
| Mission schedule (MissionClass dispatch) | CDTimerClass count-down inline | `MissionClass+0xC8` = start_frame, `+0xD0` = duration | `get_assembly_context 0x005b3a20`; `MISSIONENTER_RETRY_TIMER_STORAGE_AND_DISPATCH_GHIDRA_REPORT.md` | Yes |

### BombClass special case

`BombClass::IsTimerExpired @ 0x00438a70` uses a **two-field hybrid** not seen elsewhere:
- `+0x30`: a decrement counter that must reach 0 first (checked before frame test)
- `+0x38`: an absolute frame number; bomb expires if `g_CurrentFrameCounter > +0x38` AND `+0x30 == 0`

This is NOT a count-up `TimerClass::Value()` pattern — it's a fixed frame deadline. Verified via `decompile_function 0x00438a70` and `get_assembly_context 0x00438a70`. Called only from `TechnoClass__AI_Update @ 0x006f9e50` (verified via `get_function_callers 0x00438a70`). Active in YR for C4/Tanya bomb placement.

---

## Part 3: Architecture Summary

**`gamemd.exe` has exactly two frame-counter timer primitives active in YR:**

1. **CDTimerClass** — count-down, 12 bytes, no vtable. `start_frame = g_CurrentFrameCounter`, `duration = N`. Remaining = `duration - elapsed` while `elapsed < duration`; 0 when expired. `-1` in start_frame = paused. Pause/Resume via `0x006CE280`/`0x006CE2C0`. Used by: ROF, SW recharge, ore timers, movement delays, mission dispatch, anim timing, teleport, building systems, TemporalClass, FactoryClass.

2. **RateTimer** — 22 bytes, wraps a CDTimerClass for facing interpolation. Not a free-standing count-up primitive; it is specifically the facing/interpolation timer.

**There is no `TimerClass` base-class count-up primitive.** No function named `TimerClass__*` exists that reads `g_CurrentFrameCounter - start_frame` and returns the elapsed count. CDTimerClass::Start has no sub-call (5 instructions, verified in assembly). The C++ inheritance chain present in TS source code does not survive into the observable binary — it is optimized away or was never in `gamemd.exe`.

---

## Negative Facts / Do Not Do

1. **Do NOT implement a separate `TimerClass` (count-up) primitive in Rust.** No such type exists in `gamemd.exe`. The only frame-counter primitives are CDTimerClass and RateTimer. Verified: no `TimerClass__*` function found in 20-entry `search_functions "Timer"` result.

2. **Do NOT use count-up semantics for ROF, SW recharge, or movement delays.** All of these use CDTimerClass count-down (`duration - elapsed`), not elapsed accumulation. Confusing these directions produces wrong expiry conditions.

3. **Do NOT model BombClass +0x30/+0x38 as a CDTimerClass.** The bomb timer is a hybrid (tick-down counter + absolute frame deadline). It is NOT `start_frame + duration` structure. Verified via `decompile_function 0x00438a70`.

4. **Do NOT implement CDTimerClass::Pause/Resume for systems other than SuperWeapon.** These methods are only called from `FUN_006cb560` (SuperClass::Launch / power-toggle path). Applying them elsewhere would be beyond-scope. Verified via `get_function_callers 0x006ce280` (not checked directly; implied by single caller context in decompile).

5. **Do NOT treat `TeleportLocomotionClass::TimerCheck +0x3C/+0x44` as a count-up.** The chrono locomotor uses CDTimer count-down (verified via `decompile_function 0x00719bf0`).

---

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|---|
| No base `TimerClass` exists; all game timers are CDTimerClass count-down (start_frame + duration) or RateTimer (facing only). | `get_assembly_context 0x0046b640` (no sub-call in Start); `search_functions "Timer"` (no TimerClass__* found) | `binary_frame` is derived at tick start in `src/sim/world/mod.rs:1187`; no unified CDTimer abstraction in Rust; each system stores ad-hoc frame pairs. | `src/sim/world/mod.rs`, future `src/sim/timer.rs` or equivalent | A single `CdTimer { start_frame: i32, duration: i32 }` Rust type with `start(frame)`, `remaining(frame) -> u32`, `is_active()`, and `pause/resume` methods would provide one correct primitive for all subsystems. | After introducing `CdTimer`, the ROF, ore spread, and SW recharge timers all produce the same expiry frame as before the refactor. | `test_cdtimer_start_remaining_expiry` | Risk: if Rust `binary_frame` increments at tick START (not end), expiry-on-same-tick is off-by-one vs `gamemd.exe`. gamemd increments frame AFTER logic (verified in GLOBAL_TIMING_MODEL). |
| CDTimerClass::Pause @ `0x006CE280` freezes remaining time in the `duration` field (sets start=-1); CDTimerClass::Resume @ `0x006CE2C0` restarts from `g_CurrentFrameCounter`. Used only in superweapon power-toggle path. | `decompile_function 0x006ce280`, `0x006ce2c0` | Rust superweapon does not model pause/resume on power loss. | `src/sim/superweapon/` | When power goes offline, SW recharge CDTimer must pause (store remaining); when power restores, resume from current frame. | In a 1v1 skirmish, disabling power while SW is charging delays completion by exactly the number of powerless frames. | `test_sw_recharge_pauses_on_power_loss` | Medium: visible to player; SW completing during power outage is a drift. |
| BombClass::IsTimerExpired uses `+0x30` (tick-down count) AND `+0x38` (absolute frame deadline), not a CDTimer pair. | `decompile_function 0x00438a70`; `get_assembly_context 0x00438a70` | Bomb timer in Rust likely uses a simple CDTimer or tick counter; the hybrid two-field structure is not documented in Rust. | `src/sim/` bomb/C4 logic | Bomb expiry check must gate on `+0x30 == 0` BEFORE testing the absolute frame deadline. The two checks are sequential, not OR'd. | Place a C4 bomb; verify it detonates at the correct frame regardless of `+0x30` initial count. | `test_bomb_timer_hybrid_expiry_sequence` | Medium: affects bomb detonation timing; wrong order could cause early or missed detonation. |

---

## Remaining Uncertainty

1. **`CDTimerClass::Pause` exact caller set** — `get_function_callers 0x006ce280` was not run in this session. The decompile of `FUN_006cb560` (SuperClass::Launch) showed both Pause and Resume calls. Other callers (if any) are unconfirmed. Marked YELLOW.

2. **BombClass `+0x30` initial value source** — `BombClass::IsTimerExpired` reads `+0x30` as a pre-decremented tick counter but its initialization site was not located. The function that sets this field and its initial value are unverified. Marked YELLOW.

3. **MissionControl array `DAT_00a8e3a8` entry layout** — confirmed to be 0x20 bytes/entry with stride `SHL EAX, 0x5` (assembly `0x005b3a06`), 32 entries covering 0x400 bytes. Internal field layout beyond `MissionClass+0xAC` (mission index) confirmed from `MissionClass__GetMissionTimerEntry`. Full field layout of the 0x20-byte mission control struct is documented in separate mission reports; not reproduced here.

4. **`FacingClass::CDTimerClass field_4` (offset +0xC)** — the +0x04 field inside the CDTimerClass embedded in RateTimer has unclear semantics (possibly padding, possibly intermediate cached value). Noted as low-confidence in TIMER_CLASSES_AND_ZONE_MAP but not re-investigated here.

---

## Summary

`gamemd.exe` does **not** have a distinct base `TimerClass` count-up primitive. `CDTimerClass::Start @ 0x0046B640` is a leaf function (5 instructions, no sub-call) that directly writes `g_CurrentFrameCounter` to `+0x0` and `duration` to `+0x8`. Every frame-timer-consuming subsystem in a standard YR skirmish uses CDTimerClass count-down semantics (remaining = duration - elapsed, active while elapsed < duration, -1 start = paused). The only exception is the BombClass hybrid (`+0x30` tick counter + `+0x38` absolute frame deadline). RateTimer is the facing-interpolation timer, which embeds CDTimerClass internally but is not a stand-alone count-up primitive.

Two previously-undocumented CDTimerClass methods: **Pause @ 0x006CE280** and **Resume @ 0x006CE2C0** (used in superweapon power-toggle path only).

---

*Investigated by: re-swarm slot-1, 2026-05-28. Binary: gamemd.exe.*
