# Map Lighting Cell Compute `0x00484180` -- Ghidra Research Report

**Address(es):** `0x00484180` primary; `0x005558E0` RGB normalization helper; `0x00483E30` cell cache writer; `0x00484050` query/cache helper; `0x00544E70` LightConvert lookup/constructor; `0x00554A60`/`0x00554A80` LightSource enable/disable; `0x00554AF0` dirty-cell scheduling; `0x00554D50` deferred dirty-cell flush  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact per-cell lighting computation performed by `FUN_00484180`, including point-light falloff, fixed-point/milli scaling, clamp order, active LightSource gates visible to this function, and immediate output consumers.  
**Non-Scope:** full `LightConvertClass` palette generation, cache lifetime beyond immediate lookup/refcount evidence, `ExtraLight=` consumer, `BuildingLightClass` spotlights, superweapon lighting transition state machines, runtime screenshot comparison.  
**Confidence:** High for the primary formula and immediate gates; Medium for semantic names of output slots where downstream consumers were only touched.  
**Active in YR:** Yes. It is called during standard scenario init and rendering paths; individual LightSource contribution is conditional on source active state and `[Options] DetailLevel`.

## 0. Investigation Setup

**Target question.** What exactly does `FUN_00484180` compute for one map cell's lighting, including point-light falloff, scaling, clamp order, active LightSource gates, output channels, and YR liveness?

**Non-goals.**

- Do not re-investigate Lightning Storm.
- Do not re-investigate lamp allocation except when needed to understand active LightSource gates.
- Do not prove the complete `LightConvertClass` palette math.
- Do not patch Rust, INI files, existing docs, or `.swarm-claims.md`.

**Evidence needed to mark COMPLETE.**

- Decompile and read disassembly of `0x00484180`.
- Trace its immediate callers and output consumers far enough to know the function is live.
- Trace all non-trivial callees in the function: active state helpers, RGB normalization, LightConvert lookup, and LightSource activation.
- Confirm the constants and clamp order from disassembly where decompiler prose hides arithmetic.
- Compare the verified formula against current Rust surfaces.

**Stop conditions.**

- Stop at downstream `LightConvertClass` internal palette generation once `0x00484180` outputs and immediate cache lookup are known.
- Stop at dirty-cell scheduling once it proves when cells are recomputed; leave batch cadence details to the cache/dirty-slot swarm slot.
- Stop if a function boundary is missing and only mutation could recover it. No such mutation was required for the claimed scope.

## 1. Overview

`FUN_00484180` computes a cell's lighting state from scenario `[Lighting]` ambience, active radius `LightSourceClass` objects, the cell height byte, and the current special-lighting mode. It uses milli-units where `1000 == 1.0`, a 16.16 brightness scale in one output slot, and `0..2000` clamps for ambient/RGB channels.

The point-light path is active in standard YR, but individual sources only contribute when `LightSource+0x48 != 0` and `LightSource+0x34 <= DAT_00A8EB78` (`[Options] DetailLevel`). `LightSourceClass` constructor initializes `+0x34 = 2`, so normal lamp contribution requires DetailLevel `2`; `OptionsClass__SetDefaults @ 0x005FA370` sets the option field to `2`, while `OptionsClass__ReadFromINI @ 0x005FA782` accepts/clamps persisted values `0..2`.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type | Purpose in this slice | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `CellClass` | `+0x24/+0x26` | signed shorts | map coordinate packed in the first dword; `(0,0)` and `(-1,-1)` are treated as sentinel/non-map cells | `0x0048418A..0x004841A9`, `0x00484621..0x00484675` | Yes |
| `CellClass` | `+0x11B` | signed byte | cell height/level used for top and bottom ambient formulas | `0x00484483`, `0x00484564` | Yes |
| `CellClass` | `+0x34` | pointer | cached `LightConvertClass*` used by `0x00483E30` | `0x00483E30`; xrefs from draw/init functions | Yes |
| `LightSourceClass` | `+0x24` | int | intensity contribution, stored from `LightIntensity` scaled to milli-units | `0x00554760`, read at `0x004843BA` | Conditional |
| `LightSourceClass` | `+0x28/+0x2C/+0x30` | int | red/green/blue tint contribution, scaled to milli-units | `0x00554760`, read at `0x004843DD`, `0x00484400`, `0x0048441F` | Conditional |
| `LightSourceClass` | `+0x34` | int | source detail threshold; constructor writes `2` | `0x00554760`, gate at `0x0048429F..0x004842AA` | Conditional |
| `LightSourceClass` | `+0x38/+0x3C` | int | source world X/Y in leptons | `0x00554760`, read at `0x004842E2..0x00484318` | Conditional |
| `LightSourceClass` | `+0x44` | uint | visibility radius in leptons | `0x00554760`, read at `0x004842F0`, `0x00484392` | Conditional |
| `LightSourceClass` | `+0x48` | byte | active flag; enabled by `0x00554A60`, disabled by `0x00554A80` | `0x00554A60`, `0x00554A80`, gate at `0x00484294..0x00484299` | Conditional |
| global | `0x00ABCA14/0x00ABCA20` | vector/count | global LightSource list iterated by `0x00484180` | `0x0048427D..0x00484459`; constructor `0x00554760` inserts | Yes |
| global | `0x00A8EB78` | int | `[Options] DetailLevel`; source contributes only if threshold <= this value | `0x0048429F..0x004842AA`; `0x005FA370`; `0x005FA782` | Yes |
| global | `0x00829AE4` | byte | LightSource dirty scheduling enabled flag; off during load/clear, on afterward | `0x00554AF0`; `0x006851F0`; `0x00687AE3` | Yes |

## 3. Core Logic

### 3.1 Sentinel Cells

If the cell coordinate is `(0,0)` or `(-1,-1)`, `0x00484180` returns neutral values and does not inspect scenario lighting or LightSources.

Output values in this branch:

- 16.16 scale = `0x10000`
- additive light intensity = `0`
- ambient/top/bottom/RGB-like output slots = `1000`

Evidence: decompile `0x00484180`; disassembly `0x0048418A..0x004841A9` branches to `0x00484621..0x00484675`.

Active in YR: Yes, as an edge/sentinel path used by the same live function.

### 3.2 Scenario `[Lighting]` Base Values

For normal cells, the function initializes from current scenario lighting fields:

- ambient base = `Scenario+0x352C * 1000 / 100`
- red tint = `Scenario+0x3534 * 1000 / 100`
- green tint = `Scenario+0x3538 * 1000 / 100`
- blue tint = `Scenario+0x353C * 1000 / 100`
- point-light additive intensity starts at `0`

The disassembly implements `*1000/100` with `lea` multiplication and magic signed division by 100 (`0x51EB851F`, shift 5). `ScenarioClass__Read_INI_Basic @ 0x00689E90` reads `[Lighting]` keys into the surrounding `0x3528..0x3544` fields.

Active in YR: Yes. `ScenarioClass__Full_Init @ 0x00687AE3` calls `ScenarioClass__Read_INI_Basic`, then later `MapClass__InitCellAttributes`.

### 3.3 Active LightSource Gates

Each source in `DAT_00ABCA14[0..DAT_00ABCA20)` is skipped unless both gates pass:

1. `*(byte *)(source + 0x48) != 0`
2. `*(int *)(source + 0x34) <= DAT_00A8EB78`

`LightSourceClass__Constructor @ 0x00554760` sets `+0x34 = 2` and `+0x48 = 0`. `CreateProductionAnim @ 0x00554A60` sets `+0x48 = 1` and calls `0x00554AF0` to dirty affected cells. `0x00554A80` sets it back to `0` and dirties affected cells. Xrefs to `0x00554A60` include `BuildingClass__Unlimbo`, `BuildingClass__ReadFromINI`, `BuildingClass__GoOnline`, `BuildingClass__RestoreOnlineEffects`, `BuildingClass__OnConstructionComplete`, `BuildingClass__ChangeOwner`, and `RadSiteClass__Activate`.

Active in YR: Conditional. The mechanism is live; a source contributes when active and when user/detail setting is high enough. Constructor threshold `2` means a default-created building lamp is culled at DetailLevel `0` or `1`.

### 3.4 Cell Center And Radius Test

The target cell center is computed in leptons:

- `cell_x_center = cell_x * 256 + 128`
- `cell_y_center = cell_y * 256 + 128`

The function first compares squared distance to squared radius:

- `dx = cell_x_center - source_x`
- `dy = cell_y_center - source_y`
- if `(dx*dx + dy*dy) > radius*radius`, skip

The comparison uses unsigned `ja` after 32-bit integer multiplication. Standard lamp radii such as `3500`, `4000`, and `5000` are safe from overflow, but extremely large modded radii could wrap before the compare.

If the squared test passes, it computes `sqrt(dx^2 + dy^2)` using `Sqrt_Approx @ 0x004CAC40`, converts with `Math__ftol @ 0x007C5F00`, and applies a second inclusive guard: if `distance > radius`, skip. Distance exactly equal to radius contributes factor `0`.

Active in YR: Yes. This is inside the live point-light loop.

### 3.5 Falloff And Contribution Formula

The falloff factor is computed in integer milli-units:

```text
factor = ((radius * 1000) - (distance * 1000)) / radius
```

This is an unsigned divide at `0x004843B8`; because the prior guards ensure `distance <= radius`, the numerator is non-negative in normal execution.

Each source field contribution is then:

```text
add = trunc_toward_zero(source_field * factor / 1000)
```

The `/1000` is compiled as signed multiply by `0x10624DD3`, arithmetic shift right by 6, then sign correction. This matters for negative light intensity/tint values such as `NEGLAMP` and `NEGRED`: negative contributions are supported and truncate toward zero, not floor.

Fields affected:

- additive intensity output adds `source+0x24 * factor / 1000`
- red output adds `source+0x28 * factor / 1000`
- green output adds `source+0x2C * factor / 1000`
- blue output adds `source+0x30 * factor / 1000`

Active in YR: Yes for active sources.

### 3.6 Height, Ground, And Special-Lighting Mode

After summing sources, the function adds point-light intensity to base ambient and computes two height-adjusted ambient values:

```text
top = base_ambient + point_intensity + (level * height - ground)
bottom = base_ambient + point_intensity + (level * (height + 4) - ground)
```

The selected `ground`/`level` pair depends on three mode helpers:

| First true gate | Ground field | Level field | Evidence | Active in YR |
|---|---:|---:|---|---|
| `FUN_0053A100() != 0` | `Scenario+0x3558` | `Scenario+0x355C` | `0x00484473..0x004844BF` | Conditional; special lighting mode |
| `FUN_0053B400() != 0` | `Scenario+0x358C` | `Scenario+0x3590` for bottom, but top uses `Scenario+0x3574` as level | `0x004844C4..0x0048450A` | Conditional; special lighting mode |
| `FUN_0053A110() != 0` | `Scenario+0x3570` | `Scenario+0x3574` | `0x0048450F..0x0048455B` | Conditional; special lighting mode |
| none | `Scenario+0x3540` | `Scenario+0x3544` | `0x0048455D..0x004845A0` | Yes; standard map lighting |

The default/ordinary branch uses `[Lighting] Ground` and `Level`. The other branches are live code paths but are special-mode dependent; this slot did not prove their superweapon/state ownership beyond helper globals (`DAT_00A9FAB4`, `DAT_00A9FABC`, `DAT_00A9FAC0`).

### 3.7 Clamp And RGB Normalization Order

Clamp order is load-bearing:

1. `top` is clamped high to `2000`.
2. a duplicate top-like output is set to the clamped `top`.
3. `FUN_005558E0` clamps/normalizes RGB tints and may scale point-light additive intensity.
4. `bottom` is multiplied by the 16.16 scale output from `FUN_005558E0`, arithmetic-shifted right 16, then high-clamped to `2000`.
5. `top`, duplicate top, and scaled bottom are clamped low to `0`.

`FUN_005558E0` behavior:

- red/green/blue are first low-clamped to `0`, then high-clamped to `2000`.
- if all three are exactly `1000`, it leaves scale `0x10000` and additive intensity unchanged.
- otherwise it picks the maximum channel.
- it computes scale with `Math__ftol(max * 65536.0 * 0.001)`.
- if the resulting scale is `< 0x42` (`66`), it resets scale to `0x10000`, RGB to `1000/1000/1000`, and additive intensity to `0`.
- otherwise it normalizes the max channel to `1000`, normalizes the two non-max channels to approximately `channel * 1000 / max`, and scales additive intensity by `(scale * additive_intensity) >> 16`.
- final additive intensity is high-clamped to `2000`.

Tiny asymmetry from disassembly: in the red-max branch, the compiler preserves the floating denominator on the FPU stack and divides green/blue by the floating `red * 65.536`; in the green-max and blue-max branches, it divides by the integer `scale` stored through `Math__ftol`. This can differ by one unit near rounding boundaries.

Active in YR: Yes. `0x00484180` always calls this helper on normal cells.

### 3.8 Immediate Output Consumers

`0x00483E30` writes the computed values into `CellClass` fields and maintains the cached `LightConvertClass*`:

- If the cell is sentinel `(0,0)` or `(-1,-1)`, it fetches neutral `LightConvert(1000,1000,1000)` and writes neutral cell fields.
- Otherwise it calls `0x00484180`.
- If the existing cell `+0x34` cache pointer exists and its stored RGB key differs from the newly computed/quantized RGB, the old cache refcount at `+0x194` is decremented when `g_GameActive != 0`, then the pointer is cleared.
- If no usable cache remains, it calls `0x00544E70` to get/create a `LightConvertClass`.
- It then writes the computed outputs to `CellClass+0x104`, `+0x108`, `+0x10A`, `+0x10C`, `+0x10E`, `+0x110`, `+0x112`, and `+0x114`.

`0x00544E70` quantizes RGB cache keys through `0x00555AC0` before lookup/creation. `0x00555AC0` clamps each channel to `0..1000`, then masks off low bits depending on DetailLevel:

- DetailLevel `0`: `& 0xFFFFFF80`
- DetailLevel `1`: `& 0xFFFFFFC0`
- DetailLevel `2`: `& 0xFFFFFFE0`

Active in YR: Yes. Xrefs to `0x00483E30` include `MapClass__InitCellAttributes`, cell overlay drawing, terrain drawing, anim drawing, and `TechnoClass_DrawSHP`.

## 4. INI Keys

| Key | Section | Binary use in this slice | Default / sample | Active in YR |
|---|---|---|---|---|
| `Ambient` | map `[Lighting]` | read into Scenario current ambient; converted to milli-units in `0x00484180` | map default comes from scenario initialization; Rust default is `1.0` | Yes |
| `Red` / `Green` / `Blue` | map `[Lighting]` | read into Scenario RGB fields; converted to milli-units | Rust default `1.0` | Yes |
| `Ground` | map `[Lighting]` | ordinary branch subtracts `Ground` from height-adjusted ambient | Rust default `0.0` | Yes |
| `Level` | map `[Lighting]` | ordinary branch adds `Level * height` and `Level * (height+4)` | Rust default `0.032` | Yes |
| `LightVisibility` | building rules | constructor/source radius in leptons (`source+0x44`) | rules comments: default `5000`; lamps use `3500..5000` | Yes, if source active |
| `LightIntensity` | building rules | source intensity (`source+0x24`) | rules comments: default `0`; lamps use positive and negative values | Yes, if source active |
| `LightRedTint` / `LightGreenTint` / `LightBlueTint` | building rules | source RGB contributions (`+0x28/+0x2C/+0x30`) | rules comments: default `1.0`; constructor defaults are `1000000` fixed scale | Yes, if source active |
| `DetailLevel` | user `[Options]` | gates LightSource contribution and RGB cache quantization | SetDefaults sets `2`; INI parser clamps `0..2`; dialogs store `0` or `2` | Yes |
| `AmbientChangeRate` / `AmbientChangeStep` | rules `[General]` | not read by `0x00484180`; related to dynamic lighting transitions | `rulesmd.ini` lines 767-768: `.2`, `.2` | Out-of-scope for this formula |

Representative lamp data from `rulesmd.ini`: `GALITE`/`INGALITE` use `LightVisibility=5000`, `LightIntensity=0.2`, RGB `0.05,0.05,0.01`; `NEGLAMP` uses `LightVisibility=3500`, `LightIntensity=-0.15`; `REDLAMP` uses `LightVisibility=4000`, `LightIntensity=0.01`, red tint `1.5`.

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `0x00483E30` | main cell compute/cache writer; immediate caller of `0x00484180` | xrefs and decompile | Yes |
| `0x00484050` | query helper that computes lighting and returns/creates matching LightConvert without necessarily storing it on the cell | decompile; called by `0x00554D50` | Yes |
| `MapClass__InitCellAttributes @ 0x00568CB9` | initializes all cells, calls `0x00483E30` during standard scenario load | decompile | Yes |
| `0x00554AF0` | marks or immediately recomputes cells within a LightSource radius when a source toggles or changes | decompile | Yes, conditional on `DAT_00829AE4` and source detail gate |
| `0x00554D50` | deferred dirty-cell flush, uses `0x00484050` then `0x00483E30` and triggers redraw | decompile; caller `LogicClassPerTickUpdateLiveVector` | Yes |
| Draw paths | lazy compute if `CellClass+0x34 == 0`, then read lighting fields | xrefs to `0x00483E30`, e.g. `CellClass__DrawOverlay_Shadow`, `TechnoClass_DrawSHP` | Yes |

## 6. Current Rust Implementation Status

Current Rust has the right high-level shape but not the verified integer pipeline:

- `src/map/lighting.rs:65..77` parses `[Lighting]` keys as `f32`.
- `src/map/lighting.rs:80..97` computes `ambient * (1 - ground) + level * z`, then multiplies RGB and proportionally caps only when max exceeds `2.0`.
- `src/map/lighting.rs:152..179` collects lights from structure entities and skips `light_visibility <= 0 || light_intensity == 0.0`.
- `src/map/lighting.rs:188..217` applies Euclidean cell-space linear falloff and clamps each channel immediately to `0..2`.
- `src/app_init.rs:339..371` builds the lighting grid once at load and accumulates point lights.
- `src/rules/object_type.rs:700..708` and `1099..1108` parse light fields, but Rust defaults `LightVisibility` to `0`, while binary `BuildingTypeClass` constructor defaults it to `5000`.
- `src/rules/art_data.rs:96..99` and `329` parse `ExtraLight`, but `ExtraLight` was non-scope here.

Observed deltas:

- Rust uses cell distances in cells; binary uses lepton centers and radius in leptons.
- Rust clamps after each channel contribution; binary sums all light contributions, then performs height/ground adjustment, RGB normalization, and final clamps.
- Rust does not model DetailLevel gating (`source+0x34 <= DAT_00A8EB78`) or LightConvert cache-key quantization.
- Rust does not model separate top/bottom ambient outputs or 16.16 RGB brightness scaling.
- Rust does not model lazy/deferred dirty-cell recomputation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00484180` sentinel branch | verified | decompile + disassembly `0x0048418A..0x00484675` | none |
| `FUN_00484180` scenario base conversion | verified | decompile + disassembly `0x004841AF..0x0048426D`; `0x00689E90` prior spot-check | none |
| `FUN_00484180` LightSource loop gates | verified | `0x0048427D..0x004842AA`; constructor/enable decompiles | none |
| `FUN_00484180` distance/falloff math | verified | disassembly `0x004842B0..0x0048443E` | none |
| `FUN_00484180` height/ground branch selection | verified for formula; touched for mode ownership | `0x00484473..0x004845A0`; helper decompiles `0x0053A100`, `0x0053A110`, `0x0053B400` | owning systems for special modes are out-of-scope |
| `FUN_005558E0` clamp/normalization | verified | decompile + local disassembly from retail `gamemd.exe` bytes `0x005558E0..0x00555AB7` | `Math__ftol` exact rounding mode not independently audited |
| `FUN_00483E30` immediate cache writer | verified enough for this slice | decompile + xrefs | full cache lifecycle belongs to slot 2 |
| `FUN_00484050` query/cache helper | verified enough for this slice | decompile + xref from `0x00554D50` | none for this slice |
| `FUN_00544E70` LightConvert lookup/constructor | touched-not-exhausted | decompile + disassembly | internal `LightConvertClass__Constructor @ 0x00555DA0` not investigated |
| `FUN_00555AC0` cache-key quantization | verified enough for immediate key behavior | decompile `0x00555AC0` | downstream palette visual effect belongs to cache slot |
| `FUN_00554A60`/`0x00554A80` active flag toggles | verified | decompile + xrefs | none for this slice |
| `FUN_00554AF0` dirty radius scheduling | touched-not-exhausted | decompile | exact batching cadence belongs to dirty scheduling slot |
| `FUN_00554D50` dirty flush | touched-not-exhausted | decompile + caller xref | exact tick budget behavior belongs to dirty scheduling slot |
| Rust lighting implementation | verified surface scan | Codegraph + line reads | no code patched |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ1 -- Is 0x00484180 on a live standard YR path? -> Yes; called from 0x00483E30, which is called by MapClass init and draw paths.` (evidence: `get_function_callers 0x00484180`; xrefs to `0x00483E30`; `MapClass__InitCellAttributes @ 0x00568CB9`)
- `[RESOLVED] OQ2 -- What are the sentinel cell cases? -> `(0,0)` and `(-1,-1)` return neutral lighting.` (evidence: `0x0048418A..0x00484675`)
- `[RESOLVED] OQ3 -- What units does the radius test use? -> leptons, with cell centers at `cell*256+128`.` (evidence: `0x004842B0..0x004842D0`)
- `[RESOLVED] OQ4 -- Is the radius edge inclusive? -> yes for evaluation; exact edge contributes zero because `distance == radius` passes and factor becomes zero.` (evidence: `0x00484392..0x004843B8`)
- `[RESOLVED] OQ5 -- Is falloff linear? -> yes: `(radius-distance)/radius` in milli-units.` (evidence: `0x0048439A..0x004843B8`)
- `[RESOLVED] OQ6 -- Are negative lights supported? -> yes; signed contribution divide by 1000 handles negative source fields and truncates toward zero.` (evidence: `0x004843BA..0x0048443E`; `rulesmd.ini` `NEGLAMP`/`NEGRED`)
- `[RESOLVED] OQ7 -- What gates source participation? -> `+0x48 != 0` and `+0x34 <= DetailLevel`.` (evidence: `0x00484294..0x004842AA`; `0x00554A60`; `0x00554A80`)
- `[RESOLVED] OQ8 -- Is DetailLevel a YR/user option? -> yes, OptionsClass field at byte `0x18`, read from `[Options] DetailLevel` and clamped `0..2`.` (evidence: `0x005FA370`; `0x005FA782`)
- `[RESOLVED] OQ9 -- What is constructor default for source detail threshold? -> `LightSource+0x34 = 2`.` (evidence: `LightSourceClass__Constructor @ 0x00554760`)
- `[RESOLVED] OQ10 -- What is the ordinary map height formula? -> top uses `Level*height - Ground`; bottom uses `Level*(height+4) - Ground`.` (evidence: `0x0048455D..0x004845A0`)
- `[RESOLVED] OQ11 -- Are special lighting branches TS-only? -> Not proved TS-only; they are live conditional branches selected by global mode helpers.` (evidence: `0x0053A100`, `0x0053A110`, `0x0053B400`; fields read at `0x00484473..0x0048455D`)
- `[RESOLVED] OQ12 -- What are high clamps? -> RGB and ambient outputs high-clamp at `2000`.` (evidence: `0x004845A2..0x004845EB`; `0x005558E0..0x00555AB7`)
- `[RESOLVED] OQ13 -- What are low clamps? -> RGB inputs and final ambient outputs low-clamp to `0` by mask idiom.` (evidence: `0x005558EE..0x0055591F`; `0x004845ED..0x00484615`)
- `[RESOLVED] OQ14 -- Does RGB normalization happen per light? -> no, after all point-light contributions and after top high-clamp.` (evidence: `0x0048445F..0x004845CF`)
- `[RESOLVED] OQ15 -- Does RGB normalization affect ambient intensity? -> yes, it scales additive intensity by the computed 16.16 scale and final-clamps it.` (evidence: `0x00555A8F..0x00555AB7`)
- `[RESOLVED] OQ16 -- Is cache key exact RGB? -> no, `0x00555AC0` clamps to `0..1000` and masks low bits by DetailLevel before LightConvert lookup.` (evidence: `0x00555AC0`; `0x00544E70`)
- `[RESOLVED] OQ17 -- What current Rust function maps to falloff? -> `accumulate_point_lights` in `src/map/lighting.rs`.` (evidence: Codegraph; `src/map/lighting.rs:188..217`)
- `[RESOLVED] OQ18 -- Does Rust use binary default LightVisibility? -> no, Rust defaults it to `0`; binary constructor default is `5000`.` (evidence: `src/rules/object_type.rs:1104`; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md:88`)
- `[DEFERRED] OQ19 -- What exact palettes does `LightConvertClass__Constructor` generate from these keys?` (category: out-of-scope; reason: slot target is cell compute, not LightConvert internals; next-step-if-pursued: slot 2 should drain `0x00555DA0` and consumers)
- `[DEFERRED] OQ20 -- What owns the special Ion/Nuke/Dominator helper globals?` (category: requires-different-system-context; reason: formula branch is verified but owning transition systems are outside map-lamp cell compute; next-step-if-pursued: investigate `0x0053A090..0x0053B400` callers)
- `[DEFERRED] OQ21 -- Does `Math__ftol` round or truncate under every runtime FPU control state?` (category: bounded-cost-too-high; reason: all formula call sites are identified, but `0x007C5F00` was not audited in this slot; next-step-if-pursued: standalone math helper audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Point lights use lepton-center distance and inclusive radius guard, then zero contribution at exact edge | `0x004842B0..0x004843B8` | partial: Rust uses cell-space float distance and skips `dist >= range` | `src/map/lighting.rs::accumulate_point_lights` | Compute against lepton centers or tests equivalent to binary for all standard lamp radii | Unit test `map_lighting_lepton_center_radius_edge_is_zero` with a 256-lepton aligned source and edge cell | Do not compare cell units directly when `LightVisibility` is in leptons |
| Contribution is `(field * (((radius-distance)*1000)/radius))/1000` with integer ftol/truncation | `0x0048439A..0x0048443E` | mismatch risk: Rust uses f32 and clamps per channel per light | `src/map/lighting.rs::accumulate_point_lights` | Sum integer milli-unit source contributions before clamp/normalization | Unit test `map_lighting_negative_lamp_truncates_toward_zero` using `NEGLAMP`-like fields | Do not floor negative contributions; binary truncates toward zero |
| Source contributes only if `active && source_detail <= DetailLevel`; constructor detail is `2` | `0x00484294..0x004842AA`; `0x00554760`; `0x00554A60`; `0x005FA782` | missing | lighting collection/runtime light state surface | Add a render-detail gate and active/toggle state when dynamic lighting exists | Scenario test `map_lighting_lamp_suppressed_at_detail_0_enabled_at_2` | Do not treat all parsed lamps as always active if implementing retail options parity |
| Height formula has top and bottom ambient outputs; bottom uses `height+4` and is later scaled by RGB scale | `0x0048445F..0x004845DD` | missing: Rust stores one RGB tint grid | `src/map/lighting.rs`, render cell lighting interface | Preserve separate values if renderer needs `+0x10A/+0x10C/+0x10E` equivalents | Deterministic test `map_lighting_height_four_bottom_value_matches_binary_formula` | Do not collapse to one `cell_tint` if downstream draw paths need different top/bottom fields |
| RGB channels are normalized after all contributions; max can become `1000` while a 16.16 scale carries excess brightness | `0x005558E0..0x00555AB7` | missing | `src/map/lighting.rs::cell_tint` and point-light accumulation | Implement post-sum RGB normalization and scale output if LightConvert parity is required | Unit test `map_lighting_red_1_5_normalizes_to_rgb_key_and_scale` using `REDLAMP` center | Do not just clamp RGB floats to `2.0`; binary normalizes RGB and scales brightness separately |
| LightConvert cache keys are quantized by DetailLevel before lookup | `0x00555AC0`; `0x00544E70` | missing | future LightConvert/cache surface | Quantize RGB keys to multiples of 128/64/32 for detail 0/1/2 | Unit test `map_lighting_lightconvert_key_quantizes_by_detail_level` | Do not generate unbounded unique palettes for every one-unit RGB difference |
| Sentinel cells return neutral lighting and neutral LightConvert | `0x0048418A..0x00484675`; `0x00483E30` | unchecked | map bounds/sentinel cell handling | Preserve neutral fallback for invalid/sentinel cells | Unit test `map_lighting_sentinel_cell_returns_neutral` | Do not apply scenario ambience to sentinel fallback cells |

**Stale Docs / Follow-up Docs**

- Prior wording that Rust "matches the original engine's point light calculation" should be narrowed: linear falloff is confirmed, but the binary also uses lepton-center integer math, active/detail gates, post-sum normalization, and LightConvert quantization.
- Prior docs that imply `LightVisibility` alone makes a LightSource visible should be corrected to distinguish allocation (`LightIntensity != 0` at checked building paths) from per-cell contribution (`source+0x48` active and `+0x34 <= DetailLevel`).

## 10. Negative Facts / Do Not Do

- Do not model lamp contribution as Lightning Storm or superweapon lighting.
- Do not use `LightVisibility / 256.0` cell floats as the authoritative formula when exact parity is required; binary compares lepton-center coordinates.
- Do not clamp after each light contribution; binary accumulates then clamps/normalizes.
- Do not ignore negative intensity or tint values; stock INI has negative lamps.
- Do not assume an active `LightSourceClass` just because it exists in the global vector; constructor starts inactive.
- Do not ignore `[Options] DetailLevel`; it gates LightSource contribution and cache-key precision.
- Do not treat `ExtraLight=` as proven by this report.

## 11. Remaining Uncertainty

- `Math__ftol @ 0x007C5F00` was used as a black-box conversion helper. This report records where it is called and the integer math around it, but does not independently prove its rounding mode.
- `LightConvertClass__Constructor @ 0x00555DA0` was not investigated. This report verifies the RGB keys handed to it, not the palette tables it builds.
- Special mode helpers `0x0053A100`, `0x0053A110`, and `0x0053B400` were decompiled only enough to identify branch selection. Their owning systems and default gameplay timing belong to a separate lighting-transition investigation.
- Dirty scheduling cadence in `0x00554AF0`/`0x00554D50` was touched to prove liveness, but not exhausted for batching/tick-budget parity.

## Sources

- Ghidra decompiles: `0x00484180`, `0x00483E30`, `0x00484050`, `0x005558E0`, `0x00555AC0`, `0x00544E70`, `0x00554760`, `0x00554A60`, `0x00554A80`, `0x00554AA0`, `0x00554AF0`, `0x00554D50`, `0x0053A100`, `0x0053A110`, `0x0053B400`, `0x00568CB9`, `0x006851F0`, `0x00687AE3`, `0x005FA370`, `0x005FA782`.
- Local disassembly from retail `gamemd.exe`: `0x00484180..0x00484675`, `0x005558E0..0x00555AB7`, `0x00544E70..0x00544FFF`.
- INI checked: `ini/rulesmd.ini` lines 767-768, 3653-3657, 17214-17395.
- Rust checked: `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`, `src/rules/art_data.rs`.
- Prior docs checked: `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, `MAP_LIGHTING_AND_LIGHT_POSTS_SYSTEM_MODEL_SYNTHESIS.md`, `PIXEL_FX_CLASS_GHIDRA_REPORT.md`.
