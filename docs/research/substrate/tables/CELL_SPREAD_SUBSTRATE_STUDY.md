# Cell-Spread Tables — Substrate Study (gamemd parity + Rust-native service design)

**Date:** 2026-06-04
**Family:** Cell-spread tables (`DAT_007ED3D0` count table + `DAT_00ABD490`/`DAT_00ABD492` offset table)
**Scope:** Filled-disk cell enumeration around a center cell for all area-of-effect operations
(splash/area damage, ore destruction, wall/bridge destruction, shroud reveal).
**Doc kind:** STUDY (research + design). NO Rust code written or modified.
**Authority order:** binary → Ghidra → docs. Every binary fact below cites the exact Ghidra MCP
call used to verify it **this session** (2026-06-04). Rust file:line cited for every comparison claim.
**Burden of proof:** DRIFT by default. Equivalence claimed only with algebraic proof or
boundary-inclusive bit-identity. "Unproven equivalence" is recorded as DRIFT/UNCHECKED, never downgraded.

**Per-claim confidence summary**
- Count table values, three rounding constants, both ftol index computations, air-distance halve,
  cell-loop stride/dx-dy extraction: **PROOFED** (read live in `disassemble_function 0x00489280`
  and `read_memory` this session).
- Full 369-entry offset-table contents and the R6/R11 data defects: **PROOFED** (upgraded from HIGH on
  2026-06-04 adversarial re-check: the initializer `0x00561910` was re-decompiled live this session via
  `decompile_function 0x00561910`; idx0=(0,0), the R1 sweep, idx96=(-4,-4), and the R11 duplicate were
  read directly from the initializer body. NOTE: the static offset table `read_memory 0x00ABD490` is
  **all zeros** — it lives in BSS and is populated only at runtime by this initializer, so the table
  contents are knowable ONLY through the initializer, not a static dump. Offset-table *reader binding*
  re-verified live this session at `0x004895C7/0x004895CF/0x004899C7`.)
- Shroud-reveal cross-consumer (`RevealAroundCell 0x005678E0`): **VERIFIED** (upgraded from HIGH doc-sourced
  on 2026-06-04: re-decompiled live this session via `decompile_function 0x005678E0` — shared offset table
  `DAT_00abd490` + count table `DAT_007ed3d0`, clamp-to-10 confirmed. **CORRECTION:** reveal is NOT
  threshold-free — it applies a per-cell **cell-space** Euclidean gate `ftol(sqrt(dx²+dy²)) <= sight`
  (`Sqrt_Approx` + `Math__ftol` per iteration), distinct from the splash lepton×256 threshold. See §5d.)

---

## 1. Active-YR responsibilities

The cell-spread family is **two cooperating static tables** that drive filled-disk cell enumeration
around a center cell for every AoE operation in a stock YR skirmish.

### 1a. Count table `DAT_007ED3D0`
12 cumulative filled-disk cell counts, one per integer radius band 0..11. Tells a consumer *how many*
entries of the offset table to walk. Verified bytes this session (`read_memory 0x007ED3D0`, 48 bytes):
`[1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309, 369]`
(hex `01,09,15,25,3d,59,79,a1,cd,fd,3501,7101`).

### 1b. Offset table `DAT_00ABD490` (= `DAT_00ABD492` aliased at +2)
A single flat array of 369 packed `int32` entries (idx 0..368). Each entry encodes a signed cell
offset `(dx, dy)` from the center cell: low 16 bits = `dx` (`int16`), high 16 bits = `dy` (`int16`).
Idx 0 = `(0,0)` (the impact cell, always scanned first). Verified reader binding this session
(`disassemble_function 0x00489280`):
```
004895c3  MOV EAX,[ESP+0x10]                 ; EAX = loop index local_d8
004895c7  MOV DX, word ptr [EAX*0x4 + 0xABD490]   ; dx = low int16
004895cf  MOV AX, word ptr [EAX*0x4 + 0xABD492]   ; dy = high int16 (= base + 2)
004895d7  ADD DX, word ptr [ESP+0x18]        ; centerCellX + dx
004895dc  ADD AX, word ptr [ESP+0x1a]        ; centerCellY + dy
```
Stride `EAX*0x4` = 4-byte entries, both reads from the same base array. Loop terminator reads the
count table at `004899c7 MOV ECX,[EDX*0x4 + 0x7ed3d0]`.

### 1c. Primary consumer — `Apply_area_damage @ 0x00489280`
Walks the disk to collect every object in range and apply `ReceiveDamage`. Player-visible outputs:
which units/buildings in an explosion take damage, the **order** targets are appended to the damage
vector (= ReceiveDamage call order = RNG-consumption / delay-kill / chain timing), ore/tiberium
reduction in range, wall/overlay/fence destruction, bridge-deck damage rolls, building capture
detection, and the airborne-target scan. Reachability (`get_function_callers 0x00489280`,
prior-session verified, 19 sites): `WarheadTypeClass::Detonate (0x004690b0)`,
`BombClass::Detonate (0x00438720)`, `LightningStorm GroundStrike (0x0053a300)`,
`PsychicDominator MindControlArea (0x0053b080)`, `NukeGroundZero ApplyDamage (0x004251f0)`,
`SuperClass::Launch (0x006cc390)`, `Wave_splash_forces (0x0053cbe0)`, `DiskLaserClass::AI (0x004a7340)`,
`InfantryClass::PerCellProcess (0x00519630)`, `TerrainClass::Take_Damage (0x0071b920)`,
`AnimClass::AI`/`Middle`, `VoxelAnimClass::AI`, `FlyLocomotionClass::Process`, plus the recursive
barrel-chain self-call at `0x0048a371`. **Every splash detonation in normal play routes here — the
table is hot, every-match active.**

### 1d. Secondary consumer — shroud reveal (`RevealShroud 0x005673A0` → `RevealAroundCell 0x005678E0`)
Iterates the SAME `DAT_007ED3D0` + `DAT_00ABD490` tables to clear shroud around a cell (unit vision,
spy-sat, Psychic Reveal SW). Index rule differs (see §5d): clamps radius to `MAX_SIGHT = 10` and uses
the **raw integer sight/radius** as the count-table index — no `+0.99`, no `×256` lepton threshold
(re-verified live this session: `decompile_function 0x005678E0`; `iStack_30 = (&DAT_007ed3d0)[param_3]`
after `if (10 < param_3) param_3 = 10`; `psStack_38 = &DAT_00abd490`). **CORRECTION (2026-06-04):** reveal
is NOT filter-free — it applies a per-cell **cell-space** Euclidean distance gate inside the loop
(`Sqrt_Approx(dx²+dy²)` → `Math__ftol` → `if (sVar4 <= param_3)`), so a corner cell of the count-band
disk is skipped if `round(sqrt(dx²+dy²)) > sight`. The fine filter exists in BOTH consumers; it is just
cell-space (`ftol(sqrt)`) for reveal vs lepton-space (`ftol(CS×256)`) for splash. This is the
**cross-family hook** the synthesis stage must reconcile: one shared offset table + count table, two
consumers with different clamp + fine-filter rules.

---

## 2. Full inventory (each item with its verification)

| Item | Address | Type / size | Value(s) | Verification (this session unless noted) |
|---|---|---|---|---|
| Count table | `0x007ED3D0` | 12 × `int32` LE, 48 bytes | `[1,9,21,37,61,89,121,161,205,253,309,369]` | `read_memory 0x007ED3D0` len 48 |
| Offset table | `0x00ABD490`..`0x00ABDA53` | 369 × packed `int32` (dx low16, dy high16), 1476 bytes (0x5C4) | idx0=(0,0); R-band layout per §4 | reader binding `disassemble_function 0x00489280` @ `004895C7/CF`; contents decoded from initializer (prior session, HIGH) |
| Offset alias | `0x00ABD492` | same array +2 | high int16 = dy | `disassemble_function 0x00489280` @ `004895CF` |
| Count-index addend | `0x007E5160` | `double` | **0.99** | `read_memory 0x007E5160` = `ae47e17a14aeef3f` |
| Distance scale | `0x007E2224` | `float` | **256.0** | `read_memory 0x007E2224` = `00008043` |
| Air-flag threshold | `0x007E5168` | `float` | **0.5** | `read_memory 0x007E5168` = `0000003f` |
| Initializer | `0x00561910` | fn (`MapClass__InitRevealSpiralTable`) | populates offset table at startup (BSS→data) | name is a navigation hint; binding confirmed via WRITE xref prior session |
| Primary reader | `0x00489280` | fn (`Apply_area_damage`) | walks disk, two index computations (§5) | `disassemble_function 0x00489280` full body |
| Shroud reader | `0x005678E0` | fn (`RevealAroundCell`) | separate index/clamp (§5d) | doc-sourced (HIGH), not re-decompiled this session |

**Rules offsets used alongside the table** (read in `Apply_area_damage` decomp this session):
- `Rules+0xfac` = C4Warhead ptr; `CMP ESI,[ECX+0xfac]` @ `004892fc` sets `bVar`("absolute damage"/self-hit) flag.
- `Rules+0xff0` = warhead ptr that **bypasses** the bridge-strength RNG roll; compared @ `00489e92`/`00489fd8`/`0048a15d`/`0048a229`/`0048a283`.
- `Rules+0x1740` = BridgeStrength, the RNG ceiling for bridge-deck destruction (`PUSH [EAX+0x1740]` @ `00489fe0`).
- `Rules+0xfa8` = death-splash warhead for the recursive self-call (`MOV ECX,[EAX+0xfa8]` @ `0048a363`).
- `Rules+0xb40`/`+0xb4c` = building immune-list ptr/count (`[EBX+0xb40]`/`[EBX+0xb4c]` @ `00489745`/`0048973b`).

---

## 3. Active vs legacy/dormant (TS-ghost screen)

| Path | Verdict | TS-ghost test (gate / reachability / visible effect) |
|---|---|---|
| Count table `0x007ED3D0` + offset table `0x00ABD490` | **ACTIVE** | Not flag-gated; 19 callers incl. stock SWs + every warhead detonation; visible (who takes splash, what is destroyed) every match. |
| Count index `ftol(CS+0.99)` and threshold `ftol(CS×256)` | **ACTIVE** | Both run unconditionally on the normal warhead path (`00489592`/`004892dd`). |
| Bridge-deck destruction (BridgeStrength roll) | **ACTIVE** for Conventional warheads on bridge tiles | Gated by `*ScenarioClass & 0x8000` AND `warhead+0x144` (`TEST CH,0x80; JZ` @ `00489eb2`; `MOV AL,[EBX+0x144]` @ `00489ebb`). `0x8000` is set in stock YR (bridges destructible). |
| Airborne-target scan (`WhatAmI==2 && IsInAir → dist/=2`) | **ACTIVE** | AA splash vs aircraft is normal YR (`CMP EAX,0x2` @ `00489a60`; `SAR EAX,0x1` @ `00489a75`). |
| Building-capture detection in impact cell (idx 0 only, `dist < 0x55`) | **ACTIVE** | Gated by air-flag (CS>0.5) (`CMP EAX,0x55` @ `00489487`/`00489962`). |
| Shroud-reveal fog-edge branch (`param_5`/`param_7`) | shroud-only; **fog-edge dormant in stock YR** | `FogOfWar` defaults off in YR; only the plain-reveal path is active. Not used by the splash path at all. |
| **R=11 duplicate entry** (idx 322 = idx 319 = `(-3,11)`; mirror `(3,-11)` never written) | **present-but-dormant in stock YR** | It is in the shipped initializer (real data defect, live), but the count index only reaches 11 for `CS ≥ 10.01`; max stock CellSpread = 10 → `ftol(10.99)=10` → 309 cells, R11 band never entered. Reachable only with modded `CS > 10`. Surface it; does not affect stock output. |
| Tunnel/subterranean, FogOfWar darkening | **N/A** | No dependence in this family. |

---

## 4. Compare vs current Rust — table-by-table, helper-by-helper

Rust anchor: `src/sim/combat/cell_spread.rs` (the whole file is the substrate candidate).
Consumers: `src/sim/combat/combat_aoe.rs` (splash damage) and `src/sim/combat/mod.rs::destroy_ore_at_impact`.

### 4a. Count table — MATCH (proven across full domain)
`cell_spread.rs:14` `const CELL_SPREAD_COUNTS: [usize; 12] = [1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309, 369];`
Element-by-element identical to the verified gamemd bytes (`read_memory 0x007ED3D0`). All 12 entries.
**Verdict: MATCH.** (12 values, exhaustively equal — this is the only proven-identical artifact in the family.)

### 4b. Offset table contents & ordering — DRIFT (363/369 positions differ; element-set differs at R6,8,9,10,11)
Rust does **not** embed the gamemd offset table. `cell_spread.rs:30-64 compute_spread_offsets()`
*generates* offsets by sorting candidate cells with key `(d², |dx|, |dy|, dy, dx)` and truncating to
`CELL_SPREAD_COUNTS[R]`. gamemd uses a fixed, hand-authored sweep order embedded in the initializer.

- **Order:** gamemd interleaves d²-groups within each radius band; Rust groups all same-d² cells
  together. R1 gamemd order is `(1,-1),(0,-1),(-1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)` (NE,N,NW,W,E,SW,S,SE);
  Rust's sort produces a different sequence. **363 of 369 positions differ** (per dump-doc §7, consistent
  with the verified R1 sweep). Order is player-visible: target append order → `ReceiveDamage` call order →
  RNG consumption order for delay-kill/anim; and per-cell chain side-effects (ore/wall/barrel destruction
  before a later cell is scanned).
  *Severity:* HIGH for any warhead with chained side-effects or RNG-sensitive splash; fires every match
  any multi-cell warhead (CellSpread≥1) detonates near ≥2 targets/ore/walls — extremely frequent.
- **Element set:** the Rust truncation tie-breaker selects a **different subset** than gamemd at radii
  where candidate cells at the boundary d² exceed the count: **R6, R8, R9, R10, R11** (dump-doc §7 set_match=NO).
  A unit on a cell gamemd includes but Rust excludes (or vice versa) takes damage in one engine, not the other.
  *Severity:* HIGH for CellSpread ≥ 6 warheads (nuke, weather, Iron Curtain damage, Psychic Dominator);
  fires whenever such a superweapon detonates near a unit on a boundary cell — every match those SWs are used.
- **R6 "idx 96" note:** the dump doc's §5 "ANOMALY at idx 96 = (-5,-4)" is a **FALSE anomaly** — the
  correct value is `(-4,-4)` d²=32 and R6 is internally symmetric. The dump doc is stale on this point;
  the real first element-set divergence at R6 is the tie-break subset, not a missing-mirror at idx 96.
  (Stage-1 correction; the dump doc §5/§7 should be patched.)
- **R11 duplicate:** idx 319 and idx 322 both = `(-3,11)`; mirror `(3,-11)` absent. Rust scans each once
  (symmetric). For modded CS≥11 this is a one-cell DRIFT (gamemd scans `(-3,11)` twice, `(3,-11)` zero).
  *Severity:* present-but-dormant in stock YR (no stock CS reaches index 11). Surface; do not silently "fix."

**Verdict: DRIFT** — the Rust table is not gamemd-faithful in order (all bands ≥R1) or element set (R6,8–11).
There is no proof of equivalence; the dump doc itself concedes the divergence.

### 4c. Count-index rounding — DRIFT (wrong rule; diverges for every fractional CellSpread)
gamemd count index = `ftol(CellSpread + 0.99)` (verified this session: `00489592 FLD [ESI+0x124]`;
`00489598 FADD double [0x007e5160]`(=0.99); `0048959e CALL ftol`; `004895a3 MOV ECX,[EAX*4+0x7ed3d0]`).
For non-integer CS this is effectively `ceil`; for exact integer N it yields N.

Rust uses **floor** (truncate-toward-zero): both consumers compute the radius as
`cell_spread.to_num::<u32>()` (`combat_aoe.rs:104` and `mod.rs:1162`). `SimFixed = I16F16`
(`util/fixed_math.rs:23`); `to_num::<u32>()` truncates the fractional bits → `floor(CS)` for CS≥0.

| CellSpread | gamemd index `ftol(CS+0.99)` → count | Rust index `floor(CS)` → count | Match? |
|---|---|---|---|
| 0.0 | 0 → 1 | 0 → 1 | YES |
| 0.5 (e.g. GUARDWH, AP) | `ftol(1.49)=1` → **9** | 0 → **1** | **NO** |
| 1.0 | `ftol(1.99)=1` → 9 | 1 → 9 | YES |
| 1.5 | `ftol(2.49)=2` → **21** | 1 → **9** | **NO** |
| 2.0 | 2 → 21 | 2 → 21 | YES |
| 2.001 | `ftol(2.991)=2` → 21 | 2 → 21 | YES |
| 10.0 | `ftol(10.99)=10` → 309 | 10 → 309 | YES |
| 10.01 (modded) | `ftol(11.0)=11` → 369 (enters R11 defect band) | 10 → 309 | **NO** |

**Verdict: DRIFT.** For any warhead with a fractional CellSpread, Rust scans a smaller disk than gamemd.
CS=0.5 is the worst case: gamemd scans 9 cells (a 3×3 block); Rust's count helper returns 1 cell.
*Severity:* HIGH — stock YR has multiple fractional-CellSpread warheads (`.5` family); fires every time
one detonates. (Note: in `combat_aoe.rs` the lepton-threshold pre-filter would still reject the 8 ring cells
at CS=0.5 because their centers are >128 leptons away — see §4d — so the *damage* output may coincide for
CS=0.5 specifically; but the **ore-destruction path** in `mod.rs` has no lepton filter and WILL under-scan,
and CS in (0.5, 1.0) or (1.0, 2.0) etc. diverges in both count and damage. The rule is wrong regardless.)

### 4d. Two-quantity split (count vs lepton threshold) — PARTIAL / DRIFT
gamemd derives **two** distinct quantities from CellSpread and applies **both**:
1. Count-table index `ftol(CS+0.99)` → coarse cell pre-filter (how many cells to walk).
2. Lepton distance threshold `local_80 = ftol(CS×256)` → fine filter; an object is damaged only if its
   3D lepton distance `≤ local_80` (verified: `004892dd FLD [ESI+0x124]`; `004892e3 FMUL [0x007e2224]`(=256);
   `004892e9 CALL ftol`; stored `[ESP+0x68]`; compared `00489475 CMP EAX,[ESP+0x68]` and `00489a91`).

Rust `combat_aoe.rs`:
- Cell pre-filter uses `floor(CS)` (DRIFT §4c).
- Lepton threshold: `combat_aoe.rs:96` `spread_leptons = cell_spread.to_num::<i64>() * 256` then
  `spread_sq = spread_leptons²` (`:97`). **`to_num::<i64>()` truncates again** → uses `floor(CS)×256`,
  not `ftol(CS×256)`. For CS=0.5: gamemd threshold = `ftol(128.0)=128` leptons; Rust = `0×256=0` leptons →
  Rust rejects **everything**, including the impact-cell occupant. **DRIFT** for every fractional CS:
  the fine filter radius collapses to 0 below CS=1.0 and is rounded down for any fractional CS.
- Falloff `distance` then uses `SimFixed::from_num(dist_leptons/256)` (`:185`,`:293`) — integer-cell
  distance, losing sub-cell precision vs gamemd's lepton-space compare; separate rounding DRIFT in the
  falloff input (out of strict table scope, flagged for the combat-falloff family).

**Verdict: DRIFT** — Rust conflates the two quantities into one `floor(CS)`-derived value and truncates the
threshold; gamemd keeps `ftol(CS+0.99)` (count) and `ftol(CS×256)` (leptons) independent.

### 4e. CellSpread=0 / empty-disk handling — DRIFT
gamemd **always** walks `count[index]` cells starting at idx 0 (the impact cell); for CS=0 that is exactly
1 cell (the impact cell), and the impact-cell occupant is collected with dist=0 and damaged (the loop runs
`local_d8 = 0..count-1`, count[0]=1). Rust `combat_aoe.rs:91` `if cell_spread <= SIM_ZERO { return Vec::new(); }`
— **early-returns with zero AoE for CS=0**, delegating CS=0 damage to a separate direct-hit path
(`mod.rs:2207` comment "CellSpread handles AoE at the impact cell — there's no..."). Whether the net
observable output matches depends on that separate path; as a **table-service contract** the Rust AoE helper
does not reproduce gamemd's "CS=0 still scans the impact cell via the table" behavior. **DRIFT/UNCHECKED**
(not proven equivalent end-to-end; the two-code-path split is a divergence from the single-loop gamemd model).

### 4f. Air-flag threshold (CS > 0.5) — NOT MODELED in the table consumer
gamemd sets a capture/air-scan flag when `CellSpread > 0.5` (`00489347 FLD [ESI+0x124]`;
`0048934d FCOMP [0x007e5168]`(=0.5)). The Rust AoE path has no equivalent CS>0.5 gate for the
airborne scan / capture detection. **DRIFT/UNCHECKED** for the air-target and capture sub-behaviors
(the Rust airborne path at `combat_aoe.rs:138-154` iterates entities unconditionally rather than gating on CS>0.5).

### 4g. Duplication of the table across consumers — STRUCTURAL
The single Rust table (`SPREAD_OFFSETS` / `CELL_SPREAD_COUNTS`) is consumed via `cells_in_spread()` in two
places (`combat_aoe.rs:95`, `mod.rs:1163`), each independently recomputing the radius as `to_num::<u32>()`.
The radius-derivation rule is **duplicated** (and both copies are wrong, §4c). The shroud-reveal consumer
(`RevealAroundCell`) is **not yet ported in Rust** to this table (vision uses its own path) — when it is,
it must reuse the SAME offset table with a DIFFERENT index rule. No single owner today.

---

## 5. gamemd-native behavior contract (input → output a Rust replacement must reproduce)

Source field: `CellSpread` = `float` at `warhead+0x124`. **Two derived quantities — do not conflate.**

### 5a. Count-table index
`idx = ftol(CellSpread + 0.99)`; `count = DAT_007ED3D0[idx]`. `ftol` = truncate toward zero, so
`+0.99` makes this `ceil` **only when the fractional part is ≥ 0.01**, and identity for exact integers.
Precisely `idx = floor(CS + 0.99)` for CS≥0. **No clamp in this reader** (only the shroud reader clamps to
10). Boundary cases (CORRECTED 2026-06-04 — earlier draft wrongly said `2.001→idx3→37`): CS=2.0 → idx 2 →
21; **CS=2.001 → `ftol(2.991)` = idx 2 → 21** (a sub-0.01 fraction does NOT cross to the next band);
CS=2.01 → `ftol(3.00)` = idx 3 → 37; CS=1.99 → `ftol(2.98)` = idx 2 → 21. **Representation caveat:** this is
sensitive at `x.01`-type boundaries — gamemd loads CellSpread as **f32** before the `+0.99`, so a Rust
I16F16 port matches gamemd only where both representations land on the same integer. This holds for the
ENTIRE stock CellSpread set (integers/halves plus 0.1/0.3/0.4/0.9, all far from `x.01`), proven in
`stock_cellspread_values_match_gamemd_float_rule`; it does NOT hold for some modded non-stock values
(e.g. f32 `10.01+0.99 = 10.9999998 → ftol 10 → 309`, while exact/I16F16 gives `11 → 369` — so the §4c
"10.01→369" row is the *exact-arithmetic* value, not gamemd's f32 result; non-stock, surfaced not fixed).
CS≥11.99 indexes past the 12-entry table (OOB read) — undefined, not stock-reachable; document, do not
replicate without modeling the OOB.

### 5b. Lepton distance threshold
`threshold_leptons = ftol(CellSpread × 256.0)`. An object is damaged only if its 3D lepton distance
`d ≤ threshold_leptons`. The cell loop is a coarse pre-filter; this is the true radius gate. Examples:
CS=0.5 → 128 leptons; CS=2.0 → 512 leptons. **Both** quantities apply: a cell enumerated by the count loop
can still have its occupants rejected by the lepton threshold.

### 5c. Air/capture flag
`flag = (CellSpread > 0.5)` (strict `>` float compare). Enables the airborne-target scan and the
`dist < 0x55` (85 leptons) building-capture detection in the impact cell (idx 0 only).

### 5d. Shroud-reveal index (the cross-consumer with a DIFFERENT rule)
`RevealShroud`/`RevealAroundCell` use the SAME tables but: clamp radius to `MAX_SIGHT = 10`, use the
**raw integer** sight/radius as the count-table index (no `+0.99`, no `×256` lepton threshold), and apply
a Z-perspective cell shift before centering. The offset table is shared; the count-index rule is **not**.
(Re-verified live 2026-06-04: `decompile_function 0x005678E0` — `if (10 < param_3) param_3 = 10;
iStack_30 = (&DAT_007ed3d0)[param_3]; psStack_38 = &DAT_00abd490`.)
- **CORRECTION (2026-06-04): reveal HAS a fine filter, in cell-space.** Inside the loop the body computes
  `Sqrt_Approx((local_18-cx)² + (sStack_16-cy)²)` → `Math__ftol` → and only reveals the cell when
  `sVar4 <= param_3` (the clamped sight). So the count-band disk is further trimmed by
  `round(sqrt(dx²+dy²)) <= sight`. The earlier "no threshold" phrasing was an overstatement — both
  consumers fine-filter; splash uses `ftol(CS×256)` leptons, reveal uses `ftol(sqrt(dx²+dy²))` cells.
- **Fog-edge sub-offset:** when `Rules+0x17ee == 0 && param_5 != 0 && sight > 2`, reveal starts the walk
  at a non-zero offset into the table using `DAT_007ed3c4[sight]` (skips the inner band). This is the
  FogOfWar-gated edge path; `Rules+0x17ee` (= a FogOfWar flag) is the gate, so it is dormant in stock YR
  (FogOfWar off). The `reveal_cells` API in §6c must account for this when the vision port lands.
- (`MAPCLASS_GHIDRA_REPORT.md` §3, `PSYCHIC_REVEAL_SUPERWEAPON_GHIDRA_REPORT.md` §3. PSYCHIC_REVEAL doc's
  "Radius 0=1,1=5,2=13,3=21,…" per-ring list is WRONG — the real counts are the §1a table; that doc claim
  should be patched to cite `DAT_007ED3D0`.)

### 5e. Cell enumeration & per-cell effects (splash path)
- Loop `i = 0..count-1`. `cell = (centerCellX + dx[i], centerCellY + dy[i])`, `(dx,dy)=offset[i]`.
- Center→cell conversion uses a sign-fixed arithmetic shift for negative leptons:
  `cellX = (coord.X + ((coord.X>>31) & 0xFF)) >> 8` (verified `00489309-00489319`: `CDQ; AND EDX,0xff;
  ADD EAX,EDX; SAR EAX,0x8`); same for Y at `00489321-0048932d`.
- **Iteration order = the exact embedded table order, NOT sorted by d².** Idx 0 always first.
- Per cell, in scan order: ore/tiberium reduce (gated by `destroyTiberium` param + overlay isTiberium +
  vein/Wood checks), wall/overlay/fence destroy (gated by warhead `+0x145`/`+0x144`/(`+0x147` & material==6)),
  then collect occupants from the ground list (`cell+0xE4`) or bridge-deck list (`cell+0xE8`, chosen by the
  above-bridge flag set at `0048958d`).

### 5f. Target collection & distance
- Building (`WhatAmI==6`) in the impact cell (idx 0) → dist 0 (full damage); building in an outer cell →
  3D distance from its center; other objects → 3D distance object-center→impact.
- Aircraft in air (`WhatAmI==2 && IsInAir`) → `dist = (dist - sign) >> 1` i.e. **dist / 2** (verified
  `00489a70 MOV EAX,EDI; CDQ; SUB EAX,EDX; SAR EAX,0x1`) — effectively doubles AA splash range vs aircraft.
- Source object skipped unless C4Warhead (`Rules+0xfac`) or has self-heal. Building immune-list
  (`Rules+0xb40`) skipped when `ScenarioClass & 0x800`.

### 5g. Early-exit / boundary
- `damage==0` OR `ScenarioClass & 0x20` OR `warhead==NULL` → early return `true`, no enumeration
  (`004892a8`/`004892c9`/`004892d5`).
- After loop, returns `2` if a capture occurred, else `!captureFlag` (true normally) — never `0` for an
  empty target list.
- Off-map cells resolved by `Get_CellClass` (`0x005657a0`); the loop does not clamp dx/dy itself.

**Units/sign:** offsets are **cells** (signed int16, +X east / +Y south = sim convention); dx=low word,
dy=high word; threshold is in **leptons** (cells×256). Magic constants: 0.99 (count addend), 256.0
(lepton scale), 0.5 (air flag) — all three verified `read_memory` this session.

---

## 6. Rust-native substrate service — design

**One pure, read-only, deterministic substrate service** that owns the cell-spread family data and the
gamemd index/threshold semantics. Rust-native structure; gamemd-native semantics.

### 6a. Location & ownership
- New module: `src/sim/world/substrate/cell_spread.rs` (substrate-service tier under `sim/`, sibling to the
  other foundational helper-service slices added on this branch). It depends only on `core`/`std` and
  `util` (lepton/fixed). It must NOT depend on `render/ui/audio/net` (layering invariant preserved — it is
  pure data + arithmetic).
- Retire `src/sim/combat/cell_spread.rs` (move/replace; see §7). Combat consumers call the substrate service.
- The service is the **single owner** of: the 369-entry offset table, the 12-entry count table, and the
  three rounding/threshold helpers. No consumer recomputes the radius rule.

### 6b. Construction source
- **Const embedded table from the gamemd dump** (not generated, not INI, not map-derived). The 369 `(i16,i16)`
  entries are baked verbatim from the verified initializer dump (`CELLSPREAD_OFFSET_TABLE_DUMP` §5, including
  the R6/R11 byte-exact values). The count table is the verified `[1,9,21,37,61,89,121,161,205,253,309,369]`.
  Embedding (vs the current `compute_spread_offsets` sort) is the ONLY way to match order + element-set
  (§4b proves the generated table cannot match). The R11 duplicate is preserved verbatim as data (it is the
  real gamemd table); whether to expose it is a consumer policy, not a table edit.
- `CellSpread` itself stays INI-parsed (`warhead_type.rs`) — the service only consumes the parsed value.

### 6c. API surface (signatures; final names at implementation time)
```
// Pure data accessors — deterministic, no I/O, no state.
pub fn count_table() -> &'static [u32; 12];                 // = DAT_007ED3D0
pub fn offset_table() -> &'static [(i16, i16); 369];        // = DAT_00ABD490, exact order

// Splash/area-damage index rule (gamemd Apply_area_damage).
pub fn splash_count_index(cell_spread: SimFixed) -> usize;  // ftol(CS + 0.99), unclamped (caller guards OOB)
pub fn splash_cells(cell_spread: SimFixed) -> &'static [(i16,i16)]; // offset_table[..count[index]]
pub fn splash_threshold_leptons(cell_spread: SimFixed) -> i64;      // ftol(CS * 256)
pub fn splash_air_flag(cell_spread: SimFixed) -> bool;             // CS > 0.5

// Shroud-reveal index rule (gamemd RevealAroundCell) — shared offset table, different index.
pub fn reveal_count_index(sight_cells: u32) -> usize;       // min(sight, 10)
pub fn reveal_cells(sight_cells: u32) -> &'static [(i16,i16)];
// NOTE (2026-06-04 re-check): reveal_cells gives the count-band slice ONLY. The reveal consumer must
// ALSO apply the per-cell cell-space gate `ftol(sqrt(dx²+dy²)) <= sight` (verified in RevealAroundCell
// 0x005678E0) — the disk is trimmed at corners. A `reveal_cell_passes(dx,dy,sight)->bool` helper or a
// pre-filtered slice is required; the raw slice alone over-reveals corner cells.
```
- `splash_count_index` must implement `ftol(CS + 0.99)` exactly: compute in fixed point as
  `(cell_spread + 0.99).to_num_floor()` equivalent, i.e. add the fixed-point representation of 0.99 then
  truncate toward zero — NOT `to_num::<u32>()` on raw CS. (Acceptance test §8 pins every boundary.)
- `splash_threshold_leptons` must be `ftol(CS × 256)` = `(cell_spread × 256).to_num_trunc::<i64>()`, NOT
  `to_num::<i64>() × 256` (the current bug).
- OOB policy: `splash_count_index` returns the raw index; `splash_cells` asserts/clamps index ≤ 11 (stock
  unreachable >10; document OOB as undefined). Keep the unclamped index available so a future OOB-faithful
  mode is possible without API change.

### 6d. Determinism guarantees
- All inputs are `SimFixed`/`u32`; all arithmetic is fixed-point/integer; tables are `'static` const data →
  bit-identical across platforms and runs (lockstep-safe). No float, no allocation, no global mutable state.
- Returned slices are borrows of the const tables (no per-call Vec build; the current `LazyLock<Vec<…>>` is
  replaced by a plain `const`/`static` array).

---

## 7. Retire list (exact file:line)

| Rust artifact | file:line | Why retired / replaced by |
|---|---|---|
| `compute_spread_offsets()` (Rust-generated sort) | `src/sim/combat/cell_spread.rs:30-64` | Cannot match gamemd order/element-set (§4b). Replaced by const embedded `offset_table()`. |
| `SPREAD_OFFSETS: LazyLock<Vec<(i16,i16)>>` | `src/sim/combat/cell_spread.rs:21` | Replaced by `static OFFSET_TABLE: [(i16,i16);369]` const data. |
| `CELL_SPREAD_COUNTS` const | `src/sim/combat/cell_spread.rs:14` | Moved into the service as `count_table()` (value unchanged — already correct). |
| `cells_in_spread(radius: u32)` | `src/sim/combat/cell_spread.rs:71-75` | Split into `splash_cells(SimFixed)` / `reveal_cells(u32)` so the index rule lives in the service, not the caller. |
| Radius derivation `cell_spread.to_num::<u32>()` (splash) | `src/sim/combat/combat_aoe.rs:104` | Wrong rule (floor vs `ftol(CS+0.99)`); replaced by `splash_cells(cell_spread)`. |
| Radius derivation `cell_spread.to_num::<u32>()` (ore) | `src/sim/combat/mod.rs:1162` | Same wrong rule duplicated; replaced by `splash_cells(cell_spread)`. |
| Threshold `cell_spread.to_num::<i64>() * 256` | `src/sim/combat/combat_aoe.rs:96` | Wrong rule (`floor(CS)×256` vs `ftol(CS×256)`); replaced by `splash_threshold_leptons(cell_spread)`. |
| `if cell_spread <= SIM_ZERO { return Vec::new() }` early-return | `src/sim/combat/combat_aoe.rs:91` | Diverges from gamemd "CS=0 scans impact cell via table" (§4e); reconcile against the direct-hit path during migration (may stay as an optimization IF proven output-equivalent, else replace). |
| `clamp to MAX_SPREAD_RADIUS` in `cells_in_spread` | `src/sim/combat/cell_spread.rs:72` | Combat path is UNCLAMPED in gamemd; clamp belongs only to the reveal index helper (§5d). |
| File `src/sim/combat/cell_spread.rs` + `pub(crate) mod cell_spread;` decl | `src/sim/combat/mod.rs:20` | Whole module relocates to `sim/world/substrate/cell_spread.rs`. |

No other file embeds a duplicate offset/count table (Grep: the only literal `[1,9,21,…]` is `cell_spread.rs:14`
and its test mirror `:83`; `lightning_cell_spread` in `ruleset.rs` is an unrelated INI scalar, not this table).

---

## 8. Migration slices + acceptance tests

Ordered, each independently shippable. Slices 1–2 are **pure data parity** (no behavior change beyond the
table data + index rule). Slices 3–4 touch consumer behavior and need end-to-end output tests.

### Slice 1 — Embed the gamemd offset table + count table as const data (pure data)
- Create `sim/world/substrate/cell_spread.rs` with `OFFSET_TABLE: [(i16,i16);369]` (verbatim from dump §5,
  R6/R11 defects preserved) and `COUNT_TABLE: [u32;12]`. Expose `offset_table()`/`count_table()`.
- **Acceptance (exact-table-equality):**
  - `test_count_table_exact`: `COUNT_TABLE == [1,9,21,37,61,89,121,161,205,253,309,369]` (vs `read_memory 0x007ED3D0`).
  - `test_offset_idx0_is_origin`: `OFFSET_TABLE[0] == (0,0)`.
  - `test_offset_R1_exact_order`: `OFFSET_TABLE[1..9] == [(1,-1),(0,-1),(-1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)]` (verified sweep).
  - `test_offset_band_boundaries`: for each R, `OFFSET_TABLE[..COUNT_TABLE[R]]` length == count; band-start
    entries match dump §5 (R7 idx121=(-1,-7), R8 idx161=(-1,-8), R9 idx205=(-1,-9), R10 idx253=(-1,-10), R11 idx309=(0,11)).
  - `test_R11_duplicate_preserved`: `OFFSET_TABLE[322] == OFFSET_TABLE[319] == (-3,11)` AND `(3,-11)` absent
    (pins the verbatim gamemd defect — a regression guard, not a "fix").

### Slice 2 — Splash index/threshold/air-flag helpers (pure rule parity)
- Add `splash_count_index`, `splash_cells`, `splash_threshold_leptons`, `splash_air_flag`.
- **Acceptance (boundary-inclusive, exact-output vs gamemd ftol):**
  - `test_splash_count_index`: assert only representation-stable integers/halves — `0.0→1, 0.5→9, 1.0→9,
    1.5→21, 2.0→21, 2.5→37, 3.0→37, 9.0→253, 10.0→309`. (Each is `count[ftol(CS+0.99)]`.) Do NOT assert
    sub-0.01 fractions (e.g. `2.001`, which stays at idx 2 → 21, not 37) or `10.01`/`x.01` values — those
    are f32-vs-I16F16 representation-sensitive; the stock set is proven in
    `stock_cellspread_values_match_gamemd_float_rule` instead.
  - `test_splash_threshold_leptons`: `0.0→0, 0.5→128, 1.0→256, 2.0→512, 2.5→640, 10.0→2560`; and a
    fractional non-half `0.1→25` (`ftol(25.6)=25`) to pin truncation direction.
  - `test_splash_air_flag`: `0.5→false` (strict `>`), `0.5+ε→true`, `0.0→false`, `1.0→true`.
  - `test_negative_and_zero_guard`: CS≤0 index → 0 (count 1); document OOB CS≥11.99.

### Slice 3 — Repoint splash damage consumer (`combat_aoe.rs`)
- Replace `to_num::<u32>()` radius (`:104`) with `splash_cells`; replace `to_num::<i64>()*256` threshold
  (`:96`) with `splash_threshold_leptons`. Reconcile the `cell_spread <= 0` early-return (`:91`) against the
  direct-hit path: either prove output-equivalence (then keep as an optimization with a comment + test) or
  route CS=0 through `splash_cells` (1 cell).
- **Acceptance (exact end-to-end output):**
  - `test_aoe_cs_half_block`: CS=0.5 warhead detonates with units at impact and at the 4 orthogonal neighbors;
    assert damage set matches gamemd (impact cell occupant damaged; neighbors only if within 128 leptons —
    cell-center neighbors at 256 leptons are rejected by threshold → only impact damaged). This pins that the
    count grows to 9 but the lepton filter still bounds it (the two-quantity contract §5b).
  - `test_aoe_cs_1_5`: CS=1.5 → 21 cells scanned, threshold 384 leptons; unit at exactly 1 cell (256 lep)
    damaged, unit at 2 cells (512 lep) rejected. Compare full damage list ordering to the embedded-table order.
  - `test_aoe_target_order`: 3 units in the disk → damage-list order equals embedded-table scan order
    (not the old sorted order) — guards the RNG/chain-order parity (§4b).

### Slice 4 — Repoint ore-destruction consumer (`mod.rs::destroy_ore_at_impact`)
- Replace `to_num::<u32>()` radius (`:1162`) with `splash_cells`.
- **Acceptance:**
  - `test_ore_cs_half`: CS=0.5 ore warhead → 9 reduction requests (3×3 block) at the embedded-table cells —
    currently Rust emits 1 (regression-fixing test; pins the §4c DRIFT closed). (Ore path has NO lepton filter,
    so all 9 cells get a request — this is the clean way to demonstrate the count-rule fix.)
  - `test_ore_cs_2_cells`: CS=2 → 21 reduction requests at exactly the embedded-table offsets, in order.

### Slice 5 (deferred / cross-family) — Shroud-reveal consumer reuses the table
- When the vision/shroud reveal path is ported in Rust, add `reveal_count_index`/`reveal_cells` and point
  `RevealAroundCell`-equivalent at the SAME `offset_table()` with the clamp-to-10 index.
- **Acceptance:** `test_reveal_index_clamp`: sight 5→count[5]=89, sight 10→309, sight 15→clamped to 10→309.
  `test_reveal_shares_offset_table`: `reveal_cells(3)` and `splash_cells(3.0)` return identical slices
  (proves single shared table). This slice is gated on the vision port; flagged for synthesis, not shippable now.

---

## Anchors & Evidence

| Address / symbol | Ghidra call cited (this session) | Doc cross-ref |
|---|---|---|
| `Apply_area_damage 0x00489280` (full body, both indices, air-halve, cell loop) | `disassemble_function 0x00489280` | `CELLSPREAD_OFFSET_TABLE_DUMP_GHIDRA_REPORT.md`, `WARHEAD_DETONATE_GHIDRA_REPORT.md` §4, `TERRAIN_CLASS_GHIDRA_REPORT.md` §13.4/15.6, `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` §8 |
| Count table `0x007ED3D0` | `read_memory 0x007ED3D0` (48 B) | dump §2 |
| Offset table `0x00ABD490`/`0x00ABD492` (reader binding) | `disassemble_function 0x00489280` @ `004895C7/CF`, `004899C7` | dump §1, §5 |
| Count addend `0x007E5160` = 0.99 | `read_memory 0x007E5160` = `ae47e17a14aeef3f` | new this session (corrects dump §4 `ftol(CS)`) |
| Distance scale `0x007E2224` = 256.0 | `read_memory 0x007E2224` = `00008043` | `AAHEATSEEKER2_GUARDWH…` §3.6 |
| Air flag `0x007E5168` = 0.5 | `read_memory 0x007E5168` = `0000003f` | new this session |
| Offset table contents (idx0, R1 sweep, idx96, R11 dup) | `decompile_function 0x00561910` (initializer body, re-verified 2026-06-04) — static `read_memory 0x00ABD490` is BSS-zero, contents exist only at runtime | dump §1, §5 |
| Shroud reveal `RevealAroundCell 0x005678E0` (shared tables, clamp-10, per-cell `ftol(sqrt)` gate, fog-edge sub-offset) | `decompile_function 0x005678E0` (re-verified live 2026-06-04) | `MAPCLASS_GHIDRA_REPORT.md` §3, `PSYCHIC_REVEAL_SUPERWEAPON_GHIDRA_REPORT.md` §3, `REVEAL_Z_SHIFT_GHIDRA_REPORT.md` |
| `SimFixed = I16F16`, `to_num` truncation | `Read src/util/fixed_math.rs:23` | — |

---

## DRIFT Ledger

| Rust file:line | Current | gamemd-correct | Severity (with trigger-frequency) |
|---|---|---|---|
| `combat/cell_spread.rs:30-64` `compute_spread_offsets` | Generated sort key `(d²,\|dx\|,\|dy\|,dy,dx)`; 363/369 positions differ; element-set differs at R6,8,9,10,11 | Embed verbatim 369-entry gamemd offset table | **HIGH** — order fires every match any CellSpread≥1 warhead hits ≥2 targets/ore/walls (chain + RNG-order); element-set fires every match a CellSpread≥6 SW (nuke/weather/dominator) detonates near a boundary cell. |
| `combat_aoe.rs:104`, `mod.rs:1162` `cell_spread.to_num::<u32>()` | `floor(CS)` radius index | `ftol(CS + 0.99)` (`0x007E5160`) | **HIGH** — diverges for every fractional-CellSpread warhead (stock `.5` family); fires every time one detonates (CS=0.5: gamemd 9 cells, Rust 1). |
| `combat_aoe.rs:96` `to_num::<i64>() * 256` | `floor(CS) × 256` leptons | `ftol(CS × 256)` (`0x007E2224`) | **HIGH** — collapses the fine filter to 0 leptons for CS<1.0 and rounds down any fractional CS; fires every fractional-CS detonation (CS=0.5 → Rust rejects all targets incl. impact). |
| `combat_aoe.rs:91` `if cell_spread <= SIM_ZERO return` | CS=0 → zero AoE cells (delegated elsewhere) | CS=0 → scan 1 cell (impact) via the table, damage the impact occupant | **MEDIUM/UNCHECKED** — every CS=0 warhead (most small-arms); net output depends on the separate direct-hit path; not proven equivalent → DRIFT until proven. |
| `combat_aoe.rs:138-154` airborne path; no CS>0.5 gate | Iterates entities unconditionally; no capture detection | Air-scan + capture gated by `CellSpread > 0.5` (`0x007E5168`), capture `dist<0x55` | **MEDIUM/UNCHECKED** — affects AA splash vs aircraft and infantry-capture-by-blast; fires whenever a blast lands near aircraft / capturable. |
| `combat_aoe.rs:185,293` `from_num(dist_leptons/256)` | Integer-cell distance into falloff (sub-cell precision lost) | Lepton-space distance compare | **LOW/cross-family** — falloff rounding, fires every AoE hit but small magnitude; belongs to combat-falloff family, flagged for that synthesis. |
| `cell_spread.rs:72` clamp to MAX_SPREAD_RADIUS in splash path | Splash radius clamped to 11 | Splash reader is UNCLAMPED (only shroud clamps to 10) | **LOW** — only matters for modded CS>11 (OOB); stock-unreachable; surface, don't silently clamp in the splash helper. |
| Doc `CELLSPREAD_OFFSET_TABLE_DUMP §4` `ftol(CS)` | States count index = `ftol(CS)` | `ftol(CS+0.99)` count + `ftol(CS×256)` threshold | **doc-staleness** — patch the dump doc and `TERRAIN_CLASS §15.6` (same wrong claim). |
| Doc `CELLSPREAD_OFFSET_TABLE_DUMP §5` idx96 "ANOMALY (-5,-4)" | False anomaly | idx96 = `(-4,-4)`, R6 symmetric | **doc-staleness** — patch. |
| Doc `PSYCHIC_REVEAL §6` per-ring counts `1,5,13,21,…` | Wrong arithmetic series | Real counts = `DAT_007ED3D0` `[1,9,21,37,…]` | **doc-staleness** — patch. |

---

## Verification Log (adversarial re-check, 2026-06-04)

Method: assume each load-bearing claim WRONG until the binary proves it; re-verify live this session.
Default verdict on anything not proven this session = UNVERIFIED. Ghidra MCP call cited per claim.

| # | Claim re-checked | Verdict | Evidence (Ghidra MCP call this session) |
|---|---|---|---|
| 1 | Count table `0x007ED3D0` = `[1,9,21,37,61,89,121,161,205,253,309,369]` | **VERIFIED** | `read_memory 0x007ED3D0` (48 B) = `01..09..15..25..3d..59..79..a1..cd..fd..3501..7101` → exactly those 12 values. §4a MATCH stands. |
| 2 | Count addend `0x007E5160` = double 0.99 | **VERIFIED** | `read_memory 0x007E5160` = `ae47e17a14aeef3f` = 0.99. |
| 3 | Distance scale `0x007E2224` = float 256.0 | **VERIFIED** | `read_memory 0x007E2224` = `00008043` = 256.0. |
| 4 | Air-flag threshold `0x007E5168` = float 0.5 | **VERIFIED** | `read_memory 0x007E5168` = `0000003f` = 0.5. |
| 5 | Count index = `ftol(CS+0.99)`; `count = DAT_007ED3D0[idx]` (§4c/§5a) | **VERIFIED** | `disassemble_function 0x00489280`: `00489592 FLD [ESI+0x124]`; `00489598 FADD double [0x007e5160]`; `0048959e CALL 0x007c5f00`; `004895a3 MOV ECX,[EAX*0x4+0x7ed3d0]`. |
| 6 | Lepton threshold = `ftol(CS×256)`; compared per-target (§4d/§5b) | **VERIFIED** | `004892dd FLD [ESI+0x124]`; `004892e3 FMUL [0x007e2224]`; `004892e9 CALL 0x007c5f00`; stored `[ESP+0x68]`; compared `00489475 CMP EAX,[ESP+0x68]` and `00489a91 CMP EDI,[ESP+0x68]`. |
| 7 | Air/capture flag = `CS > 0.5` (strict `>`) (§5c) | **VERIFIED** | `00489347 FLD [ESI+0x124]`; `0048934d FCOMP [0x007e5168]`; `FNSTSW`/`TEST AH,0x41` carry/zero check sets the flag. |
| 8 | `0x007c5f00` is ftol (truncate-toward-zero float→int) | **VERIFIED** | `decompile_function 0x007c5f00` = `Math__ftol` returning `(longlong)ROUND(ST0)` (VC6 runtime ftol, truncate-toward-zero). The load-bearing `+0.99`/`×256` arithmetic is what differs from Rust floor; that is directly in the disasm. |
| 9 | Offset reader binding: dx=`[EAX*4+0xABD490]` low16, dy=`[EAX*4+0xABD492]` high16, +center, loop term `[EDX*4+0x7ed3d0]` (§1b) | **VERIFIED** | `004895c7 MOV DX,[EAX*0x4+0xabd490]`; `004895cf MOV AX,[EAX*0x4+0xabd492]`; `004895d7 ADD DX,[ESP+0x18]`; `004895dc ADD AX,[ESP+0x1a]`; `004899c7 MOV ECX,[EDX*0x4+0x7ed3d0]`. |
| 10 | Offset table idx 0 = `(0,0)` (§4b/Slice1) | **VERIFIED** | `decompile_function 0x00561910`: `_DAT_00abd490 = 0` (dx=0, dy=0). |
| 11 | R1 sweep idx1..8 = `[(1,-1),(0,-1),(-1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)]` (§4b/Slice1) | **VERIFIED** | `0x00561910` body: `abd494=0xffff0001`=(1,-1), `abd498=0xffff0000`=(0,-1), `abd49c=0xffffffff`=(-1,-1), `abd4a0=0x0000ffff`=(-1,0), `abd4a4=1`=(1,0), `abd4a8=0x0001ffff`=(-1,1), `abd4ac=0x00010000`=(0,1), `abd4b0=0x00010001`=(1,1). Exact match. |
| 12 | dump §5 idx96 "ANOMALY (-5,-4)" is FALSE; correct = `(-4,-4)` (§4b R6 note) | **VERIFIED (correction confirmed)** | `0x00561910`: idx96 → `abd610 = 0xfffcfffc` = dx=-4, dy=-4 = `(-4,-4)`. The dump's `(-5,-4)` is wrong; the doc's correction is right. |
| 13 | R11 defect: idx319 = idx322 = `(-3,11)`; mirror `(3,-11)` absent (§4b/§3) | **VERIFIED** | `0x00561910`: idx319 `abd98c = MapCoord_Set(0xfffffffd,0xb)`=(-3,11); idx322 `abd998 = MapCoord_Set(0xfffffffd,0xb)`=(-3,11) — duplicate. No `MapCoord_Set(3,0xfffffff5)`=(3,-11) anywhere in the band-11 tail. |
| 14 | Offset table is BSS-populated-at-startup, not a static array | **VERIFIED (sharpens doc)** | `read_memory 0x00ABD490` (40 B) = all zeros in the static image → table contents are knowable ONLY via the initializer `0x00561910`. Confidence on the 369 entries upgraded HIGH→PROOFED because the initializer was re-decompiled this session. |
| 15 | 19 caller sites incl. stock SWs; every detonation routes here (§1c) | **VERIFIED** | `get_function_callers 0x00489280` returns 19: all named SWs/warhead/anim/terrain present; recursive self-call confirmed in disasm at `0048a371 CALL 0x00489280`. (4 are `FUN_`-unnamed; doc listed a subset, not a completeness claim.) |
| 16 | Sign-fixed center→cell shift `(coord+(coord>>31 & 0xFF))>>8` (§5e) | **VERIFIED** | `00489309 MOV EAX,[EDI]; 00489310 CDQ; 00489311 AND EDX,0xff; 00489317 ADD EAX,EDX; 00489319 SAR EAX,0x8` (X); `00489321-0048932d` (Y). |
| 17 | Aircraft-in-air `dist /= 2` via `(dist - sign) >> 1` (§3/§5f) | **VERIFIED** | `00489a60 CMP EAX,0x2; 00489a70 MOV EAX,EDI; 00489a72 CDQ; 00489a73 SUB EAX,EDX; 00489a75 SAR EAX,0x1`. |
| 18 | Early exits: damage==0 / `Scen & 0x20` / warhead==NULL → return true (§5g) | **VERIFIED** | `004892a8 CMP ESI,EBX → 004892be JZ 0x0048a4b7` (damage==0); `004892c9 TEST byte[EAX],0x20 → JNZ` (Scen&0x20); `004892d5 CMP ESI,EBX → 004892d7 JZ` (warhead NULL). All jump to `0x0048a4b7` which sets EAX=1. |
| 19 | Bridge gate = `Scen & 0x8000` AND `warhead+0x144`; BridgeStrength `Rules+0x1740`; bypass warhead `Rules+0xff0` (§3/§2) | **VERIFIED** | `00489eb2 TEST CH,0x80` (= ECX bit15 = 0x8000) `→ JZ`; `00489ebb MOV AL,[EBX+0x144]`; `00489fe0 MOV EAX,[EAX+0x1740]`; bypass cmps `00489e92/00489fd8/0048a15d/0048a229/0048a283` all `[..+0xff0]`. |
| 20 | Rules offsets: C4WH `+0xfac`, death-splash `+0xfa8` (recursive), immune-list `+0xb40`/`+0xb4c` (§2) | **VERIFIED** | `004892fc CMP ESI,[ECX+0xfac]`; `0048a363 MOV ECX,[EAX+0xfa8]` feeding `0048a371 CALL 0x00489280`; `0048973b MOV EDX,[EBX+0xb4c]`; `00489745 MOV EBX,[EBX+0xb40]`. |
| 21 | Building-capture detection `dist < 0x55` in impact cell (§3/§5c) | **VERIFIED** | `00489487 CMP EAX,0x55 → JGE` (idx-0 pre-pass); `00489962 CMP [EBX+0x4],0x55 → JGE` (in-loop). Both gated by the air-flag byte `[ESP+0x1d]`. |
| 22 | Shroud reveal shares `DAT_00abd490` + `DAT_007ed3d0`, clamps radius to 10, raw-int index, "no threshold" (§1d/§5d) | **WRONG (corrected in place)** | `decompile_function 0x005678E0`: shared tables ✓, `if (10 < param_3) param_3 = 10` ✓, `iStack_30 = (&DAT_007ed3d0)[param_3]` ✓ — BUT reveal ALSO applies a per-cell **cell-space** gate `Sqrt_Approx(dx²+dy²)` → `Math__ftol` → `if (sVar4 <= param_3)`. The "no threshold / filter-free" phrasing was an overstatement. Also a fog-edge sub-offset `DAT_007ed3c4[sight]` when `Rules+0x17ee==0 && param_5!=0 && sight>2` (FogOfWar-gated, dormant in stock YR). §1d/§5d/§6c and the confidence summary patched. |

**Net:** 21 VERIFIED, 1 WRONG-corrected (claim 22, reveal fine-filter — corrected in §1d/§5d/§6c + confidence
summary), 0 UNVERIFIABLE. The three DRIFT pillars (offset-table order/element-set, `ftol(CS+0.99)` count
index, `ftol(CS×256)` lepton threshold) and all stage-2 migration recommendations (Slices 1–4) survive the
adversarial re-check intact. Slice 5 (deferred reveal consumer) gains a REQUIRED addition: a per-cell
`ftol(sqrt(dx²+dy²)) <= sight` gate (and a fog-edge sub-offset for the FogOfWar path), so its acceptance
tests `test_reveal_index_clamp` / `test_reveal_shares_offset_table` are necessary but NOT sufficient —
add a corner-cell trim test (e.g. sight=2: `(2,-2)` d²=8, `ftol(sqrt 8)=3 > 2` → excluded).
