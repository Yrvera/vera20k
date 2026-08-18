# MCV Deploy Target Redirection Exceptions - Ghidra Research Report

**Address(es):** `0x007393C0` (`UnitClass::Deploy`), `0x006FCDB0` (`TechnoClass::Set_ArchiveTarget`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Successful `AMCV -> GACNST` deploy target-continuity loop: which existing techno `ArchiveTarget` pointers from the old AMCV are rewritten to the new construction yard, and which binary predicates skip or clear them.
**Non-Scope:** Deploy placement legality, facing convergence, health/state transfer, construction-yard free-unit behavior, bullet target invalidation, NavCom/TarCom command targets outside `ArchiveTarget`.
**Confidence:** High for the deploy loop and exception predicates; Medium for stock-content trigger frequency because no stock `rulesmd.ini` `Doggie=` line was found.
**Active in YR:** Conditional. The loop is active in normal YR deploys; the only special clear-target exception requires a `ConstructionYard=yes` deploy target and an infantry targeter whose `Doggie=` flag is true.

## 1. Overview

After a successful MCV deploy, `UnitClass::Deploy` scans the global `TechnoClass` array and rewrites active combat targets (`TechnoClass+0x2B4`, `ArchiveTarget`) that still point at the old AMCV. The normal visible result is attack continuity: units that were firing at the AMCV immediately continue against the new `GACNST`, so selected attacker action lines also continue to the new building because action lines prefer `ArchiveTarget`.

The documented "ConYard / aircraft-on-helipad" exception is stale. The binary exception is instead: when the new building type has `ConstructionYard=yes` (`BuildingTypeClass+0x16B9`) and the targeter is `InfantryClass` (`WhatAmI()==0x0F`) and the targeter's `InfantryTypeClass+0xEC6` flag is true (`Doggie=` parser key), the targeter has `ArchiveTarget` cleared to null instead of redirected to the new ConYard.

## 2. Class Layout / Key Offsets

| Offset / global | Owner | Meaning | Evidence | Active in YR |
|---|---|---|---|---|
| `0x00A8EC7C` | global | `g_TechnoClass_Array` pointer | `0x00739739..0x0073973E` | Yes |
| `0x00A8EC88` | global | `g_TechnoClass_Count` | `0x0073972C..0x00739733`, loop compare `0x007397B8..0x007397C0` | Yes |
| `+0x2B4` | `TechnoClass` | `ArchiveTarget`, active combat target and action-line combat endpoint | Deploy reads/compares at `0x00739741`, `0x00739755`; target fields doc | Yes |
| vtable `+0x2C` | `AbstractClass` subclasses | `WhatAmI` / RTTI class id | deploy calls target and targeter slots at `0x0073974D`, `0x00739783` | Yes |
| vtable `+0x3C8` | `TechnoClass` | `Set_ArchiveTarget` | deploy calls at `0x007397A5`, `0x007397B2`; body `0x006FCDB0` | Yes |
| `+0x90` | object | alive flag used before retarget | `0x0073975D..0x00739765` | Yes |
| `BuildingClass+0x520` | building | pointer to `BuildingTypeClass` | `0x0073976F` | Yes |
| `BuildingTypeClass+0x16B9` | building type | `ConstructionYard=yes` / deployable ConYard flag | `0x00739775`; prior GACNST report/parser | Yes for `GACNST` |
| `InfantryClass` vtable `0x007EB058`, slot `+0x2C` -> `0x00523340` | infantry object | returns `0x0F` | constructor stores vtable `0x00517ACC`; vtable slot read; function `MOV EAX,0xf; RET` | Yes |
| `InfantryTypeClass+0xEC6` | infantry type | `Doggie=` parser flag | parser call at `0x005245EF` pushes string `0x00825934` (`Doggie`) and writes `[ESI+0xEC6]` at `0x005245F5`; deploy reads `0x00739795` | Conditional |

## 3. Core Logic

The redirect loop runs only after the new building is allocated, constructed, and successfully placed through its vtable `+0xD8`. It also runs after the old unit mission is set to `3`, but before the old AMCV is destroyed/removed.

Pseudocode for the verified slice:

```text
for each techno in g_TechnoClass_Array[0..g_TechnoClass_Count):
    target = techno.ArchiveTarget
    if target == null:
        continue
    if target.WhatAmI() != 1:
        continue
    if techno.ArchiveTarget != old_amcv:
        continue
    if !techno.IsAlive:
        continue
    if techno == old_amcv or techno == new_building:
        continue

    if new_building.Type.ConstructionYard
       and techno.WhatAmI() == 0x0F
       and techno.Type.Doggie:
        techno.Set_ArchiveTarget(null)
    else:
        techno.Set_ArchiveTarget(new_building)
```

Verified branch details:

- The target must be the active combat target, not merely a player-commanded or movement destination. Evidence: only `[ESI+0x2B4]` is read in the loop (`0x00739741`, `0x00739755`).
- The target object's `WhatAmI()` must return `1` before the pointer equality check against the old AMCV. This is consistent with `UnitClass::What_Am_I @ 0x00746E20` returning `1`. Evidence: `0x0073974B..0x00739755`; UnitClass report.
- The targeter must be alive and must not be the old AMCV or the new building. Evidence: alive byte `+0x90` at `0x0073975D..0x00739765`, self/new-building skips at `0x00739767..0x0073976D`.
- The clear-target exception is gated first by the new building type flag `+0x16B9`. If the new building is not a construction yard/deployable building, the code does not test targeter class or `Doggie=`. Evidence: `0x0073976F..0x0073977D`.
- The exception class id is `0x0F`, verified as `InfantryClass`, not `AircraftClass`: `InfantryClass` installs primary vtable `0x007EB058`; `0x007EB058+0x2C` points to `0x00523340`; `0x00523340` is `MOV EAX,0xf; RET`. AircraftClass vtable `0x007E22A4+0x2C` points to `0x0041C180`, which returns `2`.
- The exception flag is `Doggie=`, not helipad/dock state. The parser sequence pushes string address `0x00825934`, which reads `"Doggie"`, then writes the result to `InfantryTypeClass+0xEC6`. Deploy reads exactly `targeter.Type+0xEC6` before clearing.

## 4. INI Keys

| Key | Section / source | Default / stock value | Effect in this slice |
|---|---|---|---|
| `DeploysInto=GACNST` | `[AMCV]`, `rulesmd.ini:6977`; base `rules.ini:6098` | Stock YR AMCV uses `GACNST` | Selects the building type constructed by `UnitClass::Deploy`. |
| `ConstructionYard=yes` | `[GACNST]`, `rulesmd.ini:11625`; base `rules.ini:8495` | Yes | Maps to `BuildingTypeClass+0x16B9`; enables the special clear-target exception gate and ConYard setup branch. |
| `Doggie=` | `InfantryTypeClass::ReadINI @ 0x005240A0`, string `0x00825934` | Constructor default false at `0x0052375B`; no `Doggie=` line found in stock `rulesmd.ini`/`rules.ini` | If true on an infantry targeter, and the deploy target is a ConYard, clear target instead of redirecting. |

## 5. Integration Points

The loop is reached on the successful placement path only. Failure paths before building placement restore occupation/visibility state and return without redirecting any targeters.

`TechnoClass::Set_ArchiveTarget @ 0x006FCDB0` is the callee for both redirect and clear. It first clears `IsNewTarget` at `TechnoClass+0x50C`, returns early if the requested pointer already equals current `ArchiveTarget`, then stores the resolved pointer at `+0x2B4`. When the target becomes null, it clears current burst index `+0x3B8` and informs a spawn manager if present. This means the deploy-loop clear exception is not just a visual line clear; it resets active combat targeting state.

Target lines are indirectly affected because `TechnoClass::DrawActionLines @ 0x004DC060` uses `ArchiveTarget` before `NavCom`. Current Rust mirrors this concept at the app level: selected action lines read `entity.attack_target` in `src/app_target_lines.rs`, while combat stores entity/cell targets in `AttackTarget`.

## 6. Current Rust Implementation Status

Current Rust `deploy_mcv` in `src/sim/world/world_spawn.rs:495` despawns the MCV, spawns the construction yard, and restores selection/building-up state. It does not retarget any `GameEntity.attack_target` values from the old MCV stable id to the new ConYard stable id.

Rust combat has `AttackTarget { target: TargetKind }` in `src/sim/combat/mod.rs:199`; entity targets use stable ids (`AttackTarget::new` at line 271). The only broad target-removal helper found in this scan is `clear_targets_on_dead_entity`, which clears matching entity targets to `None` when a target dies. That is not equivalent to deploy redirection because gamemd redirects most targeters to the replacement building.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Deploy` successful placement entry to target loop | verified | `0x007393C0`, branch `0x00739717..0x0073972C` | none for this slice |
| Techno array iteration and bounds | verified | `0x0073972C..0x007397C0` | none |
| `ArchiveTarget` target predicate | verified | `0x00739741..0x0073975B`; target fields doc | none |
| alive/self/new-building skips | verified | `0x0073975D..0x0073976D` | none |
| normal redirect call | verified | `0x007397AD..0x007397B2` | none |
| ConYard/Doggie clear call | verified | `0x0073976F..0x007397A5`; `0x005245EF..0x005245F5` | stock-content `Doggie=` usage remains content-audit only |
| "aircraft-on-helipad" claimed exception | conflict-needs-resolution | no deploy-loop aircraft/helipad/cached-dock reads; AircraftClass `WhatAmI` returns `2`, not `0x0F` | update stale docs |
| `TechnoClass::Set_ArchiveTarget` callee side effects | verified | `0x006FCDB0` | none for target storage/reset; full target-resolution internals out-of-scope |
| NavCom/TarCom command fields | deferred | target fields docs | out-of-scope; deploy loop only touches `ArchiveTarget` |
| Bullet target redirection | deferred | bullet target invalidation reports | out-of-scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is the deploy-loop target field active combat target or command target? -> Active combat target, `TechnoClass+0x2B4` (`ArchiveTarget`).` (evidence: `0x00739741`, `0x00739755`; target fields report)
- `[RESOLVED] OQ2 - Does the loop run on failed deploy placement? -> No; it is below the successful `new_building.vtable+0xD8` placement branch.` (evidence: `0x00739711..0x00739719`)
- `[RESOLVED] OQ3 - Which targeters are considered? -> Every entry in `g_TechnoClass_Array`, bounded by `g_TechnoClass_Count`, with non-null `ArchiveTarget` equal to the old AMCV and alive/self/new-building filters passing.` (evidence: `0x0073972C..0x007397C0`)
- `[RESOLVED] OQ4 - Does the loop redirect target lines separately? -> No separate line state is touched; target-line continuity follows `ArchiveTarget`.` (evidence: `0x007397A5`/`0x007397B2`; `TARGET_LINES_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ5 - What is the documented exception? -> Binary exception is ConYard plus InfantryClass plus `Doggie=`, not aircraft-on-helipad.` (evidence: `0x0073976F..0x007397A5`, `0x00523340`, `0x005245EF..0x005245F5`)
- `[RESOLVED] OQ6 - Is `0x0F` AircraftClass? -> No for vtable `+0x2C`; `InfantryClass` returns `0x0F`, while `AircraftClass` returns `2`.` (evidence: Infantry vtable slot read to `0x00523340`; Aircraft vtable xref `0x007E22D0 -> 0x0041C180`)
- `[RESOLVED] OQ7 - What INI key maps to `InfantryType+0xEC6`? -> `Doggie=`, default false in constructor.` (evidence: string `0x00825934`, parser write `0x005245F5`; constructor `0x0052375B`)
- `[RESOLVED] OQ8 - Does stock `rulesmd.ini` set `Doggie=` on `[ADOG]`/`[DOG]`? -> No line found in stock base/YR INI scan.` (evidence: `rg 'Doggie=' ini/rulesmd.ini ini/rules.ini` returned no matches; `[ADOG]`/`[DOG]` sections inspected)
- `[DEFERRED] OQ9 - Do scripted maps/mods set `Doggie=` and trigger the exception in practice?` (category: out-of-scope; reason: this report is binary plus stock INI only; next-step-if-pursued: scan maps/mod packs or runtime loaded rules overlays)
- `[DEFERRED] OQ10 - How TarCom/NavCom are repaired when `ArchiveTarget` changes during deploy?` (category: out-of-scope; reason: deploy loop does not touch those fields; next-step-if-pursued: trace command-target lifecycle around `FootClass` fields `+0x5A4/+0x5CC`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| On successful AMCV deploy, every alive techno whose active combat target is the old AMCV is retargeted to the new `GACNST`, except the special clear case. | `0x0073972C..0x007397B2` | missing | `src/sim/world/world_spawn.rs::deploy_mcv`; `GameEntity.attack_target` | After spawning the ConYard and before finalizing deploy, rewrite matching `AttackTarget::Entity(old_mcv_id)` to `AttackTarget::Entity(new_conyard_id)`. | Enemy tank ordered to attack AMCV continues firing at the ConYard after deploy; selected attacker action line points to the new ConYard. | Do not clear all targeters as if the AMCV died; gamemd preserves most target continuity. |
| The exception clears, rather than redirects, only when the new building type is `ConstructionYard=yes`, the targeter is infantry (`WhatAmI()==0x0F`), and targeter type `Doggie=` is true. | `0x0073976F..0x007397A5`; `0x005245EF..0x005245F5` | missing/conditional content not parsed | rules object type parser; deploy retarget helper | If Rust later parses `Doggie=`, clear those infantry targeters in this specific ConYard case; with stock INI this may not trigger because no `Doggie=` line was found. | Modded dog infantry with `Doggie=yes` targeting an MCV loses target when it becomes a ConYard; non-dog infantry redirects. | Do not implement an aircraft/helipad exception from stale docs. |
| Target-line continuity is a consequence of active combat target continuity, not a separate deploy visual pass. | `TARGET_LINES_GHIDRA_REPORT.md`; deploy writes through `Set_ArchiveTarget` | missing through combat target only | `src/app_target_lines.rs` remains consumer; sim owns target state | Fix sim target state; app target lines should update naturally from `attack_target`. | With UnitActionLines enabled, selected attacker line changes endpoint from AMCV to ConYard on deploy without a separate app-layer patch. | Do not special-case target lines in UI while leaving combat targeting wrong. |

### Stale Docs / Follow-up Docs

- `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` line 286 currently says: "redirects any unit targeting this MCV to target the new building (or NULL if building has ConstructionYard=yes and target is aircraft on helipad)". Replace with: "redirects any alive techno whose `ArchiveTarget` is the deploying unit to the new building, except when the new building has `ConstructionYard=yes` and the targeter is `InfantryClass` with `InfantryTypeClass+0xEC6` (`Doggie=`) true; that exception calls `Set_ArchiveTarget(NULL)`."
- `READINI_FIELD_MAPS.md`, `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md`, and `DISGUISE_SYSTEM_GHIDRA_REPORT.md` appear to label `Doggie=` as `+0xEC7`; this slice verifies `Doggie=` writes `+0xEC6`. Replacement wording: "`Doggie=` is read at `InfantryTypeClass::ReadINI @ 0x005245EF`, string `0x00825934`, and written to `InfantryTypeClass+0xEC6`; `+0xEC7` is the next parsed bool (`Deployer=`)."

## Sources

- Ghidra: `UnitClass::Deploy @ 0x007393C0`; loop assembly `0x0073972C..0x007397C0`.
- Ghidra: `TechnoClass::Set_ArchiveTarget @ 0x006FCDB0`.
- Ghidra: `InfantryClass::Constructor @ 0x00517A50`; primary vtable install `0x00517ACC`; vtable slot `0x007EB084 -> 0x00523340`; `0x00523340` returns `0x0F`.
- Ghidra: `AircraftClass` vtable slot `0x007E22D0 -> 0x0041C180`; `0x0041C180` returns `2`.
- Ghidra: `InfantryTypeClass::ReadINI @ 0x005240A0`; `Doggie` string `0x00825934`; write `0x005245F5`; constructor default false at `0x0052375B`.
- INI: `ini/rulesmd.ini` `[AMCV] DeploysInto=GACNST`, `[GACNST] ConstructionYard=yes`, `[ADOG]`/`[DOG]`; no stock `Doggie=` line found in `rulesmd.ini` or `rules.ini`.
- Docs referenced: `TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`, `TARGET_LINES_GHIDRA_REPORT.md`, `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`, `UNITCLASS_GHIDRA_REPORT.md`, `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`.
