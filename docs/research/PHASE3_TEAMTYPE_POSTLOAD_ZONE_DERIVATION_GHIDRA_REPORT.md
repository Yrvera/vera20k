# Phase 3 TeamType Post-Load Zone Derivation — Ghidra Report

**Date:** 2026-08-26

**Program:** active retail Yuri's Revenge `gamemd.exe`

**Scope:** the load-time owner that derives `TeamTypeClass+0xEC/+0xF0/+0xF1`, its TaskForce inputs, the combine helper at `0x005889F0`, and the ordinary AI-trigger eligibility consumer at `0x0041FEE0`

**Mode:** exhaustive slice
**Result:** implementation-ready for the named slice; all material open questions below are resolved

## 1. Verdict

Retail does not leave the three TeamType zone fields at constructor defaults. After all fixed and map TeamTypes, Scripts, TaskForces, and AITriggers have loaded, one forward pass recomputes every TeamType from its final resolved TaskForce. Ordinary AI-trigger eligibility then consumes the result to decide whether the acting and designated enemy House bases have the required zone relationship.

The current Rust registry does not retain or derive these fields, accepts Building types in TaskForces that retail rejects, and clamps negative `Passengers` to zero even though retail tests signed nonzero. Those are required deltas for this slice. Trigger condition truth tables, team production/recruitment, weights, and unrelated presentation/TS mechanisms are outside this report.

## 2. Native ownership and load order

`ScenarioClass` load assembly at `0x0068797A..0x006879E8` establishes this order:

1. fixed then map TeamTypes;
2. fixed then map Scripts;
3. fixed then map TaskForces;
4. triggers and tags;
5. fixed then map AITriggers;
6. one call at `0x006879E8` to `0x006F2040`.

`0x006F2040` walks the global TeamType vector at `0x00A8ECA4`, using the count at `0x00A8ECB0`, in increasing index order. It calls `0x006F1FA0` exactly once for each entry. An empty vector performs no calls. This is the sole direct caller of `0x006F1FA0` found in the active program.

This placement matters: map overrides and the final fixed/map TaskForce resolution are inputs to the single derivation pass. Deriving fields while individual sections are registered would not reproduce the native owner or final-data semantics.

## 3. Relevant layouts and defaults

| Owner | Offset | Native meaning in this slice | Constructor/default evidence |
|---|---:|---|---|
| `TeamTypeClass` | `+0xE4` | resolved `TaskForceClass*` | consumed unconditionally by `0x006F1FA0` |
| `TeamTypeClass` | `+0xEC` | combined movement-zone row | initialized to `9` at `0x006F06E0`; overwritten to `9` at derivation start |
| `TeamTypeClass` | `+0xF0` | enforce House base-zone relation | initialized true at `0x006F06E0`; overwritten true at derivation start |
| `TeamTypeClass` | `+0xF1` | require transport-style crossing predicate | initialized false at `0x006F06E0`; overwritten false at derivation start |
| `TeamTypeClass` | `+0xF6` | `IsBaseDefense` | initialized false and parsed by TeamType `ReadINI` |
| `TaskForceClass` | `+0x9C` | compact resolved member count | reset to zero by `0x006E8420` |
| `TaskForceClass` | `+0xA4+i*8` | authored signed member count | copied before resolution; not read by zone derivation |
| `TaskForceClass` | `+0xA8+i*8` | resolved member type pointer | only non-null entries increase `+0x9C` |
| `TechnoTypeClass` | `+0x5B4` | `MovementZone` integer row | default `0`; parser can store `-1` for an unknown name |
| `TechnoTypeClass` | `+0x5E0` | signed `Passengers` | default `0`; native derivation compares to zero without clamping |
| `TechnoTypeClass` | `+0xCCE` | `Naval` byte | default false |

Movement row `9` is `Fly` in the retail 13-row passability matrix. The derivation resets the three outputs before examining the TaskForce, so re-running it is deterministic for unchanged rules and registries.

## 4. TaskForce member resolution is a required input contract

`TaskForceClass::ReadINI @ 0x006E8420` reads up to six numbered entries. Each positive-length value is parsed as `"%d,%s"` by `0x004C4EF0`, which returns the signed count and resolved type pointer. Resolution order is:

1. `0x00523C90` over the vector at `0x00A8E34C` — `InfantryTypeClass`;
2. `0x00747370` over the vector at `0x00A83CE4` — `UnitTypeClass`;
3. `0x0041CAA0` over the vector at `0x00A8B21C` — `AircraftTypeClass`.

There is no BuildingType lookup. At `0x006E8496..0x006E84C1`, the parser writes the count and pointer at the current compact index, then increments `TaskForce+0x9C` only when the pointer is non-null. A later valid entry overwrites an unresolved slot, so the final array contains only successfully resolved Infantry, Unit, or Aircraft entries in authored order. The order remains observable for custom rules that register the same case-insensitive ID in more than one native type family.

Consequences for parity:

- a Building ID must not become a valid TaskForce member merely because it exists in the broad rules object registry;
- the compact entry must retain the selected category-distinct type identity, not only its ambiguous case-insensitive name, because native stores the resolved pointer returned by the first successful family lookup;
- an unresolved ID contributes neither a member nor a movement row;
- the authored member count, including zero or negative custom values, does not suppress an otherwise resolved type during zone derivation;
- the native six-slot limit and compact-order behavior remain the TaskForce parser's responsibility.

## 5. Exact derivation at `0x006F1FA0`

The function performs the following ordered operations for one TeamType:

1. load `TeamType+0xE4` as the TaskForce pointer;
2. write `+0xF0 = 1`, `+0xF1 = 0`, and `+0xEC = 9`;
3. if signed `TaskForce+0x9C <= 0`, skip the member loop;
4. otherwise iterate compact resolved entries forward;
5. for every member, ignore `TaskForce+0xA4+i*8` and read only the type pointer at `+0xA8+i*8`;
6. if `TechnoType+0xCCE` (`Naval`) is nonzero:
   - when signed `TechnoType+0x5E0` (`Passengers`) is exactly zero, write `TeamType+0xF0 = 0`;
   - when it is any nonzero value, including a negative value, write `TeamType+0xF1 = 1`;
7. call `0x005889F0` with the member's `MovementZone` row first and the current combined row second, then store the return at `TeamType+0xEC`;
8. after all members, if `TeamType+0xF6` (`IsBaseDefense`) is nonzero, write `TeamType+0xF0 = 0`.

The final `IsBaseDefense` override does not clear `+0xF1` and does not change `+0xEC`. A pure naval member disables base-zone enforcement. A naval member with any signed nonzero passenger capacity instead enables the separate crossing predicate. Mixed TaskForces retain every flag set by an earlier member; no later land member clears it.

The function dereferences `TeamType+0xE4` before checking its member count. A null TaskForce is therefore invalid native state, not an alternate empty-TeamType behavior. Stock AIMD contains no null or empty TaskForce reference. Rust should continue diagnosing/refusing an unresolved required TaskForce rather than inventing derived values for it.

## 6. Exact movement-row combine helper at `0x005889F0`

The helper receives the first row in `ECX` and the second in `EDX`. It reads the literal 13-by-8 signed integer matrix at `0x0082A594`, which matches Rust's existing `MOVEMENT_ZONE_PASSABILITY` table.

For candidate rows `0..12`, in increasing order:

- reject the candidate only when either input's column value is exactly `2` and the candidate's value for that column is exactly `1`;
- otherwise score one point for a column only when both inputs and the candidate are all exactly `1` in that column;
- replace the current winner only when the candidate is valid and its score is strictly greater than the prior best score;
- initialize the best score to zero and the best row to `-1`.

Therefore score-zero candidates never win, equal scores retain the lower candidate row, and value `3` does not participate in the `2`-versus-`1` rejection rule. The helper is symmetric for valid input rows despite the register order.

The valid-row incompatible unordered pairs that return `-1` are:

`0+10`, `0+11`, `1+10`, `1+11`, `2+10`, `2+11`, `6+10`, `6+11`, `7+10`, `7+11`, `8+10`, `8+11`, `10+12`, and `11+12`.

There is no input bounds check. In this active executable a previous `-1` result becomes absorbing in a later fold: indexing one row before the table reads the eight dwords at `0x0082A574..0x0082A590`, none of whose values is `1` or `2`, so every candidate remains score zero and the helper returns `-1` again. An authored unknown `MovementZone` that parsed as `-1` has the same observed result in this binary.

This pre-table behavior is executable-layout-specific evidence, not a general safe API contract. Rust can reproduce the active result explicitly without performing an out-of-bounds read.

## 7. Ordinary eligibility consumer at `0x0041FEE0`

The function receives a TeamType, the acting House, and the designated target House. Its behavior is:

1. if `TeamType+0xF0 == 0`, return true immediately;
2. if the target House pointer is null, return true immediately;
3. obtain both Houses' reference cells through `HouseClass::Get_Base_Or_Starting_Cell @ 0x0050DEF0`, which chooses `House+0x5494` unless it equals the invalid-cell sentinel, otherwise `House+0x5490`;
4. read combined row `TeamType+0xEC` and call `MapClass::GetZoneID` for both cells with bridge flag zero;
5. if `TeamType+0xF1 == 0`, return whether the two zone IDs are equal;
6. if `TeamType+0xF1 != 0`:
   - return false when the two combined-row zone IDs are equal;
   - only when they differ, repeat both lookups with hard-coded row `5` (`Amphibious`) and return whether those two zone IDs are equal.

The `+0xF1` path is thus an exact crossing-required predicate: the bases must be separated under the TeamType's combined row but connected under Amphibious row 5. It is not a permissive fallback to Amphibious when the primary comparison already succeeds.

`0x0041E720` calls this consumer for the primary TeamType at `0x0041E9F5`; false rejects immediately. If a secondary TeamType exists, it calls again at `0x0041EA14` with the same acting and target Houses; false again rejects. This ordering is after trigger-condition gating and before buildability and `Max` gates.

The target House is selected in `0x006F0AB0` from acting `House+0x5600` (`EnemyHouseIndex`) through the global House array. Constructor default is `-1`; that produces a null target and the consumer returns true. Nonzero multiplayer strategy designates an enemy, making the mechanism active in ordinary retail skirmish after enemy selection.

`MapClass::GetZoneID` has no guard for row `-1`. The unrelated `CanReachZone` helper's `-1` early-success rule does not apply here. With malformed custom data or an incompatible fold, retail indexes the wrong MapClass area and the comparison becomes map-memory-dependent. Stock data excludes this state. Rust should retain the derived `Invalid` row faithfully but must not claim a deterministic retail eligibility verdict for malformed `-1` consumers without a separate consumer-safety decision.

## 8. Retail data census and oracle

Sources used for the census were the canonical retail-data copies:

- `aimd.ini` SHA-256 `5df41eaec00a78d0760ef5eecdf27d65ae1cd537309c7eac973318266986f89d`;
- `rulesmd.ini` SHA-256 `3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`.

Observed stock facts:

- 163 TeamTypes, 132 TaskForces, and 213 authored TaskForce entries;
- 66 unique member type IDs, all with rules sections and explicit valid `MovementZone` values;
- no zero or negative member counts; maximum count 20;
- no TeamType references an empty TaskForce;
- no type ID is shared across the stock Infantry, Vehicle, Aircraft, or Building registries;
- eight naval member types: `DEST`, `AEGIS`, `CARRIER`, `SUB`, `DRED`, `HYD`, `SQD`, `BSUB`;
- none of those eight has nonzero `Passengers`;
- seven TeamTypes contain a naval member and twelve are `IsBaseDefense`; the sets do not overlap;
- 19 TeamTypes finish with `+0xF0 = 0`; no stock TeamType finishes with `+0xF1 = 1`;
- custom data can activate `+0xF1`: stock rules define `LCRF`, `SAPC`, and `YHVR` with `Naval=yes`, `Passengers=12`, `MovementZone=Amphibious`, although stock AIMD uses none of them in a TaskForce.

Final combined-row distribution across stock TeamTypes:

| Row | Count |
|---:|---:|
| 0 | 40 |
| 1 | 3 |
| 2 | 26 |
| 3 | 4 |
| 7 | 63 |
| 8 | 2 |
| 9 | 18 |
| 10 | 7 |

The deterministic source-order oracle format is UTF-8:

`TeamTypeID<TAB>combined_row<TAB>enforce_0_or_1<TAB>crossing_0_or_1<LF>`

Its SHA-256 is `1274426ac3e9ce7adc7d4babab8bd9a04b61519cca02a720b0d0a38cd22da2e1`.

The 19 enforcement-disabled IDs, in source order, are:

`06175EFC-G`, `0CE4CEFC-G`, `0CE4CA3C-G`, `09B5330C-G`, `0CA1E67C-G`, `0CAC330C-G`, `08DA30EC-G`, `0ACCD81C-G`, `0CA18BAC-G`, `0EC2038C-G`, `05C643BC-G`, `06099EFC-G`, `06C5992C-G`, `0609946C-G`, `06109EFC-G`, `06109A4C-G`, `05FC5C6C-G`, `08B909EC-G`, `08BE3EFC-G`.

## 9. Adversarial boundary checks

1. **Empty resolved TaskForce:** defaults remain row 9 / enforce true / crossing false, then `IsBaseDefense` may disable enforcement. Stock has no such TeamType.
2. **Zero or negative authored member count:** the count does not affect derivation; a successfully resolved type is still folded and can set flags. Stock has no such count, but this is active custom-data behavior.
3. **Pure naval versus naval transport:** pure naval sets enforcement false; naval with any signed nonzero `Passengers` sets crossing true instead. A negative value is nonzero natively.
4. **Crossing flag with equal combined zones:** eligibility is false, not true; Amphibious is checked only after the combined zones differ.
5. **Incompatible or invalid movement fold:** derivation retains `-1`; later folds remain `-1` in this exact binary, but `GetZoneID` consumption is unsafe/map-memory-dependent. Stock excludes it.

Cold rechecks against raw disassembly confirmed the two easiest-to-misread points: `0x006F1FA0` passes member row then current row to `0x005889F0`, and the `0x0041FEE0` crossing branch returns false when the first two zone IDs are equal.

## 10. Current Rust status and required deltas

Current Rust evidence inspected for this report:

- `TeamTypeDefinition` retains ID, script, TaskForce, priority, and `IsBaseDefense`, but not the three derived zone fields.
- `TeamAiIniRegistry::from_sources` completes registry loading without a native-order post-load TeamType derivation pass.
- TaskForce resolution uses the broad rules object registry, allowing Building types that retail TaskForce parsing never resolves.
- `ObjectType.passengers` is unsigned and the parser clamps negative INI values to zero, losing the native signed-nonzero distinction.
- `MOVEMENT_ZONE_PASSABILITY` already contains the exact retail 13-by-8 matrix, but no exact two-row selection/fold helper is exposed for TeamType derivation.
- `TeamScriptVm` snapshots serialize definitions; adding derived fields therefore requires a snapshot version migration and round-trip coverage. Definitions are intentionally excluded from the live-team lockstep hash, so this slice should not silently change that separate ownership rule.

Required implementation sequence:

1. preserve signed `Passengers` parsing;
2. resolve TaskForce members in native Infantry, Unit/vehicle, Aircraft order, retain the selected category-distinct identity in the compact entry and snapshot, and keep unresolved members diagnostic;
3. implement the exact matrix selection rule, including strict tie handling and explicit active-binary `Invalid` absorption;
4. retain combined row, enforcement, and crossing-required fields on every TeamType definition;
5. after all final registries and triggers are resolved, derive every TeamType in source order from its compact resolved TaskForce;
6. serialize the fields with a snapshot version bump;
7. validate constructor/empty behavior, count independence, pure naval, signed passenger transport, base-defense override, incompatible folds, stock distribution, and the oracle digest.

This report does not authorize approximating `0x0041FEE0` with a generic reachability helper. The exact consumer should be implemented only in the owning eligibility slice or as a proven minimal prerequisite when that slice is selected.

## 11. Coverage and exclusions

Verified in this pass:

- owner and loader placement;
- constructor and recompute defaults;
- null/empty behavior;
- TaskForce compact member resolution, category set, lookup order, and count irrelevance;
- all four input fields and the base-defense override;
- the entire combine table algorithm, tie rule, incompatible pairs, and active-binary `-1` behavior;
- both direct eligibility call sites, their order, House cell selection, target-House origin, and exact primary/crossing predicates;
- stock liveness, edge exclusions, distribution, and full-registry oracle digest;
- Rust ownership gaps, snapshot duty, and hash non-ownership.

Evidence-backed exclusions from ordinary stock behavior:

- null or empty TaskForce references;
- unresolved TaskForce member IDs;
- zero or negative TaskForce member counts;
- invalid/incompatible final combined rows;
- naval passenger TaskForces setting `+0xF1`.

These exclusions do not make the parser/derivation rules dead: custom map/rules data can exercise all except null native state, and stock contains suitable naval passenger types.

Not investigated or claimed by this report:

- AI-trigger condition truth tables and weight selection;
- team construction, recruitment, production, or placement;
- the complete downstream implementation of `0x0041FEE0`;
- presentation systems including Railgun, LaserDraw, or Sonic Wave;
- destroyable-cliff behavior or Tiberian Sun legacy paths.

## 12. Open Questions Log — final

| ID | Question | Resolution |
|---|---|---|
| OQ01 | Who owns derivation and when? | `0x006F2040` after all fixed/map AI registry passes; forward TeamType order. |
| OQ02 | What are constructor/recompute defaults? | row 9, enforce true, crossing false. |
| OQ03 | What happens for null/empty TaskForces? | null is invalid/dereferenced; empty count preserves defaults before base-defense override. |
| OQ04 | Do authored member counts participate? | no; only compact resolved type pointers participate. |
| OQ05 | What do `+0xCCE/+0x5E0/+0x5B4` mean? | `Naval`, signed `Passengers`, and `MovementZone`. |
| OQ06 | Which member categories resolve? | Infantry, Unit, Aircraft, in that order; not Building. |
| OQ07 | What flags do naval members set? | zero passengers disables enforcement; signed nonzero sets crossing-required. |
| OQ08 | What exactly does `0x005889F0` do? | exhaustive 13-candidate matrix selection described in section 6. |
| OQ09 | How do invalid and `-1` behave? | no bounds guard; `-1` is absorbing in this binary; consumer becomes map-memory-dependent. |
| OQ10 | When is `IsBaseDefense` applied? | after all members; only forces enforcement false. |
| OQ11 | What cells are compared? | acting and target House override/base-start cells from `0x0050DEF0`. |
| OQ12 | What is the normal predicate? | equal `GetZoneID` results under combined row. |
| OQ13 | What is the crossing predicate? | combined rows must differ, then Amphibious row-5 IDs must match. |
| OQ14 | How are primary/secondary TeamTypes ordered? | primary first and rejecting; optional secondary second and rejecting. |
| OQ15 | Is there a complete stock oracle? | yes; 163-row digest recorded in section 8. |
| OQ16 | Is the mechanism live in ordinary YR? | yes after enemy designation; stock actively disables enforcement for 19 TeamTypes. |
| OQ17 | What is missing or wrong in Rust? | three fields/pass absent, TaskForce category too broad, signed passengers lost, helper absent. |
| OQ18 | What state duties follow? | snapshot serialization/version update; no new live-team registry hash ownership. |
| OQ19 | Are tick/paused/scheduler edges relevant? | no; derivation is static load-time state. Save/load serialization is relevant. |
| OQ20 | Is this TS-gated or legacy-only? | no; it is active retail YR logic in the ordinary AI-trigger path. |

No material open question remains for implementing the derivation slice. The malformed `-1` consumer result is deliberately classified rather than approximated.

## 13. Implementation handoff

Minimum coherent slice:

- exact signed data and TaskForce category prerequisites;
- exact combine helper;
- final-registry TeamType derivation and retained fields;
- snapshot migration;
- focused unit/custom-data tests plus the 163-row stock oracle.

Acceptance requires all of the following:

- a Building ID cannot resolve as a TaskForce member;
- member count values do not change zone inputs;
- matrix selection matches strict native scoring/ties and all incompatible pairs;
- negative `Passengers` is treated as nonzero;
- `IsBaseDefense` only overrides enforcement after the fold;
- the retail registry produces the distribution and digest in section 8;
- snapshot round-trip retains all three fields;
- prior fixed/map registry, placeholder, and TechLevel-threshold tests remain green.

The downstream House-zone eligibility consumer remains a recorded next-owner mechanism after this retained-data slice. It must use the exact two-stage predicate above and must not be declared closed merely because the derived fields exist.

## 14. Annotation candidates

None required. The active Ghidra program already exposes stable function boundaries for `0x006F2040`, `0x006F1FA0`, `0x005889F0`, `0x0041FEE0`, and `0x006E8420`. No metadata was modified in this pass.

## 15. Sources

- live connected Ghidra project `testProsjekt`, program `gamemd.exe`;
- Ghidra C export `gamemd.c`, read-only decompilation of the functions and call neighborhoods named above;
- raw retail disassembly from Visual Studio `dumpbin /DISASM`, especially `0x0068797A..0x006879E8`, `0x006E8420..0x006E84FA`, `0x006F1FA0`, `0x005889F0`, and `0x0041FEE0`;
- active retail bytes at `0x0082A574..0x0082A6F3` for the pre-table words and 13-by-8 passability matrix;
- canonical retail `aimd.ini` and `rulesmd.ini` identified by the hashes in section 8;
- Rust source in `src/sim/team_script_vm.rs`, `src/sim/team_script_vm/registry_install.rs`, `src/sim/pathfinding/passability.rs`, and `src/rules/object_type.rs`.
