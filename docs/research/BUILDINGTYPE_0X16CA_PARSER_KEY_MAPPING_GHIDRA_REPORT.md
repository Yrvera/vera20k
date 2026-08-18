# BuildingType +0x16CA Parser Key Mapping -- Ghidra Research Report

**Address(es):** `0x0045FE50` (`BuildingTypeClass_ReadINI_Water`), `0x00460F43` (`Artillary` parser site), `0x00460B7D` (`TickTank` parser site), `0x00739801` (`UnitClass::Deploy` post-placement facing update), `0x00443C0D` (`BuildingClass::ToggleGate` adjacent gate), `0x007089D0` (`TechnoClass::ShouldRetaliate` deploy-artillery branch)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact INI key/parser mapping for `BuildingTypeClass+0x16CA`, distinction from `+0x16C4`, constructor defaults, and bounded liveness in standard YR MCV/deploy-adjacent checks.  
**Non-Scope:** full TS tick tank/artillery behavior, exact semantic names for every neighboring BuildingType flag, and implementation of dormant TS deployer mechanics.  
**Confidence:** High for parser mapping/defaults and stock-INI absence; Medium for semantic naming of non-stock behavior because no runtime TS scenario was executed.  
**Active in YR:** Conditional/dormant. The code paths exist in `gamemd.exe`, but standard YR stock INI sets neither `Artillary=` nor `TickTank=` on any building, including `GACNST/NACNST/YACNST/YAREFN`.

## 1. Overview

`BuildingTypeClass+0x16CA` is parsed from the misspelled INI key `Artillary=`, not `Artillery=`. It is a separate boolean from `BuildingTypeClass+0x16C4`, which is parsed from `TickTank=`.

Both flags default to false in the constructor. Standard YR does not set either key in `rules.ini`, `rulesmd.ini`, `art.ini`, or `artmd.ini`; therefore these flags do not affect stock AMCV/SMCV/PCV deploy behavior or stock build-adjacency behavior.

## 2. Class Layout / Key Offsets

| Class | Offset | Type | INI key | Default | Evidence | Active in standard YR |
|---|---:|---|---|---|---|---|
| `BuildingTypeClass` | `+0x16C4` | byte/bool | `TickTank=` | `0` | ctor `0x0045E163`; parser `0x00460B7D -> 0x00460B95` | No stock content sets it |
| `BuildingTypeClass` | `+0x16CA` | byte/bool | `Artillary=` | `0` | ctor `0x0045E187`; parser `0x00460F43 -> 0x00460F56` | No stock content sets it |
| `BuildingTypeClass` | `+0x16C9` | byte/bool | `ICBMLauncher=` | `0` | parser `0x00460F29 -> 0x00460F36` | Neighbor only; distinct from `+0x16CA` |

## 3. Core Logic

### Parser mapping

In `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, the boolean reader is `0x005295F0`. The parser pushes the prior field value as the default, pushes the key string pointer, and writes `AL` back to the byte field.

- `TickTank=`: `0x00460B70` loads `[EBP+0x16C4]`; `0x00460B7D` pushes key string `0x0081A9C8`; `0x00460B85` calls bool parser; `0x00460B95` stores `AL` to `[EBP+0x16C4]`.
- `Artillary=`: `0x00460F3C` loads `[EBP+0x16CA]`; `0x00460F43` pushes key string `0x0081A828`; `0x00460F4B` calls bool parser; `0x00460F56` stores `AL` to `[EBP+0x16CA]`.
- `ICBMLauncher=` sits between nearby fields: `0x00460F29` pushes `0x0081A834`; `0x00460F36` stores to `[EBP+0x16C9]`. This confirms `Artillary` is not an `ICBMLauncher` alias.

The retail executable string table contains `Artillary` exactly at `0x0081A828` (misspelled); this investigation found no `Artillery` key string in the stock executable or stock INIs.

### Constructor defaults

`BuildingTypeClass::constructor @ 0x0045DD90` initializes the relevant bytes to zero:

- `0x0045E163`: `[ESI+0x16C4] = 0`
- `0x0045E181`: `[ESI+0x16C9] = 0`
- `0x0045E187`: `[ESI+0x16CA] = 0`

### Deploy-adjacent/read sites in scope

`UnitClass::Deploy @ 0x007393C0` reads the newly created building's `BuildingTypeClass` after placement succeeds:

- `0x007397FB`: load `new_building.Type`
- `0x00739801`: read `Type+0x16CA`
- `0x0073980B`: if `+0x16CA` is false, read `Type+0x16C4`
- `0x00739815..0x00739827`: if either flag is true, set a local facing word to `0x4000` and call `FacingClass::UpdateFacing @ 0x004C9300` on the new building's facing object.

This branch is live code in YR, but dormant for stock MCV deploy because `GACNST/NACNST/YACNST` do not set `Artillary=` or `TickTank=`.

`BuildingClass::ToggleGate @ 0x00443B90` has a similar adjacent gate:

- `0x00443C0D`: read `Type+0x16C4`
- `0x00443C17`: read `Type+0x16CA`
- if either flag is true and other mission/operation predicates allow it, it calls vtable `+0x1E8` with mission `0x13` and then vtable `+0x1EC`.

This is not stock MCV deploy behavior. It is a generic building branch that remains dormant unless a building type opts into either TS legacy flag.

`TechnoClass::ShouldRetaliate @ 0x007087C0` reads only `+0x16CA` in a deploy-target branch:

- `0x007089D0`: load `unit_type+0x404` (`DeploysInto`)
- `0x007089DA`: read `[DeploysInto + 0x16CA]`

This is consistent with a TS deployed-artillery special case. Standard YR MCVs have `DeploysInto=GACNST/NACNST/YACNST`, and those building types have default false `+0x16CA`, so this read does not make stock ConYard deploys artillery-like.

## 4. INI Keys

| INI key | Parser address | String address | Field | Stock RA2/YR value | Effect |
|---|---:|---:|---:|---|---|
| `Artillary` | `0x00460F43` | `0x0081A828` | `BuildingType+0x16CA` | absent everywhere checked | Enables TS artillery-adjacent special branches when explicitly set |
| `Artillery` | none found | none found | none | absent | Not a recognized stock key in `gamemd.exe` |
| `TickTank` | `0x00460B7D` | `0x0081A9C8` | `BuildingType+0x16C4` | absent everywhere checked | Separate TS tick-tank flag; can share facing-update consumers |
| `ICBMLauncher` | `0x00460F29` | `0x0081A834` | `BuildingType+0x16C9` | absent in stock YR building sections checked by this slice | Neighbor field; not conflated with `Artillary` |

Stock files checked with exact-key grep: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`. No `Artillary=`, `Artillery=`, or `TickTank=` assignments were found.

## 5. Integration Points

- `UnitClass::Deploy` uses `+0x16CA || +0x16C4` only after successful building placement, not in the AMCV facing precheck and not in footprint validation.
- `GACNST` deploy facing does not require `+0x16CA`; the stock AMCV facing gate uses target building `DeployFacing` at `BuildingType+0xEDC`, verified in the prior AMCV facing report.
- `+0x16CA` is separately consulted by `TechnoClass::ShouldRetaliate` through `UnitType.DeploysInto`, which is the clearest deploy-artillery semantic consumer in this slice.

## 6. Current Rust Implementation Status

`src/rules/object_type.rs` does not parse `Artillary=` or `TickTank=`. That is acceptable for stock YR behavior because no stock content sets either key and no standard ConYard/MCV path depends on them.

If modded TS-style deployer parity becomes a target, Rust will need explicit fields for these two keys rather than merging them with `DeployFacing`, `Deployer`, `DeploysInto`, or building `Adjacent`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `+0x16CA` parser key | verified | `0x00460F43` push `0x0081A828`; `0x00460F56` write `[EBP+0x16CA]` | none |
| `+0x16C4` parser key | verified | `0x00460B7D` push `0x0081A9C8`; `0x00460B95` write `[EBP+0x16C4]` | none |
| Constructor defaults | verified | `0x0045E163`, `0x0045E187` | none |
| Stock INI assignments | verified | exact grep over `rules.ini`, `rulesmd.ini`, `art.ini`, `artmd.ini` | none |
| `UnitClass::Deploy` post-placement consumer | verified | `0x00739801..0x00739827` | deeper TS behavior outside stock YR deferred |
| `BuildingClass::ToggleGate` adjacent consumer | touched-not-exhausted | `0x00443C0D..0x00443C54` | exact player-visible TS mission behavior if flags set |
| `TechnoClass::ShouldRetaliate` deploy-artillery read | touched-not-exhausted | `0x007089D0..0x007089DA` | exact retaliation behavior for a custom `DeploysInto` building with `Artillary=yes` |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- What key maps to BuildingType+0x16CA? -> The exact key is misspelled `Artillary=`, string `0x0081A828`, parser site `0x00460F43`, write `0x00460F56`.` (evidence: Ghidra xref/disassembly)
- `[RESOLVED] OQ-02 -- Is +0x16CA the same as +0x16C4? -> No. `+0x16C4` is `TickTank=`, string `0x0081A9C8`, parser `0x00460B7D`, write `0x00460B95`.` (evidence: Ghidra xref/disassembly)
- `[RESOLVED] OQ-03 -- Is the correct spelling `Artillery` accepted? -> No accepted `Artillery` key was found in the stock executable string table or stock INI files; only `Artillary` is present.` (evidence: executable string grep; stock INI grep)
- `[RESOLVED] OQ-04 -- Are these fields true by default? -> No. Constructor initializes both `+0x16C4` and `+0x16CA` to zero.` (evidence: `0x0045E163`, `0x0045E187`)
- `[RESOLVED] OQ-05 -- Does stock GACNST deploy need +0x16CA? -> No. Stock ConYard types do not set `Artillary=`, and the AMCV deploy-facing requirement is the target building `DeployFacing` field, not `+0x16CA`.` (evidence: stock INI grep; prior AMCV facing report; `0x00739801` is post-placement)
- `[RESOLVED] OQ-06 -- Is `+0x16CA` live code in YR? -> Conditional. Reads exist in YR `gamemd.exe`, but standard stock content leaves the flag false, so stock gameplay does not exercise the special behavior.` (evidence: `0x00739801`, `0x00443C17`, `0x007089DA`; stock INI grep)
- `[DEFERRED] OQ-07 -- Exact TS artillery/tick-tank player-visible behavior when a custom building sets these flags.` (category: out-of-scope; reason: user scoped this to parser mapping and standard-YR liveness; next-step-if-pursued: trace a custom or TS content scenario with `Artillary=yes`/`TickTank=yes`)
- `[DEFERRED] OQ-08 -- Exact names for all mission/vtable calls in `BuildingClass::ToggleGate` after the flag gate.` (category: out-of-scope; reason: not needed to prove stock MCV/deploy-adjacent non-use; next-step-if-pursued: focused ToggleGate mission trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Artillary=` maps to `BuildingType+0x16CA`; default false; no stock YR content sets it | `0x00460F43`, `0x00460F56`, `0x0045E187`; stock INI grep | none required for stock YR | `src/rules/object_type.rs` only if TS/mod parity is targeted | Leave unimplemented for stock unless adding TS-style deploy artillery support | Stock AMCV deploy to GACNST behaves identically with no `Artillary` field parsed | Do not add `Artillery=` spelling or treat absence as true |
| `TickTank=` maps to `BuildingType+0x16C4`, separate from `Artillary=+0x16CA` | `0x00460B7D`, `0x00460B95`, `0x0045E163` | none required for stock YR | future BuildingType parser fields | If implemented, keep two distinct bools because some consumers OR them and one consumer checks only `Artillary` | Custom INI with only `Artillary=yes` triggers `+0x16CA` consumers without setting `+0x16C4` | Do not collapse both flags into one `deployed_artillery_or_tick_tank` parser field |
| `UnitClass::Deploy` checks `+0x16CA || +0x16C4` only after successful building placement to update building facing to `0x4000` | `0x007397FB..0x00739827`; `FacingClass::UpdateFacing @ 0x004C9300` | current Rust has no dormant TS flag behavior; stock OK | `src/sim/world/world_spawn.rs::deploy_mcv` or future generic deploy conversion | No stock Rust delta; future TS/mod support should apply the facing update after building creation, not as a placement/facing prerequisite | A custom deploy target with `Artillary=yes` receives post-placement facing update; stock GACNST path unchanged | Do not use `+0x16CA` in AMCV placement validation, build adjacency, or pre-deploy facing gate |

### Negative Facts

- `Artillery=` is not the stock key; the executable string and parser xref use `Artillary=`.
- `+0x16CA` is not `TickTank=`; `TickTank=` maps to `+0x16C4`.
- Stock `GACNST/NACNST/YACNST/YAREFN` do not set `Artillary=` or `TickTank=`.
- `+0x16CA` is not part of the AMCV pre-placement facing gate or footprint validation.
- Current stock Rust does not need an `Artillary` parser field to preserve standard YR MCV deploy behavior.

### Remaining Uncertainty

- Exact player-visible TS/custom-mod behavior after `Artillary=yes` or `TickTank=yes` is set remains out of scope.
- `BuildingClass::ToggleGate` mission/vtable side effects after the `+0x16C4 || +0x16CA` gate were touched only enough to prove the gate exists.

### Status

GREEN for parser mapping and standard-YR non-use. No Rust or in-repo doc edits were made.

### Stale Docs / Follow-up Docs

- Replace `BuildingTypeClass +0x16CA = Artillery` with: `BuildingTypeClass +0x16CA = Artillary= (misspelled INI key), default false; parsed at 0x00460F43 and stored at 0x00460F56. Dormant in stock YR because no stock INI sets it.`
- Replace any wording that says `+0x16C4/+0x16CA` have an unresolved or shared key with: `+0x16C4 is TickTank=; +0x16CA is Artillary=. They are distinct fields; some consumers OR them, while TechnoClass::ShouldRetaliate checks +0x16CA through DeploysInto.`
- Replace AMCV/GACNST wording implying `+0x16CA` is needed for deploy facing with: `GACNST deploy facing is driven by BuildingType DeployFacing (+0xEDC); +0x16CA is only a dormant post-placement/TS-artillery-adjacent flag in stock YR.`

## Sources

- Ghidra HTTP read-only queries: `get_xrefs_to 0x0081A828`, `get_xrefs_to 0x0081A9C8`, `disassemble_function 0x0045FE50`, `disassemble_function 0x0045DD90`, `disassemble_function 0x007393C0`, `disassemble_function 0x00443B90`, `disassemble_function 0x007087C0`.
- Retail executable string grep: `Artillary` at file offset `0x41A828` / VA `0x0081A828`; `TickTank` at file offset `0x41A9C8` / VA `0x0081A9C8`.
- Prior docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_MASTER_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/units/AUDIT_INDEX.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`, `rulesmd.ini`, `art.ini`, `artmd.ini`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/rules/object_type.rs`.
