# Path-Neighbor Table Family — Substrate Study

**Date:** 2026-06-04
**Author lane:** substrate / tables / path-neighbor
**Scope:** The per-direction geometry + per-edge cost/tie-break data that drives A* node
expansion, path reconstruction, and corner smoothing in gamemd.exe, and the design of a
single Rust-native substrate service that owns this data byte-exact. Study only — **no Rust
code written or modified**. Authority order: binary → Ghidra → docs.

**Confidence convention (per-claim, inline):**
- **PROVEN** = bytes read out of the binary this session (cite `read_memory` / `disassemble` / `decompile`).
- **UNCHECKED** = consistent with the binary but not bit-dumped (e.g. runtime-BSS values, out-of-lane semantic labels).
- **DRIFT** = Rust differs from the proven gamemd contract (default verdict; downgraded only by algebraic/exhaustive proof, none of which applies here).

**Burden of proof:** DRIFT is the default. No "not observable / sub-pixel / 1-tick / probably
equivalent" downgrades. Every disparity surfaced regardless of size; severity carries a
trigger-frequency clause for *prioritization only*, never to decide parity.

---

## (1) Active-YR responsibilities

The path-neighbor family is the per-direction geometry that drives **A\* node expansion** in
`AStar_main_loop` @ `0x00429a90` (reached on every pathfind via `AStar_pathfind_search` @
`0x0042c900`, the core YR ground/naval/infantry path entry). Fully live in stock YR — every
move order, every mid-route repath, every scatter/guard fallback consumes it. Player-visible
outputs it drives:

- **Which 8 compass neighbors (+1 tube step) a unit considers** at each search cell → the
  actual route a unit walks. (`decompile 0x00429a90`: loop `iStack_44 < 9`.)
- **Tie-break ordering** (cardinal-before-diagonal epsilons added to g-cost) → deterministic,
  lockstep-identical path choice when two routes cost the same; prevents path oscillation/wiggle.
- **Diagonal/bridge cost weighting** → units prefer/avoid cutting bridge corners; controls
  bridge approach paths. (`decompile 0x00429830`.)
- **Path-reconstruction direction emission** and **corner-smoothing (90° turn merge)** → the
  smooth diagonal walk vs zig-zag look. (`decompile 0x0042aa90`, `decompile 0x0042b210`.)
- **Tube/tunnel direction-8 jumps** — present-but-inert in stock YR (§3).

All of these fire in every match; none is an edge case.

---

## (2) Full inventory (every table + reader)

All static tables below were byte-dumped live this session. Float decodes are little-endian
IEEE-754; int32 decodes are little-endian two's-complement.

### 2.1 Static (`.rdata`/`.data`) tables — PROVEN

| Symbol | Address | Type | Values (decoded) | Verification |
|---|---|---|---|---|
| `g_CellNeighborOffsets_8Dir` | `0x007e3774` | int32[8] | N=-512, NE=-511, E=+1, SE=+513, S=+512, SW=+511, W=-1, NW=-513 | `read_memory 0x007e3774` → `00feffff 01feffff 01000000 01020000 00020000 ff010000 ffffffff fffdffff` |
| `g_AStar_EdgeCost_BaseTable` | `0x0081870c` | float32[8] | [0]=1.0, [1]=1000.0, [2]=1.0, [3]=1.0, [4]=60.0, [5]=20.0, [6]=8.0, [7]=10000.0 | `read_memory 0x0081870c` → `0000803f 00007a44 0000803f 0000803f 00007042 0000a041 00000041 00401c46` |
| `DirectionEpsilon` | `0x0081872c` | float32[9] | [0 N]=0.001, [1 NE]=0.005, [2 E]=0.002, [3 SE]=0.006, [4 S]=0.003, [5 SW]=0.007, [6 W]=0.004, [7 NW]=0.008, [8 tube]=0.0 | `read_memory 0x0081872c` → `6f12833a 0ad7a33b 6f12033b a69bc43b a69b443b 4260e53b 6f12833b 6f12033c 00000000` |
| Reconstruct dir lookup | base `0x0081874f`, accessed `*(int*)(0x0081875f + (dy*3+dx)*4)` | int32[9], index ∈[-4,+4] | {3,4,5,2,-1,6,1,0,7} | `read_memory 0x0081874f` → `…00030000 00040000 00050000 00020000 ffffffff 00060000 00010000 00000000 000700…` |
| `DAT_007e3760` delta→dir | base `0x007e3750`, accessed `*(int*)(0x007e3760 + (dy*3+dx)*4)` | int32[9], index ∈[-4,+4] | {NW=7, N=0, NE=1, W=6, (0,0)=-1, E=2, SW=5, S=4, SE=3} | `read_memory 0x007e3750` → `07000000 00000000 01000000 06000000 ffffffff 02000000 05000000 04000000 03000000` |
| `DAT_007e3710` bridge-flank (non-bridge) | `0x007e3710` | int32[8] | {-2,-2,0,1,1,1,0,-2} | `read_memory 0x007e3710` → `feffffff feffffff 00000000 01000000 01000000 01000000 00000000 feffffff` |
| `DAT_007e3730` bridge-flank (bridge) | `0x007e3730` | int32[8] | {0,-1024,-1024,-1024,0,512,512,512} | `read_memory 0x007e3730` → `00000000 00fcffff 00fcffff 00fcffff 00000000 00020000 00020000 00020000` |
| `_g_BridgeDiag_BothSides_2_0` | `0x007e37b4` | float32 | 2.0 | `read_memory 0x007e37b4` → `00000040` |
| `_g_BridgeDiag_NonBridge_10_0` | `0x007e37b8` | float32 | 10.0 | `read_memory 0x007e37b4`+4 → `00002041` |
| `_g_BridgeApproach_CostMult_4_0` | `0x007e37bc` | float32 | 4.0 | `read_memory 0x007e37b4`+8 → `00008040` |
| `_DAT_007e2ac8` bridge one-side | `0x007e2ac8` | float32 | 1.0 | cross-ref PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT §Sources; used in `decompile 0x00429830` as `fVar1 = _DAT_007e2ac8` |
| `_DAT_007e37c0` reopen tolerance | `0x007e37c0` | **8-byte double** (added to g, x87, for closed-reopen compare) | 1.00903… (PROVEN this session) | `read_memory 0x007e37c0` (8 bytes) → `be9f1a2f dd24f03f` = LE double `0x3ff024dd2f1a9fbe` ≈ 1.00903. **Type/size correction (adversarial re-check 2026-06-04):** `disassemble 0x00429a90` shows `FADD double ptr [0x007e37c0]` at 0x00429eec and 0x00429f21 — it is read as a **double**, NOT a float32. Earlier "float" label and the §6.3 `f32` proposal are wrong; see §6.3 note and Verification Log. |

512 (`0x200`) is the **CellClass\* pointer-array stride** (the `g_CellArray_Base + (y*0x200 + x)*4`
addressing in `decompile 0x00429a90`). The 0x007e3774 deltas are this stride ±1.

### 2.2 Runtime-filled (BSS) state

| Symbol | Address | Layout | Status | Verification |
|---|---|---|---|---|
| `g_DirectionOffsets` | `0x0089f688` | (short dx, short dy)[8], 4 bytes/entry, indexed `[dir & 7]` | **layout PROVEN, values UNCHECKED-at-static (runtime-filled)** | `read_memory 0x0089f688` twice → all `00` (BSS). Layout via `disassemble 0x00481810`: `LEA EAX,[EDX*4 + 0x89f688]; MOV DX,word[EDX*4+0x89f688]; MOV AX,word[EAX+2]`; second consumer `decompile 0x00429830` reads `*(short*)(&g_DirectionOffsets + (uVar8&7))` (dx) and `+2` (dy). |
| Closed/g-cost neighbor index offsets | `0x0089a304` | int32[8] | runtime-filled from `DAT_0089c2dc` (zone-grid width), **layout+formula PROVEN** | `decompile 0x0042ac00`: `DAT_0089c2dc = *(int*)(param_2+0xc)+1+*(int*)(param_2+8)`, then slots {304=`-W`, 308=`1-W`, 30c=`+1`, 310=`W+1`, 314=`+W`, 318=`W-1`, 31c=`-1`, 320=`-1-W`} = order N,NE,E,SE,S,SW,W,NW. `read_memory 0x0089a304` → all `00` static. |

**`g_DirectionOffsets` exact runtime shorts** — the binary fills these at init; the static image
holds zero, so the stored shorts are not recoverable from the dump. They are **constrained** to
the standard compass deltas N=(0,-1), NE=(+1,-1), E=(+1,0), SE=(+1,+1), S=(0,+1), SW=(-1,+1),
W=(-1,0), NW=(-1,-1) by two independent consistency anchors: (a) the matching 512-stride int
table at 0x007e3774 (delta = `dy*512 + dx`); (b) the delta→dir map at 0x007e3760 inverts exactly
those deltas. Verdict: **values UNCHECKED-from-static; treat as standard compass deltas, do not
claim PROVEN.**

### 2.3 Consumer/reader functions (all verified callable in YR)

| Function | Address | Role | Tables it reads | Verification |
|---|---|---|---|---|
| `AStar_main_loop` | `0x00429a90` | 9-dir neighbor expansion (`iStack_44 < 9`) | 0x007e3774 (cell fetch), 0x0089a304 (closed index), 0x0081872c (epsilon), g_TubeArray (dir 8) | `decompile 0x00429a90` |
| `AStar_compute_edge_cost` | `0x00429830` | per-edge g-cost | 0x0081870c (base), 0x007e3760 (delta→dir), 0x007e3710/0x007e3730 (flank), 2.0/10.0/1.0/4.0 consts, g_DirectionOffsets (blocker predict) | `decompile 0x00429830` |
| `AStar_create_node` | `0x0042a460` | alloc node; Euclidean h; g/f stored **float** | none (geometry via caller) | `disassemble 0x0042a460` (FLD/FADD/FSTP float; FILD+sqrt for h) |
| `AStar_reconstruct_path` | `0x0042aa90` | parent-chain walk → direction array | 0x0081874f (recon table), g_TubeArray (dir 8) | `decompile 0x0042aa90` |
| `Path_smooth_corners` | `0x0042b210` | 90° turn merge | g_DirectionOffsets, g_TubeArray | `decompile 0x0042b210` |
| `MapCoord_StepByDir_GetCell` | `0x00481810` | (dx,dy) neighbor primitive; 47+ YR callers | g_DirectionOffsets | `disassemble 0x00481810`, `get_function_callers 0x00481810` (stage 1) |
| `FUN_0042ac00` | `0x0042ac00` | allocates + fills closed/g-cost arrays + 0x0089a304 | DAT_0089c2dc | `decompile 0x0042ac00` |

No vtable/COM slots in the table family itself. The only virtuals touched in the neighbor loop
are `Can_Enter_Cell` (vtable+0x1ac), type accessors (+0x2c, +0x84), and the blocker coord/loco
accessors (+0x1b8/+0x1bc/+0x1b0) in the code-2 predictor — all out of this lane's scope.

---

## (3) Active vs legacy/dormant TS split

| Table / path | Status | Trigger frequency | Notes |
|---|---|---|---|
| 0x007e3774 neighbor offsets (dir 0-7) | **LIVE** | every A* expansion | core node fetch |
| 0x0089a304 closed-set offsets | **LIVE** | every search (allocated per search in FUN_0042ac00) | separate stride from 0x007e3774 |
| 0x0081872c direction epsilons | **LIVE** | every neighbor's g-cost | float-granularity route effect |
| 0x0081874f reconstruct / 0x007e3760 delta→dir | **LIVE** | every successful path | |
| g_DirectionOffsets @ 0x0089f688 | **LIVE** | core movement/placement primitive, 47+ callers | |
| Bridge-flank tables (0x007e3710 / 0x007e3730) + 2.0/10.0/1.0 + 4.0 ramp mult | **LIVE for bridges/ramps** | only when pathing near/across bridges — common on bridge maps, absent otherwise | gated on `bridge_flag && pathfinder+0x01` and dest `cell+0x140 & 0x100/0x800/0x40000`, all set by normal bridge geometry; **not** flag-gated off in YR. DRIFT default applies. |
| Direction 8 (tube/tunnel) branch | **TS-LEGACY / DORMANT in stock YR** | dir-8 loop iteration runs every expansion, but the branch produces a jump only when `cell+0x116 != -1`, which is never true on stock YR cells | `decompile 0x00429a90`: `if (*(short*)(*piVar22+0x116) == -1) piVar23 = &DAT_0089c2e0` (skip). Tube/subterranean is TS-only (project rule + `feedback_no_tunnel_subterranean`). **Present-but-inert.** A Rust port may stub it "no tube → skip" with no observable loss, but must mark it explicitly so it is not mistaken for a missing feature. |

---

## (4) Compare vs current Rust — table-by-table, helper-by-helper

Rust anchors read this session: `src/sim/pathfinding/core.rs` and
`src/sim/movement/bump_crush.rs`. Line numbers verified against the files as read.

### 4.1 Neighbor deltas — `NEIGHBORS` (core.rs L388-397)

Rust `NEIGHBORS: [(i32,i32,bool);8]` = N(0,-1) NE(1,-1) E(1,0) SE(1,1) S(0,1) SW(-1,1) W(-1,0)
NW(-1,-1), order N,NE,E,SE,S,SW,W,NW. The **(dx,dy) deltas and the order** match
`g_DirectionOffsets`' constrained values and the 0x007e3774 stride ±1 exactly.

- **dx/dy values + ordering: NO DRIFT** (matches the constrained gamemd deltas and the proven
  closed-set offset ordering N,NE,E,SE,S,SW,W,NW from `decompile 0x0042ac00`).
- **Representation DRIFT (structural, not value):** Rust derives the neighbor cell index as
  `ny*w + nx` on a single width `w`. gamemd uses **two distinct index spaces**: a 512-stride
  CellClass\* fetch (0x007e3774) *and* a `DAT_0089c2dc`-stride closed/g-cost index (0x0089a304).
  On the Rust single-grid model the two coincide by construction, so output is identical *as long
  as the Rust grid width equals the effective stride it indexes*. This is **not** a value drift
  but a **substrate-shape note**: the new service must keep the (dx,dy) primitive separate from any
  flat-index helper so a future non-square / bordered grid does not silently collapse the two.

### 4.2 Duplicate neighbor table — `NEIGHBOR_OFFSETS` (bump_crush.rs L75-84)

`bump_crush.rs` re-declares the same 8 deltas (without the `is_diagonal` bool). **Duplication
DRIFT (maintenance):** two independent literal copies of the same gamemd table; a one-line edit
to one will not propagate. Same values, so no *current* numeric drift, but the substrate service
must own a single source and both call sites must consume it.

### 4.3 Direction tie-break — `DIR_TIEBREAK` (core.rs L371-380) vs `DirectionEpsilon` @ 0x0081872c

Rust `DIR_TIEBREAK: [i32;8] = {1,5,2,6,3,7,4,8}` (N..NW), `TUBE_DIR_TIEBREAK = 9` (L384), added
to an **integer** g-cost at L1298 (`current.g_cost + step_cost + DIR_TIEBREAK[dir_index]`).

gamemd `DirectionEpsilon` (PROVEN) = float `{0.001,0.005,0.002,0.006,0.003,0.007,0.004,0.008}`,
tube = **0.0** (not 9), added to a **float** g-cost (`decompile 0x00429a90`:
`fStack_28 = edge*speed + *(float*)(&LAB_0081872c + dir*4)`).

- **Ordering of the 8 cardinals/diagonals: matches** (cardinals < diagonals; the integer ranks
  1..8 preserve the float ordering 0.001..0.008). For pure tie-break-of-equal-integer-edges this
  reproduces the gamemd preference.
- **Magnitude / scale: DRIFT.** gamemd adds a *fixed* float epsilon (0.001..0.008) to the g-cost
  *regardless of the edge base*. Rust adds a *fixed integer* (1..8) on top of `step_cost` that is
  itself scaled by ×4/×8/×20/×1000 (cliff/code multipliers). gamemd's epsilon is a constant; Rust's
  is the same constant but the *base it competes against* is differently scaled (see 4.4), so the
  relative weight of the tie-break vs the edge differs from gamemd. The Rust comment (L99-102)
  asserting the 1000× scale makes the ratio "exactly 0.001..0.008 of base" only holds for an
  unmultiplied base edge; once `step_cost` is multiplied (cliff/code/marker), the integer
  tie-break no longer sits at 0.001..0.008 of the edge, whereas gamemd's float epsilon is added
  *after* all multipliers and is therefore independent of them.
- **Tube tie-break value: DRIFT.** gamemd `DirectionEpsilon[8] = 0.0`; Rust `TUBE_DIR_TIEBREAK = 9`.
  (Inert because dir-8 is dormant in YR, §3, but a value drift to record.)

### 4.4 Edge-cost base — `STEP_COST` (core.rs L103) vs `g_AStar_EdgeCost_BaseTable` @ 0x0081870c

Rust uses a **uniform** `STEP_COST = 1000` (L1262), then applies *multipliers*: terrain
(`×100/terrain_cost`, L1263-1267), cliff `×4` (L1271), code-2 `×{1,4,1000}` (L1279-1284), code-5
`×20` (L1285), code-6 `×8` (L1286), marker `×4` (L1294).

gamemd indexes a **per-Can_Enter_Cell-code float base table** `{1.0,1000.0,1.0,1.0,60.0,20.0,8.0,
10000.0}` (PROVEN `read_memory 0x0081870c`), i.e. `param_5 = base_table[code]` (`decompile
0x00429830`), then bridge/ramp/code-2 adjustments multiply *that*.

- **Architecture DRIFT.** gamemd selects the base by *array lookup on the Can_Enter_Cell return
  code* (0-7); Rust starts from one uniform base and *multiplies by per-code factors*. These two
  produce the same number only where `base_table[code] == 1000 × factor`. They do **not** match:
  - code-0 (clear): gamemd 1.0 vs Rust 1000 (different absolute scale; ratios within a search are
    what matter, so this alone may be benign — but see code-4/code-7 below).
  - code-1 (gamemd 1000.0) and code-7 (gamemd 10000.0) have **no Rust counterpart** — Rust never
    assigns a 1000× or 10000× *base* edge to those codes; code-1/code-7 cells are handled as
    walkability rejects upstream, not as costed edges. **Potential missing-cost DRIFT** if a code-1
    or code-7 cell is ever costed rather than rejected. (Whether code-1/7 reach the cost fn at all
    depends on out-of-lane Can_Enter_Cell @ 0x73f0a0 — UNCHECKED here.)
  - **code-4 (gamemd 60.0): NO Rust counterpart at all.** Rust's multiplier set is {2:1/4/1000,
    5:20, 6:8} (L1278-1287) — there is no factor for code 4. If Can_Enter_Cell ever returns 4 in
    YR, gamemd charges 60× the base and Rust charges 1× (falls through `_ => 1`). **DRIFT** (the
    code-4 semantic label is itself UNCHECKED — see §below — but the *value* 60.0 is PROVEN and has
    no Rust analogue).
  - code-5 (gamemd 20.0) ↔ Rust ×20, code-6 (gamemd 8.0) ↔ Rust ×8: the *factor* matches the
    gamemd *base* numerically, but only because Rust's uniform base is 1000 and gamemd's clear base
    is 1.0 — Rust computes `1000×20` for an enemy cell where gamemd computes `20.0` for the edge and
    `1.0` for a clear edge. The **ratio** enemy:clear is 20:1 in gamemd but 20000:1000 = 20:1 in
    Rust — equal ratio, so within a single search these agree *for codes 0/5/6 only*. code-4 and
    the code-2 chain break this (code-2 in gamemd sets the edge to a *flat* 4.0 or 1000.0, not a
    multiplier — see 4.5).
- **Verdict: DRIFT** (architecture differs; equivalence holds only for the code-0/5/6 subset and
  only as a ratio, and fails for code-2/code-4/code-1/code-7). The single-base-times-factor model
  must be replaced by the proven base-table lookup.

### 4.5 code-2 (moving-friendly) — `compute_code2_multiplier` (core.rs L1279) vs `decompile 0x00429830`

Rust treats code-2 as a **multiplier** (`{1,4,1000}`) applied to `STEP_COST` (L1289
`step_cost *= mult`). gamemd sets the edge cost to a **flat absolute** value: after the 10-hop
chain walk, `param_5 = 4.0` (clears) or stays predicted, and `if pathfinder+0x3c == 2 → param_5 =
1000.0` (`decompile 0x00429830`). The chain-clears case yields base `1.0` (the code-2 base table
slot) only if the chain clears *and* `+0x3c==0`; otherwise 4.0; urgency-2 → 1000.0.

- **DRIFT.** gamemd: flat edge 1.0 / 4.0 / 1000.0 (absolute). Rust: ×1 / ×4 / ×1000 of a 1000
  base = 1000 / 4000 / 1,000,000. The *ratios among the three code-2 outcomes* (1:4:1000 gamemd
  vs 1:4:1000 Rust) match, but the **absolute magnitude relative to other codes' edges** differs:
  gamemd code-2-clear edge = 1.0 (same as a clear cell), Rust code-2-clear edge = 1000 (same as a
  clear cell, since Rust clear base is also 1000). Within the uniform-base subset this again holds
  as a ratio; it breaks the moment code-4 (60.0) or the bridge-flank flat multipliers enter. **The
  10-hop chain-walk structure (CODE2_CHAIN_MAX_HOPS=10, L128) matches `iVar7 < 10`** (PROVEN). The
  prediction uses g_DirectionOffsets in gamemd (`decompile 0x00429830`); confirm Rust's chain walk
  uses the same deltas — out-of-lane for the table study, flagged for synthesis.
- **Verdict: DRIFT on absolute model** (multiplier-of-1000 vs flat-absolute); ratio-equivalent
  only within the uniform-base subset.

### 4.6 Cliff / ramp — `CLIFF_COST_MULTIPLIER` (core.rs L118) vs ramp `×4.0`

Rust applies `×4` when `current.height != neighbor_height` (L1270-1272). gamemd applies the 4.0
multiplier when `dest cell+0x140 & 0x40000` is set (`decompile 0x00429830`:
`if ((iVar6+0x140 & 0x40000) != 0) param_5 *= 4.0`). The 0x40000 bit is a **temporary A\* search
marker** set around one search (per ASTAR_…MARKER_STACKING doc), *not* a raw height-difference
comparison.

- **DRIFT (trigger condition).** Same multiplier value (4.0 PROVEN @ 0x007e37bc), **different
  trigger**: gamemd = the 0x40000 marker flag; Rust = `height != neighbor_height`. A height step
  with the marker *unset* would be costed ×4 in Rust but ×1 in gamemd, and a marked cell with no
  height change would be ×1 in Rust but ×4 in gamemd. (Rust models the marker separately as
  `apply_search_marker_cost` / `SEARCH_MARKER_COST_MULTIPLIER=4`, L139/L1294 — so Rust has *both*
  a height-diff ×4 *and* a marker ×4, where gamemd has *only* the marker ×4. The height-diff ×4 has
  no gamemd analogue in this function — cliff/slope cost in gamemd is handled in `Zone_precheck` via
  `Zone_Estimate_Slope_Cost`, a *different* function, see cross-ref.) **DRIFT: Rust applies an
  extra ×4 on height change that this gamemd function does not.**
- **Trigger frequency:** fires whenever a unit paths across any height transition (ramps, cliff
  edges) — common on hilly/bridge maps.

### 4.7 Bridge-diagonal cost — Rust helpers "blocked" (core.rs L141-145) vs live in gamemd

Rust defines `BRIDGE_FLANK_MISSING_MULTIPLIER = 10` and bridge-flank helpers but documents them as
**not wired** ("Runtime wiring is blocked until `PathfinderClass+0x01` lifecycle is verified",
L143). gamemd applies them live (`decompile 0x00429830`): on `bridge_flag && pathfinder+0x01`,
select flank via 0x007e3760 → 0x007e3710 (if `&0x800`==0) or 0x007e3730 (if set), then **both
flanks bridge → ×2.0, one → ×1.0, neither → ×10.0**.

- **DRIFT (missing behavior).** The 2.0/10.0/1.0 bridge-diagonal weighting is not applied in Rust.
  Units cutting bridge diagonals will not get the gamemd cost shaping → different bridge approach
  paths. Constants are PROVEN (2.0 @ 0x007e37b4, 10.0 @ 0x007e37b8, 1.0 @ 0x007e2ac8); the
  flank-selection tables (0x007e3710/0x007e3730) and delta→dir (0x007e3760) are PROVEN.
- **Trigger frequency:** only on bridge-diagonal pathing — bridge maps only, but every bridge
  crossing on those maps.

### 4.8 g-cost / f-cost storage — Rust `i32` vs gamemd IEEE float32

Rust `AStarNode { f_cost: i32, g_cost: i32 }` (**struct def at core.rs L2028-2032**; integer values
written at the open-heap push sites L1310-1317 / L1357-1364, and `euclidean_heuristic` returns the
×1000-scaled integer `(sum_sq * 1_000_000).isqrt()` at L2079-2085). gamemd stores
both as **IEEE float32** at `node+0x4` (g) and `node+0x8` (f) (`disassemble 0x0042a460`:
`FLD float[ESP+0x20]; FADD float[EBX+0x4]; FSTP float[ESI+0x4]` for g; `FILD…CALL sqrt; FADD
float[ESI+4]; FSTP float[ESI+8]` for f). The decompiler's `(int)` casts on those node fields are
misreads.

- **DRIFT (numeric representation).** gamemd's epsilons (0.001..0.008) and flank costs (2.0/10.0)
  and the 1.009 reopen tolerance survive into the *float* heap ordering. Rust scales everything to
  integers (×1000) to keep the epsilons representable. This is the **root architectural drift** of
  the family: gamemd is float A* with sub-unit epsilons; Rust is integer A* with ×1000 scaling.
  Integer scaling is exact *iff* every gamemd value is an exact multiple of 0.001 after all
  multipliers — which fails for the Euclidean heuristic (`sqrt(dx²+dy²)` is irrational) and for the
  1.009 reopen tolerance, and for any flank×base product that is not a 0.001 multiple. **DRIFT** by
  default; downgrading would require an exhaustive bit-identical proof across the cost/heuristic
  input space, which does not exist.
- **Heuristic:** both use Euclidean `sqrt(dx²+dy²)` (Rust `euclidean_heuristic` L917/L1309; gamemd
  `disassemble 0x0042a460` FILD+sqrt). gamemd's sqrt is an *approximation* routine (`CALL
  0x004cac40`); Rust's `euclidean_heuristic` must match that approximation bit-for-bit *and* be
  integer-comparable — **UNCHECKED** whether Rust reproduces the same rounding. Flag for synthesis.

### 4.9 Tube (dir 8) edge — core.rs L1349-1351 vs Chebyshev

Rust: `STEP_COST * tube_steps + TUBE_DIR_TIEBREAK(9)` (L1349-1351). gamemd dir-8 edge =
**Chebyshev** `max(|cur.x − iVar16.x|, |cur.y − …|)` computed inline (`decompile 0x00429a90`:
`fStack_28 = (float)(int)max(|Δrow|, |Δcol|)`), epsilon **0.0**. Both inert in YR (§3), but
record: Rust uses path-length×STEP_COST + 9; gamemd uses Chebyshev distance + 0.0. **DRIFT (inert).**

### 4.10 Reconstruct + smoothing tables

Rust path reconstruction/smoothing lives in `path_smooth.rs` (per CANONICAL_DIRECTION_ENCODING
cross-ref; not read in this lane). gamemd recon table `{3,4,5,2,-1,6,1,0,7}` @ 0x0081874f and the
90°-merge rule `(d2−d1)&7 ∈ {2,6}` excluding -1/8 (`decompile 0x0042aa90`, `0x0042b210`) are
PROVEN. **UNCHECKED vs Rust** in this lane — flag for the synthesis stage to compare against
`path_smooth.rs`.

---

## (5) gamemd-native behavior contract (the spec the substrate must reproduce)

**Coordinate frame.** Cell-grid; packed at `cell+0x24` as (short x low, short y high); +X east,
+Y south. **Two distinct strides coexist** — **512 (0x200)** for the CellClass\* pointer array
(0x007e3774; main-loop `param3[1]*0x200 + param3[0]`) and **`DAT_0089c2dc`** (zone-grid width =
`map_border_w + 1 + map_origin_w`, `decompile 0x0042ac00`) for the closed/g-cost stamp arrays
(0x0089a304). **A port MUST keep these index spaces separate**; collapsing them is DRIFT on any
non-512-effective-width grid.

**Neighbor enumeration order.** Fixed loop `dir = 0..8` inclusive (9 iterations,
`decompile 0x00429a90` `iStack_44 < 9`). dir 0-7 = compass {N,NE,E,SE,S,SW,W,NW}: cell fetched via
`piVar22 + (&g_CellNeighborOffsets_8Dir)[dir]` (512-table), closed index via `(&DAT_0089a304)[dir]`
(zone-width table). dir 8 = tube via `cell+0x116 → g_TubeArray[idx]+0x28` (inert in YR). Order is
deterministic and load-bearing for tie-breaks — must be exactly N,NE,E,SE,S,SW,W,NW,tube.

**Per-neighbor g-cost.** `total = AStar_compute_edge_cost(...) * pathfinder.speed_factor(+0x04) +
DirectionEpsilon[dir]` (float10 arithmetic, `decompile 0x00429a90`). For dir 8 the edge increment
is **Chebyshev** `max(|Δrow|, |Δcol|)` to the step endpoint (computed inline, not via the cost fn),
epsilon 0.0.

**Edge cost (dir 0-7), `decompile 0x00429830`:**
1. `param_5 = g_AStar_EdgeCost_BaseTable[CanEnterCode]` (the `param_5 == 2.8026e-45` test is the
   IEEE bit pattern for integer **2** — denormal `0x00000002` — i.e. "code == 2").
2. code-2 branch: if `pathfinder+0x3c == 0`, walk the blocker's predicted path up to **10** hops
   (using g_DirectionOffsets) then `param_5 = 4.0`; if `+0x3c == 1`, `param_5 = 4.0` (no predict);
   if `+0x3c == 2`, `param_5 = 1000.0`.
3. ramp: if dest `cell+0x140 & 0x40000` → `× 4.0`.
4. bridge-diag: if `bridge_flag && pathfinder+0x01` — flank via 0x007e3760 (delta→dir) then
   0x007e3710 (if `&0x800`==0) or 0x007e3730 (if set), opposite flank via `(idx-4)&7`; both flanks
   bridge → `×2.0`, one → `×1.0`, neither → `×10.0`.

**Required constants (all PROVEN bytes):** base table 1.0/1000.0/1.0/1.0/60.0/20.0/8.0/10000.0;
diag 2.0/10.0/1.0; ramp 4.0; epsilons 0.001..0.008 (cardinals < diagonals), tube 0.0. Any
substitution (e.g. integer octile 10/14, or a uniform-base-times-factor model) is **DRIFT**.

**Node storage.** g and f are **IEEE float32** (`node+0x4`, `node+0x8`); h = `sqrt_approx(dx²+dy²)`
Euclidean on cell coords; f = g + h, all float (`disassemble 0x0042a460`). No integer truncation.
Epsilons survive into ordering at float granularity — not cosmetic.

**Heap ordering.** Binary min-heap on `node+0x8` (f-cost), standard sift-down (`decompile
0x00429a90` heap blocks). Not admissible/consistent (diagonals get no √2 edge multiplier yet h is
Euclidean), so the path is **not** guaranteed minimum-cost — reproduce the heap+epsilon behavior,
not "a correct A*".

**Closed-reopen tolerance.** A closed node is reconsidered when `(float)current.g + _DAT_007e37c0`
beats the stored g. **The decompiler renders `(float)_DAT_007e37c0`, but the assembly is
`FADD double ptr [0x007e37c0]` (`disassemble 0x00429a90` @ 0x00429eec / 0x00429f21) — the tolerance
is an 8-byte double `0x3ff024dd2f1a9fbe` = 1.00903…, promoted/added to the float g in x87.** The
node g (`piStack_48[1]`, node+0x4) is loaded as `FLD float`, the tolerance added as `FADD double`,
the comparison done in the x87 stack. (Value PROVEN this session via `read_memory 0x007e37c0`;
cross-ref ASTAR_MAIN_LOOP_…REOPEN_TOLERANCE.)

**Reconstruct (`decompile 0x0042aa90`).** Walk parent chain backward; per step, if `|Δy(+0x26)|<2
&& |Δx(+0x24)|<2` emit `recon_table[0x0081875f + (Δy*3 + Δx)*4]` (table {3,4,5,2,-1,6,1,0,7},
deltas = predecessor − current), else emit **8** (multi-cell span / tube). Terminate the array
with **-1 (0xFFFFFFFF)**. Min depth to reconstruct: `1 < node[3]` (depth > 1) else returns 0.

**Smoothing (`decompile 0x0042b210`).** Merge a dir change where `(d_next − d_prev) & 7 ∈ {2,6}`
(90° turn) into the intermediate diagonal, excluding `d == -1` (sentinel) and `d == 8` (tube). Then
`Path_optimize_straight_segments` straightens via Can_Enter_Cell shortcut checks, marks dropped
entries **-2 (0xFFFFFFFE)**, compacts, truncates to **20**.

**Boundary / edge behavior to reproduce exactly:**
- Out-of-range / null neighbor (`*piVar23 == 0`) → skipped, no expansion.
- dir-8 with `cell+0x116 == -1` → no jump (the always-taken case in YR).
- Iteration cap: caller `param_6 < 0` → `param_6 = 0xfff7` (65527); loop guard `param_6 <=
  local_34`. The literal **10000** is a *success-tail rejection equality* (`local_34 != 10000`),
  **not** a second loop cap.
- Zero/degenerate delta in the delta→dir tables → **-1** (the (0,0) slot), the "no direction"
  sentinel.
- `g_DirectionOffsets` is the (dx,dy) frame; 0x007e3774 is the 512-stride pointer-index frame —
  never port one onto the other.

**UNCHECKED / DRIFT flags carried into design:**
- (a) `g_DirectionOffsets` exact runtime shorts — layout PROVEN, values UNCHECKED-from-static
  (BSS); treat as standard compass deltas, mark UNVERIFIED.
- (b) Semantic **names** of Can_Enter_Cell codes 1-7 — the cost *values* are PROVEN; the labels are
  out-of-lane (Can_Enter_Cell @ 0x73f0a0). The two source docs disagree on code-4/5/6 labels
  (FriendlyWall/EnemyBlock/FriendlyStationary vs OccupiedFriendly/OccupiedEnemy/Cliff); **values
  identical, labels UNCHECKED**.
- (c) `_DAT_007e37c0` reopen tolerance — **RESOLVED in the 2026-06-04 adversarial re-check**: value
  bit-dumped this session as the 8-byte double `0x3ff024dd2f1a9fbe` ≈ 1.00903, read live via
  `FADD double ptr [0x007e37c0]`. It is a **double**, not a float32; the migration const must be
  `f64` (or the exact double promoted), and the add happens in x87 against the float g. No longer
  UNCHECKED.
- Default verdict for any unproven point: **DRIFT**.

---

## (6) Design — the Rust-native substrate-service boundary

**One pure, read-only, deterministic substrate service** owning the path-neighbor data family.
Rust-native structure; gamemd-native semantics.

### 6.1 Location in the module tree

`src/sim/pathfinding/neighbor_tables.rs` (new module under the existing `pathfinding/`). It owns
*data and pure geometry only* — no grid, no heap, no search state. `core.rs` (and `bump_crush.rs`
for the bare deltas) consume it. Respects the layering invariant: lives entirely in `sim/`, depends
on nothing in `render/ui/audio/net/`, depends on no other sim subsystem (it is a leaf).

### 6.2 Data ownership & construction source

All tables are **const, embedded from the gamemd dump** — they are static `.rdata` in the binary,
not INI-parsed and not map-derived. The one runtime-shaped table (the closed-set index offsets at
0x0089a304) is *derived from the grid width at search time*, so it is exposed as a pure function of
width, not a const. `g_DirectionOffsets` is embedded as the constrained compass deltas with an
explicit `// UNVERIFIED-from-static (BSS); constrained by 0x007e3774 + 0x007e3760` note.

### 6.3 API surface (signatures — descriptive, not code-to-implement)

```
// --- pure geometry primitives (the (dx,dy) frame, == g_DirectionOffsets) ---
pub const DIR_COUNT: usize = 8;                 // compass; tube (dir 8) handled separately
pub enum Dir8 { N, NE, E, SE, S, SW, W, NW }    // ordinal == gamemd dir index 0..7

pub const NEIGHBOR_DELTA: [(i16, i16); 8];      // (dx,dy) per Dir8; == g_DirectionOffsets
pub fn step(cell: (u16,u16), dir: Dir8) -> Option<(u16,u16)>;   // == MapCoord_StepByDir_GetCell
pub fn delta_to_dir(dx: i16, dy: i16) -> Option<Dir8>;          // == 0x007e3760 table; None == (0,0)/-1
pub fn dir_to_recon_code(dx: i16, dy: i16) -> i32;              // == 0x0081874f recon table; 8 if span
pub fn is_diagonal(dir: Dir8) -> bool;

// --- index-space helpers (KEEP SEPARATE from (dx,dy)) ---
pub fn cell_ptr_offset(dir: Dir8) -> i32;       // == 0x007e3774 (512-stride ±1) — pointer-array index
pub fn closed_index_offset(dir: Dir8, zone_grid_width: u32) -> i32; // == 0x0089a304 derivation (FUN_0042ac00)

// --- cost data (float, gamemd-exact) ---
pub const EDGE_COST_BASE: [f32; 8];             // == 0x0081870c {1,1000,1,1,60,20,8,10000}
pub const DIR_EPSILON: [f32; 9];                // == 0x0081872c {0.001..0.008, tube=0.0}
pub const BRIDGE_DIAG_BOTH: f32;                // 2.0   @ 0x007e37b4
pub const BRIDGE_DIAG_NEITHER: f32;             // 10.0  @ 0x007e37b8
pub const BRIDGE_DIAG_ONE: f32;                 // 1.0   @ 0x007e2ac8
pub const RAMP_MULT: f32;                       // 4.0   @ 0x007e37bc
pub const CODE2_FLAT_JAM: f32;                  // 4.0
pub const CODE2_FLAT_ROUTE_AROUND: f32;         // 1000.0
pub const REOPEN_TOLERANCE: f64;                // 1.00903… @ 0x007e37c0 — gamemd reads it as an
                                                // 8-byte DOUBLE (FADD double), NOT f32; value PROVEN
                                                // 0x3ff024dd2f1a9fbe. Add to the float g, compare in
                                                // wider precision (gamemd uses x87 80-bit).
pub const BRIDGE_FLANK_NONBRIDGE: [i32; 8];     // == 0x007e3710 {-2,-2,0,1,1,1,0,-2}
pub const BRIDGE_FLANK_BRIDGE: [i32; 8];        // == 0x007e3730 {0,-1024,-1024,-1024,0,512,512,512}
```

`NEIGHBOR_DELTA` is the single source the `bump_crush.rs` duplicate consumes. Each const carries an
inline gamemd-address comment as evidence (no behavioral engine refs in *code*, but address-anchored
evidence belongs in the substrate-data module's doc comments per existing pathfinding-table
practice; if policy forbids the address in the comment, the evidence lives in this doc and the const
name encodes the role).

### 6.4 Determinism guarantees

- All cost data is `f32` const — **deterministic across platforms** because the values are exact
  IEEE-754 dumps and the comparisons in the heap are `<` on those exact f32s. (This *changes* the
  current integer model; see §8 slice 4 — the float migration is the load-bearing parity move and
  must be validated against the lockstep state hash.)
- Pure functions, no interior mutability, no allocation, no RNG → bit-identical for identical input
  on every run/host.
- The two index spaces (512-stride vs zone-width) are exposed as separate functions so they can
  never be silently unified.

### 6.5 What the service does NOT own

The A* loop, heap, closed-list, Can_Enter_Cell, the marker overlay, the code-2 chain-walk
*driver* (it owns the deltas the walk uses, not the walk), and reconstruct/smooth *control flow*
(it owns the recon table + the 90°-merge rule constants, not the loop). Those stay in `core.rs` /
`path_smooth.rs` and call into the service.

---

## (7) Retire list (ad hoc / duplicated / approximated Rust tables & helpers)

| Rust item | file:line | Replaced by service member | Reason |
|---|---|---|---|
| `NEIGHBORS: [(i32,i32,bool);8]` | core.rs L388-397 | `NEIGHBOR_DELTA` + `is_diagonal` | scattered literal; keep value, move to service |
| `NEIGHBOR_OFFSETS: [(i32,i32);8]` | bump_crush.rs L75-84 | `NEIGHBOR_DELTA` | **duplicate** of the same gamemd table |
| `DIR_TIEBREAK: [i32;8] = {1,5,2,6,3,7,4,8}` | core.rs L371-380 | `DIR_EPSILON` (float 0.001..0.008) | integer-scaled approximation of float epsilons |
| `TUBE_DIR_TIEBREAK = 9` | core.rs L384 | `DIR_EPSILON[8] = 0.0` | wrong value (gamemd tube epsilon is 0.0) |
| `STEP_COST = 1000` (uniform base) | core.rs L103 | `EDGE_COST_BASE[8]` lookup | uniform base replaces per-code table |
| `CLIFF_COST_MULTIPLIER = 4` (height-diff trigger) | core.rs L118, applied L1270-1272 | `RAMP_MULT = 4.0` on the 0x40000 marker only | wrong trigger (height-diff vs marker flag); the extra height-diff ×4 has no analogue in 0x00429830 |
| `SEARCH_MARKER_COST_MULTIPLIER = 4` | core.rs L139, applied L1294 | `RAMP_MULT = 4.0` (this *is* the gamemd marker ×4) | keep value; this is the correct gamemd trigger — but it is currently *in addition to* the height-diff ×4 (which must be removed) |
| `CODE2_MULT_CLEARING/JAM/ROUTE_AROUND = 1/4/1000` | core.rs L122-124 | `CODE2_FLAT_JAM=4.0`, `CODE2_FLAT_ROUTE_AROUND=1000.0` + base-table[2]=1.0 | multiplier model replaced by flat-absolute |
| `CODE5_MULT_ENEMY = 20` | core.rs L131 | `EDGE_COST_BASE[5] = 20.0` | value correct, model (multiplier vs base) wrong |
| `CODE6_MULT_STATIONARY_ALLY = 8` | core.rs L133 | `EDGE_COST_BASE[6] = 8.0` | value correct, model wrong |
| `BRIDGE_FLANK_MISSING_MULTIPLIER = 10` (+ blocked flank helpers) | core.rs L141-145 | `BRIDGE_DIAG_NEITHER=10.0` + `BRIDGE_FLANK_*` tables + `BRIDGE_DIAG_BOTH/ONE` | unwire-blocked helper → live service data |
| `AStarNode { f_cost: i32, g_cost: i32 }` | core.rs **L2028-2032** (struct def); `euclidean_heuristic` L2079-2085 | float A* (see §8 slice 4) | integer-scaled g/f vs gamemd float32 |
| `CODE2_CHAIN_MAX_HOPS = 10` | core.rs L128 | (keep; it matches) — co-locate with service | constant matches `iVar7 < 10`; relocate for cohesion |

**No code-4 (60.0) handling exists in Rust** — there is nothing to retire; it is a *missing* table
slot the service introduces.

---

## (8) Migration slices + acceptance tests

Ordered, each independently shippable. Slices 1-3 are **pure-data-parity** (no behavior change to
the search if the values already agree — they consolidate + correct the data). Slice 4 is the one
**genuinely behavior-changing / stateful-risk** slice (integer→float A*), gated behind a lockstep
state-hash check. Slices 5-7 wire previously-missing/incorrect gamemd cost behavior.

> All acceptance tests are **exact-equality vs the gamemd dump** (the byte values in §2), tested
> across the full direction index space and the listed boundaries. No "approximately equal" tests.

### Slice 1 — Introduce `neighbor_tables.rs` with the geometry primitives (pure data)
Create the module; move `NEIGHBOR_DELTA`, `Dir8`, `step`, `delta_to_dir`, `dir_to_recon_code`,
`is_diagonal`, `cell_ptr_offset`, `closed_index_offset`. Repoint `core.rs::NEIGHBORS` and
`bump_crush.rs::NEIGHBOR_OFFSETS` to it (kills the duplicate). No behavior change.
- **Test `neighbor_delta_exact`**: for each Dir8, `NEIGHBOR_DELTA[dir]` == the constrained compass
  delta {(0,-1),(1,-1),(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1)}; *and* `cell_ptr_offset(dir)` ==
  {-512,-511,1,513,512,511,-1,-513} (== `read_memory 0x007e3774`).
- **Test `closed_index_offset_exact`**: for `zone_grid_width` ∈ {1, 2, 64, 512, 65535} (boundaries:
  min, small, typical, the pointer stride, max-u16), `closed_index_offset(dir, w)` ==
  {-w, 1-w, 1, w+1, w, w-1, -1, -1-w} per Dir8 (== `decompile 0x0042ac00` slot formulas). Asserts
  the two index spaces stay separate (ptr offset uses 512, closed offset uses w).
- **Test `delta_to_dir_exact`**: for every (dx,dy) ∈ [-1,1]² (9 cases incl. (0,0)),
  `delta_to_dir` == 0x007e3760 entry; (0,0) → None (sentinel -1).
- **Test `recon_code_exact`**: for every (dx,dy) ∈ [-1,1]², `dir_to_recon_code` == 0x0081874f
  entry {3,4,5,2,-1,6,1,0,7}; out-of-[-1,1] span → 8.

### Slice 2 — Embed the cost-data consts (pure data, not yet consumed)
Add `EDGE_COST_BASE`, `DIR_EPSILON`, bridge/ramp consts, flank tables, `REOPEN_TOLERANCE`. Not yet
wired into the search (search still uses the integer model). No behavior change.
- **Test `edge_cost_base_exact`**: `EDGE_COST_BASE` bit-equals the 8 floats from
  `read_memory 0x0081870c` ({1.0,1000.0,1.0,1.0,60.0,20.0,8.0,10000.0}), compared as raw `u32`
  bit patterns (boundary: code-4=60.0 and code-7=10000.0 the high-magnitude entries).
- **Test `dir_epsilon_exact`**: `DIR_EPSILON` bit-equals the 9 floats from `read_memory 0x0081872c`
  ({0.001,0.005,0.002,0.006,0.003,0.007,0.004,0.008,0.0}); assert cardinals[0,2,4,6] < diagonals
  [1,3,5,7] and `DIR_EPSILON[8] == 0.0` (tube boundary).
- **Test `bridge_ramp_consts_exact`**: `BRIDGE_DIAG_BOTH==2.0`, `BRIDGE_DIAG_NEITHER==10.0`,
  `BRIDGE_DIAG_ONE==1.0`, `RAMP_MULT==4.0` (bit-equal `read_memory 0x007e37b4` + 0x007e2ac8).
- **Test `bridge_flank_tables_exact`**: `BRIDGE_FLANK_NONBRIDGE == {-2,-2,0,1,1,1,0,-2}`
  (`read_memory 0x007e3710`); `BRIDGE_FLANK_BRIDGE == {0,-1024,-1024,-1024,0,512,512,512}`
  (`read_memory 0x007e3730`).

### Slice 3 — Replace `DIR_TIEBREAK`/`TUBE_DIR_TIEBREAK` integer ranks with the float epsilons (data-correctness, ordering-preserving)
Swap the integer tie-break for `DIR_EPSILON` *in ordering semantics only* — if slice 4 (float A*)
has not landed, this slice keeps the integer ranks but corrects the **tube** value to sit *below*
the cardinals (gamemd 0.0 < all), reproducing the gamemd tube tie-break ordering. (The full float
fidelity comes in slice 4; this slice removes the wrong tube=9 and documents the dependency.)
- **Test `tiebreak_ordering_matches_epsilon`**: the rank order of {N..NW, tube} produced by the
  Rust tie-break equals the ascending order of `DIR_EPSILON` indices {tube(0.0) < N(.001) < E(.002)
  < S(.003) < W(.004) < NE(.005) < SE(.006) < SW(.007) < NW(.008)}. (Boundary: tube must sort
  *first*, not last — the current `TUBE_DIR_TIEBREAK=9` sorts it last, which this test fails until
  fixed.)

### Slice 4 — Float A* g/f storage (the load-bearing, stateful-risk slice)
Convert `AStarNode` g/f to `f32`, the g-cost arrays to `f32`, and the edge accumulation to
`edge*speed + DIR_EPSILON[dir]` in float; heuristic uses the same sqrt-approx rounding as gamemd
(`0x004cac40`); closed-reopen uses `(f64)g + REOPEN_TOLERANCE` where `REOPEN_TOLERANCE` is the
**8-byte double** 1.00903… (gamemd does `FLD float g; FADD double[0x007e37c0]` and compares in x87 —
the promotion to ≥64-bit before the add is part of the contract, so a naive `g_f32 + 1.009_f32`
would DRIFT). **This changes path choice** wherever
the integer scaling diverged → must be validated against the lockstep state hash, not just unit
tests. Gate behind the substrate program's migration toggle.
- **Test `gcost_is_float32`**: g/f fields are `f32`; an edge of base 1.0 + epsilon 0.001 yields
  g == 1.001f32 exactly (not 1001 integer).
- **Test `euclidean_h_matches_sqrt_approx`**: for a set of (dx,dy) including (0,0),(1,0),(1,1),
  (3,4)→5.0, (large) — `euclidean_heuristic` bit-equals the gamemd sqrt-approx output for the same
  squared input. (UNCHECKED until the 0x004cac40 routine is decoded — this test is the gate that
  proves or refutes the approximation match; if it cannot be made exact, the slice surfaces a
  documented residual DRIFT rather than silently shipping a different h.)
- **Lockstep guard `state_hash_unchanged_on_open_maps`**: run a fixed seed skirmish replay on
  bridge-free maps before/after; the per-tick state hash must be identical *or* the diff must be
  fully explained by the corrected tie-break (no unexplained path divergence).

### Slice 5 — Per-code base-table lookup (replace uniform STEP_COST × factor)
Wire `EDGE_COST_BASE[CanEnterCode]` as the edge base; convert code-2/5/6 to the flat-absolute /
base-table model; **add the missing code-4 = 60.0** slot. Requires the Can_Enter_Cell code on each
edge (already produced by the existing predicate at L1155-area; confirm it returns the 0-7 code).
- **Test `edge_base_by_code`**: for each code 0-7, the edge base before multipliers equals
  `EDGE_COST_BASE[code]` (boundary: code-4 must be 60.0, currently 1× in Rust → this test fails
  pre-fix and is the regression lock for the missing slot).
- **Test `code2_flat_not_multiplier`**: a code-2 edge with urgency 0/clear → 1.0 (base[2]); urgency
  0/10-hop-jam → 4.0; urgency 2 → 1000.0 (flat, not ×1000-of-base).

### Slice 6 — Ramp ×4 trigger correction (marker flag, not height-diff)
Remove the `current.height != neighbor_height → ×4` (core.rs L1270-1272); keep only the
marker-driven `RAMP_MULT` (L1294, which already matches gamemd's 0x40000 trigger). Cliff/slope cost
proper is `Zone_precheck`/`Zone_Estimate_Slope_Cost` territory (cross-ref) — out of this slice;
this slice only removes the *spurious* height-diff ×4 that 0x00429830 does not have.
- **Test `no_extra_cliff_mult_in_edge_cost`**: an edge across a height change with the 0x40000
  marker *unset* costs ×1 (base only), not ×4; an edge with the marker set costs ×4 regardless of
  height change. (Boundary: height-change-but-no-marker is the case that currently over-charges.)

### Slice 7 — Bridge-diagonal cost (wire the blocked helpers)
Apply the 2.0/10.0/1.0 weighting via the flank tables when `bridge_flag && pathfinder+0x01` and
dest flags 0x100/0x800 (`decompile 0x00429830`). Requires the `PathfinderClass+0x01` lifecycle
that core.rs L143 cited as blocking — resolve that (out-of-lane verification) first.
- **Test `bridge_diag_both_2x`**: a bridge diagonal whose two flanks are both bridge cells →
  edge × 2.0; one flank bridge → × 1.0; neither → × 10.0 (the three branches of `decompile
  0x00429830`). Flank selection uses `BRIDGE_FLANK_NONBRIDGE` when dest `&0x800`==0, else
  `BRIDGE_FLANK_BRIDGE` (boundary: the 0x800 toggle).
- **Lockstep guard `state_hash_unchanged_off_bridges`**: replay on bridge-free maps must be
  byte-identical (this slice may only affect bridge maps).

> **Dir-8 (tube) is intentionally NOT migrated to a live behavior.** It stays present-but-inert
> (§3); slice 1 reproduces its *table* values for completeness, but no slice wires a tube jump
> because YR authors no tubes. A code comment must mark it dormant, not missing.

---

## Anchors & Evidence

| Address / symbol | Ghidra call cited (this session) | Doc cross-ref |
|---|---|---|
| `g_CellNeighborOffsets_8Dir` 0x007e3774 | `read_memory 0x007e3774` | PATHFINDING_ASTAR_GHIDRA_REPORT §9 |
| `g_AStar_EdgeCost_BaseTable` 0x0081870c | `read_memory 0x0081870c` | PATHFINDING_STANDALONE_FUNCTIONS §Sources |
| `DirectionEpsilon` 0x0081872c | `read_memory 0x0081872c` | PATHFINDING_ASTAR_GHIDRA_REPORT §9 |
| Reconstruct table 0x0081874f | `read_memory 0x0081874f`, `decompile 0x0042aa90` | — |
| delta→dir 0x007e3760 | `read_memory 0x007e3750`, `decompile 0x00429830` | — |
| flank tables 0x007e3710 / 0x007e3730 | `read_memory 0x007e3710`, `read_memory 0x007e3730` | BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK |
| diag/ramp consts 0x007e37b4/b8/bc, 0x007e2ac8 | `read_memory 0x007e37b4` | BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK §11 |
| `g_DirectionOffsets` 0x0089f688 | `read_memory 0x0089f688` (BSS zero), `disassemble 0x00481810`, `decompile 0x00429830` | TACTICAL_SCREEN_PIXEL_TO_CELL §3.4 |
| closed-set offsets 0x0089a304 / DAT_0089c2dc | `decompile 0x0042ac00`, `read_memory 0x0089a304` | BRIDGE_ASTAR_DUAL_CLOSED_LIST |
| `AStar_main_loop` 0x00429a90 | `decompile 0x00429a90` | ASTAR_MAIN_LOOP_…REOPEN_TOLERANCE |
| `AStar_compute_edge_cost` 0x00429830 | `decompile 0x00429830` | PATHFINDING_STANDALONE_FUNCTIONS §2.4; ASTAR_…MARKER_STACKING |
| `AStar_create_node` 0x0042a460 | `disassemble 0x0042a460` | — |
| `Path_smooth_corners` 0x0042b210 | `decompile 0x0042b210` | CANONICAL_DIRECTION_ENCODING |
| `MapCoord_StepByDir_GetCell` 0x00481810 | `disassemble 0x00481810` | PATHFINDING_STANDALONE_FUNCTIONS |

---

## DRIFT Ledger

Severity = player-visibility × frequency (for prioritization only). Every row is DRIFT by default
(no equivalence proof exists). Float-bit comparison required for all f32 rows.

| Rust file:line | Current Rust | gamemd-correct (PROVEN unless noted) | Severity + trigger-frequency |
|---|---|---|---|
| core.rs L2028-2032 (AStarNode g/f) + L2079-2085 (heuristic) | `g_cost: i32`, `f_cost: i32` (×1000 integer model); h = `(sum_sq*1e6).isqrt()` | IEEE float32 g at node+0x4, f at node+0x8 (`disassemble 0x0042a460`); reopen tolerance is an **8-byte double** 1.00903 (`disassemble 0x00429a90` `FADD double[0x007e37c0]`) | **HIGH** — fires on **every pathfind in every match**; integer scaling cannot represent the irrational Euclidean h nor the 1.00903 double tolerance exactly → path divergence on some cells. |
| core.rs L371-380, L384, applied L1298/L1351 | `DIR_TIEBREAK {1,5,2,6,3,7,4,8}` int + `TUBE=9` | `DIR_EPSILON {0.001..0.008}` float, tube `0.0` (`read_memory 0x0081872c`) | **HIGH** — every neighbor every search; ordering ok for unmultiplied edges but tube sorts wrong and magnitude drifts vs multiplied edges. |
| core.rs L103, L1262 | uniform `STEP_COST=1000` base | per-code `EDGE_COST_BASE {1,1000,1,1,60,20,8,10000}` (`read_memory 0x0081870c`) | **HIGH** — every edge; **code-4=60.0 has NO Rust handling**, code-1/7 (1000/10000) unmodeled. |
| core.rs L122-124, L1279-1289 | code-2 as ×{1,4,1000} of 1000 | flat absolute 1.0 / 4.0 / 1000.0 (`decompile 0x00429830`) | **MED** — fires whenever a friendly moving unit blocks a path (group moves, convoys) — common; ratio-equivalent within uniform-base subset, breaks with code-4/flank. |
| core.rs L118, L1270-1272 | extra `×4` on `height != neighbor_height` | ramp ×4 only on `cell+0x140 & 0x40000` marker (`decompile 0x00429830`); no height-diff ×4 in this fn | **MED** — fires on every height transition (ramps/cliffs) — common on hilly/bridge maps; Rust over-charges height steps the engine does not. |
| core.rs L141-145 (helpers unwired) | bridge-diag 2.0/10.0/1.0 **not applied** | apply per flank tables (`decompile 0x00429830`) | **MED** — bridge-diagonal pathing only (bridge maps), but every bridge crossing there; different bridge approach routes. |
| core.rs L1349-1351 | tube edge `STEP_COST×steps + 9` | Chebyshev `max(|Δrow|,|Δcol|)` + epsilon 0.0 (`decompile 0x00429a90`) | **LOW (inert)** — dir-8 dormant in stock YR (no tubes authored); never fires in a normal skirmish. |
| bump_crush.rs L75-84 | `NEIGHBOR_OFFSETS` duplicate of NEIGHBORS | single shared `NEIGHBOR_DELTA` | **LOW (maintenance)** — no current numeric drift; risk is future one-sided edit. Consumed every bump/crush check. |
| core.rs (g_DirectionOffsets users) | constrained compass deltas, not bit-verified | layout PROVEN, values **UNCHECKED-from-static** (BSS) | **N/A (verification gap)** — values match the constraint anchors; flag, do not claim PROVEN. |
| (no Rust line) | code-4 cost slot **absent** | `EDGE_COST_BASE[4] = 60.0` | **MED-if-reachable** — depends on whether Can_Enter_Cell returns 4 in YR (UNCHECKED, out-of-lane). |

---

## Verification Log (adversarial re-check, 2026-06-04)

Re-verified the load-bearing claims live, defaulting every claim to DRIFT/UNVERIFIED until the
binary proved it. Method: re-`read_memory` every static table byte-for-byte, re-`decompile` /
`disassemble` the four consumer functions, and re-`Read` the cited Rust line ranges.

**Static tables — all byte-equal to the doc (PROVEN):**
- `g_CellNeighborOffsets_8Dir` 0x007e3774 → `00feffff 01feffff 01000000 01020000 00020000 ff010000 ffffffff fffdffff` = {-512,-511,1,513,512,511,-1,-513}. **VERIFIED** (`read_memory 0x007e3774`).
- `g_AStar_EdgeCost_BaseTable` 0x0081870c → `0000803f 00007a44 0000803f 0000803f 00007042 0000a041 00000041 00401c46` = {1.0,1000.0,1.0,1.0,60.0,20.0,8.0,10000.0}. **VERIFIED** (`read_memory 0x0081870c`).
- `DirectionEpsilon` 0x0081872c → `6f12833a 0ad7a33b 6f12033b a69bc43b a69b443b 4260e53b 6f12833b 6f12033c 00000000` = {0.001,0.005,0.002,0.006,0.003,0.007,0.004,0.008,0.0}. **VERIFIED** (`read_memory 0x0081872c`).
- delta→dir 0x007e3760 (base 0x007e3750) → `07 00 01 06 -1 02 05 04 03`. **VERIFIED** (`read_memory 0x007e3750`).
- recon table 0x0081874f → `{3,4,5,2,-1,6,1,0,7}`. **VERIFIED** (`read_memory 0x0081874f`).
- flank tables 0x007e3710 `{-2,-2,0,1,1,1,0,-2}` and 0x007e3730 `{0,-1024,-1024,-1024,0,512,512,512}`. **VERIFIED** (`read_memory 0x007e3710`, `read_memory 0x007e3730`).
- diag/ramp consts 0x007e37b4=2.0, 0x007e37b8=10.0, 0x007e37bc=4.0; one-side 0x007e2ac8=1.0. **VERIFIED** (`read_memory 0x007e37b4`, `read_memory 0x007e2ac8`).

**Consumer functions:**
- `AStar_main_loop` 0x00429a90: 9-dir loop (`iStack_44 < 9`); dir-8 tube branch gated `*(short*)(*piVar22+0x116)==-1`; 512-stride cell fetch `param_3[1]*0x200`; closed index `(&DAT_0089a304)[dir]` on `DAT_0089c2dc` stride; epsilon add `edge*speed + *(float*)(&LAB_0081872c + dir*4)` in float; dir-8 Chebyshev `max(|Δrow|,|Δcol|)` epsilon 0; `param_6<0 → 0xfff7`; `local_34 != 10000` is the success-tail equality, not a loop cap. **ALL VERIFIED** (`decompile 0x00429a90`, `disassemble 0x00429a90`).
- `AStar_compute_edge_cost` 0x00429830: `param_5 = base_table[code]` array lookup; `param_5 == 2.8026e-45` = the bit pattern for integer 2 (code-2 test, `convert_number 0x00000002` = 2); code-2 chain max 10 hops (`iVar7 < 10`); flat outcomes 1.0 (chain clears, +0x3c==0) / 4.0 (jam or +0x3c==1) / 1000.0 (+0x3c==2); ramp `if (dest+0x140 & 0x40000) *= 4.0`; bridge-diag gate `param_4 && pathfinder+0x01`, flank via 0x007e3760→0x007e3710(`&0x800`==0)/0x007e3730, opposite `(idx-4)&7`, both→2.0 / one→1.0 / neither→10.0; uses `g_DirectionOffsets` in blocker-predict. **ALL VERIFIED** (`decompile 0x00429830`).
- `AStar_create_node` 0x0042a460: g stored float32 at node+0x4 (`FLD float[ESP+0x20]; FADD float[EBX+0x4]; FSTP float[ESI+0x4]`); f stored float32 at node+0x8 (`FILD dword[ESP+0x20]; FSTP double[ESP]; CALL 0x004cac40; FADD float[ESI+0x4]; FSTP float[ESI+0x8]`); h = sqrt-approx of integer dx²+dy². **VERIFIED** (`disassemble 0x0042a460`).
- `FUN_0042ac00` 0x0042ac00: `DAT_0089c2dc = *(int*)(p+0xc)+1+*(int*)(p+8)`; closed-set slots {-W,1-W,1,W+1,W,W-1,-1,-1-W} = N,NE,E,SE,S,SW,W,NW; arrays sized `W*W*4`. **VERIFIED** (`decompile 0x0042ac00`).
- `AStar_reconstruct_path` 0x0042aa90: per-step `|Δrow|<2 && |Δcol|<2` → emit `recon_table[(Δrow*3+Δcol)*4]` (Δ = pred−cur), else 8; array terminated -1; min depth `1 < node[3]`. **VERIFIED** (`decompile 0x0042aa90`).
- `Path_smooth_corners` 0x0042b210: 90° merge `(d_next−d_prev)&7 ∈ {2,6}`, excluding -1 and 8; steps via g_DirectionOffsets. **VERIFIED** (`decompile 0x0042b210`).
- `MapCoord_StepByDir_GetCell` 0x00481810: `CMP dir,8/JNC` bound; `AND dir,7`; reads (short dx, short dy)[8] at 0x89f688. Callers enumerated: **51** real YR callers (≥ the doc's "47+"). **VERIFIED** (`disassemble 0x00481810`, `get_function_callers 0x00481810`).

**BSS / runtime tables:**
- `g_DirectionOffsets` 0x0089f688 → all `00` static (BSS). Layout PROVEN, values still **UNCHECKED-from-static** — doc's stance is correct, NOT claimed PROVEN. **VERIFIED-as-stated** (`read_memory 0x0089f688`).
- closed-set offsets 0x0089a304 → all `00` static (runtime-filled). **VERIFIED-as-stated** (`read_memory 0x0089a304`).

**Rust anchors re-read (line numbers checked against the current files):**
- `STEP_COST=1000` L103; `CLIFF_COST_MULTIPLIER=4` L118; code-2 `1/4/1000` L122-124; `CODE2_CHAIN_MAX_HOPS=10` L128; `CODE5=20` L131; `CODE6=8` L133; `SEARCH_MARKER=4` L139; bridge-flank helpers `10/1/2` + "wiring blocked" comment L141-147. **ALL VERIFIED.**
- `DIR_TIEBREAK {1,5,2,6,3,7,4,8}` L371-380; `TUBE_DIR_TIEBREAK=9` L384; `NEIGHBORS` L388-397. **VERIFIED.**
- `NEIGHBOR_OFFSETS` (duplicate) bump_crush.rs L75-84. **VERIFIED.**
- application: terrain/cliff/code/marker/tiebreak edge build L1262-1298; integer-node push L1306-1317; tube edge L1349-1351. **VERIFIED.**
- `AStarNode { f_cost: i32, g_cost: i32 }` **struct def L2028-2032** (the doc's §4.8/ledger "L1306-1317" was the push site, not the struct); `euclidean_heuristic` returns `(sum_sq*1_000_000).isqrt() as i32` L2079-2085. **VERIFIED + line-citation corrected in place.**

### WRONG / corrected this re-check
1. **Reopen tolerance `_DAT_007e37c0` type/size.** Doc §2.1 labeled it "float" and §6.3 proposed
   `REOPEN_TOLERANCE: f32`. **WRONG.** `disassemble 0x00429a90` shows `FADD double ptr [0x007e37c0]`
   at 0x00429eec and 0x00429f21 — it is an **8-byte double**. `read_memory 0x007e37c0` (8 bytes) =
   `be9f1a2f dd24f03f` = little-endian double `0x3ff024dd2f1a9fbe` ≈ **1.00903**. The *value* the doc
   carried (≈1.009) is correct, but it was marked UNCHECKED-from-static and typed as float.
   **Corrected in place:** §2.1 row (type → 8-byte double, value PROVEN with the bytes), §5 closed-
   reopen paragraph (FADD double, x87 promotion), §5 flag (c) (RESOLVED), §6.3 `REOPEN_TOLERANCE:
   f64` with x87-promotion note, slice-4 test note (naive `g_f32 + 1.009_f32` would DRIFT), and the
   DRIFT-ledger AStarNode row. **Impact on stage-2 recommendations:** slice 4's acceptance must use
   an `f64` (or exact-double) tolerance added to the float g with ≥64-bit intermediate precision; an
   `f32` const as originally specified would itself be a parity bug. This *strengthens* (does not
   invalidate) the slice-4 float-migration recommendation.

### UNVERIFIABLE (carried, not deleted)
- `g_DirectionOffsets` exact runtime shorts — BSS-zero at static; values remain constrained-not-proven.
  Confirmed still UNVERIFIABLE this lane (would need a live/debugger read post-init). Doc already
  marks it correctly; no change.
- Can_Enter_Cell code semantic labels (1-7) and whether code-4 is reachable in YR — out-of-lane
  (`Can_Enter_Cell` @ 0x73f0a0, corroborated as a real caller of 0x00481810 this session, but its
  return-code semantics not decoded here). The *values* in the base table are PROVEN; the labels stay
  UNCHECKED. No change.
- gamemd sqrt-approx routine 0x004cac40 — confirmed as the `CALL` target inside 0x0042a460 but not
  decoded this lane; slice-4's `euclidean_h_matches_sqrt_approx` test remains the gate. No change.

**Net: 0 fabricated facts found. Every PROVEN byte/formula re-confirmed. 1 type/size error corrected
(reopen tolerance float→double, value upgraded UNCHECKED→PROVEN). 3 line-citation tightenings
(AStarNode struct + heuristic). No DRIFT verdict was overturned — all DRIFTs stand.**
