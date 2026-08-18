# Voxel Body Matrix Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make every voxel-bodied unit's body matrix vary per tick — driven by
spring-damped rocking angles, smoothly-interpolated slope transitions, and a lighting
LUT that follows the full body orientation — to match gamemd.exe observable output.

**Architecture:** Sim-side spring-damper in fixed-point on a new `RockingState`
component (Option on `GameEntity`); render-side branches between the existing atlas
path (neutral units, fast) and a new real-time path through `vxl_compute.rs`
(actively-tilting units, parity-correct).

**Design Doc:** [docs/plans/2026-05-11-voxel-body-matrix-design.md](2026-05-11-voxel-body-matrix-design.md)

---

## Grounding Summary

- **RE docs cited:** [BODY_ROCKING_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BODY_ROCKING_GHIDRA_REPORT.md) (HIGH confidence, self-authored 2026-05-11); [VXL_DRAW_MATRIX_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/VXL_DRAW_MATRIX_GHIDRA_REPORT.md); [VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md); [VOXEL_SLOPE_TILT_SYSTEM.md](../../../ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md). All HIGH-confidence.
- **Ghidra verification:** RockingUpdate (0x0070B570), ApplyRocker (0x0070B280), WarheadTypeClass::Detonate (0x004690B0), Apply_area_damage (0x00489280), FootClass::ReceiveEMP (0x004DECF0), AI_Update call site (0x006FA236) — all decompiled this session, all constants read from binary memory.
- **Repo pattern mirrored:** [src/sim/animation.rs:375](../../src/sim/animation.rs#L375) `tick_animations` — direct-mutation per-tick system over `entities.keys_sorted()`. Optional component pattern from [src/sim/game_entity.rs](../../src/sim/game_entity.rs) (e.g. `animation: Option<Animation>`, `harvest_overlay: Option<HarvestOverlay>`). State hash field-by-field at [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs).
- **INI keys driving behavior:** `[AudioVisual] DirectRockingCoefficient=` (default 1.5), `[AudioVisual] FallBackCoefficient=` (default 0.1) — both confirmed at [rulesmd.ini:620-621](../../ini/rulesmd.ini). Per-warhead `Rocker=`, `DirectRocker=` (defaults `no`). Per-bullet `RockerScale=` (default 1.0 Q8.8). Coefficients live in `[AudioVisual]` section (parsed at [ruleset.rs:729, 749](../../src/rules/ruleset.rs#L729)), NOT `[General]`.
- **Fixed-point:** `SimFixed = I16F16` is the project-wide alias ([src/util/fixed_math.rs](../../src/util/fixed_math.rs)). All sim angles use this. Constants hoisted as module-level `const X: I16F16 = I16F16::lit("…")`.
- **Still unknown after grounding (deferred):** The warhead-detonation call site for impulses (depends on combat resolution state — stubbed). Documented in design doc §"Open Questions". (Per-class `vtable[0x298]` was resolved post-plan-review: gate is data-driven via `TypeClass+0xB0`, NOT virtual override — all 6 subclasses share one implementation. Our Rust spawn-time `Option<RockingState>` decision approximates this. See verify-doc audit 2026-05-11.)

## Key Technical Decisions

- **Spring-damper math in fixed-point.** Rust-side port of gamemd's float spring-damper using `I16F16`. — **Confidence: high.** **Source:** BODY_ROCKING_GHIDRA_REPORT.md §3 (constants, branches, ordering verified from disassembly).
- **`RockingState` as `Option<RockingState>` field on `GameEntity`.** Matches existing `animation`, `harvest_overlay` optional-component pattern. — **Confidence: high.** **Source:** repo pattern [src/sim/game_entity.rs](../../src/sim/game_entity.rs).
- **Atlas + real-time hybrid render path.** Atlas for `is_neutral()` units, real-time `vxl_compute.rs` per-frame for actively-tilting units. — **Confidence: medium.** Perf at 20k-unit scale is the open risk; design's fallback (LRU-cap with stale-slope drift) is documented. **Source:** design doc §"Alternatives Considered".
- **Coefficients in `[AudioVisual]` section.** — **Confidence: high.** **Source:** BODY_ROCKING_GHIDRA_REPORT.md §6.4 + binary `RulesClass::ReadAudioVisual` + the existing `[AudioVisual]` parser at [ruleset.rs:749](../../src/rules/ruleset.rs#L749).
- **`I16F16` precision for angles.** Resolution ~1.5e-5 rad sits just below the 2e-5 rad deadband. — **Confidence: high.** Verified resolution math in design doc §6.1.
- **Render-side SLERP via `glam::Quat::slerp` in f32.** Allowed because sim state stays fixed-point; render math doesn't feed back. — **Confidence: high.** Determinism preserved.
- **DirectRocker target gate: vehicles only (`WhatAmI == 1`).** — **Confidence: high.** **Source:** BODY_ROCKING_GHIDRA_REPORT.md §4.2 + GHIDRA `WarheadTypeClass::Detonate`.

## Open Questions

### Resolved During Planning

- **Coefficient INI section:** `[AudioVisual]`, NOT `[General]`. Grounded by [ruleset.rs:729](../../src/rules/ruleset.rs#L729).
- **Sim tick function signature pattern:** matches [src/sim/animation.rs:375](../../src/sim/animation.rs#L375) — direct mutation over `entities.keys_sorted()`. Inject after movement, before aircraft missions in `World::advance_tick`.
- **State hash inclusion pattern:** `Option<T>` hashed as `1u8` (present) + fields, or `0u8` (absent). Mirror existing pattern at [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs).
- **L8 forwards override (±π/10) gate:** Resolved via Ghidra during plan review. Triggered by `DriveLocomotionClass::Process_Drive_Track @ 0x004B1A31` setting `TechnoClass+0x6B5 = 1` when a Crusher vehicle is mid-crush of a building. NOT a "deployed building" gate as initially assumed. Deferred until building-crushing lands (no functional gap; the override never fires in practice today).

### Deferred to Implementation

- **The `vtable[0x298]` per-class rocking-gate.** **RESOLVED 2026-05-11 verify-doc audit:** gate is data-driven (NOT polymorphic). All 6 TechnoClass subclasses share one implementation at 0x006F9E10 that returns `(*(byte *)(TypeClass+0xB0) == 0)`. For Rust we approximate via `Option<RockingState>` at spawn: vehicles + buildings get `Some`, infantry + aircraft get `None`. Revisit only if a specific unit type's rocking looks wrong in-game.
- **L8 forwards ±π/10 override.** Gated by `TechnoClass+0x6B5` set in the vehicle-crushing-building code path (`DriveLocomotionClass::Process_Drive_Track @ 0x004B1A31`). Re-wire when building-crushing lands in sim/combat. Until then, all forwards saturation uses ±π/4 (parity drift only visible during the brief instant a Crusher vehicle impacts a building).
- **Where IsShipRocking gets cleared after EMP expires.** Deferred to the EMP system's timer-expiry path when that lands. Until then, ship-rocking persists until externally cleared.
- **Apply_area_damage force-floor constant `_DAT_007E5138`.** **RESOLVED 2026-05-11: 0.3 (double).** Gate is `force > 0.3` (after the 4.0 saturation). Implement directly.
- **DirectRocker normalization `_DAT_0081AEF8`.** **RESOLVED 2026-05-11: 100.0 (double), NOT 256.** Update all `/ 256` references in this plan to `/ 100.0`. Earlier 256 (Q8.8) guess would produce force impulses ~2.56× smaller than retail.
- **ApplyRocker secondary forwards dampener `_DAT_007E5168`.** **RESOLVED 2026-05-11: 0.5 (float).** When `no_dampen == false`, multiply forwards velocity by 0.5 (sideways velocity is NOT halved — asymmetric).
- **ApplyRocker `TypeClass+0x370` divisor.** **RESOLVED 2026-05-11: `Weight` field (double, default 2.0, retail range 0.5–5).** Per-unit divisor; heavier units rock proportionally less. Wired through `apply_rocker_impulse` and `ObjectType.weight` in Task 5 + Task 10 (L12c).
- **ApplyRocker distance attenuation `(0.04 − dist × 2.5e-5)`.** **L31, deferred** (documented parity drift). The impulse magnitude scales down linearly with distance from impact source; we approximate the factor as a constant 0.04 (the maximum). Effect: 3×3-cell area damage hits all targets at the same magnitude regardless of position within the radius, vs gamemd's ~24% reduction at the corner. Re-add later by replacing the literal `0.04` in `apply_rocker_impulse` with `(0.04 - dist × 2.5e-5).max(0.0)`.
- **ApplyRocker rate-timer jitter `_DAT_007E2810`.** **L32, deferred** (documented parity drift). Rotates the impulse direction by a small per-tick-varying angle. Effect: armies of identical vehicles taking the same artillery shell rock in lockstep instead of gamemd's subtly-staggered pattern. Re-add by introducing a per-tick rate timer in the sim and rotating `(dx, dy)` by `(rate_timer - 0x3FFF) × (-π/32768)` before normalization.
- **Performance at 20k-unit scale.** Validated by Task 21 benchmark. Fallback plan (LRU cap) documented in design doc §"Tech debt".

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/components.rs](../../src/sim/components.rs) | Add `RockingState` struct + `Default` + `is_neutral()` |
| Modify | [src/sim/game_entity.rs](../../src/sim/game_entity.rs) | Add `rocking: Option<RockingState>` field |
| Create | `src/sim/rocking/mod.rs` | Module entry: re-exports |
| Create | `src/sim/rocking/rocking_system.rs` | Per-tick spring-damper + slope-transition |
| Create | `src/sim/rocking/impulse.rs` | `apply_rocker_impulse` (port of ApplyRocker) |
| Create | `src/sim/rocking/self_destruct.rs` | Wide-amplitude self-destruct detection [L30] + `SelfDestructHook` trait |
| Create | `src/sim/rocking/rocking_tests.rs` | Unit + integration tests |
| Modify | [src/sim/mod.rs](../../src/sim/mod.rs) | `pub mod rocking;` |
| Modify | [src/sim/world/mod.rs](../../src/sim/world/mod.rs) | Insert `rocking::tick` between movement and aircraft phases |
| Modify | [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) | Hash `entity.rocking` (Option pattern) |
| Modify | [src/rules/warhead_type.rs](../../src/rules/warhead_type.rs) | Add `rocker`, `direct_rocker` bools |
| Modify | [src/rules/projectile_type.rs](../../src/rules/projectile_type.rs) | Add `rocker_scale: I8F8` |
| Modify | [src/rules/object_type.rs](../../src/rules/object_type.rs) | Add `weight: SimFixed` (Weight= INI key, default 2.0 — TypeClass+0x370, the divisor in ApplyRocker's force formula per L12c) |
| Modify | [src/rules/ruleset.rs](../../src/rules/ruleset.rs) | Add `direct_rocking_coefficient`, `fallback_coefficient` (parsed from `[AudioVisual]`); add `c4_warhead: Option<String>` (parsed from `[CombatDamage] C4Warhead=`, Rules+0xFA8 — reused by L30 self-destruct AND by C4 demolition) |
| Modify | [src/render/vxl_raster.rs](../../src/render/vxl_raster.rs) | Extend `VxlRenderParams` with `rocking_angles`, `slope_blend_matrix`; add `compute_slope_shear_translation` |
| Modify | [src/render/vxl_normals.rs](../../src/render/vxl_normals.rs) | Add `blinn_phong_pages_from_body_matrix` |
| Modify | [src/render/vxl_compute.rs](../../src/render/vxl_compute.rs) | Add `render_runtime` per-frame entry point + scratch texture pool |
| Modify | [src/render/unit_atlas.rs](../../src/render/unit_atlas.rs) | Pass body matrix to LUT pre-compute (closes G4 for upright-on-slope) |
| Modify | [src/app_instances/units.rs](../../src/app_instances/units.rs) | Atlas-vs-realtime branching at render hand-off |

## Interface Changes

- `GameEntity` gains `pub rocking: Option<RockingState>` — every entity-spawn site must decide `Some(default)` or `None`. (Default behavior in the spawn pattern: vehicles + buildings get `Some`, infantry + aircraft get `None`.)
- `WarheadType` gains two pub bool fields.
- `ProjectileType` gains one pub I8F8 field.
- `ObjectType` gains one pub `SimFixed` field (`weight`).
- `Ruleset` gains two pub I16F16 fields + one `Option<String>` (`c4_warhead`).
- `VxlRenderParams` gains two optional fields. Existing call sites get `None, None` defaults.
- `VxlComputeRenderer` gains a new `render_runtime` method. Existing offline-batch method unchanged.
- New `vxl_normals::blinn_phong_pages_from_body_matrix` — additive.
- New `sim/rocking` module — additive.
- State hash format extended — replays from before this change WILL fail hash validation. **Breaking change** for any persisted replay/savefile.

## Sim Checklist

- [x] All math uses `fixed`-point — `I16F16` (= `SimFixed`)
- [x] New state included in deterministic state hash (Task 3)
- [x] No dependencies on render/ui/sidebar/audio/net (sim/rocking only uses sim+rules+map+util)
- [x] Tick ordering impact: insert after movement (Phase 1), before aircraft missions (Phase 2). Rocking reads `cell.slope_type` after position update.
- [x] BTreeMap iteration order via `entities.keys_sorted()` (matches animation.rs pattern)

## Risk Areas

- **Determinism regression** — fixed-point spring-damper must produce bit-identical results across replays. Integration test (Task 12) explicitly checks `Simulation::state_hash()` parity across two seeded runs.
- **Atlas/real-time hand-off** — single branching point in [units.rs](../../src/app_instances/units.rs). Risk of double-draw or missed-draw if branching is wrong. Smoke test (Task 22) covers happy path; visual regression on existing slopes covers atlas-path stays unchanged.
- **vxl_compute per-frame perf at scale** — the largest unknown. Benchmark in Task 21 surfaces it before integration sign-off.
- **Replay/savefile breaking change** — state hash format changes once `RockingState` is added. Document in commit message.
- **Slope shear translation (L24, closes G7)** — Math is verbatim port of gamemd's shear; risk of off-by-sign mistake. Unit test (Task 14) compares to a precomputed reference.

## Parity-Critical Items

Per the design doc's tiny-detail ledger. Every item below has a `[L#]` tag matching design-doc L1–L30.

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 7 | Spring-damper integration order (integrate → saturate → dampen → deadband) | Wrong order changes whether saturation fires before damping; visible as different decay shape | Unit test step-by-step compares each axis value against precomputed expected at ticks 0/1/5/250 |
| Task 7 | Saturation cap ±π/4, but **only when stationary AND in-range AND crossing** [L7] | Moving vehicles must drift past π/4 without clamp — gamemd-specific behavior | Unit test: moving-vehicle case asserts angle > π/4 after impulse; stationary asserts angle == π/4 |
| Task 7 (DEFERRED) | Vehicle-crushing-building forwards override ±π/10 [L8] | gamemd uses ±π/10 when a Crusher vehicle is mid-crush of a building. Building-crushing isn't implemented yet → this gate never fires in practice. Plan uses ±π/4 for all forwards. | Re-enable when building-crushing lands. Documented as parity drift until then. |
| Task 7 | Strict velocity==0 short-circuit forces angle to 0 [L10] | Without this, a unit with zero velocity but small nonzero angle would never settle | Unit test: vel=0, angle=0.01 → next tick angle=0 |
| Task 7 | Deadband ±2e-5 snap-to-zero clears BOTH angle and velocity [L9] | If only one cleared, oscillation never terminates | Unit test: drift into deadband, assert both fields == 0 in same tick |
| Task 8 | Ship-rocking integrates without damping; one-sided clamp on each axis [L7 ship variant] | EMP wobble must persist while EMP is active, not decay during it | Unit test: 1000 ticks ship-rock with constant velocity, assert angle eventually clamps at boundary |
| Task 9 | Slope transition fires on `prev_slope != curr_slope`, counts 3 → 0 [L18, L20, L22] | Wrong count = visible snap on slope crossing | Integration test: drive entity across slope boundary, assert remaining=3,2,1,0 over 4 ticks |
| Task 10 | Impulse force computation, saturation at 4.0 [L11, L12] | Wrong force magnitude → rocking too small or too violent. Common in normal play. | Unit test compares output velocity against precomputed reference for known force input |
| Task 10 | Per-unit Weight divisor [L12c] | Without it, all vehicles rock identically per equivalent force — a Grizzly (Weight=2) and an Apocalypse (Weight=5) would rock the same instead of the Apocalypse rocking ~2.5× less. Visible in mixed-armor engagements every match. | Unit test: same impulse on Weight=2 and Weight=5 entities; assert resulting velocities differ by 2.5× ratio. Retail test: parse rulesmd.ini, assert HTNK (Apocalypse) Weight=5 and MTNK (Grizzly) Weight=2. |
| Task 10 | Per-axis velocity cap at 0.05 rad/tick [L13] | Without cap, large impulses produce instantaneous rotation past π/4 | Unit test: feed impulse with force 100; assert resulting vel <= 0.05 |
| Task 13–17 | Real-time render path produces correct body matrix = facing × slope_blend × rocking | Without correct composition, tilted units render at wrong angle | Smoke test (Task 22): visually compare side-by-side with gamemd reference frame |
| Task 14 | Slope shear translation offsets (L24, closes G7) | Without shear, the tilted body visually floats above or sinks into the slope surface | Unit test: precomputed reference shear values for slope_type 1–16 |
| Task 15 | Lighting LUT uses body-local light (full body matrix), not facing-only [L27] | Lighting on slope-tilted or rocking units must "follow" the body orientation | Visual smoke test: side-lit unit on slope shows correct dark-side; tilted unit's bright side shifts with tilt |
| Task 17 | Atlas path neutral check uses both rocking angles AND transition timer | If neutral check is wrong, units either always use real-time (perf hit) or never use it (parity drift) | Integration test asserts `is_neutral()` for spawned default unit, false after impulse, true after 250-tick decay |
| Task 18 | Atlas keys remain unchanged — atlas continues to bake per (type, facing, layer, frame, slope_type) | Adding new key dimensions explodes atlas; must not happen | Code review: no new fields added to `UnitSpriteKey` |
| Task 19 | Atlas-side LUT recompute uses body matrix that includes slope (per-slope-variant correct lighting, closes G4 for upright on slopes) | Upright units on slopes get wrong lighting without this | Visual smoke test: park a tank on a ramp, compare lighting to gamemd ref |
| Task 7b (NEW) | Wide-amplitude self-destruct: detect `|angle| > π` at end of tick → self-damage with max_hp using `Rules.c4_warhead`, force-kill=true [L30] | Faithful kill-on-tipover path. Almost never fires in retail (constants are tuned safely) but a modded warhead or sustained EMP on an unprotected type would diverge if omitted. Parity completeness. | Unit test: directly set `angle_sideways = 4.0` (above π), advance one tick; assert damage hook called with `max_hp` and `force_kill=true`. Integration test (when combat lands): simulate EMP on type with `ship_rock_clamp_disabled`; assert unit dies within ~2s. |

---

## Tasks

### Phase A — Sim Foundation (Tasks 1-5)

### Task 1: Define `RockingState` struct + constants

**Why:** Defines the per-entity rocking component. Pure types, no logic; every subsequent sim task depends on this.

**Files:**
- Modify: [src/sim/components.rs](../../src/sim/components.rs) (append at the end of the existing components)

**Pattern:** Mirror `HarvestOverlay` / `Animation` optional-component pattern (file already follows it).

**Step 1: Add imports + struct**

```rust
// src/sim/components.rs (append)

use fixed::types::{I8F8, I16F16};

/// Body rocking and slope-transition state for voxel-bodied units.
///
/// Tracks both the spring-damped roll/pitch angles (driven by weapon impacts
/// and EMP wobble) and the 3-tick quaternion-SLERP slope transition when the
/// unit moves to a cell with a different slope_type.
///
/// Optional component on `GameEntity` — present on vehicles, ships, and
/// voxel-bodied buildings; `None` for infantry, aircraft, SHP-bodied
/// buildings.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct RockingState {
    /// Roll angle, rad. Sign convention matches gamemd's AngleRotatedSideways.
    pub angle_sideways: I16F16,
    /// Pitch angle, rad. Sign convention matches gamemd's AngleRotatedForwards.
    pub angle_forwards: I16F16,
    /// Roll angular velocity, rad/tick.
    pub vel_sideways: I16F16,
    /// Pitch angular velocity, rad/tick.
    pub vel_forwards: I16F16,
    /// If true, integrate without damping (EMP wobble, naval continuous rocking).
    pub is_ship_rocking: bool,
    /// Slope_type before the current transition (== curr_slope when no transition).
    pub prev_slope: u8,
    /// Current cell's slope_type.
    pub curr_slope: u8,
    /// Counts down from 3 to 0. Nonzero ⇒ render-time SLERP between prev and curr.
    pub transition_ticks_remaining: u8,
}

impl RockingState {
    /// Tilt-renderer deadband (matches gamemd VXL_DRAW_MATRIX §13 epsilon).
    pub const DEADBAND: I16F16 = I16F16::lit("0.00002");

    /// Returns true when the unit can render via the static atlas path
    /// (no active rocking, no in-progress slope transition).
    pub fn is_neutral(&self) -> bool {
        !self.is_ship_rocking
            && self.transition_ticks_remaining == 0
            && self.angle_sideways.abs() <= Self::DEADBAND
            && self.angle_forwards.abs() <= Self::DEADBAND
    }
}
```

**Step 2: Add unit tests in same file**

```rust
// in #[cfg(test)] mod tests near bottom of components.rs

#[test]
fn rocking_default_is_neutral() {
    let r = RockingState::default();
    assert!(r.is_neutral());
}

#[test]
fn rocking_active_angle_is_not_neutral() {
    let mut r = RockingState::default();
    r.angle_sideways = I16F16::lit("0.01");
    assert!(!r.is_neutral());
}

#[test]
fn rocking_within_deadband_is_neutral() {
    let mut r = RockingState::default();
    r.angle_sideways = I16F16::lit("0.000015"); // below 2e-5
    assert!(r.is_neutral());
}

#[test]
fn rocking_transition_is_not_neutral() {
    let mut r = RockingState::default();
    r.transition_ticks_remaining = 1;
    assert!(!r.is_neutral());
}

#[test]
fn rocking_ship_rocking_is_not_neutral() {
    let mut r = RockingState::default();
    r.is_ship_rocking = true;
    assert!(!r.is_neutral());
}
```

**Step 3: Verify**

Run: `cargo test --lib sim::components::tests::rocking -- --nocapture`
Expected: 5 PASS

**Step 4: Commit**

```
sim/components: add RockingState component (struct + is_neutral)

Body rocking + slope-transition state for voxel-bodied units. Optional
component on GameEntity. is_neutral() drives the atlas-vs-realtime
render decision.
```

---

### Task 2: Add `rocking: Option<RockingState>` field to `GameEntity`

**Why:** Wires the new component into the entity layout. Every entity-spawn site can now hold rocking state.

**Files:**
- Modify: [src/sim/game_entity.rs](../../src/sim/game_entity.rs)

**Pattern:** Same as existing `animation: Option<Animation>` / `harvest_overlay: Option<HarvestOverlay>` fields.

**Step 1: Add the field**

In `GameEntity` struct, alongside other optional components:

```rust
/// Body rocking + slope-transition state. None for entities that don't rock
/// (infantry, aircraft, SHP-bodied buildings).
pub rocking: Option<crate::sim::components::RockingState>,
```

**Step 2: Update `Default` (if `GameEntity` derives or implements it) — the field must default to `None`**

If `GameEntity` derives `Default`, no action needed (Option defaults to None). If it has a manual `Default` impl, add `rocking: None`.

**Step 3: Verify spawn sites compile**

Run: `cargo build`
Expected: PASS (Option<X> defaults to None automatically in struct literal initializers IF Default is used; otherwise every `GameEntity { … }` literal needs `rocking: None`).

If the build fails on spawn sites, add `rocking: None,` to each one. Spawn sites to check:
- [src/sim/world/world_spawn.rs](../../src/sim/world/world_spawn.rs) — every place that constructs a `GameEntity` literal

**Step 4: Commit**

```
sim/game_entity: add rocking: Option<RockingState> field

Wires the new component into the entity layout. None by default; populated
at spawn for vehicles + voxel-bodied buildings in a later task.
```

---

### Task 3: Hash `entity.rocking` in state hash

**Why:** State hash is the lockstep determinism check. New state must contribute to the hash; otherwise replays / multiplayer can diverge silently.

**Files:**
- Modify: [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs)

**Pattern:** Existing optional-component hash pattern — `1u8.hash` tag + fields if `Some`, `0u8.hash` if `None`.

**Step 1: Find the entity hashing block**

Open `world_hash.rs`, locate the function (likely `hash_entities` or inline in `state_hash`) that walks each entity's fields. Find the existing optional-component hash blocks (e.g. for `animation`, `harvest_overlay`) to mirror.

**Step 2: Add the rocking hash block**

After existing optional-component hashes, add:

```rust
match entity.rocking {
    Some(ref r) => {
        1u8.hash(hasher);
        r.angle_sideways.to_bits().hash(hasher);
        r.angle_forwards.to_bits().hash(hasher);
        r.vel_sideways.to_bits().hash(hasher);
        r.vel_forwards.to_bits().hash(hasher);
        r.is_ship_rocking.hash(hasher);
        r.prev_slope.hash(hasher);
        r.curr_slope.hash(hasher);
        r.transition_ticks_remaining.hash(hasher);
    }
    None => 0u8.hash(hasher),
}
```

Note: `I16F16` doesn't implement `Hash` directly; use `.to_bits()` to get the underlying `i32` representation, which does.

**Step 3: Add a determinism test**

In the same file (or a `world_hash_tests.rs` sibling):

```rust
#[test]
fn rocking_state_contributes_to_hash() {
    let mut a = test_world_with_one_vehicle();
    let mut b = test_world_with_one_vehicle();
    assert_eq!(a.state_hash(), b.state_hash());
    
    // Modify only the rocking state of one
    if let Some(entity) = a.entities.values_mut().next() {
        entity.rocking = Some(RockingState {
            angle_sideways: I16F16::lit("0.1"),
            ..Default::default()
        });
    }
    assert_ne!(a.state_hash(), b.state_hash());
}
```

If `test_world_with_one_vehicle()` doesn't exist, write a minimal helper that creates a `Simulation` with one vehicle entity.

**Step 4: Verify**

Run: `cargo test --lib world_hash`
Expected: existing tests still pass + new rocking-hash test PASSES.

**Step 5: Commit**

```
sim/world: include rocking state in deterministic hash

Required for replay/lockstep correctness. Breaking change for any persisted
state from before this commit.
```

---

### Task 4: Add `Rocker` and `DirectRocker` to `WarheadType`

**Why:** Per-warhead INI gates for the two impulse paths. Without these, no impulse source can ever fire — the rocking machinery would sit idle.

**Files:**
- Modify: [src/rules/warhead_type.rs](../../src/rules/warhead_type.rs)

**Pattern:** Existing bool fields (e.g. `cell_spread`, `inf_death`) parsed via `section.get_bool(...).unwrap_or(default)`.

**Step 1: Add fields to `WarheadType` struct**

```rust
/// Enables area-damage rocking (Rocker= in [Warhead] section). When yes, the
/// warhead's detonation pushes a rocker impulse into every vehicle in a 3x3
/// cell radius. Default `no`. Source: Warhead+0x14E.
pub rocker: bool,

/// Enables direct-hit rocking (DirectRocker= in [Warhead] section). When yes,
/// fires an impulse on the bullet's target if the target is a vehicle.
/// Default `no`. Source: Warhead+0x14F.
pub direct_rocker: bool,
```

**Step 2: Add to `from_ini_section`**

Alongside other bool parses:

```rust
rocker: section.get_bool("Rocker").unwrap_or(false),
direct_rocker: section.get_bool("DirectRocker").unwrap_or(false),
```

**Step 3: Update tests**

Find the existing warhead-parsing tests (likely in [src/rules/warhead_type.rs](../../src/rules/warhead_type.rs) under `#[cfg(test)]`). Add:

```rust
#[test]
fn parse_rocker_default_false() {
    let ini = IniFile::from_str("[TestWH]\n");
    let wh = WarheadType::from_ini_section("TestWH", ini.section("TestWH").unwrap());
    assert!(!wh.rocker);
    assert!(!wh.direct_rocker);
}

#[test]
fn parse_rocker_yes() {
    let ini = IniFile::from_str("[TestWH]\nRocker=yes\nDirectRocker=yes\n");
    let wh = WarheadType::from_ini_section("TestWH", ini.section("TestWH").unwrap());
    assert!(wh.rocker);
    assert!(wh.direct_rocker);
}
```

**Step 4: Verify retail INI parses**

Stock `rulesmd.ini` has `Rocker=yes` on many warheads (SonicWarhead, V3WH, BlimpHE, etc.). Add a quick integration assertion:

```rust
#[test]
fn parse_retail_v3wh_has_rocker() {
    let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("rulesmd.ini missing");
    let ini = IniFile::from_str(&ini_text);
    let section = ini.section("V3WH").expect("V3WH missing from rulesmd.ini");
    let wh = WarheadType::from_ini_section("V3WH", section);
    assert!(wh.rocker, "V3WH should have Rocker=yes");
}
```

**Step 5: Run + commit**

Run: `cargo test --lib warhead`
Expected: existing tests pass + 3 new tests pass.

Commit:
```
rules/warhead: parse Rocker and DirectRocker flags

Both default false. Per-warhead gates for the two body-rocking impulse paths.
Verified V3WH parses Rocker=yes from retail rulesmd.ini.
```

---

### Task 5: Add `rocker_scale: I8F8` to `ProjectileType` + `weight: SimFixed` to `ObjectType` + `[AudioVisual]` coefficients + `C4Warhead` to `Ruleset`

**Why:** Round out the INI plumbing. `ProjectileType.RockerScale`, `ObjectType.Weight`, and Rules coefficients are needed to compute impulse magnitude per L11/L12/L12c. `Ruleset.c4_warhead` is the warhead used by L30's self-destruct.

**Files:**
- Modify: [src/rules/projectile_type.rs](../../src/rules/projectile_type.rs)
- Modify: [src/rules/object_type.rs](../../src/rules/object_type.rs)
- Modify: [src/rules/ruleset.rs](../../src/rules/ruleset.rs)

**Pattern:**
- For `RockerScale`: existing fields like `speed: i32` parsed via `get_i32`. RockerScale is float in INI, Q8.8 internal. Use `get_f32(...).map(I8F8::from_num)`.
- For `Weight`: parallel to `accel_factor` at [object_type.rs:725-728](../../src/rules/object_type.rs#L725). Float in INI, SimFixed (= I16F16) internal. Use `get_f32(...).map(sim_from_f32)`.
- For Rules coefficients: existing fields like `condition_yellow` parsed from `[AudioVisual]` section at [ruleset.rs:749-820](../../src/rules/ruleset.rs#L749).

**Step 1: ProjectileType — add field**

In `ProjectileType` struct:

```rust
/// Per-bullet rocker force scale (RockerScale= in [Projectile] section).
/// Multiplies the DirectRocker impulse force. Default 1.0 (Q8.8 representation
/// of 1.0). Source: Bullet+0x150.
pub rocker_scale: I8F8,
```

In the parser (alongside other fields):

```rust
rocker_scale: section
    .get_f32("RockerScale")
    .map(|v| I8F8::from_num(v))
    .unwrap_or(I8F8::ONE),
```

Test in same file:

```rust
#[test]
fn parse_rocker_scale_default_one() {
    let ini = IniFile::from_str("[TestBullet]\n");
    let p = ProjectileType::from_ini_section("TestBullet", ini.section("TestBullet").unwrap(), None);
    assert_eq!(p.rocker_scale, I8F8::ONE);
}

#[test]
fn parse_rocker_scale_custom() {
    let ini = IniFile::from_str("[TestBullet]\nRockerScale=2.5\n");
    let p = ProjectileType::from_ini_section("TestBullet", ini.section("TestBullet").unwrap(), None);
    assert_eq!(p.rocker_scale, I8F8::from_num(2.5));
}
```

**Step 1b: ObjectType — add `weight` field**

In `ObjectType` struct (alongside `accel_factor`, `decel_factor`, `slowdown_distance` around [object_type.rs:152](../../src/rules/object_type.rs#L152)):

```rust
/// Inertia weight (`Weight=` in vehicle/aircraft/infantry sections).
/// Default 2.0. Used as the divisor in `apply_rocker_impulse` (L12c):
/// `force_scaled = (…) × force / weight`. Heavier units rock less per
/// equivalent impulse. Source: TechnoTypeClass+0x370 (double in gamemd).
/// Retail range: 0.5 (light units like harvesters/IFVs) to 5 (Apocalypse).
pub weight: SimFixed,
```

In the parser body (alongside `accel_factor` around line 725):

```rust
weight: section
    .get_f32("Weight")
    .map(sim_from_f32)
    .unwrap_or(SimFixed::lit("2.0")),
```

Tests in `object_type.rs` (alongside existing ObjectType parse tests around line 1574):

```rust
#[test]
fn parse_weight_default_two() {
    let ini = IniFile::from_str("[MTNK]\nName=Grizzly Battle Tank\nCost=700\nStrength=300\n");
    let ot = ObjectType::from_ini_section("MTNK", ini.section("MTNK").unwrap(), None);
    assert_eq!(ot.weight, SimFixed::lit("2.0"));
}

#[test]
fn parse_weight_custom() {
    let ini = IniFile::from_str("[HTNK]\nWeight=5\n");
    let ot = ObjectType::from_ini_section("HTNK", ini.section("HTNK").unwrap(), None);
    assert_eq!(ot.weight, SimFixed::lit("5"));
}

#[test]
fn parse_retail_weight_apocalypse_is_five() {
    // Retail rulesmd.ini: HTNK (Apocalypse Tank) has Weight=5.
    // This verifies a key parity case — without Weight, Apocalypse would rock
    // as much as a Grizzly under equivalent impulse force.
    let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("rulesmd.ini missing");
    let ini = IniFile::from_str(&ini_text);
    let htnk = ObjectType::from_ini_section("HTNK", ini.section("HTNK").expect("HTNK section"), None);
    assert_eq!(htnk.weight, SimFixed::lit("5"));
}
```

**Step 2: Ruleset — add fields to the existing audio-visual block + a CombatDamage hook for L30**

In `Ruleset` struct (alongside `condition_yellow`, `condition_red`, `building_garrisoned_sound`):

```rust
/// Direct rocker force coefficient (DirectRockingCoefficient= in [AudioVisual]).
/// Multiplies (RockerScale × Damage / 256 [Q8.8 shift] / 100.0 [Rules normalization]) to get final force. Default 1.5.
/// Source: RulesClass+0x18B4. Retail rulesmd.ini line 620.
pub direct_rocking_coefficient: I16F16,

/// Damping coefficient when IsMoving (FallBackCoefficient= in [AudioVisual]).
/// Multiplies base 0.002 rad/tick decay rate for moving vehicles. Default 0.1
/// → ±0.0002 rad/tick effective decay. Source: RulesClass+0x18B8. Retail
/// rulesmd.ini line 621.
pub fallback_coefficient: I16F16,

/// "Absolute damage" warhead from [CombatDamage] C4Warhead=.
/// Source: RulesClass+0xFA8. Retail rulesmd.ini line 818: `C4Warhead=Super`
/// (designer-annotated: "This warhead is used throughout the code to mean
/// 'Absolute damage'"). Reused by THREE engine paths:
///   1. Tanya/SEAL/Engineer C4 demolition (the original intent),
///   2. RockingUpdate's wide-amplitude self-destruct at |angle| > π (L30),
///   3. Other catastrophic-state code paths the audit hasn't fully traced.
/// We hold this as an INI-resolved Warhead reference; the actual key lookup
/// (string → WarheadType*) happens after WarheadType vector is built.
pub c4_warhead: Option<String>,
```

In the `[AudioVisual]` parser block at [ruleset.rs:749+](../../src/rules/ruleset.rs#L749):

```rust
direct_rocking_coefficient: audio_visual
    .and_then(|av| av.get_f32("DirectRockingCoefficient"))
    .map(sim_from_f32)
    .unwrap_or(I16F16::lit("1.5")),

fallback_coefficient: audio_visual
    .and_then(|av| av.get_f32("FallBackCoefficient"))
    .map(sim_from_f32)
    .unwrap_or(I16F16::lit("0.1")),
```

In the `[CombatDamage]` parser block (locate via `grep -n '\[CombatDamage\]' src/rules/ruleset.rs`):

```rust
c4_warhead: combat_damage
    .and_then(|cd| cd.get_string("C4Warhead"))
    .map(|s| s.to_string()),
```

(Adjust the chain to match the existing pattern — the `audio_visual` and `combat_damage` binding types and how other fields handle the Option.)

**Step 3: Tests**

```rust
#[test]
fn parse_rules_rocking_coefficients_defaults() {
    let ini = IniFile::from_str("[AudioVisual]\n");
    let r = Ruleset::from_ini(&ini).expect("parse");
    assert_eq!(r.direct_rocking_coefficient, I16F16::lit("1.5"));
    assert_eq!(r.fallback_coefficient, I16F16::lit("0.1"));
}

#[test]
fn parse_rules_rocking_coefficients_explicit() {
    let ini = IniFile::from_str(
        "[AudioVisual]\nDirectRockingCoefficient=2.0\nFallBackCoefficient=0.05\n"
    );
    let r = Ruleset::from_ini(&ini).expect("parse");
    assert_eq!(r.direct_rocking_coefficient, I16F16::lit("2"));
    assert_eq!(r.fallback_coefficient, I16F16::lit("0.05"));
}

#[test]
fn parse_retail_rules_rocking_coefficients() {
    let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("rulesmd.ini missing");
    let ini = IniFile::from_str(&ini_text);
    let r = Ruleset::from_ini(&ini).expect("parse");
    assert_eq!(r.direct_rocking_coefficient, I16F16::lit("1.5"));
    assert_eq!(r.fallback_coefficient, I16F16::lit("0.1"));
}

#[test]
fn parse_rules_c4_warhead_default_none() {
    let ini = IniFile::from_str("[CombatDamage]\n");
    let r = Ruleset::from_ini(&ini).expect("parse");
    assert_eq!(r.c4_warhead, None);
}

#[test]
fn parse_retail_rules_c4_warhead_is_super() {
    let ini_text = std::fs::read_to_string("ini/rulesmd.ini").expect("rulesmd.ini missing");
    let ini = IniFile::from_str(&ini_text);
    let r = Ruleset::from_ini(&ini).expect("parse");
    assert_eq!(r.c4_warhead.as_deref(), Some("Super"));
}
```

**Step 4: Run + commit**

Run: `cargo test --lib rules`
Expected: existing tests pass + 5 new tests pass.

Commit:
```
rules: parse RockerScale and [AudioVisual] rocking coefficients

ProjectileType gains rocker_scale (Q8.8, default 1.0). Ruleset gains
direct_rocking_coefficient (default 1.5) and fallback_coefficient
(default 0.1) from the [AudioVisual] section. Verified retail rulesmd.ini
parses with expected values.
```

---

### Phase B — Sim System Implementation (Tasks 6-10)

### Task 6: Create `sim/rocking/` module skeleton + constants

**Why:** Bring the new module online with just the constants and module wiring. Subsequent tasks add the logic.

**Files:**
- Create: `src/sim/rocking/mod.rs`
- Create: `src/sim/rocking/rocking_system.rs`
- Modify: [src/sim/mod.rs](../../src/sim/mod.rs)

**Pattern:** Mirror `src/sim/miner/` and `src/sim/animation.rs` module layout.

**Step 1: Create `src/sim/rocking/mod.rs`**

```rust
//! Body rocking and slope-transition simulation.
//!
//! Implements the spring-damper that drives `RockingState::angle_*` toward zero
//! each tick, plus the 3-tick slope-transition tracker. Renderer reads the
//! resulting angles + slope-blend matrix to compose the body matrix per frame.
//!
//! Source: gamemd.exe TechnoClass::RockingUpdate @ 0x0070B570. See
//! ra2-rust-game-docs/BODY_ROCKING_GHIDRA_REPORT.md for the full RE.
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on sim/components, sim/entity_store, map,
//!   rules, util/fixed_math. Never on render/, ui/, audio/, net/.

pub mod rocking_system;
pub mod impulse;

#[cfg(test)]
mod rocking_tests;

pub use rocking_system::tick;
pub use impulse::apply_rocker_impulse;
```

**Step 2: Create `src/sim/rocking/rocking_system.rs` with constants only**

```rust
//! Per-tick spring-damper + slope-transition advance.

use fixed::types::I16F16;

use crate::sim::components::RockingState;
use crate::sim::entity_store::EntityStore;

/// Renderer/sim deadband. Both angles below ⇒ snap to zero. Matches
/// gamemd's 2e-5 rad double-precision constant at 0x007EC0B0.
pub const TILT_DEADBAND: I16F16 = I16F16::lit("0.00002");

/// Saturation cap for body roll/pitch. Matches gamemd's +π/4 float at 0x007EF8F8.
pub const SATURATION_PI4: I16F16 = I16F16::lit("0.7853982");

/// Forwards saturation cap during vehicle-vs-building crush. Matches
/// gamemd's +π/10 float at 0x007F4E64. (Earlier label "deployed buildings"
/// was wrong — the gate is `TechnoClass+0x6B5 != 0` which is set by
/// `DriveLocomotionClass::Process_Drive_Track` during a crush, not by
/// deploy/garrison state. Task 7 stays DEFERRED until building-crushing
/// lands.)
pub const SATURATION_PI10: I16F16 = I16F16::lit("0.3141593");

/// "Out of normal range" threshold (±π/2). Above this, dampening signs flip
/// to push back inward regardless of IsMoving. Matches gamemd's ±π/2 floats
/// at 0x007E897C / 0x007E8980.
pub const NORMAL_RANGE_PI2: I16F16 = I16F16::lit("1.5707963");

/// Base damping rate (rad/tick). Multiplied by FallBackCoefficient when
/// IsMoving != 0. Matches gamemd's 0.002 float at 0x007F4E70.
pub const BASE_DECAY_RATE: I16F16 = I16F16::lit("0.002");

/// Snap-back rate for the velocity-fighting-itself sub-branch (rad/tick).
/// Matches gamemd's 0.005 double at 0x007F4E68.
pub const SNAP_BACK_RATE: I16F16 = I16F16::lit("0.005");

/// Per-axis velocity cap applied at impulse-receive time. Matches gamemd's
/// 0.05 float saturation in ApplyRocker.
pub const IMPULSE_VEL_CAP: I16F16 = I16F16::lit("0.05");

/// Slope-transition duration in sim ticks. Hard-coded in gamemd
/// (CDTimerClass::Start(3)).
pub const SLOPE_TRANSITION_TICKS: u8 = 3;

/// Saturation cap on rocker impulse force from area-damage. Source:
/// Apply_area_damage clamp at force == 4.0 (_DAT_007E3CC8 double = 4.0,
/// verified 2026-05-11). Same threshold and clamp value used in
/// Detonate's DirectRocker path.
pub const FORCE_SATURATION: I16F16 = I16F16::lit("4");

/// Minimum force for Apply_area_damage's Rocker-yes 3×3 cell loop to fire
/// at all (after FORCE_SATURATION clamp). Below this, no impulses are
/// applied to targets in the radius. Source: _DAT_007E5138 double = 0.3,
/// verified 2026-05-11. Earlier working assumption was `> 0.0` which would
/// have fired the loop too often for weak warheads.
pub const APPLY_AREA_FORCE_FLOOR: I16F16 = I16F16::lit("0.3");

/// Stub entry point — implemented in Task 11.
pub fn tick(_entities: &mut EntityStore) {
    // Implemented in Task 11.
}
```

**Step 3: Create stub `src/sim/rocking/impulse.rs`**

```rust
//! Rocker impulse application (port of TechnoClass::ApplyRocker @ 0x0070B280).

use crate::sim::components::RockingState;

/// Stub — implemented in Task 10.
pub fn apply_rocker_impulse(_rocking: &mut RockingState) {
    // Implemented in Task 10.
}
```

**Step 4: Add module declaration**

In [src/sim/mod.rs](../../src/sim/mod.rs):

```rust
pub mod rocking;
```

**Step 5: Create empty `src/sim/rocking/rocking_tests.rs`**

```rust
//! Tests for the rocking system.

#![cfg(test)]
```

**Step 6: Verify build + commit**

Run: `cargo build`
Expected: PASS (no warnings about unused imports — adjust as needed).

Commit:
```
sim/rocking: module skeleton + constants

Adds the module structure and all 9 ledgered constants extracted from
gamemd.exe binary memory. Logic comes in subsequent tasks.
```

---

### Task 7: Implement `advance_axis` spring-damper

**Why:** The core per-axis math. Lifts L2–L10 out of the design ledger into executable code. Every subsequent rocking behavior depends on this.

**Files:**
- Modify: `src/sim/rocking/rocking_system.rs`
- Modify: `src/sim/rocking/rocking_tests.rs`

**Pattern:** Pure function over `(angle, velocity, cap, is_moving, fallback)` — testable without `Simulation`.

**Step 1: Implement `advance_axis`**

In `rocking_system.rs`:

```rust
/// Advance one rocking axis (sideways OR forwards) by one tick.
///
/// Order: zero-velocity short-circuit → integrate → saturate (stationary+in-range
/// only) → dampen → deadband snap. Matches gamemd's RockingUpdate normal path
/// (BODY_ROCKING_GHIDRA_REPORT.md §3.3).
///
/// `cap` is ±π/4 for sideways and most forwards, or ±π/10 for forwards on
/// deployed buildings (per L8).
pub(crate) fn advance_axis(
    angle: &mut I16F16,
    velocity: &mut I16F16,
    cap: I16F16,
    is_moving: bool,
    fallback: I16F16,
) {
    // L10: strict velocity == 0 → angle force-zero, skip integration.
    if *velocity == I16F16::ZERO {
        *angle = I16F16::ZERO;
        return;
    }

    // L2: integrate.
    let prev = *angle;
    let new_angle = prev + *velocity;
    *angle = new_angle;

    let in_range = angle.abs() <= NORMAL_RANGE_PI2;

    // L7: saturation fires only when stationary, in normal range, and crossing.
    if !is_moving && in_range {
        if new_angle > cap && prev < cap {
            *angle = cap;
            *velocity = I16F16::ZERO;
        } else if new_angle < -cap && prev > -cap {
            *angle = -cap;
            *velocity = I16F16::ZERO;
        }
    }

    // L3 / L4 / L5: dampening. Moving units use fallback-scaled rate; stationary
    // use base rate; out-of-range pushes back inward at base rate regardless.
    let decay = if is_moving { fallback * BASE_DECAY_RATE } else { BASE_DECAY_RATE };
    if *velocity > I16F16::ZERO {
        *velocity -= if in_range { decay } else { -BASE_DECAY_RATE };
    } else if *velocity < I16F16::ZERO {
        *velocity += if in_range { decay } else { -BASE_DECAY_RATE };
    }

    // L9: deadband snap clears both angle and velocity in the same tick.
    if angle.abs() <= TILT_DEADBAND {
        *angle = I16F16::ZERO;
        *velocity = I16F16::ZERO;
    }
}
```

**Step 2: Unit tests**

In `rocking_tests.rs`:

```rust
use fixed::types::I16F16;
use crate::sim::rocking::rocking_system::*;

const FALLBACK: I16F16 = I16F16::lit("0.1");
const ZERO: I16F16 = I16F16::ZERO;

#[test]
fn zero_velocity_force_zeros_angle() {
    let mut a = I16F16::lit("0.5");
    let mut v = ZERO;
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    assert_eq!(a, ZERO);
}

#[test]
fn integrate_simple() {
    let mut a = I16F16::lit("0.1");
    let mut v = I16F16::lit("0.01");
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    // Integrate: a = 0.1 + 0.01 = 0.11; not yet at cap; stationary in-range dampens base.
    // Then dampen: v positive, in_range → v -= BASE_DECAY_RATE → v = 0.01 - 0.002 = 0.008.
    assert!((a - I16F16::lit("0.11")).abs() < I16F16::lit("0.0001"));
    assert!((v - I16F16::lit("0.008")).abs() < I16F16::lit("0.0001"));
}

#[test]
fn saturation_clamps_when_stationary_and_crossing() {
    let mut a = I16F16::lit("0.78"); // just below π/4 = 0.7854
    let mut v = I16F16::lit("0.05");
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    // new = 0.83; crosses π/4 from below; stationary → clamp + zero velocity.
    assert_eq!(a, SATURATION_PI4);
    assert_eq!(v, ZERO);
}

#[test]
fn saturation_does_NOT_clamp_when_moving() {
    let mut a = I16F16::lit("0.78");
    let mut v = I16F16::lit("0.05");
    advance_axis(&mut a, &mut v, SATURATION_PI4, true /* moving */, FALLBACK);
    // Moving: no clamp; angle exceeds π/4.
    assert!(a > SATURATION_PI4);
}

#[test]
fn pi10_cap_is_supported_via_parameter() {
    // The ±π/10 cap path is correct (the L8 case in gamemd: a vehicle mid-crush
    // of a building uses this tighter cap). Building-crushing isn't implemented
    // in our codebase yet, so the caller in Task 11 wires SATURATION_PI4
    // unconditionally. This test verifies the math still works correctly when
    // the cap parameter is varied, so re-enabling L8 later is a one-line change.
    let mut a = I16F16::lit("0.30"); // just below π/10 = 0.3142
    let mut v = I16F16::lit("0.05");
    advance_axis(&mut a, &mut v, SATURATION_PI10, false, FALLBACK);
    assert_eq!(a, SATURATION_PI10);
    assert_eq!(v, ZERO);
}

#[test]
fn deadband_snaps_to_zero() {
    let mut a = I16F16::lit("0.00001"); // below 2e-5 deadband
    let mut v = I16F16::lit("0.0001");
    advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    assert_eq!(a, ZERO);
    assert_eq!(v, ZERO);
}

#[test]
fn convergence_decays_to_zero_over_time() {
    let mut a = I16F16::ZERO;
    let mut v = I16F16::lit("0.05"); // max impulse velocity
    // Stationary, in-range: dampens at BASE_DECAY_RATE (0.002 / tick).
    // Should reach deadband within ~30 ticks (0.05 / 0.002 = 25 ticks of velocity decay).
    for _ in 0..500 {
        advance_axis(&mut a, &mut v, SATURATION_PI4, false, FALLBACK);
    }
    assert!(a.abs() <= TILT_DEADBAND);
    assert!(v.abs() <= TILT_DEADBAND);
}
```

**Step 3: Verify + commit**

Run: `cargo test --lib rocking`
Expected: 7 PASS.

Commit:
```
sim/rocking: spring-damper advance_axis core

Per-axis fixed-point integration + saturation + dampening + deadband.
Covers ledger items L2-L10. Tested against precomputed reference values
for stationary, moving, deployed-building, and deadband cases.
```

---

### Task 8: Implement `advance_ship_rocking` (no-damping path)

**Why:** Separate path for EMP wobble + naval continuous rocking. Integrates without damping; one-sided clamps. Matches gamemd's IsShipRocking branch.

**Files:**
- Modify: `src/sim/rocking/rocking_system.rs`
- Modify: `src/sim/rocking/rocking_tests.rs`

**Step 1: Implement**

```rust
/// Advance ship-rocking path: integrate without damping, one-sided clamp.
/// Matches gamemd's IsShipRocking branch (BODY_ROCKING_GHIDRA_REPORT.md §3.2).
///
/// `type_supports_ship_rocking` corresponds to TypeClass+0xD6A in gamemd.
/// When false, angles are integrated but never clamped — used by units that
/// "shouldn't" ship-rock but somehow had the flag set.
pub(crate) fn advance_ship_rocking(rocking: &mut RockingState, type_supports: bool) {
    rocking.angle_forwards += rocking.vel_forwards;
    rocking.angle_sideways += rocking.vel_sideways;
    if !type_supports {
        return;
    }
    // Lower clamps to -π/4 (both axes).
    if rocking.angle_forwards < -SATURATION_PI4 {
        rocking.angle_forwards = -SATURATION_PI4;
    }
    if rocking.angle_sideways < -SATURATION_PI4 {
        rocking.angle_sideways = -SATURATION_PI4;
    }
    // Upper clamp on sideways only.
    if rocking.angle_sideways >= SATURATION_PI4 {
        rocking.angle_sideways = SATURATION_PI4;
    }
}
```

**Step 2: Tests**

```rust
use crate::sim::components::RockingState;

#[test]
fn ship_rocking_integrates_without_damping() {
    let mut r = RockingState::default();
    r.vel_sideways = I16F16::lit("0.01");
    advance_ship_rocking(&mut r, true);
    assert_eq!(r.angle_sideways, I16F16::lit("0.01"));
    assert_eq!(r.vel_sideways, I16F16::lit("0.01")); // unchanged
}

#[test]
fn ship_rocking_clamps_upper_sideways() {
    let mut r = RockingState::default();
    r.angle_sideways = SATURATION_PI4;
    r.vel_sideways = I16F16::lit("0.1");
    advance_ship_rocking(&mut r, true);
    assert_eq!(r.angle_sideways, SATURATION_PI4);
}

#[test]
fn ship_rocking_clamps_lower_both() {
    let mut r = RockingState::default();
    r.angle_forwards = -SATURATION_PI4 + I16F16::lit("0.001");
    r.vel_forwards = I16F16::lit("-0.01");
    advance_ship_rocking(&mut r, true);
    assert_eq!(r.angle_forwards, -SATURATION_PI4);
}

#[test]
fn ship_rocking_no_clamp_when_type_doesnt_support() {
    let mut r = RockingState::default();
    r.angle_sideways = SATURATION_PI4 + I16F16::lit("0.5");
    r.vel_sideways = I16F16::lit("0.01");
    advance_ship_rocking(&mut r, false);
    // No clamp applied; angle drifts past +π/4 + 0.5 + 0.01.
    assert!(r.angle_sideways > SATURATION_PI4);
}
```

**Step 3: Verify + commit**

Run: `cargo test --lib ship_rocking`
Expected: 4 PASS.

Commit:
```
sim/rocking: ship-rocking advance (no damping)

Used by EMP wobble + naval continuous rocking. Integrates velocity into
angle each tick without decay; one-sided clamps at ±π/4.
```

---

### Task 8b: Wide-amplitude self-destruct detection [L30]

**Why:** Parity-complete port of `RockingUpdate`'s end-of-function wide-amplitude
callback at 0x0070BC23. When `|angle_sideways| > π` OR `|angle_forwards| > π`, the
unit takes lethal self-damage with `Rules.c4_warhead` and force-kill. Almost never
fires in retail (constants are tuned to keep angles within ±π/4) but a faithful port
needs it for: (a) sustained EMP on a type without ship-rock clamp, (b) modded warheads
with extreme impulses, (c) external angle writes. Omitting it would diverge from
gamemd in those edge cases.

**Files:**
- Modify: `src/sim/rocking/rocking_system.rs`
- Modify: `src/sim/rocking/rocking_tests.rs`
- Modify: `src/sim/components.rs` (extend `RockingState` if a "needs self-destruct"
  flag is the chosen handoff pattern — see Step 1)

**Step 1: Pick the handoff pattern**

`rocking_system::tick` runs inside the sim main loop; the damage-apply call lives in
sim/combat (currently stubbed). Three options for how the detection flows to a
damage application:

- **Option A (chosen — direct hook):** `rocking_system::tick` takes a closure or
  trait object that applies self-damage. When combat-side damage lands, that closure
  invokes the real damage path. Until then, the closure is a no-op or logs.
- **Option B:** `RockingState` gains a `self_destruct_pending: bool` flag. Combat-side
  reads it next tick and applies damage. Simpler but introduces a 1-tick delay vs
  gamemd's same-tick kill.
- **Option C:** Tick function returns a list of `(entity_id, SelfDestructEvent)` that
  the caller (`World::advance_tick`) drains into the damage system.

Option A matches gamemd's same-tick semantics. Until combat lands, the hook is a
test-observable no-op. Use Option A.

```rust
// src/sim/rocking/self_destruct.rs (new file)

use crate::sim::components::RockingState;
use crate::sim::game_entity::GameEntity;
use fixed::types::I16F16;

/// Trigger threshold for the wide-amplitude self-destruct (rad).
/// Matches gamemd's ±π constants at 0x007F4E5C (−π) and 0x007F4E60 (+π).
pub const WIDE_AMPLITUDE_THRESHOLD: I16F16 = I16F16::lit("3.141593");

/// Callback invoked when a rocking entity's body angle exceeds ±π.
/// Implementations should apply `damage = entity.type.max_hp` with the
/// Ruleset's c4_warhead, source_house=None, source_object=None, force_kill=true.
/// Mirrors gamemd's RockingUpdate end-of-function call to TechnoClass::ReceiveDamage
/// (BODY_ROCKING_GHIDRA_REPORT.md §3.4).
pub trait SelfDestructHook {
    fn fire(&mut self, entity: &mut GameEntity);
}

/// No-op hook for tests and for the period before combat-side damage lands.
pub struct NoopSelfDestruct;
impl SelfDestructHook for NoopSelfDestruct {
    fn fire(&mut self, _entity: &mut GameEntity) {}
}

/// Inspect an entity's rocking state; if either angle exceeds ±π, invoke `hook`.
/// Caller is responsible for clearing the rocking state if the entity survives
/// the hook (gamemd doesn't — the kill is presumed to terminate the entity).
pub fn check_and_fire(entity: &mut GameEntity, hook: &mut dyn SelfDestructHook) {
    let Some(rocking) = entity.rocking.as_ref() else { return };
    if rocking.angle_sideways.abs() > WIDE_AMPLITUDE_THRESHOLD
        || rocking.angle_forwards.abs() > WIDE_AMPLITUDE_THRESHOLD
    {
        hook.fire(entity);
    }
}
```

Wire `check_and_fire` into `rocking_system::tick` AFTER the per-axis advance (so the
post-tick angles are evaluated, matching gamemd's end-of-function placement).

```rust
pub fn tick(world: &mut World, hook: &mut dyn SelfDestructHook) {
    for entity in world.entities.values_mut() {
        // (… existing per-axis advance / slope-transition tracking …)

        check_and_fire(entity, hook);
        if entity.health == 0 {
            continue;  // entity died — skip remaining post-tick work for this id
        }
    }
}
```

(In the no-combat period, pass `&mut NoopSelfDestruct` from `World::advance_tick`.
When combat lands, swap in a real implementation that calls the damage system.)

**Step 2: Tests**

```rust
use crate::sim::components::RockingState;
use crate::sim::rocking::self_destruct::{check_and_fire, SelfDestructHook,
    WIDE_AMPLITUDE_THRESHOLD};

struct CountingHook { fired: usize }
impl SelfDestructHook for CountingHook {
    fn fire(&mut self, _entity: &mut GameEntity) { self.fired += 1; }
}

#[test]
fn self_destruct_fires_when_sideways_exceeds_pi() {
    let mut e = test_entity_with_rocking();
    e.rocking.as_mut().unwrap().angle_sideways = WIDE_AMPLITUDE_THRESHOLD + I16F16::lit("0.01");
    let mut hook = CountingHook { fired: 0 };
    check_and_fire(&mut e, &mut hook);
    assert_eq!(hook.fired, 1);
}

#[test]
fn self_destruct_fires_when_forwards_exceeds_pi_negative() {
    let mut e = test_entity_with_rocking();
    e.rocking.as_mut().unwrap().angle_forwards = -WIDE_AMPLITUDE_THRESHOLD - I16F16::lit("0.01");
    let mut hook = CountingHook { fired: 0 };
    check_and_fire(&mut e, &mut hook);
    assert_eq!(hook.fired, 1);
}

#[test]
fn self_destruct_does_not_fire_within_envelope() {
    let mut e = test_entity_with_rocking();
    // Even at the saturation cap (±π/4), should not fire.
    e.rocking.as_mut().unwrap().angle_sideways = I16F16::lit("0.78");
    e.rocking.as_mut().unwrap().angle_forwards = I16F16::lit("-0.78");
    let mut hook = CountingHook { fired: 0 };
    check_and_fire(&mut e, &mut hook);
    assert_eq!(hook.fired, 0);
}

#[test]
fn self_destruct_skipped_for_entities_without_rocking() {
    let mut e = GameEntity::default();  // no rocking component
    let mut hook = CountingHook { fired: 0 };
    check_and_fire(&mut e, &mut hook);
    assert_eq!(hook.fired, 0);
}
```

**Step 3: Verify + commit**

Run: `cargo test --lib self_destruct`
Expected: 4 PASS.

Commit:
```
sim/rocking: wide-amplitude self-destruct detection (L30)

When a rocking entity's body angle exceeds ±π (180°), invoke a
SelfDestructHook trait object. Mirrors gamemd's RockingUpdate end-of-function
call to TechnoClass::ReceiveDamage with Rules.c4_warhead, max_hp damage,
force_kill=true. Hook is wired with NoopSelfDestruct until combat-side
damage lands; full path validates against gamemd's behavior on
EMP-without-ship-rock-clamp and stacked-impulse-on-moving-vehicle cases.
```

**Combat-side hook (DEFERRED — Phase F):** When warhead detonation lands in
sim/combat (Task 19), add a concrete `SelfDestructHook` implementation that:
1. Looks up `entity.type_handle.max_hp()` for the damage value (gamemd uses
   TypeClass+0xA0; verify in our TechnoType struct that this corresponds to
   `strength`/`max_hp`).
2. Resolves `Ruleset.c4_warhead: Option<String>` to a `&WarheadType` via the
   warhead vector.
3. Calls the damage system with `(target, damage, warhead, source_house=None,
   source_object=None, force_kill=true)`.
4. Updates `entity.health = 0` if the damage system doesn't (defensively — the
   force_kill flag should make this redundant, but RA2 has weird armor-vs-warhead
   tables and we don't want a divergence).

---

### Task 9: Implement slope-transition tracking in `tick()`

**Why:** The slope-transition state machine. Detects cell-slope changes, starts the 3-tick counter, counts down each tick. Render-time SLERP reads this.

**Files:**
- Modify: `src/sim/rocking/rocking_system.rs`
- Modify: `src/sim/rocking/rocking_tests.rs`

**Step 1: Add slope-transition helper**

```rust
/// Update slope-transition state for one entity.
///
/// If the entity's current cell slope_type differs from the tracked curr_slope,
/// start a 3-tick transition (prev = old curr, curr = new, ticks = 3). Else
/// decrement ticks_remaining (saturating at 0).
///
/// Matches gamemd's DriveLocomotionClass+0x18..+0x28 slope-transition state
/// (VXL_DRAW_MATRIX_GHIDRA_REPORT.md §12).
pub(crate) fn update_slope_transition(rocking: &mut RockingState, cell_slope: u8) {
    if cell_slope != rocking.curr_slope {
        rocking.prev_slope = rocking.curr_slope;
        rocking.curr_slope = cell_slope;
        rocking.transition_ticks_remaining = SLOPE_TRANSITION_TICKS;
    } else if rocking.transition_ticks_remaining > 0 {
        rocking.transition_ticks_remaining -= 1;
    }
}
```

**Step 2: Tests**

```rust
#[test]
fn slope_change_starts_three_tick_transition() {
    let mut r = RockingState::default();
    r.curr_slope = 0;
    update_slope_transition(&mut r, 5);
    assert_eq!(r.prev_slope, 0);
    assert_eq!(r.curr_slope, 5);
    assert_eq!(r.transition_ticks_remaining, 3);
}

#[test]
fn slope_unchanged_decrements_counter() {
    let mut r = RockingState::default();
    r.curr_slope = 5;
    r.transition_ticks_remaining = 3;
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 2);
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 1);
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 0);
}

#[test]
fn slope_counter_saturates_at_zero() {
    let mut r = RockingState::default();
    r.curr_slope = 5;
    r.transition_ticks_remaining = 0;
    update_slope_transition(&mut r, 5);
    assert_eq!(r.transition_ticks_remaining, 0);
}

#[test]
fn slope_change_mid_transition_resets_to_three() {
    let mut r = RockingState::default();
    r.curr_slope = 5;
    r.transition_ticks_remaining = 1;
    update_slope_transition(&mut r, 7);
    assert_eq!(r.prev_slope, 5);
    assert_eq!(r.curr_slope, 7);
    assert_eq!(r.transition_ticks_remaining, 3);
}
```

**Step 3: Verify + commit**

Run: `cargo test --lib slope_transition`
Expected: 4 PASS.

Commit:
```
sim/rocking: slope-transition tracking

3-tick counter, restarts on slope change mid-transition. Renderer reads
prev_slope/curr_slope/ticks_remaining to SLERP between slope matrices.
```

---

### Task 10: Implement `apply_rocker_impulse`

**Why:** Port of `TechnoClass::ApplyRocker` (FUN_0070B280). Computes direction-aware velocity components from an impulse source, capped at per-axis 0.05 rad/tick.

**Files:**
- Modify: `src/sim/rocking/impulse.rs`
- Modify: `src/sim/rocking/rocking_tests.rs`

**Pattern:** Pure function. Vec inputs in sim coordinates.

**Step 1: Implement**

```rust
// src/sim/rocking/impulse.rs

use fixed::types::I16F16;

use crate::sim::components::RockingState;
use crate::sim::rocking::rocking_system::{IMPULSE_VEL_CAP, FORCE_SATURATION};

/// Apply a rocker impulse to a unit. Computes a direction-aware velocity
/// pair from source-to-target vector and applies it (with per-axis 0.05
/// cap).
///
/// Port of TechnoClass::ApplyRocker (FUN_0070B280). Two gamemd simplifications
/// the design accepts for Phase A (both documented as deferred parity drifts
/// in the ledger):
///   - L31: distance attenuation `(0.04 − dist × 2.5e-5)` — small drift across
///     a 3×3-cell radius (~24% reduction at corner cells).
///   - L32: rate-timer-derived jitter rotating the direction by a small
///     per-tick angle — desyncs army-of-identical-vehicles rocking.
/// Both can be added as a follow-up without breaking the API.
///
/// `force`: pre-saturated [0, 4.0] force magnitude from the warhead-side
/// computation (Apply_area_damage or DirectRocker).
/// `weight`: target unit's Weight (TypeClass+0x370 in gamemd; INI `Weight=`).
/// Heavier units rock less per equivalent force. Default 2.0; retail range
/// 0.5–5. The L12c divisor — DO NOT drop, observable parity drift.
/// `dx`, `dy`: target_pos - source_pos in sim units (any scale; only the
/// direction matters).
pub fn apply_rocker_impulse(
    rocking: &mut RockingState,
    force: I16F16,
    weight: I16F16,
    dx: I16F16,
    dy: I16F16,
) {
    // Defensive: clamp force to known range. Source-side already saturates
    // at 4.0 but we re-clamp here to catch any wiring bugs.
    let force = if force > FORCE_SATURATION { FORCE_SATURATION } else if force < I16F16::ZERO { I16F16::ZERO } else { force };

    // Defensive: protect against Weight=0 (malformed INI). Gamemd would
    // divide by zero and produce NaN/inf, but our fixed-point would panic.
    // Treat zero/negative as default 2.0.
    let weight = if weight <= I16F16::ZERO { I16F16::lit("2.0") } else { weight };

    // Compute horizontal distance for normalization.
    let dist_sq = dx * dx + dy * dy;
    if dist_sq <= I16F16::lit("0.0000000004") {
        // Source effectively at target — no direction; abort.
        return;
    }
    // sqrt approximation (Newton-Raphson 3 iters works well for I16F16 in [0, 100^2]).
    let dist = sqrt_approx(dist_sq);
    let nx = dx / dist;
    let ny = dy / dist;

    // L12c: scale force by 1/weight. In gamemd: `force_scaled = (0.04 − dist
    // × 2.5e-5) × force / Weight`, then clamped to 0.05. We approximate the
    // distance factor as a constant 0.04 (L31 deferred), giving:
    //     force_scaled = 0.04 × force / weight   (then clamp at 0.05).
    let mut force_scaled = I16F16::lit("0.04") * force / weight;
    if force_scaled > IMPULSE_VEL_CAP { force_scaled = IMPULSE_VEL_CAP; }
    // Too-weak gate: gamemd's 0.01 floor at 0x007F4E34 — below this the
    // impulse is dropped entirely. Prevents a Weight=5 unit getting a
    // visible-but-tiny twitch from every glancing hit.
    if force_scaled < I16F16::lit("0.01") { return; }

    // L12b: gamemd halves the forwards component when ApplyRocker's
    // `no_dampen_flag` arg is false (multiplies by _DAT_007E5168 = 0.5f at
    // 0x70B54F). Both visible call sites (Apply_area_damage's Rocker loop
    // and Detonate's DirectRocker path) call ApplyRocker with the
    // no_dampen_flag stack slot defaulting to 0, so the halving applies in
    // practice in retail. Sideways is NOT halved — the asymmetry is in
    // gamemd by design.
    let vel_fwd = ny * force_scaled * I16F16::lit("0.5");
    let vel_side = -nx * force_scaled;

    // Additive — multiple hits in same tick stack (matches gamemd).
    rocking.vel_forwards = (rocking.vel_forwards + vel_fwd).clamp(-IMPULSE_VEL_CAP, IMPULSE_VEL_CAP);
    rocking.vel_sideways = (rocking.vel_sideways + vel_side).clamp(-IMPULSE_VEL_CAP, IMPULSE_VEL_CAP);
}

/// Newton-Raphson square-root for I16F16. Good for values in [0, 10000].
fn sqrt_approx(x: I16F16) -> I16F16 {
    if x <= I16F16::ZERO {
        return I16F16::ZERO;
    }
    let mut s = x;
    for _ in 0..6 {
        s = (s + x / s) / I16F16::from_num(2);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_approx_known_values() {
        let r = sqrt_approx(I16F16::from_num(4));
        assert!((r - I16F16::from_num(2)).abs() < I16F16::lit("0.001"));
        let r = sqrt_approx(I16F16::from_num(100));
        assert!((r - I16F16::from_num(10)).abs() < I16F16::lit("0.001"));
    }
}
```

**Step 2: Integration tests**

In `rocking_tests.rs`:

```rust
use crate::sim::rocking::impulse::apply_rocker_impulse;

const DEFAULT_WEIGHT: I16F16 = I16F16::lit("2.0");

#[test]
fn impulse_caps_at_005_per_axis() {
    let mut r = RockingState::default();
    apply_rocker_impulse(&mut r, I16F16::from_num(100), DEFAULT_WEIGHT, I16F16::ONE, I16F16::ZERO);
    assert!(r.vel_sideways.abs() <= IMPULSE_VEL_CAP);
    assert!(r.vel_forwards.abs() <= IMPULSE_VEL_CAP);
}

#[test]
fn impulse_direction_from_x_axis_writes_sideways() {
    let mut r = RockingState::default();
    // Strong impulse from +X direction. Weight=2, so force_scaled = 0.04 × 4.0 / 2 = 0.08,
    // clamped to 0.05. nx = 1, so vel_side = -0.05.
    apply_rocker_impulse(&mut r, I16F16::lit("4.0"), DEFAULT_WEIGHT, I16F16::ONE, I16F16::ZERO);
    assert!(r.vel_sideways < I16F16::ZERO);
    assert_eq!(r.vel_forwards, I16F16::ZERO);
}

#[test]
fn impulse_zero_distance_does_nothing() {
    let mut r = RockingState::default();
    apply_rocker_impulse(&mut r, I16F16::ONE, DEFAULT_WEIGHT, I16F16::ZERO, I16F16::ZERO);
    assert_eq!(r.vel_sideways, I16F16::ZERO);
    assert_eq!(r.vel_forwards, I16F16::ZERO);
}

#[test]
fn impulse_stacks_additively() {
    let mut r = RockingState::default();
    // Use a force large enough that 0.04 × force / 2 > 0.01 too-weak gate.
    // 0.04 × 1.0 / 2 = 0.02 → passes the gate.
    apply_rocker_impulse(&mut r, I16F16::lit("1.0"), DEFAULT_WEIGHT, I16F16::ONE, I16F16::ZERO);
    let v1 = r.vel_sideways;
    apply_rocker_impulse(&mut r, I16F16::lit("1.0"), DEFAULT_WEIGHT, I16F16::ONE, I16F16::ZERO);
    let v2 = r.vel_sideways;
    assert!(v2.abs() > v1.abs() || v2.abs() == IMPULSE_VEL_CAP);
}

#[test]
fn impulse_too_weak_dropped_by_floor_gate() {
    let mut r = RockingState::default();
    // force=0.1, weight=2 → force_scaled = 0.04 × 0.1 / 2 = 0.002 < 0.01 too-weak gate.
    apply_rocker_impulse(&mut r, I16F16::lit("0.1"), DEFAULT_WEIGHT, I16F16::ONE, I16F16::ZERO);
    assert_eq!(r.vel_sideways, I16F16::ZERO);
    assert_eq!(r.vel_forwards, I16F16::ZERO);
}

#[test]
fn impulse_heavier_unit_rocks_less_per_equal_force() {
    // L12c: a Weight=5 (Apocalypse-class) unit should receive ~2.5× less rocking
    // than a Weight=2 (default-class) unit for the same impulse magnitude.
    // Use a mid-range force so neither hits the 0.05 cap.
    let force = I16F16::lit("1.5");
    let mut light = RockingState::default();
    apply_rocker_impulse(&mut light, force, I16F16::lit("2.0"), I16F16::ONE, I16F16::ZERO);
    let mut heavy = RockingState::default();
    apply_rocker_impulse(&mut heavy, force, I16F16::lit("5.0"), I16F16::ONE, I16F16::ZERO);

    // light.vel_side = -0.04 × 1.5 / 2.0 = -0.03
    // heavy.vel_side = -0.04 × 1.5 / 5.0 = -0.012
    // Ratio ≈ 2.5×, matching the Weight ratio.
    let ratio = light.vel_sideways.abs() / heavy.vel_sideways.abs();
    assert!(ratio > I16F16::lit("2.4") && ratio < I16F16::lit("2.6"),
            "expected ~2.5× ratio (light/heavy), got {:?}", ratio);
}

#[test]
fn impulse_zero_weight_falls_back_to_default() {
    // Defensive: a malformed INI section with Weight=0 should not panic
    // or produce inf — should fall back to default 2.0.
    let mut r = RockingState::default();
    apply_rocker_impulse(&mut r, I16F16::lit("4.0"), I16F16::ZERO, I16F16::ONE, I16F16::ZERO);
    // Same as Weight=2 case.
    let mut reference = RockingState::default();
    apply_rocker_impulse(&mut reference, I16F16::lit("4.0"), DEFAULT_WEIGHT, I16F16::ONE, I16F16::ZERO);
    assert_eq!(r.vel_sideways, reference.vel_sideways);
}
```

**Step 3: Verify + commit**

Run: `cargo test --lib impulse`
Expected: 7 PASS (5 original + impulse_too_weak_dropped_by_floor_gate +
impulse_heavier_unit_rocks_less_per_equal_force + impulse_zero_weight_falls_back_to_default).

Commit:
```
sim/rocking: rocker impulse application (port of ApplyRocker)

Direction-aware impulse with per-unit Weight divisor [L12c], 0.04 base
force coefficient, 0.01 too-weak gate, 0.05 saturation cap, and 0.5×
forwards halving [L12b]. Multiple impulses in the same tick stack
additively before the cap is enforced. Heavier units (Weight=5
Apocalypse) rock ~2.5× less than default Weight=2 vehicles for the same
force — verified by impulse_heavier_unit_rocks_less_per_equal_force.

Distance attenuation (L31) and rate-timer jitter (L32) intentionally
omitted; documented as deferred parity drifts in the design doc.
```

---

### Phase C — Sim Integration (Tasks 11-12)

### Task 11: Wire `rocking_system::tick` into `World::advance_tick`

**Why:** Connects the per-tick advance to the sim main loop. Inserts at the documented standard order point (after movement, before aircraft missions).

**Files:**
- Modify: `src/sim/rocking/rocking_system.rs` — replace stub with real implementation
- Modify: [src/sim/world/mod.rs](../../src/sim/world/mod.rs) — call site insertion

**Step 1: Replace the `tick` stub with real implementation**

In `rocking_system.rs`:

```rust
use crate::map::resolved_terrain::ResolvedMap;
use crate::rules::Ruleset;
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrain;

/// Advance every entity's RockingState by one sim tick.
///
/// Order per entity:
/// 1. Read cell.slope_type at current position; update slope-transition.
/// 2. If is_ship_rocking: integrate without damping (advance_ship_rocking).
/// 3. Else: spring-damper on each axis (advance_axis).
/// 4. Wide-amplitude self-destruct check [L30] — if |angle| > π, fire hook.
///
/// Aircraft skip slope tilting (forced slope_type = 0 per L23).
pub fn tick(
    entities: &mut EntityStore,
    terrain: &ResolvedTerrain,
    rules: &Ruleset,
    self_destruct_hook: &mut dyn crate::sim::rocking::self_destruct::SelfDestructHook,
) {
    let keys: Vec<u64> = entities.keys_sorted();
    for &id in &keys {
        let Some(entity) = entities.get_mut(id) else { continue };
        if entity.rocking.is_none() { continue }

        // Read whole-entity properties BEFORE taking the &mut on rocking,
        // so the borrow checker doesn't complain. These helpers take `&GameEntity`.
        let raw_slope = terrain
            .cell(entity.position.rx, entity.position.ry)
            .map(|c| c.slope_type)
            .unwrap_or(0);
        let cell_slope = if entity.category == EntityCategory::Aircraft { 0 } else { raw_slope.min(20) };
        let is_moving = entity_is_moving(entity);
        let supports_ship_rock = entity_type_supports_ship_rocking(entity);

        // 1–3. Now mutate the rocking state (the &mut borrow is scoped to this block).
        {
            let rocking = entity.rocking.as_mut().unwrap();
            update_slope_transition(rocking, cell_slope);

            if rocking.is_ship_rocking {
                advance_ship_rocking(rocking, supports_ship_rock);
            } else {
                // L8 forwards override (±π/10 when vehicle is mid-crush of a building) is
                // DEFERRED — building-crushing isn't implemented yet, so the gate never fires.
                // Use ±π/4 for both axes uniformly. See plan §"Deferred to Implementation".
                let forward_cap = SATURATION_PI4;
                advance_axis(&mut rocking.angle_sideways, &mut rocking.vel_sideways,
                             SATURATION_PI4, is_moving, rules.fallback_coefficient);
                advance_axis(&mut rocking.angle_forwards, &mut rocking.vel_forwards,
                             forward_cap, is_moving, rules.fallback_coefficient);
            }
        }

        // 4. Wide-amplitude self-destruct check [L30]. Mirrors gamemd's
        // end-of-RockingUpdate ReceiveDamage call at 0x0070BC23.
        crate::sim::rocking::self_destruct::check_and_fire(entity, self_destruct_hook);
    }
}

/// Does the entity currently have a movement path? Used to choose between base
/// decay rate and fallback-scaled decay.
fn entity_is_moving(entity: &crate::sim::game_entity::GameEntity) -> bool {
    entity.movement_target.is_some()
}

/// Working assumption per design doc §"Open Questions" #1: vehicles and ships
/// ship-rock; infantry, aircraft, and buildings don't. Revisit when RE confirms.
fn entity_type_supports_ship_rocking(entity: &crate::sim::game_entity::GameEntity) -> bool {
    matches!(entity.category, EntityCategory::Unit)
}
```

**Notes for the implementer:**
- `entity.category` (an `EntityCategory` enum at [game_entity.rs:68](../../src/sim/game_entity.rs#L68)) replaces the `is_aircraft()` / `is_infantry()` / `is_deployed_building()` methods that were assumed during planning — none exist as methods.
- `entity.movement_target.is_some()` replaces the assumed `entity.velocity.magnitude_sq()` — there's no `velocity` field on `GameEntity`.
- `ResolvedTerrain` (not `ResolvedMap`) is the live type name. The world field is `self.resolved_terrain: Option<ResolvedTerrain>`. Wrap the tick call in `if let Some(terrain) = self.resolved_terrain.as_ref()` per the existing `tick_aircraft_missions` pattern at [world/mod.rs:1083](../../src/sim/world/mod.rs#L1083).
- The L8 forwards-override is deferred — `SATURATION_PI10` is still defined in Task 6 (it costs nothing) but no longer wired here. Re-wire when building-crushing lands.

**Step 2: Insert call into `World::advance_tick`**

In [src/sim/world/mod.rs](../../src/sim/world/mod.rs), locate the tick-ordering block (around line 1031-1169). Insert after the movement phase (~line 1071) and before aircraft missions (~line 1081):

```rust
// Phase 1.5: rocking system (after movement, before aircraft missions).
// Updates body-rocking spring-damper + slope-transition state, then runs
// the wide-amplitude self-destruct check [L30] for any entity whose body
// tipped past ±π this tick.
// Gated on terrain + rules being available — matches the existing
// tick_aircraft_missions pattern (world/mod.rs:1083).
if let Some(rules) = rules {
    if let Some(terrain) = self.resolved_terrain.as_ref() {
        // NoopSelfDestruct until combat-side damage lands (Task 19).
        // When combat lands, swap in a real hook that calls the damage system.
        let mut hook = crate::sim::rocking::self_destruct::NoopSelfDestruct;
        crate::sim::rocking::tick(&mut self.entities, terrain, rules, &mut hook);
    }
}
```

**Step 3: Verify the build + tick ordering**

Run: `cargo build`
Expected: PASS.

Run: `cargo test --lib`
Expected: existing tests still pass + rocking tests still pass.

**Step 4: Commit**

```
sim/world: wire rocking::tick into advance_tick

Inserts after movement (so slope_type reads see latest position) and before
aircraft missions. Aircraft skip slope tilt (L23); infantry skip ship rocking
(per working assumption documented in design doc open question 1).
```

---

### Task 12: Integration test — decay convergence + determinism

**Why:** End-to-end test that the sim layer behaves correctly: an entity hit by a synthetic impulse decays over ~30-50 ticks, and two seeded runs produce identical state hashes.

**Files:**
- Modify: `src/sim/rocking/rocking_tests.rs`

**Step 1: Write the integration test**

```rust
#[test]
fn integration_impulse_decays_over_time() {
    use crate::sim::rocking::impulse::apply_rocker_impulse;

    let mut sim = make_test_simulation_with_one_vehicle();
    {
        let entity = sim.entities.values_mut().next().unwrap();
        let r = entity.rocking.as_mut().expect("test vehicle should have rocking");
        apply_rocker_impulse(r, I16F16::lit("1.0"), I16F16::ONE, I16F16::ZERO);
        assert!(r.vel_sideways.abs() > I16F16::ZERO, "impulse should set velocity");
    }
    
    // Run 60 ticks (1 second at 60 fps); rocking should have decayed.
    for _ in 0..60 {
        sim.advance_tick();
    }

    let entity = sim.entities.values().next().unwrap();
    let r = entity.rocking.as_ref().unwrap();
    assert!(r.is_neutral(), "rocking should have decayed to neutral after 60 ticks");
}

#[test]
fn integration_determinism_same_seed_same_hash() {
    let mut a = make_test_simulation_with_one_vehicle();
    let mut b = make_test_simulation_with_one_vehicle();
    
    // Apply identical impulse to both.
    {
        let entity_a = a.entities.values_mut().next().unwrap();
        let r_a = entity_a.rocking.as_mut().unwrap();
        crate::sim::rocking::impulse::apply_rocker_impulse(r_a, I16F16::lit("0.5"), I16F16::ONE, I16F16::ZERO);
        
        let entity_b = b.entities.values_mut().next().unwrap();
        let r_b = entity_b.rocking.as_mut().unwrap();
        crate::sim::rocking::impulse::apply_rocker_impulse(r_b, I16F16::lit("0.5"), I16F16::ONE, I16F16::ZERO);
    }
    
    for tick in 0..200 {
        a.advance_tick();
        b.advance_tick();
        assert_eq!(a.state_hash(), b.state_hash(),
                   "diverged at tick {}", tick);
    }
}

fn make_test_simulation_with_one_vehicle() -> crate::sim::Simulation {
    // Implementer: build minimal test sim with a single vehicle entity that has
    // rocking: Some(RockingState::default()). Mirror existing test helpers in
    // src/sim/*_tests.rs (e.g. deploy_tests.rs has a make_rules_with_deploy pattern).
    todo!("write per existing helper pattern")
}
```

**Note:** The implementer must wire up `make_test_simulation_with_one_vehicle` from an existing helper pattern. If no clean helper exists, write a minimal one that creates a `Simulation` with default rules + one vehicle entity at position (5, 5) on a flat-slope cell. The `todo!` is acceptable as a structural placeholder in this plan, but MUST be replaced with real code before the task is considered complete.

**Step 2: Verify + commit**

Run: `cargo test --lib rocking::rocking_tests::integration`
Expected: 2 PASS.

Commit:
```
sim/rocking: integration tests for decay + determinism

End-to-end test that a 1.0-force impulse decays to neutral over 60 ticks
and that two seeded runs produce identical state hashes for 200 ticks.
```

---

### Phase D — Renderer Foundation (Tasks 13-15)

### Task 13: Extend `VxlRenderParams` + add `compute_slope_shear_translation`

**Why:** Plumbs the new body-matrix inputs through the renderer's parameter struct without changing behavior yet. Adds the L24 shear-offset math (closes G7).

**Files:**
- Modify: [src/render/vxl_raster.rs](../../src/render/vxl_raster.rs)

**Step 1: Extend the struct**

```rust
// In VxlRenderParams (existing struct):
/// Optional rocking angles for this render call (sideways, forwards in rad).
/// None ⇒ no rocking; renderer skips the rocking matrix entirely (L1
/// renderer-side simple path).
pub rocking_angles: Option<(f32, f32)>,

/// Optional pre-computed slope matrix (already SLERPed if mid-transition).
/// None ⇒ use slope_type field directly via compute_slope_rotation.
pub slope_blend_matrix: Option<Mat4>,
```

Update `Default for VxlRenderParams`:

```rust
rocking_angles: None,
slope_blend_matrix: None,
```

**Step 2: Add slope shear translation**

```rust
/// Compute the slope-tilt translation shear that keeps the rotated body sitting
/// visually on the slope surface. Matches gamemd's combined_Z + partial_X/Y +
/// remainder_X/Y formula at VXL_DRAW_MATRIX_GHIDRA_REPORT.md §15.2-§15.4.
///
/// `tilt_mag_x` = vxl.size_y / 2 in voxel units.
/// `tilt_mag_y` = vxl.size_x / 2 in voxel units.
/// Returns (tx, ty, tz) translation offset to add to the tilted body's position.
pub fn compute_slope_shear_translation(
    slope_type: u8,
    tilt_mag_x: f32,
    tilt_mag_y: f32,
) -> Vec3 {
    if slope_type == 0 || slope_type >= 17 {
        return Vec3::ZERO;
    }
    let (compass_rad, tilt_rad) = compass_and_tilt_for_slope(slope_type);
    let cos_pitch = tilt_rad.cos();
    let sin_pitch = tilt_rad.sin();
    let cos_roll = compass_rad.cos();
    let sin_roll = compass_rad.sin();

    let combined_z = (cos_roll.abs() * tilt_mag_x + cos_pitch.abs() * tilt_mag_y).round();
    let partial_y = (sin_pitch * tilt_mag_x).round();
    let remainder_y = (tilt_mag_x - partial_y).round();
    let remainder_y_signed = if tilt_rad < 0.0 { -remainder_y } else { remainder_y };
    let partial_x = (sin_roll * tilt_mag_y).round();
    let remainder_x = (tilt_mag_y - partial_x).round();
    let remainder_x_signed = if compass_rad >= 0.0 { -remainder_x } else { remainder_x };

    Vec3::new(remainder_x_signed, remainder_y_signed, combined_z)
}

fn compass_and_tilt_for_slope(slope_type: u8) -> (f32, f32) {
    // Lifted from existing compute_slope_rotation match arms; share the table
    // if convenient.
    match slope_type {
        // (per existing slope_type → (compass_rad, tilt_rad) mapping)
        _ => (0.0, 0.0),
    }
}
```

Implementer: hoist the existing slope_type → (compass, tilt) lookup table into a shared helper. Currently the table lives inline in `compute_slope_rotation`; extract to `compass_and_tilt_for_slope` and call from both places.

**Step 3: Unit tests for shear**

```rust
#[test]
fn shear_zero_for_flat_slope() {
    let s = compute_slope_shear_translation(0, 32.0, 32.0);
    assert_eq!(s, Vec3::ZERO);
}

#[test]
fn shear_zero_for_unpopulated_slope_17_20() {
    let s = compute_slope_shear_translation(17, 32.0, 32.0);
    assert_eq!(s, Vec3::ZERO);
}

#[test]
fn shear_for_slope_1_west_edge_nonzero() {
    let s = compute_slope_shear_translation(1, 32.0, 32.0);
    // West edge: compass=270°. Expected: nonzero translation.
    assert!(s.length() > 0.0);
}
```

**Step 4: Verify + commit**

Run: `cargo test --lib vxl_raster`
Expected: existing tests pass + new tests pass.

Commit:
```
render/vxl_raster: extend VxlRenderParams + add slope shear translation

Closes G7 from disparity scan. Renderer can now accept rocking angles and
pre-SLERPed slope matrix. Shear translation port of gamemd's
combined_Z/partial_X/Y/remainder_X/Y formula.
```

---

### Task 14: Add `blinn_phong_pages_from_body_matrix` to vxl_normals

**Why:** Body-matrix-aware lighting LUT. Closes G4 by letting the LUT follow the full body orientation, not just facing.

**Files:**
- Modify: [src/render/vxl_normals.rs](../../src/render/vxl_normals.rs)

**Pattern:** Mirror the existing `blinn_phong_pages` function exactly. Only the light-direction transformation changes.

**Step 1: Add the new function**

```rust
/// Compute the Blinn-Phong VPL page LUT using the full body matrix.
///
/// Closes the G4 disparity from 2026-05-11-disparity-scan-voxel.md: lighting
/// now follows slope tilt and body rocking, not just facing rotation. Matches
/// gamemd's per-Render-call LUT recompute (VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md
/// §6.4): dot(local_normal, body_matrix⁻¹ × world_light).
pub fn blinn_phong_pages_from_body_matrix(
    normals_mode: u8,
    body_matrix: &Mat4,
) -> [u8; 256] {
    let world_light = Vec3::new(YR_LIGHT_BASE[0], YR_LIGHT_BASE[1], YR_LIGHT_BASE[2]);
    let body_local_light = body_matrix
        .inverse()
        .transform_vector3(world_light)
        .normalize();
    let viewer = Vec3::Z;
    let halfway = (body_local_light + viewer).normalize();
    const SPECULAR_STRENGTH: f32 = 3.0;

    let mut result = [0u8; 256];
    let table: &[[f32; 3]] = match normals_mode {
        4 => &RA2_NORMALS,
        2 => &TS_NORMALS,
        _ => &RA2_NORMALS,
    };
    let normal_count = table.len().min(256);
    for i in 0..normal_count {
        let normal = Vec3::new(table[i][0], table[i][1], table[i][2]);
        let diffuse = normal.dot(body_local_light).max(0.0);
        let halfway_dot = normal.dot(halfway);
        let specular = if halfway_dot > 0.0 {
            halfway_dot / (SPECULAR_STRENGTH - halfway_dot * SPECULAR_STRENGTH + halfway_dot)
        } else {
            0.0
        };
        let brightness = diffuse + specular;
        let page = (brightness * 16.0).clamp(0.0, 255.0) as u8;
        result[i] = page;
    }
    if normal_count >= 254 { result[253] = 16; }
    if normal_count >= 255 { result[254] = 16; }
    if normal_count >= 256 { result[255] = 16; }
    result
}
```

**Step 2: Unit tests**

```rust
#[test]
fn body_matrix_lut_matches_facing_lut_for_pure_facing() {
    // A facing-only body matrix should produce a LUT close to (but not
    // identical to) blinn_phong_pages(4, facing). Tolerance covers
    // numerical differences in light-transform approach.
    let facing_rad = std::f32::consts::FRAC_PI_4;
    let body_mat = Mat4::from_rotation_z(facing_rad);
    let lut_a = blinn_phong_pages(4, facing_rad);
    let lut_b = blinn_phong_pages_from_body_matrix(4, &body_mat);
    
    let max_diff = lut_a.iter().zip(lut_b.iter())
        .map(|(a, b)| (*a as i32 - *b as i32).abs())
        .max().unwrap_or(0);
    assert!(max_diff <= 4, "max LUT diff = {}", max_diff);
}

#[test]
fn body_matrix_lut_differs_for_tilted_body() {
    let body_a = Mat4::IDENTITY;
    let body_b = Mat4::from_rotation_x(0.5);  // pitched 28.6°
    let lut_a = blinn_phong_pages_from_body_matrix(4, &body_a);
    let lut_b = blinn_phong_pages_from_body_matrix(4, &body_b);
    let differs = lut_a.iter().zip(lut_b.iter()).any(|(a, b)| a != b);
    assert!(differs, "LUT should differ between flat and pitched body");
}
```

**Step 3: Verify + commit**

Run: `cargo test --lib vxl_normals`
Expected: existing tests pass + 2 new pass.

Commit:
```
render/vxl_normals: body-matrix-aware Blinn-Phong LUT

Closes G4 from disparity scan. Lighting LUT now uses full body matrix
(facing × slope × rocking) for light transformation instead of facing-only.
```

---

### Task 15: Replace per-facing LUT call site in atlas pre-bake

**Why:** Wire the new body-matrix LUT into the atlas-bake path. Closes G4 for upright-on-slope units too (atlas-baked sprites now get correct lighting per slope variant).

**Files:**
- Modify: [src/render/vxl_raster.rs](../../src/render/vxl_raster.rs) — call site at `prepare_limb_data`

**Step 1: Replace LUT call**

In `prepare_limb_data` (around [src/render/vxl_raster.rs:322](../../src/render/vxl_raster.rs#L322)):

Current:
```rust
let vpl_pages: [u8; 256] = vxl_normals::blinn_phong_pages(limb.normals_mode, facing_rad);
```

Replace with:
```rust
// Use body-matrix-aware LUT (closes G4 — slope-aware lighting on atlas-baked sprites).
let body_mat_for_lut = rotate_to_world * slope_mat;
let vpl_pages: [u8; 256] = vxl_normals::blinn_phong_pages_from_body_matrix(
    limb.normals_mode, &body_mat_for_lut
);
```

(Adjust `body_mat_for_lut` to match the actual matrix composition in `prepare_limb_data` — should be the same composition used for transform_point3 on voxel positions, minus the section-local transform.)

**Step 2: Visual smoke test (manual)**

This task changes visual output. Recommend running the existing voxel render test (`tests/vxl_render.rs`) and visually inspecting the output PNGs to confirm lighting still looks correct on flat units (no regression) and now shifts appropriately on slope-baked variants.

Run: `cargo test --test vxl_render -- --ignored`
(Requires `RA2_DIR` env var per the test's documentation.)

If the output looks broken — body lit from wrong side, etc. — inspect the matrix composition. The most likely issue is that the LUT-input body matrix should NOT include the section-local scale/translate; only the world-orientation part.

**Step 3: Verify + commit**

Run: `cargo test --lib`
Expected: all passing (no visual smoke).

Commit:
```
render/atlas: use body-matrix LUT in atlas pre-bake

Closes G4 for upright-on-slope units. Atlas-baked sprites now have
slope-aware lighting; the per-facing LUT approximation is gone.
```

---

### Phase E — Renderer Real-Time Path (Tasks 16-18)

### Task 16: Add `vxl_compute::render_runtime` per-frame entry point

**Why:** Promote the GPU compute renderer from offline-batch-only to per-frame callable. Adds the scratch-texture pool needed for tilted-unit rendering.

**Files:**
- Modify: [src/render/vxl_compute.rs](../../src/render/vxl_compute.rs)

**Step 1: Add scratch texture pool**

Add to `VxlComputeRenderer` struct:

```rust
/// Pool of scratch textures for per-frame runtime rendering. Grows on demand.
scratch_textures: Vec<ScratchTexture>,
/// Index of next available scratch texture; reset at frame start.
scratch_next: usize,
```

Add `ScratchTexture` type and pool management:

```rust
struct ScratchTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl VxlComputeRenderer {
    /// Reset the scratch-texture pool index at the start of each frame.
    pub fn begin_frame(&mut self) {
        self.scratch_next = 0;
    }
}
```

**Step 2: Add `render_runtime` method**

```rust
/// Per-frame entry point — renders a tilted unit's voxel sprite into a
/// scratch GPU texture. Output is a wgpu::TextureView into a pool-owned
/// texture (valid until next begin_frame call).
///
/// This is the path taken when a unit's RockingState::is_neutral() is false.
pub fn render_runtime(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    vxl: &VxlFile,
    hva: Option<&HvaFile>,
    body_matrix: Mat4,
    vpl: &VplFile,
) -> RuntimeSprite {
    // 1. Use existing prepare_limb_data + compute_sprite_bounds path with the
    //    new VxlRenderParams (rocking_angles + slope_blend_matrix from caller).
    // 2. Allocate or reuse a scratch texture sized to compute_sprite_bounds.
    // 3. Dispatch splat + resolve compute passes (existing code).
    // 4. Return RuntimeSprite { texture_view, uv, size, offset }.
    
    // Implementer: follow the existing `render_sprite` method structure
    // (offline batch); the only changes are (a) skip CPU readback and (b)
    // output to scratch texture instead of staging buffer.
    todo!("implement per existing render_sprite pattern")
}

pub struct RuntimeSprite {
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
}
```

The `todo!()` is a structural placeholder — implementer must follow the existing `render_sprite` method's structure as a template (it does the GPU work; we just skip the readback and target a different texture).

**Step 3: Smoke test**

```rust
#[test]
#[ignore = "requires GPU device — run with --ignored"]
fn render_runtime_produces_nonempty_output() {
    let (device, queue) = create_test_gpu();
    let mut renderer = VxlComputeRenderer::new(&device);
    renderer.begin_frame();
    let vxl = make_test_vxl();
    let body_mat = Mat4::IDENTITY;
    let sprite = renderer.render_runtime(&device, &queue, &vxl, None, body_mat, &make_test_vpl());
    assert!(sprite.width > 0);
    assert!(sprite.height > 0);
}
```

**Step 4: Verify + commit**

Run: `cargo build`
Expected: PASS.

Commit:
```
render/vxl_compute: add render_runtime per-frame entry point

Adds scratch-texture pool + per-frame render method that outputs to a
GPU texture without CPU readback. Foundation for the real-time path used
by actively-tilting units.
```

---

### Task 17: Wire body matrix composition + branching at render hand-off

**Why:** The single branching point that routes neutral units to the atlas and tilting units to the real-time renderer.

**Files:**
- Modify: [src/app_instances/units.rs](../../src/app_instances/units.rs)

**Step 1: Add body matrix composition helper**

In a new helper module under `src/render/` or alongside `vxl_raster.rs`:

```rust
/// Compose the full body matrix for a tilted unit.
///
/// Order: world_facing × slope_blend × rocking_x × rocking_y. Matches gamemd's
/// VXL_DRAW_MATRIX §13-§15 composition.
pub fn compose_body_matrix(
    facing_rad: f32,
    rocking: &RockingState,
    cell_size: (f32, f32, f32),  // for shear translation magnitudes
) -> Mat4 {
    let facing_mat = Mat4::from_rotation_z(WORLD_YAW_OFFSET.to_radians() - facing_rad);

    // Slope (with optional SLERP).
    let slope_mat = if rocking.transition_ticks_remaining > 0 {
        let q_prev = slope_to_quat(rocking.prev_slope);
        let q_curr = slope_to_quat(rocking.curr_slope);
        let t = (SLOPE_TRANSITION_TICKS - rocking.transition_ticks_remaining) as f32
              / SLOPE_TRANSITION_TICKS as f32;
        Mat4::from_quat(q_prev.slerp(q_curr, t))
    } else {
        compute_slope_rotation(rocking.curr_slope)
    };

    // Slope shear translation (L24).
    let shear = compute_slope_shear_translation(
        rocking.curr_slope,
        cell_size.1 * 0.5,
        cell_size.0 * 0.5,
    );
    let slope_with_shear = slope_mat * Mat4::from_translation(shear);

    // Rocking (L1: only if either axis > 0.005 rad).
    let rocking_mat = {
        let side = rocking.angle_sideways.to_num::<f32>();
        let fwd = rocking.angle_forwards.to_num::<f32>();
        if side.abs() > 0.005 || fwd.abs() > 0.005 {
            Mat4::from_rotation_x(fwd) * Mat4::from_rotation_y(side)
        } else {
            Mat4::IDENTITY
        }
    };

    facing_mat * slope_with_shear * rocking_mat
}

fn slope_to_quat(slope_type: u8) -> Quat {
    // Convert slope_type → quaternion using same compass/tilt table.
    let (compass, tilt) = compass_and_tilt_for_slope(slope_type);
    Quat::from_rotation_z(compass) * Quat::from_rotation_x(tilt) * Quat::from_rotation_z(-compass)
}
```

**Step 2: Add branching at render hand-off**

In [src/app_instances/units.rs](../../src/app_instances/units.rs), find the existing unit-draw loop that calls `atlas.lookup(UnitSpriteKey { … })`. Add the branch:

```rust
let is_tilting = entity.rocking
    .as_ref()
    .map(|r| !r.is_neutral())
    .unwrap_or(false);

if is_tilting {
    let rocking = entity.rocking.as_ref().unwrap();
    let body_matrix = compose_body_matrix(facing_rad, rocking, cell_size_in_voxels(&entity));
    let sprite = realtime_renderer.render_runtime(
        device, queue, &vxl, hva.as_deref(), body_matrix, &vpl,
    );
    draw_runtime_sprite(sprite, screen_pos, alpha, …);
} else {
    // Existing atlas path — unchanged.
    let entry = atlas.lookup(UnitSpriteKey { type_id, facing, layer, frame, slope_type });
    draw_atlas_sprite(entry, screen_pos, …);
}
```

Implementer: `draw_runtime_sprite` follows the existing `draw_atlas_sprite` pattern but binds the scratch texture instead of the atlas. Add it alongside the existing draw helper.

**Step 3: Smoke test**

This is integration-level; covered by Task 22.

**Step 4: Commit**

```
render/units: branch between atlas and real-time for tilted units

Single branching point at the unit-draw site. Neutral units take the
unchanged atlas path; tilting units route to vxl_compute.render_runtime
with the full body matrix.
```

---

### Task 18: Verify atlas key surface unchanged

**Why:** Sanity-check that we didn't accidentally explode the atlas keys. Atlas continues to use the existing 5-tuple key.

**Files:**
- Read: [src/render/unit_atlas.rs](../../src/render/unit_atlas.rs)

**Step 1: Read `UnitSpriteKey` definition**

Open `unit_atlas.rs`. Locate `UnitSpriteKey` struct. Confirm fields are unchanged: `type_id, facing, layer, frame, slope_type`. No new fields.

**Step 2: Commit any unrelated cleanup if needed**

If during this verification step you notice anything off (e.g., stale references to removed signatures), fix and commit. Otherwise, no commit needed — this is verification.

---

### Phase F — Combat Integration Stubs (Task 19)

### Task 19: Stub combat-side impulse call sites

**Why:** When warhead detonation lands in sim/combat, the impulse-apply call must be in place. Stub now with TODO markers; full wiring happens when combat is ready.

**Files:**
- Modify: Wherever warhead detonation currently lives in sim/combat (TBD — implementer to locate)
- Or: create `src/sim/combat/rocker_dispatch.rs` as a stub-and-forward module

**Step 1: Locate warhead detonation site**

Run `grep -rn 'detonate\|Detonate\|warhead' src/sim/combat/ src/sim/projectile/` to find current detonation code. If detonation isn't implemented yet, document this as a deferred follow-up in the design doc; create a `rocker_dispatch.rs` stub:

```rust
//! Stub dispatcher for rocker impulses. Called by warhead detonation when
//! that system lands in sim/combat.
//!
//! Until then, this module is unreferenced — rocking machinery sits idle.

use fixed::types::I16F16;
use crate::sim::components::RockingState;
use crate::sim::rocking::impulse::apply_rocker_impulse;
use crate::rules::{Ruleset, WarheadType, ProjectileType};

/// Apply area-damage rocker impulses. Called by warhead detonation when
/// the warhead has Rocker=yes. Iterates targets in 3x3 cell radius.
///
/// For each target, call `apply_rocker_impulse(rocking, force, target.weight,
/// dx, dy)` where `weight` comes from the target's ObjectType.weight (L12c).
pub fn dispatch_area_rocker(
    /* targets: &mut [&mut GameEntity], */
    /* target_types: &[&ObjectType],   // for the Weight lookup */
    /* warhead: &WarheadType, */
    /* impact_pos: Vec3Fixed, */
    /* damage_accumulator: I16F16, */
) {
    // L11: force = accumulator × 0.01, saturate at 4.0, gate by force > 0.3 (_DAT_007E5138).
    // For each target:
    //   1. Compute direction (dx, dy) = target_pos - impact_pos.
    //   2. Look up target.type.weight (L12c per-unit divisor).
    //   3. Call apply_rocker_impulse(rocking, force, weight, dx, dy).
    todo!("wire when warhead detonation system is ready")
}

/// Apply direct-hit rocker impulse. Called by warhead detonation when
/// warhead has DirectRocker=yes AND target is a vehicle.
pub fn dispatch_direct_rocker(
    /* target: &mut GameEntity, */
    /* target_type: &ObjectType,     // for the Weight lookup */
    /* warhead: &WarheadType, */
    /* projectile: &ProjectileType, */
    /* rules: &Ruleset, */
    /* target_pos: Vec3Fixed, */
    /* source_pos: Vec3Fixed, */
) {
    // L12: force = (RockerScale × Damage >> 8) × DirectRockingCoefficient / 100.0.
    //              (>> 8 is the Q8.8 RockerScale unshift; 100.0 is Rules normalization, _DAT_0081AEF8.)
    // Saturate at 4.0 (_DAT_007E3CC8). Call apply_rocker_impulse(rocking, force,
    // target_type.weight, dx, dy) — the Weight divisor (L12c) is applied inside
    // apply_rocker_impulse, so DirectRocker callers just pass force pre-Weight.
    todo!("wire when warhead detonation system is ready")
}
```

**Step 2: Commit**

```
sim/combat: stub rocker impulse dispatchers

Placeholder for the two impulse paths (area-damage Rocker=yes, direct-hit
DirectRocker=yes vehicle-only). Wired into warhead detonation when that
system lands. Rocking machinery sits idle until then — no functional
regression.
```

---

### Phase G — Verification (Tasks 20-22)

### Task 20: Performance benchmark — 2000 tilting units

**Why:** Validates the 10-percent-of-20k tilting-units estimate. Surfaces the perf risk early.

**Files:**
- Create: `benches/voxel_rocking_perf.rs` (if benches/ exists) or a `tests/` integration benchmark

**Step 1: Write the benchmark**

```rust
//! Benchmark: 2000 tilting voxel units rendered per frame via vxl_compute.render_runtime.

use std::time::Instant;
// imports

#[test]
#[ignore = "perf benchmark — run with --ignored --nocapture"]
fn bench_2000_tilting_units() {
    let (device, queue) = create_gpu_device();
    let mut renderer = VxlComputeRenderer::new(&device);
    let vxl = load_test_vxl("htnk.vxl");  // typical tank, ~12K voxels per limb
    let vpl = load_test_vpl();

    let n = 2000;
    let start = Instant::now();
    renderer.begin_frame();
    for i in 0..n {
        let facing = (i as f32 / n as f32) * std::f32::consts::TAU;
        let body_mat = Mat4::from_rotation_z(facing) * Mat4::from_rotation_x(0.1);
        let _sprite = renderer.render_runtime(&device, &queue, &vxl, None, body_mat, &vpl);
    }
    queue.submit([]);  // flush
    device.poll(wgpu::Maintain::Wait);
    let elapsed = start.elapsed();
    println!("Rendered {} tilting units in {:?} ({:.2} ms/frame)", n, elapsed, elapsed.as_secs_f64() * 1000.0);
    // 16.6ms = 60fps budget. Hard fail if >50ms.
    assert!(elapsed.as_millis() < 50, "perf regression: {:.2}ms exceeds 50ms budget", elapsed.as_secs_f64() * 1000.0);
}
```

**Step 2: Run + record**

Run: `cargo test --release --test voxel_rocking_perf -- --ignored --nocapture`
Record the per-frame time in commit message.

**If it exceeds 16.6ms (60fps budget):** STOP. Surface to user and discuss batching strategy (single dispatch for all tilting units) or LRU cap. Don't proceed to Task 21.

**Step 3: Commit**

```
test: 2000-tilting-unit perf benchmark for vxl runtime renderer

Records baseline performance: <X> ms per frame for 2000 tilting voxel
units rendered via vxl_compute.render_runtime. Within 60fps budget /
exceeds 60fps budget and needs batching optimization.
```

---

### Task 21: Integration smoke test — slope crossing + decay

**Why:** End-to-end behavioral test that ties sim and renderer together. Confirms parity at the system level.

**Files:**
- Create: `tests/rocking_integration.rs`

**Step 1: Write the integration test**

```rust
//! Integration test: vehicle drives across slope boundary, takes impulse,
//! decays to neutral. Verifies sim + renderer hand-off works end-to-end.

#[test]
fn vehicle_crosses_slope_boundary_smoothly() {
    let mut sim = make_test_sim_with_vehicle_on_flat();
    
    // Step 1: vehicle at slope=0, no rocking → should use atlas path.
    let entity = sim.entities.values().next().unwrap();
    let r = entity.rocking.as_ref().unwrap();
    assert!(r.is_neutral(), "initial state should be neutral");
    
    // Step 2: move vehicle onto a slope-5 cell.
    move_vehicle_to_cell(&mut sim, /* slope=5 cell */);
    sim.advance_tick();
    
    let entity = sim.entities.values().next().unwrap();
    let r = entity.rocking.as_ref().unwrap();
    assert_eq!(r.curr_slope, 5);
    assert_eq!(r.prev_slope, 0);
    assert_eq!(r.transition_ticks_remaining, 3);
    
    // Step 3: tick 3 more times — transition completes.
    sim.advance_tick();
    sim.advance_tick();
    sim.advance_tick();
    
    let entity = sim.entities.values().next().unwrap();
    let r = entity.rocking.as_ref().unwrap();
    assert_eq!(r.transition_ticks_remaining, 0);
    // No rocking, only slope settled → back to atlas path.
    assert!(r.is_neutral());
}

#[test]
fn vehicle_takes_impulse_then_decays_to_neutral() {
    let mut sim = make_test_sim_with_vehicle_on_flat();
    
    // Apply impulse.
    {
        let entity = sim.entities.values_mut().next().unwrap();
        let r = entity.rocking.as_mut().unwrap();
        apply_rocker_impulse(r, I16F16::lit("1.0"), I16F16::ONE, I16F16::ZERO);
        assert!(!r.is_neutral());
    }
    
    // Run ticks until neutral.
    let mut ticks = 0;
    while ticks < 200 {
        sim.advance_tick();
        ticks += 1;
        let entity = sim.entities.values().next().unwrap();
        if entity.rocking.as_ref().unwrap().is_neutral() {
            break;
        }
    }
    assert!(ticks < 200, "rocking should decay to neutral within 200 ticks; took {}", ticks);
    println!("Rocking decayed in {} ticks", ticks);
}

fn make_test_sim_with_vehicle_on_flat() -> Simulation {
    todo!("write per existing helper pattern in src/sim/world/world_tests.rs")
}

fn move_vehicle_to_cell(sim: &mut Simulation, cell_pos: (i32, i32)) {
    todo!()
}
```

**Step 2: Verify + commit**

Run: `cargo test --test rocking_integration`
Expected: 2 PASS.

Commit:
```
test: rocking + slope-transition integration smoke

End-to-end test: vehicle crosses slope boundary (3-tick transition) and
takes impulse (decays to neutral within 200 ticks). Validates sim + atlas
+ realtime hand-off works.
```

---

### Task 22: Final sweep + state-hash compatibility note

**Why:** Last task. Run the full test suite, ensure clean build, document the replay/savefile breaking change in the user-facing changelog.

**Step 1: Run full suite**

```
cargo build
cargo test --lib
cargo test --tests
cargo test --release --test voxel_rocking_perf -- --ignored
cargo clippy --all-targets --all-features
```

All should pass.

**Step 2: Document breaking change**

In the repo's CHANGELOG.md (or equivalent), add:

```
## [Unreleased]
### Breaking
- Replay/savefile format: GameEntity gains an optional RockingState
  field; state hash now includes rocking velocity, angle, and slope
  transition state. Replays from prior versions will FAIL hash validation
  and must be re-recorded.

### Added
- Body rocking system: gamemd-accurate spring-damper on vehicles, ships,
  voxel-bodied buildings. Closes G1 from 2026-05-11 voxel disparity scan.
- Slope-tilt SLERP transition: 3-tick smooth interpolation when vehicles
  cross slope boundaries. Closes G3.
- Tilt-aware lighting: voxel LUT now uses full body matrix (facing × slope
  × rocking). Closes G4.
- Slope-tilt translation shear: body sits visually on slope after rotation.
  Closes G7.
- INI keys: Rocker=, DirectRocker= (per-warhead),
  RockerScale= (per-projectile), DirectRockingCoefficient=,
  FallBackCoefficient= (Rules [AudioVisual]).
```

If no changelog exists, add this to the design doc's "Hand-off" section as documentation.

**Step 3: Final commit**

```
docs: changelog entry for body rocking system

Documents the new feature, the four parity gaps closed, the new INI keys,
and the replay/savefile-format breaking change (state hash now includes
RockingState).
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-11-voxel-body-matrix-design.md](2026-05-11-voxel-body-matrix-design.md)
- **Disparity scan:** [docs/gap-scans/2026-05-11-disparity-scan-voxel.md](../gap-scans/2026-05-11-disparity-scan-voxel.md)
- **Ghidra reports (all HIGH confidence):**
  - [BODY_ROCKING_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BODY_ROCKING_GHIDRA_REPORT.md) — RockingUpdate algorithm + constants
  - [VXL_DRAW_MATRIX_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/VXL_DRAW_MATRIX_GHIDRA_REPORT.md) — body matrix composition, slope SLERP, tilt path
  - [VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md) — lighting LUT formula
  - [VOXEL_SLOPE_TILT_SYSTEM.md](../../../ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md) — slope-type table, aircraft skip
- **gamemd.exe addresses verified this session:**
  - TechnoClass::RockingUpdate @ 0x0070B570
  - TechnoClass::ApplyRocker @ 0x0070B280
  - WarheadTypeClass::Detonate @ 0x004690B0
  - Apply_area_damage @ 0x00489280
  - FootClass::ReceiveEMP @ 0x004DECF0
  - AI_Update RockingUpdate call site @ 0x006FA236
  - All 13 spring-damper constants (read from binary memory; full table in BODY_ROCKING_GHIDRA_REPORT.md §7)
- **INI keys:** rulesmd.ini:620-621 (DirectRockingCoefficient=1.5, FallBackCoefficient=0.1); plus per-warhead Rocker= flags surveyed in BODY_ROCKING_GHIDRA_REPORT.md §6.1
- **Repo patterns followed:**
  - Sim system tick signature: [src/sim/animation.rs:375](../../src/sim/animation.rs#L375) (`tick_animations`)
  - Optional component on GameEntity: [src/sim/game_entity.rs](../../src/sim/game_entity.rs)
  - Fixed-point types: [src/util/fixed_math.rs](../../src/util/fixed_math.rs)
  - INI parsing for `[AudioVisual]`: [src/rules/ruleset.rs:729-820](../../src/rules/ruleset.rs#L729)
  - State hash field-by-field: [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs)
- **Recent related commits (none invalidate this plan):**
  - 485d88f sim/miner: ChronoIn/OutSound (different system)
  - 4e9a63d rules: C4 flags (different system)
- **glam APIs used:** `Mat4::from_rotation_z`, `Mat4::from_quat`, `Quat::slerp`, `Mat4::inverse`, `Mat4::transform_vector3`
