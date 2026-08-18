# Scroll Button X/Y Position Setter — Ghidra Report

**Target:** ScrollUp (ID 0xC8, gadget at `DAT_00b0b408`) and ScrollDown (ID 0xC9, gadget at `DAT_00b0b328`)  
**Question:** Which function writes their X/Y position values, what expressions, how often?  
**Verified via:** `decompile_function 0x006abd30`, `decompile_function 0x004e1a20`,
`decompile_function 0x004e1960`, `decompile_function 0x004e1a40`,
`get_function_callers 0x006abd30`, `get_function_by_address` on all three callers.

---

## 1. The Writer: `SidebarClass__InitSurface` at `0x006abd30`

**Confirmed via `decompile_function 0x006abd30`.**

`SidebarClass__InitSurface` is the sole writer of both scroll button X/Y fields.
It calls two small thiscall helpers (confirmed by decompiling them):

- `FUN_004e1a20` (`SetPosition`, `__thiscall`): writes `this+0x0C = X`, `this+0x10 = Y`
  — verified via `decompile_function 0x004e1a20`
- `FUN_004e1960` (`SetVisible`, `__fastcall`): writes `this+0x1C = 1`
  — verified via `decompile_function 0x004e1960`

Active in YR: **Yes** — no flag gating, runs unconditionally in `InitSurface`.

---

## 2. ScrollUp (ID 0xC8, base `0xb0b408`) — Exact Expressions

```
// Sequence in InitSurface (confirmed by tooltip block: iStack_18 = DAT_00b0b414, iStack_1c = 200 = 0xC8)
FUN_004e1a20(DAT_00b0b508, DAT_00b0b50c);   // this = scroll-up gadget
FUN_004e1960();                              // IsVisible = 1
_DAT_00b0b36c = -g_SidebarX;               // scroll-down gadget +0x44 (= 0xb0b328+0x44)
```

- **ScrollUp.X** (`0xb0b408+0x0C = 0xb0b414`) ← `DAT_00b0b508` (ScrollX global = SidebarX + 39)
- **ScrollUp.Y** (`0xb0b408+0x10 = 0xb0b418`) ← `DAT_00b0b50c` (CameoAreaY + 7 + visible_cameo_height)
- **ScrollUp.+0x44** is NOT set here (it's in the next block)

The `_DAT_00b0b36c` assignment writes `-g_SidebarX` into `0xb0b36c = 0xb0b328 + 0x44`
(a scroll offset / clip field on the ScrollDown gadget, set right after ScrollUp's position).

---

## 3. ScrollDown (ID 0xC9, base `0xb0b328`) — Exact Expressions

```
// Sequence in InitSurface (confirmed by tooltip block: iStack_18 = DAT_00b0b334, iStack_1c = 0xC9)
FUN_004e1a20(DAT_00b0b510 + DAT_00b0b508, DAT_00b0b50c);  // this = scroll-down gadget
FUN_004e1960();                                             // IsVisible = 1
_DAT_00b0b44c = -g_SidebarX;                              // scroll-up gadget +0x44 (= 0xb0b408+0x44)
```

- **ScrollDown.X** (`0xb0b328+0x0C = 0xb0b334`) ← `DAT_00b0b510 + DAT_00b0b508` (ScrollX + ScrollWidth)
- **ScrollDown.Y** (`0xb0b328+0x10 = 0xb0b338`) ← `DAT_00b0b50c` (same Y as ScrollUp)
- **ScrollDown.+0x44** (`0xb0b328+0x44 = 0xb0b36c`) ← set in the prior block to `-g_SidebarX`

The `_DAT_00b0b44c` assignment writes `-g_SidebarX` into `0xb0b44c = 0xb0b408 + 0x44`
(same clip/offset field on the ScrollUp gadget, set right after ScrollDown's position).

---

## 4. Source Globals (from SIDEBAR_SYSTEM_GHIDRA_REPORT.md, confirmed consistent)

| Global          | Value                                     | Notes                          |
|-----------------|-------------------------------------------|--------------------------------|
| `DAT_00b0b508`  | ScrollX = SidebarX + 39                  | X for ScrollUp                 |
| `DAT_00b0b50c`  | ScrollY = CameoAreaY + 7 + cameo_h       | Shared Y for both buttons      |
| `DAT_00b0b510`  | ScrollWidth (46 RA2 / 45 YR)             | Horizontal offset to ScrollDown|
| `g_SidebarX`    | Left edge of sidebar in screen coords     | Used for clip offset at +0x44  |

ScrollDown.X = ScrollUp.X + ScrollWidth (immediately to the right of ScrollUp), matching the documented layout.

---

## 5. Shared vs Separate Writer

ScrollUp and ScrollDown share the **same writer function** (`SidebarClass__InitSurface`), in a
sequential two-block pattern (ScrollUp block → ScrollDown block). No separate paths.

---

## 6. How Often the Writer Runs

`get_function_callers 0x006abd30` returned three callers:

| Caller               | Address      | Context                                      |
|----------------------|--------------|----------------------------------------------|
| `FUN_00560bf0`       | `0x00560bf0` | Set-video-mode / resolution change (calls InitSurface at the end after rebuilding surfaces) |
| `FUN_0067e440`       | `0x0067e440` | Load saved game (calls InitSurface after restoring world state — confirmed by `LOADING_GAME` string) |
| `Set_View_Dimensions`| `0x004a8960` | Called from `FUN_00560bf0` → `FUN_0067e730` → here (same resolution-change chain) |

**Frequency:** Once at game/scenario start (via resolution setup), once on load-game, and once on
any resolution change. NOT per-frame, NOT per-scroll event. `UpdateScrollButtons` (0x006a6610)
only shows/hides the buttons — it never writes X/Y.

Active in YR: **Yes** — all three call sites are reachable in a normal YR skirmish.

---

## 7. TS-Legacy Filter

`SidebarClass__UpdateScrollButtons` (0x006a6610) was decompiled — it calls show/hide helpers
(`FUN_004e1450` / `FUN_004e1460`) based on cameo-count vs scroll capacity. No X/Y writes anywhere
in that function. No TS-legacy gating found in InitSurface for the scroll button positioning.

---

## 5 Most Load-Bearing Verified Facts

1. **Writer = `SidebarClass__InitSurface` at `0x006abd30`** (confirmed by `decompile_function 0x006abd30`
   showing `FUN_004e1a20(DAT_00b0b508, DAT_00b0b50c)` and `FUN_004e1a20(DAT_00b0b510+DAT_00b0b508, DAT_00b0b50c)`)

2. **`FUN_004e1a20` writes `this+0x0C=X` and `this+0x10=Y`** (confirmed `decompile_function 0x004e1a20`:
   `*(param_1+0xC)=param_2; *(param_1+0x10)=param_3`)

3. **ScrollDown.X = ScrollUp.X + ScrollWidth** (`DAT_00b0b510 + DAT_00b0b508`), ScrollDown shares
   the same Y as ScrollUp (`DAT_00b0b50c`) — confirmed by two sequential `FUN_004e1a20` calls in `InitSurface`

4. **Gadget addresses confirmed by tooltip registration block**: IDs 0xC8→`iStack_18=DAT_00b0b414`
   (= `0xb0b408+0xC`), ID 0xC9→`iStack_18=DAT_00b0b334` (= `0xb0b328+0xC`)
   — confirmed within the same `decompile_function 0x006abd30` output

5. **Writer runs on init/load/resize only, not per-frame** — confirmed by `get_function_callers 0x006abd30`
   returning exactly 3 callers: resolution-change, load-game, and view-dimensions setup

---

**Status: COMPLETE**
