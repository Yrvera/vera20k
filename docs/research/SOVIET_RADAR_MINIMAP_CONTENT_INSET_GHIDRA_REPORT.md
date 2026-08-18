# Soviet Radar Minimap Content Inset - Ghidra Research Report

**Address(es):** `0x0063FB20`, `0x00653100`, `0x00652CF0`, `0x00652E90`, `0x00654650`, `0x00656EC0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** ordinary in-game player sidebar minimap content rectangle reached by `PowerClass::Draw -> RadarClass::Draw`, and its relationship to the previously verified `SSCR*` / `MPSSCRN*` right-panel parent rects.  
**Non-Scope:** `SSCRA*` close-frame lifecycle, shell/right-panel transition composition outside the ordinary in-game sidebar path, full tactical minimap terrain/color generation, and retail SHP frame pixel dumps.  
**Confidence:** High for the ordinary player sidebar content aperture and call path; Medium for the negative relationship to `SSCR*` / `MPSSCRN*` because this report only proves the reached in-game path and relies on prior selector/placement reports for those separate functions.  
**Active in YR:** Yes. Evidence: `PowerClass__Draw @ 0x0063FB20` calls `RadarClass__Draw @ 0x00653100` after power-bar drawing; `SidebarClass::Draw` was already verified to call `PowerClass::Draw` in the ordinary sidebar draw sequence.

## Target Question

What is the tactical minimap content rectangle/inset inside the Soviet ordinary player sidebar radar chrome, as reached from `PowerClass::Draw -> RadarClass::Draw`, and does it derive from the `SSCR*` / `MPSSCRN*` parent rects?

## Non-goals

- Do not re-prove Soviet `SSCR*` / `MPSSCRN*` selector names or parent placement unless needed for the relationship check.
- Do not trace `SSCRA*` close-frame consumers.
- Do not expand into terrain color/shroud/object-dot generation except where needed to identify the destination rect.
- Do not modify Rust, INI files, or tracked docs outside this one research report and the shared claims file.

## Evidence Needed To Mark COMPLETE

- Prove `RadarClass::Draw` is reached by ordinary sidebar drawing through `PowerClass::Draw`.
- Prove the fixed native aperture constants used by `RadarClass`.
- Prove the side/theater branch that sets the minimap x inset for Soviet/YR layout.
- Prove the surface blit destination rectangle used for minimap content on `g_SidebarSurface`.
- Prove whether the ordinary in-game minimap content rect consumes `DAT_00b0fc1c` / `SSCR*` / `MPSSCRN*` right-panel parent rects.

## Stop Conditions

- Stop before any mutating Ghidra operation.
- Stop if Ghidra read-only decompile/disassembly is unavailable.
- Stop before tracing full minimap pixel/color generation.
- Stop before turning this into a full radar transition lifecycle report.

## 1. Overview

The ordinary player sidebar minimap content is drawn by `RadarClass::Draw` and `RadarClass::Update`, reached from `PowerClass::Draw`. Its content aperture is a sidebar-surface-local rectangle with maximum size `140 x 108`, normally positioned at `(16,49)` within the sidebar surface for standard YR layout. For smaller generated radar surfaces, the surface is centered inside that `140 x 108` aperture by integer division.

This ordinary in-game content rect is not computed from the right-panel `DAT_00b0fc1c` parent rect used by the previously verified `SSCR*` / `MPSSCRN*` shell/right-panel draws. The in-game path uses `g_SidebarSurface`, `BKGDLG/BKGDLGY` radar chrome globals, and `RadarClass` fields at `this+0x11E4..0x1218`.

## 2. Key Offsets And Globals

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `this+0x11E4` | sidebar-surface x for radar chrome; initialized `0` | `RadarClass__One_Time @ 0x00652CF0`; draw uses at `0x006575B2` | Yes |
| `this+0x11E8` | top-strip/sidebar y helper; initialized `16` | `0x00652CF0`; `RadarClass__Draw` first dirty draw uses it | Yes |
| `this+0x11EC` | radar chrome y; initialized `48` | `0x00652CF0`; draw uses at `0x006575BE` | Yes |
| `this+0x11F0` | minimap content x base; Soviet/YR branch computes `(168 - 145) / 2 + 5 = 16` | `RadarClass__Init_For_House @ 0x00652E90`; disasm context `0x00652F05..0x00652F13` | Yes |
| `this+0x11F4` | minimap content y base; initialized `49` | `0x00652CF0`; `RebuildRadarSurfaces @ 0x00654725` reads it | Yes |
| `this+0x11F8` / `this+0x1200` | max radar surface/content width, `0x8C` / `140` | `0x00652CF0` | Yes |
| `this+0x11FC` / `this+0x1204` | max radar surface/content height, `0x6C` / `108` | `0x00652CF0` | Yes |
| `this+0x120C..0x1218` | dirty/content blit rect on `g_SidebarSurface` | `RadarClass__Draw @ 0x00653100`, `RadarClass__Update @ 0x00656EC0` | Yes |
| `this+0x121C` | generated minimap/content BSurface copied into sidebar | `RadarClass__RebuildRadarSurfaces @ 0x00654650`, `RadarClass__Update @ 0x00656EC0` | Yes |
| `this+0x149C..0x14A8` | active content destination x/y/w/h after generated surface sizing and centering | `RadarClass__RebuildRadarSurfaces @ 0x00654650` | Yes |
| `DAT_00b04a38` | active in-game radar chrome SHP pointer, assigned from `g_BKGDLG_SHP` | `RadarClass__Init_For_House @ 0x00652E90` | Yes |

## 3. Core Logic

### 3.1 Ordinary path liveness

Active in YR: Yes.

`PowerClass__Draw @ 0x0063FB20` draws the power bar if its dirty/force conditions are true, then calls `RadarClass__Draw` unconditionally before returning. `get_function_callers` reports `PowerClass__Draw @ 0x0063FB20` as the caller of `RadarClass__Draw @ 0x00653100`. This is the path already reached by `SidebarClass::Draw` in ordinary sidebar composition.

Evidence:

- Decompile: `PowerClass__Draw @ 0x0063FB20`.
- Caller proof: Ghidra `get_function_callers(ram:00653100)` -> `PowerClass__Draw @ 0063fb20`.
- Disassembly-read proof: `RadarClass__Draw @ 0x00653100..0x006536FE` was readable.

### 3.2 Fixed content aperture constants

Active in YR: Yes.

`RadarClass__One_Time @ 0x00652CF0` initializes the native content/chrome constants:

- `this+0x11E4 = 0`
- `this+0x11E8 = 0x10` / `16`
- `this+0x11EC = 0x30` / `48`
- `this+0x11F4 = 0x31` / `49`
- `this+0x11F8 = 0x8C` / `140`
- `this+0x1200 = 0x8C` / `140`
- `this+0x11FC = 0x6C` / `108`
- `this+0x1204 = 0x6C` / `108`

Evidence: decompile `RadarClass__One_Time @ 0x00652CF0`; disassembly range `0x00652CF0..0x00652D6F` confirmed readable. These are not inferred from transparent pixels in `radar.shp`.

### 3.3 Soviet/YR x inset branch

Active in YR: Yes for the nonzero `g_ScenarioClass_Instance+0x34B8` branch; the exact semantic label of this field remains inherited from sidebar-layout reports, so this report only claims the branch behavior.

`RadarClass__Init_For_House @ 0x00652E90` calls its base/init virtual first, then branches on `*(int *)(g_ScenarioClass_Instance + 0x34B8)`.

- Branch `== 0`: `this+0x11F0 = (g_SIDEBAR_WIDTH_CONST - 0x90) / 2 + 4`, which is `(168 - 144) / 2 + 4 = 16`.
- Branch `!= 0`: `this+0x11F0 = (g_SIDEBAR_WIDTH_CONST - 0x91) / 2 + 5`, which is `(168 - 145) / 2 + 5 = 16` with integer division.

The same function assigns `DAT_00b04a38 = g_BKGDLG_SHP`, then stores button SHP dimensions from `DIPLOBTN` / `OPTBTN` constructor results. Those button positions are related to the same sidebar/radar area but are not the minimap content rect.

Evidence: decompile `0x00652E90`; assembly context around `0x00652F05..0x00652F13` confirms the nonzero branch constant sequence; assembly context around `0x00652F4F..0x00652F55` confirms `this+0x11F0` is copied to `DAT_00b04a1c`.

### 3.4 Generated surface size and centering inside 140x108

Active in YR: Yes.

`RadarClass__RebuildRadarSurfaces @ 0x00654650` builds/refreshes the generated terrain surface at `this+0x1220`, then stores that generated surface width/height into `this+0x14A4` and `this+0x14A8`.

It then computes the sidebar-surface destination origin:

```text
0x149C = +0x11F0
if generated_width < 140:
    +0x149C = ((140 - generated_width) / 2) + +0x11F0

+0x14A0 = +0x11F4
if generated_height < 108:
    +0x14A0 = ((108 - generated_height) / 2) + +0x11F4
```

The divide-by-two is signed-integer truncation from `CDQ; SUB; SAR 1` in the assembly context, but the guarded `(140 - generated_width)` and `(108 - generated_height)` values are nonnegative in these branches, so this behaves as floor division for odd remaining margins.

Evidence:

- Decompile `RadarClass__RebuildRadarSurfaces @ 0x00654650`.
- Assembly context: `0x006546E0..0x00654723` reads generated width, compares to `0x8C`, subtracts, `SAR 1`, and adds `this+0x11F0`.
- Assembly context: `0x00654725..0x00654742` reads `this+0x11F4`, compares generated height to `0x6C`, subtracts, `SAR 1`, and adds `this+0x11F4`.

### 3.5 Sidebar blit destination for minimap content

Active in YR: Yes when radar mode is online (`this+0x14B0 == 1`) and radar state is active (`this+0x14AC == 1`).

`RadarClass__Draw @ 0x00653100` marks `bVar2` true only when mode/state indicate active radar. `RadarClass__Update @ 0x00656EC0` then copies from the generated content surface `this+0x121C` to `g_SidebarSurface`.

When a full radar redraw is pending, it first draws `DAT_00b04a38` frame `0x20` at `(this+0x11E4, this+0x11EC)`, i.e. normally `(0,48)`, then blits the minimap content surface to the destination rect held at `this+0x120C..0x1218`. For the full active area that rect is populated from `this+0x149C..0x14A8`, so the normal full-size generated surface lands at:

```text
x = 16
y = 49
w = generated_width, normally up to 140
h = generated_height, normally up to 108
```

For a generated surface smaller than `140 x 108`, `x` and/or `y` are centered as described above.

Evidence:

- Decompile `RadarClass__Draw @ 0x00653100` and `RadarClass__Update @ 0x00656EC0`.
- Assembly context `0x006575B2` / `0x006575BE` loads `this+0x11E4` / `this+0x11EC` for the frame-32 chrome draw.
- Assembly context `0x006575E5..0x006575FC` draws `DAT_00b04a38` frame `0x20`.
- Assembly context `0x0065760F..0x0065764F` computes source offsets by subtracting `this+0x149C` / `this+0x14A0` and calls `g_SidebarSurface` vtable `+0x08` with destination rect `this+0x120C..0x1218` and source surface `this+0x121C`.

### 3.6 Relationship to `SSCR*` / `MPSSCRN*` parent rects

Active in YR: Yes for the negative relationship in the ordinary in-game sidebar path.

No decompiled function on this in-game path (`PowerClass__Draw`, `RadarClass__Draw`, `RadarClass__Update`, `RadarClass__RebuildRadarSurfaces`, `RadarClass__Init_For_House`) reads `DAT_00b0fc1c`, calls `RadarBackground @ 0x0072E920`, calls `FUN_0072E9F0`, or calls `FUN_0072EAD0`. The content draw target is `g_SidebarSurface`, using local `RadarClass` fields and the generated radar content surface.

Therefore, for ordinary player sidebar minimap content, do not anchor the terrain/content rectangle to the `SSCR*` / `MPSSCRN*` right-panel parent rect. Those parent rects remain relevant to the separate right-panel shell/radar transition functions verified in `SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT.md`.

Evidence:

- Decompile of `PowerClass__Draw @ 0x0063FB20` -> direct `RadarClass__Draw`.
- Decompile of `RadarClass__Draw @ 0x00653100` and `RadarClass__Update @ 0x00656EC0` -> no right-panel parent global usage.
- Prior report `SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT.md` confines `DAT_00b0fc1c` use to `RightPanel__ComputeLayoutRects`, `RadarBackground`, `FUN_0072E9F0`, and `FUN_0072EAD0`.

## 4. Current Rust Implementation Status

Focused scan:

- `src/sidebar/mod.rs::radar_minimap_rect_with_spec` centers a `RADAR_CONTENT_WIDTH = 150` by `RADAR_CONTENT_HEIGHT = 96` rectangle inside a generic `168 x 110` radar block at `screen_w - sidebar_width`.
- `src/sidebar/layout_spec.rs` exposes configurable `radar_content_width` and `radar_content_height` seeded from those constants.
- `src/render/sidebar_chrome.rs` loads `radar.shp` / `radary.shp`, then derives content insets by scanning transparent pixels in frame `0`.
- `src/render/minimap.rs::build_minimap_instance_in_rect` stretches the minimap texture into the caller-provided rectangle.
- `src/app_render/build_instances.rs::build_sidebar_instances` uses `active_minimap_screen_rect` and renders the minimap only when radar animation is online.

Current Rust delta: the Rust minimap content rectangle is derived from generic `radar.shp` transparent-opening detection and stock `150 x 96` constants, not from gamemd's active in-game `RadarClass` aperture of max `140 x 108` at sidebar-surface `(16,49)` with small-surface centering. Rust also derives the screen x from `screen_w - 168`, while the native in-game proof here is sidebar-surface-local and must be composed through the sidebar surface/blit transform.

## 5. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / surface | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `PowerClass__Draw @ 0x0063FB20` | ordinary sidebar draw reaches this; power dirty branch optional | `POWERP.SHP` | x `0`/`5` branch, y `g_SidebarWidth+0x45`; see sibling power report | `g_SidebarSurface` draw context | Yes | power layer before radar |
| 2 | `RadarClass__Draw @ 0x00653100` | called unconditionally by `PowerClass__Draw` | top/radar frame globals | dirty/state dependent | `g_SidebarSurface` | Yes | radar state owner |
| 3 | `RadarClass__Update @ 0x00656EC0` | active radar mode/state, dirty/content changes | `DAT_00b04a38` frame `0x20` | `(this+0x11E4, this+0x11EC)` -> normally `(0,48)` | active sidebar convert path | Yes when active/full redraw | open chrome frame |
| 4 | `RadarClass__Update @ 0x00656EC0` | content rect w/h positive | generated surface `this+0x121C` | dest `this+0x120C..0x1218`; full active content comes from `this+0x149C..0x14A8` -> normally `(16,49,140,108)` | surface blit, not SHP draw | Yes | terrain/minimap content |
| 5 | `RadarClass__Update @ 0x00656EC0` | after content blit in active path | viewport/selection overlay calls via `g_SidebarSurface+0x58` | `this+0x14DC` and expanded `this+0x149C-1/+14A0-1/+2` rect | surface primitive | Yes | viewport overlay/dirty boundary |

## 6. Asset Role Matrix

| Asset / surface | Loaded | Drawn | Visible in target | Content | Chrome/container | Overlay | Transition-only | Inactive in target | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `BKGDLG.SHP` / `BKGDLGY.SHP` via `DAT_00b04a38` | Yes | Yes | Yes | No | Yes | No | No | `0x00652E90`, `0x006575E5..0x006575FC` |
| generated terrain surface `this+0x121C` | Yes, runtime-created | Yes via blit | Yes | Yes | No | No | No | `0x00654650`, `0x0065760F..0x0065764F` |
| `DIPLOBTN.SHP` / `OPTBTN.SHP` | Yes | Not traced in this slice | Conditional | No | UI button | No | No | No | `0x00652F5B..0x00652F80`, `FUN_00653010` |
| `SSCR*` | Prior-proven loaded/drawn in right-panel functions | Not by this path | No for ordinary in-game content path proven here | No | Right-panel chrome/transition | No | Yes/separate path | Yes for this path | Prior report + no refs in `0x0063FB20/0x00653100/0x00656EC0` |
| `MPSSCRN*` | Prior-proven loaded/drawn in right-panel movie function | Not by this path | No for ordinary in-game content path proven here | No | Movie/chrome transition | No | Yes/separate path | Yes for this path | Prior report + no refs in `0x0063FB20/0x00653100/0x00656EC0` |
| Rust `radar.shp` transparent opening | Rust-loaded | Rust-visible | Rust-only current implementation | Used by Rust | Rust chrome | No | No | Not gamemd proof | `src/render/sidebar_chrome.rs` scan |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `PowerClass__Draw -> RadarClass__Draw` liveness | verified | `0x0063FB20`, caller xref to `0x00653100` | none |
| `RadarClass__One_Time` aperture constants | verified | `0x00652CF0`, disasm range `0x00652CF0..0x00652D6F` | none |
| `RadarClass__Init_For_House` x inset branch | verified | `0x00652E90`, assembly context `0x00652F05..0x00652F55` | exact semantic name/default of `ScenarioClass+0x34B8` remains sibling-doc territory |
| `RadarClass__RebuildRadarSurfaces` centering | verified | `0x00654650`, assembly context `0x006546E0..0x00654742` | none |
| `RadarClass__Update` content blit destination | verified | `0x00656EC0`, assembly context `0x006575B2..0x0065764F` | exact overlay primitive details after content blit remain out of scope |
| Relationship to `SSCR*` / `MPSSCRN*` parent rect | verified-negative for this path | no usage in `0x0063FB20/0x00653100/0x00656EC0`; prior `SOVIET_RADAR_RECT...` report | separate transition lifecycle remains out of scope |
| `SSCRA*` close-frame consumer | deferred | not on scoped path | separate `SSCRA_CLOSE_FRAME_DRAW_LIFECYCLE` target |
| Retail frame pixel dimensions/offsets | deferred | not required for binary aperture proof | separate asset dump target |

## 8. Open Questions - Final State

- `[RESOLVED] Q1` - Is `RadarClass::Draw` on the ordinary in-game player sidebar path? -> Yes, it is called by `PowerClass__Draw`, which is in the ordinary sidebar draw sequence. (evidence: `0x0063FB20`, caller xref to `0x00653100`)
- `[RESOLVED] Q2` - What fixed maximum content aperture does gamemd use? -> `140 x 108` (`0x8C x 0x6C`). (evidence: `0x00652CF0`)
- `[RESOLVED] Q3` - What is the normal content top-left in the sidebar surface? -> `x=16`, `y=49` before small-surface centering. (evidence: `0x00652E90`, `0x00652CF0`, `0x00654650`)
- `[RESOLVED] Q4` - How are smaller generated radar surfaces placed? -> Centered within `140 x 108` by integer floor half-margin added to `(16,49)`. (evidence: `0x00654650`, assembly context `0x00654715..0x00654742`)
- `[RESOLVED] Q5` - Does the ordinary minimap content blit use `g_SidebarSurface`? -> Yes, via vtable `+0x08` with destination rect `this+0x120C..0x1218` and source surface `this+0x121C`. (evidence: `0x00656EC0`, assembly context `0x0065760F..0x0065764F`)
- `[RESOLVED] Q6` - Does the ordinary content rect derive from `DAT_00b0fc1c`? -> No evidence in this path; the path uses `RadarClass` fields and `g_SidebarSurface`. (evidence: `0x0063FB20`, `0x00653100`, `0x00656EC0`)
- `[RESOLVED] Q7` - Does this prove `SSCR*` / `MPSSCRN*` content insets? -> No; it proves those parent rects are not the ordinary in-game content rect source. (evidence: this report plus `SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT.md`)
- `[DEFERRED] Q8` - Which function consumes `SSCRA*` close frames? (category: out-of-scope; reason: separate swarm slot owns close-frame lifecycle; next-step-if-pursued: trace xrefs from `g_RadarFrameClose_SHP` and close-state transition callers)
- `[DEFERRED] Q9` - What are the retail frame dimensions and embedded offsets for all Soviet sidebar radar SHPs? (category: out-of-scope; reason: separate asset-dump slot; next-step-if-pursued: dump SHP headers/frames from retail MIX files and reconcile with binary constants)
- `[DEFERRED] Q10` - What exact primitive draws the viewport rectangle after the minimap blit? (category: bounded-cost-too-high; reason: not required to prove content inset; next-step-if-pursued: trace `g_SidebarSurface` vtable `+0x58` and `this+0x1208`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary in-game minimap content aperture is sidebar-surface-local `(16,49,140,108)` for full-size generated radar content, with smaller generated surfaces centered within `140 x 108`. | `0x00652CF0`, `0x00652E90`, `0x00654650`, `0x00656EC0` | Rust uses `RADAR_CONTENT_WIDTH=150`, `RADAR_CONTENT_HEIGHT=96`, centered in a generic `168x110` block. | `src/sidebar/mod.rs`, `src/sidebar/layout_spec.rs`, `src/app_render/build_instances.rs`, `src/render/minimap.rs` | Replace transparent/opening-derived generic minimap rect with native `RadarClass` aperture semantics for the ordinary player sidebar path. | At 800x600 Soviet sidebar, minimap content occupies screen-space equivalent of sidebar-surface `(16,49,140,108)` after the sidebar-surface-to-screen transform; a 139x107 generated surface shifts to `(16,49)` plus floor half-margin where applicable. Proposed test: `test_soviet_minimap_content_uses_native_140x108_aperture_at_16_49`. | HIGH screenshot/layout risk; do not use `150x96` or transparent-opening detection as the parity source. |
| The ordinary minimap content rect is not anchored to `SSCR*` / `MPSSCRN*` `DAT_00b0fc1c`; those are separate right-panel transition/chrome draws. | `0x0063FB20`, `0x00653100`, `0x00656EC0`; prior `SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT.md` | Rust currently conflates generic `radar.shp` chrome/animation and minimap rect in `sidebar_chrome`/`radar_anim`. | `src/render/sidebar_chrome.rs`, `src/render/radar_anim.rs`, `src/app_render/build_instances.rs` | Split ordinary in-game BKGDLG minimap aperture from right-panel `SSCR*`/`MPSSCRN*` transition parent placement. | A unit test or screenshot harness can assert that changing the `SSCR*` parent origin formula does not move the ordinary in-game minimap content aperture. Proposed test: `test_soviet_ingame_minimap_rect_does_not_use_sscr_parent_rect`. | HIGH conflation risk; do not feed `DAT_00b0fc1c` into the ordinary minimap terrain rectangle. |
| Full active redraw draws the open radar chrome frame (`DAT_00b04a38` frame `0x20`) at `(0,48)` before the content blit to `(16,49,...)`. | `0x006575B2..0x0065764F` | Rust draws minimap and `radar.shp` animation through independent batches and derives content from the `radar.shp` frame. | `src/render/sidebar_chrome.rs`, `src/app_render/build_instances.rs`, `src/render/minimap.rs` | Preserve painter order: chrome frame first, then terrain/content surface blit inside aperture, then subsequent overlay primitives. | In a forced radar redraw, a pixel at chrome border `(0,48)` comes from BKGDLG frame 32 while content starts at `(16,49)`. Proposed test: `test_radar_full_redraw_draws_bkgdlg_frame_32_before_minimap_blit`. | MEDIUM-HIGH overlap risk; do not infer draw order from asset load order or from Rust batch order. |

## Negative Facts / Do Not Do

- Do not derive the ordinary player minimap content rectangle by scanning transparent pixels in `radar.shp`; the binary uses explicit `RadarClass` constants and generated-surface dimensions. Evidence: `0x00652CF0`, `0x00654650`; Rust scan: `src/render/sidebar_chrome.rs`.
- Do not use Rust's current `150 x 96` content size for parity. The active binary aperture is `140 x 108` max, with centering for smaller generated surfaces. Evidence: `0x00652CF0`, `0x00654705..0x00654742`.
- Do not anchor ordinary in-game minimap content to `DAT_00b0fc1c` or `SSCR*` / `MPSSCRN*` parent rects. Evidence: no usage in `0x0063FB20`, `0x00653100`, `0x00656EC0`; prior right-panel placement report scopes those globals elsewhere.
- Do not treat `BKGDLG/BKGDLGY` as "loaded but unused" for ordinary in-game radar; `DAT_00b04a38` is drawn by `RadarClass__Draw/Update`. Evidence: `0x00652E90`, `0x006575E5..0x006575FC`.
- Do not assume every radar redraw copies the full aperture. Dirty rects can narrow `this+0x120C..0x1218`; full active redraw uses `this+0x149C..0x14A8`. Evidence: dirty-rect merge logic in `0x00653100` and content blit in `0x00656EC0`.

## Remaining Uncertainty

- `SSCRA*` close-frame lifecycle remains unverified in this report.
- Retail SHP frame dimensions and embedded offsets were not dumped here; this report proves the binary aperture, not asset pixel masks.
- The exact semantic label/default source for `g_ScenarioClass_Instance+0x34B8` is inherited from sibling layout work; this report only verifies the branch and constants.
- The exact viewport rectangle overlay primitive after the content blit is touched but not exhausted.

## Stale Docs / Follow-up Docs

- `docs/research/RADAR_CHROME_COMPOSITING.md`: replace "BKGDLG.SHP frame 32 -> sidebar surface at (0, 48) [chrome border + dark inner]" plus "Blit at (viewport_x, viewport_y) = (16, 49)" with "Ordinary in-game radar full active redraw draws `DAT_00b04a38` frame `0x20` at `this+0x11E4,this+0x11EC` (normally `(0,48)`) and blits the generated radar surface to `this+0x149C,this+0x14A0` (normally `(16,49)`) with size from the generated surface, max `140x108`, centered inside that aperture when smaller."
- `docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace "The actual minimap content area within the chrome is 140 x 108 pixels (0x8C x 0x6C), inset at (16, 49) on the sidebar surface" with "The ordinary in-game minimap content aperture is max `140x108`; the generated surface is placed at `(16,49)` only when it is full-sized, and smaller generated surfaces are centered within the `140x108` aperture by `RadarClass__RebuildRadarSurfaces @ 0x00654650`."
- `src/render/sidebar_chrome.rs` comment-level stale implementation assumption: replace "Content insets derived from the transparent opening in radar.shp frame 0" with "For gamemd parity, ordinary in-game minimap content must use the binary `RadarClass` aperture `(16,49,140,108)` on the sidebar surface; transparent-opening detection is a non-parity fallback only."

## Sources

- Ghidra read-only decompiles: `0x0063FB20`, `0x00653100`, `0x00652CF0`, `0x00652E90`, `0x00654650`, `0x006547C0`, `0x00656EC0`.
- Ghidra read-only assembly/disassembly ranges or contexts: `0x00652CF0..0x00652D6F`, `0x00652F05..0x00652F55`, `0x006546E0..0x00654742`, `0x006575B2..0x0065764F`.
- Existing docs used as navigation/cross-checks: `docs/research/SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT.md`, `docs/research/RADAR_CHROME_COMPOSITING.md`, `docs/research/SIDEBAR_RADAR_POSITIONING.md`, `docs/research/SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT_GHIDRA_REPORT.md`.
- Rust scan: `src/sidebar/mod.rs`, `src/sidebar/layout_spec.rs`, `src/render/sidebar_chrome.rs`, `src/render/minimap.rs`, `src/app_render/build_instances.rs`, `src/app_sidebar_render.rs`.

## Status

COMPLETE for the scoped ordinary in-game `PowerClass::Draw -> RadarClass::Draw` minimap content aperture and its negative relationship to `SSCR*` / `MPSSCRN*` parent rects.
