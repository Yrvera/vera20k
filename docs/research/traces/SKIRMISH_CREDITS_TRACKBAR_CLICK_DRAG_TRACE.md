# Skirmish Credits Trackbar Click/Drag Trace

**Scenario:** Native/dev Skirmish shell dialog `0x102`, 800x600. Credits trackbar `0x511` starts at the retail default `10000`. Mouse down on the current thumb at `(470,318)`, drag to `(444,318)`, release.

**Verdict:** PARTIAL. The core hit gate, thumb gate, quantization, numeric value, and final thumb coordinate match the verified gamemd formulas for this concrete drag. Rust diverges on the changed-value notification/sound path and on the packed text color used for the displayed value. Exact final rail/plaque/PCX pixels remain unchecked without a retail screenshot/pixel capture.

**Tally:** PASS: 8 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Sources

- Verified gamemd research: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`
- Verified gamemd research: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
- Verified gamemd research: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`
- INI values: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`

## Active YR Confirmation

The scoped gamemd path is active in standard Yuri's Revenge Skirmish. The verified docs tie dialog `0x102` creation through `FUN_006AE2C0`/`FUN_006AE3F0`, owner-draw callback routing through `FUN_0060F9A0`, credits trackbar initialization through `FUN_006AE6E0`, and credits application through `FUN_006ACEE0`. The live control is `msctls_trackbar32` id `0x511`, routed to `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, not the dormant/non-Skirmish `SliderClass` plus/minus SHP path.

## Pipeline

Mouse down on current credits thumb -> trackbar y gate -> thumb hit gate -> drag state/capture -> mouse move value mapping -> quantized state update -> invalidate/notify/sound -> render rail/plaque/thumb/value text -> mouse up clears drag state.

## Stage Results

| Stage | gamemd result | Rust result | Verdict |
|---|---|---|---|
| Active control path | Dialog `0x102`, control `0x511`, callback `OwnerDraw_Trackbar_0061D950`; active in standard YR. | Native/dev shell has `SkirmishTrackbarId::Credits0x511` and routes mouse events through `handle_option_mouse_down/move/up`. | PASS |
| Retail defaults | `rulesmd.ini` has `MinMoney=5000`, `Money=10000`, `MaxMoney=10000`, `MoneyIncrement=100`; gamemd sends this range/step to `0x511`. | `CREDITS_MIN=5000`, `CREDITS_MAX=10000`, `CREDITS_STEP=100`; default `starting_credits=10000`. | PASS |
| 800x600 rect | Credits trackbar rect is `[404,314,128,21]`; active width `128 - 50 - 13 = 65`. | `compute_layout(800,600).trackbars.credits` is `[404,314,128,21]`; `trackbar_active_width` returns `65`. | PASS |
| Allowed y band | Input logic runs only when local y is `> 3` and `< 21`; `(470,318)` and `(444,318)` use local y `4`. | `trackbar_mouse_allowed_y` requires `mouse_y > rect.y + rect.h - 18 && mouse_y < rect.y + rect.h`, so y `318` is accepted. | PASS |
| Initial thumb hit | At `10000`, pixel offset is `(10000-5000)*65/5000 = 65`; thumb x interval is `[470,482)`. Down at `(470,318)` starts thumb dragging instead of remapping. | `trackbar_pixel_offset` gives `65`; `trackbar_thumb_rect` is `[470,314,12,21]`; `trackbar_thumb_hit` is true and `trackbar_drag` becomes `Credits0x511`. | PASS |
| Drag value mapping | Move x `444`: local x after bias is `444-404-6 = 34`, clamp `[1,66]`; raw `((34-1)*(5000+1))/65 = 2538`; quantized absolute value `((5000+2538)/100)*100 = 7500`. | `trackbar_mouse_value` computes `5000 + (2538/100)*100 = 7500`; `set_trackbar_visual_value` writes `starting_credits=7500`. | PASS |
| Release state | Mouse up ends capture/drag state after the changed value has been committed. | `handle_option_mouse_up` clears `trackbar_drag` and `dropdown_scroll_drag`. | PASS |
| Displayed value and thumb position | After `7500`, pixel offset is `(7500-5000)*65/5000 = 32`; thumb x is `404+1+32 = 437`; numeric text is `"7500"` in `[483,314,49,21]`. | Render computes the same offset and thumb rect, and `trackbar_display_value` returns `"7500"`; `trackbar_value_text_rect` is `[483,314,49,21]`. | PASS |
| Changed-value notification | On changed value, gamemd invalidates and sends parent `WM_HSCROLL 0x114` with low word `5` and high word current absolute value. | Rust mutates shell state directly and returns `SkirmishShellAction::None`; there is no parent notification/action equivalent in `handle_option_mouse_move` or `app.rs`. | FAIL |
| Changed-value sound | When the value changes and suppression byte is zero, gamemd plays `VocClass__PlayAtPos(1.0, 0)` from the trackbar final branch. | The Skirmish shell trackbar mouse path does not enqueue or play any UI sound. `app.rs` has main-menu button sound plumbing, but no trackbar sound call on shell value changes. | FAIL |
| Value text color | Trackbar numeric text uses the shell text wrapper with caller packed color `0x00000C05`, verified to decode as RGB `(5,12,0)`. | `SHELL_BUTTON_TEXT_RGB_00000C05` is `[0,12,5]`, and trackbar value text uses that constant. The displayed value is visibly shifted in color. | FAIL |
| Exact final pixels | gamemd uses `trofm/trofl/trofr.pcx`, `trakgrip.pcx`, and primitive bevel calls; exact final 16-bit rail/plaque raster needs screenshot validation. | Rust loads/renders the same named PCX pieces and a generated primitive rail, but no retail screenshot/pixel comparison was run in this trace. | UNCHECKED |

## Failures

### 1. Missing changed-value notification

**Stage:** Changed-value notification  
**Player-visible difference:** The native trackbar has a value-changed message boundary on every changed drag update. Rust updates local state directly and exposes no equivalent action/notification. This may not show as a separate visual for the current credits label, but it is the ordering boundary that gamemd uses before sound and parent processing.  
**Rust:** `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:388`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs:650`  
**gamemd evidence:** `OwnerDraw_Trackbar_0061D950` final branch at `0x0061E609`; verified report line says parent `WM_HSCROLL 0x114`, low word `5`, high word current absolute value.

### 2. Missing trackbar click/drag sound

**Stage:** Changed-value sound  
**Player-visible difference:** Retail plays a UI click when the drag changes credits; Rust is silent.  
**Rust:** `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:388`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs:592`  
**gamemd evidence:** `OwnerDraw_Trackbar_0061D950` final branch plays `VocClass__PlayAtPos(1.0, 0)` when value changed, sound suppression is zero, and the branch permits sound.

### 3. Trackbar value text color is channel-swapped

**Stage:** Displayed value render  
**Player-visible difference:** The numeric credits value is the right text and position but the wrong dark color: Rust uses RGB `(0,12,5)` where the verified packed shell color decodes to `(5,12,0)`.  
**Rust:** `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:45`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:1547`  
**gamemd evidence:** `FUN_00621040` shell text wrapper decodes packed caller color `0x00000C05` as RGB `(5,12,0)`; active for trackbar numeric text via `OwnerDraw_Trackbar_0061D950`.

## Adjacent Findings

- Rust hardcodes credits min/max/step in `state.rs` and render matching arms. The concrete retail numbers match this trace, but gamemd sources them from Rules fields. This should become data-driven before accepting modded rules parity.
- Existing older docs that say Rust has no checkbox/trackbar rendering are stale for current source. The current code has trackbar layout, input, PCX atlas entries, primitive rail generation, thumb/plaque rendering, and value text rendering.
- Exact primitive bevel/palette final pixels remain a screenshot-validation task, not a PASS from source inspection.
