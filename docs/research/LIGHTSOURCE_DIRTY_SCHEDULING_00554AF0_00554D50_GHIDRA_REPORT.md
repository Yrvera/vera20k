# LightSource Dirty Scheduling 00554AF0 / 00554D50 -- Ghidra Research Report

**Address(es):** `0x00554AF0`, `0x00554D50`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** LightSource enable/disable/reposition affected-cell scheduling, queued record layout, per-tick drain behavior, and immediate-vs-queued commit semantics for these two functions.  
**Non-Scope:** exact `LightConvertClass` palette construction internals, full map ambience parser, spotlight `BuildingLightClass`, `ExtraLight=`, and full radiation damage rules.  
**Confidence:** High for the two primary functions and immediate callees; Medium for global-vector ownership names.  
**Active in YR:** Yes. Building lamp posts and radiation light sites call this path in standard YR when their LightSource exists and is toggled, removed, or updated.

## 1. Target Question

What do `FUN_00554AF0` and `FUN_00554D50` do for LightSource dirty scheduling: do toggles immediately recompute cells or enqueue them, what record is queued, what bounds are affected, how is the queue drained, and is the path live in standard Yuri's Revenge?

## 2. Non-Goals

- Do not re-prove `BuildingClass+0x614` LightSource allocation or `BuildingClass+0x600` spotlight separation.
- Do not implement Rust code.
- Do not investigate Lightning Storm superweapon lighting beyond rejecting it as unrelated caller context.
- Do not fully reverse `LightConvertClass__Constructor`; only record how these functions request or reuse conversions.

## 3. Evidence Needed To Mark Complete

- Decompiled `0x00554AF0` and `0x00554D50`.
- Caller evidence for `0x00554AF0`, `0x00554D50`, and the wrappers `0x00554A60`, `0x00554A80`, `0x00554AA0`.
- Callee evidence for cell recompute and commit: `0x00484050`, `0x00483E30`, `0x00484180`, `0x00544E70`, `0x00555AC0`, `0x004F42F0`.
- Assembly-context evidence for the hidden fastcall arguments to `0x00554D50`.
- Rust scan for current lighting-grid lifecycle.

## 4. Stop Conditions

- Stop if a Ghidra function boundary is missing and would require mutating the Ghidra project.
- Stop if the analysis needs runtime debugger state to prove a branch that static decompile cannot settle.
- Stop after all open questions below are resolved or explicitly deferred.

## 5. Class Layout / Key Offsets

| Owner | Offset / global | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `LightSourceClass` | `+0x24` | Light intensity contribution, integer scaled by 1000 units. | `0x00484180` reads and adds it through linear falloff. | Yes |
| `LightSourceClass` | `+0x28`, `+0x2C`, `+0x30` | Red, green, blue tint contributions, scaled like intensity. | `0x00484180`, `0x00554AA0`, `0x00554760`. | Yes |
| `LightSourceClass` | `+0x34` | Detail/quality level gate; source contributes only if `+0x34 <= g_ExtraAnimationsEnabled`. | `0x00554AF0`, `0x00484180`. | Conditional: yes when Extra Animations detail allows it. |
| `LightSourceClass` | `+0x38`, `+0x3C`, `+0x40` | World X/Y/Z coordinate. X/Y are used for dirty radius and distance. | `0x00554AF0`, `0x00484180`. | Yes |
| `LightSourceClass` | `+0x44` | Visibility radius in leptons. | `0x00554AF0`, `0x00484180`, constructor `0x00554760`. | Yes |
| `LightSourceClass` | `+0x48` | Active/enabled byte. | `0x00554A60`, `0x00554A80`, `0x00484180`. | Yes |
| Pending record | `+0x00` | LightConvert pointer / processed sentinel. Initially zero; nonzero after `0x00484050`. | `0x00554AF0`, `0x00554D50`. | Conditional: only queued path. |
| Pending record | `+0x04` | Scale value, initialized to `0x10000`. | `0x00554AF0`, `0x00554D50`, `0x00483E30`. | Conditional |
| Pending record | `+0x08`, `+0x0A`, `+0x0C`, `+0x0E` | Four 16-bit computed cell-light/Z fields. Initial defaults are `0`, `1000`, `1000`, `1000`. | `0x00554AF0`, `0x00554D50`. | Conditional |
| Pending record | `+0x10` | Packed cell coordinate used for `MapClass::Get_CellClass`. | `0x00554AF0`, `0x00554D50`. | Conditional |
| Global queue | `DAT_00ABCA44` | Pointer array of pending 0x14-byte records. | `0x00554AF0`, `0x00554D50`. | Conditional |
| Global queue | `DAT_00ABCA50` | Pending record count. | `0x00554AF0`, `0x00554D50`. | Conditional |
| Global queue | `DAT_00ABCA7C` | Current descending preparation index. | `0x00554D50`. | Conditional |
| Global queue | `DAT_00ABCA80` | Remaining preparation count. | `0x00554D50`. | Conditional |
| Global queue | `DAT_00ABCA84` | Preparation-complete flag; commit pass may run only after this is set. | `0x00554D50`. | Conditional |
| Global tick | `DAT_00829AE4` | Master logic enable flag; `0x00554AF0` returns without work when false. | `0x00554AF0`, `SCENARIO_INIT_DEEP_DIVE.md`. | Yes; scenario init toggles it back on. |
| Global option | `g_ExtraAnimationsEnabled` / `0x00A8EB78` | Detail gate for LightSource contribution and color quantization. | `0x00554AF0`, `0x00484180`, `0x00555AC0`, `PIXEL_FX_SPARKLES_GHIDRA_REPORT.md`. | Conditional: user option/detail setting. |

## 6. Core Logic

### `0x00554A60` / `0x00554A80` / `0x00554AA0`

- `0x00554A60` enables a source: if `+0x48 == 0`, write `1` then call `0x00554AF0(update_mode)`.
- `0x00554A80` disables a source: if `+0x48 != 0`, write `0` then call `0x00554AF0(update_mode)`.
- `0x00554AA0` updates source fields `+0x24`, `+0x2C`, `+0x28`, `+0x30` and, if the source is active, calls `0x00554AF0(update_mode)`.
- Active in YR: Yes. Building online/offline/destructor/sell/death and RadSite activation/AI call these wrappers. Decompile evidence includes `0x00452260`, `0x00452480`, `0x0043BCF0`, `0x00449C30`, `0x00442230`, `0x0065B580`, `0x0065B800`.

### `0x00554AF0` affected-cell scan

`0x00554AF0` does nothing unless both conditions pass:

- `DAT_00829AE4 != 0`.
- `LightSource+0x34 <= g_ExtraAnimationsEnabled`.

Then it scans a square around the source:

- Center cell X/Y are `floor(source_x / 256)` and `floor(source_y / 256)`, using the signed-bias pattern `(value + (value >> 31 & 0xFF)) >> 8`.
- Loop radius is `floor(visibility_radius / 256) + 1`.
- The square loop is inclusive: `dx = -loop_radius..+loop_radius`, `dy = -loop_radius..+loop_radius`.
- Each candidate cell must satisfy `0 <= x < 512`, `0 <= y < 512`.
- `FUN_005657E0` must report a non-null map cell pointer.
- Final circular acceptance uses cell centers: `(cell_x * 256 + 128, cell_y * 256 + 128)` and accepts `sqrt(dx^2 + dy^2) <= LightSource+0x44`.

Active in YR: Yes. This is the dirty radius used whenever a live LightSource is toggled or updated.

### Immediate vs queued mode

`0x00554AF0`'s `param_2` selects the scheduling mode:

- `param_2 == 0`: immediate recompute/commit. For every accepted affected cell, it calls `MapClass::Get_CellClass` and then `0x00483E30` with neutral/default inputs, causing the cell to recompute from current map ambience and all active sources.
- `param_2 != 0`: queue mode. For every accepted affected cell, it allocates a 0x14-byte record, initializes it to a neutral unresolved state, stores the packed cell coord at `+0x10`, and appends it to `DAT_00ABCA44`.

Important ordering detail: if `param_2 != 0` and there is already a non-empty queue, `0x00554AF0` first forces `0x00554D50` with `ECX=0`, `DL=1`. Assembly at `0x00554B2A..0x00554B2E` verifies `MOV DL,0x1; XOR ECX,ECX; CALL 0x00554D50`. That flushes old queued work before this new scan appends records.

Active in YR: Immediate mode is Yes; all standard building/radiation callers decompiled in this pass pass `0`. Queued mode is Conditional; the code is live and drained by the logic tick, but no standard lamp/rad caller decompiled here passed nonzero.

### `0x00554D50` preparation and commit

`0x00554D50` is a two-stage queue processor.

Stage 1, preparation:

- Normal tick call passes `ECX=6`, `DL=0`. Assembly around `0x0055B5EA..0x0055B5F1` verifies `XOR DL,DL; MOV ECX,0x6; CALL 0x00554D50`.
- If `DAT_00ABCA50 != 0` and `DAT_00ABCA84 == 0`, it walks records backwards.
- If `DAT_00ABCA7C <= 0`, it initializes `DAT_00ABCA80 = count` and `DAT_00ABCA7C = count - 1`.
- For each record whose first dword is zero, it resolves the cell with `MapClass::Get_CellClass`, calls `0x00484050`, and writes the six computed outputs back into the record.
- The time-budget check runs only when `param_2 == 0` and `(index & 0x0F) == 0x0F`. If elapsed time has reached the `param_1` budget, it breaks and resumes later.
- When `DAT_00ABCA80 < 1`, it sets `DAT_00ABCA84 = 1`.

Stage 2, commit:

- Commit runs only when `DAT_00ABCA84 != 0` and either `param_2 != 0` or the preparation pass consumed roughly no time (`elapsed <= 1 ms` from the decompiled expression).
- Commit walks every record from index 0 upward, resolves the cell from record `+0x10`, calls `0x00483E30` with the record's cached LightConvert/scale/field values, frees the 0x14-byte record, and finally clears the queue.
- After commit it resets `DAT_00ABCA50 = 0`, optionally frees `DAT_00ABCA44` if the vector owns storage, clears `DAT_00ABCA4D`, `DAT_00ABCA48`, `DAT_00ABCA84`, and calls `0x004F42F0(1)`.
- Commit is all-or-nothing; the budget applies to preparation, not to the final commit loop.

Active in YR: Yes as a per-tick service after radiation-site AI and before EMPulse AI in `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.

### Cell recompute helper chain

- `0x00484050` computes the same output tuple into scratch/record form. It starts with defaults `scale=0x10000`, base `0`, and RGB/Z fields `1000`. It calls `0x00484180` unless the cell has the null/sentinel coordinate.
- `0x00484180` computes map ambience, active LightSource contributions, height terms, and clamp/normalization.
- `0x00483E30` commits the tuple into the cell. With `param_2 == 0`, it recomputes directly; with nonzero LightConvert pointer, it installs/references that LightConvert and writes cached fields.
- `0x00544E70` reuses or creates a `LightConvertClass` by clamped RGB tuple; neutral `(1000,1000,1000)` returns the first existing LightConvert if available.
- `0x00555AC0` clamps RGB tuple values to `0..1000` and quantizes by `g_ExtraAnimationsEnabled` (`& 0xFFFFFF80`, `& 0xFFFFFFC0`, `& 0xFFFFFFE0`, or no coarse mask for higher detail).

Active in YR: Yes. These helpers are reached from the LightSource scheduling path.

## 7. INI Keys

This slice does not parse INI directly. Relevant keys feed the LightSource fields before this scheduling path runs.

| Key | Source | Effect in this slice | Active in YR |
|---|---|---|---|
| `LightVisibility=` | `rules.ini` / `rulesmd.ini` building types | Becomes `LightSource+0x44`, controls affected-cell radius in leptons. | Yes for lamp building types. |
| `LightIntensity=` | `rules.ini` / `rulesmd.ini` building types | Becomes `LightSource+0x24`, contributes brightness and gates building allocation upstream. | Yes for lamp building types. |
| `LightRedTint=`, `LightGreenTint=`, `LightBlueTint=` | `rules.ini` / `rulesmd.ini` building types | Become `+0x28/+0x2C/+0x30`, color contributions. | Yes for lamp building types. |
| `RadLightDelay=` and radiation light rules fields | `rulesmd.ini` / `RulesClass` | Drive RadSite `0x00554AA0` updates through radiation light decay. | Yes when radiation sites exist. |
| `Extra Animations` option | Options data, not rules INI | Gates whether a source contributes and how colors are quantized. | Conditional, user/detail option. |

## 8. Integration Points

| Caller / callee | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::GoOnline @ 0x00452260` | Enables LightSource with immediate mode when building power returns. | Decompile calls `0x00554A60(0)` if `LightSource != 0`. | Yes |
| `BuildingClass::ApplyOfflineEffects @ 0x00452480` | Disables LightSource with immediate mode when building goes offline. | Decompile and assembly at `0x00452496` show `PUSH 0; CALL 0x00554A80`. | Yes |
| `BuildingClass::~BuildingClass @ 0x0043BCF0` | Disables source before deleting it. | Decompile calls `0x00554A80(0)` then virtual delete. | Yes |
| `BuildingClass::Sell @ 0x00449C30` | Disables LightSource during sale/deploy removal. | Decompile contains `if LightSource != 0 { 0x00554A80(0) }`. | Yes |
| `BuildingClass::ReceiveDamage @ 0x00442230` | Disables LightSource when damage result destroys/removes building. | Decompile case 4 calls `0x00554A80(0)`. | Yes |
| `BuildingClass::OnConstructionComplete @ 0x00445F80` | Allocates source if needed and immediately enables/recomputes. | Decompile calls constructor then `0x00554A60(0)`. | Yes |
| `BuildingClass::ReadFromINI @ 0x0044F820` | Map-load structures go online/offline and can immediately enable/disable source. | Decompile inlined GoOnline/ApplyOfflineEffects. | Yes during map load |
| `RadSiteClass::Activate @ 0x0065B580` | Creates LightSource and enables immediately. | Decompile calls constructor, clears detail field, `0x00554A60(0)`. | Yes when radiation appears |
| `RadSiteClass::AI @ 0x0065B800` | Updates light intensity/tints during radiation decay. | Decompile calls `0x00554AA0(...,0)`. | Yes when radiation exists |
| `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` | Drains queued records after RadSite AI, before EMPulse AI. | Decompile plus assembly `ECX=6`, `DL=0`. | Yes every game tick |
| `0x004F42F0` | Marks tactical dirty and optionally increments bridge/map counter. | Decompile writes `g_Tactical+0xD7D = 1`; if param nonzero, updates map counter. | Yes |

## 9. Current Rust Implementation Status

Current Rust has a static map-lighting pass at app initialization, not a live dirty scheduling pipeline.

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/map/lighting.rs::collect_building_lights` | Collects map-placed structure point lights once from rules data. | Missing active/inactive LightSource state, power/offline/destruction toggles, radiation light updates, detail-gated contribution. |
| `src/map/lighting.rs::accumulate_point_lights` | Applies linear point-light falloff over a startup `LightingGrid`. | Similar formula concept, but no cell cache, no LightConvert reuse/refcount, no dynamic dirty queue. |
| `src/app_init.rs` lighting setup | Builds `LightingGrid` once during app initialization. | Missing per-tick or event-driven invalidation when a lamp powers on/off, dies, sells, changes owner, or radiation fades. |
| `src/rules/object_type.rs` lighting fields | Parses `LightVisibility`, `LightIntensity`, and tint floats. | Parser/default fidelity belongs to another slot; this report only needs these fields as scheduler inputs. |

## 10. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00554AF0` immediate mode | verified | Full decompile; callers pass `0`; callee `0x00483E30`. | none |
| `0x00554AF0` queued mode | verified | Full decompile; record allocation layout; forced flush assembly. | Find a standard caller passing nonzero if one exists outside this caller set. |
| `0x00554AF0` radius bounds | verified | Full decompile constants `0x100`, `0x80`, `0x200`, `+1`, `<= radius`. | none |
| `0x00554D50` per-tick budget args | verified | Assembly `0x0055B5EA..0x0055B5F1`: `DL=0`, `ECX=6`. | none |
| `0x00554D50` forced flush args | verified | Assembly `0x00554B2A..0x00554B2E`: `DL=1`, `ECX=0`. | none |
| `0x00554D50` record preparation | verified | Full decompile, `0x00484050` decompile. | none |
| `0x00554D50` final commit/free/reset | verified | Full decompile. | none |
| `0x00484050` helper | verified | Full decompile. | Exact semantic names of all tuple fields are covered by older ZAdjust docs, not repeated here. |
| `0x00483E30` commit helper | touched-not-exhausted | Full decompile. | Full LightConvert lifecycle/refcount audit belongs to slot 2. |
| `0x00484180` formula helper | touched-not-exhausted | Full decompile. | Exact ambience formula belongs to map-lighting formula slot. |
| `0x00544E70` LightConvert lookup/create | touched-not-exhausted | Full decompile. | Constructor internals out of scope. |
| Rust dynamic lighting | verified absent at searched surfaces | Codegraph and `rg` over `src/map/lighting.rs`, `src/app_init.rs`. | Future implementation design. |

## 11. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Is 0x00554AF0 live in standard YR? -> Yes, building and RadSite callers reach it through 0x00554A60/80/AA0.` (evidence: callers/decompiles `0x00452260`, `0x00452480`, `0x00445F80`, `0x0065B580`, `0x0065B800`)
- `[RESOLVED] OQ-02 -- Does toggle param mean immediate or queued? -> `0` means immediate per-cell `0x00483E30`; nonzero means allocate queued records.` (evidence: `0x00554AF0`)
- `[RESOLVED] OQ-03 -- What is the affected radius? -> Inclusive square of `floor(radius/256)+1`, then circular filter `center-distance <= radius` in leptons.` (evidence: `0x00554AF0`)
- `[RESOLVED] OQ-04 -- Are map bounds hardcoded? -> Candidate cells require `0 <= x,y < 0x200` and an existing cell pointer.` (evidence: `0x00554AF0`, `0x005657E0`)
- `[RESOLVED] OQ-05 -- What is queued record size/layout? -> 0x14 bytes, fields at +0/+4/+8/+A/+C/+E/+10 as documented above.` (evidence: `0x00554AF0`, `0x00554D50`)
- `[RESOLVED] OQ-06 -- Is queued work committed immediately? -> Not normally; normal tick prepares with 6 ms budget, then commits only after prepared and if commit gate passes. Forced flush commits immediately.` (evidence: `0x00554D50`, assembly `0x0055B5EA..0x0055B5F1`, `0x00554B2A..0x00554B2E`)
- `[RESOLVED] OQ-07 -- Is commit time-budgeted per record? -> No. The preparation stage is budgeted; final commit walks all records and frees them.` (evidence: `0x00554D50`)
- `[RESOLVED] OQ-08 -- When in the tick does the queue drain? -> After LightningStorm, RadSite AI, and before EMPulse AI in LogicClass.` (evidence: `0x0055AFB0`)
- `[RESOLVED] OQ-09 -- Does disabling a source recompute using old or new active state? -> New state. The wrapper writes `+0x48` before calling `0x00554AF0`; `0x00484180` reads current active flags.` (evidence: `0x00554A60`, `0x00554A80`, `0x00484180`)
- `[RESOLVED] OQ-10 -- Does moving/updating a source dirty only if active? -> Yes; `0x00554AA0` calls `0x00554AF0` only when `+0x48 != 0`.` (evidence: `0x00554AA0`)
- `[RESOLVED] OQ-11 -- What tactical dirty signal is emitted? -> `0x004F42F0` sets `g_Tactical+0xD7D = 1`; with nonzero param it also updates map/bridge counter state.` (evidence: `0x004F42F0`)
- `[RESOLVED] OQ-12 -- Does Rust have equivalent dynamic scheduling? -> No; lighting grid is built once at app init.` (evidence: Codegraph context; `src/map/lighting.rs`, `src/app_init.rs`)
- `[RESOLVED] OQ-13 -- Null map cells? -> Queue/immediate scan skips cells outside bounds or without map cell pointer; commit uses fallback `MapClass::Get_CellClass` for a queued coordinate.` (evidence: `0x00554AF0`, `0x005657E0`, `0x005657A0`)
- `[RESOLVED] OQ-14 -- Allocation failure behavior? -> `0x00554AF0` does not guard record pointer before vector append once allocation returns null; later drain would dereference it. Engine assumes allocation succeeds.` (evidence: `0x00554AF0`, `0x00554D50`)
- `[RESOLVED] OQ-15 -- TS legacy filter? -> No TS-only gate found in these functions. The active gates are scenario logic enable and Extra Animations detail.` (evidence: `0x00554AF0`, `SCENARIO_INIT_DEEP_DIVE.md`, `PIXEL_FX_SPARKLES_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-16 -- Does any standard YR caller pass nonzero queued mode into 0x00554A60/80/AA0?` (category: bounded-cost-too-high; reason: all direct callers decompiled in this slice pass `0`, but proving no indirect or rare path exists would require callsite assembly audit for every wrapper caller; next-step-if-pursued: xref all wrapper calls and record pushed update-mode operand)
- `[DEFERRED] OQ-17 -- Exact LightConvert constructor palette internals.` (category: out-of-scope; reason: slot 2 owns LightConvert cache/palette construction; next-step-if-pursued: drain `0x00544E70` constructor path)

## 12. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| LightSource toggles recompute affected cells after writing the active flag, so disable removes its contribution immediately. | `0x00554A60`, `0x00554A80`, `0x00484180` | Missing | Needed dynamic lighting state near map/render boundary; current `src/map/lighting.rs` is static. | Event-driven light invalidation must use post-toggle active state. | `test_name: lamp_power_off_recomputes_cells_without_own_contribution` | Do not subtract a cached contribution blindly; binary recomputes from all currently active sources. |
| Immediate mode scans `floor(radius/256)+1` square and accepts circular center distance `<= radius`. | `0x00554AF0` | Partial/static approximation uses cell-space `ceil(range)` and excludes `dist >= range`. | `src/map/lighting.rs::accumulate_point_lights` | Match lepton center-distance bounds and inclusive radius when implementing parity mode. | `test_name: light_radius_includes_boundary_cell_center_equal_visibility` | Do not use only cell-index distance or exclusive `< radius` if targeting binary parity. |
| Normal queue drain prepares records with a 6 ms budget after RadSite AI and before EMPulse AI. | `0x0055AFB0` assembly and decompile | Missing | Future per-tick app/render lighting invalidation service. | Large queued light refresh can span ticks before final commit. | `test_name: queued_light_refresh_prepares_with_budget_before_commit` | Do not run an unbounded synchronous full-map rebuild for queued mode. |
| Final commit is all-or-nothing after preparation completes; it calls `0x00483E30` for every queued cell, frees records, clears queue, then marks tactical dirty. | `0x00554D50`, `0x004F42F0` | Missing | Rendering invalidation / lighting cache service. | Batched queued refresh should not expose half-committed cells. | `test_name: queued_light_commit_is_atomic_after_preparation_complete` | Do not commit each prepared record immediately if modeling the original queue. |
| Building online/offline/destruction/sell paths use immediate mode in decompiled standard callers. | `0x00452260`, `0x00452480`, `0x0043BCF0`, `0x00449C30`, `0x00442230` | Missing | Sim building lifecycle plus render-light bridge. | Powering a lamp off/on changes nearby cell lighting in the same event/tick path. | `test_name: building_light_lifecycle_online_offline_destroy_updates_lighting` | Do not leave map lighting frozen after app init. |
| Radiation sites use the same LightSource update machinery; their AI updates intensity/tint over time with immediate mode in decompiled caller. | `0x0065B580`, `0x0065B800` | Missing/unchecked | Radiation system and lighting bridge. | Radiation glow fades/updates cells as radiation site AI runs. | `test_name: radiation_light_decay_updates_lighting_each_rad_delay` | Do not implement lamp-only dynamic lighting if radiation light parity is in scope. |
| Extra Animations detail gates both whether a source contributes and how LightConvert colors are quantized. | `0x00554AF0`, `0x00484180`, `0x00555AC0` | Missing | Options/config plus lighting conversion. | Lower detail settings can reduce or disable light-source visual contribution. | `test_name: extra_animations_detail_gates_light_source_contribution` | Do not treat `Extra Animations` as unrelated eye candy for this path. |

### Negative Facts / Do Not Do

- Do not model lamp updates as a one-time map-load bake only.
- Do not dirty/recompute cells before flipping `LightSource+0x48`; the binary flips first.
- Do not use `LightVisibility > 0` alone as the live contribution gate; contribution also requires active byte and detail gate.
- Do not mix `BuildingLightClass` spotlights with this path; this report only covers LightSource scheduling.
- Do not make `sim/` depend on render lighting caches. The binary has visual cache behavior here, but VERA20k must preserve the sim/render boundary.
- Do not commit queued records one at a time if implementing binary-style queue semantics; binary prepares incrementally and commits all records together.

### Stale Docs / Follow-up Docs

- `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md` correctly identifies `0x00554D50` as time-budgeted incremental cell recalc, but its wording "INI-reload / rules-change cell refresh" is too narrow. Replacement wording: "`0x00554D50` drains the LightSource/cell-light pending record queue used by queued cell attribute refresh; LightSource immediate callers commonly bypass it, but the service is tick-integrated after RadSite AI."
- Any Rust comment claiming the current startup `LightingGrid` matches dynamic RA2 lamp behavior should be narrowed to "static approximation" until this queue/invalidation path exists.

## 13. Remaining Uncertainty

- No decompiled standard building/radiation caller in this slice passed nonzero queued mode. The queued path is still real and tick-integrated, but its normal YR producer outside this caller set remains unproven.
- Exact `LightConvertClass` palette construction and reference-count lifetime belong to the parallel LightConvert slot.
- Exact game-option default for `g_ExtraAnimationsEnabled` should be taken from the options/defaults report or a fresh option-system audit before tests assume a default detail level.

## Sources

- Ghidra decompiled: `0x00554AF0`, `0x00554D50`, `0x00554A60`, `0x00554A80`, `0x00554AA0`, `0x0055AFB0`, `0x00483E30`, `0x00484050`, `0x00484180`, `0x00544E70`, `0x00544FF0`, `0x00555AC0`, `0x005657A0`, `0x005657E0`, `0x004F42F0`, `0x005B1E40`.
- Ghidra caller spot-checks: `0x00452260`, `0x00452480`, `0x0043BCF0`, `0x00449C30`, `0x00442230`, `0x00445F80`, `0x0044F820`, `0x00448260`, `0x0065B580`, `0x0065B800`.
- Assembly contexts: `0x00554B2A..0x00554B2E`, `0x0055B5EA..0x0055B5F1`, `0x00452496..0x00452498`.
- Existing docs referenced: `MAP_LIGHTING_AND_LIGHT_POSTS_SYSTEM_MODEL_SYNTHESIS.md`, `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`, `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`, `SCENARIO_INIT_DEEP_DIVE.md`, `PIXEL_FX_SPARKLES_GHIDRA_REPORT.md`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`.
- Rust scanned: `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`; Codegraph context for `LightingGrid`, `collect_building_lights`, `accumulate_point_lights`.
