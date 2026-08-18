# Spark Tick RNG Ordering Fix Design

## Goal

Restore active-YR behavior-3 particle ordering so persistent motion and collision
state commits before color RNG, while lifetime remains after color progression.

## Architecture Context

`ParticleSystem` owns its `Vec<Particle>`, while `Simulation` owns the shared
scenario RNG exposed to particle code through `particle_rng()`. The behavior-3
compatibility kernel lives in `src/sim/particles/spark.rs`; public Spark spawning
and production dispatch remain gated. `tick_particle_with_facts` now owns the RNG
call, but a post-implementation fidelity check found two remaining stage-order
drifts: persistent Z is committed only after the complete movement helper, and
coordinates are committed before the collision deletion byte.

The fix follows the established Smoke/Gas/Fire pattern: simulation code receives
the authoritative `SimRng` borrow and consumes it at the verified point. It does
not introduce a renderer dependency or change particle-system ownership.

## Impact Analysis

- `src/sim/particles/spark.rs`: split movement calculation and collision
  resolution into private ordered stages while keeping the public input/result
  contract stable.
- Spark unit tests in the same file: provide a deterministic `SimRng` and assert
  state/RNG/lifetime ordering.
- No production caller migration is required because the API is currently used
  only by its focused tests.
- Public Spark activation, owner forward/reverse traversal, terrain fact
  production, and tactical point integration remain separate prerequisites.

The determinism risk is shared-RNG cursor movement. Tests must compare complete
RNG state against one call to the verified ranged helper, including collision
ticks.

## Chosen Approach

Keep `&mut SimRng` on `tick_particle_with_facts`, but introduce private staged
helpers. Compute and commit persistent Z first; then reproduce the native
coordinate store/conversion sequence. Collision resolution returns a decision
containing selected raw `f32` coordinate bits before final conversion. The tick
sets deletion from that decision, performs final Z/Y/X `ftol` conversion, commits
coordinates, and calls
`next_range_u32_inclusive(0, 0x7FFF_FFFE)` exactly once at the RandomRanged API
level, passes the result to the existing x87-compatible color helper, and only
then decrements lifetime.

This keeps the authoritative RNG owner explicit, follows existing particle code,
and prevents callers from producing the sample too early.

## Tiny-Detail Ledger

- Store persistent `old_vz - Gravity` as native-compatible `f32` before local
  coordinate/probe/collision work. [doc:
  `PARTICLE_SPARK_COLLISION_AND_PIXEL_COMPOSITOR_GHIDRA_REPORT.md` section
  "x87 control word and memory-rounding boundaries";
  Ghidra `0x0062C705..0x0062C71C`]
- Preserve component order: local coordinate stores X/Y/Z, probe-Z store, old
  `ftol` Z/Y/X, vector add X/Y/Z, candidate `ftol` Z/Y/X. [same doc section;
  Ghidra `0x0062C740..0x0062C7C5`]
- For a collision, store the deletion byte before final selected-coordinate
  `ftol` Z/Y/X and coordinate commit. [same doc section "Commit, RNG, lifetime,
  and cleanup ordering"; Ghidra `0x0062CA34..0x0062CA6C`]
- A collision deletion marker does not suppress the same-tick color draw.
  [same doc lines 249-253, 274-284]
- Request the inclusive range `0..=0x7FFF_FFFE` from the authoritative particle
  gameplay RNG. [Ghidra `0x0062CA72..0x0062CA86`]
- Preserve color arithmetic association:
  `((sample * reciprocal) * 0.05) + ColorSpeed + old accumulator`, with the
  existing native store boundaries. [approved Spark design lines 194-200;
  Ghidra `0x0062CA95..0x0062CAB1`]
- Use strict `accumulator > 1.0`; use signed `index < count - 2`; otherwise store
  exact `1.0`. [approved Spark design lines 198-200]
- Wrapping signed-`i16` lifetime decrement follows color. Only a resulting zero
  marks deletion; starting zero becomes `-1`. [collision report lines 249-253;
  Ghidra `0x0062CE86`]
- Reverse cleanup remains outside this helper and must eventually run after all
  forward particle ticks in the same owner tick. [collision report lines 251-253;
  Ghidra `0x0062E840`]
- The target is the active standard-YR behavior-3 path, not dormant TS logic.
  [collision report coverage ledger lines 494-499]

## Design

### Components

- `SparkTickInputs` continues to contain immutable movement, collision, and color
  facts, but no RNG output.
- `tick_particle_with_facts` receives `&mut SimRng`, owns ordering, and returns the
  existing diagnostic result.
- Private movement stages separate persistent-Z calculation from the remaining
  coordinate/probe work; private collision stages separate decision/reflection
  from final coordinate conversion.
- `advance_color` remains a sample-driven arithmetic helper so its exhaustive
  arithmetic tests stay independent of the RNG implementation.

### Interfaces / Contracts

The tick wrapper is the smallest public unit allowed to combine particle state
and RNG. Callers cannot pass a pre-consumed sample. The eventual Spark owner must
borrow `Simulation::particle_rng()` and call the wrapper once per particle in
native forward order.

### Data Flow

1. Compute and immediately commit persistent Z velocity.
2. Store old coordinates X/Y/Z as `f32`, store probe Z, run old `ftol` Z/Y/X,
   vector-add X/Y/Z, and run candidate `ftol` Z/Y/X.
3. Resolve collision decision/reflection from injected authoritative facts.
4. If colliding, write deletion.
5. Run final selected-coordinate `ftol` Z/Y/X and commit coordinates.
6. Draw the color sample from `SimRng`.
7. Mutate the committed Spark color state.
8. Decrement lifetime and conditionally write deletion on equality zero.

### Error Handling

Existing checked-kernel errors remain explicit. If color validation/arithmetic
fails after the RNG point, earlier native-ordered movement/collision mutations
and the consumed RNG state remain committed, while lifetime remains unchanged.
If movement fails after persistent-Z calculation, that earlier native write
remains committed. If final coordinate conversion fails after a collision
decision, the collision deletion write remains committed.
This is the closest safe-Rust ordering model for invalid states even though
native malformed-state behavior is not parity-certified.

### Testing Strategy

- Update the flat-ground trace to assert committed coordinate, persistent Z
  velocity, deletion, one ranged RNG call, and subsequent lifetime decrement.
- Add an invalid-color-count regression proving movement/collision and RNG are
  committed before the error while lifetime is not decremented afterward.
- Add staged regressions proving persistent Z survives a later movement error
  and collision deletion precedes final conversion/coordinate commit.
- Pin X/Y/Z store and Z/Y/X conversion ordering with deliberately invalid later
  components where the checked arithmetic surface makes the stage visible.
- Retain bridge equality/asymmetry, deep/near-ground, occupancy, x87 arithmetic,
  and color-boundary tests.
- Run the focused Spark test module, then a serial `cargo check -q` after
  confirming no other Cargo owner is active.

## Architectural Decisions

The design follows the existing direct `&mut SimRng` particle-system pattern and
keeps all behavior in `sim/`. Ordered helpers remain private so callers cannot
rearrange persistent-Z, delete, coordinate, color, or lifetime stages. The public
pure `integrate_motion` and `resolve_collision` conveniences compose the same
private stages for focused arithmetic tests. No new dependency or persistent
field is introduced.

## Alternatives Considered

- An RNG callback would defer the sample correctly but obscure the authoritative
  RNG route and permit arbitrary sampling semantics.
- Separate public movement, color, and lifetime stages would be easy to test but
  would allow future callers to reorder them and recreate the parity defect.
- A monolithic tick rewrite could match the assembly but would duplicate the
  existing independently tested arithmetic and collision formulas.
- A deferred event/commit list would preserve final values but reproduce the same
  wrong write timing and is therefore DRIFT.
- Keeping the pre-produced sample cannot satisfy the verified mechanism because
  sample consumption necessarily occurs before the tick wrapper's commits.
