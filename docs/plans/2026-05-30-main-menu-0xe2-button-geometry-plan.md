# Main Menu 0xE2 Button Geometry Fix — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make the dialog `0xE2` main-menu button rects match gamemd: the five non-Exit buttons are SDBTNANM cells (156×42, flush-right at x=644, grid-snapped Y); the Exit button keeps its raw dialog-template rect (162×37 @ x=638, y=536), special-cased.

**Architecture:** Pure presentation-layer change in `src/ui/main_menu_shell/layout.rs` (rect computation). The render path (`src/app_main_menu_shell_render.rs`) already draws SDBTNANM frames at native size at the rect top-left, so it needs no geometry change — only its hardcoded-rect test literals update. No `sim/` involvement.

**Design Doc:** none — source of truth is `docs/research/MAIN_MENU_0XE2_BUTTON_PAINT_AND_REPOSITION_FORK_GHIDRA_REPORT.md` (Implementation Handoff), produced this session.

---

## Grounding Summary

- **What the RE report says (`MAIN_MENU_0XE2_BUTTON_PAINT_AND_REPOSITION_FORK_GHIDRA_REPORT.md`):** Verified that `ResizeShellChildControl_0060C0C0` routes the five non-Exit `0xE2` buttons (`0x683/0x684/0x578/0x686/0x55C`) to `FUN_0060B000`, which resizes each button window to the `SDBTNANM.SHP` cell (156×42), sets X = `parent_right − delta_x − 0x9C(156)` (= 644 at 800w, flush to the panel/screen right edge), and grid-snaps Y to 42-px SDBTNANM rows anchored at the button-column top (y=199 at 800×600). The Exit button `0x3EE` returns `false` from `FUN_00608CD0`, so it takes the fallback coord-fixup and keeps its raw template rect (≈162×37 @ x=638, y=536). The paint asset is SDBTNANM (type 1) — already correct in Rust.
- **Ghidra confirmation (this session):** `decompile_function 0x0060C0C0`, `0x00608CD0`, `0x0060B000`, `0x0060F9A0` (kind-0 gate), `0x00612B70` (paint fork). `FUN_0060B000` X = `parent_right − 156` matches `RightPanel__ComputeLayoutRects @ 0x0072EC70` SDBTNANM rect (632+168−156=644). Grid-snap is round-half-up to the nearest 42-px row (remainder comparison in `FUN_0060B000`). The trace doc `traces/MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md` Stage 4/6 corroborates x=644 and rows 199/241/283/325/367 (it framed the X delta as a 6px "centering" issue; the real cause is the 156×42 window placement).
- **Repo pattern to mirror:** existing `right_anchor_rect` / `version_line_rect` in `layout.rs` (right-edge anchoring with margin), and `right_panel_rects` (panel column, `RIGHT_PANEL_WIDTH=168`, top `RIGHT_PANEL_TOP_H=199`, row `RIGHT_PANEL_TILE_H=42`).
- **INI keys:** none — pure geometry from verified binary constants + SHP dimensions.
- **Still unknown (deferred):** Exit's exact x at non-800×600. gamemd's literal fallback may leave Exit uncentered (template-relative) at high-res; this plan models it consistently with the shell centering margins (x=638+left_margin). At 800×600 both interpretations give 638. Flagged for in-game verification at 1024×768.

## Key Technical Decisions

- **Five buttons → SDBTNANM cell 156×42, x=644 flush-right, grid-snapped Y.** — Matches `FUN_0060B000`. **Confidence:** high — **Source:** Ghidra `0x0060B000`, report §3.3.
- **Exit (`0x3EE`) special-cased to raw template 162×37 @ x=638, y=536, no inset/snap.** — Matches `FUN_00608CD0` false → fallback. **Confidence:** high at 800×600 — **Source:** Ghidra `0x00608CD0`, `0x0060C0C0` fallback branch.
- **Grid-snap = round-half-up to nearest 42-px row from `right_panel.tile.y`.** — Matches `FUN_0060B000` remainder comparison (`if row_h − rem <= rem { round up }`). **Confidence:** high — **Source:** Ghidra `0x0060B000`.
- **Render path unchanged; `push_button_shp` already draws native 156×42 at `rect.x/y`.** — Verified by reading `app_main_menu_shell_render.rs:88-104`. **Confidence:** high — **Source:** repo read.
- **Exit x at letterboxed (>1023w) screens = raw 638 + shell left_margin.** — Sane interpretation consistent with the 800×600 (638-vs-644) relationship; literal gamemd high-res fallback unverified. **Confidence:** medium (high at 800×600) — **Source:** inferred; flag for `/review-plan` + in-game check.

## Open Questions

### Resolved During Planning

- Does the render need to change for the new sizes? → No. `push_button_shp` and `push_button_wave_frame` use `frame.pixel_size` (native 156×42) drawn at `rect.x/y`, not `rect.w/h`. Only the rect's x/y matter for the art; w/h only affect hit-testing and label centering. (evidence: `app_main_menu_shell_render.rs:88-128`)
- Does Exit still paint SDBTNANM art? → Yes; the paint type is shared across the `0xE2` buttons. Exit's 156×42 SDBTNANM frame draws at its rect top-left (638,536); only its window size (162×37) and position differ from the other five. (evidence: report §9)

### Deferred to Implementation

- Exit's exact horizontal position at 1024×768 retail: shell-centered (this plan: x=750) vs literal template fallback (x=638, uncentered). Requires running the game at 1024×768 and comparing to retail gamemd. At 800×600 (standard) this is moot (both = 638).
- Exact f32 rounding of the responsive-layout scaled test values (±1 px): confirm by running `cargo test`; update literals to actual output if my hand-computed values differ by a rounding ULP.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/ui/main_menu_shell/layout.rs` | Replace `button_rect_for_dlu_y` with `sdbtnanm_button_rect` (5 buttons) + `exit_button_rect` (Exit); add cell constants; rewire `compute_layout`; update unit tests |
| Modify | `src/app_main_menu_shell_render.rs` | Update the `button_shp_draws_native_size_at_rect_top_left` test literals to the new rect (no logic change) |
| Modify | `src/ui/main_menu_shell/state.rs` | Re-point three `hit_test_owner_draw_button` unit-test probe coordinates to the new SDBTNANM cell (no logic change — the function is geometry-agnostic) |

## Interface Changes

- Internal-only. `MainMenuShellLayout.buttons: [MainMenuButtonRect; 6]` keeps its type and order (SP, WWOnline, Network, Movies, Options, Exit). Only the `rect` values change. No public signature changes. Consumers (`hit_test_owner_draw_button` in `state.rs`, render in `app_main_menu_shell_render.rs`) read `layout.buttons[..].rect` generically, so the *functions* need no change. Their *unit tests*, however, hard-code probe coordinates / rect literals tied to the old x=635/y=203 geometry and must be updated (Task 5b for `state.rs`, Task 6 for the render test).

## Sim Checklist

N/A — no `sim/` files touched. No determinism/tick/hash impact.

## Risk Areas

- **Hit-testing:** `hit_test_owner_draw_button` (state.rs) iterates `layout.buttons` rects; smaller 156×42 rects (vs 162×37) slightly shrink the clickable area for the 5 buttons and shift it right (644 vs 635). This matches gamemd's button window. Regression: confirm clicking each cameo still maps to the right action (Task 6 in-game check).
- **Tests:** `layout.rs` has several hardcoded-rect unit tests, `app_main_menu_shell_render.rs` has one, and `state.rs` has three hit-test tests whose probe coordinates (`639, 204`; `700, 200`; `639, 204` again in `mouse_release_*`) assume the old x=635/y=203 geometry. All must update or it's a red build. Task 5 greps `layout.rs` stragglers; Task 5b fixes the `state.rs` probes — which the rect-literal grep cannot fully surface (they are point coordinates, not `RectPx::new(..)` literals).
- **Responsive path:** `compute_responsive_layout` scales the base rects; the base now has mixed sizes (5×156×42 + 1×162×37). Scaling each independently is fine, but the responsive test literals change.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2/3 | 5 buttons at x=644 (flush right), 156×42 | Buttons should sit flush to the panel/screen right edge, not inset 9px (visible every menu) | Ghidra `0x0060B000` X = `0x0072EC70` SDBTNANM rect; in-game vs retail at 800×600 |
| Task 2/3 | Grid-snapped Y rows 199/241/283/325/367 | gamemd snaps to 42-px SDBTNANM rows; raw DLU drifts 4/6/8px down-column | Ghidra `0x0060B000` snap math; in-game alignment with chrome tiles |
| Task 2/3 | Exit special-cased: 162×37 @ x=638, y=536 | Exit is geometrically distinct from the other five in gamemd; unifying them is wrong | Ghidra `0x00608CD0` (false for `0x3EE`) → fallback `0x0060C0C0`; in-game |
| Task 4 | SDBTNANM art still lands at the new rect top-left | Art (156×42) must fill the 5 cells 1:1 at 644; Exit art at 638 | Read `push_button_shp`; visual check art is flush-right, not 9px gap |

---

## Tasks

### Task 1: Add SDBTNANM-cell constants

**Why:** Named constants for the button cell size, used by the new rect functions. Avoids magic numbers (CLAUDE.md rule).

**Files:**
- Modify: `src/ui/main_menu_shell/layout.rs` (constants block near line 84-91)

**Pattern:** Mirrors the existing `RIGHT_PANEL_*` constant block.

**Step 1: Add constants** after `pub const LOWER_STRIP_H: i32 = 32;`:
```rust
/// SDBTNANM.SHP button-cell dimensions. gamemd (`FUN_0060B000`) resizes the five
/// non-Exit 0xE2 button windows to this cell, distinct from the dialog-template
/// 162x37 client rect. The cell height equals the SDBTNBKGD tile height (42),
/// but is named separately because it is a different asset's canvas.
pub const SDBTNANM_CELL_W: i32 = 156;
pub const SDBTNANM_CELL_H: i32 = 42;
```

**Step 2: Verify** `cargo check -p vera20k` — compiles (constants unused yet is fine; they're `pub`).

**Step 3: Commit** — "main menu 0xE2: add SDBTNANM button-cell constants".

### Task 2: Add `sdbtnanm_button_rect` and `exit_button_rect`

**Why:** The two distinct button-placement rules. Defines them before `compute_layout` consumes them (interfaces-first).

**Files:**
- Modify: `src/ui/main_menu_shell/layout.rs` — replace `button_rect_for_dlu_y` (and its doc comment, current lines ~274-291) with the two functions below.

**Pattern:** Mirrors `right_anchor_rect` / `version_line_rect` (right-edge anchoring) and `right_panel_rects` (panel column geometry).

**Step 1: Delete** the existing `button_rect_for_dlu_y` function and its `///` doc block (the comment beginning "Build the owner-draw button client rect from its DLU Y position." through the function's closing brace).

**Step 2: Add the SDBTNANM-cell rect (five non-Exit buttons):**
```rust
/// Right-anchored SDBTNANM-cell rect for the five non-Exit 0xE2 buttons.
///
/// gamemd resizes these button windows to the SDBTNANM.SHP cell (156x42),
/// right-anchors them flush to the panel's right edge (x = panel_left + 168 - 156),
/// and snaps the DLU-derived top to the nearest 42-px SDBTNANM row anchored at the
/// button-column top (the SDBTNBKGD tile origin). This replaces the dialog-template
/// 162x37 client rect and its (168-162)/2 inset. The five buttons all sit below the
/// column top, so the snap delta is non-negative.
fn sdbtnanm_button_rect(dlu_y: i32, right_panel: RightPanelRects) -> RectPx {
    let dlu_top = mul_div_round(dlu_y, BASE_Y, 8) + right_panel.top.y;
    let panel_y = right_panel.tile.y; // top of the SDBTNBKGD button column
    let row_h = RIGHT_PANEL_TILE_H; // 42-px SDBTNANM row pitch
    // Round (dlu_top - panel_y) to the nearest row, round-half-up — matching the
    // remainder comparison in gamemd's resize helper (round up when the distance to
    // the next row is <= the distance from the current row).
    let delta = (dlu_top - panel_y).max(0);
    let q = delta / row_h;
    let rem = delta % row_h;
    let q = if row_h - rem <= rem { q + 1 } else { q };
    let y = q * row_h + panel_y;
    let x = right_panel.top.x + (RIGHT_PANEL_WIDTH - SDBTNANM_CELL_W);
    RectPx::new(x, y, SDBTNANM_CELL_W, SDBTNANM_CELL_H)
}
```

**Step 3: Add the Exit rect (special case):**
```rust
/// Exit button (0x3ee) keeps its raw dialog-template rect. gamemd's `FUN_00608CD0`
/// returns false for 0x3ee, so it is NOT resized to the SDBTNANM cell and gets no
/// sidebar inset or grid snap: raw DLU (425,330,108,23) -> (638,536,162,37), offset
/// by the shell centering margins on letterboxed screens (>1023w / >767h), matching
/// how the rest of the shell block is centered.
fn exit_button_rect(screen_w: i32, screen_h: i32) -> RectPx {
    let base = dlu_rect(425, 330, 108, 23); // (638, 536, 162, 37)
    let left_margin = if screen_w > 1023 {
        (screen_w - SHELL_BASE_W) / 2
    } else {
        0
    };
    let top_margin = if screen_h > 767 {
        (screen_h - SHELL_BASE_H) / 2
    } else {
        0
    };
    RectPx::new(base.x + left_margin, base.y + top_margin, base.w, base.h)
}
```

**Step 4: Verify** `cargo check -p vera20k` — fails on `compute_layout` still calling the deleted `button_rect_for_dlu_y` (expected; fixed in Task 3). If any OTHER error appears, stop and re-read.

**Step 5: Commit** — "main menu 0xE2: add SDBTNANM-cell + Exit button rect helpers".

### Task 3: Rewire `compute_layout` button array

**Why:** Point the five non-Exit buttons at `sdbtnanm_button_rect` and Exit at `exit_button_rect`.

**Files:**
- Modify: `src/ui/main_menu_shell/layout.rs` — the `buttons: [ ... ]` array inside `compute_layout` (current lines ~315-340).

**Pattern:** Same struct-literal array; only the `rect` expressions change.

**Step 1: Replace** the `buttons: [ ... ]` array with:
```rust
        buttons: [
            MainMenuButtonRect {
                id: MainMenuControlId::SinglePlayer0x683,
                rect: sdbtnanm_button_rect(125, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::WwOnline0x684,
                rect: sdbtnanm_button_rect(152, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::Network0x578,
                rect: sdbtnanm_button_rect(179, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::MoviesAndCredits0x686,
                rect: sdbtnanm_button_rect(206, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::Options0x55c,
                rect: sdbtnanm_button_rect(233, right_panel),
            },
            MainMenuButtonRect {
                id: MainMenuControlId::ExitGame0x3ee,
                rect: exit_button_rect(screen_w, screen_h),
            },
        ],
```

**Step 2: Verify** `cargo check -p vera20k` — compiles clean (no more reference to the deleted function).

**Step 3: Commit** — "main menu 0xE2: route 5 buttons to SDBTNANM cells, Exit to template rect".

### Task 4: Confirm the render path needs no geometry change

**Why:** Prove the render already draws native 156×42 at the rect top-left, so the new rects produce the correct art positions without touching `app_main_menu_shell_render.rs` logic.

**Files:**
- Read-only: `src/app_main_menu_shell_render.rs` (`push_button_shp` ~line 88, `push_button_wave_frame` ~line 110, `build_text_draws` ~line 262).

**Step 1: Confirm** `push_button_shp` and `push_button_wave_frame` pass `frame.pixel_size` (native 156×42) to `push_entry_sized` at `rect.x, rect.y` — they do NOT scale by `rect.w/rect.h`. So the 5 buttons' art now lands at x=644 (was 635), and Exit's art at x=638. No code change.

**Step 2: Confirm** `build_text_draws` builds `text_rect` from `button.rect` (x, y, w−2, h−1) and centers the label — so labels re-center inside the new 156×42 (and Exit's 162×37) rects automatically. No code change.

**Step 3:** No commit (read-only verification task).

### Task 5: Update `layout.rs` unit tests

**Why:** Every hardcoded button-rect literal must reflect the new geometry, or the build is red.

**Files:**
- Modify: `src/ui/main_menu_shell/layout.rs` `#[cfg(test)] mod tests`.

**Step 1: Update `key_rects_match_800x600`** — replace the two button asserts:
```rust
        // Five non-Exit buttons are SDBTNANM cells: 156x42, flush-right at x=644
        // (632 panel left + 168 - 156), grid-snapped Y. Exit is the special case.
        assert_eq!(layout.buttons[0].rect, RectPx::new(644, 199, 156, 42)); // SP
        assert_eq!(layout.buttons[5].rect, RectPx::new(638, 536, 162, 37)); // Exit (raw template)
```

**Step 2: Add a dedicated test** for all five rows + Exit:
```rust
    #[test]
    fn buttons_grid_snap_and_exit_special_case_800x600() {
        let layout = compute_layout(800, 600);
        // SP/WW/Net/Movies/Options snap to 42-px SDBTNANM rows from y=199.
        let expected_y = [199, 241, 283, 325, 367];
        for (button, y) in layout.buttons[..5].iter().zip(expected_y) {
            assert_eq!(button.rect, RectPx::new(644, y, 156, 42));
        }
        // Exit (0x3ee) is not resized/inset/snapped: raw DLU template rect.
        assert_eq!(layout.buttons[5].rect, RectPx::new(638, 536, 162, 37));
    }
```

**Step 3: Replace `large_screen_buttons_use_centered_dlu_client_rects`** with:
```rust
    #[test]
    fn large_screen_buttons_sdbtnanm_cells_and_exit() {
        // 1024x768: left_margin=112, top_margin=84, panel.top.x=744 -> cells at x=756.
        // Grid anchor panel_y = 84 + 199 = 283; rows step 42.
        let layout = compute_layout(1024, 768);
        let expected_y = [283, 325, 367, 409, 451];
        for (button, y) in layout.buttons[..5].iter().zip(expected_y) {
            assert_eq!(button.rect, RectPx::new(756, y, 156, 42));
        }
        // Exit: raw 638 + left_margin 112 = 750; raw 536 + top_margin 84 = 620.
        assert_eq!(layout.buttons[5].rect, RectPx::new(750, 620, 162, 37));
    }
```

**Step 4: Update the two responsive tests.** In `responsive_layout_fills_window_by_scaling_base_shell`:
```rust
        // Base SP cell (644,199,156,42) scaled 2x/1.5x -> (1288,299,312,63);
        // base Exit (638,536,162,37) -> (1276,804,324,56).
        assert_eq!(layout.buttons[0].rect, RectPx::new(1288, 299, 312, 63));
        assert_eq!(layout.buttons[5].rect, RectPx::new(1276, 804, 324, 56));
```
In `responsive_layout_keeps_640_movie_asset_rule`:
```rust
        // Base SP cell (644,199,156,42) scaled 0.8x -> (515,159,125,34).
        assert_eq!(layout.buttons[0].rect, RectPx::new(515, 159, 125, 34));
```

**Step 5: Grep for stragglers** — search `layout.rs` for any remaining `635`, `203`, `, 162, 37)`, or `747` button-rect literals in tests and update or confirm they are unrelated (e.g., `title`/`version_line` keep 635 — those are correct and unchanged). NOTE: this grep will NOT fully surface the `state.rs` hit-test probes (they are point coordinates like `639, 204`, not rect literals) — those are handled explicitly in Task 5b.

**Step 6: Commit** — "main menu 0xE2: update layout tests for SDBTNANM button geometry".

### Task 5b: Re-point `state.rs` hit-test probe coordinates

**Why:** Three `hit_test_owner_draw_button` unit tests in `state.rs` hard-code probe points that assume the old geometry (SP client at x=635, top y=203). Moving SP to the 156×42 SDBTNANM cell at **x=644, y=199** makes those probes land outside (or newly inside) the rect, so the tests go red. The hit-test *function* is geometry-agnostic and unchanged — only its fixtures need new coordinates. (Verified: `hit_test_uses_unscaled_large_screen_button_rects` still passes against the new 1024×768 rect `(756,283,156,42)` — leave it as-is.)

**Files:**
- Modify: `src/ui/main_menu_shell/state.rs` `#[cfg(test)] mod tests`.

**Step 1: `hit_test_uses_owner_draw_button_identity`** — the SP probe `(639, 204)` is now left of the cell (x≥644). Move it inside:
```rust
        assert_eq!(
            hit_test_owner_draw_button(&layout, 700, 210),
            Some(MainMenuControlId::SinglePlayer0x683)
        );
```
(The Exit probe `(639, 537)` still lands in Exit `(638,536,162,37)`, and `(800, 203)` still misses — leave both unchanged.)

**Step 2: rewrite `client_rect_edge_excludes_old_tile_overhang`** — its premise inverts: the button is no longer a 162×37 client inset inside the 168×42 tile; it is the 156×42 SDBTNANM cell flush to the column's right edge at full tile height. The dead zone is now the left 12 px of the column (632..644), and the top edge is now 199 (= tile top), so the old "y=200 misses" no longer holds. Replace the whole test:
```rust
    #[test]
    fn button_rect_is_flush_right_sdbtnanm_cell() {
        // The hit rect is now the 156x42 SDBTNANM cell flush to the panel's right
        // edge (x=644..800, y=199..241), not the old 162x37 DLU client at x=635.
        // The 12 px on the column's left (632..644) is no longer clickable.
        let layout = compute_layout(800, 600);
        // Left of the flush-right cell (inside the old 168 tile) now misses.
        assert_eq!(hit_test_owner_draw_button(&layout, 640, 210), None);
        // Above the cell top (199) misses; the top row is inclusive.
        assert_eq!(hit_test_owner_draw_button(&layout, 700, 198), None);
        // Inside the 156x42 cell still hits.
        assert_eq!(
            hit_test_owner_draw_button(&layout, 700, 210),
            Some(MainMenuControlId::SinglePlayer0x683)
        );
    }
```

**Step 3: `mouse_release_must_match_pressed_button`** — both `mouse_down(639, 204)` presses now miss SP. Re-point the press inside the cell and the "different button" release to the WWOnline row `(644,241,156,42)`:
```rust
        mouse_down(&mut state, &layout, 700, 210);
        assert_eq!(
            mouse_up(&mut state, &layout, 700, 250),
            MainMenuShellAction::None
        );
        mouse_down(&mut state, &layout, 700, 210);
        assert_eq!(
            mouse_up(&mut state, &layout, 700, 210),
            MainMenuShellAction::SinglePlayer
        );
```

**Step 4: Verify** `cargo test -p vera20k main_menu` — the three `state.rs` tests pass.

**Step 5: Commit** — "main menu 0xE2: re-point state.rs hit-test probes to SDBTNANM cell".

### Task 6: Update the render-side test literals

**Why:** `button_shp_draws_native_size_at_rect_top_left` hardcodes the old SP rect; update for consistency (it tests the draw mechanic, not layout, so logic is unchanged).

**Files:**
- Modify: `src/app_main_menu_shell_render.rs` `#[cfg(test)] mod tests`, `button_shp_draws_native_size_at_rect_top_left` (~line 727).

**Step 1: Update** the rect and assertions:
```rust
        let rect = RectPx::new(644, 199, 156, 42);
```
and the unpressed/pressed position asserts:
```rust
        assert_eq!(out[0].position, [644.0, 199.0]);
```
```rust
        // Pressed: native size, same X, +2 px Y, no horizontal shift.
        assert_eq!(out[0].size, [156.0, 42.0]);
        assert_eq!(out[0].position, [644.0, 201.0]);
```
(The `fake_entry(156.0, 42.0)` frame and the `size` asserts already match 156×42.)

**Step 2: Verify** `cargo check -p vera20k`.

**Step 3: Commit** — "main menu 0xE2: align button render test with SDBTNANM cell rect".

### Task 7: Build + test verification

**Why:** Confirm the whole change compiles and all menu-shell tests pass.

**Files:** none.

**Step 1:** Run `cargo test -p vera20k main_menu` and read the literal `test result:` line.
- Expected: all `main_menu_shell::layout`, `main_menu_shell::state`, and `app_main_menu_shell_render` tests PASS.
- If a responsive-scale assert is off by ±1 px (f32 rounding), update the literal to the actual computed value and re-run (this is the one pre-flagged rounding deferral, not a logic bug).

**Step 2:** Run `cargo clippy -p vera20k` — no new warnings (the deleted `button_rect_for_dlu_y` must not leave a dangling reference; `SDBTNANM_CELL_*` are `pub` so no dead-code warning).

**Step 3:** Commit any rounding-literal fixups — "main menu 0xE2: pin responsive scaled-rect test values".

### Task 8: In-game verification against gamemd

**Why:** Pixel-level parity is the bar; confirm the buttons render flush-right and grid-aligned, and Exit sits distinctly.

**Files:** none.

**Verify (per `/fidelity-check` / run the app at 800×600):**
- The five top buttons' SDBTNANM art is flush to the right panel edge (right edge at x=800), NOT inset with a ~9px right gap.
- The five buttons align to the chrome tile rows (no progressive downward drift); Exit sits lower and slightly left (x=638 vs 644) of the stack.
- Clicking each cameo still triggers the correct action (hit-test against the new 156×42 / Exit 162×37 rects).
- **Deferred high-res check:** at 1024×768, observe whether retail gamemd places Exit shell-centered (≈x=750, this plan) or template-uncentered (≈x=638). If retail differs, revisit `exit_button_rect` (this is the medium-confidence item flagged for `/review-plan`).

**No commit** (observation task; file any follow-up as a new task).

## Sources & References

- **RE report (source of truth):** `docs/research/MAIN_MENU_0XE2_BUTTON_PAINT_AND_REPOSITION_FORK_GHIDRA_REPORT.md`
- **Trace:** `docs/research/traces/MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md` (Stage 4 x=644, Stage 6 rows; Stage 3 SDBTNANM 156×42)
- **gamemd.exe addresses:** `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`, `FUN_00608CD0 @ 0x00608CD0`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060F9A0 @ 0x0060F9A0`, `OwnerDraw_Button_00612B70 @ 0x00612B70`, `RightPanel__ComputeLayoutRects @ 0x0072EC70` — kept here, not in Rust comments.
- **Related code:** `src/ui/main_menu_shell/layout.rs`, `src/ui/main_menu_shell/state.rs` (`hit_test_owner_draw_button`), `src/app_main_menu_shell_render.rs`, `src/render/main_menu_shell_chrome.rs` (SDBTNANM frames — unchanged).
- **Constants:** SDBTNANM cell 156×42, `RIGHT_PANEL_WIDTH=168`, `RIGHT_PANEL_TOP_H=199`, `RIGHT_PANEL_TILE_H=42`, `SHELL_BASE_W/H=800/600`.
