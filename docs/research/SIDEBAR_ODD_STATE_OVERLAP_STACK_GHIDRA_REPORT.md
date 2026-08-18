# Sidebar Odd-State Overlap Stack - Ghidra Research Report

**Address(es):** `0x006A6C30`, `0x006A9540`, `0x0069DEB0`, `0x0063FB20`, `0x00653100`, `0x00656EC0`, `0x006579E0`, `0x004F4580`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** bounded sidebar-adjacent overlap facts for tooltip over sidebar, disabled gadget frame ordering, radar transition/minimap/chrome ordering, smooth-scroll extra draw row vs static hit zones, and observer strip branch only where it shares the normal sidebar stack.  
**Non-Scope:** full shell/dialog tooltip system, every observer UI panel, retail pixel screenshots, all palette conversion paths, and full dirty-rect flicker cadence.  
**Confidence:** High for listed order/overlap facts, Medium for tooltip internals beyond z-order.  
**Active in YR:** Yes for ordinary in-game sidebar paths; Conditional for observer branch and radar movie/transition modes.

## Target Question

When odd sidebar states occur, which layer wins the final pixels: tooltip, disabled gadgets, radar transition/minimap/chrome, smooth-scroll extra rows, and observer strip variants?

## Non-Goals

- Do not broaden into all Win32 shell tooltip/status controls.
- Do not re-audit `SSCRA*` owner-draw static geometry.
- Do not re-audit palette/ConvertClass routing beyond overlap implications.
- Do not implement Rust changes.

## Evidence Needed To Mark Complete For This Slice

- Top-level in-game frame composition proves tooltip z/order relative to sidebar.
- `SidebarClass::Draw` proves native sidebar-local ordering.
- `SBGadgetClass::Draw` proves disabled frame source and where gadget frames are painted in the sidebar stack.
- `RadarClass::Draw` / `RadarClass::PerFrameMovieUpdate` prove transition/minimap/chrome order.
- `StripClass::Draw` proves smooth-scroll extra row is visual-only relative to static select zones.
- Observer branch liveness is bounded to shared `StripClass::Draw` stack.

## Stop Conditions

- Stop at first verified owner outside ordinary/sidebar-adjacent overlap scope, such as full shell dialog tooltip rendering.
- Stop before palette/color exactness beyond layer-order consequence.
- Stop before observer-specific UI surfaces that do not reuse `SidebarClass::Draw` -> `StripClass::Draw`.

## 1. Overview

The sidebar is not painted as independent screen widgets in gamemd. `SidebarClass::Draw @ 0x006A6C30` redirects `g_PrimarySurface` to `g_SidebarSurface`, paints sidebar-local layers, calls `SidebarClass::BlitToScreen`, then restores the primary surface. The main frame compositor `RenderFrame_main @ 0x004F4580` later copies dirty sidebar/display regions and then invokes the tooltip manager, so active tooltips are above the sidebar copy, not inside the sidebar surface.

## 2. Core Ordered Composition

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Active for target? | Role |
|---|---|---|---|---|---|---|
| 1 | `SidebarClass::Draw @ 0x006A6C30` | `this+0x53A5`, redraw/full flags, not map editor | `SIDE1`, repeated `SIDE2`, `SIDE3`, `ADDON` | sidebar-local surface | Yes | background chrome |
| 2 | `SBGadgetClass::Draw @ 0x0069DEB0` from `SidebarClass::Draw` | after background; before strip | sell, repair, tab, scroll SHPs; frame from state | gadget rect | Yes | controls |
| 3 | `StripClass::Draw @ 0x006A9540` | active strip, `this+0x1C`, dirty/full force | cameo art, overlays, text | strip/cameo grid | Yes; observer conditional branch when `g_PlayerPtr == DAT_00AC1198` | strip content |
| 4 | `PowerClass::Draw @ 0x0063FB20` | called after strip | `POWERP.SHP` frames `0,4,1,2,3` | sidebar-local x `0/5`, y `g_SidebarWidth+0x45` | Yes when power display active | power meter |
| 5 | `RadarClass::Draw @ 0x00653100` | called by `PowerClass::Draw` | `DAT_00B04A38` frames, minimap surface, movie events | radar aperture/dirty rect | Yes; state dependent | radar/chrome/content |
| 6 | `SidebarClass::BlitToScreen @ 0x006A70E0` | dirty flag/full flag | copied sidebar surface | screen transform | Yes | sidebar composite |
| 7 | `RenderFrame_main @ 0x004F4580` | if `DAT_00B0B519` then sidebar surface display-chain copy, then root gadget, `FUN_005D49A0`, tooltip manager | tooltip manager output | screen/display chain | Yes when tooltip singleton exists | over-sidebar overlay |

## 3. Load-Bearing Findings

### 3.1 Tooltip is a post-sidebar overlay, not part of the sidebar surface

Active in YR: Yes. `RenderFrame_main @ 0x004F4580` performs the sidebar display-chain copy when `DAT_00B0B519 != 0` and not briefing/map-editor, then calls root gadget hook, `FUN_005D49A0`, and finally `DAT_00887368->vtable+0x0C(0)` for the tooltip singleton. `SidebarClass::Draw @ 0x006A6C30` also calls the tooltip singleton with argument `1` before `BlitToScreen`, but the confirmed visible end-of-frame call is after the sidebar copy.

Implication: a visible tooltip can cover sidebar pixels after the sidebar dirty copy; it is not clipped to the sidebar surface unless the tooltip manager itself clips it. The full tooltip manager paint internals were not expanded here.

### 3.2 Disabled sidebar gadgets use frame 2 and are painted before the strip/power/radar stack

Active in YR: Yes. `SBGadgetClass::Draw @ 0x0069DEB0` selects `cVar4 = 2` when `gadget+0x1E != 0`. Otherwise it selects idle/active/pressed frames from `+0x2C`, `+0x2D`, `+0x34`, and `+0x40`. `SidebarClass::Draw @ 0x006A6C30` calls `SBGadgetClass::Draw` for sell, repair, four tabs, and scroll controls before `StripClass::Draw` and before `PowerClass::Draw`.

Implication: disabled gadget pixels are above background chrome but can be overpainted by later strip/power/radar draws if any asset/rect overlaps. Do not draw disabled gadget frames as the last sidebar UI layer.

### 3.3 Radar transition/chrome is later than strip and power, with radar-local redraws that can repaint gadgets

Active in YR: Yes/Conditional. `PowerClass::Draw @ 0x0063FB20` always calls `RadarClass::Draw`. Inside `RadarClass::Draw @ 0x00653100`, when radar redraw flags are active it draws `DAT_00B04A38` frame `0`, transition/current frame, or frame `0x20` depending on mode, then calls `RadarClass::Update @ 0x00656EC0`. `RadarClass::PerFrameMovieUpdate @ 0x006579E0` redraws frame `0x20` before movie-surface work when `+0x14DA` is set. `RadarClass::Draw` also contains `SBGadgetClass::Draw(1)` calls after some radar redraw phases, so radar-area gadgets can be repainted after radar background/content under those conditions.

Implication: Rust's separate `radar_anim`, `minimap`, `sidebar_chrome`, and gadget batches need a single native-equivalent order for transition states. A generic always-after/always-before radar animation layer is insufficient.

### 3.4 Smooth-scroll extra row is visual-only; hit zones remain the static visible grid

Active in YR: Yes. `StripClass::Draw @ 0x006A9540` computes draw rows as the native visible row count plus `(this+0x3F != 0)` and offsets draw y by `(this+0x4C - DAT_00B0B500)` during smooth scroll. Prior slot `SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT_GHIDRA_REPORT.md` verifies `InitSelectZones @ 0x006A8220` builds static 60x48 hit zones and `SelectClass::Action @ 0x006AAD00` adds `ScrollPosition * 2` to the visible index.

Implication: the extra smooth-scroll row can be visible outside the normal static interaction grid, but it must not become clickable and must not move the hit zones.

### 3.5 Observer strip branch shares the same sidebar stack slot, not a separate top-level z-layer

Active in YR: Conditional on `g_PlayerPtr == DAT_00AC1198`. In `StripClass::Draw @ 0x006A9540`, the ordinary build-cameo branch is skipped/limited when the player pointer equals the observer house pointer; the later observer branch draws observer rows/text/icons within the same `StripClass::Draw` call. `SidebarClass::Draw` still orders that call after gadgets and before power/radar.

Implication: observer strip pixels replace/occupy the strip layer position in the normal sidebar stack. They are not a post-radar overlay and should not be modeled as a separate global HUD layer unless a separate observer UI path proves that for another surface.

## 4. Current Rust Implementation Status

Rust currently builds independent sidebar instance buffers and dispatches them in a fixed UI order:

- `src/app_render/draw_passes.rs`: `minimap` -> `viewport_rect` -> `sidebar` -> `sidebar_chrome` -> `radar_anim` -> `sidebar_cameo` -> `sidebar_gclock` -> `sidebar_cameo_overlay` -> `sidebar_text` -> cursor.
- `src/app_sidebar_build.rs`: power bar is built inside `sidebar_chrome` before cameo/gclock/text batches.
- `src/render/radar_anim.rs`: generic `radar.shp` opening/closing state separate from native `RadarClass::Draw` ordering.
- `src/sidebar/gadget_flash.rs`: frame 2 disabled selection matches the `SBGadgetClass::Draw` frame table, but disabled bits are currently v1/default false.
- `src/sidebar/sidebar_view.rs` / `src/sidebar/mod.rs`: static view/hit test exists; smooth-scroll visual offset/native extra row model is not native.
- No in-game tooltip overlay path matching native post-sidebar manager was found in the scoped Rust scan.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Top-level tooltip z/order | verified | `RenderFrame_main @ 0x004F4580`; `GSCREEN_RTACTICAL_GHIDRA_REPORT.md` | Tooltip manager internal paint/clipping |
| Sidebar local draw order | verified | `SidebarClass::Draw @ 0x006A6C30` | Dirty timing/flicker in sibling slot |
| Disabled gadget frame source | verified | `SBGadgetClass::Draw @ 0x0069DEB0` | Final overlap precedence only if asset rects overlap |
| Radar transition vs minimap/chrome order | verified for scoped modes | `RadarClass::Draw @ 0x00653100`, `RadarClass::Update @ 0x00656EC0`, `RadarClass::PerFrameMovieUpdate @ 0x006579E0` | Internal movie object draw callback details |
| Smooth-scroll extra row vs hit zones | verified-by-current+prior | `StripClass::Draw @ 0x006A9540`; prior `InitSelectZones`/`SelectClass::Action` reports | Exact runtime pixel capture of in-between offsets |
| Observer branch stack position | verified conditional | `StripClass::Draw @ 0x006A9540` | Full observer UI surface outside strip |
| Palette/color exactness | deferred | sibling palette slot owns this | separate palette/ConvertClass report |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-001 - Is tooltip rendered above sidebar? -> Yes, the end-of-frame tooltip manager call happens after sidebar display-chain copy.` (evidence: `0x004F4580`)
- `[RESOLVED] OQ-002 - Does disabled gadget frame come from binary state or Rust comment only? -> Binary state; `+0x1E != 0` selects frame 2.` (evidence: `0x0069DEB0`)
- `[RESOLVED] OQ-003 - Are disabled gadgets last in the sidebar stack? -> No, normal gadget draws precede strip, power, and radar.` (evidence: `0x006A6C30`)
- `[RESOLVED] OQ-004 - Does radar transition draw before strip? -> No, radar is reached from `PowerClass::Draw`, after strip.` (evidence: `0x006A6C30`, `0x0063FB20`, `0x00653100`)
- `[RESOLVED] OQ-005 - Can radar redraw repaint gadgets? -> Yes, `RadarClass::Draw` contains `SBGadgetClass::Draw(1)` calls around radar redraw states.` (evidence: `0x00653100`; xrefs from `0x006531CB`, `0x006531D6`, `0x00653501`, `0x0065350C`)
- `[RESOLVED] OQ-006 - Does smooth-scroll extra row affect hit zones? -> No evidence of moved hit zones; draw adds one row, prior hit-zone report keeps static zones.` (evidence: `0x006A9540`; `SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-007 - Is observer strip a post-radar layer? -> No, scoped observer branch is inside `StripClass::Draw`, before `PowerClass::Draw`/radar.` (evidence: `0x006A9540`, `0x006A6C30`)
- `[DEFERRED] OQ-008 - What exact tooltip pixels/clipping are used?` (category: `requires-different-system-context`; reason: this slot only needed z/order, not full ToolTipManager visual audit; next-step-if-pursued: investigate `ToolTipManager` vtable+0x0C target and `CCToolTip` draw callbacks)
- `[DEFERRED] OQ-009 - What are exact observer-only sidebar palette pixels?` (category: `out-of-scope`; reason: sibling palette slot owns ConvertClass/pixel colors; next-step-if-pursued: verify observer branch palette and asset routing)
- `[DEFERRED] OQ-010 - Does a runtime screenshot show flicker between partial dirty copies?` (category: `needs-runtime-debugger`; reason: binary order is proven but flicker visibility needs frame capture; next-step-if-pursued: record native frames during radar transition + scroll)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Tooltip paints after sidebar display-chain copy | `0x004F4580`; `PowerClass::RegisterTooltip @ 0x006403A0`; `ToolTipManager::Register_Tip @ 0x00724580` | missing in-game tooltip overlay in scoped scan | future tooltip renderer; `src/app_render/draw_passes.rs` | Tooltip/status overlay must draw after all sidebar layers and above sidebar pixels | Hover power bar/build cameo; tooltip covers sidebar art and cursor order matches native | Do not render tooltip inside `sidebar_chrome`/`sidebar_text` batch |
| Disabled gadget frame 2 is drawn before strip/power/radar | `0x0069DEB0`, `0x006A6C30` | frame table mostly matches; disabled bits mostly unset and order differs because power is in chrome batch | `src/sidebar/gadget_flash.rs`, `src/app_sidebar_build.rs`, `src/app_render/draw_passes.rs` | Keep frame 2 selection but place gadget batch in native order relative to strip/power/radar | Disable a tab/repair/sell control; disabled art is visible above chrome but not as final global overlay | Do not draw disabled gadgets last |
| Radar transition/minimap/chrome redraw after strip/power with radar-local gadget repaint cases | `0x00653100`, `0x00656EC0`, `0x006579E0` | Rust generic `radar_anim` currently before cameo/gclock/text and uses generic `radar.shp` model | `src/render/radar_anim.rs`, `src/render/sidebar_chrome.rs`, `src/app_render/draw_passes.rs` | Radar state renderer must reproduce native order and radar dirty rect effects | Radar opening/closing while sidebar strip has text/progress produces same overpaint order | Do not infer final order from current Rust buffer order |
| Smooth-scroll extra row is drawn but hit zones stay static | `0x006A9540`; prior `0x006A8220`, `0x006AAD00` reports | Rust scroll rows are instant/static; no native smooth offset/extra row | `src/sidebar/sidebar_view.rs`, `src/sidebar/mod.rs`, `src/app_sidebar_build.rs` | Draw an extra visual row during scroll, offset by native pixel offset, but keep click zones static | During scroll, visible extra row cannot be clicked; clicking static first row maps through `ScrollPosition * 2` | Do not make animated row positions drive hit-test rects |
| Observer branch shares strip layer slot | `0x006A9540`, `0x006A6C30` | observer sidebar path unchecked/missing | future observer sidebar view; existing sidebar batch split | Observer strip content should occupy strip layer before power/radar, not a global HUD overlay | Observer sidebar rows/icons are under later power/radar draws where overlap exists | Do not create observer branch as always-top UI |

## 8. Stale Docs / Follow-Up Wording

- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`: add that tooltip visible z-order is governed by `RenderFrame_main @ 0x004F4580`, which calls the tooltip manager after the sidebar surface display-chain copy; sidebar-local draw order alone does not place tooltips.
- `docs/research/POWER_BAR_PIXEL_RENDERING_LAYOUT_TRACE.md`: replace any implication that power is chrome-layer-adjacent with: "`PowerClass::Draw` runs after `StripClass::Draw`; it then calls `RadarClass::Draw`, so power is between strip and radar in native sidebar-local order."
- Observer/palette docs: when describing observer sidebar, state that the scoped observer branch is inside `StripClass::Draw`, sharing the strip layer position before power/radar; do not call it a post-sidebar overlay without separate evidence.

## Sources

- Ghidra decompile: `SidebarClass::Draw @ 0x006A6C30`
- Ghidra decompile: `StripClass::Draw @ 0x006A9540`
- Ghidra decompile: `SBGadgetClass::Draw @ 0x0069DEB0`
- Ghidra decompile: `PowerClass::Draw @ 0x0063FB20`
- Ghidra decompile: `RadarClass::Draw @ 0x00653100`
- Ghidra decompile: `RadarClass::Update @ 0x00656EC0`
- Ghidra decompile: `RadarClass::PerFrameMovieUpdate @ 0x006579E0`
- Ghidra decompile: `RenderFrame_main @ 0x004F4580`
- Ghidra decompile: `PowerClass::RegisterTooltip @ 0x006403A0`
- Ghidra decompile: `PowerClass::GetTooltipText @ 0x00640450`
- Ghidra decompile: `ToolTipManager::Register_Tip @ 0x00724580`
- `docs/research/SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_DRAW_COMPOSITION_ORDER_AND_SURFACE_ORIGIN_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`
- `docs/research/GSCREEN_RTACTICAL_GHIDRA_REPORT.md`
