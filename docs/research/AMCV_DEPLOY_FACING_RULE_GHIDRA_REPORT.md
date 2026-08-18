# AMCV Deploy Facing Rule -- Ghidra Research Report

**Address(es):** `0x00465D70` (`Deploy_facing_calculator` accessor), `0x007393C0` (`UnitClass__Deploy`), `0x00460C76` (`DeployFacing` parser site), `0x0045DD90` (`BuildingTypeClass` constructor), `0x0043B740` (`BuildingClass` constructor)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** player-visible AMCV -> GACNST deploy-facing source, default/override interaction, and whether the created GACNST inherits AMCV facing  
**Non-Scope:** CanDeploy predicate details, MCV undeploy/sell path, free unit after construction yard placement, generic simple-deployer facing systems  
**Confidence:** High for AMCV -> GACNST facing source and deploy gate; Medium for non-visible BuildingClass internal facing default  
**Active in YR:** Yes

## 1. Overview

AMCV deploy does not use `[AMCV] DeployFacing` and does not use `[General] DeployDir`. `UnitClass__Deploy` loads the AMCV's `DeploysInto` target (`GACNST`) and passes that target building type to `Deploy_facing_calculator @ 0x00465D70`; the accessor returns `BuildingTypeClass+0xEDC`.

For stock YR, `[GACNST]` has no `DeployFacing=` line, so it keeps the `BuildingTypeClass` constructor default `0x80` (8-bit facing 128, south). The AMCV must reach that rounded current facing before the conversion creates the GACNST.

## 2. Class Layout / Key Offsets

| Class | Offset | Type | Purpose | Active in YR |
|---|---:|---|---|---|
| `UnitClass` | `+0x6C4` | `UnitTypeClass*` | AMCV type pointer used to read `DeploysInto` | Yes; `0x007395BA` |
| `TechnoTypeClass` / `UnitTypeClass` | `+0x404` | `BuildingTypeClass*` | `DeploysInto` target (`GACNST`) | Yes; `0x007395C0`, prior AMCV doc |
| `BuildingTypeClass` | `+0xEDC` | `int` raw 8-bit facing value | `DeployFacing` after `INI_value << 5`; default `0x80` | Yes; `0x00465D70`, `0x00460C76`, `0x0045DEEC` |
| `FootClass`/unit | `+0x388` | `FacingClass`/timer-like object | current body facing sampled before deploy | Yes; `0x007395D2..0x007395E5` |
| `BuildingTypeClass` | `+0x16C4` | byte | `TickTank` flag, can trigger building-facing update after construction | Conditional; not set on `GACNST` |
| `BuildingTypeClass` | `+0x16CA` | byte | adjacent building flag checked with `+0x16C4` for facing update | Conditional; exact key not resolved, not set/needed for stock `GACNST` in this slice |

## 3. Core Logic

The AMCV deploy facing gate in `UnitClass__Deploy` is:

1. Clear/restore occupation around the unit and pass placement prechecks.
2. Load `unit_type = unit+0x6C4`.
3. Load `target_building_type = unit_type+0x404`.
4. Call `Deploy_facing_calculator(target_building_type)`.
5. `Deploy_facing_calculator @ 0x00465D70` returns `*(target_building_type + 0xEDC)`.
6. Read current unit facing through `RateTimer__Current(unit+0x388)`.
7. Quantize current facing with `((current >> 7) + 1) >> 1 & 0xFF`, i.e. rounded 16-bit facing to an 8-bit byte.
8. If quantized current facing differs from the target byte, and the locomotor is not in the blocking state checked by vtable `+0x80`, call locomotor/vtable `+0x4C` with `target << 8`, set mission `3`, mark redraw, and return `1` without creating the building.
9. Only when the rounded current facing equals the target does the path allocate `BuildingClass`, place it, transfer state, destroy/limbo the AMCV, and play the deploy sound.

For stock AMCV -> GACNST: `target_building_type` is `GACNST`; `[GACNST]` has no `DeployFacing`, so `target = 0x80`.

## 4. INI Keys

| INI key | Stock value / location | Binary effect | Active in YR |
|---|---|---|---|
| `[AMCV] DeploysInto` | `GACNST`, `rulesmd.ini:6977` | Supplies `unit_type+0x404`, which is then passed to the facing accessor | Yes |
| `[GACNST] DeployFacing` | absent in stock `rulesmd.ini` | No override; keeps constructor default `0x80` | Yes, by absence/default |
| `[SMIN] DeployFacing` | `0`, `rulesmd.ini:9098` | Example of override path; parser stores `0 << 5 = 0` | Yes for SMIN, out of AMCV scope |
| `[YAREFN] DeployFacing` | `0`, `rulesmd.ini:13291` | Example of building-side override path used by prior YAREFN report | Yes for YAREFN, out of AMCV scope |
| `[General] DeployDir` | `2`, `rulesmd.ini:668` | Read by `RulesClass::ReadGeneral`; no read in AMCV `UnitClass__Deploy` facing gate | No for AMCV deploy; active elsewhere |

The `DeployFacing` parser site at `0x00460C76` reads the current field shifted right by 5 as the default INI value, then shifts the parsed integer left by 5 before storing to `+0xEDC`. That makes INI `0..7` become raw facing bytes `0,32,64,...,224`.

## 5. Integration Points

`UnitClass__Deploy @ 0x007393C0` is entered from the deploy mission path after the unit is eligible to deploy. The facing check happens before `operator_new(0x720)` and `BuildingClass__Constructor`, so a mis-facing AMCV remains a unit and deploy returns in-progress.

The created building does not inherit the AMCV's facing in the verified deploy body. `UnitClass__Deploy` transfers health, unique ID, height, tag/link fields, targeting links, and other state, but no copy from the unit's body-facing field to the new building was found. `BuildingClass__Constructor @ 0x0043B740` initializes the building's own facing/timer state internally, including a later `RateTimer__Set` with `0x4000`; this is independent of the AMCV's required pre-deploy facing. For stock `GACNST`, no visible rotated building facing is expected from this path.

## 6. Current Rust Implementation Status

`src/sim/world/world_spawn.rs::deploy_mcv` currently despawns the MCV and calls `spawn_object_at_height(&yard_type, ..., facing=0, ...)` immediately after placement checks. It does not parse or store `DeployFacing`, does not delay deploy until the AMCV reaches the target building type's facing, and always spawns the new construction yard with Rust facing `0`.

Existing tests in `src/sim/deploy_tests.rs` cover foundation origin and occupancy cases, but not facing-gated deploy.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Deploy_facing_calculator @ 0x00465D70` | verified | decompile returns `*(param+0xEDC)` | none |
| caller setup before `0x00465D70` | verified | assembly `0x007395BA..0x007395C6` loads `ECX=[unit_type+0x404]` then calls accessor | none |
| current-facing comparison formula | verified | assembly `0x007395D2..0x007395EB` | none |
| rotate-and-return branch | verified | decompile/assembly `0x007395EF..0x0073965F` | exact locomotor vtable names out of scope |
| create-building branch after facing match | verified | `UnitClass__Deploy @ 0x00739660+` | none for facing slice |
| `DeployFacing` parser scale | verified | assembly `0x00460C6C..0x00460C86` | none |
| `BuildingTypeClass` default `+0xEDC=0x80` | verified | constructor decompile `0x0045DEEC` / `param_1[0x3B7]=0x80` | none |
| stock `[GACNST]` absence of `DeployFacing` | verified | `rulesmd.ini:11622..11631`, no `DeployFacing` in section | none |
| `[General] DeployDir=2` effect on AMCV deploy | verified negative | `rulesmd.ini:668`; no read in `0x007393C0` facing gate | none |
| exact generic use of `DeployDir` elsewhere | deferred | `RulesClass::ReadGeneral @ 0x0066D530` contains key per prior doc | out-of-scope |
| exact key for `BuildingType+0x16CA` | deferred | checked in `UnitClass__Deploy` with `+0x16C4` | not material to stock `GACNST` facing |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Is 0x00465D70 a stub or the rule body? -> It is a 7-byte accessor body returning `param+0xEDC`; the rule is split between caller setup and this accessor.` (evidence: `0x00465D70`)
- `[RESOLVED] OQ-02 -- What object is passed to 0x00465D70 for AMCV deploy? -> The caller passes `unit_type+0x404`, the `DeploysInto` building type, not the AMCV type itself.` (evidence: `0x007395BA..0x007395C6`)
- `[RESOLVED] OQ-03 -- Does AMCV's own missing `DeployFacing` matter? -> No; `[AMCV]` is not the type passed to the facing accessor in this deploy path.` (evidence: `0x007395C0`; `rulesmd.ini:6969..6977`)
- `[RESOLVED] OQ-04 -- What is GACNST's stock deploy-facing value? -> `GACNST` has no `DeployFacing`, so it keeps `BuildingTypeClass+0xEDC=0x80`.` (evidence: `0x0045DEEC`; `rulesmd.ini:11622..11631`)
- `[RESOLVED] OQ-05 -- How are `DeployFacing` INI values scaled? -> Parser default is current field `>>5`; parsed value is stored as `value << 5`.` (evidence: `0x00460C6C..0x00460C86`)
- `[RESOLVED] OQ-06 -- Does `[General] DeployDir=2` feed this AMCV deploy gate? -> No read appears in the verified `UnitClass__Deploy` facing gate; the gate uses target building type `+0xEDC`.` (evidence: `0x007395BA..0x007395EB`; `rulesmd.ini:668`)
- `[RESOLVED] OQ-07 -- What current-facing formula is compared? -> `((current >> 7) + 1) >> 1 & 0xFF`, rounded from 16-bit facing to 8-bit byte.` (evidence: `0x007395DD..0x007395EB`)
- `[RESOLVED] OQ-08 -- What happens if current facing mismatches? -> Unit is ordered/kept in mission 3, redraw is set, deploy returns `1`, and no building is created on that call.` (evidence: `0x007395EF..0x0073965F`)
- `[RESOLVED] OQ-09 -- Is the unit facing copied to GACNST after creation? -> No copy from the unit body-facing field was found in `UnitClass__Deploy`; building constructor initializes its own facing/timer state.` (evidence: `0x00739660+`; `0x0043B740`)
- `[RESOLVED] OQ-10 -- Is this live in stock YR, not TS legacy? -> Yes; stock `[AMCV] DeploysInto=GACNST` and `[GACNST] ConstructionYard=yes` drive the standard YR deploy path.` (evidence: `rulesmd.ini:6977`, `rulesmd.ini:11625`, `0x007393C0`)
- `[DEFERRED] OQ-11 -- Exact generic `[General] DeployDir` consumers outside AMCV deploy.` (category: out-of-scope; reason: this slot only needed to prove non-use in AMCV -> GACNST; next-step-if-pursued: investigate simple deployer facing)
- `[DEFERRED] OQ-12 -- Exact INI key for `BuildingType+0x16CA`.` (category: out-of-scope; reason: not needed for stock GACNST facing and not part of AMCV visible deploy-facing rule; next-step-if-pursued: targeted BuildingType parser field audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| AMCV deploy-facing target comes from `DeploysInto` building type `GACNST+0xEDC`, default `0x80`, not `[AMCV]` | `0x007395BA..0x007395C6`, `0x00465D70`, `0x0045DEEC`, `rulesmd.ini:11622` | missing | `src/rules/ruleset.rs` / object type data; `src/sim/world/world_spawn.rs::deploy_mcv` or deploy mission layer | Parse/store building `DeployFacing` as raw `value << 5`; default building value should be `128`; AMCV deploy should use target yard type's value | AMCV facing 64 cannot convert immediately; after facing reaches 128, deploy creates GACNST | `deploy_mcv_waits_for_gacnst_default_deploy_facing_south` | Do not read `[AMCV] DeployFacing`; do not use `[General] DeployDir=2` for AMCV |
| Current facing comparison rounds 16-bit current facing to an 8-bit byte before comparing to target | `0x007395DD..0x007395EB` | missing/unchecked; Rust stores `u8` facing today | `src/sim/movement/facing_class.rs`; deploy mission layer | If Rust keeps `u8`, compare direct byte facings; if using `FacingClass`, match gamemd rounding before allowing conversion | unit at near-south rounded value passes; adjacent non-rounded bucket does not | `deploy_mcv_uses_rounded_8bit_facing_gate` | Do not require exact 16-bit equality when gamemd compares rounded byte |
| GACNST does not inherit AMCV body facing; created building initializes its own facing state | `UnitClass__Deploy @ 0x00739660+`; `BuildingClass__Constructor @ 0x0043B740` | current Rust spawns facing 0; visible impact likely none for GACNST, but no copy rule documented | `src/sim/world/world_spawn.rs::deploy_mcv`; renderer if building facing is ever surfaced | Do not copy AMCV's pre-deploy facing onto the GACNST as a parity rule; if building-facing state is modeled, use BuildingClass constructor/default semantics separately | deploying from AMCV facing 128 creates GACNST without preserving AMCV facing as an inherited render-facing rule | `deploy_mcv_does_not_copy_unit_facing_to_conyard` | Avoid making GACNST orientation depend on approach direction |

### Negative Facts / Do Not Do

- Do not implement AMCV deploy as `[General] DeployDir=2` east-facing. Evidence: AMCV deploy gate passes `GACNST` type to `0x00465D70`; no `DeployDir` read appears in `0x007395BA..0x007395EB`.
- Do not implement AMCV deploy as `[AMCV] DeployFacing` or `UnitTypeClass+0xEDC`. Evidence: caller loads `ECX=[unit_type+0x404]` before calling `0x00465D70`.
- Do not treat `DeployFacing` INI values as raw `0..7` at runtime. Evidence: parser stores `value << 5` at `0x00460C83..0x00460C86`.
- Do not create the GACNST first and rotate afterward. Evidence: facing mismatch returns before `operator_new(0x720)` / `BuildingClass__Constructor`.
- Do not copy AMCV body facing into GACNST as a conversion transfer. Evidence: no facing-field copy found in the verified transfer block; building constructor initializes its own state.

### Stale Docs / Follow-up Docs

- `docs/research/units/allied/AMCV.md`: replace "The unit rotates to match `DeployFacing` from its TechnoType before deploying. AMCV doesn't override `DeployFacing` so it uses the default." with "The unit rotates to match the `DeployFacing` stored on its `DeploysInto` building type. For AMCV, `UnitClass::Deploy` passes `GACNST` to `Deploy_facing_calculator @ 0x00465D70`; stock `GACNST` has no `DeployFacing=`, so the `BuildingTypeClass` constructor default `0x80` (south) is used. `[General] DeployDir=2` and any `[AMCV] DeployFacing` line do not drive AMCV -> GACNST deploy facing."
- `docs/research/BUILDINGTYPECLASS_CTOR_DEFAULTS.md`: optional wording correction: `DeployFacing` default `0x80` is raw byte facing 128 (south in RA2 byte-facing convention), not "North-facing default."

## Sources

- Ghidra decompile: `Deploy_facing_calculator @ 0x00465D70`
- Ghidra assembly context: `UnitClass__Deploy @ 0x007395BA..0x007395EB`
- Ghidra decompile: `UnitClass__Deploy @ 0x007393C0`
- Ghidra assembly context: `BuildingTypeClass_ReadINI_Water @ 0x00460C6C..0x00460C86`
- Ghidra decompile: `BuildingTypeClass__Constructor @ 0x0045DD90`, write at `0x0045DEEC`
- Ghidra decompile: `BuildingClass__Constructor @ 0x0043B740`
- INI checked: `ini/rulesmd.ini`
- Rust scanned: `src/sim/world/world_spawn.rs::deploy_mcv`, `src/sim/deploy_tests.rs`
