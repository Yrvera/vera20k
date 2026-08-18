# Tooltip Text Source And Delay Timers - Ghidra Research Report

**Address(es):** `0x00724000`, `0x007241A0`, `0x00724200`, `0x00724520`, `0x00724530`, `0x00724540`, `0x00724580`, `0x006ABF80`, `0x006AC210`, `0x006A92E0`, `0x00640450`, `0x006AB990`, `0x006AB9E0`, `0x00479050`, `0x006D1800`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** in-game sidebar tooltip text source selection, delay/duration timers, hover reset behavior, cameo/tool/button/power source precedence, descriptor placement-byte selection, and visibility invalidation conditions.
**Non-Scope:** popup box pixels, glyph raster/wrapping internals, shell dialog `0x695` status tooltips, world/unit hover string semantics, and live runtime screenshot capture.
**Confidence:** High for standard in-game sidebar tooltip timing, registration, source precedence, and cameo delay override. Medium for the unnamed game-state guard at `0x00479050` because the guard was verified but not semantically named.
**Active in YR:** Yes for standard in-game sidebar/power/cameo/scroll/tab/sell/repair tooltip descriptors when the tooltip singleton `DAT_00887368` is constructed and enabled.

## Summary

In-game sidebar tooltips are driven by a Win32 `ToolTipManager` timer state machine, not game ticks. The default delay is `1000` ms, the auto-hide duration is `10000` ms, and the timer ID is `0x54544950` (`TTIP`). Ordinary mouse movement restarts the delay timer and hides any visible tooltip; button down/up events kill the timer and hide immediately.

The most important correction is that `ToolTipManager+0x228` is the delay value used by `WM_MOUSEMOVE`, not a hovered-tip pointer. Cameo highlight saves `+0x228`, writes `0`, and restores it on highlight off. That means cameo hover temporarily disables the delay so the next tooltip evaluation is immediate, while ordinary controls keep the default `1000` ms delay.

Text source selection has two tiers. If a registered descriptor contains a direct CSF key pointer, the popup loads that key directly. Otherwise `CCToolTip` calls its text resolver, which routes IDs through `SidebarClass__GetTooltipText`; that resolver asks `PowerClass__GetTooltipText` first, then handles scroll IDs, tab IDs, and valid cameo IDs.

## Target and Non-Scope

Target question: verify in-game sidebar tooltip text source, hover delay/timer reset behavior, source precedence for cameo/tool/button/power/gadgets, placement descriptor byte selection, and invalidation/visibility conditions.

Non-goals:

- Do not redo popup z-order, black fill, outline/text color, or retained-vs-primary surface proof except as citations.
- Do not redo `BitFont` sizing, wrapping, glyph raster, or cameo space-to-LF proof except as citations.
- Do not investigate shell dialog status-line tooltip child `0x695`.
- Do not investigate world/unit hover tooltip semantic generation.
- Do not edit Rust, INI, or tracked docs outside this research report.

Evidence needed to mark COMPLETE:

- Timer defaults and reset/hide branches verified from `ToolTipManager`.
- Descriptor registration and descriptor fields verified from sidebar/power registration.
- Direct-key-vs-ID resolver precedence verified from active show path.
- Cameo delay override verified from highlight enter/leave functions and timer reader.
- Rust-facing handoff and stale-doc wording provided.

Stop conditions:

- If a shell tooltip path appears, record as non-scope unless it contradicts the in-game popup path.
- If glyph or popup pixels appear, cite existing reports instead of redoing them.
- If a current Rust tooltip module does not exist, report `missing/unchecked` rather than inventing one.

## Verified Binary Findings

1. **Constructor defaults are `DelayMs=1000`, `DurationMs=10000`, timer ID `TTIP` is used later.**  
   Evidence: `ToolTipManager__Constructor @ 0x00724000` writes `0x3E8` to `this+0x228` and `0x2710` to `this+0x230`; `ToolTipManager__ProcessMessage @ 0x00724200` compares `WM_TIMER` `wParam` to `0x54544950` and uses the same ID in `KillTimer`/`SetTimer`. Active in YR: Yes, `Process_NetworkMessages @ 0x005D4D50` calls `0x00724200`.

2. **`ToolTipManager+0x228` is the live delay value, not a hovered-tip pointer.**  
   Evidence: `0x00724247..0x0072427B` reads `[ESI+0x228]`, tests it, and passes it as the `SetTimer` interval on `WM_MOUSEMOVE`; helper `0x00724520` writes `this+0x228`, `0x00724530` copies `+0x228` to `+0x22C`, and `0x00724540` restores it. Active in YR: Yes; `SelectClass` cameo highlight calls those helpers at `0x006AB99A`, `0x006AB9A7`, and `0x006AB9E9`.

3. **Ordinary mouse movement restarts the delay timer and hides an already visible tooltip.**  
   Evidence: `ToolTipManager__ProcessMessage @ 0x00724200`, `WM_MOUSEMOVE` branch `0x00724247..0x00724294`: if `+0x228 != 0` and paused flag `DAT_00A8F7D8 == 0`, it kills `TTIP`, sets a new timer using `+0x228`, then if `this+0x04` active tip is non-null calls vtable `+0x08` hide. Active in YR: Yes.

4. **Paused game or zero delay bypasses the delay timer and hit-tests immediately.**  
   Evidence: the same branch jumps to immediate `GetCursorPos`/`ScreenToClient` and table scan at `0x0072429E..0x00724323` when `+0x228 == 0` or `DAT_00A8F7D8 != 0`. Active in YR: Yes; zero delay is actively written by cameo highlight.

5. **Timer expiry has two phases: first fire shows; second fire hides.**  
   Evidence: `WM_TIMER` branch `0x00724332..0x007243E7` kills `TTIP`; if `this+0x04` is non-null it calls vtable `+0x08` hide; otherwise it hit-tests current mouse, calls `FUN_00724AD0`, and if that returns true schedules `SetTimer(..., this+0x230)`. Active in YR: Yes.

6. **Any mouse button down/up event kills tooltip timing and hides the active popup.**  
   Evidence: message table routes `WM_LBUTTONDOWN/UP`, `WM_RBUTTONDOWN/UP`, and `WM_MBUTTONDOWN/UP` (`0x201/0x202/0x204/0x205/0x207/0x208`) to `KillTimer`, then if `this+0x04` is non-null calls hide at `0x00724281..0x00724294`. Active in YR: Yes.

7. **Registered descriptor hit tests are inclusive on right and bottom edges.**  
   Evidence: both immediate/timer scans compare `x < left` and `x > left + width` as misses, then `y < top` and `y > top + height` as misses (`0x007242CD..0x007242F2`, `0x00724388..0x007243AD`). Therefore `x == left + width` and `y == top + height` are inside. Active in YR: Yes.

8. **`Enable(false)` is an immediate kill-and-hide operation.**  
   Evidence: `0x007241A0` changes `this+0x0C`; when disabling, it kills `TTIP`, then hides active `this+0x04` through vtable `+0x08`. Xrefs include `Main_Game`, `OptionsClass__ApplyFromInGameDialog`, and other setup/teardown callers. Active in YR: Yes.

9. **Sidebar registration creates descriptor records of ID, rect, direct key pointer, and final placement byte.**  
   Evidence: `FUN_00724580 @ 0x00724580` allocates `0x1C` bytes and copies six dwords plus one byte from the caller descriptor. `SidebarClass__InitSurface @ 0x006ABF80` fills those fields before each registration. Active in YR: Yes when `DAT_00887368 != 0`.

10. **Registration set and placement-byte classes are fixed for the standard sidebar.**  
    Evidence: `SidebarClass__InitSurface @ 0x006ABF80` registers tabs `0xCB..0xCE` with final byte `0`, visible cameo slots `1000..` with final byte `1`, sell `0x66` with direct key `TXT_SELL_MODE` and byte `0`, repair `0x65` with `TXT_REPAIR_MODE` and byte `0`, scroll up `200`/`0xC8` with byte `0`, and scroll down `0xC9` with byte `0`; `PowerClass__RegisterTooltip @ 0x006403A0` registers power ID `999` with byte `0`. Active in YR: Yes.

11. **Direct descriptor CSF key overrides the ID resolver.**  
    Evidence: `FUN_00724AD0 @ 0x00724AD0`, assembly `0x00724AE2..0x00724B06`: if descriptor `+0x14` is zero, it calls vtable `+0x14` with descriptor ID; if nonzero, it calls `StringTable__LoadString @ 0x00734E60` with the key pointer in `ECX`. Active in YR: Yes for sell/repair direct keys.

12. **The ordinary ID resolver first gates through a game-state check, then calls `SidebarClass__GetTooltipText`.**  
    Evidence: CCToolTip vtable `+0x14` entry at `0x00479050` calls a global object vtable `+0x28`; if the result is negative it returns null, otherwise it calls `FUN_006D1800` with `ECX=0x87F7E8`, and `FUN_006D1800 @ 0x006D1800` tail-calls `SidebarClass__GetTooltipText`. Active in YR: Yes for descriptor records with null direct key.

13. **`SidebarClass__GetTooltipText` asks power/radar resolver first, then handles sidebar IDs.**  
    Evidence: `SidebarClass__GetTooltipText @ 0x006AC210` begins with `PowerClass__GetTooltipText(param_1)` and returns it if non-null; then handles `200` -> string ID `0x13CD`, `0xC9` -> `0x13D3`, tabs `0xCB..0xCE` -> `0x13DB/0x13DD/0x13DF/0x13E1`, and valid cameo IDs `>=1000` -> `SidebarClass__GetCameoTooltip(id-1000)`. Active in YR: Yes.

14. **Power tooltip ID `999` formats current player power values.**  
    Evidence: `PowerClass__RegisterTooltip @ 0x006403A0` registers ID `999`; `PowerClass__GetTooltipText @ 0x00640450` checks `param_1 == 999`, reads `g_PlayerPtr+0x53A4` and `+0x53A8`, loads string ID `0x29E`, formats into `g_PowerTooltipBuf`, and returns that buffer. Active in YR: Yes with an active local player.

15. **Cameo tooltip validity is bounded by game active state, scroll offset, visible count, and max strip entries.**  
    Evidence: `SidebarClass__GetCameoTooltip @ 0x006A92E0` adds `strip+0x44 * 2` to the slot index, then requires `g_GameActive != 0`, adjusted index `< strip+0x54`, and `strip+0x54 < 0x4B`. It returns null otherwise. Active in YR: Yes.

16. **Superweapon cameos bypass normal cost/power formatting.**  
    Evidence: `SidebarClass__GetCameoTooltip @ 0x006A92E0` checks cameo entry type/value at `entry+0x5C == 0x1F` and returns the wide string at `SuperWeaponTypeClass[entry+0x58]+0x60`. Active in YR: Yes for superweapon cameo entries.

17. **Normal build cameos choose one of two format strings based on `DAT_00884B8C`, then replace every space with LF.**  
    Evidence: `0x006A92E0` calls object type vtable `+0x84`, uses string ID `0xC6E` with cost/power when `DAT_00884B8C == 0`, or string ID `0xC6C` otherwise; after formatting into `DAT_00B07BC4`, it loops over the UTF-16 buffer and rewrites `0x20` to `0x0A`. Active in YR: Yes for valid build cameo entries.

18. **Cameo highlight temporarily disables the tooltip delay, and unhighlight restores it.**  
    Evidence: `SelectClass` highlight-on assembly `0x006AB994..0x006AB9A7` loads `DAT_00887368`, calls save `0x00724530`, then pushes `0` and calls set-delay `0x00724520`; highlight-off `0x006AB9E3..0x006AB9E9` calls restore `0x00724540`. Active in YR: Yes for cameo mouse enter/leave handlers.

19. **Cameo highlight also invalidates strip/sidebar visual state independently of tooltip text.**  
    Evidence: highlight-on/off write `SelectClass+0x34`, set owning strip `+0x3C = 1`, call `FUN_004F42F0(0)`, and set `DAT_00884B8F = 1` and `DAT_00B0B518 = 1` (`0x006AB9AC..0x006AB9C9`, `0x006AB9EE..0x006ABA0C`). Active in YR: Yes.

20. **Hide clears active tooltip rectangle and active pointer.**  
    Evidence: `CCToolTip` hide path at `0x00478DB0` conditionally sets display invalidation flags and calls `FUN_00724AB0`; `FUN_00724AB0 @ 0x00724AB0` zeros `this+0x18..0x24` and active pointer `this+0x04`. Active in YR: Yes for timer/button/enable hide branches.

## Active in Standard YR?

Yes, with ordinary conditions:

- The tooltip manager must exist (`DAT_00887368 != 0`) and be enabled (`this+0x0C == 1`).
- Standard message routing reaches `ToolTipManager__ProcessMessage` through `Process_NetworkMessages @ 0x005D4D50`.
- Sidebar descriptors are registered from `SidebarClass__InitSurface @ 0x006ABF80`; power descriptor is registered from `PowerClass__RegisterTooltip @ 0x006403A0`.
- Render-frame popup pixels are covered by `TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS_GHIDRA_REPORT.md`; this report only verifies text source and timing.

Conditional active behavior:

- Paused state (`DAT_00A8F7D8 != 0`) makes non-cameo mousemove evaluation immediate.
- Cameo highlight writes delay `0`, so cameo-hover timing is immediate until highlight off restores the saved delay.
- CCToolTip vtable `+0x14` returns null if its unnamed global game-state guard returns a negative value; the guard was verified but not semantically named in this slice.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Default in-game tooltip timing is 1000 ms delay, 10000 ms visible duration, real Win32-style milliseconds; mousemove restarts delay and hides visible tip. | `0x00724000`, `0x00724247..0x007243E7` | Missing: no native in-game sidebar tooltip state machine found by focused `rg`; only shell/egui hover text exists. | Future tooltip module; `src/app.rs`/input event loop; `src/app_render/draw_passes.rs` | Track tooltip timers in wall-clock milliseconds separate from sim ticks; reset on every mouse move; auto-hide after duration. | Stop moving over a scroll/tab control for exactly the delay, then leave it visible until duration expires. | `test_sidebar_tooltip_mousemove_restarts_1000ms_delay_and_10000ms_duration`; do not tie tooltip timing to game ticks. |
| Cameo hover saves delay, sets delay to zero, and restores it on leave. | `0x006AB994..0x006AB9A7`, `0x006AB9E3..0x006AB9E9`, `0x00724520/30/40` | Missing: current sidebar hit-test has no hover-enter/leave state or tooltip delay override. | `src/sidebar/sidebar_view.rs`; future tooltip/hover tracker | Add hover transition tracking for cameo cells that temporarily switches delay to immediate while highlighted. | Moving onto a cameo can show immediately, moving off restores ordinary 1000 ms delay for tabs/scroll/power. | `test_cameo_hover_temporarily_zeroes_tooltip_delay_and_restores_on_leave`; do not treat `+0x228` as a hovered-tip pointer. |
| Source precedence is direct descriptor CSF key first, otherwise ID resolver; resolver checks power first, then scroll/tab/cameo. | `0x00724AE2..0x00724B06`, `0x006AC210`, `0x00640450` | Missing: no in-game tooltip text resolver found; shell status help is unrelated. | Future sidebar tooltip resolver; localization/string-table surface; `src/sidebar/sidebar_view.rs` descriptor model | Preserve descriptor direct-key bypass for sell/repair and ID-based resolver for power/tabs/scroll/cameos. | Sell/repair use `TXT_SELL_MODE`/`TXT_REPAIR_MODE` direct keys; power ID `999` formats current power; cameo IDs use adjusted strip entry. | `test_sidebar_tooltip_text_source_precedence_direct_key_power_then_sidebar_ids`; do not use one generic label string per control. |
| Registered hit rectangles are inclusive at right/bottom edges and descriptor byte controls placement class. | `0x007242CD..0x007242F2`, `0x00724388..0x007243AD`, `0x006ABF80` | Current `Rect::contains` behavior unchecked here; tooltip descriptors absent. | `src/sidebar/mod.rs`; `src/sidebar/sidebar_view.rs`; future tooltip placement | Tooltip hit-testing must use native inclusive descriptor bounds, and placement byte `1` only for visible cameos. | Cursor exactly on `left+width`/`top+height` still hits the tooltip descriptor; cameo placement differs from power/scroll. | `test_sidebar_tooltip_descriptor_hit_test_includes_right_bottom_and_uses_cameo_placement_byte`; do not reuse Rust action hit-test policy blindly. |
| Button events and disabling hide immediately and clear active rect/pointer; cameo highlight visual invalidation is separate. | `0x007241A0`, `0x00724281..0x00724294`, `0x00478DB0`, `0x00724AB0`, `0x006AB990/0x006AB9E0` | Missing. | Future tooltip state; `src/app_input.rs`; retained sidebar dirty model | Mouse button down/up cancels active tooltip; enable false hides; cameo highlight dirties strip/sidebar but tooltip overlay remains post-copy. | Start a sidebar drag/click while a tooltip is visible: popup disappears immediately and active state clears. | `test_sidebar_tooltip_mouse_button_and_disable_hide_and_clear_active_state`; do not leave tooltip visible through click/drag. |

## Negative Facts / Do Not Do

- Do not model `ToolTipManager+0x228` as a hovered-tip pointer. It is read as the `SetTimer` delay interval at `0x00724247..0x0072427B`; active tooltip pointer is `this+0x04`.
- Do not make cameo tooltips wait the normal `1000` ms while highlighted. `SelectClass::HighlightOn` saves delay and sets it to `0` at `0x006AB994..0x006AB9A7`.
- Do not use egui or shell `0x695` status-line tooltip behavior for in-game sidebar popups. In-game descriptors are registered by `0x006ABF80`/`0x006403A0` and rendered by the `CCToolTip` path covered in prior tooltip reports.
- Do not derive sell/repair text through the generic ID table. Their descriptors carry direct CSF keys `TXT_SELL_MODE` and `TXT_REPAIR_MODE`, and `FUN_00724AD0` loads descriptor keys before calling the ID resolver.
- Do not make hit tests half-open unless a later action-click investigation proves a separate click policy. Tooltip descriptor scans include `left+width` and `top+height`.
- Do not keep a tooltip open during mouse button down/up or after `Enable(false)`; both paths hide immediately.

## Remaining Uncertainty

- The unnamed guard in `CCToolTip` resolver `0x00479050` was verified as a negative-return gate before `SidebarClass__GetTooltipText`, but this slice did not name the global object at `DAT_00887640` or enumerate all cases where it returns negative.
- `FUN_00658770` / `FUN_004AE4F0`, reached after power ID miss, was touched but not exhausted. It may supply radar/other non-sidebar tooltip strings; this report only claims sidebar/power/cameo/registered gadget paths.
- Live screenshot/runtime input capture was not taken, so this report does not sample actual millisecond timing on a running process.
- Current Rust implementation status is from focused text scans only; no code was changed or compiled.

## Stale-Doc Replacement Wording

`C:/Users/enok/Documents/ra2-rust-game/docs/research/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md` should replace the `+0x228` / `+0x22C` table rows and the `Highlight -> tooltip interaction` wording.

Replace wording like:

> `+0x228` is `HoveredTip_Active` / currently-hovered tip pointer, also storing `DelayMs`, and `+0x22C` is a saved hovered backup while a cameo overrides the tooltip target.

With:

> `+0x228` is the live tooltip delay interval in milliseconds. The constructor writes `1000`, `ToolTipManager::ProcessMessage` reads it on `WM_MOUSEMOVE` and passes it to `SetTimer`, and `SelectClass::HighlightOn` saves it to `+0x22C` before writing `0` so cameo hover evaluates immediately. `SelectClass::HighlightOff` restores `+0x228` from `+0x22C`. The active tooltip descriptor pointer is `ToolTipManager+0x04`, not `+0x228`.

Replace wording like:

> Cameos override the tooltip target while hovered; leaving restores the previous tooltip target.

With:

> Cameo highlight overrides the tooltip delay, not the active target pointer: highlight-on saves `DelayMs`, sets it to zero, marks the cameo/strip dirty, and highlight-off restores the delay. The next tooltip target is still selected by the normal descriptor hit-test in `ToolTipManager::ProcessMessage`.

## Status

COMPLETE for the scoped in-game sidebar tooltip text-source and delay-timer slice. The standard YR timing state machine, descriptor registration, source precedence, cameo delay override, and hide/invalidation conditions were verified from binary evidence. Remaining uncertainty is limited to the unnamed resolver guard, adjacent radar text resolver fallback, and live runtime capture.

## Sources

- Ghidra decompile / assembly: `ToolTipManager__Constructor @ 0x00724000`
- Ghidra decompile / assembly: `ToolTipManager__Enable @ 0x007241A0`
- Ghidra decompile / assembly: `ToolTipManager__ProcessMessage @ 0x00724200`
- Ghidra decompile / assembly: `0x00724520`, `0x00724530`, `0x00724540`
- Ghidra decompile / assembly: `ToolTipManager::RegisterTip @ 0x00724580`
- Ghidra decompile / assembly: `SidebarClass__InitSurface @ 0x006ABF80`
- Ghidra decompile / assembly: `SidebarClass__GetTooltipText @ 0x006AC210`
- Ghidra decompile / assembly: `SidebarClass__GetCameoTooltip @ 0x006A92E0`
- Ghidra decompile / assembly: `PowerClass__RegisterTooltip @ 0x006403A0`
- Ghidra decompile / assembly: `PowerClass__GetTooltipText @ 0x00640450`
- Ghidra assembly: `CCToolTip` resolver vtable slot `0x00479050`
- Ghidra decompile: `FUN_006D1800 @ 0x006D1800`
- Ghidra decompile / assembly: `SelectClass` highlight enter/leave `0x006AB990`, `0x006AB9E0`
- Prior reports cited for non-scope pixels/glyphs: `TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS_GHIDRA_REPORT.md`, `TOOLTIP_GLYPH_RASTER_LINE_WRAPPING_GHIDRA_REPORT.md`
- Focused Rust scan: `src/app.rs`, `src/app_input.rs`, `src/app_sidebar_render.rs`, `src/sidebar/mod.rs`, `src/sidebar/sidebar_view.rs`, `src/render/bit_font.rs`, `src/render/sidebar_text.rs`
