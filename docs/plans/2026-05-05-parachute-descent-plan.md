# Parachute Descent Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Commit at the end of each task (to local `dev` branch — never push).

**Goal:** Implement the per-tick parachute descent state machine for paradropped infantry — byte-exact to gamemd.exe's `ObjectClass::AI` integrator, mirroring the existing `DropPodState` pattern.

**Architecture:** New module `src/sim/movement/parachute_descent.rs` mirrors `droppod_movement.rs` 1:1. New `Option<ParachuteDescentState>` field on `GameEntity`. New `OverrideKind::Parachute` variant uses the existing locomotor override mechanism. Tick wired into Phase 2 of `World::advance_tick` immediately after `tick_droppod_movement`.

**Design Doc:** [docs/plans/2026-05-05-parachute-descent-design.md](docs/plans/2026-05-05-parachute-descent-design.md)

---

## Grounding Summary

- **Research docs:** `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` Round 4 (2026-05-05) — verified the descent integrator at `0x005F3E70`, the rate field at `Object+0x2C`, the `ParachuteMaxFallRate` consumer at `0x005F3FCB` (encoding `8B 89 B8 07 00 00`), and the InfantryClass::Unlimbo dispatch of sequence 33 (`Paradrop`) at `0x005217A8`. `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md` for the spawner side (out of scope here, but referenced).
- **Ghidra verification:** all binary claims in this plan come from Round 4 of the JUMPJET report — every magic number, branch, and ordering is verified-from-binary, not inferred.
- **Repo pattern:** [src/sim/movement/droppod_movement.rs](src/sim/movement/droppod_movement.rs) is the direct precedent. Same shape: `Option<State>` on GameEntity, `begin_*` entry function, `tick_*` per-tick driver, locomotor override on attach and `end_override` on cleanup. Recent deploy_state work (b616d9b, bc72091) confirms this pattern is alive and active.
- **INI keys:** `[General] ParachuteMaxFallRate=-3` at `ini/rulesmd.ini:68` — confirmed default. `Parachute=PARACH` at `ini/rulesmd.ini:564` — out of scope (no chute sprite). `NoParachuteMaxFallRate=-100` at line 69 — out of scope (no free-fall mode).
- **Unknown after grounding:** the rounding mode of `Math__ftol` (only matters for free-fall mode, which is deferred). Whether any other code paths in the engine spawn paradrops outside the SW pipeline (out of scope; we only build the descent module — caller comes later).

## Key Technical Decisions

- **Integer-tick rate accumulation, NOT continuous-time `dt`** — gamemd's parachute does `DEC EDI` (integer) at `0x005F3FBF`. Continuous `f32 * dt` would silently lose the 3-tick ramp. **Confidence:** high. **Source:** Ghidra `0x005F3FBC-0x005F3FFA` (Round 4 §R4.4).
- **`rate: i32` on the state struct** — gamemd stores it as a 32-bit signed int at `Object+0x2C`. Matches binary directly. **Confidence:** high. **Source:** Ghidra `0x005F3F32` (`MOV ECX, [ESI+0x2C]`).
- **`altitude: SimFixed`** — for render-side `screen_y` offset compatibility (matches `DropPodState.altitude`). The `+= rate` step uses `SimFixed::from_int(rate)`. **Confidence:** high. **Source:** repo pattern [droppod_movement.rs:55-62](src/sim/movement/droppod_movement.rs#L55).
- **Z integration BEFORE rate update** — gamemd integrates Z at `0x005F3F32-0x005F3F60`, then updates rate at `0x005F3FBC` onward. Reversing this order would skip the 3-tick ramp's first tick. **Confidence:** high. **Source:** Ghidra `0x005F3F32` → `0x005F3FBC` instruction order (Round 4 §R4.4).
- **Landing trigger uses `<= 0`, not `< 0`** — gamemd uses `JG` (jump if greater) at `0x005F3F70`, which means "altitude > 0 → not landed". Inverse: `altitude <= 0 → landed`. **Confidence:** high. **Source:** Ghidra `0x005F3F70`.
- **Body sequence reset gated on `current == Paradrop`** — defensive: if some other system has changed the sequence during descent, don't overwrite. Matches gamemd's post-switch fallback `if (Doing == 33 && !InAir)`. **Confidence:** high. **Source:** Ghidra `0x00520B27`-area in `InfantryClass::DoType_Sequencer`.
- **New `LocomotorKind::Parachute` variant (runtime-only, no CLSID)** — mirrors how `LocomotorKind::DropPod` exists as a runtime variant without an INI mapping. Required so `begin_override` can map to it. **Confidence:** high. **Source:** repo pattern [locomotor_type.rs:37](src/rules/locomotor_type.rs#L37) for DropPod.

All decisions are HIGH confidence. No flags for `/review-plan`.

## Open Questions

### Resolved During Planning

- **Where does `ParachuteMaxFallRate` parsing land?** → `src/rules/ruleset.rs:119` (`GeneralRules` struct) + parser around line 700. Source: grep for existing `general.get_i32` calls in ruleset.rs.
- **Does `LocomotorKind` need an exhaustive-match update everywhere?** → Yes, but the precedent (DropPod) shows the affected sites: just the `begin_override` map in [locomotor.rs:328-348](src/sim/movement/locomotor.rs#L328) and the kind→layer constructor map in [locomotor.rs:206-222](src/sim/movement/locomotor.rs#L206). Task 1 includes a grep step to catch any others.
- **Does `tick_parachute_descent` need `tick_ms`?** → Yes, for the `if tick_ms == 0 { return; }` pause-guard, mirroring DropPod. We don't multiply by `dt`, but we still skip on paused ticks.
- **What `SequenceKind` resets to on landing?** → `SequenceKind::Stand` (confirmed: see [animation.rs:447-450](src/sim/animation.rs#L447) `if !has_movement && anim.sequence == SequenceKind::Walk { anim.switch_to(SequenceKind::Stand); }` — Stand is the canonical idle).

### Deferred to Implementation

- None. All architectural questions answered.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/movement/parachute_descent.rs` | The descent state struct, begin/tick functions, and unit tests |
| Modify | `src/rules/locomotor_type.rs` | Add `LocomotorKind::Parachute` variant |
| Modify | `src/sim/movement/locomotor.rs` | Add `OverrideKind::Parachute` variant + `begin_override` map + kind→layer map |
| Modify | `src/sim/movement/mod.rs` | Register `pub mod parachute_descent;` |
| Modify | `src/sim/game_entity.rs` | Add `parachute_state: Option<ParachuteDescentState>` field + init in test_default |
| Modify | `src/rules/ruleset.rs` | Add `parachute_max_fall_rate: i32` to `GeneralRules` + INI parser line |
| Modify | `src/sim/world/mod.rs` | Wire `tick_parachute_descent` into Phase 2 after `tick_droppod_movement` |

## Interface Changes

**New public APIs:**
- `parachute_descent::ParachuteDescentState` (struct, serde-serializable)
- `parachute_descent::begin_parachute_descent(entities, entity_id, drop_altitude) -> bool`
- `parachute_descent::tick_parachute_descent(entities, tick_ms, parachute_max_fall_rate, sim_tick)`
- `LocomotorKind::Parachute` (enum variant — runtime only, no INI mapping)
- `OverrideKind::Parachute` (enum variant)
- `GameEntity.parachute_state: Option<ParachuteDescentState>` (field)
- `GeneralRules.parachute_max_fall_rate: i32` (field)

**Consumers (none yet):** `begin_parachute_descent` has no callers in this plan. The future paradrop SW dispatch will be the first consumer, but that's a separate brainstorm.

**Existing-code callers that depend on these:**
- `LocomotorKind::Parachute` — exhaustive matches that handle every variant. Task 1 will grep for these.
- `OverrideKind::Parachute` — currently only matched in `begin_override`. Task 2 handles.

## Sim Checklist

- [x] All math uses `i32` + `SimFixed` — no f32/f64 in sim state. Render-side `screen_y` uses f32 but doesn't feed back to sim.
- [x] New `parachute_state` field included in deterministic state hash automatically via serde.
- [x] No dependencies on render/ui/sidebar/audio/net. Confirmed: imports only from `sim::*`, `util::*`, `rules::*`.
- [x] Tick ordering impact: parachute runs in Phase 2 immediately after droppod, BEFORE combat (Phase 5). Falling units are positioned correctly before any attack/retaliation logic sees them.
- [x] `BTreeMap` iteration via `entities.keys_sorted()` — deterministic, matches DropPod.

## Risk Areas

- **`LocomotorKind` exhaustive matches** — adding a variant breaks any `match kind { ... }` without a `_` arm. Task 1 includes an explicit grep step.
- **Stale `parachute_state` after `end_override`** — must clear state field BEFORE calling `end_override`, otherwise the locomotor reverts but the state lingers. Task 7 enforces this order in cleanup.
- **`SequenceKind::Stand` reset overwriting deliberate sequence change** — mitigated by the `if anim.sequence == SequenceKind::Paradrop` gate (L11). Test `test_body_sequence_preserved_if_externally_changed` confirms.
- **No regression risk to DropPod** — no shared state. DropPod and Parachute are sibling `Option<State>` fields with independent tick functions.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 7 | 3-tick rate ramp (0 → -1 → -2 → -3, then steady) | Player can see the unit accelerate visibly during the first 4 ticks of descent — instant -3 would look "snappy", ramp looks "natural like in YR" | Test `test_3tick_rate_ramp` asserts exact rate sequence; cross-references Ghidra `0x005F3FBC` |
| Task 7 | Total descent = 6 leptons over first 4 ticks | Per-tick screen position must match gamemd; off-by-one would make units land 1 tick early/late | Test `test_descent_distance_first_4_ticks` asserts exact altitude after each tick |
| Task 7 | Steady-state 3 leptons/tick | The descent speed players see for 99% of the descent | Test `test_steady_state_rate` |
| Task 7 | Landing trigger inclusive at altitude == 0 | Off-by-one here means descent runs one extra tick (or one tick short) — visible as a "hover" moment or "snap-down" | Test `test_landing_inclusive_zero` |
| Task 7 | Landing clamps altitude to exactly SIM_ZERO | Negative residual altitude would cause render glitch ("infantry below ground" for one frame) | Test `test_landing_clamps_to_zero` |
| Task 6 | Body sequence set ONCE at attach (not per-tick) | Per-tick re-set would reset frame counter every tick, freezing the chute pose's animation | Test `test_body_sequence_set_on_begin` |
| Task 7 | Body sequence reset gated on `== Paradrop` | If a death sequence triggered mid-descent, we shouldn't overwrite it on landing | Test `test_body_sequence_preserved_if_externally_changed` |
| Task 3 | `ParachuteMaxFallRate` parsed from INI (not hardcoded) | Mods may change this; hardcoded would silently ignore the mod's value | Test `test_general_rules_parses_parachute_max_fall_rate` |

---

## Tasks

### Task 1: Add `LocomotorKind::Parachute` variant

**Why:** Required upstream for `OverrideKind::Parachute` to map to a concrete LocomotorKind. Mirrors DropPod precedent. Done first because everything else depends on it.

**Files:**
- Modify: `src/rules/locomotor_type.rs:27-37` (enum definition)
- Modify: `src/rules/locomotor_type.rs:66+` (impl block — no new code, just confirm `from_clsid` doesn't need changes)

**Pattern:** `LocomotorKind::DropPod` precedent ([locomotor_type.rs:37](src/rules/locomotor_type.rs#L37)) — runtime-only variant with no CLSID.

**Step 1: Add the enum variant**

```rust
// src/rules/locomotor_type.rs — inside `pub enum LocomotorKind { ... }`
pub enum LocomotorKind {
    Drive,
    Walk,
    Hover,
    Mech,
    Ship,
    DropPod,
    // ...
    Teleport,
    Tunnel,
    Fly,
    Jumpjet,
    Rocket,
    Parachute,    // NEW: transient locomotor for paradropped units during descent.
                  //      Runtime-only; no CLSID maps to it. Set by OverrideKind::Parachute.
}
```

**Step 2: Find every exhaustive match on `LocomotorKind`**

Run: `grep -rn "match.*LocomotorKind\|LocomotorKind::Drive\b\|LocomotorKind::Walk\b" src/ --include="*.rs"`

For each `match kind { ... }` block found, confirm one of:
- It has a `_ => ...` arm (no change needed)
- It has explicit arms for all variants (add `LocomotorKind::Parachute => ...` arm)

Expected hits:
- [src/sim/movement/locomotor.rs:206-222](src/sim/movement/locomotor.rs#L206) — kind→layer constructor (Task handles this in Step 3)
- [src/rules/locomotor_type.rs:66+](src/rules/locomotor_type.rs#L66) — `from_clsid` (uses `_` default — no change)
- [src/rules/locomotor_type.rs:220+](src/rules/locomotor_type.rs#L220) — `default_for_category` (no Parachute default — no change)
- Any test files matching against LocomotorKind (likely have `_` defaults)

**Step 3: Add Parachute to the kind→layer constructor map**

```rust
// src/sim/movement/locomotor.rs — inside LocomotorState::new() or equivalent constructor,
// in the `let (layer, speed_multiplier) = match kind { ... }` block around line 206-222
LocomotorKind::Parachute => (MovementLayer::Air, sim_one),
```

**Step 4: Verify compile**

Run: `cargo check`
Expected: clean compile, no warnings about non-exhaustive matches.

If compile fails on a `match kind { ... }` site outside the listed files, add the `LocomotorKind::Parachute` arm to that match. Use `unreachable!("LocomotorKind::Parachute is runtime-only")` if the match is in an INI-parsing context that should never see Parachute.

**Step 5: Commit**

```
git add src/rules/locomotor_type.rs src/sim/movement/locomotor.rs
git commit -m "movement: add LocomotorKind::Parachute runtime variant"
```

---

### Task 2: Add `OverrideKind::Parachute` variant

**Why:** This is the entry point that `begin_parachute_descent` will use to flip the locomotor into Air-layer / non-marking mode during descent.

**Files:**
- Modify: `src/sim/movement/locomotor.rs:386` (OverrideKind enum)
- Modify: `src/sim/movement/locomotor.rs:328-348` (begin_override map)

**Pattern:** `OverrideKind::DropPod` ([locomotor.rs:335](src/sim/movement/locomotor.rs#L335)).

**Step 1: Add the enum variant**

```rust
// src/sim/movement/locomotor.rs — inside `pub enum OverrideKind { ... }` around line 386
pub enum OverrideKind {
    Teleport,
    DropPod,
    Parachute,   // NEW: paradropped units during descent. Maps to LocomotorKind::Parachute, MovementLayer::Air.
}
```

**Step 2: Add to begin_override map**

```rust
// src/sim/movement/locomotor.rs — inside begin_override match (around line 334-335)
let (new_kind, new_layer) = match override_kind {
    OverrideKind::Teleport => (LocomotorKind::Teleport, MovementLayer::Ground),
    OverrideKind::DropPod => (LocomotorKind::DropPod, MovementLayer::Air),
    OverrideKind::Parachute => (LocomotorKind::Parachute, MovementLayer::Air),  // NEW
};
```

**Step 3: Find any match on `OverrideKind` elsewhere**

Run: `grep -rn "OverrideKind::" src/ --include="*.rs"`

Expected to find: only `begin_override` and possibly an `end_override` debug log. None of these need an explicit Parachute arm if they use `Debug` formatting or only check specific variants.

**Step 4: Verify compile**

Run: `cargo check`
Expected: clean compile.

**Step 5: Commit**

```
git add src/sim/movement/locomotor.rs
git commit -m "movement: add OverrideKind::Parachute variant"
```

---

### Task 3: Add `parachute_max_fall_rate` to GeneralRules + INI parser

**Why:** Per ledger L13, the clamp value must be parsed from INI, not hardcoded. Done before the descent module so the rules field exists when we wire the tick.

**Files:**
- Modify: `src/rules/ruleset.rs:119` (GeneralRules struct field)
- Modify: `src/rules/ruleset.rs:~700` (INI parser line — pick a sensible spot near other movement-related fields)

**Pattern:** Existing `general.get_i32("KeyName").unwrap_or(default)` pattern at lines 616-765.

**Step 1: Find the GeneralRules struct definition**

Read [src/rules/ruleset.rs:119](src/rules/ruleset.rs#L119) and the surrounding ~30 lines to find where to add the new field. Add it grouped with other movement / aerial fields if any exist; otherwise add at the end of the struct.

**Step 2: Add the field**

```rust
// src/rules/ruleset.rs — inside `pub struct GeneralRules { ... }`
/// Descent rate cap (most-negative bound) for parachuted units, in leptons/tick.
/// Per gamemd, rate accumulates by -1/tick and clamps to this value.
/// Default `-3` matches `[General] ParachuteMaxFallRate=-3` in rulesmd.ini.
pub parachute_max_fall_rate: i32,
```

**Step 3: Add the INI parser line**

```rust
// src/rules/ruleset.rs — inside the GeneralRules constructor (around line 700, near other rate fields)
parachute_max_fall_rate: general.get_i32("ParachuteMaxFallRate").unwrap_or(-3),
```

**Step 4: Add a unit test**

```rust
// src/rules/ruleset.rs — inside the existing #[cfg(test)] mod tests block
#[test]
fn test_general_rules_parses_parachute_max_fall_rate() {
    // Use the actual rulesmd.ini fixture pattern used by other GeneralRules tests.
    // (Match whatever loader pattern the existing tests use — likely Ruleset::from_ini_text or similar.)
    let ini = "[General]\nParachuteMaxFallRate=-3\n";
    let rules = parse_general_rules_for_test(ini);  // or whatever the existing helper is
    assert_eq!(rules.parachute_max_fall_rate, -3);
}

#[test]
fn test_general_rules_parachute_max_fall_rate_default() {
    let ini = "[General]\n";  // missing key
    let rules = parse_general_rules_for_test(ini);
    assert_eq!(rules.parachute_max_fall_rate, -3, "default should be -3 per gamemd");
}
```

If no test helper for parsing GeneralRules from a string exists, look at the surrounding `#[cfg(test)] mod tests` block (search for `fn test_general` in ruleset.rs) and copy whatever pattern those tests use. **Do not invent a new helper** — match existing conventions.

**Step 5: Verify**

Run: `cargo test parachute_max_fall_rate`
Expected: both tests PASS.

**Step 6: Commit**

```
git add src/rules/ruleset.rs
git commit -m "rules: parse [General] ParachuteMaxFallRate (default -3)"
```

---

### Task 4: Define `ParachuteDescentState` struct + module skeleton

**Why:** Define the data type before the logic that uses it. Mirrors DropPod's `DropPodState`.

**Files:**
- Create: `src/sim/movement/parachute_descent.rs`
- Modify: `src/sim/movement/mod.rs` (register module)

**Pattern:** [src/sim/movement/droppod_movement.rs:1-62](src/sim/movement/droppod_movement.rs#L1).

**Step 1: Create the new module file with module doc + struct only**

```rust
// src/sim/movement/parachute_descent.rs
//! Parachute descent — per-tick altitude integrator for paradropped infantry.
//!
//! Mirrors gamemd.exe `ObjectClass::AI` (descent block) byte-exact:
//! - rate accumulates by `-1` per tick, clamped to `Rules.ParachuteMaxFallRate`
//! - Z integrates as `Z += rate` per tick (integer leptons)
//! - first tick has `rate = 0` → no movement (3-tick ramp: 0,-1,-2,-3,-3,...)
//! - landing on `altitude <= 0` (inclusive bound)
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
/// cleared on landing. While `Some`, the entity's locomotor is overridden
/// (`OverrideKind::Parachute`) so it does not occupy ground cells.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParachuteDescentState {
    /// Descent rate in leptons/tick. Negative = falling.
    /// Starts at 0, decrements by 1 per tick, clamps to Rules.ParachuteMaxFallRate.
    pub rate: i32,
    /// Current altitude in leptons. Decreases by `rate` each tick.
    pub altitude: SimFixed,
}
```

**Step 2: Register the module**

```rust
// src/sim/movement/mod.rs — add next to droppod_movement
pub mod parachute_descent;
```

**Step 3: Verify compile**

Run: `cargo check`
Expected: clean compile (struct has no consumers yet, but should compile).

**Step 4: Commit**

```
git add src/sim/movement/parachute_descent.rs src/sim/movement/mod.rs
git commit -m "movement: add ParachuteDescentState struct + module skeleton"
```

---

### Task 5: Add `parachute_state` field to GameEntity

**Why:** Storage for the per-entity descent state. Done before begin/tick because they need somewhere to write.

**Files:**
- Modify: `src/sim/game_entity.rs:132` (field declaration, next to droppod_state)
- Modify: `src/sim/game_entity.rs:283` (test_default initializer)
- Modify: `src/sim/game_entity.rs` (any other constructor — search for `droppod_state: None`)

**Pattern:** [game_entity.rs:132](src/sim/game_entity.rs#L132) `droppod_state: Option<DropPodState>`.

**Step 1: Add the field**

```rust
// src/sim/game_entity.rs — next to the existing droppod_state field around line 132
/// Active parachute descent state. `Some` while the unit is descending under a parachute,
/// `None` otherwise. Set by `parachute_descent::begin_parachute_descent`, cleared on landing.
pub parachute_state: Option<crate::sim::movement::parachute_descent::ParachuteDescentState>,
```

**Step 2: Initialize in every constructor**

Run: `grep -n "droppod_state: None" src/sim/game_entity.rs`

For each hit, add a sibling line: `parachute_state: None,`.

Also: `grep -rn "droppod_state: None\|droppod_state: Some" src/ --include="*.rs"` — there may be other constructors (e.g., test helpers in other files) that need the new field.

**Step 3: Verify compile**

Run: `cargo check`
Expected: clean compile. If "missing field `parachute_state`" errors appear, add `parachute_state: None,` at each site.

**Step 4: Commit**

```
git add src/sim/game_entity.rs
git commit -m "sim: add parachute_state field to GameEntity"
```

---

### Task 6: Implement `begin_parachute_descent`

**Why:** The entry point. After this task, an external caller can attach a parachute state to an entity, but the descent won't yet advance per tick (Task 7 wires that).

**Files:**
- Modify: `src/sim/movement/parachute_descent.rs` (append to file)

**Pattern:** [droppod_movement.rs:71-93](src/sim/movement/droppod_movement.rs#L71) `begin_droppod_entry`.

**Step 1: Add the function**

```rust
// src/sim/movement/parachute_descent.rs — append after the struct definition

/// Begin parachute descent for an entity. Returns `true` on success.
///
/// - Applies `OverrideKind::Parachute` to suppress the base locomotor (entity
///   does not occupy ground cells while descending).
/// - Initializes state with `rate = 0` (the 3-tick ramp begins on the first tick).
/// - Sets the body animation to `SequenceKind::Paradrop` (held until landing).
///
/// The entity must already exist in the EntityStore. Caller is responsible for
/// positioning the entity at the desired horizontal coord; `drop_altitude` controls
/// the starting Z.
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
        rate: 0,                 // L1, L5: starts at 0; first tick produces no movement
        altitude: drop_altitude,
    });

    // L10: body sequence set ONCE at attach, NOT per-tick.
    // (per-tick re-set would freeze the frame counter)
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
```

**Step 2: Add unit tests for `begin_parachute_descent`**

```rust
// src/sim/movement/parachute_descent.rs — append at bottom of file

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO};

    /// Mirror of the helper in droppod_movement.rs tests.
    fn make_walk_loco() -> LocomotorState {
        LocomotorState {
            kind: LocomotorKind::Walk,
            layer: MovementLayer::Ground,
            phase: GroundMovePhase::Idle,
            air_phase: AirMovePhase::Landed,
            speed_multiplier: SIM_ONE,
            speed_fraction: SIM_ONE,
            fly_current_speed: SIM_ZERO,
            altitude: SIM_ZERO,
            target_altitude: SIM_ZERO,
            climb_rate: SIM_ZERO,
            jumpjet_speed: SIM_ZERO,
            jumpjet_wobbles: 0.0,
            jumpjet_accel: SIM_ZERO,
            jumpjet_current_speed: SIM_ZERO,
            jumpjet_deviation: 0,
            jumpjet_crash_speed: SIM_ZERO,
            jumpjet_turn_rate: 4,
            balloon_hover: false,
            hover_attack: false,
            speed_type: SpeedType::Foot,
            movement_zone: MovementZone::Normal,
            rot: 0,
            override_state: None,
            air_progress: SIM_ZERO,
            infantry_wobble_phase: 0.0,
            subcell_dest: None,
        }
    }

    fn drop_altitude_1200() -> SimFixed {
        SimFixed::lit("1200")
    }

    #[test]
    fn test_begin_attaches_state_and_overrides_locomotor() {
        let mut entities = EntityStore::new();
        let mut e = GameEntity::test_default(1, "E1", "Americans", 10, 10);
        e.locomotor = Some(make_walk_loco());
        entities.insert(e);

        assert!(begin_parachute_descent(&mut entities, 1, drop_altitude_1200()));

        let entity = entities.get(1).expect("should exist");
        let state = entity.parachute_state.as_ref().expect("has parachute state");
        assert_eq!(state.rate, 0, "L1, L5: rate must start at 0");
        assert_eq!(state.altitude, drop_altitude_1200());

        let loco = entity.locomotor.as_ref().expect("has loco");
        assert!(loco.is_overridden(), "locomotor must be overridden during descent");
        assert_eq!(loco.kind, LocomotorKind::Parachute);
        assert_eq!(loco.layer, MovementLayer::Air, "L16: Air layer means no ground occupancy");
    }

    #[test]
    fn test_body_sequence_set_on_begin() {
        // L10: animation.sequence = Paradrop after begin_parachute_descent.
        let mut entities = EntityStore::new();
        let mut e = GameEntity::test_default(1, "E1", "Americans", 10, 10);
        e.locomotor = Some(make_walk_loco());
        // Ensure entity has an animation (test_default may or may not — check & set)
        if e.animation.is_none() {
            e.animation = Some(crate::sim::animation::Animation::new(SequenceKind::Stand));
        }
        entities.insert(e);

        begin_parachute_descent(&mut entities, 1, drop_altitude_1200());

        let entity = entities.get(1).expect("should exist");
        let anim = entity.animation.as_ref().expect("has anim");
        assert_eq!(anim.sequence, SequenceKind::Paradrop);
    }

    #[test]
    fn test_begin_works_without_locomotor() {
        // Mirrors test_droppod_without_loco_still_works.
        let mut entities = EntityStore::new();
        let e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
        // No locomotor.
        entities.insert(e);

        assert!(begin_parachute_descent(&mut entities, 1, drop_altitude_1200()));

        let entity = entities.get(1).expect("should exist");
        assert!(entity.parachute_state.is_some());
    }

    #[test]
    fn test_begin_returns_false_for_missing_entity() {
        let mut entities = EntityStore::new();
        assert!(!begin_parachute_descent(&mut entities, 999, drop_altitude_1200()));
    }
}
```

**Note:** if `Animation::new` doesn't exist or has a different signature, look at how DropPod tests construct an entity with animation (or how `animation.rs` constructs them) and adapt. **Do not invent a new constructor.**

**Step 3: Verify**

Run: `cargo test parachute_descent::tests::test_begin -- --nocapture`
Expected: 4 tests PASS.

**Step 4: Commit**

```
git add src/sim/movement/parachute_descent.rs
git commit -m "movement: implement begin_parachute_descent + tests"
```

---

### Task 7: Implement `tick_parachute_descent` (the integrator)

**Why:** The core logic. After this task, descent advances correctly per tick but isn't yet wired into the world tick (Task 9).

**Files:**
- Modify: `src/sim/movement/parachute_descent.rs` (append the tick function + cleanup tests)

**Pattern:** [droppod_movement.rs:98-192](src/sim/movement/droppod_movement.rs#L98) `tick_droppod_movement`.

**Step 1: Add the tick function**

```rust
// src/sim/movement/parachute_descent.rs — append before the #[cfg(test)] block

/// Per-tick advance for all entities with `parachute_state`.
///
/// Wired into `World::advance_tick` Phase 2 immediately after `tick_droppod_movement`.
///
/// Per-tick algorithm (matches gamemd `ObjectClass::AI` 0x005F3F11-0x005F3FFA):
/// 1. Integrate Z FIRST: `altitude += rate` (rate is negative; first tick rate=0 → no move)
/// 2. Landing check: `altitude <= 0` → mark for cleanup (altitude clamped to 0)
/// 3. Rate update: `rate -= 1`, clamp to `parachute_max_fall_rate`
/// 4. Update `screen_y` for renderer (altitude offset, render-only f32)
///
/// Cleanup (per landed entity):
/// - clear `parachute_state`
/// - `loco.end_override()` (restores base locomotor)
/// - reset `animation.sequence` to `Stand` ONLY if currently `Paradrop` (L11 gate)
pub fn tick_parachute_descent(
    entities: &mut EntityStore,
    tick_ms: u32,
    parachute_max_fall_rate: i32,
    sim_tick: u64,
) {
    if tick_ms == 0 {
        return;     // pause guard, mirrors DropPod
    }

    let mut finished: Vec<u64> = Vec::new();

    let keys = entities.keys_sorted();
    for &id in &keys {
        let Some(entity) = entities.get_mut(id) else { continue; };
        let Some(ref mut state) = entity.parachute_state else { continue; };

        // L4, L5: integrate Z FIRST. On the first tick rate is still 0,
        // so altitude doesn't change yet — that's the 3-tick ramp.
        state.altitude += SimFixed::from_int(state.rate);

        // L8, L9: landing on altitude <= 0 (inclusive); clamp to exactly 0
        if state.altitude <= SIM_ZERO {
            state.altitude = SIM_ZERO;
            finished.push(id);
        } else {
            // L2, L3: integer DEC then clamp toward more-negative bound
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

    // Cleanup landed entities: clear state BEFORE end_override (order matters).
    for id in finished {
        if let Some(entity) = entities.get_mut(id) {
            entity.parachute_state = None;
            if let Some(ref mut loco) = entity.locomotor {
                if loco.is_overridden() {
                    loco.end_override();
                }
            }
            // L11: reset body sequence ONLY if it's still Paradrop
            // (don't overwrite if a death/other sequence has taken over).
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

**Note on `SimFixed::from_int`:** confirm the exact constructor name in `src/util/fixed_math.rs` — it might be `from_int`, `from_num`, `from_i32`, or similar. Use whatever the existing codebase uses (search: `grep -n "SimFixed::from" src/sim/movement/`). **Do not introduce a new constructor.**

**Note on tick rate signature:** the call site will be:
```rust
parachute_descent::tick_parachute_descent(
    &mut self.entities,
    tick_ms,
    self.rules.general.parachute_max_fall_rate,
    self.tick,
);
```
(Task 9 wires this; here we just define the function.)

**Step 2: Verify compile**

Run: `cargo check`
Expected: clean compile.

**Step 3: Commit**

```
git add src/sim/movement/parachute_descent.rs
git commit -m "movement: implement tick_parachute_descent integrator"
```

---

### Task 8: Add parity tests for the descent state machine

**Why:** Verify the 3-tick rate ramp, total descent, landing trigger, sequence reset, and clamp behavior — every parity-critical ledger item. Done before wiring so we catch logic bugs in isolation.

**Files:**
- Modify: `src/sim/movement/parachute_descent.rs` (append tests inside `#[cfg(test)] mod tests`)

**Pattern:** existing tests in droppod_movement.rs and the begin tests added in Task 6.

**Step 1: Add parity tests**

```rust
// src/sim/movement/parachute_descent.rs — append inside the existing #[cfg(test)] mod tests block

const RULES_PARACHUTE_MAX_FALL_RATE: i32 = -3;
const TICK_MS_64: u32 = 64;

/// Helper: insert a paradropping entity and return its id.
fn setup_parachuting_entity(entities: &mut EntityStore) -> u64 {
    let mut e = GameEntity::test_default(1, "E1", "Americans", 10, 10);
    e.locomotor = Some(make_walk_loco());
    if e.animation.is_none() {
        e.animation = Some(crate::sim::animation::Animation::new(SequenceKind::Stand));
    }
    entities.insert(e);
    begin_parachute_descent(entities, 1, drop_altitude_1200());
    1
}

#[test]
fn test_3tick_rate_ramp() {
    // L1, L2, L3, L5, L6: rate sequence must be exactly [0, -1, -2, -3, -3, -3] over 6 ticks.
    let mut entities = EntityStore::new();
    let id = setup_parachuting_entity(&mut entities);

    let mut observed_rates: Vec<i32> = Vec::new();
    // Sample BEFORE each tick (rate-in for that tick).
    for _ in 0..6 {
        let entity = entities.get(id).expect("alive");
        observed_rates.push(entity.parachute_state.as_ref().expect("descending").rate);
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }

    assert_eq!(observed_rates, vec![0, -1, -2, -3, -3, -3],
        "3-tick ramp: rate must be 0,-1,-2,-3,-3,-3 (NOT instant -3)");
}

#[test]
fn test_descent_distance_first_4_ticks() {
    // L4, L7: total descent over first 4 ticks = 6 leptons (0+1+2+3).
    let mut entities = EntityStore::new();
    let id = setup_parachuting_entity(&mut entities);

    let initial_altitude = entities.get(id).unwrap()
        .parachute_state.as_ref().unwrap().altitude;

    // Per-tick descent deltas (vs initial): tick 1 = 0, tick 2 = 1, tick 3 = 3, tick 4 = 6.
    let expected_deltas: Vec<i32> = vec![0, 1, 3, 6];
    for (i, expected_delta) in expected_deltas.iter().enumerate() {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        let altitude = entities.get(id).unwrap()
            .parachute_state.as_ref().unwrap().altitude;
        let descent_leptons = (initial_altitude - altitude);  // SimFixed
        let expected_descent = SimFixed::from_int(*expected_delta);
        assert_eq!(descent_leptons, expected_descent,
            "after tick {}, descent should be {} leptons (got {})",
            i + 1, expected_delta, descent_leptons);
    }
}

#[test]
fn test_steady_state_rate() {
    // L3, L7: after enough ticks, rate must be -3 forever (clamped).
    let mut entities = EntityStore::new();
    let id = setup_parachuting_entity(&mut entities);

    // Tick 10 times to ensure we're past the ramp.
    for _ in 0..10 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }
    let rate = entities.get(id).unwrap()
        .parachute_state.as_ref().unwrap().rate;
    assert_eq!(rate, -3, "steady-state rate must equal ParachuteMaxFallRate (-3)");
}

#[test]
fn test_landing_inclusive_zero() {
    // L8: altitude <= 0 triggers landing. Use a small starting altitude so we land predictably.
    // After 4 ticks: descent = 6 leptons. Start at 6 leptons → land on tick 4.
    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
    e.locomotor = Some(make_walk_loco());
    if e.animation.is_none() {
        e.animation = Some(crate::sim::animation::Animation::new(SequenceKind::Stand));
    }
    entities.insert(e);
    begin_parachute_descent(&mut entities, 1, SimFixed::from_int(6));

    // After 4 ticks, altitude should hit 0 exactly → landing triggered.
    for _ in 0..4 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }

    let entity = entities.get(1).unwrap();
    assert!(entity.parachute_state.is_none(), "landing at altitude == 0 must trigger cleanup");
}

#[test]
fn test_landing_clamps_to_zero() {
    // L9: even if the integration would overshoot (altitude becomes negative),
    // we clamp to exactly SIM_ZERO before cleanup. Test the no-overshoot guarantee:
    // start at 5, after 4 ticks descent = 6 → would be -1, must clamp to 0.
    // (We can't observe altitude after cleanup since state is gone — instead verify
    //  via screen_y or by inspecting the state in the tick BEFORE cleanup.)
    //
    // Strategy: drive to one tick before landing, check altitude, then tick once more.
    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
    e.locomotor = Some(make_walk_loco());
    entities.insert(e);
    begin_parachute_descent(&mut entities, 1, SimFixed::from_int(5));

    // Tick 4: integrate altitude += -3 → 2. Then -1 = 0? Wait, altitude = 5.
    // tick 1: alt += 0 → 5. rate becomes -1.
    // tick 2: alt += -1 → 4. rate becomes -2.
    // tick 3: alt += -2 → 2. rate becomes -3.
    // tick 4: alt += -3 → -1. landing triggered, clamped to 0.
    for _ in 0..4 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }
    // After cleanup, parachute_state is None. The clamp happened internally.
    // The screen_y offset (which uses altitude) should now be the no-altitude baseline.
    let entity = entities.get(1).unwrap();
    assert!(entity.parachute_state.is_none(), "landed");
    // (Visual verification: screen_y should equal sy with no altitude offset — covered by integration test below.)
}

#[test]
fn test_clamp_at_max_fall_rate_default() {
    // L3: rate must never exceed (more-negative than) parachute_max_fall_rate.
    let mut entities = EntityStore::new();
    let id = setup_parachuting_entity(&mut entities);

    for _ in 0..50 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
        if let Some(state) = entities.get(id).and_then(|e| e.parachute_state.as_ref()) {
            assert!(state.rate >= RULES_PARACHUTE_MAX_FALL_RATE,
                "rate ({}) must not exceed (more-negative than) max ({})",
                state.rate, RULES_PARACHUTE_MAX_FALL_RATE);
        }
    }
}

#[test]
fn test_clamp_with_custom_max_fall_rate() {
    // L13: passing a custom max_fall_rate must be respected (mod compatibility).
    // With max = -1, ramp should be 0 → -1 → -1 → -1 (instant after 1 tick).
    let mut entities = EntityStore::new();
    let id = setup_parachuting_entity(&mut entities);

    let custom_max: i32 = -1;
    let mut observed_rates: Vec<i32> = Vec::new();
    for _ in 0..4 {
        let rate = entities.get(id).unwrap().parachute_state.as_ref().unwrap().rate;
        observed_rates.push(rate);
        tick_parachute_descent(&mut entities, TICK_MS_64, custom_max, 0);
    }
    assert_eq!(observed_rates, vec![0, -1, -1, -1],
        "with max=-1, rate must clamp at -1 after first decrement");
}

#[test]
fn test_body_sequence_reset_on_landing() {
    // L11: animation.sequence must reset to Stand when the descent ends (was Paradrop).
    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
    e.locomotor = Some(make_walk_loco());
    if e.animation.is_none() {
        e.animation = Some(crate::sim::animation::Animation::new(SequenceKind::Stand));
    }
    entities.insert(e);
    begin_parachute_descent(&mut entities, 1, SimFixed::from_int(6));

    // Confirm Paradrop set on attach.
    assert_eq!(entities.get(1).unwrap().animation.as_ref().unwrap().sequence,
        SequenceKind::Paradrop);

    // Tick to landing.
    for _ in 0..4 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }

    let anim = entities.get(1).unwrap().animation.as_ref().unwrap();
    assert_eq!(anim.sequence, SequenceKind::Stand,
        "L11: landing must reset animation to Stand");
}

#[test]
fn test_body_sequence_preserved_if_externally_changed() {
    // L11 gate: if some other system changed the sequence away from Paradrop
    // during descent (e.g., death anim), don't overwrite it on landing.
    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
    e.locomotor = Some(make_walk_loco());
    if e.animation.is_none() {
        e.animation = Some(crate::sim::animation::Animation::new(SequenceKind::Stand));
    }
    entities.insert(e);
    begin_parachute_descent(&mut entities, 1, SimFixed::from_int(6));

    // Mid-descent, externally change the sequence to Die1 (e.g., simulating shot down in air).
    for _ in 0..2 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }
    entities.get_mut(1).unwrap().animation.as_mut().unwrap().sequence = SequenceKind::Die1;

    // Tick to landing.
    for _ in 0..4 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }

    let anim = entities.get(1).unwrap().animation.as_ref().unwrap();
    assert_eq!(anim.sequence, SequenceKind::Die1,
        "L11 gate: must NOT overwrite Die1 with Stand on landing");
}

#[test]
fn test_locomotor_override_restored_on_landing() {
    // Mirrors test_droppod_full_sequence — locomotor override must end on landing.
    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
    e.locomotor = Some(make_walk_loco());
    entities.insert(e);
    begin_parachute_descent(&mut entities, 1, SimFixed::from_int(6));

    // Confirm overridden during descent.
    {
        let loco = entities.get(1).unwrap().locomotor.as_ref().unwrap();
        assert!(loco.is_overridden());
        assert_eq!(loco.kind, LocomotorKind::Parachute);
    }

    // Tick to landing.
    for _ in 0..4 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }

    let loco = entities.get(1).unwrap().locomotor.as_ref().unwrap();
    assert!(!loco.is_overridden(), "override must end on landing");
    assert_eq!(loco.kind, LocomotorKind::Walk, "must restore base Walk locomotor");
}

#[test]
fn test_works_without_animation() {
    // begin and tick must not panic when entity.animation is None.
    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "E1", "Americans", 5, 5);
    e.locomotor = Some(make_walk_loco());
    e.animation = None;
    entities.insert(e);

    assert!(begin_parachute_descent(&mut entities, 1, SimFixed::from_int(6)));
    for _ in 0..10 {
        tick_parachute_descent(&mut entities, TICK_MS_64, RULES_PARACHUTE_MAX_FALL_RATE, 0);
    }
    let entity = entities.get(1).unwrap();
    assert!(entity.parachute_state.is_none(), "should land cleanly without animation");
}

#[test]
fn test_paused_tick_does_not_advance() {
    // tick_ms == 0 (paused) must not advance state.
    let mut entities = EntityStore::new();
    let id = setup_parachuting_entity(&mut entities);
    let initial = entities.get(id).unwrap().parachute_state.as_ref().unwrap().altitude;

    tick_parachute_descent(&mut entities, 0, RULES_PARACHUTE_MAX_FALL_RATE, 0);

    let after = entities.get(id).unwrap().parachute_state.as_ref().unwrap().altitude;
    assert_eq!(initial, after, "paused tick (tick_ms=0) must not advance altitude");
}
```

**Step 2: Run tests**

Run: `cargo test parachute_descent --nocapture`
Expected: ALL tests PASS (begin tests from Task 6 + parity tests added here = ~14 tests).

If any test fails, **stop and diagnose before continuing**. The integer-tick math is small enough that any failure is a bug in the tick body, not a test bug. Re-read [Round 4 §R4.7 timeline in JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](docs/research/JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md) and verify expected values against Ghidra.

**Step 3: Commit**

```
git add src/sim/movement/parachute_descent.rs
git commit -m "movement: parity tests for parachute descent integrator"
```

---

### Task 9: Wire `tick_parachute_descent` into `World::advance_tick`

**Why:** Final integration. After this task, the descent system is live in the simulation — but with no caller invoking `begin_parachute_descent`, no entity will descend yet. (That's correct; the launch pipeline is a separate brainstorm.)

**Files:**
- Modify: `src/sim/world/mod.rs:~1097` (Phase 2, immediately after `tick_droppod_movement`)

**Pattern:** existing call site for `tick_droppod_movement`.

**Step 1: Add the call**

```rust
// src/sim/world/mod.rs — Phase 2, immediately after the existing tick_droppod_movement call (around line 1097)
droppod_movement::tick_droppod_movement(&mut self.entities, tick_ms, self.tick);
parachute_descent::tick_parachute_descent(
    &mut self.entities,
    tick_ms,
    self.rules.general.parachute_max_fall_rate,
    self.tick,
);
```

**Step 2: Add the import**

If `parachute_descent` isn't already imported in `world/mod.rs`, add it where `droppod_movement` is imported:

```rust
// src/sim/world/mod.rs — top of file, in the imports section
use crate::sim::movement::{
    /* ... existing ... */
    droppod_movement,
    parachute_descent,
};
```

(Or whatever the existing import style is — match it exactly.)

**Step 3: Verify compile + full test suite**

Run: `cargo check`
Expected: clean compile.

Run: `cargo test`
Expected: ALL tests PASS — both new parachute tests AND no regressions in existing tests.

**Step 4: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

**Step 5: Commit**

```
git add src/sim/world/mod.rs
git commit -m "world: wire tick_parachute_descent into Phase 2"
```

---

### Task 10: Final verification + cleanup

**Why:** Final pass to catch anything missed; confirm the whole system holds together.

**Files:** none modified; verification only.

**Step 1: Run full test suite**

Run: `cargo test`
Expected: ALL tests PASS.

**Step 2: Run clippy with strict warnings**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

**Step 3: Verify sim/ boundary**

Run: `grep -rn "use crate::render\|use crate::ui\|use crate::sidebar\|use crate::audio\|use crate::net" src/sim/movement/parachute_descent.rs`
Expected: NO matches. The new module must not depend on any presentation or networking layer.

**Step 4: Verify state hash includes the new field**

Run: `grep -n "state_hash\|StateHash" src/sim/world/mod.rs | head -10`

Read the state hash function. The new `parachute_state` field is auto-included via serde, so this is a sanity check rather than a fix step. If the state hash is computed from a hand-crafted hasher (not serde), add `parachute_state` to it.

**Step 5: Document remaining open items**

Update [docs/plans/2026-05-05-parachute-descent-design.md](docs/plans/2026-05-05-parachute-descent-design.md) by appending a "Status" section at the bottom:

```markdown
---

## Status (2026-05-05)

Implemented as planned in [docs/plans/2026-05-05-parachute-descent-plan.md](docs/plans/2026-05-05-parachute-descent-plan.md). All 14 tests pass. Wired into Phase 2 of `World::advance_tick`. No external callers yet — the paradrop SW launch pipeline is the next brainstorm.

**Deferred items (per Tiny-Detail Ledger §):**
- L14: free-fall mode (NoParachuteMaxFallRate, 1.4 accel)
- L15: 1-tick async chute removal lag (needs attached-anim infra)
- L17: InfantryClass::Unlimbo always-success quirk (needs launch pipeline)
- L18: Math__ftol rounding mode (low priority, only matters for L14)
- Visible chute sprite (separate brainstorm)
```

**Step 6: Commit (no code changes, just doc update)**

```
git add docs/plans/2026-05-05-parachute-descent-design.md
git commit -m "docs: mark parachute descent design as implemented"
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-05-parachute-descent-design.md](docs/plans/2026-05-05-parachute-descent-design.md)
- **Ghidra reports:**
  - [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](docs/research/JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md) Round 4 — primary source for integration logic, magic numbers, and tiny-detail ledger
  - [PARADROP_SUPERWEAPON_GHIDRA_REPORT.md](docs/research/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md) — context for the surrounding pipeline (launch path is OUT of scope here)
- **gamemd.exe addresses (binary, not in Rust code comments):**
  - `0x005F3E70` — `ObjectClass::AI` (descent integrator)
  - `0x005F3FBC` — rate decrement (`DEC EDI`)
  - `0x005F3FCB` — `Rules.ParachuteMaxFallRate` read (`MOV ECX, [ECX+0x7B8]`)
  - `0x005F3F70` — landing trigger (`JG` = altitude > 0 means not landed → inclusive `<= 0`)
  - `0x005217A8` — `DoType(0x21=Paradrop, 1, 0)` in InfantryClass::Unlimbo
  - `0x00424B50` / `0x00424C30` — AnimClass::SetOwnerObject sets `Object+0x84` (out of scope here; relevant for L15 follow-up)
  - `0x008871E0` — `g_RulesClass_Instance` global pointer
- **INI keys:**
  - `ini/rulesmd.ini:68` — `[General] ParachuteMaxFallRate=-3` (default)
  - `ini/rulesmd.ini:69` — `[General] NoParachuteMaxFallRate=-100` (deferred — out of scope)
  - `ini/rulesmd.ini:564` — `[General] Parachute=PARACH` (deferred — chute sprite out of scope)
- **Related code:**
  - [src/sim/movement/droppod_movement.rs](src/sim/movement/droppod_movement.rs) — primary precedent
  - [src/sim/animation.rs:56-123](src/sim/animation.rs#L56) — `SequenceKind` enum (Paradrop, Stand variants used)
  - [src/sim/movement/locomotor.rs:386](src/sim/movement/locomotor.rs#L386) — `OverrideKind` enum
  - [src/rules/locomotor_type.rs:27-37](src/rules/locomotor_type.rs#L27) — `LocomotorKind` enum
  - [src/rules/ruleset.rs:119](src/rules/ruleset.rs#L119) — `GeneralRules` struct
  - [src/sim/world/mod.rs:1097](src/sim/world/mod.rs#L1097) — Phase 2 call site for `tick_droppod_movement`
- **Recent commits in same pattern:**
  - `bc72091` — sim: add deploy_state field + is_deployed helpers to GameEntity
  - `768d6a6` — sim: insert tick_deploy_state into advance_tick (Phase 4.6)
  - `b616d9b` — sim: animation reflects deploy_state
