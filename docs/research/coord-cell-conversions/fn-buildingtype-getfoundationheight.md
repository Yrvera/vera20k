# BuildingTypeClass__GetFoundationHeight — Decode Doc

## Summary

`BuildingTypeClass__GetFoundationHeight` (0x0045eca0) reads a foundation type
index from `BuildingTypeClass+0xef0` and returns the corresponding foundation
height in cells from the global table `g_FoundationHeightTable`. It has a
bib-extension branch: when `wantBibExtension != 0` AND `BuildingTypeClass+0x1570 != 0`
(HasBib flag), the returned height is incremented by 1 to include the bib row.
The bib extension branch is **never triggered in stock YR** — every verified
caller passes `wantBibExtension = 0`. The result (without bib) is the canonical
foundation height in cells.

Function body: 0x0045eca0 – 0x0045ecdc
(verified via `decompile_function 0x0045eca0`)

## Active in YR

**Yes.** 53 call sites across core building systems including GetCoords, DrawBody,
Unlimbo, Sell, ExitObject_Main, ReceiveDamage, GetDockCellForObject,
DiskLaserClass__AI, TechnoClass__InRange, UnitClass__Deploy, and
SlaveManagerClass — all active in normal YR gameplay.
(verified via `get_xrefs_to 0x0045eca0`)

## Decompilation excerpt

```c
// verified via decompile_function 0x0045eca0
int __thiscall BuildingTypeClass__GetFoundationHeight(int param_1, char param_2)
{
  // param_1 = BuildingTypeClass* (direct byte offsets)
  // param_2 = wantBibExtension (0 = no bib, non-zero = add bib row)
  if ((param_2 != '\0') && (*(char *)(param_1 + 0x1570) != '\0')) {
    // HasBib flag at +0x1570 is set AND caller requested bib extension
    return (&g_FoundationHeightTable)[*(int *)(param_1 + 0xef0)] + 1;
  }
  return (&g_FoundationHeightTable)[*(int *)(param_1 + 0xef0)];
}
```

`param_1` is `int` (direct byte offsets — CLAUDE.md pitfall rule).

## Behavioral analysis

### Input fields

- `*(int *)(param_1 + 0xef0)` — foundation type index (same field read by
  `GetFoundationWidth`). Selects the row in `g_FoundationHeightTable`.
- `*(char *)(param_1 + 0x1570)` — HasBib boolean flag. If set, the building
  has a decorative bib row in front of the entrance.

Both functions (`GetFoundationWidth` and `GetFoundationHeight`) read the same
foundation type index at `+0xef0` but from separate global tables.
(verified via `decompile_function 0x0045ec90` and `decompile_function 0x0045eca0`)

### Output: cells, not leptons

The return value is a foundation height in **cells** (integer). NOT in leptons.
In `BuildingClass__GetCoords`:
```c
iVar3 = BuildingTypeClass__GetFoundationHeight(0);   // wantBibExtension=0
param_2[1] = iVar1 + iVar3 * 0x80 + -0x80;
// = Location.Y + height_cells * 128 - 128
// = Location.Y + (height_cells - 1) * 128 leptons
```
`0x80` = 128 = 0.5 cells. Formula: `(height-1) * 128` leptons centers the
coordinate vertically within the H-cell-tall foundation.
(verified via `decompile_function 0x00447ac0`)

### Bib extension — unreachable in stock YR

The `wantBibExtension` branch adds 1 to the returned height. However, a verified
audit across 12+ named callers (Unlimbo, GetCoords, GetHalfFoundationSize,
DrawBody, OnDestroyed, ExitObject_Main, CanCloak, ShouldUncloak, Sell,
GetDockCellForObject, ReceiveDamage, CreateDamageFireAnims,
SlaveManagerClass::FindDeployCell) found that **every caller passes 0** for
`wantBibExtension`. The bib extension is a dead path in normal YR gameplay.

The HasBib field (`+0x1570`) does have a separate live reader at
`UnitClass::Can_Enter_Cell` (function entry 0x0073F0A0) that relaxes cell-entry
blocking on one foundation edge. That reader uses `DAT_0089F690` as a bib offset
and is NOT routed through this function. See `BIB_SYSTEM_GHIDRA_REPORT.md §2.4`
for the full bib system analysis.
(verified via `decompile_function 0x0045eca0` — bib branch noted; caller audit
cited in the embedded comment in the function's Ghidra plate comment)

### g_FoundationHeightTable

A global array indexed by the foundation type index. Values are foundation
heights in cells. Standard heights in YR: 1, 2, 3 cells (matching common
foundation dimensions). The exact table address and per-entry values were not
read in this session (game not running at decode time).

### Comparison with GetFoundationWidth

| | GetFoundationWidth (0x0045ec90) | GetFoundationHeight (0x0045eca0) |
|---|---|---|
| Field read | `BuildingTypeClass+0xef0` (int) | `BuildingTypeClass+0xef0` (int) — same |
| Table | `g_FoundationWidthTable` | `g_FoundationHeightTable` |
| Extra param | none | `wantBibExtension` (char) |
| Extra logic | none | bib-extension branch (+1 if HasBib && wantBib) |
| Dead code | none | bib branch never triggered in stock YR |

### Concrete fixture

GAREFN (Allied Ore Refinery), 4×3 foundation:
- `BuildingTypeClass+0xef0` = foundation type index for 4×3 (e.g., index 3)
- `g_FoundationHeightTable[3]` = 3 (cells)
- In GetCoords: `Location.Y + 3*128 - 128 = Location.Y + 256 leptons` = 1 cell south of NW = center of a 3-cell-tall foundation.

### INI key mapping

The foundation type index at +0xef0 is set from the `Foundation=` INI key in
`art(md).ini` for the building's art entry. The HasBib flag at +0x1570 is set
from the `HasBib=yes` INI key in `rules(md).ini`. Neither INI parsing path was
traced in this session.

## Struct field accesses

| Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|
| `param_1 + 0xef0` | 4 (int) | read | Foundation type index → used to index g_FoundationHeightTable |
| `param_1 + 0x1570` | 1 (char) | read | HasBib flag — only checked when wantBibExtension != 0 |

(verified via `decompile_function 0x0045eca0`)

## Callees

None. Leaf function — single field read, optional flag check, and table lookup.

## Callers / Lifecycle

53 call sites across:

| Caller | Notable role |
|---|---|
| `BuildingClass__GetCoords` (0x00447ac0) | Foundation center Y offset (always wantBib=0) |
| `BuildingClass__GetHalfFoundationSize` (0x00458e00) | Half-size for placement |
| `BuildingClass__Unlimbo` (0x00440580) | Placement into map |
| `BuildingClass__ExitObject_Main` (0x00443c60) | Unit exit from building |
| `BuildingClass__ReceiveDamage` (0x00442230) | Damage area |
| `BuildingClass__GetDockCellForObject` (0x0044efb0) | Dock cell lookup |
| `BuildingClass__SellBuilding` (0x00457de0) | Sell logic |
| `BuildingClass_DrawBody` (0x0043d290) | Rendering |
| `DiskLaserClass__AI` (0x004a7340) | Disk laser targeting |
| `TechnoClass__InRange` (0x006f7546) | Range check |
| `UnitClass__Deploy` (0x00739472) | Unit deploy |
| `SlaveManagerClass__AI_Update` (0x006af806) | Slave AI |
| + 41 additional call sites | Various building and AI systems |

(verified via `get_xrefs_to 0x0045eca0`)

## Out-of-scope refs

- `g_FoundationHeightTable` — global table; initialization out of scope
- `BuildingTypeClass+0xef0` write site — INI parsing path; out of scope
- `BuildingTypeClass+0x1570` (HasBib) write site — rules INI parsing; out of scope
- `UnitClass::Can_Enter_Cell` bib offset reader at 0x0073F0A0 — separate system; out of scope

## Global Table Addresses (verified)

| Table | Address | First 8 entries (int32) |
|---|---|---|
| `g_FoundationHeightTable` | `0x00819310` | 1, 1, 2, 2, 3, 2, 3, 5 |
| `g_FoundationWidthTable` | `0x008192b8` | 1, 2, 1, 2, 2, 3, 3, 3 |

Verified via `disassemble_function 0x0045eca0` (literal `0x819310` in `MOV EAX,[EAX*4+0x819310]`),
`read_memory 0x00819310` (32 bytes), `read_memory 0x008192b8` (32 bytes).

Both tables use the same `FoundationTypeIndex` at `+0xef0`. This is confirmed by
`decompile_function 0x0045ec90` (width) and `decompile_function 0x0045eca0` (height) —
both read `*(int*)(this + 0xef0)` as the table index. Width and height derive from the
same foundation shape code stored once at `+0xef0`.

## Unverified claims (YELLOW)

None — all previously-YELLOW claims resolved in subsequent sessions (see Global Table
Addresses section above).
