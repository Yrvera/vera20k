# Map LightConvert Cache 00483E30 / 00544E70 - Ghidra Research Report

**Address(es):** `0x00483E30` (`CellClass` LightConvert initializer/setter), `0x00544E70` (LightConvert cache lookup/creation)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** cell `LightConvertClass*` assignment, cache lookup key, refcount increments/decrements visible in this path, stored `CellClass` light fields, and active standard YR caller paths into the cache.  
**Non-Scope:** full `ConvertClass` palette-table generation, exact blitter internals, full `FUN_00484180` point-light math, Lightning Storm/Psychic Dominator lifecycle, and renderer-side SHP/TMP blitter pixel loops.  
**Confidence:** High for `0x00483E30`/`0x00544E70` control flow and fields; Medium for human-readable names of `CellClass+0x104..0x114`.  
**Active in YR:** Yes.

## Target Question

How does standard YR initialize and cache a cell's `LightConvertClass`, what keys the cache uses, how reference counts change, which fields are stored back on the cell, and which live render/update paths call it?

## Non-Goals

- Do not rederive map lamp `LightSourceClass` creation or falloff except where `FUN_00484180` feeds this cache.
- Do not model `ConvertClass__Constructor`, `Blitter_init`, `TMP_TileBlitter`, or `CC_Draw_Shape` table internals.
- Do not patch Rust or existing docs.

## Evidence Needed To Mark COMPLETE

- Decompile `0x00483E30` and `0x00544E70`.
- Confirm immediate callers/xrefs for active path liveness.
- Decompile the key normalization helper and LightConvert constructor enough to identify key storage.
- Confirm at least one terrain/overlay/techno/anim consumer of the stored cell fields.
- Identify unresolved lower-level conversion/blitter details as non-scope or deferred.

## Stop Conditions

- Stop after the two target functions, their direct normalization/constructor helpers, and representative active callers/consumers are mapped.
- Stop before descending into generic `ConvertClass`, blitter pixel loops, or all `CC_Draw_Shape` consumers.
- Stop if Ghidra would require mutating function boundaries or labels; none were required.

## 1. Overview

`FUN_00483E30` is the active cell-level LightConvert initializer/setter. It owns `CellClass+0x34`, updates a bundle of light fields at `+0x104..+0x114`, and either accepts an explicit `LightConvertClass*` or computes/cache-lookups one through `FUN_00484180 -> FUN_00544E70`.

`FUN_00544E70` is the cache lookup/create function for `LightConvertClass` entries keyed by three normalized RGB light components. It searches the global LightConvert pointer vector, constructs a new `LightConvertClass` on cache miss, appends it to the vector, and leaves reference count updates to callers.

## 2. Class Layout / Key Offsets

### `CellClass` fields verified in this slice

| Offset | Type | Verified use | Evidence | Active in YR |
|---|---:|---|---|---|
| `+0x24` | `short` | map X, also off-map sentinel test with `+0x26` | `0x00483E30`, `0x00484180` | Yes |
| `+0x26` | `short` | map Y, also off-map sentinel test with `+0x24` | `0x00483E30`, `0x00484180` | Yes |
| `+0x34` | pointer | `LightConvertClass*`; null triggers lazy initialization in draw paths | `0x00483E30`; callers `0x00480350`, `0x00705E00`, `0x0071C250`, `0x00423200` | Yes |
| `+0x104` | dword | light normalization/scale value copied from `FUN_00484180` or explicit caller | `0x00483E30`; `0x005558E0` | Yes |
| `+0x108` | word | light auxiliary value copied from `FUN_00484180` or explicit caller | `0x00483E30`; `0x00484180` initializes it to `0` before compute | Yes |
| `+0x10A` | word | one cell brightness value consumed by overlays/terrain/anims in special branches | `0x0047F6A0`, `0x0071C250`, `0x00423200`, `0x004D1890` | Yes |
| `+0x10C` | word | common terrain/object brightness value consumed by TMP, terrain, techno, overlay, anim paths | `0x00480350`, `0x00705E00`, `0x0071C250`, `0x0047F6A0`, `0x00423200` | Yes |
| `+0x10E` | word | alternate overlay brightness value | `0x0047F6A0` bridge/overlay branch | Yes |
| `+0x110` | word | normalized red cache component written back to cell | `0x00483E30`, `0x00484180` output mapping | Yes |
| `+0x112` | word | normalized green cache component written back to cell | `0x00483E30`, `0x00484180` output mapping | Yes |
| `+0x114` | word | normalized blue cache component written back to cell | `0x00483E30`, `0x00484180` output mapping | Yes |

### `LightConvertClass` fields verified in this slice

| Offset | Type | Verified use | Evidence | Active in YR |
|---|---:|---|---|---|
| `+0x194` | int | reference count incremented by `0x00483E30`/`0x00484050` callers; decremented when replacing old pointer if `g_GameActive != 0` | `0x00483E30` | Yes |
| `+0x198` | int | red cache key component after quantization/clamp | `0x00544E70`, constructor `0x00555DA0` stores input at index `0x66` | Yes |
| `+0x19C` | int | green cache key component after quantization/clamp | `0x00544E70`, constructor `0x00555DA0` stores input at index `0x67` | Yes |
| `+0x1A0` | int | blue cache key component after quantization/clamp | `0x00544E70`, constructor `0x00555DA0` stores input at index `0x68` | Yes |
| `+0x1A4/+0x1A8/+0x1AC` | int | original/current RGB fields also initialized to input values by constructor | `0x00555DA0` writes indices `0x69..0x6B` | Yes |
| `+0x1B0` byte | byte | initialized to `0` by constructor | `0x00555DA0` writes byte at `param_1+0x1B0` | Yes |

### Global cache vector

| Global | Verified use | Evidence | Active in YR |
|---|---|---|---|
| `DAT_0087F69C` | pointer to `LightConvertClass*` vector | `0x00544E70`, `0x00545000`, `0x0053AD00` | Yes |
| `DAT_0087F6A8` | vector count | same | Yes |
| `DAT_0087F6A0` | vector capacity | append capacity test in `0x00544E70`/`0x00545000` | Yes |
| `DAT_0087F6A5` | vector owns/grow flag used before allocator call | append condition in `0x00544E70` | Yes |
| `DAT_0087F698` / `DAT_0087F6AC` | vector allocator/grow helper state | append condition calls `(**(DAT_0087F698+8))(DAT_0087F6AC + DAT_0087F6A0, 0)` | Yes |
| `DAT_00887308` | base palette/table pointer gate; if zero, cache lookup returns `0` | first branch in `0x00544E70` | Conditional: active only after palette/theater setup |
| `DAT_0087F6C0` | default global palette/convert fallback in anim draw docs; not the cache vector itself | `AnimClass::DrawIt @ 0x00423200` | Yes |

## 3. Core Logic

### `0x00483E30` - cell initializer/setter

The function has one `this` cell pointer in `ECX`, followed by six stack arguments. The common lazy-init call shape is:

```text
this = CellClass*
explicit_convert = 0
field_104 = 0x10000
field_108 = 0
field_10A = 1000
field_10C = 1000
field_10E = 1000
```

The routine has three important branches:

1. **No explicit convert and dummy/off-map cell**
   - Condition: `(cell+0x24, cell+0x26) == (0,0)` or `(-1,-1)`.
   - Calls `FUN_00544E70(1000,1000,1000)`.
   - Stores returned pointer to `cell+0x34`.
   - If non-null, increments `LightConvert+0x194`.
   - Writes defaults:
     - `+0x104 = 0x10000`
     - `+0x108 = 0`
     - `+0x10A/+0x10C/+0x10E/+0x110/+0x112/+0x114 = 1000`
   - Active in YR: Yes, used by dummy/off-map cells and fallback initialization.

2. **No explicit convert and ordinary cell**
   - Calls `FUN_00484180` to compute the eight-field light bundle.
   - If `cell+0x34` is already non-null, it normalizes the new RGB triple via `FUN_00555AC0` and compares it against existing `LightConvert+0x198/+0x19C/+0x1A0`.
   - If the key differs:
     - If `g_GameActive != 0`, decrements old `LightConvert+0x194`.
     - Clears `cell+0x34`.
   - If no matching current pointer remains, calls `FUN_00544E70(new_red,new_green,new_blue)`.
   - If non-null, increments new `LightConvert+0x194`.
   - Writes `+0x104..+0x114` from the computed bundle.
   - Active in YR: Yes, standard map and draw paths.

3. **Explicit convert pointer**
   - If `cell+0x34` is non-null and `g_GameActive != 0`, decrements the old pointer's `+0x194`.
   - Stores `explicit_convert` into `cell+0x34`.
   - Unconditionally increments `explicit_convert+0x194`.
   - Reads `explicit_convert+0x198/+0x19C/+0x1A0` back into the local RGB output values.
   - Writes caller-provided `+0x104..+0x10E` plus the explicit convert's RGB key into `+0x110..+0x114`.
   - Active in YR: Yes; used by `0x00554D50` delayed light update commit records.

Tiny details that matter:

- Replacement decrements are gated by `g_GameActive != 0`; increments are not gated.
- The explicit-convert branch does not null-check `param_2` before incrementing `param_2+0x194`; callers must only pass a valid pointer.
- In the ordinary-cell branch, the current pointer is reused if its normalized RGB key matches, even if other cell fields like `+0x104` or `+0x10A..+0x10E` changed.
- The cached key is only the normalized RGB triple that ends up in `cell+0x110/+0x112/+0x114`.
- The cell stores both the `LightConvertClass*` and scalar brightness fields; draw calls pass both.

### `0x00544E70` - cache lookup/create

Input is a three-component RGB key, passed through `ECX`, `EDX`, and one stack argument at call sites.

1. If `DAT_00887308 == 0`, return `0`.
   - Active in YR: Conditional; this protects pre-palette/pre-theater setup.

2. If the requested key is exactly `(1000,1000,1000)` and the cache count is nonzero, return the first cached pointer `*DAT_0087F69C`.
   - Active in YR: Yes.
   - Important: this bypasses the linear search and assumes entry 0 is the default convert.

3. Normalize the third component and, by calling convention, the full RGB triple through `FUN_00555AC0`.
   - The decompile shows `FUN_00555AC0(&param_3)`, but assembly callers set `ECX`, `EDX`, and push the third value. `FUN_00555AC0` is `__fastcall` and mutates three adjacent/argument variables. The resulting compare is against all three normalized values.

4. Linear-search cache entries from index `1` through `DAT_0087F6A8-1`.
   - It intentionally starts at index 1; index 0 is reserved for the default convert path.
   - A match requires exact equality:
     - `entry+0x198 == red`
     - `entry+0x19C == green`
     - `entry+0x1A0 == blue`
   - On match, return that pointer.

5. On miss, select constructor argument `uVar6`:
   - `0x35` if `red + green + blue >= 2000`
   - `0x1B` if the sum is `< 2000`
   - Active in YR: Yes; likely controls conversion/blitter mode, but exact downstream meaning is non-scope.

6. Allocate `0x1B4` bytes and call `LightConvertClass__Constructor`:
   - `this = &DAT_00ABBED0` as decompiled target storage/new object confusion
   - source/base args include `&DAT_00885780` and `DAT_00887308`
   - key RGB values are passed through
   - `param_8 = (DAT_0087F6A8 != 0)`, so only the first/default construction gets `false`
   - `param_9 = 0`
   - `param_10 = 0x35` or `0x1B`

7. Append the new pointer to the cache vector if capacity/grow permits.
   - The function returns the new pointer whether or not the append path succeeds.
   - Active in YR: Yes.

### `0x00555AC0` - RGB key normalization

Each component is normalized as follows:

- Values greater than `999` become `1000`.
- Values less than `1` become `0`.
- Detail setting quantizes components:
  - `g_ExtraAnimationsEnabled == 0`: mask `0xFFFFFF80` (multiples of 128)
  - `g_ExtraAnimationsEnabled == 1`: mask `0xFFFFFFC0` (multiples of 64)
  - `g_ExtraAnimationsEnabled == 2`: mask `0xFFFFFFE0` (multiples of 32)
  - other values: no observed mask after clamp

Active in YR: Conditional, based on the user's extra animation/detail setting.

### `0x00555DA0` - constructor facts needed for this cache

The constructor stores:

- `+0x194 = 0` initial refcount.
- `+0x198/+0x19C/+0x1A0 = input RGB key`.
- `+0x1A4/+0x1A8/+0x1AC = input RGB key` again.
- `+0x1B0 byte = 0`.
- Vtable becomes `vtable__LightConvertClass`.

It chooses alternate scenario RGB source fields when Ion/Lightning/Psychic Dominator state helpers are active, and calls `FUN_00556090` plus `Blitter_init`. Full palette-table construction is non-scope.

Active in YR: Yes; the special-state branches are conditional on active lighting transitions.

## 4. INI Keys

No INI keys are read directly by `0x00483E30` or `0x00544E70`.

Relevant upstream data:

| INI key | Where it enters this path | Evidence | Active in YR |
|---|---|---|---|
| `[Lighting] Ambient/Red/Green/Blue/Ground/Level` | Scenario fields read by `ScenarioClass::Read_INI_Basic`, consumed by `FUN_00484180`, then cached through `0x00483E30/0x00544E70` | `0x0068A83A..0x0068A979`; `0x00484180` | Yes |
| `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, `LightBlueTint` | Upstream `LightSourceClass` entries scanned by `FUN_00484180`, then converted into cell RGB key | `0x00484180`; prior synthesis `MAP_LIGHTING_AND_LIGHT_POSTS_SYSTEM_MODEL_SYNTHESIS.md` | Yes |
| `AmbientChangeRate`, `AmbientChangeStep` | Rules-driven scenario ambience transition causes `FUN_004AE4C0`/cell recompute, not direct cache lookup | `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` | Conditional |

## 5. Integration Points

### Initialization and rebuild paths

- `FUN_00545000` frees existing LightConvert cache entries, constructs the default `(1000,1000,1000)` cache entry, initializes the dummy cell, then clears `cell+0x34` for map cells. Active in YR: Yes; called from `Read_Theater_TileSets_INI`, `ScenarioClass::Read_INI_Basic` in map editor mode, and `FUN_004AE450`.
- `MapClass__InitCellAttributes @ 0x00568CB0` iterates cells and calls `0x00483E30(0,0x10000,0,1000,1000,1000)` for each cell. Active in YR: Yes.
- `FUN_004AE450` calls `FUN_00545000`, then iterates cells and calls the same default initializer. Active in YR: Yes.

### Dynamic light-source dirty/update path

- `FUN_00554AF0` scans cells inside a `LightSourceClass` radius. If committing immediately (`param_2 == 0`), it calls `MapClass__Get_CellClass` then `0x00483E30` with default/no explicit convert to recompute. If delayed (`param_2 != 0`), it appends a 0x14-byte record containing default field values and the target cell coordinate to a pending array.
- `FUN_00554D50` processes the pending array. First pass computes light fields through `0x00484050`; final pass calls `0x00483E30` with an explicit cached convert pointer and stored field bundle, then frees the record.
- `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls `FUN_00554D50()` each tick after `LightningStorm__Process()` and before EMP/object updates. Active in YR: Yes.

### Draw-time lazy fallbacks and consumers

- `CellOverlay_TileDraw @ 0x00480350`: if `cell+0x34 == 0`, calls `0x00483E30`; then passes `cell+0x34` and `cell+0x10C` to `TMP_TileBlitter`. Active in YR: Yes.
- `CellClass__DrawOverlay_Body @ 0x0047F6A0`: lazy-inits `cell+0x34`; uses `+0x10A`, `+0x10C`, or `+0x10E` depending on overlay branch. Active in YR: Yes.
- `TerrainClass__Draw_It @ 0x0071C250`: lazy-inits `cell+0x34`; uses `+0x10C` normally or `+0x10A` for one type flag branch. Active in YR: Yes.
- `TechnoClass_DrawSHP @ 0x00705E00`: lazy-inits target/current cell and uses `+0x10C` for SHP draw brightness in multiple branches. Active in YR: Yes.
- `AnimClass__DrawIt @ 0x00423200`: cell-palette branches lazy-init/read `cell+0x34`, `+0x10A`, and `+0x10C`. Active in YR: Yes.
- `FUN_004D1890` alpha/queued draw path uses `CellClass__Get_Cell_At`, lazy-init in some branches, and reads `+0x10A/+0x10C`. Active in YR: Yes.

### LightConvert propagation dispatcher

- `FUN_0053AD00` iterates every `DAT_0087F69C` LightConvert and calls vtable slot `+4`, iterates color schemes and calls their convert vtable slot `+4`, then calls `FUN_004AE4C0()` and `FUN_004F42F0(1)`. Active in YR: Yes, when global lighting/palette transitions need propagation.

## 6. Current Rust Implementation Status

Current Rust does not model this cache. It builds direct RGB tint grids:

- `src/map/lighting.rs` parses `[Lighting]`, computes `LightingGrid`, accumulates point lights directly into `[f32;3]`, and applies `ExtraLight`.
- `src/app_init.rs` builds the lighting grid once during app init and assigns a uniform ground-level terrain tint.
- `src/render/palette_textures.rs` implements GPU palette and house-ramp textures, but there is no `LightConvertClass`-style per-cell palette conversion cache/refcount keyed by normalized RGB.

Current Rust delta: meaningful mismatch for final parity. Rust has per-cell RGB multipliers, while gamemd stores/cache-selects palette conversion objects plus per-cell scalar brightness fields consumed differently by terrain, overlays, anims, terrain objects, and techno SHPs.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00483E30` no-explicit dummy/off-map branch | verified | decompile `0x00483E30` | none |
| `0x00483E30` no-explicit ordinary branch | verified | decompile `0x00483E30`; callee `0x00484180` touched | exact names of `+0x104/+0x108` remain semantic |
| `0x00483E30` explicit-convert branch | verified | decompile `0x00483E30`; caller `0x00554D50` | none |
| `0x00544E70` null-base-palette guard | verified | decompile `0x00544E70` | none |
| `0x00544E70` default `(1000,1000,1000)` fast path | verified | decompile `0x00544E70` | none |
| `0x00544E70` cache key and search bounds | verified | decompile `0x00544E70`; `0x00555AC0`; constructor `0x00555DA0` | none |
| `0x00544E70` allocation/append path | verified | decompile `0x00544E70` | vector allocator internals not named |
| Refcount increments/decrements visible in target path | verified | `0x00483E30`, `0x00484050` | no full audit of all possible decrements outside this path |
| Cache teardown/reinitialization | touched-not-exhausted | `0x00545000` | destructor/vtable slot internals not decompiled |
| Delayed dirty-cell commit path | verified for interaction with target functions | `0x00554AF0`, `0x00554D50`, `0x0055AFB0` | full scheduler tuning outside scope |
| Terrain/TMP consumer | verified | `0x00480350`, `0x0071C250` | blitter pixel loop outside scope |
| Overlay consumer | verified | `0x0047F6A0` | all overlay type flag semantics outside scope |
| Techno/anim consumers | verified representative paths | `0x00705E00`, `0x00423200` | all draw branches outside scope |
| `ConvertClass` palette table generation | deferred | `0x00555DA0` calls `ConvertClass__Constructor`, `FUN_00556090`, `Blitter_init` | follow-up if byte-exact palette table needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-LC-001 - Is 0x00483E30 active in standard YR? -> Yes; it is called by map initialization, draw fallbacks, light dirty commits, terrain/overlay/techno/anim draw paths.` (evidence: xrefs to `0x00483E30`; decompiles `0x00568CB0`, `0x00480350`, `0x00705E00`, `0x00423200`)
- `[RESOLVED] OQ-LC-002 - Is 0x00544E70 active in standard YR? -> Yes; it is called by `0x00483E30` and `0x00484050` whenever a normalized RGB key needs a convert pointer.` (evidence: xrefs to `0x00544E70`)
- `[RESOLVED] OQ-LC-003 - What is the cache lookup key? -> three normalized RGB components compared to `LightConvert+0x198/+0x19C/+0x1A0`.` (evidence: `0x00544E70`, `0x00555DA0`)
- `[RESOLVED] OQ-LC-004 - Does the cache key include cell height/brightness fields? -> No observed; reuse in `0x00483E30` compares only `+0x198/+0x19C/+0x1A0`.` (evidence: `0x00483E30`)
- `[RESOLVED] OQ-LC-005 - What happens for default full-bright RGB? -> If count is nonzero, `(1000,1000,1000)` returns cache index 0 directly.` (evidence: `0x00544E70`)
- `[RESOLVED] OQ-LC-006 - What are key bounds? -> components are clamped to `0..1000`, then quantized by `g_ExtraAnimationsEnabled` masks for settings 0/1/2.` (evidence: `0x00555AC0`)
- `[RESOLVED] OQ-LC-007 - How are refcounts updated? -> `0x00483E30` increments new pointers, decrements replaced old pointers only when `g_GameActive != 0`; explicit branch also increments the passed pointer.` (evidence: `0x00483E30`)
- `[RESOLVED] OQ-LC-008 - What if cache base palette is absent? -> `0x00544E70` returns `0`; downstream callers often lazy-init and some bail if pointer remains null.` (evidence: `0x00544E70`, `0x00705E00`)
- `[RESOLVED] OQ-LC-009 - Which cell fields are stored? -> `+0x34` pointer plus `+0x104/+0x108/+0x10A/+0x10C/+0x10E/+0x110/+0x112/+0x114` light bundle.` (evidence: `0x00483E30`)
- `[RESOLVED] OQ-LC-010 - Who consumes `+0x34`? -> TMP terrain directly passes it to `TMP_TileBlitter`; anim cell-palette branches read it; many draw paths lazy-init it.` (evidence: `0x00480350`, `0x00423200`)
- `[RESOLVED] OQ-LC-011 - Who consumes `+0x10C`? -> TMP, terrain objects, overlays, techno SHPs, anims, and queued draw paths use it as brightness/scalar input.` (evidence: `0x00480350`, `0x0071C250`, `0x0047F6A0`, `0x00705E00`, `0x00423200`, `0x004D1890`)
- `[RESOLVED] OQ-LC-012 - Does dynamic lighting use an immediate or delayed path? -> both; `0x00554AF0` can recompute immediately or enqueue records, and `0x00554D50` later commits explicit cached converts.` (evidence: `0x00554AF0`, `0x00554D50`)
- `[RESOLVED] OQ-LC-013 - Is the pending dirty path ticked in standard YR? -> Yes, `LogicClassPerTickUpdateLiveVector` calls `0x00554D50` every tick.` (evidence: `0x0055AFB0`)
- `[RESOLVED] OQ-LC-014 - What does cache creation do if append fails? -> It still returns the newly constructed pointer; append only controls future reuse.` (evidence: `0x00544E70`)
- `[RESOLVED] OQ-LC-015 - What selects constructor mode `0x35` vs `0x1B`? -> sum of normalized RGB components: `<2000` selects `0x1B`, otherwise `0x35`.` (evidence: `0x00544E70`)
- `[DEFERRED] OQ-LC-016 - What exactly do `0x35` and `0x1B` mean inside blitter/conversion code?` (category: out-of-scope; reason: requires `ConvertClass`/blitter table investigation beyond the two-function cache slice; next-step-if-pursued: investigate `LightConvertClass__Constructor -> ConvertClass__Constructor -> FUN_00556090 -> Blitter_init`)
- `[DEFERRED] OQ-LC-017 - What are exact semantic names for `CellClass+0x104` and `+0x108`?` (category: requires-different-system-context; reason: storage and consumers are verified enough for this cache slice, but naming requires broader render/light math audit; next-step-if-pursued: extend `FUN_00484180` and `FUN_005558E0` report)
- `[DEFERRED] OQ-LC-018 - Are there refcount decrements outside `0x00483E30`/`0x00484050`/teardown?` (category: bounded-cost-too-high; reason: this report proves target path refcounting, not all lifetime management; next-step-if-pursued: xref audit of writes to `LightConvert+0x194`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Cells own a `LightConvertClass*` equivalent at `+0x34`, not just raw RGB. | `0x00483E30`; consumers `0x00480350`, `0x00705E00` | missing | `src/map/lighting.rs`, render palette path | Introduce a render-facing concept equivalent to a per-cell light-convert cache or deterministic indexed light profile; keep scalar fields separate from RGB tint. | `map_lighting_cache_reuses_default_convert_for_unlit_cells`: many default cells resolve to one cached profile. | Do not store only `[f32;3]` if the renderer needs per-cell palette conversion parity. |
| Cache key is normalized RGB triple only, stored at `LightConvert+0x198/+0x19C/+0x1A0` and mirrored to cell `+0x110/+0x112/+0x114`. | `0x00483E30`, `0x00544E70`, `0x00555DA0` | missing | `src/map/lighting.rs` | Cache by integer RGB components after clamp/quantize, not by cell coordinate or full brightness bundle. | `map_lighting_cache_key_ignores_cell_coordinate_and_height`: two cells with same normalized RGB but different scalar brightness share a convert profile. | Do not key cache by `(cell_x, cell_y)` or full f32 vector. |
| Default `(1000,1000,1000)` uses cache index 0 when the cache is initialized. | `0x00544E70`, `0x00545000` | missing | palette/cache initialization surface | Seed a default full-bright profile before per-cell cache lookups. | `map_lighting_default_profile_is_singleton`: all unmodified cells reference profile 0 after init. | Do not allocate one full-bright profile per cell. |
| Components clamp to `0..1000` and may quantize by `g_ExtraAnimationsEnabled`. | `0x00555AC0` | missing | lighting config / render quality settings | Use integer fixed/scaled light components and detail-dependent quantization before cache lookup. | `map_lighting_quantizes_profile_keys_by_detail_level`: values 999/1001 clamp to 1000, negative values to 0, and detail level 0 masks low 7 bits. | Do not use raw f32 equality as the cache key. |
| `+0x10C` is a scalar brightness used by terrain tiles, terrain objects, overlays, techno SHPs, and anims. | `0x00480350`, `0x0071C250`, `0x0047F6A0`, `0x00705E00`, `0x00423200` | partially missing/mismatched | `src/map/lighting.rs`, terrain/entity/overlay/anim render paths | Preserve and feed a per-cell brightness scalar separately from RGB profile. | `map_lighting_cell_brightness_feeds_tile_and_sprite_paths`: a lamp changes both terrain tile blit brightness and object/SHP draw brightness in the same affected cell. | Do not assume terrain-only tint is enough; sprites also read cell lighting. |
| Light-source dirtying can enqueue cells and commit them later through explicit convert pointers. | `0x00554AF0`, `0x00554D50`, `0x0055AFB0` | missing; Rust builds once at app init | future dynamic lighting update surface | Support dirty-cell recompute/commit when light sources change, not only static map-load lighting. | `map_lighting_dirty_lamp_destroy_recomputes_affected_cells`: destroying/powering off a lamp restores nearby cell profiles without full map rebuild. | Do not rebuild only at app init if lamp state can change. |

### Rust Test Name Proposals

- `map_lighting_cache_reuses_default_convert_for_unlit_cells`
- `map_lighting_cache_key_ignores_cell_coordinate_and_height`
- `map_lighting_default_profile_is_singleton`
- `map_lighting_quantizes_profile_keys_by_detail_level`
- `map_lighting_cell_brightness_feeds_tile_and_sprite_paths`
- `map_lighting_dirty_lamp_destroy_recomputes_affected_cells`

### Negative Facts / Do Not Do

- Do not model LightConvert as one RGB tint per cell with no cache; gamemd caches conversion objects by normalized RGB.
- Do not key the cache by cell coordinate, map height, `LightVisibility`, or light-source identity.
- Do not treat `cell+0x34 == null` as impossible; multiple draw paths defensively lazy-initialize it.
- Do not fold `+0x10A/+0x10C/+0x10E` into a single universal tint without checking the consuming draw branch.
- Do not assume refcount decrement always happens; replacement decrements are gated by `g_GameActive`.
- Do not descend into Lightning Storm-specific behavior for ordinary map lamp posts except as a shared global lighting transition that can force recompute.

### Remaining Uncertainty

- Exact meanings of constructor mode constants `0x35` and `0x1B` inside lower-level conversion/blitter code.
- Exact semantic names for `CellClass+0x104` and `+0x108`.
- Complete lifetime audit of `LightConvert+0x194` outside this target path.
- Byte-exact `ConvertClass` table contents after `FUN_00556090` and `Blitter_init`.

## Sources

- Ghidra decompiled:
  - `0x00483E30` - cell LightConvert initializer/setter
  - `0x00544E70` - LightConvert cache lookup/create
  - `0x00555AC0` - RGB key clamp/quantization helper
  - `0x00555DA0` - `LightConvertClass__Constructor`
  - `0x00484050` - compute/reuse helper used by delayed light records
  - `0x00554AF0` - light-source affected-cell scanner/enqueuer
  - `0x00554D50` - pending dirty-cell processor/committer
  - `0x00545000` - cache teardown/default creation/cell pointer clear
  - `0x00568CB0` - `MapClass__InitCellAttributes`
  - `0x004AE450` - full cache/cell rebuild wrapper
  - `0x0053AD00` - LightConvert/color-scheme propagation dispatcher
  - `0x0055AFB0` - tick integration, calls `0x00554D50`
  - `0x00480350` - `CellOverlay_TileDraw`
  - `0x0047F6A0` - `CellClass__DrawOverlay_Body`
  - `0x0071C250` - `TerrainClass__Draw_It`
  - `0x00705E00` - `TechnoClass_DrawSHP`
  - `0x00423200` - `AnimClass__DrawIt`
  - `0x004D1890` - queued/alpha draw consumer
  - `0x00484180` and `0x005558E0` - touched to map output fields feeding the cache
- Ghidra xrefs:
  - `0x00483E30` callers: `0x00554D50`, `0x00568CB0`, `0x00545000`, `0x00554AF0`, `0x0047F5C9`, `0x0047F748`, `0x00480384`, `0x004D1BC9`, `0x004D1EC9`, `0x004AE494`, `0x00705F42`, `0x007060F7`, `0x0071C27B`, `0x0071C40A`, `0x00423273`
  - `0x00544E70` callers: `0x00483E30`, `0x00484050`
- Research docs referenced:
  - `docs/research/MAP_LIGHTING_AND_LIGHT_POSTS_SYSTEM_MODEL_SYNTHESIS.md`
  - `docs/research/CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`
  - `docs/research/LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`
  - `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
- Rust files scanned:
  - `src/map/lighting.rs`
  - `src/app_init.rs`
  - `src/render/palette_textures.rs`

## Result

COMPLETE for the requested `0x00483E30` / `0x00544E70` cache/refcount/cell-field/caller-path slice. The lower-level pixel conversion table and all lifetime/refcount edges outside this slice remain explicit follow-up work.
