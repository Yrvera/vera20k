# BuildConst RulesClass::ReadAI Binding Repair Design

**Date:** 2026-08-26
**Phase:** 3 — GSI-04.05 ownership prerequisite
**Status:** Approved bounded repair before BasePlan generation
**Native authority:** active-retail `gamemd.exe`, `RulesClass__ReadAI @ 0x00672AE0`; retail `rules.ini` and standalone YR `rulesmd.ini`

## Verdict

The existing BuildConst lifecycle and naval first-yard cap are structurally correct but are not connected to retail data. Rust reads `BuildConst=` from `[General]`; native reads it from `[AI]`, and both retail rules files place it there. Retail therefore leaves Rust's BuildConst type membership, House acquisition vector, and naval first-yard cap empty.

This repair restores the active-retail parser binding and the native zero-default `Owner` gate, then revalidates the complete parsed-rules-to-naval path. It does not implement BasePlan generation. That mechanism remains blocked on this prerequisite.

## Corrected native contract

### Rules ownership and parse order

`RulesClass__Process` calls:

1. `RulesClass__ReadBuildingTypes @ 0x00672660` from `0x00668E78`;
2. `RulesClass__ReadAI @ 0x00672AE0` from `0x00668EC8`;
3. `RulesClass__ReadTypeData @ 0x00679A10` from `0x00668EF0`.

`ReadAI` resolves the literal section `AI` at `0x00839DA4`. Its BuildConst key push is `0x00672B23`, resolver call is `0x00672B6A`, and binding block is `0x00672B14..0x00672C01`. There is no native `[General]` fallback. `Shipyard=` and `AINavalYardAdjacency=` remain `[General]` inputs.

A present BuildConst value is tokenized with comma as the only delimiter, preserving resolved pointer order and duplicates. Each nonempty token uses `BuildingTypeClass__FindOrAllocate @ 0x004653C0`; identity matching is ASCII case-insensitive. `none` and `<none>` resolve null and are omitted. Missing input yields an empty vector in a fresh RulesClass because the constructor zero-constructs it. The active retail tokens are all already registered BuildingTypes, so this repair must resolve them in the existing BuildingType registry rather than broad cross-category lookup.

Native can allocate unknown custom tokens. Exact registry-tail order then depends on every later BuildingType list in `ReadAI`, not just the seven BasePlan lists. Both active retail files have zero unresolved BuildConst/planning-list tokens, so custom unknown allocation and native OOM corruption are evidence-backed inactive exclusions from this repair. They must not be approximated by silently inventing a partial allocation order.

### Retail data

- RA2 `rules.ini:[AI]` contains `BuildConst=GACNST,NACNST`.
- YR `rulesmd.ini:[AI]` contains `BuildConst=GACNST,NACNST,YACNST`.
- YR loads standalone `rulesmd.ini`; it is not overlaid on `rules.ini`.
- Every token occurs in the corresponding retail BuildingTypes registry.

### Owner gate correction

`TechnoTypeClass` construction at `0x00711193` stores zero at `+0x6CC`; reader block `0x007149E1..0x007149F5` uses that current value as the missing-key default. Therefore an empty Rust `ObjectType.owner` represents a zero Owner mask and must fail `HouseClass__FirstBuildableFromArray @ 0x005051E0`. It must not mean all countries.

`RequiredHouses` and `ForbiddenHouses` keep their separately verified default handling. The repair changes only the Owner decision used by the native AI-list selector; it does not globally redefine generic production eligibility.

## Implementation ownership

1. `RuleSet::from_ini`
   - Read `BuildConst` from `[AI]` only.
   - Preserve token spelling, order, duplicates, comma-only tokenization, sentinel omission, and case-insensitive BuildingType resolution for the active-retail registered path.
   - Stamp `ObjectType.build_const_eligible` on every resolved BuildConst BuildingType.
   - Do not read or merge `[General] BuildConst`.

2. Native AI-list selector
   - Reject a candidate with an empty Owner vector.
   - Retain exact country-mask, Required/Forbidden, `AIBasePlanningSide`, and shell-superweapon tail behavior.

3. Documentation/provenance
   - Correct all comments and bounded design claims introduced by the naval/BasePlan work that call BuildConst a `[General]` key.
   - Record `RulesClass__ReadAI @ 0x00672AE0` and the exact BuildConst block near the parser.
   - Do not rewrite unrelated historical research in this repair; the stale parent-report claims are superseded by this design's verified correction bundle and must be fixed before the phase-wide reverse audit.

No state schema changes are required. Parsed Rules data is immutable; existing snapshots already persist the stamped entity bit and House BuildConst order in schema v106.

## Acceptance tests

1. A poisoned fixture with `[General] BuildConst=WRONG` and `[AI] BuildConst=gacnst,NACNST,YACNST` proves only the `[AI]` list is retained and stamped.
2. Case variants resolve to the registered BuildingType; duplicate list entries remain ordered; `none` and `<none>` are omitted.
3. Missing `[AI] BuildConst` yields an empty vector and no membership.
4. A candidate with no `Owner=` is rejected by the FirstBuildableFromArray predicate; an explicit matching Owner is accepted.
5. Parsed rules followed by successful ConstructionYard spawn/reveal populates `HouseState.build_const_order` in acquisition order.
6. The same parsed integration reaches the naval first-yard distance cap; it must not pass through the empty-vector bypass.
7. Existing BuildConst lifecycle, owner-transfer, snapshot/hash, and naval selector/cap tests remain green.

## Evidence-backed exclusions

- Custom unknown BuildConst allocation, full native `ReadAI` registry-tail ordering, and OOM-corrupted vector states are inactive in both retail files and remain open for generic mod parity.
- The other six BasePlan planning lists, `AIBuildThis`, generated BasePlan topology/RNG, distinct BasePlan center, Recenter/action 30, and post-deploy AI side effects remain separate open mechanisms.
- This repair does not claim GSI-04.05 or any Phase 3 row closed. It only restores a prerequisite that the naval cap and future generator require.
