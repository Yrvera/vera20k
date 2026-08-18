# Building Damage-State Gate Implementation Plan

> Execute this plan task-by-task. Do not commit or push unless the user explicitly asks.

## Goal

Replace the render-derived occupied-building damaged-state gate with a stored, sim-owned bool-like gate matching the proven native `BuildingClass+0x534 == 0/!=0` behavior for the scoped visual paths.

## Design Doc

[docs/plans/2026-05-27-building-damage-state-gate-design.md](2026-05-27-building-damage-state-gate-design.md)

## Grounding Summary

Primary contract: [docs/contracts/2026-05-27-building-bstate-visual-state-implementation-contract.md](../contracts/2026-05-27-building-bstate-visual-state-implementation-contract.md)

Primary evidence:

- `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md`: `GetCurrentFrame` reads `BuildingClass+0x534` before `CanBeOccupied`; zero returns raw body frame without occupant inspection; nonzero enters occupied frame formula.
- `docs/research/BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`: damaged active/idle anim variant selection uses `ConditionYellow`, not `ConditionRed`.
- Live Ghidra in the contract: `BuildingClass__ReceiveDamage @ 0x00442230` writes the damaged-state flag after nonzero damage using `GetHealthRatio() <= ConditionYellow`.
- Live Ghidra in the contract: `BuildingClass__EngineerRepair @ 0x00701473` restores health and calls `SetDamagedState`.

Stale or superseded warning:

- Older garrison frame-swap planning that applies occupied body-frame formula from occupancy alone is superseded by the BState-gated evidence above. Do not restore healthy occupied body frame `2`.

## Key Decisions

- Store only a bool-like gate: `building_damage_state_active`.
- Do not model a rich `BuildingBState` enum in this patch.
- Put the refresh helper on `GameEntity`, because the state and health/category data live there and all writer paths already hold mutable entities.
- Use deterministic integer comparison with `condition_yellow_x1000`.
- Render receives the stored field; render does not derive this gate from health.
- False gate returns raw/current body frame, not hardcoded frame `0`.
- Include direct combat, AoE combat, C4, Lightning Storm, and Rust building repair writer paths.
- Audit service-depot healing during implementation. The module documents unit repair; if it cannot heal structures, leave it out with a note. If it can heal structures, call the helper there too.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/game_entity.rs` | Add stored field, constructor default, refresh helper, focused helper tests |
| Modify | `src/sim/world/world_hash.rs` | Include field in deterministic hash; add hash test |
| Modify | `src/app_instances/shp.rs` | Replace render-side health proxy with stored gate; preserve raw/current frame fallback; update render tests |
| Modify | `src/sim/combat/mod.rs` | Refresh gate after direct and AoE structure damage |
| Modify | `src/sim/world/world_orders.rs` | Refresh gate after C4 building damage |
| Modify | `src/sim/superweapon/lightning_storm.rs` | Refresh gate after Lightning Storm structure damage |
| Modify | `src/sim/production/production_sell.rs` | Refresh gate after building repair tick heals |
| Audit | `src/sim/docking/building_dock.rs` | Confirm service-depot healing is unit/aircraft only, or refresh if structures can be healed |

## Interface Changes

- Add `GameEntity::building_damage_state_active: bool`.
- Add `GameEntity::refresh_building_damage_state_gate(condition_yellow_x1000: i64) -> bool`.
- Change private `rendered_garrison_body_frame_index` in `src/app_instances/shp.rs` to accept:
  - raw/current body frame;
  - stored gate bool;
  - existing occupant/health/tech/threshold inputs for formula once gate is active.

No public crate API is intended to change.

## Sim Checklist

- [ ] New state is serialized with `#[serde(default)]`.
- [ ] New state is initialized in `GameEntity::new`.
- [ ] New state is included in `Simulation::state_hash`.
- [ ] New helper uses integer math only.
- [ ] `sim/` does not import render/ui/audio/sidebar/net.
- [ ] Every in-scope structure health mutation refreshes the gate before any render-visible snapshot can read stale state.
- [ ] Repair timing caveat remains documented; this patch does not claim full native player-repair timing parity.

## Risk Areas

| Risk | Mitigation |
|---|---|
| Accidentally treating the field as full `+0x534` BState | Name it `building_damage_state_active`; document it as scoped zero/nonzero damaged-state gate only. |
| Stale render helper still derives from health | Delete `building_bstate_damage_active` or leave no production caller; tests must prove yellow health plus false stored gate returns raw frame. |
| Missing writer path | Add direct/AoE/C4/Lightning/repair tests; keep service-depot audit explicit. |
| Raw frame collapsed to hardcoded `0` | Make render helper take raw/current body frame and test false gate with a nonzero raw frame. |
| Anim variant still follows health | Add tests where health and stored gate disagree for damaged and garrisoned anim variant selection. |

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Stored per-building gate exists | Native stores state; deriving it from health at read time is known drift. | Helper tests and serde/default test. |
| 2 | Hash includes gate | Same health with different stored gate can render differently. | `state_hash` differs when only the gate differs. |
| 3 | False gate skips occupied formula | Healthy and explicitly false-gate occupied buildings must not use body frame 2. | `occupied_cagas01_yellow_bstate_false_stays_raw_frame` with nonzero raw frame. |
| 3 | True gate preserves formula | Yellow/red occupied civilian outputs remain frame 2/frame 1. | Existing formula tests updated with explicit true gate. |
| 4-6 | Writers refresh state after health mutation | Damage/repair paths can cross threshold; stale gate causes wrong pixels. | Writer-specific tests. |
| 3 | Anim variants use stored gate | Damaged/garrisoned variant selection is also gated by native damaged state. | Stored-state disagreement tests for `selected_building_anim_view` or `emit_building_anims`. |

---

## Tasks

### Task 1: Add stored gate and refresh helper on `GameEntity`

**Why:** Establish the native-like per-instance state before any writer or render path consumes it.

**Files:**

- Modify: `src/sim/game_entity.rs`

**Steps:**

1. Add a field near the other building-specific fields:

```rust
#[serde(default)]
pub building_damage_state_active: bool,
```

2. Initialize it to `false` in `GameEntity::new`.

3. Add a method on `GameEntity`:

```rust
pub fn refresh_building_damage_state_gate(&mut self, condition_yellow_x1000: i64) -> bool
```

Required behavior:

- no-op and return `false` for non-structures;
- `health.max == 0` means inactive;
- otherwise active when `current * 1000 <= max * condition_yellow_x1000`;
- use widened integer math;
- return whether the stored field changed.

4. Add focused tests in `game_entity.rs`:

- non-structure stays false even below yellow;
- structure above yellow stays false;
- structure exactly at yellow sets true;
- structure below yellow sets true;
- structure repaired above yellow clears true;
- `max == 0` clears/stays false;
- serde round trip preserves true;
- serde default works by serializing a `GameEntity` to `serde_json::Value`, removing `building_damage_state_active`, deserializing, and asserting false.

**Validation:**

Run:

```powershell
cargo test -q building_damage_state
```

Expected: new helper/serde tests pass.

### Task 2: Hash the stored gate

**Why:** The gate changes pixels and native-equivalent state. Lockstep hash must distinguish it.

**Files:**

- Modify: `src/sim/world/world_hash.rs`

**Steps:**

1. Add `entity.building_damage_state_active.hash(hasher);` near the existing entity health/category fields.

2. Add a state-hash test near the existing entity hash tests:

- create two otherwise identical simulations/entities;
- set the gate true on one structure only;
- assert `state_hash()` differs;
- optionally assert non-structure false/default remains equal.

**Validation:**

Run:

```powershell
cargo test -q building_damage_state_changes_state_hash
```

Expected: hash changes only when the stored gate differs.

### Task 3: Replace render-side gate derivation

**Why:** This closes the player-visible bug: render should read stored state, not recompute the native gate from health.

**Files:**

- Modify: `src/app_instances/shp.rs`

**Steps:**

1. Change `rendered_garrison_body_frame_index` to take a raw/current body frame and stored gate bool.

Suggested shape:

```rust
fn rendered_garrison_body_frame_index(
    raw_body_frame: u16,
    building_damage_state_active: bool,
    occupant_count: u32,
    health_current: u16,
    health_max: u16,
    tech_level: i32,
    condition_yellow: f32,
    condition_red: f32,
) -> u16
```

2. If `building_damage_state_active` is false, return `raw_body_frame`.

3. If true, keep the existing occupied frame formula.

4. Update the structure body call site to pass raw frame `0` for the current Rust body path and `entity.building_damage_state_active` for the gate.

5. Update building anim emission to pass `entity.building_damage_state_active` into `emit_building_anims`.

6. Remove `building_bstate_damage_active` if it has no remaining production caller.

7. Update/add render tests:

- `occupied_healthy_cagas01_bstate_zero_renders_frame_zero`;
- `occupied_cagas01_yellow_bstate_false_stays_raw_frame` using raw frame `7` or another nonzero sentinel;
- `occupied_cagas01_yellow_bstate_true_uses_frame_two`;
- `occupied_cagas01_red_bstate_true_collapses_to_frame_one`;
- `damaged_active_anim_variant_follows_stored_gate_not_health`;
- `garrisoned_active_anim_variant_follows_stored_gate_not_health`.

**Validation:**

Run:

```powershell
cargo test -q occupied_cagas01
cargo test -q damaged_active_anim_variant_follows_stored_gate_not_health
cargo test -q garrisoned_active_anim_variant_follows_stored_gate_not_health
```

Expected: false gate returns raw frame, true gate preserves existing formula, anim variant tests follow stored gate.

### Task 4: Refresh gate after combat damage

**Why:** Native `ReceiveDamage` writes the damaged-state gate after nonzero damage. Rust direct and AoE combat currently mutate health without a writer.

**Files:**

- Modify: `src/sim/combat/mod.rs`

**Steps:**

1. After direct target health subtraction, call:

```rust
target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
```

The method handles non-structures, so no extra category branch is required unless local style prefers it.

2. After AoE target health subtraction, call the same helper.

3. Do not call it for invulnerable hits where health did not change.

4. Add focused combat tests:

- direct damage from 60/100 to 50/100 sets true;
- direct damage that does not cross threshold leaves prior false unchanged;
- AoE damage crossing threshold sets true.

**Validation:**

Run:

```powershell
cargo test -q combat_damage_crossing_condition_yellow_sets_building_damage_state
cargo test -q aoe_damage_crossing_condition_yellow_sets_building_damage_state
```

Expected: all pass.

### Task 5: Refresh gate after C4 and Lightning Storm damage

**Why:** These are active Rust health mutation paths outside `combat/mod.rs`; missing them would leave stale gates after player-visible damage.

**Files:**

- Modify: `src/sim/world/world_orders.rs`
- Modify: `src/sim/superweapon/lightning_storm.rs`

**Steps:**

1. In C4 building damage, call the helper immediately after assigning `b.health.current = new_hp` and before death/dying branching.

2. In Lightning Storm damage application, call the helper after subtracting health.

3. Add or extend tests:

- C4 damage crossing yellow sets gate true;
- Lightning Storm damage crossing yellow sets gate true for structures;
- invulnerable/no-damage cases do not set it.

**Validation:**

Run:

```powershell
cargo test -q c4_damage_crossing_condition_yellow_sets_building_damage_state
cargo test -q lightning_storm_crossing_condition_yellow_sets_building_damage_state
```

Expected: all pass.

### Task 6: Refresh gate after repair/heal paths

**Why:** Once buildings can enter damaged visual state, repair must clear the gate when health rises above `ConditionYellow`.

**Files:**

- Modify: `src/sim/production/production_sell.rs`
- Audit: `src/sim/docking/building_dock.rs`

**Steps:**

1. In `tick_repairs`, after `entity.health.current = ...`, call:

```rust
entity.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
```

2. Audit `tick_building_docks`:

- the module header says repair depot unit repair;
- if local types guarantee it only heals non-structures, no code change is required;
- if there is any structure heal path, call the same helper after healing.

3. Add repair test:

- building starts below/equal yellow with `building_damage_state_active = true`;
- repair raises it above yellow;
- gate clears false.

If service-depot healing is excluded, add a short test or comment-backed assertion naming it unit/aircraft-only.

**Validation:**

Run:

```powershell
cargo test -q building_repair_crossing_above_condition_yellow_clears_building_damage_state
```

Expected: repair clears the stored gate.

### Task 7: Focused regression sweep

**Why:** The change touches sim state, render extraction, and multiple health writers. Run the narrow tests first, then one compile check.

**Before running Cargo:**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
```

If another active session owns Cargo, wait or ask before starting.

**Focused tests:**

```powershell
cargo test -q building_damage_state
cargo test -q occupied_cagas01
cargo test -q damaged_active_anim_variant_follows_stored_gate_not_health
cargo test -q garrisoned_active_anim_variant_follows_stored_gate_not_health
cargo test -q combat_damage_crossing_condition_yellow_sets_building_damage_state
cargo test -q aoe_damage_crossing_condition_yellow_sets_building_damage_state
cargo test -q c4_damage_crossing_condition_yellow_sets_building_damage_state
cargo test -q lightning_storm_crossing_condition_yellow_sets_building_damage_state
cargo test -q building_repair_crossing_above_condition_yellow_clears_building_damage_state
```

**Final check:**

```powershell
cargo check -q
```

Expected: tests pass; `cargo check` may still emit existing warnings, but no new errors.

## Non-Goals

- Do not model construction/selling/gate `+0x534` values.
- Do not rewrite the 21-slot building animation lifecycle.
- Do not reintroduce healthy occupied body frame `2`.
- Do not make render derive the damaged-state gate from health.
- Do not claim exact standard player-repair timing parity until the follow-up RE is done.

## Follow-Ups After Implementation

- `/re-investigate building player repair SetDamagedState threshold timing`
- `/re-investigate BuildingClass+0x534 BState table values construction selling gate`
- Revisit whether `building_damage_state_active` should widen into a richer native BState representation only after those reports are complete.
