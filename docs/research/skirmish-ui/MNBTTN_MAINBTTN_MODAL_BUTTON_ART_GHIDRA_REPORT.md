# MNBTTN.SHP + MAINBTTN.PAL Modal Button Art Investigation

Date: 2026-05-24

Scope: focused research on `MNBTTN.SHP` and `MAINBTTN.PAL`, especially whether they are live Yuri's Revenge UI assets and whether the Skirmish Start validation modal should use them.

## Summary

`MNBTTN.SHP` plus `MAINBTTN.PAL` is live owner-draw button art in `gamemd.exe`. It is not a dead or unused asset pair.

For the Skirmish Start validation modal, native dialog `0xCE` contains OK button control `0x5AE`. The binary routes that exact dialog/control pair into owner-draw button type `3`, and owner-draw type `3` selects:

- `MNBTTN.SHP` as the shape art.
- `MAINBTTN.PAL` as the draw palette/conversion resource.
- Button frames `0`, `1`, or `2` depending on the internal button state path.

The current Rust UI does not load these assets and instead draws the validation OK button through the generic `push_button_30` PCX path. That is a real visual parity mismatch for the validation modal and likely for other classic modal buttons that share the same owner-draw type.

## Verified Binary Findings

### Dialog/control allow-list

`gamemd.exe` function `0x00609E20` checks parent dialog IDs and child control IDs for a set of owner-draw controls. The relevant branch is:

- Parent dialog `0xCE`.
- Child control `0x5AE`.
- Return value is true only when the control ID matches `0x5AE`.

This proves the Skirmish Start validation modal's OK button is in the native owner-draw type-3 allow-list.

The same function also accepts many other dialog/control pairs, so this is common dialog button art rather than a one-off Skirmish-only special case. Examples observed in the same allow-list include dialog IDs `0x120`, `0x121`, `0xCF`, `0xC3`, `0xEA`, `0x11B`, `0x11E`, and others.

### Owner-draw type assignment

`gamemd.exe` function `0x0060A330` looks up the per-window/control record and assigns the owner-draw type. In the path where `0x00609E20` returns true, it writes owner-draw type `3`.

For dialog `0xCE` control `0x5AE`, this means the OK button is not supposed to use the generic shell PCX button chrome. It is supposed to use owner-draw type `3`.

### Owner-draw type 3 art selection

`gamemd.exe` function `0x00612B70` is the owner-draw button paint routine. In the owner-draw type `3` branch:

- The SHP pointer is set to `DAT_00B0FACC`.
- The palette/conversion resource is obtained by calling `0x0072B050`.
- `0x0072B050` returns `DAT_00B0FB78`.

Prior pointer/name extraction from the shell asset initialization maps:

- `DAT_00B0FACC` = `MNBTTN.SHP`
- `DAT_00B0FB78` = `MAINBTTN.PAL`

The type-3 branch then calls the shape drawing path with the chosen frame index.

### Frame-state selection

In the owner-draw type-3 branch of `0x00612B70`, the binary chooses among frames `0`, `1`, and `2`.

Verified from decompilation:

- Default type-3 state selects frame `0`.
- A record flag bit path selects frame `1`.
- Otherwise, when the button's internal pressed/active byte at the checked record offset is nonzero, the routine selects frame `2`.

The native routine also separately checks the window style for disabled rendering, so disabled appearance may be a combination of the selected frame plus the common disabled draw treatment. The important implementation point is that Rust should preserve the three-frame SHP-backed state selection rather than replacing it with the current PCX button atlas.

## Retail Asset Evidence

The asset pair was decoded from the configured retail RA2/YR install using the existing Rust asset loaders during the earlier validation-modal asset preview pass.

Observed extraction:

- `MNBTTN.SHP` source path: `ra2md.mix -> localmd.mix`
- `MAINBTTN.PAL` source path: `ra2.mix -> local.mix`
- Decoded `MNBTTN.SHP` frames used by the binary:
  - Frame `0`: `126x25`
  - Frame `1`: `126x25`
  - Frame `2`: `126x25`

Generated local previews:

- `target/validation-modal-assets/mnbttn_mainbttn_frame0.png`
- `target/validation-modal-assets/mnbttn_mainbttn_frame1.png`
- `target/validation-modal-assets/mnbttn_mainbttn_frame2.png`
- `target/validation-modal-assets/validation_modal_assets_contact_sheet.png`

These previews are convenience artifacts only; the behavior source of truth remains the binary routing and retail assets.

## Current Rust Mismatch

Current source scan found no `MNBTTN` or `MAINBTTN` usage in Rust.

Relevant current code:

- `src/render/skirmish_shell_chrome.rs`
  - `SkirmishShellChromeAtlas` does not include `MNBTTN.SHP` or `MAINBTTN.PAL`.
- `src/app_skirmish_shell_render/modals.rs`
  - The validation modal OK button is currently pushed with `push_button_30`.
- `src/app_skirmish_shell_render/chrome.rs`
  - `push_button_30` draws the generic 30-pixel PCX button slices.

That means the current validation modal has the right functional dismissal behavior, but its OK button art is not the original YR modal button art.

## Player-Visible Consequence

When the player starts Skirmish without a name, the validation modal should look like a native YR shell modal:

- `PUDLGBGN.SHP` background with `DIALOGN.PAL`.
- Body text from the dialog resource string/control path.
- OK button drawn from `MNBTTN.SHP` with `MAINBTTN.PAL`.

Rust currently draws a generic shell button instead. This is visible immediately in a normal Skirmish setup validation flow.

## Implementation Handoff

Recommended narrow implementation:

1. Add `MNBTTN.SHP` + `MAINBTTN.PAL` to `SkirmishShellChromeAtlas`.
2. Decode at least frames `0`, `1`, and `2` as a dedicated modal button art set.
3. Add a renderer helper for owner-draw type-3 modal buttons.
4. Use that helper for the validation modal OK button instead of `push_button_30`.
5. Preserve the native asset dimensions: `126x25`.
6. Map normal/up visual state to frame `0` and pressed/active state to frame `2`.
7. Treat frame `1` as the binary's alternate record-flag state; verify exact disabled/default/focus interaction with a native screenshot before using it for every disabled button.

Acceptance checks:

- The Skirmish no-name validation modal OK button renders from `MNBTTN.SHP`, not the PCX button atlas.
- The OK button is `126x25` before any placement scaling.
- Pressing/clicking the OK button visibly changes to the native pressed frame.
- Keyboard Enter/Escape dismissal remains unchanged.
- No sim-layer dependency is introduced.

## Deferred / Not Proven In This Pass

This pass did not fully catalog every dialog/control pair that uses owner-draw type `3`; it only confirmed that the list is broader than Skirmish validation and that `0xCE/0x5AE` is definitely included.

This pass also did not capture a fresh native in-game screenshot for disabled/focus/default-button state. The binary state selection is verified, but exact UX mapping for frame `1` should be screenshot-checked before using it broadly beyond the validation OK button.

