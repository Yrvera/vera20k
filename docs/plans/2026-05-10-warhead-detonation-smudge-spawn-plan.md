# Warhead-Detonation Smudge Spawn (Kill-Independent) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Spawn the warhead's AnimList animation and any Crater/Scorch-driven smudge on every warhead detonation, not only on the ones that kill an entity.

**Architecture:** Adds a single `combat::emit_warhead_detonation_effects` helper called from every detonation site (per-shot fire, death-AoE loop, Genetic Converter, Lightning Storm). Removes the kill-gated emission. Adds a sim-owned `pending_smudge_requests` vec for superweapon callsites that don't return through `CombatTickResult`, drained alongside the combat result in the existing post-combat block.

**Design Doc:** [docs/plans/2026-05-10-warhead-detonation-smudge-spawn-design.md](2026-05-10-warhead-detonation-smudge-spawn-design.md)

---

## Grounding Summary

- **Docs:** [SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md) is verified-from-binary. The 20-item parity ledger from §6 is already enforced inside [src/sim/combat/smudge_dispatch.rs](../../src/sim/combat/smudge_dispatch.rs) — altitude gate, 50/50 scorch/crater pick, `Reduce_Tiberium(6)`, ForceBigCraters path, the deterministic 256-entry unit-vector table.
- **Binary verification:** `AnimClass::Start @ 0x00424F00` runs once on first frame of an animation. Animations are spawned by `WarheadType::Detonate @ 0x004690B0`. Therefore every warhead detonation that emits an animation triggers the smudge logic — independent of kill outcome.
- **Repo pattern:** `combat::apply_aoe_damage` and `combat::destroy_ore_at_impact` are pure helpers called from both the per-shot block and the death-AoE loop. The new helper follows the same shape and lives in the same module.
- **INI keys:** `WarheadType.anim_list` already parsed at [rules/warhead_type.rs:46](../../src/rules/warhead_type.rs#L46). `ArtEntry { scorch, crater, force_big_craters }` already parsed at [rules/art_data.rs:32-34](../../src/rules/art_data.rs#L32-L34). No new INI parsing.
- **Unknowns after grounding:** none for this scope. Rocket-projectile arrival-time parity is a documented known follow-up (out of scope).

## Key Technical Decisions

- **Helper lives in `combat/mod.rs`** alongside `ExplosionEffect` and `SmudgeSpawnRequest`. Confidence: **high**. Source: existing repo pattern (`combat_aoe::apply_aoe_damage` is the closest analogue).
- **Vec-pushing helper signature**, not return-Option. Confidence: **high**. Source: brainstorm Q2; mirrors how `damage_events.push(...)` and `bridge_damage_events.push(...)` already flow in combat.
- **`pending_smudge_requests` is `#[serde(skip)]`**, like `world_effects` and `fire_events`. Confidence: **high**. Source: every other per-tick ephemeral vec in `Simulation` is `#[serde(skip)]`.
- **Drain order: `pending_smudge_requests` first, then `combat_result.smudge_spawn_requests`**. Confidence: **high**. Source: superweapons (Phase 4.5) emit before combat (Phase 5); preserving emission order as drain order keeps RNG consumption intuitive and deterministic.
- **AnimList index uses post-modifier `base_damage`** (the same value passed to `apply_aoe_damage`). Confidence: **high**. Source: existing kill-handler emission at [combat/mod.rs:770-771](../../src/sim/combat/mod.rs#L770-L771) uses post-modifier damage; we mirror it.
- **Z arg type is `u8`** (matches `ExplosionEffect.z`); the helper internally casts to `i32` for `SmudgeSpawnRequest::Anim`. Confidence: **high**. Source: identical conversion at [combat/mod.rs:783](../../src/sim/combat/mod.rs#L783).
- **Per-shot insertion point: after `destroy_ore_at_impact`, before `fire_sounds.push`** ([combat/mod.rs:1586-1594](../../src/sim/combat/mod.rs#L1586-L1594)). Confidence: **medium**. Source: any point inside the per-shot block works for correctness; this position groups all impact-cell side effects (damage → ore → anim+smudge) before the post-shot bookkeeping (sounds, fire events, burst updates). Flagged for `/review-plan` only if reviewer prefers an earlier insertion.

## Open Questions

### Resolved During Planning

- **Q: How to compute AnimList index when no `damage_events` entry exists (non-kill or cell-target)?** A: use the per-shot block's `base_damage` directly, the same value passed to `apply_aoe_damage`. The kill-path uses `(*dmg / 25)` from `damage_events`; the per-shot path uses `(base_damage / 25)`. Equivalent inputs.
- **Q: Does the helper need `&mut Simulation` for superweapon callsites?** A: no. Helper takes `&mut Vec<ExplosionEffect>` and `&mut Vec<SmudgeSpawnRequest>`. Superweapon callsites pass `&mut sim.world_effects`-via-adapter (or push WorldEffect inline like Lightning already does) and `&mut sim.pending_smudge_requests`.
- **Q: Z source for cell-target force-fire?** A: 0. The dispatcher re-derives `ground_z = terrain.cell(rx, ry).level * 15` and the altitude gate is `coord.z - ground_z < 30`; with `z=0` and any non-negative cell level, the gate evaluates to `0 - 15*level < 30` which is always satisfied (smudge attempts to land on the cell).

### Deferred to Implementation

- None. All design questions resolved.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/combat/mod.rs` | Add `emit_warhead_detonation_effects` helper; lift `explosion_effects`/`smudge_spawn_requests` vecs to `tick_combat_with_fog` scope; wire helper into per-shot block and death-AoE loop; remove kill-gated emission |
| Modify | `src/sim/world/mod.rs` | Add `Simulation::pending_smudge_requests` field; drain alongside combat-result smudges in post-combat block |
| Modify | `src/sim/superweapon/genetic_converter.rs` | Call helper at the Mutate AoE site |
| Modify | `src/sim/superweapon/lightning_storm.rs` | Call helper at the lightning-strike site |
| Modify | `src/sim/combat/combat_tests.rs` | Add tests for non-kill emit, no double, death-weapon distinct |
| Modify | `src/sim/superweapon/lightning_storm.rs` (tests) | Add lightning-emit test |

## Interface Changes

- **New `pub(crate)` function:** `combat::emit_warhead_detonation_effects(warhead: &WarheadType, base_damage: i32, rx: u16, ry: u16, z: u8, interner: &mut StringInterner, explosion_effects: &mut Vec<ExplosionEffect>, smudge_spawn_requests: &mut Vec<SmudgeSpawnRequest>)`. Internal to the crate; no external consumers.
- **New `Simulation` field:** `pub pending_smudge_requests: Vec<SmudgeSpawnRequest>` with `#[serde(skip)]`. Only mutated by superweapon callsites this session.
- **No changes** to `SmudgeSpawnRequest`, `ExplosionEffect`, `CombatTickResult`, or any external trait.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64 in game logic. **Helper uses only u8/u16/i32/InternedId; AnimList index is integer division.**
- [x] New state included in deterministic state hash. **`pending_smudge_requests` is per-tick ephemeral (drained every tick, never persists across tick boundaries) — not part of state hash, same as `world_effects`/`fire_events`.**
- [x] No dependencies on render/ui/sidebar/audio/net. **Helper and field both live in `sim/`; no new imports cross the boundary.**
- [x] Tick ordering impact noted. **Drain order: pending_smudge_requests first, then combat_result.smudge_spawn_requests, both inside the existing post-combat drain block at [world/mod.rs:1271-1289](../../src/sim/world/mod.rs#L1271-L1289). Order matches emission order (Phase 4.5 SW → Phase 5 combat).**
- [x] BTreeMap iteration order considered. **Helper does not iterate maps. Superweapon emission order follows existing tick_superweapons traversal (already deterministic).**

## Risk Areas

- **Double-emission on kill** (highest blast radius). If Task 4 doesn't fully remove the kill-handler block, every kill produces two `ExplosionEffect` + two `SmudgeSpawnRequest::Anim`. **Mitigation:** Task 4 explicitly tests "exactly one smudge request per V3 kill".
- **Determinism break vs. dev HEAD** (expected, accepted). World hash changes because the dispatcher's RNG advances on more detonations. Existing replays will desync. **Mitigation:** documented in design doc; user accepted.
- **Borrow conflicts** in the death-AoE loop and superweapon callsites where `interner.resolve(...)` is called inline. **Mitigation:** the existing pattern at [combat/mod.rs:760-786](../../src/sim/combat/mod.rs#L760-L786) already releases the resolve-borrow before calling `interner.intern(...)`; the helper follows the same shape.
- **Helper called with `wh.anim_list` empty** — must short-circuit before any RNG/interning. **Mitigation:** Task 1 unit tests cover the empty case; Task 5's empty-AnimList integration test covers it end-to-end.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | One emission per detonation; empty AnimList → zero emission | Anim+smudge are bound; gamemd's AnimClass::Start runs once per anim spawn. Two anims for one detonation = doubled smudges and doubled visual explosions. | Unit tests in helper module (basic emit, empty list, AnimList index = damage/25 clamped). |
| Task 2 | V3 non-killing AoE produces a smudge request | Player-visible: V3 strikes leave craters whether or not the splash kills. Currently zero smudges on non-kill — clear visible parity gap, fires every match. | Add test: low-damage V3 hit on full-HP target; assert `combat_result.smudge_spawn_requests` has 1 `Anim` entry at the impact cell. |
| Task 3 | Death-weapon AoE (Demo Truck, `Explodes=yes`) spawns its own AnimList anim+smudge separately from the killing shot | Player-visible: Demo Truck explosion craters every detonation. Currently the death-explosion's anim and smudge are entirely missing — the killing shot's anim shows but the demo's own UCEXPLOD doesn't. | Add test: kill an `Explodes=yes` entity with a different warhead; assert two distinct `SmudgeSpawnRequest::Anim` entries with two different `anim_name` interned IDs. |
| Task 4 | Killing V3 hit produces exactly ONE smudge request, not two | Player-visible: a kill must drop one crater, not two stacked craters. | Add test: high-damage V3 hit on low-HP target; assert exactly 1 `Anim` entry. |
| Task 5 | Drain order is deterministic across superweapon and combat emissions | Determinism + parity: out-of-order RNG consumption desyncs replays. | Implicit — drain order is fixed (pending first, combat second) and tested via Task 7 (Lightning emits, drain runs, smudge lands at expected cell). |
| Task 6 | Genetic Mutator detonation drops its AnimList anim+smudge | Player-visible: Mutator strike leaves a scorch/crater; currently doesn't. | Smoke test (compile + cargo check) — full visual verification deferred to in-game observation since Mutate has no test fixture today. |
| Task 7 | Lightning Storm bolt detonation drops its AnimList anim+smudge | Player-visible: every bolt leaves a scorch on the ground in gamemd; currently doesn't. | Add test: trigger one bolt; assert `pending_smudge_requests` has one `Anim` entry; advance one tick; assert smudge has been drained (request vec empty after drain). |

---

## Tasks

### Task 1: Add `emit_warhead_detonation_effects` helper to `combat/mod.rs`

**Why:** Define the contract first; pure logic with no callers; unit tests pin the AnimList index math + empty-list short-circuit before any wiring depends on it.

**Files:**
- Modify: `src/sim/combat/mod.rs` — add helper near `ExplosionEffect`/`SmudgeSpawnRequest` definitions (~line 555).

**Pattern:** Mirror `combat::destroy_ore_at_impact` (pure helper, takes `&mut` to caller-owned vecs, no return).

**Step 1: Add the helper function**

Insert immediately after the `SmudgeSpawnRequest` enum definition at [src/sim/combat/mod.rs:555](src/sim/combat/mod.rs#L555):

```rust
/// Emit the warhead's AnimList animation and a paired smudge spawn request
/// for one detonation at (rx, ry, z). Mirrors the gamemd dispatch where
/// WarheadType::Detonate spawns an animation from AnimList= and the
/// animation's first frame (AnimClass::Start) runs the smudge logic.
///
/// Pushes nothing if `warhead.anim_list` is empty.
///
/// `base_damage` is the post-modifier damage at the impact center; it
/// drives AnimList selection via `damage / 25`, clamped to `len - 1`.
pub(crate) fn emit_warhead_detonation_effects(
    warhead: &WarheadType,
    base_damage: i32,
    rx: u16,
    ry: u16,
    z: u8,
    interner: &mut StringInterner,
    explosion_effects: &mut Vec<ExplosionEffect>,
    smudge_spawn_requests: &mut Vec<SmudgeSpawnRequest>,
) {
    if warhead.anim_list.is_empty() {
        return;
    }
    let idx = (base_damage / ANIM_LIST_DAMAGE_STEP as i32).max(0) as usize;
    let idx = idx.min(warhead.anim_list.len() - 1);
    let interned_name = interner.intern(&warhead.anim_list[idx]);
    explosion_effects.push(ExplosionEffect {
        shp_name: interned_name,
        rx,
        ry,
        z,
    });
    smudge_spawn_requests.push(SmudgeSpawnRequest::Anim {
        anim_name: interned_name,
        rx,
        ry,
        z: z as i32,
    });
}
```

**Step 2: Promote `ANIM_LIST_DAMAGE_STEP` to module scope**

The constant currently lives inside `handle_entity_deaths` ([combat/mod.rs:660-661](src/sim/combat/mod.rs#L660-L661)). Move it to module scope so the helper can use it. Replace the in-function declaration with a reference; add at module scope near `GAME_FPS` ([combat/mod.rs:64-67](src/sim/combat/mod.rs#L64-L67)):

```rust
/// Step size for selecting explosion anim from a warhead's AnimList: idx = damage / 25.
const ANIM_LIST_DAMAGE_STEP: u16 = 25;
```

Delete the duplicate `const ANIM_LIST_DAMAGE_STEP: u16 = 25;` inside `handle_entity_deaths`.

**Step 3: Add unit tests**

Append to `src/sim/combat/combat_tests.rs` (or add a new `#[cfg(test)] mod helper_tests` block at the end of `combat/mod.rs` if `combat_tests.rs` is hard to extend in-place — check first):

```rust
#[cfg(test)]
mod emit_warhead_detonation_effects_tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::warhead_type::WarheadType;
    use crate::sim::intern::StringInterner;

    fn make_warhead_with_animlist(animlist: &[&str]) -> WarheadType {
        // WarheadType has no Default impl; build it the same way every other
        // test in warhead_type.rs does — via from_ini_section on inline INI.
        let animlist_csv = animlist.join(",");
        let ini_text = format!("[WH]\nAnimList={}\n", animlist_csv);
        let ini = IniFile::from_bytes(ini_text.as_bytes()).unwrap();
        WarheadType::from_ini_section("WH", ini.section("WH").unwrap())
    }

    #[test]
    fn empty_animlist_emits_nothing() {
        let mut interner = StringInterner::new();
        let wh = make_warhead_with_animlist(&[]);
        let mut explosions = Vec::new();
        let mut smudges = Vec::new();
        emit_warhead_detonation_effects(
            &wh, 100, 5, 5, 0, &mut interner, &mut explosions, &mut smudges,
        );
        assert!(explosions.is_empty());
        assert!(smudges.is_empty());
    }

    #[test]
    fn single_animlist_entry_emits_one_pair() {
        let mut interner = StringInterner::new();
        let wh = make_warhead_with_animlist(&["EXPLOSION1"]);
        let mut explosions = Vec::new();
        let mut smudges = Vec::new();
        emit_warhead_detonation_effects(
            &wh, 100, 5, 5, 0, &mut interner, &mut explosions, &mut smudges,
        );
        assert_eq!(explosions.len(), 1);
        assert_eq!(smudges.len(), 1);
        let expected_id = interner.intern("EXPLOSION1");
        assert_eq!(explosions[0].shp_name, expected_id);
        match &smudges[0] {
            SmudgeSpawnRequest::Anim { anim_name, rx, ry, z } => {
                assert_eq!(*anim_name, expected_id);
                assert_eq!(*rx, 5);
                assert_eq!(*ry, 5);
                assert_eq!(*z, 0);
            }
            other => panic!("expected Anim variant, got {:?}", other),
        }
    }

    #[test]
    fn animlist_index_is_damage_div_25_clamped() {
        let mut interner = StringInterner::new();
        let wh = make_warhead_with_animlist(&["EXP1", "EXP2", "EXP3"]);

        // damage=0 → idx=0
        let mut explosions = Vec::new();
        let mut smudges = Vec::new();
        emit_warhead_detonation_effects(
            &wh, 0, 0, 0, 0, &mut interner, &mut explosions, &mut smudges,
        );
        assert_eq!(explosions[0].shp_name, interner.intern("EXP1"));

        // damage=50 → idx=2 (50/25 = 2)
        let mut explosions = Vec::new();
        let mut smudges = Vec::new();
        emit_warhead_detonation_effects(
            &wh, 50, 0, 0, 0, &mut interner, &mut explosions, &mut smudges,
        );
        assert_eq!(explosions[0].shp_name, interner.intern("EXP3"));

        // damage=10000 → clamped to last (idx=2)
        let mut explosions = Vec::new();
        let mut smudges = Vec::new();
        emit_warhead_detonation_effects(
            &wh, 10000, 0, 0, 0, &mut interner, &mut explosions, &mut smudges,
        );
        assert_eq!(explosions[0].shp_name, interner.intern("EXP3"));
    }
}
```

**Step 4: Verify**

Run:
```
cargo test --lib emit_warhead_detonation_effects
```

Expected: 3 tests pass.

Run:
```
cargo build
```

Expected: clean build (no warnings about unused `ANIM_LIST_DAMAGE_STEP` after the move).

**Step 5: Commit**

```
combat: add emit_warhead_detonation_effects helper

Pure helper that pushes the warhead's AnimList anim and a paired
smudge spawn request for one detonation. Empty-AnimList warheads
short-circuit. AnimList index = damage / 25 clamped to len - 1.

Promotes ANIM_LIST_DAMAGE_STEP from handle_entity_deaths to module
scope so the helper can share it. No callers yet.
```

---

### Task 1.5: Lift `explosion_effects` and `smudge_spawn_requests` to `tick_combat_with_fog` scope

**Why:** The per-shot block (Task 2) needs to push into these vecs, but they currently only live inside `handle_entity_deaths` ([combat/mod.rs:655, 658](src/sim/combat/mod.rs#L655)). Without this scope-lift, Task 2 will not compile. Pure refactor — no behavioral change.

**Files:**
- Modify: `src/sim/combat/mod.rs` — add two `Vec` declarations at `tick_combat_with_fog` scope; `.extend()` death-handler contributions back into them; source `CombatTickResult` from the combat-scope vecs.

**Pattern:** Mirror `bridge_damage_events`/`wall_damage_events`, which are already declared at both scopes ([combat/mod.rs:656-657](src/sim/combat/mod.rs#L656-L657) inside `handle_entity_deaths`, [combat/mod.rs:1270-1271](src/sim/combat/mod.rs#L1270-L1271) inside `tick_combat_with_fog`) and `.extend()`ed at line 1738-1739:

```rust
bridge_damage_events.extend(death.bridge_damage_events);
wall_damage_events.extend(death.wall_damage_events);
```

We do exactly the same for the two new vecs.

**Step 1: Add vec declarations at `tick_combat_with_fog` scope**

Insert two lines alongside the other accumulators at [combat/mod.rs:1271](src/sim/combat/mod.rs#L1271) (right after the existing `wall_damage_events` declaration):

```rust
    let mut explosion_effects: Vec<ExplosionEffect> = Vec::new();
    let mut smudge_spawn_requests: Vec<SmudgeSpawnRequest> = Vec::new();
```

**Step 2: `.extend()` death-handler contributions into the combat-scope vecs**

Immediately after the existing two `.extend(death.bridge_damage_events / wall_damage_events)` calls at [combat/mod.rs:1738-1739](src/sim/combat/mod.rs#L1738-L1739), add:

```rust
    explosion_effects.extend(death.explosion_effects);
    smudge_spawn_requests.extend(death.smudge_spawn_requests);
```

**Step 3: Source `CombatTickResult` from the combat-scope vecs**

At [combat/mod.rs:1777-1778](src/sim/combat/mod.rs#L1777-L1778), change:

```rust
        explosion_effects: death.explosion_effects,
        smudge_spawn_requests: death.smudge_spawn_requests,
```

to:

```rust
        explosion_effects,
        smudge_spawn_requests,
```

(Note: this consumes `death.explosion_effects`/`death.smudge_spawn_requests` via the earlier `.extend()` calls — both fields of `DeathEffects` are owned `Vec`s, so `.extend(...)` consumes them.)

**Step 4: Verify**

Run:
```
cargo build
cargo test --lib combat
```

Expected: clean build; ALL existing combat tests pass with zero behavioral change. The two `Vec`s now live in `tick_combat_with_fog` and gather only the death-handler contributions (since no other emitter exists yet) — semantically identical to the previous `death.explosion_effects` / `death.smudge_spawn_requests` route.

**Step 5: Commit**

```
combat: lift explosion_effects/smudge_spawn_requests to tick_combat_with_fog scope

Pure refactor. Declares the two Vec accumulators at tick_combat_with_fog
scope (alongside bridge_damage_events/wall_damage_events) and .extend()s
the death handler's contributions into them. CombatTickResult now sources
from the combat-scope vecs. No behavioral change — the death handler
remains the only emitter; per-shot emission is wired in the next commit.
```

---

### Task 2: Wire helper into per-shot detonation site in `tick_combat_with_fog`

**Why:** Closes the V3-non-kill parity gap — every shot that detonates a warhead now emits its AnimList anim and may drop a smudge. Covers AoE branch, direct-hit branch, and Cell targets in one call.

**Files:**
- Modify: `src/sim/combat/mod.rs` — insert helper call inside the per-shot block at [combat/mod.rs:1583](src/sim/combat/mod.rs#L1583), right after `destroy_ore_at_impact(...)`.

**Pattern:** Mirror the existing post-shot side-effect cluster (ore destruction → fire sound → fire event → reveal-on-fire).

**Prerequisite:** Task 1.5 has lifted `explosion_effects` and `smudge_spawn_requests` to `tick_combat_with_fog` scope, so they are now reachable at line ~1583.

**Step 1: Resolve impact z**

Inside the per-shot block, just before the `destroy_ore_at_impact` call at [combat/mod.rs:1586](src/sim/combat/mod.rs#L1586), capture impact z. The block already uses `attack_impact_z(snap.target, entities)` for wall events — reuse it.

```rust
let impact_z: u8 = match snap.target {
    TargetKind::Entity(eid) => entities
        .get(eid)
        .map(|e| e.position.z)
        .unwrap_or(0),
    TargetKind::Cell(_, _) => 0,
};
```

Insert this declaration just before `destroy_ore_at_impact` at the per-shot block. (Note: `attack_impact_z` returns `i32`, but we need `u8` for the helper. The clean local computation above avoids casting and matches `entity.position.z`'s actual `u8` type — verified `Position.z: u8` at [components.rs:37](src/sim/components.rs#L37).)

**Step 2: Call the helper**

Immediately after `destroy_ore_at_impact(...)` at [combat/mod.rs:1586-1592](src/sim/combat/mod.rs#L1586-L1592), add:

```rust
emit_warhead_detonation_effects(
    warhead,
    base_damage,
    target_rx,
    target_ry,
    impact_z,
    interner,
    &mut explosion_effects,
    &mut smudge_spawn_requests,
);
```

After Task 1.5, `warhead` (loop-local at line 1500), `base_damage` (loop-local at line 1504-1510), `target_rx`/`target_ry` (snapshot fields), `interner` (function param), `explosion_effects` and `smudge_spawn_requests` (declared at line ~1272 by Task 1.5) are all in scope at this point.

**Step 3: Add the V3 non-killing AoE test**

Append to `src/sim/combat/combat_tests.rs`:

```rust
#[test]
fn v3_non_killing_aoe_emits_one_smudge_request() {
    // Setup: a target at (5, 5) with full HP, a V3-style warhead with
    // CellSpread=2 and AnimList=[V3EXP] firing from (10, 5). The splash
    // damage at the target is below the target's HP — target survives,
    // no death event, but warhead detonated → smudge must be emitted.
    let mut sim = test_sim_with_one_attacker_one_target(/* details below */);
    let result = combat::tick_combat_with_fog(
        &mut sim.entities,
        &mut sim.occupancy,
        &sim.rules,
        &mut sim.interner,
        None,
        &sim.power_states,
        None,
        &mut sim.production.resource_nodes,
        None,
        None,
        sim.tick,
        33,
    );
    assert!(
        sim.entities.get(target_id).map(|e| e.health.current > 0).unwrap_or(false),
        "target must survive (test setup invariant)"
    );
    assert_eq!(
        result.smudge_spawn_requests.len(), 1,
        "one detonation must emit one smudge request even on non-kill"
    );
    matches!(&result.smudge_spawn_requests[0], SmudgeSpawnRequest::Anim { .. });
}
```

The test scaffolding (RuleSet builder, attacker/target creation) should mirror the closest existing test in `combat_tests.rs` — search for the most recent V3 / cell-target / AoE test (the recent commit `e30a7ba` "combat: integration test — force-fire cell pursuit then fire" suggests fixtures already exist). If no V3-flavored fixture exists, mirror `test_prone_infantry_takes_scaled_aoe_damage` ([combat_tests.rs:435](src/sim/combat/combat_tests.rs#L435)) which already wires up an AoE warhead.

**Step 4: Verify**

Run:
```
cargo test --lib v3_non_killing_aoe_emits_one_smudge_request
cargo test --lib combat
```

Expected: new test passes; **no existing combat test regresses** except possibly tests that asserted "no smudge on non-kill" — none should exist (the bug was zero-emit, not asserted-zero). If a test fails because it asserted a specific `smudge_spawn_requests.len()` that's now off-by-one, update its expectation.

**Step 5: Commit**

```
combat: emit warhead AnimList anim + smudge at per-shot detonation

Calls emit_warhead_detonation_effects from inside the per-shot fire
block in tick_combat_with_fog, after destroy_ore_at_impact. Covers
AoE, direct-hit, and Cell-target detonations in one call. Test:
V3 non-killing AoE hit now emits one smudge request.

Note: the existing kill-handler emission is still in place; this
commit ALONE produces double emission on kills. Removed in the next
commit.
```

---

### Task 3: Wire helper into death-AoE loop in `handle_entity_deaths`

**Why:** Death-weapon detonations (Demo Truck, `Explodes=yes` units) currently emit zero anim/smudge — a Demo Truck explosion crater is missing every time. Closes the second parity gap.

**Files:**
- Modify: `src/sim/combat/mod.rs` — add helper call inside the death-AoE loop at [combat/mod.rs:853-901](src/sim/combat/mod.rs#L853-L901).

**Pattern:** Same as Task 2; this loop already has the analogous `apply_aoe_damage` + `destroy_ore_at_impact` cluster.

**Step 1: Add the helper call inside the loop body**

Inside the existing `for (rx, ry, z, dmg, wh_id, owner_id) in &death_aoe { ... }` loop, after the existing `destroy_ore_at_impact(...)` call at [combat/mod.rs:899](src/sim/combat/mod.rs#L899), and inside the `if let Some(warhead) = rules.warhead(...)` block (so we have a valid `&WarheadType`):

```rust
emit_warhead_detonation_effects(
    warhead,
    *dmg,
    *rx,
    *ry,
    *z,
    interner,
    &mut explosion_effects,
    &mut smudge_spawn_requests,
);
```

Variables: `warhead` is `&WarheadType` already bound in the `if let`; `*dmg` is `i32` from the death_aoe tuple; `*rx`, `*ry` are `u16`; `*z` is `u8`; `interner` is `&mut StringInterner` (already mutable in the function signature); `explosion_effects` and `smudge_spawn_requests` are the `Vec`s declared at the top of the function.

Verify before pasting: confirm the death_aoe tuple ordering matches the destructure pattern. Per [combat/mod.rs:650](src/sim/combat/mod.rs#L650), the type is `Vec<(u16, u16, u8, i32, InternedId, InternedId)>` — `(rx, ry, z, dmg, wh_id, owner_id)`. ✓

**Step 2: Add death-weapon-AoE distinct-emission test**

Append to `src/sim/combat/combat_tests.rs`:

```rust
#[test]
fn death_weapon_aoe_emits_separate_anim_from_killing_shot() {
    // Setup: an Explodes=yes entity with primary warhead "DEMO" (AnimList=[UCEXPLOD])
    // is killed by an attacker firing warhead "TANKHIT" (AnimList=[TANKEXP]).
    // Death-AoE loop must emit DEMO/UCEXPLOD; per-shot fire emits TANKHIT/TANKEXP.
    // Expected: two distinct SmudgeSpawnRequest::Anim entries with two distinct
    // anim_name interned IDs.
    let mut sim = test_sim_with_demo_truck_killed_by_tank(/* details below */);
    let result = combat::tick_combat_with_fog(
        &mut sim.entities,
        &mut sim.occupancy,
        &sim.rules,
        &mut sim.interner,
        None,
        &sim.power_states,
        None,
        &mut sim.production.resource_nodes,
        None,
        None,
        sim.tick,
        33,
    );

    let anim_names: Vec<crate::sim::intern::InternedId> = result
        .smudge_spawn_requests
        .iter()
        .filter_map(|req| match req {
            SmudgeSpawnRequest::Anim { anim_name, .. } => Some(*anim_name),
            _ => None,
        })
        .collect();

    assert_eq!(anim_names.len(), 2, "expected one anim from killing shot + one from death AoE");
    assert_ne!(anim_names[0], anim_names[1], "anim names must differ");

    let tankexp = sim.interner.intern("TANKEXP");
    let ucexplod = sim.interner.intern("UCEXPLOD");
    assert!(anim_names.contains(&tankexp));
    assert!(anim_names.contains(&ucexplod));
}
```

Build the fixture to give the demo-truck-style entity `explodes: true` and a primary weapon whose warhead has `anim_list=["UCEXPLOD"]`, `cell_spread > 0`, and enough `damage` to AoE-spread. The attacker's warhead has `anim_list=["TANKEXP"]`. Use damage values that ensure both detonations fire and the Demo dies on the first hit.

**Step 3: Verify**

Run:
```
cargo test --lib death_weapon_aoe_emits_separate_anim_from_killing_shot
cargo test --lib combat
```

Expected: new test passes.

**Important:** at this point, killing a target with a single shot will produce DOUBLE emission (per-shot + kill-handler). This is expected and fixed in Task 4. Any test that already asserts `smudge_spawn_requests.len() == 1` for a kill scenario will fail temporarily — leave those to Task 4 to update.

**Step 5: Commit**

```
combat: emit warhead AnimList anim + smudge in death-AoE loop

Calls emit_warhead_detonation_effects inside handle_entity_deaths'
death_aoe loop (Explodes=yes / DeathWeapon detonations). Test:
killing a Demo-Truck-style entity emits two distinct anim names —
the killing shot's and the demo's own death-explosion's.

Double-emission on kill is expected after this commit (per-shot +
kill-handler). Fixed in the next commit by removing the kill-handler
emission.
```

---

### Task 4: Remove the kill-gated emission in `handle_entity_deaths`

**Why:** The per-shot block (Task 2) now emits the killing shot's anim+smudge, so the kill-gated block at [combat/mod.rs:759-786](src/sim/combat/mod.rs#L759-L786) is redundant and produces double-emission on every kill.

**Files:**
- Modify: `src/sim/combat/mod.rs` — delete the kill-gated emission block.

**Pattern:** Pure deletion. The surrounding code (sound events, garrison/crewed building snapshots, building destruction smudges, occupancy cleanup, animation switching) all stay.

**Step 1: Locate and delete**

At [combat/mod.rs:759-786](src/sim/combat/mod.rs#L759-L786), the block to remove is:

```rust
            // Look up the warhead that dealt the killing blow for explosion selection.
            let killing_warhead = damage_events
                .iter()
                .rfind(|(tid, _, _, _)| *tid == dead_id)
                .and_then(|(_, dmg, _, wh_id)| {
                    rules.warhead(interner.resolve(*wh_id)).map(|wh| (wh, *dmg))
                });

            // Spawn explosion animation from the warhead's AnimList.
            if let Some((wh, dmg)) = &killing_warhead {
                if !wh.anim_list.is_empty() {
                    let idx = (*dmg / ANIM_LIST_DAMAGE_STEP) as usize;
                    let idx = idx.min(wh.anim_list.len() - 1);
                    let interned_name = interner.intern(&wh.anim_list[idx]);
                    explosion_effects.push(ExplosionEffect {
                        shp_name: interned_name,
                        rx,
                        ry,
                        z,
                    });
                    smudge_spawn_requests.push(SmudgeSpawnRequest::Anim {
                        anim_name: interned_name,
                        rx,
                        ry,
                        z: z as i32,
                    });
                }
            }
```

**Critical:** the `killing_warhead` lookup is also used downstream at [combat/mod.rs:821-823](src/sim/combat/mod.rs#L821-L823) for `inf_death` selection:

```rust
let inf_death: u8 = killing_warhead
    .as_ref()
    .map(|(wh, _)| wh.inf_death)
    .unwrap_or(1);
```

So we cannot delete the `killing_warhead` lookup itself — only the `if let Some((wh, dmg)) = &killing_warhead { ... explosion_effects.push / smudge_spawn_requests.push ... }` emission block. Keep the `let killing_warhead = ...` binding.

After the edit, the lookup-only code remains and the emission is gone:

```rust
            // Look up the warhead that dealt the killing blow for InfDeath selection.
            let killing_warhead = damage_events
                .iter()
                .rfind(|(tid, _, _, _)| *tid == dead_id)
                .and_then(|(_, dmg, _, wh_id)| {
                    rules.warhead(interner.resolve(*wh_id)).map(|wh| (wh, *dmg))
                });

            // (warhead-detonation anim + smudge are emitted at the per-shot fire
            // site and at the death-AoE loop; this block no longer emits them.)
```

Update the comment from "for explosion selection" to "for InfDeath selection" since that's all it drives now. Drop the trailing parenthetical comment if it would be the only addition — per the project's "no comments unless they explain WHY" rule, the lookup is self-evidently named for `killing_warhead`. Just keep:

```rust
            // killing_warhead is read below for InfDeath selection only.
            let killing_warhead = damage_events
                .iter()
                .rfind(|(tid, _, _, _)| *tid == dead_id)
                .and_then(|(_, dmg, _, wh_id)| {
                    rules.warhead(interner.resolve(*wh_id)).map(|wh| (wh, *dmg))
                });
```

**Step 2: Verify the lookup-only `killing_warhead` doesn't trigger an unused-variable warning**

If `inf_death` is the only consumer and is conditionally compiled or behind a flag, the compiler may warn. Verify by running `cargo build` after the edit. If a warning appears, narrow `killing_warhead` to just `inf_death`:

```rust
let inf_death: u8 = damage_events
    .iter()
    .rfind(|(tid, _, _, _)| *tid == dead_id)
    .and_then(|(_, _dmg, _, wh_id)| {
        rules.warhead(interner.resolve(*wh_id)).map(|wh| wh.inf_death)
    })
    .unwrap_or(1);
```

…and delete the now-unused `killing_warhead` binding entirely. Apply this simplification only if the build complains.

**Step 3: Add the no-double-emit test**

Append to `src/sim/combat/combat_tests.rs`:

```rust
#[test]
fn v3_killing_aoe_emits_exactly_one_smudge_request() {
    // V3 hits a low-HP target → AoE damage > target HP → kill.
    // Even on kill, the warhead detonates ONCE → one anim → one smudge.
    let mut sim = test_sim_with_low_hp_target_killed_by_v3(/* details below */);
    let result = combat::tick_combat_with_fog(
        &mut sim.entities,
        &mut sim.occupancy,
        &sim.rules,
        &mut sim.interner,
        None,
        &sim.power_states,
        None,
        &mut sim.production.resource_nodes,
        None,
        None,
        sim.tick,
        33,
    );
    assert_eq!(
        result.despawned_ids.len(), 1,
        "target must die (test setup invariant)"
    );
    let anim_count = result
        .smudge_spawn_requests
        .iter()
        .filter(|r| matches!(r, SmudgeSpawnRequest::Anim { .. }))
        .count();
    assert_eq!(anim_count, 1, "kill must emit exactly one anim smudge, not two");
}
```

Build the fixture identically to Task 2 but with target HP < AoE damage at center.

**Step 4: Verify**

Run:
```
cargo test --lib v3_killing_aoe_emits_exactly_one_smudge_request
cargo test --lib v3_non_killing_aoe_emits_one_smudge_request
cargo test --lib death_weapon_aoe_emits_separate_anim_from_killing_shot
cargo test --lib combat
cargo build
```

Expected: all three smudge tests pass; combat regression suite passes; clean build.

**Step 5: Commit**

```
combat: remove kill-gated AnimList anim/smudge emission

The per-shot fire block and the death-AoE loop now emit warhead
AnimList anims + smudges, so the kill-gated emission in
handle_entity_deaths was double-emitting on every kill. Removes
just the explosion_effects.push / smudge_spawn_requests.push
block; keeps the killing_warhead lookup (still needed for
InfDeath selection).

Test: V3 killing AoE now emits exactly one anim smudge, not two.
```

---

### Task 5: Add `Simulation::pending_smudge_requests` field and drain wiring

**Why:** Superweapon callsites (Genetic Converter, Lightning Storm) emit smudges outside `CombatTickResult`. They need a sim-owned vec for the same collect-then-drain pattern.

**Files:**
- Modify: `src/sim/world/mod.rs` — add field to `Simulation` struct; initialize in `with_seed`; drain in post-combat block.

**Pattern:** Mirror `Simulation::world_effects` and `Simulation::fire_events` — `#[serde(skip)]`, initialized to empty Vec, owner-of-truth for ephemeral per-tick data.

**Step 1: Add the field**

In the `Simulation` struct near the other `#[serde(skip)]` per-tick vecs (e.g. after `fire_events` at [world/mod.rs:214](src/sim/world/mod.rs#L214)):

```rust
    /// Smudge spawn requests emitted by callsites that don't return through
    /// CombatTickResult (superweapons, etc.). Drained alongside combat-emitted
    /// smudge requests in the post-combat drain block. Ephemeral — never
    /// persists across ticks.
    #[serde(skip)]
    pub pending_smudge_requests: Vec<crate::sim::combat::SmudgeSpawnRequest>,
```

**Step 2: Initialize in `Simulation::with_seed`**

In the `Self { ... }` literal inside `with_seed` at [world/mod.rs:341-386](src/sim/world/mod.rs#L341-L386), add:

```rust
            pending_smudge_requests: Vec::new(),
```

(Sorted alphabetically with neighboring fields, or grouped with other Vec fields — match the surrounding style.)

**Step 3: Wire drain into the post-combat block**

In the existing drain block at [world/mod.rs:1271-1289](src/sim/world/mod.rs#L1271-L1289), drain `pending_smudge_requests` BEFORE `combat_result.smudge_spawn_requests`, then clear:

```rust
            if let (Some(smudge_grid), Some(overlay), Some(terrain), Some(pg)) = (
                self.smudge_grid.as_mut(),
                self.overlay_grid.as_ref(),
                self.resolved_terrain.as_ref(),
                path_grid,
            ) {
                // Superweapon-emitted smudges first (Phase 4.5 emissions).
                crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
                    &self.pending_smudge_requests,
                    &rules.art_registry,
                    &rules.smudge_types,
                    &self.interner,
                    smudge_grid,
                    overlay,
                    &self.occupancy,
                    terrain,
                    pg,
                    &mut self.production.resource_nodes,
                    &mut self.rng,
                );
                // Combat-emitted smudges (Phase 5 emissions).
                crate::sim::combat::smudge_dispatch::drain_smudge_spawn_requests(
                    &combat_result.smudge_spawn_requests,
                    &rules.art_registry,
                    &rules.smudge_types,
                    &self.interner,
                    smudge_grid,
                    overlay,
                    &self.occupancy,
                    terrain,
                    pg,
                    &mut self.production.resource_nodes,
                    &mut self.rng,
                );
            }
            self.pending_smudge_requests.clear();
```

The `clear()` is unconditional (outside the `if let`) so the vec doesn't accumulate when grids are unbound (headless tests).

**Step 4: Verify**

Run:
```
cargo build
cargo test --lib
```

Expected: clean build; all existing tests pass (no behavioral change yet — vec is unused by emitters).

**Step 5: Commit**

```
sim/world: add pending_smudge_requests + drain in post-combat block

Sim-owned vec for smudge requests emitted outside CombatTickResult
(superweapons). Drained before combat_result.smudge_spawn_requests
in the existing post-combat drain block, then cleared. Ephemeral
per-tick state, #[serde(skip)] like world_effects/fire_events.

No emitters yet — vec stays empty until the superweapon callsites
are wired in the next commits.
```

---

### Task 6: Wire helper into Genetic Converter Mutate AoE

**Why:** Mutate Warhead detonates → should spawn its AnimList anim and may drop a smudge. Currently emits zero.

**Files:**
- Modify: `src/sim/superweapon/genetic_converter.rs` — call helper at the AoE site.

**Pattern:** Mirror the Lightning Storm callsite (next task) — both push directly to `sim.world_effects` for the explosion sprite (since they don't return through `CombatTickResult`) and push to `sim.pending_smudge_requests` for the smudge.

**Step 1: Inspect the existing callsite**

Read [src/sim/superweapon/genetic_converter.rs:74-126](src/sim/superweapon/genetic_converter.rs#L74-L126) to confirm the surrounding code shape — specifically that `warhead: &WarheadType` is bound in the same scope (it is, via `rules.warhead(&warhead_id)`).

**Step 2: Add the emission**

After the `apply_aoe_damage` block but before the per-hit loop (around [genetic_converter.rs:101](src/sim/superweapon/genetic_converter.rs#L101)), build a local `Vec<ExplosionEffect>` for the helper, call the helper, then push each ExplosionEffect into `sim.world_effects` with the established WorldEffect pattern.

```rust
    // Emit warhead AnimList anim + smudge for this detonation, kill-independent.
    let mut explosions: Vec<crate::sim::combat::ExplosionEffect> = Vec::new();
    crate::sim::combat::emit_warhead_detonation_effects(
        warhead,
        base_damage,
        target_rx,
        target_ry,
        0, // ground impact
        &mut sim.interner,
        &mut explosions,
        &mut sim.pending_smudge_requests,
    );
    for fx in &explosions {
        let frames = sim
            .effect_frame_counts
            .get(&fx.shp_name)
            .copied()
            .unwrap_or(20);
        sim.world_effects.push(crate::sim::components::WorldEffect {
            shp_name: fx.shp_name,
            rx: fx.rx,
            ry: fx.ry,
            z: fx.z,
            frame: 0,
            total_frames: frames,
            rate_ms: 67,
            elapsed_ms: 0,
            translucent: true,
            delay_ms: 0,
        });
    }
```

The WorldEffect literal mirrors the existing pattern at [world/mod.rs:1245-1256](src/sim/world/mod.rs#L1245-L1256) (rate_ms=67, translucent=true, frame=0).

**Step 3: Verify**

Run:
```
cargo build
cargo test --lib genetic_converter
cargo test --lib
```

Expected: clean build; no regressions in genetic_converter tests; full suite green.

(No new dedicated test for Genetic Converter — see plan section "Parity-Critical Items" Task 7 row: full visual verification deferred to in-game observation since no Mutate test fixture exists today, and the helper itself is already unit-tested in Task 1. The compile + suite-green check is the bar for this task.)

**Step 4: Commit**

```
superweapon/genetic_converter: emit AnimList anim + smudge on Mutate AoE

Genetic Mutator detonation now spawns the mutate warhead's AnimList
animation and drops any Crater/Scorch-driven smudge. Routes through
sim.world_effects (anim) and sim.pending_smudge_requests (smudge).

No prior anim/smudge was emitted at this site; this closes a parity
gap visible every time the superweapon fires.
```

---

### Task 7: Wire helper into Lightning Storm bolt AoE

**Why:** Lightning bolts detonate the lightning warhead → should spawn its AnimList anim and may drop a smudge. Currently the bolt sprite is emitted but the warhead's AnimList anim and smudge are missing.

**Files:**
- Modify: `src/sim/superweapon/lightning_storm.rs` — call helper at the AoE site (around [lightning_storm.rs:236-257](src/sim/superweapon/lightning_storm.rs#L236-L257)).

**Pattern:** Identical to Task 6.

**Step 1: Add the emission**

After the existing `if let Some(warhead) = rules.warhead(warhead_id) { ... }` block at [lightning_storm.rs:236-257](src/sim/superweapon/lightning_storm.rs#L236-L257), inside the same `if let` so we have a valid `&WarheadType`, after the per-hit damage application:

```rust
        // Emit warhead AnimList anim + smudge for this bolt detonation,
        // kill-independent. The bolt visual at line ~221 is the strike
        // animation; this is the warhead's AnimList anim (e.g. EXPLOSION).
        let mut explosions: Vec<crate::sim::combat::ExplosionEffect> = Vec::new();
        crate::sim::combat::emit_warhead_detonation_effects(
            warhead,
            rules.general.lightning_damage,
            rx,
            ry,
            0,
            &mut sim.interner,
            &mut explosions,
            &mut sim.pending_smudge_requests,
        );
        for fx in &explosions {
            let frames = sim
                .effect_frame_counts
                .get(&fx.shp_name)
                .copied()
                .unwrap_or(20);
            sim.world_effects.push(crate::sim::components::WorldEffect {
                shp_name: fx.shp_name,
                rx: fx.rx,
                ry: fx.ry,
                z: fx.z,
                frame: 0,
                total_frames: frames,
                rate_ms: 67,
                elapsed_ms: 0,
                translucent: true,
                delay_ms: 0,
            });
        }
```

`rx`, `ry` are the bolt cell coordinates already in scope (per the existing `sim.world_effects.push(WorldEffect { ... rx, ry, ... })` at [lightning_storm.rs:221-232](src/sim/superweapon/lightning_storm.rs#L221-L232)).

**Step 2: Add the lightning-emit test**

Add to `src/sim/superweapon/lightning_storm.rs` (or its existing test file if separate — search for `#[cfg(test)] mod tests` in that file first):

```rust
#[cfg(test)]
mod warhead_detonation_emit_tests {
    use super::*;
    // ... fixture imports

    #[test]
    fn lightning_strike_emits_anim_smudge_into_pending_requests() {
        let (mut sim, rules) = test_sim_with_lightning_warhead("EXPLOSION");
        let strike_rx = 5;
        let strike_ry = 5;
        // Trigger one bolt at (5, 5) — implementation: call the
        // bolt-strike entry point in lightning_storm.rs directly with
        // a dummy owner. Mirror the closest existing lightning test
        // for fixture shape.
        fire_one_bolt_at(&mut sim, &rules, strike_rx, strike_ry, /* owner */);

        let anim_smudges: Vec<_> = sim
            .pending_smudge_requests
            .iter()
            .filter(|r| matches!(r, crate::sim::combat::SmudgeSpawnRequest::Anim { .. }))
            .collect();
        assert_eq!(anim_smudges.len(), 1, "one bolt → one anim smudge");

        // Verify the explosion anim was also pushed to world_effects.
        assert!(
            sim.world_effects
                .iter()
                .any(|fx| fx.shp_name == sim.interner.intern("EXPLOSION")
                    && fx.rx == strike_rx && fx.ry == strike_ry),
            "warhead AnimList anim must be in world_effects"
        );
    }
}
```

Build the fixture by setting `rules.general.lightning_warhead` to a warhead with `anim_list = vec!["EXPLOSION".into()]`. Search for existing lightning tests in the file for the fixture shape — the test should follow whatever pattern is already there (e.g. `LightningStormState::default()` + a hand-rolled `RuleSet`).

**Step 3: Verify**

Run:
```
cargo test --lib lightning_strike_emits_anim_smudge_into_pending_requests
cargo test --lib lightning_storm
cargo test --lib
cargo build
```

Expected: new test passes; lightning_storm regression suite passes; full suite green; clean build.

**Step 4: Commit**

```
superweapon/lightning_storm: emit AnimList anim + smudge on bolt AoE

Each lightning bolt now spawns the lightning warhead's AnimList
animation (in addition to the bolt strike visual) and drops any
Crater/Scorch-driven smudge. Routes through sim.world_effects
(anim) and sim.pending_smudge_requests (smudge).

Test: one bolt produces one anim entry in pending_smudge_requests
and one matching WorldEffect.
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-10-warhead-detonation-smudge-spawn-design.md](2026-05-10-warhead-detonation-smudge-spawn-design.md)
- **Ghidra reports:**
  - [SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md) — primary source; verified-from-binary, full 20-item parity ledger.
- **gamemd.exe addresses:**
  - `AnimClass::Start @ 0x00424F00` — first-frame smudge handler
  - `WarheadType::Detonate @ 0x004690B0` — anim-spawn trigger
  - `BuildingClass::DestructionEffects @ 0x004415F0` — building-center smudge (already implemented; not touched by this plan)
  - `BuildingClass::SpawnSurvivors @ 0x00442D90` — building-survivor smudge (already implemented; not touched)
- **INI keys driving behavior:**
  - `rulesmd.ini` `[<WarheadType>] AnimList=` → `WarheadType.anim_list`
  - `artmd.ini` `[<AnimType>] Scorch=`, `Crater=`, `ForceBigCraters=` → `ArtEntry.{scorch,crater,force_big_craters}`
- **Related code:**
  - [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — `tick_combat_with_fog`, `handle_entity_deaths`, `SmudgeSpawnRequest`, `ExplosionEffect`
  - [src/sim/combat/smudge_dispatch.rs](../../src/sim/combat/smudge_dispatch.rs) — dispatcher (no changes; already correct)
  - [src/rules/warhead_type.rs:46](../../src/rules/warhead_type.rs#L46) — `anim_list: Vec<String>`
  - [src/rules/art_data.rs:32-34](../../src/rules/art_data.rs#L32-L34) — `scorch/crater/force_big_craters` bools
- **Recent commits informing the plan:**
  - 93f4b78 (combat: smudge dispatch reads frame dims from ArtEntry instead of fixed default)
  - 103fc5d (app_init: eagerly populate ArtEntry frame dims after merge_art_data)
  - f906c63 (rules: ArtRegistry::populate_anim_frame_dims for smudge-flagged anims)
  - 1dd674d (rules: add frame_width/frame_height fields to ArtEntry)
  - e30a7ba (combat: integration test — force-fire cell pursuit then fire) — establishes V3 force-fire test fixtures
