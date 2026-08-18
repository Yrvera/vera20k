# RMG Mode-3/4 Water Shapers, Bridge Pass & Tech Placement — Ghidra Research Report

**Addresses:** `0x0059D510` (river carver), `0x0059C920` (lake grower), `0x004A8BF0`, `0x0058F0C0`, `0x005905D0`, `0x00595400`, plus helper quartet `0x005A0160` / `0x005A08D0` / `0x005A0410` / `0x0059E740` / `0x0057A0C0` and bounded contract for `0x00579010`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the six functions named in the task, their formula-level internals, RNG consumption, and one-level contracts of their callees; plus the driver/entry gating around them
**Non-Scope:** `0x0058EBC0` region-rebuild/cull driver + `0x0058D620` region split (TERRAIN_SHAPING doc — note the original header called this pair "terracing", which mislabels `0x0058EBC0`; see §7), `0x005A19E0`/`0x005A17F0` (decoded there), `0x0059A6C0` standard seeding (WATER_SEED doc), `0x0059C630` finalizer (WATER_SEED doc), full internals of `MapClass__PlaceBridgeRamp_Low 0x00579010` and `MapClass__ApplyBridgeTile` (bounded, deferred), hills/LAT/tiberium phases
**Confidence:** Mixed — see the AUDIT banner below. The §3/§4.1/§4.2/§4.3 formulas are now
re-verified instruction-by-instruction; §4.4–§4.7 were originally written from uncited subagent
prose and have only been spot-checked.
**Active in YR:** Conditional — maptype (`MapSeed+0x3C`) ∈ {3,4} for the water/bridge path (player picks it in the random-map dialog); tech path runs for any maptype ≠ 0

---

## AUDIT BANNER — 2026-07-25

This document was audited against the live binary on 2026-07-25 (gamemd.exe, image base
`00400000`, confirmed via `get_current_program_info`). Corrections are inlined below and
each carries the MCP call that proves it. Summary of what changed:

- **§4.1 C.3 straightness-flag kill rule was INVERTED** — corrected.
- **§4.1 C.6 "recenter arm live" for the branch angle is DEAD CODE** — corrected.
- **§5 / §4.1 Gaussian draw accounting was WRONG** — `FUN_005980C0` is Marsaglia *polar*
  with a rejection loop, not a fixed 2-draw Box-Muller. This is a stream-desync-class error.
- **§5's ledger is scope-local, not per-generation** — it omits every draw in the region
  phase and after. Relabelled.
- **§6 "WaterAmount is a dialog field" is WRONG** — it is a derived/randomised field.
- **§2.1 `+0x308` has no constructor default** — corrected.
- **§4.3 `0x0059E740` dirs 2/4 are NOT 90° rotations of dir 0** — corrected.
- `0x0057A0C0`'s role as documented here (RMG shore finalization, label drift) was
  **challenged during this audit and re-confirmed from the callee bodies** — see §4.3.
- A new **§12 Unverified** section collects everything that could not be closed.

Anything below still marked `[UNVERIFIED]` or living in §12 must not be implemented from.

## 1. Overview

For maptype 3/4 ("river"/"lake" styles) the RMG replaces the standard all-water
seeding with a single driver call that carves **at most one river system**
(channel + optional branch + optional bridge + optional canyon-sinking +
optional terminal lake) and then grows **at most one standalone lake**, both
capped by a WaterAmount-derived cell quota. A later bridge pass rebuilds
regions and runs two per-region passes: a no-RNG neighbor/area annotator and a
connector pass that carves ramps between different-level land regions and lays
low-bridge decks across water regions. Tech buildings are placed from
`[AI] NeutralTechBuildings` — per-region for maptype 2, at uniform random cells
otherwise.

## 2. State Layouts / Key Fields

### 2.1 MapSeedClass global `0x00ABDFD8` — corrected/extended fields

The global is constructed by the **outer** constructor `0x00595740`
(static initializer `0x0058B740`: `MOV ECX,0xABDFD8; CALL 0x00595740`, atexit
dtor `0x0058B760` — verified via disassemble_bytes 0x0058b73a). It wraps the
inner `0x00595680` and then sets extended defaults (verified via
decompile_function 0x00595740):

| Field | Default | Meaning (this slice) |
|---|---|---|
| +0x180/+0x184 | 0 | generated map width/height, internal scenario dims (prior docs; xrefs `0x0058EDA1`, `0x00594B64` read them as globals `0x00ABE158/5C`) |
| +0x2BC | **0x9C4 = 2500** | RMGMinimumTiberium default (INI overrides) |
| +0x2C0 | **0x157C = 5500** | RMGMaximumTiberium default |
| +0x2FC | **500** | MaxTrees default |
| +0x300 | 1 | byte, purpose outside slice |
| +0x304 | 0 | water cells placed so far (quota progress) |
| +0x308 | **(none — see note)** | water-region id counter |
| +0x30C | **4** | current base ground level, stepped by +4 per waterfall/canyon event; written into `CellClass+0x11B` on rollback |
| +0x310 | 0 | byte: river-bridge enable coin (see §3.1) |

**CORRECTED 2026-07-25 — `+0x308` has NO constructor default.** No instruction in
`0x00595740` (body `00595740`–`005958D3`) writes `[ESI+0x308]`; the field is left at
whatever `operator_new` returned. It is zeroed *per generation* inside `0x00599650`
(paired with `+0x304`, immediately before the bridge-coin draw) and again in the
`0x00598960` cleanup tail at `0x005993AF`/`0x005993B5`. A port must zero it at
generation start, not at construction. (verified 2026-07-25: full-body
`disassemble_function 0x00595740`; `disassemble_function 0x00599650`;
`disassemble_function 0x00598960`)

The remaining defaults in the table above are re-verified 2026-07-25 by
`disassemble_function 0x00595740` (read from disassembly, not decompiler pseudocode):
`+0x2BC = 0x9C4`, `+0x2C0 = 0x157C`, `+0x2FC = 0x1F4`, `+0x300 = 1` (byte),
`+0x304 = 0`, `+0x30C = 4`, `+0x310 = 0` (byte). The outer ctor's first call is the inner
ctor `0x00595680` (`MapSeedClass__Constructor`), which sets `+0x38 = 0` (theater),
`+0x3C = 1` (map type), `+0x40 = 1`, `+0x44 = 0`, `+0x48 = 1`, `+0x4C = 0` (water),
`+0x50 = 2` (players), `+0x54..+0x70 = 0`, `+0x74 = 0xFFFFFFFF` (seed = −1)
(verified 2026-07-25: `decompile_function 0x00595680`).

### 2.2 Scratch cells / regions / real cells (from prior docs + this slice)

- Scratch array `DAT_00ABED10`, stride 0x50, idx `(y*W+x)`, `W = g_PathfinderLinearMapWidth @ 0x0089C2DC`: `+0x00` packed own coord (i16 x, i16 y; (0,0) = invalid slot), `+0x38` region id, `+0x3C` flood/queued scratch id, `+0x40` neighbor-mask cache (shore pass), `+0x44` lake allow-mask byte, `+0x4B` meander flag (set by `0x005A08D0`, cleared on rollback).
- Diamond bounds test (everywhere, literal operators): `x+y > DAT_00ABED04 && x−y < DAT_00ABED04 && y−x < DAT_00ABED04 && x+y <= DAT_00ABED08`.
- Region objects (0x50 bytes; ptr array `DAT_00ABDF94`, count `DAT_00ABDFA0`, id counter `DAT_00ABED14`; ctor `0x0058BF70`): `+0x04` neighbor-id vector (written by `0x0058F0C0`), `+0x08` id, `+0x0C` area in valid cells, `+0x10` level, `+0x14` water/green flag byte, `+0x16` seed coord, `+0x1A`/`+0x1B` flag bytes, `+0x28` embedded cell vector (`+0x2C` items / `+0x38` count used by tech placement).
- Real `CellClass` (via `MapClass__Get_CellClass 0x005657A0`, ECX=`0x0087F7E8`): `+0x24/+0x26` coords, `+0x38` iso tile index (0 or 0xFFFF = empty/clear), `+0x44` overlay dword (low-bridge deck values), `+0x11A` subtile, `+0x11B` level byte (4 units per level step in this pipeline), `+0x11C` flag, `+0x11E` bridge cross-offset.
- `g_DirectionOffsets @ 0x0089F688` — runtime-initialized by `Foundation_direction_table_init 0x0049F2F0` (verified via decompile_function this session; static image zeros): entries 0..7 = N(0,−1), NE(1,−1), E(1,0), SE(1,1), S(0,1), SW(−1,1), W(−1,0), NW(−1,−1). Codes 0/2/4/6 = N/E/S/W.
- Water tile global read by both shapers = `[0x00AA0738]` (`g_WaterSet_TileSetBase`); green tile = `[0x00AA0E18]` (`g_GreenTile`).

### 2.3 FP constant pool (all read via read_memory this session)

| Address | Value | Used for |
|---|---|---|
| 0x007ED898 | f64 `0x3DF0000000100000` = 2^-32·(1+2^-32) ≈ 2.3283064370807974e-10 | rand→[0,1) scale (≈1/(2^32−1)) |
| 0x007ED9F8 / 0x007E2AC0 | 0.008 / 100.0 | water quota |
| 0x007ED9F0 / 0x007E1738 | 75.0 / 0.5 | lake size bounds / halving |
| 0x007ED764 / 768 / 76C | f32 0.5 / 10.0 / 0.02 | lake heap priority |
| 0x007EDA38 / 0x007EDA40 | 4·2^-32(1+ε) / 6·2^-32(1+ε) | U[0,3] / U[0,5] scales |
| 0x007EDA30 / 0x007E2820 / 0x007E3D88 / 0x007EDA28 / 0x007EDA08 | 7π/4, π/2, π/4, π/6, 5π/6 | river angles |
| 0x007EDA00 / 0x007E8AF0 | −π/10 / +π/10 | heading-wobble window check (σ=π/10 as inline immediate `0x3FD41B2F769CF0E0`) |
| 0x007E3CB8 | −0.5 | width-wobble window check (σ=0.5 inline immediate) |
| 0x007EDA20 / 0x007E1718 | 0.07 / 1.0 | river width scale/min |
| 0x007EDA18 / 0x007EDA10 | 91·2^-32(1+ε) / 35.0 | bridge min-step draw U[35,125] |
| 0x007E3808 / 0x007E44E8 / 0x007ED778 | 0.01 / 0.005 / 0.7 | branch chance / termination chance / waterfall gate |
| 0x007ED630 | f32 0.25 | bridge-enable coin |
| 0x007ED8C0 / 0x007ED8B8 / 0x007ED8B0 | 3·, 101·, 2·(2^-32(1+ε)) | tech pass count / ramp rolls |
| 0x00822D80 | CW 0x0E7F | `Math__ftol 0x007C5F00` control word → RC=11 = **truncate toward zero** (verified via disassemble_function 0x007c5f00 + read_memory) |

## 3. Entry Gating & Driver (`0x0059C580`)

Call site in `0x00598960` (verified via disassemble_bytes 0x00598ac0 + decompile_function 0x00598960):
- maptype ∉ {3,4} → `0x0059A6C0` (standard seeding). maptype ∈ {3,4} **and `WaterAmount(+0x4C) != 0`** → `0x0059C580(this)` — called **exactly once** per generation. Then `0x0059C630` always.
- The `WaterAmount > 20` gate applies only to the **river phase inside** the driver, not to the driver call.

Driver body (verified via decompile_function 0x0059c580):
```
+0x308 += 1                                   // region id 1 = first water region
if (maptype==3 || maptype==4) && WaterAmount > 0x14 (20):
    for attempt in 0..9:                      // up to 10 tries, stop on first success
        if FUN_0059D510(&{0,0}, 0.0, 0) != 0: { +0x308 += 1; break }
for attempt in 0..9:                          // always; up to 10 tries
    if FUN_0059C920(&{0,0}) != 0: { +0x308 += 1; break }
```
So a mode-3/4 map gets **≤1 river system and ≤1 standalone lake**. The quota
(§4.3) is a cap, not an iterated target: if the river already consumed it, all
10 lake attempts return false immediately.

### 3.1 Bridge-enable coin (+0x310)

In the "RMG: Init random map" phase, `0x00599650` (drifted label
"CCINIClass__Constructor"; sole caller `0x00598960 @ 0x00598A74`) zeroes
`+0x304/+0x308` and then draws **one `g_MapGenRng` number**:
`+0x310 = (rand·2^-32(1+ε) < 0.25f) ? 1 : 0`
(verified via disassemble_bytes 0x0059a4a0, ECX=0xABE890 at `0x0059A4B6`;
read_memory 0x007ed630). **Only ~25% of generated maps can ever get a river
bridge**, and this draw is consumed on every generation (any maptype path
through 0x00599650) — RNG-stream relevant.

## 4. Core Logic

### 4.1 `FUN_0059D510` — river carver (thiscall MapSeed; args: `&cell`, `f64 angle` (2 dwords), `u8 isBranch`; RET 0x10; returns u8)

Evidence: decompile_function 0x0059d510 + full disassembly 0x0059D510–0x0059E62F (two chunks, read in full) + read_memory constants. All `Random__Next 0x0065C780` sites load `ECX=0xABE890` (g_MapGenRng).

> **CORRECTED 2026-07-25 — the Gaussian helper.** `FUN_005980C0` (now labelled
> `RandomMapGenerator__NextGaussian`; `ECX` = state object `0x00ABDFB8`) is the Marsaglia
> **polar** method, **not** trigonometric Box-Muller, and it does **not** consume a fixed
> 2 draws per refill:
>
> - When the cache byte at `state+0x00` is clear it runs a **rejection loop** drawing
>   **two** uniforms per attempt and repeating until `s = u² + v²` satisfies `0 < s < 1`.
>   Acceptance probability is `π/4 ≈ 0.7854`, so the expected cost is **≈2.546 raw draws
>   with an unbounded tail** — roughly **21.5 % of refills reject at least once**.
> - It then computes `f = sqrt(−2·ln(s)/s)` (as `−2·ln2·log2(s)`), caches `v·f` at
>   `state+0x08`, sets the cache byte, and returns `u·f`. The *next* call returns the
>   cached value and consumes **zero** draws.
> - The uniform source is the function pointer at `state+0x10` (`0x00ABDFC8`), written
>   exactly once — at `0x0058B79B` — to `sub_00598000`, which is
>   `MOV ECX,0x00ABE890; CALL 0x0065C780; FILD qword{EAX,0}; FMUL [0x007ED898]; RET`.
>   So Gaussian draws **do** come from the same `g_MapGenRng` stream (that part of the
>   original claim holds), but the *count* is variable.
>
> A port that assumes "2 draws per refill" will desync the RNG stream on the first
> rejection. (verified 2026-07-25: `decompile_function 0x005980C0`;
> `search_instructions` operand `0x00abdfc8` → single writer at `0x0058B79B`;
> `disassemble_bytes 0x00598000`–`0x0059802F`; `read_memory 0x007ED898` =
> `0x3DF0000000100000`)

**A. Start selection** (only when `cell == (0,0)`, i.e. the driver call; branch/recursive calls pass explicit cell+angle):
1. `edge = ftol(rand · 4·2^-32(1+ε))`, redraw while `> 3` (practically never).
2. `rx = ftol(rand · W · 2^-32(1+ε))`, redraw while `> W−1`; same for `ry` with H. (W/H = `DAT_0087F8DC/E0` map extent.)
3. Start cell by edge (diamond-border parameterization, packed (x,y)):
   - edge 0: `(rx+1, W−rx)` — heading mean 7π/4
   - edge 1: `(W+H−1−ry, H−ry)` — mean 5π/4
   - edge 2: `(W+H−1−rx, H+rx)` — mean 3π/4
   - edge 3: `(ry+1, W+ry)` — mean π/4
   (mean = 7π/4 − edge·π/2)
4. Heading = bounded-Gaussian: `h = gauss·(π/6) + mean`, redrawn until `mean−π/4 ≤ h ≤ mean+π/4`. (The recenter branch — σ=(hi−lo)/2, mean=σ+lo when the ±π/6 band doesn't fit — is dead here but present; the same inline pattern recurs below where it CAN fire.)
5. Start cell must pass the diamond test or return 0 immediately (no cleanup — nothing carved yet).

**B. Width setup:**
- `wMax = ftol(max(WaterAmount·0.07, 1.0))` → e.g. WA 21→1, 50→3, 100→7.
- `w = ftol(rand·wMax·2^-32(1+ε) + 1.0)`, redraw while `> wMax` → uniform {1..wMax}.
- `halfW = w/2` (signed, toward zero). Width-walk band `[w−halfW, w+halfW]`; walk variable `wf` starts at `w` (f64).
- `bridgeMinStep = ftol(rand·91·2^-32(1+ε) + 35.0)`, redraw while `> 125` → uniform {35..125}.
- `fx = x+0.5, fy = y+0.5`; heading clamp window `[h0−π/2, h0+π/2]` (h0 = initial heading, kept in a separate slot forever).

**C. Main step loop** (per iteration):
1. `FLD 1.0` sentinel pushed at loop head (see termination detail below). `sx=ftol(fx), sy=ftol(fy)`; break if (sx,sy) fails the diamond test or `alive==0`.
2. `s = Sin_lookup(h)`, `c = Cos_lookup(h)` (`0x004CAD00/0x004CACB0`).
3. **Carve cross-section**: `span = ftol(wf + 0.5)` (round-half-up of the width walk). Start at `(fx − (span−1)·c·0.5, fy − (span−1)·s·0.5)`, take `span` substeps advancing `(+c, +s)` (perpendicular to travel). Per substep cell (ftol both coords):
   - outside diamond → skip (not fatal);
   - scratch id == current id (`+0x308`) → re-carve (idempotent);
   - scratch id == 0 AND real cell `IsClearTile` (`0x00486380`: tile 0 or 0xFFFF) → carve: scratch`+0x38` = id; real `+0x38 = [0x00AA0738]`, `+0x11A = 0` (note: `+0x11B` untouched here);
   - scratch id == 0 AND NOT clear → **alive = 0** (rivers may not touch shore/green/anything pre-placed);
   - scratch id ∉ {0, id} → **alive = 0** (foreign water region).
   The cross-section always completes all substeps even after alive drops.
   Two straightness flags track whether all substeps stayed in one column /
   one row:
   - `[ESP+0x13]` = **column flag** — every substep kept the same `ftol(x)`
     (x accumulator `[ESP+0x68]`, advanced by `+c`).
   - `[ESP+0x10]` = **row flag** — every substep kept the same `ftol(y)`
     (y accumulator `[ESP+0x30]`, advanced by `+s`).

   **CORRECTED 2026-07-25 — the kill rule was stated inverted in the original
   text.** After the substep loop, `FLD |s|; FLD |c|; FCOMPP` (a
   *register-to-register* compare at `0x0059DC8C` — there is **no** memory
   operand, so the previous parenthetical "FCOMPP vs 0.0 @ 0x007E2800" was
   also wrong). `FNSTSW AX; TEST AH,1` tests C0, and C0 == 0 means
   `|c| ≥ |s|`:

   - `|s| ≤ |c|` → jump to `0x0059DC9C`, which clears **`[ESP+0x13]`, the
     COLUMN flag**, leaving the row flag as the usable one.
   - `|s| > |c|` → `0x0059DC95` clears **`[ESP+0x10]`, the ROW flag**.

   This is the physically consistent reading: the carve line runs along
   `(c, s)`, so when `|c|` dominates the substeps sweep across x and share a
   single y — a *row*. The direction codes in step 4 below are unchanged and
   were already correct; they are what makes the inversion detectable.
   `[0x007E2800]` is `0.0` (`read_memory`) and is the comparand of the two
   *direction* tests at `0x0059DD29` / `0x0059DD48`, not of this FCOMPP.
   (verified 2026-07-25: `disassemble_bytes 0x0059DC1B`–`0x0059DCBF`;
   `read_memory 0x007E2800`)
4. **Bridge attempt** — all of: `isBranch==0`; a straightness flag survives; `bridgeCount < 1`; `abs(ftol(h − h0))` as int `< π/4` (⇒ effectively `|h−h0| < 1.0` rad — the int-truncation makes the π/4 constant vacuous); `MapSeed+0x310 != 0`; `stepCount > bridgeMinStep` (strict). Direction code: column-straight → `s ≤ 0 ? 6(W) : 2(E)`; row-straight → `c ≤ 0 ? 4(S) : 0(N)`. Calls `FUN_0059E740(id, &sectionStart, &sectionEnd, dir, &okByte, &fx, &fy)` (§4.3; `ECX` = MapSeed `this` is implicit — the decompiled prototype has 8 params, the first being `this`). On okByte: `+0x308 += 1` (subsequent carve cells get the NEW id) and `bridgeCount = 1`.
   *All of step 4 re-verified 2026-07-25 (`disassemble_bytes 0x0059DCC0`–`0x0059DD8F`):
   `CMP [ESP+0x4C],1 / JGE` (bridgeCount gate); `FLD [ESP+0x88]; FSUB [ESP+0xB8];
   CALL ftol; CDQ/XOR/SUB` (abs) then `FILD; FCOMP [0x007E3D88]` — the operand is an
   **integer** reloaded via FILD, so the π/4 constant is indeed vacuous and the real test
   is `|h−h0| < 1.0` rad; `MOV AL, byte ptr [EBX+0x310]; TEST AL,AL; JZ` (coin, read as a
   byte); `CMP EDX,EAX; JLE` with EDX = stepCount, EAX = bridgeMinStep (strict `>`).
   The `&sectionStart` argument is `ftol(fx − (span−1)·c·0.5, …)` and `&sectionEnd` is
   `ftol(x_acc − c, y_acc − s)` = the last carved cell, confirming the A = start /
   B = end ordering that `FUN_0059E740`'s clearance rect assumes.*
5. **Advance**: `fx += s; fy −= c` (travel ⊥ to the carve line).
6. **Branch spawn**: draw rand; if `rand·2^-32(1+ε) < 0.01` AND alive AND `isBranch==0` AND `bridgeCount==0`: set own `isBranch=1` (parent can never branch again NOR bridge — the bridge gate reads the mutated flag); branch angle = bounded-Gaussian mean `h+π/2`, σ=π/6, window `[h+π/6, h+5π/6]`; branch start = one-past-the-end of the current cross-section (`ftol` of section start + span·(c,s)); recurse `FUN_0059D510(&cell, angle, 1)` with the **same region id**; the recursion's return value REPLACES the parent's alive flag — a failed branch kills (and rolls back) the whole river.
   **CORRECTED 2026-07-25 — the original text's "(recenter arm live)" is WRONG; this
   recenter arm is DEAD CODE.** The guard is
   `if (hi < mean − σ) || (mean + σ < lo)` with `mean = h+π/2`, `σ = π/6`,
   `lo = h+π/6`, `hi = h+5π/6`. Then `hi = h+2.618` vs `mean−σ = h+1.047` (never less),
   and `mean+σ = h+2.094` vs `lo = h+0.524` (never less) — neither disjunct can fire for
   any `h`. A port may omit the recenter here entirely, exactly as it may for the start
   heading in step A.4. (The recenter arms in steps 7 and 8 **can** fire and must be
   implemented.) (verified 2026-07-25: `decompile_function 0x0059D510`, branch-spawn block)
7. **Heading wobble** (only when `stepCount > 5`): bounded-Gaussian mean 0, σ=π/10, window `[h0−π/2−h, h0+π/2−h]` (recenter if the window misses ±π/10); `h += draw` — heading hard-clamped to h0±π/2 lifetime.
8. **Width wobble** (only when `halfW > 0` — **width-1 rivers never wobble**): mean 0, σ=0.5, window `[w−halfW−wf, w+halfW−wf]` (recenter if missing ±0.5); `wf += draw`.
9. **Termination**: draw rand, `stepCount += 1`; loop while `rand·2^-32(1+ε) ≥ 0.005`.

**D. Post-loop:**
- `stepCount < 0x28 (40)` → alive = 0 (strict `<`; JGE at 0x0059E1AD).
- **End-lake**: the FP compare `< 0.005` is re-run on whatever is on the FP stack — the rand product on the rand-terminated path, the loop-head `1.0` sentinel on the border/dead-break path. Hence **only rand-terminated rivers spawn a terminal lake**: if end cell (ftol fx, ftol fy) passes the diamond test and alive, call `FUN_0059C920(&endCell)`; a false return kills the river.
  *Sentinel confirmed 2026-07-25 (`disassemble_bytes 0x0059D99C`–`0x0059D9D7`): the loop
  head at `0x0059D9A3` is literally `FLD double ptr [0x007E1718]`, and `read_memory
  0x007E1718` = `0x3FF0000000000000` = 1.0. The border test that follows jumps out to
  `0x0059E1A8` leaving that 1.0 on the stack, so `1.0 < 0.005` is false. VERIFIED.*
- **Shore/green finalize** (only `isBranch==0` originally — the saved entry value, not the mutated one — and alive): `0x0057A0C0(id, 0)` (shore finalization, §4.3; ECX=0x87F7E8); **if that succeeds**, `FUN_005A0160(id, 1 ring, rect{0,0,0x200,0x200}, 0, 0)` runs and its result is stored as the new alive flag — but the green sweep that follows is **NOT** gated on it. Every map cell with scratch id == id and real tile ∈ {0, 0xFFFF} gets `+0x38 = [0x00AA0E18]` (green) regardless of whether the dilation returned 0. A failed dilation therefore paints green first and only then falls into the rollback path (which clears those same cells, so the net observable result is unchanged — but the write order differs and a port that hoists the green sweep under an `if dilation_ok` is not byte-faithful mid-pass). Failure of `0x0057A0C0` skips both. (corrected 2026-07-25: `decompile_function 0x0059D510` — the green `while` loop sits inside `if (MarkBridges…!= 0)`, *after* the `local_129 = FUN_005a0160(...)` assignment and outside any further test)
- **Waterfall/canyon** (only when `bridgeCount==0` AND alive AND `+0x30C == 4` AND `!isBranch`): draw rand; if `rand·2^-32(1+ε) < 0.7`: `FUN_005A08D0(id, 0.01f (imm 0x3C23D70A), rect{0,0,0x200,0x200}, &startCell, 1)` (meander arm from the river START); on success `FUN_005A0160(id, 6 rings, …, 0, 0)`; on success every cell NOT in region id gets real `+0x11B += 4` (the rest of the map is raised one level → the river becomes a canyon) and waterfallFlag=1. Since `+0x30C` defaults to **4** (§2.1), this branch is LIVE for the first successful river; on success `+0x30C += 4` (→8), so at most one canyon per map. If the 0.7 roll fails, flow falls through to the "no-bridge" dilation below.
- **Final dilation**: non-branch, alive, `bridgeCount==0`, no waterfall → `FUN_005A0160(id, 2, rect, 0, 0)`. With a bridge (`bridgeCount>0`) → `FUN_005A0160(id, 2, rect, 1, +0x30C)` — flag 1 makes the dilation **absorb id−1** (the pre-bridge segment) and stamp `+0x11B = +0x30C` on claimed cells. Branch calls skip all finalize/dilation (`isBranch` short-circuits to the success return).
- **Success**: if waterfallFlag `+0x30C += 4`; `+0x304 += stepCount`; return 1.
- **Rollback** (any failure): every cell with scratch id == id: scratch `+0x38=0, +0x4B=0`, real `+0x38=0, +0x11A=0, +0x11B=(u8)+0x30C` (restores base ground level 4); if `bridgeCount > 0` a second pass identically clears id−1. Return 0.

### 4.2 `FUN_0059C920` — lake grower (thiscall MapSeed; arg `&cell`; RET 4; returns u8)

Evidence: decompile_function + full disassemble_function 0x0059c920; read_memory constants.

> **AUDIT 2026-07-25 — §4.2 re-verified end to end and found ACCURATE.** Every numbered
> step below was re-derived from `decompile_function 0x0059C920` plus
> `disassemble_bytes 0x0059C92E`–`0x0059C98F` and `0x0059CA80`–`0x0059CAA7`. No
> corrections were needed. The only caveat is the Gaussian draw-count issue in step 6 —
> see the corrected box in §4.1.

1. **Quota**: `target = (WaterAmount==0) ? 0 : ftol( (+0x184) · (+0x180) · WaterAmount · 0.008 + 100.0 )` (FILD +0x184; FIMUL +0x180; FIMUL WA; FMUL 0.008; FADD 100.0). `remaining = target − (+0x304)`. If `remaining ≤ 0x4B (75)` → **return 0** (signed JLE) — this is the water-phase termination condition.
   *VERIFIED 2026-07-25 byte-for-byte (`disassemble_bytes 0x0059C92E`–`0x0059C98F`):*
   `MOV EAX,[EBX+0x4C]; TEST EAX,EAX; JZ` → `FILD [EBX+0x184]; FIMUL [EBX+0x180];`
   `FIMUL [ESP+0x50]; FMUL [0x007ED9F8]; FADD [0x007E2AC0]; CALL 0x007C5F00` then
   `SUB EAX,[EBX+0x304]; CMP EAX,0x4B; JLE 0x0059D42C`. The operand order (height ×
   width × WA) and the `≤ 75` early-out are exactly as documented.
2. Alloc: queue = `new[(max(2·remaining+2, 100))·8]` entries {packed coord, f32 priority}; 1-based binary min-heap struct {count, cap=max(2r+2,100), data=new[(cap+1)·4] zeroed inclusive, maxPtr=0, minPtr=−1}; sift-down helper `FUN_005AD870`. Heap pushes are silently dropped when `count+1 ≥ cap` (guarded insert).
3. Zero scratch `+0x3C` for every real map cell (MapClass iterator ECX=0x87F7E8).
4. **Allow-mask** (driver call, cell==(0,0)): zero `+0x44` for all W² entries; `FUN_005A0410(0, 2, −2)` *(argument values VERIFIED 2026-07-25: `disassemble_bytes 0x0059CA92` = `PUSH -0x2; PUSH 0x2; PUSH 0x0; MOV ECX,EBX; CALL 0x005A0410`)* — peels the 2-ring band of empty (id 0) cells bordering existing water, marking them id −2 and flattening their tiles (§4.3; note that `FUN_005A0410`'s body is itself unverified — §12.1); then per entry: id==0 → `+0x44=1`; id==`+0x308` → `+0x44=1`; id==−2 → `+0x38=0` (mask stays 0). Net rule: **lakes may grow on empty cells ≥3 cells (8-dir) from foreign water, or on own-region cells**. Non-driver calls (river end-lake): `+0x44=1` for ALL cells (no restriction).
5. **Seed pick** (driver call): up to **200 attempts** (counter pre-incremented; JGE 0xC8 → return 0): `rx=U[0,W−1], ry=U[0,H−1]` (rejection loops as in the river), seed = `(rx+ry+1, ry+W−rx)`; accept iff scratch `+0x38==0` AND `IsClearTile` AND `+0x44 != 0`. Non-driver: seed = given cell, unconditionally.
6. **Size draw**: `upper = max(76, remaining)` (= remaining, since remaining ≥ 76); mean = `remaining/3`, σ = `remaining/6` (signed magic-multiply divisions, toward zero); if `mean−σ > upper || mean+σ < 75.0` recenter: σ=(upper−75)·0.5, mean=σ+75 (fires for remaining < 150); `size = ftol(bounded-gauss ∈ [75.0, upper])`.
7. **Growth**: push seed (priority 0, scratch `+0x3C=id`). Loop while `placed < size`, queue nonempty, alive: pop min → scratch `+0x38=id`, real `+0x38=[0x00AA0738]`, `+0x11A=0`. For neighbor dirs {0,2,4,6} (N,E,S,W — `g_DirectionOffsets[dir&7]`): in-diamond AND scratch`+0x38==0` AND `+0x3C != id` AND `IsClearTile` AND `+0x44 != 0` → priority = `f32( dist·0.5 + 10.0·rand01 − 0.02·placed )` where `dist = Sqrt_Approx((seedx−nx)² + (seedy−ny)²)` (`0x004CAC40`, one rand draw per accepted neighbor), mark `+0x3C=id`, heap-push. Neighbor with foreign nonzero id → **alive=0** (soft: loop continues). `placed += 1` per pop.
8. **Drain**: after the size cutoff, ALL remaining queue entries are popped one by one; each must still satisfy (scratch==0, clear, mask) → carved as water, else **alive=0**. `placed` keeps counting — final lake size = size + frontier length.
9. **Success gate**: alive AND `placed > 0x4B (75)` AND `placed > size/4` (CDQ/AND 3/SAR 2 round-toward-zero). Driver calls then run `0x0057A0C0(id, 0)` → `FUN_005A0160(id, 1, rect, 0, 0)` → green pass (tile 0/0xFFFF in region → `[0x00AA0E18]`); non-driver (river-end) calls skip all three, alive stays 1.
10. Success: `+0x304 += placed`; return 1. Failure: rollback identical to the river's (scratch id cells: `+0x38=0,+0x4B=0`, real `+0x38=0,+0x11A=0,+0x11B=(u8)+0x30C`); return 0.

### 4.3 Helper quartet

> **Provenance note (added 2026-07-25):** the original §4.3 header claimed "subagent-verified;
> inline citations in the per-function notes", but the per-function notes carried **no**
> inline citations. Of the five entries here, `FUN_005A0160`, `FUN_0059E740` and
> `0x0057A0C0` were re-verified from the binary during the 2026-07-25 audit and now carry
> real citations. `FUN_005A08D0` and `FUN_005A0410` were **not** re-derived — their
> contracts below remain single-source and are listed in §12.

**`FUN_005A0160(id, N, x0, y0, w, h, flag, level)`** — ring dilator, **param 2 is a ring count** (1/2/6 at the shaper call sites), no RNG. *(VERIFIED 2026-07-25: `decompile_function 0x005A0160` — 8-param prototype confirmed; outer `do { … } while (local_4 < param_2)` confirms the ring count; rect test is `param_3 ≤ x < param_3+param_5` and `param_4 ≤ y < param_4+param_6`, i.e. `(x0,y0,w,h)`; `absorb = param_7 != 0 && r == param_1 − 1`; claim predicate `(r == 0 || absorb) && (IsClearTile || (absorb && HasBridgeOverlay))`; the `+0x11B = level` write is additionally gated on `flag != 0 && IsClearTile`; `else if (r != param_1) → return 0`; no `Random__Next` in the body.)* Frontier = region border cells (`FUN_005A0700`: scratch scan, coord≠(0,0), `+0x38==id`, ≥1 in-diamond 8-neighbor with different id). N times: each frontier cell's 8 neighbors, if in-diamond and inside rect: `r=+0x38`; absorb = `flag≠0 && r==id−1`; if `(r==0 || absorb) && (IsClearTile || (absorb && bridge-overlay))` → claim scratch`+0x38=id` (and if flag≠0 on clear cells: real `+0x11B=level`); else if `r != id` → **return 0** (hard fail on foreign region / non-clear terrain). Return 1.

**`FUN_005A08D0(id, stepScale f32, &rect, &origin, flushFlag)`** — heading-biased random meander ("wandering water arm"), **scratch-only writes** (`+0x4B=1`, `+0x38=id`; no tiles). θ0 from which rect edge is clipped (0 / π / 3π/2 / π/2). Seeds a float min-heap from region border cells in rect with priority `1.5·anglefold(|θc−θ|) + 2.0·rand01`; step budget `base = trunc(0.5f·max(ln N,1.0)/stepScale)` then `steps = base + U[0, base/2]`; per step pops min, claims it, pushes orthogonal neighbors (dirs 0,2,4,6) with the same priority form, then `θ += gauss·π/4`. Foreign region → soft-fail flag. Returns flag. Call sites: river waterfall `0.01f`; bridge constructor `0.003f`.

**`FUN_005A0410(id, rings, newId)`** — border **peeler** (no RNG, always 1): `rings` times, region-border cells get scratch `+0x38=newId`, real tile `+0x38=0`, `+0x11A=0`, `+0x11B=(u8)+0x30C`. (The task hypothesis "writes the +0x44 mask" is REFUTED — the lake grower derives the mask inline, §4.2 step 4.)

**`FUN_0059E740(id, &A, &B, dir{0,2,4,6}, &outFlag, &fx, &fy)`** — river bridge constructor (`ECX` = MapSeed `this`; the decompiled prototype is 8 params with `this` first). `*outFlag=0`. Case dir=0: clearance scan rect `x∈[A.x−2, B.x+3), y∈[A.y−12, A.y)` must be all scratch-0 AND clear, else **return 1 with outFlag 0** (quiet abort). Fill 4 rows ahead with water (`tile = [0x00AA0738] + U[0,5]`, `subtile = U[0,3]`, scratch=id), then chain `FUN_005A08D0(id, 0.003f, farHalfPlane, farOrigin, 0)` → `0x0057A0C0(id, 0)` → `FUN_005A0160(id, 2, farHalfPlane, 0, 0)` → all cells of id get real `+0x11B += 4` → fill 8 more rows of water → stamp bridge ramp/middle pieces from tileset bases `[0x00AA10A0]`/`[0x00AA073C]`/`[0x00AA1050]`/`[0x00ABB110]` (per dir 0/2/4/6) via the placement-cursor + `MapClass__ApplyBridgeTile` path, alternating 2-cell/1-cell middles by remaining-length parity; ramp-adjacent cells get tile 0xFFFF, `+0x11B += 4`, `+0x11A=0`, scratch=id. On success **`fx/fy` advance 12 cells in the travel direction** (table {0,+12,0,−12}/{−12,0,+12,0} for N/E/S/W), outFlag=1.

> **VERIFIED and CORRECTED 2026-07-25** (`decompile_function 0x0059E740`):
> - Quiet abort, both fill loops (`U[0,5]` on the tile with `while (5 < v)` rejection and
>   `U[0,3]` on the subtile with `while (3 < v)` rejection, **two draws per water cell**),
>   the `0.003f` step scale (immediate `0x3B449BA6` = 0.003 exactly), the chain order, the
>   four per-dir tileset bases, the parity alternation (`uVar & 0x80000001`; even → the
>   2-cell piece at tileset offset `+8` and `i += 2`, odd → the 1-cell piece at `+4` and
>   `i += 1`), the ramp-adjacent `0xFFFF` / `+0x11B += 4` / `+0x11A = 0` / scratch writes,
>   and the `{0,+12,0,−12}` / `{−12,0,+12,0}` advance table indexed by `dir/2` — **all
>   confirmed exactly as written**.
> - The dir-0 clearance rect is confirmed: `x` starts at `A.x−2` and ends at
>   `A.x−2 + (B.x−A.x) + 5 = B.x+3` (exclusive); `y` runs `A.y−12` to `A.y` (exclusive).
>   This also confirms `A` = cross-section **start** and `B` = **end**.
> - **CORRECTION — "others are 90° rotations" is WRONG.** The `(span+2)`-cell
>   `+0x11B −= 4` run exists **only in case 0 (N) and case 6 (W)**. Cases 2 (E) and 4 (S)
>   have no such loop at all. The four cases are hand-written, not generated by rotation,
>   and a port must not derive 2/4 from 0. (This is also why the original doc's own
>   OQ-23-adjacent note "dirs 2/4/6 rotation cell-exact coords unverified" was
>   understated — it is not just the coordinates that differ.)
> - The return value is `local_ed` (== outFlag) on the normal path; only the two clearance
>   aborts return 1 with outFlag 0. The caller ignores the return either way.

**`0x0057A0C0` — LABEL DRIFT: "MapClass__MarkBridgesForRepair_High" is wrong.** Real role: **RMG water-region shore finalization**. `(regionId, flag)`: clears the placement ghost (`FUN_004A8BF0(0)`), resets scratch `+0x40=−1`, then 4 full-map sweeps: (1) `0x0057A430` water-mask notch/strait fixes (converts pinch cells to water, recursing 8-neighbors; fails if a foreign region id is hit with flag==0); (2) `0x0057A320` straight-bank patterns → tile 0xFFFF; (3)/(4) `0x0057ACF0` stamps shore pieces (`g_ShorePieces` tileset) with run-length parity corner/straight selection and `FUN_00598030`-drawn variants (`&1`, `%3`).

> **CHALLENGED AND RE-CONFIRMED 2026-07-25.** During this audit an independent pass
> claimed this function is genuine gameplay bridge-tile repair and that the label is
> *accurate*. That claim rested on the Ghidra names of the three callees
> (`MapClass__UpdateBridgeTile_Low`, `MapClass__ClearBridgeCell_Low`,
> `MapClass__SelectBridgeTileVariant_Low`) rather than on their bodies. Reading the
> bodies settles it in favour of the original text — **those callee labels are drift too**:
> - `0x0057A430` (`"UpdateBridgeTile_Low"`) writes `real cell +0x38 = g_WaterSet_TileSetBase`
>   and `+0x11A = 0` on matched masks, registers via `FUN_005A0090`, and recurses into all
>   8 neighbours. Its failure predicate is literally
>   `FUN_005A00C0(coord) > 0 && id != regionId && flag == 0` — i.e. the "foreign region id
>   with flag 0" test the original text describes. It writes **water**, not bridge tiles.
>   (`decompile_function 0x0057A430`)
> - `0x0057ACF0` (`"SelectBridgeTileVariant_Low"`) indexes
>   `g_IsometricTileTypeClass_Array[(g_ShorePieces + piece)*4]` and passes `g_ShorePieces`
>   / `g_ShorePieces + 0x29` into `MapClass__ApplyBridgeTile`; its variant picks are
>   `FUN_00598030()` results used as `&1` and `%3`, and its run counter is the
>   `local_c & 1` parity walk. It stamps **shore pieces**. (`decompile_function 0x0057ACF0`)
> - Callers are still exclusively the five RMG water routines — `0x0059A6C0`, `0x0059BBC0`,
>   `0x0059C920`, `0x0059D510`, `0x0059E740` (`get_function_callers 0x0057A0C0`,
>   re-run 2026-07-25).
>
> **Two corrections to the original text, though:**
> 1. **"4 fail-fast passes" is wrong for sweep 2.** `MapClass__ClearBridgeCell_Low`'s
>    return value is **discarded** (`MapClass__ClearBridgeCell_Low(cell,p1,p2);` with no
>    assignment), so sweep 2 runs to completion whenever sweep 1 succeeded. Sweeps 1, 3
>    and 4 *are* fail-fast. The function returns the last sweep flag.
> 2. The function also **allocates `DAT_00ABED10` itself if it is null** (W²·0x50 bytes,
>    element-initialised via `FUN_0058BDC0`) and frees it again on exit in that case. In
>    the RMG pipeline the scratch array already exists, so this is a no-op path — but it
>    means `0x0057A0C0` is callable outside RMG in principle. It also clears
>    `g_UIModeLock` and tears down `DAT_0088098C` before returning.
>
> `0x0057A320`'s specific pattern list `{0xC7,0x7C,0xF1,0x1F,0xC6,0x6C,0xB1,0x1B}` was
> **not** re-derived this session — see §12.
>
> **The sibling is a different function, not "the same family".** `MapClass__MarkBridgesForRepair_Low`
> is at **`0x00578E60`** (body `00578E60`–`0057900B`), and it is the one
> `RandomMapGenerator__Generate` calls with `(0, 0xFFFFFFFF)`. It is genuine bridge-ramp
> work — resets a scratch flag byte per cell, then two CellIterator sweeps through
> `MapClass__PlaceBridgeRamp_Low` and `MapClass__SelectDestroyedBridgeTile_Low`. It
> consumes **zero** RNG (verified 2026-07-25: `get_function_by_address 0x00578E60`;
> `search_instructions` CALL `0x0065c780` scoped to that function → 0 matches over 132
> instructions). OQ-21 is therefore closed to the extent that the address and RNG-neutrality
> are known; its cell-level writes are still undecoded (§12).

### 4.4 `FUN_0058EF10` — bridge pass

> **AUDIT 2026-07-25.** Now labelled `RandomMapGenerator__BridgeAndConnectorPass` in
> Ghidra. Re-verified against `decompile_function 0x0058EF10` +
> `disassemble_function 0x0058EF10`; every bullet of the pseudocode below holds, with
> three refinements:
> - **Region deletion is two calls, not one.** Per region it is `FUN_0058C070(region)`
>   (the destructor, which itself removes the entry from `DAT_00ABDF94` and decrements
>   `DAT_00ABDFA0`) followed by `FUN_007C8B3D(region)` (heap free). Iteration is
>   descending. (`decompile_function 0x0058C070`, `0x007C8B3D`)
> - **The rebuild loop has an extra filter the pseudocode omits.** Before calling
>   `0x0058C800`, a scratch slot must also pass `FUN_005AC370(&packedCoord)`, which
>   returns false exactly when the packed `(x,y)` is `(0,0)`; and the slot is converted
>   through `MapClass__Get_CellClass` (`0x005657A0`) so `0x0058C800` receives a
>   `CellClass*`, not the raw scratch pointer. (`decompile_function 0x005AC370`)
> - **Zero RNG in this function and in `0x00579010`.** An address-range-filtered scan of
>   all 244 program-wide `CALL 0x0065C780` sites finds none inside `0058EF10`–`0058F0B4`
>   or `00579010`–`00579312`. The pass's only draw is the one inside `0x0058C800`.
>   (`search_instructions` mnemonic `CALL` operand `0x0065c780`)

```
ok = 1
DisplayClass::Set_Cursor_Shape(NULL)      // 0x004A8BF0, ECX=0x87F7E8 — clears the placement-cursor
                                          // footprint (CellClass+0x12C bit 0x2); NO RNG (verified)
for each cell in zigzag-diagonal MapClass iterator order:
    ok = PlaceBridgeRamp_Low(cell, -1)    // 0x00579010; loop exits after first 0
                                          // BUT: with group_id=-1 the only failure branch is
                                          // disabled (guard `id>0 && id!=group && group!=-1`
                                          // at 0x005792AD) — failure is unreachable in THIS pass
reset all scratch +0x38 = -1, +0x3C = -1
delete all region objects (0x0058C070 removes from vector, DAT_00ABDFA0--); DAT_00ABED14 = 0
for each scratch slot with +0x38 == -1 and coord != (0,0), linear order:
    region = CellClass::BuildRegion()     // 0x0058C800 — flood fill, see below
    if region: region->+0x1A = 0
if (ok):
    for i ascending: FUN_0058F0C0(region[i])   // §4.5
    for i ascending: FUN_005905D0(region[i])   // §4.6
    for i ascending: delete region[i]->+0x04 neighbor vector
return 1 always
```

**`0x00579010` (`MapClass__PlaceBridgeRamp_Low`) one-level contract:** `bool __thiscall (map@0x87F7E8; CellClass*, i32 ramp_group_id)`, RET 8. Computes an 8-bit neighbor adjacency mask (`0x00579B70`); on a fixed mask-predicate match: looks up the cell's ramp group (`FUN_005A00C0` on MapSeed), fails (AL=0) only iff `id>0 && id!=ramp_group_id && ramp_group_id!=-1`; otherwise **`cell+0x11B += 4`** (only top-level cell write), records via `FUN_005A0090(MapSeed, &xy, group)`, and recurses into all 8 neighbors (results ignored). Returns 1 in all other cases. No RNG. Internals of `0x005A0090`/`0x005A00C0`/`0x00579B70` deferred.

**`0x0058C800` region rebuild (per seed cell):** DFS flood over 8 neighbors (diamond test; `+0x38==−1`; `+0x3C != id` dedupe; same level `+0x11B`; same water/green classification: `0x004865D0` water-family ∪ `0x004867B0` green-family). Regions with pop count `> 0x4A (74)` are kept (new region object: id=`DAT_00ABED14++`, level, water/green flag at `+0x14`, area recount at `+0x0C`). Smaller LAND floods are merged into a donor: id==0 → **the pass's only RNG draw**: border cell of region 0 picked via `idx = ftol(rand·count·2^-32(1+ε))` rejection loop (ECX=0xABE890 verified at `0x0058CAF9`), first 8-neighbor with different level and not-water supplies the donor level; id≠0 → deterministic donor `(x−1,y)` else `(x,y−1)`. Merged cells: scratch id→donor id (or −1), `+0x4B=0`, real tile `+0x38=0`, `+0x11A=0`, `+0x11B=donor level` (fallback `(u8)MapSeed+0x30C` = `DAT_00ABE2E4`) — **the merge rewrites real terrain, not just scratch.**

> **VERIFIED + REFINED 2026-07-25** (`decompile_function 0x0058C800`,
> `disassemble_function 0x0058C800`, `search_instructions`):
> - Threshold: `CMP EAX,0x4B; JGE` at `0x0058CAA4` — floods of **≥75** cells are kept, i.e.
>   `> 0x4A (74)` exactly as documented.
> - The RNG site is exactly one: `MOV ECX,0xABE890` at **`0x0058CAF9`**, `CALL 0x0065C780`
>   at `0x0058CAFE`, inside the documented `idx = ftol(rand·count·[0x007ED898])` rejection
>   loop (`CMP EAX,ESI; JA 0x0058CAF9`). Confirmed as the function's **only** RNG call by
>   an address-range-filtered scan of all 244 program-wide call sites.
> - **Refinement:** the "scratch id → donor id" write applies to the RNG-free deterministic
>   merge path (`0x0058CD90`–`0x0058CDDD`). On the `donor id == 0` bootstrap path — the one
>   branch that actually makes the RNG draw — matched cells get **scratch `+0x38 = −1`**
>   (`OR EBX,0xFFFFFFFF; MOV [ECX+0x38],EBX` at `0x0058CC0B`/`0x0058CC39`), not a donor id,
>   because no donor region exists yet. Only the level byte is borrowed from the randomly
>   picked border cell, with the same `DAT_00ABE2E4` fallback. The original text's
>   parenthetical "(or −1)" is the correct case but the doc did not say *which* branch it
>   belongs to — it is precisely the RNG branch.
> - Real terrain rewriting confirmed: the writes target the `MapClass__Get_CellClass`
>   (`0x005657A0`) result, a different address range from the `DAT_00ABED10` scratch array.

### 4.5 `FUN_0058F0C0` — region neighbor/area annotator (subagent-verified)

**Zero RNG, zero FPU** (full disassembly scanned — no `CALL 0x0065C780`/`0x007C5F00`). Writes only `region+0x04` (new DynamicVector<i32> of adjacent region ids, **ascending by construction**: adjacency marks go into a `DAT_00ABED14`-sized flag array from the region's border cells' 8-neighbors (in-diamond, id ≥ 0, self excluded), then ids are emitted in a 0..N scan) and `region+0x0C` (area = count of scratch cells with `+0x38==id` and coord ≠ (0,0)). Border definition (`FUN_0058D410`): a region cell with ≥1 **in-diamond** 8-neighbor of different id (out-of-diamond neighbors never qualify; scratch-null counts as different).

### 4.6 `FUN_005905D0` — connector pass (subagent-verified)

Two modes on `region+0x14`:

**Land (`+0x14==0`) — ramps between different-level neighbors:** for each neighbor id in `+0x04` with `this.id < nbr.id` (each pair once) and `nbr.lvl != this.lvl`: roll `U[0,100]` (101·2^-32(1+ε) scale) — compared against `DAT_00ABE044`, which is **statically 0 with zero writers** (get_xrefs_to), so the bonus `U[1,2]` extra-ramp roll never triggers in retail but the first roll is **always consumed**; numRamps = 1 (retail). hi = higher-level region; border list of hi (`FUN_0058D410`); up to **100 attempts**: pick `U[0,count−1]` border cell, `Is_Cell_In_Playfield (0x00578460)`, then `FUN_00590970(&cell, loId, tol=attempt·0.01f)` — **CORRECTED 2026-07-25: this takes 3 parameters, not 4.** `hi` is *not* passed in; it is used only earlier, to choose which region's border-cell list to sample via `FUN_0058D410(hi)`. (verified 2026-07-25: `decompile_function 0x00590970` shows `char FUN_00590970(short *cellXY, undefined4 loId, float tol)`; `disassemble_function 0x005905D0` at `0x005908B9`–`0x005908EF` shows the call site pushing three arguments.) The callee is a 5×5-block density classifier (`FUN_00590FD0`, threshold `T = ftol((15.0−5.0)·(1.0f−tol)+5.0)`, bit table `DAT_0082AF18` = {0x40,0x80,0x01; 0x20,·,0x02; 0x10,0x08,0x04}); diagonal/axis ramp builders (`0x00593AF0` family) stamp tiles `g_RampBase+{0,3,7}`, `+0x11A=0`, `+0x11C∈{1,4,8}`, `+0x11B=lvl−i−1`, each axis case drawing 2× `U[0,1]` endpoint jitter; brute-force fallback when tol > 0.5f (attempt ≥ 52). Zero successes → `this+0x1B = 0`.

**Water (`+0x14!=0`) — low bridges across this region:** for neighbor pairs (i<j) where both qualify (`nbrCount>1 || area>50`), B's `+0x14==0`, and `A.lvl==B.lvl==this.lvl`: `FUN_0058F2C0(this, A, B)` — up to **200 attempts**: uniform random scratch cell of this region; 4-ray walk to both banks with corridor validity checks; endpoint region ids must be {A.id, B.id} in either order; both axes valid → strictly shorter wins (EW on tie); length gate `len < attempt/25 + 8`; stamps a 3-wide deck writing `CellClass+0x44` = {0x5E, 0x5C, 0x4A+(x mod 4)} EW / {0x60, 0x62, 0x53+(y mod 4)} NS (LOBRDG-range overlay values, field-name inference) and `+0x11E` = cross-offset 0..2; end tiles from `[0x00ABBEC8]`+{0xA,9,0xD,0xC} or `[0x00ABBEC4]`+{0,2,1,3} (50% coin + area test). Note as-written asymmetry: A's `+0x14` is never checked (only B's).

**CORRECTED 2026-07-25 — there are 15 `Random__Next` sites in this pass, not 16.** All 15
do load `ECX=0xABE890`. Breakdown: 3 in `FUN_005905D0` itself, 8 in `FUN_00590970`, and 1
each in the leaf helpers `FUN_00592440`, `FUN_00591740`, `FUN_00591D80`, `FUN_005910F0`
(sites spot-checked at `0x0059283D`, `0x00591AB7`, `0x0059218B`, `0x005914A9`).
`FUN_00590FD0` and the ramp builders `FUN_00593AF0` / `FUN_00593550` / `FUN_00593030`
contain **zero** RNG calls. (verified 2026-07-25: `disassemble_function 0x005905D0`,
`0x00590970`, `0x00590FD0`; `search_instructions` CALL `0x0065c780`)

The `DAT_00ABE044` claim is confirmed: `get_xrefs_to 0x00ABE044` returns a **single READ**
xref (at `0x005907B8`) and **no writer anywhere in the binary**; `read_memory 0x00ABE044`
= `00000000`. The `JGE` at `0x005907BE` is therefore always taken (a U[0,100] value is
always ≥ 0), so `numRamps` is always 1 in retail — but the U[0,100] draw ahead of it **is
always consumed** and must be reproduced. (verified 2026-07-25)

**The water-mode helper `FUN_0058F2C0` is now fully decoded** — see the box below; the
original doc's "admitted hole" here is closed except for three named callees (§12).

> **`FUN_0058F2C0` — low-bridge deck placer (Ghidra: `RandomMapGenerator__PlaceLowBridgeDeck`).
> Every number in the original text VERIFIED 2026-07-25 from raw disassembly:**
> - **200 attempts** — `CMP EAX,0xC8; JL` loop at `0x0059029F`–`0x005902A8`.
> - **Uniform random scratch cell of this region** — linear index over
>   `g_PathfinderLinearMapWidth²` drawn as `rand · [0x007ED898]`, rejection-looped until
>   `scratch[+0x38] == this.id` and `FUN_0050E470` accepts (`0x0058F342`–`0x0058F3AF`).
> - **4-ray walk to both banks** — four `FUN_005A7250`-driven walks (N+S for the NS axis,
>   W+E for the EW axis), each cell gated by `Is_Cell_In_Playfield` + `FUN_004863D0`.
> - **Endpoint region ids must be `{A.id, B.id}` in either order** — both orderings are
>   checked in each axis-validity block.
> - **Strictly shorter wins, EW on tie** — `if (NS_len < EW_len) drop EW else drop NS`;
>   the equal case drops NS, so EW wins ties. Confirmed.
> - **Length gate `len < attempt/25 + 8`** — `0x51EB851F` magic-multiply divide-by-25,
>   `SAR 3`, sign correction, `+8` (`0x0058F9CD`–`0x0058F9E0`).
> - **`CellClass+0x44` deck values** — EW: first column `0x5E` (`0x0058FACE`), last column
>   `0x5C` (`0x0058FB00`), middle `0x4A + (x mod 4)` (`0x0058FB3F`, `AND 0x80000003` signed
>   -mod idiom). NS: first row `0x60` (`0x0058FEE8`), last row `0x62` (`0x0058FF23`),
>   middle `0x53 + (y mod 4)` (`0x0058FF60`). Exact.
> - **`+0x11E` cross-offset 0..2** — `row − ystart` for EW, `col − xstart` for NS.
> - **End tiles** — `[0x00ABBEC8] + {0xA, 9, 0xD, 0xC}` on the coin==1 path and
>   `[0x00ABBEC4] + {0, 2, 1, 3}` on the default path, at the four end-stamp sites
>   `0x0058FC85` (EW-west, `+0xA`/`+0`), `0x0058FD57` (EW-east, `+0x9`/`+2`),
>   `0x005900B3` (NS-first, `+0xD`/`+1`), `0x00590191` (NS-second, `+0xC`/`+3`). The coin
>   is rolled **only** when the area test `0x005A7440` passes; a failed area test and a
>   coin of 0 both fall to the default array.
> - **RNG draws, in order, all `ECX=0xABE890`:** (1) `0x0058F347` scratch-cell index pick
>   (rejection loop, once per attempt); then, for the winning axis only, (2) `0x0058FBE2`
>   EW-west end coin / (3) `0x0058FCCB` EW-east end coin, **or** (4) `0x00590013` NS-first
>   end coin / (5) `0x005900FC` NS-second end coin. Each coin is a U[0,1] retry loop and
>   each is gated by its own area test.
>
> **Caution on an adjacent Ghidra label:** the plate comment currently on this function
> states the NS probe rectangles are "7×6 and 6×7". That is **wrong** — disassembly shows
> both NS end-area-test rects use `w=7, h=6` (`EBX=7`/`EBP=6` set once at `0x0058FFCE`/
> `0x0058FFE0` and reused unchanged for both `CALL 0x005A7440` sites at `0x00590003` and
> `0x005900EE`). Do not implement the asymmetry.

### 4.7 `FUN_00595400` — tech-building placement (subagent-verified)

Caller `0x005A95B0` (ECX=MapSeed), called from `0x00598960` only when `maptype != 0`:
- maptype==2: `passes = ftol(rand·3·2^-32(1+ε))` retry >2 → U{0,1,2} (0 = nothing), drawn ONCE; `passes` × (per region with `+0x20 > 0`: `FUN_00595400(region)`).
- maptype≠2 (incl. 3/4): count = `ftol(rand·5·2^-32(1+ε))` retry >4 → U{0..4} (scale const at **`0x007EDAB0`**); buildings dropped at uniform random cells `FUN_00598030(0, W²−1)` (`0x005A9723`: `ECX=0`, `EDX=W²−1`) with the same foundation checks/Unlimbo.

> **CORRECTED 2026-07-25 — which code runs on map types 3/4.** `FUN_00595400` is called
> **only** from the `maptype == 2` branch. The `maptype != 2` branch — the one that
> actually runs for types 3 and 4 — uses `FUN_005A95B0`'s **own inlined duplicate** of the
> placement logic, not a call into `FUN_00595400`. The two copies agree on the "Neutral"
> owner lookup, the `RulesClass+0xAE0/+0xAEC` type roll, the 100-attempt bound and the
> foundation checks (each verified separately in both bodies), but a port targeting types
> 3/4 should be written against the `FUN_005A95B0` copy, and the two must not be assumed
> identical without checking. (verified 2026-07-25: `disassemble_function 0x005A95B0` —
> maptype==2 arm at `0x005A95B6`–`0x005A9626` calls `0x00595400`; maptype!=2 arm at
> `0x005A9630`+ inlines; 100-attempt bounds at `0x005954B6` and `0x005A9713` respectively)

`FUN_00595400(region)`: owner = house of country "Neutral" (`0x005117D0` name→index, `0x00502D30` index→house). ONE building type rolled per call: `k = U[0, count−1]` over **`RulesClass+0xAE0` (items) / `+0xAEC` (count) = `[AI] NeutralTechBuildings`** — *both offsets VERIFIED 2026-07-25 in both bodies (`0x00595448`–`0x0059549B` and `0x005A969E`–`0x005A96F4`). The original text's alternate "vector at RulesClass+0xADC" is **WRONG/unsupported** — nothing reads `+0xADC`; the items/count pair sits 0xC apart, the same spacing as the region cell-vector `+0x2C`/`+0x38`.* Stock `ini/rulesmd.ini:3082` = CAAIRP,CATHOSP,CAOILD,CAOUTP,CAMACH,CAPOWR. NOT the MapSeed+0x2C4/+0x2E0 ore-patch lamps. Owner lookup VERIFIED: the country string at `0x0082BA08` reads `"Neutral\0"` (`read_memory`), fed to `0x005117D0` then `0x00502D30` in both bodies. `BuildingClass` constructed (`0x0043B740`, size 0x720); up to **100 attempts**: anchor = `U[0, region+0x38 − 1]` index into the region's cell list at `+0x2C` (no anchor pre-validation); `lvl = (s8)anchor_cell+0x11B`; walk the foundation list (`BuildingType vtbl+0x90` — the `obj+0x520` vtable call is verified in both bodies — yielding a `0x7FFF`-terminated `(i16,i16)` pair array; **the original text's "→ `0x0045EC20` → `[type+0xDFC]`" chain is WRONG/unverifiable: `0x0045EC20` is not a function in the current program at all (`get_function_by_address 0x0045EC20` and `decompile_function 0x0045EC20` both return "No function found"), and `[type+0xDFC]` is not read directly in either body — the pair array comes back as the vtable call's return value**): every cell must have `+0xE4==0`, `IsClearTile`, `(s8)+0x11B == lvl`, `+0xEC != 3`, in-playable (`0x00578540` — verified as a distinct address from the `0x00578460` used in §4.6; label "TechnoClass__IsOnScreen" is drift), and scratch `+0x45 == 0`. Foundation cells are NOT checked for region membership. Success → `Unlimbo` (vtbl+0xD8 → `0x00440580`) at cell-center leptons `(x·256+128, y·256+128, 0)`, dir 0, return immediately (1 building per call). Never placed → scalar deleting dtor (vtbl+0x20 → `0x00459F20`).

## 5. RNG Draw Ledger

**SCOPE CORRECTED 2026-07-25.** This ledger was headed "Per generation (mode 3/4)" but it
is **scope-local** — it covers only the functions this document decodes. It is **not** a
complete per-generation stream and must not be used as one. Every draw site listed is
`g_MapGenRng` (`0x00ABE890`), each verified in asm. Phases that consume draws on a mode-3/4
generation but are **absent from this ledger** are listed after it.

### 5.1 Sites covered by this document

(a) **init-phase bridge coin** — 1 draw (§3.1).

(b) **river attempts** — per attempt: 3 start draws (+rejections) + width draw +
bridgeMinStep draw + per step: `[bridge: none]` + `[branch check: 1]` +
`[heading wobble, step > 5: one Gaussian]` + `[width wobble, halfW > 0: one Gaussian]` +
termination draw. Carve substeps draw nothing. The bridge constructor `FUN_0059E740` draws
**two** values (U[0,5] tile + U[0,3] subtile, each a rejection loop) **per water cell of
both fills**, plus the `FUN_005A08D0` meander seed/step draws and the `0x0057A0C0`
shore-pass variant draws.

> **CORRECTED — Gaussian cost.** The original ledger said `FUN_005980C0` "consumes 2 raw
> draws per refill (Box-Muller pair caching)". **Wrong.** It is Marsaglia *polar* with a
> rejection loop: **2 raw draws per attempt, repeated until `0 < u²+v² < 1`** (expected
> ≈2.546, unbounded), and the *second* Gaussian of each pair is served from cache for
> **0** draws. See the corrected box in §4.1 for the full derivation and citations. Any
> port that hardcodes 2 will desync. The cache state at `0x00ABDFB8` is re-zeroed each
> generation by `0x00598960`.

(c) **lake attempts** — per attempt: seed rejection draws (≤200 × 2 + rejections), one
Gaussian size draw, 1 draw per accepted frontier neighbour.

(d) **bridge pass** (`0x0058EF10`) — exactly 1 draw per small-region-0 *bootstrap* merge
event inside `0x0058C800` (site `0x0058CAFE`); **0** in `0x0058EF10` itself, **0** in
`0x00579010`, **0** in `0x0058F0C0`. (all three re-verified 2026-07-25 by range-filtered
scan of every `CALL 0x0065C780` in the program)

(e) **connector pass** (`0x005905D0`) — **15** static sites, not 16 (§4.6). Per land pair:
1 mandatory U[0,100] (the `+U[1,2]` bonus is dead in retail but the first roll is always
consumed) + per attempt 1 border-index draw + builder jitter 2 × U[0,1]. Per water pair
(`FUN_0058F2C0`): per attempt 1 cell-index draw, then 1 end coin per bridge end on the
winning axis, each gated by its own area test.

(f) **tech** (`0x005A95B0`) — 1 pass/count draw + per placement 1 type draw + ≤100 anchor
draws. Note §4.7: types 3/4 run `0x005A95B0`'s inlined copy, not `FUN_00595400`.

### 5.2 Mode-3/4 draws this document does NOT account for

These fire on a normal type-3/4 generation and sit **between** and **after** the phases
above in `RandomMapGenerator__Generate`'s order (§7). A stream-exact port needs them all;
none is decoded here.

| Phase | Draw sites | Evidence |
|---|---|---|
| "Init regions" partition — `0x0058CF90` and `0x0058D010`, both of which call `0x0058C800` | 1 draw per bootstrap merge, per invocation | `decompile_function 0x0058CF90`, `0x0058D010` |
| `0x0058EBC0` → `0x0058D620` region split (types 3/4 only) | **4 distinct `Random__Next` sites** at `0x0058D787`, `0x0058D7D2`, `0x0058D8C6`, `0x0058E382`, and `0x0058D620` is re-entered from index 0 after every successful split, so the invocation count is data-dependent | `search_instructions` CALL `0x0065c780` scoped to `FUN_0058D620` → 4 matches; `decompile_function 0x0058EBC0` |
| `0x005A19E0` cliff drops | **2 sites**, `0x005A1A80` and `0x005A1B31`, inside a full-map CellIterator sweep (0 or 1 draws per cell, mask-dependent) | `search_instructions` CALL `0x0065c780` scoped to `FUN_005A19E0` → 2 matches |
| `MapClass__MarkBridgesForRepair_Low(0, −1)` @ `0x00578E60` | **zero** — safe to skip in the ledger | `search_instructions` scoped → 0 matches over 132 instructions |
| `0x005A17F0` water re-anchor → `0x005A1350` | `0x005A17F0` itself makes no direct draw but calls `0x005A1350`, which is a per-tileset-case random-variant picker with rejection loops in most cases | `decompile_function 0x005A17F0`, `0x005A1350` |
| `0x0059C630` finalizer, `0x0059B740`, start points, tiberium, hills, LATs | not examined | out of this document's scope |

## 6. INI Keys

No `rules(md).ini` keys drive the water shapers. Inputs are MapSeed fields (maptype `+0x3C`, WaterAmount `+0x4C`) and `RMGMD.INI` `[General]` (loader `0x005981F0`, called immediately before "RMG: Seeding water": RMGMinimumTiberium, RMGMaximumTiberium, RMGLevelLightSettings, RMGVegetationMinimums/Maximums, ambient light keys, MaxTrees, TemperateOrePatchLamps, SnowOrePatchLamps — **no bridge/water key; +0x310 is not INI-driven**). Tech placement reads `[AI] NeutralTechBuildings` (rulesmd.ini:3082, 6 entries).

> **CORRECTED 2026-07-25 — WaterAmount is NOT a dialog field.** The original text called
> `+0x3C` and `+0x4C` alike "dialog fields". Only `+0x3C` is:
> - **`+0x3C` map type** — genuinely dialog-driven, synced from combo control `0x405` in
>   `FUN_00596C70`, also settable by the Randomize handler at `0x005967B7`
>   (`Random__RandomRanged(1,4)` — note the Randomize button can never produce 0; 0 is
>   reachable only by manual combo selection). `MapSeedClass__ClampFields` (`0x005975E0`)
>   clamps to 0..4.
> - **`+0x38` theater** — dialog combo `0x407`, and randomized at `0x0059679E` as
>   `(0x31 < Random__RandomRanged(0,100))`, i.e. a 0/1 boolean. `+0x38 == 0` indexes the
>   theater-name string "TEMPERATE" (stride 0x70 table inside `0x00599650`), confirming
>   0 = temperate.
> - **`+0x4C` WaterAmount** — **has no dialog control at all.** `get_xrefs_to 0x00ABE024`
>   returns **zero** hits, and `FUN_00596C70` (the dialog↔field sync) touches
>   `+0x3C/+0x38/+0x40/+0x48/+0x50/+0x64/+0x68` but never `+0x4C`. It is a *derived*
>   field: `MapSeedClass__RandomizeDerivedFields` (`0x00597260`) sets it as its first draw,
>   `RandomRanged(g_nRmgWaterAmountMinByMapType[mt], g_nRmgWaterAmountMaxByMapType[mt])`.
>   Range 0..100 per `MapSeedClass__ClampFields`. **This matters for the port:** WaterAmount
>   is not player-editable, it is rolled per map type from a pair of lookup tables, and
>   those tables are what actually decide whether types 3/4 get water at all (the
>   `+0x4C != 0` driver gate). The tables' contents are **not** decoded here — see §12.
>
> (verified 2026-07-25: `get_xrefs_to 0x00ABE010`, `0x00ABE014`, `0x00ABE024`;
> `decompile_function 0x00596742`, `0x00596C70`, `0x005975E0`, `0x00597260`;
> `disassemble_function 0x00599650`)
>
> `0x005981F0` is confirmed as the `RMGMD.INI` loader — `PUSH 0x0082BDCC` at `0x005981FC`
> feeds the file open at `0x004739F0`, and `0x0082BDCC` reads as ASCII `"RMGMD.INI"`. Its
> Ghidra label has since been corrected to `MapSeedClass__ReadINI` by a parallel session,
> so the §11 "stale label" note about `0x005981F0` is now itself stale. Key→field mapping
> confirmed for RMGMinimumTiberium→`+0x2BC`, RMGMaximumTiberium→`+0x2C0`,
> MaxTrees→`+0x2FC`, TemperateOrePatchLamps / SnowOrePatchLamps (building-name lists read
> via `CCINIClass__ReadString` + `BuildingTypeClass__FindOrAllocate`), plus the light and
> vegetation triples. (verified 2026-07-25: `disassemble_function 0x005981F0`;
> `read_memory` on each key string)

## 7. Integration Points

`0x00598960` phase order: seed g_MapGenRng from `+0x74` → init/scratch-build `0x00599650` (zeroes +0x304/+0x308, draws the +0x310 coin) → RMGMD.INI load `0x005981F0` → water seeding (§3) → `0x0059C630` finalizer → "Init regions": scratch ids reset to −1, `0x0058CF90` partition, region ring-growth (`0x0058E740`/`0x0058E9B0`), `0x0058D010` → maptype 3/4 only: `0x0058EBC0` → **`0x0058EF10`** (§4.4) → `0x005A19E0` cliff drops → `MapClass__MarkBridgesForRepair_Low(0,−1)` → `0x005A17F0` re-anchor → `0x0059B740` → RecalcAttributes sweep → start points (`0x00594B50`/`0x005A1FB0` retry loops) → **tech buildings `0x005A95B0`** (maptype ≠ 0) → tiberium `0x005A23A0` → region/scratch teardown → hills → LATs → cleanup tail (frees scratch, zeroes +0x304/+0x308).

The type-3/4 block order is confirmed exactly: `FUN_0058EBC0()` → `RandomMapGenerator__BridgeAndConnectorPass()` (= `0x0058EF10`) → `FUN_005A19E0()` → `MapClass__MarkBridgesForRepair_Low(0, 0xFFFFFFFF)` → `FUN_005A17F0()`. The region partition sizing call is `FUN_0058E740((8000 < *(int*)(region+0xC)) + 4 + !is_type_3_or_4)`.

> **LABEL CORRECTIONS 2026-07-25 to this pipeline line and to the Non-Scope header:**
> - **`0x0058E740` is not "terracing".** Its single int parameter is a **ring count** — the
>   body is `frontier = FUN_0058D410(); do { grow the frontier one 8-directional step } while
>   (++i < param_1)`. So the driver expression above yields 4 or 5 rings for types 3/4 and
>   5 or 6 otherwise, +1 when the region already exceeds 8000 cells. It consumes no RNG.
>   (`decompile_function 0x0058E740`)
> - **`0x0058EBC0` is not "terracing" either.** It is a region **membership/area/bbox
>   rebuild plus an oversized-region cull driver**: pass 1 clears each region's cell count
>   (`+0x0C`), bbox (`+0x40/+0x44` = 9999,9999; `+0x48/+0x4C` = 0,0) and cell list; pass 2
>   walks every cell once in reverse and re-accumulates count, cell list (`+0x2C`/`+0x38`)
>   and bbox; pass 3 computes a size threshold via `Math__ftol` (no RNG) and, for each
>   region with `+0x1A == 0` and `+0x14 == 0` whose count exceeds it, calls `0x0058D620` —
>   restarting the scan from index 0 after each successful split, else marking the region
>   finalized (`+0x1A = 1`). **`0x0058EBC0`'s own body contains no RNG and no `+0x11B`
>   write.** (`decompile_function 0x0058EBC0`)
> - **`0x0058D620` is the actual split/elevation-step routine** — sole caller `0x0058EBC0`,
>   and it holds the 4 `Random__Next` sites listed in §5.2. Attributing "terracing" to
>   `0x0058EBC0` sent the reader to the wrong function.
> - `0x0058CF90` and `0x0058D010` consume no RNG directly but both call `0x0058C800`,
>   which does. `0x0058E9B0` consumes none.

## 8. Current Rust Implementation Status

None. `src/` contains only the `RandMap.Sed` sentinel plumbing (`src/skirmish_scenarios.rs:14`, `src/app_skirmish_shell_render.rs:80`, `src/app_list_maps.rs:498` — grep this session). No terrain generation, no MapSeed model, no scratch-cell array.

## 9. Coverage Ledger

| Area / function | Status | Evidence | What remains |
|---|---|---|---|
| Driver `0x0059C580` + call-site gate | verified | decompile 0x0059c580; disassemble_bytes 0x00598ac0 | none |
| `0x0059D510` river carver | verified | decompile + full disasm (2 chunks) + 20 constants read | none |
| `0x0059C920` lake grower | verified | decompile + full disasm + constants | none |
| `+0x310` coin / `+0x30C` default / ctor 0x00595740 | verified | search_instructions "+0x310]"; disassemble_bytes 0x0059a4a0, 0x0058b73a; decompile 0x00595740 | none |
| `0x004A8BF0` Set_Cursor_Shape | verified | subagent: decompile 0x004A8BF0/0x004A95A0/0x004A94F0 | none |
| `0x0058EF10` bridge pass | verified | subagent: decompile+disasm 0x0058EF10 | none |
| `0x00579010` ramp placer | touched-not-exhausted (contract only) | subagent: disasm 0x00579010 | internals of 0x005A0090/0x005A00C0/0x00579B70; mask predicate semantics |
| `0x0058C800` region rebuild | verified | subagent: decompile+disasm 0x0058C800 | none |
| `0x0058F0C0` neighbor/area | verified | subagent: full disasm (RNG-negative proof) | none |
| `0x005905D0` connector pass | verified (structure+formulas) | subagent: decompile+disasm + 16 RNG sites | 6 of 7 ramp builders assumed symmetric; 11 level-3 helpers |
| `0x00595400` + `0x005A95B0` tech | verified | subagent: disasm both + data-flow to [AI] | CellClass +0xE4/+0xEC field identities |
| `0x005A0160`/`0x005A08D0`/`0x005A0410`/`0x0059E740` | verified | subagent: decompile+disasm + constants | dirs 2/4/6 rotation cell-exact coords in 0x0059E740 |
| `0x0057A0C0` shore finalize | verified (role+structure), **re-confirmed 2026-07-25 against a challenge** | decompile_function 0x0057A0C0 / 0x0057A430 / 0x0057ACF0; get_function_by_address 0x0057A320; get_function_callers 0x0057A0C0 | `0x0057A320`'s pattern list; ApplyBridgeTile internals; shore-piece table provenance |
| `g_DirectionOffsets` init | verified | decompile_function 0x0049F2F0 (this session) | none |
| `Math__ftol` rounding | verified truncate | disassemble 0x007c5f00 + read_memory 0x00822d80 (CW 0x0E7F) | none |
| `MarkBridgesForRepair_Low` (pipeline sibling) | **address + RNG-neutrality verified 2026-07-25** | get_function_by_address 0x00578E60; search_instructions CALL 0x0065c780 scoped → 0 | it is NOT the shore family — it is genuine bridge-ramp work; its cell writes are undecoded |
| `0x00599650` init/scratch-build (rest of body) | touched-not-exhausted; **+0x310 block fully verified 2026-07-25** | disassemble_function 0x00599650; get_function_by_address 0x0059A4B6 → entry 0x00599650 | base level 4 initialization of +0x11B still assumed from the +0x30C default, not read from this body |
| `FUN_005980C0` Gaussian helper | **verified 2026-07-25 — prior claim was WRONG** | decompile_function 0x005980C0; search_instructions 0x00abdfc8; disassemble_bytes 0x00598000 | none |
| `FUN_0058F2C0` low-bridge deck | **verified 2026-07-25 (was the doc's admitted hole)** | disassembly of 0x0058F2C0, all constants and 5 RNG sites traced | 0x005A6C10 / 0x005904B0 / 0x005A7440 bodies (§12) |
| MapSeed `+0x38`/`+0x3C`/`+0x4C` writers | **verified 2026-07-25** | get_xrefs_to 0x00ABE010/14/24; decompile_function 0x00596742 / 0x00596C70 / 0x005975E0 / 0x00597260 | contents of the WaterAmount min/max-by-maptype tables (§12) |
| `0x0058EBC0` / `0x0058D620` roles | **verified 2026-07-25 — doc mislabelled them** | decompile_function 0x0058EBC0; search_instructions scoped to FUN_0058D620 → 4 RNG sites | `0x0058D620` internals |
| `0x005A19E0` / `0x005A17F0` / `0x005A1350` RNG presence | **verified 2026-07-25** | search_instructions scoped → 2 sites in 0x005A19E0; decompile_function 0x005A17F0 / 0x005A1350 | full contracts (out of scope here) |

## 10. Open Questions — Final State

- `[RESOLVED]` OQ-01 water quota formula → `ftol(genW·genH·WA·0.008 + 100.0)` (disasm 0x0059c92e; read_memory 0x007ed9f8/0x007e2ac0)
- `[RESOLVED]` OQ-02 what ends the water phase → `remaining ≤ 75` makes 0x0059C920 return 0; driver gives up after 10 (disasm 0x0059c969)
- `[RESOLVED]` OQ-03 river width/length model → width U{1..ftol(max(0.07·WA,1))} with ±w/2 random walk; length geometric p=0.005/step, min 40 steps (disasm + read_memory)
- `[RESOLVED]` OQ-04 per-step carve span source → `ftol(width_walk + 0.5)` (push-adjusted slot [ESP+0x98]; disasm 0x0059da32 region)
- `[RESOLVED]` OQ-05 bridge gating → +0x310 coin (25%/map, init phase), step > U[35,125], |Δheading|<1 rad, cross-section straight, ≤1/river (disasm 0x0059dca1..dd5e; 0x0059a4a0)
- `[RESOLVED]` OQ-06 who sets +0x310/+0x30C → 0x00599650 coin / ctor 0x00595740 default 4 (search_instructions; decompile)
- `[RESOLVED]` OQ-07 end-lake trigger → only rand-terminated rivers (FLD-1.0 sentinel; disasm 0x0059d9a3/0x0059e1b4)
- `[RESOLVED]` OQ-08 branch semantics → 1%/step, mean h+π/2 in [h+π/6, h+5π/6], same region id, failure kills parent (disasm 0x0059de44..dfa2)
- `[RESOLVED]` OQ-09 0x004A8BF0(0) role → clears placement-cursor footprint bits (subagent)
- `[RESOLVED]` OQ-10 0x0058F0C0 → neighbor/area annotator, no RNG (subagent)
- `[RESOLVED]` OQ-11 0x005905D0 → ramp/low-bridge connector; DAT_00ABE044=0 kills the extra-ramp roll but its U[0,100] is still consumed (subagent)
- `[RESOLVED]` OQ-12 0x00595400 building source → RulesClass [AI] NeutralTechBuildings, Neutral house (subagent data-flow)
- `[RESOLVED]` OQ-13 PlaceBridgeRamp_Low failure with −1 → unreachable in this pass (subagent, guard at 0x005792AD)
- `[RESOLVED]` OQ-14 0x0057A0C0 label → WRONG; RMG shore finalization (subagent)
- `[RESOLVED]` OQ-15 lake allow-mask rule → inline in 0x0059C920 after 0x005A0410(0,2,−2) peel (subagent + disasm 0x0059ca92)
- `[RESOLVED]` OQ-16 RNG instance per call site → every verified site loads ECX=0xABE890; no second stream (this session + all subagents)
- `[RESOLVED]` OQ-17 ftol rounding → truncate toward zero, CW 0x0E7F (disasm 0x007c5f00; read_memory 0x00822d80)
- `[RESOLVED]` OQ-18 direction table → 0x0049F2F0 initializer, N..NW clockwise (decompile this session)
- `[DEFERRED]` OQ-19 `0x00579010` internals + 0x005A0090/0x005A00C0/0x00579B70 (category: bounded-cost-too-high; next: dedicated bridge-tile slice). *2026-07-25: the top-level contract, the `0x005792AD` guard address and the predicate `id>0 && id!=ramp_group_id && ramp_group_id!=-1`, the `RET 8`, the `cell+0x11B += 4` write, the 8-neighbour recursion and the zero-RNG property are all re-verified; only the three named callees remain.*
- `[DEFERRED]` OQ-20 `MapClass__ApplyBridgeTile` + shore/bridge tileset table provenance (DAT_00AA10A0 etc.) (category: bounded-cost-too-high)
- `[PARTLY RESOLVED 2026-07-25]` OQ-21 `MarkBridgesForRepair_Low(0,−1)` pipeline call → it is a **distinct function at `0x00578E60`**, not the `0x0057A0C0` shore family, and it consumes **zero** RNG. Its per-cell writes are still undecoded.
- `[DEFERRED]` OQ-22 CellClass +0xE4/+0xEC identities in tech checks (category: requires-different-system-context; hypotheses: occupier ptr / land type 3)
- `[DEFERRED]` OQ-23 6 remaining ramp builders + 11 level-3 helpers of 0x005905D0 (category: bounded-cost-too-high; structure decoded, cell-exact stamps unverified). *2026-07-25: the ramp builders `0x00593AF0`/`0x00593550`/`0x00593030` and `0x00590FD0` are confirmed RNG-free, which bounds the stream impact of leaving them undecoded.*
- `[DEFERRED]` OQ-24 base-level-4 initialization site for real cells in 0x00599650 (category: bounded-cost-too-high; inferred from +0x30C default + rollback writes)
- `[NEW 2026-07-25]` OQ-25 contents of `g_nRmgWaterAmountMinByMapType` / `…MaxByMapType`, the tables `MapSeedClass__RandomizeDerivedFields 0x00597260` draws WaterAmount from. These decide whether types 3/4 get water at all, and the draw is the *first* in that function — so they are both a gameplay input and an RNG-stream position. **Blocks RMG-01.**
- `[NEW 2026-07-25]` OQ-26 `0x0058D620` region-split internals (4 RNG sites, types 3/4 only, re-entered per split). **Blocks stream-exact RMG-01.**
- `[NEW 2026-07-25]` OQ-27 `0x005A19E0` cliff-drop contract (2 RNG sites in a full-map sweep) and `0x005A1350`'s per-case variant draws reached via `0x005A17F0`. **Blocks stream-exact RMG-01.**
- `[NEW 2026-07-25]` OQ-28 `0x0057A320`'s straight-bank pattern list — the original text's `{0xC7,0x7C,0xF1,0x1F,0xC6,0x6C,0xB1,0x1B}` was not re-derived and has no citation.

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Single-call driver: ≤1 river + ≤1 lake, 10 attempts each, quota-capped | §3 | missing (no RMG at all) | future map-gen module (no existing surface) | reproduce attempt loops + `+0x308` increments exactly | same seed ⇒ identical water layout vs gamemd capture | do NOT iterate lakes to quota — quota is a cap via the ≤75 early-out |
| Quota `ftol(W·H·WA·0.008+100)`; ftol truncates | §4.2, §2.3 | missing | same | exact integer target | boundary WA values (0, 21, 100) match | don't round-to-nearest; don't use 1/2^32 — the scale is 2^-32(1+2^-32) |
| River: edge/heading/width draws, 0.5%-termination, 40-step min, wobble σ and clamps | §4.1 | missing | same | identical draw ORDER incl. every rejection loop | RNG-stream trace comparison vs emulated gamemd | don't skip the consumed-but-unused draws (bridge coin, U[0,100] ramp roll) |
| **Gaussian = Marsaglia polar with rejection, NOT fixed-2-draw Box-Muller** | §4.1 corrected box, §5.1 | missing | same | 2 draws per *attempt*, loop until `0 < u²+v² < 1`; cache the second value for a 0-draw next call | replay a fixed seed through ≥200 Gaussian draws and compare the stream position | **do NOT hardcode 2 draws per refill** — ~21.5% of refills reject at least once and the port desyncs |
| Cross-section straightness: `\|s\| ≤ \|c\|` kills the COLUMN flag | §4.1 C.3 corrected | missing | same | surviving flag selects the bridge direction axis | force a straight E–W cross-section and assert dir ∈ {0,4} | the original doc had this inverted — do not implement from pre-2026-07-25 copies |
| Bridge coin 25%/map at init; bridge min-step U[35,125]; ≤1 bridge; id split id/id−1 | §3.1, §4.1 | missing | same | same observable bridge frequency/placement | 4-seed sample: bridges only on coin-pass maps | +0x310 is NOT INI-driven; don't expose as setting |
| Lake: 200-attempt seed, Gaussian size [75, remaining], heap priority `0.5d+10r−0.02n`, drain-phase hard-fail | §4.2 | missing | same | identical blob shapes | fixed-seed lake cell-set equality | drain phase must fail the WHOLE lake on any stale queue entry |
| Rollback writes `+0x11B=(u8)+0x30C` with default 4 | §2.1, §4.1/4.2 | missing | same | failed attempts restore level 4, not 0 | failed-river map diff | do not assume +0x30C starts at 0 |
| Bridge pass: cursor clear, ramp recursion `+0x11B += 4`, region rebuild w/ ≤74 merge + 1 RNG draw per region-0 merge | §4.4 | missing | same | same region set + terrain rewrites | region-id map equality after pass | merges rewrite REAL cells; don't make them scratch-only |
| 0058F0C0 zero-RNG; 005905D0 always burns U[0,100] per land pair | §4.5/4.6 | missing | same | stream-exact | draw-count audit per phase | DAT_00ABE044 is 0: never place 2-3 ramps |
| Tech: [AI] NeutralTechBuildings, Neutral house, U{0,1,2}/U{0..4} counts, 100 anchor attempts, level-uniform foundation | §4.7 | missing | same | same buildings/cells | fixed-seed building list equality | don't source from ore-patch lamp vectors |

### Stale Docs / Follow-up corrections (do not silently trust these claims elsewhere)

- `RMG_TERRAIN_SHAPING_CORE_GHIDRA_REPORT.md` §2.3: defaults for +0x2BC/+0x2C0/+0x2FC are NOT 0 — outer ctor `0x00595740` sets 2500/5500/500 (and +0x30C=4, +0x310=0). The "MaxTrees=0 → no trees when RMGMD.INI absent" inference is wrong for the constructor default (the INI normally overrides).
- Same doc §3 "+0x304/+0x308 stage/attempt counters": +0x304 = water-cell count, +0x308 = region-id counter, +0x30C = ground level (default 4), +0x310 = bridge coin.
- The driver-entry gate is `WaterAmount != 0`; the `> 20` gate is river-phase-only (task context and doc wording conflated them). **Re-verified 2026-07-25** (`decompile_function 0x0059C580`, `0x00598960`): on types 3/4 with `WaterAmount == 0`, **neither** `0x0059C580` nor the standard seeder `0x0059A6C0` runs — `0x0059C630` still runs unconditionally.
- Ghidra labels `MapClass__MarkBridgesForRepair_High` (0x0057A0C0), `CCINIClass__Constructor` (0x00599650), `TechnoClass__IsOnScreen` (0x00578540), `CellClass__HasBridgeOverlay` (0x004865D0) are drift; roles as documented here. **Update 2026-07-25:** `0x005981F0` has since been relabelled `MapSeedClass__ReadINI` by a parallel session, so it is no longer drift; `0x00599650` still carries the wrong `CCINIClass__Constructor` name. Add to the drift list the three `0x0057A0C0` callees — `MapClass__UpdateBridgeTile_Low` (`0x0057A430`, actually the water notch/strait fixer), `MapClass__ClearBridgeCell_Low` (`0x0057A320`) and `MapClass__SelectBridgeTileVariant_Low` (`0x0057ACF0`, actually the `g_ShorePieces` stamper) — whose names caused a reviewer to wrongly "refute" the `0x0057A0C0` finding during the 2026-07-25 audit (§4.3).
- **New 2026-07-25:** functions renamed in Ghidra during this audit — `0x005980C0` → `RandomMapGenerator__NextGaussian`, `0x0059C580` → `RandomMapGenerator__SeedWaterInlandMountain`, `0x0059D510` → `RandomMapGenerator__CarveRiver`, `0x0059C920` → `RandomMapGenerator__GrowLake`. Plate comments were added to those plus `0x0057A0C0`. The program was **not saved** by the audit session.

## 12. Unverified (YELLOW) — added by the 2026-07-25 audit

Everything in this section is **not proven from the binary as of 2026-07-25**. It is
retained because it may well be right, but it must not be implemented from, and no
downstream doc should cite it as verified. Items are grouped by why they are unverified.

### 12.1 Single-source claims never re-derived (original text kept, no citation exists)

- **`FUN_005A08D0` full contract** (§4.3) — the θ₀-from-clipped-edge rule, the
  `1.5·anglefold(|θc−θ|) + 2.0·rand01` heap priority, the
  `base = trunc(0.5f·max(ln N, 1.0)/stepScale)` step budget and `steps = base + U[0, base/2]`,
  the `θ += gauss·π/4` update, and the scratch-only `+0x4B=1`/`+0x38=id` write set. Only
  its two call sites and their arguments are verified (`0.01f` from the river waterfall
  with `flushFlag=1`; `0.003f` from `FUN_0059E740` with `flushFlag=0`, immediate
  `0x3B449BA6` = 0.003 exactly).
- **`FUN_005A0410` full contract** (§4.3) — the "peel `rings` times, writing scratch
  `+0x38=newId`, real tile `+0x38=0`, `+0x11A=0`, `+0x11B=(u8)+0x30C`" description, and
  the "always returns 1" claim. Only the call arguments `(0, 2, −2)` are verified
  (`disassemble_bytes 0x0059CA92`).
- **`0x0057A320`'s straight-bank pattern list** `{0xC7,0x7C,0xF1,0x1F,0xC6,0x6C,0xB1,0x1B}`
  (§4.3) — the function's identity and its position as sweep 2 are verified; the byte list
  is not. (OQ-28)
- **§4.5 `FUN_0058F0C0`'s "zero RNG, zero FPU" proof** — the zero-RNG half is now
  corroborated by the program-wide `CALL 0x0065C780` scan, but the "zero FPU" claim and the
  ascending-by-construction ordering of the `+0x04` neighbour vector were not re-derived.
- **§4.6 land-mode ramp-builder details** — the `DAT_0082AF18` bit table
  `{0x40,0x80,0x01; 0x20,·,0x02; 0x10,0x08,0x04}`, the threshold
  `T = ftol((15.0−5.0)·(1.0f−tol)+5.0)`, the `g_RampBase+{0,3,7}` tile stamps,
  `+0x11C ∈ {1,4,8}`, `+0x11B = lvl−i−1`, and the `tol > 0.5f` brute-force fallback.
- **§4.7 INI provenance chain** — string `@0x0083D36C` → reader `0x00673926` in the `[AI]`
  section loader. Not checked this session. (The `RulesClass+0xAE0`/`+0xAEC` offsets and
  the `"Neutral"` country string **are** verified; the `+0xADC` claim is refuted.)
- **§4.7 `Unlimbo` vtable slot** — `0x00440580` is named `BuildingClass__Unlimbo` and the
  call signature matches, but the vtable bytes at `+0xD8` were not dumped to prove the slot
  literally holds that pointer. Per project rule, vtable-override claims need a live
  `read_memory` — this one does not have it.
- **§2.1 static initializer `0x0058B740` / atexit dtor `0x0058B760`** — `get_xrefs_to
  0x00ABDFD8` lists both as touching the global, consistent with ctor/dtor registration,
  but `disassemble_bytes 0x0058B73A` was not re-run to confirm the pattern.

### 12.2 Named functions whose bodies are undecoded

- `MapClass__ApplyBridgeTile` (OQ-20) — called from both `0x0057ACF0` and `0x0059E740`.
- `0x005A6C10` `RandomMapGenerator__StampIsometricTileBlock` — only the call-site arguments
  are confirmed (tileset ptr + offset in ECX, `&coords` in EDX, then two literal `−1, −1`
  stack args).
- `0x005904B0` `RandomMapGenerator__PlaceBridgeRepairHut` — only the primary/fallback-rect
  calling pattern from `FUN_0058F2C0` is known.
- `0x005A7440` `RandomMapGenerator__IsUniformLevelBridgeEndArea` — identity taken from its
  Ghidra label plus consistent bool-gate usage, **not** independently decompiled. Note the
  probe rect is `w=7, h=6` for both NS sites (§4.6).
- `0x005A0090`, `0x005A00C0`, `0x00579B70` (OQ-19); `0x0058D620` (OQ-26);
  `0x00578E60`'s cell writes (OQ-21).

### 12.3 Inputs not decoded

- `g_nRmgWaterAmountMinByMapType` / `g_nRmgWaterAmountMaxByMapType` contents (OQ-25).
  Without these the port cannot reproduce WaterAmount, and WaterAmount gates the entire
  mode-3/4 water phase and scales the cell quota and river width.

### 12.4 Verification-method gaps

- **No golden/oracle check exists for any claim in this document.** Everything here is
  read-from-disassembly, which is evidence but not a machine-derived reference. Per the
  project's certification rule, nothing in this doc may be described as parity-VERIFIED
  until a gamemd-derived check (emulated `RandomMapGenerator__Generate` trace, or a
  captured cell/RNG-stream dump at a fixed seed) exists and is named. Treat this doc as an
  implementation *specification*, not as parity evidence.

## Sources

- Ghidra (this session, foreground): decompile_function 0x0059D510, 0x0059C920, 0x0059C580, 0x00598960, 0x005981F0, 0x00595740, 0x0049F2F0; disassemble_function 0x0059C920, 0x007C5F00; disassemble_bytes 0x0059D510–0x0059E62F (full, 2 chunks), 0x00598AC0, 0x0059A4A0, 0x0058B73A; search_instructions "+0x310]"; get_xrefs_to / get_function_callers / get_function_callees as cited; read_memory: all constants in §2.3 plus 0x0089F688, 0x00ABE2E8, 0x00822D80.
- Ghidra (5 parallel read-only subagents, citations inline in their sections): 0x0058EF10 / 0x004A8BF0 / 0x00579010 / 0x0058C800 cluster; 0x0058F0C0 / 0x0058D410; 0x005905D0 / 0x0058F2C0 / 0x00590970 / 0x00590FD0 / 0x00593AF0; 0x005A95B0 / 0x00595400 / 0x005117D0 / 0x00502D30 / 0x0043B740 / 0x0045EC20; 0x005A0160 / 0x005A08D0 / 0x005A0410 / 0x0059E740 / 0x0057A0C0 / 0x005A0700.
- Prior docs: RMG_TERRAIN_SHAPING_CORE, RMG_WATER_SEED_0059A6C0, RMG_START_POINT_SCORING, SKIRMISH_RANDOM_MAP_GENERATOR_00598960, RMG_TIBERIUM (Gaussian helper — **its "2 draws per refill" claim is now refuted, see §4.1; that doc needs the same correction**).

### Audit pass, 2026-07-25 (Ghidra MCP, read-only except labels)

Program identity confirmed first: `get_current_program_info` → gamemd.exe, PE, x86:LE:32,
image base `00400000`.

- `decompile_function`: 0x0059C580, 0x0059D510, 0x0059C920, 0x0059E740, 0x005A0160,
  0x0058EBC0, 0x005980C0, 0x00598030, 0x0057A0C0, 0x0057A430, 0x0057ACF0.
- `disassemble_bytes`: 0x0059DC1B–0x0059DCBF, 0x0059DCC0–0x0059DD8F, 0x0059D83C–0x0059D8CF,
  0x0059D99C–0x0059D9D7, 0x0059C92E–0x0059C98F, 0x0059CA80–0x0059CAA7, 0x00598000–0x0059802F.
- `search_instructions`: operand `0x007eda20` (single site 0x0059D84F), operand `0x007ed9f8`
  (single site 0x0059C952), operand `0x00abdfc8` (single writer 0x0058B79B), and
  `CALL 0x0065c780` scoped to `FUN_0058D620` (4), `FUN_005A19E0` (2),
  `MapClass__MarkBridgesForRepair_Low` (0).
- `read_memory`: 0x007EDA20 (0.07), 0x007E2800 (0.0), 0x007ED898 (2⁻³²·(1+2⁻³²)),
  0x007E1718 (1.0), 0x00ABDFB8 (Gaussian state).
- `get_function_by_address`: 0x0057A320, 0x00578E60. `get_function_callers`: 0x0057A0C0.
- Delegated read-only verification passes (each citing its own calls inline above):
  region/terrace + cliff/re-anchor cluster; bridge pass + `0x00579010` + `0x0058C800`;
  connector pass + `FUN_0058F2C0` + tech placement; MapSeed field writers + `+0x310` coin
  + `0x005981F0`.
- **Ghidra mutations made by this audit** (renames + plate comments only; `save_program`
  deliberately NOT called — the primary session owns saving): renamed 0x005980C0,
  0x0059C580, 0x0059D510, 0x0059C920; plate comments on those four plus 0x0057A0C0.
- INI: `ini/rulesmd.ini:3082` (NeutralTechBuildings).
- Rust: grep `src/` for RMG/RandMap (sentinel only).
