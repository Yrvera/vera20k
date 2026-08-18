# RMG Tiberium Field Creation — `FUN_005a23a0` Ghidra Research Report

**Address(es):** `0x005a23a0`–`0x005a289d` (outer driver); `0x005a28c0`–`0x005a2ec8` (field BFS placer; RET 0x10 at `0x005a2ec8`, corrected from 0x005a2e60 per 2026-07-19 audit)  
**Investigation Mode:** full-decompile + assembly verification  
**Claimed Scope:** Ore/gem field placement on a freshly generated RMG map — how many fields, where (relative to regions/start positions), field size/density growth, RNG draw order, INI inputs, and resulting cell-overlay writes.  
**Non-Scope:** water seeding, region partition, start-point placement, hills, RNG primitive internals, runtime tiberium growth/spread queues (those are in `sim/`).  
**Active in YR:** Conditional — active whenever `FUN_00598960` generates a random map from a `.SED` seed file.

> **2026-07-20 correction pass.** This doc was audited **RED** on 2026-07-19 (see `docs/research/AUDIT_LOG.md`): the §4/§5 field-count formula was fundamentally wrong (no RNG is involved; the real formula lerps MapSeed+0x54 between +0x2BC/+0x2C0), the §6 gate identities for `0x00578460`/`0x00486380` were swapped and the admission logic mis-stated, and §7's gating/parameters were wrong. §§4–7 and the affected §9/§11 rows below were rewritten from a fresh full-disassembly re-derivation — every corrected claim cites its Ghidra call inline. Full recheck evidence: `RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK_GHIDRA_REPORT.md` (same folder). Sections the audit confirmed (overlay bases, density cap, RNG instance routing, bVar12 switch, TIBTRE ore-only gate, restart limit) are left untouched except where a bad address citation needed fixing.

---

## Investigation Pre-flight

- **Target question:** Decode `FUN_005a23a0`: field count per region, placement anchor, size/density BFS growth, RNG draw order, overlay type selection.
- **Non-goals:** Do not decode water, hills, LAT, or runtime spread/growth queues.
- **Evidence needed to mark COMPLETE:** verified field-count formula, placement anchor, BFS overlay write, RNG instance, overlay type draw.
- **Stop conditions:** Stop at one callee level deep; record deeper helpers as Remaining Uncertainty.

---

## 1. Overview

`FUN_005a23a0` is the **RMG tiberium placement driver**. It iterates every region, computes a per-field size budget (§4), dispatches the field BFS placer `FUN_005a28c0` once per field slot, then runs an unconditional second pass placing one compensation field at every start position (gem-typed only under the §7 flag). Both the driver and placer consume `Random__Next @ 0x0065c780` exclusively via `ECX = 0x00ABE890` — the **`g_MapGenRng` instance** seeded from `MapSeed+0x74`.

Active in YR: Conditional (verified via decompile_function 0x005a23a0 + xref chain from FUN_00598960 at 0x00598ef2).

---

## 2. MapSeed Field Inputs

| MapSeed field | Offset | Role in this function |
|---|---|---|
| `+0x3C` | theater/map-type | Gem-field flag: see §3; also gem second-pass flag: see §7 |
| `+0x40` | Resources option | Gem-field flag: see §3; also gem second-pass flag: see §7 |
| `+0x38` | theater flag | Gem-field flag: see §3 |
| `+0x54` | **Tiberium option (percent 0..100)** | Lerp parameter of the field-count formula: see §4 (verified via disassemble_function 0x005a23a0, `005a2558: FILD [ESI+0x54]`, 2026-07-20) |
| `+0x2BC` | RMGMinimumTiberium | Lerp lower bound: see §4 (asm `005a254c`, 2026-07-20) |
| `+0x2C0` | RMGMaximumTiberium | Lerp upper bound: see §4 (asm `005a2552`, 2026-07-20) |
| `+0x58` | TiberiumLayout | **NOT consumed here.** Its only reader is `FUN_00594f40` (field-slot selection during start generation), where it scales the number of field-slot positions per region — see recheck report §6 (verified via get_xrefs_to 0x00abe030 → sole READ at 0x00594f49, 2026-07-20) |

Accessed via `param_1` which is passed from `FUN_00598960` as the global `MapSeedClass*` (verified via decompile_function 0x005a23a0, asm `005a23b4: MOV EAX,[ESI+0x3c]`; `005a23c7: MOV ECX,[ESI+0x40]`).

---

## 3. Gem-Field Flag (`bVar12`)

At function entry a boolean `bVar12` is computed from `MapSeed+0x3C` (map type) and `MapSeed+0x40` (Resources option):

```
switch(MapSeed[+0x3C]):
  case 0:  bVar12 = (MapSeed[+0x40] == 3)     // land map, Resources==3 → gems
  case 1,3,4: bVar12 = (MapSeed[+0x40] != 3)  // water/island types → gems unless Resources==3
  case 2:  bVar12 = true                       // always gems
  default: bVar12 = false
```

This flag controls whether the second gem-field quality loop runs. Active in YR: Yes (verified via decompile_function 0x005a23a0 asm `005a23ba: CMP EAX,0x4`; `005a23c7: JMP [EAX*4+0x5a28a0]`; case dispatch `005a23ce–005a23e2`).

---

## 4. Region Iteration — Field Count Calculation

`FUN_005a23a0` loops over all `DAT_00abdfa0` regions using the pointer array at `DAT_00abdf94`. For each non-null region (`*piVar1 != 0`):

**Region struct offsets accessed (EDI = region pointer):**

| Offset | Meaning | Evidence |
|---|---|---|
| `+0x00` | pointer to sub-struct `region_sub` (field-slot coord array at `+0x4`, field-slot count at `+0x10`; produced by `FUN_00594f40` during start generation) | asm `005a24cf: MOV ECX,[EDI]`; `005a25a7: MOV ECX,[ECX+0x10]`; `005a2622: MOV ECX,[EAX+0x4]` (verified via disassemble_function 0x005a23a0 + get_function_callers 0x00594f40, 2026-07-20) |
| `+0x20` | number of start positions in this region | asm `005a242b: MOV EAX,[EDI+0x20]`; `005a257a: FILD [EDI+0x20]` |
| `+0x2C` | pointer to a packed-coord array (random reference-point pick when the region has no starts) | asm `005a24bd: MOV ECX,[EDI+0x2c]; MOV EDX,[ECX+EAX*4]` (verified via disassemble_function 0x005a23a0, 2026-07-20) |
| `+0x38` | length of the coord array at `+0x2C` (range of the random pick) — NOT a "start index offset" as previously claimed | asm `005a247d: MOV ESI,[EDI+0x38]; DEC ESI` used as the rejection bound `EAX > ESI → redraw` at `005a24b9` (verified via disassemble_function 0x005a23a0, 2026-07-20) |

**Field count formula (corrected 2026-07-20 — verified via disassemble_function 0x005a23a0, block `005a2548`–`005a25bb`, + read_memory 0x007e3808/0x007e1738/0x00822d80):**

The formula involves **NO `Random__Next` draw**. All `ftol` calls (`0x007c5f00`) truncate toward zero (FPU control word `DAT_00822d80 = 0x0E7F`, RC=11; verified via disassemble_function 0x007c5f00 + read_memory 0x00822d80, 2026-07-20):

```
min  = MapSeed[+0x2BC]                        ; RMGMinimumTiberium
span = MapSeed[+0x2C0] - min                  ; RMGMaximumTiberium - min
lerp = trunc( (double)MapSeed[+0x54] * 0.01 * (double)span + (double)min )
       ; FPU order: FILD +0x54 → FMUL 0.01 (@0x007e3808) → FILD span → FMULP → FIADD min → ftol
mult = max( (double)region[+0x20], 0.5 )      ; 0.5 @ 0x007e1738 — floor for 0-start regions
regionTotal = trunc( (double)lerp * mult )    ; NOTE: lerp is already truncated BEFORE this multiply
fieldCount  = region_sub[+0x10]
if (fieldCount == 0 || regionTotal == 0) → skip region        ; asm 005a25aa-005a25b4
perFieldBase = regionTotal / fieldCount       ; CDQ/IDIV, signed truncating     ; asm 005a25ba-005a25bb
```

Each field slot `i` then gets `size_i = trunc(perFieldBase + jitter_i)` where `jitter_i` is a Gaussian draw × 50.0 rejection-resampled into [−100, +100] (constants @ 0x007e4f50 / 0x007eda88 / 0x007e2ac0, verified via read_memory 2026-07-20); a negative `size_i` skips that slot's placer call (asm `005a2608: CMP EAX,EBX; JL`).

**Gem-anchor slot selection (`local_6c`) (corrected 2026-07-20):**

When `bVar12` is true, exactly one pass-1 field per region is flagged gem. The reference point is the **component-wise average of the region's start waypoint coordinates** (word sums in BX/BP, one IDIV per axis; asm `005a243b`–`005a2479`) — not "the closest start to region center". When the region has no starts, the reference point is instead a random entry of `region[+0x2C]` picked via `trunc(rand × region[+0x38] × 1/(2^32−1))` with rejection while out of range (the sole `Random__Next` in the driver, `ECX=0xabe890` at `005a2494`–`005a2499`; scale constant @ 0x007ed898 = 1/(2^32−1), verified via read_memory 2026-07-20). `local_6c` is then the index of the **field slot** (not start) nearest that reference point by `Sqrt_Approx` distance, min-tracked from 500000 (asm `005a24cf`–`005a2546`); it is compared against the loop index to form the placer's `is_gem` flag (asm `005a2614: CMP ESI,EDX; SETZ DL`). All verified via disassemble_function 0x005a23a0, 2026-07-20.

---

## 5. RNG Draw Order (Outer Driver)

Per-region RNG draw sequence in `FUN_005a23a0` (corrected 2026-07-20 — the driver contains exactly ONE direct `Random__Next` call site, at `005a2499`; verified by scanning the full disassemble_function 0x005a23a0 output for `CALL 0x0065c780`, 2026-07-20):

1. **Reference-point random pick** (only when `bVar12=true` AND the region has zero start positions): one or more `Random__Next @ ECX=0xabe890` draws, each scaled by `region[+0x38] × 1/(2^32−1)` and rejection-resampled while out of range, to pick a random entry of `region[+0x2C]`. Verified asm `005a2494: MOV ECX,0xabe890; 005a2499: CALL 0x0065c780; 005a24b9: CMP EAX,ESI; JA 005a2494` (disassemble_function 0x005a23a0, 2026-07-20).
2. **Per-field Gaussian jitter** via `FUN_005980c0 @ ECX=0xabdfb8`, one-or-more pairs per field slot (rejection-resampled until `jitter×50 ∈ [−100, +100]`). Verified asm `005a25c9: MOV ECX,0xabdfb8; CALL 0x005980c0`; rejection compares at `005a25dd`/`005a25ee` (disassemble_function 0x005a23a0 + read_memory 0x007e4f50/0x007eda88/0x007e2ac0, 2026-07-20).

   > `FUN_005980c0` is a normal-distribution approximation: pairs of `Random__Next` draws mapped to `[-1,1]`, rejected if `r²>=1`, then scaled by `sqrt(-2*ln(r²)/r²)`. Its RNG calls use a vtable pointer stored at `0x00abdfb8+0x10`, which is the same `g_MapGenRng` stream copied there at generator entry (verified asm `005989a6: MOV EDI,0xabdfb8`; `005989c9: MOVSD.REP`).

3. **No RNG in the field-count formula** (§4), **none in the gem scoring loop** (§7 — it is pure `Sqrt_Approx` accumulation, asm `005a26b2`–`005a26fa`), and **none in the gem size computation** (`ftol((score−min)×15)+500`, asm `005a27f4`–`005a280a`). The remaining draws of the tiberium stage happen inside the placer `FUN_005a28c0` (tree variant, density value, priority jitter — all `ECX=0xabe890`; see §6).

---

## 6. `FUN_005a28c0` — Field BFS Placer

Signature: `void __stdcall FUN_005a28c0(short *origin_cell, int target_size, int field_id, char is_gem_start)` — RET 0x10; incoming ECX is never read (first ECX use at `005a28ee` is a write), so the `MOV ECX` before its call sites is dead (verified via disassemble_function 0x005a28c0, 2026-07-20).

This is the **ore/gem field grower**. It uses a priority-queue BFS starting from `*origin_cell` to expand an ore blob up to `target_size` cells. Two distinct cell arrays are involved: the **RMG scratch array** `DAT_00abed10` (stride 0x50; `+0x38` region id, `+0x3C` field-claim id, `+0x45` blocked flag) and the **real `CellClass`** obtained via `MapClass__Get_CellClass @ 0x005657a0` on the MapClass instance `0x0087f7e8` (`+0x44` overlay dword, `+0x11E` density byte). Overlay/density writes land on the real CellClass; claims land in the scratch array.

**Key mechanics (corrected 2026-07-20 — verified via decompile_function + disassemble_function 0x005a28c0):**

| Mechanism | Detail | Evidence |
|---|---|---|
| Heap allocation | `operator_new(target_size * 0x50)` for cell records; priority queue `operator_new(0x14)` with capacity `target_size * 10` | asm `005a28d4–005a28e2`; `005a28ee` |
| Cell-flood claim sentinel | Writes `param_3` (field_id) to `scratch[coord][+0x3c]` in `DAT_00abed10` at candidate-push time; the fresh-empty-cell admission requires `scratch[+0x3c] != param_3`. Claims are wiped map-wide at every seed (see restarts row). | asm `005a2d10–005a2d14` (check); `005a2dd1` (write) — disassemble_function 0x005a28c0, 2026-07-20 |
| Gate 1 — playfield bounds | `MapClass__Is_Cell_In_Playfield @ 0x00578460` (ECX=`0x87f7e8`, args `&coord, 1`): isometric diamond test against MapClass `+0xF4/+0xFC/+0x100/+0x104/+0x108` with height adjustment. **This is NOT IsClearTile** (the pre-correction doc had the two identities swapped). | asm `005a2cb5`; identity verified via decompile_function 0x00578460, 2026-07-20 |
| Gate 2 — clear tile | `CellClass__IsClearTile @ 0x00486380` (ECX=cell from `0x005657a0`): returns 1 iff `cell[+0x38]` (tile index) `== 0` or `== 0xFFFF`. There is **no separate "IsPassable" call** at this site. | asm `005a2ccc` (Get_CellClass), `005a2cd9` (IsClearTile); identity verified via decompile_function 0x00486380, 2026-07-20 |
| Gate 3 — admission logic | `(cell[+0x44] == -1 AND scratch[+0x3c] != field_id)` → admit fresh empty cell; **OR** `(cell[+0x11e] < 11 AND CellClass__GetTiberiumType @ 0x00485010 != -1)` → admit existing-tiberium revisit. An empty-but-claimed cell falls into the second test and fails `GetTiberiumType != -1`, so the claim dedups empty cells. | asm `005a2ce6–005a2d31`; verified via decompile_function + disassemble_function 0x005a28c0, 2026-07-20 |
| Density write | If real `cell[+0x44] == -1`: rejection-draws `Random__Next @ 0xabe890` × `~12/2^32` (@`0x007eda90`) while result > 11 → `cell[+0x44] = draw + local_48` (draw ∈ [0,11]). Else if `cell[+0x11e] < 11`: `cell[+0x11e] += 1`; else skip. Placed-counter increments only on a write/increment. Both writes go to the **real CellClass**, not the scratch array. | asm `005a2c12–005a2c63`; read_memory 0x007eda90, 2026-07-20 |
| Write gate | The popped cell is only written at all if `scratch[cur][+0x45] == 0` (blocked flag byte, set upstream of this stage). | asm `005a2b39–005a2b3e` — disassemble_function 0x005a28c0, 2026-07-20 |
| `local_48` (density base) | `= (param_4 != 0) ? 0x66 - 0x4B : 0x66` → 0x1B (gems) or 0x66 (ore). This is the **overlay type base**. Verified asm `005a294d–005a296e`: `NEG CL; SBB ECX,ECX; AND ECX,0xffffffb5; ADD ECX,0x66` (address range corrected 2026-07-20; formula unchanged) |
| Direction offsets | 8-neighbor scan from table `g_DirectionOffsets @ 0x0089f688` (word dx at +0, word dy at +2 per entry). Loop counter `& 7`. | asm `005a2c7c–005a2c95` |
| Priority key | `Sqrt_Approx(distance² to anchor) + Random__Next(0xabe890) × 5.0 × 1/(2^32−1)` — Euclidean distance to the **current anchor** (origin at seed time, rebound to the first written cell of each seed-generation) plus **uniform [0,5] jitter**. The placer never calls the Gaussian helper `FUN_005980c0` (the pre-correction "Gaussian jitter" claim was wrong). | asm `005a2d42–005a2db6`; constants read_memory 0x007ed7c0 (=5.0) / 0x007ed898 (=1/(2^32−1)), 2026-07-20; anchor rebind asm `005a2b50–005a2b57` |
| Repeat BFS restarts | Up to 10 queue seeds total (including the first): seed counter checked `CMP [ESP+0x2C],0xA; JGE break` at `005a29a1`. Every seed wipes `scratch[+0x3c]` for **all** map cells via the MapClass cell iterator (`0x00578350`/`0x00578290`) and reseeds from **the same `origin_cell` param** — not a new origin. | asm `005a29a1`; `005a29cc–005a2a56` — disassemble_function 0x005a28c0, 2026-07-20 (citation corrected from the invalid `005a2911`) |
| Tree object on ore anchor | When `param_4 == '\0'` (ore, not gem), once per seed-generation at the first written cell: draws `v = trunc(rand × ~3/2^32 (@0x007ed8c0) + 1.0 (@0x007e1718))`, rejects while `v > 3` → `v ∈ {1,2,3}`, then `sprintf("TIBTRE0%d", v % 10)` → **`TIBTRE01`..`TIBTRE03` (never `TIBTRE00`)** → `TerrainTypeClass` find (`0x0071dd80`) + `TerrainClass` ctor (`0x0071bb90`). Variant range corrected from "0..3" 2026-07-20. | asm `005a2b8e–005a2c05`; string `0x0082c090` = `"TIBTRE0%d"`; constants read_memory 0x007ed8c0/0x007e1718, 2026-07-20 |

**Overlay type IDs:**

- Ore cells: `local_48 = 0x66 (102)` → overlay type base = 102 (ore variant 0). Density in `[0x66, 0x66+11] = [102, 113]`. Active in YR: Yes (verified asm `005a294d–005a296e`).
- Gem cells: `local_48 = 0x1B (27)` → overlay type base = 27 (gem variant 0). Density in `[0x1B, 0x1B+11] = [27, 38]`. Active in YR: Conditional on `param_4 != 0` (verified same asm).

The overlay value is written to the **real** `CellClass+0x44` (cell pointer from `MapClass__Get_CellClass @ 0x005657a0`, ECX=`0x87f7e8`) via `005a2c4c: MOV [EDX+0x44], EAX`; the field-claim sentinel is written to the **scratch** array `DAT_00abed10[(y * width@0x0089c2dc + x) * 0x50 + 0x3c]` via `005a2dd1` (targets corrected 2026-07-20 — the pre-correction text placed the `+0x44` write in the scratch array; verified via disassemble_function 0x005a28c0).

---

## 7. Second Loop — Per-Start Compensation Fields (corrected 2026-07-20)

After the first pass, `FUN_005a23a0` runs a second placement loop over all start positions in the region (`region[+0x20]` iterations). **It runs unconditionally for every region that entered placement — there is no `bVar12` gate** (control falls straight from the pass-1 loop into `005a263a`; verified via disassemble_function 0x005a23a0, 2026-07-20). The pre-correction claims "when bVar12=true", "minimum-distance starts only", and "param_3 = start_index + 0x1f4" were all wrong.

1. For each start `s`: look up its waypoint coordinate via `FUN_0068bcc0(ECX=[0x00a8b230], globalStartBase+s)` and compute `score_s` = mean Euclidean distance from the start to **all** of the region's field slots: `Σ Sqrt_Approx(dist²) / fieldCount` — sum first, one `FDIVR` (asm `005a26b2`–`005a2719`). No RNG.
2. Scores are appended to a heap-backed double array `local_1c` (growth plumbing, not a filter).
3. `minScore` = minimum over the scores, init `9999999.0` (`0x416312CFE0000000`, asm `005a2788`–`005a27bc`).
4. `gem2 = (MapSeed[+0x40] == 3) && (MapSeed[+0x3C] ∈ {1,3,4})` (asm `005a27c2`–`005a27e1`) — note this is a configuration in which `bVar12` is **false** for map types 1/3/4, so gem pass-2 fields and the pass-1 gem anchor are mutually exclusive on those types.
5. For **every** start `s`: `size = trunc((score_s − minScore) × 15.0) + 500` — the `ADD EAX,0x1f4` at `005a280a` lands on the **size**, not the field id (15.0 @ `0x007ed7b0`, verified via read_memory 2026-07-20); `field_id = globalStartBase + s + 1` (asm `005a2806: LEA ECX,[EBP+0x1]`); origin = the start's own waypoint cell; then `FUN_005a28c0(origin, size, field_id, gem2)`. The start closest (on average) to the field slots gets exactly 500; farther starts get proportionally more — a fairness compensation.
6. After the region: `globalStartBase += region[+0x20]` (asm `005a2839`–`005a2846`), so pass-1 field ids (`base+i+1` over slots) and pass-2 ids (`base+s+1` over starts) overlap numerically within a region — harmless, because every placer call wipes all scratch `+0x3C` claims at its first seed and ids are only compared within one placer run.

Active in YR: Conditional on RMG launch only (verified asm `005a263a–005a2846`; `005a2809: PUSH EBX (gem2)`; `005a280a: ADD EAX,0x1f4` — disassemble_function 0x005a23a0, 2026-07-20).

---

## 8. Cell Array Access Pattern

The `DAT_00abed10` global is the RMG scratch cell array. Cell stride is `0x50` bytes. A cell at `(x, y)` is at:

```
cell_ptr = DAT_00abed10 + (y * DAT_0089c2dc + x) * 0x50
```

Active in YR: Yes (verified asm `005a2cf1–005a2dd1`; `005a2d09: IMUL EAX,[0x0089c2dc]`; `005a2d0d: SHL EAX,0x4 (×16 = 0x50/5*16=0x50)` via `LEA [EAX+EAX*4]; SHL 4`).

---

## 9. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed Rust test name | Risk |
|---|---|---|---|---|---|
| Ore field placer BFS starts from a start-position-derived origin cell; field_id written to `cell[+0x3c]` prevents revisit; `cell[+0x44]` gets overlay type 102..113 (ore) or 27..38 (gem); density increments `cell[+0x11e]` up to 11 | Implement `rmg_place_ore_field(origin: CellCoord, target_size: u32, field_id: u32, is_gem: bool, rng: &mut MapGenRng, cells: &mut CellGrid)` using 8-neighbor priority BFS | `src/map/rmg_tiberium.rs` (new file) | After `FUN_005a23a0` completes, all non-water cells adjacent to start positions have overlay-type values in [27,38] or [102,113]; cell density field ≤11 | `test_rmg_ore_bfs_density_capped_at_11` | Overlay type offset 0x44 vs 0x11e must match CellClass layout verified against live binary |
| All `Random__Next` draws in `FUN_005a23a0` and `FUN_005a28c0` use `ECX=0xabe890` = `g_MapGenRng`; the Gaussian helper at `0xabdfb8` is also seeded from the same stream (REP MOVSD copy at generator entry) | RMG tiberium placement MUST draw from `MapGenRng`, not `ScenarioRng` or `MainRng` | `src/map/rmg_tiberium.rs` | Seeded-RNG smoke test: same seed → same field overlay pattern | `test_rmg_ore_deterministic_from_seed` | Any mismatch in RNG routing destroys lockstep replay correctness |
| `TIBTRE0%d` terrain object is spawned only on ore fields (not gem fields: `param_4==0` gate); variant **1..3** drawn via `trunc(rand × ~3/2^32 + 1.0)` rejection-resampled while `> 3`, then `% 10` (a no-op for 1..3) — corrected 2026-07-20, verified via disassemble_function 0x005a28c0 (`005a2b8e–005a2bd1`) + read_memory 0x007ed8c0/0x007e1718 | Spawn `TerrainClass(TIBTRE0{variant})` at ore cell origin only; gem origins get no tree | `src/map/rmg_tiberium.rs`, terrain spawner | Ore origin cell has TIBTRE object; gem origin cell has none | `test_rmg_ore_tree_only_on_ore_not_gem` | TIBTRE0 variant must be in [1,3] inclusive — `TIBTRE00` is never produced and does not exist in rulesmd TerrainTypes |

### Negative Facts / Do Not Do

1. **Do NOT use `ScenarioRng` or `MainRng` for placement draws.** All calls use `ECX=0x00abe890` (`g_MapGenRng`), verified at every `CALL 0x0065c780` site in both functions (asm `005a2494`, `005a2b8e`, `005a2c18`, `005a2d79`). Using the wrong RNG instance breaks determinism.

2. **Do NOT place gem trees (`TIBTRE0%d`).** The tree spawn is inside `if (param_4 == '\0')` — explicitly skipped for gem fields. Verified asm `005a2b75: MOV AL,[EBP+0x14]; 005a2b7e: TEST AL,AL; 005a2b88: JNZ 0x5a2c12` (jumps past tree allocation when `param_4` is non-zero; `TEST AL,AL` address corrected from `005a2b78` — that address holds `MOV dword ptr [ESI],0x0` — via disassemble_function 0x005a28c0, 2026-07-20).

3. **Do NOT write overlay density above 11.** The gate `CMP byte ptr [ECX+0x11e], 0xb; JNC skip` prevents incrementing beyond 11. The overlay type value from the Random draw is also clamped `CMP EAX, 0xb; JA retry`. Verified asm `005a2c3d`, `005a2c51–005a2c57`.

4. **Do NOT confuse this placement pass with runtime tiberium growth.** `FUN_005a23a0` writes the initial overlay type and density; the `TiberiumClass__InitGrowthQueues_All` and `TiberiumClass__InitSpreadQueues_All` calls that follow in `FUN_00598960` are separate queue initialization for runtime spread. The Rust `sim/` growth system (if it exists) models post-placement runtime; this report only covers generator-time seeding.

5. **Do NOT skip the up-to-10-seed limit in `FUN_005a28c0`.** At most 10 queue seeds (including the first) per field attempt, always from the same origin cell, with a map-wide scratch `+0x3C` claim wipe at each seed. Verified asm `005a29a1: CMP dword ptr [ESP+0x2C],0xA; JGE 0x005a2e9f` (citation corrected from the invalid `005a2911` via disassemble_function 0x005a28c0, 2026-07-20).

---

## 10. Remaining Uncertainty

- **`FUN_0068bcc0` start-waypoint lookup formula**: reads at `MapSeedClass[+0x632 + i*4]` (verified asm `005a244e: MOV ECX,[0x00a8b230]; CALL 0x0068bcc0`). The `+0x632` base is the waypoint coordinate table start within `MapSeedClass`. Exact start-coordinate layout and how start waypoints are seeded is documented in the sibling start-placement report (`RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md`).

- **`FUN_005adab0` (dynamic array init) and `FUN_005ad290` (resize)**: These manage the `local_1c` heap array used in the gem-quality scoring loop. Their internal logic is straightforward container helpers; exact capacity policy is unverified but does not affect the observable overlay output.

- **`FUN_005ad870` (priority-queue sift-down)**: Called inside `FUN_005a28c0` to maintain the BFS min-heap. Not decoded; only the caller behavior is verified.

- **`g_MapGenRng` Gaussian helper register (`0xabdfb8`)**: The REP MOVSD copy at `005989a6–005989c9` copies 6 dwords from the Stack-local struct into `0xabdfb8`, and that struct includes a vtable pointer `0x00598000` (verified asm `005989bf: MOV [ESP+0x2c], 0x00598000`). The vtable `+0x10` slot points to `Random__Next`. That `0x00598000` callback delegates to `g_MapGenRng @ 0xabe890`. VERIFIED: both Gaussian helper and direct `Random__Next` calls draw from the same `g_MapGenRng` stream (asm `005a25c9: MOV ECX,0xabdfb8; CALL 0x005980c0`).

- **Exact field count per region (divisor)** — RESOLVED 2026-07-20: `region_sub` (slot array at `+0x4`, count at `+0x10`) is the dynamic array returned by `FUN_00594f40` during start generation (`FUN_00594b50` → `FUN_00594870` → `FUN_00594f40`), which selects `≈ trunc((TiberiumLayout × 0.01 × 12.0 / NumPlayers@MapSeed+0x50 + 2.0) × region[+0x20])` slot coords via farthest-point sampling. Verified via get_xrefs_to 0x00abe030, disassemble_function 0x00594f40, get_function_callers 0x00594f40/0x00594870, 2026-07-20; details in `RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK_GHIDRA_REPORT.md` §6.

- **Scratch `+0x45` blocked flag**: gate on the pop-cell write path (`005a2b39`); which upstream stage sets it is untraced (NEW open item 2026-07-20).

- **Overlay type IDs 27 and 102 in INI**: The engine maps these integer overlay indices to `[OverlayTypes]` entries in `rulesmd.ini`. Verification that index 27 = gem and 102 = ore is an open INI-lookup task.

---

## 11. Coverage Ledger

| Area | Status | Evidence |
|---|---|---|
| `FUN_005a23a0` full decompile | Verified | decompile_function 0x005a23a0 |
| `FUN_005a28c0` full decompile | Verified | decompile_function 0x005a28c0 |
| `bVar12` gem-flag logic | Verified | asm 0x005a23ba–0x005a23e2 |
| RNG instance routing (`g_MapGenRng`) | Verified | asm 0x005a2494, 0x005a2b8e, 0x005a2c18, 0x005a2d79 all `ECX=0xabe890`; generator seeding 0x005989a6–0x005989c9 |
| Field-count formula (lerp of MapSeed+0x54 between +0x2BC/+0x2C0, ×max(starts,0.5), IDIV slot count; no RNG) | Verified (corrected 2026-07-20) | disassemble_function 0x005a23a0 (005a2548–005a25bb); read_memory 0x007e3808/0x007e1738/0x00822d80 |
| Overlay type base 0x66 (ore) / 0x1B (gem) | Verified | asm 0x005a294d–0x005a296e (`NEG CL; SBB; AND 0xffffffb5; ADD 0x66`; address range corrected 2026-07-20) |
| Density cap at 11 (`CellClass+0x11e`) | Verified | asm 0x005a2c3d, 0x005a2c51, 0x005a2c57 |
| Cell-overlay write to real `CellClass+0x44` (via 0x005657a0) | Verified (target corrected 2026-07-20) | asm 0x005a2c4c; `MOV [EDX+0x44], EAX`; disassemble_function 0x005a28c0 |
| Field-ID claim to scratch `DAT_00abed10[..]+0x3c` | Verified (target clarified 2026-07-20) | asm 0x005a2dd1; `MOV [EAX+ECX*1+0x3c], EDX` |
| Gate identities: 0x00578460 = playfield diamond, 0x00486380 = IsClearTile | Verified (corrected 2026-07-20 — was swapped) | decompile_function 0x00578460 / 0x00486380 |
| Admission logic `(overlay==-1 && claim!=id) OR (density<11 && tibtype!=-1)` | Verified (corrected 2026-07-20) | decompile_function + disassemble_function 0x005a28c0 (005a2ce6–005a2d31) |
| TIBTRE0%d tree on ore, not gem; variant 1..3 | Verified (variant range corrected 2026-07-20) | asm 0x005a2b75–0x005a2b88 tree gate; 0x005a2b8e–0x005a2bd1 draw; string `0x0082c090`; read_memory 0x007ed8c0/0x007e1718 |
| BFS seed limit 10, same-origin reseed, map-wide claim wipe | Verified (citation corrected 2026-07-20) | asm 0x005a29a1; 0x005a29cc–0x005a2a56 |
| 8-neighbor direction table | Verified | asm 0x005a2c7c–0x005a2c8e; `g_DirectionOffsets @ 0x0089f688` |
| Region struct offsets (+0x20, +0x2C, +0x38) | Verified (+0x38 meaning corrected 2026-07-20) | asm 0x005a242b, 0x005a247d, 0x005a24bd |
| Cell array stride 0x50 | Verified | asm 0x005a2d09–0x005a2d10 |
| Gem second-pass loop (unconditional; size=trunc(Δscore×15)+500; id=startIdx+1) | Verified (corrected 2026-07-20) | asm 0x005a263a–0x005a2846; read_memory 0x007ed7b0 |

---

*Report generated 2026-06-01; corrected 2026-07-20 after the 2026-07-19 RED audit (see top note; recheck evidence in `RMG_TIBERIUM_FIELD_COUNT_AND_GATES_RECHECK_GHIDRA_REPORT.md`). Confidence: HIGH on field-count formula, gate chain, overlay write mechanics, RNG routing, density cap, tree placement gate, gem flag, gem second pass, BFS structure — all re-read from live disassembly + memory 2026-07-20. UNCHECKED: overlay type IDs 27/102 mapped to INI OverlayTypes names; scratch `+0x45` blocked-flag producer.*
