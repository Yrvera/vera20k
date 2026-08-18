# Skirmish Checkbox / Trackbar Pixel Geometry - Ghidra Research Report

**Address(es):** `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `FUN_006AE6E0 @ 0x006AE6E0`, `FUN_0060F9A0 @ 0x0060F9A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Offline Skirmish dialog `0x102` checkbox and trackbar final placement plus owner-draw pixel geometry: checkbox icon/text placement, checked/unchecked PCX names, trackbar rail/thumb/value plaque geometry, range/value mapping, disabled visual state.  
**Non-Scope:** Runtime gameplay effects after match launch, in-game Options dialog, online/host/guest shell variants, non-Skirmish `SliderClass` gadget screens.  
**Confidence:** High for standard offline `0x102` placement, asset names, message/range mapping, checkbox icon/text offsets, and trackbar thumb/value-plaque formulas. Medium for exact primitive rail bevel pixel appearance because `FUN_006208F0` is shared beveled-rectangle code and this slot did not screenshot-match its raster output.  
**Active in YR:** Yes for the standard offline Skirmish `0x102` path. Evidence: `FUN_006AE2C0`/`FUN_006AE3F0` create dialog `0x102`; `FUN_0060F9A0` routes `Button` style `0x03` to `0x006163A0` and `msctls_trackbar32` to `0x0061D950`; `FUN_006AE6E0` initializes the scoped controls.

## 1. Overview

Standard offline Skirmish checkboxes and trackbars are normal Win32 child controls from resource `0x102`, then subclassed into Westwood owner-draw callbacks. The checkboxes use PCX icons plus text, while trackbars use primitive bevel fills for the rail, `trakgrip.pcx` for the thumb, and `trofl/trofm/trofr.pcx` for the numeric value plaque.

Active in YR: Yes. Evidence: `FUN_0060F9A0 @ 0x0060F9A0` class/style dispatch, `FUN_006AE6E0 @ 0x006AE6E0` `GetDlgItem`/`SendMessageA` setup for `0x529`, `0x511`, `0x50C`, `0x54E`, `0x69A`, `0x69D`, `0x693`, `0x696`.

## 2. Final Placement

Dialog `0x102` uses `MS Sans Serif` 8 pt with verified base units `baseX=6`, `baseY=13`; positive DLU values convert with `MulDiv`. These are the 800x600 shell-client rectangles. Per the high-res hosting report, ordinary child controls remain at their DLU-derived positions inside the full-screen top-left parent; selective right-panel anchoring does not apply to these lower option controls.

| ID | Resource DLU rect | Pixel rect `[x,y,w,h]` | Role | Active in YR |
|---:|---:|---:|---|---|
| `0x54E` | `(48,176,100,10)` | `(72,286,150,16)` | `GUI:ShortGame` checkbox | Yes; resource `0x102`, init at `0x006AE6E0` |
| `0x693` | `(48,193,100,10)` | `(72,314,150,16)` | `GUI:MCVRepacks` checkbox | Yes; `0x006AE6E0` |
| `0x696` | `(48,210,100,10)` | `(72,341,150,16)` | `GUI:CratesAppear` checkbox | Yes; `0x006AE6E0` |
| `0x69A` | `(48,228,103,10)` | `(72,371,155,16)` | `GUI:SuperWeaponsAllowed` checkbox | Yes; `0x006AE6E0` |
| `0x69D` | `(201,227,166,11)` | `(302,369,249,18)` | `GUI:BuildOffAlly` checkbox | Yes; `0x006AE6E0` |
| `0x529` | `(269,176,85,13)` | `(404,286,128,21)` | game-speed trackbar | Yes; `0x006AE6E0` |
| `0x511` | `(269,193,85,13)` | `(404,314,128,21)` | credits trackbar | Yes; `0x006AE6E0` |
| `0x50C` | `(269,210,85,13)` | `(404,341,128,21)` | unit-count trackbar | Yes; `0x006AE6E0` |

The labels at `0x699`, `0x69B`, and `0x69C` are separate statics at x `302`, y `286/314/341`, size `90x16`; they are not part of the trackbar owner-draw window. Active in YR: Yes, from resource `0x102`.

## 3. Checkbox Pixel Geometry

`OwnerDraw_Checkbox_006163A0` uses two separate rectangles:

- icon blit destination: control window top-left, fixed size `18x18`;
- label text rectangle: original control window rectangle with `left += 0x1A` (`26` px).

This means four of the five resource checkboxes have a 16 px control height but still draw an 18 px icon and accept an 18x18 click gate. The `BuildOffAlly` checkbox has an 18 px control height and matches the icon height.

| ID | Control pixel rect | Icon dest | Label text rect | Active in YR |
|---:|---:|---:|---:|---|
| `0x54E` | `[72,286,222,302]` | `[72,286,18,18]` | `[98,286,222,302]` | Yes; `0x0061649D..0x00616674` |
| `0x693` | `[72,314,222,330]` | `[72,314,18,18]` | `[98,314,222,330]` | Yes; same callback |
| `0x696` | `[72,341,222,357]` | `[72,341,18,18]` | `[98,341,222,357]` | Yes; same callback |
| `0x69A` | `[72,371,227,387]` | `[72,371,18,18]` | `[98,371,227,387]` | Yes; same callback |
| `0x69D` | `[302,369,551,387]` | `[302,369,18,18]` | `[328,369,551,387]` | Yes; same callback |

Click/toggle geometry is exactly the same 18x18 icon gate: `WM_LBUTTONDOWN (0x201)` and double-click `0x203` toggle only when `x < 0x12 && y < 0x12`. Clicking the label text does not toggle the checkbox in this owner-draw path. Active in YR: Yes. Evidence: `0x006166EE..0x00616708`.

## 4. Checkbox Assets And States

Default Skirmish init does not send variant messages `0x4E5` or `0x4E6`, so byte fields `+0xD9/+0xDA` remain zero-filled from the shell control-state constructor. The standard offline Skirmish checkbox art is therefore:

| State | PCX | Evidence | Active in YR |
|---|---|---|---|
| unchecked default | `cue_i.pcx` | format string `c%ce_i.pcx @ 0x00835968` with `%c='u'`; `0x006165AA..0x006165C8` | Yes |
| checked default | `cce_i.pcx` | same format string with `%c='c'`; `0x006165AA..0x006165C8` | Yes |
| variant checked-left | `cce_il.pcx` | direct string `0x0083598C`; branch when `+0xD9 != 0`, checked, `+0xDA == 0` | Conditional: helper active, not standard `0x102` init |
| variant unchecked-right | `cce_ir.pcx` | direct string `0x00835980`; branch when `+0xD9 != 0`, unchecked, `+0xDA != 0` | Conditional: helper active, not standard `0x102` init |

Disabled checkbox visual is style-gated by `WS_DISABLED (0x08000000)`: after icon blit it alpha-blends the icon rect using `DAT_00AC4898`, and label text switches from `DAT_00AC18A4` to disabled color `DAT_00AC1CB4`. Active in YR: Conditional; standard offline Skirmish can disable row-related controls, but these five option checkboxes are not disabled by the standard init path found in `FUN_006AE6E0`. Evidence: `0x00616619..0x00616668`.

## 5. Trackbar Geometry And Assets

All three Skirmish trackbars have a 128x21 client rect in the 800x600 layout. The first owner-draw messages leave step/display unset, so `OwnerDraw_Trackbar_0061D950` normalizes `step=1`, `numeric_display=1`, and reserves `0x32` (`50`) px at the right for the value plaque. The active slider width is:

```text
active_width = max(1, client_width - value_plaque_width - 0x0D)
             = max(1, 128 - 50 - 13)
             = 65 px
```

Active in YR: Yes. Evidence: `0x0061DA52..0x0061DA7D` active-width formula; `0x0061DB94..0x0061DBAD` default step/display normalization.

Trackbar paint uses:

| Element | Geometry / behavior | Evidence | Active in YR |
|---|---|---|---|
| numeric plaque middle | x `client_width - 50 + 1 = 79`, y `-1`, width `50`, tiled `trofm.pcx` | `0x0061DE9C..0x0061DF04`, string `trofm.pcx @ 0x00835A28` | Yes |
| plaque left cap | same plaque left/top, blit `trofl.pcx` | `0x0061DF04..0x0061DF7B`, string `0x00835A1C` | Yes |
| plaque right cap | right-aligned inside plaque, blit `trofr.pcx` | `0x0061DF7B..0x0061E005`, string `0x00835A10` | Yes |
| thumb | dest x `1 + pixel_offset`, y `0`, width `12`, height `client_height`; source full `trakgrip.pcx` | `0x0061E00C..0x0061E0AD`, string `0x00835A00` | Yes |
| rail/base | primitive beveled rectangles with border width argument `2`, not `BTN-MINS/BTN-PLUS.SHP` | calls `FUN_006208F0(2, color)` at `0x0061E1F3..0x0061E269` | Yes |
| numeric text | rect `[client_right - 0x31, client_top, client_right, client_bottom]` = `[79,0,128,21]`; centered flag `4` through `FUN_00621040` | `0x0061E2B6..0x0061E30A` | Yes |

The generic `BTN-MINS.SHP`/`BTN-PLUS.SHP` `SliderClass` path is not used by these controls. Active in YR: No for standard offline Skirmish trackbars. Evidence: `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md` plus live `FUN_006AE6E0`/`OwnerDraw_Trackbar_0061D950` PCX path.

## 6. Trackbar Value Mapping

State fields in the owner-draw control record are byte offsets from the per-control state pointer: `+0xE8` dragging, `+0xEC` thumb-drag flag, `+0xF0` range span, `+0xF4` relative value, `+0xF8` minimum, `+0xFC` pixel offset, `+0x100` step, `+0x104` numeric-display flag, `+0x108` sound-suppression flag.

| Message / input | Behavior | Evidence | Active in YR |
|---|---|---|---|
| range `0x406` | min = low word, max = high word, span = max - min; current is clamped, then pixel offset = `active_width * rel / span` | `0x0061E59A..0x0061E5C9` | Yes |
| set pos `0x405` | accepts absolute value only when `0 <= value - min <= span`; stores relative value; updates pixel offset | `0x0061E486..0x0061E4A8` | Yes |
| get pos `0x400` | returns `((min + rel) / step) * step`, integer truncating | `0x0061E4AD..0x0061E4C4` | Yes |
| mouse x mapping | uses `(mouse_x - 6)` clamped to `[1, client_right - plaque_width - 0x0C]`, then `((x - 1) * (span + 1)) / active_width`, saturating at span and quantizing by step | `0x0061DC04..0x0061DC5A`, `0x0061E545..0x0061E594` | Yes |
| mouse y gate | click/drag logic only runs when `mouse_y > client_bottom - 0x12`; with 21 px controls, y `0..3` does not begin slider interaction | `0x0061E4F5..0x0061E512` | Yes |
| thumb hit gate | if x is within `[thumb_x, thumb_x + 12)`, mouse down starts thumb dragging instead of immediately remapping the value | `0x0061E518..0x0061E540` | Yes |

Initialization values:

| Control | Range | Initial visual pos | Step | Display value | Evidence | Active in YR |
|---:|---|---|---|---|---|---|
| `0x529` | `0..6` | `6 - DAT_00A8B268` | default `1` | visual position, so faster stored speed appears farther right/left through inversion | `0x006AEB6D..0x006AEB8F`; apply reverses at `0x006AD730` | Yes |
| `0x511` | Rules `+0x1480..+0x1488` = YR `5000..10000` | `DAT_00A8B25C` | Rules `+0x148C` = `100` | credits | `0x006AEB91..0x006AEBD1`, `ini/rulesmd.ini:3018..3021` | Yes |
| `0x50C` | Rules `+0x1490..+0x1498` = YR `0..10` | `DAT_00A8B270` | default `1` | unit count | `0x006AEBD3..0x006AEBFF`, `ini/rulesmd.ini:3022..3024` | Yes |

Disabled trackbar visual is style-gated by `WS_DISABLED (0x08000000)`: the thumb receives an alpha overlay, rail/text color changes to disabled globals (`DAT_00AC1CA8` for primitive color, `DAT_00AC1CB4` for text), and normal input is still routed through the subclass/window proc depending on Windows disabled dispatch. Active in YR: Conditional; no standard `FUN_006AE6E0` branch disables these three controls. Evidence: `0x0061E0B0..0x0061E1B9`, `0x0061E2A1..0x0061E2B6`.

## 7. Current Rust Implementation Status

Rust does not yet implement these pixel-faithful checkbox/trackbar controls in the experimental Skirmish shell. `rg` found no `trakgrip`, `trofl/trofm/trofr`, `cue_i`, `cce_i`, or scoped control IDs in `src/`; current menu code exposes only simplified `egui` options such as `short_game` in `src/ui/main_menu.rs`, and the experimental shell renderer focuses on shell chrome/buttons/text.

Active in YR: Not applicable to Rust. Evidence: source scan of `src/` for the scoped assets/control IDs; Codegraph context did not find Skirmish checkbox/trackbar renderer entry points.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x102` placement of scoped controls | verified | resource rects from `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`; DLU conversion base `6/13`; high-res origin report | none |
| checkbox callback selection | verified | `FUN_0060F9A0 @ 0x0060F9A0`, `Button` style low bits `0x03` -> `0x006163A0` | none |
| checkbox icon/text geometry | verified | `0x0061649D..0x00616674` | none |
| checkbox checked/unchecked PCX names | verified | strings `0x00835968..0x00835998`; branch `0x006164E1..0x006165C8` | none |
| checkbox click hit gate | verified | `0x006166EE..0x00616708` | none |
| trackbar callback selection | verified | `FUN_0060F9A0`, class `msctls_trackbar32` -> `0x0061D950` | none |
| trackbar asset names | verified | strings `0x00835A00..0x00835A28`, paint branch `0x0061DE9C..0x0061E0AD` | none |
| trackbar active-width / thumb formula | verified | `0x0061DA52..0x0061DC5A`, `0x0061E545..0x0061E594` | none |
| trackbar primitive rail raster | touched-not-exhausted | `FUN_006208F0 @ 0x006208F0`, call sites `0x0061E1F3..0x0061E269` | screenshot/runtime validation for exact bevel pixels |
| runtime option effects after match launch | deferred | user scope excludes it | separate consumer trace |
| generic `SliderClass` plus/minus SHPs | verified-negative for this scope | `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md` | broad non-Skirmish gadget inventory if desired |

## 9. Open Questions - Final State

[RESOLVED] OQ1 - What are the final pixel positions of the scoped controls? Listed in section 2. Evidence: resource `0x102` DLU rects and verified `baseX=6/baseY=13`.  
[RESOLVED] OQ2 - Which callback paints each scoped control? Checkboxes use `OwnerDraw_Checkbox_006163A0`; trackbars use `OwnerDraw_Trackbar_0061D950`. Evidence: `FUN_0060F9A0`.  
[RESOLVED] OQ3 - Where is checkbox text placed relative to the icon? Text rect is control rect with left advanced by `0x1A`; icon is fixed `18x18` at control top-left. Evidence: `0x0061649D..0x00616674`.  
[RESOLVED] OQ4 - Which checkbox PCXs are live in normal Skirmish? `cue_i.pcx` unchecked and `cce_i.pcx` checked. Evidence: format branch `0x006165AA..0x006165C8`; no standard `0x4E5/0x4E6` sends in `FUN_006AE6E0`.  
[RESOLVED] OQ5 - Do Skirmish sliders use `BTN-MINS.SHP`/`BTN-PLUS.SHP`? No. Active path uses `trakgrip.pcx` and `trof*.pcx`; plus/minus SHPs belong to generic `SliderClass`. Evidence: `0x0061DE9C..0x0061E0AD`, prior `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md`.  
[RESOLVED] OQ6 - How does the 128x21 trackbar map to values? It reserves 50 px for the numeric plaque, subtracts 13 px more, leaving 65 px active width; mouse and thumb formulas are listed in sections 5-6. Evidence: `0x0061DA52..0x0061DC5A`, `0x0061E545..0x0061E594`.  
[DEFERRED] OQ7 - What exact final colors do the primitive rail bevel pixels have on a retail 16-bit surface? Category: needs-runtime-debugger/screenshot. Reason: `FUN_006208F0` and global color conversion were verified, but pixel-perfect final raster should be screenshot-checked.

## Sources

- Ghidra decompile/disassembly: `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`
- Ghidra decompile/disassembly: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`
- Ghidra decompile: `FUN_0060F9A0 @ 0x0060F9A0`
- Ghidra decompile: `FUN_006AE6E0 @ 0x006AE6E0`
- Ghidra decompile: `FUN_006ACEE0 @ 0x006ACEE0`
- Ghidra decompile: `FUN_006208F0 @ 0x006208F0`
- Ghidra decompile: `FUN_00621040 @ 0x00621040`
- Ghidra string memory: `cue_i.pcx @ 0x00835974`, `cce_ir.pcx @ 0x00835980`, `cce_il.pcx @ 0x0083598C`, `cce_i.pcx @ 0x00835998`, `trakgrip.pcx @ 0x00835A00`, `trofr.pcx @ 0x00835A10`, `trofl.pcx @ 0x00835A1C`, `trofm.pcx @ 0x00835A28`
- Prior docs: `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
- Rust scan: `rg` over `C:/Users/enok/Documents/ra2-rust-game/src`
