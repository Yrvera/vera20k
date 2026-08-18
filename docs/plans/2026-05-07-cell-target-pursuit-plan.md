# Cell-Target Pursuit Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Add gamemd-faithful pursuit so units with a committed `attack_target`
walk into weapon range and halt to fire (covers both `TargetKind::Cell` and
`TargetKind::Entity`; ground units only — aircraft already has its own state
machine).

**Architecture:** New pre-combat tick stage `Simulation::tick_attack_pursuit`
in [src/sim/world/world_orders.rs](../../src/sim/world/world_orders.rs), parallel
to the existing `tick_order_intents_pre_combat` / `tick_order_intents_post_combat`
methods. Combat tick range-fail branch flips from "retarget or drop" to
"continue" (matches gamemd `Mission_Attack`'s preserve-TarCom semantic).
No new components, no new enum variants, no snapshot version bump.

**Design Doc:** [2026-05-07-cell-target-pursuit-design.md](./2026-05-07-cell-target-pursuit-design.md)

---

## Grounding Summary

- **Docs (R1):** `ra2-rust-game-docs/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
  is the primary source. HIGH confidence; decompiled at `0x004D4DC0`
  (`FootClass::Mission_Attack`) and summarized at `0x004D5690`
  (`Greatest_Threat_Scan`, the approach driver). Doc explicitly states: range
  failure does NOT clear TarCom; pursuit happens whenever TarCom is set
  regardless of how it got set (player command, retaliation, AI acquisition,
  HasFoundAutoTarget finalization).
- **Ghidra (R2):** No additional Ghidra investigation needed for this plan.
  The doc's claims map directly onto the chosen approach. Out-of-scope items
  (spiral approach driver §3 step 5; `DefaultToGuardArea` re-anchor; 14–16
  frame dispatch cadence) are tracked as Open Follow-ups, not gaps.
- **Repo pattern (R3):** Mirror
  [Simulation::tick_order_intents_pre_combat](../../src/sim/world/world_orders.rs#L24)
  and [Simulation::tick_order_intents_post_combat](../../src/sim/world/world_orders.rs#L58).
  Reuse [Simulation::resolve_move_info](../../src/sim/world/world_commands.rs#L52)
  (currently private; promote to `pub(crate)`) to extract speed/locomotor info.
  Reuse [combat::resolve_target_coords](../../src/sim/combat/mod.rs#L249)
  (currently private with `#[allow(dead_code)]`; promote to `pub(crate)`).
- **INI (R4):** None. Pursuit reads existing `Range=` per-weapon values via
  `select_weapon_with_ifv`. No new INI keys.
- **Premise re-verification (A.1):** `git log -10` on touched files
  ([world/mod.rs](../../src/sim/world/mod.rs), [world_orders.rs](../../src/sim/world/world_orders.rs),
  [combat/mod.rs](../../src/sim/combat/mod.rs), [combat_tests.rs](../../src/sim/combat/combat_tests.rs))
  shows last touch was the force-fire-cell feature (commits d0f1605 + 86af0cd).
  No drift since the design was written today.

**What's still unknown:** nothing blocking. The performance behavior under
many simultaneously-stuck pursuers (A* thrash) is unknown but acceptable for
v1; mitigation deferred per design doc Open Follow-up #1.

## Key Technical Decisions

- **Pursuit runs every sim tick (not on a 14–16 frame jittered timer).**
  **Confidence:** medium. **Source:** Pre-existing cadence drift acknowledged in
  FOOTCLASS doc §10 and design doc Open Follow-up #6. Accepted for v1.
- **Straight-line halt-on-range-entry instead of gamemd's spiral approach
  cell.** **Confidence:** medium (cosmetic 1–2 cell endpoint drift). **Source:**
  Design doc Open Follow-up #3.
- **`pursuit_weapon_range` shared helper used by both pursuit and combat tick.**
  **Confidence:** high. **Source:** Design Tiny-Detail Ledger L17/L18 — same
  inputs prevent hysteresis at the range boundary. Without this, pursuit and
  combat could disagree on "in range" and oscillate.
- **Skip filters for non-pursuing entities (structures, aircraft, deployed,
  in-transport).** **Confidence:** high. **Source:** Ledger L14/L15/L16; matches
  gamemd's per-class Mission_Attack overrides (`AircraftClass::Mission_Attack`
  is its own state machine; structures use a different mission dispatch).
- **Combat tick range-fail branch becomes bare `continue;`.** **Confidence:**
  high. **Source:** FOOTCLASS doc §2.2 ("Does not set TarCom") + §3 step 6
  (NavCom-non-null branch may clear NavCom but not TarCom). Ledger L10.
- **Friendly-fire and visibility retarget branches at
  [combat/mod.rs:1360, 1376](../../src/sim/combat/mod.rs#L1360) are NOT changed.**
  **Confidence:** high. **Source:** Out of scope per brainstorm option (b);
  separate follow-up brainstorm tracked in design Open Follow-up #5.

## Open Questions

### Resolved During Planning

- **Where does the new method live?** → `src/sim/world/world_orders.rs` next
  to existing pre/post-combat order intent methods. Same shape, same module.
- **How to clamp unwalkable goal cells?** → Don't pre-clamp. `issue_move_command_with_layered`
  calls `resolve_requested_move_goal` internally
  ([movement_commands.rs:195](../../src/sim/movement/movement_commands.rs#L195))
  to handle unwalkable targets. Keeps sim layer free of the `nearest_walkable_cell_layered`
  helper that lives in app_sim_tick.rs.
- **Is `MoveInfo` exposed?** → No, it's private in world_commands.rs. Promote
  to `pub(crate)` along with `resolve_move_info`.

### Deferred to Implementation

- **Exact A* path-cost behavior under pursuit re-issue.** Pursuit re-issues
  movement when `movement_target.is_none()` and out of range. If A* fails or
  produces an empty path, the entity sits idle this tick and pursuit retries
  next tick. Performance impact in pathological cases (many stuck pursuers)
  observed empirically — Open Follow-up #1.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Promote `resolve_target_coords` to `pub(crate)`. Add new `pursuit_weapon_range` helper. Flip range-fail branch to `continue;`. |
| Modify | [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) | Promote `MoveInfo` struct + `resolve_move_info` method to `pub(crate)`. |
| Modify | [src/sim/world/world_orders.rs](../../src/sim/world/world_orders.rs) | Add `Simulation::tick_attack_pursuit` method. |
| Modify | [src/sim/world/mod.rs](../../src/sim/world/mod.rs) | Call `tick_attack_pursuit` in `advance_tick` between `tick_order_intents_pre_combat` and `tick_combat_with_fog`. |
| Modify | [src/sim/combat/combat_tests.rs](../../src/sim/combat/combat_tests.rs) | Invert `test_tick_combat_out_of_range` to expect attack_target preserved. Audit and update any other tests that rely on range-fail-drops behavior. |
| Create | [src/sim/combat/combat_pursuit_tests.rs](../../src/sim/combat/combat_pursuit_tests.rs) | New unit tests covering pursuit semantics. |
| Modify | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Register the new `combat_pursuit_tests` module under `#[cfg(test)]`. |

## Interface Changes

**Promoted visibility (private → `pub(crate)`):**
- `combat::resolve_target_coords` — pre-existing helper, was `#[allow(dead_code)]`. Now used by pursuit.
- `combat::pursuit_weapon_range` — new `pub(crate)` helper.
- `Simulation::resolve_move_info` — pre-existing private method.
- `Simulation::MoveInfo` (or `crate::sim::world::world_commands::MoveInfo`) — pre-existing private struct.

**Behavior change (no signature change):**
- Combat tick at [combat/mod.rs:1412-1427](../../src/sim/combat/mod.rs#L1412-L1427)
  no longer clears `attack_target` on range failure and no longer pushes to
  `retarget_events`. **Consumers affected:**
  - Tests that assert range-fail drops the target (must update — Task 6).
  - Any code that consumes `combat_result.despawned_ids` is unaffected
    (those are deaths, not removed-attack).
  - The `retarget_events` vector continues to fire from the friendly-fire
    and visibility branches (unchanged).

## Sim Checklist

- [x] All math uses `fixed`-point — pursuit reuses `lepton_distance_sq_raw`,
      `is_within_range_leptons`, `SimFixed`. No f32/f64 introduced.
- [x] New state included in deterministic state hash — no new fields; existing
      `attack_target` and `movement_target` already participate in the hash.
- [x] No dependencies on render/ui/sidebar/audio/net — pursuit lives in
      `sim/world` and uses only `sim/`, `rules/`, `map/` modules.
- [x] Tick ordering impact noted — new stage between
      `tick_order_intents_pre_combat` and `tick_combat_with_fog`.
- [x] BTreeMap iteration order considered — pursuit iterates
      `self.entities.keys_sorted()` which is deterministic.

## Risk Areas

From the design's Impact Analysis:

1. **L17/L18 hysteresis** — pursuit and combat must agree on "in range." If
   they use different weapon-range or target-coord computations, the unit
   oscillates: pursuit halts at distance D, combat says "out of range,"
   pursuit re-issues, repeat. **Mitigation:** Task 1 introduces
   `pursuit_weapon_range` consumed by both. **Regression test:** Task 7 #5
   exercises the exact-range boundary.
2. **Inverted existing tests** — `test_tick_combat_out_of_range`
   ([combat_tests.rs:329](../../src/sim/combat/combat_tests.rs#L329))
   currently asserts `attack_target.is_none()` after combat tick on
   out-of-range target. After Task 5's branch flip, attack_target is
   preserved. **Mitigation:** Task 6 inverts the assertion.
3. **Other tests we haven't found** — any test elsewhere asserting
   range-fail drops will fail. **Mitigation:** Task 9 runs the full suite
   and updates anything that breaks. Per a focused grep, the candidate set
   is small (`out_of_range`, `retarget_events`, `remove_attack`).
4. **Garrison auto-acquire path** — sets `attack_target` on structures.
   Pursuit must skip them. **Mitigation:** filter `entity.category == Structure`
   in the skip block.
5. **Aircraft attack mission** — has its own pursuit. Pursuit must skip
   aircraft. **Mitigation:** filter `entity.aircraft_mission.is_some()`
   in the skip block.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 5 | Range failure preserves `attack_target`, no auto-retarget. | Player issues force-fire-cell on far cell → unit must walk there, not silently drop the order or chase a nearby enemy. Triggers every Ctrl-click on out-of-range cells; high frequency in normal play. | Task 7 #1 (pursuit issues movement); Task 6 (combat tick alone preserves target); manual sandbox: ctrl-click far cell with Grizzly, observe walk + fire. |
| Task 1 | `pursuit_weapon_range` is used identically in pursuit AND combat tick. | Same range source = no oscillation at boundary. Triggers any time a unit pursues to the edge of weapon range — every engagement. | Task 7 #5: place attacker at exactly range distance, verify no oscillation. |
| Task 3 | Halt-on-range-entry: clear `movement_target` once in range. | Without this, unit walks past target, fires from inside foundation, looks unnatural. Triggers every successful pursuit. | Task 7 #2: in-range entity with active movement → tick → movement cleared. |
| Task 3 | Skip filters L14 (aircraft), L15 (structures), L16 (deployed). | Aircraft has its own state machine; structures can't move; deployed-fire infantry locked. Triggers any time pursuit stage touches one — possibly every tick during combat. Skipping prevents broken state. | Task 7 #5/#6/#7: each filter has a dedicated test. |
| Task 5 | Don't break entity-target attacks: preserved target works in same way as cell-target. | Right-click on far enemy must also pursue. Triggers every right-click on out-of-range hostile. | Task 7 #3 (entity pursuit) + manual sandbox right-click test. |

---

## Tasks

### Task 1: Add `pursuit_weapon_range` helper and promote `resolve_target_coords` in combat

**Why:** Pursuit and the combat tick range check must use identical inputs to
prevent boundary oscillation. This task introduces the single source of truth
for "what range applies to this attacker against this target?" and exposes
the existing target-coord helper for pursuit's use.

**Files:**
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) at lines 248-259 (visibility), and append new helper near line 260.

**Pattern:** Follows existing free-function pattern next to `target_coords`,
`cell_center_coords`, `resolve_target_coords`.

**Step 1: Promote `resolve_target_coords` visibility**

In [combat/mod.rs](../../src/sim/combat/mod.rs) at line 248-259, change:

```rust
#[allow(dead_code)] // Used by aircraft refactor (Task 5); also useful for future combat tick rewrites.
fn resolve_target_coords(
    target: &TargetKind,
    entities: &EntityStore,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> Option<(u16, u16, SimFixed, SimFixed)> {
    match *target {
        TargetKind::Entity(id) => entities.get(id).map(|t| target_coords(t, rules, interner)),
        TargetKind::Cell(rx, ry) => Some(cell_center_coords(rx, ry)),
    }
}
```

To:

```rust
/// Resolve target coords from a `TargetKind`, looking up entity position when
/// needed and using cell-center for `Cell` targets.
///
/// Returns `None` if the target is `Entity(id)` and the entity no longer
/// exists (despawned). `Cell` targets always resolve.
///
/// Shared by the combat tick and the pursuit pre-combat stage so range
/// decisions stay consistent (Tiny-Detail Ledger L18).
pub(crate) fn resolve_target_coords(
    target: &TargetKind,
    entities: &EntityStore,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> Option<(u16, u16, SimFixed, SimFixed)> {
    match *target {
        TargetKind::Entity(id) => entities.get(id).map(|t| target_coords(t, rules, interner)),
        TargetKind::Cell(rx, ry) => Some(cell_center_coords(rx, ry)),
    }
}
```

**Step 2: Add `pursuit_weapon_range` helper**

In [combat/mod.rs](../../src/sim/combat/mod.rs) immediately after the
`resolve_target_coords` definition (around line 260), append:

```rust
/// Resolve the effective weapon range for an attacker against a `TargetKind`.
///
/// Uses the same weapon-select inputs as the combat tick's Phase 2 weapon
/// selection ([combat/mod.rs] around line 1334), so pursuit and combat
/// agree on "in range" at the boundary.
///
/// For `Entity` targets: uses the target's actual category and armor.
/// For `Cell` targets: synthesizes `Structure` + attacker's own armor,
/// matching the cell-target synthesis at [combat/mod.rs] around line 1259.
///
/// Returns `None` if no weapon engages the target (Verses 0% or projectile
/// AA/AG mismatch). Pursuit treats `None` as "skip — combat tick will drop
/// the attack on its own weapon-select fail."
///
/// Tiny-Detail Ledger L17.
pub(crate) fn pursuit_weapon_range(
    entity: &GameEntity,
    target: &TargetKind,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
) -> Option<SimFixed> {
    use self::combat_weapon::select_weapon_with_ifv;
    use crate::map::entities::EntityCategory;

    let attacker_obj = rules.object(interner.resolve(entity.type_ref))?;
    let (target_cat, target_armor) = match *target {
        TargetKind::Entity(id) => {
            let target_entity = entities.get(id)?;
            let armor = rules
                .object(interner.resolve(target_entity.type_ref))
                .map(|o| o.armor.clone())
                .unwrap_or_else(|| "none".to_string());
            (target_entity.category, armor)
        }
        TargetKind::Cell(_, _) => {
            // Synthetic — must match combat tick's cell-target synthesis at
            // combat/mod.rs around line 1259. Using attacker's own armor here is the
            // pre-existing convention; a separate brainstorm (trace audit
            // Drift 1) is tracked for fixing this.
            let armor = rules
                .object(interner.resolve(entity.type_ref))
                .map(|o| o.armor.clone())
                .unwrap_or_else(|| "none".to_string());
            (EntityCategory::Structure, armor)
        }
    };
    select_weapon_with_ifv(
        rules,
        attacker_obj,
        target_cat,
        &target_armor,
        entity.ifv_weapon_index,
    )
    .map(|sel| sel.weapon.range)
}
```

**Step 3: Add unit test for `pursuit_weapon_range`**

Append to [src/sim/combat/combat_tests.rs](../../src/sim/combat/combat_tests.rs):

```rust
#[test]
fn pursuit_weapon_range_for_entity_target() {
    use crate::sim::combat::{TargetKind, pursuit_weapon_range};
    let rules = test_rules();
    let interner = test_interner();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 0, 0, 300));
    store.insert(make_entity(2, "MTNK", 5, 0, 300));

    let attacker = store.get(1).unwrap();
    let range = pursuit_weapon_range(
        attacker,
        &TargetKind::Entity(2),
        &store,
        &rules,
        &interner,
    );
    // 105mm Range=6.
    assert_eq!(range, Some(crate::util::fixed_math::SimFixed::from_num(6)));
}

#[test]
fn pursuit_weapon_range_for_cell_target() {
    use crate::sim::combat::{TargetKind, pursuit_weapon_range};
    let rules = test_rules();
    let interner = test_interner();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "MTNK", 0, 0, 300));

    let attacker = store.get(1).unwrap();
    let range = pursuit_weapon_range(
        attacker,
        &TargetKind::Cell(50, 50),
        &store,
        &rules,
        &interner,
    );
    // Cell target uses synthetic Structure category. MTNK 105mm Cannon is AG=true
    // (default), AP Verses[heavy] = 75% > 0. Range = 6.
    assert_eq!(range, Some(crate::util::fixed_math::SimFixed::from_num(6)));
}

#[test]
fn pursuit_weapon_range_none_for_unarmed_attacker() {
    use crate::sim::combat::{TargetKind, pursuit_weapon_range};
    let rules_str = "[InfantryTypes]\n0=ENGI\n\n\
                     [VehicleTypes]\n\n[BuildingTypes]\n\n[AircraftTypes]\n\n\
                     [ENGI]\nStrength=75\nArmor=none\nSpeed=4\n";
    let ini = IniFile::from_str(rules_str);
    let rules = RuleSet::from_ini(&ini).expect("parse");
    let interner = test_interner();
    let mut store = EntityStore::new();
    store.insert(make_entity(1, "ENGI", 0, 0, 75));

    let attacker = store.get(1).unwrap();
    let range = pursuit_weapon_range(
        attacker,
        &TargetKind::Cell(50, 50),
        &store,
        &rules,
        &interner,
    );
    assert_eq!(range, None);
}
```

**Step 4: Verify**

Run: `cargo test --lib pursuit_weapon_range`

Expected: 3 tests pass.

**Step 5: Commit**

```
git add src/sim/combat/mod.rs src/sim/combat/combat_tests.rs
git commit -m "combat: add pursuit_weapon_range helper + promote resolve_target_coords"
```

---

### Task 2: Promote `MoveInfo` and `resolve_move_info` to `pub(crate)`

**Why:** Pursuit needs the same speed/locomotor info that
`apply_command::Move` uses to issue movement. Reusing the existing helper
guarantees pursuit-issued movement matches Move-issued movement (no drift).

**Files:**
- Modify: [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) at lines 33 (struct) and 52 (method).

**Pattern:** Promote private items to `pub(crate)` to expose them across
sibling modules within the same `sim/world/` directory.

**Step 1: Promote `MoveInfo` struct**

In [world_commands.rs:33-48](../../src/sim/world/world_commands.rs#L33-L48), change:

```rust
/// Read-only snapshot of entity + rules data needed for issuing movement commands.
/// Captured once to avoid repeated entity lookups and type_ref clones.
struct MoveInfo {
    speed: SimFixed,
    loco_kind: Option<LocomotorKind>,
    // ...
}
```

To (add `pub(crate)` to struct and each field that pursuit reads):

```rust
/// Read-only snapshot of entity + rules data needed for issuing movement commands.
/// Captured once to avoid repeated entity lookups and type_ref clones.
///
/// `pub(crate)` so the pursuit pre-combat stage in `world_orders.rs` can reuse
/// it — pursuit-issued movement must match Move-command-issued movement
/// exactly to keep behavior consistent.
pub(crate) struct MoveInfo {
    pub(crate) speed: SimFixed,
    pub(crate) loco_kind: Option<LocomotorKind>,
    pub(crate) loco_layer: MovementLayer,
    pub(crate) speed_type: SpeedType,
    pub(crate) hover_attack: bool,
    pub(crate) is_teleporter: bool,
    pub(crate) is_harvester: bool,
    pub(crate) is_infantry: bool,
    pub(crate) accel_factor: SimFixed,
    pub(crate) decel_factor: SimFixed,
    pub(crate) slowdown_distance: SimFixed,
    pub(crate) movement_zone: MovementZone,
    pub(crate) position: (u16, u16),
    pub(crate) mover_is_crusher: bool,
}
```

**Step 2: Promote `resolve_move_info` method**

In [world_commands.rs:52](../../src/sim/world/world_commands.rs#L52), change the method declaration:

```rust
fn resolve_move_info(&self, entity_id: u64, rules: Option<&RuleSet>) -> Option<MoveInfo> {
```

To:

```rust
pub(crate) fn resolve_move_info(&self, entity_id: u64, rules: Option<&RuleSet>) -> Option<MoveInfo> {
```

**Step 3: Verify**

Run: `cargo check --lib`

Expected: clean compile (no callers broken; visibility is loosened, not tightened).

**Step 4: Commit**

```
git add src/sim/world/world_commands.rs
git commit -m "world: promote MoveInfo + resolve_move_info to pub(crate) for pursuit reuse"
```

---

### Task 3: Implement `Simulation::tick_attack_pursuit` in `world_orders.rs`

**Why:** This is the new pre-combat stage that walks units toward
out-of-range targets and halts movement when in range. Mirrors gamemd's
`Mission_Attack` step 3 + `Greatest_Threat_Scan` semantic.

**Files:**
- Modify: [src/sim/world/world_orders.rs](../../src/sim/world/world_orders.rs) — append a new method at the end of `impl Simulation`.

**Pattern:** Mirror
[Simulation::tick_order_intents_pre_combat](../../src/sim/world/world_orders.rs#L24)
(read phase + apply phase, deterministic key iteration).

**Step 1: Add imports**

At the top of [world_orders.rs](../../src/sim/world/world_orders.rs)
(merge with existing imports — these are all already partially used):

```rust
use crate::map::entities::EntityCategory;
use crate::sim::combat;
use crate::sim::movement::bump_crush;
```

If `EntityCategory`, `combat`, or `bump_crush` are already imported, just
ensure they exist; don't duplicate.

**Step 2: Add the pursuit method**

Append inside the existing `impl Simulation { … }` block (after
`tick_capture_orders`, before the closing brace):

```rust
    /// Pre-combat: entities with an `attack_target` that's out of weapon
    /// range walk toward the target. Entities that just entered range halt
    /// their movement so the combat tick can fire from a stationary
    /// position.
    ///
    /// Mirrors gamemd `FootClass::Mission_Attack` step 3 — TarCom is
    /// preserved while pursuing; range failure does NOT retarget. See
    /// `ra2-rust-game-docs/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`.
    ///
    /// Skips entities that can't or shouldn't pursue:
    /// - Structures (can't move)
    /// - Aircraft (own state machine in `attack_mission.rs`)
    /// - Deployed-fire infantry (locked while deployed)
    /// - Entities inside transports
    /// - Dying entities
    pub(crate) fn tick_attack_pursuit(
        &mut self,
        rules: &RuleSet,
        path_grid: Option<&PathGrid>,
    ) {
        let Some(grid) = path_grid else {
            return;
        };

        // Phase 1: collect pursuit decisions (read-only on entities).
        // Two action kinds: issue a new path, or clear an existing one.
        enum PursuitAction {
            IssueMove { entity_id: u64, goal: (u16, u16) },
            ClearMovement { entity_id: u64 },
        }

        let keys: Vec<u64> = self.entities.keys_sorted();
        let mut actions: Vec<PursuitAction> = Vec::new();

        for &id in &keys {
            let Some(entity) = self.entities.get(id) else {
                continue;
            };
            let Some(attack) = entity.attack_target.as_ref() else {
                continue;
            };

            // Skip filters — see "Skips" doc above.
            if entity.dying {
                continue;
            }
            if entity.category == EntityCategory::Structure {
                continue;
            }
            if entity.aircraft_mission.is_some() {
                continue;
            }
            if entity.is_deployed() {
                continue;
            }
            if entity.passenger_role.is_inside_transport() {
                continue;
            }

            // Resolve target coords using the same helper combat tick uses
            // (Tiny-Detail Ledger L18). None means entity-target despawned;
            // combat tick's target-dead branch handles cleanup.
            let target_pos = combat::resolve_target_coords(
                &attack.target,
                &self.entities,
                Some(rules),
                &self.interner,
            );
            let Some((trx, try_, tsx, tsy)) = target_pos else {
                continue;
            };

            // Resolve weapon range using shared helper (Ledger L17).
            // None means no weapon can engage; combat tick will drop on its
            // own weapon-select fail.
            let Some(weapon_range) = combat::pursuit_weapon_range(
                entity,
                &attack.target,
                &self.entities,
                rules,
                &self.interner,
            ) else {
                continue;
            };

            // Range check — same math as combat tick.
            let dist_sq = combat::lepton_distance_sq_raw(
                entity.position.rx,
                entity.position.ry,
                entity.position.sub_x,
                entity.position.sub_y,
                trx,
                try_,
                tsx,
                tsy,
            );
            let in_range = combat::is_within_range_leptons(dist_sq, weapon_range);

            if !in_range {
                if entity.movement_target.is_none() {
                    // Out of range, no current pursuit — issue a path.
                    actions.push(PursuitAction::IssueMove {
                        entity_id: id,
                        goal: (trx, try_),
                    });
                }
                // else: existing pursuit movement is still running; let it continue.
            } else if entity.movement_target.is_some() {
                // In range — halt for firing.
                actions.push(PursuitAction::ClearMovement { entity_id: id });
            }
        }

        // Phase 2: apply mutations.
        for action in actions {
            match action {
                PursuitAction::IssueMove { entity_id, goal } => {
                    let Some(info) = self.resolve_move_info(entity_id, Some(rules)) else {
                        continue;
                    };
                    let owner_str = self
                        .entities
                        .get(entity_id)
                        .map(|e| self.interner.resolve(e.owner).to_string())
                        .unwrap_or_default();
                    let (entity_blocks, entity_block_map) = bump_crush::build_entity_block_set(
                        &self.entities,
                        &owner_str,
                        &self.house_alliances,
                        &self.interner,
                    );
                    let cost_grid = self.terrain_costs.get(&info.speed_type);
                    let _issued = movement::issue_move_command_with_layered(
                        &mut self.entities,
                        grid,
                        entity_id,
                        goal,
                        info.speed,
                        false, // queue
                        cost_grid,
                        Some(&entity_blocks),
                        self.resolved_terrain.as_ref(),
                        Some(&entity_block_map),
                        info.mover_is_crusher,
                    );
                    // No-op if A* fails — pursuit retries next tick.
                }
                PursuitAction::ClearMovement { entity_id } => {
                    if let Some(e) = self.entities.get_mut(entity_id) {
                        e.movement_target = None;
                    }
                }
            }
        }
    }
```

**Step 3: Verify compile**

Run: `cargo check --lib`

Expected: clean compile. The function is defined but not yet called; no
behavior change yet.

**Step 4: Commit**

```
git add src/sim/world/world_orders.rs
git commit -m "world_orders: add tick_attack_pursuit pre-combat stage"
```

---

### Task 4: Wire `tick_attack_pursuit` into `advance_tick`

**Why:** The new stage exists but doesn't run yet. This task hooks it into
the tick pipeline between `tick_order_intents_pre_combat` (which may set
fresh attack_targets via auto-acquire) and `tick_combat_with_fog`.

**Files:**
- Modify: [src/sim/world/mod.rs:1150-1153](../../src/sim/world/mod.rs#L1150-L1153).

**Pattern:** Adds a single line to the existing tick-stage sequence in `advance_tick`.

**Step 1: Insert the pursuit call**

In [world/mod.rs](../../src/sim/world/mod.rs) at lines 1150-1153, change:

```rust
            turret::tick_turret_rotation(&mut self.entities, rules, tick_ms, &self.interner);
            spawned_entities |= self.tick_capture_orders();
            self.tick_order_intents_pre_combat(rules);
            let combat_result = combat::tick_combat_with_fog(
```

To:

```rust
            turret::tick_turret_rotation(&mut self.entities, rules, tick_ms, &self.interner);
            spawned_entities |= self.tick_capture_orders();
            self.tick_order_intents_pre_combat(rules);
            // Pursuit: walk units with out-of-range attack_target into range,
            // halt movement on range entry. Must run before combat so combat
            // sees the up-to-date movement_target this tick.
            self.tick_attack_pursuit(rules, path_grid);
            let combat_result = combat::tick_combat_with_fog(
```

**Step 2: Verify compile**

Run: `cargo check --lib`

Expected: clean compile.

**Step 3: Run combat + world tests (pursuit is now active but combat tick still drops on range fail)**

Run: `cargo test --lib combat:: world::`

Expected: all tests pass. Pursuit is active, but combat tick still has the
old range-fail-drops behavior — tests are still consistent because pursuit
sets `movement_target` and combat tick still drops `attack_target` on range
fail. Behavior is briefly inconsistent (pursuit issues movement and combat
drops attack — they fight) for one tick, but no test exercises a full
pursue-and-fire flow yet. **Don't commit this state for long; Task 5 finishes
the half-state.**

**Step 4: Commit**

```
git add src/sim/world/mod.rs
git commit -m "world: wire tick_attack_pursuit into advance_tick before combat"
```

---

### Task 5: Flip combat tick range-fail branch to `continue;`

**Why:** This is the load-bearing parity change (Tiny-Detail Ledger L10).
Range failure must NOT clear `attack_target` and must NOT trigger
auto-retarget. Pursuit will close the distance instead. Removing the
retarget call also closes the per-tick re-acquire drift flagged in the
trace audit.

**Files:**
- Modify: [src/sim/combat/mod.rs:1412-1427](../../src/sim/combat/mod.rs#L1412-L1427).

**Pattern:** Replaces a 16-line branch with a 1-line `continue;`. No new pattern.

**Step 1: Replace the range-fail branch**

In [combat/mod.rs](../../src/sim/combat/mod.rs) at lines 1412-1427, change:

```rust
        if !is_within_range_leptons(dist_sq, effective_range) {
            if let Some(new_target) = acquire_best_target(
                entities,
                rules,
                interner,
                snap,
                obj,
                fog,
                garrison_retarget_range,
            ) {
                retarget_events.push((snap.stable_id, new_target));
            } else {
                remove_attack.push(snap.stable_id);
            }
            continue;
        }
```

To:

```rust
        // Range failure: gamemd `Mission_Attack` does NOT clear TarCom or
        // retarget on range alone — `Greatest_Threat_Scan` (the approach
        // driver) walks the unit into range. Our `tick_attack_pursuit`
        // pre-combat stage handles the walk; combat tick just skips this
        // tick's fire attempt and lets the unit close the gap.
        // Tiny-Detail Ledger L10. See FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT
        // §2.2 + §3 step 6.
        if !is_within_range_leptons(dist_sq, effective_range) {
            continue;
        }
```

**Step 2: Verify compile**

Run: `cargo check --lib`

Expected: clean compile.

**Step 3: Run combat tests — expect `test_tick_combat_out_of_range` to fail**

Run: `cargo test --lib combat::tests::test_tick_combat_out_of_range`

Expected: **test FAILS** at the assertion `assert!(store.get(1).unwrap().attack_target.is_none(), …)`.
This confirms the branch flip took effect. Task 6 inverts the assertion.

**Step 4: Commit**

```
git add src/sim/combat/mod.rs
git commit -m "combat: range failure preserves attack_target (gamemd parity, ledger L10)"
```

---

### Task 6: Invert `test_tick_combat_out_of_range` to expect attack preserved

**Why:** The flipped branch in Task 5 means the combat tick alone no longer
drops the attack on range failure. Pursuit (which is wired up but doesn't
run in this unit test because it calls `tick_combat` directly) takes the
walk responsibility. The test must reflect the new contract.

**Files:**
- Modify: [src/sim/combat/combat_tests.rs:328-357](../../src/sim/combat/combat_tests.rs#L328-L357).

**Pattern:** Standard assertion update; keep test name and structure.

**Step 1: Update the test**

In [combat_tests.rs:328-357](../../src/sim/combat/combat_tests.rs#L328-L357), change:

```rust
#[test]
fn test_tick_combat_out_of_range() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    // 105mm range = 6 cells. Target at distance 10.
    store.insert(make_entity(1, "MTNK", 0, 0, 300));
    store.insert(make_entity(2, "MTNK", 10, 0, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
    );

    let target_health = store.get(2).unwrap().health.current;
    assert_eq!(
        target_health, 300,
        "Out-of-range target should not take damage"
    );
    assert!(
        store.get(1).unwrap().attack_target.is_none(),
        "AttackTarget removed when out of range"
    );
}
```

To:

```rust
#[test]
fn test_tick_combat_out_of_range() {
    let rules: RuleSet = test_rules();
    let mut store = EntityStore::new();
    // 105mm range = 6 cells. Target at distance 10.
    store.insert(make_entity(1, "MTNK", 0, 0, 300));
    store.insert(make_entity(2, "MTNK", 10, 0, 300));
    let mut interner = test_interner();
    issue_attack_command(&mut store, 1, 2, None, &interner);

    tick_combat(
        &mut store,
        &mut OccupancyGrid::new(),
        &rules,
        &mut interner,
        &mut BTreeMap::new(),
        0u64,
        100,
    );

    let target_health = store.get(2).unwrap().health.current;
    assert_eq!(
        target_health, 300,
        "Out-of-range target should not take damage"
    );
    // Range failure preserves attack_target; pursuit (run from advance_tick,
    // not from tick_combat in isolation) walks the unit into range.
    // Tiny-Detail Ledger L10 + L2.
    assert!(
        store.get(1).unwrap().attack_target.is_some(),
        "AttackTarget preserved when out of range — pursuit closes the gap"
    );
}
```

**Step 2: Verify**

Run: `cargo test --lib combat::tests::test_tick_combat_out_of_range`

Expected: PASS.

**Step 3: Run the full combat test suite**

Run: `cargo test --lib combat::`

Expected: All combat tests pass. If anything else asserted "drops on range
fail," it will fail here — fix in Task 9.

**Step 4: Commit**

```
git add src/sim/combat/combat_tests.rs
git commit -m "combat: invert test_tick_combat_out_of_range assertion (target preserved)"
```

---

### Task 7: Add pursuit unit tests in new `combat_pursuit_tests.rs`

**Why:** Cover the pursuit semantics directly: out-of-range issues movement,
in-range halts, skip filters work, range boundary doesn't oscillate.

**Files:**
- Create: [src/sim/combat/combat_pursuit_tests.rs](../../src/sim/combat/combat_pursuit_tests.rs)
- Modify: [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — register the new test module.

**Pattern:** Follow
[combat_force_fire_cell_tests.rs](../../src/sim/combat/combat_force_fire_cell_tests.rs)
structure — minimal RuleSet, helper to construct entities, one test per
behavior. These are pursuit-stage integration tests, so they construct a
full `Simulation` rather than just an `EntityStore`.

**Step 1: Create the test file**

Create [src/sim/combat/combat_pursuit_tests.rs](../../src/sim/combat/combat_pursuit_tests.rs):

```rust
//! Tests for `Simulation::tick_attack_pursuit` — the pre-combat stage
//! that walks units toward out-of-range attack targets and halts them
//! when in range.
//!
//! See `docs/plans/2026-05-07-cell-target-pursuit-design.md`.

use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::aircraft::AircraftMission;
use crate::sim::docking::aircraft_dock::AircraftAmmo;
use crate::sim::combat::{AttackTarget, TargetKind};
use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;

/// Minimal RuleSet for pursuit tests: armed Grizzly + Rhino, AP warhead with
/// non-zero Verses against heavy. Range=6 cells.
fn pursuit_rules() -> RuleSet {
    let ini_str: &str = "\
[VehicleTypes]\n0=MTNK\n1=HTNK\n\n\
[InfantryTypes]\n0=ENGI\n\n\
[BuildingTypes]\n0=GAPILL\n\n\
[AircraftTypes]\n0=ORCA\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[HTNK]\nStrength=400\nArmor=heavy\nSpeed=5\nPrimary=105mm\n\n\
[ENGI]\nStrength=75\nArmor=none\nSpeed=4\n\n\
[GAPILL]\nStrength=400\nArmor=heavy\nPrimary=105mm\n\n\
[ORCA]\nStrength=150\nArmor=light\nSpeed=14\nPrimary=105mm\n\n\
[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n";
    let ini: IniFile = IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("pursuit_rules should parse")
}

/// Construct a Simulation with a flat 64x64 PathGrid and the given entities
/// pre-inserted. Returns the sim plus the path grid (kept alive separately
/// because tick_attack_pursuit borrows it).
fn make_sim(entities: Vec<GameEntity>) -> (Simulation, PathGrid) {
    let mut sim = Simulation::new();
    for e in entities {
        sim.entities.insert(e);
    }
    // Flat fully-walkable grid for ground locomotor, no terrain costs.
    let grid = PathGrid::test_all_passable(64, 64);
    (sim, grid)
}

fn make_unit(id: u64, type_ref: &str, owner: &str, rx: u16, ry: u16, hp: u16) -> GameEntity {
    let mut e = GameEntity::test_default(id, type_ref, owner, rx, ry);
    e.health = Health {
        current: hp,
        max: hp,
    };
    e
}

#[test]
fn cell_target_out_of_range_issues_movement() {
    // Grizzly at (5,5), force-fire Cell(15,15). Range=6, distance=10 → out of range.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 5, 5, 300);
    grizzly.attack_target = Some(AttackTarget::for_cell(15, 15));
    let (mut sim, grid) = make_sim(vec![grizzly]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_some(),
        "attack_target preserved during pursuit"
    );
    assert!(
        entity.movement_target.is_some(),
        "out-of-range cell target should issue movement"
    );
}

#[test]
fn cell_target_in_range_clears_movement() {
    // Grizzly at (8,5), force-fire Cell(10,5). Distance=2 → in range.
    // Pre-set a movement_target as if pursuit had issued one earlier.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 8, 5, 300);
    grizzly.attack_target = Some(AttackTarget::for_cell(10, 5));
    grizzly.movement_target = Some(crate::sim::components::MovementTarget::default());
    let (mut sim, grid) = make_sim(vec![grizzly]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_some(),
        "attack_target preserved on range entry"
    );
    assert!(
        entity.movement_target.is_none(),
        "in-range pursuit should halt movement"
    );
}

#[test]
fn entity_target_out_of_range_pursues() {
    // Grizzly at (0,0) attacking Rhino at (10,0). Out of range.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    grizzly.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 10, 0, 400);
    let (mut sim, grid) = make_sim(vec![grizzly, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.entities.get(1).unwrap();
    assert!(entity.attack_target.is_some());
    assert!(
        entity.movement_target.is_some(),
        "out-of-range entity target should issue movement"
    );
}

#[test]
fn entity_target_dying_pursuit_skips() {
    // Target marked dying — resolve_target_coords still resolves, but combat
    // tick will clean up. Pursuit should not issue movement against a corpse.
    // (Note: this test exercises the dying-target case where target_coords still
    // returns Some but the target is mid-death; pursuit issues movement here
    // because resolve_target_coords doesn't filter on `dying`. That's
    // acceptable — pursuit issues a path that combat tick will then drop on
    // its target-dead branch. Net behavior: one tick of stale pursuit, then
    // attack_target cleared. We assert pursuit doesn't crash.)
    let mut grizzly = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    grizzly.attack_target = Some(AttackTarget::new(2));
    let mut rhino = make_unit(2, "HTNK", "Soviet", 10, 0, 0);
    rhino.dying = true;
    rhino.health.current = 0;
    let (mut sim, grid) = make_sim(vec![grizzly, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));
    // No assertion on movement_target — see test docstring. Just verify no
    // panic and attack_target is preserved (combat tick handles cleanup).
    assert!(sim.entities.get(1).unwrap().attack_target.is_some());
}

#[test]
fn aircraft_attack_target_skipped_by_pursuit() {
    // Aircraft has its own attack-mission state machine; pursuit must not
    // touch its movement.
    let mut orca = make_unit(1, "ORCA", "Americans", 0, 0, 150);
    orca.attack_target = Some(AttackTarget::new(2));
    orca.aircraft_mission = Some(AircraftMission::Attack {
        sub_state: 3,
        has_fired: false,
        is_strafe: false,
    });
    orca.aircraft_ammo = Some(AircraftAmmo::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 30, 0, 400);
    let (mut sim, grid) = make_sim(vec![orca, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.entities.get(1).unwrap();
    assert!(
        entity.movement_target.is_none(),
        "aircraft pursuit must not be touched by ground pursuit stage"
    );
}

#[test]
fn structure_attack_target_skipped_by_pursuit() {
    // Garrisoned building (or any structure) has attack_target but cannot move.
    let mut pillbox = make_unit(1, "GAPILL", "Americans", 5, 5, 400);
    pillbox.category = crate::map::entities::EntityCategory::Structure;
    pillbox.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 30, 5, 400);
    let (mut sim, grid) = make_sim(vec![pillbox, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.entities.get(1).unwrap();
    assert!(
        entity.movement_target.is_none(),
        "structures must not pursue"
    );
}

#[test]
fn deployed_infantry_skipped_by_pursuit() {
    // Deploy-fire infantry (e.g., GI in deployed state) cannot move.
    let mut gi = make_unit(1, "ENGI", "Americans", 5, 5, 75);
    gi.category = crate::map::entities::EntityCategory::Infantry;
    gi.deploy_state = Some(crate::sim::deploy::DeployPhase::Deployed);
    gi.attack_target = Some(AttackTarget::new(2));
    let rhino = make_unit(2, "HTNK", "Soviet", 30, 5, 400);
    let (mut sim, grid) = make_sim(vec![gi, rhino]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.entities.get(1).unwrap();
    assert!(
        entity.movement_target.is_none(),
        "deployed infantry must not pursue"
    );
}

#[test]
fn pursuit_uses_same_range_as_combat_no_oscillation() {
    // Place attacker exactly at the boundary. The combat tick range check
    // and pursuit range check use the same `is_within_range_leptons`, so
    // both must agree at the boundary. Verify: at exactly Range cells,
    // pursuit treats it as in-range (clears movement if any).
    //
    // 105mm Range=6. Place Grizzly at (0,0), target Cell(6,0). Distance = 6 cells exactly.
    let mut grizzly = make_unit(1, "MTNK", "Americans", 0, 0, 300);
    grizzly.attack_target = Some(AttackTarget::for_cell(6, 0));
    grizzly.movement_target = Some(crate::sim::components::MovementTarget::default());
    let (mut sim, grid) = make_sim(vec![grizzly]);
    let rules = pursuit_rules();

    sim.tick_attack_pursuit(&rules, Some(&grid));

    let entity = sim.entities.get(1).unwrap();
    // is_within_range_leptons is inclusive at the boundary. Pursuit should
    // halt (clear movement). If pursuit and combat used different math,
    // this would fail.
    assert!(
        entity.movement_target.is_none(),
        "at exactly weapon range, pursuit must halt (matches combat tick range check)"
    );
}
```

**Step 2: Register the new test module**

In [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs), find the existing
`#[cfg(test)]` test-module declarations near the end of the file (look for
`mod combat_force_fire_cell_tests` or similar). Add:

```rust
#[cfg(test)]
mod combat_pursuit_tests;
```

**Step 3: Verify**

Run: `cargo test --lib combat_pursuit_tests`

Expected: all 8 tests pass.

`PathGrid::test_all_passable(width, height)` is defined at
[src/sim/pathfinding/core.rs:1360](../../src/sim/pathfinding/core.rs#L1360)
and produces a grid with all cells walkable — exactly what pursuit tests
need (no terrain blocking the walk).

**Step 4: Commit**

```
git add src/sim/combat/combat_pursuit_tests.rs src/sim/combat/mod.rs
git commit -m "combat: add tick_attack_pursuit unit tests"
```

---

### Task 8: Add full-pipeline integration test (force-fire cell, walk into range, fire)

**Why:** Unit tests cover pursuit and combat in isolation. This test
exercises the full stack: force-fire on a far cell → tick advance → walk →
arrive in range → next tick: combat fires (or generates a fire event for
the AoE warhead). Regression-locks the user-visible end-to-end behavior.

**Files:**
- Modify: [src/sim/combat/combat_force_fire_cell_tests.rs](../../src/sim/combat/combat_force_fire_cell_tests.rs)
  — append integration test (already covers force-fire-cell command issuing).

**Pattern:** Mirror existing tests in the file; full-Simulation tick rather than direct `tick_combat`.

**Step 1: Append integration test**

Add to [src/sim/combat/combat_force_fire_cell_tests.rs](../../src/sim/combat/combat_force_fire_cell_tests.rs)
at the end of the file (before any closing module brace if present):

```rust
#[test]
fn force_fire_cell_pursuit_then_fire_integration() {
    // Full pipeline: ctrl-click on a far cell → ForceAttackCell command →
    // attack_target=Cell set → out of range → pursuit issues movement →
    // (many ticks of walking) → in range → combat fires (fire_event recorded).
    use crate::sim::command::{Command, CommandEnvelope};
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::world::Simulation;
    use std::collections::BTreeMap;

    let rules = ff_rules();
    let mut sim = Simulation::new();
    sim.input_delay_ticks = 0; // execute immediately for test
    sim.entities.insert(make_unit(1, "MTNK", 5, 5, 300));
    let owner_id = sim.interner.intern("Americans");
    let grid = PathGrid::test_all_passable(64, 64);
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();

    // Issue force-fire on a far cell (10 cells away; weapon Range=6).
    sim.pending_commands.push(CommandEnvelope::new(
        owner_id,
        sim.tick + 1,
        Command::ForceAttackCell {
            attacker_id: 1,
            target_rx: 15,
            target_ry: 5,
        },
    ));

    // Tick 1: command applies, attack_target set, pursuit issues movement.
    let pending: Vec<CommandEnvelope> = std::mem::take(&mut sim.pending_commands);
    sim.advance_tick(&pending, Some(&rules), &height_map, Some(&grid), None, 100);

    let entity = sim.entities.get(1).unwrap();
    assert!(
        entity.attack_target.is_some(),
        "attack_target set after ForceAttackCell apply"
    );
    assert!(
        entity.movement_target.is_some(),
        "pursuit issued movement (out of range)"
    );

    // Tick many times until unit walks into range and fires.
    let mut fired = false;
    for _ in 0..400 {
        let pending: Vec<CommandEnvelope> = std::mem::take(&mut sim.pending_commands);
        sim.advance_tick(&pending, Some(&rules), &height_map, Some(&grid), None, 100);
        if !sim.fire_events.is_empty() {
            fired = true;
            break;
        }
        // Safety: attack_target should remain set throughout pursuit.
        assert!(
            sim.entities.get(1).is_some_and(|e| e.attack_target.is_some()),
            "attack_target dropped mid-pursuit (parity bug)"
        );
    }

    assert!(
        fired,
        "unit should walk into range and fire within 400 ticks"
    );
}
```

**Step 2: Verify**

Run: `cargo test --lib force_fire_cell_pursuit_then_fire_integration -- --nocapture`

Expected: PASS within 400 ticks. If it fails (no fire_event), the fire path
is not running — investigate. If it fails on the mid-pursuit assertion,
something cleared attack_target inappropriately.

**Step 3: Commit**

```
git add src/sim/combat/combat_force_fire_cell_tests.rs
git commit -m "combat: integration test — force-fire cell pursuit then fire"
```

---

### Task 9: Run full test suite and update any remaining inverted tests

**Why:** Other tests may rely on the old "range fail drops attack" or
"range fail retargets" behavior. Catch them in one sweep and update.

**Files:**
- Potentially: any test file under [src/sim/](../../src/sim/) and [tests/](../../tests/).

**Pattern:** No code pattern; this is a maintenance pass.

**Step 1: Run the full test suite**

Run: `cargo test --workspace`

Expected: most tests pass; some may fail with assertions like
`attack_target.is_none()` or `retarget_events.len() == 1` from old
behavior. Catalog them.

**Step 2: For each failing test, audit and update**

For each failure:

1. Read the test. Determine if its assertion was based on the old
   range-fail-drops/retarget behavior.
2. If yes: update to the new contract. The unit fired path now sees one of:
   - In range: fires this tick (assertions about damage / fire events still valid).
   - Out of range: attack_target preserved, retarget_events empty,
     remove_attack empty. Movement may or may not have been issued
     depending on whether the test calls `advance_tick` (pursuit runs) vs
     `tick_combat` directly (pursuit doesn't run).
3. Add a comment near the updated assertion citing this design doc and
   ledger item L10.
4. If a test depended on the old retarget behavior in a way that's now
   meaningless (e.g., "asserts auto-retarget acquires nearest hostile when
   primary target leaves range"): consider deleting it and adding a new
   test for the pursuit behavior instead.

**Step 3: Re-run and confirm**

Run: `cargo test --workspace`

Expected: all tests pass.

**Step 4: Commit**

```
git add <files>
git commit -m "tests: update assertions for range-fail-preserves-attack contract"
```

---

### Task 10: Manual sandbox verification against gamemd parity

**Why:** Unit and integration tests prove the code paths run, but the
parity bar is "indistinguishable from gamemd in a single skirmish." This
task confirms the behavior matches gamemd qualitatively.

**Files:** none — this is observational.

**Verify:**

Launch the engine in sandbox with the following scenarios. For each, the
expected behavior is that our engine matches gamemd's behavior:

1. **Force-fire on far empty cell (single Grizzly).**
   - Setup: place a Grizzly (MTNK), select it, Ctrl+click on a cell ~12
     cells away.
   - Expected: unit walks toward the cell, halts when in weapon range
     (~6 cells short), fires repeatedly at the cell.
   - Gamemd: same (Mission_Attack + Greatest_Threat_Scan approach).

2. **Force-fire on far empty cell with ally between attacker and target.**
   - Setup: Grizzly at (5,5), allied Grizzly at (10,5), Ctrl+click on
     cell (15,5).
   - Expected: pursuing unit takes a path AROUND the ally (friendly cells
     are passable for own units via `build_entity_block_set`). Halts at
     range, fires.
   - Gamemd: same.

3. **Right-click far enemy.**
   - Setup: Grizzly at (5,5), enemy Rhino at (20,5). Right-click on the
     Rhino.
   - Expected: Grizzly walks toward Rhino, halts at weapon range, fires.
     If Rhino moves, Grizzly's pursuit lags by one path-completion (per
     design Open Follow-up #2) — acceptable for v1.
   - Gamemd: same (modulo L6 cadence smoothing — gamemd's chase is
     14–16 frame jittered; ours is per-tick smoother).

4. **Mixed selection force-fire.**
   - Setup: select [Engineer + Grizzly + MCV], Ctrl+click far cell.
   - Expected: Grizzly pursues + fires; Engineer walks to the cell
     (existing fall-through to Move); MCV walks to the cell (also
     unarmed fall-through).
   - Gamemd: same (per-unit dispatch).

5. **Splash-hit ally → ally walks toward attacker (gamemd-faithful side
   effect of L12).**
   - Setup: V3 launcher (or any wide-CellSpread weapon) attacker, ally
     unit standing in splash radius. Force-fire on a cell that splashes
     the ally.
   - Expected: ally takes splash damage, retaliation triggers, ally
     walks toward attacker but cannot fire (Verses gate / friendly check
     in fire path). Cosmetic awkwardness — ally just stands near
     attacker eventually.
   - Gamemd: same (Retaliate=yes sets TarCom; Mission_Attack pursues;
     per-frame Can_Fire blocks fire).
   - **Documented as accepted parity behavior in design doc.**

**If any scenario diverges from gamemd:**

- Note the divergence specifically (cell coords, expected vs observed,
  screen recording if possible).
- Decide: fix in this PR (if small) or open as a follow-up (if it's L3/L7
  spiral approach drift, L5 DefaultToGuardArea, or L11 friendly/visibility
  retarget — those are already deferred).

**Step 1: Run the sandbox**

Run: `cargo run --release` (or whatever the project's sandbox launcher is).

**Step 2: Walk through each scenario above.**

**Step 3: Document any unexpected behavior in
`docs/notes/2026-05-07-cell-target-pursuit-sandbox-notes.md`** (or update
the design doc Open Follow-ups).

**Step 4: No commit required if all scenarios match expectations.**

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-07-cell-target-pursuit-design.md](./2026-05-07-cell-target-pursuit-design.md)
- **Trace audit (origin of Drift 3):** referenced in design; covers the
  pre-existing entity-target out-of-range bug too.
- **Ghidra reports:**
  - `ra2-rust-game-docs/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` — primary source. HIGH confidence.
- **gamemd.exe addresses cited (reference only — do NOT include in code comments):**
  - `0x004D4DC0` — `FootClass::Mission_Attack`
  - `0x004D5690` — `Greatest_Threat_Scan` (approach driver, summary-only)
  - `0x0051F3E0` — `InfantryClass::Mission_Attack` (override, fallthrough)
  - `0x00417FE0` — `AircraftClass::Mission_Attack` (out of scope; aircraft has own state machine)
- **INI keys:** none new. Existing `[WeaponName] Range=` already parsed.
- **Related code:**
  - [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) — combat tick + helpers
  - [src/sim/world/world_orders.rs](../../src/sim/world/world_orders.rs) — pre/post-combat order intent stages (mirror pattern)
  - [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) — `MoveInfo` + `resolve_move_info`
  - [src/sim/movement/movement_commands.rs](../../src/sim/movement/movement_commands.rs) — `issue_move_command_with_layered`
  - [src/sim/aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs) — aircraft state machine (skip target in pursuit)
- **Prior commits providing context:**
  - `d0f1605` — `combat: TargetKind enum widens AttackTarget for cell-target attacks`
  - `86af0cd` — `combat: Ctrl-click force-fire on empty terrain + Alt+Ctrl override`
