# GACNST Free Unit After AMCV Deploy -- Ghidra Research Report

**Address(es):** `0x007393C0` (`UnitClass::Deploy`), `0x00445F80` (`BuildingClass::OnConstructionComplete`), `0x00460540` (`BuildingTypeClass_ReadINI_Water`), `0x00449A50` (`Mission_Construction`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** AMCV-created `GACNST` in standard Yuri's Revenge and whether that deployment creates `CMIN` through `FreeUnit=`, construction complete, deploy special handling, startup, or production logic.
**Non-Scope:** global multiplayer start-unit seeding, AI team production, refinery dock behavior after the first refinery exists, nonstandard mods that add `FreeUnit=` to `GACNST`.
**Confidence:** High for the scoped negative claim.
**Active in YR:** Yes for the AMCV->GACNST deploy path and the `OnConstructionComplete` hook; the `FreeUnit` spawn branch is active in YR but not taken for stock `GACNST`.

## 1. Overview

Deploying an Allied `AMCV` creates a `GACNST`, assigns it mission `0x12` (Construction), unlimbos it, and later lets `Mission_Construction` call `BuildingClass::OnConstructionComplete`. The only verified `FreeUnit=` spawn branch is in `OnConstructionComplete`, reading `BuildingTypeClass+0xEA0`. Standard YR `rulesmd.ini` has no `FreeUnit=` key on `[GACNST]`; `FreeUnit=CMIN` belongs to `[GAREFN]`, so AMCV-created `GACNST` does not create a `CMIN`.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `UnitTypeClass+0x404` | `BuildingTypeClass*` | `DeploysInto=` target for AMCV -> GACNST | `UnitClass::Deploy @ 0x007393C0` reads it before constructing the building | Yes |
| `BuildingTypeClass+0xEA0` | `UnitTypeClass*` | `FreeUnit=` parsed target | `BuildingTypeClass_ReadINI_Water @ 0x00460540` reads `FreeUnit` string and writes `+0xEA0` | Conditional; only sections with `FreeUnit=` |
| `BuildingTypeClass+0x16B9` | byte | `ConstructionYard=yes` | parser reads `ConstructionYard` at `0x00460540`; deploy uses it in target-redirect/special ConYard setup | Yes for `GACNST` |
| `BuildingClass+0x6DD` | byte | construction animation complete handshake | `Mission_Construction @ 0x00449A50` waits for this before completion ritual | Yes |
| `BuildingClass+0x6E4` | byte | `ActuallyPlacedOnMap`, one-shot fence for `OnConstructionComplete` | `OnConstructionComplete @ 0x00445F80` early-outs when already set and `param_2==0`, then sets it | Yes |

## 3. Core Logic

### AMCV deploy path

`UnitClass::Deploy @ 0x007393C0`:

1. Calls vtable `+0x314` (`CanDeploy`); returns `0` if false.
2. Rejects if locomotor pointer is absent/moving.
3. Reads `UnitTypeClass+0x404`; returns `0` if no `DeploysInto`.
4. Calls the target `BuildingTypeClass` placement validator through vtable `+0xA8`.
5. Turns to `DeployFacing` via `Deploy_facing_calculator`; while turning it returns `1` and does not construct the building yet.
6. Allocates `0x720` bytes, calls `BuildingClass::Constructor(DeploysInto, owner)`.
7. Immediately calls the new building's vtable `+0x1E8` with mission `0x12,0`.
8. Calls new building vtable `+0xD8` (`Unlimbo`) with the deployment coordinate.
9. If unlimbo succeeds, runs target redirection and ConYard special setup, transfers state/health/veterancy-like fields, removes/destroys the AMCV, and returns `1`.

No branch in this function reads `BuildingTypeClass+0xEA0`, calls `UnitClass::Constructor` for a `CMIN`, or parses `FreeUnit`.

### Construction completion path

`Mission_Construction @ 0x00449A50`:

1. State `0`: calls `GrandOpening(0)`, sends radio `0x0B`, plays build-up sound if present, sets state `1`.
2. State `1`: waits until `BuildingClass+0x6DD != 0`.
3. On completion: sends radio `0x0C` then `0x03`, calls `GrandOpening(1)`, calls vtable `+0x4DC` (`OnConstructionComplete`) with `param_2=0`, assigns mission `5`, snaps facing if not walkthrough, releases build-up sound.

`BuildingClass::OnConstructionComplete @ 0x00445F80` is the only verified free-unit spawn consumer:

1. It first fenceposts with `ActuallyPlacedOnMap`; if already set and `param_2==0`, it returns.
2. It performs ordinary building-startup side effects, then sets owner dirty flags and `ActuallyPlacedOnMap = true`.
3. It checks:
   - `Type+0xEA0 != 0`
   - `g_MapEditorMode == 0`
   - `param_2 == 0`
   - `g_IsMapEditor == 0`
   - for player-controlled houses with `BuildingClass+0x300 != 0`, a type build-count ceiling must not be reached.
4. If all gates pass, it constructs a `UnitClass` from `Type+0xEA0`, tries `Unlimbo` at the building exit offset plus global `DAT_0089F698`, facing `0xC0`; on failure it tries nearby passable cells with facing `0xA0`; on success it assigns mission `10` and queues it.
5. If allocation/placement ultimately fails, it refunds the unit build cost via `HouseClass__Add_Credits`.

For stock `GACNST`, gate 3 fails immediately because `Type+0xEA0 == 0`.

## 4. INI Keys

| File | Section | Key | Value | Effect in this slice | Active in YR |
|---|---|---|---|---|---|
| `ini/rulesmd.ini:6969-6977` | `[AMCV]` | `DeploysInto` | `GACNST` | feeds `UnitTypeClass+0x404`; AMCV can create GACNST | Yes |
| `ini/rulesmd.ini:11622-11631` | `[GACNST]` | `ConstructionYard` / `Factory` / `UndeploysInto` | `yes` / `BuildingType` / `AMCV` | makes GACNST a ConYard/building factory/undeploy pair | Yes |
| `ini/rulesmd.ini:11622-11651` | `[GACNST]` | `FreeUnit` | absent | `BuildingTypeClass+0xEA0` remains default null; no free unit | Yes, as absence |
| `ini/rulesmd.ini:11722-11736` | `[GAREFN]` | `FreeUnit` | `CMIN` | refinery, not ConYard, is the stock Allied CMIN free-unit source | Yes |
| `ini/rules.ini:8492-8568` | base RA2 | same pattern | `[GACNST]` absent, `[GAREFN] FreeUnit=CMIN` | base fallback agrees with YR | Yes if not overridden |

## 5. Integration Points

| Function | Role | Finding | Active in YR |
|---|---|---|---|
| `UnitClass::Deploy @ 0x007393C0` | AMCV -> GACNST object conversion | creates a building and sets mission `0x12`; no free-unit spawn | Yes |
| `BuildingClass::Unlimbo @ 0x00440580` | places the new GACNST on map | registers house/building state; no `+0xEA0` free-unit consumer | Yes |
| `Mission_Construction @ 0x00449A50` | build-up mission | waits for animation-complete byte then calls `OnConstructionComplete` | Yes |
| `BuildingClass::OnConstructionComplete @ 0x00445F80` | post-build/startup hook | contains `FreeUnit` spawn branch gated by `Type+0xEA0`; skipped for stock GACNST | Yes, branch conditional |
| `BuildingTypeClass_ReadINI_Water @ 0x00460540` | BuildingType parser | `FreeUnit` is section-local BuildingType key, not `[General]` | Yes |

## 6. Current Rust Implementation Status

Codegraph and `rg` found `Simulation::deploy_mcv` in `src/sim/world/world_spawn.rs`, plus refinery free-unit parsing helpers under `src/rules/ruleset.rs`.

Current Rust-facing delta for this slice: do not add any `CMIN` spawn to `deploy_mcv` or GACNST placement. If Rust currently only creates `GACNST` on AMCV deploy, that matches the scoped binary finding for free-unit behavior. Refinery `FreeUnit=` remains a separate implementation surface.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| AMCV deploy creates GACNST | verified | `0x007393C0` reads `UnitType+0x404`, constructs `BuildingClass`, calls mission `0x12` and vtable `+0xD8` | none |
| `UnitClass::Deploy` free-unit spawn absence | verified | full decompile `0x007393C0`; no `+0xEA0` read, no secondary `UnitClass::Constructor` except none | none |
| Construction completion invokes startup hook | verified | `0x00449A50` calls vtable `+0x4DC` after `+0x6DD` is set | none |
| `OnConstructionComplete` free-unit branch | verified | `0x00445F80`, gate on `Type+0xEA0`, map-editor flags, `param_2`, then `UnitClass::Constructor` and `Unlimbo` | none |
| `FreeUnit` parser | verified | `0x00460540`, `CCINIClass__ReadString("FreeUnit")`, `UnitTypeClass__FindOrAllocate`, write `+0xEA0` | none |
| Stock YR GACNST data | verified | `ini/rulesmd.ini:11622-11651`, no `FreeUnit` key | none |
| Stock YR GAREFN data | verified | `ini/rulesmd.ini:11722-11736`, `FreeUnit=CMIN` | none |
| Multiplayer start-unit seeding | deferred | out-of-scope by target | follow separate skirmish/player-start investigation if needed |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ1 -- Does AMCV deploy enter a live YR path? -> Yes, `UnitClass::Deploy @ 0x007393C0` constructs `DeploysInto` building and unlimbos it.` (evidence: `0x007393C0`)
- `[RESOLVED] OQ2 -- Does UnitClass::Deploy itself spawn CMIN/FreeUnit? -> No, no `Type+0xEA0` read and no free-unit `UnitClass::Constructor` branch exists there.` (evidence: `0x007393C0`)
- `[RESOLVED] OQ3 -- Does the IsDeployable/ConstructionYard branch create CMIN? -> No; the `ConstructionYard` checks in deploy are target-redirection/nonhuman ConYard setup, not unit creation.` (evidence: `0x007393C0`, `Type+0x16B9` reads)
- `[RESOLVED] OQ4 -- Where is `FreeUnit` read from INI? -> BuildingType parser reads section-local `FreeUnit` into `BuildingTypeClass+0xEA0`.` (evidence: `0x00460540`)
- `[RESOLVED] OQ5 -- Does stock `[GACNST]` set `FreeUnit`? -> No.` (evidence: `ini/rulesmd.ini:11622-11651`; base `ini/rules.ini:8492-8509`)
- `[RESOLVED] OQ6 -- Does stock Allied content set `FreeUnit=CMIN` anywhere? -> Yes, `[GAREFN]`, not `[GACNST]`.` (evidence: `ini/rulesmd.ini:11722-11736`)
- `[RESOLVED] OQ7 -- Is the free-unit branch tied to `OnConstructionComplete`? -> Yes, it is in `BuildingClass::OnConstructionComplete @ 0x00445F80`.` (evidence: `0x00445F80`)
- `[RESOLVED] OQ8 -- Is the free-unit branch tied to production placement only? -> No; the consumer is generic construction complete/startup and would run for any building type with `+0xEA0`, including refinery completion; AMCV GACNST still skips it because `+0xEA0==0`.` (evidence: `0x00445F80`, INI)
- `[RESOLVED] OQ9 -- Does building Unlimbo create the free unit? -> No; `Unlimbo` registers the building and house state but does not read `+0xEA0`.` (evidence: `0x00440580`)
- `[RESOLVED] OQ10 -- Does Mission_Construction create the free unit directly? -> No; it calls `OnConstructionComplete`; creation lives inside that hook.` (evidence: `0x00449A50`)
- `[RESOLVED] OQ11 -- Is this active in standard YR? -> AMCV deploy and OnConstructionComplete are active; the `FreeUnit` branch is active for stock refineries but not for stock GACNST.` (evidence: `0x007393C0`, `0x00445F80`, `ini/rulesmd.ini`)
- `[DEFERRED] OQ12 -- What creates the first Allied miner before any refinery is built?` (category: `out-of-scope`; reason: this report only proves AMCV-created GACNST does not create CMIN; start-unit seeding requires scenario/skirmish init tracing; next-step-if-pursued: trace skirmish start object creation and start-unit lists)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| AMCV deploy creates only GACNST for this concern; no CMIN is created by deploy or ConYard special branch | `0x007393C0`, `ini/rulesmd.ini:11622-11651` | none observed if `deploy_mcv` only replaces AMCV with GACNST | `src/sim/world/world_spawn.rs::deploy_mcv` | keep AMCV deployment as AMCV -> GACNST only; no harvester side-effect | deploy AMCV in standard Allied start; entity set gains GACNST and loses AMCV, with no new CMIN from that command | do not compensate for missing start miner by adding `CMIN` to MCV deploy |
| `FreeUnit=` is a BuildingType section key consumed on construction completion | `0x00460540`, `0x00445F80` | current Rust has `ObjectType.free_unit` and `RuleSet::refinery_free_unit`; actual placement trigger unchecked | `src/rules/object_type.rs`, `src/rules/ruleset.rs`, future building-completion/placement surface | trigger free-unit spawn only for buildings whose parsed `free_unit` is present, such as GAREFN, at completion/placement timing matching startup hook | complete/place GAREFN and observe one CMIN spawn path; complete/place GACNST and observe none | do not treat `FreeUnit` as `[General]`; do not infer by faction/ConstructionYard |
| Stock Allied CMIN source in this data path is `[GAREFN] FreeUnit=CMIN`, not `[GACNST]` | `ini/rulesmd.ini:11722-11736`; `0x00445F80` branch | unchecked beyond parser helpers | production/refinery placement systems under `src/sim/production/` | separate refinery free-unit implementation/tests from MCV deploy tests | placing/completing first Allied refinery creates one CMIN if placement succeeds; AMCV deployment alone creates none | do not attach refinery `FreeUnit` behavior to ConYard startup |

## 10. Negative Facts / Do Not Do

- Do not add `CMIN` creation to `Simulation::deploy_mcv`; `UnitClass::Deploy @ 0x007393C0` does not do that.
- Do not implement a `[General] FreeUnit=` / `FreeHarvester=` ConYard mechanism for stock `GACNST`; the verified parser is section-local `BuildingTypeClass+0xEA0`.
- Do not treat `ConstructionYard=yes` (`Type+0x16B9`) as a free-harvester flag; it gates ConYard/build-root behavior and special target handling, not miner creation.
- Do not move refinery `FreeUnit` behavior into production queue completion generically without checking the building's parsed `free_unit`; `GACNST` must remain a no-spawn case.
- Do not use `AllowedToStartInMultiplayer=no` wording from old HARV/AMCV docs as evidence that a ConYard deploy spawns the first miner; this report refutes that for AMCV-created GACNST.

## 11. Remaining Uncertainty

- Start-unit/skirmish seeding for the initial Allied miner remains outside this slice.
- Exact user-facing timing for refinery `FreeUnit=CMIN` relative to refinery buildup was not re-timed beyond the verified `OnConstructionComplete` hook.

## 12. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/AMCV.md:476` should replace:
  - Old: `The [General] FreeUnit= / FreeHarvester= mechanism on GACNST: when GACNST is deployed (from AMCV), it spawns a free CMIN nearby. This is the symmetric Allied-side equivalent of NACNST's FreeUnit=HARV.`
  - New: `Verified 2026-05-21: AMCV-created GACNST does not spawn CMIN. The binary's FreeUnit consumer is BuildingClass::OnConstructionComplete reading BuildingTypeClass+0xEA0; stock [GACNST] has no FreeUnit key, while stock [GAREFN] has FreeUnit=CMIN. Do not attach free-miner creation to AMCV deploy or ConstructionYard=yes.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/soviet/HARV.md:114` and `:325` contain analogous stale wording that says Soviet ConYard deploy spawns HARV; this report did not investigate SMCV/NACNST, but the stock INI pattern suggests those lines should be rechecked before implementation.

## Sources

- Ghidra decompiled in this investigation:
  - `UnitClass::Deploy @ 0x007393C0`
  - `BuildingClass::OnConstructionComplete @ 0x00445F80`
  - `BuildingTypeClass_ReadINI_Water @ 0x00460540`
  - `Mission_Construction @ 0x00449A50`
  - `BuildingClass::Unlimbo @ 0x00440580`
- Docs referenced:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_MISSION_GUARD_AND_CONSTRUCTION.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGTYPECLASS_CTOR_DEFAULTS.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/AMCV.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/units/structures/GACNST.md`
- INI files checked:
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
