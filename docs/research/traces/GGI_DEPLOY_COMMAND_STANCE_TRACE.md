# Guardian GI Deploy Command And Stance Trace

Scenario: rookie Guardian GI (`GGI`) at cell `(50,50)`, selected, ordered to deploy under standard Yuri's Revenge rules.

Scope: player cursor/action, accepted deploy command, deploy sound, deploy sequence timing, final deployed stance, and screen-visible result. Adjacent behavior is listed separately and was not traced as part of this run.

## Sources

- `docs/research/units/allied/GGI.md`
- `docs/research/GGI_GHIDRA_REPORT.md`
- `docs/research/GI_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `ini/soundmd.ini`
- `src/app_cursor.rs`
- `src/app_context_order.rs`
- `src/app_input.rs`
- `src/sim/deploy.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/world/mod.rs`
- `src/sim/animation.rs`
- `src/rules/infantry_sequence.rs`
- `src/app_instances/shp.rs`

No live Ghidra mutation was performed. Existing research reports already contain the needed binary references for this concrete scenario.

## Pipeline

Selected GGI at `(50,50)` -> self-hover cursor resolves to Deploy -> self-click / deploy command queues a deploy toggle -> command executes after the local command delay -> sim enters Deploying and emits `GuardianGIDeploy` -> deploy state advances -> animation renders `Deploy` frames -> sim promotes to Deployed -> animation renders `Deployed` stance.

## Stage Table

| Stage | Boundary Output | Our Output | gamemd Output | Verdict |
|---|---:|---:|---:|---|
| 1. INI data | GGI deploy-capable and sound-bearing | `Deployer=yes`, `DeployFire=yes`, `DeploySound=GuardianGIDeploy`, `Deploy=300,15,0`, `Deployed=315,1,1` | Same values in standard YR data; GGI doc says behavior is value-driven, no GGI-specific binary branch | PASS |
| 2. Cursor on selected self | Deploy cursor shown | `CursorFeedbackKind::Deploy` -> `CursorId::Deploy`, frames `110..118`, count `9` | GI/GGI report: self action is deploy; cursor-action path reports Deploy for own-cell/self | PASS |
| 3. Player command accepted | Deploy request accepted | Self-click on selected infantry with `deploy_fire=true` queues `Command::ToggleInfantryDeploy`; accepted = `1` | Player self-click / D key queues mission `0x10`; `FUN_0051F6E0` accepts when `type+0xEC8 != 0`; accepted = `1` for GGI | PASS |
| 4. Command execution delay | Ticks from player order to deploy-state mutation | Default local command delay is `2` sim ticks: command queued at `T+2`, executes on the fixed step whose threshold reaches `T+2` | Existing GI report says player path queues mission `0x10`; next AI tick dispatches `FUN_0051F6E0` | FAIL |
| 5. Deploy phase entry and sound ordering | Sound and state mutation order | On command execution, code emits `EntityDeployed { GuardianGIDeploy, 50,50 }` before assigning `deploy_state=Deploying { ticks_remaining: 54 }` | `InfantryClass::Do_Action(0x1B)` plays `TechnoType+0x56C` before writing Doing `0x1B`; sound is `GuardianGIDeploy` | PASS |
| 6. Deploy sequence duration | Numeric transition point to deployed stance | `Deploy=15` art frames -> `frames_to_ticks(15)=54`; same tick decrements to `53`; promotion occurs by the `DeployPhase` countdown, not by frame index `15` | `DoType_Sequencer` uses `current_frame < Length`; for `Length=15`, frames `0..14` play and transition fires when frame equals `15` | FAIL |
| 7. Final deployed logical stance | Final stance ID | `DeployPhase::Deployed` after countdown | Doing `0x1C` (`Deployed`) after sequence case `0x1B -> 0x1C` | PASS |
| 8. Final screen-visible frame | Exact displayed SHP frame | Deployed animation uses `[GuardianGISequence] Deployed=315,1,1`; exact frame depends on current facing | gamemd uses same sequence entry; exact frame depends on current facing | UNCHECKED |

## Failures

### Stage 4 - Command Execution Delay

Player-visible difference: deploy starts later in VERA20k than the documented gamemd path. Our app schedules the context order at `sim.tick + sim.input_delay_ticks`, and the default input delay is `2`. gamemd's documented player path queues mission `0x10` and dispatches it on the next AI tick.

Our evidence:

- `src/app_context_order.rs:61` computes `execute_tick = sim.tick.saturating_add(sim.input_delay_ticks)`.
- `src/sim/world/mod.rs:430` defaults `input_delay_ticks` to `2`.
- `src/sim/world/mod.rs:456` drains due commands only when `cmd.execute_tick <= self.tick + 1`.

gamemd evidence:

- `GI_GHIDRA_REPORT.md` documents self-click / D key -> mission `0x10` -> next AI tick dispatch -> `FUN_0051F6E0` -> `Do_Action(0x1B)`.

### Stage 6 - Deploy Sequence Timing

Player-visible difference: the final deployed stance is driven by our countdown conversion, not by gamemd's sequence frame counter. The numbers do not match: our deploy transition countdown is `54` sim ticks from the 15-frame art entry, while gamemd transitions when the sequence frame counter reaches `15`.

Our evidence:

- `src/sim/deploy.rs:57` computes `frames_to_ticks(15) = 15 * 80 / 22 = 54`.
- `src/sim/world/world_commands.rs:515` uses that conversion for the deploy phase.
- `src/sim/world/mod.rs:1226` calls `tick_deploy_state` in the same tick as command dispatch, so the freshly-created `54` immediately becomes `53`.
- `src/sim/animation.rs:442` reflects deploy state into `SequenceKind::Deploy`; when the state promotes to `Deployed`, it switches to `SequenceKind::Deployed`.

gamemd evidence:

- `GGI_GHIDRA_REPORT.md` documents `DoType_Sequencer @ 0x00520ae0`: end test is `current_frame < Length`; for GGI `Deploy=300,15,0`, frames `0..14` play and case `0x1B` transitions to `0x1C` when the frame equals `15`.

## Not Implemented

None in the traced deploy command/stance path. Adjacent deploy-command voice acknowledgement is not included in this verdict because this run scoped `DeploySound=GuardianGIDeploy`, not `VoiceSpecialAttack`.

## Unchecked

- Exact final pixel/frame for the deployed GGI, because the scenario did not specify starting facing. For a complete pixel verdict, the trace needs the initial body facing and a rendered frame comparison against gamemd.
- Exact wall-clock equality of gamemd sequence pacing in milliseconds. Existing binary docs prove the sequence-frame transition condition, but this run did not measure retail wall-clock presentation for the concrete click.

## Adjacent Findings

- The Rust accept gate for `ToggleInfantryDeploy` is `DeployFire=yes`; gamemd's player deploy gate is `Deployer=yes`. For stock GGI both are true, so this does not change this scenario's accepted result. It can matter for modded data.
- The Rust self-click deploy branch returns before the common `emit_order_voice` path. Existing GGI docs say `VoiceSpecialAttack=GuardianGIMove` is the deploy command voice, but this trace did not prove the exact gamemd voice-trigger callsite for the scoped deploy click.
- Deployed uncrushability is implemented through `entity.deploy_state` plus `deployed_crushable`; it is not screen-visible in this scenario unless a crusher interacts with the deployed GGI.

## Verdict Tally

PASS: 5 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
