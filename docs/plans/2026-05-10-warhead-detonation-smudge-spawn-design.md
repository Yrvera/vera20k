# Warhead-Detonation Smudge Spawn (Kill-Independent) Design

## Goal
Spawn the warhead's `AnimList` animation — and any `Scorch=`/`Crater=`-driven smudge — on **every** warhead detonation, not just the ones that kill an entity. Matches gamemd's `WarheadType::Detonate → AnimClass::Start` trigger documented in [SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md).

## Architecture Context

Two independent pieces already exist:

- **`combat/smudge_dispatch.rs`** — full-fidelity dispatcher: altitude gate (<30 leptons), 50/50 scorch/crater pick when both flags set, `Reduce_Tiberium(6)` ordering, `ForceBigCraters` path, building-center / building-survivor handlers, deterministic 256-entry unit-vector table for the survivor offset. Drained per-tick by `Simulation::advance_tick`.
- **`SmudgeSpawnRequest::Anim { anim_name, rx, ry, z }`** — the right transport for "this anim spawned at this cell, run AnimClass::Start logic on it". Already routed through the drain.

The bug is at the **emission** layer: `SmudgeSpawnRequest::Anim` is only pushed inside `handle_entity_deaths` ([combat/mod.rs:759-786](../../src/sim/combat/mod.rs#L759-L786)) where it's gated on `killing_warhead = damage_events.iter().rfind(...)`. A V3 splash that injures but does not kill yields no killing_warhead → no anim → no smudge. The same is true for force-fire on terrain (no entity to kill at all), for death-weapon AoE detonations (Demo Truck, `Explodes=yes` units), and for superweapon AoE (Genetic Mutator, Lightning Storm).

The `ExplosionEffect` (warhead AnimList anim) is bound to the same gate and has the same scope problem.

## Impact Analysis

**Files modified:**
- [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — add helper, add emission at per-shot AoE branch + per-shot direct-hit branch + death-AoE loop, remove emission from killing-warhead block.
- [src/sim/superweapon/genetic_converter.rs](../../src/sim/superweapon/genetic_converter.rs) — emit at the Mutate AoE callsite.
- [src/sim/superweapon/lightning_storm.rs](../../src/sim/superweapon/lightning_storm.rs) — emit at the lightning AoE callsite.
- [src/sim/world/mod.rs](../../src/sim/world/mod.rs) — add `Simulation::pending_smudge_requests`, drain alongside `combat_result.smudge_spawn_requests` in the existing drain block (~line 1271).

**No structural changes** to `SmudgeSpawnRequest`, `ExplosionEffect`, the dispatcher, or the per-tick drain ordering.

**Determinism:** the new emissions consume RNG inside the dispatcher (50/50 scorch/crater pick + scattered survivor RNG advances). World-state hash will change vs. current `dev` HEAD. Accepted by user — this is the cost of the parity fix.

**Tick-order:** superweapon emissions land in `pending_smudge_requests` during Phase 4.5 (before combat). Combat emissions land in `combat_result.smudge_spawn_requests` during Phase 5. Both vecs are drained in the existing post-combat drain block, in order: pending first, then combat result. The drain consumes RNG deterministically.

**Risk areas:**
- **Double-emission:** if the death-handler emission isn't fully removed, kills produce two anim spawns. Test 2 catches this.
- **Empty-AnimList warheads:** `Pulse` and a few esoteric warheads have empty `AnimList=`. Helper short-circuits — no emission, no RNG consumption. Test 3 catches this.
- **Cell-target z:** force-fire on terrain has no entity, so we use `z=0` (terrain ground level reference). The dispatcher re-derives `ground_z` from `terrain.cell(rx,ry).level * 15`, so the altitude gate works correctly with `z=0`.
- **Rocket-projectile timing:** for V3 (and other rockets), our damage application already happens at fire time, not arrival time. The smudge will spawn at fire time too. This is consistent with our existing damage-timing gap and not a new bug. **Documented as known follow-up parity gap.**

## Chosen Approach

**Approach B — centralized helper in `combat/mod.rs`.**

Add a single helper:

```rust
/// Emit the warhead's AnimList animation (and a paired smudge spawn request)
/// for one detonation at (rx, ry, z). Mirrors gamemd's WarheadType::Detonate
/// dispatch into AnimClass::Start. Pushes nothing if AnimList is empty.
///
/// `base_damage` is the post-modifier damage at the impact center (used to
/// pick the AnimList index via `damage / 25`, clamped to len-1).
pub(crate) fn emit_warhead_detonation_effects(
    warhead: &WarheadType,
    base_damage: i32,
    rx: u16,
    ry: u16,
    z: u8,
    interner: &mut StringInterner,
    explosion_effects: &mut Vec<ExplosionEffect>,
    smudge_spawn_requests: &mut Vec<SmudgeSpawnRequest>,
)
```

Every detonation site calls it. Superweapons receive their pushes via a sim-owned `pending_smudge_requests` vec plus a small adapter that pushes directly into `sim.world_effects` (the helper's `explosion_effects` vec is unused at superweapon sites — they spawn `WorldEffect` inline like Lightning Storm already does).

**Why this over inline copy-paste (Approach A):** four callsites, identical 8-line logic, drift risk on any future change to AnimList indexing or the request shape. One helper is materially cleaner with the same parity properties.

**Why not embed in `apply_aoe_damage` (Approach C):** the per-shot direct-hit branch (CellSpread = 0) bypasses `apply_aoe_damage` entirely, so we'd still need a separate emission. Defeats the unification and pollutes a pure damage-distribution function.

## Tiny-Detail Ledger

Each item below carries through to `/write-plan` and the implementation. The dispatcher already enforces 1-9; this design's responsibility is 10-15.

1. **Altitude gate is strictly `< 30` leptons.** [GHIDRA 0x42505A] Already in `try_dispatch_anim_smudge`.
2. **Default dmg/dmg2 = 30 (`0x1E`)** when SHP frame rect not yet cached. Already enforced via eager `populate_anim_frame_dims` (commits f906c63, 103fc5d, 93f4b78).
3. **Scorch + Crater 50/50 = `(rand * 2^-31) < 0.5`**, equivalent to `rand < 0x80000000`. Already in `rng_below_half_normalized`. [GHIDRA 0x4250AB]
4. **Reduce_Tiberium(6) fires before placement attempt** in the crater path, even when CanPlaceHere fails. Already in `try_dispatch_anim_smudge`. [GHIDRA 0x004250E1]
5. **Reduce_Tiberium uses immediate `6`**, not a rules constant. `CRATER_ORE_REDUCTION = 6` constant. [GHIDRA 0x004250E5]
6. **AnimClass::Start runs on first frame only** — anim creation, not destruction. Drain runs same tick as emission. [doc §6 ledger #7]
7. **`forceBig != 0` is truthiness, not equality with 1.** `ForceBigCraters` path passes `(300, 300, true)`. Already encoded. [GHIDRA 0x6B5DCC]
8. **At most ONE smudge per anim per frame** — scorch-arm and crater-arm both early-return after `try_place`. Already encoded. [GHIDRA 0x004250CA, 0x0042511B]
9. **Per-AnimType offsets `+0x36B/+0x36D/+0x36E`** read into `ArtEntry { scorch, crater, force_big_craters }`. Already parsed.

Items added by THIS design's emission move:

10. **AnimList index is `damage / 25`, clamped to `len-1`.** Helper computes it once. `damage` is the **post-modifier base damage at impact center** — i.e. for garrison fire, post-`occupy_damage_multiplier`; for Mutate, the `MUTATE_AOE_DAMAGE` constant; for Lightning, `rules.general.lightning_damage`; for death-weapon AoE, the death-weapon's `damage`. [combat/mod.rs:660-661, 770-771]
11. **One emission per detonation, regardless of AoE hit count.** Helper called once per detonation, AT the impact cell — never per AoE-hit-entity. Test 4 specifically asserts this for AoE warheads.
12. **Death-handler emission (the killing-warhead branch at combat/mod.rs:759-786) is REMOVED.** The killing shot's anim is emitted at the per-shot site instead. Death handler keeps its other side-effects (DieSound, BuildingCenter / BuildingSurvivor smudges, InfDeath animation selection).
13. **Death-AoE loop emits independently of the killing shot.** A Demo Truck killed by tank fire → killing shot emits Tank Cannon AnimList; death-AoE loop emits the Demo Truck's death-weapon AnimList. Two distinct detonations, two distinct emissions. Test 4 covers this.
14. **Cell targets emit too.** `TargetKind::Cell` (force-fire on terrain) detonates the warhead → emit. The current per-shot AoE branch is already the cell-target path; helper invocation is unconditional on target kind.
15. **Burst weapons emit once per burst sub-shot.** Per-shot block runs once per burst, no de-duplication.

## Design

### Components

**New helper (combat/mod.rs):**
```rust
fn emit_warhead_detonation_effects(
    warhead: &WarheadType,
    base_damage: i32,
    rx: u16,
    ry: u16,
    z: u8,
    interner: &mut StringInterner,
    explosion_effects: &mut Vec<ExplosionEffect>,
    smudge_spawn_requests: &mut Vec<SmudgeSpawnRequest>,
)
```

Body: if `warhead.anim_list.is_empty() { return; }`; index = `(base_damage / 25).min(warhead.anim_list.len() - 1)`; intern the anim name; push `ExplosionEffect` and `SmudgeSpawnRequest::Anim` with the same interned id, rx, ry, z.

**New sim field:**
```rust
// in Simulation
pending_smudge_requests: Vec<SmudgeSpawnRequest>,
```
Populated by superweapon callsites that don't return through `CombatTickResult`. Drained alongside `combat_result.smudge_spawn_requests` in the post-combat drain block, then cleared.

### Interfaces / Contracts

- Helper signature is fixed in `combat/mod.rs` — internal to the crate (`pub(crate)`).
- Drain order in `Simulation::advance_tick` is **superweapon-emitted first, then combat-emitted** (collection order). Both share the same dispatcher and the same RNG cursor; ordering is deterministic.
- Lightning Storm continues to spawn its own bolt visual via `sim.world_effects.push(...)`. The new helper-emitted anim is the **lightning warhead's** AnimList anim (e.g. `EXPLOSION` family) — additive, not replacing the bolt. Both visuals coexist.

### Data Flow

```
Phase 4.5  Superweapon (Genetic Mutator / Lightning Storm)
              └─ apply_aoe_damage(...)           [unchanged]
              └─ helper(...)
                    ├─ ExplosionEffect → spawned via existing world_effects.push pattern
                    └─ SmudgeSpawnRequest::Anim → sim.pending_smudge_requests

Phase 5    Combat — per-shot block
              └─ "Fire one shot!"
                    ├─ if warhead.cell_spread > 0: apply_aoe_damage(...)
                    ├─ else: damage_events.push(...) [single-target]
                    ├─ helper(warhead, base_damage, target_rx, target_ry, z, ...)
                    │     ├─ explosion_effects.push(...)
                    │     └─ smudge_spawn_requests.push(...)
                    └─ destroy_ore_at_impact(...) [unchanged]

Phase 5    Combat — handle_entity_deaths
              └─ death_aoe loop
                    ├─ apply_aoe_damage(...) [unchanged]
                    ├─ helper(death_warhead, dmg, rx, ry, z, ...) [NEW]
                    └─ destroy_ore_at_impact(...) [unchanged]
              └─ killing-warhead block at lines 759-786 [REMOVED]

Post-combat drain (existing block at world/mod.rs:1271)
              └─ drain_smudge_spawn_requests(&sim.pending_smudge_requests, …)
              └─ drain_smudge_spawn_requests(&combat_result.smudge_spawn_requests, …)
              └─ sim.pending_smudge_requests.clear()
```

### Error Handling

- Empty `AnimList` → helper returns early. No error path.
- Warhead lookup failures stay where they are (callers all already `if let Some(warhead) = rules.warhead(...)`).
- No new fallible operations. No new `Result` types.

### Testing Strategy

Five targeted tests, all in existing test files:

1. **V3 non-killing AoE → emits one smudge request.** Set up a target with HP > V3 splash damage at edge. Tick combat. Assert `combat_result.smudge_spawn_requests` contains exactly one `SmudgeSpawnRequest::Anim` for the impact cell. (`combat_force_fire_cell_tests.rs` or extend `combat_tests.rs`.)
2. **V3 killing AoE → emits exactly ONE smudge request, not two.** Same setup with weak target; ensure no double-emission from the removed death-handler block.
3. **Empty-AnimList warhead → emits zero.** Build a warhead with `AnimList=` empty, fire at a target, assert `smudge_spawn_requests` is empty for that detonation.
4. **Death-weapon AoE → emits its own anim+smudge separately from the killing shot.** Build a Demo-Truck-style entity (`Explodes=yes`, primary warhead with `AnimList=`); kill it with a different weapon (different `AnimList`); assert two distinct `SmudgeSpawnRequest::Anim` entries with the two distinct anim names.
5. **Lightning Storm strike → emits anim+smudge from the lightning warhead.** Trigger one bolt; assert `sim.pending_smudge_requests` has one entry, and after `advance_tick` the smudge has been drained (RNG consumed deterministically).

## Architectural Decisions

- **Helper lives in `combat/mod.rs`** because both the request type and the AnimList-index math live there. Superweapon callsites already import from `crate::sim::combat::combat_aoe`; importing one more helper from the same module is a non-deviation.
- **Sim-owned `pending_smudge_requests`** mirrors the existing `combat_result.smudge_spawn_requests` collect-then-drain pattern. Avoids the alternatives (inline drain at the superweapon callsite, mutating the combat result struct from outside) which both break the established structure.
- **Drain ordering: superweapon first, then combat-shot.** Within a tick, superweapons run before combat (Phase 4.5 vs Phase 5), so emission order follows occurrence order. Drain order matches emission order, keeping the RNG cursor advancement intuitive.
- **Tech debt: rocket-projectile timing.** This change emits at fire time for projectile weapons, matching where damage already lands. The "emit at arrival, not fire" parity gap is **NOT** addressed here — fixing it requires a separate redesign of how rocket damage is applied. Documented as a known follow-up.
- **No deviation from any existing pattern.** Determinism, tick ordering, drain order, and the headless-tick path (`tick_ms == 0` early return) are all preserved.

## Alternatives Considered

- **Approach A — inline copy-paste at every site.** Same parity outcome, ~30 lines duplicated across 4 sites. Rejected for drift risk.
- **Approach C — embed in `apply_aoe_damage`.** Doesn't cover the single-target branch (CellSpread = 0); pollutes a pure damage function. Rejected.
- **Move only `SmudgeSpawnRequest::Anim`, leave `ExplosionEffect` on death.** Rejected — leaves the warhead's AnimList anim missing on V3 non-kill hits, a player-visible parity hole. The anim and the smudge are bound in gamemd's AnimClass::Start; we keep them bound here.
- **Defer rocket-projectile detonation to arrival time as part of this scope.** Rejected — couples combat smudge emission to movement; fixes a different parity gap that needs its own RE-grounded design (rocket damage timing).
- **Preserve current world hashes.** Rejected — would require feature-flagging the new emission, adding dead code for no parity benefit.

## Known Deferred Follow-up

Rocket-projectile detonation timing: V3, Dreadnought, Boomer-sub missiles, etc. apply damage AND now spawn anim/smudge at **fire** time. gamemd applies them at **arrival** time. Fixing this is a separate design — must rework rocket damage application to fire from `tick_rocket_movement`'s detonated-IDs list, with the rocket entity carrying its warhead/damage payload. Out of scope here.
