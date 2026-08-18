# GACNST Deploy Placement Blocker Taxonomy - Ghidra Research Report

**Address(es):** `0x00716150`, `0x0047C620`, supporting `0x005FDD20`, `0x0047E040`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** AMCV -> GACNST target-footprint rejection taxonomy through `0x00716150 -> 0x0047C620`: `CellClass+0x140` bits `0x100` and `0x400`, overlay exceptions, slope, bridge structural cells, and Buildable/LandType fallback.  
**Non-Scope:** general sidebar placement UI, full `BuildingTypeClass::CanBePlacedAt`, deploy-facing, construction-yard post-spawn lifecycle, AddOccupy/RemoveOccupy, and generic pathing walkability.  
**Confidence:** High for active placement blockers and ordering; Medium for the human-readable name of `0x400`.  
**Active in YR:** Yes. Stock YR has `[AMCV] DeploysInto=GACNST`, `[GACNST] ConstructionYard=yes`, and `artmd.ini [GACNST] Foundation=4x4`.

## 1. Overview

AMCV -> GACNST deploy validates the target building type's base foundation cells before creating the building. `0x00716150` walks the `GACNST` foundation offset list and calls `Cell_passability_building_placement @ 0x0047C620` for each cell.

For ordinary stock `GACNST`, the decisive per-cell fallback is: no blocking overlay, no bridge structural bit `0x100`, no bridge inactive/fallback bit `0x400`, `SlopeIndex == 0`, then LandType `Buildable=`. Unit pathing walkability is not consulted as the source of truth for this footprint gate.

## 2. Key Offsets

| Offset / field | Placement meaning | Evidence | Active in YR |
|---|---|---|---|
| BuildingType vtable `+0x90` | Returns base foundation offsets, sentinel `(0x7FFF,0x7FFF)` | `0x00716183..0x007161BB` | Yes |
| BuildingType vtable `+0xA8` | Target-footprint validator, concrete `0x00716150` | call from deploy path and vtable xrefs | Yes |
| BuildingType `+0x67C` | SpeedType-like parameter passed to `0x0047C620`; stock ordinary building path reaches `-1` Buildable fallback | `0x007161EF`, `0x0047C9D1` | Yes |
| BuildingType `+0x16BF` | `LaserFence=` special overlay branch, not stock `GACNST` | `0x0047C957`, prior laser-fence report | Conditional; No for stock `GACNST` |
| Cell `+0x44` | Overlay type index, `-1` means no overlay | `0x0047C8BE..0x0047C94F` | Yes |
| Cell `+0x11C` | SlopeIndex; nonzero rejects ordinary placement | `0x0047C98D`, `0x0047C9F3` | Yes |
| Cell `+0x11E` | Overlay/wall state byte; `> 0x0F` enables limited wall/gate replacement exceptions | `0x0047C926` | Conditional |
| Cell `+0x124` | Ground occupation flags; low six bits block ordinary placement | `0x0047C88D` branch in decompile | Yes |
| Cell `+0x140 & 0x100` | Bridge structural/live bridge-surface bit; blocks terrain fallback placement | `0x0047C97B`, `0x0047C9E1` | Yes |
| Cell `+0x140 & 0x400` | Bridge inactive/fallback endpoint marker; also blocks terrain fallback placement | `0x0047C984`, `0x0047C9EA`; writer `0x0047E040` | Yes |
| Cell `+0xEC` | LandType row index for speed/Buildable table | `0x0047CA33`, `0x0047CA4D` | Yes |

## 3. Core Logic

### 3.1 Foundation walk at `0x00716150`

`0x00716150` rejects the sentinel target cell immediately if it matches the global invalid coordinate. Otherwise it gets the base foundation cell list from vtable `+0x90` and loops until `(0x7FFF,0x7FFF)`.

For each offset, it adds the target origin cell, calls `MapClass__Get_CellClass`, and if the type's `WhatAmI()` result is `7`, calls `0x0047C620` with the candidate cell, type speed field, building type pointer, and owner. A false return marks the foundation as blocked. Any blocked cell makes the final return false unless the special `BuildingType+0xE58` flag path returns the accumulated state.

Active in YR: Yes. This is the target `GACNST` building type virtual used by AMCV deploy.

### 3.2 Ordinary stock GACNST path through `0x0047C620`

For stock `GACNST`, the material ordinary placement path is:

1. Map editor global mode bypasses; normal gameplay does not.
2. Existing object/building blockers and `Cell+0x124 & 0x3F` reject before terrain fallback.
3. If `Cell+0x44 != -1`, normal gameplay rejects nonempty overlays unless a special wall/gate/laser-fence/tiberium branch accepts first.
4. If no overlay blocks and the passed speed type is `-1`, the terrain fallback requires:
   - `(Cell+0x140 & 0x100) == 0`
   - `(Cell+0x140 & 0x400) == 0`
   - `Cell+0x11C == 0`
   - then returns LandType table Buildable byte.
5. If speed type is not `-1`, the function accepts when the speed-vs-LandType table entry is nonzero instead of using the Buildable column.

Active in YR: Yes. Stock `GACNST` is an ordinary non-naval construction-yard building and has no `LaserFence=`/`WaterBound=` exception.

### 3.3 Overlay taxonomy

`Cell+0x44 == -1` is the clean ordinary terrain path. Nonempty overlays normally reject gameplay placement before terrain fallback.

Exceptions are narrow and mostly not stock `GACNST` behavior:

- Overlay type `2` or `0`: may accept if the requested building's `BuildingType+0xE54` overlay pointer matches and `Cell+0x11E > 0x0F`, or if the requested type is one of three rules-global wall/gate pointers and owner matches.
- Overlay type `0x1A`: same shape, with two different rules-global special pointers.
- `LaserFence=` building types can accept overlay `0x7E` or a tiberium overlay if the cell has no `0x100`, no `0x400`, and zero slope.
- In normal gameplay, any other nonempty overlay rejects. The map-editor overlay fallback is editor-only.

Active in YR: Conditional for exceptions; ordinary overlay rejection is active for stock `GACNST`.

### 3.4 Bridge and slope taxonomy

Bridge structural cells reject by `Cell+0x140 & 0x100`. The `0x400` bit also rejects independently. Prior targeted bridge-flag work verified `0x400` is written by `CellClass::SetBridgeDirection_NESW/NWSE` when the bridge state argument is zero and cleared when bridge marking uses state `1`; the best behavior-derived name is bridge inactive/fallback endpoint marker.

Slope rejection is byte-based and independent of mixed cell height: any nonzero `Cell+0x11C` rejects ordinary placement, but there is no all-foundation-cells-same-height comparison in this path.

Active in YR: Yes. These are normal bridge/slope cell flags and the live per-cell placement predicate reads them.

### 3.5 Buildability and LandType

With speed type `-1`, ordinary non-naval placement reaches the Buildable column at `0x0089EA60`, indexed from `Cell+0xEC` by a 9-slot row stride. With speed type not `-1`, the function uses the speed-vs-LandType float table at `0x0089EA40` and accepts nonzero entries.

For stock `GACNST`, the relevant effect is Buildable-based land legality after bridge/overlay/slope blockers are clear. `[Clear]` and `[Road]` are buildable; `[Water]` and `[Rock]` are not.

Active in YR: Yes.

## 4. INI Keys

| Key / section | Retail YR value | Effect in this slice |
|---|---|---|
| `[AMCV] DeploysInto` | `GACNST` | Selects target building type for the `+0xA8` footprint virtual |
| `[GACNST] Foundation` | `4x4` in `artmd.ini` | Selects base foundation cell list walked by `0x00716150` |
| `[GACNST] ConstructionYard` | `yes` | Makes the unit-to-yard deploy path active; not a footprint blocker by itself |
| `[GACNST] WaterBound` / `Naval` | absent/default false | Stock `GACNST` uses ordinary land Buildable fallback |
| `[GACNST] LaserFence` | absent/default false | Stock `GACNST` does not use the laser-fence overlay exception |
| LandTypes `Buildable=` | per LandType in rules data | Used only after overlay, bridge bits, and slope are clear |

## 5. Current Rust Implementation Status

`Simulation::deploy_mcv` in `src/sim/world/world_spawn.rs` resolves `DeploysInto`, computes the yard origin, checks structure overlap, then calls `effective_build_blocked` for every foundation cell before despawning the AMCV. Current tests cover mixed-height acceptance and a structure blocker.

`Simulation::effective_build_blocked` returns true for non-destroyed bridge runtime cells and otherwise uses `ResolvedTerrainCell.build_blocked`; destroyed bridge cells fall back to `base_build_blocked`. This may miss the binary's pure `0x400` bridge inactive/fallback endpoint marker if Rust only models walkable/non-walkable bridge state.

Ready-building `cell_placeable` has a stricter explicit check set (`!build_blocked`, `!overlay_blocks`, `!terrain_object_blocks`, `!has_bridge_deck`, `!bridge_walkable`, `slope_type == 0`) than `deploy_mcv`, which currently relies on `effective_build_blocked` for the non-structure taxonomy. Whether `resolved_terrain.build_blocked` fully carries overlay, slope, and LandType Buildable for deploy should be covered by acceptance tests.

## 6. Coverage Ledger

| Area / branch | Status | Evidence | What remains |
|---|---|---|---|
| AMCV target `+0xA8` foundation walk | verified | `0x00716150`; prior AMCV deploy report | none for scoped cells |
| Per-cell ordinary terrain fallback | verified | `0x0047C620`, assembly `0x0047C9D6..0x0047CA40` | none |
| `Cell+0x140 & 0x100` placement rejection | verified | `0x0047C97B`, `0x0047C9E1` | none |
| `Cell+0x140 & 0x400` placement rejection | verified | `0x0047C984`, `0x0047C9EA`; writer `0x0047E040` | exact Westwood symbol name unknown |
| Overlay exception taxonomy | verified for branch shape | `0x0047C8BE..0x0047C9CD`, `0x005FDD20` | full wall/gate placement UI visuals out-of-scope |
| Slope rejection | verified | `0x0047C98D`, `0x0047C9F3` | none |
| Buildable/LandType fallback | verified | `0x0047CA33..0x0047CA40` | no runtime fixture run |
| Rust current deploy blocker mapping | touched-not-exhausted | `world_spawn.rs`, `world/mod.rs`, `production_placement.rs` scan | acceptance tests needed for each blocker class |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `0x00716150 -> 0x0047C620` live for stock AMCV -> GACNST? -> Yes, target building type footprint validation walks `GACNST` foundation cells before building creation.` (evidence: `0x00716150`; `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-2 - Does a bridge structural foundation cell reject? -> Yes, `Cell+0x140 & 0x100` rejects before Buildable lookup.` (evidence: `0x0047C97B`, `0x0047C9E1`)
- `[RESOLVED] OQ-3 - Does `Cell+0x140 & 0x400` reject? -> Yes, independently of `0x100`; it is a bridge inactive/fallback marker in prior writer audit.` (evidence: `0x0047C984`, `0x0047C9EA`, `0x0047E040`)
- `[RESOLVED] OQ-4 - Do normal overlays reject GACNST deploy? -> Yes. Nonempty overlay cells reject in gameplay unless a narrow special branch accepts first; stock `GACNST` does not use those exception flags.` (evidence: `0x0047C8BE..0x0047C9CD`; `rulesmd.ini [GACNST]`)
- `[RESOLVED] OQ-5 - Are tiberium/laser-fence overlay exceptions stock GACNST behavior? -> No. The branch requires `BuildingType+0x16BF` (`LaserFence=`), which stock `GACNST` does not set.` (evidence: `0x0047C957..0x0047C995`; `rulesmd.ini [GACNST]`)
- `[RESOLVED] OQ-6 - Does slope block? -> Yes, any nonzero `Cell+0x11C` rejects ordinary placement.` (evidence: `0x0047C98D`, `0x0047C9F3`)
- `[RESOLVED] OQ-7 - Does mixed height block? -> No same-height comparison is present in this scoped path; slope is separate from level mismatch.` (evidence: `0x00716150`, `0x0047C620`; prior mixed-height reports)
- `[RESOLVED] OQ-8 - Does ordinary GACNST use unit pathing walkability? -> No. It reaches LandType Buildable after placement-specific overlay/bridge/slope checks.` (evidence: `0x0047CA33..0x0047CA40`)
- `[DEFERRED] OQ-9 - Exact original symbol/name for `Cell+0x140 & 0x400`.` (category: requires-different-system-context; reason: behavior and writers are verified, original naming is not; next-step-if-pursued: whole cell-flag naming audit)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock `GACNST` deploy rejects any foundation cell with bridge structural `0x100` or bridge inactive/fallback `0x400` before Buildable lookup | `0x0047C9D6..0x0047C9ED`; `0x0047E040` | partial/unchecked for pure `0x400` marker cells | `src/sim/world/world_spawn.rs::deploy_mcv`; `Simulation::effective_build_blocked`; bridge runtime terrain mapping | Reject deploy on live bridge and inactive/fallback bridge endpoint marker cells without consuming AMCV | AMCV attempts deploy with one 4x4 footprint cell tagged as live bridge or inactive/fallback bridge marker; result false, AMCV remains, no GACNST | Do not use unit pathing walkability or bridge_walkable alone as the placement truth |
| Stock `GACNST` deploy rejects normal nonempty overlays; special wall/gate/laser-fence exceptions are not normal GACNST behavior | `0x0047C8BE..0x0047C9CD`; `rulesmd.ini [GACNST]` | unchecked in `deploy_mcv` acceptance coverage | `ResolvedTerrainCell.overlay_blocks`; `deploy_mcv` tests | Ensure overlay-blocked foundation cells reject AMCV deploy while clear mixed-height cells still accept | AMCV deploy over ore/wall/generic overlay cell rejects and keeps AMCV; clear mixed-height terrain still deploys | Do not port laser-fence/tiberium exception as a blanket GACNST overlay allowance |
| Stock `GACNST` deploy rejects nonzero slope and nonbuildable LandTypes; it does not reject mixed-height clear cells | `0x0047C9F3..0x0047CA40`; prior mixed-height report | mixed-height covered; slope/nonbuildable deploy tests missing/unchecked | `resolved_terrain`, `deploy_mcv`, deploy tests | Cover slope and LandType Buildable separately from height equality | AMCV footprint with one slope cell rejects; footprint on `[Water]`/`[Rock]` rejects; same cells at different clear levels accept | Do not reintroduce an all-foundation same-height gate |

## 9. Negative Facts / Do Not Do

- Do not treat `UnitClass` pathing walkability as the source of truth for AMCV -> GACNST footprint placement. Evidence: `0x0047C620` uses placement-specific flags and Buildable/LandType.
- Do not conflate `Cell+0x140 & 0x400` with A* bridge-approach `0x40000`. Evidence: prior pathfinder report and placement reader use different bits.
- Do not allow ordinary overlays under stock `GACNST` because laser-fence/tiberium exceptions exist. Stock `GACNST` lacks the required exception flags.
- Do not reject mixed-height clear foundations. No same-height gate exists in `0x00716150` or `0x0047C620`.
- Do not consume the AMCV on any scoped placement rejection. The target-footprint gate runs before building allocation/despawn success.

## 10. Remaining Uncertainty

- Exact original symbolic name for `Cell+0x140 & 0x400` remains unknown; behavior is verified as bridge inactive/fallback endpoint marker and placement blocker.
- This report did not run a runtime fixture in gamemd.exe; it is static Ghidra-backed plus INI/Rust scan.
- Rust's exact projection from bridge damage state into `resolved_terrain.build_blocked` / `effective_build_blocked` needs implementation-side acceptance tests, especially pure `0x400` marker cells.

## 11. Stale Docs / Follow-up Docs

- `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`: replace "Exact semantic name of `CellClass+0x140 bit 0x400` deferred" with "`CellClass+0x140 bit 0x400` is a bridge inactive/fallback endpoint marker, written by `CellClass::SetBridgeDirection_NESW/NWSE` when the bridge state argument is zero and cleared by live bridge marking with state `1`. It blocks `0x0047C620` ordinary terrain fallback placement independently of `0x100`; it is not the A* bridge-approach `0x40000` bit."
- `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`: refine "Bridge: `Cell+0x140 & 0x100` and `& 0x400` reject placement fallback" to "Bridge: `Cell+0x140 & 0x100` rejects live/structural bridge cells and `Cell+0x140 & 0x400` rejects bridge inactive/fallback endpoint marker cells before slope and Buildable/LandType fallback."
- `traces/IMPLEMENTATION_MCV_DEPLOY_MIXED_HEIGHT_TRACE_RERUN_2026-05-21.md`: retain the stale-doc correction from the AMCV predicate report: current Rust no longer has the same-height gate; remaining acceptance coverage should target overlay, slope, nonbuildable LandType, live bridge, and pure `0x400` bridge marker cells.

## Sources

- Ghidra decompile: `FUN_00716150 @ 0x00716150`
- Ghidra decompile and assembly context: `Cell_passability_building_placement @ 0x0047C620`
- Ghidra decompile: `CellClass__OverlayToTiberiumIndex @ 0x005FDD20`
- Ghidra decompile: `CellClass::SetBridgeDirection_NESW @ 0x0047E040`
- Prior docs: `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`, `CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`, `CELLCLASS_0X140_BIT_0X400_PATHGRID_SEMANTIC_GHIDRA_REPORT.md`, `BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini [AMCV]`, `ini/rulesmd.ini [GACNST]`, `ini/artmd.ini [GACNST]`
- Rust scanned: `src/sim/world/world_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/production/production_placement.rs`, `src/sim/deploy_tests.rs`
