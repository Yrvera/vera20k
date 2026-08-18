# Building Bracket Runtime Overlap Cases - Ghidra Research Report

**Address(es):** `0x006D8DB0`, `0x006F60D0`, `0x006F5190`, `0x006DBB60`, `0x004BFD30`, `0x0043CEA0`, `0x0043D290`, `0x0043DA80`, `0x004801F0`, `0x0047EFE0`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** implementation-oriented representative standard YR selected-building bracket cases where draw order, A-buffer, Z-test, or first-pass conditional art changes visible pixels.  
**Non-Scope:** screenshot capture, exhaustive per-building pixel enumeration, display-layer sort proof beyond prior reports, full gap visual renderer, or Rust implementation.  
**Confidence:** High for binary contracts and stock-data case selection; Medium for exact final pixels in each map/camera placement because this slot did not run runtime captures.  
**Active in YR:** Yes for selected-building brackets, ordinary shroud A-buffer, ordinary SHP building bodies, gates, construction/buildup art, and selected gap-generator visual rings; Conditional for gap visual overlap and VXL/turret cases depending on stock building type/state; No for TS fog-of-war dimming by default.

## 1. Overview

Selected building brackets are not a pure UI overlay. The engine submits back/left bracket stubs in `DrawBehind`, front/right/top stubs in `DrawExtras`, and a later second `DrawExtras` pass after first-pass object drawing. Final line pixels are written through the primary surface line drawer, which Z-tests against `g_ZBuffer`, reads `g_ABuffer`, and suppresses or modulates pixels from A-buffer values.

The cases below are representative implementation fixtures. They are meant to exercise the contracts that change player-visible pixels without requiring an exhaustive screenshot matrix.

## 2. Runtime Contracts Verified

| Contract | Active in YR | Evidence |
|---|---|---|
| Selected building bracket gate is `Techno+0x83 != 0` and `WhatAmI()==6`. | Yes | `TechnoClass::DrawBehind @ 0x006F60D0`, `TechnoClass::DrawExtras @ 0x006F5190`, `BuildingClass::WhatAmI @ 0x00459EC0` from prior bracket reports and fresh decompile. |
| Back bracket work is separate from front bracket work. | Yes | `0x006F60D0` emits five back/left edges; `0x006F5190` emits four `DrawBracketCorner` edges plus three direct stubs. |
| A later second `DrawExtras` pass redraws selected building front bracket work after first-pass object work. | Yes | `Tactical_ObjectRenderingLoop @ 0x006D8DB0`, second loop at `0x006D95AF..0x006D97B5` calls vtable `+0x110`. |
| Bracket line pixels Z-test and A-buffer-modulate, but stock bracket callers do not Z-write. | Yes | `Tactical::DrawLine3D @ 0x006DBB60`; `Surface::DrawLine_ABufModulated_ZClipped @ 0x004BFD30`; stock bracket final flag `0` at prior evidence sites `0x006F5FCE`, `0x006F5762`, `0x006F58B1`, `0x006F59D3`. |
| Ordinary shroud writes non-neutral values into `g_ABuffer`; `0` suppresses bracket pixels, nonzero/non-`0x7F` dims them. | Yes | `Shroud_fog_edge_rendering @ 0x004801F0`, `ShroudEdge_BlitToABuffer @ 0x0047EFE0`, surface predicate in `0x004BFD30`. |
| Fog-of-war A-buffer dimming uses the same path but is off in standard YR. | No by default | `0x004801F0` checks `SpecialFlags & 0x1000`; `rulesmd.ini [MultiplayerDialogSettings] FogOfWar=no` in prior A-buffer report. |
| First-pass `BuildingClass +0x104(flag=1)` can draw conditional gate/construction overlay and VXL body/turret art. | Conditional | `FUN_0043CEA0 @ 0x0043CEA0`, `FUN_0043DA80 @ 0x0043DA80`; stock data examples below. |

## 3. Representative Cases

### Case A - Ordinary SHP Building Silhouette: `GACNST`

Setup: select a visible Allied Construction Yard with stock art `[GACNST] Foundation=4x4`, `Height=4`, `Buildup=GACNSTMK`.

Expected implementation behavior: draw `DrawBehind` back/left bracket stubs before building body work, then draw front/right/top `DrawExtras` stubs in the bracket phase. Do not draw all twelve bracket stubs as a single final no-depth UI overlay: back/ground stubs that fall under the body silhouette must be absent or covered, while front/top stubs reappear in the later `DrawExtras` phase.

Why this changes pixels: a one-pass overlay makes rear and buried ground stubs visible where gamemd has them hidden by body/order; a body-only pass without the second `DrawExtras` loses front/top bracket pixels that gamemd redraws.

Active in YR: Yes. Evidence: `0x006F60D0`, `0x006F5190`, `0x006D8DB0`, `0x0043D290`; stock art `ini/artmd.ini:1599-1604`.

### Case B - Shroud-Edge A-buffer Suppression/Dim: `GAPOWR` or `GACNST` At Reveal Boundary

Setup: select an owned visible standard building near the edge of unexplored shroud so one or more bracket stubs cross SHROUD.SHP edge pixels. `GACNST` is a large easy fixture; `GAPOWR` is a small common fixture.

Expected implementation behavior: bracket candidates that land where `g_ABuffer==0` are not drawn; bracket candidates that land on nonzero/non-`0x7F` shroud values are channel-modulated; neutral `0x7F` pixels draw normal bracket color.

Why this changes pixels: a UI overlay drawn after shroud ignores the original A-buffer contract and shows full-white bracket pixels through black or partially dark shroud edges.

Active in YR: Yes. Evidence: `0x004BFD30` write predicate; `0x004801F0` and `0x0047EFE0` shroud A-buffer writes; `GACNST` art `ini/artmd.ini:1599-1604`, `GAPOWR` art `ini/artmd.ini:3206-3215`.

### Case C - Gap-Generator Visual A-buffer Modulation: Selected Building Under `GAGAP` Field

Setup: select a visible owned building whose bracket pixels pass through a Gap Generator visual field, or select the `[GAGAP]` itself while its field is active. The stock Allied Gap Generator has `GapGenerator=yes`, `Foundation=1x1`, `Height=6`.

Expected implementation behavior: gap visuals must affect the same A-buffer modulation stage before bracket line writes. Do not treat gap coverage as ordinary SHROUD.SHP edges; prior binary-backed shroud docs say gap visual darkening is a separate A-buffer/AlphaShape system.

Why this changes pixels: treating gap as only entity visibility or as only shroud-edge state misses the darkened bracket pixels inside the gap visual area.

Active in YR: Conditional. The stock `[GAGAP]` exists and uses `GapGenerator=yes`; the pixel effect requires bracket pixels under the active gap visual field. Evidence: `ini/rulesmd.ini:12221-12226`, `ini/artmd.ini:4700-4713`, prior binary-backed `SHROUD_DISPARITIES.md:273-291` for separate gap A-buffer overlay.

### Case D - Two Overlapping Selected Buildings: Adjacent `GACNST` + `GAPOWR` or Two Gates

Setup: place two selected visible buildings whose screen silhouettes/brackets overlap, e.g. a large `GACNST` next to `GAPOWR`, or two adjacent selected `GAGATE_A` wall gates.

Expected implementation behavior: first-pass order is per display-layer object (`DrawBehind`, first `DrawExtras`, first-pass draw dispatcher), but final front bracket work is phase-batched by the second `DrawExtras` loop. Implementations must not model overlap as strictly per-object `back -> body -> front` for each building.

Why this changes pixels: the later second `DrawExtras` pass lets an earlier building's front stubs be submitted after later first-pass object art. A per-object-only renderer can leave those stubs hidden or incorrectly place later buildings' body pixels over them.

Active in YR: Yes for selected visible buildings; exact pair overlap depends on placement. Evidence: `0x006D8DB0` first loop and second loop; `BUILDING_BRACKET_MULTI_OBJECT_INTERLEAVING_GHIDRA_REPORT.md`; `GACNST` art `ini/artmd.ini:1599-1604`; `GAGATE_A` art/rules `ini/artmd.ini:4204-4214`, `ini/rulesmd.ini:17186-17206`.

### Case E - Gate First-Pass Overlay: Selected `GAGATE_A`

Setup: select stock `[GAGATE_A]` while opening/closing. Art has `SpecialZOverlay=GAGATEZA`, `GateStages=9`, `Buildup=GAGATE_A`; rules have `Selectable=yes`, `Gate=yes`, `GateCloseDelay=.2`.

Expected implementation behavior: first-pass `+0x104(flag=1)` can draw gate/construction-style SHP overlay art between the first bracket submissions and the second `DrawExtras`. The second `DrawExtras` pass must still redraw selected front stubs above this conditional first-pass art.

Why this changes pixels: if first-pass gate art is omitted or if selected brackets are only submitted once before it, moving gate overlay pixels can cover or fail to cover bracket stubs differently from gamemd.

Active in YR: Conditional. The gate exists in stock YR and is selectable, but the overlay path requires the gate/construction state helpers to report an active state. Evidence: `FUN_0043DA80 @ 0x0043DA80`; `ini/artmd.ini:4204-4214`; `ini/rulesmd.ini:17186-17206`.

### Case F - Stock Turret/VXL-Style First-Pass Art: `YAREFN` / `YAGGUN`

Setup: select stock Yuri Ore Refinery `[YAREFN]` or Yuri Gattling Cannon `[YAGGUN]`. `[YAREFN]` has `Turret=yes`, `TurretAnim=SMINTUR`; `[YAGGUN]` has `Turret=yes`, `TurretAnim=YAGGUN`.

Expected implementation behavior: conditional building first-pass art from `FUN_0043DA80` can be drawn after the first selected brackets and before the second `DrawExtras`; front stubs need the later pass to land above conditional moving/turret/VXL-style building art.

Why this changes pixels: a renderer that emits brackets once before building first-pass extras can let turret/body art cover final front bracket pixels; a renderer that emits all brackets last can incorrectly expose back stubs.

Active in YR: Conditional. The binary path is active for building types with the relevant VXL/turret gates; the stock examples provide selected turreted building fixtures, but exact `Type+0x16C5/+0x16C6` mapping remains a follow-up. Evidence: `FUN_0043DA80 @ 0x0043DA80`; `ini/artmd.ini:1799-1814`, `ini/rulesmd.ini:13234-13246`, `ini/artmd.ini:4932-4960`, `ini/rulesmd.ini:13590-13618`.

## 4. Implementation Implications

1. Keep at least three building-bracket phases: back/behind, first/front extras, and second/front extras. The second `DrawExtras` is not optional for overlap parity.
2. Route bracket line pixels through the same surface contract as gamemd: clip, Z-test, A-buffer sample, optional color modulation, no Z-write for stock bracket callers.
3. Do not classify shroud and gap as the same source. Shroud edge SHP writes A-buffer directly; gap generator visual darkening is a separate A-buffer/AlphaShape overlay system.
4. Use stock art/rules dimensions and state flags. The representative fixtures above should be derived from INI data, not hardcoded case geometry.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Bracket gate and selected-building reachability | verified | `0x006F60D0`, `0x006F5190`, `0x00459EC0` | none |
| Back/front bracket split | verified | `0x006F60D0`, `0x006F5190` | exact per-pixel screenshot matrix |
| Multi-object second `DrawExtras` phase | verified | `0x006D8DB0`, `0x006D95AF..0x006D97B5` | exact display-layer sort insertion not repeated here |
| Surface A-buffer/Z-test/no-Z-write contract | verified | `0x006DBB60`, `0x004BFD30`, prior caller flag evidence | none |
| Shroud A-buffer overlap | verified | `0x004801F0`, `0x0047EFE0`, `0x004BFD30` | exact SHROUD.SHP frame pixel table not enumerated |
| Gap visual A-buffer overlap | touched-not-exhausted | `SHROUD_DISPARITIES.md:273-291`, `ini/rulesmd.ini:12221-12226` | full gap renderer Ghidra re-trace out of scope |
| Gate/construction first-pass overlay | verified | `0x0043DA80`, `ini/artmd.ini:4204-4214`, `ini/rulesmd.ini:17186-17206` | exact gate frame pixel outcome |
| VXL/turret first-pass art | touched-not-exhausted | `0x0043DA80`, stock turret fixtures | exact art-key-to-`Type+0x16C5/+0x16C6` mapping |

## 6. Open Questions - Final State

[RESOLVED] OQ-RUNTIME-001 - Are bracket pixels final UI overlay pixels? No; they route through `Surface::DrawLine_ABufModulated_ZClipped` with Z-test and A-buffer modulation. Evidence: `0x006DBB60`, `0x004BFD30`. Active in YR: Yes.

[RESOLVED] OQ-RUNTIME-002 - Is standard shroud relevant to selected bracket pixels after the line reaches the surface routine? Yes; shroud writes `g_ABuffer`, and the line drawer suppresses/dims by the sampled value. Evidence: `0x004801F0`, `0x0047EFE0`, `0x004BFD30`. Active in YR: Yes.

[RESOLVED] OQ-RUNTIME-003 - Is TS fog-of-war dimming normal standard YR behavior? No; fog A-buffer blending is gated by `SpecialFlags & 0x1000`, and standard YR has `FogOfWar=no`. Evidence: `0x004801F0`, prior A-buffer report INI check. Active in YR: No by default.

[RESOLVED] OQ-RUNTIME-004 - Is overlapping selected-building order per-object only? No; there is a second phase that calls `DrawExtras` for visible techno objects after first-pass display layers. Evidence: `0x006D8DB0`. Active in YR: Yes.

[DEFERRED] OQ-RUNTIME-005 - Which stock buildings set the exact `Type+0x16C5/+0x16C6` VXL gates? Category: out-of-scope. This report needs representative conditional cases, not a full art parser field map. Next step: targeted BuildingType art-key parser investigation.

[DEFERRED] OQ-RUNTIME-006 - Exact pixel tables for each representative case. Category: needs-runtime-debugger. Static evidence identifies the contracts; screenshots/pixel probes should validate final implementation.

## Sources

- Ghidra decompiled read-only: `0x006D8DB0`, `0x006F60D0`, `0x006F5190`, `0x006DBB60`, `0x004BFD30`, `0x0043CEA0`, `0x0043D290`, `0x0043DA80`, `0x004801F0`, `0x0047EFE0`
- Prior reports: `building-selection-brackets/BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md`, `building-selection-brackets/BUILDING_BRACKET_MULTI_OBJECT_INTERLEAVING_GHIDRA_REPORT.md`, `building-selection-brackets/BUILDING_FIRST_PASS_DISPLAY_0043DA80_GHIDRA_REPORT.md`, `building-selection-brackets/TECHNO_DRAWBEHIND_BUILDING_BRACKET_EDGES_GHIDRA_REPORT.md`, `building-selection-brackets/TECHNO_DRAWEXTRAS_BUILDING_BRACKET_BLOCK_GHIDRA_REPORT.md`, `SHROUD_DISPARITIES.md`
- INI checked read-only: `ini/artmd.ini`, `ini/rulesmd.ini`
