# MCV Redeploy UI Command Gate - Ghidra Research Report

**Address(es):** `0x00449BC0` (`BuildingClass__CanUndeployMCV`), `0x0044F5C0` (`BuildingClass__ShouldShowDeployButton`), `0x004555D0` (`BuildingClass__CanSellOrUndeploy`)
**Investigation Mode:** exhaustive-slice, downgraded to evidence-constrained slice because this subagent session had no callable Ghidra MCP
**Claimed Scope:** player-visible ability to expose and issue ConYard/GACNST redeploy before the reverse-conversion state machine completes
**Non-Scope:** refund-on-failure, successful state transfer, AMCV deploy-to-GACNST completion, slave miner reverse conversion internals
**Confidence:** High where prior read-only Ghidra reports agree on the same addresses; Medium for exact sidebar/event-dispatch feedback because no fresh live decompile was available in this slot
**Active in YR:** Conditional. Active for standard YR ConYards with `UndeploysInto`, but the ConYard-specific acceptance chain requires MP/skirmish-style game mode, human-controlled owner, `MCVRedeploys` enabled, and no power-link field.

## 1. Overview

ConYard redeploy is gated before the sell/undeploy mission body. The player normally sees no redeploy button while the UI visibility predicate rejects the building; if a deploy/undeploy command reaches the runtime gate anyway, `BuildingClass__CanUndeployMCV @ 0x00449BC0` returns false and the command does not proceed to the MCV repack path.

The important split is that `UndeploysInto=AMCV` is necessary but not sufficient for ConYards. Non-ConYard `UndeploysInto` buildings bypass the MP/`MCVRedeploys` chain, while `ConstructionYard=yes` buildings enter the stricter MCV redeploy gate.

## 2. Key Fields And Globals

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `TechnoTypeClass+0x408` | `UndeploysInto` target type pointer | `0x00449BC0`, `0x0044F5C0`, `TechnoTypeClass__ReadINI`; prior YAREFN/MCV reports | Yes; `[GACNST] UndeploysInto=AMCV` |
| `BuildingTypeClass+0x16B9` | `ConstructionYard=yes`; enters ConYard-only redeploy restrictions | `0x00449BC0`, `0x0044F5C0`; prior AMCV/GACNST reports | Yes; `[GACNST] ConstructionYard=yes` |
| `DAT_00A8B238` | nonzero MP/skirmish-style game mode gate in prior reports | `MCV_DEPLOY_GHIDRA_REPORT.md`, `0x00449BC0` | Conditional; SP/campaign-style path rejects ConYard redeploy |
| `DAT_00A8B320` | MCV Repacks / `MCVRedeploys` checkbox | `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`, `0x006AD7E4..0x006AD7F9`, `0x00449BC0` | Yes when game option enabled; stock default `yes` |
| `BuildingClass+0x2C0` | link field checked as a hard redeploy blocker for ConYards | `0x00449BC0`, `0x0044F5C0`; prior MCV/skirmish option reports | Conditional; if nonzero, command hidden/rejected |
| `BuildingClass+0x504` | production-busy count used by `ShouldShowDeployButton` | `YAREFN_UNDEPLOY_TO_SMIN_SLAVEMANAGER_PATH_GHIDRA_REPORT.md`, `0x0044F5C0` | Yes for UI visibility; stale broad MCV report used older `entity[0x141]` wording |

## 3. Core Gate Logic

### UI visibility: `BuildingClass__ShouldShowDeployButton @ 0x0044F5C0`

Verified by prior read-only Ghidra reports:

1. If production count/busy field `+0x504 > 0`, return false.
2. If the object is already in the deployed state handled by this helper, return true.
3. If the type is a ConYard (`Type+0x16B9 != 0`), require human-controlled owner, MP/skirmish-style mode, `MCVRedeploys`, and `Building+0x2C0 == 0`.
4. Return whether `Type+0x408 UndeploysInto` is non-null.

Player-visible result: for the normal local player, unavailable ConYard redeploy is represented primarily by the deploy/redeploy command not being offered. The reports did not verify a separate EVA/message at this visibility stage.

### Runtime acceptance: `BuildingClass__CanUndeployMCV @ 0x00449BC0`

Verified by prior read-only Ghidra reports:

1. If `Type+0x408 == 0`, return false.
2. If `Type+0x16B9 == 0`, return true immediately. This is why YAREFN/SMIN reverse conversion is not gated by `MCVRedeploys`.
3. For ConYards, require MP/skirmish-style mode, an owner, player/human-control, `DAT_00A8B320 != 0`, and `Building+0x2C0 == 0`.
4. Production-busy was verified at the UI predicate; the current available reports do not show `0x00449BC0` re-checking `+0x504`.

Player-visible result: if a command reaches this function while one of these runtime gates is false, the acceptance predicate returns false and the building does not enter the reverse MCV conversion path. No verified report found an EVA "cannot deploy" style message for this ConYard redeploy gate; absence of feedback beyond no command/no action remains medium confidence because the event-dispatch caller was not freshly decompiled here.

### Generic sell/undeploy legality: `BuildingClass__CanSellOrUndeploy @ 0x004555D0`

Prior YAREFN research touched this as a generic building command legality gate: health, timer/mission, EMP/offline-style restrictions, and factory-like powered checks may participate before sell/undeploy infrastructure proceeds. This report does not claim full coverage of those generic conditions; it only records that they are separate from the MCV-specific `MCVRedeploys` gate.

## 4. INI Keys

| Section/key | Stock YR value | Effect in this slice | Evidence |
|---|---:|---|---|
| `[MultiplayerDialogSettings] MCVRedeploys` | `yes` | default for the MCV Repacks checkbox/global; runtime reads `DAT_00A8B320` | `ini/rulesmd.ini:3041`; `0x006AD7E4..0x006AD7F9`; `0x00449BC0` |
| `[GACNST] ConstructionYard` | `yes` | causes ConYard-only SP/MP/player/option/link gates | `ini/rulesmd.ini:11625`; `0x00449BC0` |
| `[GACNST] UndeploysInto` | `AMCV` | necessary target pointer for redeploy | `ini/rulesmd.ini:11631`; `0x00449BC0`, `0x0044F5C0` |
| `[AMCV] DeploysInto` | `GACNST` | confirms bidirectional stock pair, but forward deploy is out of scope | `ini/rulesmd.ini:6977`; prior AMCV reports |

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Skirmish option packing | checkbox control `0x693` writes `DAT_00A8B320`; mirror `DAT_00A8B3DB` | `0x006AD7E4..0x006AD7F9` in skirmish option report | Yes for skirmish/MP setup |
| Button visibility | production-busy first; ConYard then checks human/MP/MCVRedeploys/link before `UndeploysInto` | `0x0044F5C0` | Yes |
| Runtime command acceptance | `UndeploysInto` required; non-ConYard bypass; ConYard checks MP/owner/human/MCVRedeploys/link | `0x00449BC0` | Yes, conditional |
| Reverse mission body | later `BuildingClass__Sell @ 0x00449C30` handles MCV creation/refund/transfer | prior BuildingClass/MCV reports | Out of scope here except as downstream consumer |

## 6. Current Rust Implementation Status

Current Rust exposes and issues `Command::UndeployBuilding` from object-click/self-click when the selected structure's object type has `undeploys_into`: `src/app_context_order.rs:194..197` and `src/app_context_order.rs:459..462`.

Execution validates ownership and special-cases slave miner, then calls `undeploy_building`: `src/sim/world/world_commands.rs:492..505`. The generic path only checks structure category, `building_up`, `building_down`, and rules target before starting `BuildingDown`: `src/sim/world/world_spawn.rs:590..625`.

`GameOptions` has `mcv_redeploy` with default `true` at `src/sim/game_options.rs:25..65`, but the scanned command/UI path does not consume it for ConYard redeploy. The scanned Rust path also does not model the SP/MP gate, owner human-control gate, production-busy button hide, or `Building+0x2C0` link blocker for this command.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__CanUndeployMCV @ 0x00449BC0` | verified from prior read-only reports | MCV/YAREFN/skirmish option reports | fresh live decompile unavailable in this subagent |
| `BuildingClass__ShouldShowDeployButton @ 0x0044F5C0` | verified from prior read-only reports | YAREFN report, broad MCV report | exact sidebar event caller not re-traced |
| `BuildingClass__CanSellOrUndeploy @ 0x004555D0` | touched-not-exhausted | YAREFN report | full generic command gate matrix belongs to sell/repair slice |
| `MCVRedeploys` option packing | verified | skirmish packed option report; `rulesmd.ini:3041` | none for this consumer |
| Production-busy UI gate | verified for `+0x504` | YAREFN report `0x0044F5C0` | older `entity[0x141]` wording should be treated stale |
| Event/network dispatch feedback | touched-not-exhausted | broad MCV report notes network deploy event; no fresh caller decompile | exact sound/EVA/no-op result needs live dispatcher trace |
| Rust UI command issue | verified by source scan | `src/app_context_order.rs:194`, `:459` | none |
| Rust command execution | verified by source scan | `src/sim/world/world_commands.rs:492`, `world_spawn.rs:590` | none |

## 8. Open Questions - Final State

- [RESOLVED] OQ-1 - Does `UndeploysInto` alone expose ConYard redeploy? -> No for gamemd ConYards; it is necessary, but `ConstructionYard=yes` adds MP/human/MCVRedeploys/link gates. (evidence: `0x00449BC0`, `0x0044F5C0`, `rulesmd.ini:11625/11631`)
- [RESOLVED] OQ-2 - Does `MCVRedeploys` affect non-ConYard `UndeploysInto` buildings? -> No; `0x00449BC0` returns true before the option chain when `Type+0x16B9 == 0`. (evidence: `0x00449BC0`, YAREFN report)
- [RESOLVED] OQ-3 - Where is the UI production-busy gate? -> `0x0044F5C0` returns false when `BuildingClass+0x504 > 0`. (evidence: YAREFN report `0x0044F5C0`)
- [RESOLVED] OQ-4 - Is stock YR default `MCVRedeploys` enabled? -> Yes, `[MultiplayerDialogSettings] MCVRedeploys=yes`, packed into `DAT_00A8B320` by the skirmish start path. (evidence: `rulesmd.ini:3041`, `0x006AD7E4..0x006AD7F9`)
- [RESOLVED] OQ-5 - Does current Rust consume `GameOptions::mcv_redeploy` in the ConYard command path? -> No matching use found; command path keys off `undeploys_into`. (evidence: `rg mcv_redeploy`, `src/app_context_order.rs:194`, `src/sim/world/world_spawn.rs:590`)
- [DEFERRED] OQ-6 - What exact user feedback fires if a stale/networked command reaches `0x00449BC0` and returns false? (category: needs-runtime-debugger; reason: prior reports verify false return/no conversion, but not a fresh caller-side EVA/sound/no-op branch; next-step-if-pursued: decompile deploy command dispatcher and UI click handler around the call to `0x00449BC0`)
- [DEFERRED] OQ-7 - What exact semantic name should be assigned to `Building+0x2C0` in this gate? (category: requires-different-system-context; reason: prior AMCV report calls it attached/power-link and this target only needs the nonzero blocker; next-step-if-pursued: offset-specific owner/link field audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| ConYard redeploy is hidden/rejected unless MP/skirmish-style mode, human owner, `MCVRedeploys`, and link field clear all pass | `0x00449BC0`, `0x0044F5C0`, `rulesmd.ini:3041/11625/11631` | missing | `src/app_context_order.rs`, command availability/cursor surface, `src/sim/world/world_commands.rs` | gate both command exposure and command execution for `ConstructionYard=yes` buildings, not every `UndeploysInto` building | With `mcv_redeploy=false`, selected GACNST cannot be ordered to pack, while YAREFN can still undeploy to SMIN | Do not globally block all `UndeploysInto` when MCV repacks is off |
| Production-busy ConYard should not expose the redeploy button | `0x0044F5C0` with `+0x504 > 0` | missing/unclear | UI command button/cursor availability and production/factory state | hide/disable redeploy while the building production count/queue is nonempty | GACNST producing a building has no pack-up action; after queue clears, action appears if other gates pass | Do not put this only in the late state-machine completion path; the player-visible button gate matters |
| Runtime acceptance repeats core ConYard gates and returns false before conversion | `0x00449BC0` | missing | `src/sim/world/world_commands.rs::Command::UndeployBuilding`, `world_spawn.rs::undeploy_building` or helper predicate | reject stale/desynced commands without starting `BuildingDown` | A queued undeploy command submitted before `mcv_redeploy` is disabled or link becomes nonzero does not start reverse buildup | Do not rely on UI gating alone in lockstep/deterministic command execution |

## 10. Negative Facts

- `MCVRedeploys` is not a universal `UndeploysInto` gate; non-ConYard buildings return true before that check in `0x00449BC0`.
- The current evidence does not support treating `entity[0x141]` as the production queue gate for button visibility; the sharper report says `BuildingClass+0x504 > 0`.
- The verified gate does not hardcode `GACNST` by ID; it uses `ConstructionYard=yes` plus `UndeploysInto`.
- The verified pre-command gate does not cover successful MCV health/refund/state transfer; those are downstream `0x00449C30` concerns and out of scope here.
- No verified report found an `EVA_CannotDeployHere`-style feedback call for unavailable ConYard redeploy; current evidence is hidden command or false acceptance/no conversion.

## Stale Docs / Follow-up Docs

- In `MCV_DEPLOY_GHIDRA_REPORT.md`, replace the production queue row "Production queue empty | `entity[0x141] <= 0` | Cannot undeploy while producing" with: "Redeploy button visibility returns false when `BuildingClass+0x504 > 0` in `BuildingClass__ShouldShowDeployButton @ 0x0044F5C0`; this is a UI exposure gate. Current narrower reports did not confirm the older `entity[0x141]` wording."
- In broad gate tables, avoid wording that implies all `UndeploysInto` buildings require MP/`MCVRedeploys`; the ConYard-only chain is entered only after `Type+0x16B9 != 0`.

## Sources

- Prior read-only Ghidra reports: `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md`, `docs/research/YAREFN_UNDEPLOY_TO_SMIN_SLAVEMANAGER_PATH_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`, `docs/research/AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/app_context_order.rs`, `src/sim/world/world_commands.rs`, `src/sim/world/world_spawn.rs`, `src/sim/game_options.rs`.
- Tool limitation: no callable Ghidra MCP or local Ghidra runner was available in this subagent session; no Ghidra mutations were made.
