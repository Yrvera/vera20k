# RMG unconsumed options — UrbanPresence, Accessibility, RegionSize

**Target:** gamemd.exe (verified `get_current_program_info`: name `gamemd.exe`, image base `00400000`,
`executable_path` `<ra2-install>/gamemd.exe`).
**Date:** 2026-07-25
**Scope:** the three MapSeed random-map-generator options that the Rust port parses, clamps,
randomizes and round-trips but never feeds into `PipelineInputs` — `urban_presence`,
`accessibility`, `region_size`.

**Headline:** the three fields do **not** share a verdict.

| Field | MapSeed offset | Global | Consumers in gamemd | Verdict |
|---|---|---|---|---|
| UrbanPresence | `+0x60` | `0x00ABE038` | **none** | **Inert in gamemd too — port is accidentally correct** |
| Accessibility | `+0x6C` | `0x00ABE044` | 1 read, MapType 3/4 only | **Real DRIFT** |
| RegionSize | `+0x70` | `0x00ABE048` | 1 read, MapType 3/4 only | **Real DRIFT** |

---

## 0. Anchor facts re-derived this session

The global MapSeed instance is at **`0x00ABDFD8`**. Confirmed independently of any prior plate
comment: `RandomizeDerivedFields` writes `[EDI+0x60]`, and the generator phase functions read the
same field set at absolute addresses `0x00ABE014`/`0x00ABE030`/`0x00ABE044`/`0x00ABE048`, i.e.
`0x00ABDFD8 + 0x3C/0x58/0x6C/0x70` (verified via `get_xrefs_to 0x00ABE014`, `get_xrefs_to
0x00ABE030`, `get_xrefs_to 0x00ABE044`, `get_xrefs_to 0x00ABE048`).

### 0.1 Offset↔name binding proven from the binary's own INI keys

The prior session's field names came from a plate comment. They are re-proven here from the
`.SED`/`[RandomMap]` serializer, which pairs each struct offset with a literal key string — this is
binary-internal evidence, not inference.

Section string `0x0082BB24` = `"RandomMap"` (verified `read_memory 0x0082BB24`, 16 bytes).

**Writer** (`Put_Int(section, key, value)`, verified `disassemble_bytes 0x005978B0–0x005979B0`):

| Store site | Field read | Key ptr | Key string |
|---|---|---|---|
| `0x005978CD` | `[ESI+0x70]` | `0x0082BBBC` | `"RegionSize"` |
| `0x005978E5` | `[ESI+0x44]` | `0x0082BBB0` | `"Ruggedness"` |
| `0x005978FD` | `[ESI+0x6C]` | `0x0082BBA0` | `"Accessibility"` |
| `0x00597975` | `[ESI+0x60]` | `0x0082BB68` | `"UrbanPresence"` |

**Loader** (`Get_Int(section, key, default=current)`, verified `disassemble_bytes
0x00597BB0–0x00597C98`):

| Key ptr | Key string | Stores to |
|---|---|---|
| `0x0082BBBC` `"RegionSize"` | | `[ESI+0x70]` @ `0x00597BF0` |
| `0x0082BBA0` `"Accessibility"` | | `[ESI+0x6C]` @ `0x00597C15` |
| `0x0082BB68` `"UrbanPresence"` | | `[ESI+0x60]` @ `0x00597C94` |

Key strings verified via `search_strings "Urban"` → `0x0082BB68`; `search_strings "Accessibility"`
→ `0x0082BBA0`; `search_strings "RegionSize"` → `0x0082BBBC`; `read_memory 0x0082BBB0` →
`"Ruggedness"`.

**Both directions agree, so `+0x60`=UrbanPresence, `+0x6C`=Accessibility, `+0x70`=RegionSize is
proven, not assumed.**

### 0.2 Correction to the prior session's range-table addresses

`disassemble_function 0x00597260` shows the **min** tables for draws 3 and 4 are the reverse of
what the incoming brief stated:

```
005972a2: MOV EAX,dword ptr [ESI*0x4 + 0x82b0f8]   ; max
005972a9: MOV ECX,dword ptr [ESI*0x4 + 0xabed40]   ; min   <-- UrbanPresence min
005972bc: MOV dword ptr [EDI + 0x60],EAX           ; -> UrbanPresence
005972bf: MOV EAX,dword ptr [ESI*0x4 + 0x82b0d0]   ; max
005972c6: MOV ECX,dword ptr [ESI*0x4 + 0xabed18]   ; min   <-- Accessibility min
005972d9: MOV dword ptr [EDI + 0x6c],EAX           ; -> Accessibility
```

So **UrbanPresence min = `0x00ABED40`** and **Accessibility min = `0x00ABED18`** — the brief had
these two swapped. Both tables are all-zero (verified `read_memory 0x00ABED40` 20 bytes → all `00`;
`read_memory 0x00ABED18` 20 bytes → all `00`), so the numeric result is unaffected; the address
attribution is corrected for future reference.

### 0.3 Per-map-type range tables (all verified by `read_memory`)

Indexed by MapType (`MapSeed+0x3C`, global `0x00ABE014`), 5 entries of `int32`:

| Field | Min table | Min values | Max table | Max values |
|---|---|---|---|---|
| UrbanPresence | `0x00ABED40` | `[0,0,0,0,0]` | `0x0082B0F8` | `[50,100,100,100,0]` |
| Accessibility | `0x00ABED18` | `[0,0,0,0,0]` | `0x0082B0D0` | `[100,100,100,100,20]` |
| RegionSize | `0x0082B080` | `[50,0,35,0,0]` | `0x0082B094` | `[100,100,100,100,50]` |

Verified: `read_memory 0x0082B0F8` (20 bytes) → `32 00…64 00…64 00…64 00…00 00`;
`read_memory 0x0082B0D0` (20 bytes) → `64,64,64,64,20`; `read_memory 0x0082B080` (40 bytes) →
min `[50,0,35,0,0]` followed at `0x0082B094` by max `[100,100,100,100,50]`.

### 0.4 MapType enum

Pointer array of 5 label strings at `0x0082B034` (verified `read_memory 0x0082B034` 20 bytes;
targets confirmed by `get_xrefs_to 0x0082B934`→`0x0082B044`, `get_xrefs_to 0x0082B984`→`0x0082B034`,
`get_xrefs_to 0x0082B948`→`0x0082B040`):

| Index | Ptr | Label |
|---|---|---|
| 0 | `0x0082B984` | `TXT_MAP_ARCHIPELAGO` |
| 1 | `0x0082B970` | `TXT_MAP_CONTINENT` |
| 2 | `0x0082B958` | `TXT_MAP_TEAM_CONTINENTS` |
| 3 | `0x0082B948` | `TXT_MAP_INLAND` |
| 4 | `0x0082B934` | `TXT_MAP_MOUNTAINOUS` |

Label strings verified `read_memory 0x0082B8D0` (120 bytes) and `read_memory 0x0082B948` (140 bytes).

---

## 1. The MapType 3/4 gate — governs BOTH live fields

Both surviving consumers sit inside one branch of `RandomMapGenerator__Generate` (`0x00598960`).
Verified in **raw assembly**, not decompiler output (`disassemble_bytes 0x00598D30–0x00598D80`):

```
00598d55: MOV EAX, dword ptr [ESI + 0x3c]   ; ESI = MapSeed, +0x3C = MapType
00598d58: CMP EAX, 0x4
00598d5b: JZ  0x00598d62                    ; MapType == 4 -> enter
00598d5d: CMP EAX, 0x3
00598d60: JNZ 0x00598d87                    ; MapType not in {3,4} -> SKIP BOTH
00598d62: CALL 0x0058ebc0                   ; region split/cull   (reads RegionSize)
00598d67: CALL 0x0058ef10                   ; BridgeAndConnectorPass (reads Accessibility)
```

`get_xrefs_to 0x0058EBC0` returns exactly two call sites: `0x00598D62` (Generate) and `0x005A1E20`
in `FUN_005A1E10`. `get_xrefs_to 0x005A1E10` returns **no references** — that second caller is dead
code. `get_xrefs_to 0x0058EF10` returns the same two sites (`0x00598D67`, `0x005A1E25`).

**Therefore: Accessibility and RegionSize have effect ONLY on Inland (3) and Mountainous (4) maps.**
On Archipelago / Continent / Team Continents they are rolled, saved, and never read.

---

## 2. UrbanPresence (`+0x60`, global `0x00ABE038`) — INERT IN GAMEMD

### 2.1 Every access, exhaustively

| Site | Kind | Function |
|---|---|---|
| `0x005972BC` | WRITE | `MapSeedClass__RandomizeDerivedFields` (draw 3) |
| `0x00597975` | READ | `.SED` writer (`Put_Int "UrbanPresence"`) |
| `0x00597C94` | WRITE | `.SED` loader (`Get_Int "UrbanPresence"`) |

**No other access exists.** Evidence:

1. `get_xrefs_to 0x00ABE038` → *"No references found to address: 0x00abe038"*. This instrument is
   sound for this address class: the same call correctly returns the absolute readers for
   `0x00ABE044`, `0x00ABE048`, `0x00ABE030` and `0x00ABE014`.
2. Absolute addressing is how generator phases reach the MapSeed (proven for MapType, TiberiumLayout,
   Accessibility, RegionSize in §0). The only other access form is `[reg+0x60]` inside a routine
   holding the MapSeed pointer.
3. `search_instructions operand_pattern="0x60]"` scoped per function returns **no MapSeed field
   read** in any of: `RandomMapGenerator__Generate` (878 instrs, 0 matches),
   `MapSeedClass__ReadINI` (538, 0), `RandomMapGenerator__BridgeAndConnectorPass` (138, 0),
   `FUN_005905D0` (287, 0), `FUN_0058CF90` (44, 0), `FUN_005A35F0` (198, 0),
   `RandomMapSetupDialog__Proc` (766, 0), `FUN_00595BC0` (83, 0), `FUN_0058C800` (496, 0).
   `RandomMapGenerator__CreateStartingPoints`, `RandomMapGenerator__CreateTiberium`,
   `FUN_005A95B0` (tech buildings) and
   `RandomMapGenerator__PaintLatPatchesTreesRocks_Temperate` each match only `[ESP+0x60]`
   **stack locals**, never a MapSeed field.

### 2.2 Contract

- Rolled: `RandomRanged(0, UrbanPresenceMax[MapType])`, max `[50,100,100,100,0]`.
- On MapType 4 (Mountainous) min == max == 0, so `RandomRanged` consumes **no draw** at all
  (its whole body is guarded by `if (min != max)`), leaving Mountainous with seven draws in
  `RandomizeDerivedFields`, not eight.
- Persisted to `[RandomMap] UrbanPresence` and reloaded.
- **Never consulted by any generation phase.** No city structures, pavement, or urban tiles are
  placed as a function of it. There is no urban-tile pass in the pipeline keyed to this value.

### 2.3 Consequence for the port

**The Rust port is already correct on UrbanPresence.** Parsing, clamping, randomizing and
round-tripping it while the generator ignores it reproduces gamemd exactly. The only parity
obligations are the *observable* ones the port already meets: the value must still be rolled in
draw-order position 3 (it consumes RNG from the same stream and therefore shifts every later
draw), and it must still round-trip through the `.SED` key `UrbanPresence`.

> The RNG-consumption point is the one place UrbanPresence is player-visible: skipping the draw
> would change WaterAmount-through-Seed for every subsequent field. That is a draw-order
> obligation, not a generator obligation.

---

## 3. Accessibility (`+0x6C`, global `0x00ABE044`) — REAL DRIFT

### 3.1 Sole reader

`0x005907B8`, inside `FUN_005905D0` (verified `get_xrefs_to 0x00ABE044` → exactly one entry,
`From 005907b8 in FUN_005905d0 [READ]`).
Call chain (verified `get_function_callers 0x005905D0`):
`Generate 0x00598D67` → `RandomMapGenerator__BridgeAndConnectorPass 0x0058EF10` → `FUN_005905D0`.

`FUN_005905D0` is invoked per region and branches on the region's water flag (`region+0x14`):

- `region+0x14 == 0` (**land**) → the connector/ramp carve pass. **This is the Accessibility path.**
- `region+0x14 != 0` (**water**) → `RandomMapGenerator__PlaceLowBridgeDeck` between same-level
  region pairs. **Accessibility is NOT used on the bridge path.**

### 3.2 Exact predicate (verified `disassemble_bytes 0x00590778–0x00590800`)

```
00590792: MOV  ECX, 0xabe890          ; generator RNG instance (g_MapGenRng)
00590797: CALL 0x0065c780             ; Random__Next -> EAX (uint32)
005907a4: FILD qword ptr [ESP+0x2c]   ; zero-extended (EBX = 0)
005907a8: FMUL double ptr [0x007ed8b8]
005907ae: CALL 0x007c5f00             ; Math__ftol (truncate)
005907b3: CMP  EAX, 0x64
005907b6: JA   0x00590792             ; reject and redraw while r1 > 100 (unsigned)
005907b8: CMP  EAX, dword ptr [0x00abe044]   ; r1 vs Accessibility
005907be: JGE  0x005907f6             ; SIGNED: r1 >= Accessibility -> EAX = 0
005907c0: MOV  ECX, 0xabe890
005907c5: CALL 0x0065c780             ; Random__Next
005907d6: FMUL double ptr [0x007ed8b0]
005907dc: FADD double ptr [0x007e1718]
005907e2: CALL 0x007c5f00             ; Math__ftol
005907e7: CMP  EAX, 0x2
005907ea: JA   0x005907c0             ; reject and redraw while r2 > 2
005907f6: XOR  EAX, EAX               ; taken when r1 >= Accessibility
005907fb: INC  EAX                    ; count = EAX + 1
```

Constants (verified `read_memory`, decoded as IEEE-754 doubles):

| Address | Raw bytes | Value | Note |
|---|---|---|---|
| `0x007ED8B8` | `004019000040593e` | `2.3515895014516053e-08` | `× 0xFFFFFFFF` = exactly `101.0` |
| `0x007ED8B0` | `000010000000003e` | `4.656612874161595e-10` | `× 0xFFFFFFFF` = exactly `2.0` |
| `0x007E1718` | `000000000000f03f` | `1.0` | additive bias |

`Math__ftol` (`0x007C5F00`) **truncates toward zero**: it loads the control word from
`0x00822D80` = `0x0E7F` (verified `read_memory 0x00822D80`), whose RC field (bits 11:10) is `11`
= chop. Ghidra's decompilation renders this as `ROUND`, which is **wrong** — read the control word,
not the pseudocode.

**Contract:**

```
r1 = trunc(Random__Next() * 2.3515895014516053e-08)   // redraw while r1 > 100  -> r1 in [0,100]
if (r1 < Accessibility)                               // SIGNED comparison
    r2 = trunc(Random__Next() * 4.656612874161595e-10 + 1.0)  // redraw while r2 > 2 -> r2 in {1,2}
else
    r2 = 0
connections = r2 + 1
```

So **`connections == 1` when the roll fails, and `connections ∈ {2,3}` when it passes.** It is never
uniform over 1..3. Both rejection loops are live but fire only for the single draw
`0xFFFFFFFF` (`r1` would be `101`, `r2` would be `3`) — probability `2^-32` each, still required for
bit-exact RNG stream parity.

**Boundary behaviour:** `Accessibility <= 0` ⇒ `r1 >= Accessibility` always ⇒ `connections == 1`.
This is **not** a skipped pass — one connection is still carved. `Accessibility == 100` ⇒ the roll
passes unless `r1 == 100`.

### 3.3 What it does with the count

Per **ordered pair of adjacent regions** where `this.id < neighbour.id` (dedupe, so each pair is
processed once) **and the two regions' `+0x10` values differ**, the routine:

1. builds the shared border-cell list via `FUN_0058D410`;
2. loops at most 100 attempts, each picking a uniform random border cell (rejection-sampled against
   the list length) and calling `FUN_00590970(cell, lowerLevelId, attemptIndex * 0.01f)`;
3. stops once `connections` carves have succeeded;
4. if **zero** succeeded, clears `region+0x1B`.

`FUN_00590970` (verified `decompile_function 0x00590970`) is the ramp/passage carver: it tests eight
directional edge-mask cases (`0x82`, `0x0A`, `0x28`, `0xA0`, plus four diagonal fallbacks) and calls
one of `FUN_00593AF0` / `FUN_00593550` / `FUN_00593030` / `FUN_00592440` / `FUN_00591740` /
`FUN_00591D80` / `FUN_005910F0`. Its third argument is a leniency float; **once it exceeds `0.5`
(attempt index 51+) four extra fallback orientations are unlocked**, so late attempts can place
ramps the early ones refused.

### 3.4 Player-visible consequence

Accessibility is **the number of ramps carved through each cliff line between two adjacent plateaus
of different height** on Inland and Mountainous maps. Low → every plateau boundary has exactly one
choke point; high → most boundaries get two or three. This directly sets how open or choke-heavy an
Inland/Mountainous skirmish map plays, and it is visible on the minimap at first glance.

---

## 4. RegionSize (`+0x70`, global `0x00ABE048`) — REAL DRIFT

### 4.1 Sole reader

`0x0058ED89` in `FUN_0058EBC0` (verified `get_xrefs_to 0x00ABE048` → exactly one entry,
`From 0058ed89 in FUN_0058ebc0 [READ]`). Reached only from `Generate 0x00598D62` under the
MapType 3/4 gate (§1).

### 4.2 Exact formula (verified `disassemble_function 0x0058EBC0`)

```
0058ed89: FILD  dword ptr [0x00abe048]   ; RegionSize
0058ed8f: FMUL  double ptr [0x007e44e8]  ; * 0.005
0058ed95: FADD  double ptr [0x007e8ae8]  ; + 0.05
0058ed9b: FIMUL dword ptr [0x00abe15c]   ; * MapSeed+0x184  (generated map H)
0058eda1: FIMUL dword ptr [0x00abe158]   ; * MapSeed+0x180  (generated map W)
0058eda7: FADD  ST0,ST0                  ; * 2
0058eda9: CALL  0x007c5f00               ; Math__ftol (truncate)
0058edae: MOV   EBX,EAX                  ; threshold, computed ONCE
```

Constants verified by `read_memory`: `0x007E44E8` = `7b14ae47e17a743f` = **`0.005`**;
`0x007E8AE8` = `9a9999999999a93f` = **`0.05`**.

```
threshold = trunc( 2 * (0.005 * RegionSize + 0.05) * genH * genW )
```

`0x00ABE158` / `0x00ABE15C` are **MapSeed `+0x180` / `+0x184`** (`0x00ABDFD8 + 0x180 = 0x00ABE158`).
They are the generated map dimensions: `MapSeedClass__InitDefaults` writes `[ESI+0x180]` at
`0x00595762` (verified `search_instructions operand_pattern="0x180]"`), and the sibling formula in
`RandomMapGenerator__CreateStartingPoints` at `0x00594B53`–`0x00594B7E` reads the identical pair
and multiplies by `0x007ED8D0` = `b81e85eb51b89e3f` = **`0.03`** (verified `read_memory 0x007ED8D0`),
matching that function's existing plate comment "`threshold = ftol(max(genH*genW*0.03, 400))`".
`get_xrefs_to 0x00ABE158` and `get_xrefs_to 0x00ABE15C` each return exactly these two READ sites and
**no writer**, consistent with the writes being pointer-form.

**The threshold is computed once, before the split loop, so it is constant for the whole pass.**

### 4.3 The split loop (verified `disassemble_function 0x0058EBC0`, `0x0058EDB5`–`0x0058EE8B`)

Immediately before the threshold computation the routine rebuilds every region's cell count: it
zeroes `region+0xC` and the `+0x40..+0x4C` bounding box for all regions, then walks the whole
`genW × genW` cell grid, and for each cell inside the playfield diamond (four bounds tests against
`0x00ABED04` / `0x00ABED08`) increments the owning region's `region+0xC` and extends its bbox.

Then:

```
for each region (index 0 .. regionCount-1):
    if (region+0x1A != 0) continue          // already finalised
    if (region+0x14 != 0) continue          // water region — never split
    if (region->CellCount(+0xC) > threshold):
        if (FUN_0058D620(region) != 0):     // split succeeded
            destroy + remove this region from the list, restart the walk from index 0
    else:
        region+0x1A = 1                     // mark final, never revisited
```

The restart-from-zero on a successful split is why the loop is written as a `goto` back to
`0x0058EDB5`.

**Boundary behaviour / no-op case:** `threshold` scales linearly from `0.1 · genW · genH` at
`RegionSize = 0` to `1.1 · genW · genH` at `RegionSize = 100`. Because regions occupy only the
playfield diamond (roughly half of `genW × genH`), a high RegionSize drives the threshold above the
largest possible region, so **every region is immediately marked final and the split pass becomes a
complete no-op**. This is the specific case where the Rust port's current "ignore RegionSize"
behaviour coincidentally matches gamemd — but only at the top of the range, and only on MapType 3/4.

### 4.4 Player-visible consequence

RegionSize sets **how finely Inland/Mountainous maps are partitioned into terrain regions**. Low →
many small regions → more plateaus, more height changes, more cliff lines (and, via §3, more ramps,
since ramps are only carved between regions of differing level). High → few large regions → broad
open terrain. It also feeds start placement indirectly: `CreateStartingPoints` culls regions below
`genH·genW·0.03` and distributes start slots across the surviving regions, so region granularity
changes how starts are spread.

---

## 5. Ranking by player-visible effect in a stock YR skirmish

1. **RegionSize** — shapes the macro terrain layout (plateau count and size) of every Inland and
   Mountainous map. Changing it changes the silhouette of the map, and it cascades into ramp count
   and start distribution. Fires on every Inland/Mountainous generation; inert on the other three
   map types.
2. **Accessibility** — sets choke-point density on the same two map types. Very visible in play
   (one ramp per cliff versus three), but it is a modifier on the layout RegionSize already decided,
   so it ranks second. Fires on every Inland/Mountainous generation.
3. **UrbanPresence** — zero visible effect, in gamemd as well as in the port. Its only observable
   role is consuming one RNG draw in `RandomizeDerivedFields`.

### Implementation shape

- **RegionSize** — a modification threaded through an existing phase, not a new one. It requires the
  region **split/cull pass** (`FUN_0058EBC0` + `FUN_0058D620`) to exist first. If the port has no
  region-split pass at all, that pass is the real work item and RegionSize is one line inside it.
- **Accessibility** — likewise a modification threaded through the **connector/ramp pass**
  (`BridgeAndConnectorPass` → `FUN_005905D0` → `FUN_00590970`). The count formula is trivial; the
  carrier pass is the work.
- **UrbanPresence** — nothing to implement. Do not add an urban placement pass; gamemd has none.

### Ready for an implementation contract?

- **Accessibility: yes for the count predicate.** The draw sequence, both rejection bounds, the
  signed comparison direction, the truncation mode and the `{1} / {2,3}` outcome split are all
  proven. The surrounding carve pass is **not** fully decoded — `FUN_00590970`'s eight orientation
  cases and its seven callees were read but not verified tile-by-tile.
- **RegionSize: yes for the threshold and the loop.** Formula, constants, comparison direction,
  truncation, skip conditions and the restart-on-split control flow are proven. `FUN_0058D620`
  (the actual split geometry) was **not** decoded this session.
- **UrbanPresence: yes — the contract is "do nothing", and it is proven.**

---

## 6. Tiberian Sun check

None of the three is TS-legacy dead code in the classic sense (no `SpecialFlags` gate, no
disabled-by-default INI switch). The gating is a live YR data gate:

- Accessibility and RegionSize are reachable in a normal YR skirmish **whenever the player picks
  Inland or Mountainous** in the random-map setup dialog. That is ordinary YR behaviour, not legacy.
- UrbanPresence is **genuinely inert in gamemd itself**. It is not gated off — there is simply no
  reader. This is the "port is accidentally correct" case and must not be filed as drift.

The only dead code found is `FUN_005A1E10` (`get_xrefs_to 0x005A1E10` → no references), an
unreferenced duplicate of the `FUN_0058EBC0` + `BridgeAndConnectorPass` call pair.

---

## 7. Unverified (YELLOW)

Claims below were **not** proven from the binary this session. Do not build an implementation on
them without further Ghidra work.

- **`region+0x10` is the region's terrain height/level.** Inferred from usage only: `FUN_005905D0`
  compares it with `!=` and `<` between neighbours and picks the lower one as the carve target, and
  the bridge branch requires equality. The field's writer was not traced.
- **`region+0x14` is the water flag** and **`region+0x1A` is the "finalised / do not split" flag**,
  **`region+0x1B` a connectivity flag.** Inferred from control-flow role in `FUN_0058EBC0` and
  `FUN_005905D0`; writers not traced.
- **`MapSeed+0x180` = generated width and `+0x184` = generated height** (as opposed to the reverse).
  The *product* is proven to be the map cell area and is all the two formulas use, so the H/W
  assignment is not load-bearing here; the individual identification leans on the existing
  `CreateStartingPoints` plate comment, which was not independently re-verified.
- **`FUN_0058D620` (region split) and `FUN_00590970`'s seven carve callees** were not decoded. The
  observable *shape* of a split region and the exact tiles a carve writes are unknown.
- **No exhaustive whole-binary scan for `[reg+0x60]` on a MapSeed receiver was performed.** The
  UrbanPresence no-consumer finding rests on (a) zero absolute xrefs to `0x00ABE038` and (b) scoped
  per-function scans of the generator driver, all reachable RMG phase entry points, the dialog and
  both serializers. A MapSeed-typed pointer read in some function not in that set would not have
  been caught. Confidence is high, but this is sampling, not proof.
- **`search_instructions` cannot match absolute memory operands.** Verified this session: a scan for
  `0xabe048` returns 0 matches across 1,151,698 instructions even though `FILD dword ptr
  [0x00abe048]` demonstrably exists at `0x0058ED89`. Only `get_xrefs_to` sees those. Any prior
  finding that used `search_instructions` to argue *absence* of an absolute reference is invalid.
