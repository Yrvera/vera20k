# Skirmish Post-Shell Start Unit Budget - Ghidra Research Report

**Address(es):** `0x00686890`, `0x005D6D80`, `0x005D7030`, `0x005D70F0`, `0x00688ED0`, `0x00505310`, `0x006886B0`
**Investigation Mode:** exhaustive-slice for the standard Battle/ManBattle/FreeForAll style post-shell Skirmish start-unit path; coverage-map for non-Battle mode-specific overrides.
**Claimed Scope:** offline Skirmish after shell handoff reaches `ScenarioClass::Post_Map_Init`, selected mode start callbacks, `UnitCount` budget computation, side/country/type filters, MCV/base-unit creation, and per-house iteration.
**Non-Scope:** shell UI packing/validation, full start-point assignment, exact map loader token path, complete Siege/Unholy/Cooperative custom callback behavior, and later AI economy/production.
**Confidence:** High for `Post_Map_Init`, `FUN_005D6D80`, Battle/ManBattle/FreeForAll vtable binding, budget formula, candidate gates, standard MCV callback, and current Rust delta. Medium for non-Battle mode-specific `+0xC8` bodies where Ghidra lacks function boundaries.
**Active in YR:** Yes. Offline Skirmish uses selected `MPModesMD.ini` mode objects (`DAT_00A8B23C`) in `Post_Map_Init`; standard Battle/ManBattle/FreeForAll bind the verified standard callbacks.

## 0. Working Notes Seed

Target question: After Start handoff, how does standard offline Skirmish generate opening MCVs and starting units from `UnitCount`, house side/country masks, and selected mode callbacks?

Non-goals: Do not re-investigate shell packing, UI validation, preview, map chooser, or unrelated gameplay after initial start-unit generation.

Evidence needed to mark COMPLETE: decompile plus assembly/xref evidence for `0x00686890 -> 0x005D6D80`, vtable evidence for standard selected-mode callbacks, binary evidence for budget/filter/house iteration, INI/default evidence for `UnitCount`/`Bases`/`BaseUnit`, and current Rust source scan.

Stop conditions: stop at first custom non-Battle callback whose full body needs a separate mode investigation; do not mutate Ghidra or write Rust; write only this report plus the shared claims line.

## 1. Overview

Standard offline Skirmish does not use the null-mode `ScenarioClass::Generate_Random_Units` branch. `Post_Map_Init @ 0x00686890` checks selected mode object `DAT_00A8B23C`: if present, it calls mode vtable `+0x84`, reloads the selected object into `ECX`, then calls `FUN_005D6D80 @ 0x005D6D80`.

`FUN_005D6D80` is the selected-mode start-unit orchestrator. It computes a money-like budget from `UnitCount * average eligible type cost`, then loops every non-special house, skips observer humans, calls the mode `+0xC8` MCV/base callback, calls the mode `+0xCC` extra-unit callback, optionally adds leftover credits, and resets four house production/factory vectors.

Current Rust has a launch-session path that creates all configured launch houses and spawns one MCV per assigned slot, but it still lacks the native budgeted extra-unit generator, spawnability/tech/house-mask filters, fallback placement, selected-mode callback model, and custom mode overrides.

## 2. Key Offsets And Globals

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B23C` | selected MPModes object | `0x0068691A` null check; `0x0068692B` calls vtable `+0x84`; `0x00686931` reloads it before `0x005D6D80` | Yes |
| `DAT_00A8B270` | `UnitCount` / start-unit count | `0x005D6D83` load and `0x005D6D8E` test; `rulesmd.ini [MultiplayerDialogSettings] UnitCount=10` | Yes |
| `DAT_00A8B258` | `Bases` option | `0x005D7030` standard MCV callback returns immediately when false; `rulesmd.ini` default `Bases=yes` | Yes |
| `RulesClass+0xB20` vector | `BaseUnit=` list | `0x005D7051..0x005D7059`, `FUN_00505310`; `rulesmd.ini [General] BaseUnit=AMCV,SMCV,PCV` | Yes |
| `Type+0x6D5` | `AllowedToStartInMultiplayer` / spawnable gate | `0x005D6E10`, `0x005D6E7B`, `0x005D7178`, `0x005D722F..` assembly/decompile | Yes |
| `Type+0x6CC` | house/side mask | `FUN_00505310`; `FUN_005D6D80` mask tests; `0x005D7186` and second list branch | Yes |
| `Type+0x634` / decompile `piVar[0x18D]` | TechLevel | `0x005D7192..0x005D719E`, `0x005D7252..0x005D725E`; compared to house `+0x1D4` | Yes |
| `HouseType+0x1A6` | special / non-participant house gate | `0x006868A8`, `0x005D6EF0`, `0x005D6FDD` loop gate | Yes |
| `House+0x1602A` | launch node/name string compared to human node records | `0x005D6F1E..0x005D6F31` | Yes |
| node `+0x6B` | observer marker; `-1` skips unit generation for that human node | `0x005D6F41..0x005D6F53` | Conditional; observer humans |
| `House+0x5490/+0x5494` | primary/alternate base center cells used for MCV/unit placement | `FUN_0050DF30`, `FUN_0050DEF0` | Yes |

## 3. Core Logic

### 3.1 `Post_Map_Init @ 0x00686890`

Verified control flow:

1. Saves map editor mode, sets it to 0 during generation, and skips generation in map editor.
2. If `DAT_00A8B23C == null`, calls `ScenarioClass::Generate_Random_Units @ 0x006886B0`.
3. Else calls selected mode vtable `+0x84`, reloads `ECX = DAT_00A8B23C`, then calls `FUN_005D6D80`.
4. Restores map editor mode, then performs event/crate/final house init and mode `+0x88/+0x8C` callbacks.

Assembly evidence: `0x00686928 MOV EAX,[ECX]`, `0x0068692B CALL [EAX+0x84]`, `0x00686931 MOV ECX,[0x00A8B23C]`, `0x00686937 CALL 0x005D6D80`, null branch `0x00686940 CALL 0x006886B0`.

Active in YR: Yes. `ScenarioClass__Post_Map_Init` callers are `ScenarioClass__Full_Init @ 0x00686B20` and `ScenarioClass__Read_Scenario @ 0x00684620`.

### 3.2 Standard Mode Vtable Bindings

Read-only vtable memory verifies the standard callbacks:

| Mode vtable | Mode | `+0x84` | `+0xC8` | `+0xCC` | Finding |
|---|---|---:|---:|---:|---|
| `0x007EE184` | Battle | `0x005D6C70` | `0x005D7030` | `0x005D70F0` | standard path |
| `0x007EE50C` | ManBattle | `0x005D6C70` | `0x005D7030` | `0x005D70F0` | same standard start generation |
| `0x007EE424` | FreeForAll | `0x005D6C70` | `0x005D7030` | `0x005D70F0` | same standard start generation |
| `0x007EE27C` | Cooperative | `0x005C2EF0` | `0x005D7030` | `0x005D70F0` | custom `+0x84`, standard MCV/units |
| `0x007EE6FC` | Siege | `0x005CA800` | `0x005CAAC0` | `0x005D70F0` | custom MCV/base callback, standard extra units |
| `0x007EE814` | Unholy | `0x005D6C70` | `0x005CB440` | `0x005D70F0` | custom MCV/base callback, standard extra units |

`0x005D7030` data xrefs include `0x007EE24C`, `0x007EE344`, `0x007EE4EC`, `0x007EE5D4`, `0x007EE6BC`, and `0x007EEE28`; the Battle vtable bytes at `0x007EE24C` decode to `30 70 5D 00`, and `0x007EE250` decodes to `F0 70 5D 00`.

Active in YR: Yes/Conditional by selected mode. Stock local `mpmodesmd.ini` exposes Battle, ManBattle, FreeForAll, Unholy, and Cooperative; Siege is binary-supported but not present in the exposed local roster.

### 3.3 `FUN_005D6D80` Budget And Per-House Orchestration

If `DAT_00A8B270 <= 0`, the budget pointer remains zero and the function still iterates houses. This means `UnitCount=0` does not itself suppress the MCV callback; `Bases` decides MCV creation in `+0xC8`.

When `UnitCount > 0`, it:

1. Builds a side mask from all non-special houses: `mask |= 1 << (HouseType+0xB4 & 0x1F)`.
2. Scans unit and infantry type arrays.
3. Includes only types where `+0x6D5 != 0`, type house mask intersects the live side mask, and type tech level is `<= DAT_00822CF4`.
4. Excludes `RulesClass+0xB20` BaseUnit entries from the unit-type scan before summing.
5. Computes `budget = (((eligible_count / 2 + total_cost) / eligible_count) * UnitCount)`, after forcing `eligible_count = 1` when no eligible types were found.

That `eligible_count / 2` term is integer rounding before division by count: it rounds the average cost to nearest-ish integer rather than truncating straight down.

Then it loops `g_HouseClass_Array` in deterministic array order. For each non-special house:

1. If a matching local human node is observer (`node+0x6B == -1`), it skips this house.
2. Calls selected mode `+0xC8(house, &budget)`.
3. Calls selected mode `+0xCC(house, &budget)`.
4. If the stack-local leftover credit value is positive, calls `HouseClass__Add_Credits`.
5. Calls vtable `+0x0C` on four house vectors at `House+0x55A0`, `+0x55DC`, `+0x55C8`, `+0x55B4`.
6. Logs final sync random with `Random(0, 0xFFFF)` and `Finished unit generation. Random`.

Assembly evidence for the two mode callback calls: `0x005D6F59..0x005D6F69` pushes `&budget` and house then calls `[selected_vtable+0xC8]`; `0x005D6F77..0x005D6F83` repeats the same arguments and calls `[selected_vtable+0xCC]`.

Active in YR: Yes for selected-mode Skirmish starts.

### 3.4 Standard `+0xC8 @ 0x005D7030` MCV/Base Callback

`0x005D7030` is the standard MCV callback for Battle, ManBattle, FreeForAll, and Cooperative:

1. If `DAT_00A8B258 == 0` (`Bases=no`), returns true without creating an MCV.
2. Calls `FUN_00505310(Rules+0xB20)` to choose the first `BaseUnit` whose `Type+0x6CC` side mask matches the current house side.
3. Allocates `0x8E8` bytes and constructs a `UnitClass` with that base unit and house.
4. Calls `FUN_0050DF30` to convert the house base center to coordinates and tries `vtable+0xD8 Place`.
5. If direct place fails, calls `FUN_0050DEF0` to get the base cell and `FUN_00688ED0(mcv, base_cell, 1)` for fallback placement.
6. Deletes the MCV object via vtable `+0x20` if both placement paths fail.

Active in YR: Yes. Standard YR `BaseUnit=AMCV,SMCV,PCV`, so side/country behavior is data-driven through masks and list order, not by hardcoded Rust country branches.

### 3.5 Standard `+0xCC @ 0x005D70F0` Extra Unit Callback

Ghidra lacks a function boundary at `0x005D70F0`, but vtable memory binds it and disassembly covers `0x005D70F0..0x005D7494`. It is the standard extra-unit generator:

1. Reads the `int*` budget argument; if `*budget <= 0`, returns true immediately.
2. Builds two dynamic candidate lists from the infantry/unit type arrays.
3. Candidate gates are `AllowedToStartInMultiplayer` (`+0x6D5`), `TechLevel <= house+0x1D4`, and `Type+0x6CC & house_side_mask != 0`.
4. One list also excludes `Rules+0xB20` BaseUnit entries.
5. The spending loop compares spent cost against `(budget * 2) / 3` using the signed multiply-by-`0x55555556` divide-by-3 sequence at `0x005D7337..0x005D7347`.
6. It creates objects through type vtable `+0x8C`, filters/placeable-converts, calls `FUN_00688ED0(..., radius=4)` for placement, adds type cost through vtable `+0xAC`, applies Initial Veteran when special flags include `0x200`, then assigns mission `5` for human control or `0x0B` for AI/control false via object vtable `+0x1F0`.

Assembly evidence: candidate gates at `0x005D7178`, `0x005D7186`, `0x005D7192..0x005D719E`, base-unit exclusion at `0x005D71A0..0x005D71C4`, create/place path `0x005D7393..0x005D73C4`, veteran gate `0x005D73F5..0x005D740A`, mission assignment `0x005D741A..0x005D742A`.

Active in YR: Yes for all stock mode vtables checked here via common `+0xCC = 0x005D70F0`.

### 3.6 Fallback Placement `FUN_00688ED0`

The shared placement helper first tries the target cell exactly if it is in-playfield and free/compatible, placing at cell center `cell * 0x100 + 0x80` with ground height. If that fails, it searches radius `param_3..0x1F`:

- picks a random starting compass direction `Random(0,7)`;
- tries 8 directions with map-bound clamping;
- does a second pass with random jitter `0..1` cells on each axis and a `< 0x32` (50%) sign choice;
- skips the original target cell;
- returns 1 on first successful `Place`, otherwise 0 after radius 31.

Active in YR: Yes. Standard MCV callback uses radius 1; standard extra-unit callback uses radius 4.

## 4. INI Keys

| INI key | Stock YR value | Binary consumer | Effect | Active in YR |
|---|---:|---|---|---|
| `[MultiplayerDialogSettings] UnitCount` | `10` | `DAT_00A8B270`, `0x005D6D83` | multiplier for average-cost budget; zero disables extra-unit budget but not MCV callback | Yes |
| `[MultiplayerDialogSettings] MinUnitCount` | `0` | shell/default range, not this post-map consumer | permits `UnitCount=0` | Yes |
| `[MultiplayerDialogSettings] MaxUnitCount` | `10` | shell/default range, not this post-map consumer | upper dialog value | Yes |
| `[MultiplayerDialogSettings] Bases` | `yes` | `DAT_00A8B258`, `0x005D7030` | gates MCV creation; `Bases=no` skips standard MCV callback | Yes |
| `[General] BaseUnit` | `AMCV,SMCV,PCV` | `Rules+0xB20`, `FUN_00505310` | first matching side-mask entry supplies opening MCV type | Yes |
| object `AllowedToStartInMultiplayer` | default yes unless `no` | `Type+0x6D5` | random starting-unit eligibility | Yes |
| object `TechLevel` | type-specific | `Type+0x634 <= House+0x1D4` | tech cap for random start candidates | Yes |
| object `Owner` / house mask | type-specific | `Type+0x6CC & side_mask` | side/country-family filter for MCV and extra units | Yes |

## 5. Integration Points

`Post_Map_Init` runs after map objects/houses are loaded and after start assignment setup. Its start-unit generation is therefore not the shell and not map roster parsing. It consumes houses already created from the launch session and start/base centers already established by earlier init.

The selected mode callbacks split responsibilities:

- `+0x84`: selected mode pre/post start assignment helper before `FUN_005D6D80`.
- `+0xC8`: MCV/base callback; standard is `0x005D7030`, but Siege and Unholy override it.
- `+0xCC`: extra unit budget callback; stock checked modes share `0x005D70F0`.
- `+0x88/+0x8C`: later post-init callbacks after crate/final house setup.

`ScenarioClass::Generate_Random_Units @ 0x006886B0` remains a useful null-mode comparison and uses the same concepts, but it is not the standard selected-mode offline Skirmish branch.

## 6. Current Rust Implementation Status

Current Rust is better than older two-MCV wording, but still not native:

| Rust surface | Current status | Native delta |
|---|---|---|
| `src/skirmish_launch.rs:111..174` | stores `unit_count`, `bases`, `tech_level`, and maps them into `GameOptions` | options exist but are not consumed by native start-unit generation |
| `src/app_skirmish.rs:162..248` | creates launch houses from session, assigns starts, spawns one MCV per assignment | no `UnitCount` budget, no `+0xC8/+0xCC` callback split, no extra random units |
| `src/app_skirmish.rs:375..419` | explicit starts then first unused start; deficient starts set `unsupported_deficient_starts` | no native `Gather_Start_Positions` random fallback or farthest/random assignment behavior in this path |
| `src/skirmish_launch.rs:56..62` / `src/app_skirmish.rs:541` | hardcoded MCV candidate order by `LaunchCountry.side_index()` | native uses `[General] BaseUnit` vector and type house masks via `FUN_00505310` |
| `src/sim/game_options.rs:44,73` | `unit_count` default 10 is present | no spending loop by average eligible cost |
| legacy `src/app_skirmish.rs:25..126` | older `seed_skirmish_opening_if_needed` still has `take(2)` | stale for launch-session path, but still a shortcut if that legacy path is used |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Post_Map_Init` selected-mode branch | verified | decompile `0x00686890`; assembly `0x00686928..0x00686940` | none |
| `FUN_005D6D80` budget formula | verified | decompile `0x005D6D80`; assembly entry `0x005D6D83..0x005D6D90` | none |
| standard Battle/ManBattle/FFA vtable `+0xC8/+0xCC` | verified | vtable memory `0x007EE184`, `0x007EE50C`, `0x007EE424`; data xrefs to `0x005D7030` | none |
| Cooperative standard MCV/extra-unit callbacks | verified for `+0xC8/+0xCC` binding | vtable memory `0x007EE27C` | custom `+0x84` body not decoded |
| Siege / Unholy custom `+0xC8` | touched-not-exhausted | vtable memory `0x007EE6FC`, `0x007EE814` | separate mode-specific investigation if implementing those modes |
| standard MCV callback `0x005D7030` | verified | decompile and assembly `0x005D7030..0x005D70AA`; callees | none |
| standard extra-unit callback `0x005D70F0` | verified from disassembly | vtable memory plus assembly `0x005D70F0..0x005D7494` | no Ghidra function boundary; do not rename/create without approval |
| fallback placement `0x00688ED0` | verified | decompile `0x00688ED0` | none for this handoff |
| current Rust delta | verified by source scan | `src/app_skirmish.rs`, `src/skirmish_launch.rs`, `src/sim/game_options.rs` | implementation remains future work |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does standard offline Skirmish call null-mode Generate_Random_Units directly? -> No when a selected mode object exists; it calls mode +0x84 then FUN_005D6D80.` (evidence: `0x00686890`, `0x00686928..0x00686940`)
- `[RESOLVED] OQ-2 - What consumes UnitCount? -> DAT_00A8B270 in FUN_005D6D80 computes average eligible cost times UnitCount.` (evidence: `0x005D6D83`, decompile `0x005D6D80`)
- `[RESOLVED] OQ-3 - Does UnitCount=0 suppress MCV creation? -> No; it suppresses the extra-unit budget, but house iteration still calls mode +0xC8; Bases gates standard MCV creation.` (evidence: `0x005D6D8E`, `0x005D6F69`, `0x005D7030`)
- `[RESOLVED] OQ-4 - How is the MCV type selected? -> standard callback calls FUN_00505310 over Rules+0xB20 BaseUnit and returns first type whose house mask matches the current house side.` (evidence: `0x005D7051..0x005D7059`, `0x00505310`, `rulesmd.ini BaseUnit`)
- `[RESOLVED] OQ-5 - Which houses are iterated? -> all `g_HouseClass_Array` entries in order, skipping special house types and observer human nodes.` (evidence: `0x005D6EF0..0x005D6F53`)
- `[RESOLVED] OQ-6 - What candidate gates are used for random start units? -> spawnable, type house mask, type tech <= house tech; one list excludes BaseUnit entries.` (evidence: `0x005D7178..0x005D71C4`, `0x005D722F..0x005D7299`)
- `[RESOLVED] OQ-7 - Does standard callback use direct waypoint spawn only? -> No; it uses base center coords, direct `Place`, then `FUN_00688ED0` spiral fallback.` (evidence: `0x005D7090..0x005D70C4`, `0x00688ED0`)
- `[RESOLVED] OQ-8 - Does current Rust still cap all session starts at two MCVs? -> No for `apply_skirmish_launch_session`; yes for older `seed_skirmish_opening_if_needed` shortcut if used.` (evidence: `src/app_skirmish.rs:162..248`, `src/app_skirmish.rs:25..126`)
- `[DEFERRED] OQ-9 - Exact Siege custom +0xC8 behavior.` (category: out-of-scope; reason: custom role mode is a separate swarm slot; next-step-if-pursued: decode `0x005CAAC0` with data/control-flow disassembly)
- `[DEFERRED] OQ-10 - Exact Unholy custom +0xC8 behavior.` (category: out-of-scope; reason: not standard Battle start-unit budget; next-step-if-pursued: decode `0x005CB440`)
- `[DEFERRED] OQ-11 - Exact Cooperative +0x84 body.` (category: out-of-scope; reason: cooperative mode start assignment differs from standard Battle and was not required for Battle UnitCount handoff; next-step-if-pursued: decode `0x005C2EF0`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Selected-mode post-map generation calls `+0xC8` MCV/base then `+0xCC` extra units for every non-special, non-observer house | `0x00686937`, `0x005D6F59..0x005D6F83`, vtables `0x007EE184+0xC8/+0xCC` | missing; Rust directly spawns one MCV per assignment | `src/app_skirmish.rs`, future scenario-start module | model a post-map start generation stage over launch houses, not ad hoc spawn-per-slot only | `skirmish_start_generation_invokes_all_active_non_observer_houses` | Do not treat Start Game or house creation as the unit generation step |
| `UnitCount` is a rounded average-cost budget over eligible types; `UnitCount=0` still allows MCVs when `Bases=yes` | `0x005D6D83..0x005D6EA9`, `0x005D7030` | `unit_count` stored but unused for spawning | `src/skirmish_launch.rs`, `src/sim/game_options.rs`, start generator | use `unit_count` to spend cost budget for extra starting units after MCV handling | `skirmish_unit_count_zero_spawns_mcv_only_when_bases_enabled` | Do not spawn exactly N units; the slider is not a literal unit count |
| Standard MCV type comes from `[General] BaseUnit` first side-mask match, not hardcoded country candidates | `FUN_00505310`, `rulesmd.ini BaseUnit=AMCV,SMCV,PCV` | hardcoded candidates by `LaunchCountry.side_index()` | `src/skirmish_launch.rs`, `src/app_skirmish.rs` | resolve base unit from parsed rules vector and type owner/house masks | `skirmish_baseunit_vector_selects_side_matching_mcv` | Do not special-case AMCV/SMCV/PCV beyond parsed INI fallback |
| Extra-unit candidates require `AllowedToStartInMultiplayer`, tech <= house tech, and house-mask intersection; BaseUnit entries are excluded from one list | `0x005D7178..0x005D71C4`, `0x005D722F..0x005D7299` | missing | rules object model and skirmish start generator | filter the random start roster from rules data before budget spending | `skirmish_start_unit_budget_filters_spawnable_tech_and_house_mask` | Do not include harvesters/miners or disabled advanced units just because they are buildable later |
| Placement uses native `Place` plus spiral fallback radius 1 for MCVs and radius 4 for extra units | `0x005D70AD..0x005D70C4`, `0x005D73B0..0x005D73C4`, `0x00688ED0` | direct `spawn_object` at waypoint/base cell | `src/app_skirmish.rs`, `src/sim/world/world_spawn.rs` | try direct placement, then deterministic fallback search around base cell | `skirmish_start_unit_uses_spiral_fallback_when_start_cell_blocked` | Do not fail the whole house start just because the exact waypoint cell is occupied |

## 10. Negative Facts / Do Not Do

- Do not say current launch-session Rust starts at most two MCVs. That is stale for `apply_skirmish_launch_session`; the remaining gap is native budget/callback/fallback parity.
- Do not use null-mode `Generate_Random_Units @ 0x006886B0` as the standard selected Battle Skirmish implementation. It is the fallback when `DAT_00A8B23C == null`.
- Do not treat `UnitCount` as "spawn this many objects." Native computes a cost budget from average eligible type cost.
- Do not suppress MCV creation when `UnitCount=0`; standard MCV creation is gated by `Bases`, not by positive UnitCount.
- Do not hardcode AMCV/SMCV/PCV selection when the parsed `[General] BaseUnit` vector and type masks are available.
- Do not put CMIN/HARV into the start roster unless their `AllowedToStartInMultiplayer` gate and masks make them eligible; stock harvesters/miners are excluded.

## 11. Remaining Uncertainty

- Siege `+0xC8 = 0x005CAAC0`, Unholy `+0xC8 = 0x005CB440`, and Cooperative `+0x84 = 0x005C2EF0` need separate mode-specific investigations before implementing those modes fully.
- Ghidra has no function boundary for `0x005D70F0`; this report uses vtable memory plus disassembly address ranges rather than decompiler pseudocode for that body.
- Exact RNG stream parity for all extra-unit picks was not runtime-debugged; static evidence pins call sites and ranges, not a full seed replay transcript.

## 12. Stale Docs / Follow-up Docs

- Replace older wording like "offline/skirmish Post_Map_Init calls Generate_Random_Units directly" with: "In standard offline Skirmish with a selected MPModes object, `Post_Map_Init @ 0x00686890` calls selected mode `+0x84`, then `FUN_005D6D80`; `Generate_Random_Units @ 0x006886B0` is the null-selected-mode fallback."
- Replace older Rust status wording like "Rust starts at most two MCVs" with: "The legacy `seed_skirmish_opening_if_needed` shortcut still caps at two pairings, but the current launch-session path `apply_skirmish_launch_session` iterates assigned launch slots. It still lacks native `UnitCount` budgeted extra units, BaseUnit mask selection, selected-mode callbacks, and spiral fallback placement."
- Keep the prior `House+0x16058`/`House+0x1605C` correction unchanged: this report does not change start/team field ownership.

## Sources

- Ghidra decompiled/read-only: `ScenarioClass__Post_Map_Init @ 0x00686890`, `FUN_005D6D80 @ 0x005D6D80`, `FUN_005D7030 @ 0x005D7030`, `FUN_00505310 @ 0x00505310`, `FUN_0050DF30 @ 0x0050DF30`, `FUN_0050DEF0 @ 0x0050DEF0`, `FUN_00688ED0 @ 0x00688ED0`, `ScenarioClass__Generate_Random_Units @ 0x006886B0`.
- Ghidra assembly/disassembly context: `0x00686928..0x00686940`, `0x005D6F59..0x005D6F83`, `0x005D70F0..0x005D7494`, `0x005D7178..0x005D71C4`, `0x005D722F..0x005D7299`.
- Ghidra read memory: vtables `0x007EE184`, `0x007EE50C`, `0x007EE424`, `0x007EE27C`, `0x007EE6FC`, `0x007EE814`; Battle `0x007EE24C..0x007EE25B`.
- Existing docs referenced: `skirmish-ui/SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`, `skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_HOUSE_0X1605C_TEAM_ADJUNCT_CONSUMER_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`, `FIRST_ALLIED_MINER_SOURCE_GHIDRA_REPORT.md`, `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/mpmodesmd.ini`.
- Rust scanned: `src/app_skirmish.rs`, `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs`, `src/sim/game_options.rs`.

## Status

COMPLETE for standard selected-mode Battle/ManBattle/FreeForAll post-shell start-unit budget and Rust handoff. PARTIAL only for custom non-Battle `+0x84/+0xC8` mode bodies, which are explicitly outside this slot's implementation handoff.
