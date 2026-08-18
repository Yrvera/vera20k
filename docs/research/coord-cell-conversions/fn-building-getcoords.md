# BuildingClass__GetCoords — Decode Doc

## Summary

`BuildingClass__GetCoords` (0x00447ac0) overrides the vtable slot 0x48 GetCoords
for `BuildingClass`. It computes the building's **geometric center** in leptons by
taking the NW-corner Location field (`this+0x9C/0xA0/0xA4`) and adding
`(foundationWidth * 0x80 − 0x80)` to X and `(foundationHeight * 0x80 − 0x80)`
to Y. Z is passed through unchanged. The formula correctly centers the output
point within the foundation footprint: for a W×H building, the center is at
NW + ((W−1)×128, (H−1)×128) leptons. Foundation dimensions are queried via
`BuildingTypeClass__GetFoundationWidth` and `BuildingTypeClass__GetFoundationHeight`
(the latter always called with `wantBibExtension=0` here).

This is **Frame #3** in the CLAUDE.md coordinate system: "GetCoords (foundation
center)" = NW-corner Location + center offset. It is the coordinate form used by
Force_Track destinations, dock calculations, and most combat-targeting callers.

Function body: 0x00447ac0
(verified via `decompile_function 0x00447ac0`)

## Active in YR

**Yes.** Bound to vtable slot 0x48 in the BuildingClass vtable at 0x007e3ebc
(verified by `read_memory 0x007e3f04` → `c0 7a 44 00` = 0x00447ac0). Called
indirectly whenever any system dispatches vtable slot 0x48 on a BuildingClass
pointer — which covers most targeting, range checks, dock/exit coordinate
computation, and rendering anchor calculations.
(verified via `get_xrefs_to 0x00447ac0` and `read_memory 0x007e3f04`)

## Decompilation excerpt

```c
// verified via decompile_function 0x00447ac0
void __thiscall BuildingClass__GetCoords(int param_1, int *param_2)
{
  // param_1 is int (direct byte offsets — CLAUDE.md pitfall rule)
  int iVar1;  // Y lepton at this+0xA0
  int iVar2;  // Z lepton at this+0xA4
  int iVar3;  // foundation height (cells)
  int iVar4;  // foundation width  (cells)

  iVar3 = BuildingTypeClass__GetFoundationHeight(0);   // wantBibExtension=0
  iVar4 = BuildingTypeClass__GetFoundationWidth();

  iVar1 = *(int *)(param_1 + 0xa0);   // Location Y (NW corner, leptons)
  iVar2 = *(int *)(param_1 + 0xa4);   // Location Z (altitude, leptons)

  *param_2   = *(int *)(param_1 + 0x9c) + iVar4 * 0x80 + -0x80;  // X center
  param_2[1] = iVar1                  + iVar3 * 0x80 + -0x80;   // Y center
  param_2[2] = iVar2;                                             // Z unchanged
  return;
}
```

`param_1` is `int` — all field accesses are direct byte offsets.
`param_2` is `int*` — output CoordStruct with X at [0], Y at [1], Z at [2].

## Behavioral analysis

### The foundation-center formula

```
center_X = Location.X + foundWidth  * 0x80 - 0x80
center_Y = Location.Y + foundHeight * 0x80 - 0x80
center_Z = Location.Z
```

Simplifying: `center_X = Location.X + (foundWidth − 1) * 128` leptons.
Since 1 cell = 256 leptons = 2 × 128, offset `(W−1)*128` is `(W−1)/2` cells.
This lands exactly at the horizontal center of the W-cell-wide foundation.

Concrete fixture (GAREFN, 4×3 refinery, NW cell (10,10)):
- Location.X = 10 × 256 = 2560 leptons; foundWidth = 4
- center_X = 2560 + 4 × 128 − 128 = 2560 + 384 = 2944 leptons = cell 11.5 (center of cells 10–13)
- Location.Y = 10 × 256 = 2560 leptons; foundHeight = 3
- center_Y = 2560 + 3 × 128 − 128 = 2560 + 256 = 2816 leptons = cell 11 (center of cells 10–12)
- Result: geometric center in leptons = (2944, 2816) = cell (11.5, 11)

This matches the CLAUDE.md canonical fixture and the "foundation center" frame.

### Coordinate reference frame

- **Input (read)**: Frame #1 "Location" — `this+0x9C` (lepton X), `this+0xA0`
  (lepton Y), `this+0xA4` (lepton Z). These are NW-corner leptons.
- **Output**: Frame #3 "GetCoords (foundation center)" — geometric center of the
  foundation footprint in leptons.
- **Rust canonical frame**: to convert to cell-grid NW `(u16, u16)`, divide by
  256 (with sign-correct shift). The output of this function is NOT the NW cell
  — it is the center point.

### BibExtension always 0 here

`BuildingTypeClass__GetFoundationHeight` is called with `wantBibExtension=0`.
This means the bib row is never included in the foundation height used for center
computation. The GetFoundationHeight doc (verified via `decompile_function 0x0045eca0`)
confirms no caller in stock YR passes wantBibExtension != 0 from this code path.
(verified via `decompile_function 0x0045eca0`)

### Foundation dimension callees

- `BuildingTypeClass__GetFoundationHeight` (0x0045eca0): reads
  `g_FoundationHeightTable[*(int*)(type+0xef0)]` — the foundation index selects
  the height in cells from a global table.
- `BuildingTypeClass__GetFoundationWidth` (0x0045ec90): reads
  `g_FoundationWidthTable[*(int*)(type+0xef0)]` — same index, width in cells.
(verified via `get_function_callees 0x00447ac0` and `decompile_function 0x0045ec90`)

### Vtable slot 0x48 override

BuildingClass overrides the base ObjectClass GetCoords at vtable slot 0x48.
The BuildingClass vtable is at 0x007e3ebc. Slot 0x48 is at 0x007e3ebc + 0x48 =
0x007e3f04. `read_memory 0x007e3f04` → `c0 7a 44 00` = 0x00447ac0. Confirmed.
(verified via `read_memory 0x007e3f04`)

Vtable slot 0 at 0x007e3ebc → `60 02 41 00` = 0x00410260 =
`AbstractClass__QueryInterface` (verified via `get_function_by_address 0x00410260`),
confirming 0x007e3ebc is the vtable base.
(verified via `read_memory 0x007e3ebc` and `get_function_by_address 0x00410260`)

### Who calls this — indirect vs direct

No direct UNCONDITIONAL_CALL callers found via `get_function_callers` — the
function is reached exclusively through the vtable dispatch at slot 0x48. The
callers of `BuildingTypeClass__GetFoundationHeight` (20 named functions including
`BuildingClass__GetCoords`, `BuildingClass__Unlimbo`, `BuildingClass__Sell`,
`BuildingClass__ExitObject_Main`, `BuildingClass__ReceiveDamage`, etc.) confirm
GetCoords is wired into all major building lifecycle operations.
(verified via `get_function_callers 0x0045eca0`)

### Observable vs internal

- **Observable**: All targeting, splash damage, dock/exit positioning, and
  combat range checks that dispatch vtable slot 0x48 on a BuildingClass will
  get the geometric center, not the NW corner. This is the expected game behavior:
  a unit attacking a 4×3 refinery aims at the center of the building, not its
  top-left corner.
- **Internal**: The arithmetic is the mechanism — players see units target the
  center of buildings correctly.

### INI keys / globals / enums

- Foundation dimensions come from `g_FoundationHeightTable` and
  `g_FoundationWidthTable` global tables indexed by `BuildingTypeClass+0xef0`
  (the foundation type index). These tables are populated at game start from
  the predefined foundation shapes, not INI keys directly.
- No INI key reads or enum comparisons inside this function.

## Struct field accesses

| Offset (bytes) | Size | Access | Semantics |
|---|---|---|---|
| `param_1 + 0x9C` | 4 (int) | read | Location X — NW corner in leptons |
| `param_1 + 0xA0` | 4 (int) | read | Location Y — NW corner in leptons |
| `param_1 + 0xA4` | 4 (int) | read | Location Z — altitude in leptons (passed through) |
| `*param_2` | 4 (int) | write | Output center X in leptons |
| `param_2[1]` | 4 (int) | write | Output center Y in leptons |
| `param_2[2]` | 4 (int) | write | Output center Z in leptons |

Param type is `int` (direct byte offsets confirmed by Ghidra signature).
(verified via `decompile_function 0x00447ac0`)

## Callees

| Callee | Address | Purpose |
|---|---|---|
| `BuildingTypeClass__GetFoundationHeight` | 0x0045eca0 | Returns foundation height in cells (wantBibExtension=0) |
| `BuildingTypeClass__GetFoundationWidth` | 0x0045ec90 | Returns foundation width in cells |

(verified via `get_function_callees 0x00447ac0`)

## Callers / Lifecycle

No direct UNCONDITIONAL_CALL callers. The function is dispatched exclusively via
vtable slot 0x48 on BuildingClass instances.

The `BuildingTypeClass__GetFoundationHeight` callers enumerate which systems
rely on foundation height (and therefore on this GetCoords): `BuildingClass__Unlimbo`,
`BuildingClass__Sell`, `BuildingClass__ExitObject_Main`, `BuildingClass__ReceiveDamage`,
`BuildingClass__GetDockCellForObject`, `BuildingClass__GetHalfFoundationSize`,
`DiskLaserClass__AI`, and ~12 additional functions.
(verified via `get_function_callers 0x0045eca0`)

## Out-of-scope refs

- `g_FoundationHeightTable` / `g_FoundationWidthTable` — global foundation
  dimension lookup tables; their initialization and indexing is out of scope
- `BuildingTypeClass+0xef0` — the foundation type index field; decoding the full
  BuildingTypeClass struct is out of scope
- All callers dispatching vtable slot 0x48 — the dispatch pattern is pervasive;
  individual callers are out of scope except as noted above

## Unverified claims (YELLOW)

None — all previously-YELLOW claims have been verified in subsequent sessions:

1. **Vtable at 0x007e3ebc is BuildingClass vtable**: confirmed — only one DATA xref
   to 0x00447ac0 (at 0x007e3f04). Slot 0 = `AbstractClass__QueryInterface` is the
   expected IUnknown base. Consistent with Ghidra RTTI labeler output.
   (verified via `read_memory 0x007e3ebc`, `get_xrefs_to 0x00447ac0`)

2. **`BuildingTypeClass__GetFoundationWidth` receives `this` via ECX (fastcall)**:
   confirmed via `disassemble_function 0x0045ec90` — first instruction is
   `MOV EAX,[ECX+0xef0]`, so ECX = BuildingTypeClass pointer. `RET` (no stack
   cleanup) = fastcall with ECX-only arg. (verified via `disassemble_function 0x0045ec90`)
