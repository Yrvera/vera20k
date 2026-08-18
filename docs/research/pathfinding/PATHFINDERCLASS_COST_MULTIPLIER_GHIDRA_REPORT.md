# PathfinderClass +0x04 cost_multiplier — Setter and Source

**Primary addresses:**
- `0x0087e8b8` — PathfinderClass singleton (static global, NOT embedded in MapClass)
- `0x0087e8bc` — cost_multiplier field (+0x04)
- `0x0042a6d0` — real PathfinderClass constructor (writes cost_multiplier = 1.0)
- `0x0040afa0` — CRT static-init thunk that calls the constructor at program startup
- `0x008120a8` — CRT init table entry pointing at the thunk
- `0x00429a90` — AStar_main_loop (reads cost_multiplier, multiplies edge cost)
- `0x0042a900` — PathfinderClass destructor (no write to +0x04)

**Confidence:** HIGH — disassembly + decompilation + xref enumeration all consistent.

**Active in YR:** YES — cost_multiplier is read on every A* edge expansion in normal YR skirmish play.

---

## 1. Short answer

**The cost_multiplier at `PathfinderClass+0x04` is a hardcoded `1.0f`. It is written exactly once at program startup by the PathfinderClass constructor `FUN_0042a6d0` and never modified again for the lifetime of the process.** There is no INI key, no per-locomotor field, no per-MovementZone constant, and no per-search override behind it. The "multiplier" is effectively a structural no-op — every A* edge cost is multiplied by 1.0.

---

## 2. Structural correction to prior research (important)

`PATHFINDERCLASS_GHIDRA_REPORT.md` §1 states PathfinderClass is "embedded within the MapClass singleton at MapClass+0xEC". **This is wrong.** Direct disassembly of every caller of the PathfinderClass methods (constructor, Reset, AStar_pathfind_search, etc.) shows `MOV ECX, 0x87e8b8` immediately before the call — i.e., the `this` pointer is a fixed absolute address `0x0087e8b8`, not `MapClass+0xEC`.

Evidence (xrefs to `0x0087e8b8`, all `MOV ECX,0x87e8b8` followed by a method call):

| Site | Calls | Method |
|------|-------|--------|
| 0x0040afa0 | 0x0042a6d0 | constructor (this report) |
| 0x004cbc2c | 0x0042c900 | AStar_pathfind_search (from FootClass::Run_AStar) |
| 0x004d3c97 | 0x0042d170 | path distance estimator (from Find_Path) |
| 0x004d43fc/4722/4823/48de/48fe/4a59 | 0x0042d170 | from Mission_Patrol (6 sites) |
| 0x004d6799/6845 | 0x0042d170 | from Greatest_Threat_Scan |
| 0x005671a0 | 0x0042ac00 | per-map array resize (from InitZoneMap) |
| 0x0056721a | 0x0042c1c0 | per-map zone array alloc (from InitZoneMap) |
| 0x00504da9 | 0x0042c900 | AStar_pathfind_search (other caller) |

What InitZoneMap (`FUN_00567110`) actually does at `0x005671a0`:

```
00567196: LEA ECX,[ESI + 0xec]    ; ESI = MapClass; this pushes MapClass+0xEC as PARAM_2
0056719c: MOV [ESI + 0x70], EAX
0056719f: PUSH ECX                 ; param_2 = MapClass+0xEC
005671a0: MOV ECX, 0x87e8b8        ; this = PathfinderClass singleton
005671a5: CALL 0x0042ac00          ; per-map resize, NOT the constructor
```

`MapClass+0xEC` is passed as *param_2* to `FUN_0042ac00` because that function reads `*(param_2 + 8)` and `*(param_2 + 0xC)` — which are `MapClass+0xF4` (MapWidth) and `MapClass+0xF8` (MapHeight). So MapClass+0xEC is just a base offset used to read map dimensions; the PathfinderClass object itself lives at `0x0087e8b8`.

`FUN_0042ac00` is the **per-map array resize**, not the C++ constructor. The real constructor is `FUN_0042a6d0`.

---

## 3. The real constructor — `FUN_0042a6d0`

Decompilation snippet (full body in Ghidra):

```c
*param_1 = 0;                                          // +0x00 byte = 0
param_1[1] = 0;                                        // +0x01 bridge_aware = 0
param_1[2] = 0;                                        // +0x02 padding
param_1[3] = 1;                                        // +0x03 byte = 1
*(undefined4 *)(param_1 + 4) = 0x3f800000;             // +0x04 cost_multiplier = 1.0f
param_1[8] = 1;                                        // +0x08 unknown_flag_08 = 1
*(undefined4 *)(param_1 + 0x18) = 0;                   // +0x18 closed_ground_stamp = null
... (allocates heap pools: search_node_pool 0x100004 bytes,
     trail_pool 0x180004 bytes, zone_node_pool 160000 bytes,
     open_set_heap with 0x10000 capacity, zone_precheck_heap with 10000 capacity) ...
```

Disassembly of the key write:

```
0042a6eb: MOV dword ptr [ESI + 0x4], 0x3f800000   ; ESI = this; literal 1.0f
```

`0x3f800000` is the IEEE-754 single-precision encoding of `1.0`.

This is a **literal immediate in the instruction stream**, not loaded from `.rdata` or any configurable table. There is no INI hook, no per-unit field, no MovementZone-indexed array.

---

## 4. CRT static initialization

The constructor is invoked once at program startup via the CRT init table:

- `0x008120a8` contains the function pointer `0x0040afa0` (a Ghidra-confirmed DATA xref to 0x0040afa0)
- `0x0040afa0` is a thunk:
  ```
  0040afa0: MOV ECX, 0x87e8b8         ; this = PathfinderClass singleton
  0040afa5: CALL 0x0042a6d0           ; real constructor — writes cost_multiplier=1.0
  0040afaa: PUSH 0x40afc0             ; destructor thunk address
  0040afaf: CALL 0x007c978a           ; atexit() (CRT exit-handler registration)
  0040afb4: POP ECX
  0040afb5: RET
  ```
- `0x0040afc0` is the matching destructor thunk that tail-calls `0x0042a900` (frees pools).

This is the classic MSVC static-storage C++ object pattern. The singleton is constructed before `main()` and lives for the lifetime of the process.

`FUN_0042ac00` at `0x005671a0` (per-map InitZoneMap call) does NOT touch +0x04 — it only allocates/resizes the closed-set and g-cost arrays at +0x18..+0x24 sized to `(MapWidth + 1 + MapHeight)²`. The cost_multiplier persists from initial startup across all maps in the same process.

---

## 5. Read site — AStar_main_loop @ 0x00429a90

```c
fStack_28 = (float)(
    fVar25 * (float10)*(float *)(param_1 + 4)             // edge_cost × cost_multiplier
    + (float10)*(float *)(&DAT_0081872c + iStack_44 * 4)  // + direction tiebreaker
);
```

Where:
- `fVar25` = return of `AStar_compute_edge_cost(...)` — looks up base cost in `DAT_0081870C[Can_Enter_Cell return code]` (table from `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` §3)
- `param_1 + 4` reads PathfinderClass+0x04 = **1.0f**
- `DAT_0081872c[dir]` = small direction-tiebreaker epsilon (~0.001..0.008)

Effective formula: **`fStack_28 = edge_cost + tiebreaker[dir]`** — the cost_multiplier contributes nothing observable.

---

## 6. Verification — no other writer exists

Exhaustive enumeration of writes:

- All 24 xrefs to `0x0087e8b8` (PathfinderClass+0) were enumerated and disassembled; only `0x0040afa0` reaches `FUN_0042a6d0` (the constructor). All others reach Reset / AStar / per-map-resize / destructor methods.
- `get_xrefs_to 0x0087e8bc` (the +0x04 field itself) returned: **No references found**. The field is never accessed by absolute address; only via `this+4` inside the constructor and the AStar_main_loop reader.
- Decompilation reviewed for: `FUN_0042ac00` (per-map resize), `PathfinderClass__Reset` (0x0042a5b0), `AStar_pathfind_search` (0x0042c900), `AStar_main_loop` (0x00429a90), `FUN_0042c1c0` (zone alloc), `FUN_0042d170` (path estimator), `FUN_0042a900` (destructor), `FootClass::Find_Path` (0x004d3920), `FootClass::Run_AStar` (0x004cbba0). **None of them writes to +0x04.**
- The value `0x3f800000` (1.0f) appears as an immediate in `0x0042a6eb` only; no other write site found in any pathfinding-related function.

---

## 7. Open Questions

1. **Was this field intended to be modifiable in TS?** The C++ constructor explicitly initializes it to 1.0 (vs leaving it BSS-zero), which suggests the designers anticipated future writes. None ever materialized in YR. Could be TS-legacy dead infrastructure (the field exists, the slot is initialized, but no system was ever wired up to mutate it).
2. **Significance of paired +0x03 = 1 and +0x08 = 1?** The constructor writes byte 1 to both. These are not part of this slot's scope but were observed in the same write sequence; flagged here so the next investigator doesn't re-research them.
3. **Why does `FUN_0042ac00` exist if `FUN_0042a6d0` already allocates the arrays?** The per-map function `FUN_0042ac00` reallocates +0x18..+0x24 sized to the current map; the static constructor leaves those nulled (`MOV dword ptr [ESI + 0x18], EBX` where `EBX=0`). So map load uses `FUN_0042ac00` to size them. The flow is: startup constructor → per-map InitZoneMap resize → first A* search. Out of scope for this report.

---

## 8. Implications for the Rust port

The Rust implementation can **omit the cost_multiplier field entirely**. The original game's cost_multiplier is a structural placeholder set to 1.0 and never changed — multiplying every edge cost by 1.0 is equivalent to not multiplying at all. Per the parity bar (observable outputs, not internals), the Rust A* can simplify `cost = edge_cost + tiebreaker[dir]` directly with no observable difference.

If the Rust port already implements no cost_multiplier (per `PATHFINDERCLASS_GHIDRA_REPORT.md` §9 "Gaps"), that gap is **not a gap** — it correctly matches the binary's observable behavior.

`PATHFINDERCLASS_GHIDRA_REPORT.md` §10 Q2 ("What sets this value?") is now answered: nothing after startup; the value is a constant 1.0.

---

## Sources

### Ghidra addresses decompiled this session:
- `0x0042a6d0` — real PathfinderClass constructor (writes +0x04 = 1.0)
- `0x0042a900` — PathfinderClass destructor
- `0x0042ac00` — per-map array resize (not the constructor)
- `0x0042a5b0` — PathfinderClass::Reset
- `0x00429a90` — AStar_main_loop (the reader)
- `0x0042c900` — AStar_pathfind_search
- `0x0042c1c0` — AllocZoneArrays
- `0x0042d170` — path distance estimator
- `0x004cbba0` — FootClass::Run_AStar (caller of AStar_pathfind_search)
- `0x004d3920` — FootClass::Find_Path
- `0x00567110` — MapClass::InitZoneMap
- `0x00685120` — scenario one-time setup (alternate InitZoneMap call)

### Disassembly inspected:
- `0x0040afa0..0x0040afc4` — CRT init/destroy thunks
- `0x0042a6d0..0x0042a8f9` — full constructor disassembly
- `0x004cbba0..0x004cbc3b` — full FootClass::Run_AStar disassembly (confirms `MOV ECX, 0x87e8b8` directly before the AStar_pathfind_search call)

### Memory reads:
- `0x008120a0..0x008120bf` — CRT init table (contains `0xA0AF4000` little-endian = pointer to `0x0040afa0`)
- `0x0087e8b8..0x0087e8f7` — PathfinderClass BSS region (all zeros at static load; populated at runtime by constructor)

### Xrefs:
- 24 xrefs to `0x0087e8b8` — all `MOV ECX,0x87e8b8 ; CALL <method>` patterns
- 0 xrefs to `0x0087e8bc` (the +0x04 field's absolute address)
- 1 xref to `0x0042a6d0` (from `0x0040afa5` only)
- 1 xref to `0x0040afa0` (from `0x008120a8` DATA — CRT init table)

### Related documents:
- `PATHFINDERCLASS_GHIDRA_REPORT.md` — overall PathfinderClass layout (note: §1 "embedded in MapClass+0xEC" claim corrected here)
- `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` — cost table at `DAT_0081870C`, Can_Enter_Cell codes
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` — A* algorithm overview
