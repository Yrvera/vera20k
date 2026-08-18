# Tick / Animation Visible Timing Leftovers - Ghidra Addendum

Date: 2026-05-16
Binary: `gamemd.exe` (Yuri's Revenge)
Scope: follow-up to `TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md`, focused on visible timing paths that are not fully explained by `AnimClass::Rate`.

Confidence: High for functions decompiled in this pass; Medium where the binary behavior is verified but standard-skirmish reachability still needs a retail observation probe.
Active in YR: Yes unless explicitly marked conditional.

## Prior State

The previous reports already established:

- `g_CurrentFrameCounter` is the main gameplay frame source.
- `AnimClass::Rate` stores frame delays, not milliseconds: `internal_delay = 900 / INI_Rate`.
- `Normalized=yes` applies the game-speed normalization helper, including a small-delay lookup table.
- SHP vehicle body animation uses `WalkRate` / `IdleRate`.
- Infantry sequence cadence is action-table driven, not just art `Sequence=` data.
- Several Rust paths still use wall-clock `dt_ms`, hardcoded 67 ms, or 45 Hz sim ticks.

This addendum checks the remaining visible systems most likely to make "tick speed" or "animation speed" still feel wrong.

## New Findings

### 1. Standard speed has multiple defaults, and `rulesmd.ini` changed the multiplayer start speed

Binary evidence:

- `OptionsClass__SetDefaults @ 0x005FA350` writes `Options.GameSpeed = 3`.
- `OptionsClass__ReadFromINI @ 0x005FA620` reads `[Options] GameSpeed=` directly into `Options+0x00`; no clamp was visible around this specific field in the decompiled function.
- `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` maps the in-game slider as `DAT_00A8EB60 = 6 - slider_position`.
- `FUN_0069BAB0 @ 0x0069BAB0` can force `DAT_00A8EB60 = 2` once when:
  - `Scenario+0x30D8` byte is still 0,
  - `g_GameActive != 0`,
  - `DAT_0083ED20` caches the old speed if it is `-1`,
  - and the scenario object's first dword is `0`.
- `FUN_0055E160 @ 0x0055E160` waits in two separate units:
  - single-player/menu path uses `GetRadarTimer()`, where `GetRadarTimer @ 0x006C8C40 = timeGetTime() >> 4`, i.e. 16 ms buckets.
  - network/non-mode-0/non-mode-5 path uses `timeGetTime()` milliseconds and loops with `Sleep(0)`.

INI evidence:

- `ini/rules.ini [MultiplayerDialogSettings] GameSpeed=0`.
- `ini/rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1`.
- The comment says `0=fastest, 6=slowest`.

Why this matters:

Treating "GameMD speed" as one fixed 15 Hz assumption is too simple. YR multiplayer/skirmish dialog defaults appear to start from `GameSpeed=1`, options defaults start from `3`, and startup code can temporarily force `2`. The actual parity calibration must choose a concrete scenario: e.g. standard YR skirmish after the dialog applies `rulesmd.ini`, not the constructor default.

Open empirical probe:

- Start retail YR skirmish with default dialog settings and observe `DAT_00A8EB60` after scenario startup, or measure wall-clock frame cadence. This resolves whether `1`, `2`, or a later UI-applied value is the normal calibration point.

### 2. Radar events are frame-counter driven, but scalar motion is per tick, not milliseconds

Binary evidence:

- `TickRadarEvent @ 0x0065FE00`:
  - early-outs if `event+0x3D == 0`;
  - phase timers compare `g_CurrentFrameCounter - start_frame` against per-type durations;
  - on transition to phase 2, writes both timer starts to `g_CurrentFrameCounter`;
  - radius shrinks by `Rules.RadarEventSpeed` every tick;
  - rotation advances by current rotation speed every tick;
  - color fade advances by `Rules.RadarEventColorSpeed` every tick and bounces at 0.0 / 1.0.
- `TickAndDrawRadarEvents @ 0x00660000` also uses `g_CurrentFrameCounter` for timer expiry before drawing.

INI evidence:

- `RadarEventVisibilityDurations=200,...`
- `RadarEventDurations=400,...`
- `RadarEventSpeed=1.2`
- `RadarEventRotationSpeed=.05`
- `RadarEventColorSpeed=.1`

Disparity observed in Rust:

- `Simulation::radar_events.tick(tick_ms)` is called from `src/sim/world/mod.rs`.
- `src/app_building_anim.rs:update_radar_state(state, SIM_TICK_MS as f32)` is called every runtime pass in `src/app_sim_tick.rs`, even outside the fixed-step loop.

Why this matters:

Radar diamonds can look too fast or too slow if durations are converted to milliseconds or if UI chrome runs once per render pass while event state runs once per sim tick. GameMD event duration is frame-count based, while the per-frame visual scalars are applied once per event tick.

### 3. Sidebar and power-bar timing mix three clock domains

Binary evidence:

- `SidebarClass__Action @ 0x006A7780` calls `StripClass__AI` for the four strips once per sidebar action/AI pass.
- `StripClass__AI @ 0x006A8B30`:
  - scroll animation changes offsets by `DAT_00B0B514` toward `DAT_00B0B500`;
  - tab flashing is scheduled to the next 10-frame boundary with `10 - g_CurrentFrameCounter % 10`;
  - flash scheduling calls `FUN_0069DFC0(10, delay, initial_state)`;
  - `FUN_0069E010` decrements a countdown every call and toggles the flash byte when it reaches zero.
- `PowerClass__AnimationTick @ 0x0063FE80` uses `GetRadarTimer()` buckets, not `g_CurrentFrameCounter`, for both:
  - 10-step power-change pulse (`+0x1510/+0x1518`, delay 3 buckets);
  - gradual power distribution bar movement (`+0x1520/+0x1528`).
- Tooltip timing, from earlier `SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`, uses Win32 timers: 1000 ms delay, 10000 ms duration.

Disparity observed in Rust:

- `update_power_bar_anim(state)` is called once per app runtime update, not through a `GetRadarTimer()` bucket model.
- `update_radar_state(state, SIM_TICK_MS as f32)` receives a constant 66 ms per app update rather than a frame/bucket-derived elapsed value.

Why this matters:

The sidebar is not "15 Hz everywhere." Some parts are frame-counted, some decrement every sidebar AI call, power animation uses 16 ms radar timer buckets, and tooltips use real Win32 milliseconds. A single render `dt_ms` or a single 45 Hz sim tick cannot reproduce all of those cadences.

### 4. Particles are a separate frame-domain, and Spark/Railgun are currently no-op in Rust

Binary evidence:

- `ParticleSystemClass__AI @ 0x0062FD60` dispatches by `ParticleSystemType.BehavesLike`, then decrements system lifetime by one. If lifetime reaches zero, it marks the system for deletion.
- `ParticleClass__AI_Dispatch @ 0x0062CE40` dispatches by `ParticleType.BehavesLike`, then decrements particle lifetime by one. If lifetime reaches zero, it sets `particle+0x131 = 1`.
- `ParticleTypeClass__ReadINI @ 0x00644F50` parses:
  - `MaxDC`, `MaxEC`, `StartFrame`, `NumLoopFrames`,
  - `StartStateAI`, `EndStateAI`, `StateAIAdvance`,
  - `Translucent25State`, `Translucent50State`,
  - `Normalized`,
  - `ColorSpeed`.
- `ParticleClass__Constructor @ 0x0062B5E0` copies `StateAIAdvance` to `particle+0x12C`, `StartStateAI` to `+0x12E`, and translucency to `+0x12F`.
- For non-railgun particles, initial lifetime is `MaxEC + abs(Random % MaxEC)`. For railgun particles, it is `MaxEC + abs(Random % 10)`.
- If `ParticleType.Normalized` is true, the constructor recomputes the state-advance byte from a distance-related calculation, rather than using the INI byte directly.
- `ParticleSystemClass__AI_Smoke @ 0x0062ED40` spawns at `g_CurrentFrameCounter % SpawnFrames == 0`, with spawn slowdown/cutoff also advanced once per system tick.
- `ParticleSystemClass__AI_Fire @ 0x0062F9A0` spawns when `g_CurrentFrameCounter % SpawnFrames == 0`, or every third frame while tracking an attached firing object.
- `ParticleSystemClass__AI_Spark @ 0x0062E840` uses `SparkSpawnFrames`, `SpawnSparkPercentage`, random velocity, optional one-frame light, and changes facing by +/-3 with clamps.

Prior doc evidence:

- `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` confirms:
  - gas movement occurs only on odd `g_CurrentFrameCounter` frames;
  - wind drift uses `10 / WindEffect` frame intervals;
  - color gradients advance by `ColorSpeed + random(0..0.05)`;
  - particle frame selection depends on `StartFrame`, `NumLoopFrames`, lifetime remaining, and state.

Disparity observed in Rust:

- `src/sim/particles/system_ai.rs` says Spark and Railgun are Tier 3 no-ops.
- Rust `advance_state` models the parity/`StateAIAdvance` denominator, but not the constructor's `Normalized` distance rewrite for fire particles.
- Rust particles are ticked once per `Simulation::advance_tick`; with the current app fixed step, that is the synthetic sim tick, not necessarily GameMD's current-frame cadence.

Why this matters:

Particles are among the most visible "speed feel" systems: smoke plumes, gas clouds, sparks, railgun trails, and flamethrower streams. Missing Spark/Railgun and missing `Normalized` constructor behavior can make effects look static, absent, or wrong-speed even after generic `AnimClass` is fixed.

### 5. Parachute SHP timing is correct only if both the AnimClass frame clock and descent clock are correct

Binary evidence:

- `SpawnUnitsWithParachute @ 0x004585C0` constructs an `AnimClass` for the parachute and then writes `anim+0x100 = -200`.
- `ObjectClass__DetachParachute @ 0x005F6DA0` clears the owner pointer when the attached anim is detached.
- Prior `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` verified:
  - `[PARACH] Rate=400` becomes internal delay `900 / 400 = 2` frames;
  - `LoopStart=20`, `LoopEnd=39`, `LoopCount=30`;
  - the chute is a normal attached `AnimClass`;
  - `ZAdjust=-10` affects depth, not screen Y.

INI evidence:

- `[General] Parachute=PARACH`
- `[General] ParachuteMaxFallRate=-3`
- `[General] NoParachuteMaxFallRate=-100`
- `[PARACH] Rate=400`

Disparity observed in Rust:

- `src/app_chute_anim.rs` advances PARACH render frames by accumulated `dt_ms` and `rate_ms`.
- `src/sim/movement/parachute_descent.rs` updates descent once per sim tick and uses `tick_ms` only as a pause guard.

Why this matters:

PARACH frame speed can look right at 133 ms per frame while the falling infantry descends at the wrong wall-clock speed if the sim tick rate differs from GameMD's effective frame cadence. The visual is two coupled clocks: attached `AnimClass` frame advance and falling-object descent.

### 6. Muzzle flashes are AnimClass, not ad-hoc 67 ms render effects

Binary evidence:

- `TechnoClass::Fire_At @ 0x006FDD50` creates muzzle flash `AnimClass` instances with `drawFlags=0x600`, delay 0, loopCount 1.
- Prior `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` verified:
  - 8-way `WeaponType.Anim=` selection uses the weapon anim vector when it has eight entries;
  - facing index uses `((*facing >> 0xC) + 1) >> 1 & 7`;
  - non-building firers attach the muzzle anim to the owner via `AnimClass::SetOwnerObject`;
  - garrison/building muzzle flash Z-adjust can be hard `-200`;
  - muzzle flash palette does not inherit owner house color.

Disparity observed in Rust:

- Prior report and current scan agree: non-garrison `WeaponType.Anim=` muzzle flashes are not implemented.
- `src/app_building_anim.rs` has a garrison-only `tick_garrison_muzzle_flashes` path with hardcoded `rate_ms: 67`.

Why this matters:

Every shot is a timing probe. If normal unit/infantry muzzle flashes are absent, and garrison muzzle flashes use a hardcoded millisecond loop instead of `AnimClass` frame rules, firing cadence will feel visually wrong even if ROF is correct.

### 7. Terrain `AnimationRate=3` is a live non-AnimClass path

Binary evidence:

- `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` reads:
  - `IsAnimated` to type `+0x2B3`;
  - `AnimationRate` to type `+0x2A0`;
  - `AnimationProbability` to type `+0x2A4`.
- `TerrainClass__AI @ 0x0071C730`:
  - if animated and no active timer, it randomly starts animation based on `AnimationProbability`;
  - when started, stores `g_CurrentFrameCounter` and uses type `AnimationRate` as the timer duration;
  - each timer expiry advances frame by `terrain+0x31`;
  - special `SpawnsTiberium` animated terrain resets at half the SHP frame count and calls `CellClass__SpreadTiberium`.
- `TerrainClass__Draw_It @ 0x0071C1B0` chooses frame from `terrain+0x2B` for animated terrain.

INI evidence:

- Several animated terrain definitions in `rulesmd.ini` use `AnimationRate=3` and `AnimationProbability=.003`.

Disparity risk:

If Rust treats these as generic SHP animations or render `dt_ms`, animated terrain, vein/ore-adjacent legacy visuals, and ambient terrain effects will not match. This is separate from `AnimClass`.

### 8. Rust still mixes at least five visible timing domains

Current Rust evidence:

- `src/sim/world/mod.rs` derives `binary_frame = (total_sim_ms * 15) / 1000`, but also advances many systems every `Simulation::advance_tick`.
- `src/app_sim_tick.rs` fixed-step loop uses `SIM_TICK_MS`; app state has `sim_speed_tps`.
- Render-side building overlays and damage fire use `dt_ms`.
- World effects use `rate_ms`, with some hardcoded `67`.
- Radar/power/sidebar app animations are advanced outside the fixed simulation loop.
- Particle systems tick once per sim tick, and Spark/Railgun no-op.
- Rocking, parachute descent, movement, and aircraft movement are per sim tick.

Why this matters:

GameMD does not have one universal visible clock. The player sees a composition of:

- Game-frame timers (`g_CurrentFrameCounter`);
- `CDTimerClass` frame timers;
- 16 ms `GetRadarTimer()` bucket timers;
- real Win32 millisecond timers for tooltips/dialog UI;
- render/movie callbacks;
- owner-coupled draw-frame selection.

The Rust implementation currently collapses too many of those into `dt_ms`, `SIM_TICK_MS`, or hardcoded `rate_ms`.

## Confirmed Parity Gaps To Track

1. Standard YR skirmish speed calibration remains unresolved until the retail/default path is observed after scenario startup.
2. `rulesmd.ini [MultiplayerDialogSettings] GameSpeed=1` must be considered; base RA2 `rules.ini` says `0`.
3. Radar event durations and scalar motion should be expressed in GameMD frame/tick terms, not only milliseconds.
4. Sidebar tab flash, cameo flash, power-bar animation, and tooltips use different clocks.
5. Particle Spark and Railgun behavior are absent in Rust.
6. Particle `Normalized` constructor-side state-advance rewrite is not modeled.
7. Particle systems should not be audited as `AnimClass`; they have independent lifetime/state/frame formulas.
8. PARACH frame animation and parachute descent must be checked together.
9. Non-garrison muzzle flash `WeaponType.Anim=` is still absent.
10. Garrison muzzle flash hardcoded `67 ms` is not a faithful replacement for `AnimClass`.
11. Terrain `AnimationRate=` is live and separate from `AnimClass`.
12. App/render timers outside fixed sim remain a broad source of visible drift.

## Recommended Next Verification Probes

These should be retail-observable probes, not new implementation:

1. Standard YR skirmish default speed:
   - record elapsed wall-clock for 300 `g_CurrentFrameCounter` increments immediately after map start;
   - compare default skirmish, options default, and each game-speed slider value.
2. Particle visual probe:
   - flamethrower `FireStream` (`Normalized=yes`, `StateAIAdvance=6`);
   - railgun trail (`SmallRailgunPart` / `LargeRailgunPart`, ColorSpeed);
   - welding/spark system (`SparkSpawnFrames`, one-frame light).
3. Parachute probe:
   - count wall-clock descent frames from spawn to landing;
   - separately count PARACH SHP frame changes.
4. Sidebar/power probe:
   - record cameo tab flash period, power-bar change step period, and tooltip delay while game is paused and unpaused.
5. Muzzle probe:
   - fire a normal infantry/unit weapon with 8-way `Anim=`;
   - verify frame chosen for each facing and whether the flash follows the moving firer for one frame.
6. Terrain probe:
   - observe an `IsAnimated=yes` terrain type with `AnimationRate=3` and confirm frame interval and random start behavior.

## Bottom Line

There is more uncovered timing complexity. The core `AnimClass Rate=` formula is correct, but it is not sufficient. GameMD uses several visible timing domains, and the Rust engine still has multiple paths that advance on the wrong clock or are missing the binary system entirely.

