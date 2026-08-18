# Parachute SHP Rendering Design

## Goal

Render the PARACH SHP above each paradropped infantry during descent —
center-anchored on the GI's screen position, deploy-then-loop animation,
sorted in the GI's depth band, removed on landing or GI death.

## Architecture Context

**The descent system is already wired.** [src/sim/movement/parachute_descent.rs](../../src/sim/movement/parachute_descent.rs)
maintains `ParachuteDescentState { rate, altitude }` per falling entity,
applies `OverrideKind::Parachute` to suppress the base locomotor, sets
`SequenceKind::Paradrop` on the body, and updates
`entity.position.screen_y` so the GI sprite lifts with altitude. Landing
clears the state. **All this works today** — it's the chute SHP overlay
that's missing.

**Closest precedent: `GarrisonMuzzleFlash`** at
[src/sim/components.rs:510](../../src/sim/components.rs#L510) — a
transient SHP anim attached to an entity (a building when a garrison
fires). Its lifecycle:
1. **Spawn:** sim emits `pending_fire_effects`; app drains and creates
   one `GarrisonMuzzleFlash` per event with `frame=0`, `rate_ms`,
   `target_id`.
2. **Tick:** `tick_garrison_muzzle_flashes` accumulates `elapsed_ms`,
   advances `frame`, removes when `frame >= total_frames`.
3. **Render:** per-frame instance build looks up the target entity's
   screen position, applies a pixel offset, picks the SHP frame, emits
   a sprite instance.

The chute follows the same shape with two differences:
- Lifecycle is **bound by entity state** (descend until landing), not a
  fixed frame count → polling-based, not event-based.
- The "anchor offset" relative to the entity is `(0, 0)` (centered on
  the GI's screen position), not a per-shot pixel offset.

**Altitude-to-screen-Y math is already done.** The GI's `screen_y` is
already lifted by `ALTITUDE_VISUAL_SCALE * altitude`. The chute draws at
the GI's screen position with no further altitude math.

**Verified gamemd render mechanics** (full report:
[`PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md)):
- **Layer for attached anims:** `AnimClass::GetLayer` (vtable+0x78 at
  0x00424cb0) forces Layer 2 (Ground) when `OwnerObject != 0`,
  overriding art.ini `Layer=`. The chute sorts in the same depth band as
  the GI body — NOT in any air-effects layer.
- **Anchor:** flag 0x200 ("Center sprite") = sprite center is at the
  passed screen coords. Chute's canopy extends `H/2` above the GI
  position; payload portion overlaps the GI body.
- **ZAdjust=-10:** depth-sort offset only. Does NOT shift screen Y.
  Matches the infantry's own -10 fudge so chute and GI sort together.
- **AltPalette=yes:** uses `g_ColorSchemeArray[0]->ConvertPalette` — a
  fixed unit-flavored palette, NOT owner-tinted. Chute looks the same
  regardless of who dropped it.
- **Rate=400:** = `900/400 = 2 ticks/frame ≈ 133ms/frame at 15 FPS`. The
  project's `art_rate_to_delay_ms` helper already implements this
  correctly.
- **Loop structure:** frames 0..LoopStart-1 play once on the first
  cycle (deploy phase = implicit), then frames LoopStart..End loop. No
  explicit phase state machine needed; the wraparound `if frame >= End:
  frame = LoopStart` produces deploy-then-loop naturally.

## Impact Analysis

**New struct:**
- `ParachuteAnim` next to `GarrisonMuzzleFlash` in
  [src/sim/components.rs](../../src/sim/components.rs).

**Modified files:**
- [src/sim/components.rs](../../src/sim/components.rs) — add struct.
- [src/app.rs](../../src/app.rs) — add `parachute_anims: Vec<ParachuteAnim>`
  to `AppState` + `Default::default()` init.
- [src/app_building_anim.rs](../../src/app_building_anim.rs) — add
  `tick_parachute_anims` (or split into a sibling
  `app_chute_anim.rs` if file size grows; current
  `app_building_anim.rs` is ~570 lines, so a new sibling is cleaner).
- [src/app_sim_tick.rs](../../src/app_sim_tick.rs) — call
  `tick_parachute_anims` next to the other per-frame UI ticks.
- New module or extension to
  [src/app_instances/](../../src/app_instances/) — `build_parachute_instances`
  emits sprite instances per frame.
- [src/app_render/main_pass.rs](../../src/app_render/main_pass.rs) (or
  wherever entity sprite vectors are concatenated) — include parachute
  instances in the entity sprite layer.
- [src/rules/ruleset.rs](../../src/rules/ruleset.rs) — extend the
  general-rules parser to load PARACH metadata (rate_ms, loop_start,
  loop_end, end frame, z_adjust, alt_palette flag) at startup. The
  `Parachute=` key from `[General]` already gives us the section name
  ("PARACH"); we need to look up that section in artmd.ini and parse
  the same fields `BuildingAnimConfig` parses.

**Asset side:**
- PARACH SHP must be loaded into a sprite atlas at startup. Verify it's
  reachable via the existing asset manager. If not already loaded, add a
  one-line registration during effect-atlas init.
- Owner palette is irrelevant — chute uses ColorScheme[0]'s convert
  palette. Need to identify or add the existing renderer's path that
  routes a sprite through the unit/Convert palette instead of the anim
  palette.

**Risk areas:**
- **Atlas registration:** PARACH may not be in the unit atlas (it's
  considered an "anim" in art.ini). If it's only in an animation atlas
  routed through the anim palette, we need to either re-register PARACH
  in the unit/effect atlas or detect AltPalette=yes at sprite-instance
  time and select the correct palette buffer.
- **Depth ordering with the GI body:** the chute must draw on top of
  the GI body. Both are in Layer 2. We need a small depth epsilon so
  the chute sorts above the body but below other Layer 2 sprites that
  are screen-Y-greater (i.e., closer to camera).
- **Atlas overflow:** loading PARACH may push an atlas past its current
  page count. Multi-page atlas support exists per memory entry
  `feedback_multi_atlas`. Should be benign.
- **Save/load mid-descent:** parachute_anims live in `AppState`, not
  `Simulation`. Save/load reconstructs anims from `parachute_state` on
  the next polling pass. Frame counter resets to 0 — visible as a 1-frame
  hiccup. Acceptable.

**Determinism:** parachute anims are render-only. They do not feed into
any sim state. State hash unchanged.

## Chosen Approach

**Approach 1 — Mirror `GarrisonMuzzleFlash` 1:1, polling-based lifecycle.**

- Per-frame app tick: scan `sim.entities` for entities with
  `parachute_state.is_some()`. Spawn a `ParachuteAnim` for any not yet
  tracked. Despawn anims whose target entity is missing or has
  `parachute_state.is_none()`.
- Tick advances `frame` by `elapsed_ms / rate_ms`. Wrap from `End` to
  `LoopStart` (no explicit deploy/loop phase state — implicit via
  wraparound).
- Render emits one sprite instance per anim per frame at the target
  entity's `screen_x/screen_y`, sprite-center anchored, depth offset by
  `ZAdjust=-10` plus a small epsilon to draw above the body.

**Rejected alternatives:**
- *Sim-emitted events* (Approach 2): adds an event channel for what's
  fundamentally a continuous "tracking" anim. Save/load story is worse
  (need to reconstruct on load). Not justified.
- *Pure derived data* (Approach 3): would require adding render-only
  fields (`attach_tick`) to a sim struct. Cuts against sim-cleanliness.
  Frame derivation gets messy across deploy → loop transition.

## Tiny-Detail Ledger

All UNKNOWNs from the original brainstorm have been resolved by the
Ghidra investigation. Each item is now sourced.

| # | Detail | Source | Implementation home |
|---|---|---|---|
| P1 | Deploy phase: frames 0..LoopStart-1 play once (implicit, via wraparound from End to LoopStart) | `[GHIDRA AnimClass::AI 0x00423AC0]` (verified in `ANIM_CLASS_GHIDRA_REPORT.md` §AI) | Single `frame: u16` counter; no state machine. On `frame >= End`: `frame = LoopStart` |
| P2 | Loop phase: frames LoopStart..End-1 cycle continuously while target entity exists with `parachute_state.is_some()` | `[ini: artmd.ini [PARACH] LoopStart=20 LoopEnd=39]` + `[GHIDRA]` | Wraparound on `frame >= End` |
| P3 | Anim removed when target entity's `parachute_state.is_none()` (landed) or entity is missing (died mid-descent) | `[GHIDRA SetOwnerObject(NULL) on landing]` + `[doc: parachute-descent-design.md §Lifecycle]` | Polling phase 2: drop anims whose target lookup fails |
| P4 | Frame timing: Rate=400 → `art_rate_to_delay_ms(400) = 133ms/frame` (project helper at `art_data.rs:134-140`) | `[GHIDRA AnimTypeClass::ReadINI 0x00427D00 line "iVar4 = 900 / iVar4"]` | `rate_ms: u32 = art_rate_to_delay_ms(rules.parachute.rate)` at parser load time |
| P5 | ZAdjust=-10 = **depth-sort offset, NOT pixel-Y shift** | `[GHIDRA AnimClass::DrawIt 0x00422CA0 depth formula `YDrawOffset + ZAdjust - Z_correction - 2`]` | Apply -10 to the sprite instance's depth value, not to its screen position |
| P6 | AltPalette=yes → `g_ColorSchemeArray[0]->ConvertPalette` — fixed, NOT owner-tinted | `[GHIDRA AnimClass::DrawIt cascade lines ~265-275]` + `[doc: ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md §3.6]` | Sprite-instance build picks the unit/theater palette buffer regardless of GI's owner |
| P7 | Anchor: sprite **center** = passed screen coordinate (flag 0x200) | `[GHIDRA Constructor drawFlags=0x600]` + `[doc: ANIM_CLASS_GHIDRA_REPORT.md §0x600]` | Sprite emits as centered (subtract `W/2`, `H/2` from anchor) — match existing center-anchor convention if any |
| P8 | Layer: forced to Layer 2 (Ground) when attached, overriding art.ini `Layer=` | `[GHIDRA AnimClass::GetLayer 0x00424cb0]` + `[doc: LAYER_CLASS_GHIDRA_REPORT.md §3]` | Emit in same render pass as ground entity sprites; don't route to any air-effects layer |
| P9 | Lifecycle ordering: spawn after sim's `begin_parachute_descent`; despawn after sim's landing tick clears `parachute_state` | repo: existing tick order — sim advance happens before app-tick UI updates | Polling naturally runs after sim advance; order is correct |
| P10 | ChuteSound (out of scope for this design) | `[commit: 0b7d959 sim: add ChuteSound variant to SimSoundEvent]` | Verify separately during implementation that descent attach emits the event and audio plays it |
| P11 | Multiple chutes per paradrop: 8 GIs from one carrier = 8 anims | `[doc: PARADROP_SUPERWEAPON_GHIDRA_REPORT.md]` | `Vec<ParachuteAnim>` handles naturally |
| P12 | GI dies mid-descent: anim removed | `[GHIDRA SetOwnerObject cleanup chain]` | Polling phase 2: target entity missing → drop anim |
| P13 | Frame index 0 start: deploy phase begins on frame 0 (NOT frame 1) | `[ini: artmd.ini Start= unset → defaults 0]` | Init `frame: 0` on spawn |
| P14 | PARACH SHP frame count: inferred 40 (indices 0-39) — End auto-detected from SHP | `[INFERRED — needs HIGH-confidence verification by hex-dump or mix-browser]` | Implementation reads `End` from SHP frame count at load time (matches gamemd auto-detect) |
| P15 | PARACH is single-facing (anims have no Facings= field in gamemd) | `[GHIDRA AnimTypeClass::ReadINI 0x00427D00]` (no Facings parse) | Render the same SHP regardless of GI orientation |
| P16 | LoopCount=30 is an upper bound; chute is destroyed externally on landing, well before exhausting it | `[GHIDRA AnimClass::AI loop logic]` | Don't wire LoopCount-based termination; rely solely on polling-based despawn |

## Design

### Components

#### `ParachuteAnim` struct

Lives in `sim/components.rs` next to `GarrisonMuzzleFlash`. (The "sim"
namespace is a placement convention — like `GarrisonMuzzleFlash`, it's
actually app-level state stored on `AppState`. Co-locating struct
definitions for transient anim records keeps them discoverable.)

```rust
// In src/sim/components.rs (proposed shape — final field set TBD at write-plan)
pub struct ParachuteAnim {
    /// Stable ID of the descending entity. Used to look up screen position
    /// each frame and to detect landing/death (entity missing or
    /// parachute_state.is_none()).
    pub target_id: u64,
    /// Current animation frame (0..end_frame).
    pub frame: u16,
    /// Frame range bounds (loaded once at startup from artmd.ini [PARACH]).
    pub loop_start: u16,
    pub end_frame: u16,
    /// ms per frame; computed via art_rate_to_delay_ms(Rate=).
    pub rate_ms: u32,
    /// Accumulated ms since last frame advance.
    pub elapsed_ms: u32,
}
```

#### `AppState` field

```rust
// In src/app.rs
pub(crate) parachute_anims: Vec<ParachuteAnim>,
```

#### Static PARACH metadata in `RuleSet`

A new field on `GeneralRules` (or wherever PARACH is best surfaced)
holding the parsed art.ini values for the section named in
`[General] Parachute=`:

```rust
pub struct ParachuteRenderConfig {
    pub shp_name: String,           // "PARACH"
    pub rate_ms: u32,                // 133
    pub loop_start: u16,             // 20
    pub end_frame: u16,              // 40 (auto-detected from SHP)
    pub z_adjust: i16,               // -10
    pub alt_palette: bool,           // true
}
```

Loaded at rules-init time alongside the other `rate_from_section`
calls. The shp_name comes from `[General] Parachute=` and the section
metadata comes from `[PARACH]` in artmd.ini.

#### `tick_parachute_anims(state, dt_ms)`

In a new sibling file `app_chute_anim.rs` (or appended to
`app_building_anim.rs` if it stays under ~600 lines).

Three phases, mirroring `tick_garrison_muzzle_flashes`:
1. **Spawn:** scan `sim.entities`; for each entity with
   `parachute_state.is_some()` not present in `parachute_anims` (lookup
   by `target_id`), push a new `ParachuteAnim` with `frame=0`,
   `elapsed_ms=0`, `rate_ms / loop_start / end_frame` from the static
   config.
2. **Despawn:** drop any anim whose target entity is missing in
   `sim.entities` or whose `parachute_state.is_none()`.
3. **Advance:** for each surviving anim, accumulate `elapsed_ms` by
   `dt_ms`. While `elapsed_ms >= rate_ms`, subtract `rate_ms` and
   increment `frame`. On `frame >= end_frame`, set `frame = loop_start`.

#### `build_parachute_instances(state)`

Per-render-frame instance builder. For each anim, look up the target
entity's `screen_x` and `screen_y`, compute the chute's depth (entity
depth + ZAdjust offset + epsilon), emit one sprite instance with
center anchor, current frame, and unit-palette routing.

#### `current_sidebar_view_hit` and other unrelated paths — no change

This design does not touch the sidebar, cursor, or any input handling.
Pure render addition.

### Interfaces / Contracts

- `Vec<ParachuteAnim>` is private app state. No external readers.
- `tick_parachute_anims` consumes `&mut state`. Called once per render
  frame from the existing per-frame update loop in `app_sim_tick.rs`.
- `build_parachute_instances` consumes `&state` + atlas/render context.
  Emits to the existing entity sprite instance vector.
- `Command::*` is not touched. No new sim commands.

### Data Flow

```
Sim tick (existing):
  parachute_descent::tick_parachute_descent → updates state.altitude,
                                              state.rate, body screen_y

App-tick (new):
  tick_parachute_anims(state, dt_ms):
    Phase 1 — spawn:
      for entity in sim.entities where parachute_state.is_some():
          if no anim with target_id == entity.stable_id:
              push ParachuteAnim { target_id, frame: 0, ... config }
    Phase 2 — despawn:
      drop anims where:
          - target entity missing, OR
          - target entity's parachute_state.is_none()
    Phase 3 — advance:
      for each anim:
          elapsed_ms += dt_ms
          while elapsed_ms >= rate_ms:
              elapsed_ms -= rate_ms
              frame += 1
              if frame >= end_frame:
                  frame = loop_start

Render (new):
  build_parachute_instances(state):
    for each anim in state.parachute_anims:
      entity = sim.entities.get(anim.target_id)
      if entity missing: skip (next tick despawns)
      sprite = SpriteInstance {
          position: (entity.screen_x, entity.screen_y),
          frame: anim.frame,
          anchor: Center,
          palette: UnitPalette,
          depth: entity.depth + Z_ADJUST_FACTOR * -10 + EPSILON,
      }
      append to entity_sprites_vec
```

### Error Handling

- **Target entity missing on render frame:** skip rendering this anim.
  Phase 2 of the next tick removes it. No log.
- **PARACH SHP not in atlas:** log a warning ONCE (via OnceLock) at
  first attempt; render falls back to no-sprite (no chute drawn). This
  matches the project's `feedback_silent_render_failures` rule.
- **`Parachute=` rule missing from general rules:** log at startup; no
  ParachuteRenderConfig is built; tick_parachute_anims is a no-op.
  Paradrops still work, just without chute visuals.

### Testing Strategy

Unit tests in `app_chute_anim.rs` (or wherever the tick lives):
- Spawn-on-state: single entity with `parachute_state.is_some()` →
  one anim spawned with `frame=0`.
- Despawn-on-landing: anim → set `parachute_state = None` → next tick
  despawns.
- Despawn-on-death: anim → remove entity from store → next tick
  despawns.
- Frame advance: mock dt_ms accumulator → frame increments at correct
  intervals.
- Wraparound: frame at `end_frame - 1` → next advance → frame ==
  `loop_start`.
- Multiple chutes: 8 entities with `parachute_state.is_some()` → 8
  anims, each with own frame counter.
- Static config parsing: artmd.ini PARACH section → correct rate_ms
  (133), loop_start (20), end_frame (auto-detected or fallback),
  z_adjust (-10), alt_palette (true).

In-game verification (post-implementation):
- Launch a paradrop. Confirm chute SHP appears above each falling GI.
- Cursor over different paradrop angles — chute is the same single-facing SHP.
- Compare visual to gamemd.exe side-by-side: chute deploys (frames 0-19)
  visible during initial fall, then loops (20-39) until landing.
- Save/load mid-descent: chute regenerates next frame from
  `parachute_state`, no crash, frame counter restarts at 0 (visible
  as a 1-frame hiccup, acceptable).

### Determinism Considerations

`parachute_anims` lives in `AppState`, not `Simulation`. Render-only.
Does not affect lockstep state hash. Save/load behavior: chute anims
are not serialized; on load, polling phase 1 reconstructs them next
tick from each entity's `parachute_state`. Frame counter resets to 0,
which is a visual hiccup but does not affect game state.

## Architectural Decisions

### Patterns followed

- **`GarrisonMuzzleFlash` template** — same struct shape (target_id,
  frame, rate_ms, elapsed_ms), same lifecycle phases (spawn / tick /
  despawn / render), same file layout (struct in components.rs, tick
  in app_*_anim.rs, render in app_instances/).
- **`art_rate_to_delay_ms`** — reuse existing helper for Rate
  conversion. Don't duplicate the formula.
- **Polling-based lifecycle** — natural for tracking anims whose
  duration is bound by an external state field, not a fixed frame
  count. Avoids event-channel coordination.

### Patterns deviated from

- *None* — this is purely additive, mirroring an existing pattern.

### Tech debt

- **Q2 frame count is inferred** (MEDIUM confidence). Implementation
  must read `End` from the SHP frame count at load time (matching
  gamemd's auto-detect). If the SHP is mocked in tests, hard-code the
  expected 40-frame layout for the test PARACH, document it, and verify
  on first real-asset run.
- **Suspicious `* 2` in `app_instances/shp.rs:454`** — out of scope for
  chutes, but if it's a real bug it could affect any future anim
  rendered through the same path. Flagged in the Ghidra report for a
  separate `/disparity-scan building-anim timing` pass.

## Alternatives Considered

### Approach 2 — Sim-emitted events

Sim emits `SimSoundEvent`-style `DescentStarted` / `DescentEnded`
events; app drains and translates to anims. Rejected because the chute's
lifecycle is naturally bound by entity state (`parachute_state`), not by
discrete events. Polling is simpler and robust to save/load.

### Approach 3 — Pure derived data

No `ParachuteAnim` struct; frame derived from a global anim clock and
an `attach_tick` field added to `ParachuteDescentState`. Rejected
because it pushes render concerns into a sim struct, violating the
sim/render boundary. Marginal saving in state size doesn't justify the
architectural cost.
