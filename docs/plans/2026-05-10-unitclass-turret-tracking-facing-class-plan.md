# UnitClass Turret Tracking — FacingClass Equivalent — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Reimplement gamemd.exe's per-tick turret rotation, fire-decision codes, and Fire_At_Target → Facing_Update tick order using a Rust-native `FacingClass` value type and a `FireDecision` enum, achieving observable parity with the binary's turret behavior on UnitClass instances.

**Architecture:** Introduce `FacingClass` (sim/movement) as a timer-based 16-bit interpolator mirroring the binary's 24-byte primitive, paired with a `FireDecision` enum (sim/combat) for behaviorally-distinct fire outcomes. Add a synthetic 15 Hz `binary_frame` counter on `Simulation` derived from accumulated `tick_ms`. Flip Phase 5 so combat reads barrel facing before turret rotation writes the next target. Body smoothing stays as a separate follow-up.

**Design Doc:** [docs/plans/2026-05-10-unitclass-turret-tracking-facing-class-design.md](docs/plans/2026-05-10-unitclass-turret-tracking-facing-class-design.md)

---

## Grounding Summary

**ra2-rust-game-docs/:** Single primary report `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` (just authored, confidence HIGH). All Ghidra evidence flows from it: FacingClass byte layout (§1.3), per-tick interpolation algorithm (§2.1), Set semantics (§2.4), SetROT clamp + shift (§2.7), Facing_Update structure (§5.1), TurretSpins formula (§5.2, deferred), GetFireError codes (§4.2), tick order (§7), `compute_facing_to_target` (§6). No companion FACING/FIRE/UNITCLASS/OPPORTUNITY-FIRE reports exist in the docs archive — this report is the sole reference for the binary primitives. Cross-referenced with `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` §726-728 (FacingClass offsets), `OPPORTUNITY_FIRE_GHIDRA_REPORT.md` §4 (mission 0x10), `GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md` (Type+0xCD5 IsGattling).

**Ghidra verification:** Addresses captured in design doc §Sources. All decompilations were freshly authored for this report; no follow-up Ghidra session needed for the implementation. Confidence HIGH on FacingClass internals, GetFireError code surface, and tick order. The mystery `+0x0C` field write (uninitialized stack source) is verified-from-binary as having no effect on interpolation — safe to omit.

**Repo pattern:** Closest existing pattern is `src/sim/movement/teleport_movement.rs::TeleportState` and `src/sim/superweapon/invulnerability.rs::InvulnerabilityState` — both are sim value types with serde-derived state, lifetime-tracked via tick counters, included in `world_hash::hash_entities()`. `FacingClass` follows the same shape: plain struct, derives `Copy + Clone + Serialize + Deserialize + Hash`, no methods that allocate. State-hash extension into `hash_entities` follows the existing pattern (line 314+; see `entity.attack_target` block at 372-378).

**INI keys:** `[UnitType] ROT=` parsed at `src/rules/object_type.rs:763` into `obj.turret_rot: i32`, with the existing Harvester-override-to-10 special case (verified against `UnitTypeClass::ReadINI 0x747620`). No new INI parsing needed for this round — `TurretSpins=`, `TurretLocked=`, `TurretScansNearby=` are deferred follow-ups (per design doc deferred items).

**Unknowns after grounding:**
- Atan2 axis convention discrepancy: our `facing_from_delta_int_u16` uses `atan2(dx, -dy)`, binary uses `atan2(dy, -dx)`. Whether these produce identical u16 values for the same geometry is unverified — and verifying requires gamemd ground-truth values (debugger trace or hand-derivation of the binary's full atan2-to-DirStruct conversion). **Deferred to a tracked follow-up** (Task 16) rather than gating this round, since the FacingClass refactor neither improves nor regresses whatever the current convention produces — it just stores and interpolates u16 values.

## Key Technical Decisions

- **Timer-based FacingClass mirroring binary's 24-byte struct** — the only design that captures snap-on-step<1, wrap-via-signed-short, smooth retarget, and ROT-dependent is_rotating in one primitive. **Confidence:** high. **Source:** ra2-rust-game-docs/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §2.1, §2.4, §2.5; design doc §Components.
- **Behavioral subset FireDecision enum (8 variants, code 5 collapsed)** — matches binary's distinct post-fire dispatch outcomes without threading 30 sub-reasons that all produce "no fire". **Confidence:** high. **Source:** doc §4.2, §4.7, §4.8.
- **Synthetic 15 Hz binary_frame counter on Simulation** — `binary_frame = (total_sim_ms * 15) / 1000`, drift-free, replicates binary's frame-counter semantics with zero accumulation drift across long sessions. **Confidence:** high. **Source:** design doc §World time fields; existing `rof_to_cooldown_ticks` precedent at `combat/mod.rs:1843` for converting binary frames → our ticks.
- **Tick order flip: tick_combat_with_fog → tick_turret_rotation in Phase 5** — combat reads previous tick's barrel facing, matching binary's Fire_At_Target → Facing_Update order. **Confidence:** high. **Source:** design doc §Data flow; doc §7.
- **Defer body smoothing (entity.facing: u8 stays untouched)** — independent refactor, FacingClass type already body-ready, scoped out per user choice in brainstorm. **Confidence:** high. **Source:** design doc §Body scope; user decision in brainstorm.
- **Compute is_rotating on demand (no cached field)** — two integer compares inlined; cache adds dirty-flag invariant + serialized state for no measurable benefit. **Confidence:** high. **Source:** design doc §FireDecision; user decision in brainstorm.
- **Command-handler bug fix bundled with migration** — current code at `combat/mod.rs:366,439` and `combat_targeting.rs:324` directly assigns `attacker.turret_facing = Some(desired_u16)`, snapping the turret instantly to the desired facing. This bypasses the rotation pipeline entirely (a pre-existing bug). Migration removes these direct writes; rotation now flows exclusively through `tick_turret_rotation`. **Confidence:** high. **Source:** code inspection of `combat/mod.rs:362-371, 435-444` and `combat_targeting.rs:312-329`.

## Open Questions

### Resolved During Planning

- **How to thread `binary_frame` to render-layer call sites?** Render already accepts a `Simulation` reference; expose `sim.binary_frame: u32` and let render call `entity.barrel_facing.as_ref().map(|f| f.current(sim.binary_frame))`. No new plumbing needed. **Source:** code inspection of `app_instances/units.rs:154-164` and `app_instances/shp.rs:315-358`.
- **Does `entity.turret_facing` participate in state hashing today?** No — `world_hash::hash_entities` (line 314) hashes `entity.facing` but skips `entity.turret_facing`. Adding `barrel_facing` to the hash is a new addition; load-bearing because combat decisions now depend on it.
- **Where do spawn sites initialize the turret?** Two places: `world_spawn.rs:140` and `world_spawn.rs:336`. Both call `body_facing_to_turret(facing)` to convert the initial body facing to a u16 turret facing. Migration: replace with `FacingClass::new(body_facing_to_turret(facing), obj.turret_rot as u8)`.

### Deferred to Implementation

- **Test fallout from Phase 5 tick-order flip.** Any existing test that asserts "issue attack, advance one tick, expect fired this tick" now expects fire on the tick AFTER alignment completes. Surface via `cargo test`, audit each, update assertion timing. Cannot enumerate up front without running the suite.
- **Whether `facing_from_delta_int_u16` axis convention matches binary.** Tracked as Task 16 follow-up; requires gamemd debugger trace or full hand-derivation of binary's atan2→DirStruct conversion (only partial info in research doc §6).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/movement/facing_class.rs` | FacingClass timer-based 16-bit interpolator + tests |
| Create | `src/sim/combat/fire_decision.rs` | FireDecision enum + post-fire dispatch helpers + tests |
| Modify | `src/sim/movement/mod.rs` | Add `pub mod facing_class;` and re-export `FacingClass` |
| Modify | `src/sim/combat/mod.rs` | Add `mod fire_decision;`; rewrite alignment check + post-dispatch around FireDecision; remove `attacker.turret_facing = Some(...)` snap writes |
| Modify | `src/sim/movement/turret.rs` | Rewrite `tick_turret_rotation` around FacingClass; signature changes from `tick_ms: u32` to `binary_frame: u32`; delete `rot_to_facing_delta_u16` and `is_turret_aligned_u16` |
| Modify | `src/sim/game_entity.rs` | Replace `turret_facing: Option<u16>` with `barrel_facing: Option<FacingClass>`; update `test_default` and `Default` impls |
| Modify | `src/sim/world/mod.rs` | Add `total_sim_ms: u64` and `binary_frame: u32` fields on `Simulation`; advance both at top of `advance_tick`; flip Phase 5 order: `tick_combat_with_fog` then `tick_turret_rotation` |
| Modify | `src/sim/world/world_hash.rs` | Hash `total_sim_ms`, `binary_frame`; hash `entity.barrel_facing` in `hash_entities` |
| Modify | `src/sim/world/world_spawn.rs` | At lines 140 and 336, replace `Some(body_facing_to_turret(facing))` with `Some(FacingClass::new(body_facing_to_turret(facing), obj.turret_rot as u8))` |
| Modify | `src/sim/combat/combat_targeting.rs` | `AttackerSnapshot::turret_facing: Option<u16>` → `barrel_facing: Option<FacingClass>`; remove direct `entity.turret_facing = Some(desired_u16)` write at line 324 |
| Modify | `src/app_instances/units.rs` | Migrate read at line 154 to `entity.barrel_facing.as_ref().map(|f| f.current(sim.binary_frame))`; thread `sim.binary_frame` through call site |
| Modify | `src/app_instances/shp.rs` | Same migration at line 315 |

`canonical_turret_facing(u16)` at `src/render/unit_atlas.rs:1056` requires no change — it operates on the u16 facing value, not the storage type.

## Interface Changes

**Public types added:**
- `pub struct FacingClass` (sim/movement/facing_class.rs) — exported via `sim::movement::FacingClass`. Used by `GameEntity::barrel_facing` and `AttackerSnapshot::barrel_facing`.
- `pub enum FireDecision` (sim/combat/fire_decision.rs) — used internally in combat module.

**Public types modified:**
- `GameEntity::turret_facing: Option<u16>` → `GameEntity::barrel_facing: Option<FacingClass>`. Snapshot serialization format CHANGES — old snapshots will not deserialize. Acceptable: there is no on-disk save format yet; in-process snapshots are short-lived.
- `Simulation` gains `total_sim_ms: u64` and `binary_frame: u32` (both serialized).
- `AttackerSnapshot::turret_facing: Option<u16>` → `barrel_facing: Option<FacingClass>` (internal to combat module).

**Function signature changes:**
- `tick_turret_rotation(entities, rules, tick_ms: u32, interner)` → `tick_turret_rotation(entities, rules, binary_frame: u32, interner)`. Caller in `world/mod.rs:1150` updates to pass `self.binary_frame`.

**Functions deleted:**
- `is_turret_aligned_u16(turret_facing: u16, target_facing: u16) -> bool` (turret.rs:90) — replaced by `barrel.current(binary_frame) == desired && !barrel.is_rotating(binary_frame)`.
- `rot_to_facing_delta_u16(rot: i32, tick_ms: u32) -> u16` (turret.rs:76) — subsumed by `FacingClass::set_rot`.
- `shortest_rotation_u16(current: u16, target: u16) -> i32` (turret.rs:60) — no longer needed; FacingClass handles wrap internally.

The 8-bit cousins (`shortest_rotation`, `rot_to_facing_delta`) STAY — body smoothing migration will retire them in a future round.

## Sim Checklist

- [x] All math uses `fixed`-point or integer — no f32/f64 in game logic. FacingClass uses i16/u16/u32 only. `facing_toward_lepton` keeps existing f32 atan2 (unchanged in this round, deferred to Task 16).
- [x] New state included in deterministic state hash. Task 1 adds `total_sim_ms` + `binary_frame`. Task 7 adds `barrel_facing` to `hash_entities`.
- [x] No dependencies on render/ui/sidebar/audio/net. FacingClass and FireDecision are pure sim types.
- [x] Tick ordering impact noted. Phase 5 order changes from `turret → combat` to `combat → turret`. Documented in Task 13 with regression-test plan in Task 14.
- [x] BTreeMap iteration order preserved. `tick_turret_rotation` uses existing `entities.keys_sorted()` pattern.

## Risk Areas

From the design doc's Impact Analysis:

1. **Tick-order flip surfaces test fallout.** Any test that assumes "issue attack, advance one tick, expect fire" will fail. Mitigation: Task 14 enumerates expected fallout and Task 15 audits the suite.
2. **Atan2 axis convention discrepancy.** Existing tests in `fixed_math.rs` may pass under our convention but diverge from binary. Mitigation: Task 16 (deferred follow-up) will gather binary ground-truth values and assert byte-exact match.
3. **Snapshot serialization format change.** Pre-existing snapshots will not deserialize after `barrel_facing` field rename. Mitigation: no on-disk save format exists yet; in-memory snapshots are recreated per session. Documented in Interface Changes.
4. **Removing `attacker.turret_facing = Some(...)` in command handlers fixes a pre-existing snap bug.** Some tests may have implicitly depended on the snap. Mitigation: Task 14 covers explicit "rotate-then-fire" timing assertions that should now pass; Task 15 audits any test that assumed instant turret aim post-command.
5. **Render layer reads `entity.barrel_facing` in display path.** If `binary_frame` is stale or out-of-sync between sim and render thread, animated value could be stale by one frame. Mitigation: render reads happen after `advance_tick` returns (single-threaded for this concern); `binary_frame` is updated at the top of `advance_tick`.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | FacingClass interpolation curve (animated value at every binary frame) | Player sees turret rotation cadence every match; off-by-one frame is visibly wrong | Unit tests in Task 4 with hand-computed expected values from doc §2.1 formula; integration test in Task 14 for visible rotation arc |
| Task 4 | step_size < 1 snap behavior | Tiny rotation requests (smaller than one frame's ROT) snap instead of interpolate; affects retarget responsiveness when target moves slightly | Unit test in Task 4 |
| Task 4 | Wrap-around via signed short subtraction | Turret going 0xFFE0→0x0010 must traverse +0x30 (the short way), not -0xFFD0; visibly wrong if backward | Unit test in Task 4 |
| Task 5 | FacingClass::set snapshots animated into prev | Mid-rotation retarget must continue from current animated position, not snap back to prev — visibly jarring otherwise | Unit test in Task 5; integration test in Task 14 |
| Task 6 | ROT byte clamp at 0x7F + shift <<8 | Affects per-frame step magnitude; even small drift compounds visibly across rotations | Unit test in Task 6 with hand-computed values |
| Task 11 | Idle no-target turret returns to body facing | Player sees turret track body when not engaged; current Rust has a "spin halfway ahead" hack on harvesters | Behavior verified manually + integration test in Task 14 |
| Task 12 | Turret-alignment check uses FacingClass.current(frame) == desired AND !is_rotating | Slow-ROT turrets must take more frames to align than fast-ROT (current 2048 flat-tolerance is wrong); player sees wrong fire timing on heavy-ROT units | Integration test in Task 14: ROT=2 vs ROT=10 acquisition timing |
| Task 13 | Phase 5 tick order: combat-then-rotation | Fire decision uses last frame's facing (1-tick acquisition latency); player feels acquisition cadence match gamemd | Integration test in Task 14: 1-tick acquisition latency assertion |
| Task 12 | Gattling stage spin-up gates on FireDecision ∈ {Fire, Facing, Cooldown, Generic} | Gattling weapons must spin up while rotating-into-position, not just while firing; player sees spin-up cadence | Unit test in Task 3 + integration test in Task 14 |
| Task 11 | rot_byte from obj.turret_rot at spawn (not hardcoded 5) | Per-unit ROT in rules.ini must drive actual rotation rate; current `tick_turret_rotation` defaults to 5 if rules lookup fails | Spawn-test verification in Task 7 |

---

## Tasks

### Task 1: Add `total_sim_ms` and `binary_frame` to Simulation; advance and hash both

**Why:** Every subsequent task depends on having a synthetic 15 Hz binary-frame counter on `Simulation`. Doing this first means FacingClass tests can use a deterministic frame counter from day one.

**Files:**
- Modify: `src/sim/world/mod.rs:192-280` (struct definition), `src/sim/world/mod.rs:336-380` (`Simulation::new`), `src/sim/world/mod.rs:954-963` (`advance_tick` top)
- Modify: `src/sim/world/world_hash.rs:18-38` (top-level state hash)

**Pattern:** Mirrors the existing `pub tick: u64` field at `world/mod.rs:200`. Hash addition mirrors `self.tick.hash(&mut hasher)` at `world_hash.rs:21`.

**Step 1: Add fields to `Simulation` struct**

In `src/sim/world/mod.rs`, after the `pub tick: u64,` field (line 200), add:

```rust
    /// Total accumulated sim-tick milliseconds since world creation.
    /// Authoritative time source; binary_frame is derived from this.
    pub total_sim_ms: u64,
    /// Synthetic gamemd 15 Hz frame counter. Computed each tick as
    /// (total_sim_ms * 15 / 1000). Used by FacingClass methods to compute
    /// animated values that match gamemd binary-frame timing exactly.
    pub binary_frame: u32,
```

**Step 2: Initialize in `Simulation::new()`**

In the `Simulation::new()` constructor (around line 336), add to the struct initializer:

```rust
            total_sim_ms: 0,
            binary_frame: 0,
```

**Step 3: Advance at top of `advance_tick`**

In `src/sim/world/mod.rs`, in `advance_tick` immediately after the function signature (right before line 963 `let execute_tick = self.tick.saturating_add(1);`), add:

```rust
        // Advance synthetic 15 Hz binary-frame counter. Drift-free: every
        // binary-frame boundary is exactly when total_sim_ms crosses a
        // multiple of 1000/15 ≈ 66.67ms.
        self.total_sim_ms = self.total_sim_ms.saturating_add(tick_ms as u64);
        self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32;
```

**Step 4: Add to state hash**

In `src/sim/world/world_hash.rs`, in `state_hash()` (around line 21), after `self.tick.hash(&mut hasher);`, add:

```rust
        self.total_sim_ms.hash(&mut hasher);
        self.binary_frame.hash(&mut hasher);
```

**Step 5: Add unit test for binary_frame advancement**

Append to `src/sim/world/world_hash.rs` (in the existing test module structure, or a new `mod binary_frame_tests` block at the bottom):

```rust
#[cfg(test)]
mod binary_frame_tests {
    use super::Simulation;
    use std::collections::BTreeMap;

    #[test]
    fn binary_frame_drift_free_at_22ms_ticks() {
        let mut sim = Simulation::new();
        let height_map = BTreeMap::new();
        // 45 ticks at 22ms = 990ms ≈ 14.85 binary frames; floor = 14.
        for _ in 0..45 {
            sim.advance_tick(&[], None, &height_map, None, None, 22);
        }
        assert_eq!(sim.total_sim_ms, 990);
        assert_eq!(sim.binary_frame, 14);
    }

    #[test]
    fn binary_frame_advances_each_66ms_block() {
        let mut sim = Simulation::new();
        let height_map = BTreeMap::new();
        // Three 67ms ticks should each advance binary_frame by 1
        // (67ms * 15 / 1000 = 1.005, floor = 1 per tick).
        sim.advance_tick(&[], None, &height_map, None, None, 67);
        assert_eq!(sim.binary_frame, 1);
        sim.advance_tick(&[], None, &height_map, None, None, 67);
        assert_eq!(sim.binary_frame, 2);
        sim.advance_tick(&[], None, &height_map, None, None, 67);
        assert_eq!(sim.binary_frame, 3);
    }

    #[test]
    fn binary_frame_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        let height_map = BTreeMap::new();
        sim_a.advance_tick(&[], None, &height_map, None, None, 100);
        // sim_b stays at frame 0; sim_a is at (100*15/1000)=1.
        assert_ne!(sim_a.state_hash(), sim_b.state_hash());
    }
}
```

**Step 6: Verify**

Run: `cargo test -p ra2-rust-game binary_frame_tests`
Expected: all 3 tests PASS.

Run: `cargo build` (catch any other call sites broken by struct field addition).
Expected: clean build.

**Step 7: Commit**
```
sim/world: add binary_frame + total_sim_ms synthetic 15 Hz counter

Foundation for FacingClass turret rotation: gamemd-frame-counter
equivalent advanced from accumulated tick_ms. Drift-free formula
(total_sim_ms * 15 / 1000). Hashed for determinism.
```

---

### Task 2: Create `FireDecision` enum + tests

**Why:** Independent of FacingClass; can be defined first as the type that combat will switch on. Pure data, easy to test.

**Files:**
- Create: `src/sim/combat/fire_decision.rs`
- Modify: `src/sim/combat/mod.rs:20-26` (module declarations)

**Pattern:** Plain enum mirroring existing `EntityCategory` and similar sim enums. Tests inline via `#[cfg(test)] mod tests`.

**Step 1: Create the file**

`src/sim/combat/fire_decision.rs`:

```rust
//! Per-tick fire-decision outcomes for one attacker.
//!
//! Behavioral subset of gamemd's GetFireError codes (see
//! ra2-rust-game-docs/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md
//! §4.2). Code 5 (Generic) collapses ~30 binary sub-reasons since they all
//! map to "no fire this tick"; threading sub-reason complexity buys zero
//! observable difference.
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on standard library.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FireDecision {
    Fire,
    Cooldown,
    Facing,
    Range,
    NoAmmo,
    CloakedTarget,
    ForceFire,
    Generic,
}

impl FireDecision {
    /// Whether this decision drives gattling-weapon spin-up (gamemd codes
    /// {0, 2, 3, 4} per research doc §4.8). Code 4 is unmapped in our enum;
    /// we approximate with Generic since it covers "rotation/cooldown-related
    /// no-fire" cases.
    pub fn drives_gattling_spinup(self) -> bool {
        matches!(
            self,
            Self::Fire | Self::Facing | Self::Cooldown | Self::Generic
        )
    }

    /// Whether this decision means "fire happens this tick".
    pub fn is_fire(self) -> bool {
        matches!(self, Self::Fire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drives_gattling_spinup_truth_table() {
        assert!(FireDecision::Fire.drives_gattling_spinup());
        assert!(FireDecision::Facing.drives_gattling_spinup());
        assert!(FireDecision::Cooldown.drives_gattling_spinup());
        assert!(FireDecision::Generic.drives_gattling_spinup());

        assert!(!FireDecision::Range.drives_gattling_spinup());
        assert!(!FireDecision::NoAmmo.drives_gattling_spinup());
        assert!(!FireDecision::CloakedTarget.drives_gattling_spinup());
        assert!(!FireDecision::ForceFire.drives_gattling_spinup());
    }

    #[test]
    fn is_fire_only_for_fire_variant() {
        assert!(FireDecision::Fire.is_fire());
        assert!(!FireDecision::Facing.is_fire());
        assert!(!FireDecision::ForceFire.is_fire());
        assert!(!FireDecision::Cooldown.is_fire());
    }
}
```

**Step 2: Register the module**

In `src/sim/combat/mod.rs`, in the module declarations block (around line 20-26), add:

```rust
pub(crate) mod fire_decision;
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game fire_decision::tests`
Expected: 2 tests PASS.

**Step 4: Commit**
```
combat: add FireDecision enum (8-variant subset of gamemd GetFireError)

Behavioral subset of gamemd's ~10 fire-error codes; collapses code 5's
~30 sub-reasons into FireDecision::Generic. Used by upcoming combat
loop refactor to drive gattling spin-up + force-fire dispatch.
```

---

### Task 3: Create `FacingClass` skeleton (struct + new + set_rot + destination)

**Why:** Define the type and its constructors before adding behavior. Makes Task 4-6 incremental (each adds one method with its own tests).

**Files:**
- Create: `src/sim/movement/facing_class.rs`
- Modify: `src/sim/movement/mod.rs` (add `pub mod facing_class; pub use facing_class::FacingClass;`)

**Pattern:** Mirrors `src/sim/movement/teleport_movement.rs::TeleportState` shape — plain struct, derives `Copy + Clone + Debug + PartialEq + Eq + Hash + Serialize + Deserialize`, no allocations, sim-only.

**Step 1: Create the file with struct + constructors + ROT setter**

`src/sim/movement/facing_class.rs`:

```rust
//! Timer-based 16-bit facing interpolator, mirroring gamemd's FacingClass primitive.
//!
//! At any binary frame, the animated value is a pure function of state +
//! frame: `current(frame) = prev + sign(diff) * rot_per_frame * elapsed`.
//! Setting a new target snapshots the current animated value into `prev`
//! so rotations retarget smoothly without snap-back.
//!
//! Verified against gamemd.exe — see
//! ra2-rust-game-docs/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md
//! §1.3 (24-byte byte layout), §2.1 (Current interpolation), §2.4 (Set
//! semantics), §2.7 (SetROT clamp + shift).
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on serde and std.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FacingClass {
    /// Destination — where the rotation will end up. 16-bit DirStruct.
    current: u16,
    /// Where the current rotation began. Updated on `set` to the animated
    /// value at the moment of the new request, so retargets continue
    /// smoothly from the visible position.
    prev: u16,
    /// Binary frame when the rotation began. None = never started.
    start_frame: Option<u32>,
    /// Total binary frames needed to complete the rotation. When this is
    /// 0, `current()` returns `current` immediately (snap-on-step<1).
    duration_frames: u16,
    /// Per-frame step in 16-bit facing units. Stored as `(rot_byte << 8)`.
    /// Zero means instant rotator (no interpolation).
    rot_per_frame: u16,
}

impl FacingClass {
    /// Construct a new FacingClass at the given initial facing with the
    /// given ROT byte. ROT byte is the value from rules.ini (e.g. 5 for
    /// War Miner, 10 for Harvester) before the binary's <<8 shift.
    pub fn new(initial: u16, rot_byte: u8) -> Self {
        let mut fc = Self {
            current: initial,
            prev: initial,
            start_frame: None,
            duration_frames: 0,
            rot_per_frame: 0,
        };
        fc.set_rot(rot_byte);
        fc
    }

    /// Update the rate of turn. Mirrors gamemd's SetROT (FUN_004C9680):
    /// clamps input > 126 to 127, then stores `(byte << 8)`.
    pub fn set_rot(&mut self, rot_byte: u8) {
        let clamped: u8 = if rot_byte > 0x7E { 0x7F } else { rot_byte };
        self.rot_per_frame = (clamped as u16) << 8;
    }

    /// Destination facing — where the rotation will end (regardless of
    /// where the animation currently is).
    pub fn destination(&self) -> u16 {
        self.current
    }

    /// Per-frame step value, exposed for tests and for callers that need
    /// to know the rotation rate.
    pub fn rot_per_frame(&self) -> u16 {
        self.rot_per_frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_at_given_facing() {
        let fc = FacingClass::new(12345, 5);
        assert_eq!(fc.destination(), 12345);
        assert_eq!(fc.rot_per_frame(), 5 * 256); // 5 << 8 = 1280
    }

    #[test]
    fn set_rot_clamps_at_0x7f() {
        let mut fc = FacingClass::new(0, 0);
        fc.set_rot(0x7E);
        assert_eq!(fc.rot_per_frame(), 0x7E00); // 126 << 8
        fc.set_rot(0x7F);
        assert_eq!(fc.rot_per_frame(), 0x7F00); // 127 << 8
        fc.set_rot(0xFF);
        assert_eq!(fc.rot_per_frame(), 0x7F00); // clamped to 127, 127 << 8
        fc.set_rot(200);
        assert_eq!(fc.rot_per_frame(), 0x7F00); // clamped
    }

    #[test]
    fn set_rot_zero_means_instant() {
        let mut fc = FacingClass::new(100, 5);
        fc.set_rot(0);
        assert_eq!(fc.rot_per_frame(), 0);
    }
}
```

**Step 2: Register the module**

In `src/sim/movement/mod.rs`, add (alongside existing module declarations):

```rust
pub mod facing_class;
pub use facing_class::FacingClass;
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game facing_class::tests`
Expected: 3 tests PASS.

**Step 4: Commit**
```
movement: add FacingClass struct skeleton (new, set_rot, destination)

Timer-based 16-bit facing interpolator mirroring gamemd's 24-byte
primitive. ROT clamp at 0x7F + shift <<8 verified against
SetROT (FUN_004C9680). Behavior methods (current, set, snap,
is_rotating) added in subsequent tasks.
```

---

### Task 4: Implement `FacingClass::current()` interpolation algorithm + tests

**Why:** `current()` is the core interpolator that all other behavior reads through. Writing it before `set()` means we can test interpolation in isolation by manually constructing FacingClass instances mid-rotation.

**Files:**
- Modify: `src/sim/movement/facing_class.rs` (add `current()` method + tests)

**Pattern:** Pure function of state + frame, mirroring the C decompilation in research doc §2.1.

**Step 1: Add the `current()` method**

Append to the `impl FacingClass` block in `src/sim/movement/facing_class.rs`:

```rust
    /// Animated facing at the given binary frame. Pure function of state.
    ///
    /// Returns `current` when:
    /// - rot_per_frame == 0 (instant rotator)
    /// - start_frame is None (no rotation initiated)
    /// - elapsed >= duration_frames (rotation complete)
    /// - step_size < 1 (rotation request smaller than one frame's ROT — snaps)
    ///
    /// Otherwise interpolates linearly along the shortest signed arc from
    /// prev to current at exactly rot_per_frame units per frame.
    pub fn current(&self, binary_frame: u32) -> u16 {
        if self.rot_per_frame == 0 {
            return self.current;
        }
        let Some(start) = self.start_frame else {
            return self.current;
        };
        let elapsed: u32 = binary_frame.saturating_sub(start);
        if elapsed >= self.duration_frames as u32 {
            return self.current;
        }
        let remaining: u16 = self.duration_frames - elapsed as u16;

        // Signed short subtraction gives shortest signed delta.
        // 0xFFE0 → 0x0010 wraps to +0x30, not -0xFFD0.
        let diff: i16 = self.current.wrapping_sub(self.prev) as i16;

        // step_size < 1 snaps (research doc §2.2).
        let step_size: u16 = diff.unsigned_abs() / self.rot_per_frame;
        if step_size < 1 {
            return self.current;
        }

        // animated = current - sign(diff) * rot_per_frame * remaining
        // (equivalent to: prev + sign(diff) * rot_per_frame * elapsed)
        let signed_step: i32 = (diff.signum() as i32) * (self.rot_per_frame as i32);
        let delta: i32 = signed_step * (remaining as i32);
        ((self.current as i32) - delta).rem_euclid(65536) as u16
    }
}
```

**Note:** the closing `}` above replaces the existing closing `}` of the `impl FacingClass` block — make sure not to add a duplicate.

**Step 2: Add unit tests for `current()`**

Append to the `#[cfg(test)] mod tests` block:

```rust
    /// Helper: construct a FacingClass mid-rotation (skips set() so we can
    /// test current() in isolation).
    fn mid_rotation(prev: u16, current: u16, start: u32, duration: u16, rot_byte: u8) -> FacingClass {
        let mut fc = FacingClass::new(current, rot_byte);
        fc.prev = prev;
        fc.start_frame = Some(start);
        fc.duration_frames = duration;
        fc
    }

    #[test]
    fn current_returns_destination_when_rot_zero() {
        let mut fc = mid_rotation(0, 1000, 0, 10, 5);
        fc.set_rot(0);
        assert_eq!(fc.current(0), 1000);
        assert_eq!(fc.current(5), 1000);
        assert_eq!(fc.current(100), 1000);
    }

    #[test]
    fn current_returns_destination_when_no_start_frame() {
        let fc = FacingClass::new(1000, 5);
        // start_frame = None
        assert_eq!(fc.current(0), 1000);
        assert_eq!(fc.current(50), 1000);
    }

    #[test]
    fn current_returns_destination_when_elapsed_exceeds_duration() {
        // prev=0, current=12800 (10 frames at ROT=5 → 1280/frame), duration=10.
        let fc = mid_rotation(0, 12800, 0, 10, 5);
        assert_eq!(fc.current(10), 12800); // exactly at end
        assert_eq!(fc.current(15), 12800); // past end
        assert_eq!(fc.current(100), 12800);
    }

    #[test]
    fn current_interpolates_linearly() {
        // prev=0, current=12800 (= 10 frames * 1280), duration=10.
        // At elapsed=5, animated = 0 + 5 * 1280 = 6400.
        // Equivalently: animated = current - 5 * 1280 = 12800 - 6400 = 6400.
        let fc = mid_rotation(0, 12800, 0, 10, 5);
        assert_eq!(fc.current(0), 0);    // remaining=10, animated = 12800 - 1280*10 = 0
        assert_eq!(fc.current(1), 1280);
        assert_eq!(fc.current(5), 6400);
        assert_eq!(fc.current(9), 11520);
    }

    #[test]
    fn current_snaps_when_step_size_below_one() {
        // diff = current - prev = 100; rot_per_frame = 1280; step_size = 100/1280 = 0.
        // Should snap to current immediately.
        let fc = mid_rotation(0, 100, 0, 0, 5);
        assert_eq!(fc.current(0), 100);
    }

    #[test]
    fn current_handles_wrap_around_short_path() {
        // 0xFFE0 → 0x0010: shortest signed delta is +0x30 (48 units), not -0xFFD0.
        // ROT=1 byte → 256/frame; duration = 48/256 = 0 → snaps. Use larger arc.
        // Let's go 0xFF00 → 0x0100: signed diff = 0x0100 - 0xFF00 (as i16) = 0x0200 = 512.
        // ROT=1, rot_per_frame=256; duration = 512/256 = 2.
        let fc = mid_rotation(0xFF00, 0x0100, 0, 2, 1);
        // At elapsed=0: animated = current - sign(+) * 256 * 2 = 0x0100 - 512 = 0xFF00.
        assert_eq!(fc.current(0), 0xFF00);
        // At elapsed=1: animated = 0x0100 - 256 = 0 (i.e., 0x0000, just past wrap).
        assert_eq!(fc.current(1), 0x0000);
        // At elapsed=2: complete.
        assert_eq!(fc.current(2), 0x0100);
    }

    #[test]
    fn current_handles_wrap_around_short_path_negative_diff() {
        // 0x0100 → 0xFF00: signed diff = (0xFF00 - 0x0100) as i16 = 0xFE00 = -512.
        // shortest path is COUNTER-clockwise by 512 units (back through 0).
        let fc = mid_rotation(0x0100, 0xFF00, 0, 2, 1);
        // At elapsed=0: animated = 0xFF00 - sign(-) * 256 * 2 = 0xFF00 + 512 = 0x0100.
        assert_eq!(fc.current(0), 0x0100);
        // At elapsed=1: animated = 0xFF00 + 256 = 0x0000.
        assert_eq!(fc.current(1), 0x0000);
        // At elapsed=2: complete.
        assert_eq!(fc.current(2), 0xFF00);
    }
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game facing_class::tests::current`
Expected: 7 `current_*` tests PASS.

**Step 4: Commit**
```
movement/facing_class: add current() interpolation per research doc §2.1

Pure function of state + binary_frame. Snap-on-step<1, wrap via
signed short subtraction, returns destination when rot=0 / no start /
expired. Tests cover linear interpolation, wrap-around shortest-path,
and snap edge cases.
```

---

### Task 5: Implement `FacingClass::set()` smooth setter + tests

**Why:** `set()` is the API combat and turret rotation use to initiate a new rotation. Snapshots the current animated position into `prev` so retargets are smooth.

**Files:**
- Modify: `src/sim/movement/facing_class.rs` (add `set()` method + tests)

**Step 1: Add the `set()` method**

Append to the `impl FacingClass` block (before the closing `}`):

```rust
    /// Smooth setter — initiates a new rotation toward `new_target`.
    /// Snapshots the current animated position into `prev` (research doc
    /// §2.4) so retargets continue smoothly from the visible position.
    ///
    /// No-op when `new_target == current`. Returns true if state changed.
    pub fn set(&mut self, new_target: u16, binary_frame: u32) -> bool {
        if new_target == self.current {
            return false;
        }
        if self.rot_per_frame > 0 {
            // Snapshot animated value into prev BEFORE writing new target.
            self.prev = self.current(binary_frame);
        } else {
            self.prev = self.current;
        }
        self.current = new_target;
        if self.rot_per_frame > 0 {
            let diff: i16 = self.current.wrapping_sub(self.prev) as i16;
            self.duration_frames = diff.unsigned_abs() / self.rot_per_frame;
            self.start_frame = Some(binary_frame);
        }
        true
    }
```

**Step 2: Add unit tests for `set()`**

Append to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn set_no_op_when_target_equals_current() {
        let mut fc = FacingClass::new(1000, 5);
        let changed = fc.set(1000, 0);
        assert!(!changed);
        assert_eq!(fc.destination(), 1000);
        assert!(fc.start_frame.is_none());
    }

    #[test]
    fn set_initiates_rotation_when_target_differs() {
        let mut fc = FacingClass::new(0, 5);
        let changed = fc.set(12800, 0);
        assert!(changed);
        assert_eq!(fc.destination(), 12800);
        assert_eq!(fc.start_frame, Some(0));
        // duration = abs(12800 - 0) / 1280 = 10.
        assert_eq!(fc.duration_frames, 10);
    }

    #[test]
    fn set_snapshots_animated_into_prev_mid_rotation() {
        let mut fc = FacingClass::new(0, 5);
        fc.set(12800, 0); // start rotation: 0 → 12800 over 10 frames.

        // At frame 5, animated = 6400. Now retarget.
        assert_eq!(fc.current(5), 6400);
        fc.set(25600, 5);

        // After re-set, prev should be 6400 (the animated position at the
        // moment of the new request), NOT 0 (the old prev).
        assert_eq!(fc.prev, 6400);
        assert_eq!(fc.destination(), 25600);
        assert_eq!(fc.start_frame, Some(5));
        // New duration = abs(25600 - 6400) / 1280 = 19200 / 1280 = 15.
        assert_eq!(fc.duration_frames, 15);
    }

    #[test]
    fn set_with_zero_rot_writes_destination_without_timer() {
        let mut fc = FacingClass::new(0, 0);
        let changed = fc.set(1000, 5);
        assert!(changed);
        assert_eq!(fc.destination(), 1000);
        // No timer state set when rot is 0.
        assert!(fc.start_frame.is_none());
    }

    #[test]
    fn set_handles_wrap_around_shortest_path() {
        // From 0x0100 to 0xFF00, shortest path is COUNTER-clockwise (-512 units).
        let mut fc = FacingClass::new(0x0100, 1);
        fc.set(0xFF00, 0);
        // duration = abs((-512 as i16).unsigned_abs()) / 256 = 512/256 = 2.
        assert_eq!(fc.duration_frames, 2);
    }
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game facing_class::tests::set`
Expected: 5 `set_*` tests PASS.

**Step 4: Commit**
```
movement/facing_class: add set() smooth setter per research doc §2.4

Snapshots animated position into prev before writing new target,
enabling smooth mid-rotation retargets. No-op when target equals
current. Computes duration via signed short diff for wrap-correct
shortest-path rotation.
```

---

### Task 6: Implement `FacingClass::snap()` and `is_rotating()` + tests

**Why:** Completes the FacingClass API. `snap()` is needed for spawn / locomotor takeoff / deploy paths that want no smoothing; `is_rotating()` is the equivalent of the binary's `field_0x4A0` per-tick latch (research doc §5.5).

**Files:**
- Modify: `src/sim/movement/facing_class.rs` (add `snap()` and `is_rotating()` + tests)

**Step 1: Add the methods**

Append to the `impl FacingClass` block:

```rust
    /// Snap setter — writes target to both current and prev, resets the
    /// timer. Mirrors gamemd's UpdateFacing (FUN_004C9300) used by spawn /
    /// locomotor takeoff / deploy paths that want no smoothing.
    /// Returns true if the destination changed.
    pub fn snap(&mut self, new_target: u16, binary_frame: u32) -> bool {
        let animated = self.current(binary_frame);
        if animated == new_target && self.current == new_target {
            self.duration_frames = 0;
            return false;
        }
        self.current = new_target;
        self.prev = new_target;
        self.start_frame = Some(binary_frame);
        self.duration_frames = 0;
        true
    }

    /// Whether a rotation is currently in progress at the given binary
    /// frame. Equivalent to gamemd's CDTimerClass::Remaining test on
    /// FacingClass (research doc §5.5, §1.5). Computed on demand —
    /// not cached.
    pub fn is_rotating(&self, binary_frame: u32) -> bool {
        if self.rot_per_frame == 0 {
            return false;
        }
        let Some(start) = self.start_frame else {
            return false;
        };
        let elapsed: u32 = binary_frame.saturating_sub(start);
        (elapsed as u32) < (self.duration_frames as u32)
    }
```

**Step 2: Add unit tests**

Append to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn snap_writes_both_current_and_prev() {
        let mut fc = FacingClass::new(0, 5);
        fc.snap(12800, 10);
        assert_eq!(fc.current, 12800);
        assert_eq!(fc.prev, 12800);
        assert_eq!(fc.duration_frames, 0);
        assert_eq!(fc.current(10), 12800); // animated = destination immediately
    }

    #[test]
    fn snap_no_op_when_already_at_target() {
        let mut fc = FacingClass::new(1000, 5);
        let changed = fc.snap(1000, 0);
        assert!(!changed);
    }

    #[test]
    fn is_rotating_false_when_rot_zero() {
        let fc = FacingClass::new(0, 0);
        assert!(!fc.is_rotating(0));
        assert!(!fc.is_rotating(100));
    }

    #[test]
    fn is_rotating_false_when_no_start_frame() {
        let fc = FacingClass::new(0, 5);
        assert!(!fc.is_rotating(0));
    }

    #[test]
    fn is_rotating_true_during_rotation_false_after() {
        let mut fc = FacingClass::new(0, 5);
        fc.set(12800, 0); // duration = 10
        assert!(fc.is_rotating(0));
        assert!(fc.is_rotating(5));
        assert!(fc.is_rotating(9));
        assert!(!fc.is_rotating(10)); // exactly at end
        assert!(!fc.is_rotating(100));
    }

    #[test]
    fn turret_spins_formula_smoke_test() {
        // Forward-test the deferred Floating Disk permaspin formula:
        // each frame, set target = ((current(f) >> 7 + 1) >> 1 + 8) << 8.
        // ROT byte = 100 (Disk's ROT), per-frame step = 25600 in 16-bit units.
        // Per-frame target advance = 8 << 8 = 2048 < 25600, so step_size = 0
        // → snaps every frame to the new target. After 32 frames, full revolution.
        let mut fc = FacingClass::new(0, 100);
        for f in 0..32u32 {
            let animated = fc.current(f);
            let rounded_8bit: u16 = (((animated >> 7) + 1) >> 1) & 0xFF;
            let next_target: u16 = ((rounded_8bit + 8) & 0xFF) << 8;
            fc.set(next_target, f);
        }
        // After 32 frames of advancing by 8 8-bit units, we should be back at
        // (8 * 32) % 256 = 256 % 256 = 0 (in 8-bit), so 0 << 8 = 0 in 16-bit.
        assert_eq!(fc.current(32), 0);
    }
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game facing_class::tests`
Expected: all FacingClass tests PASS (3 from Task 3 + 7 from Task 4 + 5 from Task 5 + 6 from Task 6 = 21 tests).

**Step 4: Commit**
```
movement/facing_class: add snap() and is_rotating()

Completes the FacingClass API. snap() mirrors UpdateFacing for
no-smoothing setters (spawn/takeoff/deploy). is_rotating() is the
on-demand equivalent of gamemd's field_0x4A0 latch. Includes a
smoke test for the deferred TurretSpins permaspin formula —
verifies FacingClass with rot=100 correctly handles per-frame
sets with sub-step deltas (snap-every-frame behavior).
```

---

### Task 7: Replace `entity.turret_facing: Option<u16>` with `entity.barrel_facing: Option<FacingClass>`

**Why:** Surface the type change everywhere it's stored. Migrate spawn sites, default constructors, and state hash. Once this lands, every reader/writer in the codebase will fail to compile, surfacing all migration sites at once.

**Files:**
- Modify: `src/sim/game_entity.rs:97, 269, 412`
- Modify: `src/sim/world/world_spawn.rs:140, 336`
- Modify: `src/sim/world/world_hash.rs:314+` (hash_entities — ADD turret to hash)

**Pattern:** Field-rename + type-change refactor. Mirrors how `attack_target: Option<AttackTarget>` is hashed at `world_hash.rs:372-378`.

**Step 1: Update field declaration in GameEntity**

In `src/sim/game_entity.rs:95-97`, replace:

```rust
    /// Independent turret facing — only on entities with Turret=yes in rules.ini.
    /// 16-bit DirStruct (0–65535), full FacingClass precision.
    pub turret_facing: Option<u16>,
```

With:

```rust
    /// Independent turret/barrel facing — only on entities with Turret=yes in rules.ini.
    /// Timer-based 16-bit interpolator mirroring gamemd's BarrelFacing primitive.
    pub barrel_facing: Option<crate::sim::movement::FacingClass>,
```

**Step 2: Update default in `test_default` constructor**

In `src/sim/game_entity.rs:269`, replace:

```rust
            turret_facing: None,
```

With:

```rust
            barrel_facing: None,
```

**Step 3: Update test assertion**

In `src/sim/game_entity.rs:412`, replace:

```rust
        assert!(e.turret_facing.is_none());
```

With:

```rust
        assert!(e.barrel_facing.is_none());
```

**Step 4: Update spawn sites in world_spawn.rs**

In `src/sim/world/world_spawn.rs:140`, replace the existing assignment block (read +/- 5 lines for context first to confirm the structure, then replace):

```rust
                ge.turret_facing = Some(crate::sim::movement::turret::body_facing_to_turret(
```

The full replacement at line 140 — and the matching pattern at line 336 — depends on the surrounding code structure. Open the file, locate each site, and convert from:

```rust
ge.turret_facing = Some(crate::sim::movement::turret::body_facing_to_turret(facing));
```

To:

```rust
let initial = crate::sim::movement::turret::body_facing_to_turret(facing);
let rot_byte = obj.turret_rot.clamp(0, 0xFF) as u8;
ge.barrel_facing = Some(crate::sim::movement::FacingClass::new(initial, rot_byte));
```

If the local variable `obj` is not in scope at that site, fall back to looking up the rules object. The `world_spawn` module already has `rules: &RuleSet` and `interner: &StringInterner` — call `rules.object(interner.resolve(type_ref))` to get the `ObjectType`. If lookup fails, default `rot_byte = 5` (matches existing `turret.rs:163` fallback).

**Step 5: Hash barrel_facing in `hash_entities`**

In `src/sim/world/world_hash.rs:444` (right before the closing `}` of the per-entity loop body, around line 444 where `entity.ifv_weapon_index.hash(hasher);` is), add:

```rust
            // Barrel facing — Hash-derived, all primitive fields contribute.
            if let Some(ref barrel) = entity.barrel_facing {
                1u8.hash(hasher);
                barrel.hash(hasher);
            } else {
                0u8.hash(hasher);
            }
```

(`FacingClass` derives `Hash`, so this works automatically.)

**Step 6: Verify field-rename surfaces all consumer breakage**

Run: `cargo build 2>&1 | head -100`
Expected: compile errors at every site that reads or writes `entity.turret_facing` or `AttackerSnapshot::turret_facing`. These sites become Tasks 8–10. Do NOT fix them in this task.

If `cargo build` succeeds (no errors), it means another session has already migrated the sites — re-grep to confirm and adjust scope.

**Step 7: Commit (with build broken)**

This is intentional — committing the broken state shows the surface area cleanly:

```
game_entity: rename turret_facing → barrel_facing, type to FacingClass

Pre-migration surface change. Compile is INTENTIONALLY broken at all
consumer sites — Tasks 8-10 migrate them. Spawn sites + state hash
updated. Reverts cleanly if this round is abandoned.
```

(Use a normal commit; do not skip hooks.)

---

### Task 8: Migrate command-handler call sites in `combat/mod.rs` (remove instant snap)

**Why:** `combat/mod.rs:362-371` and `combat/mod.rs:435-444` currently set `attacker.turret_facing = Some(desired_u16)` directly when an attack command is issued. This snaps the turret instantly to the desired facing — bypassing the rotation pipeline. Migration: remove the direct write entirely. The attack_target alone is enough; `tick_turret_rotation` will drive the rotation per-tick.

**Files:**
- Modify: `src/sim/combat/mod.rs:362-371` (issue_attack_target_command body-vs-turret split)
- Modify: `src/sim/combat/mod.rs:435-444` (issue_attack_cell_command body-vs-turret split)

**Pattern:** Behavior-preserving for the body case (it stays `entity.facing = facing_from_delta(...)`), bug-fix for the turret case (remove the snap).

**Step 1: Read the two command-handler functions to confirm the pattern**

Read `src/sim/combat/mod.rs` lines 340-449. Confirm the structure: pre-mutation extract → mutate `attacker` → set facing → set attack_target. The change is: the turret-facing branch no longer writes anything; the body branch keeps its existing logic.

**Step 2: Replace the issue_attack_target_command turret branch**

In `src/sim/combat/mod.rs:362-371`, replace:

```rust
    // Update facing toward target (lepton-precise for turrets, cell-level for body).
    if has_turret {
        let desired_u16 = crate::sim::movement::turret::facing_toward_lepton(
            arx, ary, asx, asy, trx, try_, tsx, tsy,
        );
        attacker.turret_facing = Some(desired_u16);
    } else {
        let dx: i32 = trx as i32 - arx as i32;
        let dy: i32 = try_ as i32 - ary as i32;
        attacker.facing = crate::sim::movement::facing_from_delta(dx, dy);
    }
```

With:

```rust
    // Body-only: instantly face the target. For turreted units, the turret
    // rotates over multiple ticks — driven by tick_turret_rotation reading
    // attack_target — so we set NO facing here. This matches gamemd: command
    // handlers set the target; Facing_Update drives the rotation.
    if !has_turret {
        let dx: i32 = trx as i32 - arx as i32;
        let dy: i32 = try_ as i32 - ary as i32;
        attacker.facing = crate::sim::movement::facing_from_delta(dx, dy);
    }
```

**Step 3: Apply the same change to issue_attack_cell_command**

In `src/sim/combat/mod.rs:435-444`, replace the analogous block:

```rust
    if has_turret {
        let desired_u16 = crate::sim::movement::turret::facing_toward_lepton(
            arx, ary, asx, asy, trx, try_, tsx, tsy,
        );
        attacker.turret_facing = Some(desired_u16);
    } else {
        let dx: i32 = trx as i32 - arx as i32;
        let dy: i32 = try_ as i32 - ary as i32;
        attacker.facing = crate::sim::movement::facing_from_delta(dx, dy);
    }
```

With:

```rust
    if !has_turret {
        let dx: i32 = trx as i32 - arx as i32;
        let dy: i32 = try_ as i32 - ary as i32;
        attacker.facing = crate::sim::movement::facing_from_delta(dx, dy);
    }
```

**Step 4: Update the `is_some()` check sites**

In `src/sim/combat/mod.rs:347` and `src/sim/combat/mod.rs:407`, replace:

```rust
            a.turret_facing.is_some(),
```

With:

```rust
            a.barrel_facing.is_some(),
```

**Step 5: Verify**

Run: `cargo build 2>&1 | head -50`
Expected: compile errors ONLY in files not yet migrated (combat_targeting.rs, the rest of combat/mod.rs, app_instances/units.rs, app_instances/shp.rs, sim/movement/turret.rs).

**Step 6: Commit**
```
combat: remove instant turret-facing snap from command handlers

issue_attack_target_command and issue_attack_cell_command no longer
write attacker.turret_facing directly — the snap was a pre-existing
bug that bypassed rotation. Turreted units rotate over multiple ticks
via tick_turret_rotation; body-only units keep their instant facing.

Updates is_some() checks to barrel_facing.
```

---

### Task 9: Migrate `combat_targeting.rs` (AttackerSnapshot field + retaliation)

**Why:** `AttackerSnapshot` is the per-attacker snapshot extracted before the combat loop's mutable iteration. Its `turret_facing` field flows into the alignment check; the retaliation path also has the instant-snap bug.

**Files:**
- Modify: `src/sim/combat/combat_targeting.rs:58, 99, 313-329`

**Pattern:** Same migration shape as Task 8 — type-rename + snap-removal.

**Step 1: Update AttackerSnapshot field**

In `src/sim/combat/combat_targeting.rs:57-58`, replace:

```rust
    pub turret_facing: Option<u16>,
```

With:

```rust
    pub barrel_facing: Option<crate::sim::movement::FacingClass>,
```

**Step 2: Update snapshot extraction**

In `src/sim/combat/combat_targeting.rs:99`, replace:

```rust
        turret_facing: entity.turret_facing,
```

With:

```rust
        barrel_facing: entity.barrel_facing,
```

(`FacingClass` is `Copy`, so this clones cheaply.)

**Step 3: Remove retaliation snap**

In `src/sim/combat/combat_targeting.rs:312-329`, the existing block:

```rust
            if let Some(entity) = entities.get_mut(entity_id) {
                if entity.turret_facing.is_some() {
                    let desired_u16 = crate::sim::movement::turret::facing_toward_lepton(
                        entity.position.rx,
                        entity.position.ry,
                        entity.position.sub_x,
                        entity.position.sub_y,
                        attacker_pos.0,
                        attacker_pos.1,
                        attacker_pos.2,
                        attacker_pos.3,
                    );
                    entity.turret_facing = Some(desired_u16);
                } else {
                    let dx: i32 = attacker_pos.0 as i32 - entity.position.rx as i32;
                    let dy: i32 = attacker_pos.1 as i32 - entity.position.ry as i32;
                    entity.facing = crate::sim::movement::facing_from_delta(dx, dy);
                }
                entity.movement_target = None;
                entity.attack_target = Some(crate::sim::combat::AttackTarget::new(attacker_sid));
            }
```

Becomes (turret branch removes the snap, body branch preserves it):

```rust
            if let Some(entity) = entities.get_mut(entity_id) {
                if entity.barrel_facing.is_none() {
                    // Body-only retaliator — instantly face the attacker. Turreted
                    // retaliators get their turret rotation driven by
                    // tick_turret_rotation in subsequent ticks (matches gamemd).
                    let dx: i32 = attacker_pos.0 as i32 - entity.position.rx as i32;
                    let dy: i32 = attacker_pos.1 as i32 - entity.position.ry as i32;
                    entity.facing = crate::sim::movement::facing_from_delta(dx, dy);
                }
                entity.movement_target = None;
                entity.attack_target = Some(crate::sim::combat::AttackTarget::new(attacker_sid));
            }
```

**Step 4: Verify**

Run: `cargo build 2>&1 | head -50`
Expected: combat_targeting compiles; remaining errors are in render layer + main combat loop + turret.rs.

**Step 5: Commit**
```
combat_targeting: AttackerSnapshot uses barrel_facing; retaliation no-snap

Type rename + retaliation no longer instant-snaps turret. Body-only
retaliators preserve instant-face behavior (no rotation pipeline).
```

---

### Task 10: Migrate render layer (units.rs + shp.rs) to read FacingClass via binary_frame

**Why:** Render reads animated turret facing via `entity.turret_facing.unwrap_or(0u16)`. After Task 7's field rename, render now reads `entity.barrel_facing.as_ref().map(|f| f.current(sim.binary_frame)).unwrap_or(0u16)`. Need to thread `binary_frame` to the call sites.

**Files:**
- Modify: `src/app_instances/units.rs:154-164`
- Modify: `src/app_instances/shp.rs:315`

**Pattern:** Read animated value at the call site; the helper functions (`canonical_turret_facing`, etc.) keep their `u16` signature.

**Step 1: Identify the Simulation/sim reference scope at each call site**

Read `src/app_instances/units.rs:140-170` and `src/app_instances/shp.rs:300-320` to confirm a `Simulation`-typed reference is in scope. (Look for `sim:` parameter, `&self.sim`, or similar.) If a `Simulation` reference is NOT in scope, the change becomes "thread `binary_frame: u32` through the function signature."

**Step 2: Update units.rs read site**

In `src/app_instances/units.rs:154-164`, replace:

```rust
        if let Some(turret_facing) = entity.turret_facing {
```

With (assuming `sim` is in scope as `&Simulation`):

```rust
        if let Some(turret_facing) = entity
            .barrel_facing
            .as_ref()
            .map(|f| f.current(sim.binary_frame))
        {
```

If `sim` is not directly available, look at the function signature; the `Simulation` reference is upstream — thread it through or use the existing context object.

**Step 3: Update shp.rs read site**

In `src/app_instances/shp.rs:315`, replace:

```rust
                            entity.turret_facing.unwrap_or(0u16),
```

With:

```rust
                            entity
                                .barrel_facing
                                .as_ref()
                                .map(|f| f.current(sim.binary_frame))
                                .unwrap_or(0u16),
```

**Step 4: Verify**

Run: `cargo build 2>&1 | head -50`
Expected: app_instances compiles; remaining errors limited to combat/mod.rs (alignment check) and turret.rs (rewrite pending).

**Step 5: Commit**
```
app_instances: read barrel_facing.current(binary_frame) for render

Render now reads the animated FacingClass value at the current binary
frame, replacing direct u16 read. canonical_turret_facing (render
helper) unchanged — still operates on u16.
```

---

### Task 11: Rewrite `tick_turret_rotation` around FacingClass

**Why:** The turret rotation phase is rewritten end-to-end: signature changes to take `binary_frame`, body uses `FacingClass::set` for both attack-target and idle-return-to-body paths. Replaces the per-tick step-clamp with the timer-based interpolator.

**Files:**
- Modify: `src/sim/movement/turret.rs` (full rewrite of `tick_turret_rotation`; delete `is_turret_aligned_u16`, `rot_to_facing_delta_u16`, `shortest_rotation_u16`)
- Modify: `src/sim/world/mod.rs:1150` (caller signature update)

**Pattern:** Two-phase Vec collection (Phase 1 read, Phase 2 write) preserved from existing structure to avoid borrow conflicts.

**Step 1: Replace `tick_turret_rotation` with the new implementation**

In `src/sim/movement/turret.rs`, replace the existing `tick_turret_rotation` function (lines ~123-236) with:

```rust
/// Per-binary-frame turret rotation — drives barrel_facing toward each
/// entity's desired facing.
///
/// - If entity has AttackTarget: rotate barrel toward target (lepton-precise).
/// - Otherwise: rotate barrel back to body facing (idle return — research
///   doc §5.1, ledger #20).
///
/// Calls FacingClass::set, which is a no-op when the desired facing equals
/// the current destination — so this function is idempotent.
pub fn tick_turret_rotation(
    entities: &mut EntityStore,
    rules: &RuleSet,
    binary_frame: u32,
    interner: &crate::sim::intern::StringInterner,
) {
    struct TurretUpdate {
        id: u64,
        target_facing: u16,
    }
    let mut updates: Vec<TurretUpdate> = Vec::new();

    // Phase 1: read each turreted entity's desired facing.
    let keys: Vec<u64> = entities.keys_sorted();
    for &id in &keys {
        let entity = match entities.get(id) {
            Some(e) => e,
            None => continue,
        };
        if entity.barrel_facing.is_none() {
            continue;
        }

        let desired_facing: u16 = if let Some(ref attack) = entity.attack_target {
            // Look up target position. Entity targets via stable ID,
            // Cell targets via cell-center leptons (force-fire on ground).
            let target_pos = match attack.target {
                crate::sim::combat::TargetKind::Entity(target_id) => {
                    entities.get(target_id).map(|t| {
                        (
                            t.position.rx,
                            t.position.ry,
                            t.position.sub_x,
                            t.position.sub_y,
                        )
                    })
                }
                crate::sim::combat::TargetKind::Cell(rx, ry) => Some((
                    rx,
                    ry,
                    crate::util::fixed_math::SimFixed::from_num(128),
                    crate::util::fixed_math::SimFixed::from_num(128),
                )),
            };
            match target_pos {
                Some((trx, try_, tsx, tsy)) => facing_toward_lepton(
                    entity.position.rx,
                    entity.position.ry,
                    entity.position.sub_x,
                    entity.position.sub_y,
                    trx,
                    try_,
                    tsx,
                    tsy,
                ),
                // Target gone — idle-return to body facing.
                None => body_facing_to_turret(entity.facing),
            }
        } else {
            // No target — return to body facing (research doc §5.1).
            body_facing_to_turret(entity.facing)
        };

        updates.push(TurretUpdate {
            id,
            target_facing: desired_facing,
        });
    }

    // Phase 2: apply rotation via FacingClass::set. Idempotent — no-op when
    // target already equals current destination.
    for update in &updates {
        let rot_byte: u8 = rules
            .object(
                interner.resolve(
                    entities
                        .get(update.id)
                        .map(|e| e.type_ref)
                        .unwrap_or_default(),
                ),
            )
            .map(|obj| obj.turret_rot.clamp(0, 0xFF) as u8)
            .unwrap_or(5);
        if let Some(entity) = entities.get_mut(update.id) {
            if let Some(ref mut barrel) = entity.barrel_facing {
                // Refresh ROT in case rules changed (cheap; idempotent).
                barrel.set_rot(rot_byte);
                barrel.set(update.target_facing, binary_frame);
            }
        }
    }
}
```

**Step 2: Delete the obsolete helper functions**

In `src/sim/movement/turret.rs`, delete:
- `pub fn shortest_rotation_u16` (lines ~58-69)
- `pub fn rot_to_facing_delta_u16` (lines ~71-87)
- `pub fn is_turret_aligned_u16` (lines ~89-93)
- `pub const TURRET_ALIGN_THRESHOLD_U16` (line ~23)

The 8-bit cousins (`shortest_rotation`, `rot_to_facing_delta`) STAY for now — body smoothing will retire them in a later round.

Delete the corresponding tests in `#[cfg(test)] mod tests`:
- `test_shortest_rotation_u16_*` (3 tests)
- `test_rot_to_facing_delta_u16_*` (3 tests)
- `test_is_turret_aligned_u16` (1 test)

**Step 3: Update caller signature in world/mod.rs**

In `src/sim/world/mod.rs:1150`, replace:

```rust
            turret::tick_turret_rotation(&mut self.entities, rules, tick_ms, &self.interner);
```

With:

```rust
            turret::tick_turret_rotation(
                &mut self.entities,
                rules,
                self.binary_frame,
                &self.interner,
            );
```

**Step 4: Verify**

Run: `cargo build 2>&1 | head -30`
Expected: turret.rs and world/mod.rs compile. Remaining errors are in combat/mod.rs alignment check (Task 12).

Run: `cargo test -p ra2-rust-game movement::turret`
Expected: remaining `facing_toward_lepton` tests PASS; deleted-test references should be cleaned up by the deletion in Step 2.

**Step 5: Commit**
```
movement/turret: rewrite tick_turret_rotation around FacingClass

Per-binary-frame rotation now drives barrel_facing via FacingClass::set,
with idle units returning to body facing (research doc §5.1).
Idempotent — no-op when target unchanged. Removes is_turret_aligned_u16
+ rot_to_facing_delta_u16 + shortest_rotation_u16 (subsumed by FacingClass).
```

---

### Task 12: Replace combat alignment check with FireDecision-driven flow

**Why:** Combat snapshot loop currently uses `is_turret_aligned_u16` (deleted in Task 11) plus `continue`-based control flow. Replace with `barrel.current(frame) == desired && !barrel.is_rotating(frame)` and FireDecision-tagged outcomes for the gattling-spinup gate.

**Files:**
- Modify: `src/sim/combat/mod.rs:1010-1060` (snapshot extraction, ~25 lines)
- Modify: `src/sim/combat/mod.rs:1190-1260` (snapshot construction, ~70 lines)
- Modify: `src/sim/combat/mod.rs:1480-1500` (alignment check + decision dispatch)

**Pattern:** Adopt FireDecision per-attacker; alignment check becomes a method-call, not a tolerance-band test.

**Step 1: Migrate the snapshot extraction sites**

In `src/sim/combat/mod.rs`, lines around 1012, 1033, 1202, 1220, 1253: every place that reads `entity.turret_facing` or assigns into `AttackerSnapshot.turret_facing` needs to become `barrel_facing`. The fix is mechanical:

```rust
// Old:
entity.turret_facing,
// becomes:
entity.barrel_facing,
```

```rust
// Old:
turret_facing,
// becomes (in snapshot field-init shorthand):
barrel_facing,
```

Read each call site and apply the rename. Don't change semantics yet — just the field name.

**Step 2: Replace the alignment check at combat/mod.rs:1483-1497**

The existing block:

```rust
        if let Some(turret_facing) = snap.turret_facing {
            let desired: u16 = crate::sim::movement::turret::facing_toward_lepton(
                snap.pos_rx,
                snap.pos_ry,
                snap.sub_x,
                snap.sub_y,
                target_rx,
                target_ry,
                target_sub_x,
                target_sub_y,
            );
            if !crate::sim::movement::turret::is_turret_aligned_u16(turret_facing, desired) {
                continue;
            }
        }
```

Becomes:

```rust
        if let Some(ref barrel) = snap.barrel_facing {
            let desired: u16 = crate::sim::movement::turret::facing_toward_lepton(
                snap.pos_rx,
                snap.pos_ry,
                snap.sub_x,
                snap.sub_y,
                target_rx,
                target_ry,
                target_sub_x,
                target_sub_y,
            );
            // Aligned iff destination matches AND no rotation in progress.
            // Both checks needed: destination may match while interpolation
            // is still mid-arc (animated value not yet at destination).
            let aligned = barrel.current(world_binary_frame) == desired
                && !barrel.is_rotating(world_binary_frame);
            if !aligned {
                // FireDecision::Facing — drives gattling spin-up via
                // drives_gattling_spinup() == true.
                continue;
            }
        }
```

The reference `world_binary_frame` is the current sim's `binary_frame`. Plumb it into the function: the surrounding function `tick_combat_with_fog` (around line 462) already takes `tick_ms` — add `binary_frame: u32` to its signature, and pass `self.binary_frame` from the caller in `world/mod.rs:1157-1170`.

**Step 3: Update tick_combat_with_fog signature**

In `src/sim/combat/mod.rs:462`, find the `tick_combat_with_fog` (or `tick_combat`) signature and add `binary_frame: u32` as a parameter (next to `tick_ms`). Update the inner code to use it where the alignment check now needs it.

In `src/sim/world/mod.rs:1157`, update the call to pass `self.binary_frame`:

```rust
            let combat_result = combat::tick_combat_with_fog(
                ...
                self.tick,
                tick_ms,
                self.binary_frame, // NEW
            );
```

**Step 4: Verify**

Run: `cargo build 2>&1 | head -30`
Expected: clean build (or remaining errors are all in tests, addressed in Task 14).

Run: `cargo test -p ra2-rust-game --lib 2>&1 | tail -50`
Expected: tests run; some may fail due to tick-order changes — those are addressed in Task 14.

**Step 5: Commit**
```
combat: alignment check uses FacingClass.current + is_rotating

Replaces is_turret_aligned_u16 flat-tolerance with destination-match-
and-not-rotating test. Threads binary_frame into tick_combat_with_fog.
Aligned iff barrel.current(frame) == desired AND !is_rotating(frame) —
fixes the slow-ROT misalignment bug (current 2048-tolerance is wrong).
```

---

### Task 13: Flip Phase 5 tick order (combat before turret rotation)

**Why:** gamemd runs Fire_At_Target → Facing_Update; our current order is the opposite. Flipping makes combat read last frame's facing, matching binary's 1-tick acquisition latency.

**Files:**
- Modify: `src/sim/world/mod.rs:1146-1170` (Phase 5 ordering)

**Step 1: Read the current Phase 5 block**

Read `src/sim/world/mod.rs:1146-1170`. Confirm the current order is:
1. `tick_turret_rotation`
2. `tick_capture_orders`
3. `tick_order_intents_pre_combat`
4. `tick_attack_pursuit`
5. `tick_combat_with_fog`

**Step 2: Move `tick_turret_rotation` to AFTER `tick_combat_with_fog`**

Move the line:

```rust
            turret::tick_turret_rotation(
                &mut self.entities,
                rules,
                self.binary_frame,
                &self.interner,
            );
```

To AFTER the `let combat_result = combat::tick_combat_with_fog(...)` block (around line 1170).

Update the existing comment at `world/mod.rs:1148` from:

```rust
            // --- Phase 5: Turrets + Combat ---
            // DEPENDS ON: vision/fog (targeting uses fog state), power (cloaking),
            //   turret rotation MUST run before combat so turrets are aligned when firing.
```

To:

```rust
            // --- Phase 5: Combat + Turret rotation ---
            // DEPENDS ON: vision/fog (targeting uses fog state), power (cloaking).
            // Combat reads barrel.current(binary_frame) at the START of the tick
            // (matching gamemd's Fire_At_Target which uses last-frame facing).
            // tick_turret_rotation runs AFTER combat to drive rotation toward the
            // target for the NEXT frame's fire decision (matches Facing_Update order).
```

**Step 3: Verify**

Run: `cargo build`
Expected: clean build.

Run: `cargo test -p ra2-rust-game --lib 2>&1 | tail -80`
Expected: many combat-related tests may now fail with messages about timing (e.g., "expected fire at tick 5, got fire at tick 6"). These are the expected fallout — addressed in Task 14.

**Step 4: Commit**
```
world: Phase 5 — flip tick order to combat-before-turret-rotation

Mirrors gamemd: Fire_At_Target → Facing_Update. Combat reads
barrel.current(binary_frame) at tick start (= previous-frame facing).
tick_turret_rotation runs after to advance toward the target for the
NEXT frame's fire decision. Adds the 1-tick acquisition latency the
binary has — observable in fire-cadence parity.

Test fallout addressed in Task 14.
```

---

### Task 14: Add integration tests for new turret + fire-decision behavior

**Why:** Surface the parity-critical behaviors as explicit tests that catch regressions: 1-tick acquisition latency, mid-rotation retarget smoothness, slow vs fast ROT alignment timing.

**Files:**
- Create: `src/sim/combat/combat_turret_facing_tests.rs`
- Modify: `src/sim/combat/mod.rs` (add `#[cfg(test)] mod combat_turret_facing_tests;`)

**Pattern:** Follow existing test pattern from `src/sim/combat/combat_pursuit_tests.rs` and `combat_force_fire_cell_tests.rs` — use `Simulation::new()`, seed entities via helpers, call `advance_tick`, assert state transitions.

**Step 1: Create the test file**

`src/sim/combat/combat_turret_facing_tests.rs`:

```rust
//! Integration tests for turret rotation + fire decision parity.
//!
//! Verifies the FacingClass-driven combat behavior end-to-end through
//! `Simulation::advance_tick`, covering 1-tick acquisition latency,
//! mid-rotation retarget, slow vs fast ROT alignment timing, and the
//! flipped Phase 5 tick order.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::sim::combat::AttackTarget;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::FacingClass;
use crate::sim::movement::turret::{body_facing_to_turret, facing_toward_lepton};
use crate::sim::world::Simulation;

fn empty_height_map() -> BTreeMap<(u16, u16), u8> {
    BTreeMap::new()
}

/// Spawn a turreted attacker at (rx, ry) facing north (0) with the given ROT byte.
fn spawn_turreted(sim: &mut Simulation, stable_id: u64, rx: u16, ry: u16, rot_byte: u8) {
    let mut entity = GameEntity::test_default(stable_id, "MTNK", "Americans", rx, ry);
    entity.barrel_facing = Some(FacingClass::new(body_facing_to_turret(0), rot_byte));
    sim.entities.insert(entity);
}

/// Spawn a passive target at (rx, ry).
fn spawn_target(sim: &mut Simulation, stable_id: u64, rx: u16, ry: u16) {
    let entity = GameEntity::test_default(stable_id, "GAPILE", "Soviet", rx, ry);
    sim.entities.insert(entity);
}

#[test]
fn one_tick_acquisition_latency_first_tick_no_fire() {
    // After issuing an attack, the binary takes 1+ frames to rotate the turret
    // before firing (combat reads last-frame's facing). Even with ROT large
    // enough to fully rotate in 1 frame, the FIRST tick after target-set
    // produces no fire because combat ran BEFORE turret_rotation.
    let mut sim = Simulation::new();
    spawn_turreted(&mut sim, 1, 5, 5, 100); // ROT=100 → rot_per_frame=25600
    spawn_target(&mut sim, 2, 8, 5);

    // Attach attack_target so combat will try to fire on the next tick.
    if let Some(e) = sim.entities.get_mut(1) {
        e.attack_target = Some(AttackTarget::new(2));
    }

    let initial_target_health = sim.entities.get(2).unwrap().health.current;
    sim.advance_tick(&[], None, &empty_height_map(), None, None, 67);

    // Target should still be alive — combat ran before turret rotation, so
    // turret was at facing 0 (body), not aligned with target.
    let target_health_after_one_tick = sim.entities.get(2).unwrap().health.current;
    assert_eq!(
        target_health_after_one_tick, initial_target_health,
        "First tick after acquisition should not fire (1-tick latency)"
    );
}

#[test]
fn slow_rot_takes_more_frames_to_align_than_fast_rot() {
    // ROT=1 vs ROT=10: same acquisition geometry, the slow turret takes
    // proportionally more binary frames to align. Fixes the current
    // is_turret_aligned_u16 flat-tolerance bug.
    let mut sim_slow = Simulation::new();
    let mut sim_fast = Simulation::new();
    spawn_turreted(&mut sim_slow, 1, 5, 5, 1); // ROT=1 → rot_per_frame=256
    spawn_turreted(&mut sim_fast, 1, 5, 5, 10); // ROT=10 → rot_per_frame=2560
    spawn_target(&mut sim_slow, 2, 5, 8); // 3 cells south
    spawn_target(&mut sim_fast, 2, 5, 8);

    // Attach attack_target on both.
    sim_slow.entities.get_mut(1).unwrap().attack_target = Some(AttackTarget::new(2));
    sim_fast.entities.get_mut(1).unwrap().attack_target = Some(AttackTarget::new(2));

    // Compute the expected duration: from facing 0 (north, after body_facing_to_turret(0))
    // to facing south (~32768). Diff = 32768. ROT=1: duration = 32768/256 = 128 frames.
    // ROT=10: duration = 32768/2560 = 12 frames.
    // Run 13 binary frames worth of ticks. Fast turret should be done; slow not.

    // Each 67ms tick advances binary_frame by ~1.
    for _ in 0..13 {
        sim_slow.advance_tick(&[], None, &empty_height_map(), None, None, 67);
        sim_fast.advance_tick(&[], None, &empty_height_map(), None, None, 67);
    }

    let slow_rotating = sim_slow
        .entities
        .get(1)
        .unwrap()
        .barrel_facing
        .as_ref()
        .map(|f| f.is_rotating(sim_slow.binary_frame))
        .unwrap_or(false);
    let fast_rotating = sim_fast
        .entities
        .get(1)
        .unwrap()
        .barrel_facing
        .as_ref()
        .map(|f| f.is_rotating(sim_fast.binary_frame))
        .unwrap_or(false);

    assert!(slow_rotating, "ROT=1 turret should still be rotating after 13 frames");
    assert!(
        !fast_rotating,
        "ROT=10 turret should be done rotating after 13 frames"
    );
}

#[test]
fn idle_turret_returns_to_body_facing() {
    // No attack_target, body facing east (64) — turret should rotate to match.
    let mut sim = Simulation::new();
    let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
    entity.facing = 64; // body east
    entity.barrel_facing = Some(FacingClass::new(body_facing_to_turret(0), 100));
    // ROT=100 → rot_per_frame=25600. Diff from 0 (north turret) to body_facing_to_turret(64) =
    // 64*256 = 16384. Duration = 16384/25600 = 0 → snaps in 1 frame.
    sim.entities.insert(entity);

    // Run 2 ticks to ensure turret_rotation has had a chance to act.
    sim.advance_tick(&[], None, &empty_height_map(), None, None, 67);
    sim.advance_tick(&[], None, &empty_height_map(), None, None, 67);

    let barrel = sim.entities.get(1).unwrap().barrel_facing.as_ref().unwrap();
    assert_eq!(
        barrel.destination(),
        body_facing_to_turret(64),
        "Idle turret should target body facing"
    );
}

#[test]
fn mid_rotation_retarget_snapshots_into_prev() {
    // Start a rotation, advance partway, set a new target. The new prev
    // should equal the animated value at the moment of the new set (not the
    // original prev) — visible smoothness of mid-rotation retarget.
    let mut fc = FacingClass::new(0, 5);
    fc.set(12800, 0); // rotation 0 → 12800 over 10 frames.
    let animated_at_5 = fc.current(5);
    fc.set(25600, 5); // retarget mid-rotation.

    // After re-set, prev should equal the animated value at frame 5, NOT 0.
    assert_eq!(
        fc.current(5),
        animated_at_5,
        "Animated value immediately after re-set should equal pre-set animated value (no jump)"
    );
}
```

**Step 2: Register the test module**

In `src/sim/combat/mod.rs`, add to the existing `#[cfg(test)]` module declarations block:

```rust
#[cfg(test)]
#[path = "combat_turret_facing_tests.rs"]
mod combat_turret_facing_tests;
```

**Step 3: Verify**

Run: `cargo test -p ra2-rust-game combat_turret_facing_tests`
Expected: all 4 tests PASS.

If `one_tick_acquisition_latency_first_tick_no_fire` FAILS, it likely means the tick order flip didn't take effect — re-verify Task 13.

**Step 4: Commit**
```
combat: integration tests for FacingClass-driven turret + fire decision

Tests cover: 1-tick acquisition latency (validates Phase 5 flip),
slow vs fast ROT alignment timing (validates is_turret_aligned bug fix),
idle return-to-body, and mid-rotation retarget smoothness.
```

---

### Task 15: Run full test suite, audit + fix any pre-existing test fallout

**Why:** The Phase 5 tick-order flip and instant-snap removal will cause some existing tests to fail in ways that are EXPECTED — they need to be updated to reflect the new (parity-correct) timing. Surface every failure, audit each, fix.

**Files:**
- Modify: any test file in `src/sim/combat/` or related that asserts pre-flip timing.

**Step 1: Run full test suite**

Run: `cargo test -p ra2-rust-game --lib 2>&1 | tail -150`

**Step 2: Categorize failures**

For each failing test, determine which category it falls into:

(a) **Tick-order regression** — test expected fire on tick N, now fires on tick N+1. Fix: bump the assertion's expected tick by 1 (or use a loop until fired).

(b) **Instant-snap regression** — test assumed turret was instantly aimed after issuing an attack command. Fix: advance ticks until alignment completes before asserting fire.

(c) **Genuine new bug** — test fails for reasons unrelated to (a) or (b). Stop and diagnose; do NOT mass-edit tests to make them pass.

(d) **Test pre-supposes deleted helper** (e.g., `is_turret_aligned_u16`) — Fix: replace with `barrel.current(frame) == desired && !barrel.is_rotating(frame)`.

**Step 3: Apply fixes per category**

Walk through each failure. For categories (a) and (b), the fix is mechanical — update the expected timing. For (c), STOP and report back; don't paper over.

**Step 4: Re-run full suite**

Run: `cargo test -p ra2-rust-game 2>&1 | tail -30`
Expected: all tests PASS.

**Step 5: Commit**
```
combat: update timing assertions for Phase 5 flip + no-snap migration

Tick-order regression: tests that asserted fire on tick N now expect
fire on tick N+1 (matches gamemd's 1-tick acquisition latency).
Instant-snap regression: tests that assumed instant turret aim now
advance ticks until alignment completes.

No genuine new failures uncovered.
```

(If you find category-(c) failures, commit only the (a) + (b) fixes here, then report category-(c) failures back as a separate issue.)

---

### Task 16 (Deferred follow-up — NOT required to land this round)

**Why:** Atan2 axis convention discrepancy between our `facing_from_delta_int_u16` (uses `atan2(dx, -dy)`) and the binary's `compute_facing_to_target` (uses `atan2(dy, -dx)` per research doc §6) is unverified. The FacingClass refactor neither improves nor regresses whatever the current convention produces — it just stores u16 values.

**Owner:** Whoever next has gamemd in a debugger, or whoever does the next /re-investigate of facing math.

**Steps:**
1. Set up gamemd in a debugger. Place a unit at known cell (e.g., (50, 50)) with a known target (e.g., (53, 50) = 3 cells east).
2. Read `BarrelFacing.Current` u16 value at known points: 0° (north), 45°, 90° (east), etc.
3. Compute our `facing_toward_lepton` for the same geometry. Compare.
4. If values differ: trace through the binary's full atan2-to-DirStruct conversion (the radians-to-u16 multiplier is missing from research doc §6); update `facing_from_delta_int_u16` to match. Add a `#[ignore]`-attribute test seeded with the binary ground-truth values.
5. If values match: add the test as a regression guard.

This task is documented here so future work has a concrete starting point. **Not required for the current round.**

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-10-unitclass-turret-tracking-facing-class-design.md](docs/plans/2026-05-10-unitclass-turret-tracking-facing-class-design.md)
- **Ghidra reports:**
  - `ra2-rust-game-docs/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` (primary, HIGH confidence)
  - `ra2-rust-game-docs/TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` §726-728 (FacingClass offsets)
  - `ra2-rust-game-docs/OPPORTUNITY_FIRE_GHIDRA_REPORT.md` §4 (mission 0x10)
  - `ra2-rust-game-docs/GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md` (Type+0xCD5 IsGattling)
  - `ra2-rust-game-docs/UNITCLASS_GHIDRA_REPORT.md` §3 (UnitClass::AI tick order)
- **gamemd.exe addresses (for future Ghidra cross-reference, NOT in Rust comments):**
  - `0x004C9220` FacingClass::Set (Ghidra label `RateTimer__Set`)
  - `0x004C9300` FacingClass::UpdateFacing (snap variant)
  - `0x004C93D0` FacingClass::Current (interpolation reader)
  - `0x004C9480` FacingClass::IsRotating
  - `0x004C9680` FacingClass::SetROT
  - `0x00736990` UnitClass::Facing_Update
  - `0x00736DF0` UnitClass::Fire_At_Target
  - `0x006FC0B0` TechnoClass::GetFireError
  - `0x005F3DB0` compute_facing_to_target
  - `0x007353C0` UnitClass::Constructor (verifies field offsets)
- **INI keys driving behavior:**
  - `[UnitType] ROT=` parsed at `src/rules/object_type.rs:763` → `obj.turret_rot: i32`
  - Harvester ROT override (forced=10) at parse time, verified against `UnitTypeClass::ReadINI 0x747620`
- **Related code:**
  - Closest existing pattern: `src/sim/movement/teleport_movement.rs::TeleportState`, `src/sim/superweapon/invulnerability.rs::InvulnerabilityState`
  - Existing time-conversion precedent: `rof_to_cooldown_ticks` at `src/sim/combat/mod.rs:1843`
  - Existing entity-state-hash pattern: `src/sim/world/world_hash.rs:314+` (hash_entities)
  - Spawn sites: `src/sim/world/world_spawn.rs:140, 336`
- **Recent commits affecting touched files (none invalidate the design):**
  - `57a20d8` combat: add tick_attack_pursuit unit tests
  - `3c7e38b` combat: range failure preserves attack_target (gamemd parity)
  - `343eae2` world: wire tick_attack_pursuit into advance_tick before combat
