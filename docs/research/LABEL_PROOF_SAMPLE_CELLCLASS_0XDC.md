# Label Proof Sample: CellClass+0xDC Reservation Bitmask

Date: 2026-05-31
Mode: exhaustive-slice sample for label validation only. No Rust implementation.

## Question

Can the `CellClass+0xDC` field be safely labeled as a per-house reservation/placement mask,
and can we reject the stale/misleading label that treats it as GapGen or visibility state?

## Verdict

`CellClass+0xDC` is correctly labeled as a per-house placement/reservation-style bitmask.
It is not the cell visibility mask and is not the GapGen/visibility shared mask.

Confidence: HIGH for "per-house reservation/placement mask" and HIGH for "not visibility/GapGen".
The exact setter lifecycle remains outside this sample.

## Binary Evidence

### Reader: `FUN_0050b760`

Ghidra decompile, read-only, 2026-05-31:

```c
if (g_GameMode == 0) {
  return 1;
}
iVar1 = *(int *)(g_RulesClass_Instance + 0x1460);
iVar3 = BuildingTypeClass__GetFoundationWidth();
iVar3 = iVar3 + iVar1 * 2;
iVar4 = BuildingTypeClass__GetFoundationHeight(0);
uVar2 = *(undefined4 *)(param_1 + 0x30);
iVar4 = iVar4 + iVar1 * 2;
...
iVar5 = MapClass__Get_CellClass(&param_2);
if ((*(uint *)(iVar5 + 0xdc) & 1 << ((byte)uVar2 & 0x1f)) != 0) {
  return 1;
}
```

Facts:

- The function scans a rectangle derived from a building foundation plus `RulesClass+0x1460`.
- It calls `MapClass__Get_CellClass` for each candidate cell.
- It tests `cell + 0xDC` as a 32-bit bitmask.
- The bit index comes from `param_1 + 0x30`, which is a house/player index style value.
- It returns as soon as any scanned cell has the matching bit set.

Interpretation:

This is a per-house cell reservation/placement collision check, not a visual visibility test.

### Distinct visibility field: `CellClass__IsVisibleToHouse @ 004870b0`

Ghidra decompile, read-only, 2026-05-31:

```c
bool __thiscall CellClass__IsVisibleToHouse(int param_1,byte param_2)
{
  return (*(uint *)(param_1 + 0x78) & 1 << (param_2 & 0x1f)) != 0;
}
```

Facts:

- Visibility reads `cell + 0x78`.
- It does not read `cell + 0xDC`.

### GapGen/visibility-style writer: `FUN_00487110`

Ghidra decompile, read-only, 2026-05-31:

```c
void __thiscall FUN_00487110(int param_1,byte param_2)
{
  *(uint *)(param_1 + 0x78) = *(uint *)(param_1 + 0x78) | 1 << (param_2 & 0x1f);
  return;
}
```

Facts:

- This writer sets bits at `cell + 0x78`.
- It does not touch `cell + 0xDC`.
- Any label claiming `+0xDC` is the GapGen/visibility mask conflicts with this direct reader/writer pair.

### Constructor initialization: `CellClass__Constructor @ 0047BC50`

Relevant constructor writes:

```c
param_1[0x1e] = 0;  // offset 0x78
...
param_1[0x37] = 0;  // offset 0xDC
```

Facts:

- Both fields are initialized independently.
- The binary keeps two separate dword masks, not one aliased field.

### Xrefs to `FUN_0050b760`

Ghidra xrefs, read-only, 2026-05-31:

```text
From 00506a33 in FUN_005060b0 [UNCONDITIONAL_CALL]
From 00444fba in BuildingClass__ExitObject_Main [UNCONDITIONAL_CALL]
```

Facts:

- The reader is called from building/placement-adjacent code.
- The sample did not fully classify every caller branch, so the setter lifecycle and full active scenario
  should remain a follow-up target.

## Label Recommendation

Recommended label:

```text
CellClass+0xDC = ReservationBitmask / PlacementReservationMask
```

Do not label:

```text
GapGen
Visibility
Shroud
Fog
```

Safer long-form label until setter lifecycle is fully drained:

```text
CellClass+0xDC = PerHousePlacementReservationMask_UNCHECKED_SETTER
```

## Why This Is A Good Label-Proof Sample

This sample uses the minimum proof needed for high-confidence label validation:

1. Verify the field reader directly.
2. Verify nearby competing labels use a different offset.
3. Verify constructor independence.
4. Check xrefs for system context.
5. Preserve the unresolved part instead of over-naming it.

The same pattern should be used for high-risk struct fields and bitflags before a Ghidra rename becomes
load-bearing for Rust parity work.

## Open Follow-Ups

- Find every writer to `CellClass+0xDC`.
- Prove whether the setter is AI-only, base-placement-only, or has other active YR callers.
- Tie `RulesClass+0x1460` to the exact INI key/default with binary reader evidence.
- Decide whether the final Ghidra field name should include `AIBasePlacement` or stay at the more general
  `PlacementReservationMask`.
