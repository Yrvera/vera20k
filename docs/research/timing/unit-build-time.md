# Unit / Building / Infantry / Aircraft Build Time

## Overview

**Player-visible effect:** queue a unit or building from the sidebar and
the cameo progress bar fills over a few seconds (cheap unit) to a minute+
(expensive building). The fill rate depends on the unit's cost, on how
many production buildings of the matching category you have (a second War
Factory makes vehicle production faster), on your current power state
(low power slows production), and on AI difficulty (Easy AI builds 20%
faster than Normal/Difficult).

**Mechanism in plain terms:** every production cameo is split into
exactly **54 discrete steps**. The engine wakes up the FactoryClass once
per game tick; when the per-step countdown reaches 0, `Progress` advances
by 1 (and the cameo fills by 1/54 of its width). The countdown duration
in ticks is computed once per recalculation as `Rate = max(1, min(255,
cost / 54))`, so total build time = `Rate × 54` ticks. The cost number
fed into that formula is the **adjusted cost** — TechnoType base cost,
multiplied by per-house build-time bonus, divided by power ratio,
optionally adjusted for the naval flag, and multiplied for each
additional factory of the matching category. Credit deduction happens
**incrementally** at each step (per-step charge = remaining-balance /
remaining-steps), not as a single lump sum upfront — so cancelling
mid-build refunds whatever you've already paid.

There is **no fractional-tick progression**. The cameo bar appears smooth
because 54 steps over ~30 seconds is fine-grained enough; internally
each step is a discrete int-bump. The 54 constant is hardcoded; no INI
key changes it.

The clock is the **master game-tick clock** — `Progress` advances when
`g_CurrentFrameCounter - timer_start >= Rate` holds. So build times
scale with the GameSpeed slider exactly like everything else.

**Notable invariant:** the **`HouseClass::GetBuildTimeBonus()` is INERT
in shipping YR** — all the per-house multiplier keys
(`BuildTimeBuildingsMult`, `BuildTimeUnitsMult`, etc.) are commented out
in `rulesmd.ini`, and the accessor returns `1.0` for every house. So
country-specific "builds X% faster" bonuses, while parseable, do not
fire in standard play. Per-type `BuildTimeMultiplier=` on TechnoType
similarly defaults to 1.0 and is rarely set.

---

## INI surface

### `rulesmd.ini` — per-`[TechnoType]` (per-unit cost / multiplier)

```ini
[MTNK]                       ; Grizzly Battle Tank
...
Cost=700
Soylent=700                  ; refund value when "sold" (not the same as Cost)
...
```

| Key | Type | Default | TechnoType byte offset | Notes |
|---|---|---|---|---|
| `Cost=` | int | `0` | (TechnoType field, read by `TechnoTypeClass::GetCost`) | Base credit cost — the **input** to the BuildStepTime formula |
| `Soylent=` | int | `0` | (TechnoType field) | Sell-refund value — not build-time relevant |
| `BuildTimeMultiplier=` | float | `1.0` | (read at `0x00714371` inside `TechnoTypeClass::ReadINI`) | Per-type build-speed override; multiplies the cost-derived rate. Rarely set in shipping YR. |
| `Owner=` | comma-list of house names | (none) | n/a | Restricts who can build this type — passability for the production check, not timing |
| `Prerequisite=` | comma-list of TechnoType names | (none) | n/a | Cameo gate, not timing |
| `TechLevel=` | int | `-1` | n/a | Cameo gate, not timing |

`TechnoTypeClass::GetCost(owner)` (vtable lookup at offset `+0x84`)
returns the cost adjusted for owner — currently a passthrough of the
raw `Cost=` value, but the virtual exists in case a subclass overrides.

### `rulesmd.ini` — `[General]` (build-rate globals)

```ini
MultipleFactory=0.8       ; Ick.  This is now a straight discount multiplier that is cumulative.
                          ; ie at .8 you get 1, .8, .64, .512 instead of 1, 1, 1.25,etc
                          ; gs factory bonus for multiples [1=full bonus, 0=no bonus] (def=1)
                          ; <--their way at 1 you get 1, 1, .5, .33, .25, etc

MaximumQueuedObjects=29
```

| Key | Type | Default | RulesClass byte offset | Notes |
|---|---|---|---|---|
| `MultipleFactory=` | float | `1.0` | `+0x57C` | Per-extra-factory **cumulative** discount multiplier. With 0.8: 1 factory = ×1.0, 2 = ×0.8, 3 = ×0.64, 4 = ×0.512 (each extra factory makes builds 20% faster). |
| `MaximumQueuedObjects=` | int | `29` | `+0xF0` | Hard cap on simultaneously-queued items per FactoryClass |
| `BuildSpeed=` | (multiplier on AI Progress headstart) | n/a | (in AI difficulty section) | Indirectly affects AI build speed; technically a difficulty knob |

Note the rules comment clarifies the meaning of `MultipleFactory=0.8`:
each additional factory **after the first** multiplies the cost (and
therefore the Rate) by `0.8`. So you cannot use this key to slow
production down by setting it > 1.0 — the code only applies the
multiplier when `multiFactoryBonus < 1.0` (a "discount").

### `rulesmd.ini` — AI difficulty sections (`[Easy]` / `[Normal]` / `[Difficult]`)

```ini
[Easy]
Groundspeed=1.0
Airspeed=1.0
BuildTime=.8                  ; AI builds 20% faster on Easy
Armor=1.2
ROF=.8
Cost=1.0
RepairDelay=.02
BuildDelay=.03
DestroyWalls=no
ContentScan=yes

[Normal]
...
BuildTime=1
...

[Difficult]
...
BuildTime=1.0
...
BuildSlowdown=yes             ; in Normal+Difficult only
```

| Key | Type | Default | Notes |
|---|---|---|---|
| `BuildTime=` | float | `1.0` | AI-only build-time multiplier — applied at AI's Progress headstart computation, not at the per-step Rate. Lower = faster. |
| `Cost=` | float | `1.0` | AI cost multiplier (separate from BuildTime — affects credit-spend, not rate) |
| `BuildSlowdown=` | bool | depends | AI-specific gating |

Read by `RulesClass::ReadDifficulty` at `0x0066d366` for the "BuildTime"
key; subsequent keys parsed in sequence.

### `rulesmd.ini` / hardcoded — `[HouseType]` (per-country build-time multipliers — INERT in YR)

Documented in the existing report as "INERT in YR (all keys commented
out, always 1.0)". The parser exists; the keys are recognized; the
values are simply not present in shipping `rulesmd.ini`. So
`HouseClass::GetBuildTimeBonus()` returns `1.0` for every house.

| Key | HouseType byte offset | Status in YR |
|---|---|---|
| `BuildTime=` (per-house, read at `HouseTypeClass::ReadINI` @ `0x00511a4e`) | (HouseType field) | **INERT** (not in shipping INI; default 1.0) |
| `BuildTimeBuildingsMult=` (read at `0x008252dc`) | (HouseType field) | **INERT** |
| `BuildTimeDefensesMult=` (read at `0x008252c4` / `0x00511ce6`) | (HouseType field) | **INERT** |
| `BuildTimeAircraftMult=` (`0x008252f4`) | (HouseType field) | **INERT** |
| `BuildTimeUnitsMult=` (`0x0082530c`) | (HouseType field) | **INERT** |
| `BuildTimeInfantryMult=` (`0x00825320`) | (HouseType field) | **INERT** |
| `BuildTimeMultiplier=` (on TechnoType, `0x00843cf0`, read at `TechnoTypeClass::ReadINI` @ `0x00714371`) | (TechnoType field) | Parser is live; rarely set in shipping data |

If a mod sets these, the build-time bonus pipeline lights up — but
parity-testing against shipping YR can assume `1.0` throughout.

### `[Recharge]` block (commented in INI, but the recharge keys are real)

```ini
; ******* Special weapon charge times *******
;[Recharge]
;NukeStrike=13
;EMPulse=5
;IonCannon=11
;FirestormDefense=4
```

These are **superweapon** recharge times, not unit-build times — owned
by [superweapon-recharge.md](superweapon-recharge.md). Listed here only
to clarify they're not part of the unit-build pipeline.

---

## Hardcoded constants

### The 54-step constant

From `FactoryClass::CalcRate` @ `0x004C9FB0`:

```c
int FactoryClass::CalcRate() {
    int totalTime = 0;
    if (Object != NULL) {
        totalTime = GetBuildStepTime();
    }
    int rate = totalTime / 54;       // 54 hardcoded
    rate = clamp(rate, 1, 255);
    return rate;
}
```

**54 (= `0x36`) production steps per cameo.** Not in INI. Per the
existing report:

> Total production time = `Rate × 54` frames.
> At 15 FPS game speed: a unit costing 1000 credits with no modifiers:
> - `GetBuildStepTime() ≈ 1000` (base cost)
> - `Rate = 1000 / 54 ≈ 18` frames per step
> - Total = `18 × 54 = 972` frames ≈ 64.8 seconds

So at GameSpeed=Medium (≈20 ticks/sec) a 1000-credit unit takes ~972
ticks ≈ 49 s. At GameSpeed=Slowest (≈10 ticks/sec) it takes ~97 s. At
GameSpeed=Fastest (uncapped, ~60 ticks/sec ceiling) it takes ~16 s.

The clamp to `[1, 255]` per-step has two implications:
- A unit costing < 54 credits effectively becomes `Rate = 1` (one tick
  per step → 54 ticks total = ~2.7 s at Medium). So no unit can build
  faster than 54 ticks.
- A unit with adjusted cost > 13,770 credits hits the upper clamp at
  `Rate = 255` (255 × 54 = 13,770 ticks ≈ 11.5 min at Medium). This is
  the hard upper bound on build time.

### The `GetBuildStepTime` pipeline

From `FactoryClass::GetBuildStepTime` @ `0x006F47A0` (decompiled in
[FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md](../FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md)):

```c
int FactoryClass::GetBuildStepTime() {
    TechnoTypeClass* type = Object->GetTypeClass();              // vtable+0x88

    // 1. Base cost
    int baseCost = type->GetCost(Owner);                          // vtable+0x84

    // 2. House build-time bonus (INERT in shipping YR — always 1.0)
    float bonus = HouseClass::GetBuildTimeBonus(Owner);
    int adjustedCost = ftol(baseCost * bonus);

    // 3. Power ratio penalty (low power → slow production)
    float powerRatio = HouseClass::GetPowerRatio(Owner);          // [0.0..1.0]
    adjustedCost = ftol(adjustedCost / powerRatio);                // divide → lower ratio = higher cost = slower

    // 4. Building-specific naval flag check (RTTI == 1)
    if (rtti == 1) {                                              // BuildingClass
        // Read byte at TechnoTypeClass+0xCCE (IsNaval flag)
        // Apply modifier (exact effect unclear — likely scales for naval-yard production)
    }

    // 5. Multiple-factory bonus (cumulative discount)
    float multiFactoryBonus = *(float*)(g_RulesClass_Instance + 0x57C);
    if (multiFactoryBonus < 1.0f) {                               // discount only
        int factoryCount = HouseClass::GetFactoryCount(Owner);
        int extraFactories = factoryCount - 1;
        while (extraFactories > 0) {
            adjustedCost = ftol(adjustedCost * multiFactoryBonus);
            extraFactories--;
        }
    }

    // 6. Special unit case
    if (rtti == 6 && *(byte*)(Owner[0x148] + 0x1571) != 0) {
        adjustedCost = ftol(adjustedCost * someModifier);
    }

    return adjustedCost;
}
```

The output is in **adjusted credits**, which is then `/ 54` to produce
ticks-per-step.

**Confidence: HIGH** for steps 1, 2, 3, 5 (verified in the existing
report). **MEDIUM** for steps 4 and 6 (the exact modifier semantics
were not extracted in the original disassembly — flagged for follow-up
in [FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md](../FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md)).

### Per-step cost deduction formula

From `FactoryClass::AI` @ `0x004C9B20`:

```c
int stepsLeft = 54 - Progress;
int costThisStep;
if (Object == NULL) {
    costThisStep = 0;
} else if (stepsLeft == 0) {
    costThisStep = Balance;        // final step pays the rounding remainder
} else {
    costThisStep = Balance / stepsLeft;
}
costThisStep = min(costThisStep, Balance);

if (Owner->GetAvailableCredits() < costThisStep) {
    NoFunds = true;
    Progress -= 1;                  // stall: net 0 progress this tick
} else {
    Owner->SpendMoney(costThisStep);
    NoFunds = false;
    Balance -= costThisStep;
}

if (Progress == 54) {
    IsSuspended = true;
    Rate = 0;
    Owner->SpendMoney(Balance);     // pay leftover from integer rounding
    Balance = 0;
}
```

**Per-step charge = remaining-cost / remaining-steps**. So the cost is
deducted incrementally as the cameo fills. If the house can't afford a
step, `Progress` is decremented (rolling back to "not advanced this
tick") and `NoFunds = true` is set. The next tick re-attempts. So
**insufficient funds = production stalls, doesn't lose progress**.

### Insufficient-funds stall behavior

Per the formula above, when `available < costThisStep`:
- `Progress` is bumped to `Progress + 1` then immediately decremented
  back to `Progress + 1 - 1 = Progress` (net zero advance)
- `NoFunds = true` flags the sidebar to show "low funds" warning
- Timer resets normally, so next tick re-evaluates

This means **a poor player whose income matches outflow will see
production crawl** — every step that exceeds available credits stalls
until income arrives. The cameo bar just sits at the same fill level.

### AI Progress headstart

From `HouseClass::Begin_Production` (per the existing report § 5):

```c
if (!wasSuspended && isMultiplayer && house->IsAI) {
    int headstart = (buildSpeed * multiplier / 60) * 54 / cost;
    if (headstart > 53) headstart = 53;       // cap one step short of done
    factory->Progress = headstart;
}
```

The AI does **not** get a faster step rate — it gets an instant
Progress bump at the moment production begins. `buildSpeed` and
`multiplier` come from the difficulty section's `BuildSpeed=` and
`BuildTime=` values. So Easy AI starts each production cycle with a
significant Progress fraction already filled in.

**Player-visible effect:** AI production looks the same fill-speed-wise
on the cameo (which the player never sees anyway since AI cameos aren't
visible), but the **wall-clock arrival rate of AI units** is faster
than what their per-step Rate suggests. This is why Easy AI can field
a comparable army to Normal AI despite the lower difficulty knobs.

### FactoryClass per-tick AI

From `FactoryClass::AI` @ `0x004C9B20`:

```c
if (IsSuspended) return;
if (Object == NULL && SpecialItem == 0) return;
if (Progress == 54) return;

int timeRemaining = CDTimer.GetTimeRemaining();
if (timeRemaining != 0 || Rate == 0) {
    HasTicked = false;
    return;
}

// Step expired:
Progress += StepIncrement;     // always +1
HasTicked = true;
Timer.Start = g_CurrentFrameCounter;
Timer.Duration = Rate;
IsDifferent = true;             // signal sidebar to redraw

// ... per-step credit deduction (above)
// ... completion check (above)
```

Called via `vtable + 0x5C` slot from `LogicClass::PerTickUpdate`'s
per-entity iteration over `g_FactoryClass_Array`. Per
[logic-vs-render-loop.md](logic-vs-render-loop.md): this loop runs
**unconditionally during pause** — so production continues to advance
through the menu pause. Confirmed against the partial-pause model.

### FactoryClass field layout (timing-relevant subset)

From [BUILD_QUEUE_GHIDRA_REPORT.md](../BUILD_QUEUE_GHIDRA_REPORT.md):

| Byte offset | Field | Type | Purpose |
|---|---|---|---|
| `+0x24` | `Progress` | int | Stage counter 0..54 |
| `+0x28` | `HasTicked` | bool | Set true when Progress advanced this frame |
| `+0x2C` | `Timer.Start` | int | `g_CurrentFrameCounter` snapshot at last step |
| `+0x34` | `Timer.Duration` | int | Countdown initial value (= Rate) |
| `+0x38` | `Rate` | int | Frames per production step, `clamp(cost/54, 1, 255)` |
| `+0x3C` | `StepIncrement` | int | Always 1 |
| `+0x44` | `QueueArray` | ptr | Array of `TechnoTypeClass*` (queued items) |
| `+0x50` | `QueueCount` | int | Number of items queued |
| `+0x58` | `Object` | ptr | TechnoClass being produced (NULL if none) |
| `+0x5C` | `NoFunds` | bool | True if stalled due to insufficient funds |
| `+0x5D` | `IsDifferent` | bool | "Changed" flag, read+reset by sidebar |
| `+0x60` | `Balance` | int | Remaining cost to deduct |
| `+0x64` | `OrigBalance` | int | Cost at production start |
| `+0x68` | `SpecialItem` | int | Building heap index (AI building queue) |
| `+0x6C` | `Owner` | ptr | HouseClass* owner |
| `+0x70` | `IsSuspended` | bool | True if paused / completed |
| `+0x71` | `CanAfford` | bool | Stored when suspending |

Total size: 0x74 (116 bytes).

### HouseClass factory pointers

| HouseClass byte offset | Field | RTTI Match |
|---|---|---|
| `+0x53AC` | InfantryFactory | RTTI 2 / 3 (Infantry) |
| `+0x53B0` | AircraftFactory | RTTI 0x0F / 0x10 (Aircraft) |
| `+0x53B4` | BuildingFactory | RTTI 1 / 0x28 (Building, non-naval) |
| `+0x53B8` | NavalBuildFactory | RTTI 1 / 0x28 (Building, naval) |
| `+0x53BC` | VehicleFactory | RTTI 6 / 7 (Unit, non-naval) |
| `+0x53CC` | NavalFactory | RTTI 6 / 7 (Unit, naval) |

So each house owns **six** independent FactoryClass instances (one per
category), each with its own Rate, Progress, and queue. The categories
operate in parallel — you can build a building, a vehicle, an
infantry, an aircraft, a naval unit, and a naval building all at the
same time.

### `MaximumQueuedObjects = 29`

`rules` global at `RulesClass + 0xF0`. Per FactoryClass, the queue is
capped at 29. So **theoretical max in-flight items per house = 1
producing + 29 queued = 30, × 6 factories = 180 items**. In practice
players queue 5–10 deep at most.

### `MultipleFactory = 0.8`

`rules` global at `RulesClass + 0x57C`. Per-extra-factory **cumulative**
multiplier applied to the cost in `GetBuildStepTime`:

| Factories | Effective cost multiplier | Build time |
|---|---|---|
| 1 | ×1.000 | base |
| 2 | ×0.800 | 80% of base |
| 3 | ×0.640 | 64% of base |
| 4 | ×0.512 | 51.2% of base |
| 5 | ×0.410 | 41% of base |

So a 4th War Factory makes vehicle production almost twice as fast as
a single War Factory. This is **the** scaling mechanism for late-game
production rate.

### `RecalcAllRates` triggered on factory count change

From `FactoryClass::RecalcAllRates` @ `0x004CA6E0`: when a factory is
constructed or destroyed, iterates all `g_FactoryClass_Array` and
recomputes each owned-by-this-house factory's `Rate`. So the
multi-factory discount applies immediately to **existing in-progress
production**, not just to newly-queued items.

### Refund-on-cancel formula

From `FactoryClass::AbandonProduction` @ `0x004C9FF0`:

```c
int fullCost = type->GetCost(Owner);
int alreadyPaid = fullCost - Balance;
HouseClass::Add_Credits(alreadyPaid);
Balance = 0;
```

**Refund = original cost - remaining balance = full refund of what
was already spent.** Per the existing report § Refund Formula: this is
verified-from-binary behavior — the player gets back every credit
they've put in. No cancellation penalty.

### Production timer reset patterns

- **SetRate (resume)** @ `0x004C9EA0`: `Timer.Duration = Rate`,
  `Timer.Start = g_CurrentFrameCounter`, `Timer.TimeLeft = Rate`
- **Suspend** @ `0x004C9E60`: `Timer.Duration = 0`, `Timer.Start =
  g_CurrentFrameCounter`, `Timer.TimeLeft = 0` (timer effectively
  cleared)
- **AbandonProduction**: clears timer (Duration=0), Progress=0,
  IsSuspended=true
- **CompletedProduction**: clears timer, Progress=0, IsSuspended=true

### `IsDifferent` flag and sidebar redraw cadence

The sidebar polls `FactoryClass::HasChanged()` once per render frame
(per [BUILD_QUEUE_GHIDRA_REPORT.md](../BUILD_QUEUE_GHIDRA_REPORT.md)
§ 6). `HasChanged()` is a **read-and-reset** function — calling it
returns the current `IsDifferent` value and clears it to false. So the
sidebar only redraws the cameo when state has *changed* this frame,
not every frame. This keeps the sidebar render efficient even with
many factories active.

---

## Tick / frame topology

| Stage | Clock | Where |
|---|---|---|
| `FactoryClass::AI` invocation | game-tick | `LogicClass::PerTickUpdate` per-entity loop (vtable `+0x5C`) |
| Per-step countdown | game-tick (CDTimerClass) | `Timer.GetTimeRemaining()` reads `g_CurrentFrameCounter - Timer.Start` |
| `Progress` advance | game-tick | once per step expiry |
| Per-step credit deduction | game-tick | inside the same AI call |
| Cameo redraw | render-frame (gated by `IsDifferent`) | `SidebarClass::DrawCameo` polls `HasChanged()` per render |
| Cost recalculation on factory count change | event-driven | `RecalcAllRates` triggered when a factory is added/destroyed |
| AI Progress headstart | once at production start | `HouseClass::Begin_Production` |

### Clock binding

All build-time progress is on the **master game-tick clock**. Wall-clock
build time therefore scales linearly with the GameSpeed slider:

| GameSpeed | Approx ticks/sec | 1000-credit unit (Rate=18, 972 ticks) |
|---|---|---|
| 0 (Fastest) | uncapped (~60) | ~16 s |
| 3 (Medium, SP default) | ~20 | ~49 s |
| 4 (Slow) | ~15 | ~65 s |
| 6 (Slowest) | ~10 | ~97 s |

Power-ratio penalty applies as a *cost* multiplier (cost ÷ ratio), so a
house at 50% power produces at half speed.

### Per-tick AI flow

```
LogicClass::PerTickUpdate (every tick)
├── ... (other entity iterations)
├── per-FactoryClass loop (g_FactoryClass_Array)
│   for each factory:
│     if (Owner != null && !IsSuspended && Object != null && Progress < 54):
│       if (CDTimer expired):
│         Progress += 1
│         Timer.Start = g_CurrentFrameCounter
│         Timer.Duration = Rate
│         IsDifferent = true
│         costThisStep = Balance / (54 - Progress)
│         if (Owner.credits >= costThisStep):
│           Owner.SpendMoney(costThisStep)
│           Balance -= costThisStep
│         else:
│           NoFunds = true
│           Progress -= 1     (stall — net 0)
│         if (Progress == 54):
│           IsSuspended = true
│           Rate = 0
│           Owner.SpendMoney(Balance)   (pay rounding remainder)
│           Balance = 0
└── ... (other entity iterations)
```

### Completion → delivery

After `Progress == 54`:

- **Infantry / Aircraft:** `StripClass::AI` auto-creates network
  command `0x0B` (Place_Production) with auto-calculated exit
  coordinates next frame. Unit exits the factory building.
- **Buildings:** `StripClass::AI` detects completion; player clicks to
  place. Network command `0x0B` with chosen coordinates.
- **Vehicles:** `StripClass::AI` writes the completed vehicle into the pending
  land/naval delivery global via `FUN_00734250`; delivery is then committed by
  the vehicle placement/delivery flow and `HouseClass::Place_Production`. A
  successful `ExitObject` calls `CompletedProduction` and restarts the next
  queued item in the same command. A blocked stock war-factory `ExitCoord`
  unlimbo returns failure before `CompletedProduction`/`FUN_004FAA10`, so the
  completed vehicle remains pending and the queue does not advance.

Detailed delivery / exit-from-factory pathfinding is owned by a future
production-delivery doc.

### Per-render cameo state

`StripClass::AI` (sidebar) polls `FactoryClass::HasChanged()`. When it
returns true, the sidebar:
1. Redraws the cameo progress bar to current `Progress / 54`
2. Updates the "ready" / "low power" / "low funds" overlay
3. Plays the EVA "construction complete" voice if `Progress == 54`

---

## Multipliers and modifiers

### `Cost=` (per-TechnoType)

The primary input. Higher Cost → longer build time. Range in shipping
YR: ~75 (Conscript) to ~3000+ (Apocalypse Tank, Aircraft Carrier).

### `BuildTimeMultiplier=` (per-TechnoType)

Multiplies the cost-derived rate. Default 1.0. Rarely set in shipping
YR.

### `HouseClass::GetBuildTimeBonus()` (per-house, **INERT in YR**)

Returns 1.0 for all shipping houses. The `BuildTime*Mult` family of
HouseType keys exists in the parser but is not populated in
`rulesmd.ini`.

### `HouseClass::GetPowerRatio()` (per-house)

Returns `power_output / power_drain`, capped at `[0..1.0]`. When a
house is at low power (< 100%), `GetBuildStepTime` divides by this
ratio → cost goes up → Rate goes up → production slows. At 0 power
production effectively halts (division by ~0 hits the upper Rate clamp
of 255 ticks/step = 11.5 min total at Medium). Cross-ref
[power-state-machine.md](power-state-machine.md).

### `[General] MultipleFactory=0.8`

Per-extra-factory cumulative discount. See "Hardcoded constants" table
above. Applied in `GetBuildStepTime` step 5.

### AI difficulty `BuildTime=` (per-difficulty)

Easy = 0.8 (AI builds 20% faster), Normal = 1.0, Difficult = 1.0.
**Applied at AI Progress headstart**, not at per-step Rate. So Easy AI
gets a bigger headstart on every production cycle.

### `Owner` and `Prerequisite` and `TechLevel`

Gate which units can be built at all. If gates fail, the cameo doesn't
appear in the sidebar. Not timing.

### Naval flag

For buildings with `IsNaval=yes`: routes production through
`NavalBuildFactory` (HouseClass `+0x53B8`) instead of `BuildingFactory`
(`+0x53B4`). For vehicles with `IsNaval=yes`: routes through
`NavalFactory` (`+0x53CC`) instead of `VehicleFactory` (`+0x53BC`).
The two categories have **independent queues and independent multi-factory
counts** — a 2nd Naval Yard doesn't speed up production at a War Factory.

### Special modifier at `Owner[0x148] + 0x1571` for vehicles

Per `GetBuildStepTime` step 6: when this byte is set on the owner's
sub-struct, an additional multiplier applies to vehicle production.
Likely a "production-speed crate" or "upgrade-aura" flag (Battle Lab
adjacency?). Semantics not yet identified — flagged for follow-up.

---

## Edge cases

### Pause behavior

Per [logic-vs-render-loop.md](logic-vs-render-loop.md):
`LogicClass::PerTickUpdate` runs unconditionally during pause. Therefore
`FactoryClass::AI` advances Progress and deducts credits during the
in-game menu pause. **Player-visible effect:** open the in-game menu
for 30 seconds and your production completes / advances meaningfully
during that time. This is *faithful gamemd behavior* — RA2 has always
worked this way; the in-game menu does not pause the world for the
local player in this particular sense (though Mission_Attack and unit
input do pause).

### Save / load mid-build

Save state includes `Progress`, `Timer.Start`, `Timer.Duration`,
`Rate`, `Balance`, `OrigBalance`, `IsSuspended`, `IsManual`,
`SpecialItem`, `Owner`, and the `Object` pointer. On load, the factory
resumes mid-build at the exact stage. Since `g_CurrentFrameCounter` is
also saved, the `Timer.Start` reference frame stays valid.

### Replay determinism

The per-step Rate is purely arithmetic — no RNG involved. AI headstart
uses the deterministic Random and per-house difficulty values. So
production timing is bit-identical across replay and across MP peers.

### Cancel mid-build

`AbandonProduction` refunds `OrigCost - Balance = AlreadyPaid` to the
owner and destroys the Object. Progress, Timer, IsSuspended all reset.
Next item in queue is **not** automatically started — that requires a
separate `Begin_Production` event or `StartNextQueued` trigger (per
[BUILD_QUEUE_GHIDRA_REPORT.md](../BUILD_QUEUE_GHIDRA_REPORT.md) § 7).

### Queue priority and order

Strict FIFO. `QueueArray[0]` is next; new items append at
`QueueArray[QueueCount]`. No priority, no insertion. Player must
cancel queued items to reorder.

### Right-click on active production = Suspend

`Suspend` clears the timer (Duration=0) and sets `IsSuspended=true`.
Progress is **preserved** — re-clicking resumes from where it left
off. The `CanAfford` flag is stored to remember whether the player
could afford the next step at suspend time (used by `SetRate` to
restore the manual-hold state).

### Right-click on queued (not-yet-started) = Cancel single

`AbandonProduction`-style removal of one queued item (refund applies
if Object is allocated). Decreases `QueueCount` by 1.

### Insufficient funds at exact moment of step

Per the formula: `Progress -= 1` rolls back, `NoFunds = true` flags
the sidebar. The cameo bar appears to "stall" at the previous fill
level. Next tick: timer expires again, retry. If credits arrive, step
completes and Progress advances.

### Power loss mid-build

`GetPowerRatio` drops below 1.0 → cost increases → Rate increases. But
the existing in-progress timer's Duration is **not** updated until
`RecalcAllRates` fires (which happens on factory count changes, not
power changes). So **a single power drop mid-build does not slow
already-in-progress items until the next factory count event**. New
items begin production with the new (slower) Rate.

**Confidence: MEDIUM** — `RecalcAllRates` is documented to fire on
factory add/destroy. Whether it also fires on power-state transitions
is not explicitly verified; flagged for follow-up in a focused
power-vs-build doc if needed.

### Building destroyed mid-build

If the producing factory building is destroyed:
- Per [BUILD_QUEUE_GHIDRA_REPORT.md](../BUILD_QUEUE_GHIDRA_REPORT.md):
  the FactoryClass instance is NOT destroyed (it's per-house, not
  per-building). Production continues if any remaining factory of the
  same category exists. If none remain, production stalls (cameo
  greyed out).
- `RecalcAllRates` fires for the surviving factories (now with one
  fewer multi-factory discount step).

### Completion → idle

Once `Progress == 54`, `IsSuspended = true` and `Rate = 0` halt the AI
function. The cameo shows the "ready" flash (cross-ref
[cameo-flash-pulse.md](cameo-flash-pulse.md)). On click → place →
delivery, `CompletedProduction` clears `Object`, `Progress = 0`,
`Rate = 0`. For successful normal delivery, `FUN_004FAA10(heapId=-1)`
then reaches `StartNextQueued` in the same `Place_Production` command and
pulls the next queued item. A blocked stock war-factory vehicle exit returns
before this point and keeps the completed vehicle pending.

---

## TS-legacy filter

| Field / branch | TS-legacy? | Notes |
|---|---|---|
| `Cost=` / `Soylent=` / `Prerequisite=` / `Owner=` / `TechLevel=` | **Live in YR** | All units. |
| `[General] MultipleFactory=0.8` | **Live in YR** | Cumulative multi-factory discount. |
| `[General] MaximumQueuedObjects=29` | **Live in YR** | Queue cap. |
| `BuildTime=` per AI difficulty | **Live in YR** | AI headstart only. |
| `BuildSpeed=` per AI difficulty | **Live in YR (inferred)** | AI-only knob, parsed from difficulty section. |
| `BuildTimeBuildingsMult` / `BuildTimeUnitsMult` / `BuildTimeInfantryMult` / `BuildTimeAircraftMult` / `BuildTimeDefensesMult` (per-house) | **Parsed but INERT in YR** | All commented out / unset in shipping rulesmd. Always 1.0. |
| `BuildTimeMultiplier=` (per-TechnoType) | **Parser is live; rarely set** | Default 1.0. |
| `Power penalty on build speed` | **Live in YR** | `GetPowerRatio` division in GetBuildStepTime. |
| `Soylent` (sell-refund) | **Live in YR** | Per-unit refund value. |
| 54-step constant | **Live in YR** | Hardcoded. |
| Naval flag → separate factory | **Live in YR** | Naval Yard splits from War Factory. |
| Right-click suspend / cancel | **Live in YR** | Sidebar interaction. |
| `[Recharge]` block | **Superweapon recharge — different system** | Owned by [superweapon-recharge.md](superweapon-recharge.md), not unit build. |

---

## Cross-references

- [game-speed-master-clock.md](game-speed-master-clock.md) — game-tick
  to wall-clock conversion (build times scale with GameSpeed slider)
- [logic-vs-render-loop.md](logic-vs-render-loop.md) — `FactoryClass::AI`
  runs in unconditional per-entity loop in `LogicClass::PerTickUpdate`,
  so production continues during the in-game menu pause
- [power-state-machine.md](power-state-machine.md) — `GetPowerRatio`
  penalty cadence
- [building-construction-anim.md](building-construction-anim.md) —
  BuildupAnim / placement animation timing (post-build delivery)
- [cameo-flash-pulse.md](cameo-flash-pulse.md) — cameo "ready" pulse
- [repair-rate-cost-tick.md](repair-rate-cost-tick.md) — building
  repair tick rate (sibling concept on the same FactoryClass-style
  state machine)
- [self-heal-tick.md](self-heal-tick.md) — separate self-healing
  cadence
- [superweapon-recharge.md](superweapon-recharge.md) — superweapon
  charge time (related: AI difficulty `BuildTime` does NOT affect
  superweapon recharge)
- [multiplayer-frame-step.md](multiplayer-frame-step.md) — production
  events flow via network command `0x0E` (Begin_Production), `0x0B`
  (Place_Production), `0x0F` (Suspend), `0x10` (Cancel single),
  `0x2E` (Cancel all)
- [FACTORYCLASS_PRODUCTION_DEEP_DIVE.md](../FACTORYCLASS_PRODUCTION_DEEP_DIVE.md)
  — complete FactoryClass system reference
- [FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md](../FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md)
  — detailed build-speed formula (cited heavily)
- [FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md](../FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md)
  — credit deduction internals
- [BUILD_QUEUE_GHIDRA_REPORT.md](../BUILD_QUEUE_GHIDRA_REPORT.md) —
  FactoryClass struct layout, lifecycle, queue mechanics, network
  command flow

---

## Coverage audit

| Item | Disposition |
|---|---|
| `[TechnoType] Cost` | Owned here (timing-input) |
| `[TechnoType] Soylent` | Cross-referenced (sell-refund, not build-time) |
| `[TechnoType] BuildTimeMultiplier` | Owned here |
| `[TechnoType] Prerequisite / TechLevel / Owner` | Cameo gates, not timing — cross-referenced |
| `[General] MultipleFactory` | Owned here (verified at `RulesClass + 0x57C`) |
| `[General] MaximumQueuedObjects` | Owned here (verified at `RulesClass + 0xF0`) |
| `[Easy/Normal/Difficult] BuildTime` | Owned here (AI Progress headstart) |
| `[Easy/Normal/Difficult] BuildSpeed` | Owned here (AI headstart multiplier) |
| `[Easy/Normal/Difficult] Cost` | Cross-referenced (AI credit multiplier, separate concern) |
| `[Easy/Normal/Difficult] BuildSlowdown` | Cross-referenced (AI behavior gate) |
| `[HouseType] BuildTime / BuildTime*Mult` family | Owned here (flagged INERT in YR) |
| 54-step constant | Owned here |
| `[1..255]` Rate clamp | Owned here |
| `MultipleFactory` cumulative formula | Owned here |
| Per-step cost = Balance / steps_left | Owned here |
| Insufficient-funds Progress rollback | Owned here |
| AI Progress headstart formula | Owned here |
| Refund-on-cancel = full refund | Owned here |
| `RecalcAllRates` event-driven trigger | Owned here |
| `Owner[0x148] + 0x1571` vehicle modifier flag | Owned here (flagged — semantics deferred) |
| Naval flag → separate factory routing | Owned here |
| `IsDifferent` sidebar redraw flag | Owned here |
| Right-click suspend / cancel network events (0x0E / 0x0F / 0x10 / 0x2E / 0x0B) | Cross-referenced to [multiplayer-frame-step.md](multiplayer-frame-step.md) |
| Per-house 6 FactoryClass pointers (Infantry / Aircraft / Building / NavalBuild / Vehicle / Naval) | Owned here |
| FactoryClass `+0x24..+0x74` struct layout | Owned here |
| Production-delivery / unit-exit | Cross-referenced to future production-delivery doc |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| Read [FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md](../FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md) lines 1–360 | Confirmed `GetBuildStepTime` pipeline (5 steps), 54-step constant, [1,255] Rate clamp, `MultipleFactory` cumulative formula, AI headstart formula, refund formula, `RecalcAllRates` trigger semantics, HouseClass production tracking fields, completion / SetRate / Suspend / AbandonProduction full pseudocode |
| Read [BUILD_QUEUE_GHIDRA_REPORT.md](../BUILD_QUEUE_GHIDRA_REPORT.md) lines 1–260 | Confirmed FactoryClass struct layout (`+0x24` Progress, `+0x38` Rate, `+0x58` Object, `+0x60` Balance, `+0x6C` Owner, etc.); per-house factory pointers (`+0x53AC..+0x53CC`); `MaximumQueuedObjects=29` at `RulesClass+0xF0`; network command flow (0x0B/0x0E/0x0F/0x10/0x2E); per-tick `FactoryClass::AI` body and credit-deduction-with-rollback |
| `grep ^MultipleFactory rulesmd.ini` | Confirmed `MultipleFactory=0.8` in shipping rulesmd; comment clarifies cumulative semantics |
| `grep ^BuildTime ini/rulesmd.ini` | Confirmed AI difficulty entries: `[Easy] BuildTime=.8`, `[Normal/Difficult] BuildTime=1.0`. No per-house `BuildTime*Mult` keys present in shipping data — confirms INERT status |
| `search_strings "MultipleFactory"` | Single hit at `0x0083caec`; read by `RulesClass::ReadGeneral` at `0x0066ebc3` → stored at `RulesClass + 0x57C` (float) |
| `search_strings "BuildTime"` | 7 hits: `BuildTimeDefensesMult`, `BuildTimeBuildingsMult`, `BuildTimeAircraftMult`, `BuildTimeUnitsMult`, `BuildTimeInfantryMult`, `BuildTime`, `BuildTimeMultiplier` |
| `get_xrefs_to 0x00843cf0` (`BuildTimeMultiplier`) | Read by `TechnoTypeClass::ReadINI` at `0x00714371` — per-type override |
| `get_xrefs_to 0x00825464` (`BuildTime`) | Two readers: `HouseTypeClass::ReadINI` @ `0x00511a4e`, `RulesClass::ReadDifficulty` @ `0x0066d366` |
| `get_xrefs_to 0x008252c4` (`BuildTimeDefensesMult`) | Read by `HouseTypeClass::ReadINI` @ `0x00511ce6` |
| Read `[MTNK]` rulesmd section | Confirmed shipping Cost=700 / Soylent=700 — sample for the 1000-credit-unit calc above |
| Read `[Easy]` rulesmd section | Confirmed `BuildTime=.8`, `Cost=1.0`, `BuildSlowdown` flag |
