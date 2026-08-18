# Parachute Descent System Design

## Goal

Implement the per-tick parachute descent state machine for paradropped infantry — byte-exact to gamemd.exe's `ObjectClass::AI` integrator, mirroring the existing `DropPodState` pattern.

**Scope:** descent state machine only. Body sequence trigger included (set `SequenceKind::Paradrop` on attach, reset on landing). The visible chute sprite (separate Anim above the body) is OUT — needs an attached-anim infrastructure brainstorm. The paradrop SW launch pipeline (carrier aircraft, V-pattern, missions) is OUT — separate brainstorm.

---

## Architecture Context

The codebase already has a fully-working precedent for "infantry-shaped thing falls from sky and lands": `DropPodState` at [src/sim/movement/droppod_movement.rs](src/sim/movement/droppod_movement.rs).

Relevant infrastructure that exists today:

- **`GameEntity`** at [src/sim/game_entity.rs](src/sim/game_entity.rs) — unified per-entity struct. Holds `droppod_state: Option<DropPodState>` (line ~132) and `animation: Option<Animation>`.
- **`LocomotorState`** at [src/sim/movement/locomotor.rs](src/sim/movement/locomotor.rs) — owns `override_state: Option<OverrideLocomotor>` (line ~178). Methods `begin_override(OverrideKind)` / `end_override()` push/pop the base locomotor during transient phases (Teleport, DropPod). When overridden, layer becomes `Air` so no ground-cell occupancy is marked.
- **Tick stage Phase 2** in [src/sim/world/mod.rs:~1097](src/sim/world/mod.rs#L1097) — sequence: ground movement → air + special movement (teleport, tunnel, rocket, droppod) → vision → power → ... Parachute slots in Phase 2 directly after `tick_droppod_movement`.
- **Snapshot serialization** ([src/sim/snapshot.rs](src/sim/snapshot.rs)) — auto via serde derive on `GameEntity`. New optional fields are picked up for free.
- **Animation sequence dispatch** at [src/sim/animation.rs](src/sim/animation.rs) — `SequenceKind::Paradrop` enum variant exists; renderer reads `entity.animation.sequence` to pick frames.
- **General rules parsing** in [src/rules/general_rules.rs](src/rules/general_rules.rs) — `ParachuteMaxFallRate` and `NoParachuteMaxFallRate` are NOT yet parsed.

What does NOT exist today:
- Any "anim attached to entity" mechanism beyond the body anim. (Why the chute sprite is out of scope.)
- Any paradrop SW dispatch — `ParaDrop`/`AmerParaDrop` are recognized as type names in rules but no launch handler exists. (Why the launch pipeline is out of scope.)

---

## Impact Analysis

**New files (1):**
- `src/sim/movement/parachute_descent.rs` — ~350 lines including tests.

**Modified files (4):**
- `src/sim/game_entity.rs` — add `parachute_state: Option<ParachuteDescentState>` field. Free via serde.
- `src/sim/movement/locomotor.rs` — add `OverrideKind::Parachute` variant + handle layer assignment (`MovementLayer::Air`).
- `src/sim/movement/mod.rs` — `pub mod parachute_descent;`.
- `src/sim/world/mod.rs` (~line 1097) — call `parachute_descent::tick_parachute_descent(entities, sim_tick)` after `tick_droppod_movement`.

**Modified rules (1):**
- `src/rules/general_rules.rs` — add `parachute_max_fall_rate: i32` field, parse `[General] ParachuteMaxFallRate=`, default `-3`. Defer `no_parachute_max_fall_rate` (out of scope).

**Determinism risks:**
- Integer-tick rate accumulation — pure `i32` ops, deterministic by definition.
- Iteration order via `entities.keys_sorted()` — deterministic, matches DropPod.
- Render-side `screen_y` update uses `f32` (`sim_to_f32`, `ALTITUDE_VISUAL_SCALE`) — render-only, doesn't feed back to sim state.

**State hash impact:** `parachute_state` is included automatically. Lockstep replays will diverge if any tick produces different rate/altitude — but that's the intent.

**Snapshot back-compat:** N/A (no shipped saves).

---

## Chosen Approach

**Mirror DropPod's pattern 1:1**, with one intentional deviation: integer-tick rate ramp instead of continuous `SimFixed * dt` integration. DropPod uses continuous time because gamemd's drop pod descent is genuinely continuous physics. Parachute uses discrete-tick because gamemd's parachute is `rate -= 1; rate = max(rate, MaxFall); Z += rate` integer-tick math at `0x005F3FBC`–`0x005F3FFA`.

Two systems, two correct integration choices.

### Why not the alternatives

- **Bake into `LocomotorState` directly** (no separate state struct). Rejected: cuts against the DropPod precedent. Future reader looking for `parachute_movement.rs` next to `droppod_movement.rs` wouldn't find it.
- **Object-level fields on `GameEntity`** (`descent_rate: i32`, `in_air: bool`, etc., flat). Rejected: bloats every entity for fields used by 0.001% of units, fights the project's `Option<State>` convention.

---

## Tiny-Detail Ledger

Parity constraints from `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` Round 4. Each item must have a home in the implementation.

| # | Detail | Source | Implementation home |
|---|---|---|---|
| L1 | Rate is signed int leptons/tick at `Object+0x2C`, accumulated across ticks | GHIDRA `0x005F3F32`, `0x005F3FBC` | `ParachuteDescentState.rate: i32` |
| L2 | Per-tick rate update: `rate -= 1` (integer DEC) | GHIDRA `0x005F3FBF` | `state.rate -= 1` in tick body |
| L3 | Rate clamp: `rate = max(rate, MaxFallRate)` (toward more-negative bound) | GHIDRA `0x005F3FCB`-`0x005F3FF8` | `.max(rules.parachute_max_fall_rate)` after decrement |
| L4 | Z integration: `Z_new = Z_base + rate` per tick (integer add) | GHIDRA `0x005F3F32`-`0x005F3F60` | `state.altitude += SimFixed::from_int(state.rate)` |
| L5 | First integrator tick does NOT move the unit (rate is still 0) | GHIDRA — initial state | Init `rate = 0`; integrate BEFORE rate update each tick |
| L6 | 3-tick rate ramp: `0 → -1 → -2 → -3`, then steady at `-3` | doc §R4.7 timeline | Falls out of L1+L2+L3+L5 — verified by test |
| L7 | Total descent = 6 leptons over first 4 ticks; 3 leptons/tick steady-state | doc §R4.7 | Verified by test |
| L8 | Landing trigger: `altitude ≤ 0` (inclusive bound, NOT strict `<`) | GHIDRA `0x005F3F70` (`JG`) | `if state.altitude <= SIM_ZERO` |
| L9 | On landing: clamp altitude to exactly 0 (no negative residual) | GHIDRA `0x005F3F7A` (`SetHeight(0)`) | `state.altitude = SIM_ZERO` before cleanup |
| L10 | Body sequence trigger fires at attach-time (once), NOT each tick | GHIDRA `0x005217A8` (`DoType(0x21,1,0)` in InfantryClass::Unlimbo) | Set in `begin_parachute_descent`, never in tick |
| L11 | Body sequence reset fires when landed AND was-Paradrop. Reset to Stand. | GHIDRA `0x00520B27`-area (post-switch DoType_Sequencer fallback) | Cleanup gates on `if animation.sequence == Paradrop` before resetting |
| L12 | Rate field is NOT reset to 0 on landing — retained for next descent | doc §R4.7 edge case | State struct lifecycle handles this — fresh state on each `begin` |
| L13 | `Rules.ParachuteMaxFallRate` default = -3 leptons/tick | ini default + doc PARADROP §4 | `general_rules.parachute_max_fall_rate` field, default `-3` |
| L16 | Mark/Unmark cell-occupancy wrap on Z write only when `Object+0x74 != 0`. Paradropped infantry mid-air has `+0x74 == 0`. | GHIDRA `0x005F3F37` + branch | `OverrideKind::Parachute` sets `MovementLayer::Air` → no cell marking happens during descent |

**In scope:** L1–L13, L16.

**Deferred to follow-up brainstorms:**
- L14 (free-fall mode `NoParachuteMaxFallRate`, 1.4 accel)
- L15 (1-tick async chute removal lag — needs attached-anim infra)
- L17 (InfantryClass::Unlimbo always-success quirk — needs launch pipeline)
- L18 (`Math__ftol` rounding mode — needs verify-doc pass)

---

## Design

### Components

`src/sim/movement/parachute_descent.rs`:

```rust
//! Parachute descent — per-tick altitude integrator for paradropped infantry.
//!
//! Mirrors gamemd.exe ObjectClass::AI (0x005F3E70) descent block exactly:
//! - rate accumulates by -1 per tick, clamped to Rules.ParachuteMaxFallRate
//! - Z integrates as `Z += rate` per tick (integer leptons)
//! - first tick has rate=0 → no movement (3-tick ramp: 0,-1,-2,-3,-3,...)
//! - landing on altitude <= 0 (inclusive bound)
//! - body sequence set to Paradrop on attach, reset to Stand on landing
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/game_entity, sim/entity_store, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::animation::SequenceKind;
use crate::sim::debug_event_log::DebugEventKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::movement::locomotor::OverrideKind;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_to_f32};

/// Visual height offset per lepton of altitude (matches DropPod). Render-only f32.
const ALTITUDE_VISUAL_SCALE: f32 = 0.06;

/// Per-entity parachute descent state. Set by `begin_parachute_descent`,
/// cleared on landing. While Some, the entity's locomotor is overridden
/// (OverrideKind::Parachute) so it does not occupy ground cells.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParachuteDescentState {
    /// Descent rate in leptons/tick. Negative = falling. Starts at 0,
    /// decrements by 1 per tick, clamps to Rules.ParachuteMaxFallRate.
    pub rate: i32,
    /// Current altitude in leptons. Decreases by `rate` each tick.
    pub altitude: SimFixed,
}

/// Begin parachute descent for an entity. Returns true on success.
///
/// - Applies OverrideKind::Parachute to suppress the base locomotor
/// - Initializes state with rate=0 (3-tick ramp begins on first tick)
/// - Sets body sequence to Paradrop (held until landing)
pub fn begin_parachute_descent(
    entities: &mut EntityStore,
    entity_id: u64,
    drop_altitude: SimFixed,
) -> bool {
    let Some(entity) = entities.get_mut(entity_id) else {
        return false;
    };

    if let Some(ref mut loco) = entity.locomotor {
        loco.begin_override(OverrideKind::Parachute);
    }

    entity.parachute_state = Some(ParachuteDescentState {
        rate: 0,                 // L1, L5: starts at 0
        altitude: drop_altitude,
    });

    // L10: body sequence set ONCE at attach, not per-tick
    if let Some(ref mut anim) = entity.animation {
        anim.sequence = SequenceKind::Paradrop;
    }

    entity.push_debug_event(
        0,
        DebugEventKind::SpecialMovementStart {
            kind: "Parachute".into(),
        },
    );
    true
}

/// Per-tick advance for all entities with `parachute_state`. Wired into
/// World::advance_tick Phase 2 immediately after tick_droppod_movement.
pub fn tick_parachute_descent(
    entities: &mut EntityStore,
    parachute_max_fall_rate: i32,  // from rules.general
    sim_tick: u64,
) {
    let mut finished: Vec<u64> = Vec::new();

    for &id in &entities.keys_sorted() {
        let Some(entity) = entities.get_mut(id) else { continue; };
        let Some(ref mut state) = entity.parachute_state else { continue; };

        // L4, L5: integrate Z FIRST. On the first tick rate is still 0,
        // so altitude doesn't change yet — that's the 3-tick ramp.
        state.altitude += SimFixed::from_int(state.rate);

        // L8, L9: landing on altitude <= 0 (inclusive)
        if state.altitude <= SIM_ZERO {
            state.altitude = SIM_ZERO;
            finished.push(id);
        } else {
            // L2, L3: rate update — integer DEC, clamp toward more-negative bound
            state.rate = (state.rate - 1).max(parachute_max_fall_rate);
        }

        // Render-side: update screen_y with altitude offset.
        // sim_to_f32 + f32 mul are render-only; do NOT feed back to sim state.
        let (sx, sy) = crate::util::lepton::lepton_to_screen(
            entity.position.rx,
            entity.position.ry,
            entity.position.sub_x,
            entity.position.sub_y,
            entity.position.z,
        );
        entity.position.screen_x = sx;
        entity.position.screen_y = sy - sim_to_f32(state.altitude) * ALTITUDE_VISUAL_SCALE;
    }

    // Cleanup landed entities
    for id in finished {
        if let Some(entity) = entities.get_mut(id) {
            entity.parachute_state = None;
            if let Some(ref mut loco) = entity.locomotor {
                if loco.is_overridden() {
                    loco.end_override();
                }
            }
            // L11: reset body sequence ONLY if it's still Paradrop
            // (don't overwrite if some other system has already changed it)
            if let Some(ref mut anim) = entity.animation {
                if anim.sequence == SequenceKind::Paradrop {
                    anim.sequence = SequenceKind::Stand;
                }
            }
            entity.push_debug_event(sim_tick as u32, DebugEventKind::SpecialMovementEnd);
        }
    }
}
```

(Final stub names — `SequenceKind::Stand` may need adjustment to match the actual idle variant in the enum; verify during implementation.)

### Interfaces / Contracts

**Public API (called from outside the module):**
- `begin_parachute_descent(entities, entity_id, drop_altitude) -> bool` — entry point. Same shape as `begin_droppod_entry`.
- `tick_parachute_descent(entities, parachute_max_fall_rate, sim_tick)` — per-tick driver. Wired into `World::advance_tick`.

**Required upstream:**
- `OverrideKind::Parachute` variant must be added to `locomotor.rs` and must set `MovementLayer::Air` when active (matches `DropPod` behavior).
- `general_rules.parachute_max_fall_rate: i32` must be parsed from `[General] ParachuteMaxFallRate=` in INI.

**Downstream consumers:**
- Renderer reads `entity.position.screen_y` (already updated by tick).
- Renderer reads `entity.animation.sequence` (set to `Paradrop` during descent).
- Snapshot serializer auto-includes `entity.parachute_state`.

### Data Flow

**Per tick:**
```
Phase 2 of advance_tick:
  ... ground movement ...
  ... air movement, teleport, tunnel, rocket ...
  tick_droppod_movement(entities, tick_ms, sim_tick)
  tick_parachute_descent(entities, rules.general.parachute_max_fall_rate, sim_tick)  ← NEW
  ... aircraft mission state machines ...
```

**Single-entity descent timeline** (drop_altitude = D leptons, ParachuteMaxFallRate = -3):

| Tick | rate IN | altitude IN | altitude after Z integrate | landed? | rate OUT | altitude OUT |
|---|---|---|---|---|---|---|
| 1 | 0 | D | D + 0 = D | no | -1 | D |
| 2 | -1 | D | D - 1 | no | -2 | D - 1 |
| 3 | -2 | D - 1 | D - 3 | no | -3 (clamped) | D - 3 |
| 4 | -3 | D - 3 | D - 6 | no | -3 | D - 6 |
| 5 | -3 | D - 6 | D - 9 | no | -3 | D - 9 |
| ... | ... | ... | ... | ... | ... | ... |
| N | -3 | … | D - 3·(N-2) | depends | -3 | D - 3·(N-2) |

For D = 1200 (DropPod's default): landing at tick `N` where `D - 3·(N-2) ≤ 0` → `N ≥ D/3 + 2 = 402`. At 64ms/tick that's ~25.7 seconds.

### Error Handling

- Missing entity: `begin_parachute_descent` returns `false`. No panic.
- Missing locomotor: skip override (state still attached, descent still works). Mirrors DropPod's `test_droppod_without_loco_still_works`.
- Missing animation: sequence trigger silently skipped. Tested.
- Already in parachute state: caller's responsibility not to double-attach. We could `assert!(entity.parachute_state.is_none())` defensively but the project convention (per CLAUDE.md "Don't add error handling for scenarios that can't happen") says trust the caller.

### Testing Strategy

All tests are direct unit tests in `parachute_descent.rs`. No engine spinup required (DropPod's pattern).

| Test | Verifies | Ledger items |
|---|---|---|
| `test_3tick_rate_ramp` | rate sequence is exactly `[0, -1, -2, -3, -3, -3]` over 6 ticks | L1, L2, L3, L5, L6 |
| `test_descent_distance_first_4_ticks` | altitude after 4 ticks = drop_altitude − 6 leptons | L4, L7 |
| `test_steady_state_rate` | after tick 4+, descent rate is exactly 3 leptons/tick | L3, L7 |
| `test_landing_inclusive_zero` | altitude exactly 0 triggers landing, altitude > 0 does not | L8 |
| `test_landing_clamps_to_zero` | post-landing altitude is exactly `SIM_ZERO` (no negative residual) | L9 |
| `test_clamp_at_max_fall_rate` | rate never exceeds (more-negative than) `ParachuteMaxFallRate` | L3 |
| `test_clamp_with_custom_max_fall_rate` | passing `-1` makes rate cap at `-1` (2-tick ramp) | L3, L13 |
| `test_body_sequence_set_on_begin` | `begin_parachute_descent` sets `animation.sequence = Paradrop` | L10 |
| `test_body_sequence_reset_on_landing` | landing clears animation back to `Stand` | L11 |
| `test_body_sequence_preserved_if_externally_changed` | if sequence is no longer Paradrop at landing, don't overwrite | L11 (gating clause) |
| `test_locomotor_override_restored_on_landing` | mirrors `test_droppod_full_sequence` | architectural |
| `test_works_without_locomotor` | mirrors `test_droppod_without_loco_still_works` | architectural |
| `test_works_without_animation` | begin doesn't panic when `entity.animation` is None | error handling |

The first three tests are the parity tests. They must pass byte-exact against the gamemd timeline in §R4.7 of the JUMPJET report.

---

## Architectural Decisions

**Followed:**
- DropPod's `Option<State>` per-entity pattern.
- DropPod's `OverrideKind` mechanism for layer/locomotor swap.
- DropPod's render-side `screen_y` altitude offset (same `ALTITUDE_VISUAL_SCALE`).
- Project's "no floats in sim state" rule.
- Module size ~600 lines (this will be ~350 including tests).

**Deviated (with reason):**
- DropPod uses `dt_from_tick_ms` + continuous-time `altitude -= speed * dt`. Parachute uses discrete-tick `altitude += rate; rate -= 1`. **Reason:** gamemd's parachute is integer-tick math (`DEC EDI` at `0x005F3FBF`), and continuous integration would silently lose the 3-tick ramp.

**Tech debt introduced:** None. All ledger items have a home; no shortcuts taken in scope.

**Tech debt acknowledged (out of scope):** L14, L15, L17, L18 are documented as deferred. Plus the visible chute sprite needs an attached-anim brainstorm before it can be implemented.

---

## Alternatives Considered

1. **Bake parachute into `LocomotorState` (no separate state struct).** Rejected — cuts against the DropPod precedent, harder to test in isolation.

2. **Object-level flat fields on `GameEntity` (`descent_rate: i32`, `in_air: bool`, etc.).** Rejected — bloats every entity for fields used by ~0.001% of units, fights the project's `Option<State>` convention. The "matches gamemd field offsets" benefit is theoretical; we cross-reference via the GHIDRA report, not at runtime.

3. **Continuous-time integration (DropPod-style).** Rejected for parachute specifically — see "Deviated" above. Integer-tick is what the binary does and what the parity ledger requires.

---

## Status (2026-05-05)

Implemented as planned in [docs/plans/2026-05-05-parachute-descent-plan.md](2026-05-05-parachute-descent-plan.md). Commits `86fbd1a..` on `dev`. **All 16 tests pass; 1518 total tests pass with zero regressions.**

Live in `World::advance_tick` Phase 2 immediately after `tick_droppod_movement`. No external callers yet — the paradrop SW launch pipeline is the next brainstorm.

**Deferred items** (per Tiny-Detail Ledger):
- L14: free-fall mode (`NoParachuteMaxFallRate`, 1.4 accel)
- L15: 1-tick async chute removal lag (needs attached-anim infra)
- L17: `InfantryClass::Unlimbo` always-success quirk (needs launch pipeline)
- L18: `Math__ftol` rounding mode (low priority — only matters for L14)
- Visible chute sprite (separate brainstorm on attached-anim infra)
- Paradrop SW launch pipeline (carrier aircraft + Drop_Payload + missions)
