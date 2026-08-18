# SmudgeClass & SmudgeTypeClass -- Ghidra Report

Confidence: HIGH for struct layouts, INI key-to-offset mappings, and spawn-path
function identities (all verified from disassembly + CCINIClass::ReadX calls).
MEDIUM for the "which type list feeds which damage tier" (`Scorches`,
`Scorches1..4`, `Craters`) -- those rules-side parsers were not decompiled this
iteration.

## Overview

Smudges are **purely cosmetic** ground decals: craters from explosions, scorch
marks from incendiary damage, and pre-placed decals loaded from maps. They
touch cell state in exactly one place -- `CellClass+0x48`
(`SmudgeTypeIndex`) -- but never change gameplay, pathing, or LOS. They are
OBJECTCLASS but not TECHNOCLASS (no health, owner, AI).

This report covers the type class, the instance class, the allocators, the
cell-validity check, and the pre-placed loader. Rendering pass
(`Tactical_layer_smudges @ 0x006D3290`) is referenced but not fully decoded --
it shares the generic `Cell_ContentRendering` path.

---

## 1. INI Fields and Rules Offsets

### Per-type section (e.g. `[CRATER01]`, `[BURN01]`)

Read by `SmudgeTypeClass::ReadINI @ 0x006B56D0` (no explicit symbol; function
is untagged in Ghidra but starts at that address after padding).

| INI Key  | Type Offset | Type   | Default | Notes |
|----------|-------------|--------|---------|-------|
| `Crater` | +0x2A0      | bool   | 0       | Type is a crater (explosion leftover) |
| `Burn`   | +0x2A1      | bool   | 0       | Type is a scorch/burn mark |
| `Width`  | +0x298      | int    | 1       | Footprint in **cells**, not pixels |
| `Height` | +0x29C      | int    | 1       | Footprint in cells |

Plus inherited `ObjectTypeClass` keys (e.g. `Image=`) and the Name field at
+0x24.

After the keys are read, the function loads the `.shp` at
`SmudgeType+0xA4`. The filename format depends on byte `+0x22c`:

- +0x22c == 0: fixed filename from string table (DAT_0081834c).
- +0x22c != 0: theater-dependent filename built via a
  format-string lookup at `0x007e1bc6 + theater_index * 0x70`.

### `[Smudge]` section (map file)

Read by `SmudgeClass::ReadINI @ 0x006B4C80`. Each entry has format
`Key=TYPENAME,CellX,CellY,IsBaked`. The constructor is called with cell
coordinates converted via `X * 0x100 + 0x80`, `Y * 0x100 + 0x80` (cell centre
in leptons).

### `[SmudgeTypes]` section (rules)

Numeric-keyed list (`1=CR1`, `2=CR2`, ...). Used to populate the global
SmudgeTypeClass array via `SmudgeTypeClass::FindOrAllocate @ 0x006B5910`.

### `[General]` keys that categorise smudge types by damage tier

These keys exist in `ini/rulesmd.ini [General]` and feed the spawn path, but
their rules-offset mapping was not verified this iteration:

- `Scorches=`      -- default burn list
- `Scorches1..4=`  -- burn lists per escalating damage tier
- `Craters=`       -- default crater list
- `ForceBigCraters=` (bool) -- force 2x2 smudge pick regardless of damage

---

## 2. SmudgeTypeClass -- Struct Layout

`sizeof(SmudgeTypeClass) = 0x2A4` (676 bytes), confirmed by the
`operator_new(0x2A4)` in `FindOrAllocate`.

| Offset | Size | Type            | Field           | Source                                  |
|--------|------|-----------------|-----------------|-----------------------------------------|
| +0x00  | 4    | vtable*         | vtable          | Constructor writes `&vtable__SmudgeTypeClass` |
| +0x04..+0x0C | 12 | vtable*         | secondary vtables | Constructor writes three secondary vtables |
| +0x24  | -    | char*           | Name            | Inherited from AbstractType |
| +0xA4  | 4    | ShapeFileClass* | SHP_Ptr         | `ReadINI` @ 0x006b57c7 |
| +0x1F8 | 128  | char[128]       | Filename buffer | `ReadINI` @ 0x006b57b4 (sprintf target) |
| +0x22C | 1    | bool            | IsTheater?      | `ReadINI` reads at 0x006b5764 to choose filename mode |
| +0x22F | 1    | byte            | (unknown flag)  | Constructor sets to 1 |
| +0x230 | 1    | byte            | (unknown flag)  | Constructor clears |
| +0x231 | 1    | byte            | (unknown flag)  | Constructor clears |
| +0x232 | 1    | byte            | (unknown flag)  | Constructor sets to 1 |
| +0x233 | 1    | byte            | (unknown flag)  | Constructor sets to 1 |
| +0x235 | 1    | byte            | (unknown flag)  | Constructor clears |
| +0x294 | 4    | int             | ArrayIndex      | Constructor sets to vector-slot index or -1 |
| +0x298 | 4    | int             | **Width**       | `ReadINI` (Width= key) |
| +0x29C | 4    | int             | **Height**      | `ReadINI` (Height= key) |
| +0x2A0 | 1    | bool            | **Crater**      | `ReadINI` (Crater= key) |
| +0x2A1 | 1    | bool            | **Burn**        | `ReadINI` (Burn= key) |

The "unknown flags" at +0x22F..+0x235 are set to fixed defaults by the
constructor and were not traced to consumers this iteration.

### Global array

- **Base:** `DAT_00A8EC1C` (`SmudgeTypeClass**`)
- **Count:** `DAT_00A8EC28` (`int`)
- **Capacity-ish state:** `DAT_00A8EC20`, `DAT_00A8EC25`, `DAT_00A8EC2C` --
  standard `DynamicVectorClass` control fields.

---

## 3. SmudgeClass -- Struct Layout

`sizeof(SmudgeClass) = 0xB0` (176 bytes), confirmed by the
`operator_new(0xB0)` in each spawn path.

| Offset | Size | Type             | Field          | Notes                              |
|--------|------|------------------|----------------|------------------------------------|
| +0x00  | 4    | vtable*          | vtable         | `&vtable__SmudgeClass` |
| +0x04..+0x0C | 12 | vtable*          | secondary vtables | 3 secondary vtables |
| +0x14  | -    | (from AbstractClass) | UniqueID    | via `AbstractClass::AssignUniqueID(this+1)` |
| +0xAC  | 4    | SmudgeTypeClass* | Type           | `param_1[0x2b] = param_2` in constructor |

SmudgeClass inherits from ObjectClass (not TechnoClass). No Owner, no Health,
no Mission, no AI. The base ObjectClass fields occupy +0x00..+0xAC and are
covered by `OBJECTCLASS_GHIDRA_REPORT.md`.

### Global array

- **Base:** `DAT_00A8B1E4` (`SmudgeClass**`)
- **Count:** `DAT_00A8B1F0` (`int`)

### Constructor flow (`SmudgeClass::Constructor @ 0x006B4A50`)

1. Call `ObjectClass::Constructor()`.
2. Install vtables (4 slots).
3. `AbstractClass::AssignUniqueID(this+1)`.
4. Register in global smudge vector (DynamicVector grow-if-needed, else drop).
5. If `param_3` (coord) differs from the sentinel at `DAT_00B0B728` (a
   global "zero coord"), call `ObjectClass::Reveal(coord, 0)`. On failure
   calls `ObjectClass::UnInit()` (smudge is rejected and destroyed).
6. `ObjectClass::Reveal` is the function that writes the smudge's
   TypeIndex into the target cell(s) at `CellClass+0x48` -- the only place
   cell state is modified.

---

## 4. Spawn Paths (Verified)

There are **three** ways a SmudgeClass is constructed at runtime, plus one
map-load path.

### 4.1 Crater spawner -- `Debris_Smoke @ 0x006B5C90`

**Ghidra name is misleading** -- this function places a **crater**
smudge, not smoke. It filters SmudgeTypes by `Crater=yes` (byte +0x2A0).

Signature (reconstructed):
```
uint SmudgeClass::CreateCrater(CoordStruct *coord, int dmg, int dmg2, char forceBig);
```

Flow:
1. Check `DAT_00B0B788/8A`: if this cell is the same as the last smudge
   coord, bail (dedup -- prevents crater-on-crater at identical cell).
2. Iterate all SmudgeTypes. For each type with `Crater == 1`, call
   `SmudgeTypeClass::CanPlaceHere` (see §5). Passing types are collected into
   `local_14` array (first-pass list).
3. Filter by size:
   - If `forceBig == 0` (small hit): pass types with `Width==1 && Height==1`,
     OR any size when both `dmg > 0x3C` (60) and `dmg2 > 0x32` (50). Collected
     into `local_2C` array (preferred list).
   - If `forceBig != 0`: pass only types with `Width>=2 && Height>=2`.
4. Allocate `operator_new(0xB0)`; random-pick from preferred list, fallback to
   first-pass list. Construct SmudgeClass with `coord` and sentinel ID
   0xFFFFFFFF.

### 4.2 Scorch spawner -- `SpawnDebris @ 0x006B59A0`

**Also misleadingly named** -- this places a **burn/scorch** smudge. Identical
structure to §4.1 but filters on byte +0x2A1 (`Burn`) instead of +0x2A0
(`Crater`). All the size-selection thresholds are the same (60/50 cutoffs for
large smudges).

### 4.3 Direct constructor wrapper -- `FUN_006B55C0 @ 0x006B55C0`

Simple wrapper:
```c
pv = operator_new(0xB0);
if (pv) SmudgeClass::Constructor(type, &DAT_00B0B7A8, -1);
return pv;
```
Places a smudge of a specific passed-in type at the coord stored in the
global `DAT_00B0B7A8`. Likely called from a warhead path that has already
picked the type (e.g. a fixed explosive fingerprint) or from a debug/cheat
entry point.

### 4.4 Map-load -- `SmudgeClass::ReadINI @ 0x006B4C80`

For each entry in the map's `[Smudge]` section:
1. Read "TYPENAME,CellX,CellY,IsBaked" via two levels of `CRT::strtok`.
2. Look up TYPENAME in the SmudgeType array (`FUN_006B5440`); skip if -1.
3. Build coord: `{X * 0x100 + 0x80, Y * 0x100 + 0x80, 0}` (cell centre in
   leptons, Z = 0 = ground).
4. Allocate `operator_new(0xB0)`; construct with sentinel ID 0xFFFFFFFF.

Note: the "IsBaked" fourth parameter is parsed but only checked for zero vs.
non-zero; the constructor call doesn't forward it. It may affect whether the
smudge is part of the saved game state separately.

---

## 5. Cell Validity Check -- `SmudgeTypeClass::CanPlaceHere @ 0x006B5F80`

```
char SmudgeTypeClass::CanPlaceHere(this, CellCoord *cellXY, bool allowBuilding);
```

Iterates the type's `Width × Height` rectangle of cells starting at `cellXY`.
For each cell, **all** of these must hold, else returns 0:

- Cell must be in-bounds (`Cell_in_bounds_check`).
- `CellClass+0x11C == 0` (flag; likely "IsImpassable" / "IsWater" -- not
  pinpointed this iteration).
- `CellClass+0x48 == -1` (no existing SmudgeTypeIndex -- no smudge already).
- `CellClass+0x44 == -1` (no OverlayTypeIndex -- no overlay already, so
  smudges can't overwrite walls, gems, tiberium, fences).
- If `allowBuilding == 0`: `Look_up_building_in_cell(cell) == NULL`.
- Cell's LandType entry (indexed by `CellClass+0x38` into
  `DAT_00A8ED2C[]`) must have its +0x2E0 "accepts-smudge" flag set.

If all cells pass, returns 1 (`CONCAT31(..., 1)`).

This is the consolidated placement gate. No `Crater`/`Burn` matching happens
here -- that's the caller's job.

---

## 6. Dedup Against Repeat Hits

The global pair `DAT_00B0B788` (short: cell.X) and `DAT_00B0B78A` (short:
cell.Y) stores the last cell where a smudge was placed. The spawn paths
(§4.1 and §4.2) bail early if the incoming coord matches the stored pair.

This is why, in practice, **you never see two craters stacked on the exact
same cell from rapid-fire damage** -- the first one locks out the cell from
the spawn path until a different cell is written to the globals. Save/load
serialisation of these globals was not investigated.

---

## 7. Rendering -- Pointer Only

`Tactical_layer_smudges @ 0x006D3290` is the function named by Ghidra for the
smudge render layer. It mostly delegates to `Cell_ContentRendering` (the
general cell-content draw). Smudges are drawn through that generic path --
they don't have a dedicated `SmudgeClass::Draw`. The field at
`CellClass+0x48` is how the renderer knows a cell has a smudge and which
SHP frame to use.

A full rendering trace (frame selection, colour remap, z-depth) is deferred
to a future iteration.

---

## 8. TS-Legacy Notes

Smudge is shared TS/YR code; it is **live in YR** (smudges appear on craters
and fires in every skirmish). No `SpecialFlags` gate, no dormant-in-YR
behaviour. The misleading Ghidra names (`Debris_Smoke` for
the crater path, `SpawnDebris` for the scorch path) are one of the common
traps mentioned in CLAUDE.md's Ghidra-annotation rules -- do NOT trust those
labels without reading the body.

`ForceBigCraters` (`[General]`) is a standard YR-active switch, not a TS
ghost.

---

## 9. What an Implementation Must Match (Summary)

1. Parse `[SmudgeTypes]` (numeric-keyed list) and allocate one
   SmudgeTypeClass per name. Reserve index 0 implicitly via the array's
   DynamicVector layout (constructor sets `ArrayIndex = vector-slot`).
2. Per-type: read `Crater` (bool), `Burn` (bool), `Width` (int, default 1),
   `Height` (int, default 1). Store them at the offsets above.
3. On warhead impact with ground: call the crater spawner first if the
   warhead is "explosive" (reads `Craters` list); then the scorch spawner if
   the warhead is "incendiary" (reads `Scorches` list). Bail if the cell is
   the same as the last spawn dedup point.
4. Enforce `CanPlaceHere` before construction: all `Width×Height` cells
   in-bounds, cell's OverlayIndex == -1, cell's SmudgeIndex == -1, no
   building, LandType allows smudges.
5. Map-load pre-placed smudges: coord is `(X*256+128, Y*256+128, 0)`, sentinel
   ID 0xFFFFFFFF.
6. On construction, write `SmudgeTypeIndex` into `CellClass+0x48` (via the
   ObjectClass::Reveal path -- same machinery other ObjectClass derivatives
   use to register into cells).
7. Rendering: Smudges are drawn as part of the generic cell-content pass;
   the renderer reads `Cell+0x48` and blits the type's SHP. No per-frame
   animation, no facing, no lighting variation.

Gameplay touches: **none**. A smudge cannot be targeted, cannot block
movement, cannot be destroyed (except by being overwritten by a later smudge
once the dedup globals tick off-cell, or by the cell being rebuilt by
overlay/building placement, which the `CanPlaceHere` check prevents
anyway). Deterministic lockstep should still serialise SmudgeTypeIndex per
cell and the dedup globals, because visual divergence between replays is
jarring even if not simulation-affecting.

---

## 10. Gaps for a Future Iteration

- **Rules-side parser offsets** for `Scorches`, `Scorches1..4`, `Craters`,
  `ForceBigCraters`. Not traced this iteration; the caller that invokes
  `Debris_Smoke` / `SpawnDebris` picks a *type list* by damage tier and
  passes it in -- that caller is the one reading these rules fields.
- **CellClass+0x11C flag name** (the "IsImpassable/IsWater" tested in
  CanPlaceHere at +0x11C).
- **LandType +0x2E0 flag name** (the "accepts-smudge" gate).
- **SmudgeType +0x22F..+0x235** (the six fixed-default bytes -- not yet
  mapped to INI keys; may be theater/variant switches).
- **`Tactical_layer_smudges`** full render trace (SHP frame selection, remap
  palette, z-write behaviour).
