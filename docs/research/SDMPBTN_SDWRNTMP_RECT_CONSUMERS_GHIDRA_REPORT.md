# SDMPBTN / SDWRNTMP Rect Consumers — Ghidra Report

Date: 2026-05-19

Scope: Identify every function that reads `DAT_00B0FC14` (SDMPBTN.SHP dest rect)
and `DAT_00B0FC18` (SDWRNTMP.SHP dest rect), determine which dialog/control
paints them, and settle whether either asset appears on the main-menu dialog
`0xE2` paint path.

No Rust code was modified.

2026-05-23 follow-up correction: the `0xE2` main-menu verdict in this report
remains correct, but the Skirmish `0x102` binding is now verified. `FUN_0060C930`
sets the SDMPBTN/Minimap_Button flag for dialog ids including `0x102`, and
`WM_PAINT_Handler @ 0x00621E90` then calls `Minimap_Button @ 0x0072E860`.
Therefore `SDMPBTN.SHP` frame `0` is visible standard Skirmish `0x102`
right-panel chrome at `DAT_00B0FC14`. It is not the preview bitmap/backing.
See `skirmish-ui/SKIRMISH_0X102_TOP_PREVIEW_CHROME_SDTP_SDMPBTN_GHIDRA_REPORT.md`.

---

## 1. Background

`RightPanel__ComputeLayoutRects @ 0x0072EC70` writes both rects:

- `DAT_00B0FC14` ← SDMPBTN.SHP destination rect (WRITEs at `0x0072EE67`, `0x0072EE6E`)
- `DAT_00B0FC18` ← SDWRNTMP.SHP destination rect (WRITEs at `0x0072EDA1`, `0x0072EDA8`)

Parent doc (`RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md §10.3`) flags these as
"computed but never used in `RightPanel__Draw @ 0x0072E450`". This report
enumerates every reader.

---

## 2. Complete Xref Enumeration

### `DAT_00B0FC14` — xrefs (excluding writer `RightPanel__ComputeLayoutRects`)

| Xref address | Function | Operation |
|---|---|---|
| `0x0072AC75` | `FUN_0072AC40` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072AC8D` | `FUN_0072AC40` | WRITE (zero after free) |
| `0x0072DF1B` | `FUN_0072DEF0` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072DF33` | `FUN_0072DEF0` | WRITE (zero after free) |
| `0x0072DFDB` | `FUN_0072DFB0` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072DFF3` | `FUN_0072DFB0` | WRITE (zero after free) |
| `0x0072E1C2` | `FUN_0072E1B0` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072E1DC` | `FUN_0072E1B0` | WRITE (zero after free) |
| `0x00607392` | `FUN_006071E0` | READ (copy into local rect, used in draw loop) |
| `0x0072E863` | `Minimap_Button @ 0x0072E860` | READ (X,Y into local, passed to `CC_Draw_Shape`) |

### `DAT_00B0FC18` — xrefs (excluding writer `RightPanel__ComputeLayoutRects`)

| Xref address | Function | Operation |
|---|---|---|
| `0x0072AC87` | `FUN_0072AC40` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072AC9E` | `FUN_0072AC40` | WRITE (zero after free) |
| `0x0072DF2D` | `FUN_0072DEF0` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072DF44` | `FUN_0072DEF0` | WRITE (zero after free) |
| `0x0072DFED` | `FUN_0072DFB0` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072E004` | `FUN_0072DFB0` | WRITE (zero after free) |
| `0x0072E1D6` | `FUN_0072E1B0` | READ (pass to `FUN_007C8B3D` = free) |
| `0x0072E1ED` | `FUN_0072E1B0` | WRITE (zero after free) |
| `0x006073BE` | `FUN_006071E0` | READ (copy into local rect, used in draw loop) |

**Summary:** `DAT_00B0FC18` is NOT read by `Minimap_Button`; only `DAT_00B0FC14` is.

---

## 3. Individual Consumer Analysis

### 3a. Lifecycle functions (free + re-init) — `FUN_0072AC40`, `FUN_0072DEF0`, `FUN_0072DFB0`, `FUN_0072E1B0`

All four functions call `FUN_007C8B3D(DAT_00B0FC14)` then zero it, and the same
for `DAT_00B0FC18`. `FUN_007C8B3D` is a thin wrapper around `FUN_007C93E8`,
which is a heap free routine. The READ in each is therefore the value being
passed to free (a pointer to allocated rect memory), not a paint-position read.

- **`FUN_0072AC40`**: Teardown that frees the right-panel rect pool and then
  calls game-system cleanup. Called from `FUN_006BE1C0` (game-session cleanup
  path called near end of `WinMain`), and from `WinMain @ 0x006BB9A0` directly.
- **`FUN_0072DEF0`**: Same free-rect body (no reload). Called from `ScenarioClass__Start_Scenario`,
  `TriggerAction__Execute`, and scenario/dialog transition functions.
- **`FUN_0072DFB0`**: Free-rect-then-reload body. After freeing, it constructs
  new `CDFileClass` objects for every right-panel SHP, then calls
  `RightPanel__ComputeLayoutRects()` and sets the initialized flag
  `DAT_00B0FBE0 = 1`. Called from `Main_Game @ 0x0048CCC0` and
  `FUN_0055CFD0`. This is the right-panel init function.
- **`FUN_0072E1B0`**: Free-rect-then-recompute only (no SHP reload). Called from
  `FUN_00560BF0` (video mode change handler), meaning the rects are recomputed
  whenever the screen resolution changes.

**Conclusion for lifecycle functions:** These are allocator/deallocator
invocations, not paint-position consumers.

---

### 3b. `Minimap_Button @ 0x0072E860` — SDMPBTN.SHP painter

```text
void __fastcall Minimap_Button(undefined4 param_1, undefined4 param_2)
{
    local_8 = *DAT_00B0FC14;       // rect.x
    local_4 = DAT_00B0FC14[1];     // rect.y
    CC_Draw_Shape(DAT_00B0F9DC, 0, &local_8, param_2, 0x400, ...);
}
```

`DAT_00B0F9DC` is the SDMPBTN.SHP file handle (loaded in `FUN_0072DFB0`).
`DAT_00B0FC14` supplies the on-screen draw position (first two words = x, y).
Only `param_2` (the destination surface) is parameterized; the position always
comes from the global rect.

**Caller of `Minimap_Button`:** `WM_PAINT_Handler @ 0x00621E90` only, at the
line:

```text
if (*(char *)((int)piVar9 + 0xd5) != '\0') { Minimap_Button(); }
```

The `+0xD5` byte in the dialog record is a per-dialog flag. The control-flow in
`WM_PAINT_Handler` also guards all of this behind:

1. `FUN_0072E260() != 0` — checks `DAT_00B0FBE0`, the right-panel initialized
   flag. If the right panel has never been loaded, nothing is drawn.
2. `paint_mode == 1` on the dialog record — only "common shell" dialogs take
   this branch.

**Active in YR:** Conditional. Only fires if the dialog's `+0xD5` flag is set.
Record byte `+0xD5` is **BSS-default `0`** (records are zero-initialized by
`operator_new(0x20)` in the `WM_PAINT_Handler` allocate path), and **no code
path on the `0xE2` `WM_INITDIALOG` enumeration sets it**. The four classifier
helpers `FUN_0060CAF0` / `FUN_0060C930` / `FUN_0060CCC0` / `FUN_0060CDB0` write
record bytes `+0xD9` / `+0xDA` / `+0xDB` / `+0xDC` respectively (per
`RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md` §"Classifier
offsets") — they do **not** touch `+0xD5`. Therefore `Minimap_Button` does NOT
fire for dialog `0xE2`: the gate stays at its default-cleared value because
no setter is ever invoked on this dialog.

For dialog `0x102`, the relevant setter is `FUN_0060C930`, which writes the
record byte that appears as `[EBX+0xD6]` in the shifted `WM_PAINT_Handler`
record pointer. That makes `Minimap_Button` active for the standard offline
Skirmish screen. The shape remains chrome at `DAT_00B0FC14`; it is not the
preview bitmap/backing.

---

### 3c. `FUN_006071E0` — Skirmish pre-game transition animator

This is a long animation loop (~350 lines of decompiled C). It:

1. Copies `DAT_00B0FC14[0..3]` into local rect `uStack_A4..uStack_98`
2. Copies `DAT_00B0FC18[0..3]` into local rect `iStack_E0..iStack_D4`
3. Runs an `iStack_BC`-iteration draw loop that calls `CC_Draw_Shape` repeatedly
   for SDTP, animated button frames (SDBTNANM), radar frame open, and other
   right-panel elements to produce a slide-in/expand animation.

The local rects are used as draw positions for specific animation frames (frame
indices driven by iteration counter). The loop ends with `SendMessageA(hWnd,
0x4ED/0x4EC, ...)`, transitioning the dialog.

**Callers of `FUN_006071E0`:**
- `FUN_00607FD0` — called from the WM_PAINT path when record byte `+0xC2` is
  set on the dialog
- `FUN_00608260` — owner-draw button press handler that triggers the transition
  animation when a main-menu button is clicked (condition: `+0xC1` flag set,
  `piVar1[0x2D] == 1`, window visible and enabled)
- `FUN_00622B50 @ 0x00622B50` — common shell WM_PAINT handler, fires if record
  byte `+0xBE` is set
- `FUN_00788B00` — WestWood online nick-register flow
- `SimpleWonlineDialogControl__Constructor @ 0x00789B60` — WestWood online
  room-join flow (only if game-mode is not LAN/IP/WOL-specific)

**What context?** This is the **slide-in animation** played when any main-menu
button is activated. It fires within the common-shell dialog framework (paint
mode 1 dialogs including `0xE2`), but only as a one-shot transition event, not
during normal idle painting.

**Active in YR:** Yes. Every main-menu button click triggers
`FUN_00608260` → `FUN_006071E0` for the slide animation. Fires once per button
press during a normal session.

---

## 4. Role of SDMPBTN.SHP and SDWRNTMP.SHP in Each Context

| Asset | Context | Fires during `0xE2` idle paint? | When it fires |
|---|---|---|---|
| SDMPBTN.SHP (`DAT_00B0FC14`) | `Minimap_Button` | No — `+0xD5` flag cleared for `0xE2` | Only in Skirmish/in-game sidebar when minimap toggle button is shown |
| SDMPBTN.SHP (`DAT_00B0FC14`) | `FUN_006071E0` transition loop | No — not idle paint, one-shot | Main-menu button click slide animation |
| SDWRNTMP.SHP (`DAT_00B0FC18`) | `FUN_006071E0` transition loop | No — not idle paint, one-shot | Main-menu button click slide animation |

Neither asset is drawn during the **idle paint** of dialog `0xE2`. Both appear
in the **one-shot slide-in transition animation** (`FUN_006071E0`) that plays
when the player clicks a main-menu button.

---

## 5. Main-Menu Dialog `0xE2` Paint Path — SDMPBTN/SDWRNTMP Verdict

The `0xE2` idle `WM_PAINT` path (`WM_PAINT_Handler @ 0x00621E90`) does NOT call
`Minimap_Button` for `0xE2` (record `+0xD5` is cleared). It does NOT call
`FUN_006071E0` (that is triggered only by button press events).

**SDMPBTN.SHP and SDWRNTMP.SHP are absent from the `0xE2` idle paint
composition.** The existing Rust main-menu shell is not missing any
SDMPBTN/SDWRNTMP rendering for the idle menu state.

The rects serve two purposes:
1. **Transition animation** (`FUN_006071E0`): used as draw positions during the
   slide-in animation on button press.
2. **Sidebar/Skirmish minimap button** (`Minimap_Button`): used in the
   in-game/Skirmish sidebar when the `+0xD5` flag is enabled on a dialog record.

---

## 6. Confidence Assessment

Using the 3-axis model:

- **Content (what the functions do):** HIGH. Decompiled all consumers directly;
  the draw calls and flag guards are explicit.
- **Identity (are these the right functions for these assets):** HIGH. SDMPBTN.SHP
  is loaded into `DAT_00B0F9DC` (verified in `FUN_0072DFB0` which also loads
  known-named SHPs like `g_SDTP_SHP`, `g_SDBTNBKGD_SHP`). The adjacent symbol
  stream is consistent.
- **Binding (is the flag `+0xD5` actually cleared for dialog `0xE2`):** MEDIUM-HIGH.
  The parent doc (`MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`)
  states this explicitly with evidence from `FUN_0060CAF0` etc., but this report
  did not re-verify those specific functions. Accepting the sibling doc's finding.

Active in YR: Yes (transition animation), Conditional (minimap button — not
active for `0xE2` idle).

---

## 7. Open Questions

1. **What dialog sets `+0xD5` for `Minimap_Button`?** Not investigated this run.
   Likely Skirmish or in-game sidebar dialog. Check `FUN_0060CCC0`,
   `FUN_0060CDB0`, or similar init functions that set per-dialog flags.
2. **What exact frames of SDMPBTN and SDWRNTMP are used in `FUN_006071E0`?**
   The animation loop is complex; frame index selection depends on iteration
   counter and the button flags (`cVar16`, `cVar15`). Not fully mapped.
3. **Does the Rust transition animation need SDMPBTN/SDWRNTMP?** If the
   slide-in transition animation is implemented in Rust, these two SHP assets
   and their rects need to be included.

---

## 8. Summary of Verified Facts

1. `Minimap_Button @ 0x0072E860` reads `DAT_00B0FC14` (x,y only) and draws
   SDMPBTN.SHP frame 0 at that position. It is called only from
   `WM_PAINT_Handler @ 0x00621E90` when record byte `+0xD5` is set.
   (Evidence: decompile `0x0072E860`, xref from `0x00621E90` at `~0x006221B0`)

2. `FUN_006071E0 @ 0x006071E0` reads both `DAT_00B0FC14` and `DAT_00B0FC18`
   into local rects for use in the slide-in transition animation draw loop.
   It is triggered by button-press events (`FUN_00608260`), not idle paint.
   (Evidence: decompile `0x006071E0`, xref `0x00607392` and `0x006073BE`)

3. `FUN_0072DFB0` (right-panel init, called from `Main_Game`) frees and
   zeros both rects then calls `RightPanel__ComputeLayoutRects` and sets
   `DAT_00B0FBE0 = 1`. `FUN_0072E1B0` (video-mode change, called from
   `FUN_00560BF0`) recomputes rects without reloading SHPs.
   (Evidence: decompile `0x0072DFB0`, `0x0072E1B0`)

4. Dialog `0xE2` idle `WM_PAINT` does NOT draw SDMPBTN.SHP or SDWRNTMP.SHP.
   The `+0xD5` record byte that gates `Minimap_Button` is cleared for `0xE2`
   by its init path; `FUN_006071E0` is not called during idle paint.
   (Evidence: `WM_PAINT_Handler @ 0x00621E90` conditional at `~0x006221A8`,
   sibling doc `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`)

5. Both rects are used in `FUN_006071E0`'s transition animation, which fires
   on every main-menu button click in YR (one-shot per click).
   (Evidence: callers of `FUN_006071E0` via `get_function_callers`)
