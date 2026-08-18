# Particle System Rendering Pipeline — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Wire `Simulation.particle_systems` into the render pipeline so combat
smoke, refinery dump bursts, gas clouds, and fire trails become visible at
gamemd parity (Layer 3, anim.pal palette, translucency states), and add the
per-tick state-AI advance the renderer depends on.

**Architecture:** A single instance builder in `app_instances/particles.rs`
emits `SpriteInstance`s from `Simulation.particle_systems` per frame, dispatched
on `behaves_like` for frame-index calculation. SHPs register through the
existing `effect_type_ids` channel (anim.pal palette). A new draw Step 7.5
between cliff redraw and debug overlays draws the particle pass with the
existing passthrough pipeline. State-AI advance lands inside the existing
Tier-2 tick functions, gated by gamemd's `(num_loop_frames % 2 + 1) +
StateAIAdvance` denominator, with translucency-state transitions writing the
exact byte values the renderer reads.

**Design Doc:** [docs/plans/2026-05-07-particle-system-rendering-design.md](docs/plans/2026-05-07-particle-system-rendering-design.md)

---

## Grounding Summary

- **Docs:** `ra2-rust-game-docs/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` is the
  authoritative source — covers struct layouts, BehavesLike enum (gamemd has
  Smoke=0/Gas=1 at the SYSTEM level vs Gas=0/Smoke=1 at the PARTICLE level,
  already correct in repo), `Draw_It` rendering at FUN_0062cec0, state-AI
  advance formula at §3.6/§9.12.3, ColorList interpolation, all 13 particle
  systems + 22 particle types defined in stock YR (§10.5), and `Image=` SHP
  binding via `ObjectTypeClass::ReadINI` with anim.pal palette (§11.4).
- **Ghidra verification:** addresses captured in design doc ledger and Sources
  section. `ParticleClass::Draw_It` at 0x0062cec0, `GetImageFrame` at
  0x0062d830, `GetLayer = 3` at 0x0062d770, all V7-verified.
- **TS-legacy filter:** Fog-of-war check (SpecialFlags & 0x1000) is TS-legacy,
  FogOfWar=false in YR — explicitly NOT implemented per CLAUDE.md.
  Spark/Railgun pixel rendering is Tier 3 — already filtered at
  [spawn.rs:42](src/sim/particles/spawn.rs#L42).
- **Repo pattern:** Mirrors `build_world_effect_instances` /
  `build_damage_fire_instances` / `build_parachute_instances` in
  [app_instances/overlays.rs](src/app_instances/overlays.rs); SHP atlas
  registration mirrors the post-step-1d `effect_names` block at
  [sprite_atlas.rs:653-723](src/render/sprite_atlas.rs#L653); frame-count
  plumbing reuses `Simulation::effect_frame_counts` ([app_init.rs:466-473](src/app_init.rs#L466))
  the same way chrono-teleport ([miner_system.rs:705-709](src/sim/miner/miner_system.rs#L705))
  and superweapon-invoke ([force_shield.rs:111-112](src/sim/superweapon/force_shield.rs#L111))
  do.
- **INI keys:** Driven by `[Particles]` (HoldsWhat, Image, MaxEC, MaxDC,
  Damage, EndStateAI, StartStateAI, StateAIAdvance, Translucent25State,
  Translucent50State, DeleteOnStateLimit, Velocity, Deacc, Translucency,
  Radius) and `[ParticleSystems]` (BehavesLike, HoldsWhat, Spawns,
  SpawnFrames, ParticleCap, SpawnRadius, Lifetime). All already parsed in
  [particle_type.rs](src/rules/particle_type.rs) /
  [particle_system_type.rs](src/rules/particle_system_type.rs) — no new INI
  parsing in this plan.
- **Unknown after grounding:** Fire facing-band band selection at Tier 2 —
  particle systems set `facing: 0x1D` at construction (per
  [spawn.rs:61](src/sim/particles/spawn.rs#L61)); divided by 0x40 = 0
  band, so every Tier-2 fire renders as facing band 0. Adequate parity
  for stock YR (FireStreamSys → FireStream is the only fire system in
  the table); flagged as deferred follow-up.

## Key Technical Decisions

- **State-AI advance lives in `sim/particles/system_ai.rs`** (one shared
  `advance_state` helper, called from each tick function) — **Confidence:** high
  - **Source:** doc §3.6 + §9.12.3, formula verified against gamemd
    FUN_0062f9a0 / FUN_0062ed40
- **Frame-count resolution via existing `Simulation::effect_frame_counts`** —
  **Confidence:** high
  - **Source:** repo pattern at [app_init.rs:466-473](src/app_init.rs#L466),
    consumed by miner_system.rs and superweapon code
- **Particle pass at Step 7.5 (after cliff redraw, before debug)** with
  passthrough pipeline (no depth interaction) — **Confidence:** high
  - **Source:** doc §5.3 (`GetLayer = 3`); user-locked in Q2 of brainstorm
- **Single instance builder dispatching on `behaves_like`** — **Confidence:** high
  - **Source:** user-locked in Q3 of brainstorm; ~80% shared code per design
- **Atlas registration via `effect_type_ids` (anim.pal channel)** —
  **Confidence:** high
  - **Source:** doc §11.4 confirms `Image=` SHPs go through ObjectTypeClass
    path → anim.pal
- **Tier-3 dispatch arm uses `OnceLock<HashSet>` for once-per-type log** —
  **Confidence:** medium
  - **Source:** repo doesn't have a unified once-per-type log helper; this
    introduces a small local one. Could alternatively use the `log` crate's
    target/key filtering, but a local set is simpler.
- **Lepton-to-screen helper lives in `map/terrain.rs` next to `iso_to_screen`** —
  **Confidence:** high
  - **Source:** terrain.rs already owns iso math; matches "low-level helpers
    near similar helpers" pattern
- **`state_advance_counter: u8` is wrapping**, denom always ≤ 8 in practice —
  **Confidence:** high
  - **Source:** denom = `(num_loop_frames % 2 + 1) + StateAIAdvance`; max
    `StateAIAdvance` in stock YR is 6 (FireStream); max denom = 8

## Open Questions

### Resolved During Planning

- **Q: Where should particle frame counts come from in sim?**
  - Resolved: reuse existing `Simulation::effect_frame_counts` map. Atlas
    loader populates `active_anim_frame_counts` for the registered SHPs;
    [app_init.rs:469-472](src/app_init.rs#L469) already copies these into
    `sim.effect_frame_counts`. No new map needed. Lookup by uppercase image
    name (atlas keys are uppercased per
    [sprite_atlas.rs:717](src/render/sprite_atlas.rs#L717)).

- **Q: Does `effect_type_ids` need to be case-normalized?**
  - Resolved: `effect_names` preserves the case from rules.ini, but
    `effect_type_ids.insert(name.clone())` and the atlas key lookup both
    use the same case-preserved name, so as long as we push `pt.image` as-is
    (e.g., "WCCLOUD1" or "gaslrgmk"), the round-trip works. Frame count
    map IS uppercased — must `.to_ascii_uppercase()` for `effect_frame_counts`
    lookup.

- **Q: Does `RuleSet` need a particle iteration helper?**
  - Resolved: yes — add `pub fn particle_types_iter(&self) -> impl Iterator<Item = &ParticleType>`.
    Used once at atlas-load to enumerate all `Image=` strings. Tiny addition.

### Deferred to Implementation

- **Q: Exact depth bucketing inside the particle pass.** The pass uses the
  passthrough pipeline (no GPU depth read/write), so the `SpriteInstance.depth`
  field is only used for CPU-side sort order. Use Y descending so closer
  particles draw last (over farther ones). Concrete encoding chosen at impl
  time to match the convention used for sort comparators (depth descending).

- **Q: Z-adjust as screen-Y nudge: −15 px for all particles, or scale with
  altitude?** The design (Ledger L25) treats it as a fixed −15 px lift.
  gamemd's `-15 - AdjustForZ()` adds the altitude lift on top. For Tier 2
  (no airborne particles), AdjustForZ ≈ 0, so −15 is correct. If airborne
  fire trails ever appear (post-Tier-2), revisit.

- **Q: Should the `OnceLock<HashSet>` for the Tier-3 warn-log be reset per
  map?** Probably yes — a fresh skirmish should log again — but reset
  hooks aren't urgent. Leaves the static behavior for the first impl;
  re-evaluate if log spam becomes a problem.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/particles/mod.rs` | Add `state_advance_counter: u8` field to `Particle`; update tests |
| Modify | `src/sim/particles/spawn.rs` | Initialize `state_advance_counter = 0` in spawn paths |
| Modify | `src/sim/particles/smoke.rs` | Init counter in `make_particle`; call `advance_state` in `tick_particle` |
| Modify | `src/sim/particles/gas.rs` | Init counter in spawn helper; call `advance_state` in `tick_particle` |
| Modify | `src/sim/particles/fire.rs` | Init counter in spawn helper; call `advance_state` in `tick_particle` |
| Modify | `src/sim/particles/system_ai.rs` | Add `advance_state` helper + denom resolver; thread `effect_frame_counts` into tick |
| Modify | `src/sim/world/world_hash.rs` | Hash `state_advance_counter` field per-particle |
| Modify | `src/rules/ruleset.rs` | Add `particle_types_iter()` accessor |
| Modify | `src/map/terrain.rs` | Add `lepton_to_screen(IVec3) -> (f32, f32)` helper |
| Modify | `src/render/sprite_atlas.rs` | New step 1f: register `ParticleType.image` SHPs in `effect_names` |
| Create | `src/app_instances/particles.rs` | `build_particle_instances` — emit SpriteInstances from particle store |
| Modify | `src/app_instances/mod.rs` | Add `mod particles; pub(crate) use particles::*;` |
| Modify | `src/app_render/build_instances.rs` | Add `particle_paged` to `WorldInstances`; call builder; sort |
| Modify | `src/app_render/mod.rs` | Upload `particle_p0`..`particle_p3` pool keys |
| Modify | `src/app_render/draw_passes.rs` | Add `particle_paged` to `DrawPassData`; new Step 7.5 draws |
| Modify | `src/app_building_anim.rs` | Spawn-coord fix: `rx*256` → `rx*256 + 128` (cell center) |

## Interface Changes

- **`Particle::state_advance_counter`** — new field. Read by
  `system_ai::advance_state`; hashed in `world_hash`. Initial value 0 from
  every spawn path.
- **`system_ai::tick_particle_systems`** signature is unchanged (still takes
  `&mut Simulation, &RuleSet`); the new dependency on `effect_frame_counts`
  is satisfied through `&sim.effect_frame_counts` since `&mut sim` already
  carries it.
- **`system_ai::advance_state`** — new `pub(super) fn`. Called from each
  Tier-2 tick function.
- **`RuleSet::particle_types_iter`** — new public method. Used once at
  atlas-load.
- **`terrain::lepton_to_screen`** — new public function.
- **`WorldInstances::particle_paged`** — new field on phase struct.
- **`DrawPassData::particle_paged`** — new field on draw-pass struct.
- **`SpriteAtlas`** unchanged (extension is in atlas-build code, not the
  type's public API).

## Sim Checklist

- [x] All math uses `fixed`-point or integer types — no f32/f64 in game logic.
      State-AI advance is integer math (u8 counter mod u8 denom). Lifetime is
      already i16. No floats in the new sim code.
- [x] New state included in deterministic state hash. `state_advance_counter`
      added to `hash_particle_systems` per-particle loop.
- [x] No dependencies on render/ui/sidebar/audio/net. `system_ai::advance_state`
      reads only `&Particle`, `&ParticleType`, `&HashMap<InternedId, u16>`.
      All sim-internal types.
- [x] Tick ordering preserved. `advance_state` runs inside existing
      `tick_particle_systems` (Phase 5.5). State write happens **before**
      lifetime decrement, matching gamemd ordering.
- [x] BTreeMap iteration order preserved. `ParticleSystemStore` is BTreeMap;
      `tick_particle_systems` already uses `.ids()` snapshot for ordered
      traversal.

## Risk Areas

- **Determinism with atlas state.** `effect_frame_counts` is populated from
  the atlas at app init. If two clients have different SHP files (modding
  desync), they will compute different `denom` → divergent
  `animation_state` → state-hash mismatch. This is a general replay risk,
  not a new one — every existing consumer of `effect_frame_counts` (chrono
  teleport, superweapon invoke anims) has the same exposure. No new
  mitigation in this plan.
- **Tier-3 system slipping past spawn filter.** Spawn-side filter at
  [spawn.rs:42](src/sim/particles/spawn.rs#L42) is the primary guard. The
  render-side defensive arm protects against future bugs (snapshot loaded
  from a different build, hand-crafted test data). Once-per-type log
  prevents spam; no functional impact if the arm fires.
- **Atlas miss on map with mod-defined particle Image=.** Atlas-load logs
  "WorldEffect SHP X: N frames loaded" for successes;
  [sprite_atlas.rs:719](src/render/sprite_atlas.rs#L719) doesn't currently
  warn on miss for `effect_names`. Particle Image= names that fail to load
  produce silent skip per particle at render time — matching gamemd. If
  parity demands a startup warning when a referenced SHP is missing, that's
  a separate improvement (low priority — stock YR all SHPs exist).
- **Layer 3 vs cliff redraw.** Particle pass renders AFTER cliff redraw,
  so a particle drawn near a cliff face renders ON TOP of the cliff. Per
  gamemd Layer 3, that's correct; if it visually feels wrong (smoke
  occluding a cliff face the player should still see), revisit at the
  visual end-to-end pass.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | State-AI denominator: `(image_frame_count % 2 + 1) + StateAIAdvance` | Wrong denom → smoke advances at wrong rate; visible every match where any system damages a building or refinery dumps ore | doc §3.6 + §9.12.3 (FUN_0062f9a0); unit test denom for both odd-frame and even-frame SHP |
| Task 2 | EndStateAI behavior: DeleteOnStateLimit → mark for deletion; otherwise reset state to 0 | Wrong → particles either leak forever or never loop. Visible every match | doc §3.8 Smoke; unit test both branches |
| Task 2 | Translucent50State → translucency = 0x19; Translucent25State → 0x32 | Wrong byte values silently break the renderer's alpha mapping; visible on every dissipating smoke plume (every damaged building) | doc §3.8 Fire / §9.7 |
| Task 4 | `lepton_to_screen` rounding (`div_euclid` / `rem_euclid` for negatives) | Particles drift to negative leptons near map edge; rounding-toward-zero puts them on wrong cell, drawn one cell over | numerical correctness; unit tests with negative coords |
| Task 5 | Particle SHPs use anim.pal palette, not unit.pal | Wrong palette → visibly wrong colors (gas clouds appear green-tinted unit-color instead of poison-yellow) | doc §11.4; verify by checking `effect_type_ids` insertion |
| Task 6 | Smoke/Gas frame index = `animation_state` directly; Fire frame index = `facing_band * EndStateAI + animation_state` | Wrong frame math → wrong SHP frame drawn (smoke shows fire-colored gradient or vice versa) | doc §9.12.3 (FUN_0062d830) |
| Task 6 | Translucency byte → alpha mapping: 0x00→1.0, 0x19→0.5, 0x32→0.25, ≥0x4A→0.16 | Wrong alpha → smoke either looks solid (no fade) or transparent from the start. Visible every damaged building | doc §8.7 + §9.7 |
| Task 6 | `house_color = HouseColorIndex(0)`, `tint = [1.0, 1.0, 1.0]` | Otherwise smoke would tint to owner color instead of staying neutral grey. Visible every match | gamemd CC_Draw_Shape `remap = 0` |
| Task 6 | `facing = 0` in atlas key (single-direction SHPs) | Otherwise atlas miss; particles silently invisible | particle SHPs have no `Facings=` |
| Task 6 | Y-sort within particle pass, depth descending (back-to-front) | Otherwise translucent particles stack incorrectly; visible on dense smoke clouds | standard alpha-blending requirement |
| Task 6 | −15 px screen-Y lift on particle position (gamemd's `-15 - AdjustForZ()`) | Smoke origin should sit just above the spawn cell, not buried in the ground. Visible every smoke plume | doc §9.7 |
| Task 8 | Particle pass at Step 7.5 — after cliff redraw, before debug | Layer 3 = above ALL ground objects + cliffs. Wrong placement → smoke either rendered behind buildings (Q2c rejected) or above UI (insertion past Step 8) | doc §5.3 (`GetLayer = 3`); locked Q2-(b) |
| Task 9 | Refinery smoke origin coord: `rx*256 + 128`, `ry*256 + 128` (cell center) | Currently NW corner; smoke billows from corner instead of cell center. Visible every refinery dump | doc §11.8.C, harvester dock trace |

---

## Tasks

### Task 1: Add `state_advance_counter` field + spawn init

**Why:** Foundation for the state-AI advance. The new field must exist before
the advance helper can read or write it. Initialized from every spawn path so
no Particle ever has uninitialized state.

**Files:**
- Modify: `src/sim/particles/mod.rs:53-80` (Particle struct)
- Modify: `src/sim/particles/spawn.rs:100-122` (spawn_particle helper)
- Modify: `src/sim/particles/smoke.rs:177-199` (make_particle helper)
- Modify: `src/sim/particles/gas.rs` (gas spawn helper)
- Modify: `src/sim/particles/fire.rs:75-96` (fire spawn helper)
- Modify: `src/sim/particles/mod.rs:160-179` (test helper `fake_system` if it constructs Particles — verify)

**Pattern:** New field on existing struct, initialized to 0 in every spawn
path. Mirrors how `damage_counter`, `lifetime_remaining`, and other
per-spawn-init fields are handled today.

**Step 1: Add field to Particle struct**
In `src/sim/particles/mod.rs` immediately after `prev_delta`:
```rust
    /// Per-particle sub-tick accumulator for the state-AI advance.
    /// Increments every tick; when it hits the per-type denominator
    /// `(image_frame_count % 2 + 1) + StateAIAdvance`, animation_state
    /// bumps by 1. Wraps at 256 (denom is always small in practice).
    pub state_advance_counter: u8,
```

**Step 2: Initialize in `spawn::spawn_particle`**
Add `state_advance_counter: 0,` to the `Particle` literal in
`src/sim/particles/spawn.rs` around line 117 (after `prev_delta`).

**Step 3: Initialize in `smoke::make_particle`**
Add `state_advance_counter: 0,` to the `Particle` literal in
`src/sim/particles/smoke.rs:197` (after `prev_delta`).

**Step 4: Initialize in `gas` spawn helper**
Open `src/sim/particles/gas.rs`, find the spawn helper around line 207
(`animation_state: pt.start_state_ai,`). Add `state_advance_counter: 0,`
to the same Particle literal.

**Step 5: Initialize in `fire` spawn helper**
Open `src/sim/particles/fire.rs:75-96` (or wherever the Particle literal lives
near `animation_state: pt.start_state_ai,`). Add `state_advance_counter: 0,`.

**Step 6: Verify**
```
cargo build
cargo test -p ra2_engine particles
```
Expected: clean build, all existing particle tests still pass.

**Step 7: Commit**
Message: `particles: add state_advance_counter sub-tick field on Particle`

---

### Task 2: Implement `advance_state` helper + unit tests

**Why:** Pure logic, fully testable in isolation, before any wiring. Owns
the gamemd state-AI formula and translucency-state byte writes. Establishes
parity-critical behavior that downstream tasks depend on.

**Files:**
- Modify: `src/sim/particles/system_ai.rs:1-50` (top of file, before
  `tick_particle_systems`)

**Pattern:** Pure function on `&mut Particle, &ParticleType, u16`. Mirrors
the way `move_smoke_with_wind` in `smoke.rs` is a pure helper with explicit
parameters and dedicated unit tests.

**Step 1: Add the helper at the top of `system_ai.rs`**
Insert after the imports, before `pub fn tick_particle_systems`:
```rust
/// Advance one Tier-2 particle's animation-state machine by one tick.
///
/// Implements gamemd's per-particle AI step (FUN_0062f9a0 / FUN_0062ed40 /
/// FUN_0062cb10): a sub-tick counter increments every call; when it hits a
/// per-type denominator computed from the SHP's frame-count parity and
/// the type's `StateAIAdvance` divisor, `animation_state` advances by 1.
/// Reaching `EndStateAI` either marks the particle for deletion (when
/// `DeleteOnStateLimit`) or resets the state to 0. Reaching
/// `Translucent50State` or `Translucent25State` writes the corresponding
/// translucency byte the renderer reads.
///
/// `image_frame_count` is the SHP frame count from
/// `Simulation::effect_frame_counts`. When it's 0 (image not registered or
/// missing SHP), the denominator falls through to `1 + StateAIAdvance` —
/// the same as if the SHP had an odd frame count, which is the gamemd
/// fallback when GetImageFrameCount returns 0.
pub(super) fn advance_state(
    p: &mut crate::sim::particles::Particle,
    pt: &crate::rules::particle_type::ParticleType,
    image_frame_count: u16,
) {
    let parity_bit = (image_frame_count % 2) as u8;
    let denom = (parity_bit + 1).saturating_add(pt.state_ai_advance).max(1);

    p.state_advance_counter = p.state_advance_counter.wrapping_add(1);
    if p.state_advance_counter % denom != 0 {
        return;
    }

    p.animation_state = p.animation_state.saturating_add(1);

    if p.animation_state == pt.end_state_ai {
        if pt.delete_on_state_limit {
            p.marked_for_deletion = true;
        } else {
            p.animation_state = 0;
        }
    }

    if pt.translucent_50_state != 0xFF && p.animation_state >= pt.translucent_50_state {
        p.translucency = 0x19;
    }
    if pt.translucent_25_state != 0xFF && p.animation_state >= pt.translucent_25_state {
        p.translucency = 0x32;
    }
}
```

**Step 2: Add unit tests**
Inside the existing `#[cfg(test)] mod tests` block at the bottom of
`system_ai.rs`, append:
```rust
mod advance_state_tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::particle_type::ParticleTypeId;
    use crate::sim::particles::Particle;
    use crate::util::fixed_math::SimFixed;
    use glam::IVec3;

    fn pt_rules(extra: &str) -> RuleSet {
        let ini = format!(
            "[Particles]\n1=Smk\n[Smk]\nBehavesLike=Smoke\nMaxEC=10\n{extra}\n"
        );
        RuleSet::from_ini(&IniFile::from_str(&ini)).expect("rules parse")
    }

    fn fake_particle(pt: &crate::rules::particle_type::ParticleType) -> Particle {
        Particle {
            type_id: ParticleTypeId(0),
            coords: IVec3::ZERO,
            previous_coords: IVec3::ZERO,
            origin: IVec3::ZERO,
            direction: [SimFixed::from_num(0); 3],
            velocity: SimFixed::from_num(0),
            lifetime_remaining: 100,
            damage_counter: 0,
            state_ai_advance: pt.state_ai_advance,
            animation_state: pt.start_state_ai,
            translucency: pt.translucency,
            hit_ground: false,
            marked_for_deletion: false,
            drift_x: 0, drift_y: 0, drift_z: 0,
            current_color: [0; 3],
            color_index: 0,
            color_accumulator: SimFixed::from_num(0),
            prev_delta: [SimFixed::from_num(0); 3],
            state_advance_counter: 0,
        }
    }

    #[test]
    fn even_frame_count_denom_is_state_ai_advance_plus_1() {
        // image_frame_count=20 (even), StateAIAdvance=4 → denom = (0+1) + 4 = 5.
        // After 4 ticks: counter=4, no advance. After 5: counter=5, animation_state=1.
        let rules = pt_rules("StateAIAdvance=4\nEndStateAI=99");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        for _ in 0..4 {
            advance_state(&mut p, pt, 20);
        }
        assert_eq!(p.animation_state, 0, "no advance before denom");
        advance_state(&mut p, pt, 20);
        assert_eq!(p.animation_state, 1, "advance on tick 5");
    }

    #[test]
    fn odd_frame_count_denom_is_state_ai_advance_plus_2() {
        // image_frame_count=21 (odd), StateAIAdvance=4 → denom = (1+1) + 4 = 6.
        let rules = pt_rules("StateAIAdvance=4\nEndStateAI=99");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        for _ in 0..5 {
            advance_state(&mut p, pt, 21);
        }
        assert_eq!(p.animation_state, 0);
        advance_state(&mut p, pt, 21);
        assert_eq!(p.animation_state, 1);
    }

    #[test]
    fn end_state_with_delete_on_state_limit_marks_for_deletion() {
        let rules = pt_rules("StateAIAdvance=0\nEndStateAI=2\nDeleteOnStateLimit=yes");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        // denom = (0+1)+0 = 1 → advance every tick.
        advance_state(&mut p, pt, 4); // state 0→1
        assert!(!p.marked_for_deletion);
        advance_state(&mut p, pt, 4); // state 1→2 (==EndStateAI)
        assert!(p.marked_for_deletion);
    }

    #[test]
    fn end_state_without_delete_resets_to_zero() {
        let rules = pt_rules("StateAIAdvance=0\nEndStateAI=2");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        advance_state(&mut p, pt, 4); // 0→1
        advance_state(&mut p, pt, 4); // 1→2 → reset to 0
        assert_eq!(p.animation_state, 0);
        assert!(!p.marked_for_deletion);
    }

    #[test]
    fn translucent_50_state_writes_0x19() {
        let rules = pt_rules("StateAIAdvance=0\nEndStateAI=99\nTranslucent50State=3");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        for _ in 0..3 {
            advance_state(&mut p, pt, 4);
        }
        assert_eq!(p.translucency, 0x19, "Translucent50State sets 0x19");
    }

    #[test]
    fn translucent_25_state_writes_0x32() {
        let rules = pt_rules("StateAIAdvance=0\nEndStateAI=99\nTranslucent25State=2");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        for _ in 0..2 {
            advance_state(&mut p, pt, 4);
        }
        assert_eq!(p.translucency, 0x32);
    }

    #[test]
    fn translucent_state_0xff_means_never() {
        // Both Translucent25State and Translucent50State default to 0xFF.
        let rules = pt_rules("StateAIAdvance=0\nEndStateAI=99");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        for _ in 0..50 {
            advance_state(&mut p, pt, 4);
        }
        // Translucency should still be the spawn-time value.
        assert_eq!(p.translucency, pt.translucency);
    }

    #[test]
    fn frame_count_zero_falls_through_to_odd_denom() {
        // image_frame_count=0 → parity_bit = 0 → denom = (0+1)+0 = 1.
        let rules = pt_rules("StateAIAdvance=0\nEndStateAI=99");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        advance_state(&mut p, pt, 0);
        assert_eq!(p.animation_state, 1, "denom=1 advances every tick");
    }

    #[test]
    fn counter_wraps_without_breaking_modulo() {
        // denom=5; let counter overflow.
        let rules = pt_rules("StateAIAdvance=4\nEndStateAI=99");
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut p = fake_particle(pt);
        for _ in 0..260 {
            advance_state(&mut p, pt, 20);
        }
        // 260 ticks / denom 5 = 52 advances expected. EndStateAI=99, no reset.
        assert_eq!(p.animation_state, 52);
    }
}
```

**Step 3: Verify**
```
cargo test -p ra2_engine particles::system_ai::tests::advance_state_tests
```
Expected: 8 tests pass.

**Step 4: Commit**
Message: `particles: add advance_state helper + state-AI denom unit tests`

---

### Task 3: Wire `advance_state` into Tier-2 tick functions

**Why:** Connect the helper to the live tick path. Smoke, Gas, and Fire each
need one call to `advance_state` per particle per tick, before the lifetime
decrement. The denominator needs the SHP frame count from
`Simulation::effect_frame_counts`.

**Files:**
- Modify: `src/sim/particles/system_ai.rs:18-50` (`tick_particle_systems`,
  `tick_one_system`, `tick_smoke`/`tick_gas`/`tick_fire`)
- Modify: `src/sim/particles/smoke.rs:114-124` (`tick_particle`)
- Modify: `src/sim/particles/gas.rs` (gas tick_particle)
- Modify: `src/sim/particles/fire.rs:135-145` (`tick_particle` — already
  has translucency mapping, see Step 4)

**Pattern:** Mirror `tick_particle_systems`'s existing borrow-juggle approach
(remove → mutate → reinsert). Resolve the per-particle `image_frame_count`
inside the per-particle inner loop, by looking up the particle type's
uppercased `image` name in `effect_frame_counts`.

**Step 1: Helper to resolve image frame count**
Add at top of `system_ai.rs`, before `advance_state`:
```rust
/// Resolve the SHP frame count for a particle's `Image=` field via the
/// existing `Simulation::effect_frame_counts` map. Returns 0 when the
/// type has no image set or the SHP is not registered (matches gamemd's
/// fallback).
pub(super) fn resolve_image_frame_count(
    sim: &crate::sim::world::Simulation,
    pt: &crate::rules::particle_type::ParticleType,
) -> u16 {
    let Some(image) = pt.image.as_deref() else { return 0 };
    let key = image.to_ascii_uppercase();
    let Some(id) = sim.interner.get(&key) else { return 0 };
    sim.effect_frame_counts.get(&id).copied().unwrap_or(0)
}
```

**Step 2: Update `smoke::tick_particle` to take the resolver**
In `smoke.rs`, change the signature of the per-particle helper:
```rust
pub(super) fn tick_particle(p: &mut Particle, pt: &ParticleType, image_frame_count: u16) {
    super::system_ai::advance_state(p, pt, image_frame_count);
    p.lifetime_remaining = p.lifetime_remaining.saturating_sub(1);
    if p.lifetime_remaining <= 0 {
        p.marked_for_deletion = true;
    }
    if p.velocity > SIM_ZERO {
        p.velocity = (p.velocity - pt.deacc).max(SIM_ZERO);
    }
}
```
And update its caller in the same file's `tick_system` (Phase 1 loop) to
resolve and pass the frame count. Since `tick_system` takes `&mut Simulation`,
call `resolve_image_frame_count(sim, pt)` inline.

**Step 3: Update `gas::tick_particle` analogously**
Same change: take `image_frame_count: u16`, call `advance_state` first,
then keep the existing damage/lifetime/decel logic. Update its caller in
`gas::tick_system`.

**Step 4: Update `fire::tick_particle` analogously**
Take `image_frame_count: u16`. Call `advance_state` AT THE TOP. Then
**remove** the existing translucency-state mapping ([fire.rs:148-162](src/sim/particles/fire.rs#L148))
because `advance_state` now owns it (avoid double-write). Keep the
damage/lifetime/decel logic. Update its caller in `fire::tick_system`.

**Verification step:** Read `fire.rs:148-162` before deletion to confirm
the translucency mapping is fully subsumed by `advance_state`. The existing
fire mapping reads `p.animation_state` and writes `p.translucency = 0x19 / 0x32`
based on the same Translucent50State / Translucent25State thresholds — so it
IS redundant once `advance_state` runs.

**Step 5: Update existing fire tests that pinned `animation_state` directly**
[fire.rs:296-360](src/sim/particles/fire.rs#L296) tests that force
`animation_state` to a specific value still pass — they don't call `tick_particle`,
they call the translucency mapping helper directly. If the helper is removed,
those tests need to either:
  (a) call `advance_state` instead with hand-set state, or
  (b) be deleted because `advance_state` is fully covered by Task 2's tests.

Read those tests first; pick (a) or (b) per test. The translucency-mapping
helper's tests should be deleted (covered by Task 2). The
final-damage-state-affects-damage tests (around fire.rs:355-365) are about
damage logic, not state-AI — keep them, they're independent of the
translucency byte.

**Step 6: Verify**
```
cargo test -p ra2_engine particles
```
Expected: existing tests still pass; new tick path doesn't regress lifetime
or damage behavior. If the test count drops by N (deleted redundant fire
translucency tests), that's expected and Task 2's tests now cover those
cases.

**Step 7: Commit**
Message: `particles: wire advance_state into smoke/gas/fire tick path`

---

### Task 4: `lepton_to_screen` helper

**Why:** Particle world coords are in leptons (256 = 1 cell). The renderer
needs sub-cell precision. Independent of all sim-side work, so can land
in parallel.

**Files:**
- Modify: `src/map/terrain.rs` — add helper near `iso_to_screen` at line 187

**Pattern:** Mirrors `iso_to_screen`'s style — short pub fn, doc-commented,
no allocations, with co-located unit tests.

**Step 1: Add the helper**
After `iso_to_screen` (around terrain.rs:192), insert:
```rust
/// Convert lepton-world coords to screen pixels with sub-cell precision.
///
/// 256 leptons = 1 cell. Used for systems with finer-than-cell positioning
/// (particles, smooth movement, projectiles). Returns the lifted screen
/// position so callers can apply per-sprite anchor offsets without
/// re-doing iso math.
///
///   X = (cell_x - cell_y) * TILE_WIDTH/2 + sub_offset_x
///   Y = (cell_x + cell_y) * TILE_HEIGHT/2 + TILE_HEIGHT/2 + sub_offset_y - z_lift
///
/// Negative coords are handled with `div_euclid` / `rem_euclid` so a
/// particle drifting just outside the map's NW corner stays on the
/// correct cell.
pub fn lepton_to_screen(coords: glam::IVec3) -> (f32, f32) {
    const LEPTONS_PER_CELL: i32 = 256;
    let cell_x = coords.x.div_euclid(LEPTONS_PER_CELL);
    let cell_y = coords.y.div_euclid(LEPTONS_PER_CELL);
    let sub_x = coords.x.rem_euclid(LEPTONS_PER_CELL) as f32;
    let sub_y = coords.y.rem_euclid(LEPTONS_PER_CELL) as f32;

    let cell_sx = (cell_x as f32 - cell_y as f32) * TILE_WIDTH / 2.0;
    let cell_sy = (cell_x as f32 + cell_y as f32) * TILE_HEIGHT / 2.0
        + TILE_HEIGHT / 2.0;

    let sub_sx = (sub_x - sub_y) * (TILE_WIDTH / 2.0) / LEPTONS_PER_CELL as f32;
    let sub_sy = (sub_x + sub_y) * (TILE_HEIGHT / 2.0) / LEPTONS_PER_CELL as f32;

    let z_lift = coords.z as f32 / LEPTONS_PER_CELL as f32 * HEIGHT_STEP;

    (cell_sx + sub_sx, cell_sy + sub_sy - z_lift)
}
```

**Step 2: Add unit tests**
Inside the existing test module at the bottom of terrain.rs, append:
```rust
#[test]
fn lepton_to_screen_zero_matches_iso_origin() {
    let (sx, sy) = lepton_to_screen(glam::IVec3::ZERO);
    // (0,0) cell center per iso_to_screen: ((0-0)*30 = 0, (0+0)*15 + 15 = 15).
    // lepton_to_screen returns the cell CENTER (not NW corner like iso_to_screen).
    assert_eq!(sx, 0.0);
    assert_eq!(sy, TILE_HEIGHT / 2.0);
}

#[test]
fn lepton_to_screen_integer_cell_lands_at_iso_center() {
    // 4 cells east, 2 cells south = (4*256, 2*256, 0).
    let (sx, sy) = lepton_to_screen(glam::IVec3::new(4 * 256, 2 * 256, 0));
    assert_eq!(sx, (4.0 - 2.0) * TILE_WIDTH / 2.0);
    assert_eq!(sy, (4.0 + 2.0) * TILE_HEIGHT / 2.0 + TILE_HEIGHT / 2.0);
}

#[test]
fn lepton_to_screen_sub_cell_offset_is_iso_subdivided() {
    // Sub-cell offset of (128, 0) — half a cell east in lepton coords.
    let (sx, sy) = lepton_to_screen(glam::IVec3::new(128, 0, 0));
    assert!((sx - (128.0 - 0.0) * (TILE_WIDTH / 2.0) / 256.0).abs() < 1e-3);
    assert!((sy - (TILE_HEIGHT / 2.0 + (128.0) * (TILE_HEIGHT / 2.0) / 256.0)).abs() < 1e-3);
}

#[test]
fn lepton_to_screen_negative_coords_use_euclidean_rounding() {
    // A particle at -50 leptons (west of origin) should land just west of cell 0.
    let (sx, _sy) = lepton_to_screen(glam::IVec3::new(-50, 0, 0));
    // div_euclid(-50, 256) = -1, rem_euclid = 206. So cell_x = -1, sub_x = 206.
    // Screen X = (-1 - 0) * 30 + (206 - 0) * 30 / 256 = -30 + 24.14… ≈ -5.86
    assert!(sx < 0.0, "sx={sx}");
    assert!(sx > -10.0, "sx={sx}");
}

#[test]
fn lepton_to_screen_z_lift_uses_height_step() {
    // Z = 256 leptons = 1 cell of altitude → screen Y lifted by HEIGHT_STEP.
    let (_, sy_low) = lepton_to_screen(glam::IVec3::ZERO);
    let (_, sy_high) = lepton_to_screen(glam::IVec3::new(0, 0, 256));
    assert!((sy_low - sy_high - HEIGHT_STEP).abs() < 1e-3);
}
```

**Step 3: Verify**
```
cargo test -p ra2_engine lepton_to_screen
```
Expected: 5 tests pass.

**Step 4: Commit**
Message: `terrain: lepton_to_screen helper for sub-cell sprite positioning`

---

### Task 5: Register particle SHPs into the sprite atlas

**Why:** The renderer's atlas lookup will silently miss every particle until
the SHPs are pre-rendered. Reuses the existing `effect_names` /
`effect_type_ids` channel so particle SHPs use anim.pal (matching gamemd's
ObjectTypeClass `Image=` path).

**Files:**
- Modify: `src/rules/ruleset.rs` — add `particle_types_iter` accessor
- Modify: `src/render/sprite_atlas.rs:653-723` — extend the `effect_names`
  collection step

**Pattern:** Mirrors the existing `effect_names.push(r.general.warp_in.name.clone())`
and damage-fire-types loops at sprite_atlas.rs:660-672.

**Step 1: Add `particle_types_iter` to RuleSet**
In `src/rules/ruleset.rs` near the existing `particle_type` accessor at
line 1413:
```rust
    /// Iterate every parsed `[Particles]` definition.
    pub fn particle_types_iter(&self) -> impl Iterator<Item = &ParticleType> {
        self.particle_types.iter()
    }
```

**Step 2: Extend `effect_names` collection in atlas builder**
In `src/render/sprite_atlas.rs`, just after the OccupantAnim loop at line
~694 and before the `for name in &effect_names` loop at line 696, insert:
```rust
            // Particle SHPs: ParticleType.Image= goes through ObjectTypeClass's
            // Image= path → anim.pal palette. Register every distinct name.
            for pt in r.particle_types_iter() {
                if let Some(image) = pt.image.as_deref() {
                    if !effect_names
                        .iter()
                        .any(|n| n.eq_ignore_ascii_case(image))
                    {
                        effect_names.push(image.to_string());
                    }
                }
            }
```

**Step 3: Verify particle SHP loading at startup**
The existing loop at sprite_atlas.rs:696-722 already handles MIX lookup,
frame-count enumeration, atlas key registration, and effect-palette tracking.
After Task 5, expect log lines like:
```
WorldEffect SHP LGRYSMK1: 21 frames loaded
WorldEffect SHP SGRYSMK1: 21 frames loaded
WorldEffect SHP WCCLOUD1: 28 frames loaded
WorldEffect SHP gaslrgmk: 12 frames loaded
WorldEffect SHP TXGASG: 21 frames loaded
WorldEffect SHP TXGASR: 21 frames loaded
```
Frame counts may vary by mod. Stock YR has these names per
`PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §10.5.2`.

**Step 4: Verify the frame counts flow into Simulation**
`active_anim_frame_counts` map is then copied into `Simulation::effect_frame_counts`
at [app_init.rs:469-472](src/app_init.rs#L469). Verify by adding a one-time
log in app_init.rs after the existing copy block:
```rust
log::info!(
    "Sim received {} effect frame counts from atlas (incl. particle SHPs)",
    sim.effect_frame_counts.len()
);
```
(This log line is for verification only — remove or keep as info, your call.)

**Step 5: Build + run a skirmish**
```
cargo run --release
```
Load a skirmish map. Expected: log lines confirm particle SHPs registered.
No visual change yet — particles still don't render until Task 8.

**Step 6: Commit**
Message: `sprite_atlas: register ParticleType.Image SHPs into effect_names`

---

### Task 6: `build_particle_instances` builder

**Why:** Generates the per-frame `SpriteInstance` vector for the new
particle pass. Single dispatch on `behaves_like` for frame index. Defensive
Tier-3 stub. This is the meatiest task — it carries most of the
parity-critical render-side details (Ledger L9-L21, L25-L26, L29-L30).

**Files:**
- Create: `src/app_instances/particles.rs`
- Modify: `src/app_instances/mod.rs:1-27` — add module + re-export

**Pattern:** Mirrors
[`build_world_effect_instances`](src/app_instances/overlays.rs#L59),
[`build_damage_fire_instances`](src/app_instances/overlays.rs#L113),
and [`build_parachute_instances`](src/app_instances/overlays.rs#L580).
Same atlas-lookup loop, same in-view culling, same shp_paged-style multi-page
output (but with its own pool keys). Tier-3 once-per-type log uses a local
`OnceLock<Mutex<HashSet>>`.

**Step 1: Create `src/app_instances/particles.rs`**
```rust
//! Particle-system instance builder — Layer 3 (above all ground objects).
//!
//! Reads `Simulation.particle_systems` and emits one SpriteInstance per
//! live particle, dispatched on the per-system BehavesLike for frame-index
//! calculation. Smoke/Gas use `animation_state` as the frame directly; Fire
//! uses `facing_band * EndStateAI + animation_state`. Spark/Railgun (Tier 3)
//! are filtered at spawn but a defensive once-per-type warn-log catches any
//! that slip through.
//!
//! Output pages match the sprite atlas page layout — particle pass uses its
//! own pool keys ("particle_p0".."particle_p3") drawn at Step 7.5 (between
//! cliff redraw and debug overlays).
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::app::AppState;
use crate::map::terrain;
use crate::render::batch::SpriteInstance;
use crate::render::sprite_atlas::ShpSpriteKey;
use crate::rules::house_colors::HouseColorIndex;
use crate::rules::particle_system_type::ParticleSystemBehavesLike;
use crate::rules::particle_type::ParticleBehavesLike;

use super::helpers::in_view;

/// Screen-Y nudge applied to every particle position. gamemd's CC_Draw_Shape
/// gets `-15 - AdjustForZ()`; for Tier-2 (no airborne particles) AdjustForZ
/// is 0, so −15 px is the lift that puts smoke origins just above the
/// spawn cell instead of buried in it.
const PARTICLE_Y_LIFT: f32 = 15.0;

/// Build SpriteInstance entries for every live particle in the simulation.
///
/// Caller passes the paged output vector list (one Vec per atlas page, sized
/// `state.sprite_atlas.page_count()`). This function appends; sorting is the
/// caller's responsibility (see `build_world_instances`).
pub(crate) fn build_particle_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
) {
    let (sim, atlas, rules) = match (
        &state.simulation,
        &state.sprite_atlas,
        &state.rules,
    ) {
        (Some(s), Some(a), Some(r)) => (s, a, r),
        _ => return,
    };

    let z = state.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.camera_x,
        state.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );

    for (_sys_id, sys) in sim.particle_systems.iter() {
        let pst = rules.particle_system_type(sys.type_id);
        match pst.behaves_like {
            ParticleSystemBehavesLike::Spark | ParticleSystemBehavesLike::Railgun => {
                warn_once_per_tier3_type(pst.behaves_like);
                continue;
            }
            _ => {}
        }

        for p in &sys.particles {
            let pt = rules.particle_type(p.type_id);
            let Some(image_name) = pt.image.as_deref() else { continue };

            // Frame index dispatch (L9 / L10 / L11).
            let frame: u16 = match pt.behaves_like {
                ParticleBehavesLike::Smoke | ParticleBehavesLike::Gas => {
                    p.animation_state as u16
                }
                ParticleBehavesLike::Fire => {
                    let facing_band = (sys.facing as u16 / 0x40) & 0x3;
                    facing_band * pt.end_state_ai as u16 + p.animation_state as u16
                }
                _ => continue, // Particle-side Spark/Railgun — silent skip.
            };

            // Atlas lookup (L18-L21). Silent miss per L19.
            let key = ShpSpriteKey {
                type_id: image_name.to_string(),
                facing: 0,
                frame,
                house_color: HouseColorIndex(0),
            };
            let Some(entry) = atlas.get(&key) else { continue };

            // Lepton coords → screen (L29 / L30).
            let (sx, sy_raw) = terrain::lepton_to_screen(p.coords);
            let sy = sy_raw - PARTICLE_Y_LIFT;

            if !in_view(sx, sy, 64.0, 64.0, cam_x, cam_y, sw, sh, 120.0) {
                continue;
            }

            // Translucency byte → alpha (L13-L16).
            let alpha = match p.translucency {
                0x00 => 1.0,
                0x19 => 0.5,
                0x32 => 0.25,
                t if t >= 0x4A => 0.16,
                _ => 1.0, // unexpected byte: render opaque, don't crash
            };

            // Depth = sy for back-to-front Y-sort (L24). The pass uses
            // passthrough (no GPU depth read/write); this field is only
            // used by the CPU-side sort_by_depth_desc the caller runs.
            let depth = sy;

            paged[entry.page as usize].push(SpriteInstance {
                position: [sx + entry.offset_x, sy + entry.offset_y],
                size: entry.pixel_size,
                uv_origin: entry.uv_origin,
                uv_size: entry.uv_size,
                depth,
                tint: [1.0, 1.0, 1.0], // L20: no owner tint
                alpha,
            });
        }
    }
}

/// Once-per-type warn log for Tier-3 systems that slip past the spawn-side
/// filter at sim/particles/spawn.rs:42. Defense in depth — the spawn side
/// is the primary guard, this catches snapshot loads / future bugs.
fn warn_once_per_tier3_type(kind: ParticleSystemBehavesLike) {
    static SEEN: OnceLock<Mutex<HashSet<ParticleSystemBehavesLike>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut set) = seen.lock() {
        if set.insert(kind) {
            log::warn!(
                "particles: render found Tier-3 system {:?} in store \
                 (spawn-side filter should have caught this); skipping",
                kind
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // Pure-logic tests for the alpha mapping and frame-index dispatch.
    // Full instance-building integration is covered by Task 11.

    #[test]
    fn translucency_byte_to_alpha_table() {
        // Mirror the match arm; values come from doc §8.7 / §9.7.
        fn alpha(b: u8) -> f32 {
            match b {
                0x00 => 1.0,
                0x19 => 0.5,
                0x32 => 0.25,
                t if t >= 0x4A => 0.16,
                _ => 1.0,
            }
        }
        assert_eq!(alpha(0x00), 1.0);
        assert_eq!(alpha(0x19), 0.5);
        assert_eq!(alpha(0x32), 0.25);
        assert_eq!(alpha(0x4A), 0.16);
        assert_eq!(alpha(0xFF), 0.16);
        // Unexpected mid-range values default opaque.
        assert_eq!(alpha(0x40), 1.0);
    }

    #[test]
    fn fire_frame_uses_facing_band_times_end_state() {
        // Manual dispatch reproduction so the formula is unit-testable
        // without spinning up an AppState.
        fn fire_frame(facing: u8, end_state_ai: u8, animation_state: u8) -> u16 {
            let facing_band = (facing as u16 / 0x40) & 0x3;
            facing_band * end_state_ai as u16 + animation_state as u16
        }
        // Default ParticleSystem.facing=0x1D → facing_band=0 → frame = animation_state.
        assert_eq!(fire_frame(0x1D, 19, 5), 5);
        // Facing 0x40 → band 1 → frame = end_state_ai + animation_state.
        assert_eq!(fire_frame(0x40, 19, 5), 24);
        assert_eq!(fire_frame(0x80, 19, 5), 43);
        assert_eq!(fire_frame(0xC0, 19, 5), 62);
    }
}
```

**Step 2: Wire into `app_instances/mod.rs`**
In `src/app_instances/mod.rs`, after the `mod overlays;` block at line 25:
```rust
mod particles;
pub(crate) use particles::*;
```

**Step 3: Verify**
```
cargo test -p ra2_engine app_instances::particles
cargo build
```
Expected: 2 unit tests pass; clean build (no callers wired yet, builder
is dead code).

**Step 4: Commit**
Message: `app_instances: build_particle_instances renderer for Layer 3`

---

### Task 7: Wire builder into `WorldInstances` + GPU upload

**Why:** Connect the builder to the per-frame pipeline. Allocate the paged
vector, call the builder, sort each page, upload to pool keys.

**Files:**
- Modify: `src/app_render/build_instances.rs:32-46` (`WorldInstances` struct)
- Modify: `src/app_render/build_instances.rs:94-247` (`build_world_instances`)
- Modify: `src/app_render/mod.rs:119-192` (`upload_to_gpu`)

**Pattern:** Mirrors how `shp_paged` is allocated, filled, sorted, and uploaded.

**Step 1: Add `particle_paged` to `WorldInstances`**
In `build_instances.rs:32-46`, append a new field:
```rust
    /// Per-particle SpriteInstances (Layer 3). Drawn at Step 7.5 — above
    /// all ground objects + cliffs, below debug/shroud/UI.
    pub particle_paged: Vec<Vec<SpriteInstance>>,
```

**Step 2: Allocate, fill, sort in `build_world_instances`**
After the existing `shp_paged` allocation at build_instances.rs:183, add:
```rust
    let mut particle_paged: Vec<Vec<SpriteInstance>> = vec![Vec::new(); shp_page_count];
```

After the `app_instances::build_parachute_instances(state, &mut shp_paged);`
call at build_instances.rs:211, add:
```rust
    // Layer 3 particle systems — separate paged list, drawn AFTER cliff redraw
    // (Step 7.5), above all ground geometry per gamemd ParticleClass::GetLayer.
    app_instances::build_particle_instances(state, &mut particle_paged);
    for page in &mut particle_paged {
        sort_by_depth_desc(page);
    }
```

In the `WorldInstances { ... }` literal at line 234, add:
```rust
        particle_paged,
```

**Step 3: Upload to pool**
In `app_render/mod.rs::upload_to_gpu` after the bridge_shp_paged upload
loop (around line 164):
```rust
    const PARTICLE_KEYS: [&str; 4] = [
        "particle_p0", "particle_p1", "particle_p2", "particle_p3",
    ];
    for (i, page_inst) in world.particle_paged.iter().enumerate() {
        if i < PARTICLE_KEYS.len() {
            pool.upload(&state.gpu, PARTICLE_KEYS[i], page_inst);
        }
    }
```

**Step 4: Verify**
```
cargo build
```
Expected: clean build. No visual change yet — instance buffer is uploaded
but never drawn.

**Step 5: Commit**
Message: `app_render: allocate + upload particle_paged buffer for Layer 3`

---

### Task 8: Add Step 7.5 draw call for the particle pass

**Why:** Final wiring — actually draws the uploaded particle instances. After
this task, particles are visible in-game.

**Files:**
- Modify: `src/app_render/draw_passes.rs:26-33` (`DrawPassData`)
- Modify: `src/app_render/mod.rs:99-107` (DrawPassData construction in
  `render_game`)
- Modify: `src/app_render/draw_passes.rs:151-163` (insert Step 7.5 between
  steps 7 and 8)

**Pattern:** Mirrors the `bridge_shp_paged` draw-loop pattern in
[draw_passes.rs](src/app_render/draw_passes.rs) Step 4, but with passthrough
pipeline (no depth read/write) and against the sprite atlas pages directly.

**Step 1: Extend `DrawPassData`**
In draw_passes.rs:26-33, add the field:
```rust
    pub particle_paged: &'a [Vec<SpriteInstance>],
```

**Step 2: Pass through from `render_game`**
In `app_render/mod.rs::render_game` at line 99, add to the `DrawPassData`
literal:
```rust
            particle_paged: &world.particle_paged,
```

**Step 3: Insert Step 7.5**
In draw_passes.rs, after the cliff redraw block at line 163 and BEFORE the
"--- Step 8: Debug overlays ---" comment at line 165, add:
```rust
    // --- Step 7.5: Particles (Layer 3, above all ground geometry incl. cliffs) ---
    // gamemd ParticleClass::GetLayer returns 3 for all particles, drawing them
    // above Layer 2 (buildings, units, turrets) and above cliff redraw.
    // Passthrough pipeline (no depth interaction) — particles are translucent
    // and Y-sorted on the CPU, so no GPU depth read/write needed.
    const PARTICLE_KEYS: [&str; 4] = [
        "particle_p0", "particle_p1", "particle_p2", "particle_p3",
    ];
    for (i, key) in PARTICLE_KEYS.iter().enumerate() {
        if let Some(page) = state.sprite_atlas.as_ref().and_then(|a| a.page(i)) {
            if let Some((buf, count)) = pool.get(key) {
                if count == 0 {
                    continue;
                }
                state.batch_renderer.draw_passthrough_range(
                    &mut pass,
                    &page.texture,
                    buf,
                    0,
                    count,
                );
            }
        }
    }
```

**Step 4: Verify**
```
cargo build --release
```
Expected: clean build. Run a skirmish.

**Step 5: Visual end-to-end check**
1. Build a power plant and an ore refinery.
2. Wait for miner to dump ore.
3. Expected: refinery dump bursts visible — small smoke puff(s) above the
   refinery roof when the miner empties.
4. Damage a building (artillery, GI rifles): smoke plume should rise above
   the roof and fade through translucent states.
5. With a Yuri Disk weapon (psychic gas), spam the cloud over a clump of
   infantry: gas cloud should drift, fade through translucent states, and
   spawn smaller dissipation clouds.

If no particles are visible:
- Check log: was "WorldEffect SHP LGRYSMK1: ..." printed at startup? (Task 5)
- Check log: was "Sim received N effect frame counts" printed? (Task 5)
- Check `Simulation::particle_systems.len()` via debug overlay — is the
  store actually populated?
- Add a single-tick log inside `build_particle_instances` to count emitted
  SpriteInstances per frame; bisect from there.

**Step 6: Commit**
Message: `app_render: Step 7.5 particle pass between cliff redraw and debug`

---

### Task 9: Spawn-coord fix — refinery dump from cell center

**Why:** Independent rider task identified during the harvester dock trace.
Once the renderer exists this becomes visually verifiable: refinery dump
smoke should bloom from the cell center, not the cell's NW corner.

**Files:**
- Modify: `src/app_building_anim.rs:393-394` (origin coord computation)

**Pattern:** Single-line change. gamemd's `BuildingClass::GetCoords` returns
top-left cell **center** (rx*256+128, ry*256+128).

**Step 1: Apply the fix**
In `app_building_anim.rs::consume_bale_events` around line 391-407, change:
```rust
                    let origin_x = building.position.rx as i32 * 256;
                    let origin_y = building.position.ry as i32 * 256;
```
to:
```rust
                    // BuildingClass::GetCoords returns cell CENTER per
                    // gamemd UndockUnit's (-0x80, +0x80) baseline. The +128
                    // is the lepton offset from cell NW corner to center.
                    let origin_x = building.position.rx as i32 * 256 + 128;
                    let origin_y = building.position.ry as i32 * 256 + 128;
```

**Step 2: Verify**
Visual: load a skirmish, dump ore, the smoke origin should now visually
center over each refinery cell rather than land in the NW corner. Compare
side-by-side against gamemd if possible.

Headless: existing tests pass (`cargo test -p ra2_engine consume_bale`).
The +128 shift doesn't change cell membership, so no test should regress.

**Step 3: Commit**
Message: `refinery: anchor dump particle spawn at cell center per gamemd`

---

### Task 10: World-hash extension for `state_advance_counter`

**Why:** Determinism. The new sub-tick counter must be in the state hash or
two replays could diverge silently when `delete_on_state_limit` fires on a
different tick.

**Files:**
- Modify: `src/sim/world/world_hash.rs:55-65` (per-particle hash loop)

**Pattern:** One-line addition inside the existing per-particle loop. The
loop already hashes every other determinism-relevant field.

**Step 1: Hash the field**
In `hash_particle_systems`, inside the per-particle loop (around line 55-64),
add:
```rust
                p.state_advance_counter.hash(hasher);
```
Place it next to the other state-AI fields (after `p.translucency.hash(hasher);`).

**Step 2: Add a determinism unit test**
At the bottom of the existing `particle_hash_tests` module in world_hash.rs,
append:
```rust
    #[test]
    fn state_advance_counter_changes_hash() {
        use crate::sim::particles::Particle;
        use crate::rules::particle_type::ParticleTypeId;
        use crate::util::fixed_math::SimFixed;

        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let mut sys_a = fake_system(IVec3::ZERO);
        let mut sys_b = fake_system(IVec3::ZERO);
        // Two systems, each with one particle. Counter differs between them.
        let make_p = |counter: u8| Particle {
            type_id: ParticleTypeId(0),
            coords: IVec3::ZERO, previous_coords: IVec3::ZERO, origin: IVec3::ZERO,
            direction: [SimFixed::from_num(0); 3],
            velocity: SimFixed::from_num(0),
            lifetime_remaining: 100, damage_counter: 0,
            state_ai_advance: 4, animation_state: 0, translucency: 0,
            hit_ground: false, marked_for_deletion: false,
            drift_x: 0, drift_y: 0, drift_z: 0,
            current_color: [0; 3], color_index: 0,
            color_accumulator: SimFixed::from_num(0),
            prev_delta: [SimFixed::from_num(0); 3],
            state_advance_counter: counter,
        };
        sys_a.particles.push(make_p(0));
        sys_b.particles.push(make_p(3));
        sim_a.particle_systems.insert(sys_a);
        sim_b.particle_systems.insert(sys_b);
        assert_ne!(sim_a.state_hash(), sim_b.state_hash(),
                   "state_advance_counter must affect state hash");
    }
```

**Step 3: Verify**
```
cargo test -p ra2_engine particle_hash_tests
```
Expected: existing 2-3 tests + new test all pass. If existing tests fail,
their `Particle` fixtures need updating — that should already be the case
from Task 1.

**Step 4: Commit**
Message: `world_hash: include state_advance_counter for replay determinism`

---

### Task 11: Headless integration test

**Why:** End-to-end verification without spinning up the GPU. Asserts the
full sim → render data path produces the expected SpriteInstance count.

**Files:**
- Create: `tests/particle_render_integration.rs` (or add to an existing
  `src/sim/world/integration_tests.rs` if there's a natural fit)

**Pattern:** Mirrors
[`smudge_state_hash_stable_across_advance_tick`](src/sim/world/smudge_integration_tests.rs#L99) —
build a minimal Simulation + RuleSet, spawn the system, advance, observe.

**Step 1: Decide module location**
Run `grep -l "advance_tick" tests/` and look for a particle-related test
file. If none exists, create `tests/particle_render_integration.rs`.
(Note: `tests/` directory contains integration tests by Cargo convention.)

**Step 2: Write the test**
```rust
//! Integration test: spawn one BigGreySmokeSys, advance ticks, verify the
//! state-AI advance progresses animation_state and translucency state and
//! that particles persist in the store as expected.

use ra2_engine::rules::ini_parser::IniFile;
use ra2_engine::rules::particle_system_type::ParticleSystemTypeId;
use ra2_engine::rules::ruleset::RuleSet;
use ra2_engine::sim::particles::system_ai::tick_particle_systems;
use ra2_engine::sim::world::Simulation;
use glam::IVec3;

/// Minimal RuleSet with one Smoke system + type, mimicking BigGreySmokeSys.
fn build_smoke_rules() -> RuleSet {
    let ini = IniFile::from_str("\
[Particles]\n\
1=LargeGreySmoke\n\
[LargeGreySmoke]\n\
BehavesLike=Smoke\n\
Image=LGRYSMK1\n\
MaxEC=80\n\
Translucency=50\n\
EndStateAI=20\n\
StateAIAdvance=4\n\
DeleteOnStateLimit=yes\n\
[ParticleSystems]\n\
1=BigGreySmokeSys\n\
[BigGreySmokeSys]\n\
BehavesLike=Smoke\n\
HoldsWhat=LargeGreySmoke\n\
Spawns=yes\n\
SpawnFrames=10\n\
ParticleCap=15\n\
");
    RuleSet::from_ini(&ini).expect("parse")
}

#[test]
fn smoke_animation_state_advances_over_ticks() {
    let rules = build_smoke_rules();
    let mut sim = Simulation::new();

    // Pre-populate effect_frame_counts as if the atlas had registered
    // LGRYSMK1 with 21 frames (matches stock YR).
    let id = sim.interner.intern("LGRYSMK1");
    sim.effect_frame_counts.insert(id, 21);

    let sys_id = sim.spawn_particle_system(
        ParticleSystemTypeId(0),
        IVec3::new(1024, 1024, 0),
        None, None, IVec3::ZERO, None,
        &rules,
    ).expect("spawn");

    // Advance enough ticks to spawn at least one particle and let
    // animation_state advance from 0.
    // SpawnFrames=10 → first particle at tick 10.
    // image_frame_count=21 (odd) → denom = (1+1) + 4 = 6.
    for _ in 0..30 {
        tick_particle_systems(&mut sim, &rules);
        sim.tick += 1;
    }

    let sys = sim.particle_systems.get(sys_id).expect("system alive");
    assert!(!sys.particles.is_empty(), "should have spawned particles");
    let p = &sys.particles[0];
    // 30 ticks - 10 spawn delay = 20 alive ticks; 20/6 = 3 advances.
    // (Within tolerance — exact value depends on borrow-juggle timing in
    // tick_system. Assert advance happened, not the exact value.)
    assert!(p.animation_state > 0, "animation_state should advance");
}
```

**Step 3: Verify**
```
cargo test --test particle_render_integration
```
Expected: 1 test pass.

**Step 4: Commit**
Message: `particles: integration test for sim → animation_state advance`

---

### Task 12: Visual verification against gamemd.exe

**Why:** Final parity check. Code-level correctness doesn't guarantee
indistinguishable visuals — this task explicitly observes side-by-side
against the original engine.

**Verify:**
- **Refinery dump smoke:** Build a refinery, harvest, observe each dump.
  - Expected (gamemd): brief grey/black smoke puff(s) above the refinery
    roof at the moment of dump, anchored at cell center, fading through
    translucent states over ~80 ticks.
  - Verify: matches in our engine after Task 9 (cell-center spawn).

- **Damage smoke:** Damage a building below ConditionYellow.
  - Expected (gamemd): persistent smoke plume rising above the damaged
    building, animated through SHP frames, fading at Translucent50State /
    Translucent25State (smoke gradually transparentizes before disappearing).
  - Verify: animation cycles through frames, not stuck on frame 0.

- **Gas cloud (Yuri Disk vs infantry):** Fire psychic gas weapon at a clump.
  - Expected (gamemd): green-yellow gas cloud that drifts slightly,
    persists for ~50 ticks, dissipates via NextParticle chain (smaller
    secondary cloud).
  - Verify: dissipation visible (smaller particles spawn as parents die).

- **Layer 3 ordering:** Place a building behind smoke.
  - Expected (gamemd): smoke draws ON TOP of the building's roof, even
    when the smoke's screen-Y would otherwise sort it behind.
  - Verify: smoke is never occluded by building chrome.

- **Cliff occlusion:** Drift smoke near a cliff face.
  - Expected (gamemd): smoke draws on top of the cliff face (Layer 3 >
    cliff redraw).
  - Verify: smoke not clipped by cliff redraw pass.

- **No house tint:** Build refineries for two different players, dump ore.
  - Expected: both refineries' smoke is the same neutral grey — particles
    don't tint to owner colors.
  - Verify: no remap / tint on particle SHPs.

If any check fails: log a follow-up issue (don't fix in-place) and revisit
in a separate task.

**No commit step** — verification only.

---

## Sources & References

- **Design doc:** docs/plans/2026-05-07-particle-system-rendering-design.md
- **Ghidra report:** ra2-rust-game-docs/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md
  (3,482 lines — primary source for struct layouts, AI dispatch, Draw_It,
  state-AI machine, ColorList, INI keys, all 13 systems + 22 types).
- **Key gamemd addresses (kept here, NOT in Rust comments):**
  - `0x0062cec0` `ParticleClass::Draw_It` — render dispatch (§8.7)
  - `0x0062d770` `ParticleClass::GetLayer` — returns 3 (§5.3)
  - `0x0062d830` `ParticleClass::GetImageFrame` — frame index by
    BehavesLike (§9.12.3)
  - `0x0062fd60` `ParticleSystemClass::AI` — system AI dispatch (§3.1)
  - `0x0062ed40` `ParticleSystemClass::AI_Smoke` — smoke advance (§3.3)
  - `0x0062f9a0` `ParticleSystemClass::AI_Fire` — fire advance + state
    AI (§3.6)
  - `0x00644F50` `ParticleTypeClass::ReadINI` — INI parsing (§4.2)
  - `0x005F92D0` `ObjectTypeClass::ReadINI` — Image= path (§11.4)
  - `0x005F9070` SHP loader — anim.pal (§11.4)
- **INI keys consumed:**
  - `[Particles]` Image, BehavesLike, MaxEC, MaxDC, Damage, EndStateAI,
    StartStateAI, StateAIAdvance, Translucent25State, Translucent50State,
    DeleteOnStateLimit, Velocity, Deacc, Translucency, Radius, NumLoopFrames,
    NextParticle, NextParticleOffset
  - `[ParticleSystems]` BehavesLike, HoldsWhat, Spawns, SpawnFrames,
    SpawnRadius, Slowdown, ParticleCap, Lifetime, SpawnCutoff,
    SpawnTranslucencyCutoff, SpawnDirection
- **Repo patterns mirrored:**
  - `src/app_instances/overlays.rs:59-176` — instance-builder pattern
    (build_world_effect_instances, build_damage_fire_instances,
    build_parachute_instances)
  - `src/render/sprite_atlas.rs:653-723` — `effect_names` SHP registration
  - `src/app_init.rs:466-473` — frame-count plumbing into Simulation
  - `src/sim/miner/miner_system.rs:705-709` — chrono-teleport effect_frame_counts
    consumer pattern
  - `src/app_render/draw_passes.rs:151-203` — draw-step insertion pattern
  - `src/sim/world/world_hash.rs:42-66` — per-particle determinism hashing
- **Related design doc:** docs/plans/2026-05-07-particle-system-rendering-design.md
  (this plan implements that design doc end-to-end).

