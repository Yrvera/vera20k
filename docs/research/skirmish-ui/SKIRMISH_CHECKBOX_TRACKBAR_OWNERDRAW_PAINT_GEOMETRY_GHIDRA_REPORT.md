# Skirmish Checkbox / Trackbar Owner-Draw Paint Geometry - Ghidra Research Report

**Address(es):** `FUN_0060F9A0`, `OwnerDraw_Checkbox_006163A0`, `OwnerDraw_Trackbar_0061D950`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_0061F210`, `FUN_006208F0`, `FUN_00621040`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard offline Yuri's Revenge Skirmish dialog `0x102` checkbox and trackbar owner-draw/subclass paths, paint geometry, control-state fields, asset names, checked/pressed/disabled behavior, active-YR reachability, current Rust status, and implementation handoff.  
**Non-Scope:** In-game Options dialog reuse, online/host/guest lobby variants, complete primitive bevel raster proof by screenshot, post-launch gameplay effects of the packed options, PCX decoder internals below the already verified owner-draw cache, and localization/CSF text payloads.  
**Confidence:** High for callback routing, active-YR reachability, asset names, geometry constants, state offsets, message behavior, and current Rust delta. Medium for final retail RGB of primitive rail bevel pixels because this pass verified the code path but did not capture a live 16-bit retail surface.  
**Active in YR:** Yes for the standard offline Skirmish `0x102` controls. Conditional only for disabled and variant art paths that the helpers support but standard Skirmish init does not normally enable.

## 1. Overview

Offline Skirmish checkboxes and trackbars are ordinary Win32 child controls from dialog resource `0x102` that the common shell owner-draw hook subclasses during dialog initialization. Button style low bits `0x03` route the five option checkboxes to `OwnerDraw_Checkbox_006163A0`; class `msctls_trackbar32` routes the three sliders to `OwnerDraw_Trackbar_0061D950`.

The player-visible result is PCX-backed checkbox icons (`cue_i.pcx` / `cce_i.pcx`) plus owner-draw text, and primitive-backed trackbars with `trakgrip.pcx` and optional right-side `trofl/trofm/trofr.pcx` value plaques. No `BTN-MINS.SHP`, `BTN-PLUS.SHP`, or `bst_*` checkbox art is live for these standard `0x102` controls.

Current Rust status correction as of 2026-05-23: current Rust now exposes final
checkbox rects, renders `cue_i/cce_i` icons and labels, renders trackbar
rail/plaque/thumb/value text, and implements icon-only checkbox hits plus
y-gated trackbar click/drag behavior. `GameOptions::default().build_off_ally`
now matches the verified YR enabled fallback. The remaining scoped uncertainty
is final retail-pixel validation of the pre-rendered primitive rail color.

## 2. Class Layout / Key Offsets

The owner-draw state pointer used by both callbacks is the per-control record returned from the hash table rooted at `DAT_00AC1B00`; decompilation exposes fields as `piVar[slot]`, which means byte offset `slot * 4` from the callback's `piVar10/piVar12` state pointer unless listed as a direct byte offset.

| Control state area | Offset / slot | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|---|
| common cached surface | `+0x10` / `[4]` | pointer | Trackbar cached background/control surface; allocated on first paint if null | `0x0061DDxx..0x0061DE8F` | Yes |
| common text pointer | `+0x28` / `[10]` | pointer | Checkbox label text gate; only draw label if non-null | `0x00616644..0x00616674` | Yes |
| common font pointer | `+0x64` / `[0x19]` | pointer | Font passed to `FUN_00621040` for checkbox labels and trackbar value text | `0x00616674`, `0x0061E30A` | Yes |
| checkbox checked state | `+0xE8` / `[0x3A]` | int | Stored check state returned by `0xF0`, written by `0xF1`, toggled on icon click | `0x0061670E`, `0x00616813` | Yes |
| checkbox variant A | `+0xD9` | byte | Chooses default formatted PCX path when zero, alternate variant path when nonzero | `0x006164E1`, `0x00616833` | Helper active; standard `0x102` leaves zero |
| checkbox variant B | `+0xDA` | byte | Selects left/right alternate PCX within variant path | `0x0061650x..0x006165A0`, `0x00616833` | Helper active; standard `0x102` leaves zero |
| trackbar capture/dragging | `+0xE8` / `[0x3A]` | int | Mouse capture state | `0x0061E4CE..0x0061E4F5`, final write `0x0061E609` | Yes |
| trackbar thumb-drag flag | `+0xEC` / `[0x3B]` | int | Set when mouse down begins inside current 12 px thumb interval | `0x0061E518..0x0061E540` | Yes |
| trackbar span | `+0xF0` / `[0x3C]` | int | `max - min`; defaulted from previous Win32 proc if zero | `0x0061DA85..0x0061DB76`, `0x0061E59A` | Yes |
| trackbar relative value | `+0xF4` / `[0x3D]` | int | Current value relative to minimum | `0x0061E486`, final write `0x0061E609` | Yes |
| trackbar minimum | `+0xF8` / `[0x3E]` | int | Range minimum | `0x0061E59A` | Yes |
| trackbar pixel offset | `+0xFC` / `[0x3F]` | int | Thumb x offset into active track | `0x0061E486..0x0061E4A8` | Yes |
| trackbar step | `+0x100` / `[0x40]` | int | Quantization step; zero normalizes to `1` | `0x0061DB94..0x0061DBAD`, `0x0061E4AD` | Yes |
| trackbar numeric display | `+0x104` / `[0x41]` | int | Enables `0x32` px value plaque and numeric text | `0x0061DA52`, `0x0061DB94..0x0061DBAD` | Yes |
| trackbar sound suppression | `+0x108` / `[0x42]` | int | Suppresses click sound after value changes | `0x0061E46x` custom `0x4AE`, final branch `0x0061E609` | Yes |

Tiny detail: the checkbox and trackbar callbacks reuse the same numeric slots for different meanings because each control has its own callback-specific state interpretation.

## 3. Core Logic

### 3.1 Active entry and subclass routing

`FUN_006AE2C0` calls `FUN_0072CF40`, creates dialog `0x102` through the shell dialog creation path, and runs the dialog pump until Start `0x617` or Back `0x5C0`. `FUN_006AE3F0` delegates every message to the common shell dialog proc first; for custom init message `0x497`, it calls `FUN_006AE6E0`, and for `WM_COMMAND 0x111`, it calls `FUN_006ACEE0`.

`FUN_0060F9A0` is the owner-draw hook setup. It reads each child class name and style, installs thunk/proc `0x00610CA0` with `SetWindowLongA(GWL_WNDPROC)`, stores the callback and previous WndProc in hash tables, creates/updates the per-control state record, snapshots existing text, then sends message `0x497`.

Routing facts:

| Class / style | Callback | Scoped controls | Evidence | Active in YR |
|---|---:|---|---|---|
| `Button`, style low bits `(style & 3) == 3` after earlier button checks | `OwnerDraw_Checkbox_006163A0` | `0x54E`, `0x693`, `0x696`, `0x69A`, `0x69D` | `FUN_0060F9A0`, button style dispatch | Yes |
| `msctls_trackbar32` | `OwnerDraw_Trackbar_0061D950` | `0x529`, `0x511`, `0x50C` | `FUN_0060F9A0`, class-name dispatch | Yes |

Style-order detail: button style `(style & 7) == 7` and `(style & 0x0B) == 0x0B` are tested before `(style & 3) == 3`. The Skirmish option buttons use the checkbox style and therefore route to `0x006163A0`, not the main button callback.

### 3.2 Final control placement

The current trusted placement source is dialog resource `0x102` using `MS Sans Serif` 8 pt and base units `baseX=6`, `baseY=13`. Positive DLU values convert through Win32 `MulDiv`. Ordinary option checkboxes/trackbars remain DLU-derived child controls; the right-panel anchoring policy does not move them.

| ID | Resource DLU rect | Pixel rect `[x,y,w,h]` in 800x600 shell | Role | Evidence | Active in YR |
|---:|---:|---:|---|---|---|
| `0x54E` | `(48,176,100,10)` | `[72,286,150,16]` | `GUI:ShortGame` checkbox | prior resource/DLU reports, `FUN_006AE6E0` init | Yes |
| `0x693` | `(48,193,100,10)` | `[72,314,150,16]` | `GUI:MCVRepacks` checkbox | same | Yes |
| `0x696` | `(48,210,100,10)` | `[72,341,150,16]` | `GUI:CratesAppear` checkbox | same | Yes |
| `0x69A` | `(48,228,103,10)` | `[72,371,155,16]` | `GUI:SuperWeaponsAllowed` checkbox | same | Yes |
| `0x69D` | `(201,227,166,11)` | `[302,369,249,18]` | `GUI:BuildOffAlly` checkbox | same | Yes |
| `0x529` | `(269,176,85,13)` | `[404,286,128,21]` | Game speed trackbar | prior resource/DLU reports, `FUN_006AE6E0` init | Yes |
| `0x511` | `(269,193,85,13)` | `[404,314,128,21]` | Credits trackbar | same | Yes |
| `0x50C` | `(269,210,85,13)` | `[404,341,128,21]` resource; Rust currently applies a one-pixel y fixup to `[404,340,128,21]` | Unit count trackbar | resource/DLU report plus Rust parity tests | Yes |

### 3.3 Checkbox paint geometry

`OwnerDraw_Checkbox_006163A0` handles:

| Message / branch | Behavior | Evidence | Active in YR |
|---:|---|---|---|
| `0x497` | Calls previous WndProc with `0xF0` and stores the returned check state into `+0xE8` | `0x00616813..0x0061682B` | Yes |
| `0xF0` | Returns stored check state | `0x006163A0` branch | Yes |
| `0xF1` | Stores `wParam` as checked state and invalidates | `0x006163A0` branch | Yes |
| `WM_PAINT 0x0F` | Draws PCX icon at top-left, optional disabled alpha overlay, then label text offset by `0x1A` | `0x0061649D..0x00616679` | Yes |
| `WM_LBUTTONDOWN 0x201` / `WM_LBUTTONDBLCLK 0x203` | Toggles only inside 18x18 icon gate; label clicks do not toggle | `0x006166EE..0x00616730` | Yes |
| `0x4E5`, `0x4E6`, `0x4E7` | Set/query variant bytes `+0xD9/+0xDA` | `0x00616833..0x00616854` | Helper active; not standard `0x102` init |

Paint geometry:

| Element | Geometry / behavior | Evidence | Active in YR |
|---|---|---|---|
| icon destination | Window top-left, fixed `18x18` rect; the source PCX size is queried then blitted, but destination/click constants are `0x12` | `0x0061649D..0x00616621`, `0x006166EE..0x00616708` | Yes |
| text rect | Same control rect after adding `0x1A` (`26`) to left | assembly `0x0061663E..0x00616646` | Yes |
| text flags | `FUN_00621040` receives flags `0x04`, so v-centered and left anchored | assembly call `0x00616661..0x00616674` | Yes |
| normal text color | `DAT_00AC18A4` | `0x0061664C` | Yes |
| disabled text color | `DAT_00AC1CB4` when `WS_DISABLED 0x08000000` is set | `0x00616651..0x00616655` | Conditional |
| disabled icon overlay | `AlphaBlendRect(..., DAT_00AC4898)` after icon blit when `WS_DISABLED` | `0x00616619..0x00616635` | Conditional |
| click sound | `VocClass__PlayAtPos(1.0, 0)` after toggle | `0x0061672F` region | Yes |
| parent notification | Sends parent `WM_COMMAND 0x111`; low word is control id, high word is new checked state | `0x00616753..0x0061678B` | Yes |

Asset selection:

| State | Selected PCX | Evidence | Active in YR |
|---|---|---|---|
| default unchecked, `+0xD9 == 0` | `cue_i.pcx` via format `c%ce_i.pcx` with `%c='u'` | string `0x00835968`, branch `0x006164E1..0x006165C8` | Yes |
| default checked, `+0xD9 == 0` | `cce_i.pcx` via format with `%c='c'` | same | Yes |
| variant unchecked-right | `cce_ir.pcx` | string `0x00835980`; branch with `+0xD9 != 0`, unchecked, `+0xDA != 0` | Conditional |
| variant checked-left | `cce_il.pcx` | string `0x0083598C`; branch with `+0xD9 != 0`, checked, `+0xDA == 0` | Conditional |
| variant checked default/right-complete | `cce_i.pcx` | string `0x00835998`; variant branch | Conditional |

Important negative findings:

- `bst_uckg.pcx`, `bst_chkg.pcx`, `bst_uchk.pcx`, and `bst_chkd.pcx` are preloaded by `FUN_0061F210` but are not referenced by `OwnerDraw_Checkbox_006163A0`.
- `FUN_006AE6E0` sends only `BM_SETCHECK 0xF1` to the scoped standard Skirmish checkboxes. It does not send `0x4E5` or `0x4E6`, so the live standard path stays on `cue_i.pcx` / `cce_i.pcx`.
- Missing PCX lookup is not robust in this callback. After `FUN_006BA140`, the callback immediately dereferences the returned surface vtable.

### 3.4 Trackbar paint geometry

`OwnerDraw_Trackbar_0061D950` handles:

| Message / branch | Behavior | Evidence | Active in YR |
|---:|---|---|---|
| `WM_PAINT 0x0F` | Restores/caches background, draws optional numeric plaque, thumb, primitive rail/bevel, optional numeric text, validates | `0x0061DDxx..0x0061E30A` | Yes |
| `TBM_GETPOS 0x400` | Returns quantized absolute value `((min + rel) / step) * step` | `0x0061E4AD..0x0061E4C4` | Yes |
| `TBM_SETPOS-like 0x405` | Accepts value only if `0 <= value - min <= span`; updates relative value and pixel offset | `0x0061E486..0x0061E4A8` | Yes |
| `TBM_SETRANGE-like 0x406` | Low word min, high word max, span `max - min`; clamps current and recomputes pixel offset | `0x0061E59A..0x0061E5C9` | Yes |
| `0x4AB` | Sets step | `0x0061E43x` | Yes |
| `0x4AC` | Enables/disables numeric display | `0x0061E43x` | Yes |
| `0x4AE` | Sets sound suppression byte from `param_3 == 0` | `0x0061E43x` | Yes |
| mouse down / dblclick / move / up | Capture, thumb dragging, click-to-value mapping, invalidation, parent `WM_HSCROLL` notify | `0x0061E4CE..0x0061E609` | Yes |

State normalization and active width:

```text
if numeric_display == 0 before normalization:
    value_plaque_width = 0
active_width = (client_width - value_plaque_width) - 0x0D
if active_width < 2:
    active_width = 1
if step == 0:
    value_plaque_width = 0x32
    step = 1
    numeric_display = 1
```

For standard `128x21` Skirmish trackbars after normalization:

```text
value_plaque_width = 50
active_width = 128 - 50 - 13 = 65
```

Evidence: `0x0061DA52..0x0061DA7D`, `0x0061DB94..0x0061DBAD`.

Trackbar paint elements:

| Element | Geometry / behavior | Evidence | Active in YR |
|---|---|---|---|
| numeric plaque middle | `trofm.pcx`, tiled at x `client_width - 50 + 1`, y `-1`, width `50` | `0x0061DE9C..0x0061DF04`, string `0x00835A28` | Yes |
| plaque left cap | `trofl.pcx`, direct blit at same plaque left/top | `0x0061DF04..0x0061DF7B`, string `0x00835A1C` | Yes |
| plaque right cap | `trofr.pcx`, direct blit right-aligned inside plaque | `0x0061DF7B..0x0061E005`, string `0x00835A10` | Yes |
| thumb | `trakgrip.pcx`, direct blit at `x = control_left + 1 + pixel_offset`, `y = control_top`, height from control surface; math treats thumb width as `12` px | `0x0061E00C..0x0061E0AD`, string `0x00835A00` | Yes |
| disabled thumb overlay | `AlphaBlendRect(..., DAT_00AC4898)` after thumb blit if `WS_DISABLED` | `0x0061E0B0..0x0061E0D9` | Conditional |
| rail/base | Two calls to `FUN_006208F0` with border width argument `2`; color is `DAT_00AC4624` normal or `DAT_00AC1CA8` disabled after DirectDraw conversion | `0x0061E1F3..0x0061E269`, `0x006208F0` | Yes |
| numeric value text | `FUN_007CA564` formats the quantized value, then `FUN_00621040` draws in rect `[right-0x31, top, right, bottom]`, flags `0x05` | `0x0061E29C..0x0061E30A` | Yes |

Trackbar input/value mapping:

| Detail | Behavior | Evidence | Active in YR |
|---|---|---|---|
| thumb interval | Current thumb gate is `[thumb_x, thumb_x + 12)`. Mouse down inside it starts drag instead of immediate remap | `0x0061E518..0x0061E540` | Yes |
| vertical click gate | Slider interaction runs only when `mouse_y > client_bottom - 0x12`; for 21 px controls, y `0..3` does not start interaction | `0x0061E4F5..0x0061E512` | Yes |
| raw mouse x clamp | Uses `mouse_x - 6`, clamped to `[1, client_right - plaque_width - 0x0C]` | `0x0061E545..0x0061E568` | Yes |
| raw value | `((x - 1) * (span + 1)) / active_width`, then saturates at `span` | `0x0061E568..0x0061E594` | Yes |
| quantization | `((raw + min) / step) * step - min`, integer truncating | `0x0061E58x..0x0061E594` | Yes |
| parent notify | On changed value/range/min, invalidates and sends parent `WM_HSCROLL 0x114` with low word `5`, high word current absolute value | final branch `0x0061E609` | Yes |
| click sound | Plays `VocClass__PlayAtPos(1.0, 0)` if changed, branch permits sound, and suppression state is zero | final branch `0x0061E609` | Yes |

Important negative findings:

- `BTN-MINS.SHP` and `BTN-PLUS.SHP` strings have no function xrefs and are not used by `OwnerDraw_Trackbar_0061D950`.
- The owner-draw trackbars are PCX/primitive controls, not the older plus/minus SHP `SliderClass` path.
- Missing `trof*` or `trakgrip.pcx` lookup is not robust in this callback; the returned surface is immediately dereferenced.

### 3.5 Skirmish init/apply integration

`FUN_006AE6E0` initializes the scoped controls:

| Control | Init behavior | Evidence | Active in YR |
|---:|---|---|---|
| `0x529` | Copies `DAT_00A8B3CC` to `DAT_00A8B268`; sends range `0..6`; sends position `6 - DAT_00A8B268` | `0x006AEB6D..0x006AEB8F` | Yes |
| `0x511` | Copies `DAT_00A8B3D0` to `DAT_00A8B25C`; sends Rules min/max money; sends position; sends step from Rules `+0x148C` | `0x006AEB91..0x006AEBD1` | Yes |
| `0x50C` | Copies `DAT_00A8B3D4` to `DAT_00A8B270`; sends Rules min/max unit count; sends position | `0x006AEBD3..0x006AEBFF` | Yes |
| `0x54E` | Copies mirror to live Short Game and sends `0xF1` if child exists | `0x006AEDA0..0x006AEDD0` | Yes |
| `0x69A` | Copies mirror to live Super Weapons and sends `0xF1` | same block | Yes |
| `0x69D` | Copies mirror to live Build Off Ally and sends `0xF1` | same block | Yes |
| `0x693` | Copies mirror to live MCV Repacks and sends `0xF1` | same block | Yes |
| `0x696` | Copies mirror to live Crates Appear and sends `0xF1` | same block | Yes |

`FUN_006ACEE0` applies values only on Start/Back accept path:

- `0x529` read with `0x400`; stored speed becomes `6 - returned_pos`.
- `0x511` and `0x50C` read with `0x400`.
- Checkbox reads use `0xF0` and compare result exactly to `1`; any other value becomes false.
- Mirror globals are updated after live globals.
- Checkbox click notifications do not directly update these option globals; the accept path rereads the child controls.

## 4. INI Keys

The paint geometry is not INI-driven. Initial values and ranges are upstreamed through `[Skirmish]` persisted settings and `[MultiplayerDialogSettings]` Rules fallback.

| UI control | `[Skirmish]` key | Rules fallback / source | YR default evidence | Effect in this slice |
|---:|---|---|---|---|
| `0x529` | `GameSpeed` | Rules `+0x14A0` | `rulesmd.ini:3026` = `GameSpeed=1` | Initial value, visually inverted as `6 - stored` |
| `0x511` | `Credits` | Rules `+0x1484`; range `+0x1480/+0x1488`; step `+0x148C` | `rulesmd.ini:3018..3021` = `5000/10000/10000/100` | Trackbar range/value/step |
| `0x50C` | `UnitCount` | Rules `+0x1494`; range `+0x1490/+0x1498` | `rulesmd.ini:3022..3024` = `0/10/10` | Trackbar range/value |
| `0x54E` | `ShortGame` | Rules `+0x14B6` | `rulesmd.ini:3039` = `yes` | Initial checkbox state |
| `0x69A` | `SuperWeaponsAllowed` | Rules `+0x14B9`; constructor fallback if absent | no supplied `rulesmd.ini` key; constructor seeds true in prior verified report | Initial checkbox state |
| `0x69D` | `BuildOffAlly` | Rules `+0x14BA`; constructor fallback if absent | no supplied `rulesmd.ini` key; constructor seeds true in prior verified report | Initial checkbox state |
| `0x693` | `MCVRepacks` | Rules `+0x14B8`, fallback key `MCVRedeploys` | `rulesmd.ini:3041` = `yes` | Initial checkbox state |
| `0x696` | `CratesAppear` | Rules `+0x14B1`, fallback key `Crates` | `rulesmd.ini:3034` = `yes` | Initial checkbox state |

Base RA2 contrast: `rules.ini:2506` has `GameSpeed=0`; YR `rulesmd.ini:3026` overrides it to `1`. YR `*md` priority applies.

## 5. Integration Points

| Integration point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006AE2C0` | Offline Skirmish launcher; creates/runs dialog and returns true only on Start `0x617` | decompile `0x006AE2C0` | Yes |
| `FUN_006AE3F0` | Skirmish dialog proc; delegates common shell proc, routes `0x497`, `WM_PAINT`, and `WM_COMMAND` | decompile `0x006AE3F0` | Yes |
| `FUN_00622B50` | Common shell dialog proc that drives owner-draw enumeration through `FUN_0060F9A0` | prior common parent paint reports | Yes |
| `FUN_0060F9A0` | Owner-draw hook/class/style router and initial `0x497` sender | decompile `0x0060F9A0` | Yes |
| `FUN_0061F210` | One-time owner-draw PCX preload pool; preloads live and dead/unproven assets | decompile `0x0061F210` | Yes |
| `FUN_006BA140` | Finds converted PCX/surface by name; callbacks assume success for scoped checkbox/trackbar pieces | prior owner-draw follow-up and string xrefs | Yes |
| `FUN_006BA3E0` | Tiles plaque middle `trofm.pcx` | `0x0061DE9C..0x0061DF04` | Yes |
| `FUN_006208F0` | Draws primitive beveled rectangles for the trackbar rail/base | `0x0061E1F3..0x0061E269`, decompile `0x006208F0` | Yes |
| `FUN_00621040` | Draws checkbox labels and trackbar numeric text with caller-provided rect/flags/color | `0x00616674`, `0x0061E30A` | Yes |

Tick-cycle note: this is shell UI event-loop behavior, not deterministic match-tick simulation. It runs during the modal shell pump before game launch, then `FUN_006ACEE0` packs accepted control values into session globals used later by match setup.

## 6. Current Rust Implementation Status

Rust has partial layout knowledge but does not yet implement the scoped owner-draw controls as player-visible shell widgets.

| Area | Current Rust status | Evidence |
|---|---|---|
| Trackbar layout rects | Present in `SkirmishShellLayout.trackbars` and tested. Game speed and credits match `[404,286,128,21]` / `[404,314,128,21]`; unit count currently uses `[404,340,128,21]` via a one-pixel fixup. | `src/ui/skirmish_shell/layout.rs:67`, `src/ui/skirmish_shell/layout.rs:194`, `src/ui/skirmish_shell/layout.rs:229`, `src/ui/skirmish_shell/layout.rs:286` |
| Checkbox layout/control IDs | Missing from `ShellControlId` and `SkirmishShellLayout`; no checkbox rect collection is exposed. | `src/ui/skirmish_shell/layout.rs:28`, `src/ui/skirmish_shell/layout.rs:74` |
| Trackbar rendering | Missing. Render path builds background/chrome/buttons/flags/preview pieces, but no trackbar draw roles or `trakgrip/trof*` rendering exist. | `src/app_skirmish_shell_render.rs:68`, `src/app_skirmish_shell_render.rs:498`, `src/app_skirmish_shell_render.rs:542` |
| Checkbox rendering | Missing. No `cue_i.pcx` / `cce_i.pcx` draw path, label rect, disabled alpha, or icon-only hit gate is implemented in the experimental shell. | `rg` over `src/ui`, `src/render`, `src/app_skirmish_shell_render.rs`; `src/ui/skirmish_shell/state.rs:121` |
| Shell state | Only `starting_credits` and `short_game` are carried into experimental shell state; no game speed, unit count, super weapons, build off ally, crates, or MCV repack state in this shell state. | `src/ui/skirmish_shell/state.rs:35`, `src/ui/skirmish_shell/state.rs:70` |
| Hit testing | Buttons and color combos only. Checkbox icon-only toggle and trackbar drag/click are absent. | `src/ui/skirmish_shell/state.rs:121` |
| Launch settings bridge | Only selected map/countries/credits/start position/short game/zoom are returned. | `src/ui/skirmish_shell/state.rs:70` |
| Sim option defaults | `GameOptions` has fields for the relevant gameplay options. As of current Rust, `build_off_ally` defaults true, matching prior verified binary evidence that the YR constructor fallback seeds Build Off Ally true unless overridden. | `src/sim/game_options.rs`, `SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md` |
| Asset atlas | Current render roles include owner-draw buttons, flags, preview, markers, and labels, but no checkbox or trackbar asset roles. | `src/app_skirmish_shell_render.rs:68` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish `0x102` reachability | verified | `FUN_006AE2C0`, `FUN_006AE3F0` | none |
| Owner-draw hook routing | verified | `FUN_0060F9A0` | none |
| Checkbox callback paint/input path | verified | `OwnerDraw_Checkbox_006163A0` | final screenshot validation optional |
| Checkbox default PCX selection | verified | strings `0x00835968`, `0x00835974`, `0x00835998`; branch `0x006164E1..0x006165C8` | none |
| Checkbox variant PCX selection | verified | strings `0x00835980`, `0x0083598C`, `0x00835998`; messages `0x4E5/0x4E6/0x4E7` | standard `0x102` does not exercise it |
| Checkbox disabled path | verified | `WS_DISABLED` checks around `0x00616619..0x00616655` | runtime scenario where standard options become disabled was not observed |
| Trackbar callback paint/input path | verified | `OwnerDraw_Trackbar_0061D950` | final screenshot validation optional |
| Trackbar plaque/thumb asset names | verified | strings `0x00835A00`, `0x00835A10`, `0x00835A1C`, `0x00835A28`; paint branch | none |
| Trackbar primitive rail calls | verified code path, not screenshot-exhausted | `0x0061E1F3..0x0061E269`, `FUN_006208F0` | retail surface pixel capture for exact RGB/bevel appearance |
| Trackbar value/range/mouse mapping | verified | `0x0061DA52..0x0061E609` | none |
| Skirmish init for scoped controls | verified | `FUN_006AE6E0` | none |
| Start/Back apply packing | verified | `FUN_006ACEE0` | post-launch consumers out of scope |
| INI/rules fallback values | verified from prior reports + local INI | `rulesmd.ini:3017..3041`, `rules.ini:2497..2521`, prior Ghidra settings reports | physical user `RA2MD.INI` values out of scope |
| `BTN-MINS.SHP` / `BTN-PLUS.SHP` for scoped trackbars | verified-negative | no xrefs; `OwnerDraw_Trackbar_0061D950` uses PCX/primitive path | broad legacy use-site hunt out of scope |
| `bst_*` for scoped checkboxes | verified-negative for this callback | `FUN_0061F210` preload only; `OwnerDraw_Checkbox_006163A0` does not use them | broad hidden-use audit out of scope |
| Current Rust checkbox/trackbar state/render/hit status | verified by source scan | `src/ui/skirmish_shell/*`, `src/app_skirmish_shell_render.rs`, `src/sim/game_options.rs` | implementation work is out of scope for this research task |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which functions enter the standard offline Skirmish `0x102` shell? -> `FUN_006AE2C0` creates/runs the dialog and `FUN_006AE3F0` handles its proc after common shell delegation.` (evidence: `0x006AE2C0`, `0x006AE3F0`)
- `[RESOLVED] OQ-02 - Which function subclasses the controls? -> `FUN_0060F9A0` installs `0x00610CA0`, stores callback/previous proc records, and sends `0x497`.` (evidence: `FUN_0060F9A0`)
- `[RESOLVED] OQ-03 - Which callback paints Button style checkbox controls? -> Button style low bits `0x03` route to `OwnerDraw_Checkbox_006163A0` after earlier button-style tests fail.` (evidence: `FUN_0060F9A0`)
- `[RESOLVED] OQ-04 - Which callback paints `msctls_trackbar32` controls? -> `OwnerDraw_Trackbar_0061D950`.` (evidence: `FUN_0060F9A0`)
- `[RESOLVED] OQ-05 - Are these paths active in YR? -> Yes, the standard offline Skirmish dialog initializes and applies all scoped controls.` (evidence: `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`)
- `[RESOLVED] OQ-06 - What checkbox PCXs are live for standard `0x102`? -> `cue_i.pcx` unchecked and `cce_i.pcx` checked; variants are not initialized by standard Skirmish.` (evidence: `0x006164E1..0x006165C8`, `FUN_006AE6E0`)
- `[RESOLVED] OQ-07 - Are `bst_*` checkbox assets used here? -> No; they are preloaded but not referenced by the scoped checkbox callback.` (evidence: `FUN_0061F210`, `OwnerDraw_Checkbox_006163A0`)
- `[RESOLVED] OQ-08 - Where is checkbox label text drawn? -> Control rect with left advanced by `0x1A`; flags `0x04`, v-centered and left anchored.` (evidence: `0x0061663E..0x00616674`)
- `[RESOLVED] OQ-09 - What toggles a checkbox? -> Only `x < 0x12 && y < 0x12` on left click or double click; label clicks do not toggle.` (evidence: `0x006166EE..0x00616708`)
- `[RESOLVED] OQ-10 - What happens on checkbox disabled paint? -> Icon receives alpha overlay using `DAT_00AC4898`; text color switches to `DAT_00AC1CB4`.` (evidence: `0x00616619..0x00616655`)
- `[RESOLVED] OQ-11 - What trackbar assets are used? -> `trakgrip.pcx`, `trofl.pcx`, `trofm.pcx`, `trofr.pcx`.` (evidence: `0x0061DE9C..0x0061E0AD`, strings `0x00835A00..0x00835A28`)
- `[RESOLVED] OQ-12 - Are `BTN-MINS.SHP` / `BTN-PLUS.SHP` used by these trackbars? -> No; scoped path uses PCXs and primitive bevel drawing.` (evidence: `OwnerDraw_Trackbar_0061D950`; no xrefs for `BTN-MINS.SHP`)
- `[RESOLVED] OQ-13 - How wide is the standard active track? -> `128 - 50 - 13 = 65` px after default step/display normalization.` (evidence: `0x0061DA52..0x0061DBAD`)
- `[RESOLVED] OQ-14 - How does mouse x map to a value? -> Clamp `(mouse_x - 6)` to `[1, right - plaque_width - 0x0C]`, compute `((x - 1) * (span + 1)) / active_width`, saturate at span, then quantize by step.` (evidence: `0x0061E545..0x0061E594`)
- `[RESOLVED] OQ-15 - Is every click height accepted by the trackbar? -> No; slider logic requires `mouse_y > client_bottom - 0x12`, so the top 4 px of a 21 px control do not start interaction.` (evidence: `0x0061E4F5..0x0061E512`)
- `[RESOLVED] OQ-16 - What starts thumb dragging? -> Mouse down inside `[thumb_x, thumb_x + 12)` sets the thumb-drag flag instead of immediately remapping the value.` (evidence: `0x0061E518..0x0061E540`)
- `[RESOLVED] OQ-17 - How are trackbar values initialized and applied? -> `0x529` uses visual `6 - stored`; credits and unit count use direct values; Start/Back rereads `0x400` and mirrors globals.` (evidence: `FUN_006AE6E0`, `FUN_006ACEE0`)
- `[RESOLVED] OQ-18 - Which INI/rules keys matter for initial values? -> `GameSpeed`, `Credits`, `UnitCount`, `ShortGame`, `SuperWeaponsAllowed`, `BuildOffAlly`, `MCVRepacks`, and `CratesAppear`, with Rules fallback values as listed in section 4.` (evidence: `rulesmd.ini:3017..3041`, prior `SessionClass__ReadSkirmishSettings` report)
- `[RESOLVED] OQ-19 - Are these controls match-tick systems? -> No; they run in the shell event loop before game launch and pack state on accept.` (evidence: `FUN_006AE2C0`, `FUN_006ACEE0`)
- `[RESOLVED] OQ-20 - What current Rust surfaces know about checkbox/trackbar placement? -> Current Rust now represents the five checkbox rects and three trackbar rects, including first-four checkbox x `71`, BuildOffAlly x `302`, and unit-count y `340`.` (evidence: `SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`; current source scan)
- `[RESOLVED] OQ-21 - Does Rust currently render checkbox or trackbar owner-draw widgets? -> Yes. Current Rust renders `cue_i/cce_i` checkbox icons and labels, trackbar rail/plaque/thumb/value text, and implements scoped input gates.` (evidence: `SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`; current source scan)
- `[RESOLVED] OQ-22 - Is there a current Rust option default mismatch relevant to this slice? -> No current mismatch observed for BuildOffAlly fallback: `GameOptions::default().build_off_ally` is now true, matching verified YR fallback unless overridden.` (evidence: `src/sim/game_options.rs`; prior `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-23 - What are the exact final retail RGB pixels of the primitive trackbar rail bevel?` (category: `needs-runtime-debugger`; reason: code path and color globals are verified, but exact final 16-bit surface appearance should be screenshot/pixel captured; next-step-if-pursued: capture a retail offline Skirmish frame and sample the three trackbar rails)
- `[DEFERRED] OQ-24 - Which non-Skirmish shell, if any, uses `bst_*`, `BTN-MINS.SHP`, or `BTN-PLUS.SHP`?` (category: `out-of-scope`; reason: this report only claims standard offline Skirmish `0x102` checkbox/trackbar controls; next-step-if-pursued: whole-shell static-string/use-site inventory)
- `[DEFERRED] OQ-25 - Which exact runtime flow disables any of the five standard option checkboxes or three sliders?` (category: `requires-different-system-context`; reason: disabled paint branches are verified, but standard init does not disable these controls in the scoped path; next-step-if-pursued: trace all `EnableWindow` calls for IDs `0x54E/0x693/0x696/0x69A/0x69D/0x529/0x511/0x50C`)

The deferred set is intentionally narrow: it does not block implementation of standard enabled-state geometry, assets, hit gates, or value mapping.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Checkbox controls are separate owner-draw widgets with fixed 18x18 icon and label rect left+26 | `0x0061649D..0x00616674` | missing | `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Represent five checkbox rects/IDs; render icon and label as separate pieces; label is v-centered/left anchored | In offline Skirmish, Short Game/MCV Repacks/Crates/Super Weapons/Build Off Ally labels align beside 18x18 icons at verified positions | Do not make a generic egui checkbox or scale the whole control |
| Checkbox toggles only within the 18x18 icon gate, not on label text | `0x006166EE..0x00616708` | missing | `src/ui/skirmish_shell/state.rs` hit testing | Implement icon-only checkbox hit test and preserve owner state until launch/apply | Clicking text beside Short Game does nothing; clicking the icon toggles and plays shell click feedback when audio is wired | Do not treat the full label/control rect as the toggle target |
| Standard checkbox art is `cue_i.pcx` unchecked and `cce_i.pcx` checked | `0x006164E1..0x006165C8`; `FUN_006AE6E0` sends no variant messages | missing | shell chrome/PCX atlas loader and Skirmish render pass | Load/render the two default PCXs from retail assets; keep variant PCXs available only if variant messages are later implemented | Toggling any standard option switches between the two 18x18 icons | Do not use `bst_*` assets for these Skirmish checkboxes |
| Trackbars reserve a 50 px value plaque, subtract 13 px more, and use 65 px active track width in `128x21` controls | `0x0061DA52..0x0061DBAD` | layout only; render/input missing | `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Render plaque/rail/thumb using binary geometry and map values through the verified formulas | Drag Game Speed/Credits/Unit Count: thumb movement and numeric values step at the same positions as retail | Do not use a normalized 0.0-1.0 slider stretched across the full 128 px width |
| Trackbar mouse y gate excludes top pixels; thumb hit starts dragging, outside hit remaps value | `0x0061E4F5..0x0061E540` | missing | `src/ui/skirmish_shell/state.rs` input path | Add trackbar-specific hit/drag state with vertical gate and 12 px thumb interval | Clicking top edge of the slider does not move it; clicking below the thumb rail changes or drags according to retail | Do not make the whole control rect active |
| Trackbar assets are `trakgrip.pcx` and `trofl/trofm/trofr.pcx`; rail is primitive bevel via `FUN_006208F0` | `0x0061DE9C..0x0061E30A`, `FUN_006208F0` | missing | shell asset atlas/render primitives | Decode/render the PCX pieces and a primitive bevel rail; numeric text rect is `[right-49, top, right, bottom]` flags `0x05` | The value plaque appears at the right, the thumb sits on the left rail, and numeric value is centered in the plaque | Do not use `BTN-MINS.SHP` or `BTN-PLUS.SHP` |
| Game Speed slider is visually inverted: init sends `6 - stored`, apply stores `6 - TBM_GETPOS` | `FUN_006AE6E0`, `FUN_006ACEE0` | shell state missing | `src/ui/skirmish_shell/state.rs`, launch settings / game options bridge | Store both UI position or convert on boundaries; keep launch value equal to YR stored speed | Moving the Game Speed UI to visual position 5 stores speed code 1, matching YR | Do not display/apply GameSpeed as a direct left-to-right stored value |
| Credits range is Rules min/max with step `MoneyIncrement`; Unit Count range is Rules min/max step 1 | `FUN_006AE6E0`, `rulesmd.ini:3018..3024` | credits value exists, range/trackbar behavior missing; unit count missing in shell state | `src/ui/skirmish_shell/state.rs`, settings bridge | Bind sliders to Rules/YR defaults and quantize by step | Credits snaps by 100 from 5000 to 10000; Unit Count snaps 0..10 | Do not hardcode ranges in render-only code when Rules data is available |
| Start/Back apply rereads live controls and mirrors values; checkbox click does not immediately write option globals | `FUN_006ACEE0` | launch settings only preserves `starting_credits` and `short_game` | `src/ui/skirmish_shell/state.rs`, `src/ui/main_menu.rs`, sim game options bridge | Keep shell control state as source of truth until Start, then pack all relevant options into launch settings/game options | Toggle Super Weapons, then start; sidebar/superweapon availability follows packed value | Do not mutate gameplay state on every shell click |
| Build Off Ally default should be true in standard YR fallback unless overridden | prior binary constructor evidence; `rulesmd.ini` lacks override | mismatch | `src/sim/game_options.rs:55` and upstream option loading | Align default/fallback behavior with verified YR source chain | New skirmish with no persisted override starts with Build Off Ally checked/enabled | Do not leave false as the YR default |

## Stale Docs / Follow-up Docs

- This report supersedes any implementation-facing shorthand that treats Skirmish checkbox/trackbar visuals as generic UI widgets. They are owner-draw controls with fixed icon/plaque/thumb geometry and callback-specific input gates.
- Existing `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md` remains broadly valid. This report is the canonical owner-draw paint geometry handoff because it consolidates subclass routing, state offsets, disabled/variant behavior, Rust status, and deferred runtime-capture questions.

## Sources

- Ghidra decompiled/read-only:
  - `FUN_006AE2C0`
  - `FUN_006AE3F0`
  - `FUN_006AE6E0`
  - `FUN_006ACEE0`
  - `FUN_0060F9A0`
  - `OwnerDraw_Checkbox_006163A0`
  - `OwnerDraw_Trackbar_0061D950`
  - `FUN_0061F210`
  - `FUN_006208F0`
  - `FUN_00621040`
- Ghidra assembly contexts:
  - checkbox label/click: `0x0061663E`, `0x00616674`, `0x006166EE`, `0x0061670E`
  - trackbar geometry/paint/input: `0x0061DA52`, `0x0061DE9C`, `0x0061E00C`, `0x0061E0B0`, `0x0061E1F3`, `0x0061E2D9`, `0x0061E30A`, `0x0061E486`, `0x0061E4F5`, `0x0061E518`, `0x0061E545`, `0x0061E59A`
- Ghidra strings:
  - `cue_i.pcx @ 0x00835974`
  - `cce_ir.pcx @ 0x00835980`
  - `cce_il.pcx @ 0x0083598C`
  - `cce_i.pcx @ 0x00835998`
  - `trakgrip.pcx @ 0x00835A00`
  - `trofr.pcx @ 0x00835A10`
  - `trofl.pcx @ 0x00835A1C`
  - `trofm.pcx @ 0x00835A28`
  - `bst_uckg/chkg/uchk/chkd.pcx @ 0x00835E5C..0x00835E8C`
  - `BTN-MINS.SHP @ 0x0083FDB8`, `BTN-PLUS.SHP @ 0x0083FDC8`
- Prior research checked:
  - `docs/research/skirmish-ui/SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`
- INI checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
- Rust scanned:
  - `src/ui/skirmish_shell/layout.rs`
  - `src/ui/skirmish_shell/state.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/sim/game_options.rs`
  - `src/ui/main_menu.rs`
