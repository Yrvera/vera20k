# AStar_pathfind_search — Decode Doc
**Proposed Ghidra label:** AStar_pathfind_search (already labelled)

## Summary

Top-level A* search orchestrator at `0x0042C900`. Called by `FootClass__Run_AStar`
(`0x004cbba0`). Prepares the PathfinderClass state, resolves bridge-aware source and
destination coordinates, performs a zone reachability precheck, calls `AStar_main_loop`
in a retry loop (up to 5 retries by default, 4 for a unit heading towards a specific
target cell), updates hierarchical edges on each failure, and drives the post-process
smoothing/optimization pipeline (`Path_smooth_corners`, `Path_optimize_straight_segments`)
via `AStar_main_loop`'s internal call chain. Returns the result from `AStar_main_loop`.

**Active in YR: Yes.** Reachable through `FootClass__Run_AStar` (`0x004cbba0`) →
`AStar_pathfind_search`. `FootClass__Run_AStar` is called from the locomotor movement
update paths for Drive, Ship, and Walk locomotors, all of which are live in standard
YR skirmish play. Verified via `get_function_callers 0x0042C900` — sole caller is
`FootClass__Run_AStar @ 004cbba0`.

---

## Signature

```c
int __thiscall
AStar_pathfind_search(
    int   param_1,        // PathfinderClass* (this, ECX)
    short *param_2,       // source MapCoord (start cell, short[2])
    short *param_3,       // destination MapCoord (target cell, short[2])
    int   *param_4,       // FootClass* (the moving unit)
    undefined4 param_5,   // extra arg forwarded to AStar_main_loop
    int   param_6,        // target cell index (-1 = none)
    uint  param_7,        // movement zone (0xFFFFFFFF → derive from unit type)
    uint  param_8         // [in/out flag byte] HS-capable flag (hierarchical search enabled)
)
```

`param_1` arrives in `ECX` (thiscall). Assembly at entry confirms:
`MOV byte ptr [EBP + 0x38], 0x1` — sets PathfinderClass+0x38 immediately.
(verified via `get_assembly_context 0x0042c900`)

---

## Decompilation Excerpt

```c
// verified via decompile_function 0x0042C900
*(undefined1 *)(param_1 + 0x38) = 1;        // PathfinderClass+0x38 = search-active flag
PathfinderClass__Reset();                    // @ 0x0042a5b0

// Reset 3 open-set heap objects at PathfinderClass+0x74..+0x8C (stride 6 dwords each)
piVar6 = (int *)(param_1 + 0x74);
iVar7 = 3;
do {
    (**(code **)(*piVar6 + 0xc))();          // vtable+0x0C on each heap: Clear()
    piVar6 = piVar6 + 6;
    iVar7--;
} while (iVar7 != 0);

*(uint *)(param_1 + 0x3c) = param_8;        // PathfinderClass+0x3C = HS-capable flag copy

iStack_18 = MapClass__Get_CellClass(param_2);  // source CellClass*
iVar7    = MapClass__Get_CellClass(param_3);   // dest CellClass*

// Resolve movement zone: if 0xFFFFFFFF, derive from unit's TechnoType.SpeedType
// vtable+0x84 = TechnoClass__GetTechnoType_Trampoline (verified: read_memory 0x007eb058 slot 33)
// TechnoType+0x5b4 = SpeedType/MovementZone field
if (param_7 == 0xffffffff) {
    iVar3  = (**(code **)(*param_4 + 0x84))();
    uVar5  = *(uint *)(iVar3 + 0x5b4);
}

// Derive zone IDs for source and dest cells (for hierarchical reachability precheck)
iStack_14 = MapClass__GetZoneID(param_2, uVar5, (char)param_4[0x23]);
// param_4[0x23] = *(param_4 + 0x8C) = FootClass bool field (Crusher/Bridge-aware flag?)

// Resolve bridge-aware coordinates
puVar4   = MapClass__ResolvePathCoord_BridgeAware(&param_8, iStack_18, ...);
uStack_1c = *puVar4;  // resolved source coord (MapCoord, 4-byte)
puVar4   = MapClass__ResolvePathCoord_BridgeAware(&param_8, iVar7, ...);
uStack_20 = *puVar4;  // resolved dest coord

// Infantry chrono-teleport path: if unit is infantry AND TechnoType has teleporter flag
// vtable+0x2C = What_Am_I() — for InfantryClass returns 0xF (verified: read_memory 0x007eb058 + 0x2C)
iVar7 = (**(code **)(*param_4 + 0x2c))();
if ((iVar7 == 0xf) && (*(char *)(param_4[0x1b0] + 0xd94) != '\0')) {
    // Chrono-infantry special path: CoCreateInstance locomotor, get move target, release
    // param_7 = 7 (SPEED_Foot override for chrono infantry)
    ...
}

// HS-capable check:
// Condition: !TechnoType.IsSpeedTypeAir && FootClass+0x3D5 != 0 &&
//            !(vtable+0x320() == non-zero) && src/dst in playfield
// Sets bit 0 of param_8 (the hs_enable byte)
iVar7 = (**(code **)(*param_4 + 0x84))();  // get TechnoType again
if ((*(char *)(iVar7 + 0xc94) == '\0')          // TechnoType+0xC94: IsSpeedTypeAir?
 && (*(char *)((int)param_4 + 0x3d5) != '\0')   // FootClass+0x3D5: HS allowed flag
 && (cVar2 = (**(code **)(*param_4 + 800))(), cVar2 == '\0')  // vtable+0x320 = chrono lock?
 && MapClass__Is_Cell_In_Playfield(src, 1)
 && MapClass__Is_Cell_In_Playfield(dst, 1)) {
    param_8 |= 1;  // enable hierarchical search
}

// Zone-ID equality check — if zones differ and HS is on, bail immediately
if (iStack_14 == iVar3) {
    if ((char)param_8 != 0 && Zone_precheck(src, dst, param_7, param_4) == 0) {
        // HS feasibility failed: log "Hierarchical findpath failure"
        param_8 &= ~1;  // disable HS
    }
} else if ((char)param_8 != 0) {
    return 0;  // zones differ with HS on: unreachable
}

// Retry loop — max iStack_14 retries
// iStack_14 = (param_6 != -1) ? 4 : 5 — 4 retries for targeted move, 5 otherwise
param_4  = 0;  // reuse as retry counter
iStack_14 = (-(uint)(param_6 != -1) & 0xfffffffc) + 5;  // 4 or 5
while (true) {
    if (!(char)param_8) {
        // Warn if HS disabled and src != dst: "Warning: A* without HS"
        if ((short)uStack_1c != (short)uStack_20 || uStack_1c._2_2_ != uStack_20._2_2_)
            Register_heap_pool("Warning. A* without HS...", ...);
    }
    iStack_18 = AStar_main_loop(param_2, param_3, piVar1, param_5, param_6, param_8);
    if (iStack_18 != 0 || !(char)param_8) break;  // success or HS disabled
    // Failure: log "Regular findpath failure" if src/dst are >1 cell apart (Chebyshev)
    param_4++;  // increment retry count
    PathfinderClass__UpdateHierarchicalEdges(piVar1);  // invalidate blocked edges
    PathfinderClass__Reset();
    // Re-evaluate HS flag
    bVar8 = *(char *)(param_1 + 0x38) != '\0';
    param_8 = (param_8 & ~1) | bVar8;
    if (iStack_14 <= (int)param_4) return iStack_18;  // retry limit hit
    if (bVar8 && Zone_precheck(src, dst, param_7, piVar1) == 0) return iStack_18;
}
return iStack_18;
```

---

## Behavioral Analysis

### Phase 1 — Initialization
- Sets `PathfinderClass+0x38 = 1` (search-active marker) immediately on entry.
- Calls `PathfinderClass__Reset()` to clear open-set and node table.
- Clears 3 heap objects at `PathfinderClass+0x74` (stride 6 dwords, 3 slots) via
  their vtable `Clear()` call at vtable+0xC on each heap object.
- Stores `param_8` (HS-capable flag) into `PathfinderClass+0x3C`.

### Phase 2 — Zone ID resolution and bridge-aware coord snap
- Gets `CellClass*` for both source and destination via `MapClass__Get_CellClass`.
- Derives movement zone via unit's TechnoType if `param_7 == 0xFFFFFFFF`
  (vtable+0x84 = `TechnoClass__GetTechnoType_Trampoline`, field `TechnoType+0x5B4` = SpeedType).
- Calls `MapClass__GetZoneID` for both to get hierarchical zone indices.
- Calls `MapClass__ResolvePathCoord_BridgeAware` on both cells to produce
  bridge-snapped MapCoord values (`uStack_1c` / `uStack_20`).

### Phase 3 — Infantry chrono-teleport pre-pass
- If `What_Am_I() == 0xF` (infantry, vtable+0x2C confirmed via read_memory)
  AND `TechnoTypeClass+0xD94 != 0` (teleporter flag):
  - Creates a COM locomotor instance via `CoCreateInstance` using CLSID from
    `TechnoTypeClass+0x34C` (infantry locomotor CLSID).
  - Calls the locomotor's vtable+0x0C to get the move target coord into `auStack_10`.
  - Overrides `param_7 = 7` (SPEED_Foot).
  This block enables chrono-infantry to resolve their warp target before pathfinding.
  The COM create/release pattern with `GameDebugLog__Assert(0x80004003)` on failure
  is the standard YR locomotor-attachment guard.

### Phase 4 — HS-capable flag decision
- Four conditions all required for HS (hierarchical search):
  1. `TechnoType+0xC94 == 0` — NOT an air-speed unit (verified: read_memory slot 33 gives TechnoType trampoline).
  2. `FootClass+0x3D5 != 0` — per-unit HS-allowed flag (set during unit init).
  3. `vtable+0x320() == 0` — vtable slot at byte offset 800 is NOT a chrono-locked unit.
     (YELLOW — vtable slot 0x320/4=200 not verified in this session; marking unverified.)
  4. Both source and destination cells in playfield.
- If all four pass: `param_8 |= 1` enabling hierarchical (zone-level) search.
- Zone-ID mismatch: if zones differ AND HS is on → return 0 immediately (unreachable).
- Zone-ID match: run `Zone_precheck`; if it fails, disable HS for this search.

### Phase 5 — Retry loop
- Max retries: `iStack_14 = (param_6 != -1) ? 4 : 5`.
  - `param_6 == -1` means no specific target cell (e.g., scatter) → 5 retries.
  - `param_6 != -1` means moving toward a specific cell → 4 retries.
- Each iteration calls `AStar_main_loop`. On failure with HS enabled:
  - Logs "Regular findpath failure" (if src/dst Chebyshev distance > 1).
  - Calls `PathfinderClass__UpdateHierarchicalEdges` to invalidate newly-blocked
    hierarchical edges.
  - Calls `PathfinderClass__Reset` to clear node tables.
  - Re-evaluates the HS flag from `PathfinderClass+0x38`.
  - Checks `Zone_precheck` again; if it fails, returns immediately.
- If `AStar_main_loop` returns non-zero (path found), or HS is disabled: exit loop.
- Return value is always the raw return of `AStar_main_loop`.

### Open-set / closed-set lifecycle
`AStar_main_loop` owns the open/closed sets internally (within PathfinderClass heap
buffers at `+0x0C`, `+0x10`, `+0x14`, `+0x68`). `AStar_pathfind_search` does not
directly manipulate open/closed sets — it only calls `PathfinderClass__Reset` which
zeroes those buffers between retries.

### Post-process pipeline
`AStar_pathfind_search` does **not** call `Path_smooth_corners` or
`Path_optimize_straight_segments` directly. These are called inside `AStar_main_loop`
after path reconstruction. `AStar_pathfind_search` drives the retry loop only; the
post-process pipeline is internal to `AStar_main_loop`.

---

## Struct Field Accesses

| Access site | Object | Byte offset | Interpretation |
|---|---|---|---|
| `*(param_1 + 0x38) = 1` | PathfinderClass | 0x38 | Search-active / valid-search flag (bool) |
| `*(param_1 + 0x3C) = param_8` | PathfinderClass | 0x3C | HS-capable flag store (uint) |
| `param_1 + 0x74` .. `+0x8C` | PathfinderClass | 0x74–0x8C | 3 × open-set heap objects (stride = 6 dwords = 0x18 bytes) |
| `param_4[0x23]` | FootClass (int*) | 0x23×4=0x8C | Crusher/bridge-aware passability param |
| `*(param_4 + 0x3D5)` | FootClass (byte) | 0x3D5 | Per-unit HS-allowed flag |
| `*(param_4[0x1b0] + 0xD94)` | TechnoTypeClass | 0xD94 | Teleporter/chrono-infantry flag |
| `*(TechnoType + 0x5B4)` | TechnoTypeClass | 0x5B4 | SpeedType / movement zone |
| `*(TechnoType + 0xC94)` | TechnoTypeClass | 0xC94 | Is-air-speed flag |

Frame note: `param_4` is a `FootClass*` (int*-typed in decompile). All dword-indexed
accesses (`param_4[N]`) translate to byte offset `N × 4`.

---

## Callers

| Caller | Address | Notes |
|---|---|---|
| `FootClass__Run_AStar` | `0x004cbba0` | Sole caller (verified via `get_function_callers 0x0042C900`) |

---

## Callees

| Callee | Address | Role |
|---|---|---|
| `PathfinderClass__Reset` | `0x0042a5b0` | Zeroes open/closed set heaps + increments generation counter |
| `MapClass__Get_CellClass` | `0x005657a0` | Returns CellClass* for a coord |
| `MapClass__GetZoneID` | `0x0056d230` | Returns zone index for HS reachability check |
| `MapClass__ResolvePathCoord_BridgeAware` | `0x00583180` | Snaps coord to bridge-aware grid position |
| `MapClass__Is_Cell_In_Playfield` | `0x00578460` | Validates coord within map bounds |
| `Zone_precheck` | `0x0042c290` | Hierarchical zone feasibility A* over zone graph |
| `AStar_main_loop` | `0x00429a90` | Core A* node expansion loop |
| `PathfinderClass__UpdateHierarchicalEdges` | `0x0042ccd0` | Invalidates zone edges after path failure |
| `Register_heap_pool` | `0x004068e0` | Debug logging (out-of-scope, runtime utility) |
| `GameDebugLog__Assert` | `0x007dc720` | Debug assert on COM failure (out-of-scope) |

All callees verified via `get_function_callees 0x0042C900`.

---

## Globals / Enums / INI

| Symbol | Address | Role |
|---|---|---|
| `s_Warning__A__without_HS__` (string) | `0x008187f0` | "Warning. A* without HS..." — debug log for non-HS path attempt |
| `s_Regular_findpath_failure` (string) | `0x008187c0` | "Regular findpath failure..." — retry failure log |
| `s_Hierarchical_findpath_failure` (string) | `0x00818820` | "Hierarchical findpath failure..." — zone precheck failure log |
| `DAT_00818858` (CLSID?) | `0x00818858` | GUID-like data used in `CoCreateInstance` call for chrono-infantry locomotor |

No INI keys directly read by this function. Movement zone is read from `TechnoType+0x5B4`
which is set during TechnoType INI parsing (`Speed=` / `SpeedType=` keys).

---

## Out-of-Scope References

- `CoCreateInstance` / COM locomotor pattern (chrono-infantry block) — locomotor-system,
  separately documented.
- `PathfinderClass__UpdateHierarchicalEdges` (`0x0042ccd0`) — in-scope task #11.
- `Zone_precheck` (`0x0042c290`) — in-scope task #16.
- `AStar_main_loop` (`0x00429a90`) — in-scope task #4.
- `PathfinderClass__Reset` (`0x0042a5b0`) — in-scope task #9.

---

## Unverified / YELLOW

- **YELLOW: vtable+0x320 identity.** The call `(**(code **)(*param_4 + 800))()` at byte
  offset 0x320 in the HS-capable check was not resolved in this session. The vtable read
  at `0x007eb058` only covers 200 bytes (50 slots); slot 200 (offset 0x320) was not fetched.
  Likely a `Is_Chrono_Locked` or `Is_Currently_Teleporting` virtual — function returns
  0 when HS is allowed, non-zero to suppress HS. Mark UNCHECKED until verified.

- **YELLOW: TechnoType+0xD94 semantic.** Identified as the chrono-infantry teleporter
  flag (tested only when `What_Am_I() == 0xF`). The exact INI key mapping for offset
  0xD94 was not traced in this session. `param_4[0x1b0]` is the TechnoTypeClass* pointer
  at `FootClass+0x6C0` (= `0x1b0 * 4`).

- **YELLOW: TechnoType+0xC94 semantic.** Identified as "IsSpeedTypeAir" from context
  (disables HS for aircraft). Exact INI mapping not traced.

- **YELLOW: FootClass+0x3D5 semantic.** Single-byte flag at FootClass+0x3D5 gates HS
  eligibility per-unit. Not traced to initialization site in this session.

- **YELLOW: param_4[0x23] (=FootClass+0x8C) semantic.** Passed as the third argument
  to `MapClass__GetZoneID` and `MapClass__ResolvePathCoord_BridgeAware`. Likely a
  Crusher/VehicleTransport passability mode byte. Not confirmed.
