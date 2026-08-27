# Phase 3 `LastTilesInSet` compatibility translation research

**Verdict:** COMPLETE / HIGH confidence.

**Active in YR:** conditional. The loader and translator are live active-retail
code, but the installed stock theater data makes the exception table empty.

**Player impact:** high whenever triggered: affected ground cells can resolve
the wrong TMP and then feed wrong variant, LAT, terrain-property, bridge, and
rendering behavior. Trigger frequency is zero in installed stock data because
all six active YR theater INIs contain zero active `LastTilesInSet=` keys.

**Scope:** exception-table construction, IsoMap ingress translation,
persistence, immediate consumers, and evidence-backed exclusions only.

## Native owners and lifecycle

`Read_Theater_TileSets_INI @ 0x00545150` constructs the compatibility table
while reading `%sMD.INI` and `[TileSet%04d]`.

Verified direct callers:

| Caller | Callsite | Behavior |
|---|---:|---|
| `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70` | `0x004AD048` | ordinary scenario map load |
| `RandomMapGenerator__InitMapFromSyntheticINI @ 0x00599650` | `0x0059A1E9` | RMG theater initialization |
| `MouseClass__Load @ 0x005BDF70` | `0x005BE647` | save-game restoration |

Each caller compares the requested theater with cached theater
`DAT_00822CF8`. If equal, it calls `FUN_00547110` instead and does not rebuild
or clear the compatibility table. The table therefore persists across
same-theater reuse. All three loader calls pass `DL=1`.

At a real rebuild, `0x0054526B..0x005452B1` deletes every prior exception
record back-to-front and reduces the active count to zero while retaining the
vector backing allocation/capacity.

### Static vector layout

Static initialization at `0x00543E70..0x00543EA1` proves that
`DAT_00AA1078` is a `DynamicVectorClass` object:

| Address / offset | Exact role |
|---|---|
| `DAT_00AA1078 + 0x00` | vtable `0x007ECBDC` |
| `DAT_00AA107C + 0x04` | backing array of `LegacyException*` pointers |
| `DAT_00AA1080 + 0x08` | pointer-array capacity |
| `DAT_00AA1084 + 0x0C` | vector-valid byte, initialized `1` |
| `DAT_00AA1085 + 0x0D` | owns-backing-storage byte |
| `DAT_00AA1088 + 0x10` | signed active count |
| `DAT_00AA108C + 0x14` | capacity growth step, exactly `10` |

The backing array is not a flat array of eight-byte records. Each four-byte
pointer targets one separately allocated record:

```text
LegacyException +0x00: signed i32 legacy_boundary
LegacyException +0x04: signed i32 delta
```

The resize body begins at `0x0054A630`, although Ghidra has no function
boundary there. It allocates `new_capacity * 4`, copies the old pointer slots,
frees owned old storage, and returns `AL=1`; allocation failure returns `AL=0`.
Ordinary capacity progresses `0 -> 10 -> 20 -> ...`; there is no semantic
fixed entry cap.

## Exact INI parser and exception formula

Section ordinal starts at zero and advances by one. `TileSet%04d` uses a
minimum width, not a 10,000-section limit.

`TilesInSet`:

- is read through `CCINIClass__ReadInt @ 0x005276D0`;
- uses key string `0x0082929C`;
- has signed default `-1`, proven before `0x00545FD4`;
- terminates only on exact signed `-1`;
- therefore terminates on a missing section, missing key, or absent value;
- converts present malformed text through CRT `atoi` to zero;
- accepts `$xx` and `xxh`/`xxH` hexadecimal forms;
- does not terminate for other negative values;
- has no native `0..10000` loop cap.

`LastTilesInSet` uses key string `0x0082928C`, is read only after a
non-terminating `TilesInSet`, has signed default `-1` at `0x00545FF7`, and uses
the same malformed/hexadecimal parsing behavior.

Native maintains separate wrapping signed 32-bit actual-registry and legacy
cursors. For each non-terminating set with `T = TilesInSet` and
`L = LastTilesInSet`:

```text
if L != -1 and L != T:
    boundary = wrapping_i32(legacy_cursor + L)
    delta    = wrapping_i32(T - L)
    append { boundary, delta }
    legacy_cursor = boundary
else:
    legacy_cursor = wrapping_i32(legacy_cursor + T)
```

Evidence is `0x00545FFE..0x0054602D` and
`0x0054608C..0x0054609A`: `ADD ECX,EBX` creates the boundary,
`SUB EDX,EBX` creates the delta, and the mismatch/non-mismatch paths publish
the corresponding next legacy cursor.

Entries append in declaration order. There is no validation, sorting,
deduplication, clamping, or positivity gate. `LastTilesInSet=-1` and equality
with `TilesInSet` suppress an entry. Positive `TilesInSet` runs the tile-object
loop and advances the actual cursor by `T`. Zero or negative values other than
`-1` create no tile objects, but legacy-cursor arithmetic still occurs. Missing
TMP files do not remove registry slots. Every add/subtract wraps as x86 `i32`.

The pointer vector grows by ten slots when full. A null eight-byte record
allocation is immediately dereferenced and crashes. If pointer-array growth
fails, the record leaks and is not appended, but the legacy cursor still
advances. These OOM/invalid-input failures are excluded from deliberate Rust
emulation.

The loop publishes the candidate tileset start and increments
`DAT_00ABC558` before reading `TilesInSet`; consequently the terminating
ordinal leaves one candidate start entry. No exception/translation consumer
uses this sentinel bookkeeping, so this slice need not manufacture a fake
Rust tileset.

## Exact transform

`CalculateLegacyMapTileIndex @ 0x00544E30`, body
`0x00544E30..0x00544E68`, is:

```rust
fn translate(raw: i32, entries: &[LegacyException]) -> i32 {
    if raw == 0x0000_FFFF {
        return 0x0000_FFFF;
    }
    let mut result = raw;
    for entry in entries {
        if entry.boundary > raw {
            break;
        }
        result = result.wrapping_add(entry.delta);
    }
    result
}
```

The sentinel is positive `65535`; `-1` is not special. Input/output are full
signed 32-bit values. Count `<= 0` is identity. Comparisons are signed against
the original raw input, equality applies the entry, adds wrap, and the first
boundary strictly greater than raw stops the walk. Nonmonotonic malicious
tables therefore retain declaration order and can hide later entries.

Example:

```text
TileSet0000: TilesInSet=3, Last absent
TileSet0001: TilesInSet=5, LastTilesInSet=2 -> { boundary=5, delta=+3 }
TileSet0002: TilesInSet=4, LastTilesInSet=6 -> { boundary=11, delta=-2 }
```

| Raw | Result |
|---:|---:|
| 4 | 4 |
| 5 | 8 |
| 10 | 13 |
| 11 | 12 |
| 65535 | 65535 |

## Every transform caller and ingress order

The helper has exactly five callers:

| Decoder | Callsite | Input behavior |
|---|---:|---|
| `[IsoMapPack]` `FUN_0056B5A0` | `0x0056B636` | prewrites dword `0xFFFF`, then reads a zero-extended two-byte raw index |
| `[IsoMapPack2]` `FUN_0056B780` | `0x0056B828` | full four-byte raw index |
| `[IsoMapPack3]` `FUN_0056B8A0` | `0x0056B92B` | full four-byte raw index |
| `[IsoMapPack4]` `FUN_0056B9A0` | `0x0056BA48` | full four-byte raw index |
| `ReadIsoMapPack5 @ 0x0056BAC0` | `0x0056BB68` | full four-byte raw index |

`Read_Map_Section_And_IsoMapPacks` attempts those sections in order at
`0x004AD422`, `0x004AD4D5`, `0x004AD588`, `0x004AD63B`, and `0x004AD6E6`.
Multiple present sections are processed in that order.

For each valid Pack5 record native reads signed `X:i16` and unsigned `Y:u16`,
computes `Y*512+X`, validates `[0,0x3FFFF]` plus non-null destination, prewrites
`Cell+0x38=0xFFFF`, reads the raw dword, translates it, overwrites `+0x38`, then
reads subtile `+0x11A`, absolute level `+0x11B`, and ice growth `+0x119`.
Pack2-5 invalid/null destinations consume remaining bytes without translating;
Pack1's fixed traversal can use the shared dummy and still translate.

There is no registry-length check before translation/storage. Later
`ProcessTileVariantsAndCullUnusedTMPs @ 0x00546DA0` treats exact `0xFFFF` and
signed `tile_count <= tile` as invalid, but hostile negative non-`-1` values
can index before the registry.

Variant processing runs at `0x004AD743` after every pack reader. Full Init
returns from map load at `0x006879FF` and later calls
`CellClass::RecalcAttributes @ 0x0047D2B0` from `0x00687A5A`. Translated IDs
are therefore authoritative before variant selection, LAT/slope work, land
derivation, and every later consumer.

## Evidence-backed exclusions

- **Fill:** pre-IsoMap Fill writes cached actual registry IDs from ClearTile or
  WaterSet and never calls the translator. It occurs before current-theater
  table publication. A numerically matching Fill ID must remain unchanged.
- **Runtime Mark/mutation:** `IsometricTileClass::Mark @ 0x00543330` and direct
  runtime tile mutations already receive actual registry IDs and never call
  the translator. Translating them would double-translate live state.
- **Pack1-4 installed data:** all 386 recognized installed map payloads contain
  exactly `[IsoMapPack5]`; Pack1-4 counts are zero. Those callers remain live
  compatibility code but are absent from the installed corpus.
- **Unsafe failures:** native allocator crashes/leaks, negative-index memory
  access, and other hostile-data corruption are invalid-domain behaviors, not
  safe Rust parity requirements.

The physical map census covered 17 `MAPS01.MIX`, 17 `MAPS02.MIX`, 14
`mapsmd03.mix`, 97 `MULTI.MIX`, 173 `multimd.mix`, 13 `expandmd01.mix`, 53
top-level `.mmx`/`.yro` packages, and two loose maps.

## Save/load/replay implications

`MouseClass__Save @ 0x005BE6D0` saves allocated cells through OLE.
`CellClass__Save @ 0x00483C10` reaches `AbstractClass__Save @ 0x00410320`, whose
raw receiver block includes translated `Cell+0x38`. Load restores that block
through `CellClass__Load @ 0x004839F0` and `AbstractClass__Load @ 0x00410380`
before pointer swizzles/map attachment and never calls the translator.

Thus saves persist already-translated actual IDs, load restores them verbatim,
and no second translation occurs. The exception vector itself is not
serialized; its xrefs close to static lifecycle, theater loading, and the
helper. Changed-theater load reconstructs it; same-theater load preserves it.
If theater contents change between save/load, native does not migrate numeric
cell IDs. No replay/network command directly calls the helper; initial replay
scenario loading uses ordinary ingress and later commands use actual IDs.

## Installed retail theater data

All six active theater INIs resolve from `ra2md.mix -> localmd.mix`; repository
extracts match the installed entries byte-for-byte.

| Theater INI | Bytes | TileSet sections | Active `LastTilesInSet` | Commented | FNV-1a |
|---|---:|---:|---:|---:|---|
| `temperatmd.ini` | 25,269 | 82 | 0 | 20 | `d7dfe4e2e00155e5` |
| `snowmd.ini` | 24,258 | 83 | 0 | 18 | `084cde6830ee27ce` |
| `urbanmd.ini` | 30,237 | 111 | 0 | 20 | `b189cec0969fb87a` |
| `urbannmd.ini` | 32,614 | 122 | 0 | 20 | `96f5634036b5de84` |
| `desertmd.ini` | 25,514 | 82 | 0 | 20 | `ab2c2af32b0e9b8b` |
| `lunarmd.ini` | 26,560 | 85 | 0 | 20 | `e2f225d71782d6f3` |

The commented assignments are historical/editor documentation. Map-local
`[TileSet...]` sections cannot activate the feature because native reads the
separate theater MD.INI object.

## Current Rust mismatch

`TilesetLookup` has no compatibility state or transform API.
`parse_tileset_ini` currently loops `0..10000`, stops only when the section is
absent, parses `TilesInSet` as `get_i32(...).unwrap_or(0).max(0) as u32`, and
discards the signed values needed for native exception construction. Its
nearby default comment is also wrong. `IniSection::read_int` already provides
the exact missing/malformed/hexadecimal semantics.

`parse_iso_map_pack_records` correctly preserves Pack5's raw signed tile dword
and coordinate validation and should remain theater-independent.
`materialize_map_load_cells` applies Fill and then accepted explicit records,
but does not translate those explicit indexes before source lineage, LAT, or
variant lookup. `MapFile.cells` should remain raw; translation belongs in the
theater-aware materialization step.

## Builder-ready handoff

Add a compact declaration-ordered exception vector to `TilesetLookup` and a
pure `translate_legacy_map_tile_index(raw: i32) -> i32` method with the exact
sentinel, original-input signed comparison, wrapping prefix accumulation, and
no clamp.

Parser requirements:

1. Maintain separate signed actual and legacy cursors.
2. Resolve ordinal `TileSet%04d` without a fixed 10,000 cap.
3. Read `TilesInSet` via `read_int(...,-1)`; stop only on exact `-1`.
4. Read `LastTilesInSet` via `read_int(...,-1)`.
5. Build exception state with wrapping arithmetic before safe allocation
   normalization.
6. Other negative `TilesInSet` values may create zero Rust registry slots
   while retaining exact legacy arithmetic; do not emulate memory corruption.

Apply translation only to coordinate-accepted explicit map records, after
Pack5 parsing and before they replace Fill/source lineage. Never translate
Fill, runtime Mark/direct mutation, or snapshot restoration. Keep the API
format-neutral for future Pack1-4 support.

Required tests cover parser construction; equality and multi-entry boundaries;
comparison against original raw; empty-table identity; missing/malformed/hex
ReadInt behavior; exact `-1` termination; nonterminating negatives; wrapping
and `-1` versus `65535`; end-to-end explicit source/LAT/variant/final state;
Fill, OOB, runtime-mutation, and persistence exclusions; all six retail
fixtures empty; and a nonmonotonic declaration-order early-stop case.

## Stale-document corrections

- `ISOMAPPACK5_DECODER_GHIDRA_REPORT.md` incorrectly describes a flat sorted
  pair array and calls the helper validation. It is a declaration-ordered
  pointer vector of compatibility records and does not validate registry
  range.
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` places the TilesInSet read before
  start publication; live order publishes the candidate start first.
- `ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md` uses `{first_tile,count_diff}` and
  implies a derived Last default. Exact default is `-1`; field zero is the
  cumulative legacy boundary.
- “Global tile-array gaps/unloaded tilesets” is not the proven formula. The
  exact mechanism reconciles historical per-set counts declared by
  `LastTilesInSet`.

## Certainty ledger

| Area | Status | Confidence |
|---|---|---:|
| Loader lifecycle and same-theater reuse | verified | HIGH |
| Parser defaults/sentinel | verified | HIGH |
| Exception vector/record layout | verified | HIGH |
| Capacity/order/failure behavior | verified/excluded where unsafe | HIGH |
| Boundary/delta formula | verified | HIGH |
| Transform signedness/sentinel/wrapping | verified | HIGH |
| Five-caller closure and ingress ordering | verified | HIGH |
| Variant/LAT ordering | verified | HIGH |
| Fill/Mark/save/replay exclusions | verified | HIGH |
| Installed theater/map reachability | verified | HIGH |
| Current Rust divergence and handoff | verified | HIGH |

No load-bearing questions remain for this mechanism.

## Ghidra annotation candidates

No metadata was changed. Candidates are `DAT_00AA1078` as
`g_LegacyTileIndexExceptions`, `DAT_00AA107C` as
`LegacyTileIndexException**`, the subsequent capacity/valid/owns/count/growth
fields, and a function boundary at `0x0054A630` for the pointer-vector resize.
`Read_Theater_TileSets_INI` and `CalculateLegacyMapTileIndex` are already
correctly named.

## Sources

- Active retail `gamemd.exe`, image base `0x00400000`, live Ghidra
  decompilation/assembly/xrefs at the addresses cited above.
- `docs/research/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md`
- `docs/research/ISOMAPPACK5_DECODER_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md`
- `src/map/theater.rs`, `src/map/map_file.rs`,
  `src/map/resolved_terrain.rs`, and `src/rules/ini_value.rs`.
- Installed active theater entries and the physical installed map corpus.
