# Particle Spark LightSize / OneFrameLight Lifecycle -- Ghidra Research Report

Date: 2026-05-23
Target: PARTICLE_SPARK_LIGHTSIZE_ONEFRAMELIGHT_LIFECYCLE
Investigation mode: exhaustive-slice, scoped follow-up for lighting re-swarm slot 3
Status: COMPLETE

## Target Question

Verify `ParticleSystemClass` spark lighting behavior for `ParticleSystemType` `LightSize` and `OneFrameLight`: whether lights are persistent or one-frame, the live call paths, lifetime/destruction/update semantics, radius/intensity/tint inputs, and stock standard YR activity.

## Non-goals

- Do not re-investigate particle movement, railgun visuals, or generic particle physics beyond inputs needed for live spark particle counts.
- Do not re-investigate map ambience, scenario `[Lighting]`, building lamps, or LightConvert except to distinguish this spark path from map/cell lighting.
- Do not analyze unrelated combat light callers of the same helper beyond xref separation.
- Do not modify Rust, INI, in-repo docs, or Ghidra state.

## Evidence Needed To Mark COMPLETE

- Decompile plus assembly/xref evidence for persistent spark light creation.
- Decompile plus assembly/xref evidence for `OneFrameLight` creation, draw, removal, and radius formula.
- Decompile/xref evidence for the shared light-effect helper, global vector update/removal, and render draw path.
- INI/default plus reader-address evidence for `SparkSpawnFrames`, `LightSize`, `OneFrameLight`, and stock spark systems.
- Rust-facing scan showing whether a particle light bridge already exists.

All required evidence was collected.

## Stop Conditions

- Stop at `ParticleSystemClass` spark behavior and the shared screen light-effect interface.
- Stop after proving active standard YR stock references for spark systems.
- Stop after identifying Rust deltas and tests; no implementation in this report.

## Active In YR Labels

- `SparkSys`: Active in standard YR. It is the stock `DefaultSparkSystem` and appears in many `DamageParticleSystems` entries.
- `LGSparkSys`: Active in standard YR. It appears in stock building `DamageParticleSystems` entries with `SparkSys`.
- `WeldingSys`: Conditionally active in standard YR. It is the stock `DefaultRepairParticleSystem`; activity depends on repair particle paths.
- `FirestormSparkSys`: Conditional / low-confidence standard activity. It is configured as `DefaultFirestormExplosionSystem`, but Firestorm is TS-legacy-facing and not normal YR skirmish surface.

## Verified Findings

### Persistent Spark Light Path

`ParticleSystemClass__AI_Spark` at `0x0062E840` creates a persistent light effect only when all of these are true:

- global extra animation/detail value at `0x00A8EB78` equals `2`;
- current spark frame counter `PSC+0xF0` equals `PSType+0x300` (`SparkSpawnFrames`);
- `PSType+0x304` (`LightSize`) is greater than zero;
- `PSType+0x30C` (`OneFrameLight`) is false;
- allocation of the 0x18 light-effect object succeeds.

The assembly around `0x0062EBF2..0x0062EC5B` checks the global detail value, compares `PSC+0xF0` against `PSType+0x300`, requires positive `PSType+0x304`, rejects true `PSType+0x30C`, allocates `0x18`, copies system coordinates from `PSC+0x9C/+0xA0/+0xA4`, pushes raw `LightSize`, and calls `FUN_005FF250`.

The persistent path does not copy `PSC+0xF4` facing into the light effect. `FUN_005FF250` initializes effect `+0xC` to zero; the global update later advances that stage.

### One-Frame Spark Light Path

`FUN_0062E280`, the `ParticleSystemClass` vtable `+0x114` target, is the one-frame light helper. A vtable memory read at `0x007EFB9C + 0x114 = 0x007EFCB0` points to `0x0062E280`, and the function has only a data xref from that vtable slot.

It runs only when all of these are true:

- `PSType+0x30C` (`OneFrameLight`) is true;
- `PSType+0x304` (`LightSize`) is greater than zero;
- `PSC+0xCC` live particle count is greater than zero.

Assembly around `0x0062E28D..0x0062E2B9` proves these gates. The radius is not raw `LightSize`; assembly around `0x0062E2BF..0x0062E314` computes:

`Math__ftol(clamp(live_particle_count / ParticleCap, 0.4, 1.0) * LightSize)`

where `ParticleCap` is `PSType+0x2A4`. The helper allocates a 0x18 light effect, calls `FUN_005FF250`, copies `PSC+0xF4` into effect `+0xC`, calls `FUN_005FF850` immediately, removes the effect from the global vector with `FUN_005FF2D0`, then frees it.

### Shared Light-Effect Interface

`FUN_005FF250` constructs a 0x18 screen light-effect entry:

- `+0x00/+0x04/+0x08`: coordinates;
- `+0x0C`: stage/facing index, initialized to zero;
- `+0x10`: radius;
- `+0x14`: flags, initialized to zero.

It inserts the pointer into the global vector at `DAT_00AC167C..DAT_00AC1688`. The function takes coordinates and radius only. No particle color list, RGB tint, alpha, or map lighting profile is passed into this constructor.

`FUN_005FF850` draws/applies one light-effect entry against the primary surface, using effect `+0x0C`, flags `+0x14`, and static lookup tables including `DAT_0083358C` and `DAT_00AC1698`. This is a screen-space/surface effect, not a cell lighting grid rebuild.

`FUN_005FF2D0` removes an effect pointer from the global vector. It is not the map/cell application routine.

### Lifetime, Destruction, And Render Order

`FUN_005FF390`, called from `LogicClassPerTickUpdateLiveVector` at `0x0055B5BE`, iterates the global light-effect vector backwards. For each persistent entry, it adds `8` to effect `+0x0C`; when the new value is greater than `0x4F`, it removes the entry from the vector and frees it.

Therefore a persistent spark light created with stage zero is removed on the update that raises its stage from `72` to `80`. The exact first visible stage depends on tick/render ordering, but the destruction threshold and step are verified.

`FUN_005FFFA0`, called from `TacticalClass_Draw` at `0x006D4664`, iterates the global vector backwards and calls `FUN_005FF850` for each persistent entry. The one-frame helper bypasses persistence by calling `FUN_005FF850` itself and then removing/freeing the entry immediately.

### INI Reader And Defaults

`ParticleSystemTypeClass__ReadINI` at `0x006442D0` reads:

- `SparkSpawnFrames` into `PSType+0x300`;
- `LightSize` into `PSType+0x304`;
- `OneFrameLight` into `PSType+0x30C`;
- `SpawnSparkPercentage` into `PSType+0x2F8`.

Assembly around `0x006445A7`, `0x006445C3..0x006445D4`, and `0x006445DA..0x006445E9` proves the reader-address mapping for the first three fields.

`ParticleSystemClass` construction copies `PSType+0x300` into `PSC+0xF0`, initializes `PSC+0xF4` to `0x1D`, and clears destruction/direction flags. `AI_Spark` decrements `PSC+0xF0` each tick and marks the system done when it drops below one.

### Stock Spark System Data

Stock `rulesmd.ini` has:

- `[SparkSys]`: `BehavesLike=Spark`, `ParticleCap=6`, `SparkSpawnFrames=1`, `LightSize=15`, no `OneFrameLight`, so persistent mode.
- `[FirestormSparkSys]`: `ParticleCap=20`, `SparkSpawnFrames=1`, `LightSize=21`, no `OneFrameLight`, so persistent mode.
- `[WeldingSys]`: `ParticleCap=15`, `SparkSpawnFrames=20`, `LightSize=25`, `OneFrameLight=true`.
- `[LGSparkSys]`: `ParticleCap=7`, `SparkSpawnFrames=5`, `LightSize=25`, `OneFrameLight=true`.

Stock content references `SparkSys` and `LGSparkSys` through damage particle systems and references `SparkSys` through default spark paths. Binary evidence includes damage particle selection in `TechnoClass__AI_Update`, default spark spawning in `CaptureManagerClass__Update`, and default spark spawning in `WarpAttachClass__UpdateAttack`.

## Verified Versus Inferred

- Verified: persistent path gates, raw `LightSize` radius, global vector insertion, and detail gate.
- Verified: one-frame path gates, scaled radius formula, immediate draw/remove/free behavior, and no persistence.
- Verified: persistent entry update step `+8` and removal when stage exceeds `0x4F`.
- Verified: no RGB/tint/color-list input to the 0x18 light-effect constructor.
- Inferred naming only: effect `+0x0C` acts as a stage/facing lookup index. The one-frame helper seeds it from system facing; persistent entries use time-aged stages from zero.

## Current Rust Implementation Status

`src/rules/particle_system_type.rs` already parses `light_size`, `one_frame_light`, `spark_spawn_frames`, and `spawn_spark_percentage` with defaults matching the observed INI semantics.

`src/sim/particles/system_ai.rs` currently treats `ParticleSystemBehavesLike::Spark | Railgun` as a no-op tier-3 path, so Rust has no spark particle spawning and no light behavior for these systems.

`src/map/lighting.rs` and rebuild hooks in `src/app_init.rs` cover map/building-style lighting and do not implement this screen-space spark light-effect vector. No particle light bridge was found.

## Implementation Handoff

1. Verified behavior: `OneFrameLight=false` spark systems create a persistent screen light effect on the first spark frame only when detail/global extra animation value is `2`, using raw `LightSize`.
   Rust delta: implement a render-facing transient spark light effect event/vector, not a map lighting grid entry.
   Affected surface: spark particle AI, app/render bridge, tactical render pass.
   Acceptance scenario: stock damaged object using `SparkSys` creates a short-lived spark flash that fades/ages through the light-effect stage and expires.
   Proposed test: `spark_system_light_size_false_creates_persistent_draw_light_on_first_spawn_frame`.
   Risk: medium; requires preserving sim/render layering.

2. Verified behavior: `OneFrameLight=true` spark systems draw an immediate one-frame effect only after live particles exist, with radius `ftol(clamp(live_count / ParticleCap, 0.4, 1.0) * LightSize)`, then remove/free the effect.
   Rust delta: implement one-frame spark light emission after particle spawn/update has established live count.
   Affected surface: spark particle AI and render effect queue.
   Acceptance scenario: stock `WeldingSys` and `LGSparkSys` flash proportionally to current live particle count and do not persist in the light-effect vector.
   Proposed test: `spark_system_one_frame_light_scales_radius_by_live_particle_ratio_and_does_not_persist`.
   Risk: medium-high if radius rounding or tick ordering is made pixel-exact.

3. Verified behavior: persistent light-effect entries age by `+8` per logic light update and are removed/freed when the new stage exceeds `0x4F`; render draws existing vector entries through the tactical draw light-effect pass.
   Rust delta: add a separate spark light-effect lifetime/update path parallel to tactical rendering, with deterministic aging.
   Affected surface: app-level logic tick and render consumers, not `sim` depending on `render`.
   Acceptance scenario: a persistent `SparkSys` light never remains after the stage-crossing expiry update.
   Proposed test: `spark_light_effect_lifetime_expires_when_stage_reaches_0x50`.
   Risk: low-medium; ordering with draw pass may affect one frame of visibility.

## Negative Facts / Do Not Do

- Do not implement spark `LightSize` through `src/map/lighting.rs` point lights or cell RGB lighting. Evidence: constructor/draw path uses global screen light-effect vector and `TacticalClass_Draw -> FUN_005FFFA0 -> FUN_005FF850`.
- Do not treat `OneFrameLight=true` as "no light". Evidence: `FUN_0062E280` runs only when `OneFrameLight` is true and explicitly draws/removes a one-frame effect.
- Do not use raw `LightSize` for one-frame mode. Evidence: `FUN_0062E280` scales by clamped `live_particle_count / ParticleCap` before `Math__ftol`.
- Do not create a one-frame light before any spark particles exist. Evidence: `FUN_0062E280` requires `PSC+0xCC > 0`.
- Do not tint spark lights from particle `ColorList`, `StartColor`, or `EndColor`. Evidence: `FUN_005FF250` accepts coordinates and radius only; draw uses static light-effect lookup data and flags.

## Remaining Uncertainty

- Exact integer rounding mode of `Math__ftol` for fractional one-frame radius was not runtime-debugged. The formula and call are verified; this only matters for strict pixel-radius tests.
- Exact first visible persistent-light stage in the same logic frame as creation depends on global tick ordering. Creation, update step, render draw path, and destruction threshold are verified.
- `WeldingSys` activity is stock-configured and conditionally live through repair systems, but this slice did not fully trace the complete repair spawn path.

## Stale Docs / Replacement Wording

Affected doc: `C:/Users/enok/Documents/ra2-rust-game-docs/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md`

Replacement wording for stale sections around the old `FUN_0062E280`, `FUN_005FF850`, and `FUN_005FF2D0` descriptions:

`FUN_0062E280` is the `OneFrameLight=true` helper, reached through `ParticleSystemClass` vtable `+0x114`. It runs only when `PSType.OneFrameLight` is true, `LightSize > 0`, and the system has at least one live particle. Its radius is `Math__ftol(clamp(live_particle_count / ParticleCap, 0.4, 1.0) * LightSize)`, not raw `LightSize`. It creates a 0x18 screen-space light effect, copies `PSC+0xF4` into effect `+0x0C`, calls the draw/apply routine `FUN_005FF850`, removes the effect from `DAT_00AC167C` via `FUN_005FF2D0`, then frees it. The persistent path is separate: `AI_Spark` creates a raw-`LightSize` effect only on the first spark frame when detail/global `0x00A8EB78 == 2` and `OneFrameLight == false`; the global light-effect update `FUN_005FF390` later ages/removes persistent entries. `FUN_005FF2D0` removes an effect from the global vector; it is not a cell-light-grid apply routine.

## Proposed Rust Tests

- `stock_rules_parse_spark_light_fields_for_sparksys_weldingsys_lgsparksys`
- `spark_system_light_size_false_creates_persistent_draw_light_on_first_spawn_frame`
- `spark_system_light_detail_gate_suppresses_persistent_light_below_high_detail`
- `spark_system_one_frame_light_requires_existing_particles`
- `spark_system_one_frame_light_scales_radius_by_live_particle_ratio_and_does_not_persist`
- `spark_light_effect_lifetime_expires_when_stage_reaches_0x50`
- `spark_damage_particle_systems_spawn_spark_light_surface_effect`

## Sources

- Ghidra decompile and assembly: `ParticleSystemClass__AI_Spark` `0x0062E840`.
- Ghidra decompile, assembly, and vtable memory: one-frame helper `FUN_0062E280`, vtable slot `0x007EFCB0`.
- Ghidra decompile/xrefs: light-effect constructor `FUN_005FF250`, draw/apply `FUN_005FF850`, remove `FUN_005FF2D0`, updater `FUN_005FF390`, draw-all `FUN_005FFFA0`.
- Ghidra decompile/assembly: `ParticleSystemTypeClass__ReadINI` `0x006442D0`.
- Ghidra xrefs/callers: `LogicClassPerTickUpdateLiveVector`, `TacticalClass_Draw`, `TechnoClass__AI_Update`, `CaptureManagerClass__Update`, `WarpAttachClass__UpdateAttack`.
- INI evidence: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/rules/particle_system_type.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/particles/system_ai.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/map/lighting.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs`.
