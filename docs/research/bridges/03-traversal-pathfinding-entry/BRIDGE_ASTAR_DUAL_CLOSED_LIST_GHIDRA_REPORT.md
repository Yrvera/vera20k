# Bridge A* Spine & Dual Closed-List — Ghidra Research Report

**Phase:** Phase 1 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #1 (AStar_main_loop), #2 (AStar_pathfind_search), #3 (PathfinderClass__UpdateBridgePassability), #7 (PathfinderClass dual closed-list layout), #8 (FootClass__Find_Path), #9 (FootClass__Run_AStar)
**Companion doc:** `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` (items #4, #5, #6)
**Date:** 2026-05-13
**Active in YR:** **Yes** — every function on this page is reached by every standard skirmish FootClass that issues a move order. No SpecialFlags or fog gates.

> Every claim below cites a Ghidra address + decompilation excerpt or `read_memory` byte dump.
> Confidence axes: **C**=content (algorithm verified), **I**=identity (function name verified), **B**=binding (caller path verified).

---

## 1. Overview

PathfinderClass implements A* over a 2D cell grid with **two parallel closed-lists per cell** — one for the ground layer and one for the bridge layer above it. A* expands neighbours, decides per neighbour which layer to enter, and writes that cell's "closed" marker + g-cost into the per-layer array. The same cell can therefore be visited twice in the same search (once via ground, once via bridge) without collision.

The spine consists of four functions in execution order:

```
FootClass::Find_Path (0x4D3920)
   └→ FootClass::Run_AStar (0x4CBBA0)         [thin wrapper, ~50 instr]
         └→ AStar_pathfind_search (0x42C900)  [outer orchestrator, hierarchical retry]
               └→ AStar_main_loop (0x429A90)  [per-step expansion, ~2.4 KB]
                     ├→ AStar_compute_edge_cost (0x429830)
                     ├→ PathfinderClass::UpdateBridgePassability (0x42ACF0)
                     ├→ AStar_create_node (0x42A460)
                     ├→ AStar_reconstruct_path (0x42AA90)
                     ├→ Path_smooth_corners (0x42B210)
                     └→ Path_optimize_straight_segments (0x42B7F0)
```

Bridge-related responsibilities, by function:

| Function | Bridge role |
|----------|-------------|
| `AStar_main_loop` | Source/dest height selection, per-neighbour layer decision, dual closed-list r/w, calls `UpdateBridgePassability` before+after expansion |
| `PathfinderClass::UpdateBridgePassability` | XOR-toggles cell flag `0x40000` (BridgeApproach) on cells around peers' planned paths to discourage collision |
| `AStar_pathfind_search` | Hierarchical retry loop, calls `Zone_precheck` for reachability gate, snaps coords to bridge endpoints via `MapClass::ResolvePathCoord_BridgeAware` |
| `FootClass::Find_Path` | Pre-A* setup: dest cell `Can_Enter_Cell` probe, fallback to nearby passable cell, post-A* path stuffing into FootClass.path_queue |

---

## 2. PathfinderClass struct layout (verified from raw assembly)

PathfinderClass is passed as `__thiscall` (`this` in ECX → `param_1`). All offsets below are **direct byte offsets** (`int param_1`, NOT `int *`), so the CLAUDE.md `param_1` arithmetic trap does not apply here.

| Offset | Type | Purpose | Evidence |
|--------|------|---------|----------|
| `+0x08` | u8 | Byte flag passed to `Can_Enter_Cell` as `param_5` (likely "include crushable as walkable") | `0x429F37: MOV AL,byte ptr [ESI + 0x8]` |
| `+0x14` | ptr | Min-heap descriptor for A* open list. Layout: `{count, capacity, array, hi_water_ptr, lo_water_ptr}` (5 ints, 0x14 bytes) | `0x42A052: MOV EDI,dword ptr [ESI + 0x14]; MOV ECX,dword ptr [EDI]` and heap manipulation through EDI+0..+0x10 |
| `+0x18` | ptr | **GROUND closed-list marker array** (one u32 per cell, holds epoch value when closed) | `0x429C57..0x429C5D` writes [ESI+0x18][idx] = [ESI+0x28]; `0x429FFB..0x42A001` reads same; `0x42A12D` ground-write |
| `+0x1C` | ptr | **BRIDGE closed-list marker array** | `0x429C42..0x429C48` writes [ESI+0x1C][idx] = [ESI+0x28]; `0x42A00D..0x42A013` reads same; `0x42A13F` bridge-write |
| `+0x20` | ptr | **BRIDGE f-cost array** (float per cell) | `0x429F1B: MOV EAX,dword ptr [ESI + 0x20]; FLD float ptr [EAX + EBP*0x1]` |
| `+0x24` | ptr | **GROUND f-cost array** (float per cell) | `0x429EE6: MOV EAX,dword ptr [ESI + 0x24]; FLD float ptr [EAX + EBP*0x1]` |
| `+0x28` | u32 | **Current-search epoch.** Incremented per A* search; stored into the closed marker arrays so stale closes from previous searches are auto-invalidated without zeroing. | `0x429C45: MOV ECX,dword ptr [ESI + 0x28]; MOV [marker_array+idx],ECX` (write epoch on close) |
| `+0x2C` | u32 | Copied from `TechnoType+0x67C` at init — likely speed-cat or movement-cat for cost lookup | `0x429BB7-0x429BC6: CALL vtable+0x84 (→TechnoType); MOV EAX,[EAX+0x67C]; MOV [ESI+0x2c],EAX` |
| `+0x30` | i32 | **Current source height** (height level + 4 if on bridge). Mutates as A* expands. | `0x429B23: MOV [ESI+0x30],EAX` (initial set); reread at every neighbour expansion |
| `+0x34` | i32 | **Destination height** (cell.Level + 4 if dest is on a bridge cell AND locomotor != 2) | `0x429B57: MOV [ESI+0x34],EAX` |
| `+0x3C` | u32 | **Bridge-collision avoidance mode**. 0=off, 1=normal, 2=urgent. Read in two places: gates the `UpdateBridgePassability` calls (`if (+0x3C != 0)`) and gates the moving-friendly cost prediction in `compute_edge_cost`. Value 2 forces the 1000.0 reroute multiplier. | `0x429C13: MOV EAX,dword ptr [ESI + 0x3c]; TEST EAX,EAX; JZ skip` (twice); plus `[ESI+0x3c]` read in compute_edge_cost |
| `+0x40` | ptr | Level-0 zone-on-precheck-path marker array. Set by Zone_precheck; read by main A* to prune cells whose zone wasn't on the Dijkstra path. | `0x429BC3: MOV EDX,[ESI+0x40]; ...; 0x429EA4: CMP [EDX+ECX*4],EAX` |
| `+0x44` | ptr | Level-1 zone-marker array (precheck only) | `Zone_precheck` `param_1+0x40+local_38*4` with `local_38==1` |
| `+0x48` | ptr | Level-2 zone-marker array (precheck only) | same, `local_38==2` |
| `+0x4C..+0x54` | 3×ptr | Per-level zone CLOSED marker arrays (Dijkstra-visited) | `Zone_precheck` `param_1+0x4c+local_38*4` |
| `+0x58..+0x60` | 3×ptr | Per-level zone f-cost arrays | `Zone_precheck` `param_1+0x58+local_38*4` |
| `+0x64` | ptr | Dijkstra node pool (16-byte entries: parent_idx, zone_id, f-cost, depth) | `Zone_precheck` `*(int*)(param_1 + 100)` (100 = 0x64) |
| `+0x68` | ptr | Min-heap descriptor for Dijkstra (same 5-int layout as +0x14) | `Zone_precheck` `*(int**)(param_1 + 0x68)` |
| `+0x6C` | u32 | Current index into pathfinder.zone_chain (the buffer at +0xBE) tracking which zone the A* expansion has currently entered | `0x429CCD-0x429D2B`: `[ESI+0x6c]` increments when reaching a chain zone |
| `+0x70` | u32 | Snapshot of start cell coord (set once at A* entry) | `0x429BD4: MOV ECX,dword ptr [EBP]; MOV [ESI+0x70],ECX` |
| `+0x74..` | 3 × 0x18-byte slot | 3 zone-graph adjacency handles (one per level). Iterated via `piVar6 += 6` (24 bytes) in `AStar_pathfind_search` Reset loop. | `AStar_pathfind_search 0x42C900` near `piVar6 = (int *)(param_1 + 0x74); iVar7 = 3; do { (**(piVar6+0xc))(); piVar6 += 6; } while (iVar7--);` |
| `+0x84+lvl*0x18` | u32 | Bridge-edge exclusion count, per zone-level | `Zone_precheck` `*(int *)(iVar20 + 0x84 + param_1)` where `iVar20 = local_38 * 0x18` |
| `+0x78+lvl*0x18` | ptr | Bridge-edge exclusion array, per zone-level (packed `hi<<16 \| lo` zone pairs) | `Zone_precheck` `*(int *)(iVar20 + 0x78 + param_1)` |
| `+0xBE` | u16[N] | Zone-chain buffer — sequence of zone IDs the path is expected to cross at level 0. Filled by `Zone_precheck`. A* uses this to advance `+0x6C` as it crosses zones. | `0x429CCD: MOV CX,word ptr [ESI + EAX*0x2 + 0xbe]` |
| `+0xBC + lvl*1000` | u16 | First zone ID of path at level lvl (per Zone_precheck) | `Zone_precheck` `*(short *)(param_1 + 0xbc + local_38 * 1000)` |
| `+0xC74 + lvl*4` | u32 | Path zone count per level | `Zone_precheck` `*(undefined4 *)(param_1 + 0xc74 + local_38 * 4) = 1;` |

**Confidence:** C=HIGH (raw assembly verified each offset), I=HIGH (struct semantics derived from usage patterns, not name labels), B=HIGH (single PathfinderClass singleton, called via `MapClass::Pathfinder` chain — confirmed by `get_function_callers` showing each spine function has exactly the expected caller).

### 2.1 Critical decompiler artifact — closed-list pairing

**Ghidra's decompiler produced incorrect output** for the init code at 0x429C3E. It showed both branches writing to `+0x20`:

```c
// Decompiler output (INCORRECT)
if (cell.height < pathfinder.+0x30) {
  *(undefined4 *)(*(int *)(param_1 + 0x1c) + iVar12 * 4) = epoch;
  *(undefined4 *)(*(int *)(param_1 + 0x20) + iVar12 * 4) = 0;
} else {
  *(undefined4 *)(*(int *)(param_1 + 0x18) + iVar12 * 4) = epoch;
  *(undefined4 *)(*(int *)(param_1 + 0x20) + iVar12 * 4) = 0;  // ← should be +0x24
}
```

Raw assembly at `0x429C3B..0x429C63` proves the GROUND branch writes to **+0x24**, not +0x20:

```asm
00429c40: JLE 0x00429c57           ; if cell.height >= start_height → GROUND branch
00429c42: MOV EDX, [ESI + 0x1c]    ; --- BRIDGE branch ---
00429c45: MOV ECX, [ESI + 0x28]    ; epoch
00429c48: MOV [EDX+EAX*4], ECX     ; BRIDGE marker write
00429c4b: MOV EDX, [ESI + 0x20]
00429c4e: MOV [EDX+EAX*4], 0       ; BRIDGE f-cost zero
00429c55: JMP 0x00429c6a
00429c57: MOV ECX, [ESI + 0x18]    ; --- GROUND branch ---
00429c5a: MOV EDX, [ESI + 0x28]    ; epoch
00429c5d: MOV [ECX+EAX*4], EDX     ; GROUND marker write
00429c60: MOV ECX, [ESI + 0x24]    ; ← +0x24 (NOT +0x20 as decompiler claimed)
00429c63: MOV [ECX+EAX*4], 0       ; GROUND f-cost zero
```

Therefore the correct, **binary-verified** pairing is:

| Layer | Closed marker | f-cost array |
|-------|---------------|--------------|
| **GROUND** | `pathfinder+0x18` | `pathfinder+0x24` |
| **BRIDGE** | `pathfinder+0x1C` | `pathfinder+0x20` |

This is corroborated by the lookup code at `0x429EE2..0x429F1E` and the closure-write at `0x42A12D..0x42A156`, both of which load the **same pairings** consistently. Anyone reading the decompilation alone would have inverted GROUND vs BRIDGE — the assembly is authoritative.

---

## 3. Source/destination height resolution (start of every A* search)

`AStar_main_loop` opens with two near-identical blocks computing the "height" the search must reach at start cell and dest cell. The exact assembly form is below.

### 3.1 Destination height (lands in pathfinder.+0x34)

```asm
00429af1: MOV EBX,[ESP+0x68]        ; FootClass*
00429af5: MOV ECX,EBX
00429af7: MOV EAX,[EBX]
00429af9: CALL [EAX + 0x2c]          ; vtable+0x2C — returns locomotor kind
00429afc: CMP EAX,0x2                ; is kind == 2 ?
00429aff: MOV EAX,[ESP+0x30]         ; EAX = dest cell ptr
00429b03: JZ 0x00429b1c              ; if kind==2, take "ground only" branch
00429b05: MOV ECX,[EAX + 0x140]      ; cell.flags
00429b0b: TEST CH,0x1                ; flags & 0x100 ? (CH is bits 8-15, 0x1 there = bit 0x100)
00429b0e: JZ 0x00429b1c              ; not bridge cell → "ground"
00429b10: MOVSX EAX,byte ptr [EAX+0x11b] ; cell.Level (signed)
00429b17: ADD EAX,0x4                ; height = Level + 4 (BRIDGE BUMP)
00429b1a: JMP 0x00429b23
00429b1c: MOVSX EAX,byte ptr [EAX+0x11b]
00429b23: MOV [ESI+0x34],EAX
```

**Verified rule** (destination height): `dest_height = cell.Level + (cell.flags & 0x100 && locomotor != 2 ? 4 : 0)`.

vtable+0x2C is the **locomotor kind selector**. Empirically kind values:
- `1` = infantry
- `2` = something that ignores bridges (probably aircraft / fly — confirms by main loop also using kind==1 for the infantry-special path in `bVar9`)
- `0xF` = Drive / Ship / Hover (returned by `iVar7 = vtable+0x2c` in AStar_pathfind_search and compared to `0xF` to gate the Path_walk_directions_to_cell adjustment)

**Active in YR:** Yes. Every path call goes through this. Locomotor kind 2 (likely Fly) being treated as "ignores bridges" matches the player-observable rule that aircraft fly over bridges without caring about layer.

### 3.2 Source height (lands in pathfinder.+0x30)

Mirror block at `0x429B26..0x429B57`:

```asm
00429b2a: CALL [EDX + 0x2c]          ; vtable+0x2C again
00429b2d: CMP EAX,0x2
00429b30: JZ 0x00429b4c
00429b32: MOV AL,byte ptr [EBX+0x8c]  ; FootClass+0x8C → on_bridge byte
00429b38: TEST AL,AL
00429b3a: JZ 0x00429b4c
00429b3c: MOV EAX,[ESP+0x60]         ; EAX = start cell ptr
00429b40: MOVSX EAX,byte ptr [EAX+0x11b]
00429b47: ADD EAX,0x4                ; start_height = Level + 4
00429b4a: JMP 0x00429b57
00429b4c: MOV ECX,[ESP+0x60]
00429b50: MOVSX EAX,byte ptr [ECX+0x11b]
00429b57: MOV [ESI+0x30],EAX
```

**Verified rule** (source height): `start_height = start_cell.Level + (FootClass.on_bridge && locomotor != 2 ? 4 : 0)`.

The destination uses **cell flag 0x100** (cell is structurally a bridge cell); the source uses **FootClass+0x8C** (this unit IS currently on the bridge). These are **different signals** — important distinction. Asymmetry: dest is layer-determined by cell metadata, source by the unit's own state.

### 3.3 Force-on-bridge override (height-diff ≥ 3 case)

After computing source height, an additional **override** kicks in if the unit's actual Z coordinate disagrees with the cell-derived height by 3 or more height-units. Assembly at `0x429B5A..0x429BB3`:

```asm
00429b5e: CALL [EDX + 0x84]            ; vtable+0x84 → TechnoType*
00429b64: MOV CL,byte ptr [EAX+0xc94]  ; TechnoType+0xC94 (looks like "occupies-bridge-layer" flag)
00429b6a: TEST CL,CL
00429b6c: JZ 0x00429bb3                ; not set → skip override
00429b6e: MOV EAX,[ESP+0x60]           ; start cell
00429b72: MOV ECX,[EAX+0x140]          ; cell.flags
00429b78: TEST CH,0x1                  ; flags & 0x100 (on bridge cell)
00429b7b: JZ 0x00429bb3
00429b94: MOV EAX,[ECX+0x8]             ; FootClass+0xA4 = Z lepton coord
00429b97: MOV ECX,[ESI+0x30]            ; start_height
00429b9a: CDQ
00429b9b: IDIV [0x0089c2d8]             ; EAX = Z / LEPTONS_PER_HEIGHT_LEVEL
00429ba1: SUB EAX,ECX                   ; signed: z_in_height_units - start_height
00429ba3: CDQ
00429ba4: XOR EAX,EDX                   ; abs
00429ba6: SUB EAX,EDX
00429ba8: CMP EAX,0x2                   ; abs > 2 ?
00429bab: JLE 0x00429bb3                ; if abs <= 2 → skip
00429bad: ADD ECX,0x4                   ; force-on-bridge: start_height += 4
00429bb0: MOV [ESI+0x30],ECX
```

**Verified rule:** if `TechnoType.+0xC94 != 0` AND `start_cell.flags & 0x100` AND `abs(unit.Z / LEPTONS_PER_HEIGHT - start_height) > 2` (i.e. ≥ 3), then bump `start_height += 4`. This handles the case where the unit is visually well above the cell's ground level (e.g. mid-traverse on the bridge deck) but `FootClass+0x8C` wasn't set.

The constant `DAT_0089c2d8` is a **runtime-initialized BSS** global (read as 0 from a cold dump — it gets set during MapClass init from rules/INI). Confidence: C=HIGH (math verified), I=MEDIUM (no symbol for the constant; named by purpose), B=HIGH (single divisor in this one site for this purpose).

---

## 4. Per-neighbour layer decision (the **height-diff ≥ 2** gate)

After choosing source/dest heights, the main loop expands 9 neighbours per node (8 cardinal/diagonal + 1 "tube" slot). For each neighbour, it must decide whether to write into the GROUND closed list or the BRIDGE closed list.

Assembly at `0x429E54..0x429E7A`:

```asm
00429e54: MOV EAX,[EBX + 0x140]      ; neighbour.flags
00429e5a: TEST AH,0x1                ; flags & 0x100 ? (CH bit 0)
00429e5d: JZ 0x00429e7a              ; not bridge → ground layer
00429e5f: MOVSX EDX,byte ptr [EBX+0x11b]   ; neighbour.Level
00429e66: MOV EAX,[ESI+0x30]          ; pathfinder.start_height (== current source height after override)
00429e69: MOV byte ptr [ESP+0x60],0x0 ; default: BRIDGE layer flag
00429e6e: SUB EAX,EDX                 ; start_height - neighbour.Level
00429e70: CDQ
00429e71: XOR EAX,EDX                 ; abs
00429e73: SUB EAX,EDX
00429e75: CMP EAX,0x1                 ; abs > 1 ?  (i.e. ≥ 2)
00429e78: JG 0x00429e7f              ; → keep BRIDGE
00429e7a: MOV byte ptr [ESP+0x60],0x1 ; → GROUND layer flag
```

**Verified rule** (`(char)param_2` in the decompilation):
```
GROUND layer  ⇔  (neighbour.flags & 0x100) == 0  OR  abs(start_height − neighbour.Level) < 2
BRIDGE layer  ⇔  (neighbour.flags & 0x100) != 0  AND abs(start_height − neighbour.Level) ≥ 2
```

This is the **central layer-decision gate** for A* expansion. Polarity: the byte at `[ESP+0x60]` is **1 = ground**, **0 = bridge**.

### 4.1 Four different height-difference thresholds (parity trap)

The codebase uses **four distinct thresholds** for "is this a bridge interaction?":

| Site | Test | Symmetric? | Meaning |
|------|------|------------|---------|
| Source-height override (§3.3) | `abs(z_units − start_height) > 2` (≥ 3) | yes | Force `start_height += 4` if unit visibly on the deck |
| Neighbour layer-decision (§4) | `abs(start_height − cell.Level) ≥ 2` | yes | Use BRIDGE closed-list when entering high cell |
| Blocker prediction in compute_edge_cost | `prev.Level − cur.Level < 3` | **no** | Stay on ground when descending by < 3 |
| UpdateBridgePassability layer choice | `abs(unit_cell.Level − cell.Level) < 4  AND  !on_bridge` | yes (and joined with on_bridge byte) | Walk ground list if locally similar height |

**These are NOT consistent.** Implementers must match each gate exactly. The asymmetric `< 3` in blocker prediction (compute_edge_cost) is particularly easy to miss — it only fires when descending, not when ascending. Documented in the companion costs report.

---

## 5. UpdateBridgePassability (0x42ACF0) — the `0x40000` toggle

This function is called **twice** in `AStar_main_loop`: once before the search and once after, guarded by `if (pathfinder.+0x3C != 0)`. A call can **XOR-toggle** bit `0x40000` of `CellClass+0x140` on peer-path or 5x5 bridge-record cells, but normal/no-peer mode can clear `PathfinderClass+0x3C` and return without toggling. When the two guarded calls both perform the same toggle set, XORs cancel — so net effect is "mark cells temporarily, then unmark" — except the marking happens during the A* search itself.

### 5.1 Phase 1 — mark other moving units' planned paths

```c
// Decompilation (sanitized — Ghidra mislabels param convention)
char on_bridge_flag = (FootClass.+0x8C);
CoordStruct unit_cell_coord = FootClass.vtable+0x1B8();   // 0x1B8 = Get_Coord
CellClass *unit_cell = MapClass.Get_CellClass(unit_cell_coord);

// PSEUDO-RANDOM neighbour selection (NOT direction-of-travel):
int rand_dir = (RateTimer__Current() >> 0xc + 1) >> 1) & 7;
CellClass *probe_cell = unit_cell + DirectionOffsets[rand_dir];

// Layer-decision for which occupancy list to walk:
int *obj_list;
if (!(probe_cell.flags & 0x100) ||
    (abs(unit_cell.Level − probe_cell.Level) < 4 && FootClass.+0x8C == 0))
  obj_list = probe_cell.+0xE4;   // ground occupancy list head
else
  obj_list = probe_cell.+0xE8;   // bridge occupancy list head

// If the chosen list is empty, FUN_0042b080 creates/looks-up an alt-object list
// at the appropriate height level (Level + 4 for bridge, Level for ground):
if (obj_list == 0)
  obj_list = FUN_0042b080(probe_cell + 0x24,
                          probe_cell.Level + (bridge_chosen ? 4 : 0));

// Walk the list — for each object that is kind 1 (FootClass/Unit) or 0xF (drive-like):
for each obj in obj_list:
  if (kind == 1 || kind == 0xF):
    // Walk obj's planned path (path_queue stored at obj.+0x178 / 0x179 / 0x17A which are *next* directions; up to 24 steps)
    // For each cell in the path, XOR cell.flags with 0x40000 — but only flip
    // when the bit changes meaning relative to a "source" cell.

    XOR write at 0x42AFA0:
      *cell.flags ^= ((~src.flags ^ cell.flags) & 0x40000)
    // (effectively: cell.flags = (cell.flags & ~0x40000) | (~src.flags & 0x40000))
    // — turn it ON or leave it OFF based on the source pattern.
```

> **2026-05-22 correction:** the source-pattern comments above are stale. A
> verify-doc audit of `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`,
> parent spot-checked with `disassemble_function 0x0042ACF0`, showed both
> `Get_CellClass` calls in the masked write resolve the same updated coordinate.
> Effective behavior for `0x40000` is `dest.flags ^= 0x40000`, not an
> alternating inverse-of-source marker pattern. The scanned peer path starts at
> `path[0]` for both kind `1` and kind `0xF`; kind `1` requires `path[0]` and
> `path[1]`, while kind `0xF` requires `path[0]`, `path[1]`, and `path[2]`.

### 5.2 Phase 2 — 5×5 area XOR around the probe cell

If the obj_list scan succeeded (`bVar14`) OR `pathfinder.+0x3C` is in mode 2 (urgent), execute the 5×5 area toggle (LAB_0042afcb):

```c
for (dy = -2; dy <= 2; dy++) {
  for (dx = -2; dx <= 2; dx++) {
    CellClass *c = Get_CellClass(probe_cell.coord + (dx, dy));
    if (c.+0x124 != 0) {                  // c.+0x124 = bridge-record index byte (bit set if cell participates in a BridgeRecord)
      if (c.coord == unit_cell.coord)
        continue;                          // never toggle the unit's own cell
      c.flags ^= 0x40000;                  // TOGGLE the BridgeApproach bit
    }
  }
}
// Finally toggle the probe_cell itself:
probe_cell.flags ^= 0x40000;
```

`cell.+0x124` is the **bridge-record-index byte**. Only cells that participate in a BridgeRecord get the toggle. The own cell is skipped to avoid corrupting the unit's footprint.

### 5.3 Semantics of bit `0x40000` (BridgeApproach)

In `compute_edge_cost`:
```c
if (cell.flags & 0x40000) param_5 *= 4.0;   // g_BridgeApproach_CostMult_4_0 at 0x7E37BC
```

So when bit `0x40000` is set on a cell, A* assigns it a **4× cost multiplier**. The XOR design means:
- Peer-path cells and/or neighbouring bridge-record cells can be marked ×4 cost while THIS A* search runs.
- The second guarded call can XOR the same set again, clearing the temporary marks.
- If no peer path is processed in normal mode, the helper can clear `PathfinderClass+0x3C` and return without toggling, so "every call toggles" is not correct.

**Player-observable effect:** units pathing through a bridge area near OTHER moving units take 4× cost penalty on cells those other units are about to use → A* finds a way around them. Reduces "stuck in queue on bridge" behaviour.

**Note** the use of `RateTimer__Current` for direction randomization. This is `g_FrameCounter`-derived pseudo-random, so it is **deterministic within a fixed-seed game** but not directionally meaningful — same unit at same tick always picks the same probe direction. The randomness exists to break ties between near-identical pathing situations and to avoid pathological alignment.

### 5.4 Caller binding

`get_function_callers(0x42ACF0)` returns exactly: **`AStar_main_loop @ 0x429a90`**. Both `JZ 0x42acf0` call sites at `0x429C1A` and `0x42A42D` are inside the main loop. Binding confidence: **HIGH**.

**Active in YR:** Conditional. Gated by `pathfinder.+0x3C != 0`, which is set by `AStar_pathfind_search` at `0x42C92F` from the last search argument. The nonzero caller cases are live, but this report did not establish that every collision-concern skirmish path defaults nonzero.

---

## 6. Main expansion loop iteration limits

`AStar_main_loop` has two hard ceilings:

| Limit | Value | Evidence | Behaviour |
|-------|-------|----------|-----------|
| `local_34` overall iteration cap | 10000 | `0x42A3E2: CMP ECX,0x2710` (= 10000); on equal → take FAIL path | Hard A* abort, returns 0 |
| `param_6` "max_steps" passed by caller | clamped to **0xFFF7** (65527) if negative | `0x429D57: JGE 0x00429D61; 0x429D59: MOV [ESP+0x70],0xfff7` | Per-call budget; on hit, returns the partial path if length > 2, else 0 |

If `local_34 == 10000` OR `local_34 == param_6` OR open list empty (`piStack_48 == 0`) — return condition: `if (path_length > 1) return reconstructed_path; else return 0`.

**Critical detail**: the 10000 hard cap is **not** the same as the `param_6` caller budget. The caller-supplied budget is checked first; the hard cap is a backstop against pathological situations.

---

## 7. End-of-search "close enough" early termination

If the expansion reaches the destination **cell** but `Can_Enter_Cell` for the dest returned a code ≥ 7 (Impassable), the search would normally throw the path away. There's an escape clause at `0x42A17D..0x42A19B`:

```asm
0042a17d: CMP EDI,[ESP+0x38]       ; is this the dest cell?
0042a181: JNZ 0x0042a1a1
0042a183: MOV AL,byte ptr [ESP+0x13]  ; bVar9 (special infantry flag)
0042a187: TEST AL,AL
0042a189: JNZ 0x0042a1a1           ; if bVar9 set, don't take shortcut
0042a18b: MOV EAX,[ESI+0x30]        ; current source height
0042a18e: MOV EBP,[ESI+0x34]        ; dest height
0042a191: SUB EAX,EBP
0042a193: CDQ
0042a194: XOR EAX,EDX               ; abs
0042a196: SUB EAX,EDX
0042a198: CMP EAX,0x1
0042a19b: JLE 0x0042a3de            ; if abs <= 1, jump to "path success"
```

**Verified rule**: if reach is `dest_cell` AND `bVar9 == false` AND `abs(start_height − dest_height) ≤ 1`, accept the path even though dest cell is blocked.

`bVar9` (set at `0x429D2B..0x429D4C`) = `locomotor_kind == 1 AND TechnoType.+0xE0C != 0`. So this shortcut is **disabled for a special class of infantry** (likely garrisonable / engineerable infantry that REALLY can't stand on the dest cell). For all other units, "we got within 1 height of the dest" counts as success.

**Player-observable consequence:** vehicles pathing to a cell blocked by a building will stop adjacent to it (because adjacent cells share the same height ±0 or 1), reporting the path as successful, then the higher-level command logic decides what to do. Same behaviour holds for stationary friendly units (return code 6) being treated as soft-blockers; A* will stop just short instead of failing.

---

## 8. AStar_pathfind_search (0x42C900) — the hierarchical retry orchestrator

Top-level wrapper that handles:
- Coord snap to bridge endpoints via `MapClass::ResolvePathCoord_BridgeAware (0x583180)` for both start and dest
- `Zone_precheck` gate — bail if hierarchy says unreachable
- Up to **5 retries** when same-zone search fails
- Calls `PathfinderClass::UpdateHierarchicalEdges (0x42CCD0)` and `Reset` between retries

Key behavioural details:

### 8.1 Retry budget

```c
iStack_14 = (-(uint)(param_6 != -1) & 0xfffffffc) + 5;   // = 1 if param_6 != -1, else 5
```

`param_6` is the per-call iteration budget; if not -1, allow 1 hierarchical try; if -1, allow up to 5. The retry loop calls `PathfinderClass::UpdateHierarchicalEdges @ 0x0042CCD0` and `Reset` between attempts. That helper flood-fills reachable zones and appends/invalidates zone-edge exclusions; it is not a global zone-graph rebuild.

### 8.2 Same-zone case

If `iStack_14 == iVar3` (start_zone == end_zone), runs Zone_precheck only as a sanity check; on failure logs "Hierarchical findpath failure" but **continues to main loop with `param_8` cleared** (no hierarchical hint). On different-zone failure, returns 0 immediately.

The split: same-zone always tries A* even without precheck data; cross-zone REQUIRES precheck to succeed.

### 8.3 Hierarchical assist flag (`param_8`)

This bool drives whether main A* will use the zone-on-path pruning (`pathfinder.+0x40` reads). The loop toggles it across retries — first try with hierarchy, retry without if same-zone failure, etc.

### 8.4 Movement-zone discovery (`param_7 == 0xFFFFFFFF`)

If caller passes `param_7 == 0xFFFFFFFF`, the function fetches it from `TechnoType.+0x5B4` via `vtable+0x84`. **`TechnoType.+0x5B4` = MovementZone enum value** (verified by `AStar_pathfind_search`'s three identical `vtable+0x84; *(uint *)(iVar3 + 0x5B4)` patterns).

### 8.5 Locomotor-kind 0xF + `vtable+0x4C` (Drive-class hook)

```c
iVar7 = vtable+0x2c();   // locomotor kind
if (iVar7 == 0xf && TechnoType.+0xD94 != 0) {
  // Query DAT_00818858 (COM interface GUID for ILocomotionExtension?) via QueryInterface
  // If success, call vtable+0xC on the queried interface
  // Then Release
}
```

This is a `QueryInterface` call on the locomotor (likely `ILocomotion::IUnknown::QueryInterface`). The GUID at `0x00818858` returns an extension that's queried only for kind-0xF (Drive/Hover/Ship) with `TechnoType+0xD94` set. The `vtable+0xC` call on the result appears to perform some path-context refresh.

`TechnoType+0xD94` — unknown semantically; **possible TS-legacy** since the kind+0xF check is so narrow. Not investigated further in this phase; flagged.

---

## 9. FootClass::Find_Path (0x4D3920) and Run_AStar (0x4CBBA0)

### 9.1 Find_Path responsibilities

- Calls `vtable+0x2CC` (pre-pathfind validation) — if false, mark path slot invalid (`FootClass.+0x178 = -1`) and return 0.
- Computes target zone via `vtable+0x84 → TechnoType+0x5B4` (MovementZone).
- Calls `vtable+0x1AC` (Can_Enter_Cell) on dest cell. Branches on return code:
  - **Code 6 (FriendlyStationary)**: not TooBig (TechnoType+0xC94 == 0) → call `FootClass::Find_Nearby_Passable_Cell` and substitute the dest if within tolerance (via `FUN_0042D170` which uses Zone_precheck to estimate true cost). The cost comparison is `cost ≤ direct_distance + 6`.
  - **Code 7 (Impassable)** AND not TooBig: query buildings in cell (`Look_up_building_in_cell`); if any, do the same nearby-cell substitution.
  - Other codes: proceed with original dest.
- Calls `vtable+0x124` (probably Process_Map_Update or similar prepatory hook).
- Calls `FootClass::Run_AStar (0x4CBBA0)` — the actual search.
- **Copies the result path into FootClass.+0x178** (the path_queue, 0x18 bytes = 24 directions max).
- For infantry (kind == 1), walks the `FootClass.+0x6C8` linked-list of other infantry in the same cell and triggers their re-path if their first-cell doesn't match this one's. This is the **group-formation infantry recursive re-path** behaviour.

### 9.2 The 24-step copy-into-path_queue

```asm
; Copy up to MIN(24-current_offset, returned_length) directions
0x4D3? area: copy loop with edge-clamp at 0x18 (= 24)
```

`FootClass.+0x178` is a 24-byte array. The path is stored as direction codes 0-7 plus a `-1` terminator. The copy is from `&stack0x3c` (the location AStar wrote the smoothed path to) into `&FootClass.+0x178 + current_offset`.

### 9.3 Bridge-related post-conditions in Find_Path

After Run_AStar returns, Find_Path inspects:

```c
if ((1 < height_diff) ||
    ((FootClass.+0x8C == 0 && cell.flags & 0x100))) {
  // Path failed AND (we're trying to enter a high cell, OR we're not on bridge but dest is bridge cell)
  // → trigger emergency relocate via vtable+0x480
  // → for non-player-controlled units, fall back to FUN_00500200 (random nearby destination)
}
```

So bridge-layer mismatches trigger an alternate-destination fallback. Confidence C=MEDIUM here — Ghidra's signature recovery for Find_Path is poor (uses `extraout_ECX` and stack-offset variables). The exact polarity may differ slightly.

### 9.4 Run_AStar (0x4CBBA0) — light pass

```c
vtable+0x4C(local_c, 0);              // fetch start coord
if (unaff_retaddr == 0) return 0;
Path_walk_directions_to_cell(param_4, &FootClass.+0x178);   // pre-walk existing path
return AStar_pathfind_search(&stack0xffffffe4, uStack_4, FootClass*, retval, -1, -1, urgency);
```

5-instruction wrapper. The interesting part is `Path_walk_directions_to_cell` — it "consumes" the current path's directions to compute a starting *coord* offset from the unit's true position. This is used so that A* searches from where the unit WILL be after its current sub-step commits, not from current position. Avoids a 1-frame stutter.

`urgency` (param_5) passes through to `pathfinder.+0x3C` — the same value that gates UpdateBridgePassability and the moving-friendly cost prediction.

---

## 10. Heap and node structure

`AStar_create_node (0x42A460)` allocates from a pool indexed by node index (each node = 16 bytes). Returned ptr is added to the heap at `pathfinder.+0x14`. Node layout, verified from heap operations:

| Offset | Field |
|--------|-------|
| `+0x00` | Parent index (back-pointer for path reconstruction) |
| `+0x04` | Stored cell pointer / coord key, plus a float (loaded via `FLD float ptr [EDX+0x4]` in tie-break check at `0x429EE9`) |
| `+0x08` | **f-cost (float)** — the heap key. Compared via `FCOMP float ptr [N+0x8]` everywhere in heap ops. |
| `+0x0C` | Path depth (path length so far). Used by `if (piStack_48[3] > 1)` for "is path long enough to reconstruct" check. |

Min-heap descriptor (5 ints, 0x14 bytes):

| Offset | Field |
|--------|-------|
| `+0x00` | count |
| `+0x04` | capacity |
| `+0x08` | backing array pointer |
| `+0x0C` | hi water mark (max ptr observed) |
| `+0x10` | lo water mark (min ptr observed) |

Heap ops verified via the textbook sift-up at `0x42A052..0x42A0CB` and sift-down at `0x42A2BB..0x42A3AD`.

### 10.1 Tie-break epsilon — 1.009

When checking if existing closed-list entry has a better f-cost than the new candidate:

```asm
00429ee9: FLD float ptr [EDX + 0x4]     ; parent node's [+4] field — interpreted as float
00429eec: FADD double ptr [0x007e37c0]  ; add 1.009 (double)
00429ef2: FLD float ptr [EAX + EBP*0x1] ; existing f-cost
00429ef5: FCOMPP
```

`_DAT_007e37c0` is **8 bytes** at 0x007e37c0: `be 9f 1a 2f dd 24 f0 3f` → IEEE 754 double `0x3FF024DD2F1A9FBE` = **1.009** (≈ 1 + 9/1000).

So the gate to keep existing path is: `existing_fcost < (parent_field + 1.009)`. This is an **additive 1.009 cost-unit tolerance**, not a percentage comparison: an existing closed-list entry can block reopening even when it is slightly worse than the new candidate by less than 1.009 cost units. Tie-break is therefore biased toward "first path found", reducing thrashing when costs are nearly equal.

**Confidence**: C=HIGH (assembly+memory), I=HIGH (Ghidra labels the constant as `_DAT_007e37c0`), B=HIGH (two read sites in `AStar_main_loop`: `0x00429EEC` and `0x00429F21`).

---

## 11. The 8-direction + tube structure of the expansion loop

`iStack_44` (the loop counter) runs **0 to 8** (9 iterations):

- `0..7` = 8 cardinal/diagonal neighbours
- `8` = tube (cell.+0x116 is the tube index; if not -1, fetch the partner cell via `g_TubeArray`)

Direction codes (verified via tables at 0x7E3774 and 0x7E3750):

| Code | Direction | Cell array offset (int units, map_width=512) |
|------|-----------|----------------------------------------------|
| 0 | N | -512 |
| 1 | NE | -511 |
| 2 | E | +1 |
| 3 | SE | +513 |
| 4 | S | +512 |
| 5 | SW | +511 |
| 6 | W | -1 |
| 7 | NW | -513 |
| 8 | (tube) | special — looked up via g_TubeArray |

The 3×3 `(dy*3 + dx) → direction_code` table at `0x7E3750`:

| dy\dx | -1 | 0 | 1 |
|-------|----|----|----|
| **-1** | 7 (NW) | 0 (N) | 1 (NE) |
| **0** | 6 (W) | -1 (self/invalid) | 2 (E) |
| **1** | 5 (SW) | 4 (S) | 3 (SE) |

So directions cycle **clockwise from N**. This matters for the diagonal-bridge cost computation in the companion report.

### 11.1 Tube case (iStack_44 == 8) — special handling

```c
if (iStack_44 == 8) {
  if (cell.tube_index == -1) {
    piVar23 = &DAT_0089c2e0;   // dummy/empty cell
  } else {
    coord = g_TubeArray[cell.tube_index].+0x28;   // tube partner coord
    piVar23 = MapClass.Get_CellClass(coord);
  }
}
```

And the cost for a tube step is **Chebyshev distance** (max of dx, dy) — not the per-direction table cost. From assembly at `0x429FA3..0x429FE6`:

```asm
00429fad: MOVSX ECX,word ptr [EDX+0x2]      ; dest.y
00429fb1: MOVSX EAX,word ptr [EDI+0x26]     ; src.y
00429fb5: ADD EDI,0x24
00429fb8: SUB EAX,ECX                       ; dy
... abs(dy) into ECX, abs(dx) into EAX ...
00429fd2: CMP EAX,ECX
00429fd4: MOV [ESP+0x34],EAX                ; cost = abs(dx)
00429fd8: JG 0x00429fde
00429fda: MOV [ESP+0x34],ECX                ; cost = max(abs(dx), abs(dy))
00429fde: FILD dword ptr [ESP+0x34]          ; int → float
00429fe2: MOV EDI,[ESP+0x24]
00429fe6: FSTP float ptr [ESP+0x34]
```

Tube traversal cost = `max(|dx|, |dy|)` (Chebyshev) as a float. No bridge multiplier applied. Tubes are **not subject to the bridge cost logic**.

### 11.2 Reservation pass — `bVar10` (`TechnoType+0xC94`)

Before the main expansion, if `TechnoType+0xC94 != 0` (the "occupy bridge layer" flag), there's a reservation loop at `0x429C8C..0x429D2B` that pre-marks neighbours in a 3-cell-arc-around-current-heading as **closed**. The arc is determined by a random direction (via `RateTimer__Current`) and includes cells at angular distance 3, 4, 5 from the current heading. This is an **anti-self-collision** mechanic — wide vehicles (Mammoth Tank etc.) won't path through their own footprint.

The arc-determined closures use the same layer-decision rule as main expansion: if a chosen arc cell is `cell.Level + 1 < pathfinder.start_height`, write to BRIDGE marker; else GROUND. Note the **+ 1** in this gate (instead of the usual ≥ 2 abs check) — yet another subtle threshold.

---

## 12. Cross-doc contradictions resolved

### 12.1 Prior audit claim: GROUND uses (+0x18, +0x20) and BRIDGE uses (+0x1C, +0x24)

**Refuted by assembly.** Prior audits inheriting from the Ghidra decompiler's erroneous output had it backwards. Authoritative pairing:
- GROUND: (+0x18 marker, **+0x24** f-cost)
- BRIDGE: (+0x1C marker, **+0x20** f-cost)

### 12.2 Prior audit claim: Zone_precheck contains a TS-legacy branch

**Refuted.** The `iVar22 != 1` block at `LAB_0042c7c5` is just an **inlined min-heap sift-down**. Standard binary heap code, fully reachable in YR. Not TS. Confirmed by tracing: every comparison uses `FCOMP float ptr [N+0x8]` on the node's f-cost field — identical pattern to the main A* heap ops.

### 12.3 The "1.009 epsilon" — previously undocumented

This is an additive 1.009 cost-unit tolerance on closed-list f-cost comparisons. Tiebreaks favour existing paths. Worth documenting for any Rust implementation — using an EXACT-equal comparison instead would produce visible thrashing in tied-cost situations.

---

## 13. Active-in-YR confirmation per function

| Function | Reachable in YR? | Evidence | Gating flags |
|----------|------------------|----------|--------------|
| `FootClass::Find_Path (0x4D3920)` | Yes | `get_function_callers` shows ~14 caller sites including vtable dispatch for every locomotor's Process tick | None — direct call |
| `FootClass::Run_AStar (0x4CBBA0)` | Yes | Caller: `FootClass::Find_Path` only | None |
| `AStar_pathfind_search (0x42C900)` | Yes | Caller: `FootClass::Run_AStar` only | None |
| `AStar_main_loop (0x429A90)` | Yes | Caller: `AStar_pathfind_search` only | None |
| `PathfinderClass::UpdateBridgePassability (0x42ACF0)` | Yes | Caller: `AStar_main_loop` only | Gated by `pathfinder.+0x3C != 0`. Set by callers — non-zero is the default for player units with collision concerns. Always reachable in skirmish. |
| `AStar_compute_edge_cost (0x429830)` | Yes | Caller: `AStar_main_loop` only | None |
| `AStar_create_node (0x42A460)` | Yes | Caller chain through `AStar_main_loop` | None |

None of these functions are TS-only. No `SpecialFlags & 0x1000` checks. The entire spine is live in standard YR skirmish.

---

## 14. Current Rust Implementation Status

**This section maps verified findings to existing Rust code, NOT a port plan.** Cross-reference only.

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| A* main loop | [src/sim/pathfinding/core.rs](../../ra2-rust-game/src/sim/pathfinding/core.rs) | Implemented with layered PathGrid (`ground_walkable`, `bridge_walkable`, `transition`, `height`) — different mechanism than gamemd's dual closed-list, but observable output should match if the layer-decision gate is identical. |
| Source/dest height resolution | [src/sim/movement/movement_path.rs](../../ra2-rust-game/src/sim/movement/movement_path.rs) | `supports_layered_bridge_pathing()` gates by locomotor kind + on_bridge — covers vtable+0x2C kind==2 case and FootClass.+0x8C check. Needs audit against the asymmetric source-vs-dest signal split (§3). |
| Layer-decision gate (height-diff ≥ 2) | [src/sim/pathfinding/core.rs](../../ra2-rust-game/src/sim/pathfinding/core.rs) | Current layered A* implements the closed-list layer gate with `path_height.abs_diff(cell.ground_level) >= 2`; `movement_bridge.rs` is runtime `on_bridge` transition state, not the A* layer-selection implementation. |
| UpdateBridgePassability XOR toggle | none | **Missing.** Rust does not have a per-A*-call temporary "other unit nearby" cost bump. Player-observable effect: groups pathing simultaneously may collide more than in gamemd. |
| 4.0 BridgeApproach multiplier (0x40000 flag) | none | **Missing** (depends on UpdateBridgePassability). |
| 4-retry hierarchical loop in pathfind_search | partial | `src/sim/pathfinding/zone_search.rs` has hierarchical zone Dijkstra but the retry-with-rebuild flow is simpler. |
| 1.009 tie-break epsilon | none | Rust uses exact f64 comparison — equal-cost paths chosen by hash order. Could produce visible variance from gamemd's "prefer existing path" behaviour. |
| Find_Path nearby-cell fallback (code 6/7) | partial | `cell_entry.rs` has fallback paths but the FootClass-level "find passable cell within tolerance" loop is not fully replicated. |
| TechnoType+0xC94 anti-self-collision arc | none | **Missing.** Wide vehicles in Rust may path through their own footprint differently. |

(Severity assessment for parity divergence is intentionally deferred to the Phase 7 synthesis doc.)

---

## 15. Open Questions

1. **TechnoType+0xC94 semantic** — exact INI key it maps to (likely `OccupySize`, `BridgeOnly`, or a similar flag). Needs separate `TechnoTypeClass` field-resolution pass.
2. **TechnoType+0x67C** (copied to `pathfinder.+0x2C`) — semantic unknown. Possibly a speed-cat-by-cell-type index. Needs follow-up.
3. **TechnoType+0xD94** (gates the kind-0xF QueryInterface in AStar_pathfind_search) — unknown.
4. **DAT_0089c2d8** runtime value — read as 0 (BSS). Needs runtime trace or boot-time init function discovery.
5. **The COM GUID at 0x00818858** in AStar_pathfind_search — find the implementor and what `vtable+0xC` does.
6. **`FUN_0042b080`** in UpdateBridgePassability — creates/fetches an alt-object list for a given height. Needs decomp to understand what data structure backs the "alt object list at level N" lookup.
7. **`Path_walk_directions_to_cell`** at line in Run_AStar — what does it consume from the path_queue and how does it affect start coord?
8. **`bVar10` arc-direction encoding** — the `iVar12 = 0..8` arc loop with the angular-distance-from-random check is unusual. Reverse-engineer the exact pattern for parity.

---

## 16. Sources

**Ghidra functions decompiled (with body addresses):**
- `AStar_main_loop` @ 0x00429A90 (body to 0x0042A45C, ~2.4 KB)
- `AStar_pathfind_search` @ 0x0042C900 (body to 0x0042CCCD)
- `PathfinderClass__UpdateBridgePassability` @ 0x0042ACF0 (body to 0x0042B072)
- `Zone_precheck` @ 0x0042C290 (body to 0x0042C8F7) — full coverage in companion doc
- `AStar_compute_edge_cost` @ 0x00429830 — full coverage in companion doc
- `FootClass__Find_Path` @ 0x004D3920 (body to 0x004D41F2)
- `FootClass__Run_AStar` @ 0x004CBBA0 (body to 0x004CBC3D, ~157 bytes)

**Raw assembly examined:**
- AStar_main_loop full disassembly (verified dual closed-list pairing, height-diff gates, heap operations, tie-break epsilon)

**Memory reads:**
- 0x007E37B0..0x007E37CF (cost constants region — see companion doc)
- 0x007E37C0 (tie-break epsilon = 1.009 double)
- 0x007E3750..0x007E3793 (dy*3+dx encoder + 8-dir cell offsets)
- 0x007E3774 (g_CellNeighborOffsets_8Dir verified)

**Callers traced (binding evidence):**
- AStar_main_loop ← AStar_pathfind_search [only]
- AStar_pathfind_search ← FootClass::Run_AStar [only]
- UpdateBridgePassability ← AStar_main_loop [only]
- Find_Path ← (recursive self + various command handlers)

**Callees enumerated:**
- AStar_main_loop → 8 callees (AStar_compute_edge_cost, AStar_create_node, AStar_reconstruct_path, Path_optimize_straight_segments, Path_smooth_corners, UpdateBridgePassability, RateTimer__Current, ZoneMap__CellToZoneIndex)
- UpdateBridgePassability → 4 callees (FUN_0042b080, MapClass::Get_CellClass, MapClass::Is_Cell_In_Playfield, RateTimer__Current)

**Companion docs:**
- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` (this report's costs/precheck pair)
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` (prior doc — partially superseded by this report on dual closed-list specifics)
- `PATHFINDERCLASS_GHIDRA_REPORT.md` (prior doc — struct layout in §2 here supersedes it)
- `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` (Phase 2 dependency for the Can_Enter_Cell return codes referenced in §3.1)
