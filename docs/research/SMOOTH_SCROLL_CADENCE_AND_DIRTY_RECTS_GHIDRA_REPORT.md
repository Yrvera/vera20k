# Smooth Scroll Cadence And Dirty Rects - Ghidra Research Report

Date: 2026-05-27
Swarm slot: 5
Target: `SMOOTH_SCROLL_CADENCE_AND_DIRTY_RECTS`

## Summary

The native sidebar strip scroll path is a row-request state machine, not a moving hit-zone model. Scroll button actions add or subtract a page-sized request after the current `SidebarClass::Action` call has already run `StripClass::AI` for all four strips. `StripClass::AI @ 0x006A8B30` then consumes one requested row per AI pass, sets the strip dirty flags, and invalidates the sidebar copy path.

The binary has a smooth-scroll draw branch in `StripClass::Draw @ 0x006A9540`: when `Strip+0x3F` is nonzero it draws `visible_rows + 1` rows and applies `ScrollPixelOffset - row_height` to every cameo row. However, stock layout initialization sets `DAT_00B0B500` row height and `DAT_00B0B514` scroll step both to `0x32` (50), so a started row scroll reaches its terminal condition inside the same `StripClass::AI` pass. The extra-row branch is real and must be modeled for mechanism parity, but standard stock YR does not prove a multi-frame easing cadence from these constants alone.

## Target and Non-Scope

Target:

- Verify strip smooth-scroll state fields, request consumption, direction semantics, animation step constants, dirty invalidation writes, draw-loop extra-row behavior, and hit-zone separation.
- Bound the standard-YR liveness of the scroll path.
- Identify current Rust touchpoints and acceptance tests.

Non-scope:

- Do not rediscover the full static 2-column layout beyond citing the existing proof for hit-zone separation.
- Do not re-audit sidebar chrome, tabs, sell/repair geometry, palette conversion, tooltip pixels, or radar overlap ordering.
- Do not implement Rust changes.
- Do not claim runtime screenshot/flicker visibility beyond binary state transitions.

Investigation mode: exhaustive-slice for the binary scroll state machine and dirty invalidation writes; runtime visual capture remains deferred.

## Verified Binary Findings

### 1. `SidebarClass::Action` runs strip AI before it handles the current scroll-button event

Active in standard YR: Yes when `g_IsMapEditor == 0`.

At the top of `SidebarClass::Action @ 0x006A7780`, the function calls a sidebar virtual at `+0xD8`, then iterates four strips and calls `StripClass::AI` on each one before any current event ID is decoded. The scroll-button branches are later in the same function.

Evidence:

- `SidebarClass::Action @ 0x006A7780`: `if (g_IsMapEditor == 0)`, loop count `4`, calls `StripClass__AI(this)`, advances by `0x3E5` dwords per strip.
- The scroll-down/up event checks happen after that loop, against `DAT_00B0B34C | 0x8000` and `DAT_00B0B42C | 0x8000`.

Implication: a click/held-repeat event changes `Strip+0x48 ScrollRequest` after the AI pass for that action call. The request cannot be consumed until a later `StripClass::AI` pass.

### 2. Scroll button actions enqueue page-sized row requests, but do not move hit zones or scroll position directly

Active in standard YR: Yes for ordinary player sidebar.

For scroll down, `SidebarClass::Action @ 0x006A7780` computes the native visible row count:

```text
rows = (((DAT_00886F9C - margin) - DAT_00B0B4F8 - 7 + g_SidebarWidth) / 0x32)
margin = 0x1A if ScenarioClass+0x34B8 == 0 else 0x12
```

If ordinary player mode and `(ScrollPosition + rows) * 2 < CameoCount`, it adds `rows` to `ScrollRequest`. In observer mode the request amount is `1`.

For scroll up, ordinary mode subtracts `rows` from `ScrollRequest` only if `ScrollPosition != 0`. Observer mode again uses `1`.

Evidence:

- `SidebarClass::Action @ 0x006A7780`: scroll-down branch writes `param_1[active_strip * 0x3E5 + 0x563] += iStack_10`.
- `SidebarClass::Action @ 0x006A7780`: scroll-up branch writes `param_1[active_strip * 0x3E5 + 0x563] -= iVar4`.
- Existing static-zone proof: `SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT_GHIDRA_REPORT.md`.

Implication: scroll button input is a request accumulator. It is not a direct page teleport and does not update any `SelectClass` hit rectangle.

### 3. `StripClass::AI` consumes one row per pass, with asymmetric up/down position timing

Active in standard YR: Yes.

Relevant strip fields:

| Offset | Meaning | Verified use |
|---|---|---|
| `+0x3E` | scroll direction | `0` = up, `1` = down in `StripClass::AI` |
| `+0x3F` | is scrolling | gates animation and extra draw row |
| `+0x44` | scroll position, in rows | used by AI, draw, and select action |
| `+0x48` | scroll request, in rows | positive down, negative up |
| `+0x4C` | scroll pixel offset | animated visual offset |
| `+0x50` | previous/last pixel offset copy | written at end of `StripClass::Draw` |
| `+0x54` | cameo/entry count | bounds for ordinary strip draw/action |

When `+0x3F == 0` and `+0x48 != 0`, `StripClass::AI @ 0x006A8B30` tries to start one row:

- Up request (`ScrollRequest < 0`): if `ScrollPosition != 0`, it increments `ScrollRequest` toward zero, writes `ScrollDirection = 0`, writes `IsScrolling = 1`, pre-decrements `ScrollPosition`, and writes `ScrollPixelOffset = 0`.
- Down request (`ScrollRequest >= 0`): if `(ScrollPosition + rows) * columns < total`, it decrements `ScrollRequest`, writes `ScrollPixelOffset = DAT_00B0B500`, writes `ScrollDirection = 1`, and writes `IsScrolling = 1`. `ScrollPosition` is not incremented yet.

Then, in the same `StripClass::AI` call if `IsScrolling` is nonzero:

- Direction `0` adds `DAT_00B0B514` to `ScrollPixelOffset`; if the result is `>= DAT_00B0B500`, it clears `IsScrolling` and resets offset to `0`.
- Direction `1` subtracts `DAT_00B0B514`; if the result is `< 1`, it clears `IsScrolling`, resets offset to `0`, and post-increments `ScrollPosition`.

Evidence:

- `StripClass::AI @ 0x006A8B30`; disassembly range confirmed `0x006A8B30..0x006A8C9F`.
- `DAT_00B0B500 = 0x32` and `DAT_00B0B514 = 0x32` from `SIDEBAR_INIT_LAYOUT_GLOBALS_EXACT_RECHECK_GHIDRA_REPORT.md`.

Implication: the up/down asymmetry is load-bearing. Up changes logical top row before animation; down changes it after animation completion.

### 4. Stock YR constants make one requested row complete in one AI pass

Active in standard YR: Yes for stock layout constants.

`DAT_00B0B500` row height is `50`, and `DAT_00B0B514` scroll step is also `50`. Because `StripClass::AI` immediately runs the animation block after starting a row, the terminal condition is reached inside the same pass:

- Up: `0 + 50 >= 50`, so `IsScrolling` clears and offset returns to `0`.
- Down: `50 - 50 < 1`, so `IsScrolling` clears, offset returns to `0`, and `ScrollPosition++`.

Evidence:

- `StripClass::AI @ 0x006A8B30` start-plus-animate flow.
- `SIDEBAR_INIT_LAYOUT_GLOBALS_EXACT_RECHECK_GHIDRA_REPORT.md`: `DAT_00B0B500 = 0x32`, `DAT_00B0B514 = 0x32`.

Implication: do not add multi-frame easing or acceleration to stock sidebar scroll. The draw-time extra-row mechanism is real, but the stock constants mean the normal row step is effectively one AI pass unless another live path changes those globals.

### 5. `StripClass::Draw` supports an extra visual row only when `IsScrolling` is nonzero

Active in standard YR: Code path yes; visible multi-frame effect conditional on `Strip+0x3F` surviving to draw.

`StripClass::Draw @ 0x006A9540` returns immediately if the strip is inactive (`+0x1C == 0`) or if neither strip dirty (`+0x3C`) nor force-redraw parameter is set. When drawing, it clears `+0x3C`, sets `DAT_00B0B518 = 1`, computes:

```text
draw_rows = visible_rows + (Strip+0x3F != 0)
entry_index = col + (draw_row + ScrollPosition) * 2
y = StripY + 1 + DAT_00B0B500 * draw_row
if IsScrolling:
    y += ScrollPixelOffset - DAT_00B0B500
```

There is no `SelectClass` hit-zone rebuild in this draw path, and no extra hit zone for the extra row.

Evidence:

- `StripClass::Draw @ 0x006A9540`; disassembly range confirmed `0x006A9540..0x006A96AF`.
- `SidebarClass::InitSelectZones @ 0x006A8220` creates static `0x3C x 0x30` rectangles for visible rows only.
- `SelectClass::Action @ 0x006AAD00` maps clicks as `Select.CameoIndex + Strip.ScrollPosition * 2`, with no `ScrollPixelOffset` read.

Clipping note: this function does not establish a dedicated cameo-grid scissor around the extra row. It draws into `g_SidebarSurface` through the same `CC_Draw_Shape`, text, and alpha-rect primitives as normal rows. Exact primitive-level clipping is delegated below this function and was not re-expanded in this slot.

### 6. Dirty invalidation is explicit and separate from visual geometry

Active in standard YR: Yes.

When `StripClass::AI` changes scroll/progress/observer-visible state, it sets a dirty bundle:

- `DAT_00884B8E = 1`
- `(&DAT_00880D68)[DAT_00884B84 * 0xF94] = 1`
- `FUN_004F42F0(0)` twice, which sets `g_Tactical + 0xD7D = 1` when tactical exists
- `Strip+0x3C = 1`
- `DAT_00884B8F = 1`
- `DAT_00B0B518 = 1`

`StripClass::Draw` consumes the strip dirty byte by clearing `Strip+0x3C = 0` and sets `DAT_00B0B518 = 1` again before drawing. `SidebarClass::Draw @ 0x006A6C30` later uses `DAT_00B0B518 || Sidebar+0x53A7` to decide whether to call `SidebarClass::BlitToScreen` with a copy-needed byte, then clears `Sidebar+0x53A6` and `Sidebar+0x53A7`.

Evidence:

- `StripClass::AI @ 0x006A8B30`.
- `FUN_004F42F0 @ 0x004F42F0`.
- `StripClass::Draw @ 0x006A9540`.
- `SidebarClass::Draw @ 0x006A6C30`.

Implication: Rust needs a retained sidebar-surface dirty model that can invalidate the strip separately from hit-test geometry. Per-frame rebuilt sprite lists are not the native dirty-rect mechanism.

### 7. Scroll-button show/hide is capacity-based, not endpoint-based

Active in standard YR: Yes.

`SidebarClass::UpdateScrollButtons @ 0x006A6610` checks whether the current strip/observer list has more entries than the visible capacity. If not, it calls the hide/disable helper on both scroll buttons. If yes, it calls the show/enable helper on both. The function does not read `ScrollPosition`, `ScrollRequest`, `ScrollPixelOffset`, or per-direction endpoint conditions.

The endpoint behavior is instead in `SidebarClass::Action`: pressing up at `ScrollPosition == 0` or down at the bottom simply does not enqueue a request.

Evidence:

- `SidebarClass::UpdateScrollButtons @ 0x006A6610`.
- `FUN_004E1450 @ 0x004E1450`: clears gadget `+0x1E` and sets gadget dirty `+0x1C`.
- `FUN_004E1460 @ 0x004E1460`: sets gadget `+0x1E` and sets gadget dirty `+0x1C`.
- `FUN_004E1470 @ 0x004E1470`: returns `gadget+0x1E == 0`.

Implication: do not gray/hide only the up arrow at the top or only the down arrow at the bottom unless a different live path proves that state. The proven function only toggles both arrows based on list capacity.

## Active in Standard YR?

Yes for ordinary in-game player sidebar paths:

- `SidebarClass::Action @ 0x006A7780` executes when `g_IsMapEditor == 0` and calls `StripClass::AI` for all four strips.
- Scroll-button event IDs `0xC9` down and `0xC8` up are the same IDs established by sibling scroll-gadget reports.
- `StripClass::Draw @ 0x006A9540` is called from `SidebarClass::Draw @ 0x006A6C30` before `PowerClass::Draw`.
- Standard stock layout constants initialize row height and scroll step both to `50`.

Conditional / bounded:

- Observer mode has a one-column/request-size variant (`g_PlayerPtr == DAT_00AC1198`), but this report only uses it to avoid misapplying ordinary two-column formulas.
- The extra-row draw branch is binary-live when `Strip+0x3F != 0`; stock constants make normal row animation complete inside one AI pass, so a visible multi-frame smooth scroll is not proven without runtime frame capture or a proven alternate writer for `DAT_00B0B514`.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Scroll input writes a row request after the current AI pass, not an immediate scroll position jump | `0x006A7780`, `0x006A8B30` | mismatch/unchecked: Rust currently passes `scroll_rows` directly into `SidebarView` construction | `src/sidebar/mod.rs`, `src/sidebar/sidebar_view.rs`, app state that owns sidebar scroll | Add native-style per-tab strip scroll state: request rows, current row, direction, is-scrolling, pixel offset | Press scroll down once; request is enqueued and later consumed one row at a time, bounded by native bottom guard | Do not mutate visible item list directly from button click without preserving request/AI ordering |
| Up pre-decrements `ScrollPosition`; down post-increments after offset reaches `< 1` | `0x006A8B30` | missing | future sidebar scroll state; `src/sidebar/sidebar_view.rs` draw indexing | Preserve asymmetric row timing | Unit test `test_sidebar_scroll_ai_up_predecrements_down_postincrements` using native constants and multi-row request | Do not implement symmetric `scroll_position += direction` logic |
| Stock row height and scroll speed are both `50`, causing one row to complete in one AI pass | `0x006A8B30`; layout report for `DAT_00B0B500/DAT_00B0B514` | likely mismatch if future easing is added | future sidebar scroll update loop | Default stock scroll should not ease over multiple rendered frames unless a proven alternate cadence exists | `test_sidebar_scroll_stock_constants_complete_one_row_per_ai_pass` | Do not add acceleration, delay, or tweening for aesthetics |
| Draw supports `visible_rows + 1` only while `IsScrolling != 0`; hit zones remain visible rows only | `0x006A9540`, `0x006A8220`, `0x006AAD00` | partial mismatch: Rust only stores visible `items` and no extra visual row | `src/sidebar/sidebar_view.rs`, `src/app_sidebar_build.rs` | Separate visual rows from hit zones; when scrolling state is active, build one extra visual row with `pixel_offset - row_height` | `test_sidebar_scroll_extra_visual_row_is_not_clickable` | Do not generate a clickable `SidebarItem` for the extra row |
| Dirty invalidation writes `Strip+0x3C`, `DAT_00B0B518`, `DAT_00884B8F`, tactical dirty, and active-strip dirty flags | `0x006A8B30`, `0x004F42F0`, `0x006A9540`, `0x006A6C30` | mismatch: current Rust rebuilds per-frame instance vectors rather than retained sidebar-surface dirty copy | `src/app_render/draw_passes.rs`, `src/app_sidebar_build.rs`, future retained sidebar surface | Model strip dirty separately from sidebar chrome dirty and from hit-test state | `test_sidebar_scroll_dirty_marks_strip_and_triggers_sidebar_copy_without_rebuilding_hit_rects` | Do not treat dirty as "all sidebar layers redraw every frame" |
| Scroll buttons are enabled/disabled as a pair based on list capacity, not per endpoint | `0x006A6610`, `0x004E1450`, `0x004E1460`, `0x004E1470`, `0x006A7780` | unchecked: Rust does not model native scroll-arrow gadgets in the current view | `src/sidebar/mod.rs`, `src/sidebar/sidebar_view.rs`, future scroll gadget state | If native scroll arrows are added, show/hide both when capacity fits/overflows; endpoint clicks should be ignored by action guards | `test_sidebar_scroll_arrows_capacity_toggle_not_endpoint_toggle` | Do not hide only up at top or only down at bottom from this evidence |

## Negative Facts / Do Not Do

- Do not make scroll animation a multi-frame easing/tween by default. Stock `row_height == scroll_step == 50`.
- Do not move `SelectClass` hit rectangles during scroll animation. Native hit rects are static visible slots.
- Do not make the extra visual row clickable.
- Do not treat scroll-button press as direct page teleport. The press adds page-sized request; `StripClass::AI` consumes one requested row per pass.
- Do not implement symmetric up/down scroll position updates. Native up pre-decrements; native down post-increments.
- Do not collapse strip dirty (`+0x3C`) into sidebar chrome dirty (`Sidebar+0x53A7`) or into gadget dirty bytes.
- Do not infer endpoint-specific scroll-arrow enable/disable from `UpdateScrollButtons`; the proven function toggles both arrows based on capacity, while endpoint guards live in `SidebarClass::Action`.

## Remaining Uncertainty

- Runtime frame capture is still needed to prove whether any standard frame ordering exposes `Strip+0x3F` to `StripClass::Draw` between row-start and row-complete. The binary state machine with stock constants completes the row inside one `StripClass::AI` pass.
- Exact primitive-level clipping inside `CC_Draw_Shape`, `DrawText`, and `AlphaBlendRect` was not re-expanded here. `StripClass::Draw` itself does not install a separate cameo-grid clip around the extra row.
- This slot did not trace every caller of `SidebarClass::Action`; it verified the action-local ordering and draw-local ordering. Full game-loop ordering can be spot-checked in a separate runtime/frame-compositor trace if needed.
- Observer sidebar scroll cadence was bounded but not exhaustively specified.

## Stale-Doc Replacement Wording

- `docs/research/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`, section 5.2: replace "The player sees exactly one partial-frame of scroll" with "The draw code supports an extra-row smooth-scroll frame while `Strip+0x3F` is nonzero, but stock `DAT_00B0B514 == DAT_00B0B500 == 50` makes a row reach its terminal condition inside the same `StripClass::AI` pass. A visible partial-frame requires runtime frame-order proof or an alternate live scroll-step value."
- `docs/research/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`, section 5.4: replace "Scroll button becomes disabled via `UpdateScrollButtons` when the strip can't scroll further in the requested direction" with "`UpdateScrollButtons @ 0x006A6610` toggles both scroll buttons based on whether the list exceeds visible capacity; top/bottom endpoint suppression is handled by `SidebarClass::Action` guards that avoid enqueueing a request."
- `docs/research/SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT_GHIDRA_REPORT.md`: keep the existing extra-row draw finding, but add the stock-cadence caveat that `DAT_00B0B514 == DAT_00B0B500` normally clears `IsScrolling` in the same AI pass.

## Status

COMPLETE for Ghidra-verified scroll state, request consumption, dirty invalidation, static hit-zone separation, and stock-constant cadence.

PARTIAL only for runtime visual capture of whether a standard frame ever presents an in-between smooth-scroll row; that requires a frame trace or screenshot harness outside this read-only Ghidra slot.

## Sources

- Ghidra decompile: `SidebarClass::Action @ 0x006A7780`
- Ghidra decompile: `StripClass::AI @ 0x006A8B30`
- Ghidra decompile: `StripClass::Draw @ 0x006A9540`
- Ghidra decompile: `SidebarClass::Draw @ 0x006A6C30`
- Ghidra decompile: `SidebarClass::InitSelectZones @ 0x006A8220`
- Ghidra decompile: `SelectClass::Action @ 0x006AAD00`
- Ghidra decompile: `SidebarClass::GetVisibleSlotCount @ 0x006AC430`
- Ghidra decompile: `SidebarClass::UpdateScrollButtons @ 0x006A6610`
- Ghidra decompile: `FUN_004E1450 @ 0x004E1450`
- Ghidra decompile: `FUN_004E1460 @ 0x004E1460`
- Ghidra decompile: `FUN_004E1470 @ 0x004E1470`
- Ghidra decompile: `FUN_004F42F0 @ 0x004F42F0`
- Ghidra disassembly confirmation: `0x006A7780..0x006A7C2F`
- Ghidra disassembly confirmation: `0x006A8B30..0x006A8C9F`
- Ghidra disassembly confirmation: `0x006A9540..0x006A96AF`
- `docs/research/SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_INIT_LAYOUT_GLOBALS_EXACT_RECHECK_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_DIRTY_RECTS_REDRAW_FLICKER_PIXEL_CADENCE_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`
