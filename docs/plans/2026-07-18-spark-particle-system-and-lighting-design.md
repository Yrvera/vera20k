# Spark Particle System and Screen-Light Design

## Goal

Implement active Yuri's Revenge Spark particle-system semantics in Rust-native
owners, including deterministic burst/particle AI and the verified persistent and
one-frame screen-light paths, without approximating unresolved pixel composition.

## Architecture Context

`ParticleSystemType` and `ParticleType` already parse the stock Spark fields and
resolve `HoldsWhat` through `RuleSet`. `Simulation::particle_systems` is a
`BTreeMap`-backed authoritative store ticked in Phase 5.5. Smoke, Gas, and Fire
already dispatch through `src/sim/particles/system_ai.rs`; Spark and Railgun are
currently rejected by `spawn_particle_system`, no-op in system AI, and skipped by
`src/app_instances/particles.rs`.

Spark state belongs in the existing particle store. Presentation-only light effects
must not become gameplay authority: native persistent spark lights live in a separate
screen-effect vector, are gated by the local detail setting, age independently, and
are drawn by the tactical presentation path. Native one-frame lights are constructed,
drawn, removed, and freed in one presentation call.

Primary evidence:

- `ParticleSystemClass::AI_Spark` at `0x0062E840`, live-decompiled 2026-07-18.
- Spark `ParticleClass` AI at `0x0062C6E0`, live-decompiled 2026-07-18.
- one-frame light helper at `0x0062E280`, live-decompiled and disassembled 2026-07-18.
- light constructor/update/draw at `0x005FF250`, `0x005FF390`, `0x005FF850`, and
  `0x005FFFA0`, live-decompiled 2026-07-18.
- particle draw at `0x0062CEC0`, live-decompiled 2026-07-18.
- `docs/research/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`.
- `docs/research/PARTICLE_SPARK_LIGHTSIZE_ONEFRAMELIGHT_LIFECYCLE_GHIDRA_REPORT.md`.
- `docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` as a source map; the focused
  light-lifecycle report supersedes its stale early interpretation of `0x0062E280`.

## Impact Analysis

Expected implementation surfaces after the blockers below are closed:

- `src/sim/particles/mod.rs`: expose the Spark module and preserve required runtime
  fields.
- `src/sim/particles/spawn.rs`: accept Spark systems and use the Spark-specific
  constructor path.
- `src/sim/particles/system_ai.rs`: dispatch Spark in Phase 5.5.
- `src/sim/particles/spark.rs`: own burst creation, per-particle AI, movement,
  collision, color progression, and deterministic tests.
- `src/rules/particle_type.rs` and the applicable General rules parser: correct only
  fields/defaults proved necessary by the verified constructor and gravity path.
- `src/sim/world/world_hash.rs`: hash every Spark field that affects later state or
  RNG-visible results.
- `src/sim/snapshot.rs`: coordinate particle-system persistence rather than silently
  losing live Spark state. This file is currently owned by another active task.
- app/render presentation modules: consume persistent-light events, project one-frame
  lights, and draw Spark pixels through an exact screen-light/A/Z composition path.

Risks and dependencies:

- `world/mod.rs`, `world_hash.rs`, and `snapshot.rs` currently have in-flight changes
  from the skirmish-shell task. Implementation must wait for that ownership to clear.
- Current particle systems are hashed but skipped by serde snapshots. A live Spark
  cutover must not deepen that inconsistency.
- The renderer uses CPU depth sorting for ordinary particle sprites and has no proven
  equivalent of the native Spark A-buffer/Z-buffer pixel test.
- Stock Spark callers span Techno common AI, Structure AI, capture, warp attack, and
  repair mechanisms. Missing caller owners are prerequisite gaps, not permission to
  fake spawn cadence inside the Spark runtime.

## Chosen Approach

Use the existing deterministic particle store for all state that affects Spark
simulation. Add a Spark-specific module rather than widening the generic Smoke/Gas/
Fire helpers with behavior-dependent branches.

Persistent lighting crosses from simulation to presentation through a typed candidate
event emitted at the verified first-frame creation point. The app applies the local
high-detail gate and owns the short-lived screen-light vector. One-frame lighting stays
presentation-owned and is projected from the current Spark system only when live
particles exist. Neither path uses map/cell lighting.

This is Rust-native structure with gamemd-native semantics: the simulation owns the
particle mechanism and RNG sequence; the app owns local screen effects; the renderer
owns surface composition. The boundary follows existing `SimSoundEvent`/presentation
event patterns without making `sim/` depend on `render/`.

Implementation is blocked until the unresolved evidence and file-ownership gates are
closed. The design does not authorize an approximate first cut.

## Tiny-Detail Ledger

- Spark system dispatch is enum value `3`; particle Spark is also value `3`.
  `[GHIDRA 0x0062FD60, 0x0062CE40]`
- The final Spark frame spawns unconditionally. Earlier frames draw
  `RandomRanged(0, 0x7FFFFFFE)` and compare its normalized value inclusively against
  `SpawnSparkPercentage`. `[GHIDRA 0x0062E840]`
- Eligible bursts draw raw `Random::Next`, use absolute-value modulo
  `ParticleCap / 2`, and add `ParticleCap / 2`. Stock `ParticleCap=6` produces
  `3..=5`, not six. `[GHIDRA 0x0062E840]`
- Directionless systems draw one shared X/Y/Z bias before the particle loop. Every
  stock Spark system is directionless because `SpawnDirection` is absent/zero.
  `[GHIDRA 0x0062E840; ini: rulesmd.ini Spark systems]`
- Each particle is constructed and appended before three per-particle raw velocity
  draws. X and Y use signed raw remainder; Z uses absolute raw modulo plus
  `MinZVelocity`. `[GHIDRA 0x0062E840]`
- After adding the fixed/shared bias, direction is normalized and multiplied by the
  pre-bias magnitude. Zero post-bias magnitude skips division. `[GHIDRA 0x0062E840]`
- Persistent light creation is inside the eligible-spawn branch and requires detail
  value `2`, first Spark frame, positive `LightSize`, and `OneFrameLight=false`; its
  radius is raw `LightSize`. `[GHIDRA 0x0062E840]`
- The system decrements `spark_spawn_frames` after optional spawning/light creation,
  marks itself for deletion below `1`, then draws facing RNG while the pre-decrement
  counter was positive. `[GHIDRA 0x0062E840]`
- Facing subtracts or adds `3` in the outer probability bands and clamps to `0x11`
  and `0x29`; the middle band leaves it unchanged. `[GHIDRA 0x0062E840]`
- Existing particles are dispatched in forward order, then marked particles are
  uninitialized in reverse order. `[GHIDRA 0x0062E840]`
- Spark particle AI subtracts `RulesClass+0x16B8` gravity from Z twice in the recovered
  movement setup, evaluates terrain/bridge/building collision, commits coordinates,
  and then draws color-progression RNG. Exact Rust arithmetic must follow assembly,
  not the summary wording “subtract gravity once.” `[GHIDRA 0x0062C6E0]`
- Color accumulation adds `ColorSpeed + normalized RandomRanged * 0.05`; comparison
  with `1.0` is strict `>`. Intermediate pairs increment the byte index and reset to
  `0`; the final pair clamps the accumulator to exactly `1.0`.
  `[GHIDRA 0x0062C6E0]`
- Generic particle dispatch decrements lifetime after behavior AI and marks deletion
  when it reaches zero. `[GHIDRA 0x0062CE40]`
- One-frame light requires `OneFrameLight=true`, positive `LightSize`, and positive
  live particle count. Its radius is `Math__ftol(clamp(count / ParticleCap, 0.4,
  1.0) * LightSize)`. It seeds stage from system facing, draws immediately, removes,
  and frees the entry. `[GHIDRA assembly 0x0062E28D..0x0062E36E]`
- Persistent light entries initialize stage/flags to zero, age backwards through the
  global vector by `+8`, and are removed when the new stage is greater than `0x4F`.
  `[GHIDRA 0x005FF250, 0x005FF390]`
- Persistent lights are drawn from `TacticalClass_Draw`; their updater is called from
  the live logic-vector tick. `[GHIDRA callers of 0x005FFFA0 and 0x005FF390]`
- Spark particles are single pixels, not SHP frames. Drawing requires a nonzero
  A-buffer value, performs the native Z-buffer inequality, selects start color at
  color index zero or `ColorList[index]` otherwise, brightness-scales RGB when the
  A-buffer value is below `0x7F`, then writes to the primary surface.
  `[GHIDRA 0x0062CEC0]`
- Spark screen lights receive no particle `ColorList`, start color, tint, or alpha.
  `[GHIDRA 0x005FF250, 0x005FF850]`

## Design

### Components

#### Spark simulation module

`src/sim/particles/spark.rs` owns the Spark system and particle AI. It receives the
current system, rules, terrain/bridge queries, and the scenario particle RNG. It does
not read app graphics options or emit render objects.

The module returns typed presentation candidates only for native persistent-light
creation points. A candidate contains tick, coordinates, and raw radius; it does not
contain RGB tint or gameplay state.

#### Presentation light state

An app-owned `ScreenLightEffectStore` keeps insertion order and entries containing
coordinates, stage, radius, and flags. It applies the local detail gate when accepting
a persistent candidate, advances entries exactly once per corresponding logic tick,
removes expired entries with order-preserving compaction, and exposes immutable render
projections.

One-frame lights do not enter this store. The renderer derives them from current Spark
systems with `OneFrameLight=true`, positive `LightSize`, and positive live count, using
the verified facing and radius formula.

#### Spark pixel and light compositor

A dedicated render path must reproduce the native pixel eligibility, A-buffer
brightness, Z-buffer inequality, clipping, and screen-light lookup/composition. Reusing
the SHP particle atlas or map lighting is prohibited. Until that substrate is proved,
the implementation cannot receive a pixel-parity verdict.

### Interfaces / Contracts

- Spark AI may consume RNG only through helpers whose semantics match the exact native
  call at that ledger position: `RandomRanged` versus raw signed/absolute modulo must
  remain distinct.
- Missing or invalid `HoldsWhat` follows verified native failure behavior. The current
  decompile dereferences a null construction result in the ordinary allocation path;
  Rust must not invent a harmless fallback without deciding the malformed/modded-data
  policy explicitly.
- Presentation event handling may not alter simulation state or scenario RNG.
- Local detail settings may suppress screen effects and Spark pixel drawing but may not
  suppress Spark AI, particle lifetime, movement, or RNG consumption.
- New Spark fields that can affect later simulation or RNG-visible state are hashed.
  Presentation light state remains outside the lockstep hash only after its lack of
  gameplay consumers is preserved and tested.

### Data Flow

1. A verified gameplay owner calls `spawn_particle_system` with a Spark type.
2. The system enters `ParticleSystemStore` with parsed countdown, facing `0x1D`,
   directionless state, lifetime, owner, target, and coordinates.
3. Phase 5.5 visits it in stable-ID order and runs Spark system AI.
4. The system performs native eligibility and RNG draws, constructs particles, emits a
   persistent-light candidate when gated, decrements its counter, jitters facing, ticks
   particles forward, and removes dead particles in reverse.
5. Generic system AI decrements the system lifetime and retains or drops the system by
   the existing lifecycle rule.
6. The app drains presentation candidates after each simulation tick, applies local
   detail policy, and advances persistent light stages at the verified ordering point.
7. Rendering projects Spark pixels, persistent lights, and eligible one-frame lights
   without mutating simulation or consuming RNG.

### Error Handling

- Invalid stock metadata is a load/initialization error, not a one-pixel or one-particle
  fallback.
- Malformed mod behavior that the binary would crash on remains an explicit policy
  decision; it must not be mislabeled parity.
- Renderer unavailability or low detail suppresses presentation only.
- Missing A/Z composition support blocks the visual cutover rather than selecting an
  approximate passthrough pipeline.

### Testing Strategy

- Parse stock `SparkSys`, `WeldingSys`, and `LGSparkSys` fields from the merged
  `rules.ini`/`rulesmd.ini` authority.
- Branch-complete burst tests: final frame, probability fail/succeed, single-frame
  systems, odd caps, and native malformed-cap policy.
- Exact RNG-state tests after each branch, including shared directionless draws,
  constructor color draw, per-particle velocity draws, facing draw, and color draws.
- Vector fixtures for direction normalization, magnitude restoration, facing clamps,
  gravity, terrain/bridge/building collision, coordinate conversion, and deletion.
- Color fixtures for index zero, intermediate pairs, final pair, strict `> 1.0`, and
  empty/single-entry policy.
- Persistent-light tests for high-detail first-frame creation, raw radius, stage
  sequence `0,8,...,72`, and removal on the update to `80`.
- One-frame tests for live-count gate, facing seed, clamp/radius formula, immediate
  draw/remove behavior, and absence from persistent state.
- Hash tests for every newly consumed Spark simulation field and negative tests proving
  local detail/presentation state cannot shift lockstep state.
- Snapshot round-trip once the particle store is included in the coordinated snapshot
  contract.
- Retail-derived pixel fixtures for A-buffer zero/nonzero, brightness `<0x7F`, Z pass/
  fail, clipping, start color, ColorList color, and light-stage lookup. Hand-computed
  goldens cannot certify pixel parity.

## Implementation Blockers

1. Verify the exact first-visible persistent-light stage relative to creation,
   `FUN_005FF390`, and `TacticalClass_Draw`. Creation/update/draw functions are known;
   the same-frame ordering remains unproved.
2. Bind `Math__ftol` for fractional one-frame radii to the project's exact signed
   conversion helper with a concrete fractional fixture.
3. Produce an implementation contract for the Spark particle A-buffer/Z-buffer pixel
   path, including the Rust render surface that can preserve it.
4. Decode the Spark collision/matrix path into a coordinate-frame contract and walk at
   least one ground, bridge-crossing, and building-contact fixture.
5. Wait for the active skirmish task to release `world/mod.rs`, `world_hash.rs`, and
   `snapshot.rs`, then re-read their final state before planning.

## Architectural Decisions

- Reuse `ParticleSystemStore`; do not add an ECS or a parallel Spark registry.
- Use a Spark-specific AI module because its constructor, movement, color, draw, and
  light mechanisms are materially different from Smoke/Gas/Fire.
- Keep local screen-light state above `sim/`; local detail options must not affect
  lockstep hashes.
- Preserve presentation events as typed data, following existing simulation-to-app
  event patterns.
- Do not route screen lights through `src/map/lighting.rs`; binary evidence proves a
  separate primary-surface effect path.
- Do not combine Railgun into this design. Railgun is a distinct path-burst and laser
  mechanism with different creation and movement semantics.

## Alternatives Considered

### Store screen lights in hashed simulation

Rejected. Persistent creation is gated by a local graphics/detail setting and the
light vector has no verified gameplay consumers. Hashing it would allow local display
settings to create cross-client state divergence.

### Derive all lights from live Spark systems at render time

Rejected. Persistent lights have an independent stage/lifetime and can no longer be
reconstructed exactly from the current Spark system after creation. This would change
their duration and draw stage.

### Render Spark as a one-pixel sprite in the existing passthrough pass

Rejected as parity. Native drawing reads the A-buffer, evaluates a Z-buffer inequality,
brightness-scales RGB, and writes directly to the primary surface. A plain sprite can
be a diagnostic visualization only, never the production parity path.
