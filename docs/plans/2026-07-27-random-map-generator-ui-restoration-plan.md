# Random Map Generator UI Faithful Restoration Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Restore the stable Random Map Generator dialog `0x105` at 800x600 with
the retail red `MNSCRNL` background, complete shell rail/lower strip, correct
top highlight, and unchanged RMG behavior.

**Architecture:** Extend the existing skirmish shell atlas and shared
app-render composition. Keep dialog-specific background selection and optional
chrome explicit, then paint the existing RMG controls, preview, and text above
that foundation. No simulation, generation, persistence, or input authority
changes are permitted.

**Design Doc:**
`docs/plans/2026-07-27-random-map-generator-ui-restoration-design.md`

---

## Grounding Summary

- The supplied retail capture shows the `0x105` screen using the red world-map
  artwork, a complete 168-pixel right rail, and the 632-pixel lower strip at
  800x600.
- High-confidence background-pointer research and a fresh read-only decompile
  of `0x0060CF00` show that `0x105` has no special case and therefore uses the
  generic shell family: `MNSCRNS.SHP` at 640, `MNSCRNL.SHP` otherwise, through
  `SHELL.PAL`.
- A fresh read-only decompile of `0x00622820` confirms that `0x105` enables the
  `data+0xD5`/`SDTP#1` top highlight and does not enable the
  `data+0xD6`/`SDMPBTN` map-button layer.
- `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md` verifies the common
  stable order: right-panel base, lower strip, parent background, optional
  chrome, then child controls.
- The `0x105` geometry report remains authoritative for the 533x369-DLU
  template and control rectangles. Its stale inference that `0x105` shares
  `0x6B` artwork has been superseded in that report by the 2026-07-27
  correction.
- The owner-draw combo research verifies cached parent/backing composition
  rather than one literal green face color. Exact final pixels remain
  runtime-capture dependent.
- `src/render/main_menu_shell_chrome.rs` already demonstrates decoding
  `MNSCRNL.SHP` frame 0 through `SHELL.PAL`; the skirmish atlas should mirror
  that asset-loading pattern without borrowing its texture.
- The current working tree already contains the uncommitted shared rail/profile
  extraction from the Customize Battle restoration in
  `app_skirmish_shell_render.rs`, `chrome.rs`, `draw_order.rs`, and `modals.rs`.
  The implementation must build on and preserve those changes.
- Current RMG rendering still selects the Customize Battle asset, covers it
  with `SHELL_MODAL_BG_RGB`, and returns before shared rail composition.
- No INI key drives this presentation. Retail SHP/PAL files and the dialog ID
  dispatch are the sources of truth.
- No TS-legacy path is involved: `0x583 -> 0x005E8590 -> 0x00595BC0 -> 0x105`
  is live in ordinary offline YR Skirmish when random maps are available.

## Key Technical Decisions

- Add separately labeled generic-background atlas entries rather than replacing
  the existing `0x102` `MNSCRNS` entry. The same indexed SHP uses
  `MnScrnLCoopGameSetup.PAL` for `0x102` but `SHELL.PAL` for generic `0x105`.
  **Confidence: high**
  - **Source:** live `0x0060CF00`; `FUN_0060CF00_DIALOG_BACKGROUND_POINTER_TABLE_GHIDRA_REPORT.md`;
    current atlas palette split.
- Pack generic backgrounds into `SkirmishShellChromeAtlas` rather than sharing
  `MainMenuShellChromeAtlas` entries. Each atlas owns its texture and batch.
  **Confidence: high**
  - **Source:** repo pattern `src/render/main_menu_shell_chrome.rs` and
    `src/render/skirmish_shell_chrome.rs`.
- Add `RandomMapSetup0x105` to the existing render-only chrome profile.
  It draws `SDTP#1` and never `SDMPBTN`.
  **Confidence: high**
  - **Source:** live `0x00622820`; common-dialog worknote field map.
- Reuse common rail and lower-strip emitters before the RMG background.
  Do not paint the normal `0x102` setup screen beneath the modal.
  **Confidence: high**
  - **Source:** common parent-paint report and current modal replacement
    lifecycle.
- Use one explicit interior policy: preserve composed retail backing when the
  SHP exists; use opaque interiors only in the bounded missing-asset fallback.
  **Confidence: high for composition mechanism; exact pixels unchecked**
  - **Source:** `OwnerDraw_ComboBox_00617250` backing-copy evidence and the
    supplied retail capture. Final RGB remains a visual residual.
- Keep native SHP dimensions and integer placement. For non-640 widths select
  `MNSCRNL` but do not scale it.
  **Confidence: high for asset selection; medium for presentation above 800**
  - **Source:** `Background_Overlay @ 0x0072E730`; main-menu composition report;
    project exactification policy.

## Open Questions

### Resolved During Planning

- **Does `0x105` reuse Customize Battle artwork?** No. It falls through
  `0x0060CF00` to generic `MNSCRNS/MNSCRNL` with `SHELL.PAL`.
- **Does `0x105` draw the top highlight?** Yes. `0x00622820` includes it in the
  `data+0xD5` set.
- **Does `0x105` draw `SDMPBTN`?** No. It is absent from the `data+0xD6` set.
- **Is the RMG behavior path active in standard YR?** Yes, conditionally through
  the ordinary offline Choose Map Create Random Map command.
- **Are INI parsing changes needed?** No. This slice is entirely shell
  asset/composition work.

### Deferred to Implementation

- **Exact final combo-face pixels:** native owner-draw painting composes cached
  backing pixels and state-dependent primitives. The implementation will remove
  the known-wrong broad green plates and preserve the retail backing; the user
  will validate the final stable frame at 800x600.
- **Transition animation:** the show/close slide sequence is outside this stable
  screen restoration.
- **Exact presentation above 800:** retain native-size centered artwork and
  bounded fallback without claiming pixel parity.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/render/skirmish_shell_chrome.rs` | Pack role-specific generic `MNSCRNS/MNSCRNL` entries through `SHELL.PAL` |
| Modify | `src/app_skirmish_shell_render/draw_order.rs` | Define the `0x105` profile, background roles, and semantic order |
| Modify | `src/app_skirmish_shell_render/modals.rs` | Select the RMG background/fallback and paint controls without covering retail art |
| Modify | `src/app_skirmish_shell_render.rs` | Wire common shell foundation and RMG controls in verified order; add regression tests |
| Preserve | `src/app_skirmish_shell_render/chrome.rs` | Reuse current uncommitted rail/lower-strip/optional-chrome helpers without behavioral change |
| Preserve | `src/ui/skirmish_shell/layout.rs` | Keep resource-derived `0x105` geometry unchanged |
| Preserve | `src/ui/skirmish_shell/state/random_map_setup.rs` | Keep RMG state, commands, and lifecycle unchanged |
| Preserve | `src/app_skirmish_shell_render/preview.rs` | Keep generated preview rendering unchanged |
| Preserve | `src/app_skirmish_shell_render/text.rs` | Keep RMG labels and button text unchanged |
| Preserve | `src/app.rs` and `src/map/rmg/` | Keep generation, accept/cancel, and saved-seed behavior unchanged |

`skirmish_shell_chrome.rs`, `app_skirmish_shell_render.rs`, `modals.rs`, and
`chrome.rs` already exceed roughly 600 lines, but they are existing cohesive
atlas/composition or test-heavy files. This bounded change adds small
role-specific blocks and does not justify a new module split.

## Interface Changes

All changes are internal to render planning:

- `SkirmishShellChromeAtlas` gains two optional entries:
  `generic_background_640_mnscrns_shell` and
  `generic_background_large_mnscrnl_shell`.
- `ShellDialogChromeProfile` gains `RandomMapSetup0x105`.
- `SkirmishShellDrawRole` gains RMG background/control roles.
- The modal interior-policy type is generalized from a Choose Map listbox name
  to a backdrop/control name.
- `push_random_map_setup_modal_instances` becomes a control-only emitter and
  receives the selected interior policy.

No public game API, serialized state, event schema, config schema, or
simulation interface changes.

## Risk Areas

- Four target app-render files contain the user's uncommitted Customize Battle
  restoration. Never replace them from `HEAD`, discard their diff, or recreate
  pre-extraction code.
- `MNSCRNS.SHP` must exist twice in the skirmish atlas with different labels and
  palettes. Reusing the current `background_640_mnscrns` entry would subtly
  decode `0x105` with the `0x102` palette.
- The shared profile helper serves `0x102` and `0x6B`; regression tests must
  preserve setup's `SDMPBTN` and Choose Map's lack of it.
- A fallback full-screen fill would cover the restored rail. The fallback rect
  must remain the common left canvas ending above the lower strip.
- Saved-seed Load/Save/Delete subdialogs reuse modal helpers but are out of
  scope. Do not change their lifecycle or backing policy.
- Preview and text are later passes. Do not move them into the sprite
  foundation or change their state source.

## Player-Experience Critical Items

Representative scenario: at 800x600, open offline Skirmish, open Customize
Battle, choose Create Random Map, inspect and operate the stable Generate Map
dialog, generate a preview, and return or cancel.

| Task | Class | Item | Why it matters | Verification |
|---|---|---|---|---|
| 1 | MILESTONE-BLOCKING | `MNSCRNL#0` decoded through `SHELL.PAL` | Defines almost the entire visible left canvas | Ignored retail decode test plus user capture |
| 2 | MILESTONE-BLOCKING | Common rail/lower strip and `SDTP#1`, no `SDMPBTN` | Restores the dominant metal frame without importing setup-only art | Semantic order regression |
| 3 | MILESTONE-BLOCKING | No opaque green dialog cover or broad combo plates | Keeps the red retail artwork visible | Interior-policy tests and visual acceptance |
| 4 | MILESTONE-BLOCKING | Controls/preview/text stay above the foundation | The screen must remain usable, not merely decorative | Existing RMG tests and user interaction |
| 4 | COMPOUNDING | Preserve `0x102`, `0x6B`, and saved-seed paths | Shared helpers can regress adjacent ordinary shell screens | Three profile/order tests |
| 5 | EXACTIFICATION-RESIDUAL | Show/close transition | Happens once per open/close; stable controls and state are unaffected | Record as unchecked, not a blocker |
| 5 | EXACTIFICATION-RESIDUAL | Exact combo RGB and >800 pixels | Presentation-only, bounded, and not authority-bearing | User 800x600 review; no parity claim |

---

## Tasks

### Task 1: Pack the Verified Generic Shell Backgrounds

**Why:** The RMG renderer cannot select the correct retail artwork until the
skirmish atlas owns `MNSCRNS/MNSCRNL` decoded through `SHELL.PAL`.

**Files:**

- Modify: `src/render/skirmish_shell_chrome.rs:33-52`
- Modify: `src/render/skirmish_shell_chrome.rs:315-365`
- Modify: `src/render/skirmish_shell_chrome.rs:447-468`
- Modify: `src/render/skirmish_shell_chrome.rs:557-580`
- Modify: `src/render/skirmish_shell_chrome.rs:927-1155`

**Pattern:** Mirror `MainMenuShellChromeAtlas`'s generic-background load and the
skirmish atlas's existing labeled multi-palette entries. This extends an
existing pattern.

**Step 1: Add role-specific atlas fields**

Add these fields without replacing `background_640_mnscrns`, which remains the
`0x102`/Coop-palette entry:

```rust
pub struct SkirmishShellChromeAtlas {
    // existing fields remain in their current order
    pub background_640_mnscrns: Option<SkirmishShellChromeEntry>,
    pub background_800_coop_game_setup: Option<SkirmishShellChromeEntry>,
    pub choose_map_background_800_customize_battle: Option<SkirmishShellChromeEntry>,
    /// Generic common-shell small background, decoded through SHELL.PAL.
    pub generic_background_640_mnscrns_shell: Option<SkirmishShellChromeEntry>,
    /// Generic common-shell large background, decoded through SHELL.PAL.
    pub generic_background_large_mnscrnl_shell: Option<SkirmishShellChromeEntry>,
    // existing lower-side and remaining fields follow
}
```

**Step 2: Pack frame 0 through the already-required shell palette**

Insert this after the common lower-strip assets and before the
dialog-specific palette blocks:

```rust
for (name, label) in [
    ("MNSCRNS.SHP", "mnscrns.shp#shell"),
    ("MNSCRNL.SHP", "mnscrnl.shp#shell"),
] {
    push_optional(
        &mut rendered,
        render_shp_entry_labeled(assets, name, label, &shell_palette, 0),
        name,
    );
}
```

This intentionally packs `MNSCRNS.SHP` a second time: the existing unlabeled
entry uses `MnScrnLCoopGameSetup.PAL`, while this labeled entry uses
`SHELL.PAL`.

**Step 3: Bind the packed entries**

Add these assignments to `SkirmishShellChromeAtlas` construction:

```rust
generic_background_640_mnscrns_shell: by_label
    .get("mnscrns.shp#shell")
    .copied(),
generic_background_large_mnscrnl_shell: by_label
    .get("mnscrnl.shp#shell")
    .copied(),
```

**Step 4: Promote the verified asset classification**

Update the test-only classifier:

```rust
"mnscrns.shp" | "mnscrnl.shp" | "mnscrnlcoopgamesetup.shp" => {
    ShellAssetRole::VerifiedParentBackground
}
```

Remove `"mnscrnl.shp"` from the `ResearchCandidate` arm. Update
`skirmish_shell_asset_classification_matches_live_render_path` to expect
`VerifiedParentBackground` for `MNSCRNL.SHP`.

**Step 5: Extend the ignored retail-asset oracle**

Add a focused ignored test:

```rust
#[test]
#[ignore]
fn retail_generic_shell_backgrounds_decode_with_shell_palette() {
    let config = crate::util::config::GameConfig::load().expect("game config");
    let assets = AssetManager::new(&config.paths.ra2_dir).expect("asset manager");
    let palette = load_named_palette(&assets, "SHELL.PAL").expect("SHELL.PAL");
    let small =
        render_shp_entry(&assets, "MNSCRNS.SHP", &palette, 0).expect("MNSCRNS frame 0");
    let large =
        render_shp_entry(&assets, "MNSCRNL.SHP", &palette, 0).expect("MNSCRNL frame 0");

    assert_eq!((small.width, small.height), (472, 448));
    assert_eq!((large.width, large.height), (632, 568));
}
```

**Step 6: Verify**

Run:

```powershell
cargo test -p vera20k skirmish_shell_asset_classification_matches_live_render_path -- --nocapture
cargo test -p vera20k retail_generic_shell_backgrounds_decode_with_shell_palette -- --ignored --nocapture
```

Expected: both named tests report `ok`; the retail test finds frame 0 of both
assets at the verified dimensions.

### Task 2: Define the `0x105` Chrome and Semantic Order Contract

**Why:** The renderer needs a testable contract before integration so shared
shell helpers cannot silently import `0x102`-only chrome.

**Files:**

- Modify: `src/app_skirmish_shell_render/draw_order.rs:12-178`
- Modify tests/imports: `src/app_skirmish_shell_render.rs:10-18`
- Modify tests: `src/app_skirmish_shell_render.rs:1430-1525`

**Pattern:** Extend the current `ShellDialogChromeProfile`,
`push_base_shell_roles`, and Choose Map semantic-order pattern. No new
architecture pattern.

**Step 1: Add the generic background role and selector**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GenericBackgroundRole {
    Mnscrns640,
    MnscrnlLarge,
}

pub(super) const fn generic_background_role(
    layout: &SkirmishShellLayout,
) -> GenericBackgroundRole {
    match layout.screen.w {
        640 => GenericBackgroundRole::Mnscrns640,
        _ => GenericBackgroundRole::MnscrnlLarge,
    }
}
```

**Step 2: Extend the profile without changing sibling behavior**

```rust
pub(super) enum ShellDialogChromeProfile {
    SkirmishSetup0x102,
    ChooseMap0x6b,
    RandomMapSetup0x105,
}

impl ShellDialogChromeProfile {
    pub(super) const fn draws_top_highlight(self) -> bool {
        true
    }

    pub(super) const fn draws_map_button(self) -> bool {
        matches!(self, Self::SkirmishSetup0x102)
    }
}
```

**Step 3: Add RMG semantic roles**

Add these variants to `SkirmishShellDrawRole`:

```rust
RandomMapBackgroundMnscrns640,
RandomMapBackgroundMnscrnlLarge,
RandomMapModalBackdrop,
RandomMapOptionControl,
RandomMapOwnerDrawButton,
RandomMapPreviewStatic,
```

**Step 4: Add the stable `0x105` semantic-order function**

```rust
pub fn random_map_setup_semantic_draw_order(
    layout: &SkirmishShellLayout,
    generic_background_available: bool,
) -> Vec<SkirmishShellDrawRole> {
    let mut roles = Vec::new();
    push_base_shell_roles(&mut roles, layout, false);
    if generic_background_available {
        roles.push(match generic_background_role(layout) {
            GenericBackgroundRole::Mnscrns640 => {
                SkirmishShellDrawRole::RandomMapBackgroundMnscrns640
            }
            GenericBackgroundRole::MnscrnlLarge => {
                SkirmishShellDrawRole::RandomMapBackgroundMnscrnlLarge
            }
        });
    } else {
        roles.push(SkirmishShellDrawRole::RandomMapModalBackdrop);
    }
    push_steady_optional_roles(&mut roles, ShellDialogChromeProfile::RandomMapSetup0x105);
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::RandomMapOptionControl).take(6));
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::RandomMapOwnerDrawButton).take(7));
    roles.push(SkirmishShellDrawRole::RandomMapPreviewStatic);
    roles
}
```

The six option roles are five combos plus the player trackbar. The seven button
roles are Surprise Me, Generate Map, Use Map, Load, Save, Delete, and Cancel.
Conditional progress widgets and an open dropdown are later overlays and do not
belong in the stable-order function.

**Step 5: Add an 800x600 regression test**

Import `random_map_setup_semantic_draw_order` under `#[cfg(test)]`, then add:

```rust
#[test]
fn random_map_setup_semantic_draw_order_uses_generic_shell_profile() {
    let layout = compute_layout(800, 600);
    let order = random_map_setup_semantic_draw_order(&layout, true);
    let background = order
        .iter()
        .position(|role| {
            *role == SkirmishShellDrawRole::RandomMapBackgroundMnscrnlLarge
        })
        .expect("RMG MNSCRNL role");
    let highlight = order
        .iter()
        .position(|role| {
            *role == SkirmishShellDrawRole::RightPanelTopHighlightSdtpFrame1
        })
        .expect("RMG SDTP frame-1 role");
    let first_control = order
        .iter()
        .position(|role| *role == SkirmishShellDrawRole::RandomMapOptionControl)
        .expect("RMG option-control role");

    assert_eq!(order[0], SkirmishShellDrawRole::RightPanelTopSdtp);
    assert_eq!(order[10], SkirmishShellDrawRole::RightPanelBottomSdbtm);
    assert_eq!(order[11], SkirmishShellDrawRole::LowerSideLwscrnl);
    assert!(background < highlight);
    assert!(highlight < first_control);
    assert!(!order.contains(&SkirmishShellDrawRole::RightPanelMapButtonSdmpbtn));
    assert!(!order.contains(&SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10));
    assert!(!order.contains(&SkirmishShellDrawRole::ChooseMapBackgroundCustomizeBattle800));

    let fallback = random_map_setup_semantic_draw_order(&layout, false);
    assert!(fallback.contains(&SkirmishShellDrawRole::RandomMapModalBackdrop));
    assert!(!fallback.contains(&SkirmishShellDrawRole::RandomMapBackgroundMnscrnlLarge));
}
```

Retain the existing setup and Choose Map semantic tests unchanged except for
imports required by the new function.

**Step 6: Verify**

Run:

```powershell
cargo test -p vera20k random_map_setup_semantic_draw_order_uses_generic_shell_profile -- --nocapture
cargo test -p vera20k choose_map_modal_semantic_draw_order_replaces_parent_shell -- --nocapture
cargo test -p vera20k semantic_draw_order_records_verified_right_panel_sequence -- --nocapture
```

Expected: all three tests report `ok`; setup still contains `SDMPBTN`, while
Choose Map and RMG do not.

### Task 3: Separate the RMG Foundation from Child Controls

**Why:** The correct asset and rail will still be invisible if the current
screen-sized green fill and unconditional combo plates remain above them.

**Files:**

- Modify: `src/app_skirmish_shell_render/modals.rs:53-190`
- Modify: `src/app_skirmish_shell_render/modals.rs:262-455`
- Modify tests: `src/app_skirmish_shell_render/modals.rs:600-639`

**Pattern:** Generalize the current Choose Map retail-art/fallback policy and
reuse `common_shell_origin` plus the bounded left-canvas fallback. This extends
the current uncommitted Customize Battle restoration.

**Step 1: Generalize the interior policy name**

Rename `ListboxInteriorPaint` to:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BackdropInteriorPaint {
    PreserveBacking,
    OpaqueFallback,
}

impl BackdropInteriorPaint {
    const fn paints_solid_fill(self) -> bool {
        matches!(self, Self::OpaqueFallback)
    }
}

const fn backdrop_interior(retail_background_available: bool) -> BackdropInteriorPaint {
    if retail_background_available {
        BackdropInteriorPaint::PreserveBacking
    } else {
        BackdropInteriorPaint::OpaqueFallback
    }
}
```

Update `push_choose_map_listbox_instances`,
`push_choose_map_background_instances`, and the saved-seed callsite to use the
new type. Their behavior must not change.

**Step 2: Generalize the bounded fallback helper**

Rename `choose_map_fallback_rect` to `shell_content_fallback_rect` and retain
its exact implementation:

```rust
pub(super) fn shell_content_fallback_rect(layout: &SkirmishShellLayout) -> RectPx {
    let (origin_x, origin_y) = common_shell_origin(layout);
    let shell_h = if layout.screen.h > 767 {
        600
    } else {
        layout.screen.h
    };
    RectPx::new(
        origin_x,
        origin_y,
        (layout.right_panel.top.x - origin_x).max(0),
        (shell_h - LOWER_STRIP_H).max(0),
    )
}
```

Update the Choose Map background helper and its tests to call the generalized
name.

**Step 3: Replace the wrong RMG background selector**

Import `GenericBackgroundRole` and `generic_background_role` from
`draw_order.rs`, then replace `random_map_setup_background_entry` with:

```rust
pub(super) fn random_map_setup_background_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> Option<SkirmishShellChromeEntry> {
    match generic_background_role(layout) {
        GenericBackgroundRole::Mnscrns640 => atlas.generic_background_640_mnscrns_shell,
        GenericBackgroundRole::MnscrnlLarge => atlas.generic_background_large_mnscrnl_shell,
    }
}
```

**Step 4: Add an RMG foundation helper**

```rust
pub(super) fn push_random_map_setup_background_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> BackdropInteriorPaint {
    let background = random_map_setup_background_entry(atlas, layout);
    let interior = backdrop_interior(background.is_some());
    if let Some(background) = background {
        let (x, y) = common_shell_origin(layout);
        push_entry_native(
            out,
            background,
            x,
            y,
            SHELL_PARENT_BACKGROUND_DEPTH,
        );
    } else {
        let fallback = shell_content_fallback_rect(layout);
        push_solid_rect(
            out,
            atlas,
            fallback,
            SHELL_MODAL_BG_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00008,
        );
        push_rect_outline(
            out,
            atlas,
            fallback,
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            SHELL_DROPDOWN_DEPTH - 0.00009,
        );
    }
    interior
}
```

**Step 5: Make the existing RMG emitter controls-only**

Rename it and add the selected policy:

```rust
pub(super) fn push_random_map_setup_modal_control_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &RandomMapSetupLayout,
    interior: BackdropInteriorPaint,
    modal: &RandomMapSetupModalState,
) {
    let chrome = atlas.control_chrome();
    // The existing control body follows.
}
```

Delete the old leading background lookup and the unconditional
`layout.dialog` solid fill/outline. Do not change player-trackbar, action-button,
right-panel-button, preview-frame, progress, or open-dropdown behavior.

Inside the five-combo loop, replace the unconditional face plate with:

```rust
if interior.paints_solid_fill() {
    push_solid_rect(
        out,
        atlas,
        face,
        SHELL_MODAL_PANEL_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00009,
    );
}
push_ownerdraw_two_pixel_bevel_frame(
    out,
    atlas,
    face,
    SHELL_DROPDOWN_DEPTH - 0.000095,
);
```

The bevel, arrow, disabled state, selected text, dropdown background, progress
background, and all buttons remain present. Only the known-wrong broad backing
plates become fallback-only.

**Step 6: Extend policy and geometry tests**

Update the existing Choose Map tests to use `BackdropInteriorPaint` and
`backdrop_interior`, then add:

```rust
#[test]
fn random_map_retail_art_preserves_control_backing() {
    let interior = backdrop_interior(true);

    assert_eq!(interior, BackdropInteriorPaint::PreserveBacking);
    assert!(!interior.paints_solid_fill());
}

#[test]
fn random_map_missing_art_uses_bounded_opaque_fallback() {
    let layout = crate::ui::skirmish_shell::compute_layout(800, 600);
    let interior = backdrop_interior(false);

    assert_eq!(interior, BackdropInteriorPaint::OpaqueFallback);
    assert!(interior.paints_solid_fill());
    assert_eq!(
        shell_content_fallback_rect(&layout),
        RectPx::new(0, 0, 632, 568)
    );
}
```

Keep the existing 1024 centered fallback assertion, renamed to the generalized
helper.

**Step 7: Verify**

Run:

```powershell
cargo test -p vera20k random_map_retail_art_preserves_control_backing -- --nocapture
cargo test -p vera20k random_map_missing_art_uses_bounded_opaque_fallback -- --nocapture
cargo test -p vera20k choose_map_retail_art_preserves_listbox_backing -- --nocapture
cargo test -p vera20k choose_map_fallback -- --nocapture
```

Expected: all policy and fallback geometry tests report `ok`.

### Task 4: Wire the Complete `0x105` Composition

**Why:** This integration closes the visible loop by placing the common shell,
generic background, optional chrome, and existing controls in verified order.

**Files:**

- Modify: `src/app_skirmish_shell_render.rs:205-246`
- Verify without behavior changes:
  `src/app_skirmish_shell_render/preview.rs`,
  `src/app_skirmish_shell_render/text.rs`,
  `src/ui/skirmish_shell/state/random_map_setup.rs`

**Pattern:** Match the current working-tree Choose Map branch, substituting the
`0x105` profile and RMG background/control helpers.

**Step 1: Replace only the RMG branch body**

Replace the current `else if let Some(modal)` body with:

```rust
} else if let Some(modal) = shell.random_map_setup_modal.as_ref() {
    let setup_layout = compute_random_map_setup_layout(
        choose_map_layout.screen.w as u32,
        choose_map_layout.screen.h as u32,
    );
    push_right_panel_base_instances(&mut instances, atlas, layout, 0, false);
    push_lower_strip_instance(&mut instances, atlas, layout);
    let interior =
        push_random_map_setup_background_instances(&mut instances, atlas, layout);
    push_steady_optional_chrome_instances(
        &mut instances,
        atlas,
        layout,
        ShellDialogChromeProfile::RandomMapSetup0x105,
    );
    push_random_map_setup_modal_control_instances(
        &mut instances,
        atlas,
        &setup_layout,
        interior,
        modal,
    );
}
```

Leave the saved-seed branch and ordinary Choose Map branch unchanged. The outer
modal return remains in place so `0x102` is not painted underneath.

**Step 2: Confirm later passes remain authoritative**

Read the final diff and verify:

- generated RMG preview still comes from the existing setup-modal preview path;
- `push_random_map_setup_modal_text_draws` is still called for the active modal;
- cursor rendering remains last;
- no RMG state, command, generation, file, or saved-seed code changed.

No source edits are expected in `preview.rs`, `text.rs`,
`random_map_setup.rs`, `app.rs`, or `src/map/rmg/`.

**Step 3: Run the focused UI/RMG regression group**

Run:

```powershell
cargo test -p vera20k random_map_setup -- --nocapture
cargo test -p vera20k choose_map_modal -- --nocapture
cargo test -p vera20k saved_seed -- --nocapture
```

Expected: every literal `test result:` line reports zero failures. Existing
generation, cancel, preview, combo, trackbar, and saved-seed tests remain green.

### Task 5: Format, Build, and Perform the User Visual Handoff

**Why:** Rendering work is complete only when focused regressions, compilation,
and the real 800x600 player path all agree.

**Files:**

- Format only the edited Rust files.
- Do not commit, stage, push, or modify unrelated dirty files.

**Step 1: Inspect the final scoped diff**

Run:

```powershell
git diff -- src/render/skirmish_shell_chrome.rs src/app_skirmish_shell_render.rs src/app_skirmish_shell_render/draw_order.rs src/app_skirmish_shell_render/modals.rs src/app_skirmish_shell_render/chrome.rs
```

Confirm the pre-existing Customize Battle rail extraction remains present and
that no unrelated RMG backend changes entered this slice.

**Step 2: Format only edited Rust files**

Run:

```powershell
rustfmt --edition 2024 src/render/skirmish_shell_chrome.rs src/app_skirmish_shell_render.rs src/app_skirmish_shell_render/draw_order.rs src/app_skirmish_shell_render/modals.rs
```

Inspect the scoped diff again for accidental churn.

**Step 3: Coordinate Cargo**

Run:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
```

If another session owns Cargo, wait for it to finish. Do not start parallel
Cargo commands.

**Step 4: Run final validation serially**

Run:

```powershell
cargo test -p vera20k random_map_setup -- --nocapture
cargo test -p vera20k choose_map_modal_semantic_draw_order_replaces_parent_shell -- --nocapture
cargo test -p vera20k semantic_draw_order_records_verified_right_panel_sequence -- --nocapture
cargo check -q
```

Report the literal `test result:` lines and `cargo check` exit code.

**Step 5: User visual acceptance**

Do not use Windows app control. Ask the user to run the game at 800x600 and
capture the stable Generate Map dialog with:

- the red world-map artwork visible across the left 632x568 canvas;
- a continuous metal title rail, button bays, lower side strip, and Cancel
  footer;
- `SDTP#1` title highlight and no stray `SDMPBTN`;
- all five option combos and the player trackbar readable;
- Generate/Surprise Me and right-panel buttons functional;
- a generated preview visible after generation;
- Cancel returning without committing and Use Map preserving current behavior.

Classify the result as retail-convincing if those conditions pass. Do not claim
native pixel parity; exact transition frames, final combo RGB, and non-800
pixels remain recorded residuals.

## Sources & References

- **Design doc:**
  `docs/plans/2026-07-27-random-map-generator-ui-restoration-design.md`
- **Primary background evidence:**
  `docs/research/FUN_0060CF00_DIALOG_BACKGROUND_POINTER_TABLE_GHIDRA_REPORT.md`
  - High confidence; full decompile plus assembly per branch.
- **Common parent composition:**
  `docs/research/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`
  - High confidence for the named common paint slice; first-paint ready-gate
    value is the report's bounded medium-confidence detail.
- **Rail assets/palettes/order:**
  `docs/research/skirmish-ui/SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`
  - High confidence for SHP/PAL mapping and draw order.
- **Corrected `0x105` geometry:**
  `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_DIALOG_0X105_LAYOUT_GEOMETRY_GHIDRA_REPORT.md`
  - High confidence for PE resource geometry; 2026-07-27 correction supersedes
    its earlier `0x6B` background inference.
- **RMG lifecycle/controls:**
  `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_SETUP_DIALOG_CONTROLS_OPTIONS_GHIDRA_REPORT.md`
  - High confidence for active dialog path, controls, result values, and
    generation gates.
- **Generic background repo analogue:**
  `docs/research/MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
  - High confidence for generic `MNSCRNS/MNSCRNL` plus `SHELL.PAL`.
- **Combo backing evidence:**
  `docs/research/skirmish-ui/SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md`
  - High confidence that native painting uses backing composition and the
    current solid RGB is unverified; exact final pixels remain uncaptured.
- **Optional chrome field map:**
  `docs/research/substrate/worknotes/gadget-dialog-20260610/dialog-delta.md`
  - Verified worknote mapping `data+0xD5` to top highlight and `data+0xD6` to
    minimap button.
- **Fresh binary checks:**
  `gamemd.exe` `0x0060CF00`, `0x00622820`, and `0x00595BC0`.
- **Retail assets:**
  `MNSCRNS.SHP`, `MNSCRNL.SHP`, `SHELL.PAL`, `SDTP.SHP`,
  `SDBTNBKGD.SHP`, `SDBTM.SHP`, `LWSCRNL.SHP`, `LWSCRNS.SHP`.
- **INI keys:** none.
- **Related code:**
  `src/render/main_menu_shell_chrome.rs`,
  `src/render/skirmish_shell_chrome.rs`,
  `src/app_skirmish_shell_render.rs`,
  `src/app_skirmish_shell_render/chrome.rs`,
  `src/app_skirmish_shell_render/draw_order.rs`,
  `src/app_skirmish_shell_render/modals.rs`.
- **Relevant historical commits:**
  `470fae54` (native skirmish shell),
  `faec3184` (initial RMG modal chrome),
  `97dcb32b` (RMG combo/chrome fill), and the current uncommitted Customize
  Battle rail restoration that must be preserved.
