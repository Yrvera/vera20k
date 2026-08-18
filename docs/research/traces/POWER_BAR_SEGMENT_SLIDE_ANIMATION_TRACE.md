# Power Bar Segment Slide Animation — Trace Report

**Mechanic:** Sidebar power bar segment slide animation (priority, per-step count, direction, halt).
**Scope:** Priority order, one-segment-per-step, direction, halt condition, convergence time.
**Binary:** `gamemd.exe` — `PowerClass::AnimationTick` at `0x0063fea0` (decompiled this session).
**Rust source:** `src/sidebar/power_bar_anim.rs` — `fn tick()`, `fn step_one_segment()`.
**Date:** 2026-05-20

---

## Scenario A — Power Plant Sell (fixture)

Starting: surplus=10, output=5, drain=8, max=50, filled=23.
After sell: target_surplus=2, output=5, drain=8 (surplus shrinks by 8).

---

## Claim-by-Claim Trace

### (1) anim_tick_counter resets to TICKS_PER_STEP=9 on change detection

**Our code** (`update()`, lines 153-155):
```rust
self.animating = true;
self.anim_tick_counter = TICKS_PER_STEP;  // = 9
```
Change detected → `anim_tick_counter` set to 9. CORRECT per our implementation.

**gamemd binary:** Uses a wall-clock timer, not a fixed-tick counter. The stabilize timer
(`+0x1520`/`+0x1528`) stores `GetRadarTimer()` (= `timeGetTime() >> 4`, 16 ms buckets) as
a start timestamp, and the interval at `+0x1528` is computed by a `Math__ftol()` call on
each step (not a fixed constant). The initial interval for the stabilize timer is set after
the first step fires (not at change detection time); the flash timer interval is hardcoded
to 3 (48 ms per flash tick).

For the stabilize timer (segment slide), there is NO initial reset at change-detection time
in the binary — the timer check fires on each AI tick once `+0x1538` (animating) is set.
The interval between steps is determined by wall-clock elapsed time, not a stored fixed count.

**Verdict: UNCHECKED** — Our `anim_tick_counter = TICKS_PER_STEP` on change is a
tick-count approximation. The binary's wall-clock stabilize timer resets after each step
fires, not at change-detection time. The net wall-clock cadence may match but the reset
point differs. The exact interval value from `Math__ftol()` was not captured this session.

---

### (2) step_one_segment() moves ONLY ONE segment per fired step

**Our code** (`step_one_segment()`, lines 269-293): Each call to `step_one_segment()` adjusts
exactly one counter by ±1, then calls `clamp_segments()`. One segment per step. CORRECT for
our implementation.

**gamemd binary:** `PowerClass::AnimationTick` at `0x0063fea0` (decompiled) shows that when
a segment differs from its target, the binary:
1. Adjusts the primary counter by ±1 (one segment).
2. Immediately calls `PowerClass__Calc_Power_Distribution` a SECOND time to get updated targets.
3. Applies a **compensating ±1** to a second counter to keep the total segment count constant.

This means the binary moves **up to 2 counters per timer expiry** (one primary + one
compensating). Our `step_one_segment()` moves only ONE counter and does no compensation.

**Verdict: FAIL** — The binary adjusts 1 primary + 1 compensating counter per step.
Our code adjusts only 1 counter per step and performs no compensation.

---

### (3) Priority order: surplus → drain → output (our claim)

**Our code** (`step_one_segment()`, lines 270-288):
```
if surplus != target_surplus → move surplus
else if drain != target_drain → move drain
else if output != target_output → move output
```
Priority in our code: **surplus first, drain second, output last**.

**gamemd binary** (decompiled from `0x0063fea0`):
```c
iVar4 = *(param_1 + 0x1534);      // current drain  (+0x1534)
if (iVar4 == iStack_20) {          // target drain   (Calc param_3)
    iVar4 = *(param_1 + 0x152c);  // current surplus (+0x152C)
    if (iVar4 == iStack_14) {      // target surplus  (Calc param_1)
        // check output (+0x1530) vs iStack_10 (Calc param_2) — third
    }
}
```
Offset mapping (verified from `POWER_BAR_RENDERING.md` param-to-offset table):
- `+0x1534` = drain, `+0x152C` = surplus, `+0x1530` = output
- `iStack_20` = target drain, `iStack_14` = target surplus, `iStack_10` = target output

Binary priority: **drain first (+0x1534), then surplus (+0x152C), then output (+0x1530)**.

The doc `POWER_BAR_RENDERING.md` (Phase 2 section) also states:
> "Check order (verified from assembly at 0x640064-0x640241): drain first (+0x1534),
> then surplus (+0x152C), then output (+0x1530)."

**Verdict: FAIL** — Our priority (surplus → drain → output) is WRONG.
Binary priority is **drain → surplus → output**. The first two are swapped.

**Scenario A walkthrough with CORRECT binary priority (drain first):**
- drain=8, target_drain=8 → drain already at target, SKIP.
- surplus=10, target_surplus=2 → surplus differs, MOVE surplus (surplus: 10→9, step 1).
- Steps 2-8: surplus slides 9→8→7→6→5→4→3→2. 8 steps total.
- All counters at target → animating flag cleared.

**With our WRONG priority (surplus first, drain second):**
- Same result here because drain is already at target in this scenario.
- The priority bug manifests when BOTH drain AND surplus are off-target simultaneously.

**Scenario B (second fixture — surplus +3 AND drain +2 simultaneously):**

With CORRECT binary priority (drain first):
- Step 1: drain != target → drain moves (drain: current→current+1). 1 step.
- Step 2: drain != target → drain moves. 2 steps.
- drain reaches target after 2 steps.
- Step 3: surplus != target → surplus moves. 3 steps.
- Step 4: surplus → moves. 4 steps.
- Step 5: surplus reaches target. 5 steps total. animating=false.

With our WRONG priority (surplus first):
- Steps 1-3: surplus moves first (3 steps).
- Steps 4-5: drain moves (2 steps).
- Same total (5 steps), but ORDER differs — surplus moves before drain.

**Verdict: FAIL on priority order for any scenario where both drain and surplus differ.**

---

### (4) Direction: surplus decreases 10→2 (current > target → -1)

**Our code** (`step_one_segment()`, lines 271-275):
```rust
if self.surplus_segments < self.target_surplus {
    self.surplus_segments += 1;
} else {
    self.surplus_segments -= 1;
}
```
When `surplus_segments=10 > target_surplus=2` → decrements by 1 each step. CORRECT direction.

**gamemd binary:** When primary counter exceeds target (`iStack_20 < iVar4` for drain branch,
equivalent for surplus branch): `*(param_1 + 0x152c) = iVar4 + -1;` — decrements by 1.
When below target: `*(param_1 + 0x152c) = iVar4 + 1;` — increments by 1. Matches signed comparison.

**Verdict: PASS** — Direction logic (±1 based on sign of current−target) is correct.

---

### (5) Animation halts (animating=false) when ALL three == targets

**Our code** (`step_one_segment()`, lines 288-290): The `else` branch (all three counters
at target) sets `self.animating = false`.

**gamemd binary:** At the top of the stabilize-timer-expired block:
```c
*(param_1 + 0x1538) = 0;   // pre-clear animating flag
if (drain != target) {  *(param_1 + 0x1538) = 1; ... }
elif (surplus != target) { *(param_1 + 0x1538) = 1; ... }
elif (output != target) { *(param_1 + 0x1538) = 1; ... }
// if none matched, flag stays 0 → not animating
```
The binary pre-clears `+0x1538` to 0 then sets it to 1 only if a counter still differs.
Same logical halt condition: all-at-target → flag stays 0.

**Verdict: PASS** — Halt condition is equivalent (animating=false when all three counters
reach their targets).

---

### (6) Convergence time for Scenario A: 8 steps × ~200ms ≈ 1.6s

**Our code:** 8 segments to slide (10→2). TICKS_PER_STEP=9 at 45Hz = 200ms per step.
Total: 8 × 200ms = 1.6s wall-clock. The comment in the source is correct.

**gamemd binary:** Uses `GetRadarTimer()` (= `timeGetTime() >> 4`, 16ms buckets). The
stabilize interval is set via `Math__ftol()` after each step — the exact computed value
was not captured this session. The flash interval is 3 × 16ms = 48ms (hardcoded as `3`
radar-timer units). Whether the stabilize interval is also ≈3 radar-timer units (≈48ms)
or some other value is UNCHECKED.

The doc states "3 ticks at ~15Hz = ~200ms per step" but this refers to game logic ticks,
not GetRadarTimer buckets. The stabilize timer and flash timer are distinct timers; the
stabilize cadence is not confirmed from the decompilation in this session.

**Verdict: UNCHECKED** — Our 200ms wall-clock target is stated but the binary's stabilize
interval (from `Math__ftol()` at timer reset) was not directly verified. The flash timer
is 48ms per decrement (3 × 16ms radar buckets), confirmed. Stabilize cadence needs a
separate decompilation pass on `Math__ftol()` in context.

---

### (7) Compensation step (binary-only, not in our code)

**gamemd binary:** After adjusting the primary counter by ±1, the binary calls
`PowerClass__Calc_Power_Distribution` again and applies a compensating ±1 to one of
the other counters to prevent the total filled count from drifting. The compensation
priority when incrementing: drain → surplus → output. When decrementing: surplus → drain → output.

**Our code:** No compensation. `step_one_segment()` moves one counter and calls
`clamp_segments()` to cap values, but does NOT call `compute_targets()` again to compensate.

**Verdict: NOT-IMPLEMENTED** — The binary's compensation step (2nd Calc_Power_Distribution
call + secondary ±1 adjustment per step) is absent from our implementation.

---

## TS-vs-YR Filter

The decompiled function `PowerClass__AnimationTick` at `0x0063fea0` is guarded only by
`DAT_00884b8d` (power bar enabled flag). This guard is NOT a `SpecialFlags` gate or a
TS-era feature flag. `PowerClass` is the sidebar power bar, present and active in
standard YR skirmish. The slide animation is live in YR, NOT TS-only.

**Verdict: CONFIRMED YR-live.** No TS-legacy filter applies to this system.

---

## Summary Table

| # | Claim | Verdict | Notes |
|---|-------|---------|-------|
| 1 | anim_tick_counter=TICKS_PER_STEP on change | UNCHECKED | Binary uses wall-clock timer, not tick counter; reset point differs |
| 2 | Only ONE segment moves per step | FAIL | Binary moves 1 primary + 1 compensating per step (2 total) |
| 3 | Priority: surplus→drain→output | FAIL | Binary priority is drain→surplus→output (drain and surplus swapped) |
| 4 | Direction: signed comparison (±1) | PASS | Binary uses identical ±1 direction logic |
| 5 | Halt when all three == targets | PASS | Binary pre-clears animating then sets only on mismatch; equivalent |
| 6 | 8 steps × 200ms ≈ 1.6s convergence | UNCHECKED | Stabilize interval from Math__ftol() not captured; flash=48ms confirmed |
| 7 | Compensation step (NOT in our code) | NOT-IMPLEMENTED | Binary calls Calc_Power_Distribution twice per step; ours does not |

**PASS: 2 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1**

---

## Top 5 Most Player-Visible Failures

1. **FAIL — Priority inversion (drain vs surplus):** Binary moves drain first, then surplus. Our code does surplus first. Fires whenever power changes with both drain AND surplus off-target simultaneously (e.g., spy infiltration, selling/building a power plant while consuming buildings exist). Affects every such power-change event in a normal match.

2. **FAIL — No compensation step:** Binary applies 1 primary + 1 compensating adjustment per step to keep total filled count constant. Our code adjusts only 1 counter and clamps instead. Causes visible total-segment drift during animation (bar may momentarily show wrong total fill level). Fires on every animated step.

3. **NOT-IMPLEMENTED — Compensation priority sub-ordering:** Even if we add the compensation step, the sub-priority within compensation (drain→surplus→output when incrementing; surplus→drain→output when decrementing) is not implemented. This affects which band "floats" during transitions.

4. **UNCHECKED — Stabilize timer cadence (200ms claim):** The per-step interval comes from `Math__ftol()` in the binary, not a hardcoded tick count. If the computed value differs from our TICKS_PER_STEP=9 equivalent, every animated transition runs at the wrong speed. Fires on every power-change animation.

5. **UNCHECKED — Timer reset point:** Binary does not reset the stabilize timer at change-detection; it resets after each step fires. Our code resets `anim_tick_counter` at change-detection. If the timer was mid-interval when a change occurs, the binary fires the first step sooner than ours does. Fires at the start of every animated transition.

---

## Status

PARTIAL — Ghidra decompilation of `PowerClass::AnimationTick` (0x0063fea0) completed and
priority order confirmed. Two definitive FAILs found. `Math__ftol()` cadence for the
stabilize timer was not captured; mark items 1 and 6 UNCHECKED pending a follow-up
decompilation of the stabilize interval computation.
