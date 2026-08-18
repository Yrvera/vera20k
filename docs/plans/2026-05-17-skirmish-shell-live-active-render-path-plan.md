# Skirmish Shell Live Active Render Path Implementation Plan

## Superseded Button-Art Note - 2026-05-24

The Start/Choose/Back `bue_*30` / `bde_*30` button-art expectations in this
older plan are superseded for the standard Skirmish setup sidebar. Use
`docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`:
these three right-panel buttons are owner-draw type `1` and use `SDBTNANM.SHP`
frames `2`/`4`.

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not commit unless the user explicitly asks for commits in the execution session.

**Goal:** Update the dedicated Skirmish shell renderer so its visible background, button, text, and preview-marker behavior follows the live Ghidra-verified offline YR Skirmish dialog `0x102` active render path.

**Architecture:** UI/render/app work only. `ui/skirmish_shell` remains render-agnostic layout/state and hit testing; `render/skirmish_shell_chrome` owns retail asset loading and atlas entries; `app_skirmish_shell_render` builds draw instances in shell pixel coordinates. No `sim/` changes are part of this plan.

**Design Doc:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`

**Primary Research:** `docs/research/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`

**Supersedes:** `docs/plans/2026-05-17-skirmish-shell-verified-assets-plan.md` where that plan marked `MNSCRNL.SHP` and the Skirmish setup background as unresolved. Live Ghidra now verifies their offline Skirmish parent-background role.

---

## Grounding Summary

- The architecture design remains valid: this is a dedicated Skirmish shell layer, not a generic Win32 dialog host, and it follows the existing sidebar-style split between layout/state, render atlas, and app glue.
- Current git state has already moved past the original design: `src/app.rs` contains a default-off dev gate for the shell, and `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/assets/pcx_file.rs`, and `src/ui/skirmish_shell/` are currently untracked shell scaffolding.
- Live Ghidra confirms the offline Skirmish caller `0x006AE2C0` runs `0x0072CF40` before dialog `0x102`; this actively loads `MnScrnLCoopGameSetup.PAL` and conditionally loads `MnScrnLCoopGameSetup.shp` at screen width `800`.
- Live Ghidra confirms dialog `0x102` parent state `+0x74` is the `MnScrnLCoopGameSetup.PAL` convert object, parent `+0xE0` is `MNSCRNL.SHP`, and parent `+0xE4` is `MnScrnLCoopGameSetup.shp`.
- Live Ghidra confirms parent background selection is width-branchy: `640` selects `MNSCRNL.SHP`; `800` selects `MnScrnLCoopGameSetup.shp`; `>800` still needs screenshot or null-shape behavior verification because the alternate pointer is only loaded at exactly `800`.
- Live Ghidra confirms Skirmish Start/Choose/Back owner-draw buttons use `bue_li30/mi30/ri30.pcx` unpressed, `bde_li30/mi30/ri30.pcx` pressed, tiled middle, direct caps, pressed text/content y-offset `+2`, and disabled alpha dimming rather than `bud_*`.
- Live Ghidra confirms shell text wrapper behavior: color conversion to the active 16-bit display format, clipping to the supplied rect, vertical centering flag `0x04`, and lower bitfont horizontal align flags.
- Live Ghidra confirms offline Skirmish preview order: preview surface first, then `STARTBUT.SHP` frame `0` available-start markers, then numeric labels, all in final child `0x468` shell/backbuffer coordinates.
- Live Ghidra confirms `mmpb.shp` is active YR assigned-player marker code elsewhere, but it is not called by the offline Skirmish dialog `WM_PAINT` path. It must not be used as the offline Skirmish preview backing.
- INI does not drive shell art names or layout. Map INI `[Preview]`, `[PreviewPack]`, and `[Waypoints]` matter for preview/marker data; `rulesmd.ini` `[Countries]`, `[Sides]`, `[Colors]`, and `[MultiplayerDialogSettings]` remain setup-data inputs, not active render-path art selection.

## Key Technical Decisions

- **Keep the shell behind the existing default-off dev gate until visual comparison passes.** The current `RA2_DEV_SKIRMISH_SHELL` gate in `src/app.rs` already matches this policy. **Confidence:** high.
  - **Source:** current repo state; prior verified-assets plan; player-visible parity risk.
- **Promote `MNSCRNL.SHP`, `MnScrnLCoopGameSetup.shp`, and `MnScrnLCoopGameSetup.PAL` from research candidates to verified parent-background inputs.** They are active in offline Skirmish dialog `0x102`. **Confidence:** high.
  - **Source:** live Ghidra `0x0072CF40`, `0x0060CF00`, `0x00621E90`, `Background_Overlay`.
- **Implement only the verified `640` and `800` parent-background branches.** Keep `>800` background blank or debug-only until the high-resolution null/alternate behavior is verified. **Confidence:** medium-high.
  - **Source:** live Ghidra verifies the branch but leaves `DAT_00B0FA18 == 0` behavior open above `800`.
- **Do not copy parent background fields into child layout/control state.** Parent `+0xE0` is a background SHP pointer; child `+0xE0` is a context-dependent right-anchor inset override and remains zero for the researched controls. **Confidence:** high.
  - **Source:** live Ghidra `0x0060CF00`; viewport origin follow-up report.
- **Model owner-draw buttons as cap/middle/cap PCX composition, not a stretched single sprite.** Middle repeats; caps keep native widths; pressed uses `bde_*` and moves content down. **Confidence:** high.
  - **Source:** live Ghidra `0x00612B70`, `0x006BA3E0`.
- **Keep `mmpb.shp` separate from offline Skirmish preview.** It is not a backing image and is not in the confirmed `0x00640710` ordering. **Confidence:** high.
  - **Source:** live Ghidra `0x00640710`, `0x00640A40`, xref at `0x00553687`.

## Open Questions

### Resolved During Planning

- **Should `MNSCRNL.SHP` remain research-only?** No. It is written into dialog `0x102` parent state `+0xE0` and selected by the 640-width parent-background branch.
- **Should `MnScrnLCoopGameSetup.shp/.PAL` be loaded for offline Skirmish?** Yes. `0x0072CF40` is called by the offline Skirmish setup path before dialog creation.
- **Should `mmpb.shp` be drawn in child `0x468`?** No for the confirmed offline Skirmish first-paint path.
- **Is the current dev-gate plan still needed?** Yes. The shell should remain opt-in until the verified active path has visual parity coverage.

### Deferred To Follow-Up Verification

- **High-resolution `>800` background behavior:** live Ghidra shows the branch but not the final player-visible result when the alternate pointer is null. Verify with retail YR screenshot or lower-level `CC_Draw_Shape(NULL, ...)` trace before enabling a default high-res background.
- **Exact right-panel SHP palette:** the live report verifies the parent-background palette object, not a complete independent right-panel palette story. Use the current palette path initially, then compare screenshots.
- **Exact shell font identity and color:** the wrapper behavior is verified, but the report leaves upstream font identity partly open. Use existing `GAME.FNT` plumbing for now and verify visually.
- **Preview image decode and source bounds:** current `MapMenuEntry.preview` records metadata only. Drawing markers faithfully needs the same destination/source projection as the preview surface, including the source origin/extent used by `DrawStartPositions`; full `PreviewPack` image decode may need a separate plan if it is not already available.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/render/skirmish_shell_chrome.rs` | Load and classify verified shell/background/button/marker assets and palettes |
| Modify | `src/app_skirmish_shell_render.rs` | Build draw instances in verified parent-background, button, text, and preview order |
| Modify | `src/ui/skirmish_shell/layout.rs` | Keep final dialog/child coordinate formulas and add any projection helper tests if they stay render-agnostic |
| Modify | `src/ui/skirmish_shell/state.rs` | Add render-agnostic button pressed state and start-marker data selectors if needed |
| Modify | `src/app.rs` | Route mouse-down/up state to shell only when `RA2_DEV_SKIRMISH_SHELL` is enabled |
| Modify | `src/app_init.rs` | Add lightweight map metadata fields only if preview/start markers need them in the menu renderer |
| Modify | `src/app_list_maps.rs` | Populate added `MapMenuEntry` metadata from map INI |
| Modify | `src/map/preview.rs` | Add preview-size/source-bounds helpers only if needed for marker projection tests |
| Modify | `src/map/waypoints.rs` | Reuse existing multiplayer start waypoint parsing; add tests only for UI projection consumers if needed |

## Interface Changes

- `SkirmishShellChromeAtlas` should expose semantically named entries:
  - verified parent backgrounds: `background_640_mnscrnl`, `background_800_coop_game_setup`
  - verified palette source for those backgrounds: `MnScrnLCoopGameSetup.PAL` at load time
  - verified buttons: `button_up_left_30`, `button_up_mid_30`, `button_up_right_30`, `button_down_left_30`, `button_down_mid_30`, `button_down_right_30`
  - verified marker: `start_marker`
  - separate non-offline-Skirmish marker: `assigned_player_marker_mmpb`, not consumed by default `0x468` drawing
- `SkirmishShellState` may gain a render-agnostic pressed-control field for active owner-draw button state.
- `MapMenuEntry` may gain sorted multiplayer start waypoint metadata and preview/source bounds if marker drawing is wired before full map load.
- No public gameplay/sim interfaces change.

## Sim Checklist

This plan does not touch `src/sim/`.

- [x] No fixed-point or floating-point sim math changes.
- [x] No deterministic state hash changes.
- [x] No dependency from `sim/` to UI/render/sidebar/audio/net.
- [x] No tick ordering impact.
- [x] No `EntityStore` iteration impact.

## Risk Areas

- The working tree is dirty and the shell files are untracked. Do not revert unrelated changes; execute only within the files named by this plan.
- `src/app.rs` is already modified and contains the dev gate. Preserve that behavior instead of reintroducing default-visible shell rendering.
- The current atlas uses `sidebar.pal` first. Parent-background SHPs now have a verified palette source, but right-panel palette verification remains weaker. Keep palette choices explicit and easy to audit.
- The current renderer fits `mmpb.shp` or `SDMPBTN.SHP` into `layout.map_preview`. That is now known to be wrong for offline Skirmish.
- Current button rendering uses only the unpressed pieces and stretches cap heights to the control rect. The middle needs repeat/clip behavior, and pressed visual state needs mouse-state plumbing.
- Current text rendering uses the sidebar text helper. This can be a pragmatic first step, but the wrapper must centralize clipping/alignment/pressed-offset choices so later font/color parity fixes do not require broad rewrites.
- Full preview rendering may require more than metadata. Do not fake the preview surface or marker projection just to make markers visible.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Verified asset classification | Prevents generic shell/WOL assets from masquerading as offline Skirmish output | Unit tests and code inspection |
| Task 2 | Parent-background 640/800 branch | Main visible backdrop must match `gamemd.exe` at the resolutions with verified behavior | Dev-shell screenshots at 640x480 and 800x600 |
| Task 3 | Button cap/middle/cap tiling | Start/Choose/Back buttons are large first-viewport pixels; stretching/tile errors are obvious | Unit tests for generated segments; screenshot check |
| Task 4 | Pressed/disabled/text offset behavior | Button feedback must match the original input feel and visual movement | Manual click/hold check in dev shell |
| Task 5 | Text alignment/clipping wrapper | Button labels must center like shell owner-draw text and stay inside the control | Unit position tests and screenshot check |
| Task 6 | Preview draw order | Start markers and numbers must sit over the preview, not behind or in the wrong coordinate space | Code inspection and screenshot once preview surface exists |
| Task 7 | `mmpb.shp` exclusion | Assigned-player markers are not available-start markers and would misrepresent the Skirmish setup screen | Unit test or assertion that `mmpb` is not used by default preview path |

---

## Tasks

### Task 1: Replace Shell Asset Classification With Live-Verified Roles

**Why:** The atlas currently exposes ambiguous `background_large`, `background_small`, and `preview_marker` fields. The live report proves more precise roles.

**Files:**
- Modify: `src/render/skirmish_shell_chrome.rs`

**Pattern:** Existing atlas-building pattern in `skirmish_shell_chrome.rs`; keep the single texture atlas and named entries.

**Steps:**

1. Rename or add fields so parent backgrounds are explicit:
   - `background_640_mnscrnl` for `MNSCRNL.SHP`
   - `background_800_coop_game_setup` for `MnScrnLCoopGameSetup.shp`
2. Add `assigned_player_marker_mmpb` only if keeping `mmpb.shp` loaded for future work. Do not expose it as `preview_marker`.
3. Keep `MNSCRNS.SHP`, `MnScrnLCustomizeBattle.shp`, `dbak6440.pcx`, and `dlgsys*.pcx` out of the default render fields unless they are placed in an explicit `research_candidates` collection.
4. Load all verified button pieces and all verified flag PCXs that prior reports mapped:
   - `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`
   - `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`
   - `usai.pcx`, `japi.pcx`, `frai.pcx`, `geri.pcx`, `gbri.pcx`, `djbi.pcx`, `arbi.pcx`, `lati.pcx`, `rusi.pcx`, `yrii.pcx`, `obsi.pcx`, `rani.pcx`
5. Add classification tests that assert:
   - `MNSCRNL.SHP` is a verified parent background.
   - `MnScrnLCoopGameSetup.shp` is a verified parent background.
   - `STARTBUT.SHP` is a verified offline Skirmish marker.
   - `mmpb.shp` is not a default offline Skirmish preview asset.
   - `MnScrnLCustomizeBattle.shp` is not default offline Skirmish art.

**Verify:**
- Run `cargo test skirmish_shell -- --nocapture`.
- Run `cargo check`.

### Task 2: Load Parent Backgrounds With The Verified Palette Source

**Why:** `0x0072CF40` loads `MnScrnLCoopGameSetup.PAL`, and `0x0060CF00` stores that convert object in parent `+0x74` before the background overlay.

**Files:**
- Modify: `src/render/skirmish_shell_chrome.rs`

**Pattern:** Existing SHP decode through `ShpFile::frame_to_rgba`, but choose palette by verified role instead of a generic first fallback.

**Steps:**

1. Add a helper for the parent-background palette that loads `MnScrnLCoopGameSetup.PAL`.
2. Render `MNSCRNL.SHP` and `MnScrnLCoopGameSetup.shp` using that parent-background palette.
3. Keep PCX conversion using each PCX embedded palette through `PcxFile::to_rgba`; do not pass any external palette to PCX assets.
4. Keep right-panel SHP palette selection in a clearly separate helper. Name it as provisional if it still uses `sidebar.pal`, `SHELL.PAL`, or `DIALOG.PAL`, because the live report did not fully settle right-panel palette identity.
5. If `MnScrnLCoopGameSetup.PAL` is missing, skip only the verified parent-background entries and log a specific warning. Do not fall back to `sidebar.pal` for these parent backgrounds.

**Verify:**
- Run `cargo check`.
- Run ignored/manual retail asset dimension test if available and confirm `MnScrnLCoopGameSetup.shp` and `MNSCRNL.SHP` decode without parser errors.

### Task 3: Implement Verified Parent Background Branching

**Why:** The current renderer chooses `background_small` for `<=640` and `background_large` otherwise. Live Ghidra gives exact branch semantics for the verified paths.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`

**Pattern:** Use a small pure selector before emitting `SpriteInstance`s; keep render-specific drawing in app layer.

**Steps:**

1. Replace the current `layout.screen.w <= 640` background selection with a helper that returns:
   - `MNSCRNL.SHP` when `screen_w == 640`
   - `MnScrnLCoopGameSetup.shp` when `screen_w == 800`
   - no default background when `screen_w > 800`
   - no default background for other widths unless later evidence covers them
2. Draw the selected parent background at shell origin `(0,0)` in native asset size, matching the parent background path instead of fitting/stretching to the screen.
3. Keep the right-panel draw after parent background and before child controls.
4. Add tests for the selector:
   - `640 -> background_640_mnscrnl`
   - `800 -> background_800_coop_game_setup`
   - `1024 -> none`
5. Add a log when a high-resolution dev shell frame intentionally has no verified parent background. Log once per process or behind existing debug logging to avoid spam.

**Verify:**
- Run `cargo test skirmish_shell -- --nocapture`.
- Run `cargo check`.

### Task 4: Match Owner-Draw Button Cap/Middle/Cap Composition

**Why:** The current `push_button_30` uses only unpressed art and does not model pressed state. The original callback selects unpressed/pressed PCXs, repeats the middle, clips the final middle segment, and keeps caps direct.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/ui/skirmish_shell/state.rs` if pressed state belongs there
- Modify: `src/app.rs` for mouse-down/up wiring if needed

**Pattern:** Keep composition as pure render-instance building; keep input state render-agnostic.

**Steps:**

1. Add a small render/input enum for the three owner-draw buttons:
   - Start `0x617`
   - Choose Map `0x5AA`
   - Back `0x5C0`
2. Track which button is currently pressed while the left mouse button is down in dev shell mode.
3. Update mouse handling so press state is set on mouse down and cleared on mouse up. Trigger the existing action on mouse up if the pointer is still inside the same control.
4. Change button drawing to choose `bue_*30` for unpressed and `bde_*30` for pressed.
5. Preserve the native left/right cap widths.
6. Repeat the middle entry across the remaining width.
7. Clip the final middle segment by shortening UV width and destination width.
8. Do not use `bud_*` for disabled Skirmish buttons. If disabled state is introduced, draw unpressed art and apply alpha `0x80 / 255.0`.
9. Add pure tests for segment generation:
   - start button width `162` emits left cap, one or more middle segments, and right cap.
   - final middle segment clips when remaining width is not a multiple of the middle PCX width.
   - pressed state selects the `bde_*30` entries.

**Verify:**
- Run `cargo test skirmish_shell -- --nocapture`.
- Run `cargo check`.
- Manual dev-shell check: hold Start/Choose/Back and confirm the skin changes while held.

### Task 5: Centralize Shell Text Placement And Pressed Offset

**Why:** `0x00621040` centers vertically under flag `0x04`, clips to the supplied rect, supports horizontal alignment flags, and the button callback shifts pressed content/text by `+2` pixels.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`

**Pattern:** Wrap existing `SidebarTextRenderer` calls behind a shell-specific helper so later font/color parity fixes are localized.

**Steps:**

1. Replace direct `push_centered_text` usage with a shell text helper that accepts:
   - destination rect
   - horizontal alignment: left, center, right
   - vertical center enabled/disabled
   - y offset
   - color role
2. For Start/Choose/Back labels, use center alignment, vertical centering, and y offset `0` unpressed or `+2` pressed.
3. Keep clipping policy explicit. If `SidebarTextRenderer` cannot clip glyphs to a rect today, note that as a verification risk in a short comment and prevent text from drawing outside known button rects by computing start coordinates conservatively.
4. Define a shell button text color constant from the live recovered value as a named research-derived constant, but keep a comment that GPU RGB matching must be visually verified because `0x00621040` converts to 16-bit display format.
5. Add tests for text position math:
   - a centered label starts at `(rect.w - text_w) / 2`.
   - vertical centering uses `(rect.h - glyph_h) / 2`.
   - pressed state adds exactly `2` to y.

**Verify:**
- Run `cargo test skirmish_shell -- --nocapture`.
- Run `cargo check`.
- Manual screenshot check for button label centering at 800x600.

### Task 6: Remove Wrong Preview Backing And Separate Preview Preconditions

**Why:** The current renderer fits `mmpb.shp` or `SDMPBTN.SHP` into `layout.map_preview`, but live Ghidra proves neither is the offline Skirmish preview backing.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/render/skirmish_shell_chrome.rs`

**Pattern:** Draw nothing for unknown content rather than substituting plausible shell art.

**Steps:**

1. Delete the default preview backing selection that uses `atlas.preview_marker.or(atlas.sd_map_button)`.
2. Ensure `SDMPBTN.SHP` remains only right-panel/map-button chrome if it is still used there.
3. Ensure `mmpb.shp` is not accessible through any default preview field name.
4. Add a focused test or assertion around default preview roles:
   - `STARTBUT.SHP` is allowed as an available-start marker.
   - `mmpb.shp` is not allowed as a default preview backing.
   - `SDMPBTN.SHP` is not allowed as a preview backing.
5. If a dev debug rectangle is still needed for child `0x468`, put it behind a separate debug flag and make it visually plain. Do not use retail art for the placeholder.

**Verify:**
- Run `cargo test skirmish_shell -- --nocapture`.
- Run `cargo check`.

### Task 7: Add Start-Marker Metadata Without Faking Preview Image Parity

**Why:** Live Ghidra verifies marker order and final coordinate space, but current menu metadata does not yet expose enough source-bounds data to project starts the way `DrawStartPositions` does.

**Files:**
- Modify: `src/app_init.rs`
- Modify: `src/app_list_maps.rs`
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/map/preview.rs` only if a projection helper needs preview size normalization
- Reuse: `src/map/waypoints.rs`

**Pattern:** Keep map metadata lightweight for the menu; do not load the full scenario into `sim/`.

**Steps:**

1. Extend `MapMenuEntry` with sorted multiplayer start waypoints from `[Waypoints]` indices `0..=7`.
2. Populate the new waypoint field in `read_map_menu_entry` using the existing `map::waypoints::parse_waypoints` and `multiplayer_start_waypoints` helpers.
3. Add a separate optional metadata field for preview/source bounds. This must carry the equivalent of the binary projection inputs: source origin X/Y and source width/height, corresponding to `ScenarioClass+0x112C`, `+0x1130`, `+0x1134`, and `+0x1138`.
4. Populate source bounds only from a verified map INI source. Candidate source is `[Map] LocalSize=left,top,width,height`, but do not assume it is correct until checked against a retail map and the live `DrawStartPositions` formula.
5. Keep fallback entries empty when the map INI cannot be read or when source bounds cannot be verified.
6. Add tests in `app_list_maps.rs` or a nearby module that build a small INI with `[Waypoints]` and confirm the menu entry exposes starts sorted by index.
7. Add tests for source-bounds parsing only after the `[Map] LocalSize` mapping is verified. The test must assert all four values: origin X, origin Y, width, and height.
8. Do not add a render projection helper yet unless both prerequisites are available: real preview surface dimensions and verified source bounds. Without those, Task 8 must continue to skip marker drawing.
9. This preserves the original order: preview first, markers second, numbers third, without inventing marker positions from incomplete data.

**Verify:**
- Run `cargo test waypoints -- --nocapture`.
- Run `cargo test skirmish_shell -- --nocapture`.
- Run `cargo check`.

### Task 8: Draw `STARTBUT.SHP` Markers Only After A Preview Surface Exists

**Why:** The original order is preview surface first, then markers, then numeric labels. Drawing markers over a blank/debug preview would create a misleading parity screenshot.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Optional modify: preview image decode files if a verified decoder already exists

**Pattern:** Keep marker drawing as a separate layer after preview drawing.

**Steps:**

1. Add a local marker-building function that receives projected positions from Task 7 and emits `STARTBUT.SHP` frame `0`.
2. Offset each marker by `x - 9`, `y - 6` as verified by live Ghidra.
3. Emit numeric label `i + 1` after each marker at a higher draw depth than the marker.
4. Clip marker and label drawing to the final `layout.map_preview` rect as closely as the current sprite renderer allows.
5. Gate this function behind the presence of a real preview surface. If no preview surface is available, skip markers and log once in dev shell mode that marker draw is waiting on preview decode.
6. Do not draw `mmpb.shp` here.

**Verify:**
- Run `cargo check`.
- Once preview decode exists, compare 800x600 marker positions against retail YR.

### Task 9: Keep High-Resolution Background Explicitly Unresolved

**Why:** The live report leaves `>800` parent-background behavior open. The implementation must not guess a backdrop for 1024x768 and later treat that as verified.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/render/skirmish_shell_chrome.rs` only if debug fields are needed

**Pattern:** Evidence-gated rendering.

**Steps:**

1. Ensure the background selector from Task 3 returns no background for `screen_w > 800`.
2. Keep right-panel rendering active at 1024x768 because the layout/right-panel path is separately verified.
3. Add a comment next to the high-resolution branch explaining that `0x0072CF40` only loads `DAT_00B0FA18` at exactly `800`, while `Background_Overlay` selects the alternate path for non-640 widths.
4. Add a manual verification note to the dev-shell log when running at `1024x768` or higher.
5. Do not add any automatic substitution from `MNSCRNL.SHP`, `MNSCRNS.SHP`, or `MnScrnLCustomizeBattle.shp` for high-resolution backgrounds.

**Verify:**
- Run `cargo test skirmish_shell -- --nocapture`.
- Run `cargo check`.
- Manual dev-shell check at 1024x768: right panel and controls render, parent background remains intentionally absent unless a later verified path is implemented.

### Task 10: Visual Smoke Verification

**Why:** This feature is pixel-visible. Compilation is not enough.

**Files:**
- No source edits unless verification finds a bug.

**Steps:**

1. Run the app without `RA2_DEV_SKIRMISH_SHELL`. Expected: normal egui Skirmish setup remains visible.
2. Run with `RA2_DEV_SKIRMISH_SHELL=1` at 800x600. Expected: verified parent background uses `MnScrnLCoopGameSetup.shp`, right panel appears, Start/Choose/Back buttons use `bue_*30`, pressed state uses `bde_*30`.
3. Run with `RA2_DEV_SKIRMISH_SHELL=1` at 640x480. Expected: verified parent background uses `MNSCRNL.SHP`, right panel uses the 640 tile-count/layout path.
4. Run with `RA2_DEV_SKIRMISH_SHELL=1` at 1024x768. Expected: right panel/control layout matches existing tests; parent background remains intentionally unresolved.
5. Click and hold Start/Choose/Back. Expected: pressed art appears while held and text moves down by 2 pixels.
6. Confirm `mmpb.shp` is not drawn in child `0x468`.
7. Record any visual mismatch as a follow-up research or implementation bug, not as a silent compromise.

**Verify Commands:**
- `cargo test skirmish_shell -- --nocapture`
- `cargo test waypoints -- --nocapture`
- `cargo check`
- `cargo run --bin vera20k` with and without `RA2_DEV_SKIRMISH_SHELL=1`

## Sources & References

- **Design doc:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`
- **Prior implementation plan:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-plan.md`
- **Superseded verified-assets plan:** `docs/plans/2026-05-17-skirmish-shell-verified-assets-plan.md`
- **Live Ghidra report:** `docs/research/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`
- **Prior Ghidra reports:**
  - `docs/research/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- **Live addresses:** `0x0072CF40`, `0x00622B50`, `0x0060CF00`, `0x00612B70`, `0x00621040`, `0x00640710`, `0x00640A40`, `0x006AE2C0`, `0x006AE3F0`.
- **Related code:**
  - `src/app.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/render/skirmish_shell_chrome.rs`
  - `src/assets/pcx_file.rs`
  - `src/ui/skirmish_shell/layout.rs`
  - `src/ui/skirmish_shell/state.rs`
  - `src/app_init.rs`
  - `src/app_list_maps.rs`
  - `src/map/preview.rs`
  - `src/map/waypoints.rs`
- **INI references:**
  - Map INI `[Preview]`
  - Map INI `[PreviewPack]`
  - Map INI `[Waypoints]`
  - `ini/rulesmd.ini [Countries]`
  - `ini/rulesmd.ini [Sides]`
  - `ini/rulesmd.ini [Colors]`
  - `ini/rulesmd.ini [MultiplayerDialogSettings]`

## Post-Plan Self-Review

- Spec coverage: covers the live report's resolved background, button, text, preview-order, and `mmpb` findings.
- Placeholder scan: no task relies on an unspecified "implement later" step; unresolved items are explicitly deferred.
- Architecture check: changes stay in UI/render/app/map metadata layers; `sim/` is not touched.
- Interface ordering: atlas roles and palette selection land before render behavior consumes them.
- Risk coverage: dirty worktree, high-res background uncertainty, palette uncertainty, preview decode absence, and wrong preview backing are explicit.
- Self-containment: each task names files, exact behavior, and verification commands.
- Sim compliance: no `sim/` task exists.
- Grounding coverage: cites research docs, live Ghidra, current repo files, and relevant map/rules INI sections.
- Confidence tagging: key decisions include confidence and sources.
- Deferred questions: high-res background, right-panel palette, font identity/color, and preview image decode are separated from implementation tasks.
- Parity-critical items: parent background, buttons, text, marker ordering, and `mmpb` exclusion are listed before tasks.
