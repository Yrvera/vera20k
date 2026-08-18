# Skirmish Side Combo Open/Scroll/Select/Sound Trace

Trace target: standard offline Yuri's Revenge Skirmish dialog `0x102`, 800x600, local player Side combo.

Concrete scenario: open the local player's Side combo, inspect visible row count/dropdown rectangle/scrollbar, click the scrollbar down arrow once, select the newly visible `Great Britain` row, and compare native item order, top-index behavior, state update, close/open sounds, and current Rust.

Status: COMPLETE. Live Ghidra read-only access was attempted for `0x00617250`, `0x0061C690`, `0x004E3A00`, `0x004E4170`, and `0x006AE3F0`, but this session's MCP returned `Function not found`; binary evidence below therefore uses existing verified skirmish-ui Ghidra reports. No Ghidra mutating tools were used.

Verdict tally: PASS: 12 | FAIL: 2 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Pipeline

Trigger: mouse down on local Side combo arrow -> owner-draw combo opens `ComboDropWin` -> popup paints 7 visible country rows plus scrollbar -> scrollbar down arrow changes top index `0 -> 1` silently -> row click selects item index `5` (`Great Britain` / native item data `4`) -> popup closes and close sound plays -> collapsed combo shows the selected country.

## Concrete Values

At `compute_layout(800, 600)`, current Rust places the local Side combo at `RectPx { x: 287, y: 59, w: 117, h: 120 }`.

The concrete open click is `x=403,y=60`, inside the rightmost 20 px arrow reserve.

Native and current Rust side item order for standard stock YR:

`Random, America/Americans, Korea/Alliance, France/French, Germany/Germans, Great Britain/British, Libya/Africans, Iraq/Arabs, Cuba/Confederation, Russia/Russians, Yuri/YuriCountry`.

Opened popup values:

- item count: `11`
- visible rows: `7`
- row height: `23`
- dropdown rect: `x=287,y=84,w=117,h=161`
- content rect: `x=287,y=84,w=97,h=161`
- scrollbar rect: `x=384,y=84,w=20,h=161`
- top index before scroll: `0`
- max top index: `4`
- initial thumb rect: `x=384,y=106,w=20,h=74`

After clicking the scrollbar down arrow at `x=385,y=244`, current Rust sets `top_index=1`; native side dropdown arrow-scroll is verified as a one-row step, so the visible rows become item indices `1..7`.

Selecting row 4 in that scrolled popup, e.g. `x=289,y=177`, selects item index `5`, which is `Great Britain` in Rust and item data `4` / `British` in gamemd. Rust writes `player_country_random=false`, `player_country=GreatBritain`, and closes `open_combo_dropdown=None`.

## Stage Results

| Stage | Verdict | Comparison |
|---|---:|---|
| Active standard YR path | PASS | Verified reports show dialog `0x102` installs `FUN_006AE3F0`, routes commands through `FUN_006ACEE0`, and initializes standard side combo `0x6A1`; active in standard offline YR, not TS legacy. |
| Side item list/order/count | PASS | Native `FUN_004E3A00` inserts Random item data `-2`, then stock country item data `0..9`; Rust `SkirmishCountry::ALL` and `combo_items` produce the same 11-row order at `src/ui/main_menu.rs:54` and `src/ui/skirmish_shell/state.rs:839`. |
| Arrow-open hit test | PASS | Native owner-draw combo toggles from the rightmost `20` px; Rust uses `COMBO_ARROW_RESERVE_W=20` and `combo_arrow_at` at `src/ui/skirmish_shell/layout.rs:23` and `src/ui/skirmish_shell/state.rs:993`. Concrete click `403,60` is inside both. |
| Open sound | PASS | Native plays `[AudioVisual] GUIComboOpenSound` / `MenuACBOpen` on combo mouse down; Rust queues `GuiComboOpenSound` when opening at `src/ui/skirmish_shell/state.rs:1160` and maps it through `src/app.rs:909`. INI default is `rulesmd.ini:650`. |
| Dropdown rectangle and visible rows | PASS | Native side cap is `7`, row height is `23`, popup top is face height `24 + 1`; Rust computes `287,84,117,161` via `combo_dropdown_rect` at `src/ui/skirmish_shell/state.rs:593`. |
| Scrollbar/content geometry | PASS | Native scrollbar width is `20` and row client shrinks by `20`; Rust computes content `w=97` and scrollbar `x=384,w=20` at `src/ui/skirmish_shell/state.rs:669` and `:687`. |
| Thumb geometry at top index 0 | PASS | Native constants are 22 px arrow buttons and 14 px minimum thumb; for `161` high, `7/11` page ratio gives `74` px thumb at `y=106`. Rust matches in `combo_dropdown_scroll_thumb_rect` at `src/ui/skirmish_shell/state.rs:717`. |
| Scroll down one step | PASS | Native scrollbar down arrow changes the dropdown top index by one row and sends scroll notification; Rust down-arrow branch calls `scroll_open_combo_by_rows(..., 1)` at `src/ui/skirmish_shell/state.rs:1089`, producing `top_index 0 -> 1`. |
| Scrollbar sound silence | PASS | Native `OwnerDraw_ScrollBar_0061C690` has no `VocClass__PlayAtPos` call; Rust emits no sound for down-arrow/drag/track scrollbar clicks, covered at `src/ui/skirmish_shell/state.rs:1081` and test `dropdown_scrollbar_arrows_step_and_drag_clamp_top_index`. |
| Scrollbar pressed down-arrow visual state | PASS | Native uses the pressed down-arrow PCX through `FUN_00620720`; Rust records `DropdownScrollbarPart::DownArrow` and renders `scrollbar_arrow_down_pressed` at `src/ui/skirmish_shell/state.rs:1090` and `src/app_skirmish_shell_render.rs:675`. |
| Select newly visible Great Britain row | PASS | With `top_index=1`, visible row 4 maps to item index `5`; Rust writes `GreatBritain` at `src/ui/skirmish_shell/state.rs:1026`, matching native item data `4` for `British` from the final-writes report. |
| Close sound on selection | PASS | Native `ComboDropWin` close/select path plays `GUIComboCloseSound` / `MenuACBClose`; Rust queues `GuiComboCloseSound` on row selection at `src/ui/skirmish_shell/state.rs:1123` and maps it via `src/app.rs:912`. INI default is `rulesmd.ini:651`. |
| Selected popup row fill geometry | PASS | Native fills the full row rect before text/swatch; current Rust now returns full content width and full `23` px row height at `src/app_skirmish_shell_render.rs:1110`. |
| Popup text clipping/truncation | FAIL | Native truncates row text to `client_width - 20`; for content width `97`, the fit limit is `77` px. Rust draws popup labels with rect width `content.w - 3 = 94` at `src/app_skirmish_shell_render.rs:1775`, so long labels such as `Great Britain` can clip/draw differently. |
| Popup primitive colors/border | FAIL | Native derives popup/frame/selected primitive colors from owner-draw globals and display conversion; Rust uses fixed approximate constants at `src/app_skirmish_shell_render.rs:46`. Final pixel delta was not numerically captured, but the implementation source is not the native color contract. |
| Exact Win32 message/notification ordering | UNCHECKED | Native routes through `ComboDropWin`, `CB_SETCURSEL`, parent notifications, and `CB_SHOWDROPDOWN`; Rust mutates state directly during mouse-down handling. Final state/sounds match this scenario, but exact message ordering was not reconstructed numerically. |
| Final glyph pixels after close | UNCHECKED | Rust should display `Great Britain` after close, but exact retail GAME.FNT glyph pixels and final post-close screenshot were not compared. |
| Live binary recheck in this session | UNCHECKED | Ghidra MCP was read-only but did not resolve the target functions in this session. Existing verified reports supply the binary evidence; no new live decompile was captured here. |

## Player-Visible Failures

1. Popup row text can fit/clip differently for longer country names because Rust uses a 94 px draw width where gamemd truncates to 77 px before drawing. Affected Rust: `src/app_skirmish_shell_render.rs:1775`; gamemd evidence: `ComboDropWin` text block `0x0060DE1F..0x0060DFC8` in `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`.
2. Popup background/border/selected colors are hardcoded approximations instead of native owner-draw global/display-converted colors. Affected Rust: `src/app_skirmish_shell_render.rs:46`; gamemd evidence: `OwnerDraw_ComboBox_00617250`, `FUN_006208F0`, and `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`.

No scoped NOT-IMPLEMENTED findings remain for open, scrollbar one-step scroll, select, or open/close sound playback.

## Adjacent Findings

- Rust has direct dropdown mouse-wheel support in `handle_option_mouse_wheel`; verified native reports found no direct `WM_MOUSEWHEEL` handler in the scoped combo/list/scrollbar callbacks. Wheel behavior is adjacent and was not traced in this concrete mouse-only scenario.
- Non-arrow collapsed combo face clicks may differ because native plays the combo-open sound before the rightmost-20-px gate, while this scenario clicks the arrow and does not exercise face-only clicks.
- Color combo row-8 omission, disabled overlay, AI row restrictions, and full Start Game packing are adjacent systems and not traced here.

## Verification Run

Focused tests run successfully:

- `cargo test -q skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index --lib`
- `cargo test -q dropdown_scrollbar_arrows_step_and_drag_clamp_top_index --lib`
- `cargo test -q combo_outside_click_closes_with_close_sound --lib`
- `cargo test -q selecting_random_country_updates_shell_choice_state --lib`

All four passed. The test runs emitted unrelated existing warnings.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_SCROLLBAR_SOUNDS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_SIDE_COMBO_DROPDOWN_OPEN_SELECT_SCROLL_TRACE.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`
