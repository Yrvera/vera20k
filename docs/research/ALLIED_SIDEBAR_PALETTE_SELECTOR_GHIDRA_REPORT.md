# Allied Sidebar Palette Selector (FUN_0072f350 / PaletteLoad) — Ghidra Research Report

**Primary Address:** `0x0072f350` (`PaletteLoad`, also labeled in Ghidra)
**Confidence:** HIGH — full decompilation + assembly trace + xref verification for all claims
**Active in YR:** Yes — called unconditionally from `InitSideMixFiles` at `0x00534fa0`
**Report date:** 2026-05-19

---

## 1. Overview

`FUN_0072f350` (named `PaletteLoad` in Ghidra) is the **left-panel palette loader**.
It is called once per side-switch from `InitSideMixFiles` (`0x00534fa0`), after the
side-specific MIX files are mounted. It loads 5 ConvertClass palette objects into
globals used by the left panel (radar area, loading screen, observer icon, Yuri icon).
It does **NOT** load DIALOG.PAL, DIALOGY.PAL, or DIALOGN.PAL — those are loaded in
a separate function (`FUN_0072aa40`) during game startup.

The prior SIDEBAR_CONSTRUCTION §6 reference to `FUN_0072f350` as "loads SIDEBAR.PAL
variants" was partially wrong: it does load SIDEBAR.PAL for one slot, but the primary
discriminator is UIBKGDY.PAL (Yuri) vs UIBKGD.PAL (non-Yuri) for the left panel
background ConvertClass.

---

## 2. Exact Call Sequence (verified via `disassemble_function 0x0072f350`)

The function body in pseudocode:

```
PaletteLoad():
    FUN_00730100()              // Free left-panel SHP globals (lazy-init teardown)
    FUN_00730400()              // Free 5 ConvertClass palette globals
    FUN_00730530()              // Free layout rect globals (0xb0fc30..0xb0fc6c)
    DAT_00b0fc0c = 0            // Clear lazy-init flag for LeftPanel__Draw
    MIX_LoadNeutral()           // 0x0072fa10 — reload left-panel SHPs from MIX

    if (ScenarioClass+0x34B8 == 2):   // Yuri (side index 2)
        LoadPal(UIBKGDY.PAL, buf=0xb0fbec, convert=0xb0fbf0)
        LoadPal(RADARYURI.PAL, buf=0xb0fbf4, convert=0xb0fbf8)
    else:                             // Allied (0) or Soviet (1) — no distinction here
        LoadPal(SIDEBAR.PAL, buf=0xb0fbf4, convert=0xb0fbf8)
        LoadPal(UIBKGD.PAL, buf=0xb0fbec, convert=0xb0fbf0)

    // Shared for all sides:
    LoadPal(SIDEBAR.PAL, buf=0xb0fbe4, convert=0xb0fbe8)
    LoadPal(OBSERVER.PAL, buf=0xb0fbfc, convert=0xb0fc00)
    LoadPal(YRII.PAL, buf=0xb0fc04, convert=0xb0fc08)

    LeftPanel__ComputeLayoutRects()   // 0x0072fc60
    DAT_00b0fc0c = 1                  // Set lazy-init flag
```

Where `LoadPal(filename, buf, convert)` = call to `0x0072ade0` with
ECX=filename_ptr (from data table at `0x00844bXX`), EDX=buf_global_address,
PUSH=convert_global_address.

---

## 3. Palette Global Mapping (verified via assembly + string reads)

### PaletteLoad (FUN_0072f350) — left panel palettes

| Filename string | String addr | Palette buf global | ConvertClass global | Side condition |
|---|---|---|---|---|
| `UIBKGDY.PAL` | `0x00845414` | `0x00b0fbec` | `0x00b0fbf0` | Yuri (side==2) only |
| `RADARYURI.PAL` | `0x00845404` | `0x00b0fbf4` | `0x00b0fbf8` | Yuri (side==2) only |
| `SIDEBAR.PAL` | `0x0084542c` | `0x00b0fbf4` | `0x00b0fbf8` | Allied/Soviet (side!=2) |
| `UIBKGD.PAL` | `0x00845420` | `0x00b0fbec` | `0x00b0fbf0` | Allied/Soviet (side!=2) |
| `SIDEBAR.PAL` | `0x0084542c` | `0x00b0fbe4` | `0x00b0fbe8` | All sides (shared) |
| `OBSERVER.PAL` | `0x008453f4` | `0x00b0fbfc` | `0x00b0fc00` | All sides (shared) |
| `YRII.PAL` | `0x008453e8` | `0x00b0fc04` | `0x00b0fc08` | All sides (shared) |

**SIDEBAR.PAL is loaded TWICE** for Allied/Soviet: once into buf/convert pair
`(0xb0fbf4, 0xb0fbf8)` and once into `(0xb0fbe4, 0xb0fbe8)`. Two separate
ConvertClass instances from the same source palette.

### FUN_0072aa40 — DIALOG palette group (loaded at game startup, NOT in PaletteLoad)

| Filename string | String addr | Palette buf global | ConvertClass global |
|---|---|---|---|
| `DIALOG.PAL` | `0x0084550c` | `0x00b0fb64` | `0x00b0fb68` |
| `DIALOGY.PAL` | `0x00845518` | `0x00b0fb6c` | `0x00b0fb70` |
| `DIALOGN.PAL` | `0x00845524` | `0x00b0fb5c` | `0x00b0fb60` |
| `MAINBTTN.PAL` | `0x008454fc` | `0x00b0fb74` | `0x00b0fb78` |

Pointer table for DIALOG group is at `0x00844b9c`–`0x00844ba8` (verified by
`read_memory 0x00844b9c`).

---

## 4. Who Loads DIALOG.PAL — The Actual Side Selector

The SIDEBAR_CONSTRUCTION doc §8 listed these four globals as "per-side palette
selector" but the selection logic is **not** in PaletteLoad — it is in the
**WM_PAINT_Handler** (`0x00621e90`) loading screen path (mode == 2).

Exact selection in `WM_PAINT_Handler` (verified via `decompile_function 0x00621e90`):

```c
iVar2 = DAT_00b0fc8c;   // PUDLGBGY SHP (Yuri loading background)
iVar7 = DAT_00b0fc88;   // PUDLGBGS SHP (Soviet loading background)
iVar4 = DAT_00b0fc84;   // PUDLGBGA SHP (Allied loading background)
iVar10 = DAT_00b0fc80;  // PUDLGBGN SHP (Neutral loading background)

cVar3 = FUN_0069bbe0();   // IsInGame check

if (cVar3 == '\0') {                               // NOT in game
    iVar8 = FUN_0072b030();                        // DIALOGN.PAL ConvertClass
    // iVar10 stays = PUDLGBGN SHP
}
else if (ScenarioClass+0x34B8 == 0) {             // Allied
    iVar8 = FUN_0072aff0();                        // DIALOG.PAL ConvertClass
    iVar10 = iVar4;                                // Allied SHP
}
else if (ScenarioClass+0x34B8 == 1) {             // Soviet
    iVar8 = FUN_0072aff0();                        // DIALOG.PAL ConvertClass (same as Allied!)
    iVar10 = iVar7;                                // Soviet SHP
}
else {                                             // Yuri (side == 2)
    iVar8 = FUN_0072b010();                        // DIALOGY.PAL ConvertClass
    iVar10 = iVar2;                                // Yuri SHP
}

if (iVar8 != 0 && iVar10 != 0) {
    CC_Draw_Shape(iVar10, 0, &pos, &rect, 0x400, 0, 0, 0, 1000, ...);
}
```

**Finding: Allied (side==0) and Soviet (side==1) both use DIALOG.PAL for the
loading screen overlay.**  Only Yuri uses DIALOGY.PAL.  DIALOGN.PAL is used
when no game is loaded (menu/lobby state).

The accessor functions:
- `FUN_0072aff0` (`0x0072aff0`) → returns `DAT_00b0fb68` (DIALOG.PAL ConvertClass)
- `FUN_0072b010` (`0x0072b010`) → returns `DAT_00b0fb70` (DIALOGY.PAL ConvertClass)
- `FUN_0072b030` (`0x0072b030`) → returns `DAT_00b0fb60` (DIALOGN.PAL ConvertClass)

---

## 5. ConvertClass Global Usage Map

### `DAT_00b0fbe8` / `DAT_00b0fbe4` — SIDEBAR.PAL (shared, all sides)

- Read heavily by `LeftPanel__Draw` (`0x0072f540`) — feeds all left-panel SHP draws
- Also read by `OwnerDraw_Button_00612B70` (`0x00612b70`) when button type == 2
  (returns via `FUN_0072f4b0` at `0x0072f4b0`)
- Released/nulled in `FUN_00730400` (`0x00730400`)

### `DAT_00b0fbf0` / `DAT_00b0fbec` — UIBKGDY.PAL (Yuri) or UIBKGD.PAL (non-Yuri)

- Read by `LeftPanel__Draw` at offset `0x0072f61c` (first background SHP draw)
- Released in `FUN_00730400`

### `DAT_00b0fbf8` / `DAT_00b0fbf4` — RADARYURI.PAL (Yuri) or SIDEBAR.PAL (non-Yuri)

- Read by `LeftPanel__Draw` at offset `0x0072f6e4`
- Read by `RadarClass__Draw` (`0x00653100`), `RadarClass__DrawJammedMode`
  (`0x00653fa0`), `RadarClass__PerFrameMovieUpdate` (`0x006579e0`),
  `RadarClass__Update` (`0x00656ec0`) — all via `FUN_0072f510` (`0x0072f510`)
  which returns `DAT_00b0fbf8`
- Released in `FUN_00730400`

### `DAT_00b0fc00` / `DAT_00b0fbfc` — OBSERVER.PAL (all sides)

- Read by `FUN_0072fbc0` (`0x0072fbc0`) — palette reload sub-function
- Used for observer/spectator icon drawing

### `DAT_00b0fc08` / `DAT_00b0fc04` — YRII.PAL (all sides)

- Read by `FUN_0072f4d0` (`0x0072f4d0`) which is the font ConvertClass accessor
  (returns `DAT_00b0fc08`)
- Read by `FUN_0072fbc0`
- Despite the name "YRII.PAL", this is loaded for ALL sides unconditionally

### `DAT_00b0fb68` — DIALOG.PAL ConvertClass (loaded by FUN_0072aa40)

- Read by loading screen palette selector in `WM_PAINT_Handler` (Allied + Soviet)
- Accessor: `FUN_0072aff0`

### `DAT_00b0fb70` — DIALOGY.PAL ConvertClass (loaded by FUN_0072aa40)

- Read by loading screen palette selector in `WM_PAINT_Handler` (Yuri only)
- Accessor: `FUN_0072b010`

### `DAT_00b0fb60` — DIALOGN.PAL ConvertClass (loaded by FUN_0072aa40)

- Read by loading screen palette selector in `WM_PAINT_Handler` (no-game/menu)
- Accessor: `FUN_0072b030`

### `DAT_00b0fb78` — MAINBTTN.PAL ConvertClass (loaded by FUN_0072aa40)

- Accessor: `FUN_0072b050` (`0x0072b050`)
- Used by `OwnerDraw_Button_00612B70` when button type == 3

---

## 6. CDFileClass__Constructor / LoadPal Inner Mechanics (verified via `decompile_function 0x0072ade0`)

```
LoadPal(filename_ptr ECX, buf_addr EDX, convert_addr PUSH):
    CCFileClass__Constructor(filename_ptr)    // open PAL file by name
    data_ptr = FUN_004a3890()                // read file into heap buffer
    if data_ptr == 0: return early (file not found)

    palette_buf = operator_new(0x300)        // 256 × 3 bytes = 768 bytes
    for i in 0..256:
        r = data_ptr[i*3+0] << 2            // shift left 2 = multiply by 4
        g = data_ptr[i*3+1] << 2
        b = data_ptr[i*3+2] << 2
        palette_buf[i*3+0] = r
        palette_buf[i*3+1] = g
        palette_buf[i*3+2] = b
    *buf_addr = palette_buf                  // write RGB buffer to output global

    free(data_ptr)

    convert_obj = operator_new(0x188)        // 392 bytes for ConvertClass
    *convert_addr = ConvertClass__Constructor(palette_buf, palette_buf, DAT_00887310, 1, 0)
```

**Key detail: each 8-bit PAL component is shifted left by 2 (multiplied by 4).**
The raw PAL file stores values in range 0–63 (6-bit), the shift expands them to 0–252
(8-bit, stops at 252 not 255 because `63 << 2 == 252`). This is the standard C&C PAL
format. The ConvertClass is constructed with the main DirectDraw surface (`DAT_00887310`)
as the target.

---

## 7. Caller Chain

```
InitSideMixFiles (0x00534fa0)
    → PaletteLoad (0x0072f350)
        → FUN_00730100          // free left-panel SHPs
        → FUN_00730400          // free 5 ConvertClass objects
        → FUN_00730530          // free 16 layout rect globals
        → MIX_LoadNeutral (0x0072fa10)   // reload SHPs
        → [side-branching LoadPal calls via 0x0072ade0]
        → LeftPanel__ComputeLayoutRects (0x0072fc60)
```

`InitSideMixFiles` (verified via `get_function_callers 0x0072f350`) is the ONLY caller.
It passes side index as `param_1`; Yuri (2) is mapped to Soviet (1) for MIX selection,
but then `PaletteLoad` reads the LIVE ScenarioClass+0x34B8 field directly (not param_1),
so it checks for `== 2` on the actual stored side value.

FUN_0072aa40 (loads DIALOG/DIALOGY/DIALOGN/MAINBTTN) is called from game init
(`0x0052ba60`) — only once at startup, not per side switch.

---

## 8. Lazy-Init Guard: `DAT_00b0fc0c`

`PaletteLoad` sets `DAT_00b0fc0c = 0` at entry and `= 1` at the end (after
`LeftPanel__ComputeLayoutRects`). `LeftPanel__Draw` (`0x0072f540`) checks this flag:

```c
if (DAT_00b0fc0c == '\0') {
    MIX_LoadNeutral();
    FUN_0072fbc0();                       // palette-only reload (no teardown)
    LeftPanel__ComputeLayoutRects();
    DAT_00b0fc0c = '\x01';
}
// proceed with drawing
```

This means `LeftPanel__Draw` has its OWN lazy-init that re-runs if the flag is 0.
`PaletteLoad` deliberately clears it first so the teardown order is safe:
- `PaletteLoad` tears down, then reloads, then sets flag=1
- If `LeftPanel__Draw` fires while flag=0 (impossible in normal flow since PaletteLoad
  sets flag=1 before returning), it uses `FUN_0072fbc0` (palette-only reload, no
  teardown) not the full PaletteLoad path.

`FUN_0072fbc0` (`0x0072fbc0`) is a lighter reload that performs only the side-branch
ConvertClass construction, without tearing down SHPs or layout rects.

---

## 9. Teardown Functions

### FUN_00730100 (`0x00730100`) — Free left-panel SHP globals

Frees 16 SHP globals with lazy-loaded flag check pattern: (corrected 2026-05-29: was "14"; binary decompile_function 0x00730100 shows exactly 16 SHP globals freed — OPERATOR_OR_ORDER_DRIFT / off-by-count)
```
if (flag_byte != 0 && shp_ptr != 0): free(shp_ptr); flag_byte = 0
```
Frees: `DAT_00b0f9ec`, `g_BKGDSM_SHP`, `g_BKGDMD_SHP`, `g_SIDEBTTN_SHP`,
`g_RADAR_SHP`, `DAT_00b0f9e0`, `g_BKGDLG_SHP`, `g_CREDITS_SHP`, `DAT_00b0fafc`,
`DAT_00b0fa00`, `DAT_00b0fa8c`, `DAT_00b0fa48`, `DAT_00b0fa3c`, `DAT_00b0fa90`,
`DAT_00b0faa8`, `DAT_00b0fabc`.

### FUN_00730400 (`0x00730400`) — Free 5 ConvertClass palette globals

Pattern: if raw palette buf != 0, free it; call ConvertClass destructor (vtable slot 0,
arg=1), set ptr = 0. Frees exactly these 5 pairs in order:
`(0xb0fc04, 0xb0fc08)`, `(0xb0fbfc, 0xb0fc00)`,
`(0xb0fbf4, 0xb0fbf8)`, `(0xb0fbe4, 0xb0fbe8)`, `(0xb0fbec, 0xb0fbf0)`.

### FUN_00730530 (`0x00730530`) — Free 16 layout rect globals

Frees `DAT_00b0fc30` through `DAT_00b0fc6c` (every 4 bytes = 16 slots = the layout
rect globals produced by `LeftPanel__ComputeLayoutRects`).

### FUN_0072b230 (`0x0072b230`) — Free DIALOG/DIALOGY/DIALOGN/MAINBTTN palettes

Frees the 4 dialog palette pairs:
`(0xb0fb6c, 0xb0fb70)`, `(0xb0fb64, 0xb0fb68)`,
`(0xb0fb5c, 0xb0fb60)`, `(0xb0fb74, 0xb0fb78)`.
ConvertClass destructor called with arg=3 (not 1 — different from FUN_00730400).

---

## 10. Side-to-Palette Routing Summary

### For loading screen overlay (DIALOG group, selected in WM_PAINT_Handler):

| Side value | Palette | SHP |
|---|---|---|
| No game active | DIALOGN.PAL (`0xb0fb60`) | PUDLGBGN.SHP (`DAT_00b0fc80`) |
| 0 = Allied | **DIALOG.PAL** (`0xb0fb68`) | PUDLGBGA.SHP (`DAT_00b0fc84`) |
| 1 = Soviet | **DIALOG.PAL** (`0xb0fb68`) | PUDLGBGS.SHP (`DAT_00b0fc88`) |
| 2 = Yuri | DIALOGY.PAL (`0xb0fb70`) | PUDLGBGY.SHP (`DAT_00b0fc8c`) |

Allied and Soviet both use the same DIALOG.PAL ConvertClass — the visual difference is
only in the SHP art (`PUDLGBGA` vs `PUDLGBGS`), not the palette.

### For left panel / radar ConvertClass (PaletteLoad, side-branching):

| Side | `0xb0fbf0` | `0xb0fbf8` |
|---|---|---|
| Allied (0) | UIBKGD.PAL | SIDEBAR.PAL |
| Soviet (1) | UIBKGD.PAL | SIDEBAR.PAL |
| Yuri (2) | UIBKGDY.PAL | RADARYURI.PAL |

`0xb0fbe8` = SIDEBAR.PAL always (all sides).
`0xb0fc00` = OBSERVER.PAL always (all sides).
`0xb0fc08` = YRII.PAL always (all sides, despite the name).

---

## 11. DIALOG.PAL vs SIDEBAR_CONSTRUCTION §8 — Correction

The SIDEBAR_CONSTRUCTION doc §8 listed:
- `DAT_00b0fb68` = DIALOG.PAL "Allied sidebar variant"
- `DAT_00b0fb70` = DIALOGY.PAL "Soviet/Yuri sidebar variant"
- `DAT_00b0fb60` = DIALOGN.PAL "Neutral sidebar variant"

**Correction:** These palettes are used for the **loading screen** background SHP,
NOT for in-game sidebar chrome rendering. They are loaded once at startup (not per
side switch). The "Allied sidebar variant" label is inaccurate — Soviet also uses
DIALOG.PAL for its loading screen. The correct description is:
- DIALOG.PAL = loading screen palette for Allied + Soviet
- DIALOGY.PAL = loading screen palette for Yuri
- DIALOGN.PAL = loading screen palette when no game is active (menus)

---

## 12. Active in YR Analysis

| System | Active in YR | Notes |
|---|---|---|
| PaletteLoad (0x0072f350) | **Yes** — always | Called from InitSideMixFiles, no flag gate |
| DIALOG.PAL loading (0x0072aa40) | **Yes** — always | Called from game init, no flag gate |
| Yuri UIBKGDY/RADARYURI branch | **Yes, conditional** | Only when side == 2 |
| DIALOGN.PAL (menu state) | **Yes, conditional** | Only when FUN_0069bbe0 returns 0 |
| FUN_00730100/400/530 teardown | **Yes** | Called at start of PaletteLoad |

No TS-legacy gates detected — all paths are reachable in standard YR gameplay.

---

## 13. Key Function Reference

| Address | Name | Purpose |
|---|---|---|
| `0x0072f350` | PaletteLoad | Main subject — left-panel palette load + teardown |
| `0x0072ade0` | LoadPal_Inner | Loads one .PAL file → raw buf + ConvertClass |
| `0x0072aa40` | InitStartupPalettes | Loads DIALOG/DIALOGY/DIALOGN/MAINBTTN at startup |
| `0x0072b230` | FreeDialogPalettes | Frees the 4 dialog palette ConvertClasses |
| `0x00534fa0` | InitSideMixFiles | Only caller of PaletteLoad |
| `0x00621e90` | WM_PAINT_Handler | Selects DIALOG.PAL vs DIALOGY.PAL for loading screen |
| `0x0072aff0` | GetDialogPal | Returns DAT_00b0fb68 (DIALOG.PAL ConvertClass) |
| `0x0072b010` | GetDialogYPal | Returns DAT_00b0fb70 (DIALOGY.PAL ConvertClass) |
| `0x0072b030` | GetDialogNPal | Returns DAT_00b0fb60 (DIALOGN.PAL ConvertClass) |
| `0x0072b050` | GetMainBttnPal | Returns DAT_00b0fb78 (MAINBTTN.PAL ConvertClass) |
| `0x0072f4b0` | GetSidebarPal | Returns DAT_00b0fbe8 (SIDEBAR.PAL ConvertClass) |
| `0x0072f4d0` | GetFontPal | Returns DAT_00b0fc08 (YRII.PAL ConvertClass) |
| `0x0072f510` | GetRadarPal | Returns DAT_00b0fbf8 (SIDEBAR or RADARYURI ConvertClass) |
| `0x0072fbc0` | ReloadPalettesOnly | Light reload without teardown (LeftPanel lazy init) |
| `0x00730100` | FreeSHPs | Frees 14 left-panel SHP globals |
| `0x00730400` | FreeConvertClasses | Frees 5 ConvertClass palette globals |
| `0x00730530` | FreeLayoutRects | Frees 16 layout rect globals |

---

## 14. Open Questions — Final State

- `[RESOLVED] Q1 — What is FUN_0072f350's full decompilation?`
  → Full disassembly obtained at `0x0072f350`; it calls 3 teardown fns, MIX_LoadNeutral,
  5 side-branching LoadPal calls, LayoutRects. (evidence: `disassemble_function 0x0072f350`)

- `[RESOLVED] Q2 — Which side index triggers which palette?`
  → Side==2 (Yuri): UIBKGDY+RADARYURI in left-panel slots. Side!=2: UIBKGD+SIDEBAR.
  (evidence: `disassemble_function 0x0072f350` assembly, `read_memory 0x00844bf0`)

- `[RESOLVED] Q3 — What are the DIALOG.PAL/DIALOGY.PAL/DIALOGN.PAL loading paths?`
  → Not in FUN_0072f350. Loaded in FUN_0072aa40 at game startup. Selected in
  WM_PAINT_Handler (0x00621e90) for loading screen draws.
  (evidence: `decompile_function 0x0072aa40`, `disassemble_function 0x0072aa40`,
  `decompile_function 0x00621e90`)

- `[RESOLVED] Q4 — Do Allied (0) and Soviet (1) differ in palette selection?`
  → No difference in PaletteLoad (both use UIBKGD+SIDEBAR). In loading screen:
  both use DIALOG.PAL; they differ only in SHP art (PUDLGBGA vs PUDLGBGS).
  (evidence: `disassemble_function 0x0072f350`, `decompile_function 0x00621e90`)

- `[RESOLVED] Q5 — Which ConvertClass feeds DAT_0087f6cc and DAT_0087f6d0?`
  → These were from SIDEBAR_SYSTEM §35. They are NOT written by PaletteLoad or
  FUN_0072aa40. They are set in the game-init function 0x0052ba60 via multiple
  ConvertClass__Constructor calls. Not connected to the PaletteLoad system.
  (evidence: `decompile_function 0x0052ba60`)

- `[RESOLVED] Q6 — Who calls PaletteLoad and when?`
  → Sole caller: InitSideMixFiles (0x00534fa0), after side MIX files are mounted.
  (evidence: `get_function_callers 0x0072f350`)

- `[RESOLVED] Q7 — What does FUN_0072fbc0 do (reload sub-function)?`
  → Runs only the side-branching ConvertClass construction, without SHP or rect teardown.
  Called by LeftPanel__Draw lazy-init. (evidence: `decompile_function 0x0072fbc0`)

- `[RESOLVED] Q8 — What does FUN_00730400 do (free ConvertClasses)?`
  → Frees exactly 5 ConvertClass global pairs by calling destructor with arg=1 and
  freeing raw palette buffer. (evidence: `decompile_function 0x00730400`)

- `[RESOLVED] Q9 — What does FUN_00730100 do (free SHPs)?`
  → Frees 14 left-panel SHP globals with lazy flag check.
  (evidence: `decompile_function 0x00730100`)

- `[RESOLVED] Q10 — What does FUN_00730530 do?`
  → Frees 16 layout rect globals (0xb0fc30..0xb0fc6c).
  (evidence: `decompile_function 0x00730530`)

- `[RESOLVED] Q11 — What is the PAL file format (raw bytes → palette buf)?`
  → 256×3 bytes, each component × 4 (left-shift 2). Max component value = 252 (not 255).
  ConvertClass allocated as 0x188 bytes. (evidence: `decompile_function 0x0072ade0`)

- `[RESOLVED] Q12 — Which SHPs use which palette ConvertClass?`
  → 0xb0fbe8 (SIDEBAR.PAL): left panel SHPs in LeftPanel__Draw + button type 2.
     0xb0fbf8 (SIDEBAR/RADARYURI): radar draw functions via FUN_0072f510.
     0xb0fc08 (YRII.PAL): font palette via FUN_0072f4d0.
  (evidence: xrefs to 0xb0fbe8, 0xb0fbf8, 0xb0fc08 via `get_xrefs_to`)

- `[DEFERRED] Q13 — What exact SHPs are drawn using the DIALOG.PAL palette as remap?`
  (category: out-of-scope; reason: DIALOG.PAL feeds loading screen CC_Draw_Shape call,
  full SHP list for loading screen is outside PaletteLoad scope;
  next-step-if-pursued: trace CC_Draw_Shape call site in WM_PAINT_Handler mode==2)

- `[DEFERRED] Q14 — Which ConvertClass feeds DAT_0087f6cc / DAT_0087f6d0 (cameo chrome)?`
  (category: requires-different-system-context; reason: these are set by game init
  0x0052ba60 with separate palette loads not connected to this function;
  next-step-if-pursued: trace ConvertClass__Constructor calls in 0x0052ba60)

---

## Sources

- `disassemble_function 0x0072f350` — primary assembly map for PaletteLoad
- `decompile_function 0x0072ade0` — LoadPal_Inner mechanics (PAL format, ConvertClass)
- `decompile_function 0x0072aa40` + `disassemble_function 0x0072aa40` — DIALOG palette loading
- `decompile_function 0x00621e90` — WM_PAINT_Handler loading screen palette selection logic
- `decompile_function 0x0072f540` — LeftPanel__Draw lazy-init + ConvertClass usage
- `decompile_function 0x0072fbc0` — light palette reload sub-function
- `decompile_function 0x00730100/400/530` — teardown functions
- `decompile_function 0x00534fa0` — InitSideMixFiles (caller)
- `decompile_function 0x0052ba60` — game init (for DAT_0087f6cc context)
- `decompile_function 0x0072aff0/b010/b030/b050/f4b0/f4d0/f510` — accessor functions
- `get_function_callers 0x0072f350` — confirmed sole caller
- `get_xrefs_to 0x00b0fbe8/fbf0/fbf8/fc00/fc08` — ConvertClass consumers
- `get_xrefs_to 0x00b0fb68/70/60` — DIALOG palette ConvertClass consumers
- `read_memory 0x00844bf0` (16 bytes) — palette filename pointer table
- `read_memory 0x00844b9c` (16 bytes) — DIALOG group filename pointer table
- `search_strings "DIALOG"` + `"\.PAL"` — palette filename string locations
- SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md §8 (partially corrected by this report)
- SIDEBAR_SYSTEM_GHIDRA_REPORT.md §35 (DAT_0087f6cc/d0 context)
