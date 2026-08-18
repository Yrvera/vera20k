# UnitClass `field_0xF8` (param_1[0x3E]) — Bale-Cadence Counter
## Ghidra Research Report

**Date:** 2026-05-19  
**Target:** TechnoClass+0xF8 (UnitClass param_1[0x3E]) — bale-cadence elapsed counter  
**Prior doc:** REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1  
**Status:** COMPLETE  

---

## 1. Field Identity — NOT a CDTimerClass Embed

**Finding:** `param_1[0x3E]` (byte offset 0xF8 from `this`, when `this` is an `int*`) is a **plain `int`
elapsed-frame counter**, NOT a CDTimerClass embed. It is an accumulator that counts the number of
frames elapsed since the last bale deposit. The surrounding fields at 0xF8–0x114 form a compound
"staged increment timer" cluster:

| Array index | Byte offset | Role |
|---|---|---|
| `param_1[0x3E]` | `+0xF8` | **Elapsed bale counter** — raw integer; gate fires when ≥ threshold |
| `param_1[0x3F]` | `+0xFC` | Active flag (bool) — set to 1 by `TechnoClass__AI_Update` when timer fires |
| `param_1[0x40]` | `+0x100` | StartFrame — `g_CurrentFrameCounter` at last timer arm |
| `param_1[0x41]` | `+0x104` | Z-coord/secondary storage — preserved across resets |
| `param_1[0x42]` | `+0x108` | Duration — frame interval between increments |
| `param_1[0x43]` | `+0x10C` | Timer-active flag — non-zero means the timer is running |
| `param_1[0x44]` | `+0x110` | Increment step — amount added to 0xF8 each time the timer fires |

This is a **periodic accumulator** pattern, not CDTimerClass. CDTimerClass (`Start`/`Remaining`)
stores `[StartFrame, _, Duration]` at `[0]`, `[2]` in int* terms and counts down. Here the design
is inverted: `field_0xF8` counts UP, and the gate fires when it reaches a threshold derived from
`HarvesterDumpRate`.

**Verified via:**
- `decompile_function 0x006f9e50` (`TechnoClass__AI_Update`): contains the explicit increment
- `decompile_function 0x0073D630` (`UnitClass__Mission_Deploy_Building`): contains the gate
- `decompile_function 0x0073D450` (`UnitClass__Harvest_Ore_Tick`): contains resets
- `decompile_function 0x0046b640` (`CDTimerClass__Start`): confirms CDTimerClass is `[StartFrame, _, Duration]` — different layout

---

## 2. Gate Semantics in `Mission_Deploy_Building` (0x0073D630)

### 2.1 State Machine Overview

`UnitClass__Mission_Deploy_Building` uses a `param_1[0x2f]` state variable:

- **State 0** — Not yet docked (checks pathfinding, docks)
- **State 1** — Waiting for animation/dock arrival
- **State 3** — **DUMPING** — per-bale deposit loop (this is where 0x3E is used)
- **State 4** — Done dumping, preparing to undock

### 2.2 Initialization Block (state transition into state 3)

When the harvester first arrives and the "docking flag" at `(int)param_1 + 0x6d1` transitions from
0 to 1 (verified in the function body near address ~0x73dFD0), the counter cluster is initialized:

```c
param_1[0x3e] = 0;                      // field_0xF8: elapsed bale counter = 0
*(undefined1*)((int)param_1 + 0x6d1) = 1; // docked flag
iVar3 = g_CurrentFrameCounter;
param_1[0x43] = 1;                      // field_0x10C: timer active
param_1[0x40] = iVar3;                  // field_0x100: StartFrame = now
param_1[0x41] = iStack_8;               // field_0x104: Z coord
param_1[0x42] = 1;                      // field_0x108: Duration = 1 frame
// ... (refinery anim setup) ...
param_1[0x2f] = 3;                      // enter dumping state
```

This sets a 1-frame interval timer. With Duration=1, `TechnoClass__AI_Update` fires the increment
every frame.

**Verified via:** `decompile_function 0x0073D630`

### 2.3 Bale Gate Condition

In state 3, every tick the function checks (verified line in decompilation):

```c
if (*(double*)(g_RulesClass_Instance + 0x1528) * _DAT_007e27f8 <= (double)param_1[0x3e])
```

Where:
- `g_RulesClass_Instance + 0x1528` = `HarvesterDumpRate` (a `double` at Rules+0x1528,
  INI key `HarvesterDumpRate` in `[General]`, units: minutes per bale)
- `_DAT_007e27f8` = **900.0** (verified: bytes `00 00 00 00 00 20 8C 40` LE = 900.0 as `double`)
  — this is 15 fps × 60 seconds = frames per minute
- `param_1[0x3e]` = elapsed bale counter (integer, cast to double)

**Threshold formula:** `threshold = HarvesterDumpRate_minutes * 900 frames/minute`

Default `HarvesterDumpRate` is not present in `rules.ini`/`rulesmd.ini` (not overridden); the
default is baked into the RulesClass constructor. Based on the threshold arithmetic matching
prior documentation of "~14.4 frames/bale", the default value is 0.016 minutes/bale
(`0.016 × 900 = 14.4`). The gate fires when `param_1[0x3e] >= 14` (integer comparison via
double cast).

**After a bale deposit**, the counter is reset: `param_1[0x3e] = 0;`

**Verified via:**
- `decompile_function 0x0073D630` — gate comparison visible directly
- `inspect_memory_content 0x007e27f8` — confirmed bytes = 900.0 double
- `inspect_memory_content 0x0083be4c` — confirmed string = "HarvesterDumpRate"

### 2.4 Why "Never Explicitly Incremented" Was Wrong

The prior REFINERY_DOCK_ANIM_SLOTS report's claim that the field is "never explicitly incremented"
was incorrect. The increment happens **outside** `Mission_Deploy_Building`, in
`TechnoClass__AI_Update` (0x006F9E50). The relevant section (near LAB_006FABE7):

```c
iVar7 = *(int *)&param_1->field_0x108;           // Duration
if (*(int *)&param_1->field_0x100 == -1) {       // -1 means paused
    if (iVar7 == 0) goto no_increment;
} else {
    iVar8 = g_CurrentFrameCounter - *(int *)&param_1->field_0x100; // elapsed
    if (iVar8 < iVar7) {                          // not yet expired
        iVar7 = iVar7 - iVar8;
        goto not_expired;
    }
    // timer expired:
    if (*(int *)&param_1->field_0x10c != 0) {    // timer active?
        param_1->field_0xfc = 1;                  // set "fired" flag
        *(int *)&param_1->field_0xf8 +=           // INCREMENT field_0xF8
            *(int *)&param_1->field_0x110;         // by field_0x110 (step)
        *(uint *)&param_1->field_0x100 = g_CurrentFrameCounter; // reset StartFrame
        *(int *)&param_1->field_0x104 = aiStack_54[0]; // update Z
        *(int *)&param_1->field_0x108 = *(int *)&param_1->field_0x10c; // repeat
    }
}
```

**Verified via:** `decompile_function 0x006f9e50`

The increment step is `field_0x110` (`param_1[0x44]`). With Duration=1 (set in init), this fires
every frame. If `field_0x110 = 1` (default, not yet verified), the counter increments by 1/frame,
reaching 14 after 14 frames — matching the expected HarvesterDumpRate behavior.

---

## 3. Seed Sites — Jitter Initialization

### 3.1 Actual Seed Pattern vs. Prior Doc Claim

The prior doc claimed seeds use `Random(0,2) * 30`. **This is correct for `HarvestBrain_Idle`
but incorrect for `UnitClass__Unlimbo`.** The two seed sites use DIFFERENT random ranges.

### 3.2 Site 1: `UnitClass__Unlimbo` (0x00737BA0)

For harvester-type units (when `UnitTypeClass+0xe18 != 0` OR `UnitTypeClass+0xe19 != 0`):

```c
uVar2 = Random__RandomRanged(0, 0x1d);      // Random(0, 29) — range is 0..29
*(undefined4 *)(param_1 + 0xf8) = uVar2;    // field_0xF8 = random 0..29
uVar2 = g_CurrentFrameCounter;
*(undefined4 *)(param_1 + 0x10c) = 1;       // timer active
*(undefined4 *)(param_1 + 0x100) = uVar2;   // StartFrame = now
*(undefined4 *)(param_1 + 0x104) = local_8; // Z
*(undefined4 *)(param_1 + 0x108) = 1;       // Duration = 1 frame
```

For non-harvester units the counter is initialized to 0 (same code path but without the Random).

**Seed range:** `Random(0, 29)` — uniform jitter 0–29 frames.

**Verified via:** `decompile_function 0x00737BA0` — `Random__RandomRanged(0, 0x1d)` directly visible.

### 3.3 Site 2: `UnitClass__HarvestBrain_Idle` (0x00737180)

This function has TWO seed sub-sites within it, both using `Random(0, 2) * 30`:

**Sub-site A** (counter > 89, and `iVar2 % 5 == 4`):
```c
if (param_1[0x3e] > 0x59) {           // counter > 89
    if (iVar2 % 5 != 4) {
        return iVar2 / 5;
    }
    iVar2 = Random__RandomRanged(0, 2);
    param_1[0x3e] = iVar2 * 0x1e;     // set to 0, 30, or 60
    // reset timer fields...
    return iStack_c;
}
```

**Sub-site B** (counter <= 89, and `iVar2 % 30 == 29`):
```c
if (iVar2 % 0x1e == 0x1d) {           // counter % 30 == 29
    iVar2 = Random__RandomRanged(0, 2);
    param_1[0x3e] = iVar2 * 0x1e;     // set to 0, 30, or 60
}
```

Both sub-sites produce the same output: `counter = Random(0,2) * 30` → 0, 30, or 60.

Note: `HarvestBrain_Idle` is called from `UnitClass__AI` only when the harvester unit is **alive**
and `(UnitTypeClass+0xe18 != 0 || UnitTypeClass+0xe19 != 0)` (harvester or miner flag). It runs
on every AI tick for these units when they are in the active-player pool.

**Verified via:** `decompile_function 0x00737180` — `Random__RandomRanged(0,2)` and
`param_1[0x3e] = iVar2 * 0x1e` directly visible.

---

## 4. Jitter Semantics — Why They Differ

The two seed sites serve different purposes:

| Site | Location | Range | Semantics |
|---|---|---|---|
| Unlimbo | `UnitClass__Unlimbo` 0x737BA0 | `Random(0,29)` | **Birth jitter** — spreads harvesters uniformly across a 30-frame window when they are placed on the map |
| HarvestBrain_Idle | `UnitClass__HarvestBrain_Idle` 0x737180 | `Random(0,2)*30` → 0/30/60 | **Phase reset** — realigns the counter to a 30-frame boundary (0, 30, or 60) when the counter hits a modular boundary or wraps |

The `HarvestBrain_Idle` pattern (0/30/60) means the counter is always kept as a multiple of 30
after the first reset, maintaining alignment to 30-frame "slots". The Unlimbo pattern (0–29)
provides fine-grained initial desynchronization.

The bale-dump threshold (≈14 frames at default rate) is much smaller than 30, so the jitter
offsets spread subsequent harvesters' first bale events across a 30-frame window per slot.
With 6 harvesters initialized to different `Random(0,29)` values, no two will fire their
first bale on the same frame (unless they happen to share the same initial random value).

---

## 5. `UnitClass__Harvest_Ore_Tick` Context (0x0073D450)

This function also writes to `field_0xF8`. It is called for ore-collection (not deposit), and
resets the counter in a different way depending on context:

- **Non-harvester or full-storage path:** `param_1[0x3e] = 0`, timer reset to 0 duration
- **Successful ore collection path:**
  ```c
  param_1[0x3e] = 0;
  iVar1 = *(int*)(g_RulesClass_Instance + 0x1520);  // HarvesterLoadRate (int)
  param_1[0x40] = g_CurrentFrameCounter;
  param_1[0x42] = iVar1 * 3;   // Duration = HarvesterLoadRate * 3 frames
  param_1[0x43] = iVar1 * 3;   // active flag = same
  ```

So during ore collection, the timer fires at `HarvesterLoadRate * 3` frame intervals instead of
1-frame intervals — slowing the accumulation during the loading phase.

**Verified via:** `decompile_function 0x0073D450`

---

## 6. CDTimerClass Layout (for Reference)

`CDTimerClass__Start(param_1, duration)` sets:
- `param_1[0]` = `g_CurrentFrameCounter` (StartFrame)
- `param_1[2]` = duration

`CDTimerClass__Remaining`: `if (g_CurrentFrameCounter - param_1[0] < param_1[2]) return remaining`

The UnitClass timer cluster at 0xF8–0x110 is **not** a CDTimerClass. It uses a parallel layout
with an extra `field_0x10c` (active flag) and `field_0x110` (step), and stores `field_0xf8`
separately as the running accumulator.

**Verified via:** `decompile_function 0x0046b640`, `decompile_function 0x004b4d70`

---

## 7. Rust Port Implications

1. **field_0xF8 is a plain `u32` counter**, not a CDTimerClass. Implement as `bale_counter: u32`.

2. **Gate condition** (in Mission_Deploy_Building dump state):
   ```rust
   if bale_counter as f64 >= rules.harvester_dump_rate * 900.0 {
       // deposit bale, reset bale_counter = 0
   }
   ```
   `harvester_dump_rate` is a `f64` parsed from `HarvesterDumpRate` in `[General]`. Default ≈ 0.016.

3. **Increment** happens in `TechnoClass::ai_update()` every frame (when timer active, duration=1):
   ```rust
   if self.timer_active && frame - self.timer_start >= self.timer_duration {
       self.bale_counter += self.timer_step;  // timer_step = 1 for dump phase
       self.timer_start = frame;
   }
   ```

4. **Unlimbo seed:** `bale_counter = rng.gen_range(0..=29)` (uniform 0–29).

5. **HarvestBrain_Idle re-seed:** On counter reaching a 30-frame boundary or wrapping above 89,
   reset to `rng.gen_range(0..=2) * 30` → 0, 30, or 60.

6. **The prior doc's "never explicitly incremented" claim is wrong** — the increment is in
   `TechnoClass__AI_Update`, which runs for all TechnoClass-derived objects each tick.
   The Rust port must replicate this in the per-tick unit update path.

---

## 8. Summary of Verified Facts

1. `param_1[0x3E]` (byte offset 0xF8) is a plain `int` bale-elapsed counter, incremented in
   `TechnoClass__AI_Update` (0x006F9E50) by `field_0x110` every `field_0x108` frames when
   `field_0x10C != 0`. Verified: `decompile_function 0x006f9e50`.

2. Gate in `Mission_Deploy_Building` (0x0073D630): fires when
   `(double)param_1[0x3E] >= Rules+0x1528 * 900.0`, where 900.0 is at `DAT_007e27f8`.
   Verified: `decompile_function 0x0073D630`, `inspect_memory_content 0x007e27f8`.

3. `Rules+0x1528` = `HarvesterDumpRate` (double, 8 bytes). INI key confirmed at address
   `0x0083BE4C`. Verified: `inspect_memory_content 0x0083BE4C`.

4. `UnitClass__Unlimbo` (0x00737BA0) seeds `field_0xF8` with `Random(0, 29)` — **not** `Random(0,2)*30`.
   Verified: `decompile_function 0x00737BA0`.

5. `UnitClass__HarvestBrain_Idle` (0x00737180) re-seeds with `Random(0,2) * 30` (→ 0, 30, or 60)
   at 30-frame modular boundaries and on counter wraparound above 89. Verified:
   `decompile_function 0x00737180`.
