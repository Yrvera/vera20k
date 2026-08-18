---
title: Tick And Animation Frame Timing - Extension Investigation
date: 2026-05-16
scope: Targeted Ghidra-backed extension of tick/animation disparity scan gaps
parent_reports:
  - docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md
  - docs/gap-scans/2026-05-16-disparity-scan-tick-animation-speed.md
confidence: High for frame source/order, AnimType rate conversion, AnimClass cadence, building slot ownership, infantry action timers, SHP vehicle WalkRate/IdleRate, temporal/gap visual timers; Medium for VoxelAnimType.Normalized draw-time effect.
---

# Tick And Animation Frame Timing - Extension Investigation

## Scope Decision

The parent `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` is recent and high-confidence, but it left enough implementation-critical edge cases that a targeted extension was warranted. This report does not re-cover the whole system. It extends the disparity scan gaps around:

- authoritative frame source and update ordering;
- `AnimType Rate=`, `RandomRate=`, `Normalized=`, `Next=`, and frame-delay reload semantics;
- owner-specific visible animation cadence for buildings, infantry, SHP vehicles, voxel anims, temporal/gap visuals;
- Rust parity risks caused by mixing 45Hz sim ticks, synthetic 15fps binary frames, milliseconds, and raw `sim.tick`.

No Rust source was modified for this investigation.

## Evidence Read

Live Ghidra decompilation:

- `Main_Tick @ ram:0055D360`
- `FUN_0055E160`
- `CDTimerClass__GetTimeRemaining`
- `FUN_005fb2e0` normalized-rate helper
- `AnimTypeClass__Constructor`
- `AnimTypeClass__ReadINI`
- `AnimClass__Constructor`
- `AnimClass__AI`
- `BuildingClass__UpdateAnimation`
- `BuildingClass__CreateAnimForSlot`
- `BuildingClass__ClearAnimSlot`
- `BuildingClass__UpdateAnimFacingAndDirection`
- `TechnoTypeClass__Constructor`
- `TechnoTypeClass__ReadINI`
- `FootClass__AI`
- `UnitTypeClass__Constructor`
- `UnitTypeClass__ReadINI`
- `UnitClass__Draw_Body_And_Turret`
- `InfantryClass__AI`
- `InfantryClass__Do_Action`
- `InfantryClass__DoType_Sequencer`
- `InfantryClass__Fire_At_Target`
- `VoxelAnimTypeClass__ReadINI`
- `VoxelAnimClass__AI`
- `ParticleTypeClass__ReadINI`
- `TechnoClass__UpdateTemporalVisual`
- `TechnoClass__UpdateGapVisual`

Repository read-only checks:

- `src/util/fixed_math.rs`
- `src/app_types.rs`
- `src/app_sim_tick.rs`
- `src/sim/world/mod.rs`
- `src/sim/animation.rs`
- `src/sim/components.rs`
- `src/rules/art_data.rs`
- `src/rules/ruleset.rs`
- `src/rules/infantry_sequence.rs`
- `src/rules/shp_vehicle_sequence.rs`
- `src/app_building_anim.rs`
- `src/app_chute_anim.rs`
- `src/sim/power_system.rs`
- `src/sim/superweapon/*`

INI sampling:

- `ini/artmd.ini`, `ini/art.ini`
- `ini/rulesmd.ini`, `ini/rules.ini`

## Bottom Line

The disparity scan's main suspicion is confirmed, but the hidden complexity is larger than a tick-rate constant. GameMD has one gameplay frame counter and several frame-derived timing families. Rust currently has:

- a 45Hz sim tick;
- a derived 15fps-ish `binary_frame`;
- raw `sim.tick` in several frame-duration systems;
- millisecond animation accumulators;
- hardcoded `67ms` fallback animation rates;
- parser-time conversion from art `Rate=` to milliseconds.

That means changing `SIM_TICK_HZ` alone would not restore parity. The mismatch is that many systems have lost their original timing domain.

## Finding 1 - GameMD Frame Counter Increments Late

**Verified binary behavior:** `Main_Tick @ ram:0055D360` increments `g_CurrentFrameCounter` only near the end of the tick, after input, `LogicClass__AI`, optional house AI, `Map__Logic`, `RenderFrame_main`, replay/desync side work, `LogicClass__PerTickUpdate`, several UI/tactical services, and `Network_ServiceLoop`.

The final increment is gated:

```text
if DAT_00A83D49 == 0
and DAT_00A8ECD0 == 0
and DAT_008B41C0 == 0
and DAT_00A83D48 == 0:
    g_CurrentFrameCounter++
```

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Logic and render inside the tick see the old frame number. | `Main_Tick @ ram:0055D360`, increment after `Network_ServiceLoop` | High | Yes |
| Scenario-delay render-only path can process input/network/render and return without incrementing the frame counter. | `Main_Tick`, branch on `ScenarioClass+0x62c` | High | Conditional |
| Single-player/game-mode throttle uses `DAT_00A8EB60`, copied into `DAT_00887350`, with `GetRadarTimer()` units of `timeGetTime() >> 4` (16ms buckets). | `Main_Tick`, `FUN_0055E160`, `GetRadarTimer` | High | Yes |
| Game mode 0 can temporarily force stored game speed and throttle to `2` when `DAT_00A8EDDC == 0`. | `Main_Tick` early game-mode-0 branch | High | Conditional |
| Multiplayer/network paths can use 0, 2, `0x3c / DAT_00A8B558`, or add 10ms increments based on remote frame-budget thresholds. | `Main_Tick` network branches | High | Multiplayer |

**Parity implication:** Any Rust system that starts/checks timers using a counter incremented at the beginning of `advance_tick()` can be one frame early versus GameMD. Any system that uses raw 45Hz `sim.tick` for GameMD frame durations can be much faster than intended.

## Finding 2 - CDTimerClass Is Computed From Global Frame, Not Decremented

**Verified binary behavior:** `CDTimerClass__GetTimeRemaining` stores a start frame and duration. It does not self-decrement.

```text
duration = timer[2]
if timer[0] != -1:
    elapsed = g_CurrentFrameCounter - timer[0]
    if elapsed < duration:
        return duration - elapsed
    return 0
return duration
```

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `start_frame == -1` returns raw duration rather than zero. | `CDTimerClass__GetTimeRemaining` | High | Yes |
| Expiry comparison is `elapsed < duration`; `elapsed == duration` is expired. | `CDTimerClass__GetTimeRemaining` | High | Yes |
| Timers started during tick `N` use `start_frame = N` until the global frame increments late in `Main_Tick`. | `CDTimerClass__GetTimeRemaining`, `Main_Tick` | High | Yes |
| The same pattern appears inline in temporal/gap visual functions rather than always calling a named timer function. | `TechnoClass__UpdateTemporalVisual`, `TechnoClass__UpdateGapVisual` | High | Yes |

**Parity implication:** Rust's one-decrement-per-sim-tick timers are not equivalent if the sim tick is not the GameMD frame. This affects blackout, invulnerability, superweapon charge/progress, temporal/gap visuals, and any future CDTimer-backed behavior.

## Finding 3 - `AnimType Rate=` Is A Frame Delay, Then Optional Game-Speed Normalization

**Verified binary behavior:** `AnimTypeClass__ReadINI` reads `Rate=` as an integer. If present:

```text
if Rate < 1:
    internal_rate = 0
else:
    internal_rate = 900 / Rate   // integer division
AnimType.Rate = internal_rate
```

`RandomRate=min,max` endpoints use the same conversion independently, except endpoint `-1` means "not specified" for that endpoint during the read.

**Hidden correction/nuance:** `AnimTypeClass__Constructor` initializes stored `RandomRate.Min` and `RandomRate.Max` to `0,0`. `ReadINI` uses stack defaults of `-1,-1` to avoid overwriting endpoints when the INI omits the key, but the stored default remains `0,0`. Constructor/runtime random-rate selection checks `(min != 0 || max != 0) && min <= max`, so the default `0,0` means "no random rate."

**Verified normalized helper:** `FUN_005fb2e0`:

```text
if rate == 0:
    return 0
game_speed = *ECX_or_options_ptr
if rate < 5:
    return small_rate_table[rate][game_speed]
return (rate << 3) / (game_speed + 1)
```

Small-rate table from the parent report:

| Internal delay | speed 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2 | 2 | 1 | 1 | 1 | 1 | 1 | 1 |
| 2 | 3 | 3 | 3 | 2 | 2 | 2 | 1 | 1 |
| 3 | 5 | 4 | 4 | 3 | 3 | 2 | 2 | 1 |
| 4 | 7 | 6 | 5 | 4 | 4 | 4 | 3 | 2 |

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Normalization happens after `900 / Rate`, not on raw INI `Rate=`. | `AnimTypeClass__ReadINI`, `AnimClass__Constructor`, `FUN_005fb2e0` | High | Yes |
| `Rate=200` becomes 4 frames, `Rate=300` becomes 3, `Rate=400` becomes 2, `Rate=450` becomes 2. | integer divide in `AnimTypeClass__ReadINI` | High | Yes |
| `Rate<=0` stores internal `0`; normalized helper also returns `0` for input `0`. | `AnimTypeClass__ReadINI`, `FUN_005fb2e0` | High | Yes |
| `RandomRate` is selected first, then normalized if the anim type is `Normalized=yes`. | `AnimClass__Constructor`, parent `AnimClass__AI` Next path | High | Yes |
| No clamp exists inside `FUN_005fb2e0` for the game-speed index. It indexes from the stored game-speed value. | `FUN_005fb2e0` decompile | High | Yes, callers/options normally constrain |
| `Normalized=no` deliberately leaves frame delays unadjusted by game speed. INI comments use this for hard-frame matching, e.g. Tesla/Prism-related art. | `AnimClass__Constructor`, INI sampling | High | Yes |

**Parity implication:** Parser-time conversion to milliseconds is lossy. It loses integer frame delay, game-speed normalization, zero-rate freeze behavior, small-rate table behavior, and the distinction between normalized and non-normalized art.

## Finding 4 - AnimClass Has Significant Pre-Advance Side Effects

**Verified binary behavior:** `AnimClass__AI` does not simply "advance frame if timer expired." It performs side effects before frame advancement:

- looping sound update;
- bounce/meteor physics;
- psi-warning visibility;
- special hide-if-no-ore visibility;
- `MakeInfantry` coordinate capture;
- bouncer collision/ground-water handling;
- trailer spawning;
- tiberium/overlay validation;
- lazy `End` and `LoopEnd` setup.

Only after that does it check frame-advance gating.

The frame advance path:

```text
remaining = CDTimerClass__GetTimeRemaining(this + 0x0B4)
if remaining != 0 or FrameDelayReload == 0:
    FrameAdvanced = false
    return

FrameAdvanced = true
CurrentFrame += FrameStep
LastFrameTime = g_CurrentFrameCounter
FrameDelay = FrameDelayReload
```

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Trailer anim spawn is before normal frame advancement and uses `g_CurrentFrameCounter` modulo `TrailerSeperation`, with separation `1` effectively every frame. | `AnimClass__AI` | High | Yes |
| `FrameDelayReload == 0` blocks normal frame advancement even when the timer is expired. | `AnimClass__AI` | High | Yes |
| `LastFrameTime` is rewritten to `g_CurrentFrameCounter` only when the frame advances. | `AnimClass__AI` | High | Yes |
| Reverse/ping-pong uses `FrameStep` sign rather than a separate frame sequence. | `AnimClass__Constructor`, `AnimClass__AI` | High | Yes |
| `Next=` reuses the same `AnimClass` object, reloads type/rate/loop state, resets accumulated damage, sets `CurrentFrame` to the next type's `Start`, and calls `Middle()`. | Parent report plus `AnimClass__AI` | High | Yes |
| `LoopCountRemaining` is byte-sized. Constructor multiplies type loop count by the constructor loop-count argument and clamps stored values below 2 up to 1. | `AnimClass__Constructor` | High | Yes |

**Parity implication:** A generic millisecond world-effect component cannot match GameMD `AnimClass` by only producing the same frame index over time. Side-effect ordering can affect sounds, spawned trailers, damage, owner state, visibility, and chained animations.

## Finding 5 - Building Overlays Are Mostly Attached AnimClass Objects, Not Independent Render Timers

**Verified binary behavior:** `BuildingClass__UpdateAnimation` has a building-local CDTimer-backed frame field, but building art slots are created as real `AnimClass` objects through `BuildingClass__CreateAnimForSlot`.

`BuildingClass__CreateAnimForSlot`:

1. resolves the building slot's AnimType by index;
2. computes slot offset from `BuildingType + slot * 0x44 + 0xF7C`;
3. calls `AnimClass__Constructor(type, coords, delay, 1, 0x1600, 0, 0)`;
4. writes slot X/Y offsets into the new `AnimClass` fields at `+0x100` and `+0x104`;
5. stores the object pointer into one of 21 building anim slots;
6. if a slot already has an AnimClass, copies the old anim's `CurrentFrame` (`+0xAC`) into the new anim before deleting the old object.

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Building slot replacement preserves current frame from the old slot object. | `BuildingClass__CreateAnimForSlot` copies old `+0xAC` | High | Yes |
| Building damage/remap changes propagate over all 21 anim slots. | `BuildingClass__CreateAnimForSlot`, `BuildingClass__UpdateAnimFacingAndDirection` | High | Yes |
| `BuildingClass__ClearAnimSlot(-2)` deletes all 21 slot anims; otherwise it deletes one slot. | `BuildingClass__ClearAnimSlot` | High | Yes |
| Attached anims can inherit building translucency and special Iron Curtain draw state. | `BuildingClass__CreateAnimForSlot`, `UpdateAnimFacingAndDirection` | High | Yes |
| Garrison/muzzle and active/damaged/powered slot decisions happen in the building state machine, but playback is still through `AnimClass` objects for slot art. | `BuildingClass__UpdateAnimation`, `CreateAnimForSlot` | High | Yes |

**Parity implication:** The Rust app-side building animation timers are too shallow if they only advance local frame counters by `dt_ms`. To match GameMD, building slot art must obey `AnimClass` timing, object lifecycle, frame preservation, owner/building remap, and slot-clear behavior.

## Finding 6 - SHP Vehicle Body Animation Is Foot BodyFrameCounter Plus WalkRate/IdleRate

**Verified binary behavior:** `TechnoTypeClass__Constructor` sets:

```text
TechnoType.WalkRate = 1
TechnoType.IdleRate = 0
```

`TechnoTypeClass__ReadINI` reads `WalkRate=` and `IdleRate=` directly into those fields. There is no `900 / Rate` conversion and no normalized helper on this path.

`FootClass__AI` gates increments of `FootClass.BodyFrameCounter` (`this+0x538`) using `g_CurrentFrameCounter % WalkRate` while moving, and `IdleRate` while eligible/idle.

`UnitClass__Draw_Body_And_Turret` maps the counter to final SHP frame:

- moving/body loop: `BodyFrameCounter % WalkFrames`;
- standing/idle/death/firing use separate fields such as `StandingFrames`, `FiringFrames`, `DeathFrameRate`, and start offsets;
- turret/facing selection is a separate draw-time stride.

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `IdleRate=0` disables the idle-specific modulo path; it does not mean "every frame." | `FootClass__AI` | High | Yes |
| The moving path has no visible zero guard before modulo; retail content relies on positive `WalkRate`. | `FootClass__AI`, INI comments | Medium | Yes |
| Rules INI explicitly comments that these rates are for "unit as sprite" hacks and should be powers of two for modulo performance. | `rulesmd.ini`, `rules.ini` | High | Yes |
| `WalkFrames=` and `FiringFrames=` are layout/stride counts, not cadence by themselves. | `UnitTypeClass__ReadINI`, `Draw_Body_And_Turret` | High | Yes |

**Parity implication:** Rust code that builds SHP vehicle sequences from `WalkFrames`/`FiringFrames` and advances by `tick_ms` cannot match dolphins, terror drones, squid, and other SHP-bodied units until it models `BodyFrameCounter`, `WalkRate`, and `IdleRate`.

## Finding 7 - Infantry Cadence Is A Binary Action Timer, Not Art Rate

**Verified binary behavior:** Infantry uses art `Sequence=` for frame ranges, but frame cadence comes from a binary action-delay table and `ActionTimer`.

`InfantryClass__Do_Action`:

```text
if action_id in {9,10,0x12,0x13,0x17,0x20}:
    delay = Normalized(action_delay_table[action_id])
else:
    delay = action_delay_table[action_id]

ActionTimer.start = g_CurrentFrameCounter
ActionTimer.duration = delay
ActionTimer.reload = delay
DoingFrame = 0 or random start frame
```

`InfantryClass__Fire_At_Target` fires only when `DoingFrame == selected_fire_frame`.

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Only six action ids use normalized timing: `9`, `10`, `0x12`, `0x13`, `0x17`, `0x20`. | `InfantryClass__Do_Action` | High | Yes |
| If a target sequence has zero frames, `Do_Action` returns false and does not switch action. | `InfantryClass__Do_Action` | High | Yes |
| Random start clamps frame count below 2 up to 1 before calling random range. | `InfantryClass__Do_Action` | High | Yes |
| Weapon fire is tied to sequence frame, not action start or generic ROF tick. | `InfantryClass__Fire_At_Target` | High | Yes |
| The action-blocking flag and action delay byte are separate byte slots in the 4-byte action record table near `0x007EAF7C`. | Parent report, `Do_Action` data references | High | Yes |

**Parity implication:** Rust's hardcoded per-sequence `tick_ms` model is not parity-correct even if the frame ranges are correct. Infantry visible speed and firing moment must be driven by action ids and their CDTimer-style delay.

## Finding 8 - `Normalized` Exists Outside AnimType, But It Is Not The Same Proven Rate Path Everywhere

**Verified binary behavior:** String xrefs show `Normalized` is read by:

- `AnimTypeClass__ReadINI` at `type+0x362`;
- `VoxelAnimTypeClass__ReadINI` at `type+0x294`;
- `ParticleTypeClass__ReadINI` at `type+0x30F`;
- `TechnoTypeClass__ReadINI` also references the string in a broader parser context.

`VoxelAnimClass__AI` itself does not use `AnimType Rate=`. Its lifetime field decrements once per AI tick, and trailer spawning uses global frame parity:

```text
if Duration > 0:
    Duration--
if Duration > 0 and TrailerAnim exists and (g_CurrentFrameCounter & 1) == 0:
    spawn trailer AnimClass
```

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| VoxelAnimType parses `Normalized=`, but the AI function's lifetime cadence is still per game tick. | `VoxelAnimTypeClass__ReadINI`, `VoxelAnimClass__AI` | High | Yes |
| VoxelAnim trailer spawn cadence is every other global frame, not elapsed milliseconds. | `VoxelAnimClass__AI` | High | Yes |
| ParticleType parses `Normalized=`, but this pass did not trace the particle draw/state runtime far enough to define its exact timing effect. | `ParticleTypeClass__ReadINI` | Medium | Yes |

**Parity implication:** Do not blindly apply `AnimType` `Rate=` semantics to every `Normalized` key. For voxel anims and particles, the key exists but needs owner-specific runtime tracing before implementation.

## Finding 9 - Temporal And Gap Visuals Are Hardcoded CDTimer State Machines

**Verified binary behavior:** `TechnoClass__UpdateTemporalVisual` and `TechnoClass__UpdateGapVisual` use inline CDTimer-style start/duration fields and hardcoded frame counts.

Temporal visual phase durations include:

- 6 frames;
- 4 frames;
- random `20 + Random(-5,5)`;
- 8 frames;
- 16 frames;
- threshold transition when external timer remaining `< 0x36`;
- threshold transition when external timer remaining `< 0x1F`;
- 6, 4, and 20 frame ending phases.

Gap visual is structurally similar, but the hold phases use `0x40` frames and threshold `< 0x9E`.

**Tiny details:**

| Detail | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| State 0 initializes a timer and immediately returns; the next phase is not processed in the same call. | `TechnoClass__UpdateTemporalVisual`, `UpdateGapVisual` | High | Yes |
| The middle visual shimmer uses random duration `20 + Random(-5,5)`. | both functions | High | Yes |
| Late phases depend on an external CDTimer remaining threshold, not only local visual phase duration. | both functions | High | Yes |
| Gap and temporal visuals are not `AnimType Rate=` animations. | both functions | High | Yes |

**Parity implication:** These visuals should not be grouped with generic world-effect animation cadence. They are frame-count state machines coupled to an external effect timer.

## Finding 10 - Current Rust Still Has Multiple Timing Domains In Conflict

**Read-only Rust verification:**

| Rust path | Current state | Parity risk |
|---|---|---|
| `src/util/fixed_math.rs` | `SIM_TICK_HZ = 45`; comments still mention 15fps/66ms in places | Sim tick is not GameMD frame, comments are misleading |
| `src/app_types.rs` | `SIM_TICK_MS = 1000 / SIM_TICK_HZ` with stale `// 66ms` comment | At 45Hz this is 22ms |
| `src/sim/world/mod.rs` | `binary_frame = (total_sim_ms * 15) / 1000` computed near start of `advance_tick` | Counter ordering differs from GameMD's late increment |
| `src/rules/art_data.rs` | `art_rate_to_delay_ms` converts `Rate=` to milliseconds | Loses frame-delay and normalized semantics |
| `src/app_building_anim.rs` | building/fire/muzzle anims accumulate `dt_ms` / `rate_ms` | Bypasses attached `AnimClass` timing and slot lifecycle |
| `src/sim/components.rs` | world effects use `rate_ms` and `elapsed_ms` | Bypasses `AnimClass` frame timers and side effects |
| `src/rules/infantry_sequence.rs` / `src/sim/animation.rs` | infantry sequences use hardcoded `tick_ms` | Missing binary action-delay table and six-action normalization |
| `src/rules/shp_vehicle_sequence.rs` | SHP vehicle body sequences use hardcoded ms defaults | Missing `WalkRate`/`IdleRate` body-frame gate |
| `src/sim/superweapon/*`, `src/sim/power_system.rs` | several duration systems use raw `sim.tick` or decrement once per Rust tick | GameMD frame durations complete too fast at 45Hz |
| `src/sim/movement/facing_class.rs` | facing interpolation consumes `binary_frame` | This is closer to GameMD than raw tick, but ordering still needs care |

## Implementation-Relevant Parity Model

This is not a code plan, but the binary evidence implies these timing domains must remain distinct in any faithful design:

| Timing domain | Binary source | Examples |
|---|---|---|
| Gameplay frame counter | `g_CurrentFrameCounter`, incremented late in `Main_Tick` | CDTimer, AnimClass, infantry action timers, temporal/gap visuals |
| Frame throttle / game speed | `DAT_00A8EB60`, `GetRadarTimer`, wait helper | Wall-clock pacing and normalized animation formula |
| `AnimType` frame delay | `900 / Rate`, optional normalized helper | Generic `AnimClass`, building slot anims, explosions, fire/smoke/chrono anims |
| Foot body frame counter | `FootClass+0x538`, `WalkRate`/`IdleRate` modulo | SHP vehicle bodies |
| Infantry action timer | binary action delay table and action id | Infantry walk/idle/fire/death/action sequences |
| VoxelAnim lifetime | per-AI decrement and frame-parity trailer gate | Debris/voxel animations |
| Temporal/gap visual phases | hardcoded CDTimer state machines | Chrono/temporal/gap visuals |

## Answer To The Open Question

Yes, there were hidden complexities beyond the disparity scan:

1. Building overlay playback is not just a render-side building animation table; slot overlays are real attached `AnimClass` instances, and replacing a slot preserves current frame.
2. `RandomRate` default handling is more subtle than "defaults to -1"; the read uses `-1` sentinels, but constructed storage defaults to `0,0`.
3. `Normalized` is a shared INI key string across AnimType, VoxelAnimType, and ParticleType, but only AnimType's `Rate=` path is proven to use the `900 / Rate` plus small-table helper semantics.
4. VoxelAnim trailer spawning uses global frame parity, not elapsed milliseconds.
5. Temporal/gap visuals are separate hardcoded frame-state machines, not generic animation effects.
6. Main frame increment ordering matters because GameMD increments after logic/render, while Rust currently derives `binary_frame` near the start of `advance_tick`.

## Open Questions

1. Particle runtime timing for `ParticleType.Normalized` needs its own trace before implementing particle animation cadence.
2. The exact standard-skirmish reachability of the temporary stored game-speed value `2` should be verified through scenario setup if default wall-clock comparisons are being calibrated.
3. A complete infantry action-id name map `0..0x29` would help implementation, but the timing-critical normalized action ids and table layout are already verified.

## Conclusion

The correct parity target is not "15Hz everywhere" and not "45Hz everywhere." GameMD uses a late-increment gameplay frame counter plus separate throttle/game-speed handling. Visible animation speed is split across several owner-specific frame domains. The current Rust implementation still collapses many of those domains into milliseconds or raw 45Hz ticks, so tick speed and animation play speed can both be visibly off even when individual frame ranges or art assets are correct.
