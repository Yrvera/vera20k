# Sidebar Repair / Sell Buttons — Ghidra Report

**Status:** GREEN (live decompilation verification of gamemd.exe, 2026-05-20)
**Scope:** How gamemd.exe loads, positions, and renders the Repair (0x65) and Sell (0x66) gadget buttons that sit above the production tab strip.
**Companion to:** SIDEBAR_SYSTEM_GHIDRA_REPORT.md §1, §12, §18; SIDEBAR_INIT_GADGET_POSITIONING_GHIDRA_REPORT.md; SIDEBAR_CONSTRUCTION_GHIDRA_REPORT.md.

Every load-bearing address / offset / value below is followed by the Ghidra MCP call that produced it. Claims with no inline citation are restated from already-GREEN companion docs and labelled accordingly.

---

## 0. Executive summary (verdict-first)

1. **Palette:** Both buttons render through the global sidebar `ConvertClass` at `DAT_0087f6cc`. That ConvertClass is constructed from a 768-byte palette buffer at `DAT_00b0fbe4`, which `PaletteLoad`/`MIX_LoadPalettes` (`0x0072f350`) populates from **SIDEBAR.PAL**. **The OBSERVER.PAL claim in the in-repo TODO is wrong** — OBSERVER.PAL is a *different* slot (`DAT_00b0fbfc` / `DAT_00b0fc00`) used elsewhere.
2. **CC_Draw_Shape flag:** `0` normally, `0x800` only when the gadget's `+0x55` "highlight" byte is non-zero. The chrome's `0x400` flag is **not** used for these gadgets.
3. **Frame state machine:** five frames are consulted — `0` = normal, `1` = mode-active (Repair-mode / Sell-mode toggled on), `2` = disabled, `3` = mouse-down (mode inactive), `4` = mouse-down (mode active).
4. **Positions:** local-side-driven. Repair X = `SidebarX + 0x14`
   (Allied) / `SidebarX + 0x21` (Soviet or Yuri). Sell X = Repair X +
   `DAT_00b0b4e4` (=`0x40` Allied / `0x34` Soviet or Yuri). Repair Y =
   Sell Y = `g_SidebarWidth + 8` (Allied) / `g_SidebarWidth + 7`
   (Soviet or Yuri); `g_SidebarWidth` is the constant `0x9E = 158`.
5. **Side-specific art:** The filenames `REPAIR.SHP` and `SELL.SHP` are
   hard-coded in `SidebarClass::LoadSHPs` with no per-side filename branch.
   Which physical SHP wins is determined by the active side MIX search order,
   not by a filename branch in `LoadSHPs`.

---

## 1. Entry points and call chain

| Function | Address | Role |
|---|---|---|
| `SidebarClass::Init` | `0x006A5310` | Writes Repair gadget at `DAT_00b0b3a0` (`ID=0x65`) and Sell gadget at `DAT_00b07df8` (`ID=0x66`). |
| `SidebarClass::InitSidebarRect` | `0x006A5130` | Sets `DAT_00b0b4dc` (Repair X). |
| `SidebarClass::InitLayoutConstants` | `0x006A5090` | Sets `DAT_00b0b4e0` (Repair Y), `DAT_00b0b4e4` (Sell-X delta). |
| `SidebarClass::LoadSHPs` *(decomp labels it "LoadArt")* | `0x006A5840` | Loads `REPAIR.SHP` / `SELL.SHP` into the two gadgets via `FUN_0069de00`. Builds the sidebar `ConvertClass` at `DAT_0087f6cc`. |
| `SidebarClass::Draw` | `0x006A6C30` | Calls `SBGadgetClass::Draw` for Sell, then Repair, then tabs, then scroll arrows. |
| `SBGadgetClass::Draw` | `0x0069DEB0` | The actual gadget renderer; chooses the SHP frame and calls `CC_Draw_Shape`. |
| `SidebarClass::AI` | `0x006A7780` | Dispatches button events `0x8065` (Repair) → `FUN_004ac8c0` and `0x8066` (Sell) → `FUN_004ac660`. The former `Action` label was stale. |
| `CC_Draw_Shape` | `0x004AED70` | Confirmed via `get_function_by_address 0x004AED70`. |
| Repair-mode toggle | `0x004AC8C0` | Sets / clears the `+0x46c` byte on SidebarClass and the gadget `+0x2D` mirror. |
| Sell-mode toggle | `0x004AC660` | Sets / clears the `+0x11B1` byte on SidebarClass and the gadget `+0x2D` mirror. |

Verified via `get_function_by_address` and the `search_functions_enhanced` query on `SidebarClass__Init` / `SBGadgetClass`.

---

## 2. The SBGadgetClass instance — field map

The Repair gadget occupies `DAT_00b0b3a0` … `DAT_00b0b400`; the Sell gadget occupies `DAT_00b07df8` … `DAT_00b07e58`. The struct is ≈ `0x60` bytes.

Field map derived from `decompile_function 0x006A5310` (writes) and `decompile_function 0x0069DEB0` (reads):

| Offset | Type | Meaning | Init value (Repair) | Init value (Sell) |
|---|---|---|---|---|
| `+0x0C` | i32 | X | `DAT_00b0b4dc` | `DAT_00b0b3ac + DAT_00b0b4e4` |
| `+0x10` | i32 | Y | `DAT_00b0b4e0` | `DAT_00b0b3b0` (= Repair Y) |
| `+0x14` | i32 | SHP width  *(written by `FUN_0069de00`)* | derived from SHP header | derived |
| `+0x18` | i32 | SHP height *(written by `FUN_0069de00`)* | derived from SHP header | derived |
| `+0x1D` | u8  | always-1 byte (unknown purpose) | 1 | 1 |
| `+0x1E` | u8  | **disabled** flag → frame 2 | 0 | 0 |
| `+0x24` | u32 | Gadget ID | `0x65` | `0x66` |
| `+0x2C` | u8  | "highlight" hover (used only when `+0x40==0`) | 0 (init) | 0 |
| `+0x2D` | u8  | **mode-active** flag (toggled by repair/sell mode) | 0 | 0 |
| `+0x30` | u32 | unknown (set to 1) | 1 | 1 |
| `+0x40` | u32 | **pressable** flag (1 = button, 0 = static toggle) | 1 | 1 |
| `+0x44` | i32 | unknown X offset (set to `-480`) | `0xFFFFFE20` | `0xFFFFFE20` |
| `+0x48` | i32 | unknown Y offset (set to 0) | 0 | 0 |
| `+0x4C` | u32 | 1 ⇒ draw to `g_SidebarSurface`, 0 ⇒ primary | 1 | 1 |
| `+0x50` | `ConvertClass*` | **palette** (the per-gadget remap object) | `DAT_0087f6cc` | `DAT_0087f6cc` |
| `+0x54` | u8  | "draw happened" latch (set after draw) | — | — |
| `+0x55` | u8  | **0x800 / highlight** bit; pushes 0x800 flag when set | 0 (init) | 0 |
| `+0x58` | `void*` | SHP shape pointer (set by `FUN_0069de00` = `SBGadgetClass::SetShape`) | repair.shp | sell.shp |
| `+0x5C` | u8  | "loaded" flag set by `FUN_0069dea0` from CDFile success byte | 1 on success | 1 on success |

The `+0x34` byte is read by `SBGadgetClass::Draw` (frame logic) but is **not** initialised in `SidebarClass::Init`; it is written elsewhere as the transient "currently mouse-down on this gadget" bit.

> Verified via `decompile_function 0x006A5310` (Init writes) and `decompile_function 0x0069DEB0` + `disassemble_function 0x0069DEB0` (Draw reads). Caller chain via `get_function_callers 0x0069DEB0` (3 callers).

---

## 3. Palette resolution — answers Investigation Point #1

### 3.1 The path from SIDEBAR.PAL to the gadget

```
PaletteLoad      0x0072f350
  └── (table lookup [0x00844bf0] = "SIDEBAR.PAL")
  └── CALL 0x0072ade0
        – reads SIDEBAR.PAL into 768-byte buffer at DAT_00b0fbe4
        – constructs ConvertClass into DAT_00b0fbe8
SidebarClass::LoadSHPs   0x006A5840
  └── EAX = FUN_0072f4a0()          // returns DAT_00b0fbe4
  └── copies 0xC0 dwords (=768 B) of palette to stack
  └── operator_new(0x188)            // sizeof ConvertClass
  └── ConvertClass__Constructor(palette, palette, [0x00887308], 1, 0)
        → DAT_0087f6cc               // global sidebar ConvertClass
  └── For Sell:   DAT_00b07e48 = DAT_0087f6cc   (gadget+0x50)
  └── For Repair: DAT_00b0b3f0 = DAT_0087f6cc   (gadget+0x50)
SBGadgetClass::Draw   0x0069DEB0
  └── 0069df8f  MOV EDX, [ESI+0x50]  // EDX = gadget palette ConvertClass
  └── 0069df96  CALL CC_Draw_Shape   // EDX is the palette arg (__fastcall slot 2)
```

> SIDEBAR.PAL string verified via `search_strings("sidebar.pal") → {address: 0x0084542c}`.
> Table entry at `[0x00844bf0] → 0x0084542c` verified via `read_memory(0x00844bf0, 40)` returning `2c 54 84 00 …`.
> `PaletteLoad` body verified via `disassemble_function 0x0072f3d0` (the lookup at `0x0072f3ca` loads `[0x00844bf0]` into ECX before calling `0x0072ade0` with `EDX=0xb0fbe4`, `PUSH 0xb0fbe8`).
> `FUN_0072f4a0` body verified via `decompile_function 0x0072f4a0`: `return DAT_00b0fbe4;`.
> `LoadSHPs` constructs the global ConvertClass at `DAT_0087f6cc`: verified via `disassemble_function 0x006A5840` lines `006a5880 CALL operator_new(0x188)` and `006a58a8 CALL 0x0048e740` (ConvertClass ctor), result stored at `[0x0087f6cc]`.
> Gadget palette field at `+0x50` verified via `disassemble_function 0x006A5840` lines `006a5938 MOV [0x00b0b3f0], EDX` (Repair: `0xb0b3f0 - 0xb0b3a0 = 0x50`) and `006a590c MOV [0x00b07e48], EAX` (Sell: `0xb07e48 - 0xb07df8 = 0x50`).
> `SBGadgetClass::Draw` reads `+0x50` into EDX before calling `CC_Draw_Shape`: verified via `disassemble_function 0x0069DEB0` line `0069df8f MOV EDX, dword ptr [ESI + 0x50]`.

### 3.2 Sibling palette slots — what each is *actually* for

| Buffer slot | ConvertClass slot | Filename (string) | Selector address |
|---|---|---|---|
| `DAT_00b0fbe4` | `DAT_00b0fbe8` | **SIDEBAR.PAL** | `[0x00844bf0]` |
| `DAT_00b0fbec` | `DAT_00b0fbf0` | `UIBKGD.PAL` (side ≠ 2) / `UIBKGDY.PAL` (Yuri side 2) | `[0x00844bf4]` / `[0x00844bf8]` |
| `DAT_00b0fbf4` | `DAT_00b0fbf8` | `RADARYURI.PAL` / `SIDEBAR.PAL` (urban) | `[0x00844bfc]` / `[0x00844c00]` |
| `DAT_00b0fbfc` | `DAT_00b0fc00` | **OBSERVER.PAL** | `[0x00844c04]` |
| `DAT_00b0fc04` | `DAT_00b0fc08` | `YRII.PAL` | `[0x00844c08]` |

> Filename addresses verified via `read_memory(0x00844bf0, 40)`, then `read_memory(0x008453c4, 128)` decoded the strings:
> `0x00845404 = "RADARYURI.PAL"`, `0x00845414 = "UIBKGDY.PAL"`, `0x00845420 = "UIBKGD.PAL"`, `0x0084542c = "SIDEBAR.PAL"`, `0x008453e8 = "YRII.PAL"`, `0x008453f4 = "OBSERVER.PAL"`.

**The Rust TODO's "OBSERVER.PAL" guess is incorrect.** OBSERVER.PAL exists, but it lives in `DAT_00b0fc00`, not in the slot read by `FUN_0072f4a0`. The sidebar gadget palette is unambiguously **SIDEBAR.PAL**.

---

## 4. Draw flag and surface — answers Investigation Point #2

`SBGadgetClass::Draw` issues exactly one `CC_Draw_Shape` call per draw. From `disassemble_function 0x0069DEB0`:

```
0069df64  MOV AL, byte ptr [ESI + 0x55]   ; highlight bit
0069df71  NEG AL
0069df75  SBB EAX, EAX                    ; EAX = -1 if set, 0 if clear
0069df78  AND EAX, 0x800                  ; EAX = 0x800 or 0
...
0069df85  PUSH EAX                        ; flag arg
0069df8f  MOV EDX, [ESI + 0x50]           ; palette ConvertClass
0069df94  MOV ECX, EDI                    ; surface (DAT_00887300 or DAT_00887314)
0069df96  CALL CC_Draw_Shape
```

The surface choice is in lines `0069df12-0069df1f`:

```
0069df12  MOV AL, byte ptr [ESI + 0x4c]
0069df15  MOV EDI, [0x00887300]           ; g_SidebarSurface
0069df1b  TEST AL, AL
0069df1d  JNZ 0x0069df25
0069df1f  MOV EDI, [0x00887314]           ; g_PrimarySurface
```

> Translation: `+0x4C == 1` ⇒ draw to `g_SidebarSurface` (`[0x00887300]`); `+0x4C == 0` ⇒ draw to `g_PrimarySurface` (`[0x00887314]`). Both Repair and Sell init `+0x4C = 1`, so **both draw to `g_SidebarSurface`**.

**Flag values seen at the call site:**

| Source of flag bit | Value | When |
|---|---|---|
| Gadget `+0x55 == 0` | `0` | Normal (the steady-state for Repair/Sell) |
| Gadget `+0x55 != 0` | `0x800` | "Predator"/alpha / highlight blit (rarely toggled for these buttons) |

The sidebar chrome calls (radar frame, tab background, etc.) use flag `0x400` — see `disassemble_function 0x006A6C30` lines `006a6d16 PUSH 0x400`, `006a6e03 PUSH 0x400`. **`SBGadgetClass::Draw` never passes `0x400`.** That bit is specific to the chrome blits, not the gadget blits.

---

## 5. Frame selection — answers Investigation Point #3

From `decompile_function 0x0069DEB0` (the conditional in `SBGadgetClass::Draw`):

```c
if (gadget->disabled /* +0x1E */) {
    frame = 2;
} else if (gadget->pressable /* +0x40 */ == 0) {
    frame = (gadget->hover_static /* +0x2C */ != 0) ? 1 : 0;
} else /* pressable */ {
    if (gadget->mouse_down /* +0x34 */) {
        frame = (gadget->mode_active /* +0x2D */ != 0) ? 4 : 3;
    } else {
        frame = (gadget->mode_active /* +0x2D */ != 0) ? 1 : 0;
    }
}
```

For Repair and Sell `+0x40 == 1` always (set by `SidebarClass::Init` at `006a5337` and `006a53b8`), so the frame table is:

| State | `+0x1E` | `+0x34` | `+0x2D` | Frame |
|---|---|---|---|---|
| Disabled (no power / no money / no eligible target) | 1 | x | x | **2** |
| Idle | 0 | 0 | 0 | **0** |
| Mode active (Repair-mode or Sell-mode cursor is on) | 0 | 0 | 1 | **1** |
| Mouse-down while idle | 0 | 1 | 0 | **3** |
| Mouse-down while already-active | 0 | 1 | 1 | **4** |

`+0x2D` is *not* a transient hover bit for these gadgets — it is the toggled-mode bit. `SidebarClass::AI` at `0x006A7780` confirms:
- Event `0x8065` (Repair click) calls `FUN_004ac8c0(-1)`, which writes the toggled state to `param_1[0x46c]` where `param_1` is `int*` — actual **byte offset `+0x11B0`** on SidebarClass (corrected 2026-05-29: doc previously quoted `param_1[0x46c]` without noting the `int*` multiplier; binary decompile of `FUN_004ac8c0` at `0x004AC8C0` shows `(char)param_1[0x46c]` with `int*` param_1, and the companion zero-write `*(undefined1*)((int)param_1 + 0x11b1)` = direct byte `+0x11B1` confirms the Repair bit is at `+0x11B0` — ROOT_CAUSE: PARAM1_TYPE_MISREAD). Separately, the gadget `+0x2D` mirror is at `DAT_00b0b3cd` (= Repair gadget `+0x2D`, `0xb0b3a0 + 0x2D = 0xb0b3cd`).
- Event `0x8066` (Sell click) calls `FUN_004ac660(-1)`, which writes to `*(char*)(param_1 + 0x11b1)` (SidebarClass) and `DAT_00b07e25` (= Sell gadget `+0x2D`, `0xb07df8 + 0x2D = 0xb07e25`).

> Frame-selection logic verified via both `decompile_function 0x0069DEB0` and `disassemble_function 0x0069DEB0` lines `0069ded3..0069df12` (full conditional chain).
> Command dispatch verified via `decompile_function 0x006A7780` (the `if (uVar3 == 0x8066)` / `if (uVar3 == 0x8065)` arms call `FUN_004ac660` / `FUN_004ac8c0` respectively, after `VocClass__PlayAtPos(0x3f800000, 0)` — i.e., they play the same click voc).
> The tail of `SidebarClass::AI` reads `DAT_00b07e25` (Sell `+0x2D`) and `DAT_00b0b3cd` (Repair `+0x2D`) directly, confirming those addresses *are* the gadget mirror of mode-active state.

### 5.1 SHP frame-count expectation

The frame table requires SHP indices **0..4 inclusive** — five frames. Whether the retail `REPAIR.SHP` / `SELL.SHP` actually contain five usable frames is a file-format check, not a binary check — out of scope for this Ghidra report, but flagged here so the brainstorm/patch session does it explicitly (a missing frame 2 will manifest as a black/garbage image when the gadget goes disabled).

---

## 6. Position constants — answers Investigation Point #4

### 6.1 Init writes (`SidebarClass::Init` at `0x006A5310`)

From `decompile_function 0x006A5310`, gadget-relative offsets resolved against bases `0xb0b3a0` (Repair) and `0xb07df8` (Sell):

```c
// Repair gadget at DAT_00b0b3a0
gadget.X  /* +0x0C */ = DAT_00b0b4dc;
gadget.Y  /* +0x10 */ = DAT_00b0b4e0;
gadget.ID /* +0x24 */ = 0x65;

// Sell gadget at DAT_00b07df8
gadget.X  /* +0x0C */ = DAT_00b0b4e4 + DAT_00b0b3ac;   // = Repair X + Sell-delta
gadget.Y  /* +0x10 */ = DAT_00b0b3b0;                   // = Repair Y
gadget.ID /* +0x24 */ = 0x66;
```

So Repair drives the layout; Sell is `(DAT_00b0b4e4, 0)` to the right of Repair.

### 6.2 Where each layout constant comes from

**`DAT_00b0b4dc` = Repair X** — written by `SidebarClass::InitSidebarRect` (`0x006A5130`). From `decompile_function 0x006A5130`:

```c
int side = *(int*)(g_ScenarioClass_Instance + 0x34b8);
if (side == 0) { DAT_00b0b4dc = g_SidebarX + 0x14; }   // SidebarX + 20
else           { DAT_00b0b4dc = g_SidebarX + 0x21; }   // SidebarX + 33
```

The decomp has separate arms for `side == 1` and `side >= 2`, but both write
`g_SidebarX + 0x21`. **The split is Allied side `0` versus Soviet/Yuri side
nonzero, not RA2-vs-YR or theater.** Fresh writer evidence establishes
`Scenario+0x34B8 = HouseTypeClass+0xBC`: `Read_Scenario`,
`0x0068479D..0x006847C9`; `Full_Init`, `0x00687794..0x00687833`.

**`DAT_00b0b4e0` = Repair Y** — written by `SidebarClass::InitLayoutConstants` (`0x006A5090`):

```c
if (side == 0) { DAT_00b0b4e0 = g_SidebarWidth + 8; }   // = 158 + 8 = 166
else           { DAT_00b0b4e0 = g_SidebarWidth + 7; }   // = 158 + 7 = 165
```

`g_SidebarWidth = 0x9E = 158` is a constant set by `SidebarClass::InitSidebarRect` at `006a51ae / 006a51d4` (`MOV g_SidebarWidth, 0x9e`). It is the height of the sidebar's top section (radar + reserved padding) in *sidebar-surface-local* pixels — not a screen X, despite the "Width" name (legacy from TS where the sidebar was drawn rotated).

**`DAT_00b0b4e4` = Sell X delta** — written by `SidebarClass::InitLayoutConstants`:

```c
if (side == 0) { DAT_00b0b4e4 = 0x40; }   // 64
else           { DAT_00b0b4e4 = 0x34; }   // 52
```

### 6.3 Final pixel coordinates

| Local side | Repair X (screen) | Sell X (screen) | Y (sidebar-surface-local) |
|---|---|---|---|
| `0` (Allied) | `g_SidebarX + 20` | `g_SidebarX + 20 + 64` = `g_SidebarX + 84` | `166` |
| `1` (Soviet), `2` (Yuri) | `g_SidebarX + 33` | `g_SidebarX + 33 + 52` = `g_SidebarX + 85` | `165` |

Note that Sell **ends up at almost the same X** in both branches (`84` vs `85`) — the side difference compensates Repair-X and Sell-delta together.

> Position math verified via `decompile_function 0x006A5130` (InitSidebarRect) and `decompile_function 0x006A5090` (InitLayoutConstants).
> The `SidebarX` value itself: `g_SidebarX = g_RadarViewportWidth + g_RadarViewportOffsetX` — i.e., the right edge of the radar viewport (the sidebar's left edge in screen space).
> Y is in *surface-local* coordinates: `SBGadgetClass::Draw` selects `g_SidebarSurface` (because `+0x4C == 1`), and the engine treats that surface's origin as the top of the radar/sidebar area — not the screen top.

### 6.4 Why the existing Rust `repair_y` / `sell_y` in `layout_spec.rs` may be drifting

Rust-side note (for the patch session, not this report's main scope): if `layout_spec.rs` derives `repair_y` from "CameoY + 8", it is computing the wrong base. `CameoY` is the *first cameo's* Y, not Repair's Y. gamemd does **not** position Repair relative to the cameo strip — it positions it at a fixed `g_SidebarWidth + 8` / `+ 7`. These coincide only if `CameoY == g_SidebarWidth`, which is true in some layouts but not by construction.

---

## 7. Side-specific behaviour — answers Investigation Point #5

### 7.1 What gamemd does (or doesn't do)

`SidebarClass::LoadSHPs` (`0x006A5840`) loads the two SHPs unconditionally:

```
006a58cb  LEA EDX, [ESP + 0x14]
006a58cf  MOV ECX, 0x83fa4c           ; "SELL.SHP"
006a58dd  CALL CDFileClass::Ctor      ; reads file from MIX
006a58e3  MOV ECX, 0xb07df8           ; Sell gadget base
006a58e8  CALL SBGadgetClass::SetShape

006a5903  LEA EDX, [ESP + 0x14]
006a5907  MOV ECX, 0x83fa40           ; "REPAIR.SHP"
006a5911  CALL CDFileClass::Ctor
006a5917  MOV ECX, 0xb0b3a0           ; Repair gadget base
006a591c  CALL SBGadgetClass::SetShape
```

No side / country / house / scenario branch — both names are fixed string constants.

> Filenames verified via `search_strings("repair.shp") → 0x0083fa40` and `search_strings("sell.shp") → 0x0083fa4c`.
> Xrefs verified via `get_xrefs_to 0x0083fa40` and `get_xrefs_to 0x0083fa4c` — each has exactly one reference, both inside `SidebarClass::LoadSHPs`.

### 7.2 What actually varies, then

The art varies only via **MIX-archive search order**. RA2/YR ship the following:

- `sidebar.mix` / `sidebarmd.mix` — shared SHP archives.
- `sidec01.mix`, `sidec02.mix`, `sidec03.mix` — Allied / Soviet / Yuri side-specific.
- `sidec0Xmd.mix` — YR overrides.

If `REPAIR.SHP` exists in any of those, the search order determines which copy wins. The gamemd code **does not** choose; it just calls `LoadFileFromMIX("REPAIR.SHP")`. Inspecting the retail MIX contents (a file-system question, outside this binary's scope) is required to know whether the side actually changes the art.

> `CDFileClass::Ctor` calls `LoadFileFromMIX()` first (cross-MIX search), then falls back to CCFileClass for CD/local — verified via `decompile_function 0x004A38D0`.

### 7.3 The current Rust upload path (context only)

`src/render/sidebar_chrome.rs:282-294` re-loads `repair.shp` / `sell.shp` with the side's theme palette (sidebar.pal for Allied/Soviet, radaryuri.pal for Yuri) *on chrome rebuild* — which means the Rust port is *already* attempting per-side art. That is **more** side-aware than gamemd itself; gamemd loads the SHP once and never reloads it per side. Decide during the brainstorm session whether to match gamemd (load once) or keep the chrome-rebuild reload.

---

## 8. The render call in context (full argument layout)

`CC_Draw_Shape` at `0x004AED70` is `__fastcall`:

```
ECX = surface drawing context       (g_SidebarSurface or g_PrimarySurface)
EDX = palette ConvertClass*         (DAT_0087f6cc for repair/sell)
[ESP+ 0] = shape*                   (gadget.SHP = +0x58)
[ESP+ 4] = frame index              (0..4 per §5)
[ESP+ 8] = point*                   (struct { i32 x; i32 y; ... } at &local_20[…])
[ESP+12] = rect*                    (window rect from surface->GetWindowRect, vtable +0x78)
[ESP+16] = flags                    (0 or 0x800 per §4)
[ESP+20] = 0
[ESP+24] = 0
[ESP+28] = 0
[ESP+32] = 0x3E8                    (=1000, scaling/zoom; matches chrome calls)
[ESP+36] = 0
[ESP+40] = 0
[ESP+44] = 0
[ESP+48] = 0
[ESP+52] = 0
```

The point structure in stack is wide (the decomp records `local_20[0] = Y` separately from the X write 0x28 bytes earlier); for porting purposes, what matters is that the call delivers `(X, Y) = (gadget.X + gadget.+0x44 /* = -480 */, gadget.Y + gadget.+0x48 /* = 0 */)` to the renderer's point arg.

The `+0x44 = -480` X offset is the gadget's "off-screen sentinel" — it is *added* to gadget.X before drawing. In stock RA2 the sidebar surface is at least 480 px wide and the `-480` cancels against an internal `+480` shift somewhere in `CC_Draw_Shape` / surface clipping (likely the surface's blit-origin). In the Rust port, this `-480` is most likely safe to ignore (treat the gadget's drawn X as `gadget.X` directly) — but verify by tracing how `g_SidebarSurface` is constructed before relying on that.

> Point/rect/flag stack offsets verified via `disassemble_function 0x0069DEB0` lines `0069df53..0069df96`.

---

## 9. Helper functions (for cross-referencing)

| Symbol (proposed) | Address | What it does |
|---|---|---|
| `SBGadgetClass::SetShape` | `0x0069DE00` | Stores SHP pointer at `gadget+0x58`, reads SHP header to set `gadget+0x14` (width) and `gadget+0x18` (height). Frees previous if owned. Verified via `decompile_function 0x0069DE00`. |
| `SBGadgetClass::SetLoadedFlag` | `0x0069DEA0` | One-line: `gadget+0x5C = byte`. Verified via `decompile_function 0x0069DEA0`. |
| `FUN_0069DFF0` | `0x0069DFF0` | "Register/link gadget into draw chain" — called after each Init block. Not yet labelled; out of scope here. |
| `FUN_0072F4A0` | `0x0072F4A0` | `return DAT_00b0fbe4;` — pointer to the SIDEBAR.PAL buffer. Verified via `decompile_function 0x0072F4A0`. |
| `CDFileClass::Ctor` *(misnamed; really "Load file from MIX, malloc'd buffer")* | `0x004A38D0` | First tries `LoadFileFromMIX()`; falls back to CCFileClass. Returns malloc'd buffer + a success-byte at `*EDX`. Verified via `decompile_function 0x004A38D0`. |

These are **not** renamed in this session — they remain as-is in Ghidra. If a follow-up session wants to rename them, do it after manually verifying the bodies once more.

---

## 10. Unverified / out-of-scope

- **`+0x44 = -480` sentinel:** the exact reason for the negative-480 X shift is plausibly a clip/blit-origin compensation, but the surface-construction code (`SidebarClass::InitSurface` at `0x006ABD30`) was not deeply traced for this report. Mark **YELLOW** and trace before assuming you can drop it on the Rust side.
- **Actual frame-count of retail `REPAIR.SHP` / `SELL.SHP`:** the binary expects frames `0..4`. A `.shp`-format check (header `frame_count` field at offset `+0x06`) of the live retail asset is needed; out of scope for this Ghidra-only doc. Mark **YELLOW**.
- **Whether `+0x55` (highlight, 0x800 flag) is ever toggled for Repair/Sell** in normal play: no writer to `0xb0b3f5` or `0xb07e4d` was found via direct `get_xrefs_to` (since writes use field-relative addressing, not address-literal), and tracing all writers would require a deeper field-access scan. Mark **YELLOW**: assume 0 in steady state, which matches the visual reality (no "pulsing" glow on these buttons in retail).
- **Which MIX physically supplies `REPAIR.SHP` / `SELL.SHP`** in retail: requires opening the actual `.mix` archives, which is outside Ghidra. Mark **YELLOW** for the follow-up patch session.

---

## 11. Recommended next step (not part of this doc)

Brainstorm session should treat the current Rust state as:
- Palette: already correct in intent (`sidebar.pal`), but the disabled draw block needs to be re-enabled — the TODO's "OBSERVER.PAL" speculation should be deleted.
- Flag: pass `0` (or `0x800` if a "highlighted" mode is introduced); do **not** copy the chrome's `0x400`.
- Frames: ensure `repair.shp` / `sell.shp` are uploaded with all available frames into the atlas (current code may only be uploading frame 0). Then index 0..4 at draw time based on `disabled / mode_active / mouse_down`.
- Position: derive from `(sidebar_left + 20-or-33, sidebar_top + 166-or-165)` based on the active local side. Don't anchor to `CameoY`.
- Side art: the hard-coded filename load is single-path, while the physical asset
  is selected by the already-established side MIX search order.
