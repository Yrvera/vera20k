# Random Map Generator UI Faithful Restoration Design

## Goal

Restore the stable Random Map Generator setup dialog `0x105` at 800x600 to the
retail-convincing Yuri's Revenge composition shown in the user reference:

- the red `MNSCRNL.SHP` world-map artwork fills the left shell canvas;
- the complete metallic right rail and lower strip frame the screen;
- the title highlight, option controls, preview, and buttons appear above that
  foundation;
- existing random-map generation, saved-seed, preview, and input behavior remain
  unchanged.

This design targets the stable visible screen. It does not claim native pixel
parity or add the original show/close transition animation.

## Architecture Context

The Random Map Generator is the active nested setup dialog represented by
`RandomMapSetupModalState`. App input and generation behavior already route
through the existing random-map setup state and backend. The visible path is:

1. `compute_random_map_setup_layout` supplies the resource-derived control,
   preview, and right-panel rectangles.
2. `build_skirmish_shell_instances` detects the active random-map setup modal,
   calls `push_random_map_setup_modal_instances`, and returns before the normal
   setup composition.
3. `render_skirmish_shell_with_atlas` appends the generated preview and text
   above the modal sprites.

The current failure has three presentation causes:

- `random_map_setup_background_entry` incorrectly selects
  `MnScrnLCustomizeBattle`, the orange Choose Map `0x6B` artwork.
- `push_random_map_setup_modal_instances` then paints an opaque green dialog
  rectangle above that background, covering it.
- the random-map branch returns before the common right rail and lower strip are
  emitted.

Fresh read-only decompilation resolves the background and chrome contract:

- common background selection `0x0060CF00` has no special `0x105` case, so
  `0x105` uses the generic shell background family;
- at 640 width that family is `MNSCRNS.SHP`, and otherwise it is
  `MNSCRNL.SHP`, both converted through `SHELL.PAL`;
- common dialog initialization `0x00622820` includes `0x105` in the
  `data+0xD5` set, which maps to the `SDTP.SHP` frame-1 top-highlight marker;
- `0x105` is not in the `data+0xD6` minimap-button set, so it must not inherit
  setup dialog `0x102`'s `SDMPBTN` layer;
- `0x105` is in the common fullscreen shell set rather than the centered
  mode-2 modal-background set.

The existing geometry/control research for `0x105` remains useful, but its claim
that `0x105` shares the orange `0x6B` background is contradicted by the common
background selector and must be corrected separately.

This work stays in the render, app-render, and shell-UI presentation layers. It
does not change `sim/`, generation algorithms, RNG consumption, modal authority,
map selection, persistence, or file formats.

## Impact Analysis

Primary production surfaces:

- `src/render/skirmish_shell_chrome.rs`
  - Pack `MNSCRNL.SHP` frame 0 through `SHELL.PAL` into the skirmish shell
    atlas.
  - Expose the entry with a role-specific name rather than borrowing the
    main-menu atlas at draw time.
- `src/app_skirmish_shell_render/draw_order.rs`
  - Add an explicit `RandomMapSetup0x105` chrome profile.
  - Add semantic roles and an order contract for the generic RMG background,
    rail, lower strip, top highlight, controls, and preview frame.
- `src/app_skirmish_shell_render/chrome.rs`
  - Reuse the existing common base-rail and lower-strip emitters.
  - Reuse the optional steady-chrome emitter with the new `0x105` profile.
- `src/app_skirmish_shell_render/modals.rs`
  - Replace the Customize Battle background lookup with the generic shell
    background lookup.
  - Make retail-art and primitive fallback compositions mutually exclusive.
  - Stop opaque combo/control backing from hiding the retail artwork while
    retaining readable fallback behavior.
- `src/app_skirmish_shell_render.rs`
  - Compose the `0x105` shell foundation before its child controls instead of
    returning from an isolated modal-only sprite path.

Inspected and expected to remain behaviorally unchanged:

- `src/ui/skirmish_shell/state/random_map_setup.rs`: options, pressed state,
  combo state, enablement, saved seeds, and modal results.
- `src/ui/skirmish_shell/layout.rs`: existing `0x105` geometry and hit
  rectangles.
- `src/app_skirmish_shell_render/preview.rs`: generated preview texture and
  marker presentation.
- `src/app_skirmish_shell_render/text.rs`: labels, selected values, player
  count, button captions, and progress text.
- `src/app.rs` and the RMG backend: generation, surprise-me, load/save/delete,
  accept/cancel, and persistence.
- `src/render/main_menu_shell_chrome.rs`: it already proves the asset/parser
  path for `MNSCRNL.SHP` plus `SHELL.PAL`, but its atlas is not coupled into the
  skirmish renderer.

The main blast-radius risk is regressing the already-restored `0x102` and `0x6B`
shell compositions while extending their shared chrome helpers. The new profile
and semantic draw-order tests must pin all three screens independently.

## Chosen Approach

Extend the existing shared skirmish shell-composition system.

The random-map branch will emit:

1. common right-panel base:
   `SDTP#0`, repeated `SDBTNBKGD#0`, optional repeated `SDBTNANM#10`,
   and source-clipped `SDBTM#0`;
2. the width-selected lower strip, `LWSCRNS#0` or `LWSCRNL#0`;
3. the width-selected generic background:
   `MNSCRNS#0` at 640 or `MNSCRNL#0` otherwise;
4. `0x105` steady optional chrome:
   `SDTP#1`, with no `SDMPBTN`;
5. option controls, right-panel buttons, preview frame, and conditional progress
   controls;
6. generated preview, text, and cursor through the existing outer passes.

If the generic retail background is unavailable, the renderer uses a bounded
primitive fallback for the left content canvas. The fallback must not cover the
right rail or lower strip. Control interiors may be opaque in fallback mode for
readability, while the retail-art path preserves the artwork behind the
owner-draw frames.

This approach is preferred because it follows the verified native division
between common shell chrome, per-dialog background selection, optional flags,
and child controls. It also keeps all three skirmish shell screens on one
composition mechanism without sharing unrelated renderer textures.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` - The current screen is a flat green canvas with floating
  controls and buttons; the retail screen is dominated by the red world-map
  artwork and complete metal frame. The mismatch is visible every time Create
  Random Map opens. [runtime: user captures
  `codex-clipboard-22194799-4655-4053-b6e9-2598beeb9759.png` and
  `codex-clipboard-a1da452c-e0c6-4bce-8d2c-a7ecb8611e96.png`]
- `MILESTONE-BLOCKING` - Dialog `0x105` takes the default branch in
  `0x0060CF00`: `MNSCRNS` at 640, `MNSCRNL` otherwise, through `SHELL.PAL`.
  It does not take the `0x6B` Customize Battle background branch. [binary:
  fresh read-only decompile of `0x0060CF00`] [doc:
  `FUN_0060CF00_DIALOG_BACKGROUND_POINTER_TABLE_GHIDRA_REPORT.md`]
- `MILESTONE-BLOCKING` - Current Rust both selects the wrong orange background
  and covers it with `SHELL_MODAL_BG_RGB`. The retail-art and primitive
  fallback layers must be alternatives. [rust:
  `src/app_skirmish_shell_render/modals.rs`,
  `random_map_setup_background_entry` and
  `push_random_map_setup_modal_instances`]
- `MILESTONE-BLOCKING` - The RMG branch returns before emitting the common rail
  and lower strip. The restored screen needs the same verified base order as the
  other fullscreen shell dialogs. [rust:
  `src/app_skirmish_shell_render.rs`,
  `build_skirmish_shell_instances`] [doc:
  `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`]
- `MILESTONE-BLOCKING` - At 800x600, the common rail occupies the right
  168-pixel column and the lower strip spans the 632-pixel left canvas. The
  `SDBTM` bottom source is clipped rather than vertically stretched. [doc:
  `SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`] [rust:
  `src/ui/shell/geom.rs`]
- `MILESTONE-BLOCKING` - `0x105` enables the `data+0xD5` top-highlight family,
  so `SDTP#1` appears above the background. It does not enable the
  `data+0xD6` minimap-button family and must not draw `SDMPBTN`. [binary:
  fresh read-only decompile of `0x00622820`] [doc:
  `substrate/worknotes/gadget-dialog-20260610/dialog-delta.md`]
- `MILESTONE-BLOCKING` - Existing combo values, player trackbar, action
  buttons, saved-map buttons, Cancel button, progress widgets, and preview must
  remain above the restored foundation and retain their current hit rectangles
  and enablement. [rust: `src/ui/skirmish_shell/layout.rs`;
  `src/ui/skirmish_shell/state/random_map_setup.rs`]
- `MILESTONE-BLOCKING` - Surprise Me, Generate Map, Use Map, Load, Save, Delete,
  and Cancel must preserve their current behavioral contracts. This visual
  restoration cannot change seed generation, accept/cancel boundaries, preview
  promotion, or saved-seed persistence. [rust: `src/app.rs` and RMG modules]
- `COMPOUNDING` - The RMG, Customize Battle, and Skirmish Setup screens share
  common rail helpers but use different backgrounds and optional chrome.
  Profile-specific tests must prevent a correction to one from changing either
  sibling. [rust: `src/app_skirmish_shell_render/chrome.rs` and
  `draw_order.rs`]
- `COMPOUNDING` - The generic background must be packed into the skirmish atlas
  using its own `SHELL.PAL` conversion. Reusing the main-menu render texture at
  runtime would couple independent batches and ownership. [rust:
  `src/render/main_menu_shell_chrome.rs`;
  `src/render/skirmish_shell_chrome.rs`]
- `COMPOUNDING` - Missing-asset fallback must cover only the left content
  canvas and stop above the lower strip. A full-screen fallback would recreate
  the missing-rail defect. [rust: optional atlas-entry contract]
- `COMPOUNDING` - The existing Load/Save/Delete subdialogs are outside this
  restoration. Shared control-paint changes must not silently change their
  background or modal ownership.
- `EXACTIFICATION-RESIDUAL` - The stable screen is in scope, but the native
  show/close slide sequence is not. Trigger: entering or leaving dialog
  `0x105`. Player effect: immediate/current transition instead of the exact
  retail sequence. Downstream risk: bounded presentation only; steady state and
  input authority are unaffected.
- `EXACTIFICATION-RESIDUAL` - Exact output above the native 800-wide reference
  is not established. Preserve current integer anchoring and bounded fallback;
  do not stretch the retail SHP. Trigger: non-640/non-800 shell sizes.
- `EXACTIFICATION-RESIDUAL` - Manual comparison against the supplied screenshot
  can establish a retail-convincing composition verdict, but not native pixel
  parity without an executable same-resolution image oracle.

## Design

### Components

Extend `ShellDialogChromeProfile` with:

- `SkirmishSetup0x102`
- `ChooseMap0x6b`
- `RandomMapSetup0x105`

The profile is render planning only. For `0x105`:

- `draws_top_highlight()` returns true;
- `draws_map_button()` returns false.

Add generic skirmish-shell background entries to
`SkirmishShellChromeAtlas`, conceptually:

- `parent_background_small_mnscrns`
- `parent_background_large_mnscrnl`

The existing setup- and chooser-specific entries remain separate. Asset
selection is based on the dialog's verified background family, not on visual
similarity.

Split RMG sprite emission conceptually into:

- shell foundation selection;
- child-control painting.

This need not create durable state or a new renderer. It should use the current
base-rail, lower-strip, and optional-chrome helpers.

### Interfaces and Contracts

- Generic RMG background resolution accepts the atlas and screen layout and
  returns only the verified width-selected generic entry.
- Exactly one left-canvas base is emitted:
  - retail generic background; or
  - bounded primitive fallback.
- Common rail and lower strip are emitted before either base.
- `SDTP#1` is emitted after the base and before child controls.
- No `SDMPBTN` role is present for `0x105`.
- In retail-art mode, combo/control frame painters do not lay broad opaque
  green plates over the artwork. Selection, bevel, arrow, thumb, and text layers
  remain visible.
- In fallback mode, explicit opaque interiors remain allowed for readability.
- Preview and text passes keep their current interfaces and depths.
- No render availability flag enters `RandomMapSetupModalState`.

### Data Flow

1. The Choose Map flow opens `RandomMapSetupModalState`.
2. The renderer computes normal shell geometry and `RandomMapSetupLayout`.
3. The RMG branch emits common base rail and the width-selected lower strip.
4. It resolves `MNSCRNS#0` or `MNSCRNL#0`; if unavailable it emits the bounded
   fallback.
5. It emits the `RandomMapSetup0x105` optional chrome profile.
6. It emits combo/trackbar/action/right-panel/progress/preview-frame sprites
   using the selected backdrop policy.
7. Existing outer passes emit the generated preview, labels, values, button
   captions, progress text, and cursor.
8. Existing input and app logic update modal state and request redraws without
   new renderer-to-state coupling.

### Error Handling

`MNSCRNL.SHP` and `SHELL.PAL` loading remains optional at the atlas boundary.
Missing or invalid retail assets produce one warning and activate the bounded
primitive fallback. The draw loop performs no file I/O, does not panic, and
does not borrow a visually similar dialog asset as an undocumented substitute.

The common rail behavior follows the existing atlas contract. This change does
not weaken construction requirements for already-mandatory rail assets.

### Testing Strategy

- Add/extend atlas tests proving `MNSCRNL.SHP` is classified and packed as frame
  0 through `SHELL.PAL`.
- Add a semantic `0x105` draw-order test asserting:
  - base rail and lower strip precede the generic background;
  - `SDTP#1` follows the background;
  - no `SDMPBTN` role is present;
  - child controls and preview frame follow optional chrome.
- Preserve and run the existing `0x102` and `0x6B` semantic-order tests to pin
  their different background and optional-layer profiles.
- Add a retail-art composition test proving no opaque full-dialog cover follows
  the `MNSCRNL` entry.
- Add a fallback composition test proving the fallback is bounded to the left
  canvas and does not cover the rail or lower strip.
- Add focused control-backing tests proving retail-art mode preserves the
  background while fallback mode remains readable.
- Preserve existing random-map layout, input, combo, trackbar, generate,
  accept/cancel, preview, and saved-seed tests.
- Format only edited Rust files.
- Before Cargo work, check for active Cargo/rustc owners; run focused tests
  serially, then one final `cargo check -q`.
- The user performs visual acceptance at 800x600 without Windows app control.
  Acceptance requires:
  - red world-map artwork visible across the left canvas;
  - continuous metallic right rail and lower strip;
  - framed Generate Map title and preview bay;
  - recessed Use/Load/Save/Delete and Cancel bays;
  - readable controls with no flat green screen-sized cover;
  - unchanged generation, preview, saved-seed, and cancel behavior.

## Architectural Decisions

The generic `MNSCRNL` entry belongs in the skirmish atlas even though the main
menu atlas already loads the same asset. Each renderer owns one packed texture
and batch; sharing a runtime entry across those atlases would introduce a
cross-renderer dependency for negligible memory savings.

Dialog differences remain explicit profiles. This mirrors the verified native
contract: common shell composition, background selection by dialog ID, optional
chrome markers, then owner-draw child controls.

The design does not introduce a general skin system, rewrite modal state,
change RMG behavior, or alter resource-derived geometry. Those would expand the
blast radius without addressing the visible defect.

The contradicted `0x105` background claim in
`SKIRMISH_RANDOM_MAP_DIALOG_0X105_LAYOUT_GEOMETRY_GHIDRA_REPORT.md` is a
documentation follow-up, not production authority. Implementation follows the
fresh common-selector evidence and the stronger full background-pointer-table
report.

## Alternatives Considered

### Reuse the main-menu atlas entry directly

The main menu already packs `MNSCRNL#0` with `SHELL.PAL`, but its texture and
render batch have separate ownership. Sharing the entry would couple renderer
lifetimes, texture bindings, and draw orchestration for no behavioral benefit.

### Duplicate a standalone Random Map screen renderer

A dedicated renderer could assemble the screenshot quickly, but it would
duplicate the rail, lower-strip, button, and text composition already shared by
the other skirmish dialogs. Future asset or clipping corrections would drift
between three implementations.

### Reuse the complete Customize Battle composition

This matches the dialog frame geometry but selects the wrong orange background
family. It would also encourage copying `0x6B`-specific assumptions instead of
representing the verified `0x105` background and chrome markers.

### Bake the reference screenshot into one image

This could resemble the supplied capture at one resolution, but it would
replace retail assets with generated pixels, break native clipping and palette
roles, interfere with dynamic controls and preview content, and fail at other
shell sizes.
