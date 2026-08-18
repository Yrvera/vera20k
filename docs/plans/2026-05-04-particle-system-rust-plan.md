# Particle System (Tier 2) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained and ends in a commit. Tier 2 covers `Smoke`, `Gas`, and `Fire` BehavesLike variants only — `Spark` and `Railgun` are parsed-but-deferred until a separate render-design pass.

**Goal:** Land `ParticleSystemClass` + `ParticleClass` in Rust as authoritative sim state for Tier 2 BehavesLike variants (Smoke / Gas / Fire), retire the `DamageFireOverlays` placeholder, and wire every consumer-side spawn path covered by retail YR INI.

**Architecture:** Two new deterministic stores (`ParticleSystemStore` and per-PSC `Vec<Particle>`), interned `ParticleSystemTypeId(u32)` / `ParticleTypeId(u32)`, new tick phase 5.5 between combat (phase 5) and retaliation (phase 6), included in `state_hash`, all math fixed-point, all randomness via `sim.rng`. Existing SHP rendering pipeline reused; pixel-render path for Spark/Railgun deferred.

**Design Doc:** [docs/plans/2026-05-04-particle-system-rust-architecture-design.md](2026-05-04-particle-system-rust-architecture-design.md)

---

## Grounding Summary

- **`ra2-rust-game-docs/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md`** — sections §1–§11 cover every binary-verified offset, formula, behavior, and edge case for Tier 2. §11 is the gap-closing pass that resolved every prior open question. All addresses below cite this doc.
- **Ghidra verification:** all primary functions confirmed at the addresses cited in §11.11 (PSType ReadINI 0x006442D0, PType ReadINI 0x00644F50, PSC ctor 0x0062DC50, ParticleClass ctor 0x0062B5E0, system AIs at 0x0062ED40 / 0x0062E6D0 / 0x0062F9A0, particle AIs at 0x0062BD50 / 0x0062C540 / 0x0062CB10). All TS-legacy verdicts in §11.5 are binary-traced.
- **Repo pattern mirrored:** `Animation` runtime component in [src/sim/animation.rs](../../src/sim/animation.rs) (frame-timing accumulator pattern), `WeaponType` in [src/rules/weapon_type.rs](../../src/rules/weapon_type.rs) (rules-side struct + INI parse), `EntityStore` in [src/sim/entity_store.rs](../../src/sim/entity_store.rs) (`BTreeMap<u64, T>` deterministic store), `Simulation::advance_tick` in [src/sim/world/mod.rs](../../src/sim/world/mod.rs) (13-phase tick).
- **INI keys driving behavior:**
  - `[ParticleSystems]` master list section header in `rulesmd.ini`
  - `[Particles]` master list section header
  - 13 PSType entries (GasCloudSys, FireStreamSys, BigGreySmokeSys, SmallGreySSys, DebrisSmokeSys, SparkSys, FirestormSparkSys, TestSmokeSys, SmallRailgunSys, LargeRailgunSys, WeldingSys, LGSparkSys, PsychCloudSys) per §10.5.1
  - 22 PType entries per §10.5.2
  - Consumer keys: `DamageParticleSystems`, `DestroyParticleSystems` (DEAD per §11.5.C), `RefinerySmokeParticleSystem`, `NaturalParticleSystem`, `RefinerySmokeOffsetOne..Four`, `DamageSmokeOffset`, `DestroySmokeOffset`, `GapGenerator`, `AttachedParticleSystem`, `UseFireParticles`, `UseSparkParticles` (Tier 3), `BarrelParticle` ([General] not [AudioVisual] per §11.8.H), `DefaultSparkSystem` and 8 sibling [CombatDamage] slots per §11.8.G
- **Existing placeholder:** `DamageFireOverlays` struct lives in [src/sim/components.rs:475-502](../../src/sim/components.rs) and is a field on `GameEntity` ([src/sim/game_entity.rs:103](../../src/sim/game_entity.rs)), but its **tick logic runs in app layer** at [src/app_building_anim.rs:66-202](../../src/app_building_anim.rs) and rendering at [src/app_instances/overlays.rs:109](../../src/app_instances/overlays.rs). Retirement removes the struct, the field, the app-side tick, and the render branch.
- **Still unknown after grounding:** the precise iteration order of cell occupants for gas-damage application — this needs a quick verification pass during Task C6 before relying on it for determinism. Captured as a deferred open question.

## Key Technical Decisions

- **Two-store sim model: `ParticleSystemStore` alongside `EntityStore`.** Rationale: 6 spawn paths produce parentless PSCs (TriggerAction at waypoint, Scenario_Start global, area damage, EBolt visual, BarrelParticle, refinery dump after harvester leaves). A subsystem-on-parent approach can't represent these. **Confidence:** HIGH. **Source:** §5.2 of report (particles not in global object list), design doc §"Architecture Context".
- **Authoritative sim state, new tick phase 5.5.** Rationale: gas and fire deal gameplay damage; deterministic damage requires sim-tick placement. Phase 5.5 is between combat (phase 5, `// --- Phase 5: Turrets + Combat ---` at src/sim/world/mod.rs:1163) and retaliation (phase 6, `// --- Phase 6: Retaliation + Passengers ---` at line 1264) so retaliation can see damage applied this tick. The `5.5` numbering matches the existing `Phase 4.5: Superweapons` precedent at line 1156. **Confidence:** HIGH. **Source:** binary §11.6 places PSC AI in the global object loop; design Q3 chose A; phase numbering verified in src/sim/world/mod.rs.
- **Interned `ParticleSystemTypeId(u32)` / `ParticleTypeId(u32)`.** Rationale: matches binary's index-into-vector storage at TTC+0x764, avoids per-spawn HashMap lookup, aligns with the broader String→u32 type-ref migration. **Confidence:** HIGH. **Source:** §2.1/§2.2 of report (BehavesLike enum is `int (index)`), CLAUDE.md memory `project_string_interning.md`, design Q4 chose B.
- **Tier 2 scope: `Smoke + Gas + Fire` only via existing SHP pipeline.** Rationale: Spark + Railgun need a new pixel-write render path (3-byte RGB direct-to-screen) that doesn't exist today; that warrants its own design pass. **Confidence:** HIGH. **Source:** §7 of report covers pixel-render path; design Q2 chose Tier 2.
- **`Vec<Particle>` per PSC, not `SmallVec` or arena.** Rationale: simplest match for the binary's `DynamicVectorClass<ParticleClass*>`. Most PSCs cap at <50 particles per `ParticleCap`. **Confidence:** MEDIUM. **Source:** §2.3 of report; revisit if profiling shows allocation pressure.
- **Pure `Vec<Rgb>` for ColorList, not embedded vector header.** Rationale: snapshot format is independent (per the in-flight snapshot project in CLAUDE.md memory); idiomatic Rust over binary-mirror layout. The binary §11.1 layout stays as a doc reference. **Confidence:** HIGH. **Source:** §11.1 of report.
- **Tick.rs split into 3 files (system_ai.rs / particle_ai.rs / movement.rs)** instead of the design doc's single 600-line tick.rs. Rationale: CLAUDE.md ~600-line guidance; three cohesive units (system-level AI dispatch, per-particle AI per BehavesLike, movement helpers). **Confidence:** HIGH. **Source:** CLAUDE.md "Aim for ~600 lines per file".

## Open Questions

### Resolved During Planning

- *Where does `DamageFireOverlays` actually tick today?* — It's a sim-side struct (lives on `GameEntity` in [src/sim/components.rs:475](../../src/sim/components.rs)) but its tick logic runs in app layer at [src/app_building_anim.rs:66](../../src/app_building_anim.rs). Retirement removes both.
- *Where does refinery dump cycle live?* — [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) (per grep results).
- *Where does barrel destruction happen?* — Area damage path in [src/sim/combat/combat_aoe.rs](../../src/sim/combat/combat_aoe.rs); barrel state in [src/sim/components.rs](../../src/sim/components.rs) and [src/sim/world/world_spawn.rs](../../src/sim/world/world_spawn.rs).
- *Where does gap generator state live?* — [src/sim/vision/mod.rs](../../src/sim/vision/mod.rs) (vision system handles cloak/gap shroud).

### Deferred to Implementation

- **Cell-occupant iteration order for gas damage application.** Task C6 must verify (via grep) that `Cell.objects` (or whatever the equivalent is in `src/sim/`) iterates deterministically. If not, the pre-existing bug must be fixed there before particle damage relies on it.
- **`condition_yellow` representation in sim vs app.** Today [src/app_ui_overlays.rs:766](../../src/app_ui_overlays.rs) uses `f32` for the threshold. The sim-side spawn from `DamageParticleSystems` (Task D2) must use whatever representation `Health` uses internally — likely `I16F16` ratio. Confirm at task time.
- **Whether `BarrelParticle` parser site already exists.** RulesClass `[General]` parsing exists somewhere in `src/rules/`; Task A5 locates it and either extends or creates it.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/rules/particle_type.rs` | `ParticleType` struct, `ParticleTypeId`, `ParticleBehavesLike`, INI parser |
| Create | `src/rules/particle_system_type.rs` | `ParticleSystemType`, `ParticleSystemTypeId`, `ParticleSystemBehavesLike`, INI parser |
| Modify | `src/rules/ruleset.rs` | Add particle type registries + 2-pass resolve scaffolding |
| Modify | `src/rules/weapon_type.rs:200-240` | Migrate `attached_particle_system: Option<String>` → `Option<ParticleSystemTypeId>` |
| Modify | `src/rules/object_type.rs` (or wherever TechnoType lives — locate in Task A5) | Add particle-related TTC fields |
| Modify | `src/rules/ruleset.rs` (or general.rs / combat_damage.rs as found) | Add `BarrelParticle` + 9 `[CombatDamage]` Default*System fields |
| Create | `src/sim/particles/mod.rs` | `ParticleSystem`, `Particle` structs, `ParticleSystemStore`, public API |
| Create | `src/sim/particles/spawn.rs` | `spawn_particle_system`, `SpawnParticle`, `SpawnParticleWithInsert` |
| Create | `src/sim/particles/system_ai.rs` | Per-BehavesLike system AI (Smoke / Gas / Fire) |
| Create | `src/sim/particles/particle_ai.rs` | Per-particle AI dispatch and per-variant tick |
| Create | `src/sim/particles/movement.rs` | `Move_Smoke`, `Move_Gas`, fire inline movement |
| Create | `src/sim/particles/damage.rs` | Gas/fire cell-occupant damage application |
| Create | `src/sim/particles/wind.rs` | Wind drift table constants |
| Create | `src/render/particles.rs` | SHP draw collection for gas/smoke/fire particles |
| Modify | `src/sim/world/mod.rs` | Add `particle_systems` field to `Simulation`, insert tick phase 5.5 (between Phase 5 combat and Phase 6 retaliation) |
| Modify | `src/sim/world/world_hash.rs` | Include `ParticleSystemStore` in `state_hash` |
| Modify | `src/sim/combat/mod.rs` | Spawn `AttachedParticleSystem` in fire path; spawn `DamageParticleSystems` on health threshold |
| Modify | `src/sim/combat/combat_aoe.rs` | Spawn `BarrelParticle` on barrel destruction |
| Modify | `src/sim/miner/miner_dock_sequence.rs` | Spawn `RefinerySmokeParticleSystem` on dump cycle |
| Modify | `src/sim/vision/mod.rs` | Spawn `NaturalParticleSystem` on gap-generator state 3→0 |
| Modify | `src/app_render/build_instances.rs` | Iterate `ParticleSystemStore`, emit sprite instances |
| Delete | `DamageFireOverlays` / `DamageFireAnim` from `src/sim/components.rs` and field from `src/sim/game_entity.rs` (Task E4) |
| Delete | `tick_damage_fire_overlays` from `src/app_building_anim.rs` and call site in `src/app_sim_tick.rs:164` (Task E4) |
| Delete | `DamageFireAnim` rendering branch in `src/app_instances/overlays.rs:109` (Task E4) |

## Interface Changes

**New public types** (in `src/rules/`):
- `ParticleType`, `ParticleTypeId(u32)`, `ParticleBehavesLike`
- `ParticleSystemType`, `ParticleSystemTypeId(u32)`, `ParticleSystemBehavesLike`

**New public types** (in `src/sim/particles/`):
- `ParticleSystem`, `Particle`, `ParticleSystemStore`

**New `Ruleset` methods:**
- `particle_type(&self, id: ParticleTypeId) -> &ParticleType`
- `particle_system_type(&self, id: ParticleSystemTypeId) -> &ParticleSystemType`
- `p_type_id_by_name(&self, name: &str) -> Option<ParticleTypeId>`
- `ps_type_id_by_name(&self, name: &str) -> Option<ParticleSystemTypeId>`

**New `Simulation` methods:**
- `particle_systems(&self) -> &ParticleSystemStore`
- `particle_systems_mut(&mut self) -> &mut ParticleSystemStore`
- `spawn_particle_system(&mut self, type_id, coords, attached_entity, owner_entity, target_coords, owner_house) -> Option<u64>`

**Modified field types:**
- `WeaponType.attached_particle_system`: `Option<String>` → `Option<ParticleSystemTypeId>`. **Affected:** every consumer of this field (Task B0 audits before changing).

**Removed types (Task E4):**
- `DamageFireOverlays`, `DamageFireAnim` — every caller must be removed in the same commit.

## Sim Checklist

(All tasks touching `src/sim/`.)

- [x] All math uses `I16F16` fixed-point — no f32/f64 in sim logic. Wind drift `i32`, lifetime `i32`, animation state `u8`. Translucency byte preserved as `u8`. Float-style velocity → `I16F16`.
- [x] New state included in deterministic state hash — `ParticleSystemStore` hashed in Task B3 / state_hash extension.
- [x] No dependencies on `render/`, `ui/`, `sidebar/`, `audio/`, `net/` from `src/sim/particles/` — verified by Task B1 (module skeleton).
- [x] Tick ordering impact noted — new phase 5.5 between combat (phase 5) and retaliation (phase 6). Matches the existing `Phase 4.5: Superweapons` precedent. Replays from before this plan won't match after; acceptable since particle behaviour was previously absent.
- [x] BTreeMap iteration order preserved — `ParticleSystemStore` uses `BTreeMap<u64, ParticleSystem>` per Task B2; `Vec<Particle>` is index-ordered per insertion which matches binary's vector-append semantics.

## Risk Areas

| Risk | Mitigation |
|------|------------|
| `WeaponType.attached_particle_system` migration breaks consumers | Task B0 audits all consumers and updates them in the same commit as the field change (Task A5 step 4) |
| 2-pass parse changes Ruleset construction order; existing rules-parser tests may need updates | Task A4 runs the existing rules test suite after the 2-pass scaffolding lands |
| Tick-phase insertion shifts state_hash output; replays from before this plan don't match after | Documented in design; acceptable. Save-game compat across plan boundary explicitly broken. New phase numbering is `5.5` to match the existing `Phase 4.5: Superweapons` convention. |
| DamageFireOverlays retirement leaves a visible regression if smoke PSCs don't render correctly | Task E3 is a side-by-side visual parity check before E4 deletes the placeholder. |
| Cell-occupant iteration during gas damage may not be deterministic today | Task C6 step 1 verifies before relying on it. |
| Smoke double-spawn (§9.6 of report) is easy to forget when porting | Task C5 has an explicit unit test for the two-child symmetric spawn. |
| Spawning a Spark/Railgun PSC at Tier 2 silently no-ops | Task B4 returns `None` and warns; downstream code must check the `Option<u64>`. |

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| C2 | Smoke `NextParticle` two-child symmetric spawn | Visible smoke trail forks into two streams when smoke chains; binary §9.6 confirmed via decompile, prior research had it as one | Unit test in C5 + visual check against gamemd.exe |
| C3 | Gas vs Smoke wind table SE-direction asymmetry (gas DX[3]=1, smoke DX[3]=2) | Gas drifts subtly differently than smoke under SE wind; player would see if smoke and gas drift identically | Constant tables in `wind.rs` + unit test asserting both tables match §10.14.4 |
| C3 | Gas frames-per-shift = `10 / WindEffect`, drift accumulator clamped [-2, +2] | Gas cloud movement timing; player observes diffusion rate | Unit test stepping a gas particle N ticks under fixed wind |
| C4 | Fire stream death on rising terrain (`old_ground < new_ground`) | Flamethrower fire stops at cliffs in retail YR; without this, fire would tunnel through hills | Unit test: spawn fire near a cliff, verify particle marked deleted |
| C4 | Fire FinalDamageState gate (damage stops before animation ends, default state 14 of 19) | Flamethrower stops dealing damage before flame animation fades out — visible in HP bars | Unit test: tick a fire particle past FinalDamageState, assert no further damage |
| C6 | Gas damage hits ALL objects in cell (no friend/foe filter) | Gas is area-denial; players expect it to hurt allies too | Unit test: place 2 same-house units in a cell, spawn gas, both lose health |
| D1 | Per-shot `AttachedParticleSystem` spawn timing in fire path | Tracer / projectile particles must appear on the same tick the bullet spawns; off-by-one would look unsynchronized | Manual in-game observation against gamemd.exe |
| D2 | `DamageParticleSystems` smoke filter on `ReceiveDamage` (Smoke=0 only; Spark filter is for AI_Update Tier-3 path) | Smoke spawn timing on damaged buildings; player observes when smoke first appears | Unit test: damage building below ConditionYellow, verify smoke PSC spawns and uses BehavesLike==Smoke entries only |
| D3 | Refinery 4-spawn pattern (one per `RefinerySmokeOffset{N}` slot, skip sentinel) | Refinery dump animation produces 4 distinct smoke plumes at fixed offsets; missing one = visible asymmetry | Unit test: simulated dump, count spawns + compare offsets to TTC+0x7CC..0x7F8 |
| D5 | `BarrelParticle` lives in `[General]` not `[AudioVisual]` (§11.8.H corrects scoping pass) | Without correct INI section, the key wouldn't parse and barrel-destruction smoke would be silent | Parser test asserting `[General] BarrelParticle=` is read |
| Translucency byte mapping | 0x00 → 0x2800 opaque, 0x19 → 0x2802 50%, 0x32 → 0x2804 25%, 0x4A+ → 0x2806 fade | Per §7 of report; visible smoke/gas fade-out cadence | Render-side unit test asserting the byte→flag mapping |
| Spawn cap enforcement (`ParticleCap`) | Default 50; smoke caps at type-defined limit; without cap, GPU instance buffer would overflow | Unit test: aggressively spawn into a capped PSC, assert vector never exceeds cap |
| Fire `SpawnParticleWithInsert` random insertion | Fire stream visual variety relies on out-of-order insertion within last N entries; strict creation-order looks too uniform | Unit test: spawn 10 fire particles, assert vector order is not monotonically increasing |
| C2 spawn-timer accumulator (`Slowdown` adds to `+0xE8` per tick, triggers `done_spawning` when > `SpawnCutoff`) | Smoke fade-out timing on damaged buildings; off would mean smoke never stops or stops instantly | Unit test stepping accumulator against §3.3 formula |

---

## Tasks

The plan has 25 numbered tasks across 5 phases. Phase A is rules-side, Phase B is sim-runtime data and the spawn API, Phase C is the AI implementations, Phase D is consumer-side hookups, Phase E is rendering and retirement.

Tasks within a phase generally have intra-phase dependencies (later tasks depend on earlier ones in the same phase). Cross-phase dependencies are: A→B (sim runtime needs rules types), B→C (AI needs runtime), C→D (hookups need AI working), and E depends on D.

---

### Task A1: Add particle ID newtypes + BehavesLike enums + RGB helper

**Why:** Foundation for all subsequent rules code. ID types are `Copy` u32; enums preserve binary's asymmetric variant ordering (Smoke=0/Gas=1 for systems, Gas=0/Smoke=1 for particles per §2.1/§2.2 of report).

**Files:**
- Create: `src/rules/particle_type.rs` (skeleton, ~40 lines this task)
- Create: `src/rules/particle_system_type.rs` (skeleton, ~40 lines this task)

**Pattern:** Newtype-pattern around `u32` matching CLAUDE.md memory `project_string_interning.md` direction. Enum variants ordered to match binary indexing.

**Step 1: Create `src/rules/particle_type.rs` skeleton**
```rust
//! ParticleType — runtime parameters for a single particle (gas, smoke, fire, spark, railgun).
//! Source: PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §2.2.

use serde::{Deserialize, Serialize};

/// Interned identifier for a `ParticleType`. Resolved at INI parse time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ParticleTypeId(pub u32);

/// Per-particle behavior dispatch enum.
///
/// IMPORTANT: variant ordering matches the binary's string table at 0x008370BC.
/// The [Particles] section uses `Gas=0, Smoke=1, Fire=2, Spark=3, Railgun=4`,
/// which is DIFFERENT from [ParticleSystems] (see ParticleSystemBehavesLike).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ParticleBehavesLike {
    Gas = 0,
    Smoke = 1,
    Fire = 2,
    Spark = 3,
    Railgun = 4,
}

impl ParticleBehavesLike {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Gas" => Some(Self::Gas),
            "Smoke" => Some(Self::Smoke),
            "Fire" => Some(Self::Fire),
            "Spark" => Some(Self::Spark),
            "Railgun" => Some(Self::Railgun),
            _ => None,
        }
    }
}
```

**Step 2: Create `src/rules/particle_system_type.rs` skeleton**
```rust
//! ParticleSystemType — container that owns particles, manages spawning, dispatches AI.
//! Source: PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §2.1.

use serde::{Deserialize, Serialize};

/// Interned identifier for a `ParticleSystemType`. Resolved at INI parse time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ParticleSystemTypeId(pub u32);

/// System-level behavior dispatch.
///
/// Variant ordering matches the binary's string table at 0x00836EE0:
/// `Smoke=0, Gas=1, Fire=2, Spark=3, Railgun=4`. Note this DIFFERS from
/// `ParticleBehavesLike` — Smoke and Gas are swapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ParticleSystemBehavesLike {
    Smoke = 0,
    Gas = 1,
    Fire = 2,
    Spark = 3,
    Railgun = 4,
}

impl ParticleSystemBehavesLike {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Smoke" => Some(Self::Smoke),
            "Gas" => Some(Self::Gas),
            "Fire" => Some(Self::Fire),
            "Spark" => Some(Self::Spark),
            "Railgun" => Some(Self::Railgun),
            _ => None,
        }
    }
}
```

**Step 3: Add module declarations in `src/rules/mod.rs`**
```rust
pub mod particle_type;
pub mod particle_system_type;
```

**Step 4: Add unit tests for the asymmetric enum mappings**
```rust
// In particle_type.rs:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_string_to_enum() {
        // Per binary string table at 0x008370BC
        assert_eq!(ParticleBehavesLike::parse("Gas"),     Some(ParticleBehavesLike::Gas));
        assert_eq!(ParticleBehavesLike::parse("Smoke"),   Some(ParticleBehavesLike::Smoke));
        assert_eq!(ParticleBehavesLike::parse("Fire"),    Some(ParticleBehavesLike::Fire));
        assert_eq!(ParticleBehavesLike::parse("Spark"),   Some(ParticleBehavesLike::Spark));
        assert_eq!(ParticleBehavesLike::parse("Railgun"), Some(ParticleBehavesLike::Railgun));
        assert_eq!(ParticleBehavesLike::parse("nope"),    None);
    }

    #[test]
    fn behaves_like_discriminants_match_binary() {
        // String index in binary's table = enum discriminant
        assert_eq!(ParticleBehavesLike::Gas as u8,     0);
        assert_eq!(ParticleBehavesLike::Smoke as u8,   1);
        assert_eq!(ParticleBehavesLike::Fire as u8,    2);
        assert_eq!(ParticleBehavesLike::Spark as u8,   3);
        assert_eq!(ParticleBehavesLike::Railgun as u8, 4);
    }
}

// In particle_system_type.rs — symmetric tests confirming Smoke=0, Gas=1.
```

**Step 5: Verify**
```
cargo build -p <crate>
cargo test -p <crate> particle_type particle_system_type
```
Expected: builds clean, 4 tests pass.

**Step 6: Commit**
`git commit -m "particles: add ID newtypes and BehavesLike enums (asymmetric ordering preserved)"`

---

### Task A2: Define `ParticleType` struct and INI parser

**Why:** Tier 3 fields (Spark/Railgun: XVelocity, ColorList, etc.) are parsed but unused at Tier 2 — the binary parses them unconditionally and we must too so Tier 3 lights up cleanly later.

**Files:**
- Modify: `src/rules/particle_type.rs` (~250 lines total after this task)

**Pattern:** `WeaponType::from_ini_section` in [src/rules/weapon_type.rs](../../src/rules/weapon_type.rs).

**Step 1: Add struct fields per §2.2 / §11.1 of report**
```rust
use crate::util::fixed_math::SimFixed;
use crate::util::interned::InternedId;
use crate::rules::warhead_type::WarheadId;
use glam::IVec3;

#[derive(Debug, Clone)]
pub struct ParticleType {
    // Identity
    pub name: InternedId,
    pub behaves_like: ParticleBehavesLike,

    // Object-base (from ObjectTypeClass)
    pub image: Option<String>,                 // SHP via ObjectTypeClass::ReadINI Image=

    // From [Particles] direct keys (§2.2)
    pub max_dc: u16,                            // damage countdown reset
    pub max_ec: u16,                            // lifetime in frames
    pub damage: i32,
    pub warhead: Option<WarheadId>,
    pub start_frame: u16,
    pub num_loop_frames: u16,
    pub translucency: u8,                       // 0/25/50
    pub wind_effect: u8,                        // 0..5
    pub velocity: SimFixed,
    pub deacc: SimFixed,
    pub radius: i32,
    pub delete_on_state_limit: bool,
    pub end_state_ai: u8,
    pub start_state_ai: u8,
    pub state_ai_advance: u8,                   // default 4
    pub final_damage_state: u8,                 // default = end_state_ai if INI absent (§9.2)
    pub translucent_25_state: u8,               // default 0xFF
    pub translucent_50_state: u8,               // default 0xFF
    pub normalized: bool,
    pub next_particle: Option<ParticleTypeId>,  // resolved in 2nd pass (§A4)
    pub next_particle_offset: IVec3,            // CoordStruct

    // ColorList runtime — pure Vec, NOT a vector header (per §11.1 of report)
    pub color_list: Vec<[u8; 3]>,               // packed RGB, 3 bytes per entry
    pub color_speed: SimFixed,
    pub start_color_1: [u8; 3],
    pub start_color_2: [u8; 3],

    // Spark-only (parsed but unused at Tier 2)
    pub x_velocity: i32,
    pub y_velocity: i32,
    pub min_z_velocity: i32,
    pub z_velocity_range: i32,
}
```

**Step 2: Implement `ParticleType::from_ini_section`**
- Mirror `WeaponType::from_ini_section` shape: `pub fn from_ini_section(name: &str, section: &IniSection) -> Self`
- For each field, read with the appropriate INI helper (`get_int`, `get_bool`, `get_float`, `get_string`, `get_color_rgb`).
- Defaults from §11.1:
  - `state_ai_advance`: 4 (NOT 0 — the ctor at 0x00644BE0 sets 4 explicitly)
  - `translucent_25_state` / `translucent_50_state`: 0xFF
  - `final_damage_state`: same as `end_state_ai` if absent (§9.2 — ReadINI uses prior +0x309 as default)
  - `next_particle`: `None` initially; the **string** is captured in a parse-time-only field for the 2-pass resolver
- `next_particle` cannot be resolved here (referenced PType may not be parsed yet). Store the **string name** in a private `pending_next_particle: Option<String>` field on a separate parse-state struct — see Task A4.
- `behaves_like` defaults to `Gas` if INI omits the key (matches binary fallthrough at the string-table loop).

**Step 3: Implement ColorList parser**
```rust
/// Parse `ColorList=R,G,B,R,G,B,...` into a Vec<[u8; 3]>.
/// Per §11.1: stride is 3 bytes, no padding; binary's strtok loop
/// reads triples. Empty / missing key returns empty Vec.
fn parse_color_list(value: Option<&str>) -> Vec<[u8; 3]> {
    let Some(raw) = value else { return Vec::new(); };
    let mut nums = raw
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .map(|n| n.clamp(0, 255) as u8);
    let mut out = Vec::new();
    while let (Some(r), Some(g), Some(b)) = (nums.next(), nums.next(), nums.next()) {
        out.push([r, g, b]);
    }
    out
}
```

**Step 4: Tests**
```rust
#[test]
fn color_list_packs_triplets() {
    let v = parse_color_list(Some("255,255,255,200,200,80,200,10,10,0,0,0"));
    assert_eq!(v, vec![[255,255,255], [200,200,80], [200,10,10], [0,0,0]]);
}

#[test]
fn color_list_handles_partial_trailing() {
    // 5 numbers — only one full triplet
    let v = parse_color_list(Some("1,2,3,4,5"));
    assert_eq!(v, vec![[1,2,3]]);
}

#[test]
fn color_list_empty_or_missing() {
    assert_eq!(parse_color_list(None), Vec::<[u8;3]>::new());
    assert_eq!(parse_color_list(Some("")), Vec::<[u8;3]>::new());
}

#[test]
fn from_ini_uses_documented_defaults() {
    // Empty section — every field at its constructor default per §9.2
    let section = IniSection::new("Foo");
    let pt = ParticleType::from_ini_section("Foo", &section);
    assert_eq!(pt.state_ai_advance, 4);
    assert_eq!(pt.translucent_25_state, 0xFF);
    assert_eq!(pt.translucent_50_state, 0xFF);
    assert_eq!(pt.color_list, Vec::<[u8;3]>::new());
    assert_eq!(pt.behaves_like, ParticleBehavesLike::Gas); // fallthrough default
}
```

**Step 5: Verify**
`cargo test -p <crate> particle_type`
Expected: 7 tests pass (3 from A1 + 4 new).

**Step 6: Commit**
`git commit -m "particles: parse ParticleType from [Particles] sections"`

---

### Task A3: Define `ParticleSystemType` struct and INI parser

**Why:** Same pattern as A2 but for the `[ParticleSystems]` master section. Tier 3 fields (Spark/Railgun) parsed but unused.

**Files:**
- Modify: `src/rules/particle_system_type.rs` (~180 lines after this task)

**Pattern:** Mirror Task A2.

**Step 1: Add struct fields per §2.1 / §11 corrections**
```rust
use crate::util::fixed_math::SimFixed;
use crate::util::interned::InternedId;
use crate::rules::particle_type::ParticleTypeId;
use glam::IVec3;

#[derive(Debug, Clone)]
pub struct ParticleSystemType {
    pub name: InternedId,
    pub behaves_like: ParticleSystemBehavesLike,

    pub holds_what: Option<ParticleTypeId>,    // resolved in 2nd pass
    pub spawns: bool,
    pub spawn_frames: u32,                     // default 1
    pub slowdown: SimFixed,                    // default 0.0
    pub particle_cap: u32,                     // default 50 (0x32)
    pub spawn_radius: i32,
    pub spawn_cutoff: SimFixed,
    pub spawn_translucency_cutoff: SimFixed,
    pub lifetime: i32,                         // default -1 (§9.1 correction)
    pub spawn_direction: IVec3,                // (0,0,0) default

    // Railgun (Tier 3 — parsed but unused)
    pub particles_per_coord: SimFixed,         // default 0.1
    pub spiral_delta_per_coord: SimFixed,      // default 0.025 (§9.1 correction)
    pub spiral_radius: SimFixed,               // default 25.0 (§9.1 correction)
    pub position_perturbation_coefficient: SimFixed,
    pub movement_perturbation_coefficient: SimFixed,
    pub velocity_perturbation_coefficient: SimFixed,

    // Spark (Tier 3 — parsed but unused)
    pub spawn_spark_percentage: SimFixed,
    pub spark_spawn_frames: u32,
    pub light_size: i32,
    pub one_frame_light: bool,
    pub laser: bool,
    pub laser_color: [u8; 3],
}
```

**Step 2: Implement `from_ini_section`**
- Same shape as A2.
- Defaults from §9.1 (Constructor at 0x006440A0):
  - `lifetime`: -1
  - `particles_per_coord`: 0.1 (NOT 0)
  - `spiral_delta_per_coord`: 0.025 (NOT 0.1)
  - `spiral_radius`: 25.0 (NOT 0 or 2.9)
  - `particle_cap`: 50
  - `spawn_frames`: 1
- `holds_what`: store the **string name** in a parse-state struct, resolved in A4.

**Step 3: Tests**
```rust
#[test]
fn defaults_match_binary_constructor() {
    let s = IniSection::new("Foo");
    let pst = ParticleSystemType::from_ini_section("Foo", &s);
    assert_eq!(pst.lifetime, -1);
    assert_eq!(pst.particles_per_coord, SimFixed::from_num(0.1));
    assert_eq!(pst.spiral_delta_per_coord, SimFixed::from_num(0.025));
    assert_eq!(pst.spiral_radius, SimFixed::from_num(25.0));
    assert_eq!(pst.particle_cap, 50);
    assert_eq!(pst.spawn_frames, 1);
    assert_eq!(pst.behaves_like, ParticleSystemBehavesLike::Smoke); // index-0 fallthrough
}

#[test]
fn behaves_like_smoke_is_zero() {
    // Critical asymmetric mapping: PSC enum has Smoke=0, NOT Gas=0
    assert_eq!(ParticleSystemBehavesLike::Smoke as u8, 0);
}
```

**Step 4: Verify**
`cargo test -p <crate> particle_system_type`
Expected: 5+ tests pass.

**Step 5: Commit**
`git commit -m "particles: parse ParticleSystemType from [ParticleSystems] sections"`

---

### Task A4: Wire into `Ruleset` with 2-pass resolver

**Why:** ParticleType.next_particle and ParticleSystemType.holds_what reference other types by name. Resolve them after all types are loaded so order in INI doesn't matter.

**Files:**
- Modify: `src/rules/ruleset.rs`
- Modify: `src/rules/particle_type.rs` (add a `Pending` shape for unresolved name)
- Modify: `src/rules/particle_system_type.rs` (same)

**Pattern:** Two-phase parse — collect raw entries first, then resolve cross-references. New pattern in this codebase; document explicitly.

**Step 1: Add `PendingParticleType` and `PendingParticleSystemType` parse-state structs**
```rust
// In particle_type.rs
pub(crate) struct PendingParticleType {
    pub partial: ParticleType,                     // with next_particle = None
    pub next_particle_name: Option<String>,        // pending resolve
}

// In particle_system_type.rs
pub(crate) struct PendingParticleSystemType {
    pub partial: ParticleSystemType,
    pub holds_what_name: Option<String>,
}
```
Move the existing `from_ini_section` into a `from_ini_section_pending` returning these structs; provide a `finalize` method that takes the resolver indices.

**Step 2: Add registries to `Ruleset`**
```rust
pub struct Ruleset {
    // ... existing fields ...
    particle_types: Vec<Arc<ParticleType>>,
    particle_types_by_name: HashMap<String, ParticleTypeId>,
    particle_system_types: Vec<Arc<ParticleSystemType>>,
    particle_system_types_by_name: HashMap<String, ParticleSystemTypeId>,
}
```

**Step 3: Implement `Ruleset` accessors**
```rust
impl Ruleset {
    pub fn particle_type(&self, id: ParticleTypeId) -> &ParticleType {
        &self.particle_types[id.0 as usize]
    }
    pub fn particle_system_type(&self, id: ParticleSystemTypeId) -> &ParticleSystemType {
        &self.particle_system_types[id.0 as usize]
    }
    pub fn p_type_id_by_name(&self, name: &str) -> Option<ParticleTypeId> {
        self.particle_types_by_name.get(name).copied()
    }
    pub fn ps_type_id_by_name(&self, name: &str) -> Option<ParticleSystemTypeId> {
        self.particle_system_types_by_name.get(name).copied()
    }
}
```

**Step 4: Add 2-pass parse to `Ruleset::from_ini`**
```rust
// PHASE 1: collect all PendingParticleType from [Particles] section's referenced sections
let pending_p: Vec<PendingParticleType> = collect_p_pending(rules_ini);
let mut p_by_name: HashMap<String, ParticleTypeId> = HashMap::new();
for (idx, p) in pending_p.iter().enumerate() {
    p_by_name.insert(p.partial.name.to_string(), ParticleTypeId(idx as u32));
}

// PHASE 1: same for ParticleSystemType
let pending_pst: Vec<PendingParticleSystemType> = collect_pst_pending(rules_ini);
let mut pst_by_name: HashMap<String, ParticleSystemTypeId> = HashMap::new();
for (idx, pst) in pending_pst.iter().enumerate() {
    pst_by_name.insert(pst.partial.name.to_string(), ParticleSystemTypeId(idx as u32));
}

// PHASE 2: finalize ParticleType (resolve next_particle string -> ID)
let particle_types: Vec<Arc<ParticleType>> = pending_p.into_iter().map(|mut p| {
    if let Some(name) = p.next_particle_name {
        p.partial.next_particle = p_by_name.get(&name).copied();
        // missing reference: log warn-once and leave as None (per design "missing PType" handling)
    }
    Arc::new(p.partial)
}).collect();

// PHASE 2: finalize ParticleSystemType (resolve holds_what string -> ID)
let particle_system_types: Vec<Arc<ParticleSystemType>> = pending_pst.into_iter().map(|mut pst| {
    if let Some(name) = pst.holds_what_name {
        pst.partial.holds_what = p_by_name.get(&name).copied();
    }
    Arc::new(pst.partial)
}).collect();
```

**Step 5: Tests**
```rust
#[test]
fn two_pass_resolves_next_particle_regardless_of_order() {
    let ini = "
[Particles]
1=ChainEnd
2=ChainStart

[ChainStart]
NextParticle=ChainEnd
BehavesLike=Gas

[ChainEnd]
BehavesLike=Gas
";
    let rs = Ruleset::from_ini_str(ini).unwrap();
    let start_id = rs.p_type_id_by_name("ChainStart").unwrap();
    let end_id = rs.p_type_id_by_name("ChainEnd").unwrap();
    assert_eq!(rs.particle_type(start_id).next_particle, Some(end_id));
}

#[test]
fn two_pass_resolves_holds_what() {
    let ini = "
[Particles]
1=Smoke1
[ParticleSystems]
1=BigSmoke

[BigSmoke]
HoldsWhat=Smoke1
BehavesLike=Smoke

[Smoke1]
BehavesLike=Smoke
";
    let rs = Ruleset::from_ini_str(ini).unwrap();
    let s = rs.ps_type_id_by_name("BigSmoke").unwrap();
    let p = rs.p_type_id_by_name("Smoke1").unwrap();
    assert_eq!(rs.particle_system_type(s).holds_what, Some(p));
}

#[test]
fn missing_reference_logs_and_leaves_none() {
    let ini = "
[Particles]
1=GhostRef

[GhostRef]
NextParticle=DoesNotExist
BehavesLike=Gas
";
    let rs = Ruleset::from_ini_str(ini).unwrap();
    let id = rs.p_type_id_by_name("GhostRef").unwrap();
    assert_eq!(rs.particle_type(id).next_particle, None);
}
```

**Step 6: Verify**
- `cargo test -p <crate> ruleset` — pre-existing rules tests still pass
- `cargo test -p <crate> particle` — new 2-pass tests pass
- Check the existing rules-parse test suite for any test that depends on parse ordering — should still pass since we appended a new phase.

**Step 7: Commit**
`git commit -m "particles: 2-pass INI resolver for cross-type references"`

---

### Task A5: Migrate WeaponType + add TechnoType / General / CombatDamage particle fields

**Why:** Consumer-side. WeaponType field type changes (String → ID); TechnoType gains many fields; new RulesClass fields land. Doing all consumer-side parsing in one task because they all depend on the 2-pass resolver from A4 and share the parse phase.

**Files:**
- Modify: `src/rules/weapon_type.rs:200-240` — field type migration
- Modify: `src/rules/object_type.rs` (or wherever TechnoType-shared parsing lives) — new fields
- Modify: `src/rules/ruleset.rs` — General + CombatDamage fields (or new files if they don't exist)
- Audit: every consumer of `WeaponType.attached_particle_system`

**Pattern:** Mirror existing TechnoType field declarations. New CombatDamage parse follows §11.8.G's 9-slot enumeration.

**Step 1: Audit `WeaponType.attached_particle_system` consumers**
```
grep -rn "attached_particle_system" src/
```
Expected: 1-2 references (rules-side struct + maybe a render-side reader). Make a list.

**Step 2: Migrate `WeaponType` field**
```rust
// Before:
pub attached_particle_system: Option<String>,
// After:
pub attached_particle_system: Option<ParticleSystemTypeId>,
```
Update `from_ini_section` to resolve the name via `ruleset.ps_type_id_by_name(&s)` after the 2-pass phase. This means WeaponType parsing must happen AFTER ParticleSystemType — adjust `Ruleset::from_ini` ordering.

Update every consumer found in Step 1 to use the new type.

**Step 3: Add TechnoType particle fields**
Locate the TechnoType-shared base struct (likely in `src/rules/object_type.rs` since `OBJECT_TYPE` is the C++ ObjectTypeClass equivalent base, or a TechnoType-specific file). Add per §11.2:
```rust
// Particle-related TechnoType fields (binary offsets in §11.2 of report)
pub damage_particle_systems: Vec<ParticleSystemTypeId>,           // §11.2 TTC+0x778..0x78F
pub destroy_particle_systems: Vec<ParticleSystemTypeId>,          // §11.5.C — DEAD in YR but parse for completeness
pub refinery_smoke_particle_system: Option<ParticleSystemTypeId>, // TTC+0x774
pub natural_particle_system: Option<ParticleSystemTypeId>,        // TTC+0x764
pub natural_particle_location: IVec3,                             // TTC+0x768/0x76C/0x770
pub damage_smoke_offset: IVec3,                                   // TTC+0x7B0/0x7B4/0x7B8
pub dam_smk_off_scrn_rel: bool,                                   // TTC+0x7BC
pub destroy_smoke_offset: IVec3,                                  // TTC+0x7C0/0x7C4/0x7C8
pub refinery_smoke_offsets: [IVec3; 4],                           // TTC+0x7CC..0x7F8 (4 triplets)
pub gap_generator: bool,                                          // TTC+0xCD1
pub gap_radius_in_cells: u8,                                      // TTC+0xCD2
pub super_gap_radius_in_cells: u8,                                // TTC+0xCD3
```
Read each from the appropriate INI section. CSV-list keys (`DamageParticleSystems=A,B,C`) parse via `split(',').filter_map(|n| ruleset.ps_type_id_by_name(n.trim()))`.

**Step 4: Add `Ruleset::general.barrel_particle` field per §11.8.H**
```rust
// In whatever struct holds [General] keys (likely src/rules/ruleset.rs's General):
pub barrel_particle: Option<ParticleSystemTypeId>,    // [General] BarrelParticle=
```
Note §11.8.H corrected the section: it's `[General]`, NOT `[AudioVisual]`.

**Step 5: Add `[CombatDamage]` parsing per §11.8.G**
If a `CombatDamage` parser doesn't exist yet, create `src/rules/combat_damage.rs`:
```rust
//! [CombatDamage] section — global default particle systems used by
//! various combat effects.
//! Source: PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §11.8.G,
//! RulesClass::ReadCombatDamage @ 0x0066BBB0.

use crate::rules::particle_system_type::ParticleSystemTypeId;

#[derive(Debug, Clone, Default)]
pub struct CombatDamageDefaults {
    pub default_large_grey_smoke_system: Option<ParticleSystemTypeId>,    // +0x1018
    pub default_small_grey_smoke_system: Option<ParticleSystemTypeId>,    // +0x101C
    pub default_spark_system: Option<ParticleSystemTypeId>,               // +0x1020 (Tier 3 consumer)
    pub default_large_red_smoke_system: Option<ParticleSystemTypeId>,     // +0x1024
    pub default_small_red_smoke_system: Option<ParticleSystemTypeId>,     // +0x1028
    pub default_debris_smoke_system: Option<ParticleSystemTypeId>,        // +0x102C
    pub default_fire_stream_system: Option<ParticleSystemTypeId>,         // +0x1030 (Tier 2 consumer)
    pub default_test_particle_system: Option<ParticleSystemTypeId>,       // +0x1034
    pub default_repair_particle_system: Option<ParticleSystemTypeId>,     // +0x1038 (Tier 3 consumer)
}

impl CombatDamageDefaults {
    pub fn from_ini_with_resolver(
        section: &IniSection,
        resolve: impl Fn(&str) -> Option<ParticleSystemTypeId>,
    ) -> Self {
        Self {
            default_large_grey_smoke_system: section.get_string("DefaultLargeGreySmokeSystem").and_then(|s| resolve(&s)),
            // ... 8 more, same shape ...
        }
    }
}
```
Add a `pub combat_damage: CombatDamageDefaults` field to `Ruleset`.

**Step 6: Tests**
```rust
#[test]
fn techno_type_parses_damage_particle_systems_csv() {
    let ini = "
[Particles]
1=Sm
[ParticleSystems]
1=SparkSys
2=SmallGreySSys

[SparkSys]
BehavesLike=Spark
[SmallGreySSys]
BehavesLike=Smoke

[Sm]
BehavesLike=Smoke

[E1]    ; A unit type
DamageParticleSystems=SparkSys,SmallGreySSys
";
    let rs = Ruleset::from_ini_str(ini).unwrap();
    let e1 = rs.techno_type_by_name("E1").unwrap();
    assert_eq!(e1.damage_particle_systems.len(), 2);
}

#[test]
fn barrel_particle_lives_in_general_not_audiovisual() {
    let ini = "
[Particles]
1=Sm
[ParticleSystems]
1=SmallGreySSys

[SmallGreySSys]
BehavesLike=Smoke
[Sm]
BehavesLike=Smoke

[General]
BarrelParticle=SmallGreySSys
";
    let rs = Ruleset::from_ini_str(ini).unwrap();
    assert!(rs.general.barrel_particle.is_some());
}

#[test]
fn combat_damage_default_fire_stream_system_resolves() {
    let ini = "
[Particles]
1=Fp
[ParticleSystems]
1=FireStreamSys

[FireStreamSys]
BehavesLike=Fire
[Fp]
BehavesLike=Fire

[CombatDamage]
DefaultFireStreamSystem=FireStreamSys
";
    let rs = Ruleset::from_ini_str(ini).unwrap();
    assert!(rs.combat_damage.default_fire_stream_system.is_some());
}
```

**Step 7: Verify**
- `cargo build -p <crate>` — every WeaponType consumer compiles.
- `cargo test -p <crate>` — full rules suite passes including 3 new tests.

**Step 8: Commit**
`git commit -m "particles: migrate WeaponType + add TechnoType / General / CombatDamage particle keys"`

---

### Task B1: Create `src/sim/particles/` module skeleton with `ParticleSystem` and `Particle` structs

**Why:** Sim-side runtime data structures. Foundation for all subsequent sim-side work.

**Files:**
- Create: `src/sim/particles/mod.rs`
- Modify: `src/sim/mod.rs` to add `pub mod particles;`

**Pattern:** Module structure mirrors `src/sim/animation.rs` (data + functions, no impl-heavy methods).

**Step 1: Create `src/sim/particles/mod.rs`**
```rust
//! Particle systems — authoritative sim state for visual + damage particle effects.
//!
//! Two-tier model:
//!   - `ParticleSystem` — container that owns a `Vec<Particle>`, manages spawning,
//!     dispatches per-tick AI based on its `ParticleSystemBehavesLike` type.
//!   - `Particle` — individual entity with position, velocity, lifetime, animation
//!     state, optionally dealing damage to cell occupants (gas / fire variants).
//!
//! Stored in `Simulation::particle_systems: ParticleSystemStore` (BTreeMap).
//! Particles never enter `EntityStore` — they're owned by their parent PSC.
//!
//! Tier 2 implements Smoke / Gas / Fire via the existing SHP render pipeline.
//! Spark / Railgun are parsed but spawn returns None (warn + skip).

use crate::rules::particle_system_type::ParticleSystemTypeId;
use crate::rules::particle_type::ParticleTypeId;
use crate::sim::entity_store::EntityId;
use crate::sim::HouseId;
use crate::util::fixed_math::SimFixed;
use glam::IVec3;
use std::collections::BTreeMap;

pub mod spawn;
pub mod system_ai;
pub mod particle_ai;
pub mod movement;
pub mod damage;
pub mod wind;

#[derive(Debug, Clone)]
pub struct ParticleSystem {
    pub stable_id: u64,
    pub type_id: ParticleSystemTypeId,
    pub coords: IVec3,                          // current world position (leptons)
    pub offset: IVec3,                          // offset from attached object
    pub particles: Vec<Particle>,
    pub spawn_timer: SimFixed,                  // smoke spawn accumulator
    pub lifetime: i32,                          // -1 = until particles die
    pub spark_spawn_frames: i32,                // (Tier 3 unused at Tier 2)
    pub facing: u8,                             // default 29 (0x1D)
    pub marked_for_deletion: bool,
    pub directionless: bool,
    pub attached_entity: Option<EntityId>,
    pub owner_entity: Option<EntityId>,
    pub target_coords: IVec3,                   // (Tier 3 — railgun endpoint)
    pub owner_house: Option<HouseId>,
    pub done_spawning: bool,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub type_id: ParticleTypeId,
    pub coords: IVec3,
    pub previous_coords: IVec3,
    pub origin: IVec3,                          // float copy of spawn pos (binary uses float; we use IVec3 in leptons)
    pub direction: [SimFixed; 3],               // normalized direction
    pub velocity: SimFixed,
    pub lifetime_remaining: i16,
    pub damage_counter: i16,
    pub state_ai_advance: u8,
    pub animation_state: u8,
    pub translucency: u8,
    pub hit_ground: bool,
    pub marked_for_deletion: bool,

    // Per-BehavesLike scratch fields (overlapping in binary's union; explicit here)
    pub drift_x: i32,
    pub drift_y: i32,
    pub drift_z: i32,

    // Tier 3 color state (present but unused at Tier 2)
    pub current_color: [u8; 3],
    pub color_index: u8,
    pub color_accumulator: SimFixed,
}

impl ParticleSystem {
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}
```

**Step 2: Add `pub mod particles;` to `src/sim/mod.rs`** (after the existing modules).

**Step 3: Verify the sim-boundary invariant**
```
grep -E "use (crate::)?(render|ui|sidebar|audio|net)" src/sim/particles/
```
Expected: zero results. Sim does not depend on render/ui/audio/net.

**Step 4: Verify build**
`cargo build -p <crate>`
Expected: builds cleanly. (No tests yet — purely structural.)

**Step 5: Commit**
`git commit -m "particles: scaffold sim/particles module with ParticleSystem and Particle structs"`

---

### Task B2: Implement `ParticleSystemStore` (BTreeMap-based deterministic store)

**Why:** Public store API for inserting / removing / iterating PSCs. Mirrors `EntityStore` shape so determinism rules carry over.

**Files:**
- Modify: `src/sim/particles/mod.rs`

**Pattern:** Mirror [src/sim/entity_store.rs](../../src/sim/entity_store.rs).

**Step 1: Add the store**
```rust
#[derive(Debug, Clone, Default)]
pub struct ParticleSystemStore {
    systems: BTreeMap<u64, ParticleSystem>,
    next_id: u64,
}

impl ParticleSystemStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &ParticleSystem)> + '_ {
        self.systems.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&u64, &mut ParticleSystem)> + '_ {
        self.systems.iter_mut()
    }

    pub fn get(&self, id: u64) -> Option<&ParticleSystem> {
        self.systems.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut ParticleSystem> {
        self.systems.get_mut(&id)
    }

    /// Inserts a new system; returns the assigned stable id.
    pub fn insert(&mut self, mut sys: ParticleSystem) -> u64 {
        self.next_id += 1;
        sys.stable_id = self.next_id;
        let id = self.next_id;
        self.systems.insert(id, sys);
        id
    }

    /// Re-inserts a system at its existing stable id (used by tick borrow-juggle).
    pub fn reinsert(&mut self, sys: ParticleSystem) {
        let id = sys.stable_id;
        debug_assert!(id > 0, "reinsert requires a previously-assigned stable_id");
        self.systems.insert(id, sys);
    }

    pub fn remove(&mut self, id: u64) -> Option<ParticleSystem> {
        self.systems.remove(&id)
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// Stable iteration over IDs for tick traversal — collects to a Vec
    /// so the caller can mutate the store while ticking.
    pub fn ids(&self) -> Vec<u64> {
        self.systems.keys().copied().collect()
    }
}
```

**Step 2: Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn fake_system() -> ParticleSystem {
        ParticleSystem {
            stable_id: 0,
            type_id: ParticleSystemTypeId(0),
            coords: IVec3::ZERO,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(0),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1D,
            marked_for_deletion: false,
            directionless: false,
            attached_entity: None,
            owner_entity: None,
            target_coords: IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        }
    }

    #[test]
    fn insert_assigns_increasing_ids() {
        let mut store = ParticleSystemStore::new();
        let a = store.insert(fake_system());
        let b = store.insert(fake_system());
        assert!(b > a);
    }

    #[test]
    fn iteration_is_sorted_by_id() {
        let mut store = ParticleSystemStore::new();
        let _ = store.insert(fake_system());
        let _ = store.insert(fake_system());
        let _ = store.insert(fake_system());
        let ids: Vec<u64> = store.iter().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn reinsert_preserves_id() {
        let mut store = ParticleSystemStore::new();
        let id = store.insert(fake_system());
        let sys = store.remove(id).unwrap();
        store.reinsert(sys);
        assert!(store.get(id).is_some());
        assert_eq!(store.len(), 1);
    }
}
```

**Step 3: Verify**
`cargo test -p <crate> particle_system_store`

**Step 4: Commit**
`git commit -m "particles: ParticleSystemStore with deterministic BTreeMap iteration"`

---

### Task B3: Add `particle_systems` field on `Simulation`, insert tick phase 5.5 stub, extend state_hash

**Why:** Wires the store into the simulation tick loop and the determinism contract.

**Files:**
- Modify: `src/sim/world/mod.rs` (add field, add phase)
- Modify: `src/sim/world/world_hash.rs` (extend hash)

**Pattern:** Mirror how other deterministic stores hook in. Phase numbering follows the existing `Phase 4.5: Superweapons` precedent (a new sub-phase between numbered phases). State-hash extension follows the existing `hash_houses` / `hash_production` / `hash_entities` sub-method pattern in [src/sim/world/world_hash.rs:18-36](../../src/sim/world/world_hash.rs).

**Step 1: Add field to `Simulation`**
```rust
// src/sim/world/mod.rs
use crate::sim::particles::ParticleSystemStore;

pub struct Simulation {
    // ... existing fields ...
    pub particle_systems: ParticleSystemStore,
}
```
Initialize to `ParticleSystemStore::new()` in `Simulation::new` / `Simulation::default`.

**Step 2: Insert tick phase 5.5 stub**
In `Simulation::advance_tick`, between the existing **Phase 5: Turrets + Combat** (the `combat::tick_combat_with_fog(...)` call near line 1170 and its result-handling block) and **Phase 6: Retaliation + Passengers** (`combat::tick_retaliation(...)` at line 1266), add:
```rust
// --- Phase 5.5: ParticleSystems ---
// DEPENDS ON: combat (gas/fire damage spawned this tick).
// PRODUCES: damage applied via gas/fire particles, must be visible to phase 6 retaliation.
crate::sim::particles::system_ai::tick_particle_systems(self);
```
Where `tick_particle_systems` is a stub for now:
```rust
// src/sim/particles/system_ai.rs
pub fn tick_particle_systems(_sim: &mut crate::sim::Simulation) {
    // Implemented in Tasks C1–C4.
}
```

**Step 3: Extend state_hash via a new sub-method**
The existing `state_hash` at [src/sim/world/world_hash.rs:18-36](../../src/sim/world/world_hash.rs) is a 9-line dispatcher that delegates to ~11 named sub-methods (`hash_game_options`, `hash_houses`, `hash_production`, `hash_power_states`, `hash_fog_and_alliances`, `hash_bridge_state`, `hash_overlay_grid`, `hash_super_weapons`, `hash_entities`). Each uses `field.hash(hasher)` via the `Hash` trait, NOT byte-level `hasher.update(&bytes)`. Follow that pattern.

3a. Add a single delegation line to the dispatcher:
```rust
// src/sim/world/world_hash.rs — inside Simulation::state_hash (after hash_entities)
self.hash_particle_systems(&mut hasher);
```

3b. Add the sub-method as a sibling of the other `hash_*` methods:
```rust
/// Hash all particle systems in stable-id order (BTreeMap iteration).
/// Each system contributes its type, position, lifetime, and ordered particle list.
fn hash_particle_systems(&self, hasher: &mut impl Hasher) {
    self.particle_systems.len().hash(hasher);
    for (id, sys) in self.particle_systems.iter() {
        id.hash(hasher);
        sys.type_id.0.hash(hasher);
        sys.coords.x.hash(hasher);
        sys.coords.y.hash(hasher);
        sys.coords.z.hash(hasher);
        sys.lifetime.hash(hasher);
        sys.facing.hash(hasher);
        sys.marked_for_deletion.hash(hasher);
        sys.done_spawning.hash(hasher);
        sys.particles.len().hash(hasher);
        for p in &sys.particles {
            p.type_id.0.hash(hasher);
            p.coords.x.hash(hasher);
            p.coords.y.hash(hasher);
            p.coords.z.hash(hasher);
            p.lifetime_remaining.hash(hasher);
            p.animation_state.hash(hasher);
            p.translucency.hash(hasher);
            p.marked_for_deletion.hash(hasher);
        }
    }
}
```

If `Hash` isn't already derived on `ParticleSystemTypeId` / `ParticleTypeId`, derive it in Task A1 (it's already specified in the type definitions there — confirm).

**Step 4: Tests**
```rust
#[test]
fn empty_particle_store_hashes_consistently() {
    let mut a = Simulation::new(/* ... */);
    let mut b = Simulation::new(/* ... */);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn particle_state_changes_hash() {
    let mut sim = Simulation::new(/* ... */);
    let h1 = sim.state_hash();
    sim.particle_systems.insert(fake_system_with(/* coords */ IVec3::new(100, 0, 0)));
    let h2 = sim.state_hash();
    assert_ne!(h1, h2);
}
```

**Step 5: Verify**
`cargo test -p <crate> sim::world` — pre-existing world tests still pass; new state_hash tests pass.

**Step 6: Commit**
`git commit -m "particles: wire ParticleSystemStore into Simulation, add tick phase 5.5 stub, extend state_hash"`

---

### Task B4: Implement `spawn_particle_system` + `SpawnParticle` + `SpawnParticleWithInsert`

**Why:** Public spawn API. Every consumer (combat, refinery, gap-gen, area-damage) calls `world.spawn_particle_system(...)`. Internal `SpawnParticle` and `SpawnParticleWithInsert` are used by system AI for runtime particle creation (Smoke `NextParticle` chains, Fire's stream insertion).

**Files:**
- Modify: `src/sim/particles/spawn.rs`

**Pattern:** Per §3.1 / §10.1 of report. Three spawn variants — direct, with-insert (Fire), and the high-level public entry point.

**Step 1: Public spawn entry point**
```rust
//! Particle spawn helpers.
//! Source: PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §10.1, §11.

use super::{Particle, ParticleSystem, ParticleSystemStore};
use crate::rules::particle_system_type::{ParticleSystemBehavesLike, ParticleSystemTypeId};
use crate::sim::entity_store::EntityId;
use crate::sim::HouseId;
use crate::sim::Simulation;
use crate::util::fixed_math::SimFixed;
use glam::IVec3;

impl Simulation {
    /// Spawn a new particle system. Returns the new system's stable id, or `None` if:
    ///   - the type is `Spark` or `Railgun` (Tier 3 — not implemented yet)
    ///   - allocation failed
    pub fn spawn_particle_system(
        &mut self,
        type_id: ParticleSystemTypeId,
        coords: IVec3,
        attached_entity: Option<EntityId>,
        owner_entity: Option<EntityId>,
        target_coords: IVec3,
        owner_house: Option<HouseId>,
    ) -> Option<u64> {
        let pst = self.ruleset.particle_system_type(type_id);
        match pst.behaves_like {
            ParticleSystemBehavesLike::Spark | ParticleSystemBehavesLike::Railgun => {
                tracing::warn!(
                    target: "particles",
                    "Tier 3 PSC type {:?} requested at {:?} — skipped",
                    pst.behaves_like, coords,
                );
                return None;
            }
            _ => {}
        }
        let directionless = pst.spawn_direction == IVec3::ZERO;
        let sys = ParticleSystem {
            stable_id: 0, // assigned by store
            type_id,
            coords,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(pst.spawn_frames as i32),
            lifetime: pst.lifetime,
            spark_spawn_frames: pst.spark_spawn_frames as i32,
            facing: 0x1D,
            marked_for_deletion: false,
            directionless,
            attached_entity,
            owner_entity,
            target_coords,
            owner_house,
            done_spawning: false,
        };
        Some(self.particle_systems.insert(sys))
    }
}
```

**Step 2: Internal `spawn_particle` (used by system AI)**
```rust
/// Spawn a single particle into a PSC's vector.
/// Mirrors §10.1.1 (binary's standard SpawnParticle at 0x0062E380).
/// Returns `true` if the particle was added, `false` if `HoldsWhat` is unset.
pub(super) fn spawn_particle(
    sys: &mut ParticleSystem,
    coords: IVec3,
    spawn_origin: IVec3,
    rules: &crate::rules::Ruleset,
    rng: &mut crate::sim::rng::SimRng,
) -> bool {
    let pst = rules.particle_system_type(sys.type_id);
    let Some(pt_id) = pst.holds_what else { return false; };
    let pt = rules.particle_type(pt_id);

    // Lifetime per §9.4 — random%MaxEC + MaxEC for non-railgun, or random%10 for railgun.
    let lifetime_extra = if pt.behaves_like == crate::rules::particle_type::ParticleBehavesLike::Railgun {
        (rng.next_range_u32(10) as i16).abs()
    } else {
        let base = pt.max_ec.max(1) as u32;
        (rng.next_range_u32(base) as i16).abs()
    };
    let lifetime = pt.max_ec as i16 + lifetime_extra;

    let particle = Particle {
        type_id: pt_id,
        coords,
        previous_coords: spawn_origin,
        origin: coords,
        direction: [SimFixed::from_num(0); 3],
        velocity: pt.velocity,
        lifetime_remaining: lifetime,
        damage_counter: pt.max_dc as i16,
        state_ai_advance: pt.state_ai_advance,
        animation_state: pt.start_state_ai,
        translucency: pt.translucency,
        hit_ground: false,
        marked_for_deletion: false,
        drift_x: 0,
        drift_y: 0,
        drift_z: 0,
        current_color: [0; 3],     // Tier 3
        color_index: 0,             // Tier 3
        color_accumulator: SimFixed::from_num(0),
    };

    if sys.particles.len() < pst.particle_cap as usize {
        sys.particles.push(particle);
        true
    } else {
        false
    }
}
```

**Step 3: `spawn_particle_with_insert` (Fire only)**
```rust
/// Per §10.1.3. After appending, randomly reposition within the last `insert_range`
/// elements by shifting elements right. Used by Fire system AI for visual variety.
pub(super) fn spawn_particle_with_insert(
    sys: &mut ParticleSystem,
    coords: IVec3,
    spawn_origin: IVec3,
    insert_range: usize,
    rules: &crate::rules::Ruleset,
    rng: &mut crate::sim::rng::SimRng,
) -> bool {
    if insert_range == 0 || !spawn_particle(sys, coords, spawn_origin, rules, rng) {
        return false;
    }
    // After spawn_particle pushed the new particle at the end:
    let count = sys.particles.len();
    if count < 2 {
        return true; // nothing to shuffle
    }
    let actual_range = insert_range.min(count);
    let random_offset = rng.next_range_u32(actual_range as u32) as usize;
    let insert_pos = count.saturating_sub(2).saturating_sub(random_offset);
    if insert_pos + 1 >= count {
        return true; // already at end
    }
    // Pop the just-pushed last element and re-insert at insert_pos+1.
    let p = sys.particles.pop().unwrap();
    sys.particles.insert(insert_pos + 1, p);
    true
}
```

**Step 4: Tests**
```rust
#[test]
fn spawn_returns_none_for_spark_at_tier_2() {
    let mut sim = test_sim_with_ps_type(ParticleSystemBehavesLike::Spark);
    assert!(sim.spawn_particle_system(
        ParticleSystemTypeId(0), IVec3::ZERO, None, None, IVec3::ZERO, None
    ).is_none());
}

#[test]
fn spawn_returns_some_for_smoke() {
    let mut sim = test_sim_with_ps_type(ParticleSystemBehavesLike::Smoke);
    let id = sim.spawn_particle_system(
        ParticleSystemTypeId(0), IVec3::new(100, 100, 0), None, None, IVec3::ZERO, None,
    );
    assert!(id.is_some());
    assert_eq!(sim.particle_systems.len(), 1);
}

#[test]
fn spawn_particle_respects_particle_cap() {
    let mut sim = test_sim_with_ps_type_capped(ParticleSystemBehavesLike::Smoke, 3);
    let sys_id = sim.spawn_particle_system(
        ParticleSystemTypeId(0), IVec3::ZERO, None, None, IVec3::ZERO, None,
    ).unwrap();
    let sys = sim.particle_systems.get_mut(sys_id).unwrap();
    for _ in 0..10 {
        super::spawn::spawn_particle(sys, IVec3::ZERO, IVec3::ZERO, &sim.ruleset, &mut sim.rng);
    }
    assert_eq!(sys.particles.len(), 3);
}

#[test]
fn spawn_with_insert_does_not_exceed_cap() {
    let mut sim = test_sim_with_ps_type_capped(ParticleSystemBehavesLike::Fire, 5);
    // Same shape as above; assert vector never exceeds 5 after 10 inserts.
}
```

**Step 5: Verify**
`cargo test -p <crate> particles::spawn`

**Step 6: Commit**
`git commit -m "particles: spawn API (public + SpawnParticle + SpawnParticleWithInsert)"`

---

### Task C1: Wind tables + per-BehavesLike system AI dispatch skeleton

**Why:** Centralized wind constants (gas vs smoke tables differ at SE direction per §10.14.4 — parity-critical) and the dispatch entry point that Tier 2 ticks fill in.

**Files:**
- Create content: `src/sim/particles/wind.rs`
- Modify: `src/sim/particles/system_ai.rs`

**Pattern:** Per §8.1, §10.14.4 of report. Read the four tables from the binary at 0x836664 / 0x836684 / 0x008366A4 / 0x008366C4.

**Step 1: Wind tables**
```rust
//! Wind drift tables.
//! Source: PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §8.1 (gas) and §10.14.4 (smoke).
//! Binary tables at 0x00836664 / 0x00836684 / 0x008366A4 / 0x008366C4 verified.

/// Wind drift deltas indexed by FacingType 0..7 (N, NE, E, SE, S, SW, W, NW).
/// `[General] WindDirection=` is the index into these.

/// Gas DX / DY (binary 0x00836664 / 0x00836684).
pub const GAS_WIND_DX: [i32; 8] = [0, 2, 2, 1, 0, -2, -2, -2];
pub const GAS_WIND_DY: [i32; 8] = [-2, -2, 0, 2, 2, 2, 0, -2];

/// Smoke DX / DY (binary 0x008366A4 / 0x008366C4).
/// Differs from Gas at index 3 (SE): smoke=2, gas=1. Asymmetry is intentional in retail.
pub const SMOKE_WIND_DX: [i32; 8] = [0, 2, 2, 2, 0, -2, -2, -2];
pub const SMOKE_WIND_DY: [i32; 8] = [-2, -2, 0, 2, 2, 2, 0, -2];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_and_smoke_dx_differ_at_se() {
        assert_eq!(GAS_WIND_DX[3], 1);
        assert_eq!(SMOKE_WIND_DX[3], 2);
    }

    #[test]
    fn dy_tables_are_identical() {
        assert_eq!(GAS_WIND_DY, SMOKE_WIND_DY);
    }
}
```

**Step 2: System AI dispatch skeleton**
```rust
//! Per-tick AI dispatch for ParticleSystems.
//! Source: PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §3.1, §3.3–§3.7.

use crate::rules::particle_system_type::ParticleSystemBehavesLike;
use crate::sim::Simulation;

pub fn tick_particle_systems(sim: &mut Simulation) {
    let ids = sim.particle_systems.ids();
    for id in ids {
        let Some(mut sys) = sim.particle_systems.remove(id) else { continue; };
        tick_one_system(&mut sys, sim);

        // Lifetime decrement (§3.1)
        sys.lifetime -= 1;
        if sys.lifetime == 0 {
            sys.marked_for_deletion = true;
        }

        // Drop the system if it's done and out of particles (§3.1).
        if sys.marked_for_deletion && sys.particles.is_empty() {
            // System was removed above; skip reinsert.
        } else {
            sim.particle_systems.reinsert(sys);
        }
    }
}

fn tick_one_system(sys: &mut crate::sim::particles::ParticleSystem, sim: &mut Simulation) {
    let pst = sim.ruleset.particle_system_type(sys.type_id);
    match pst.behaves_like {
        ParticleSystemBehavesLike::Smoke   => super::system_ai_smoke::tick(sys, sim),
        ParticleSystemBehavesLike::Gas     => super::system_ai_gas::tick(sys, sim),
        ParticleSystemBehavesLike::Fire    => super::system_ai_fire::tick(sys, sim),
        ParticleSystemBehavesLike::Spark   => { /* Tier 3 — no-op */ }
        ParticleSystemBehavesLike::Railgun => { /* Tier 3 — no-op */ }
    }
}
```

If sub-modules per BehavesLike are inappropriate, fold the per-variant tick into `system_ai.rs` with named functions. Stylistic choice; decide based on file size projection.

**Step 3: Verify**
`cargo build && cargo test -p <crate> wind`

**Step 4: Commit**
`git commit -m "particles: wind tables + system AI dispatch skeleton"`

---

### Task C2: Implement Smoke system AI + smoke particle AI + Move_Smoke

**Why:** First Tier 2 BehavesLike. Smoke is the highest-volume particle type (building damage, refinery, debris). Includes the §9.6 two-child NextParticle finding — parity-critical.

**Files:**
- Modify: `src/sim/particles/system_ai.rs` (smoke system AI)
- Modify: `src/sim/particles/particle_ai.rs` (smoke particle AI)
- Modify: `src/sim/particles/movement.rs` (Move_Smoke)

**Pattern:** Per §3.3 (system) and §3.8 smoke / §10.2.3 (Move_Smoke) of report.

**Step 1: Smoke system AI**
```rust
// In system_ai.rs — smoke tick
pub(super) mod system_ai_smoke {
    use super::*;
    use crate::sim::particles::{Particle, ParticleSystem};
    use glam::IVec3;

    pub fn tick(sys: &mut ParticleSystem, sim: &mut Simulation) {
        // Phase 1: follow attached object (§3.3)
        if let Some(att) = sys.attached_entity {
            if let Some(entity) = sim.entities.get(att) {
                if entity.is_alive() {                       // see entity helper API
                    let new_pos = entity.coords() + sys.offset;
                    sys.coords = new_pos;
                }
            }
        }

        // Phase 2: tick all existing particles
        let pst = sim.ruleset.particle_system_type(sys.type_id).clone();
        for p in &mut sys.particles {
            super::particle_ai_smoke::tick(p, &pst, &mut sim.rng);
        }

        // Phase 3: handle dead-particle cleanup + NextParticle TWO-CHILD spawn (§9.6)
        let mut i = sys.particles.len();
        while i > 0 {
            i -= 1;
            if sys.particles[i].marked_for_deletion {
                let dying = sys.particles[i].clone();
                let pt = sim.ruleset.particle_type(dying.type_id);
                if let Some(next_id) = pt.next_particle {
                    // §9.6: spawn TWO child particles at symmetric (+dx,+dy) and (-dx,-dy).
                    let r = pt.radius >> 3; // type+0x304 >> 3 = Radius/8
                    let dx = symmetric_offset(r, &mut sim.rng);
                    let dy = symmetric_offset(r, &mut sim.rng);
                    spawn_smoke_child(sys, &dying, next_id, IVec3::new(dx, dy, 0), sim);
                    spawn_smoke_child(sys, &dying, next_id, IVec3::new(-dx, -dy, 0), sim);
                }
                sys.particles.remove(i);
            }
        }

        // Phase 4: spawn new particles if conditions met (§3.3)
        if !sys.done_spawning && pst.spawns {
            let timer_int = sys.spawn_timer.to_num::<i32>().max(1);
            if sim.tick_counter % timer_int as u64 == 0 {
                // gating: attached object null, not selected, OR health < 0
                let allow = match sys.attached_entity {
                    None => true,
                    Some(eid) => {
                        let e = sim.entities.get(eid);
                        e.map(|e| !e.is_selected() || e.health() < 0).unwrap_or(true)
                    }
                };
                if allow {
                    let r = pst.spawn_radius;
                    let off_x = (sim.rng.next_range_u32((r + 1) as u32) as i32);
                    let off_y = (sim.rng.next_range_u32((r + 1) as u32) as i32);
                    let spawn_pos = IVec3::new(
                        sys.coords.x + off_x,
                        sys.coords.y + off_y,
                        sys.coords.z + 10,
                    );
                    if let Some(holds) = pst.holds_what {
                        // SpawnParticle inline (avoid the public helper to thread sim borrowing)
                        let pt = sim.ruleset.particle_type(holds);
                        if sys.particles.len() < pst.particle_cap as usize {
                            // Apply translucency cutoff fade (§3.3)
                            let mut translucency = pt.translucency;
                            if pst.spawn_translucency_cutoff < sys.spawn_timer {
                                translucency = translucency.saturating_add(0x19);
                            }
                            let mut new_particle = make_particle(holds, spawn_pos, pt);
                            new_particle.translucency = translucency;

                            // Reduce velocity per §3.3 — `(accumulator - SpawnFrames) * 0.025`, clamped at 2.0.
                            let delta = sys.spawn_timer - SimFixed::from_num(pst.spawn_frames as i32);
                            let factor = SimFixed::from_num(0.025);
                            let new_v = new_particle.velocity - delta * factor;
                            new_particle.velocity = new_v.max(SimFixed::from_num(2));
                            sys.particles.push(new_particle);
                        }
                    }
                }
            }
        }

        // Phase 5: spawn accumulator (§3.3)
        sys.spawn_timer += pst.slowdown;
        if pst.spawn_cutoff < sys.spawn_timer {
            sys.done_spawning = true;
        }
    }

    fn symmetric_offset(r: i32, rng: &mut crate::sim::rng::SimRng) -> i32 {
        if r <= 0 { return 0; }
        let raw = rng.next_range_u32(r as u32) as i32;
        // Per §9.6: if raw < 1 then negate base; this is a sign-randomization
        // that biases the offset to be +/- around r.
        if raw < 1 { raw - r } else { raw + r }
    }

    fn spawn_smoke_child(
        sys: &mut ParticleSystem,
        parent: &Particle,
        next_type: crate::rules::particle_type::ParticleTypeId,
        delta: IVec3,
        sim: &mut Simulation,
    ) {
        let coords = parent.coords + delta;
        let pt = sim.ruleset.particle_type(next_type);
        if sys.particles.len() < /* particle_cap */ {
            let mut child = make_particle(next_type, coords, pt);
            child.velocity = parent.velocity;
            // Translucency: parent + (1-in-6 random 0x19 fade) per §9.6
            let extra = if sim.rng.next_range_u32(6) != 0 { 0x19 } else { 0 };
            child.translucency = parent.translucency.saturating_add(extra);
            sys.particles.push(child);
        }
    }
}
```

(The above is condensed for plan readability. Real code expands the type dependencies and shares `make_particle` with `spawn.rs`.)

**Step 2: Smoke particle AI per §3.8**
```rust
// In particle_ai.rs
pub(super) mod particle_ai_smoke {
    use super::*;

    pub fn tick(p: &mut Particle, pt: &ParticleType, rng: &mut SimRng) {
        // Every other frame, 25% chance to drift random X or Y, clamped [-5, +5].
        // Decel per Deacc each frame while velocity > 0.
        // Animation state advance via state_ai_advance divisor.
        // EndStateAI + DeleteOnStateLimit -> mark_for_deletion.
        // ...full implementation per §3.8 smoke...
    }
}
```

**Step 3: Move_Smoke per §10.2.3**
```rust
// In movement.rs
pub fn move_smoke(p: &mut Particle, pt: &ParticleType, wind_dir: u8, tick: u64, rules: &Ruleset) {
    // Apply smoke wind drift via SMOKE_WIND_{DX,DY} indexed by wind_dir.
    // Multiply by WindEffect (smoke scales by WindEffect, gas does not — §10.14.4).
    // Apply X/Y/Z drift fields.
    // Bridge collision: if smoke would pass through bridge from below, mark deleted.
    // ...full implementation per §10.2.3...
}
```

**Step 4: Tests — parity-critical**
```rust
#[test]
fn smoke_next_particle_spawns_two_children_at_symmetric_offsets() {
    // Regression test for §9.6 finding — must spawn TWO children per dying particle.
}

#[test]
fn smoke_spawn_cap_enforced() {
    // Spawn aggressively, assert sys.particles.len() <= particle_cap.
}

#[test]
fn smoke_done_spawning_when_accumulator_exceeds_cutoff() {
    // Per §3.3 — once spawn_timer > spawn_cutoff, done_spawning = true.
}

#[test]
fn smoke_wind_drift_uses_smoke_table_at_se() {
    // SMOKE_WIND_DX[3] = 2 (NOT 1 like gas).
}
```

**Step 5: Verify**
`cargo test -p <crate> particles::smoke`

**Step 6: Commit**
`git commit -m "particles: Smoke BehavesLike (system AI + particle AI + Move_Smoke + two-child NextParticle)"`

---

### Task C3: Implement Gas system AI + gas particle AI + Move_Gas

**Why:** Gas is the second SHP-rendered Tier-2 variant. Includes damage application (parity-critical: no friend/foe filter, hits all cell occupants).

**Files:**
- Modify: `src/sim/particles/system_ai.rs` (gas system AI)
- Modify: `src/sim/particles/particle_ai.rs` (gas particle AI)
- Modify: `src/sim/particles/movement.rs` (Move_Gas)

**Pattern:** Per §3.4 (system), §8.4 / §10.12 (particle AI + damage), §10.2.2 (Move_Gas) of report.

**Step 1: Gas system AI per §3.4**
- First pass: tick all particles.
- Second pass (reverse): handle NextParticle chaining (single child, NOT double like smoke). Copy velocity, drift_x/y/z to child.
- Per §3.4 the gas chain is `GasCloudM1 → GasCloud1 → GasCloudD1` with NextParticleOffset.

**Step 2: Gas particle AI per §8.4 / §10.12**
- Damage countdown decrement. When zero AND `pt.damage != 0`, reset from `MaxDC`, iterate cell occupants (Task C6), apply damage.
- Wind drift: 1-in-8 chance per even frame, axis randomization, clamped [-2, +2].
- Gravity: Z velocity = -2.0 - RulesClass.Gravity.
- Bridge collision per §8.4 (cell flag at +0x140 & 0x100). Skipping detailed bridge logic for Tier 2 — treat as flat-ground for first pass; bridge support can land as a follow-up if needed.
- Animation state advance per §3.8 gas.

**Step 3: Move_Gas per §10.2.2**
- Odd frames only.
- Wind drift: `if wind_effect > 0 && tick % (10/wind_effect) == 0` apply `GAS_WIND_{DX,DY}[wind_dir]`.
- Settles toward `ground + 5` with max drop of 2 per tick.
- Apply X/Y/Z drift fields from particle scratch.

**Step 4: Tests**
```rust
#[test]
fn gas_damage_hits_all_cell_occupants_no_friend_foe() {
    // Place 2 same-house units in a cell, spawn gas, both take damage.
}

#[test]
fn gas_uses_gas_wind_table_se_dx_is_one() {
    // GAS_WIND_DX[3] == 1 (smoke is 2).
}

#[test]
fn gas_next_particle_spawns_one_child_with_velocity_copy() {
    // §3.4 — single-child chain, velocity preserved.
}

#[test]
fn gas_damage_countdown_resets_from_max_dc() {
    // Tick exactly MaxDC frames, assert damage applied once, counter reset.
}
```

**Step 5: Verify**
`cargo test -p <crate> particles::gas`

**Step 6: Commit**
`git commit -m "particles: Gas BehavesLike (system AI + particle AI + Move_Gas + damage)"`

---

### Task C4: Implement Fire system AI + fire particle AI + fire movement

**Why:** Third Tier 2 variant. Fire is more elaborate — orbital tracking of attached object, `SpawnParticleWithInsert` for vector ordering variety, FinalDamageState gate, ground collision.

**Files:**
- Modify: `src/sim/particles/system_ai.rs` (fire system AI)
- Modify: `src/sim/particles/particle_ai.rs` (fire particle AI)
- Modify: `src/sim/particles/movement.rs` (fire inline movement in dispatch)

**Pattern:** Per §3.6 (system), §3.8 fire / §10.13 (particle AI), §10.2.1 (Fire movement is inline in dispatch — different from gas/smoke which have helpers).

**Step 1: Fire system AI per §3.6**
- Tick + Move all particles.
- Track attached object via filter (verify alive).
- If attached object dead: mark_for_deletion and return.
- Orbital motion calculation using RateTimer + cos/sin lookup tables (use existing math helpers in `src/util/`).
- Spawn via `SpawnParticleWithInsert` from B4. Spawn frequency: every `pst.spawn_frames` ticks OR every 3 ticks if target moved.

**Step 2: Fire particle AI per §3.8 + §10.13**
- If velocity ≤ 0: mark_for_deletion. Return.
- Apply random jitter to direction (±5%).
- Update prev-position delta (used by movement dispatch).
- Animate state. When reaching `translucent_50_state` set translucency = 0x19. When `translucent_25_state` set = 0x32.
- Decel by `pt.deacc`.
- Damage countdown — gated by `animation_state <= pt.final_damage_state` (§10.13.3, parity-critical: fire stops dealing damage at FinalDamageState).
- Distance scaling: damage reduced by `distance/10` from particle to target (§10.13.3).

**Step 3: Fire movement (inline in dispatch per §10.2.1)**
- Add prev-delta to position.
- Ground collision: if `old_ground < new_ground` (terrain rises), mark hit_ground + marked_for_deletion (§10.13.2 — parity-critical fire-stream death on cliffs).
- No bridge check for fire (per §10.13.2).

**Step 4: Tests**
```rust
#[test]
fn fire_dies_on_rising_terrain() {
    // Spawn fire near a cliff cell, tick, assert marked_for_deletion.
}

#[test]
fn fire_spawn_with_insert_does_not_produce_strict_creation_order() {
    // After 10 fire-particle spawns, vector order is not monotonic.
}

#[test]
fn fire_final_damage_state_gates_damage() {
    // Tick fire particle past FinalDamageState (default 14), assert
    // no damage applied even though counter reached zero.
}

#[test]
fn fire_translucency_changes_at_state_thresholds() {
    // Tick to translucent_50_state -> translucency = 0x19;
    // Continue to translucent_25_state -> translucency = 0x32.
}
```

**Step 5: Verify**
`cargo test -p <crate> particles::fire`

**Step 6: Commit**
`git commit -m "particles: Fire BehavesLike (system AI + particle AI + inline movement + FinalDamageState gate)"`

---

### Task C5: Particle AI dispatch entry point + lifetime/cleanup loop

**Why:** Wires C2-C4 per-variant ticks behind a single dispatch fn called from the system AIs. Final lifecycle convergence (lifetime decrement at the end of every particle tick).

**Files:**
- Modify: `src/sim/particles/particle_ai.rs`

**Pattern:** Per §3.2 of report.

**Step 1: Dispatch fn**
```rust
pub fn tick_particle(p: &mut Particle, sim: &mut Simulation) {
    let pt = sim.ruleset.particle_type(p.type_id);
    match pt.behaves_like {
        ParticleBehavesLike::Gas   => particle_ai_gas::tick(p, pt, &mut sim.rng /* ... */),
        ParticleBehavesLike::Smoke => particle_ai_smoke::tick(p, pt, &mut sim.rng),
        ParticleBehavesLike::Fire  => particle_ai_fire::tick(p, pt, &mut sim.rng /* ... */),
        ParticleBehavesLike::Spark | ParticleBehavesLike::Railgun => {
            // Tier 3 — no-op. Particle still ages out via the lifetime decrement below.
        }
    }
    p.lifetime_remaining = p.lifetime_remaining.saturating_sub(1);
    if p.lifetime_remaining == 0 {
        p.marked_for_deletion = true;
    }
}
```

**Step 2: Update C2-C4 system AIs to call this fn instead of per-variant directly.**

**Step 3: Tests**
```rust
#[test]
fn particle_ages_out_after_lifetime_remaining_hits_zero() {
    // Spawn a particle with lifetime_remaining = 3.
    // Tick 3 times. Assert marked_for_deletion = true on tick 3.
}
```

**Step 4: Verify**
`cargo test -p <crate> particles`

**Step 5: Commit**
`git commit -m "particles: dispatch + lifetime decrement at end of every particle tick"`

---

### Task C6: Gas/fire damage application — cell-occupant iteration

**Why:** Damage is the gameplay-affecting part. Must iterate cell occupants deterministically. Per §11.10 deferred question, verify iteration order before relying on it.

**Files:**
- Modify: `src/sim/particles/damage.rs`

**Pattern:** Existing damage application in [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs).

**Step 1: Verify cell-occupant iteration order**
```
grep -rn "ObjectsInCell\|cell_occupants\|cell.objects" src/sim/
```
If iteration is by `EntityStore` insertion order (BTreeMap u64 sort) — deterministic ✓. If by some other criterion — verify or fix in a separate pre-task. Document the result in this task's commit.

**Step 2: `apply_gas_damage`**
```rust
//! Gas particle damage — hits ALL objects in cell, no friend/foe filter (§10.12).

use super::Particle;
use crate::sim::Simulation;

pub fn apply_gas_damage(particle: &mut Particle, sim: &mut Simulation) {
    let pt = sim.ruleset.particle_type(particle.type_id).clone();
    if pt.damage == 0 {
        return;
    }
    particle.damage_counter -= 1;
    if particle.damage_counter > 0 {
        return;
    }
    particle.damage_counter = pt.max_dc as i16;

    // Cell occupant iteration — relies on Step 1's verified deterministic order.
    let cell = sim.cell_at_coord(particle.coords);
    let occupants: Vec<EntityId> = sim.cell_occupants(cell).collect();
    for occ_id in occupants {
        let Some(occ) = sim.entities.get(occ_id) else { continue; };
        if !occ.is_alive() || occ.health() <= 0 { continue; }

        // Owner house: passed to receive_damage for kill credit.
        let owner_house = particle.owner_house_via_psc(sim);
        let warhead = pt.warhead;
        sim.apply_damage(occ_id, pt.damage, warhead, owner_house);
    }
}
```
The `apply_damage` helper exists in src/sim/combat — reuse it.

**Step 3: `apply_fire_damage`** — like above, but:
- Gate on `particle.animation_state <= pt.final_damage_state` (§10.13.3).
- Distance scaling: damage `pt.damage` reduced by `distance / 10`.
- Bridge layer check (§10.13.3) — Tier 2 may defer this; flag.
- Exclude particle's own attached object (§10.13.3).

**Step 4: Tests**
```rust
#[test]
fn gas_damage_no_friend_foe_filter() {
    // Setup: 2 units same house in cell.
    // Spawn gas, advance MaxDC ticks, assert both took damage.
}

#[test]
fn fire_damage_excludes_attached_object() {
    // Fire stream attached to firing unit; that unit doesn't take damage from its own stream.
}

#[test]
fn fire_damage_distance_scales() {
    // Two targets at different distances; closer takes more damage.
}
```

**Step 5: Verify**
`cargo test -p <crate> particles::damage`

**Step 6: Commit**
`git commit -m "particles: gas/fire damage application with deterministic cell-occupant iteration"`

---

### Task D1: Hook `AttachedParticleSystem` in combat fire path

**Why:** First consumer hookup. Every weapon with `AttachedParticleSystem=` spawns a PSC at fire time.

**Files:**
- Modify: `src/sim/combat/mod.rs`

**Step 1: Locate the fire-shot path**
```
grep -n "fire_at\|fire_shot\|on_weapon_fire" src/sim/combat/mod.rs
```

**Step 2: Add the spawn at fire time**
```rust
// At the point where a shot is fired, after damage is applied:
if let Some(psc_type) = weapon_type.attached_particle_system {
    let _ = sim.spawn_particle_system(
        psc_type,
        attacker_coords,                                 // bullet position
        Some(attacker_id),                                // attached
        Some(attacker_id),                                // owner
        target_coords,                                    // (railgun unused at Tier 2)
        attacker_house,
    );
}
```

**Step 3: Hook `UseFireParticles` per §11.8.D / D6 logic**
```rust
if weapon_type.use_fire_particles {
    if let Some(default_fire) = sim.ruleset.combat_damage.default_fire_stream_system {
        let _ = sim.spawn_particle_system(
            default_fire, attacker_coords, Some(attacker_id), Some(attacker_id),
            target_coords, attacker_house,
        );
    }
}
```

**Step 4: Hook `UseSparkParticles` Tier-3-skip**
```rust
if weapon_type.use_spark_particles {
    if let Some(default_spark) = sim.ruleset.combat_damage.default_spark_system {
        // Tier 2: spawn_particle_system returns None for Spark. Warn-once.
        let _ = sim.spawn_particle_system(default_spark, /* ... */);
    }
}
```

**Note on `IsRailgun` (also Tier 3 — deferred):** [src/rules/weapon_type.rs:239](../../src/rules/weapon_type.rs) parses `is_railgun: bool` from `IsRailgun=yes`. Per §10.6.3 of the report, this is a third weapon-side spawn path with a different signature (computed endpoint via `FUN_0070C690`, target object passed as NULL, attached to the bullet). It uses the same `weapon.attached_particle_system` field as UseFireParticles/UseSparkParticles but with the railgun call shape. **At Tier 2 we do NOT implement the IsRailgun spawn path** — railgun PSCs warn-skip via `spawn_particle_system`, and the render-side pixel-write plumbing they need doesn't exist yet. Document this explicitly: when IsRailgun lands at Tier 3, the spawn site goes here, alongside the other three weapon-flag branches.

**Step 5: Tests**
- Integration test: a unit with `AttachedParticleSystem=GasCloudSys` fires once, assert `sim.particle_systems.len() == 1`.
- A unit with `UseFireParticles=yes` fires once, assert a fire-type PSC spawned.

**Step 6: Verify**
`cargo test -p <crate> combat`

**Step 7: Commit**
`git commit -m "particles: spawn AttachedParticleSystem / UseFireParticles in combat fire path"`

---

### Task D2: Hook `DamageParticleSystems` on building damage threshold

**Why:** Replaces the placeholder DamageFireOverlays trigger. Smoke spawns when health drops below ConditionYellow (per §8.6.2 — ReceiveDamage filters BehavesLike==Smoke).

**Files:**
- Modify: `src/sim/combat/mod.rs` (damage application)

**Step 1: Locate the damage application path**
```
grep -n "apply_damage\|receive_damage\|ApplyDamage" src/sim/combat/
```

**Step 2: Add health-threshold transition spawn**
```rust
// After damage is applied to a building:
let prev_ratio = prev_health as f64 / max_health as f64;
let new_ratio = new_health as f64 / max_health as f64;
let condition_yellow = sim.ruleset.general.condition_yellow as f64;

if new_ratio <= condition_yellow && prev_ratio > condition_yellow {
    // Crossed below yellow threshold this tick — spawn smoke from filtered list.
    let smoke_systems: Vec<ParticleSystemTypeId> = building_type
        .damage_particle_systems
        .iter()
        .filter(|id| {
            sim.ruleset.particle_system_type(**id).behaves_like
                == ParticleSystemBehavesLike::Smoke
        })
        .copied()
        .collect();
    if !smoke_systems.is_empty() {
        let pick = sim.rng.next_range_u32(smoke_systems.len() as u32) as usize;
        let chosen = smoke_systems[pick];
        let coord_offset = building_type.damage_smoke_offset;
        let coords = building.coords() + coord_offset;
        let _ = sim.spawn_particle_system(
            chosen, coords, Some(building.id), Some(building.id), IVec3::ZERO, building.house,
        );
    }
}
```

**Step 3: Tests**
- Integration: damage a building below ConditionYellow; assert at least one smoke PSC spawned, of BehavesLike==Smoke.
- Damage to ConditionRed: confirm no second spawn (only the threshold crossing fires).

**Step 4: Verify**
`cargo test -p <crate> combat::damage_particles`

**Step 5: Commit**
`git commit -m "particles: spawn DamageParticleSystems on ConditionYellow threshold crossing"`

---

### Task D3: Hook `RefinerySmokeParticleSystem` in refinery dump cycle

**Why:** Refinery dump produces 4 smoke plumes at fixed offsets per §11.8.C / §8.6.4.

**Files:**
- Modify: `src/sim/miner/miner_dock_sequence.rs`

**Step 1: Locate the dump-frame trigger**
```
grep -n "dump\|deposit" src/sim/miner/miner_dock_sequence.rs
```

**Step 2: At dump completion, fire 4 smoke spawns**
```rust
let psc_type = building_type.refinery_smoke_particle_system?;
let coords = building.coords();
for offset in &building_type.refinery_smoke_offsets {
    if *offset == IVec3::ZERO { continue; }      // sentinel skip
    let _ = sim.spawn_particle_system(
        psc_type, coords + *offset, Some(building.id), Some(building.id),
        IVec3::ZERO, building.house,
    );
}
```

**Step 3: Tests**
- Simulate a dump; assert 4 PSCs spawned (or fewer if sentinel offsets skip some).

**Step 4: Verify + Commit**

---

### Task D4: Hook `NaturalParticleSystem` on gap-generator state transition

**Why:** Per §8.6.4 / §11.5.B — gap generators spawn smoke on state 3→0 transition. NaturalParticleSystem is always null in retail YR but the code path is reachable; spawn must accept null gracefully.

**Files:**
- Modify: `src/sim/vision/mod.rs` (gap generator state machine)

**Step 1: Locate state machine + state-3→0 transition**
```
grep -n "gap_state\|GapState\|gap_fade" src/sim/vision/mod.rs
```

**Step 2: At the transition, spawn (if non-null)**
```rust
if old_state == 3 && new_state == 0 {
    if let Some(psc_type) = building_type.natural_particle_system {
        let offset = building_type.natural_particle_location;
        let coords = building.coords() + offset;
        let _ = sim.spawn_particle_system(
            psc_type, coords, Some(building.id), None, IVec3::ZERO, building.house,
        );
    }
}
```

**Step 3: Tests**
- State 3→0 with `natural_particle_system = None` → no spawn, no panic.
- State 3→0 with valid PSType → 1 spawn.

**Step 4: Verify + Commit**

---

### Task D5: Hook `BarrelParticle` on barrel destruction (area damage)

**Why:** Per §11.8.H — when a barrel overlay cell is destroyed by area damage, spawn the global `[General] BarrelParticle` PSC.

**Files:**
- Modify: `src/sim/combat/combat_aoe.rs`

**Step 1: Locate barrel destruction**
```
grep -n "barrel\|Barrel" src/sim/combat/combat_aoe.rs
```

**Step 2: At barrel-cell-destroyed, spawn**
```rust
if let Some(barrel_psc) = sim.ruleset.general.barrel_particle {
    let _ = sim.spawn_particle_system(
        barrel_psc, cell_coord, None, None, IVec3::ZERO, attacker_house,
    );
}
```

**Step 3: Tests**
- Damage a barrel cell; assert one PSC spawn.

**Step 4: Verify + Commit**

---

### Task E1: SHP draw collection for gas/smoke/fire particles

**Why:** Tier 2 render integration. Each particle emits a SpriteInstance.

**Files:**
- Create: `src/render/particles.rs`

**Step 1: Implement collector**
```rust
//! Particle render — emit SHP sprite instances for gas/smoke/fire particles.
//! Source: PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §7 (Draw_It).

use crate::render::sprite::SpriteInstance;
use crate::rules::Ruleset;
use crate::rules::particle_type::ParticleBehavesLike;
use crate::sim::particles::ParticleSystemStore;

pub fn collect_particle_draw_instances(
    particle_systems: &ParticleSystemStore,
    rules: &Ruleset,
    art: &crate::rules::art_data::ArtRegistry,
    out: &mut Vec<SpriteInstance>,
) {
    for (_id, sys) in particle_systems.iter() {
        for p in &sys.particles {
            let pt = rules.particle_type(p.type_id);
            match pt.behaves_like {
                ParticleBehavesLike::Gas
                | ParticleBehavesLike::Smoke
                | ParticleBehavesLike::Fire => {
                    let Some(image) = pt.image.as_deref() else {
                        // Skip with warn-once per feedback_silent_render_failures memory.
                        continue;
                    };
                    let Some(shp) = art.resolve_image(image) else { continue; };

                    let frame = compute_frame_index(p, pt);
                    let translucency_flags = translucency_byte_to_flags(p.translucency);
                    let depth_z = -15 - adjust_for_z(p.coords);   // §7

                    out.push(SpriteInstance {
                        shp,
                        frame,
                        coords: p.coords,
                        flags: 0x0E00 | translucency_flags,
                        z_offset: depth_z,
                        // ... layer 3 ...
                    });
                }
                ParticleBehavesLike::Spark | ParticleBehavesLike::Railgun => {
                    // Tier 3 — pixel render path not implemented yet.
                }
            }
        }
    }
}

fn translucency_byte_to_flags(b: u8) -> u32 {
    // §7 of report: 0x00 -> 0x2800 opaque, 0x19 -> 0x2802 50%,
    //                0x32 -> 0x2804 25%,  0x4A+ -> 0x2806 fade.
    match b {
        0 => 0x2800,
        0x19 => 0x2802,
        0x32 => 0x2804,
        b if b > 0x4A => 0x2806,
        _ => 0x2800,
    }
}

fn compute_frame_index(p: &Particle, pt: &ParticleType) -> u16 {
    // §10.14.4 — gas/smoke return animation_state directly; fire computes
    // facing-indexed frame.
    match pt.behaves_like {
        ParticleBehavesLike::Gas | ParticleBehavesLike::Smoke => p.animation_state as u16,
        ParticleBehavesLike::Fire => {
            // Fire: directional frame = facing_index * end_state_ai + animation_state.
            // For Tier 2 (no per-particle facing yet), use animation_state.
            p.animation_state as u16
        }
        _ => 0,
    }
}
```

**Step 2: Tests**
- Translucency byte mapping: 4 boundary values map to 4 flag values.
- Image-missing: warn-once + skip (no panic).

**Step 3: Verify**
`cargo test -p <crate> render::particles`

**Step 4: Commit**
`git commit -m "particles: SHP draw collection for gas/smoke/fire"`

---

### Task E2: Wire particles into `app_render/build_instances.rs`

**Why:** Connects render-side collector to the actual draw pipeline.

**Files:**
- Modify: `src/app_render/build_instances.rs` — locate the sprite-build entity iteration site (NOT `update_minimap` at line 277, which is minimap dot iteration). Grep for `sim.entities` and `SpriteInstance` together to find the right site.

**Step 1: Locate the sprite-build entity iteration**
```
grep -n "sim.entities" src/app_render/build_instances.rs
grep -n "SpriteInstance" src/app_render/build_instances.rs
```
The right site is the one that pushes into the sprite-instance vector(s) during world drawing, not the minimap dot loop.

**Step 2: Add the call**
```rust
// Alongside the existing entity iteration:
crate::render::particles::collect_particle_draw_instances(
    &sim.particle_systems,
    &sim.ruleset,
    &art,
    &mut sprite_instances,
);
```

**Step 3: Verify**
- `cargo build` — clean.
- Run the app: `cargo run`. Spawn a unit with a smoke weapon, observe smoke draws.

**Step 4: Commit**
`git commit -m "particles: wire collector into build_instances render pipeline"`

---

### Task E3: Visual parity check — `DamageFireOverlays` vs new smoke PSC side-by-side

**Why:** Before deleting the placeholder, confirm the new smoke PSC produces equivalent visible output for damaged buildings. Per CLAUDE.md "Verify the end-to-end result of every change".

**Files:** none modified — investigation/observation only.

**Step 1: In-game scenario**
- Spawn a building with low HP (below ConditionYellow).
- Compare: original gamemd.exe rendering of damage smoke vs current Rust engine rendering.
- Confirm: smoke spawns at the right offsets, fades at the right cadence, looks visually similar (within 99% parity bar).

**Step 2: If parity is off**
- Re-read §8.6.2 of report and the code path in Task D2.
- Adjust DamageSmokeOffset wiring, ParticleCap, or spawn timing as needed.
- Re-test.

**Step 3: If parity is matched**
- Proceed to Task E4.

**Step 4: Document the comparison**
Note in a commit message what was checked and what matched. No code commit unless adjustments were needed.

---

### Task E4: Retire `DamageFireOverlays` placeholder

**Why:** With smoke PSC parity confirmed in E3, the placeholder can go. CLAUDE.md "Avoid backwards-compatibility hacks like renaming unused _vars... If you are certain that something is unused, you can delete it completely."

**Files:**
- Modify: `src/sim/components.rs` — delete `DamageFireOverlays`, `DamageFireAnim` structs (lines 475-502).
- Modify: `src/sim/game_entity.rs:103` — delete `damage_fire_overlays: Option<DamageFireOverlays>` field.
- Modify: `src/app_building_anim.rs` — delete `tick_damage_fire_overlays` function and its references.
- Modify: `src/app_sim_tick.rs:164` — delete the call to `tick_damage_fire_overlays`.
- Modify: `src/app_instances/overlays.rs:109` — delete the `DamageFireAnim` rendering branch.

**Step 1: Audit usage**
```
grep -rn "DamageFireOverlay\|DamageFireAnim\|tick_damage_fire" src/
```
List all references.

**Step 2: Delete**
- Delete the structs from `src/sim/components.rs`.
- Delete the field from `GameEntity`.
- Delete the tick fn and call site.
- Delete the render branch.

**Step 3: Build + test**
```
cargo build --all-targets
cargo test
```
Expected: all green.

**Step 4: Visual regression check**
- Run the same scenario from E3.
- Confirm damage smoke still appears (now coming from PSC, not overlay).
- Confirm no console errors / warnings about missing DamageFireOverlays.

**Step 5: Commit**
`git commit -m "particles: retire DamageFireOverlays placeholder (replaced by Smoke PSC via DamageParticleSystems)"`

---

### Task E5: Final integration test — full skirmish smoke test

**Why:** Confirm the whole Tier 2 system works end-to-end in a representative scenario.

**Files:** none modified.

**Step 1: Test scenario**
- Build a small skirmish: 2 buildings (one refinery), 2 units (one with `UseFireParticles=yes` weapon), some barrels on the map, a gap-generator building if available.
- Run for 5 minutes.
- Observe:
  - Damage smoke on damaged buildings ✓
  - Refinery dump smoke ✓ (4 plumes per dump)
  - Fire-stream particles from the fire-weapon unit ✓
  - Barrel destruction smoke when explosions hit barrels ✓
- Take a snapshot of `sim.particle_systems.len()` periodically — should grow during action and shrink as PSCs complete.

**Step 2: Determinism check**
- Save the scenario seed.
- Run twice with the same seed and assert state hashes match every tick.

**Step 3: Profile**
- `cargo run --release` with a flame graph.
- Confirm `tick_particle_systems` is not in the top-10 hot functions.
- If it is, profile and address before merging.

**Step 4: Commit / merge**
- Push branch, open PR.
- Reference design doc and §11 of report.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-04-particle-system-rust-architecture-design.md](2026-05-04-particle-system-rust-architecture-design.md)
- **Investigation plan:** [docs/plans/2026-05-04-particle-system-gaps-investigation-plan.md](2026-05-04-particle-system-gaps-investigation-plan.md)
- **Research doc:** `docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §1–§11 (binary-verified)
- **Sibling docs referenced:**
  - `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` — TTC ReadINI base, used for Task A5
  - `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` — IPersistStream pattern (deferred — not in Tier 2)
  - `OBJECTCLASS_GHIDRA_REPORT.md` — vtable layout reference
- **Key gamemd.exe addresses:**
  - PSType ReadINI 0x006442D0, ctor 0x006440A0 (defaults)
  - PType ReadINI 0x00644F50, ctor 0x00644BE0
  - ColorList parser 0x00476B20, vtable 0x007E4E58
  - ColorStruct stride: 3 bytes packed (verified in 5 fns per §11.1)
  - System AI: Smoke 0x0062ED40, Gas 0x0062E6D0, Fire 0x0062F9A0
  - Particle AI: Smoke 0x0062C540, Gas 0x0062BD50, Fire 0x0062CB10
  - Move: Move_Smoke 0x0062D3F0, Move_Gas 0x0062D2A0, Move_Dispatch 0x0062D5E0
  - Wind tables: gas 0x00836664/0x00836684, smoke 0x008366A4/0x008366C4
  - BehavesLike string tables: PSType 0x00836EE0 (Smoke=0/Gas=1), PType 0x008370BC (Gas=0/Smoke=1)
  - DefaultSparkSystem read site: RulesClass::ReadCombatDamage 0x0066BBB0, slot RulesClass+0x1020
  - BarrelParticle read site: RulesClass::ReadGeneral 0x0066D530, slot RulesClass+0x74, [General] section
- **INI keys driving behavior:**
  - rulesmd.ini: `[ParticleSystems]` master list (13 PSType entries), `[Particles]` master list (22 PType entries), `[CombatDamage]` 9 Default*System keys, `[General] BarrelParticle=`, `[General] WindDirection=`, plus consumer keys on TechnoType / WeaponType
- **Repo files mirrored / extended:**
  - [src/sim/animation.rs](../../src/sim/animation.rs) — frame-timing pattern
  - [src/sim/entity_store.rs](../../src/sim/entity_store.rs) — BTreeMap deterministic store pattern
  - [src/rules/weapon_type.rs](../../src/rules/weapon_type.rs) — type-class INI parse pattern
  - [src/rules/art_data.rs](../../src/rules/art_data.rs) — Image=→SHP resolution
  - [src/sim/world/mod.rs `advance_tick`](../../src/sim/world/mod.rs) — 13-phase tick loop
  - [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) — state_hash inclusion pattern
- **Memories applied:**
  - `project_string_interning.md` — drives Q4 decision (interned IDs for new code)
  - `feedback_silent_render_failures.md` — warn-not-skip on missing SHP
  - `feedback_no_engine_refs_in_comments.md` — gamemd.exe addresses kept in this plan and the report, NOT in Rust code comments
  - `feedback_branches_and_prs.md` — work lands on a feature branch + PR, never directly to main
