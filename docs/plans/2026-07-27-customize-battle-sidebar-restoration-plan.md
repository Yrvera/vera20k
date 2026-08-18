# Customize Battle Sidebar Restoration Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Restore the complete retail right-hand shell rail on the active
800x600 Customize Battle screen while preserving the orange artwork, modal
behavior, and existing quality-of-life controls.

**Architecture:** Keep shell ownership in the app-render layer. Extract the
already-working Skirmish right-panel layers into shared emitters, select
verified steady optional layers through a dialog profile, and compose the
Choose Map background and controls between those layers. No simulation, UI
state, input, or asset-loading authority changes.

**Design Doc:** `docs/plans/2026-07-27-customize-battle-sidebar-restoration-design.md`

---

## Grounding Summary

- The user's 800x600 capture shows the modal's 168-pixel right region reserved
  but black, with only preview and button controls floating over it. The retail
  reference shows continuous metallic chrome and a lower strip.
- Current Rust confirms the mechanism: `build_skirmish_shell_instances`
  returns from the modal branch at
  `src/app_skirmish_shell_render.rs:204-224`, before the normal rail starts at
  line 227.
- Dialog `0x6B` is active standard-YR Choose Map and participates in common
  fullscreen shell setup. Its `MnScrnLCustomizeBattle` background binding is
  distinct from setup `0x102`. Source:
  `SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md`.
- Common parent paint composes the right-panel base before the
  dialog-specific background and optional steady chrome. Source:
  `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`,
  `WM_PAINT_Handler @ 0x00621E90`, `RightPanel__Draw @ 0x0072E450`,
  and `Background_Overlay @ 0x0072E730`.
- Verified base order is `SDTP#0`, repeated `SDBTNBKGD#0`, optional repeated
  `SDBTNANM#10`, source-clipped `SDBTM#0`, then
  `LWSCRNS/LWSCRNL#0`.
- Dialog-flag evidence distinguishes steady profiles: `0x102` has D9=1 and
  DA=1; `0x6B` has D9=1 and DA=0. Therefore both show `SDTP#1`, but only
  setup shows `SDMPBTN#0`. Source:
  `SHELL_DIALOG_LIFETIME_TRANSITION_EVIDENCE_GATES_GHIDRA_REPORT.md`,
  Gate 4.
- Retail asset inspection confirms: `SDTP` 168x199 with 2 frames,
  `SDBTNBKGD` 168x42 with 1 frame, `SDBTM` 168x65 with 1 frame,
  `SDBTNANM` 156x42 with 17 frames, `SDMPBTN` 156x84 with 7 frames,
  `SDWRNTMP` 168x177 with 6 frames, and `LWSCRNL` 632x32 with 1 frame.
- `src/render/skirmish_shell_chrome.rs` already packs every stable asset/frame
  required here. No new asset or palette field is needed.
- The closest repo pattern is the existing normal Skirmish shell composition
  in `src/app_skirmish_shell_render.rs` plus geometry helpers in
  `src/app_skirmish_shell_render/chrome.rs`.
- No INI key drives this composition. Asset names, frames, palette roles,
  dialog id, and geometry come from retail shell resources and verified binary
  paths.
- Consequential stable-state uncertainties are resolved. Deferred presentation
  residuals are the `0x6B` D9 show/close animation and exact non-800
  background output.

## Key Technical Decisions

- Use one shared base-rail emitter and a small dialog profile instead of
  duplicating rail code. — **Confidence: high**
  - **Source:** current Rust normal-shell pattern; Ghidra
    `RightPanel__Draw @ 0x0072E450`.
- Preserve the current normal setup order while extracting helpers: base rail,
  player-name edit, lower strip, parent background, steady optional chrome,
  then child controls. — **Confidence: high**
  - **Source:** current Rust
    `src/app_skirmish_shell_render.rs:227-305`; semantic order tests.
- Compose stable `0x6B` as base rail and lower strip, modal background,
  `SDTP#1`, then modal controls; omit `SDMPBTN#0`. — **Confidence: high**
  - **Source:**
    `SHELL_DIALOG_LIFETIME_TRANSITION_EVIDENCE_GATES_GHIDRA_REPORT.md`,
    Gate 4; user retail capture.
- Keep `SDBTNANM#10` row overlay disabled for both ordinary steady profiles.
  — **Confidence: high**
  - **Source:**
    `SKIRMISH_RIGHT_PANEL_SDBTNANM_FRAME10_STATE_FLAG_GHIDRA_REPORT.md`;
    current `right_panel_frame10_overlay_active`.
- Bound the primitive modal fallback to the 632x568 left content region so it
  cannot cover the rail or lower strip. — **Confidence: high**
  - **Source:** shared `right_panel_rects`/`common_shell_origin` geometry;
    renderer fallback policy from the approved design.
- Do not add `SDWRNTMP` transition support in this slice. — **Confidence: high**
  - **Source:** approved design scope; stable screenshot is the acceptance
    target and transition drift is recorded explicitly.

## Open Questions

### Resolved During Planning

- Does `MnScrnLCustomizeBattle.SHP` contain the whole rail? No. It is the
  dialog-specific background selected after the common rail composition;
  current runtime proves its native image ends where the reserved rail begins.
- Can the normal `0x102` rail be reused unchanged? No. Base rail and top
  highlight are shared, but `SDMPBTN#0` is setup-only because `0x6B` has DA=0.
- Are new atlas entries required? No. Stable `0x6B` uses fields already present
  in `SkirmishShellChromeAtlas`.
- Does the fix require modal-state or input changes? No. The symptom occurs
  entirely before child-control emission in the render planner.

### Deferred to Implementation

- Manual visual confirmation remains necessary to prove aggregate layer
  overlap and palette appearance on the user's retail assets. The user will
  perform this check without Windows app control.
- The 11-tick D9 `SDWRNTMP` show/close sequence remains outside this stable
  composition slice.
- Exact native pixels above logical width 800 remain unclaimed; the
  implementation preserves native-sized helper anchoring and does not stretch
  assets.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/app_skirmish_shell_render/draw_order.rs` | Dialog chrome profile and semantic layer-order contracts |
| Modify | `src/app_skirmish_shell_render/chrome.rs` | Shared base rail, lower strip, and steady optional-chrome sprite emitters |
| Modify | `src/app_skirmish_shell_render/modals.rs` | Modal background/fallback separated from modal child controls |
| Modify | `src/app_skirmish_shell_render.rs` | Setup and Choose Map integration plus focused regression tests |

No new production file is required. `src/render/skirmish_shell_chrome.rs`,
`src/ui/skirmish_shell/layout.rs`, `src/app.rs`,
`src/app_skirmish_shell_render/text.rs`, and modal state remain read-only
dependencies for this implementation.

## Interface Changes

- Add private `ShellDialogChromeProfile` in
  `app_skirmish_shell_render::draw_order`.
- Add private renderer helpers in `app_skirmish_shell_render::chrome`.
- Change crate-visible
  `choose_map_modal_semantic_draw_order(bool)` to
  `choose_map_modal_semantic_draw_order(&SkirmishShellLayout, bool)`.
  Current repo search finds only the renderer test module and re-export as
  consumers.
- Change private `push_choose_map_modal_instances` into a controls-only helper
  that accepts an explicit `ListboxInteriorPaint`.
- Add private `push_choose_map_background_instances`, which returns the
  listbox interior policy selected by the background/fallback result.

No public data type, persistent format, asset schema, input action, or UI state
changes.

## Risk Areas

- Extracting the normal rail can regress setup `0x102` ordering, wave top-cap
  displacement, bottom-cap UV clipping, or `SDMPBTN` visibility.
- An over-broad fallback rectangle can re-cover the newly restored rail.
- Drawing `SDTP#1` before the modal background can hide or flatten the retail
  top frame.
- Reusing the complete setup profile can incorrectly add `SDMPBTN#0` to
  Choose Map.
- Moving child controls beneath optional chrome can obscure the preview or
  buttons.
- The working tree already contains the approved orange-background changes in
  `src/app_skirmish_shell_render.rs` and
  `src/app_skirmish_shell_render/modals.rs`; implementation must preserve and
  build on them, not replace them.

## Player-Experience Critical Items

Representative scenario: launch ordinary offline Skirmish at logical 800x600,
open Choose Map, browse an overflowing retail map list, inspect the preview,
press and release Use Map or Random Map, and cancel back to setup.

| Task # | Class | Item | Why it matters | Verification |
|---|---|---|---|---|
| 2-4 | MILESTONE-BLOCKING | Continuous top/tile/bottom rail | Dominant missing visual every time Choose Map opens | Semantic order test and user screenshot |
| 2-4 | MILESTONE-BLOCKING | `LWSCRNL` at `(0,568,632,32)` | Removes the black lower gap and restores footer framing | Geometry test and screenshot |
| 2-4 | MILESTONE-BLOCKING | Source-clipped `SDBTM` bottom cap | Prevents a visibly squeezed footer | Existing UV crop test plus setup regression |
| 1,4 | MILESTONE-BLOCKING | `0x6B` gets `SDTP#1` but no `SDMPBTN#0` | Distinguishes modal chrome from setup chrome | Profile and semantic-order assertions |
| 3-4 | MILESTONE-BLOCKING | Orange background remains visible | Previous restoration must not regress | Retail/fallback tests and screenshot |
| 3-4 | MILESTONE-BLOCKING | Controls remain above chrome | Preview, selection, scrolling, and buttons must stay readable and usable | Focused Choose Map tests and manual input check |
| 2,4 | COMPOUNDING | Setup `0x102` profile remains unchanged | Returning from modal must not regress the parent shell | Existing setup semantic-order suite |
| 3 | COMPOUNDING | Fallback cannot cover rail/lower strip | Missing modal art must still yield a coherent shell | Pure fallback geometry tests |
| 5 | EXACTIFICATION-RESIDUAL | D9 transition animation unchanged | Brief open/close motion differs but stable UI and state are correct | Record residual; do not claim exact transition parity |
| 5 | EXACTIFICATION-RESIDUAL | Non-800 background policy | Unverified widths may not match native aggregate pixels | Preserve native sizes; report UNCHECKED |

---

## Tasks

### Task 1: Define the dialog chrome profile and semantic order

**Why:** Establish the verified `0x102` versus `0x6B` presentation contract
before production emitters consume it.

**Files:**

- Modify: `src/app_skirmish_shell_render/draw_order.rs:25-133`
- Modify: `src/app_skirmish_shell_render.rs:1408-1513` (tests and updated
  function call)

**Pattern:** Extend the existing semantic draw-role model; no new public
renderer abstraction.

**Step 1: Add the private profile and verified optional-layer predicates**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellDialogChromeProfile {
    SkirmishSetup0x102,
    ChooseMap0x6b,
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

Place this after `LowerStripRole`. Keep it private to the
`app_skirmish_shell_render` module tree.

Because the production renderer will select a profile outside tests, add a
non-test import in `src/app_skirmish_shell_render.rs`:

```rust
use self::draw_order::ShellDialogChromeProfile;
```

Keep the existing `LowerStripRole`, `ParentBackgroundRole`,
`lower_strip_role`, and `parent_background_role` import behind `#[cfg(test)]`.
Do not add the private profile to the module's public re-export list.

**Step 2: Extract shared semantic role helpers**

```rust
fn push_base_shell_roles(
    roles: &mut Vec<SkirmishShellDrawRole>,
    layout: &SkirmishShellLayout,
    overlay_frame10_active: bool,
) {
    roles.push(SkirmishShellDrawRole::RightPanelTopSdtp);
    roles.extend(
        std::iter::repeat(SkirmishShellDrawRole::RightPanelTileSdbtnbkgd)
            .take(layout.right_panel.tile_count.max(0) as usize),
    );
    if overlay_frame10_active {
        roles.extend(
            std::iter::repeat(SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10)
                .take(layout.right_panel.tile_count.max(0) as usize),
        );
    }
    roles.push(SkirmishShellDrawRole::RightPanelBottomSdbtm);
    roles.push(match lower_strip_role(layout) {
        LowerStripRole::Lwscrns640 => SkirmishShellDrawRole::LowerSideLwscrns,
        LowerStripRole::LwscrnlLarge => SkirmishShellDrawRole::LowerSideLwscrnl,
    });
}

fn push_steady_optional_roles(
    roles: &mut Vec<SkirmishShellDrawRole>,
    profile: ShellDialogChromeProfile,
) {
    if profile.draws_top_highlight() {
        roles.push(SkirmishShellDrawRole::RightPanelTopHighlightSdtpFrame1);
    }
    if profile.draws_map_button() {
        roles.push(SkirmishShellDrawRole::RightPanelMapButtonSdmpbtn);
    }
}
```

Use `push_base_shell_roles` at the start of
`skirmish_shell_semantic_draw_order`. Preserve parent-background placement,
then call `push_steady_optional_roles` with
`SkirmishSetup0x102` before the three setup owner-draw button roles.

**Step 3: Update the Choose Map semantic contract**

Replace the existing function with:

```rust
pub fn choose_map_modal_semantic_draw_order(
    layout: &SkirmishShellLayout,
    customize_battle_background_available: bool,
) -> Vec<SkirmishShellDrawRole> {
    let mut roles = Vec::new();
    push_base_shell_roles(&mut roles, layout, false);
    if customize_battle_background_available {
        roles.push(SkirmishShellDrawRole::ChooseMapBackgroundCustomizeBattle800);
    } else {
        roles.push(SkirmishShellDrawRole::ChooseMapModalBackdrop);
    }
    push_steady_optional_roles(&mut roles, ShellDialogChromeProfile::ChooseMap0x6b);
    roles.extend(std::iter::repeat(SkirmishShellDrawRole::ChooseMapListbox).take(2));
    roles.extend(
        std::iter::repeat(SkirmishShellDrawRole::ChooseMapOwnerDrawButton).take(3),
    );
    roles.push(SkirmishShellDrawRole::ChooseMapPreviewStatic);
    roles
}
```

**Step 4: Strengthen semantic tests**

Update `choose_map_modal_semantic_draw_order_replaces_parent_shell` to pass
`&compute_layout(800, 600)` and assert:

```rust
let layout = compute_layout(800, 600);
let order = choose_map_modal_semantic_draw_order(&layout, true);
assert_eq!(order[0], SkirmishShellDrawRole::RightPanelTopSdtp);
assert_eq!(
    &order[1..10],
    [SkirmishShellDrawRole::RightPanelTileSdbtnbkgd; 9]
);
assert_eq!(order[10], SkirmishShellDrawRole::RightPanelBottomSdbtm);
assert_eq!(order[11], SkirmishShellDrawRole::LowerSideLwscrnl);
assert_eq!(
    order[12],
    SkirmishShellDrawRole::ChooseMapBackgroundCustomizeBattle800
);
assert_eq!(
    order[13],
    SkirmishShellDrawRole::RightPanelTopHighlightSdtpFrame1
);
assert!(!order.contains(&SkirmishShellDrawRole::RightPanelMapButtonSdmpbtn));
assert!(
    !order.contains(&SkirmishShellDrawRole::RightPanelOverlaySdbtnanmFrame10)
);
assert!(!order.contains(&SkirmishShellDrawRole::OwnerDrawButton));
```

Update the fallback call to
`choose_map_modal_semantic_draw_order(&layout, false)`. Preserve the existing
normal setup test and its assertion that `RightPanelMapButtonSdmpbtn` remains
present.

**Step 5: Verify**

Run:

```powershell
cargo test -q --lib choose_map_modal_semantic_draw_order_replaces_parent_shell -- --nocapture
cargo test -q --lib semantic_draw_order_records_verified_right_panel_sequence -- --nocapture
```

Expected: both focused tests report `test result: ok`.

### Task 2: Extract shared production rail emitters without changing setup

**Why:** Make the verified common rail reusable while first proving the
existing `0x102` path remains byte-for-byte equivalent in instance order and
geometry.

**Files:**

- Modify: `src/app_skirmish_shell_render/chrome.rs:1-17, 560-639`
- Modify: `src/app_skirmish_shell_render.rs:227-305`

**Pattern:** Move the current inline instance emission into focused helpers in
the existing chrome module. Preserve wave-specific top-cap displacement as an
explicit parameter.

**Step 1: Import the profile and lower-strip depth**

Extend imports in `chrome.rs`:

```rust
use super::draw_order::{
    LowerStripRole, ParentBackgroundRole, ShellDialogChromeProfile, lower_strip_role,
    parent_background_role,
};
use super::{
    BUTTON_DISABLED_ALPHA, OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
    OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7, PRESSED_BUTTON_CONTENT_OFFSET_Y,
    SHELL_LOWER_STRIP_DEPTH,
};
```

Add named depths near the imports:

```rust
const RIGHT_PANEL_TOP_DEPTH: f32 = 0.00080;
const RIGHT_PANEL_TILE_DEPTH: f32 = 0.00079;
const RIGHT_PANEL_OVERLAY_DEPTH: f32 = 0.000785;
const RIGHT_PANEL_BOTTOM_DEPTH: f32 = 0.00078;
```

**Step 2: Add the shared base rail emitter**

```rust
pub(super) fn push_right_panel_base_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    top_offset_x: i32,
    overlay_frame10_active: bool,
) {
    if let Some(top) = atlas.right_panel_top_sdtp {
        push_entry(
            out,
            top,
            layout.right_panel.top.translate(top_offset_x, 0),
            RIGHT_PANEL_TOP_DEPTH,
        );
    }
    if let Some(tile) = atlas.right_panel_tile_sdbtnbkgd {
        for row in 0..layout.right_panel.tile_count {
            push_entry(
                out,
                tile,
                RectPx::new(
                    layout.right_panel.tile.x,
                    layout.right_panel.tile.y + row * layout.right_panel.tile.h,
                    layout.right_panel.tile.w,
                    layout.right_panel.tile.h,
                ),
                RIGHT_PANEL_TILE_DEPTH,
            );
        }
    }
    if overlay_frame10_active {
        if let Some(overlay) = atlas.right_panel_overlay_sdbtnanm_frame10 {
            for row in 0..layout.right_panel.tile_count {
                push_entry(
                    out,
                    overlay,
                    right_panel_overlay_rect(layout, row, overlay),
                    RIGHT_PANEL_OVERLAY_DEPTH,
                );
            }
        }
    }
    if let Some(bottom) = atlas.right_panel_bottom_sdbtm {
        push_entry_top_clipped_native(
            out,
            bottom,
            layout.right_panel.bottom,
            RIGHT_PANEL_BOTTOM_DEPTH,
        );
    }
}
```

**Step 3: Add separate lower-strip and steady optional emitters**

```rust
pub(super) fn push_lower_strip_instance(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) {
    if let Some(lower_strip) = lower_strip_entry(atlas, layout) {
        push_entry(
            out,
            lower_strip,
            lower_strip_rect(layout, lower_strip),
            SHELL_LOWER_STRIP_DEPTH,
        );
    }
}

pub(super) fn push_steady_optional_chrome_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    profile: ShellDialogChromeProfile,
) {
    if profile.draws_top_highlight() {
        if let Some(top_highlight) = atlas.right_panel_top_highlight_sdtp_frame1 {
            push_entry(
                out,
                top_highlight,
                layout.right_panel.top,
                SHELL_LOWER_STRIP_DEPTH - 0.00001,
            );
        }
    }
    if profile.draws_map_button() {
        if let Some(sdmpbtn) = atlas.sd_map_button {
            push_entry(
                out,
                sdmpbtn,
                sdmpbtn_rect(layout, sdmpbtn),
                SHELL_LOWER_STRIP_DEPTH - 0.00002,
            );
        }
    }
}
```

**Step 4: Rewire only normal setup**

In `build_skirmish_shell_instances`, compute the existing top displacement:

```rust
let top_offset_x = if wave.is_some_and(|wave| !wave.is_complete())
    && layout.screen.w >= RADAR_TRANSITION_MIN_WIDTH
{
    RADAR_TRANSITION_SHIFT_PX
} else {
    0
};
push_right_panel_base_instances(
    &mut instances,
    atlas,
    layout,
    top_offset_x,
    right_panel_frame10_overlay_active(shell),
);
```

Then preserve the existing order:

```rust
push_player_name_edit_instances(&mut instances, atlas, font, layout, shell);
push_lower_strip_instance(&mut instances, atlas, layout);

if let Some(background) = parent_background_entry(atlas, layout) {
    push_entry_native(
        &mut instances,
        background,
        layout.screen.x,
        layout.screen.y,
        SHELL_PARENT_BACKGROUND_DEPTH,
    );
}

push_steady_optional_chrome_instances(
    &mut instances,
    atlas,
    layout,
    ShellDialogChromeProfile::SkirmishSetup0x102,
);
```

Delete only the replaced inline top/tile/overlay/bottom/lower-strip/top-highlight
and `SDMPBTN` blocks. Leave owner-draw buttons, wave frame selection, combos,
checkboxes, trackbars, flags, preview, and text untouched.

**Step 5: Preserve the crop regression**

Keep the existing `push_entry_top_clipped_native` tests. Confirm the
`168x65` fixture still produces destination size `168x23` and UV height ratio
`23/65`, not a stretched full-source UV.

**Step 6: Verify**

Run:

```powershell
cargo test -q --lib semantic_draw_order_records_verified_right_panel_sequence -- --nocapture
cargo test -q --lib push_entry_top_clipped_native -- --nocapture
cargo test -q --lib lower_strip_rect_uses_native_asset_size_at_screen_bottom -- --nocapture
```

Expected: all focused tests report `test result: ok`.

### Task 3: Split modal foundation from modal child controls

**Why:** Let the outer renderer insert `SDTP#1` between the modal background
and modal controls, and ensure a missing-art fallback cannot cover the rail.

**Files:**

- Modify: `src/app_skirmish_shell_render/modals.rs:7-18, 55-224, 568-end`

**Pattern:** Continue the existing explicit `RetailArtwork` versus
`OpaqueFallback` policy. Keep presentation availability out of modal state.

**Step 1: Import shell geometry**

Extend the UI imports with `SkirmishShellLayout`, and add:

```rust
use crate::ui::shell::geom::LOWER_STRIP_H;
```

Extend the local chrome imports with `common_shell_origin`.

**Step 2: Add a pure bounded fallback rect**

```rust
pub(super) fn choose_map_fallback_rect(layout: &SkirmishShellLayout) -> RectPx {
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

This produces `(0,0,632,568)` at 800x600 and
`(112,84,632,568)` at 1024x768.

**Step 3: Add the background/fallback emitter**

```rust
pub(super) fn push_choose_map_background_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    shell_layout: &SkirmishShellLayout,
    modal_layout: &ChooseMapModalLayout,
) -> ListboxInteriorPaint {
    let background = choose_map_background_entry(atlas, modal_layout);
    let interior = choose_map_listbox_interior(background.is_some());
    if let Some(background) = background {
        push_entry_native(
            out,
            background,
            modal_layout.screen.x,
            modal_layout.screen.y,
            SHELL_PARENT_BACKGROUND_DEPTH,
        );
    } else {
        let fallback = choose_map_fallback_rect(shell_layout);
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

**Step 4: Make the existing modal helper controls-only**

Rename it to `push_choose_map_modal_control_instances` and add
`interior: ListboxInteriorPaint` before `shell`. Remove its local background
lookup and base-layer block. Keep:

- mode/map selection derivation;
- two `push_choose_map_listbox_instances` calls using `interior`;
- three `push_right_panel_button_shp` calls;
- preview outline.

Do not alter the random-map setup or saved-seed functions in this task.

**Step 5: Add pure geometry tests**

Add:

```rust
#[test]
fn choose_map_fallback_stops_before_800_rail_and_lower_strip() {
    let layout = crate::ui::skirmish_shell::compute_layout(800, 600);
    assert_eq!(choose_map_fallback_rect(&layout), RectPx::new(0, 0, 632, 568));
}

#[test]
fn choose_map_fallback_preserves_centered_1024_shell_geometry() {
    let layout = crate::ui::skirmish_shell::compute_layout(1024, 768);
    assert_eq!(
        choose_map_fallback_rect(&layout),
        RectPx::new(112, 84, 632, 568)
    );
}
```

Keep the two existing listbox-interior tests unchanged.

**Step 6: Verify**

Run:

```powershell
cargo test -q --lib choose_map_fallback_ -- --nocapture
cargo test -q --lib choose_map_retail_art_preserves_listbox_backing -- --nocapture
cargo test -q --lib choose_map_missing_art_uses_opaque_listbox_fallback -- --nocapture
```

Expected: all focused tests report `test result: ok`.

### Task 4: Integrate the complete stable `0x6B` composition

**Why:** Close the actual player-visible modal paint sequence after the shared
foundation and fallback contracts are independently pinned.

**Files:**

- Modify: `src/app_skirmish_shell_render.rs:204-224, 1492-1513`

**Pattern:** Preserve the existing separate-modal ownership and early return.
Only the ordinary Choose Map branch receives the new stable foundation in this
slice; random-map setup and saved-seed composition remain unchanged.

**Step 1: Compose the ordinary Choose Map branch**

Keep the existing saved-seed and random-map setup branches. Replace the final
ordinary Choose Map `else` body with:

```rust
push_right_panel_base_instances(
    &mut instances,
    atlas,
    layout,
    0,
    false,
);
push_lower_strip_instance(&mut instances, atlas, layout);
let interior = push_choose_map_background_instances(
    &mut instances,
    atlas,
    layout,
    choose_map_layout,
);
push_steady_optional_chrome_instances(
    &mut instances,
    atlas,
    layout,
    ShellDialogChromeProfile::ChooseMap0x6b,
);
push_choose_map_modal_control_instances(
    &mut instances,
    atlas,
    choose_map_layout,
    interior,
    shell,
    modes,
);
```

Retain the modal-branch `return instances;`. This continues suppressing setup
player controls, setup buttons, setup background, flags, combos, checkboxes,
and trackbars while `0x6B` is active.

**Step 2: Confirm production order against the semantic contract**

The instance path must now mirror:

```text
SDTP#0
9 x SDBTNBKGD#0
SDBTM#0 cropped
LWSCRNL#0
MnScrnLCustomizeBattle#0 or bounded fallback
SDTP#1
two listboxes
three SDBTNANM owner-draw buttons
preview frame
```

Do not add `SDMPBTN#0`, repeated `SDBTNANM#10`, setup parent background, or
setup owner-draw controls.

**Step 3: Update the semantic test indices**

Use the Task 1 order assertions and additionally locate the first listbox role:

```rust
let first_listbox = order
    .iter()
    .position(|role| *role == SkirmishShellDrawRole::ChooseMapListbox)
    .expect("Choose Map listbox role");
assert!(first_listbox > 13);
assert_eq!(
    &order[first_listbox..first_listbox + 2],
    [SkirmishShellDrawRole::ChooseMapListbox; 2]
);
assert_eq!(
    &order[first_listbox + 2..first_listbox + 5],
    [SkirmishShellDrawRole::ChooseMapOwnerDrawButton; 3]
);
```

**Step 4: Run the focused Choose Map suite**

First check build ownership:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

If no other session owns Cargo, run:

```powershell
cargo test -q --lib choose_map_ -- --nocapture
```

Expected: literal `test result: ok` with zero failures. Report the literal
pass/fail/ignored line rather than inferring success from command exit alone.

### Task 5: Format, compile, build, and hand off manual visual acceptance

**Why:** Validate both structural correctness and the actual retail-facing
screen without using Windows app control.

**Files:**

- Format only:
  - `src/app_skirmish_shell_render.rs`
  - `src/app_skirmish_shell_render/chrome.rs`
  - `src/app_skirmish_shell_render/draw_order.rs`
  - `src/app_skirmish_shell_render/modals.rs`

**Pattern:** Project-standard focused formatting and serial Cargo coordination.

**Step 1: Format only edited Rust files**

```powershell
rustfmt --edition 2024 `
    src/app_skirmish_shell_render.rs `
    src/app_skirmish_shell_render/chrome.rs `
    src/app_skirmish_shell_render/draw_order.rs `
    src/app_skirmish_shell_render/modals.rs
```

Inspect:

```powershell
git diff --check -- `
    src/app_skirmish_shell_render.rs `
    src/app_skirmish_shell_render/chrome.rs `
    src/app_skirmish_shell_render/draw_order.rs `
    src/app_skirmish_shell_render/modals.rs
```

Expected: no whitespace errors. Preserve unrelated working-tree changes.

**Step 2: Run focused tests serially**

After confirming no other session owns Cargo:

```powershell
cargo test -q --lib choose_map_ -- --nocapture
cargo test -q --lib semantic_draw_order_records_verified_right_panel_sequence -- --nocapture
cargo test -q --lib push_entry_top_clipped_native -- --nocapture
```

Expected: each command prints a literal `test result: ok` with zero failures.

**Step 3: Run final compile and debug build**

```powershell
cargo check -q
cargo build -q --bin vera20k
```

Expected: both commands exit `0`. Pre-existing warnings may remain; no new
errors are acceptable.

**Step 4: Hand off to the user**

Provide:

`target/debug/vera20k.exe`

Do not use Windows app control. Ask the user to open ordinary offline Skirmish
at logical 800x600 and verify:

- the top title/preview area has continuous retail metal chrome;
- button wells and the stacked rail panels are visible behind Use Map and
  Random Map;
- the bottom rail and Cancel footer are framed;
- `LWSCRNL` replaces the black lower-left gap;
- the orange battle artwork remains visible behind both lists;
- red selections, yellow text, scrollbar, preview, and buttons remain clear;
- Cancel returns to an unchanged normal setup rail.

If the manual capture still differs, stop and compare the exact missing
layer/rect before adding another visual patch.

**Step 5: Report evidence honestly**

Report the focused test result lines, `cargo check`/build status, and the
user's visual verdict. Call the result retail-convincing when supported; do not
claim native pixel parity. Record unchanged residuals:

- `0x6B` D9/`SDWRNTMP` transition animation;
- exact non-800 modal background pixels.

## Sources & References

- **Design doc:**
  `docs/plans/2026-07-27-customize-battle-sidebar-restoration-design.md`
- **Primary modal composition:**
  `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md`
- **Modal visual integration:**
  `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`
- **Modal geometry:**
  `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`
- **Common parent paint:**
  `docs/research/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`
- **Per-dialog optional chrome flags and frame schedule:**
  `docs/research/skirmish-ui/SHELL_DIALOG_LIFETIME_TRANSITION_EVIDENCE_GATES_GHIDRA_REPORT.md`
- **Right-panel layout and clipping model:**
  `docs/research/skirmish-ui/SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`
- **Dialog background pointer table:**
  `docs/research/FUN_0060CF00_DIALOG_BACKGROUND_POINTER_TABLE_GHIDRA_REPORT.md`
- **Frame-10 steady gate:**
  `docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_SDBTNANM_FRAME10_STATE_FLAG_GHIDRA_REPORT.md`
- **Retail SHP dimensions:**
  `docs/research/RIGHT_PANEL_SHP_HEADER_DIMENSIONS_GHIDRA_REPORT.md`
- **gamemd.exe anchors:** modal wrapper `0x005E68A0`; common shell setup
  `0x00622820`; dialog background binding `0x0060CF00`; modal asset load
  `0x0072D120`; parent paint `0x00621E90`; right-panel draw `0x0072E450`;
  background overlay `0x0072E730`.
- **Related Rust:**
  `src/app_skirmish_shell_render.rs`,
  `src/app_skirmish_shell_render/chrome.rs`,
  `src/app_skirmish_shell_render/draw_order.rs`,
  `src/app_skirmish_shell_render/modals.rs`,
  `src/render/skirmish_shell_chrome.rs`,
  `src/ui/skirmish_shell/layout.rs`,
  `src/ui/shell/geom.rs`.
- **INI keys:** none.
