# Power Bar Flash Phase — End-to-End Trace

**Scenario:** Player builds a Tesla Reactor (Power=200, Soviet). `theoretical_total`
rises from N to N+200. Trace the flash phase only (timing, count, blink rule).

**Date:** 2026-05-20  
**Subagent:** Slot 3 of /trace-swarm

---

## Pipeline Overview

```
TRIGGER:      theoretical_total changes (Tesla Reactor placed)
              → update_power_bar_anim() called each render frame
DETECTION:    PowerBarAnimState::update() compares cached values
FLASH START:  flashes_remaining = FLASH_COUNT; flash_tick_counter = TICKS_PER_STEP
TICK:         PowerBarAnimState::tick() decrements flash_tick_counter each game tick
BLINK QUERY:  render_power_bar() calls is_flashing() → draws frame 4 if true
SCREEN:       Frame 4 segment at empty/filled boundary blinks on/off
```

**YR liveness confirmed:** `PowerClass__AnimationTick` (0x0063fea0) is called from
`SidebarClass__Action` (0x006a7780) → `CommandBar_Dispatch` (0x006d0680). Live in
standard YR skirmish, not TS-only. No flag gate found.

---

## Stage 1 — Change Detection and Flash Initialization

**File:** `src/sidebar/power_bar_anim.rs`, lines 127–157 (`update()`)

**Our code:**
```rust
if power_output != self.cached_output
    || power_drain != self.cached_drain
    || theoretical_total != self.cached_theoretical   // ← third condition
{
    self.flashes_remaining = FLASH_COUNT;   // = 10
    self.flash_tick_counter = TICKS_PER_STEP; // = 9
    ...
}
```

**gamemd (0x0063fea0, verified via `decompile_function 0x0063fea0`):**
```c
// Change detection compares ONLY drain (+0x153c) and output (+0x1540):
iVar4 = Power_Drain();   // vtable+0x24
if (iVar4 != *(param_1 + 0x153c)) → change
iVar4 = Power_Output();  // vtable+0x20
if (iVar4 != *(param_1 + 0x1540)) → change

// On change:
*(param_1 + 0x151c) = 10;   // flash counter = 10  ✓
*(param_1 + 0x1518) = 3;    // timer interval = 3 timer-units
```

**Findings:**

| Check | Ours | gamemd | Verdict |
|-------|------|--------|---------|
| FLASH_COUNT = 10 | `FLASH_COUNT = 10` (line 17) | `*(+0x151c) = 10` (confirmed) | PASS |
| Change conditions | drain OR output OR theoretical | drain OR output ONLY | FAIL |

**FAIL detail — extra trigger condition:**
Our `update()` fires the flash on `theoretical_total` changes even when drain and output
are unchanged. gamemd's `PowerClass__AnimationTick` reads `Power_Drain()` and
`Power_Output()` from the IHouse COM interface (+0x53A8, +0x53A4) and compares them to
cached values at +0x153C and +0x1540. There is no third cached value for theoretical
total anywhere in the PowerClass layout. A change in theoretical total (e.g., selling
a power plant that had 0 operational output due to damage) that leaves drain and output
unchanged would trigger a flash in our engine but not in gamemd.

**Frequency:** This fires whenever theoretical_total changes independently of output/drain,
which happens on building placement/sale of any power producer regardless of health state.
In practice this is low-frequency but architecturally wrong.

---

## Stage 2 — Flash Tick Counter Reset Value (TICKS_PER_STEP)

**Our code:** `TICKS_PER_STEP = 9` (line 22), timer is game-tick based (45Hz).  
**Claimed equivalence:** `9 ticks × (1000ms / 45Hz) ≈ 200ms per step`  
**gamemd timer:** `GetRadarTimer()` (0x006c8c40) = `timeGetTime() >> 4` = wall-clock ms ÷ 16  
**gamemd interval:** 3 timer units → `3 × 16ms = 48ms per step`

**Verdict: FAIL**

| Metric | Ours | gamemd | Delta |
|--------|------|--------|-------|
| Timer source | game tick (45Hz) | timeGetTime() >> 4 (wall clock ÷ 16) | Different mechanism |
| Per-step interval | 9 × (1000/45) ≈ 200ms | 3 × 16 = 48ms | **4.2× too slow** |
| Total flash duration | 10 × 200ms = 2000ms | 10 × 48ms = 480ms | **4.2× too slow** |

**Root cause:** The comment in our code reads "Original game: 3 ticks at ~15Hz = ~200ms
per step." This is wrong. The original does NOT use game ticks for the flash timer.
It uses `GetRadarTimer()` = `timeGetTime() >> 4`, meaning each timer unit is 16ms, not
one game tick (which at 15Hz would be ~67ms). The interval of 3 units = 48ms, not 200ms.

**Correct TICKS_PER_STEP for 45Hz equivalent of 48ms wall-clock:**
`48ms / (1000ms/45Hz) = 48 × 45 / 1000 = 2.16 ticks`
→ `TICKS_PER_STEP` should be 2 (rounding down) for nearest equivalent.
At 2 ticks: `2 × (1000/45) ≈ 44ms` per step (8% error vs 48ms — acceptable).
At 3 ticks: `3 × (1000/45) ≈ 67ms` per step (39% error).

**Player visibility:** The flash lasts 2 seconds in our engine vs 480ms in gamemd. The
player sees a blink that persists over 4× longer than the original. This is highly
visible any time power changes.

---

## Stage 3 — Tick Simulation: is_flashing() vs Binary Blink Rule

**Our code** (`power_bar_anim.rs` lines 187–189):
```rust
pub fn is_flashing(&self) -> bool {
    self.flashes_remaining > 0 && (self.flashes_remaining % 2 == 0)
}
```

**gamemd Draw_It** (0x0063fb20, verified via `decompile_function 0x0063fb20`):
```c
if (0 < *(uint *)(this + 0x151c)) {
    uVar3 = *(uint *)(this + 0x151c) & 0x80000001;
    if ((int)uVar3 < 0) {
        uVar3 = (uVar3 - 1 | 0xfffffffe) + 1;  // handle negative (never reached for counter > 0)
    }
    if (uVar3 == 0) {  // counter is even → draw blink frame
        CC_Draw_Shape(g_PowerBarSHP, 4, ...);
        iVar5 += 3;
        iVar2 = 1;   // surplus loop starts at index 1
    }
}
```

`counter & 0x80000001`: for positive counter values, this is MSVC's codegen for
`counter % 2`. Result is 0 when even → blink draws. Equivalent to `counter % 2 == 0`.

**Blink sequence match** (both our code and gamemd, starting from 10):

| Counter | Even? | is_flashing (ours) | Draw blink (gamemd) | Match |
|---------|-------|--------------------|---------------------|-------|
| 10 | yes | true | yes | PASS |
| 9 | no | false | no | PASS |
| 8 | yes | true | yes | PASS |
| 7 | no | false | no | PASS |
| 6 | yes | true | yes | PASS |
| 5 | no | false | no | PASS |
| 4 | yes | true | yes | PASS |
| 3 | no | false | no | PASS |
| 2 | yes | true | yes | PASS |
| 1 | no | false | no | PASS |
| 0 | — | false (guard `> 0`) | no (guard `0 <`) | PASS |

**Verdict: PASS** — The blink logic (even/odd rule, start at 10 = blinking, guard at 0)
is identical between our code and gamemd.

**First-frame blink:** Counter initialized to 10 (even) in both engines → first animation
frame IS blinking. **PASS.**

---

## Stage 4 — Total Flash Duration

| Metric | Ours | gamemd | Verdict |
|--------|------|--------|---------|
| Steps | 10 | 10 | PASS |
| Per-step duration | ~200ms | 48ms | FAIL (see Stage 2) |
| Total | ~2000ms (~2s) | ~480ms (~0.48s) | FAIL |

---

## Stage 5 — First Frame Blinking

**Our code:** `update()` sets `flashes_remaining = 10` (even). `is_flashing()` checks
`flashes_remaining > 0 && flashes_remaining % 2 == 0`. At 10: `true`. **PASS.**

**gamemd:** Counter written as 10 before any tick. Draw_It checks `0 < counter` (true)
then `counter & 0x80000001 == 0` (10 & 1 = 0, true) → blink draws. **Same result.**

**Verdict: PASS**

---

## Stage 6 — Blink Placement (Adjacent Finding, In Scope)

**Our render** (`app_sidebar_build.rs` line 274):
```rust
if flashing && n_surplus > 0 {
    // draw frame 4 blink
}
```

**gamemd Draw_It** (0x0063fb20): Draws blink segment at the empty/filled boundary
**regardless of whether surplus > 0**. The blink occupies the position of the first
surplus segment even when surplus_count == 0 (e.g., power deficit).

**Verdict: FAIL**

The condition `n_surplus > 0` in our render code prevents the blink from appearing
when the player is in a power deficit (all filled segments are drain/output, no surplus).
In gamemd, the blink always draws at the top of the filled area during the flash phase,
even in deficit.

**Frequency:** Fires any time power changes while the player is in a deficit state
(drain > output). Common scenario: player builds many units before enough power plants.

---

## Summary Table

| Check | Our Value | gamemd Value | Verdict |
|-------|-----------|--------------|---------|
| FLASH_COUNT = 10 | 10 | 10 (literal at 0x0063ff3a) | PASS |
| flash_tick_counter reset value | 9 (TICKS_PER_STEP) | 3 (timer units × 16ms each) | FAIL |
| Per-step wall-clock duration | ~200ms | 48ms | FAIL |
| Total flash wall-clock duration | ~2000ms | ~480ms | FAIL |
| Blink rule (even counter = blink) | `% 2 == 0` | `& 0x80000001 == 0` | PASS |
| Counter starts at 10 (even) | yes | yes | PASS |
| First frame IS blinking | yes | yes | PASS |
| Guard: no blink at counter = 0 | `> 0` check | `0 <` check | PASS |
| Change trigger: drain changed | yes | yes | PASS |
| Change trigger: output changed | yes | yes | PASS |
| Change trigger: theoretical changed | yes (extra) | no (not present) | FAIL |
| Blink draws when surplus = 0 | no (guarded) | yes (always) | FAIL |

---

## Top 5 Player-Visible Failures

1. **Flash duration 4.2× too long** (2000ms vs 480ms): The bar blinks for ~2 seconds
   instead of ~0.5 seconds every time power changes. Highly visible every build action.

2. **Blink absent in deficit state** (n_surplus=0 guard): When drain > output, the blink
   frame never draws during flash phase. Player sees no blink when power is already short
   and they add another building. Fires any time a power-hungry unit/building is built
   during a deficit.

3. **Spurious flash on theoretical-only change**: Our engine triggers a flash when
   theoretical_total changes independently of operational output/drain. gamemd does not.
   Fires when a 0-output damaged power plant is sold (theoretical drops, operational
   output unchanged).

4. **Flash step cadence wrong**: Each blink step is ~200ms in our engine vs 48ms in
   gamemd. The blink feels sluggish rather than a quick alert flash.

5. **Timer mechanism mismatch**: We use sim tick counters; gamemd uses wall-clock
   (`timeGetTime() >> 4`). This means our flash rate is frame-rate independent but wrong
   in absolute timing; gamemd's is wall-clock correct. (Player-visible as items 1 and 4
   above, not separately observable.)

---

## Recommended Fixes (not implemented)

1. **TICKS_PER_STEP**: Change from 9 to 2 (= 44ms at 45Hz, closest to gamemd's 48ms).
   Or use a wall-clock accumulator for exact parity.
2. **Change detection**: Remove `theoretical_total` from the change-detection condition
   in `update()`. Only compare `power_output` and `power_drain` against their caches.
3. **Blink placement**: Remove the `n_surplus > 0` guard in `render_power_bar()`.
   Always draw the blink frame at the top of the filled area when flashing, even if
   surplus == 0.

---

**Report file:** `C:/Users/enok/Documents/ra2-rust-game-docs/traces/POWER_BAR_FLASH_PHASE_TRACE.md`

**PASS: 8 | FAIL: 5 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0**

**Status: COMPLETE**
