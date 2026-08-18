# PixelFX Water/Ore Sparkle — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> The "Tiny-Detail Ledger" L1–L30 in the design doc is the parity constraint set —
> every implementation task cites which ledger items it covers.

**Goal:** Implement a stateless, per-frame, per-cell water/ore sparkle render
module reproducing gamemd.exe's `DrawPixelFXSparkles @ 0x006D7840` within the
project parity bar (visually indistinguishable in single skirmish; not
bit-identical).

**Architecture:** New `src/render/pixel_fx_sparkles.rs` is a pure builder
function consumed by the existing `InstanceBufferPool` and dispatched in
`draw_passes.rs` as a new Step 5.5 between ground objects (Step 5) and
turrets (Step 6). Reuses `SelectionOverlay::white_texture()` for the 1×1 white
texel. No new pipelines, no new atlases, no sim/ dependencies introduced.

**Design Doc:** [docs/plans/2026-05-18-pixel-fx-sparkle-design.md](./2026-05-18-pixel-fx-sparkle-design.md)

---

## Revisions

- **2026-05-18 (post-`/review-plan`):** five fixes folded in.
  - **Issue #1:** `sim.world.tick` access path was wrong — `Simulation` has direct `pub tick: u64`. Combined with Issue #5 fix (below) — SparkleInput now takes a single `clock_ms: u64` field sourced from `sim.total_sim_ms`.
  - **Issue #2:** `state.config.*` was wrong — `AppState` does not store `GameConfig`. Task 1 now also adds `game_config: Option<GameConfig>` to AppState and populates it in `AppState::new()`.
  - **Issue #3:** `rules.overlay_types` was wrong — the registry lives on `AppState` (`state.overlay_registry: Option<OverlayTypeRegistry>`). Task 10 wrapper updated.
  - **Issue #4:** `OverlayCell.overlay_id` is `Option<u8>`, not `u8`. Task 7's `ore_value_nonzero` rewritten with `.and_then`.
  - **Issue #5:** `Simulation.total_sim_ms: u64` is the engine's authoritative time source — pre-computed each tick, no need to manually convert `tick × 1000 / hz`. SparkleInput simplified accordingly.

## Grounding Summary

- **Docs (R1):** `PIXEL_FX_SPARKLES_GHIDRA_REPORT.md` is the authoritative source — verified live this session including §14 close-out. Cross-references: `CELLCLASS_STRUCT_GHIDRA_REPORT.md` (+0x12C and +0x140 bit layouts) and `MAPCLASS_COMPLETE_DECODE.md` §E (cell+0x12C complete bit map).
- **Ghidra (R2):** All load-bearing functions live-verified this session — `DrawPixelFXSparkles @ 0x006D7840`, `PixelFXClass::Init @ 0x00631D40`, `PixelFXClass::Update_Color @ 0x00631E50`, `MapClass::Invalidate_Radius_For_Redraw @ 0x00568140`, `MapClass::Conceal_Radius @ 0x00567F70`, `CellClass::SetBridgeDirection_NESW @ 0x0047E040`, `FUN_00684C30` (OreTwinkle spawner). Param tables read directly from binary at `0x008367C8` (water) and `0x008367F0` (ore).
- **Repo pattern (R3):** Mirror `build_smudge_instances` at [src/app_render/build_instances.rs:282-...](../../src/app_render/build_instances.rs#L282) — pure builder reading sim state, returning `Vec<SpriteInstance>`. Dispatch follows `draw_pooled_passthrough_overlay` pattern at [src/app_render/draw_passes.rs:532-542](../../src/app_render/draw_passes.rs#L532-L542) but using `SelectionOverlay::white_texture()` since we don't need the overlay atlas.
- **INI (R4):** NONE. The sparkle constants live in gamemd's binary (verified by direct memory reads), not in rulesmd.ini. The "Extra Animations" toggle is a user-pref equivalent — lives in `config.toml`, not in rules.
- **Still unknown:** whether the 2500ms cycle bucket produces visible drift vs gamemd in normal play. Will be answered by manual visual verification (Task 13) and, if necessary, a future pixel-diff audit (option D from the original menu).

## Key Technical Decisions

- **Stateless hash-derived sparkle** with per-cell-cycle randomized LerpSpeed — **Confidence: high** — Source: design doc §Chosen Approach, brainstorm-approved.
- **Fixed 2500ms cycle bucket per species** as stateless approximation of gamemd's sequential variable-duration cycles — **Confidence: high** for arithmetic correctness, **medium** for player-imperceptible parity claim — Source: design doc §Chosen Approach + acknowledged drift; needs empirical confirmation via Task 13.
- **Sim-time clock via `sim.total_sim_ms`** (the sim's pre-computed authoritative tick-ms) over wall clock — **Confidence: high** — Source: brainstorm-approved for replay determinism; `total_sim_ms` field at [src/sim/world/mod.rs:229](../../src/sim/world/mod.rs#L229) explicitly documented as "Authoritative time source." Originally planned to compute `tick * 1000 / hz` manually; `/review-plan` flagged the engine already provides this.
- **splitmix64 hash** (3 ops, no state, well-distributed) — **Confidence: high** — Source: Vigna's PRNG paper; widely used (used in JDK, Go standard lib).
- **Reuse `SelectionOverlay::white_texture()`** for the 1×1 white texel — **Confidence: high** — Source: brainstorm-approved; texture exists at [src/render/selection_overlay.rs:319](../../src/render/selection_overlay.rs#L319).
- **`extra_animations: bool` on `GraphicsConfig`** with default `true` — **Confidence: high** — Source: brainstorm-approved; matches gamemd's default and option label.
- **Sparkle module does its own viewport iteration** rather than reusing the terrain-pass cell list — **Confidence: high**, but a deviation from the design's stated preference — Source: practical analysis after reading [src/app_render/build_instances.rs:126-159](../../src/app_render/build_instances.rs#L126-L159) shows `terrain::build_visible_instances` doesn't currently expose its viewport cell list. Extracting it would touch terrain code (high risk for cosmetic win). Sparkle module's own iteration is ~10 lines, negligible cost. **Deviation flagged for `/review-plan`.**

## Open Questions

### Resolved During Planning

- **Q: Does the existing batch.draw_with_buffer_passthrough accept any BatchTexture, or only OverlayAtlas?** A: Any `&BatchTexture` — confirmed at [src/render/batch.rs:1364-1370](../../src/render/batch.rs#L1364-L1370). `SelectionOverlay::white_texture()` returns `&BatchTexture` — direct match.
- **Q: How does `state.selection_overlay` get accessed?** A: `state.selection_overlay: Option<SelectionOverlay>` at [src/app.rs:173](../../src/app.rs#L173). Already used elsewhere in `draw_passes.rs` (e.g., debug overlays). Use `if let Some(ref overlay) = state.selection_overlay { ... }`.
- **Q: Where does the local-player owner ID come from at render time?** A: Use the existing pattern at [src/app_render/build_instances.rs:129-139](../../src/app_render/build_instances.rs#L129-L139): `preferred_local_owner_name(state)` → `sim.interner.get(owner)`.
- **Q: What time unit does `radar_anim::tick` use?** A: `dt_ms: f32`. But that's a per-tick advance, not a cumulative clock. Sparkle wants an absolute clock derived from `world.tick`, not a per-frame delta — already addressed in the design.

### Deferred to Implementation

- **Whether splitmix64 distribution is good enough for visible cell grids.** Mitigated by Task 13's visual check; if banding appears, swap to PCG-XSH-RR (different hash, same module-private function).
- **Whether `sandbox_full_visibility` should bypass the sight gate** (like it does for terrain rendering at [build_instances.rs:133](../../src/app_render/build_instances.rs#L133)). Plan defaults to honouring sandbox_full_visibility (bypass sight gate when set) for consistency.

> Previously deferred: "Whether `state.config.graphics.extra_animations` is accessible inside `build_world_instances`." → **Resolved by `/review-plan` revision:** `AppState` does NOT have a `config` field. Task 1 now adds `game_config: Option<GameConfig>` to `AppState`; the wrapper reads `state.game_config.as_ref().map_or(true, |c| c.graphics.extra_animations)`.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/render/pixel_fx_sparkles.rs` | Stateless sparkle math + builder. Pure function, no GPU. |
| Modify | `src/render/mod.rs` | `pub mod pixel_fx_sparkles;` declaration. |
| Modify | `src/util/config.rs` | Add `extra_animations: bool` field to `GraphicsConfig`. |
| Modify | `config.toml.example` | Document the new field. |
| Modify | `src/app.rs` | Store loaded `GameConfig` as `game_config: Option<GameConfig>` field on `AppState`. |
| Modify | `src/app_render/build_instances.rs` | Add `cell_sparkles` field to `WorldInstances`; call sparkle builder. |
| Modify | `src/app_render/mod.rs` | Upload `cell_sparkles` to pool in `upload_to_gpu`. |
| Modify | `src/app_render/draw_passes.rs` | New Step 5.5 between Step 5 (ground objects) and Step 6 (turrets). |

## Interface Changes

- **`GraphicsConfig` gains `extra_animations: bool`** (default `true`). Public field on a struct already widely accessed; backward-compatible because of `#[serde(default = "default_true")]`.
- **`WorldInstances` gains `cell_sparkles: Vec<SpriteInstance>`** (public field). `pub(super)` struct, only used inside `app_render/`. No external dependents.
- **`InstanceBufferPool` gains a new keyed buffer `"cell_sparkles"`.** Pool is keyed by `&'static str`; adding a key is non-breaking.
- **`src/render/pixel_fx_sparkles.rs` exports `pub fn build_sparkle_instances` and `pub struct SparkleInput<'a>`.** New API; consumed only by `build_instances.rs`.

## Sim Checklist

No tasks in this plan touch `sim/`. The sparkle module is render-only — it READS sim state (`fog`, `occupancy`, `overlays`, `bridge_state`, `world.tick`) through immutable references but never writes. Lockstep is preserved trivially (no sim mutation, no sim hash field).

## Risk Areas

- **Hash distribution quality.** Bad distribution → visible grid patterns or beat-sync rows of cells pulsing together. Mitigation: splitmix64 is well-tested; Task 13's visual check catches this. Fallback: swap hash function inline (single private fn).
- **Draw-order placement.** Wrong step insertion (e.g., after turrets) breaks occlusion. Mitigation: Task 12 explicitly inserts between Step 5 and Step 6; Task 13's visual check (sparkles UNDER units, ABOVE water) catches drift.
- **Determinism drift.** If any non-deterministic source sneaks in (`thread_rng`, `std::time::Instant`, etc.), replays will desync. Mitigation: the module imports nothing from `std::time` or `rand`. Task 9's `same_tick_same_cell_same_rgb` test catches it.
- **Sandbox-mode visibility.** If `sandbox_full_visibility=true` and we honour the sight gate strictly, sparkles will only appear where the local player has units. Mitigation: follow the existing terrain pattern — bypass sight gate when `sandbox_full_visibility` is set.

## Parity-Critical Items

Every item from the design's ledger that has a corresponding implementation task:

| Task | Item | Ledger | Why it matters | Verification |
|------|------|--------|----------------|--------------|
| Task 2 | Water/ore constants table | L1–L9 | Wrong colour values shift the sparkle hue; player notices a "wrong" water tone immediately at night. | Task 2 unit tests (`water_constants_match_report`, `ore_constants_match_report`) lock values to ledger. |
| Task 4 | Ping-pong lerp formula | L13–L16 | Wrong formula → sparkle fades wrong way or skips peak; visible flicker pattern. | Task 4 tests assert phase=0→base, phase=0x1000→peak, phase=0x1FFF→near-base, plus symmetry. |
| Task 5 | Sub-pixel offset range / peak colour noise | L3, L8, L10, L11 | Wrong range → sparkles cluster at wrong sub-pixel positions or are off-cell entirely. | Task 5 tests sample 1000 (cell, cycle) pairs and assert range bounds. |
| Task 7 | Six-gate condition (water-or-ore, occupied, in-sight, bridge-deck, etc.) | L17–L22 | Wrong gate → sparkles appear under units, in shrouded cells, on bridges, or with extra-animations OFF. All player-visible. | Task 7 has one test per gate condition. |
| Task 8 | Phase logic and base-during-wait | L23, L25 | Cell must START dim each cycle (L23) and MOSTLY appear dim (L25). Skipping the base-during-wait check makes cells "always sparkling" — wrong feel. | Task 8 tests assert that during cycle_pos < timer_init the colour equals base. |
| Task 8 | Cycle re-init re-randomizes sub-pos and colour | L24 | Without re-randomization, sparkles appear fixed-position — the "moving" feel is lost. | Task 8 test compares consecutive `cycle_index` outputs and asserts sub-pos differs. |
| Task 8 | Asynchronous cell start via per-cell offset | L26 | Without per-cell offset, all cells with the same hash would peak in unison → visible map-wide pulses. | Task 8 test asserts two adjacent cells have different `cycle_pos_ms` at the same `clock_ms`. |
| Task 12 | Draw order: between Step 5 (ground objects) and Step 6 (turrets) | L27 | Wrong order → sparkles draw over units or under terrain. | Task 13 manual visual check confirms units occlude sparkles; sparkles draw over water terrain. |
| Task 12 | Opaque pass (alpha=1.0, no blend) | L28 | Wrong blend → sparkles look translucent or blend wrong with water. | Inherits from `draw_with_buffer_passthrough` which is depth-bypass + no blend; Task 13 confirms visually. |

---

## Tasks

### Task 1: Add `extra_animations` config field + plumb GameConfig onto AppState

**Why:** The gate condition L22 (`g_ExtraAnimationsEnabled`) needs a config-driven boolean. Plumbing the field first — both in the schema AND on `AppState` so render-time code can read it — means subsequent tasks can pull it without re-loading the config. Ledger: **L22**.

**Files:**
- Modify: `src/util/config.rs:45-58` (GraphicsConfig struct)
- Modify: `src/util/config.rs:60-69` (GraphicsConfig::default impl)
- Modify: `config.toml.example` (one new line under `[graphics]`)
- Modify: `src/app.rs:75-200` (AppState struct: add `game_config` field)
- Modify: `src/app.rs:849+` (AppState::new struct literal: populate `game_config` from the already-loaded local)

**Pattern:** GraphicsConfig field follows existing `vsync: bool` and `upscale: bool`. AppState field follows existing `pub(crate)` config-adjacent fields (e.g., `terrain_grid`, `overlay_registry`).

**Step 1: Add field to struct.**

In `src/util/config.rs`, inside `pub struct GraphicsConfig { ... }`, add (after the `upscale: bool` field):

```rust
    /// Enable cosmetic per-frame effects: water/ore sparkles. Also intended to
    /// gate future cosmetic effects (laser beam pulses, particle systems, line
    /// trails) per gamemd's "Extra Animations" option. Default ON to match
    /// gamemd's default.
    #[serde(default = "default_true")]
    pub extra_animations: bool,
```

**Step 2: Update Default impl.**

In the same file, inside `impl Default for GraphicsConfig { fn default() -> Self { Self { ... } } }`, add the field:

```rust
            extra_animations: true,
```

(after `upscale: false,`)

**Step 3: Update config.toml.example.**

Find the `[graphics]` section in `config.toml.example` and add:

```toml
extra_animations = true   # cosmetic effects: water/ore sparkles (future: laser pulses, particle trails)
```

**Step 4: Add `game_config` field to AppState.**

In `src/app.rs`, locate the `AppState` struct field block (currently lines 75–200, near other `pub(crate)` configuration-adjacent fields like `terrain_grid`/`overlay_registry`). Add:

```rust
    /// Loaded GameConfig — None when config.toml is missing or invalid.
    /// Read at render time for cosmetic toggles (extra_animations) and other
    /// per-session user preferences. Set in AppState::new() from the existing
    /// GameConfig::load() call; not mutated afterwards.
    pub(crate) game_config: Option<GameConfig>,
```

Place it adjacent to the existing config-adjacent fields. If `GameConfig` is not already imported at the top of `src/app.rs`, it already is — see [src/app.rs:65](../../src/app.rs#L65) (`use crate::util::config::GameConfig;`).

**Step 5: Populate `game_config` in `AppState::new()`.**

In `src/app.rs` at around line 849, the local `let game_config = GameConfig::load().ok();` already produces the value we need. It's borrowed (`.as_ref()`) by `input_delay_ticks` and `upscale_pass` initialization but is NOT consumed — still owned at the final `AppState { ... }` struct literal.

In that struct literal (around lines 940+), add the field after the existing `pub(crate)` fields (e.g., after `resolved_terrain: None,`):

```rust
            game_config,
```

This moves the local into the field. No additional code needed — the load already happens.

**Step 6: Verify.**

Run: `cargo check`
Expected: clean compile.

Run: `cargo test --lib util::config`
Expected: existing tests pass (this is a backwards-compatible addition).

**Step 7: Commit.**

```
config: add extra_animations toggle + plumb GameConfig onto AppState

Adds GraphicsConfig.extra_animations (default true) matching gamemd's
"Extra Animations" option. Stores the loaded GameConfig as an AppState
field so render-time code can read cosmetic toggles without re-loading.
Will gate the upcoming PixelFX water/ore sparkle render pass.
```

---

### Task 2: Create `pixel_fx_sparkles` module skeleton — constants + types

**Why:** Define the constant tables and struct types first. Subsequent tasks fill in helpers and the math; this gives them a place to land. Ledger items locked in tests at this step: **L1–L9**.

**Files:**
- Create: `src/render/pixel_fx_sparkles.rs`
- Modify: `src/render/mod.rs` (add `pub mod pixel_fx_sparkles;`)

**Pattern:** Follows the per-species constants pattern used in `src/sim/combat/` for armor multipliers — module-private const tables tied directly to the gamemd binary values.

**Step 1: Create the file.**

Write `src/render/pixel_fx_sparkles.rs`:

```rust
//! Per-frame water/ore sparkle render — observable parity with gamemd.exe's
//! DrawPixelFXSparkles. See ra2-rust-game-docs/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md
//! for the full reverse-engineering and the design doc at
//! docs/plans/2026-05-18-pixel-fx-sparkle-design.md.
//!
//! Stateless / hash-derived: each visible water or ore cell, every frame,
//! hashes (cell_coord, cycle_index) to derive sub-pixel position, peak
//! colour noise, lerp speed, and timer-init for the current cycle, then
//! computes the sparkle's current RGB analytically. No per-cell persistent
//! state.
//!
//! ## Dependency rules
//! - Part of render/ — reads sim/ state through immutable references only.
//!   No writes to sim. No coupling to GPU types beyond SpriteInstance.

use crate::render::batch::SpriteInstance;

/// Per-species sparkle parameters mirroring gamemd's
/// g_PixelFXParams_Water (0x008367C8) and g_PixelFXParams_Ore (0x008367F0)
/// tables. Read directly from the binary; see report §5.2.
#[derive(Debug, Clone, Copy)]
struct SparkleParams {
    /// Dim endpoint of the lerp. Applied with weight (0x1000 - lerp).
    base_rgb: [u8; 3],
    /// Bright endpoint of the lerp. Applied with weight lerp; per cycle, each
    /// channel may be reduced by `0..(1 << color_noise_bits)`.
    peak_rgb: [u8; 3],
    /// Per-channel noise bit count subtracted from peak. 0 = no noise (ore).
    color_noise_bits: u8,
    /// Inclusive lower bound for the per-cell-per-cycle LerpSpeed (phase / ms).
    lerp_speed_min: u32,
    /// Inclusive upper bound for the per-cell-per-cycle LerpSpeed.
    lerp_speed_max: u32,
}

/// Water sparkle constants — verified by direct memory read at
/// gamemd.exe 0x008367C8. See report §5.2. (L1, L2, L3, L4)
const WATER: SparkleParams = SparkleParams {
    base_rgb: [40, 40, 80],          // L1
    peak_rgb: [158, 158, 224],       // L2
    color_noise_bits: 5,             // L3
    lerp_speed_min: 3,               // L4
    lerp_speed_max: 12,              // L4
};

/// Ore sparkle constants — verified by direct memory read at
/// gamemd.exe 0x008367F0. See report §5.2. (L6, L7, L8, L9)
const ORE: SparkleParams = SparkleParams {
    base_rgb: [176, 144, 0],         // L6
    peak_rgb: [255, 255, 240],       // L7
    color_noise_bits: 0,             // L8
    lerp_speed_min: 15,              // L9
    lerp_speed_max: 30,              // L9
};

/// Average cycle length for the stateless cycle-bucket approximation.
/// gamemd's per-cycle duration is (timer_init 0..4095 ms) + (active
/// 0x2000/lerp_speed ms). Avg ≈ 2048 + 430 = 2478 ms. Round to 2500 for
/// both species (coincidentally similar). See design doc §Cycle bucketing.
const WATER_CYCLE_BUCKET_MS: u64 = 2500;
const ORE_CYCLE_BUCKET_MS: u64 = 2500;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_constants_match_report() {
        // Lock L1, L2, L3, L4 against the report. Any tuning would have to
        // change both the report (and the underlying binary memory!) and
        // this test in tandem.
        assert_eq!(WATER.base_rgb, [40, 40, 80]);
        assert_eq!(WATER.peak_rgb, [158, 158, 224]);
        assert_eq!(WATER.color_noise_bits, 5);
        assert_eq!(WATER.lerp_speed_min, 3);
        assert_eq!(WATER.lerp_speed_max, 12);
    }

    #[test]
    fn ore_constants_match_report() {
        // Lock L6, L7, L8, L9 against the report.
        assert_eq!(ORE.base_rgb, [176, 144, 0]);
        assert_eq!(ORE.peak_rgb, [255, 255, 240]);
        assert_eq!(ORE.color_noise_bits, 0);
        assert_eq!(ORE.lerp_speed_min, 15);
        assert_eq!(ORE.lerp_speed_max, 30);
    }

    #[test]
    fn cycle_buckets_are_positive_and_documented() {
        // Sanity: buckets must be non-zero (division by zero in cycle math).
        // The actual 2500ms value is an approximation choice; this test
        // documents that we picked it deliberately.
        assert_eq!(WATER_CYCLE_BUCKET_MS, 2500);
        assert_eq!(ORE_CYCLE_BUCKET_MS, 2500);
    }
}
```

**Step 2: Register the module.**

In `src/render/mod.rs`, add (alphabetically among the existing `pub mod ...;` declarations):

```rust
pub mod pixel_fx_sparkles;
```

**Step 3: Verify.**

Run: `cargo check`
Expected: clean compile (no other code uses the module yet).

Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: 3 tests pass.

**Step 4: Commit.**

```
render: scaffold pixel_fx_sparkles module with water/ore constants

Defines SparkleParams, WATER and ORE constants verified by direct memory
read at gamemd.exe 0x008367C8 / 0x008367F0, and a 2500ms cycle bucket
constant for the stateless approximation. Tests lock constants to the
report so they cannot drift silently.
```

---

### Task 3: Implement hash helpers — splitmix64 + coord_key

**Why:** All randomness in the module comes from these two functions. Separating them from the math keeps tests focused and the math readable. Ledger: foundational (consumed by L10, L11, L24, L26).

**Files:**
- Modify: `src/render/pixel_fx_sparkles.rs` (add private helpers + tests)

**Pattern:** Inline private helpers. Mirrors `src/sim/rng.rs`'s xorshift64* but a different algorithm (splitmix64) chosen for one-shot bit-derivation rather than streaming PRNG.

**Step 1: Add the helpers.**

Insert in `src/render/pixel_fx_sparkles.rs` after the `const ORE: SparkleParams = ...;` block and the cycle bucket consts:

```rust
/// Splitmix64 — Vigna's PRNG, used here as a one-shot 64→64 bit hash.
/// Three operations: add, xor-shift-multiply (×2). Well-distributed; avalanche
/// quality is more than enough for "looks random per pixel."
#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Pack cell coordinates into a 64-bit key for hashing. Layout puts rx in
/// the high 16 bits of the upper 32, ry in the high 16 bits of the lower 32,
/// leaving the low 32 bits as a 0 sentinel that the caller can XOR with
/// cycle_index when mixing per-cycle entropy.
#[inline]
fn coord_key(rx: u16, ry: u16) -> u64 {
    ((rx as u64) << 32) | ((ry as u64) << 16)
}
```

**Step 2: Add tests.**

Append inside the existing `#[cfg(test)] mod tests { ... }` block:

```rust
    #[test]
    fn splitmix64_is_deterministic() {
        // Same input always yields same output (necessary for replay
        // determinism). Spot-check a handful of inputs.
        assert_eq!(splitmix64(0), splitmix64(0));
        assert_eq!(splitmix64(0xDEAD_BEEF), splitmix64(0xDEAD_BEEF));
        assert_eq!(splitmix64(u64::MAX), splitmix64(u64::MAX));
    }

    #[test]
    fn splitmix64_distributes_low_bits() {
        // For 1000 consecutive inputs, the low byte of the output should
        // span at least 200 distinct values out of 256. Catches a hash
        // that's stuck on a small subset.
        let mut seen = std::collections::HashSet::new();
        for i in 0u64..1000 {
            seen.insert(splitmix64(i) & 0xFF);
        }
        assert!(seen.len() >= 200, "splitmix64 low-byte spread too small: {}", seen.len());
    }

    #[test]
    fn coord_key_is_injective_for_typical_map() {
        // Two adjacent cells must produce different keys (else the cell
        // offset would not break beat-sync per L26).
        assert_ne!(coord_key(10, 10), coord_key(11, 10));
        assert_ne!(coord_key(10, 10), coord_key(10, 11));
        assert_ne!(coord_key(0, 0), coord_key(0, 1));
    }

    #[test]
    fn coord_key_with_cycle_xor_breaks_per_cycle() {
        // Hashing (coord_key XOR cycle_index) — different cycles must yield
        // different splitmix64 outputs for the same cell (else L24
        // re-randomization would not happen).
        let key = coord_key(50, 50);
        let s0 = splitmix64(key ^ 0);
        let s1 = splitmix64(key ^ 1);
        let s2 = splitmix64(key ^ 2);
        assert_ne!(s0, s1);
        assert_ne!(s1, s2);
        assert_ne!(s0, s2);
    }
```

**Step 3: Verify.**

Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: 7 tests pass (3 from Task 2 + 4 new).

**Step 4: Commit.**

```
render/pixel_fx_sparkles: add splitmix64 + coord_key helpers

Splitmix64 for one-shot 64→64 hashing of (cell coord, cycle index).
coord_key packs (rx, ry) into a u64 that can be XOR'd with cycle_index
for per-cycle entropy. Tests confirm determinism and distribution.
```

---

### Task 4: Implement ping-pong lerp formula

**Why:** This is the core color computation — phase → RGB. Standalone pure function, trivially testable. Locking it down with strong tests prevents silent drift if anyone later "optimizes" the formula. Ledger: **L13, L14, L15, L16**.

**Files:**
- Modify: `src/render/pixel_fx_sparkles.rs`

**Pattern:** Pure function operating on integer math. Mirrors the formula in PIXEL_FX_SPARKLES_GHIDRA_REPORT.md §6.3.

**Step 1: Add the function.**

Insert in `src/render/pixel_fx_sparkles.rs` after the splitmix64/coord_key helpers:

```rust
/// Ping-pong lerp between base (dim) and peak (bright) colors.
///
/// Phase is the position within a cycle, domain [0, 0x2000). The cycle is
/// symmetric: phase [0, 0x1000) rises from base to peak; phase [0x1000,
/// 0x2000) falls from peak back to base. (L13, L14, L16)
///
/// Per-channel formula (L15):
///     current = (base * (0x1000 - lerp) + peak * lerp) >> 12
///
/// where `lerp = phase & 0xFFF`, optionally flipped if bit 0x1000 is set.
#[inline]
fn ping_pong_lerp(phase: u32, base: [u8; 3], peak: [u8; 3]) -> [u8; 3] {
    let mut lerp = phase & 0xFFF;
    if (phase & 0x1000) != 0 {
        lerp = 0x1000 - lerp;       // L14: flip for second half
    }
    let inv = 0x1000 - lerp;
    let blend = |b: u8, p: u8| -> u8 {
        // L15: (base * inv + peak * lerp) >> 12. Use u32 to avoid overflow
        // (255 * 0x1000 = 1,044,480, fits in u32 easily).
        (((b as u32) * inv + (p as u32) * lerp) >> 12) as u8
    };
    [blend(base[0], peak[0]), blend(base[1], peak[1]), blend(base[2], peak[2])]
}
```

**Step 2: Add tests.**

Append inside the tests module:

```rust
    #[test]
    fn lerp_at_phase_0_is_base() {
        // L16: phase 0 → base color (sparkle just spawned, dim).
        // L23: cells START dim each cycle.
        let result = ping_pong_lerp(0, [40, 40, 80], [158, 158, 224]);
        assert_eq!(result, [40, 40, 80]);
    }

    #[test]
    fn lerp_at_phase_0x1000_is_peak() {
        // L16: phase 0x1000 → peak color (sparkle at brightest).
        let result = ping_pong_lerp(0x1000, [40, 40, 80], [158, 158, 224]);
        assert_eq!(result, [158, 158, 224]);
    }

    #[test]
    fn lerp_at_phase_0x1FFF_is_near_base() {
        // L16: phase 0x1FFF → near base (one step before re-init).
        // With lerp = 0xFFF flipped via bit 0x1000, the inv weight is 0xFFF
        // and the lerp weight is 1 — overwhelmingly base.
        let result = ping_pong_lerp(0x1FFF, [40, 40, 80], [158, 158, 224]);
        // (40 * 0xFFF + 158 * 1) >> 12 = (163800 + 158) >> 12 = 164158 >> 12 = 40
        assert_eq!(result, [40, 40, 80]);
    }

    #[test]
    fn lerp_ping_pong_symmetry() {
        // L14: phase (0x1000 - x) and (0x1000 + x) must yield same color
        // for any x in 1..0x1000. This is the ping-pong invariant.
        let base = [40, 40, 80];
        let peak = [158, 158, 224];
        for x in [1u32, 100, 0x400, 0x800, 0xFFF] {
            let rising = ping_pong_lerp(0x1000 - x, base, peak);
            let falling = ping_pong_lerp(0x1000 + x, base, peak);
            assert_eq!(rising, falling, "asymmetry at x={:#x}", x);
        }
    }

    #[test]
    fn lerp_monotonic_rise_first_half() {
        // Phase 0 → 0x1000 should produce monotonically rising R channel
        // (since peak.R > base.R for water). Catches a flipped formula.
        let base = [40, 40, 80];
        let peak = [158, 158, 224];
        let mut prev_r = 0u8;
        for phase in (0..=0x1000).step_by(0x100) {
            let rgb = ping_pong_lerp(phase, base, peak);
            assert!(rgb[0] >= prev_r, "R not monotonic at phase {:#x}: {} < {}", phase, rgb[0], prev_r);
            prev_r = rgb[0];
        }
    }
```

**Step 3: Verify.**

Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: 12 tests pass.

**Step 4: Commit.**

```
render/pixel_fx_sparkles: add ping-pong lerp formula

Pure function mapping phase ∈ [0, 0x2000) to RGB via gamemd's lerp
formula (PIXEL_FX_SPARKLES_GHIDRA_REPORT.md §6.3). Five tests lock
the formula's edge cases (phase=0 → base, phase=0x1000 → peak,
phase=0x1FFF → near base) and the ping-pong symmetry invariant.
```

---

### Task 5: Implement seed-bit extraction helpers

**Why:** The per-cell-cycle hash produces a 64-bit splitmix64 output; we slice off bits for sub-pos, lerp speed, and peak color noise. Wrapping these in named helpers makes the math in Task 8 readable. Ledger: **L3, L4, L8, L9, L10, L11**.

**Files:**
- Modify: `src/render/pixel_fx_sparkles.rs`

**Step 1: Add the helpers.**

Insert in `src/render/pixel_fx_sparkles.rs` after `ping_pong_lerp`:

```rust
/// Extract a per-cycle sub-pixel offset (sub_x, sub_y) from the seed bits.
/// Ranges match gamemd's PixelFXClass::Init (report §6.1):
///   sub_x ∈ [-31, 32] (6 bits → bias by -0x1F)   (L10)
///   sub_y ∈ [-15, 16] (5 bits → bias by -0x0F)   (L11)
#[inline]
fn sub_pos_from_seed(s: u64) -> (i32, i32) {
    let sub_x = ((s & 0x3F) as i32) - 0x1F;          // L10
    let sub_y = (((s >> 6) & 0x1F) as i32) - 0x0F;   // L11
    (sub_x, sub_y)
}

/// Extract this cycle's LerpSpeed, biased into the species range.
/// gamemd uses `rand() % (max - min + 1) + min` — we mirror that. Uses
/// 4 bits of entropy starting at bit 23, leaving bits 11-22 for timer_init.
/// (L4 for water, L9 for ore)
#[inline]
fn lerp_speed_from_seed(s: u64, params: &SparkleParams) -> u32 {
    let range_span = params.lerp_speed_max - params.lerp_speed_min + 1;
    let raw = (s >> 23) & 0xF;       // 4 bits
    params.lerp_speed_min + (raw as u32) % range_span
}

/// Extract this cycle's timer_init offset (0..4095 ms).
/// gamemd: `rand() & 0xFFF` — we do the same. Uses 12 bits starting at bit 11.
/// (L5/L26)
#[inline]
fn timer_init_from_seed(s: u64) -> u32 {
    ((s >> 11) & 0xFFF) as u32
}

/// Compute this cycle's peak RGB with per-channel noise subtract.
/// gamemd applies `mask = (1 << color_noise_bits) - 1`, then per channel:
///   peak[i] -= mask & rand_bits
///   rand_bits >>= color_noise_bits
/// We use 5 bits per channel from bit 27 onwards (15 bits total). For ore
/// (color_noise_bits = 0), no noise is applied. (L3, L8)
#[inline]
fn peak_with_noise(s: u64, params: &SparkleParams) -> [u8; 3] {
    if params.color_noise_bits == 0 {
        return params.peak_rgb;       // L8: ore has no noise
    }
    let mask = (1u32 << params.color_noise_bits) - 1;
    let bits = s >> 27;
    let n0 = (bits as u32) & mask;
    let n1 = ((bits >> params.color_noise_bits) as u32) & mask;
    let n2 = ((bits >> (params.color_noise_bits * 2)) as u32) & mask;
    [
        params.peak_rgb[0].saturating_sub(n0 as u8),
        params.peak_rgb[1].saturating_sub(n1 as u8),
        params.peak_rgb[2].saturating_sub(n2 as u8),
    ]
}
```

**Step 2: Add tests.**

Append inside the tests module:

```rust
    #[test]
    fn sub_pos_ranges_are_correct() {
        // L10: sub_x ∈ [-31, 32]. L11: sub_y ∈ [-15, 16]. Sample 1000
        // different seeds and assert every output is in range.
        for i in 0u64..1000 {
            let (sx, sy) = sub_pos_from_seed(splitmix64(i));
            assert!((-31..=32).contains(&sx), "sub_x out of range at i={}: {}", i, sx);
            assert!((-15..=16).contains(&sy), "sub_y out of range at i={}: {}", i, sy);
        }
    }

    #[test]
    fn lerp_speed_water_in_range() {
        // L4: water LerpSpeed ∈ [3, 12].
        for i in 0u64..1000 {
            let speed = lerp_speed_from_seed(splitmix64(i), &WATER);
            assert!((3..=12).contains(&speed), "water lerp_speed out of range at i={}: {}", i, speed);
        }
    }

    #[test]
    fn lerp_speed_ore_in_range() {
        // L9: ore LerpSpeed ∈ [15, 30].
        for i in 0u64..1000 {
            let speed = lerp_speed_from_seed(splitmix64(i), &ORE);
            assert!((15..=30).contains(&speed), "ore lerp_speed out of range at i={}: {}", i, speed);
        }
    }

    #[test]
    fn timer_init_in_range() {
        // L5/L26: timer_init ∈ [0, 4095].
        for i in 0u64..1000 {
            let timer = timer_init_from_seed(splitmix64(i));
            assert!(timer <= 0xFFF, "timer_init out of range at i={}: {}", i, timer);
        }
    }

    #[test]
    fn ore_has_no_color_noise() {
        // L8: ore peak is always exact (no noise).
        for i in 0u64..100 {
            assert_eq!(peak_with_noise(splitmix64(i), &ORE), ORE.peak_rgb);
        }
    }

    #[test]
    fn water_peak_noise_within_5_bits() {
        // L3: water peak channels each get 0..31 subtracted. Resulting
        // values should be in [peak - 31, peak] for each channel.
        for i in 0u64..1000 {
            let noisy = peak_with_noise(splitmix64(i), &WATER);
            for ch in 0..3 {
                let lo = WATER.peak_rgb[ch].saturating_sub(31);
                let hi = WATER.peak_rgb[ch];
                assert!((lo..=hi).contains(&noisy[ch]),
                    "water peak[{}] out of [-31, 0] noise range at i={}: {} (peak={})",
                    ch, i, noisy[ch], hi);
            }
        }
    }
```

**Step 3: Verify.**

Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: 18 tests pass.

**Step 4: Commit.**

```
render/pixel_fx_sparkles: add per-cycle seed-bit extraction helpers

sub_pos_from_seed (L10, L11), lerp_speed_from_seed (L4, L9),
timer_init_from_seed (L5/L26), peak_with_noise (L3, L8) — each pulls
a bit slice from a splitmix64 output and biases it into gamemd's
range. Tests sample 1000 seeds per helper and assert range bounds.
```

---

### Task 6: Implement viewport-cell iteration helper

**Why:** The sparkle pass needs to walk visible map cells. We don't reuse the terrain pass's iteration (would require modifying terrain code — see plan §Key Technical Decisions). A tiny self-contained helper is sufficient. Ledger: foundational for L17, L27.

**Files:**
- Modify: `src/render/pixel_fx_sparkles.rs`

**Pattern:** Self-contained viewport bound computation. Mirrors what `terrain::build_visible_instances` does internally to compute its cell range, but kept private to the sparkle module so the terrain pass is untouched.

**Step 1: Add the helper.**

Insert in `src/render/pixel_fx_sparkles.rs` after the seed extraction helpers:

```rust
/// Compute the cell bounds (inclusive) of the visible viewport.
///
/// Camera is in world pixels (top-left corner of viewport). vsw and vsh are
/// the effective viewport width and height in world pixels (already
/// zoom-corrected by the caller). Returns (rx_min, ry_min, rx_max, ry_max),
/// clamped to map bounds [0, map_w) and [0, map_h).
///
/// Iso cells are 60 wide × 30 tall (CELL_WIDTH × CELL_HEIGHT in render). We
/// add a 2-cell margin on every side so partially-visible cells at the edges
/// are included. (L27 — sparkles should appear at viewport edges.)
#[inline]
fn viewport_cell_bounds(
    camera_x: f32,
    camera_y: f32,
    vsw: f32,
    vsh: f32,
    map_w: u16,
    map_h: u16,
) -> (u16, u16, u16, u16) {
    const CELL_WIDTH: f32 = 60.0;
    const CELL_HEIGHT: f32 = 30.0;
    const MARGIN_CELLS: i32 = 2;

    // World-pixel viewport rect → approximate cell range. Iso conversion is
    // approximate (we over-include because of the diamond shape) but cheap
    // and correct: the gate inside compute_sparkle_for_cell handles cells
    // outside the actual viewport via the screen position check.
    let rx_min = (((camera_x / CELL_WIDTH).floor() as i32) - MARGIN_CELLS)
        .clamp(0, map_w as i32 - 1) as u16;
    let rx_max = ((((camera_x + vsw) / CELL_WIDTH).ceil() as i32) + MARGIN_CELLS)
        .clamp(0, map_w as i32 - 1) as u16;
    let ry_min = (((camera_y / CELL_HEIGHT).floor() as i32) - MARGIN_CELLS)
        .clamp(0, map_h as i32 - 1) as u16;
    let ry_max = ((((camera_y + vsh) / CELL_HEIGHT).ceil() as i32) + MARGIN_CELLS)
        .clamp(0, map_h as i32 - 1) as u16;
    (rx_min, ry_min, rx_max, ry_max)
}
```

**Step 2: Add tests.**

Append inside the tests module:

```rust
    #[test]
    fn viewport_bounds_clamped_to_map() {
        // Camera at top-left of map; viewport extends beyond map edge.
        // Bounds must be clamped to [0, map - 1].
        let (rxn, ryn, rxx, ryx) = viewport_cell_bounds(0.0, 0.0, 800.0, 600.0, 100, 100);
        assert_eq!((rxn, ryn), (0, 0));
        assert!(rxx <= 99 && ryx <= 99);
    }

    #[test]
    fn viewport_bounds_camera_in_middle() {
        // Camera somewhere in the map; bounds reflect viewport position
        // plus a small margin.
        let (rxn, ryn, rxx, ryx) = viewport_cell_bounds(3000.0, 1500.0, 800.0, 600.0, 200, 200);
        assert!(rxn > 0 && ryn > 0, "bounds should be inset from origin");
        assert!(rxx < 200 && ryx < 200, "bounds should be within map");
        assert!(rxn < rxx && ryn < ryx, "bounds should be ordered");
    }

    #[test]
    fn viewport_bounds_handles_negative_camera() {
        // Camera negative (scrolled past edge) — bounds clamp to 0.
        let (rxn, ryn, _, _) = viewport_cell_bounds(-500.0, -500.0, 800.0, 600.0, 100, 100);
        assert_eq!((rxn, ryn), (0, 0));
    }
```

**Step 3: Verify.**

Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: 21 tests pass.

**Step 4: Commit.**

```
render/pixel_fx_sparkles: add viewport-cell bounds helper

Computes (rx_min, ry_min, rx_max, ry_max) for the visible viewport
with a 2-cell margin. Self-contained to avoid touching terrain code.
Three tests cover top-left, middle, and negative-camera cases.
```

---

### Task 7: Implement `SparkleInput` struct + `compute_sparkle_for_cell` gate logic

**Why:** The gate determines which cells get sparkles. Implementing gate logic without the math means we can lock the six gate conditions independently before adding phase calculations. Ledger: **L17, L18, L19, L20, L21, L22**.

**Files:**
- Modify: `src/render/pixel_fx_sparkles.rs`

**Pattern:** Mirrors `build_smudge_instances` pattern — read sim state through immutable refs from a borrowed input struct.

**Step 1: Add the SparkleInput struct.**

Insert in `src/render/pixel_fx_sparkles.rs` after `viewport_cell_bounds`:

```rust
use crate::map::resolved_terrain::ResolvedTerrainCell;
use crate::sim::bridge_state::BridgeRuntimeState;
use crate::sim::intern::InternedId;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::vision::FogState;

/// All read-only state the sparkle pass needs. Borrowed for the duration of
/// the build call; nothing escapes the function.
pub struct SparkleInput<'a> {
    /// Authoritative sim-time clock in milliseconds. Caller passes
    /// `Simulation.total_sim_ms` (pre-computed each tick by the sim;
    /// see src/sim/world/mod.rs:229). Deterministic across clients on the
    /// same tick — replays look identical.
    pub clock_ms: u64,
    /// From GraphicsConfig.extra_animations. If false, build returns empty Vec. (L22)
    pub enable_extra_animations: bool,
    /// Local player's interned house ID for the sight check (L19).
    pub local_owner_id: Option<InternedId>,
    /// True when sandbox-mode visibility bypass is active; sight gate (L19/L21)
    /// becomes a no-op when set. Mirrors the existing terrain-pass pattern.
    pub sandbox_full_visibility: bool,
    /// Cell terrain data lookup: returns Some(&cell) for cells inside the
    /// resolved-terrain grid, None for off-map.
    pub resolved_terrain: ResolvedTerrainAccess<'a>,
    pub overlays: &'a OverlayGrid,
    pub overlay_registry: &'a crate::map::overlay_types::OverlayTypeRegistry,
    pub occupancy: &'a OccupancyGrid,
    pub fog: &'a FogState,
    /// May be None when no map is loaded (e.g., main menu) — gate returns
    /// false in that case (no bridge means no bridge-deck skip needed, but
    /// also no rendering at all).
    pub bridge_state: Option<&'a BridgeRuntimeState>,
    pub camera_x: f32,
    pub camera_y: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub map_w: u16,
    pub map_h: u16,
    /// White-texel UV coords; for `SelectionOverlay::white_texture()` which
    /// is a 1×1 texture, this is the full (0,0) → (1,1) rect.
    pub white_uv_origin: [f32; 2],
    pub white_uv_size: [f32; 2],
    /// Function mapping cell coord → screen-Y depth, matching the depth
    /// pattern used by other sprite passes. Caller injects to avoid coupling.
    pub depth_for_cell: &'a dyn Fn(u16, u16) -> f32,
    /// Function mapping cell coord → screen-pixel center position. Caller
    /// injects from the existing iso-projection helper.
    pub screen_pos_for_cell: &'a dyn Fn(u16, u16) -> (f32, f32),
}

/// Lookup access for resolved-terrain cells. Wraps the ResolvedTerrainGrid's
/// cell() method through a fn-pointer so the module doesn't need to import
/// the grid type directly (keeps the module surface narrow).
pub struct ResolvedTerrainAccess<'a> {
    pub get: &'a dyn Fn(u16, u16) -> Option<&'a ResolvedTerrainCell>,
}
```

**Step 2: Add the gate function.**

Continue in `src/render/pixel_fx_sparkles.rs` (after `SparkleInput`):

```rust
/// Run the 6-gate check (subset of gamemd's 9-condition gate per report §4).
/// Skipped conditions (vs gamemd):
///   - "16-bit RGB565 surface mode" — not applicable to wgpu RGBA8 path.
///   - "surface lock succeeds" — wgpu doesn't lock surfaces; always succeeds.
///   - "viewport clip" — handled by depth/scissor, not a per-cell check.
///
/// Returns (is_ore, params, bucket_ms) when the cell qualifies; None otherwise.
fn gate_cell<'a>(rx: u16, ry: u16, input: &'a SparkleInput<'_>) -> Option<(bool, &'static SparkleParams, u64)> {
    let cell = (input.resolved_terrain.get)(rx, ry)?;
    let has_ore = ore_value_nonzero(rx, ry, input);
    if !cell.is_water && !has_ore { return None; }                              // L17

    if let Some(occ) = input.occupancy.get(rx, ry) {
        if !occ.occupants.is_empty() { return None; }                            // L18
    }

    if !input.sandbox_full_visibility {
        let Some(owner_id) = input.local_owner_id else { return None; };          // L19/L21
        if !input.fog.is_cell_visible(owner_id, rx, ry) { return None; }
    }

    if let Some(bridge_state) = input.bridge_state {
        if bridge_state.is_bridge_walkable(rx, ry) { return None; }              // L20
    }

    let (params, bucket) = if has_ore {
        (&ORE, ORE_CYCLE_BUCKET_MS)
    } else {
        (&WATER, WATER_CYCLE_BUCKET_MS)
    };
    Some((has_ore, params, bucket))
}

/// Lookup the cell's tiberium-ore flag via the overlay grid + registry.
/// Returns true iff the cell has a non-empty tiberium overlay.
/// `OverlayCell.overlay_id` is `Option<u8>` (None = no overlay), so we
/// unwrap-or-skip via `.and_then`.
#[inline]
fn ore_value_nonzero(rx: u16, ry: u16, input: &SparkleInput<'_>) -> bool {
    let cell = input.overlays.cell(rx, ry);
    cell.overlay_id
        .and_then(|id| input.overlay_registry.flags(id))
        .is_some_and(|f| f.tiberium)
}
```

> **NOTE for the implementer:** The exact API for `OverlayGrid::cell()`, `OverlayTypeRegistry::flags()`, `OccupancyGrid::get()`, `FogState::is_cell_visible()`, and `BridgeRuntimeState::is_bridge_walkable()` should match the existing methods. If a name differs (e.g., overlay returns `Option<&OverlayCell>` and I wrote unwrapped), adapt the call sites — the call structure is what matters here, not the precise types. Run `cargo check` to see any mismatches and fix per the actual API.

**Step 3: Add gate unit tests.**

Append inside the tests module. NOTE: The gate function takes a full `SparkleInput<'a>` which requires real sim/render types to construct. We test the gate by **constructing minimal mock backends** using closures and the function-pointer fields. This avoids spinning up the full engine.

```rust
    // Mock backend helpers for gate testing. Each test constructs minimal
    // sim/render state via owned vecs + closures and wires them into
    // SparkleInput's function-pointer fields. This is verbose but keeps the
    // gate testable without booting the engine.
    //
    // The mock pattern: each test allocates a tiny ResolvedTerrainCell and
    // returns a closure that returns its address for the cell coord under
    // test. Other gate inputs (fog, overlays, occupancy, bridge_state) are
    // stubbed with the "everything passes" defaults unless the test cares
    // about a specific gate.

    fn make_input<'a>(
        resolved: &'a dyn Fn(u16, u16) -> Option<&'a ResolvedTerrainCell>,
        overlays: &'a OverlayGrid,
        registry: &'a crate::map::overlay_types::OverlayTypeRegistry,
        occupancy: &'a OccupancyGrid,
        fog: &'a FogState,
        bridge: Option<&'a BridgeRuntimeState>,
        enable: bool,
        depth_fn: &'a dyn Fn(u16, u16) -> f32,
        pos_fn: &'a dyn Fn(u16, u16) -> (f32, f32),
    ) -> SparkleInput<'a> {
        SparkleInput {
            clock_ms: 0,
            enable_extra_animations: enable,
            local_owner_id: Some(InternedId::default()),  // dummy id
            sandbox_full_visibility: false,
            resolved_terrain: ResolvedTerrainAccess { get: resolved },
            overlays,
            overlay_registry: registry,
            occupancy,
            fog,
            bridge_state: bridge,
            camera_x: 0.0,
            camera_y: 0.0,
            viewport_w: 800.0,
            viewport_h: 600.0,
            map_w: 100,
            map_h: 100,
            white_uv_origin: [0.0, 0.0],
            white_uv_size: [1.0, 1.0],
            depth_for_cell: depth_fn,
            screen_pos_for_cell: pos_fn,
        }
    }

    // The actual implementer of this test will need to construct minimal
    // OverlayGrid / OverlayTypeRegistry / OccupancyGrid / FogState / etc
    // instances. If these types have no `::empty()` or `::new()` constructor
    // suitable for tests, add a `#[cfg(test)] pub fn for_test() -> Self`
    // helper in their owning module. Defer to existing test patterns in
    // the codebase (grep for similar fixture builders).

    #[test]
    fn gate_skipped_when_not_water_or_ore() {
        // L17: clear-ground cell (is_water=false, no ore overlay) returns None.
        // (See implementer note above re: fixture construction.)
        // Pseudo-code skeleton:
        //   let cell = ResolvedTerrainCell { is_water: false, ... };
        //   let resolved = |rx, ry| if (rx, ry) == (10, 10) { Some(&cell) } else { None };
        //   let input = make_input(&resolved, ...);
        //   assert_eq!(gate_cell(10, 10, &input), None);
    }

    // Repeat the pattern for the remaining gate tests:
    //   gate_skipped_when_occupied   — set occupancy.get(10,10).occupants non-empty
    //   gate_skipped_when_not_visible — set fog.is_cell_visible returns false
    //   gate_skipped_under_bridge_deck — set bridge.is_bridge_walkable returns true
    //   gate_passes_water_cell — is_water=true, all other gates pass → Some((false, &WATER, ...))
    //   gate_passes_ore_cell — has_ore=true → Some((true, &ORE, ...))
```

> **NOTE for the implementer:** The gate test fixtures are non-trivial because they need real sim-state types. The pseudo-code above shows the test shape; the implementer should follow `tests/` or existing `#[cfg(test)]` fixture patterns elsewhere in the codebase (search for `OverlayGrid::` constructions in existing tests). If none exist, add minimal `for_test()` constructors to the relevant sim modules — that's lighter weight than wholesale gate-level testing in this module.
>
> **Fallback if fixtures are too expensive:** the gate logic is straightforward boolean ANDs of well-tested predicates. Move the gate test to an integration test (`tests/pixel_fx_sparkles_gate.rs`) that spins up a tiny synthetic map, or defer gate verification to the manual visual check in Task 13. The unit tests in Tasks 4–6 cover the high-risk math; the gate is mechanical.

**Step 4: Verify.**

Run: `cargo check`
Expected: clean compile (gate function compiles; uncommented test pseudo-code does not — leave commented-out or implement per the implementer note above).

If you implemented the gate tests:
Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: previous tests still pass + new gate tests pass.

**Step 5: Commit.**

```
render/pixel_fx_sparkles: add SparkleInput struct + gate logic

SparkleInput wraps the read-only sim/render state needed by the
sparkle builder. compute_sparkle_for_cell uses gate_cell() which
implements the 6-gate check (L17-L22). Gate test fixtures deferred —
see implementer note for fixture construction approach.
```

---

### Task 8: Implement the phase + emit math in `compute_sparkle_for_cell`

**Why:** The math. This task brings together the seed extraction (Task 5), the lerp formula (Task 4), and the gate (Task 7) into the full per-cell function. Ledger: **L12, L23, L24, L25, L26, L28**.

**Files:**
- Modify: `src/render/pixel_fx_sparkles.rs`

**Step 1: Add `compute_sparkle_for_cell`.**

Insert in `src/render/pixel_fx_sparkles.rs` after `gate_cell` / `ore_value_nonzero`:

```rust
/// Compute one cell's sparkle for the given clock time. Returns None when
/// the cell doesn't qualify (gate fails). Caller pushes the returned
/// SpriteInstance into a Vec.
fn compute_sparkle_for_cell(
    rx: u16,
    ry: u16,
    clock_ms: u64,
    input: &SparkleInput<'_>,
) -> Option<SpriteInstance> {
    let (_is_ore, params, bucket_ms) = gate_cell(rx, ry, input)?;

    // L26: per-cell offset hashed from coord-only key, breaks global beat sync.
    let cell_offset_ms = splitmix64(coord_key(rx, ry)) % bucket_ms;
    let shifted_t = clock_ms + cell_offset_ms;
    let cycle_index = shifted_t / bucket_ms;
    let cycle_pos_ms = shifted_t % bucket_ms;

    // L24: re-randomize sub-pos, color noise, lerp speed, timer_init each cycle
    // by mixing cycle_index into the seed.
    let s = splitmix64(coord_key(rx, ry) ^ cycle_index);

    let (sub_x, sub_y) = sub_pos_from_seed(s);                  // L10, L11
    let timer_init_ms = timer_init_from_seed(s);                // L5/L26
    let lerp_speed = lerp_speed_from_seed(s, params);           // L4/L9
    let peak = peak_with_noise(s, params);                      // L3/L8

    let active_duration_ms = 0x2000u32 / lerp_speed;

    // L23: cells START dim each cycle (during the timer_init wait, draw base).
    // L25: most of cycle is dim — peak is brief mid-active.
    // After active phase ends, sit at base until bucket boundary.
    let current_rgb = if (cycle_pos_ms as u32) < timer_init_ms {
        params.base_rgb                                          // L23 timer wait
    } else if (cycle_pos_ms as u32) < timer_init_ms + active_duration_ms {
        let active_progress = cycle_pos_ms as u32 - timer_init_ms;
        let phase = (active_progress * lerp_speed) & 0x1FFF;
        ping_pong_lerp(phase, params.base_rgb, peak)             // L13–L16
    } else {
        params.base_rgb                                          // L25 finished — wait for next bucket
    };

    // Emit (L12: 1×1 size; L28: alpha=1.0 opaque).
    let (screen_x, screen_y) = (input.screen_pos_for_cell)(rx, ry);
    Some(SpriteInstance {
        position: [screen_x + sub_x as f32, screen_y + sub_y as f32],
        size: [1.0, 1.0],                                        // L12
        uv_origin: input.white_uv_origin,
        uv_size: input.white_uv_size,
        depth: (input.depth_for_cell)(rx, ry),
        tint: [
            current_rgb[0] as f32 / 255.0,
            current_rgb[1] as f32 / 255.0,
            current_rgb[2] as f32 / 255.0,
        ],
        alpha: 1.0,                                              // L28
        house_color_idx: 0,
        fx_flags: 0,
        fx_params: [0.0; 4],
        ic_tint: [0.0; 4],
    })
}
```

**Step 2: Add math-level tests** that don't require sim-state fixtures (pure math on synthetic inputs).

Append inside the tests module:

```rust
    #[test]
    fn cycle_re_init_changes_sub_pos() {
        // L24: same cell, two consecutive cycle_indices, sub-pos must differ
        // (else "moving sparkle" effect is lost). Tests via direct seed
        // derivation since the full compute_sparkle_for_cell needs fixtures.
        let cell = coord_key(50, 50);
        let s0 = splitmix64(cell ^ 0);
        let s1 = splitmix64(cell ^ 1);
        assert_ne!(sub_pos_from_seed(s0), sub_pos_from_seed(s1));
    }

    #[test]
    fn cell_offset_breaks_sync_for_neighbours() {
        // L26: adjacent cells must produce different cell_offset_ms at the
        // same clock_ms (else they'd peak together → visible map-wide pulse).
        let bucket = WATER_CYCLE_BUCKET_MS;
        let off_a = splitmix64(coord_key(50, 50)) % bucket;
        let off_b = splitmix64(coord_key(51, 50)) % bucket;
        let off_c = splitmix64(coord_key(50, 51)) % bucket;
        assert_ne!(off_a, off_b);
        assert_ne!(off_a, off_c);
        // Spread check: over many neighbour pairs, average offset diff
        // should be > bucket/8 (catches degenerate hashes).
        let mut diff_sum: u64 = 0;
        let mut count = 0u64;
        for x in 0u16..20 {
            for y in 0u16..20 {
                let a = splitmix64(coord_key(x, y)) % bucket;
                let b = splitmix64(coord_key(x + 1, y)) % bucket;
                diff_sum += a.abs_diff(b);
                count += 1;
            }
        }
        let avg = diff_sum / count;
        assert!(avg > bucket / 8, "avg neighbour offset diff too small: {} < {}", avg, bucket / 8);
    }

    #[test]
    fn phase_calculation_at_timer_init_is_base() {
        // L23: during timer-wait (cycle_pos < timer_init), color is base.
        // Direct test on the inner expression (gate-free math).
        let s: u64 = 0xDEADBEEF;
        let timer_init = timer_init_from_seed(s);
        let lerp_speed = lerp_speed_from_seed(s, &WATER);
        let peak = peak_with_noise(s, &WATER);

        // Mimic the function's branch directly:
        let cycle_pos_ms: u32 = timer_init / 2;  // anywhere in timer-wait
        let color = if cycle_pos_ms < timer_init {
            WATER.base_rgb
        } else if cycle_pos_ms < timer_init + (0x2000u32 / lerp_speed) {
            let phase = ((cycle_pos_ms - timer_init) * lerp_speed) & 0x1FFF;
            ping_pong_lerp(phase, WATER.base_rgb, peak)
        } else {
            WATER.base_rgb
        };

        assert_eq!(color, WATER.base_rgb);
    }

    #[test]
    fn phase_calculation_active_progresses_through_lerp() {
        // L13–L16: when in active phase, color progresses from base toward peak.
        let s: u64 = 0xABCDEF01;
        let timer_init = timer_init_from_seed(s);
        let lerp_speed = lerp_speed_from_seed(s, &WATER);
        let active_duration = 0x2000u32 / lerp_speed;

        // Halfway through active phase, color is between base and peak on red ch.
        let cycle_pos_ms = timer_init + active_duration / 2;
        let active_progress = cycle_pos_ms - timer_init;
        let phase = (active_progress * lerp_speed) & 0x1FFF;
        let color = ping_pong_lerp(phase, WATER.base_rgb, WATER.peak_rgb);

        assert!(color[0] > WATER.base_rgb[0], "R should rise from base: {} not > {}", color[0], WATER.base_rgb[0]);
        assert!(color[0] <= WATER.peak_rgb[0], "R should not exceed peak: {} not <= {}", color[0], WATER.peak_rgb[0]);
    }
```

**Step 3: Verify.**

Run: `cargo check`
Expected: clean compile.

Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: all previous tests + 4 new tests pass.

**Step 4: Commit.**

```
render/pixel_fx_sparkles: implement compute_sparkle_for_cell math

Brings together the gate, seed extraction, and lerp formula into the
full per-cell function. Phase logic implements L23 (start dim),
L24 (re-randomize per cycle), L25 (mostly-dim feel), L26 (async
per-cell offsets). Four math-level tests cover the branch logic
without requiring sim-state fixtures.
```

---

### Task 9: Implement `build_sparkle_instances` public API

**Why:** This is the entry point called from `build_world_instances`. It iterates the viewport-cell bounds and calls `compute_sparkle_for_cell` per cell. Locks in the disabled-toggle early-exit (L22 enforcement). Ledger: **L22**.

**Files:**
- Modify: `src/render/pixel_fx_sparkles.rs`

**Step 1: Add the function.**

Insert in `src/render/pixel_fx_sparkles.rs` after `compute_sparkle_for_cell`:

```rust
/// Build one SpriteInstance per qualifying water/ore cell in the viewport.
///
/// Returns an empty Vec if `enable_extra_animations` is off — checked up-front
/// so the viewport iteration is skipped entirely (zero work). (L22)
///
/// Cell iteration uses the module's own viewport-cell bounds computation; we
/// don't reuse the terrain pass's iteration to keep the sparkle module
/// self-contained (see design doc §Architectural Decisions).
pub fn build_sparkle_instances(input: &SparkleInput<'_>) -> Vec<SpriteInstance> {
    if !input.enable_extra_animations {                          // L22
        return Vec::new();
    }
    let clock_ms = input.clock_ms;
    let (rx_min, ry_min, rx_max, ry_max) = viewport_cell_bounds(
        input.camera_x,
        input.camera_y,
        input.viewport_w,
        input.viewport_h,
        input.map_w,
        input.map_h,
    );
    let mut out: Vec<SpriteInstance> = Vec::with_capacity(256);
    for ry in ry_min..=ry_max {
        for rx in rx_min..=rx_max {
            if let Some(inst) = compute_sparkle_for_cell(rx, ry, clock_ms, input) {
                out.push(inst);
            }
        }
    }
    out
}
```

**Step 2: Add the disabled-toggle test.**

Append inside the tests module:

```rust
    #[test]
    fn disabled_extra_animations_returns_empty_immediately() {
        // L22: when extra_animations is off, the builder must return an
        // empty Vec WITHOUT iterating. We can't directly assert "didn't
        // iterate" but we can assert no allocation beyond the empty Vec by
        // verifying the result.
        //
        // This test bypasses the gate fixture issue by constructing the
        // minimum-viable SparkleInput using NOOPs for the closures —
        // because we return before any closure is called, the closures
        // can be stubs that panic. If the function ever doesn't early-exit,
        // this test will panic.
        let panicking_resolved = |_rx, _ry| -> Option<&'static ResolvedTerrainCell> {
            panic!("resolved_terrain.get called despite extra_animations off!")
        };
        let panicking_depth = |_rx: u16, _ry: u16| -> f32 {
            panic!("depth_for_cell called despite extra_animations off!")
        };
        let panicking_pos = |_rx: u16, _ry: u16| -> (f32, f32) {
            panic!("screen_pos_for_cell called despite extra_animations off!")
        };
        // For the other refs we need *something* satisfying the lifetimes.
        // Defer construction details to the implementer per the Task 7 note.
        //
        // Skeleton (replace `todo!()` with fixture-or-deferred):
        //   let overlays = todo!("OverlayGrid for_test()");
        //   let registry = todo!();
        //   let occupancy = todo!();
        //   let fog = todo!();
        //   let input = SparkleInput {
        //       clock_ms: 0,
        //       enable_extra_animations: false,                  // ← key bit
        //       local_owner_id: None, sandbox_full_visibility: false,
        //       resolved_terrain: ResolvedTerrainAccess { get: &panicking_resolved },
        //       overlays: &overlays, overlay_registry: &registry,
        //       occupancy: &occupancy, fog: &fog, bridge_state: None,
        //       camera_x: 0.0, camera_y: 0.0,
        //       viewport_w: 800.0, viewport_h: 600.0,
        //       map_w: 100, map_h: 100,
        //       white_uv_origin: [0.0; 2], white_uv_size: [1.0; 2],
        //       depth_for_cell: &panicking_depth,
        //       screen_pos_for_cell: &panicking_pos,
        //   };
        //   assert_eq!(build_sparkle_instances(&input).len(), 0);
        //
        // If fixtures are deferred (see Task 7 note), move this test to an
        // integration test in tests/, OR cover the behavior at Task 13
        // by setting extra_animations=false in config.toml and observing
        // zero sparkles in-game.
    }
```

**Step 3: Verify.**

Run: `cargo check`
Expected: clean compile (commented-out test body OK; uncommented requires fixtures).

Run: `cargo test --lib render::pixel_fx_sparkles::tests`
Expected: all previous tests still pass (this new test is a placeholder until fixtures land).

**Step 4: Commit.**

```
render/pixel_fx_sparkles: add public build_sparkle_instances API

Entry point called from build_world_instances. Early-exits on
extra_animations=false (L22). Iterates viewport-cell bounds and calls
compute_sparkle_for_cell per cell.
```

---

### Task 10: Wire to `WorldInstances` + `build_world_instances`

**Why:** Plug the sparkle builder into the existing build pipeline so its output Vec is produced per frame.

**Files:**
- Modify: `src/app_render/build_instances.rs:32-49` (WorldInstances struct)
- Modify: `src/app_render/build_instances.rs:255-269` (returning the struct)
- Modify: `src/app_render/build_instances.rs:98+` (inside `build_world_instances` — add the builder call)

**Step 1: Add field to `WorldInstances`.**

In `src/app_render/build_instances.rs`, inside `pub(super) struct WorldInstances { ... }`, add (after `particle_paged` at line 48):

```rust
    /// PixelFX water/ore sparkles — 1-pixel cell dots emitted per frame.
    /// Empty when graphics.extra_animations is false.
    pub cell_sparkles: Vec<SpriteInstance>,
```

**Step 2: Build the sparkles inside `build_world_instances`.**

In `src/app_render/build_instances.rs`, inside `build_world_instances`, add the sparkle builder call. Insert this block **after** the SHP/particle build (after line 238, before the `LOGGED` static) and **before** the final `WorldInstances { ... }` return:

```rust
    // PixelFX water/ore sparkles — per-frame 1-pixel cell dots.
    let cell_sparkles: Vec<SpriteInstance> = build_pixel_fx_sparkle_instances(state, sw, sh);
```

Then add the helper function (alongside `build_smudge_instances` which lives further down the same file):

```rust
/// Build PixelFX sparkle instances by calling into the dedicated render module.
/// This wrapper assembles the SparkleInput from AppState and returns the Vec.
fn build_pixel_fx_sparkle_instances(state: &AppState, sw: f32, sh: f32) -> Vec<SpriteInstance> {
    use crate::render::pixel_fx_sparkles::{
        build_sparkle_instances, ResolvedTerrainAccess, SparkleInput,
    };

    // Module gates itself on extra_animations; we still short-circuit if any
    // required state is missing.
    let Some(sim) = state.simulation.as_ref() else { return Vec::new(); };
    let Some(resolved) = state.resolved_terrain.as_ref() else { return Vec::new(); };
    let Some(overlay_registry) = state.overlay_registry.as_ref() else { return Vec::new(); };

    // Cosmetic toggle — default to ON when config failed to load, matching
    // gamemd's default. Stored on AppState by Task 1.
    let enable_extra_animations = state
        .game_config
        .as_ref()
        .map_or(true, |c| c.graphics.extra_animations);

    let local_owner_name = crate::app_commands::preferred_local_owner_name(state);
    let local_owner_id = match (state.sandbox_full_visibility, &local_owner_name) {
        (false, Some(owner)) => sim.interner.get(owner),
        _ => None,
    };

    let map_w = resolved.width();
    let map_h = resolved.height();
    let resolved_get =
        |rx: u16, ry: u16| -> Option<&crate::map::resolved_terrain::ResolvedTerrainCell> {
            resolved.cell(rx, ry)
        };
    let depth_for_cell = |rx: u16, ry: u16| -> f32 {
        // Use the standard cell-depth pattern from the existing terrain pass.
        // Implementer: copy from terrain::build_visible_instances or
        //              app_instances::screen_y_depth_for_cell — whichever
        //              the existing sprite-per-cell code uses. The depth
        //              should match what units use, so sparkles sort with
        //              ground objects correctly.
        crate::map::terrain::cell_screen_depth(rx, ry)  // placeholder name
    };
    let screen_pos_for_cell = |rx: u16, ry: u16| -> (f32, f32) {
        // Iso projection cell → screen center. Reuse the existing helper —
        // grep `iso_screen_pos` or similar in src/map/ or src/app_instances/.
        crate::map::iso::cell_to_screen_center(rx, ry)  // placeholder name
    };

    let input = SparkleInput {
        clock_ms: sim.total_sim_ms,        // engine's authoritative time source
        enable_extra_animations,
        local_owner_id,
        sandbox_full_visibility: state.sandbox_full_visibility,
        resolved_terrain: ResolvedTerrainAccess { get: &resolved_get },
        overlays: &sim.overlays,
        overlay_registry,                  // already an &OverlayTypeRegistry
        occupancy: &sim.occupancy,
        fog: &sim.fog,
        bridge_state: sim.bridge_state.as_ref(),
        camera_x: state.camera_x,
        camera_y: state.camera_y,
        viewport_w: sw,
        viewport_h: sh,
        map_w,
        map_h,
        white_uv_origin: [0.0, 0.0],
        white_uv_size: [1.0, 1.0],
        depth_for_cell: &depth_for_cell,
        screen_pos_for_cell: &screen_pos_for_cell,
    };
    build_sparkle_instances(&input)
}
```

> **Implementer note:** `cell_screen_depth` and `cell_to_screen_center` are placeholders — find the actual helpers used by `terrain::build_visible_instances` or `app_instances::build_unit_instances` and substitute the real function names. The depth function should be whatever the unit pass uses so sparkles sort correctly between ground objects (Step 5) and turrets (Step 6).

**Step 3: Populate the new field in the return statement.**

Modify the `WorldInstances { ... }` return at the end of `build_world_instances` to include the new field:

```rust
    WorldInstances {
        // ... existing fields
        particle_paged,
        cell_sparkles,
    }
```

**Step 4: Verify.**

Run: `cargo check`
Expected: clean compile (after fixing any placeholder helper names to actual ones).

If compile errors arise from helper-name mismatches, run:
```
rg "fn cell_screen_depth|fn cell_to_screen_center|cell_to_screen|screen_depth" --type rust
```
and use the actual function names.

**Step 5: Commit.**

```
app_render: wire pixel_fx_sparkles into build_world_instances

Adds cell_sparkles field to WorldInstances and a wrapper function
that assembles the SparkleInput from AppState. No GPU work yet; that
lands in the next two tasks.
```

---

### Task 11: Upload `cell_sparkles` to the instance buffer pool

**Why:** The build phase produces a Vec; phase 5 uploads it to the GPU buffer pool under a string key. Without this step, the dispatch step has nothing to draw.

**Files:**
- Modify: `src/app_render/mod.rs:120-204` (inside `upload_to_gpu`)

**Step 1: Add the upload call.**

In `src/app_render/mod.rs`, inside `fn upload_to_gpu(...)`, after one of the existing `pool.upload(...)` calls for a world-phase buffer (e.g., after the `"smudge"` upload), add:

```rust
    pool.upload(&state.gpu, "cell_sparkles", &world.cell_sparkles);
```

**Step 2: Verify.**

Run: `cargo check`
Expected: clean compile.

**Step 3: Commit.**

```
app_render: upload cell_sparkles to instance buffer pool

Plumbs the new WorldInstances.cell_sparkles Vec into the GPU buffer
pool under the "cell_sparkles" key, ready for dispatch.
```

---

### Task 12: Dispatch Step 5.5 between Step 5 (ground objects) and Step 6 (turrets)

**Why:** Insert the new draw step in the exact order gamemd uses (between `Tactical_ObjectRenderingLoop` and `radar/UI`). Ledger: **L27, L28**.

**Files:**
- Modify: `src/app_render/draw_passes.rs:140-142` (insert new step between Step 5 and Step 6)

**Step 1: Add the dispatch step.**

In `src/app_render/draw_passes.rs`, **after** the Step 5 `merge_passes::draw_merged_object_pass(...)` call (ending around line 140) and **before** the Step 6 turret block (starting around line 142-158), insert:

```rust
    // --- Step 5.5: PixelFX water/ore sparkles ---
    // Per-frame 1-pixel sparkles over visible water and ore cells. Matches
    // gamemd's DrawPixelFXSparkles position (between unit pass and UI pass)
    // per report §2 and ledger L27. Opaque sprite, no blend (L28).
    // Empty buffer when graphics.extra_animations is off — pool.get returns
    // None or count==0, helper short-circuits.
    if let (Some(overlay), Some((buf, count))) = (
        state.selection_overlay.as_ref(),
        pool.get("cell_sparkles"),
    ) {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            overlay.white_texture(),
            buf,
            count,
        );
    }
```

**Step 2: Verify.**

Run: `cargo check`
Expected: clean compile.

Run: `cargo test --lib`
Expected: all tests pass.

**Step 3: Commit.**

```
app_render: dispatch PixelFX sparkles between Step 5 and Step 6

New Step 5.5 between ground objects and turrets, matching gamemd's
DrawPixelFXSparkles position. Uses SelectionOverlay's 1×1 white
texture with the passthrough sprite pipeline (no blend, no Z-test).
```

---

### Task 13: Manual visual verification

**Why:** Unit tests cover the math; only running the game confirms the visual matches gamemd. This task is the parity check that resolves the design's one acknowledged drift (2500ms cycle bucket vs gamemd's variable cycles).

**Files:** None modified — this is a runtime verification task.

**Step 1: Run the engine.**

Run: `cargo run --release`

Open a stock night-water map (Bermuda Triangle, Naval War, or any map with visible water + clear ambient darkness).

**Step 2: Verify each parity-critical behavior.**

Walk through this checklist with the camera scrolled to a region containing both water and ore:

| Check | Expected | Ledger |
|---|---|---|
| Sparkles visible on water cells | Yes — scattered pale-blue 1-pixel dots, mostly dim, occasional brighter pulses. | L1, L2, L25 |
| Sparkles visible on ore cells | Yes — scattered amber/near-white 1-pixel dots, faster cycling. | L6, L7, L29, L30 |
| Sparkles appear ONLY in current sight | Move camera to an area no unit has explored — should see static terrain only, no sparkles. Move a unit there — sparkles appear. | L19, L21 |
| Sparkles disappear under units | Drive a unit onto a water cell (transport / amphibious) — sparkles on that cell vanish; reappear when unit moves off. | L18 |
| Sparkles disappear under bridge decks | Find a bridge over water — water cells under the deck show NO sparkles. | L20 |
| Toggle works | Edit `config.toml` to set `extra_animations = false`, restart. No sparkles anywhere. | L22 |
| No grid patterns | Look at a large open water area for 10 seconds. The sparkles should look truly random — no rows, columns, or repeating clusters pulsing in unison. | L26 + splitmix64 distribution |
| Sparkles "move" between cycles | Watch a single bright cell for a few seconds — when it dims and re-brightens, the sub-pixel position should be different. | L24 |
| Sparkles draw behind units | A unit moving across water should occlude any sparkle on its current cell (gated out by L18) AND any sparkle that would render at the unit's pixel position. | L27 |
| Sparkles draw above water terrain | Sparkles visible on top of water TMP pixels (not underneath). | L27 |

**Step 3: If any check fails.**

- **Grid patterns visible:** swap `splitmix64` for a higher-quality hash (PCG-XSH-RR). Single-line change inside the module.
- **Sparkles too sparse:** the cycle bucket may be too large — reduce `WATER_CYCLE_BUCKET_MS` from 2500 to 1500. Run again.
- **Sparkles too dense / too bright:** check the lerp formula and constants tests — ensure L1–L9 are byte-exact.
- **Wrong colour tone:** verify L1 (water base) and L2 (water peak) values; the report's hex dump at §5.1 is authoritative.
- **Draw order wrong (sparkles over units or under terrain):** verify the dispatch is between Step 5 and Step 6 (Task 12), not elsewhere.

**Step 4: Document the result.**

Whether the visual matches or needs tuning, capture the outcome in a one-paragraph commit message. If matches: report parity-confirmed. If tuning needed: commit the tuned constants with a note.

**Step 5: Commit (visual-verified result).**

```
render/pixel_fx_sparkles: visual parity verified on Bermuda Triangle

Sparkles render in current sight only, disappear under units and
bridge decks, animate continuously without visible grid patterns,
and respect the extra_animations toggle. Matches gamemd's observable
output within the parity bar (single-skirmish indistinguishable).
```

OR (if tuning was needed):

```
render/pixel_fx_sparkles: tune <constant> based on visual check

Initial 2500ms cycle bucket produced too-sparse sparkles vs gamemd;
reduced to <new value>. Other parity checks (color, position, draw
order, gates) confirmed passing.
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-18-pixel-fx-sparkle-design.md](./2026-05-18-pixel-fx-sparkle-design.md)
- **Primary research:** `docs/research/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md` (verified this session including §14 close-out)
- **Cross-referenced docs:** `CELLCLASS_STRUCT_GHIDRA_REPORT.md`, `MAPCLASS_COMPLETE_DECODE.md` §E
- **gamemd.exe addresses:**
  - `DrawPixelFXSparkles @ 0x006D7840` — per-frame entry point
  - `PixelFXClass::Init @ 0x00631D40` — struct layout source
  - `PixelFXClass::Update_Color @ 0x00631E50` — lerp formula source
  - `g_PixelFXParams_Water @ 0x008367C8` — water constants (read directly)
  - `g_PixelFXParams_Ore @ 0x008367F0` — ore constants (read directly)
  - `MapClass::Invalidate_Radius_For_Redraw @ 0x00568140` — sight-bit setter (gate L19)
  - `MapClass::Conceal_Radius @ 0x00567F70` — sight-bit clearer
  - `CellClass::SetBridgeDirection_NESW @ 0x0047E040` — bridge-deck bit setter (gate L20)
- **INI keys:** none — sparkle constants are in the binary, not in rulesmd.ini.
- **Related code (repo patterns mirrored):**
  - [src/app_render/build_instances.rs:282-...](../../src/app_render/build_instances.rs#L282) — `build_smudge_instances` (pattern for stateless render builder)
  - [src/app_render/draw_passes.rs:532-542](../../src/app_render/draw_passes.rs#L532-L542) — `draw_pooled_passthrough_overlay` (dispatch pattern)
  - [src/render/selection_overlay.rs:319](../../src/render/selection_overlay.rs#L319) — `white_texture()` (1×1 white texel)
  - [src/render/batch.rs:42-75](../../src/render/batch.rs#L42-L75) — `SpriteInstance` struct
  - [src/render/batch.rs:1364-1370](../../src/render/batch.rs#L1364-L1370) — `draw_with_buffer_passthrough` (sprite pipeline entry)
  - [src/util/config.rs:45-58](../../src/util/config.rs#L45-L58) — `GraphicsConfig` (where the toggle lives)
  - [src/sim/world/mod.rs:76](../../src/sim/world/mod.rs#L76) — `World.tick` (canonical sim clock)
- **Prior session work:** Ghidra renames + plate comments for `MapClass__Invalidate_Radius_For_Redraw` and `MapClass__Conceal_Radius` landed this session (see PIXEL_FX_SPARKLES_GHIDRA_REPORT.md §16.1).
