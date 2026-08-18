# Customize Battle Sidebar Restoration Design

## Goal

Restore the complete retail right-hand shell rail on the active 800x600
Customize Battle screen while preserving the already-restored orange artwork,
modal behavior, and existing quality-of-life controls.

## Architecture Context

Customize Battle is the active Skirmish Choose Map dialog `0x6B`. The app owns
its state in `ChooseMapModalState`; `render_skirmish_shell_with_atlas` computes
both the normal `SkirmishShellLayout` and the modal
`ChooseMapModalLayout`; `build_skirmish_shell_instances` selects the modal
branch; and `push_choose_map_modal_instances` emits the modal background and
controls.

The current modal branch returns before the normal shell's common right-panel
composition runs. That is why the 168-pixel rail is correctly reserved at
`x=632` but remains black except for modal controls and the preview surface.
The normal shell already has the required atlas entries, geometry, clipping,
and steady button art:

- `SDTP.SHP` frame 0, `168x199`, as the top rail;
- `SDBTNBKGD.SHP` frame 0, `168x42`, repeated through the middle;
- `SDBTM.SHP` frame 0, `168x65`, source-clipped into the remaining bottom cap;
- `LWSCRNL.SHP` frame 0, `632x32`, across the lower left edge at 800 width;
- `SDTP.SHP` frame 1 as the enabled `0x6B` steady top-highlight layer;
- `SDBTNANM.SHP` frames 2 and 4 for idle/pressed modal buttons.

The retail common parent paint contract is right-panel base chrome, then the
dialog-specific background, then enabled optional chrome, then child controls.
Choose Map enables the D9/top-highlight family but not the DA/`SDMPBTN` family.
Therefore the correct stable `0x6B` rail is not identical to setup dialog
`0x102`: both use the common base rail and top highlight, but only `0x102`
adds `SDMPBTN.SHP`.

This work stays in `ui/`, app-render planning, and render-asset composition. It
does not change `sim/`, modal authority, map selection, random-map generation,
input capture, deterministic state, or persistence.

## Impact Analysis

Primary production surfaces:

- `src/app_skirmish_shell_render.rs`
  - Replace the modal early-return gap with a complete modal-shell composition.
  - Reuse the same base rail emitter for setup and Choose Map.
- `src/app_skirmish_shell_render/chrome.rs`
  - Own the shared right-panel base and optional steady-layer emitters.
  - Express the `0x102` versus `0x6B` optional-chrome profile explicitly.
- `src/app_skirmish_shell_render/draw_order.rs`
  - Extend the semantic Choose Map order to include the common rail, lower
    strip, modal background, and the `0x6B` top highlight.
- `src/app_skirmish_shell_render/modals.rs`
  - Keep modal controls separate from the shell foundation so the foundation
    can appear between background and child-control layers.
  - Ensure an asset-missing fallback covers only the left content region and
    cannot cover the restored rail.

Inspected and expected to remain behaviorally unchanged:

- `src/render/skirmish_shell_chrome.rs`: all stable rail assets and relevant
  frames are already packed. No new runtime asset lookup is required.
- `src/ui/skirmish_shell/layout.rs`: verified 800x600 rail and `0x6B` control
  geometry already exists.
- `src/app_skirmish_shell_render/text.rs`: text remains above the shell and
  control sprites.
- `src/app.rs` and `src/ui/skirmish_shell/state/choose_map.rs`: modal actions,
  filtering, highlight/commit separation, scrolling, capture/release, and
  sounds remain unchanged.

The principal blast-radius risk is accidentally changing the normal setup
dialog's already-working rail while extracting the shared emitter. A semantic
order regression test and fixed 800x600 geometry tests must pin both profiles.
There are no data migrations or public format changes.

## Chosen Approach

Introduce one shared, profile-aware shell-rail composition in the Skirmish
renderer.

The base emitter draws `SDTP#0`, repeated `SDBTNBKGD#0`, the conditionally
enabled frame-10 row overlay, source-clipped `SDBTM#0`, and the width-selected
lower strip. A small dialog profile then selects steady optional layers:

- setup `0x102`: `SDTP#1` and `SDMPBTN#0`;
- Choose Map `0x6B`: `SDTP#1`, but no `SDMPBTN`;
- both standard offline steady states: no repeated `SDBTNANM#10` row overlay.

For Choose Map, the complete stable order is:

1. common right-panel base and lower strip;
2. `MnScrnLCustomizeBattle.SHP` background at exact logical width 800, or a
   bounded left-content fallback;
3. `SDTP.SHP` frame 1 top highlight;
4. listbox selection/borders/scrollbar, modal buttons, and preview frame;
5. decoded preview, markers, localized text, and cursor through the existing
   outer passes.

This approach is preferred because it closes the actual parent-paint sequence,
keeps asset roles explicit, and prevents the setup and modal rail paths from
drifting apart.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — The current screenshot shows an empty black
  168-pixel rail with floating preview and buttons; retail shows a continuous
  metallic rail from the title cap through the Cancel footer. This dominates
  the screen and is visible every time Choose Map opens. [runtime: user
  captures `codex-clipboard-33fac5d6-d1e9-4591-a880-9b70a55c4301.png` and
  `codex-clipboard-fa0f126f-c3a4-457a-bb90-87ef08e270a8.png`]
- `MILESTONE-BLOCKING` — `build_skirmish_shell_instances` returns from its
  modal branch before emitting any common rail layers. The normal path emits
  them immediately after that branch. [rust:
  `src/app_skirmish_shell_render.rs`, `build_skirmish_shell_instances`]
- `MILESTONE-BLOCKING` — Active `0x6B` uses the common fullscreen shell path
  and binds `MnScrnLCustomizeBattle` as its dialog-specific background, not as
  a monolithic replacement for the common rail. [doc:
  `SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md` §§4-5]
- `MILESTONE-BLOCKING` — Base rail order is `SDTP#0`, repeated
  `SDBTNBKGD#0`, optional repeated `SDBTNANM#10`, `SDBTM#0`, then
  `LWSCRNL/LWSCRNS#0`; the dialog background overlays afterward. [doc:
  `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md` §§3.5-3.7]
- `MILESTONE-BLOCKING` — At 800x600 the rail geometry is top
  `(632,0,168,199)`, nine middle cells from `y=199`, and bottom
  `(632,577,168,23)`. The 65-pixel `SDBTM` source is cropped, not squeezed,
  into that 23-pixel destination. [doc:
  `SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`] [rust:
  `src/ui/shell/geom.rs`]
- `MILESTONE-BLOCKING` — The lower edge is a separate
  `LWSCRNL.SHP` `632x32` layer at `(0,568)`. Leaving it out produces the black
  status strip visible in the current capture. [doc:
  `RIGHT_PANEL_SHP_HEADER_DIMENSIONS_GHIDRA_REPORT.md`] [retail asset
  inspection: `LWSCRNL.SHP`, one frame]
- `MILESTONE-BLOCKING` — `0x6B` enables the D9 steady top-highlight family but
  not the DA `SDMPBTN` family. The stable modal therefore uses `SDTP#1` above
  its background and must not inherit setup's `SDMPBTN#0`. [doc:
  `SHELL_DIALOG_LIFETIME_TRANSITION_EVIDENCE_GATES_GHIDRA_REPORT.md`, Gate 4]
- `MILESTONE-BLOCKING` — Modal child controls remain above the shell
  foundation: buttons use `SDBTNANM#2` idle and `#4` pressed; selection rows,
  list borders, preview, headings, and status text must remain readable and
  interactive. [doc:
  `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`]
- `COMPOUNDING` — The background and rail are separate asset roles and palette
  paths. Baking them into one generated image would destroy source clipping,
  optional-layer policy, and future transition compatibility. [doc:
  `FUN_0060CF00_DIALOG_BACKGROUND_POINTER_TABLE_GHIDRA_REPORT.md`] [rust:
  `src/render/skirmish_shell_chrome.rs`]
- `COMPOUNDING` — The normal setup rail is already used in production. The
  extraction must preserve its current order, `SDTP#1`, and `SDMPBTN#0`;
  changing that sibling path would regress the screen the user leaves and
  returns to. [rust: `src/app_skirmish_shell_render.rs`]
- `COMPOUNDING` — Missing `MnScrnLCustomizeBattle` artwork must not make the
  lists unreadable or cover the rail. The fallback should fill only the
  left-content region bounded by `right_panel.top.x` and the lower strip.
  [rust: atlas `Option` contract and modal fallback]
- `EXACTIFICATION-RESIDUAL` — The verified `0x6B` show/close transition uses
  an 11-tick D9 sequence with `SDWRNTMP` frames. This restoration targets the
  stable screen shown in the user's references and leaves that short
  transition animation unchanged. Trigger: entering/leaving Choose Map.
  Player effect: immediate/still-current transition rather than exact retail
  per-frame reveal. Frequency: once per open/close. Downstream risk: bounded
  presentation only; stable rail composition and control state are unaffected.
  [doc: `SHELL_DIALOG_LIFETIME_TRANSITION_EVIDENCE_GATES_GHIDRA_REPORT.md`,
  Gate 4]
- `EXACTIFICATION-RESIDUAL` — Exact background output above logical width 800
  is not established by the active SHP load branch. Keep current helper-based
  anchoring and do not globally scale or stretch rail assets. Trigger:
  non-800 logical shell. Player effect: fallback/empty content outside the
  native canvas. Downstream risk: presentation-only. [doc:
  `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md` §6]
- `EXACTIFICATION-RESIDUAL` — Manual screenshot review can support a
  retail-convincing verdict but not native pixel parity without a
  same-resolution executable image comparison. [project: `AGENTS.md` truth
  bar]

## Design

### Components

Add an app-render-only dialog chrome profile, conceptually:

- `SkirmishSetup0x102`
- `ChooseMap0x6b`

It does not enter app state. It selects only verified presentation layers.

Split shell emission into:

- a common base-rail emitter;
- a dialog-background emitter;
- a steady optional-chrome emitter;
- existing dialog child-control emitters.

The existing normal setup and Choose Map branches call those pieces in their
verified order. The modal branch continues to return before setup controls are
emitted, preserving parent suppression.

### Interfaces / Contracts

- The common base-rail emitter accepts the atlas, `SkirmishShellLayout`, and
  frame-10 overlay state, and emits no dialog-specific background or child
  control.
- The optional-chrome emitter accepts the dialog profile:
  - both current profiles emit `SDTP#1`;
  - only `SkirmishSetup0x102` emits `SDMPBTN#0`.
- Choose Map background resolution remains authoritative in
  `choose_map_background_entry`.
- The fallback rect is derived from shell geometry, not a hardcoded full-screen
  rectangle:
  - left edge at the common shell origin;
  - right edge at the right-panel origin;
  - bottom edge at the lower-strip top.
- Existing modal listbox interior policy remains:
  transparent over verified artwork, opaque over fallback.
- No profile changes modal input, pressed state, preview selection, or text.

### Data Flow

1. The app opens `ChooseMapModalState`.
2. The renderer computes normal shell and modal layouts.
3. The modal sprite branch emits common base rail and lower strip.
4. It resolves and emits the modal-specific background or bounded fallback.
5. It emits `0x6B` steady optional chrome (`SDTP#1`, no `SDMPBTN`).
6. It emits Choose Map listboxes, buttons, and preview frame.
7. Existing outer passes emit the preview texture, start markers, text, and
   cursor.
8. Cancel or Use Map closes the modal and the normal `0x102` branch resumes
   with its own profile.

### Error Handling

The common rail assets remain mandatory for construction of the Skirmish shell
atlas. If the modal-specific background or palette is unavailable, the shell
continues with the bounded primitive fallback and opaque list interiors.
Rendering does not panic, perform draw-loop I/O, or invent a replacement SHP.

### Testing Strategy

- Extend the semantic Choose Map order test to assert:
  - base top, nine tiles, bottom cap, and large lower strip precede the modal
    background;
  - `SDTP#1` follows the modal background;
  - no frame-10 row overlay or `SDMPBTN` role is present;
  - modal child controls follow optional chrome.
- Preserve the normal setup semantic-order test and assert its
  `SDMPBTN#0` role remains present.
- Add fixed 800x600 geometry tests for the rail and bounded fallback:
  `(632,0,168,199)`, nine `42`-pixel cells, `(632,577,168,23)`, and
  `(0,568,632,32)`.
- Add a bottom-cap instance test proving source UV height is cropped rather
  than the 65-pixel source being scaled.
- Preserve chooser listbox transparency/fallback tests and all modal
  interaction tests.
- Format only edited Rust files, run focused renderer/UI tests serially, then
  one `cargo check -q` after checking for active Cargo owners.
- The user performs the visual acceptance at 800x600 without Windows app
  control. Acceptance requires a continuous retail rail, framed title/preview,
  recessed button bays, lower strip, and Cancel footer with no regression to
  the orange artwork, list readability, preview, or interaction.

## Architectural Decisions

The design follows the existing shared shell geometry and atlas pattern. It
adds no new renderer, skin format, or durable state. Dialog differences are
expressed as a small profile because native uses common shell composition plus
per-dialog flags; duplicating the entire rail per dialog would hide that real
relationship.

The design deliberately preserves native asset sizes and integer anchoring.
Quality-of-life behavior is limited to retaining current responsive placement
outside the exact-800 path; it does not stretch artwork or controls.

The stable rail is the scoped delivery result. The verified D9 transition
animation remains recorded as presentation drift rather than being silently
claimed complete.

## Alternatives Considered

### Duplicate the normal rail code inside the modal branch

This would produce the right screenshot quickly, but it would duplicate the
same top/tile/bottom/lower-strip sequence and make future clipping or palette
corrections easy to apply to one screen but not the other.

### Bake the rail into one 800x600 background image

This removes runtime layering, but it conflates dialog background and common
chrome, loses the clipped bottom-cap contract, cannot represent per-dialog
optional layers, and depends on generated pixels instead of retail assets.

### Reuse the complete `0x102` shell foundation unchanged

This is closer, but still wrong: setup enables `SDMPBTN#0`, while Choose Map
does not. A dialog profile is necessary to avoid importing setup-only chrome
into the modal.
