# RMG Tiberium — Field-Count Formula & BFS Gates Recheck — Ghidra Research Report

**Address(es):** `0x005a23a0`–`0x005a289d` (driver `FUN_005a23a0`); `0x005a28c0`–`0x005a2ec8` (field BFS placer `FUN_005a28c0`, RET 0x10 at `0x005a2ec8`); `0x00594f40`–`0x005953f1` (field-slot selector `FUN_00594f40`); `0x00598960` (RMG generator, caller)
**Investigation Mode:** exhaustive-slice (full decompile + full disassembly of driver, placer, and slot selector; every FP constant read from memory)
**Claimed Scope:** Re-derivation of the four items flagged WRONG in the 2026-07-19 RED audit of `RMG_TIBERIUM_CREATION_005A23A0_GHIDRA_REPORT.md`: (a) the exact field-count formula; (b) the exact BFS admission-gate chain; (c) where `TiberiumLayout` (MapSeed+0x58 / `DAT_00ABE030`) is consumed; (d) the gem second-pass loop structure.
**Non-Scope:** water/hills/LAT stages, region partitioning internals, RNG primitive internals, runtime tiberium growth.
**Active in YR:** Conditional — the whole slice runs only when the RMG generates a map from a `.SED` seed (call chain `FUN_00598960` → `FUN_005a23a0` at `0x00598ef4`, inside the `"RMG: Creating tiberium"` heap-pool phase; verified via decompile_function 0x00598960 + get_xrefs_to 0x005a23a0 2026-07-20). Inherited RMG feature (present in RA2, reachable in YR skirmish via the random-map dialog).

---

## 1. FP constants used by this slice (all read from memory this session)

| Address | Bytes (LE) | Value | Used at | Role |
|---|---|---|---|---|
| `0x007e3808` | `7b14ae47e17a843f` | **0.01** | `005a2565`, `00594f55` | percent scaling of Tiberium option / TiberiumLayout |
| `0x007e1738` | `000000000000e03f` | **0.5** | `005a257d`, `005a2592` | floor for the start-count multiplier |
| `0x007ed898` | `000010000000f03d` | **1/(2^32−1)** ≈ 2.3283064370807974e-10 | `005a24ae`, `005a2da3` | uniform scale of a 32-bit `Random__Next` draw to [0,1] |
| `0x007e4f50` | `0000000000004940` | **50.0** | `005a25d3` | Gaussian field-size jitter scale |
| `0x007eda88` | `00000000000059c0` | **−100.0** | `005a25dd` | jitter rejection lower bound |
| `0x007e2ac0` | `0000000000005940` | **+100.0** | `005a25ee` | jitter rejection upper bound |
| `0x007ed7b0` | `0000000000002e40` | **15.0** | `005a27fb` | gem-pass size-per-distance multiplier |
| `0x007ed7c0` | `0000000000001440` | **5.0** | `005a2d98` | BFS priority jitter range |
| `0x007ed8c0` | `000018000000083e` | **≈ 3/2^32** (exactly 3·2^−32·(1+2^−24) ≈ 6.98491938e-10) | `005a2ba8` | TIBTRE variant draw scale |
| `0x007e1718` | `000000000000f03f` | **1.0** | `005a2bae` | TIBTRE variant draw offset |
| `0x007eda90` | `000018000000283e` | **≈ 12/2^32** (exactly 12·2^−32·(1+2^−24) ≈ 2.79396775e-9) | `005a2c32` | overlay density draw scale |
| `0x007ed8d8` | `0000000000002840` | **12.0** | `00594f77` | slot-count formula multiplier |
| `0x007e1708` | `0000000000000040` | **2.0** | `00594f83` | slot-count formula offset |

(each verified via read_memory 2026-07-20)

**ftol rounding:** every `CALL 0x007c5f00` (`Math__ftol`) loads FPU control word `DAT_00822d80 = 0x0E7F` before `FISTP` (verified via disassemble_function 0x007c5f00 + read_memory 0x00822d80 2026-07-20). CW `0x0E7F` has RC bits (11:10) = `11` = **round toward zero (truncate)**. All `ftol` below means truncation.

---

## 2. (a) The exact field-count formula — NO RNG involved

Verified via disassemble_function 0x005a23a0 (block `005a2548`–`005a25bb`) 2026-07-20. Executed once per non-null region, after the optional gem-anchor block (§4):

```
min  = MapSeed[+0x2BC]                      ; RMGMinimumTiberium   (005a254c)
span = MapSeed[+0x2C0] - min                ; RMGMaximumTiberium   (005a2552, 005a255b)
; FPU chain, exact op order:
;   FILD MapSeed[+0x54]  →  FMUL 0.01  →  FILD span → FMULP  →  FIADD min  →  ftol
lerp = trunc( (double)MapSeed[+0x54] * 0.01 * (double)span + (double)min )    (005a2558-005a2575)

; start-count multiplier, floored at 0.5:
;   FILD region[+0x20]; FLD 0.5; FCOMP; if (0.5 > startCount) replace with 0.5
mult = max( (double)region[+0x20], 0.5 )                                       (005a257a-005a2592)

; NOTE: lerp is ALREADY truncated to int before this multiply (FILD [ESP+0x38] reloads it):
regionTotal = trunc( (double)lerp * mult )                                     (005a2598-005a259e)

fieldCount = region_sub[+0x10]              ; region_sub = *(region+0x00)      (005a25a3-005a25a7)
if (fieldCount == 0 || regionTotal == 0)  → skip this region entirely          (005a25aa-005a25b4)

perFieldBase = regionTotal IDIV fieldCount  ; CDQ/IDIV, signed truncating      (005a25ba-005a25bb)
```

- `MapSeed[+0x54]` is the **Tiberium** dialog option, read as a signed int and scaled by 0.01 → it is a **percent (0..100)** lerp parameter between `RMGMinimumTiberium` and `RMGMaximumTiberium`.
- There is **exactly one** `CALL 0x0065c780` (`Random__Next`) in the whole driver — at `005a2499`, inside the no-starts reference-pick path of the gem-anchor block (§4). The field-count computation consumes **zero** RNG draws (verified by scanning the full disassembly for `CALL 0x0065c780` 2026-07-20).
- Clamping: the only clamp is the `max(startCount, 0.5)` floor. `regionTotal` and `perFieldBase` are not otherwise clamped; the zero-tests skip the region.

**Per-field size loop** consuming `perFieldBase` (i = 0..fieldCount−1, `005a25c9`–`005a2638`):

```
do { j = FUN_005980c0(ECX=0xabdfb8) * 50.0 }        ; Box-Muller Gaussian * 50.0
while (j < -100.0 || j > 100.0)                     ; rejection resample        (005a25c9-005a25f9)
size_i = trunc( (double)perFieldBase + j )                                      (005a25fb-005a2603)
if (size_i >= 0)                                                                (005a2608)
    FUN_005a28c0( &region_sub[+0x4][i],             ; field-slot coord (packed x/y words)
                  size_i,
                  globalStartBase + i + 1,          ; field_id                  (005a261a)
                  (i == nearestSlotIdx) )           ; is_gem                    (005a2614-005a2616)
; negative size_i → slot silently skipped (no placer call); loop index advances regardless
```

The Gaussian rejection window means every field draws at least one Gaussian pair regardless of `bVar12`; RNG stream shape per region = (optional 1+ reference-pick draws) + (≥1 Gaussian per field slot).

---

## 3. (b) The exact BFS admission-gate chain in `FUN_005a28c0`

Verified via decompile_function + disassemble_function 0x005a28c0 2026-07-20. Signature: `void __stdcall FUN_005a28c0(short* origin_coord, int target_size, int field_id, char is_gem)` — RET 0x10; incoming ECX is **never read** (first ECX use at `005a28ee` is a write), so the `MOV ECX` seen before its call sites is dead code.

Two distinct cell arrays are involved — the RED doc conflated them:

- **Scratch array** `DAT_00abed10`, stride 0x50, indexed `(y * width@0x0089c2dc + x) * 0x50`: holds `+0x38` region id, `+0x3C` field-claim id, `+0x45` blocked flag. RMG bookkeeping only; freed at generator end.
- **Real `CellClass`** obtained via `CALL 0x005657a0` (`MapClass__Get_CellClass`, ECX = MapClass `0x0087f7e8`): holds `+0x38` tile index, `+0x44` overlay dword, `+0x11E` density byte. **The overlay/density writes land here**, not in the scratch array.

**Neighbor admission chain, exact order** (loop `005a2c76`–`005a2e54`, i = 0..7):

```
1. coord_n = cur + g_DirectionOffsets[i & 7]        ; word dx @ [0x0089f688 + (i&7)*4],
                                                    ; word dy @ +2                    (005a2c7c-005a2c95)
2. MapClass__Is_Cell_In_Playfield(&coord_n, 1)      ; CALL 0x00578460, ECX=0x87f7e8   (005a2cb5)
      == 0 → REJECT
      ; 0x00578460 is the isometric playfield diamond test against MapClass
      ; +0xF4/+0xFC/+0x100/+0x104/+0x108 with a height adjustment when arg2=1
      ; (verified via decompile_function 0x00578460 2026-07-20). It is NOT IsClearTile.
3. cell = MapClass__Get_CellClass(&coord_n)         ; CALL 0x005657a0, ECX=0x87f7e8   (005a2ccc)
4. CellClass__IsClearTile(cell)                     ; CALL 0x00486380, ECX=cell       (005a2cd9)
      == 0 → REJECT
      ; 0x00486380 returns 1 iff cell[+0x38] (tile index) == 0 or == 0xFFFF
      ; (verified via decompile_function 0x00486380 2026-07-20)
5. if (cell[+0x44] == -1)                           ; no overlay yet                  (005a2ce6)
      if (scratch[coord_n][+0x3C] != field_id)  → ADMIT (fresh empty cell)            (005a2d10-005a2d14)
      else fall through to 6                        ; empty but already claimed by this run
6. if (cell[+0x11E] < 0x0B                          ; density < 11                    (005a2d1a-005a2d21)
       && CellClass__GetTiberiumType(cell) != -1)   ; CALL 0x00485010                 (005a2d29-005a2d31)
      → ADMIT (existing-tiberium revisit)
   else → REJECT
   ; an empty-but-claimed cell reaches 6 and fails GetTiberiumType()==-1 → the
   ; +0x3C claim is what dedups empty cells within one seed-generation
```

**On ADMIT** (`005a2d3b`–`005a2e41`):

```
priority = Sqrt_Approx( (anchor - coord_n)² sum )                               (005a2d42-005a2d6d)
         + Random__Next(ECX=0xabe890) * 5.0 * 1/(2^32-1)   ; uniform [0,5]      (005a2d79-005a2db6)
scratch[coord_n][+0x3C] = field_id                          ; claim at PUSH time (005a2dd1)
push (coord_n, priority) onto float min-heap; silently dropped if heap
count+1 >= capacity (target_size*10)                                            (005a2def)
```

The priority jitter is **uniform** — `FUN_005980c0` (Gaussian) is never called inside the placer (verified by scanning the full disassembly 2026-07-20). `anchor` = origin coord at each seed, replaced by the first cell actually written in that seed-generation (`005a2b50`-`005a2b57`).

**Pop-cell write path** (`005a2b0c`–`005a2c63`):

```
gate: scratch[cur][+0x45] == 0                       ; blocked-flag byte         (005a2b39)
first written cell of each seed-generation (flag [ESP+0x13]):
    anchor = cur; CLEAR the priority queue; reset record count                   (005a2b50-005a2b80)
    if (!is_gem):                                                                (005a2b75-005a2b88)
        do { v = trunc(rand * ~3/2^32 + 1.0) } while (v > 3)   ; v ∈ {1,2,3}    (005a2b8e-005a2bbc)
        sprintf("TIBTRE0%d", v % 10)   ; → TIBTRE01..TIBTRE03 (never TIBTRE00)   (005a2bbe-005a2bd1)
        TerrainTypeClass find (0x0071dd80) + TerrainClass ctor (0x0071bb90)      (005a2bed-005a2c05)
if (cell[+0x44] == -1):                                                          (005a2c12)
    do { d = trunc(rand * ~12/2^32) } while (d > 11)           ; d ∈ [0,11]     (005a2c18-005a2c40)
    cell[+0x44] = d + base          ; base = 0x66 ore / 0x1B gem                 (005a2c4a-005a2c4c)
else if (cell[+0x11E] < 0x0B): cell[+0x11E] += 1                                 (005a2c51-005a2c5d)
else: skip (no placed++)
placed++                                                                         (005a2c63)
```

**Restart structure:** outer loop runs while `placed < target_size` (`005a2e93`-`005a2e99`) and breaks when the seed counter `[ESP+0x2C]` reaches 10 (`CMP [ESP+0x2C],0xA; JGE` at `005a29a1`) — max **10 seeds including the first**. Each seed (queue empty → `local_74 == 0`): clears `scratch[+0x3C]` for **every** map cell via the MapClass cell iterator (`0x00578350` init / `0x00578290` next, ECX=0x87f7e8, `005a29cc`-`005a2a1a`), then reseeds the queue from **the same `origin_coord` param** (`005a2a2d`: `MOV EAX,[EDX]; MOV [EDI],EAX` with EDX = param_1) — not from a new origin.

All three RNG sites in the placer use `ECX = 0xabe890` (`005a2b8e`, `005a2c18`, `005a2d79`) — consistent with the audit-confirmed MapGen stream.

---

## 4. Gem-anchor selection in pass 1 (`bVar12` block, `005a2417`–`005a2548`)

When `bVar12` (the §3 gem flag of the parent doc) is true, exactly **one** pass-1 field per region is flagged gem: `nearestSlotIdx` (`local_6c`, init −1):

- If `region[+0x20] >= 1`: reference point = **component-wise average of the region's start waypoint coords** (loop `005a243b`-`005a2467` sums x into BX / y into BP via `FUN_0068bcc0(ECX=[0x00a8b230], globalStartBase+i)`; `IDIV` by count per axis at `005a2469`-`005a2479`). Not "closest start to region center" as the RED doc claimed.
- Else: reference point = random entry `region[+0x2C][k]`, `k = trunc(rand * (double)region[+0x38] * 1/(2^32-1))` rejection-resampled while `k > region[+0x38]-1` (`005a247d`-`005a24c7`). So `region[+0x38]` is the **length of the coord array at `region[+0x2C]`**, not a "start index offset".
- `nearestSlotIdx` = argmin over the region's field slots of `Sqrt_Approx(dx²+dy²)` to the reference point; running min is `ftol`'d on update, init 500000 (`005a24cf`-`005a2546`).

When `bVar12` is false the block is skipped entirely (`005a2423: JZ 005a254c`) and pass 1 places only ore.

---

## 5. (d) The gem second pass — unconditional, per-start compensation

Verified via disassemble_function 0x005a23a0 (block `005a263a`–`005a2877`) 2026-07-20. **Runs for every region that entered placement** — there is no `bVar12` gate on it (control falls straight from the pass-1 loop into `005a263a`); the RED doc's "when bVar12=true" was wrong.

```
scores = dynamic double array (vtable 0x007eda6c, grow step 10)                  (005a263a-005a265b)
for s in 0..region[+0x20]:                                                       (005a2662-005a2779)
    start_s  = FUN_0068bcc0(ECX=[0x00a8b230], globalStartBase + s)   ; waypoint coord
    score_s  = ( Σ over all fieldCount slots of Sqrt_Approx(dist²(start_s, slot)) )
               / fieldCount                       ; sum first, ONE divide (FDIVR, 005a2715)
    append score_s
minScore = min(scores)      ; init 9999999.0 (0x416312CFE0000000)                (005a277f-005a27bc)
gem2 = (MapSeed[+0x40] == 3) && (MapSeed[+0x3C] ∈ {1,3,4})                       (005a27be-005a27e1)
for s in 0..region[+0x20]:                                                       (005a27e6-005a2837)
    size    = trunc( (score_s - minScore) * 15.0 ) + 500     ; ADD EAX,0x1F4 at 005a280a
    origin  = FUN_0068bcc0(globalStartBase + s)              ; the start's own waypoint cell
    FUN_005a28c0(origin, size, globalStartBase + s + 1, gem2)
globalStartBase += region[+0x20]                                                 (005a2839-005a2846)
```

Load-bearing corrections vs the RED doc:

- **Every** start gets a pass-2 field, not just "starts that qualify"; the container-capacity check in the scoring loop is dynamic-array growth plumbing, not a filter.
- The `+0x1F4` (500) is added to the **size**, not the field id. The start closest (on average) to the region's field slots gets exactly 500; farther starts get `+15 per average-distance-unit` of excess — a fairness compensation.
- Pass-2 fields are **gem** only when `Resources == 3 && map type ∈ {1,3,4}` — a configuration in which `bVar12` is false (types 1/3/4 make `bVar12 = Resources != 3`). Net: water-type maps with Resources==3 get all-ore pass 1 + gem pass 2; every other gem-capable config gets one gem anchor in pass 1 + ore pass 2; land type 0 with Resources!=3 gets no gems at all.
- Pass-1 ids (`base+i+1`, i over field slots) and pass-2 ids (`base+s+1`, s over starts) **overlap numerically** within a region. Harmless: each placer call wipes all scratch `+0x3C` claims at its first seed, and claims are only compared within a single placer run. Ids are ≥ 1, and the wipe writes 0, so no false claim match.

---

## 6. (c) `TiberiumLayout` / `DAT_00ABE030` resolution

- Arithmetic: MapSeed base `0x00ABDFD8` + `0x58` = **`0x00ABE030`** — so `DAT_00ABE030` is the global MapSeedClass instance's TiberiumLayout field.
- `get_xrefs_to 0x00abe030` (2026-07-20) → **exactly one reference**: READ at `0x00594f49` in `FUN_00594f40`. Neither `FUN_005a23a0` nor `FUN_005a28c0` touches it (confirms the audit).
- `FUN_00594f40` head formula (verified via disassemble_function 0x00594f40, `00594f49`–`00594f8d`, + read_memory of all constants 2026-07-20):

```
slotTarget = trunc( ( (double)TiberiumLayout            ; FILD [0x00abe030]
                      * 0.01                            ; FMUL [0x007e3808]
                      * 12.0                            ; FMUL [0x007ed8d8]
                      / (double)NumPlayers@0x00abe028   ; FIDIV [0x00abe028] = MapSeed+0x50
                      + 2.0 )                           ; FADD [0x007e1708]
                    * regionStartQuota )                ; FIMUL this[+0x20]; ftol
; then: if (this[+0x20]==0 && candCount>0) slotTarget = candCount;
;       if (slotTarget > candCount || slotTarget == 0) { slotTarget = candCount;
;           if (this[+0x20]==0) return NULL; }                                   (00594f92-00594fc6)
```

  `0x00ABE028` = `0xABDFD8+0x50` = MapSeed+0x50 = the NumPlayers/start-quota option (also read by `FUN_00594b50` at `0x00594df0`; INI-parsed write at `0x00596018`, default 4 at `0x007a2494` — all via get_xrefs_to 0x00abe028 + get_assembly_context 2026-07-20; identity as "NumPlayers" corroborated by the 2026-07-19 GREEN audit of `RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md`).
- The rest of `FUN_00594f40` farthest-point-samples `slotTarget` coords from the candidate list (`param_2+0x4` array / `param_2+0x10` count): first pick = a member of the max-pairwise-distance pair; each subsequent pick maximizes the min distance to the already-picked set; every distance gets **+20.0** added when the two cells' scratch region ids (`DAT_00abed10[..]+0x38`) differ (`AND ESI,0x14; FIADD` pattern at `005950e3`/`0059530b`). It returns a dynamic array with data at `+0x4` and count at `+0x10` — **exactly the `region_sub` shape (`*region+0x00` → `+0x4` slot array, `+0x10` count) that `FUN_005a23a0` consumes as its field slots and divisor**.
- Call chain: `FUN_00594b50` (start generation, "Creating starting points" phase) → `FUN_00594870` → `FUN_00594f40` (verified via get_function_callers on both 2026-07-20).

**Conclusion:** TiberiumLayout does not appear in the tiberium driver because it acts **one stage earlier** — it scales the *number of field-slot positions* selected per region (≈ `(TiberiumLayout% × 12 / NumPlayers + 2)` slots per start), which then becomes the driver's `fieldCount` divisor. This resolves the start-scoring doc's MEDIUM-unidentified `DAT_00ABE030` and the 2026-07-19 audit's UNVERIFIABLE identity item.

---

## 7. Coverage ledger

| Area | Status | Evidence (all 2026-07-20) |
|---|---|---|
| Field-count formula, exact FPU op order + constants | Verified | disassemble_function 0x005a23a0 (005a2548-005a25bb); read_memory 0x007e3808/0x007e1738 |
| ftol = truncate toward zero | Verified | disassemble_function 0x007c5f00; read_memory 0x00822d80 (CW 0x0E7F, RC=11) |
| No RNG in field-count computation | Verified | full-disassembly scan: only `CALL 0x0065c780` in driver is 005a2499 |
| Per-field Gaussian jitter ±100 clamp, skip-if-negative | Verified | disassemble_function 0x005a23a0 (005a25c9-005a2638); read_memory 0x007e4f50/0x007eda88/0x007e2ac0 |
| BFS gate chain order + operators | Verified | decompile+disassemble_function 0x005a28c0 (005a2c76-005a2d3b) |
| 0x00578460 = playfield diamond test; 0x00486380 = IsClearTile | Verified | decompile_function 0x00578460 / 0x00486380 |
| Overlay/density writes on real CellClass (via 0x005657a0), claims on scratch DAT_00abed10 | Verified | disassemble_function 0x005a28c0 (005a2b0c-005a2c5d vs 005a2dd1) |
| Priority jitter uniform [0,5], not Gaussian | Verified | disassemble_function 0x005a28c0 (005a2d79-005a2db6); read_memory 0x007ed7c0/0x007ed898 |
| TIBTRE variant ∈ {1,2,3} | Verified | disassemble_function 0x005a28c0 (005a2b8e-005a2bd1); read_memory 0x007ed8c0/0x007e1718 |
| Reseed from same origin; map-wide +0x3C wipe per seed; 10-seed cap | Verified | disassemble_function 0x005a28c0 (005a29a1, 005a29cc-005a2a56) |
| Placer ignores ECX (stdcall, RET 0x10) | Verified | disassemble_function 0x005a28c0 (no ECX read before first write; 005a2ec8) |
| Gem second pass unconditional; size=trunc(Δscore×15)+500; id=startIdx+1; gem2 flag | Verified | disassemble_function 0x005a23a0 (005a263a-005a2846); read_memory 0x007ed7b0 |
| DAT_00ABE030 = MapSeed+0x58; sole reader FUN_00594f40; slot-count formula | Verified | get_xrefs_to 0x00abe030; disassemble_function 0x00594f40; read_memory 0x007ed8d8/0x007e1708 |
| DAT_00ABE028 = MapSeed+0x50 (NumPlayers), default 4 | Verified | get_xrefs_to 0x00abe028; get_assembly_context 00596018/007a2494 |
| Driver called with ECX = generator's MapSeedClass this | Verified | get_assembly_context 00598ef4 (`MOV ECX,ESI; CALL 0x005a23a0`); decompile_function 0x00598960 |

## 8. Open questions — final state

- **RESOLVED:** field-count formula (was RED §4/§5) — §2 above. Gate identities/order (was RED §6) — §3. TiberiumLayout consumer — §6. Gem-pass structure — §5. `region_sub[+0x10]` provenance (RED §10 open item) — populated by `FUN_00594f40` slot selection.
- **OPEN (out of scope):** what populates scratch `+0x45` (blocked flag) — set upstream of the tiberium stage, not traced here. Exact INI names for overlay indices 27/102 (unchanged from parent doc). Semantics of `FUN_0068bcc0` waypoint lookup internals (owned by the start-generation doc). `region[+0x2C]` array contents provenance (region partition slot).

## 9. Implementation handoff

| Verified behavior | Rust delta | Acceptance check | Risk |
|---|---|---|---|
| `regionTotal = trunc(trunc(tib%·0.01·(max−min)+min) · max(starts,0.5))`; `perFieldBase = regionTotal / fieldCount` (truncating); skip region on either zero | RMG tiberium field sizing must use this two-stage truncation (inner lerp truncated **before** the multiply) with fixed-point-safe math | Exhaustive vector test over tib∈[0,100], min/max from SED, starts∈[0,8] comparing against this formula | Inner-truncation order changes results for fractional lerps — do not fold into one expression |
| Per-field size = `trunc(base + N(0,1)·50)` rejection-clamped to ±100, skip if < 0 | Draw Gaussian per slot from MapGen stream even for skipped slots' predecessors (stream shape) | Seeded replay: identical slot-size sequence | Rejection resampling consumes variable draws — must match |
| Gate chain §3 order: playfield-diamond → IsClearTile(tile∈{0,0xFFFF}) → (overlay==−1 && claim≠id) OR (density<11 && tibtype≠−1) | Implement in this exact short-circuit order; claims live in a scratch layer, overlay/density on real cells | Unit test with a crafted 3×3 neighborhood exercising all four reject paths + both admit paths | Swapping gate 2/4 identities (the RED doc's bug) admits water/cliff cells |
| Priority = dist-to-anchor + uniform[0,5]; anchor rebinds to first written cell per seed; 10 seeds max, same origin, map-wide claim wipe per seed | Min-heap on f32 priority; capacity target×10 with silent drop | Seeded replay: identical blob shape | Anchor-rebind subtly changes blob eccentricity |
| TIBTRE variant `trunc(rand·3/2^32+1)` reject >3 → TIBTRE01..03, ore-anchor only, once per seed-generation | Never spawn TIBTRE00 | Assert spawned names ∈ {TIBTRE01..03} | Old doc said 0..3 — TIBTRE00 does not exist in rulesmd TerrainTypes |
| Gem pass: every start, size `trunc(Δscore·15)+500`, gem iff Resources==3 ∧ type∈{1,3,4} | Unconditional second pass per region | Scenario test on water-type + Resources 3: gem overlay 27..38 at every start | Gating it on bVar12 (old doc) drops all pass-2 fields on gem maps |
| Slot count per region ≈ `trunc((TibLayout%·12/NumPlayers+2)·starts)` (FUN_00594f40, farthest-point sampling, +20 cross-region penalty) | Belongs to the start-generation implementation slice, feeds this one | Cross-check fieldCount fed into driver | Lives in a different function/stage — do not duplicate |

## 10. Sources

- decompile_function / disassemble_function: 0x005a23a0, 0x005a28c0, 0x00594f40, 0x00598960, 0x00578460, 0x00486380, 0x007c5f00 (2026-07-20)
- read_memory: 0x007e3808, 0x007e1738, 0x007ed898, 0x007ed7b0, 0x007e4f50, 0x007eda88, 0x007e2ac0, 0x007ed8c0, 0x007e1718, 0x007eda90, 0x007ed8d8, 0x007e1708, 0x00822d80 (2026-07-20)
- get_xrefs_to: 0x00abe030, 0x00abe028, 0x005a23a0, 0x00abdfd8; get_function_callers: 0x00594f40, 0x00594870; get_assembly_context: 0x00598ef4, 0x00596018, 0x007a2494, 0x00594df0 (2026-07-20)
- 2026-07-19 audits: `AUDIT_LOG.md` entries for `RMG_TIBERIUM_CREATION_005A23A0_GHIDRA_REPORT.md` (RED) and `RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md` (YELLOW, NumPlayers identity)

*Report generated 2026-07-20. Confidence: HIGH (content) on all formulas/gates — every claim read from live disassembly + memory this session; HIGH (identity/binding) for 0x00578460/0x00486380/0x007c5f00 via body decompile; MapSeed-instance binding (0xABDFD8) rests on field-offset arithmetic + the start-generation audit's NumPlayers verification, not on a writer-scan of the instance pointer itself.*
