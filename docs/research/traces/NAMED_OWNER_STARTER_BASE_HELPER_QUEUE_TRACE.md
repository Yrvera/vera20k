# Named Owner Starter-Base Helper Queue Trace

Date: 2026-05-23

Scope: Rust starter-base helper for local human owner `Commander` with selected Soviet/Russia or Yuri country. This trace is limited to the helper queue identity decision after skirmish launch and MCV deploy. Adjacent sidebar theme and sell-survivor issues are out of scope.

## Summary

Verdict: FAIL for current Rust helper behavior.

Retail `gamemd.exe` has no direct "starter-base helper" that queues an opening after deploy. The retail comparison is therefore not a helper-to-helper equality check. The active standard-YR evidence does establish the side/country setup contract: the selected country is packed by the Skirmish Start flow, consumed by `ScenarioClass::Create_Houses`, and faction-appropriate MCV/building data then controls what a player can build. Rust launch mirrors this by storing `HouseState.country` and `HouseState.side_index` for owner `Commander`.

Current Rust helper does not use that selected country/side when choosing its opening IDs. It passes raw owner name `Commander` into `pick_building_for_owner`, which cannot match INI `Owner=` lists such as `Russians` or `YuriCountry`; the fallback then picks the first candidate in each list, i.e. Allied IDs. After the recent production gate fix, `build_options_for_owner` correctly exposes Soviet/Yuri enabled options, so the helper's Allied picks are filtered out and nothing is queued for Soviet/Yuri named owners.

Player-visible result: pressing the Rust starter-base helper as a named Soviet or Yuri player after MCV deploy produces no starter queue. The expected selected-side identities are Soviet `NAPOWR` first after `SMCV -> NACNST`, and Yuri `YAPOWR` first after `PCV -> YACNST`.

## Evidence Sources

- Rust:
  - `src/app_commands.rs:418` `place_starter_base_for_local_owner`
  - `src/app_commands.rs:671` `pick_building_for_owner`
  - `src/app_skirmish.rs:298` `populate_launch_houses`
  - `src/skirmish_launch.rs:56` `LaunchCountry::country_name`
  - `src/skirmish_launch.rs:71` `LaunchCountry::side_index`
  - `src/skirmish_launch.rs:79` `LaunchCountry::opening_mcv_candidates`
  - `src/sim/production/production_tech.rs:155` `owner_matches_build_identity`
  - `src/sim/production/production_queue.rs:296` visible build option filter
- INI:
  - `ini/rulesmd.ini:12418` `[NACNST]`, `Owner=Russians,Confederation,Africans,Arabs`
  - `ini/rulesmd.ini:13091` `[YACNST]`, `Owner=YuriCountry`
  - `ini/rulesmd.ini:12450` `[NAPOWR]`, `Prerequisite=NACNST`
  - `ini/rulesmd.ini:13125` `[YAPOWR]`, `Prerequisite=YACNST`
  - `ini/rulesmd.ini:7838` `[SMCV]`, `Owner=Russians,Confederation,Africans,Arabs`
  - `ini/rulesmd.ini:8826` `[PCV]`, `Owner=YuriCountry`
- Existing verified docs:
  - `docs/research/skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`: active standard-YR Start Game/session/Create_Houses path and node country consumer.
  - `docs/research/skirmish-ui/SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md`: active standard-YR Start packing and Create_Houses field copies.
  - `docs/research/units/soviet/SMCV.md`: SMCV deploys into NACNST and the mechanism is the standard MCV deploy path.

## Pipeline

`Skirmish country selection` -> `Rust launch session` -> `HouseState for Commander` -> `MCV type/deploy target` -> `starter-base helper opening pick` -> `production build-option filter` -> `queued commands` -> `player sees queue`

## Stage Trace

| Stage | Rust computed output | gamemd / retail-side evidence | Verdict |
|---|---|---|---|
| 1. Active retail side/country setup | N/A for helper. Standard Skirmish Start has selected country data. | Active in YR: Start branch writes country/session data; `Create_Houses` consumes node `+0x4B` / AI country arrays. See `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md:8,23,33-39`. | NOT-IMPLEMENTED for direct helper; verified setup evidence exists |
| 2. Rust named owner launch setup | For local player `Commander`, `populate_launch_houses` stores country `Russians`/`YuriCountry`, side index `1`/`2`, and `is_human=true`. | Retail uses selected country, not the typed player display name, to create the house/faction identity. | PASS for Rust launch-side identity storage |
| 3. Rust selected MCV identity | `LaunchCountry::Russia` chooses `SMCV` first; `LaunchCountry::Yuri` chooses `PCV` first. | INI owner/deploy data: `SMCV` is Soviet-side and deploys to `NACNST`; `PCV` is Yuri-side and deploys to `YACNST`. Retail helper comparison remains absent. | PASS for Rust launch MCV side selection |
| 4. Post-deploy expected first power identity | With deployed Soviet `NACNST`, enabled power prerequisite is `NAPOWR` (`Prerequisite=NACNST`). With deployed Yuri `YACNST`, enabled power prerequisite is `YAPOWR` (`Prerequisite=YACNST`). | Retail build identity follows the side/country Construction Yard and INI prerequisites. No direct helper exists. | UNCHECKED for literal gamemd queue equality; expected side identities are computed from active setup plus INI |
| 5. Helper opening pick | For owner `Commander`, `pick_building_for_owner(["GAPOWR","NAPOWR","YAPOWR"])` first pass matches none because no INI `Owner=` entry equals `Commander`; fallback returns `GAPOWR`. Barracks returns `GAPILE`; refinery returns `GAREFN`. | Retail has no corresponding helper. This is internally inconsistent with Rust's own selected country state. | FAIL |
| 6. Production filter | `build_options_for_owner` now matches `Commander` against `HouseState.country`; after `NACNST`, enabled option includes `NAPOWR`, not `GAPOWR`; after `YACNST`, enabled option includes `YAPOWR`, not `GAPOWR`. Helper filters for the wrongly picked Allied IDs, so `queueable=[]`. | Retail-side expectation after side-correct ConYard is side-correct build options. Direct helper queue equality is not applicable. | FAIL |
| 7. Command queue/player result | No `Command::QueueProduction` is scheduled for Soviet/Yuri named owner in this scenario. | Retail has no debug helper; selected-side production should not be empty merely because the player display name is `Commander`. | FAIL |

## Entry Points Checked

- Trigger entry: `src/app_input.rs:291` calls `place_starter_base_for_local_owner` from the debug input path.
- Helper entry: `src/app_commands.rs:418` builds three opening candidate picks and schedules `Command::QueueProduction`.
- Candidate picker: `src/app_commands.rs:671` compares only `ObjectType.owner` against the raw owner string, then falls back to the first building candidate.
- Production eligibility: `src/sim/production/production_tech.rs:155` correctly matches raw owner or `HouseState.country`.
- Launch setup: `src/app_skirmish.rs:298` stores `HouseState.country` and `side_index` for named launch slots.

Coverage: only this Rust helper path was traced. Normal sidebar build options after the earlier production fix are not the failing path here.

## Failure

Stage 5/6/7: named Soviet/Yuri starter-base helper queues no opening.

Root cause: `pick_building_for_owner` has no access to `Simulation` or `HouseState`, so it cannot map `Commander` to `Russians` or `YuriCountry`. It therefore picks Allied fallback IDs before the production filter runs.

Player-visible difference: after starting as a named Soviet/Yuri player and deploying MCV, invoking the helper does not queue the selected-side starter structure. The helper should choose from the selected side/country identity before filtering: `NAPOWR` for Soviet/Russia, `YAPOWR` for Yuri.

Suggested fix direction: resolve owner build identities through the same country-aware helper used by production, or pass the already computed enabled build options into the opening selection and choose the first enabled candidate in side-preferred order.

## Adjacent Findings

- The helper candidate arrays are Allied-first. This is fine only if the selection logic is side-aware before fallback.
- The direct retail helper comparison is not available. This should be treated as a Rust debug/helper consistency bug, not proof of a missing gamemd helper.

## Verdict Tally

PASS: 2 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

