# Global Timing System Completion - Investigation Plan

> **For Claude:** This plan scopes a `/re-investigate` pass. Execute it by
> running `/re-investigate global timing system completion` with this plan loaded
> as context. Do not write Rust code during the investigation.

**Topic:** Global timing model completion across `gamemd.exe` clock spine, frame-counter timers, animation cadence, and high-risk subsystem consumers.
**Scope Size:** Large - 49 named functions, plus INI and Rust timing-surface audit.
**Est. Effort:** ~10-14 hours of `/re-investigate` work, batched by phase.
**Prior Research:** Recent high-confidence reports cover the timing spine, animation-speed pieces, timer primitives, particles, sidebar timing, factory speed, and several subsystem timers. They do not yet form a complete timing ecosystem map.
**Expected Output:** research document at
`docs/research/GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md`
**Next Pipeline Step:** `/brainstorm` for a unified Rust timing architecture, then `/write-plan` for implementation.

---

## 1. Goal

Produce a complete player-visible timing model for YR skirmish/local play: what advances once per `gamemd` frame, what is wall-clock throttled, what is normalized by game speed, and which Rust systems currently use incompatible clocks. The final report must distinguish the global clock spine from subsystem consumers, so future implementation can map every frame-based behavior onto a single explicit synthetic binary frame model.

This is not a full decompile of every timed subsystem. It is a completion pass over the timing ecosystem: the shared primitives, their major hot-path consumers, and the open gaps left by prior timing reports.

## 2. Prior Research Inventory

| Report | Scope | Confidence | Known Gaps |
|--------|-------|------------|------------|
| `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` | Main tick, wait helper, frame counter, speed source, CDTimer/RateTimer/AnimClass basics | High | Explicitly leaves live retail measurement, full normalized table, and full subsystem taxonomy open |
| `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md` | Tick order, speed buckets, normalized animation, infantry action timing, RateTimer, temporal/gap visuals | High | Broad but not exhaustive for all subsystem consumers |
| `DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md` | Default YR skirmish speed source and wait units | High | Needs runtime probe for actual observed counter/sec across slider positions |
| `VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md` | Rust vs GameMD visible timing risks | Medium-High | Static audit only; not a full map of all timers |
| `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md` | CDTimer, RateTimer, facing, drive locomotion timers | High | Timer primitive coverage is good; caller taxonomy is incomplete |
| `SKIRMISH_SPEED_AND_PARTICLE_NORMALIZED_GHIDRA_REPORT.md` | Skirmish speed and particle normalized behavior | High | Particle family still needs integration into global timing map |
| `PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md` | Particle timing details | High | Narrow particle cases only |
| `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` | ParticleSystemClass behavior | Medium-High | Needs cross-check against current global speed assumptions |
| `SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md` | Sidebar timing and tooltip cadence | High | UI timing must be classified separately from sim-frame timing |
| `FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md` | Factory production speed and build timers | High | Needs integration with power state and frame-counter semantics |
| `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` | Building per-tick AI update | Medium-High | Timing branches need cross-map into global frame order |
| `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md` | Building animation update behavior | Medium-High | Needs reconciliation with generic AnimClass and Rust render-side `dt_ms` animation |
| `AIRCRAFTCLASS_GHIDRA_REPORT.md` | Aircraft timers, spawn delays, reload-like behavior | Medium-High | Needs only global timing classification here, not full aircraft reinvestigation |
| `combat/systems/rof_burst_timing.md` | Weapon ROF/burst timing | Medium | Needs active binary cross-check before timing architecture work |
| `TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md` | Ore/tiberium growth and spread timing | High | Needs classification in per-frame/global queue model |

**Conflicts between reports:** none confirmed in scoping. The main unresolved tension is terminology: several older Rust/doc comments treat `45 FPS` or one Rust tick as if it were a GameMD frame, while recent Ghidra reports show default local skirmish is a speed-byte throttle over a late-incremented global frame counter.

## 3. Function Inventory

| # | Phase | Address | Current Name | Scope Reason | Depth Target | TS-Legacy Risk |
|---|-------|---------|--------------|--------------|--------------|----------------|
| 1 | 1 | `0x0055D360` | `Main_Tick` | Root tick entry; establishes update order and frame increment point | FULL | Low |
| 2 | 1 | `0x0055E160` | `FUN_0055e160` | Wait/throttle helper for local and network modes | FULL | Low |
| 3 | 1 | `0x006C8C40` | `GetRadarTimer` | Local wait unit source, `timeGetTime() >> 4` | FULL | Low |
| 4 | 1 | `0x005FA350` | `OptionsClass__SetDefaults` | Default options speed source | MEDIUM | Low |
| 5 | 1 | `0x005FA620` | `OptionsClass__ReadFromINI` | Reads `[Options] GameSpeed` | MEDIUM | Low |
| 6 | 1 | `0x004E1DE0` | `OptionsClass__ApplyFromInGameDialog` | Speed slider mapping and live speed update/command path | FULL | Low |
| 7 | 1 | `0x00671EA0` | `RulesClass__ReadMultiplayerDialogSettings` | Reads `[MultiplayerDialogSettings] GameSpeed` | MEDIUM | Low |
| 8 | 1 | `0x00697F10` | `SessionClass__ReadSkirmishSettings` | Skirmish speed fallback and session copy | FULL | Low |
| 9 | 1 | `0x005B67F0` | `FUN_005b67f0` | Network/session option packet applies live speed | MEDIUM | Multiplayer |
| 10 | 1 | `0x0055AFB0` | `LogicClass__PerTickUpdate` | Major per-frame side work after render and before late frame increment | FULL | Medium - contains TS-era systems; verify active branches |
| 11 | 1 | `0x0055DEE0` | `LogicClass::AI` | Per-frame object AI dispatcher | MEDIUM | Low |
| 12 | 2 | `0x0046B640` | `CDTimerClass__Init` | Starts frame-count timer from global frame | FULL | Low |
| 13 | 2 | `0x00426630` | `CDTimerClass__GetTimeRemaining` | Primary frame-count remaining formula | FULL | Low |
| 14 | 2 | `0x004C9480` | `CDTimerClass__Remaining` | Bool/RateTimer-style remaining form | FULL | Low |
| 15 | 2 | `0x004C9220` | `RateTimer__Set` | Retargeting and duration formula for facing-like interpolation | FULL | Low |
| 16 | 2 | `0x004C93D0` | `RateTimer__Current` | Current interpolated facing/value formula | FULL | Low |
| 17 | 2 | `0x004C9300` | `FacingClass__UpdateFacing` | Main FacingClass wrapper around RateTimer | MEDIUM | Low |
| 18 | 2 | `0x00427D00` | `AnimTypeClass__ReadINI` | `Rate=`, `RandomRate=`, `Normalized=` conversion | FULL | Low |
| 19 | 2 | `0x005FB2E0` | `FUN_005fb2e0` | Normalized delay helper and small-rate table indexing | FULL | Low |
| 20 | 2 | `0x00421EA0` | `AnimClass__Constructor` | Initial delay/reload/loop/reverse setup | FULL | Low |
| 21 | 2 | `0x00423AC0` | `AnimClass__AI` | Frame advancement, trailer separation, loop/expire timing | FULL | Low |
| 22 | 2 | `0x00424CE0` | `AnimClass__Middle` | Playback transition after initial delay | MEDIUM | Low |
| 23 | 2 | `0x00424F00` | `AnimClass__Start` | Start side effects tied to animation timing | MEDIUM | Low |
| 24 | 2 | `0x004255B0` | `AnimClass__Destroy` | ExpireAnim and deferred delete timing | MEDIUM | Low |
| 25 | 2 | `0x0051D6F0` | `InfantryClass__Do_Action` | Infantry action timer and normalized action subset | FULL | Low |
| 26 | 2 | `0x0051CDB0` | `InfantryClass__UpdateIdleAction` | Idle fidget/action cadence caller | MEDIUM | Low |
| 27 | 2 | `0x00736990` | `UnitClass__Facing_Update` | Unit body/turret RateTimer consumer | FULL | Low |
| 28 | 2 | `0x004DB1A0` | `FootClass__GetCurrentSpeed` | Movement speed sampling and frame-rate sensitivity | MEDIUM | Low |
| 29 | 2 | `0x00520F40` | `FootClass__Locomotion_AI` | Locomotion per-frame process bridge | MEDIUM | Medium - locomotor families include dormant TS paths |
| 30 | 2 | `0x004DA530` | `FootClass__AI` | Main foot per-tick AI and locomotor dispatch timing | MEDIUM | Medium - verify active locomotor branches |
| 31 | 3 | `0x0070E5A0` | `TechnoClass__UpdateTemporalVisual` | Hardcoded frame-count visual state machine | FULL | Low |
| 32 | 3 | `0x0070E920` | `TechnoClass__UpdateGapVisual` | Hardcoded frame-count gap visual state machine | FULL | Low |
| 33 | 3 | `0x0070B570` | `TechnoClass__RockingUpdate` | Rocking/sinking visual cadence and RateTimer sampling | MEDIUM | Low |
| 34 | 3 | `0x0043FB20` | `BuildingClass::Update` | Building update hub and timer branch ordering | FULL | Low |
| 35 | 3 | `0x00450630` | `BuildingClass::UpdateRepairAndPower` | Repair/power frame cadence | FULL | Low |
| 36 | 3 | `0x0043E7B0` | `BuildingClass::UpdateGarrisonFire` | Garrison muzzle/fire timing | MEDIUM | Low |
| 37 | 3 | `0x00454DB0` | `BuildingClass__UpdateGapGenerator_Tick` | Gap generator state machine cadence | MEDIUM | Low |
| 38 | 3 | `0x004C9B20` | `FactoryClass::AI` | Production timer hot path | FULL | Low |
| 39 | 3 | `0x004C9C70` | `FactoryClass::StartProduction` | Initializes production timing | FULL | Low |
| 40 | 3 | `0x004C9EA0` | `FactoryClass::CompletionStep` | Build-progress step cadence | FULL | Low |
| 41 | 3 | `0x004CA6E0` | `FactoryClass::UpdateAllStepDelays` | Global rate recalculation on power/build changes | MEDIUM | Low |
| 42 | 3 | `0x004C9FB0` | `FactoryClass::CalcRate` | Build rate formula timing inputs | FULL | Low |
| 43 | 3 | `0x006F47A0` | `FactoryClass::GetProductionSpeed` | Production speed multiplier source | MEDIUM | Low |
| 44 | 3 | `0x006CAF90` | `SuperClass::Constructor` | Superweapon timer initialization | MEDIUM | Low |
| 45 | 3 | `0x006CBEE0` | `SuperClass::AnimStage` | Sidebar/superweapon stage timing | MEDIUM | Low |
| 46 | 3 | `0x006CC390` | `SuperClass::Launch` | Launch timing and recharge/side-effect ordering | MEDIUM | Low |
| 47 | 3 | `0x0071A760` | `TemporalClass::Update` | Temporal weapon countdown and erasure update | MEDIUM | Low |
| 48 | 3 | `0x006297F0` | `TemporalClass::AI` | Temporal 5-state visual animation machine | MEDIUM | Low |
| 49 | 3 | `0x006A7780` | `SidebarClass::AI` | UI-frame timing, tooltip/cameo update cadence | MEDIUM | Low |

**Deferred from this single plan:** particle system subfunctions (`0x0062ED40`, `0x0062F9A0`, `0x0062E6D0`), aircraft spawn/reload, ore growth/spread queues, radiation/EMP, Ivan/C4, and detailed superweapon variants should each be handled as follow-up slice plans if Phase 3 shows unresolved parity risk. They are named here as timing consumers, but not decompiled in full during this pass.

## 4. Detail Checklist

- **Clock units:** identify every place the binary uses `GetRadarTimer()` buckets, `timeGetTime()` milliseconds, or `g_CurrentFrameCounter` frames.
- **Frame increment ordering:** verify what reads old frame `N` and what sees `N+1`, especially timers started and checked in the same `Main_Tick`.
- **Game speed semantics:** resolve stored speed byte, slider position inversion, local skirmish default, network mode exceptions, and any temporary override paths.
- **Timer boundaries:** record start value, stopped sentinel, elapsed formula, `elapsed == duration` behavior, zero duration, negative or zero rate, and restart semantics.
- **Normalized animation:** extract the complete small-delay table and formula for all relevant game-speed values; mark where `Normalized=no` deliberately avoids speed compensation.
- **Subsystem classification:** for every Phase 3 function, classify as frame-counter timer, per-frame decrement, wall-clock/UI timer, or mixed.
- **Hardcoded frame constants:** list all constants that produce visible cadence, including temporal/gap stage thresholds, factory step sizes, garrison flashes, sidebar delays, and repair/power intervals.
- **Rust disparity taxonomy:** map every current Rust timing surface to one of the binary clock classes: synthetic binary frame, fixed sim tick, wall-clock render `dt_ms`, or UI real time.
- **Runtime probe requirements:** define minimal instrumentation needed to observe retail counter/sec at each visible slider position and compare VERA20k `binary_frame` progression.

## 5. INI Keys in Scope

| Key | Section | Default / Example | Suspected Purpose | Currently Parsed in Rust? |
|-----|---------|-------------------|-------------------|----------------------------|
| `GameSpeed` | `[MultiplayerDialogSettings]` | YR `1`, base RA2 `0` | Stored skirmish speed default | Yes, partial |
| `GameSpeed` | `[Options]` / `RA2MD.INI` | local options commonly `3` | Options dialog speed, not absent-skirmish fallback | Partial |
| `Rate` | art animation sections | many values, commonly `200-450` | Converts to internal frame delay as `900 / Rate` | Yes, but often as ms |
| `RandomRate` | art animation sections | e.g. `220,600` | Randomized animation frame delay range | Partial |
| `Normalized` | art animation sections | many `yes`, some deliberate `no` | Game-speed compensation for animation delay | Partial / likely incorrect |
| `LoopCount` | art animation sections | `-1`, `1`, `4`, etc. | Loop count / infinite loop behavior | Partial |
| `SpawnDelay` | art animation / particle sections | `2`, `3` | Trailer/spawn separation cadence | Partial |
| `TrailerSeperation` | art animation sections | varies | Anim trailer spawn frame gate | Unknown |
| `DelayedFireDelay` | art building/weapon anim sections | `28` examples | Hard frame delay for delayed fire | Unknown |
| `RechargeTime` | superweapon sections | minutes, e.g. `10` | Superweapon charge duration | Yes, partial |
| `ShowTimer` | superweapon sections | `yes/no` | UI timer visibility | Partial |
| `BuildSpeed` / factory rate keys | rules economy/general | varies | Production progress cadence | Partial |
| `Repair` `Rate` | `[Repair]` | e.g. `.08` | Repair cadence/economy | Partial |
| `ROF`, `Reload`, `EmptyReload`, `ReloadIncrement` | weapons/techno ammo | many values | Combat/ammo frame timers | Partial, defer full combat slice |
| `Growth`, `Spread`, `GrowthPercentage`, `SpreadPercentage` | `[Tiberiums]` | e.g. `2200`, `.06` | Ore/tiberium growth queues | Partial, defer ore slice |
| `LightningHitDelay`, `LightningScatterDelay` | `[General]` / lightning rules | varies | Lightning Storm frame timers | Partial, defer superweapon slice |
| `DelayKillFrames`, `DelayKillAtMax` | warhead sections | e.g. `5`, `7.0` | Delayed kill frame timing | Unknown, defer combat slice |

## 6. Caller & Integration Map

| Caller Address | Calls Into | When Invoked | Should Executor Decompile? |
|----------------|------------|--------------|----------------------------|
| `0x0055D360` | `LogicClass::AI`, `LogicClass__PerTickUpdate`, render/network/service loops | Every active game tick | YES - root ordering is critical |
| `0x0055D360` | `FUN_0055e160` | End-of-tick throttle and scenario-delay branch | YES |
| `0x00423AC0` | `CDTimerClass__GetTimeRemaining` | Anim frame advance | YES |
| `0x004C93D0` | `CDTimerClass__Remaining` | RateTimer interpolation read | YES |
| `0x00736990` | `RateTimer__Set` / `RateTimer__Current` | Unit body/turret facing | YES |
| `0x0043FB20` | building timer consumers | Building update pass | YES |
| `0x004C9B20` | factory timer consumers | Factory AI pass | YES |
| `0x006A7780` | sidebar timers/tooltips | UI/sidebar frame | YES |
| Particle AI functions | normalized particle spawn gates | Effects pass | NO in this pass; classify from existing reports unless contradiction found |
| Aircraft/ore/radiation/Ivan/C4 systems | frame timers | Specialized subsystem AI | NO in this pass; plan follow-up slices |

Rust integration notes:

- `src/app_types.rs` maps stored game speed to approximate TPS.
- `src/app_sim_tick.rs` scales elapsed wall-clock time into fixed sim steps.
- `src/sim/world/mod.rs` owns `binary_frame`, but many systems still tick at Rust sim frequency or use `dt_ms`.
- `src/app_building_anim.rs`, `src/app_chute_anim.rs`, `src/app_fire_effects.rs`, and `src/app_instances/overlays.rs` use render/app-side millisecond animation timers.
- `src/sim/superweapon`, `src/sim/movement`, `src/sim/animation.rs`, and particle paths contain frame/tick timers that must be classified against GameMD semantics.

## 7. TS-Legacy Risk Register

- **Fog/shroud and TS-style visibility:** any timing branch tied to fog, shroud recalc, or observer behavior must verify standard YR defaults before reporting it as normal gameplay.
- **Dormant locomotors:** Foot/locomotor timing can expose TS-era dormant locomotor paths; classify only active YR locomotor families as normal.
- **Tiberium naming:** several ore/tiberium fields are TS-named; do not infer RA2/YR ore behavior from names alone.
- **EMP/IonStorm:** disabled or campaign/legacy code can appear in rules and binary; verify whether it is reachable in standard YR.
- **Network timing:** mode 4/network wait behavior is real but not local skirmish default; keep network findings separate from single-player/skirmish timing.
- **Scenario-delay render-only branch:** can render and wait without incrementing the frame counter; verify when it is active before applying it to normal play.

## 8. Current Rust Implementation Surface

| File | Timing Surface | Risk |
|------|----------------|------|
| `src/app_types.rs` | `SIM_TICK_MS`, game-speed to TPS mapping | May treat speed byte as direct TPS rather than binary bucket budget |
| `src/app_sim_tick.rs` | fixed-step scheduler, speed scaling, frame stepping | Mixes wall-clock elapsed, fixed sim tick, and speed scaling |
| `src/sim/world/mod.rs` | `binary_frame` and main sim advance | Needs late-increment parity audit |
| `src/sim/animation.rs` | entity animation timers | Likely ms/tick-based instead of GameMD frame timer for some paths |
| `src/rules/art_data.rs` | `Rate=` conversion | Known risk around `900 / Rate`, `Normalized`, and small-rate table |
| `src/app_building_anim.rs` | crane, idle, damage fire, radar, garrison flashes via `dt_ms` | Render-side timers may bypass binary frame cadence |
| `src/app_chute_anim.rs` | parachute frame advance via `dt_ms` | Needs classification as render-only or GameMD-frame behavior |
| `src/app_fire_effects.rs` | muzzle flash frame advance via `dt_ms` | Likely cadence drift |
| `src/app_instances/overlays.rs` | terrain/overlay idle animation timer | Uses global elapsed ms; needs binary frame mapping |
| `src/app_instances/units.rs` | facing interpolation uses `binary_frame` for some visuals | Good direction, but must match RateTimer math exactly |
| `src/sim/movement` | movement timers and `tick_ms` speed integration | Must separate physical movement step from GameMD frame-count timers |
| `src/sim/superweapon` | superweapon charge/effect timers | Some frame-count semantics exist, but recharge units and UI need audit |

## 9. Deferred Open Questions

1. What is the measured `g_CurrentFrameCounter` rate in retail YR local skirmish for every visible game-speed slider position?
2. Does any normal YR local skirmish setup temporarily force speed `2`, and under what scenario/session flags?
3. What exact table values does normalized delay helper `0x005FB2E0` use for small delays and all practical speed bytes?
4. Which Rust timers currently treat one Rust fixed tick as one GameMD frame?
5. Which render-side `dt_ms` animations are purely presentational and which must be tied to GameMD frame cadence?
6. Are sidebar/tooltips and selection affordances tied to game frames, wall-clock UI time, or mixed service-loop timing?
7. Which Phase 3 subsystem consumers deserve their own follow-up plan after classification?

## 10. Execution Strategy

Use **multi-phase batched `/re-investigate`**:

1. **Phase 1 checkpoint:** functions #1-#11. Confirm clock spine, speed sources, frame increment ordering, and normal YR local-skirmish path. Stop and summarize before deeper consumers.
2. **Phase 2 checkpoint:** functions #12-#30. Complete frame-timer primitives, normalized animation, infantry action timing, facing/RateTimer, and movement-facing consumers.
3. **Phase 3 checkpoint:** functions #31-#49. Classify major subsystem consumers and decide which deferred slices need full plans.
4. **Rust taxonomy pass:** after Ghidra phases, audit Rust timing surfaces into a table. Do not change code.
5. **Runtime probe design:** include a small proposed measurement plan for retail and VERA20k, but do not implement it during this research pass.

The scope is deliberately capped. If Phase 3 finds that particles, ore, superweapons, aircraft, or combat timers conflict with the global model, write a follow-up `/plan-investigation` for that slice rather than expanding this pass indefinitely.

## 11. Success Criteria

The executed research document must:

- Answer every question in Section 1.
- Include every function from Section 3, or explicitly justify any omission.
- Resolve every deferred question from Section 9 or re-document it as unresolved with next evidence needed.
- State `Active in YR: Yes/No/Conditional` for every timing behavior.
- Identify the exact clock class for every major subsystem consumer: frame counter, local speed bucket, millisecond timer, UI wall-clock, or mixed.
- Provide a Rust disparity table that can directly feed a timing-architecture brainstorm.
- Cite Ghidra addresses for every high-confidence claim.

## Sources

- Ghidra/address map sampled: `docs/research/ADDRESS_MAP.md`
- Prior docs searched: `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`, `DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md`, `VISIBLE_PACE_AUDIT_GHIDRA_REPORT.md`, `TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`, particle/sidebar/factory/building/aircraft timing reports.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust files sampled: `src/app_types.rs`, `src/app_sim_tick.rs`, `src/sim/world/mod.rs`, `src/sim/animation.rs`, `src/rules/art_data.rs`, `src/app_building_anim.rs`, `src/app_chute_anim.rs`, `src/app_fire_effects.rs`, `src/app_instances/overlays.rs`, `src/app_instances/units.rs`, `src/sim/movement`, `src/sim/superweapon`
- Related plan: `docs/plans/2026-05-16-tick-animation-speed-gap-investigation-plan.md`
