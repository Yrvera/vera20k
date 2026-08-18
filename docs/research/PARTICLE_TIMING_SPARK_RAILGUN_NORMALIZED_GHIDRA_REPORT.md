# Particle Timing: Spark, Railgun, Normalized, and Frame Selection

Date: 2026-05-16
Binary: `gamemd.exe` (Yuri's Revenge)
Scope: targeted follow-up for tick/animation parity; focuses on particle visual timing paths that are not `AnimClass`.

Confidence: High for decompiled functions listed below. Medium for semantic names where Ghidra did not recover type names but the behavior is clear.
Active in YR: Yes. `SparkSys`, `FirestormSparkSys`, `SmallRailgunSys`, `LargeRailgunSys`, `WeldingSys`, and `LGSparkSys` are referenced by standard YR INI content.

## Functions Decompiled

- `ParticleSystemTypeClass__ReadINI @ 0x006442D0`
- `ParticleTypeClass__ReadINI @ 0x00644F50`
- `ParticleTypeClass__Constructor @ 0x00644BE0`
- `ParticleSystemClass__Constructor @ 0x0062DC50`
- `ParticleSystemClass__AI @ 0x0062FD60`
- `ParticleSystemClass__AI_Spark @ 0x0062E840`
- `ParticleSystemClass__AI_Railgun @ 0x0062F230`
- `ParticleClass__Constructor @ 0x0062B5E0`
- `ParticleClass__AI_Dispatch @ 0x0062CE40`
- `ParticleClass__Draw_It @ 0x0062CEC0`
- `FUN_0062C3A0` - railgun particle AI
- `FUN_0062C6E0` - spark particle AI
- `FUN_0062CB10` - fire particle AI
- `FUN_0062C540` - smoke particle AI
- `FUN_0062BD50` - gas particle AI
- `FUN_0062D830` - particle draw-frame selection
- `FUN_0062D2A0` - gas movement
- `FUN_0062D3F0` - smoke movement

## Executive Result

Particles are their own timing world. Spark and Railgun are not optional polish: they are live YR systems used by damage effects, railgun weapons, welding sparks, and default spark systems. They do not use `AnimClass::Rate`, do not use art `Rate=`, and do not behave like generic SHP animation. The current Rust implementation explicitly treats Spark and Railgun as no-op Tier 3 branches, so visible timing parity is incomplete even if `AnimClass` is fixed.

## INI Surface

Relevant YR data from `rulesmd.ini`:

- `[General] DefaultSparkSystem=SparkSys`
- `[SmallRailgunSys]`
  - `HoldsWhat=SmallRailgunPart`
  - `BehavesLike=Railgun`
  - `ParticlesPerCoord=.1`
  - `SpiralDeltaPerCoord=.035`
  - `SpiralRadius=6`
  - `PositionPerturbationCoefficient=20`
  - `MovementPerturbationCoefficient=.3`
  - `VelocityPerturbationCoefficient=.6`
  - `Laser=yes`
  - `LaserColor=255,128,0`
- `[LargeRailgunSys]`
  - `ParticlesPerCoord=.15`
  - `SpiralDeltaPerCoord=.03`
  - `SpiralRadius=15`
  - `PositionPerturbationCoefficient=30`
  - `MovementPerturbationCoefficient=.4`
  - `VelocityPerturbationCoefficient=.6`
  - `Laser=yes`
  - `LaserColor=25,20,255`
- `[SparkSys]`
  - `ParticleCap=6`
  - `SparkSpawnFrames=1`
  - `LightSize=15`
  - `SpawnSparkPercentage=1`
- `[WeldingSys]`
  - `ParticleCap=15`
  - `SparkSpawnFrames=20`
  - `LightSize=25`
  - `OneFrameLight=true`
  - `SpawnSparkPercentage=.4`
- `[LGSparkSys]`
  - `SparkSpawnFrames=5`
  - `LightSize=25`
  - `OneFrameLight=true`
  - `SpawnSparkPercentage=.2`
- `[FireStream]`
  - `BehavesLike=Fire`
  - `Velocity=28.0`
  - `Deacc=0.01`
  - `StartStateAI=1`
  - `EndStateAI=19`
  - `StateAIAdvance=6`
  - `Translucent50State=15`
  - `Translucent25State=10`
  - `DeleteOnStateLimit=yes`
  - `Normalized=yes`
  - `FinalDamageState=14`
- Spark and railgun particle colors:
  - `[Spark] ColorSpeed=.13`
  - `[FirestormSpark] ColorSpeed=.13`
  - `[WeldingSpark] ColorSpeed=.13`
  - `[SmallRailgunPart] ColorSpeed=.03`
  - `[LargeRailgunPart] ColorSpeed=.009`

## Binary Findings

### 1. System and particle behavior enums are intentionally different

Verified from read-INI dispatch tables:

- `ParticleSystemType.BehavesLike`: `Smoke=0`, `Gas=1`, `Fire=2`, `Spark=3`, `Railgun=4`.
- `ParticleType.BehavesLike`: `Gas=0`, `Smoke=1`, `Fire=2`, `Spark=3`, `Railgun=4`.

This is easy to get wrong because only Gas/Smoke swap. Spark, Fire, and Railgun line up.

### 2. Binary constructor defaults differ from some Rust parser defaults

`ParticleTypeClass__Constructor @ 0x00644BE0` initializes:

- `MaxEC` (`+0x2E0`) to `1`.
- `NumLoopFrames` (`+0x2F0`) to `1`.
- `StateAIAdvance` (`+0x30B`) to `4`.
- `Translucent25State` and `Translucent50State` to `0xFF`.
- `NextParticle` and `BehavesLike` indices to `-1`.

Rust scan:

- `src/rules/particle_type.rs` defaults `max_ec` to `0`.
- `src/rules/particle_type.rs` defaults `num_loop_frames` to `0`.

Impact:

Standard YR particles usually specify `MaxEC`, but parser defaults still matter for malformed/modded data and for parity tests. A missing `MaxEC` in GameMD is not an immediate zero-lifetime particle; it starts from constructor value `1`.

### 3. Spark systems spawn bursts, not single particles

`ParticleSystemClass__AI_Spark @ 0x0062E840`:

- Runs while `system+0xF0` (`spark_spawn_frames_remaining`) is greater than 0.
- Spawns on the final frame unconditionally, or otherwise only when random probability is <= `SpawnSparkPercentage`.
- The number of spark particles spawned in one tick is randomized:
  - `local_count = random % (ParticleCap / 2) + (ParticleCap / 2)`.
  - For `SparkSys ParticleCap=6`, this means 3..5 sparks per eligible tick.
  - For `WeldingSys ParticleCap=15`, integer half is 7, so 7..13 sparks per eligible tick.
- Every spawned spark gets a random X/Y/Z velocity from the held `ParticleType`:
  - X from `XVelocity`
  - Y from `YVelocity`
  - Z from `MinZVelocity + random % ZVelocityRange`
- The direction vector is normalized and then scaled back to the original random magnitude.
- If the system is marked directionless (`system+0xF9 != 0`), it adds shared per-burst random offsets instead of `SpawnDirection`.
- `spark_spawn_frames_remaining` decrements once per system tick; when it drops below 1, the system marks itself for deletion.

Why this matters:

Spark visual density is timing-sensitive. It is not "spawn one particle per tick"; it is burst-based with probability, cap-derived count, and a finite per-system countdown.

### 4. Spark light behavior has two modes

Inline spark AI creates a persistent light only when:

- detail level global `DAT_00A8EB78 == 2`,
- this is the first spark frame (`system+0xF0 == type.SparkSpawnFrames`),
- `LightSize > 0`,
- and `OneFrameLight == false`.

For `OneFrameLight=true`, earlier research identified the vtable `+0x114` light path: it creates a one-frame light each tick and destroys/applies it immediately. That matches `WeldingSys` and `LGSparkSys`.

Why this matters:

Spark speed is not just particle position. Light cadence is different for `SparkSys` vs `WeldingSys` / `LGSparkSys`.

### 5. Spark system facing jitters by +/-3 and clamps

At the end of `ParticleSystemClass__AI_Spark`, the system facing field `+0xF4` is randomly adjusted:

- A random float chooses one of three outcomes.
- One branch subtracts 3 and clamps below to `0x11`.
- One branch adds 3 and clamps above to `0x29`.
- The middle branch leaves facing unchanged.

Why this matters:

Spark motion has a visible wandering direction independent of individual particle velocity. A static-facing implementation will look too uniform.

### 6. Railgun systems front-load path particle creation

`ParticleSystemClass__AI_Railgun @ 0x0062F230`:

- Only creates the railgun path if `system.marked_for_deletion == false` and current particle count is 0.
- Computes a vector from system start coords to target coords.
- Computes path length and horizontal length.
- Builds a transform from Z rotation plus X rotation.
- Iterates along the beam path. The loop count is derived from distance and `ParticlesPerCoord`.
- For each step:
  - computes spiral phase using `distance_fraction * path_length * SpiralDeltaPerCoord`;
  - applies `SpiralRadius`;
  - applies `PositionPerturbationCoefficient`;
  - allocates one `ParticleClass`;
  - writes direction vector from spiral basis;
  - applies `MovementPerturbationCoefficient`;
  - normalizes the perturbed direction;
  - applies `VelocityPerturbationCoefficient`;
  - sets particle velocity to type `Velocity` plus accumulated perturbation.
- If `Laser=yes`, constructs a `LaserDrawClass` line from start to target using `LaserColor`.
- Marks the railgun system done/deleting after the initial spawn pass.
- After that, it only ticks and prunes existing railgun particles.

Why this matters:

Railgun is not a continuous `SpawnFrames` effect. The beam line and particles are created as a burst along the whole path, then particles age out. Implementing it as one particle per sim tick will visibly lag behind the shot.

### 7. Spark and railgun particles share ColorSpeed interpolation

`FUN_0062C3A0` and `FUN_0062C6E0` both use the same color progression pattern:

```text
accumulator += ParticleType.ColorSpeed + random(0.0..0.05)
if accumulator > 1.0:
    if color_index < ColorListCount - 2:
        color_index += 1
        accumulator = 0.0
    else:
        accumulator = 1.0
```

Draw-time color source in `ParticleClass__Draw_It`:

- If `color_index == 0`, use particle's current start color at `+0xB0`.
- Otherwise use `ParticleType.ColorList[color_index]`.
- Spark/Railgun draw a single colored pixel, not an SHP sprite.
- The pixel is brightness-scaled by the existing A-buffer value before being written to the primary surface.

Why this matters:

`ColorSpeed=.13` means sparks can traverse color pairs in roughly 6-8 particle ticks with random jitter. Railgun parts are much slower (`.03` or `.009`). Hardcoded fading or SHP-frame animation will not match.

### 8. Particle drawing is gated by performance/detail state

`ParticleClass__Draw_It @ 0x0062CEC0` starts with two gates:

- `FUN_0055AF60()` compared to `DAT_00ABCD44`; if the returned threshold is greater than current measured frame rate state, draw may skip unless the particle has nonzero `Damage`.
- If `DAT_00A8EB78 == 0`, Smoke and Spark particles are suppressed by the second gate.

`OptionsClass__SetDefaults` initializes detail level to `2`, so normal high-detail YR draws these. But low-detail or frame-skip paths can suppress smoke/spark visuals while gameplay particle state continues.

Why this matters:

Particle timing parity has a render-policy dimension. A renderer that always draws every particle can differ from low-detail GameMD behavior, while a sim that stops ticking hidden particles would also be wrong.

### 9. Gas and smoke movement are explicitly frame-parity gated

Gas movement helper `FUN_0062D2A0`:

- Applies wind drift when `g_CurrentFrameCounter % (10 / WindEffect) == 0`.
- Only applies settling/drift movement on odd frames.
- Settles toward `ground + 5`, dropping by at most 2 leptons per tick.

Smoke movement helper `FUN_0062D3F0`:

- Applies wind vector every move call.
- Checks bridge flag and may mark smoke dead when crossing a bridge-height threshold.

Gas AI `FUN_0062BD50` and smoke AI `FUN_0062C540` also use `g_CurrentFrameCounter & 1` to gate random drift.

Why this matters:

The current Rust particle tick running once per `Simulation::advance_tick` is only parity-correct if that tick maps to GameMD current-frame cadence. At 45 Hz synthetic sim stepping, odd/even frame gates happen at the wrong wall-clock speed.

### 10. Fire `Normalized=yes` changes constructor state, not just direction vector

`ParticleClass__Constructor @ 0x0062B5E0`:

- Copies `ParticleType.StateAIAdvance` to particle byte `+0x12C`.
- If `ParticleType.Normalized` (`type+0x30F`) is true, it overwrites that particle byte after distance-related calculations.
- The decompiler does not recover the exact expression cleanly; the block computes from the source/destination coordinate delta and writes the final `Math__ftol()` result into `particle+0x12C`.

Verified standard YR data:

- `[FireStream] Normalized=yes`
- `[FireStream] StateAIAdvance=6`

Why this matters:

For flamethrower/fire-stream visuals, the state advance denominator is per-particle and distance-derived. Rust currently parses `normalized` but does not apply a constructor-side rewrite before `advance_state`.

Open follow-up:

- A micro-decompile/assembly pass should isolate the exact `Normalized` formula. The behavior is verified, but the formula deserves a separate focused note before implementation.

### 11. Particle draw-frame selection is not generic sequence playback

`FUN_0062D830`:

- Gas and Smoke return current `animation_state` directly.
- Fire chooses a directional bank:
  - directions 0 or 4: `animation_state`
  - directions 1 or 5: `EndStateAI + animation_state`
  - directions 2 or 6: `2 * EndStateAI + animation_state`
  - directions 3 or 7: `3 * EndStateAI + animation_state`
- Spark and Railgun return 0; they are pixel-color particles, not SHP-frame particles.

Fire direction is computed from screen-space relation between current position and previous/origin coordinates via `FUN_0075F230`.

Why this matters:

Fire-stream visual frames are directional banks, not a single linear animation. Spark/Railgun have no SHP frame animation at all.

## Rust Disparities

Current Rust findings from source scan:

- `src/sim/particles/system_ai.rs` explicitly no-ops `ParticleSystemBehavesLike::Spark | Railgun`.
- `src/rules/particle_type.rs` parses Spark/Railgun fields, but comments say they are unused at Tier 2.
- `src/rules/particle_system_type.rs` parses railgun/spark system fields, but comments say they are unused.
- `src/sim/particles/fire.rs` states several deferred items:
  - attached-object tracking,
  - target-moved 3-tick fallback,
  - map-ground movement wiring,
  - damage application,
  - and historically notes animation-state auto-advance as deferred even though `advance_state` now exists centrally.
- Rust `advance_state` uses a simple counter. GameMD uses a formula tied to current image/frame query and lifetime remaining, and `Normalized=yes` can rewrite the per-particle state advance byte.
- Rust `ParticleType` defaults `MaxEC=0` and `NumLoopFrames=0`; GameMD constructor defaults both to `1`.

## Confirmed Parity Gaps

1. Spark systems are missing, and these are used by default damage spark systems.
2. Railgun systems are missing, and railgun visuals are path-burst effects, not per-frame spawn loops.
3. `ColorSpeed` interpolation with random `0..0.05` jitter is missing for Spark/Railgun.
4. Spark light modes (`OneFrameLight` vs persistent first-frame light) are missing.
5. Spark facing jitter and cap-derived burst size are missing.
6. Railgun `Laser=yes` line creation is missing.
7. Fire `Normalized=yes` constructor rewrite is not modeled.
8. Particle rendering gates by detail/performance state are not modeled.
9. Gas/smoke odd-frame and modulo gates must be tied to GameMD frame cadence, not arbitrary sim tick cadence.
10. Particle parser defaults for `MaxEC` and `NumLoopFrames` differ from GameMD constructor defaults.

## Implementation-Relevant Ordering Notes

No Rust implementation was done in this investigation, but these are the binary ordering facts future implementation must preserve:

- `ParticleSystemClass__AI` dispatches behavior first, then decrements system lifetime.
- `ParticleClass__AI_Dispatch` dispatches behavior first, then decrements particle lifetime.
- Spark system decrements `SparkSpawnFrames` after optional spawn/light creation.
- Railgun system creates all beam particles before ticking them on later passes.
- Dead particles are pruned by the owning system, not by a global object list.
- Spark/Railgun draw path is single-pixel color write, not `CC_Draw_Shape`.

## Next Narrow Question

The only remaining high-value unknown inside this target is the exact arithmetic for `ParticleType.Normalized` in `ParticleClass__Constructor`. It is verified that the byte gets rewritten, but the decompiler output is not clean enough to safely implement the formula. That should be a tiny follow-up investigation on `0x0062B5E0` assembly/dataflow only.

