# Smudge Spawn Triggers & Damage-Tier List Selection — Ghidra Research Report

**Address(es):** `AnimClass::Start @ 0x00424F00` (primary spawn site), `BuildingClass::DestructionEffects @ 0x004415F0`, `BuildingClass::SpawnSurvivors @ 0x00442D90`, `AnimTypeClass::ReadINI @ 0x00427D00`, `RulesClass::ReadCombatDamage @ 0x0066BBB0`, `Debris_Smoke @ 0x006B5C90`, `SpawnDebris @ 0x006B59A0`
**Confidence:** HIGH (everything below decompiled and stack-traced from x86 assembly)
**Active in YR:** Yes — every smudge in a standard skirmish flows through these paths
**Extends:** `SMUDGE_CLASS_GHIDRA_REPORT.md` §10 (filled the three open gaps); supersedes its `[General] Scorches/Craters/ForceBigCraters` speculation

## 1. Headline finding

**Smudges are NOT spawned by warhead detonation.** The trigger is on **AnimType**,
not WarheadType. A warhead spawns animations via `AnimList=`, and each
animation's first-frame handler (`AnimClass::Start` at `0x00424F00`) reads
three per-AnimType bools and decides whether to drop a smudge:

| AnimType byte offset | INI key | Effect |
|---|---|---|
| `+0x36B` | `Scorch=` (bool) | Anim drops a scorch on first frame |
| `+0x36D` | `Crater=` (bool) | Anim drops a crater on first frame |
| `+0x36E` | `ForceBigCraters=` (bool) | Crater path forces W≥2 H≥2 selection |

This relocates the entire spawn-trigger story from "warhead has flag X" to
"the warhead's AnimList entry is an AnimType with flag X". The smudge doc's
§10 speculation that triggers were rules-side or warhead-side is wrong.

## 2. AnimClass::Start smudge sequence

`AnimClass::Start @ 0x00424F00` runs **once when the animation starts** (first
frame). Smudge logic is the last block of the function, after particle spawn
and tile-anim handling.

### 2.1 Early-out gate

```
height = ObjectClass::GetHeight(this)        // vtable+0x1C8 → 0x005F5F40
if height >= 30:  return                     // 0x42505A: CMP EAX,0x1e / JGE
```

`GetHeight` returns `Location_Z - CellClass::GetGroundHeight(coord) -
(OnBridge ? bridge_offset : 0)`. The check `height < 30` means **smudges only
spawn when the animation is within 30 leptons of ground level**. Anims
floating high in the air (rocketry, paratrooper drops at altitude) never
leave smudges. (Confidence: HIGH, traced both decompilation and assembly.)

### 2.2 dmg/dmg2 parameters

`uVar4_dmg` and `uVar4_dmg2` are pulled from cached SHP frame dimensions:

```
EBP             = AnimType+0x29C     (default 0x1E; cached SHP frame width)
[ESP+0x10]      = AnimType+0x2A0     (default 0x1E; cached SHP frame height)
```

Both default to `0x1E` (30). On first call, `vtable+0x6C` returns nonzero → 
the cached fields are filled by reading the SHP frame rect (`SHP_frame_rect_getter
@ 0x0069E7E0` returns rect; +0x8=width, +0xC=height). On subsequent calls the
cached values are reused.

**Implication:** larger explosion sprites get larger smudges. A 32px-wide
anim fails the `0x3C < dmg` (60) threshold and only picks 1×1 smudges; a
64+px anim crosses the threshold and can pick 2×2 smudges. Real visual
scaling tied to sprite size.

### 2.3 Branch selection (decompiled flow)

```
if AnimType.Scorch:                                    // +0x36B
    if not AnimType.Crater:                            // +0x36D
        SpawnDebris(coord, fwidth, fheight, forceBig=0)
        return
    // both Scorch AND Crater set
    rand = Random__RandomRanged(0, 0x7FFFFFFE)
    if rand * (1.0 / 2^31) < 0.5:                      // 50% probability
        SpawnDebris(coord, fwidth, fheight, forceBig=0)
        return
    // 50% chance: fall through to crater path

if AnimType.Crater:                                    // +0x36D
    CellClass::Reduce_Tiberium(6)                      // SIDE EFFECT: -6 ore on cell
    if AnimType.ForceBigCraters:                       // +0x36E
        Debris_Smoke(coord, 300, 300, forceBig=1)      // forced big
    else:
        Debris_Smoke(coord, fwidth, fheight, forceBig=0)
```

Verified bit-by-bit against assembly at `0x004250AD`, `0x004250D1`,
`0x004250FC`, `0x00425123`. The 50/50 probability uses doubles at
`0x007E3570` (= 1/2^31, the random-to-double normalizer) and `0x007E1738`
(= 0.5 exactly, the threshold).

### 2.4 Stack-arg verification

The Ghidra signatures `Debris_Smoke(coord, dmg, dmg2, forceBig)` and
`SpawnDebris(coord, dmg, dmg2, forceBig)` were verified by tracing the
`PUSH` sequence around each call site in `AnimClass::Start`'s assembly:

| Site | Pre-CALL pushes | EDX at CALL | Result |
|------|-----------------|-------------|--------|
| Scorch (`0x004250C4`) | `PUSH 0` then `PUSH height` then `PUSH coord` (last popped by GetCoords) | `EBP` = width | `SpawnDebris(coord, width, height, 0)` |
| Crater no-ForceBig (`0x0042513A`) | `PUSH 0` then `PUSH height` then `PUSH coord` | `EBP` = width | `Debris_Smoke(coord, width, height, 0)` |
| Crater ForceBig (`0x00425116`) | `PUSH 1` then `PUSH 0x12C` then `PUSH coord` | `0x12C` = 300 | `Debris_Smoke(coord, 300, 300, 1)` |

GetCoords (`AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0`, vtable+0x48)
is `__thiscall` with one stack arg → cleans 4 bytes only, leaving the other
two PUSHes intact for the spawn function.

## 3. RulesClass [CombatDamage] smudge keys are TS-LEGACY DEAD CODE

`RulesClass::ReadCombatDamage @ 0x0066BBB0` parses five smudge-related keys
from `[CombatDamage]` (NOT `[General]` as the smudge doc speculated):

| INI key | RulesClass byte offset | Stored as |
|---------|------------------------|-----------|
| `Scorches` | `+0x7C4` (vector base) | `DynamicVector<SmudgeTypeClass*>` — ptr at `+0x7D4`, cap `+0x7D8`, count `+0x7DC` |
| `Scorches1` | `+0x7E0` | ptr `+0x7F0`, cap `+0x7F4`, count `+0x7F8` |
| `Scorches2` | `+0x7FC` | ptr `+0x80C`, cap `+0x810`, count `+0x814` |
| `Scorches3` | `+0x818` | ptr `+0x828`, cap `+0x82C`, count `+0x830` |
| `Scorches4` | `+0x834` | ptr `+0x844`, cap `+0x848`, count `+0x84C` |

(Absolute addresses with `g_RulesClass_Instance @ 0x008871E0` base:
`0x008879B4 / 9D0 / 9EC / A08 / A24` for the data pointers.)

**`get_xrefs_to` against all five data pointers and all five vector-base
addresses returns "No references found".** The lists are populated from INI
but **never read by anything** in YR. They are TS-legacy dead code — the
TS-era branch selection by damage tier was replaced in YR by the live
per-type filter in `Debris_Smoke` / `SpawnDebris` (which iterate ALL
SmudgeTypes and filter by per-type `Crater=`/`Burn=` flags).

There is **no `Craters=` key parsed at all** — `search_strings("Craters")`
returns only `ForceBigCraters`. The smudge doc's `[General] Craters=` was
never a real key.

This is a meaningful TS ghost: an implementation that reads
`Scorches/Scorches1..4` from rules and uses them for spawn picks would be
emulating dead TS code, not YR behavior.

## 4. Building destruction smudges (two paths)

Buildings drop smudges via two separate code paths:

### 4.1 BuildingClass::DestructionEffects @ 0x004415F0

When a building dies AND its foundation is at least 2×2:
```
GetFoundationWidth(); GetFoundationHeight()  → both > 1
Random__RandomRanged(0, W-2)                 // result discarded — RNG advance
Random__RandomRanged(0, H-2)                 // result discarded — RNG advance
roll = Random__RandomRanged(0, 99)
coord = (cell_X*256+128, cell_Y*256+128, building.Z)
if roll < 50:
    SpawnDebris(coord, 100, 100, 1)         // forceBig scorch (one big mark)
else:
    Debris_Smoke(coord, 100, 100, 1)        // forceBig crater (one big mark)
```

The two discarded `RandomRanged` calls are determinism-relevant — they
advance the global RNG state. A port that skips them will desync replays /
multiplayer hashes from the original.

### 4.2 BuildingClass::SpawnSurvivors @ 0x00442D90

Per surviving foundation cell, after passing `CheckCellPassability`:
```
roll = Random__RandomRanged(0, 99)
coord_offset = FUN_0049f420(0x80, 0)        // random offset within cell, magnitude 0x80
coord = cell_center + coord_offset
if roll < 50:
    SpawnDebris(coord, 100, 100, 0)         // small scorch
else:
    Debris_Smoke(coord, 100, 100, 0)        // small crater
```

`forceBig=0` here, but `dmg=100 > 0x3C` and `dmg2=100 > 0x32` → big smudges
are still allowed by the threshold check. Effective behavior matches the
forceBig path. The visible difference vs §4.1 is **per-cell randomized
position within the cell** rather than dead-center.

### 4.3 Combined behavior

A destroyed 4×4 building therefore drops:
- 1 large scorch-or-crater dead-center (§4.1, with two RNG-advance calls)
- Up to 16 smaller scorches/craters scattered across foundation cells (§4.2)

…producing the visible "wreckage spread" of a destroyed conyard / war
factory vs the lone scorch under a destroyed Pillbox. (Confidence: HIGH.)

## 5. SmudgeType list filtering inside spawn functions

(Confirmed in the audit but recapped here since they're load-bearing for
the brainstorm.)

- `Debris_Smoke @ 0x006B5C90` iterates ALL SmudgeTypes (`DAT_00A8EC1C`,
  count `DAT_00A8EC28`), filtering on `+0x2A0 != 0` (Crater flag). Per-call
  candidate list is built **live** from the global registry, not from a
  pre-built rules list.
- `SpawnDebris @ 0x006B59A0` does the same with `+0x2A1 != 0` (Burn flag).
- The size filter is then applied:
  - `forceBig == 0`: keep types with `Width==1 AND Height==1`, **plus** any
    size if `0x3C < dmg AND 0x32 < dmg2` (i.e. dmg>60 AND dmg2>50).
  - `forceBig != 0`: keep types with `Width≥2 AND Height≥2` only.
- Random pick from the filtered list via `Random__RandomRanged`. If the
  filtered list is empty, falls back to picking from the unfiltered Crater/
  Burn list.

## 6. Tiny-detail ledger (the things that compound)

Every item below is a constant, ordering, or edge case that a "summary"
implementation would lose. Every one matters for parity.

1. **Smudge-spawn altitude gate is `< 30` leptons (`< 0x1E`)**, NOT `<= 30`.
   Verified at `0x42505A` (`CMP EAX, 0x1E; JGE`). [GHIDRA 0x42505A]
2. **Default dmg/dmg2 when SHP frame rect not yet cached is `0x1E` = 30**.
   Both width AND height default to 30 — initialized via `MOV EBP, 0x1E` and
   `MOV [ESP+0x10], EBP`. [GHIDRA 0x424F57, 0x424F62]
3. **The 50/50 probability when both Scorch+Crater are set is EXACTLY 0.5**
   (double at `0x007E1738`), not "approximately". The random source is
   `Random__RandomRanged(0, 0x7FFFFFFE)` (NOT `0x7FFFFFFF`), normalized by
   `1/2^31` (double at `0x007E3570`). [GHIDRA 0x42507A-0x4250AB]
4. **The 50/50 multiplied through is `(rand * 2^-31) < 0.5`**, equivalent
   to `rand < 2^30 = 0x40000000`. A port that uses `(rand & 1) == 0` is
   close but NOT identical (different RNG-state advancement).
5. **`AnimClass::Start`'s crater path calls `CellClass::Reduce_Tiberium(6)`
   BEFORE testing `ForceBigCraters`** — every crater selected by this AnimType
   branch destroys 6 units of ore on the cell, INCLUDING when the crater itself
   fails to place (`CanPlaceHere` gates inside `Debris_Smoke` later). Direct
   `BuildingClass::DestructionEffects` / `SpawnSurvivors` calls to
   `Debris_Smoke` do **not** reduce ore. Order matters: the Anim branch's ore
   reduction is not conditional on smudge placement success.
   [GHIDRA 0x004250E1-0x004250E7; 0x004415F0; 0x00442D90; 0x006B5C90]
6. **`Reduce_Tiberium(6)` uses the cached coord at `[ESP+0x1C]`** — the
   coord obtained from the FIRST GetCoords call at the top of the function
   (line 1 of decompilation), NOT the just-recomputed coord. Different
   reference point if the anim has been moving. [GHIDRA 0x004250E1]
7. **AnimClass::Start runs ONCE — first frame only** — so smudges spawn
   on anim creation, not anim end. A port that fires smudges on
   destruction would be wrong. (Confirmed by function role + caller trace.)
8. **`AnimType+0x29C` (cached width) and `AnimType+0x2A0` (cached height)
   are mutated on first read** — these are `const`-looking fields on the
   type that get filled lazily. Cross-AnimClass sharing: the second anim
   of the same type uses the cached values from the first. Implementation
   note: must initialize these to `0xFFFFFFFF` (-1) like the original to
   trigger the cache fill. [GHIDRA 0x424F78, 0x424FA8]
9. **The `Width`/`Height` fields of SmudgeType (`+0x298`, `+0x29C`) are
   cell counts, NOT pixel counts** — confirmed via the smudge doc audit
   and the filter logic `*(int *)(iVar1 + 0x298) == 1` checking for
   1×1-cell smudges. Don't conflate with the anim-side `+0x29C/+0x2A0`
   which are pixel dimensions of SHP frames.
10. **`BuildingClass::DestructionEffects` calls `Random__RandomRanged`
    THREE times before the smudge pick:**
    - `RandomRanged(0, W-2)` (discarded result — RNG advance only)
    - `RandomRanged(0, H-2)` (discarded result — RNG advance only)
    - `RandomRanged(0, 99)` (the actual scorch/crater pick)
    A port that skips the discarded calls will desync. [GHIDRA 0x4416XX block]
11. **`BuildingClass::SpawnSurvivors` per-cell coord is random-offset
    within the cell, magnitude `0x80` leptons** — `FUN_0049F420(0x80, 0)`
    returns a (dx, dy, dz) offset added to cell-center coords. Note the
    magnitude is `0x80` (= half a cell side in leptons), so smudges scatter
    within the cell without crossing into neighbors. [GHIDRA 0x004432F2,
    0x00443387]
12. **The threshold check is `0x3C < dmg AND 0x32 < dmg2` — STRICTLY
    LESS-THAN, not less-or-equal.** `dmg=60` exactly fails; `dmg=61` passes.
    Same for `dmg2=50` (fails) vs `dmg2=51` (passes). Confirmed verbatim
    in the binary. (Cross-check from the smudge-class audit.)
13. **`forceBig != 0` means `*any* nonzero value`, not specifically `1`.**
    The crater-with-ForceBigCraters path passes `0x12C = 300` as the
    forceBig stack slot — works because the test is `if (param_4 != 0)`,
    not `if (param_4 == 1)`. Implementation must check truthiness, not
    equality with 1. [GHIDRA 0x004250FE → 0x6B5DCC test path]
14. **The crater path does NOT have a "scorch overlay" backup** — if both
    Scorch and Crater are set on the AnimType and the 50/50 lands on the
    crater branch, the function falls through to the crater code. There's
    no double-spawn. Each anim drops at most ONE smudge per frame. Confirmed
    by the early `return` after each successful spawn arm. [GHIDRA 0x004250CA, 0x0042511B, 0x00425141]
15. **The `CellClass::Reduce_Tiberium(6)` call uses immediate `6`, NOT a
    rules constant** — hardcoded 6-unit ore reduction per crater. No INI
    key controls this in YR. [GHIDRA 0x004250E5: `PUSH 0x6`]
16. **`ForceBigCraters=` exists as a per-AnimType key (offset `+0x36E`),
    NOT a global rule** — the smudge doc's reference to `[General]
    ForceBigCraters` is wrong. Read by `AnimTypeClass::ReadINI @ 0x00427D00`
    line corresponding to string at `0x008185E4`. [GHIDRA 0x00427ED7]
17. **The crater-with-ForceBigCraters path passes hardcoded `dmg=300,
    dmg2=300` regardless of the anim's frame size.** Even a tiny anim with
    `ForceBigCraters=yes` spawns big-only crater picks. [GHIDRA 0x00425104]
18. **The `BuildingClass::DestructionEffects` smudge fires only when
    foundation is at least 2×2.** A 1×1 building (e.g. Sentry Gun) drops
    NO scorch/crater from this path; only the per-cell §4.2 path triggers.
    [GHIDRA 0x4416XX `if (1 < W) && (1 < H)`]
19. **`AnimType.Crater=yes` causes `Reduce_Tiberium(6)` even if
    `CanPlaceHere` later fails (e.g. cell has overlay).** The ore is
    reduced regardless of whether a visible crater appears. [GHIDRA flow:
    Reduce_Tiberium called BEFORE Debris_Smoke at 0x004250E7]
20. **The dedup-against-repeat-hits described in the smudge doc §6 is
    DEAD CODE.** Confirmed in the audit: `DAT_00B0B788/8A` are zero-init
    and never written by smudge code. Repeat-hit prevention is via
    `CanPlaceHere` checking `Cell+0x48 != -1`. Carried forward as
    correction.

## 7. Per-AnimType INI keys read by AnimTypeClass::ReadINI

For brainstorm reference, the full set of AnimType bool keys read at
`0x00427D00` that affect smudge / damage behavior:

| Key | Offset | Read at | Affects smudges? |
|-----|--------|---------|------------------|
| `Scorch` | `+0x36B` | 0x428105 | YES — gate for SpawnDebris |
| `Crater` | `+0x36D` | 0x42811E | YES — gate for Debris_Smoke + Reduce_Tiberium |
| `ForceBigCraters` | `+0x36E` | 0x428137 | YES — forces W≥2 H≥2 picks |
| `Sticky` | `+0x36F` | 0x428150 | No |
| `Bouncer` | `+0x35A` | — | No |
| `Tiled` | `+0x35B` | — | No |
| `IsTiberium` | `+0xD6 (idx)` | — | No |

Defaults: all bool flags default to whatever the constructor at
`0x00427530` initializes them to (typically 0 / false).

## 8. INI keys (canonical list for parity)

| INI section | Key | Type | Default | Effect | YR-active? |
|-------------|-----|------|---------|--------|------------|
| `[<AnimType>]` (artmd.ini) | `Scorch` | bool | no | Anim drops scorch on first frame (vtable Start handler) | YES |
| `[<AnimType>]` (artmd.ini) | `Crater` | bool | no | Anim drops crater on first frame; also triggers `Reduce_Tiberium(6)` on cell | YES |
| `[<AnimType>]` (artmd.ini) | `ForceBigCraters` | bool | no | Crater path forces W≥2 H≥2 selection; passes (300, 300, 1) instead of (frameW, frameH, 0) | YES |
| `[<SmudgeType>]` (rulesmd.ini) | `Crater` | bool | no | SmudgeType is selectable by Debris_Smoke (crater filter pool) | YES |
| `[<SmudgeType>]` (rulesmd.ini) | `Burn` | bool | no | SmudgeType is selectable by SpawnDebris (scorch filter pool) | YES |
| `[<SmudgeType>]` (rulesmd.ini) | `Width` | int | 1 | Footprint cells (used by both filter and CanPlaceHere) | YES |
| `[<SmudgeType>]` (rulesmd.ini) | `Height` | int | 1 | Footprint cells | YES |
| `[CombatDamage]` (rulesmd.ini) | `Scorches` | list | empty | **TS-LEGACY DEAD** — parsed into RulesClass+0x7C4 but never read | NO |
| `[CombatDamage]` (rulesmd.ini) | `Scorches1..4` | list | empty | **TS-LEGACY DEAD** — same as above | NO |
| (none) | `Craters` | — | — | **DOES NOT EXIST as a parsed key in YR** — smudge doc §1 erroneous | — |

## 9. Integration / call graph

```
WeaponTypeClass / WarheadTypeClass::Detonate (0x004690B0)
    └─→ spawns animations from Warhead.AnimList=  
            └─→ AnimClass::Constructor → AnimClass::Start (0x00424F00)  ← SMUDGE TRIGGER 1
                    ├─ if AnimType.Scorch: SpawnDebris (filter Burn=yes)
                    └─ if AnimType.Crater: Reduce_Tiberium(6) + Debris_Smoke (filter Crater=yes)

BuildingClass::Destroy
    ├─→ BuildingClass::DestructionEffects (0x004415F0)  ← SMUDGE TRIGGER 2
    │       └─ if foundation ≥ 2×2: 50/50 SpawnDebris(forceBig) | Debris_Smoke(forceBig)
    └─→ BuildingClass::SpawnSurvivors (0x00442D90)      ← SMUDGE TRIGGER 3
            └─ per cell, if CheckCellPassability: 50/50 SpawnDebris | Debris_Smoke

MapInit
    └─→ SmudgeClass::ReadINI (0x006B4C80)               ← SMUDGE TRIGGER 4 (map load)
            └─ for each [Smudge] entry with IsBaked != 1: SmudgeClass constructor
```

(No other live callers of Debris_Smoke or SpawnDebris exist —
`get_function_callers` returned exactly the three above plus AnimClass::Start.)

## 10. Current Rust Implementation Status

| Subsystem | Rust file | Status |
|-----------|-----------|--------|
| `[SmudgeTypes]` parser | none | **MISSING** — no parser for the rules-side numeric list |
| `[Smudge]` map-load parser | none | **MISSING** — `MapFile` does not parse the `[Smudge]` section |
| Per-cell `SmudgeTypeIndex` storage | none | **MISSING** — no `SmudgeGrid` analogue to `OverlayGrid` |
| `AnimType.Scorch / Crater / ForceBigCraters` parsing | [src/rules/anim_type.rs](src/rules/anim_type.rs) (does not parse these flags) | **MISSING** — three INI bools to add |
| `WarheadType.AnimList` → animation spawn | [src/sim/combat/mod.rs:535](src/sim/combat/mod.rs#L535) (`ExplosionEffect`) | **PARTIAL** — animations spawn for rendering but no `AnimClass::Start`-equivalent fires the smudge logic |
| Smudge spawn dispatcher (`AnimClass::Start` analogue) | none | **MISSING** — needs height-gate + 50/50 probability + `Reduce_Tiberium(6)` side-effect |
| Building destruction → smudges | none in [src/sim/combat/mod.rs:420](src/sim/combat/mod.rs#L420) | **MISSING** — both DestructionEffects and SpawnSurvivors paths absent |
| Render path for smudges | none | **MISSING** — needs a static decal layer between terrain and entities |

(File paths verified at audit time. Spot-checking line numbers may drift; verify before plan writing.)

## 11. Resolved follow-ups (post-publication)

### 11.1 IsoTileTypeClass `+0x2E0` "accepts-smudge" gate — RESOLVED

The CanPlaceHere gate at CellClass+0x38 → IsoTileTypeClass+0x2E0 is the
**`Morphable=` per-TileSet bool** documented in
`ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §2 and §3.3.

| Field | Value |
|-------|-------|
| INI section | `[TileSetNNNN]` (per theater INI: temperatmd.ini, snowmd.ini, urbanmd.ini, desertmd.ini, lunarmd.ini) |
| INI key | `Morphable=` |
| Type | bool |
| Default | `false` |
| Stored at | IsoTileTypeClass+0x2E0 (byte) |

**Implication:** smudges only land on tilesets that explicitly opt in via
`Morphable=yes`. Most decorative tilesets (cliffs, water, beaches, ice,
shores, ramps) default to `false` and therefore reject smudges. Standard
"flat ground" tilesets (Clear, Rough, Sand, Green, Pave) typically have
`Morphable=yes` set. Implementation: include `morphable: bool` on our
TileSet struct, propagate it onto `ResolvedTerrainCell` (as
`accepts_smudge` or similar), and gate `SmudgeGrid::can_place_here`
on it. (Confidence: HIGH — cross-doc verified.)

### 11.2 `FUN_0049F420(magnitude, flag)` — RESOLVED

This is the per-game **"random direction, fixed magnitude, optional
cell-snap"** offset helper. Used by 7 systems including building
destruction smudges, bullet impact randomization, and FlyLocomotion
processing. Decompiled and disassembled at `0x0049F420`.

**Algorithm (verified from FPU instruction trace):**

```
1. byte = Random__Next() & 0xFF    ; consume EXACTLY 1 RNG byte
2. binary_angle_raw =
     (signed_short_cast(byte << 8)) - 0x3FFF
3. angle_rad = binary_angle_raw * (-pi / 32768)    ; constant at 0x007E2810
4. cos_v = Cos_lookup(angle_rad)    ; FPU fcos via lookup
5. sin_v = Sin_lookup(angle_rad)    ; FPU fsin via lookup
6. dx_leptons = (int)(sin_v * magnitude)            ; ftol round
   dy_leptons = (int)(-cos_v * magnitude)            ; FSUBR pattern: base.Y - cos*mag
7. result.X = base.X + dx_leptons
   result.Y = base.Y + dy_leptons
   result.Z = base.Z                                 ; Z preserved unchanged
8. if abs(result.X) >> 8 >= 0x200  OR  abs(result.Y) >> 8 >= 0x200:
       result = base                                  ; bounds-check fallback (>=512 cells)
9. if flag != 0:
       result.X = (result.X & ~0xFF) + 0x80          ; snap to cell-center
       result.Y = (result.Y & ~0xFF) + 0x80
       (Z untouched)
```

**Smudge-relevant call:** `FUN_0049F420(magnitude=0x80, flag=0)`. Effect:
random unit vector * 128 leptons, added to base coord, no cell-snap.

**HOW IT'S USED IN SpawnSurvivors:** The CALLER (BuildingClass::SpawnSurvivors
at `0x004432F2-0x00443354`) takes the offset coord, drops to cell coords
via `>> 8`, then re-builds at the resolved cell's center via
`(cell * 0x100) + 0x80` for both X and Y. Net behavior: **the random
offset can shift the smudge to a NEIGHBORING cell** (within 1 cell of the
foundation cell, since 128 leptons = half a cell). The smudge then places
at the picked cell's center, NOT sub-cell. This is the mechanism
that scatters debris BEYOND the foundation footprint.

**Determinism implication:** the offset MUST consume exactly 1 RNG byte
per call. The angle table is a 256-entry deterministic lookup. Implementation:
pre-compute a `[Coord; 256]` unit-vector table at engine init from the
formula above; runtime use is `let v = unit_vec_table[rng.next_byte()]; dx = v.x * magnitude / scale; ...`.
No floating-point at runtime — keep it sim-fixed. (Confidence: HIGH —
fully decompiled and disassembled.)

### 11.3 vtable+0x6C resolver on AnimClass

User-accepted as deferred. Practical impact is just "default 30 if SHP
frame rect not yet cached" — eliminated by eager frame-dim init at
AnimType registry load.

### 11.4 DestructionEffects' two discarded RandomRanged calls

User-accepted as pure RNG-state advances. Implementation must replicate.

### 11.5 Reduce_Tiberium(6) coord source

Verified — uses the early-cached coord at `[ESP+0x1C]`, not a
just-recomputed coord. For static cells (most cases), no observable
difference. For moving anims, the side-effect lands on the original
cell, not the current. Keep for visibility.

## Sources

### Ghidra addresses decompiled
- `0x00424F00` — `AnimClass::Start` (full decompile + assembly trace)
- `0x004415F0` — `BuildingClass::DestructionEffects` (full assembly trace through smudge call)
- `0x00442D90` — `BuildingClass::SpawnSurvivors` (full decompile + assembly trace)
- `0x00427D00` — `AnimTypeClass::ReadINI` (full decompile, AnimType offsets confirmed)
- `0x00427530` — `AnimTypeClass::Constructor` (defaults verified)
- `0x0066BBB0` — `RulesClass::ReadCombatDamage` (Scorches/Scorches1..4 parser)
- `0x006B5C90` — `Debris_Smoke` (re-verified from audit)
- `0x006B59A0` — `SpawnDebris` (re-verified from audit)
- `0x005F5F40` — `ObjectClass::GetHeight` (vtable+0x1C8 resolved)
- `0x005F3E30` — vtable+0x6C resolver (left as Open Question)
- `0x00422BE0` — `AnimClass::GetCoords_WithOwnerOffset` (vtable+0x48 resolved)

### Globals / constants read
- `0x008871E0` — `g_RulesClass_Instance` (RulesClass base address for offset math)
- `0x007E3570` — `1.0 / 2^31` (double, RandomRanged normalizer)
- `0x007E1738` — `0.5` (double, scorch/crater 50/50 threshold)
- `0x008879B4`, `0x008879D0`, `0x008879EC`, `0x00887A08`, `0x00887A24` — Scorches{,1..4} data ptrs (NO XREFS — TS-legacy dead)
- `0x00A8EC1C` / `0x00A8EC28` — SmudgeType global array / count
- `0x00B0B788` / `0x00B0B78A` — dedup globals (zero-init only, see audit)

### Doc files referenced
- `SMUDGE_CLASS_GHIDRA_REPORT.md` (parent — corrected here in §3, §6 ledger #20, §8 row "Craters")
- `WARHEAD_DETONATE_GHIDRA_REPORT.md` (warhead → AnimList linkage, anim-spawn entry point)
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` (cell offsets +0x44/+0x48 for CanPlaceHere gates)
- `AUDIT_LOG.md` (recent SMUDGE_CLASS audit dated 2026-05-06)

### INI files checked
- `ini/rulesmd.ini` `[SmudgeTypes]` (lines 1682-1716+), `[CombatDamage]` (Scorches keys at 806-810, no `Craters=` key)
- `ini/artmd.ini` (per-AnimType `Scorch=`, `Crater=`, `ForceBigCraters=` flags — present per `Scorch=yes` pattern matches)
