# Skirmish Choose Map Native Modal Implementation Plan

**Goal:** Make the Skirmish "Choose Map" flow look and behave like the native Yuri's Revenge shell dialog `0x6B`, with a separate modal shell, verified listbox rendering, correct accept/cancel semantics, and no post-launch gameplay changes.

**Scope:** Visual and interaction parity for the offline Skirmish Choose Map modal. This plan does not make the native Skirmish shell the default route, does not implement random map generation, and does not change skirmish spawn/gameplay handoff behavior.

**Do not commit unless the user explicitly asks.**

---

## Grounding Summary

**Current Rust surface:** The repo already has a dev-gated native Skirmish shell, verified `0x102` layout helpers, `ChooseMapModalState`, app routing for `ChooseMap`, and an initial modal render path. The current modal is structurally useful but still visually simplified: it is drawn as an overlay over the parent setup shell, uses generic panel colors, and does not yet reproduce the verified `0x6B` shell background/listbox paint contract.

**Research docs:**

- `docs/research/SKIRMISH_UI_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/SKIRMISH_SHELL_UI_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`

**Architecture fit:** Keep the current split:

- `ui/skirmish_shell/layout.rs`: render-agnostic pixel rects and hit testing.
- `ui/skirmish_shell/state.rs`: modal state, selected/highlighted rows, top indices, actions.
- `app_skirmish_shell_render.rs`: app-level renderer using shell assets, preview textures, and text.
- `render/skirmish_shell_chrome.rs`: loads retail shell art into the atlas.
- `app.rs`: routes input and modal commit/cancel effects.

No `sim/` changes. No new crate. No Win32 emulation layer.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/render/skirmish_shell_chrome.rs` | Load verified `0x6B` modal background art/palette entries and expose atlas handles. |
| Modify | `src/ui/skirmish_shell/layout.rs` | Encode `0x6B` rects, `19px` listbox row height, content/scrollbar geometry, and hit helpers. |
| Modify | `src/ui/skirmish_shell/state.rs` | Track real listbox top indices/highlights, modal accept/cancel state, and scrolling. |
| Modify | `src/ui/skirmish_shell/mod.rs` | Re-export new layout/state helpers. |
| Modify | `src/app_skirmish_shell_render.rs` | Render the modal as native `0x6B`: background, list frames, selected rows, text, buttons, preview placeholder. |
| Modify | `src/app.rs` | Route modal input, close/commit/cancel, and keep parent setup controls inactive while modal is open. |

## Parity-Critical Ledger

| Item | Source | Required implementation effect |
|---|---|---|
| Choose Map is a separate modal dialog `0x6B`, not a child overlay or in-place map cycler. | `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md` | Modal renderer/state should visually replace/block the parent setup controls while active. |
| `0x6B` replaces the visible parent setup shell while open; the parent `0x102` is hidden before modal entry in retail. | same | Modal-open rendering must emit only modal/common shell roles needed for `0x6B`, not parent setup rows/checkboxes/trackbars/preview/right-panel labels behind it. |
| At exact `800` width, `0x6B` uses `MnScrnLCustomizeBattle.shp` / `.PAL`, not `MnScrnLCoopGameSetup.*`; above `800`, do not stretch the 800 alternate background without verification. | same | Load/render modal-specific exact-800 assets when available; keep `>800` conservative and screenshot-verified. |
| `0x6B` dialog rect is `533x369`; all 11 resource controls are part of the native visual contract. | same | Preserve verified rects for buttons, title/status statics, headings, listboxes, and preview placeholder. |
| `0x6EB` and `0x553` are real owner-drawn listboxes, not `ComboDropWin`. | same; listbox report | Do not reuse combo dropdown row geometry for these lists. |
| Listbox row height is font height + 2, standard inferred `19px`. | `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` | Replace any stale fixed `16px` modal row assumption. |
| Selected listbox fill covers the full item content row. | listbox report | Draw full-row selected fill before row text. |
| Listbox text starts at item-left + 2 and truncates to available content width. | listbox report | Use a `+2` text inset and width clipping/truncation through shell text helpers. |
| Overflow listbox scrollbar reserves `20px` width. | listbox report | Content rows and text must not draw under scrollbar. |
| Modal row highlight does not refresh the setup preview before Use Map. | `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md` | Keep highlighted selection separate from committed parent selection. |
| Use Map commits selected globals; Cancel restores saved selection. | accept/cancel report | Keep existing saved-selection model and add tests around app-level commit/restore. |
| Create Random Map button is present and dispatches to a special flow, but generator visuals are out of scope here. | modal visual layout report | Render/recognize the button; leave existing logged unimplemented behavior unless user scopes random map generation. |

## Key Technical Decisions

- **Separate modal render branch inside the Skirmish shell renderer.** Reuse the same `render_skirmish_shell` entry point, but when `choose_map_modal.is_some()`, draw the `0x6B` modal composition as a replacement shell state. Do not emit parent `0x102` setup roles behind it.
- **Dedicated listbox helpers.** Add `ChooseMapListboxId`, listbox content rect, scrollbar rect, visible-row count, row rect, and row hit-test helpers instead of reusing combo dropdown helpers.
- **Keep modal state in `SkirmishShellState`.** The current `ChooseMapModalState` is the right ownership location. Extend it rather than introducing app-level duplicate selection state.
- **Asset loading stays in the existing chrome atlas.** Add optional `choose_map_background_800_customize_battle` and any necessary modal palette-derived entries to `SkirmishShellChromeAtlas`. Missing assets should degrade to the current dark fallback, with a warning.
- **Random Map remains a recognized no-op.** Rendering and hit testing should match the button location, but actual RMG is not part of this visual parity pass.
- **Scrollbar interaction is not optional for overflowing lists.** Implement normal visible listbox scrolling behavior needed to use long map lists: wheel, arrow clicks, track clicks, and thumb drag if the existing scrollbar state can support it. Only hold-to-repeat timing may remain a named follow-up.

## Risk Areas

- **Accidentally applying combo facts to listboxes:** `ComboDropWin` and real `LISTBOX` have separate row paint contracts. Keep helpers and tests named separately.
- **Preview refresh semantics:** browsing rows must not change `skirmish_shell_state.selected_map_idx` or the cached preview texture. Only Use Map commit does that.
- **High-res background assumptions:** `0x6B` loads the alternate `MnScrnLCustomizeBattle.shp` only at exact `800`. Do not stretch the 800 background above 800 unless a verified report says to.
- **Interaction during modal:** parent setup controls must not receive clicks or hover/drag state while the modal is open.
- **Stale row-height constants:** if old tests expect `16px`, update them to the verified `19px` contract and source the change in test names/comments.

---

## Tasks

### Task 1: Rename and lock the Choose Map row-height contract

**Why:** The current modal helpers need to reflect the real owner-drawn listbox contract before rendering/input work builds on them.

**Files:**

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Keep or rename `CHOOSE_MAP_LIST_ROW_H` to make the native contract explicit, e.g. `CHOOSE_MAP_LISTBOX_ROW_H`.
2. Set it to `19`.
3. Update `choose_map_modal_list_row_at` or replace it with a helper that uses top index and the native listbox row height.
4. Update layout tests currently covering modal list row hit testing.

**Tests to add/update:**

- `choose_map_modal_list_hit_test_uses_verified_owner_draw_row_height`
- `choose_map_modal_listbox_has_11_full_rows_with_2px_remainder`

**Verification:**

```powershell
cargo test choose_map_modal --lib
```

### Task 2: Add native listbox geometry helpers

**Why:** Choose Map listboxes need content and scrollbar geometry separate from combo dropdowns.

**Files:**

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add:
   - `ChooseMapListboxId::{Mode0x6eb, Map0x553}`
   - `choose_map_listbox_rect(layout, id) -> RectPx`
   - `choose_map_listbox_visible_row_count(rect) -> usize`
   - `choose_map_listbox_needs_scrollbar(row_count, rect) -> bool`
   - `choose_map_listbox_scrollbar_rect(rect, row_count) -> Option<RectPx>`
   - `choose_map_listbox_content_rect(rect, row_count) -> RectPx`
   - `choose_map_listbox_row_rect(content, visible_row) -> RectPx`
2. Use the verified `20px` scrollbar width. Reuse an existing constant only if the name clearly applies to listboxes too; otherwise create `LISTBOX_SCROLLBAR_W`.
3. Keep row rects in content width, not full control width when scrollbar exists.

**Tests to add:**

- `choose_map_listbox_scrollbar_reserves_20_px_when_overflowing`
- `choose_map_listbox_row_rect_uses_content_width_not_scrollbar`
- `choose_map_listbox_no_scrollbar_for_11_rows_in_211_px`

**Verification:**

```powershell
cargo test choose_map_listbox --lib
```

### Task 3: Extend `ChooseMapModalState` for listbox top indices and scrolling

**Why:** Rendering and hit testing need explicit top indices for the two real listboxes.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Keep `mode_top_index` and `map_top_index`; add helper methods instead of direct field arithmetic where possible:
   - `mode_row_count(&self, modes: &[SkirmishGameMode])`
   - `map_row_count(&self)`
   - `top_index(&self, id: ChooseMapListboxId) -> usize`
   - `set_top_index_clamped(...)`
   - `scroll_listbox_by_rows(...)`
2. When selecting a mode, reset/clamp `map_top_index` and keep highlighted row visible.
3. When selecting a map row, do not commit parent selection.
4. If row count shrinks, clamp top index.

**Tests to add/update:**

- `choose_map_modal_select_mode_clamps_map_top_index`
- `choose_map_modal_scroll_clamps_to_last_visible_page`
- `choose_map_modal_highlight_does_not_commit_parent_selection`

**Verification:**

```powershell
cargo test choose_map_modal --lib
```

### Task 4: Load verified `0x6B` modal background assets

**Why:** The modal cannot look native while it is rendered as a generic dark panel.

**Files:**

- `src/render/skirmish_shell_chrome.rs`
- `src/app_skirmish_shell_render.rs`

**Steps:**

1. In `SkirmishShellChromeAtlas`, add optional exact-800 entries for:
   - `MnScrnLCustomizeBattle.shp` frame 0
   - any palette-specific rendered variant needed by the existing atlas packing path
2. Load `MnScrnLCustomizeBattle.PAL` through the existing named-palette helpers.
3. Do not use this entry as a universal background. At `>800`, preserve the verified no-stretch/uncertain behavior until runtime evidence says otherwise.
4. Keep the entry optional; missing retail asset should log and preserve fallback rendering.
5. Add a small role enum/test hook if existing semantic order tests need to assert modal background role.

**Tests to add/update:**

- `skirmish_choose_map_modal_asset_role_is_customize_battle_not_coop_setup`
- If role helpers exist: `choose_map_modal_parent_background_role_matches_width_contract`
- `choose_map_modal_does_not_stretch_800_customize_battle_background_above_800`

**Verification:**

```powershell
cargo test choose_map_modal_asset --lib
```

### Task 5: Render `0x6B` modal background as a separate shell surface

**Why:** Retail hides the setup shell and shows a fullscreen modal. Rendering over active setup controls reads wrong visually and makes input bugs easier.

**Files:**

- `src/app_skirmish_shell_render.rs`

**Steps:**

1. Split modal rendering into a named helper, e.g. `build_choose_map_modal_instances(...)`.
2. Add a semantic role helper or test-visible draw-role list for modal-open frames, similar to the existing shell semantic tests.
3. When modal is open, draw:
   - shell/modal background first;
   - modal right-panel/buttons/listbox frames/preview placeholder next;
   - listbox selected fills before list text;
   - modal text last with scissor/clipping per text rect.
4. Do not draw any parent `0x102` setup roles while modal is active: no parent row controls, checkboxes, trackbars, setup preview, setup right-panel title/game-type/map labels, or parent shell buttons behind the modal.
5. Keep preview texture behavior unchanged for now: the modal should show the currently committed preview, not highlight-driven preview changes.

**Tests to add/update:**

- `choose_map_modal_draw_order_starts_with_modal_background`
- `choose_map_modal_draw_order_does_not_emit_parent_setup_controls`
- `choose_map_modal_draw_order_emits_no_0x102_roles_when_open`

**Verification:**

```powershell
cargo test choose_map_modal_draw --lib
```

### Task 6: Render owner-drawn listbox frames, selected rows, and row text

**Why:** The two listboxes are the center of the modal; they need to look like real shell controls.

**Files:**

- `src/app_skirmish_shell_render.rs`
- `src/ui/skirmish_shell/layout.rs` if helper gaps appear

**Steps:**

1. Add a helper for listbox primitive frame drawing. Reuse existing primitive bevel/pixel atlas entries if available.
2. Add selected-fill instances spanning the full content row rect.
3. Add row text at `row_rect.x + 2`, vertically centered.
4. Clip row text to content rect width, excluding scrollbar when present.
5. Preserve source order for map rows and current order for mode rows.

**Tests to add/update:**

- `choose_map_listbox_selected_fill_uses_full_content_row`
- `choose_map_listbox_text_uses_two_pixel_inset`
- `choose_map_listbox_map_rows_preserve_source_order`

**Verification:**

```powershell
cargo test choose_map_listbox --lib
```

### Task 7: Add native listbox scrollbar rendering and interaction

**Why:** Many map lists overflow. Without scrollbar/content shrink, the modal still looks non-native on real installs.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/app_skirmish_shell_render.rs`
- `src/app.rs`

**Steps:**

1. Render a child scrollbar when row count exceeds visible rows.
2. Reuse existing scrollbar atlas pieces if they are the same verified shell scrollbar assets. If not, add explicit entries in the chrome atlas.
3. Route mouse wheel while modal is open to the hovered or active listbox.
4. Add click handling for scrollbar up/down arrows: one row per click, clamped.
5. Add track-click handling: move top index/page according to the verified owner-draw scrollbar contract or the closest already-verified shell scrollbar helper.
6. Add thumb drag if the existing combo scrollbar state can be reused cleanly without coupling listboxes to combo dropdowns. If this proves larger than expected, leave only hold-to-repeat timing as a named follow-up; do not defer arrow/track interaction.

**Tests to add/update:**

- `choose_map_listbox_wheel_scrolls_map_list_when_modal_open`
- `choose_map_listbox_arrow_click_scrolls_one_row`
- `choose_map_listbox_track_click_scrolls_without_exceeding_bounds`
- `choose_map_listbox_scrollbar_thumb_clamps_top_index`
- `choose_map_modal_mouse_wheel_does_not_scroll_parent_combo`

**Verification:**

```powershell
cargo test choose_map_listbox --lib
```

### Task 8: Fix modal input routing and parent blocking

**Why:** The player-visible modal contract includes blocking parent setup controls. Parent controls should not change while the chooser is active.

**Files:**

- `src/app.rs`
- `src/ui/skirmish_shell/state.rs`

**Steps:**

1. Ensure mouse down/up/move/wheel return early through modal handlers while `choose_map_modal.is_some()`.
2. Prevent parent dropdowns, trackbar drag, and pressed owner-draw button state from surviving modal open.
3. Add modal listbox hit testing with top-index offsets.
4. Keep Cancel and Use Map button sounds aligned with existing shell button sound behavior.

**Tests to add/update:**

- `choose_map_modal_open_clears_parent_drag_and_dropdown_state`
- `choose_map_modal_clicking_parent_control_area_does_not_mutate_parent_shell`
- `choose_map_modal_map_row_hit_test_includes_top_index`

**Verification:**

```powershell
cargo test choose_map_modal --lib
```

### Task 9: Add full `0x6B` resource control matrix rendering/tests

**Why:** The modal can pass listbox tests while still missing visible native controls and labels. Retail `0x6B` has 11 controls; all should have an explicit rect/render responsibility.

**Files:**

- `src/ui/skirmish_shell/layout.rs`
- `src/app_skirmish_shell_render.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add named rects or helper accessors for all verified `0x6B` controls:
   - Cancel `0x5C0` at `(425,346,108,23)`;
   - Use Map `0x6C5` at `(425,122,108,23)`;
   - Create Random Map `0x583` at `(425,149,108,23)`;
   - title static `0x694` at `(425,1,108,10)`;
   - `GUI:SelectEngagement` static at `(80,20,257,12)`;
   - mode listbox `0x6EB` at `(77,78,130,211)`;
   - map listbox `0x553` at `(225,78,130,211)`;
   - `GUI:GameType` heading at `(77,60,130,10)`;
   - `GUI:GameMap` heading at `(225,60,130,10)`;
   - status/help static `0x695` at `(2,355,303,12)`;
   - preview static `0x468` at `(428,23,96,69)`.
2. Render the static text labels with shell text helpers and native clipping rects.
3. Keep `0x468` as the modal preview placeholder/current committed preview; do not treat it as the setup preview rect.
4. Do not add setup-only labels `0x6EC` or `0x5A8`; they are not present in `0x6B`.

**Tests to add/update:**

- `choose_map_modal_uses_resource_0x6b_control_rects`
- `choose_map_modal_has_no_setup_0x6ec_or_0x5a8_labels`
- `choose_map_modal_preview_static_uses_0x6b_x428_rect`
- `choose_map_modal_static_labels_have_verified_rects`

**Verification:**

```powershell
cargo test choose_map_modal --lib
```

### Task 10: Preserve accept/cancel and preview refresh semantics

**Why:** This is the most important non-visual behavior inside the visual pass. Native feel breaks if browsing rows changes the parent preview immediately.

**Files:**

- `src/app.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app_skirmish_shell_render.rs` only if preview cache handling needs a small hook

**Steps:**

1. Keep `ChooseMapModalState::saved_selection`.
2. On Cancel `0x5C0`, close without committing, without changing the parent selected map/mode, and without invalidating `skirmish_preview_texture` when the parent committed selection is unchanged.
3. On Use Map `0x6C5`, commit selected record and mode, update `selected_map_idx`, and clear `skirmish_preview_texture`.
4. On row highlight or mode highlight, do not clear `skirmish_preview_texture`.
5. Keep Create Random Map logged/unimplemented unless a random-map sentinel already exists and is safe to show as a row. Do not fake generation visuals.

**Tests to add/update:**

- `choose_map_modal_cancel_restores_saved_selection`
- `choose_map_modal_cancel_does_not_invalidate_preview_cache`
- `choose_map_modal_use_map_commits_and_invalidates_preview_cache`
- `choose_map_modal_row_highlight_does_not_invalidate_preview_cache`
- `choose_map_modal_create_random_map_remains_mode_gated_without_generation`

**Verification:**

```powershell
cargo test choose_map_modal --lib
```

### Task 11: Manual visual verification

**Why:** The goal is native visual progress. Unit tests pin geometry; the final check must look at the actual shell.

**Steps:**

1. Build and run with the dev Skirmish shell enabled.
2. At `800x600`, open Skirmish setup, click Choose Map, and verify:
   - setup controls are not visibly active over the modal;
   - modal background uses Customize Battle shell art when assets are present;
   - two listboxes are aligned and row height looks native;
   - selected row fill spans the content row;
   - Use Map, Cancel, and Create Random Map are in the verified right column.
3. At `1024x768`, repeat and verify:
   - modal remains centered in the shell coordinate model;
   - no unverified 800-background stretching is introduced;
   - listboxes and text do not drift.
4. Use Map on a different map:
   - modal closes;
   - parent setup selected map changes;
   - preview refreshes after commit.
5. Cancel after highlighting a different row:
   - modal closes;
   - parent map and preview remain old.

**Suggested commands:**

```powershell
cargo test choose_map --lib
cargo check
```

If a browser/screenshot workflow is available for this native window, capture `800x600` and `1024x768` reference screenshots under `docs/visual-checks/skirmish-choose-map/`. Do not add generated screenshots unless they are useful for review.

## Out Of Scope

- Making the native shell the default route.
- Implementing random map generation.
- Post-launch MPModes callbacks, start-unit budget, fallback starts, or spawn parity.
- RA2MD.INI persistence.
- Player-name edit control behavior.
- Scrollbar hold-to-repeat timing, only if arrow clicks, track clicks, wheel scrolling, and top-index clamping are implemented and tested first.

## Completion Criteria

- `cargo test choose_map --lib` passes.
- `cargo check` passes or any unrelated existing failure is documented.
- Opening Choose Map in the dev native Skirmish shell visibly presents a separate `0x6B`-shaped modal, not a generic overlay.
- Use Map and Cancel match the verified commit/restore behavior.
- No `sim/` dependencies on UI/render/audio/net are introduced.
