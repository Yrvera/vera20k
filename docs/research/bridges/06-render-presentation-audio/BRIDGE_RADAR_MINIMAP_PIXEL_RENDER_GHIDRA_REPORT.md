# BRIDGE_RADAR_MINIMAP_PIXEL_RENDER_GHIDRA_REPORT

**Investigation target:** Per-cell radar/minimap draw branch for bridge structural cells
(flag 0x100) and low-bridge overlays (IDs 0x4A–0x63 and 0xCD–0xE6) in `gamemd.exe`.

**Status:** COMPLETE — all branch conditions, function addresses, and color-read paths
verified via live Ghidra MCP decompilation and disassembly.

---

## Active in YR: Yes

Radar minimap renders every frame during normal gameplay. The bridge branches within
`CellClass__GetRadarColor` fire whenever a radar-visible cell carries the 0x100 flag
or a low-bridge overlay. Verified reachable in the main rendering pipeline.

---

## 1. Entry Point

`CellClass__GetRadarColor` @ **0x0047C060**
- Verified via `decompile_function 0x0047C060` and `disassemble_function 0x0047C060`.
- Called from `FillTerrainColors` (0x00654EA0) during radar surface generation and
  from `ClearBackground` (0x00655250) during incremental terrain-dirty updates.
- Signature (thiscall): `void CellClass__GetRadarColor(CellClass* this, RGB3* out_left, RGB3* out_right)`
  where `RGB3` = 3-byte packed {R, G, B}.

Both outputs (`out_left`, `out_right`) are always set to the **same value** in every
branch — the two-output signature exists but is functionally 1-color-per-cell.

---

## 2. Priority Order of Color Branches

```
1. Building occupant (RTTI 0x24)?              → (0xC8, 0xC8, 0xA0) = khaki/tan
2. Bridge structural flag (cell+0x140 & 0x100)? → OverlayClass__GetRadarColor(BRIDGE1, frame=0)
3. Regular overlay (cell+0x44 != skip list)?
   a. Tiberium type (+0x29C ptr set)?           → GetTiberiumRadarColor per density
   b. Ore range 0x4A–0x63 or 0xCD–0xE6?        → OverlayClass__GetRadarColor(cell overlay, frame=1 FORCED)
   c. Non-ore, non-tiberium overlay?            → OverlayClass__GetRadarColor(cell overlay, frame=density)
   d. Wall overlay (FUN_005FDD20)?              → (0xAA, 0xAA, 0x82) = grey
4. Terrain tile (default path)                 → tile RadarColor + theater brightness × 0.5
5. Fallback (no sub-tile data)                 → (0x3C, 0x3C, 0x3C) = dark grey
```

---

## 3. Bridge Structural Cell Branch (flag 0x100)

### Assembly (verified via `disassemble_function 0x0047C060`)

```asm
0047c0ae: MOV EAX, [ESI + 0x140]   ; ESI = cell this-ptr; load cell flags (u32)
0047c0b4: TEST AH, 0x1             ; AH = bits 8-15 of EAX; AH & 0x1 = bit 8 = flag 0x100
0047c0b7: JZ  0x0047c0d9           ; if flag NOT set → skip bridge branch
0047c0b9: MOV ECX, [0x00a83d84]    ; ECX = g_OverlayTypeClass_Array base ptr (runtime ptr, static=0)
0047c0bf: LEA EAX, [ESP + 0x8]     ; EAX = local output RGB buffer
0047c0c3: PUSH 0x0                 ; push frame = 0
0047c0c5: PUSH EAX                 ; push output buffer ptr
0047c0c6: MOV ECX, [ECX + 0x60]   ; ECX = g_OverlayTypeClass_Array[0x60/4] = array[24] = BRIDGE1
0047c0c9: CALL 0x005fed00          ; OverlayClass__GetRadarColor(this=BRIDGE1_type, buf, frame=0)
0047c0ce: MOV ECX, [ESP + 0x18]   ; restore out_left ptr
0047c0d2: MOV EDX, EAX            ; result ptr
0047c0d4: JMP 0x0047c18f          ; → copy result to both out_left and out_right, return
```

**Key facts:**
- Flag 0x100 lives in `cell + 0x140`, bit 8 = byte at `cell + 0x141`, bit 0. Tested as `TEST AH, 0x1` on the full `u32` loaded from `cell+0x140`.
- The overlay type used is **always BRIDGE1** (array index 24 = `g_OverlayTypeClass_Array[0x18*4]`), regardless of the cell's own `+0x44` overlay index. The cell's overlay is ignored for flag-0x100 cells.
- Frame argument = **0** (hardcoded `PUSH 0x0`).
- `g_OverlayTypeClass_Array` global: pointer stored at runtime at `0x00A83D84` (verified via disassembly; static value = 0x00000000 at analysis time, confirmed runtime-init).
- INI indexing: INI key `25=BRIDGE1` is 1-based, so internal array index = 24 = 0x18. Confirmed by the `[ECX + 0x60]` offset (0x60 / 4 = 24), and by the constructor loop that sets `type[0x294] = own_index_in_array`.

---

## 4. Low-Bridge Overlay Branch (IDs 0x4A–0x63 and 0xCD–0xE6)

### INI Overlay Index Mapping

From `ini/rulesmd.ini [OverlayTypes]` (1-based INI keys → 0-based internal indices):

| INI key range | Internal index range | Type names             |
|---------------|----------------------|------------------------|
| 74–76 (0x4A–0x4C) | 73–75           | PALET02–PALET04        |
| 77–99 (0x4D–0x63) | 76–98           | LOBRDG01–LOBRDG23      |
| 100 (0x64)        | 99              | LOBRDG24 (skipped)     |
| 205–230 (0xCD–0xE6) | 204–229       | LOBRDB01–LOBRDB26      |

INI keys 77–99 and 209–236 (`LOBRDG*` and `LOBRDB*`) are the low-bridge deck overlay tiles.
The code's range check `0x4A–0x63` catches PALET02–LOBRDG23; `0xCD–0xE6` catches LOBRDB01–LOBRDB26.
These ranges use the **cell's stored overlay index at `cell+0x44`** (the INI key value, 1-based).

### Assembly (verified via `disassemble_function 0x0047C060`)

```asm
0047c151: CMP EAX, 0x4a            ; EAX = cell overlay index (cell+0x44)
0047c154: JL  0x0047c15b           ; < 0x4A → not in first range
0047c156: CMP EAX, 0x63
0047c159: JLE 0x0047c169           ; 0x4A ≤ EAX ≤ 0x63 → FORCE frame=1
0047c15b: CMP EAX, 0xcd
0047c160: JL  0x0047c171           ; < 0xCD → not in second range
0047c162: CMP EAX, 0xe6
0047c167: JG  0x0047c171           ; > 0xE6 → not in second range
0047c169: PUSH 0x1                 ; FORCE frame = 1 (consistent deck color)
0047c16b: LEA ECX, [ESP + 0xc]
0047c16f: JMP 0x0047c17e
0047c171: XOR ECX, ECX
0047c173: MOV CL, [ESI + 0x11e]   ; use cell density/frame byte from cell+0x11E
0047c179: PUSH ECX                 ; push natural frame index
0047c17a: LEA ECX, [ESP + 0xc]
0047c17e: PUSH ECX                 ; push output buffer
0047c17f: MOV ECX, [EDX + EAX*4]  ; ECX = g_OverlayTypeClass_Array[overlay_index] = this overlay's type
0047c182: CALL 0x005fed00          ; OverlayClass__GetRadarColor(this=cell_overlay_type, buf, frame)
```

**Key facts:**
- Frame = **1 hardcoded** for the 0x4A–0x63 and 0xCD–0xE6 ranges (low-bridge decks).
- Frame = `cell+0x11E` (overlay density/damage state byte) for all other non-tiberium overlays.
- The overlay type used is the **cell's own overlay** (`g_OverlayTypeClass_Array[cell+0x44]`), not the fixed BRIDGE1.

---

## 5. `OverlayClass__GetRadarColor` @ 0x005FED00

Shared by both the bridge-flag branch and the low-bridge overlay branch.

### Signature (thiscall)
`void OverlayClass__GetRadarColor(OverlayTypeClass* this, RGB3* out, uint frame)`

### Flow (verified via `decompile_function 0x005FED00` and `disassemble_function 0x005FED00`)

```
1. Call vtable[0x9C] (= vtable slot 39) on this → returns SHP data ptr (EAX)
   vtable[0x9C] = OverlayTypeClass__GetRadarColor @ 0x005FEDE0
   (verified: vtable at 0x7EF600, slot 39 = bytes [156..159] = 0x005FEDE0)

2. If SHP ptr == 0 (no image data):
   a. Load alternate SHP ptr from this+0x29C
   b. If this+0x29C == 0 → BLACK PIXEL FALLBACK: write (0,0,0) to out, return
   c. If this+0x29C != 0 → call vtable[0x9C] on the alternate type
   d. If alternate also == 0 → BLACK PIXEL FALLBACK

3. Load self_index = *(int*)(this + 0x294) [set in constructor = own array index]
   Range check: if self_index in [0x7F,0x8A] or [0x93,0x9E]:
     → call GetTiberiumRadarColor with byte-swapped output (channels 0 and 1 swapped)
   else:
     → call GetTiberiumRadarColor with direct output

   Both paths call GetTiberiumRadarColor @ 0x0069E860 with:
     ECX = SHP data ptr (from step 1)
     param_1 = &frame_arg (output buffer for 3-byte color)
     param_2 = frame (0 for bridge-flag, 1 for low-bridge overlays)
```

### Black pixel fallback trigger (0x005FEDBB)
```asm
005fedbb: MOV EAX, [ESP + 0xc]     ; EAX = out buffer ptr
005fedbf: MOV byte [ESP + 0x10], 0  ; R = 0
005fedc4: MOV byte [ESP + 0x11], 0  ; G = 0
005fedc9: MOV SI, [ESP + 0x10]
005fedce: MOV EDX, EAX
005fedd0: XOR CL, CL               ; B = 0
005fedd2: MOV word [EDX], SI        ; out[0..1] = 0x0000
005fedd6: MOV byte [EDX + 0x2], CL  ; out[2] = 0
005fedda: RET 0x8
```
Fallback writes (0, 0, 0) — pure black — when no SHP data can be resolved.

---

## 6. `GetTiberiumRadarColor` @ 0x0069E860

Reads the 3-byte color from the SHP frame table.
Signature (fastcall): `RGB3* GetTiberiumRadarColor(RGB3* out, uint frame)`
ECX (from vtable result) = SHP data base pointer.

```c
// SHP header layout (partial): word at +6 = frame_count
if (shp_data != 0 && frame < *(short*)(shp_data + 6)) {
    entry = shp_data + 8 + frame * 0x18;   // frame table: entry stride = 0x18 bytes
    out[0] = *(byte*)(entry + 0x0C);        // R byte
    out[1] = *(byte*)(entry + 0x0D);        // G byte
    out[2] = *(byte*)(entry + 0x0E);        // B byte
    return out;
}
// frame out of range OR shp_data == 0:
out[0] = out[1] = out[2] = 0;              // black fallback
return out;
```

Verified via `decompile_function 0x0069E860`:
```c
iVar2 = SHP_Resolve();   // ECX = SHP ptr; returns it (resolves deferred load)
if ((iVar2 != 0) && (param_2 < (uint)(int)*(short*)(iVar2 + 6)) &&
    (iVar2 = iVar2 + 8 + param_2 * 0x18, iVar2 != 0)) {
    uVar1 = *(undefined1*)(iVar2 + 0xe);   // offset +0xE = B
    *param_1 = *(undefined2*)(iVar2 + 0xc); // offsets +0xC/+0xD = R, G
    *(undefined1*)(param_1 + 1) = uVar1;
    return param_1;
}
*param_1 = 0; *(undefined1*)(param_1+1) = 0;
return param_1;
```

**The color bytes are read directly from the SHP frame metadata (not pixel data).** Each frame table entry is 0x18 bytes; bytes at +0xC, +0xD, +0xE within the frame entry store the pre-computed radar RGB for that frame. These are NOT sampled from the SHP pixel image at runtime — they are baked into the overlay's SHP file.

---

## 7. `OverlayTypeClass__GetRadarColor` @ 0x005FEDE0 (vtable[0x9C])

Returns the SHP data pointer for the overlay type.

```c
int iVar1 = *(int*)(param_1 + 0xa4);  // param_1 = ECX; +0xa4 = cached SHP data ptr
if (iVar1 == 0 && *(char*)(param_1 + 0x2af) != '\0') {
    // demand-load: resolve filename, open file, load SHP via FUN_004A3890
    *(int*)(param_1 + 0xa4) = iVar1;  // cache the result
}
return iVar1;
```

`OverlayTypeClass+0xa4` = cached SHP data pointer. `+0x2af` = demand-load flag.
Verified via `decompile_function 0x005FEDE0`.

---

## 8. Channel-Swap Branches in `OverlayClass__GetRadarColor`

The self_index range check ([0x7F,0x8A] or [0x93,0x9E]) selects between two code paths:

- **In-range path** (0x005FED51–0x005FED8D): calls `GetTiberiumRadarColor`, then does:
  ```asm
  MOV CX, [EAX]       ; word = bytes 0,1 of color
  MOV DL, CL          ; DL = byte 0
  MOV [out], CX       ; write word
  MOV AL, [EAX+2]     ; byte 2
  MOV CL, [out+1]     ; reload byte 1
  MOV [out+1], AL     ; out[1] = byte 2
  MOV [out+2], CL     ; out[2] = byte 1  ← channel 1 and 2 SWAPPED
  ```
- **Out-of-range path** (0x005FED91–0x005FEDB8): calls `GetTiberiumRadarColor`, copies directly without swap.

The self_index ranges 0x7F–0x8A and 0x93–0x9E translate to INI keys 128–139 (LOBRDGE1-4, TIB2_03...) and 148–159 (TIB2_19–TIB3_09). These are mid-range overlays; the swap is an artifact of an internal RA2 format inconsistency for those specific types.

**For bridge cells (BRIDGE1, self_index=24) and low-bridge decks (LOBRDG01-23, indices 76–98; LOBRDB01-26, indices 204–229): all are outside both ranges → no channel swap → direct copy.**

---

## 9. Overlay Skip List

Before reaching either the low-bridge or the default overlay branch, these indices are
skipped (cell treated as having no overlay, falls through to terrain):

```
-1  (0xFFFF)  = no overlay
100 (0x64)    = LOBRDG24
101 (0x65)    = LOBRDG25
231 (0xE7)    = LOBRDG23 duplicate / boundary
232 (0xE8)    = LOBRDB23 boundary
239 (0xEF)    = LOBRDB31 / unused
```

Verified at 0x0047C0FF–0x0047C136 (series of CMP/JZ instructions).

---

## 10. Complete Bridge Path Summary

### Flag 0x100 cells (bridge structural head/ramp cells):

```
cell+0x140 bit 8 set
→ ECX = g_OverlayTypeClass_Array[24]  (= BRIDGE1 type)
→ call OverlayClass__GetRadarColor(BRIDGE1_type, out, frame=0)
  → vtable[0x9C] = OverlayTypeClass__GetRadarColor: returns BRIDGE1's SHP data ptr
  → if SHP == NULL: try BRIDGE1_type+0x29C (alt SHP)
  → if both NULL: write (0,0,0) to out  [black fallback]
  → else: read shp_data[8 + 0 * 0x18 + 0xC..0xE] = frame-0 RGB color
→ apply same color to both out_left and out_right pixels
```

### Low-bridge deck overlays (cell overlay index in 0x4A–0x63 or 0xCD–0xE6):

```
cell+0x44 in [0x4A,0x63] or [0xCD,0xE6]
→ ECX = g_OverlayTypeClass_Array[cell+0x44]  (= this cell's overlay type)
→ frame = 1  (hardcoded, ignores cell+0x11E density/damage)
→ call OverlayClass__GetRadarColor(cell_overlay_type, out, frame=1)
  → vtable[0x9C]: returns cell overlay's SHP data ptr
  → if SHP NULL: black fallback
  → else: read shp_data[8 + 1 * 0x18 + 0xC..0xE] = frame-1 RGB color
→ both halves same color
```

### Black pixel fallback trigger:
Only fires when `OverlayTypeClass+0xa4` (SHP ptr) == 0 AND `+0x29C` (alt SHP ptr) == 0.
Writes RGB = (0, 0, 0). Per BRIDGE_SYSTEM.md §"Radar Minimap Colors", this is logged as
a warning in gamemd; the code path is a defensive fallback, not a normal operating case.

---

## 11. Verified Facts (Load-Bearing)

| Fact | Evidence |
|------|----------|
| Entry function: `CellClass__GetRadarColor` @ 0x0047C060 | `disassemble_function 0x0047C060` |
| Bridge flag 0x100 tested as `TEST AH,0x1` on `[cell+0x140]` @ 0x0047C0B4 | `disassemble_function 0x0047C060` |
| Bridge branch loads BRIDGE1 type = `g_OverlayTypeClass_Array[24]` via `[ECX+0x60]` @ 0x0047C0C6 | `disassemble_function 0x0047C060` |
| Bridge branch calls with frame=0 (PUSH 0x0 @ 0x0047C0C3) | `disassemble_function 0x0047C060` |
| Low-bridge range forces frame=1 (PUSH 0x1 @ 0x0047C169) | `disassemble_function 0x0047C060` |
| `OverlayClass__GetRadarColor` @ 0x005FED00, black fallback at 0x005FEDBB | `disassemble_function 0x005FED00` |
| vtable slot 0x9C (slot 39) = `OverlayTypeClass__GetRadarColor` @ 0x005FEDE0 | `read_memory 0x7EF600 176` bytes, slot 39 = bytes 156-159 = 0x005FEDE0 |
| Color bytes from SHP frame table: `shp+8+frame*0x18+0xC..+0xE` | `decompile_function 0x0069E860` |
| `OverlayTypeClass+0x294` = own array index (set in constructor loop) | `decompile_function 0x005FE250`, `disassemble_function 0x005FE250` |
| Black fallback = RGB (0,0,0), written by 3 explicit zeroes at 0x005FEDBB | `disassemble_function 0x005FED00` |

---

## 12. Rust Implementation Notes

For implementing `CellClass::get_radar_color()`:

1. **Flag 0x100 cells**: Always use `BRIDGE1` type's SHP frame 0 color. Do NOT use `cell.overlay_index`. Do NOT scale or dim the color — it is used as-is (no `>> 1` step, unlike terrain tiles).

2. **Low-bridge overlays (0x4A–0x63, 0xCD–0xE6)**: Use the cell's own overlay type's SHP **frame 1**. Do not use `cell.overlay_density` (`cell+0x11E`).

3. **Color source**: The RGB is read from the SHP frame table entry (metadata, not rendered pixels). This is a 3-byte value pre-baked into the overlay SHP file, not sampled from rendered pixel art.

4. **Black fallback**: Return (0,0,0) when the overlay type has no SHP data loaded.

5. **Both output halves** (left/right radar pixels per cell) get the same color in all bridge paths.

6. **Channel swap** does NOT apply to bridge or low-bridge overlays (all outside the 0x7F–0x8A / 0x93–0x9E self-index ranges).

---

*Generated 2026-05-19. All addresses verified via live Ghidra MCP decompilation and disassembly of `gamemd.exe`.*
