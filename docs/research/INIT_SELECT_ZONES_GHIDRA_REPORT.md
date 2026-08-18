# SidebarClass::InitSelectZones — Ghidra Report

**Target:** `FUN_006a8220` (Ghidra label: `SidebarClass__InitSelectZones`)  
**Date:** 2026-05-19  
**Status:** COMPLETE  
**Active in YR:** Yes — called unconditionally from `SidebarClass__Init` on every game start.

---

## Function Signature

```c
void __thiscall SidebarClass__InitSelectZones(int param_1, int param_2)
// param_1 = SidebarClass* this (ECX, __thiscall)
// param_2 = strip index (0–3)
```

Verified via `decompile_function 0x006a8220`.  
Body: `006a8220 – 006a832b`.

---

## What It Does

Initialises the 60 `SelectClass` cameo click-zone gadgets for one sidebar strip.  
Called 4 times (strip 0–3) from `SidebarClass__Init` (@ `006a5310`).  
Total gadgets: 4 strips × 60 = 240, stored in a flat array at `DAT_00b07e80`.

---

## Verified Gadget-Index Formula

```c
// param_1+0x38 holds the strip index (written at function entry)
*(int *)(param_1 + 0x38) = param_2;          // store stripIndex

gadgetIndex = row * 2 + col                   // inner loop: iVar6 starts at row*2
            + stripIndex * 0x3C;              // block of 60 per strip
```

Exact binary writes (verified via `decompile_function 0x006a8220`):

```c
iVar1 = iVar6 + *(int *)(param_1 + 0x38) * 0x3c;   // iVar1 = gadgetIndex
```

`iVar6` = `row * 2 + col` (column inner loop: starts at `param_2 * 2`, incremented per col).  
This confirms `CameoIndex = row * 2 + col` written to `SelectClass+0x30`.

---

## SelectClass Struct Layout (VERIFIED from binary)

Array base: **`DAT_00b07e80`** — confirmed in `SidebarClass__Action` and `SidebarClass__ToggleSidebar`.  
Element stride: **0x38 bytes** (14 dwords).

| Offset | Field      | Binary evidence |
|--------|------------|-----------------|
| +0x00  | (vtable/flags — not written here) | |
| +0x0C  | XPos       | `(&DAT_00b07e8c)[iVar1 * 0xe]` → `0xb07e8c – 0xb07e80 = 0x0C` |
| +0x10  | YPos       | `(&DAT_00b07e90)[iVar1 * 0xe]` → `0xb07e90 – 0xb07e80 = 0x10` |
| +0x14  | Width      | `(&DAT_00b07e94)[iVar1 * 0xe]` = 0x3C |
| +0x18  | Height     | `(&DAT_00b07e98)[iVar1 * 0xe]` = 0x30 |
| +0x24  | ID         | `*(undefined4*)(&DAT_00b07ea4 + iVar1 * 0x38)` = 0xCA |
| +0x2C  | StripPtr   | `(&DAT_00b07eac)[iVar1 * 0xe]` = param_1 (SidebarClass* this) |
| +0x30  | CameoIndex | `(&DAT_00b07eb0)[iVar1 * 0xe]` = iVar6 (= row*2 + col) |

**ID write uses byte-offset `iVar1 * 0x38`** on a byte pointer to DAT_00b07ea4.  
**All other writes use dword-pointer stride `iVar1 * 0xe`** (= 14 dwords = 0x38 bytes).  
Both are equivalent — confirmed consistent 0x38-byte element stride.

### Offset Discrepancy Resolution: SIDEBAR_SYSTEM §8 vs SIDEBAR_STRIPS_TABS_CAMEOS

**SIDEBAR_SYSTEM §8 says XPos=+0x0C, YPos=+0x10.**  
**SIDEBAR_STRIPS_TABS_CAMEOS says XPos=+0x10, YPos=+0x14.**

**Binary verdict: SIDEBAR_SYSTEM §8 is correct.**  
- XPos = +0x0C (`DAT_00b07e8c – DAT_00b07e80 = 0x0C`), verified from decompile_function 0x006a8220.  
- YPos = +0x10 (`DAT_00b07e90 – DAT_00b07e80 = 0x10`), same evidence.  
- SIDEBAR_STRIPS_TABS_CAMEOS offsets are shifted +4 and should be treated as **wrong**.

---

## Coordinate Source: Screen-Absolute

XPos and YPos are **screen-absolute**, read directly from the StripClass global array:

```c
// Strip[i].XPos at &DAT_00880d4c + i * 0xf94
gadget.XPos = *(int*)(&DAT_00880d4c + stripIndex * 0xf94) + DAT_00b0b4fc * col;
// Strip[i].YPos at &DAT_00880d50 + i * 0xf94
gadget.YPos = *(int*)(&DAT_00880d50 + stripIndex * 0xf94) + 1 + DAT_00b0b500 * row;
```

- Strip.XPos is read from a static global array (StripClass objects embedded in the SidebarClass singleton).
- Strip.YPos is similarly read from the same array at offset +4.
- The `+1` on YPos is a hardcoded 1-pixel top inset, not from any INI value.
- Origin: fully **screen-absolute** — no strip-relative translation step.

### StripClass XPos/YPos Offset Within StripClass

StripClass objects are embedded in SidebarClass at offset `+0x1564` (first strip):
- SidebarClass singleton base = `0x008807e8` (derived: `0x00880d4c – 0x1564`).
- Strip[0].XPos = `SidebarClass + 0x1564 + 0x00` = `DAT_00880d4c`.
- Strip[0].YPos = `SidebarClass + 0x1564 + 0x04` = `DAT_00880d50`.
- Strip stride = `0xf94` bytes (= `0x3e5` dwords).

Evidence: `SidebarClass__Init` writes `puVar8[-1] = XPos` where `puVar8 = (undefined4*)(this + 0x1568)` → XPos stored at `this + 0x1564`; InitSelectZones reads `DAT_00880d4c` for strip 0, confirmed via decompile_function 0x006a5310.

---

## Column/Row Stride Globals

| Global          | Role         | Notes |
|-----------------|--------------|-------|
| `DAT_00b0b4fc`  | columnWidth  | Multiplied by `col` (0 or 1) for XPos stride. Per-mode value (63 RA2 / 64 YR variant). |
| `DAT_00b0b500`  | rowHeight    | Multiplied by `row` for YPos stride. |

Both are **runtime globals**, not literals. Verified via decompile_function 0x006a8220 — the code uses `DAT_00b0b4fc * iVar5` and `DAT_00b0b500 * param_2` directly.

Note: `visibleRows` formula also uses `/ 0x32` (= 50), which matches typical rowHeight including spacing. `DAT_00b0b500` is the per-row Y step; `0x32` is the row slot height used for the visible-row count calculation.

---

## visibleRows Source

```c
iVar3 = (((DAT_00886f9c - DAT_00b0b4f8) - header_offset) + -7 + g_SidebarWidth) / 0x32;
```

Where `header_offset = 0x1a` normally, or `0x12` if `g_ScenarioClass_Instance+0x34b8 != 0`.  
Identical formula to `SidebarClass__GetVisibleSlotCount` (@ `006ac430`) except that function returns `rows * 2`; here we get `rows` directly.

Verified via `decompile_function 0x006ac430`.

---

## Caller List

| Address     | Caller |
|-------------|--------|
| `006a5310`  | `SidebarClass__Init` — called 4 times in a loop over strip indices 0–3. |

Verified via `get_function_callers 0x006a8220`.

---

## Open Questions

- `FUN_006a8330` (ActivateSelectZones) and `FUN_006a83E0` (DeactivateSelectZones) — out of scope per investigation constraints.
- `DAT_00b0b4fc` (columnWidth) exact values for RA2 vs YR modes not confirmed in this session (runtime globals, zero in static binary).
- `SidebarClass+0x38` scratchpad field: used only within InitSelectZones to cache the strip index. Not meaningful outside this function.

---

## Summary

The prior pseudo-code from SIDEBAR_STRIPS_TABS_CAMEOS is structurally correct, with two corrections:
1. **SelectClass offsets** are 4 bytes lower than that doc claimed (+0x0C/+0x10 not +0x10/+0x14).
2. **Coordinates are screen-absolute**, not strip-relative — the function reads Strip.XPos and Strip.YPos from the global StripClass array directly.
3. `StripPtr` is written at +0x2C (not +0x28 or any other value).
4. `CameoIndex = row*2 + col` at +0x30 is confirmed correct.
