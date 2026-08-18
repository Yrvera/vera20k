# Skirmish Shell Background/Text/Preview Implementation Plan

## Superseded Button-Art Note - 2026-05-24

The Start/Choose/Back `bue_*30` / `bde_*30` button-art expectations in this
older plan are superseded for the standard Skirmish setup sidebar. Use
`docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`:
these three right-panel buttons are owner-draw type `1` and use `SDBTNANM.SHP`
frames `2`/`4`.

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not commit unless the user explicitly asks for commits in the execution session.

**Goal:** Finish the dev-only Skirmish shell renderer so its background, right panel, owner-draw buttons, text, flags, and preview-marker behavior only use assets and paths verified active in standard offline YR Skirmish dialog `0x102`. Unknown backgrounds and preview backing stay blank/debug-only, never substituted.

**Architecture:** UI/render/app/map-metadata work only. `ui/skirmish_shell` owns render-agnostic layout/state/hit testing, `render/skirmish_shell_chrome` owns retail asset decode and atlas entries, and `app_skirmish_shell_render` owns draw-instance order. This plan does not touch `src/sim/`.

**Design Doc:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`

**Primary Research:** `docs/research/SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`

**Superseded Research Caveat:** `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md` is still useful for owner-draw/text/preview ordering, but its 640 parent-background identification is stale. The GREEN-audited follow-up proves `MNSCRNS.SHP`, not `MNSCRNL.SHP`, is the active 640 offline Skirmish parent.

**Supersedes:**
- `docs/plans/2026-05-17-skirmish-shell-live-active-render-path-plan.md` where the task list is broader than the current partially implemented shell path.
- `docs/plans/2026-05-17-skirmish-shell-verified-assets-plan.md` where parent backgrounds were still unresolved.

---

## Grounding Summary

- The existing pixel-parity design remains architecturally valid: the shell is a dedicated renderer, not a Win32 dialog host, and it follows the sidebar-style split between layout/state, render assets, and app glue.
- Current repo state has already partially implemented the live-active plan. `src/app.rs` has a default-off `RA2_DEV_SKIRMISH_SHELL` gate, shell mouse down/up routing, and lazy chrome loading; `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, and `src/ui/skirmish_shell/` contain shell scaffolding and tests.
- Live Ghidra verified offline Skirmish launcher `0x006AE2C0` calls `0x0072CF40` before creating dialog `0x102`, then later calls cleanup `0x0072CF90`.
- `0x0072CF40` actively loads `MnScrnLCoopGameSetup.PAL` for offline Skirmish and loads `MnScrnLCoopGameSetup.shp` only when `g_ScreenWidth == 800`.
- `0x0060CF00` initializes dialog `0x102` parent state with the `FUN_0072D030()` palette convert object, `DAT_00B0FB50` as the small parent/background SHP pointer, and `DAT_00B0FA18` as the alternate SHP pointer. The exact byte-offset shorthand remains sourced from `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`; the output-determining pointers are verified in the follow-up.
- `0x0072DFB0` maps `DAT_00B0FB50` to `MNSCRNS.SHP` and `DAT_00B0FA04` to `MNSCRNL.SHP`. Therefore `Background_Overlay @ 0x0072E730` selects `MNSCRNS.SHP` for 640-wide offline Skirmish and `MnScrnLCoopGameSetup.shp` for exact 800-wide offline Skirmish.
- For `>800`, `Background_Overlay` still selects `DAT_00B0FA18`, but `0x0072CF40` only loads that pointer at exact width 800 and `CC_Draw_Shape @ 0x004AED70` returns early for null SHP pointers. Keep the fresh `>800` parent background blank/no-draw unless runtime watchpoints prove stale non-null pointer history.
- `MNSCRNL.SHP`, `MnScrnLCustomizeBattle.shp`, `dbak6440.pcx`, `dlgsysa.pcx`, and `dlgsysi.pcx` are not verified as the standard 640 offline Skirmish parent/background. `MNSCRNL.SHP` remains a generic large common-shell helper/layout-dimension asset, not the 640 dialog `0x102` parent.
- Right-panel SHP palette split is now verified: `SHELL.PAL` for `SDTP.SHP`, `SDBTM.SHP`, and `LWSCRNS/LWSCRNL`; `SHELL2.PAL` for `SDBTNBKGD.SHP`; `SDBTNANM.PAL` for `SDBTNANM.SHP` frame `10`.
- Owner-draw buttons are verified at `0x00612B70`: Start/Choose/Back use `bue_li30/mi30/ri30.pcx` unpressed, `bde_li30/mi30/ri30.pcx` pressed, tiled middle, native caps, disabled alpha `0x80`, and a pressed content y-offset of `+2`.
- Shell text wrapper `0x00621040` performs RGB-to-display-format conversion, rect clipping, vertical centering flag handling, and lower bitfont horizontal alignment. The current Rust sidebar-font helper is a temporary approximation that must be isolated behind shell-specific helper functions.
- Preview paint order is verified: common shell/background first, then map preview surface, then `STARTBUT.SHP` frame `0`, then numeric labels. `mmpb.shp` is an assigned-player/house marker path elsewhere and is not a default offline Skirmish preview backing or available-start marker.
- INI does not drive shell art names or layout. Map INI `[Waypoints]`, `[Preview]`, and `[PreviewPack]` matter for future preview/marker projection. `[Map] LocalSize` is a candidate for source bounds but is not verified as the exact menu preview source.

## Key Technical Decisions

- **Keep the shell default-off.** `RA2_DEV_SKIRMISH_SHELL` remains the only shell gate until screenshot parity is proven. Default user-visible Main Menu continues through the existing egui path.  
  **Confidence:** high. **Source:** current `src/app.rs` gate and project policy from `AGENTS.md`.
- **Render only verified parent-background widths.** At 640 render `MNSCRNS.SHP`; at 800 render `MnScrnLCoopGameSetup.shp`; above 800 render no parent background unless runtime evidence proves `DAT_00B0FA18` can be non-null in a fresh standard Skirmish entry.  
  **Confidence:** high for 640/800 and null no-draw, medium for stale-pointer runtime history above 800. **Source:** GREEN-audited follow-up report `0x0072DFB0`, `0x0072CF40`, `0x0072CF90`, `0x0060CF00`, `0x0072E730`, `0x004AED70`.
- **Keep parent-background palette explicit.** Parent-background SHPs decode through `MnScrnLCoopGameSetup.PAL`. Do not substitute `sidebar.pal`, `SHELL.PAL`, or right-panel palettes for parent backgrounds.  
  **Confidence:** high. **Source:** primary Ghidra report `0x0072CF40`, `0x0072ADE0`, `0x0060CF00`.
- **Use verified right-panel palettes.** Decode `SDTP.SHP`, `SDBTM.SHP`, and `LWSCRNS/LWSCRNL` with `SHELL.PAL`; decode `SDBTNBKGD.SHP` with `SHELL2.PAL`; decode `SDBTNANM.SHP` frame `10` with `SDBTNANM.PAL`. Do not substitute `SIDEBAR.PAL` or parent-background palettes for these SHPs.  
  **Confidence:** high. **Source:** GREEN-audited follow-up report `0x0072E450`, `0x0072ADE0`, palette table `0x00844BE4..0x00844BEC`.
- **Use semantic atlas fields.** Atlas entries should name verified roles, not guesses: `background_640_mnscrns`, `background_800_coop_game_setup`, `right_panel_top_sdtp`, `right_panel_tile_sdbtnbkgd`, `right_panel_overlay_sdbtnanm_frame10`, `right_panel_bottom_sdbtm`, `lower_side_640_lwscrns`, `lower_side_large_lwscrnl`, `start_marker`, and `assigned_player_marker_mmpb`.  
  **Confidence:** high. **Source:** current repo pattern in `src/render/skirmish_shell_chrome.rs` plus GREEN-audited asset-role findings.
- **Do not draw `mmpb.shp` in the default preview path.** It may be loaded as a separately named future assigned-player marker, but it must not stand in for preview backing or `STARTBUT.SHP`.  
  **Confidence:** high. **Source:** primary Ghidra report `0x00640710`, `0x00640A40`, and caller checks.
- **Do not fake preview surfaces.** `STARTBUT.SHP` and numeric labels draw only after a real map preview surface and verified source bounds are available.  
  **Confidence:** high. **Source:** primary Ghidra report `0x00640710` draw order and unresolved preview source-bounds mapping.
- **Keep map preview source bounds empty until verified.** `MapMenuEntry.preview_source_bounds` should remain `None` unless a later binary/screenshot pass proves the source mapping, including all origin and size fields.  
  **Confidence:** high for not guessing; low for the eventual source identity. **Source:** current `src/app_list_maps.rs` behavior and primary Ghidra report unresolved question.

## Open Questions

### Resolved By The New Ghidra Report

- `MNSCRNS.SHP` is active for offline Skirmish at 640-wide shell mode.
- `MNSCRNL.SHP` is not the 640 dialog `0x102` parent; it is a generic large common-shell helper/layout asset.
- `MnScrnLCoopGameSetup.shp` and `MnScrnLCoopGameSetup.PAL` are active for offline Skirmish at 800-wide shell mode.
- `LWSCRNS.SHP` and `LWSCRNL.SHP` are active lower/side strip assets, selected at 640 and non-640 widths respectively.
- Right-panel palettes are verified: `SHELL.PAL`, `SHELL2.PAL`, and `SDBTNANM.PAL` split by SHP role.
- `MnScrnLCustomizeBattle.shp` is not the standard offline Skirmish dialog `0x102` background.
- `mmpb.shp` is not the default offline Skirmish available-start marker or preview backing.
- Owner-draw buttons use `bue_*30` and `bde_*30`, not `bud_*` for the normal disabled path.

### Deferred To Follow-Up Verification

- Player-visible high-resolution `>800` parent-background behavior when `DAT_00B0FA18` is not loaded by the exact-800 branch.
- Exact shell font identity and final RGB parity for button labels after the display-format conversion.
- Exact map preview decode and source-bounds mapping used by `DrawStartPositions`.
- Whether any later paint path draws `mmpb.shp` in a different non-default or assigned-player customization state.

## Current Repo State Notes

- `src/app.rs` already has `DEV_SKIRMISH_SHELL_ENV: &str = "RA2_DEV_SKIRMISH_SHELL"`, default-off parsing, startup/lazy shell atlas loading, and mouse down/up owner-draw button handling.
- `src/render/skirmish_shell_chrome.rs` may still carry stale `background_640_mnscrnl` naming from the pre-follow-up plan. Implementation must rename/repoint that role to `background_640_mnscrns` and keep `MNSCRNL.SHP` out of the standard 640 parent path.
- `src/render/skirmish_shell_chrome.rs` should expose separate atlas entries for the verified right-panel assets and palettes: `SDTP.SHP`/`SHELL.PAL`, `SDBTNBKGD.SHP`/`SHELL2.PAL`, `SDBTNANM.SHP` frame `10`/`SDBTNANM.PAL`, `SDBTM.SHP`/`SHELL.PAL`, `LWSCRNS.SHP`/`SHELL.PAL`, and `LWSCRNL.SHP`/`SHELL.PAL`.
- `src/app_skirmish_shell_render.rs` already suppresses `STARTBUT.SHP` marker drawing until real preview data exists. Its parent-background selector must be checked for stale `MNSCRNL` semantics and updated to `MNSCRNS` at 640.
- `src/app_skirmish_shell_render.rs` does not yet reproduce the full verified right-panel order: `SDBTNANM.SHP` frame `10` overlay and the lower/width-selected panel piece need explicit handling.
- `src/app_list_maps.rs` already exposes sorted multiplayer start waypoints and intentionally keeps `preview_source_bounds` as `None` because `[Map] LocalSize` is not verified as the menu-preview source.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/render/skirmish_shell_chrome.rs` | Retail shell asset loading, role classification, atlas fields, verified palette bindings |
| Modify | `src/app_skirmish_shell_render.rs` | Parent-background selection, right-panel order, button composition, shell text placement, preview-marker suppression |
| Modify | `src/ui/skirmish_shell/layout.rs` | Verified dialog/control coordinates and render-agnostic helper tests |
| Modify if needed | `src/ui/skirmish_shell/state.rs` | Pressed button state and hit-testing contracts only |
| Modify if needed | `src/app.rs` | Dev-gated shell routing and input clearing only |
| Modify if needed | `src/app_list_maps.rs` | Map menu metadata only after source fields are verified |
| Modify if needed | `src/app_init.rs` | `MapMenuEntry` metadata only; no gameplay behavior |
| Do not modify | `src/sim/` | Out of scope |

## Interface Changes

- Preserve `RA2_DEV_SKIRMISH_SHELL` as the single shell feature gate.
- Keep `SkirmishShellChromeAtlas` as the only render-facing shell asset bundle.
- Keep `SkirmishShellState.pressed_owner_draw_button: Option<OwnerDrawButton>` as render-agnostic UI state.
- Keep `MapMenuEntry.multiplayer_start_waypoints` and `MapMenuEntry.preview_source_bounds` as lightweight menu metadata. Do not add gameplay dependencies to these fields.
- Do not change public simulation APIs.

## Sim Checklist

This plan does not touch `src/sim/`.

- [x] No fixed-point or floating-point simulation math changes.
- [x] No deterministic state hash changes.
- [x] No dependency from `sim/` to `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- [x] No tick-ordering impact.
- [x] No `EntityStore` iteration impact.

## Risk Areas

- The working tree is dirty and shell files are already modified/untracked. Preserve unrelated changes and only edit files named by this plan.
- Broad `cargo fmt` can touch unrelated dirty files. Prefer targeted formatting only for files actually edited.
- Parent-background substitutions are easy to miss visually. Tests must assert that 640 selects `MNSCRNS.SHP`, 800 selects `MnScrnLCoopGameSetup.shp`, and 1024-wide layouts select no fresh parent background.
- The current shell text path uses the sidebar font texture and no per-glyph clipping. Keep that mismatch isolated so a later exact shell bitfont path can replace it cleanly.
- `SDBTNANM.SHP` currently renders frame `0` in atlas construction, but Ghidra reports a frame `10` overlay in the common shell path. Implementation must either load frame `10` separately or document why the overlay is still deferred.
- Current lower-strip selection in Rust may still be width-limited to 640/800. The verified binary rule is 640 uses `LWSCRNS.SHP`; every non-640 width, including 800 and 1024, uses `LWSCRNL.SHP`.
- `SDMPBTN.SHP` is right-panel chrome, not a preview backing. Avoid naming or drawing it as map preview content.
- `STARTBUT.SHP` can be correctly ordered only after the preview surface exists. Drawing it over blank content would create a false parity signal.

## Parity-Critical Items

| Task # | Item | Player-visible reason | Verification |
|--------|------|-----------------------|--------------|
| 1 | Dev gate and default route | Players should not see unverified shell pixels by default | Run without `RA2_DEV_SKIRMISH_SHELL`; egui menu appears |
| 2 | Asset roles and palettes | Wrong backgrounds/palettes dominate first-viewport pixels | Unit tests and retail asset load smoke tests |
| 3 | Parent-background selector | 640/800 must match YR; 1024 must not substitute | Unit test for 640/800/1024 roles and asset names |
| 4 | Right-panel order | Repeated shell chrome forms the right-side frame | Unit-level instance order test or inspected render instances |
| 5 | Button composition | Tiled center and pressed state are obvious during clicks | Segment tests and manual click/hold check |
| 6 | Text wrapper isolation | Labels must stay centered and move with pressed buttons | Position tests and screenshot comparison |
| 7 | Preview policy | Wrong preview backing or marker order misrepresents map selection | Code tests assert no markers without real preview |
| 8 | Map metadata | Future markers need correct sorted starts but no guessed source bounds | Existing waypoint metadata tests |

---

## Tasks

### Task 1: Reconfirm The Default-Off Shell Gate

**Why:** The shell remains experimental until verified screenshot parity. The default route must not load or render shell chrome.

**Files:**
- Inspect/modify if needed: `src/app.rs`

**Steps:**

1. Confirm `DEV_SKIRMISH_SHELL_ENV` is exactly `RA2_DEV_SKIRMISH_SHELL`.
2. Confirm `App::dev_skirmish_shell_enabled()` treats missing, empty, `0`, `false`, `off`, and `no` as disabled.
3. Confirm startup atlas construction runs only when the gate is enabled.
4. Confirm `GameScreen::MainMenu` renders the existing egui menu when the gate is disabled.
5. Confirm shell mouse down/up routing only runs when `state.screen == GameScreen::MainMenu && state.dev_skirmish_shell_enabled`.
6. If any point fails, make the narrowest change in `src/app.rs` and add no new gate.

**Verification:**
- Code inspection for the six points above.
- Manual run without `RA2_DEV_SKIRMISH_SHELL`: main menu uses egui setup path.
- Manual run with `RA2_DEV_SKIRMISH_SHELL=1`: shell path renders if assets load.

### Task 2: Harden Shell Asset Roles And Palette Boundaries

**Why:** The renderer must distinguish active offline Skirmish assets from generic shell and research candidates.

**Files:**
- Modify: `src/render/skirmish_shell_chrome.rs`

**Steps:**

1. Rename or add the verified 640 parent-background field as `background_640_mnscrns`, loaded from `MNSCRNS.SHP`.
2. Keep `background_800_coop_game_setup`, loaded from `MnScrnLCoopGameSetup.shp`.
3. Keep `MNSCRNL.SHP` out of the standard 640 parent-background role. If it is loaded for generic/common-shell layout support, name it as a large/common-shell helper, not as Skirmish 640 parent art.
4. Decode both active parent-background SHPs through the parent convert path returned by `FUN_0072D030()` / `DAT_00B0FCE0`, backed by `MnScrnLCoopGameSetup.PAL`.
5. Add right-panel atlas entries with verified palette bindings:
   - `right_panel_top_sdtp`: `SDTP.SHP` frame `0`, `SHELL.PAL`
   - `right_panel_tile_sdbtnbkgd`: `SDBTNBKGD.SHP` frame `0`, `SHELL2.PAL`
   - `right_panel_overlay_sdbtnanm_frame10`: `SDBTNANM.SHP` frame `10`, `SDBTNANM.PAL`
   - `right_panel_bottom_sdbtm`: `SDBTM.SHP` frame `0`, `SHELL.PAL`
   - `lower_side_640_lwscrns`: `LWSCRNS.SHP` frame `0`, `SHELL.PAL`
   - `lower_side_large_lwscrnl`: `LWSCRNL.SHP` frame `0`, `SHELL.PAL`
6. Keep `start_marker` for `STARTBUT.SHP`.
7. Keep `assigned_player_marker_mmpb` separate from `start_marker`; do not consume it from the default preview path.
8. Load owner-draw PCXs through their embedded PCX palettes, with transparent index `0`.
9. Keep `MnScrnLCustomizeBattle.shp`, `dbak6440.pcx`, `dlgsysa.pcx`, and `dlgsysi.pcx` out of default render fields.
10. Remove the duplicated `mmpb.shp` negative assertion while preserving the positive `AssignedPlayerMarker` assertion.

**Verification:**
- `cargo test skirmish_shell_asset_classification_matches_live_render_path`
- Ignored retail smoke tests may be run locally with `--ignored` if retail assets are available.

### Task 3: Enforce Parent-Background Width Selection

**Why:** The most visible shell background is verified only for 640 and 800. High resolution must not silently borrow the wrong art.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`

**Steps:**

1. Replace stale `ParentBackgroundRole::Mnscrnl640` naming with `ParentBackgroundRole::Mnscrns640`.
2. Ensure `parent_background_role()` returns:
   - `Some(Mnscrns640)` for width `640`
   - `Some(CoopGameSetup800)` for width `800`
   - `None` for every other width
3. Draw the selected background at native size and origin; do not stretch or fit it.
4. Add a once-only log for the unresolved high-resolution blank-background case if the current logging is insufficient.
5. Do not use `MNSCRNL.SHP`, `MnScrnLCustomizeBattle.shp`, `dbak6440.pcx`, `dlgsysa.pcx`, or `dlgsysi.pcx` as fallback.

**Verification:**
- `cargo test parent_background_role_uses_only_verified_widths`
- Manual dev-shell screenshot check at 640x480, 800x600, and 1024x768.

### Task 4: Complete The Verified Right-Panel Draw Order

**Why:** The right panel is a repeated shell surface. Missing frame overlays or bottom pieces alter the first screen before any interaction.

**Files:**
- Modify: `src/render/skirmish_shell_chrome.rs`
- Modify: `src/app_skirmish_shell_render.rs`
- Modify if pure layout helper is needed: `src/ui/skirmish_shell/layout.rs`

**Steps:**

1. In the atlas, represent `SDTP.SHP` frame `0`, `SDBTNBKGD.SHP` frame `0`, `SDBTM.SHP` frame `0`, `SDBTNANM.SHP` frame `10`, `LWSCRNS.SHP` frame `0`, and `LWSCRNL.SHP` frame `0` as separately named entries.
2. Decode those entries with the verified palette split: `SHELL.PAL` for `SDTP/SDBTM/LWSCRNS/LWSCRNL`, `SHELL2.PAL` for `SDBTNBKGD`, and `SDBTNANM.PAL` for `SDBTNANM` frame `10`.
3. If frame `10` cannot be decoded with the current SHP parser, keep the field absent and log a warning that names `SDBTNANM.SHP` frame `10`; do not substitute frame `0`.
4. In the renderer, draw the parent background first.
5. Draw the right-panel top piece.
6. Draw exactly `layout.right_panel.tile_count` tile pieces in order from top to bottom.
7. If the atlas has `SDBTNANM.SHP` frame `10` and the verified boolean branch is active, draw it tiled once per right-panel tile row at the overlay rect, advancing by the overlay rect height each row. Do not draw it as a single stretched or single-row overlay.
8. Draw `SDBTM.SHP` frame `0` at the verified bottom-cap rect after the tiled overlay.
9. Draw the separate lower/side piece after `SDBTM.SHP`: use `LWSCRNS.SHP` when `g_ScreenWidth == 640`, and `LWSCRNL.SHP` for every non-640 width, including 800 and 1024. Derive the rect from the `RightPanel__ComputeLayoutRects @ 0x0072EC70` behavior; if the current layout layer does not yet expose that rect, keep this as a named deferral and do not substitute `SDMPBTN.SHP`.
10. Replace any stale lower-strip role naming/tests such as `Lwscrnl800` or "verified widths" with non-640/large semantics. Tests must assert `LWSCRNS` at 640 and `LWSCRNL` at both 800 and 1024.
11. Keep `SDMPBTN.SHP` out of the map preview; if it is still loaded, treat it as right-panel chrome only.

**Verification:**
- Add a deterministic test hook/helper that records semantic draw roles in order; do not rely on manual inspection as the only verification for right-panel order.
- Assert the role order: parent background, `SDTP`, repeated `SDBTNBKGD`, optional repeated `SDBTNANM` frame `10` when the verified branch is active, `SDBTM`, lower strip, then buttons/flags/preview policy.
- Assert lower-strip selection separately: 640 -> `LWSCRNS`; 800 -> `LWSCRNL`; 1024 -> `LWSCRNL`.
- Run a dev-shell screenshot comparison after the semantic-order tests pass.

### Task 5: Keep Owner-Draw Button Composition Exact

**Why:** Start/Choose/Back are high-attention controls. Stretching the middle or using the wrong pressed asset is immediately visible.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Modify if hit testing changes are needed: `src/ui/skirmish_shell/state.rs`
- Modify if event clearing changes are needed: `src/app.rs`

**Steps:**

1. Keep the unpressed asset triplet as `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`.
2. Keep the pressed asset triplet as `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`.
3. Compose each button as left cap, repeated/clipped middle, right cap.
4. Keep disabled rendering on the unpressed triplet with alpha `0x80 / 255.0`; do not introduce `bud_*` as the normal disabled path.
5. Preserve mouse-down tracking and trigger button action only on mouse-up inside the same owner-draw button.
6. Clear `pressed_owner_draw_button` when shell is disabled or the screen leaves `GameScreen::MainMenu`, if current app flow leaves stale pressed state.

**Verification:**
- `cargo test button_segments_tile_middle_and_keep_caps`
- `cargo test final_middle_segment_clips_when_width_is_not_tile_multiple`
- `cargo test pressed_buttons_select_down_skin_assets`
- Manual dev-shell click/hold on Start, Choose Map, and Back.

### Task 6: Isolate Shell Text Placement

**Why:** The current sidebar text path is useful scaffolding, but shell text parity requires exact rect alignment, clipping, color, and pressed offset behavior.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`

**Steps:**

1. Keep all button-label text placement in shell-specific helpers such as `shell_text_origin()` and `push_centered_text()`.
2. Apply vertical centering for button labels.
3. Apply the pressed y-offset `+2` only when the matching owner-draw button is pressed.
4. Keep text clipped or skipped when it would exceed the verified owner-draw rect until an exact glyph-clipping implementation exists.
5. Keep the live-Ghidra recovered color value documented as pre-display-format RGB evidence, not final screenshot-proven RGB.
6. Do not scatter shell text color or offsets through call sites.

**Verification:**
- `cargo test text_origin_centers_and_applies_pressed_offset`
- `cargo test text_origin_supports_left_and_right_alignment_flags`
- Manual screenshot comparison for button label center and pressed movement.

### Task 7: Keep Preview Rendering Evidence-Gated

**Why:** The preview area is easy to overfill with plausible assets, but the verified order requires the real preview surface before markers.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Modify if metadata is verified later: `src/app_list_maps.rs`, `src/app_init.rs`, `src/map/preview.rs`

**Steps:**

1. Do not draw `SDMPBTN.SHP`, `mmpb.shp`, `dbak6440.pcx`, or any dialog PCX as a map preview backing.
2. Keep `push_start_marker_sprites()` returning without drawing when `real_preview_surface_available` is false.
3. Keep `push_start_marker_labels()` returning without drawing when `real_preview_surface_available` is false.
4. When real preview decode is later added, draw order must be preview surface, then `STARTBUT.SHP`, then numeric labels.
5. Project marker positions only from verified source bounds and destination rect math; do not use `[Map] LocalSize` until verified against retail behavior.

**Verification:**
- Add a unit test or assertion path that a false `real_preview_surface_available` produces no start-marker sprite instances.
- Code inspection that `assigned_player_marker_mmpb` is not referenced by `build_skirmish_shell_instances()`.

### Task 8: Preserve Verified Map Metadata Without Guessing Source Bounds

**Why:** Start markers eventually need multiplayer start waypoints, but marker projection must not guess source rectangles.

**Files:**
- Inspect/modify if needed: `src/app_list_maps.rs`
- Inspect/modify if needed: `src/app_init.rs`

**Steps:**

1. Keep `MapMenuEntry.multiplayer_start_waypoints` populated from `[Waypoints]` indices `0..=7`, sorted by index.
2. Keep `MapMenuEntry.preview_source_bounds` as `None` until the exact source is verified.
3. Do not parse `[Map] LocalSize` into `preview_source_bounds` in this plan.
4. Keep map-list metadata parsing independent from gameplay map loading.

**Verification:**
- `cargo test menu_entry_exposes_sorted_multiplayer_start_waypoints`
- Code inspection that `preview_source_bounds_from_verified_source()` returns `None`.

### Task 9: Run Targeted Verification

**Why:** The shell code touches rendering and app input, but unrelated dirty `src/sim/` work may make broad checks noisy.

**Files:**
- No planned file edits.

**Steps:**

1. Run `cargo test skirmish_shell -- --nocapture`.
2. Run `cargo test menu_entry_exposes_sorted_multiplayer_start_waypoints`.
3. Run `cargo check`.
4. If `cargo check` fails in unrelated dirty files, report the exact failing file and diagnostic without fixing unrelated work.
5. Run the app without `RA2_DEV_SKIRMISH_SHELL` and confirm the egui menu is still default.
6. Run the app with `RA2_DEV_SKIRMISH_SHELL=1` and verify:
   - 640 shows only the verified `MNSCRNS.SHP` parent background branch.
   - 800 shows only the verified `MnScrnLCoopGameSetup.shp` parent background branch.
   - 1024 shows no substituted parent background but still uses the verified non-640 `LWSCRNL.SHP` lower/side strip.
   - Start/Choose/Back buttons press visually with `+2` content movement.
   - No preview backing or markers appear until the real preview path exists.

## Sources & References

- `AGENTS.md`
- `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`
- `docs/plans/2026-05-17-skirmish-shell-live-active-render-path-plan.md`
- `docs/plans/2026-05-17-skirmish-shell-verified-assets-plan.md`
- `docs/research/SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`
- Ghidra-verified addresses from the primary and follow-up reports: `0x006AE2C0`, `0x0072CF40`, `0x0072CF90`, `0x0060CF00`, `0x00622B50`, `0x0072DFB0`, `0x0072E450`, `0x0072E730`, `0x0072EC70`, `0x0072ADE0`, `0x004AED70`, `0x00612B70`, `0x00621040`, `0x00640710`, `0x00640A40`.
- Current code surfaces inspected for this plan: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_init.rs`, `src/app_list_maps.rs`, `src/map/preview.rs`.
