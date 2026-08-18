# Full BulletClass Implementation Plan

> Execute this plan task-by-task. This is a planning document only; do not implement Rust from this document until it is explicitly approved.

**Goal:** Replace the current instant-hit approximation with a phased, deterministic `BulletClass`-style projectile simulation covering fire, per-tick AI, BounceCheck, proximity/ranged/homing, airburst/cluster/flak, warhead detonation effects, and render/audio handoff boundaries.

**Architecture:** Projectiles become authoritative sim state in `sim/`, driven by `rules/` data and deterministic integer/fixed-point math. Render and audio consume sim-emitted facts (`WorldEffect`, projectile render snapshots, sound events) but never feed presentation types back into sim. Existing instant-hit GI behavior, including the verified `InvisibleLow` visible-impact scatter and the G1 FLH muzzle/report fix, stays as a migration baseline until each weapon family is promoted to real bullets.

**Design Input:** User-approved task brief for Full BulletClass plus the verified Ghidra reports listed in Sources. There is no separate `*-design.md`; this plan treats the supplied brief and research set as the approved design scope.

---

## Grounding Summary

- `BulletClass::Init` writes the runtime payload: target, target speed, warhead, bright, damage, projectile type, firer, animation timer, weapon pointer, and default rocker scale. Verified in `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`.
- `BulletClass::Fire` is active for normal weapons, including `Inviso=yes`; bullets are concealed before launch and revealed during Fire. Verified active in YR.
- Normal GI `[M60]` and deployed GI `[Para]` use `[InvisibleLow]`, which is `Inviso=yes`, `Image=none`, `SubjectToCliffs=yes`, `SubjectToElevation=yes`, `SubjectToWalls=yes`.
- Current Rust already preserves GI impact sub-cell offsets and applies the verified `Inviso=yes` radius `0x20`, snap-false visual scatter for the instant-hit path. This must not regress.
- Normal `InvisibleLow` GI target hits set detonation in the same-cell target branch before BounceCheck; BounceCheck and `ProximityDetector::Check` are skipped because `ROT=0` and `Ranged=no`.
- `BulletClass::AI` divides into non-homing arcing/straight/vertical and homing (`ROT > 0`) paths, with per-tick animation, trailer, movement, collision, proximity, bounce, detonation, degeneration, and last-cell updates.
- `BounceCheck` is shared for real in-flight collisions: subject-to-wall/cliff predicate, underground, flak-below-target, level/passability, and AA close-target cases.
- `Ranged=yes` and `ROT > 0` gate the embedded `ProximityDetector::Check`; `Proximity=yes` is parsed but not the runtime gate.
- `Airburst=yes` bypasses the non-airburst `Cluster` loop, detonates once, then spawns exactly 9 real sub-bullets from `AirburstWeapon`: 8 neighbor-cell targets and 1 impact-cell target.
- `Cluster=N` is not sub-bullet spawning; it is repeated warhead detonation with random `0x100..0x200` lepton scatter after each live detonation.
- Warhead detonation owns damage, special warheads, AnimList impact animations, combat light/smudge/debris/particle outputs, shrapnel, and airburst sub-bullet spawning.
- Current Rust combat still applies weapon damage and warhead effects immediately inside `src/sim/combat/mod.rs`; `rocket_movement` and `homing_movement` exist as partial projectile-like systems but are not a full production `BulletClass` pipeline.
- `Simulation::state_hash` currently hashes entities, RNG, particles, bridge/overlay/smudge state, etc.; new authoritative bullet state must be included there.

## Key Technical Decisions

- **Create a dedicated `sim::bullet` subsystem instead of encoding bullets as `GameEntity`.** Runtime bullets have different lifecycle, collision, and render needs than units/buildings, and a dedicated store avoids bloating `GameEntity` further. **Confidence:** high. **Source:** current `GameEntity` shape, `EntityStore` deterministic pattern, BulletClass reports.
- **Use `BTreeMap<u64, BulletInstance>` for `BulletStore`.** This mirrors deterministic entity iteration without adding ECS or nondeterministic containers. **Confidence:** high. **Source:** AGENTS.md and `src/sim/entity_store.rs`.
- **Represent bullet coordinates as flat lepton integers plus helpers, not screen coordinates.** Sim owns game-space `CoordLepton { x, y, z }`; app/render projects later. **Confidence:** high. **Source:** `WorldEffect` sub-cell fix and sim/render boundary.
- **Use integer/BAM lookup tables for trig-like behavior in sim.** No new `f32`/`f64` in sim projectile logic, even where gamemd used doubles. **Confidence:** high. **Source:** AGENTS.md fixed-point rule; homing/airburst reports.
- **Keep current instant-hit path as a compatibility fallback until each projectile family migrates.** This prevents broad weapon regressions while the BulletClass implementation lands in phases. **Confidence:** high. **Source:** current combat architecture and GI impact fixes.
- **Do not route normal GI `InvisibleLow` hits through BounceCheck.** Preserve the verified Fire -> same-cell AI -> detonation path. **Confidence:** high. **Source:** `INVISIBLELOW_DETONATION_COORDSTRUCT_GHIDRA_REPORT.md`.
- **Treat render/audio handoff as data emission only.** Sim emits bullet render snapshots, world effects, and sound event IDs; app resolves atlases, sprite offsets, and audio playback. **Confidence:** high. **Source:** existing `SimFireEvent`, `SimSoundEvent`, `WorldEffect`.
- **Retire or adapt existing `rocket_movement` / `homing_movement` behind the new bullet subsystem, not as parallel authoritative projectile systems.** Keeping multiple projectile truth sources would fragment detonation timing. **Confidence:** medium-high. **Source:** current code scan; full BulletClass scope.

## Open Questions

### Resolved During Planning

- **Does normal GI use BounceCheck for impact placement?** No. Fire leaves the bullet at the target/body CoordStruct, AI sets detonation in the same-cell target branch, then skips BounceCheck.
- **Does `InvisibleLow` visual impact scatter belong in render?** No. It consumes sim RNG and produces a game-space CoordStruct before `WorldEffect` rendering.
- **Is Flak Cannon stock airburst?** No. Stock `Airburst=yes` is `[V3AirburstP]`; flak uses `Inviso=yes`, `FlakScatter=yes`, `Ranged=yes`, and `Inaccurate=yes`.
- **Is `Proximity=yes` the AI proximity gate?** No. Runtime gate is `ROT > 0 || Ranged=yes`.

### Deferred to Implementation

- **Exact bridge-deck Z behavior for `CellClass::GetGroundHeight` in all BulletClass paths:** use existing bridge reports and add focused parity tests before enabling bridge-sensitive projectile collision for non-GI bullets.
- **Exact fixed-point approximation for homing pitch/terrain avoidance:** implement from verified branch structure using deterministic BAM tables, then compare gameplay traces against gamemd for representative missiles.
- **Whether to keep existing `rocket_movement` tests as compatibility tests or rewrite them as bullet tests:** decide after the new `BulletStore` lands and the old code has one clear caller or no caller.
- **Shrapnel target-search parity details:** trajectory report gives medium-high confidence; implement after the core BulletDetonation path is stable.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/sim/bullet/mod.rs` | Module root, exports, tick entry point, subsystem docs |
| Create | `src/sim/bullet/types.rs` | Bullet ids, coords, targets, phases, render snapshots, output events |
| Create | `src/sim/bullet/store.rs` | `BTreeMap<u64, BulletInstance>` allocation, iteration, removal |
| Create | `src/sim/bullet/fire.rs` | Bullet init/fire pipeline, source/target CoordStruct setup, Inviso fire resolver |
| Create | `src/sim/bullet/ai.rs` | Per-tick BulletClass::AI dispatcher and tick ordering |
| Create | `src/sim/bullet/movement.rs` | Arcing, straight, vertical movement using fixed-point/integer math |
| Create | `src/sim/bullet/homing.rs` | `ROT > 0` tracking, course lock, wobble, approach detector |
| Create | `src/sim/bullet/proximity.rs` | Embedded ProximityDetector Set/Check equivalent |
| Create | `src/sim/bullet/collision.rs` | BounceCheck, wall/cliff helpers, bridge/ground/building checks |
| Create | `src/sim/bullet/detonation.rs` | BulletDetonation coord overrides, cluster loop, detonation dispatch |
| Create | `src/sim/bullet/airburst.rs` | 8+1 AirburstWeapon sub-bullet spawning |
| Create | `src/sim/bullet/render.rs` | Sim-side projectile render snapshots only, no render imports |
| Modify | `src/sim/world/mod.rs` | Add bullet store, tick bullets, drain bullet outputs, expose snapshots |
| Modify | `src/sim/world/world_hash.rs` | Hash all authoritative bullet state |
| Modify | `src/sim/combat/mod.rs` | Split firing into instant-hit fallback vs bullet spawn; keep GI compatibility during migration |
| Modify | `src/sim/components.rs` | Add presentation-neutral bullet/world effect payloads only if needed |
| Modify | `src/rules/projectile_type.rs` | Fix BulletType defaults and remove projectile-level `speed` misparse |
| Modify | `src/rules/weapon_type.rs` | No expected schema change; use existing `Projectile=`, `Warhead=`, `Speed=`, `Report=`, `Anim=` |
| Modify | `src/app_instances/overlays.rs` and app render builders | Consume projectile snapshots/effects from sim without changing sim dependencies |

## Interface Changes

- `Simulation` gains `bullets: BulletStore` and a per-tick bullet output buffer. These are authoritative and serialized unless a deliberate snapshot policy says otherwise.
- `BulletStore` exposes deterministic iteration by bullet id and allocation through `Simulation` so `next_stable_entity_id` or a dedicated `next_bullet_id` remains deterministic.
- Combat firing gains a `spawn_bullet_from_weapon(...)` path returning a bullet id or an instant-hit fallback result.
- Bullet detonation emits the same downstream data categories the current instant-hit path emits: damage events, wall/bridge damage, ore destruction, smudge requests, `WorldEffect` impact anims, and sound ids.
- App/render receives projectile render snapshots as data. It does not call back into sim and sim does not import app/render/audio modules.

## Sim Checklist

- [ ] All projectile math uses integer or fixed-point types in sim. No new `f32`/`f64` in `src/sim/bullet`.
- [ ] `BulletStore` and all fields that affect future gameplay are included in `state_hash`.
- [ ] `sim/` imports no `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- [ ] Bullet tick ordering is explicit in `Simulation::advance_tick`.
- [ ] Deterministic iteration order uses `BTreeMap` / sorted ids.
- [ ] RNG draw order is documented for Inviso scatter, cluster scatter, airburst launch angle, debris/smudge follow-ons.

## Risk Areas

- **Damage timing:** moving non-Inviso projectiles from fire-time damage to arrival-time damage changes gameplay immediately. Migrate one projectile family at a time behind explicit gates.
- **RNG order:** current instant-hit Inviso scatter already consumes RNG. Moving detonation later must preserve draw order for migrated bullets or consciously update parity tests.
- **GI regression:** `M60`/`Para` visible puffs, muzzle `Anim=MGUN-*`, and `Report=` must remain aligned with the G1 FLH fix.
- **Multiple projectile systems:** `rocket_movement`, `homing_movement`, and new bullets must not all detonate the same weapon family.
- **State hashing:** un-hashed bullet state would create replay/lockstep divergence.
- **Rules defaults:** `ProjectileType` currently has known default/comment issues (`Cluster`, `Acceleration`, `SpawnDelay`, `Elasticity`, `Shadow`, projectile `speed`). Fix these before full migration.

## Parity-Critical Items

| Phase | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | GI `InvisibleLow` impact placement and `0x20` visual scatter | Visible on every GI shot | Existing fidelity tests plus side-by-side gamemd GI fire |
| 2 | Fire tick and muzzle/report timing | G1 FLH fix must not regress | Existing muzzle flash tests and visual check |
| 3 | Non-Inviso arrival-time detonation | Rockets/shells must damage when they land, not when fired | V3/tank shell gameplay trace |
| 4 | BounceCheck ordering | Walls, cliffs, flak, and AA proximity produce different visible impacts | Unit tests plus targeted gamemd traces |
| 5 | Homing turn/course/wobble behavior | Missile paths and hit timing are highly visible | AAHeatSeeker2 and ClusterBits traces |
| 6 | Cluster scatter RNG | Explosion footprint and damage distribution depend on exact scatter order | Deterministic RNG tests and V3 cluster check |
| 7 | Airburst 8+1 spawn pattern | V3 signature cluster footprint | V3 strike side-by-side |
| 8 | Warhead effect handoff | Impact anims, smudges, debris, and sounds must appear at detonation coord | Existing smudge/world effect tests plus visual check |

---

## Phased Plan

### Phase 0: Baseline Lockdown

**Why:** Preserve working GI/small-arms behavior and prevent the projectile migration from re-breaking recently fixed visuals.

1. Add regression tests around current `emit_warhead_detonation_effects` and Inviso scatter for `[M60]` / `[Para]`, asserting sub-cell `WorldEffect` coordinates and one RNG byte consumption.
2. Add a combat test that verifies non-garrison GI fire still emits a `SimFireEvent` with `Report=` and weapon `Anim=` data unchanged.
3. Add a short doc note in this plan's follow-up checklist that G1 FLH muzzle/report behavior is presentation-side and not part of bullet detonation.
4. Run `cargo test inviso_impact_anim_scatter --lib`, `cargo test emit_warhead_detonation_effects --lib`, and the existing muzzle flash tests.

### Phase 1: BulletType and Rules Foundation

**Why:** Real bullets consume BulletType defaults and fields every tick; incorrect defaults make later behavior wrong even if the AI code is correct.

1. Fix `src/rules/projectile_type.rs` so `Cluster=1`, `Acceleration=3`, `SpawnDelay=3`, `Elasticity=0.75`, `Shadow=true`, and `AG=true` match `BulletTypeClass::Constructor`.
2. Remove or deprecate `ProjectileType::speed`; projectile speed comes from `WeaponType.Speed`, while BulletType `+0x2F0` is `Arm=`.
3. Ensure `AnimLow`, `AnimHigh`, `AnimRate`, `Trailer`, `SpawnDelay`, `Flat`, and `AnimPalette` read from the resolved `Image=` art section, not the projectile rules section.
4. Add parser tests using `[InvisibleLow]`, `[V3AirburstP]`, `[ClusterBits]`, and a blank projectile section.
5. Verify `rules.projectile("InvisibleLow")`, `rules.weapon("M60")`, and `rules.weapon("V3Cluster")` expose the data needed by the bullet spawn path.

### Phase 2: BulletStore and Data Model

**Why:** All later behavior needs one authoritative place to hold in-flight projectile state.

1. Create `src/sim/bullet/types.rs` with `CoordLepton { x: i32, y: i32, z: i32 }`, `BulletTarget`, `BulletPhase`, `BulletVelocity`, `ProximityState`, and `BulletInstance`.
2. Store rule references as interned ids or stable strings following existing rules/interner patterns; do not clone raw `String` values per tick.
3. Include fields matching verified BulletClass payload: damage, projectile type, weapon type, warhead, firer id, firer owner, target, location, source coord, target coord, last cell, target speed, bright, alive, animation frame/timer, course lock, approach state, proximity state, rocker scale, and Inviso/on-bridge metadata.
4. Create `BulletStore` as `BTreeMap<u64, BulletInstance>` with `insert`, `remove`, `get`, `get_mut`, `iter`, `keys_sorted`, and `next_id` management.
5. Add `Simulation::bullets` and initialize it in constructors/deserialization.
6. Hash bullet count and every gameplay-affecting bullet field in `world_hash.rs`, including fixed-point bits and deterministic target references.
7. Add hash tests proving a changed bullet location, velocity, target, proximity watermark, or phase changes `state_hash`.

### Phase 3: Fire Pipeline and Instant-Hit Migration Gate

**Why:** Weapon fire must create a real bullet without immediately breaking all weapons that still rely on instant-hit damage.

1. Split current `tick_combat_with_fog` shot emission into `resolve_fire_context` and `dispatch_weapon_fire`.
2. `dispatch_weapon_fire` resolves `WeaponType.projectile -> ProjectileType` and selects one of three paths:
   - `InstantCompatibility`: current damage/effect path for not-yet-migrated projectiles.
   - `InvisoImmediateBullet`: create a bullet, run Fire/AI same-tick semantics, detonate through the new bullet detonation path.
   - `InFlightBullet`: create a bullet and let future bullet ticks detonate it.
3. Begin with only `[InvisibleLow]` / `Inviso=yes, ROT=0, Ranged=no` enabled for `InvisoImmediateBullet`; leave everything else on `InstantCompatibility`.
4. Implement `BulletClass::Init` equivalent from `SelectedWeapon`, attacker snapshot, target snapshot, and rules.
5. Implement `BulletClass::Fire` setup: reveal/active flag, source coord, target coord, last cell, velocity from fire context, proximity Set, animation timer seed, and render snapshot creation.
6. For normal `InvisibleLow`, preserve current base coordinate and `0x20` visible scatter until the Inviso line helper is implemented in Phase 4.
7. Route weapon `Report=` through the existing `SimFireEvent` / `SimSoundEvent` mechanism; bullets do not play audio directly.

### Phase 4: Inviso Fire Resolver and Same-Tick Detonation

**Why:** This replaces the current GI approximation with real `BulletClass::Fire` placement while keeping the verified same-tick AI ordering.

1. Implement flat lepton-to-cell conversion using the verified signed `(coord + sign_adjust_0xFF) >> 8` behavior.
2. Implement `FUN_005880A0` equivalent for Inviso hostile active laser-fence interception. Ordinary walls/buildings are not handled here.
3. Implement the sentinel/no-blocker fallback so normal GI final Fire location is the target/body CoordStruct.
4. Implement `FUN_004CC100` / `FUN_004CC360` as shared wall/cliff predicates for fallback and BounceCheck, using resolved terrain, overlay wall flags, effective height, and `[WallModel] AlliedWallTransparency`.
5. Ensure normal GI AI same-cell branch sets the detonation flag before BounceCheck and skips `ProximityDetector::Check`.
6. Move current Inviso impact visual scatter to run from the final `BulletDetonation` coord, not from the old instant-hit target coord.
7. Add tests for:
   - open-ground GI target: final base coord is target/body coord,
   - normal GI does not call BounceCheck,
   - `ROT=0, Ranged=no` skips ProximityDetector check,
   - Inviso visual scatter changes only the anim coord, not damage/wall/ore coords.

### Phase 5: BulletDetonation and Warhead Dispatch Unification

**Why:** Instant-hit and real bullets must use the same warhead effect code, with the projectile choosing when and where detonation happens.

1. Extract current direct-hit, AoE, wall/bridge, ore destruction, smudge, and `emit_warhead_detonation_effects` logic into a reusable `detonate_warhead_at_coord(...)` helper.
2. Implement `BulletClass::BulletDetonation` coord selection:
   - start from `bullet.location`,
   - skip target snap if `Inaccurate=yes`,
   - close target `< 0x20` override when non-airburst,
   - EMEffect and Airburst gates,
   - ground/air/building target CoordStruct overrides using the verified thresholds.
3. Preserve current `WorldEffect` sub-cell payload and render projection.
4. Implement non-Airburst `Cluster` loop as repeated warhead detonation with `RandomRanged(0x100,0x200)` then `Random::Next()` direction scatter for the next coord.
5. For `Cluster <= 0`, follow verified constructor default after Phase 1; stock default should make ordinary bullets detonate once.
6. Add unit tests for target override thresholds, Inaccurate no-snap, Airburst no-cluster, and cluster RNG draw order.

### Phase 6: Non-Homing Movement, BounceCheck, and Collision

**Why:** Real shells, straight projectiles, vertical payloads, and bouncy/flak behavior require per-tick AI before broad migration.

1. Implement `tick_bullets` Phase A: skip dead bullets, update bullet sprite animation frame/timer, spawn trailer requests, save old position.
2. Implement arcing movement with fixed-point velocity, gravity from `[General] Gravity=`, Floater branch gated but initially trace-verified before enabling stock/mod behavior.
3. Implement straight movement speed ramp from `WeaponType.Speed` and `ProjectileType.Acceleration`.
4. Implement vertical movement using `DetonationAltitude`.
5. Implement bridge crossing, ground impact, building-cell collision, out-of-map, stopped-near-ground, and same-cell target detonation predicates.
6. Implement BounceCheck before detonation for moving bullets:
   - subject-to-cliffs/walls predicate,
   - deep underground,
   - `FlakScatter` below target,
   - `Level` passage block,
   - `AA` close-target branch.
7. Keep normal GI `InvisibleLow` same-tick branch out of this BounceCheck path.
8. Migrate one non-homing visible projectile family at a time, starting with a low-risk tank shell fixture, then V3 primary after airburst is ready.

### Phase 7: Proximity, Ranged, Flak, and Homing

**Why:** Flak and missiles are visibly wrong without the real proximity and homing gates.

1. Implement `ProximityDetector::Set` with creation frame, arming frame, arming delay from `Arm=`, reference CoordStruct, and closest-distance watermark.
2. Implement `ProximityDetector::Check` return values 0/1/2 using integer distance and half-distance thresholds.
3. Gate Check by `ROT > 0 || Ranged=yes`, not `Proximity=yes`.
4. Implement flak-specific behavior: `FlakScatter=yes`, `Inaccurate=yes`, `Ranged=yes`, target altitude branch in BounceCheck, detonate at current position rather than target snap.
5. Port or replace `homing_movement` as `sim::bullet::homing`, preserving useful BAM helpers but removing sim-critical f32/atan2 usage from the new implementation.
6. Implement homing course lock, speed ramp/deceleration, 15-frame wobble, target-lost safety altitude, approach-rate accumulation/EMA, and proximity thresholds.
7. Add tests for AAHeatSeeker2 and FlakProj gates using rules fixtures.
8. Migrate `[AAHeatSeeker2]` and `[FlakProj]` after tests pass.

### Phase 8: Airburst, ClusterBits, and Shrapnel

**Why:** V3 and related ordnance need real sub-bullet spawning rather than fire-time splash.

1. Implement airburst fork: if primary projectile `Airburst=yes`, call warhead detonation once and skip the non-airburst cluster loop.
2. At the end of warhead detonation, spawn 9 sub-bullets from `AirburstWeapon`: directions 0..7 neighbor cells plus the impact cell.
3. Each sub-bullet uses `AirburstWeapon.Projectile`, `Damage`, `Warhead`, hardcoded target speed 50, bright false, parent firer/owner, parent impact location as source.
4. Implement verified horizontal launch velocity: `AirburstWeapon.Speed / 10`, `RandomRanged(0, 0x20)`, facing conversion, `VelZ = 0`.
5. Target cells, not original target entities, for all 9 V3 sub-bullets.
6. Implement `ShrapnelWeapon` / `ShrapnelCount` after airburst, using expanding ring enemy search and random fallback from `SpawnShrapnel`.
7. Add V3 rules fixture tests asserting 9 bullets spawn with the expected targets and payloads.
8. Migrate `[V3AirburstP]`, `[V3Cluster]`, and `[ClusterBits]`.

### Phase 9: Render and Audio Handoff

**Why:** Full bullets need visible projectile sprites/trailers without sim depending on rendering.

1. Add a sim-side `BulletRenderSnapshot` containing bullet id, projectile type id, image/anim frame, `CoordLepton`, facing/BAM, flat/shadow/firers-palette flags, and house color index.
2. `Simulation` exposes snapshots by read-only accessor or a drained per-frame vector; app/render builds actual sprite instances.
3. App render resolves projectile `Image=`, `AnimLow/High/Rate`, `Rotates`, `Flat`, `Shadow`, `FirersPalette`, `Trailer`, and palette details.
4. Trailer and bounce/expire animations are emitted as world effects or projectile presentation events with game-space coords.
5. Audio remains event-based: fire reports from combat/fire events, impact sounds from warhead/anim/audio data when those parsers are available.
6. Add visual smoke tests with a local skirmish fixture: projectile sprite appears, moves, detonates, and impact anim appears at the bullet detonation coord.

### Phase 10: Broad Migration and Cleanup

**Why:** Once core behavior is stable, remove instant-hit special cases and obsolete projectile movement paths.

1. Add a migration table listing each stock projectile section and its current handling: instant, InvisoImmediateBullet, InFlightBullet, blocked pending research.
2. Promote projectile families in safe batches: `InvisibleLow`, generic straight/arc shells, homing missiles, flak, airburst, shrapnel.
3. Delete or fold obsolete production uses of `rocket_movement` and `homing_movement` after all callers route through `sim::bullet`.
4. Keep tests that describe old accepted approximations only if they are renamed as compatibility tests or updated to BulletClass parity.
5. Run full combat, movement, bridge, smudge, and world hash test suites.
6. Add a final parity checklist comparing representative weapons: GI, Guardian GI missile, Rhino/Grizzly cannon, Flak Cannon, V3, Dreadnought/Boomer missile if represented.

## Test Strategy

- **Unit tests:** coordinate conversion, deterministic trig/BAM tables, ProximityDetector, BounceCheck predicates, cluster scatter, airburst target selection, BulletStore hashing.
- **Rules tests:** projectile defaults and section parsing for `InvisibleLow`, `FlakProj`, `V3AirburstP`, `ClusterBits`, and blank sections.
- **Combat integration tests:** fire-time vs detonation-time damage, GI same-tick Inviso, non-Inviso delayed damage, wall/bridge/ore effects emitted from bullet detonation.
- **Hash tests:** bullet state mutations change `Simulation::state_hash`; render-only snapshots and skipped app fields do not.
- **Regression tests:** G1 FLH muzzle/report, GI impact scatter, bridge/smudge behavior, current garrison fire routing.
- **Parity tests:** targeted gamemd side-by-side checks for GI fire, flak AA burst, AAHeatSeeker2 homing, tank shell wall/cliff behavior, and V3 9-bullet airburst footprint.

## Migration Strategy

1. Keep instant-hit as the default for all projectiles.
2. Route `InvisibleLow` through real bullet init/fire/detonation first because its same-tick behavior can preserve existing gameplay timing while improving CoordStruct parity.
3. Extract warhead detonation into shared helpers before enabling delayed projectile families.
4. Enable in-flight bullets only for narrow projectile sections with dedicated tests.
5. Move render/audio consumers after sim state is correct; no presentation code should drive projectile behavior.
6. Remove instant-hit branches only when the projectile migration table has coverage for every stock section or an explicit documented fallback.

## Sources & References

- `docs/research/INVISIBLELOW_DETONATION_COORDSTRUCT_GHIDRA_REPORT.md`
- `docs/research/BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`
- `docs/research/BULLET_CLASS_AI_GHIDRA_REPORT.md`
- `docs/research/BULLETCLASS_TRAJECTORY_AND_HOMING.md`
- `docs/research/BULLETTYPECLASS_GHIDRA_REPORT.md`
- `docs/research/BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`
- `docs/research/AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md`
- `docs/research/WARHEAD_DETONATE_GHIDRA_REPORT.md`
- `docs/fidelity-checks/2026-05-17-gi-small-arms-warhead-impact-placement.md`
- Current code inspected: `src/sim/combat/mod.rs`, `src/sim/combat/combat_weapon.rs`, `src/sim/combat/combat_aoe.rs`, `src/sim/components.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`, `src/rules/projectile_type.rs`, `src/rules/weapon_type.rs`, `src/app_instances/overlays.rs`, `src/sim/movement/homing_movement.rs`, `src/sim/movement/rocket_movement.rs`.
- INI anchors: `ini/rulesmd.ini` `[M60]`, `[Para]`, `[InvisibleLow]`, `[FlakProj]`, `[V3AirburstP]`, `[V3Cluster]`, `[ClusterBits]`; `[General] Gravity`, `BallisticScatter`, `HomingScatter`, `MissileSpeedVar`, `MissileROTVar`, `MissileSafetyAltitude`, `MaxDamage`; `[WallModel] AlliedWallTransparency=no`.
