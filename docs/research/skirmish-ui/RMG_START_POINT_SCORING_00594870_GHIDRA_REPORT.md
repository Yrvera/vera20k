# RMG Start-Point Candidate Scoring and Selection — 0x00594870 / 0x00594F40 — Ghidra Research Report

**Addresses:** `0x00594870` (candidate gatherer + waypoint writer), `0x00594F40` (best-first selector), `0x005A7250` (6×6 passability gate)
**Investigation mode:** exhaustive-slice — resolves OQ-11 from `RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md`
**Confidence:** High for all material scoring paths; Medium for `DAT_00abe030` identity (candidate-list capacity) and the `CellClass__IsClearTile` clear-tile exact contract (body unreachable from known naming in this session)
**Active in YR:** Conditional — same path-liveness conditions as parent report: `.SED` random-map launch only

---

## 0. Investigation Scope

- **Target question:** How does `0x00594870` gather candidate cells and how does `0x00594F40` select the best start position from them?
- **Non-goals:** water seeding, region partition, tiberium, LAT, `FUN_005A1FB0` flood-fill, RNG primitive internals, `DAT_00ABE028` full UI lifecycle.
- **Evidence needed to mark COMPLETE:** passability gate body; candidate-gather loop with all guards; scoring formula with distance computation and cross-region bonus; selector's first-pass (pair scan) vs second-pass (max-min-distance) algorithm; RNG draw order; waypoint write path; success/failure return semantics.
- **Stop conditions:** all three functions decoded or remaining uncertainty explicitly named.

---

## 1. Overview

`0x00594870` is the per-region candidate gatherer and waypoint writer:
1. RNG-draws a lane index (0–2) and derives an iteration limit.
2. Inner loop: RNG-draws a cell index from the region's candidate list, applies a 6×6 passability gate (`0x005A7250`) and a view-region bounds check (`MapClass__IsCellInViewRegion`), and accumulates passing cells into a local buffer.
3. Calls `0x00594F40` to pick the best candidate from the buffer.
4. If `0x00594F40` returns non-null, writes waypoint slots (via `FUN_0068BF50`) and marks cell bits; if null, no waypoints are written.

`0x00594F40` is the best-first selector:
- First pass: if there are ≥2 candidates, scan all candidate pairs and keep the index of the pair with the **maximum** Euclidean distance (+ optional cross-region bonus). The candidate from that pair becomes the first seed.
- Second pass: iteratively pick the candidate that maximises **minimum distance** to all already-selected candidates (also with cross-region bonus), until `local_34` slots are chosen.

`0x005A7250` is the 6×6 passability gate:
- Walks all cells in a `(start_x - 3, start_y - 3, 6, 6)` window (verified from `-3` offsets in ASM at `0x00594973..0x00594976` and `0x6` constants).
- Checks each cell for roads/paved tiles, water/special tiles, and `CellClass__IsClearTile`.
- Returns 0 immediately on any blocking cell; returns 1 only if the entire 6×6 window is clear.

Active in YR: Yes, conditional. Evidence: decompile `0x00594870`; assembly `0x00594870..0x00594b4d`; decompile `0x00594F40`; decompile `0x005A7250`.

---

## 2. `FUN_00594870` — Candidate Gather Loop (Detailed)

### 2.1 Lane / iteration-limit RNG draw

At function entry (verified via assembly `0x005948cd..0x005948f1`):
```
MOV ECX, 0xABE890          ; RNG instance
CALL Random__Next           ; draw random uint
FILD [result] ; push to FPU
FMUL [0x7ED8C0]             ; scale by 1/RAND_MAX equivalent
CALL Math__ftol             ; convert float → int
CMP EAX, 2                  ; retry if > 2
JA  retry_draw
```
Result `uVar2` is in `0..2`. The outer loop limit (`EDI`) is computed as:
```
ECX = param_1[8]            ; region quota (number of starts for this bucket)
EDI = uVar2 + ECX * 15
```
Iteration count = `random_lane (0–2) + quota × 15`. Maximum iterations: `2 + quota×15`. Loop also caps at 300 absolute (`CMP dword ptr [ESP+0x14], 0x12C`).

Active in YR: Yes, conditional. Evidence: assembly `0x005948cd..0x005948f3`; `0x00594910 CMP dword ptr [ESP+0x14], 0x12c`.

### 2.2 Inner candidate-draw RNG

For each iteration (verified `0x00594926..0x0059495c`):
```
ESI = param_1[0x38] - 1     ; region candidate count − 1 (upper bound)
ECX = region_candidate_count ; for float multiply
Call Random__Next
FILD [result]
FMUL [ECX as float]          ; scale to [0, count)
FMUL [0x7ED898]
CALL Math__ftol
CMP EAX, ESI                 ; retry if > upper bound
JA  retry_inner_draw
```
Result: uniform random index into the region's candidate-cell array at `param_1[0x2c + index*4]` (`0x0059495e..0x00594967`).

Active in YR: Yes, conditional. Evidence: assembly `0x00594925..0x0059495c`.

### 2.3 Passability gate — `FUN_005A7250` (6×6 window)

Call site (verified `0x0059498a..0x00594997`):
```
; Set up rect: (cell_x - 3, cell_y - 3, 6, 6)
SUB EAX, 3      ; x origin
SUB ECX, 3      ; y origin
MOV [rect+0],EAX ; start_x
MOV [rect+2],ECX ; start_y (word)
MOV [rect+4], 6  ; width
MOV [rect+6], 6  ; height
CALL FUN_005A7250
TEST AL, AL
JZ skip          ; candidate rejected
```

`FUN_005A7250` itself (decompile verified; semantics CORRECTED 2026-07-20 — the original
version of this section had the pave polarity backwards and mislabeled `DAT_00ABBEC4`):
the signature is `__fastcall (rect*, char road_policy, char road_end_policy)`; the
0x00594870 call site passes both policy chars as 0. Before the cell walk, all four rect
CORNERS must pass the diamond-band test (`DAT_00ABED04/08`), else return 0. Then for
each of the 36 cells (`MapClass__Get_CellClass` per cell):
1. `FUN_004866D0` — tile in `[g_PavedRoads, +15)` → verdict = `road_policy` (0 here → reject).
2. `FUN_004866F0` — tile in `[DAT_00ABBEC4, +4)` — `DAT_00ABBEC4` is **PavedRoadEnds**
   (theater `[General]` key, verified via `disassemble_bytes 0x00545ec0` against the
   `Read_Theater_TileSets_INI` key table), NOT a water set → verdict = `road_end_policy`
   (0 here → reject).
3. `CellClass__IsClearTile()` true → cell **passes**.
4. else `FUN_00486650` tile in `[g_MiscPaveTile, +14)` → cell **passes**.
5. else verdict = `FUN_00486670` tile in `[g_PaveTile, +16)` → **passes** if pave,
   rejects otherwise.

Net effect: every covered cell must be clear, misc-pave, or pave; roads and road-ends
reject under the zero policies; everything else (green LAT, shore, water, rough)
rejects via the fall-through. Any rejection returns 0 immediately; 1 only if all 36
cells pass. Evidence: `decompile_function 0x005a7250` (2026-07-20).

Active in YR: Yes, conditional. Evidence: decompile `0x005A7250`; decompile `0x004866D0`, `0x004866F0`, `0x00486650`, `0x00486670`.

### 2.4 View-region bounds check

After passability gate, `MapClass__IsCellInViewRegion` is called with a margin struct built from `DAT_0087f8e8..0x0087f8f0` (map boundary globals, +4/-8 inset). If out of bounds, candidate is rejected. Evidence: assembly `0x005949a5..0x005949b3`.

### 2.5 Buffer admission and cap

After both gates pass (verified `0x005949b5..0x005949fb`):
- If `local_8 < local_10` (current count < buffer capacity), OR if buffer can be grown via vtable `+0x8` call, candidate is appended to the buffer and `local_8` incremented.
- Buffer capacity starts at 10 (`local_4 = 10`), growth-capable via the vtable path.

### 2.6 After gather: call `FUN_00594F40`

At `0x00594a13..0x00594a21`: after the gather loop, `FUN_00594F40` is called with `(this=param_1, buf=buffer_ptr)`. Return is tested for null/non-null. Zero → no waypoints written; non-zero → write path. Evidence: assembly `0x00594a13..0x00594a23`.

### 2.7 Waypoint write path

On `FUN_00594F40` success (`EDI != 0`, verified `0x00594a23..0x00594ae1`):
- Fills `ScenarioClass+0x11C0..0x11DC` range with `DAT_00ABE300` (init/clear value): loop `0x11c0..0x11e0` step 4.
- For each of `param_1[8]` quota slots (index `0..quota-1`):
  - Calls `FUN_0068BF50(index + param_2, candidate_cell)` → writes `ScenarioClass+0x632 + (index+param_2)*4`.
  - Calls `FUN_0068BCC0` to read back the packed cell.
  - Calls `MapClass__Get_CellClass` on the packed cell, ORs `CellClass+0x140 |= 4` (start-cell marker bit).
  - Stores the cell into `ScenarioClass+0x11BC + (i+1)*4`.

Active in YR: Yes, conditional. Evidence: assembly `0x00594a58..0x00594ae1`; decompile `0x00594870`; decompile `0x0068BF50`/`0x0068BCC0`.

---

## 3. `FUN_00594F40` — Best-First Selector (Detailed)

### 3.1 Capacity/threshold computation

Entry (verified `0x00594f40..0x00594f92`):
```
FILD [DAT_00ABE030]         ; load candidate-list capacity (likely = 10 or buffer actual count)
FMUL [0x007E3808]           ; scale constant A
; ...
FMUL [0x007ED8D8]           ; scale constant B
FIDIV [DAT_00ABE028]        ; divide by start quota
FADD [0x007E1708]           ; add offset constant
FIMUL [ESI]                 ; multiply by region quota (param_1+0x20)
CALL Math__ftol             ; → local_34 = selection target count
```
`local_34` = target number of candidates to select for this region, derived from:
`floor((DAT_00ABE030 × A × B / DAT_00ABE028 + C) × region_quota)`

Constants RESOLVED 2026-07-20 (`read_memory 0x007e3808 / 0x007ed8d8 / 0x007e1708`):
A = `0.01`, B = `12.0`, C = `2.0`. `DAT_00ABE030` is the **TiberiumLayout** option
(cross-confirmed by `RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK_GHIDRA_REPORT.md` §6),
so `local_34 = trunc((TibLayout×0.01×12/quota_total + 2)×region_quota)`.

If `local_34 == 0` and `param_1+0x20 == 0`, returns null (no selection possible). Evidence: decompile `0x00594F40`; assembly `0x00594f40..0x00594f92`.

### 3.2 First pass — pair scan (seed selection)

If `candidate_count >= 2` (decompile `0x00594F40` body, outer loop over pairs): for each pair `(i, j)` where `j > i`:
1. Look up region zone ID for both cells using `(y * DAT_0089C2DC + x) * 0x50 + 0x38 + DAT_00ABED10`.
2. Compute Euclidean distance: `sqrt((dx*dx) + (dy*dy))`.
3. Apply cross-region bonus: `if zone_i != zone_j: distance += 20` (value `0x14` = 20).
4. Track the pair with maximum adjusted distance; record `local_20 = i` (index of the first candidate in the best pair).

The best-pair first candidate is pushed into the output list immediately.

Active in YR: Yes, conditional. Evidence: decompile `0x00594F40`; `(float10)(-(iVar11 != local_24) & 0x14) + fVar12` — the `-(-1) & 0x14 = 0x14 = 20` bonus pattern.

### 3.3 Second pass — max-min-distance greedy selection

Loop runs until `output_list.count >= local_34` (decompile, `LAB_0059519b` loop):
- For each remaining candidate `c`:
  - Initialize `min_dist = 9999999.0`.
  - For each already-selected candidate `s`: compute `sqrt(dx²+dy²) + cross_region_bonus`; track minimum.
  - Score of `c` = its minimum distance to any selected candidate.
- Push the candidate with maximum score (max of min distances) into the output list.

This is the classic **furthest-point-first** / **Farthest-First Traversal** algorithm.

Active in YR: Yes, conditional. Evidence: decompile `0x00594F40`; `local_30 = 9999999.0` init; `if (local_10 < local_30) { local_10 = local_30; local_24 = local_3c; }` greedy pick.

### 3.4 Single-candidate fallback

If `candidate_count == 1`: the sole candidate is pushed directly into the output list (no pair scan). Evidence: decompile path when `iVar8 == 1`.

### 3.5 Return semantics

Returns `local_38` (output list pointer, allocated with `operator_new(0x18)`) on success. Returns null (`0`) if no candidates or `local_34 == 0`. The output list is heap-allocated; caller (`0x00594870`) is responsible for freeing it (call to `FUN_007c8b3d` / `operator_delete`). Evidence: decompile `0x00594F40`; assembly `0x00594fca..0x00594fda` (heap alloc) and `0x00594fbb XOR EAX,EAX; RET 0x4` (null path).

---

## 4. RNG Consumption Order

Within one call to `0x00594870` for a single region:
1. **Draw 1:** lane index (0–2) — one `Random__Next` call plus `Math__ftol`.
2. **Per iteration (up to 300):** **Draw 2:** candidate cell index — one `Random__Next` call plus `Math__ftol`.
3. No RNG consumed inside `0x00594F40` (pure deterministic scoring).

Active in YR: Yes, conditional. Evidence: decompile `0x00594870`; assembly confirms only two `CALL 0x0065c780` sites.

---

## 5. Key Globals / Offsets

| Global / offset | Meaning | Evidence |
|---|---|---|
| `DAT_00ABE030` | Candidate buffer capacity (used in `0x00594F40` threshold formula) | `0x00594f49 FILD dword ptr [0x00abe030]` |
| `DAT_00ABE028` | Start quota (total starts to generate); used as divisor in threshold formula | `0x00594f7d FIDIV dword ptr [0x00abe028]` |
| `DAT_0089C2DC` | Map width (cells per row); used in zone ID lookup `(y * width + x) * 0x50 + 0x38` | assembly `0x00595044 IMUL EAX,[0x0089c2dc]` |
| `DAT_00ABED10` | Region array base pointer (0x50-stride); zone field at `+0x38` | decompile `0x00594F40`; `*(int *)((row * DAT_0089C2DC + col) * 0x50 + 0x38 + DAT_00ABED10)` |
| `param_1[0x2c]` (`EBX+0x2c`) | Pointer to region's candidate cell array | assembly `0x0059495e MOV EDX, dword ptr [EBX+0x2c]` |
| `param_1[0x38]` (`EBX+0x38`) | Region candidate cell count | assembly `0x0059491e MOV ESI, dword ptr [EBX+0x38]` |
| `param_1[0x20]` (`EBX+0x20`) | Region start quota for this bucket | assembly `0x005948f3 MOV ECX, dword ptr [EBX+0x20]`; `0x00594f51` |
| `CellClass+0x38` | Tile/terrain type ID; compared against road/water/pave ranges | decompile `0x004866D0`, `0x004866F0`, `0x00486650`, `0x00486670` |
| `CellClass+0x140` | Cell flags; bit 2 (`0x4`) set on chosen start cell | assembly `0x00594abd OR ECX,0x4` |
| `ScenarioClass+0x632 + index*4` | Waypoint packed cell storage | decompile `0x0068BF50`, `0x0068BCC0` |
| `ScenarioClass+0x11BC + (i+1)*4` | Start cell metadata mirror | assembly `0x00594ad1` |
| `g_PavedRoads` | Road tile range start (15 entries) | decompile `0x004866D0` |
| `DAT_00ABBEC4` | **PavedRoadEnds** tile range start (4 entries) — earlier "water/special" label was wrong | decompile `0x004866F0`; `disassemble_bytes 0x00545ec0` (writer in the theater key table) |
| `g_MiscPaveTile` | Misc pave tile range start (14 entries) | decompile `0x00486650` |
| `g_PaveTile` | Pave tile range start (16 entries) | decompile `0x00486670` |

---

## 6. Negative Facts / Do Not Do

1. **Do NOT implement a radius-exclusion distance threshold.** The engine has no minimum-distance cutoff; it uses a purely score-maximizing greedy algorithm. Any hardcoded "must be X cells apart" rule does not exist in `0x00594F40`. Evidence: decompile `0x00594F40` — no absolute threshold comparison on distance result.
2. **Do NOT apply the cross-region bonus as an additive score outside the `zone_i != zone_j` guard.** The `0x14` (+20) bonus is only added when both cells come from different zone IDs (`iVar11 != local_24`). Evidence: `(float10)(-(iVar11 != local_24) & 0x14)` pattern in decompile.
3. **Do NOT skip the 6×6 passability window.** All 36 cells in the window centered on the candidate must pass road/water/clear-tile checks. Using a smaller window (1×1 or 3×3) produces different results. Evidence: decompile `0x005A7250`; `-3` offsets and 6-step loop bounds at `0x00594973..0x00594986`.
4. **Do NOT consume extra RNG draws for scoring.** `0x00594F40` contains no `Random__Next` calls. Evidence: callees list for `0x00594F40`: no `0x0065c780`.
5. **Do NOT reuse the previous session's `0x005A7250` identification as a "cell passability check."** It is specifically a 6×6 footprint walk, not a single-cell check. The function takes `(cell_x, cell_y, width=6, height=6)` as its 4 packed arguments. Evidence: decompile body; parameter layout at `0x0059498a..0x0059498e`.

---

## 7. Implementation Handoff

### Handoff 1 — Candidate passability gate

- **Verified behavior:** Before admitting a candidate start cell to the buffer, the engine walks a 6×6 window centered at `(cell_x - 3, cell_y - 3)` and rejects the cell if any tile is a road, water, misc-pave, or pave tile, or if `CellClass__IsClearTile()` returns false. Evidence: decompile `0x005A7250`; assembly `0x00594973..0x0059498e`.
- **Rust delta:** No passability gate currently exists in the Rust RMG stub.
- **Affected surface:** `src/` — wherever start-position candidate filtering is implemented in the RMG.
- **Required implementation effect:** `fn is_start_candidate_passable(map, cell_x, cell_y) -> bool` that iterates the 6×6 window and checks tile-type ranges for each cell.
- **Acceptance scenario:** A cell adjacent to a road tile must be rejected; a cell with 6×6 open ground must be accepted.
- **Proposed Rust test name:** `test_start_candidate_passability_rejects_road_adjacent`
- **Risk:** tile-type range globals (`g_PavedRoads`, `DAT_00ABBEC4`, `g_MiscPaveTile`, `g_PaveTile`) must be loaded from the engine's tile-type init before RMG runs; verify initialization order.

### Handoff 2 — Best-first selector (max-min-distance greedy)

- **Verified behavior:** `0x00594F40` first seeds the output with the candidate from the maximum-distance pair (cross-region +20 bonus), then iteratively picks the candidate with the highest minimum-distance to all already-selected candidates, until `local_34` are selected. Evidence: decompile `0x00594F40` full body.
- **Rust delta:** No selector exists.
- **Affected surface:** RMG start-placement module.
- **Required implementation effect:** `fn select_start_positions(candidates: &[CellCoord], target: usize, zone_map: &ZoneMap) -> Vec<CellCoord>` with pair-scan seed + greedy max-min-dist loop. Cross-region bonus = 20.0 (integer cast from `0x14`).
- **Acceptance scenario:** Given 4 candidates in a line, the selector must choose the 2 most distant; given candidates in two different zones, the cross-region pair must win over same-zone pairs at equal distance.
- **Proposed Rust test name:** `test_start_selector_greedy_max_min_dist` and `test_start_selector_cross_region_bonus`
- **Risk:** `local_34` threshold formula depends on `DAT_00ABE030` identity (see Remaining Uncertainty below); use `min(target, candidates.len())` as a safe conservative fallback.

### Handoff 3 — RNG draw order

- **Verified behavior:** Two RNG draws per candidate attempt: draw 1 (lane index, 0–2) once per region call; draw 2 (cell index within region) per iteration. No RNG in the scorer. Evidence: two `CALL 0x0065c780` sites in `0x00594870`; zero in `0x00594F40`.
- **Rust delta:** No RNG draw sequencing exists for start-placement.
- **Affected surface:** RNG integration in RMG start-placement.
- **Required implementation effect:** Consume the lane-draw once at the top of the region loop, then one candidate-index draw per iteration. Do not draw additional RNG inside the scoring/selection pass.
- **Acceptance scenario:** Deterministic replay: same seed → same lane draw → same candidate-index sequence → same output (before passability filters).
- **Proposed Rust test name:** `test_rmg_start_rng_draw_order_deterministic`
- **Risk:** The RNG instance used is `0xABE890` (assembly `0x005948cd`, `0x00594935`) — confirm this is `Scen->Random` vs `g_MainRng` in the Rust RNG-routing table before wiring.

---

## 8. Remaining Uncertainty — RESOLVED 2026-07-20

All four items closed during Task 9 implementation:

- **`DAT_00ABE030` identity → RESOLVED:** it is the **TiberiumLayout** option, not a
  buffer capacity (see §3.1 and the tiberium recheck doc §6).
- **`CellClass__IsClearTile` → RESOLVED:** `0x00486380`; tile index 0 or the 0xFFFF
  sentinel only (matches the Rust `TileIds::is_clear`). Evidence:
  `disassemble_bytes 0x005a2220` call site; prior tiles decode.
- **`0x007E3808` / `0x007ED8D8` / `0x007E1708` → RESOLVED:** `0.01`, `12.0`, `2.0`
  (`read_memory`, see §3.1).
- **RNG instance `0xABE890` → RESOLVED:** it is the map-generator RNG used by every
  RMG phase (`MOV ECX,0xabe890` before each `CALL 0x0065c780` at `0x00594ea1`,
  `0x005948cd`, `0x00594935`), i.e. the Rust `RmgRng` stream.

New facts found during implementation (evidence inline):

- **`Sqrt_Approx` (0x004CAC40) is a table square root, not FSQRT:** input narrowed to
  f32, mantissa bucketed by its top 14 bits (implicit bit folded in for odd
  exponents), result mantissa from the 16384-entry dword table at `0x008650BC`,
  exponent halved arithmetically. The retail table equals
  `trunc((sqrt(bucket_start)−1)·2^23)` for all 16384 entries (verified byte-exact
  against the retail PE, exhaustive). Rust: `src/map/rmg/sqrt_approx.rs` generates the
  table and golden-tests it against `ini/sqrt_table.bin` (retail dump).
- **The +20 cross-region bonus is inert on this path:** the zone stamps `0x00594F40`
  reads at scratch `+0x38` are the region ids written by the starts-phase rebuild
  (`0x00594420`), and one gatherer call draws candidates from a single region's cell
  list — the stamps always match. The mechanism is still modeled.
- **The leftover selection becomes the region's field-slot list:** after writing
  `quota` waypoints, `0x00594870` removes the first `quota` entries of the selector
  output and stores the surviving list object at region `+0x00`
  (`*param_1 = list`), which the tiberium phase consumes. Evidence: decompile
  `0x00594870` tail (shift-down loop + store).
- **The lane-draw scale `[0x007ED8C0]`** reads `0x3E08_0000_0018_0000`
  (`read_memory 0x007ed8c0`): the usual perturbed-mantissa ~3·2⁻³².

---

## 9. Stale-Doc Update for OQ-11

The parent doc (`docs/research/skirmish-ui/RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md`) §9 OQ-11 should be updated:

**Replace:**
```
- `[DEFERRED] OQ-11 - Exact candidate scoring/passability in `0x00594870` / `0x00594F40`.` (category: out-of-scope; reason: this slot only claims count/success/metadata contract; next-step-if-pursued: dedicated start candidate scoring report)
```
**With:**
```
- `[RESOLVED] OQ-11 - Exact candidate scoring/passability in `0x00594870` / `0x00594F40`.` (resolved in `RMG_START_POINT_SCORING_00594870_GHIDRA_REPORT.md`; passability gate is a 6×6 window walk rejecting road/water/pave/non-clear tiles; selector uses max-min-distance greedy algorithm with cross-region +20 bonus; two RNG draws per candidate attempt; no minimum-distance threshold)
```
