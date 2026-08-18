# Skirmish Validation Modal Visual Proof Re-swarm Reconciliation

Date: 2026-05-24

Scope: reconcile the five read-only re-swarm reports for the Skirmish Start validation modal, especially the `PUDLGBG*` background family, `MNBTTN.SHP + MAINBTTN.PAL` OK-button art, text layout, and current Rust deltas after the user-requested Soviet-theme override.

## Source Reports

- `SKIRMISH_VALIDATION_MODAL_NATIVE_PIXEL_RECTS_GHIDRA_REPORT.md`
- `PUDLGBG_MODE2_DIALOG_INVENTORY_GHIDRA_REPORT.md`
- `VALIDATION_MODAL_OK_BUTTON_STATE_FRAMES_GHIDRA_REPORT.md`
- `VALIDATION_MODAL_TEXT_LAYOUT_AND_WRAPPING_GHIDRA_REPORT.md`
- `CURRENT_RUST_VALIDATION_MODAL_VISUAL_DELTA_AFTER_SOVIET_OVERRIDE_REPORT.md`

## Settled Findings

The Skirmish Start validation modal is a real live path, not speculative UI art. The Start command failure path reaches the ordinary validation dialog through RT_DIALOG `0xCE`; its body static is control `0x5B0`, and its OK button is control `0x5AE`.

For ordinary shell/no-game validation, native Yuri's Revenge paints the mode-2 dialog background as:

- `PUDLGBGN.SHP + DIALOGN.PAL`

The Soviet background is also real, but it is not native for the ordinary offline shell validation popup. It is selected by the broader mode-2 paint path when the active in-game side resolves to side `1`:

- side `0`: `PUDLGBGA.SHP + DIALOG.PAL`
- side `1`: `PUDLGBGS.SHP + DIALOG.PAL`
- other in-game side: `PUDLGBGY.SHP + DIALOGY.PAL`

Therefore the current Rust use of `PUDLGBGS.SHP + DIALOG.PAL` for the validation modal is a deliberate user-requested Soviet style override. It should not be documented as native parity for the ordinary shell validation popup.

The mode-2 dialog allow-list is broader than this one validation popup:

`0x10D, 0xD9, 0xF0, 0xCE, 0x120, 0x121, 0x115, 0xD3, 0xCF, 0x11F, 0xC3, 0x11B, 0xE1, 0x11E, 0xC4, 0x130, 0xD0, 0xFC, 0x126`

Per-dialog gameplay reachability for every listed id was intentionally not proven by this swarm. Treat the list as paint-path capability, not as a menu-flow inventory.

## OK Button Art

`MNBTTN.SHP + MAINBTTN.PAL` is the correct live owner-draw art for the validation modal OK button.

The corrected frame mapping is:

- frame `0`: normal
- frame `1`: mouse-down / pressed / armed state
- frame `2`: timer/default-highlight byte path

This corrects earlier wording that treated frame `2` as the ordinary pressed frame. For the validation OK button, current Rust's `pressed -> frame 2` mapping is a parity mismatch. It should use frame `1` for ordinary mouse press. Frame `2` should be reserved unless the native timer/default-highlight activation is proven for this exact OK button state.

The OK label is drawn after the button art. Native owner-draw centers the OK label, and the pressed label position receives a small inset of approximately `left + 2`, `top + 5`.

## Body Text

The body static `0x5B0` is owner-drawn with `GAME.FNT`. It is left/top anchored, wrapped, and clipped inside the resource static rect. It is not horizontally or vertically centered.

Current Rust still centers the body text with H-center and V-center behavior. That is a visible layout mismatch and should be fixed before further cosmetic work.

## Pixel Rect Status

The resource DLU facts are verified:

- dialog template: `300 x 200` DLUs
- body static `0x5B0`: `40, 40, 220, 50` DLUs
- OK button `0x5AE`: `207, 175, 83, 15` DLUs

Native centering is also verified: the child dialog is centered against the current screen dimensions through the standard RA2/YR child-centering path, using a `+1` before division and clamping to zero, with size preserved by `SWP_NOSIZE`.

Exact screenshot-grade 800x600 and 1024x768 pixel rects remain runtime-only because `CreateDialogIndirectParamA` converts DLUs using live dialog font metrics. A candidate conversion using `6x13` base units produces an 800x600 dialog around `175,138,450,325`, but that is not final parity proof. Do not infer the final dialog pixel size from the SHP dimensions alone.

Current Rust's `451x326` modal size is asset/layout-driven and plausible, but not screenshot-proven native parity.

## Current Rust Delta List

After the earlier implementation pass, Rust is close in the asset path but still has two concrete behavior/layout mismatches:

1. OK mouse-down art uses `MNBTTN` frame `2`; native ordinary pressed state uses frame `1`.
2. Body text is centered; native body text is left/top anchored, wrapped, and clipped.

There is also one intentional non-parity styling choice:

3. Rust uses the Soviet `PUDLGBGS + DIALOG.PAL` background for this shell validation modal by request. Native ordinary shell validation uses neutral `PUDLGBGN + DIALOGN.PAL`.

Remaining proof gap:

4. Exact native pixel rects require a runtime capture/pixel comparison. The Ghidra path proves resource DLUs and centering behavior, but not final Win32 font-converted pixels.

## Recommended Next Implementation

The highest-value next code change is small:

1. Change `modal_button_mnbttn_frame_index(true)` from frame `2` to frame `1`.
2. Change validation body text rendering from centered text to left/top wrapped/clipped text.
3. Preserve OK button centered label rendering and pressed text inset.

Do not spend implementation time on random maps or broader dialog inventory from this swarm. They are separate systems and not the current player-visible mismatch.

If exact screenshot parity is needed before changing modal size, run a focused native runtime capture for `0xCE` at 800x600 and 1024x768, then compare Rust render output against those captures.

## Negative Claims

- Do not treat `PUDLGBGS + DIALOG.PAL` as native shell validation parity.
- Do not use `MNBTTN` frame `2` for ordinary mouse press.
- Do not center the validation body text.
- Do not infer final modal pixel size from `PUDLGBG*.SHP` dimensions.
- Do not treat the mode-2 allow-list as proof that every listed dialog is reachable in normal Skirmish shell flow.
