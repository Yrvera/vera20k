# MapClass Remainders — Ghidra Research Report (2026-04-24)

Closes three MapClass follow-ups from the revisit report:

1. `UpdateBridgeZonesHelper` full internals + `g_PassabilityMatrix` semantics
2. The 16-function `UpdateRamp_*` family (all variants tabulated)
3. The bridge-repair-hut registry at `DAT_008B41A8`

**Confidence:** HIGH — every finding comes from freshly-decompiled
code listed in Sources. Spot-checked passability matrix contents
against raw memory dump.

**Active in YR:** Yes — all three systems run in every standard
skirmish.

---

## 1. UpdateBridgeZonesHelper + ZoneFloodFillScanLine internals

### Entry point (`0x56C510`, 320+ lines)

Full zone rebuild, called when:
- A bridge is destroyed / repaired
- A building is placed / removed in a way that changes passability
- Scenario load, after cell attributes are computed
- Incremental zone updates fall back (when conflict count ≥ 4)

Phases:

**Phase 1 — Teardown:**
```
1. For each of 256 hash buckets at this+0x14: call vtable[3] (Clear)
2. For i in 0..13: free zone_ids[i] at this+0x18+i*4, set to NULL
3. Zero the cluster_id (bytes 2–3) of every zone_cell_data entry
```

**Phase 2 — Sentinel zone and flood-fill:**
```
4. Allocate and insert sentinel type 7 as "cluster 0, zone 7"
5. uVar21 = 1 (next zone id)
   pbVar5 = zone_cell_data begin
   pbVar1 = zone_cell_data end (= this+0x68 + this+0x6C * 4)
   while pbVar5 < pbVar1:
       if zone_type == 7 OR cluster_id != 0:  # already assigned or impassable
           pbVar5 += 4
           continue
       MapClass::ZoneFloodFillScanLine(pbVar5, uVar21, &scanline_out_len)
       record the scanline's (cluster_id, cell_count) in a growing list
       uVar21 += 1
       pbVar5 += scanline_out_len * 4
6. Store total cluster count: *(this + 0x4C) = uVar21 & 0xFFFF
```

**Phase 3 — Bridge record adjacency:**
```
7. For each of this+0x60 bridge records (each 16 bytes):
     if record.is_intact != 0:
         stride = this+0xF4 + this+0xF8 + 1
         idxA = record.endpoint_a.Y * stride + record.endpoint_a.X  (clamped)
         idxB = record.endpoint_b.Y * stride + record.endpoint_b.X  (clamped)
         clusterA = zone_cell_data[idxA].cluster_id
         clusterB = zone_cell_data[idxB].cluster_id
         if clusterA != clusterB:
             record adjacency edge (clusterA, clusterB) in hash table
               hash = (min(A,B) & 0xF) << 4 | (max(A,B) & 0xF)
```

**Phase 4 — Per-cluster degree count:**
```
8. Allocate `cluster_degrees[cluster_count]` (ushort[])
9. For each bucket in hash table:
     for each (A, B) adjacency edge in bucket:
         cluster_degrees[A]++
         cluster_degrees[B]++
```

**Phase 5 — Per-cluster edge lists:**
```
10. Allocate cluster_edges[] (int*[cluster_count]) — per-cluster edge-target arrays
11. Zero cluster_degrees[]
12. Iterate hash table again; for each (A, B):
      cluster_edges[A][cluster_degrees[A]] = B (as short)
      cluster_edges[B][cluster_degrees[B]] = A
      cluster_degrees[A]++; cluster_degrees[B]++
```

**Phase 6 — Extract cluster zone_types:**
```
13. Allocate cluster_type[cluster_count] (u8)
14. Walk zone_cell_data, for each unique cluster copy cluster.zone_type
```

**Phase 7 — Build zone_ids arrays for all 13 MovementZones:**
```
15. Allocate scratch visit[cluster_count] (ushort)
16. For each MZ in 0..13 (via pointer walk into g_PassabilityMatrix):
      zone_ids[MZ] = allocate ushort[cluster_count]
      next_zone = 2  (zones 0, 1 reserved for impassable/start)
      For each unvisited passable cluster c:
          zone_ids[MZ][c] = next_zone
          BFS via cluster_edges[]: for each reachable cluster r with
                                   g_PassabilityMatrix[MZ][cluster.type] == 1:
              zone_ids[MZ][r] = next_zone
              visited[r] = next_zone
          next_zone++
      zone_ids[MZ][0] = 0xFFFF  (sentinel terminator)
```

**Phase 8 — Cleanup:**
```
17. Free all temporary allocations (cluster_edges rows, cluster_degrees,
    cluster_type, visit, scratch result list).
```

Returns the total cluster count (or max cluster id found).

### `ZoneFloodFillScanLine` (`0x56CB90`) — classic scanline flood fill

Inputs: seed cell in zone_cell_data, new zone_id (u16), out_scanline_length.

Algorithm:
```
1. seed_type = *seed;  seed_is_water = (seed_type == 6)
2. Walk LEFT along the row:
     stop when zone_type != seed_type OR |height_delta| ≥ 2
     each step: assign cluster_id = new_zone_id
3. At left edge, record adjacency with the blocking cell:
     if blocking.cluster_id != 0 AND (|height_delta| < 2 OR seed_is_water)
     AND blocking.cluster_id != DAT_00abde8c  [last-seen edge, de-dup cache]
     AND blocking.cluster_id != new_zone_id:
         push edge (blocking.cluster_id, new_zone_id) into hash bucket
           bucket = hash_table[(blocking & 0xF) << 4 | (new & 0xF)]
         duplicate-check existing bucket entries first; only push if new
     DAT_00abde8c = blocking.cluster_id
4. Walk RIGHT from seed the same way:
     stop when zone_type != seed_type OR |height_delta| ≥ 4  (note 4, not 2)
     each step: assign cluster_id = new_zone_id
5. Record adjacency with right-edge blocking cell (same logic)
6. out_scanline_length = cells assigned in this scanline
7. Recurse UP and DOWN:
     stride = map.size_width + 1 + map.size_height
     For each cell in the row ABOVE (scanning over [leftmost, rightmost]):
         if that cell.cluster_id == 0 AND cell.zone_type == seed_type
            AND |height_delta| < 2 (diff from the cell directly below it):
             recursively flood-fill from that cell
         else if already assigned AND cluster_id != new_zone_id:
             record adjacency
     Same for the row BELOW.
```

**Notable constants / quirks:**
- Height delta threshold is **1** (so diff ≤ 1) for same-zone continuation
  in the left walk, but **3** (diff ≤ 3) in the right walk. Likely a
  deliberate asymmetry to handle stair-stepped terrain from the
  leftmost seed direction. Could also be a bug preserved for decades.
- `seed_type == 6` flag relaxes boundary checks: when recording
  adjacency at a height-delta boundary that would otherwise be
  skipped, water seeds still record the edge.
- `DAT_00abde8c` is a single-entry cache of "last-recorded edge
  cluster_id" — avoids hash lookup when consecutive cells share the
  same boundary.
- Hash bucket is 256 entries, hashed by xor of low nibbles. Each
  bucket is a DynVec of 8-byte entries (`uint32` packed + 4 reserved
  bytes). The "reserved" 4 bytes are written identical to the packed
  value — possibly a debug duplicate or just unused overhead.

### Passability matrix — `g_PassabilityMatrix` at `0x0082A594`

Layout: 13 rows (one per MovementZone) × 8 columns (one per zone_type)
× 4 bytes (int32) = 416 bytes total, ending at `0x0082A734`.

Raw values (decoded from memory dump):

| Row | MZ (guessed) | types 0..7 |
|-----|-------|------|
| 0 | MovementZone 0 | `1, 2, 2, 2, 2, 2, 2, 3` |
| 1 | MovementZone 1 | `1, 1, 2, 2, 2, 2, 2, 3` |
| 2 | MovementZone 2 | `1, 1, 1, 2, 2, 2, 2, 3` |
| 3 | MovementZone 3 | `1, 1, 1, 1, 1, 1, 2, 3` |
| 4 | MovementZone 4 | `1, 1, 2, 1, 1, 2, 2, 3` |
| 5 | MovementZone 5 | `1, 2, 2, 1, 1, 2, 2, 3` |
| 6 | MovementZone 6 | `1, 1, 1, 2, 2, 2, 1, 3` |
| 7 | MovementZone 7 | `1, 2, 2, 2, 2, 1, 2, 3` |
| 8 | MovementZone 8 | `1, 1, 1, 2, 2, 1, 2, 3` |
| 9 | MovementZone 9 | `1, 1, 1, 1, 1, 1, 1, 3` |
| 10 | MovementZone 10 | `2, 2, 2, 2, 1, 2, 2, 3` |
| 11 | MovementZone 11 | `2, 2, 2, 1, 1, 2, 2, 3` |
| 12 | MovementZone 12 | `1, 1, 1, 2, 2, 2, 2, 3` |

**Value semantics** (from `enum PassabilityType` and its use in the
flood-fill loop):

- `1` = passable — the flood-fill may enter cells of this zone_type
- `2` = impassable — flood-fill skips; adjacency is recorded but no
  zone spans across
- `3` = sentinel — always column 7 (type 7 = the "impassable
  sentinel" type assigned at construction). Column 7 is always 3.

The flood-fill construction loop writes:
```
ushort is_blocked = (matrix[MZ][cluster.zone_type] != 1) ? 1 : 0;
```

Then BFS skips clusters with `is_blocked == 1`. So a cluster (a
connected region of same-type cells) gets a zone id in this MZ only
if its type is PASSABLE (matrix value = 1) for this MZ.

**Observations:**
- Row 9 passes types 0–6 → most permissive ground zone, likely *All*
  ground infantry or amphibious foot variant.
- Row 3 passes 0–5 → high-mobility wheeled/tracked.
- Row 10, 11 start with `2` at type 0 → cannot walk on "standard"
  land type → naval.
- Row 6, 7, 8 have unique patterns → specialty zones (amphibious,
  hover?).
- All rows terminate with `3` at column 7 → consistent with type 7
  being the map-edge / impassable sentinel.

**Mapping to named MovementZones** needs a Rust-side cross-reference
against `rules::locomotor_type::MovementZone` — row ordering
probably matches the enum values.

### Zone graph edge count — integrity check

The gamemd zone system maintains TWO parallel edge representations:
1. The **hash table at MapClass+0x14** (adjacency as a set, deduped).
2. The per-MZ **cluster_edges[]** lists built inside
   `UpdateBridgeZonesHelper` (scratch data, freed at end).

The scratch cluster_edges table has a neat property: after build,
`cluster_degrees[c]` equals the number of clusters adjacent to `c`,
and `cluster_edges[c]` holds the exact list. The outer BFS in phase
7 uses this to compute zone connectivity per MZ.

---

## 2. UpdateRamp_\* — all 16 variants tabulated

All 16 functions share the same structural template. The variance is
in:
- **Orientation** (NS vs EW) — controls which neighbor cell we update
- **Height** (Low vs High) — switches tile base constant
- **Variant** (DamageA, DamageB, CollapseA, CollapseB) — state
  transition rules + tile offset constants

### Template (pseudocode — all 16 follow this shape)

```
void UpdateRamp_<ORIENT>_<VARIANT>_<HEIGHT>(short *self_coord, uint direction):
    dir_mod8 = direction & 7
    neighbor_coord = (self_coord.x + dir_offsets.x[dir_mod8],
                      self_coord.y + dir_offsets.y[dir_mod8])
    neighbor_cell = lookup_cell(neighbor_coord)

    if neighbor_cell.flags[0x140] & 0x80:          # is-ramp
        apply variant-specific damage step transition on +0x11E

    tile_offset = neighbor_cell.IsoTileTypeIndex - BASE_TILE + 1

    if tile_offset == PAVEMENT_CHECK_A or PAVEMENT_CHECK_B:
        ToggleBridgePavement(neighbor_coord, 1, 0)
        return

    match tile_offset:
        case OFFSET_1:
            SetOverlayAndPropagate(neighbor_coord, BASE_TILE + DELTA_1, -1, -1, 0)
        case OFFSET_2:
            SetOverlayAndPropagate(neighbor_coord, BASE_TILE + DELTA_2, -1, -1, 0)
        case OFFSET_3:  # CollapseA/B only
            UpdateRamp_<...>(recurse)
            if not (neighbor_cell.flags[0x11A] & 1):   # NS-oriented bridge
                BlowUpBridge(cells in column-Y range)
            else:                                       # EW-oriented
                BlowUpBridge(cells in column-Y-1 range)
            SetOverlayAndPropagate(neighbor_coord, final_tile, -1, level-4, 0)
```

> **Axis-label note.** State byte 0..8 is **physically EW** and state byte
> 9..0x11 is **physically NS** (verified 2026-05-13 by SHP frame visual
> inspection — `bridge.tem` frame 0 is a NW→SE screen-space diagonal,
> frame 9 is NE→SW). gamemd's `UpdateRamp_NS_*_Low` family operates on the
> 0..8 range despite the "NS" in the name (mis-named in the binary), and
> `UpdateRamp_EW_*_Low` operates on the 9..0x11 range. The headings below
> use the **physical** axis labels.

### Damage-step state transitions — EW orientation (state byte 0..8)

Implemented by the binary's `UpdateRamp_NS_*_Low` functions (despite the
mis-named "NS" suffix). State range used at e.g. `UpdateRamp_NS_DamageA_Low
@ 0x56ED40`: `if (state < 4) state = 4; else if (state == 5) state = 6;`.

| Variant | From | To |
|---------|------|----|
| DamageA | 0,1,2,3 | **4** |
| DamageA | 5 | **6** |
| DamageB | 0,1,2,3 | **5** |
| DamageB | 4 | **6** |
| CollapseA | <7 | **7** |
| CollapseA | 8 | **0** (+ full collapse sequence) |
| CollapseB | <7 | **8** |
| CollapseB | 7 | **0** (+ full collapse sequence) |

EW pavement-check constants (for the binary's `_NS_*` family): `DAT_00ABC1E8`
and `DAT_00AA0E38` (the values listed in the Base-tile constant table below).

### Damage-step state transitions — NS orientation (state byte 9..0x11)

Implemented by the binary's `UpdateRamp_EW_*_Low` functions (despite the
mis-named "EW" suffix). State range used at e.g. `UpdateRamp_EW_DamageA_Low
@ 0x56F690`: `if (state > 8 && state < 0xD) state = 0xE; else if (state ==
0xD) state = 0xF;`.

| Variant | From | To |
|---------|------|----|
| DamageA | 9, 0xA, 0xB, 0xC | **0xE** |
| DamageA | 0xD | **0xF** |
| DamageB | 9, 0xA, 0xB, 0xC | **0xD** |
| DamageB | 0xE | **0xF** |
| CollapseA | <0xF | **0x10** |
| CollapseA | 0x11 | **9** (+ full collapse sequence) |
| CollapseB | <0xF | **0x11** |
| CollapseB | 0x10 | **9** (+ full collapse sequence) |

NS pavement-check constants (for the binary's `_EW_*` family): `DAT_00ABC1D0`
and `DAT_00AA1540` (NOT the EW constants in the Base-tile table below).

So the damage step is a 4-state lattice (illustrated for NS; EW is offset by +9):
```
      DamageA
  0 ────────► 4 ─────┐
  │ DamageB   │      │
  ▼           ▼      ▼
  5 ────────► 6
  │ DamageA  (both halves damaged)
  │
  DamageB
  
  Any state ─CollapseA─► 7 ─CollapseA(recur)─► 0 + BlowUp
  Any state ─CollapseB─► 8 ─CollapseB(recur)─► 0 + BlowUp
```

### Base-tile constant table

| Variant slot | Constant |
|--------------|----------|
| Low-bridge base | `DAT_00ABAD1C` |
| High-bridge base | `DAT_00AA0E28` |
| Low pavement check A | `DAT_00ABC1E8` (DamageA + CollapseA) / `DAT_00ABC2B4` (DamageB + CollapseB) |
| Low pavement check B | `DAT_00AA0E38` (DamageA + CollapseA) / `DAT_00AA1130` (DamageB + CollapseB) |

(corrected 2026-07-12: was "DAT_00ABC1E8 (DamageA) / DAT_00ABC2B4
(DamageB/Collapse)" — the parenthetical implied both Collapse variants
share DamageB's pavement-check constants. Binary shows CollapseA_Low
(`0x56EF50`) tests the SAME constants as DamageA_Low (`DAT_00ABC1E8`/
`DAT_00AA0E38`), while CollapseB_Low (`0x56F2F0`) tests the same
constants as DamageB_Low (`DAT_00ABC2B4`/`DAT_00AA1130`) — grouping is
A-variant vs B-variant, not "DamageA vs DamageB+Collapse" — verified
via `decompile_function` on `0x56ED40`, `0x56EE40`, `0x56EF50`,
`0x56F2F0` — INFERENCE_HARDENED)
| Delta A-damage (offset 0) | `+0` |
| Delta B-damage (offset 1) | `+1` |
| Delta both-damaged | `+2` |
| Delta collapsed | `+3` |

### Full address list

| # | Name | Address |
|---|------|---------|
| 1 | `MapClass::UpdateRamp_NS_DamageA_Low` | `0x56ED40` |
| 2 | `MapClass::UpdateRamp_NS_DamageB_Low` | `0x56EE40` |
| 3 | `MapClass::UpdateRamp_NS_CollapseA_Low` | `0x56EF50` |
| 4 | `MapClass::UpdateRamp_NS_CollapseB_Low` | `0x56F2F0` |
| 5 | `MapClass::UpdateRamp_EW_DamageA_Low` | `0x56F690` |
| 6 | `MapClass::UpdateRamp_EW_DamageB_Low` | `0x56F7A0` |
| 7 | `MapClass::UpdateRamp_EW_CollapseA_Low` | `0x56F8B0` |
| 8 | `MapClass::UpdateRamp_EW_CollapseB_Low` | `0x56FC80` |
| 9 | `MapClass::UpdateRamp_NS_DamageA_High` | `0x572230` |
| 10 | `MapClass::UpdateRamp_NS_DamageB_High` | `0x572330` |
| 11 | `MapClass::UpdateRamp_NS_CollapseA_High` | `0x572440` |
| 12 | `MapClass::UpdateRamp_NS_CollapseB_High` | `0x5727E0` |
| 13 | `MapClass::UpdateRamp_EW_DamageA_High` | `0x572B80` |
| 14 | `MapClass::UpdateRamp_EW_DamageB_High` | `0x572C90` |
| 15 | `MapClass::UpdateRamp_EW_CollapseA_High` | `0x572DA0` |
| 16 | `MapClass::UpdateRamp_EW_CollapseB_High` | `0x573170` |

### Cell fields referenced

| Field | Purpose |
|-------|---------|
| `cell + 0x38` | IsoTileTypeIndex — current tile graphic |
| `cell + 0x44` | overlay_anim_ptr — cleared to -1 on full collapse |
| `cell + 0x11A` bit 0 | orientation (0 = NS bridge, 1 = EW) |
| `cell + 0x11B` | height level (used for collapse level-4) |
| `cell + 0x11E` | damage step (0, 4, 5, 6, 7, 8) |
| `cell + 0x140` bit 7 (0x80) | is-ramp flag |

### Direction offsets

`g_DirectionOffsets` and `DAT_0089F68A` are **paired 16-entry tables**:
- 8 directions × 2 shorts each (one X delta, one Y delta per direction)
- Indexed by `direction & 7`
- The DamageA variants use the compact `CONCAT22` pattern (no vector
  add helper); CollapseA/B use `MapCoord_Add` helper

### Rust parity implication

The Rust `src/sim/bridge_state.rs::BridgeRuntimeCell::deck_level: u8`
represents the final collapse state but not the intermediate 4
damage states (4, 5, 6 — half-damaged, other-half, both). For visual
parity (the bridge shows a different SHP depending on which side
took damage), Rust needs:
- A damage-step field (u8, values `{0, 4, 5, 6, 7, 8}`)
- 16 handler paths matching the gamemd dispatch (one per
  orient×height×variant combination)
- Tile-index rewriting via `SetOverlayAndPropagate` equivalent

Current Rust only handles intact/destroyed. The per-half-damage
animations are missing.

---

## 3. Bridge-repair-hut registry — `DAT_008B41A8`

### Structure

A **global `DynamicVectorClass<BuildingClass*>`** living as a
stand-alone object in `.data`:

| Address | Field | Type |
|---------|-------|------|
| `0x008B41A8` | vtable | int* — set to `0x7EA5A4` or `0x7EA5C4` in two different constructor calls |
| `0x008B41AC` | data_ptr | BuildingClass** — array of building pointers |
| `0x008B41B0` | capacity | int |
| `0x008B41B4` | is_valid | bool (byte) — (corrected 2026-05-28: was "owns_memory"; binary `DynamicVectorClass__Constructor @ 0x00525250` shows `+0x0C` = `is_valid` byte, `+0x0D` = `is_allocated` — PARAM1_TYPE_MISREAD / OFFSET_RETYPED_WRONG) |
| `0x008B41B5` | is_allocated (owns_memory) | bool (byte) — (corrected 2026-05-28: was "flag"; binary shows this is the `is_allocated` / owns-memory flag — PARAM1_TYPE_MISREAD / OFFSET_RETYPED_WRONG) |
| `0x008B41B8` | count | int |
| `0x008B41BC` | grow_step | int |

The vtable write at `0x004E7F4D` (→ `0x7EA5A4`) is the last step of
the **constructor** at `0x004E7F30`: zero data_ptr/capacity/count, set
`is_valid=1`/`is_allocated=0`, then overwrite the vtable with the
derived `DynamicVectorClass<T>` form and call an exit-registration
wrapper (`FUN_007C978A`, itself a thin wrapper over `FUN_007C970C`
returning the CRT `atexit`-style 0/-1 convention).

The write at `0x004E7F78` (→ `0x7EA5C4`) is **not** a second
constructor. It is the first real instruction of a separate routine at
`0x004E7F70` (load data_ptr, zero EBX, overwrite the vtable, then
`if (data_ptr != 0 && is_allocated) free(data_ptr)`) — the classic
destructor teardown idiom, not initialization. The constructor at
`0x004E7F30` pushes this exact address (`68 70 7F 4E 00` = `PUSH
0x004E7F70`) immediately before calling the exit-registration wrapper,
so `0x004E7F70` runs at program/static-object teardown, not at
construction. (corrected 2026-07-12: was "appear to be two
constructors (primary ctor + init helper) ... one is the base
`DynamicVectorClass<AbstractClass*>` and the other is a typed
subclass"; binary shows constructor `0x004E7F30` + destructor
`0x004E7F70`, the latter registered via the push-address-then-call-
atexit-wrapper pattern at `0x004E7F32`/`0x004E7F66` — verified via
`read_memory 0x004E7F30` length 100 (manual disassembly), `get_xrefs_to
0x004E7F70` (only reference is the PUSH at `0x004E7F32`, no other
caller), and `decompile_function 0x7C978A` — INFERENCE_HARDENED. The
two vtable values most likely represent derived `DynamicVectorClass<T>`
(`0x7EA5A4`, set at constructor exit) vs. base `VectorClass` (`0x7EA5C4`,
set at destructor entry to un-derive before the conditional free) —
same class hierarchy, not "different element types" as previously
guessed. This specific vtable-identity refinement is UNVERIFIED pending
a mangled-name/RTTI check of both vtables.)

### Producers (who pushes)

Two places populate this list:

**1. `FUN_00684C30`** — scenario post-init (calls
`MapClass::ComputeBridgeZones` + `UpdateBridgeZonesHelper` at the
end; likely `ScenarioClass::Do_Post_Init`).

For every building in `g_BuildingClass_Array`, checks if it has type
flag bit `0x4` (via `FUN_006E61F0`, probably
`BuildingTypeClass::Get_Category_Flags`). If set, pushes the
building pointer into the DynVec.

**2. `FUN_0067F9C0`** — savegame loader / file reader. Reads N
building pointers from the stream (where N is another 4-byte field
read from the stream) and pushes each into the DynVec.

### Consumers (who reads)

**1. `MapClass::UnregisterBridgeRepairHut` (`0x00577920`)** — vtable
slot 25. Reads `DAT_008B41A8 + 0x10` (= vtable+0x10, slot 4, likely
Find) to locate the building in the list, then shifts to remove.

**2. `FUN_00684C30`** itself — consumes via the push-only pattern,
so there's no separate "for each hut in list do X" consumer yet
traced beyond the unregister path.

### Purpose (inferred)

> **CORRECTED in §5 (item 5):** the framing below is wrong. `DAT_008B41A8`
> holds **TagClass instances filtered by destroyed-event-category bit 0x4**,
> not BuildingClass instances filtered by BuildingTypeClass category flag.
> The bridge-repair-hut consumer is one user of this registry, not its
> identity. Read §5 alongside this section.

Tracks the set of BuildingClass instances whose BuildingTypeClass
has category-flag bit `0x4` set. Based on:
- Usage exclusively from bridge-repair-hut teardown paths
  (`MapClass::UnregisterBridgeRepairHut` — the vtable slot is
  specifically named for bridge repair huts)
- Separate from the `MapClass+0x115C` cells-with-attached-object
  registry (that one stores *cell coords*; this one stores *building
  pointers*)
- Save/load persistence implies it's gameplay-relevant state that
  survives across sessions

The category-flag bit `0x4` is almost certainly
`BridgeRepairHut=yes` in art/rules INI. Worth verifying by grepping
for a type flag that maps to that INI key.

### Why two registries?

| Registry | Granularity | Used for |
|----------|-------------|----------|
| `MapClass+0x115C` | cell coords | finding cells with attached objects during tag/repair teardown |
| `DAT_008B41A8` | building pointers | finding all repair-hut buildings directly (faster than iterating all buildings + filtering) |

They track overlapping state from different angles. A single repair
hut contributes cells to the first AND its building ptr to the
second.

### Rust parity status

Rust does not implement bridge repair huts at all. If they're added:
- The `+0x115C` cell registry is more critical (needed for
  per-tick hut-repair logic)
- The `DAT_008B41A8` building registry is an optimization; can be
  replaced by a filter over the BuildingEntityStore

---

## 4. Other DynVec registries in the neighborhood

While tracing `DAT_008B41A8`, two more DynVecs at nearby addresses
surfaced. These are out of scope for this report but worth
capturing for future study:

- `DAT_008B40C8` (vtable) / `DAT_008B40CC` (data) / `DAT_008B40D0`
  (capacity) / `DAT_008B40D5` (flag) / `DAT_008B40D8` (count) /
  `DAT_008B40DC` (grow_step) — populated by `FUN_00684C30` for
  buildings with category-flag bit `0x10` (unknown — possibly
  ServiceDepot or NavalYard).

- A per-HouseClass DynVec at `house + 0x3C / +0x40 / +0x48 / +0x4C`
  — populated for buildings with category-flag bit `0x8`. Likely
  the per-house list of "specialty" buildings (construction
  yards, service depots, etc.).

The three type flags `0x4`, `0x8`, `0x10` are probably consecutive
bits in the same `BuildingTypeClass::CategoryFlags` field. Worth
decompiling `FUN_006E61F0` to enumerate all bit meanings.

---

## 5. Still-open items

**Update (2026-04-24, Task 13):** Items 3, 4, 5 resolved by the
MapClass Complete Decode cycle. See `MAPCLASS_COMPLETE_DECODE.md`
§§F, H, A/G for the evidence. Items 1, 2 are naming/metadata
conveniences only.

1. **Named rename proposal for Ghidra:**
   - `0x82A594` → `g_PassabilityMatrix` — already labeled ✓
   - `0x8B41A8` → `g_DestroyedEventTagList` (not "bridge repair hut"
     — the DynVec holds tags with destroyed-category events; the
     hut-scan was one consumer, not the owner). See
     `MAPCLASS_COMPLETE_DECODE.md` §A.
   - `0x8B40C8` → `g_AttackEventTagList` — tags with bit-0x10
     (attack/fire) events. See `MAPCLASS_COMPLETE_DECODE.md` §A + §G.

2. **Enum `PassabilityType`:** Ghidra already has the class symbol;
   extract the enum values and map them to `{1, 2, 3}` = `{Passable,
   Impassable, Sentinel}` or similar.

3. ~~**MovementZone row mapping:** 13 rows in the matrix.~~
   → **Resolved:** `MAPCLASS_COMPLETE_DECODE.md` §F — all 13 rows
   labeled and 1:1 verified against
   `src/rules/locomotor_type.rs::MovementZone`.

4. ~~**The `|height_delta| < 4` quirk in the right-walk of
   `ZoneFloodFillScanLine`.**~~
   → **Resolved:** `MAPCLASS_COMPLETE_DECODE.md` §H — documented as
   a preserved TS-era bug (left walk uses `|Δh| ≤ 1`, right walk
   uses `|Δh| ≤ 3`). Recommendation: Rust should implement symmetric
   `|Δh| ≤ 1` in both directions; the observable effect on authored
   YR maps is zero.

5. ~~**FUN_006E61F0 full enumeration.**~~
   → **Resolved with correction:** `MAPCLASS_COMPLETE_DECODE.md` §A
   — `FUN_006E61F0` is NOT a BuildingTypeClass flag lookup. It
   aggregates TagClass → event-category flags. Category bits 0x01,
   0x02, 0x04, 0x08, 0x10 map to time/counter/destroyed/proximity/
   attack event types. Exhaustive event-code → bit table in §A.
   The three DynVecs (`DAT_008B40C8`, `DAT_008B41A8`, `HouseClass+
   0x3C`) are pre-filtered trigger-tag registries, not
   building-type registries.

**Additional items resolved this cycle (from broader MapClass
research, not listed above but closed in the Complete Decode):**

- `UpdateRamp` family 16 variants — NS/EW orientation use different
  state-value ranges. See `MAPCLASS_COMPLETE_DECODE.md` §L; the §2
  "Template: all 16 follow this shape" claim above is **correct
  per-orientation** but **incorrect across orientations** — the NS
  binary family `UpdateRamp_NS_*_Low` (mis-named "NS"; actually transitions
  the **physically-EW** state range) uses state values `{0, 4, 5, 6, 7, 8}`,
  while `UpdateRamp_EW_*_Low` (mis-named "EW"; actually the **physically-NS**
  family) uses the full range `{9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0x11}`
  — `{0x10, 0x11}` are the half-collapsed/collapsed analogues of `{7, 8}`,
  not the whole NS state range. Verified at `UpdateRamp_EW_DamageA_Low @
  0x56F690`. Physical-axis attribution confirmed 2026-05-13 via SHP frame
  inspection (frame 0 = EW, frame 9 = NS). See §2 for per-axis transition tables.
- `UpdateBridgeZonesHelper` caller taxonomy — 33 sites across 8
  categories. See §K of the Complete Decode.

---

## Sources

### Newly decompiled / re-read
- `0x56C510` MapClass::UpdateBridgeZonesHelper (re-read in depth)
- `0x56CB90` MapClass::ZoneFloodFillScanLine (re-read in depth)
- `0x56ED40` UpdateRamp_NS_DamageA_Low
- `0x56EE40` UpdateRamp_NS_DamageB_Low
- `0x56F2F0` UpdateRamp_NS_CollapseB_Low
- `0x572230` UpdateRamp_NS_DamageA_High
- `0x00684C30` (scenario post-init, populates bridge-hut registry)
- `0x0067F9C0` (savegame reader)

### Raw memory
- `0x0082A594, 416 bytes` — g_PassabilityMatrix full dump

### Field access scans
- `get_field_access_context(0x82A594)` → callers:
  `UpdateBridgeZonesHelper`, `Zone_precheck` (`0x42C290`),
  `ZoneMap::FloodFillReachableZones` (`0x5840C0`),
  `ZoneMap::FindBestCompatibleMovementZone` (`0x5889F0`)
- `get_field_access_context(0x8B41A8)` → populated by scenario init
  + savegame reader; consumed by `UnregisterBridgeRepairHut`

### Function search
- `search_functions("UpdateRamp")` → 16 variants, all addresses
  confirmed

### Referenced docs
- `MAPCLASS_GHIDRA_REPORT.md` (zone system baseline)
- `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` (UpdateRamp template, partial
  zone graph docs)
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` (vtable corrections,
  +0x115C clarification)
- `ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md` (AssignOrphaned vs
  MergeAdjacent; relevant for the phase-7 BFS entry point)
