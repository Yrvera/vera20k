# Radiation Green Glow (Render) — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make irradiated ground (and the units/buildings standing on it) visibly glow green
on every Desolator deploy / `RadLevel>0` detonation, by feeding the existing sim-side
`RadiationState` into the existing per-cell `CellLightGrid` as one green point light per site.

**Architecture:** Render/app-only feature. A new app module (`app_radiation_light`) derives one
`PointLight` per live radiation site from `RadiationState` + `[Radiation]` rules and folds them
into `rebuild_lighting_grid_from_sim` alongside building lamps; a per-frame epoch check rebuilds the
lighting grid only when a site crosses a `RadLightDelay` step boundary (per-step cadence). No sim
state, snapshot version, or state-hash change — the glow is a pure read-only function of serialized
sim state.

**Design Doc:** [docs/plans/2026-06-15-radiation-glow-render-design.md](2026-06-15-radiation-glow-render-design.md)

---

## Grounding Summary

- **Docs:** gamemd behavior is binary-verified in the design doc §2 and the worknotes under
  `docs/research/substrate/worknotes/radiation-glow-render-20260615/` (`gamemd-radsite-light.md`,
  `gamemd-lighting-system.md`, `verification.md`), plus `RADIATION_EMP_GHIDRA_REPORT.md` and
  `CELLCLASS_MAPCLASS_…STUDY.md §2.6`. All three load-bearing claims (intensity formula, dual-curve
  fade + cadence, additive→clamp per-cell compositing) were **independently re-verified** this round
  (`verification.md`): `read_memory 0x007edae0 = 2000.0`; intensity `min(level×RadLightFactor,2000)`
  at `0x0065B580`; tint `min(c×1000/255×RadTintFactor,2000) × remaining/duration` with intensity on a
  fixed per-step subtraction at `0x0065B800`; light steps every `RadLightDelay`; not TS/SpecialFlags
  gated.
- **Repo pattern this mirrors:** building lamps — `lighting::point_light_from_object` +
  `accumulate_point_lights` (`src/map/lighting.rs`) driven by `collect_live_building_lights` +
  `rebuild_lighting_grid_from_sim` (`src/app_init.rs:171-218`). Radiation reuses the identical
  `PointLight` → additive
  `CellLightGrid` path, the `1000==1.0` light units, and the matching `LIGHT_CLAMP_MAX = 2000`
  (`lighting.rs:32`).
- **INI keys (all already parsed, nothing to add — `src/rules/ruleset.rs:953-1035`):**
  `[Radiation] RadColor=0,255,0`, `RadLightFactor=0.1`, `RadTintFactor=1.0`, `RadLightDelay=90`,
  `RadDurationMultiple=1`. RA2 `rules.ini` == YR `rulesmd.ini` here. `RadSiteColor` does NOT exist
  (Ares-only — do not add).
- **Sim API the render reads (verified `src/sim/radiation.rs`, `src/sim/world/mod.rs:382`):**
  `sim.radiation: RadiationState` → `sites() -> impl Iterator<&RadSite>`, `is_empty()`,
  `RadiationState::current_site_level(site)`. `RadSite { center, spread, radius_leptons, level,
  level_steps, duration, remaining, .. }` (all `pub`).
- **Still unknown after grounding (deferred, not blocking — see Open Questions):** SNOW-theater R/B
  channel forcing, `EMPulseSparkles` anim, exotic non-stock-factor `SimFixed`-vs-`double` drift,
  `ftol` ±1-unit rounding, and whether a per-site vs per-cell feed is bit-identical (resolved in the
  in-app verification task).

## Key Technical Decisions

- **SEAM A — accumulate a green `PointLight` per site into `CellLightGrid`.** Reproduces gamemd's
  observable result (one `LightSourceClass` per site → per-cell tint read by terrain AND sprite
  draws) with zero GPU/shader/pipeline change. — **Confidence:** high — **Source:** design §5,
  `rust-render-architecture.md` worknote, `src/map/lighting.rs`.
- **Per-site light model** (one `PointLight` from `sites()`, recompute falloff via the site radius),
  not per-cell. Matches gamemd's one-LightSource-per-site object model. — **Confidence:** high —
  **Source:** user decision 2026-06-15; design §8.6; `0x0065B580`.
- **Per-step cadence via a `RadLightDelay`-quantized epoch.** Light params are computed from the
  integer step index `k = (duration − remaining) / RadLightDelay`, so the glow dims in discrete
  steps exactly like gamemd; the lighting grid is rebuilt only when the epoch (sites × their `k`)
  changes. — **Confidence:** high — **Source:** user decision 2026-06-15; design §2.4/§2.5;
  `0x0065B800`.
- **Fold radiation into `rebuild_lighting_grid_from_sim`** so both of its callers (map load
  `app_init.rs:1069`, building placement `app_input.rs:827`) automatically include the glow. Screen
  transitions *assign* a precomputed grid (`app_transitions.rs:151` = `state.lighting_grid =
  result.lighting_grid;`, not a rebuild call); a transition that restores a saved grid self-heals
  within ≤90 ticks (and on the first in-game frame) via the per-step trigger. — **Confidence:** high
  — **Source:** repo structure (single rebuild entry point; two call sites verified).
- **Render-side `f32` factor math** via `sim_to_f32(rules.light_factor/tint_factor)`. At stock
  `0.1`/`1.0` this yields exactly intensity `50` / green `1000` (verified by the Task 2 unit test);
  drift only at exotic non-stock factors. — **Confidence:** medium — **Source:** design §8.3;
  `sim_to_f32` `src/util/fixed_math.rs:78`. (Flagged for /review-plan.)
- **Restructure the design's slices 1–4 into one correct math implementation + staged in-app
  verification.** The four design slices differ only by which formula terms are enabled; the math is
  binary-verified and unit-testable up front, so a throwaway static-then-fade build is wasteful.
  Instead, Task 2 implements the full correct per-site math with unit tests (covering slices 2/3/4's
  formula correctness), and the in-app verification (Task 6) is **staged** (green-appears → fade →
  expiry → stacking) to give the same incremental debugging signal slices 1→4 intended, without
  intermediate throwaway code. Slice 5 (secondary visuals) stays deferred. — **Confidence:** medium
  — **Source:** plan author's call; **flag for /review-plan** (deviates from the approved slice-by-
  slice build).

## Open Questions

### Resolved During Planning

- *Refresh cadence?* → **per-step** (user, 2026-06-15) — implemented via the `k`-quantized epoch.
- *Per-site or per-cell light model?* → **per-site** (user, 2026-06-15).
- *Does `RadiationState` expose everything render needs?* → Partially. Sites + geometry yes; the
  glow constants live on `RuleSet.radiation` (threaded into the adapter separately), and there is no
  separate light timer — the fade is derived from `remaining`/`duration` + `RadLightDelay`, so **no
  new sim field / no snapshot bump** (design §3.1).
- *Where does the green actually land?* → `PointLight` → `accumulate_point_lights` (additive) →
  `CellLightGrid` tint → read by terrain (`terrain_tile_tint_at`) AND sprite (`techno_tint_at`)
  draws. Confirmed in `src/map/lighting.rs`.

### Deferred to Implementation / Later

- **Per-site vs per-cell pixel-identity** — confirm in Task 6 that the per-site feed produces the
  same visible footprint as the sim's per-cell field.
- **SNOW-theater R/B channel forcing** (design §8.4) — UNKNOWN gamemd intent; needs a fresh Ghidra
  read; out of scope (Slice 5).
- **`EMPulseSparkles` secondary anim** (design §8.5) — separate from the light; scope question;
  out of scope (Slice 5).
- **Exotic non-stock `RadLightFactor`/`RadTintFactor`** — the lossy step is the INI→`SimFixed`
  (I16F16) parse at `ruleset.rs:1018-1025` (≈1.5e-5 quantization, **already in committed code**),
  compounded by the render `f32` cast; gamemd keeps `double`. Stock `0.1`/`1.0` is exact — verified:
  `sim_from_f32(0.1)` = bits 6554 = 0.1000061, ×500 → `50`; `sim_from_f32(1.0)` exact → green base
  `1000`. Not fixed here (design §8.3).
- **Intensity `.max(0)` floor is a proven no-op, not a DRIFT** — gamemd's per-step subtract
  (`0x0065B800`) has no clamp-to-0, but with `k` clamped to `[0, steps_total]` and integer
  `decrement = intensity_spawn / steps_total` (floor), `steps_total × decrement ≤ intensity_spawn`,
  so the value never goes negative across the site's lifetime. The `.max(0)` is a safety net that
  never triggers — it does not diverge from gamemd's unclamped subtract.
- **Pre-existing `accumulate_point_lights` DRIFT** (per-light per-channel clamp + f32 cell-space
  distance vs gamemd's sum-then-clamp + lepton-center distance, design §8.7) — radiation inherits it;
  out of scope to fix the shared accumulator here.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/map/lighting.rs` | Add `radiation_point_light()` constructor (needs the private `HALF_CELL_LEPTONS`) + unit test |
| Create | `src/app_radiation_light.rs` | Pure per-site light math (`radiation_site_light`), `collect_radiation_lights`, `radiation_light_epoch` + unit tests |
| Modify | `src/lib.rs:46` | Declare `pub mod app_radiation_light;` |
| Modify | `src/app_init.rs:171-186` | Fold radiation lights into `rebuild_lighting_grid_from_sim` |
| Modify | `src/app.rs:286,2550` | Add `last_radiation_light_epoch: u64` field (after :286) + init in the sole `AppState` literal (`AppState {` at :2438, field init at :2550) |
| Modify | `src/app_sim_tick.rs:259-260` | Add `refresh_radiation_glow()` + call it in `advance_in_game_runtime` |

## Interface Changes

- **New public fn** `lighting::radiation_point_light(rx, ry, radius_leptons, intensity, tint) ->
  PointLight` — consumed only by `app_radiation_light`. No existing caller affected.
- **New module** `app_radiation_light` with `collect_radiation_lights(&Simulation, &RuleSet) ->
  Vec<PointLight>` and `radiation_light_epoch(&RadiationState, &RadiationRules) -> u64`.
- **Changed fn body (not signature)** `rebuild_lighting_grid_from_sim` — now also accumulates
  radiation lights. Both call sites (`app_init.rs:1069`, `app_input.rs:827`) keep their call
  unchanged and gain the glow; neither must *exclude* radiation (every rebuild should reflect live
  radiation).
- **New `AppState` field** `last_radiation_light_epoch: u64` — app view-state only. There is exactly
  ONE `AppState` struct literal (`AppState {` at `app.rs:2438`, with `lighting_grid` init at
  `app.rs:2550`); initialize the new field to `0` there. (Do NOT touch `app_transitions.rs:65` — that
  is a `MapLoadResult` literal, a different struct.)

## Sim Checklist

This plan touches **no `sim/` code** — `src/sim/radiation.rs` is read-only here. Therefore:

- [x] No `fixed`/`f32` change in sim logic (render-side `f32` only, in the app layer).
- [x] No new state in the deterministic state hash (the glow reads serialized sim state; it never
      mutates sim or feeds back into the hash). **Determinism unchanged** — assert in Task 5.
- [x] No dependency added from `sim/` onto `render/ui/...` (the app layer reads `sim/` one-way).
- [x] No `advance_tick` tick-ordering change.
- [x] `BTreeMap` order: `sites()` iterates in deterministic center order; the epoch hash is therefore
      stable across runs.

## Risk Areas

- **Borrow conflict in `refresh_radiation_glow`** — recomputing the grid (immutable borrows of
  `state.simulation`/`state.rules`/`state.map_lighting_config`) then assigning `state.lighting_grid`
  (mutable). Mitigated by computing the new grid in an inner scope that drops the shared borrows
  before the assignment (Task 4 code).
- **Performance** — `accumulate_point_lights` iterates all grid cells per rebuild. Rebuilds now also
  fire on radiation step boundaries (every `RadLightDelay`≈90 ticks while a site is alive) and on the
  first frame radiation appears/disappears. Idle matches (no sites) pay one epoch hash per frame and
  never rebuild. Acceptable; flagged for the in-app FPS check in Task 6.
- **First-frame redundant rebuild** — `last_radiation_light_epoch` defaults to `0`; the no-site epoch
  is the FNV seed (≠0), so the first in-game frame triggers exactly one extra full lighting rebuild
  (same cost as map-load lighting). Negligible, documented.
- **Residual cells after a site dies** — the sim leaves decayed cell levels after a site self-deletes;
  the glow tracks **sites**, not cells, so it vanishes exactly when the site dies (matches gamemd's
  light dying with the `RadSite`). The site-removal transition changes the epoch → one clearing
  rebuild.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 2 | Intensity `min(level×RadLightFactor, 2000)`, `ftol`/trunc | Onset brightness of every glow | Unit test (stock = 50); design §2.2, `0x0065B580`, `read_memory 0x007edae0=2000.0` |
| 2 | Tint `min(c×1000/255×RadTintFactor, 2000)` | Exact green (stock = `[0,1000,0]`); the `×1000/255` rescale is easy to miss | Unit test; design §2.3, `0x0065B580` |
| 2 | **Dual decay curves** — tint by `remaining/duration` ratio, intensity by fixed per-step subtraction `intensity_spawn/(duration/RadLightDelay)` | The two fade at different rates; a single shared scalar is wrong | Unit test across k=0..5; design §2.4, `0x0065B800` |
| 2,4 | **Per-step (stepwise) cadence** on `RadLightDelay` | gamemd dims in ~5–6 discrete steps, not smoothly | `k`-quantized math (Task 2) + epoch trigger (Task 4); in-app step-count check (Task 6); design §2.5 |
| 1,3 | **Additive** accumulation, ground+sprite uniform tint | Player sees green *ground* + units, summed not max-blended | Reuses `accumulate_point_lights` (already additive); in-app (Task 6); design §2.6/§2.7 |
| 1 | Detail forced on (`detail: true`) | Radiation is never culled by `[Options]DetailLevel` (unlike lamps) | `radiation_point_light` sets `detail: true`; design §2.6 |
| 6 | Expiry timing | Glow must vanish exactly when `remaining < 1` (site self-deletes) | In-app + gamemd side-by-side (Task 7) |

---

## Tasks

### Task 1: `radiation_point_light` constructor in `lighting.rs`

**Why:** Define the producer type first. The adapter needs to build a `PointLight` at a cell center,
but `HALF_CELL_LEPTONS` (the cell-center offset) is private to `lighting.rs`, so the constructor must
live here (mirrors `point_light_from_object`).

**Files:**
- Modify: `src/map/lighting.rs` (add after `point_light_from_object`, ~line 569)

**Pattern:** Mirrors `point_light_from_object` (`lighting.rs:539-569`).

**Step 1: Add the constructor**
```rust
// src/map/lighting.rs — insert after point_light_from_object (after line 569)

/// Build a radiation-glow point light at a cell center.
///
/// `intensity` and `tint` are already in `1000 == 1.0` units (the radiation
/// light math is done by the caller). Detail is forced on: radiation is never
/// culled by the detail level, unlike ordinary lamps.
pub fn radiation_point_light(
    rx: u16,
    ry: u16,
    radius_leptons: i32,
    intensity: i32,
    tint: [i32; 3],
) -> PointLight {
    PointLight {
        rx,
        ry,
        center_x: i32::from(rx) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        center_y: i32::from(ry) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        radius_leptons: radius_leptons.max(0),
        intensity,
        tint,
        active: true,
        detail: true,
    }
}
```

**Step 2: Add a unit test** (in the existing `#[cfg(test)] mod tests` in `lighting.rs`)
```rust
#[test]
fn test_radiation_point_light_center_and_flags() {
    // spread 10 → radius 10*256+128 = 2688; center cell (100,100).
    let light = radiation_point_light(100, 100, 2688, 50, [0, 1000, 0]);
    assert_eq!(light.center_x, 100 * 256 + 128);
    assert_eq!(light.center_y, 100 * 256 + 128);
    assert_eq!(light.radius_leptons, 2688);
    assert_eq!(light.intensity, 50);
    assert_eq!(light.tint, [0, 1000, 0]);
    assert!(light.active && light.detail, "radiation light is active and detail-forced");
}
```

**Step 3: Verify**
Run: `cargo test -p vera20k --lib radiation_point_light -- --nocapture`
Expected: `test result: ok`. (Read the literal `test result:` line.)

**Step 4: Commit** — `render: radiation glow T1 — radiation_point_light constructor`

---

### Task 2: `app_radiation_light` module — pure per-site light math + collectors

**Why:** The binary-verified intensity/tint/fade formulas, isolated as a pure, fully unit-tested
function. This is where slices 2/3/4's formula correctness is proven without running the game.

**Files:**
- Create: `src/app_radiation_light.rs`
- Modify: `src/lib.rs:46` (add `pub mod app_radiation_light;`)

**Pattern:** New module. Bridges `sim::radiation` + `rules::ruleset::RadiationRules` +
`map::lighting::PointLight` — lives in the app layer (which may depend on everything).

**Step 1: Declare the module** in `src/lib.rs` (after line 46, `pub mod app_init;`)
```rust
pub mod app_radiation_light;
```

**Step 2: Write the module**
```rust
// src/app_radiation_light.rs
//! Render-layer radiation glow: derive one green point light per live radiation
//! site from the sim-side `RadiationState` + `[Radiation]` rules, mirroring
//! gamemd's per-site `LightSourceClass`. Pure read of sim state — never mutates
//! it, never feeds the deterministic hash.
//!
//! Depends on `sim/` (read-only `RadiationState`/`RadSite`), `rules/`
//! (`RadiationRules`), and `map/lighting` (`PointLight`). Part of the app layer.
//!
//! ## Float exception
//! Light intensity/tint use render-side `f32`, matching the design's render math
//! exception (the original computes this in `double`; values are never hashed).

use crate::map::lighting::{self, LIGHT_CLAMP_MAX, LIGHT_UNIT, PointLight};
use crate::rules::ruleset::{RadiationRules, RuleSet};
use crate::sim::radiation::{RadSite, RadiationState};
use crate::sim::world::Simulation;
use crate::util::fixed_math::sim_to_f32;

/// FNV-1a seed/prime for the per-step light epoch (any non-cryptographic mix).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Derive the stepwise green point light for one radiation site, or `None` when
/// the site is degenerate or fully faded.
///
/// Stepwise (per-step cadence): every value is quantized to the integer step
/// index `k = (duration - remaining) / RadLightDelay`, so the light is
/// piecewise-constant between steps and dims in discrete steps like gamemd.
///
/// - intensity = `min(level * RadLightFactor, 2000)` minus a fixed per-step
///   decrement `intensity_spawn / (duration / RadLightDelay)`, clamped at 0.
/// - tint_c = `min(c * 1000 / 255 * RadTintFactor, 2000) * remaining_at_step / duration`.
pub fn radiation_site_light(site: &RadSite, rules: &RadiationRules) -> Option<PointLight> {
    if site.duration < 1 {
        return None;
    }
    let light_delay = rules.light_delay.max(1);
    let steps_total = (site.duration / light_delay).max(1);
    let elapsed = site.duration - site.remaining; // remaining <= duration always
    let k = (elapsed / light_delay).clamp(0, steps_total);
    let remaining_at_step = site.duration - k * light_delay;

    // Intensity: min(level * RadLightFactor, 2000), faded by the fixed per-step decrement.
    let light_factor = sim_to_f32(rules.light_factor);
    let intensity_spawn = ((site.level as f32 * light_factor) as i32).min(LIGHT_CLAMP_MAX);
    let decrement = intensity_spawn / steps_total;
    let intensity = (intensity_spawn - k * decrement).max(0);

    // Tint: per channel min(c * 1000/255 * RadTintFactor, 2000), faded by the
    // remaining/duration ratio (computed at the step boundary).
    let tint_factor = sim_to_f32(rules.tint_factor);
    let channel_base = |c: u8| -> i32 {
        let rescaled = (i32::from(c) * LIGHT_UNIT) / 255; // x1000/255 byte rescale (verified)
        ((rescaled as f32 * tint_factor) as i32).min(LIGHT_CLAMP_MAX)
    };
    let faded = |base: i32| -> i32 {
        // i64 intermediate avoids overflow at heavy stacking; gamemd uses 32-bit.
        (i64::from(base) * i64::from(remaining_at_step) / i64::from(site.duration)) as i32
    };
    let (cr, cg, cb) = rules.color;
    let tint = [faded(channel_base(cr)), faded(channel_base(cg)), faded(channel_base(cb))];

    if intensity == 0 && tint == [0, 0, 0] {
        return None;
    }
    Some(lighting::radiation_point_light(
        site.center.0,
        site.center.1,
        site.radius_leptons,
        intensity,
        tint,
    ))
}

/// Collect one green point light per live radiation site.
pub fn collect_radiation_lights(sim: &Simulation, rules: &RuleSet) -> Vec<PointLight> {
    sim.radiation
        .sites()
        .filter_map(|site| radiation_site_light(site, &rules.radiation))
        .collect()
}

/// A cheap epoch that changes only when a site is added/removed or crosses a
/// `RadLightDelay` step boundary. Drives the per-step rebuild trigger so the
/// lighting grid is rebuilt stepwise, not every frame.
pub fn radiation_light_epoch(rad: &RadiationState, rules: &RadiationRules) -> u64 {
    let light_delay = rules.light_delay.max(1);
    let mut h = FNV_OFFSET;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(FNV_PRIME);
    };
    for site in rad.sites() {
        let steps_total = (site.duration / light_delay).max(1);
        let elapsed = site.duration - site.remaining;
        let k = (elapsed / light_delay).clamp(0, steps_total);
        mix(u64::from(site.center.0));
        mix(u64::from(site.center.1));
        mix(k as u64);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stock_rules() -> RadiationRules {
        RadiationRules {
            duration_multiple: 1,
            application_delay: 16,
            level_max: 500,
            level_delay: 90,
            light_delay: 90,
            level_factor: 0.2,
            light_factor: crate::util::fixed_math::sim_from_f32(0.1),
            tint_factor: crate::util::fixed_math::sim_from_f32(1.0),
            color: (0, 255, 0),
            site_warhead: "RadSite".to_string(),
        }
    }

    /// Build a stock Desolator site at a given `remaining` (level 500, spread 10,
    /// duration 500).
    fn site_at_remaining(remaining: i32) -> RadSite {
        RadSite {
            center: (100, 100),
            spread: 10,
            radius_leptons: 10 * 256 + 128,
            level: 500,
            level_steps: 500 / 90,
            duration: 500,
            remaining,
            level_timer_start: 0,
            level_timer_duration: 90,
        }
    }

    #[test]
    fn stock_desolator_spawn_intensity_and_pure_green_tint() {
        // k=0 (remaining == duration): intensity 50, tint [0,1000,0].
        let light = radiation_site_light(&site_at_remaining(500), &stock_rules()).unwrap();
        assert_eq!(light.intensity, 50, "500 * 0.1 = 50, under the 2000 clamp");
        assert_eq!(light.tint, [0, 1000, 0], "green 255*1000/255*1.0 = 1000; R/B = 0");
        assert_eq!(light.radius_leptons, 2688);
    }

    #[test]
    fn stock_desolator_dual_decay_curves_are_stepwise() {
        let rules = stock_rules();
        // steps_total = 500/90 = 5; decrement = 50/5 = 10.
        // k=1 (remaining 410): intensity 40, green 1000*410/500 = 820.
        let l1 = radiation_site_light(&site_at_remaining(410), &rules).unwrap();
        assert_eq!((l1.intensity, l1.tint[1]), (40, 820));
        // k=2 (remaining 320): intensity 30, green 640.
        let l2 = radiation_site_light(&site_at_remaining(320), &rules).unwrap();
        assert_eq!((l2.intensity, l2.tint[1]), (30, 640));
        // k=5 (remaining 50): intensity 0 (faded out first), green 100 (still lit).
        let l5 = radiation_site_light(&site_at_remaining(50), &rules).unwrap();
        assert_eq!((l5.intensity, l5.tint[1]), (0, 100));
    }

    #[test]
    fn stepwise_holds_constant_between_step_boundaries() {
        let rules = stock_rules();
        // remaining 410 (k=1) and 330 (still k=1: elapsed 170 / 90 = 1) must match —
        // the value steps only at boundaries, not continuously.
        let a = radiation_site_light(&site_at_remaining(410), &rules).unwrap();
        let b = radiation_site_light(&site_at_remaining(330), &rules).unwrap();
        assert_eq!((a.intensity, a.tint), (b.intensity, b.tint));
    }

    #[test]
    fn intensity_clamps_at_2000_when_stacked() {
        // level 25000 * 0.1 = 2500 → clamped to 2000.
        let mut site = site_at_remaining(1);
        site.level = 25000;
        site.duration = 25000;
        site.remaining = 25000;
        let light = radiation_site_light(&site, &stock_rules()).unwrap();
        assert_eq!(light.intensity, LIGHT_CLAMP_MAX);
    }

    #[test]
    fn tint_channel_clamps_at_2000_with_high_tint_factor() {
        let mut rules = stock_rules();
        rules.tint_factor = crate::util::fixed_math::sim_from_f32(3.0); // 1000*3 = 3000 → clamp 2000
        let light = radiation_site_light(&site_at_remaining(500), &rules).unwrap();
        assert_eq!(light.tint[1], LIGHT_CLAMP_MAX);
    }

    #[test]
    fn degenerate_duration_yields_no_light() {
        let mut site = site_at_remaining(0);
        site.duration = 0;
        assert!(radiation_site_light(&site, &stock_rules()).is_none());
    }

    #[test]
    fn epoch_changes_only_on_step_boundary() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(
            crate::sim::radiation::RadDetonation { rx: 10, ry: 10, rad_level: 500, spread: 10 },
            0,
            &rules,
            None,
        );
        let e0 = radiation_light_epoch(&rad, &rules);
        // Tick within the first step window — epoch unchanged.
        for f in 1..=89 {
            rad.tick_decay(f, &rules, None);
        }
        assert_eq!(radiation_light_epoch(&rad, &rules), e0, "no step crossed yet");
        // Cross the 90-frame boundary — epoch changes.
        rad.tick_decay(90, &rules, None);
        assert_ne!(radiation_light_epoch(&rad, &rules), e0, "first step crossed");
    }
}
```

**Step 3: Verify**
Run: `cargo test -p vera20k --lib app_radiation_light -- --nocapture`
Expected: `test result: ok. 7 passed`. (Read the literal `test result:` line; if any expected value
mismatches, STOP — the formula or its source is wrong, do not adjust the test to match the code.)

**Step 4: Commit** — `render: radiation glow T2 — per-site light math (intensity/tint/dual-fade) + epoch`

---

### Task 3: Fold radiation lights into `rebuild_lighting_grid_from_sim`

**Why:** Make every lighting rebuild include the glow, so map load, building placement, and screen
transitions all reflect live radiation — no callsite can drop it.

**Files:**
- Modify: `src/app_init.rs:171-186`

**Pattern:** Extends the existing building-lamp accumulation in the same function.

**Step 1: Extend the rebuild** — replace the body of `rebuild_lighting_grid_from_sim`
(`app_init.rs:177-185`) so radiation lights are appended before accumulation:
```rust
    let mut lighting_grid = lighting::build_cell_light_grid_from_heights(
        resolved_terrain
            .iter()
            .map(|cell| ((cell.rx, cell.ry), cell.level)),
        lighting_config,
    );
    let mut point_lights = collect_live_building_lights(simulation, rules);
    // Radiation green glow: one green point light per live radiation site,
    // accumulated additively alongside building lamps (render-only).
    if let (Some(sim), Some(rules)) = (simulation, rules) {
        point_lights.extend(crate::app_radiation_light::collect_radiation_lights(sim, rules));
    }
    lighting::accumulate_point_lights(&mut lighting_grid, &point_lights);
    lighting_grid
```

**Step 2: Verify it compiles + existing lighting tests still pass**
Run: `cargo test -p vera20k --lib lighting -- --nocapture`
Expected: `test result: ok` (base lighting + building-lamp tests unchanged).

**Step 3: Commit** — `render: radiation glow T3 — fold radiation lights into the lighting rebuild`

---

### Task 4: `AppState` epoch field + per-frame `refresh_radiation_glow` trigger

**Why:** Drive the per-step rebuild — recompute the lighting grid only when a site crosses a step
boundary (or the site set changes), so the glow dims stepwise and idle matches pay nothing.

**Files:**
- Modify: `src/app.rs` (field decl after line 286; init in the sole `AppState` literal at line 2550)
- Modify: `src/app_sim_tick.rs` (add `refresh_radiation_glow` + call it)

**Pattern:** New app view-state field (mirrors other transient `AppState` fields); the trigger
mirrors the event-driven `state.lighting_grid = rebuild_lighting_grid_from_sim(...)` callers.

**Step 1: Add the field** in `src/app.rs` after line 286 (`pub(crate) lighting_grid: CellLightGrid,`):
```rust
    /// Last radiation-glow light epoch applied to `lighting_grid`. The glow is
    /// rebuilt only when this changes (a site stepped on `RadLightDelay`, or the
    /// site set changed). App view-state only — never serialized or hashed.
    pub(crate) last_radiation_light_epoch: u64,
```

**Step 2: Initialize it** in the sole `AppState` struct literal.
In `src/app.rs` after line 2550 (`lighting_grid: CellLightGrid::new(),`, inside the `AppState {` literal
that begins at line 2438):
```rust
            last_radiation_light_epoch: 0,
```
There is no second `AppState` literal — `app_transitions.rs:65` is a `MapLoadResult` literal (a
different struct) and must NOT get this field, or the build fails with `error[E0560]`. The map-load
path assigns `state.lighting_grid = result.lighting_grid` (`app_transitions.rs:151`) into this same
already-constructed `AppState`, so no other init site is needed; a stale epoch carried into a new
match self-corrects on the first frame (the `0`/seed mismatch forces one rebuild).

**Step 3: Add the trigger fn** in `src/app_sim_tick.rs` (e.g. after `advance_fixed_simulation`,
before `schedule_fixed_steps` at line 853):
```rust
/// Per-frame radiation-glow refresh. Rebuilds the lighting grid only when the
/// radiation light epoch changes (a site crossed a `RadLightDelay` step boundary,
/// or a site appeared/disappeared) — i.e. stepwise, matching gamemd. Idle matches
/// (no sites) pay one epoch hash per frame and never rebuild. Render-only: never
/// touches sim state or the deterministic hash.
fn refresh_radiation_glow(state: &mut AppState) {
    let epoch = match (state.simulation.as_ref(), state.rules.as_ref()) {
        (Some(sim), Some(rules)) => {
            crate::app_radiation_light::radiation_light_epoch(&sim.radiation, &rules.radiation)
        }
        _ => return,
    };
    if epoch == state.last_radiation_light_epoch {
        return;
    }
    // Recompute in an inner scope so the shared borrows of `state` drop before
    // the mutable assignment to `state.lighting_grid`. Terrain is sourced from
    // `state.resolved_terrain` to match the existing building-placement caller
    // (app_input.rs:826) — same grid the building lamps light off.
    let new_grid = {
        let (Some(sim), Some(rules)) = (state.simulation.as_ref(), state.rules.as_ref()) else {
            return;
        };
        let Some(terrain) = state.resolved_terrain.as_ref() else {
            return;
        };
        crate::app_init::rebuild_lighting_grid_from_sim(
            terrain,
            &state.map_lighting_config,
            Some(sim),
            Some(rules),
        )
    };
    state.last_radiation_light_epoch = epoch;
    state.lighting_grid = new_grid;
}
```

**Step 4: Call it** in `advance_in_game_runtime`, immediately after the `if run_sim { … }` block
closes (`src/app_sim_tick.rs:259`), before `update_radar_state` (line 261):
```rust
    }

    // Refresh the radiation green glow after the sim steps (stepwise; no-op when
    // no radiation site crossed a step boundary this frame).
    refresh_radiation_glow(state);

    crate::app_building_anim::update_radar_state(state, SIM_TICK_MS as f32);
```

**Step 5: Verify it compiles**
Run: `cargo check -p vera20k`
Expected: clean (no errors). Confirm the literal "Finished" / no `error[` lines.

**Step 6: Commit** — `render: radiation glow T4 — per-step glow refresh trigger`

---

### Task 5: Full regression + determinism gate

**Why:** Confirm nothing regressed and — critically — that the render-only feature did NOT shift the
deterministic state hash (the #1 invariant).

**Files:** none (verification only).

**Step 1: Build + lint the touched files**
Run: `cargo clippy -p vera20k` → expect no new warnings in `lighting.rs`, `app_radiation_light.rs`,
`app_init.rs`, `app_sim_tick.rs`, `app.rs`, `app_transitions.rs`.
Then format only the touched files: `rustfmt --edition 2024 src/app_radiation_light.rs src/map/lighting.rs src/app_init.rs src/app_sim_tick.rs` (and confirm no churn in regions you didn't edit; do NOT run crate-wide `cargo fmt`).

**Step 2: Full test suite**
Run: `cargo test -p vera20k`
Expected: read the literal `test result:` line — the pre-change pass count **+ the new tests**, zero
failures. In particular the golden/replay/state-hash tests (`determinism_replay`, snapshot goldens)
must be **unchanged** — if any state-hash golden shifts, STOP: the glow leaked into sim state, which
violates the design (render-only). Investigate before proceeding.

**Step 3: Commit** (only if rustfmt changed anything) — `render: radiation glow T5 — fmt`

---

### Task 6: In-app staged visual verification

**Why:** Confirm the seam actually lights the ground + units, the fade is stepwise, expiry is exact,
and stacking brightens — the observable acceptance criteria (design §7), staged for debuggability.

**Files:** none (run the app — use the project run path).

**Verify (in order; each stage isolates a different risk):**
1. **Seam (glow appears):** Start a skirmish, build/deploy a Desolator. Expected: the irradiated
   cells (the `(2·spread+1)²` square, falling off to the edge) turn green — **both the ground tile
   and any unit/building standing on them**. If the ground tints but sprites don't (or vice-versa),
   the seam is wrong, not the math.
2. **Stepwise fade:** Watch one deploy over its ~33 s lifetime (15 fps). Expected: the green **dims
   in discrete steps** (~5–6 steps), not a smooth ramp; intensity fades to flat before the green tint
   fully disappears (dual curves).
3. **Expiry:** Expected: the glow vanishes the frame the site dies (~500 frames for stock), leaving
   no green residue.
4. **Stacking:** Deploy two Desolators on overlapping squares (or a nuke over a deploy). Expected:
   the overlap reads **brighter** (additive), and does not blow out past the clamp.
5. **Detail level:** Lower `[Options]DetailLevel`. Expected: the glow still shows (radiation forces
   detail on).
6. **Performance:** Confirm no FPS drop while radiation is active (the rebuild fires only every ~90
   ticks per step).

Capture a screenshot of stage 1 and stage 4 for the record. If stage 1 fails, re-check Task 3/4
wiring; if stages 2–4 look wrong but Task 2 tests pass, the bug is in the feed/cadence, not the math.

---

### Task 7: gamemd side-by-side parity verification

**Why:** The bar is indistinguishable-from-gamemd. Confirm the observable result matches the original.

**Files:** none.

**Verify:** Deploy a Desolator at the same cell on the same map in both `gamemd.exe` and this engine;
compare:
- **Onset frame** — glow appears the same tick relative to deploy.
- **Color** — the same green hue/intensity at spawn.
- **Footprint** — same affected square + falloff extent.
- **Fade-step cadence** — same number of dim steps and the same per-step brightness drop (tint vs
  intensity curves).
- **Expiry frame** — disappears at the same point.

If any differs, the discrepancy is a bug (default to DRIFT): re-open `verification.md` and the cited
addresses (`0x0065B580`, `0x0065B800`) before adjusting code. Optionally run `/fidelity-check` on the
radiation glow.

---

## Sources & References

- **Design doc:** [docs/plans/2026-06-15-radiation-glow-render-design.md](2026-06-15-radiation-glow-render-design.md)
- **Worknotes:** `docs/research/substrate/worknotes/radiation-glow-render-20260615/`
  (`existing-research-and-rust-state.md`, `gamemd-radsite-light.md`, `gamemd-lighting-system.md`,
  `rust-render-architecture.md`, `ini-keys.md`, `verification.md`)
- **Ghidra reports:** `RADIATION_EMP_GHIDRA_REPORT.md` (§1.6/§1.8/§1.11),
  `CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (§2.6),
  `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`,
  `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`,
  `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`
- **gamemd.exe addresses (kept here, not in Rust comments):** `RadSiteClass__Activate 0x0065B580`,
  `RadSiteClass__AI 0x0065B800`, clamp const `0x007edae0 = 2000.0`, `ftol 0x007c5f00`,
  `LightSourceClass ctor 0x00554760`, per-cell compute `0x00484180`/`0x00483E30`, blend `0x004842B0`,
  per-tick driver `0x0055AFB0`. INI offsets `RulesClass+0x1814` (RadLightDelay), `+0x1820`
  (RadLightFactor), `+0x1828` (RadTintFactor), `+0x1830` (RadColor).
- **INI keys:** `rulesmd.ini [Radiation] RadColor=0,255,0`, `RadLightFactor=0.1`, `RadTintFactor=1.0`,
  `RadLightDelay=90`, `RadDurationMultiple=1` (== `rules.ini`).
- **Related code:** `src/sim/radiation.rs`, `src/rules/ruleset.rs:953-1035`, `src/map/lighting.rs`
  (`CellLightGrid`, `accumulate_point_lights`, `PointLight`, `LIGHT_CLAMP_MAX`), `src/app_init.rs:171`,
  `src/app_sim_tick.rs:203-281`, `src/util/fixed_math.rs:78`.
- **Prior commit:** `86b0d4bf` (sim-core radiation, Slice 7).
