# Tick and Animation Speed Gap Investigation Plan

Date: 2026-05-16

## Scope

Investigate why VERA20k still feels off in tick speed and animation play speed compared with `gamemd.exe`, after the initial high-confidence `AnimClass::Rate`, normalized-rate, `WalkRate`/`IdleRate`, infantry-action, `RateTimer`, and main-throttle research.

This is a gap-focused reverse-engineering plan, not an implementation plan. The follow-up investigation should produce a verified research addendum and should not edit Rust.

## Prior State

Relevant research exists and is useful but incomplete for the symptom. The current best report is:

- `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
  - High confidence: global frame counter, `CDTimerClass`, `AnimClass::Rate`, normalized table, `WalkRate`/`IdleRate`, `RateTimer`, infantry action timers, temporal/gap visual phases, ammo reload timing.
  - Remaining gaps: standard skirmish game-speed value, full infantry SHP frame formula, Rust tick-vs-binary-frame audit, full infantry action-id table, weapon-fire cadence side effects.

Related reports to consult:

- `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`: mission `Rate=` throttles and movement dispatch.
- `FOOTCLASS_AI_GHIDRA_REPORT.md`: `FootClass__AI` order and locomotor processing.
- `DRIVE_TRACK_SYSTEM.md`: drive locomotion per-tick budget.
- `BODY_ROCKING_GHIDRA_REPORT.md`: per-tick visual tilt and `RateTimer` usage.
- `BUILDING_ANIM_STATE_MACHINE.md`: building animation slot behavior.
- `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`: turret/facing timing if present.
- `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md`: voxel/HVA frame and interpolation notes.

## Hypotheses To Test

1. Rust is advancing some systems every 45 Hz sim tick while gamemd advances them only on the 15 Hz frame counter or through mission timers.
2. Rust is advancing render overlays by wall-clock `dt_ms`, while gamemd advances `AnimClass` by game frames and sometimes normalizes by game speed.
3. The default standard-skirmish speed path is not the same value assumed from `OptionsClass` defaults.
4. Infantry and SHP unit draw-frame mapping is still incomplete, so the frame selected may be wrong even if cadence is close.
5. Locomotor/mission rates are being perceived as animation speed because body frames, movement distance, and facing interpolation are coupled in the original.
6. Some visible animations are not `AnimClass` at all: building slots, sidebar/radar, parachutes, muzzle flashes, temporal/gap, voxel HVA, and body rocking use separate timing paths.

## Function Inventory

### A. Main Frame Cadence And Game-Speed Source

1. `Main_Tick @ 0x0055D360`
   - Re-check all paths that assign `DAT_00A8EB60`, `DAT_00887348`, and `DAT_00887350`.
   - Depth target: branch-by-branch table for standard skirmish, campaign, network, replay, paused, and scenario-delay paths.

2. `FUN_0055E160 @ 0x0055E160`
   - Wait/sleep helper.
   - Depth target: exact wall-clock pacing formula for each mode, including `Sleep(0)` vs `Sleep(n)`.

3. `GetRadarTimer @ 0x006C8C40`
   - Confirms 16 ms bucket source.
   - Depth target: verify all callers that treat this as a frame timer.

4. `FUN_0069BAB0`
   - Known path can force `DAT_00A8EB60 = 2`.
   - Depth target: determine whether this is reached in standard YR skirmish startup and when it is overwritten.

5. `OptionsClass__SetDefaults @ 0x005FA350`
   - Game speed default.
   - Depth target: default only; compare with scenario/init overrides.

6. `OptionsClass__ReadFromINI @ 0x005FA620`
   - Reads `[Options] GameSpeed=`.
   - Depth target: clamp/range behavior and persistence.

7. `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`
   - Slider inversion path.
   - Depth target: mapping from UI slider value to stored speed.

### B. Global Tick Dispatch And Per-Class Frequency

8. `LogicClass__PerTickUpdate @ 0x0055AFB0`
   - Global object/factory/house update dispatch.
   - Depth target: confirm exactly which lists tick once per `g_CurrentFrameCounter` increment.

9. `TechnoClass__AI_Update @ 0x006F9E50`
   - Common techno per-frame update.
   - Depth target: order of mission dispatch, visual update, rocking, temporal/gap, and object AI.

10. `ObjectClass__AI @ 0x005F3E70`
   - Falling/sinking/sound/splash visual path.
   - Depth target: determine which visible effects tick independently of `AnimClass`.

11. `FootClass__AI @ 0x004DA530`
   - Body frame counter and locomotor processing.
   - Depth target: every gate that suppresses or allows body-frame increments.

12. `UnitClass__AI @ 0x007360C0`
   - Unit ordering around fire, facing, body, reload, and harvest.
   - Depth target: exact call order and which calls happen before/after `FootClass__AI`.

13. `InfantryClass__AI @ 0x0051BAB0`
   - Infantry ordering around `FootClass__AI`, fire, sequencer, locomotion.
   - Depth target: final call order and timer interaction table.

14. `MissionClass__Mission_Dispatch @ 0x005B3060`
   - Mission rate throttling.
   - Depth target: verify mission `Rate=` values, returned timers, and when AI work is skipped.

### C. AnimClass And Building/Overlay Consumers

15. `AnimTypeClass__ReadINI @ 0x00427D00`
   - Already mostly covered.
   - Depth target: only re-check `Rate=0`, `RandomRate`, `RandomLoopDelay`, and `Normalized` edge cases.

16. `AnimClass__Constructor @ 0x00421EA0`
   - Construction delay and first-frame behavior.
   - Depth target: first tick, `Delay`, `Middle()`, random rate, and `LoopCount` setup.

17. `AnimClass__AI @ 0x00423AC0`
   - Runtime frame advancement.
   - Depth target: exact first-frame/last-frame ordering, `Next=`, `Damage`, `Trailer`, `LoopDelay`.

18. `BuildingClass__UpdateAnimation @ 0x00451265`
   - Building animation slot updates.
   - Depth target: determine when building slots spawn `AnimClass` vs use local counters.

19. `BuildingClass__Update / related building slot owner`
   - Find exact current Ghidra name from `BUILDING_ANIM_STATE_MACHINE.md`.
   - Depth target: active/idle/special/superweapon animation slot cadence.

20. `TechnoClass_DrawSHP @ 0x00705E00`
   - Generic SHP render path.
   - Depth target: confirm whether final frame selection can differ from sim-side animation state.

21. `UnitClass__Draw_Body_And_Turret @ 0x0073C5F0`
   - SHP vehicle/body frame mapping.
   - Depth target: complete standing/firing/death/moving frame formula and firing counter decrement source.

### D. Infantry Sequence Completion

22. `FUN_00523D00`
   - Infantry `Sequence=` parser.
   - Depth target: complete action-id to sequence-name map and token mode table.

23. `InfantryClass__Do_Action @ 0x0051D6F0`
   - Starts infantry action timer.
   - Depth target: full action delay table dump, including blocking flags and normalized ids.

24. `InfantryClass__DoType_Sequencer @ 0x00520AE0`
   - Sequence completion and sound side effects.
   - Depth target: when `DoingFrame` advances, when action changes, and when object death/destruction fires.

25. `InfantryClass__Fire_At_Target @ 0x005206B0`
   - Fires weapon on selected sequence frame.
   - Depth target: map fire-frame fields and confirm primary/secondary/prone/elite selection.

26. Infantry draw-frame function, exact address unresolved
   - Discover through vtable/draw xrefs from `InfantryClass`.
   - Depth target: final SHP frame index from `DoingFrame`, facing token, crouch/prone state, and sequence entry.

### E. Movement, Facing, And Voxel Animation Coupling

27. `RateTimer__Set @ 0x004C9220`
   - Already covered.
   - Depth target: callers that set rate field `+0x14`; confirm default rates by class.

28. `RateTimer__Current @ 0x004C93D0`
   - Already covered.
   - Depth target: caller-specific interpretation of the packed facing value.

29. `UnitClass__Facing_Update @ 0x00736990`
   - Unit body/turret facing target updates.
   - Depth target: when visible facing changes relative to firing and movement.

30. `DriveLocomotionClass` main tick, likely `FUN_004B0500`
   - Vehicle drive-track movement cadence.
   - Depth target: per-frame lepton budget, residual handling, and HVA/body-frame coupling.

31. `HoverLocomotionClass__SpeedUpdate @ 0x00515Fxx` candidates
   - Hover movement speed update.
   - Depth target: whether hover units use frame counters or wall-clock-like rates.

32. `FlyLocomotionClass__Process @ 0x004CDA68`
   - Aircraft visual/movement update.
   - Depth target: aircraft speed and animation cadence, especially ammo reload and muzzle/turn timing.

33. `TechnoClass__RockingUpdate @ 0x0070B570`
   - Already covered in a separate report.
   - Depth target: only verify Rust dispatch frequency and caller order.

34. Voxel/HVA frame advancement functions, exact addresses from `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md`
   - Depth target: whether HVA animation uses `g_CurrentFrameCounter`, local frame timers, or render interpolation.

### F. Non-AnimClass Visible Timing

35. `TechnoClass__UpdateTemporalVisual @ 0x0070E5A0`
   - Temporal visual phases.
   - Depth target: caller ordering and final draw mapping.

36. `TechnoClass__UpdateGapVisual @ 0x0070E920`
   - Gap visual phases.
   - Depth target: caller ordering and final draw mapping.

37. `FUN_006FB010 @ 0x006FB010`
   - Ammo reload tick.
   - Depth target: connect reload cadence to visible weapon/firing cadence if it affects perceived speed.

38. `FUN_006FB080 @ 0x006FB080`
   - Ammo reload duration helper.
   - Depth target: formula already known; verify all callers.

39. Parachute animation path, exact functions unresolved
   - Depth target: compare against Rust `app_chute_anim.rs` hardcoded `133 ms`.

40. Sidebar/radar chrome animation path, exact functions unresolved
   - Depth target: determine whether UI animations tick on render frames, game frames, or their own timers.

## INI Surface To Audit

Audit these keys and confirm each binary parser field, unit, default, clamp, and caller:

- Game speed: `[Options] GameSpeed=`, scenario/game-mode overrides.
- AnimType: `Rate=`, `RandomRate=`, `Normalized=`, `RandomLoopDelay=`, `LoopStart=`, `LoopEnd=`, `LoopCount=`, `Start=`, `End=`.
- TechnoType/body: `WalkRate=`, `IdleRate=`, `ROT=`, `TurretROT=`, `JumpJetTurnRate=`.
- Unit SHP: `WalkFrames=`, `FiringFrames=`, `StandingFrames=`, `DeathFrames=`, `DeathFrameRate=`, `Facings=`, `Start*Frame=`.
- Infantry: `Sequence=` sections and all per-action entries.
- Mission control: mission `[Move] Rate=`, `[Attack] Rate=`, `[Guard] Rate=`, `AARate=`.
- Ammo/reload: `InitialAmmo=`, `Ammo=`, `Reload=`, `EmptyReload=`, `ReloadIncrement=`, `PipWrap=`.
- Special visuals: `AnimationRate=`, parachute/fall-rate keys, superweapon invoke animations, muzzle flash anims.

## Rust Surface To Audit

Map every current Rust timer to one of these categories:

- Uses synthetic `binary_frame`.
- Uses `sim.tick` at 45 Hz.
- Uses `dt_ms` / wall-clock elapsed.
- Uses hardcoded `rate_ms`.
- Uses INI-derived frame counts but converts to milliseconds.
- Advances in render code rather than sim code.

Initial files to audit:

- `src/app.rs`: `sim_speed_tps` and runtime speed controls.
- `src/app_sim_tick.rs`: fixed-step loop, `tick_ms`, and ordering.
- `src/sim/world/mod.rs`: `binary_frame`, `advance_tick` phase order.
- `src/sim/animation.rs`: generic sequence advancement.
- `src/rules/art_data.rs`: `Rate=` conversion.
- `src/rules/infantry_sequence.rs`: hardcoded infantry tick ms.
- `src/rules/shp_vehicle_sequence.rs`: SHP vehicle sequence model.
- `src/app_building_anim.rs`: crane/fire/garrison/radar animation `dt_ms`.
- `src/app_chute_anim.rs`: parachute `rate_ms`.
- `src/app_instances/shp.rs`: final SHP frame resolution and elapsed-ms loops.
- `src/app_instances/units.rs`: voxel/body/warp/facing frame use.
- `src/sim/movement/movement_tick.rs`: movement tick frequency and `tick_ms`.
- `src/sim/movement/drive_track.rs`: drive track budget and residual.
- `src/sim/rocking/rocking_system.rs`: per-tick visual update frequency.
- `src/sim/superweapon/*.rs`: hardcoded invoke/bolt animation rates.

## Empirical Comparison Targets

The investigation should not stop at decompilation. It should specify observable probes against retail YR:

1. Default skirmish wall-clock frame cadence at each game-speed slider value.
2. A normalized looping anim with `Rate=200`, `Rate=400`, and `Rate=450`.
3. A non-normalized anim, to confirm it changes wall-clock speed with game speed.
4. SHP unit moving body frames with `WalkRate=1`, `2`, and `4`.
5. SHP unit idle frames with `IdleRate=0`, `4`, and `8`.
6. Infantry walk, idle fidget, prone/crawl, fire, and death sequences.
7. Vehicle turret/body turn interpolation over a 90-degree target change.
8. Drive locomotion apparent pixels-per-second and body-frame cadence together.
9. Building idle/active overlays, especially refinery/power-plant smoke and active slots.
10. Parachute descent animation frame cadence.
11. Muzzle flash/garrison flash duration and frame cadence.
12. Temporal/gap effect phase durations.

## Expected Deliverables From `/re-investigate`

- Addendum report in `docs/research/`.
- A matrix of timing systems: source counter, unit, parser, default, game-speed normalized yes/no, Rust current behavior.
- A full list of Rust timing mismatches with file references, but no code changes.
- A prioritized fix list grouped by player-visible severity.
- Empirical measurement instructions or captured observations for at least the default skirmish speed and three visible animation classes.

## Stop Conditions

The investigation is complete only when it can answer:

- What is the exact `gamemd` frame cadence in normal YR skirmish at the default speed?
- Which visible animation classes are game-frame driven, render-time driven, mission-rate driven, or locally timer driven?
- Which Rust paths currently run at 45 Hz when they should run at the binary frame cadence?
- Which Rust paths currently use wall-clock `dt_ms` where gamemd uses frame counters?
- Which visible speed mismatch is most likely to explain the user's current symptom?
