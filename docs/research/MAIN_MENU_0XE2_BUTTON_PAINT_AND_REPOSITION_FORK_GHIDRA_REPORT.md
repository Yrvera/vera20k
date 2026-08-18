# Main Menu 0xE2 Button Paint Asset & Reposition Fork — Ghidra Research Report

**Address(es):** `OwnerDraw_Button_00612B70 @ 0x00612B70` (paint), `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0` (dispatch), `FUN_0060B000 @ 0x0060B000` (stack resize helper), `FUN_00609730 @ 0x00609730` (Exit predicate), `FUN_0060B350 @ 0x0060B350` (Exit resize helper), `FUN_00608CD0 @ 0x00608CD0` (first gate predicate), `FUN_0060F9A0 @ 0x0060F9A0` (subclass/record init)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** For dialog `0xE2` (standard YR main menu): (1) which reposition helper the six right-side buttons take (`FUN_0060B000` vs `FUN_0060B1D0` vs fallback), (2) the resulting window size/position of each button, (3) which paint-asset branch (`bue/bde` PCX type 0 vs `SDBTNANM.SHP` type 1) the buttons render through.
**Non-Scope:** Hover/focus rule, the SDBTNANM frame-10 panel overlay, non-`0xE2` shell dialogs, and the alternate nonstandard shell mode selected when `FUN_0069BBE0 != 0`.
**Confidence:** High for all six normal-mode button rectangles and the owner-draw pressed composition after the 2026-07-18 correction pass; High (by geometric coherence) for the SDBTNANM paint conclusion; the direct record-field setter for the paint type was not located statically (see Open Questions).
**Active in YR:** Yes — standard initial main-menu dialog `0xE2`, no TS gate.

## 1. Overview

All six main-menu buttons use a 156×42 `SDBTNANM.SHP` window in normal shell mode.
Single Player `0x683`, WW Online `0x684`, Network `0x578`, Movies/Credits `0x686`, and
Options `0x55C` are grid-snapped into the right-panel column by `FUN_0060B000`. Exit
`0x3EE` bypasses that first predicate but is then recognized by `FUN_00609730` and routed
to `FUN_0060B350`, which places it in the final complete 42-pixel tile immediately above
the bottom cap. The button artwork is the `SDBTNANM.SHP` animation (frames 2/3/4), not
the `bue_*30/bde_*30` PCX pieces.

> **2026-07-18 correction:** The earlier version of this report stopped after
> `FUN_00608CD0` returned false for Exit and missed the following
> `FUN_00609730 -> FUN_0060B350` branch. The raw 162×37 Exit conclusion is superseded.
> Evidence: `decompile_function(0x0060C0C0)`, `decompile_function(0x00609730)`, and
> `decompile_function(0x0060B350)` against the active YR binary.

## 2. Class Layout / Key Offsets

Owner-draw record: `operator_new(0x208)` (520 bytes), HWND at byte 0, hash-chain next at
`record[0x81]` (byte 0x204). Handlers reach the record as `rec = node + 1` (node base +4),
so "`rec[N]`" = node byte `(N+1)*4`.

| Field (rec-relative) | Byte (rec base) | Byte (node base) | Meaning | Evidence |
|---|---:|---:|---|---|
| `rec[0x1A]` | `0x68` | `0x6C` | Control-kind code (0=button, 1=edit, 2=static, 3=combo, 4=list, 6/7/8=progress/track/scroll, …). **This is the `piVar1[0x1b]` gate read in `ResizeShellChildControl`.** | `decompile_function 0x0060F9A0` (`piVar14[0x1a] = local_ab0`) |
| `rec[0x2C]` | `0xB0` | `0xB4` | **Paint-asset type**: 0=PCX `bue/bde`, 1=`g_SDBTNANM_SHP`, 2=`DAT_00B0F9EC`, 3=`DAT_00B0FACC` | `decompile_function 0x00612B70` (`iVar14 = piVar17[0x2c]`) |
| `rec[4]` | `0x10` | `0x14` | Cached offscreen `BSurface` (built in PCX path) | `decompile_function 0x00612B70` |
| `rec[5]`, `rec[6]` | `0x14`,`0x18` | `0x18`,`0x1C` | Cached up/down art surfaces (PCX path only) | `decompile_function 0x00612B70` |
| `rec+0xC5` | `0xC5` | `0xC9` | Hover/focus highlight flag (toggled on WM_SETFOCUS) | `decompile_function 0x00612B70` |

## 3. Core Logic

### 3.1 `FUN_00608CD0` — the shell-button predicate (decompiled this session, `0x00608CD0`)

Pure predicate `(parent_dialog_id, control_id) -> bool`. For parent `0xE2` it returns:

| Control | Result |
|---|---|
| `0x683` SP, `0x684` WWOnline, `0x578` Network, `0x686` Movies, `0x55C` Options | **true** |
| `0x55F` (website; not present in `0xE2` template), `0x694` heading, `0x71C` static | **true** |
| `0x3EE` Exit | **false** |

The `iVar4 == 0xE2` block returns true only for `{0x686,0x578,0x55C,0x683,0x55F,0x684}`;
`0x3EE` is absent and falls through to `return false`. (`0x694`/`0x71C` are caught by
earlier blocks in the same function.) Evidence: `decompile_function 0x00608CD0`.

### 3.2 Dispatcher routing in `ResizeShellChildControl_0060C0C0` (`0x0060C0C0`)

Per child, in order:

1. `FUN_00608500` → for `0xE2` returns 0 (its dialog-id checks are `0x94/0x103/0xBC/0xBD/0xC2/0xC9/0xBC7` only). Skip absolute-rect placer `FUN_0060AF50`. Evidence: `decompile_function 0x00608500`.
2. **Button-style branch** (`style & 0x0B == 0x0B`, true for all six buttons): if the control record is found AND `rec-kind (piVar1[0x1b]) == 0` AND `FUN_00608CD0() != 0` → **`FUN_0060B000`**, return.
   - Plain buttons have kind 0 (`FUN_0060F9A0` sets `local_ab0 = 0` for the `(style&0xB)==0xB` branch), so the gate is satisfied.
   - The 5 non-Exit buttons → `FUN_0060B000`. Exit (`0x3EE`) makes this first predicate false, so dispatch continues.
3. The following button-special predicate `FUN_00609730(parent_id, control_id)` returns true for exactly parent `0xE2`, control `0x3EE`; dispatch calls **`FUN_0060B350`** and returns. Evidence: `decompile_function(0x0060C0C0)` and `decompile_function(0x00609730)`.
4. Standalone `FUN_00608CD0()` true → `FUN_0060B1D0` (sidebar-inset). Catches statics `0x694` (title) and `0x71C`.
5. `FUN_00601360 && id==0x695` → `FUN_0060B550` (tooltip/status static).
6. `parent==0xE2 && id==0x71D` → `FUN_0060B610` (version line).
7. The raw fallback exists, but Exit does **not** reach it in standard YR.

### 3.3 `FUN_0060B000` — SDBTNANM-cell resize+grid-snap (`0x0060B000`)

Normal shell mode (`FUN_0069BBE0 == 0`):

```
delta_x   = max(0, (parent_w - 800) / 2)
shp_w     = *(short*)(g_SDBTNANM_SHP + 2)        // 156
shp_h     = *(short*)(g_SDBTNANM_SHP + 4)        // 42
X         = (parent_right - delta_x - parent_left) - 0x9C    // 0x9C = 156
row_h     = *(int*)(DAT_00B0FC24 + 0xC)          // 42 (per trace doc)
panel_y   = *(int*)(DAT_00B0FC24 + 4)            // 199 (top of button column)
row       = round( (button_top - panel_y - parent_top) / row_h )
Y         = row * row_h + panel_y
MoveWindow(button, X, Y, shp_w, shp_h, 0)        // window := 156 x 42
```

At 800×600: `X = 800 - 0 - 156 = 644`, width 156 → window right edge = **800** (flush to
screen edge). Y snaps the DLU tops (203/247/291/335/379) to grid rows (199/241/283/325/367).
This X **equals** the SDBTNANM rect from `RightPanel__ComputeLayoutRects @ 0x0072EC70`
(`tile_x + tile_w - shp_w = 632 + 168 - 156 = 644`, per the button-SHP trace) — i.e., the
button window is placed exactly on the SDBTNANM frame rect.

### 3.3b `FUN_0060B350` — Exit final-row placement (`0x0060B350`)

In normal shell mode (`FUN_0069BBE0 == 0`), Exit is resized to the same native
SDBTNANM 156×42 canvas, flush-right in the panel. Its top is derived from the panel
composition rather than from the resource DLU top:

```
X = panel_right - 156
Y = right_panel_bottom_cap_top - right_panel_tile_height
MoveWindow(exit, X, Y, 156, 42, 0)
```

This produces `(484,409,156,42)` at 640×480, `(644,535,156,42)` at 800×600,
and `(756,619,156,42)` at 1024×768. Evidence:
`decompile_function(0x0060B350)`, `decompile_function(0x0069BBE0)`, and the
right-panel rectangles recovered by `decompile_function(0x0072EC70)`.

### 3.4 `OwnerDraw_Button_00612B70` paint fork (`0x00612B70`)

```
iVar14 = rec[0x2C];                 // paint-asset type
if (iVar14 == 0) {                  // PCX bue/bde (generic default)
    if (rec[5] == 0)  build b%c%c_li/mi/ri_d.pcx  (0x0083589C/588C/587C), tile middle
    else              blit cached up/down surface (rec[5]/rec[6])
} else if (iVar14 == 1) {           // SDBTNANM
    pal = FUN_0072E2C0();           // DAT_00B0FBDC (SDBTNANM.PAL)
    shp = g_SDBTNANM_SHP;
    frame = pressed ? 4 : (highlight(rec+0xC5) ? 3 : 2);
    CC_Draw_Shape(shp, frame, &client, ..., 0x400, 0, ...);   // 0x400 = no remap
} else if (iVar14 == 2) { shp = DAT_00B0F9EC; ... }
  else if (iVar14 == 3) { shp = DAT_00B0FACC; ... }
```

The window (`GetClientRect`) the SHP draws into is 156×42 for all six buttons. Pressed
state changes the frame index to 4 without translating the SHP destination. The text
rectangle boundaries are `(left=x, top=y+1, right=x+w-2, bottom=y+h)` normally and
`(left=x+2, top=y+5, right=x+w-2, bottom=y+h)` while pressed. In `(x,y,w,h)` form those
are `(x,y+1,w-2,h-1)` and `(x+2,y+5,w-4,h-5)`. Centering therefore moves the visible
glyph result by approximately +1 X/+2 Y, but the clipping rectangle is the exact
mechanism. Evidence: `disassemble_function(0x00612B70)`, instructions
`0x00613568..0x006135EE`.

### 3.5 Why SDBTNANM, not PCX (the fork resolution)

The button paint-type field `rec[0x2C]` zero-inits to 0 (PCX). A direct static store to
that field was not located (byte-pattern search for imm/reg/byte stores at both candidate
offsets returned nothing — but that search method also failed to find a *known* store in
the same record, so it is unreliable and proves nothing; see Open Questions). The
conclusion rests instead on **geometric coherence, which is decisive**:

- `FUN_0060B000` resizes the five buttons' windows to exactly the `SDBTNANM.SHP` frame
  dimensions (156×42) and positions them at the SDBTNANM frame rect (x=644, flush-right
  in the 168-px panel, grid-snapped Y).
- That placement is byte-identical to the SDBTNANM rect `RightPanel__ComputeLayoutRects`
  computes for the button column.
- gamemd would have no reason to size/grid-place a button to the SDBTNANM frame if it
  painted `bue/bde` PCX (those have different dimensions and the PCX path tiles a middle
  piece to an arbitrary width). The window == the SDBTNANM cell ⇒ the paint is the
  SDBTNANM cell (type 1).

This corroborates the existing button-SHP trace and the Rust render's SDBTNANM choice, and
supersedes the two 2026-05-17 owner-draw/composition reports that described the type-0
`bue/bde` PCX branch as the active main-menu button art. Those reports decompiled the
generic-default branch of `OwnerDraw_Button_00612B70` without proving `rec[0x2C]` for
`0xE2`; the resize geometry shows the active type is 1, not 0.

## 4. INI Keys

None gate this fork. The button mouse-down sound (`RulesClass+0x188`,
`[AudioVisual] GUIMainButtonSound`, default `MenuClick`) is unchanged and out of scope.

## 5. Integration Points

`WM_INITDIALOG` (`FUN_00622B50`) → `FUN_0060F4B0` → child passes `FUN_0060F760` (flags),
`LAB_0060F320` (uninspected — see Open Questions), `FUN_0060F9A0` (subclass + record
create) → `FUN_0060C540` → `FUN_0060C4A0` → `EnumChildWindows(ResizeShellChildControl_0060C0C0)`.
Paint per child via the installed `OwnerDraw_Button_00612B70` window proc on `WM_PAINT`.

## 6. Current Rust Implementation Status

As of the 2026-07-18 parity repair, `src/ui/main_menu_shell/layout.rs` routes the five
stacked buttons through the 156×42 snapped-row anchor and Exit through the final-row
anchor. The resulting Exit fixtures are 409/535/619 Y at 640/800/1024 respectively.
Rendering and hit testing consume the same resolved rectangles.

`src/app_main_menu_shell_render.rs` keeps pressed SHP art stationary, selects the native
pressed frame through the shared paint policy, and uses the exact normal/pressed label
rectangles above. Focused tests pin the three Exit fixtures, the unchanged five-button
stack, stationary pressed art, and both label rectangles.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00608CD0` truth table for `0xE2` | verified | `decompile_function 0x00608CD0` | none |
| Reposition gate field = control-kind (`rec[0x1A]`), =0 for buttons | verified | `decompile_function 0x0060F9A0` | none |
| 5 buttons → `FUN_0060B000`; Exit → `FUN_00609730` / `FUN_0060B350` | verified | `decompile_function 0x0060C0C0`, `0x00609730`, `0x0060B350` | none |
| `FUN_0060B000` window = 156×42 @ x=644, grid-snap Y | verified (mechanism); `DAT_00B0FC24` row=42/panel=199 cross-ref | `decompile_function 0x0060B000`; trace `0x0072EC70` | exact `DAT_00B0FC24` values not re-decompiled here |
| `FUN_00608500` returns 0 for `0xE2` | verified | `decompile_function 0x00608500` | none |
| Paint fork `rec[0x2C]` 0=PCX / 1=SDBTNANM | verified | `decompile_function 0x00612B70` | none |
| `0xE2` buttons paint type = 1 (SDBTNANM) | verified by geometric coherence | 3.3 X==3.5 SDBTNANM rect (644) | direct `rec[0x2C]` setter not located |
| `rec[0x2C]` writer location | conflict-needs-resolution | byte-search unreliable (failed known-store control) | runtime watch / proper xref-by-offset |
| `LAB_0060F320` shell-init child pass | not-touched | xref only (`0x00622850/0x0060F5F6/0x00622F23`) | inspect for any `rec[0x2C]` write |

## 8. Open Questions — Final State

- `[RESOLVED]` Which reposition helper do the five non-Exit `0xE2` buttons take? → `FUN_0060B000` (resize to SDBTNANM 156×42, x=644, grid-snap Y). (evidence: `0x0060C0C0`, `0x00608CD0`, `0x0060F9A0`, `0x0060B000`)
- `[RESOLVED]` Is Exit (`0x3EE`) special-cased? → Yes; `FUN_00608CD0` is false, then `FUN_00609730` is true and dispatch selects `FUN_0060B350`. Normal-mode output is 156×42 in the final row above the bottom cap, `(644,535,156,42)` at 800×600. (evidence: `decompile_function(0x0060C0C0)`, `decompile_function(0x00609730)`, `decompile_function(0x0060B350)`)
- `[RESOLVED]` Title `0x694` X? → `FUN_0060B1D0` sidebar inset → 635 (Rust matches). (evidence: `0x0060C0C0`, `0x0060B1D0`)
- `[RESOLVED]` PCX vs SDBTNANM paint? → SDBTNANM (type 1), by geometric coherence: button window == SDBTNANM frame rect (644/156/42). (evidence: `0x0060B000` X == `0x0072EC70` SDBTNANM rect; `0x00612B70` fork)
- `[DEFERRED]` Exact instruction that writes `rec[0x2C]=1`. (category: needs-runtime-debugger; reason: byte-pattern search proven unreliable — it missed a known store to the same record; next-step-if-pursued: set a write watchpoint on `record+0xB0` during first `0xE2` paint, or inspect `LAB_0060F320` once a function boundary is created. The conclusion (type 1) does not depend on this — geometry already settles it — but the setter site is the last loose end.)
- `[DEFERRED]` Exact `DAT_00B0FC24` row-height/panel-top values used by `FUN_0060B000` Y snap. (category: bounded-cost-too-high; reason: cross-referenced from the existing button-SHP trace's `0x0072EC70` decompile, not re-verified here; next-step: re-decompile `RightPanel__ComputeLayoutRects`.)

## 9. Visual/UI Composition Ledger (button layer, dialog `0xE2`, 800×600)

| Order | Function / address | Condition | Asset / frame | Rect / anchor (gamemd) | Active? | Role |
|---|---|---|---|---|---|---|
| per-button | `OwnerDraw_Button_00612B70` (5 non-Exit) | `FUN_00608CD0` true → `FUN_0060B000` window | `SDBTNANM.SHP` frame 2/3/4 | 156×42 @ x=644, y∈{199,241,283,325,367} | yes | content (animated button) |
| per-button | `OwnerDraw_Button_00612B70` (Exit `0x3EE`) | `FUN_00608CD0` false → `FUN_00609730` true → `FUN_0060B350` | `SDBTNANM.SHP` frame 2/3/4 (same type) | 156×42 @ x=644, y=535 (final row above bottom cap) | yes | content (bottom-row button) |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in `0xE2` | Content | Chrome | Inactive | Evidence |
|---|---|---|---|---|---|---|---|
| `SDBTNANM.SHP` frames 2/3/4 | yes | yes | yes | yes (per-button art) | — | — | `0x00612B70` type-1, `0x0060B000` sizing |
| `bue_*30/bde_*30.pcx` | yes (preloaded) | no | no | — | — | yes (type-0 default branch not active for `0xE2`) | `0x00612B70` type-0 branch not reached |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| 5 non-Exit buttons sized to SDBTNANM frame 156×42, x=644 (flush-right in 168 panel) | `0x0060B000`, `0x0072EC70` | repaired before the 2026-07-18 correction pass | `src/ui/main_menu_shell/layout.rs`, `src/render/shell_paint.rs` | preserve button rect/art at 156×42 and x=644 | at 800×600, SP button art occupies x∈[644,800], y top=199 | do NOT restore the `(168-162)/2` center-inset |
| Button Y grid-snapped to 42-px SDBTNANM rows from y=199 | `0x0060B000` (snap math), trace `0x0072EC70` | repaired before the 2026-07-18 correction pass | `src/ui/shell/layout.rs` | preserve `199 + n*42` row placement | SP=199, WW=241, Net=283, M&C=325, Options=367 | do NOT use DLU 13/8 Y directly |
| Exit `0x3EE` uses `FUN_0060B350`: 156×42, flush-right, final tile above the bottom cap | `0x0060C0C0`, `0x00609730`, `0x0060B350` | repaired 2026-07-18 | `src/ui/shell/layout.rs`, `src/ui/main_menu_shell/layout.rs` | derive Y from `panel.bottom.y - tile_h` | Exit=(644,535,156,42) at 800×600 | do NOT use raw DLU Y; do NOT grid-snap Exit against the upper stack |
| Pressed frame changes without moving SHP; label uses exact pressed clipping rect | `0x00612B70`, assembly `0x00613568..0x006135EE` | repaired 2026-07-18 | `src/render/shell_paint.rs`, `src/app_main_menu_shell_render.rs` | art origin unchanged; label `(x+2,y+5,w-4,h-5)` | pressed art origin equals default; label boundaries match | do NOT approximate the result as only a glyph offset |
| Buttons paint `SDBTNANM.SHP` frames 2/3/4 (not `bue/bde` PCX) | `0x00612B70` type-1 + geometric coherence | none observed | `src/render/main_menu_shell_chrome.rs` | keep SDBTNANM frames | colored (not greyscale) buttons | do NOT switch to `bue/bde` PCX per the older composition docs |

### Stale Docs / Follow-up

- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` §3/§4 and
  `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` "Button Rendering and
  Text": replace "buttons render `bue_li30/mi30/ri30.pcx` … 30-px art centered in a 162×37
  client" with: "the five non-Exit `0xE2` buttons are resized to the `SDBTNANM.SHP` cell
  (156×42) and painted with SDBTNANM frames 2/3/4 (type-1 owner-draw branch); the
  `bue/bde` PCX (type-0) branch is the generic default and is NOT the active `0xE2` art.
  Exit `0x3EE` is also 156×42 but is placed by the distinct final-row helper."
- `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md` branch 2 / control-to-helper
  table: replace "`FUN_00608CD0` does not return non-zero for `0xE2` … buttons → fallback
  coord-fixup" with the verified `FUN_00608CD0` truth table (§3.1): 5 buttons + title +
  `0x71C` true → `FUN_0060B000` (buttons, kind 0) / `FUN_0060B1D0` (statics); Exit makes
  that first predicate false but the subsequent `FUN_00609730` predicate routes it to
  `FUN_0060B350`. The title `0x694` final X is 635 (via `FUN_0060B1D0`), not raw 638.

## Sources

- Ghidra (decompiled/inspected, including the 2026-07-18 correction pass): `0x00612B70`, assembly `0x00613568..0x006135EE`, `0x00608CD0`, `0x00609730`, `0x0060B000`, `0x0060B350`, `0x0060B1D0`, `0x0060C0C0`, `0x0060F9A0`, `0x0060F4B0`, `0x0060F760`, `0x00608500`, `0x0069BBE0`, byte-search of `record+0xB0/0xB4` (inconclusive), `LAB_0060F320` xrefs.
- Docs: `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`, `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`, `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md`, `traces/MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md` (SDBTNANM 156×42; `RightPanel__ComputeLayoutRects @ 0x0072EC70` x=644).
- Rust: `src/ui/main_menu_shell/layout.rs`, `src/app_main_menu_shell_render.rs`, `src/render/main_menu_shell_chrome.rs`.
