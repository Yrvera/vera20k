# Sidebar Draw Composition Order And Surface Origin - Ghidra Report

Status: COMPLETE
Date: 2026-05-27
Scope: `SidebarClass::Draw @ 0x006A6C30` and immediate draw helpers needed for composition order, surface choice, clipping, origin offsets, dirty/full-redraw behavior.

## Target Question

What exact order does active YR use to compose sidebar chrome, gadgets, active-strip cameos/status overlays, power/radar layers, tooltips, and the final sidebar-surface blit, and which coordinate origin/surface is used for those draws?

## Non-goals

- Do not re-trace `AddCameo`, `InsertEntry`, `CompareItems`, or build availability ordering.
- Do not re-prove palette construction except where a helper consumes an already-known ConvertClass.
- Do not investigate radar SHP filename selectors, minimap movie sequencing, or exact radar rects beyond the fact that `PowerClass::Draw` calls `RadarClass::Draw`.
- Do not edit Rust, INI, published docs, or Ghidra metadata.

## Evidence Needed To Mark COMPLETE

- Read-only Ghidra decompile of `SidebarClass::Draw @ 0x006A6C30`.
- Read-only Ghidra decompile of immediate draw helpers: `SBGadgetClass::Draw @ 0x0069DEB0`, `StripClass::Draw @ 0x006A9540`, `PowerClass::Draw @ 0x0063FB20`, and `SidebarClass::BlitToScreen @ 0x006A70E0`.
- Handoff-critical draw-order/origin claims backed by decompile plus address-range evidence from the same functions or already-audited sibling reports.
- Active-in-YR labels for each material draw path.

## Stop Conditions

- Ghidra MCP unavailable or only mutating inspection possible.
- Helper expansion turns into a full sidebar system trace rather than immediate draw composition.
- Evidence cannot distinguish draw order from load order.

## Verified Findings

### 1. `SidebarClass::Draw` redirects the global primary draw target to the sidebar surface for the duration of the draw.

Active in YR: Yes. This is the vtable draw method for the live `SidebarClass` singleton and is cited as active by `SIDEBAR_SYSTEM_GHIDRA_REPORT.md`.

Evidence: read-only Ghidra decompile of `SidebarClass::Draw @ 0x006A6C30` shows it saves `g_PrimarySurface`, copies four surface-rect globals from `DAT_00b07dc8..dd4` into `DAT_008809f4..a00`, assigns `g_PrimarySurface = g_SidebarSurface`, and restores the saved primary surface before returning. Ghidra assembly range checked read-only with `disassemble_bytes 0x006A6C30..0x006A7110`.

Implication: sidebar chrome, gadgets, strip contents, power meter, and radar draw into an off-screen sidebar surface first; screen-space placement happens at `SidebarClass::BlitToScreen`, not at each `CC_Draw_Shape` call.

### 2. Full background composition is conditional, and the side-piece order is `SIDE1`, repeated `SIDE2`, `SIDE3`, then `ADDON`.

Active in YR: Yes, conditional on sidebar active, not map editor, and the redraw flags in the live draw method.

Evidence: `SidebarClass::Draw @ 0x006A6C30` decompile enters the chrome block only when `this+0x53A5` sidebar-active is nonzero, one of `forceRedraw`, `this+0x53A6`, or `this+0x53A7` is set, and `g_IsMapEditor == 0`. Inside that block, the `SIDE1/SIDE2/SIDE3/ADDON` draws are further gated by `this+0x53A7`. It draws `DAT_00b0b468` frame 0 at x 0, y `g_SidebarWidth`; loops `DAT_00b0b46c` using the SHP height at `+4`; then draws `DAT_00b0b470` and `DAT_00b0b474`. These are the `SIDE1.SHP`, `SIDE2.SHP`, `SIDE3.SHP`, and `ADDON.SHP` globals established by `SIDEBAR_SOVIET_SHP_LOAD_PATH_FUN_006D02B0_GHIDRA_REPORT.md`. Existing audited assembly citations in `SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md` show chrome `CC_Draw_Shape` flag `0x400` in `SidebarClass::Draw` at `0x006A6D16` and `0x006A6E03`.

Implication: layout code must not treat `SIDE2` tiling as an unconditional per-frame background repaint. Dirty redraw can skip the background pieces while still drawing gadgets/strip/power layers.

### 3. Gadget buttons are drawn after background side pieces and before the active strip.

Active in YR: Yes. The same `SidebarClass::Draw` path calls `SBGadgetClass::Draw @ 0x0069DEB0` for sidebar gadgets when the chrome/gadget redraw block runs.

Evidence: `SidebarClass::Draw @ 0x006A6C30` decompile calls `SBGadgetClass__Draw` twice, then iterates `DAT_00b07c48` to `< 0xb07dc8` in 0x60-byte strides for the four tab gadgets, then calls two more `SBGadgetClass__Draw` instances. `SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md` identifies that order as Sell, Repair, four tabs, then scroll arrows, and confirms `SBGadgetClass::Draw @ 0x0069DEB0`. `SBGadgetClass::Draw` decompile uses `g_SidebarSurface` when gadget `+0x4C != 0`, draws `gadget+0x58` with frame chosen from `+0x1E/+0x40/+0x34/+0x2D`, and sets `gadget+0x54 = 1`.

Implication: repair/sell/tab/scroll button SHPs are painter-ordered above background chrome but below active-strip cameo/status/progress layers.

### 4. Active strip contents draw before `PowerClass::Draw`; `PowerClass::Draw` draws `POWERP.SHP` strips and then calls `RadarClass::Draw`.

Active in YR: Yes. `SidebarClass::Draw` unconditionally calls the active strip draw and then the inherited power/radar draw in the live visible-sidebar path.

Evidence: `SidebarClass::Draw @ 0x006A6C30` decompile calls `StripClass__Draw(this + activeTab * 0xF94 + 0x1544, this+0x53A7)` before `PowerClass__Draw(this)`. `ORDINARY_BUILD_CAMEO_PALETTE_PATH_GHIDRA_REPORT.md` cites the active strip call at `0x006A6FDF`. `PowerClass::Draw @ 0x0063FB20` decompile draws `g_PowerBarSHP` frames 0/4/1/2/3 at x `5` for theater 0 and x `0` otherwise, y starting `g_SidebarWidth + 0x45`, then calls `RadarClass__Draw`. `POWER_BAR_PIXEL_RENDERING_LAYOUT_TRACE.md` already verified the `POWERP.SHP` frame order and y += 3 segment cadence from the same function.

Implication: native painter order is background/gadgets -> active strip cameo/status/progress -> power meter -> radar draw -> tooltip/blit. A renderer that batches `POWERP.SHP` inside the chrome pass before cameo/gclock/status layers is not a faithful global order, even if stock assets rarely overlap those regions.

### 5. Dirty flags are folded after strip/power draw, then `SidebarClass::BlitToScreen` copies from sidebar-surface local coordinates to screen coordinates.

Active in YR: Yes. The blit is the final output step whenever the sidebar is active and the game/display gates allow it.

Evidence: `SidebarClass::Draw @ 0x006A6C30` clears `DAT_00b07e4c`, `DAT_00b0b3f4`, each gadget dirty byte at offsets ending `+0x54`, `DAT_00b0b37c`, and `DAT_00b0b45c`, setting `DAT_00b0b518 = 1` when any were dirty. It draws tooltip singleton `DAT_00887368` via vtable `+0x0C`, computes a blit-needed byte from `DAT_00b0b518 || this+0x53A7`, calls `SidebarClass__BlitToScreen(this, byte)`, then clears `this+0x53A6` and `this+0x53A7`. `SidebarClass::BlitToScreen @ 0x006A70E0` decompile guards `this+0x53A5`, `g_GameActive`, and `DAT_00a8ed5c`, then copies regions from `g_SidebarSurface` to `DAT_00887308` using source rects based on sidebar-surface coordinates and destination x adjusted by `g_RadarViewportWidth` when `DAT_00a8eb7c` is set.

Implication: Rust should model sidebar-local layout first and apply exactly one screen placement transform/blit-equivalent. Per-layer screen offsets must not diverge from the shared sidebar surface origin.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Native draws active strip before `PowerClass::Draw`, and `PowerClass::Draw` calls radar after the power meter. | Current Rust builds power meter in `build_sidebar_chrome_instances_for_layout` and uploads/draws `sidebar_chrome` before `sidebar_cameo`, `sidebar_gclock`, overlay, and text. | `src/app_sidebar_build.rs`, `src/app_render/draw_passes.rs` | In a diagnostic overlap fixture, cameo/progress/text must appear before power/radar according to native painter order, or the Rust renderer must prove no overlap for stock and modded sidebar art. | `test_sidebar_draw_order_active_strip_before_power_and_radar` | HIGH screenshot/parity risk for modded or widened sidebar art; medium for stock visible regions. |
| Background side pieces draw only inside the redraw block and `SIDE1/SIDE2/SIDE3/ADDON` are additionally gated by strip/full redraw byte `+0x53A7`. | Current Rust emits chrome instances every frame with no native dirty/full-redraw distinction. | `src/app_sidebar_build.rs`, future sidebar-surface cache | Toggle only a tab/gadget dirty state and assert the surface cache repaints gadget/strip layers without forced side-piece repaint unless the native full-strip redraw flag is set. | `test_sidebar_background_side_pieces_respect_native_redraw_gate` | MEDIUM; affects transition/flicker parity and dirty-region correctness. |
| Sidebar draws into `g_SidebarSurface` using sidebar-local coordinates, then blits once to screen coordinates. | Current Rust emits independent screen-space UI batches for minimap, chrome, radar animation, cameo, overlays, and text. | `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`, sidebar layout model | Resize/toggle sidebar/radar viewport offset and verify all sidebar layers move by the same shared origin, with no layer-specific drift. | `test_sidebar_layers_share_single_surface_origin_transform` | HIGH for pixel layout parity; frequent in every visible sidebar frame. |

## Negative Facts / Do Not Do

- Do not draw `POWERP.SHP` as part of the same native layer as `SIDE1/SIDE2/SIDE3`; evidence: `SidebarClass::Draw @ 0x006A6C30` calls `PowerClass::Draw` only after `StripClass::Draw`.
- Do not draw repair/sell/tab/scroll gadgets underneath `SIDE1/SIDE2/SIDE3`; evidence: `SidebarClass::Draw` draws gadgets after background side pieces, and `SBGadgetClass::Draw @ 0x0069DEB0` targets the sidebar surface.
- Do not treat every `SidebarClass::Draw` invocation as a full side-piece repaint; evidence: the `SIDE1/SIDE2/SIDE3/ADDON` calls are inside the `this+0x53A7` branch.
- Do not apply separate screen origins to cameo, gclock, text, power, and radar layers; evidence: all scoped helpers draw to `g_SidebarSurface`/sidebar-local coordinates and `SidebarClass::BlitToScreen @ 0x006A70E0` performs the screen copy.
- Do not infer asset load order as draw order; evidence: `LoadSHPs` loads `GCLOCK2/SELL/REPAIR/TAB/R-UP/R-DN/SIDE1/SIDE2/SIDE3/ADDON/POWERP`, but `SidebarClass::Draw` composes background/gadgets/strip/power/radar in a different order.

## Remaining Uncertainty

- Exact `RadarClass::Draw` internal layer order is outside this slot; only the fact that it is called after `POWERP.SHP` segments was verified here.
- Exact tooltip singleton content and whether it can overlap sidebar pixels was not traced; this slot only verified tooltip draw happens before blit.
- The decompiler names `g_SidebarWidth`, `g_SidebarTopClip`, and some surface/destination locals remain semantically risky; coordinate conclusions here are based on read/write use, not variable names.
- Stock asset overlap between `POWERP.SHP`, cameo layers, radar layers, and text was not checked from retail SHP dimensions in this slot.

## Stale Doc Wording Suggested

- `docs/research/traces/POWER_BAR_PIXEL_RENDERING_LAYOUT_TRACE.md`: replace the Stage 9 sentence "SidebarClass chrome pieces (side1, side2, side3) order relative to bar: UNCHECKED" with "Fresh `SidebarClass::Draw @ 0x006A6C30` evidence shows background chrome/gadgets draw before `StripClass::Draw`, and `PowerClass::Draw @ 0x0063FB20` draws `POWERP.SHP` after the active strip and before `RadarClass::Draw`."
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md` drawing pipeline wording should expand item 7 from "Call `FUN_0063fb20` (PowerBar::Draw)" to "Call `PowerClass::Draw @ 0x0063FB20`; it draws `POWERP.SHP` segments and then calls `RadarClass::Draw`."

## Rust Reconnaissance

- `src/app_sidebar_build.rs:112..205` builds radar, `SIDE1`, tabs, tab-button SHPs, repeated `SIDE2`, `SIDE3`, sell/repair, and power meter together in the `sidebar_chrome` instance batch.
- `src/app_sidebar_build.rs:418..569` builds cameo base art, gclock/progress, and dark text backplates as separate batches.
- `src/app_render/draw_passes.rs:467..521` draws `sidebar_chrome` before `radar_anim`, `sidebar_cameo`, `sidebar_gclock`, `sidebar_cameo_overlay`, and `sidebar_text`.
- `src/app_render/build_instances.rs:718..745` constructs sidebar/chrome/cameo/text/radar batches independently rather than through a cached sidebar-surface composition model.

## Evidence Log

- Ghidra MCP read-only: `decompile_function 0x006A6C30`, `0x0069DEB0`, `0x006A9540`, `0x0063FB20`, `0x006A70E0`.
- Ghidra MCP read-only: `disassemble_bytes 0x006A6C30..0x006A7110` for top-level draw range confirmation.
- Corroborating sibling docs: `SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md`, `ORDINARY_BUILD_CAMEO_PALETTE_PATH_GHIDRA_REPORT.md`, `POWER_BAR_PIXEL_RENDERING_LAYOUT_TRACE.md`, `SIDEBAR_SYSTEM_GHIDRA_REPORT.md`.
