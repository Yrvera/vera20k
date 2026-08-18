# YAREFN -> SMIN Undeploy SlaveManager Path - Ghidra Research Report

**Address(es):** `0x00449C30` (`BuildingClass__Sell`), `0x00449BC0` (`BuildingClass__CanUndeployMCV`), `0x0044F5C0` (`BuildingClass__ShouldShowDeployButton`), `0x004555D0` (`BuildingClass__CanSellOrUndeploy`), `0x006AF580` (`PowerUp_Cleanup`), `0x007353C0` (`UnitClass__Constructor`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock YR `[YAREFN] UndeploysInto=SMIN` reverse conversion, SMIN origin/facing choice, `SlaveManagerClass` transfer back to the unit, and checks before/at conversion.  
**Non-Scope:** full slave economy, normal MCV undeploy beyond comparison gates, ordinary refinery docking, and full sell/garrison survivor behavior.  
**Confidence:** High for the path and transfer; Medium for exact UI command plumbing before mission selection because this slice decompiled the gate functions and mission body, not the network event dispatcher.  
**Active in YR:** Yes. Stock `rulesmd.ini` gives `[YAREFN] UndeploysInto=SMIN`, `[YAREFN] Enslaves=SLAV`, `[SMIN] Enslaves=SLAV`; `artmd.ini` gives `[YAREFN] Foundation=2x2`.

## 1. Overview

Stock YR does not reverse `YAREFN` through `UnitClass__Deploy`. Forward `SMIN -> YAREFN` uses the generic unit `DeploysInto=` path, but reverse `YAREFN -> SMIN` is a building-side `UndeploysInto=` branch inside `BuildingClass__Sell @ 0x00449C30`.

For YAREFN specifically, the new SMIN is constructed with `UnitClass__Constructor(Type+0x408, Owner)`, placed at the YAREFN building coordinate because the YAREFN foundation is `2x2`, and receives the old YAREFN `SlaveManagerClass` via `PowerUp_Cleanup`.

## 2. Key Offsets

| Offset | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `TechnoTypeClass+0x408` | `UndeploysInto` UnitType pointer | `TechnoTypeClass__ReadINI` string `UndeploysInto`; `BuildingClass__Sell @ 0x00449C30` tests/constructs from this field | Yes - `[YAREFN] UndeploysInto=SMIN` |
| `TechnoTypeClass+0xEDC` | `DeployFacing` raw facing value | `Deploy_facing_calculator @ 0x00465D70` returns `*(param+0xEDC)` | Yes - `[YAREFN] DeployFacing=0` |
| `TechnoTypeClass+0x16B9` | Construction-yard special undeploy restrictions | `BuildingClass__CanUndeployMCV @ 0x00449BC0`; `BuildingClass__Sell @ 0x00449C30` | Conditional - false for stock YAREFN, so ConYard-only `MCVRedeploys` restrictions do not apply |
| `TechnoClass+0x2D8` | SlaveManager pointer | `BuildingClass__Sell @ 0x00449C30` checks it before `PowerUp_Cleanup`; `PowerUp_Cleanup @ 0x006AF580` rewrites owner and slave masters | Yes - YAREFN and SMIN both have `Enslaves=SLAV` |
| `SlaveManager+0x24` | current master techno pointer | `PowerUp_Cleanup @ 0x006AF580` clears old owner `+0x2D8`, then writes new owner | Yes |
| `SlaveManager+0x3C/+0x48` | slave pointer array and count | `PowerUp_Cleanup @ 0x006AF580` iterates count and rewrites each live slave `+0x2DC` | Yes |
| `TechnoClass+0x2DC` on slave | slave's master techno pointer | `PowerUp_Cleanup @ 0x006AF580` writes new owner to each slave | Yes |

## 3. Core Logic

### UI/runtime eligibility gates

`BuildingClass__ShouldShowDeployButton @ 0x0044F5C0` returns false if production queue/count `+0x504 > 0`; otherwise it returns true for already-deployed state or, for non-ConYard buildings, simply `Type+0x408 != 0`.

**Active in YR:** Yes. YAREFN is non-ConYard and has `UndeploysInto=SMIN`, so the deploy/undeploy command is exposed unless the building is in the production-busy guard. Evidence: Ghidra decompile `0x0044F5C0`, INI `rulesmd.ini:13289`.

`BuildingClass__CanUndeployMCV @ 0x00449BC0` returns true immediately for any building with `Type+0x408 != 0` and `Type+0x16B9 == 0`; the multiplayer/player/`MCVRedeploys`/power-link chain is only entered for `ConstructionYard=yes`.

**Active in YR:** Yes. YAREFN has `UndeploysInto=SMIN` and no `ConstructionYard=yes`, so it is not blocked by `MCVRedeploys`. Evidence: Ghidra decompile `0x00449BC0`, INI `rulesmd.ini:13234..13300`.

`BuildingClass__CanSellOrUndeploy @ 0x004555D0` also guards generic building command legality: no low-power/offline disallow case, no EMP lock, nonzero health, powered gate for certain factory-like flags, no active no-sell timer, and current mission not `0x12` or `0x13`.

**Active in YR:** Yes as a generic building-side command gate. It is not YAREFN-specific, but YAREFN reaches it through the same building command infrastructure. Evidence: Ghidra decompile `0x004555D0`.

### Reverse conversion body

In `BuildingClass__Sell @ 0x00449C30`, state 2 waits until `building+0x6DD != 0` (reverse/opening animation complete). Then, if `Type+0x408 != 0` and the same ConYard-special chain passes or is skipped, it allocates `0x8E8` bytes and calls `UnitClass__Constructor(Type+0x408, Owner)`.

**Active in YR:** Yes. `[YAREFN] Type+0x408` points at `SMIN`, and `Type+0x16B9` is false, so this branch constructs an SMIN. Evidence: `BuildingClass__Sell @ 0x00449C30`, `UnitClass__Constructor @ 0x007353C0`, INI `rulesmd.ini:13289`.

### Origin and facing

After constructing the unit, `BuildingClass__Sell` chooses the spawn coordinate from the source building foundation:

```text
if foundation_width < 3 and foundation_height < 3:
    spawn_coord = building.Location_X/Y/Z
else:
    spawn_coord = centered/adjusted coordinate using DAT_0089F6F0/DAT_0089F6F4 and +0x80 cell centers
```

YAREFN is `Foundation=2x2`, so the stock path uses the YAREFN building coordinate directly. It does not apply the large-foundation centering adjustment.

Facing comes from `Deploy_facing_calculator @ 0x00465D70`, which is a direct read of `TechnoTypeClass+0xEDC`. For YAREFN this is `[YAREFN] DeployFacing=0`, and that value is passed to the new unit's placement/unlimbo vtable call.

**Active in YR:** Yes. Evidence: `BuildingClass__Sell @ 0x00449C30` width/height branch; `Deploy_facing_calculator @ 0x00465D70`; `artmd.ini:1799..1804`; `rulesmd.ini:13291`.

### Placement/path checks

The reverse path does not run the forward `UnitClass__Deploy` building-placement validator. It constructs the SMIN and then calls the new unit's vtable `+0xD8` with the computed coordinate and facing. If that placement/unlimbo call returns false, the successful-transfer block is skipped.

No pre-conversion path search for an alternate SMIN cell was found in this slice. The only alternate-cell logic decompiled nearby was in `BuildingClass__Mission_Hunt @ 0x0044D880`, which is for deployed contents/slave/vehicle ejection and not the `UndeploysInto=SMIN` converter.

**Active in YR:** Yes for the direct placement attempt; no evidence of a YAREFN-specific alternate-cell probe in this branch. Evidence: `BuildingClass__Sell @ 0x00449C30`; comparison with `BuildingClass__Mission_Hunt @ 0x0044D880`.

### SlaveManager transfer

The new SMIN constructor calls `TechnoClass__Init_Managers @ 0x006F3F40`; because `[SMIN] Enslaves=SLAV`, a fresh manager may be created first. Immediately after successful placement and health transfer, `BuildingClass__Sell` checks old YAREFN `+0x2D8` and calls `PowerUp_Cleanup` with the new unit as owner.

`PowerUp_Cleanup @ 0x006AF580` performs the same owner transfer used on forward deploy:

```text
old_owner->SlaveManager = null
manager->owner = new_smin
if new_smin already has a manager: cleanup/destroy it
new_smin->SlaveManager = old_manager
for each live slave in manager array:
    slave->master = new_smin
```

**Active in YR:** Yes. Both stock forms declare `Enslaves=SLAV`; the transfer path is conditional on the old YAREFN actually having `+0x2D8 != 0`. Evidence: `BuildingClass__Sell @ 0x00449C30`, `PowerUp_Cleanup @ 0x006AF580`, `UnitClass__Constructor @ 0x007353C0`, `rulesmd.ini:9099..9102`, `rulesmd.ini:13279..13282`.

## 4. INI Keys

| Section/key | Stock value | Use in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[YAREFN] UndeploysInto` | `SMIN` | Enables building-side reverse conversion via `Type+0x408` | `rulesmd.ini:13289`; `BuildingClass__Sell @ 0x00449C30` | Yes |
| `[YAREFN] DeployFacing` | `0` | Facing passed to new SMIN placement | `rulesmd.ini:13291`; `0x00465D70` | Yes |
| `[YAREFN] Enslaves` | `SLAV` | Allows manager allocation on building form and old-manager transfer source | `rulesmd.ini:13279`; `0x006F3F40` | Yes |
| `[YAREFN] SlavesNumber/SlaveRegenRate/SlaveReloadRate` | `5/500/25` | Manager constructor inputs if manager is allocated | `rulesmd.ini:13280..13282`; `0x006F3F40` | Yes |
| `[SMIN] Enslaves` | `SLAV` | New SMIN constructor can create a manager before transfer cleanup replaces it | `rulesmd.ini:9099`; `0x007353C0`, `0x006F3F40`, `0x006AF580` | Yes |
| `[YAREFN] Foundation` | `2x2` | Selects direct building-coordinate spawn, no centering adjustment | `artmd.ini:1804`; `BuildingClass__Sell @ 0x00449C30` | Yes |
| `[YAREFN] DeploySound` | `SlaveMinerUndeploy` | Command/transition sound data, not a placement rule | `rulesmd.ini:13298` | Yes |
| `[YAREFN] VoiceDeploy` | `SlaveMinerUnDeployVoice` | Command voice data, not path selection | `rulesmd.ini:13299` | Yes |

## 5. Integration Points

| Path | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Button visibility | Non-ConYard with `UndeploysInto` passes after production-count guard | `0x0044F5C0` | Yes |
| Runtime undeploy capability | Non-ConYard with `UndeploysInto` returns true without `MCVRedeploys` | `0x00449BC0` | Yes |
| Generic sell/undeploy command legality | Health, EMP, power/timer, and mission-state guards | `0x004555D0` | Yes |
| Reverse conversion | `BuildingClass__Sell` state 2 constructs `UnitClass` from `Type+0x408` | `0x00449C30`, `0x007353C0` | Yes |
| Spawn placement | New unit vtable `+0xD8` receives coordinate and facing | `0x00449C30`, `0x00465D70` | Yes |
| Manager transfer | Old manager moves to new SMIN; any fresh SMIN manager is removed | `0x006AF580` | Yes, conditional on old `+0x2D8 != 0` |

## 6. Current Rust Implementation Status

Current Rust has two relevant reverse paths:

- Generic building undeploy starts `BuildingDown` and computes a center cell from the foundation: `src/sim/world/world_spawn.rs:590..632`, `src/sim/world/world_spawn.rs:687..` and completion in `src/sim/world/mod.rs:955..1004`.
- A separate `undeploy_slave_miner` helper despawns bound slaves, despawns YAREFN, and spawns SMIN at the same cell: `src/sim/slave_miner.rs:513..551`.

Compared to gamemd for stock YAREFN, the separate same-cell helper matches the `2x2` origin outcome, but its slave handling differs: gamemd transfers the existing manager and rewrites live slaves' master pointer; the Rust helper removes bound slaves instead of carrying a manager/slave set across the conversion. This report makes no code changes.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock YAREFN/SMIN INI identity | verified | `rulesmd.ini:9042..9106`, `13234..13300`; `artmd.ini:1799..1804` | none |
| `UndeploysInto` parser field | verified | string anchor `UndeploysInto @ 0x00844170`; `TechnoTypeClass__ReadINI`; field `+0x408` consumers | none |
| Button visibility | verified | `BuildingClass__ShouldShowDeployButton @ 0x0044F5C0` | exact sidebar event dispatch not traced |
| Runtime undeploy capability | verified | `BuildingClass__CanUndeployMCV @ 0x00449BC0` | none for YAREFN/non-ConYard gating |
| Generic command legality | touched-not-exhausted | `BuildingClass__CanSellOrUndeploy @ 0x004555D0` | all caller/UI contexts outside scope |
| Reverse conversion body | verified | `BuildingClass__Sell @ 0x00449C30` state 2 | exact animation frame count for reverse buildup outside scope |
| SMIN constructor | verified | `UnitClass__Constructor @ 0x007353C0` | full constructor side effects outside scope |
| YAREFN origin/facing | verified | `BuildingClass__Sell @ 0x00449C30`; `Deploy_facing_calculator @ 0x00465D70` | none for stock `2x2` YAREFN |
| Placement/unlimbo result | touched-not-exhausted | `BuildingClass__Sell @ 0x00449C30` calls new unit vtable `+0xD8` | concrete `UnitClass` vtable `+0xD8` body not decompiled in this slot |
| SlaveManager transfer | verified | `PowerUp_Cleanup @ 0x006AF580` | none for owner/slave pointer transfer |
| Building `Mission_Hunt` deployed-content ejection | touched-not-exhausted | `BuildingClass__Mission_Hunt @ 0x0044D880` | out of scope except to exclude it as the reverse converter |
| Full slave harvesting economy | deferred | scope limit | separate slave-economy investigation |
| Ordinary ConYard MCV redeploy | deferred | scope limit; only gate comparison used | use MCV-specific report |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Does YAREFN reverse through `UnitClass__Deploy`? No; `UnitClass__Deploy @ 0x007393C0` is forward `DeploysInto` building creation, while YAREFN reverse is `BuildingClass__Sell @ 0x00449C30` using `Type+0x408`. Evidence: decompiles `0x007393C0`, `0x00449C30`.

[RESOLVED] OQ2 - Is YAREFN subject to multiplayer `MCVRedeploys`? No; that chain is only for `Type+0x16B9 != 0`, and stock YAREFN is not a `ConstructionYard`. Evidence: `0x00449BC0`, `0x0044F5C0`, INI `rulesmd.ini:13234..13300`.

[RESOLVED] OQ3 - Where is the SMIN origin selected? In `BuildingClass__Sell`; `2x2` foundations use source building coordinate directly. Evidence: `0x00449C30`, `artmd.ini:1804`.

[RESOLVED] OQ4 - Where is facing selected? `Deploy_facing_calculator @ 0x00465D70` returns source type `+0xEDC`; YAREFN sets `DeployFacing=0`. Evidence: `0x00465D70`, `rulesmd.ini:13291`.

[RESOLVED] OQ5 - How does `SlaveManager` transfer back? `PowerUp_Cleanup @ 0x006AF580` clears old owner, replaces/destroys any new-owner manager, writes old manager to new SMIN, then rewrites slave master pointers. Evidence: `0x006AF580`, `0x00449C30`.

[RESOLVED] OQ6 - What placement/path checks apply before undeploy? Button/runtime/generic gates run before mission; actual placement is one new-unit vtable `+0xD8` attempt at computed coordinate/facing after animation. No YAREFN-specific alternate path search found. Evidence: `0x0044F5C0`, `0x00449BC0`, `0x004555D0`, `0x00449C30`.

[DEFERRED] OQ7 - What exact function body backs UnitClass vtable `+0xD8` for the SMIN unlimbo attempt? Category: bounded-cost-too-high. The caller contract and success/fail branch are verified; full `UnitClass` placement internals belong in a placement/unlimbo slice.

## Sources

- Ghidra decompiled: `BuildingClass__Sell @ 0x00449C30`, `BuildingClass__CanUndeployMCV @ 0x00449BC0`, `BuildingClass__ShouldShowDeployButton @ 0x0044F5C0`, `BuildingClass__CanSellOrUndeploy @ 0x004555D0`, `PowerUp_Cleanup @ 0x006AF580`, `TechnoClass__Init_Managers @ 0x006F3F40`, `UnitClass__Constructor @ 0x007353C0`, `UnitClass__Deploy @ 0x007393C0`, `UnitClass__DeployHelper @ 0x0073EFC0`, `BuildingClass__Mission_Hunt @ 0x0044D880`, `Deploy_facing_calculator @ 0x00465D70`.
- Ghidra string anchor: `UndeploysInto @ 0x00844170` referenced by `TechnoTypeClass__ReadINI`.
- INI checked: `ini/rulesmd.ini:9042..9106`, `ini/rulesmd.ini:13234..13300`, `ini/artmd.ini:1799..1804`.
- Prior context read: `SLAVE_MINER_DEPLOY_SMIN_YAREFN_PATH_GHIDRA_REPORT.md`, `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`, `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`.
- Rust scan only: `src/sim/world/world_spawn.rs:590..632`, `src/sim/world/mod.rs:955..1004`, `src/sim/slave_miner.rs:513..551`.
