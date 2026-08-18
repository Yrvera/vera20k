# RateTimer Current / Current-Frame Helper Semantics - Ghidra Research Report

**Address(es):** `0x004C93D0` (`RateTimer__Current` / `FacingClass::Current`), `0x004C9220` (`RateTimer__Set`), `0x00426630` (`CDTimerClass__GetTimeRemaining`), `0x004C9480` (RateTimer embedded remaining helper), `0x0055D360` (`Main_Tick`)
**Investigation Mode:** exhaustive-slice, downgraded to partial because no live Ghidra MCP tools were exposed in this slot
**Claimed Scope:** shared current-frame timer helper contract used by `RateTimer::Current`, `CDTimerClass`-style start/current/duration checks, proximity detector arming, body rocking/sinking, building update animation timers, and Rust-facing timer model implications.
**Non-Scope:** full animation state machines, full projectile lifecycle, full rocking physics, full building slot ownership/lifecycle, full ammo/reload helper, and unbounded xref inventory for every timer field in gamemd.exe.
**Confidence:** Medium overall. High for facts cited to existing high-confidence Ghidra reports; Medium for this consolidation because fresh live decompilation was unavailable.
**Active in YR:** Yes. The cited paths are normal game-loop, projectile, TechnoClass AI, building update, and UnitClass facing paths in standard Yuri's Revenge unless individually marked conditional below.

## 1. Overview

`gamemd.exe` uses a shared frame-counter timer contract: timer structs store a `start_frame` and `duration`, then compute elapsed/remaining time from `g_CurrentFrameCounter` on demand. They do not self-decrement.

`RateTimer__Current` is the facing/interpolation version of that contract. It uses the embedded CDTimer fields to compute remaining frames, then returns an interpolated low 16-bit facing/value. Shared callers must preserve the same boundary rule: `elapsed < duration` means still active, while `elapsed == duration` is expired/final.

## 2. Class Layout / Key Offsets

| Owner | Offset | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| Global | `0x00A8ED84` | `g_CurrentFrameCounter`, authoritative gameplay frame counter | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `Main_Tick @ 0x0055D360` | Yes |
| `CDTimerClass` | `+0x00` | `start_frame`; `-1` means paused/not-started | `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`; `0x00426630` | Yes |
| `CDTimerClass` | `+0x04` | side/unknown field; not read by basic remaining helper | `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`; `0x00426630` | Yes |
| `CDTimerClass` | `+0x08` | `duration` in game frames; `0` expires immediately | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `0x00426630` | Yes |
| `RateTimer` / `FacingClass` | `+0x00` | target/current packed value; low word is the interpolated value's destination | `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`; `0x004C93D0` | Yes |
| `RateTimer` / `FacingClass` | `+0x04` | previous/source packed value used for interpolation | same | Yes |
| `RateTimer` / `FacingClass` | `+0x08` | embedded CDTimer start frame | same | Yes |
| `RateTimer` / `FacingClass` | `+0x10` | embedded CDTimer duration | same | Yes |
| `RateTimer` / `FacingClass` | `+0x14` | rate in low 16-bit facing units per frame; `SetROT` stores `rot_byte << 8` | same; `0x004C9680` | Yes |
| `BuildingClass` | `+0x100..0x10B` | BState/production frame CDTimer | `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`; `0x004509E4..0x00450A36` | Yes |
| `BuildingClass` | `+0x388` | RateTimer/Facing field used by shadow-direction/body-facing consumers | `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md` | Yes |
| `ProximityDetector` | `+0x0C/+0x14` | arming timer start and delay | `AAHEATSEEKER2_ARMING_PROXIMITY_DETECTOR_GHIDRA_REPORT.md`; `0x004E11F0` | Yes |

## 3. Core Logic

### CDTimer-style remaining helper

Active in YR: Yes. Evidence: `CDTimerClass__GetTimeRemaining @ 0x00426630` in `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` and `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`.

Contract:

```text
duration = timer.duration
if start_frame != -1:
    elapsed = g_CurrentFrameCounter - start_frame
    if elapsed < duration:
        return duration - elapsed
    return 0
return duration
```

Material details:

| Detail | Evidence | Active in YR |
|---|---|---|
| Countdown is computed on demand from the global frame counter; no timer field self-decrements. | `0x00426630`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` section 3.3 | Yes |
| `elapsed == duration` is expired because the live comparison is `elapsed < duration`. | `0x00426630`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` section 3.3 | Yes |
| `duration == 0` is immediately expired. | `0x00426630` | Yes |
| `start_frame == -1` returns raw `duration` rather than zero. | `0x00426630` | Yes |
| `CDTimerClass::Start @ 0x0046B640` writes `start_frame = g_CurrentFrameCounter` and writes duration; it does not advance or clear other shared users. | `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` | Yes |

### RateTimer::Current / FacingClass::Current

Active in YR: Yes. Evidence: `RateTimer__Current @ 0x004C93D0`, `RateTimer__Set @ 0x004C9220`, `CDTimerClass::Remaining @ 0x004C9480` in `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` and `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`.

Contract:

```text
if rate < 1:
    return target

remaining = CDTimerRemaining(start_frame, duration)
if remaining == 0:
    return target

diff = target.low16 - source.low16   // signed 16-bit arithmetic
step_count = abs(diff) / rate        // integer division
if step_count < 1:
    return target

out.low16 = target.low16 - (diff / step_count) * remaining
out.high16 = target.high16
```

Material details:

| Detail | Evidence | Active in YR |
|---|---|---|
| Interpolation is passive and frame-counter based; there is no separate "advance by ROT this tick" state mutation. | `RateTimer__Current @ 0x004C93D0`; `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` section 2.1 | Yes |
| `RateTimer__Set` snapshots the current interpolated value first when retargeting mid-turn, then starts the new timer from `g_CurrentFrameCounter`. | `RateTimer__Set @ 0x004C9220`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` section 9 | Yes |
| Duration is `abs(new_target.low16 - source.low16) / rate`, integer division. Small rotations below one rate unit become zero-duration snaps. | `0x004C9220`; `0x004C93D0` | Yes |
| Only the low 16 bits are interpolated; high 16 bits are copied from the target packed value. | `0x004C93D0`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` section 9 | Yes |
| Expiration uses the same `elapsed < duration` boundary; at `elapsed == duration`, `Current` returns the final target. | `0x004C93D0`; `0x004C9480`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` section 9 | Yes |
| `SetROT` stores `(rot_byte << 8)` and clamps inputs above `0x7E` to `0x7F` before shifting. | `0x004C9680`; `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` | Yes |

### Global counter ordering

Active in YR: Yes, with pause/render-only conditions. Evidence: `Main_Tick @ 0x0055D360` in `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` and `TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`.

`Main_Tick` increments `g_CurrentFrameCounter` near the end of the tick after input, logic, map logic, render, per-tick side work, and network/service processing. Timers started and checked earlier in the same tick therefore see the old frame value until the end-of-tick increment. Some scenario-delay/render-only branches can process input/network/render and return without incrementing the counter. Active in YR: Conditional for the no-increment branch; Yes for standard counter ordering.

## 4. INI Keys

This helper contract is not itself INI-driven. It consumes durations supplied by other systems.

| Key | Scope | Timer effect | Evidence | Active in YR |
|---|---|---|---|---|
| `ROT=` | Techno/projectile/unit type contexts | Converted by `SetROT` to per-frame `rate = ROT << 8` for RateTimer-style facing interpolation in UnitClass turret/barrel and related facing fields. | `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`; `0x004C9680`; `0x00735570..0x0073558D` | Yes |
| `Arm=` | `BulletTypeClass` | Supplies proximity detector arming delay at `+0x14`; `Check` uses `g_CurrentFrameCounter - start_frame >= arm_delay`. | `AAHEATSEEKER2_ARMING_PROXIMITY_DETECTOR_GHIDRA_REPORT.md`; `0x004E11F0` | Conditional by target kind |
| `WalkRate=` / `IdleRate=` | `TechnoTypeClass` | Not CDTimer fields; raw `g_CurrentFrameCounter % rate` gates for SHP body frame counter. Included to avoid folding modulo consumers into RateTimer. | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `FootClass__AI @ 0x004DA530` | Yes |
| `Rate=` / `Normalized=` | `AnimTypeClass` | Not RateTimer; converted to internal frame delays for `AnimClass` CDTimer-style frame delay, with optional game-speed normalization. | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `0x00427D00`; `0x005FB2E0` | Yes |

## 5. Integration Points

| Consumer | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Proximity detector | Arming gate is `current_frame - start_frame < arm_delay` => not armed; armed at `elapsed >= arm_delay`. | `AAHEATSEEKER2_ARMING_PROXIMITY_DETECTOR_GHIDRA_REPORT.md`; `0x004E11F6..0x004E1211` | Yes |
| Body rocking / sinking | Sinking path samples `RateTimer__Current` and extracts `((timer >> 12) + 1) >> 1 & 7` to choose forward tilt direction; rocking update itself runs per Techno AI tick. | `BODY_ROCKING_GHIDRA_REPORT.md`; `TechnoClass::RockingUpdate @ 0x0070B570` | Yes |
| Building UpdateAnimation phase A | Reads `CDTimerClass::GetTimeRemaining(&this+0x100)`. When remaining is zero and duration at `+0x10C` is nonzero, it advances `BState_Frame`, rewrites `+0x100 = g_CurrentFrameCounter`, and reloads `+0x108 = +0x10C`. | `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`; `0x004509E4..0x00450A36` | Yes |
| Building charged/superweapon animation | Computes superweapon remaining time with same start/current/duration arithmetic: `remaining = total - (g_CurrentFrameCounter - start)` if start is valid, else raw total. | `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`; charged anim section | Yes |
| Unit turret/barrel/body facing | `RateTimer__Set` and `Current` drive visible interpolation and retargeting; firing/facing paths consume current animated values in UnitClass AI. | `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` | Yes |
| AnimClass | Uses CDTimer-style frame delay fields and `g_CurrentFrameCounter` for frame advancement, but is not RateTimer interpolation. | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `AnimClass__AI @ 0x00423AC0` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current status | Parity implication |
|---|---|---|
| `src/sim/movement/facing_class.rs` | Implements a passive frame-counter facing interpolator with `current(binary_frame)`, retarget snapshot, `elapsed >= duration` final-target boundary, `rot_byte << 8`, and tests for zero duration/wrap cases. | Closest existing Rust model for `RateTimer__Current`; good helper candidate, but it uses `u32`/`Option` and saturating elapsed semantics rather than signed `start_frame == -1` raw-duration semantics. |
| `src/sim/world/mod.rs` | `advance_tick()` updates `total_sim_ms` and `binary_frame` at the start of the Rust tick before command/combat/turret work. | Binary `g_CurrentFrameCounter` increments near end of `Main_Tick`; start-of-tick Rust updates can be one frame early for timers started/checked in the same Rust tick. |
| `src/sim/superweapon/invulnerability.rs` | Passive timer contract `elapsed = current_frame.saturating_sub(start_frame); active = elapsed < duration_frames`. | Matches core expiration boundary for forward-moving frames; does not model `start_frame == -1` paused/raw-duration because the component uses `Option`. |
| `src/sim/world/mod.rs` building-up/down | Uses `elapsed_ticks += 1` and `elapsed_ticks >= total_ticks` removal. | Deterministic but not the shared start/current/duration helper; susceptible to cadence/order differences if intended to model CDTimer-style GameMD frame timers. |
| `src/app_building_anim.rs` and `src/sim/animation.rs` | Many overlays use `elapsed_ms`/`rate_ms` loops. | Render-side approximation does not preserve GameMD CDTimer/RateTimer exact frame counter ordering or normalized frame-delay behavior. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CDTimerClass__GetTimeRemaining @ 0x00426630` | verified from prior Ghidra report | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` | Fresh live decompile unavailable in this slot |
| `CDTimerClass__Start @ 0x0046B640` | verified from prior Ghidra report | `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` | Fresh live decompile unavailable |
| `RateTimer__Current @ 0x004C93D0` | verified from prior Ghidra reports | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` | Fresh live decompile unavailable |
| `RateTimer__Set @ 0x004C9220` | verified from prior Ghidra reports | same | Fresh live decompile unavailable |
| `SetROT @ 0x004C9680` | verified from prior Ghidra report | `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` | none for helper contract |
| `Main_Tick @ 0x0055D360` counter ordering | verified from prior Ghidra report | `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`; `TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md` | exact pause/scenario branch reachability remains separate system context |
| Proximity detector arming | verified from prior Ghidra report | `AAHEATSEEKER2_ARMING_PROXIMITY_DETECTOR_GHIDRA_REPORT.md`; `0x004E11F0` | none for timer helper |
| Body rocking sinking RateTimer bits | verified from prior Ghidra report | `BODY_ROCKING_GHIDRA_REPORT.md`; `0x0070B570` | exact type inventory for ship-rock gates remains out of scope |
| Building UpdateAnimation frame timer | verified from prior Ghidra report | `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`; `0x004509E4..0x00450A36` | full 21-slot anim lifecycle is out of scope |
| Exhaustive xref list for all RateTimer/Current callers | deferred | no live Ghidra MCP in this slot | follow-up with xref tooling if needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED]` OQ-RTC-001 - What is the shared CDTimer expiration boundary? -> Active while elapsed < duration; expired when elapsed == duration. (evidence: `CDTimerClass__GetTimeRemaining @ 0x00426630` via `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`)
- `[RESOLVED]` OQ-RTC-002 - Does the timer self-decrement? -> No; remaining is computed from `g_CurrentFrameCounter - start_frame`. (evidence: `0x00426630`)
- `[RESOLVED]` OQ-RTC-003 - What does start_frame == -1 mean? -> Basic CDTimer remaining returns raw duration rather than zero. (evidence: `0x00426630`)
- `[RESOLVED]` OQ-RTC-004 - Does RateTimer use the same boundary? -> Yes; remaining zero returns final target, so elapsed == duration is final. (evidence: `RateTimer__Current @ 0x004C93D0`; `CDTimerClass::Remaining @ 0x004C9480`)
- `[RESOLVED]` OQ-RTC-005 - How does RateTimer retarget mid-turn? -> `Set` snapshots current interpolated value first, then writes target and starts a new duration from the current frame. (evidence: `RateTimer__Set @ 0x004C9220`)
- `[RESOLVED]` OQ-RTC-006 - What happens when a requested RateTimer turn is smaller than rate? -> Integer division makes duration/step zero and Current snaps to target. (evidence: `0x004C9220`; `0x004C93D0`)
- `[RESOLVED]` OQ-RTC-007 - Is proximity arming a shared frame-counter helper consumer? -> Yes; `Check` arms when `g_CurrentFrameCounter - start >= arm_delay`. (evidence: `ProximityDetector::Check @ 0x004E11F0`)
- `[RESOLVED]` OQ-RTC-008 - Is body rocking's sinking branch a RateTimer consumer? -> Yes; it samples RateTimer Current and derives a 3-bit bucket from the facing word. (evidence: `TechnoClass::RockingUpdate @ 0x0070B570`)
- `[RESOLVED]` OQ-RTC-009 - Are building update animation frame timers CDTimer-style? -> Yes; phase A checks `GetTimeRemaining`, advances on zero remaining, and reloads start/duration from current frame/stage duration. (evidence: `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`)
- `[RESOLVED]` OQ-RTC-010 - Is Main_Tick counter ordering start-of-tick or end-of-tick? -> End-of-tick in GameMD. (evidence: `Main_Tick @ 0x0055D360`)
- `[RESOLVED]` OQ-RTC-011 - Is Rust `FacingClass` already in the right family? -> Mostly yes for RateTimer interpolation; current scan found passive binary-frame interpolation and retarget snapshot tests. (evidence: `src/sim/movement/facing_class.rs`)
- `[RESOLVED]` OQ-RTC-012 - Does Rust have a generic CDTimer helper used everywhere? -> Not found in this scan; several systems use local elapsed ticks/ms or saturating passive helpers. (evidence: `rg` scan of `src/sim`, `src/app_building_anim.rs`)
- `[DEFERRED]` OQ-RTC-013 - Exact xref inventory for every RateTimer::Current caller. (category: `bounded-cost-too-high`; reason: no live Ghidra xref/decompile tooling exposed; next-step-if-pursued: run Ghidra xrefs on `0x004C93D0` and classify only live YR callers)
- `[DEFERRED]` OQ-RTC-014 - Signed overflow/wrap behavior of `g_CurrentFrameCounter` after multi-hour matches for every consumer. (category: `requires-different-system-context`; reason: one prior UnitClass report documents an 8-frame gate overflow quirk, but full signed-counter audit is outside this helper slice; next-step-if-pursued: dedicated frame-counter overflow audit)
- `[DEFERRED]` OQ-RTC-015 - Whether each Rust timer should use `u32`, `i32`, or `Option` for sentinel fidelity. (category: `requires-different-system-context`; reason: depends on each consumer's save/load and paused-timer needs; next-step-if-pursued: Rust timer API design pass)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| CDTimer-style timers are passive `start_frame + duration` checks with active boundary `elapsed < duration`; `elapsed == duration` is expired. | `0x00426630`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` | Partial: `invulnerability.rs` matches boundary; many animation/building paths use elapsed counters/ms. | `src/sim/superweapon/invulnerability.rs`, `src/sim/world/mod.rs`, `src/sim/animation.rs`, `src/app_building_anim.rs` | Shared helper/API should make the boundary explicit and prevent per-system off-by-one drift. | Timer started at frame 100 with duration 2 is active at frames 100 and 101, expired at 102. Proposed test: `cdtimer_elapsed_equal_duration_expires`. | Do not use `<= duration`, decrementing countdowns, or wall-clock ms for GameMD frame timers. |
| `RateTimer__Current` interpolates from source to target by computing remaining frames from the embedded CDTimer; retargeting snapshots current animated value first. | `0x004C93D0`; `0x004C9220`; `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` | Mostly present in `src/sim/movement/facing_class.rs`; check all consumers use binary frames consistently. | `src/sim/movement/facing_class.rs`, turret/combat call sites in `src/sim/world/mod.rs` | Use a passive binary-frame `current(frame)` everywhere facing/body/turret visual interpolation is needed; retarget from visible value. | Start 0->12800 at frame 0, retarget to 25600 at frame 5, source becomes 6400 and new duration is 15. Proposed test: `ratetimer_retarget_snapshots_visible_current`. | Do not implement as mutable per-tick "add ROT" advancement; it breaks same-frame reads and retargets. |
| GameMD increments `g_CurrentFrameCounter` near the end of `Main_Tick`; systems inside the tick see the old frame until the increment. | `Main_Tick @ 0x0055D360`; `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` | Rust `advance_tick()` updates `binary_frame` at the start of the tick. | `src/sim/world/mod.rs::advance_tick`, timer call sites using `self.binary_frame` | Decide and document whether Rust `binary_frame` represents GameMD's pre-tick visible frame or post-tick counter; adjust timer start/check ordering if off by one. | A timer started and queried in the same GameMD-equivalent tick must see elapsed 0, not 1. Proposed test: `binary_frame_timer_started_same_tick_has_zero_elapsed`. | Do not silently use a beginning-of-tick counter for GameMD helpers without compensating at start/check sites. |
| Proximity detector arming uses the shared frame-counter subtraction directly, not a countdown. | `ProximityDetector::Check @ 0x004E11F0`; `AAHEATSEEKER2_ARMING_PROXIMITY_DETECTOR_GHIDRA_REPORT.md` | Current Rust projectile proximity implementation was not deeply scanned in this slot. | Projectile/proximity detector surfaces when implemented or audited. | Store `start_frame` and `arm_delay`; arm only when `current - start >= delay`. | `Arm=2`: checks at start and start+1 return unarmed; start+2 is armed. Proposed test: `proximity_arm_delay_arms_at_elapsed_equal_delay`. | Do not decrement `arm_delay` per projectile tick or tie it to render frames. |

### Negative Facts / Do Not Do

| Do not do | Evidence | Active in YR |
|---|---|---|
| Do not model `CDTimerClass` as a self-decrementing countdown. | `0x00426630` computes from `g_CurrentFrameCounter - start_frame`; no field update occurs. | Yes |
| Do not treat `elapsed == duration` as still active. | `elapsed < duration` is the only active branch in `0x00426630`; RateTimer final-target boundary matches. | Yes |
| Do not fold `RateTimer`, `AnimType Rate=`, `WalkRate`, and proximity arming into one milliseconds-based timer. | Separate consumers and conversions in `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`, `AAHEATSEEKER2...`, and `BODY_ROCKING...`. | Yes |
| Do not implement RateTimer as a mutable per-tick ROT step. | `0x004C93D0` returns current as a pure function of target/source/start/duration/current frame. | Yes |
| Do not use the wrong sinking-bit precedence. | `BODY_ROCKING_GHIDRA_REPORT.md` verifies `SHR 0xC; INC; SHR 1; AND 7`, i.e. `((timer >> 12) + 1) >> 1 & 7`. | Yes |

### Remaining Uncertainty

- Fresh live Ghidra xrefs and decompilation were unavailable in this slot; every binary claim here cites prior reports rather than new tool output.
- Exact signed wrap behavior for all current-frame consumers remains unresolved; one prior UnitClass report documents a signed-mask edge in an 8-frame gate, but this report does not generalize it to all timers.
- Rust's best canonical timer representation (`i32 start_frame` with `-1`, `Option<u32>`, or separate paused state) is an implementation design decision that should be made per consumer after save/load requirements are checked.

### Stale Docs / Follow-up Docs

- No new stale-doc replacement wording found in this slot. Prior reports already corrected the misleading `RateTimer`/`FacingClass` names and the body-rocking sinking-bit precedence.

## Sources

- `docs/research/TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`
- `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- `docs/research/TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`
- `docs/research/AAHEATSEEKER2_ARMING_PROXIMITY_DETECTOR_GHIDRA_REPORT.md`
- `docs/research/BODY_ROCKING_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`
- `src/sim/movement/facing_class.rs`
- `src/sim/superweapon/invulnerability.rs`
- `src/sim/world/mod.rs`
- `src/sim/animation.rs`
- `src/app_building_anim.rs`
