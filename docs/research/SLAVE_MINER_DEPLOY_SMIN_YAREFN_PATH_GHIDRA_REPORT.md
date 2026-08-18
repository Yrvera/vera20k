# Slave Miner SMIN -> YAREFN Deploy Path - Ghidra Research Report

**Address(es):** `0x007393C0` (`UnitClass::Deploy`), `0x006AFD60` (slave-manager deploy/relocation state), `0x006AF580` (slave-manager owner transfer), `0x006B0300` (slave-manager deploy-cell search)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** stock YR `[SMIN] -> [YAREFN]` deploy path, origin cell, minimal placement/height behavior, and whether generic `DeploysInto=` and slave-manager lifecycle both participate.
**Non-Scope:** full slave economy, refinery unload/deposit math, full undeploy behavior, slave harvesting loop details, and unrelated generic MCV deploy cases.
**Confidence:** High
**Active in YR:** Yes - stock `rulesmd.ini` defines `[SMIN] DeploysInto=YAREFN`, `[SMIN]/[YAREFN] Enslaves=SLAV`, and `artmd.ini` defines `[YAREFN] Foundation=2x2`.

## 1. Overview

Stock YR SMIN deploy uses both systems, not an either/or path. The physical unit-to-building conversion is the generic `UnitClass::Deploy` `DeploysInto=` path, and the slave-miner lifecycle is layered on top by transferring the existing `SlaveManagerClass` from the SMIN unit to the new YAREFN building.

The YAREFN origin is the SMIN unit cell. `UnitClass::Deploy` only applies the northwest `(-1,-1)` origin adjustment when the target foundation width or height is greater than 2; YAREFN is `2x2`, so the branch keeps the unit cell unchanged.

## 2. Key Offsets

| Offset | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass+0x2D8` | `SlaveManagerClass*` on SMIN/YAREFN | `TechnoClass__Init_Managers @ 0x006F3F40`; transfer in `PowerUp_Cleanup @ 0x006AF580` | Yes - created when type `+0xD40` (`Enslaves`) is non-null |
| `TechnoTypeClass+0xD40` | `Enslaves` type pointer | `TechnoClass__Init_Managers @ 0x006F3F40` reads `+0xD40` before constructing `SlaveManagerClass` | Yes - `[SMIN]` and `[YAREFN]` both set `Enslaves=SLAV` |
| `TechnoTypeClass+0xD44/+0xD48/+0xD4C` | `SlavesNumber`, `SlaveRegenRate`, `SlaveReloadRate` constructor args | `TechnoClass__Init_Managers @ 0x006F3F40`; `SlaveManagerClass__Constructor @ 0x006AF1A0` | Yes - stock values are `5`, `500`, `25` |
| `UnitTypeClass+0x404` | `DeploysInto` target building type | `UnitClass::Deploy @ 0x007393C0`; `Mission_Deploy_Building @ 0x0073D630` | Yes - `[SMIN] DeploysInto=YAREFN` |
| `BuildingTypeClass+0xEF0` foundation tables | width/height helper input | `GetFoundationWidth @ 0x0045EC90`, `GetFoundationHeight @ 0x0045ECA0`; assembly at `0x00739472..0x00739491` | Yes - `[YAREFN] Foundation=2x2` |

## 3. Core Logic

### Generic conversion

`UnitClass::Mission_Deploy_Building @ 0x0073D630` reaches the non-harvester `DeploysInto != NULL` state machine for any unit type with a deploy target. In state 1, after the locomotor is stopped, it calls `UnitClass::Deploy`.

**Active in YR:** Yes. `[SMIN]` has `DeploysInto=YAREFN` and is neither `Harvester=yes` nor `Weeder=yes` in stock `rulesmd.ini`, so it fits this generic non-harvester branch when its mission is Deploy.

### Origin rule

`UnitClass::Deploy @ 0x007393C0`:

```text
unit_cell = unit.GetCell()
target = unit.Type.DeploysInto
if target.foundation_width > 2 or target.foundation_height > 2:
    origin = unit_cell + (-1,-1)
else:
    origin = unit_cell
building_coord = origin * 256 + 128, z = 0
new BuildingClass(target).Unlimbo(building_coord)
```

Evidence: assembly `0x0073945A` calls the unit cell vtable slot; `0x00739472` calls width helper; `0x00739477 CMP EAX,0x2 / 0x0073947A JG 0x00739499`; `0x00739489` calls height helper; `0x0073948E CMP EAX,0x2 / 0x00739491 JG 0x00739499`; the no-adjust path stores the original cell at `0x00739493`; the adjust path adds `DAT_0089F6A4/6` at `0x00739499..0x007394B1`. `0x007396DF..0x00739711` converts the selected origin cell to leptons with `+0x80` and passes z=0 to the building unlimbo vtable call.

**Active in YR:** Yes. This is the `UnitClass::Deploy` reached by stock `DeploysInto=` units. For SMIN specifically, `artmd.ini:1799..1804` sets `[YAREFN] Foundation=2x2`, so both comparisons are false and origin is the SMIN cell.

### Placement/height checks on this path

Before construction, `UnitClass::Deploy` calls a target-building placement check after calculating the origin. If placement fails, it resets locomotor state and either plays human feedback or asks AI placement logic before falling back to Guard. After construction it calls the new building's unlimbo vtable with coordinate z=0. No SMIN-specific placement formula appears in `UnitClass::Deploy`; SMIN inherits the same generic `DeploysInto=` checks.

**Active in YR:** Yes. Evidence is `UnitClass::Deploy @ 0x007394D8..0x0073956B` for placement failure handling and `0x007396DF..0x00739711` for building unlimbo coordinate.

### Slave-manager transfer

If the deploying unit has a slave manager at `+0x2D8`, `UnitClass::Deploy` calls `0x006B0D10` then `0x006AF580` after the YAREFN is created and health is copied. `0x006AF580` clears the old owner's `+0x2D8`, changes manager owner `+0x24` to the new building, destroys any pre-existing manager on the new building, writes the manager into the new building's `+0x2D8`, and rewrites each live slave's master pointer at `+0x2DC`.

**Active in YR:** Yes. Evidence: `UnitClass::Deploy` assembly `0x00739956 TEST [EBP+0x2D8]`, `0x00739960 CALL 0x006B0D10`, `0x0073996C CALL 0x006AF580`; `PowerUp_Cleanup @ 0x006AF580` performs the owner-pointer transfer. This is the binary implementation of the stock INI comment about both SMIN and YAREFN having `Enslaves=` and the "brain transplant" avoiding an extra manager.

### Slave-manager deployment/AI lifecycle

The manager-level state machine at `0x006AFD60` calls `UnitClass::Deploy` from state 2 and state 3 when a mobile slave miner reaches a deploy cell. State 5 deploys slaves when the owner is already a deployed building and may trigger relocation using `SlaveMinerLongScan` and `SlaveMinerScanCorrection`.

**Active in YR:** Yes. Evidence: `0x006AFD60` state 2/3 call `UnitClass::Deploy`; state 5 calls `SlaveManagerClass__DeploySlaves @ 0x006B04C0` unless relocation state 6 is entered. `rulesmd.ini:313..317` defines the scan and correction keys used by this state machine.

## 4. INI Keys

| Section/key | Stock value | Use in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[SMIN] DeploysInto` | `YAREFN` | Enables generic `UnitClass::Deploy` conversion | `ini/rulesmd.ini:9097`; binary `UnitTypeClass+0x404` | Yes |
| `[SMIN] DeployFacing` | `0` | Generic deploy facing target | `ini/rulesmd.ini:9098`; `Deploy_facing_calculator` called by `UnitClass::Deploy` | Yes |
| `[SMIN] Enslaves` | `SLAV` | Causes `TechnoClass__Init_Managers` to create a slave manager on mobile SMIN | `ini/rulesmd.ini:9099`; `TechnoClass__Init_Managers @ 0x006F3F40` | Yes |
| `[SMIN] SlavesNumber/SlaveRegenRate/SlaveReloadRate` | `5/500/25` | Constructor args for the manager | `ini/rulesmd.ini:9100..9102`; `SlaveManagerClass__Constructor @ 0x006AF1A0` | Yes |
| `[YAREFN] Enslaves` | `SLAV` | Building form is also eligible for manager construction, but deploy transfer destroys any pre-existing manager before installing SMIN's manager | `ini/rulesmd.ini:13279..13284`; `PowerUp_Cleanup @ 0x006AF580` | Yes |
| `[YAREFN] UndeploysInto` | `SMIN` | Reverse direction, not fully investigated here | `ini/rulesmd.ini:13289` | Yes; undeploy path is out of scope |
| `[YAREFN] Foundation` | `2x2` | Makes generic deploy origin equal to the unit cell | `ini/artmd.ini:1799..1804`; `UnitClass::Deploy @ 0x00739472..0x007394B1` | Yes |

## 5. Integration Points

| Path | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Player/mission deploy | `Mission_Deploy_Building` generic `DeploysInto` branch calls `UnitClass::Deploy` once movement has stopped | `0x0073D630`, call near `0x0073DDCB` | Yes - stock SMIN has `DeploysInto=YAREFN` |
| Slave-manager AI relocation | manager state 2/3 calls `UnitClass::Deploy`; state 1 finds a deploy cell first | `0x006AFD60`; `FindDeployCell @ 0x006B0300` | Yes - manager exists on stock SMIN/YAREFN |
| Brain transfer | SMIN manager is moved to YAREFN after conversion | `0x00739956..0x0073996C`; `0x006AF580` | Yes - active when old SMIN has `+0x2D8 != 0` |
| Deployed slave spawn | manager state 5 calls `DeploySlaves`; deployed owner uses east edge + vertical center as slave launch center | `DeploySlaves @ 0x006B04C0`; `GetDeployCenter @ 0x006B0690` | Yes - active after YAREFN is stable/deployed |

## 6. Current Rust Implementation Status

Current Rust has two relevant paths:

- Generic `deploy_mcv` now uses the same origin rule as gamemd for foundations `>2`, otherwise same cell: `src/sim/world/world_spawn.rs:662..669`.
- A separate `deploy_slave_miner` helper spawns YAREFN at the same cell and creates Rust slave bindings immediately: `src/sim/slave_miner.rs:436..500`. This is structurally different from gamemd's binary path, where `UnitClass::Deploy` performs the conversion and the existing manager is transferred.

This report does not change Rust files.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock SMIN/YAREFN INI identity | verified | `ini/rulesmd.ini:9042..9106`, `13234..13299`; `ini/artmd.ini:1799..1804` | none |
| Generic deploy origin for YAREFN | verified | `UnitClass::Deploy @ 0x007393C0`; assembly `0x0073945A..0x007394B1` | none |
| Building unlimbo coordinate | verified | assembly `0x007396DF..0x00739711` | none for origin/z in this path |
| Placement failure handling | touched-not-exhausted | `UnitClass::Deploy @ 0x007394D8..0x0073956B` | full validator internals are out of scope |
| Slave-manager construction from `Enslaves` | verified | `TechnoClass__Init_Managers @ 0x006F3F40`; `SlaveManagerClass__Constructor @ 0x006AF1A0` | none for this deploy path |
| Slave-manager transfer during deploy | verified | `UnitClass::Deploy @ 0x00739956..0x0073996C`; `PowerUp_Cleanup @ 0x006AF580` | none |
| Slave-manager AI deploy call | verified | `0x006AFD60` state 2/3 call `UnitClass::Deploy` | none |
| Full slave harvest/deposit economy | deferred | outside requested scope | investigate separately if needed |
| YAREFN undeploy-to-SMIN | deferred | `[YAREFN] UndeploysInto=SMIN` only noted | separate bounded slice |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does SMIN use generic `UnitClass::Deploy` origin rules? Yes. Evidence: `[SMIN] DeploysInto=YAREFN`; `Mission_Deploy_Building @ 0x0073D630` calls `UnitClass::Deploy`; manager state `0x006AFD60` also calls `UnitClass::Deploy`. Active in YR: Yes.

[RESOLVED] OQ-2 - What is the deployed YAREFN origin cell? The SMIN unit cell. Evidence: `UnitClass::Deploy @ 0x00739472..0x007394B1`; `[YAREFN] Foundation=2x2` at `ini/artmd.ini:1804`. Active in YR: Yes.

[RESOLVED] OQ-3 - Is there a separate slave-miner lifecycle? Yes, via `SlaveManagerClass`, but it wraps/transfers around generic deploy instead of replacing it. Evidence: manager construction at `0x006F3F40`, transfer at `0x006AF580`, manager state machine at `0x006AFD60`. Active in YR: Yes.

[RESOLVED] OQ-4 - Does AI/manager-driven relocation use a different conversion routine? No; manager state 2/3 calls `UnitClass::Deploy`. It uses `FindDeployCell` before the call, but final conversion/origin is still generic. Evidence: `0x006AFD60`, `0x006B0300`. Active in YR: Yes.

[DEFERRED] OQ-5 - Exact internals of target building placement validator and all height/occupy tests. Reason: only the SMIN deploy-path call sites and z/origin were needed for this slice; full placement validation belongs to the building placement validator slot. Category: out-of-scope.

## Sources

- Ghidra: `UnitClass::Deploy @ 0x007393C0`; assembly contexts `0x0073945A..0x007394B1`, `0x007396DF..0x00739711`, `0x00739956..0x0073996C`.
- Ghidra: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra: `TechnoClass__Init_Managers @ 0x006F3F40`; `SlaveManagerClass__Constructor @ 0x006AF1A0`; `PowerUp_Cleanup @ 0x006AF580`; `SlaveManagerClass__FindDeployCell @ 0x006B0300`; `SlaveManagerClass__DeploySlaves @ 0x006B04C0`; `SlaveManagerClass__GetDeployCenter @ 0x006B0690`; manager state machine `0x006AFD60`.
- INI: `ini/rulesmd.ini:313..317`, `9042..9106`, `13234..13299`; `ini/artmd.ini:1799..1804`.
- Current Rust read-only comparison: `src/sim/world/world_spawn.rs:662..669`; `src/sim/slave_miner.rs:436..500`.
