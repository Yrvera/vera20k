# Tooltip Manager Sidebar Overlap Pixels - Ghidra Report

**Address(es):** `0x004F44F0`, `0x00724200`, `0x00724AD0`, `0x00724B80`, `0x00478E10`, `0x00478E30`, `0x00478BA0`, `0x00478DB0`, `0x006ABF80`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** In-game `CCToolTip` / `ToolTipManager` popup rendering when a tooltip overlaps the sidebar after `SidebarClass::BlitToScreen`.
**Non-Scope:** Shell dialog status-line tooltips such as child `0x695`, shell owner-draw status text, full unit/world hover text semantics, and runtime debugger screenshot capture.
**Confidence:** High for z-order, target surface selection in the normal frame pass, fill/border/text colors, registration reachability, and dirty-state interaction. Medium for all placement edge cases because the show/placement helper was disassembled but not converted into a named Ghidra function.
**Active in YR:** Yes, for standard in-game sidebar tooltips when `DAT_00887368` exists and tooltips are enabled.

## Summary

In-game sidebar popups are not rendered inside the retained sidebar surface batch. `RenderFrame_main @ 0x004F44F0` performs the optional sidebar display-chain copy first, then calls the tooltip singleton vtable slot `+0x0C(0)`. That normal frame path sets `CCToolTip+0x260 = 0`, refreshes the active tooltip if needed, and draws the active popup to `g_PrimarySurface` / backbuffer when the popup falls in the visible screen region.

The popup pixel style is a black filled rectangle, a 1-pixel rectangle outline in the current sidebar text color, and one text draw in that same color. The verified popup path does not call `AlphaBlendRect(0,0xAF)`, does not use a sidebar SHP, and does not draw a separate text shadow.

## Target and Non-Scope

Target:

- Confirm the draw order relative to `SidebarClass::BlitToScreen`.
- Identify the target surface used for a tooltip overlapping the in-game sidebar.
- Decode the fill, border, text, clipping, and dirty/copy interaction visible to the player.
- Keep the scope to in-game tooltip popups registered by sidebar/power/radar code.

Non-scope:

- Shell dialog status text, including main menu and skirmish `0x695`.
- Unit/world hover text ownership or shroud semantics.
- Full runtime screenshot validation.
- Rust implementation patches.

## Verified Binary Findings

1. **Tooltip popup z-order is after the sidebar copy.**  
   Evidence: `RenderFrame_main @ 0x004F44F0` calls display-chain copy for `g_SidebarSurface` when `DAT_00B0B519 != 0`, clears `DAT_00B0B519`, then later calls `DAT_00887368->vtable+0x0C(0)` at `0x004F4562`. Active in YR: yes, in the standard render frame.

2. **The normal render-frame tooltip call uses `CCToolTip+0x260 = 0`.**  
   Evidence: `CCToolTip` vtable is installed as `0x007F74C4` by `CCToolTip__Constructor @ 0x007784A0`; vtable slot `+0x0C` points to `0x00478E10`. Disassembly `0x00478E10..0x00478E25` reads the stack byte argument, stores it at `[ECX+0x260]`, pushes logical NOT of that argument, and calls `FUN_00724B80`. Because `RenderFrame_main` passes `0`, the normal frame stores `0` and then calls `FUN_00724B80(1)`.

3. **`FUN_00724B80` draws only when an active tip exists, and it routes actual drawing through vtable slot `+0x10`.**  
   Evidence: `FUN_00724B80 @ 0x00724B80` first calls `FUN_00724AD0` when its argument is `1`, then tests `this+0x04` active-tip pointer, then calls `this->vtable+0x10(this+0x18)`. In `CCToolTip`, slot `+0x10` is `0x00478E30`. Active in YR: yes, because it is the direct callee of the render-frame tooltip slot.

4. **The active popup record is registered by the in-game sidebar setup, not by shell dialog tooltip code.**  
   Evidence: `SidebarClass__InitSurface @ 0x006ABF80` calls `PowerClass__RegisterTooltip`, then if `DAT_00887368 != 0` registers tab IDs `0xCB..0xCE`, cameo IDs `1000..`, sell `0x66`, repair `0x65`, scroll `0xC8/0xC9` through `FUN_00724580`. `PowerClass__RegisterTooltip @ 0x006403A0` registers power tooltip ID `999`. Active in YR: yes during sidebar surface initialization.

5. **The tooltip descriptor copied by `FUN_00724580` is 0x1C bytes: ID, x, y, width, height, optional string pointer, and a final byte flag.**  
   Evidence: `FUN_00724580 @ 0x00724580` allocates `0x1C`, copies six dwords and one byte from the caller stack descriptor, then stores the pointer in the manager tables. `SidebarClass__InitSurface` prepares exactly those fields before each call. Active in YR: yes for every registered in-game sidebar/power/radar tooltip.

6. **The actual popup draw path chooses `g_PrimarySurface` for the normal frame pass over the sidebar.**  
   Evidence: `0x00478E30` compares the popup rect against the tactical/sidebar split edge. In the ordinary branch, when the rect is on the screen/backbuffer side of the edge, it selects `DAT_0088731C` (`g_PrimarySurface` / frame backbuffer) and translates X by the edge as needed. The alternate `DAT_00887300` (`g_SidebarSurface`) branch is guarded by `CCToolTip+0x260 == 1`; the normal `RenderFrame_main` call sets that byte to `0`, so the retained sidebar-surface branch is not used for the visible frame tooltip.

7. **The split-surface branch can dirty the sidebar, but the visible normal frame tooltip does not.**  
   Evidence: `0x00478EC4` writes `DAT_00B0B518 = 1` only on the branch that selects `DAT_00887300` while `CCToolTip+0x260 == 1`. The render-frame path stores `0` at `+0x260` before draw, so a tooltip over the sidebar after `BlitToScreen` does not itself set `DAT_00B0B518`. Active in YR: the normal path is active; the split branch exists but was not proven as the standard visible frame path.

8. **Tooltip fill is a black rectangle, not `AlphaBlendRect(0,0xAF)`.**  
   Evidence: `0x00478E30` calls the selected surface vtable slot `+0x14` with the popup rect and a pushed color/value `0`. For a `DSurface`, slot `+0x14` resolves through `0x004BB5F0` to fill-rect worker `0x004BB620`. `get_function_callers(0x00621B80)` does not include the tooltip draw path; `AlphaBlendRect(0,0xAF)` callers are the sidebar dark text strips and owner-draw controls, not this popup path.

9. **Tooltip outline uses the current sidebar text color.**  
   Evidence: `0x00478F65..0x00478FDC` reads `DAT_00B0FA1C`, `DAT_00B0FA1D`, and `DAT_00B0FA1E`, packs them through the runtime DirectDraw channel shifts/loss values, then passes that packed color to selected surface vtable slot `+0x58`. For `DSurface`, slot `+0x58` resolves to `0x007BAD90`, which calls `0x007BADC0`; that worker draws four edges through surface vtable slot `+0x2C`. Active in YR: yes; `SetSidebarTextColor @ 0x0072F440` writes these globals for the active side.

10. **Tooltip text uses the same packed sidebar text color and a single draw call.**  
    Evidence: after the outline call, `0x00478E30` calls `FUN_00433C90(font,1)`, `FUN_00433CA0(font,&rect)`, `FUN_00433C70(font,packed_color)`, then calls `FUN_00434CD0` with the active wide text buffer, target surface, inner text origin, width/height, and zero fade arguments. There is no second offset text draw in this function, so no native popup text shadow was found in the scoped path.

11. **Tooltip inner text inset is 2 px horizontally and 4 px vertically from the outer rect used for fill/border.**  
    Evidence: immediately before the `FUN_00434CD0` call, disassembly at `0x00479024..0x0047902D` adds `2` to the left coordinate and `4` to the top coordinate before pushing the text origin. Width/height are derived from the outer rect deltas. Active in YR: yes on the active popup draw path.

12. **Text measurement and popup box sizing use `BitFont__MeasureText`, then add small padding.**  
    Evidence: show/placement function `0x00478BA0` calls `BitFont__MeasureText @ 0x00433CF0`; after the call it adds `4` to one measured dimension and `3` to the other (`0x00478C43..0x00478C66`), then ensures the active popup width/height are at least those values. Active in YR: yes through `FUN_00724AD0 -> vtable+0x04`.

13. **Placement differs for descriptor final-byte flag `0` versus `1`.**  
    Evidence: `0x00478BA0` checks the active descriptor byte at `+0x18`. When nonzero, it positions around the registered target rect with a `5` px gap and clamps against the chosen region's top/left (`0x00478CB6..0x00478D51`). When zero, it uses the mouse-derived rect, applies a `+16` vertical offset, and clamps right/bottom against the chosen region (`0x00478D58..0x00478D9A`). In `SidebarClass__InitSurface`, cameo hit-zone tooltips set final byte `1`; tab/repair/sell/scroll/power descriptors set `0`.

14. **Hide invalidates display-side state, not retained sidebar content in the normal path.**  
    Evidence: `CCToolTip` vtable slot `+0x08` at `0x00478DB0` checks the current point against the split edge, conditionally sets `DAT_00884B8E = 1` and `DAT_00884B8F = 1`, calls `FUN_004F42F0(0)`, then clears the active rect and active-tip pointer through `FUN_00724AB0`. It does not write `DAT_00B0B518`; the only tooltip-side `DAT_00B0B518` write found in the scoped path is the `+0x260 == 1` split-surface draw branch at `0x00478EC4`.

## Active in Standard YR?

Yes. The standard in-game path constructs the tooltip singleton in game startup, enables/disables it around game mode setup, routes Win32 messages through `Process_NetworkMessages @ 0x005D4D50 -> ToolTipManager::ProcessMessage @ 0x00724200`, registers in-game sidebar/power tooltip descriptors in `SidebarClass__InitSurface @ 0x006ABF80`, and draws the active popup from `RenderFrame_main @ 0x004F44F0`.

The standard visible frame path passes `0` to `CCToolTip+0x0C`, which means popup-over-sidebar pixels are drawn after the sidebar copy on `g_PrimarySurface`. The alternate retained-sidebar-surface branch exists but is not the normal visible frame pass.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Tooltip popups draw after `SidebarClass::BlitToScreen` on the primary/backbuffer in the normal frame pass. | `0x004F44F0`, `0x00478E10`, `0x00478E30` | Missing/unchecked; no in-game sidebar popup tooltip renderer found by `rg tooltip src/sidebar src/app_render`. | `src/app_render/draw_passes.rs`, `src/app_render/build_instances.rs`, future tooltip/input module | Add a post-sidebar-copy popup overlay stage, not a retained sidebar-surface layer. | `test_sidebar_tooltip_draws_after_sidebar_copy_and_overwrites_sidebar_pixels`: open a tooltip overlapping the sidebar and assert popup pixels cover chrome even when no sidebar dirty copy is requested that frame. | Do not draw sidebar tooltips inside `sidebar_chrome`, `sidebar_text`, or retained sidebar surface composition. |
| Popup pixel style is black fill, sidebar-text-color 1 px outline, and text in the same packed color. | `0x00478E30`, `0x004BB5F0`, `0x004BB620`, `0x007BAD90`, `0x0072F440` | Missing. Current Rust has shell status text and sidebar text, but no native in-game popup box. | future tooltip renderer, `src/render/sidebar_text.rs` for color source, side color setup | Fill with black, draw one outline rectangle in current side text color, then draw text once in that color. | `test_sidebar_tooltip_box_pixels_black_fill_yellow_outline_text_for_soviet`: Soviet tooltip over known background has black interior, yellow border/text, no alpha dark strip. | Do not use `AlphaBlendRect(0,0xAF)` or GPU translucent panels for popup background. |
| Text inset is outer left +2 and top +4; sizing uses measured text plus small native padding. | `0x00478BA0`, `0x00479024..0x0047902D`, `0x00433CF0` | Missing. | future tooltip layout module | Use native measured text dimensions and the verified text origin inside the popup box. | `test_sidebar_tooltip_text_inset_2_4_and_box_measure_padding`: text origin and border match native for single-line and multi-line cameo text. | Do not center the text in the box or use egui default tooltip padding. |
| Cameo descriptors use final byte `1` and are placed around the registered target rect with a 5 px gap; ordinary button/power descriptors use byte `0` and mouse-based placement with +16 y offset. | `0x006ABF80`, `0x006403A0`, `0x00478BA0` | Missing. | `src/sidebar/sidebar_view.rs`, future hover/tooltip state | Carry the descriptor final-byte behavior through registration and placement. | `test_cameo_tooltip_uses_target_rect_placement_but_scroll_button_uses_mouse_offset`: same mouse coordinate over cameo vs scroll button produces native-relative popup anchors. | Do not use one generic tooltip anchor rule for all sidebar controls. |
| Tooltip redraw is independent of sidebar dirty-copy state in the normal visible frame; hide/show set display invalidation state, not retained sidebar dirty. | `0x004F44F0`, `0x00478DB0`, `0x00478EC4` | Missing/unchecked. | `src/app_render/draw_passes.rs`, retained sidebar dirty implementation | Tooltip overlay should be scheduled as a post-copy overlay and should not force retained sidebar repaint except if a future proven split-surface mode is implemented. | `test_tooltip_visibility_toggle_does_not_mark_retained_sidebar_dirty`: moving off a tooltip hides it and invalidates display overlay but leaves retained sidebar content dirty flag untouched. | Do not tie tooltip visibility to `DAT_00B0B518` or require a sidebar blit to redraw the tooltip. |

## Negative Facts / Do Not Do

- Do not render in-game sidebar popups as part of the retained `g_SidebarSurface` batch in the normal frame path.
- Do not use `AlphaBlendRect(0,0xAF)` for the popup background. That helper belongs to Ready/queue/Hold dark text strips, not the scoped popup box.
- Do not draw a separate text shadow in the scoped in-game popup path; only one `FUN_00434CD0` text call was found.
- Do not reuse shell `0x695` status-line behavior for in-game sidebar popups. It is a different mechanism.
- Do not treat tooltip show/hide as equivalent to sidebar content dirtying. The normal visible draw is post-copy overlay work.
- Do not collapse descriptor final-byte `0` and `1` placement. Cameos and button/power regions place differently.

## Remaining Uncertainty

- Runtime screenshot capture was not performed, so this report does not provide sampled final pixels from a live game frame.
- The alternate `CCToolTip+0x260 == 1` retained-sidebar-surface branch is real and writes `DAT_00B0B518`, but this pass did not prove a standard visible in-game frame caller that passes `1`.
- Exact concrete DirectDraw surface class for every possible display mode was inferred from established `DSurface` vtable evidence; the active normal frame target and slot behavior are verified, but no live object vtable was captured with a debugger.
- World/unit hover tooltip text generation was not investigated; only the popup manager rendering and sidebar/power descriptor path are in scope.
- Font glyph raster internals in `FUN_00434CD0` were not re-audited beyond confirming a single draw call, color setup, and text rectangle.

## Stale-Doc Replacement Wording

Replace wording that says:

> `AlphaBlendRect(0, 0xaf)` is used for tooltip backgrounds.

With:

> In-game sidebar popup tooltips do not use `AlphaBlendRect(0,0xAF)`. The verified `CCToolTip` popup draw path fills the popup rect with black via the surface fill-rect slot, draws a 1 px outline in the current sidebar text color, and draws the tooltip text once in that same packed color. `AlphaBlendRect(0,0xAF)` applies to sidebar Ready/queue/Hold dark text strips, not this popup box.

Replace wording that says:

> Sidebar tooltips are a sidebar surface layer.

With:

> In the normal frame pass, sidebar-overlapping tooltip popups draw after `SidebarClass::BlitToScreen` from `RenderFrame_main` onto `g_PrimarySurface` / the frame backbuffer. They should be modeled as a post-sidebar-copy overlay, not as part of the retained sidebar-surface chrome/text batch.

## Status

COMPLETE for the scoped in-game sidebar popup overlap path: z-order, target surface in the normal render pass, fill/border/text colors, placement classes, and dirty-state interaction were verified from binary evidence. Remaining items are runtime capture and the conditional `+0x260 == 1` branch liveness.

## Sources

- `RenderFrame_main @ 0x004F44F0`
- `Process_NetworkMessages @ 0x005D4D50`
- `ToolTipManager::ProcessMessage @ 0x00724200`
- `FUN_00724AD0` active-tip text/show preparation
- `FUN_00724B80` active-tip draw dispatcher
- `CCToolTip__Constructor @ 0x007784A0`
- `CCToolTip` vtable pointer `0x007F74C4`
- `CCToolTip+0x0C @ 0x00478E10`
- `CCToolTip+0x10 @ 0x00478E30`
- `CCToolTip+0x04 @ 0x00478BA0`
- `CCToolTip+0x08 @ 0x00478DB0`
- `ToolTipManager::RegisterTip @ 0x00724580`
- `SidebarClass__InitSurface @ 0x006ABF80`
- `PowerClass__RegisterTooltip @ 0x006403A0`
- `SetSidebarTextColor @ 0x0072F440`
- `BitFont__MeasureText @ 0x00433CF0`
- `FUN_00434CD0` bit-font draw helper
- `DSurface` vtable `0x007E85D4`, including slot `+0x14 -> 0x004BB5F0` and slot `+0x58 -> 0x007BAD90`
- Prior docs: `SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`, `ALPHABLENDRECT_0xAF_DARK_STRIP_PIXEL_MATH_GHIDRA_REPORT.md`, `SIDEBAR_ODD_STATE_OVERLAP_STACK_GHIDRA_REPORT.md`
