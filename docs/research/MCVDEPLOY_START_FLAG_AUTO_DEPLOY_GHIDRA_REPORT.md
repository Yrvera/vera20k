# MCVDeploy Start Flag Auto-Deploy -- Ghidra Research Report

**Address(es):** `0x006886B0` (Generate_Random_Units), `0x004FC060` (House helper), `0x00740DF0` / `0x00740E20` (deploy queue helpers), `0x007393C0` (UnitClass deploy body)  
**Investigation Mode:** exhaustive-slice, downgraded to partial because this subagent had no live Ghidra MCP tools exposed  
**Claimed Scope:** multiplayer/skirmish start `MCVDeploy` flag behavior from initial MCV creation through queued deploy mission entry  
**Non-Scope:** deploy-facing, GACNST FreeUnit, ConYard special branch, full starting-unit random pool, and redeploy UI gates  
**Confidence:** Medium-High from prior archived Ghidra reports; no new live decompiler spot-check was possible in this slot  
**Active in YR:** Yes, for standard skirmish/multiplayer startup when `Bases=yes` and the active `[SpecialFlags] MCVDeploy` flag is set

## 1. Overview

`MCVDeploy` is a start-of-match automation flag. It is checked after each starting MCV has been created and successfully placed, before the rest of the starting-unit budget loop runs. The helper does not directly create `GACNST`; it assigns the MCV as the house primary object and queues the normal Deploy mission, which later enters the same `UnitClass::Deploy @ 0x007393C0` path used by ordinary MCV deploy.

This means the startup flag bypasses player input/command timing, but not the deploy mission body. Footprint validation, facing requirements, success conversion, and failure behavior remain owned by the standard deploy path.

## 2. Class Layout / Key Offsets

| Offset / global | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `DAT_00A8B258` | byte | `Bases` lobby/session option; gates initial MCV creation | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, `MCV_DEPLOY_GHIDRA_REPORT.md` | Yes |
| `DAT_00A8B230` / `ScenarioClass+0x00` | u32 flags | active gameplay `SpecialFlags` word | `SPECIAL_FLAGS_SYSTEM.md`, `GAME_START_INITIALIZATION.md` | Yes |
| bit 8 / `0x0100` | flag | `[SpecialFlags] MCVDeploy` active bit in final SpecialFlags word | `SPECIAL_FLAGS_SYSTEM.md`, `GAME_START_INITIALIZATION.md`, `MCV_DEPLOY_GHIDRA_REPORT.md` | Yes |
| `HouseClass+0x53DC` | pointer | house primary factory / primary MCV pointer updated by `0x004FC060` | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | Yes |
| `HouseClass+0x53E0` | cell/sentinel | primary factory cell cleared before auto-deploy helper call | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | Yes |
| `UnitClass+0x81` | bool | limbo guard in `0x004FC060`; limbo units are rejected | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | Yes |
| `UnitClass[0x1B3]` | mission/deploy target field | set by `0x00740DF0` only if currently `-1` | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | Yes |

## 3. Core Logic

Startup generation path, scoped to the MCVDeploy slice:

1. `Generate_Random_Units @ 0x006886B0` gathers start positions and iterates houses.
2. If `Bases=yes` (`DAT_00A8B258 != 0`), it selects a side-appropriate `BaseUnit` from `RulesClass+0xB20` and constructs a `UnitClass` MCV.
3. It converts the start cell to centered leptons (`cell * 256 + 128`) and calls the object's place vtable slot `+0xD8`.
4. If direct placement fails, it tries the documented spiral placement helper `0x00688ED0`; total failure deletes the MCV and skips to starting units.
5. Only after successful placement does it clear `house+0x53DC` and `house+0x53E0`.
6. If active `MCVDeploy` is set, it calls `FUN_004FC060(house, mcv, 1)`.
7. `FUN_004FC060` returns `0` if the unit pointer is null or `unit+0x81` says the unit is in limbo.
8. Otherwise it clears the prior primary/deploy assignment via `FUN_004FBE40`, queues deploy on the MCV via `FUN_00740DF0`, writes `house+0x53DC = mcv`, and returns `1`.
9. `FUN_00740DF0` queues mission `MISSION_DEPLOY` through the unit vtable only if the deploy target field is still `-1`; the archived report says it writes `unit[0x1B3]` first, then calls the mission queue vtable with mission `2`.
10. The rest of starting-unit generation runs after this queueing step.

The key timing fact is that `0x004FC060` queues the Deploy mission; it does not call `UnitClass::Deploy @ 0x007393C0` directly in the archived pseudocode. The existing deep dive states the MCV processes the Deploy mission on the next sim tick.

## 4. INI Keys

| Key | Location | Default / stock value | Effect in this slice | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `MCVDeploy` | map `[SpecialFlags]` | default `0` in reset; can be set by scenario/session | Gates startup auto-queue of MCV Deploy mission | `SPECIAL_FLAGS_SYSTEM.md`; `GAME_START_INITIALIZATION.md` | Yes |
| `Bases` | `[MultiplayerDialogSettings]` / session byte | stock default enabled in current Rust defaults; binary byte `DAT_00A8B258` | If off, no starting MCV is created and `MCVDeploy` has no MCV to affect | `MCV_DEPLOY_GHIDRA_REPORT.md`; `src/sim/game_options.rs` | Yes |
| `BaseUnit` | `[General]` | `AMCV,SMCV,PCV` in `rulesmd.ini` | Provides side-filtered starting MCV type | `ini/rulesmd.ini:390`; `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | Yes |
| `[AMCV] DeploysInto` | `rulesmd.ini` | `GACNST` | Standard deploy mission target, not startup helper-specific | `ini/rulesmd.ini:6977`; prior AMCV reports | Yes |

## 5. Integration Points

`Generate_Random_Units @ 0x006886B0` runs during post-map initialization for skirmish/offline multiplayer startup. Prior reports place it after map/house setup and before starting credits/final map initialization. Within the function, the auto-deploy check is after initial MCV placement and before the random starting-unit budget loop.

The deploy helper path is:

`0x006886B0` -> `0x004FC060` -> `0x004FBE40` / `0x00740DF0` -> queued `MISSION_DEPLOY` -> ordinary mission processing -> `UnitClass::Deploy @ 0x007393C0`.

Player-visible implication: the player starts with an MCV already in its deploy mission when the match begins if the flag is set. The player does not need to issue the deploy command, but the MCV still has to satisfy normal deploy-facing and placement gates before the ConYard exists.

## 6. Current Rust Implementation Status

Rust currently seeds skirmish MCVs in `src/app_skirmish.rs::seed_skirmish_opening_if_needed`. It spawns MCVs at multiplayer start waypoints and sets credits/base center, but it has no `MCVDeploy` setting, no `[SpecialFlags] MCVDeploy` parser field, and no startup call that queues or applies deploy.

Rust player/AI deploy uses `Command::DeployMcv`, handled in `src/sim/world/world_commands.rs`, which immediately calls `src/sim/world/world_spawn.rs::deploy_mcv`. That function despawns the MCV and spawns the construction yard immediately after current placement checks. It does not model a queued `MISSION_DEPLOY` layer for MCVs.

Rust AI has `src/sim/ai.rs::try_deploy_mcv`, which issues `Command::DeployMcv` when an AI has an undeployed `DeploysInto=` unit and no deployed MCV flag. That is AI logic after match start, not the binary's `MCVDeploy` start flag.

`src/map/basic.rs::SpecialFlagsSection` currently parses only `TiberiumGrows`, `TiberiumSpreads`, and `DestroyableBridges`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Runtime `MCVDeploy` bit | verified-from-prior-docs | `SPECIAL_FLAGS_SYSTEM.md`, `GAME_START_INITIALIZATION.md`, `MCV_DEPLOY_GHIDRA_REPORT.md` | fresh live Ghidra spot-check unavailable |
| `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` bit-4 claim | conflict-needs-doc-patch | conflicts with `SPECIAL_FLAGS_SYSTEM.md` and `GAME_START_INITIALIZATION.md` | patch stale doc in a non-swarm edit pass |
| `Generate_Random_Units @ 0x006886B0` placement-before-helper order | verified-from-prior-docs | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, `MCV_DEPLOY_GHIDRA_REPORT.md` | fresh live Ghidra spot-check unavailable |
| `FUN_004FC060` null/limbo guard and primary assignment | verified-from-prior-docs | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | exact assembly not re-read in this slot |
| `FUN_00740DF0` queued deploy mission | verified-from-prior-docs | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | exact concrete vtable slot owner not re-read |
| Entry into `UnitClass::Deploy @ 0x007393C0` | verified-from-prior-docs | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, AMCV deploy reports | precise tick scheduler callsite not re-opened |
| Rust startup MCV seeding | verified | `src/app_skirmish.rs` scan | none |
| Rust `deploy_mcv` immediate conversion | verified | `src/sim/world/world_spawn.rs` scan | none |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Is this exhaustive or a coverage map? -> Intended exhaustive-slice, but final status is partial because no live Ghidra MCP tools were exposed.` (evidence: tool availability in this session)
- `[RESOLVED] OQ-02 -- Which startup function owns starting MCV creation? -> `Generate_Random_Units @ 0x006886B0`.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`; `MCV_DEPLOY_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-03 -- Is the active `MCVDeploy` flag bit `0x10` or `0x100`? -> Treat active `[SpecialFlags] MCVDeploy` as bit 8 / `0x0100`; the deep-dive bit-4 wording is stale/conflicted.` (evidence: `SPECIAL_FLAGS_SYSTEM.md`; `GAME_START_INITIALIZATION.md`; `MCV_DEPLOY_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-04 -- Does `MCVDeploy` run before or after initial MCV placement? -> After MCV construction and successful place/spiral placement.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`)
- `[RESOLVED] OQ-05 -- Does `MCVDeploy` run before or after remaining starting units? -> Before the random starting-unit budget loop.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`)
- `[RESOLVED] OQ-06 -- Does `FUN_004FC060` directly spawn a ConYard? -> No; archived decompilation shows it queues Deploy mission via `0x00740DF0`.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`)
- `[RESOLVED] OQ-07 -- Does the path bypass normal player command timing? -> Yes for input/command issuance: it is called during startup generation without player command.` (evidence: `0x006886B0` flow in prior docs)
- `[RESOLVED] OQ-08 -- Does the path bypass normal `UnitClass::Deploy` behavior? -> No; it enters the standard Deploy mission path and later `UnitClass::Deploy @ 0x007393C0`.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`; AMCV deploy reports)
- `[RESOLVED] OQ-09 -- What happens if the helper receives a null/limbo unit? -> It returns `0` and queues nothing.` (evidence: `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`)
- `[RESOLVED] OQ-10 -- Is this active in stock YR skirmish/multiplayer? -> Conditional yes: `Bases=yes` creates the starting MCV and `MCVDeploy` set queues deploy.` (evidence: `MCV_DEPLOY_GHIDRA_REPORT.md`; `SPECIAL_FLAGS_SYSTEM.md`)
- `[RESOLVED] OQ-11 -- Does current Rust parse the start flag? -> No; `SpecialFlagsSection` lacks `mcv_deploy`.` (evidence: `src/map/basic.rs`)
- `[RESOLVED] OQ-12 -- Does current Rust startup seed auto-deploy? -> No; `seed_skirmish_opening_if_needed` only spawns MCVs and sets house state.` (evidence: `src/app_skirmish.rs`)
- `[RESOLVED] OQ-13 -- Does current Rust MCV deploy use a mission queue? -> No; `Command::DeployMcv` immediately calls `deploy_mcv`.` (evidence: `src/sim/world/world_commands.rs`; `src/sim/world/world_spawn.rs`)
- `[DEFERRED] OQ-14 -- Exact instruction bytes for the final `0x006886B0` flag test in the current Ghidra database.` (category: requires-different-system-context; reason: no Ghidra MCP exposed to this slot; next-step-if-pursued: re-open `0x006886B0` and verify the mask directly)
- `[DEFERRED] OQ-15 -- Exact tick scheduler function that consumes the queued Deploy mission after `0x004FC060`.` (category: requires-different-system-context; reason: mission scheduler was outside this slice and not freshly available; next-step-if-pursued: targeted MissionClass/UnitClass mission dispatch trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Startup `MCVDeploy` is active bit 8 / `0x0100` in the final `SpecialFlags` word, not bit 4 / `0x10` | `SPECIAL_FLAGS_SYSTEM.md`; `GAME_START_INITIALIZATION.md`; `MCV_DEPLOY_GHIDRA_REPORT.md` | missing | `src/map/basic.rs`, `src/sim/game_options.rs`, skirmish startup settings plumbing | represent `MCVDeploy` separately from `mcv_redeploy`; drive startup auto-deploy from the final map/session flag | skirmish map with `MCVDeploy=yes` starts with each starting MCV queued to deploy | Do not reuse `mcv_redeploy`; that is ConYard -> MCV undeploy permission |
| Auto-deploy is invoked only after the starting MCV is constructed and successfully placed/fallback-placed, and before starting units are generated | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | missing | `src/app_skirmish.rs::seed_skirmish_opening_if_needed` or a future game-start generator | after spawning each MCV, if `Bases=yes && MCVDeploy`, queue deploy before non-MCV starting units are added | player and AI MCVs at valid start cells enter deploy path before extra units exist | Do not spawn a construction yard directly instead of creating/placing an MCV first |
| `FUN_004FC060` queues Deploy mission through `0x00740DF0`; it does not directly call the conversion body | `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md` | missing/mismatch | MCV deploy mission/state surface, currently `src/sim/world/world_commands.rs` and `src/sim/world/world_spawn.rs::deploy_mcv` | startup auto-deploy should enqueue the same standard deploy behavior that player deploy uses; if a mission queue is not yet modeled, preserve one-tick/standard-path semantics in tests | immediately after game-start seeding the AMCV still exists with deploy intent; after mission processing it becomes GACNST subject to normal facing/placement gates | Do not make `MCVDeploy` an unconditional instant `spawn_object_at_height("GACNST")` shortcut |

### Stale Docs / Follow-up Docs

- `docs/research/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, section 7: replace the table row `| 4 | 0x10 | MCVDeploy | Yes -- forces MCV auto-deploy at game start |` with `| 8 | 0x0100 | MCVDeploy | Yes -- when set in the active SpecialFlags word, starting MCVs queue the standard Deploy mission after successful placement |`.
- Same section: replace the note beginning `the runtime bitfield accessed by Generate_Random_Units, the check is & 0x10` with `Earlier notes confused the bit-4/session flag path with `[SpecialFlags] MCVDeploy`. Current cross-doc evidence treats MCVDeploy as SpecialFlags bit 8 / 0x0100; re-check `0x006886B0` in live Ghidra before editing lower-level mask prose beyond this correction.`
- Same report confidence table: replace `MCVDeploy bit position (0x10 vs >>8) | MEDIUM | Discrepancy between runtime and serialization` with `MCVDeploy bit position | MEDIUM pending fresh live re-check of `0x006886B0`; archived SpecialFlags and startup reports agree on bit 8 / 0x0100, while this report's older bit-4 prose is stale.`

## Sources

- `docs/research/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`
- `docs/research/SPECIAL_FLAGS_SYSTEM.md`
- `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md`
- `docs/research/GAME_START_INITIALIZATION.md`
- `docs/research/LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `src/app_skirmish.rs`
- `src/map/basic.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/ai.rs`
