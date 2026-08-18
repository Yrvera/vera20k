# SOVIET_RADAR_LEFT_PANEL_SHP_SELECTORS_FOLLOWUP_GHIDRA_REPORT

Status: COMPLETE

## Target Question

Retry the failed Soviet radar/left-panel selector slot using read-only Ghidra MCP.
Prove filename selection for side `1` (Soviet) in `FUN_0072D460`,
`FUN_0072D830`, and `FUN_0072FA10`, including `640` versus `800+`
predicates and whether the path is active in standard Yuri's Revenge.

## Non-goals

- Do not re-investigate ordinary sidebar chrome loading in
  `FUN_006D02B0` / `SidebarClass::LoadSHPs`.
- Do not prove MIX archive membership or resolver precedence.
- Do not inspect text color, Ready/status text, or ordinary build-cameo palette
  paths.
- Do not modify Rust, INI, Ghidra state, or published docs outside this report.

## Evidence Needed To Mark COMPLETE

- Read-only Ghidra decompile of all three target functions.
- Assembly/context evidence for the side and width predicates.
- Filename string source plus selector address for every Soviet-relevant branch.
- Caller or direct draw/layout consumer evidence proving the loaded SHPs are live
  in standard YR.
- At least one Rust-facing implementation handoff with concrete test names.

## Stop Conditions

- Stop if Ghidra MCP read-only access is unavailable.
- Stop if target function boundaries cannot be decompiled or inspected read-only.
- Stop before any mutating Ghidra operation.
- Stop if the investigation expands into unrelated sidebar systems.

## Verified Findings

### 1. Radar background selector uses explicit side filenames, not generic `RADAR.SHP`

Active in YR: Yes.

`RadarBackground_SHPLoad @ 0x0072D460` decompiles as a side selector over
`param_1` with a nested `g_ScreenWidth == 0x280` branch. Ghidra assembly shows:

- `0x0072D460..0x0072D477`: `TEST ECX,ECX`, `JNZ 0x0072D4EF`,
  `MOV EAX,[0x008A00A4]`, `CMP EAX,0x280` for Allied side `0`.
- `0x0072D4EF..0x0072D507`: `CMP ECX,0x1`, `JNZ 0x0072D57F`,
  `CMP EAX,0x280` for Soviet side `1`.
- `0x0072D57F+`: side values other than `1` use the Yuri/default branch.

The Soviet branch loads `SSCR*`, with `SM` only at exact width `640` and `MD`
otherwise:

| Side | Predicate | Filenames |
|---|---|---|
| Soviet `1` | `g_ScreenWidth == 0x280` | `SSCRBKSM.SHP`, `SSCRTSM.SHP`, `SSCRASM.SHP` |
| Soviet `1` | `g_ScreenWidth != 0x280` | `SSCRBKMD.SHP`, `SSCRTMD.SHP`, `SSCRAMD.SHP` |

Filename string sources:

- `[0x00844C70] -> 0x0084528C "SSCRBKSM.SHP"`
- `[0x00844C78] -> 0x00845270 "SSCRTSM.SHP"`
- `[0x00844C80] -> 0x00845258 "SSCRASM.SHP"`
- `[0x00844C74] -> 0x0084527C "SSCRBKMD.SHP"`
- `[0x00844C7C] -> 0x00845264 "SSCRTMD.SHP"`
- `[0x00844C84] -> 0x0084524C "SSCRAMD.SHP"`

YR activity evidence: wrapper `FUN_0072D300 @ 0x0072D300` calls
`RadarBackground_SHPLoad`, and caller `FUN_006C8E80 @ 0x006C8E80` passes
`ScenarioClass+0x34B8` as `ECX` before calling the wrapper:
`0x006C8E83..0x006C8E8E` loads `g_ScenarioClass_Instance`, reads
`[EAX+0x34B8]`, then calls `0x0072D300`. That wrapper is followed by
draw calls in `FUN_006C8E80`, so the selected SHPs are on an active standard
YR radar open/background path.

### 2. Yuri/default radar background reuses Soviet small assets at 640 but uses `SY*` at non-640

Active in YR: Yes for side `2`; not Soviet, but relevant to avoiding a wrong
Soviet/Yuri fallback.

For `param_1 != 0 && param_1 != 1`, `RadarBackground_SHPLoad @ 0x0072D460`
branches by the same width predicate:

| Side | Predicate | Filenames |
|---|---|---|
| Yuri/default | `g_ScreenWidth == 0x280` | `SSCRBKSM.SHP`, `SSCRTSM.SHP`, `SSCRASM.SHP` |
| Yuri/default | `g_ScreenWidth != 0x280` | `SYCRBKMD.SHP`, `SYCRTMD.SHP`, `SYCRAMD.SHP` |

Filename sources include `[0x00844C88] -> "SSCRBKSM.SHP"`,
`[0x00844C90] -> "SSCRTSM.SHP"`, `[0x00844C98] -> "SSCRASM.SHP"`,
`[0x00844C8C] -> "SYCRBKMD.SHP"`, `[0x00844C94] -> "SYCRTMD.SHP"`,
and `[0x00844C9C] -> "SYCRAMD.SHP"`.

### 3. Minimap/radar transition movie selector uses `MPSSCRN*` for Soviet

Active in YR: Yes.

`RadarTransitionMovie_SHPLoad @ 0x0072D830` decompiles as the same side selector
shape. Ghidra assembly shows:

- `0x0072D830..0x0072D843`: side `0` plus `g_ScreenWidth == 0x280`.
- `0x0072D867..0x0072D87B`: `CMP ECX,0x1`, then Soviet width branch.
- `0x0072D89F..0x0072D8B7`: default/Yuri branch.

The Soviet filenames are:

| Side | Predicate | Filename |
|---|---|---|
| Soviet `1` | `g_ScreenWidth == 0x280` | `MPSSCRNS.SHP` |
| Soviet `1` | `g_ScreenWidth != 0x280` | `MPSSCRNL.SHP` |

Filename string sources:

- `[0x00844CA8] -> 0x008451F4 "MPSSCRNS.SHP"`
- `[0x00844CAC] -> 0x008451E4 "MPSSCRNL.SHP"`

YR activity evidence: wrapper `FUN_0072D730 @ 0x0072D730` calls
`RadarTransitionMovie_SHPLoad`, and caller `FUN_005C9720 @ 0x005C9720`
loads `ScenarioClass+0x34B8` before calling the wrapper:
`0x005C9723..0x005C972E` reads `[EAX+0x34B8]` and calls `0x0072D730`;
the wrapper then feeds `FUN_0072EAD0` close-transition drawing.

### 4. Yuri/default minimap movie reuses Soviet small asset at 640 but uses `MPY*` at non-640

Active in YR: Yes for side `2`; not Soviet.

The default/Yuri branch in `RadarTransitionMovie_SHPLoad @ 0x0072D830` uses:

| Side | Predicate | Filename |
|---|---|---|
| Yuri/default | `g_ScreenWidth == 0x280` | `MPSSCRNS.SHP` |
| Yuri/default | `g_ScreenWidth != 0x280` | `MPYSCRNL.SHP` |

Filename sources: `[0x00844CB0] -> "MPSSCRNS.SHP"` and
`[0x00844CB4] -> "MPYSCRNL.SHP"`.

### 5. Left-panel loader has no Soviet-specific branch; Soviet side uses non-Yuri generic names

Active in YR: Yes for shell/left-panel drawing.

`MIX_LoadNeutral @ 0x0072FA10` does not branch on Soviet side `1`. It reads
`ScenarioClass+0x34B8` and compares only against `2`:
`0x0072FA10..0x0072FA21` loads `g_ScenarioClass_Instance`, compares
`[EAX+0x34B8]` to `0x2`, and jumps to the non-Yuri branch for every side other
than Yuri.

For Soviet side `1`, the non-Yuri branch selects generic left-panel names,
including:

- `RADAR.SHP` via `[0x00844D14] -> 0x008450B0`
- `BKGDMD.SHP` via `[0x00844D04] -> 0x008450DC`
- `BKGDSM.SHP` via `[0x00844D00] -> 0x008450E8`
- `BKGDLG.SHP` via `[0x00844D08] -> 0x008450D0`
- `SIDEBTTN.SHP` via `[0x00844CFC] -> 0x008450F4`

The `640`/`800+` decision is not in the loader; it is in
`LeftPanel__ComputeLayoutRects @ 0x0072FC60` and `LeftPanel__Draw @ 0x0072F540`.
Both select the top-left panel background by width:

- `screen_width == 0x280`: use `g_BKGDSM_SHP` (`BKGDSM.SHP` for Soviet).
- `screen_width == 800`: use `g_BKGDMD_SHP` (`BKGDMD.SHP` for Soviet).
- otherwise: use `g_SIDEBTTN_SHP` (`SIDEBTTN.SHP` for Soviet/non-Yuri).

YR activity evidence: `LeftPanel__Draw @ 0x0072F540` calls `MIX_LoadNeutral`
when `DAT_00B0FC0C == 0`, then calls `LeftPanel__ComputeLayoutRects`, then
draws the loaded SHP globals. `WM_PAINT_Handler @ 0x00621F00` reaches
`LeftPanel__Draw` at call site `0x00621FD5` on the left-panel branch, so this is
a live standard YR UI draw path.

## Implementation Handoff

1. Verified behavior -> Soviet in-game radar background/open/close assets are
   explicit `SSCRBK*`, `SSCRT*`, and `SSCRA*` names selected by side `1` and
   exact-width `640` vs non-640.
   Rust delta -> `src/render/sidebar_chrome.rs` currently builds Soviet radar
   chrome from `radar.shp` in `sidec02.mix`.
   Affected surface -> radar housing/open/close chrome in game sidebar.
   Acceptance scenario -> Soviet local player at 640 loads `SSCRBKSM/SSCRTSM/SSCRASM`,
   at 800 loads `SSCRBKMD/SSCRTMD/SSCRAMD`, and does not require generic
   `radar.shp` for this path.
   Proposed test -> `test_soviet_radar_background_selector_uses_sscr_assets_by_width`.
   Risk -> HIGH screenshot parity risk whenever Soviet radar opens/closes or is
   visible.

2. Verified behavior -> Soviet minimap transition movie uses `MPSSCRNS.SHP` at
   640 and `MPSSCRNL.SHP` for non-640.
   Rust delta -> `src/render/radar_anim.rs` animates pre-rendered `radar.shp`
   frames; it does not model the separate native `MPSSCRN*` transition movie.
   Affected surface -> radar gain/loss transition frames.
   Acceptance scenario -> Soviet radar close/open transition selects
   `MPSSCRNS` at 640 and `MPSSCRNL` at 800+.
   Proposed test -> `test_soviet_minimap_transition_movie_uses_mpsscrn_by_width`.
   Risk -> HIGH for radar transition parity, medium for static online radar.

3. Verified behavior -> left-panel loader is Yuri-vs-non-Yuri only; Soviet side
   `1` uses generic `RADAR.SHP`, `BKGD*.SHP`, and shared strip assets, with
   width selection deferred to `LeftPanel__ComputeLayoutRects`.
   Rust delta -> do not create Soviet-specific `SS*` or `SY*` left-panel
   filenames for this loader; keep non-Yuri generic names for side `1`.
   Affected surface -> shell/left-panel UI surfaces, not ordinary in-game
   sidebar chrome from `FUN_006D02B0`.
   Acceptance scenario -> side `1` loads `RADAR/BKGDMD/BKGDSM/BKGDLG` and
   never `RADARY`/`BKGD*Y`; `640` selects `BKGDSM`, `800` selects `BKGDMD`.
   Proposed test -> `test_soviet_left_panel_loader_uses_non_yuri_generic_assets`.
   Risk -> MEDIUM shell/screen-edge parity risk.

## Negative Facts / Do Not Do

- Do not use generic `radar.shp` as the binary-proven Soviet in-game radar
  background/open/close selector for `FUN_0072D460`; side `1` selects explicit
  `SSCR*` filenames at `0x0072D4EF..0x0072D57E`.
- Do not use `SYCR*` for Soviet. `SYCR*` appears only in the side-not-0/1
  non-640 branch, not in side `1`.
- Do not use `MPY*` for Soviet minimap transitions. `MPYSCRNL.SHP` is in the
  default/Yuri non-640 branch of `0x0072D830`, not the side `1` branch.
- Do not add a Soviet-specific branch to `FUN_0072FA10` left-panel loading.
  The loader checks only `ScenarioClass+0x34B8 == 2`; side `1` falls through to
  generic non-Yuri names.
- Do not treat `800+` literally as `>=800` for the radar selectors. The binary
  predicate is `g_ScreenWidth == 0x280` for small; every other width uses the
  non-small branch. The left panel separately distinguishes `640`, `800`, and
  other widths.

## Remaining Uncertainty

- Exact retail MIX membership and resolver precedence for the `SSCR*`,
  `MPSSCRN*`, `RADAR`, and `BKGD*` files was not checked in this slot.
- Full palette/ConvertClass selection for these radar/left-panel SHPs was not
  traced here.
- The exact semantic names of every `DAT_00B0FA*` global were not finalized;
  this report relies on selector addresses plus filename pointer sources rather
  than decompiler-assigned global names.

## Stale-doc Wording

`C:/Users/enok/Documents/ra2-rust-game/docs/research/SIDEBAR_RADAR_POSITIONING.md`
left-panel table should replace the rows that claim `DAT_00b0fa68` is
`BKGDLG.SHP` / `BKGDLGY.SHP` with:

> `MIX_LoadNeutral @ 0x0072FA10` first loads `RADAR.SHP` for non-Yuri sides and
> `RADARY.SHP` for side `2`, then loads `BKGDMD(.Y)`, `BKGDSM(.Y)`, and
> `BKGDLG(.Y)` through delayed constructor-result stores. Do not rely on
> decompiler-assigned `g_BKGD*` names for these globals without following the
> store-after-next-setup pattern.

The existing radar-background and minimap-transition filename tables in that doc
are corroborated by this report.
