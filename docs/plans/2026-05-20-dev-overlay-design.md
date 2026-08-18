# In-Game Developer Overlay Design

## Goal

A single egui panel — toggled with backtick (`` ` ``) — that exposes runtime
knobs (game speed, audio volumes, debug toggles), diagnostic readouts (FPS,
frame time, tick budget, entity count), and a small Save/Load section for
dev workflow (named "Save As", reload-last-loaded, inline list of the 5 most
recent saves with one-click Load, last-save readout).

## Architecture Context

### Existing debug infrastructure

The app layer already carries every knob this panel needs:

- [src/app.rs:301](../../src/app.rs#L301) `sim_speed_tps: u32` — drives the
  wall-clock throttle in [src/app_sim_tick.rs:229-231](../../src/app_sim_tick.rs#L229-L231).
- [src/app.rs:269](../../src/app.rs#L269) `music_player: Option<MusicPlayer>`,
  [src/app.rs:271](../../src/app.rs#L271) `sfx_player: Option<SfxPlayer>` — both
  expose `set_volume(f64)` (already clamped 0.0..=1.0 internally).
- [src/app.rs:296-319](../../src/app.rs#L296-L319) booleans: `paused`,
  `debug_frame_step_requested`, `debug_show_pathgrid`, `debug_show_cell_grid`,
  `debug_show_heightmap`, `debug_unit_inspector`, plus
  `sandbox_full_visibility` (reveal map / no fog).

### Existing precedent: pause menu pattern

[src/ui/pause_menu.rs](../../src/ui/pause_menu.rs) already does game-speed +
music-volume sliders via a clean data-in / action-out pattern:

1. Caller builds `PauseMenuInfo` from `AppState` (read-only data).
2. `draw_pause_menu(ctx, &info) -> PauseMenuAction` renders and returns the
   chosen action.
3. Caller (`App::handle_pause_menu` in [src/app.rs:1341-1394](../../src/app.rs#L1341-L1394))
   dispatches the action with a match-and-mutate handler.

The dev overlay mirrors this exactly.

### Existing debug panels

[src/app_debug_panel.rs](../../src/app_debug_panel.rs) (590 lines) contains
three read-only diagnostic panels: PathGrid info, unit event history, hotkey
reference. The shared light-themed `debug_panel_frame()` helper is reusable.
These panels take `&AppState` (read-only); the dev overlay produces actions
instead of mutating directly, keeping the read-only invariant intact for the
existing file.

### Existing save/load infrastructure

- **M (quicksave)** at [src/app_input.rs:498-527](../../src/app_input.rs#L498-L527)
  writes `saves/save_tick{tick}_{unix_secs}.bin` and invalidates the
  save-list cache.
- **N (quickload)** loads the most-recently-modified `.bin` via
  `most_recent_save_path` ([src/app_input.rs:530-544](../../src/app_input.rs#L530-L544))
  and `load_save_file` ([src/app_input.rs:558+](../../src/app_input.rs#L558)).
  `load_save_file` is already `pub(crate)`.
- **F5 modal** ([src/app_save_load_panel.rs](../../src/app_save_load_panel.rs))
  lists/loads/deletes saves with the game's client theme.

The dev-overlay Save/Load section reuses `load_save_file` directly, adds
one new write helper for the named-save case, and reads the top 5 entries
from the existing `state.save_list_cache` (already populated and invalidated
correctly by quicksave / F5 modal / dev overlay save). No new directory
scan; the inline list piggybacks on the cache that's already there.

### Hotkey state today

[src/app_input.rs:340-490](../../src/app_input.rs#L340-L490) handles all
existing toggles inline (F9 = pathgrid, L = cell grid, K = heightmap, X =
inspector, J = pause, V = reveal map, etc.). Three of these have non-trivial
side effects beyond flipping a boolean:

- **X (unit inspector)** allocates/frees per-entity `debug_log` storage
  ([src/app_input.rs:452-473](../../src/app_input.rs#L452-L473)).
- **F9/P (pathgrid)** resets `debug_terrain_cost_speed_type = None` on toggle
  off ([src/app_input.rs:400-402](../../src/app_input.rs#L400-L402)).
- **J (pause)** resets `last_update_time` and `sim_accumulator_ms` on unpause
  to prevent a 100-tick catch-up spike
  ([src/app_input.rs:474-481](../../src/app_input.rs#L474-L481)).

The dev panel's checkboxes must reproduce these side effects identically.

## Impact Analysis

**Touched files:**

- `src/app_dev_overlay.rs` — new module (~230 lines incl. Save/Load section
  with inline recent list).
- [src/app.rs](../../src/app.rs) — new fields: `show_dev_overlay: bool`,
  `dev_overlay_save_name: String`, `last_loaded_save_path: Option<PathBuf>`,
  `last_save_tick: Option<u32>`, `last_save_instant: Option<Instant>`. New
  `handle_dev_overlay` method, render hookup, cursor-visibility wiring.
- [src/app_input.rs](../../src/app_input.rs) — backtick binding; extract
  shared toggle helpers from existing hotkey handlers; small refactor to
  `quicksave` and `load_save_file` to record `last_save_tick` /
  `last_loaded_save_path`; new `save_with_name(state, &name)` helper.
- [src/app_debug_panel.rs](../../src/app_debug_panel.rs) — one new line in
  the hotkey reference panel.
- [src/lib.rs](../../src/lib.rs) — register new module.

**Risk areas:**

1. **Cursor visibility transitions.** The egui sliders need the OS cursor.
   Closing the panel must restore the software cursor unless the game is
   paused. Mirror the F5 save-panel pattern at
   [src/app_input.rs:371-383](../../src/app_input.rs#L371-L383).
2. **Side-effect drift between hotkey and panel.** Without dedup, the
   inspector checkbox and X hotkey could diverge silently. Mitigation:
   extract `toggle_unit_inspector`, `toggle_pathgrid_overlay`,
   `toggle_debug_pause` helpers in `app_input.rs`; both call sites converge.
3. **Speed slider lower bound.** `sim_speed_tps = 0` would either deadlock
   the throttle ([src/app_sim_tick.rs:229](../../src/app_sim_tick.rs#L229)
   divides by `SIM_TICK_HZ`, not `sim_speed_tps`, but a value of 0 still
   stalls the accumulator). Clamp to `>= 1`.
4. **egui keyboard focus.** Backtick should be ignored when egui owns the
   keyboard (text input). Check existing input flow to confirm a guard exists.
5. **`app_debug_panel.rs` size.** Currently 590 lines vs the 600-line target.
   Adding the dev overlay there would push it over; a new file is cleaner.
6. **Save filename sanitization.** User-typed save names land in a filesystem
   path. Must strip path separators (`/`, `\`), trim whitespace, reject
   empty names, and length-cap to avoid filesystem limits. Final filename
   becomes `save_{sanitized}_tick{tick}_{unix_secs}.bin` so collisions are
   impossible and the existing list-panel parser still works.
7. **`last_loaded_save_path` becomes stale on delete.** If the user loads
   `foo.bin`, deletes it via F5, then clicks "Reload last load", the file
   won't exist. The button must check existence and log/no-op gracefully —
   `load_save_file` already handles missing files with a warning, so this
   only needs a friendly disable-on-missing in the UI.
8. **Inline recent-saves list staleness.** The list is built from
   `save_list_cache.entries`. The cache only rescans when invalidated, so a
   save written by another process while the dev overlay is open would not
   appear. Acceptable for a dev tool — the cache is already invalidated by
   every in-process save/delete. The dev-overlay panel-open transition
   should also call `save_list_cache.invalidate()` so reopening reflects
   fresh disk state.

## Chosen Approach

**Approach A — Mirror the pause menu's data-in / action-out pattern.**

The dev overlay is rendered by a pure function in `app_dev_overlay.rs` that
takes a `DevOverlayInfo` snapshot and returns a `DevOverlayAction` enum.
`App::handle_dev_overlay` (in `app.rs`) builds the info, calls draw, and
dispatches the action via the same match-and-mutate pattern as
`handle_pause_menu`. Shared toggle helpers extracted from `app_input.rs`
ensure the panel checkbox and the legacy hotkey produce identical side
effects.

Rejected alternatives:

- **Direct `&mut AppState`** — terser but duplicates the inspector/pathgrid/
  pause side-effect logic, which is the actual maintenance risk.
- **Extend `app_debug_panel.rs`** — mixes read-only diagnostic panels with
  mutating control widgets and pushes the file past 600 lines.

## Tiny-Detail Ledger

**N/A — this is a dev tool, not a gamemd parity feature.** gamemd.exe has no
equivalent dev overlay, so there is no observable-output spec to reproduce.
The correctness details that matter here are codebase-internal (cursor
transitions, side-effect reproduction, clamps) and are tracked under Risk
Areas above. Flagging this explicitly so the ledger step isn't accidentally
skipped on a future feature that does have a parity surface.

## Design

### Components

#### `src/app_dev_overlay.rs` (new)

```rust
//! Developer overlay panel — runtime knobs and diagnostic readouts.
//!
//! Toggled with backtick (`). Pure egui rendering: data-in / action-out.
//! Caller (app.rs) snapshots state into DevOverlayInfo, draws, and dispatches
//! the returned DevOverlayAction.
//!
//! ## Dependency rules
//! - Part of the app layer — takes pure data in, returns actions out.
//! - No direct AppState dependency in this module (mirrors ui/pause_menu.rs).

use crate::app_debug_panel::debug_panel_frame_pub;  // small pub helper
                                                     // exposed for reuse

pub(crate) struct DevOverlayInfo<'a> {
    pub sim_speed_tps: u32,
    pub paused: bool,
    pub music_volume: f64,
    pub sfx_volume: f64,
    pub show_pathgrid: bool,
    pub show_cell_grid: bool,
    pub show_heightmap: bool,
    pub show_unit_inspector: bool,
    pub reveal_map: bool,
    pub fps: f32,
    pub frame_ms: f32,
    pub tick_budget_ms: f32,
    pub entity_count: usize,
    /// Mutable buffer for the "Save As" text field. Lives in AppState as
    /// `dev_overlay_save_name: String` so it persists across frames while
    /// the panel is open.
    pub save_name_buf: &'a mut String,
    /// Last save tick (None if no save since session start).
    pub last_save_tick: Option<u32>,
    /// "X seconds ago" string for the last save, formatted by the caller.
    pub last_save_age: Option<String>,
    /// Whether a "Reload last loaded" target exists and the file is on disk.
    pub last_load_available: bool,
    /// Display name (filename stem) of the last-loaded save, for the button
    /// tooltip.
    pub last_load_display: Option<String>,
    /// Top 5 most recent saves from `save_list_cache`, ready to render.
    /// Empty if no saves on disk.
    pub recent_saves: Vec<RecentSaveRow>,
}

/// One row in the inline recent-saves list. Caller builds these from
/// `save_list_cache.entries`. Owned strings so the panel doesn't borrow
/// the cache across the draw call.
pub(crate) struct RecentSaveRow {
    pub path: std::path::PathBuf,
    pub display_name: String,  // filename stem, e.g. "save_dock_fix_a_tick1284_..."
    pub tick: u32,
    pub age_str: String,       // "32s ago" / "4m ago" / "1h 12m ago"
}

pub(crate) enum DevOverlayAction {
    None,
    SetGameSpeed(u32),
    SetMusicVolume(f64),
    SetSfxVolume(f64),
    TogglePause,
    StepOneTick,
    TogglePathGrid,
    ToggleCellGrid,
    ToggleHeightmap,
    ToggleUnitInspector,
    ToggleRevealMap,
    ResetGameSpeed,
    /// User clicked "Save As" with a non-empty sanitized name.
    SaveAs,
    /// User clicked "Reload last load".
    ReloadLastLoad,
    /// User clicked Load on a row in the recent-saves list.
    LoadSave(std::path::PathBuf),
}

pub(crate) fn draw_dev_overlay(
    ctx: &egui::Context,
    info: &DevOverlayInfo,
) -> DevOverlayAction;
```

#### `src/app_input.rs` — extracted helpers

```rust
pub(crate) fn toggle_unit_inspector(state: &mut AppState);
pub(crate) fn toggle_pathgrid_overlay(state: &mut AppState);
pub(crate) fn toggle_debug_pause(state: &mut AppState);
// (cell-grid / heightmap / reveal-map are pure-boolean — no helper needed)

/// Sanitize and save with a user-typed name.
/// Final filename: `save_{sanitized}_tick{tick}_{unix_secs}.bin`.
/// Empty or whitespace-only names no-op with a log warning.
pub(crate) fn save_with_name(state: &mut AppState, raw_name: &str);
```

Also: `quicksave` and `load_save_file` gain three lines each to record
`state.last_save_tick`, `state.last_save_instant`, and
`state.last_loaded_save_path` on success — so quicksave (M), the F5 modal,
and the dev overlay all keep these readouts in sync.

Sanitization rules in `save_with_name`:
- `trim()`, then reject empty.
- Replace each of `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|` with `_`.
- Cap length to 64 characters.

Also: bump `format_timestamp` in
[src/app_save_load_panel.rs:101](../../src/app_save_load_panel.rs#L101) from
private `fn` to `pub(crate) fn` so the dev overlay's `handle_dev_overlay`
can reuse it for the `age_str` field instead of duplicating the formatter.

Existing F9/X/J branches call the helpers. New `KeyCode::Backquote` branch
toggles `state.show_dev_overlay` with the cursor-visibility dance.

#### `src/app.rs` — wiring

```rust
pub(crate) show_dev_overlay: bool,

// FrameTimer for FPS/frame-ms rolling mean (existing field if any, else new):
pub(crate) frame_timer: FrameTimer,  // small ring-buffer over last 60 frames

fn handle_dev_overlay(state: &mut AppState) {
    use crate::app_dev_overlay::{DevOverlayInfo, DevOverlayAction, draw_dev_overlay};

    let info = DevOverlayInfo {
        sim_speed_tps: state.sim_speed_tps,
        paused: state.paused,
        music_volume: state.music_player.as_ref().map_or(0.5, |p| p.volume()),
        sfx_volume:   state.sfx_player.as_ref().map_or(0.7, |p| p.volume()),
        show_pathgrid:       state.debug_show_pathgrid,
        show_cell_grid:      state.debug_show_cell_grid,
        show_heightmap:      state.debug_show_heightmap,
        show_unit_inspector: state.debug_unit_inspector,
        reveal_map:          state.sandbox_full_visibility,
        fps:             state.frame_timer.fps(),
        frame_ms:        state.frame_timer.frame_ms_mean(),
        tick_budget_ms:  1000.0 / state.sim_speed_tps as f32,
        entity_count:    state.simulation.as_ref()
                              .map_or(0, |s| s.entities.len()),
    };

    match draw_dev_overlay(&state.egui.ctx, &info) {
        DevOverlayAction::None => {}
        DevOverlayAction::SetGameSpeed(tps)    => state.sim_speed_tps = tps.max(1),
        DevOverlayAction::SetMusicVolume(v)    => if let Some(p) = &mut state.music_player { p.set_volume(v); },
        DevOverlayAction::SetSfxVolume(v)      => if let Some(p) = &mut state.sfx_player   { p.set_volume(v); },
        DevOverlayAction::TogglePause          => crate::app_input::toggle_debug_pause(state),
        DevOverlayAction::StepOneTick          => if state.paused { state.debug_frame_step_requested = true; },
        DevOverlayAction::TogglePathGrid       => crate::app_input::toggle_pathgrid_overlay(state),
        DevOverlayAction::ToggleCellGrid       => state.debug_show_cell_grid = !state.debug_show_cell_grid,
        DevOverlayAction::ToggleHeightmap      => state.debug_show_heightmap = !state.debug_show_heightmap,
        DevOverlayAction::ToggleUnitInspector  => crate::app_input::toggle_unit_inspector(state),
        DevOverlayAction::ToggleRevealMap      => state.sandbox_full_visibility = !state.sandbox_full_visibility,
        DevOverlayAction::ResetGameSpeed       => state.sim_speed_tps = crate::app_types::default_yr_skirmish_tps(),
        DevOverlayAction::SaveAs               => {
            let name = std::mem::take(&mut state.dev_overlay_save_name);
            crate::app_input::save_with_name(state, &name);
        }
        DevOverlayAction::ReloadLastLoad       => {
            if let Some(path) = state.last_loaded_save_path.clone() {
                crate::app_input::load_save_file(state, &path);
            }
        }
        DevOverlayAction::LoadSave(path)       => {
            crate::app_input::load_save_file(state, &path);
        }
    }
}
```

Building `recent_saves` for the info struct:

```rust
// Inside handle_dev_overlay, before draw_dev_overlay:
state.save_list_cache.refresh_if_dirty();
let recent_saves: Vec<RecentSaveRow> = state
    .save_list_cache
    .entries
    .iter()
    .take(5)
    .map(|e| RecentSaveRow {
        path: e.path.clone(),
        display_name: e.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?").to_string(),
        tick: e.header.tick,
        age_str: format_age(e.header.save_timestamp),  // reuse format_timestamp
                                                       // from app_save_load_panel
    })
    .collect();
```

Render hookup goes inside the existing light-visuals push/pop block at
[src/app.rs:1263-1279](../../src/app.rs#L1263-L1279), gated on
`state.show_dev_overlay`.

`use_software_cursor` ([src/app.rs:346-348](../../src/app.rs#L346-L348))
gains a `&& !self.show_dev_overlay` term.

#### `src/app_debug_panel.rs` — hotkey help

Add one line to the Debug Overlays section in `draw_hotkey_help`:

```rust
("`", "Developer overlay (game speed, volumes, toggles)"),
```

### Data Flow

```
backtick keypress
   → app_input.rs toggles state.show_dev_overlay
   → cursor visibility flip

each frame, while show_dev_overlay:
   app.rs::handle_dev_overlay
      → build DevOverlayInfo from state
      → draw_dev_overlay(ctx, &info) renders egui panel
      → user moves slider / clicks checkbox
      → draw_dev_overlay returns DevOverlayAction
      → match arm dispatches
         - simple field mutation, OR
         - shared toggle helper in app_input.rs
```

### Panel Layout

```
┌─ Developer Overlay (`) ─────────────────────┐
│ Sim                                          │
│   Speed: [────●────] 63 tps   [Reset]        │
│   Presets: [15] [30] [60] [63] [100]         │
│            [200] [500]                       │
│   [Pause]  [Step 1 tick]   paused=OFF        │
│   Tick budget: 15.87 / 16.00 ms              │
│   Entities: 247                              │
│                                              │
│ Render                                       │
│   FPS: 60.0   Frame: 16.67 ms                │
│                                              │
│ Audio                                        │
│   Music: [────●─────] 0.50                   │
│   SFX:   [───────●──] 0.70                   │
│                                              │
│ Debug Overlays                               │
│   ☐ PathGrid       (F9/P)                    │
│   ☐ Cell grid      (L)                       │
│   ☐ Heightmap      (K)                       │
│   ☐ Unit inspector (X)                       │
│   ☐ Reveal map     (F10/V)                   │
│                                              │
│ Save / Load                                  │
│   Name: [miner_stuck_repro_____] [Save As]   │
│   Recent:                                    │
│     [Load] dock_fix_a    tick 1284  32s ago  │
│     [Load] miner_stuck   tick  982  4m ago   │
│     [Load] before_repro  tick  301  12m ago  │
│     [Load] save_t100     tick  100  1h ago   │
│     [Load] save_t50      tick   50  2h ago   │
│   [Reload last load: dock_fix_a.bin]         │
│   Last save: tick 1284 (32s ago)             │
└──────────────────────────────────────────────┘
```

Positioned default at right side of viewport. Resizable, collapsible, draggable.
Light theme via existing `debug_panel_frame()`/`push_debug_light_visuals` helpers.

### FrameTimer

Small ring buffer to compute rolling mean frame time. Either add to AppState
or piggyback on existing render timing if any. Footprint: `[Duration; 60]`
+ index + sample method. ~30 lines.

### Error Handling

None — all operations are infallible (slider sets, checkbox flips). Audio
players gracefully handle `None` if audio init failed.

### Testing Strategy

- **No unit tests** for the egui rendering (requires a Context). Skip.
- **Hand-test checklist** (will be repeated in `/write-plan` output):
  1. Backtick opens/closes the panel.
  2. OS cursor visible while panel is open; software cursor restored on close
     (unless paused).
  3. Game speed slider changes throttle live: drag to 200 tps, units move
     visibly faster; drag to 15 tps, slower.
  4. Reset button returns to 63 (YR default).
  5. Music/SFX sliders change volume live: clip at 0 = silent, 1 = full.
  6. Pause checkbox in panel and J hotkey stay in sync.
  7. Unpause via either path does NOT catch up by hundreds of ticks.
  8. PathGrid checkbox and F9 stay in sync; toggling off via either clears
     the speed-type override.
  9. Unit-inspector checkbox and X stay in sync; toggling on allocates logs,
     off frees them.
  10. Reveal-map checkbox and F10/V stay in sync.
  11. FPS readout updates each frame; tick-budget reflects current
      `sim_speed_tps`; entity count matches selection panel.
  12. Backtick is ignored when typing into an egui text field elsewhere
      (specifically: when focus is in the Save As name field, pressing
      backtick types a backtick instead of closing the panel).
  13. Save As with name "miner stuck repro" creates
      `saves/save_miner_stuck_repro_tick{N}_{ts}.bin` and clears the field.
  14. Save As with empty/whitespace name no-ops and logs a warning.
  15. Save As with `../foo` produces a sanitized filename, not a path escape.
  16. Reload last load button is disabled when no save has been loaded this
      session OR when the path no longer exists on disk.
  17. Last-save readout updates after every save (M hotkey, dev overlay
      Save As) and shows "Xs ago" / "Xm ago" / etc. The F5 modal does not
      create saves, only loads/deletes, so it doesn't touch this readout
      (loads update `last_loaded_save_path` instead).
  18. Inline recent list shows the 5 most recent saves, newest first, with
      one-click `[Load]` per row. Match the order shown by F5.
  19. Saving via Save As immediately adds the new file to the inline list
      (cache invalidated by `save_with_name`); next frame shows it on top.
  20. Deleting a save via F5 immediately removes it from the inline list
      next time the dev overlay reads from the cache.
  21. With zero saves on disk, the Recent section collapses to a single
      "(no saves)" muted label rather than empty space.
  22. Opening the dev overlay (backtick) invalidates the save cache so the
      inline list reflects any saves written outside this session.

## Architectural Decisions

**Patterns followed:**

- Data-in / action-out pure rendering pattern, established by
  [src/ui/pause_menu.rs](../../src/ui/pause_menu.rs).
- Light-themed debug-panel chrome via existing helpers in
  [src/app_debug_panel.rs](../../src/app_debug_panel.rs).
- Cursor visibility pattern from the F5 save-panel branch in
  [src/app_input.rs:371-383](../../src/app_input.rs#L371-L383).
- Per-CLAUDE.md, module-level `//!` doc with dependency rules.

**Patterns deviated from:**

- The new module lives under `src/app_dev_overlay.rs`, not `src/ui/`. The
  pause menu lives in `ui/` because it's parameterized by a non-AppState
  data type and is genuinely UI-only. The dev overlay's `Info` struct is
  app-layer-specific and the action dispatch reaches into multiple subsystems
  (sim toggles, audio mixers, debug flags) — keeping it adjacent to
  `app_debug_panel.rs` is more honest about its scope.

**Tech debt:** none introduced. The toggle-helper extraction in
`app_input.rs` is a net dedup; the existing hotkey branches inline the same
logic that now lives in one place.

**Determinism guarantee:**

- `sim_speed_tps` is wall-clock-only — verified not in
  [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs).
- Music/SFX volumes are app-layer audio mixer state, not in sim.
- Debug toggles do not touch sim state.
- `sandbox_full_visibility` is an existing accepted debug toggle, not new
  surface.

## Alternatives Considered

- **Direct `&mut AppState`** — terser but duplicates inspector / pathgrid /
  pause side-effect logic between hotkey and panel. Rejected on dedup grounds.
- **Extend `app_debug_panel.rs`** — mixes mutating control widgets with the
  file's existing read-only diagnostic panels and pushes it past the 600-line
  target. Rejected on cohesion grounds.
- **Persist sliders to disk** — explicitly chosen against per scoping
  question; session-only avoids stale-slowdown surprises on relaunch.
- **Cheats (give credits, instant-build, kill selected, spawn at cursor)** —
  out of scope for first cut per scoping question. Easy to add later as new
  `DevOverlayAction` variants; the architecture supports it.
