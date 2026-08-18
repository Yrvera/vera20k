# Skirmish Checkbox / Trackbar Owner-Draw Visuals 800x600 Trace

**Scenario:** Standard offline Yuri's Revenge Skirmish setup dialog `0x102` at 800x600, five option checkboxes and three trackbars in enabled idle/checked/value states.

**Verdict:** PARTIAL. Current Rust now implements the checkbox and trackbar owner-draw surfaces and much of the input/value behavior, but it is not structurally pixel-equal yet. The main visible mismatches are first-four checkbox x placement, unit-count trackbar y placement, trackbar value-plaque x/y placement, and shell text source colors.

**Tally:** PASS: 5 | FAIL: 5 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Sources

- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_OWNERDRAW_VARIANT_WRITERS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md`
- `docs/research/traces/SKIRMISH_CREDITS_TRACKBAR_CLICK_DRAG_TRACE.md`
- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app_skirmish_shell_render.rs`
- `src/render/skirmish_shell_chrome.rs`
- `src/sim/game_options.rs`

## Active YR Confirmation

The scoped gamemd references are active in standard YR offline Skirmish, not dormant TS legacy. The verified reports tie dialog `0x102` creation to `FUN_006AE2C0`/`FUN_006AE3F0`, owner-draw subclass routing to `FUN_0060F9A0`, checkbox painting/input to `OwnerDraw_Checkbox_006163A0`, trackbar painting/input to `OwnerDraw_Trackbar_0061D950`, standard option initialization to `FUN_006AE6E0`, and Start/Back packing to `FUN_006ACEE0`.

## Pipeline

Dialog `0x102` resource DLU rects -> Win32 child control placement -> common owner-draw subclass setup -> checkbox/trackbar state initialization -> owner-draw paint using PCX/primitive/text calls -> Rust layout/state/render emission -> player sees check icons, labels, slider rails, thumbs, plaques, and value text.

## Stage Results

| Stage | gamemd result | Rust result | Verdict |
|---|---|---|---|
| Active owner-draw path | `Button` checkbox style routes to `OwnerDraw_Checkbox_006163A0`; `msctls_trackbar32` routes to `OwnerDraw_Trackbar_0061D950`; active in standard `0x102`. | Current Rust has `SkirmishCheckboxId`, `SkirmishTrackbarId`, atlas entries, render instances, and input handlers for these controls. | PASS |
| Checkbox control rects | Retail 800x600 rects: `0x54E [72,286,150,16]`, `0x693 [72,314,150,16]`, `0x696 [72,341,150,16]`, `0x69A [72,371,155,16]`, `0x69D [302,369,249,18]`. | Rust computes `[71,286,150,16]`, `[71,314,150,16]`, `[71,341,150,16]`, `[71,371,155,16]`, `[302,369,249,18]`. | FAIL |
| Checkbox icon/text geometry | Icon destination is fixed `18x18` at control top-left; label rect advances left by `26`; flags vertical-center only. | Constants match `18x18` and `+26`, but the first four derived icon/text rects inherit the wrong x `71` instead of `72`. | FAIL |
| Checkbox assets/states | Standard `0x102` uses `cue_i.pcx` unchecked and `cce_i.pcx` checked; variant bytes remain `0/0`; defaults are checked from YR rules/session fallback. | Rust loads `cue_i.pcx`/`cce_i.pcx`, maps all five state fields, and `GameOptions::default()` now has Short Game, Super Weapons, Build Off Ally, Crates, and MCV Repacks true. | PASS |
| Checkbox hit behavior | `WM_LBUTTONDOWN`/double-click toggles only inside local `18x18` icon gate; label clicks do not toggle. | `handle_option_mouse_down` checks `checkbox_icon_rect` only; tests cover icon toggle and label non-toggle. | PASS |
| Trackbar control rects | Retail rects are game speed `[404,286,128,21]`, credits `[404,314,128,21]`, unit count `[404,341,128,21]`. | Rust computes game speed `[404,286,128,21]`, credits `[404,314,128,21]`, unit count `[404,340,128,21]`. | FAIL |
| Trackbar active width and value mapping | For `128x21`, numeric display reserves `50`, subtracts `13`, active width `65`; mouse x uses `mouse_x - 6`, clamp `[1,66]`, `(x-1)*(span+1)/65`, saturation, then step quantization. | `trackbar_active_width`, `trackbar_pixel_offset`, and `trackbar_mouse_value` implement the same values for the concrete standard ranges `0..6`, `5000..10000 step 100`, and `0..10`. | PASS |
| Trackbar plaque geometry | Plaque middle/caps start at local x `client_width - 50 + 1 = 79`, y `-1`, width `50`; value text rect is local `[79,0,128,21]`. Absolute plaque starts are `[483,285]`, `[483,313]`, `[483,340]`. | `trackbar_plaque_rect` starts at `rect.x + rect.w - 50`, `rect.y`: `[482,286]`, `[482,314]`, `[482,340]`. Value text rect starts at `[483,y]`, so text x matches but value plaque art is shifted; unit y only coincidentally matches plaque y because the control rect is off by -1. | FAIL |
| Trackbar thumb and asset family | Retail uses `trakgrip.pcx`, thumb x `control_left + 1 + pixel_offset`, width `12`, height `client_height`; `trofl/trofm/trofr.pcx` are plaque pieces; no `BTN-MINS/BTN-PLUS.SHP`. | Rust loads and renders `trakgrip.pcx` and `trofl/trofm/trofr.pcx`, uses `trackbar_thumb_rect(rect, px)` with width `12`, and does not use plus/minus SHPs. | PASS |
| Primitive rail pixels | Retail uses `FUN_006208F0` two-ring primitive bevel calls with fixed bevel globals; final pixels still depend on the DirectDraw surface conversion and need screenshot/pixel capture for literal equality. | Rust pre-renders a `skirmish_trackbar_rail` RGBA primitive with equivalent-looking colors and geometry, but no retail pixel capture was compared in this trace. | UNCHECKED |
| Shell text colors for this slice | Normal owner-draw label source color is packed `0x0000FFFF` => RGB `(255,255,0)`; button/value source `0x00000C05` => RGB `(5,12,0)`; checkbox flags are `0x04`, trackbar value flags are centered. | Rust uses muted label RGB `[0.94,0.84,0.42]`, and `SHELL_BUTTON_TEXT_RGB_00000C05` is `[0,12,5]` instead of `[5,12,0]`. Text rect constants mostly match, but checkbox and unit-count text inherit the rect errors above. | FAIL |

## Failures

### 1. First four checkbox controls are one pixel too far left

**Stage:** Checkbox control rects  
**Player-visible difference:** Short Game, MCV Repacks, Crates Appear, and Super Weapons icons and labels render one pixel left of retail.  
**Rust:** `src/ui/skirmish_shell/layout.rs:201`, `src/ui/skirmish_shell/layout.rs:469`, `src/ui/skirmish_shell/layout.rs:473`, `src/ui/skirmish_shell/layout.rs:477`, `src/ui/skirmish_shell/layout.rs:481`  
**gamemd evidence:** `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md` section 2 lists retail control rects at x `72`, active via dialog `0x102` and `FUN_006AE6E0`.

### 2. Unit Count trackbar is one pixel too high

**Stage:** Trackbar control rects  
**Player-visible difference:** The unit-count slider rail/thumb/value text row appears one pixel above retail and can crowd the Crates/Super Weapons rows differently.  
**Rust:** `src/ui/skirmish_shell/layout.rs:376`, `src/ui/skirmish_shell/layout.rs:464`  
**gamemd evidence:** `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md` section 2 lists `0x50C` as `[404,341,128,21]`; standard init at `FUN_006AE6E0`.

### 3. Trackbar plaque art is placed one pixel left and usually one pixel too low

**Stage:** Trackbar plaque geometry  
**Player-visible difference:** The numeric plaque art behind Game Speed/Credits/Unit Count values is not aligned like retail; the text may look centered over a shifted plate.  
**Rust:** `src/ui/skirmish_shell/layout.rs:222`, `src/app_skirmish_shell_render.rs:728`  
**gamemd evidence:** `OwnerDraw_Trackbar_0061D950` paint geometry uses local x `client_width - 50 + 1 = 79`, y `-1` for `trofm/trofl/trofr`, verified in `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.

### 4. Trackbar/button dark text color is channel-swapped

**Stage:** Shell text colors  
**Player-visible difference:** Trackbar numeric values and owner-draw button text use the wrong dark tint, reading as swapped blue/red relative to the verified source color.  
**Rust:** `src/app_skirmish_shell_render.rs:45`, `src/app_skirmish_shell_render.rs:1547`  
**gamemd evidence:** `SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md` verifies packed `0x00000C05` is RGB `(5,12,0)`, and `SKIRMISH_CREDITS_TRACKBAR_CLICK_DRAG_TRACE.md` already records this as a visible fail for trackbar values.

### 5. Checkbox/static label color is not the verified owner-draw source color

**Stage:** Shell text colors  
**Player-visible difference:** Checkbox labels and other normal shell labels are muted tan instead of the verified source yellow used by the owner-draw text wrapper.  
**Rust:** `src/app_skirmish_shell_render.rs:46`, `src/app_skirmish_shell_render.rs:1527`  
**gamemd evidence:** `SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md` verifies `DAT_00AC18A4 = 0x0000FFFF`, interpreted by `FUN_00621040` as RGB `(255,255,0)`, active for checkbox/static/combo label callers.

## Not Implemented

None for the scoped enabled idle/checked/value visual surfaces. Conditional disabled overlays/colors are verified in gamemd but are outside this concrete enabled standard setup state, so they were not counted as NOT-IMPLEMENTED here.

## Unchecked

Primitive rail final pixels remain unchecked. Static Ghidra evidence verifies the `FUN_006208F0` geometry/color ordering, and Rust has a corresponding pre-rendered RGBA rail, but this trace did not capture a retail 800x600 frame and compare literal rail pixels.

## Adjacent Findings

- Rust still hardcodes the three trackbar ranges in UI/render paths. The concrete standard YR numbers match this scenario, but retail sources credits/unit/speed defaults from Rules/session fields.
- Existing older docs that say checkbox/trackbar rendering is absent are stale; current source has atlas loading, rendering, state, and hit tests for these controls.
- Changed-value `WM_HSCROLL` notifications and trackbar UI sounds are not visual parity issues for this trace, but `SKIRMISH_CREDITS_TRACKBAR_CLICK_DRAG_TRACE.md` already records them as behavioral/audio differences.

