# Slice 5a-ii Grounding — Lane C: Rust paint-seam map for the 0xBBB owner-draw render

Read-only mapping. Anchors on CURRENT code (the 2026-06-02 slice5b plan is SUPERSEDED naming;
its structure is reusable but the file/seam landscape below is authoritative). All paths absolute.

---

## (1) The ControlPaint seam — `app_skirmish_shell_render/controls.rs`

### Where the per-control-kind dispatch lives

- `enum ControlPaint` — `src/app_skirmish_shell_render/controls.rs:242-256`. Four variants today:
  - `Checkbox { checked: bool, rect: RectPx }`
  - `Trackbar { rect: RectPx, thumb_px: i32 }`
  - `Combo { rect, swatch: Option<[f32;3]>, open: bool, disabled: bool }`
  - `ScrollBar { scrollbar: RectPx, thumb: RectPx, pressed_part: Option<DropdownScrollbarPart> }`
  - Visibility: `pub(super)`. It is an app-layer data enum (NOT a trait) by design (comment at
    `controls.rs:232-241`): the emitters + skirmish layout live in the app layer, and `render/`
    must not depend on the app layer, so the seam can't sit in `render/shell_paint.rs`.

- `fn paint_control(out: &mut Vec<SpriteInstance>, chrome: &ControlChrome, paint: ControlPaint)`
  — `src/app_skirmish_shell_render/controls.rs:263-383`. `pub(super)`. Each arm resolves its glyphs
  from the `ControlChrome` subset itself and pushes `SpriteInstance`s into `out`.
  - Checkbox arm: `269-278` — one `push_entry` at `checkbox_icon_rect(rect)`, `SHELL_CONTROL_DEPTH`.
  - Trackbar arm: `279-290` — rail (`push_entry_native`) → `paint_trackbar_plaque` (mid/left/right) →
    thumb (`push_entry` at `trackbar_thumb_rect(rect, thumb_px)`, depth `SHELL_CONTROL_DEPTH-0.00002`).
  - Combo arm: `291-329` — face glyph → swatch (`push_tinted_entry`, color faces only) → arrow variant.
  - ScrollBar arm: `330-381` — track fill (`push_solid_rect_px`) → up/down arrows → thumb (`push_scrollbar_thumb`)
    → 2-ring bevel (`push_ownerdraw_two_pixel_bevel_frame_px`), all at hardcoded `SHELL_DROPDOWN_DEPTH` offsets.

### How each arm emits sprite instances

Arms call the low-level helpers in `chrome.rs` (all `pub(super)`):
- `push_entry(out, entry, rect, depth)` — `chrome.rs:109-125`. Scales `entry` to `rect`.
- `push_entry_native(out, entry, x, y, depth)` — `chrome.rs:173-181`. Native `pixel_size` at `(x,y)`.
- `push_tinted_entry(out, entry, rect, tint, depth)` — `chrome.rs:380-397`.
- `push_solid_rect_px(out, white_pixel, rect, tint, depth)` — `chrome.rs:399-410`.
- `push_ownerdraw_two_pixel_bevel_frame_px(out, white_pixel, rect, depth)` — `chrome.rs:525-549`.
- Helper `paint_trackbar_plaque` — `controls.rs:199-222`; `push_scrollbar_thumb` — `controls.rs:137-164`.

`SpriteInstance` is `crate::render::batch::SpriteInstance` (fields position/size/uv_origin/uv_size/depth/tint/alpha;
constructed with `..Default::default()`).

### Per-family wrappers that BUILD `ControlPaint` and call `paint_control`

These read the skirmish layout + state, then push one `ControlPaint` per control:
- `push_checkbox_instances` — `controls.rs:385-407` (iterates `layout.checkboxes`).
- `push_trackbar_instances` — `controls.rs:409-431` (value→pixel quantization stays here; only `thumb_px`+rect cross).
- `push_combo_instances` — `controls.rs:433-549`.
- `push_dropdown_instances` — `controls.rs:551-626` (open popup; calls the `ScrollBar` arm at `614-622`).

All four are invoked from `build_skirmish_shell_instances` (`app_skirmish_shell_render.rs:340-369`):
```
push_combo_instances(...);   // 340
push_checkbox_instances(...);// 341
push_trackbar_instances(...);// 342
...
push_dropdown_instances(...);// 369
```

### Where a new `ControlPaint::Button` arm goes + inputs it needs

- Add the variant to `enum ControlPaint` (`controls.rs:242-256`). It must carry the RESOLVED per-control
  STATE + rect (mirroring the existing arms): at minimum
  `Button { rect: RectPx, pressed: bool, hovered: bool, enabled: bool, frame: <state→frame selector> }`.
- Add the match arm in `paint_control` (`controls.rs:268`). For 0xBBB the art is **SIDEBTTN.SHP** type-2
  (frame 0 released / 1 pressed / 2 timer-highlight), centered/anchored on its rect. The arm reads the
  SIDEBTTN entries from `ControlChrome` (so those fields must be added to `ControlChrome` first — see (2)).
  Glyph push: native-size centered via a `chrome.rs`-style helper (compare `shell_paint.rs`
  `modal_button_centered_position` at `shell_paint.rs:480-487`).
- The state→frame mapping should live as a small pure helper (like `right_panel_button_sdbtnanm_frame_index`
  at `chrome.rs:342-347`, or `shell_paint.rs::modal_button_frame_index` at `shell_paint.rs:373-381`) so it
  is unit-testable without an atlas. Caller threads `pressed/hovered/enabled` from the descriptor default
  (`ControlDescriptor.enabled`) layered with `shell_controller` runtime state — the seam never re-derives
  hit-testing (same contract as `shell_paint.rs::PaintButton`, `shell_paint.rs:85-93`).
- NOTE the 5a-ii plan's note: active 0xBBB uses **SIDEBTTN** (NOT SDBTNANM/MNBTTN/PCX); shell 0xF5 Back
  uses SDBTNANM frame 2/4/3. Do not reuse `push_right_panel_button_shp` (`chrome.rs:321-340`) which is
  SDBTNANM — that is the 0x102/0xF5 path.

---

## (2) ControlChrome + atlas — `src/render/skirmish_shell_chrome.rs`

### The `ControlChrome` struct (the Default-able, texture-free seam input)

- `struct ControlChrome` — `src/render/skirmish_shell_chrome.rs:96-121`. `#[derive(Debug, Clone, Copy, Default)]`.
  Carries ONLY owner-draw glyph `Option<SkirmishShellChromeEntry>` fields (no GPU `texture`). Current fields:
  checkbox cue_i/cce_i, trackbar rail/trofl/trofm/trofr/trakgrip, white_pixel, combo_face_150/117/44/38,
  combo arrow released/pressed/gray pairs, scrollbar arrow up/down released/pressed, scrollbar thumb top/mid/bottom.
- `impl SkirmishShellChromeAtlas { fn control_chrome(&self) -> ControlChrome }` — `skirmish_shell_chrome.rs:123-153`.
  Snapshots the live atlas's glyph entries into the Default-able subset. Callers do `atlas.control_chrome()`
  (e.g. `controls.rs:396, 419, 447, 613`).

### How glyphs are loaded + stored + frame dims (the full atlas)

- `struct SkirmishShellChromeAtlas` — `skirmish_shell_chrome.rs:33-88`. The live atlas: one `texture: BatchTexture`
  plus `Option<SkirmishShellChromeEntry>` per glyph. `SkirmishShellChromeEntry` (`:26-31`) = `uv_origin[2]`,
  `uv_size[2]`, `pixel_size[2]` (frame dims in px, set from SHP canvas / PCX size).
- Build fn: `fn build_skirmish_shell_chrome_atlas(gpu, batch, assets) -> Option<SkirmishShellChromeAtlas>`
  — `skirmish_shell_chrome.rs:177-469`. Pipeline:
  1. Load palettes via `load_named_palette` (`:505-516`) / `load_shell_palette` (`:501-503`).
  2. Render each asset to a `RenderedShellEntry { label, width, height, rgba }`:
     - SHP: `render_shp_entry` (`:570-583`) / `render_shp_entry_labeled` (`:585-625`) / `mandatory_shp` (`:555-568`).
     - PCX: `render_pcx_entry` (`:627-636`); flags use `render_flag_pcx_entry` (`:638-647`, magenta color-key).
     - Synthesized primitives: `render_primitive_bevel_entry` (`:663-747`), `render_solid_entry` (`:649-660`,
       the white_pixel).
  3. PCX owner-draw glyphs loaded in the loop at `skirmish_shell_chrome.rs:337-361` (`cue_i`, `cce_i`,
     `trakgrip`, `trofl/m/r`, `dnarrow*`, `gdnarrow*`, `uparrow*`, `sbgrip*`).
  4. `pack_entries` (`:814-884`) packs all `rendered` into one atlas texture; returns per-label
     `SkirmishShellChromeEntry`s keyed by lowercased label in a `HashMap` (`by_label`, `:396-400`).
  5. The struct is built from `by_label.get("...").copied()` at `:411-468`.

### Confirm SDBTNANM already loaded — YES

- SDBTNANM.SHP frames 0..=16 are baked: loop at `skirmish_shell_chrome.rs:219-236` (palette `SDBTNANM.PAL`),
  stored as `right_panel_button_sdbtnanm_frame2/3/4` (`:417-419`), the full `[Option; 17]` array
  `right_panel_button_sdbtnanm_frames` (`:420-422`), and overlay frame10 (`:416`). So 0xF5 Back (SDBTNANM
  2/4/3) needs NO new asset load — only a paint arm.

### How to add SIDEBTTN.SHP + SIDEBAR.PAL (NOT loaded today)

Grep confirms SIDEBTTN/SIDEBAR.PAL appear ONLY in comments (`layout.rs:41`, `descriptor.rs:86`) — never loaded.
To add, inside `build_skirmish_shell_chrome_atlas`:
1. Load the palette: `let sidebar_palette = load_named_palette(assets, "SIDEBAR.PAL");` (returns `Option<Palette>`,
   pattern at `:182-186`). Guard with a `log::warn!` on `None` like the SDBTNANM block (`:237-239`).
2. Bake frames 0..=2 with `render_shp_entry_labeled(assets, "SIDEBTTN.SHP", &format!("sidebttn.shp#{frame}"),
   &sidebar_palette, frame)` in a `for frame in [0,1,2]` loop (mirror the MNBTTN block at `:240-259` or the
   SDBTNANM block). `push`/`push_optional` each into `rendered`.
3. Add three `Option<SkirmishShellChromeEntry>` fields to `SkirmishShellChromeAtlas` (`:33-88`), e.g.
   `options_button_sidebttn_frame0/1/2`, and populate from `by_label.get("sidebttn.shp#0"...)` at `:411-468`.
4. Add the SAME three fields to `ControlChrome` (`:96-121`) AND copy them in `control_chrome()` (`:123-153`)
   so the `Button` arm in `paint_control` can read them.
5. (Optional) extend the `#[cfg(test)] classify_shell_asset` map (`:518-541`) — add a `sidebttn.shp` arm
   (currently the `sidebar.pal` test at `:987-990` only asserts it is NOT an owner-draw button).

### `src/render/shell_paint.rs` — the OTHER (descriptor-driven) emitter

- `shell_paint.rs` is the `render/`-layer emitter for the FRONT-END right-panel shells (0xE2/0x100) and the
  mode-2 SHP modals — NOT the skirmish/0xBBB owner-draw control seam. It already has a clean `PaintButton`
  (`shell_paint.rs:85-93`) + `paint_buttons` (`:272-324`) and the mode-2 modal family
  `paint_modal_shp`/`paint_modal_sprites`/`ModalButtonFrames` (`:435-487`). Reuse it ONLY for shared
  primitives (the 5a-ii plan §3 explicitly says "Do NOT use `paint_modal_shp`/`MNBTTN` for Options").
  Its `BUTTON_DEPTH`/`TEXT_DEPTH`/`CHROME_DEPTH` constants (`:36-43`) are a good depth-ordering reference but
  the 0xBBB overlay must define its OWN front-most depths (overlay over the frozen battlefield, not the shell stack).

---

## (3) End-to-end: DialogDescriptor + layout → draw output

### Today (skirmish 0x102) — the live walk-and-emit

1. `build_skirmish_shell_instances(atlas, font, layout, ...) -> Vec<SpriteInstance>`
   — `app_skirmish_shell_render.rs:187-381`. This is the emitter that walks all controls and calls the
   per-family `push_*_instances` wrappers (which call `paint_control`). NOTE: skirmish uses its OWN
   `SkirmishShellLayout` (`compute_layout`, `:468`), NOT the descriptor/`layout_pass` path — the
   descriptor pipeline (`ui::shell::layout::layout_pass`) is currently consumed only by 0xE2/0x100 via
   `shell_paint.rs`. For 0xBBB you wire the `build_in_game_options_descriptor()` → `layout_pass` →
   `LaidOutControl` rects into a NEW emitter (see below).
2. The render pass: `render_skirmish_shell_with_atlas` — `app_skirmish_shell_render.rs:461-721`. It calls
   `build_skirmish_shell_instances` (`:541-553`), creates the instance buffer (`:603-609`), opens ONE
   `begin_render_pass` (`:633-654`), and submits via `state.batch_renderer.draw_with_buffer_passthrough`
   (`:655-657`, the passthrough = no Z-test sprite draw). Text draws are scissored buffers submitted after
   (`:693-709`); cursor last (`:711-717`).

### How layout_pass output is shaped

- `layout_pass(desc, screen_w, screen_h) -> Vec<LaidOutControl>` — `src/ui/shell/layout.rs:24-50`.
  For `RepositionPolicy::InGameOptions` (`:43-45`) it returns the **raw DLU→pixel** rect per control
  (5a-i baseline; 5a-ii will replace this branch with the native child-resize anchoring — centered offsets
  + SIDEBTTN right-edge button anchoring). `LaidOutControl { id, rect }` (`:16-20`).
- DLU→px helper: `geom::dlu_rect` (the `x*6/4`, `y*13/8` round-half-up factor; tested at
  `in_game_options.rs:264-269` and `layout.rs:238-243`).
- Descriptor source: `build_in_game_options_descriptor()` — `src/ui/shell/in_game_options.rs:67-131`
  (9 controls; `BgKind::InGameOptions`; `RepositionPolicy::InGameOptions`).

### Where `app_skirmish_shell_render/in_game_options.rs` should live + hook in

- CREATE `src/app_skirmish_shell_render/in_game_options.rs` and declare it in the module header
  `src/app_skirmish_shell_render.rs:6-12` (`mod chrome; mod controls; ... mod in_game_options;`).
  App-layer is correct: it needs both the `ControlChrome` glyphs AND the `ui::shell` descriptor/layout,
  and `render/` may not depend on the app layer (same rationale as the `ControlPaint` seam comment,
  `controls.rs:232-241`). It should re-use `controls::paint_control` (make the needed items `pub(super)`/
  `pub(crate)` as required) and the new `Button` arm.
- It builds: `build_in_game_options_descriptor()` → `layout_pass(&desc, w, h)` → for each `LaidOutControl`
  match `desc.controls[i].kind` → emit the matching `ControlPaint` (Button/Trackbar/Checkbox) into a
  `Vec<SpriteInstance>` over the frozen battlefield frame. Hidden/disabled handling per the descriptor
  `enabled` flag (VisualDetails ships `enabled:false`, `in_game_options.rs:102-107`).
- HOOK the render: the in-game frame composes in `app.rs` `GameScreen::InGame` branch (the egui pause
  card draws at `app.rs:2914-2922`). The Options overlay render must run on `encoder`/`view` BEFORE
  `state.egui.end_frame_and_render` (`app.rs:2923-2929`), gated on the new "options dialog open" state
  (which replaces `state.paused`'s egui card — see (4)). Mirror the `render_skirmish_shell` entry shape
  (`app_skirmish_shell_render.rs:407-422`): take `state`, `encoder`, `&TextureView`, open a render pass that
  LOADS (not clears) the battlefield color, draws the instance buffer passthrough, then text/cursor.

---

## (4) The egui pause card to replace

- `fn handle_pause_menu(state)` — `src/app.rs:3001-3057`. Calls `pause_menu::draw_pause_menu(&state.egui.ctx, &info)`
  (`app.rs:3011`) and matches `PauseMenuAction` (`crate::ui::pause_menu::PauseMenuAction`): `Resume`,
  `ReturnToMenu`, `NextTrack`, `SetMusicVolume`, `SetGameSpeed`, `None` (`app.rs:3013-3056`).
  Resume (`:3014-3024`) flips `state.paused=false`, resets `last_update_time` + `sim_accumulator_ms=0`,
  re-hides OS cursor.
- Invoked at `app.rs:2914-2922` inside the `GameScreen::InGame` egui block (only `if state.paused`).
- The egui pause module is `src/ui/pause_menu.rs` (`PauseMenuAction`, `PauseMenuInfo`, `draw_pause_menu`).
- ESC trigger — `src/app_input.rs:425-447`:
  - `state.paused` true → unpause (resets `last_update_time`, `sim_accumulator_ms=0`, re-hide cursor) (`:426-435`).
  - else if `targeting_mode` → cancel targeting (`:436-438`).
  - else → `state.paused = true` + show OS cursor (`:439-446`).
  - For 0xBBB parity (5a-ii plan §4): ESC must open the Options dialog (route through dialog handling
    BEFORE this global ESC/pause toggle) and global ESC must not close/unpause the old egui route while
    Options is active. There is ALSO a debug-pause toggle `toggle_pause` at `app_input.rs:405-415`.
- Paused freeze gate — `advance_in_game_runtime` — `src/app_sim_tick.rs:203-211`:
  `run_sim = if frame_stepping { true } else { !state.paused }` (`:206-211`). So while `state.paused`,
  `advance_fixed_simulation` (`:222`) is skipped → world tick frozen. The modal-pump decision helper
  `modal_pump_should_advance_sim(mode, reentrancy)` exists at `app_sim_tick.rs:199-201` (network advances,
  offline freezes) but its live `service_tick` wrapper is NOT wired yet (comment `:195-198`). For offline
  0xBBB the existing `!state.paused` freeze is the correct behavior; the Options state should gate the
  same freeze (either keep `state.paused` set while Options is open, or add a parallel `options_open`
  flag the freeze also honors). The egui card draw must be suppressed while the native Options overlay is up.

---

## (5) Draw-list / byte-identical-emission test idiom

The established idiom: build a hand-rolled `ControlChrome` (or entry) with sentinel uv/pixel values,
call the emitter into a `Vec<SpriteInstance>`, then assert `out.len()` and each instance's
`position`/`size`/`uv_origin`/`depth` against the layout helpers (NOT hardcoded geometry), plus an
empty-chrome → empty-output case.

- PRIMARY example file: `src/app_skirmish_shell_render/controls.rs` (the `#[cfg(test)] mod tests`, `:648-1191`).
  - `fn checkbox_paint_seam_emits_icon_at_icon_rect_with_control_depth` — `controls.rs:717-762`.
    Builds `ControlChrome { checkbox_checked_cce_i: Some(entry), ..Default::default() }`, calls
    `paint_control`, asserts `out.len()==1` + position/size/uv/depth, then empty-chrome → `assert!(empty.is_empty())`.
  - Also `trackbar_paint_seam_emits_rail_plaque_thumb_in_native_order` (`:765-856`),
    `combo_face_paint_seam_emits_face_swatch_arrow` (`:858-1008`),
    `scrollbar_paint_seam_emits_track_arrows_thumb_bevel` (`:1011-1172`).
- SECONDARY (modal/button shape, useful for the `Button` arm): `src/render/shell_paint.rs` tests
  `fn modal_sprites_use_pressed_frame_centered_on_control` — `shell_paint.rs:881-918` (asserts frame
  selection by uv + centered position + depth), and `paint_buttons` Native/Fit tests (`:544-634`).

A new `Button` arm should get a sibling test in `controls.rs` (or in the new `in_game_options.rs` test mod)
named e.g. `options_button_paint_seam_emits_sidebttn_frame_centered`, asserting SIDEBTTN frame 0/1/2 select
by uv + centered/anchored position + the overlay depth, plus empty-chrome → empty.

---

## Cross-cutting notes / gotchas

- The skirmish renderer does NOT use the `ui::shell` descriptor pipeline; only 0xE2/0x100 (via `shell_paint.rs`)
  and the new 0xBBB path do. So 0xBBB is the first consumer of `layout_pass` for `RepositionPolicy::InGameOptions`.
- 0xBBB trackbar value→thumb_px quantization (GameSpeed/ScrollRate `6-pos` inversion, range 0..6; VisualDetails
  range 0..2 direct, DISABLED) must be computed in the new emitter (analogous to `push_trackbar_instances`
  reading `shell.trackbar_bounds`, `controls.rs:425-429`). The skirmish trackbar IDs differ (GameSpeed/Credits/
  UnitCount) so 0xBBB needs its own value source, not `SkirmishShellState`.
- `right_panel_button_sdbtnanm_frame_index(pressed, disabled)` (`chrome.rs:342-347`) and
  `shell_paint.rs::modal_button_frame_index` (`:373-381`) are the two existing pure frame-select helpers to
  pattern the SIDEBTTN type-2 selector after.
