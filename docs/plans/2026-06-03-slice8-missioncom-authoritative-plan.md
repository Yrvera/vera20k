# Slice 8 — Make MissionCom Authoritative + Global Parity Harness — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Commit after each on `dev` (no branch/PR/push). `docs/` is gitignored — no commit step for this plan.

**Goal:** Promote `MissionCom` from an unhashed shadow to hashed canonical state (the one sanctioned hash change), drop the Slice-2 shadow asserts, and add the global lockstep parity harness — all behavior-neutral, no field deletions.

**Architecture:** `MissionCom` is the always-present `mission` field on `GameEntity` ([game_entity.rs:497](../../src/sim/game_entity.rs)). `refresh_mission_shadow()` re-derives `current`/`substate` from the authoritative `Option<T>` machines each tick tail ([world/mod.rs:2445](../../src/sim/world/mod.rs)), *before* `state_hash()` (2454). Slice 8 folds `mission` into `hash_entities`, keeps `refresh_mission_shadow` as the deterministic projection writer, and deletes only the `debug_assert_mission_shadow_consistent` cross-check.

**Design Doc:** the approved brainstorm in this session (no separate `-design.md` written; design captured here).

---

## Grounding Summary

- **Docs:** `docs/plans/2026-06-01-mission-radio-substrate-implementation-plan.md` §Slice 8 + the V5 selector-retirement map. Two of its assumptions are **stale against current code** and corrected here (see Key Technical Decisions).
- **Code verified this session:** Slices 2 & 6 landed (`MissionCom` at [mission/mod.rs:188](../../src/sim/mission/mod.rs); field renamed to `mission` + un-skipped/serialized; full verb API in [mission/verb.rs](../../src/sim/mission/verb.rs) + [retask.rs](../../src/sim/mission/retask.rs)). `hash_entities` does **not** fold `mission` yet ([world_hash.rs:382](../../src/sim/world/world_hash.rs)). `refresh_mission_shadow` runs at 2445 < `state_hash()` 2454.
- **Repo pattern mirrored:** the explicit per-field fold idiom in `hash_entities` (`(x as u8)`, Option presence-tag) + the `hash_drive_track_state` free-fn helper; test patterns from [world_hash.rs](../../src/sim/world/world_hash.rs) `radio_contact_hash_tests` and [slice6_retask_tests.rs](../../src/sim/world/slice6_retask_tests.rs).
- **No INI keys** drive this slice (pure determinism/serialization work).
- **Seed hook:** `Simulation::with_seed(seed)` ([world/mod.rs:510](../../src/sim/world/mod.rs)) for the harness `HARNESS_SEED`.
- **Still unknown (deferred to impl):** the exact `GLOBAL_HARNESS_FINAL_HASH` and the new `SLICE6_BASELINE_HASH` values (captured from first green run); the exact harness entity composition that spawns + runs deterministically for 600 ticks.

## Key Technical Decisions

- **"Authoritative" = hashed canonical state, NOT sole decision source.** Keep `refresh_mission_shadow` (deterministic projection of the authoritative `Option<T>` machines); fold `mission` into the hash; delete only the asserts. Rewriting all readers to consume `mission` is a large behavioral change, out of scope. — **Confidence:** high — **Source:** approved design + [world/mod.rs:2445](../../src/sim/world/mod.rs).
- **`order_intent` is KEPT** (V5 map row corrected). It is load-bearing substate (sole store of AttackMove goal / Guard anchor coords; the `Unloading` transport-unload flag; the retaliation gate `is_busy` provably cannot replace). No fields are deleted in Slice 8. — **Confidence:** high — **Source:** [components.rs:490](../../src/sim/components.rs), [world_orders.rs:87](../../src/sim/world/world_orders.rs), [passenger.rs](../../src/sim/passenger.rs), [combat_targeting.rs:346](../../src/sim/combat/combat_targeting.rs); user decision this session.
- **The `dock_reservations` hash fold is KEPT (design Task 1b DROPPED).** `RefineryDockContacts` is a *live transitional mirror*, not dead: `hello_or_wait`/`try_reserve` push `.contacts`, `link_on_pad` `.on_pad`, `mark_contact_entered` `.contact_entered` ([miner_dock.rs:47-82](../../src/sim/miner/miner_dock.rs)), all called by the production dock path ([miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) 591–988/1221, [miner_system.rs](../../src/sim/miner/miner_system.rs) 168/701/938). Deleting its fold would change the hash whenever a miner docks and blind the desync detector. Its own doc says "retired in a *later* slice." — **Confidence:** high — **Source:** verified this session. **This makes the MissionCom fold the SOLE hash change in Slice 8 — exactly the contract.**
- **`SNAPSHOT_VERSION` bumped 16→17** for documentation only (MissionCom already serializes since Slice 6 → bincode layout unchanged; only the hash changes). — **Confidence:** high.
- **Only `SLICE6_BASELINE_HASH` re-baselines** (the lone golden constant; verified by grep). `round_trip_preserves_state_hash`/`determinism_replay` are relative tests — self-pass after the fold. — **Confidence:** high.

## Open Questions

### Resolved During Planning
- Is `dock_reservations` dead? **No** — live transitional mirror; keep its hashing (above).
- Other golden-hash constants beyond `SLICE6_BASELINE_HASH`? **None** (grep-verified).
- Does the shadow assert still pass between fold (Task 1) and its deletion (Task 3)? **Yes** — `refresh_mission_shadow` sets `current`/`substate` = derived right before the assert; debug builds pass.

### Deferred to Implementation
- `GLOBAL_HARNESS_FINAL_HASH` and the new `SLICE6_BASELINE_HASH` numeric values — captured from the first green run (cannot compute without running).
- Final harness entity composition — finalized against what spawns + advances deterministically for 600 ticks without panic. A panic mid-run is a real bug to surface, not to paper over.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/world_hash.rs` | Add `hash_mission_com` helper + call it in `hash_entities` |
| Modify | `src/sim/world/mod.rs` | Delete `debug_assert_mission_shadow_consistent` (def + call); update `refresh_mission_shadow` doc; wire 2 new test mods |
| Modify | `src/sim/world/world_tests.rs` | Invert `mission_shadow_does_not_change_state_hash` |
| Modify | `src/sim/snapshot.rs` | Bump `SNAPSHOT_VERSION` 16→17 |
| Modify | `src/sim/world/slice6_retask_tests.rs` | Re-baseline `SLICE6_BASELINE_HASH` (same commit as the fold) |
| Create | `src/sim/world/mission_authoritative_tests.rs` | `mission`-folds-into-hash unit tests |
| Create | `src/sim/world/global_parity_harness_tests.rs` | Global lockstep replay harness + `GLOBAL_HARNESS_FINAL_HASH` |

## Interface Changes
None public. `hash_mission_com` is a private free fn in `world_hash.rs`. No `GameEntity` field or `MissionCom` shape change (it already derives the needed traits; `Hash` is **not** added — explicit fold per file convention).

## Sim Checklist
- [x] All math integer/`u32` — no f32/f64.
- [x] New hashed state: `mission` folded into `state_hash` (the intended change).
- [x] No deps on render/ui/sidebar/audio/net.
- [x] Tick ordering unchanged (`refresh_mission_shadow` stays at 2445, before `state_hash`).
- [x] BTreeMap iteration: `hash_entities` already walks `entities.values()` (stable id order).

## Risk Areas
- **Re-baseline discipline:** `SLICE6_BASELINE_HASH` shifts once, in the fold commit, with a one-line reason. If *any other* test with a committed hash breaks, STOP — it means the fold changed more than intended (it shouldn't; grep found only one).
- **Harness flakiness/determinism:** the 600-tick replay must be bit-reproducible. Use `with_seed`; `sim/` never reads the clock. If record vs replay hashes diverge, that's a real non-determinism bug — investigate, don't loosen the assert.
- **Debug-only assert deletion** has zero release-hash effect (compiled out of release).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | MissionCom fold encoding/order (`current as u16`, Option presence-tag, timer `start_frame` then `duration`, `tick_counter`) | Lockstep hash pre-image must be identical across clients; a wrong encoding desyncs the detector | Determinism: harness intra-run record==replay; `mission_authoritative_tests` pin each field affects the hash |
| 1 | `dock_reservations` fold **retained** | Live dock state must stay in the hash or desync goes blind mid-dock | Confirmed live writers; full regression green |
| 1 | Refresh-before-hash ordering preserved (2445<2454) | Folded `mission` must reflect *this* tick | Do not move `refresh_mission_shadow`; ordering unchanged |
| 6 | Harness run through real `ReplayLog`/`ReplayRunner::run` | Regression guard must exercise the production replay path, not a bespoke loop | `global_skirmish_replay_is_deterministic_and_baseline_stable` |

---

## Tasks

### Task 1: Fold `MissionCom` into `state_hash` + re-baseline `SLICE6_BASELINE_HASH`

**Why:** The headline sanctioned change — promote `mission` to hashed canonical state. Re-baseline the lone golden constant in the same commit (contract).

**Files:**
- Modify: `src/sim/world/world_hash.rs` (helper near line 13; call near line 622)
- Modify: `src/sim/world/slice6_retask_tests.rs:75`

**Pattern:** mirrors `hash_drive_track_state` (free-fn helper) + the explicit per-field fold idiom in `hash_entities`.

**Step 1: Add the helper** — in `world_hash.rs`, after `hash_drive_track_state` (ends ~line 26), add:
```rust
/// Fold the `MissionCom` mission component into the state hash.
///
/// Explicit field fold (MissionCom intentionally does NOT derive `Hash`): enum
/// discriminants cast to `u16` (matching the `category as u8` idiom in
/// `hash_entities`), `Option`s as a `0u8`/`1u8` presence tag plus value. As of
/// Slice 8 `mission` is canonical hashed lockstep state, no longer an unhashed
/// shadow; `refresh_mission_shadow` keeps `current`/`substate` a deterministic
/// projection of the authoritative machines, so this fold cannot desync.
fn hash_mission_com(mission: &crate::sim::mission::MissionCom, hasher: &mut impl Hasher) {
    (mission.current as u16).hash(hasher);
    match mission.queued {
        Some(m) => {
            1u8.hash(hasher);
            (m as u16).hash(hasher);
        }
        None => 0u8.hash(hasher),
    }
    match mission.suspended {
        Some(m) => {
            1u8.hash(hasher);
            (m as u16).hash(hasher);
        }
        None => 0u8.hash(hasher),
    }
    mission.substate.hash(hasher);
    mission.timer.start_frame.hash(hasher);
    mission.timer.duration.hash(hasher);
    mission.tick_counter.hash(hasher);
}
```

**Step 2: Call it** — in `hash_entities`, at the END of the per-entity loop, immediately after the `rocking` `if let … else { 0u8.hash(hasher); }` block (the last field, ~line 622) and before the loop's closing `}`:
```rust
            // Mission substrate — folded as of Slice 8 (MissionCom is now
            // canonical hashed state, not an unhashed shadow).
            hash_mission_com(&entity.mission, hasher);
```
Do **NOT** touch the `hash_production` `dock_reservations.{contacts,contact_entered,on_pad}` fold (lines 222–240) — it is a live mirror (see Key Technical Decisions).

**Step 3: Build** — `cargo check -p vera20k`. Expected: compiles (MissionType is `#[repr(u16)]` so `as u16` is valid; `MissionTimer.start_frame`/`duration` are `u32`).

**Step 4: Capture the new baseline** — run `cargo test -p vera20k replay_hash_stable_through_slice6`. It will FAIL with a `left: <new>` / `right: <old 15996979913631807698>`. Read the literal `left:` value. Edit `slice6_retask_tests.rs:75`:
```rust
// Re-baselined for Slice 8: MissionCom folded into state_hash (the scripted
// entities now contribute their default mission bytes — composition change,
// not a behavior drift).
const SLICE6_BASELINE_HASH: u64 = <PASTE_LEFT_VALUE>;
```
Also append to the existing doc comment above the const (the "Re-baselined for Slice 7b" line) a "then Slice 8" note.

**Step 5: Verify** — `cargo test -p vera20k replay_hash_stable_through_slice6` PASS; `cargo test -p vera20k round_trip_preserves_state_hash` PASS (relative test — MissionCom already serializes, round-trip preserves it).

**Step 6: Commit** — `Slice 8 T1: fold MissionCom into state_hash + re-baseline SLICE6 hash`.

---

### Task 2: Delete the Slice-2 shadow agreement assert; retitle `refresh_mission_shadow`

**Why:** With `mission` now hashed and trusted, the debug cross-check against the legacy machines is retired. `refresh_mission_shadow` stays as the projection writer.

**Files:** Modify `src/sim/world/mod.rs`.

**Step 1: Delete the assert definition** — remove the entire `#[cfg(debug_assertions)] pub(crate) fn debug_assert_mission_shadow_consistent(&self) { … }` method (def ~line 921, the full body through its closing `}`).

**Step 2: Delete the call site** — at ~line 2446–2447 remove:
```rust
        #[cfg(debug_assertions)]
        self.debug_assert_mission_shadow_consistent();
```
Leave `self.refresh_mission_shadow();` (2445) and `debug_assert_s1_shadow` (2453, a separate object-AI program — do NOT touch) in place.

**Step 3: Retitle `refresh_mission_shadow` doc** — replace its doc comment (lines ~901–907) so it no longer claims `mission` is absent from `world_hash`. New doc:
```rust
    /// Refresh the `mission` component's `current`/`substate` on every entity
    /// from the authoritative `Option<T>` machines, and advance its per-entity
    /// `tick_counter`. As of Slice 8 `mission` IS folded into `world_hash`, so
    /// this is the canonical projection writer: `current`/`substate` are a
    /// deterministic function of the authoritative machines (the verbs own
    /// `queued`/`suspended`/`timer`). Runs before `state_hash()` each tick tail,
    /// so the folded value reflects the current tick. `values_mut()` yields
    /// deterministic ascending-id order.
```
Body unchanged.

**Step 4: Verify** — `cargo test -p vera20k -- world` (debug build; the deleted assert no longer compiles-in, refresh still runs). Expected: PASS.

**Step 5: Commit** — `Slice 8 T2: drop MissionCom shadow assert; mission is now hashed-authoritative`.

---

### Task 3: Invert `mission_shadow_does_not_change_state_hash`

**Why:** Folding `mission` means `refresh_mission_shadow` now *moves* the hash (tick_counter 0→1, current/substate). The Slice-2 invariant test is now false and must assert the opposite.

**Files:** Modify `src/sim/world/world_tests.rs:565-586`.

**Step 1: Replace the test** with:
```rust
#[test]
fn mission_refresh_changes_state_hash() {
    // As of Slice 8 `mission` is folded into world_hash, so refreshing it (which
    // advances tick_counter and re-derives current/substate) DOES move the
    // lockstep hash — the inverse of the Slice-2 shadow invariant.
    let mut sim = Simulation::new();
    sim.substrate
        .entities
        .insert(GameEntity::test_default(1, "E1", "Americans", 3, 3));
    let before = sim.state_hash();
    sim.refresh_mission_shadow();
    let after = sim.state_hash();
    assert_ne!(
        before, after,
        "mission refresh must perturb the state hash now that mission is folded"
    );
    assert_eq!(
        sim.substrate.entities.get(1).unwrap().mission.tick_counter,
        1,
        "refresh_mission_shadow actually ran (tick_counter advanced)"
    );
}
```

**Step 2: (optional) doc-comment fix** — in `techno_ai.rs` line ~244 the comment "Mirrors `mission_shadow_does_not_change_state_hash`" now names a renamed test; update it to "Mirrors the Slice-2 no-op-stage hash invariant" (the test `techno_ai_shell_is_passthrough_no_hash_change` itself is unaffected — `object_ai_stage` is a no-op so before==after regardless of the fold).

**Step 3: Verify** — `cargo test -p vera20k mission_refresh_changes_state_hash` PASS; `cargo test -p vera20k techno_ai_shell_is_passthrough_no_hash_change` PASS.

**Step 4: Commit** — `Slice 8 T3: invert mission shadow hash-invariant test`.

---

### Task 4: Bump `SNAPSHOT_VERSION` 16 → 17

**Why:** Documents the authoritative flip. Conservative (layout unchanged; MissionCom already serialized since Slice 6).

**Files:** Modify `src/sim/snapshot.rs:22`.

**Step 1: Edit** — add a comment line and bump:
```rust
// Bumped 16 -> 17: MissionCom folded into state_hash (Slice 8); bincode layout
// unchanged (MissionCom already serialized since Slice 6), only the hash changed.
const SNAPSHOT_VERSION: u32 = 17;
```

**Step 2: Verify** — `cargo test -p vera20k round_trip_preserves_state_hash` PASS (and any snapshot version-roundtrip test).

**Step 3: Commit** — `Slice 8 T4: bump SNAPSHOT_VERSION 16->17 (MissionCom authoritative)`.

---

### Task 5: New `mission_authoritative_tests.rs`

**Why:** Pin that each folded `MissionCom` field actually affects the hash. No `order_intent` tripwire (it stays).

**Files:** Create `src/sim/world/mission_authoritative_tests.rs`; wire mod in `world/mod.rs`.

**Step 1: Create the file:**
```rust
//! Slice 8 — `MissionCom` is folded into `state_hash`. These pin that each
//! component field is hash-relevant (the inverse of the Slice-2 shadow tests).
//! No `order_intent` selector tripwire: `order_intent` is load-bearing substate
//! and is retained (V5 map corrected).

use super::Simulation;
use crate::sim::game_entity::GameEntity;
use crate::sim::mission::{MissionTimer, MissionType};

fn two_sims() -> (Simulation, Simulation) {
    let mut a = Simulation::new();
    let mut b = Simulation::new();
    a.substrate
        .entities
        .insert(GameEntity::test_default(1, "MTNK", "Americans", 10, 10));
    b.substrate
        .entities
        .insert(GameEntity::test_default(1, "MTNK", "Americans", 10, 10));
    assert_eq!(a.state_hash(), b.state_hash(), "baseline sims must hash equal");
    (a, b)
}

#[test]
fn mission_current_changes_state_hash() {
    let (a, mut b) = two_sims();
    b.substrate.entities.get_mut(1).unwrap().mission.current = MissionType::Attack;
    assert_ne!(
        a.state_hash(),
        b.state_hash(),
        "mission.current must contribute to the state hash"
    );
}

#[test]
fn mission_timer_and_substate_change_state_hash() {
    let (a, mut b) = two_sims();
    // substate
    b.substrate.entities.get_mut(1).unwrap().mission.substate = 7;
    assert_ne!(a.state_hash(), b.state_hash(), "mission.substate must affect hash");
    // reset substate -> back to equal -> then perturb the timer
    b.substrate.entities.get_mut(1).unwrap().mission.substate = 0;
    assert_eq!(a.state_hash(), b.state_hash(), "substate reset restores equality");
    b.substrate.entities.get_mut(1).unwrap().mission.timer = MissionTimer::armed(5, 30);
    assert_ne!(a.state_hash(), b.state_hash(), "mission.timer must affect hash");
}

#[test]
fn mission_queued_and_suspended_change_state_hash() {
    let (a, mut b) = two_sims();
    b.substrate.entities.get_mut(1).unwrap().mission.queued = Some(MissionType::Guard);
    assert_ne!(a.state_hash(), b.state_hash(), "mission.queued must affect hash");
    b.substrate.entities.get_mut(1).unwrap().mission.queued = None;
    assert_eq!(a.state_hash(), b.state_hash(), "queued reset restores equality");
    b.substrate.entities.get_mut(1).unwrap().mission.suspended = Some(MissionType::Move);
    assert_ne!(a.state_hash(), b.state_hash(), "mission.suspended must affect hash");
}
```

**Step 2: Wire the mod** — in `world/mod.rs`, after the `slice6_retask_tests` mod (~line 2490) add:
```rust
#[cfg(test)]
#[path = "mission_authoritative_tests.rs"]
mod mission_authoritative_tests;
```

**Step 3: Verify** — `cargo test -p vera20k mission_authoritative` → 3 tests, all PASS.

**Step 4: Commit** — `Slice 8 T5: mission_authoritative tests (MissionCom is hash-relevant)`.

---

### Task 6: New `global_parity_harness_tests.rs` (the global lockstep regression guard)

**Why:** Project-wide determinism guard: a deterministic multi-faction skirmish recorded as a `ReplayLog` and re-run through the real `ReplayRunner::run`, asserting intra-run determinism + a committed final-hash baseline.

**Files:** Create `src/sim/world/global_parity_harness_tests.rs`; wire mod in `world/mod.rs`.

**Pattern:** entity/command/`advance_tick` setup mirrors [slice6_retask_tests.rs](../../src/sim/world/slice6_retask_tests.rs) (`unit()`, `cmd_envelope()`, `spawn_from_map`, the `advance_tick(&due, Some(&rules), &heights, Some(&grid), None, tick_ms)` loop). Refinery + ore enrichment mirrors `spawn_refinery` / `resource_nodes` in [miner_tests.rs:146/235](../../src/sim/miner/miner_tests.rs). Replay path is [replay.rs](../../src/sim/replay.rs) (`ReplayHeader`/`ReplayLog::new`/`record_tick`/`ReplayRunner::run`).

**Step 1: Create the file** with this structure (entity composition is the *starting* scenario; finalize empirically — see Step 3):
```rust
//! Slice 8 — global lockstep parity harness.
//!
//! Records a deterministic multi-faction skirmish as a `ReplayLog` and re-runs it
//! through the SAME `ReplayRunner::run` path the live game uses, asserting (1)
//! every tick's replayed hash equals the recorded hash (intra-run determinism)
//! and (2) the final hash equals a committed baseline. This is the project-wide
//! desync tripwire for the whole mission/radio substrate migration.

use super::*;
use crate::map::entities::{EntityCategory, MapEntity};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::pathfinding::PathGrid;
use crate::sim::replay::{ReplayHeader, ReplayLog, ReplayRunner};
use std::collections::BTreeMap;

const HARNESS_SEED: u64 = 0xC0FFEE_1234;
const HARNESS_TICKS: u64 = 600;
const HARNESS_TICK_MS: u32 = 67;

/// Committed final-hash baseline. Captured from the first green run (Step 3).
/// Re-baselines at most once; a later change to it needs a one-line reason.
const GLOBAL_HARNESS_FINAL_HASH: u64 = 0; // <PLACEHOLDER — fill from first run>

fn harness_rules() -> RuleSet {
    // Multi-faction vehicles + infantry + buildings (war factory, refinery).
    // Short weapon ranges so combat only fires when scripted/adjacent — keeps
    // the scenario deterministic and the baseline stable.
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=MTNK\n1=HARV\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GAWEAP\n1=GAREFN\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [HARV]\nStrength=600\nArmor=heavy\nSpeed=5\n\n\
         [GAWEAP]\nStrength=1000\nArmor=wood\nFoundation=4x3\n\n\
         [GAREFN]\nStrength=1000\nArmor=wood\nFoundation=3x3\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
    );
    RuleSet::from_ini(&ini).expect("harness rules should parse")
}

fn unit(owner: &str, type_id: &str, cx: u16, cy: u16, cat: EntityCategory) -> MapEntity {
    MapEntity {
        owner: owner.to_string(),
        type_id: type_id.to_string(),
        health: 256,
        cell_x: cx,
        cell_y: cy,
        facing: 64,
        category: cat,
        sub_cell: 0,
        veterancy: 0,
        high: false,
    }
}

/// Build the recorded scenario into `sim` and return the per-tick command script.
fn seed_scenario(sim: &mut Simulation, rules: &RuleSet, heights: &BTreeMap<(u16, u16), u8>) {
    sim.spawn_from_map(
        &[
            // House A: war factory + refinery + miner + a tank + infantry.
            unit("Americans", "GAWEAP", 3, 3, EntityCategory::Structure),
            unit("Americans", "GAREFN", 3, 10, EntityCategory::Structure),
            unit("Americans", "HARV", 6, 12, EntityCategory::Unit),
            unit("Americans", "MTNK", 8, 8, EntityCategory::Unit),
            unit("Americans", "E1", 9, 9, EntityCategory::Infantry),
            // House B: an opposing tank + infantry (Soviet — hostile by default).
            unit("Soviet", "MTNK", 40, 8, EntityCategory::Unit),
            unit("Soviet", "E1", 41, 9, EntityCategory::Infantry),
        ],
        Some(rules),
        heights,
    );
}

/// Scripted commands keyed by `execute_tick` (fires when tick+1 == execute_tick).
fn harness_script() -> Vec<(u64, Command)> {
    vec![
        (2, Command::Move { entity_id: 4, target_rx: 20, target_ry: 8, queue: false, group_id: None }),
        (40, Command::AttackMove { entity_id: 4, target_rx: 38, target_ry: 8, queue: false }),
        (120, Command::Move { entity_id: 5, target_rx: 30, target_ry: 12, queue: false, group_id: None }),
        (300, Command::Stop { entity_id: 4 }),
        (320, Command::Move { entity_id: 4, target_rx: 8, target_ry: 8, queue: false, group_id: None }),
    ]
}

#[test]
fn global_skirmish_replay_is_deterministic_and_baseline_stable() {
    let rules = harness_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let script = harness_script();

    // ---- Record pass: build a ReplayLog through the live advance_tick path. ----
    let mut rec = Simulation::with_seed(HARNESS_SEED);
    seed_scenario(&mut rec, &rules, &heights);
    let mut log = ReplayLog::new(ReplayHeader {
        version: 1,
        tick_hz: 15,
        seed: HARNESS_SEED,
        map_name: "global_parity_harness".to_string(),
        rules_hash: 0,
    });
    for tick in 0..HARNESS_TICKS {
        let due: Vec<CommandEnvelope> = script
            .iter()
            .filter(|(t, _)| *t == tick + 1)
            .map(|(t, c)| {
                let owner = rec.interner.get("Americans").expect("Americans interned");
                CommandEnvelope::new(owner, *t, c.clone())
            })
            .collect();
        let result = rec.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, HARNESS_TICK_MS);
        log.record_tick(tick, due, result.state_hash);
    }

    // ---- Replay pass: fresh sim, real ReplayRunner::run, assert tick-by-tick. ----
    let mut rep = Simulation::with_seed(HARNESS_SEED);
    seed_scenario(&mut rep, &rules, &heights);
    let replayed = ReplayRunner::run(&mut rep, &log, Some(&rules), &heights, Some(&grid), HARNESS_TICK_MS);

    assert_eq!(replayed.len(), log.ticks.len(), "replay tick count matches record");
    for (i, h) in replayed.iter().enumerate() {
        assert_eq!(
            *h, log.ticks[i].state_hash,
            "intra-run determinism: replay tick {i} hash must equal the recorded hash"
        );
    }

    let final_hash = *replayed.last().expect("at least one tick");
    assert_eq!(
        final_hash, GLOBAL_HARNESS_FINAL_HASH,
        "committed global-harness baseline. If this shifts for a real behavior \
         reason, re-baseline once with a one-line documented reason. (paste this \
         `left` value into GLOBAL_HARNESS_FINAL_HASH)"
    );
}
```

**Step 2: Wire the mod** — in `world/mod.rs`, after `mission_authoritative_tests` add:
```rust
#[cfg(test)]
#[path = "global_parity_harness_tests.rs"]
mod global_parity_harness_tests;
```

**Step 3: Run, validate, capture the baseline.** Run `cargo test -p vera20k global_skirmish_replay_is_deterministic_and_baseline_stable -- --nocapture`.
- **First** the *intra-run determinism* asserts must pass (replay == record tick-by-tick). If they FAIL, that is a real non-determinism bug — STOP and investigate (do not loosen the assert).
- If determinism passes, the *baseline* assert fails on `0` (placeholder). Read the `left:` value, paste into `GLOBAL_HARNESS_FINAL_HASH`, re-run → PASS.
- If `spawn_from_map`/`advance_tick` **panics** during the 600-tick run (e.g., a building/miner/INI path needs more setup), that is a real bug to surface; trim the scenario to the entities that spawn + advance cleanly (keep it multi-faction + multi-system) and note in the file's doc comment exactly what is exercised. Re-derive the baseline after any composition change.

**Step 4: Verify** — `cargo test -p vera20k global_skirmish_replay_is_deterministic_and_baseline_stable` PASS.

**Step 5: Commit** — `Slice 8 T6: global parity harness (deterministic replay + baseline)`.

---

### Task 7: Full regression + final verification

**Why:** Confirm the slice is behavior-neutral end-to-end and no unrelated golden state shifted.

**Steps:**
1. `cargo test -p vera20k mission_authoritative` → PASS.
2. `cargo test -p vera20k round_trip_preserves_state_hash` → PASS.
3. `cargo test -p vera20k global_skirmish_replay_is_deterministic_and_baseline_stable` → PASS.
4. `cargo test -p vera20k determinism_replay` → PASS (relative).
5. `cargo test -p vera20k replay_hash_stable_through_slice6` → PASS (re-baselined in T1).
6. `cargo test -p vera20k` → full suite green. Read the literal `test result:` line; if any *other* committed-hash test failed, STOP — the fold changed more than intended.
7. `cargo clippy -p vera20k` → no new warnings on touched files.
8. **Docs (local-only, no commit):** correct the V5 map row in `docs/plans/2026-06-01-…-implementation-plan.md` to `order_intent → KEEP (load-bearing substate)`; note Slice 8 deletes no fields and retains the live `dock_reservations` hashing (mirror retirement deferred). Append the two captured baselines + reasons to the Hash-baseline change ledger.

**No commit** (verification + local docs only) — or fold any clippy fixes into the relevant task's commit.

## Sources & References
- **Design:** approved brainstorm, this session.
- **Plan §Slice 8:** `docs/plans/2026-06-01-mission-radio-substrate-implementation-plan.md` (V5 map rows corrected here).
- **Code anchors:** `mission/mod.rs:188`, `mission/verb.rs`, `mission/retask.rs`, `world/mod.rs:478/510/617/908/2445/2454/2468-2490`, `world_hash.rs:13/222-240/382-623`, `world_tests.rs:565`, `snapshot.rs:22/306`, `replay.rs`, `slice6_retask_tests.rs:75`, `miner_dock.rs:6/47-82`, `components.rs:490`.
- **No Ghidra/INI** dependencies for this slice (determinism/serialization only).
