# Phase 3 Team Production Reachability — Ghidra Research Report

**Address(es):** `0x0052CD70`, `0x00686B20`, `0x006F19B0`, `0x00691970`, `0x006E8220`, `0x0041F2E0`, `0x004F8440`, `0x006F0AB0`, `0x006F09C0`, `0x006E8A90`, `0x006E9140`, `0x006EAA90`, `0x006EA610`, `0x006EA500`
**Investigation Mode:** coverage-map
**Claimed Scope:** active-YR fixed AI-data loading and map override order; the minimum ordinary-skirmish path from House AI-trigger cadence through TeamType selection, empty Team creation, and TaskForce recruitment/admission; retail-data liveness and evidence-backed exclusions needed to make the Phase 3 base-defense Team response reachable
**Non-Scope:** complete AITrigger condition semantics, all ScriptType opcode bodies, campaign/map-trigger reinforcement creation, complete AI production strategy, Railgun/LaserDraw/Sonic Wave/destroyable-cliff behavior, and all Tiberian Sun legacy
**Confidence:** High for loader order, registry shapes, ordinary creation cadence, empty-Team construction, recruitment scan shape, and the retail-data census. Medium for individual unnamed candidate-admission predicates whose callees were not recursively decoded. Those predicates remain an implementation blocker rather than being approximated.
**Active in YR:** Yes. The standalone YR load root is `AIMD.INI`; the House update calls the AI-trigger selector on `TeamDelays` cadence in ordinary nonhuman play; the selected TeamTypes create empty TeamClass instances which recruit against live units on subsequent Team ticks.

## 1. Verdict

The retained Phase 3 base-defense Team response is not production-reachable in current Rust. The native producer is a four-stage mechanism, not a single missing registration call:

1. `AIMD.INI` is loaded into ScriptType, TaskForce, TeamType, and AITriggerType registries, then the map INI overlays those same registries by identifier.
2. each nonhuman House periodically evaluates the live AITriggerType registry using difficulty, owner, technology, condition, buildability, TeamType capacity, base-defense capacity, and weighted-selection gates;
3. the selected primary and optional secondary TeamTypes create **empty** TeamClass instances;
4. each Team tick recruits live matching objects one TaskForce slot at a time using owner, type, locomotion/state, group/zone, priority-preemption, and capacity gates.

Rust currently installs `TeamScriptVm::default()` with no production definitions or Teams. Its only TeamType/TaskForce/Script registrations and Team creation calls are in tests. Its `create_team_from_type` also admits a supplied candidate slice immediately, whereas retail construction is empty and live-world recruitment is incremental. The separate Rust attack-wave routine cannot be treated as a native Team producer: it selects generic idle military objects by stable ID and issues attack-move commands, bypassing AITrigger selection, TaskForce quotas, TeamType priority/Max/base-defense state, and native recruitment order.

The smallest coherent next prerequisite is therefore the exact AIMD-plus-map registry loader, with source-order override tests. That work is independently verifiable and does not require inventing Team behavior. It does **not** close production reachability: the row remains open until the selector, empty creation, live recruitment, and an end-to-end no-test-registration base-defense scenario are exact and green.

## 2. Authority and Method

The active program was `gamemd.exe` in the connected `testProsjekt` Ghidra project. All binary work in this report was read-only. The analysis used decompilation, exact instruction disassembly where Ghidra's function boundary or signature was unreliable, call/xref searches, and direct inspection of the repository's retail `ini/aimd.ini` and `ini/rulesmd.ini` corpus.

Important boundary correction: `RulesClass__ReadTypeData @ 0x00679A10` does **not** load ScriptType, TaskForce, TeamType, or AITriggerType records. It loops ordinary Rules-owned type registries and MissionControl data. The four AI registries are loaded explicitly later by `ScenarioClass::Full_Init @ 0x00686B20`. Any earlier project prose assigning them to `RulesClass__ReadTypeData` is stale for the active program.

Ghidra contains a spurious nested function record named `TeamClass__Recruit_Or_Add @ 0x006E9380`. It is a mis-bounded middle span of `TeamClass::AI @ 0x006E9140`, has no independent callers, and is not used as authority here.

## 3. Active File Root and Override Order

### 3.1 Standalone YR root

`Load_Game_Rules @ 0x0052CD70` opens the standalone YR roots:

- `RULESMD.INI`
- `ARTMD.INI`
- `AIMD.INI`

`AIMD.INI` is referenced at image string `0x0082621C`; its sole code xref is the active rules loader. `AI.INI` is not an additional base layer on this standalone YR path. The analogous `RULES.INI`/`ART.INI`/`AI.INI` names must not be composed underneath the MD files.

### 3.2 Scenario registry passes

In `ScenarioClass::Full_Init`, the fixed AIMD CCINI object is at `0x00887128` and the map INI object is held in `EDI`. Exact call order at `0x0068797A..0x006879E3` is:

| Order | Registry | Fixed AIMD call | Map call |
|---:|---|---|---|
| 1 | TeamTypes | `0x00687984 -> 0x006F19B0`, source flag `1` | `0x0068798D -> 0x006F19B0`, source flag `0` |
| 2 | ScriptTypes | `0x0068799C -> 0x00691970` | `0x006879A5 -> 0x00691970` |
| 3 | TaskForces | `0x006879B4 -> 0x006E8220` | `0x006879BD -> 0x006E8220` |
| 4 | map Trigger/Tag data | — | `0x007275D0`, then `0x006E5ED0` |
| 5 | AITriggerTypes | `0x006879DA -> 0x0041F2E0` | `0x006879E3 -> 0x0041F2E0` |
| 6 | TeamType zone recompute | — | `0x006F2040` after both sources |

For each registry, an identifier already present from AIMD is found and its `ReadINI` method is called again with the map section. Thus the map mutates/overrides the existing object rather than appending a duplicate. A map identifier not found in the fixed registry allocates a new object and appends it. The loader's source byte distinguishes fixed AIMD-owned from map-owned definitions; it is not a precedence switch.

This is per-registry layering, not a single merged INI file read once. Cross-registry references resolve against the registry state available when each object is read, and the order above is observable.

## 4. Registry Contracts

### 4.1 ScriptType

`FUN_00691970` reads `[ScriptTypes]`, finds/reuses each value identifier or allocates `0x234` bytes, invokes the object's INI reader, and records the source tag at `+0x9C`.

`ScriptTypeClass::ReadINI @ 0x006918A0`:

- resets the action count at `+0xA0`;
- probes numeric keys `0` through `49` in ascending order;
- parses each nonempty value as the native integer pair;
- appends it at `+0xA4/+0xA8 + compact_index*8`;
- increments the stored count only for nonempty rows.

Numbering gaps therefore compact. Key `7` following an absent key `6` becomes the next stored action, not stored slot seven. The hard maximum is 50 records. Rust must preserve the signed action argument; it must not infer or normalize unsupported opcode meanings during load.

### 4.2 TaskForce

`FUN_006E8220` reads `[TaskForces]`, finds/reuses each identifier or allocates `0xD4` bytes, reads the object, and stores the source tag at `+0xA0`.

Constructor `0x006E7E80` sets:

- `Group` at `+0x98` to `-1`;
- count at `+0x9C` to zero;
- all six `{count, type}` pairs to zero;
- source byte at `+0xA0` to zero.

`TaskForceClass::ReadINI @ 0x006E8420` resets the count, probes numeric keys `0` through `5`, parses a signed count and TechnoType identifier, stores count first at `+0xA4+i*8` and the resolved type pointer at `+0xA8+i*8`, and increments the stored entry count only when type resolution succeeds. It then reads `Group` into `+0x98`. Six entries are the native maximum. Unresolved types do not become wildcard entries.

### 4.3 TeamType

`FUN_006F19B0` reads `[TeamTypes]`, where list values are TeamType identifiers. It finds/reuses by identifier or allocates `0xF8` bytes, calls `TeamTypeClass` construction/INI load, and writes source tag `+0xE8`.

Constructor `0x006F06E0` establishes load-bearing defaults including:

- signed `Priority +0xB4 = 7`;
- signed `Max +0xB8 = -1`;
- live count `+0xDC = 0`;
- `AreTeamMembersRecruitable +0xF5 = true`;
- `IsBaseDefense +0xF6 = false`;
- source `+0xE8 = 0`.

`TeamTypeClass::ReadINI @ 0x006F1090` reads at least the following fields used by the creation/recruitment corridor:

| INI field | Offset | Corridor use |
|---|---:|---|
| `Group` | `+0x9C` | fallback group/zone selector |
| `VeteranLevel` | `+0xA0` | member state after admission |
| `Recruiter` | `+0xA8` | candidate admission branch |
| `Autocreate` | `+0xA9` | AI creation/recruitment state |
| `Aggressive` | `+0xAD` | Team behavior after formation |
| `LooseRecruit` | `+0xAE` | recruitment gate |
| `Priority` | `+0xB4` | preemption and Phase 3 suspension threshold |
| `Max` | `+0xB8` | per-House/per-type live Team cap |
| `TechLevel` | `+0xCC` | trigger/Team eligibility |
| `Waypoint` | `+0xD4` | recruitment anchor |
| `TransportWaypoint` | `+0xD8` | transport behavior |
| `Script` | `+0xE0` | ScriptType pointer |
| `TaskForce` | `+0xE4` | TaskForce pointer |
| `AvoidThreats` | `+0xF2` | behavior flag |
| `TransportsReturnOnUnload` | `+0xF4` | behavior flag |
| `AreTeamMembersRecruitable` | `+0xF5` | member marking/admission |
| `IsBaseDefense` | `+0xF6` | House defense count and response assignment |
| `OnlyTargetHouseEnemy` | `+0xF7` | target filtering |

Other parsed booleans include `Loadable`, `Full`, `Annoyance`, `GuardSlower`, `Prebuild`, `Reinforce`, `Whiner`, `Suicide`, `Droppod`, `UseTransportOrigin`, and `OnTransOnly`, plus owner, MindControlDecision, Tag, and transport data. They are not silently discarded by native even where this corridor does not yet consume them.

`TeamTypeClass::ReadINI` resolves a valid Script identity through the call at `0x006F14A3 -> 0x00691C00` and a valid TaskForce identity through `0x006F14DC -> 0x006E85F0`. Both helpers find or allocate the requested identity immediately. They reject `none` and `<none>` through the case-insensitive comparison at `0x007C8D20`; the outer reader then substitutes registry entry zero and succeeds when that entry exists, or fails when the registry is still empty. Because the TeamType pass precedes the explicit ScriptTypes and TaskForces passes, those later readers fill the same placeholder objects in place; first TeamType-reference order therefore owns the registry prefix. A valid identity absent from its later list remains an empty native placeholder rather than attaching the first authored ScriptType or TaskForce.

### 4.4 AITriggerType

`FUN_0041F2E0` reads `[AITriggerTypes]`. Unlike the three list registries above, each **key** is the AITriggerType identifier and its value is one comma record. It finds/reuses by key or allocates `0x110` bytes. The raw reader starts at `0x0041F580`; Ghidra lacks a sound function record there, so exact disassembly is the authority.

All 165 stock AIMD records contain exactly 18 comma tokens. The loader consumes them in this structure:

| Token | Native result |
|---:|---|
| 1 | display name copied to `+0x64` |
| 2 | primary TeamType pointer at `+0xDC` |
| 3 | owner mode at `+0xA0`; named-country index at `+0xA8`, `<all>` uses mode `2` |
| 4 | required token, but its value is discarded; `+0xB0` is explicitly zeroed before primary/secondary TaskForce TechLevel folds from `0x006E8780` |
| 5 | condition enum at `+0x98` |
| 6 | optional Building/Unit/Infantry/Aircraft type pointer at `+0xD8` |
| 7 | 32-byte comparison/mask payload at `+0xE4..+0x103` |
| 8–10 | three numeric weights converted to doubles at `+0xB8`, `+0xC0`, `+0xC8` |
| 11 | boolean at `+0xD0` |
| 12 | consumed by the parser; no retained destination was established in the swept body |
| 13 | signed integer at `+0xAC` |
| 14 | boolean at `+0xD1` |
| 15 | optional secondary TeamType pointer at `+0xE0` |
| 16–18 | three difficulty booleans at `+0xD2`, `+0xD3`, `+0xD4` |

The semantic names of tokens 11–14 are intentionally not guessed. Their storage and use sites must be named from a complete consumer trace before implementation.

The token-4 parser call at `0x0041F712` checks only that the token exists; `0x0041F728` writes zero to `+0xB0` without converting the token. For each referenced TeamType, `0x0041FA5C..0x0041FADD` calls `0x006E8780` on its TaskForce and keeps the larger fold result. `0x006E8780` starts at zero and walks compact TaskForce member slots in order. A member TechLevel above the accumulator replaces it. Otherwise the accumulator is retained, except that a member TechLevel of `-1` replaces it with `11` when `g_GameMode != 0`. That exception is order-sensitive: a `12` followed by `-1` yields `11`, while `-1` followed by `12` yields `12`. TaskForce member counts do not participate.

The retail threshold oracle uses AIMD SHA-256 `5df41eaec00a78d0760ef5eecdf27d65ae1cd537309c7eac973318266986f89d` and rulesmd SHA-256 `3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`. In `[AITriggerTypes]` encounter order, UTF-8 rows formatted as `ID<TAB>threshold<LF>` hash to `3253b17c65d2006bf542c38a811ec68ef2847e588dc1f21165e7070af5d5e1f7` when `g_GameMode == 0` and `76096bc2d9592ff4c1054c23a38660e74c3860afbc2882db5c6dcc2074da8aad` otherwise. The only mode-sensitive stock row is `0C8C51BC-G` (`5` versus `11`). An oracle parser must honor native INI comment handling for the active headers `[CCOMAND] ;...`, `[YURI] ;...`, and `[DISK];...`; treating those sections as absent corrupts 16 threshold rows.

The fixed AIMD pass enables fixed records through `+0xA4`. On the map pass, `[AITriggerTypesEnable]` is keyed by AITriggerType identifier. `CCINIClass::ReadBool(..., default=false)` is read for each listed key: false clears `+0xA4` only when `g_GameMode == 0`; in nonzero game modes every listed key is enabled even when authored false. Constructor `0x0041E350` initializes the three weights to `1.0`, the three difficulty bytes true, and enabled false before the loading pass changes them (`FUN_0041F2E0`, decompiled complete body).

## 5. Ordinary-Skirmish Selection and Creation

### 5.1 Cadence and first fire

`HouseClass::SetDifficulty @ 0x004F6EC0` seeds the House timer:

```text
House+0x5798 = current binary frame
House+0x57A0 = Rules.TeamDelays[difficulty] + HouseIndex * 0xAF
```

`HouseClass::Update @ 0x004F8440` checks the timer for a nonhuman, non-passive House. On expiry it calls `FUN_006F0AB0`, creates each returned TeamType through `FUN_006F09C0`, and resets the duration to `Rules.TeamDelays[difficulty]`. Stock `rulesmd.ini` supplies `TeamDelays=2000,2500,3500`.

`AutocreateTime=1` also exists in stock rules data, but it is not the timer read by this active ordinary selector call site. Substituting `AutocreateTime` for `TeamDelays` would change cadence.

Team ticks occur before House updates. A Team created by this House pass therefore cannot execute or recruit until the next binary frame.

### 5.2 Eligibility and weighted selection

`FUN_006F0AB0` is not equivalent to selecting every TeamType with `Autocreate=yes`. It:

- performs a House probability gate with one `RandomRanged(1,100)` draw;
- counts the House's live Teams and base-defense Teams;
- applies per-difficulty total-Team and base-defense limits from Rules;
- walks the global AITriggerType array in registry order;
- calls eligibility owner `FUN_0041E720` for each record;
- forms a weighted distribution from the trigger's current difficulty weight;
- performs one `RandomRanged(1,total_weight)` draw when the distribution is nonempty;
- returns the chosen trigger's primary TeamType and optional secondary TeamType in that order.

`FUN_0041E720` rejects at least disabled, wrong-difficulty, wrong-owner, wrong-session, technology-ineligible, condition-failing, zone-incompatible, unbuildable, and TeamType-`Max`-saturated candidates. For base-defense triggers it also checks the global/base-defense enable and House base-defense capacity. `FUN_0041FEE0` owns TeamType zone compatibility, `FUN_00509610` owns the TaskForce/buildability corridor, and `FUN_005095D0` counts matching live Teams for a House and TeamType.

The complete condition-enum truth table and every current-weight feedback branch were not decoded in this bounded investigation. They remain required before implementing the selector. A reduced selector based only on `Autocreate`, `IsBaseDefense`, or random TeamType choice is not evidence-backed.

### 5.3 Empty construction and caps

`FUN_006F09C0(TeamType, House)` resolves the owner fallback, applies signed `TeamType.Max`, allocates `0xA0` bytes, and invokes `TeamClass::Constructor @ 0x006E8A90`. In multiplayer/skirmish the cap is evaluated against the number of live Teams owned by the House and carrying that TeamType.

The constructor:

- inserts the Team into the global Team registry;
- stores TeamType at `+0x24` and House at `+0x2C`;
- constructs its Script object from `TeamType+0xE0`;
- starts with no members at `+0x54`;
- sets the recruiting/status latch `+0x7D=1` and the Phase 3 response latch `+0x83=0`;
- increments `TeamType+0xDC` unconditionally;
- increments House `+0x566C` only when `TeamType.IsBaseDefense` is true.

No TaskForce member is synchronously admitted here. An API that builds a fully populated Team from a caller-supplied candidate list does not match this producer.

## 6. Live Recruitment and Admission

### 6.1 Tick structure

At the start of `TeamClass::AI @ 0x006E9140`, the previously implemented Phase 3 response timer and the `+0x7D` helper run before recruitment. Later in the same tick, while the Team is not in a blocking human/control state, the function walks TaskForce entries in stored slot order. For each slot whose recruited tally at `Team+0x88+slot*4` is below the desired count at `TaskForce+0xA4+slot*8`, it calls `FUN_006EAA90(slot)` once.

Consequences:

- each deficient TaskForce slot gets at most one primary candidate per Team tick;
- different slots may each recruit one candidate in the same tick;
- the new Team remains empty when nothing is eligible;
- an unfilled empty Team is later dissolved using the native `DissolveUnfilledTeamDelay` path rather than being considered complete.

### 6.2 Candidate scan and order

`FUN_006EAA90` derives the recruitment anchor from the Team's current anchor or TeamType waypoint (`FUN_006F18A0`). It derives the required group from `TeamType.Group`, falling back to `TaskForce.Group` when TeamType uses `-1` (`FUN_006F1870`). It resolves the TaskForce TechnoType's runtime category and scans exactly one corresponding global array:

- UnitClass array for runtime categories `1`/`0x28`;
- AircraftClass array for categories `2`/`3`;
- InfantryClass array for categories `0x0F`/`0x10`.

Candidates must match the Team House and exact TechnoType pointer. The scan uses squared distance to the recruitment anchor. A candidate outside the preferred group receives a fixed `0x3200` score penalty when cross-group recruitment is permitted. Replacement occurs only for a strictly smaller score, so global registry order wins exact ties.

Once chosen, the object is told to leave its current activity through vtable `+0x3C8(0)`, then passed to `TeamClass::Add_Member`. The Unit branch may additionally attach linked companion/cargo objects returned by `FUN_00473450`; that attached chain must not be flattened into ordinary independent nearest-candidate picks without further proof.

### 6.3 Admission predicate

`FUN_006EA610(Team, candidate, out_taskforce_slot, forced)` is shared by the recruiter and `TeamClass::Add_Member`. Its proven gates include:

- non-null, alive/active candidate, not already in the receiving Team;
- exact House ownership;
- not in rejected limbo/map state;
- acceptable path/locomotion and aircraft-carrier state;
- TechnoType present in one TaskForce slot unless the call is forced;
- capacity remains for the resolved slot unless forced;
- acceptable current mission/transport/deployment state;
- acceptable recruitability bytes on the object and TeamType;
- no disqualifying existing Team, or an existing TeamType with strictly lower signed priority than the receiving TeamType;
- additional class-specific state gates before success.

Several called helpers in those gates remain unnamed and were not recursively decoded in this coverage-map pass. Their exact truth tables are necessary for an exact live recruiter. The report therefore does not collapse them into an `idle` boolean.

### 6.4 Add order and state

`TeamClass::Add_Member @ 0x006EA500` re-runs the shared predicate, removes the object from any prior Team, increments the matching TaskForce slot tally for non-forced admission, and push-front links the member:

```text
member.TeamNext(+0x5D8) = Team.first_member(+0x54)
Team.first_member(+0x54) = member
member.Team(+0x5D4) = Team
```

All later per-member Team/Script iteration therefore observes reverse recruitment order. The function updates group, aggregate member count/strength, anchor state, the `+0x7D/+0x7E` latches, and the member recruitability byte from `TeamType.AreTeamMembersRecruitable`. Rust candidate input order is not a native ordering authority.

## 7. Retail Data Liveness and Exclusions

The repository retail `ini/aimd.ini` census is:

| Registry/content | Count |
|---|---:|
| `[TaskForces]` identifiers | 132 |
| `[ScriptTypes]` identifiers | 88 |
| `[TeamTypes]` identifiers / sections | 163 |
| `[AITriggerTypes]` records | 165 |
| unique TeamTypes referenced by stock AITrigger records | 153 |
| stock TeamTypes with `Autocreate=yes` | 163 |
| stock TeamTypes with `IsBaseDefense=yes` | 12 |

All 165 stock AITrigger records have 18 tokens. Stock TeamType signed priority distribution is:

| Priority | Count |
|---:|---:|
| 5 | 89 |
| 7 | 46 |
| 10 | 4 |
| 12 | 1 |
| 14 | 2 |
| 15 | 1 |
| 20 | 2 |
| 25 | 6 |
| 30 | 4 |
| 50 | 8 |

Stock `rulesmd.ini` supplies `SuspendPriority=1`. Therefore the specific subbranch that suspends Teams with `Priority < SuspendPriority` has an evidence-backed **stock AIMD exclusion**: no fixed stock TeamType qualifies. It is still reachable through map-authored or map-overridden TeamTypes because map definitions overlay the same registry and signed comparison.

That exclusion does not make the broader Phase 3 base-defense Team response dead. Twelve stock TeamTypes are base-defense Teams, stock AITriggers reference such teams, and their House/Team state is used by the ordinary active responder when the AI base is attacked. Rust's zero production Teams removes that live state entirely. The most frequent ordinary symptom is not “a priority-0 team fails to suspend”; it is “the AI never owns the native base-defense Team objects that the responder can reassign, count, or protect.”

## 8. Current Rust Disparity

Evidence was checked on feature branch `feature/phase3-map-dummy-z` at `9e6c9bc58cd32f33e2d04641ff35cabfecb28342`:

- `src/sim/world/mod.rs` constructs `team_script_vm: TeamScriptVm::default()`.
- `register_script`, `register_task_force`, `register_team_type`, and `create_team_from_type` have no production callers; repository hits outside their definitions are tests/snapshot tests.
- the loading pipeline retains merged Rules and the map INI but does not load or carry `AIMD.INI` as its own fixed AI registry root.
- `src/sim/team_script_vm.rs::create_team_from_type` immediately fills a Team from a caller-provided candidate slice in TaskForce entry order.
- `src/sim/ai.rs::send_attack_wave` independently gathers idle Unit/Infantry objects, sorts by stable ID, takes a fixed wave size, and issues `AttackMove` commands.

The following tempting bridges are rejected:

1. **Register only the 12 base-defense TeamTypes.** This drops the shared fixed registry, AITrigger references, Script/TaskForce attachments, map override behavior, and global order.
2. **Create one base-defense Team at match start.** Native construction is selected on House cadence and bounded by trigger eligibility/capacity; eager precreation changes counts, RNG, timing, and recruitment.
3. **Wrap Rust attack waves in synthetic TeamTypes.** Stable-ID wave selection is not native AITrigger/TaskForce selection and supplies no authoritative TeamType identity, priority, Script, or Max.
4. **Keep immediate candidate admission.** Retail creates empty and recruits incrementally from category registries using distance, group penalty, eligibility, priority preemption, and push-front order.
5. **Use merged Rules INI as AIMD.** Native has a distinct fixed AIMD root and per-registry map overlay sequence.

## 9. Implementation Handoff

### 9.1 Stage A — exact registry ingestion (smallest safe prerequisite)

Implement this first and validate it independently:

1. load `aimd.ini` from the same AssetManager/source resolution used for retail roots;
2. retain the original map INI separately for AI registry overlays;
3. parse in native per-registry order: TeamTypes, ScriptTypes, TaskForces, AITriggerTypes, fixed first then map for each registry;
4. preserve list order and identifier reuse; map sections replace/re-read an existing identifier and may append new identifiers;
5. implement ScriptType 0..49 compacting, TaskForce 0..5 resolved entries and signed counts/Group, TeamType native defaults plus all corridor fields, and lossless 18-token AITrigger DTO storage;
6. resolve TeamType Script/TaskForce pointers during the TeamType pass with the native find-or-allocate helpers, preserve first-reference order, and fill those placeholders in place during the later registry passes;
7. install the parsed definitions into Simulation so a normal map load no longer leaves the Team registry definitions empty;
8. include first-reference order, map re-read and later placeholder fill, valid unfilled-reference persistence, empty-registry sentinel refusal, compaction, six-entry cap, and stock-corpus count tests.

Acceptance evidence for Stage A:

- stock AIMD yields exactly 132 TaskForces, 88 ScriptTypes, 163 TeamTypes, and 165 AITriggerTypes before map additions;
- a map override mutates the existing identifier without changing its registry position;
- a map-added identifier appends in list order;
- a gapped ScriptType compacts;
- a seventh TaskForce row is ignored;
- the 12 stock base-defense TeamTypes and exact priority distribution above are reproduced;
- an ordinary scenario load installs definitions without any test-only registration call.

Stage A is a committed prerequisite, not row closure.

### 9.2 Stage B — exact House selector

Before implementation, close the remaining AITrigger consumer questions: all condition enum bodies, tokens 11–14 semantics, weight feedback/reset, total/base-defense limit offsets, probability gate inputs, buildability/zone helpers, and RNG draw order. Then implement House timer seed/reset, registry-order eligibility, weighted selection, primary/secondary output, and Max/base-defense caps.

Acceptance must assert exact draw count/order and exact selected TeamTypes for controlled fixtures. `Autocreate=yes` alone is not an admissible selector.

### 9.3 Stage C — empty construction and live recruitment

Replace immediate `create_team_from_type` admission with native empty construction and subsequent Team-tick recruitment. Close every `FUN_006EA610` helper predicate and the Unit attached-chain semantics first. Integrate against live simulation Unit/Aircraft/Infantry registries, use native anchor/distance/group scoring and tie order, perform priority preemption, maintain per-slot tallies, and push-front members.

Acceptance must cover empty-first-frame state, next-frame recruitment, one-primary-per-slot-per-tick, strict tie order, group penalty, exact-type/owner rejection, higher-priority preemption, lower/equal-priority refusal, reverse member iteration, and unfilled-Team dissolution.

### 9.4 Stage D — production reachability proof

Run a normal skirmish-style load with no direct Team registration or creation in the test. Advance the House cadence through a controlled eligible AITrigger, observe empty Team creation after the Team pass, observe next-frame live recruitment, damage the AI base, and prove the existing base-defense responder reads/updates that production-created Team. Recheck snapshot/CRC and the earlier response timer/order tests.

Only Stage D closes the critic's production-reachability finding. The stock low-priority suspension branch remains a documented exclusion unless the fixture deliberately supplies a map TeamType below `SuspendPriority`.

## 10. Coverage Ledger

| Mechanism / question | Native owner | Status | Result / required follow-up |
|---|---|---|---|
| active standalone AI file root | `0x0052CD70` | RESOLVED | `AIMD.INI`; no `AI.INI` base layer |
| fixed/map load order | `0x00686B20` | RESOLVED | Team, Script, TaskForce, triggers/tags, AITrigger; fixed then map per registry |
| duplicate identifier behavior | four registry loaders | RESOLVED | re-read existing object in place; new ID appends |
| ScriptType shape and gaps | `0x00691970`, `0x006918A0` | RESOLVED | 50 max, gaps compact, signed pair payload |
| TaskForce shape and gaps | `0x006E8220`, `0x006E8420` | RESOLVED | six max, count then type, unresolved type not counted, Group retained |
| TeamType defaults/attachments | `0x006F06E0`, `0x006F1090` | RESOLVED for corridor | broader non-corridor fields listed but not all consumer-traced |
| AITrigger 18-token storage | `0x0041F580` raw body | PARTIAL | offsets/order resolved; token 12 retained meaning absent, tokens 11–14 semantic labels deliberately open |
| House first-fire and cadence | `0x004F6EC0`, `0x004F8440` | RESOLVED | TeamDelays plus House-index stagger; Team creation follows Team tick |
| selector registry/RNG shape | `0x006F0AB0` | RESOLVED structurally | probability draw, ordered eligibility, weighted draw, primary then secondary |
| complete trigger conditions/weights | `0x0041E720` and callees | OPEN | required before Stage B |
| TeamType Max enforcement | `0x006F09C0`, `0x005095D0` | RESOLVED | House/type live-Team cap on skirmish path |
| empty Team construction | `0x006F09C0`, `0x006E8A90` | RESOLVED | constructor adds empty Team and count/latch state |
| TaskForce slot cadence | `0x006E9140`, `0x006EAA90` | RESOLVED | deficient slots in order, max one primary pick per slot per tick |
| candidate category/score/tie order | `0x006EAA90` | RESOLVED | category array, squared distance, `0x3200` group penalty, registry-order ties |
| complete candidate eligibility | `0x006EA610` and callees | OPEN | several class-state helper meanings must be closed before Stage C |
| member link/order | `0x006EA500` | RESOLVED | push-front, later Team actions reverse recruitment order |
| Unit attached-chain admission | `0x006EAA90`, `0x00473450` | OPEN | required before Stage C Unit parity |
| unfilled Team lifetime | `0x006E9140` | RESOLVED structurally | empty Team waits/recruits, later dissolves; exact prior timer report owns detailed timer semantics |
| stock producer liveness | retail AIMD/rules + runtime callers | RESOLVED | active ordinary skirmish; 165 triggers, 153 referenced TeamTypes, 12 base-defense TeamTypes |
| stock `Priority < 1` branch | retail AIMD/rules | RESOLVED EXCLUSION | zero fixed stock TeamTypes qualify; map override can activate |
| Rust fixed registries | loading + `TeamScriptVm` | CONFIRMED GAP | default/empty in production |
| Rust selector/producer | `sim/ai.rs`, `team_script_vm.rs` | CONFIRMED GAP | generic attack wave is not native producer |
| Rust construction/recruitment shape | `create_team_from_type` | CONFIRMED WRONG | immediate candidate admission must be replaced for production path |
| save/load/hash of future registries | Rust snapshot/hash | OPEN | Stage A/B/C must extend deterministic serialization/hash and recheck prior fixes |
| campaign forced teams | `0x0065DD30` and map triggers | OUT OF SCOPE | not needed for ordinary-skirmish reachability; record as later residual |
| TS legacy | — | EXCLUDED | prohibited by task scope |

## 11. Final Open-Question Log

| ID | Question | Final status |
|---|---|---|
| OQ-01 | Which fixed AI file is active in standalone YR? | RESOLVED — `AIMD.INI` |
| OQ-02 | Is `AI.INI` layered under it? | RESOLVED — no on this active path |
| OQ-03 | What is the fixed/map registry call order? | RESOLVED — exact order in section 3.2 |
| OQ-04 | Do map duplicate IDs append or override? | RESOLVED — object reused/re-read in place |
| OQ-05 | Do ScriptType numbering gaps preserve slot numbers? | RESOLVED — no, they compact |
| OQ-06 | What is the TaskForce maximum and pair order? | RESOLVED — six, count then resolved type |
| OQ-07 | What defaults matter for TeamType selection/recruitment? | RESOLVED for the claimed corridor |
| OQ-08 | Is AITriggerType list-keyed like TeamTypes? | RESOLVED — no, record key is the identifier |
| OQ-09 | Is the stock AITrigger record width stable? | RESOLVED — all 165 have 18 tokens |
| OQ-10 | What does AITrigger token 12 mean? | OPEN — consumed but no retained destination/consumer meaning proven |
| OQ-11 | What are the semantic names of tokens 11–14? | OPEN — storage proven; labels withheld pending consumer trace |
| OQ-12 | Which Rules timer drives ordinary creation? | RESOLVED — `TeamDelays`, with first-fire House-index stagger |
| OQ-13 | Does a House create every `Autocreate=yes` TeamType? | RESOLVED — no, AITrigger eligibility and weighted choice own creation |
| OQ-14 | Is exact selector RNG order known? | PARTIAL — probability then weighted draw shape known; all feedback branches remain open |
| OQ-15 | Are all AITrigger condition enums closed? | OPEN — required before Stage B |
| OQ-16 | Does Team construction synchronously admit TaskForce members? | RESOLVED — no, construction is empty |
| OQ-17 | When can the new Team first recruit? | RESOLVED — next frame because Team pass precedes House creation |
| OQ-18 | What is recruitment slot cadence? | RESOLVED — one primary candidate per deficient slot per tick |
| OQ-19 | What wins a nearest-candidate tie? | RESOLVED — earlier global registry entry |
| OQ-20 | Is candidate input/stable-ID order native? | RESOLVED — no |
| OQ-21 | Are all candidate state gates understood? | OPEN — `FUN_006EA610` helpers require focused closure |
| OQ-22 | What is the Unit linked-companion chain? | OPEN — `FUN_00473450` corridor requires focused closure |
| OQ-23 | Is push-front member order established? | RESOLVED — yes, reverse recruitment order at execution |
| OQ-24 | Are base-defense Teams present in fixed retail AI data? | RESOLVED — 12 TeamTypes, active trigger references |
| OQ-25 | Does fixed retail AIMD exercise `Priority < SuspendPriority(1)`? | RESOLVED EXCLUSION — no; map data can |
| OQ-26 | Can Rust's generic attack wave be reused as the Team producer? | RESOLVED — no exact identity/selection/admission correspondence |
| OQ-27 | What is the smallest non-approximate implementation step? | RESOLVED — Stage A exact registry ingestion |
| OQ-28 | Does Stage A close production reachability? | RESOLVED — no; Stages B–D remain mandatory |
| OQ-29 | Must save/load/hash coverage expand with these registries and live states? | OPEN IMPLEMENTATION DUTY — yes, each stage must prove it |
| OQ-30 | Are campaign forced/reinforcement Team creators required here? | RESOLVED OUT OF SCOPE — ordinary-skirmish producer is independently active |

## 12. Evidence-Backed Stop Condition

This coverage-map investigation closes the boundary needed to begin the loader prerequisite. It does not authorize a production Team shortcut. Production reachability remains open until all OPEN entries that feed Stages B and C are resolved, the implementation follows the native four-stage path, prior Phase 3 timer/order fixes are rechecked, and the no-test-registration end-to-end proof passes.
