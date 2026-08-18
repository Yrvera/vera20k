# Sidebar Dirty Rects Redraw Flicker Pixel Cadence - Ghidra Report

Status: COMPLETE
Date: 2026-05-27
Scope: ordinary in-game sidebar surface dirty flag writers, partial-copy timing, frame-to-frame preservation, and post-blit display signaling. This report does not re-investigate SHP selectors, minimap aperture constants, or gadget dimensions except where they feed dirty state.

## Working Notes Gate

- Target question: Which native sidebar paths set or consume `DAT_00B0B518`, `DAT_00B0B519`, `this+0x53A6..0x53A8`, `DAT_008809F4..A00`, and `DAT_00B07DC8..DD4`, and what does that prove about dirty-rect cadence and unchanged-pixel preservation?
- Non-goals: Do not re-investigate SHP selectors, minimap content geometry, gadget dimensions, palette conversion, radar movie filenames, or scroll-grid dimensions except as dirty-rect inputs.
- Evidence needed to mark COMPLETE: read-only Ghidra decompile for `SidebarClass::Draw @ 0x006A6C30`, `SidebarClass::BlitToScreen @ 0x006A70E0`, main post-blit consumer, and representative dirty writers; plus local read-only byte disassembly ranges confirming address-level stores/compares.
- Stop conditions: stop if Ghidra read-only access is unavailable, if proving every writer requires leaving the ordinary sidebar surface path, or if a function boundary is missing and cannot be inspected without mutation.

## Evidence Log

- Ghidra MCP read-only: `decompile_function 0x006A6C30`, `0x006A70E0`, `0x004F44F0`, `0x0063FB20`, `0x00653100`, `0x006A65F0`, `0x006A7A80`, `0x006A9540`, `0x006AB990`, `0x006D0E60`, `0x006A5ED0`, `0x006A60A0`, `0x006A7D20`, `0x006A76C0`.
- Ghidra MCP read-only: byte-pattern searches for globals `DAT_00B0B518`, `DAT_00B0B519`, `DAT_008809F4`, `DAT_00B07DC8`, and sidebar offsets `0x53A6..0x53A8`.
- Local read-only Capstone disassembly of retail `gamemd.exe` from `<ra2-install>/gamemd.exe` for exact instruction ranges. This was used because the Ghidra disassembly endpoint returned success metadata without instruction text.
- Sibling baseline consulted: `SIDEBAR_BLIT_TO_SCREEN_DIRTY_RECTS_GHIDRA_REPORT.md`, `SIDEBAR_DRAW_COMPOSITION_ORDER_AND_SURFACE_ORIGIN_GHIDRA_REPORT.md`, `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`.

## Verified Findings

### 1. `SidebarClass::Draw` snapshots the previous sidebar-surface dirty rectangle before drawing, then draws into the same retained sidebar surface.

Active in YR: Yes.

Evidence: `SidebarClass::Draw @ 0x006A6C30` decompile copies `DAT_00B07DC8/CC/D0/D4` into `DAT_008809F4/F8/FC/A00`, then sets `g_PrimarySurface = g_SidebarSurface`. Local disassembly confirms the start of the function: `0x006A6C34` reads `0x00B07DC8`, `0x006A6C3D` reads `0x00B07DD0`, and `0x006A6C49` writes `0x008809F4`. The function restores `g_PrimarySurface` only after `SidebarClass::BlitToScreen`.

Implication: `DAT_008809F4..A00` is the frame-start snapshot/working rect used to decide what changed during the sidebar draw. Unchanged pixels persist on `g_SidebarSurface`; native does not rebuild a fresh sidebar image every frame.

### 2. Full/sidebar-wide redraw is raised by explicit sidebar mutation and animation paths, not by every frame.

Active in YR: Yes.

Evidence: `SidebarClass::Draw @ 0x006A6C30` computes a full-redraw predicate from function argument, `this+0x53A6`, and `this+0x53A7`; only when active and not map editor does it redraw side chrome/gadgets and then write `DAT_00B0B518 = 1` at `0x006A6FB7`. Other representative writers are:

- `PowerClass::Draw @ 0x0063FB20`: if forced or `this+0x150C` power dirty and `DAT_00884B8D` is true, clears `this+0x150C` and sets `DAT_00B0B518 = 1`; local disassembly confirms store at `0x0063FB55`.
- `StripClass::Draw @ 0x006A9540`: returns when inactive and not forced; otherwise clears strip dirty `this+0x3C` and sets `DAT_00B0B518 = 1`; local disassembly confirms store at `0x006A957D`.
- `RadarClass::Draw @ 0x00653100`: radar event/dirty merge paths update `this+0x120C..0x1218` and set `DAT_00B0B518 = 1`; local disassembly confirms stores at `0x00653621` and `0x00653723`.
- `DrawCreditsSHPBackground @ 0x006D0E60`: draws the sidebar credit/background SHP into `g_SidebarSurface` then sets `DAT_00B0B518 = 1`; local disassembly confirms store at `0x006D0EAE`.

This set is representative, not an exhaustive whole-program writer map. It covers the ordinary in-game sidebar draw path and its active strip/power/radar subpaths.

### 3. `this+0x53A6` and `this+0x53A7` are higher-level invalidation bytes with different repaint effects.

Active in YR: Yes.

Evidence: `SidebarClass::Draw @ 0x006A6C30` includes both bytes in the full-redraw predicate at `0x006A6CB1..0x006A6CBB`; it specifically gates side-piece chrome repaint on `this+0x53A7` before drawing `SIDE1/SIDE2/SIDE3/ADDON`. It clears both bytes after `BlitToScreen`; local disassembly confirms `0x006A70B7` clears `this+0x53A6` and `0x006A70BD` clears `this+0x53A7`.

Representative active writers:

- `SidebarClass::AddCameo @ 0x006A65F0`: switching active strip writes `this+0x53A7 = 1`; later writes `this+0x53A6 = 1` and may set `DAT_00B0B518` when the updated tab is active. Local disassembly confirms `0x006A65CF` and `0x006A65DC`.
- `FUN_006A6820` and `SidebarClass::SwitchTab @ 0x006A76C0`: active tab changes hide/show cameo slots and set `this+0x53A7 = 1`; local disassembly confirms `0x006A69F0` and `0x006A76D9`.
- `FUN_006A5ED0`, `FUN_006A60A0`, `SelectionClass::RemoveFromSelection @ 0x006A5F80`, `FUN_006A7D20`, and `SidebarClass::Action @ 0x006A7A80`: build-limit, production, selection, recalculation, and gadget-state changes set `this+0x53A6 = 1` and usually mark the active strip dirty.

Implication: native distinguishes "strip/control data changed" from "active tab/strip identity changed enough to repaint side chrome." Rust should not collapse both into unconditional full sidebar repaint if flicker/dirty cadence parity matters.

### 4. `BlitToScreen` preserves unchanged pixels by skipping copies or copying only dirty rectangles, then clears the folded dirty byte.

Active in YR: Yes.

Evidence: `SidebarClass::BlitToScreen @ 0x006A70E0` has early-outs on inactive sidebar, game inactive, and display gate disabled; all clear `DAT_00B0B518` without setting `DAT_00B0B519`. When `DAT_00B0B518 == 0`, function argument is zero, and `DAT_008809F4..A00` matches `DAT_00B07DC8..DD4`, it either returns with no copy or, conditionally on `this+0x53A8`, copies only the top strip. Local disassembly confirms the fast-path compare and clear sequence at `0x006A7143..0x006A7166`, `this+0x53A8` check at `0x006A71AB`, and final `DAT_00B0B518 = 0` at `0x006A748A`.

When changes are present, `BlitToScreen` marks `g_SidebarSurface` through the display chain, then can copy the top strip, lower body from y `g_SidebarWidth`, current surface dirty rect, or `DAT_008809F4..A00` partial rect. Local disassembly confirms `DAT_00B0B518 = 1` at `0x006A723A`, `this+0x53A8` checks/clears at `0x006A72A8` and `0x006A7315`, partial-rect source push at `0x006A7465`, `DAT_00B0B519 = 1` at `0x006A7481`, and folded dirty clear at `0x006A748A`.

Implication: unchanged sidebar-surface pixels are retained frame to frame. Flicker prevention depends on not repainting or copying unrelated regions unless the native dirty inputs require it.

### 5. `DAT_00B0B519` is the post-blit display-stage handoff, consumed once in the main render frame.

Active in YR: Yes.

Evidence: `SidebarClass::BlitToScreen @ 0x006A70E0` sets `DAT_00B0B519 = 1` only after a copy path reaches `0x006A7481`; early-outs do not set it. `RenderFrame_main @ 0x004F44F0` checks `DAT_00B0B519` and `g_IsMapEditor == 0`, calls display-chain vtable `+0x40(g_SidebarSurface, 1)`, then clears `DAT_00B0B519`; local disassembly confirms read at `0x004F451B` and clear at `0x004F4540`.

Implication: a sidebar copy has a second-stage display notification in the same render pipeline. A Rust implementation that only rebuilds sprite batches each frame has no native equivalent for "copied sidebar surface, notify display once."

### 6. `this+0x53A8` only has proven clear/check behavior in the scoped sidebar path.

Active in YR: Conditional, writer not proven in this scoped pass.

Evidence: `SidebarClass::Constructor @ 0x006A4EC0` clears `this+0x53A8`; local disassembly confirms `0x006A4ECD: mov byte ptr [edi + 0x53A8], bl`. `BlitToScreen` checks it at `0x006A71AB` and `0x006A72A8`, and clears it after top-strip copies at `0x006A722F` and `0x006A7315`. A local read-only `.text` scan for `0x53A8` references in the sidebar address band `0x006A0000..0x006ABFFF` found those constructor/Blit references but no `mov [sidebar+0x53A8], 1`.

Implication: the top-strip-only branch exists and must be modeled if its writer is later found, but this pass does not prove a standard in-game writer that arms it.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Native sidebar drawing retains `g_SidebarSurface` between frames; no-dirty frames can skip both repaint and screen copy. | Add a retained sidebar-surface/dirty model, or explicitly prove full per-frame sprite redraw is pixel-identical under all flicker/overlap states before claiming parity. | `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`, future sidebar surface cache | Run two frames with no sidebar mutation after a full draw; the second frame must not redraw side chrome, StripClass, PowerClass, or notify the display-stage sidebar copy. | `test_sidebar_no_dirty_frame_preserves_surface_without_repaint` | HIGH for flicker and transition overlap parity. |
| `this+0x53A6` and `this+0x53A7` are distinct invalidation bytes: `0x53A7` drives side-piece chrome repaint; `0x53A6` can force strip/control redraw without that identity change. | Split Rust invalidation into active-strip/content dirty vs active-tab/side-piece dirty instead of one "sidebar dirty" or unconditional frame rebuild. | sidebar state/model, `src/sidebar/mod.rs`, `src/app_sidebar_build.rs` | Change a build-limit/production state and verify only the active strip/control region is invalidated; switch tab and verify side-piece/chrome repaint path is triggered. | `test_sidebar_53a6_content_dirty_differs_from_53a7_tab_repaint_dirty` | HIGH for dirty cadence and medium for steady screenshots. |
| `DAT_00B0B519` is set only after a screen-copy path and consumed once by `RenderFrame_main`; early-outs clear `DAT_00B0B518` without setting it. | Model post-blit display notification separately from "sidebar data changed"; skipped copies must not force present-stage sidebar notification. | render scheduling around `src/app_render/mod.rs`, `src/app_render/draw_passes.rs` | Disable sidebar/display gate for a dirty draw and assert folded dirty clears without a sidebar present notification; then enable and assert one notification on the first copied frame only. | `test_sidebar_post_blit_display_flag_sets_only_after_copy` | MEDIUM-HIGH for flicker/present timing parity. |

## Negative Facts / Do Not Do

- Do not redraw/reupload every sidebar layer every frame and call it native dirty parity. Evidence: `StripClass::Draw @ 0x006A9540` returns immediately when `this+0x3C == 0` and force is zero; `BlitToScreen @ 0x006A7143..0x006A71AB` can perform no copy at all.
- Do not collapse `this+0x53A6` and `this+0x53A7` into one generic dirty bit. Evidence: `SidebarClass::Draw @ 0x006A6CEE` gates side-piece repaint specifically on `this+0x53A7`, while many production/selection paths set only `0x53A6`.
- Do not assume `DAT_00B0B518` persists to a later frame after skipped copy. Evidence: `BlitToScreen` clears it at `0x006A748A` on both copied and early-out paths.
- Do not treat `DAT_00B0B519` as another request to redraw sidebar content. Evidence: `RenderFrame_main @ 0x004F451B..0x004F4540` consumes it by calling display-chain `+0x40(g_SidebarSurface, 1)` and clearing the byte.
- Do not mark the top-strip-only branch as standard active gameplay until a writer for `this+0x53A8 = 1` is proven. Evidence: this scoped sidebar scan found constructor clear and Blit clears/checks, but no setter in `0x006A0000..0x006ABFFF`.

## Rust Reconnaissance

- `src/app_render/build_instances.rs` builds independent `sidebar`, `chrome`, `cameo`, `gclock`, `cameo_overlay`, `text`, `minimap`, `viewport_rect`, and `radar_anim` batches per frame; no retained sidebar surface or native dirty-copy state is visible.
- `src/app_render/draw_passes.rs` draws those batches as independent UI passes. That is structurally different from native draw-into-surface then dirty-rect blit.
- `src/sidebar/mod.rs` exposes one `SIDEBAR_WIDTH = 168.0`, while dirty copy cadence still depends on the previously proven `g_SidebarWidth = 158` vs `g_SidebarTopClip = 168` split.
- `src/app_sidebar_build.rs` and `src/app_sidebar_text.rs` build/render sidebar visual layers independently, so they currently lack a native equivalent of `DAT_00B0B519` post-copy display signaling.

## Remaining Uncertainty

- Exact implementations of the display-chain vtable calls used by `BlitToScreen` and `RenderFrame_main` were not decompiled in this slot; their roles are inferred from call placement and sibling display-surface reports.
- No standard in-game writer for `this+0x53A8 = 1` was found in the scoped sidebar code band, but a whole-program dataflow pass would be needed to prove the top-strip-only branch unreachable.
- This report covers ordinary in-game sidebar cadence. Shell/dialog owner-draw paths and observer-specific sidebar paths were not traced here.

## Stale Doc Replacement Wording

- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`: replace `Blit sidebar surface to screen` with `SidebarClass::Draw snapshots the previous sidebar-surface dirty rect, draws only when force/0x53A6/0x53A7 or child dirty flags require it, and BlitToScreen can no-op, copy top strip, copy lower body, copy current surface dirty rect, or copy a cached partial rect. DAT_00B0B518 is cleared at BlitToScreen exit; DAT_00B0B519 is set only after copied paths.`
- `docs/research/RADAR_CHROME_COMPOSITING.md`: replace any wording implying the sidebar is fully repainted every frame with `Ordinary sidebar/radar drawing is retained-surface based: radar/strip/power writers mark DAT_00B0B518 and update dirty rectangles, then SidebarClass::BlitToScreen copies only the selected sidebar-surface-local rects and signals DAT_00B0B519 for the display stage.`
- `docs/research/SIDEBAR_BLIT_TO_SCREEN_DIRTY_RECTS_GHIDRA_REPORT.md`: append `Follow-up cadence check: this+0x53A8 top-strip copy is proven as a BlitToScreen branch, but this scoped sidebar scan did not find a standard in-game setter; treat active liveness as unresolved unless a writer is later proven.`
