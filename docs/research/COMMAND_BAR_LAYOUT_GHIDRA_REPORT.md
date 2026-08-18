# CommandBar::Layout — Ghidra Research Report

**Target:** `FUN_006d0fd0` — CommandBar button position layout  
**Function body:** `0x006d0fd0` – `0x006d1128` (~344 bytes)  
**Date:** 2026-05-19  
**Status:** COMPLETE  

---

## 1. Function Identity and Callers

- **Address:** `0x006d0fd0` (verified via `get_function_by_address 0x006d0fd0`)
- **Single caller:** `FUN_006d03a0` @ `0x006d03a0` (verified via `get_function_callers 0x006d0fd0`)
  - `FUN_006d03a0` is the **CommandBar initializer**; it calls `SidebarClass__Init`, then `FUN_0072f430()` (checks game-not-map-editor), then calls `FUN_006d0fd0()`, then positions the two thumb gadgets (IDs `0xF0` and `0xF1`).
  - `FUN_006d03a0` is itself called from: the **video mode change** function (`FUN_00560bf0`) and the **load-game** function (`FUN_0067e440`) — both after sidebar surfaces are rebuilt.

**Active in YR:** Yes — called unconditionally when `g_IsMapEditor == 0` and `FUN_0072f430()` (IsInGame check) returns non-zero.

---

## 2. Phase 0 — Strip Gadget Teardown

```c
puVar6 = &DAT_00b0c1c0;          // first strip-data entry
do {
    FUN_0069de00(0, 0, 0);       // detach / zero this strip's SHP
    puVar6 += 0x18;              // advance by 0x60 bytes (0x18 int-words)
} while ((int)puVar6 < 0xb0cb20);
```

- Iterates all 25 strip entries at `DAT_00b0c1c0`, stride **0x60 bytes** (`0x18 × 4`).
- `FUN_0069de00` zeroes the gadget's SHP pointer and clears its cached width/height fields (`+0x14`, `+0x18`).
- End sentinel `0xb0cb20 = 0xb0c1c0 + 25 × 0x60`.

**Active in YR:** Yes — unconditional.

---

## 3. Phase 1 — Per-Button X Position Assignment (the layout loop)

```c
iVar4 = *DAT_00b0fc64;      // DAT_00b0fc64[0] = X origin of LENDCAP (bottom-bar left anchor)
iVar8 = 0;                  // loop counter i = 0..24
iVar1 = DAT_00b0fc64[2];    // DAT_00b0fc64[2] = LENDCAP width (pixels)

do {
    iVar3 = (&DAT_00b0cb78)[iVar8];   // command ID for slot i (0..24 range check)
    if ((-1 < iVar3) && (iVar3 < 0x19)) {
        iVar7      = iVar3 * 0x60;    // byte offset into strip-data array
        piVar5     = &DAT_00b0c1c0 + iVar3 * 0x18;   // pointer to strip entry

        // X = LENDCAP_startX + i * button_pitch
        iVar3 = iVar3 * DAT_00b0cc38 + *DAT_00b0fc68;

        // clip: only place if button fits within LENDCAP rect
        if (DAT_00b0fc68[2] + iVar3 <= iVar1 + iVar4) {

            // set gadget ID (0x80D6 + i)
            *(int *)(&DAT_00b0c1e4 + iVar7) = iVar8 + 0xd6;   // 0xd6 = 214; IDs 0x80D6..0x80EE

            // set color scheme / palette
            *(undefined4 *)(&DAT_00b0c210 + iVar7) = DAT_0087f6cc;

            // clear flags
            (&DAT_00b0c1ed)[iVar7] = 0;
            *(undefined4 *)(&DAT_00b0c1f0 + iVar7) = 0;
            *(undefined4 *)(&DAT_00b0c1e0 + iVar7) = 5;     // gadget state = 5

            // vtable[25]: SetPosition(x=iVar3, y=uVar2)   where uVar2 = DAT_00b0cc24 = bottom-bar Y
            (**(code **)(*piVar5 + 100))(iVar3, uVar2);

            // vtable[34]: SetSHP(shp_ptr, 0, 0)
            (**(code **)(*piVar5 + 0x88))((&DAT_00b0c148)[iVar8], 0, 0);
        }
    }
    iVar8++;
} while (iVar8 < 0x19);  // 0x19 = 25
```

### X formula (decoded)
```
button_X[i] = *DAT_00b0fc68 + (slot_index * DAT_00b0cc38)
```

Where:
- `*DAT_00b0fc68` = X origin of the RENDCAP/button-tile zone (a rect: x, y, w, h stored at `DAT_00b0fc68`)
- `DAT_00b0cc38` = **button pitch** (= `BTTNBKGD_tile_width` = `DAT_00b0fc68[2] - _DAT_00b0cd38 + _DAT_00b0cd38` = `SHP_header.width[BTTNBKGD]`)
  - Set in `FUN_006d02b0`: `DAT_00b0cc38 = _DAT_00b0cd10 + _DAT_00b0cd38` where `_DAT_00b0cd10 = DAT_00b0fc68[2] - _DAT_00b0cd38` and `_DAT_00b0cd38 = *(short*)(DAT_00b0c148 + 2)` (width of first Button SHP)
  - Net result: `DAT_00b0cc38 = DAT_00b0fc68[2]` = BTTNBKGD tile width (from SHP header, **not hardcoded**)
- `slot_index` = `(&DAT_00b0cb78)[i]` — the command-slot mapping for button i (remappable)

### Y value
```
Y = DAT_00b0cc24 = *(int*)(DAT_00b0fc58 + 4)
```
`DAT_00b0fc58` is a layout rect allocated in `LeftPanel__ComputeLayoutRects`:
```c
*DAT_00b0fc58   = 0;
DAT_00b0fc58[1] = param_2 - 0x20;   // param_2 = screen_height → Y = screen_height - 32
DAT_00b0fc58[2] = iVar6;             // width = sidebar_width - 0xa8
DAT_00b0fc58[3] = 0x20;             // height = 32
```
So `DAT_00b0cc24 = screen_height - 32`. **Y is screen_height − 32 (= 0x20).**

**Active in YR:** Yes — unconditional within the loop (no `IsOpen` branch).

---

## 4. Alignment Anchor

`*DAT_00b0fc68` (X origin of the button tile zone) is set in `LeftPanel__ComputeLayoutRects`:
```c
*DAT_00b0fc68 = _DAT_00b0fad8;
_DAT_00b0fad8 = *DAT_00b0fc6c - DAT_00b0fc60[2] - *DAT_00b0fc60;
```
And RENDCAP position:
```c
*DAT_00b0fc6c = *DAT_00b0fc34 - DAT_00b0fc6c[2];   // RENDCAP_X = right_panel_X - RENDCAP_width
```
Where `*DAT_00b0fc34` = `screen_width - RADAR_width`.

The chain resolves to:
```
button_tile_zone_X = RENDCAP_X - RENDCAP_width - LENDCAP_width - (N_tiles * tile_width)
```
Buttons start at `LSPACER_width + LENDCAP_width` (right of the left caps), confirmed by `_DAT_00b0fad8` = LENDCAP right edge = `LENDCAP_startX + LENDCAP_width + LENDCAP_spacing_correction`.

Buttons are placed left-to-right from this anchor, each advancing by `DAT_00b0cc38` (= BTTNBKGD tile width from SHP header).

**Active in YR:** Yes.

---

## 5. Phase 2 — Mark Selected Button (post-loop)

```c
iVar4 = (&DAT_00b0cb78)[DAT_00b0cc1c];   // DAT_00b0cc1c = currently selected command index
if ((0 <= iVar4) && (iVar4 < 0x19) && (iVar4 * 0x60 != -0xb0c1c0)) {
    *(undefined4*)(&DAT_00b0c1f0 + iVar4*0x60) = 1;    // set IsSelected flag
    (&DAT_00b0c200)[iVar4*0x60] = 1;                   // set extra selected flag
}
```
Marks the strip entry for the currently-selected command slot as "pressed/selected."

---

## 6. Phase 3 — Mark Available Command Range

```c
if (DAT_00b0cc20 <= DAT_00b0cd28) {
    // DAT_00b0cc20 = first available command, DAT_00b0cd28 = last available command
    for (iVar4 = DAT_00b0cc20; iVar4 <= DAT_00b0cd28; iVar4++) {
        iVar1 = (&DAT_00b0cb78)[iVar4];
        if ((0 <= iVar1) && (iVar1 < 0x19) && (iVar1 * 0x60 != -0xb0c1c0)) {
            *(undefined4*)(&DAT_00b0c1e0 + iVar1 * 0x60) = 0x55;   // state = 0x55 (available/enabled)
        }
    }
}
```
Sets the enabled-state field for buttons in the available range to `0x55`.

---

## 7. IsOpen Branch — Not Present

`FUN_006d0fd0` does **not** read or branch on `SidebarClass + 0x5544` (IsOpen).  
The function always tears down and re-lays-out all 25 strips regardless of sidebar state.  
The `IsOpen` flag at `+0x5544` is consumed by `FUN_006d1200` (FullRelayout, out of scope) — confirmed by xref scan on `DAT_00b0cc20` and `DAT_00b0cd28`, which are both read inside `FUN_006d1200` but not gated inside `FUN_006d0fd0`.

**Active in YR:** N/A (branch does not exist in this function).

---

## 8. Key Data Address Summary

| Address | Role | Verified via |
|---|---|---|
| `DAT_00b0c1c0` | Strip-data array base (25 × 0x60) | `decompile_function 0x006d0fd0` |
| `DAT_00b0c148` | Button SHP pointer array (25 entries) | `decompile_function 0x006d0fd0` |
| `DAT_00b0cb78` | Command-slot mapping array (25 entries) | `decompile_function 0x006d0fd0` |
| `DAT_00b0fc64` | Rect: {x=LENDCAP_startX, y=bar_Y, w=LENDCAP_w, h=LENDCAP_h} | `decompile_function 0x0072ff00` |
| `DAT_00b0fc68` | Rect: {x=button_tile_zone_X, y=bar_Y, w=tile_w, h=tile_h} | `decompile_function 0x0072ff00` |
| `DAT_00b0fc58` | Rect: {x=0, y=screen_h−32, w=bar_w, h=32} — bottom bar dims | `decompile_function 0x0072ff00` |
| `DAT_00b0cc24` | Cached Y = screen_height − 32 (copied from fc58[1]) | `decompile_function 0x006d02b0` |
| `DAT_00b0cc38` | Button pitch = BTTNBKGD tile width from SHP header | `decompile_function 0x006d02b0` |
| `DAT_00b0cc1c` | Selected command index | `get_xrefs_to 0x00b0cc1c` |
| `DAT_00b0cc20` | First available command index | `decompile_function 0x006d0fd0` |
| `DAT_00b0cd28` | Last available command index | `decompile_function 0x006d0fd0` |

---

## 9. Open Questions

- **FUN_006d1200** (FullRelayout / open-path re-add) — **out of scope per brief; do not expand here.**

---

## 10. Five Most Load-Bearing Verified Facts

1. **Y = screen_height − 32 always.** `DAT_00b0cc24` is set from `DAT_00b0fc58[1]` which is `param_2 − 0x20` in `LeftPanel__ComputeLayoutRects` (param_2 = screen_height). Verified: `decompile_function 0x0072ff00`, `decompile_function 0x006d02b0`.

2. **X spacing is SHP-driven, not hardcoded.** `DAT_00b0cc38` = BTTNBKGD tile width read from the Button SHP header (short at offset +2). Verified: `decompile_function 0x006d02b0`.

3. **Layout anchor = button-tile-zone X, not screen X=0.** The loop starts X at `*DAT_00b0fc68` (the tile-zone left edge, right of LSPACER+LENDCAP caps), advancing per slot by `slot_index × DAT_00b0cc38`. Verified: `decompile_function 0x006d0fd0`, `decompile_function 0x0072ff00`.

4. **No IsOpen branch in this function.** `FUN_006d0fd0` unconditionally tears down and repositions all 25 strips. The expand/collapse split lives in FUN_006d1200. Verified: full body `decompile_function 0x006d0fd0` — no read of +0x5544.

5. **Single caller: `FUN_006d03a0` (CommandBar initializer).** Called on video-mode-change and game-load — not per-tick. Verified: `get_function_callers 0x006d0fd0`, `decompile_function 0x006d03a0`.
