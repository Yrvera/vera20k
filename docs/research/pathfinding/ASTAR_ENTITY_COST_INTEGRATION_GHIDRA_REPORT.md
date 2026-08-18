# A* Entity Cost Integration — Ghidra Research Report

**Addresses:**
- `0x00429a90` AStar_main_loop (Can_Enter_Cell dispatch site)
- `0x00429830` AStar_compute_edge_cost (cost table lookup + code-2 prediction)
- `0x0073F0A0` UnitClass::Can_Enter_Cell (vtable+0x1AC, returns 0-7 codes)
- `0x004b2630` DriveLocomotionClass::Process_Movement (urgency escalation)
- `0x0081870C` DAT — Can_Enter_Cell code → cost table (8 floats)

**Confidence:** HIGH (decompiled and verified, 2026-04-05, cross-referenced with four prior reports)

**Active in YR:** Yes — core ground-unit movement path, runs on every A* and every movement tick

**Purpose of this report:** Answer the specific question — *does gamemd.exe's A* consult entity positions during search, and what mechanism routes ground units around friendly movers?* This report was prompted by discovery of an invented "cooperative pathfinding" mechanic in the Rust code (`penalty_cells` with 4× multiplier on friendly movers' upcoming path cells) that was NOT in the binary.

---

## 1. Overview — The Short Answer

**Yes, the binary's A* checks entity positions during search.** It does so per-neighbor via the `Can_Enter_Cell` virtual call (vtable offset `+0x1AC`). The return code 0-7 drives BOTH passability (code ≥ 7 rejects the node) AND cost (code indexes the cost table at `DAT_0081870c`).

Friendly-mover avoidance is handled by **dynamic cost assignment** at A* expansion time, NOT by precomputed "penalty cell sets" or path reservations. The cost assigned to a moving-friendly cell can be `1.0` when the prediction chain shows the blocker will clear, `4.0` for normal jam/urgency, or `1000.0` for destroyer urgency escalated via the `BlockedDelay` timer. Urgency state is stored in `PathfinderClass+0x3C` and controlled by the `urgency` argument passed to `FootClass::Find_Path`.

---

## 2. A* Neighbor Expansion — Where Entity Costs Enter

### 2.1 The per-neighbor Can_Enter_Cell call

Inside `AStar_main_loop` at `0x00429a90`, within the 9-direction expansion loop (`do { ... } while (iStack_44 < 9)`):

```c
iVar17 = (**(code **)(*param_4 + 0x1ac))(
    iVar16,                                   // target cell
    iStack_44,                                // facing direction (0-8)
    *(undefined4 *)(param_1 + 0x30),          // prev height
    *piVar22,                                 // source cell
    CONCAT31((int3)((uint)iVar17 >> 8),
             *(undefined1 *)(param_1 + 8))    // bridge flag
);
```

- `param_4` is the `FootClass*` unit doing the pathfinding
- `vtable+0x1AC` is `Can_Enter_Cell` (for `UnitClass` this is `0x0073F0A0`, decompiled in [UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md](UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md))
- The call is made once per neighbor direction (0-8) per expanded node

### 2.2 Crusher override

```c
if ((bVar10) && (iVar17 < 7)) {
    iVar17 = 0;   // Crusher treats all non-impassable codes as clear
}
```

- `bVar10` is set earlier: `*(char *)(iVar12 + 0xc94) != '\0'` — `TechnoTypeClass::Crusher` flag
- Effect: Crusher units (Apocalypse, Rhino Tank) ignore friendly/enemy occupancy costs and just check pure terrain passability

### 2.3 Rejection threshold

```c
if (iVar17 < 7) {
    // ... compute cost, add node to open set ...
}
```

- Codes `0-6` → node added to open set with cost from cost table
- Code `7+` → node not expanded (impassable)

### 2.4 Cost computation chain

```c
fVar25 = AStar_compute_edge_cost(piVar22, piVar23, bridge_flag, iVar17, piVar18);
fStack_28 = fVar25 * PathfinderClass[0x04]          // speed factor
          + DirectionEpsilon[iStack_44];             // 0.001..0.008 tiebreaker
```

The Can_Enter_Cell return code `iVar17` is passed as the **5th argument** (`float param_5`) to `AStar_compute_edge_cost`. Inside that function, `(int)param_5` reads back the integer code via bit reinterpretation (x86 stack-passing trick — int and float are both 4 bytes, so the integer bits survive the `float` parameter type).

---

## 3. The Cost Table — `DAT_0081870C`

Verified by raw memory read (8 floats, 32 bytes, in `.rdata` section):

| Index | Can_Enter_Cell Code | Base Cost | Semantic (from UNIT_CAN_ENTER_CELL report) |
|-------|---------------------|-----------|--------------------------------------------|
| 0 | Clear | **1.0** | Fully passable |
| 1 | Crushable | **1000.0** | Civilian / crushable object |
| 2 | TemporaryBlock | **1.0 / 4.0 / 1000.0** (see §4) | Moving friendly unit |
| 3 | ScatterRequired | **1.0** | Scatter-required / friendly to bump (corrected 2026-07-12: code-name column was "BridgeRamp", self-contradicting the doc's own Semantic column; binary body at 0x0073F0A0 sets code 3 via `BuildingClass__CanGarrison`==false + `HouseClass__IsAlliedWith`==true on the occupying building — an allied non-garrisonable building blocking the cell, matching the existing Ghidra plate comment "3=ScatterRequired (allied building, bump it)" — verified via `decompile_function 0x0073F0A0` — RTTI_LABEL_DRIFT) |
| 4 | OccupiedFriendly | **60.0** | Friendly wall / stationary friendly blocking |
| 5 | OccupiedEnemy | **20.0** | Enemy unit (may fight through) |
| 6 | FriendlyStationary | **8.0** | Non-moving allied non-building object (verified from Can_Enter_Cell @ 0x73F0A0) |
| 7 | Impassable | **10000.0** | Never expanded (rejected above) |

**Verified addresses:** cross-checked against memory read in [PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md](PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md) §1.1, also matches [PATHFINDING_ASTAR_GHIDRA_REPORT.md](PATHFINDING_ASTAR_GHIDRA_REPORT.md) §6.2.

**Naming note:** The original A* report labeled these indices with LandType names (Clear/Road/Water/Rock/Wall) which is MISLEADING — the table is indexed by the semantic Can_Enter_Cell return, NOT by cell LandType. The verification report corrected this.

### 3.1 Codes 4 vs 5 vs 6 — ordering observation

The cost for "OccupiedFriendly" (code 4) is **60.0** while "OccupiedEnemy" (code 5) is **20.0** — **friendly occupation is 3× more expensive than enemy occupation**. This reflects the engine's preference: units will **fight through** enemies rather than path around them, but will **avoid stationary allies** (and just wait for them via the BlockedDelay timer) rather than route through.

Meanwhile `FriendlyStationary` (code 6) is **8.0** — the cheapest non-trivial non-clear cost in this table, signaling that non-moving allied non-building blockers are the most tolerable detour, while "Crushable" (code 1) at **1000.0** almost always routes around civilians unless the unit is a Crusher.

---

## 4. Code 2 (Moving Friendly) — Path Prediction + Urgency

**Verified from assembly at `0x00429830`-`0x004299a4` (2026-04-05).** Labels added in Ghidra: `AStar_cost_predict_loop_top` (0x00429886), `AStar_cost_predict_blocker_clears` (0x00429986), `AStar_cost_predict_set_jam_cost_4_0` (0x0042998d), `AStar_cost_predict_urgency_override` (0x00429995).

### 4.1 Prediction loop semantics

The loop walks the **first blocker**'s predicted trajectory up to 10 cells ahead:

```c
// Initial state: param_5 = cost_table[2] = 1.0 (base cost)
piVar9 = cell->E4 (or E8 for bridge);     // object list at neighbor cell
EBP = 0;                                   // loop counter
if (PathfinderClass+0x3C != 0) goto AStar_cost_predict_set_jam_cost_4_0;

AStar_cost_predict_loop_top:
    if (piVar9 == NULL) goto AStar_cost_predict_blocker_clears;
    if (!(piVar9[0x14] & 0x4)) goto AStar_cost_predict_set_jam_cost_4_0;  // not active
    if (piVar9[0x578 double] == 0.0) {      // velocity == 0
        direction = piVar9[0x5E0];           // path_queue[0]
        if (direction == -1) goto AStar_cost_predict_blocker_clears;
    } else {
        direction = (RateTimer_Current() >> 12 + 1) >> 1 & 7;
    }
    cell_coords = piVar9->GetCellCoords();   // vtable+0x1B8
    next_cell = cell_coords + DirectionOffsets[direction];
    next_cell_obj = MapClass::Get_CellClass(next_cell);
    // Select ground or bridge list based on bridge flag + height diff
    piVar9 = next_cell_obj->E4 (or E8);
    EBP++;
    if (EBP < 10) goto AStar_cost_predict_loop_top;
    // else fall through to set_jam_cost (4.0)

AStar_cost_predict_set_jam_cost_4_0:
    param_5 = 4.0;

AStar_cost_predict_blocker_clears:   // (only reachable if urgency == 0)
    // Skips the 4.0 assignment — param_5 stays at 1.0 (base)

AStar_cost_predict_urgency_override:
    if (PathfinderClass+0x3C == 2) param_5 = 1000.0;
```

### 4.2 All 5 exit paths (verified from assembly trace)

| Exit condition | Final cost | Meaning |
|---|---|---|
| urgency != 0, prediction skipped | **4.0** (urg=1) / **1000.0** (urg=2) | Unit is already blocked, fast path |
| Loop: piVar9 == NULL (empty cell) | **1.0** | Blocker's path leads to clear terrain — will clear |
| Loop: stationary object + path_queue[0] == -1 | **1.0** | Blocker has no path, transient — will clear |
| Loop: object's IsOnMap bit 2 clear (inactive) | **4.0** | Object along chain is not active (destroyed/removed) |
| Loop: 10 iterations completed without empty cell | **4.0** | 10-cell jam ahead — soft block |

**Key finding:** Moving-friendly cells can cost as low as **1.0** (same as clear terrain!) when the prediction shows the blocker will clear. This is a **path-prediction optimization**, NOT a soft-block — the binary prefers routes where friendlies are momentarily in the way but about to move.

### 4.3 Urgency-to-cost mapping (verified)

| Urgency | Prediction runs? | Cost for code-2 cells |
|---|---|---|
| 0 (new path) | Yes | **1.0** (clears) or **4.0** (jam) |
| 1 (blocked, waiting) | No | **4.0** flat |
| 2 (blocked, expired) | No | **1000.0** flat |

---

## 5. Urgency Escalation — Tick-Level State Machine

**Verified from assembly at `0x004b3649`-`0x004b3a0e` (2026-04-05).** Labels added in Ghidra: `ProcMove_dispatch_on_return_code` (0x004b3649), `ProcMove_code2_moving_friendly_branch` (0x004b3656), `ProcMove_code2_init_blocked_delay` (0x004b3663), `ProcMove_blocked_delay_expired_urgency2` (0x004b36ed), `ProcMove_Find_Path_urgency1_entry` (0x004b39d1), `ProcMove_call_Find_Path_with_urgency` (0x004b3a0e).

### 5.1 Full state machine (assembly-verified)

When `Process_Movement` receives code `2` from `Can_Enter_Cell`:

```c
// [0x004b3649] Dispatch on return code:
if (return_code != 2) goto other_code_branch;  // 0x004b3a97

// [0x004b3656] Code-2 branch — check if this is the first tick blocked:
if (foot+0x6B7 == 0) {                          // path_blocked_flag not set
    foot+0x6B7 = 1;                              // set flag
    foot+0x668 = g_CurrentFrameCounter;          // blocked_delay_start
    foot+0x66C = snapshot_of_pending_facing;     // blocked_delay_facing
    foot+0x670 = Rules+0x1768;                   // blocked_delay_ticks = BlockagePathDelay (60 default)
}

// [0x004b3690] Check movement_delay (rate limiter for pathfinder calls):
if (foot+0x640 != -1) {
    elapsed = current_frame - foot+0x640;
    if (elapsed < foot+0x648) goto skip_this_tick;   // movement_delay still active
}

// [0x004b36bc] movement_delay expired — derive urgency from blocked_delay:
if (foot+0x6B7 == 0) {
    // Flag not set — go to urgency=1 path (unreachable from code-2 first-entry)
    goto ProcMove_Find_Path_urgency1_entry;  // 0x004b39d1, XOR BL,BL → urgency=1
}
if (foot+0x668 != -1) {
    elapsed = current_frame - foot+0x668;
    if (elapsed < foot+0x670) {
        goto ProcMove_Find_Path_urgency1_entry;  // 0x004b39d1 — blocked_delay still running
    }
}
// blocked_delay expired:
BL = 1;                                          // [0x004b36ed]
goto after_XOR_BL_BL;                            // 0x004b39d3 — preserves BL=1

// [0x004b39d1] ProcMove_Find_Path_urgency1_entry:
XOR BL, BL;                                      // BL = 0 → urgency will be 1

// [0x004b39d3] after_XOR_BL_BL — compute cell coords + call Find_Path:
cell_x = (this+0x34 + sign_byte) >> 8;           // convert lepton X to cell X (DriveLocomotionClass's OWN field, not the FootClass's — read directly off EBP, not through the +0xc backpointer)
cell_y = (this+0x38 + sign_byte) >> 8;           // convert lepton Y to cell Y (same struct as above)
dest_packed = (cell_y << 16) | cell_x;

// [0x004b39fb-0x004b3a00] Compute urgency from BL:
urgency = (BL != 0) ? 2 : 1;                     // TEST BL,BL; SETNZ DL; INC EDX

// [0x004b3a0e] Call Find_Path:
Find_Path(dest_packed, /*is_crusher*/ 0, urgency);
```

(corrected 2026-07-12: the coordinate reads at `[ESP... ]`-adjacent `[EBP+0x34]`/`[EBP+0x38]` were labeled "foot+0x34"/"foot+0x38", implying a FootClass field read through the `+0xc` backpointer used elsewhere in this same function for `+0x640/+0x648/+0x668/+0x66C/+0x670/+0x6B7`. Disassembly of `0x004b2630`-`0x004b2637` shows `MOV EBP,ECX` at function entry — EBP holds the DriveLocomotionClass `this` pointer for the whole function body — and `[EBP+0x34]`/`[EBP+0x38]` are read directly off EBP, with no `[EBP+0xc]` indirection. This is a distinct coordinate-frame bug class per project convention: the field belongs to DriveLocomotionClass itself, not FootClass, even though it likely still represents a cell/lepton coordinate. Verified via `disassemble_function 0x004b2630` — STRUCT_FAMILY_CASCADE)

### 5.2 Urgency values actually used

- **urgency=0** — used for NEW path requests (no blocking context). Call sites: `DriveLocomotionClass::Process_Movement@0x004b28a3` (initial path) and `DriveLocomotionClass::Process_Movement@0x004b3f37` (code-6 stationary-ally branch, with is_crusher from Crusher flag).
- **urgency=1** — `blocked_delay` timer still running (unit has been blocked < 60 frames). Entry at `0x004b39d1`.
- **urgency=2** — `blocked_delay` timer expired (unit has been blocked ≥ 60 frames). Entry at `0x004b39d3` after `MOV BL,0x1` at `0x004b36ed`.

### 5.3 Two-timer system

| Timer | Offsets | Purpose |
|---|---|---|
| `movement_delay` | `+0x640` start / `+0x644` facing / `+0x648` ticks | **Rate limiter** — prevents repath thrashing. If active, skip pathfinder call this tick. |
| `blocked_delay` | `+0x668` start / `+0x66C` facing / `+0x670` ticks | **Patience timer** — controls urgency escalation. Running → urgency=1. Expired → urgency=2. |

`path_blocked_flag` at `+0x6B7` tracks whether the unit is currently in a code-2 blocked state. Set on first code-2 encounter; cleared when a movement step succeeds normally (code 0 arrival).

### 5.2 INI keys

| Key | Section | Default | Purpose | Location |
|-----|---------|---------|---------|----------|
| `BlockagePathDelay` | `[General]` | **60** | Frames of patience before escalating to destroyer urgency | `rulesmd.ini:3107` |
| `CloseEnough` | `[General]` | **2.25** | Cells — if destination is within this, stop instead of re-pathing around stationary ally | `rulesmd.ini:58` |

Both keys are read via RulesClass offsets:
- `g_RulesClass + 0x1768` = `BlockagePathDelay`
- `g_RulesClass + 0x1718` = `CloseEnough`

---

## 6. Summary of the Real Mechanism

```
 [movement tick: Process_Movement]
           │
           ▼
   Can_Enter_Cell(next_cell) ──► code 2 (moving friendly blocks)?
           │                            │
           │ yes                        ▼
           ▼                    blocked_delay_start timer begins
 start/check BlockedDelay       (urgency escalates after 60 frames)
           │
           ▼
   Find_Path(dest, urgency=1 or 2)
           │
           ▼
 PathfinderClass+0x3C := urgency
           │
           ▼
   [AStar_main_loop per neighbor]
           │
           ▼
   Can_Enter_Cell(neighbor) ──► returns 0-7
           │
           ▼
   code < 7?  if no: reject neighbor
           │
           ▼
   AStar_compute_edge_cost:
     - cost = DAT_0081870c[code]
     - if code == 2 AND urgency == 0:
         - walk blocker's path up to 10 cells
         - if loop finds an empty cell or stationary blocker: cost = 1.0 (blocker clears)
         - if loop finds inactive object or hits 10-iter limit: cost = 4.0 (jam)
     - if code == 2 AND urgency == 1: cost = 4.0 (no prediction)
     - if code == 2 AND urgency == 2: cost = 1000.0 (destroyer — route around)
     - apply bridge 0x40000 multiplier (4.0) if set
     - apply diagonal bridge cost modifiers
     - multiply by PathfinderClass+0x04 speed factor
     - add DirectionEpsilon tiebreaker
           │
           ▼
   add to open set
```

---

## 7. Current Rust Implementation Status

### 7.1 Current Rust status

This section was originally written against an older Rust implementation that had `penalty_cells` and `COOPERATIVE_COST_MULTIPLIER`. A 2026-05-22 verify-doc audit found that status stale: current Rust now has `entity_block_map` with cost codes, `AStarOptions::urgency`, and `[General] BlockagePathDelay` parsing. The binary facts in §§2-6 remain the implementation target, but the old "penalty_cells" inventory below is historical.

### 7.2 What the binary has that we don't

| Mechanism | Binary | Rust |
|-----------|--------|------|
| Per-neighbor Can_Enter_Cell call during A* | **Yes** (`*param_4[0x1ac]`) | Partial: current Rust precomputes `entity_block_map` cost codes rather than making the vtable call per neighbor |
| Return-code-indexed cost table (0-7) | **Yes** (`DAT_0081870c`) | Partial: current Rust carries entity cost codes 2/5/6 plus hard blockers; exact full table parity still needs implementation audit |
| Moving friendly → cost 1.0 / 4.0 / 1000.0 depending prediction and urgency | **Yes** | Partial: current Rust has `entity_block_map` code 2, `AStarOptions::urgency`, and a 10-hop chain walk; exact object-list/locomotor parity still needs implementation audit (corrected 2026-06-01: was "urgency state, but full 10-step prediction-chain parity was not re-audited here"; source shows `compute_code2_multiplier` lines 2087-2131, while binary target is verified via `decompile_function 0x00429830` - STALE) |
| Stationary friendly → cost 8.0 (soft) | **Yes** | Partial: current Rust maps stationary friendly entities into `entity_block_map` as code 6 and applies an 8x multiplier; exact Can_Enter_Cell decision-tree parity still needs implementation audit (corrected 2026-07-12: line citation drifted; `build_entity_block_sets` lives in `src/sim/movement/bump_crush.rs` (not `core.rs`), function starts line 114, the code-6 stationary-friendly insert is at lines 207-215 (was cited "201-209", which is now the code-2 moving-friendly insert) — file content re-read this session; `CODE6_MULT_STATIONARY_ALLY` at `core.rs:133-134` still correct; binary cost table verified via `read_memory 0x0081870C` - source drift, line numbers only) |
| Enemy → cost 20.0 (soft) | **Yes** | Partial: current Rust maps enemy entities into `entity_block_map` as code 5 and applies a 20x multiplier; exact Can_Enter_Cell decision-tree parity still needs implementation audit (corrected 2026-07-12: line citation drifted; `build_entity_block_sets` lives in `src/sim/movement/bump_crush.rs` (not `core.rs`), the code-5 enemy insert is at lines 176-190 (was cited "170-183") — file content re-read this session; `CODE5_MULT_ENEMY` at `core.rs:130-131` still correct; binary cost table verified via `read_memory 0x0081870C` - source drift, line numbers only) |
| Path prediction loop (10 cells) | **Yes** | Partial: current Rust implements a 10-hop code-2 chain walk in `compute_code2_multiplier`; exact parity for first-blocker selection, inactive objects, and layer choice still needs implementation audit (corrected 2026-06-01: was "No"; source shows `CODE2_CHAIN_MAX_HOPS` at `core.rs:126-128` and loop at `core.rs:2113-2131`, binary loop verified via `decompile_function 0x00429830` - STALE) |
| Urgency escalation (BlockedDelay) | **Yes** | Partial: current Rust has `AStarOptions::urgency` and parses `BlockagePathDelay`; end-to-end parity still needs a focused implementation audit |
| Crusher override | **Yes** | Partial: current Rust bypasses entity soft-block costs for `AStarOptions::mover_is_crusher`; full crusher Can_Enter_Cell/path-through parity still needs implementation audit (corrected 2026-06-01: was "No"; source shows `mover_is_crusher` at `core.rs:709-711` and the bypass at `core.rs:1274-1276`, binary override verified via `decompile_function 0x00429a90` - STALE) |
| Destroyer mode (cost 1000.0) | **Yes** | Partial: current Rust maps `urgency >= 2` to the 1000x code-2 route-around multiplier; end-to-end blocked-delay ordering still needs implementation audit (corrected 2026-06-01: was "No"; source shows `CODE2_MULT_ROUTE_AROUND` at `core.rs:124` and `compute_code2_multiplier` lines 2105-2107, binary override verified via `decompile_function 0x00429830` - STALE) |
| `BlockagePathDelay` INI key | **Yes** (read at runtime) | Parsed in current Rust |
| `CloseEnough` INI key | **Yes** (read at runtime) | Parsed in current Rust (corrected 2026-07-12: line citation drifted from "ruleset.rs lines 1079-1082" as of 2026-06-01; current code reads `CloseEnough` at `src/rules/ruleset.rs:1372` (`close_enough:` field spans 1371-1374) — file content re-read this session; binary Rules+0x1718 use was verified via `decompile_function 0x004B2630` and `decompile_function 0x004D3920` - source drift, line numbers only) |

### 7.3 What our Rust has that the binary doesn't

Historical stale note: the old Rust implementation used **`penalty_cells` on friendly movers' upcoming 24 path cells with 4× multiplier**. That mechanism did not match gamemd.exe because it conflated:
  - The bridge 0x40000 flag multiplier (real, value 4.0, but triggered by bridge approach, not by friendly path cells)
  - The Can_Enter_Cell code-2 cost (real, value 1.0 / 4.0 / 1000.0 depending prediction and urgency, but triggered by CURRENT occupancy, not PREDICTED path cells) (corrected 2026-06-01: was "4.0 / 1000.0"; binary shows code-2 can leave base-table 1.0 on blocker-clears exit via `decompile_function 0x00429830` - MISLEADING)
  - The 24-waypoint loop limit (real, but that's `UpdateBridgePassability` walking scanned peer object path queues, not the searching unit's own path queue)

2026-05-22 correction from `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` audit: the peer-path marker starts at `path[0]` for both kind `1` and kind `0xF` after their prerequisites, and the masked write is equivalent to XOR-toggling the destination cell's `0x40000` bit. It is not an alternating inverse-of-source pattern.

### 7.4 Fidelity implications

**Old behavior (with invented `penalty_cells`; stale as of 2026-05-22):**
- Friendly moving units soft-blocked their future path cells (4× cost)
- Stationary friendly units hard-blocked
- Enemies hard-blocked
- No urgency — same cost regardless of how long blocked

**Binary behavior:**
- Friendly moving units soft-block their CURRENT cell with code-2 dynamic cost 1.0 / 4.0 / 1000.0 depending prediction and urgency (corrected 2026-06-01: was "cost 4.0"; binary shows base-table 1.0, jam 4.0, and urgency-2 1000.0 via `decompile_function 0x00429830` - MISLEADING)
- Stationary friendly units soft-block their current cell (cost 8.0) — NOT hard-blocked
- Enemies soft-block (cost 20.0) — NOT hard-blocked
- Urgency escalates; stuck units re-path at 1000.0 cost to route around

**Divergences to re-check against current Rust:**
1. Stationary friendlies should be cost 8.0, allowing path-through at higher cost.
2. Enemies should be cost 20.0, allowing fight-through.
3. Moving-friendly code 2 should apply to current occupancy and can produce 1.0, 4.0, or 1000.0 depending prediction and urgency.
4. Urgency escalation must flow from blocked-delay state into `PathfinderClass+0x3C` / `AStarOptions::urgency`.

---

## 8. Historical Implementation Pathways

This section predates the current Rust `entity_block_map` / `AStarOptions::urgency` work. Keep it only as historical context for why the old `penalty_cells` model was rejected; do not use it as the current implementation plan without first re-auditing current Rust.

### 8.1 Historical Option A — Minimal fidelity fix (small scope)

Remove the old `penalty_cells` entirely. Keep the old hard-block behavior for all entities.

**Pros:** Small change, removes false binary-citation, no new state needed.
**Cons:** Even further from binary — moving friendlies now become fully passable, worsening divergence. May cause visible regression in group movement.

### 8.2 Historical Option B — Map entity blocks to cost codes (medium scope)

Replace `entity_blocks: BTreeSet<(u16,u16)>` with `entity_costs: BTreeMap<(u16,u16), u8>` where the value is the Can_Enter_Cell-style code (2, 4, 5, 6, 7). A* indexes a cost table by this code.

**Pros:** Close to binary semantics, allows fight-through and path-through. No urgency needed for first pass.
**Cons:** Requires per-tick BTreeMap construction; API churn across all `AStarOptions`, `find_*` wrappers, `build_entity_block_*`; need a cost table constant.

### 8.3 Option C — Full fidelity (large scope)

Implement Can_Enter_Cell as a callable function (not a precomputed map) invoked per-neighbor during A* expansion. Add urgency state to `PathfinderClass` equivalent. Wire BlockedDelay timer into movement tick for urgency escalation.

**Pros:** Full binary parity, supports Crusher, supports prediction loop, supports destroyer mode.
**Cons:** Large refactor touching A*, movement tick, and rules parsing. Requires callback plumbing into A* (currently a pure function).

---

## 9. Open Questions

### 9.1 For implementation

- **Q:** In Option B, should `entity_costs` be layer-separated (ground vs bridge) like current `entity_blocks`? Likely yes — same reasoning (bridge/ground coexistence).
- **Q:** In Option B, what to do about the goal cell? Binary allows arrival on occupied cell (Find_Nearby_Passable_Cell redirects before A*). Our current code exempts goal from `entity_blocks`. For `entity_costs`, should goal be exempt from occupancy cost too?
- **Q:** Does removing `penalty_cells` regress observable group movement? Only verifiable by in-game test.
- **Q:** Should Option B also implement `Find_Nearby_Passable_Cell` behavior for blocked destinations? (Existing report at `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`.)

### 9.2 For further research

- **Q:** What decides the Can_Enter_Cell return code mapping in the caller? This report covered the A* side; the actual vtable implementation at `0x0073F0A0` is documented in `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`, but the mapping from "moving friendly" → code 2 is spread across that function's ~466-line object-iteration loop. Precise decision tree is documented there.
- **Q:** How does `InfantryClass::Can_Enter_Cell` differ from `UnitClass::Can_Enter_Cell`? (Infantry may have different occupancy semantics via sub-cells.)
- **Q:** What's in `PathfinderClass+0x04` (the `cost_multiplier` applied to all costs)? The PATHFINDERCLASS report calls it `cost_multiplier`; exact formula not documented here.

---

## 10. Key Corrections to Prior Claims

**Claim (from earlier in this session):** "A* in the binary is entity-blind; entity collisions are resolved at FootClass bump/wait at movement tick, not in A*."

**Reality:** A* in the binary is **entity-aware**. It calls `Can_Enter_Cell` per neighbor, which checks cell occupancy and returns a code 0-7. The cost table bakes entity occupancy into the A* cost function. FootClass bump/wait at movement tick is a SEPARATE mechanism that handles what happens when a unit arrives at a previously-clear cell that has since become occupied — it does NOT replace A*'s entity awareness.

**Claim:** "The `penalty_cells` 4× multiplier at `BRIDGE_APPROACH_COST_MULTIPLIER` is dead code until bridges land."

**Reality:** The 4.0 constant is used for TWO DIFFERENT mechanisms in the binary:
1. **Bridge 0x40000 flag multiplier** (`DAT_007e37bc`, read at `0x004299bc`, applied when `cell+0x140 & 0x40000`). Set by `PathfinderClass::UpdateBridgePassability` for bridge-approach steering.
2. **Code-2 moving-friendly cost** (the `4.0` literal at `0x0042998d` in `AStar_compute_edge_cost` is the jam / urgency-1 assignment; code 2 can also stay at base-table `1.0` when prediction clears or be overridden to `1000.0` at urgency 2). (corrected 2026-06-01: was "hardcoded 4.0 ... applied when Can_Enter_Cell returns 2 AND urgency == 0"; binary shows the `0x0042998d` assignment is skipped by the clears path and urgency 2 overrides at `0x00429995` via `decompile_function 0x00429830` - OPERATOR_OR_ORDER_DRIFT)

These share the value `4.0` but are distinct mechanics with different triggering conditions.

**Claim:** "Moving friendly units should become fully passable in A* (matching binary)."

**Reality:** Moving friendly units should use the **code-2 dynamic cost** in A*: `1.0` if the prediction chain clears, `4.0` for jam / urgency 1, and `1000.0` for urgency 2. Fully passable as code 0 is still wrong because code 2 preserves blocker semantics even when the resulting cost equals clear terrain. (corrected 2026-06-01: was "should have cost 4.0"; binary shows the three-way code-2 cost path via `decompile_function 0x00429830` - MISLEADING)

---

## Sources

### Binary addresses decompiled this session
- `AStar_main_loop@0x00429a90` — full decompilation, confirmed `Can_Enter_Cell` call at vtable+0x1AC, crusher override, 7+ rejection threshold
- `AStar_compute_edge_cost@0x00429830` — full decompilation, confirmed code-2 prediction loop + cost table lookup

### Prior reports cross-referenced
- [PATHFINDING_ASTAR_GHIDRA_REPORT.md](PATHFINDING_ASTAR_GHIDRA_REPORT.md) — original A* pipeline report (note: had misleading "LandType" names for cost table)
- [PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md](PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md) — corrected cost-table naming, documented prediction loop
- [UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md](UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md) — full Can_Enter_Cell decompilation with code 0-7 semantic meanings
- [UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md](UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md) — urgency escalation state machine
- [BRIDGE_SYSTEM.md](../bridges/00-system-models/BRIDGE_SYSTEM.md) — 0x40000 flag details
- [ADDRESS_MAP.md](../ADDRESS_MAP.md) — address index

### INI keys verified
- `rulesmd.ini:3107` — `BlockagePathDelay=60` in `[General]`
- `rulesmd.ini:58` — `CloseEnough=2.25` in `[General]`

### Rust implementation state (as of 2026-04-05)
- [src/sim/pathfinding/core.rs:51](../src/sim/pathfinding/core.rs#L51) — `COOPERATIVE_COST_MULTIPLIER = 4` (misnamed)
- [src/sim/pathfinding/core.rs:149-150](../src/sim/pathfinding/core.rs#L149-L150) — `AStarOptions::penalty_cells` field
- [src/sim/pathfinding/core.rs:550-555](../src/sim/pathfinding/core.rs#L550-L555) — cooperative penalty branch
- [src/sim/movement/bump_crush.rs:98-163](../src/sim/movement/bump_crush.rs#L98-L163) — friendly-mover penalty collection (24-step lookahead)

### Rust implementation state update (2026-05-22 verify-doc)
- The 2026-04-05 line anchors above are historical; current Rust no longer has `penalty_cells` or `COOPERATIVE_COST_MULTIPLIER`.
- Current Rust has `entity_block_map` cost-code plumbing, `AStarOptions::urgency`, and `BlockagePathDelay` parsing. Treat remaining Rust deltas in this report as implementation-audit targets, not as a current inventory.
