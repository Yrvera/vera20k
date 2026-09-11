# Gameplay loops and proposed goal boundaries

Use this reference to compose a goal for the already-playable game. Rows describe
related work; they do not report current completion, prescribe execution order,
or authorize implementation by themselves. [ENGINE.md](../../ENGINE.md) governs
evidence, architecture, validation and delivery, including the scale exception.

## Start with the requested outcome

**A row identifier alone is not a goal prompt.** R1, B1, B5, U1/U2, U9 and
F1/F4/F10 in particular describe broad families. State the actual intended scope:

| User's intended result | Preserve this completion bar |
|---|---|
| Complete an object or family like retail | Enumerate active behavior and variants, including required interactions found during the work. Sample scenarios start the investigation; they cannot close the full scope. |
| Complete a named gameplay loop like retail | Cover that loop and its named variants through continuation/cleanup. Include broken required handoffs; do not absorb unrelated behavior merely because a shared helper reaches it. |
| Fix a demonstrated regression or refactor existing code | State the trigger and intended behavior-preservation/fix bar. Do not silently turn this into exhaustive family parity or treat existing Rust output as retail evidence. |

Preserve what the user requested. Do not downgrade a full-family goal to one
convenient scenario, or expand a bounded fix into a full-family project. Resolve
discoverable details from current source/evidence; ask only when a consequential
scope or support decision cannot be inferred. A wide scope can take multiple
PRs and resumptions without becoming multiple disconnected goals.

### A concise goal brief

Use the [goal-prompt skill](../../.agents/skills/goal-prompt/SKILL.md) when composing
the final text. State the outcome/reason, scope/variants, starting evidence,
comparison and production bar, dependency boundary and completion condition.
Use current branch/HEAD and reproduction when available; an unknown discrepancy
can be investigated inside the goal without inventing a finding. This brief is
the prompt itself, not an additional mandatory planning artifact.

> Follow ENGINE.md. Deliver **[outcome]** because **[problem or capability]**.
> The scope is **[full object/family, named loop, or bounded fix]**, including
> **[variants and necessary interactions]**. Start from **[relevant evidence and
> known current context]**, verifying assumptions against current source.
> Compare **[the same native operation/inputs, or the behavior-preservation bar]**
> and validate **[real entry → result → continuation/release]**, including
> **[important interruption/ownership cases and observable outputs]**.
> Reuse working behavior, include coherent prerequisites and check affected
> consumers. Complete the named scope; a helper, passing sample or merged PR
> alone does not close it. Preserve any remaining in-scope work for continuation.

Preserve granted publication authority, budgets and workflow settings; add none
implicitly. Leave design/decomposition/tools open except for real constraints.
If implementation is authorized, preparation alone does not complete the task.

### Load the relevant evidence, not the whole catalogue

Read the selected rows and the entry below, then follow source/native references
as the action requires. Evidence is a dated starting point, not a fixed file list
or permission to skip newly discovered consumers. The phase inventory and migration
history are optional coverage aids, not required session startup reading.

| Selected work | Evidence entry |
|---|---|
| R1 | [E1 Resources and mining](2026-09-11-gameplay-boundary-evidence.md#e1-resources-and-mining) |
| R2 | [E2 Slave economy](2026-09-11-gameplay-boundary-evidence.md#e2-slave-economy) |
| B1–B4 | [E3 Purchases and base lifecycle](2026-09-11-gameplay-boundary-evidence.md#e3-purchases-and-base-lifecycle) |
| B5–B8 | [E4 Power capture and benefits](2026-09-11-gameplay-boundary-evidence.md#e4-power-capture-and-benefits) |
| C1, U1–U6 | [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo) |
| U7–U11, X15–X17 | [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects) |
| X14 | [E1](2026-09-11-gameplay-boundary-evidence.md#e1-resources-and-mining) for miner return and [E5](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo) for installed movement. General teleport/infantry recovery still needs a targeted native-owner lookup; neither entry is its full contract. |
| W1–W7 | [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information) |
| S1–S10 | [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers) |
| F1–F6 | [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows) |
| F7–F11, shared output problems | [E10 Persistence networking and presentation](2026-09-11-gameplay-boundary-evidence.md#e10-persistence-networking-and-presentation) |
| Other X goals | Their named effect and actual callers need a targeted lookup; the evidence file does not contain a full contract for each. Start with [weapon selection](../../src/sim/combat/combat_weapon.rs) / [effect dispatch](../../src/sim/combat/world_receiver.rs), then the active native owner. Use E4/E5/E7 for affected power, input or world consumers. |

## How relationships determine scope

| Relationship found | Consequence for a goal |
|---|---|
| Same state machine or transaction, with different data/branches | Usually investigate together, with explicit variant checks. War/Chrono Miner, GI/Guardian GI deployment and different factory products are examples. |
| Direct producer and consumer | Include the integration needed for the named result. TIBTRE creates resource cells that growth and miners consume; a spawning-only test does not complete that resource loop. |
| A relationship that persists during an action | Own establishment, operation and release together: passenger/transport, controller/victims, launcher/spawn pool, aircraft/airfield. |
| Shared service, but different state and termination | Keep each complete action, and check affected consumers when changing the service. Sharing credits does not merge repair and sale; sharing ownership transfer does not merge engineers and mind control. |
| Same button, class name, visual effect or theme | Insufficient reason to group. MCV, GI, Desolator and Slave Miner all deploy, but their resulting lifecycles differ. |

**A goal can cross catalogue rows.** These entries identify coherent default
scopes and important relationships; they are not walls around source directories.
A family may contain several complete goals, and a large goal may take several
PRs. Neither fact permits a required consumer to be deferred while claiming the
named goal complete.

Do not merge every transitive dependency into one giant task. Stop expanding when
the selected action reaches its required continuing state through an established
consumer contract. For example, a purchase must yield a usable tank that can
receive its first order; it need not reimplement every tank weapon. If that first
order exposes a broken required handoff, include the repair in the purchase goal.

## Resources and the base

References are identifiers, not an execution order. Each row includes the normal
entry, actual gameplay result, feedback and continuation or cleanup. Variants
named here are starting coverage; exhaustive goals require a full active census.
Include reachable civilian, preplaced, reinforcement, elite, special-acquisition
and mode/map/campaign variants where they affect the named scope.

| Ref | Complete outcome and variants to examine together | Boundary and continuing result |
|---|---|---|
| R1 | **Resource supply becomes spendable income through standard miners.** Map ore/gems, TIBTRE01–03, growth/spread, War/Chrono Miner, refineries, cargo/value/storage and Ore Purifier. | Supply → authoritative resource cell → visible/searchable resource → mining/cargo → return/dock → income → release → next trip. Preserve different return paths and source-placement versus existing-field-growth rules. Include relevant orders/combat interruptions and concurrent-spending checks. Existing spending systems consume the wallet; this is not ownership of all economy code. |
| R2 | **Slave Miner and its workforce sustain harvesting through relocation and loss.** Mobile/deployed master, slaves, assignment, harvesting, replacement/liberation and ownership. | Share R1's resource/payout contracts; own master/worker state, return/deposit, deployment and cleanup here. Verify resources produced by R1 remain usable by slaves. Worker deposit does not use the standard refinery queue. |
| B1 | **A purchase becomes a usable product and the queue continues.** Infantry, vehicles, ships, aircraft, structures/walls; faction/factory variants; prerequisites, build limits, prices, active building upgrades/slots, Industrial Plant and Cloning Vat consumers. | Sidebar/AI order → eligible queue → factory-held identity → charging/hold/cancel → completion → exit or placement/buildup/activation → next product. Blocked exits, provider loss, upgrade admission/effects, cloned-product delivery and completed-object disposition belong here. Building placement and mobile delivery are terminal variants of one purchase lifecycle. |
| B2 | **MCVs establish and relocate usable construction bases.** Stock MCV/Construction Yard variants and applicable reverse conversion. | Command → stop/turn/admission → successful target creation → source removal → selection/build authority → reverse conversion and usable mobile unit. Coordinate sale-completion machinery with B4. Slave Miner stays with R2's workforce lifecycle. |
| B3 | **Buildings repair correctly while the match continues.** Repair toggle, funding, HP/damage states and power effects. | Order → scheduled debit/healing → presentation/power update → completion/cancel/destruction. Include competing production/depot spending where ordering matters. The building remains the same live object; this differs from sale and unit service. |
| B4 | **Selling releases a building, its dependents and its value correctly.** Ordinary, occupied, docked, upgraded and capability-providing variants where active. | Sell → animation/state → crew/passenger/refinery/bunker release → removal → refund and house/production/power changes → usable ground and released units. Check the resulting world, not only a refund formula. B2 owns reverse-MCV acceptance when this machinery is involved. |
| B5 | **Power/provider changes disable and restore the right capabilities.** Plants, Bio Reactor occupants, radar, production, defenses, Robot Control Center/Robot Tanks and strategic availability. | Gain/change/lose provider → output/demand/provider state → actual shutdown → feedback → restoration and resumed use. Bio Reactor passenger admission/release must reach power. Establish Robot's specific provider relationship; generic positive house power is not a sufficient specification. Tesla charging and Disc drain include their B5 integration. |
| B6 | **Engineers capture buildings, use their benefits and lose them correctly.** Base/factory structures, Oil Derrick, Secret Lab, Tech Hospital/Machine Shop and other active tech variants. | Approach/admission → old-owner cleanup → transfer/engineer disposition → actual benefit → later loss/revocation. Each benefit retains its mechanism owner: production, periodic income, unlocks or passive healing. Hospitals/Machine Shops are not repair-depot visits. Engineer bridge-hut behavior integrates W1. |
| B7 | **A unit visits a service facility and returns to use.** Active repair-depot/service variants, including affected miners. | Enter/radio/reservation → service/debit/healing → release → next order or resumed mining. Include contention, interruption and provider loss. Reuse docking primitives while retaining refinery unload and airfield sorties as their distinct complete loops. |
| B8 | **A Grinder admits and consumes a unit with correct settlement.** Eligible units, ownership and interrupted entry. | Approach/admission → consumption → value/house effects → cleanup and continued facility use. Shared credits or Enter machinery do not turn this into building repair or sale. Remaining exact admission/settlement rules require a targeted native trace. |

### Example: resources and miners

A broad goal can be:

> Make the standard resource economy work like retail from map/TIBTRE supply
> through ore/gem growth, War and Chrono Miner harvesting, refinery unloading and
> income, including interruptions and the next trip. Reuse existing behavior and
> establish native coverage for both miner variants.

A narrower complete goal can be:

> Make TIBTRE01–03 work from normal map loading and animation through resource
> creation, subsequent resource behavior and real miner harvesting. Include
> the shared resource changes required for that result and verify affected consumers.

The narrower goal does not become “parse TIBTRE fields,” nor does it require
reproving every unrelated gem/cost-modifier branch. If full R1 parity was
requested, all its included variants remain required. Selecting a smaller goal
must be explicit; it cannot silently shrink an existing full goal.

## Units, passengers and continuing combat relationships

| Ref | Complete outcome and grouped variants | Boundary and continuing result |
|---|---|---|
| C1 | **Battlefield controls reach actual gameplay.** Scrolling/bookmarks, single/bandbox/type/control-group selection, hotkeys, queued/planned waypoints, radar navigation and permitted pause/resume/speed changes. | View/select → cursor/order admission → visible/audible feedback → actual execution → replacement/cancellation. Reach U1/U2 or the actual selected action. A marker or directly injected simulation command cannot certify the player interaction. |
| U1 | **Units obey movement/replacement orders through arrival.** Walking, driving, ship and hover variants. | Command → mission/destination/installed locomotor → path/turn/traffic/occupancy/crush → arrival → actual next action. Include stop, blocked recovery, replacement and death. Select the affected locomotor set explicitly; sharing this contract does not make all locomotors identical. Ability-specific movement and sorties stay integrated with their parent actions. |
| U2 | **An ordinary engagement resolves from order to aftermath.** Common infantry/vehicle/building/naval/air cases; direct, ballistic, homing and applicable anti-air/air-to-air/strafe/bombing attacks. | Approach/acquire → weapon/range/facing/fire gates → burst/projectile/impact → armor/damage/fear/prone/veterancy/death/attribution → next order/target. Include debris and affected terrain/resource consequences. Reuse shared combat authority; representative units are coverage cases, not duplicate implementations. |
| U3 | **GI and Guardian GI fight through deployment and recovery.** Both sustained deployed-infantry variants. | Deploy input → stance/animation/movement gate → actual deployed targeting/firing → undeploy/reorder/damage/death. Integrate ordinary movement/combat and applicable host interactions. Desolator radiation and Yuri pulses remain with their effects; Siege Chopper with U9. |
| U4 | **Mobile transports and passengers work through carrying, fighting and release.** Land/sea/air transports, IFV and Battle Fortress. | Admission → cargo membership/concealment → movement and applicable firing → unload placement/retry or host destruction → usable passengers. IFV selects the host's weapon; Battle Fortress passenger firing retains its own responsibility. Air-transport landing/exit is a required variant. |
| U5 | **Infantry occupy, fight from and leave buildings.** Civilian garrisons and Soviet Battle Bunker `NABNKR`. | Admission → occupant/ownership state → occupant firing/credit and occupied art → voluntary/forced evacuation, sale or destruction → released actors/building state. Keep presentation and firing with occupancy. |
| U6 | **Yuri Tank Bunker `NATBNK` installs, supports and releases a vehicle.** | Approach/radio → reciprocal single-vehicle link/install → actual combat → release/sale/destruction → link cleared and vehicle usable. This differs from U5's infantry occupation model. |
| U7 | **Harrier and Black Eagle complete repeatable airfield sorties.** Both aircraft and provider/pad variants. | Production/idle → takeoff/attack → return/reservation/landing → rearm → next sortie. Include provider loss/capture, contention, replacement orders and aircraft loss. Flight alone cannot complete this loop. |
| U8 | **Launchers manage spawned aircraft or rockets through repeated attacks.** Carrier/Destroyer and V3/Dreadnought/Boomer. | Parent target → fixed spawn pool → child launch/attack → return/reload or missile regeneration → next attack. Include parent/child/target loss and owner changes. Returning aircraft and expendable missiles are variants of the same pool owner. Boris has a different designation/airstrike lifecycle. |
| U9 | **Hovering airborne units move and fight through required transitions.** Rocketeer, Kirov, Nighthawk, Disc, Siege Chopper and active scenario variants. | Jumpjet/altitude behavior → actual attack/carry order → stop/landing where permitted → next action. Chopper landing/deployment/weapon change/resumed flight is a complete named outcome. Disc drain and Nighthawk passengers integrate their effect/cargo owners; locomotion alone cannot certify them. |
| U10 | **Reversible mind control maintains and releases victims.** Yuri Clone/Prime, Psychic Tower and Master Mind. | Acquisition → controller membership/owner transfer → actual order/house/production/visual consumers → capacity/overload and release/controller/victim loss → required restored state. Group controller variants; permanent Dominator effects and deployment pulses differ. Inspect acquisition and release callers, not only manager fields. |
| U11 | **Parasites maintain and release their hosts.** Terror Drone infestation and Giant Squid grapple. | Admission → parasite-host relationship → continuing effects → service/escape/detach/death → cleanup. Group the relationship with explicit variant rules; do not merge Temporal or Magnetron merely because they share the special-weapon dispatcher. |

### Distinct abilities that retain their own effect loops

These are separate candidate goals, not one “finish special units” task. Each
includes ordinary movement/combat and presentation consumers required to exercise
the ability. Exact branch coverage remains work for the selected goal.

| Ref | Ability | Complete outcome |
|---|---|---|
| X1 | Gattling Tank/Cannon | Fire → stage progression and actual weapon/damage/feedback changes → interruption/retarget/cooldown → subsequent firing. Check unit/building consumers together. |
| X2 | Prism support | Support membership/availability → supported shot/damage/beam → target/provider interruption and cleanup. Prism Tank is an attack consumer, not automatically part of the tower network. |
| X3 | Tesla charging/attacks | Trooper/coil relationship → charging/overpower and power exceptions → actual shot/bolt → loss/release/recovery. Ordinary Tesla weapons reuse confirmed effect machinery. |
| X4 | Sonic attacks | Fire → wave and actual hit behavior → effect/source/target expiry. Shared drawing code does not merge this with lasers. |
| X5 | Ordinary laser attacks | Fire → beam and actual hit behavior → effect/source/target expiry. Disc-specific attack/drain is X19. |
| X6 | C4 and bridge charges | Legal approach/action → explosion and actor/bridge consequences → cleanup. Include world and release consumers. |
| X7 | Ivan bombs | Attachment/ownership → fuse/defusing → detonation or removal → cleanup/feedback. This is distinct from an immediate explosive attack. |
| X8 | Suicide attacks | Firing/detonation and firer removal → collateral damage/credit/cleanup. Verify active unit variants. |
| X9 | Desolator radiation | Deployment/firing → radiation-site/target damage → source changes/expiry → continuing world state. Deployment belongs with this effect. |
| X10 | Virus effects | Source attack → persistent damage/effect lifecycle → affected actors/ownership → expiry/cleanup. Include actual damage and feedback. |
| X11 | Chaos Drone | Effect admission → changed behavior → ownership/liveness interactions → expiration and resumed behavior. |
| X12 | Other active fire/status effects | Select the named active effect and its consumers; follow creation → continuing damage/behavior → expiration. This is a coverage family, not a task to implement every particle system. |
| X13 | Yuri/Prime deployment pulses | Deploy → area-damage weapon/target rules → stance recovery → next orders. Group pulse variants separately from reversible capture. |
| X14 | Chrono movement | Teleport admission → movement/occupancy transition → recovery → usable next action, including affected infantry/miner variants. R1 includes the miner's complete return outcome. |
| X15 | Temporal erasure | Attacker/target link → suspension/progress/competing attackers → interruption or erasure → cleanup. |
| X16 | Magnetron | Source/victim locomotor handoff → lift/carry → drop/restoration and terrain/damage effects → cleanup. Jumpjet is a dependency, not proof this is an ordinary aircraft. |
| X17 | Boris airstrike | Designation → called aircraft → actual attack/departure → source/target/cancellation cleanup and subsequent use. |
| X18 | Dog leap | Legal attack → leap/impact → target loss or interruption → recovery/next order, with applicable disguise/detection interactions. |
| X19 | Floating Disc | Flight/attack/drain admission → target-class money/power/defense effects → release/source/target loss → dependent capabilities recover. |
| X20 | Spy infiltration | Disguise/approach/detection/admission → target-specific result → spy disposition → actual power/economy/information/production/research consumer. Shared Enter/ownership services do not make this engineer capture. |

## World interaction and information

| Ref | Connected goal | Boundary and variants |
|---|---|---|
| W1 | **Bridges remain coherent through traversal, damage, collapse and repair.** | High/low/orientation variants share the world lifecycle. Include occupants/on-under passage, combat, engineer/hut admission, topology/zone/overlay/radar refresh and traversal after repair. Repair animation is insufficient without restored passage. |
| W2 | **Scenery affects the world throughout its lifetime.** | Trees/rocks share terrain lifecycle with type/theater/immune/damage variants: load → occupation/appearance → applicable interaction → removal/spatial cleanup. TIBTRE integrates this terrain owner while spawning is R1. Do not assume every tree burns or can be crushed. |
| W3 | **Walls and active gates alter passage correctly.** | Connections, placement/load, opening/obstruction, damage/removal and navigation/appearance stay together. B1 owns purchased wall delivery; this loop owns the obstacle. Prove active YR gate variants before importing legacy fence behavior. |
| W4 | **Exploration and shroud sources produce correct per-viewer knowledge.** | Scout/source activation → reconciliation → tactical/radar/selection/targeting → source loss/ownership/alliance/observer changes. Gap/SpySat are explicit source/latch cases; strategic reveals include their activation/effect lifecycle. Powered radar and revealed terrain are different facts. |
| W5 | **Concealment and detection govern observer knowledge and attacks.** | Cloak/disguise exposure or sensor/provider movement → counted detection/resident reevaluation → targeting/picking/rendering → owner/limbo/expiry cleanup. Include Spy/Mirage/naval variants; share W4 without collapsing distinct concealment rules. |
| W6 | **Psychic Sensors show the correct enemy-intent warnings.** | Provider/enemy-order eligibility → warning lines → order/source changes and cleanup. Establish its own radius/rules; this is not merely radar dots or cloak detection. |
| W7 | **Crates yield complete rewards.** | Spawn/regeneration → ordinary movement pickup → effect/feedback/removal → later unit/economy consequences. Active reward classes are variants; include ordinary stock skirmish. |

## Strategic powers

Named powers share granting/revocation, charge, sidebar and targeting support.
Each power goal includes that support through its actual effect and aftermath.
A common charge timer does not make every power one task.

| Ref | Power | Result the complete goal must reach |
|---|---|---|
| S1 | Nuclear Missile (`MultiMissile`) | Launch/descent/impact → damage and world effects → cleanup. |
| S2 | Lightning Storm | Activation → storm scheduling and actual strikes → termination and continued play. |
| S3 | Iron Curtain | Eligible target application → protection/damage interactions → expiry and subsequent use. |
| S4 | Force Shield | Application → protection and applicable house/power consequences → recovery. |
| S5 | Chronosphere plus ChronoWarp | Both linked clicks → destination admission → affected-object transfer and aftermath. The first click cannot close the action. |
| S6 | Psychic Dominator | Effect sequence → actual damage/ownership result → continuing victim/world behavior. |
| S7 | Genetic Mutator | Eligibility → transformation → usable resulting actors and source cleanup. |
| S8 | American and Tech Airport paradrops | Provider/payload variants → aircraft/drop → actual landed passengers and aircraft cleanup. Check both variants. |
| S9 | Spy Plane | Targeting → approach/overflight and actual information effect → departure/cleanup. |
| S10 | Psychic Reveal | Targeting → actual per-viewer knowledge effect → required duration/termination. |

Weather, protection, transformation and permanent ownership changes have different
state and cleanup despite sharing sidebar code. S5's stages and S8's variants
remain coupled scopes; row identifiers do not split their internals into tasks.

Provider loss/capture, relevant power changes, targeting cancellation, repeated
use and save/restore belong to each selected power. Shared-framework changes
require affected-consumer checks. An unregistered INI section or an EMPulse-named
key used by the live nuke is not evidence of another active strategic power.

## Opponents and complete application flows

| Ref | Goal and related variants | Boundary and integration |
|---|---|---|
| F1 | **Configure, launch, play and finish a skirmish.** | Shell choices → actual map/rules/mode/assets/starts → loading → playable match → outcome/surrender/scores/statistics/results/restart/exit → clean next session. Include launch failure/cancel paths. Faction/map/difficulty are variants. Exercise content precedence, theaters, ramps/cliffs/shores/bounds and mode/map overrides through actual world construction and play. |
| F2 | **Generate a map and play that same map.** | Options/seed → generator/preview → matching launch → movement/construction and cleanup. Share F1's launch contract; generation has its own RNG/content identity. |
| F3 | **Navigate the shell and retain settings.** | Menu/dialog → correct child route/settings operation → apply/cancel/back/focus → handoff or exit/persistence. Shell navigation is bounded; campaign/network mechanics are destination goals. “All destinations work” requires combined integration. Include controls, display/audio/gameplay options and restart. |
| F4 | **An AI opponent builds, fights, defends and recovers.** | House decisions → ordinary production/deploy/place/attack consumers → resulting state → future decisions. Connected internal loops: base economy/placement/rebuilding and team selection/recruitment/script execution/replenishment. Faction/difficulty are variants. A loaded registry or advancing cursor cannot establish an actual attack/defense. |
| F5 | **Authored scenario events produce their full consequences.** | Trigger/Tag/variables/latches → ordered action → real units/teams/reinforcements/camera/messages/objectives → next event/outcome and saved continuation. Share Team/command machinery with F4; scenario triggers have different conditions and persistent state. Include active convoy/special-mission cases when established. |
| F6 | **Campaigns launch, advance and resume progress.** | Campaign/mission/difficulty/briefing → actual scenario → outcome → next mission/carryover → persisted progression. Integrate F5 scripts and F7 media. Campaign progression is not a skirmish setting variant. |
| F7 | **Movies and briefing media play and return correctly.** | Start → synchronized video/speech/subtitles → skip/finish → correct screen/mission. Formats and destinations are variants; reusable media machinery stays shared. |
| F8 | **Save a match and resume it through supported entry points.** | UI → content/version validation → prepared world/fixups → commit → rebuild presentation/reset pacing → continued play. Include shell startup/in-game paths, failure/cancel and active feature state. Restoring within an existing content context cannot by itself establish cold-start loading. |
| F9 | **Record and replay a match.** | Identity/scenario/seed → scenario initialization → timed commands → completion/stop or actionable divergence. Shared deterministic commands do not make replay a save restore or LAN session. |
| F10 | **Host, join, play, observe and leave LAN matches.** | Discovery/lobby/content/seed/options/transfer → launch → synchronized real peers → communications/alliances/observers → result/disconnect/recovery-or-abort. Select complete children such as host-to-finished-match or transferred-map-to-play. Staging/network internals alone do not establish playable LAN. |
| F11 | **Use a chosen online service to enter and leave actual matches.** | Explicit service/support decision → session/account/chat/discovery → actual multiplayer handoff → return/disconnect. Reuse F10 match authority; settle service policy before a retail-equivalent online claim. |

## Shared work stays integrated

Rendering, audio, input, persistence, determinism, identity and reference expiry
are obligations of affected loops, not later finishing phases. A bridge repair
includes visible/audible results; a deposit includes cargo/pips, refinery feedback
and displayed credits; save replacement clears old effects and rebuilds output.

Shared defects can justify a focused cross-consumer goal: mixed TMP/SHP/voxel
depth/palette/light/shadow composition, selection/camera projection, concurrent
unit-voice/EVA arbitration, positional sound or music transitions. Scope that
through real consumers/output, rather than “finish the rendering module.” Extra
VERA zoom behavior needs an explicit target instead of implied retail parity.

Command queues, pause/game-speed policy, clocks/timers/RNG, snapshot/hash/replay,
lifecycle cleanup and scale remain standing ENGINE obligations. Check them where
the selected action depends on them. Preserve one owner per mechanism;
overlapping acceptance does not authorize competing implementations.

## Keep the goal intact while working

When investigation crosses a row or module, distinguish these cases:

| Finding | Action within the authorized goal |
|---|---|
| Required state, producer, handoff or cleanup is broken | Include the coherent prerequisite and its validation, even across rows. A TIBTRE result cannot close while its created ore is invisible or unharvestable. |
| A changed shared owner has other consumers | Check the affected behavior in those consumers and fix regressions caused by the change. This does not automatically require full retail parity for each consumer. Preserve one owner and coordinate with any task already changing it. |
| Existing adjacent defect does not prevent the requested result and is not a regression from this change | Record its concrete trigger/effect and continue the selected goal. If the user's full-family scope includes it, it is required work rather than an adjacent deferral. |

Use current implementations and reusable native comparisons where their identity,
inputs and coverage still apply. Focus new acceptance on the selected operation
and the handoffs it can break: repeated trips/attacks, replacement orders,
contention, provider/target loss or relevant saved-state continuation. These are
case-selection aids, not a fixed test quota or a substitute for exhaustive scope.
Keep the native comparison bar separate from Rust regressions and rendered/audio
output checks, as ENGINE requires.

One owner carries the result across PRs and resumptions. Follow the existing
[checkpoint convention](../../.agents/skills/_shared/handoff.md) for sustained
work; do not create another catalogue status tracker. On resume, recover the
governing prompt/amendments, actual Git state, supported work, remaining scope,
validation/review state and next safe action. A completed prerequisite, successful
test or merged PR does not end a larger authorized goal. Missing required proof
keeps the relevant scope open; it does not authorize weakening completion or
ignoring an explicit stop/budget limit.

### Filled example: one loop, two unit variants

> Follow ENGINE.md. Complete GI and Guardian GI sustained deployed combat like
> retail (U3), preserving existing playable behavior. Scope includes both units'
> normal deploy input, stance/animation and movement gates, actual deployed
> targeting/firing, undeploy/replacement orders and relevant damage/death cleanup.
> Start with E5 and verify its current command, stance, mission and weapon
> consumers. Establish active native branches and compare the same states/inputs;
> exercise the real control-to-combat path, visible transitions and resumed
> movement/firing for both variants. Cover interruption and repeated use, with
> saved-state checks where affected. Include broken required handoffs and check
> other consumers of changed shared state. Complete this loop and retain its
> evidence; this does not certify the entire infantry roster or either unit's
> unrelated abilities. Preserve outstanding in-scope behavior for continuation.

For “make the GI work exactly like retail,” that narrower example is insufficient:
whole-object scope also covers ordinary movement/combat/damage, applicable host
and special-effect interactions, and lifecycle. Use entries to find those
relationships rather than substituting U3 for the user's larger objective.

## Coverage, priority and limits

The [old phase inventory](2026-07-30-clean-slate-system-implementation-order.md)
remains a source of scope candidates, including its added stock mechanisms.
This catalogue retains the original gameplay breadth while integrating assets,
presentation and lifecycle with their consumers. The
[evidence document](2026-09-11-gameplay-boundary-evidence.md) includes a migration
crosswalk so consolidation does not silently discard earlier scope.

Retain active-YR proof gates for TS fog, veins/veinhole, legacy fences,
inactive locomotors/missions/transports and strategic variants. Preserve required
data/enum round trips; section names alone are not liveness proof. Editor-only
and cheat behavior is not silently part of these goals.

Select work from demonstrated current-match problems and the benefit of finishing
a connected action. Reuse working behavior; merged fixes are not automatically
open again. Reassess after a completed goal and actual match feedback. There is
no “finish all resources, then all units” gate or fixed ranking of every detail.

**Evidence limit:** this examination used main
`ed8f4837910be9329505c3dfc2fc074d9c1f3106`. No game run or fresh native execution
was performed for this revision. Current paths were read directly; existing
native reports were used with their limits and stale current-status claims
excluded. The companion evidence distinguishes supported relationships from
remaining uncertainty. This is a better-grounded planning catalogue, not an
exhaustive retail-parity certification.
