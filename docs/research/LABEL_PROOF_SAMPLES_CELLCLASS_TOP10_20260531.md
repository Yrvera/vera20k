# Label Proof Samples: CellClass Top 10 Fields And Bitflags

Date: 2026-05-31
Mode: label-validation sample set, read-only Ghidra evidence, no Rust implementation.

## Purpose

This is a reusable sample of the proof standard for high-impact struct fields and bitflags.
The target is `CellClass` because stale names here can poison map, pathing, placement, shroud,
bridge, overlay, ore, and object-list research.

Existing Ghidra labels were treated as hints only. Each sample below records the field, the safe label,
the evidence, and the rename/trust recommendation.

## Summary Ledger

| # | Offset / bit | Safe label | Confidence | Action |
|---|---|---|---|---|
| 1 | `+0x78` | `VisibleToHousesGapGenMask` | LIVE_VERIFIED | KEEP/RENAME |
| 2 | `+0xDC` | `PerHousePlacementReservationMask` | LIVE_VERIFIED reader, setter unresolved here | KEEP/RENAME conservative |
| 3 | `+0xE4`, `+0xE8` | `FirstObject`, `AltObject` | LIVE_VERIFIED | KEEP |
| 4 | `+0x124`, `+0x128` | `OccupationFlags`, `AltOccupationFlags` | LIVE_VERIFIED | KEEP |
| 5 | `+0x4C` | `ZoneType` / `ReducedZoneType` | LIVE_VERIFIED | RENAME if still generic |
| 6 | `+0xEC` | `LandType` | LIVE_VERIFIED | KEEP |
| 7 | `+0x11B`, `+0x140 bit 0x80` | `Level`, `HasBridgeOverlayHeight` | LIVE_VERIFIED | KEEP |
| 8 | `+0x44`, `+0x11E` | `OverlayTypeIndex`, `OverlayData` | LIVE_VERIFIED | KEEP |
| 9 | `+0x122` | `BlockerNeighborCount` | LIVE_VERIFIED reader | KEEP/RENAME |
| 10 | `+0x12C bits 0x08/0x10` | `ShroudFlags` explored/redraw bits | LIVE_VERIFIED | KEEP |

## 1. `CellClass+0x78`: VisibleToHousesGapGenMask

Recommended label:

```text
CellClass+0x78 = VisibleToHousesGapGenMask
```

Evidence:

- `CellClass__IsVisibleToHouse @ 004870b0` reads:

```c
return (*(uint *)(param_1 + 0x78) & 1 << (param_2 & 0x1f)) != 0;
```

- `FUN_00487110 @ 00487110` writes:

```c
*(uint *)(param_1 + 0x78) = *(uint *)(param_1 + 0x78) | 1 << (param_2 & 0x1f);
```

- `CellClass__Constructor @ 0047BC50` initializes `param_1[0x1e] = 0`, which is offset `0x78`.

Conclusion:

`+0x78` is a 32-bit per-house bitmask used by visibility-style readers/writers. It is distinct from
`+0xDC`.

Risk if mislabeled:

Merging this with `+0xDC` would corrupt shroud/GapGen and placement-reservation reasoning.

## 2. `CellClass+0xDC`: PerHousePlacementReservationMask

Recommended conservative label:

```text
CellClass+0xDC = PerHousePlacementReservationMask
```

If setter lifecycle is not fully drained in the active Ghidra program, use:

```text
CellClass+0xDC = PerHousePlacementReservationMask_UNCHECKED_SETTER
```

Evidence:

- `FUN_0050b760 @ 0050b760` scans a building-foundation-shaped rectangle, calls
  `MapClass__Get_CellClass`, and reads:

```c
if ((*(uint *)(iVar5 + 0xdc) & 1 << ((byte)uVar2 & 0x1f)) != 0) {
  return 1;
}
```

- The bit index comes from `param_1 + 0x30`, a house/player-index-style value.
- `FUN_0050b760` has xrefs from `FUN_005060b0` and `BuildingClass__ExitObject_Main`.
- `+0x78` is separately proven as the visibility-style mask above.
- `CellClass__Constructor @ 0047BC50` initializes `param_1[0x37] = 0`, which is offset `0xDC`.

Conclusion:

`+0xDC` is not GapGen and not visibility. It is a separate per-house placement/reservation-style mask.

Risk if mislabeled:

A wrong `GapGen` or `visibility` name here would cause Rust to conflate two unrelated per-house masks.

## 3. `CellClass+0xE4/+0xE8`: FirstObject / AltObject

Recommended labels:

```text
CellClass+0xE4 = FirstObject
CellClass+0xE8 = AltObject
```

Evidence:

- `CellClass__AddContent @ 0047E8A0` selects one list based on the caller-passed layer argument:

```c
if (in_stack_00000008 == '\0') {
  puVar9 = this->FirstObject;
} else {
  puVar9 = this->AltObject;
}
```

- Buildings (`WhatAmI == 6`) append to the selected list tail through object link `+0x30`.
- Non-buildings prepend to `FirstObject` or `AltObject`.
- `CellClass__RemoveContent @ 0047EA90` selects the same one list and unlinks through object link `+0x30`.
- Xrefs:

```text
CellClass__AddContent <- TechnoClass__EnterCell_AddToMultiCells @ 005684bb
CellClass__RemoveContent <- TechnoClass__ExitCell_RemoveFromMultiCells @ 005688eb
```

Conclusion:

These are two independent per-cell object-list heads: ground and alternate/bridge layer. Removal touches
only the selected list.

Risk if mislabeled:

Bad labels here break object ordering, bridge/ground separation, conceal/unlimbo parity, and foundation
occupancy reconstruction.

## 4. `CellClass+0x124/+0x128`: OccupationFlags / AltOccupationFlags

Recommended labels:

```text
CellClass+0x124 = OccupationFlags
CellClass+0x128 = AltOccupationFlags
```

Evidence:

- `ObjectClass__Mark_Occupation @ 007441B0` sets vehicle bit `0x20`:

```c
if ((groundHeight + DAT_00b1d0ac <= objectZ) && ((*(uint *)(cell + 0x140) & 0x100) != 0)) {
  *(uint *)(cell + 0x128) |= 0x20;
  return;
}
*(uint *)(cell + 0x124) |= 0x20;
```

- `ObjectClass__Clear_Occupation @ 00744210` clears vehicle bit `0x20`:

```c
if (groundHeight + DAT_00b1d0ac <= objectZ) {
  *(uint *)(cell + 0x128) &= 0xffffffdf;
  return;
}
*(uint *)(cell + 0x124) &= 0xffffffdf;
```

- `CellClass__CheckCellPassability @ 004834A0` selects `AltOccupationFlags` when required height is
  `Level + 4` and `Flags & 0x100` is set, otherwise `OccupationFlags`; it then applies masks `& 0xE0`
  and `& 0x5F`.

Conclusion:

`+0x124` and `+0x128` are separate ground/alternate occupation dwords. Bit `0x20` is vehicle occupancy in
the sampled writer path. The bridge gate is asymmetric: mark requires `Flags & 0x100`; clear only checks
height.

Risk if mislabeled:

Conflating these with object lists or using one unified occupancy field will drift bridge traversal and
path/placement checks.

## 5. `CellClass+0x4C`: ZoneType / ReducedZoneType

Recommended label:

```text
CellClass+0x4C = ZoneType
```

Safer long form:

```text
CellClass+0x4C = ReducedZoneType
```

Evidence:

- `CellClass__RecalcZoneType @ 00483C80` writes `this->field_0x4c` to values `0..7`.
- Direct outcomes include:

```c
field_0x4c = 7; // outside playfield
field_0x4c = 1; // crushable overlay
field_0x4c = 2; // wall
field_0x4c = 6; // impassable
field_0x4c = 4; // water
field_0x4c = 3; // beach
field_0x4c = 5; // building
field_0x4c = 0; // default ground
```

- `CellClass__RecalcAttributes @ 0047D2B0` calls `CellClass__RecalcZoneType` and mirrors the resulting
  byte into the compact zone-map arrays.
- Xrefs to `CellClass__RecalcZoneType` are only from `CellClass__RecalcAttributes`.

Conclusion:

`+0x4C` is a reduced derived zone type, not raw terrain land type.

Risk if mislabeled:

Using raw terrain/LandType where native expects reduced ZoneType breaks movement-zone passability and zone
connectivity.

## 6. `CellClass+0xEC`: LandType

Recommended label:

```text
CellClass+0xEC = LandType
```

Evidence:

- `CellClass__RecalcAttributes @ 0047D2B0` writes `this->LandType` from overlay type land data, tile
  land data, or fallback terrain logic.
- `CellClass__RecalcZoneType @ 00483C80` reads `this->LandType` and maps it into reduced `+0x4C`.
- `CellClass__CheckCellPassability @ 004834A0` reads:

```c
iVar4 = this->LandType;
...
if (g_SpeedType_LandType_Table[speed + iVar4 * 9] == 0.0 && !bridgeLayer) {
  return 0;
}
```

Conclusion:

`+0xEC` is the raw terrain land type used by speed/cost and by `RecalcZoneType`; it is distinct from
`+0x4C`.

Risk if mislabeled:

Collapsing `LandType` and `ZoneType` causes passability-table row/column confusion.

## 7. `CellClass+0x11B` and `CellClass+0x140 bit 0x80`: Level and bridge-height bit

Recommended labels:

```text
CellClass+0x11B = Level
CellClass+0x140 bit 0x80 = HasBridgeOverlayHeight
```

Evidence:

- `CellClass__GetEffectiveHeight @ 00487D50` returns:

```c
(int)*(char *)(cell + 0x11b) + ((*(uint *)(cell + 0x140) >> 7) & 1) * 4
```

- The `+0x11B` read is signed (`*(char *)`).
- `AStar_main_loop @ 00429A90` also reads `+0x11B` signed and adds `4` when bridge conditions apply.
- `CellClass__RecalcAttributes @ 0047D2B0` mirrors `this->Level` into compact zone-map storage.

Conclusion:

`+0x11B` is signed cell level. `+0x140 bit 0x80` contributes +4 to effective height. Do not model this as
floating elevation.

Risk if mislabeled:

Wrong signedness or a missing bridge +4 causes pathing, bridge occupancy, height gates, and render ordering
drift.

## 8. `CellClass+0x44/+0x11E`: OverlayTypeIndex / OverlayData

Recommended labels:

```text
CellClass+0x44 = OverlayTypeIndex
CellClass+0x11E = OverlayData
```

Evidence:

- `CellClass__PlaceTiberium @ 00487190` uses `+0x11E` as tiberium density:

```c
if (currentDensity == maxDensity - 1) ...
*(byte *)(cell + 0x11e) = clampedDensity;
```

- Germination writes:

```c
*(char *)(cell + 0x11e) = (char)densityAmount;
```

- `CellClass__Reduce_Tiberium @ 00480A80` full removal writes:

```c
param_1->OverlayTypeIndex = -1;
param_1->field_0x11e = 0;
CellClass__RecalcAttributes(param_1);
```

- `CellClass__GetRadarColor @ 0047C060` reads `+0x44` as overlay type and passes `+0x11E` as overlay or
  tiberium visual data.

Conclusion:

`+0x44` is the overlay type index. `+0x11E` is overlay payload data, including ore/tiberium density in the
sampled paths.

Risk if mislabeled:

Overlay visual state, ore amount mirroring, wall/bridge overlay state, radar color, and harvest behavior
will diverge.

## 9. `CellClass+0x122`: BlockerNeighborCount

Recommended label:

```text
CellClass+0x122 = BlockerNeighborCount
```

Conservative label if writer coverage is incomplete:

```text
CellClass+0x122 = BlockerNeighborCount_UNCHECKED_WRITERS
```

Evidence:

- `CellClass__Constructor @ 0047BC50` initializes:

```c
*(undefined1 *)(cell + 0x122) = 0;
```

- `AStar_main_loop @ 00429A90` reads it as a boolean gate:

```c
if ((*(char *)(neighborCell + 0x122) == '\0') && (param_7 != '\0')) goto skip_neighbor;
```

Conclusion:

The sampled active reader proves this is not ore and not fog. It is a pathing blocker-neighbor style byte
read as boolean by A*.

Risk if mislabeled:

Bad names here send pathfinding work toward ore/fog code and hide an A* adjacency rule.

## 10. `CellClass+0x12C`: ShroudFlags

Recommended label:

```text
CellClass+0x12C = ShroudFlags
```

Evidence:

- `CellClass__RevealShroudFlags @ 004876F0` writes:

```c
*(uint *)(cell + 0x12c) |= 0x18;
```

- The same function conditionally writes `cell + 0x140 bit 0x20` when `cell + 0x130 > 0`.
- `CellClass__Constructor @ 0047BC50` initializes adjacent shroud/fog edge bytes at `+0x120` and `+0x121`
  to `0xFE`, then later initializes the dword region containing `+0x12C` through `param_1[0x4b]` masking
  and related zeroing.

Conclusion:

`+0x12C` is a shroud-state flag dword. In this sampled writer, bits `0x08` and `0x10` are set together by
reveal.

Risk if mislabeled:

Conflating this with TS fog, radar color, or per-house visibility would produce wrong explored/unexplored
state and redraw behavior.

## Labeling Pattern Demonstrated

Each proof sample follows the same rule:

1. Confirm constructor/default where possible.
2. Find a direct reader and a direct writer.
3. Check caller/xref context for system role.
4. Separate similar-looking fields instead of merging them.
5. Use conservative names when writer or active-path coverage is incomplete.

## Recommended Next Label Batch

Next broad orchestrator wave should keep the same proof style and expand to:

1. Remaining `CellClass+0x140` bits, especially `0x100`, `0x400`, `0x10000`, `0x20000`, `0x40000`.
2. `ObjectClass+0x30` object-list link versus unrelated `CellClass+0x30` scratch.
3. `TechnoClass` owner, mission, target, and bridge/on-bridge fields.
4. `BuildingClass` foundation, exit-cell, power, factory, and placement fields.
5. `HouseClass+0x30` house index and fields that feed per-house bitmasks.
