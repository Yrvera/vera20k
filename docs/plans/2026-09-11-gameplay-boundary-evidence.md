# Evidence for the revised gameplay goal boundaries

Companion to the [catalogue](2026-09-11-gameplay-goal-catalog.md), examined on
2026-09-11 against main `ed8f4837910be9329505c3dfc2fc074d9c1f3106`.
This records planning evidence, not a permanent progress ledger.

**Read one relevant E-section as an entry point.** Its source paths and native
reports are leads for the selected action, not a mandatory reading list. The
2026-09-11 source-status statements below describe the inspected commit; check
current source before carrying a missing/implemented claim into a new prompt.
The catalogue's scope definitions are separate from these dated observations.

Current Rust callers/state/consumers were read directly. Existing native reports
and local retail data support intended relationships; their historical Rust
status was not treated as current. No new binary execution, game test, downloads
or installations were performed. Exact timing/numeric/branch parity requires
the selected goal's own evidence. Shared Rust code alone cannot prove the native
engine uses the same boundary.

## E1 Resources and mining

**Current production path.** The Unit branch of
[the object AI host](../../src/sim/world/techno_ai.rs) calls
`dispatch_harvest_for_object`. The
[Harvest dispatcher](../../src/sim/miner/harvest_mission.rs) uses the entity's
mission cursor and timer, then calls the common
[miner state machine](../../src/sim/miner/miner_system.rs).
War/Chrono are branches in that owner, including the return/contact decisions;
they are not independent harvesting systems. Both reach the
[refinery sequence](../../src/sim/miner/miner_dock_sequence.rs), which follows
admission, unload, payout, release and resumption. Payout reads the refinery
owner and purifier/income modifiers; source/refinery loss has explicit abort
and interruption paths.

**The resource connection is an actual data path.**
[Terrain spawning](../../src/sim/terrain_spawn.rs), `place_tiberium_empty`, calls
the shared `tiberium::place_tiberium` in the loaded production context. The
[resource authority](../../src/sim/tiberium/mod.rs) reads and changes live
OverlayGrid type/density, handles growth/dirty effects, and is consumed by the
miner's `handle_harvest` through `reduce_tiberium_at_with_native_context`.
[Growth/spread](../../src/sim/ore_growth.rs) uses the same cell mutation context.
`ResourceNode` is a compatibility surface, not the authoritative loaded-map
resource store. This distinction matters when tracing a goal: an isolated test
of that compatibility store does not exercise TIBTRE-to-miner production.

**Native support.** The
[HARV/CMIN harvest report](../research/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md)
identifies the shared `Mission_Harvest` → `Harvest_Ore_Tick` → `Reduce_Tiberium`
path for both stock miners and ore/gems. The
[refinery synthesis](../research/CHRONO_MINER_REFINERY_DOCK_UNLOAD_SYSTEM_MODEL_SYNTHESIS.md)
records their common Harvest return decision with different type/rules branches.
The [TIBTRE placement report](../research/TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md)
traces terrain AI → forced spread → permitted new-cell placement → growth queue
and tactical/radar dirty state. It specifically distinguishes that new-cell
branch from growth of an existing field.

**Decision:** R1 joins War/Chrono, source/field behavior, docking and settlement
as a connected workstream. A narrower selected source/dock outcome still reaches
actual resource/miner/income consumers. Do not give the TIBTRE caller every
permissive behavior of its shared placement helper.

**Limits:** the old TIBTRE synthesis describes a detached Rust spawner and missing
live terrain lifecycle; that status is stale against current terrain ownership.
Existing research also contains corrected harvest timing. No new exhaustive
source/field/return/save/visual comparison was run for this catalogue.

## E2 Slave economy

[Slave harvesting](../../src/sim/slave_miner.rs) owns master/worker state and
`SearchOre → MoveToOre → Harvest → ReturnToMaster → Deposit`. It uses shared
`extract_bale`/resource access and purifier/income helpers, but `handle_slave_deposit`
credits whole storage slots through the master relationship. Its nearby native
evidence identifies `SlaveManagerClass::AI_Update` and the slave storage deposit
call. Standard Harvest dispatch explicitly excludes the slave host.

**Decision:** retain R2 as a related complete workforce loop, rather than a third
copy of the standard miner task. Changes to shared resource or payout contracts
require both R1/R2 consumer checks. Matching wallets and ore cells does not erase
worker assignment, liberation, master loss or deployment responsibilities.

## E3 Purchases and base lifecycle

[Factory lifecycle](../../src/sim/production/factory_lifecycle.rs) creates and owns
the held object through enqueue, completion and release/consumption.
[Production delivery](../../src/sim/production/production_queue.rs) and
[placement](../../src/sim/production/production_placement.rs) consume that identity
for mobile products, buildings and walls. This supports B1's consolidation of
queue/charging/completion/exit/placement with category/faction variants.
The [FactoryClass study](../research/FACTORYCLASS_PRODUCTION_DEEP_DIVE.md)
provides native owner context; it is not current implementation certification.

[MCV conversion](../../src/sim/world/world_spawn.rs), `deploy_mcv` and reverse
conversion, creates a replacement and transfers/removes its source independently
of the purchase queue. The
[MCV lifecycle synthesis](../research/MCV_DEPLOY_UNDEPLOY_LIFECYCLE_SYSTEM_MODEL_SYNTHESIS.md)
supports grouping stock MCV variants and distinguishes Slave Miner behavior.

[Repair and sale](../../src/sim/production/production_sell.rs) share a file but
not their transaction: repair changes a live building's HP/flag/wallet; sale
releases dependents, removes the building and settles value/capabilities.
The [native sale/repair report](../research/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md)
likewise describes different handlers. The current
[tick owner](../../src/sim/world/mod.rs) records the repair-versus-factory debit
ordering seam. These observations justify B3/B4 separation and joint checks at
the wallet/scheduler boundary, not a merged “money operations” task.

**Limits:** exact pricing/refund rules in older reports have corrections and
qualifications. B1's Industrial Plant integration is grounded in price consumers
and retail `FactoryPlant`/cost-modifier fields; a full Cloning Vat acquisition,
second-product and delivery trace is still required before its implementation
scope is final. Placing it with production is a consumer-based proposal, not a
claim of a completed current clone path.

## E4 Power capture and benefits

[Power state](../../src/sim/power_system.rs) consumes provider/occupant state and
feeds building/radar changes; [factory stepping](../../src/sim/production/factory.rs)
consumes the resulting production conditions. The
[power research](../research/POWER_SYSTEM_GHIDRA_REPORT.md) is a starting reference,
but its generic Robot power explanation does not establish the complete stock
`ROBO PoweredUnit` / `GAROBO PowersUnit=ROBO` provider relationship. The catalogue
keeps that exact gate open for investigation.

[Engineer resolution](../../src/sim/world/world_orders.rs) reaches
`change_owner_with_rules` and consumes the engineer. The central
[ownership transaction](../../src/sim/world/mod.rs), `change_owner_impl`, updates
tracking/counts/sensors/base authority and capture income;
[credit income](../../src/sim/credit_income.rs) handles relevant capture/startup
and timed-cash consumers. Native relationship support is in
[ChangeOwner lifecycle](../research/TECHNOCLASS_CHANGEOWNER_LIFECYCLE_ORDER_RESWARM_20260528.md).

**Decision:** B6 owns capture → benefit → loss as an observable outcome, but
production, income, unlocks and passive healing retain their actual state owners.
Mind control and slave liberation reuse ownership transfer while keeping their
different initiators and release rules. Hospital/Machine Shop retail fields
establish passive-benefit classification; their complete current consumer census
was not established here. Grinder and remaining special-benefit exactness are
explicitly targeted investigations, not proven equivalent admission paths.

## E5 Movement stance and cargo

[Camera/input state](../../src/app/input/camera.rs) and
[context-order admission](../../src/app/input/context_order.rs) include real
bookmark/selection/action decisions before a simulation command exists. This
supports a selectable battlefield-control goal through actual execution and
replacement. U1/U2 tests that directly inject commands cannot certify that input
loop; the revised catalogue explicitly retains its named interaction variants.

[Movement](../../src/sim/movement/mod.rs) and the
[installed locomotor slot](../../src/sim/movement/locomotion/slot.rs) connect orders,
movement state and lifetime. The
[movement ownership study](../research/GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP_GHIDRA_REPORT.md)
supports the cross-system action boundary; historical Rust status is excluded.
This supports U1, not an instruction to complete every locomotor before combat.

[Deploy commands](../../src/sim/world/world_commands.rs),
[stance state](../../src/sim/deploy.rs) and
[mission handlers](../../src/sim/world/techno_ai/mission_handlers.rs) connect
GI/GGI sustained stance to actual combat. The latter explicitly branches Yuri
pulses and Desolator radiation away from that behavior.
[GGI command evidence](../research/traces/GGI_DEPLOY_COMMAND_STANCE_TRACE.md)
supports the shared command/stance relationship. Similar input is insufficient
to merge those other effects or Siege Chopper flight into U3.

[Passenger admission](../../src/sim/passenger.rs),
[departure](../../src/sim/passenger/departure.rs) and
[vehicle unloading](../../src/sim/transport_unload.rs) connect cargo membership,
concealment, weapon changes, movement and release. Native
[IFV/OpenTopped evidence](../research/IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md)
supports keeping IFV/BFRT inside the complete passenger loop while preserving
host-versus-passenger firing responsibilities. One shared current representation
is not proof that those native firing owners can be collapsed.

Garrison admission, ownership reconciliation and combat use a building occupant
relationship, supported by the
[garrison synthesis](../research/GARRISON_SYSTEM_MODEL_SYNTHESIS.md).
By contrast, [BunkerLink](../../src/sim/docking/bunker_link.rs) and
[entity state](../../src/sim/game_entity.rs) define a reciprocal single-unit link
for `NATBNK`, explicitly distinct from cargo. The
[Tank Bunker entry/exit report](../research/TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE_GHIDRA_REPORT.md)
supports U6's separate full loop. Soviet `NABNKR` remains an infantry garrison
variant. The original catalogue's generic “bunker” wording missed this scope.

## E6 Sorties pools and attached effects

[Aircraft state](../../src/sim/aircraft/mod.rs) and
[attack missions](../../src/sim/aircraft/attack_mission.rs) connect Harrier/Black
Eagle attacks to pads, return and service. Native context is in the
[aircraft mission family](../research/AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md).
Two coexisting current docking state machines require investigation; the grouping
does not bless duplicate authority. Jumpjet users have different continuing
flight/landing rules and are not airfield-sortie variants.

[SpawnManager](../../src/sim/spawn_manager.rs) owns pool creation, target handoff,
launch, returning/expendable child state and regeneration; its
[native report](../research/SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) supports grouping
Carrier/Destroyer with V3/Dreadnought/Boomer. Parent/child expiry reaches
[common lifecycle](../../src/sim/world/lifecycle.rs). Boris instead enters an
AirstrikeClass path identified in [projectile dispatch](../../src/sim/projectile.rs).
That is a substantive owner difference, not just an aircraft-type distinction.

[CaptureManager](../../src/sim/capture_manager.rs), construction and pointer
expiry provide current state seams, while
[mind-control research](../research/MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md)
distinguishes reversible controller-owned capture from the Dominator's permanent
ownership effect. Group reversible controllers with their capacity/overload
variants. Acquisition and the full effect path are not currently established:
[special detonation](../../src/sim/combat/world_receiver.rs) routes several claimed
effects to an unimplemented endpoint.

The same limitation applies to using current code as proof of complete Parasite,
Temporal or Magnetron behavior. Their intended distinct relationships come from
the [Parasite](../research/PARASITE_CLASS_GHIDRA_REPORT.md),
[Temporal](../research/TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md) and
[Magnetron](../research/MAGNETRON_SYSTEM_GHIDRA_REPORT.md) owner studies. Drone/Squid
share the parasite/host family; erasure and locomotor replacement have different
progress/termination. Older broad reports contain incorrect identifiers/offset
tables; this revision uses their owner distinctions, not those details as new
implementation specifications.

## E7 World and information

[Engineer/hut interaction](../../src/sim/world/world_orders.rs) reaches
[the bridge walker](../../src/sim/bridge_state/walker.rs), low/high repair,
overlay/zone/radar refresh and engineer removal. The
[native bridge trace](../research/bridges/08-traces/ENGINEER_HIGH_BRIDGE_REPAIR_MUTATION_TRACE.md)
supports that chain; its old missing-Rust admission/radar findings are stale.
High/low/orientation are variant coverage of the integrated bridge lifecycle.

[Terrain lifecycle](../../src/sim/terrain_object.rs), combat finalization and
[terrain presentation](../../src/app/presentation/instances/overlays.rs) connect
ordinary scenery occupation and removal to its actual visual/spatial result.
Source flags determine interactions; no generic tree-crushing claim is justified.

[Vision reconciliation](../../src/sim/vision/mod.rs),
[sensor lifecycle](../../src/sim/sensor_lifecycle.rs) and
[radar availability](../../src/sim/radar.rs) have related consumers but different
state: shroud knowledge/source latches, counted detection circles, and powered
radar providers. Native references are
[Gap/radar interaction](../research/GAP_RADAR_SHROUD_MINIMAP_INTERACTION_GHIDRA_REPORT.md)
and [Psychic Sensor intent lines](../research/PSYCHIC_SENSOR_ENEMY_ACTION_LINES_RECHECK_GHIDRA_REPORT.md).
Psychic intent warnings require their own eligibility; they are not interchangeable
with cloak detection or map reveal. Whole information-building goals still
include the shared viewer/targeting consumers.

## E8 Strategic powers

[Superweapon instances](../../src/sim/superweapon/mod.rs) own grant/revoke/charge;
[command dispatch](../../src/sim/world/world_commands.rs) hands off the selected
effect and recharge. The
[native lifecycle study](../research/SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md)
supports shared support ownership with power-specific effects.

Retail `ChronoWarp` explicitly has `PostClick=yes` and
`PreDependent=ChronoSphere`; the
[two-stage native handler](../research/CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md)
supports an inseparable complete teleport action. American/Tech Airport drops
are provider/payload variants requiring both checks. Distinct storm, mutation and protection owners require separate effect evidence
and comparisons, but do not establish separate goal-session boundaries. A retail
strategic-powers goal can own shared lifecycle and all those effects through
several increments. Full current effect coverage was not audited in this revision.

## E9 AI scenarios and session flows

[Current AI](../../src/sim/ai.rs) emits ordinary gameplay commands through the
[production tick](../../src/sim/world/mod.rs); actual outcomes feed later
decisions. Native [base placement](../research/PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md)
and [Team reachability](../research/PHASE3_TEAM_PRODUCTION_REACHABILITY_GHIDRA_REPORT.md)
support grouping decisions with their real production/recruitment/action
consumers. Existing attack waves are playable behavior, not proof of native Team
equivalence. [The Team VM](../../src/sim/team_script_vm.rs) supports a bounded
action set; advancing a cursor does not establish the actual scripted action.

[Trigger runtime](../../src/sim/trigger_runtime.rs) instead owns persistent event
conditions/latches and action ordering, reaching real
[simulation/app consumers](../../src/app/match_runtime/sim_tick.rs). It shares
Team/command capabilities with AI while retaining a distinct scenario lifecycle.

[Shell routing](../../src/app/shell_main_menu.rs),
[skirmish session](../../src/app/frontend/skirmish_session.rs),
[load transitions](../../src/app/loading/transitions.rs) and
[session exit](../../src/app/match_runtime/scenario_exit.rs) establish full
launch/play/return as a coherent flow. Native
[Single Player dispatch](../research/SINGLE_PLAYER_SUBMENU_DIALOG_CASE1_GHIDRA_REPORT.md)
establishes destination structure, not complete campaign semantics. Current
campaign selection lacks launch; Network/WOL routes report unimplemented.
A shell-route task is therefore bounded separately from destination backend
completion, while an “all destinations work” claim remains open.

## E10 Persistence networking and presentation

[Persistence commands](../../src/app/persistence/commands.rs),
[prepared replacement](../../src/app/persistence/mod.rs) and
[restore](../../src/app/match_runtime/restore.rs) form one save/resume path,
including clearing old effects and rebuilding visual/UI state. Current restoration
requires an existing matching simulation/map/rules context; this is not proof of
cold-start save loading. Native context:
[save/load frontier](../research/core-services-map/frontier-saveload.md).

[Replay](../../src/sim/replay.rs) has a separate identity/initialization/playback
flow. Native [replay/save distinction](../research/REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md)
explicitly distinguishes scenario reinitialization from saved-object restoration.
[Net staging](../../src/net/mod.rs) and offline
[live session selection](../../src/app/match_runtime/sim_tick.rs) do not provide a
complete host/join protocol. [Native frame scheduling](../research/NETWORK_FRAME_SCHEDULING_GHIDRA_REPORT.md)
is support evidence for LAN, not an implementation claim. Save, replay and LAN
share deterministic foundations but have different inputs/state/termination.

[Instance projection](../../src/app/presentation/render/build_instances.rs),
[sound dispatch](../../src/app/match_runtime/sound_dispatch.rs) and
[lighting](../../src/app/presentation/lighting.rs) consume real gameplay state
and events. [Light lifetime research](../research/LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md)
connects power/death/sale/save with presentation and retains an outer restore-call
uncertainty. This supports per-goal presentation/cleanup obligations, not a later
isolated “all rendering” phase. Shared cross-consumer defects and media playback
can still be complete separately selected outcomes.

## Scope carried forward from the original catalogue

This is a migration index, not a progress chart. Old identifiers refer to the
catalogue at commit `5937c06c`. A repeated destination means integration or
variant coverage, not competing implementation owners.

| Previous entries | Revised home |
|---|---|
| P1–P4, P9 | F1–F3/F6–F7 and actual session outcomes. |
| P5–P7 | Selectable battlefield-control loop, U1/U2, B1/B3–B5, W4–W6, and shared input/presentation consumers. |
| P8 | Per-loop cues, shared voice/EVA/positional/music consumers, F7 media. |
| W1, W9 | F1/F2 content-to-world launch; every relevant loop's map/presentation consumers; shared composition corrections. |
| W2, W5–W8 | W2, W1, W3, W4/W5 and W7 respectively. |
| W3–W4, E1–E5 | R1/R2, including standard miner variants, terrain supply, docking and income. |
| E6–E9 | B1/B2 with actual placement/activation/product delivery. |
| E10–E11 | B3/B4/B5 as distinct repair, sale and provider lifecycles. |
| E12 | Purifier R1; Industrial Plant/Cloning Vat B1; Bio Reactor B5 with passenger integration; Grinder B8. |
| E13–E14 | B6 tech-benefit acceptance, B5 Robot/provider relationship; Tech Airport with paradrops. |
| U1–U6 | U1/U2, W5, B6; special combat dependencies remain with their actual effect owners. |
| U7–U10 | U4/B7/U7/U9, with applicable U8 spawned-child consumers. |
| A1–A4 | U3, U9, U4/U5. Add explicit Tank Bunker U6. |
| A5–A10 | Named distinct weapon/effect loops with B5/U2/W1 and persistent world consumers. |
| A11–A13 | U11/U10 with explicit parasite/controller variants. |
| A14–A17 | Chrono movement, Temporal, Spy infiltration and Magnetron whole effects. |
| A18–A19 | U8 launcher-owned pools, including returning and expendable variants. |
| A20–A23 | Boris, Dog, Disc and Yuri pulse full-effect goals. |
| S1–S10 | Strategic-power section, preserving all named powers and linked/drop variants. |
| S11 | W4 Gap/SpySat, W6 Psychic warning, with B5 provider dependencies. |
| G1–G4 | F4–F6: opponent, authored events and campaign continuation. |
| N1–N4 | F8–F11: complete save, replay, LAN and online flows. |

Remaining priority decisions require current-match evidence. Remaining exact
branch contracts require selected-goal native work. Neither is inferred from
how many catalogue rows a family now occupies.
