# Cell-Target Movement-Into-Range (Pursuit) — Design

**Date:** 2026-05-07
**Status:** Approved (brainstorm complete; design ready for `/write-plan`)
**Scope:** Add gamemd-faithful pursuit for ground units with an `attack_target` that's
out of weapon range. Covers BOTH `TargetKind::Cell` (force-fire on terrain) and
`TargetKind::Entity` (player-issued Attack/ForceAttack, AI-issued attacks, retaliation).
Aircraft already handles its own pursuit via the 11-state attack mission.

## Goal

Units with a committed `attack_target` walk into weapon range and halt to fire,
matching gamemd's `Mission_Attack` → `Greatest_Threat_Scan` behavior. Closes Drift
3 from the 2026-05-07 force-fire trace audit and the parallel pre-existing
out-of-range bug for entity-target attacks.

## Architecture Context

### Where the gap lives today

The combat tick at [combat/mod.rs:1398-1413](../../src/sim/combat/mod.rs#L1398-L1413) handles the
"range check failed" case by calling `acquire_best_target` and either
auto-retargeting to a hostile entity or dropping the attack via `remove_attack`.
There is no branch that says "walk closer." This single branch is shared by
entity-target Attack/ForceAttack, the new ForceAttackCell, AI-issued attacks,
and retaliation.

`issue_attack_cell_command` ([combat/mod.rs:330](../../src/sim/combat/mod.rs#L330))
sets `attack_target` and explicitly *clears* `movement_target` (line 388). No
movement is issued anywhere — the unit just sits.

For aircraft, the equivalent `attack_mission.rs` state machine
([attack_mission.rs:142-181](../../src/sim/aircraft/attack_mission.rs#L142-L181))
already handles approach via state 3 issuing `move_to`. **Aircraft is out of
scope for this change** — no behavioral changes there.

### Existing patterns we'll parallel

- `tick_order_intents_pre_combat` ([world_orders.rs:24](../../src/sim/world/world_orders.rs#L24))
  is a method on `Simulation` that runs before combat and modifies entities in a
  single pass. The new `tick_attack_pursuit` will live next to it and follow the
  same shape (`pub(crate) fn …(&mut self, rules: &RuleSet, path_grid: Option<&PathGrid>)`).
- `tick_order_intents_post_combat` ([world_orders.rs:58](../../src/sim/world/world_orders.rs#L58))
  already calls `movement::issue_move_command_with_layered` from the world layer
  to resume `OrderIntent::AttackMove` paths after combat — same pattern, same
  call site, same imports.
- `app_context_order::try_queue_context_order_at_screen_point` clamps
  unwalkable goal cells via `nearest_walkable_cell_layered` before issuing a
  Move. Pursuit will mirror this clamp.

### gamemd model (FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md)

`FootClass::Mission_Attack` at `0x004D4DC0`:

1. Step 1: `DefaultToGuardArea` re-anchor for ground units (not modelled here).
2. Step 2: One-shot `HasFoundAutoTarget` finalization. Auto-acquire happens here, exactly once per acquisition cycle.
3. Step 3: If `TarCom != NULL`, call `Greatest_Threat_Scan` (the misnamed approach driver at `0x004D5690`). Discards return value.
4. Step 4: Return mission timer 14–16 frames.

The handler **does not** call `Fire_At` (firing happens in per-frame `UnitClass::AI`),
**does not** check weapon cooldowns, and **does not** clear `TarCom` on range failure.
`TarCom` is only cleared when the target dies/despawns, at which point step 3 calls
`OnArrival(0,1)` to transition to Guard.

The approach driver (`Greatest_Threat_Scan`):
- Spirals 8 directions from a bearing computed via `atan2(target − self)`.
- For each candidate cell, checks `InRange(cell, target, weapon)` + passability.
- First valid cell becomes destination via `Set_Destination`.
- If no cell found, falls back to `Set_Destination(TarCom, 1)` (raw target).

This design implements the **semantic** of step 3 (pursue when out of range,
preserve TarCom) at sim-tick rate, with a straight-line halt-on-range-entry
approximation of the spiral.

## Impact Analysis

| Layer | File | What changes |
|---|---|---|
| sim/world | [world_orders.rs](../../src/sim/world/world_orders.rs) | New `Simulation::tick_attack_pursuit` method (~80 LoC plus shared helper). |
| sim/world | [world/mod.rs:1313](../../src/sim/world/mod.rs#L1313) | Call new pursuit stage between `tick_order_intents_pre_combat` and `tick_combat_with_fog`. |
| sim/combat | [combat/mod.rs:1398-1413](../../src/sim/combat/mod.rs#L1398-L1413) | Replace range-fail retarget/drop branch with bare `continue;`. Range failure no longer modifies attack_target. |
| sim/combat | [combat/mod.rs](../../src/sim/combat/mod.rs) | Optional: extract a `pursuit_weapon_range` helper that BOTH the new pursuit stage AND the combat tick range-check consume, ensuring inputs match (L17/L18 hysteresis fix). |
| Tests | [combat/combat_tests.rs:323 `test_tick_combat_out_of_range`](../../src/sim/combat/combat_tests.rs#L323) | Inverts: attack_target preserved, no retarget event. |
| Tests | new file `combat/combat_pursuit_tests.rs` (or append to world_tests.rs) | New unit tests for pursuit semantics. |

### Dependencies on what we're changing

- **`tick_retaliation`** ([combat_targeting.rs:254](../../src/sim/combat/combat_targeting.rs#L254))
  sets `attack_target` on idle units that were hit. After this change those units
  walk toward the attacker (gamemd-faithful, see L12).
- **Garrison auto-acquire** ([combat/mod.rs:937-1078](../../src/sim/combat/mod.rs#L937-L1078))
  sets `attack_target` on garrisoned buildings. Pursuit must skip structures (they
  can't move; L15).
- **AI** ([ai.rs](../../src/sim/ai.rs)) sets `attack_target` directly on AI units.
  Pursuit applies — consistent with intent.
- **Aircraft state machine** ([aircraft/attack_mission.rs](../../src/sim/aircraft/attack_mission.rs))
  has its own pursuit. Filter aircraft out of the new stage (L14).

### Migration concerns

- **No new struct fields, no new components, no new enum variants.** Pursuit
  state is implicit in `attack_target` + `movement_target` + entity position.
- **No snapshot version bump.** State hash unchanged.
- **Replay determinism preserved** — same inputs, same per-tick decisions.

### Determinism / state-hash

- Iteration over `self.entities.keys_sorted()` (deterministic).
- All math via existing fixed-point helpers (`lepton_distance_sq_raw`,
  `is_within_range_leptons`, `cell_center_coords`, `target_coords`,
  `select_weapon_with_ifv`).
- Pursuit reads existing fields and writes only `movement_target`. State hash
  already covers `movement_target`.
- No new global state. No new tick-order changes beyond the new stage call.

### Blast radius

Medium. The change is scoped to one new function + one branch flip in combat,
but every attack pipeline funnels through the load-bearing branch. Tests will
catch most regressions; a sandbox skirmish run is recommended after
implementation.

## Chosen Approach

**Approach A — straight-line pursuit + halt-on-range.** New pre-combat stage
issues movement toward target when out of range, clears movement when in range.
Combat tick range-fail branch becomes a no-op `continue;`. No new components, no
new enum variants, no snapshot bump.

Rejected alternatives (see "Alternatives Considered"):
- **Approach B** — gamemd-faithful 8-direction spiral approach driver.
  Asymptotically more parity-correct but 3-5× the LoC and open questions in the
  source decompile. A's straight-line halt approximates B's endpoint within 1-2
  cells for typical geometries; ship A first, B as cosmetic follow-up if a
  player notices.
- **Approach C** — A plus delete friendly/visibility retarget branches at
  [combat/mod.rs:1346, 1362](../../src/sim/combat/mod.rs#L1346). Strictly
  parity-correct (gamemd preserves TarCom across friendly + visibility cases too,
  per FOOTCLASS doc §2.2) but bundles two design decisions; better as a separate
  brainstorm.

## Tiny-Detail Ledger

Sourced from `ra2-rust-game-docs/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
(decompile of `FootClass::Mission_Attack` at `0x004D4DC0` and
`Greatest_Threat_Scan` at `0x004D5690`).

| # | Detail | Source | Where it lives in this design |
|---|---|---|---|
| L1 | Auto-acquire is one-shot per acquisition cycle, not per tick. | [GHIDRA 0x004D4DC0 step 2] | Our `acquire_best_target_for_entity` only fires when `attack_target.is_none()` — already correct. ✓ |
| L2 | Range failure does NOT clear `TarCom`. Only target death/despawn does. | [GHIDRA 0x004D4DC0 step 3 + §2.2] | Combat tick range-fail branch becomes `continue;`. The "target dead" branch ([mod.rs:1280-1298](../../src/sim/combat/mod.rs#L1280-L1298)) stays. |
| L3 | Approach driver picks an in-range passable cell via 8-direction spiral. | [GHIDRA 0x004D5690 §3 step 5] | **Known cosmetic drift** — Approach A walks straight, halts on range entry. May be 1-2 cells closer than gamemd at endpoint. |
| L4 | `Greatest_Threat_Scan` early-returns 0 when `WarpedOutOf == TarCom`. | [doc §3 step 1] | **Niche.** Chrono-out-of mid-pursuit; our chrono system already short-circuits. Out of scope. |
| L5 | `DefaultToGuardArea` re-anchors ground units to a nearby passable cell each dispatch. | [GHIDRA 0x004D4DC0 step 1] | **Known parity drift** — not modelled. Affects patrol-style units. Tracked as follow-up. |
| L6 | Mission_Attack dispatch cadence: 14–16 frames jittered, halve to 7 if close + short-range. | [GHIDRA 0x004D4DC0 step 4-5] | **Pre-existing carry-over drift** flagged in FOOTCLASS doc §10. Our pursuit runs every sim tick. Not addressed here. |
| L7 | Approach driver uses bearing `atan2(target − self) >> 7` and 8-direction spiral with passability + sub-cell positioning. | [doc §3 step 5] | **Same drift as L3.** Skipped under Approach A. |
| L8 | Combat firing path runs every frame, decoupled from Mission_Attack dispatch. | [doc §2 + §5] | Same pre-existing cadence drift as L6. |
| L9 | `Set_Destination` 2nd-arg force flag (1 vs 0). | [doc §2/§3] | Our `issue_move_command_with_layered` always replaces; behavior similar enough. ✓ |
| L10 | Range fail does not retarget. Per-frame `Can_Fire` blocks fire if alliance/Verses say no; `TarCom` is preserved. | [doc §2.2 + §3 step 6] | **Load-bearing change.** Combat tick range-fail at [mod.rs:1398](../../src/sim/combat/mod.rs#L1398) becomes `continue;`. |
| L11 | Friendly-fire and visibility branches at [combat/mod.rs:1346, 1362](../../src/sim/combat/mod.rs#L1346) similarly should not retarget per gamemd. | [doc §2.2 + §9] | **Existing drifts unchanged.** Out of scope per agreement; flagged for separate brainstorm. |
| L12 | `Retaliate=yes` triggers TarCom set; pursuit applies even if can't fire (e.g., ally splash). | [doc §2.2 + §7] | **Accepted side effect** of option (b). Retaliating units now pursue including against allies they can't damage. Document in user-visible behavior notes. |
| L13 | Halt condition: gamemd's approach driver sets destination to a cell where `InRange(cell, target, weapon)` is true; unit walks there and stops. | [doc §3 step 5] | Approach A's halt: combat tick clears `movement_target` when our range check passes from current position. Equivalent endpoint for straight-line pursuit. ✓ |
| L14 | Aircraft attack runs in its own 11-state machine; ground pursuit must not touch aircraft. | [doc §4.3] | Skip filter: `entity.aircraft_mission.is_some()` OR `movement_layer_or_ground() == Air`. |
| L15 | Garrisoned structures cannot move; pursuit must skip. | [combat/mod.rs:937-1078](../../src/sim/combat/mod.rs#L937-L1078) | Skip filter: `entity.category == Structure`. |
| L16 | Deployed-fire units (`DeployFire=yes` GI/GGI in deployed state) cannot move. | [sim/deploy.rs](../../src/sim/deploy.rs) | Skip filter: `entity.is_deployed()`. |
| L17 | Range check uses lepton-precise `lepton_distance_sq_raw` against weapon range from `select_weapon_with_ifv`. | [combat/mod.rs:1380-1397](../../src/sim/combat/mod.rs#L1380-L1397) | Pursuit and combat tick MUST use identical math and identical weapon-select inputs. **Hysteresis hazard if they diverge.** |
| L18 | `target_coords` returns foundation-center for buildings, NW corner for units, cell-center for cells. | [combat/mod.rs:202-258](../../src/sim/combat/mod.rs#L202-L258) | Pursuit must use the existing `resolve_target_coords` helper. Mismatch with combat tick → unit walks to NW corner of building, combat says "out of range from foundation center" → infinite oscillation. |

### Accepted drifts (carry-over to user-visible parity table)

- **L3, L7:** straight-line pursuit instead of spiral approach cell. Cosmetic
  1-2 cell drift at endpoint.
- **L5:** `DefaultToGuardArea` re-anchor not modelled. Affects patrol units.
- **L6, L8:** dispatch cadence at sim-tick rate, not 14–16 frames. Pre-existing.
- **L11:** friendly-fire / visibility retargets for entity attacks unchanged.
  Pre-existing, flagged for separate brainstorm.
- **L12:** retaliating units now pursue, including against allies they cannot
  damage. Gamemd-faithful per option (b).

## Design

### Components

| Component | Lives in | Role |
|---|---|---|
| `Simulation::tick_attack_pursuit(&mut self, rules: &RuleSet, path_grid: Option<&PathGrid>)` | [src/sim/world/world_orders.rs](../../src/sim/world/world_orders.rs) | New pre-combat stage. Iterates entities with `attack_target`, issues movement when out of range, halts when in range. Skips structures, aircraft, deployed, in-transport. |
| Combat tick range-fail branch | [src/sim/combat/mod.rs:1398-1413](../../src/sim/combat/mod.rs#L1398-L1413) | Replaced with `continue;`. Range failure no longer retargets or removes attack_target. |
| Optional shared helper `pursuit_weapon_range` | [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs) | Single source of truth for "what weapon range applies to this attacker against this target?" — consumed by both the pursuit stage and the combat tick range check. Eliminates L17/L18 hysteresis risk. |
| Integration point | [src/sim/world/mod.rs:1313](../../src/sim/world/mod.rs#L1313) | New tick-order line: `self.tick_attack_pursuit(rules, path_grid);` between `tick_order_intents_pre_combat` and `tick_combat_with_fog`. |

No new modules, no new components, no new enum variants, no new fields.

### Interfaces / Contracts

```rust
impl Simulation {
    /// Pre-combat: entities with an `attack_target` that's out of weapon range
    /// walk toward the target. Entities that just entered range halt their
    /// movement so the combat tick can fire from a stationary position.
    ///
    /// Mirrors gamemd `Mission_Attack` step 3 (`Greatest_Threat_Scan` approach
    /// driver) — TarCom is preserved while pursuing; range failure does NOT
    /// retarget. Auto-acquire/retaliation set attack_target via other paths;
    /// pursuit consumes whatever target is committed.
    ///
    /// Skips entities that can't or shouldn't pursue:
    /// - Structures (can't move)
    /// - Aircraft (own state machine in attack_mission.rs)
    /// - Deployed-fire infantry (locked while deployed)
    /// - Entities inside transports
    pub(crate) fn tick_attack_pursuit(
        &mut self,
        rules: &RuleSet,
        path_grid: Option<&PathGrid>,
    );
}
```

The combat-side helper signature (if extracted):

```rust
/// Resolve the effective weapon range for an attacker against a target.
/// Used by both the pursuit pre-combat stage and the combat tick range
/// check, so range decisions stay consistent.
pub(crate) fn pursuit_weapon_range(
    entity: &GameEntity,
    target: &TargetKind,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
) -> Option<SimFixed>;
```

No public API changes outside sim/. Render and UI consume the same EntityStore
shape; only the contents of `movement_target` differ when pursuit is active.

### Data Flow

```
Tick T:
  apply_command (if any) → may set attack_target + clear movement_target
  ground/air/special movement → may carry pursuit-set movement_target
  vision refresh
  power
  superweapons / deploy
  turret rotation
  capture orders
  tick_order_intents_pre_combat → AttackMove auto-acquire (sets attack_target on no-target idle)
  tick_attack_pursuit                                                ← NEW
    for each entity with attack_target.is_some():
      if structure / aircraft / deployed / in-transport: skip
      resolve target coords (Entity → target_coords; Cell → cell_center_coords)
        if entity-target despawned: skip (combat handles cleanup)
      resolve weapon range via pursuit_weapon_range (shared with combat)
        if no engaging weapon: skip (combat will drop next tick)
      compute lepton_distance_sq_raw
      if dist² > range²:
        if movement_target.is_none():
          clamp goal to nearest walkable cell
          issue_move_command_with_layered toward target_cell
        else: leave existing pursuit movement
      else (in range):
        if movement_target.is_some(): clear it (halt for firing)

  tick_combat_with_fog
    Phase 1: snapshot attackers (cooldowns advance)
    Phase 2: per attacker
      ...weapon select...
      friendly-fire / visibility / target-dead retarget arms (unchanged)
      range check:
        if range fail: continue;                                     ← CHANGED
      cooldown / burst gate
      turret-aligned gate
      fire shot: damage_event / AoE / fire_event / wall / bridge / ore
      burst / cooldown update
    Phase 3+: apply retargets, burst, ammo, deaths
```

Invariants:
- `attack_target` is preserved across pursuit ticks. Only target death/despawn
  (via existing combat-tick branch) or explicit player override clears it.
- `movement_target` carries pursuit motion; cleared when in range.
- `pursuit_weapon_range` is the single source of truth for range decisions
  (eliminates L17/L18 hysteresis).
- Pursuit and combat run in the same tick, in that order. Halt-on-range-entry
  happens the same tick combat fires.

### Error Handling

| Case | Behavior |
|---|---|
| Target entity died this tick | `resolve_target_coords` returns None → pursuit skips. Combat tick's existing `target_data` lookup at [mod.rs:1226](../../src/sim/combat/mod.rs#L1226) returns None for dead targets, falls into the "hp > 0" arm at line 1280, which still retargets-or-drops. **This branch stays** — only the range-fail branch flips. |
| Cell out of map bounds | Coordinate is `u16`; map bounds are checked by the path grid. A* fails → `issue_move_command_with_layered` returns false → unit doesn't move. Pursuit retries each tick (perf concern, see Open Follow-ups). |
| Target cell unreachable (water/cliff for non-amphibious) | A* finds best partial path → unit walks until path exhausted → arrives short of range → next tick re-issues same path. **A* thrash on stuck pursuits.** Mitigation deferred (see Open Follow-ups). |
| Weapon-select returns None (Verses 0 or AG/AA mismatch) | Pursuit skips; combat tick will drop attack on its weapon-select fail at [mod.rs:1316/1331](../../src/sim/combat/mod.rs#L1316). Consistent. |
| Attacker has no weapon at all | Same as above — weapon-select returns None → skip. |
| Pursuit-clear of movement that was player-set | At pursuit time, `attack_target` was set by `apply_command`, which already cleared the old `movement_target` ([mod.rs:316, 388](../../src/sim/combat/mod.rs#L316)). Any current `movement_target` is therefore pursuit-set. No conflict. |
| Multiple selected units force-firing same cell | Each unit pursues independently. Pile-up at target cell handled by existing scatter / occupancy systems. Same as concurrent Move commands. |

### Testing Strategy

**New unit tests** (in `src/sim/combat/combat_pursuit_tests.rs` or appended to `world_tests.rs`):

1. `cell_target_out_of_range_issues_movement` — Grizzly at (5,5),
   `attack_target=Cell(15,15)`, range=5. After `tick_attack_pursuit`,
   `movement_target.is_some()` and goal is reachable.
2. `cell_target_in_range_clears_movement` — Grizzly at (8,5) with
   `movement_target` set, `attack_target=Cell(10,5)`, in range. After tick,
   `movement_target.is_none()`.
3. `entity_target_out_of_range_pursues` — same as (1) but `TargetKind::Entity`.
4. `entity_target_dies_pursuit_skips` — target removed/dying → pursuit no-ops;
   combat tick handles cleanup.
5. `aircraft_attack_target_skipped` — aircraft with `attack_target` →
   pursuit doesn't touch movement (aircraft has its own state machine).
6. `garrisoned_building_skipped` — structure with `attack_target` → pursuit
   doesn't touch movement.
7. `deployed_infantry_skipped` — `deploy_state == Deployed` → skipped.
8. `combat_tick_no_longer_drops_on_range_fail` — invert
   [combat_tests.rs:323 `test_tick_combat_out_of_range`](../../src/sim/combat/combat_tests.rs#L323).
   After combat tick: `attack_target` preserved, `retarget_events` empty,
   `remove_attack` empty.
9. `pursuit_then_fire_in_range_integration` — full path: out-of-range
   force-fire-cell → walk → in range → next tick: combat fires (verify
   `fire_events` recorded for AoE warhead, or `damage_event` for
   non-AoE entity target).
10. `pursuit_uses_same_range_as_combat` — set up an attacker exactly at
    `range + 0.1` cells from target. Pursuit issues movement (out of range).
    Move attacker to exactly `range - 0.1` cells. Pursuit clears movement,
    combat fires. Hysteresis check: at exactly `range`, behavior is consistent.

**Existing-test inversions:**
- [combat_tests.rs:323 `test_tick_combat_out_of_range`](../../src/sim/combat/combat_tests.rs#L323):
  expects unit to drop attack when out of range. Update to expect attack
  preserved.
- Audit other tests in `combat_tests.rs` and `world_tests.rs` that rely on the
  old retarget-on-range-fail behavior. (Search for `out_of_range` test names
  and `retarget_events`/`remove_attack` assertions.)

**Sandbox / manual:**
- Force-fire on far cell with single Grizzly. Verify it walks until in range,
  then fires.
- Mixed selection (Grizzly + Engineer). Force-fire on far cell. Grizzly walks
  toward cell + fires; Engineer walks to cell (doesn't fire). Existing
  per-unit dispatch covers this.
- Right-click far enemy unit. Verify Grizzly pursues into weapon range.
- Hit an ally with splash. Verify ally walks toward attacker but doesn't fire
  (gamemd-faithful side effect of L12).
- Run the existing 1681 unit tests; expect 1-3 inverted tests to need fixing,
  no other regressions.

### Determinism Considerations

- No new struct fields, no `#[derive]` changes.
- Iteration over `self.entities.keys_sorted()` (deterministic).
- All math via existing fixed-point helpers.
- Pursuit reads existing fields and writes only `movement_target`. State hash
  already covers `movement_target`.
- Replay test (existing snapshot tests cover the framework): a session with
  out-of-range force-fires should produce identical state hash on replay.

## Architectural Decisions

- **Pattern followed:** new method on `Simulation` next to existing
  `tick_order_intents_pre_combat` / `tick_order_intents_post_combat`. Same
  module, same call shape, same call site.
- **Pattern followed:** sim/movement (`issue_move_command_with_layered`)
  invoked from sim/world layer (already done by post-combat order resume).
- **Pattern deviation:** none.
- **Tech debt acknowledged:**
  - L6 dispatch cadence: pursuit runs at sim-tick rate, gamemd at 14–16 frames.
    Pre-existing carry-over from FOOTCLASS doc §10. Documented; not addressed.
  - L3/L7 approach-cell spiral: straight-line approximation. Cosmetic 1-2 cell
    drift at endpoint.
  - L5 `DefaultToGuardArea` re-anchor: not modelled. Affects patrol units.
  - L11 friendly/visibility retargets remain. Out of scope; separate brainstorm.

## Open Follow-ups

1. **A\* thrash on unreachable targets.** When pursuit lands "as close as A\*
   could get" but unit is still out of range, every tick re-invokes A*. Cheap
   for 1–10 stuck units; pathological for 50+. Mitigation options when needed:
   (a) cache A\* result by `(origin, goal)` for N ticks; (b) per-entity dispatch
   stagger `tick % 14 == stable_id % 14` that approximately matches the L6
   gamemd cadence; (c) gate on entity position changing since last issue.
2. **Target-moved-while-pursuing not re-evaluated.** Unit walks to where the
   entity target was, finishes path, finds target moved, re-issues to current
   position. Lags by one path-completion vs gamemd's mid-walk re-aim
   (Greatest_Threat_Scan §6 NavCom-non-null branch).
3. **Spiral approach cell (L3/L7).** Promote Approach A to Approach B if
   players notice the 1-2 cell endpoint drift.
4. **`DefaultToGuardArea` re-anchor (L5).** Implement step 1 of Mission_Attack
   for ground units with `DefaultToGuardArea=yes` so force-fire fires once and
   drifts back to anchor.
5. **Friendly / visibility retarget removal (L11).** Separate brainstorm.
   Currently combat tick retargets when target becomes invisible or alliance
   flips. Gamemd preserves TarCom and lets per-frame `Can_Fire` block; we should
   match.
6. **Mission timer / dispatch cadence (L6/L8).** Move pursuit and approach
   decisions to a 14–16 frame jittered timer (per-entity) instead of every-tick.
   Significantly larger architectural change.

## Alternatives Considered

- **Approach B — gamemd-faithful 8-direction spiral approach driver.**
  Bearing-from-target compass spiral, in-range cell search, sub-cell positioning
  for infantry. Closer to gamemd; closes L3/L7. Rejected as initial scope:
  3-5× LoC, several open questions in the source decompile (doc §11). Track as
  Open Follow-up #3.
- **Approach C — Approach A plus delete friendly-fire and visibility retarget
  branches.** Strictly parity-correct (gamemd preserves TarCom across these
  cases per FOOTCLASS doc §2.2) but bundles two design decisions into one
  change. Rejected as initial scope; track as Open Follow-up #5.
- **Synthetic ground-target entity.** Spawn a hidden entity at the target cell
  for pursuit-to-entity reuse. Rejected during force-fire-cell brainstorm
  (2026-05-07-force-fire-ctrl-click-design.md). Same rejection applies here:
  pollutes EntityStore, every iterator has to filter.
- **Per-tick re-acquire on range fail (current behavior).** This is the bug
  being fixed. Documented for completeness.

## References

- `ra2-rust-game-docs/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`
- `docs/plans/2026-05-07-force-fire-ctrl-click-design.md` — preceding design
- `docs/plans/2026-05-07-force-fire-ctrl-click-plan.md` — preceding plan
- `docs/gap-scans/2026-05-07-gap-scan.md` — context for trace audit
