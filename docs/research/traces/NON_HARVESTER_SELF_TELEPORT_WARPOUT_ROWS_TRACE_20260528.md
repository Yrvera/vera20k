# Non-Harvester Self-Teleport WarpOut Rows Trace - 2026-05-28

## Scope

Concrete scenario only: a Chrono Legionnaire-style non-harvester active Teleport locomotor self-teleports from open ground cell `(10,10,z=0)` to open ground cell `(30,10,z=0)` with stock YR `[General]` chrono settings.

Question: does current Rust emit the two self-teleport `WarpOut` rows through the generic teleport movement state machine with constructor-equivalent fields `type`, `coords`, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=false`, while preserving a nonzero chrono delay / BeingWarped countdown for a non-harvester?

Adjacent behavior is recorded only in "Adjacent Findings"; it was not expanded into a separate trace.

## Evidence Used

- `docs/research/ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md:37-58`, `:63-69`, `:84-89`, `:144-149`
- `docs/research/TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md:210-214`, `:223-331`, `:333-345`, `:382-408`, `:688-737`, `:761-773`, `:907-912`
- Read-only Ghidra spot-checks on 2026-05-28:
  - `TeleportLocomotionClass__StateMachineTick`: active state-0 self-teleport path reads `RulesClass+0x33C`, constructs departure and arrival `AnimClass`, sets timer from `distance / ChronoDistanceFactor`, sets `Techno+0x271`, and only clears it immediately for `WhatAmI()==1 && Harvester=yes`.
  - `AnimClass__Constructor`: full constructor accepts `type, coords, delay, loopCount, drawFlags, zAdjust, reverse`, registers the object in `g_AnimClass_Array`, stores draw flags, clamps loop count to at least one, and starts immediately when `delay==0`.
  - `TechnoClass__IsWarpingOut`, `TechnoClass__IsBeingWarped`, `TechnoClass__Draw`: `IsBeingWarped` returns `this+0x271`; draw ORs translucency flags when `IsWarpingOut` or `IsBeingWarped` is true.
- `ini/rulesmd.ini:223-227`, `:549`
- `src/sim/movement/teleport_movement.rs:34-77`, `:120-136`, `:210-242`, `:258-363`, `:650-701`, `:916-950`
- `src/sim/world/mod.rs:1280-1294`
- `src/sim/components.rs:780-815`, `:817-887`
- `src/app_instances/units.rs:244-249`, `:302-310`
- Focused tests run:
  - `cargo test -q relocate_spawns_departure_and_arrival_warpout_rows` - passed
  - `cargo test -q test_non_harvester_uses_full_chrono_delay` - passed
  - `cargo test -q test_chrono_delay_formula` - passed

## Pipeline

Move command / active Teleport Head_To_Coord -> Rust `start_teleport_state` -> Rust `tick_teleport_movement` Relocate -> `TeleportVisuals::spawn_warp_out` twice -> `WorldEffect::from_anim_spawn` rows -> ChronoDelay countdown -> unit render while countdown is nonzero.

Gamemd equivalent: active YR `TeleportLocomotionClass__StateMachineTick` state 0 self-teleport -> departure `AnimClass(Rules+0x33C, current coords, 0, 1, 0x600, 0, 0)` -> distance timer and `BeingWarped=1` -> non-harvester does not clear timer -> move/mark/mission/occupation -> arrival `AnimClass(Rules+0x33C, current coords, 0, 1, 0x600, 0, 0)` -> later `TimerCheck` clears `BeingWarped`.

## Stage Verdicts

### Stage 1 - Active YR Path And Asset Key

Gamemd active standard YR path: `TeleportLocomotionClass__StateMachineTick` is live for active `Locomotor=Teleport` / `Teleporter=yes` units; existing report marks active in YR and read-only Ghidra spot-check confirmed the state-0 path and constructor rows.

Stock key: `ini/rulesmd.ini:549` gives `WarpOut=WARPOUT;WAKE2`; Rust parser treats `;WAKE2` as comment per `src/rules/ruleset.rs:128-132` and stores `rules.general.warp_out.name = WARPOUT` at `src/rules/ruleset.rs:984-986`.

Verdict: PASS for requested asset identity. Gamemd row type `Rules+0x33C/WarpOut/WARPOUT`; Rust row type `rules.general.warp_out/WARPOUT`.

### Stage 2 - Chrono Delay / BeingWarped Setup

Scenario distance: `(10,10,0)` to `(30,10,0)` is `dx=20*256=5120`, `dy=0`, `dz=0`, distance `5120` leptons.

Gamemd stock settings from `ini/rulesmd.ini:223-227`: `ChronoDistanceFactor=48`, `ChronoTrigger=yes`, `ChronoMinimumDelay=16`, `ChronoRangeMinimum=0`. Delay is `5120 / 48 = 106` integer ticks; `106 > 16`; range force does not apply. State-0 sets `BeingWarped(+0x271)=1`; non-harvester Chrono Legionnaire-style infantry does not satisfy the UnitClass Harvester branch, so the nonzero timer remains.

Rust computes `distance_leptons = isqrt(5120^2 + 0^2) = 5120` at `src/sim/movement/teleport_movement.rs:222-226`, then `compute_chrono_delay` returns `5120 / 48 = 106` with the same minimum/range gates at `:120-136`. `start_teleport_state(..., is_harvester=false)` stores `being_warped_ticks=106` at `:227-242`.

Verdict: PASS. Delay value equals `106` on both sides; non-harvester keeps a nonzero BeingWarped/countdown state.

### Stage 3 - Departure And Arrival WarpOut Constructor Rows

Gamemd state-0 self-teleport creates two constructor rows:

- departure before movement: `AnimClass(Rules+0x33C, source coords, delay=0, loop=1, flags=0x600, zAdjust=0, reverse=0)`.
- arrival after movement/marking/occupation: same row at destination/current coords.

Rust `Simulation::advance_tick` wires `TeleportVisuals` from `rules.general.warp_out` and calls the generic teleport tick at `src/sim/world/mod.rs:1280-1294`. In `TeleportPhase::Relocate`, Rust spawns one row at `old_rx/old_ry/old_z` and one at target/current coords at `src/sim/movement/teleport_movement.rs:285-304`. `TeleportVisuals::spawn_warp_out` sets:

- `type_name = warp_out_type`
- source/destination coords with `sub_x=128`, `sub_y=128`, `z=0` for this scenario
- `delay=0`
- `loop_count=1`
- `draw_flags=0x600`
- `z_adjust=0`
- `reverse=false`

These fields are stored in `WorldEffect.anim_spawn` by `WorldEffect::from_anim_spawn` at `src/sim/components.rs:863-887`. The focused Rust test `relocate_spawns_departure_and_arrival_warpout_rows` asserts count `2`, positions, type, delay, loop, flags, z-adjust, and reverse at `src/sim/movement/teleport_movement.rs:650-701`.

Verdict: PASS for the requested row descriptors. Row count is `2` on both sides; field values match exactly for this scenario.

### Stage 4 - ChronoDelay Countdown Lifetime

Gamemd: after state-0 self-teleport, `WarpPhase` remains `0`, `BeingWarped=1`, and the pre-phase check calls `TimerCheck`; expiration clears `BeingWarped(+0x271)`.

Rust: after `Relocate`, nonzero `being_warped_ticks` transitions to `TeleportPhase::ChronoDelay` at `src/sim/movement/teleport_movement.rs:319-325`; subsequent ticks decrement by `1` until zero, then clear `teleport_state` at `:327-363`. The focused test `test_non_harvester_uses_full_chrono_delay` confirms a non-harvester remains in `ChronoDelay` after the relocation tick with the initial nonzero count still present at `:916-950`.

Verdict: PASS for the countdown existence and first relocation tick. Exact final clear tick is not independently runtime-compared against gamemd in this trace, so full lifetime parity beyond the first tick remains UNCHECKED.

### Stage 5 - Unit Translucency While BeingWarped

Gamemd: `TechnoClass__Draw` calls `IsWarpingOut` and `IsBeingWarped`; when either is true, it ORs draw flags with `0x2004`. For this non-harvester self-teleport, `BeingWarped=1` for `106` ticks, so the unit is rendered translucent during the post-warp delay.

Rust: unit instance generation currently sets `alpha: f32 = 1.0` unconditionally at `src/app_instances/units.rs:244-249`, and the emitted sprite uses that alpha at `:302-310`. I found no Rust render path that maps `entity.teleport_state.being_warped_ticks > 0` to the gamemd `0x2004`/50% translucent draw result.

Verdict: FAIL. Player-visible difference: for the 106-tick post-warp delay, gamemd draws the unit translucent; current Rust draws it fully opaque.

### Stage 6 - Full AnimClass Object Semantics

Gamemd rows allocate and register true `AnimClass` objects in `g_AnimClass_Array`; `AnimClass__Constructor` stores draw flags and z-adjust, clamps loop count, and starts playback immediately for `delay==0`.

Rust stores constructor-equivalent row fields in `AnimClassSpawnDescriptor` and wraps them in a `WorldEffect`. This is sufficient to audit the requested row fields, but it is not a full generic `AnimClass` implementation. `WorldEffect` has no global `AnimClass` identity, owner/listener behavior, native object lifecycle, or confirmed `AnimClass::Middle` frame-start parity.

Verdict: NOT-IMPLEMENTED for full native `AnimClass` object semantics. For this specific trace, the requested row descriptor fields are present, but pixel/lifecycle parity of the resulting animation object is not proven.

## Verdict Tally

PASS: 4 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Top Player-Visible Findings

1. FAIL - Stage 5 - During the 106-tick post-warp cooldown, gamemd draws the teleported non-harvester translucent via `BeingWarped -> 0x2004`, but Rust emits unit sprites with `alpha=1.0`; Rust: `src/app_instances/units.rs:244-310`; gamemd: `TechnoClass__Draw`, `TechnoClass__IsBeingWarped`, `TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md:688-737`.
2. NOT-IMPLEMENTED - Stage 6 - Rust stores constructor-like `WorldEffect` rows but does not instantiate native-equivalent global `AnimClass` objects, so animation lifecycle/pixel parity is not proven; Rust: `src/sim/components.rs:817-887`; gamemd: `AnimClass__Constructor`, `ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md:51`.

## Adjacent Findings

- Temporal weapon visuals are separate from this teleport trace. Existing evidence says Chrono Legionnaire temporal attack uses `SQDG`, `Wake`, and rubble rows, not teleport `WarpOut`; this was not traced here.
- Chrono miner harvester self-teleport behavior is adjacent. Gamemd clears the timer and BeingWarped only for `WhatAmI()==1 && Harvester=yes`; this trace intentionally stayed on the non-harvester path.
- Rust world-effect rendering details, palette/blitter parity, audio ordering, and full screen-pixel comparison were not audited in this run.

## Status

COMPLETE for the requested generic teleport row emission and non-harvester chrono-delay check. One render-stage failure and one full-AnimClass implementation gap are recorded separately.
