# Layer Enum (Ground / Surface / Underground) — Consumer Identification

**Date:** 2026-05-18
**Status of finding:** **RESOLVED — the strings are the same LayerClass display-sort enum, used as art.ini `[AnimType] Layer=` INI parser values.**
**Active in YR:** **Yes** — 423 `Layer=` keys in `artmd.ini` (323 in `art.ini`) parse through this consumer every time art INI loads.
**Verdict:** **NOT TS-legacy dead data.** Live YR consumer with broad usage.

This report closes the open question §6 + §8 Q4 of
`LAYER_SYSTEM_GHIDRA_REPORT.md` ("identify the consumer of the
Ground/Surface/Underground strings at `0x0081DB84`").

---

## 1. Headline

The strings at `0x0081DB84..0x0081DB94` are **not a separate cell-feature
enum**. They are entries 2, 1, 0 of `g_LayerNameTable @ 0x0081da78` —
the canonical layer-index ↔ name table for the 5-layer LayerClass
display-sort system documented in `LAYER_CLASS_GHIDRA_REPORT.md`.

The original "open question" in `LAYER_SYSTEM_GHIDRA_REPORT.md` arose
because that doc inspected the string region directly without first
following the indirect pointer table immediately above it. The strings
look like a separate enum, but they're just the targets of the existing
LayerClass name table.

The consumer is **`CCINIClass::ReadLayer @ 0x00477050`**, called from
**`AnimTypeClass::ReadINI @ 0x00427d00`** to parse the `Layer=` key in
art.ini `[AnimType]` sections. The parsed value is stored at
`AnimTypeClass+0x364` (verified by existing `ANIM_CLASS_GHIDRA_REPORT.md`
§ "Layer Enum") and read back at draw time by `AnimClass::GetLayer @
0x00424cb0` (vtable+0x78) to select which `g_DisplayLayers[i]` bucket
the animation renders in.

---

## 2. The string table at `0x0081da78` — pointer table

Read via `read_memory 0x0081da78 length=24`:

| Pointer Address | Target Address | String | Layer Index |
|-----------------|----------------|--------|-------------|
| `0x0081da78` | `0x0081db94` | "Underground" | **0** |
| `0x0081da7c` | `0x0081db8c` | "Surface"     | **1** |
| `0x0081da80` | `0x0081db84` | "Ground"      | **2** |
| `0x0081da84` | `0x0081758c` | "Air"         | **3** |
| `0x0081da88` | `0x0081db80` | "Top"         | **4** |

Five entries, 4 bytes each = 20 bytes; loop bound `< 0x81da8c` confirmed
by `Layer_From_Name` decompilation (`0x0048e063`).

**Why three strings looked stand-alone:** they sit in a contiguous
descending-pointer block at `0x0081db80..0x0081db94` (Top, Ground,
Surface, Underground). The earlier doc only spotted the three with
visible string preview; "Air" (`0x0081758c`) sits in a different
read-only data region (a separate string used by direction tables too),
and "Top" (`0x0081db80`) was off by one from the inspected window.

C/I/B = HIGH (memory verified, decompilation verified).

---

## 3. The reader and writer

### `Layer_From_Name @ 0x0048e063` (string → index)

```c
int __cdecl Layer_From_Name(char *name) {
    if (in_ECX != 0) {                 // ECX = name
        iVar3 = 0;
        ppuVar2 = &g_LayerNameTable;
        do {
            if (FUN_007c8d20(*ppuVar2) == 0)  // _stricmp(table[i], name)
                return iVar3;
            ppuVar2++;
            iVar3++;
        } while ((int)ppuVar2 < 0x81da8c);
    }
    return -1;
}
```

Case-insensitive (`_stricmp`-equivalent at `0x007c8d20`). Order of
comparison matches the table → "Underground"=0, "Surface"=1, "Ground"=2,
"Air"=3, "Top"=4.

### `Layer_To_Name @ 0x0048e095` (index → string)

```c
char *__cdecl Layer_To_Name(uint index) {
    if (index < 5) return (&g_LayerNameTable)[index];
    return &DAT_00817474;               // fallback (likely "")
}
```

### `CCINIClass::ReadLayer @ 0x00477050` (INI key reader)

```c
int CCINIClass__ReadLayer(section, key, default_value) {
    char buf[128];
    char *default_name = Layer_To_Name(default_value);
    if (CCINIClass__ReadString(section, key, default_name, buf, 0x80) != 0)
        return Layer_From_Name(buf);
    return default_value;
}
```

Round-trips the default through `Layer_To_Name` so the INI read fallback
preserves the original 0-based index.

---

## 4. Live YR consumer — AnimType.Layer

### Call site

`AnimTypeClass::ReadINI @ 0x00427d00` calls:

```c
iVar4 = CCINIClass__ReadLayer(piVar8, s_Layer_00818644, param_1[0xd9]);
param_1[0xd9] = iVar4;
```

`param_1` is `int *` (per the ReadINI plate comment), so byte offset =
`0xd9 * 4 = 0x364`. Stored at `AnimTypeClass+0x364`. Field documented
independently in `ANIM_CLASS_GHIDRA_REPORT.md` line 159, and in
`BULLETTYPECLASS_GHIDRA_REPORT.md` line 381's AnimTypeClass plate.

### INI key surface (verified in repo)

```
ini/artmd.ini : 423 occurrences of "Layer=..."
ini/art.ini   : 323 occurrences of "Layer=..."
```

Unique values appearing in stock YR INIs (case-insensitive — values
match `Layer_From_Name`):

| INI value     | Parses to | Notes |
|---------------|-----------|-------|
| `Layer=ground`   | 2 | dominant; default; covers explosions, smoke, damage anims |
| `Layer=surface`  | 1 | annotated `; SJM: Lower than ground -- go under ships` in artmd.ini |
| `Layer=Top` / `Layer=top` | 4 | weather clouds (WCCLOUD1/2/3), cruise-altitude effects |

**`Layer=Underground` and `Layer=Air` are NOT used by stock YR art INIs**
— but the parser accepts them. `Underground=0` is reachable in principle
but no shipped AnimType uses it; `Air=3` is the runtime default for
ownerless typeless anims (see §5).

### The renderer consumer (already documented elsewhere)

`AnimClass::GetLayer @ 0x00424cb0` (vtable+0x78):

```c
int AnimClass__GetLayer(AnimClass* this) {
    if (this->field_0xCC != 0) return 2;    // attached to owner → Ground
    if (this->AnimType != NULL)
        return this->AnimType->Layer;       // AnimType+0x364
    return 3;                                // default: Air
}
```

The returned value indexes `g_DisplayLayers[]` (the 5 LayerClass
instances at `0x008A0360`). So `Layer=surface` puts the anim in
`g_DisplayLayers[1]`, drawn before Ground — under ships, exactly as the
INI comment promises.

---

## 5. TS-vs-YR classification

**Live in YR. Not TS-legacy.** Evidence:

1. **Real INI surface in stock YR data:** 423 `Layer=` keys in
   `artmd.ini`, 323 in `art.ini`. The shipped game data exercises this
   parser on every art INI load.
2. **Live render-path consumer:** `AnimClass::GetLayer` is on the
   per-tick render path (`DisplayClass::Tick` re-buckets every visible
   anim into `g_DisplayLayers[layer]` and then draws each bucket
   sorted by Z). Every animation drawn this frame went through this
   field.
3. **Observable player-visible effect:** `Layer=surface` makes
   anims render UNDER ships (see WAKE/wake anim comment in artmd.ini);
   `Layer=Top` makes weather clouds (WCCLOUD*) render above everything;
   `Layer=ground` is the default Y-sorted ground bucket. All three are
   visible in normal play.
4. **Cross-doc confirmation:** `LAYER_CLASS_GHIDRA_REPORT.md`
   §"Important" lines 137–143 explicitly resolves the same table to the
   same layer indices, and lines 197–207 enumerate the active-in-YR
   semantics of each of the 5 layers.

**Underground=0 caveat:** Index 0 ("Underground") is parseable and the
machinery exists, but **no stock YR AnimType uses it**, and no live YR
locomotor returns it from `In_Which_Layer` either (per
`LAYER_SYSTEM_GHIDRA_REPORT.md` §5). So `g_DisplayLayers[0]` is
effectively empty in standard play. This is the TS-Tunnel-locomotor
residue, but the *string and parser* are still live YR code paths — the
string just happens to never match anything in stock data.

---

## 6. Other xrefs to `0x0081DB84` — clarification

`get_xrefs_to 0x0081DB84` reports three additional sites that are
**unrelated to the layer enum**:

| From | Function | What it is |
|------|----------|-----------|
| `0x0068a91f` | `ScenarioClass::Read_INI_Basic` | Reads `[Lighting] Ground=` key (a brightness value into `ScenarioClass+0x3540`). Coincidental string reuse — same `"Ground"` literal serves as both the layer name AND the lighting INI key. |
| `0x0068b698` | `FUN_0068ad70` (scenario writer) | Writes the matching `[Lighting] Ground=` value out. |
| `0x005998fa` | `CCINIClass::Constructor` | Initial-default of the `[Lighting] Ground=` field when constructing a fresh INI. |

These are the **`[Lighting] Ground=` map key**, not the layer enum. The
proximity of the same literal to two unrelated uses is a string-pool
deduplication artifact (the compiler shared one `"Ground"` literal
across the layer table and the `[Lighting]` key parser).

C/I = HIGH for the lighting-key claim (decompilation directly shows
`s_Ground_0081db84` used as `key_name` argument to `CCINIClass__ReadDouble`
at `0x68a92e`).

The `0x0081da80` xref reported by `get_xrefs_to 0x0081DB84` is the
pointer-table entry that resolves to `0x0081db84` — i.e., the layer-table
slot for index 2 (Ground).

---

## 7. Open Questions

These follow-ups are **NOT in scope** for this slot. Listed for the
parent reconciliation pass:

1. **`Underground`-as-INI-string is parseable but unused** — verify no
   modder or hidden art override exercises it. Likely safe to treat as
   "supported value, no current consumer" in the Rust impl.
2. **Hover/Teleport/Rocket locomotor In_Which_Layer slots** — still
   open from the original Layer System doc §8 Q1–Q3, unrelated to this
   string question.
3. The fallback string `DAT_00817474` returned by `Layer_To_Name` for
   out-of-range indices was not verified this pass.

---

## 8. Verified facts (≤5)

1. `g_LayerNameTable @ 0x0081da78` holds 5 string-pointers: Underground(0),
   Surface(1), Ground(2), Air(3), Top(4) — confirmed by `read_memory
   0x0081da78 len=24` and `Layer_To_Name` body.
2. `Layer_From_Name @ 0x0048e063` is the string→index lookup (case-
   insensitive via `_stricmp`-equivalent `FUN_007c8d20`), bounded
   `< 0x81da8c` (5 entries).
3. The only live caller of `CCINIClass::ReadLayer @ 0x00477050` is
   `AnimTypeClass::ReadINI @ 0x00427d00`, which parses the `[AnimType]
   Layer=` INI key into `AnimTypeClass+0x364` (`param_1[0xd9]`).
4. `ini/artmd.ini` contains **423** `Layer=` lines (`ini/art.ini` 323),
   covering values `ground`, `surface`, `Top`/`top` — confirming the
   consumer is exercised heavily on every YR art load.
5. The other xrefs to `0x0081DB84` (from `ScenarioClass::Read_INI_Basic`
   and CCINI default-init) are the **`[Lighting] Ground=` map key**,
   not the layer enum — string-pool reuse of the same `"Ground"`
   literal, verified by decompilation at `0x68a92e`.

---

## 9. Sources

**Ghidra functions decompiled (this pass):**
- `Layer_From_Name @ 0x0048e063`
- `Layer_To_Name @ 0x0048e095`
- `CCINIClass::ReadLayer @ 0x00477050`
- `FUN_004770b0` (the `Layer_To_Name`-using helper, likely
  `CCINIClass::WriteLayer`)
- `AnimTypeClass::ReadINI @ 0x00427d00` (verified `Layer=` key wiring)
- `ScenarioClass::Read_INI_Basic @ 0x00689e90` (verified `[Lighting]
  Ground=` is the unrelated coincidental use)
- `FUN_0068ad70` (scenario writer, also `[Lighting] Ground=`)
- `CCINIClass::Constructor` (verified default-init of `[Lighting] Ground=`)
- `SpeedType::FromName @ 0x0048e014` (scan bound `< 0x81da78` —
  confirms the layer table is the upper neighbour of SpeedType table)

**Memory reads:**
- `0x0081da78` len 24 — pointer table (5 entries)
- `0x0081db80` len 4 — "Top"
- `0x0081758c` len 32 — "Air" + adjacent direction strings

**Xrefs traversed:**
- `get_xrefs_to 0x0081DB84` → 4 (1 table-pointer, 3 [Lighting] uses)
- `get_xrefs_to 0x0081DB8C` → 2 (table-pointer + Layer_From_Name body)
- `get_xrefs_to 0x0081DB94` → 2 (table-pointer + Layer_From_Name body)
- `get_xrefs_to 0x0081da78` → 4 (Layer_From_Name, Layer_To_Name,
  SpeedType_FromName, AnimType writer FUN_004770b0)
- `get_function_callers Layer_To_Name` → CCINIClass__ReadLayer,
  FUN_004770b0
- `get_function_callers CCINIClass__ReadLayer` →
  AnimTypeClass__ReadINI

**Companion docs (already authoritative, not modified):**
- `LAYER_CLASS_GHIDRA_REPORT.md` — full LayerClass display-sort doc;
  lines 120–143 already describe `g_LayerNameTable` and the 0-based
  index encoding.
- `ANIM_CLASS_GHIDRA_REPORT.md` line 159 — independent record of the
  field at `AnimTypeClass+0x364` and consumer `CCINIClass__ReadLayer`.
- `LAYER_SYSTEM_GHIDRA_REPORT.md` — the original "open question" doc;
  §6 + §8 Q4 are now resolved by this report.
- `BULLETTYPECLASS_GHIDRA_REPORT.md` line 381 — plate comment with
  AnimType layout confirming `Layer(0x364)`.
- `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` line 593, 602 — independent
  cite of `AnimType.Layer` and `AnimClass::GetLayer`.

---

*End of report. The "mystery" strings were never an independent enum —
they're the lower half of an existing 5-entry layer name table, used as
the parser surface for art.ini `Layer=` keys. Active in YR, used by
~423 art entries, drives display-sort layer assignment for every
animation rendered.*
