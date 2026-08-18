# SidebarClass::Init — Gadget Positioning Report

Source: live decompilation of `gamemd.exe` via Ghidra MCP.  
Function: `SidebarClass__Init` at `0x006a5310`.  
Focus: gadget X/Y/W/H initial writes ONLY.  
Verified via: `decompile_function 0x006a5310`, `decompile_function 0x006a5090`,
`decompile_function 0x006a5130`, `decompile_function 0x006a8220`,
`disassemble_function 0x006a5310`.

Active in YR: **Yes** (all positioning in this function is unconditional within the `g_IsMapEditor == 0` gate).

---

## Call Sequence

```
SidebarClass__Init (0x006a5310)
  ├─ FUN_00653010()                  -- inherited Init (skipped: out-of-scope)
  ├─ SidebarClass__InitLayoutConstants (0x006a5090)  -- sets DAT_00b0b4dc..b0b4fc
  └─ SidebarClass__InitSidebarRect (0x006a5130, param=0) -- sets DAT_00b0b4f4..b0b514,
                                                           sets g_SidebarX, g_SidebarWidth
```

---

## Layout Constant Globals (set by InitLayoutConstants + InitSidebarRect)

Verified via `decompile_function 0x006a5090` and `decompile_function 0x006a5130`.

**Branch field is the selected local side index, not theater or an RA2-vs-YR
flag.** CORRECTED 2026-07-25 after tracing the writer:
`Read_Scenario`, `0x0068479D..0x006847C9`, and `Full_Init`,
`0x00687794..0x00687833`, copy `HouseTypeClass+0xBC` to
`Scenario+0x34B8`. Stock values are Allied `0`, Soviet `1`, Yuri `2`.
`InitSidebarRect` has three arms (`iVar1==0`, `iVar1==1`, `iVar1>=2`); the
latter two produce the same values.

| Address | Allied (side 0) | Soviet/Yuri (side ≥ 1) | Meaning |
|---------|-----------------------|-------------|---------|
| `DAT_00b0b4dc` | `g_SidebarX + 0x14` | `g_SidebarX + 0x21` | Repair/Sell X |
| `DAT_00b0b4e0` | `g_SidebarWidth + 8` | `g_SidebarWidth + 7` | Repair button Y |
| `DAT_00b0b4e4` | `0x40` (64) | `0x34` (52) | Repair-to-Sell X offset (= Sell.X − Repair.X) |
| `DAT_00b0b4e8` | `g_SidebarX + 0x1a` | `g_SidebarX + 0x14` | Tab 0 X base |
| `DAT_00b0b4ec` | `g_SidebarWidth + 0x27` | `g_SidebarWidth + 0x27` | Tab Y (same both modes) |
| `DAT_00b0b4f0` | `0x1d` (29) | `0x20` (32) | Tab X spacing per index |
| `DAT_00b0b4f4` | `g_SidebarX + 0x16` | `g_SidebarX + 0x16` | Cameo strip XPos |
| `DAT_00b0b4f8` | `g_SidebarWidth + 0x45` (≈227) | `g_SidebarWidth + 0x45` (≈227) | Cameo strip YPos |
| `DAT_00b0b4fc` | `0x3f` (63) | `0x40` (64) | Cameo column width |
| `DAT_00b0b500` | `0x32` (50) | `0x32` (50) | Cameo row height |
| `DAT_00b0b504` | `rows×50` (computed) | `rows×50` (computed) | Total visible cameo height |
| `DAT_00b0b50c` | `CameoY + 7 + DAT_00b0b504` | same | Scroll button Y |
| `DAT_00b0b510` | `0x2e` (46) | `0x2d` (45) | Scroll button width |
| `DAT_00b0b514` | `0x32` (50) | `0x32` (50) | Scroll speed px/tick |

Side gate: `*(int*)(g_ScenarioClass_Instance + 0x34b8)` — `0` = Allied,
`1` = Soviet, `2` = Yuri in stock data. The conditional in `InitSidebarRect`
has three arms but side indices ≥ 1 collapse to the same constants.
Active in YR: **Yes**; the branch is independent of map theater.
PATCHED 2026-05-20: row `DAT_00b0b4f8` corrected from `g_SidebarX + 0x45` to `g_SidebarWidth + 0x45` (numerical result ≈227 unchanged, but the variable binding for any Rust port is different).

---

## 1. Repair Button (ID 0x65, object at `0xb0b3a0`)

Verified via `disassemble_function 0x006a5310` (instructions at 0x006a5338–006a53a2).

```asm
006a5338: MOV ECX, [0x00b0b4e0]          ; ECX = DAT_00b0b4e0 = Repair Y
006a533e: MOV EAX, [0x00b0b4dc]          ; EAX = DAT_00b0b4dc = Repair X
006a534b: MOV dword ptr [0x00b0b3b0],ECX ; [base+0x10] = Y
006a5371: MOV [0x00b0b3ac],EAX           ; [base+0x0C] = X
006a5367: MOV dword ptr [0x00b0b3c4],0x65 ; [base+0x24] = ID
```

Struct offsets (from base `0xb0b3a0`):
- `+0x0C` = **X** = `DAT_00b0b4dc` (= SidebarX + 0x14 Allied, SidebarX + 0x21 Soviet/Yuri)
- `+0x10` = **Y** = `DAT_00b0b4e0` (= SidebarWidth + 8 Allied, SidebarWidth + 7 Soviet/Yuri)
- No Width/Height writes for Repair in Init.

⚠️ **Disparity vs SIDEBAR_SYSTEM doc §9**: doc claims X at +0x10, Y at +0x14. Assembly shows X at +0x0C, Y at +0x10 — one DWORD shift. Active in YR: Yes.

---

## 2. Sell Button (ID 0x66, object at `0xb07df8`)

Verified via `disassemble_function 0x006a5310` (instructions at 0x006a53a7–006a540c).

```asm
006a53a7: MOV EAX, [0x00b0b3ac]         ; EAX = Repair.X
006a53ac: MOV ECX, [0x00b0b4e4]         ; ECX = DAT_00b0b4e4 (64 Allied / 52 Soviet/Yuri)
006a53b8: ADD ECX,EAX                   ; ECX = Repair.X + offset
006a53b2: MOV EDX, [0x00b0b3b0]         ; EDX = Repair.Y
006a53bf: MOV dword ptr [0x00b07e04],ECX ; [base+0x0C] = X
006a53db: MOV dword ptr [0x00b07e08],EDX ; [base+0x10] = Y
006a53d1: MOV dword ptr [0x00b07e1c],0x66 ; [base+0x24] = ID
```

Struct offsets (from base `0xb07df8`):
- `+0x0C` = **X** = `Repair.X + DAT_00b0b4e4`  (= Repair.X + 64 Allied, + 52 Soviet/Yuri)
- `+0x10` = **Y** = `Repair.Y` (identical to Repair button Y)
- No Width/Height writes for Sell in Init.

Active in YR: Yes.

---

## 3. Tab Buttons (4 × SBGadgetClass at `0xb07c48`, stride `0x60`)

Verified via `disassemble_function 0x006a5310` (instructions at 0x006a5413–006a5484).

```asm
; Loop: ESI starts at 0xb07c6c (= array_base + 0x24), stride +0x60, 4 iterations
; tab_index (EBP) = 0..3
006a541c: MOV EDX, [0x00b0b4f0]          ; EDX = spacing (29 Allied / 32 Soviet/Yuri)
006a5422: MOV EAX, [0x00b0b4e8]          ; EAX = Tab X base
006a5427: IMUL EDX,EBP                   ; EDX = spacing * index
006a542a: LEA ECX,[EBP + 0xcb]           ; ECX = ID = 0xcb + index (0xcb..0xce)
006a5434: ADD EDX,EAX                    ; EDX = Tab X base + spacing*index (final X)
006a5436: MOV EAX, [0x00b0b4ec]          ; EAX = DAT_00b0b4ec = Tab Y  (corrected 2026-05-28: prior listing omitted this instruction, making it appear EAX still held Tab X base when writing [ESI-0x14]; binary confirms EAX is reloaded with DAT_00b0b4ec before the Y write — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT / omitted instruction)
006a543b: MOV dword ptr [ESI],ECX        ; [elem+0x24] = ID
006a5443: MOV dword ptr [ESI + -0x18],EDX ; [elem+0x0C] = X
006a5446: MOV dword ptr [ESI + -0x14],EAX ; [elem+0x10] = Y = DAT_00b0b4ec
```

Where:
- `elem = ESI - 0x24` = array base + `tab_index * 0x60`
- `[elem+0x0C]` = **X** = `DAT_00b0b4e8 + DAT_00b0b4f0 * tab_index`
- `[elem+0x10]` = **Y** = `DAT_00b0b4ec`

Tab X values (Soviet/Yuri): `SidebarX+0x14 + 32*i` for i=0..3.  
Tab Y (both modes): `g_SidebarWidth + 0x27`.  
No Width/Height writes for Tab buttons in Init.

⚠️ **Disparity vs SIDEBAR_SYSTEM doc §9**: doc says X at +0x10, Y at +0x14. Assembly confirms X at +0x0C, Y at +0x10. Active in YR: Yes.

---

## 4. Scroll Down Button (ID 0xC9, object at `0xb0b328`)

Verified via `disassemble_function 0x006a5310` (instructions at 0x006a5486–006a54ba).

```asm
006a5496: MOV byte ptr [0x00b0b345],0x1   ; [base+0x1D] = IsActive = 1
006a549d: MOV dword ptr [0x00b0b34c],0xc9 ; [base+0x24] = ID = 0xC9
006a54b4: MOV dword ptr [0x00b0b348],0x55 ; [base+0x20] = 0x55 (unknown field)
006a54ae: MOV dword ptr [0x00b0b378],EDX  ; [base+0x50] = SHP pointer
```

**No X or Y writes** for Scroll Down in Init. Position is set elsewhere (not in this function).  
Width `0x55 = 85` is written to `+0x20` — this offset is not X, Y, W, or H as scoped.  
Active in YR: Yes (unconditional).

---

## 5. Scroll Up Button (ID 0xC8, object at `0xb0b408`)

Verified via `disassemble_function 0x006a5310` (instructions at 0x006a54bf–006a54ec).

```asm
006a54c9: MOV byte ptr [0x00b0b425],0x1    ; [base+0x1D] = IsActive = 1
006a54d0: MOV dword ptr [0x00b0b42c],0xc8  ; [base+0x24] = ID = 0xC8
006a54e6: MOV dword ptr [0x00b0b428],0x55  ; [base+0x20] = 0x55 (unknown field)
006a54e1: MOV [0x00b0b458],EAX             ; [base+0x50] = SHP pointer
```

**No X or Y writes** for Scroll Up in Init.  
Active in YR: Yes.

---

## 6. SelectClass Cameos (240 entries at `0xb07e80`, stride `0x38`)

Positioning is delegated to `SidebarClass__InitSelectZones` at `0x006a8220`, called 4 times (once per strip).  
Strip init loop (instructions at 0x006a54f1–006a553b): writes StripClass.XPos and StripClass.YPos first, then calls InitSelectZones.

```asm
; Strip loop: ESI = this+0x1568 (=Strip[N].XPos+4), stride +0xF94, N=0..3
006a5502: MOV [ESI - 0x4], ECX   ; Strip.XPos = DAT_00b0b4f4
006a550e: MOV [ESI + 0x0], EDX   ; Strip.YPos = DAT_00b0b4f8
006a5523: MOV [EDX + 0x8], 0x3C  ; some strip field = 60 (cameo count per col × rows?)
006a5529: MOV [EDX + 0xC], EAX   ; some strip field = DAT_00b0b504 (total height)
```

Then `SidebarClass__InitSelectZones(strip_index, &strip_base)` at `0x006a8220` performs  
(verified via `decompile_function 0x006a8220`):

```c
// visibleRows = ((g_SidebarHeight - Strip.YPos - margin) / 0x32)
int visibleRows = ((DAT_00886f9c - DAT_00b0b4f8) - margin - 7 + g_SidebarWidth) / 50;
for (row = 0; row < visibleRows; row++) {
    for (col = 0; col < 2; col++) {
        int idx = strip.TabIndex * 0x3C + row * 2 + col;
        SelectGadgets[idx].X      /* +0x0C */ = Strip.XPos + DAT_00b0b4fc * col;
        SelectGadgets[idx].Y      /* +0x10 */ = Strip.YPos + 1 + DAT_00b0b500 * row;
        SelectGadgets[idx].Width  /* +0x14 */ = 0x3C;   // 60, literal
        SelectGadgets[idx].Height /* +0x18 */ = 0x30;   // 48, literal
        SelectGadgets[idx].ID     /* +0x24 */ = 0xCA;   // 202 (PATCHED 2026-05-20: was incorrectly +0x2C; +0x2C receives the strip parent pointer, not ID)
    }
}
```

Width and Height are **literals** (not from globals).  
`margin` = 0x1a (Allied) or 0x12 (Soviet/Yuri).  
Active in YR: Yes.

---

## 7. SBGadgetClass Struct Layout Correction

Direct assembly evidence (X at `[ESI-0x18]`, Y at `[ESI-0x14]`, ESI = base+0x24):

| Offset | Field | Evidence |
|--------|-------|----------|
| `+0x0C` | **X** | `[ESI-0x18]` where ESI=base+0x24 — Tab loop, Repair, Sell writes |
| `+0x10` | **Y** | `[ESI-0x14]` where ESI=base+0x24 — Tab loop, Repair writes |
| `+0x1D` | IsActive | byte write `0x1` in scroll button init |
| `+0x24` | ID | Tab ID (0xcb+i), Repair (0x65), Sell (0x66), Scroll (0xC8/C9) |
| `+0x50` | SHP pointer | scroll button SHP write |

Prior SIDEBAR_SYSTEM doc §9 listed X at +0x10, Y at +0x14 — **off by one DWORD**. Corrected here.

SelectClass layout (from InitSelectZones writes, array base `0xb07e80`):

| Offset | Field | Value | Evidence |
|--------|-------|-------|----------|
| `+0x0C` | X | `Strip.XPos + colWidth*col` | `DAT_00b07e8c` = base+0x0C |
| `+0x10` | Y | `Strip.YPos + 1 + rowH*row` | `DAT_00b07e90` = base+0x10 |
| `+0x14` | Width | `0x3C` (60) — literal | `DAT_00b07e94` = base+0x14 |
| `+0x18` | Height | `0x30` (48) — literal | `DAT_00b07e98` = base+0x18 |
| `+0x24` | ID | `0xCA` — literal | `DAT_00b07ea4` = base+0x24 |
| `+0x2C` | StripParent | strip pointer | passed in by `InitSelectZones` (not ID — see PATCHED note below) |

> **PATCHED 2026-05-20.** Prior version listed ID at `+0x2C`. Live disassembly of `InitSelectZones @ 0x006a8220` writes `0xCA` to `DAT_00b07ea4` = `base + 0x24`. The slot at `+0x2C` (`DAT_00b07eac`) receives the strip parent pointer (`param_1`) instead. A Rust port reading the gadget ID at `+0x2C` would read a pointer, not the literal `0xCA`.

---

## Open Questions (out of scope, noted for follow-up)

1. **Scroll button X/Y position** — not written in `SidebarClass__Init`. Must be set in a separate function (possibly the `(*vtable[0x22])()` call at 0x006a5541, or during `InitSidebarRect(1)`).
2. **SBGadgetClass Width/Height** — not written for Tab, Repair, or Sell buttons in Init; sizing may come from SHP frame dimensions at draw time.
3. **`+0x20` field on scroll buttons** — value `0x55 = 85` written; purpose unknown. Not a position field.
4. **FUN_004e1460** — called inside Tab loop, purpose unknown (SHP load?).
5. **FUN_0069dff0** — called after each gadget init group, purpose unknown (link into gadget chain?).

---

## Top 5 Verified Facts

1. **Tab X formula** (assembly 0x006a5427–006a5443): `tab.X = DAT_00b0b4e8 + DAT_00b0b4f0 * index`. Soviet/Yuri: `SidebarX+0x14 + 32*i`. Allied: `SidebarX+0x1a + 29*i`. Written to SBGadgetClass `+0x0C`.
2. **SBGadgetClass X at +0x0C, Y at +0x10** (not +0x10/+0x14 as prior doc claimed). Direct assembly evidence from Tab loop `[ESI-0x18]`/`[ESI-0x14]` with `ESI=base+0x24`.
3. **Sell.X = Repair.X + DAT_00b0b4e4** (assembly 0x006a53ac–006a53bf): 64 for Allied, 52 for Soviet/Yuri. Both Repair and Sell share the same Y.
4. **SelectClass Width=0x3C and Height=0x30 are hardcoded literals** (InitSelectZones 0x006a8220 decompile), not read from globals.
5. **Scroll Up/Down buttons have no X/Y writes in SidebarClass::Init** — their position is set outside this function. Only IsActive, ID, an unknown +0x20 field, and SHP pointer are initialized here.

---

Status: **COMPLETE**
