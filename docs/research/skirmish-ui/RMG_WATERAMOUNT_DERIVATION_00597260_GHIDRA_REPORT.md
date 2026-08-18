# RMG WaterAmount derivation — `MapSeedClass__RandomizeDerivedFields` (0x00597260)

**Date:** 2026-07-25
**Program identity:** `gamemd.exe`, PE, x86:LE:32, image base `00400000`, 10035 functions
(verified: `get_current_program_info`).
**Scope:** how `MapSeed.WaterAmount` (+0x4C) — the field that gates the entire map-type-3/4
water phase — is produced, from which tables, on which RNG stream, and on which code paths.
**Authority:** everything in the verified body below was read out of the binary in this
session. Anything I could not prove is in the **Unverified / residual risk** section at the
end. Prior plate comments in the Ghidra project were treated as hypotheses and re-derived
from assembly; label drift found is recorded in §8.

---

## 0. TL;DR for the port

```
WaterAmount = RandomRanged(g_MainRng,
                           WATER_MIN_BY_MAP_TYPE[MapType],
                           WATER_MAX_BY_MAP_TYPE[MapType])

WATER_MIN_BY_MAP_TYPE = [75,   0,  50,   0,   0]   // 0x0082B0A8
WATER_MAX_BY_MAP_TYPE = [100, 25, 100, 100, 100]   // 0x0082B0BC
//                       Arch  Cont TeamC Inl  Mtn
```

* Both bounds **inclusive**.
* `RandomRanged` **consumes no draw when `min == max`** — matters for other fields, not for
  WaterAmount (no stock entry has min == max here).
* No truncation, no rejection-bias correction beyond the masked rejection loop, no clamp
  inside the routine; `MapSeedClass__ClampFields` afterwards clamps +0x4C to 0..100, a no-op
  for every stock range.
* It is draw **#1** of the derived-field block. Get the order right or every later field
  drifts.
* Map types 3 (Inland) and 4 (Mountainous) both have min = 0, so a legitimately generated
  Inland/Mountainous map can come out with `WaterAmount == 0` and **no water phase at all**.

---

## 1. Function contract — `MapSeedClass__RandomizeDerivedFields` (0x00597260)

Signature (from assembly, `disassemble_function 0x00597260`):

```
__thiscall void MapSeedClass__RandomizeDerivedFields(MapSeed* this /*ECX*/,
                                                     int mapType /*[ESP+8]*/)
```

`PUSH ESI / MOV ESI,[ESP+8] / PUSH EDI / MOV EDI,ECX … RET 0x4` — one stack argument,
`this` in ECX, callee-cleans 4 bytes. `ESI` = mapType (the array index), `EDI` = `this`.

All three call sites pass `this->MapType` (+0x3C) as the argument
(`get_assembly_context 0x005967FD,0x00596E08,0x005973E7`):

| call site | `this` (ECX) | argument |
|---|---|---|
| `0x005967FD` | `MOV ECX,0xABDFD8` (global MapSeed) | `MOV EDX,[0x00ABE014]` = g_MapSeed+0x3C |
| `0x00596E08` | `MOV ECX,ESI` | `MOV EDX,[ESI+0x3C]` |
| `0x005973E7` | `MOV ECX,ESI` | `MOV ECX,[ESI+0x3C]` |

`0x00ABE014 − 0x00ABDFD8 = 0x3C`, which also pins the **global MapSeed object at
`0x00ABDFD8`** (corroborated by the Randomize handler writing `DAT_00ABE010/14/18/20/3C/40/4C/50`
into exactly the +0x38/+0x3C/+0x40/+0x48/+0x64/+0x68/+0x74/+0x78 slots —
`decompile_function 0x00596300`).

### 1.1 RNG instance — `g_MainRng`, **not** the scenario RNG

Every one of the eight `CALL 0x0065C7E0` sites is preceded by a literal
`MOV ECX,0x886B88` (`disassemble_function 0x00597260`, offsets `0x00597278`, `0x00597295`,
`0x005972B2`, `0x005972CF`, `0x005972EC`, `0x00597303`, `0x00597354`, `0x00597365`).

`0x00886B88` is `g_MainRng` (`list_globals name_substring=MainRng` → `g_MainRng @ 00886b88`,
95 xrefs). Identity confirmed independently rather than from the label:
`get_xrefs_to 0x00886B88` shows the only WRITEs are at `0x0052FE51` and `0x0052FEAB`, both
inside `Init_Random_Number_System` (`decompile_function 0x0052FC20`), where the code seeds
twice and copies 0xFD (253) dwords first into `g_ScenarioClass_Instance + 0x218`
(the synchronised scenario RNG) and then into `&g_MainRng`. **Two separate streams, same
seed value.** The RMG uses the second one.

Generator shape, read off `Random__RandomRanged` (`decompile_function 0x0065C7E0`):
`+0x00` guard byte, `+0x04` tap index A, `+0x08` tap index B, `+0x0C` 250 × `uint32` state,
both indices reset to 0 when they exceed `0xF9` (249), next word = `state[A] ^= state[B]`.
Total 12 + 1000 = 1012 bytes = 253 dwords, exactly matching the `0xFD`-dword copy in
`Init_Random_Number_System`. This is an R250-style lagged-Fibonacci XOR generator.

### 1.2 `Random__RandomRanged` bound semantics (0x0065C7E0)

```
__thiscall int Random__RandomRanged(RandomClass* this /*ECX*/, int min, int max)
```
Argument order proven from the push order at the call sites: the value pushed **second**
(i.e. `PUSH ECX` at `0x00597277`) is the first parameter. For WaterAmount that value comes
from `0x0082B0A8`, so `0x0082B0A8` is the **min** table and `0x0082B0BC` the **max** table.

Body (`decompile_function 0x0065C7E0`):

1. `if (min == max) return min;` — **early-out that consumes NO random draw.**
2. Otherwise swap so `min <= max`; `span = max - min` (unsigned).
3. Find the index of the highest set bit of `span` (scan from bit 31 down).
4. Rejection loop: draw a fresh 32-bit word, mask it to `msb+1` bits, repeat while the
   masked value `> span`.
5. `return min + value;`

⇒ **uniform over the closed interval `[min, max]`**, both ends inclusive. Not modulo, not
exclusive-max, no truncation, no clamp. Note the loop is a rejection loop, so the number of
raw 32-bit words consumed per call is variable (≥ 1) — a port that reproduces the same R250
stream reproduces the same consumption automatically; a port that draws differently will not.

Guard byte: if `*(char*)this != 0` the loop body yields 0 and the call returns `min` without
drawing. See §9 — I did not establish what sets that byte.

### 1.3 Exact draw order (LOAD-BEARING)

| # | writes | expression |
|---|---|---|
| 1 | `+0x4C` WaterAmount | `RandomRanged(g_MainRng, WaterMin[mt], WaterMax[mt])` |
| 2 | `+0x44` Ruggedness | `RandomRanged(g_MainRng, RuggedMin[mt], RuggedMax[mt])` |
| 3 | `+0x60` UrbanPresence | `RandomRanged(g_MainRng, UrbanMin[mt], UrbanMax[mt])` |
| 4 | `+0x6C` Accessibility | `RandomRanged(g_MainRng, AccessMin[mt], AccessMax[mt])` |
| 5 | `+0x70` RegionSize | `RandomRanged(g_MainRng, RegionMin[mt], RegionMax[mt])` |
| — | `+0x54` Tiberium | `this->Resources(+0x40) * 20` — **computed, no draw** |
| 6 | `+0x58` TiberiumLayout | `RandomRanged(g_MainRng, 0, 100)` |
| 7 | `+0x5C` Vegetation | see §5.2 — INI-driven bounds, clamped, then `RandomRanged` |
| 8 | `+0x74` Seed | `RandomRanged(g_MainRng, 0, 0xFFFF)` |

The Tiberium multiply is `LEA EAX,[EAX+EAX*4] ; SHL EAX,2` at `0x00597300`/`0x00597308`,
i.e. `×5 <<2` = `×20` (`0x14`). It is *interleaved* between draws 5 and 6 in program order
but consumes nothing, so it does not affect the stream.

**Draw count is 8 for map types 0–3 and 7 for map type 4.** On Mountainous,
`UrbanMin[4] == UrbanMax[4] == 0`, so draw #3 hits the `min == max` early-out and skips the
generator entirely. A port that unconditionally draws will desynchronise every field after
UrbanPresence on Mountainous maps.

---

## 2. The tables — addresses, layout, literal values

`read_memory 0x0082B080` (180 bytes) covers the whole contiguous `.data` run; the two
all-zero min tables live in `.bss` and were read separately (`read_memory 0x00ABED18`,
64 bytes). Element type **`int` (4 bytes)**, element **count 5**, index = `MapSeed.MapType`
(+0x3C), stride between tables 0x14 = 20 bytes.

Raw hex from `read_memory 0x0082B080` (180 bytes), decoded as 45 little-endian dwords:

```
0082B080  32 00 00 00  00 00 00 00  23 00 00 00  00 00 00 00  00 00 00 00   -> 50,  0, 35,  0,  0
0082B094  64 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00  32 00 00 00   ->100,100,100,100, 50
0082B0A8  4B 00 00 00  00 00 00 00  32 00 00 00  00 00 00 00  00 00 00 00   -> 75,  0, 50,  0,  0
0082B0BC  64 00 00 00  19 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00   ->100, 25,100,100,100
0082B0D0  64 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00  14 00 00 00   ->100,100,100,100, 20
0082B0E4  64 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00   ->100,100,100,100,100
0082B0F8  32 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00  00 00 00 00   -> 50,100,100,100,  0
0082B10C  14 00 00 00  14 00 00 00  14 00 00 00  14 00 00 00  14 00 00 00   -> 20, 20, 20, 20, 20
0082B120  64 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00  64 00 00 00   ->100,100,100,100,100
```

Table assignment, each address taken from the operand of the actual load instruction in
`disassemble_function 0x00597260` (not from the Ghidra label), then cross-checked against
`list_globals name_substring=Rmg`:

| address | table | load instruction | `[0]` Arch | `[1]` Cont | `[2]` TeamC | `[3]` Inland | `[4]` Mtn |
|---|---|---|---|---|---|---|---|
| `0x0082B0A8` | **WaterAmount MIN** | `0x0059726F MOV ECX,[ESI*4+0x82B0A8]` | **75** | **0** | **50** | **0** | **0** |
| `0x0082B0BC` | **WaterAmount MAX** | `0x00597268 MOV EAX,[ESI*4+0x82B0BC]` | **100** | **25** | **100** | **100** | **100** |
| `0x0082B10C` | Ruggedness MIN | `0x0059728C` | 20 | 20 | 20 | 20 | 20 |
| `0x0082B120` | Ruggedness MAX | `0x00597285` | 100 | 100 | 100 | 100 | 100 |
| `0x00ABED40` | UrbanPresence MIN | `0x005972A9` | 0 | 0 | 0 | 0 | 0 |
| `0x0082B0F8` | UrbanPresence MAX | `0x005972A2` | 50 | 100 | 100 | 100 | **0** |
| `0x00ABED18` | Accessibility MIN | `0x005972C6` | 0 | 0 | 0 | 0 | 0 |
| `0x0082B0D0` | Accessibility MAX | `0x005972BF` | 100 | 100 | 100 | 100 | 20 |
| `0x0082B080` | RegionSize MIN | `0x005972E3` | 50 | 0 | 35 | 0 | 0 |
| `0x0082B094` | RegionSize MAX | `0x005972DC` | 100 | 100 | 100 | 100 | 50 |
| `0x0082B0E4` | *(unreferenced)* | — | 100 | 100 | 100 | 100 | 100 |

`min <= max` holds for all 25 stock pairs — a consistency check on the min/max assignment.

### 2.1 The two zero tables are genuinely zero — verified negative result

`g_nRmgAccessibilityMinByMapType` (`0x00ABED18`) and `g_nRmgUrbanPresenceMinByMapType`
(`0x00ABED40`) read as all-zero in the image (`read_memory 0x00ABED18`, 64 bytes) and each has
**exactly one xref in the entire binary — the READ inside this function**
(`get_xrefs_to 0x00ABED18` → `From 005972c6 in MapSeedClass__RandomizeDerivedFields [DATA]`;
`get_xrefs_to 0x00ABED40` → `From 005972a9 … [DATA]`). No writer exists. The adjacent `.bss`
slot at `0x00ABED2C` is unreferenced as well (`get_xrefs_to 0x00ABED2C` → none), consistent
with a run of all-zero arrays parked in `.bss` rather than fields of a live object.

So in the port: `AccessMin[mt] = 0` and `UrbanMin[mt] = 0` for all map types, hardcoded.

### 2.2 The unreferenced table at `0x0082B0E4`

Five ints, all 100, sitting between Accessibility-max and UrbanPresence-max in the table run.
`get_xrefs_to 0x0082B0E4` → **"No references found"**. Nothing in the binary reads or writes
it. **Do not port.** (Labelled `g_nRmgUnreferencedByMapTypeTable` this session.) A guess at
what it once was is in §9.

### 2.3 Index domain: map type 0..4, and what each index means

`g_szRmgMapTypeLabels @ 0x0082B034` is `char*[5]`; `read_memory 0x0082B034` (20 bytes) gives
the pointers `{0x0082B984, 0x0082B970, 0x0082B958, 0x0082B948, 0x0082B934}` and
`read_memory 0x0082B934` (96 bytes) gives the strings:

| index | CSF key | plain name |
|---|---|---|
| 0 | `TXT_MAP_ARCHIPELAGO` | Archipelago |
| 1 | `TXT_MAP_CONTINENT` | Continent |
| 2 | `TXT_MAP_TEAM_CONTINENTS` | Team Continents |
| 3 | `TXT_MAP_INLAND` | Inland |
| 4 | `TXT_MAP_MOUNTAINOUS` | Mountainous |

Three independent corroborations that this ordering is right rather than assumed:

1. `MapSeedClass__ClampFields` clamps `+0x3C` to `0..4` (`decompile_function 0x005975E0`).
2. The generator branches to the *inland/mountain* water seeder for map types 3 and 4
   exactly (§3).
3. The data itself is semantically coherent: Archipelago has the highest water floor (75),
   Continent the lowest ceiling (25), Mountainous has zero urban presence and the lowest
   accessibility ceiling (20) and the lowest region-size ceiling (50).

**No evidence of the tables being indexed by anything other than map type** — the index
register `ESI` is loaded once from the single stack argument and never re-derived
(`disassemble_function 0x00597260`).

---

## 3. Where WaterAmount is consumed (the gate)

Verified independently of the prompt, via `disassemble_bytes 0x00598AB0-0x00598B10`
(inside `RandomMapGenerator__Generate`, 0x00598960):

```
00598ADE  CALL 0x005981F0            ; MapSeedClass__ReadINI(this) — rmg(md).ini reloaded
00598AE3  PUSH 0x0082BEEC            ; "RMG: Seeding water\n"
00598AED  MOV  EAX,[EBP+0x3C]        ; MapType
00598AF3  CMP  EAX,0x3 / JZ  0x00598B06
00598AF8  CMP  EAX,0x4 / JZ  0x00598B06
00598AFD  MOV  ECX,EBP / CALL 0x0059A6C0    ; non-3/4 water path
00598B04  JMP  end
00598B06  CMP  [EBP+0x4C],EBX        ; EBX == 0 → "WaterAmount == 0 ?"
00598B09  JZ   end                   ; ← skip the entire water phase
00598B0B  MOV  ECX,EBP / CALL 0x0059C580   ; map-type-3/4 water seeder
```

So the type-3/4 water seeder is at **`0x0059C580`**, and `WaterAmount == 0` is a total skip,
not a degraded path. `RandomMapGenerator__Generate` itself does **not** call
`RandomizeDerivedFields` (see §4).

---

## 4. Callers — when is the derivation actually run?

`get_xrefs_to 0x00597260` returns **exactly three** call sites:

| site | in | live? |
|---|---|---|
| `0x005967FD` | `RandomMapSetupDialog__Proc` — **Randomize** button `0x621` | live |
| `0x00596E08` | `RandomMapSetupDialog__SyncOptionsFromControls` (0x00596C70) | live |
| `0x005973E7` | `MapSeedClass__RandomizeAllOptions` (0x00597380) | **dead** — that function has zero xrefs (`get_xrefs_to 0x00597380` → "No references found") |

**So it is NOT only the Randomize button.** The second live caller is the far more frequent
one.

### 4.1 `RandomMapSetupDialog__SyncOptionsFromControls` (0x00596C70)

`decompile_function 0x00596C70`. Reads the six exposed controls back into the MapSeed:

| control | field |
|---|---|
| combo `0x405` | `+0x3C` MapType |
| combo `0x407` | `+0x38` Theater |
| combo `0x408` | `+0x40` Resources |
| combo `0x3EA` | `+0x48` TimeOfDay |
| combo `0x406` | `+0x64` Width **and** `+0x68` Height (one control drives both) |
| trackbar `0x3EB` (`TBM_GETPOS`) | `+0x50` NumPlayers |

Any read differing from the stored value sets the dirty byte `DAT_00ABE2D8`. Then:

```
if (playersChanged || dirty) {
    MapSeedClass__RandomizeDerivedFields(this, this->MapType);   // 7–8 draws
    destroy cached preview (DAT_00ABE154);
    dirty = 0;
}
MapSeedClass__ClampFields(this);        // always
```

It is invoked from `RandomMapSetupDialog__Proc` on: `CBN_SELCHANGE` for `0x3EA` and
`0x405..0x408`, `WM_HSCROLL` on `0x3EB`, and at the top of the **Generate `0x620`**,
**Load `0x6C2`**, **Save `0x6C3`**, **Delete `0x6C4`** and **OK `0x6C5`** handlers
(`decompile_function 0x00596300`).

### 4.2 Generate re-rolls by default — except right after a Load

The Generate `0x620` handler does **not** simply force a re-roll. It consults a latch:

```
bVar14 = (DAT_0082B030 == 0);
if (bVar14) DAT_0082B030 = 1;
DAT_00ABE2D8 = !bVar14;                    // dirty
FUN_00596C70(hDlg);                        // → may re-roll
```

`DAT_0082B030` is set to `1` at `WM_INITDIALOG`, so a normal Generate arrives with
`DAT_0082B030 != 0` → `dirty = 1` → **derived fields are re-rolled before every generate.**

The **Load `0x6C2`** handler is the one exception:

```
case 0x6C2:  FUN_00596C70(hDlg);
             if (LoadSaveDialog(...)) {        // a saved seed was chosen
                 DAT_0082B030 = 0;             // ← clear the latch
                 PostMessage(hDlg, WM_COMMAND, 0x620, …);   // queue a Generate
             }
             …
             RandomMapSetupDialog__SyncControlsFromOptions(hDlg);   // controls ← loaded seed
```

`PostMessage` is asynchronous, so the controls are re-synced from the loaded seed *before*
the queued Generate is dispatched. When it is dispatched: `DAT_0082B030 == 0` →
`dirty = 0`, and every control read inside `SyncOptionsFromControls` matches the stored
value → **`RandomizeDerivedFields` is skipped and the loaded WaterAmount survives verbatim.**

### 4.3 The `.SED` does store WaterAmount

The seed file is an INI with a `[RandomMap]` section (`"RandomMap"` @ `0x0082BB24`,
`"WaterAmount"` @ `0x0082BB94`). `get_xrefs_to 0x0082BB94` gives exactly two sites, a writer
and a reader; both are in code Ghidra has not wrapped in a function, so they were read with
`disassemble_bytes`:

* **Writer** (`disassemble_bytes 0x005978F0-0x00597960`):
  `0x00597915 MOV EAX,[ESI+0x4C]` → `PUSH EBX / PUSH EAX / PUSH "WaterAmount" / PUSH "RandomMap" / CALL 0x005275C0` (INI WriteInt).
* **Reader** (`disassemble_bytes 0x00597BF0-0x00597C60`):
  `0x00597C12 MOV ECX,[ESI+0x4C]` (used as the *default*) → `PUSH ECX / PUSH "WaterAmount" / PUSH "RandomMap" / CALL 0x005276D0` (INI ReadInt) → `0x00597C3E MOV [ESI+0x4C],EAX`.

The same two blocks round-trip `+0x44` Ruggedness, `+0x54` Tiberium (`"Tiberium"` @
`0x00817278`), `+0x58` TiberiumLayout, `+0x5C` Vegetation, `+0x6C` Accessibility,
`+0x70` RegionSize — i.e. every derived field. `ChooseMap__AcceptRandomMapSetup`
(`0x005E8590`) writes `RandMap.Sed` on accept (`decompile_function 0x005E8590`).

**Answer to the critical sub-question:** yes — a `.SED` persists WaterAmount, and the load
path deliberately suppresses the derivation so the stored value is used. A port must
(a) derive on the dialog/randomize paths, and (b) read WaterAmount from the `.SED` and
**not** re-derive on the load path.

---

## 5. The other fields the same routine rolls

| field | offset | source | port already needs? |
|---|---|---|---|
| WaterAmount | `+0x4C` | tables `0x0082B0A8` / `0x0082B0BC` | **gates map-type-3/4 water phase (0x0059C580) and, per the mode-3/4 report, the river width and field quota** |
| Ruggedness | `+0x44` | tables `0x0082B10C` / `0x0082B120` (all 20..100) | terrain shaping |
| UrbanPresence | `+0x60` | `0x00ABED40` (zeros) / `0x0082B0F8` | urban/city placement |
| Accessibility | `+0x6C` | `0x00ABED18` (zeros) / `0x0082B0D0` | passability/connectivity |
| RegionSize | `+0x70` | `0x0082B080` / `0x0082B094` | region partition (`0x0058CF90`) |
| Tiberium | `+0x54` | `Resources * 20`, **no draw** | ore/gem quota |
| TiberiumLayout | `+0x58` | `RandomRanged(0,100)`, no table | ore field layout |
| Vegetation | `+0x5C` | rmg(md).ini, see §5.2 | trees/props |
| Seed | `+0x74` | `RandomRanged(0,0xFFFF)` | master RMG seed |

### 5.1 Tiberium is computed, and its clamp floor is 1

`+0x54 = Resources(+0x40) * 20`, and `Resources` is a 0..3 combo, so the raw values are
`{0, 20, 40, 60}`. `MapSeedClass__ClampFields` (`decompile_function 0x005975E0`) clamps
`+0x54` to **1..100**, floor **1 not 0** — so `Resources == 0` yields `Tiberium == 1`, not 0.
Every other clamped field has floor 0 except NumPlayers (2..8) and MapType (0..4).

### 5.2 Vegetation is INI-driven, indexed by map type

`0x00597316`/`0x0059731C` load *pointers* (`MOV ECX,[0x00ABE260]` / `MOV EDX,[0x00ABE27C]`)
and index them by `mapType*4`. Those two globals are `g_MapSeed + 0x288` and
`g_MapSeed + 0x2A4` — the `VectorClass` data pointers of the two vectors that
`MapSeedClass__ReadINI` (`0x005981F0`) fills from `rmg(md).ini` `[General]`
(`decompile_function 0x005981F0`; the vector objects are at `this+0x284` and `this+0x2A0`,
`get_xrefs_to 0x0082BD74` / `0x0082BD5C` land in that function).

Logic:

```
lo = RMGVegetationMinimums[mt];  hi = RMGVegetationMaximums[mt];
lo = clamp(lo, 0, 100);  hi = clamp(hi, 0, 100);
if (hi < lo) lo = hi;                      // note: min is lowered to max, not vice versa
Vegetation = RandomRanged(g_MainRng, lo, hi);
```

Stock values (in-repo `ini/rmgmd.ini` and `ini/rmg.ini`, identical in both):

```
RMGVegetationMinimums=60,60,60,60,60
RMGVegetationMaximums=100,100,100,100,100
```

so in stock YR this is always `RandomRanged(60, 100)` — but it must stay INI-driven.

`MapSeedClass__ReadINI` runs before the dialog opens (`0x00595BCA MOV ECX,0xABDFD8` then
`CALL 0x005981F0`, `get_assembly_context 0x00595BC5`) **and** again at the top of every
generate (`0x00598ADE`), so the pointers are never null when the derivation reads them.

---

## 6. Gating / reachability (Tiberian Sun ghost check)

* **Not flag-gated.** `MapSeedClass__RandomizeDerivedFields` has no conditional guard of any
  kind — no `SpecialFlags`, no difficulty, no `g_IsMapEditor` test
  (`disassemble_function 0x00597260`, straight-line code, 0x118 bytes, no branches except the
  vegetation clamps).
* **Not map-editor-only.** The reverse is true: `RandomMapSetupDialog__Proc`'s `WM_INITDIALOG`
  enables the Generate button `0x620` only when `g_IsMapEditor == 0`, and the
  `Save_Scenario_Map_File` branch under OK `0x6C5` is the map-editor-only path
  (`decompile_function 0x00596300`).
* **Reachable in a stock retail build.** Chain:
  `[call site 0x005E6A11, inside the multiplayer game-options / Choose-Map screen]` →
  `ChooseMap__AcceptRandomMapSetup` (`0x005E8590`) → `FUN_00595BC0` (RMG dialog runner —
  calls `MapSeedClass__ReadINI`, runs dialog resource `0x105`, returns 1 on OK / 2 on Cancel)
  → `RandomMapSetupDialog__Proc` (`0x00596300`). The `RandMap.Sed` string is also consumed by
  `MPGameOptions__ParsePacket`, `SessionClass__VerifyRandomMapDigest` and
  `MPGameOptions__GetScenarioPlayerCount` (`get_xrefs_to 0x0082BC30`), i.e. the random map is
  a first-class multiplayer map selection, not editor-only scaffolding.
* **No TS legacy detected in this routine.** Every field it writes is read back by live YR
  RMG code, and every table it reads is in the live `.data`/`.bss` run — with the single
  exception of `0x0082B0E4`, which is dead in the strict sense (zero xrefs) and must simply
  not be ported.
* **One dead sibling:** `MapSeedClass__RandomizeAllOptions` (`0x00597380`) has zero xrefs. It
  is an out-of-line twin of the inlined Randomize `0x621` handler. Port the inlined handler,
  not this.

---

## 7. Randomize-button draw order (context for stream parity)

From `decompile_function 0x00596300`, the `0x621` handler, all on `g_MainRng`:

```
FUN_00596C70(hDlg);                                   // sync (may itself re-roll derived!)
SyncControlsFromOptions(hDlg);
Theater(+0x38)   = (RandomRanged(0,100) > 0x31);      // boolean-ised, 0 or 1
MapType(+0x3C)   = RandomRanged(1,4);                 // ← never 0: Randomize can never
                                                      //   produce Archipelago
TimeOfDay(+0x48) = RandomRanged(0,3);                 // ← drawn BEFORE Resources
Resources(+0x40) = RandomRanged(0,3);
Width(+0x64) = Height(+0x68) = RandomRanged(0,3);     // one draw feeds both
MapSeedClass__RandomizeDerivedFields(&g_MapSeed, MapType);   // 7–8 draws
Description(+0x78) = string 0xF5E;
Seed(+0x74)      = RandomRanged(0,0xFFFF);            // SECOND seed draw, supersedes draw #8
MapSeedClass__ClampFields(&g_MapSeed);
```

---

## 8. Label drift found this session

* The plate comment already present on `0x00597260` (dated 2026-07-21) was **substantially
  correct** on table addresses, field offsets and draw order — all re-verified from assembly
  here. Two gaps corrected and written back: it did not record the `min == max` no-draw
  early-out (which changes the draw count from 8 to 7 on Mountainous), and it named only the
  Randomize button as caller, omitting the far more frequent
  `SyncOptionsFromControls` path and not marking `0x00597380` as dead.
* `FUN_00596C70` and `FUN_00597380` were unnamed; renamed this session (§10).
* No wrong labels found on the RMG range-table globals: every `g_nRmg*ByMapType` name matched
  the address the assembly actually loads (checked operand-by-operand against
  `list_globals name_substring=Rmg`).

---

## 9. Unverified / residual risk (YELLOW — do not cite as fact)

1. **`Random__RandomRanged` guard byte at `this+0x00`.** When non-zero the function returns
   `min` without drawing. I did not establish what writes it or whether it is ever non-zero in
   a normal session. Assumed 0 throughout; **unproven**.
2. **Absence of an indirect writer to `0x00ABED18` / `0x00ABED40`.** Ghidra xref analysis only
   catches direct-address operands. A write through a computed pointer or a wide `memset`
   would not appear. None was found and the neighbouring slots are unreferenced, but "no
   writer exists" is HIGH-confidence, not proof.
3. **Origin of the unreferenced table at `0x0082B0E4`.** All 100s, zero xrefs — that much is
   verified. My guess that it is a leftover max-table for TiberiumLayout or Vegetation (both
   of which now get their bounds from a literal `0,100` and from the INI respectively) is
   **inference only**.
4. **The exact UI command that opens the RMG dialog.** The call site `0x005E6A11` is inside a
   region Ghidra has not defined as a function, so I could not name the control ID. An
   existing plate comment claims command `0x583`; **I did not verify that.**
5. **Whether `g_MainRng` is in a deterministic state at RMG time in multiplayer.** The RMG
   dialog runs pre-match, and `g_MainRng` is the *unsynchronised* stream shared with combat
   effects/animations. Reproducing gamemd's exact WaterAmount for a given wall-clock session
   would require reproducing every prior `g_MainRng` consumer. **This is not a blocker for the
   port** (the RMG only needs to be statistically and structurally identical, and `.SED`
   round-trip must be exact), but it does mean a bit-exact "same button press → same map"
   comparison against gamemd is not achievable from the derivation contract alone.
6. **River width / field quota consumption of WaterAmount.** The prompt states
   `wMax = ftol(max(WaterAmount * 0.07, 1.0))` inside the type-3/4 seeder. That is owned by
   `RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md` and was **not** re-verified here; only the
   `MapType ∈ {3,4} && WaterAmount != 0 → CALL 0x0059C580` gate was.

---

## 10. Ghidra annotations applied this session

Not saved — the coordinator owns `save_program`.

| address | action |
|---|---|
| `0x00597260` | plate comment rewritten (full contract, RNG identity, no-draw early-out, all three callers) |
| `0x00596C70` | renamed `RandomMapSetupDialog__SyncOptionsFromControls` + plate (control→field map, re-roll condition, load-path latch) |
| `0x00597380` | renamed `MapSeedClass__RandomizeAllOptions` + plate (**dead code**, zero xrefs) |
| `0x0082B0A8` | plate: WaterAmount MIN literals + map-type meaning + gate consequence |
| `0x0082B0BC` | plate: WaterAmount MAX literals + inclusive-bound semantics |
| `0x00ABED18` | plate: all-zero, **no writer** (negative result) |
| `0x00ABED40` | plate: all-zero, no writer, + the Mountainous 7-draw stream warning |
| `0x0082B034` | plate: map-type index → CSF label table, with the three corroborations |
| `0x0082B0E4` | `set_global` → `g_nRmgUnreferencedByMapTypeTable`, `int[5]`, plate: **zero xrefs, do not port** |
