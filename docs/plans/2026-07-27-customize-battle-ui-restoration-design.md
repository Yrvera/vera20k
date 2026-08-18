# Customize Battle UI Faithful Restoration Design

## Goal

Restore the ordinary offline-skirmish Customize Battle screen at 800x600 to the retail-convincing Yuri's Revenge composition shown in the user reference: the orange retail battle artwork remains visible behind the controls, while the existing chooser behavior stays intact.

## Architecture Context

Customize Battle is the active Choose Map modal represented by `ChooseMapModalState`. `App::open_choose_map_modal` creates that state, app input routes button and listbox events into it, and `render_skirmish_shell_with_atlas` selects a modal-only render branch while it is open. The parent skirmish setup composition is therefore already suppressed rather than painted underneath the chooser.

The visual path has three stages:

1. `compute_choose_map_modal_layout` supplies the verified list, button, title, heading, status, and preview rectangles.
2. `build_skirmish_shell_instances` calls `push_choose_map_modal_instances`, which emits the retail background, listbox chrome, selection rows, buttons, and preview frame.
3. `render_skirmish_shell_with_atlas` appends the selected-map preview and text draws above those sprites.

`SkirmishShellChromeAtlas` already loads frame 0 of `MnScrnLCustomizeBattle.shp` through its matching `MnScrnLCustomizeBattle.PAL` and exposes it as `choose_map_background_800_customize_battle`. The current visual failure is composition, not asset discovery: after emitting that retail background, `push_choose_map_modal_instances` unconditionally emits a closer full-screen opaque green rectangle. Each chooser listbox also unconditionally emits an opaque green interior. These layers cover the orange artwork that the user reference shows through the screen and list interiors.

This work stays in the app/render/UI layers. It does not change `sim/`, map selection authority, random-map generation, deterministic state, or persisted settings.

## Impact Analysis

Primary production surface:

- `src/app_skirmish_shell_render/modals.rs`
  - Make the retail-art and fallback compositions mutually exclusive.
  - Let chooser listboxes omit their opaque interior only when the retail background is active.
  - Keep the saved-seed browser's existing composition outside this change.

Validation surfaces:

- `src/app_skirmish_shell_render/draw_order.rs`
  - Preserve and strengthen the existing semantic contract that the Customize Battle background and primitive fallback are alternatives, not simultaneous layers.
- `src/app_skirmish_shell_render.rs`
  - Add focused composition tests near the existing Choose Map draw-order tests if the lower-level helper cannot expose the invariant cleanly.

Inspected and expected to remain behaviorally unchanged:

- `src/render/skirmish_shell_chrome.rs`: the correct one-frame SHP and modal-specific palette are already loaded.
- `src/ui/skirmish_shell/layout.rs`: the current 800x600 resource-derived geometry is already aligned with the verified dialog layout.
- `src/app_skirmish_shell_render/text.rs`: localized headings, rows, button captions, and status help already occupy the intended rectangles.
- `src/ui/skirmish_shell/state/choose_map.rs` and `src/app.rs`: list filtering, selection, scrolling, preview commit boundary, button capture/release, sounds, and modal actions are already wired.

The main blast-radius risk is the shared `push_choose_map_listbox_instances` helper, which is also used by the saved-seed browser. The implementation must pass an explicit backdrop policy or use a chooser-specific wrapper so transparency is not silently imposed on other dialogs. There are no data migrations or public-format changes.

## Chosen Approach

Use an asset-first, fallback-explicit composition.

At exact 800-pixel screen width, if the verified Customize Battle atlas entry is available, paint frame 0 natively at the screen origin and do not paint either the full-screen primitive backdrop or opaque chooser-list interiors. Paint the red selection rows, owner-draw borders, scrollbar, preview, buttons, and text above the retail artwork.

If the verified entry is unavailable, keep a readable primitive fallback: paint the flat full-screen backdrop and opaque list interiors before the existing overlays. This prevents missing or corrupt retail assets from turning the chooser into black/undefined content.

This approach fixes the whole contradicted composition while preserving current architecture and behavior. It is preferred over a background-only removal because that would leave the two opaque list interiors visibly covering a large part of the artwork. It is preferred over a renderer redesign because the current modal state, geometry, input, preview, text, and asset-loading boundaries already fit the verified production loop.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — The active standard-YR offline-skirmish dialog is Choose Map `0x6B`, and its exact-800 shell path binds `MnScrnLCustomizeBattle.shp/.PAL`, distinct from the parent setup `0x102` artwork. The SHP branch uses frame 0 and the asset is one frame, so there is no uninspected animation variant for this role. [doc: `SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md` §§3, 7] [doc: `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md` asset inventory]
- `MILESTONE-BLOCKING` — The user reference shows the orange battle artwork and metallic shell visible behind both listboxes. The current runtime capture instead shows a flat green screen and green list interiors. [runtime: user captures `codex-clipboard-2459831f-9f46-46cf-9803-8d87395a0355.png` and `codex-clipboard-5d321692-77b3-4f22-b2f4-db4c779ca623.png`]
- `MILESTONE-BLOCKING` — Current Rust emits the correct background and then covers it with `SHELL_MODAL_BG_RGB`; listbox construction separately covers the art with `SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE`. The background/fallback semantic draw-order model says these are alternatives. [rust: `src/app_skirmish_shell_render/modals.rs`, `push_choose_map_modal_instances` and `push_choose_map_listbox_instances`] [rust: `src/app_skirmish_shell_render/draw_order.rs`, `choose_map_modal_semantic_draw_order`]
- `MILESTONE-BLOCKING` — Draw order in retail-art mode is: modal-specific background; selection fills and list chrome; owner-draw buttons and preview frame; decoded preview surface and start markers when applicable; localized headings, rows, button captions, and status text. Reordering the preview or text beneath the background would recreate a blank-looking screen. [rust: `src/app_skirmish_shell_render.rs`, `build_skirmish_shell_instances` and `render_skirmish_shell_with_atlas`]
- `MILESTONE-BLOCKING` — The two listboxes retain the verified 19-pixel visible-row cadence, 2-pixel text inset, red full-content-row selection, 20-pixel scrollbar reservation when overflowing, and their existing verified rectangles. Transparency changes only the interior backdrop, not input or row geometry. [doc: `SKIRMISH_CHOOSE_MAP_0X6B_POST_IMPLEMENTATION_GAP_AUDIT_GHIDRA_REPORT.md` listbox sections] [rust: `src/ui/skirmish_shell/layout.rs`; `src/ui/skirmish_shell/state/choose_map.rs`]
- `MILESTONE-BLOCKING` — Use Map commits the highlighted record; passive map highlighting does not replace the parent selection/preview; Cancel closes without committing; Create Random Map opens its setup path. These behavior boundaries must not change in a visual fix. [doc: `SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md` modal result paths] [rust: `src/app.rs`, `handle_choose_map_modal_mouse_up`]
- `MILESTONE-BLOCKING` — Buttons retain their current owner-draw idle/pressed SHP frames, press-capture-release activation, and menu sound trigger. The right-side map preview and marker overlay remain above their black preview plate. [rust: `src/app_skirmish_shell_render/modals.rs`; `src/app.rs`; `src/app_skirmish_shell_render.rs`]
- `COMPOUNDING` — The parent setup screen must remain suppressed throughout the modal lifetime. Painting it below translucent chooser regions would leak unrelated controls and chrome into the restored screen. [doc: `SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md` lifecycle/composition sections] [rust: `src/app_skirmish_shell_render.rs`, modal early return]
- `COMPOUNDING` — Missing-asset fallback must remain self-contained and readable. It needs both the primitive screen backdrop and opaque list interiors because it has no retail content layer to show through transparency. [rust: current atlas `Option` contract and warning path in `src/render/skirmish_shell_chrome.rs`]
- `COMPOUNDING` — Saved-seed browsing reuses the listbox painter but is not the scoped Choose Map composition. The transparency decision must be explicit at the callsite so the fix cannot silently alter that neighboring dialog. [rust: `src/app_skirmish_shell_render/modals.rs`, `push_saved_seed_modal_instances`]
- `EXACTIFICATION-RESIDUAL` — Native behavior above exact width 800 is not proven by the supplied reference capture, whose presentation has been scaled/cropped. Keep the existing non-800 fallback and do not stretch the 632x568 asset without runtime evidence. Trigger: non-800 shell width. Player effect: less-authentic background at higher resolutions. Frequency: depends on selected shell resolution. Downstream risk: bounded to presentation; no state or authority effect. [doc: `SKIRMISH_CHOOSE_MAP_0X6B_POST_IMPLEMENTATION_GAP_AUDIT_GHIDRA_REPORT.md` unresolved width policy]
- `EXACTIFICATION-RESIDUAL` — The runtime comparison can establish retail-convincing composition but cannot certify pixel parity without a same-resolution native capture and executable image comparison. The result must be reported as visually restored, not exact pixel parity. [project: `AGENTS.md` truth bar]

## Design

### Components

Introduce a small explicit visual policy at the Choose Map composition boundary, conceptually:

- `RetailArtwork`: verified atlas entry exists at exact width 800.
- `PrimitiveFallback`: verified entry is unavailable or the screen width does not activate it.

The policy is derived once in `push_choose_map_modal_instances`. It controls:

- whether the retail background or the flat backdrop is emitted;
- whether the two chooser listboxes emit an opaque interior.

It does not enter `ChooseMapModalState`; this is renderer availability, not durable UI state. The listbox helper should accept an explicit interior policy, or a chooser-only wrapper should add transparency, so saved-seed behavior cannot change accidentally.

### Interfaces / Contracts

- `choose_map_background_entry(atlas, layout)` remains the authority for whether the verified exact-800 artwork is active.
- Exactly one base layer is emitted:
  - retail SHP entry; or
  - primitive full-screen backdrop plus fallback outline.
- In retail mode, chooser listbox base fill count is zero. Selection, scrollbar, bevel frame, and text remain present.
- In fallback mode, chooser listbox base fills remain present for readability.
- Existing layout, state, input, asset atlas, preview texture, and text interfaces remain unchanged.

### Data Flow

1. The app opens `ChooseMapModalState`.
2. The renderer computes the chooser layout.
3. The renderer resolves the optional exact-800 Customize Battle atlas entry.
4. The modal sprite builder selects retail or fallback composition.
5. It emits both listboxes using the matching interior policy, then buttons and preview frame.
6. The outer renderer adds the decoded selected-map preview and optional start-marker overlays.
7. The text pass adds localized labels, visible rows, captions, and status help.
8. Input continues to mutate modal selection/scroll/pressed state; redraws consume that state without new coupling.

### Error Handling

Asset loading remains optional. The existing atlas loader logs missing or invalid `MnScrnLCustomizeBattle.PAL/SHP`. Rendering then chooses the primitive fallback instead of failing the shell or leaving transparent undefined regions. No new panic, file read, or runtime asset lookup is introduced in the draw loop.

### Testing Strategy

- Add a focused structural test proving that retail-art mode emits the verified background without a primitive full-screen cover.
- Add a focused structural test proving chooser list interiors are transparent in retail-art mode but opaque in fallback mode.
- Preserve the existing semantic draw-order test asserting `ChooseMapBackgroundCustomizeBattle800` and `ChooseMapModalBackdrop` are mutually exclusive.
- Preserve existing 800x600 layout, row hit-testing, scrollbar, selection, Use Map, Cancel, Create Random Map, and hover-status tests.
- Format only edited Rust files.
- Coordinate with active Cargo owners, then run the narrow renderer/UI tests serially and one final `cargo check -q`.
- Launch the production shell at 800x600 and capture Customize Battle with:
  - a selected game type and map;
  - overflowing map list and visible scrollbar;
  - selected-map preview and markers;
  - idle and pressed right-panel buttons where practical.
- Compare that capture to the supplied reference by composition category: orange artwork and metal frame visible, transparent list interiors, correct overlay order, no parent setup leakage, readable controls, and functional interactions. Do not claim native pixel parity from this visual review alone.

## Architectural Decisions

The design follows the existing atlas-optional fallback pattern already used by validation modal painting: an available retail asset is authoritative, while primitive geometry is emitted only when that asset is missing. It also follows the existing modal-only render branch and keeps presentation decisions out of UI state.

The design deliberately does not create a generalized skinning system, change modal geometry, or move app input behavior into rendering. Those would be new patterns without a need in this bounded restoration.

No simulation or deterministic-state debt is introduced. The only retained debt is the named non-800 presentation residual, which remains honest until native runtime evidence proves a scaling or alternate-background policy.

## Alternatives Considered

### Uncover only the full-screen background

Remove the unconditional full-screen green cover but retain opaque list interiors. This is smaller, but it still hides much of the defining retail artwork and fails the supplied reference in the most visually prominent control region.

### Rebuild the entire Customize Battle renderer

Replace current geometry, state, input, and chrome composition with a new dedicated renderer. This could centralize everything, but it duplicates already-correct modal ownership and behavior, expands the blast radius, and creates unnecessary parity risk.

### Stretch the retail artwork at every resolution

Scale frame 0 to fill any window. This might look more polished at high resolution, but active binary evidence only proves the SHP branch at exact width 800. Stretching would turn an unresolved native policy into invented behavior and is therefore deferred.
