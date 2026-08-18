# UIMD.INI [Art_800] UI Art Loader — Ghidra Research Report

**Target:** FUN_007681e0 (`0x007681e0`–`0x00768f4f`)  
**Mislabel:** Ghidra RTTI labeler tagged this as `CDFileClass__Constructor` — **incorrect**.  
  The actual function is the UI layout/art loader that reads UIMD.INI section [Art] or [Art_800].  
**Date:** 2026-05-19  
**Session:** /re-swarm slot 5, allied-sidebar  

Verification calls cited inline. All offsets are direct byte offsets — `param_1` is `int`
(not `int *`), so `*(int *)(param_1 + 0x44)` is a direct byte offset of 0x44.

---

## 1. Function Identity

| Field | Value |
|---|---|
| Address | `0x007681e0` |
| Body end | `0x00768f4f` |
| Ghidra name | `CDFileClass__Constructor` (WRONG — label collision) |
| Real purpose | UIMD.INI UI-layout reader + art asset loader for the help-bar / sidebar panel |
| Call convention | `__thiscall` (param_1 = `this` pointer to UI panel class instance) |

Verified via `decompile_function 0x007681e0`. The function body contains every INI key
listed in §4 below, plus the screen-width resolution branch.

---

## 2. Caller Chain

```
InitSideMixFiles (0x00534fa0 / 0x005352f8)
  → loads UIMD.INI (CCFileClass__Constructor "UIMD.INI")
  → RulesClass__ReadCommandBar (0x00674650)    ← command-bar keys (different subtarget)
  → FUN_006d02b0                               ← SHP loader + layout init
      → SidebarClass__LoadSHPs
      → [indirect] FUN_007681e0                ← THIS FUNCTION
          caller xref: 0x0076c5a7 inside CDFileClass__Constructor (0x0076c290)
```

Direct caller confirmed via `get_xrefs_to 0x007681e0`: called from `0x0076c5a7`
(inside the function body at `0x0076c290`).

**Active in YR:** Yes — `InitSideMixFiles` is invoked during normal skirmish startup.  
UIMD.INI is loaded from the side MIX file (`SIDENC_0x_MIX`), not from a mod flag.

---

## 3. Resolution-Based Section Switch (Art vs Art_800)

```c
// Pseudocode reconstructed from decompile_function 0x007681e0

// Attempt to open the [Art_800] section
iVar3 = try_open_ini_section("Art_800");   // FUN_007b63c0 + FUN_00528c00

if ((screen_width > 800) && (iVar3 < 1)) {
    // Width > 800 but Art_800 section wasn't found yet — activate it
    activate_section("Art_800");
}
// Otherwise remain in the default unnamed/empty section (equivalent to [Art])
```

**String addresses verified via `inspect_memory_content`:**
- `"Art_800"` at `0x00848b24` — confirmed 7 bytes, null-terminated  
  (xref `0x0076831f` inside FUN_007681e0 body — verified via `get_xrefs_to 0x00848b24`)
- Memory at `0x00848b2c`: `"Art_\0"` — appears to be a partial string literal for the
  default section name; however, this address is **not directly referenced** in the
  decompile — the default section path uses `&DAT_00889f64` (an all-zero global),
  which the INI stack machinery treats as the unnamed/root section.

**Threshold:** strict `>` (greater than), not `>=`. Width = 800 uses the default section.  
**Default section:** Unnamed (NULL pointer passed to section-push helper), which in UIMD.INI
corresponds to the `[Art]` section (UNVERIFIED — no explicit `"Art"` string seen; the NULL
sentinel is the INI machinery's way of referencing the top-level / unnamed section context).

**Screen width source:** fetched dynamically via vtable call on the screen surface object
at `DAT_0088730c`: `(**(code **)(*DAT_0088730c + 0x80))()` = GetHeight,
`(**(code **)(*DAT_0088730c + 0x7c))()` = GetWidth.  
Not a global integer — it's a virtual call on the surface/display manager.

---

## 4. INI Keys Enumerated

All keys are read from the active section (either `[Art]` or `[Art_800]` per §3).
`param_1` is the `this` pointer of the UI panel class. All struct offsets are **direct byte offsets**.

Helper functions:
- `FUN_00529a30` = **ReadPoint/ReadSize** — parses `"%d %d"` via sscanf into two ints  
  (verified via `decompile_function 0x00529a30`: calls `CRT__sscanf` with `s__d__d_0081c000`)
- `FUN_00527f20` = **ReadRect** — parses `"%d %d %d %d"` via sscanf into four ints  
  (verified via `decompile_function 0x00527f20`: calls `CRT__sscanf` with `s__d__d__d__d_00825bbc`)
- `FUN_00528c00` = **ReadString** — returns string value, length in return value  
  (verified via `decompile_function 0x00528c00`: returns string length as `int`)
- `FUN_005278f0` = **ReadInt** — wraps `CCINIClass__ReadInt`  
  (verified via `decompile_function 0x005278f0`)

### 4.1 Sizing Keys

| # | Key string | Addr | Helper | Default | Dest (struct offset) | Notes |
|---|---|---|---|---|---|---|
| 1 | `Size` | `0x00820178` | ReadPoint | `0x280, 400` (640×400) | `local_94` (x), `local_90` (y) | Read first from unnamed section; NOT written into param_1 — stored in locals, used for centering math |
| 2 | `SideBarSize` | `0x00848b18` | ReadPoint | `0x0, 0` | `local_7c`, `local_78` | Width/height of sidebar SHP; used for X centering: `(screen_w - local_7c - local_94) / 2` |
| 3 | `HelpBarSize` | `0x00848b0c` | ReadPoint | `0x0, 0` | `local_74`, `local_70` | Width/height of help bar SHP; used for Y centering: `(screen_h - local_70 - local_90) / 2` |

**Centering math verified from decompilation:**
```c
x_off = ((screen_width  - local_7c) - local_94) / 2;  // → param_1+0x44 and +0x4c
y_off = ((screen_height - local_70) - local_90) / 2;  // → param_1+0x48
param_1+0x50 = y_off + local_90;  // bottom of top panel
```

### 4.2 Rectangle Keys (ReadRect — 4 ints: x, y, w, h)

Rect values are relative; after reading, X is offset by `param_1+0x44` and Y by `param_1+0x48`.

| # | Key string | Addr | Dest (struct offsets) | Notes |
|---|---|---|---|---|
| 4 | `TextRect` | `0x008302f0` | `param_1+0x7c/0x80/0x84/0x88` | Main text area rect; X += x_off, Y += y_off |
| 5 | `TooltipRect` | `0x00848b00` | `param_1+0x8c/0x90/0x94/0x98` | Tooltip display rect; X += x_off, Y += y_off |
| 6 | `TitleRect` | `0x00848af4` | `param_1+0x9c/0xa0/0xa4/0xa8` | Title bar rect; X += x_off, Y += y_off |
| 7 | `BackTextRect` | `0x00848ae4` | `param_1+0xac/0xb0/0xb4/0xb8` | Back-button text rect; X += x_off, Y += y_off |
| 8 | `BackButtonRectangle` | `0x00848ab8` | `param_1+0x54/0x58/0x5c/0x60` | Back button click rect (not offset-adjusted at read time) |

String addresses verified via `search_strings` and `inspect_memory_content`.

### 4.3 String / SHP Art Keys (ReadString — returns filename)

| # | Key string | Addr | Dest / Purpose | Notes |
|---|---|---|---|---|
| 9 | `Opening` | `0x00848adc` | `auStack_b8` → MSAnim constructor | Opening animation SHP; only loaded if `FUN_007b54b0() != 0` (multiplayer check?) → `param_1+0x18` |
| 10 | `Background` | `0x008241b4` | `auStack_b8` → CDFileClass::Constructor | Background SHP → `param_1+0x1c` |
| 11 | `ClickMap` | `0x008304b4` | `auStack_b8` → CCFileClass::Constructor → BSurface | Click-map surface → `param_1+0x24` |
| 12 | `SideBar` | `0x00848ad4` | `auStack_b8` → CDFileClass::Constructor | Sidebar panel SHP; only loaded if `FUN_007b54b0() != 0`; Y-centered |
| 13 | `HelpBar` | `0x00848acc` | `auStack_b8` → CDFileClass::Constructor | Help bar SHP; only loaded if `FUN_007b54b0() != 0` → `param_1+0x20` |
| 14 | `BackButton` | `0x00848a40` | `auStack_b8` → MSShapeAnim::Constructor | Back button SHP anim → `param_1+0x64` (index 100) |
| 15 | `BackButtonHighlighted` | `0x00848a28` | (local; no direct param_1 write seen) | Highlighted state SHP |
| 16 | `BackButtonPalette` | `0x00848a80` | `auStack_b8` → CDFileClass::Constructor | Palette for back-button SHP → `param_1+0x78` |
| 17 | `OverlayPrefix` | `0x00848a18` | `param_1+0x28` (string field) | Prefix string for overlay SHP filenames |
| 18 | `OverlayPalette` | `0x00848a08` | `auStack_b8` → CDFileClass::Constructor → `param_1+0x14` | Overlay palette |
| 19 | `DefaultPalette` | `0x008489e0` | `auStack_b8` → `FUN_007691e0` → palette load | Default palette; conditional: only if `0 < iVar3` (ReadString returned > 0 chars) |
| 20 | `NameKeyPrefix` | `0x008489f0` | `auStack_88` (local string context) | Prefix for named-key lookup (used in the loop below) |

### 4.4 Integer / Frame Keys (ReadInt)

| # | Key string | Addr | Dest (struct offset) | Notes |
|---|---|---|---|---|
| 21 | `BackButtonNormalFrame` | `0x00848a68` | `param_1+0x6c` | Frame index for normal state |
| 22 | `BackButtonDepressedFrame` | `0x00848a4c` | `param_1+0x70` | Frame index for pressed state |

Verified via `decompile_function 0x005278f0` — wraps `CCINIClass__ReadInt`.

### 4.5 Point Key (ReadPoint)

| # | Key string | Addr | Dest | Notes |
|---|---|---|---|---|
| 23 | `BackButtonOrigin` | `0x00848aa4` | `iStack_84`, `iStack_80` (locals → used in MSShapeAnim constructor) | Origin point for back button placement |

### 4.6 Named-Key Loop (up to 100 entries)

After all the above, a loop runs `0..100` times:
```c
do {
    // Build key from NameKeyPrefix + index via FUN_007b6320(2)
    // FUN_007b63c0 appends the section
    // FUN_00528c00 tries to read the key value
    if (found) {
        FUN_0076f910(param_2, value, param_1+0x28, auStack_88);  // add named entry
        ...
    }
    ...
} while (i < 100);
```
This is a variadic named-entry system driven by `NameKeyPrefix`. It reads keys like
`NameKeyPrefix_0`, `NameKeyPrefix_1`, …, `NameKeyPrefix_99` (exact key naming depends on
`FUN_007b6320(2)` — format string generator).

---

## 5. Struct Offsets Summary (param_1 = UI panel `this`)

| Offset | Field | Written by |
|---|---|---|
| `+0x14` | Overlay palette ptr | `DefaultPalette` / CDFileClass |
| `+0x18` | Opening anim ptr | `Opening` key |
| `+0x1c` | Background SHP ptr | `Background` key |
| `+0x20` | HelpBar SHP ptr | `HelpBar` key |
| `+0x24` | ClickMap surface ptr | `ClickMap` key |
| `+0x28` | OverlayPrefix string | `OverlayPrefix` key |
| `+0x44` | x_off (horizontal center offset) | Computed from `SideBarSize` |
| `+0x48` | y_off (vertical center offset) | Computed from `HelpBarSize` |
| `+0x4c` | x_off copy (right panel) | = `+0x44` |
| `+0x50` | y_off + top_panel_height | = `+0x48` + `Size.y` |
| `+0x54..0x60` | BackButtonRectangle (4 ints) | `BackButtonRectangle` key |
| `+0x6c` | BackButtonNormalFrame | `BackButtonNormalFrame` key |
| `+0x70` | BackButtonDepressedFrame | `BackButtonDepressedFrame` key |
| `+0x78` | BackButton palette ptr | `BackButtonPalette` key |
| `+0x7c..0x88` | TextRect (4 ints) | `TextRect` key |
| `+0x8c..0x98` | TooltipRect (4 ints) | `TooltipRect` key |
| `+0x9c..0xa8` | TitleRect (4 ints) | `TitleRect` key |
| `+0xac..0xb8` | BackTextRect (4 ints) | `BackTextRect` key |

---

## 6. `sdbtnanm.pal` Literal

The key string `"sdbtnanm.pal"` at `0x00848a94` is pushed **as a section name context**
(not as a key name) immediately before the `BackButtonPalette` read. This appears to
establish a palette search context (side-palette MIX lookup) rather than reading an INI key.
Verified via `search_strings` — three hits: `SDBTNANM.SHP` (0x00845178),
`SDBTNANM.PAL` (0x00845438), and `sdbtnanm.pal` (0x00848a94, lowercase, UIMD.INI context).

---

## 7. Dead / Legacy Key Assessment

| Key | Status |
|---|---|
| `BackButtonHighlighted` | Read but no visible `param_1` write in decompile — value may be unused or written via a local that was optimized away. Flag as **potentially dead** — UNVERIFIED. |
| `Opening` | Gated behind `FUN_007b54b0() != 0`. If this is a "multiplayer only" check, `Opening` animation may be inactive in single-player skirmish. Caller context unverified. |
| `SideBar`, `HelpBar` | Same gate as `Opening`. Active in YR: Conditional on `FUN_007b54b0()`. |
| Named-key loop | Active in YR: Yes — loop runs unconditionally. Number of entries that actually exist in UIMD.INI is INI-file-dependent. |

---

## 8. Open Questions

1. **Default section name** — The NULL/empty sentinel (`&DAT_00889f64`) used when width ≤ 800
   corresponds to what section in UIMD.INI? Most likely `[Art]` but not confirmed from binary —
   would require reading the actual UIMD.INI file to verify.

2. **`FUN_007b54b0()`** — What does this return? Appears to gate `Opening`, `SideBar`, `HelpBar`
   loading. Likely a multiplayer-vs-skirmish check. Not decompiled in this session.

3. **`BackButtonHighlighted`** — Read from INI but write destination not visible in decompile.
   May be consumed by the `MSShapeAnim::Constructor` call or stored in a register not
   captured in the pseudocode.

4. **Named-key loop format** — `FUN_007b6320(2)` generates the key suffix. Not decompiled;
   unclear whether format is `%d`, `%02d`, or alphanumeric.

5. **`Size` key default** — Default is `local_94=0x280` (640d), `local_90=400`.
   This suggests the default layout target is 640×400, not 640×480 — worth verifying against
   UIMD.INI values.

---

## 9. Confidence Summary

| Claim | Confidence | Basis |
|---|---|---|
| All 23 key names | HIGH | `search_strings` + `inspect_memory_content` on each address |
| Helper = ReadRect (4 ints) | HIGH | `decompile_function 0x00527f20` — sscanf `%d %d %d %d` confirmed |
| Helper = ReadPoint (2 ints) | HIGH | `decompile_function 0x00529a30` — sscanf `%d %d` confirmed |
| Helper = ReadString | HIGH | `decompile_function 0x00528c00` — returns string length |
| Helper = ReadInt | HIGH | `decompile_function 0x005278f0` — wraps CCINIClass__ReadInt |
| Struct offsets (+0x44 etc.) | HIGH | Direct read from decompile, param_1 is `int` (not `int *`) |
| Art_800 threshold = `> 800` | HIGH | `if ((800 < g_ScreenWidth) && (iVar3 < 1))` |
| Screen width from vtable call | HIGH | vtable call at `*DAT_0088730c + 0x7c` in decompile |
| Default section = [Art] | UNVERIFIED | NULL sentinel inferred; no `"Art"` string found |
| BackButtonHighlighted write dest | UNVERIFIED | Read confirmed; write destination unclear |
| `FUN_007b54b0()` semantics | UNVERIFIED | Not decompiled; gating purpose unknown |
