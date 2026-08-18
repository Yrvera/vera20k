# Sidebar Cameo Grid Select Zones Scroll Layout - Ghidra Report

Date: 2026-05-27
Swarm slot: 4
Target: `SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT`

## Target Question

What exact standard-YR layout formulas control sidebar build-cameo visible slot count, visible hit-zone placement, scroll-button placement, scroll request amount, and smooth-scroll draw offset?

## Non-goals

- Do not re-investigate `CompareItems`, cameo insertion order, or factory production semantics except where a field affects visible layout.
- Do not re-investigate cameo palette selection, SHP load paths, radar layout, or power/credits text layout.
- Do not edit Rust, INI files, or existing published research docs in this slot.

## Evidence Needed To Mark COMPLETE

- Read-only Ghidra MCP decompile for `SidebarClass__InitSelectZones @ 0x006A8220`, `SidebarClass__GetVisibleSlotCount @ 0x006AC430`, `StripClass__Draw @ 0x006A9540`, `StripClass__AI @ 0x006A8B30`, `SidebarClass__Action @ 0x006A7780`, and `SidebarClass__InitSurface @ 0x006ABD30`.
- Ghidra MCP disassembly-range confirmation for coordinate/bounds-critical functions:
  - `0x006A8220..0x006A835F`
  - `0x006AC430..0x006AC48F`
  - `0x006A95C8..0x006A96A3`
  - `0x006A8B30..0x006A8C97`
  - `0x006A7780..0x006A7C2F`
  - `0x006ABD30..0x006ABF37`
- Active-in-standard-YR evidence from normal sidebar init/action/draw paths, not shell/menu-only code.

## Stop Conditions

- If Ghidra MCP read-only tools are unavailable, return FAILED and do not substitute raw binary disassembly or prior docs.
- If a coordinate or inclusive/exclusive bound is visible only in decompiler prose and cannot be paired with an MCP disassembly range, leave it as uncertainty.
- If the path is observer-only, map-editor-only, or shell-only, label it separately and do not fold it into normal player sidebar layout.

## Verified Findings

### 1. Visible slot count is rows times two, not row count

Active in standard YR: Yes. `SidebarClass__GetVisibleSlotCount @ 0x006AC430` computes:

```c
rows = (((DAT_00886f9c - DAT_00b0b4f8) - margin - 7 + g_SidebarWidth) / 0x32);
return rows * 2;
```

`margin` is `0x1A` when `ScenarioClass+0x34B8 == 0`, else `0x12`. `0x32` is the row height, and `* 2` is the two-column slot count. Evidence: decompile `0x006AC430`; disassembly range `0x006AC430..0x006AC48F`.

This same row formula appears in `SidebarClass__InitSelectZones`, `SidebarClass__SwitchTab`, `SidebarClass__Action`, `StripClass__AI`, `StripClass__Draw`, and `SidebarClass__UpdateScrollButtons`.

### 2. InitSelectZones creates static visible-grid click rectangles

Active in standard YR: Yes. `SidebarClass__InitSelectZones @ 0x006A8220` sets one tab's select-gadget metadata for the currently visible rows:

```c
for row in 0..rows:
  for col in 0..2:
    idx = tabIndex * 0x3C + row * 2 + col;
    Select[idx].ID = 0xCA;
    Select[idx].X = Strip.XPos + DAT_00b0b4fc * col;
    Select[idx].Y = Strip.YPos + 1 + DAT_00b0b500 * row;
    Select[idx].W = 0x3C;
    Select[idx].H = 0x30;
    Select[idx].StripPtr = strip;
    Select[idx].CameoIndex = row * 2 + col;
```

Field offsets are `X +0x0C`, `Y +0x10`, `W +0x14`, `H +0x18`, `ID +0x24`, `StripPtr +0x2C`, and visible-grid `CameoIndex +0x30` on `0x38`-byte `SelectClass` entries. The rectangles are static for visible rows; scroll changes the cameo index under the same rectangle, not the rectangle itself. Evidence: decompile `0x006A8220`; disassembly range `0x006A8220..0x006A835F`.

### 3. Click mapping adds row scroll position to the static slot index

Active in standard YR: Yes. `SelectClass__Action @ 0x006AAD00` reads the owning strip pointer at select `+0x2C`, then maps the click to:

```c
visibleIndex = Select.CameoIndex + Strip.ScrollPosition * 2;
entry = Strip.Cameos[visibleIndex];
```

The bounds check is `visibleIndex < Strip.CameoCount` before acting. This proves hit zones remain tied to visible row/column while scroll state chooses which persistent cameo entry the click targets. Evidence: decompile `0x006AAD00`; disassembly range `0x006AAD00..0x006AAD8B`.

### 4. Normal draw iterates one extra row while smooth scrolling

Active in standard YR: Yes for normal player strips. `StripClass__Draw @ 0x006A9540` computes:

```c
drawRows = rows + (Strip.IsScrolling != 0);
entryIndex = col + (drawRow + Strip.ScrollPosition) * 2;
x = Strip.XPos - g_SidebarX + DAT_00b0b4fc * col;
y = Strip.YPos + 1 + DAT_00b0b500 * drawRow;
if (Strip.IsScrolling) {
  y += Strip.ScrollPixelOffset - DAT_00b0b500;
}
```

The draw loop still uses two columns. The extra row is draw-only; `SwitchTab` and select-zone activation still use `rows * 2`, not `(rows + scrolling) * 2`. Evidence: decompile `0x006A9540`; coordinate/bounds disassembly range `0x006A95C8..0x006A96A3`.

### 5. Scroll requests are page-sized, but the animation moves one row per AI tick

Active in standard YR: Yes. `SidebarClass__Action @ 0x006A7780` changes the active strip's `ScrollRequest` by a page:

- Scroll-down button event `DAT_00b0b34c | 0x8000`: if `(ScrollPosition + rows) * 2 < CameoCount`, add `rows` to `ScrollRequest`.
- Scroll-up button event `DAT_00b0b42c | 0x8000`: if `ScrollPosition != 0`, subtract `rows` from `ScrollRequest`.
- Observer branch uses `1` as request amount; normal player branch uses `rows`.

`StripClass__AI @ 0x006A8B30` consumes one requested row at a time. Up requests pre-decrement `ScrollPosition`, set `ScrollDirection = 0`, `ScrollPixelOffset = 0`, then animate `ScrollPixelOffset += DAT_00b0b514` until it reaches row height. Down requests set `ScrollDirection = 1`, `ScrollPixelOffset = DAT_00b0b500`, then animate downward by subtracting `DAT_00b0b514`; when `< 1`, they post-increment `ScrollPosition`.

With stock `DAT_00b0b500 = DAT_00b0b514 = 50`, each requested row completes in one `StripClass__AI` pass, but the mechanism is still request-per-row, not direct page teleport. Evidence: decompile `0x006A7780` and `0x006A8B30`; disassembly ranges `0x006A7780..0x006A7C2F` and `0x006A8B30..0x006A8C97`.

### 6. Scroll button positions are set in InitSurface, not in SidebarClass::Init

Active in standard YR: Yes. `SidebarClass__InitSurface @ 0x006ABD30` positions the scroll gadgets after tab gadgets:

```c
ScrollDownGadget.X = DAT_00b0b508;
ScrollDownGadget.Y = DAT_00b0b50c;
ScrollUpGadget.X = DAT_00b0b508 + DAT_00b0b510;
ScrollUpGadget.Y = DAT_00b0b50c;
```

`DAT_00b0b508 = g_SidebarX + 0x27`; `DAT_00b0b50c = DAT_00b0b4f8 + 7 + DAT_00b0b504`; `DAT_00b0b510` is `0x2E` for `ScenarioClass+0x34B8 == 0`, else `0x2D`. The global object at `0x00B0B328` has ID `0xC9` and action branch scrolls down; the object at `0x00B0B408` has ID `0xC8` and action branch scrolls up. Evidence: decompile `0x006ABD30`, `0x006A5310`, `0x006A5130`, and `0x006A7780`; disassembly range `0x006ABD30..0x006ABF37`.

## Implementation Handoff

1. Native visible-slot formula and select-zone activation -> Rust should derive visible slots as `rows * 2`, with row count from the native formula, not from rendered `side2` tile count alone -> `src/sidebar/sidebar_view.rs`, `src/sidebar/mod.rs` -> `test_sidebar_native_visible_slot_count_is_rows_times_two`.

2. Static hit zones plus scroll-position index mapping -> Rust click handling should map `(visible_row, col)` to `scroll_rows * 2 + row * 2 + col` and should not move hit boxes during smooth scroll -> `src/sidebar/sidebar_view.rs` -> `test_sidebar_cameo_click_uses_visible_slot_plus_scroll_position`.

3. Smooth-scroll draw offset and extra draw row -> Rust render model should support drawing `rows + 1` rows during active scroll using `pixel_offset - row_height`, while active/selectable zones remain `rows * 2` -> `src/sidebar/sidebar_view.rs`, `src/render/sidebar_chrome.rs` -> `test_sidebar_smooth_scroll_draws_one_extra_row_without_extra_hit_zone`.

## Negative Facts / Do Not Do

- Do not treat `GetVisibleSlotCount` as returning rows; it returns `rows * 2`.
- Do not add `IsScrolling` to the number of active click gadgets; the `+1` row exists in `StripClass::Draw`, not in `SwitchTab`/select-zone activation.
- Do not move SelectClass hit rectangles during scroll animation; only the drawn cameo y coordinate is offset.
- Do not jump a page of rows instantly on scroll button press; the action enqueues page-sized row requests, and `StripClass::AI` consumes them one row at a time.
- Do not position scroll buttons in `SidebarClass::Init`; `Init` assigns IDs/SHP state, while `InitSurface` sets the current x/y positions.

## Remaining Uncertainty

- Ghidra MCP `disassemble_bytes` confirmed address ranges but returned range summaries rather than mnemonic listings in this session; the report therefore cites decompile text plus MCP disassembly-range confirmation, not pasted instruction mnemonics.
- Exact semantic name of `ScenarioClass+0x34B8` remains inherited from sibling sidebar docs; this slot treats only its binary branch effect on layout constants.
- Observer/spectator sidebar layout has special one-column/house-list paths in `StripClass::Draw`, `StripClass::AI`, and `UpdateScrollButtons`; this report records them only where they affected normal-branch comparison.

## Stale-Doc Wording

- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md` section 11 says scroll down is positioned at `ScrollX + ScrollWidth` immediately right of scroll up. Fresh `InitSurface @ 0x006ABD30` evidence shows the `0xC9` scroll-down gadget is positioned at `DAT_00b0b508`, while the `0xC8` scroll-up gadget is positioned at `DAT_00b0b508 + DAT_00b0b510`.
- `docs/research/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md` still mixes older wording around visible-count naming in places; any future patch should consistently call `0x006AC430` a visible slot count (`rows * 2`) and separately name `rows`.

## Status

COMPLETE
