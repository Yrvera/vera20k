# Building MCV Deploy Origin Blast Radius - Ghidra Research Report

**Address(es):** `0x007393C0`, `0x0045EC90`, `0x0045ECA0`, `0x0049F2F0`  
**Investigation Mode:** exhaustive-slice for discrepancy sizing only  
**Claimed Scope:** Player-visible origin-cell delta between current Rust and `gamemd.exe` for stock YR `DeploysInto=` unit-to-building cases.  
**Non-Scope:** Full deploy/undeploy lifecycle, buildup timing, slave miner economy behavior, placement validation, AddOccupy/RemoveOccupy, and building draw offsets except where origin shift directly affects them.  
**Confidence:** High for AMCV/SMCV/PCV ConYard deltas; Medium-High for SMIN/YAREFN current-Rust command-path delta because the active command path is verified, while complete intended slave-miner AI/economy behavior is outside this slice.  
**Active in YR:** Yes. `UnitClass::Deploy` is the stock YR unit deploy path and retail `rulesmd.ini` defines the listed `DeploysInto=` pairs.

## 1. Overview

`gamemd.exe` does not center a deployed building under the unit by subtracting half the target foundation. It takes the deploying unit cell, and for target foundations with width > 2 or height > 2 applies one hardcoded northwest cell adjustment `(-1,-1)` before calling the building Unlimbo path.

Current Rust's generic `deploy_mcv` path computes `origin = unit_cell - (foundation_width / 2, foundation_height / 2)`. For stock 4x4 Construction Yards this places the building one cell northwest of `gamemd.exe`. For the active generic command path applied to SMIN -> YAREFN, Rust also shifts the 2x2 refinery one cell northwest while `gamemd.exe` keeps the unit cell as origin.

## 2. Retail `DeploysInto=` Inventory

Scoped `rulesmd.ini` search found exactly four active retail YR `DeploysInto=` unit-to-building pairs:

| Unit | Target building | Target foundation | INI evidence | Active in YR |
|---|---|---:|---|---|
| `AMCV` | `GACNST` | 4x4 | `ini/rulesmd.ini:6969..6977`; `ini/artmd.ini:1599..1601` | Yes - Allied MCV is a stock skirmish base unit and `DeploysInto=GACNST` is live. |
| `SMCV` | `NACNST` | 4x4 | `ini/rulesmd.ini:7838..7845`; `ini/artmd.ini:1651..1653` | Yes - Soviet MCV is a stock skirmish base unit and `DeploysInto=NACNST` is live. |
| `PCV` | `YACNST` | 4x4 | `ini/rulesmd.ini:8826..8834`; `ini/artmd.ini:1622..1626` | Yes - Yuri MCV is a stock YR skirmish base unit and `DeploysInto=YACNST` is live. |
| `SMIN` | `YAREFN` | 2x2 | `ini/rulesmd.ini:9042..9097`; `ini/artmd.ini:1799..1804` | Yes - Yuri Slave Miner deploys into its refinery building in stock YR. |

Base RA2 `rules.ini` only has `AMCV -> GACNST` and `SMCV -> NACNST`; YR `rulesmd.ini` adds `PCV -> YACNST` and `SMIN -> YAREFN`.

## 3. Verified Binary Deploy-Origin Rule

| Binary detail | Evidence | Active in YR |
|---|---|---|
| `UnitClass::Deploy` gets the unit cell through vtable `+0x1B8`, then reads the target building type from unit state and calls foundation width/height helpers. | Ghidra `UnitClass__Deploy @ 0x007393C0`; assembly `0x0073945A..0x0073948E`. | Yes - this is the deploy function reached by stock `DeploysInto=` units. |
| Width and height are read from `BuildingTypeClass+0xEF0` foundation index tables. | `BuildingTypeClass__GetFoundationWidth @ 0x0045EC90`; `BuildingTypeClass__GetFoundationHeight @ 0x0045ECA0`. | Yes - stock `Foundation=` art values feed this index. |
| If `width > 2`, branch directly applies `DAT_0089F6A4/6`; otherwise height is checked and the same adjustment applies if `height > 2`. If both dimensions are `<= 2`, the unit cell is kept. | Assembly `0x00739472..0x007394B1`: `CMP EAX,0x2`, `JG 0x00739499`; second `CMP EAX,0x2`, `JG 0x00739499`; else stores original cell. | Yes - active for 4x4 ConYards and bypassed for 2x2 YAREFN. |
| `DAT_0089F6A4` is initialized to dword `0xFFFFFFFF`, signed shorts `(-1,-1)`. | `Foundation_direction_table_init @ 0x0049F2F0`; prior `DAT_0089F6A0_RUNTIME_SOURCE_AND_VALUE_GHIDRA_REPORT.md`. | Yes - constructor-table initialization is active at process startup. |
| The building Unlimbo coordinate is `origin_cell * 256 + 128`, so the cell delta directly becomes the building's stored foundation-origin cell. | Prior `BUILDING_POSITION_FOUNDATION_ORIGIN_PARITY_GHIDRA_REPORT.md`; `UnitClass::Deploy @ 0x007396DF..0x00739711`. | Yes - the deployed building is created and unlimboed on this path. |

Binary formula:

```text
unit_cell = deploying_unit.GetCell()
if target_foundation_width > 2 or target_foundation_height > 2:
    gamemd_origin = unit_cell + (-1, -1)
else:
    gamemd_origin = unit_cell
```

## 4. Current Rust Comparison

Current generic Rust deploy path:

```text
deploy_mcv:
    target = rules.object(unit.DeploysInto)
    rust_origin = deploy_origin_from_center(unit.rx, unit.ry, target.foundation)

deploy_origin_from_center:
    rust_origin = (unit.rx - width / 2, unit.ry - height / 2)
```

Evidence:

| Rust point | Evidence | Active in YR |
|---|---|---|
| `Command::DeployMcv` calls `Simulation::deploy_mcv`. | `src/sim/world/world_commands.rs:477..482`. | Yes for the YR counterpart behavior; this is current Rust's command surface for YR `DeploysInto=` units. |
| Keyboard deploy queues `Command::DeployMcv` for selected non-infantry objects with `DeploysInto=`. | `src/app_input.rs:842..883`. | Yes for the YR counterpart behavior; all stock non-infantry `DeploysInto=` cases are covered by the condition. |
| Context self-click queues `Command::DeployMcv` when `DeploysInto=` or `Deployer=` is present. | `src/app_context_order.rs:468..479`. | Yes for the YR counterpart behavior; it includes MCV-style deploy input. |
| AI deploy logic also treats any owned object with `DeploysInto=` as deployable. | `src/sim/ai.rs:91..98`, `src/sim/ai.rs:143..174`. | Yes for the YR counterpart behavior; AI starts routinely deploy MCVs. |
| `deploy_mcv` resolves target via `DeploysInto=` and spawns at `deploy_origin_from_center`. | `src/sim/world/world_spawn.rs:503..527`, `src/sim/world/world_spawn.rs:584..599`. | Yes for the YR counterpart behavior. |
| `deploy_origin_from_center` subtracts `width / 2` and `height / 2`. | `src/sim/world/world_spawn.rs:680..685`. | Yes for the YR counterpart behavior; this is the mismatching formula. |
| A separate `deploy_slave_miner` helper would spawn at the same cell, but scoped search found no callers. | `src/sim/slave_miner.rs:436..468`; `rg "deploy_slave_miner\\(" src` found only the definition. | Conditional - SMIN/YAREFN is active in YR, but this matching Rust helper is not the verified active command path. |

## 5. Exact Discrepancy Sizing

`Rust - gamemd` is the current Rust origin cell minus the verified `gamemd.exe` origin cell.

| Pair | gamemd origin relative to unit | current Rust generic origin relative to unit | `Rust - gamemd` origin delta | Occupancy/appearance delta | Frequency in normal skirmish | Severity |
|---|---:|---:|---:|---|---|---|
| `AMCV -> GACNST` | `(-1,-1)` | `(-2,-2)` | `(-1,-1)` | Entire 4x4 ConYard appears and occupies one cell northwest of gamemd; 3x3 overlap, 7 Rust-only cells on north/west edges, 7 gamemd cells missing on south/east edges. | Every Allied start MCV deploy, plus any Allied redeploy/crate MCV. Happens in the opening seconds of ordinary skirmish. | **Severe/Frequent** - first base anchor, footprint blocking, build adjacency, selection/hit testing, shroud/reveal, and path blocking are all shifted. |
| `SMCV -> NACNST` | `(-1,-1)` | `(-2,-2)` | `(-1,-1)` | Entire 4x4 ConYard appears and occupies one cell northwest of gamemd; same 3x3 overlap and 7-cell edge swap. | Every Soviet start MCV deploy, plus redeploy/crate MCV. Opening-seconds ordinary skirmish. | **Severe/Frequent** - same base-anchor and blocking impact. |
| `PCV -> YACNST` | `(-1,-1)` | `(-2,-2)` | `(-1,-1)` | Entire 4x4 ConYard appears and occupies one cell northwest of gamemd; same 3x3 overlap and 7-cell edge swap. | Every Yuri start PCV deploy, plus redeploy/crate PCV. Opening-seconds ordinary YR skirmish. | **Severe/Frequent** - same base-anchor and blocking impact. |
| `SMIN -> YAREFN` through current generic `Command::DeployMcv` path | `(0,0)` | `(-1,-1)` | `(-1,-1)` | 2x2 YAREFN appears and occupies one cell northwest of gamemd; 1x1 overlap, 3 Rust-only cells on north/west edges, 3 gamemd cells missing on south/east edges. | Conditional in current Rust: explicit selected-unit deploy and the generic AI deploy check use this path; the matching `deploy_slave_miner` helper appears unused. In stock YR, Slave Miner deploy into refinery is a normal Yuri economy action. | **Moderate** for current Rust command-path origin sizing; potentially Severe for full Yuri economy if that generic path is used for normal automated SMIN deploys. |

## 6. Player-Visible Blast Radius

The ConYard discrepancy is not a cosmetic half-cell drift; it is a full-cell northwest foundation-origin error. Because Rust stores building positions as the foundation origin and seeds occupancy from that origin, the error moves both the visible building anchor and the blocked cells. A 4x4 ConYard shifted by `(-1,-1)` still overlaps most of the gamemd footprint, but the entire north and west edges are too blocked while the south and east edges are incorrectly free.

Normal-play frequency is high because initial MCV deployment is a required opening action in standard skirmish for all three sides. AI also deploys its MCV through the same `Command::DeployMcv` path once `ai.mcv_deployed` is false. Any later MCV redeploy repeats the same one-cell northwest error.

The SMIN/YAREFN finding is narrower: the binary target is active in stock YR, and current generic Rust deploy input would misplace it by the same `(-1,-1)` delta. However, the separate same-cell `deploy_slave_miner` helper is not wired in the scanned command path, so this report sizes the active command discrepancy and does not claim the complete slave-miner economy path.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Retail `DeploysInto=` inventory | verified | `rg "DeploysInto=" ini/rulesmd.ini ini/rules.ini`; section snippets for AMCV/SMCV/PCV/SMIN | none |
| ConYard `Foundation=4x4` and YAREFN `Foundation=2x2` | verified | `ini/artmd.ini:1599..1601`, `1622..1626`, `1651..1653`, `1799..1804` | none |
| `UnitClass::Deploy` origin branch | verified | Ghidra `0x007393C0`; assembly `0x0073945A..0x007394D2` | none for deploy-origin sizing |
| Foundation width/height helpers | verified | Ghidra `0x0045EC90`, `0x0045ECA0` | none |
| `DAT_0089F6A4` value | verified | Ghidra `0x0049F2F0`; prior DAT report | none |
| Rust generic MCV deploy formula | verified | `src/sim/world/world_spawn.rs:503..527`, `680..685` | none |
| Rust command wiring for `DeploysInto=` | verified | `src/app_input.rs`, `src/app_context_order.rs`, `src/sim/ai.rs`, `src/sim/world/world_commands.rs` | none for current command surface |
| Separate slave-miner deployment lifecycle | touched-not-exhausted | `src/sim/slave_miner.rs:436..468`; no callers found by scoped `rg` | Full slave-miner economy/AI deploy behavior is outside this origin-sizing slice. |
| Placement validation differences caused by shifted origin | deferred | current Rust checks footprint from shifted origin | Out of scope; sibling placement/path blocking audits should size passability and validation separately. |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Which stock YR unit-to-building `DeploysInto=` pairs exist? Four: `AMCV->GACNST`, `SMCV->NACNST`, `PCV->YACNST`, `SMIN->YAREFN`. Evidence: `ini/rulesmd.ini:6977`, `7845`, `8834`, `9097`. Active in YR: Yes.  
[RESOLVED] OQ-2 - What exact origin does `gamemd.exe` use for 4x4 ConYards? `unit_cell + (-1,-1)`. Evidence: `UnitClass::Deploy @ 0x007393C0`, assembly `0x00739472..0x007394B1`, `DAT_0089F6A4=0xFFFFFFFF`. Active in YR: Yes.  
[RESOLVED] OQ-3 - What exact origin does `gamemd.exe` use for 2x2 YAREFN? `unit_cell`. Evidence: same branch keeps original cell when width <= 2 and height <= 2; `YAREFN Foundation=2x2`. Active in YR: Yes.  
[RESOLVED] OQ-4 - What exact origin does current Rust use for 4x4 targets? `unit_cell + (-2,-2)`. Evidence: `deploy_origin_from_center` subtracts `width/2,height/2`; `GACNST/NACNST/YACNST Foundation=4x4`. Active in YR: Yes for counterpart behavior.  
[RESOLVED] OQ-5 - What exact origin does current Rust generic command path use for 2x2 YAREFN? `unit_cell + (-1,-1)`. Evidence: `Command::DeployMcv` queues for any non-infantry `DeploysInto=` and `deploy_origin_from_center` subtracts `1,1` for 2x2. Active in YR: Yes for the counterpart SMIN deploy target; current Rust path is conditional on command usage.  
[RESOLVED] OQ-6 - Does current Rust contain a matching same-cell SMIN helper? Yes, but scoped search found it unused. Evidence: `src/sim/slave_miner.rs:436..468`; `rg "deploy_slave_miner\\(" src`. Active in YR: Conditional for Rust only; SMIN deploy is active in YR, helper reachability in current Rust is not verified beyond no callers.  
[RESOLVED] OQ-7 - How often does the ConYard error occur in ordinary skirmish? Severe/Frequent: initial MCV deploy for Allied, Soviet, and Yuri uses this target class and happens at the start of normal skirmish. Evidence: stock `BaseUnit=AMCV,SMCV,PCV` at `ini/rulesmd.ini:390`; deploy command wiring and AI deploy check. Active in YR: Yes.

## Sources

- Ghidra: `UnitClass__Deploy @ 0x007393C0`; `get_assembly_context` around `0x00739450`, `0x00739460`, `0x00739477`, `0x0073948E`, `0x007394B1`, `0x007394D8`.
- Ghidra: `BuildingTypeClass__GetFoundationWidth @ 0x0045EC90`; `BuildingTypeClass__GetFoundationHeight @ 0x0045ECA0`; `Foundation_direction_table_init @ 0x0049F2F0`.
- Prior context: `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDING_POSITION_FOUNDATION_ORIGIN_PARITY_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/DAT_0089F6A0_RUNTIME_SOURCE_AND_VALUE_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`.
- Current Rust: `src/sim/world/world_spawn.rs`, `src/sim/world/world_commands.rs`, `src/app_input.rs`, `src/app_context_order.rs`, `src/sim/ai.rs`, `src/sim/slave_miner.rs`.
