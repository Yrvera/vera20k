# Parachute SHP Rendering Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Render the PARACH SHP above each paradropped infantry during
descent — center-anchored on the GI's screen position, deploy-then-loop
animation, sorted in the GI's depth band, removed on landing or GI death.

**Architecture:** Mirrors `GarrisonMuzzleFlash` 1:1. New `ParachuteAnim`
struct in `sim/components.rs`, `Vec<ParachuteAnim>` on `AppState`,
polling-based lifecycle ticked once per render frame, sprite instances
emitted to the entity sprite layer per frame. Pure render addition; no
sim-side changes.

**Design Doc:** [docs/plans/2026-05-06-parachute-shp-rendering-design.md](2026-05-06-parachute-shp-rendering-design.md)

---

## Grounding Summary

- **Docs (`ra2-rust-game-docs/`):**
  [`PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md)
  (HIGH confidence, written this session) covers Rate semantics, ZAdjust
  math, AltPalette path, layer override for attached anims, anchor
  centering. References sibling reports
  `ANIM_CLASS_GHIDRA_REPORT.md` (full struct layout),
  `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` (palette §3.6, attachment
  §3.5), `LAYER_CLASS_GHIDRA_REPORT.md` (§3 GetLayer override).
- **Ghidra verification (this session):**
  - `AnimTypeClass::ReadINI` at `0x00427D00` → `internal_rate = 900 / INI_Rate`
  - `AnimClass::DrawIt` at `0x00422CA0` → depth formula
    `YDrawOffset + ZAdjust − Z_correction − 2`
  - `AnimClass::GetLayer` at `0x00424cb0` → owner-attached anims forced
    to Layer 2 (Ground)
- **Repo pattern mirrored:** `GarrisonMuzzleFlash` —
  - Struct in [src/sim/components.rs:510](../../src/sim/components.rs#L510)
  - Lifecycle in [src/app_building_anim.rs:495+](../../src/app_building_anim.rs#L495)
    (`tick_garrison_muzzle_flashes`)
  - Render in [src/app_instances/overlays.rs:508](../../src/app_instances/overlays.rs#L508)
    (`build_garrison_muzzle_flash_instances`)
  - Render call site in [src/app_render/build_instances.rs:200](../../src/app_render/build_instances.rs#L200)
- **INI keys driving behavior:**
  - `[General] Parachute=PARACH` (rulesmd.ini) — section name
  - `[PARACH] Rate=400, LoopStart=20, LoopEnd=39, LoopCount=30,
    AltPalette=yes, ZAdjust=-10` (artmd.ini:15642-15648)
  - Existing helper `art_rate_to_delay_ms` at
    [src/rules/art_data.rs:134](../../src/rules/art_data.rs#L134)
    converts Rate=400 → 133ms/frame correctly.
- **Already-wired infrastructure:**
  - `[General] ParachuteMaxFallRate`, `ParaDropPlane`, paradrop lists,
    paradrop radius — all parsed
    ([ruleset.rs:692-698](../../src/rules/ruleset.rs#L692))
  - `SimSoundEvent::ChuteSound` emitted from
    `aircraft/drop_payload.rs:175` and `aircraft/mod.rs:649-656`,
    translated by app at `app_sim_tick.rs:487`
- **Still unknown after grounding:**
  - **PARACH SHP frame count** (P14): inferred 40 (indices 0-39) from
    LoopEnd=39 + community refs. Plan uses `LoopEnd + 1 = 40` as the
    end-frame bound, matching gamemd's clamp invariant. To upgrade to
    HIGH: hex-dump PARACH.SHP.

## Key Technical Decisions

- **Render state lives in `AppState`, not `Simulation`.** `parachute_anims`
  is render-only; sim is ignorant of the chute visual. **Confidence:**
  high. **Source:** mirrors `GarrisonMuzzleFlash` (also in `AppState`); design
  doc §Determinism.
- **Polling-based lifecycle, not sim events.** Spawn for any entity with
  `parachute_state.is_some()` not yet tracked; despawn on landing or
  death. **Confidence:** high. **Source:** chute lifecycle is bound by
  entity state, not discrete events; design doc Approach 1.
- **`end_frame = LoopEnd + 1` from art.ini, no SHP introspection at
  parse time.** PARACH's `End` field auto-detects from SHP frame count
  in gamemd, but for our wraparound logic, `LoopEnd + 1 = 40` produces
  identical behavior (frames 0..39 play, then 20..39 loop). Avoids
  coupling rules parsing to atlas loading. **Confidence:** medium.
  **Source:** PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md §13 Q2; works
  because gamemd clamps `LoopEnd <= End`.
- **Frame timing via `art_rate_to_delay_ms(Rate)`.** Project helper
  already implements `(900 / Rate) * 1000 / 15` per gamemd.
  **Confidence:** high. **Source:** verified at AnimTypeClass::ReadINI
  0x00427D00 + project art_data.rs:134.
- **Anchor: sprite center at entity screen position via SHP atlas's
  pre-baked offsets.** Same convention as `GarrisonMuzzleFlash`:
  `entry.offset_x/offset_y` from the atlas handle the center-anchor
  math. **Confidence:** high. **Source:** verified gamemd flag 0x200
  semantics + repo pattern.
- **Depth: chute draws on top of GI body via small fixed epsilon.**
  ZAdjust=-10 leptons in gamemd is a fudge factor; in our depth-buffer
  rendering, a small epsilon (~0.0005) below the GI's depth value
  achieves the same effect (lower depth = closer to camera = on top).
  **Confidence:** medium. **Source:** existing
  `compute_sprite_depth_params` math (helpers.rs:43-53) shows depth is
  in `[0.001, 0.999]` with Z_bias of `0.0001` per Z step. Pick epsilon
  small but non-zero.
- **Palette: chute uses unit/object palette, NOT owner-tinted.** AltPalette=yes
  in gamemd selects `g_ColorSchemeArray[0]->ConvertPalette` which is a
  fixed unit-flavored palette. **Confidence:** high. **Source:**
  ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md §3.6.

## Open Questions

### Resolved During Planning

- **Where to parse [PARACH] section?** Resolved: alongside other
  general-rules anim parsing in `ruleset.rs` `resolve_art_rates`
  / equivalent late-pass. Already a precedent for parsing referenced
  art.ini sections (warp_in, warp_out, wake, fire types).
- **What's the wraparound bound?** Resolved: `LoopEnd + 1`. Avoids
  coupling rules parser to SHP atlas load timing.
- **How to resolve `Parachute=` SHP name?** Resolved: not yet parsed.
  The existing paradrop block in
  [ruleset.rs:692-698](../../src/rules/ruleset.rs#L692) handles
  `ParachuteMaxFallRate`, `ParadropRadius`, `ParaDropPlane` — but NOT
  `Parachute=`. Plan adds it.

### Deferred to Implementation

- **PARACH SHP atlas registration.** Don't know yet whether PARACH is
  already loaded as a sprite into the project's sprite atlas. Task 8
  is "verify and register if needed." If PARACH isn't loaded, the
  render path will silently no-op (no-sprite warning per
  `feedback_silent_render_failures`); fix is to register it in the
  effect/anim atlas init.
- **Exact depth epsilon value.** Picked 0.0005 as starting value; may
  need tuning (e.g., 0.001) if visible z-fighting occurs. Test in-game
  in Task 9.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/ruleset.rs` | Add `parachute_render: Option<ParachuteRenderConfig>` field on `GeneralRules` (or sibling), parse `[General] Parachute=` + `[PARACH]` artmd.ini section |
| Modify | `src/sim/components.rs` | Add `ParachuteAnim` struct |
| Modify | `src/app.rs` | Add `parachute_anims: Vec<ParachuteAnim>` to `AppState` + init |
| Create | `src/app_chute_anim.rs` | `tick_parachute_anims` lifecycle (spawn/despawn/advance) + tests |
| Modify | `src/lib.rs` | `mod app_chute_anim;` |
| Modify | `src/app_sim_tick.rs` | Call `tick_parachute_anims` once per render frame |
| Modify | `src/app_instances/overlays.rs` | Add `build_parachute_instances` |
| Modify | `src/app_render/build_instances.rs` | Wire `build_parachute_instances` into render pass |

## Interface Changes

- **`AppState.parachute_anims: Vec<ParachuteAnim>`** — new field. Read by
  `tick_parachute_anims` and `build_parachute_instances` only.
- **`ParachuteAnim`** — new public struct in `sim/components.rs`.
  Consumers: `app_chute_anim.rs`, `app_instances/overlays.rs`.
- **`ParachuteRenderConfig`** — new struct on `GeneralRules` (or whatever
  rules-side container is appropriate). Loaded once at startup;
  read-only thereafter.
- No `Command::*` changes. No sim-side changes.

## Sim Checklist

This plan does NOT modify `sim/`. Verify:

- [x] No new sim state introduced (`parachute_anims` lives in `AppState`)
- [x] No tick-ordering changes
- [x] No new fixed-point math (render only; render f32 is allowed)
- [x] No new dependencies from sim/ on render/ui/sidebar/audio/net
- [x] State hash unchanged

## Risk Areas

- **PARACH SHP not in sprite atlas at startup.** If the existing atlas
  initialization doesn't include PARACH, the render path will silently
  drop the sprite. Mitigation: Task 8 verifies via grep + if missing,
  registers it in the existing atlas-load code.
- **Depth ordering with GI body.** Both are in Layer 2; chute must
  z-sort above the body. Mitigation: explicit small epsilon in depth
  computation; in-game verification in Task 9.
- **Atlas overflow.** PARACH adds N frames to the atlas. Multi-page
  atlas support already exists per memory entry `feedback_multi_atlas`,
  so this should be transparent.
- **Frame timing parity.** Rate=400 → 133ms/frame is verified; deploy
  phase = 20 frames × 133ms ≈ 2.67 seconds. Visible drift means the
  conversion is off — check `art_rate_to_delay_ms` actually returns 133
  for input 400.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Rate=400 → 133ms/frame conversion | Wrong frame timing = visibly wrong chute deploy speed in every paradrop. | Unit test in Task 1: `art_rate_to_delay_ms(400) == 133` |
| Task 1 | LoopStart=20, LoopEnd=39 produce frames 0-39 in first cycle, 20-39 thereafter | Wrong loop bounds = chute deploy phase plays wrong number of frames or skips loop entirely. | Unit test in Task 4: tick advances frame 0..39 then wraps to 20 |
| Task 6 | Sprite center-anchored on GI's screen position (not corner-anchored) | Wrong anchor = chute floats off to one side of the GI. | In-game verification Task 9: chute canopy directly above GI head |
| Task 6 | Chute uses unit/Convert palette, NOT owner-tinted | gamemd: chutes are NOT owner-tinted; all chutes look identical regardless of who dropped them. | In-game verification Task 9: drop Soviet + Allied paradrops in same frame, chutes look the same |
| Task 6 | Chute renders ABOVE GI body (lower depth value) | Wrong depth = chute draws BEHIND the GI body, hidden. | In-game verification Task 9: chute canopy visible above GI |
| Task 4 | Despawn on landing | Stale chute hovers over a standing GI = visible bug. | Unit test in Task 4 + in-game verification Task 9 |
| Task 4 | Despawn on GI death (e.g., AA fire) | Phantom chute floating in air = visible bug. | Unit test in Task 4 + in-game (manual: shoot down a paradrop) |
| Task 9 | Chute looping motion (frames 20-39 cycling) | Static chute = obvious parity break. | In-game observation: chute canopy "swings" during descent |
| Task 9 | Single-facing chute (same SHP regardless of orientation) | Multi-facing rendering = wrong (anims have no facings). | Verified by impl: facing always 0 in `ShpSpriteKey` |

---

## Tasks

### Task 1: Add `ParachuteRenderConfig` parsing in `ruleset.rs`

**Why:** Foundation — provides the static config (rate_ms, loop_start,
loop_end, end_frame, z_adjust, alt_palette, shp_name) that the tick
function and renderer both consume. No dependents until Tasks 4 and 6.

**Files:**
- Modify: `src/rules/ruleset.rs` — add struct, add field on `GeneralRules`,
  add parser pass.

**Pattern:** `resolve_art_rates` at
[src/rules/ruleset.rs:912](../../src/rules/ruleset.rs#L912) — late-pass
that reads art.ini sections referenced by general rules. Mirror that
shape.

**Step 1: Define the struct** (top of `ruleset.rs` near other
config structs, around line 100-150 area where `AnimRef` (the
world-effect anim reference struct) lives):

```rust
/// Static art.ini metadata for the `[General] Parachute=` SHP.
/// Loaded once at startup; consumed by app_chute_anim and the parachute
/// render path. Pure data; no sim-side dependencies.
#[derive(Debug, Clone)]
pub struct ParachuteRenderConfig {
    /// SHP section name from `[General] Parachute=` (e.g., "PARACH"). Uppercased.
    pub shp_name: String,
    /// ms per anim frame. Computed via `art_rate_to_delay_ms(Rate=)`.
    /// For Rate=400 this is 133.
    pub rate_ms: u32,
    /// Frame to wrap to after `frame >= end_frame`. From art.ini `LoopStart=`.
    pub loop_start: u16,
    /// Wraparound bound (exclusive). Set to `LoopEnd + 1` from art.ini.
    /// Frames 0..end_frame play once on first cycle, then wraparound to
    /// loop_start.
    pub end_frame: u16,
    /// Depth-sort offset (gamemd leptons, signed; -10 for PARACH).
    /// Used by the renderer to put the chute in the same depth band as
    /// the GI body, slightly on top.
    pub z_adjust: i16,
    /// Whether to use the unit/Convert palette instead of the standard
    /// anim palette. From art.ini `AltPalette=`. NOT owner-tinted.
    pub alt_palette: bool,
}
```

**Step 2: Add field to `GeneralRules`**

In the `GeneralRules` struct (around line 140), add after the existing
`paradrop_*` fields:

```rust
    /// Parsed render config for the parachute SHP (from `[General] Parachute=`).
    /// `None` if the key is unset OR if the referenced art.ini section is
    /// missing. Render path is a no-op when this is `None`.
    pub parachute_render: Option<ParachuteRenderConfig>,
```

**Step 3: Add to `Default::default()` impl**

In the `impl Default for GeneralRules` block (around line 453), add:

```rust
            parachute_render: None,
```

**Step 4: Add parser**

In the same `resolve_art_rates` function (line 905+) where
warp/wake/fire rates are loaded from art.ini, append this block. Use
`art_rate_to_delay_ms` from `art_data.rs`:

```rust
        // Parachute render config: [General] Parachute= names the section in
        // artmd.ini that holds the chute SHP's animation metadata.
        self.general.parachute_render = self
            .general
            .parachute_shp
            .as_deref()
            .and_then(|shp_name| {
                let section = art_ini.section(shp_name)?;
                let shp_owned = shp_name.to_uppercase();
                let rate = section.get_i32("Rate").unwrap_or(1);
                let rate_ms = crate::rules::art_data::art_rate_to_delay_ms(rate);
                let loop_start = section.get_i32("LoopStart").unwrap_or(0).max(0) as u16;
                let loop_end = section.get_i32("LoopEnd").unwrap_or(0).max(0) as u16;
                let end_frame = loop_end.saturating_add(1);
                let z_adjust = section.get_i32("ZAdjust").unwrap_or(0) as i16;
                let alt_palette = section.get_bool("AltPalette").unwrap_or(false);
                Some(ParachuteRenderConfig {
                    shp_name: shp_owned,
                    rate_ms,
                    loop_start,
                    end_frame,
                    z_adjust,
                    alt_palette,
                })
            });
        if let Some(ref pc) = self.general.parachute_render {
            log::info!(
                "Parachute render config loaded: shp={} rate_ms={} loop_start={} end_frame={} z_adjust={} alt_palette={}",
                pc.shp_name, pc.rate_ms, pc.loop_start, pc.end_frame, pc.z_adjust, pc.alt_palette,
            );
        } else {
            log::warn!(
                "Parachute render config NOT loaded (missing [General] Parachute= or referenced art.ini section)"
            );
        }
```

**Step 5: Add `parachute_shp` field to whatever holds the `[General]`
section's `Parachute=` value.**

If `GeneralRules` already has the parsed `[General]` raw section
(`general` field in step 4), use it. If not, this requires an extra
field. Check what's there now by reading
[src/rules/ruleset.rs:692-698](../../src/rules/ruleset.rs#L692):

The existing block parses `paradrop_aircraft_type` from
`general.get("ParaDropPlane")`. So `general` is the parsed
`IniSection` available in the constructor. Add to the constructor (around
line 692):

```rust
        // [General] Parachute= → uppercase or None if unset/empty.
        let parachute_shp_name: Option<String> = general
            .get("Parachute")
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty());
```

Then save it onto `GeneralRules`:

```rust
            parachute_shp: parachute_shp_name,
```

And add the field to the struct (alongside `paradrop_aircraft_type`):

```rust
    /// Parsed `[General] Parachute=` value (uppercased SHP name, e.g. "PARACH").
    /// `None` if unset or empty. Used by `resolve_art_rates` to
    /// resolve `parachute_render`.
    pub parachute_shp: Option<String>,
```

And to `Default::default()`:

```rust
            parachute_shp: None,
```

**Step 6: Add tests**

At the bottom of `ruleset.rs` test module:

```rust
    #[test]
    fn parses_parachute_render_config_from_artmd() {
        // Use the embedded artmd.ini test fixture if available, or a
        // minimal inline INI. Mirror existing test pattern for warp anims.
        let rules_text = "\
[General]
Parachute=PARACH
";
        let art_text = "\
[PARACH]
Rate=400
LoopStart=20
LoopEnd=39
LoopCount=30
AltPalette=yes
ZAdjust=-10
";
        let rules_ini = IniFile::from_str(rules_text);
        let art_ini = IniFile::from_str(art_text);
        let mut rs = RuleSet::default();
        rs.load_general_from_ini(&rules_ini); // or whatever existing entry point
        rs.resolve_art_rates(&art_ini);
        let pc = rs.general.parachute_render.as_ref().expect("parachute_render");
        assert_eq!(pc.shp_name, "PARACH");
        assert_eq!(pc.rate_ms, 133); // 900 / 400 = 2; 2 * 1000 / 15 = 133
        assert_eq!(pc.loop_start, 20);
        assert_eq!(pc.end_frame, 40); // LoopEnd + 1
        assert_eq!(pc.z_adjust, -10);
        assert!(pc.alt_palette);
    }

    #[test]
    fn parachute_render_none_when_general_parachute_unset() {
        let rules_text = "[General]\nFlightLevel=1500\n";
        let art_text = "[PARACH]\nRate=400\n";
        let rules_ini = IniFile::from_str(rules_text);
        let art_ini = IniFile::from_str(art_text);
        let mut rs = RuleSet::default();
        rs.load_general_from_ini(&rules_ini);
        rs.resolve_art_rates(&art_ini);
        assert!(rs.general.parachute_render.is_none());
    }
```

(Adjust function names like `load_general_from_ini` to match the actual
constructor entry point. If `resolve_art_rates` doesn't exist by
that exact name in the file, find the actual name via grep — it's the
function around line 905 that calls `rate_from_section`.)

**Step 7: Verify**

Run:
```
cargo test --lib parses_parachute_render_config parachute_render_none
```
Expected: 2 tests pass.

Run:
```
cargo check --lib
```
Expected: PASS, no errors.

**Step 8: Commit**

Message: `rules: parse [General] Parachute= + [PARACH] art.ini metadata`

---

### Task 2: Add `ParachuteAnim` struct to `sim/components.rs`

**Why:** Foundation type, no dependents until Task 3. Mirrors
`GarrisonMuzzleFlash` shape.

**Files:**
- Modify: `src/sim/components.rs` (after `WorldEffect`, around line ~560).

**Pattern:** `GarrisonMuzzleFlash` at line 510. Same field shape:
target_id (entity stable_id), frame, rate_ms, elapsed_ms.

**Step 1: Add the struct**

After `WorldEffect`'s closing `}` (around line 560), add:

```rust
/// A parachute SHP rendering above a paradropped infantry during descent.
///
/// Spawned by `tick_parachute_anims` when an entity gains
/// `parachute_state.is_some()`. Removed when the entity lands
/// (`parachute_state.is_none()`) or dies. The chute follows the entity's
/// world position via `target_id` lookup; no separate altitude state.
///
/// Frame advancement follows gamemd's `AnimClass::AI` loop semantics:
/// frames `0..end_frame` play once on the first cycle (deploy phase implicit
/// in `0..loop_start`); on `frame >= end_frame`, wrap to `loop_start` (loop
/// phase). For PARACH: deploy = frames 0-19, loop = frames 20-39.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParachuteAnim {
    /// Stable ID of the descending entity. Render looks up screen position
    /// each frame via `sim.entities.get(target_id)`.
    pub target_id: u64,
    /// Current animation frame (0..end_frame).
    pub frame: u16,
    /// Frame to wrap to on `frame >= end_frame`. Copied from
    /// ParachuteRenderConfig at spawn time.
    pub loop_start: u16,
    /// Wraparound bound (exclusive). Copied from ParachuteRenderConfig.
    pub end_frame: u16,
    /// ms per frame. Copied from ParachuteRenderConfig.
    pub rate_ms: u32,
    /// Accumulated ms since last frame advance.
    pub elapsed_ms: u32,
}
```

**Step 2: Verify**

Run:
```
cargo check --lib
```
Expected: PASS. Dead-code warning expected (no consumers yet —
acceptable).

**Step 3: Commit**

Message: `sim/components: add ParachuteAnim struct`

---

### Task 3: Add `parachute_anims` field to `AppState`

**Why:** Storage for `ParachuteAnim` records. Required by Task 4 (tick)
and Task 6 (render).

**Files:**
- Modify: `src/app.rs` — struct decl + init.

**Pattern:** Mirror `garrison_muzzle_flashes` field at
[src/app.rs](../../src/app.rs) (location verified earlier in this session
via grep `garrison_muzzle_flashes` → `app.rs`).

**Step 1: Find the existing field**

Grep `garrison_muzzle_flashes` in `src/app.rs`. There are two hits:
- Field decl (around line ~280, with the comment "Active garrison muzzle
  flash animations.")
- Init in `Default` impl (around line 635)

**Step 2: Add the new field after `garrison_muzzle_flashes`**

Field decl:

```rust
    /// Active parachute animations, one per descending paradropped infantry.
    /// Polling-based lifecycle: spawned when an entity gains parachute_state
    /// in the sim, removed on landing or death. Render-only; not snapshotted.
    pub(crate) parachute_anims: Vec<crate::sim::components::ParachuteAnim>,
```

Init in `Default::default()` (right after `garrison_muzzle_flashes: Vec::new(),`):

```rust
            parachute_anims: Vec::new(),
```

**Step 3: Verify**

Run:
```
cargo check --lib
```
Expected: PASS. Dead-code warning still expected.

**Step 4: Commit**

Message: `app: add parachute_anims field to AppState`

---

### Task 4: Implement `tick_parachute_anims` lifecycle

**Why:** The core spawn/despawn/advance logic. Required by Task 5
(render-loop integration).

**Files:**
- Create: `src/app_chute_anim.rs`
- Modify: `src/lib.rs` — add `mod app_chute_anim;`

**Pattern:** `tick_garrison_muzzle_flashes` at
[src/app_building_anim.rs:495](../../src/app_building_anim.rs#L495).
Three-phase shape: spawn from external state, despawn from terminal
condition, advance frame.

**Step 1: Add module declaration**

In `src/lib.rs`, find the `pub(crate) mod app_*` block (the alphabetical
block including `app_building_anim`, `app_camera`, etc.) and add:

```rust
pub(crate) mod app_chute_anim;
```

**Step 2: Create the module file**

Create `src/app_chute_anim.rs` with:

```rust
//! Parachute SHP animation lifecycle — spawn/despawn/advance per render frame.
//!
//! Mirrors `tick_garrison_muzzle_flashes` (app_building_anim.rs) for an
//! attached SHP anim, but with a polling-based lifecycle bound by the target
//! entity's `parachute_state`, not a fixed frame count. Despawn happens on
//! landing (parachute_state.is_none()) or entity death (target missing).
//!
//! Render-only state; lives on AppState, not in Simulation.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::sim::components::ParachuteAnim;

/// Per-render-frame lifecycle pass.
///
/// Phases:
/// 1. **Despawn:** drop anims whose target entity is missing or whose
///    `parachute_state.is_none()`.
/// 2. **Spawn:** scan entities for any with `parachute_state.is_some()` not
///    yet tracked; push a new anim using `ParachuteRenderConfig`.
/// 3. **Advance:** accumulate `elapsed_ms`; advance `frame` per `rate_ms`;
///    wrap on `frame >= end_frame` to `loop_start`.
pub(crate) fn tick_parachute_anims(state: &mut AppState, dt_ms: u32) {
    // Bail if the render config or simulation is not yet loaded.
    let Some(rules) = state.rules.as_ref() else {
        state.parachute_anims.clear();
        return;
    };
    let Some(config) = rules.general.parachute_render.as_ref() else {
        state.parachute_anims.clear();
        return;
    };
    let Some(sim) = state.simulation.as_ref() else {
        state.parachute_anims.clear();
        return;
    };

    // Phase 1: despawn.
    state.parachute_anims.retain(|anim| {
        match sim.entities.get(anim.target_id) {
            Some(entity) => entity.parachute_state.is_some(),
            None => false,
        }
    });

    // Phase 2: spawn for any descending entity not yet tracked.
    // Collect target_ids first to avoid borrow conflict on state.parachute_anims.
    let new_targets: Vec<u64> = sim
        .entities
        .values()
        .filter(|e| e.parachute_state.is_some())
        .map(|e| e.stable_id)
        .filter(|sid| {
            !state
                .parachute_anims
                .iter()
                .any(|a| a.target_id == *sid)
        })
        .collect();

    for target_id in new_targets {
        state.parachute_anims.push(ParachuteAnim {
            target_id,
            frame: 0,
            loop_start: config.loop_start,
            end_frame: config.end_frame,
            rate_ms: config.rate_ms,
            elapsed_ms: 0,
        });
    }

    // Phase 3: advance frames.
    for anim in &mut state.parachute_anims {
        if anim.rate_ms == 0 {
            continue;
        }
        anim.elapsed_ms = anim.elapsed_ms.saturating_add(dt_ms);
        while anim.elapsed_ms >= anim.rate_ms {
            anim.elapsed_ms -= anim.rate_ms;
            anim.frame = anim.frame.saturating_add(1);
            if anim.frame >= anim.end_frame {
                anim.frame = anim.loop_start;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::components::ParachuteAnim;

    /// Build a minimal `ParachuteAnim` for unit tests.
    fn make_anim(target: u64, frame: u16) -> ParachuteAnim {
        ParachuteAnim {
            target_id: target,
            frame,
            loop_start: 20,
            end_frame: 40,
            rate_ms: 133,
            elapsed_ms: 0,
        }
    }

    /// Pure-data tick: build a Vec<ParachuteAnim>, advance frames, assert.
    /// We can't easily test the full `tick_parachute_anims` without a
    /// `Simulation`, so we test the frame-advancement math separately and
    /// rely on integration verification (Task 9) for spawn/despawn against
    /// a real sim.
    fn advance_frames(anim: &mut ParachuteAnim, dt_ms: u32) {
        if anim.rate_ms == 0 {
            return;
        }
        anim.elapsed_ms = anim.elapsed_ms.saturating_add(dt_ms);
        while anim.elapsed_ms >= anim.rate_ms {
            anim.elapsed_ms -= anim.rate_ms;
            anim.frame = anim.frame.saturating_add(1);
            if anim.frame >= anim.end_frame {
                anim.frame = anim.loop_start;
            }
        }
    }

    #[test]
    fn frame_advances_at_rate_ms_intervals() {
        let mut anim = make_anim(1, 0);
        advance_frames(&mut anim, 132);
        assert_eq!(anim.frame, 0, "below rate_ms threshold should not advance");
        advance_frames(&mut anim, 1); // total 133
        assert_eq!(anim.frame, 1);
    }

    #[test]
    fn frame_wraps_from_end_to_loop_start() {
        let mut anim = make_anim(1, 39); // last valid frame
        advance_frames(&mut anim, 133);
        assert_eq!(anim.frame, 20, "frame should wrap to loop_start (20)");
    }

    #[test]
    fn deploy_phase_plays_frames_0_through_19_once_then_loops_20_to_39() {
        let mut anim = make_anim(1, 0);
        for expected_frame in 1..=39 {
            advance_frames(&mut anim, 133);
            assert_eq!(anim.frame, expected_frame as u16);
        }
        // Next tick wraps to loop_start, NOT to 0.
        advance_frames(&mut anim, 133);
        assert_eq!(anim.frame, 20, "after frame 39, must wrap to 20 (loop_start), not 0");
        // Cycle through loop frames once.
        for expected_frame in 21..=39 {
            advance_frames(&mut anim, 133);
            assert_eq!(anim.frame, expected_frame as u16);
        }
        advance_frames(&mut anim, 133);
        assert_eq!(anim.frame, 20, "second loop wraps again");
    }

    #[test]
    fn multiple_frames_per_tick_advance_correctly() {
        let mut anim = make_anim(1, 0);
        // 5 frames worth of dt (5 * 133 = 665ms).
        advance_frames(&mut anim, 665);
        assert_eq!(anim.frame, 5);
    }

    #[test]
    fn zero_rate_does_not_advance_or_panic() {
        let mut anim = ParachuteAnim {
            rate_ms: 0,
            ..make_anim(1, 0)
        };
        advance_frames(&mut anim, 1000);
        assert_eq!(anim.frame, 0, "zero rate must not advance");
    }
}
```

**Step 3: Verify**

Run:
```
cargo test --lib app_chute_anim
```
Expected: 5 tests pass.

Run:
```
cargo check --lib
```
Expected: PASS.

**Step 4: Commit**

Message: `app_chute_anim: add tick_parachute_anims lifecycle + tests`

---

### Task 5: Wire `tick_parachute_anims` into the per-frame app update

**Why:** Without this call, the lifecycle from Task 4 never runs.

**Files:**
- Modify: `src/app_sim_tick.rs` — add the call alongside other per-frame
  UI ticks.

**Pattern:** `tick_garrison_muzzle_flashes` is called from
`app_sim_tick.rs:189-191` (verified earlier this session). Add the chute
tick call adjacent to it.

**Step 1: Find the call site**

Grep `tick_garrison_muzzle_flashes` in `src/app_sim_tick.rs`. There's
one hit at line ~189.

**Step 2: Add the call**

Read the surrounding 5 lines of context to find the exact `dt_ms`
variable name (it's `sim_elapsed.min(MAX_UPDATE_DELTA_MS) as u32` per
earlier grep — confirm by reading context).

Insert immediately after the `tick_garrison_muzzle_flashes(...)` call:

```rust
        crate::app_chute_anim::tick_parachute_anims(
            state,
            sim_elapsed.min(MAX_UPDATE_DELTA_MS) as u32,
        );
```

(If the muzzle flash call uses a different variable for dt_ms, mirror
that variable name exactly. Read the source first.)

**Step 3: Verify**

Run:
```
cargo check --lib
cargo test --lib
```
Expected: PASS, all existing tests still pass.

**Step 4: Commit**

Message: `app_sim_tick: call tick_parachute_anims each render frame`

---

### Task 6: Implement `build_parachute_instances` renderer

**Why:** Emits sprite instances per frame so the chute actually appears.
Without this the lifecycle ticks but nothing draws.

**Files:**
- Modify: `src/app_instances/overlays.rs` — add function adjacent to
  `build_garrison_muzzle_flash_instances`.

**Pattern:** `build_garrison_muzzle_flash_instances` at line 508 (read
earlier this session). Same shape: iterate the Vec, look up entity by
stable_id, compute screen position, pick atlas entry by SHP key, push
sprite instance.

**Step 1: Add the function**

After the closing `}` of `build_garrison_muzzle_flash_instances` (around
line 566 — find it by grepping; current location may have shifted):

```rust
/// Emit one sprite instance per active parachute anim, anchored above
/// each descending GI's screen position. Mirrors
/// `build_garrison_muzzle_flash_instances` but reads from
/// `state.parachute_anims` and uses the GI's screen_x/screen_y as the
/// anchor (no per-anim pixel offset; SHP atlas's offset_x/offset_y handle
/// center-anchoring).
///
/// Depth: chute uses the GI's body depth offset by a small epsilon so it
/// sorts above the body in the same Layer 2 (Ground) band — matching
/// gamemd's AnimClass::GetLayer override that forces attached anims to
/// Layer 2 regardless of art.ini Layer=.
pub(crate) fn build_parachute_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
) {
    /// Depth epsilon — chute sorts slightly above the GI body. Tuned to
    /// 0.0005 (half of the existing per-Z bias of 0.0001 in
    /// compute_sprite_depth_params). Increase if z-fighting is observed.
    const CHUTE_DEPTH_EPSILON: f32 = 0.0005;

    let (sim, atlas) = match (&state.simulation, &state.sprite_atlas) {
        (Some(s), Some(a)) => (s, a),
        _ => return,
    };
    let z = state.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.camera_x,
        state.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));

    for anim in &state.parachute_anims {
        let entity = match sim.entities.get(anim.target_id) {
            Some(e) => e,
            None => continue,
        };
        let pos = &entity.position;
        let (gx, gy) = (pos.screen_x, pos.screen_y);
        if !in_view(gx, gy, 200.0, 200.0, cam_x, cam_y, sw, sh, 200.0) {
            continue;
        }

        // Resolve PARACH SHP via the rules-side static config.
        let config = match state
            .rules
            .as_ref()
            .and_then(|r| r.general.parachute_render.as_ref())
        {
            Some(c) => c,
            None => continue,
        };

        // Single-facing anim (anims have no Facings= in gamemd).
        let key = ShpSpriteKey {
            type_id: config.shp_name.clone(),
            facing: 0,
            frame: anim.frame,
            house_color: HouseColorIndex(0),
        };
        let Some(entry) = atlas.get(&key) else {
            // Silent fallback per feedback_silent_render_failures: the
            // PARACH SHP isn't loaded into the atlas. Logged once at
            // startup, not per-frame.
            continue;
        };
        let cx: f32 = gx + entry.offset_x;
        let cy: f32 = gy + entry.offset_y;

        // No tint: AltPalette=yes uses the unit/Convert palette which
        // is already the default for unit-style sprites in this atlas.
        // (If the atlas selects per-sprite palettes via tint, route
        //  AltPalette through that path instead — verify in Task 9.)
        let tint: [f32; 3] = [1.0, 1.0, 1.0];

        // Depth: GI's depth value with a small epsilon to draw on top.
        // ZAdjust=-10 in gamemd is a depth-sort fudge; in our
        // depth-buffer rendering, lower depth = closer to camera = on top.
        let gi_depth = compute_sprite_depth_params(origin_y, world_height, gy, pos.z);
        let depth = (gi_depth - CHUTE_DEPTH_EPSILON).clamp(0.001, 0.999);

        paged[entry.page as usize].push(SpriteInstance {
            position: [cx, cy],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha: 1.0,
        });
    }
}
```

**Step 2: Verify imports**

Confirm the file already imports `ShpSpriteKey`, `HouseColorIndex`,
`SpriteInstance`, `compute_sprite_depth_params`, `in_view`. The
muzzle-flash function uses all of these, so they're already in scope.
Read the top of the file to verify; add any missing imports if grep
suggests they aren't there.

**Step 3: Verify**

Run:
```
cargo check --lib
cargo test --lib sidebar:: app_chute_anim
```
Expected: PASS.

**Step 4: Commit**

Message: `app_instances: add build_parachute_instances renderer`

---

### Task 7: Wire `build_parachute_instances` into the render pass

**Why:** Without this call, the renderer ignores parachute anims even
though they exist.

**Files:**
- Modify: `src/app_render/build_instances.rs` — add the call adjacent
  to the muzzle flash call.

**Pattern:** `build_garrison_muzzle_flash_instances` is called at
[src/app_render/build_instances.rs:200](../../src/app_render/build_instances.rs#L200).
Add the chute call immediately after.

**Step 1: Find the call site**

Grep `build_garrison_muzzle_flash_instances` in
`src/app_render/build_instances.rs`. There's one hit (line ~200).

**Step 2: Add the call**

Insert immediately after the existing call:

```rust
    app_instances::build_parachute_instances(state, &mut shp_paged);
```

(If the variable name for the paged sprite vec is different in your file,
mirror it. Read the surrounding 5 lines first to confirm.)

**Step 3: Re-export from `app_instances/mod.rs` (if needed)**

If `app_instances` re-exports the muzzle flash function (typical), do
the same for the parachute one. Grep
`pub use ... build_garrison_muzzle_flash_instances` to confirm.

**Step 4: Verify**

Run:
```
cargo check --lib
cargo test --lib
```
Expected: PASS.

Run:
```
cargo build --lib
```
Expected: PASS, no warnings about unused parachute symbols.

**Step 5: Commit**

Message: `app_render: wire build_parachute_instances into render pass`

---

### Task 8: Register PARACH SHP in the sprite atlas (unit-palette path)

**Why:** PARACH is currently NOT loaded into the sprite atlas. Without
this, the render path's `atlas.get(&key)` returns None and the chute is
silently dropped — the most likely "didn't work" failure mode if Task 9
visible verification fails.

**Critical nuance: AltPalette=yes means unit.pal, not anim.pal.** The
existing effect-anim load block at
[src/render/sprite_atlas.rs:653-723](../../src/render/sprite_atlas.rs#L653-L723)
loads world-effect SHPs (warp, fire, occupant_anim, warhead anims) and
**adds them to `effect_type_ids`**, which routes the renderer's palette
selection at [line 759-763](../../src/render/sprite_atlas.rs#L759-L763)
to `effect_palette` (anim.pal). For PARACH with `AltPalette=yes`,
gamemd uses the unit/Convert palette — equivalent to **`palette`
(unit.pal) in our project**. So PARACH must be **registered into
`needed`** (so the atlas has frame entries) but **NOT added to
`effect_type_ids`** (so the renderer's palette branch picks unit.pal).

This is structurally different from the UCFLASH/warp/fire pattern;
copying that pattern would put PARACH on the wrong palette.

**Files:**
- Modify: `src/render/sprite_atlas.rs` — add a new registration block in
  `build_sprite_atlas` after the existing effect-anim load loop (around
  line 723).

**Pattern:** Adjacent to the `effect_names`-loop block, but
deliberately separate. The novel piece is: register frames without the
`effect_type_ids.insert(...)` call.

**Step 1: Read the existing effect-anim load block**

Read [src/render/sprite_atlas.rs:653-723](../../src/render/sprite_atlas.rs#L653-L723)
to see the existing loop that loads warp/fire/occupant_anim SHPs into
`needed` and `effect_type_ids`. Note the structure of:

- Building `effect_names` from rules
- Iterating `effect_names`, opening each SHP via `asset_manager`
- For each frame in the SHP, inserting into `needed` with
  `house_color: HouseColorIndex(0)`
- Recording frame count in `active_anim_frame_counts`
- **Adding to `effect_type_ids`** — this is what we DO NOT do for PARACH

Also note: `needed` is `HashSet<ShpSpriteKey>`, `active_anim_frame_counts`
is `HashMap<String, u16>` (per the existing code), and `asset_manager` /
`ShpFile` / `HouseColorIndex` are already in scope.

**Step 2: Add the parachute registration block**

Immediately after the closing `}` of the effect-anim load block (around
line 723, after `effect_type_ids.insert(name.clone());` block ends),
add:

```rust
    // Step 1e: Pre-load the parachute SHP (`[General] Parachute=`).
    // Unlike world-effect SHPs above, PARACH's `AltPalette=yes` means
    // gamemd renders it with the unit palette (ColorScheme[0]'s
    // ConvertPalette = our `palette`, NOT `effect_palette`). Register
    // frames into `needed` so the atlas has entries, but DO NOT add to
    // `effect_type_ids` — that would route us to anim.pal.
    if let Some(r) = rules {
        if let Some(pc) = r.general.parachute_render.as_ref() {
            let lower: String = pc.shp_name.to_ascii_lowercase();
            let candidates: Vec<String> =
                vec![format!("{}.shp", lower), format!("{}.SHP", pc.shp_name)];
            if let Some(data) = candidates.iter().find_map(|c| asset_manager.get_ref(c)) {
                if let Ok(shp) = ShpFile::from_bytes(data) {
                    let frame_count: u16 = shp.frames.len() as u16;
                    for f in 0..frame_count {
                        needed.insert(ShpSpriteKey {
                            type_id: pc.shp_name.clone(),
                            facing: 0,
                            frame: f,
                            house_color: HouseColorIndex(0),
                        });
                    }
                    active_anim_frame_counts.insert(pc.shp_name.clone(), frame_count);
                    // Intentionally NOT inserted into effect_type_ids:
                    // AltPalette=yes wants unit.pal, which is the default
                    // for keys not in effect_type_ids.
                    log::info!(
                        "Parachute SHP {}: {} frames loaded (unit palette per AltPalette=yes)",
                        pc.shp_name, frame_count
                    );
                } else {
                    log::warn!(
                        "Parachute SHP {} found in MIX but failed to parse",
                        pc.shp_name
                    );
                }
            } else {
                log::warn!(
                    "Parachute SHP {} not found in MIX archives — chute will not render",
                    pc.shp_name
                );
            }
        }
    }
```

**Step 3: Verify**

Run:
```
cargo check --lib
cargo build --bin vera20k
```
Expected: PASS, no errors. Look for the log line at startup:
`Parachute SHP PARACH: N frames loaded (unit palette per AltPalette=yes)`
where N is the actual SHP frame count (expected ~40 per the design's
inferred-MEDIUM ledger item P14).

**Step 4: Commit**

Message: `sprite_atlas: register PARACH frames in needed without effect_type_ids (AltPalette=yes)`

---

### Task 9: Full integration verification

**Why:** Confirm the chute actually appears in-game, looks right, deploys
and loops, and despawns on landing/death.

**Files:** none modified (verification only).

**Verify:**

Run:
```
cargo run --bin vera20k
```

In-game checklist:
1. **Chute appears.** Trigger an American paradrop. Confirm the PARACH
   SHP is visible above each falling GI during descent.
2. **Anchor is correct.** The chute canopy is directly above the GI's
   head, not offset to one side.
3. **Deploy phase.** On the first ~2.7 seconds of descent, the chute
   plays frames 0-19 once (visibly different opening animation).
4. **Loop phase.** After deploy, the chute settles into a continuous
   loop (frames 20-39) — visually a "swinging" or settled-canopy
   animation.
5. **Multiple chutes.** 8 GIs from one carrier produce 8 chutes, each
   independent. None glitch into each other.
6. **Despawn on landing.** When a GI's `parachute_state` clears (lands),
   the chute disappears within one frame. No stale chute over a
   standing GI.
7. **Despawn on death.** Manually kill a paradropping GI mid-descent
   (e.g., AA fire). The chute disappears with the GI; no phantom chute
   floats in air.
8. **Palette consistency.** Drop both Soviet and Allied paradrops in
   the same map; chutes are the same color (NOT owner-tinted).
9. **Depth ordering.** The chute draws above the GI body (no chute
   hidden behind body).
10. **No regressions.** Run `cargo test --lib` and confirm 1606+ tests
    still pass.

**Failure diagnosis:**
- **No chute visible at all:** Task 8 atlas registration likely missed.
  Grep for the SHP key in atlas-load logs.
- **Chute visible but wrong frame timing (too fast/slow):** Task 1
  `art_rate_to_delay_ms` math — confirm `art_rate_to_delay_ms(400) ==
  133` via `cargo test`.
- **Chute hidden behind GI:** depth epsilon too small. Increase
  `CHUTE_DEPTH_EPSILON` in Task 6 (try 0.001 or 0.002).
- **Chute floats off to one side:** anchor offset wrong. Check
  `entry.offset_x/offset_y` semantics in the atlas — they may need
  centering math added explicitly instead of relying on pre-baked
  offsets.
- **Chute lingers after landing:** Task 4 despawn phase failure. Add
  log line at start of phase 1 to verify.

**Commit:** none (verification step). If a fix-up commit is needed for
any failure above, add it inline before declaring task complete.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-06-parachute-shp-rendering-design.md](2026-05-06-parachute-shp-rendering-design.md)
- **Ghidra reports:**
  - [`ra2-rust-game-docs/PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md)
    (HIGH confidence, written this session)
  - [`ra2-rust-game-docs/ANIM_CLASS_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/ANIM_CLASS_GHIDRA_REPORT.md)
    (HIGH; full struct, AI, Constructor, DrawIt)
  - [`ra2-rust-game-docs/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md)
    (HIGH; §3.5 attachment, §3.6 palette)
  - [`ra2-rust-game-docs/LAYER_CLASS_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/LAYER_CLASS_GHIDRA_REPORT.md)
    (HIGH; §3 GetLayer override)
- **gamemd.exe addresses:**
  - `0x00427D00` — `AnimTypeClass::ReadINI` (Rate conversion `900/Rate`)
  - `0x00422CA0` — `AnimClass::DrawIt` (depth + palette cascade)
  - `0x00424cb0` — `AnimClass::GetLayer` (vtable+0x78, owner-attached → Layer 2)
  - `0x00421EA0` — `AnimClass::Constructor` (drawFlags=0x600 includes
    bit 0x200 center-sprite)
- **INI keys:**
  - `rulesmd.ini [General] Parachute=PARACH`
  - `artmd.ini [PARACH] Rate=400 LoopStart=20 LoopEnd=39 LoopCount=30
    AltPalette=yes ZAdjust=-10` (lines 15642-15648)
- **Repo patterns mirrored:**
  - `GarrisonMuzzleFlash` struct at [`src/sim/components.rs:510`](../../src/sim/components.rs#L510)
  - `tick_garrison_muzzle_flashes` at [`src/app_building_anim.rs:495`](../../src/app_building_anim.rs#L495)
  - `build_garrison_muzzle_flash_instances` at [`src/app_instances/overlays.rs:508`](../../src/app_instances/overlays.rs#L508)
  - `compute_sprite_depth_params` at [`src/app_instances/helpers.rs:43`](../../src/app_instances/helpers.rs#L43)
  - `art_rate_to_delay_ms` at [`src/rules/art_data.rs:134`](../../src/rules/art_data.rs#L134)
  - `resolve_art_rates` (or equivalent name) at
    [`src/rules/ruleset.rs:905+`](../../src/rules/ruleset.rs#L905)
- **Related sim infrastructure (read-only, not modified):**
  - `src/sim/movement/parachute_descent.rs` — descent state machine
    (`ParachuteDescentState`, `OverrideKind::Parachute`, body sequence
    set to `SequenceKind::Paradrop`)
  - `SimSoundEvent::ChuteSound` — already wired
    (`aircraft/drop_payload.rs:175`, `app_sim_tick.rs:487`)
- **Prior commits:**
  - `0b7d959 sim: add ChuteSound variant to SimSoundEvent`
  - `1c87146 rules: parse paradrop INI keys into GeneralRules`
  - `01e5ef1 aircraft: paradrop_mission Approach + Overfly handlers wired`
