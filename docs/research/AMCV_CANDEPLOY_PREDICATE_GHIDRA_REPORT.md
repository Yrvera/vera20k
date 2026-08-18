# AMCV CanDeploy Predicate - Ghidra Research Report

**Address(es):** `0x007393C0`, `0x00700D50`, `0x00716150`, `0x0047C620`, `0x0045EE70`, `0x00440580`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR `AMCV -> GACNST` deploy validation and failure side effects, including UnitClass vtable `+0x314` and the `EVA_CannotDeployHere` failure branch around `UnitClass::Deploy @ 0x00739502`.  
**Non-Scope:** Deploy-facing rotation, GACNST post-deploy construction-yard special branch, free harvester spawn, ConYard undeploy, slave miner deploy specifics, and exact names for unrelated non-AMCV deployer branches.  
**Confidence:** High for AMCV material blockers and failure side effects; Medium for human-readable names of several generic Techno/House fields not needed by stock AMCV.  
**Active in YR:** Yes. Retail YR has `[AMCV] DeploysInto=GACNST`, `[GACNST] ConstructionYard=yes`, and `artmd.ini [GACNST] Foundation=4x4`.

## 1. Overview

AMCV deploy validation is split across multiple gates. `UnitClass::Deploy @ 0x007393C0` first calls UnitClass vtable `+0x314`, which resolves to `0x00700D50`; for stock AMCV this is a generic deploy-readiness / attachment / bridge-adjacency gate, not the full GACNST footprint validator.

The actual 4x4 GACNST foundation validation happens immediately after origin calculation through the target building type vtable `+0xA8`, concrete `FUN_00716150 @ 0x00716150`, which walks base foundation offsets and calls `Cell_passability_building_placement @ 0x0047C620` per cell. Failure of that target type gate reaches `EVA_CannotDeployHere @ 0x0082012C` at `UnitClass::Deploy @ 0x00739502`, restores deploy state, and returns failure without consuming the AMCV.

## 2. Class Layout / Key Offsets

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| UnitClass vtable `0x007F5C70 + 0x314` | deploy predicate pointer `0x00700D50` | `inspect_memory_content 0x007F5F80`, dword at `0x007F5F84`; call `0x007393CC` | Yes |
| TechnoType `+0x404` | `DeploysInto` building type pointer | `0x007394C5`; `[AMCV] DeploysInto=GACNST` | Yes |
| TechnoType `+0x5EC` | deploy-target mode flag that suppresses EVA when nonzero | `0x007394EF..0x0073950A` | Conditional; stock AMCV follows default/zero behavior |
| Unit/Techno `+0x674` | locomotor/deploy interface pointer | asserts and interface calls at `0x007393EA..0x00739408`, `0x0073954D..0x00739565` | Yes |
| Unit/Techno `+0x684` | signed deploy state/readiness byte checked by `0x00700D50` | `0x00700E59..0x00700E61` | Yes |
| Unit/Techno `+0x694` | attached-object/deploy blocker | `0x00700EB4..0x00700EBC` | Conditional |
| Unit/Techno `+0x2C0` | power-link / attached powered unit blocker for ConstructionYard-style deploy | `0x00700EC6..0x00700ED8` after target `+0x16B9` check | Conditional |
| BuildingType `+0x16B9` | ConstructionYard / deployable building flag on target type | `0x00700EC6`; `[GACNST] ConstructionYard=yes` | Yes |
| CellClass `+0x124` | ground occupation bits; low six bits block ordinary building placement | `0x0047C620` | Yes |
| CellClass `+0x140` bit `0x100` | bridge structural flag | `0x00701013..0x00701064`, `0x0047C620` | Yes |
| CellClass `+0x140` bit `0x400` | placement-blocking cell flag | `0x0047C620` | Yes |
| CellClass `+0x11C` | slope index; nonzero blocks ordinary placement | `0x0047C620` | Yes |
| CellClass `+0x44` | overlay type index; nonempty overlays normally block | `0x0047C620`, `0x0045EE70` | Yes |
| CellClass `+0xEC` | LandType row; ordinary placement uses the Buildable column when speed type is `-1` | `0x0047C620`; LandType INI | Yes |

## 3. Core Logic

### UnitClass vtable `+0x314`

`UnitClass::Deploy` starts with `CALL [this.vtable + 0x314]`; if false it returns `0` before target building validation, allocation, EVA, or unit consumption. UnitClass vtable `+0x314` is `0x00700D50`. Ghidra lacks a function boundary there, so evidence is raw assembly `0x00700D50..0x007010CA`.

For the AMCV branch (`vtable+0x2C == 1`), material checks are:

- generic status virtual `+0x37C` true -> false (`0x00700E47..0x00700E53`);
- signed byte `unit+0x684` not negative -> false (`0x00700E59..0x00700E61`);
- `type+0x404 DeploysInto` present -> building deploy branch (`0x00700E67..0x00700EA4`);
- `unit+0x694` nonzero -> false (`0x00700EB4..0x00700EBC`);
- if target `BuildingType+0x16B9` is set, `unit+0x2C0` must be zero (`0x00700EC6..0x00700ED8`);
- current cell plus four direction-table neighbor cells must not have `CellClass+0x140 & 0x100` (`0x00700F00..0x00701064`);
- otherwise return true at `0x007010C1..0x007010CA`.

Active in YR: Yes. This is the concrete virtual called by stock AMCV deploy.

### Target Footprint Gate

After computing the target origin, `UnitClass::Deploy` calls the target building type vtable `+0xA8` at `0x007394D2`. The concrete building-type implementation is `FUN_00716150 @ 0x00716150`.

`FUN_00716150` walks the base foundation offset list from vtable `+0x90`, adding each offset to the target origin until sentinel `(0x7FFF,0x7FFF)`. For building types (`WhatAmI()==7`), every base foundation cell is validated by `Cell_passability_building_placement @ 0x0047C620`. If any foundation cell fails, the virtual returns false.

Active in YR: Yes. GACNST is a stock building type with `Foundation=4x4`.

### Player-Visible Blockers

| Blocker | Verified binary behavior | Evidence | Active in YR |
|---|---|---|---|
| Occupied cells | existing building/object blockers in `Cell+0xE4`, nearest visible object, RTTI `0x24`, and `Cell+0x124 & 0x3F` reject ordinary building placement | `0x0047C620`; `0x00716150` foundation walk | Yes |
| Mixed height | no all-foundation-cells-same-height comparison exists in scoped active validators | `0x00716150`, `0x0047C620`, `0x0045EE70`, `0x00440580` | Yes: absence is active |
| Slope | nonzero `Cell+0x11C` rejects ordinary placement | `0x0047C620` | Yes |
| Bridge | `Cell+0x140 & 0x100` and `& 0x400` reject placement fallback; `+0x314` also rejects `0x100` around the unit | `0x0047C620`; `0x00700F00..0x00701064` | Yes |
| Overlay | nonempty overlays normally reject; wall/gate/laser-fence exceptions are conditional and not normal GACNST deploy behavior | `0x0047C620`, `0x0045EE70` | Yes for ordinary overlay rejection |
| Buildability | ordinary non-naval building placement with speed type `-1` uses the LandType `Buildable=` column; `[Clear]`/`[Road]` are buildable, `[Water]`/`[Rock]` are not | `0x0047C620`; `rulesmd.ini` LandType sections | Yes |
| Map bounds / screen | gameplay requires valid/on-screen cells through `TechnoClass__IsOnScreen`; map-editor mode uses `Cell_in_bounds_check`; `CanBePlacedAt` also calls `Cell_in_bounds_check` | `0x0047C620`, `0x0045EE70` | Yes |

### EVA and Failure Side Effects

If target type `+0xA8` returns false:

1. `0x007394D8` tests false and falls through.
2. `0x007394E0..0x0073950A` checks human player and `type+0x5EC == 0`, then calls `VoxClass__PlayEVA` with `ECX=0x0082012C`, the string `EVA_CannotDeployHere`.
3. Non-player-control contexts may also call `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70` at `0x00739536`.
4. `0x0073954D..0x00739565` re-enables the locomotor/deploy interface with `+0x9C(1)` and UnitClass vtable `+0x124(1)`.
5. The function clears deploy state and returns `0`.

No building has been allocated on this branch, and the AMCV is not removed. If the later `BuildingClass::Unlimbo` call at `0x00739711` fails after allocation, `0x00739A6E` destroys the temporary building via vtable `+0x20(1)`, restores deploy state, and returns `0`; the AMCV is still not consumed. The AMCV is destroyed only on the success path after building unlimbo and state transfer.

## 4. INI Keys

| Key / section | Retail YR value | Binary effect | Active in YR |
|---|---|---|---|
| `[AMCV] DeploysInto` | `GACNST` | enters unit-to-building deploy branch | Yes |
| `[AMCV] MovementZone` | `Normal` | movement/pathing setting, not footprint placement source | Yes, but not a deploy footprint gate |
| `[GACNST] ConstructionYard` | `yes` | sets target `BuildingType+0x16B9`, making `unit+0x2C0` a deploy blocker in `0x00700D50` | Yes |
| `[GACNST] Foundation` | `4x4` in `artmd.ini` | target type `+0xA8` walks the 4x4 base foundation list | Yes |
| `PlaceAnywhere` | absent on GACNST | no ordinary placement bypass | Yes: default false |
| `WaterBound` / `Naval` | absent on GACNST | ordinary land placement uses Buildable fallback | Yes: default false |
| LandTypes `[Clear]`, `[Road]` | `Buildable=yes` | accepted when other blockers are clear | Yes |
| LandTypes `[Water]`, `[Rock]` | `Buildable=no` | rejected for ordinary GACNST placement | Yes |

## 5. Integration Points

| Function / path | Role | Status | Active in YR |
|---|---|---|---|
| `UnitClass::Deploy @ 0x007393C0` | AMCV-to-GACNST conversion and failure side effects | verified | Yes |
| UnitClass vtable `+0x314 -> 0x00700D50` | generic deploy predicate before target placement validation | verified | Yes |
| BuildingType vtable `+0xA8 -> 0x00716150` | target footprint validator before allocation | verified | Yes |
| `Cell_passability_building_placement @ 0x0047C620` | per-cell blocker/buildability validator | verified | Yes |
| `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70` | secondary object/scatter validator in non-player failure contexts and other placement contexts | verified | Conditional in this failure branch |
| `BuildingClass::Unlimbo @ 0x00440580` | commit-time placement; failure aborts without AMCV consumption | verified for side-effect boundary | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Observed status | Evidence |
|---|---|---|
| `Simulation::deploy_mcv` | resolves `DeploysInto`, checks structure overlap and `effective_build_blocked`, then despawns MCV only before spawning GACNST | `src/sim/world/world_spawn.rs:495..583` |
| `deploy_origin_from_center` | matches large-foundation `(-1,-1)` origin rule for 4x4 targets | `src/sim/world/world_spawn.rs:662..668` |
| Mixed-height test | current test expects acceptance of mixed-height clear foundation | `src/sim/deploy_tests.rs:231..263` |
| Occupied foundation rejection test | rejects structure in rightmost foundation column and keeps AMCV/blocker | `src/sim/deploy_tests.rs:267..299` |
| EVA feedback | no scoped Rust path found that emits `EVA_CannotDeployHere` on `deploy_mcv` failure | `rg "EVA_CannotDeployHere|CannotDeploy" src` |

Rust already matches the no-consume-on-failure and mixed-height-accept outcomes. The scoped missing player-visible effect is the cannot-deploy EVA on legitimate placement failure, plus exact binary ordering/taxonomy for bridge/overlay/slope/buildability if `effective_build_blocked` does not cover those exactly.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| UnitClass vtable `+0x314` binding | verified | `0x007F5F84 = 0x00700D50`; call `0x007393CC` | none |
| `0x00700D50` AMCV-material branch | verified | assembly `0x00700D50..0x007010CA` | unrelated deployer branches not named |
| Target type placement virtual `0x00716150` | verified | Ghidra decompile and call from `0x007394D2` | none |
| Per-cell placement helper `0x0047C620` | verified | Ghidra decompile plus prior report | rare special-case building flags outside AMCV |
| `BuildingTypeClass::CanBePlacedAt 0x0045EE70` | verified | Ghidra decompile; branch at `0x00739536` | exact allied scatter ordering |
| `BuildingClass::Unlimbo 0x00440580` side-effect boundary | verified | Ghidra decompile; failure branch at `0x00739A6E` | full unlimbo lifecycle out of scope |
| Occupied cells | verified | `0x00716150 -> 0x0047C620`; `0x0045EE70` | Rust exact object-list parity remains implementation work |
| Mixed height | verified | no same-height comparison in scoped validators | none |
| Bridge/overlay/buildability | verified | `0x00700D50`, `0x0047C620`, INI LandTypes | exact `Cell+0x140 bit 0x400` semantic name |
| Map bounds | verified | `0x0047C620`, `0x0045EE70` | no runtime fixture executed |
| Failure abort/no consumption | verified | `0x00739502..0x0073956B`, `0x00739A6E`; unit destruction only on later success path | none |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Which concrete method is UnitClass vtable +0x314? -> 0x00700D50.` (evidence: `0x007F5F84`; `0x007393CC`; Active in YR: Yes)
- `[RESOLVED] OQ-2 - Does +0x314 perform full GACNST footprint validation? -> No; it checks deploy state, attached/power links, and bridge structural bits around the unit. Full foundation validation is target type +0xA8.` (evidence: `0x00700E47..0x00701064`, `0x007394D2`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - Where are occupied foundation cells checked? -> Target building type +0xA8 (`0x00716150`) walks foundation cells and calls `0x0047C620`; optional `0x0045EE70` also checks objects/scatter in some contexts.` (evidence: `0x00716150`, `0x0047C620`, `0x0045EE70`; Active in YR: Yes)
- `[RESOLVED] OQ-4 - Is mixed height a blocker? -> No same-height gate exists; slope `Cell+0x11C != 0` is a blocker, but differing cell levels alone are not.` (evidence: `0x00716150`, `0x0047C620`, `0x0045EE70`, `0x00440580`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - Are bridge/overlay/buildability checked? -> Yes, through `0x0047C620`; `+0x314` also rejects bridge structural flags around the unit.` (evidence: `0x00700F00..0x00701064`, `0x0047C620`; Active in YR: Yes)
- `[RESOLVED] OQ-6 - What reaches EVA_CannotDeployHere? -> Target type +0xA8 failure reaches `0x00739502`, loads `0x0082012C`, and calls `VoxClass__PlayEVA` for human players when `type+0x5EC == 0`.` (evidence: `0x007394D8..0x0073950A`; Active in YR: Yes)
- `[RESOLVED] OQ-7 - Does deploy failure consume AMCV? -> No. On +0xA8 failure no building is allocated; on later unlimbo failure the temporary building is destroyed and AMCV remains.` (evidence: `0x0073954D..0x0073956B`, `0x00739A6E`; Active in YR: Yes)
- `[RESOLVED] OQ-8 - Does standard AMCV use AMCV-specific hardcoding? -> No; it is generic `DeploysInto=GACNST`.` (evidence: prior AMCV audit; `rulesmd.ini [AMCV]`; Active in YR: Yes)
- `[DEFERRED] OQ-9 - Exact names for generic non-AMCV branches in `0x00700D50`.` (category: out-of-scope; reason: not material for standard `AMCV -> GACNST`; next-step-if-pursued: investigate non-MCV deployers)
- `[DEFERRED] OQ-10 - Exact semantic name of `CellClass+0x140 bit 0x400`.` (category: requires-different-system-context; reason: placement-blocking effect is verified; next-step-if-pursued: extend cell flag audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Legitimate AMCV placement failure plays `EVA_CannotDeployHere` for human player and returns failure without consuming AMCV | `UnitClass::Deploy @ 0x007394D8..0x0073956B`; string `0x0082012C` | missing EVA feedback; abort/no-consume already present | `src/sim/world/world_spawn.rs::deploy_mcv`, command/audio event path | emit local-human EVA on deploy footprint rejection while preserving AMCV/blockers | `deploy_mcv_blocked_structure_emits_cannot_deploy_eva_and_keeps_unit` | Do not despawn AMCV before all placement gates have succeeded |
| Mixed-height clear foundations are accepted; slope/buildability/overlay/bridge blockers are separate per-cell predicates | `0x00716150`, `0x0047C620`, `0x0045EE70`, `0x00440580`; current Rust test `deploy_mcv_accepts_mixed_height_clear_foundation` | none observed for mixed height; exact bridge/overlay taxonomy unchecked | `src/sim/world/world_spawn.rs::deploy_mcv`, `Simulation::effective_build_blocked` | keep accepting level-mixed clear cells; reject slope, nonbuildable land, overlays, bridge structural cells | `deploy_mcv_rejects_bridge_or_overlay_foundation_cell_without_consuming_unit` | Do not reintroduce an all-foundation-cells-same-height check |
| `vtable+0x314` is not the footprint validator; footprint blockers are target BuildingType `+0xA8` over base foundation cells | `0x00700D50` assembly; `0x007394D2`; `0x00716150` | high-level Rust surface is in `deploy_mcv`; exact base-foundation/per-cell order partial | `src/sim/world/world_spawn.rs::deploy_mcv`, foundation helpers | keep foundation validation driven by target building data, not AMCV-specific logic | `deploy_mcv_rejects_structure_in_rightmost_foundation_column` | Do not hardcode AMCV/GACNST or move full placement validation into a generic deploy-state flag |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/AMCV.md`: replace "Check `CanDeploy` (vtable slot 0x314) - verifies preconditions (cell free, no enemies nearby, valid placement footprint)." with: "Check `CanDeploy` (UnitClass vtable `+0x314`, concrete `0x00700D50`) - for stock AMCV this verifies generic deploy readiness, attached/power-link blockers, and bridge structural flags around the unit. The actual `GACNST` 4x4 footprint validation is the later target `BuildingTypeClass` vtable `+0xA8` call (`0x00716150`) over base foundation cells, using `Cell_passability_building_placement @ 0x0047C620`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/IMPLEMENTATION_MCV_DEPLOY_MIXED_HEIGHT_TRACE_RERUN_2026-05-21.md`: replace the Stage 2/4 claim that current Rust still rejects mixed-height foundations and has a rejecting test with: "Current Rust source no longer contains the same-height gate in `src/sim/world/world_spawn.rs::deploy_mcv`; `src/sim/deploy_tests.rs::deploy_mcv_accepts_mixed_height_clear_foundation` asserts acceptance. The remaining deploy-failure feedback gap is `EVA_CannotDeployHere` on legitimate blockers, not mixed-height clear cells."

## 10. Negative Facts / Do Not Do

- Do not treat AMCV as a hardcoded unit ID; stock behavior is generic `DeploysInto=GACNST`.
- Do not treat UnitClass vtable `+0x314` as the full footprint validator; it does not walk the 4x4 GACNST foundation.
- Do not reject clear mixed-height GACNST foundations; no same-height gate exists in the active validators.
- Do not consume or destroy the AMCV on placement failure; consumption is after successful building unlimbo and transfer.
- Do not use generic unit pathing walkability as the source of truth for building placement; ordinary land placement uses `Buildable=`, overlay/object/occupation bits, slope, and bridge flags.

## 11. Remaining Uncertainty

- Exact semantic names for several generic non-AMCV branches and fields in `0x00700D50` remain deferred; they are not material for stock `AMCV -> GACNST`.
- Exact global meaning of `CellClass+0x140 bit 0x400` remains delegated to cell-flag research; its placement-blocking effect is verified here.

## Sources

- Ghidra read-only decompiled: `UnitClass::Deploy @ 0x007393C0`, `FUN_00716150 @ 0x00716150`, `Cell_passability_building_placement @ 0x0047C620`, `BuildingTypeClass::CanBePlacedAt @ 0x0045EE70`, `BuildingClass::Unlimbo @ 0x00440580`.
- Ghidra read-only assembly/memory: UnitClass vtable `0x007F5F80..0x007F5F8F`; `0x00700D50..0x007010CA`; `0x007394D8..0x0073956B`; `EVA_CannotDeployHere @ 0x0082012C`.
- Docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDING_PLACEMENT_VALIDATOR_FOUNDATION_HEIGHT_OCCUPY_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/CELL_PASSABILITY_BUILDING_PLACEMENT_FLAGS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDING_MCV_DEPLOY_ORIGIN_BLAST_RADIUS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/AMCV.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/world/world_spawn.rs`, `src/sim/world/world_commands.rs`, `src/sim/deploy_tests.rs`, `src/sim/production/production_placement.rs`.
