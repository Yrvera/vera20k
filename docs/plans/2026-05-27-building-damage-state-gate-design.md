# Building Damage-State Gate Design

## Goal

Model the proven native `BuildingClass+0x534` zero/nonzero damaged-state gate as stored simulation state, so occupied building visuals no longer derive that gate from health in render.

## Architecture Context

Native YR stores a per-building gate and later reads it from `BuildingClass::GetCurrentFrame`. If the gate is zero, `GetCurrentFrame` returns the raw body frame and does not inspect occupants. If the gate is nonzero and the type is `CanBeOccupied`, it runs the occupied body-frame formula.

Current Rust has no equivalent stored field on `GameEntity`. `src/app_instances/shp.rs` derives the gate from `health_current / health_max <= ConditionYellow` inside render. That gets the common healthy case right after the previous patch, but it is still the wrong ownership model: render is deciding a native building instance byte.

Relevant current surfaces:

- `src/sim/game_entity.rs`: authoritative entity state; has health, construction state, building anim overlays, and damage fire overlays, but no building damaged-state gate.
- `src/sim/combat/mod.rs`: mutates target health for direct and AoE damage.
- `src/sim/world/world_orders.rs`: mutates building health for C4 damage.
- `src/sim/superweapon/lightning_storm.rs`: mutates entity health for Lightning Storm damage.
- `src/sim/production/production_sell.rs`: mutates structure health during Rust's repair tick.
- `src/sim/world/world_hash.rs`: hashes deterministic entity state; must include the new field because it affects rendered pixels and downstream visual state.
- `src/app_instances/shp.rs`: consumes entity state for body-frame and building-anim emission; should read the stored gate instead of recomputing it.

The design respects the project boundary: `sim/` owns deterministic game state; render reads it. `sim/` does not depend on render.

## Impact Analysis

Primary implementation touchpoints:

- `src/sim/game_entity.rs`: add a stored bool-like field with serde default and constructor default.
- `src/sim/components.rs` or a small sim-local helper module: add the threshold update helper.
- `src/sim/combat/mod.rs`: call the helper after structure health is reduced by direct and AoE damage.
- `src/sim/world/world_orders.rs`: call the helper after C4 reduces building health.
- `src/sim/superweapon/lightning_storm.rs`: call the helper after Lightning Storm reduces structure health.
- `src/sim/production/production_sell.rs`: call the helper after repair heals a structure.
- `src/sim/world/world_hash.rs`: hash the stored gate.
- `src/app_instances/shp.rs`: change body-frame and building-anim gate inputs to the stored field.

Risk areas:

- Field naming must not overclaim full `+0x534` semantics. The broader BState table/construction/selling/gate behavior is still blocked.
- Writer coverage must catch all current Rust structure health mutations in scope. Direct combat, AoE combat, C4, Lightning Storm, and repair are in scope. Unrelated instant-destruction paths that set health to zero may also call the helper when they keep the entity alive long enough for rendering; otherwise removal/death handling owns the result.
- Service-depot healing in `src/sim/docking/building_dock.rs` must be audited during implementation. If it can heal structures, it joins the required writer set; if it only services units/aircraft, document that exclusion in the implementation notes/tests.
- Repair implementation is a Rust parity improvement but not full native lifecycle closure, because exact standard player-repair writer timing is still unverified.
- Render tests must distinguish "formula result when gate is active" from "native gate decides whether the formula is entered."

## Chosen Approach

Use a narrow bool-like stored field on `GameEntity`, for example:

```rust
#[serde(default)]
pub building_damage_state_active: bool
```

This field represents only the proven zero/nonzero damaged-state gate for currently scoped visuals. It is not a full BState enum and must not be used to model construction, selling, gate stages, or the BState table until those cases are separately researched.

One sim-owned helper updates the field from the entity's current health and `rules.general.condition_yellow_x1000`, only for structures. The helper uses deterministic integer math:

```text
active = current_health * 1000 <= max_health * condition_yellow_x1000
```

with `max_health == 0` treated as inactive. Callers run this helper immediately after authoritative structure health mutations.

Render receives the stored field and uses it as the entry gate. The occupied body-frame formula itself remains unchanged: yellow occupied civilian frame `2`; red occupied civilian frame collapses to `1`.

## Tiny-Detail Ledger

- Native `GetCurrentFrame` reads `BuildingClass+0x534` before the `CanBeOccupied` branch. If zero, it returns raw body frame and does not inspect occupant count. Source: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md`.
- Occupied civilian yellow under active gate remains frame `2`. Source: same report.
- Occupied civilian red under active gate computes `3`, then `TechLevel == -1` collapses to frame `1`. Source: same report.
- `ConditionYellow` gates damaged active/idle anim variant selection; `ConditionRed` is not the anim damaged threshold. Source: `docs/research/BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`.
- `ConditionRed` remains part of the occupied body-frame formula once the gate is active. Source: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md`.
- Native damage receive recomputes `GetHealthRatio() <= Rules.ConditionYellow`, writes the damaged-state flag, and re-images existing slots when it changes. Source: live Ghidra `BuildingClass__ReceiveDamage @ 0x00442230`, captured in `docs/contracts/2026-05-27-building-bstate-visual-state-implementation-contract.md`.
- Native engineer full repair restores health and calls `SetDamagedState`, clearing the flag when above yellow. Source: live Ghidra `BuildingClass__EngineerRepair @ 0x00701473`, captured in the same contract.
- Standard player repair writer timing is still `UNKNOWN - needs RE`; Rust repair should still clear the stored field when crossing above yellow, but the design must not claim exact native timing closure.
- The stored field affects pixels and must be serialized and included in deterministic state hashing.
- This design does not model multi-valued `+0x534` stage/table semantics. Source: contract blocker row.

## Design

### Components

Add a field to `GameEntity`:

```rust
#[serde(default)]
pub building_damage_state_active: bool
```

Constructor default is `false`. This matches healthy/newly spawned buildings and native `+0x534 == 0` for the scoped render gate.

Do not reuse the existing `DamageState` enum in `src/sim/components.rs` as the stored field. That enum is a derived green/yellow/red helper, while native `+0x534` is stored instance state. Reusing it would blur the exact state contract.

Add a helper with a name that states the narrow contract, for example:

```rust
pub fn refresh_building_damage_state_gate(
    entity: &mut GameEntity,
    condition_yellow_x1000: i64,
) -> bool
```

Return value is `true` when the field changed. That allows later animation-slot re-image hooks without changing callers again. Initial implementation can ignore the return value if the slot refresh is not implemented in this patch.

Helper behavior:

- if `entity.category != EntityCategory::Structure`, do nothing and return `false`;
- if `entity.health.max == 0`, set/keep false;
- otherwise set active when `current * 1000 <= max * condition_yellow_x1000`;
- use widened integer math, not floats.

### Interfaces / Contracts

Damage writer contract:

- after direct damage mutates a structure's health, call the helper before death handling emits the render-visible state;
- after AoE damage mutates a structure's health, call the same helper;
- after C4 mutates building health, call the same helper before setting final dying/death state;
- after Lightning Storm mutates structure health, call the same helper;
- fully nullified invulnerable hits do not call it because health did not change.

Repair writer contract:

- after repair mutates a structure's health, call the helper;
- if repair crosses above yellow, the stored gate clears;
- exact native player-repair call order remains a follow-up RE item.

Render contract:

- `rendered_garrison_body_frame_index` should take an explicit stored gate bool and a raw/current body frame rather than deriving the gate from health;
- if gate is false, return the raw/current body frame. Current Rust usually passes `0`, but the mechanism must not bake in `0` because native returns `BuildingClass+0xF8`;
- if gate is true, run the existing occupied formula using `ConditionYellow` and `ConditionRed`;
- `emit_building_anims` should receive `entity.building_damage_state_active` instead of calling `building_bstate_damage_active`.

Hash/serde contract:

- serde default preserves compatibility with old saves/tests that do not specify the field;
- `world_hash` includes the bool so two otherwise identical worlds with different stored gates hash differently.

### Data Flow

```text
combat direct/AoE damage
  -> mutate structure health
  -> refresh_building_damage_state_gate(entity, ConditionYellow)
  -> death/removal handling
  -> render reads stored gate

C4 / Lightning Storm damage
  -> mutate structure health
  -> refresh_building_damage_state_gate(entity, ConditionYellow)
  -> death/removal handling where applicable
  -> render reads stored gate if entity remains visible

repair tick
  -> mutate structure health
  -> refresh_building_damage_state_gate(entity, ConditionYellow)
  -> stop repair if max health
  -> render reads stored gate

render body/anims
  -> read entity.building_damage_state_active
  -> if false: do not enter occupied body-frame formula; return raw/current body frame
  -> if true: use existing occupied yellow/red/civilian-collapse formula
```

### Error Handling

No fallible runtime error is needed. Missing rules should use the existing defaults already present in rendering paths, but sim writer calls should have `RuleSet` available and use `condition_yellow_x1000`.

If a health mutation path cannot access rules, it should be listed as a follow-up rather than deriving from float render data.

### Testing Strategy

Focused tests:

- `occupied_healthy_cagas01_bstate_zero_renders_frame_zero`
- `occupied_cagas01_yellow_bstate_false_stays_raw_frame`
- `occupied_cagas01_yellow_bstate_true_uses_frame_two`
- `occupied_cagas01_red_bstate_true_collapses_to_frame_one`
- `combat_damage_crossing_condition_yellow_sets_building_damage_state`
- `aoe_damage_crossing_condition_yellow_sets_building_damage_state`
- `c4_damage_crossing_condition_yellow_sets_building_damage_state`
- `lightning_storm_crossing_condition_yellow_sets_building_damage_state`
- `building_repair_crossing_above_condition_yellow_clears_building_damage_state`
- `damaged_active_anim_variant_follows_stored_gate_not_health`
- `garrisoned_active_anim_variant_follows_stored_gate_not_health`
- `building_damage_state_is_serialized_and_hashed`

Existing tests that assert healthy occupied frame `2` must stay deleted or updated; that expectation is native drift.

## Architectural Decisions

- Use a bool-like field, not an enum, because only zero/nonzero damaged-state gate behavior is proven for this patch.
- Store in `GameEntity` because the state is per-instance and authoritative, not render-derived.
- Put threshold update in `sim`, because native writers live on gameplay health mutation paths.
- Use `condition_yellow_x1000` integer comparisons, matching existing deterministic sim style.
- Keep render formula logic in `shp.rs`, but make its entry gate an input from sim state and preserve raw/current body frame fallback when the gate is false.

Tech debt introduced:

- The field may be widened or renamed when full `BuildingClass+0x534` semantics are proven.
- Standard player repair timing remains not fully proven against native.
- Existing damage-fire overlay lifecycle may still be health-derived and should not be conflated with this stored gate.

## Alternatives Considered

### Rich `BuildingBState` enum now

Rejected for this patch. Multi-valued `+0x534` behavior for construction, selling, gates, and BState table indexing is not implementation-ready. Modeling it now would turn UNKNOWN rows into code.

### Centralized health-derived render helper

Rejected. It would clean up duplication but preserve the known parity drift: render would still derive native stored state at read time.

### Leave repair out

Rejected for the chosen implementation scope. Without repair clearing the stored gate, Rust buildings could enter damaged visual state and fail to leave it during existing repair. The design includes repair with an explicit caveat that exact native standard repair timing still needs RE.

## Follow-Ups

- `/re-investigate building player repair SetDamagedState threshold timing`
- `/re-investigate BuildingClass+0x534 BState table values construction selling gate`
- After both are resolved, revisit whether `building_damage_state_active` should widen into a fuller native BState representation.
