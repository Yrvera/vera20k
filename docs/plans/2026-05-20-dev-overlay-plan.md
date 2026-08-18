# Developer Overlay Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** A single egui panel — toggled with backtick (`` ` ``) — exposing
runtime knobs (game speed, audio volumes, debug toggles), diagnostic
readouts (FPS, frame time, tick budget, entity count), and a Save/Load
section (named Save As, reload-last-loaded, top-5 recent saves, last-save
readout).

**Architecture:** Pure app-layer addition mirroring the pause-menu
data-in/action-out pattern. New module `app_dev_overlay.rs` produces
actions; `App::handle_dev_overlay` dispatches them via shared toggle
helpers extracted from `app_input.rs`. No sim/, render/, or net/ changes.

**Design Doc:** [docs/plans/2026-05-20-dev-overlay-design.md](2026-05-20-dev-overlay-design.md)

---

## Grounding Summary

- **R1 (ra2-rust-game-docs/):** N/A. Dev tool with no gamemd parity surface;
  the design's Tiny-Detail Ledger explicitly notes this.
- **R2 (Ghidra):** N/A for the same reason. Verified only that
  `sim_speed_tps` is not in `world_hash.rs` — confirmed by grep.
- **R3 (repo patterns):**
  - Data-in / action-out: [src/ui/pause_menu.rs](../../src/ui/pause_menu.rs)
    (`PauseMenuInfo` / `PauseMenuAction` / `draw_pause_menu`).
  - Caller dispatch: [src/app.rs:1341-1394](../../src/app.rs#L1341-L1394)
    `App::handle_pause_menu` is the template for `handle_dev_overlay`.
  - Save/load helpers: `quicksave` ([src/app_input.rs:498-527](../../src/app_input.rs#L498-L527)),
    `load_save_file` ([src/app_input.rs:558-627](../../src/app_input.rs#L558-L627)),
    `SaveListCache` ([src/app_save_load_panel.rs:30-56](../../src/app_save_load_panel.rs#L30-L56)),
    `format_timestamp` ([src/app_save_load_panel.rs:101-122](../../src/app_save_load_panel.rs#L101-L122)).
  - Cursor visibility on panel toggle: F5 branch at
    [src/app_input.rs:371-383](../../src/app_input.rs#L371-L383).
  - Debug panel chrome: `debug_panel_frame`, `push_debug_light_visuals`,
    `pop_debug_light_visuals` in
    [src/app_debug_panel.rs:15-44](../../src/app_debug_panel.rs#L15-L44).
  - egui input gating: existing `egui_consumed` check at
    [src/app.rs:713-756](../../src/app.rs#L713-L756) automatically blocks
    hotkeys when a text field has focus — risk area #4 is handled.
- **R4 (INI keys):** N/A. No INI-driven constants; defaults come from
  existing code (`tps_for_game_speed`, audio default volumes).
- **Premise re-check (step A.1):** `git log -10` on every modified file
  shows no conflicting commits since the design doc's date (2026-05-20).
  Recent app.rs commits are about main-menu shell rendering — orthogonal.
- **Still unknown:** none. Every claim in the design has a verified anchor
  in the current repo.

## Key Technical Decisions

- **Data-in / action-out pattern (Approach A from design).** Mirrors
  `pause_menu.rs` exactly. — **Confidence:** high — **Source:** repo
  pattern [src/ui/pause_menu.rs](../../src/ui/pause_menu.rs).
- **Extract `toggle_unit_inspector`, `toggle_pathgrid_overlay`,
  `toggle_debug_pause` helpers** so hotkey and panel paths converge on
  one implementation. — **Confidence:** high — **Source:** design Risk
  Area #2, with current duplication visible in
  [src/app_input.rs:398-487](../../src/app_input.rs#L398-L487).
- **`sim_speed_tps` is wall-clock only.** Safe to mutate at runtime
  without affecting replay or state hash. — **Confidence:** high —
  **Source:** verified via grep against
  [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) (no
  matches) and reading
  [src/app_sim_tick.rs:229-231](../../src/app_sim_tick.rs#L229-L231).
- **Reuse `SaveListCache` for the inline recent-saves list.** Cache is
  already invalidated by every in-process save/delete; no new scan path.
  — **Confidence:** high — **Source:** repo pattern
  [src/app_save_load_panel.rs:30-56](../../src/app_save_load_panel.rs#L30-L56).
- **Promote `format_timestamp` to `pub(crate)`** rather than duplicating.
  — **Confidence:** high — **Source:** design Risk Area mitigations.
- **Sanitize save filenames** by replacing `/ \ : * ? " < > |` with `_`,
  trim, cap at 64 chars. — **Confidence:** medium — **Source:** Windows
  reserved-character list + cross-platform safety. Acceptable for a dev
  tool; final filename always contains `_tick{N}_{ts}` so collisions are
  impossible and the existing list-panel parser still works.
- **`FrameTimer` ring buffer of last 60 sample Durations**, sampled at the
  top of `render_frame` every frame. — **Confidence:** high — **Source:**
  standard pattern; no existing FPS counter to reuse (grep confirmed).
- **egui input gating is automatic.** The existing `!egui_consumed` check
  at [src/app.rs:750](../../src/app.rs#L750) blocks hotkeys when egui has
  keyboard focus — so the Save-As text field naturally swallows backtick
  while focused. No new code needed for risk area #4. — **Confidence:**
  high — **Source:** read [src/app.rs:713-756](../../src/app.rs#L713-L756).

## Open Questions

### Resolved During Planning

- **Where does `FrameTimer` live?** → New struct inside
  `src/app_dev_overlay.rs`. Sampled from `App::render_frame` at the top
  every frame so the readout is always fresh when the panel opens.
- **Does egui's text field eat the backtick close-key?** → Yes,
  automatically, via the existing `egui_consumed` gating. No extra code
  needed.
- **Should panel-open invalidate the save cache?** → Yes (design risk
  #8). Backtick keybinding calls `save_list_cache.invalidate()`.

### Deferred to Implementation

- **None.** Every detail is grounded.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/app_dev_overlay.rs` | Dev-overlay panel: types, `draw_dev_overlay`, `FrameTimer` |
| Modify | `src/lib.rs` | Register new module |
| Modify | `src/app.rs` | New AppState fields; `handle_dev_overlay`; render hookup; `use_software_cursor` update; FrameTimer sampling in `render_frame`; default-init the new fields |
| Modify | `src/app_input.rs` | Extract toggle helpers; add `save_with_name`; record `last_save_*` and `last_loaded_save_path` from `quicksave`/`load_save_file`; add backtick keybinding |
| Modify | `src/app_save_load_panel.rs` | Promote `format_timestamp` from `fn` to `pub(crate) fn` |
| Modify | `src/app_debug_panel.rs` | Add one line in `draw_hotkey_help` for backtick |

## Interface Changes

- New module `app_dev_overlay` with `pub(crate)` types `DevOverlayInfo<'a>`,
  `RecentSaveRow`, `DevOverlayAction`, `FrameTimer`, and function
  `draw_dev_overlay`. **No external consumers** beyond `app.rs`.
- `app_save_load_panel::format_timestamp` visibility bump from `fn` to
  `pub(crate) fn`. **Consumers added:** `app.rs::handle_dev_overlay`.
- New `pub(crate)` helpers in `app_input.rs`: `toggle_unit_inspector`,
  `toggle_pathgrid_overlay`, `toggle_debug_pause`, `save_with_name`.
  **Consumers added:** `app.rs::handle_dev_overlay`. **Existing callers
  refactored:** the X/F9/P/J hotkey branches in `handle_hotkey_pressed`
  now call these helpers instead of inlining.

## Sim Checklist

N/A — this plan does not touch `src/sim/`. All changes are app-layer.

## Risk Areas

Lifted from design Impact Analysis (#1–#8). Tasks below cite each one.

1. **Cursor visibility transitions** — addressed in Task 11.
2. **Hotkey/panel side-effect drift** — addressed by extracting helpers
   in Tasks 1-3.
3. **`debug_show_pathgrid` cleanup** — preserved by `toggle_pathgrid_overlay`
   helper (Task 2).
4. **Pause/step pacing reset** — preserved by `toggle_debug_pause` helper
   (Task 3).
5. **Speed slider lower bound** — `.max(1)` clamp in handler (Task 9).
6. **egui keyboard focus** — handled automatically by existing
   `egui_consumed` gating; no new code.
7. **Filename sanitization** — `save_with_name` rules (Task 4).
8. **`last_loaded_save_path` stale on delete** — handler checks via
   `Path::exists` before calling load (Task 9).
9. **Recent-list staleness from external writes** — backtick toggle
   invalidates cache (Task 11).

## Parity-Critical Items

**None.** This is a dev tool. gamemd.exe has no equivalent overlay, so
no observable-output spec exists to reproduce. The design explicitly
documents this in its Tiny-Detail Ledger section. All correctness items
are codebase-internal and tracked in Risk Areas above.

---

## Tasks

### Task 1: Extract `toggle_unit_inspector` helper

**Why:** Foundation for dedup. The X hotkey and the panel checkbox must
share one implementation so they cannot drift (design risk #2).

**Files:**
- Modify: `src/app_input.rs` (existing X branch at lines 452-473)

**Pattern:** Pure refactor — move an existing inline block into a
`pub(crate) fn` and call it from the original site.

**Step 1: Add the helper above `handle_hotkey_pressed`**

Add this function before the `handle_hotkey_pressed` function (search for
`pub(crate) fn handle_hotkey_pressed` to find the right spot):

```rust
/// Toggle the unit-inspector debug overlay.
///
/// Beyond flipping `state.debug_unit_inspector`, this allocates per-entity
/// debug logs on enable and frees them on disable, and sets the sim flag
/// `debug_event_logging`. Called by both the X hotkey and the dev overlay
/// checkbox so the two paths cannot drift.
pub(crate) fn toggle_unit_inspector(state: &mut AppState) {
    state.debug_unit_inspector = !state.debug_unit_inspector;
    if let Some(sim) = &mut state.simulation {
        sim.debug_event_logging = state.debug_unit_inspector;
        if state.debug_unit_inspector {
            for entity in sim.entities.values_mut() {
                if entity.debug_log.is_none() {
                    entity.debug_log =
                        Some(crate::sim::debug_event_log::DebugEventLog::new());
                }
            }
            log::info!("Debug unit inspector: ON");
        } else {
            for entity in sim.entities.values_mut() {
                entity.debug_log = None;
            }
            log::info!("Debug unit inspector: OFF");
        }
    }
}
```

**Step 2: Replace the X-branch body in `handle_hotkey_pressed`**

Find this branch (currently around line 452):

```rust
        KeyCode::KeyX => {
            state.debug_unit_inspector = !state.debug_unit_inspector;
            if let Some(sim) = &mut state.simulation {
                sim.debug_event_logging = state.debug_unit_inspector;
                if state.debug_unit_inspector {
                    // Allocate logs on all existing entities.
                    for entity in sim.entities.values_mut() {
                        if entity.debug_log.is_none() {
                            entity.debug_log =
                                Some(crate::sim::debug_event_log::DebugEventLog::new());
                        }
                    }
                    log::info!("Debug unit inspector: ON (X)");
                } else {
                    // Drop all logs to free memory.
                    for entity in sim.entities.values_mut() {
                        entity.debug_log = None;
                    }
                    log::info!("Debug unit inspector: OFF");
                }
            }
        }
```

Replace its body with a single call:

```rust
        KeyCode::KeyX => {
            toggle_unit_inspector(state);
        }
```

**Step 3: Verify**

Run: `cargo build`
Expected: builds cleanly.

Run: `cargo run --bin ra2-engine` (load a map, press X, confirm
`Debug unit inspector: ON` appears in the log, press X again, confirm
OFF).

**Step 4: Commit**

```
app_input: extract toggle_unit_inspector helper for hotkey/panel dedup
```

---

### Task 2: Extract `toggle_pathgrid_overlay` helper

**Why:** Same dedup rationale as Task 1. Preserves the cleanup behavior of
clearing `debug_terrain_cost_speed_type` on toggle-off (design risk #3).

**Files:**
- Modify: `src/app_input.rs` (existing F9/P branch at lines 398-411)

**Pattern:** Same as Task 1.

**Step 1: Add the helper**

Place after `toggle_unit_inspector`:

```rust
/// Toggle the PathGrid / terrain-cost debug overlay.
///
/// Beyond flipping `state.debug_show_pathgrid`, this resets the per-overlay
/// SpeedType override to None when the overlay turns off, so reopening
/// the overlay defaults back to "auto from selected unit". Called by both
/// the F9/P hotkey and the dev overlay checkbox.
pub(crate) fn toggle_pathgrid_overlay(state: &mut AppState) {
    state.debug_show_pathgrid = !state.debug_show_pathgrid;
    if !state.debug_show_pathgrid {
        state.debug_terrain_cost_speed_type = None;
    }
    log::info!(
        "Debug terrain cost overlay: {}",
        if state.debug_show_pathgrid { "ON" } else { "OFF" }
    );
}
```

**Step 2: Replace the F9/P branch body**

Find:

```rust
        KeyCode::F9 | KeyCode::KeyP => {
            state.debug_show_pathgrid = !state.debug_show_pathgrid;
            if !state.debug_show_pathgrid {
                state.debug_terrain_cost_speed_type = None;
            }
            log::info!(
                "Debug terrain cost overlay: {}",
                if state.debug_show_pathgrid {
                    "ON"
                } else {
                    "OFF"
                }
            );
        }
```

Replace with:

```rust
        KeyCode::F9 | KeyCode::KeyP => {
            toggle_pathgrid_overlay(state);
        }
```

**Step 3: Verify**

Run: `cargo build` — expect clean.
In-game: press F9, confirm `Debug terrain cost overlay: ON`; press [
to cycle SpeedType, confirm change; press F9 to turn off, then F9 to
turn on, confirm SpeedType has reset (no carry-over).

**Step 4: Commit**

```
app_input: extract toggle_pathgrid_overlay helper
```

---

### Task 3: Extract `toggle_debug_pause` helper

**Why:** Same dedup. Preserves the timing-accumulator reset on unpause
that prevents a 100-tick catch-up spike (design risk #4).

**Files:**
- Modify: `src/app_input.rs` (existing J branch at lines 474-482)

**Pattern:** Same as Tasks 1-2.

**Step 1: Add the helper**

Place after `toggle_pathgrid_overlay`:

```rust
/// Toggle debug pause (J hotkey / dev overlay).
///
/// Beyond flipping `state.paused`, this resets `last_update_time` and
/// `sim_accumulator_ms` on unpause so the sim does not catch up by
/// hundreds of ticks after a long pause. Called by both the J hotkey
/// and the dev overlay button.
pub(crate) fn toggle_debug_pause(state: &mut AppState) {
    state.paused = !state.paused;
    if !state.paused {
        state.last_update_time = std::time::Instant::now();
        state.sim_accumulator_ms = 0;
    }
    log::info!("Debug pause: {}", if state.paused { "ON" } else { "OFF" });
}
```

**Step 2: Replace the J-branch body**

Find:

```rust
        KeyCode::KeyJ => {
            state.paused = !state.paused;
            if !state.paused {
                // Reset timing to prevent sim accumulator spike after pause.
                state.last_update_time = std::time::Instant::now();
                state.sim_accumulator_ms = 0;
            }
            log::info!("Debug pause: {}", if state.paused { "ON" } else { "OFF" });
        }
```

Replace with:

```rust
        KeyCode::KeyJ => {
            toggle_debug_pause(state);
        }
```

**Step 3: Verify**

Run: `cargo build` — expect clean.
In-game: press J, confirm pause log; wait 5 seconds; press J to unpause;
confirm the game does NOT speed-replay the missed time.

**Step 4: Commit**

```
app_input: extract toggle_debug_pause helper
```

---

### Task 4: Add `save_with_name` helper + readout-recording in `quicksave` / `load_save_file`

**Why:** The dev overlay's "Save As" button and the `last_save_*` /
`last_loaded_save_path` readouts both need new write-side plumbing. Done
in one task because the changes are colocated and small.

**Files:**
- Modify: `src/app_input.rs` (existing `quicksave` and `load_save_file`
  functions; add new `save_with_name` after `quicksave`)

**Pattern:** Mirrors existing `quicksave` body; adds readout-update side
effects at success points.

**Step 1: Add the `save_with_name` helper**

Place immediately after the existing `quicksave` function (after line
527 — search for `fn quicksave(state: &mut AppState)` and place after
its closing brace):

```rust
/// Save the current sim with a user-supplied name (dev overlay "Save As").
///
/// Sanitizes the name (strips path-unsafe chars, trims, length-caps),
/// then writes to `saves/save_{sanitized}_tick{tick}_{unix_secs}.bin` so
/// the existing list-panel parser still works and collisions are
/// impossible. Updates the last-save readout fields. No-ops with a log
/// warning on empty input.
pub(crate) fn save_with_name(state: &mut AppState, raw_name: &str) {
    let sanitized: String = sanitize_save_name(raw_name);
    if sanitized.is_empty() {
        log::warn!("Save As: empty or whitespace-only name, ignored");
        return;
    }
    let Some(sim) = &state.simulation else {
        log::warn!("Save As: no active simulation");
        return;
    };
    let rules_h = state
        .rules
        .as_ref()
        .map(crate::app_sim_tick::rules_hash)
        .unwrap_or(0);
    let map_name = &state.theater_name;
    let bytes = crate::sim::snapshot::GameSnapshot::save(sim, 0, rules_h, map_name);
    if let Err(e) = std::fs::create_dir_all(SAVES_DIR) {
        log::error!("Save As: failed to create saves dir: {e}");
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("save_{sanitized}_tick{}_{}.bin", sim.tick, now);
    let path = format!("{SAVES_DIR}/{filename}");
    match std::fs::write(&path, &bytes) {
        Ok(()) => {
            log::info!("Save As: saved {} bytes to {}", bytes.len(), path);
            state.last_save_tick = Some(sim.tick);
            state.last_save_instant = Some(std::time::Instant::now());
            state.save_list_cache.invalidate();
        }
        Err(e) => log::error!("Save As: write failed: {e}"),
    }
}

/// Sanitize a user-typed save name for use in a filename.
///
/// Replaces Windows-reserved characters (`/ \ : * ? " < > |`) with `_`,
/// trims surrounding whitespace, then caps at 64 chars. Returns an empty
/// string for empty/whitespace-only input.
fn sanitize_save_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            c if c.is_control() => out.push('_'),
            c => out.push(c),
        }
    }
    out.truncate(64);
    out
}

#[cfg(test)]
mod save_name_tests {
    use super::sanitize_save_name;

    #[test]
    fn empty_returns_empty() {
        assert_eq!(sanitize_save_name(""), "");
        assert_eq!(sanitize_save_name("   "), "");
        assert_eq!(sanitize_save_name("\t\n"), "");
    }

    #[test]
    fn strips_path_separators() {
        assert_eq!(sanitize_save_name("../foo"), ".._foo");
        assert_eq!(sanitize_save_name("a/b\\c"), "a_b_c");
    }

    #[test]
    fn strips_windows_reserved_chars() {
        assert_eq!(sanitize_save_name("a:b*c?d\"e<f>g|h"), "a_b_c_d_e_f_g_h");
    }

    #[test]
    fn keeps_normal_chars() {
        assert_eq!(
            sanitize_save_name("miner stuck repro"),
            "miner stuck repro"
        );
        assert_eq!(sanitize_save_name("dock_fix_a"), "dock_fix_a");
    }

    #[test]
    fn caps_at_64_chars() {
        let long: String = "x".repeat(100);
        let out = sanitize_save_name(&long);
        assert_eq!(out.len(), 64);
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(sanitize_save_name("  hello  "), "hello");
    }
}
```

**Step 2: Update `quicksave` to record readouts**

Find this block in `quicksave` (around line 520-525):

```rust
    match std::fs::write(&path, &bytes) {
        Ok(()) => {
            log::info!("Quicksave: saved {} bytes to {}", bytes.len(), path);
            state.save_list_cache.invalidate();
        }
        Err(e) => log::error!("Quicksave: write failed: {e}"),
    }
```

Replace the `Ok(())` arm so it also records the readout fields:

```rust
    match std::fs::write(&path, &bytes) {
        Ok(()) => {
            log::info!("Quicksave: saved {} bytes to {}", bytes.len(), path);
            state.last_save_tick = Some(sim.tick);
            state.last_save_instant = Some(std::time::Instant::now());
            state.save_list_cache.invalidate();
        }
        Err(e) => log::error!("Quicksave: write failed: {e}"),
    }
```

**Step 3: Update `load_save_file` to record `last_loaded_save_path`**

Find this line near the end of `load_save_file` (around line 626):

```rust
    log::info!("Load: restored simulation from {}", path.display());
}
```

Insert before the `log::info!`:

```rust
    state.last_loaded_save_path = Some(path.to_path_buf());
```

So the function ends with:

```rust
    state.last_loaded_save_path = Some(path.to_path_buf());
    log::info!("Load: restored simulation from {}", path.display());
}
```

**Step 4: Verify**

Run: `cargo test save_name_tests -- --nocapture`
Expected: 6 tests pass.

Run: `cargo build`
Expected: builds cleanly. (`state.last_save_tick` etc. don't exist yet —
this task **will fail to compile in isolation**. Task 7 adds the fields.
If executing tasks individually, run Task 7's field additions first or
proceed straight to Task 7 next.)

> **Sequencing note:** Tasks 4-11 form a chain where new field references
> only resolve once Task 7 lands. Either commit Tasks 1-3, then 7, then
> 4-6, 8-11; OR commit all of 1-11 then test. The plan keeps the natural
> reading order; the executor should batch the build/test pass at the
> end of Task 7.

**Step 5: Commit**

```
app_input: add save_with_name and record last-save / last-load readouts
```

---

### Task 5: Promote `format_timestamp` to `pub(crate)`

**Why:** The dev overlay's recent-saves list and last-save readout both
need this formatter. Reuse rather than duplicate.

**Files:**
- Modify: `src/app_save_load_panel.rs` (line 101)

**Pattern:** Pure visibility change.

**Step 1: Bump visibility**

Find (line 100-101):

```rust
/// Format a unix timestamp as a human-readable relative string.
fn format_timestamp(unix_secs: u64) -> String {
```

Replace with:

```rust
/// Format a unix timestamp as a human-readable relative string.
pub(crate) fn format_timestamp(unix_secs: u64) -> String {
```

**Step 2: Verify**

Run: `cargo build`
Expected: builds cleanly. No new consumers in this commit; the function
is just exposed for Task 8.

**Step 3: Commit**

```
app_save_load_panel: expose format_timestamp pub(crate) for reuse
```

---

### Task 6: Promote `debug_panel_frame` and visuals helpers to `pub(crate)`

**Why:** The dev overlay's egui Window needs the same light-theme chrome
as the existing debug panels. Promote rather than duplicate.

**Files:**
- Modify: `src/app_debug_panel.rs` (function `debug_panel_frame` at
  line 15, `push_debug_light_visuals` at line 29, `pop_debug_light_visuals`
  at line 42)

**Pattern:** Visibility-only bump. `push_debug_light_visuals` and
`pop_debug_light_visuals` are already `pub(crate)`; only `debug_panel_frame`
needs promotion.

**Step 1: Bump visibility on `debug_panel_frame`**

Find (line 14-15):

```rust
/// Light-themed frame for all debug panels — .NET/Windows-style appearance.
fn debug_panel_frame() -> egui::Frame {
```

Replace with:

```rust
/// Light-themed frame for all debug panels — .NET/Windows-style appearance.
pub(crate) fn debug_panel_frame() -> egui::Frame {
```

**Step 2: Verify**

Run: `cargo build`
Expected: builds cleanly.

**Step 3: Commit**

```
app_debug_panel: expose debug_panel_frame pub(crate) for reuse
```

---

### Task 7: Add new fields to `AppState` and default-init them

**Why:** All later tasks read and write these fields. Adding them in one
task makes Tasks 1-6 buildable as a chain (some of them reference fields
that arrive here).

**Files:**
- Modify: `src/app.rs` — `AppState` struct (around line 268-326) and the
  constructor at line 1080-1120.

**Pattern:** Mirror the existing dev-state field group (search for
`debug_show_pathgrid: bool,` to find the right cluster).

**Step 1: Add field declarations**

Find this cluster in the `AppState` struct (around line 319-321):

```rust
    /// Save/load panel visible. Toggle with F5.
    pub(crate) show_save_load_panel: bool,
    /// Cached save-file listing for the save/load panel (avoids per-frame disk I/O).
    pub(crate) save_list_cache: crate::app_save_load_panel::SaveListCache,
```

Add immediately after `save_list_cache`, before the `// -- Reusable
per-frame scratch buffers` comment:

```rust
    /// Developer overlay panel visible. Toggle with backtick (`).
    pub(crate) show_dev_overlay: bool,
    /// Text-field buffer for the dev overlay's "Save As" name input.
    /// Lives in AppState so the field persists across frames while open.
    pub(crate) dev_overlay_save_name: String,
    /// Tick number recorded by the most recent save this session.
    pub(crate) last_save_tick: Option<u32>,
    /// Wall-clock instant of the most recent save this session.
    pub(crate) last_save_instant: Option<std::time::Instant>,
    /// Path of the most recently loaded save (for "Reload last load").
    pub(crate) last_loaded_save_path: Option<std::path::PathBuf>,
    /// Rolling FPS / frame-time tracker for the dev overlay readout.
    pub(crate) frame_timer: crate::app_dev_overlay::FrameTimer,
```

**Step 2: Add constructor defaults**

Find the constructor block at line 1115-1116:

```rust
            show_save_load_panel: false,
            save_list_cache: crate::app_save_load_panel::SaveListCache::new(),
```

Add immediately after:

```rust
            show_dev_overlay: false,
            dev_overlay_save_name: String::new(),
            last_save_tick: None,
            last_save_instant: None,
            last_loaded_save_path: None,
            frame_timer: crate::app_dev_overlay::FrameTimer::new(),
```

**Step 3: Verify**

This task references `crate::app_dev_overlay::FrameTimer`, which doesn't
exist until Task 8. The build will fail until Task 8 lands. That is
expected — commit this task without building, then proceed to Task 8.

If you prefer a clean build at every commit, do Task 8 first and Task 7
second; the order is interchangeable. The plan keeps reading order.

**Step 4: Commit**

```
app: add dev overlay AppState fields (visibility, readouts, frame timer)
```

---

### Task 8: Create `src/app_dev_overlay.rs` with types and `FrameTimer`

**Why:** Defines the contract that `App::handle_dev_overlay` and the
draw function depend on. Types-first per write-plan ordering.

**Files:**
- Create: `src/app_dev_overlay.rs`

**Pattern:** Mirrors `src/ui/pause_menu.rs` (data-in / action-out with a
draw function).

**Step 1: Create the file**

```rust
//! Developer overlay panel — runtime knobs and diagnostic readouts.
//!
//! Toggled with backtick (`). Pure egui rendering: data-in / action-out.
//! Caller (app.rs) snapshots state into DevOverlayInfo, draws, and
//! dispatches the returned DevOverlayAction.
//!
//! ## Dependency rules
//! - Part of the app layer — takes pure data in, returns actions out.
//! - No direct AppState dependency in this module (mirrors ui/pause_menu.rs).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::app_debug_panel::debug_panel_frame;
use crate::app_save_load_panel::format_timestamp;

/// Number of frames to average for the FPS / frame-time readout.
const FRAME_TIMER_WINDOW: usize = 60;

/// Speed slider hard bounds. Lower bound prevents throttle-math stalls;
/// upper bound is a sane dev maximum (already faster than gamemd allows).
const SPEED_MIN_TPS: u32 = 1;
const SPEED_MAX_TPS: u32 = 200;

/// One row in the inline recent-saves list. Caller builds these from
/// `save_list_cache.entries`. Owned strings so the panel doesn't borrow
/// the cache across the draw call.
pub(crate) struct RecentSaveRow {
    pub path: PathBuf,
    pub display_name: String,
    pub tick: u32,
    pub age_str: String,
}

/// Snapshot of app state passed into `draw_dev_overlay`.
///
/// The text-field buffer is borrowed mutably so egui can edit it in place.
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
    pub save_name_buf: &'a mut String,
    pub last_save_tick: Option<u32>,
    pub last_save_age: Option<String>,
    pub last_load_available: bool,
    pub last_load_display: Option<String>,
    pub recent_saves: Vec<RecentSaveRow>,
}

/// Actions produced by the dev overlay each frame.
#[derive(Debug, Clone)]
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
    SaveAs,
    ReloadLastLoad,
    LoadSave(PathBuf),
}

/// Rolling FPS / frame-time tracker. Sampled once per `render_frame`.
pub(crate) struct FrameTimer {
    samples: VecDeque<Duration>,
    last_tick: Option<Instant>,
}

impl FrameTimer {
    pub(crate) fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(FRAME_TIMER_WINDOW),
            last_tick: None,
        }
    }

    /// Record one frame boundary. Call from the top of `render_frame`.
    pub(crate) fn sample(&mut self, now: Instant) {
        if let Some(prev) = self.last_tick {
            let dt = now - prev;
            if self.samples.len() == FRAME_TIMER_WINDOW {
                self.samples.pop_front();
            }
            self.samples.push_back(dt);
        }
        self.last_tick = Some(now);
    }

    /// Mean frame time in milliseconds over the current window, or 0
    /// if no samples have been recorded yet.
    pub(crate) fn frame_ms_mean(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let total_ns: u128 = self.samples.iter().map(|d| d.as_nanos()).sum();
        let mean_ns: u128 = total_ns / self.samples.len() as u128;
        (mean_ns as f64 / 1_000_000.0) as f32
    }

    /// FPS derived from the mean frame time, or 0 if no samples.
    pub(crate) fn fps(&self) -> f32 {
        let ms = self.frame_ms_mean();
        if ms <= 0.0 { 0.0 } else { 1000.0 / ms }
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the dev overlay. Returns the chosen action, if any.
pub(crate) fn draw_dev_overlay(
    ctx: &egui::Context,
    info: &mut DevOverlayInfo<'_>,
) -> DevOverlayAction {
    let mut action = DevOverlayAction::None;

    egui::Window::new("Developer Overlay (`)")
        .default_pos([ctx.content_rect().max.x - 340.0, 200.0])
        .default_width(320.0)
        .frame(debug_panel_frame())
        .collapsible(true)
        .resizable(true)
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(30, 30, 30));

            // ── Sim ──
            ui.label(egui::RichText::new("Sim").strong());
            ui.horizontal(|ui| {
                let mut tps = info.sim_speed_tps;
                let resp = ui.add(
                    egui::Slider::new(&mut tps, SPEED_MIN_TPS..=SPEED_MAX_TPS)
                        .text("tps")
                        .integer(),
                );
                if resp.changed() && tps != info.sim_speed_tps {
                    action = DevOverlayAction::SetGameSpeed(tps);
                }
                if ui.button("Reset").clicked() {
                    action = DevOverlayAction::ResetGameSpeed;
                }
            });
            ui.horizontal(|ui| {
                let pause_label = if info.paused { "Resume" } else { "Pause" };
                if ui.button(pause_label).clicked() {
                    action = DevOverlayAction::TogglePause;
                }
                if ui
                    .add_enabled(info.paused, egui::Button::new("Step 1 tick"))
                    .clicked()
                {
                    action = DevOverlayAction::StepOneTick;
                }
                ui.label(format!("paused={}", if info.paused { "ON" } else { "OFF" }));
            });
            ui.label(format!(
                "Tick budget: {:.2} ms  ({} tps)",
                info.tick_budget_ms, info.sim_speed_tps
            ));
            ui.label(format!("Entities: {}", info.entity_count));

            ui.separator();

            // ── Render ──
            ui.label(egui::RichText::new("Render").strong());
            ui.label(format!(
                "FPS: {:.1}   Frame: {:.2} ms",
                info.fps, info.frame_ms
            ));

            ui.separator();

            // ── Audio ──
            ui.label(egui::RichText::new("Audio").strong());
            let mut music = info.music_volume as f32;
            if ui
                .add(egui::Slider::new(&mut music, 0.0..=1.0).text("Music"))
                .changed()
            {
                action = DevOverlayAction::SetMusicVolume(music as f64);
            }
            let mut sfx = info.sfx_volume as f32;
            if ui
                .add(egui::Slider::new(&mut sfx, 0.0..=1.0).text("SFX"))
                .changed()
            {
                action = DevOverlayAction::SetSfxVolume(sfx as f64);
            }

            ui.separator();

            // ── Debug Overlays ──
            ui.label(egui::RichText::new("Debug Overlays").strong());
            let mut b = info.show_pathgrid;
            if ui.checkbox(&mut b, "PathGrid (F9/P)").changed() {
                action = DevOverlayAction::TogglePathGrid;
            }
            let mut b = info.show_cell_grid;
            if ui.checkbox(&mut b, "Cell grid (L)").changed() {
                action = DevOverlayAction::ToggleCellGrid;
            }
            let mut b = info.show_heightmap;
            if ui.checkbox(&mut b, "Heightmap (K)").changed() {
                action = DevOverlayAction::ToggleHeightmap;
            }
            let mut b = info.show_unit_inspector;
            if ui.checkbox(&mut b, "Unit inspector (X)").changed() {
                action = DevOverlayAction::ToggleUnitInspector;
            }
            let mut b = info.reveal_map;
            if ui.checkbox(&mut b, "Reveal map (F10/V)").changed() {
                action = DevOverlayAction::ToggleRevealMap;
            }

            ui.separator();

            // ── Save / Load ──
            ui.label(egui::RichText::new("Save / Load").strong());
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(info.save_name_buf)
                        .desired_width(180.0)
                        .hint_text("save name"),
                );
                let can_save = !info.save_name_buf.trim().is_empty();
                if ui
                    .add_enabled(can_save, egui::Button::new("Save As"))
                    .clicked()
                {
                    action = DevOverlayAction::SaveAs;
                }
            });

            // Recent saves (top 5).
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Recent:").italics());
            if info.recent_saves.is_empty() {
                ui.label(
                    egui::RichText::new("(no saves)")
                        .italics()
                        .color(egui::Color32::from_rgb(140, 140, 140)),
                );
            } else {
                for row in &info.recent_saves {
                    ui.horizontal(|ui| {
                        if ui.button("Load").clicked() {
                            action = DevOverlayAction::LoadSave(row.path.clone());
                        }
                        ui.label(format!(
                            "{}  tick {}  {}",
                            row.display_name, row.tick, row.age_str
                        ));
                    });
                }
            }

            ui.add_space(4.0);
            let reload_label = match &info.last_load_display {
                Some(name) => format!("Reload last load: {name}"),
                None => "Reload last load".to_string(),
            };
            if ui
                .add_enabled(info.last_load_available, egui::Button::new(reload_label))
                .clicked()
            {
                action = DevOverlayAction::ReloadLastLoad;
            }

            // Last-save readout.
            match (info.last_save_tick, &info.last_save_age) {
                (Some(tick), Some(age)) => {
                    ui.label(format!("Last save: tick {tick} ({age})"));
                }
                (Some(tick), None) => {
                    ui.label(format!("Last save: tick {tick}"));
                }
                _ => {
                    ui.label(
                        egui::RichText::new("Last save: (none this session)")
                            .italics()
                            .color(egui::Color32::from_rgb(140, 140, 140)),
                    );
                }
            }
        });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_timer_empty_returns_zero() {
        let t = FrameTimer::new();
        assert_eq!(t.frame_ms_mean(), 0.0);
        assert_eq!(t.fps(), 0.0);
    }

    #[test]
    fn frame_timer_single_sample_is_still_zero() {
        // First sample establishes the baseline; no delta yet.
        let mut t = FrameTimer::new();
        t.sample(Instant::now());
        assert_eq!(t.frame_ms_mean(), 0.0);
    }

    #[test]
    fn frame_timer_two_samples_record_one_delta() {
        let mut t = FrameTimer::new();
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(16);
        t.sample(t0);
        t.sample(t1);
        let mean = t.frame_ms_mean();
        assert!((mean - 16.0).abs() < 0.5, "expected ~16ms, got {mean}");
        let fps = t.fps();
        assert!((fps - 62.5).abs() < 5.0, "expected ~62.5 fps, got {fps}");
    }

    #[test]
    fn frame_timer_window_caps_at_60() {
        let mut t = FrameTimer::new();
        let t0 = Instant::now();
        for i in 0..200 {
            t.sample(t0 + Duration::from_millis(16 * i));
        }
        assert_eq!(t.samples.len(), FRAME_TIMER_WINDOW);
    }
}
```

**Step 2: Verify**

Run: `cargo build` — will fail until Task 9 (lib.rs registration). That's
expected; proceed to Task 9.

**Step 3: Commit**

```
app_dev_overlay: add types, draw function, FrameTimer (unwired)
```

---

### Task 9: Register the new module in `src/lib.rs`

**Why:** Without this, `crate::app_dev_overlay` doesn't resolve.

**Files:**
- Modify: `src/lib.rs` (around line 133)

**Pattern:** Mirrors existing `pub mod app_debug_panel;` declarations.

**Step 1: Add the module declaration**

Find this near the end of `lib.rs` (around line 131-135):

```rust
// Debug visualization overlays — pathgrid walkability, terrain costs.
// Toggled via hotkeys (P / F9 = pathgrid).
pub mod app_debug_overlays;
// Debug info panel — egui overlay with PathGrid/entity info (shown with pathgrid overlay).
pub mod app_debug_panel;
// Save/load panel — egui overlay for managing save files (F5).
pub mod app_save_load_panel;
```

Add at the end:

```rust
// Developer overlay — egui panel with runtime knobs, diagnostics,
// and save/load helpers. Toggled with backtick (`).
pub mod app_dev_overlay;
```

**Step 2: Verify**

Run: `cargo build`
Expected: builds cleanly (or surfaces only the remaining wiring gaps
from Task 7's field references — which Tasks 1-8 should have satisfied
by this point).

Run: `cargo test app_dev_overlay::tests -- --nocapture`
Expected: 4 frame-timer tests pass.

**Step 3: Commit**

```
lib: register app_dev_overlay module
```

---

### Task 10: Add `handle_dev_overlay` to `App` and render hookup

**Why:** This is the integration point. After this task, the panel is
fully functional in-game.

**Files:**
- Modify: `src/app.rs` — add new method around line 1397 (next to
  `handle_save_load_panel`); add render hookup in the debug-panel block
  around line 1270.

**Pattern:** Mirrors `App::handle_pause_menu` and `App::handle_save_load_panel`.

**Step 1: Add the `handle_dev_overlay` method**

Place immediately after `handle_save_load_panel` (after the closing
brace of that function, before the closing brace of the `impl App`
block):

```rust
    /// Draw the dev overlay and dispatch its actions. No-op when the
    /// overlay is hidden — caller checks `show_dev_overlay` before
    /// calling.
    fn handle_dev_overlay(state: &mut AppState) {
        use crate::app_dev_overlay::{
            self, DevOverlayAction, DevOverlayInfo, RecentSaveRow,
        };

        // Build the recent-saves snapshot from the existing cache.
        state.save_list_cache.refresh_if_dirty();
        let recent_saves: Vec<RecentSaveRow> = state
            .save_list_cache
            .entries
            .iter()
            .take(5)
            .map(|e| RecentSaveRow {
                path: e.path.clone(),
                display_name: e
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string(),
                tick: e.header.tick,
                age_str: crate::app_save_load_panel::format_timestamp(
                    e.header.save_timestamp,
                ),
            })
            .collect();

        let last_save_age: Option<String> = state.last_save_instant.map(|t| {
            let secs = t.elapsed().as_secs();
            if secs < 60 {
                format!("{secs}s ago")
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else {
                format!("{}h {}m ago", secs / 3600, (secs % 3600) / 60)
            }
        });

        let last_load_available = state
            .last_loaded_save_path
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false);
        let last_load_display = state
            .last_loaded_save_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(str::to_string);

        // Temporarily move the save-name buffer out so it can be borrowed
        // mutably by the info struct without conflicting with state.
        let mut save_name = std::mem::take(&mut state.dev_overlay_save_name);

        let mut info = DevOverlayInfo {
            sim_speed_tps: state.sim_speed_tps,
            paused: state.paused,
            music_volume: state.music_player.as_ref().map_or(0.5, |p| p.volume()),
            sfx_volume: state.sfx_player.as_ref().map_or(0.7, |p| p.volume()),
            show_pathgrid: state.debug_show_pathgrid,
            show_cell_grid: state.debug_show_cell_grid,
            show_heightmap: state.debug_show_heightmap,
            show_unit_inspector: state.debug_unit_inspector,
            reveal_map: state.sandbox_full_visibility,
            fps: state.frame_timer.fps(),
            frame_ms: state.frame_timer.frame_ms_mean(),
            tick_budget_ms: if state.sim_speed_tps == 0 {
                0.0
            } else {
                1000.0 / state.sim_speed_tps as f32
            },
            entity_count: state
                .simulation
                .as_ref()
                .map_or(0, |s| s.entities.len()),
            save_name_buf: &mut save_name,
            last_save_tick: state.last_save_tick,
            last_save_age,
            last_load_available,
            last_load_display,
            recent_saves,
        };

        let action = app_dev_overlay::draw_dev_overlay(&state.egui.ctx, &mut info);

        // Restore the (possibly-edited) buffer.
        state.dev_overlay_save_name = save_name;

        match action {
            DevOverlayAction::None => {}
            DevOverlayAction::SetGameSpeed(tps) => {
                state.sim_speed_tps = tps.max(1);
                log::info!("Game speed: {} tps", state.sim_speed_tps);
            }
            DevOverlayAction::ResetGameSpeed => {
                state.sim_speed_tps = crate::app_types::default_yr_skirmish_tps();
                log::info!("Game speed reset to {} tps", state.sim_speed_tps);
            }
            DevOverlayAction::SetMusicVolume(v) => {
                if let Some(p) = &mut state.music_player {
                    p.set_volume(v);
                }
            }
            DevOverlayAction::SetSfxVolume(v) => {
                if let Some(p) = &mut state.sfx_player {
                    p.set_volume(v);
                }
            }
            DevOverlayAction::TogglePause => {
                app_input::toggle_debug_pause(state);
            }
            DevOverlayAction::StepOneTick => {
                if state.paused {
                    state.debug_frame_step_requested = true;
                }
            }
            DevOverlayAction::TogglePathGrid => {
                app_input::toggle_pathgrid_overlay(state);
            }
            DevOverlayAction::ToggleCellGrid => {
                state.debug_show_cell_grid = !state.debug_show_cell_grid;
            }
            DevOverlayAction::ToggleHeightmap => {
                state.debug_show_heightmap = !state.debug_show_heightmap;
            }
            DevOverlayAction::ToggleUnitInspector => {
                app_input::toggle_unit_inspector(state);
            }
            DevOverlayAction::ToggleRevealMap => {
                state.sandbox_full_visibility = !state.sandbox_full_visibility;
                log::info!(
                    "Reveal map: {}",
                    if state.sandbox_full_visibility { "ON" } else { "OFF" }
                );
            }
            DevOverlayAction::SaveAs => {
                let name = std::mem::take(&mut state.dev_overlay_save_name);
                app_input::save_with_name(state, &name);
            }
            DevOverlayAction::ReloadLastLoad => {
                if let Some(path) = state.last_loaded_save_path.clone() {
                    if path.exists() {
                        app_input::load_save_file(state, &path);
                    } else {
                        log::warn!(
                            "Reload last load: file no longer exists: {}",
                            path.display()
                        );
                    }
                }
            }
            DevOverlayAction::LoadSave(path) => {
                app_input::load_save_file(state, &path);
            }
        }
    }
```

**Step 2: Add render hookup**

Find this block (around line 1260-1279):

```rust
                let any_debug_panel = state.debug_show_pathgrid
                    || state.debug_unit_inspector
                    || state.show_hotkey_help;
                let prev_visuals = if any_debug_panel {
                    Some(crate::app_debug_panel::push_debug_light_visuals(
                        &state.egui.ctx,
                    ))
                } else {
                    None
                };
                if state.debug_show_pathgrid {
                    crate::app_debug_panel::draw_debug_panel(&state.egui.ctx, state);
                }
                crate::app_debug_panel::draw_event_history_panel(&state.egui.ctx, state);
                if state.show_hotkey_help {
                    crate::app_debug_panel::draw_hotkey_help(&state.egui.ctx);
                }
                if let Some(prev) = prev_visuals {
                    crate::app_debug_panel::pop_debug_light_visuals(&state.egui.ctx, prev);
                }
```

Replace with (adds dev-overlay to both the visuals gate and the draw
list):

```rust
                let any_debug_panel = state.debug_show_pathgrid
                    || state.debug_unit_inspector
                    || state.show_hotkey_help
                    || state.show_dev_overlay;
                let prev_visuals = if any_debug_panel {
                    Some(crate::app_debug_panel::push_debug_light_visuals(
                        &state.egui.ctx,
                    ))
                } else {
                    None
                };
                if state.debug_show_pathgrid {
                    crate::app_debug_panel::draw_debug_panel(&state.egui.ctx, state);
                }
                crate::app_debug_panel::draw_event_history_panel(&state.egui.ctx, state);
                if state.show_hotkey_help {
                    crate::app_debug_panel::draw_hotkey_help(&state.egui.ctx);
                }
                if state.show_dev_overlay {
                    Self::handle_dev_overlay(state);
                }
                if let Some(prev) = prev_visuals {
                    crate::app_debug_panel::pop_debug_light_visuals(&state.egui.ctx, prev);
                }
```

**Step 3: Verify**

Run: `cargo build`
Expected: builds cleanly.

**Step 4: Commit**

```
app: add handle_dev_overlay and render hookup
```

---

### Task 11: Update `use_software_cursor`, add `FrameTimer::sample` call, fix cursor on toggle

**Why:** Three small wiring fixes that share the same file. Cursor
behavior must mirror the F5 panel (design risk #1); FrameTimer needs a
sampler call to populate data (without it the FPS readout stays 0).

**Files:**
- Modify: `src/app.rs` — `use_software_cursor` at line 346; render_frame
  top at line 1124.
- (Cursor on backtick keypress is done in Task 12, in `app_input.rs`.)

**Pattern:** Same as how `show_save_load_panel` is treated.

**Step 1: Update `use_software_cursor`**

Find (line 346-348):

```rust
    pub(crate) fn use_software_cursor(&self) -> bool {
        self.software_cursor.is_some() && !self.paused && !self.show_save_load_panel
    }
```

Replace with:

```rust
    pub(crate) fn use_software_cursor(&self) -> bool {
        self.software_cursor.is_some()
            && !self.paused
            && !self.show_save_load_panel
            && !self.show_dev_overlay
    }
```

**Step 2: Sample the FrameTimer at the top of `render_frame`**

Find the top of `render_frame` (around line 1124):

```rust
    fn render_frame(state: &mut AppState, event_loop: &ActiveEventLoop) -> Result<()> {
        if let Some(until) = state.startup_splash_until {
```

Replace with:

```rust
    fn render_frame(state: &mut AppState, event_loop: &ActiveEventLoop) -> Result<()> {
        state.frame_timer.sample(Instant::now());
        if let Some(until) = state.startup_splash_until {
```

**Step 3: Verify**

Run: `cargo build`
Expected: builds cleanly.

**Step 4: Commit**

```
app: wire FrameTimer sampling and dev overlay into cursor logic
```

---

### Task 12: Add backtick keybinding + cursor toggle + cache invalidate

**Why:** Connects the input layer to the panel. Mirrors the F5
keybinding's cursor-visibility dance.

**Files:**
- Modify: `src/app_input.rs` — `handle_hotkey_pressed`, near the F5
  branch (around line 371-383).

**Pattern:** Same shape as the F5 KeyCode branch.

**Step 1: Add the backtick branch**

Find the F5 branch (around line 371-383):

```rust
        KeyCode::F5 => {
            state.show_save_load_panel = !state.show_save_load_panel;
            if state.show_save_load_panel {
                state.save_list_cache.invalidate();
                // Show OS cursor for egui interaction.
                if state.software_cursor.is_some() {
                    state.window.set_cursor_visible(true);
                }
            } else if state.software_cursor.is_some() && !state.paused {
                // Re-hide OS cursor so the software cursor takes over.
                state.window.set_cursor_visible(false);
            }
        }
```

Add this **after** the F5 branch (before the next existing branch — find
`KeyCode::KeyH =>` and place above it, OR place anywhere in the
KeyCode arm list — order doesn't matter, but keep it near F5 for
readability):

```rust
        KeyCode::Backquote => {
            state.show_dev_overlay = !state.show_dev_overlay;
            if state.show_dev_overlay {
                // Force a fresh disk scan so the recent-saves list
                // reflects saves written outside this process.
                state.save_list_cache.invalidate();
                // Show OS cursor for egui interaction with sliders/text.
                if state.software_cursor.is_some() {
                    state.window.set_cursor_visible(true);
                }
            } else if state.software_cursor.is_some() && !state.paused {
                // Re-hide OS cursor so the software cursor takes over.
                state.window.set_cursor_visible(false);
            }
        }
```

**Step 2: Verify**

Run: `cargo build`
Expected: builds cleanly.

Run: `cargo run` (skirmish, in-game): press backtick. The dev overlay
should appear top-right with OS cursor active. Drag the title bar to
move it. Press backtick again. Overlay vanishes and software cursor
returns.

**Step 3: Commit**

```
app_input: add backtick keybinding for dev overlay
```

---

### Task 13: Update `draw_hotkey_help` to list the backtick key

**Why:** Discoverability. The F1 help panel lists every other hotkey;
the new one belongs there too.

**Files:**
- Modify: `src/app_debug_panel.rs` — `draw_hotkey_help` (around line
  97-105).

**Pattern:** Add one tuple to the existing `debug_keys` array.

**Step 1: Add the entry**

Find (lines 97-105):

```rust
            let debug_keys: &[(&str, &str)] = &[
                ("F1", "This help panel"),
                ("P / F9", "Terrain costs + debug panel"),
                ("[ ]", "Cycle SpeedType (when P active)"),
                ("K", "Height map (blue=bridge)"),
                ("L", "Cell grid (cyan+yellow)"),
                ("V / F10", "Toggle fog of war"),
                ("X", "Unit inspector (event log)"),
            ];
```

Add a new entry at the end of the array, before the closing `]`:

```rust
            let debug_keys: &[(&str, &str)] = &[
                ("F1", "This help panel"),
                ("P / F9", "Terrain costs + debug panel"),
                ("[ ]", "Cycle SpeedType (when P active)"),
                ("K", "Height map (blue=bridge)"),
                ("L", "Cell grid (cyan+yellow)"),
                ("V / F10", "Toggle fog of war"),
                ("X", "Unit inspector (event log)"),
                ("`", "Developer overlay (speed, volumes, saves)"),
            ];
```

**Step 2: Verify**

Run: `cargo build`
Expected: builds cleanly.

In-game: press F1, confirm the new line is visible in the help panel.

**Step 3: Commit**

```
app_debug_panel: list backtick / dev overlay in hotkey help
```

---

### Task 14: Run the full hand-test checklist

**Why:** The whole point of the feature is what it feels like in-game.
Type checks and unit tests don't catch UI bugs.

**Files:** None modified — verification only.

**Step 1: Build + boot**

```
cargo build
cargo run
```

Load a skirmish.

**Step 2: Run each item in the checklist**

| # | Test | Pass criteria |
|---|------|---------------|
| 1 | Press backtick | Panel appears top-right |
| 2 | Press backtick again | Panel disappears |
| 3 | Cursor while open | OS cursor visible, software cursor hidden |
| 4 | Cursor on close (not paused) | Software cursor returns |
| 5 | Drag title bar | Panel moves with mouse |
| 6 | Click collapse caret | Panel collapses to title; expand restores |
| 7 | Speed slider to 200 | Units move visibly faster |
| 8 | Speed slider to 15 | Units move visibly slower |
| 9 | Reset button | Returns to 63 (YR default) |
| 10 | Music slider to 0 | Music goes silent |
| 11 | Music slider to 1 | Music at full volume |
| 12 | SFX slider | Click sound effect; volume change audible |
| 13 | Pause button + J hotkey | Both toggle pause; values stay in sync |
| 14 | Unpause via either | No 100-tick catch-up (game proceeds normally) |
| 15 | Step 1 tick (while paused) | Sim advances exactly one tick |
| 16 | PathGrid checkbox + F9 | Both stay synced; overlay toggles |
| 17 | Cell grid + L | Same |
| 18 | Heightmap + K | Same |
| 19 | Unit inspector + X | Same; event log allocates/frees |
| 20 | Reveal map + F10 | Same |
| 21 | FPS readout | Non-zero after a few frames |
| 22 | Tick budget readout | Matches `1000 / sim_speed_tps` |
| 23 | Entity count | Matches selection count of all units |
| 24 | Save As "miner_stuck" | Creates `saves/save_miner_stuck_tick{N}_{ts}.bin` |
| 25 | Save As "" or "   " | No-ops, logs warning |
| 26 | Save As "../foo" | Creates `saves/save_.._foo_tick{N}_{ts}.bin`, no path escape |
| 27 | Recent list after save | New save appears at top |
| 28 | Click Load on a recent row | Loads that save |
| 29 | Reload last load | After loading a save, button is enabled; click reloads same save |
| 30 | Reload last load — no save yet | Button disabled |
| 31 | Reload last load — file deleted | Button disabled (path-exists check) |
| 32 | Last save readout | "tick N (Xs ago)" updates after save; refreshes age over time |
| 33 | Type backtick in Save-As field | Inserts a backtick; does NOT close the panel |
| 34 | Empty saves dir | Recent list shows "(no saves)" |
| 35 | Open dev overlay after writing a save from another process | Recent list reflects it (cache invalidated on open) |

**Step 3: Fix any failures**

For any test that fails, return to the corresponding task and revise.
Common failure modes:
- **#14 catch-up bug:** `toggle_debug_pause` helper not called from
  panel handler — check Task 10's `TogglePause` arm.
- **#19 inspector log leak:** Panel checkbox bypasses
  `toggle_unit_inspector` — check Task 10.
- **#33 backtick closes panel while typing:** This SHOULD work via
  existing `egui_consumed` gating; if it doesn't, the issue is upstream
  in the egui-consumed propagation, not this feature.

**Step 4: Commit (if any fix needed)**

```
app_dev_overlay: fix <specific issue from hand-test>
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-20-dev-overlay-design.md](2026-05-20-dev-overlay-design.md)
- **Repo patterns:**
  - [src/ui/pause_menu.rs](../../src/ui/pause_menu.rs) — data-in/action-out
  - [src/app.rs:1341-1394](../../src/app.rs#L1341-L1394) — `handle_pause_menu` dispatch template
  - [src/app_save_load_panel.rs](../../src/app_save_load_panel.rs) — `SaveListCache`, `format_timestamp`
  - [src/app_debug_panel.rs:15-44](../../src/app_debug_panel.rs#L15-L44) — light-theme helpers
  - [src/app_input.rs:371-383](../../src/app_input.rs#L371-L383) — F5 cursor-visibility pattern
  - [src/app_input.rs:498-627](../../src/app_input.rs#L498-L627) — `quicksave` / `load_save_file`
  - [src/app.rs:713-756](../../src/app.rs#L713-L756) — `egui_consumed` input gating
- **Verified absent:** `sim_speed_tps` not in
  [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs)
  (confirmed by grep). Wall-clock-only throttle.
- **Audio APIs used:**
  [src/audio/sfx.rs:329-337](../../src/audio/sfx.rs#L329-L337),
  [src/audio/music.rs:157-168](../../src/audio/music.rs#L157-L168) —
  `set_volume` / `volume` already clamp 0.0..=1.0.
- **Ghidra:** N/A — dev tool, no parity surface.
- **INI keys:** N/A — no constants come from INI.
