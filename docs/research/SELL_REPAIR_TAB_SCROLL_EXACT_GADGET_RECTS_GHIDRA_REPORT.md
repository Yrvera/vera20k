# Sell/Repair/Tab/Scroll Gadget Rects - Ghidra Research Report

**Address(es):** `0x006A5310`, `0x006A5130`, `0x006A5090`, `0x006ABD30`, `0x0069DE00`, `0x004E15A0`, `0x004E13F0`, `0x006A7780`
**Investigation Mode:** exhaustive-slice, downgraded to partial for numeric SHP-derived W/H
**Claimed Scope:** ordinary in-game player sidebar sell, repair, four build tabs, and scroll-arrow gadget IDs, x/y formulas, draw origins, hit-test rect mechanism, and asset-to-width/height source.
**Non-Scope:** cameo select-zone rects except scroll-arrow contrast, observer sidebar, tactical minimap/radar, retail SHP frame dimensions/offset dump, and Rust implementation.
**Confidence:** High for binary formulas, IDs, event dispatch, and hit-test mechanism; Medium for final numeric W/H until the sibling retail-SHP-dimensions slot supplies asset headers.
**Active in YR:** Yes for ordinary player sidebar paths inside the `g_IsMapEditor == 0` branch. Evidence: `SidebarClass__Init @ 0x006A5310`, `SidebarClass__InitSurface @ 0x006ABD30`, `SidebarClass__AI @ 0x006A7780` (former `Action` label corrected).

## 0. Working Notes

- Target question: Prove exact native x/y/w/h, IDs, and hit/draw rect origins for sell, repair, the four build tabs, and sidebar scroll arrows in ordinary player sidebar.
- Non-goals: Do not re-investigate cameo hit zones, radar, observer paths, or retail asset dimensions beyond proving the binary reads SHP header width/height.
- Evidence needed to mark COMPLETE: decompile plus assembly context for coordinate writers, IDs, SHP width/height source, draw/hit rect consumption, and active YR action dispatch.
- Stop conditions: Stop without mutating Ghidra; stop before asset-dump work owned by `RETAIL_SOVIET_SIDEBAR_SHP_DIMENSIONS_OFFSETS`; mark partial if concrete numeric W/H cannot be proven from binary alone.

## 1. Overview

The ordinary in-game sidebar gadgets are retained `GadgetClass`-style controls. `SidebarClass__Init` assigns their IDs and first positions, `SidebarClass__LoadSHPs` binds their SHP art and writes width/height from the loaded SHP header, `SidebarClass__InitSurface` refreshes live x/y positions after surface/layout setup, and the base `GadgetClass` hit-test uses the same `X/Y/W/H` fields.

The key correction is the scroll-arrow side: object `0x00B0B328` is ID `0xC9` and is positioned at `ScrollX`; object `0x00B0B408` is ID `0xC8` and is positioned at `ScrollX + ScrollWidth`. Older `SCROLL_BUTTON_POSITION_SETTER_GHIDRA_REPORT.md` labels those two object roles backwards.

## 2. Class Layout / Key Offsets

### SBGadgetClass / inherited GadgetClass fields

| Offset | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `+0x0C` | X | `FUN_004E1A20` writes `this+0x0C`; `GadgetClass__Hit_Test` reads `param_1[3]` | Yes |
| `+0x10` | Y | `FUN_004E1A20` writes `this+0x10`; `GadgetClass__Hit_Test` reads `param_1[4]` | Yes |
| `+0x14` | Width | `FUN_0069DE00` writes SHP header `*(short*)(shape+2)`; hit-test reads `param_1[5]` | Yes |
| `+0x18` | Height | `FUN_0069DE00` writes SHP header `*(short*)(shape+4)`; hit-test reads `param_1[6]` | Yes |
| `+0x1C` | redraw/visible dirty bit | `FUN_004E1960` writes `+0x1C = 1` | Yes |
| `+0x1E` | disabled / skipped by hit-test | `GadgetClass__Hit_Test` skips when nonzero | Yes |
| `+0x24` | gadget command ID | `SidebarClass__Init` writes IDs; action receives `0x8000 | ID` | Yes |
| `+0x44` | draw X offset, normally `-g_SidebarX` after `InitSurface` | `0x006ABE24..0x006ABE37`, `0x006ABE5B..0x006ABE67`, tab loop, scroll blocks | Yes |
| `+0x48` | draw Y offset, initialized `0` for sidebar gadgets | `SidebarClass__Init` writes `0` | Yes |
| `+0x50` | ConvertClass pointer | init/load writes `DAT_0087F6CC` | Yes |
| `+0x58` | SHP pointer | `FUN_0069DE00` writes loaded shape pointer | Yes |

## 3. Core Logic

### 3.1 Layout branch inputs

Active in YR: Yes. `SidebarClass__InitLayoutConstants @ 0x006A5090` and
`SidebarClass__InitSidebarRect @ 0x006A5130` read
`g_ScenarioClass_Instance + 0x34B8`. CORRECTED 2026-07-25: fresh writer
tracing establishes that this is the selected local side index copied from
`HouseTypeClass+0xBC`, not theater. Evidence: `Read_Scenario`,
`0x0068479D..0x006847C9`; `Full_Init`, `0x00687794..0x00687833`.

Important globals:

| Global | Allied side 0 | Soviet/Yuri side nonzero | Evidence |
|---|---:|---:|---|
| `g_SidebarWidth` | `158` | `158` | `0x006A5130` |
| `DAT_00B0B4DC` repair X | `g_SidebarX + 20` | `g_SidebarX + 33` | `0x006A5130` |
| `DAT_00B0B4E0` repair/sell Y | `158 + 8 = 166` | `158 + 7 = 165` | `0x006A5090` |
| `DAT_00B0B4E4` sell X delta | `64` | `52` | `0x006A5090` |
| `DAT_00B0B4E8` tab X base | `g_SidebarX + 26` | `g_SidebarX + 20` | `0x006A5130` |
| `DAT_00B0B4EC` tab Y | `158 + 39 = 197` | `197` | `0x006A5090` |
| `DAT_00B0B4F0` tab X spacing | `29` | `32` | `0x006A5090` |
| `DAT_00B0B508` scroll X | `g_SidebarX + 39` | `g_SidebarX + 39` | `0x006A5130` |
| `DAT_00B0B50C` scroll Y | `DAT_00B0B4F8 + 7 + DAT_00B0B504` | same | `0x006A5130` |
| `DAT_00B0B510` scroll X delta | `46` | `45` | `0x006A5130` |

Assembly evidence: `0x006A5338..0x006A5371` repair writes, `0x006A53A7..0x006A53BF` sell writes, `0x006A5413..0x006A5484` tab loop, `0x006ABEAA..0x006ABF0F` scroll position writers.

### 3.2 Final gadget rect formulas

Active in YR: Yes for ordinary player sidebar. Evidence: the coordinate writes are reached through `SidebarClass__Init @ 0x006A5310` when `g_IsMapEditor == 0`, and refreshed through `SidebarClass__InitSurface @ 0x006ABD30`.

| Gadget | Object | ID | X | Y | W/H source |
|---|---:|---:|---|---|---|
| Repair | `0x00B0B3A0` | `0x65` | Allied: `g_SidebarX + 20`; Soviet/Yuri: `g_SidebarX + 33` | Allied: `166`; Soviet/Yuri: `165` | `REPAIR.SHP` header `+2/+4` via `0x0069DE00` |
| Sell | `0x00B07DF8` | `0x66` | Allied: `g_SidebarX + 84`; Soviet/Yuri: `g_SidebarX + 85` | same as repair | `SELL.SHP` header `+2/+4` via `0x0069DE00` |
| Tab 0 | `0x00B07C48` | `0xCB` | Allied: `g_SidebarX + 26`; Soviet/Yuri: `g_SidebarX + 20` | `197` | `TAB00.SHP` header `+2/+4` via `0x0069DE00` |
| Tab 1 | `0x00B07CA8` | `0xCC` | Allied: `g_SidebarX + 55`; Soviet/Yuri: `g_SidebarX + 52` | `197` | `TAB01.SHP` header `+2/+4` |
| Tab 2 | `0x00B07D08` | `0xCD` | Allied: `g_SidebarX + 84`; Soviet/Yuri: `g_SidebarX + 84` | `197` | `TAB02.SHP` header `+2/+4` |
| Tab 3 | `0x00B07D68` | `0xCE` | Allied: `g_SidebarX + 113`; Soviet/Yuri: `g_SidebarX + 116` | `197` | `TAB03.SHP` header `+2/+4` |
| Scroll down | `0x00B0B328` | `0xC9` | `g_SidebarX + 39` | `DAT_00B0B4F8 + 7 + DAT_00B0B504` | `R-DN.SHP` header `+2/+4` |
| Scroll up | `0x00B0B408` | `0xC8` | Allied: `g_SidebarX + 85`; Soviet/Yuri: `g_SidebarX + 84` | same as scroll down | `R-UP.SHP` header `+2/+4` |

The table's tab objects use stride `0x60`. Object bases: `0x00B07C48 + index * 0x60`.

### 3.3 Width and height are SHP-header-driven, not layout constants

Active in YR: Yes. `FUN_0069DE00 @ 0x0069DE00` is the SBGadget shape binder. It stores the SHP pointer at `gadget+0x58`, then:

- if shape pointer is null: `gadget+0x14 = 0`, `gadget+0x18 = 0`
- otherwise: `gadget+0x14 = (int16)shape[+2]`, `gadget+0x18 = (int16)shape[+4]`
- optional nonzero override args can replace width/height, but `SidebarClass__LoadSHPs @ 0x006A5840` passes `0,0` for sell, repair, tabs, and scroll arrows.

Strong evidence:

- Decompile `0x0069DE00`.
- Assembly `0x0069DE32..0x0069DE3D`: `MOVSX ECX, word ptr [EAX+0x2] -> [ESI+0x14]`, `MOVSX EDX, word ptr [EAX+0x4] -> [ESI+0x18]`.
- Load binding contexts:
  - `0x006A58CF..0x006A58E8`: `"SELL.SHP"` string `0x0083FA4C`, object `0x00B07DF8`, call `0x0069DE00`.
  - `0x006A5907..0x006A591C`: `"REPAIR.SHP"` string `0x0083FA40`, object `0x00B0B3A0`, call `0x0069DE00`.
  - `0x006A5943..0x006A5986`: format string `"TAB%02d.SHP"` at `0x0083FA34`, tab objects, call `0x0069DE00`.
  - `0x006A5994..0x006A59A4`: `"R-DN.SHP"` string `0x0083FA28`, object `0x00B0B328`, call `0x0069DE00`.
  - `0x006A59CA..0x006A59DA`: `"R-UP.SHP"` string `0x0083FA1C`, object `0x00B0B408`, call `0x0069DE00`.
- String identity was spot-checked by read-only byte searches for each ASCII filename.

### 3.4 Hit-test rect and edge rule

Active in YR: Yes for these sidebar gadgets because they derive from the active GadgetClass framework and feed command IDs into `SidebarClass__AI`.

`GadgetClass__Hit_Test @ 0x004E15A0` walks the linked gadget list and ignores disabled gadgets (`+0x1E != 0`). For enabled gadgets it tests:

```text
0x0C <= mouse_x < +0x0C + +0x14
+0x10 <= mouse_y < +0x10 + +0x18
```

It then selects the smallest-area containing gadget; if areas tie, the later list item replaces the prior winner because the comparison accepts `new_area <= saved_area`.

`GadgetClass__Clicked_On @ 0x004E13F0` repeats a half-open-style bounds rejection using unsigned deltas: it rejects when `width <= mouse_x - X` or `height <= mouse_y - Y`. This confirms the right and bottom edges are outside the hit rect.

Evidence:

- Decompile `0x004E15A0`.
- Assembly context `0x004E15C4..0x004E1626` confirms the disabled-byte read and area comparison range.
- Decompile `0x004E13F0`.

### 3.5 Draw origin

Active in YR: Yes. `SBGadgetClass__Draw @ 0x0069DEB0` chooses `g_SidebarSurface` when `gadget+0x4C != 0`; all scoped sidebar gadgets initialize this field as `1`. It passes the gadget SHP pointer, frame, convert class, and point to `CC_Draw_Shape`. The draw point is based on:

```text
draw_x = gadget.X + gadget.+0x44
draw_y = gadget.Y + gadget.+0x48
```

`SidebarClass__InitSurface @ 0x006ABD30` writes `+0x44 = -g_SidebarX` for repair, sell, tabs, scroll down, and scroll up after setting their positions. `+0x48` remains initialized as `0` for these sidebar gadgets. Therefore draw-space x is sidebar-surface-local, while hit/input x remains screen-space.

Evidence:

- Decompile `0x0069DEB0`: surface choice and point setup.
- Assembly `0x006ABE24..0x006ABE37` repair `+0x44`, `0x006ABE5B..0x006ABE67` sell `+0x44`, tab loop `0x006ABE94..0x006ABE9E`, scroll blocks `0x006ABEDF` and `0x006ABF0F`.

### 3.6 Action IDs and active dispatch

Active in YR: Yes. `SidebarClass__AI @ 0x006A7780` receives command IDs with `0x8000` set:

- `0x8066` dispatches sell mode via `FUN_004AC660(-1)`.
- `0x8065` dispatches repair mode via `FUN_004AC8C0(-1)`.
- `0x80CB..0x80CE` switch tabs.
- `DAT_00B0B34C | 0x8000` handles scroll-down requests; since `DAT_00B0B34C = 0xC9`, ID `0xC9` is scroll down.
- `DAT_00B0B42C | 0x8000` handles scroll-up requests; since `DAT_00B0B42C = 0xC8`, ID `0xC8` is scroll up.

Evidence:

- Decompile `0x006A7780`.
- Assembly context `0x006A78C8` sell branch and `0x006A78D2..0x006A78FA` repair branch.
- `SidebarClass__Init @ 0x006A5486..0x006A54EC` assigns scroll IDs.

## 4. INI Keys

No INI key controls these gadget rectangles. Inputs are selected local side
index, viewport/sidebar globals, and loaded SHP headers. Active in YR: Yes;
evidence from binary readers and writer trace above. Retail asset dimensions are
intentionally deferred to the sibling asset-dimension slot.

## 5. Integration Points

| Function | Role | Active in YR |
|---|---|---|
| `0x006A5310` `SidebarClass__Init` | initial IDs, state, repair/sell/tab x/y, scroll IDs | Yes inside `g_IsMapEditor == 0` |
| `0x006A5840` `SidebarClass__LoadSHPs` | binds SHPs and writes W/H through `0x0069DE00` | Yes |
| `0x006ABD30` `SidebarClass__InitSurface` | refreshes x/y and draw offsets, registers tooltip rects | Yes on init/load/resize |
| `0x0069DEB0` `SBGadgetClass__Draw` | draws scoped gadgets to `g_SidebarSurface` | Yes through `SidebarClass__Draw` |
| `0x004E15A0` / `0x004E13F0` | hit-test and clicked-on bounds | Yes through GadgetClass framework |
| `0x006A7780` `SidebarClass__AI` | consumes command IDs | Yes |

## 6. Current Rust Implementation Status

- `src/sidebar/mod.rs:22` uses one `SIDEBAR_WIDTH = 168.0`, while native coordinate formulas use `g_SidebarWidth = 158` and separate 168 top/sidebar surface concepts.
- `src/sidebar/sidebar_view.rs:117..134` centers tabs using atlas widths and manual nudges; native uses `tab_base + tab_spacing * index`.
- `src/sidebar/layout_spec.rs:82..85` has fixed repair/sell offsets `8/20/96/20`, not side-driven `+20/+33`, `+84/+85`, `165/166`.
- `src/sidebar/mod.rs:379..399` hit-tests tabs, repair, sell, and items in bespoke order, not the base GadgetClass smallest-area/last-on-tie walk. It does use half-open `Rect::contains` if that implementation is standard, but ordering/disabled semantics need native parity checks.
- `src/render/sidebar_chrome.rs:314..325` loads five repair/sell frames; this aligns with the frame count implied by `SBGadgetClass__Draw`, but numeric rect W/H still depends on decoded SHP frame/header behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| repair/sell IDs and x/y | verified | `0x006A5310`, `0x006A5130`, `0x006A5090` | none |
| tab IDs and x/y | verified | `0x006A5310`, `0x006A5130`, `0x006A5090` | none |
| scroll IDs and x/y | verified | `0x006A5310`, `0x006ABD30`, `0x006A5130` | none |
| width/height source | verified | `0x0069DE00`, `0x006A5840` | concrete numeric W/H requires retail SHP header dump |
| draw origin and surface | verified | `0x0069DEB0`, `0x006ABD30` | deeper `CC_Draw_Shape` internal point handling out of scope |
| hit-test edge rule | verified | `0x004E15A0`, `0x004E13F0` | exact gadget list linkage order not re-derived here |
| action IDs | verified | `0x006A7780` | none |
| observer/sidebar special branch | deferred | out-of-scope | separate observer pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which object is repair? -> 0x00B0B3A0, ID 0x65.` (evidence: `0x006A5338..0x006A5371`)
- `[RESOLVED] OQ-02 - Which object is sell? -> 0x00B07DF8, ID 0x66.` (evidence: `0x006A53A7..0x006A540C`)
- `[RESOLVED] OQ-03 - Which object is scroll down? -> 0x00B0B328, ID 0xC9, positioned at ScrollX.` (evidence: `0x006A5486..0x006A54BA`, `0x006ABEB0..0x006ABEC7`)
- `[RESOLVED] OQ-04 - Which object is scroll up? -> 0x00B0B408, ID 0xC8, positioned at ScrollX + ScrollWidth.` (evidence: `0x006A54BF..0x006A54EC`, `0x006ABED1..0x006ABEFC`)
- `[RESOLVED] OQ-05 - Are tab positions centered or formula-spaced? -> Formula-spaced by `DAT_00B0B4E8 + DAT_00B0B4F0 * index`.` (evidence: `0x006A541C..0x006A5446`)
- `[RESOLVED] OQ-06 - Where do W/H come from? -> `FUN_0069DE00` reads loaded SHP header shorts at `+2/+4`; no nonzero override is passed for scoped gadgets.` (evidence: `0x0069DE00`, `0x006A5840`)
- `[RESOLVED] OQ-07 - What is the hit edge rule? -> half-open left/top inclusive, right/bottom exclusive.` (evidence: `0x004E15A0`, `0x004E13F0`)
- `[RESOLVED] OQ-08 - Is the path active in standard YR? -> Yes, ordinary sidebar init/action/draw paths run when not map editor; no TS-only gate was found in scoped functions.` (evidence: `0x006A5310`, `0x006ABD30`, `0x006A7780`)
- `[DEFERRED] OQ-09 - What are the concrete numeric W/H values for every side-specific retail SHP?` (category: `requires-different-system-context`; reason: sibling slot owns retail SHP dimension/offset dump; next-step-if-pursued: read `REPAIR/SELL/TAB00..03/R-DN/R-UP` SHP headers from active Soviet MIX order)
- `[DEFERRED] OQ-10 - Exact linked-list order among all gadgets after `FUN_0069DFF0`?` (category: `bounded-cost-too-high`; reason: current target only needed rect identities and base hit rule; next-step-if-pursued: trace `FUN_0069DFF0` / LinkClass insertion and GadgetClass input list head)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `SidebarClass__Draw @ 0x006A6C30` | sibling draw-order report | sell gadget draw | `Sell.X/Y`, W/H from `SELL.SHP` | `DAT_0087F6CC` | Yes | control chrome |
| 2 | `SidebarClass__Draw @ 0x006A6C30` | sibling draw-order report | repair gadget draw | `Repair.X/Y`, W/H from `REPAIR.SHP` | `DAT_0087F6CC` | Yes | control chrome |
| 3 | `SidebarClass__Draw @ 0x006A6C30` | tab loop over `0x00B07C48` stride `0x60` | `TAB00..03.SHP` | tab formula rects | `DAT_0087F6CC` | Yes | tab chrome |
| 4 | `SidebarClass__Draw @ 0x006A6C30` | scroll gadget draw after tabs | `R-DN.SHP`, `R-UP.SHP` | scroll formula rects | `DAT_0087F6CC` | Yes | scroll controls |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Chrome/container | Evidence |
|---|---|---|---|---|---|
| `SELL.SHP` | Yes | Yes | Yes | control | `0x006A58CF..0x006A58E8`, `0x0069DEB0` |
| `REPAIR.SHP` | Yes | Yes | Yes | control | `0x006A5907..0x006A591C`, `0x0069DEB0` |
| `TAB00..03.SHP` | Yes | Yes | Yes | tab control | `0x006A5943..0x006A5986` |
| `R-DN.SHP` | Yes | Yes | Conditional on scroll visibility | scroll control | `0x006A5994..0x006A59A4` |
| `R-UP.SHP` | Yes | Yes | Conditional on scroll visibility | scroll control | `0x006A59CA..0x006A59DA` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Repair/sell/tab/scroll x/y use local-side-driven globals, not centered/spec nudges | `0x006A5090`, `0x006A5130`, `0x006A5310`, `0x006ABD30`; writer `0x0068479D..0x006847C9` | mismatch: fixed layout spec and tab centering | `src/sidebar/layout_spec.rs`, `src/sidebar/sidebar_view.rs`, `src/sidebar/mod.rs` | compute native gadget rects from side/layout globals | At 800x600 Soviet/Yuri, repair is `SidebarX+33,165`, sell `SidebarX+85,165`, tab x are `+20,+52,+84,+116`, scroll down `+39`, scroll up `+84`; proposed test `test_sidebar_gadget_rects_use_native_side_globals` | Do not center tabs or use manual nudges |
| Gadget W/H are loaded from SHP header and used for hit-test, not hardcoded layout constants | `0x0069DE00`, `0x004E15A0`, `0x004E13F0` | partial/mismatch risk: Rust uses atlas sizes for repair/sell, tab size optional; scroll buttons are not modeled in this view | `src/render/sidebar_chrome.rs`, `src/sidebar/sidebar_view.rs`, asset metadata structs | expose decoded SHP header dimensions for each gadget rect and hit-test | Changing retail side MIX changes gadget W/H exactly as loaded; proposed test `test_sidebar_gadget_hit_rects_use_loaded_shp_header_dimensions` | Do not assume all side/theater art shares fixed dimensions without asset proof |
| Base GadgetClass hit-test is half-open and skips disabled gadgets, with smallest-area/last-on-tie precedence | `0x004E15A0`, `0x004E13F0` | unchecked/mismatch: Rust bespoke order tabs -> repair -> sell -> items | `src/sidebar/mod.rs::hit_test`, future native gadget list model | route sidebar gadget hit-tests through native rect/disabled/list-order semantics | A click on the right/bottom edge misses; disabled scroll/repair/sell cannot hit; equal-area overlaps resolve to later linked gadget; proposed test `test_sidebar_gadget_hit_test_matches_gadgetclass_half_open_order` | Do not treat visual draw order alone as hit order until `FUN_0069DFF0` list insertion is fully modeled |

## Negative Facts / Do Not Do

- Do not reverse scroll arrows: ID `0xC9` at object `0x00B0B328` is scroll down and sits at `DAT_00B0B508`; ID `0xC8` at object `0x00B0B408` is scroll up and sits at `DAT_00B0B508 + DAT_00B0B510`. Evidence: `0x006A5486..0x006A54EC`, `0x006ABEB0..0x006ABEFC`, `0x006A7780`.
- Do not center tab buttons or apply per-tab X nudges. Native tab X is exactly `DAT_00B0B4E8 + DAT_00B0B4F0 * index`. Evidence: `0x006A541C..0x006A5446`.
- Do key these gadget rects on the live local side. The active branch reads
  scenario field `+0x34B8`, whose writers copy `HouseTypeClass+0xBC`; do not
  key it on theater. Evidence: `0x006A5090`, `0x006A5130`,
  `0x0068479D..0x006847C9`.
- Do not hardcode repair/sell/tab/scroll W/H from current Rust comments as binary constants. Native width/height are loaded from each SHP header by `0x0069DE00`. Evidence: `0x0069DE32..0x0069DE3D`.
- Do not make the right or bottom edge clickable. Base `GadgetClass__Hit_Test` and `Clicked_On` use exclusive upper bounds. Evidence: `0x004E15A0`, `0x004E13F0`.

## Remaining Uncertainty

- Concrete numeric W/H for active Soviet retail `REPAIR.SHP`, `SELL.SHP`, `TAB00..03.SHP`, `R-DN.SHP`, and `R-UP.SHP` are deferred to `RETAIL_SOVIET_SIDEBAR_SHP_DIMENSIONS_OFFSETS`.
- Exact linked-list insertion order from `FUN_0069DFF0` was not exhausted; hit-test precedence is proven at the base GadgetClass level, but final overlap precedence for all sidebar gadgets should be rechecked if any rects overlap after asset dimensions are known.
- The semantic writer of `ScenarioClass+0x34B8` is resolved as the selected
  local side index by `0x0068479D..0x006847C9` and
  `0x00687794..0x00687833`.

## Stale Docs / Follow-up Docs

- `docs/research/SCROLL_BUTTON_POSITION_SETTER_GHIDRA_REPORT.md`: replace "ScrollUp.X (`0xb0b408+0x0C`) <- `DAT_00b0b508`; ScrollDown.X (`0xb0b328+0x0C`) <- `DAT_00b0b510 + DAT_00b0b508`" with "ScrollDown ID `0xC9` at `0x00B0B328` gets `X = DAT_00B0B508`; ScrollUp ID `0xC8` at `0x00B0B408` gets `X = DAT_00B0B508 + DAT_00B0B510`."
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`: replace section 4 labels
  `RA2` / `YR` with `Allied side 0` / `Soviet-or-Yuri side nonzero`, and
  replace `DAT_00b0b4e4 Repair button height` with `Sell X delta`.
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`: in any scroll wording, state `0xC9 = scroll down at ScrollX`, `0xC8 = scroll up at ScrollX + ScrollWidth`.

## Sources

- Ghidra decompile: `0x006A5090`, `0x006A5130`, `0x006A5310`, `0x006A5840`, `0x006ABD30`, `0x0069DE00`, `0x0069DEB0`, `0x004E15A0`, `0x004E13F0`, `0x006A7780`.
- Ghidra assembly context: `0x006A5338..0x006A54EC`, `0x006A58CF..0x006A59DA`, `0x006ABE10..0x006ABF0F`, `0x0069DE32..0x0069DE5B`, `0x004E15C4..0x004E1626`.
- Prior docs referenced: `SIDEBAR_INIT_LAYOUT_GLOBALS_EXACT_RECHECK_GHIDRA_REPORT.md`, `SIDEBAR_CAMEO_GRID_SELECT_ZONES_SCROLL_LAYOUT_GHIDRA_REPORT.md`, `SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md`, `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md`.

**Status:** PARTIAL - binary rect formulas, IDs, action mapping, draw origin, and hit-test mechanism are complete; final numeric W/H awaits the sibling retail SHP dimensions/offsets investigation.
