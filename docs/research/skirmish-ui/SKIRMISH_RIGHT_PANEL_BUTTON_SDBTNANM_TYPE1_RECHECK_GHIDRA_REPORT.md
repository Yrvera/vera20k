# Skirmish Right-Panel Button SDBTNANM Type-1 Recheck - Ghidra Research Report

Date: 2026-05-24

Scope: standard offline YR Skirmish setup dialog `0x102`, right-panel Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0` button chrome.

Status: COMPLETE for the asset-family correction. This report supersedes prior claims that these three right-panel buttons should be restored to the generic gray `bue_*30.pcx` / `bde_*30.pcx` PCX path.

## Verified Finding

The active shell setup classification path reclassifies the scoped right-panel buttons to owner-draw type `1` before paint. In `OwnerDraw_Button_00612B70`, type `1` draws `g_SDBTNANM_SHP` through the shell button animation path:

- released/default: `SDBTNANM.SHP` frame `2`;
- pressed: `SDBTNANM.SHP` frame `4`;
- hover/timer state: `SDBTNANM.SHP` frame `3` where that state is active;
- enabled label color remains `DAT_00AC18A4 = 0x0000FFFF` yellow.

The generic PCX branch in the same owner-draw callback is real, but it is not the parity source for these three right-panel shell buttons after the `FUN_0060A330` classification step. Older PCX-focused reports followed that generic branch and missed the type field set by the shell setup classifier.

## Rust Consequence

For `Start Game`, `Choose Map`, and `Back` in the Skirmish setup sidebar:

- use `SDBTNANM.SHP` frame `2` for released/default;
- use `SDBTNANM.SHP` frame `4` for pressed;
- keep `SDBTNANM.SHP` frame `3` available for hover/timer state work;
- keep gray PCX button pieces only as fallback/shared assets for other verified type-0 controls;
- do not implement a Skirmish sidebar "fix" that replaces these buttons with `bue_*30.pcx` / `bde_*30.pcx`.

Current Rust guard tests:

- `right_panel_buttons_use_sdbtnanm_type1_frames`;
- `button_label_color_uses_owner_draw_button_yellow_source`.

## Superseded Claims

Treat the following older claim shape as stale for these three right-panel buttons:

> Start/Choose/Back use `bue_*30.pcx` released and `bde_*30.pcx` pressed as the normal Skirmish sidebar parity path.

That claim may still be relevant for separate owner-draw type-0 buttons or modal-specific controls only when their active classifier path proves type `0`. It must not be applied to `0x617`, `0x5AA`, or `0x5C0` in the standard Skirmish setup sidebar without rechecking the classifier.

## Source Ledger

- `FUN_0060A330`: shell owner-draw classification; sets the scoped right-panel button owner-draw type to `1`.
- `OwnerDraw_Button_00612B70`: type `1` branch selects `g_SDBTNANM_SHP`; generic PCX path is a different branch.
- `DAT_00AC18A4`: enabled button label yellow `0x0000FFFF`.
- `src/render/skirmish_shell_chrome.rs`: packs `SDBTNANM.SHP` frames `2`, `3`, `4`, and `10`.
- `src/app_skirmish_shell_render/chrome.rs`: `push_right_panel_button_shp` selects SDBTNANM frame `2` or `4`.
- `src/app_skirmish_shell_render/text.rs`: button labels use shell yellow.
