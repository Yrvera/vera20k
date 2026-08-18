# Sidebar Init Layout Globals Exact Recheck - Ghidra Report

Target: `SIDEBAR_INIT_LAYOUT_GLOBALS_EXACT_RECHECK`

Date: 2026-05-27

Primary functions:
- `SidebarClass__InitLayoutConstants @ 0x006A5090`
- `SidebarClass__InitSidebarRect @ 0x006A5130`
- `SidebarClass__Init @ 0x006A5310`

Status: COMPLETE

Correction (2026-07-25): the original audit verified the `Scenario+0x34B8`
consumers but did not trace that field's writer and therefore inherited a wrong
"theater index" label. Fresh writer tracing establishes that the field holds the
selected local `HouseTypeClass+0xBC` side index: Allied `0`, Soviet `1`, Yuri
`2`. Evidence: `Read_Scenario`, `0x0068479D..0x006847C9`; `Full_Init`,
`0x00687794..0x00687833`.

## Target Question

What exact layout globals/formulas does standard YR use for the in-game sidebar X/width, repair/sell, tabs, cameo strip origin, visible row height, scroll button anchors, and strip initialization, and which branches drive them for a Soviet sidebar?

## Non-Goals

- Do not re-check sidebar SHP filename selection, side MIX resolver order, or palettes.
- Do not trace `SidebarClass::Draw` composition order.
- Do not trace radar `SSCR*` placement or minimap movie placement.
- Do not derive final retail SHP frame dimensions or archive membership.
- Do not patch Rust or existing research docs.

## Evidence Needed To Mark Complete

- Fresh read-only Ghidra MCP decompile of `0x006A5090`, `0x006A5130`, and `0x006A5310`.
- Assembly context for branch predicates and handoff-critical coordinate/global writes.
- Active-in-YR evidence through `SidebarClass__Init @ 0x006A5310`.
- Rust-facing handoff with concrete acceptance test names.

## Stop Conditions

- Stop if Ghidra MCP read-only access is unavailable.
- Stop if any target requires creating missing Ghidra functions or comments/labels.
- Stop if exact draw order or radar placement becomes necessary; those are separate slots.

## Verified Findings

### 1. Active call path and init sequence

Active in YR: Yes.

`SidebarClass__Init @ 0x006A5310` calls inherited init, then `SidebarClass__InitLayoutConstants`, then `SidebarClass__InitSidebarRect(0)` before gadget positioning. Later it calls vtable slot `+0x88`, then `SidebarClass__InitSidebarRect(1)`.

Evidence:
- Decompile `0x006A5310`: `FUN_00653010(); SidebarClass__InitLayoutConstants(); SidebarClass__InitSidebarRect(0); ... (**(code **)(*(int *)this + 0x88))(); SidebarClass__InitSidebarRect(1);`
- Assembly context `0x006A5315..0x006A5326`: `CALL 0x00653010`, `CALL 0x006a5090`, `PUSH EBX`, `CALL 0x006a5130`.
- Assembly context `0x006A553D..0x006A554B`: `CALL dword ptr [EAX + 0x88]`, `PUSH 0x1`, `CALL 0x006a5130`.

### 2. `InitLayoutConstants` writes six Y/spacing globals from local side index

Active in YR: Yes.

`SidebarClass__InitLayoutConstants @ 0x006A5090` reads
`g_ScenarioClass_Instance + 0x34B8`. The branch is local side `0` vs non-zero:
Allied selects the first column; Soviet and Yuri select the second.

Allied/side `0`:
- `DAT_00b0b4e0 = g_SidebarWidth + 8`
- `DAT_00b0b4e4 = 0x40`
- `DAT_00b0b4ec = g_SidebarWidth + 0x27`
- `DAT_00b0b4f0 = 0x1D`
- `DAT_00b0b4f8 = g_SidebarWidth + 0x45`
- `DAT_00b0b4fc = 0x3F`

Soviet or Yuri/side non-zero:
- `DAT_00b0b4e0 = g_SidebarWidth + 7`
- `DAT_00b0b4e4 = 0x34`
- `DAT_00b0b4ec = g_SidebarWidth + 0x27`
- `DAT_00b0b4f0 = 0x20`
- `DAT_00b0b4f8 = g_SidebarWidth + 0x45`
- `DAT_00b0b4fc = 0x40`

Evidence:
- Decompile `0x006A5090`.
- Assembly context `0x006A5090..0x006A509D`: reads `[0x00a8b230]`, then `[EAX + 0x34b8]`, `TEST EAX,EAX`, `JNZ 0x006a50dc`.
- Assembly context `0x006A50A5..0x006A50D6`: writes `0x40`, `0x1d`, `0x3f`, then `g_SidebarWidth+8`, `+0x27`, `+0x45`.
- Assembly context `0x006A50E1..0x006A5127`: writes `g_SidebarWidth+7`, `+0x27`, `+0x45`, `0x34`, `0x20`, `0x40`.

### 3. `InitSidebarRect` distinguishes 158px chrome width from 168px top clip

Active in YR: Yes.

`SidebarClass__InitSidebarRect @ 0x006A5130` sets:
- `g_SidebarWidth = 0x9E` (158)
- normal `g_SidebarX = g_RadarViewportWidth + g_RadarViewportOffsetX`
- `g_SidebarTopClip = g_SIDEBAR_WIDTH_CONST` from `0x007f5bf8` (168)
- `DAT_00886f9c = DAT_007f5bfc + g_RadarViewportHeight - 0x9E + g_RadarViewportOffsetY`

In the `param_1 != 0` branch, the same globals are computed from `FUN_0072AD90`'s returned rect, but `g_SidebarWidth` remains `0x9E`.

Evidence:
- Decompile `0x006A5130`.
- Assembly context `0x006A5130..0x006A5193`: normal branch tests `param_1`, checks `DAT_00a8eb7c`, writes `0x00886f94 = 0x9e`, `0x00886f90`, `0x00886f98`, `0x00886f9c`.
- Assembly context `0x006A5195..0x006A51DC`: rect branch calls `0x0072ad90`, writes `0x00886f90`, `0x00886f98`, `0x00886f94 = 0x9e`, `0x00886f9c`.

### 4. Cameo strip origin, row height, total height, and scroll anchors are formula-driven

Active in YR: Yes.

`InitSidebarRect` always sets:
- `DAT_00b0b500 = 0x32` (50px cameo row height)
- overhead constant `iVar3 = 0x1A` when the local side index is `0`, otherwise `0x12`
- `DAT_00b0b504 = floor(((DAT_00886f9c - DAT_00b0b4f8 - iVar3 - 7 + g_SidebarWidth) / 50)) * 50`
- `DAT_00b0b50c = DAT_00b0b4f8 + 7 + DAT_00b0b504`
- `DAT_00b0b514 = 0x32`
- `DAT_00b0b4f4 = g_SidebarX + 0x16`
- `DAT_00b0b508 = g_SidebarX + 0x27`

`DAT_00b0b510` is `0x2E` for local side `0`, `0x2D` otherwise.

Evidence:
- Decompile `0x006A5130`.
- Assembly context `0x006A51E9..0x006A5243`: reads the local side index, writes `DAT_00b0b500 = 0x32`, chooses `0x1A`/`0x12`, computes the division by 50 through multiply by `0x51eb851f`, then writes `DAT_00b0b504`.
- Assembly context `0x006A5248..0x006A5287`: writes scroll Y, `DAT_00b0b510`, and `DAT_00b0b514`.
- Assembly context `0x006A5284..0x006A5305`: writes `DAT_00b0b4f4`, `DAT_00b0b508`, `DAT_00b0b4dc`, `DAT_00b0b4e8`.

### 5. Repair/sell, tab, and strip positioning use the globals directly

Active in YR: Yes, inside the `g_IsMapEditor == 0` branch of `SidebarClass__Init`.

Repair:
- X = `DAT_00b0b4dc`
- Y = `DAT_00b0b4e0`
- ID = `0x65`

Sell:
- X = repair X + `DAT_00b0b4e4`
- Y = repair Y
- ID = `0x66`

Tabs:
- X = `DAT_00b0b4e8 + DAT_00b0b4f0 * index`
- Y = `DAT_00b0b4ec`
- IDs `0xCB..0xCE`
- loop stride is `0x60`, four iterations.

Strips:
- four strips at `this + 0x1564`, stride `0xF94`
- X = `DAT_00b0b4f4`
- Y = `DAT_00b0b4f8`
- field `+0x0C` from strip data base = `0x3C`
- field `+0x10` from strip data base = `DAT_00b0b504`
- calls `SidebarClass__InitSelectZones` once per strip.

Evidence:
- Decompile `0x006A5310`.
- Assembly context `0x006A5338..0x006A5371`: repair writes.
- Assembly context `0x006A53A7..0x006A53BF`: sell X/Y writes.
- Assembly context `0x006A5413..0x006A5484`: tab loop.
- Assembly context `0x006A54F1..0x006A553B`: four strip init loop and calls to `0x006A8220`.

## Implementation Handoff

1. Verified behavior: binary sidebar layout uses `g_SidebarWidth = 158`, `g_SidebarTopClip = 168`, and `g_SidebarX = viewport_right_edge`, with separate chrome and top-clip concepts. Rust delta: `src/sidebar/mod.rs` and `src/sidebar/layout_spec.rs` currently expose a single `SIDEBAR_WIDTH = 168.0` / `sidebar_width = 168.0` as the panel/chrome width. Affected surface: sidebar panel placement, camera edge exclusion, minimap rects, chrome draw origin. Acceptance scenario: at 800x600, native layout globals use `g_SidebarX = 642`, not `632`, while top clip remains 168. Proposed test: `test_sidebar_init_rect_distinguishes_158_width_from_168_top_clip`. Risk: high screenshot/input drift across the whole right sidebar.

2. Verified behavior: repair/sell/tab positions are local-side-driven. Rust delta: `src/sidebar/layout_spec.rs` has fixed `repair_x=8`, `repair_y=20`, `sell_x=96`, `sell_y=20`, and `src/sidebar/sidebar_view.rs` centers tabs from atlas width with manual nudges instead of `DAT_00b0b4e8 + DAT_00b0b4f0*i`. Affected surface: Allied versus Soviet/Yuri sidebar buttons and tabs on every theater. Acceptance scenario: Soviet/Yuri repair/sell are `SidebarX+33,165` and `SidebarX+85,165`; Allied repair/sell are `SidebarX+20,166` and `SidebarX+84,166`; tab X uses `+20 + 32*i` or `+26 + 29*i`. Proposed test: `test_sidebar_gadget_positions_follow_local_side_layout_globals`. Risk: high button and tab pixel drift.

3. Verified behavior: visible cameo height is `DAT_00b0b504`, rounded down to a 50px multiple from `DAT_00886f9c`, cameo Y, overhead, 7px scroll gap, and the 158px width constant; strips always receive X=`SidebarX+22`, Y=`g_SidebarWidth+0x45`, height=`DAT_00b0b504`. Rust delta: `src/sidebar/mod.rs::compute_layout_with_spec` derives rows from item count and reserves a custom bottom control block, then `src/sidebar/sidebar_view.rs` positions cameos via configurable insets/gaps. Affected surface: visible row count, SIDE2 tiling, scroll button Y, and cameo hit rects. Acceptance scenario: the sidebar shows the native number of 50px rows for a given screen height regardless of current item count, and scroll Y is `cameo_y + 7 + visible_height`. Proposed test: `test_sidebar_visible_cameo_rows_use_native_height_formula_not_item_count`. Risk: high row-count and scroll-layout drift.

## Negative Facts / Do Not Do

- Do not treat `0x007f5bf8 = 168` as `g_SidebarWidth`; `InitSidebarRect` writes actual `g_SidebarWidth = 0x9E` at `0x006A515B` / `0x006A51C6`, while `0x007f5bf8` feeds `g_SidebarTopClip`.
- Do key these layout branches on the live local Soviet/Allied/Yuri side;
  `0x006A5090` and `0x006A5130` read `g_ScenarioClass_Instance + 0x34B8`,
  whose writers copy `HouseTypeClass+0xBC`. Do not key them on map theater.
- Do not center tab buttons or apply manual per-tab nudges for native parity; `0x006A541C..0x006A5446` writes `tab.X = base + spacing * index`.
- Do not make the cameo row region depend on current build item count; `0x006A5213..0x006A5243` computes `DAT_00b0b504` from screen/sidebar globals before any strip entries are considered.
- Do not put scroll button X/Y in `SidebarClass__Init`; `0x006A5486..0x006A54EC` initializes scroll button IDs/SHP state but does not write X/Y. Scroll X/Y globals are set by `InitSidebarRect`.

## Remaining Uncertainty

- Exact `SidebarClass__InitSelectZones @ 0x006A8220` select-zone per-cell writes were not rechecked in this slot; they belong to the separate cameo-grid/select-zone slot.
- Exact `SIDE1/SIDE2/SIDE3` draw coordinates and composition order were not traced here; they belong to the draw-composition slot.
- Absolute runtime `DAT_00886f9c` depends on live viewport globals; formulas are verified, but this slot did not debugger-read runtime values at each resolution.
- The former semantic uncertainty for `g_ScenarioClass_Instance + 0x34B8` is resolved:
  it is the selected local side index. The original slot did not check the writer;
  the 2026-07-25 correction did.

## Stale-Doc Wording Suggested

- `docs/research/INIT_LAYOUT_CONSTANTS_GHIDRA_REPORT.md`: replace branch labels
  `RA2` / `YR` with `Allied side 0` / `Soviet-or-Yuri side non-zero`. Keep the
  numeric table and identify `Scenario+0x34B8` as the selected local side index.
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`: replace `At 800x600: SidebarX = 800 - 158 = 642` with `At 800x600 when the tactical viewport width is 642 and offset X is 0, SidebarX = viewport_width + viewport_offset_x = 642; do not compute it from full screen width unless the viewport has already excluded the 158px sidebar.`
- `src/sidebar/mod.rs` comments: replace `Original RA2 sidebar chrome width (all SHPs are 168px wide)` with wording that separates native layout width `158`, top/radar clip `168`, and individual SHP canvas widths.

## Rust Scan

- `SIDEBAR_WIDTH`, `compute_layout_with_spec`, native width/top clip split -> `src/sidebar/mod.rs`, `src/sidebar/layout_spec.rs`, `src/app_camera.rs`, `src/app_cursor.rs` -> existing tests in `src/sidebar/mod.rs` assert `layout.sidebar_x = screen_w - SIDEBAR_WIDTH` -> likely ownership: `sidebar/` layout model plus app cursor/camera consumers.
- `repair_x`, `repair_y`, `sell_x`, `sell_y`, tab positioning -> `src/sidebar/layout_spec.rs`, `src/sidebar/sidebar_view.rs` -> existing hit-test tests route repair/sell but do not assert native coordinates -> likely ownership: `sidebar_view`.
- `visible_rows`, `side2_tile_count`, `scroll_rows` -> `src/sidebar/mod.rs`, `src/sidebar/sidebar_view.rs`, `src/app_building_anim.rs` -> existing stock layout test clamps to 4 rows from item count -> likely ownership: `sidebar` layout and render consumers.
