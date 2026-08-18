# Skirmish ResizeShellChildControl 0x102 Policy - Ghidra Research Report

**Address(es):** `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`, `FUN_00608CD0 @ 0x00608CD0`, `FUN_00609730 @ 0x00609730`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060B1D0 @ 0x0060B1D0`, `FUN_0060B350 @ 0x0060B350`, `FUN_0060B550 @ 0x0060B550`, `FUN_0060B950 @ 0x0060B950`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** complete child-control anchoring/fallback policy for standard offline YR Skirmish dialog `0x102` during the `ResizeShellChildControl_0060C0C0` enumeration pass, including right-anchor, Back/bottom anchor, one-pixel fixups, and preserve-rect fallback.
**Non-Scope:** map preview decode, PreviewPack, Back-button asset dimensions beyond cross-checking already resolved `SDBTNANM.SHP=156x42`, Choose Map modal internals, start-session packing, and runtime screenshot capture.
**Confidence:** High for static binary branch policy and formulas; Medium-high for Start/Choose final rects because the static branch evidence is direct but this slot did not attach a runtime breakpoint.
**Active in YR:** Yes. Evidence: `FUN_006AE2C0 @ 0x006AE2C0` creates/pumps standard offline dialog `0x102`; `FUN_00622B50 @ 0x00622B50` / `FUN_0060C4A0 @ 0x0060C4A0` resize the shell parent and enumerate children through `0x0060C0C0`. No TS-only gate is present on this path.

## 1. Overview

The `0x0060C0C0` callback is not a scaler. It is a branch dispatcher over the child HWND, its parent dialog id, the child control id, and the child owner-draw metadata class field at record `+0x68`.

For dialog `0x102`, the final policy is:

- selected right-panel text/static/preview controls move by right-anchor formulas;
- owner-draw Start and Choose buttons use the earlier PCX-button snap helper, not the generic static right-anchor helper;
- Back uses the bottom/right button helper already resolved in prior docs;
- tooltip/status static `0x695` bottom-left anchors;
- a small set of ordinary controls get one-pixel fixups;
- every other child preserves its existing Win32-created pixel rectangle relative to the resized parent.

## 2. Class Layout / Key Offsets

| Field / global | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `DAT_00AC48A8` | current shell parent HWND; child is skipped unless `GetParent(child) == DAT_00AC48A8` | Yes | `0x0060C0C0`, set by `0x0060C4A0` before `EnumChildWindows` |
| owner-draw record `+0x68` | class/type field set by `FUN_0060F9A0`; `0` for owner-draw Button style branch, `2` for Static, `3` for ComboBox, `7` for Trackbar, etc. | Yes | `FUN_0060F9A0 @ 0x0060F9A0` writes `piVar14[0x1A]`; branch tests `[record+0x68]` at `0x0060C1B0` and `0x0060C213` |
| parent record `+0x6C` | shell dialog id, `0x102` for standard offline Skirmish | Yes | `FUN_00622820`, `FUN_00622B50`, `0x0060C0C0` parent lookup |
| child record `+0xE0` | optional right-anchor inset override for `FUN_0060B1D0`; zero for scoped 0x102 child statics in prior follow-up | Yes | `FUN_0060B1D0`; `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md` |
| `DAT_007F5BE4 / DAT_007F5BF0` | 800x600 base shell dimensions used by centering offsets | Yes | helpers `0x0060B000`, `0x0060B1D0`, `0x0060B550` |
| `DAT_00B0FC24 / DAT_00B0FC28` | right-panel tile and bottom-cap rect globals used by button snap and Back y | Yes | `0x0060B000`, `0x0060B350`; prior Back/right-panel reports |

## 3. Core Logic

### 3.1 Dispatcher branch order

`ResizeShellChildControl_0060C0C0` tests branches in this order. First match wins, and every taken branch calls `FUN_0060B950` before returning.

| Order | Predicate for `0x102` | Helper | Effect | Active in YR |
|---:|---|---|---|---|
| 1 | `FUN_00608500` absolute-rect table | `FUN_0060AF50` | hardcoded absolute rect plus center offset | No for `0x102`; `FUN_00608500` has no parent-id `0x102` case |
| 2 | Button-style child, child record `+0x68 == 0`, and `FUN_00608CD0(parent, child)` true | `FUN_0060B000` | right-anchored PCX button, width/height from `SDBTNANM`, y snapped to nearest right-panel tile row | Yes for Start `0x617` and Choose `0x5AA` |
| 3 | `FUN_00608CD0(parent, child)` true | `FUN_0060B1D0` | right-anchor preserving current child size; y gains centered-shell vertical offset | Yes for `0x694`, `0x468`, `0x6EC`, `0x5A8` |
| 4 | Button-style child, child record `+0x68 == 0`, and `FUN_00609730(parent, child)` true | `FUN_0060B350` | Back-button bottom/right anchor, size from `SDBTNANM` | Yes for Back `0x5C0` |
| 5 | `FUN_00609730(parent, child)` true | `FUN_0060B420` | generic bottom/right anchor | No for `0x102`; only `0x5C0` matches and branch 4 preempts it |
| 6 | `FUN_00601360(parent_id)` true and child id `0x695` | `FUN_0060B550` | bottom-left status/tooltip anchor | Yes for `0x695` |
| 7 | parent id `0xE2` and child id `0x71D` | `FUN_0060B610` | main-menu version-line anchor | No for `0x102` |
| 8 | parent id not in the late-center list | inline `MoveWindow` | preserve child rect relative to parent | Yes for all remaining `0x102` children |
| 9 | parent id in late-center list | `FUN_0060B7A0` | alternate widescreen centering | No for `0x102`; `0x102` is not in the late-center list |

### 3.2 Complete `0x102` right-anchor allowlist

`FUN_00608CD0(parent, child)` returns true for these `0x102` controls:

| Control | Helper actually reached | Why | Active in YR |
|---:|---|---|---|
| `0x617` Start Game | `FUN_0060B000` | Button style `(style & 0x0B) == 0x0B`, child record `+0x68 == 0`, then `FUN_00608CD0` true | Yes |
| `0x5AA` Choose Map | `FUN_0060B000` | same owner-draw Button preemption as Start | Yes |
| `0x694` Skirmish title | `FUN_0060B1D0`, then `FUN_0060B950` y+1 | Static child, not Button branch | Yes |
| `0x468` map preview placeholder | `FUN_0060B1D0` | Static child, preview allowlist branch | Yes |
| `0x6EC` game-type text | `FUN_0060B1D0` | Static child, explicit `0x102` case | Yes |
| `0x5A8` map/scenario label | `FUN_0060B1D0` | Static child, explicit `0x102` case | Yes |

Material correction: the binary's branch order means Start and Choose are not plain `FUN_0060B1D0` users in standard setup. The assembly at `0x0060C1A9..0x0060C1C8` adds `4` to the hash node to obtain the child record, checks `[record+0x68] == 0`, calls `FUN_00608CD0`, then calls `FUN_0060B000`. `FUN_0060F9A0` assigns owner-draw Button controls `+0x68 = 0`.

### 3.3 PCX-button snap helper for Start and Choose

`FUN_0060B000` computes normal-shell button placement:

- `x = parent_width - max(0, (parent_width - 800) / 2) - 156`;
- `width = SDBTNANM.SHP.width = 156`;
- `height = SDBTNANM.SHP.height = 42`;
- `y` snaps the original child top to the nearest `DAT_00B0FC24` tile row using row height `42`;
- ties choose/start to the right-panel button grid, not to the resource width `162`.

Formula results using the already verified right-panel globals:

| Control | 640x480 | 800x600 | 1024x768 | Active in YR |
|---:|---:|---:|---:|---|
| `0x617` Start | `(484,241,156,42)` | `(644,241,156,42)` | `(756,325,156,42)` | Yes |
| `0x5AA` Choose Map | `(484,283,156,42)` | `(644,283,156,42)` | `(756,367,156,42)` | Yes |

The `-1`/`-3` y differences versus resource positions are not arbitrary nudges; they fall out of nearest-row snapping against `SDBTNBKGD` tile rows.

### 3.4 Generic right-anchor helper for statics/preview

`FUN_0060B1D0` computes:

- `offset_x = max(0, (parent_width - 800) / 2)`;
- `offset_y = max(0, (parent_height - 600) / 2) - max(0, (lparam_height - 600) / 2)`;
- default inset `= (168 - child_width) / 2` unless child record `+0xE0` is nonzero;
- `x = parent_width - offset_x - child_width - inset`;
- `y = original_child_y + offset_y`;
- width/height are preserved.

For standard `0x102`, `FUN_00622B50`/`FUN_0060C4A0` pass the `{640,480}` pair, so the lparam vertical term clamps to zero. Active in YR: Yes for `0x694`, `0x468`, `0x6EC`, and `0x5A8`.

## 4. INI Keys

No Skirmish-specific INI key affects this child-layout policy. The path uses the current video dimensions already in globals (`g_ScreenWidth`, `g_ScreenHeight`) and the shell/right-panel assets/globals. Active in YR: Yes as binary layout code; conditional only on the current video mode.

## 5. Integration Points

| Integration | Status | Active in YR | Evidence |
|---|---|---|---|
| Offline Skirmish dialog creation | verified | Yes | `FUN_006AE2C0` creates dialog `0x102` and pumps until `0x617`/`0x5C0` |
| Common shell init | verified | Yes | `FUN_00622B50` `WM_INITDIALOG` path enumerates children through `FUN_0060F9A0`, then full-screen resize path |
| Parent resize before child policy | verified | Yes | `FUN_0060C4A0`: `MoveWindow(parent,0,0,g_ScreenWidth,g_ScreenHeight,0)`, then `EnumChildWindows(...,0x0060C0C0,...)` |
| Owner-draw metadata class field | verified | Yes | `FUN_0060F9A0` class/style dispatch sets record `+0x68`; `0x0060C0C0` uses it to split button helpers from generic helpers |
| Hit testing implication | verified as Rust-facing | Yes | binary moves actual child HWNDs; Win32 hit tests use final HWND rects, so Rust hit rectangles must follow final helper rects |

## 6. Current Rust Implementation Status

Current Rust has the right high-level model (full-screen parent, selective right-panel controls, no global scale) but misses parts of the complete policy:

- `src/ui/skirmish_shell/layout.rs:105..115` implements one generic `right_anchor` and currently applies it to Start and Choose at `layout.rs:191..192`; binary routes those two owner-draw buttons through `FUN_0060B000`.
- `layout.rs:195` leaves `player_name` at raw DLU coordinates; binary applies `FUN_0060B950` to move `0x6A0` x by `+1` and width by `+1`.
- `layout.rs` does not expose `0x694`, `0x6EC`, `0x5A8`, `0x695`, checkboxes, or trackbars, so their verified anchoring/fixup policy is not yet representable.
- `src/ui/skirmish_shell/state.rs:105..119` and `state.rs:122..153` hit-test using the layout rectangles. Once Start/Choose layout is corrected, hit testing should follow the same actual child HWND rectangles, including exclusive right/bottom edges from `RectPx::contains`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ResizeShellChildControl_0060C0C0` entry guard | verified | `0x0060C0C0`; parent must equal `DAT_00AC48A8` | none |
| `FUN_00608500` absolute rect branch | verified | decompile `0x00608500`; no `0x102` parent case | none for `0x102` |
| Button-style right branch | verified | assembly `0x0060C1A9..0x0060C1C8`; `FUN_0060F9A0` Button `+0x68=0` | runtime breakpoint optional only |
| Generic right branch | verified | `0x00608CD0`, `0x0060B1D0` | none |
| Button-style Back branch | verified | assembly `0x0060C1F3..0x0060C227`; `FUN_00609730` `0x102/0x5C0` | none |
| Generic bottom branch | verified | `0x00609730`, `0x0060B420`; preempted for `0x5C0` | none for `0x102` |
| `0x695` bottom-left branch | verified | `FUN_00601360` includes `0x102`; `0x0060C2B6..0x0060C2D0`; `FUN_0060B550` | none |
| `0xE2/0x71D` branch | verified | `0x0060C324..0x0060C33A`; parent id compare is exact `0xE2` | none |
| Preserve-rect fallback | verified | `0x0060C396` onward; `0x102` not in late-center list | none |
| One-pixel fixups | verified | `FUN_0060B950` `0x102` cases | none |
| Rust layout comparison | verified | `src/ui/skirmish_shell/layout.rs:105..199`; tests `:205..270` | no Rust modified in this slot |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ1 - Is 0x0060C0C0 on the standard offline YR Skirmish path? -> Yes, through FUN_006AE2C0 -> FUN_00622B50/FUN_0060C4A0 -> EnumChildWindows.` (evidence: `0x006AE2C0`, `0x00622B50`, `0x0060C4A0`)
- `[RESOLVED] OQ2 - Does any absolute-rect table case apply to parent 0x102? -> No.` (evidence: `FUN_00608500 @ 0x00608500`)
- `[RESOLVED] OQ3 - Which 0x102 controls does FUN_00608CD0 select? -> 0x617, 0x5AA, 0x694, 0x468, 0x6EC, 0x5A8.` (evidence: `FUN_00608CD0 @ 0x00608CD0`)
- `[RESOLVED] OQ4 - Do Start/Choose take the same helper as right-anchored statics? -> No; Button metadata +0x68==0 preempts to FUN_0060B000.` (evidence: `0x0060C1A9..0x0060C1C8`, `FUN_0060F9A0`)
- `[RESOLVED] OQ5 - Which 0x102 control does FUN_00609730 select? -> Back 0x5C0 only.` (evidence: `FUN_00609730 @ 0x00609730`)
- `[RESOLVED] OQ6 - Does 0x695 anchor in 0x102? -> Yes; FUN_00601360 accepts parent 0x102 and the dispatcher routes ctrl 0x695 to FUN_0060B550.` (evidence: `FUN_00601360`, `0x0060C2B6..0x0060C2D0`)
- `[RESOLVED] OQ7 - Is 0x102 in the late-center list for FUN_0060B7A0? -> No; unmatched children use preserve-rect fallback.` (evidence: final predicate in `0x0060C0C0`)
- `[RESOLVED] OQ8 - What one-pixel 0x102 fixups exist? -> 0x694 y+1, 0x50C y-1, 0x54E/0x693/0x696/0x69A x-1, 0x6A0 x+1 and w+1.` (evidence: `FUN_0060B950 @ 0x0060B950`)
- `[RESOLVED] OQ9 - Are ordinary color combos and flags scaled/recentered? -> No; they are not in the special lists and fall through to preserve-rect behavior.` (evidence: `0x00608CD0`, `0x00609730`, fallback in `0x0060C0C0`)
- `[RESOLVED] OQ10 - Are INI keys involved? -> No direct INI reads in this path; only video dimension globals and loaded shell assets are consumed.` (evidence: helper decompilations, prior origin docs)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Start `0x617` and Choose `0x5AA` route through `FUN_0060B000`, not generic `FUN_0060B1D0`; final 800 rects are `(644,241,156,42)` and `(644,283,156,42)` by static formula | `0x0060C1A9..0x0060C1C8`, `FUN_0060B000`, `FUN_0060F9A0` | mismatch: Rust uses `right_anchor` and tests `(635,242,162,37)` / `(635,286,162,37)` | `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs` | model owner-draw button snap rect separately from static right-anchor rect; hit-test the actual final HWND rect | At 800x600, clicking x=641,y=242 should not hit Start; x=644,y=241 should hit Start; proposed test `skirmish_start_choose_buttons_use_pcx_snap_rects_800` | Do not reuse resource `162x37` rect for owner-draw button hit boxes |
| Static/preview right-anchor allowlist is exactly `0x694`, `0x468`, `0x6EC`, `0x5A8` for generic `FUN_0060B1D0`; title then gets y+1 in `FUN_0060B950` | `FUN_00608CD0`, `FUN_0060B1D0`, `FUN_0060B950` | partial: Rust has map preview but not title/game-type/map label surfaces | `src/ui/skirmish_shell/layout.rs`, render text surfaces | add explicit layout slots when those controls become visible; preserve sizes and use `(168-w)/2` inset, not button snap | At 1024x768, map preview remains `(756,121,144,112)` while game-type/map-label statics move with the same right panel offset; proposed test `skirmish_right_panel_static_controls_anchor_without_scaling` | Do not globally move all statics; only the allowlisted ids right-anchor |
| Fallback preserves ordinary child rects, but `FUN_0060B950` still applies 0x102 one-pixel fixups to specific ordinary controls | `0x0060C396` fallback, `FUN_0060B950` | partial/missing: player name lacks `+1/+1w`; checkboxes/trackbars not modeled | `src/ui/skirmish_shell/layout.rs`, future checkbox/trackbar hit testing | keep ordinary controls at DLU pixel positions except documented fixups: `0x6A0 x+1,w+1`, `0x50C y-1`, checkbox x-1 set, title y+1 | At 800x600, player-name edit should be raw DLU `(57,59,150,23)` adjusted to `(58,59,151,23)`; proposed test `skirmish_fallback_controls_apply_only_binary_one_pixel_fixups` | Do not scale or center the player table, flags, combos, checkboxes, or trackbars in high-res modes |

### Negative Facts / Do Not Do

- Do not implement a uniform 800x600 centered child transform for dialog `0x102`. Active in YR: No. Evidence: fallback branch in `0x0060C0C0` preserves current child rect for `0x102`.
- Do not treat Start/Choose as generic `FUN_0060B1D0` static right-anchor controls. Active in YR: No for standard Button metadata. Evidence: `0x0060C1A9..0x0060C1C8` preempts Button `+0x68==0` to `FUN_0060B000`.
- Do not resize ordinary controls to fill or follow the right panel. Active in YR: No. Evidence: `FUN_00608CD0` allowlist omits color combos, flags, checkboxes, sliders, and player row controls; fallback preserves them.
- Do not apply main-menu `0xE2/0x71D` version-line bottom-right policy to Skirmish. Active in YR: No for `0x102`. Evidence: dispatcher compare is exact parent id `0xE2` before `FUN_0060B610`.
- Do not ignore `FUN_0060B950` after fallback. Active in YR: Yes. Evidence: every dispatcher branch, including fallback, calls `FUN_0060B950`.

### Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md` replacement wording for Start/Choose rows: "Start `0x617` and Choose Map `0x5AA` are selected by `FUN_00608CD0`, but because they are owner-draw Button controls with record `+0x68 == 0`, `ResizeShellChildControl_0060C0C0` routes them through `FUN_0060B000`; the static formula gives 800x600 rects `(644,241,156,42)` and `(644,283,156,42)`, pending live screenshot confirmation."
- `docs/research/skirmish-ui/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md` replacement wording for Start/Choose rows: "The child `+0xE0` default-inset finding remains relevant to generic right-anchor statics such as `0x468`, but Start/Choose are preempted by the owner-draw Button branch and should not be listed as `FUN_0060B1D0` final-rect users."
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` replacement wording for final active rect table: "Back remains `(644,535,156,42)` at 800x600; Start/Choose require the `FUN_0060B000` grid-snap rects from `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md` rather than the prior generic-right-anchor rects."

## Sources

- Ghidra read-only decompilation/assembly: `0x006AE2C0`, `0x00622650`, `0x00622820`, `0x00622B50`, `0x0060C4A0`, `0x0060C0C0`, `0x00608500`, `0x00608CD0`, `0x00609730`, `0x00601360`, `0x0060AF50`, `0x0060B000`, `0x0060B1D0`, `0x0060B350`, `0x0060B420`, `0x0060B550`, `0x0060B610`, `0x0060B7A0`, `0x0060B950`, `0x0060F9A0`, `0x00612B70`.
- Prior docs cross-checked: `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `STATIC_ANIMATION_CLASSIFIER_REACHABILITY_ON_0X102_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`.
