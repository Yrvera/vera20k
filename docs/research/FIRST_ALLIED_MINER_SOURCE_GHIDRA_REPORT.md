# First Allied Miner Source -- Ghidra Research Report

**Address(es):** `0x00686890` (`Post_Map_Init`), `0x006886B0` (`Generate_Random_Units`), `0x00445F80` (`BuildingClass::OnConstructionComplete`), `0x00460540` (`BuildingTypeClass` INI `FreeUnit` parser), `0x007393C0` (`UnitClass::Deploy`)  
**Investigation Mode:** coverage-map, downgraded from requested exhaustive-slice because this subagent session exposed no Ghidra MCP/tool namespace for fresh decompilation.  
**Claimed Scope:** Standard YR Allied start/miner source decision: whether first `CMIN` comes from AMCV-created `GACNST`, random starting-unit/`UnitCount` generation, side-specific start lists, `HarvesterUnit`, first refinery `FreeUnit`, or another known init path.  
**Non-Scope:** Exact RNG outcome of all non-miner starting units, live network-manager variant `vtable+0x84`, AI later economy/production beyond the first refinery free unit, Yuri slave miner worker creation, and runtime debugger observation.  
**Confidence:** Medium-High for the conclusion from existing high-confidence Ghidra reports plus stock INI; Low for any claim requiring fresh binary re-check in this slot.  
**Active in YR:** Yes for the documented paths; `Generate_Random_Units` and `BuildingClass::OnConstructionComplete` are active in standard YR skirmish/multiplayer setup, with network multiplayer using a network variant for unit generation.

## 1. Overview

The first stock Allied `CMIN` is not created by AMCV deploy and is not eligible for the random starting-unit/`UnitCount` pool. Existing Ghidra reports identify the live start generator at `Generate_Random_Units @ 0x006886B0`, but that generator filters random vehicles through `TechnoTypeClass+0x6D5` (`AllowedToStartInMultiplayer` / spawnable), and stock `[CMIN]` sets `AllowedToStartInMultiplayer=no`.

The remaining verified stock data path is the first Allied refinery: `[GAREFN] FreeUnit=CMIN`, consumed by `BuildingClass::OnConstructionComplete @ 0x00445F80` after the refinery completes/places. Therefore "first Allied miner" in normal play is the refinery free unit, not an initial skirmish seed.

## 2. Key Fields / Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00a8b270` | lobby `[MultiplayerDialogSettings] UnitCount` | prior `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, `Generate_Random_Units @ 0x006886B0` | Yes |
| `DAT_00a8b258` | `Bases` option; if set, MCV counts against start budget | prior Ghidra start-unit docs | Yes |
| `TechnoTypeClass+0x6D5` | `AllowedToStartInMultiplayer` / spawnable gate | `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`; start generator filters on `+0x6D5` | Yes |
| `RulesClass+0xB20` | `BaseUnit` vector used for MCV selection | prior start-unit report, `FUN_00505310 @ 0x00505310` | Yes |
| `RulesClass+0xB4C` | `HarvesterUnit` vector nearby; production/economy preference, not start spawn source in verified path | prior start-unit report notes storage only | Active elsewhere; no start-use verified here |
| `BuildingTypeClass+0xEA0` | parsed `FreeUnit=` pointer | `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`, parser `0x00460540` | Conditional; stock `GAREFN` yes, stock `GACNST` no |

## 3. Core Logic

### A. Start-unit generator

Existing Ghidra report `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` identifies this chain:

`Post_Map_Init @ 0x00686890` -> offline/skirmish `Generate_Random_Units @ 0x006886B0`; network multiplayer calls a network manager `vtable+0x84` variant plus `FUN_005D6D80`.

`Generate_Random_Units @ 0x006886B0`:

1. Reads `UnitCount` from `DAT_00a8b270`.
2. If `Bases` is on, subtracts one unit from the budget because the MCV counts as one start unit.
3. Builds infantry and vehicle candidate lists.
4. Candidate gates include `Spawnable` / `AllowedToStartInMultiplayer` at `TechnoTypeClass+0x6D5`, `TechLevel <= house tech`, and owner/side `HouseMask`.
5. The random budget loop spends cost budget, first on infantry while `spentBudget < totalBudget * 2 / 3`, then on vehicles.

Stock `[CMIN]` cannot enter that vehicle list because `rulesmd.ini:7371` has `AllowedToStartInMultiplayer=no`.

### B. MCV / construction yard path

Prior report `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md` verifies:

1. `UnitClass::Deploy @ 0x007393C0` creates `DeploysInto=GACNST`.
2. No branch in `UnitClass::Deploy` reads `BuildingTypeClass+0xEA0` or constructs a `CMIN`.
3. `BuildingClass::OnConstructionComplete @ 0x00445F80` is the verified `FreeUnit=` consumer.
4. Stock `[GACNST]` has no `FreeUnit` key, so its `+0xEA0` stays null.

### C. Refinery path

Stock `[GAREFN]` has `FreeUnit=CMIN` in `rulesmd.ini:11736`. The same existing Ghidra report verifies parser `0x00460540` stores that section-local `FreeUnit` into `BuildingTypeClass+0xEA0`, and `OnConstructionComplete @ 0x00445F80` constructs and places the unit when completion gates pass.

This is the first confirmed stock Allied miner source.

## 4. INI Keys

| File / section | Key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `rulesmd.ini [General]` | `BaseUnit` | `AMCV,SMCV,PCV` | selects Allied MCV for Allied house; not miner source | Yes |
| `rulesmd.ini [General]` | `HarvesterUnit` | `HARV,CMIN` | preferred harvesters for build/economy systems; no verified start-spawn use in `Generate_Random_Units` | Active elsewhere; not start source here |
| `rulesmd.ini [MultiplayerDialogSettings]` | `UnitCount` | `10` | start-unit budget input; cannot override CMIN's spawnable gate | Yes |
| `rulesmd.ini [AMCV]` | `DeploysInto` | `GACNST` | creates ConYard path only | Yes |
| `rulesmd.ini [CMIN]` | `AllowedToStartInMultiplayer` | `no` | excludes CMIN from random starting-unit candidate list | Yes |
| `rulesmd.ini [GACNST]` | `FreeUnit` | absent | no CMIN from ConYard completion | Yes |
| `rulesmd.ini [GAREFN]` | `FreeUnit` | `CMIN` | first verified stock Allied miner source | Yes |

## 5. Integration Points

| Path | Role | Finding | Active in YR |
|---|---|---|---|
| `Post_Map_Init @ 0x00686890` | start initialization | calls start-unit generation after map/houses are loaded | Yes |
| `Generate_Random_Units @ 0x006886B0` | MCV plus random start roster | can create start units but excludes `CMIN` via `+0x6D5` | Yes |
| network manager `vtable+0x84` + `FUN_005D6D80` | network multiplayer start variant | touched only via prior docs; expected same purpose, not freshly verified here | Conditional/deferred |
| `UnitClass::Deploy @ 0x007393C0` | AMCV -> GACNST | no `CMIN` side effect | Yes |
| `BuildingClass::OnConstructionComplete @ 0x00445F80` | building completion/free unit | creates `FreeUnit` for `GAREFN`; skipped for `GACNST` | Yes |

## 6. Current Rust Implementation Status

Current Rust `src/app_skirmish.rs::seed_skirmish_opening_if_needed` only seeds MCVs at multiplayer start waypoints and credits; it does not implement the full gamemd `UnitCount` random starting-unit budget. It also only pairs `take(2)` houses, which is a broader skirmish parity gap outside this miner slice.

Current Rust has a refinery free-unit surface in `src/sim/production/production_refinery.rs::maybe_spawn_refinery_harvester`, resolved through `RuleSet::refinery_free_unit`. Existing tests cover modded refinery free units and no-spawn when `FreeUnit` is absent.

The implementation risk is not adding CMIN to AMCV/GACNST or start seeding; the likely Rust delta is to ensure standard `[GAREFN] FreeUnit=CMIN` is the accepted first-miner path and that any future start-unit generator respects `AllowedToStartInMultiplayer=no`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| AMCV-created GACNST as CMIN source | verified negative | prior Ghidra report: `0x007393C0`, `0x00445F80`, stock `[GACNST]` no `FreeUnit` | none |
| `GAREFN FreeUnit=CMIN` as source | verified | prior Ghidra report: `0x00460540`, `0x00445F80`; `rulesmd.ini:11736` | exact runtime frame timing relative to buildup not rechecked here |
| start-unit generator candidate gate | verified from prior report | `Generate_Random_Units @ 0x006886B0`, `TechnoType+0x6D5`; `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` | fresh Ghidra re-read unavailable in this slot |
| stock `[CMIN]` start eligibility | verified from INI + prior field map | `rulesmd.ini:7371`, `TechnoType+0x6D5` | none |
| `HarvesterUnit=HARV,CMIN` as start source | verified negative for scoped path | prior start-unit pseudocode does not read `HarvesterUnit`; INI storage only at Rules `+0xB4C` | follow AI/economy production if needed |
| network multiplayer unit generation variant | touched-not-exhausted | prior start report mentions network manager `vtable+0x84` + `FUN_005D6D80` | fresh decompile needed to prove identical CMIN exclusion |
| Rust MCV seeding | verified by source scan | `src/app_skirmish.rs::seed_skirmish_opening_if_needed` | full `UnitCount` parity remains unimplemented |
| Rust refinery free unit | verified by source scan | `src/sim/production/production_refinery.rs`, `src/rules/ruleset.rs` | acceptance against stock `GAREFN` should be explicit |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Does AMCV deploy create the first CMIN? -> No; `UnitClass::Deploy` creates `GACNST` only and stock `GACNST` lacks `FreeUnit`.` (evidence: `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`, `0x007393C0`, `0x00445F80`, `rulesmd.ini [GACNST]`)
- `[RESOLVED] OQ-2 -- Can `CMIN` be selected by standard start-unit generation? -> No for stock rules; the generator filters on `TechnoType+0x6D5`, and `[CMIN] AllowedToStartInMultiplayer=no` clears that eligibility.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, `rulesmd.ini:7371`)
- `[RESOLVED] OQ-3 -- Does `UnitCount=10` force a miner despite the CMIN gate? -> No evidence in the verified start generator; `UnitCount` produces a cost budget over eligible spawnable types, not a mandatory harvester slot.` (evidence: `Generate_Random_Units @ 0x006886B0` prior report)
- `[RESOLVED] OQ-4 -- Does `BaseUnit=AMCV,SMCV,PCV` create CMIN? -> No; it selects the side-matching MCV via `FUN_00505310`, not a harvester.` (evidence: prior start-unit report, `FUN_00505310 @ 0x00505310`, `rulesmd.ini:390`)
- `[RESOLVED] OQ-5 -- Does `HarvesterUnit=HARV,CMIN` create the first miner at start? -> Not in the verified start-unit path; it is parsed/stored near `RulesClass+0xB4C` but not used by `Generate_Random_Units` in prior decompilation.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, `rulesmd.ini:393`)
- `[RESOLVED] OQ-6 -- What is the first verified stock Allied CMIN source? -> First Allied refinery completion/placement through `[GAREFN] FreeUnit=CMIN`.` (evidence: `0x00445F80`, `0x00460540`, `rulesmd.ini:11736`)
- `[DEFERRED] OQ-7 -- Does the network multiplayer `vtable+0x84` variant use exactly the same `+0x6D5` CMIN exclusion?` (category: needs-runtime-debugger; reason: no Ghidra MCP/tool exposed in this subagent session; next-step-if-pursued: fresh decompile network `GenerateUnits` and `FUN_005D6D80`)
- `[DEFERRED] OQ-8 -- Exact frame on which first refinery free `CMIN` appears relative to `GAREFN` buildup completion.` (category: requires-different-system-context; reason: existing report proves `OnConstructionComplete` source but this slice did not re-time buildup frames; next-step-if-pursued: trace `Mission_Construction @ 0x00449A50` frame cadence)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock CMIN is excluded from random starting-unit generation by `AllowedToStartInMultiplayer=no` | prior `0x006886B0` report; `TechnoType+0x6D5`; `rulesmd.ini:7371` | full start-unit budget not implemented | future start-unit seeding near `src/app_skirmish.rs` or sim init | if/when UnitCount generation is added, filter candidates by parsed start eligibility so CMIN/HARV do not spawn as random start units | proposed test `standard_allied_start_unit_budget_excludes_cmin` | Do not special-case "harvester" into start roster |
| First confirmed Allied miner source is first `GAREFN` completion/placement via `FreeUnit=CMIN` | `0x00445F80`, `0x00460540`, `rulesmd.ini:11736` | Rust has `maybe_spawn_refinery_harvester`; stock acceptance should be explicit | `src/sim/production/production_refinery.rs`, `src/rules/ruleset.rs` | preserve refinery-driven free unit, not ConYard-driven free unit | proposed test `standard_allied_refinery_completion_spawns_first_cmin` | Do not attach `FreeUnit` to `GACNST` or AMCV deploy |
| AMCV/GACNST path remains no-miner | prior `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`; stock `[GACNST]` no `FreeUnit` | Rust MCV seed/deploy should stay no-CMIN | `src/app_skirmish.rs`, `src/sim/world/world_spawn.rs::deploy_mcv` | spawning/deploying AMCV may create AMCV/GACNST only for this concern | proposed test `amcv_deploy_does_not_spawn_cmin_before_refinery` | Do not compensate for missing start miner by adding CMIN in deploy or SpawnPick |

## 10. Negative Facts / Do Not Do

- `AMCV -> GACNST` does not create `CMIN`; stock `GACNST` has no `FreeUnit`.
- `UnitCount=10` is a budget over eligible start units, not a hard "give one miner" rule.
- `CMIN` is not eligible for the random start roster because stock `[CMIN]` sets `AllowedToStartInMultiplayer=no`.
- `HarvesterUnit=HARV,CMIN` is not the verified first-miner start source; do not use it as a spawn list for opening units.
- `BaseUnit=AMCV,SMCV,PCV` selects MCVs only; do not infer side-specific miners from that list.

## 11. Remaining Uncertainty

- No fresh Ghidra decompilation was possible in this subagent slot because no Ghidra MCP/resources/tools were registered. The report relies on existing high-confidence Ghidra reports and local INI/Rust scans.
- Network multiplayer's `vtable+0x84` start-unit variant should be freshly decompiled before claiming complete parity for live LAN/WOL start generation.
- Exact refinery free-unit timing relative to buildup animation completion was not re-timed here.

## 12. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/CMIN.md:127` should replace:
  - Old: `Allied ConYard's FreeUnit=CMIN spawns the first one on deploy.`
  - New: `Verified 2026-05-21: stock CMIN is excluded from random starting-unit generation by AllowedToStartInMultiplayer=no. AMCV-created GACNST does not spawn CMIN; the first confirmed stock Allied CMIN source is GAREFN completion/placement via [GAREFN] FreeUnit=CMIN.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/CMIN.md:280` should replace:
  - Old: `AllowedToStartInMultiplayer=no -- Allied ConYard spawns first CMIN via its FreeUnit= line (the Allied-side FreeHarvester mechanism).`
  - New: `AllowedToStartInMultiplayer=no excludes CMIN from the lobby starting-unit complement. The first confirmed stock Allied CMIN is created later by the Allied refinery's section-local FreeUnit=CMIN path, not by GACNST/ConYard deploy.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/MTNK.md:384` should replace "only the AMCV -> GACNST -> free CMIN starter sequence" with "AMCV start plus later GAREFN FreeUnit=CMIN; no GACNST free-miner spawn."
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/soviet/HARV.md:114` and `:325` contain analogous ConYard/free-harvester wording and should be rechecked against `[NAREFN] FreeUnit=HARV`; this report did not freshly investigate Soviet start.

## Sources

- Existing Ghidra reports/docs referenced:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/GAME_START_INITIALIZATION.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/SPAWN_POINT_ASSIGNMENT_SYSTEM.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`
- INI files checked:
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
- Rust files scanned:
  - `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/game_options.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_refinery.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/rules/ruleset.rs`

## Status

PARTIAL for fresh Ghidra workflow; ANSWERED for the scoped source decision from existing verified reports and stock data. The first confirmed stock Allied CMIN source is `[GAREFN] FreeUnit=CMIN` consumed by `BuildingClass::OnConstructionComplete`, not AMCV/GACNST deploy and not random start-unit generation.
