# AI_RecalcBuildOptions BasePlan Generation Design

**Date:** 2026-08-26
**Phase:** 3 — GSI-04.05
**Status:** Approved bounded mechanism
**Native authority:** `HouseClass__AI_RecalcBuildOptions @ 0x005054B0`, `FUN_00505180 @ 0x00505180`, successful `UnitClass__Deploy @ 0x007393C0` block `0x00739855..0x00739926`, active-retail rules data

## Verdict

Rust now owns the ordered BasePlan state but fresh non-human skirmish play still leaves it empty. Native populates that vector through `AI_RecalcBuildOptions`, using seven source-ordered `[AI]` BuildingType lists, global BuildingType registration order, exact eligibility and prerequisite topology, and shared Scenario RNG insertions. A successful non-controlled ConstructionYard deploy invokes that generator only for an empty plan, then writes node zero and the distinct BasePlan center.

This mechanism implements that active-retail plan-generation transaction and its plan/center outputs. It deliberately does not claim the complete native deploy branch: `Computer_Paranoid`, three House flag writes, and `FUN_0050C920` unit dispersal remain separately active open mechanisms. `HouseClass__RecenterBase` also remains open because its only verified caller is trigger action 30, not a generic structure lifecycle hook.

## Player-experience ledger

| Retail behavior | Rust owner after this slice |
|---|---|
| An AI MCV produces a complete ordered base plan when its ConstructionYard successfully appears | `deploy_mcv` invokes the House BasePlan generator after successful Building Unlimbo and only when the plan is empty |
| Different factions and countries receive their native infrastructure order | `RuleSet` retains the exact planning lists and type facts; the generator filters global BuildingType order with the House country, side, and TechLevel |
| Prerequisites determine later plan order without using the player build menu | The generator uses native negative Rules-list tokens and explicit BuildingType indices, not generic production eligibility |
| Refineries and defense sentinels appear at deterministic random positions | Every inclusive insertion draw mutates `Simulation.scenario_rng` in native order and sees the previously shifted vector |
| The deployed yard anchors node zero and BasePlan placement center | House state keeps a distinct packed-zero-default BasePlan center; the deploy transaction writes primary center, node zero, then BasePlan center |
| Save/load and lockstep preserve generated planning state | Existing BasePlan nodes and Scenario RNG remain authoritative; the new center is snapshot/hash-covered in schema v107 |

## Exact Rules/type inputs

### `[AI]` BuildingType lists

`RulesClass__ReadAI @ 0x00672AE0` owns all seven lists. Each uses the same native `char[128]` ReadString projection, whole-buffer trim, comma-only `strtok`, case-insensitive BuildingType resolution, native sentinel omission, source order, and duplicates already established for BuildConst:

| List | Key/block anchor |
|---|---:|
| BuildConst | `0x00672B14..0x00672C01` |
| BuildPower | `0x00672C06..0x00672CE5` |
| BuildRefinery | `0x00672CEB..0x00672DCA` |
| BuildBarracks | `0x00672DD0..0x00672EAF` |
| BuildTech | `0x00672EB4..0x00672F83` |
| BuildWeapons | `0x00672F89..0x00673058` |
| BuildRadar | `0x0067368C..0x0067375B` |

Active YR values are:

- `BuildConst=GACNST,NACNST,YACNST`
- `BuildPower=NAPOWR,GAPOWR,YAPOWR`
- `BuildRefinery=NAREFN,GAREFN,YAREFN`
- `BuildBarracks=NAHAND,GAPILE,YABRCK`
- `BuildTech=NATECH,GATECH,YATECH`
- `BuildWeapons=GAWEAP,NAWEAP,YAWEAP`
- `BuildRadar=GAAIRC,NARADR,AMRADR,NAPSIS`

All active-retail tokens already exist in BuildingTypes. Custom unknown allocation depends on the complete later `ReadAI` list order and remains an evidence-backed stock-inactive exclusion rather than a partial approximation.

### Object facts and masks

- Add `ObjectType.ai_build_this`, constructor default false at `0x0045E21F`, parsed by `0x00460FE2..0x00460FF6`.
- Preserve signed `TechLevel` without a lower-bound gate.
- Reuse exact native AI-list country masks: empty Owner is mask zero and rejects; empty Required/Forbidden retain their separately verified `-1` semantics.
- Reuse `AIBasePlanningSide`, registered BuildingType index, primary SuperWeapon resolution, BuildTech exemption, and `SuperWeaponType.disableable_from_shell`.
- Scan `RuleSet.building_ids` in native registration order. Do not use `BuildOption.enabled`, credits, factories, BuildLimit, stolen tech, generic prerequisite groups, or UI/sidebar order.

### General lists and difficulty vectors

Retain source-ordered `[General] HarvesterUnit` from `RulesClass__ReadGeneral @ 0x0066D530`, block `0x0066F8C8..0x0066F9CB`. It uses a native `char[128]` reader, whole-buffer trim, comma-only `strtok`, no per-token trim, case-insensitive UnitType resolution, null omission for `none`/`<none>`, and preserved order/duplicates. Active YR resolves the already-registered `HARV,CMIN`; custom unknown UnitType allocation and OOM order remain stock-inactive exclusions parallel to the BuildingType lists.

Parse the signed DynamicVectors used by Recalc from `[General]`:

- `AISlaveMinerNumber=4,3,2` at `Rules+0x133C/+0x1340`, reader `0x00670585..0x006705B7`;
- `AIExtraRefineries=2,1,0` at `+0x1374/+0x1378`, reader `0x006705F9..0x0067062A`;
- `AlliedBaseDefenseCounts=25,20,6` at `+0xD80/+0xD84`, reader within `0x00670013..0x006700BE`;
- `SovietBaseDefenseCounts=25,22,6` at `+0xD9C/+0xDA0`;
- `ThirdBaseDefenseCounts=25,22,6` at `+0xDB8/+0xDBC`.

Each constructor/missing-key state is an empty vector. `DifficultyClass__ReadINI_IntVector @ 0x00475D70` uses a `char[512]` reader (511 payload bytes), whole-buffer trim, comma tokenization with collapsed empty fields, and native wrapping `atoi`; it retains every parsed entry without clamping or padding to three. A missing/empty key preserves the existing vector. Store exact `Vec<i32>` contents rather than substituting retail defaults.

The native difficulty index is exactly `HouseDifficulty::{Hard,Normal,Easy} = 0,1,2`, and native Recalc directly indexes these vectors without a count check. Rust must perform a deterministic **VERA-internal** preflight only after ordinary deployment validation has identified a qualifying non-human, nonzero-mode ConstructionYard whose BasePlan is empty and therefore will actually invoke Recalc. The selected refinery-count branch must contain the current difficulty index. Only side zero/one/two requires its matching defense vector; an unknown side inserts zero and needs none. Unsupported malformed data fails before destructive MCV removal without consuming generator RNG. This defensive failure has no claimed gamemd equivalent; it does not pad, clamp, or invent a simulation value. A nonempty plan performs native node-zero/center writes without running this preflight or generator. `HarvestersPerRefinery=2,2,1` is a separate active rule but is not read by `AI_RecalcBuildOptions` and is not added as dead generator state in this slice.

## Exact generator transaction

### Eligibility vector

Preserve `BasePlanState.percent_built`, clear the complete node vector, and scan BuildingTypes in global registration order. Include a type only when all gates pass:

1. Owner mask contains the House country/type bit.
2. `AIBasePlanningSide == house.side_index` or `-1`.
3. `AIBuildThis` is true.
4. signed `TechLevel <= house.tech_level`.
5. RequiredHouses is unrestricted or contains the House bit.
6. ForbiddenHouses is unrestricted or excludes the House bit.
7. With shell superweapons disabled and a resolved primary SuperWeapon, the exact type occurs in BuildTech or the SuperWeaponType is not disableable from shell.

Create a parallel selected-byte vector initialized false.

### Seed and topology order

1. Resolve first-buildable BuildConst using only the native AI-list selector. If it occurs in eligible, mark its eligible slot selected and append it.
2. Resolve first-buildable BuildPower and append it unconditionally. Do not mark its eligible occurrence selected, so it may appear again later.
3. Resolve first-buildable BuildBarracks. If it occurs in eligible, move it to eligible slot zero while reproducing native selected-byte movement: source receives old slot-zero byte and slot zero becomes false.
4. Resolve first-buildable BuildWeapons and do the same at eligible slot one.
5. Set signed/wrapping-equivalent `remaining = eligible_count - 1` irrespective of successful seeds.
6. At the start of every full eligible scan, snapshot `pass_start_len = priority.len()`. Repeatedly scan unselected eligible entries in current order. Literal ID `GAPLUG` cannot take the normal prerequisite-success branch. Evaluate every other candidate only against `priority[..pass_start_len]`: a type appended earlier in the same scan is invisible to later prerequisites until the next pass. Append and select every passing type, decrementing remaining. If a pass makes no progress, append/select the last unselected index as the deterministic cycle break. Stop when remaining reaches zero or no unselected entry exists.

For each prerequisite token, require one pointer in the pass-start priority slice:

- explicit registered BuildingType index;
- `POWER -> BuildPower`;
- `FACTORY -> BuildWeapons`;
- `BARRACKS -> BuildBarracks`;
- `RADAR -> BuildRadar`;
- `TECH -> BuildTech`;
- `PROC -> BuildRefinery`.

These are the native `-1..-6` token families. Do not consult Rust's generic prerequisite alias satisfaction.

### Refinery duplication and RNG

Resolve first-buildable BuildRefinery. Scan HarvesterUnit forward for the first Owner-compatible UnitType. Use `AIExtraRefineries[difficulty]` when found; otherwise use `AISlaveMinerNumber[difficulty] - 1`. Find the first refinery occurrence strictly before the priority vector's last entry.

For each positive duplicate count:

1. consume `scenario_rng.next_range_u32_inclusive(refinery_index, current_count - 1)`;
2. insert the same refinery immediately after the drawn index;
3. allow that insertion to change the next range.

### Defense sentinels and RNG

Copy priority entries zero, one, and two, then the remaining entries. Active retail guarantees those entries. Select the side-specific difficulty count; unknown sides use zero.

For each sentinel:

1. consume `scenario_rng.next_range_u32_inclusive(3, final_count - 1)`;
2. insert signed `-1` immediately after the drawn index;
3. let each insertion change the next range.

The first four pre-existing entries necessarily precede every sentinel.

### BasePlan nodes

Append final values in order. Generated nonnegative values store the BuildingType registry index; signed controls `-4..-1` store literally. Every generated node is `{packed_cell: 0, filled: false, retry_count: 0}`. Allocation-failure partial vectors and native OOM corruption are outside active retail; Rust must not silently seed a generic replacement plan.

## Successful ConstructionYard deploy integration

The hook belongs in `Simulation::deploy_mcv` after successful ConstructionYard spawn/Unlimbo. Rust's `building_up` state is gameplay-significant and must still be initialized on every successful deployment; no native equivalence between that Rust field and Building byte `+0x6DD` is claimed. Prevalidate Recalc's required difficulty vectors before the destructive MCV removal, then gate the plan/center transaction on:

- successful target Building creation;
- `ConstructionYard=true`;
- `session.game_mode_nonzero`;
- `!house.is_controlled_by_human(true)`.

The target Building's committed north-west anchor cell is authoritative. Native directly reads the new Building fields `+0x9C/+0xA0`, applies signed division by 256 truncating toward zero, and narrows to the packed CellStruct; it does not call `GetCoords` or use the source Unit. In current Rust this is exactly the `rx,ry` returned by `deploy_origin_from_unit_cell` and passed to `spawn_object_at_height`. A stock 4x4 yard deployed from `(unit_rx,unit_ry)` therefore writes `(unit_rx-1,unit_ry-1)`. Within the bounded outputs, preserve native order:

1. write primary `House+0x5490` through existing `HouseState.base_center`;
2. if BasePlan node count is zero, run the complete generator above with the shared Scenario RNG;
3. write the yard cell to node zero without changing its type/control, filled latch, or retry count;
4. write distinct `House+0x5750` through new `HouseState.base_plan_center`.

Nonempty scenario-authored plans are not regenerated, but node zero and the BasePlan center are still written by a qualifying successful deploy. Failed deploy, human-controlled owner, campaign/mode zero, and non-ConstructionYard targets perform none of this bounded transaction.

## State, snapshot, and hash

- Add `HouseState.base_plan_center: (u16,u16)`, default `(0,0)`.
- Bump current snapshot schema `106 -> 107`.
- Persist and hash the new center only in v107/current state. Preserve all historical hash/version probes.
- Existing BasePlan nodes, House facts, Scenario session option, and Scenario RNG are already snapshot/hash authorities.
- Immutable Rules/type facts do not enter snapshots.

## Focused acceptance

1. All seven `[AI]` lists and `[General] HarvesterUnit` use the exact 127-byte parser. Fixtures pin the byte prefix, whole-buffer trim, comma-only tokenizer, collapsed empty fields, no per-token trim, `none`/`<none>` omission, duplicates, spelling/order, case-insensitive registered category resolution, missing/empty preservation across rules layers, and poisoned wrong-section keys.
2. `AIBuildThis` defaults false and parses exact true/false values.
3. Eligibility truth table covers Owner zero, Required/Forbidden, side, signed TechLevel, AIBuildThis, and shell-SuperWeapon tail.
4. Seed test proves BuildConst selected, BuildPower unselected, Barracks/Weapons selected-byte movement, and no generic eligibility gates.
5. Prerequisite tests cover all six token families, explicit BuildingType identity, GAPLUG withholding, deterministic last-unselected cycle break, and the fixed `pass_start_len` boundary proving an earlier same-pass append is invisible until the next scan.
6. Refinery insertion fixtures distinguish `AIExtraRefineries` from `HarvestersPerRefinery`, pin Hard/Normal/Easy duplicate counts, node order, inclusive ranges, insertion-shifted subsequent ranges, and final Scenario RNG state; no-Harvester fallback pins `AISlaveMinerNumber - 1`.
7. Allied/Soviet/Third difficulty fixtures pin sentinel counts/order/RNG; an unknown side inserts zero.
8. Generated nodes have exact zero cell/filled/retry fields and preserve PercentBuilt.
9. Successful non-human nonzero-mode ConstructionYard deploy generates before node-zero write, uses the deployed Building north-west `rx,ry`, mutates primary and distinct BasePlan centers, and still initializes normal Rust `building_up` gameplay state.
10. Human-control, campaign-mode, non-ConYard, and nonempty-plan fixtures prove the generator consumes no RNG while the otherwise successful deployment still consumes the source MCV and initializes the target Building normally; nonempty plans still receive node-zero and center writes.
11. Actual placement failure and VERA-internal malformed-vector preflight preserve the source MCV, consume no generator RNG, and produce no center/plan mutation.
12. DynamicVector parser fixtures pin the 511-byte prefix, whole-buffer trim, collapsed empty fields, native leading-whitespace/sign/decimal-prefix/wrapping `atoi`, exact short/long length without padding/clamping, and missing/empty preservation across rules layers.
13. BasePlan center round-trips and affects only current/v107 state hash; snapshot version gates remain correct.
14. Existing BasePlan lifecycle, BuildConst/naval, MCV deploy, snapshot, and RNG-routing focused filters remain green.

## Explicit open mechanisms

The following keep GSI-04.05 open after this mechanism passes:

- `Computer_Paranoid`/alliance recalculation before Recalc in the native deploy branch;
- House bytes `+0x1EE/+0x1F2/+0x1F3` and their verified readers;
- post-deploy `FUN_0050C920` owned-unit dispersal;
- `HouseClass__RecenterBase @ 0x0050C210`, its trigger-action-30 caller, and retail activation census;
- `HouseClass__ComputerTakeover` population;
- runtime refinery/weapons insertions, wall expansion, projected-power splice;
- wildcard satisfaction/recycling, production-plan site writes, cached-site selection, ordinary selector, placement result integration, and scheduler/influence/defense planning;
- full custom unknown-Type allocation ordering and native OOM states.

No item above is treated as inactive merely because this slice does not implement it.
