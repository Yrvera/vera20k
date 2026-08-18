# Particles C4 — Fire BehavesLike Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Implement the third Tier-2 BehavesLike — Fire — in a per-BehavesLike file (`src/sim/particles/fire.rs`), mirroring the C2/C3 pattern. Land the parity-critical core (velocity-gated death, FinalDamageState gate, translucency thresholds, ground-rise cliff death) without crossing into other sim modules. Defer damage application (C6), orbital attached-object tracking, and wiring `move_fire` into the per-tick path.

**Architecture:** Fire is the third Tier-2 particle variant. The system AI lives in `src/sim/particles/fire.rs`; the existing `system_ai.rs` dispatcher calls into it for `ParticleSystemBehavesLike::Fire`. The per-tick movement helper `move_fire` is provided as a tested standalone function — it stays uncalled by `tick_system` until the per-tick path is wired (same deferred-wiring pattern smoke and gas use today). Fire's particle-level AI extends the smoke/gas template with three new gates (velocity-zero death, animation-state→translucency byte mapping, FinalDamageState clamp on damage-counter resets) and one new mutable field (`prev_delta`) that fire AI writes for `move_fire` to consume. Animation-state *auto-advance* matches the gas/smoke deferral — the binary's advance formula uses SHP frame count from the asset layer, which `sim/` can't reach. Tests pin `animation_state` manually to verify the threshold mapping; auto-advance lands when render integration provides a frame-count helper.

**Design Doc:** `docs/plans/2026-05-04-particle-system-rust-plan.md` (Task C4, lines ~1774-1836). This plan doc supersedes that section with the actual repo facts and the plan-mismatches caught during C2/C3 (`Simulation` path, `sim.tick`, no engine refs in code comments, per-BehavesLike file split, deferred damage application).

---

## Grounding Summary

- **Research docs:** `ra2-rust-game-docs/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` covers the full system. Fire-specific sections: §3.6 (Fire System AI), §3.8 fire (particle AI), §3.9 (Move Dispatch fire path), §10.13 (deep dive on jitter, ground detection, FinalDamageState gate). All claims tagged "verified-from-binary" in the report.
- **Repo pattern:** `src/sim/particles/smoke.rs` (C2) and `src/sim/particles/gas.rs` (C3) are the prior art. Both use a per-file layout: `tick_system` → `tick_particle` → `move_*` → private `make_particle` / `make_child` helpers + a struct-only `tests` module. Fire mirrors this exactly with three additions (velocity gate, translucency state machine, FinalDamageState gate).
- **INI source of truth:** `ini/rulesmd.ini` has one fire particle (`[FireStream]`, line 26054) and one fire system (`[FireStreamSys]`, line 26016). Key INI values for tests: `MaxEC=500, MaxDC=3, Damage=2, Velocity=28.0, Deacc=0.01, StartStateAI=1, EndStateAI=19, StateAIAdvance=6, Translucent50State=15, Translucent25State=10, FinalDamageState=14, DeleteOnStateLimit=yes, Normalized=yes`. All keys already parsed by `src/rules/particle_type.rs`.
- **Existing infrastructure:** `spawn_particle_with_insert` from B4 is ready ([src/sim/particles/spawn.rs:127](../../src/sim/particles/spawn.rs#L127)). Fire uses `insert_range=4` per the binary.
- **Unknown after grounding:** Exact RNG-call ordering inside fire AI (jitter is `Random() % 10 - 5` — single call, range -5..+4). Whether `prev_delta` should persist on `Particle` or be recomputed inline is settled in this plan: persist as field for clean cross-call (AI writes, `move_fire` reads).

## Key Technical Decisions

- **Per-BehavesLike file** — Fire lands in `src/sim/particles/fire.rs`, not in `particle_ai.rs` + `movement.rs` as the original spec suggested. **Confidence:** high. **Source:** repo pattern, smoke.rs and gas.rs both follow this.
- **`prev_delta: [SimFixed; 3]` added to `Particle`** — Fire AI computes it (jitter * direction); `move_fire` consumes it. Keeping it on `Particle` rather than threading through return values simplifies eventual wiring (when the per-tick path picks up `move_fire`, it just calls fns sequentially with no plumbing). **Confidence:** high. **Source:** binary §10.13.1 stores into particle at +0x100/+0x104/+0x108.
- **`move_fire` takes `(old_ground, new_ground)` as i32 args, not a map handle** — The cliff-death rule is testable without map wiring. The "compute ground heights from the map and call move_fire" wiring is the explicitly deferred per-tick-path step. **Confidence:** high. **Source:** mirror of smoke's `move_smoke_with_wind` test-friendly form.
- **`move_fire` advances coords unconditionally, even on cliff death** — The binary's `Move_Dispatch` (§10.2.1) sets `marked_for_deletion` on rising terrain *and* still calls `SetCoords(new_pos)` afterward. Player-visible: a fire particle dying on a cliff renders one frame at the cliff cell, then gets pruned. Skipping the advance (returning early after the kill) leaves the particle one frame at its pre-move position — a subtle but parity-bar-relevant divergence. **Confidence:** high. **Source:** §10.2.1 lines 1785-1806 of the report — `SetCoords(new_pos)` is the unconditional last line.
- **Animation-state auto-advance deferred** — The binary's advance formula is `frame_ticks % ((total_frames % 2 + 1) + StateAIAdvance) == 0` where `total_frames` comes from `GetImageFrameCount()` on the SHP asset. That data lives in `assets/`, and `sim/` can't depend on it. Gas (C3) and smoke (C2) already defer auto-advance for the same reason — particles render at their `start_state_ai` until something else sets the state. C4 adds the threshold-byte mapping (when state ≥ Translucent25State, set 0x32; when ≥ Translucent50State, overwrite to 0x19) but leaves the auto-advance for a follow-up. Tests pin `animation_state` directly. **Confidence:** high. **Source:** §8.4 lines 977-989 of the report; matches existing gas/smoke `tick_particle` shape.
- **Orbital attached-object tracking deferred** — System AI for C4 follows particles + spawns at `SpawnFrames` cadence. The orbital math (distance, angle from `RateTimer`, cos/sin offsets, target-moved bonus spawn) requires entity coordinate access (a deferred Position→IVec3-leptons helper) and `RateTimer` (not yet in sim). **Confidence:** high. **Source:** matches gas's deferred entity-following decision in C3.
- **Damage application deferred** — Counter bookkeeping (decrement, FinalDamageState gate, conditional reset) is implemented and tested. Actual damage call to cell occupants lands in C6. **Confidence:** high. **Source:** explicit user direction; matches gas C3 approach.
- **Direction jitter applied to a stored `direction` vector via `prev_delta = jitter * direction`** — The binary keeps `direction` stable and writes a fresh jittered delta each frame. **Confidence:** medium. **Source:** §10.13.1 verified-from-binary; jitter range is `Random() % 10 - 5` → -5..+4 → factor 0.95..1.04. Verify exact RNG semantics during execution; if `next_range_u32(10) - 5` gives the wrong distribution, adjust.

## Open Questions

### Resolved During Planning

- **"Where does `prev_delta` live?"** — On `Particle` as a new `[SimFixed; 3]` field. Initialized to `[SIM_ZERO; 3]` in every spawn site (`spawn.rs`, `smoke.rs::make_particle`, `gas.rs::make_particle`, `fire.rs::make_particle`).
- **"Does this need to be in `state_hash`?"** — No. `state_hash` already covers the observable result (coords, animation_state, translucency, marked_for_deletion). `prev_delta` is intermediate scratch; deterministic RNG ensures cross-run identity without explicit hashing.
- **"How does fire system AI handle a dead attached object?"** — Deferred. The full §3.6 attached-object-alive check + mark_for_deletion needs entity-alive helpers (same defer as gas/smoke entity-following). For C4, the system survives until its `Lifetime` countdown expires.
- **"Smoke/gas tick_particle takes `(p, pt)`. Fire needs RNG for jitter — does that ripple?"** — No. Fire's `tick_particle` takes `(p, pt, rng)`; smoke/gas signatures stay unchanged. Each per-BehavesLike file owns its own helper signatures.

### Deferred to Implementation

- **Exact magnitude of the `0.01` jitter constant in fixed-point** — `_DAT_007efb40 = 0.01` per the report. Use `SimFixed::from_num(0.01)` literal; if downstream parity testing reveals drift, switch to `SimFixed::lit("0.01")`.
- **Whether `direction` needs to be normalized when the particle spawns** — `[FireStream]` has `Normalized=yes`. The spawner that creates the system should set particle direction from a normalized vector (target − source). For C4 (system-driven cadence-spawn from `sys.coords`, no target), `direction` stays at `[SIM_ZERO; 3]` from spawn — fire AI will compute `prev_delta = 0` and `move_fire` will treat it as no-motion. Tests pin `direction` explicitly.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/particles/mod.rs` | Add `pub mod fire;`. Add `prev_delta: [SimFixed; 3]` field to `Particle`. |
| Modify | `src/sim/particles/spawn.rs` | Initialize `prev_delta: [SIM_ZERO; 3]` in `spawn_particle`. |
| Modify | `src/sim/particles/smoke.rs` | Initialize `prev_delta: [SIM_ZERO; 3]` in `make_particle`. |
| Modify | `src/sim/particles/gas.rs` | Initialize `prev_delta: [SIM_ZERO; 3]` in `make_particle`. |
| Create | `src/sim/particles/fire.rs` | Fire system AI + particle AI + `move_fire` helper + tests. |
| Modify | `src/sim/particles/system_ai.rs` | Replace `tick_fire` no-op with `super::fire::tick_system(sys, sim, rules)`. Update module-level doc. |

## Interface Changes

- **`Particle` struct** — adds `pub prev_delta: [SimFixed; 3]`. Internal to `sim::particles`. No external consumers (render layer reads `coords`, `animation_state`, `translucency` only).
- **No public API changes.** `Simulation::spawn_particle_system` signature unchanged. The `tick_particle_systems` entry point unchanged.

## Sim Checklist

- [x] All math uses `fixed`-point — `SimFixed` everywhere; no f32/f64 in fire AI or move_fire
- [x] No new state must enter `state_hash` (observable result already covered by coords + animation_state + translucency + marked_for_deletion)
- [x] No dependencies on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/` — verified by import list (only `crate::rules`, `crate::sim::*`, `crate::util::fixed_math`, `glam::IVec3`)
- [x] Tick ordering unaffected — fire system AI invoked from `tick_particle_systems` (already wired in phase 5.5)
- [x] BTreeMap iteration order preserved — `ParticleSystemStore` unchanged

## Risk Areas

- **Adding `prev_delta` to `Particle`** — Particle is constructed in 4 places (`spawn.rs`, `smoke.rs::make_particle`, `gas.rs::make_particle`, `fire.rs::make_particle`). Forgetting one site will fail compile (struct-init shorthand catches missing fields). Risk: low.
- **`tick_particle` signature divergence** — Fire's takes `(p, pt, rng)`; smoke/gas take `(p, pt)`. Each per-BehavesLike module owns its own helper, so no cross-module breakage. Risk: low.
- **FinalDamageState clamp semantics** — When `animation_state > pt.final_damage_state`, the binary stops *resetting* the counter, even though it still decrements. Test must distinguish "counter dropped past zero and stayed there" from "counter wrapped back to MaxDC". Tests verify both cases by pinning `animation_state` directly (auto-advance is deferred). Risk: medium.
- **`move_fire` cliff-death coord update** — The binary sets `marked_for_deletion` AND advances coords on cliff death (`SetCoords(new_pos)` runs unconditionally). Test asserts both flags AND post-move coords. The dying particle renders one frame at the cliff cell, then gets pruned next tick. Risk: low.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 3 (AI) | Velocity-zero death | Flame streams visibly truncate at the end of each weapon shot. A bug here makes the trail linger or never finish. | Unit test: set `velocity = SIM_ZERO`, tick once, assert `marked_for_deletion`. |
| Task 3 (AI) | FinalDamageState clamp on damage-counter reset | Past `FinalDamageState` (default 14 for `[FireStream]`), fire visibly fades but does no damage. Without the clamp, fire damages all the way to `EndStateAI=19`. | Unit test: tick to `animation_state > final_damage_state`, drive `damage_counter` to zero, assert it does NOT reset to MaxDC. |
| Task 3 (AI) | Translucency-byte threshold mapping | `[FireStream]` should fade to a lighter alpha (0x32) when state ≥ 10 and to a deeper alpha (0x19) when state ≥ 15. The mapping is what we land here — auto-advance of `animation_state` is deferred (binary formula needs SHP frame count). | Unit test: pin `animation_state` at threshold values, assert `translucency == 0x32` then `0x19`. |
| Task 4 (move) | Cliff death (terrain rises → mark dead, coords still advance) | Visible every time a flame stream hits a wall or ridge: the dying particle renders one frame at the cliff cell, then disappears. Skipping the coord advance (returning early after the kill) leaves the sprite one frame behind, which the player can see on slow-moving fire trails. | Unit test: call `move_fire` with `old_ground=0, new_ground=10`, assert `hit_ground == true` AND `marked_for_deletion == true` AND `coords` advanced to the cliff cell. |
| Task 5 (system) | `SpawnParticleWithInsert` ordering variety | Visible as a non-monotonic flame stream: particles in the front of the stream aren't strictly older than particles in the back. Without the insert-shuffle, the stream looks too uniform. | Integration: tick the system many ticks, assert vector ordering doesn't match strict-creation-time ordering (insertion happens within last `insert_range=4` slots). |

---

## Tasks

### Task 1: Add `prev_delta` field to `Particle` and update existing initializers

**Why:** Fire AI writes a per-tick velocity delta that `move_fire` consumes. Storing it on `Particle` means the eventual per-tick wiring is just "call AI then call move_fire" with no extra plumbing. Smoke and gas don't use it (they touch `coords` directly via wind tables) but must initialize it so the struct-init shorthand stays sound.

**Files:**
- Modify: [src/sim/particles/mod.rs](../../src/sim/particles/mod.rs) — Particle struct
- Modify: [src/sim/particles/spawn.rs:100-120](../../src/sim/particles/spawn.rs#L100-L120) — `spawn_particle`
- Modify: [src/sim/particles/smoke.rs:177-198](../../src/sim/particles/smoke.rs#L177-L198) — `make_particle`
- Modify: [src/sim/particles/gas.rs:182-205](../../src/sim/particles/gas.rs#L182-L205) — `make_particle`

**Pattern:** Mirrors existing scratch fields (`drift_x/y/z`, `current_color`, `color_index`, `color_accumulator`).

**Step 1: Add the field to `Particle`**

In `src/sim/particles/mod.rs`, in the `Particle` struct (after `drift_z: i32,`), insert:

```rust
    /// Fire-only scratch: per-tick velocity delta computed by fire AI and
    /// consumed by `move_fire` (jitter * direction). Zero for smoke/gas.
    pub prev_delta: [SimFixed; 3],
```

**Step 2: Initialize in `spawn.rs`**

In `spawn_particle` (around line 119, after `color_accumulator:`), add:

```rust
        prev_delta: [SimFixed::from_num(0); 3],
```

**Step 3: Initialize in `smoke.rs::make_particle`**

In `make_particle` (around line 197, after `color_accumulator:`), add:

```rust
        prev_delta: [SIM_ZERO; 3],
```

**Step 4: Initialize in `gas.rs::make_particle`**

In `make_particle` (around line 205, after `color_accumulator:`), add:

```rust
        prev_delta: [SIM_ZERO; 3],
```

**Step 5: Verify**

Run: `cargo build`
Expected: clean build, no warnings about missing struct fields.

Run: `cargo test --lib particles::`
Expected: all 25 existing particle tests pass (the new field is unused on smoke/gas paths).

**Step 6: Do NOT commit yet** — this lands as part of the single C4 commit at the end.

---

### Task 2: Create `src/sim/particles/fire.rs` skeleton with module-level doc + `make_particle` helper

**Why:** Establish the file structure that subsequent tasks fill in. The doc comment up front pins what's deferred so the reviewer knows the scope without reading the plan. `make_particle` is the lowest-level helper and has no dependencies on the AI logic — landing it first lets the AI tasks just call it.

**Files:**
- Create: [src/sim/particles/fire.rs](../../src/sim/particles/fire.rs)

**Pattern:** Copy the structure of [src/sim/particles/gas.rs:1-32](../../src/sim/particles/gas.rs#L1-L32) (header + imports) and lines 173-203 (`make_particle` helper).

**Step 1: Write the file**

```rust
//! Fire `BehavesLike` system + particle AI.
//!
//! Per-tick fire logic for both the system (cadence-driven spawning via
//! `spawn_particle_with_insert` for ordering variety) and individual
//! particles (velocity-gated death, direction jitter, animation-state
//! translucency thresholds, decel, damage-counter bookkeeping with
//! FinalDamageState clamp).
//!
//! Fire differs from smoke and gas in three parity-critical ways:
//!   - A particle dies the instant its velocity drops to zero, not just
//!     on lifetime expiry — the flame trail truncates cleanly when a
//!     weapon stops firing.
//!   - The damage counter only resets when `animation_state` is at or
//!     below `FinalDamageState`. Past that, the counter still decrements
//!     but stops looping back to MaxDC, so faded fire stops dealing damage.
//!   - Translucency is animation-state-driven: at `Translucent50State`
//!     the byte flips to 0x19, at `Translucent25State` to 0x32.
//!
//! `move_fire` applies the AI-written `prev_delta` and kills the particle
//! on rising terrain (cliff death) — the canonical "flame hits a wall"
//! visual.
//!
//! ## Deferred (tracked for follow-up tasks)
//! - Damage application to cell occupants (Task C6). Counter bookkeeping
//!   + FinalDamageState gate are in place; the apply call is the only
//!   missing piece. Distance scaling (`distance/10`) and bridge-layer
//!   awareness land with C6.
//! - Wiring `move_fire` into the per-tick path — needs ground-height
//!   queries against the map. Today `move_fire` is a tested helper
//!   waiting for a caller.
//! - Animation-state auto-advance — the binary's formula is
//!   `frame_ticks % ((total_frames % 2 + 1) + StateAIAdvance) == 0`,
//!   where `total_frames` comes from `GetImageFrameCount()` on the SHP
//!   asset. `sim/` can't reach the asset layer; gas/smoke defer this
//!   too. The threshold-byte mapping below works correctly once
//!   something external advances `animation_state`.
//! - Orbital attached-object tracking in `tick_system` — needs entity
//!   coordinate access + `RateTimer`. Today the system spawns from its
//!   fixed `sys.coords` at the SpawnFrames cadence.
//! - Attached-object alive check (mark system for deletion when its
//!   target dies). Needs an entity-alive helper.
//! - Spawn-on-target-moved bonus (3-tick fallback when target moves).

use super::spawn::spawn_particle_with_insert;
use super::{Particle, ParticleSystem};
use crate::rules::particle_type::{ParticleType, ParticleTypeId};
use crate::rules::ruleset::RuleSet;
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;
use crate::util::fixed_math::{SIM_ZERO, SimFixed};
use glam::IVec3;

/// `SpawnParticleWithInsert` range used by fire systems — particles inserted
/// within the last 4 slots create the non-monotonic flame trail.
const FIRE_INSERT_RANGE: usize = 4;

/// Animation-state translucency byte at the 50% threshold.
const TRANSLUCENT_50_BYTE: u8 = 0x19;
/// Animation-state translucency byte at the 25% threshold.
const TRANSLUCENT_25_BYTE: u8 = 0x32;

fn make_particle(
    type_id: ParticleTypeId,
    coords: IVec3,
    spawn_origin: IVec3,
    pt: &ParticleType,
    rng: &mut SimRng,
) -> Particle {
    let base = (pt.max_ec as u32).max(1);
    let lifetime_extra = rng.next_range_u32(base) as i16;
    let lifetime_remaining = (pt.max_ec as i16).saturating_add(lifetime_extra);
    Particle {
        type_id,
        coords,
        previous_coords: spawn_origin,
        origin: coords,
        direction: [SIM_ZERO; 3],
        velocity: pt.velocity,
        lifetime_remaining,
        damage_counter: pt.max_dc as i16,
        state_ai_advance: pt.state_ai_advance,
        animation_state: pt.start_state_ai,
        translucency: pt.translucency,
        hit_ground: false,
        marked_for_deletion: false,
        drift_x: 0,
        drift_y: 0,
        drift_z: 0,
        current_color: [0; 3],
        color_index: 0,
        color_accumulator: SimFixed::from_num(0),
        prev_delta: [SIM_ZERO; 3],
    }
}
```

**Step 2: Verify**

Run: `cargo build`
Expected: clean build (the file is unused but compiles).

---

### Task 3: Implement fire particle AI (`tick_particle`)

**Why:** The three parity-critical particle-level rules that *can* be implemented inside `sim/` — velocity-zero death, FinalDamageState gate, jitter-driven `prev_delta` — are the heart of fire's distinct behavior versus smoke/gas. Animation-state auto-advance is deferred (binary formula uses SHP frame count from the asset layer); the threshold-byte mapping is included so it fires correctly once an external caller sets `animation_state`.

**Files:**
- Modify: [src/sim/particles/fire.rs](../../src/sim/particles/fire.rs) — append `tick_particle`

**Pattern:** Extends the smoke/gas `tick_particle` template ([src/sim/particles/gas.rs:112-128](../../src/sim/particles/gas.rs#L112-L128)) with three additions: velocity-zero gate, jitter into `prev_delta`, and FinalDamageState clamp on damage-counter reset. The `apply_translucency_thresholds` helper does NOT advance state — it only maps the current state value to the right translucency byte.

**Step 1: Append the function to `fire.rs`** (after `make_particle`, before any test module):

```rust
/// Per-tick AI for one fire particle.
///
/// Order matches the binary: velocity gate → jitter prev_delta →
/// translucency-threshold byte mapping → decel → damage counter
/// (gated by FinalDamageState).
///
/// Animation-state auto-advance is deferred: the binary's formula reads
/// SHP frame count from the asset layer, which `sim/` can't reach.
/// `animation_state` stays at `start_state_ai` unless something external
/// sets it; the threshold mapping below still fires correctly once it does.
///
/// Damage application to cell occupants is deferred to Task C6; this
/// function only does the counter bookkeeping.
pub(super) fn tick_particle(p: &mut Particle, pt: &ParticleType, rng: &mut SimRng) {
    // Velocity-zero death: fire dies the instant its momentum runs out.
    if p.velocity <= SIM_ZERO {
        p.marked_for_deletion = true;
        return;
    }

    // Direction jitter: factor in [0.95, 1.04]. Compute fresh each tick;
    // direction itself stays stable.
    let raw = rng.next_range_u32(10) as i32 - 5;
    let jitter = SimFixed::from_num(1) + SimFixed::from_num(raw) * SimFixed::from_num(0.01);
    p.prev_delta = [
        p.direction[0] * jitter,
        p.direction[1] * jitter,
        p.direction[2] * jitter,
    ];

    // Lifetime decrement (matches smoke/gas).
    p.lifetime_remaining = p.lifetime_remaining.saturating_sub(1);
    if p.lifetime_remaining <= 0 {
        p.marked_for_deletion = true;
    }

    apply_translucency_thresholds(p, pt);

    // Decel — fire decel is per-frame regardless of velocity sign because
    // the velocity-zero gate above already handled the zero case.
    p.velocity = (p.velocity - pt.deacc).max(SIM_ZERO);

    // Damage countdown — only resets when the particle is still in its
    // damaging window (animation_state ≤ final_damage_state). Past that,
    // counter drops below zero and stays there: damage stops permanently.
    p.damage_counter = p.damage_counter.saturating_sub(1);
    if p.damage_counter <= 0 && p.animation_state <= pt.final_damage_state {
        // C6 hooks the damage-to-cell-occupants iteration here.
        p.damage_counter = pt.max_dc as i16;
    }
}

/// Map the current `animation_state` to the correct translucency byte.
///
/// 25State triggers first (lower state value); 50State overwrites once
/// its higher state value is reached. For [FireStream]: state ≥ 10
/// → 0x32, then state ≥ 15 → 0x19. Both checks fire every tick — they're
/// idempotent (no state crossing required), so an externally-driven state
/// jump still produces the correct fade. `0xFF` is the "never" sentinel.
fn apply_translucency_thresholds(p: &mut Particle, pt: &ParticleType) {
    if pt.translucent_25_state != 0xFF && p.animation_state >= pt.translucent_25_state {
        p.translucency = TRANSLUCENT_25_BYTE;
    }
    if pt.translucent_50_state != 0xFF && p.animation_state >= pt.translucent_50_state {
        p.translucency = TRANSLUCENT_50_BYTE;
    }
}
```

**Step 2: Verify**

Run: `cargo build`
Expected: clean build, no unused-fn warnings (the helper is private; tick_particle is `pub(super)` for the system AI to call).

---

### Task 4: Implement `move_fire` movement helper

**Why:** Cliff death (rising terrain → marked_for_deletion) is the parity-critical ground-collision behavior. By taking ground heights as i32 args, the helper stays testable today while the actual map query is the deferred wiring step.

**Files:**
- Modify: [src/sim/particles/fire.rs](../../src/sim/particles/fire.rs) — append `move_fire`

**Pattern:** Mirror the smoke `move_smoke` / `move_smoke_with_wind` two-form pattern ([src/sim/particles/smoke.rs:131-145](../../src/sim/particles/smoke.rs#L131-L145)). For fire, only the explicit-args form is needed (no global wind direction to fall back on); the deferred wiring step will be a one-liner that queries the map and calls this helper.

**Step 1: Append to `fire.rs`** (after the animation helper):

```rust
/// Apply fire movement to one particle.
///
/// Adds the AI-written `prev_delta` to coords, then kills the particle
/// if the new position lands on rising terrain (cliff death). Caller
/// supplies pre-queried ground heights; the actual map query is the
/// deferred per-tick wiring.
///
/// On cliff death the particle still advances to `new_coords` — that
/// matches the binary's `Move_Dispatch` which calls `SetCoords(new_pos)`
/// unconditionally after marking the particle dead. The dying particle
/// renders one frame at the cliff cell, then gets pruned next tick.
///
/// `move_fire` is a no-op when the particle's velocity has already
/// dropped to zero (fire AI's velocity gate would have marked it for
/// deletion this tick anyway, but `move_fire` may be called standalone
/// in tests or in the eventual wiring sequence).
///
/// Bridge-layer interaction is deferred to C6 — fire particles pass
/// through bridges in the binary too (no bridge check in fire move).
pub(super) fn move_fire(
    p: &mut Particle,
    old_ground: i32,
    new_ground: i32,
) {
    if p.velocity <= SIM_ZERO {
        return;
    }
    let dx = p.prev_delta[0].to_num::<i32>();
    let dy = p.prev_delta[1].to_num::<i32>();
    let dz = p.prev_delta[2].to_num::<i32>();
    let new_coords = p.coords + IVec3::new(dx, dy, dz);
    if old_ground < new_ground {
        // Cliff death — terrain rises, particle hits ground.
        p.hit_ground = true;
        p.marked_for_deletion = true;
        // Coords still advance — the binary's SetCoords runs after the kill.
    }
    p.previous_coords = p.coords;
    p.coords = new_coords;
}
```

**Step 2: Verify**

Run: `cargo build`
Expected: clean build.

---

### Task 5: Implement fire system AI (`tick_system`)

**Why:** The system-level driver: tick all particles, prune dead ones, spawn new ones at SpawnFrames cadence using `spawn_particle_with_insert`. This is what the `system_ai.rs` dispatcher calls.

**Files:**
- Modify: [src/sim/particles/fire.rs](../../src/sim/particles/fire.rs) — append `tick_system`

**Pattern:** Follow the smoke/gas system AI structure ([src/sim/particles/gas.rs:34-110](../../src/sim/particles/gas.rs#L34-L110)) but replace the direct push with `spawn_particle_with_insert` and skip the NextParticle chaining (fire particles don't chain — `[FireStream]` has no `NextParticle=`). Skip the `Slowdown`/`SpawnCutoff` accumulator logic (fire systems use `SpawnFrames` directly without accumulator).

**Step 1: Append to `fire.rs`**:

```rust
/// Advance one fire `ParticleSystem` by one tick.
pub(super) fn tick_system(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    let pst = rules.particle_system_type(sys.type_id);
    let cap = pst.particle_cap as usize;
    let tick = sim.tick;

    // Phase 1 — tick existing particles.
    for p in &mut sys.particles {
        let pt = rules.particle_type(p.type_id);
        tick_particle(p, pt, &mut sim.rng);
    }

    // Phase 2 — prune dead particles. Fire has no NextParticle chaining
    // (the [FireStream] type has none and no fire chain exists in vanilla).
    sys.particles.retain(|p| !p.marked_for_deletion);

    // Phase 3 — spawn at SpawnFrames cadence via the insert-shuffle helper.
    // Orbital attached-object tracking + the target-moved 3-tick fallback
    // are deferred (see module doc).
    if !sys.done_spawning && pst.spawns {
        let frames = (pst.spawn_frames as u64).max(1);
        if tick % frames == 0 && sys.particles.len() < cap {
            let _ = spawn_particle_with_insert(
                sys,
                sys.coords,
                sys.coords,
                FIRE_INSERT_RANGE,
                rules,
                &mut sim.rng,
            );
        }
    }
}
```

**Step 2: Verify**

Run: `cargo build`
Expected: clean build, no unused warnings.

---

### Task 6: Wire dispatcher and module declaration

**Why:** Until the dispatcher calls `fire::tick_system`, the new code is dead. This task is the one-line plumbing that activates it.

**Files:**
- Modify: [src/sim/particles/mod.rs](../../src/sim/particles/mod.rs) — add `pub mod fire;`
- Modify: [src/sim/particles/system_ai.rs:60-62](../../src/sim/particles/system_ai.rs#L60-L62) — replace `tick_fire` no-op
- Modify: [src/sim/particles/system_ai.rs:5-6](../../src/sim/particles/system_ai.rs#L5-L6) — update module-level doc

**Pattern:** Identical to the C3 wiring step (`super::gas::tick_system(sys, sim, rules)` lives at [src/sim/particles/system_ai.rs:56-58](../../src/sim/particles/system_ai.rs#L56-L58)).

**Step 1: Add `fire` to the module list**

In `src/sim/particles/mod.rs`, change:

```rust
pub mod gas;
pub mod smoke;
pub mod spawn;
pub mod system_ai;
pub mod wind;
```

to:

```rust
pub mod fire;
pub mod gas;
pub mod smoke;
pub mod spawn;
pub mod system_ai;
pub mod wind;
```

**Step 2: Wire the dispatcher**

In `src/sim/particles/system_ai.rs`, change:

```rust
fn tick_fire(_sys: &mut ParticleSystem, _sim: &mut Simulation, _rules: &RuleSet) {
    // Implemented in Task C4.
}
```

to:

```rust
fn tick_fire(sys: &mut ParticleSystem, sim: &mut Simulation, rules: &RuleSet) {
    super::fire::tick_system(sys, sim, rules);
}
```

**Step 3: Update the module-level doc**

In `src/sim/particles/system_ai.rs`, change the line:

```rust
//! then either drops or reinserts. Smoke (C2) and Gas (C3) have full bodies;
//! Fire (C4) and the Tier-3 variants (Spark, Railgun) are still no-ops.
```

to:

```rust
//! then either drops or reinserts. Smoke (C2), Gas (C3), and Fire (C4) have
//! full bodies; the Tier-3 variants (Spark, Railgun) are still no-ops.
```

**Step 4: Verify**

Run: `cargo build`
Expected: clean build.

Run: `cargo test --lib particles::`
Expected: all 25 existing particle tests still pass (no fire tests yet).

---

### Task 7: Add fire tests

**Why:** Lock the parity-critical rules in CI before the eventual wiring lands. Each test pins one rule from the parity-critical table.

**Files:**
- Modify: [src/sim/particles/fire.rs](../../src/sim/particles/fire.rs) — append `#[cfg(test)] mod tests`

**Pattern:** Mirror the gas test module ([src/sim/particles/gas.rs:212-352](../../src/sim/particles/gas.rs#L212-L352)).

**Step 1: Append the test module to `fire.rs`**:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::particle_system_type::ParticleSystemTypeId;
    use crate::sim::particles::ParticleSystem;
    use glam::IVec3;

    fn fake_system(type_id: ParticleSystemTypeId) -> ParticleSystem {
        ParticleSystem {
            stable_id: 0,
            type_id,
            coords: IVec3::ZERO,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(1),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1D,
            marked_for_deletion: false,
            directionless: false,
            attached_entity: None,
            owner_entity: None,
            target_coords: IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        }
    }

    fn parse(ini_text: &str) -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(ini_text)).expect("rules parse")
    }

    #[test]
    fn velocity_zero_marks_for_deletion() {
        // Parity rule: fire dies the instant its momentum runs out.
        let rules = parse(
            "[Particles]\n\
             1=Fire\n\
             [Fire]\n\
             BehavesLike=Fire\n\
             MaxEC=500\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::ZERO,
            IVec3::ZERO,
            pt,
            &mut sim.rng,
        );
        p.velocity = SIM_ZERO;
        tick_particle(&mut p, pt, &mut sim.rng);
        assert!(p.marked_for_deletion, "zero-velocity fire dies immediately");
    }

    #[test]
    fn translucency_thresholds_apply_at_states() {
        // Translucent25State=10 → 0x32 (lighter fade); Translucent50State=15
        // → 0x19 (deeper fade). Auto-advance is deferred, so the test pins
        // animation_state directly between ticks. Both checks fire every
        // tick, so externally-driven state changes still produce the
        // correct fade.
        let rules = parse(
            "[Particles]\n\
             1=Fire\n\
             [Fire]\n\
             BehavesLike=Fire\n\
             MaxEC=500\n\
             Velocity=28.0\n\
             StartStateAI=5\n\
             EndStateAI=19\n\
             Translucent50State=15\n\
             Translucent25State=10\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::ZERO,
            IVec3::ZERO,
            pt,
            &mut sim.rng,
        );
        // State starts at 5 — below both thresholds. Translucency stays at
        // the spawn-time value (the type's Translucency, default 0).
        tick_particle(&mut p, pt, &mut sim.rng);
        assert_eq!(p.translucency, 0);
        // Pin state at Translucent25State; tick → 0x32 (lighter fade).
        p.animation_state = 10;
        tick_particle(&mut p, pt, &mut sim.rng);
        assert_eq!(p.translucency, TRANSLUCENT_25_BYTE);
        // Pin state at Translucent50State; tick → 0x19 (deeper fade overwrites).
        p.animation_state = 15;
        tick_particle(&mut p, pt, &mut sim.rng);
        assert_eq!(p.translucency, TRANSLUCENT_50_BYTE);
    }

    #[test]
    fn final_damage_state_clamps_damage_counter_reset() {
        // Past FinalDamageState, the counter still decrements but stops
        // resetting — fire visibly fades but does no damage.
        let rules = parse(
            "[Particles]\n\
             1=Fire\n\
             [Fire]\n\
             BehavesLike=Fire\n\
             MaxEC=500\n\
             MaxDC=3\n\
             Velocity=28.0\n\
             StartStateAI=20\n\
             EndStateAI=99\n\
             FinalDamageState=14\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::ZERO,
            IVec3::ZERO,
            pt,
            &mut sim.rng,
        );
        // Force animation_state past final_damage_state (default 14).
        p.animation_state = 20;
        // Drive damage_counter to zero — must NOT reset to MaxDC.
        for _ in 0..5 {
            tick_particle(&mut p, pt, &mut sim.rng);
        }
        assert!(
            p.damage_counter <= 0,
            "past FinalDamageState, counter must NOT reset (got {})",
            p.damage_counter
        );
    }

    #[test]
    fn move_fire_marks_dead_when_terrain_rises() {
        // Cliff death: old_ground < new_ground → hit_ground + marked dead.
        // Coords still advance (binary's Move_Dispatch does SetCoords after
        // the kill); the dying particle renders one frame at the cliff cell.
        let rules = parse(
            "[Particles]\n\
             1=Fire\n\
             [Fire]\n\
             BehavesLike=Fire\n\
             MaxEC=500\n\
             Velocity=28.0\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::new(100, 100, 0),
            IVec3::ZERO,
            pt,
            &mut sim.rng,
        );
        p.prev_delta = [SimFixed::from_num(5), SIM_ZERO, SIM_ZERO];
        // old_ground=0, new_ground=10 → terrain rises.
        move_fire(&mut p, 0, 10);
        assert!(p.hit_ground, "cliff death sets hit_ground");
        assert!(p.marked_for_deletion, "cliff death marks for deletion");
        // Coords advance to the cliff cell — matches binary parity.
        assert_eq!(p.coords, IVec3::new(105, 100, 0));
        assert_eq!(p.previous_coords, IVec3::new(100, 100, 0));
    }

    #[test]
    fn move_fire_advances_on_flat_ground() {
        // Sanity counterpart: equal grounds → coords advance, no death.
        let rules = parse(
            "[Particles]\n\
             1=Fire\n\
             [Fire]\n\
             BehavesLike=Fire\n\
             MaxEC=500\n\
             Velocity=28.0\n",
        );
        let pt = rules.particle_type(ParticleTypeId(0));
        let mut sim = Simulation::new();
        let mut p = make_particle(
            ParticleTypeId(0),
            IVec3::new(100, 100, 0),
            IVec3::ZERO,
            pt,
            &mut sim.rng,
        );
        p.prev_delta = [SimFixed::from_num(5), SIM_ZERO, SIM_ZERO];
        move_fire(&mut p, 0, 0);
        assert!(!p.marked_for_deletion);
        assert_eq!(p.coords, IVec3::new(105, 100, 0));
    }

    #[test]
    fn fire_spawn_cap_enforced() {
        // Cap=3 — even with aggressive cadence, particle count must stay ≤ 3.
        let rules = parse(
            "[Particles]\n\
             1=Fire\n\
             [Fire]\n\
             BehavesLike=Fire\n\
             MaxEC=1000\n\
             Velocity=28.0\n\
             [ParticleSystems]\n\
             1=Sys\n\
             [Sys]\n\
             BehavesLike=Fire\n\
             HoldsWhat=Fire\n\
             ParticleCap=3\n\
             SpawnFrames=1\n\
             Spawns=yes\n",
        );
        let mut sim = Simulation::new();
        let mut sys = fake_system(ParticleSystemTypeId(0));
        for _ in 0..50 {
            tick_system(&mut sys, &mut sim, &rules);
            sim.tick += 1;
        }
        assert!(
            sys.particles.len() <= 3,
            "cap exceeded: {}",
            sys.particles.len()
        );
    }
}
```

**Step 2: Verify**

Run: `cargo test --lib particles::fire`
Expected: 6 tests pass.

Run: `cargo test --lib particles::`
Expected: 25 + 6 = 31 particle tests pass.

---

### Task 8: Run full regression and commit

**Why:** Lock the work behind a single atomic commit, mirroring C2/C3.

**Step 1: Build clean**

Run: `cargo build`
Expected: clean build, no new warnings.

**Step 2: Run full lib test suite**

Run: `cargo test --lib`
Expected: 1457 + 6 = 1463 tests pass, 0 failures.

**Step 3: Commit**

```sh
git add src/sim/particles/fire.rs \
        src/sim/particles/mod.rs \
        src/sim/particles/system_ai.rs \
        src/sim/particles/spawn.rs \
        src/sim/particles/smoke.rs \
        src/sim/particles/gas.rs

git commit -m "particles: Fire BehavesLike (system AI + particle AI + Move_Fire + FinalDamageState gate)"
```

Run: `git status`
Expected: working tree clean, branch ahead of `origin/dev` (do NOT push).

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-04-particle-system-rust-plan.md](2026-05-04-particle-system-rust-plan.md) — Task C4 lines ~1774-1836
- **Ghidra reports:** `ra2-rust-game-docs/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` §3.6 (Fire System AI), §3.8 (Fire Particle Behaviors), §3.9 (Move Dispatch fire path), §10.13 (Fire Particle Ground Detection deep dive — jitter formula at §10.13.1, ground detection at §10.13.2, FinalDamageState gate at §10.13.3)
- **Repo patterns:** [src/sim/particles/gas.rs](../../src/sim/particles/gas.rs) (C3 prior art), [src/sim/particles/smoke.rs](../../src/sim/particles/smoke.rs) (C2 prior art), [src/sim/particles/spawn.rs:127](../../src/sim/particles/spawn.rs#L127) (`spawn_particle_with_insert` from B4)
- **INI keys:** `ini/rulesmd.ini` `[FireStream]` (line 26054) — MaxEC, MaxDC, Damage, Warhead, Velocity, Deacc, StartStateAI, EndStateAI, StateAIAdvance, Translucent50State, Translucent25State, FinalDamageState, DeleteOnStateLimit, Normalized; `[FireStreamSys]` (line 26016) — HoldsWhat, Spawns, SpawnFrames, BehavesLike, Image, Lifetime
- **Prior commits:** c4bc036 (C2 Smoke), 37c7608 (C3 Gas) — same pattern this plan follows
