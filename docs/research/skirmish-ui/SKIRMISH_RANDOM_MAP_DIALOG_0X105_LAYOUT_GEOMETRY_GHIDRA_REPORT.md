# Skirmish Random-Map Setup Dialog 0x105 — Layout / Geometry Research Report

> **Superseding background correction (2026-07-27):** The control geometry and
> the finding that WndProc `0x00596300` performs no dialog-background blit remain
> valid. The inference that the common parent surface is therefore identical to
> dialog `0x6B` is not valid. Fresh read-only decompilation of common background
> selector `0x0060CF00` shows no special `0x105` branch, so `0x105` takes the
> generic parent family: `MNSCRNS.SHP` at width 640 and `MNSCRNL.SHP` otherwise,
> converted through `SHELL.PAL`. Fresh decompilation of common initializer
> `0x00622820` also places `0x105` in the `data+0xD5` top-highlight set but not
> the `data+0xD6` minimap-button set. Therefore the same-background claims in
> the overview, sections 4, 9, and 10 are superseded; reuse the verified
> geometry, not `MnScrnLCustomizeBattle` artwork. Section 6 is a historical Rust
> snapshot and is also stale. Stronger evidence:
> `FUN_0060CF00_DIALOG_BACKGROUND_POINTER_TABLE_GHIDRA_REPORT.md` plus live
> `0x0060CF00`/`0x00622820` decompilation.

**Address(es):** RT_DIALOG resource id `0x105` (PE `.rsrc`, template at file offset `0x500028`, size 1344); WndProc `0x00596300`; sibling frame RT_DIALOG `0x6B` (template at file `0x4F26D8`).
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** the exact pixel/DLU geometry of the random-map setup dialog `0x105` — the per-control x/y/cx/cy in dialog units, control IDs, window classes/styles, and caption (CSF) keys; the dialog frame size; whether a custom background asset is painted; and the DLU→pixel handoff. This is the OQ-16 gap the behavior report (`SKIRMISH_RANDOM_MAP_SETUP_DIALOG_CONTROLS_OPTIONS`) left deferred.
**Non-Scope:** the dialog's *behavior* (control→`RmgOptions` field mapping, defaults, clamps, Randomize/Generate/OK/Cancel semantics) — that is fully covered by the two behavior reports and is NOT re-derived here (only cross-referenced). Runtime `GetDialogBaseUnits` capture and localized caption *text* (vs the CSF key) remain deferred.
**Confidence:** HIGH for geometry (extracted from the actual PE resource bytes and cross-validated against `0x6B`), HIGH for the no-custom-background finding (WndProc decompile). MEDIUM for the exact DLU→pixel scale at runtime (inherits the same open caveat as the port's `0x6B` handling).
**Active in YR:** Conditional — the dialog is created (`CreateDialogIndirectParamA` via `0x00622650` from `0x00595BC0`) when Choose Map command `0x583` fires in standard YR Skirmish and the mode allows random maps.

## 1. Overview

Dialog `0x105` is a standard Windows `DLGTEMPLATEEX` dialog (8pt MS Sans Serif) on the **same 533×369-DLU frame as the Choose Map dialog `0x6B`**, with the same font and the same background surface. It carries counterparts to `0x6B`'s Cancel/UseMap buttons, title/blank statics, and preview box — but **at its own x coordinates, 2–3 DLU from `0x6B`'s** (see the correction note at the end of §3; only the `0x695` blank actually coincides). The random-map dialog replaces `0x6B`'s two listboxes (`0x6EB` GameType, `0x553` GameMap) with the RMG option controls (five combos + a players trackbar + a seed edit + Randomize/Generate buttons) in the left column, and adds three saved-seed buttons to the right column. The WndProc paints **no custom background**; the frame/background is the shared shell dialog surface (already rendered by the port for `0x6B`), and the only WndProc-specific paint is drawing the generated preview into child `0x468`.

**Method (reusable):** the geometry was extracted by parsing the PE `RT_DIALOG` resource directly from `gamemd.exe` (a dependency-free `DLGTEMPLATEEX` parser). This is the same class of evidence the port's "verified `0x6B` geometry" rests on. The parser was validated by extracting `0x6B` and confirming its controls exactly match the port's known choose-map control IDs (`0x6C5` UseMap, `0x5C0` Cancel, `0x583` CreateRandomMap, listboxes `0x6EB`/`0x553`, preview `0x468`).

## 2. Frame Geometry

`RT_DIALOG 0x105` — `DLGTEMPLATEEX`, `style=0x40000040` (`DS_SETFONT | DS_3DLOOK`-ish; the low `0x40` is `DS_SETFONT`), `exStyle=0`, **x=0 y=0 cx=533 cy=369 DLUs**, font **8pt "MS Sans Serif"**, 25 controls. Identical frame dims + font to `RT_DIALOG 0x6B`.

**Stale-doc note:** `src/ui/skirmish_shell/layout.rs:42-43` states the choose-map modal was "derived from the 300x200-DLU template". The actual `0x6B` (and `0x105`) template is **533×369 DLUs**, not 300×200. Correct the comment (the pixel rects the port derives may already be right via the 800×600 resource-geometry tests; only the DLU figure in the comment is wrong).

## 3. Control Geometry (exact, from the resource)

DLU coordinates, verbatim from the `0x105` template. `id=-1` (0xFFFFFFFF) are static labels. `style` is the raw window style; `WS_VISIBLE = 0x10000000` — controls lacking it are hidden until the WndProc shows them (progress UI).

| ID | Class | x | y | cx | cy | style | Caption (CSF key) | Role / field (from behavior doc) |
|---|---|---:|---:|---:|---:|---|---|---|
| `0x694` | STATIC | 422 | 1 | 108 | 10 | `0x50020001` | `GUI:GenerateMap` | dialog title (top-right) |
| `0x407` | COMBOBOX | 179 | 90 | 150 | 103 | `0x50000213` | — | theater → `+0x38` |
| — | STATIC | 74 | 90 | 93 | 12 | `0x50000200` | `GUI:Theater` | label for `0x407` |
| `0x405` | COMBOBOX | 179 | 41 | 150 | 103 | `0x50000313` | — | map type/landform → `+0x3C` |
| — | STATIC | 74 | 40 | 93 | 14 | `0x50000200` | `GUI:Environment` | label for `0x405` |
| `0x406` | COMBOBOX | 179 | 114 | 150 | 103 | `0x50000213` | — | size → `+0x64` & `+0x68` |
| — | STATIC | 74 | 114 | 93 | 12 | `0x50000200` | `GUI:MapSize` | label for `0x406` |
| `0x408` | COMBOBOX | 179 | 138 | 150 | 103 | `0x50000213` | — | resources → `+0x40` |
| — | STATIC | 74 | 138 | 93 | 12 | `0x50000200` | `GUI:Resources` | label for `0x408` |
| `0x3EA` | COMBOBOX | 179 | 65 | 150 | 101 | `0x50000313` | — | time of day → `+0x48` |
| — | STATIC | 74 | 64 | 93 | 14 | `0x50000200` | `GUI:TimeOfDay` | label for `0x3EA` |
| `0x3EB` | msctls_trackbar32 | 179 | 163 | 150 | 13 | `0x50000004` | `Slider1` | **players trackbar** → `+0x50` |
| — | STATIC | 74 | 162 | 93 | 14 | `0x50000200` | `GUI:Players` | label for `0x3EB` |
| `0x3FB` | EDIT | 279 | 287 | 50 | 12 | `0x48002000` | — | seed → `+0x74` (disabled edit, no `WS_VISIBLE`) |
| `0x621` | BUTTON | 74 | 257 | 83 | 15 | `0x5000000b` | `GUI:SurpriseMe` | Randomize |
| `0x620` | BUTTON | 246 | 257 | 83 | 15 | `0x5000000b` | `GUI:PreviewMap` | Generate/preview |
| `0x6C5` | BUTTON | 422 | 122 | 108 | 23 | `0x5000000b` | `GUI:UseMap` | OK/accept → result 1 |
| `0x6C2` | BUTTON | 422 | 149 | 108 | 23 | `0x5000000b` | `GUI:LoadMap` | saved-seed load |
| `0x6C3` | BUTTON | 422 | 176 | 108 | 23 | `0x5000000b` | `GUI:SaveMap` | saved-seed save |
| `0x6C4` | BUTTON | 422 | 203 | 108 | 23 | `0x5000000b` | `GUI:DeleteMap` | saved-seed delete |
| `0x5C0` | BUTTON | 423 | 346 | 108 | 23 | `0x5000000b` | `GUI:Cancel` | Cancel → result 2 |
| `0x468` | STATIC | 430 | 23 | 96 | 69 | `0x50000004` | — | preview box (`DrawStartPositions` target) |
| `0x639` | BUTTON | 229 | 217 | 100 | 21 | `0x40000007` | — | **hidden** (no `WS_VISIBLE`); progress button |
| `0x638` | STATIC | 74 | 219 | 150 | 11 | `0x40000200` | `GUI:WorkingPleaseWait` | **hidden** progress text (shown during Generate) |
| `0x695` | STATIC | 2 | 355 | 303 | 12 | `0x50000200` | `GUI:Blank` | bottom status line |

Notes:
- **Two-column layout.** Left column: labels at x=74 (cx≈93) with their controls at x=179 (cx=150), rows at y = 41/65/90/114/138/163 for Environment / TimeOfDay / Theater / MapSize / Resources / Players. Below them Randomize (x=74) and Generate (x=246) at y=257, and the seed edit at (279,287). Right column: the stacked action buttons at x=422 (UseMap y=122, LoadMap 149, SaveMap 176, DeleteMap 203) and Cancel at (423,346); the preview box `0x468` at (430,23), 96×69.
- **`0x3EB` is a Windows trackbar** (`msctls_trackbar32`), NOT a spin/updown — the behavior doc's "player count spin/control" wording is imprecise. Players is a horizontal slider.
- **`0x405` label is `GUI:Environment`** (not "map type"). The item-data still writes `+0x3C` (map type/landform) per the behavior doc; the visible label is "Environment".
- **`0x3FB` seed edit lacks `WS_VISIBLE`** and is `WS_DISABLED` (`0x48002000` = `WS_CHILD|WS_DISABLED|ES_...`), consistent with the behavior doc's "display sync formats seed then sends a set-text message; typed commit not drained" — it is a display field, not a user-editable one in the visible flow.
- **Hidden progress controls:** `0x639` (button, `0x40000007`, no visible) and `0x638` ("Working Please Wait" static, no visible) are shown during the synchronous Generate block; `0x638`/`0x639` at (74,219)/(229,217).
- **Right-column chrome is NOT at the same coordinates as `0x6B`** (corrected 2026-07-21; an earlier revision of this bullet wrongly claimed it was). Fresh extraction of the `0x6B` template (file `0x4F26D8`, 11 controls, same 533×369 frame) versus `0x105`:

  | Control | `0x6B` x | `0x105` x | Δ DLU | Δ px @800 |
  |---|---:|---:|---:|---:|
  | `0x694` title | 425 | 422 | −3 | −5 |
  | `0x6C5` UseMap | 425 | 422 | −3 | −5 |
  | `0x5C0` Cancel | 425 | 423 | −2 | −3 |
  | `0x468` preview | 428 | 430 | +2 | +3 |
  | `0x695` blank | 2 | 2 | 0 | 0 |

  Only `0x695` matches. `y`/`cx`/`cy` are identical throughout; **only `x` differs**. The frame, font, and background are genuinely shared. Use the §3 values for `0x105` rather than `0x6B`'s, for data fidelity.

  **Whether that x difference is observable depends on the consumer.** In this port it is not: the right-column helpers (`right_anchor`, `snap_button_biased_truncate`, `back_rect`) are all panel-anchored and **discard the source rect's x**, reading only `y`/`w`/`h` — which are identical between the two dialogs. So inheriting `0x6B`'s right-column rects there yields byte-identical output. A consumer that positions from `x` directly *would* see a 3–5 px shift. (The port's `layout.rs` constants — `dlu_rect(425,122,108,23)` UseMap, `dlu_rect(428,23,96,69)` preview, `dlu_rect(425,1,108,10)` title — match `0x6B` exactly and are correct **for `0x6B`**.)
- `0x694`'s caption differs: `GUI:GenerateMap` vs `0x6B`'s `GUI:ChooseMap`. `0x6B`'s `0x583` CreateRandomMap button occupies (425,149) — the slot `0x105` gives to `0x6C2` LoadMap (422,149).

## 4. Background / paint (WndProc `0x00596300`)

`decompile_function 0x00596300`, `param_2 == 0xf` (WM_PAINT):
```
if (DAT_00abe154 != 0) {                 // a generated preview wrapper exists
    GetDlgItem(hDlg, 0x468);              // the preview child
    if (FUN_006067a0() == 0)             // suppression check
        DrawStartPositions(hDlg);        // draw preview + start markers
}
if (FUN_00643e60() != 0) FUN_00643ae0(-1,-1);  // display-chain redraw
ValidateRect(hDlg, 0);
```
**There is no custom background SHP/PCX blit in this WndProc.** The dialog is a real Windows `DLGTEMPLATEEX` dialog; its frame/background is the shared RA2 shell dialog surface drawn by the display chain (`g_DisplayChain`), identical to `0x6B`. The only dialog-specific paint is the preview draw into `0x468` (behavior owned by `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS`). Implication: the port renders `0x105` on the **same modal background it already draws for `0x6B`** — no new asset.

## 5. DLU → pixel

`0x105` shares `0x6B`'s exact frame (533×369 DLUs, 8pt MS Sans Serif). Therefore the port's existing `0x6B` DLU→pixel transform (whatever scale `compute_choose_map_modal_layout` / `compute_fixed_800_choose_map_modal_layout` applies) maps `0x105` controls with the **same math** — no new scaling is needed; only the control rect table differs. The precise runtime `GetDialogBaseUnits` value is not captured here (same caveat the port already carries for `0x6B` at `layout.rs:42-43`); the resource-geometry tests (`row_combo_rects_match_800x600_resource_geometry`) are the port's working reference for the pixel mapping.

## 6. Current Rust status

- App command `0x583` is log-only (`src/app.rs:1337`), and `add_random_map_sentinel` (`src/ui/skirmish_shell/state/choose_map.rs:160`) is defined but never called — so this dialog is not opened at all today.
- The choose-map modal frame/background, right-column buttons, title/blank statics, and preview box ARE already implemented for `0x6B` (`src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`) — the reusable base for `0x105`.
- Options model: `RmgOptions` already carries every field with `normalize()` = the `0x005975E0` clamps; `.SED` read/write done. Generator done and verified in-game.

## 7. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| `0x105` frame dims + font | verified | PE RT_DIALOG `0x105` template (file `0x500028`) | none |
| all 25 control rects/IDs/classes/captions | verified | same template; cross-validated vs `0x6B` | none |
| parser correctness | verified | `0x6B` extraction matches port's known control IDs | none |
| custom background paint | verified (none) | `decompile_function 0x00596300` WM_PAINT | none |
| preview draw target | verified | WM_PAINT draws `DrawStartPositions` on `0x468` | preview pixel content owned by sibling report |
| DLU→pixel scale | touched-not-exhausted | shares `0x6B` frame; port's `0x6B` transform applies | exact runtime `GetDialogBaseUnits` not captured (deferred) |
| localized caption text | deferred | CSF keys extracted; text is CSF-resolved at runtime | resolve `GUI:*` keys via the port's CSF loader at build time |
| saved-seed file UX (`0x6C2/0x6C3/0x6C4`) | deferred | geometry captured; behavior owned by behavior doc | full file-browser flow (separate feature) |

## 8. Open Questions — Final State

- `[RESOLVED] Q1 — exact per-control DLU geometry of 0x105?` → full table in §3 (evidence: PE RT_DIALOG `0x105` template).
- `[RESOLVED] Q2 — dialog frame size + font?` → 533×369 DLU, 8pt MS Sans Serif (evidence: template header).
- `[RESOLVED] Q3 — is there a custom background asset?` → no; WndProc WM_PAINT draws only the preview; frame is the shared shell surface (evidence: `decompile_function 0x00596300`).
- `[RESOLVED] Q4 — what draws the preview and where?` → `DrawStartPositions` into child `0x468` at (430,23) 96×69, gated on `DAT_00abe154 != 0` (evidence: WM_PAINT case).
- `[RESOLVED] Q5 — is 0x105 a unique frame or shared with 0x6B?` → shared 533×369 frame; same Cancel/UseMap/title/blank/preview positions (evidence: `0x6B` vs `0x105` templates).
- `[RESOLVED] Q6 — control class of the players control 0x3EB?` → `msctls_trackbar32` (slider), not a spin (evidence: template class string).
- `[RESOLVED] Q7 — which controls are hidden by default?` → `0x638`, `0x639` (no `WS_VISIBLE`), plus the disabled seed edit `0x3FB`; shown/used during Generate (evidence: template styles + WndProc `0x620`).
- `[RESOLVED] Q8 — how was 0x6B geometry obtained (method to reuse)?` → from the DLU dialog template resource; reproduced here with a PE `RT_DIALOG` parser (evidence: `layout.rs:42`, this report's parser).
- `[DEFERRED] Q9 — exact runtime DLU→pixel base units.` (category: bounded-cost-too-high; reason: shares `0x6B`'s frame so the port's existing transform applies; next-step: capture `GetDialogBaseUnits` at runtime if the `0x6B` pixel rects ever need re-derivation.)
- `[DEFERRED] Q10 — localized text behind each GUI:* CSF key.` (category: out-of-scope; reason: keys captured; the port resolves CSF at build time; next-step: map keys through the port's CSF loader when rendering labels.)
- `[DEFERRED] Q11 — full saved-seed file-browser UX for 0x6C2/0x6C3/0x6C4.` (category: out-of-scope; reason: a separate feature; the geometry is captured here.)

## 9. Visual/UI Composition Ledger

| Order | Source | Condition | Asset / content | Rect / anchor | Active for 0x105? | Role |
|---|---|---|---|---|---|---|
| 1 | shared shell dialog surface (display chain, same as `0x6B`) | always while modal | RA2 shell modal background (no unique SHP) | 533×369-DLU frame | yes | chrome/container |
| 2 | Windows dialog manager | always | standard controls (combos/buttons/trackbar/edit/statics) at §3 rects | per §3 | yes | content (controls) |
| 3 | WndProc `0x00596300` WM_PAINT → `DrawStartPositions` | `DAT_00abe154 != 0` (a preview generated) | generated map preview + start markers | child `0x468` (430,23) 96×69 | conditional | content/preview |
| 4 | WndProc, Generate `0x620` | during synchronous Generate | `0x638` "Working Please Wait" + `0x639` shown | (74,219)/(229,217) | conditional | overlay (progress) |

Asset role matrix: the only *asset* drawn dialog-specifically is the **map preview** (content/preview, child `0x468`). Everything else is standard Windows controls over the shared shell background (chrome). No unique SHP/PCX is loaded for this dialog's frame.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x105` is the same 533×369-DLU frame/font/background as `0x6B`, but its right-column controls sit 2–3 DLU from `0x6B`'s | PE templates `0x105`/`0x6B` | missing (`0x583` log-only) | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs` | Reuse the `0x6B` modal frame + background asset; give `0x105` its **own** rect table for BOTH columns from §3 | Opening Create Random Map shows the same frame as Choose Map with RMG option controls where the two listboxes were, and the right column at the §3 x values | Do NOT invent a new background asset; do NOT inherit `0x6B`'s right-column rects — they are 2–3 DLU off (3–5 px at 800) |
| 25 controls with the exact DLU rects/IDs/classes/captions in §3 | PE `0x105` template | missing | new `0x105` layout table + hit-test | Lay out the five combos (theater/environment/size/resources/time), the players trackbar `0x3EB`, the seed edit `0x3FB`, Randomize/Generate/OK/Load/Save/Delete/Cancel, preview `0x468`, at the §3 positions scaled by the port's `0x6B` DLU→pixel transform | Control rects match a resource-geometry test analogous to `row_combo_rects_match_800x600_resource_geometry` for `0x105` | Do NOT approximate positions; use the §3 DLU values verbatim |
| Players is a trackbar (2..8), not a spin | `0x3EB` class `msctls_trackbar32`; clamp `0x005975E0` | missing | control widget choice | Render `0x3EB` as a horizontal slider bound to `num_players` (2..8) | The players control is a slider, not up/down arrows | Do NOT model it as a numeric spin |
| WM_PAINT draws no dialog background; only the preview into `0x468` | `decompile_function 0x00596300` | n/a | render layer | Do not paint a per-dialog background; draw the generated preview into the `0x468` rect when a preview exists | With no preview generated, the box is empty over the shared shell background | Do NOT treat the preview as terrain; it is a UI image |
| Hidden progress controls `0x638`/`0x639` appear during Generate | template styles + WndProc `0x620` | missing | Generate/blocking state | Show a "Working / Please Wait" indicator during the (synchronous) generate-preview action; disable all interactive controls incl. Cancel while it runs | Pressing Generate shows the wait UI and re-enables controls after | Do NOT allow cancel/mutation mid-generate |

**Stale-doc follow-up:** correct `src/ui/skirmish_shell/layout.rs:42-43` — the choose-map template is 533×369 DLUs, not 300×200.

## Sources

- PE `RT_DIALOG` templates parsed from `gamemd.exe` (`.rsrc`): id `0x105` (file `0x500028`, 1344 bytes) and id `0x6B` (file `0x4F26D8`, 636 bytes) via a dependency-free `DLGTEMPLATEEX` parser. `0x6B` extraction cross-validated against the port's known choose-map control IDs.
- Ghidra: `decompile_function 0x00596300` (WndProc WM_PAINT / commands `0x497`/`0x620`/`0x621`/`0x6C5`/`0x5C0`).
- Behavior reports cross-referenced (NOT re-derived): `SKIRMISH_RANDOM_MAP_SETUP_DIALOG_CONTROLS_OPTIONS_GHIDRA_REPORT.md`, `SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`, `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`.
- Rust scan: `src/app.rs:1337`, `src/ui/skirmish_shell/state/choose_map.rs:160`, `src/ui/skirmish_shell/layout.rs:42`.
