# PathfinderClass__UpdateBridgePassability — Decode Doc
**Proposed Ghidra label:** PathfinderClass__UpdateBridgePassability (already labelled)

## Summary

Bridge-approach A* cost marker toggle function at `0x0042ACF0`. Called exclusively by
`AStar_main_loop` (`0x00429A90`) before and after each search when `PathfinderClass+0x3C != 0`.
Temporarily XOR-toggles `CellClass+0x140 bit 0x40000` on cells along nearby peer units'
queued movement paths, so the A* cost consumer (`AStar_compute_edge_cost @ 0x00429830`)
sees a 4× cost multiplier on cells where peer units are already moving. The normal lifecycle
is: toggle-on → A* search → toggle-off (second call cancels the first via XOR). A `PathfinderClass+0x3C != 0` guard at the top exits immediately when bridge passability marking
is disabled.

**Active in YR: Yes, conditional on `PathfinderClass+0x3C != 0`.** Sole verified caller is
`AStar_main_loop @ 0x00429A90`; function is enabled by constructor (`PathfinderClass+0x03 = 1`).
(verified via `get_function_callers 0x0042ACF0` and `decompile_function 0x0042ACF0`)

---

## Signature

```c
void __thiscall PathfinderClass__UpdateBridgePassability(int param_1, int *param_2)
```

`param_1` is the PathfinderClass* (`this`). `param_2` is the FootClass* of the
searching unit. (`__thiscall` with two params: `this` in ECX, `param_2` as first
stack arg — but the Ghidra decompile uses `param_1` for the unit and `param_2` for
the PathfinderClass, based on how fields are accessed.)

Note: The Ghidra decompile calls `param_1` the unit coord (initial: `local_1c = (short)param_1`)
and `piVar13 = param_2` (the FootClass*). The function reads `*(char *)(param_1 + 3)` as the
master enable byte, so `param_1` is actually the PathfinderClass* with the coord packed in the
low bytes. Cross-verified against constructor: `param_1[3] = 1` in constructor writes byte
at offset 3 of param_1 (PathfinderClass). Confirmed via `decompile_function 0x0042A6D0`.

---

## Decompilation Excerpt

```c
// verified via decompile_function 0x0042ACF0

// Master enable check: PathfinderClass+0x03
if (*(char *)(param_1 + 3) == '\0') return;

// Get searching unit's current cell via vtable+0x1B8 (Get_Cell_Packed / NW-cell getter)
puVar3 = (**(code **)(*param_2 + 0x1b8))(&param_2);
uStack_24 = *puVar3;  // unit's current cell coord (MapCoord, packed shorts)
iVar4 = MapClass__Get_CellClass(&uStack_24);  // CellClass* for current cell

// Pseudo-random probe direction from timer
puVar5 = RateTimer__Current(auStack_8);
uVar10 = ((*puVar5 >> 0xc) + 1 >> 1) & 7;  // dir = (((timer >> 12) + 1) >> 1) & 7

// Probe cell = current cell + g_DirectionOffsets[dir]
uStack_10 = current_coord + g_DirectionOffsets[uVar10];
iVar6 = MapClass__Get_CellClass(&uStack_10);  // CellClass* for probe cell

// Layer/list selection on probe cell:
// Ground list Cell+0xE4 when: NOT a bridge cell, OR level gap ≤ 3 AND unit not on bridge
// Bridge list Cell+0xE8 when: probe is bridge AND (level gap > 3 OR unit on bridge at Foot+0x8C)
if (!(probe.flags & 0x100) || (abs(current.level - probe.level) < 4 && !piVar13[0x23])) {
    piVar7 = Cell+0xE4;  // ground object list
    bVar1 = 0;
} else {
    piVar7 = Cell+0xE8;  // bridge object list
    bVar1 = 1;
}

// Fallback: if selected list is null, scan 5x5 via FUN_0042B080
if (piVar7 == null) {
    piVar7 = FUN_0042B080(probe_cell + 0x24, probe.level + (bVar1 ? 4 : 0));
}

// Iterate peer objects in selected list
iVar4 = (**(code **)(*piVar13 + 0x84))();  // searching unit's TechnoType
do {
    iVar8 = (**(code **)(*piVar7 + 0x2c))();  // peer kind (What_Am_I)
    if (iVar8 == 1 || iVar8 == 0xf) {          // only infantry (0xF) and unit/vehicle (1)
        // Gate: skip if PathfinderClass+0x3C != 2 and peer is same type / lower rank
        // or if peer path start is outside playfield
        if (PathfinderClass+0x3C == 2 ||
            (searching_type != peer_type &&
             TechnoType+0x678(searching) > TechnoType+0x678(peer) &&
             MapClass::Is_Cell_In_Playfield(peer_path_start))) {
            // Walk peer's queued path: base = peer + 0x5E0 (peer[0x178]),  max 24 entries
            // Kind 1: needs path[0] != -1 and path[1] != -1
            // Kind 0xF: needs path[0], [1], [2] != -1
            iVar9 = peer[0x17a];  // third path entry for kind 0xF
            do {
                if (iVar12 > 0x17) break;  // max 24 entries
                dir = *piVar13;   // current direction byte from path queue
                if (dir == 8) {
                    // Tube step: read g_TubeArray[Cell+0x116]+0x28 for next coord
                    // -1 tube index → reset coord to (0,0)
                } else {
                    coord += g_DirectionOffsets[dir];
                }
                dest_cell = MapClass::Get_CellClass(coord);
                // XOR-toggle: dest_cell.flags[0x140] ^= (~src_cell.flags[0x140] ^ dest_cell.flags[0x140]) & 0x40000
                // Since Get_CellClass is called twice with same coord, src == dest, so net: dest ^= 0x40000
                *(uint *)(dest + 0x140) ^= 0x40000;
                piVar13++;
                iVar12++;
            } while (*piVar13 != -1);
            bVar14 = true;
        }
    }
    piVar7 = piVar7[0xc];  // next object in list
} while (piVar7 != null);

// If no peer path found and PathfinderClass+0x3C == 1: clear +0x3C to 0 and return
if (!bVar14 && PathfinderClass+0x3C == 1) {
    PathfinderClass+0x3C = 0;
    return;
}

// 5x5 fallback occupation toggle (centered on probe cell)
for (iVar4 = -2; iVar4 <= 2; iVar4++) {
    for (iVar8 = -2; iVar8 <= 2; iVar8++) {
        candidate = probe.coord + (iVar4, iVar8);
        cell = MapClass::Get_CellClass(candidate);
        if (cell+0x124 != 0) {  // occupied?
            if (cell+0x24 == searching_unit_coord) continue;  // skip own cell
            cell.flags[0x140] ^= 0x40000;
        }
    }
}
// Unconditionally toggle probe center cell
probe_cell.flags[0x140] ^= 0x40000;
return;
```

---

## Behavioral Analysis

### When it is invoked

`AStar_main_loop` (`0x00429A90`) calls this function in three places (verified via
`decompile_function 0x00429a90`):

1. Before seeding the initial closed list, when source/destination cell or height differs
   and `PathfinderClass+0x3C != 0`.
2. After successful path reconstruction/smoothing (success tail), gated by `+0x3C != 0`.
3. On failure/no-result exit, gated by `+0x3C != 0`.

The XOR toggle means the second call (post-search cleanup) cancels the first call's marks.
Net effect after a complete search: the static path grid is unchanged.

### Master enable byte

`PathfinderClass+0x03` is set to `1` by `PathfinderClass__Constructor` (`0x0042A6D0`).
The function returns immediately if zero. (Verified: constructor decompile has
`param_1[3] = 1`; function checks `*(char *)(param_1 + 3)`.)

### Probe selection

A pseudo-random adjacent probe cell is chosen using `RateTimer__Current`:
- `dir = (((timer >> 0xC) + 1) >> 1) & 7` — maps to 0–7, one of 8 compass directions.
- Probe = unit's current cell + `g_DirectionOffsets[dir]`.

### Peer scan

For each ground or bridge object on the probe cell:
- Only processes kinds `1` (infantry?) and `0xF` (vehicle/unit?). All others are skipped.
  (Note: `0xF` returns from `InfantryClass__What_Am_I` and `1` from vehicle type — YELLOW,
   pending formal verification of these kind values against `What_Am_I` returns.)
- Priority gate: if `PathfinderClass+0x3C != 2`, skips peers that are the same TechnoType,
  have TechnoType field +0x678 ≥ the searching unit's, or are outside the playfield.
- If eligible, walks the peer's queued path (base `peer + 0x5E0`), up to 24 steps.
- Each step XOR-toggles `CellClass+0x140 bit 0x40000` on the destination cell.
- Tube directions (code 8) are handled via `CellClass+0x116` tube index.

### 5x5 fallback

If no eligible peer path was found and `PathfinderClass+0x3C == 1`: clears `+0x3C = 0`
and returns (no 5x5 phase). Otherwise runs the 5x5 phase:
- Iterates dx, dy ∈ {-2, -1, 0, 1, 2} around the probe cell.
- For each cell: if `CellClass+0x124 != 0` (occupied) and not the searching unit's
  own cell, XOR-toggles `CellClass+0x140 bit 0x40000`.
- Unconditionally toggles the probe center cell at the end.

### Cost consumer

`AStar_compute_edge_cost` (`0x00429830`) reads `dest_cell+0x140 & 0x40000` and
multiplies edge cost by `4.0` when set. The `0x40000` bit is separate from `0x400`
(bridge inactive/fallback marker). (Verified per prior research doc `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.)

---

## Struct Field Accesses

| Access | Object | Byte offset | Interpretation |
|--------|--------|-------------|----------------|
| `*(char *)(param_1 + 3)` | PathfinderClass | 0x03 | Master enable byte (1=enabled) |
| `*(int *)(iStack_20 + 0x3c)` | PathfinderClass | 0x3C | Per-search HS-capable/urgency mode |
| `(*param_2 + 0x1b8)` vtable | FootClass | vtable+0x1B8 | Get_Cell_Packed: NW cell coord getter |
| `piVar13[0x23]` | FootClass (int*) | 0x23×4=0x8C | On-bridge byte |
| `piVar7[0x156]` | Peer object (int*) | 0x156×4=0x558 | Peer path queue index? |
| `piVar7[0x178]` | Peer object (int*) | 0x178×4=0x5E0 | Peer path queue base (direction array) |
| `piVar7[0x179]` | Peer object (int*) | 0x179×4=0x5E4 | Peer path[1] |
| `piVar7[0x17a]` | Peer object (int*) | 0x17a×4=0x5E8 | Peer path[2] |
| `Cell+0xE4` | CellClass | 0xE4 | Ground object list head |
| `Cell+0xE8` | CellClass | 0xE8 | Bridge/deck object list head |
| `Cell+0x116` | CellClass (short) | 0x116 | Tube index (-1 = no tube) |
| `Cell+0x11B` | CellClass (char) | 0x11B | Signed cell level |
| `Cell+0x124` | CellClass (byte) | 0x124 | Occupation byte |
| `Cell+0x140 & 0x100` | CellClass | 0x140 bit 8 | Structural bridge cell flag |
| `Cell+0x140 & 0x40000` | CellClass | 0x140 bit 18 | Temporary A* bridge-approach cost marker |
| `TechnoType+0x678` | TechnoTypeClass | 0x678 | Priority/rank field for peer eligibility |

---

## Callers

| Caller | Address | Notes |
|--------|---------|-------|
| `AStar_main_loop` | `0x00429a90` | Sole caller (verified via `get_function_callers 0x0042ACF0`). Called before/after A* when `PathfinderClass+0x3C != 0`. |

---

## Callees

| Callee | Address | Role |
|--------|---------|------|
| `MapClass__Get_CellClass` | `0x005657a0` | Convert packed coord to CellClass* |
| `RateTimer__Current` | `0x004c93d0` | Pseudo-random probe direction source |
| `MapClass__Is_Cell_In_Playfield` | `0x00578460` | Peer path start validation |
| `FUN_0042B080` | `0x0042b080` | Fallback 5x5 nearby object scanner (in-scope task #25) |

All callees verified via `get_function_callees 0x0042ACF0`.

---

## Globals / Enums / INI

| Symbol | Address | Role |
|--------|---------|------|
| `g_DirectionOffsets` (inferred) | `~0x0089F688` | 8-entry table of `(dx,dy)` cell offsets indexed by direction 0–7 |
| `g_TubeArray` | referenced in decompile | Low-bridge tube array (indexed by `Cell+0x116`) |
| `DAT_0089C2D8` | referenced in FUN_0042B080 | Leptons-per-height-level scale factor |

No INI keys directly configure this function. `PathfinderClass+0x3C` value is set by
`AStar_pathfind_search` from its `param_8` argument.

---

## Out-of-Scope References

- `AStar_compute_edge_cost` (`0x00429830`) — consumer of the `0x40000` marker,
  in-scope task #5 (already completed).
- `FUN_0042B080` (`0x0042b080`) — fallback scanner, in-scope task #25.
- `PathfinderClass__Constructor` (`0x0042a6d0`) — sets master enable byte, in-scope task #8.
- `RateTimer__Current` — out-of-scope runtime utility (per manifest).
- Prior research: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` and
  `PATHFINDER_ALT_OBJECT_LIST_FUN_0042B080_GHIDRA_REPORT.md` contain exhaustive analysis.
  This decode doc cross-verifies key claims and consolidates for the pathfinding team.

---

## Unverified / YELLOW

- **YELLOW: Peer kind values.** The code checks `What_Am_I() == 1` and `What_Am_I() == 0xF`.
  From prior decode: `InfantryClass__What_Am_I` returns `0xF`. The `1` kind is labeled
  here as vehicle/unit. `UnitClass__What_Am_I` at `0x00746e20` was not decompiled in this
  session — kind 1 is UNCHECKED; it may be vehicle, building, or something else.

- **YELLOW: `PathfinderClass+0x3C` semantic label.** Called "per-search urgency" and
  "HS-capable flag" in different docs. The `AStar_pathfind_search` decode confirms it's
  written as the HS-capable flag. When `+0x3C == 2`: bypasses all peer eligibility gates.
  When `+0x3C == 1`: cleared to 0 and returns if no peer found. When `+0x3C == 0`: function
  exits immediately. The exact values 1 and 2 are not formally named.

- **YELLOW: TechnoType+0x678 semantic.** Used as a priority/rank comparison between
  the searching unit and peer objects: searching unit must have higher `+0x678` value
  than peer to process that peer's path. Exact INI key mapping for this field was not
  traced in this session.

- **YELLOW: vtable+0x1B8 identity.** The decompile shows `(**(code **)(*param_2 + 0x1b8))(&param_2)` to get the unit's current cell. Labeled as `Get_Cell_Packed` per the InfantryClass vtable analysis in the main `CLAUDE.md`. Consistent with vtable+0x1B8 description in research docs but not independently verified in this session via read_memory.
