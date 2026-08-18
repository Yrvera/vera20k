# Scheduler-Backed AnimClass Damage Fire Implementation Plan

> Execution note: implement this plan task-by-task. Re-read the named files and
> preserve unrelated local edits before each task. Do not broaden the slice into a
> general animation rewrite.

**Goal:** Replace building damage-fire's app-side overlay approximation with the
smallest sim-owned `AnimClass` mechanism that preserves verified active
`gamemd.exe` semantics and can later host other ordinary animations.

**Architecture:** Add an `AnimStore` beside `EntityStore`; both use the existing
global stable-ID allocator and `LogicVector`. `object_ai_walk` dispatches each live
ID to either a game entity or an animation. Buildings retain eight animation IDs,
while rendering and audio consume immutable sim state/events. Art/asset metadata is
resolved before simulation begins, so simulation contains no filesystem access or
floating-point math.

**Approved design:**
`docs/plans/2026-07-18-scheduler-backed-animclass-damage-fire-design.md`

## Scope Guard

This plan implements only:

- one generic ordinary-SHP `AnimObject` runtime;
- shared scheduling, lifecycle, snapshot, and hashing support;
- `Next`, trailer, delay/rate, loop, ping-pong, reverse, start/report sound, and
  stop-sound identity needed by the verified runtime contract;
- building damage fire as the first authoritative consumer;
- renderer/audio adapters needed to retire the old damage-fire overlay path.

It does not implement combat animations, debris, particles, palette animation,
voxel animations, owner/remap behavior, or complete mixer equivalence. It does not
claim parity for arbitrary nonstock `ConditionYellow`/`ConditionRed` values; map
initialization rejects values outside the certified stock ratios for this slice.

## Grounding and Confidence

| Decision | Evidence | Confidence |
|---|---|---|
| `AnimClass` participates in the same live logic vector and may append a trailer that is visited later in the same pass | `ANIMCLASS_AI_FIRST_SAFE_MIGRATION_SLICE_GHIDRA_REPORT.md`; `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md` | VERIFIED |
| Constructor reveal precedes delay-zero `Middle` processing | live decompile/assembly at `0x00421EA0`, recorded in the approved design/research | VERIFIED |
| SHP frame count supplies omitted `End`; shadow halves the loaded count; explicit `End`/`LoopEnd` are applied afterward; `-1` means last loaded frame | `ANIMTYPE_END_LOAD_DAMAGE_FIRE_FRAME_RNG_GHIDRA_REPORT.md` | VERIFIED |
| Stock `FIRE01`, `FIRE02`, and `FIRE03` raw frame counts are 30, 64, and 30 | retail SHPs plus the End-load report | VERIFIED |
| Damage-fire selection, initial type/frame RNG order, slot ordering, fixed eight-slot ownership, threshold cache, and cleanup behavior | `ANIMCLASS_BUILDING_OBJECT_DAMAGE_RUNTIME_SPAWNS_GHIDRA_REPORT.md` | VERIFIED |
| Ordinary and occupiable buildings use the yellow and red thresholds respectively | same runtime-spawn report plus stock `rulesmd.ini` | VERIFIED for stock 1/2 and 1/4 |
| Art offsets are pixel pairs transformed by the tactical isometric matrix, then truncated toward zero; building render origin subtracts 128 leptons from X/Y | `FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`, including `0x006D2070`, `0x006D1C20`, `0x005AFB80`, and `0x007C5F00` | VERIFIED |
| Damage-fire depth uses the verified signed formula and clamps with `min(0)` | `ANIMCLASS_DRAWIT_ZADJUST_DEPTH_GHIDRA_REPORT.md` and runtime-spawn report | VERIFIED |
| Current app-side overlay runtime is not an acceptable parity mechanism | direct inspection of `src/app_building_anim.rs` | CONFIRMED Rust drift |

### Explicitly deferred uncertainty

- A renderer screenshot can regression-test the new slot lookup, but pixel-perfect
  damage-fire composition remains `UNCHECKED` until compared to a retail-derived
  executable capture.
- Mixer internals remain outside this slice. This plan preserves per-animation
  start/stop identity and event order, which is the prerequisite for later audio
  fidelity work.
- Arbitrary nonstock damage thresholds remain rejected until native floating-point
  comparison equivalence is proven for the full accepted input domain.

## Current-Work Coordination

The worktree already contains unrelated edits in several overlapping files,
including `src/app_init.rs`, `src/sim/snapshot.rs`, `src/sim/world/mod.rs`, and
`src/sim/world/world_hash.rs`. At the start of every task:

1. Run `git diff -- <task files>` and re-read the current file contents.
2. Preserve session/player-start, house-difficulty, RNG-accessor, and formatting
   changes that are not part of this plan.
3. Treat `SNAPSHOT_VERSION` as dynamic: increment the accepted value once when the
   final persisted shape lands; do not assume a particular starting number.
4. Never run crate-wide formatting. Format only edited Rust files with edition
   2024 and inspect their diffs.

## File Map

| File | Planned responsibility |
|---|---|
| `src/rules/art_data.rs` | Presence-aware animation bounds, raw SHP metadata binding, damage-fire offset preprocessing, reachable animation closure |
| `src/rules/ruleset.rs` | Certified damage-fire health ratios derived from parsed stock rules |
| `src/app_init.rs` | Bind and validate all required animation assets/metadata before simulation starts |
| `src/render/sprite_atlas.rs` | Pack the complete raw-frame range for scheduler-owned animation types |
| `src/sim/anim_class.rs` | `AnimObject`, `AnimRuntime`, `AnimStore`, constructor, AI visit, transition, trailer, and destruction mechanics |
| `src/sim/anim_class_tests.rs` | Focused mechanism and scheduler tests |
| `src/sim/mod.rs` | Export the new module and include its test module |
| `src/sim/world/substrate.rs` | Own `AnimStore` beside `EntityStore` |
| `src/sim/world/mod.rs` | Global IDs, registration, reveal/conceal/delete, load rebuild, sound events |
| `src/sim/world/techno_ai.rs` | Resolve each live ID as entity or animation; run building damage-fire state check |
| `src/sim/game_entity.rs` | Eight fixed damage-fire IDs and cached active-state byte |
| `src/sim/world/world_hash.rs` | Hash animation registry/runtime and building-owned IDs/cache |
| `src/sim/snapshot.rs` | Persist the new state and update round-trip/version tests |
| `src/app_instances/overlays.rs` | Build fire draw instances from sim animation IDs |
| `src/app_sim_tick.rs` | Remove the app-side damage-fire tick and translate animation sound events |
| `src/audio/events.rs` | Carry animation ID on start/stop events |
| `src/audio/sfx.rs` | Track and stop the mixer handle belonging to one animation ID |
| `src/app_building_anim.rs` | Delete the retired damage-fire overlay runtime after cutover |
| `src/sim/components.rs` | Delete transitional damage-fire runtime structs; retain unrelated presentation types |

## Core Interfaces

Use a Rust-native registry and the verified gamemd-visible state. Do not introduce
an ECS, raw pointers, a second scheduler, or a separately maintained compiled
animation catalog.

```rust
pub type AnimId = u64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimRuntime {
    pub current_frame: i32,
    pub frame_step: i32,
    pub delay_remaining: u16,
    pub rate_reload: u16,
    pub rate_elapsed: u16,
    pub loop_remaining: u8,
    pub first_ai_guard: bool,
    pub constructor_reverse: bool,
    pub inactive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimObject {
    pub stable_id: AnimId,
    pub type_id: InternedId,
    pub world_coord: glam::IVec3,
    pub draw_flags: u32,
    pub z_adjust: i32,
    pub effective_end: i32,
    pub effective_loop_end: i32,
    pub runtime: AnimRuntime,
    pub in_logic_vector: bool,
    pub start_sound_active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimStore(BTreeMap<AnimId, AnimObject>);
```

`AnimStore` exposes only deterministic `get`, `get_mut`, `insert`, `remove`, and
ordered iteration. `AnimId` shares the entity allocator; an ID must exist in
exactly one store. An inactive animation remains resolvable until deferred delete
flush, matching the scheduler's compaction/deletion separation.

The building fields are fixed-size authoritative state:

```rust
pub damage_fire_state_active: bool,
pub damage_fire_anim_ids: [Option<AnimId>; 8],
```

The old vector-backed `DamageFireOverlays` is not retained as a compatibility
mirror.

## Task 1: Bind Exact Animation and Damage-Fire Metadata

**Why first:** every runtime task depends on authoritative loaded frame bounds,
coordinates, thresholds, and the animation closure. Resolving these at startup
keeps asset access and floating point out of simulation.

**Files:**

- Modify `src/rules/art_data.rs`
- Modify `src/rules/ruleset.rs`
- Modify `src/app_init.rs`
- Modify `src/render/sprite_atlas.rs`

**Steps:**

1. Replace `(i32, i32)` damage-fire offsets with a serializable value that retains
   the source pixels for depth and precomputes the signed world-lepton delta:

   ```rust
   pub struct DamageFireOffset {
       pub pixel_x: i32,
       pub pixel_y: i32,
       pub world_dx: i32,
       pub world_dy: i32,
   }
   ```

2. Add presence-aware fields to `AnimTypeRuntimeConfig`:

   ```rust
   pub explicit_end: Option<i32>,
   pub explicit_loop_end: Option<i32>,
   pub raw_shp_frame_count: Option<i32>,
   pub loaded_end: i32,
   pub loaded_loop_end: i32,
   ```

   Do not use `unwrap_or(0)` to erase key presence. Parse signed values, including
   explicit `-1`.

3. Add one initialization binder on `ArtRegistry`. For every animation reachable
   from `DamageFireTypes` through `Next` and `Trailer`, it must:

   - resolve the retail SHP using the existing candidate/name rules;
   - read the signed 16-bit header count from bytes 6..8;
   - resolve native load order exactly: fresh zero values, header-derived End when
     zero, shadow halving, LoopEnd fallback to End, then explicit End/LoopEnd;
   - record the raw count and resolved loaded fields;
   - reject missing assets, negative raw counts, or impossible frame bounds;
   - traverse cycles safely with a visited set, because valid `Next` graphs may
     loop back to an already resolved type;
   - store the closure in a `BTreeSet<String>` on `ArtRegistry` for atlas packing.

   Centralize the rules in a pure helper and test it directly:

   ```rust
   fn resolve_loaded_bounds(
       raw_count: i32,
       shadow: bool,
       explicit_end: Option<i32>,
       explicit_loop_end: Option<i32>,
   ) -> Result<(i32, i32), ArtDataError> {
       let header_end = if shadow { raw_count / 2 } else { raw_count };
       let fallback_loop_end = header_end;
       let loaded_end = explicit_end.unwrap_or(header_end);
       let loaded_loop_end = explicit_loop_end.unwrap_or(fallback_loop_end);
       validate_loaded_bounds(raw_count, loaded_end, loaded_loop_end)?;
       Ok((loaded_end, loaded_loop_end))
   }
   ```

   The validator must allow `-1` only in the explicit native sentinel positions;
   object construction resolves it to the final loaded frame.

4. Precompute each art offset with the verified tactical matrix. Use the exact
   coefficient bits `0x408888CE`; require source/intermediate integers to be exactly
   representable as `f32`; perform the final `f32` product and truncate toward zero.
   Reject overflow or non-finite results. This computation belongs in rules/asset
   initialization, never `sim/`.

   ```rust
   const ISO_PIXEL_TO_WORLD_A: f32 = f32::from_bits(0x4088_88CE);

   fn damage_fire_world_delta(px: i32, py: i32) -> Result<(i32, i32), ArtDataError> {
       let twice_y = py.checked_mul(2).ok_or(ArtDataError::OffsetOverflow)?;
       let native_x = px.checked_add(twice_y).ok_or(ArtDataError::OffsetOverflow)?;
       let native_y = px.checked_neg()
           .and_then(|v| v.checked_add(twice_y))
           .ok_or(ArtDataError::OffsetOverflow)?;
       validate_exact_f32_integer(px)?;
       validate_exact_f32_integer(py)?;
       validate_exact_f32_integer(native_x)?;
       validate_exact_f32_integer(native_y)?;
       Ok((
           (ISO_PIXEL_TO_WORLD_A * native_x as f32).trunc() as i32,
           (ISO_PIXEL_TO_WORLD_A * native_y as f32).trunc() as i32,
       ))
   }
   ```

5. Parse `ConditionYellow` and `ConditionRed` through the existing CCINI-compatible
   `read_double`, then derive a `DamageFireHealthRatios` rule value only when the
   parsed numbers are exactly the certified stock `1/2` and `1/4`. Surface an
   initialization error otherwise. Retain any `f32` convenience fields used solely
   by rendering/UI, but simulation reads only the integer ratios.

6. Call the binder in `app_init` after INI merge and before cloning the canonical
   `ArtRegistry` into `RuleSet` or building the atlas.

7. In the atlas, pack the complete raw frame range for all types in the resolved
   scheduler-owned closure. Do not divide those counts by two. Leave unrelated
   transitional `WorldEffect` packing unchanged until its own migration. Stop
   publishing atlas-derived half-counts as gameplay bounds for the new runtime.

**Focused tests:**

- Omitted End resolves retail `FIRE01/FIRE02/FIRE03` to 30/64/30 loaded frames.
- Explicit End overrides header End; explicit LoopEnd overrides its fallback.
- Explicit `End=0` remains distinct from omission and suppresses frame RNG.
- Explicit `-1` survives loading and resolves only at object construction.
- Shadow halving occurs before explicit overrides.
- `Next`/trailer closure order is deterministic and rejects a missing asset.
- `(-24, -1)` and `(64, 36)` produce fixed expected world deltas from the verified
  matrix; negative products truncate toward zero.
- Stock thresholds produce `(1,2)` and `(1,4)`; unsupported nonstock values fail
  initialization.
- Atlas exposes frame 63 for `FIRE02`.

**Verification:** run the focused `rules::art_data`, ruleset, app-init validation,
and sprite-atlas tests serially.

## Task 2: Add the Ordinary AnimObject Runtime

**Files:**

- Create `src/sim/anim_class.rs`
- Create `src/sim/anim_class_tests.rs`
- Modify `src/sim/mod.rs`
- Modify `src/sim/components.rs`

**Steps:**

1. Add the core interfaces above and `AnimClassSpawnDescriptor` consumption using
   `InternedId`, integer world coordinates, logic-frame timers, and fixed-width
   fields. Remove `elapsed_logic_ms` from authoritative animation state.

2. Move the reusable ordinary-animation mechanics currently embedded in
   `app_building_anim.rs` into `sim/anim_class.rs`; preserve their verified order,
   but do not copy app ownership or wall-clock accumulation.

3. Implement `Simulation::spawn_anim_object(rules, descriptor) -> Result<AnimId,
   AnimSpawnError>` in this exact constructor order:

   - allocate the shared stable ID;
   - resolve loaded End/LoopEnd, including `-1` against the raw frame count;
   - initialize direction, loop byte, rate/delay, draw flags, and z-adjust;
   - insert into `AnimStore`;
   - reveal/register it in `LogicVector`;
   - if constructor delay is zero, run `Middle` immediately, including
     start/report sound event emission;
   - return the ID.

   Constructor failure must occur before scheduler insertion. Do not leave a
   partially registered object.

   Convert art rate exactly at construction: `Rate > 0` becomes integer
   `900 / Rate`; `Rate <= 0` becomes zero and prevents normal advancement. Compute
   the remaining-loop byte with native wrapping multiplication of the constructor
   loop byte and AnimType loop count, clamp wrapped values below two to one, and
   preserve `0xFF` as infinite.

4. Implement one `visit_anim(rules, id)` function with the verified order:

   - trailer eligibility and child construction from the committed binary frame;
   - first-AI guard;
   - inactive/delay/rate checks;
   - frame advance;
   - ping-pong/reverse handling;
   - loop decrement/random-loop delay;
   - in-place `Next` type transition followed by `Middle`, or destruction.

5. Trailer descriptors use the verified values: same world coordinate, delay 1,
   loop byte 1, flags `0x600`, z-adjust 0, non-reverse. A trailer appended to the
   logic vector may be visited later in the same live walk; its first-AI guard must
   prevent an unintended frame advance.

6. Use scenario RNG only where the verified config enables random rate or loop
   delay. Stock damage fires do not consume hidden constructor draws.

7. Implement `destroy_anim(id)` as an idempotent transition: emit one animation
   stop/release event, compact it from `LogicVector`, mark inactive, and enqueue the
   ID for deferred deletion. Do not remove it immediately.

8. Keep `AnimRuntime` used by garrison muzzle flash only if that presentation path
   can use the same pure frame-transition helper without gaining sim authority.
   Otherwise rename its local presentation state. Delete only the transitional
   damage-fire structs from `components.rs` at final cutover.

**Focused tests:**

- Reveal precedes delay-zero Middle/start event.
- First live visit consumes only the guard.
- Delay/rate use logic frames, not elapsed milliseconds.
- Forward, reverse, ping-pong, finite loop, infinite loop-byte, and `Next`
  transitions preserve exact boundary frames.
- Constructor loop multiplication wraps as a byte, clamps wrapped values below two
  to one, and preserves `0xFF` as infinite.
- Trailer append is visible in the same live walk but guarded on its first visit.
- Random features consume exactly their documented scenario RNG calls; disabled
  features consume none.
- Destruction emits one stop/release event and is idempotent.

## Task 3: Integrate AnimStore with Global IDs and LogicVector

**Files:**

- Modify `src/sim/world/substrate.rs`
- Modify `src/sim/world/mod.rs`
- Modify `src/sim/anim_class_tests.rs`

**Steps:**

1. Add `anims: AnimStore` to `ObjectSubstrate`.

2. Rename `next_stable_entity_id` to a neutral global object allocator name and
   update all callers/tests. Do not introduce type-tagged IDs or a second counter.

3. Generalize registration/concealment helpers so they update the matching store's
   `in_logic_vector` bit and the one shared `LogicVector`. Assert in debug builds
   that an ID belongs to exactly one store.

4. Extend deferred deletion flush to remove the ID from exactly one registry.

5. Extend `rebuild_logic_membership` after snapshot load to reconcile both stores
   against the serialized scheduler. Preserve scheduler order as serialized; do not
   rebuild it from BTreeMap iteration.

6. Retain the live-walk rule that re-reads scheduler length after each visit and
   responds correctly to tail append and current-entry compaction.

**Focused tests:**

- Entity and animation allocation is one monotonic ID sequence.
- Duplicate membership across stores fails the invariant.
- Current-entry deletion visits the shifted successor exactly once.
- Tail-appended animation is visited in the same pass.
- Deferred-delete objects remain resolvable until flush and disappear afterward.
- Snapshot membership rebuild repairs flags without changing serialized order.

## Task 4: Dispatch Live Animation AI and Define Sound Identity Events

**Files:**

- Modify `src/sim/world/techno_ai.rs`
- Modify `src/sim/world/mod.rs`
- Modify `src/audio/events.rs`
- Modify `src/sim/anim_class_tests.rs`

**Steps:**

1. Change `object_ai_walk` to resolve each live ID once:

   ```rust
   if self.substrate.entities.contains_key(&id) {
       self.techno_ai_shell(id, rules);
   } else if self.substrate.anims.contains_key(&id) {
       self.visit_anim(id, rules);
   }
   ```

   Preserve the existing entity path and record animation IDs in the debug live
   order output. A missing live ID is a deterministic release-mode no-op; the
   existing tick-boundary invariant check must expose it in debug builds without
   consuming replacement RNG.

2. Add sim sound variants carrying animation identity, interned sound identity,
   and world position:

   ```rust
   AnimationStarted { anim_id: AnimId, sound_id: InternedId, world: IVec3 },
   AnimationStopped {
       anim_id: AnimId,
       stop_sound_id: Option<InternedId>,
       world: IVec3,
   },
   ```

3. `Middle` emits start sound or the verified Report fallback once. Destruction
   emits `AnimationStopped` once even when no StopSound is configured, because the
   app still must release the current handle.

4. Sound queues are transient outputs: snapshots preserve the state needed to
   continue an animation but do not replay previously drained start/stop events.

**Focused tests:** mixed entity/animation order, same-pass trailer order, one start
event, one stop event, Report fallback, and no event replay after snapshot restore.

## Task 5: Make Building Damage Fire Authoritative

**Files:**

- Modify `src/sim/game_entity.rs`
- Modify `src/sim/world/techno_ai.rs`
- Modify `src/sim/world/mod.rs`
- Modify `src/sim/anim_class.rs`
- Modify `src/sim/anim_class_tests.rs`

**Steps:**

1. Add the fixed active-state byte and eight fixed animation IDs to
   `GameEntity`; initialize both empty/false.

2. Call `update_building_damage_fire` at the beginning of the Structure arm in
   `techno_ai_shell`, before any later building-specific work.

3. Compute the threshold without floating point:

   ```rust
   let ratio = if object_type.can_be_occupied {
       rules.general.damage_fire_occupied_ratio // 1/4 after validation
   } else {
       rules.general.damage_fire_ordinary_ratio // 1/2 after validation
   };
   let active = i64::from(health) * i64::from(ratio.denominator)
       <= i64::from(max_health) * i64::from(ratio.numerator);
   ```

   Use native signed widths/guards established in research for invalid or zero max
   health. Do not use the old x1000 approximation.

4. Compare the result to `damage_fire_state_active`. If unchanged, return without
   RNG, allocation, or slot mutation. Update the cached byte before performing the
   transition effects.

5. On false-to-true, return before RNG and slot scanning when `DamageFireTypes` is
   empty; the cached state remains updated. Otherwise make the verified inclusive
   initial type roll before inspecting slots. Equal bounds still call the ranged
   helper but consume no raw RNG word. Then walk art offsets 0..7 in order:

   - return from the creation routine at the first missing/sentinel offset or the
     first occupied slot; never skip a hole or refill a later slot;
   - use the current selected `DamageFireTypes` entry;
   - pass the verified constructor row: delay 0, loop byte 1, draw flags `0x600`,
     facing 0, z 0, and non-reverse;
   - build the world position from the building absolute lepton position, subtract
     128 leptons from X/Y for render origin, add the precomputed world delta, and
     convert building height levels with the named 128-lepton constant;
   - construct/reveal the animation and store its ID in the slot;
   - set z-adjust with wrapping 32-bit signed arithmetic and arithmetic shift:

     ```rust
     let foundation_sum = i32::from(foundation_w)
         .wrapping_add(i32::from(foundation_h));
     let scaled = offset.pixel_y
         .wrapping_sub(foundation_sum.wrapping_mul(15))
         .wrapping_mul(3);
     let depth = (scaled >> 1).wrapping_sub(10).min(0);
     ```

   - for every positive effective End, call inclusive
     `RandomRanged(0, End - 1)` and assign the result; `End == 1` therefore calls
     the helper with equal bounds but consumes no raw word, while non-positive End
     performs no call;
   - advance the selected type for the next successful slot.

   Do not consume a frame RNG call when End is 0 or 1. Do not hardcode fire names,
   counts, offsets, or frame counts.

6. On true-to-false, destroy every owned animation in slot order and clear all
   eight IDs.

7. The zero-health/destruction update path clears rather than creates fire even
   though zero is below the threshold. Call the same idempotent slot cleanup at the
   start of centralized entity `uninit`, so sell, death, limbo/uninit, and deletion
   cannot orphan animations. Owner change alone must not detach, recreate, or
   attach the fixed-world animations to an owner object.

8. Any impossible spawn failure after initial validation is an invariant failure;
   do not silently continue with a partial authoritative state.

**Focused tests:**

- Ordinary threshold boundary is exactly 50%; occupiable boundary exactly 25%.
- Cached unchanged state consumes no RNG and creates/destroys nothing.
- Initial type draw precedes slot work; successful slots use type/frame RNG in the
  verified interleaving; equal-bound calls consume no raw word; tests compare the
  complete resulting scenario RNG state so rejection sampling is covered.
- Empty `DamageFireTypes` updates the cache but performs no RNG call; missing art or
  an occupied slot stops the scan immediately without visiting later slots.
- Coordinates match the two stock GACNST offset fixtures and include the -128/-128
  origin adjustment.
- Z-adjust matches negative, zero-clamp, and signed-shift fixtures.
- Damage recovery, sell, death, and uninit clear all slots in order without double
  stop events; zero-health update clears rather than creates; owner change preserves
  them.
- Repair later in the same building update does not clear fires until the next
  building live visit.

## Task 6: Persist and Hash the New Authoritative State

**Files:**

- Modify `src/sim/world/world_hash.rs`
- Modify `src/sim/snapshot.rs`
- Modify focused snapshot/hash tests

**Steps:**

1. Hash `AnimStore` in stable BTreeMap order, including IDs, type IDs, coordinates,
   effective bounds, runtime fields, flags, z-adjust, membership, and sound-active
   identity state.

2. Hash the building active-state byte and each of the eight optional IDs in index
   order.

3. Serialize the animation registry and new entity fields through existing serde
   state. Keep pending sound events transient.

4. After all persisted fields in this plan have landed, increment the then-current
   `SNAPSHOT_VERSION` exactly once and update the literal-version test. Coordinate
   with the existing house-difficulty snapshot edit instead of overwriting it.

**Focused tests:**

- Round-trip a mixed entity/animation scheduler at a supported save boundary and
  assert that the pending-delete queue is empty there.
- Round-trip a building with sparse fixed slots and an active cached state.
- Loaded world hash equals pre-save hash.
- Changing one runtime byte, effective End, slot index, or scheduler position
  changes the hash.
- Snapshot restore does not replay start/stop audio events.

## Task 7: Cut Rendering Over to Sim Animation IDs

**Files:**

- Modify `src/app_instances/overlays.rs`
- Modify `src/app_sim_tick.rs`
- Modify `src/app_building_anim.rs`
- Modify `src/sim/components.rs`
- Modify renderer/app tests

**Steps:**

1. Replace `build_damage_fire_instances` traversal of `DamageFireOverlays` with:

   - entity fixed slots in index order;
   - immutable `AnimStore` lookup by ID;
   - animation type, current frame, world coordinate, flags, and z-adjust from
     authoritative state;
   - existing palette/draw submission only.

2. Rendering must not repair missing IDs, advance frames, mutate caches, request
   RNG, or read asset files.

3. Remove `tick_damage_fire_overlays` from `app_sim_tick` only after the new
   renderer path is exercised by tests.

4. Delete damage-fire-specific `DamageFireOverlays`, `DamageFireAnim`, and their
   app runtime functions. Retain garrison/presentation animation code if it remains
   outside authoritative simulation.

5. Remove the old `effect_frame_counts` dependency only where it existed to drive
   damage-fire gameplay. Do not break unrelated transitional effects.

**Focused tests:** slot order determines draw order; missing/deferred-delete lookup
does not mutate sim; FIRE02 frame 63 reaches the instance builder; one sim tick,
not wall-clock delta, advances the authoritative runtime.

## Task 8: Adapt Audio to Per-Animation Handles

**Files:**

- Modify `src/audio/events.rs`
- Modify `src/audio/sfx.rs`
- Modify `src/app_building_anim.rs` or the current sim-event drain owner
- Modify focused audio adapter tests

**Steps:**

1. Translate sim `AnimationStarted`/`AnimationStopped` events without dropping
   `anim_id`.

2. Add a deterministic ID-to-player-handle map in `SfxPlayer` for animation-owned
   sounds. A new start for an existing ID first releases the old handle.

3. On stop, remove/drop that animation's active handle before optionally playing
   its configured StopSound as a one-shot. A missing handle is an idempotent no-op.

4. Keep unrelated positional and UI sounds on their existing paths. Do not claim
   native mixer parity from this adapter change.

**Focused tests:** two identical sounds with different animation IDs stop
independently; duplicate stop is harmless; replacement start releases the prior
handle; StopSound occurs after handle release; unrelated SFX remain unaffected.

## Task 9: Final Verification and Fidelity Gate

**Files:** all files changed above; no new feature scope.

**Steps:**

1. Inspect the full diff for accidental edits, stale transitional fields, duplicate
   animation clocks, asset reads below rules/render initialization, floats in
   `sim/`, unordered animation storage, or a second allocator/scheduler.

2. Search explicitly:

   ```powershell
   rg -n "DamageFireOverlays|DamageFireAnim|tick_damage_fire_overlays|elapsed_logic_ms" src
   rg -n "f32|f64" src/sim
   rg -n "effect_frame_counts" src
   rg -n "next_stable_entity_id" src
   ```

   Each remaining hit must be unrelated presentation code or intentionally renamed;
   no authoritative damage-fire hit may remain.

3. Check for another Cargo owner before each long command:

   ```powershell
   Get-Process cargo,rustc -ErrorAction SilentlyContinue |
       Select-Object ProcessName,Id,CPU
   ```

4. Run focused tests serially, then one final check:

   ```powershell
   cargo test -p vera20k anim_class -- --nocapture
   cargo test -p vera20k damage_fire -- --nocapture
   cargo test -p vera20k snapshot -- --nocapture
   cargo test -p vera20k sprite_atlas -- --nocapture
   cargo test -p vera20k animation_sound -- --nocapture
   cargo check -q -p vera20k
   ```

   Report the literal `test result:` line from every test command.

5. Format only edited Rust files using edition 2024, inspect the formatting diff,
   and rerun the affected focused tests if formatting changes code layout.

6. Run a fidelity check over the completed slice. The acceptance result must name:

   - retail-derived frame-count/End checks;
   - exact RNG ledger tests;
   - scheduler mutation-order tests;
   - threshold, coordinate, and z-adjust fixtures;
   - snapshot/hash tests;
   - renderer frame-63 reachability;
   - audio identity release tests.

7. Mark unexecuted retail pixel/audio comparisons `UNCHECKED`; do not upgrade them
   to VERIFIED based on Rust-only regression tests.

## Acceptance Criteria

- Buildings own eight fixed animation IDs and a cached active-state byte.
- Damage-fire creation/destruction occurs in sim during the live logic walk.
- All runtime objects share the existing allocator, scheduler, deletion pipeline,
  snapshot, and world hash.
- Stock fire type/frame RNG consumption and slot order match the verified ledger.
- Omitted End uses retail raw frame counts; FIRE02 frame 63 is both selectable and
  renderable.
- Art offsets, building render origin, threshold selection, and z-adjust use the
  verified native mechanisms without sim floating point.
- Rendering is read-only and wall-clock-independent for damage fire.
- Audio start/stop retains animation identity and releases the correct handle.
- The old app-side damage-fire overlay clock and vector state are removed.
- No approximation, fallback constant, or second animation authority remains in
  the migrated slice.

## Source Documents

- `docs/plans/2026-07-18-scheduler-backed-animclass-damage-fire-design.md`
- `docs/research/ANIMCLASS_AI_FIRST_SAFE_MIGRATION_SLICE_GHIDRA_REPORT.md`
- `docs/research/ANIMTYPE_END_LOAD_DAMAGE_FIRE_FRAME_RNG_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_BUILDING_OBJECT_DAMAGE_RUNTIME_SPAWNS_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_CONSTRUCTOR_CALLER_TAXONOMY_RUNTIME_SPAWNS_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_DRAWIT_ZADJUST_DEPTH_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_AI_TRAILERANIM_PERIODIC_SPAWNS_GHIDRA_REPORT.md`
- `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_DAMAGE_FIRE_SELECTOR_RNG_GHIDRA_REPORT.md`
- `docs/research/BUILDING_DAMAGEFIRE_SLOT_CLEAR_DESTROY_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/ANIMATION_SOUNDS_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`
