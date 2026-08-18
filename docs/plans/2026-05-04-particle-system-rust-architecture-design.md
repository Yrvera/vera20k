# Particle System — Rust Architecture Design

**Date:** 2026-05-04
**Scope:** Tier 2 MVP — `Smoke`, `Gas`, `Fire` BehavesLike variants. Spark + Railgun deferred to a later pass behind a separate render-design brainstorm.
**Source of truth:** [PARTICLESYSTEMCLASS_GHIDRA_REPORT.md](../../docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md) §1–§11 (binary-verified).
**Predecessor plan:** [docs/plans/2026-05-04-particle-system-gaps-investigation-plan.md](2026-05-04-particle-system-gaps-investigation-plan.md) — research executed and merged into §11 of the report.

## Goal

Land the `ParticleSystemClass` + `ParticleClass` runtime in Rust as **authoritative sim state**, with two new deterministic stores, integrated into the existing tick loop, replacing the `DamageFireOverlays` placeholder, and covering every non-pixel-rendered particle path in retail YR.

---

## Architecture Context

How the existing engine works in the area this feature touches:

- **Entity model.** [src/sim/game_entity.rs](../../src/sim/game_entity.rs) — `GameEntity` is a single struct with always-present + `Option<Subsystem>` fields, indexed by `u64` stable ID in `EntityStore: BTreeMap<u64, GameEntity>` ([src/sim/entity_store.rs](../../src/sim/entity_store.rs)). No enum-based `EntityKind`. Animations attach as `Option<Animation>` on any entity. Bullets/projectiles aren't persistent entities — they're transient.
- **Tick loop.** [src/sim/world/mod.rs:980 advance_tick](../../src/sim/world/mod.rs) — 13 ordered phases (commands → ground move → air → vision → power → superweapons → turrets+combat → retaliation+passengers → scatter+production+repairs+docks+ore → AI → defeat → building anims+cleanup → state hash). Every phase mutates deterministically.
- **Rules parsing.** [src/rules/](../../src/rules/) — types parse from INI sections into structs, stored on `RuleSet` as `HashMap<String, Arc<T>>` (e.g., `weapon_types: HashMap<String, Arc<WeaponType>>`). [src/rules/weapon_type.rs:200-250](../../src/rules/weapon_type.rs) already parses `AttachedParticleSystem`, `UseFireParticles`, `UseSparkParticles` as `Option<String>` placeholders.
- **Animation / SHP binding.** [src/sim/animation.rs](../../src/sim/animation.rs) — `SequenceKind` + `SequenceDef` + `Animation` runtime component. SHP filename comes from `ArtEntry.image` ([src/rules/art_data.rs](../../src/rules/art_data.rs)). Frame timing tracked via `elapsed_ms` accumulator.
- **Rendering.** [src/app_render/build_instances.rs:277](../../src/app_render/build_instances.rs) — iterates `sim.entities.values()` deterministically, dispatches sprite vs voxel, emits draw instances. Translucency flags exist on the SHP draw path. **No pixel-write path** for spark/railgun (Tier 3 concern, deferred).
- **Determinism.** [src/sim/rng.rs](../../src/sim/rng.rs) — single `SimRng` (xorshift64*), seeded once, never re-seeded; all sim RNG goes through `sim.rng`. [src/util/fixed_math.rs](../../src/util/fixed_math.rs) — `I32F16` from `fixed` crate; no floats in sim. [src/sim/world/world_hash.rs:18-36 state_hash](../../src/sim/world/world_hash.rs) — hashes EntityStore + production + power + fog + bridge + overlays at end of every tick.
- **Existing placeholder.** [src/sim/components.rs:475-502](../../src/sim/components.rs) — `DamageFireOverlays { fires: Vec<DamageFireAnim> }` field on building entities, ticked from app layer ([src/app_building_anim.rs](../../src/app_building_anim.rs)), rendered by [src/app_instances/overlays.rs](../../src/app_instances/overlays.rs). 5 referencing files total.
- **Combat hooks.** [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — fire path and damage application; building damage state transitions read `entity.health`. The natural spawn sites for `AttachedParticleSystem` (per-shot) and `DamageParticleSystems` (per-damage-event).

## Impact Analysis

**New code:**
- `src/rules/particle_type.rs` — `ParticleType` parser, ~250 lines.
- `src/rules/particle_system_type.rs` — `ParticleSystemType` parser, ~180 lines.
- `src/sim/particles/` — new module:
  - `mod.rs` — `ParticleSystem`, `Particle`, `ParticleSystemStore`, public API. ~120 lines.
  - `tick.rs` — system-AI dispatch (Smoke/Gas/Fire), particle-AI dispatch, lifecycle, NextParticle chaining. ~600 lines (data-heavy, may split if it exceeds 600).
  - `spawn.rs` — `world.spawn_particle_system(...)` + helpers (SpawnParticle, SpawnParticleWithInsert). ~150 lines.
  - `damage.rs` — gas/fire damage application to cell occupants, FinalDamageState gate. ~100 lines.
  - `wind.rs` — gas/smoke wind drift tables (constants from §10.14.4). ~30 lines.
- `src/render/particles.rs` — Tier 2 SHP draw integration (gas/smoke/fire); particles emit sprite instances through the existing pipeline. ~200 lines.

**Modified existing code:**
- [src/rules/ruleset.rs](../../src/rules/ruleset.rs) — add `particle_types`, `particle_system_types`, the `PsTypeIndex` interner. Add 2-pass resolve step.
- [src/rules/weapon_type.rs](../../src/rules/weapon_type.rs) — replace `attached_particle_system: Option<String>` with `Option<ParticleSystemTypeId>`. Resolve at parse-time.
- [src/rules/techno_type.rs](../../src/rules/techno_type.rs) (or wherever TechnoType lives) — add fields for `DamageParticleSystems`, `RefinerySmokeParticleSystem`, `NaturalParticleSystem`, `DamageSmokeOffset`, `RefinerySmokeOffset{One..Four}`, `GapGenerator` flag. All references typed as `ParticleSystemTypeId`.
- [src/rules/general.rs](../../src/rules/general.rs) (or wherever `[General]` parsing lives) — add `barrel_particle: Option<ParticleSystemTypeId>`.
- [src/rules/combat_damage.rs](../../src/rules/combat_damage.rs) (or new file under rules/) — add the 9 `[CombatDamage]` Default*System slots (`DefaultLargeGreySmokeSystem`, `DefaultSmallGreySmokeSystem`, `DefaultSparkSystem`, `DefaultLargeRedSmokeSystem`, `DefaultSmallRedSmokeSystem`, `DefaultDebrisSmokeSystem`, `DefaultFireStreamSystem`, `DefaultTestParticleSystem`, `DefaultRepairParticleSystem`). Tier 2 only consumes the smoke/fire ones; Spark/Repair land with Tier 3.
- [src/sim/world/mod.rs](../../src/sim/world/mod.rs) — add `particle_systems: ParticleSystemStore` field on `Simulation`. Insert new tick phase between combat (7) and retaliation (8): "particle systems".
- [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) — extend `state_hash` to include `particle_systems`.
- [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — at fire site, spawn `AttachedParticleSystem` if weapon has it. At damage application, spawn from `DamageParticleSystems` on health threshold transitions.
- [src/sim/buildings/](../../src/sim/buildings/) (gap generator, refinery dump, barrel handling) — wire `NaturalParticleSystem`, `RefinerySmokeOffset{N}`, `BarrelParticle` spawns.
- [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs) — add a particle-system iteration alongside the existing entity iteration.

**Removed code (staged retirement):**
- [src/sim/components.rs:475-502](../../src/sim/components.rs) — `DamageFireOverlays`, `DamageFireAnim` structs.
- [src/app_building_anim.rs `tick_damage_fire_overlays`](../../src/app_building_anim.rs) — function and its tick site in [src/app_sim_tick.rs:164](../../src/app_sim_tick.rs).
- [src/app_instances/overlays.rs](../../src/app_instances/overlays.rs) — `DamageFireAnim` rendering branch.
- Any building-damage hookup that pushed into `DamageFireOverlays` from the simulation side.

**Risk areas:**
- **Tick-phase placement.** Inserting a new phase shifts the implicit ordering contract. Replays from before the change won't match after. Acceptable since particle behaviour was previously absent — but worth flagging.
- **State-hash growth.** Each particle adds bytes to the hash. With ~20 active PSCs each holding ~50 particles, that's ~1000 hashable records per tick. Real but bounded. Profile early.
- **Determinism of damage application.** Gas particles deal damage on a `MaxDC` countdown to all units in the cell. Cell-occupant ordering must be deterministic — `Cell.objects` should already be a sorted iteration; verify before relying on it.
- **DamageFireOverlays retirement timing.** Cutting over too early loses the visible damage smoke; cutting over too late leaves dead code. Land PSC + verify visually + then delete in same branch.
- **2-pass rules parse.** Adding ID resolution as a second pass changes `Ruleset::from_ini` ordering. Existing types that resolve type refs by name (still HashMap-based) keep working; only ParticleSystemType / ParticleType references go through the interner. Future migrations of WeaponType, TechnoType, etc., to interned IDs follow the same pattern.

## Chosen Approach

A two-store model with interned type IDs and authoritative sim ticking. Tier 2 scope (gas, smoke, fire only) using the existing SHP render pipeline.

The four major decisions (one per Q in the brainstorm) were:

| Decision | Choice |
|----------|--------|
| Where PSCs and Particles live | **Dedicated `ParticleSystemStore` alongside `EntityStore`** — particles owned by their PSC, never enter the entity store. Mirrors binary §5.2. |
| MVP scope | **Tier 2: Smoke + Gas + Fire**, all via existing SHP render. Spark + Railgun deferred behind separate render-design brainstorm. |
| Sim/cosmetic split | **Authoritative sim state** — fixed-point math, `sim.rng`, included in `state_hash`, new tick phase between combat (7) and retaliation (8). |
| Type-ref shape | **Interned `ParticleSystemTypeId(u32)` / `ParticleTypeId(u32)`** resolved at INI-parse-time. Matches binary's index-based storage and aligns with the broader String→u32 migration. |

`DamageFireOverlays` is retired **in this work**, staged: smoke PSC first, visually verified against current damage rendering, then `DamageFireOverlays` and its 5 referencing files deleted in the same branch.

---

## Design

### Components

#### `src/rules/particle_type.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleTypeId(pub u32);

#[derive(Debug, Clone)]
pub struct ParticleType {
    pub name: InternedId,                      // section name "GasCloud1"
    pub behaves_like: ParticleBehavesLike,     // enum, see below
    pub image: Option<String>,                 // SHP via existing Image=→SHP path
    pub max_dc: u16,                           // damage countdown reset
    pub max_ec: u16,                           // lifetime in frames
    pub damage: i32,
    pub warhead: Option<WarheadId>,            // resolved at parse
    pub start_frame: u16,
    pub num_loop_frames: u16,
    pub translucency: u8,                      // 0/25/50
    pub wind_effect: u8,                       // 0..5
    pub velocity: I32F16,
    pub deacc: I32F16,
    pub radius: i32,
    pub delete_on_state_limit: bool,
    pub end_state_ai: u8,
    pub start_state_ai: u8,
    pub state_ai_advance: u8,
    pub final_damage_state: u8,                // defaults to end_state_ai if unset
    pub translucent_25_state: u8,
    pub translucent_50_state: u8,
    pub normalized: bool,
    pub next_particle: Option<ParticleTypeId>,
    pub next_particle_offset: Vec3<I32F16>,
    pub color_list: Vec<Rgb>,                  // PURE Vec — no embedded vector header
    pub color_speed: I32F16,
    pub start_color_1: Rgb,
    pub start_color_2: Rgb,
    pub x_velocity: i32,
    pub y_velocity: i32,
    pub min_z_velocity: i32,
    pub z_velocity_range: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleBehavesLike { Gas, Smoke, Fire, Spark, Railgun }
//                                   0      1      2      3      4
//                             ↑ matches PARTICLE TYPE enum (NOT system enum)
```

`ColorList` stores as a pure `Vec<Rgb>` — we do NOT mirror the binary's embedded `DynamicVectorClass` header layout. The binary layout exists for save-compat parity, but our snapshot format is independent (per the snapshot project), so we can use idiomatic Rust here. The §11.1 layout is preserved as a doc reference for anyone tracing back to the binary.

`Spark` and `Railgun` variants are parsed but unused in Tier 2. Spawning a PSC of either type returns an error from `spawn_particle_system` until Tier 3.

#### `src/rules/particle_system_type.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleSystemTypeId(pub u32);

#[derive(Debug, Clone)]
pub struct ParticleSystemType {
    pub name: InternedId,
    pub behaves_like: ParticleSystemBehavesLike,  // DIFFERENT enum from ParticleType — see below
    pub holds_what: Option<ParticleTypeId>,
    pub spawns: bool,
    pub spawn_frames: u32,
    pub slowdown: I32F16,
    pub particle_cap: u32,                        // default 50
    pub spawn_radius: i32,
    pub spawn_cutoff: I32F16,
    pub spawn_translucency_cutoff: I32F16,
    pub lifetime: i32,                            // -1 = infinite
    pub spawn_direction: Vec3<I32F16>,
    // Railgun-only fields (parsed, ignored at Tier 2):
    pub particles_per_coord: I32F16,
    pub spiral_delta_per_coord: I32F16,
    pub spiral_radius: I32F16,
    pub position_perturbation_coefficient: I32F16,
    pub movement_perturbation_coefficient: I32F16,
    pub velocity_perturbation_coefficient: I32F16,
    // Spark-only fields (parsed, ignored at Tier 2):
    pub spawn_spark_percentage: I32F16,
    pub spark_spawn_frames: u32,
    pub light_size: i32,
    pub one_frame_light: bool,
    pub laser: bool,
    pub laser_color: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSystemBehavesLike { Smoke, Gas, Fire, Spark, Railgun }
//                                       0      1      2      3      4
//                                  ↑ matches PARTICLE SYSTEM enum (Smoke=0, Gas=1)
```

The asymmetric enum ordering between `ParticleBehavesLike` (Gas=0, Smoke=1) and `ParticleSystemBehavesLike` (Smoke=0, Gas=1) is preserved exactly as the binary defines (§2.1 / §2.2 of the report). Both enums implement `From<u8>` / `Into<u8>` for INI parse + state-hash stability.

#### `src/sim/particles/mod.rs`

```rust
pub struct ParticleSystem {
    pub stable_id: u64,
    pub type_id: ParticleSystemTypeId,
    pub coords: Vec3<I32F16>,
    pub offset: Vec3<I32F16>,                  // from attached object
    pub particles: Vec<Particle>,              // capped by type.particle_cap
    pub spawn_timer: I32F16,                   // spawn accumulator (smoke)
    pub lifetime: i32,                         // countdown
    pub spark_spawn_frames: i32,               // countdown
    pub facing: u8,                            // 0..63 (default 29 = 0x1D)
    pub marked_for_deletion: bool,
    pub directionless: bool,                   // true if SpawnDirection==(0,0,0)
    pub attached_entity: Option<EntityId>,     // unit/building/projectile
    pub owner_entity: Option<EntityId>,        // who fired/owns (for damage attribution)
    pub target_coords: Vec3<I32F16>,           // railgun endpoint (Tier 3)
    pub owner_house: Option<HouseId>,
    pub done_spawning: bool,
}

pub struct Particle {
    pub type_id: ParticleTypeId,
    pub coords: Vec3<I32F16>,
    pub previous_coords: Vec3<I32F16>,
    pub origin: Vec3<I32F16>,                  // float copy of spawn pos
    pub direction: Vec3<I32F16>,               // normalized
    pub velocity: I32F16,
    pub lifetime_remaining: i16,               // ticks
    pub damage_counter: i16,                   // resets from MaxDC
    pub state_ai_advance: u8,
    pub animation_state: u8,                   // current frame in state machine
    pub translucency: u8,                      // 0/25/50 byte
    pub hit_ground: bool,
    pub marked_for_deletion: bool,
    // Per-BehavesLike scratch (gas/smoke drift, fire prev-delta):
    pub drift_x: i32,
    pub drift_y: i32,
    pub drift_z: i32,
    // Spark/railgun color state (Tier 3, present but unused in Tier 2):
    pub current_color: Rgb,
    pub color_index: u8,
    pub color_accumulator: I32F16,
}

pub struct ParticleSystemStore {
    systems: BTreeMap<u64, ParticleSystem>,
    next_id: u64,
}

impl ParticleSystemStore {
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &ParticleSystem)> { ... }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&u64, &mut ParticleSystem)> { ... }
    pub fn insert(&mut self, sys: ParticleSystem) -> u64 { ... }
    pub fn remove(&mut self, id: u64) -> Option<ParticleSystem> { ... }
    pub fn len(&self) -> usize { ... }
}
```

`Vec<Particle>` is the obvious match for the binary's `DynamicVectorClass<ParticleClass*>`. Most PSCs cap at <50 particles, so SmallVec might be a perf win, but adds complexity — pick `Vec` per the "no premature abstraction" rule and revisit if profiling shows allocation pressure.

#### `src/sim/particles/tick.rs`

System AI dispatch entry point:

```rust
pub fn tick_particle_systems(
    sim: &mut Simulation,
) {
    // Iterate by stable id — BTreeMap order
    let ids: Vec<u64> = sim.particle_systems.iter().map(|(id, _)| *id).collect();
    for id in ids {
        // Borrow-juggle: pull system, run AI, push back if not deleted
        if let Some(mut sys) = sim.particle_systems.remove(id) {
            tick_one_system(&mut sys, sim);
            if !(sys.marked_for_deletion && sys.particles.is_empty()) {
                sim.particle_systems.insert_at(id, sys);
            }
        }
    }
}

fn tick_one_system(sys: &mut ParticleSystem, sim: &mut Simulation) {
    let ty = sim.rules.particle_system_type(sys.type_id);
    match ty.behaves_like {
        ParticleSystemBehavesLike::Smoke => ai_smoke(sys, ty, sim),
        ParticleSystemBehavesLike::Gas   => ai_gas(sys, ty, sim),
        ParticleSystemBehavesLike::Fire  => ai_fire(sys, ty, sim),
        // Tier 3:
        ParticleSystemBehavesLike::Spark | ParticleSystemBehavesLike::Railgun => {
            // No-op — Tier 3 brings these online.
        }
    }
    // Lifetime decrement (§3.1)
    sys.lifetime -= 1;
    if sys.lifetime == 0 {
        sys.marked_for_deletion = true;
    }
}
```

The borrow-juggle is necessary because system AI may spawn other PSCs (via `NextParticle` chains in `[Particles]`) and apply damage to entities in `sim`. Pulling the PSC out by ID, ticking with full sim access, then pushing back gives clean exclusive borrowing. The cost is `O(log n)` per system per tick; with ~20 active PSCs that's negligible.

Each particle AI mirrors the per-BehavesLike branches in §3.5–§3.7 and §3.8 of the report, with fixed-point math throughout.

#### Tick-phase placement

```rust
// src/sim/world/mod.rs — advance_tick
1.  Commands
2.  Ground movement
3.  Air + special movement
4.  Vision
5.  Power
6.  Superweapons
7.  Turrets + Combat
7.5 ParticleSystems    ← NEW
8.  Retaliation + Passengers
9.  Scatter + Production + ...
10. AI
11. Defeat detection
12. Building animations + cleanup
13. State hash
```

Why between combat (7) and retaliation (8): gas/fire particles deal damage. That damage must be visible to retaliation logic in the same tick (a unit struck by gas this tick should retaliate against the gas's owner this tick, not next). Placing particles after combat means combat fires fresh PSCs, particles tick once before retaliation reads health changes.

#### Spawn API

```rust
// src/sim/particles/spawn.rs

impl Simulation {
    pub fn spawn_particle_system(
        &mut self,
        type_id: ParticleSystemTypeId,
        coords: Vec3<I32F16>,
        attached_entity: Option<EntityId>,
        owner_entity: Option<EntityId>,
        target_coords: Vec3<I32F16>,             // for railgun; pass zero for others
        owner_house: Option<HouseId>,
    ) -> Option<u64>
}
```

Returns `Option<u64>` because spawning a Spark/Railgun PSC at Tier 2 logs a warning and returns `None` — Tier 3 lights this path up. Returns `Some(id)` on Tier 2 success.

Per §11.5.B, the constructor must accept null type pointer (gap generators with `NaturalParticleSystem=` unset). In Rust we represent that as `Option<ParticleSystemTypeId>` at the call site — `None` means "don't spawn". The spawn helper signature takes a non-optional `type_id`, so the null-check happens at the caller.

#### Consumer-side wiring

| Consumer | File | Trigger | Spawn |
|----------|------|---------|-------|
| `WeaponType.attached_particle_system` | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Per-shot fire | `spawn_particle_system(weapon.attached_particle_system, fire_coords, Some(bullet_or_attacker), Some(attacker), zero, attacker_house)` |
| `WeaponType.use_fire_particles` | combat fire path | Per-shot fire | Spawn a fire PSC (use `RulesClass.DefaultFireStreamSystem` per §11.8.G) |
| `WeaponType.use_spark_particles` | combat fire path | Per-shot fire (Tier 3 only — log + skip at Tier 2) | — |
| `TechnoType.damage_particle_systems` | [src/sim/combat/](../../src/sim/combat/) damage application | Health drops below `ConditionYellow` (smoke) or `ConditionRed` (sparks Tier 3) | `spawn_particle_system(...)` filtered by behaves_like (Smoke for ReceiveDamage path, Spark for AI_Update path per §8.6.1–§8.6.2) |
| `TechnoType.refinery_smoke_particle_system` | [src/sim/buildings/refinery.rs](../../src/sim/buildings/) (or wherever refinery dump lives) | Each dump cycle, gated on `BuildingType.refinery_smoke_frames` | Up to 4 smoke PSCs at the 4 `RefinerySmokeOffset{N}` slots, skipping ones equal to the sentinel coord |
| `TechnoType.natural_particle_system` | gap generator state machine | State 3→0 transition | Spawn at building coord + `BuildingType.gap_shroud_offset`, attached to the cell |
| `RulesClass.barrel_particle` | barrel-overlay destruction | Cell with barrel destroyed by area damage | `spawn_particle_system(rules.general.barrel_particle, cell_coord, ...)` |
| `RulesClass.combat_damage.default_fire_stream_system` | fire-weapon path | Hooked from `use_fire_particles` | (above) |

The 9 `[CombatDamage]` Default*System slots all parse to `Option<ParticleSystemTypeId>`. Tier 2 wires the smoke/fire ones; the spark/repair ones are parsed and held but unused.

### Interfaces / Contracts

**Rules public API (from `Ruleset`):**

```rust
impl Ruleset {
    pub fn particle_type(&self, id: ParticleTypeId) -> &ParticleType;
    pub fn particle_system_type(&self, id: ParticleSystemTypeId) -> &ParticleSystemType;
    pub fn ps_type_id_by_name(&self, name: &str) -> Option<ParticleSystemTypeId>;
    pub fn p_type_id_by_name(&self, name: &str) -> Option<ParticleTypeId>;
}
```

The `_by_name` lookups exist for INI parse-time resolution and debug tooling. Hot-path code (combat, damage, render) holds `*Id` values and uses the indexed accessors.

**Sim public API (from `Simulation`):**

```rust
impl Simulation {
    pub fn particle_systems(&self) -> &ParticleSystemStore;
    pub fn particle_systems_mut(&mut self) -> &mut ParticleSystemStore;
    pub fn spawn_particle_system(/* see above */) -> Option<u64>;
}
```

**Render public API:**

```rust
// src/render/particles.rs
pub fn collect_particle_draw_instances(
    particle_systems: &ParticleSystemStore,
    rules: &Ruleset,
    art: &ArtRegistry,
    out: &mut Vec<SpriteInstance>,
);
```

Called from [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs) alongside the existing entity iteration. Particles emit `SpriteInstance` records with translucency flags set per the `Particle.translucency` byte (0 / 0x19 / 0x32 / 0x4B+ → opaque / 50% / 25% / heavy fade per §7).

### Data Flow

```
INI files (rulesmd.ini, artmd.ini)
      │
      ▼
Ruleset::from_ini  ──► [Particles] sections   ──► Vec<ParticleType>           (assigned ParticleTypeId in scan order)
                  ──► [ParticleSystems]       ──► Vec<ParticleSystemType>     (assigned ParticleSystemTypeId)
                  ──► 2nd pass: resolve TechnoType / WeaponType / RulesClass refs (String → *Id)
      │
      ▼
Simulation init: all consumers hold *Id values
      │
      ▼
runtime tick:
   combat.fire() ──► spawn_particle_system(*Id, coord, ...) ──► ParticleSystemStore.insert(...)
                                                                       │
                                                                       ▼
   advance_tick phase 7.5 ──► tick_particle_systems
                                │
                                ▼
                         per-system AI:  spawn particles, apply gas/fire damage,
                                        chain NextParticle, decrement lifetime,
                                        mark for deletion
                                │
                                ▼
                         per-particle AI: drift, animate, age, deal damage in cell
      │
      ▼
phase 13 state_hash includes ParticleSystemStore
      │
      ▼
render frame: collect_particle_draw_instances(...) emits SHP draws via sprite pipeline
```

### Error Handling

- Missing PSType reference (e.g., TechnoType lists `DamageParticleSystems=NonExistent`): rules parser logs a warning, drops the entry. Other entries proceed.
- Missing PType reference inside a PSType's `HoldsWhat=`: same treatment.
- `Spark` / `Railgun` PSC spawn at Tier 2: warn-once log, return `None` from `spawn_particle_system`, no allocation.
- Null `ParticleSystemTypeId` at consumer (e.g., `NaturalParticleSystem` unset on a gap-generator building): caller short-circuits before calling spawn.
- SHP missing for a Particle's `Image=`: render skips the draw with a once-per-image warning. Per the `feedback_silent_render_failures` memory: warn-level log, do not silently fall through to a default texture.
- Cell-occupant iteration during gas damage: relies on existing `Cell.objects` deterministic order; if that's not deterministic today, that's a pre-existing bug to fix in a separate task — do not add a sort here.

### Testing Strategy

- **Unit tests (per-AI variant).** For each of Smoke / Gas / Fire system AI: build a `Simulation`, spawn a PSC, advance N ticks, assert particle count, position, lifetime, damage applied. ~6 tests per variant.
- **Cell damage determinism test.** Spawn a gas PSC over a cell with 3 units. Tick. Assert all three lose health by the documented `MaxDC` cadence. Run twice from the same seed and assert identical state hashes.
- **NextParticle chaining test.** Spawn a `GasCloudM1` (which chains to `GasCloud1` then `GasCloudD1`). Tick through the chain. Assert state transitions match §3.4.
- **Smoke double-spawn test (regression for §9.6 finding).** Spawn a smoke PSC with `NextParticle=` set. When a particle dies, assert TWO new particles spawn at symmetric offsets, not one.
- **Spawn-cap test.** Spawn a smoke PSC with `ParticleCap=5`. Tick aggressively. Assert vector never exceeds 5.
- **Wind-table test.** Build a gas PSC with `WindEffect=2` and `WindDirection=2` (East). Tick. Assert position drifts +2 in X per `(10/2) = 5` ticks.
- **Determinism test.** Two `Simulation` instances seeded identically run the same combat scenario. Assert their state hashes match every tick.
- **DamageFireOverlays parity test.** Before deletion, run a short scenario with the old system enabled and the new PSC system enabled side-by-side; visually inspect the rendered output to confirm parity. Once confirmed, retire.

---

## Architectural Decisions

**Patterns followed:**
- Fixed-point math (`I32F16`) and `sim.rng` exclusively — same as every other sim system.
- `BTreeMap<u64, T>` deterministic store — same shape as `EntityStore`.
- New tick phase inserted in the middle of `advance_tick` — same pattern used when other systems were added.
- Type-class parsing into `Ruleset` — same shape as `WeaponType`, `WarheadType`, etc.

**Patterns deviated from (and why):**
- **Interned `*Id(u32)` instead of `String` type refs.** Every existing rules module uses `Option<String>` for type references (e.g., `weapon_types: HashMap<String, Arc<WeaponType>>`). This design uses `ParticleSystemTypeId(u32)` and `ParticleTypeId(u32)`. Rationale: the binary indexes by `int` not by name, and the broader codebase memory has the `String → u32` migration on the roadmap. New code should land on the new pattern, not the old one. WeaponType / TechnoType / etc. migrate to interned IDs in follow-up work.
- **Two-store sim model (`EntityStore` + `ParticleSystemStore`).** Existing convention is one entity store with `Option<Subsystem>` fields. PSCs don't fit that mould: many spawn paths produce parentless PSCs (TriggerAction, Scenario_Start, BarrelParticle, area damage), and bloating `GameEntity` with a 50+ field particle subsystem most entities never use is worse than a dedicated store. The binary itself models PSCs and Particles as distinct ObjectClass instances; a separate store mirrors that.
- **`Vec<Particle>` over a particle pool.** Per-PSC owned vec, no shared arena. Simpler. Profile if hot.

**Tech debt introduced:**
- Until WeaponType's `attached_particle_system` field migrates from `Option<String>` to `Option<ParticleSystemTypeId>`, the rules-side type system has two parallel conventions (String-keyed and Id-keyed). Mitigation: do the WeaponType migration as the immediate follow-up to this work — it's a small mechanical change once `PsTypeIndex` exists.

**Tech debt avoided:**
- No `DamageFireOverlays` left running parallel to PSC. Staged retirement keeps the codebase clean.
- No String-based type refs introduced in new code.

## Alternatives Considered

| Alternative | Rejected because |
|-------------|------------------|
| Single `EntityStore` with `Option<ParticleSystem>` field on `GameEntity` (Q1 option A) | Bloats GameEntity with a subsystem most entities never use; "ghost entities" for parentless PSCs feel wrong |
| PSC as pure subsystem on parent entity, no standalone PSCs (Q1 option C) | Can't represent the 6 parentless spawn paths (TriggerAction, Scenario_Start, area damage gas, EBolt, BarrelParticle, refinery dump) — fails 99% parity bar |
| Tier 1 only — Smoke alone (Q2 option) | Too narrow — ships a partial system that retires only `DamageFireOverlays` and leaves gas/fire weapons unrendered |
| Tier 3 — full set including Spark + Railgun (Q2 option) | Pixel-render plumbing for spark/railgun warrants its own design pass; scope creep |
| Cosmetic-only particles, ticked from render layer (Q3 option B) | Gas and fire deal gameplay damage; non-deterministic damage breaks lockstep |
| Split: damaging-particles authoritative, decorative cosmetic (Q3 option C) | Mode boundary is easy to violate; one mistake (a future BehavesLike with Damage=0 that turns out to matter) breaks determinism |
| String-name refs at spawn time (Q4 option A) | Continues the String-based type-ref pattern the broader project plans to retire; HashMap lookup on every spawn |
| `DamageFireOverlays` left running in parallel | Two systems competing to draw building damage smoke; modder edits break either; defeats the cleanup |
| Mirror binary's `DynamicVectorClass<ColorStruct>` header in Rust `ParticleType` | Save format is independent (snapshot project); idiomatic `Vec<Rgb>` is cleaner; binary layout preserved as doc reference |
