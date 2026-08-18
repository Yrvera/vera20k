# SSCRA Close Frame Draw Lifecycle - Ghidra Report

Status: COMPLETE

## Target Question

Prove direct consumers and draw lifecycle for loaded Soviet close/closing radar
frame assets `SSCRA*` (`SSCRASM.SHP` / `SSCRAMD.SHP`) and related close-frame
globals. Specifically: when are they drawn, with which frame index/rect/origin,
and how does this differ from `SSCRT*` open-frame and `MPSSCRN*` transition movie?

## Non-goals

- Do not re-run the full Soviet filename selector proof.
- Do not investigate tactical minimap terrain/content inset.
- Do not implement Rust changes.
- Do not update published research docs outside this report.

## Evidence Needed To Mark COMPLETE

- Direct xrefs to `g_RadarFrameClose_SHP @ 0x00B0FB30`.
- Draw or non-draw consumer proof for every material read of that global.
- Stronger evidence for any Rust-facing claim: xref/caller evidence plus
  decompile or assembly-context evidence.
- Standard YR liveness labels for selector, draw, and owner-draw paths.

## Stop Conditions

- Stop if Ghidra read-only access is unavailable.
- Stop if a function boundary would require mutating Ghidra to create.
- Stop if only decompiler prose supports handoff-critical draw details.
- Stop before expanding into minimap contents or unrelated shell dialogs.

## Verified Findings

### 1. `SSCRA*` loads into `g_RadarFrameClose_SHP`, but the normal right-panel draw functions do not read it

Active in YR: Yes for loading; No for direct right-panel draw in the checked
open/movie functions.

Fresh xrefs to `0x00B0FB30` show writes from `RadarBackground_SHPLoad @
0x0072D460` and only these reads:

- `FUN_0072D350 @ 0x0072D350`: teardown/free path.
- `FUN_00603870 @ 0x00603870`: owner-draw static SHP pointer provider.
- `0x0072D6BB`: second teardown/free path in an unlabelled nearby cleanup body.

No xref from `FUN_0072E9F0`, `FUN_0072EAD0`, `RadarBackground @ 0x0072E920`, or
`RightPanel__Draw @ 0x0072E450` reads `0x00B0FB30`. Assembly-context evidence:
`0x0072EA44` reads `0x00B0FB34` for the open frame; `0x0072EB24` reads
`0x00B0FB1C` for the `MP*SCRN*` movie.

### 2. Open-frame `SSCRT*`/`SSCRBK*` and transition movie `MPSSCRN*` are separate direct draw paths

Active in YR: Yes.

`FUN_0072E9F0 @ 0x0072E9F0` draws `g_RadarFrameOpen_SHP @ 0x00B0FB34` frame `0`
at `DAT_00B0FC1C.x/y`, after `Fill_Margins()` and `RightPanel__Draw(0)`.
Assembly context at `0x0072EA57` confirms the `CC_Draw_Shape` call reads
`0x00B0FB34`.

`FUN_0072EAD0 @ 0x0072EAD0` draws `g_MinimapMovie_SHP @ 0x00B0FB1C` frame `0`
at the same `DAT_00B0FC1C.x/y` origin. Assembly context at `0x0072EB37`
confirms the `CC_Draw_Shape` call reads `0x00B0FB1C`.

Neither path draws `g_RadarFrameClose_SHP @ 0x00B0FB30`.

### 3. The proven `SSCRA*` draw consumer is the owner-draw static kind-4 path for `(dialog 0x103/0xBC7, child 0x72B)`

Active in YR: Conditional. The infrastructure is active in standard YR shell
owner-draw setup; prior `FUN_0060CF00` research identifies `0x103` and `0xBC7`
as reachable in-game radar dialogs. The branch requires child control `0x72B`.

`FUN_0060A5B0 @ 0x0060A5B0` classifies statics. Assembly context around
`0x0060A944..0x0060A987` sets record kind `4` (`[ESI+0x70] = 4`) for
`(0x103, 0x72B)` and `(0xBC7, 0x72B)`. It then calls:

- `FUN_006035F0 @ 0x006035F0`, which selects the convert path through
  `FUN_0072D450`.
- `FUN_00603870 @ 0x00603870`, which returns `g_RadarFrameClose_SHP @
  0x00B0FB30` when parent dialog id is `0x103` or `0xBC7` and child control id is
  `0x72B`.

Assembly context at `0x0060A9B2` confirms the returned SHP pointer is stored to
the static record at `[ESI+0x78]`, with the convert/filename value stored at
`[ESI+0x74]`.

### 4. Owner-draw static paints `SSCRA*` as a centered kind-4 SHP animation, not at `DAT_00B0FC1C`

Active in YR: Conditional on the kind-4 static record being armed for child
`0x72B`.

`OwnerDraw_Static_006153E0 @ 0x006153E0` handles `WM_PAINT` for kind `4`. For
kind `3` or `4` with record `[0x1E] / +0x78` non-null, it:

- gets the child client rect and cached local surface rect;
- centers the SHP by comparing SHP header width `+2` and height `+4` against the
  available rect;
- reads current frame index from record `[0x26] / +0x98`;
- calls `CC_Draw_Shape(record[0x1E], record[0x26], centered_point, surface_rect,
  flags=0x400, z=1000, ...)`;
- if kind `4` is running, increments frame and wraps to `0` when it reaches
  record `[0x25] / +0x94`.

Assembly context at `0x006159EB` confirms the owner-draw `CC_Draw_Shape` call.
The source SHP comes from the record pointer installed by `FUN_0060A5B0`, not
from a direct global read at paint time. This means `SSCRA*` does not use
`DAT_00B0FC1C`, the right-panel origin, or the static-background `+80` branch.

### 5. `0x4D3` starts the `0x72B` kind-4 SHP animation; `0x4D5` can set an explicit frame

Active in YR: Yes for the send site; Conditional for visible `SSCRA*` animation
on the child record being the `(0x103/0xBC7, 0x72B)` owner-draw static.

`OwnerDraw_Static_006153E0` message `0x4D3` starts kind-4 SHP animation only when
record `[0x1E]` is non-null, kind is `4`, and the running byte is clear. It calls
`FUN_006033F0`, which returns `100` ms for `(0x103/0xBC7, 0x72B)`, then starts
timer id `0`.

Assembly context at `0x006C93C2` shows active code sending `0x4D3` to child
`0x72B` through the imported dialog-child message helper. The adjacent branch
sends `0x4D5` with frame `0x1D` to the same child, and the owner-draw static
message `0x4D5` stores the requested frame into record `[0x26] / +0x98` and
invalidates.

## Implementation Handoff

1. Verified behavior -> `SSCRA*` is not the direct right-panel close/movie draw;
the direct close-transition function draws `MPSSCRN*` frame `0`, while `SSCRA*`
is consumed through an owner-draw static record for `(0x103/0xBC7, 0x72B)`.
Rust delta -> separate side-specific right-panel `SSCR*`/`MPSSCRN*` shell
transition assets from in-game `radar.shp` chrome animation state. Affected
surface -> `src/render/radar_anim.rs`, `src/render/sidebar_chrome.rs`,
`src/app_render/build_instances.rs`. Acceptance scenario -> losing radar in
gameplay does not pretend `SSCRAMD` is the same frame sequence as
`MPSSCRNL` or generic `radar.shp`; shell/radar-dialog close static can use
`SSCRA*` independently. Proposed test name ->
`test_soviet_sscra_close_static_is_not_mp_transition_movie`. Risk -> HIGH
pixel/asset-lifecycle drift.

2. Verified behavior -> `SSCRA*` owner-draw placement is centered inside child
control `0x72B`'s client paint rect and never uses `DAT_00B0FC1C` or the
right-panel `+80` x-offset. Rust delta -> do not reuse right-panel radar origin
math for any future `SSCRA*` static/dialog animation path. Affected surface ->
future shell/radar-dialog renderer plus `src/sidebar/mod.rs` if shared helpers
are introduced. Acceptance scenario -> `SSCRAMD` frame placement for a dialog
static is computed from the child static rect and SHP dimensions, not from
`screen_w - 168`, `DAT_00B0FC1C`, or `+80`. Proposed test name ->
`test_sscra_ownerdraw_static_centers_in_child_rect_not_right_panel_origin`.
Risk -> MEDIUM-HIGH for shell/radar-dialog screenshot parity.

3. Verified behavior -> `0x4D3` starts the kind-4 animation and `0x4D5` sets
the current frame before invalidation; owner-draw paint advances and wraps by
record frame count. Rust delta -> if modeling this path, implement it as an
owner-draw/static animation protocol, not as a self-contained radar availability
state machine. Affected surface -> future shell owner-draw/static support,
possibly `src/render/radar_anim.rs` if reused. Acceptance scenario -> sending
start then explicit frame to child `0x72B` reproduces native current-frame
storage and wrap behavior. Proposed test name ->
`test_ownerdraw_static_0x72b_sscra_start_and_set_frame_protocol`. Risk -> MEDIUM.

## Negative Facts / Do Not Do

- Do not draw `SSCRA*` from `FUN_0072EAD0`; evidence: `0x0072EAD0` reads
  `0x00B0FB1C` (`g_MinimapMovie_SHP`), not `0x00B0FB30`.
- Do not call `MPSSCRN*` the same asset as `SSCRA*`; evidence: selector writes
  use separate globals `0x00B0FB1C` and `0x00B0FB30`, and draw xrefs are
  separate.
- Do not place `SSCRA*` at `DAT_00B0FC1C` or with the static background `+80`
  x-offset; evidence: the only paint path is owner-draw static centering at
  `0x006153E0`, while `DAT_00B0FC1C` consumers are `0x0072E9F0` and
  `0x0072EAD0`.
- Do not infer `SSCRA*` liveness from `RightPanel__Draw`; evidence: xrefs to
  `0x00B0FB30` show no `RightPanel__Draw` read.
- Do not collapse the owner-draw static protocol into Rust's current
  `RadarAnimState::set_has_radar`; evidence: native `0x4D3` / `0x4D5` messages
  and record fields drive the `SSCRA*` static animation path.

## Remaining Uncertainty

- Exact visual semantic name of dialog `0x103` versus `0xBC7` was not rederived
  in this slot; prior `FUN_0060CF00` report labels both as in-game radar dialogs.
- Retail `RT_DIALOG` geometry for child `0x72B` was not dumped here, so the exact
  screen-space rect is not listed in pixels.
- The actual tactical minimap content/inset remains outside this report.

## Focused Rust Scan

- `radar.shp` generic animation -> `src/render/radar_anim.rs`,
  `src/render/sidebar_chrome.rs`, `src/app_transitions.rs`,
  `src/app_render/build_instances.rs` -> existing tests only cover the generic
  `RadarAnimPhase` state machine -> likely ownership is render/sidebar chrome,
  with future shell/static support separate.
- Right-sidebar radar layout -> `src/sidebar/mod.rs`,
  `src/sidebar/layout_spec.rs`, `src/app_sidebar_build.rs` -> current model uses
  fixed sidebar/radar layout; no owner-draw static protocol.

## Stale Doc Wording

- `docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace the section title
  `FUN_0072ead0 -- Radar Frame Close Transition` with
  `FUN_0072EAD0 -- MP*SCRN* transition movie draw; this function does not draw
  g_RadarFrameClose_SHP / SSCRA*`.
- `docs/research/RADAR_CHROME_COMPOSITING.md`: replace any wording that says
  `radar.shp` alone models native Soviet open/close radar chrome with
  `Current Rust uses generic radar.shp, but native Soviet shell/radar paths use
  separate SSCRBK*/SSCRT*/SSCRA* and MPSSCRN* assets; SSCRA* is consumed by the
  owner-draw static path for radar dialog child 0x72B, not by FUN_0072EAD0.`

## Sources

- Ghidra MCP read-only: `get_xrefs_to 0x00B0FB30`.
- Ghidra MCP read-only: `get_bulk_xrefs 0x00B0FB34,0x00B0FB00,0x00B0FB30,0x00B0FB1C`.
- Ghidra MCP read-only: decompile `0x00603870`, `0x0060A5B0`,
  `0x006153E0`, `0x006035F0`, `0x006033F0`, `0x0072E9F0`, `0x0072EAD0`,
  `0x006C8E80`, `0x005C9720`.
- Ghidra MCP read-only: assembly context at `0x0060A982`, `0x0060A9B2`,
  `0x006159EB`, `0x006C93C2`, `0x0072EA57`, `0x0072EB37`.
- Existing local research: `FUN_0060CF00_DIALOG_BACKGROUND_POINTER_TABLE_GHIDRA_REPORT.md`,
  `SOVIET_RADAR_LEFT_PANEL_SHP_SELECTORS_FOLLOWUP_GHIDRA_REPORT.md`,
  `SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT.md`,
  `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`.
