# Customize Battle UI Faithful Restoration Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Restore the ordinary offline-skirmish Customize Battle screen at 800x600 so the orange retail battle artwork and metallic shell remain visible behind the chooser controls without changing selection, preview, scrolling, or button behavior.

**Architecture:** Keep the fix inside the existing app-layer modal sprite builder. The atlas remains the asset authority, `ChooseMapModalState` remains the interaction authority, and the outer skirmish renderer remains responsible for preview and text overlays. Select one of two presentation-only compositions at paint time: retail backing when the exact-800 asset is available, or the current opaque primitive fallback when it is not.

**Design Doc:** `docs/plans/2026-07-27-customize-battle-ui-restoration-design.md`

---

## Grounding Summary

- Standard offline YR reaches Choose Map as modal dialog `0x6B`; the parent setup dialog `0x102` is hidden while it owns the shell. The current Rust modal-only early return already preserves that lifecycle. Source: `SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md`.
- Dialog `0x6B` binds `MnScrnLCustomizeBattle.shp/.PAL`; the SHP load is active at exact screen width 800. The verified asset has one frame, so frame 0 is the complete variant set for this role. Source: `SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md`; `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`.
- Current Rust already loads that SHP with its modal-specific palette and emits it from `push_choose_map_modal_instances`. Source: `src/render/skirmish_shell_chrome.rs:348-363`, `src/app_skirmish_shell_render/modals.rs:108-140`.
- Current Rust then emits a closer full-screen `SHELL_MODAL_BG_RGB` rectangle, covering the retail asset. It also fills each listbox with a closer solid rectangle. Source: `src/app_skirmish_shell_render/modals.rs:51-67`, `118-171`.
- The supplied retail reference visibly retains the orange artwork beneath the unselected list rows, while the current runtime capture is flat green. Source: user captures `codex-clipboard-2459831f-9f46-46cf-9803-8d87395a0355.png` and `codex-clipboard-5d321692-77b3-4f22-b2f4-db4c779ca623.png`.
- Connected Ghidra research describes native listbox/scrollbar painting as backing-surface composition plus owner-draw overlays, not as one proven flat unselected-background RGB. Omitting the Rust-only list fill in retail-art mode is therefore consistent with the visible reference; it is not a claim that native uses alpha transparency. Source: `SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md` §§3.1, 5, 9.
- `choose_map_modal_semantic_draw_order` already models the Customize Battle background and primitive backdrop as mutually exclusive choices. The production sprite builder currently violates that model. Source: `src/app_skirmish_shell_render/draw_order.rs:120-132`.
- The saved-seed browser reuses `push_choose_map_listbox_instances`; it must continue requesting an opaque interior because its composition is outside this approved fix. Source: `src/app_skirmish_shell_render/modals.rs:463-505`.
- No INI keys drive this composition. Asset names, the exact-width gate, dialog rectangles, and control state already come from the atlas, verified resource layout, and modal state.
- No new live Ghidra check is required: the consequential activation, asset/frame, lifecycle, and owner-draw composition boundaries are covered by verified reports and the user's contradictory runtime/reference captures.
- Still unverified: exact final native listbox/scrollbar pixels after DirectDraw conversion and the native presentation policy above width 800. These remain visual exactification residuals, not blockers for the approved 800x600 restoration.

## Key Technical Decisions

- Resolve retail-versus-fallback once from `choose_map_background_entry(atlas, layout)` and use the same result for both the full-screen base layer and listbox-interior policy. — **Confidence: high**
  - **Source:** verified exact-800 asset binding in `SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md`; current optional-atlas pattern in `push_validation_modal_instances`.
- Represent the shared listbox behavior with a private `ListboxInteriorPaint` enum rather than a Boolean. `PreserveBacking` means “do not emit a Rust solid fill”; `OpaqueFallback` keeps the current readable fallback. — **Confidence: high**
  - **Source:** backing-surface composition in `SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md`; shared helper callsites in `modals.rs`.
- Do not change layout, text, preview, input, modal state, button frames, sounds, asset loading, or random-map behavior. — **Confidence: high**
  - **Source:** current Rust paths are already wired and outside the contradicted draw layers.
- Keep non-800 screens on the current fallback and do not stretch the 632x568 asset. — **Confidence: high for evidence honesty; medium for visual fidelity above 800**
  - **Source:** exact-width branch at `0x0072D120`; higher-resolution runtime composition remains unverified.
- Verify retail-convincing composition by an 800x600 production capture, but do not claim native pixel parity. — **Confidence: high**
  - **Source:** project truth bar in `AGENTS.md`.

## Open Questions

### Resolved During Planning

- Is the orange artwork absent because the asset is missing? No. The atlas loads and emits `MnScrnLCustomizeBattle` before closer opaque rectangles cover it.
- Should the listboxes retain their current solid green interior over the restored art? No for the exact-800 retail-art branch; the reference and native backing-composition evidence show the dialog artwork through unselected rows.
- Should the saved-seed browser become backing-preserving too? No. It is outside the approved screen and keeps `OpaqueFallback` explicitly.
- Is a new palette or INI constant needed? No. The modal-specific PAL path already exists and no INI controls this composition.

### Deferred to Implementation

- Exact final native RGB for unselected listbox backing and scrollbar track remains capture-dependent. The implementation preserves the retail backing and existing scrollbar overlay without relabeling approximate colors as exact.
- The final production screenshot can reveal an unexpected atlas offset, color-conversion problem, or overlay-depth problem. If it does, stop and diagnose that observed layer rather than adding speculative offsets or colors.
- Native behavior above 800 pixels remains unverified. It does not block the approved 800x600 scenario and stays on the existing primitive fallback.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app_skirmish_shell_render/modals.rs` | Select the retail or fallback base layer and make shared listbox interior painting explicit. |
| Modify | `src/app_skirmish_shell_render.rs` | Strengthen the existing semantic draw-order regression for mutually exclusive base layers. |
| Create | `docs/plans/2026-07-27-customize-battle-ui-restoration-plan.md` | Record this implementation handoff. |

No file approaches the roughly 600-line split threshold because `modals.rs` is currently 538 lines and the planned policy/tests add fewer than 45 lines. The already-large parent render module receives only two assertions in its existing test.

## Interface Changes

No public API, persisted format, configuration schema, UI state, or simulation interface changes.

One private renderer-helper signature changes:

```rust
fn push_choose_map_listbox_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    list: RectPx,
    row_count: usize,
    top_index: usize,
    selected_index: Option<usize>,
    interior: ListboxInteriorPaint,
    depth: f32,
)
```

All three same-file callsites must be updated. The two Choose Map callsites use the policy derived from retail-background availability; the saved-seed callsite explicitly uses `ListboxInteriorPaint::OpaqueFallback`.

## Risk Areas

- Drawing the retail asset and fallback together would leave the current bug intact. Use an `if let ... else` so exactly one branch emits a base layer.
- Making every listbox backing-preserving would alter the saved-seed browser. Pass an explicit policy to every helper call.
- Removing selection, scrollbar, frame, preview, button, or text layers would make the screen incomplete. Only gate the initial unselected-interior solid fill.
- Changing depths while removing fills could bury the preview or text. Keep all existing depth values.
- Stretching or centering the SHP could invent non-native presentation. Keep the existing native-size origin placement.
- Active Cargo processes may belong to another session. Check before every Cargo run and wait rather than starting a competing build.

## Player-Experience Critical Items

Representative scenario: start an ordinary offline YR stock skirmish at the standard 800x600 shell size, open Customize Battle, inspect and scroll the mode/map lists, choose a map, and exercise Use Map, Random Map, and Cancel.

| Task # | Class | Item | Why it matters | Verification |
|--------|-------|------|----------------|--------------|
| 1-2 | MILESTONE-BLOCKING | Retail backdrop and primitive backdrop are mutually exclusive | The closer primitive layer currently erases the defining orange artwork | Semantic draw-order test plus 800x600 production capture |
| 1-2 | MILESTONE-BLOCKING | Unselected chooser rows preserve the retail backing | Opaque green list interiors would still hide most of the artwork | Policy unit tests plus reference comparison |
| 2 | MILESTONE-BLOCKING | Red selected rows, 19-pixel cadence, text, frame, and overflow scrollbar remain above the backing | These are continuously visible and interactive | Existing chooser tests plus runtime selection/scroll check |
| 2 | MILESTONE-BLOCKING | Preview and owner-draw buttons retain their current draw/input paths | Use Map, Random Map, and Cancel close the ordinary selection loop | Runtime button and preview check; no changes to app/state paths |
| 2 | COMPOUNDING | Parent setup remains suppressed | Leaking the parent through backing-preserving lists would mix two dialogs | Existing modal early-return test and runtime capture |
| 2 | COMPOUNDING | Saved-seed browser retains an opaque list interior | Shared-helper drift would damage an adjacent modal | Explicit `OpaqueFallback` callsite and focused policy test |
| 3 | EXACTIFICATION-RESIDUAL | Non-800 screens retain primitive fallback | Native higher-resolution policy is unverified; effect is presentation-only | Confirm no stretch code and record residual |
| 3 | EXACTIFICATION-RESIDUAL | Exact final list/scrollbar pixels remain unclaimed | Native DirectDraw conversion and backing composition lack executable pixel comparison | Report retail-convincing visual result without parity certification |

---

## Tasks

### Task 1: Define and lock the base-layer/listbox policy

**Why:** Establish the private renderer contract and regression expectations before changing production composition.

**Files:**

- Modify: `src/app_skirmish_shell_render/modals.rs:45-51, after line 538`
- Modify: `src/app_skirmish_shell_render.rs:1492-1511`

**Pattern:** Follow the existing private paint-policy helpers in the skirmish render modules: derive a small `Copy` value from render availability, keep it out of durable UI state, and test it without constructing GPU resources.

**Step 1: Add the explicit listbox interior policy before `push_choose_map_listbox_instances`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListboxInteriorPaint {
    PreserveBacking,
    OpaqueFallback,
}

impl ListboxInteriorPaint {
    const fn paints_solid_fill(self) -> bool {
        matches!(self, Self::OpaqueFallback)
    }
}

const fn choose_map_listbox_interior(
    retail_background_available: bool,
) -> ListboxInteriorPaint {
    if retail_background_available {
        ListboxInteriorPaint::PreserveBacking
    } else {
        ListboxInteriorPaint::OpaqueFallback
    }
}
```

Add this comment immediately above the enum:

```rust
/// Native owner-draw listboxes preserve a composed backing surface. On the
/// exact-800 chooser that backing is the Customize Battle artwork; the solid
/// interior exists only to keep the asset-missing fallback readable.
```

**Step 2: Add pure policy tests at the end of `modals.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_map_retail_art_preserves_listbox_backing() {
        let interior = choose_map_listbox_interior(true);

        assert_eq!(interior, ListboxInteriorPaint::PreserveBacking);
        assert!(!interior.paints_solid_fill());
    }

    #[test]
    fn choose_map_missing_art_uses_opaque_listbox_fallback() {
        let interior = choose_map_listbox_interior(false);

        assert_eq!(interior, ListboxInteriorPaint::OpaqueFallback);
        assert!(interior.paints_solid_fill());
    }
}
```

**Step 3: Strengthen the existing semantic draw-order regression**

In `choose_map_modal_semantic_draw_order_replaces_parent_shell`, add the retail assertion after `order[0]` is checked:

```rust
assert!(!order.contains(&SkirmishShellDrawRole::ChooseMapModalBackdrop));
```

After the fallback's first role is checked, add:

```rust
assert!(!fallback.contains(
    &SkirmishShellDrawRole::ChooseMapBackgroundCustomizeBattle800
));
```

**Step 4: Verify**

Before Cargo, run:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

If Cargo/Rustc is active, wait for its owner to finish. Then run serially:

```powershell
cargo test -q choose_map_retail_art_preserves_listbox_backing -- --nocapture
cargo test -q choose_map_missing_art_uses_opaque_listbox_fallback -- --nocapture
cargo test -q choose_map_modal_semantic_draw_order_replaces_parent_shell -- --nocapture
```

Expected for each command: exit code 0 and a literal `test result: ok.` line.

### Task 2: Make retail and fallback composition mutually exclusive

**Why:** Remove the two Rust-only opaque layers that conceal the retail art while preserving a readable missing-asset path and every existing overlay.

**Files:**

- Modify: `src/app_skirmish_shell_render/modals.rs:51-106, 118-171, 498-505`

**Pattern:** Follow `push_validation_modal_instances`: emit primitive backdrop/chrome only when the native asset is absent. Keep the existing atlas lookup and depths.

**Step 1: Extend the shared listbox helper with the explicit policy**

Replace its signature with:

```rust
pub(super) fn push_choose_map_listbox_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    list: RectPx,
    row_count: usize,
    top_index: usize,
    selected_index: Option<usize>,
    interior: ListboxInteriorPaint,
    depth: f32,
) {
```

Replace the unconditional initial `push_solid_rect` with:

```rust
    let content = choose_map_listbox_content_rect(row_count, list);
    if interior.paints_solid_fill() {
        push_solid_rect(
            out,
            atlas,
            list,
            SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE,
            depth,
        );
    }
```

Leave the selected-row fill, scrollbar, owner-draw frame, and all their depth offsets unchanged.

**Step 2: Replace the chooser's overlapping base layers with one branch**

Replace the current background block plus unconditional dialog fill/outline with:

```rust
    let background = choose_map_background_entry(atlas, layout);
    let listbox_interior = choose_map_listbox_interior(background.is_some());
    if let Some(background) = background {
        push_entry_native(
            out,
            background,
            layout.screen.x,
            layout.screen.y,
            SHELL_PARENT_BACKGROUND_DEPTH,
        );
    } else {
        push_solid_rect(
            out,
            atlas,
            layout.dialog,
            SHELL_MODAL_BG_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00008,
        );
        push_rect_outline(
            out,
            atlas,
            layout.dialog,
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            SHELL_DROPDOWN_DEPTH - 0.00009,
        );
    }
```

Do not change `choose_map_background_entry`: its exact-width gate is part of the verified contract.

**Step 3: Pass the derived policy to both chooser listboxes**

The two calls become:

```rust
    push_choose_map_listbox_instances(
        out,
        atlas,
        layout.mode_list,
        mode_row_count,
        modal.mode_top_index,
        selected_mode_index,
        listbox_interior,
        SHELL_DROPDOWN_DEPTH - 0.00010,
    );
    push_choose_map_listbox_instances(
        out,
        atlas,
        layout.map_list,
        modal.filtered_record_indices.len(),
        modal.map_top_index,
        modal.highlighted_filtered_index,
        listbox_interior,
        SHELL_DROPDOWN_DEPTH - 0.00010,
    );
```

Keep all button and preview-frame code below them unchanged.

**Step 4: Preserve the saved-seed browser explicitly**

Update its call to include:

```rust
        ListboxInteriorPaint::OpaqueFallback,
```

between `browser.selected` and the existing depth argument. Do not change the saved-seed backdrop, outline, edit plate, or buttons.

**Step 5: Inspect the production diff**

Run:

```powershell
git diff -- src/app_skirmish_shell_render/modals.rs src/app_skirmish_shell_render.rs
```

The diff must show:

- one optional retail/fallback base-layer branch;
- conditional listbox interior fill only;
- unchanged selection/scrollbar/frame/button/preview depths;
- explicit opaque saved-seed policy;
- tests only outside the bounded production edits.

**Step 6: Verify focused behavior**

After confirming no other Cargo owner is active, run:

```powershell
cargo test -q choose_map_ -- --nocapture
```

Expected: exit code 0 and a literal `test result: ok.` line. Read the reported test counts rather than inferring success from command completion.

### Task 3: Format, regress, and perform the 800x600 production acceptance

**Why:** Confirm that the small composition fix survives the surrounding chooser suite and produces the supplied retail visual without architecture drift.

**Files:**

- Verify: `src/app_skirmish_shell_render/modals.rs`
- Verify: `src/app_skirmish_shell_render.rs`
- Verify visually: the production `vera20k` binary at 800x600

**Pattern:** Use focused tests first, one final package check, then production-path visual validation. Do not add a second renderer or a hand-authored golden.

**Step 1: Format only edited Rust files and inspect for churn**

```powershell
rustfmt --edition 2024 src/app_skirmish_shell_render/modals.rs src/app_skirmish_shell_render.rs
git diff --check
git diff --stat -- src/app_skirmish_shell_render/modals.rs src/app_skirmish_shell_render.rs
git diff -- src/app_skirmish_shell_render/modals.rs src/app_skirmish_shell_render.rs
```

Expected: no whitespace errors and no unrelated formatting churn.

**Step 2: Run the focused UI/render regressions serially**

Check for active Cargo/Rustc first, then run:

```powershell
cargo test -q choose_map_ -- --nocapture
cargo test -q --lib skirmish_shell -- --nocapture
```

Expected: both commands exit 0 with literal `test result: ok.` lines. Record each reported passed/failed/ignored count.

**Step 3: Run the final compile check**

```powershell
cargo check -q
```

Expected: exit code 0.

**Step 4: Launch the production shell at its fixed 800x600 size**

In one PowerShell session:

```powershell
$env:RA2_DEV_SKIRMISH_SHELL='1'
cargo run --bin vera20k
```

Open Customize Battle. Use a stock mode whose map list overflows so the scrollbar is present.

Verify all of these in the live production renderer:

- orange `MnScrnLCustomizeBattle` artwork and metallic frame are visible;
- no flat green rectangle covers the full screen;
- unselected rows preserve the artwork behind them;
- selected mode and map rows remain solid red;
- headings and row text remain readable and aligned;
- scrollbar arrows, track, and thumb remain visible;
- selected-map preview and numbered start markers remain visible;
- the parent setup controls do not leak through;
- Use Map commits and returns to setup with the selected map;
- Cancel returns without committing the temporary selection;
- Random Map opens the existing random-map setup path;
- owner-draw buttons still show pressed feedback and play the existing sound.

Capture an 800x600 screenshot and compare it by composition category with `<local>/AppData/Local/Temp/codex-clipboard-5d321692-77b3-4f22-b2f4-db4c779ca623.png`.

**Step 5: Stop on contradiction**

If the retail asset remains absent, appears at the wrong offset, has incorrect palette colors, or is still covered, do not tune coordinates or colors by eye. Reinspect the emitted sprite instances and atlas entry/palette path to identify the specific incorrect layer. If the live screen matches the composition categories, report it as retail-convincing restoration with non-800 and exact-pixel residuals still `UNCHECKED`.

## Sources & References

- **Design doc:** `docs/plans/2026-07-27-customize-battle-ui-restoration-design.md`
- **Primary Ghidra reports:**
  - `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_POST_IMPLEMENTATION_GAP_AUDIT_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- **gamemd.exe anchors retained in research, not Rust comments:** modal entry/lifetime `0x006ACEE0`, modal wrapper `0x005E68A0`, modal asset binding `0x0060CF00`, exact-800 asset loader `0x0072D120`, owner-draw listbox `0x00618D40`, PE dialog resource `0x6B`.
- **INI keys:** none.
- **Related code:**
  - `src/app_skirmish_shell_render/modals.rs`
  - `src/app_skirmish_shell_render/draw_order.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/render/skirmish_shell_chrome.rs`
  - `src/ui/skirmish_shell/layout.rs`
  - `src/ui/skirmish_shell/state/choose_map.rs`
  - `src/app.rs`
- **Runtime references:**
  - Current Rust: `<local>/AppData/Local/Temp/codex-clipboard-2459831f-9f46-46cf-9803-8d87395a0355.png`
  - Retail target: `<local>/AppData/Local/Temp/codex-clipboard-5d321692-77b3-4f22-b2f4-db4c779ca623.png`
- **Relevant prior commits:** `470fae54`, `6c0ea72e`, `946e7e1a`, `2e252350`, `97dcb32b`, `c06f5ba4`.
