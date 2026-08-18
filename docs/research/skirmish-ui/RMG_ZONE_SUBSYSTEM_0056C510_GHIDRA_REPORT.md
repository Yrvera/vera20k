# RMG Zone Subsystem — 0x0056C510 and Friends (Ghidra Report)

**Date:** 2026-07-20
**Scope:** The zone subsystem as consumed by the RMG starts phase (0x00594B50): the
zone flood fill, per-cell passability classification, the 13-kind passability matrix,
derived-zone lookup (`GetZoneID`), the bridge/tube crossing list, `CellClass+0x4C`,
and the MovementZone kind identity.
**Method:** Live Ghidra MCP against gamemd.exe. Every load-bearing claim cites the
verification call inline. Labels were treated as hints; all behavior read from bodies.

Context verified by the parent session (not re-derived here): 0x00594B50 calls
0x0056C510 with ECX=0x0087F7E8 (MapClass), takes the returned largest-component base
zone id, maps it through the derived table for kind `{5,5,5,0,0}[map_type]`, and seeds
start regions only on cells with `GetZoneID(cell, kind, 0) == reference_zone` and
`*(int*)(cell+0x4C) == 0`.

---

## Overview

Pipeline (all verified this session):

1. `MapClass__Resize` (0x00565C10) → `MapClass__InitZoneMap` (0x00567110) allocates the
   per-cell zone records and calls, in order: `MapClass__InitCellAttributes` (0x00568BB0)
   → `MapClass__ComputeBridgeZones` (0x0056D6E0) → the zone recompute 0x0056C510
   (verified via `decompile_function 0x00567110`, `get_function_callees 0x00567110`,
   `get_function_callers 0x00567110`).
2. `InitCellAttributes` runs `CellClass__RecalcAttributes` (0x0047D2B0) on every cell;
   `RecalcAttributes` calls `CellClass__RecalcZoneType` (0x00483C80) which writes the
   passability **class** into `CellClass+0x4C`, then copies class → zone-record byte0
   and cell Level → zone-record byte1 (verified via `decompile_function 0x0047D2B0`).
3. 0x0056C510 zeroes all zone ids, scanline-flood-fills base zones (ids from 1),
   collects base-zone adjacency edges (during the fill and from the crossing list),
   then builds one derived table per each of the 13 MovementZone kinds by BFS over the
   edges, keeping only components whose matrix value is 1
   (verified via `decompile_function 0x0056C510`).
4. `MapClass__GetZoneID` (0x0056D230) maps a cell → base zone → derived zone id for a
   kind (verified via `decompile_function 0x0056D230`).

Per-cell zone record (MapClass+0x68, count at +0x6C, 4 bytes/cell):
`byte0 = passability class (0..7)`, `byte1 = cell Level`, `bytes2..3 = u16 base zone id`.
Array is `S*S` entries with `S = MapClass+0xF4 + MapClass+0xF8 + 1`; linear index
`= y*S + x` clamped to `[0, count-1]` (verified via `decompile_function 0x00567110`
for the `S*S` allocation and `decompile_function 0x0056D3F0` =
`ZoneMap__CellToZoneIndex` for the index formula and clamping: negative → 0,
`>= count` → `count-1`).

---

## 1. MapClass__ZoneFloodFillScanLine — 0x0056CB90

Found via `get_function_callees 0x0056C510`; behavior verified via
`decompile_function 0x0056CB90` and `disassemble_function 0x0056CB90`.

Signature: `__thiscall (this=MapClass, record_ptr, zone_id, int* out_run_advance)`,
returns the number of cells filled. `out_run_advance` = cells from the seed to the
right end of the seed run + 1 (the caller in 0x0056C510 uses it to advance its scan).

**Connectivity: 4-directional scanline fill** in linear-index space:
- Horizontal runs along the row (index ±1).
- Recursion into the row above (index − S) and row below (index + S), where
  `S = [this+0xF4] + [this+0xF8] + 1` (verified at 0x0056CE04–0x0056CE12 in the
  disassembly: `MOV ECX,[EAX+0xf8]; MOV EDX,[EAX+0xf4]; LEA EAX,[ECX+EDX*1+0x1]`).
  No diagonal spread.

**Cell-compatibility condition** (what joins the same base zone):
- Class byte must be **exactly equal** to the seed's class byte (byte0 of the record).
  No class-group logic.
- Level condition (byte1 of the record), verified from assembly:
  - Leftward run extension: `|level(cell) − level(right neighbor)| ≤ 1`
    (0x0056CBCA: `CMP EAX,0x2; JGE exit`).
  - **Rightward run extension: `|level(cell) − level(left neighbor)| ≤ 3`**
    (0x0056CCE2: `CMP EAX,0x4; JGE exit`). This left/right asymmetry is real in the
    binary, not a decompiler artifact.
  - Vertical recursion (row above/below): recurse into an unzoned (`zone==0`) cell iff
    class equal AND `|Δlevel| ≤ 1` (0x0056CE9D and 0x0056CFEC: `CMP EAX,0x2; JGE skip`).

**Adjacency-edge registration during the fill** (this is how derived zones merge):
whenever a run end or a vertical neighbor holds a **nonzero, different** zone id, and
(`|Δlevel| ≤ 1` OR seed class == 6), an undirected edge `(neighbor_zone, current_zone)`
is appended to the 256-bucket hash table at `MapClass+0x14`
(bucket = `(idA & 0xF) << 4 | (idB & 0xF)`, bucket stride 0x18, entry = the packed pair
`idA<<16|idB` stored twice per 8-byte slot; duplicate pairs rejected by linear scan of
the bucket). Verified via `decompile_function 0x0056CB90` (all three edge-add sites)
and the seed-class-6 bypass at 0x0056CBA1/0x0056CBA8 (`CMP EDI,0x6; SETZ`).
Class 6 (Wheel-impassable ground, see §2) links to touching zones regardless of level.

**DAT_00ABDE8C**: a module-global "last neighbor zone registered" cache. 0x0056C510
sets it to 0 before each seed call (`decompile_function 0x0056C510`:
`DAT_00abde8c = 0;`); the fill skips the edge-add when the neighbor zone equals the
cached id and updates the cache after each registration attempt. It is purely a
redundant-work skip — the bucket linear-scan dedups anyway, so it does not change the
resulting edge set.

**Active in YR:** yes — runs on every map load/resize and on every bridge
destroy/repair (see the caller list from `get_function_callers 0x0056C510`).

---

## 2. Per-cell passability class writer — CellClass__RecalcZoneType (0x00483C80)

Chain verified: `InitCellAttributes` (0x00568BB0) → per-cell
`CellClass__RecalcAttributes` (0x0047D2B0) → `CellClass__RecalcZoneType` (0x00483C80)
writes `CellClass+0x4C`; `RecalcAttributes` then copies it into the zone record:
`record[0] = cell->field_0x4C; record[1] = cell->Level;` with
`record = *(MapClass+0x68) + ZoneMap__CellToZoneIndex(cell)*4` (all three sites in
`decompile_function 0x0047D2B0`; the RMG main 0x00598960 calls `InitCellAttributes`
directly before the starts phase, verified via `get_function_callees 0x00598960`).

Classification, in priority order (verified via `decompile_function 0x00483C80`):

| Priority | Condition | Class |
|---|---|---|
| 1 | `MapClass__Is_Cell_In_Playfield(coord, 1)` false | **7** |
| 2 | overlay present and `OverlayTypeClass+0x22D` (Crushable) | **1** |
| 3 | overlay present and `OverlayTypeClass+0x2A8` (Wall) | **2** |
| 4 | overlay present and Ground[overlay LandType].Wheel == 0.0 | **6** |
| 5 | overlay present and `OverlayTypeClass+0x2B5` (IsARock) | **6** |
| 6 | overlay present and `OverlayTypeClass+0x2B4` set | **0** (early out) |
| 7 | cell `LandType == 2` (Water) | **4** |
| 8 | cell `LandType == 6` (Beach) | **3** |
| 9 | Ground[cell LandType].Wheel ≤ 0.01 | **6** |
| 10 | occupier walk: a Building (RTTI 6) with the laser-fence / gate type-flag cases | **6** |
| 11 | occupier walk: a Terrain object (RTTI 0x24, trees) | **5** (or **2** in one scenario-gated case) |
| 12 | otherwise | **0** |

- The "Wheel" column identity is verified: the classifier indexes
  `DAT_0089EA48[LandType * 9]`; `RulesClass__ReadSpeedTypeLandTypeTable` (containing
  0x0067413B, the only writer per `get_xrefs_to 0x0089EA48`) fills the land table rows
  (stride 9 dwords, base row at 0x0089EA40) from INI keys in the order
  Foot(0x0089EA40), Track(+4), **Wheel(+8 = 0x0089EA48)**, Hover(+0xC),
  Winged forced 1.0 (+0x10), Float(+0x14), Amphibious(+0x18), one more speed key
  (+0x1C), Buildable byte (+0x20) — key strings verified via
  `read_memory 0x0081DBD4` ("Foot") and the string args in
  `decompile_function 0x0067413B`. So classes 6 vs 0 for plain terrain are driven by
  the rules INI `[land-type]` `Wheel=` percentage.
- LandType 2 = Water and 6 = Beach are cross-confirmed by the matrix (§3): kind 10
  "Water" passes exactly class 4; kind 11 "WaterBeach" passes exactly {3,4}.
- **Tile → LandType**: for non-overlay cells, `FUN_00544BE0` maps the TMP subtile
  terrain-type byte (subtile header +0x29) through the 16-entry table at 0x008288E4
  (verified via `decompile_function 0x00544BE0`, `read_memory 0x008288E4`):

  | terrain byte | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | A | B | C | D | E | F |
  |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
  | LandType | 0 Clear | 8 Ice | 8 | 8 | 8 | 10 Tunnel | 9 Railroad | 3 Rock | 3 Rock | 2 Water | 6 Beach | 1 Road | 1 Road | 0 Clear | 7 Rough | 3 Rock |

**Consequences for RMG-generated cells** (stock rulesmd `[Ground]` values):
clear / green LAT / rough / road → class 0; water → class 4; beach/shore pieces →
class 3; cliff tiles (terrain byte 7/8/F → LandType Rock, Wheel=0%) → class 6; the
out-of-playfield border ring → class 7. Trees placed later → class 5 on their cells.

**Active in YR:** yes — every cell, every attribute recalc.

---

## 3. g_PassabilityMatrix — 0x0082A594

Base address confirmed: the derived-table loop in 0x0056C510 starts its row pointer at
`&g_PassabilityMatrix` and terminates at 0x0082A734 after 13 strides of 8 dwords
(`decompile_function 0x0056C510`); xrefs at 0x0056C997/0x0056C9A2/… inside 0x0056C510
resolve to 0x0082A594 (`get_xrefs_to 0x0082A594`). 13*32 = 416 bytes read via
`read_memory 0x0082A594 length 416`:

| kind | MovementZone | c0 clear | c1 crush | c2 wall | c3 beach | c4 water | c5 tree | c6 wheel-0 | c7 OOB |
|---|---|---|---|---|---|---|---|---|---|
| 0 | Normal | **1** | 2 | 2 | 2 | 2 | 2 | 2 | 3 |
| 1 | Crusher | **1** | **1** | 2 | 2 | 2 | 2 | 2 | 3 |
| 2 | Destroyer | **1** | **1** | **1** | 2 | 2 | 2 | 2 | 3 |
| 3 | AmphibiousDestroyer | **1** | **1** | **1** | **1** | **1** | **1** | 2 | 3 |
| 4 | AmphibiousCrusher | **1** | **1** | 2 | **1** | **1** | 2 | 2 | 3 |
| 5 | Amphibious | **1** | 2 | 2 | **1** | **1** | 2 | 2 | 3 |
| 6 | Subterannean | **1** | **1** | **1** | 2 | 2 | 2 | **1** | 3 |
| 7 | Infantry | **1** | 2 | 2 | 2 | 2 | **1** | 2 | 3 |
| 8 | InfantryDestroyer | **1** | **1** | **1** | 2 | 2 | **1** | 2 | 3 |
| 9 | Fly | **1** | **1** | **1** | **1** | **1** | **1** | **1** | 3 |
| 10 | Water | 2 | 2 | 2 | 2 | **1** | 2 | 2 | 3 |
| 11 | WaterBeach | 2 | 2 | 2 | **1** | **1** | 2 | 2 | 3 |
| 12 | CrusherAll | **1** | **1** | **1** | 2 | 2 | 2 | 2 | 3 |

**Value semantics** (verified in `decompile_function 0x0056C510` and independently in
`decompile_function 0x0042C2A7` = Zone_precheck, which tests
`g_PassabilityMatrix[kind*8 + class] == 1`): **1 = passable for that kind; any other
value (2, 3) = impassable.** 3 is used only for class 7 (out of playfield); the 2/3
distinction has no other consumer in the derived-table builder — both fail the `== 1`
test identically there.

**RMG rows:**
- **Kind 5 (Amphibious, map types 0–2):** passable = {clear 0, beach 3, water 4}.
- **Kind 0 (Normal, map types 3–4):** passable = {clear 0} only.

---

## 4. MapClass__GetZoneID — 0x0056D230

Verified via `decompile_function 0x0056D230`. Signature:
`__thiscall (this=MapClass, CellStruct* coord, int kind, char check_bridge)`.

- With `check_bridge == 0` (**the RMG starts case**), it is a pure lookup:
  1. `idx = (this+0xF4 + this+0xF8 + 1) * coord.y + coord.x`, clamped: `< 0 → 0`,
     `>= this+0x6C → count-1` (same formula/clamp as `ZoneMap__CellToZoneIndex`).
  2. `base = *(u16*)(this+0x68 array + idx*4 + 2)`.
  3. return `*(u16*)( (this+0x18)[kind] + base*2 )` — the derived table for that kind.
- With `check_bridge != 0`: if the cell's `CellClass Flags (+0x140)` has bit 0x100
  (on-bridge), it resolves the bridge record via `MapClass__FindBridgeRecord` and, for
  destroyed bridges, walks perpendicular to find a ground endpoint, substituting that
  endpoint's coordinate before the lookup. Returns 0xFFFFFFFF if no record found.
  Irrelevant for RMG starts (arg is 0).

Derived-table entry values (from the builder in `decompile_function 0x0056C510`):
entry 0 = 0xFFFF; base components whose class fails the matrix keep the initial value
1; passable components get BFS group ids starting at 2 (BFS over the adjacency-edge
buckets, merging only components whose matrix value is also 1).

**Active in YR:** yes.

---

## 5. Crossing list at MapClass+0x54 — bridge/tube records

**What populates it:** only `MapClass__ComputeBridgeZones` (0x0056D6E0), which is
called only from `MapClass__InitZoneMap` (0x00567110), which is called only from
`MapClass__Resize` (0x00565C10) (verified via `decompile_function 0x0056D6E0`,
`get_function_callees 0x00567110`, `get_function_callers 0x00567110`). It clears the
vector (vtable+0xC on the vector object at +0x50) and then, iterating all cells:
- For every bridge-start tile (IsBridge/IsWoodBridge with the start subtile per the
  tables at 0x0082A734/0x0082A774), walks to the other end and appends a 16-byte
  record: `+0 endpoint A (x,y i16)`, `+4 endpoint B (x,y i16)`, `+8 u8 is_intact`,
  `+0xC i32 kind (0 = bridge, 1 = tube)`.
- For tube (TS tunnel) entrances, appends a kind-1 record.

**How it is consumed:** 0x0056C510 walks the records after the flood fill; for each
record with `is_intact != 0` it adds an adjacency edge between the base zones under
endpoint A and endpoint B into the same +0x14 buckets
(`decompile_function 0x0056C510`).

**RMG starts-time state — EMPTY, proven:**
- `ComputeBridgeZones` is unreachable except through `InitZoneMap` ← `Resize`;
  `Resize` necessarily runs when the map is allocated, before any RMG stamping, so the
  scan sees no bridge/tube tiles and appends nothing.
- Neither the RMG main (0x00598960) nor the starts phase (0x00594B50) calls
  `InitZoneMap` or `ComputeBridgeZones` afterwards (verified via
  `get_function_callees 0x00598960` and `get_function_callees 0x00594B50` — the RMG
  path re-runs only `InitCellAttributes` and 0x0056C510).
- Tubes are TS legacy; YR RMG never generates them.

**However — the "no merging" inference is FALSE.** An empty crossing list does NOT
mean derived zones equal matrix-filtered base components: the flood fill itself
registers adjacency edges between every pair of spatially touching base zones with
compatible levels (§1). On an RMG map, clear(0)↔beach(3) and beach(3)↔water(4) zones
that touch DO get edges, and under kind 5 (Amphibious) all three classes are
matrix-passable, so the BFS merges shoreline-connected land+beach+water into single
derived zones. The crossing list only adds *extra* edges (bridges/tubes); on an RMG
map it adds none.

---

## 6. CellClass+0x4C — the cell's zone passability class

**Identity: a 4-byte int holding the passability class 0..7 described in §2.** Not an
occupier pointer, not a terrain-object pointer.

Evidence:
- Writer: `CellClass__RecalcZoneType` writes `*(int*)(cell+0x4C) = 0..7` at every exit
  path (`decompile_function 0x00483C80`).
- Consumer 1: `CellClass__RecalcAttributes` copies it into zone-record byte0
  (`*record = this->field_0x4c`, `decompile_function 0x0047D2B0`) — i.e. it IS the
  class byte the flood fill and matrix consume.
- Consumer 2: the RMG start-region check `*(int*)(cell+0x4C) == 0` in
  0x00594B50/0x00594420 (parent-verified) therefore means "plain clear-class ground".
- The Ghidra `CellClass` struct has **no field defined at 0x4C** (gap between
  `SmudgeTypeIndex` at 0x48 and `Jumpjet` at 0xE0; `get_struct_layout CellClass`) —
  the YRpp-imported layout simply never named it; the binary usage above is
  authoritative.

**What makes it nonzero on a freshly generated RMG map:** water cells (4), beach/shore
cells (3), cliff/rock tiles and any Wheel=0% land (6), the out-of-playfield border (7),
tree cells (5), and overlay cells (1/2/6 by overlay flags). Start-region seeds are
restricted to class-0 cells.

---

## 7. Zone kinds — the 13 MovementZone values

Parser: `CCINIClass__ReadMovementZone` (containing 0x00474E50,
`decompile_function 0x00474E50`) reads the `MovementZone=` key (string "MovementZone"
at 0x008431C8, referenced from `TechnoTypeClass__ReadINI` at 0x00716065,
`get_xrefs_to 0x008431C8`) and linearly scans the name table at 0x0081BA88
(`g_MovementZone_NameTable`), 13 entries (loop bound 0x0081BABC), returning the match
index (−1 if none). Table contents resolved via `read_memory 0x0081BA60`,
`read_memory 0x0081BAB0`, `read_memory 0x0081BB60`, `read_memory 0x00817F64`,
`read_memory 0x008173D0`:

| index | name | index | name |
|---|---|---|---|
| 0 | Normal | 7 | Infantry |
| 1 | Crusher | 8 | InfantryDestroyer |
| 2 | Destroyer | 9 | Fly |
| 3 | AmphibiousDestroyer | 10 | Water |
| 4 | AmphibiousCrusher | 11 | WaterBeach |
| 5 | Amphibious | 12 | CrusherAll |

(Index 6 is "Subterannean" — binary spelling — TS legacy: the derived table for kind 6
is still *built* every recompute, but its consumers are the subterranean locomotor,
dead in YR per project policy.)

**RMG confirmation:** kind 5 = **Amphibious** (map types 0–2), kind 0 = **Normal**
(map types 3–4). The matrix rows (§3) are consistent with every name (e.g. Infantry
passes trees, Water passes only water, Fly passes everything in-playfield).

---

## Key globals / offsets

| Address / offset | Meaning | Verified via |
|---|---|---|
| 0x0056C510 | Zone recompute: fill base zones, build 13 derived tables; returns base id of largest component (strict `<`, first wins ties) | decompile_function 0x0056C510 |
| 0x0056CB90 | MapClass__ZoneFloodFillScanLine | decompile + disassemble 0x0056CB90 |
| 0x00483C80 | CellClass__RecalcZoneType (class writer) | decompile_function 0x00483C80 |
| 0x0047D2B0 | CellClass__RecalcAttributes (copies class+level into zone record) | decompile_function 0x0047D2B0 |
| 0x00568BB0 | MapClass__InitCellAttributes (per-cell recalc loop; called by RMG main) | decompile_function 0x00568BB0 |
| 0x0056D6E0 | MapClass__ComputeBridgeZones (sole +0x54 populator) | decompile_function 0x0056D6E0 |
| 0x00567110 | MapClass__InitZoneMap (allocator + orchestration) | decompile_function 0x00567110 |
| 0x0056D230 | MapClass__GetZoneID | decompile_function 0x0056D230 |
| 0x0056D3F0 | ZoneMap__CellToZoneIndex (y\*S+x, clamp) | decompile_function 0x0056D3F0 |
| 0x0082A594 | g_PassabilityMatrix, 13 rows × 8 dwords (1=passable) | read_memory 0x0082A594; get_xrefs_to 0x0082A594 |
| 0x008288E4 | TMP subtile terrain byte (hdr+0x29) → LandType, 16 entries | read_memory 0x008288E4; decompile_function 0x00544BE0 |
| 0x0089EA48 | Ground land table, Wheel column of row 0 (stride 9 dwords/land) | decompile_function 0x0067413B; get_xrefs_to 0x0089EA48 |
| 0x0081BA88 | g_MovementZone_NameTable, 13 char* | read_memory 0x0081BA60/0x0081BAB0; decompile_function 0x00474E50 |
| 0x00ABDE8C | Last-registered neighbor-zone cache for edge dedup skip | decompile 0x0056CB90 + 0x0056C510 |
| MapClass+0x14 | 256 hash buckets (stride 0x18) of zone adjacency edges | decompile_function 0x0056C510 |
| MapClass+0x18..+0x48 | 13 derived-table pointers (u16 per base zone) | decompile_function 0x0056C510 / 0x0056D230 |
| MapClass+0x4C | Base zone count (largest id + 1) | decompile_function 0x0056C510 |
| MapClass+0x54/+0x60 | Bridge/tube record array (stride 0x10) / count | decompile_function 0x0056D6E0 |
| MapClass+0x68/+0x6C | Per-cell zone records (class, level, u16 zone) / count = S² | decompile_function 0x00567110 |
| MapClass+0x70 | Per-cell 10-byte hierarchical-zone records (3×u16 level-zone ids, level byte at +8; built by ZoneMap__BuildZoneLevel — separate system) | decompile 0x00567110, 0x0047D2B0, 0x0042C2A7 |
| MapClass+0xF4/+0xF8 | Map dimensions; row stride S = f4+f8+1 | decompile 0x00567110/0x0056D230/0x0056D3F0 |
| CellClass+0x4C | int passability class 0..7 | §6 |
| CellClass+0x11B | Level byte (zone-record byte1 source) | get_struct_layout CellClass; decompile 0x0047D2B0 |

## Negative facts

- There is **no 8-directional or diagonal spread** in the base-zone fill.
- The fill's class comparison is **exact equality only** — no class grouping.
- `DAT_00ABDE8C` never changes the resulting edge set — dedup skip only.
- The crossing list (+0x54) is **not** written anywhere on the RMG path after map
  allocation; nothing besides `ComputeBridgeZones` appends to it.
- An empty crossing list does **not** disable derived-zone merging (fill-time
  adjacency edges exist regardless) — see §5.
- The 2-vs-3 distinction in the matrix is not consumed by the derived-table builder or
  Zone_precheck (both test `== 1`).
- `GetZoneID` with `check_bridge=0` (the RMG case) never touches bridge records.
- The Ghidra CellClass struct's field names are YRpp-imported and have no entry at
  +0x4C; do not trust the struct for this field.

## Implementation handoff — Rust RMG starts phase

To reproduce the reference-zone / seed-cell selection bit-identically:

1. **Classify every generated cell** into class 0..7 exactly per §2. For RMG output
   before object placement this reduces to: out-of-playfield border → 7; water land
   → 4; beach land → 3; land with `[Ground]` Wheel ≤ 0.01 (Rock = cliffs, walls) → 6;
   everything else (clear, green LAT, road, rough, ice, railroad per INI Wheel) → 0.
   Read the Wheel percentages from the in-repo INI, not hardcoded.
2. **Base zones:** 4-dir scanline flood fill over the S×S linear array
   (S = width+height+1 in gamemd terms — map to the Rust map geometry equivalent),
   sequential ids from 1, skipping class-7 cells; join condition = equal class AND the
   level rule (left ≤1, right ≤3, vertical ≤1). On a flat-per-zone RMG map the level
   asymmetry is unlikely to matter, but implement it exactly — cliff-adjacent
   staircases can hit it.
3. **Adjacency edges:** collect the undirected pairs exactly as the fill does
   (different nonzero neighbor zone, `|Δlevel| ≤ 1` or seed class 6). No bridge/tube
   edges on RMG maps (§5).
4. **Derived zone table for the kind:** kind = Amphibious(5) for map types 0–2,
   Normal(0) for 3–4. Component passable iff matrix[kind][class] == 1 (§3 table —
   safe to hardcode; it is compile-time data in gamemd, not INI). BFS-merge passable
   components over the edges; ids from 2; non-passable keep 1; entry 0 = 0xFFFF.
   **BFS order matters for id assignment**: gamemd seeds the BFS by ascending base
   zone id, and the reference comparison is by id equality only, so any order that
   groups identically is output-equivalent — but if you ever compare derived ids
   against gamemd traces, replicate the ascending-seed order.
5. **Reference zone:** largest base component by fill-returned cell count (strict `>`
   to replace, so first-encountered wins ties = lowest id), mapped through the derived
   table.
6. **Seed filter:** cell class == 0 AND derived_table[base_zone(cell)] ==
   reference_zone. This is what `*(int*)(cell+0x4C)==0` + `GetZoneID(...)==ref` means.
7. On Amphibious map types the reference zone spans water+beach+clear connected
   through shorelines — a start region on one island can share the reference zone with
   another island connected by water. Do not "fix" this; it is gamemd behavior.

## Remaining uncertainty

- The exact identity of the `OverlayTypeClass+0x2B4` flag (priority 6 early-out to
  class 0) was not traced to its INI key this session; it does not fire for RMG
  terrain (no overlays at starts time). UNVERIFIED beyond the decompile.
- The Building/Terrain occupier sub-cases (priorities 10–11, including the
  `g_ScenarioClass+0x1258 == 1` gate on the Terrain→class-2 path) were read from the
  decompile but their type-flag INI names were not chased; irrelevant pre-placement.
- The 9th `[Ground]` speed column key (`g_SpeedTypeStringConstants` arg at
  `pfVar4[6]`) was not resolved to its string; the classifier only reads the Wheel
  column.
- Whether any RMG phase between land-blob generation and the starts phase can stamp
  bridge tiles was concluded from call-graph reachability (no path recomputes the
  crossing list), not from auditing every FUN_ in 0x00598960's callee list.
