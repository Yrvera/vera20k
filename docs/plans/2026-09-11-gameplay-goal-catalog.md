# Gameplay loops and proposed goal boundaries

Revised 2026-09-11 after examining current production callers, state owners,
downstream consumers and cleanup, alongside existing native research and local
retail data. This replaces the earlier catalogue's default separation of object
variants and implementation responsibilities into individual tasks.

**VERA20k is already playable. These are ways to scope improvements to the
existing game, not a list of missing features or an instruction to rebuild it.**
The target remains active Yuri's Revenge behavior and the intentional
20,000-unit/30-player scale exception in [ENGINE.md](../../ENGINE.md).

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

## The important corrections

| Earlier separation | Revised default |
|---|---|
| TIBTRE; ore/gems; War Miner; Chrono Miner; refinery docking; income | One connected resource-to-income workstream. Standard miner variants belong together. TIBTRE and natural growth supply the same resource authority. A narrower source or docking goal still exercises actual downstream harvesting/income. |
| War Miner versus Chrono Miner | One standard harvesting/refinery goal, checking both variants and their different return, movement and combat interactions. Separate checks do not imply separate owners or sessions. |
| Slave Miner alongside standard miners | A related resource-economy goal with its own workforce lifecycle. Shared resource removal and income rules do not erase its master/slave ownership, deployment and deposit behavior. |
| GI versus Guardian GI deployment | One sustained deployed-infantry combat goal. Desolator radiation, Yuri pulses and Siege Chopper flight retain their own complete effect/flight loops. |
| Transport loading; passenger combat; IFV; Battle Fortress | One mobile passenger lifecycle, including applicable movement, host/passenger firing, unload and destruction. Preserve IFV host firing versus Battle Fortress passenger firing. |
| Spawned aircraft versus launched rockets | One launcher-owned spawn-pool family, with returning-wing and expendable-rocket variants. Boris designation uses a different owner. |
| Special economic buildings grouped by theme | Ore Purifier belongs with resource payout; Industrial Plant and Cloning Vat with production; Bio Reactor with passenger-dependent power; tech-building benefits with capture/use/loss. Grinder intake is its own action. |
| Main menu versus every destination backend | A bounded shell goal owns routes, input and handoffs. Complete skirmish/campaign/network flows own their backends. “Every menu destination works” remains a larger integration claim. |

Detailed evidence and its limits are in
[the boundary examination](2026-09-11-gameplay-boundary-evidence.md).

## Resources and the base

References are identifiers, not an execution order. Each row includes the normal
entry, actual gameplay result, feedback and continuation or cleanup. Variants
named here are starting coverage; exhaustive goals require a full active census.

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

**Battlefield control is a complete selectable goal:** view/select → cursor and
order admission → visible/audible feedback → actual execution → replacement or
cancellation. Include scrolling/bookmarks, single/bandbox/type/control-group
selection, hotkeys, queued/planned waypoint orders, radar navigation and permitted
pause/resume/speed changes. The input owner must reach real U1/U2 or other action
consumers; an admitted command or drawn marker alone does not complete the loop.
Conversely, correct movement after a directly injected command does not establish
the player-facing interaction.

| Ref | Complete outcome and grouped variants | Boundary and continuing result |
|---|---|---|
| U1 | **Units obey movement/replacement orders through arrival.** Walking, driving, ship and hover variants. | Command → mission/destination/installed locomotor → path/turn/traffic/occupancy/crush → arrival → actual next action. Include stop, blocked recovery, replacement and death. Select the affected locomotor set explicitly; sharing this contract does not make all locomotors identical. Ability-specific movement and sorties stay integrated with their parent actions. |
| U2 | **An ordinary engagement resolves from order to aftermath.** Common infantry/vehicle/building/naval/air cases; direct, ballistic, homing and applicable anti-air/air-to-air/strafe/bombing attacks. | Approach/acquire → weapon/range/facing/fire gates → burst/projectile/impact → armor/damage/fear/prone/veterancy/death/attribution → next order/target. Include debris and affected terrain/resource consequences. Reuse shared combat authority; representative units are coverage cases, not duplicate implementations. |
| U3 | **GI and Guardian GI fight through deployment and recovery.** Both sustained deployed-infantry variants. | Deploy input → stance/animation/movement gate → actual deployed targeting/firing → undeploy/reorder/damage/death. Integrate ordinary movement/combat and applicable host interactions. Desolator radiation and Yuri pulses remain with their effects; Siege Chopper with U9. |
| U4 | **Mobile transports and passengers work through carrying, fighting and release.** Land/sea/air transports, IFV and Battle Fortress. | Admission → cargo membership/concealment → movement and applicable firing → unload placement/retry or host destruction → usable passengers. IFV selects the host's weapon; Battle Fortress passenger firing retains its own responsibility. Air-transport landing/exit is a required variant. |
| U5 | **Infantry occupy, fight from and leave buildings.** Civilian garrisons and Soviet Battle Bunker `NABNKR`. | Admission → occupant/ownership state → occupant firing/credit and occupied art → voluntary/forced evacuation, sale or destruction → released actors/building state. Keep presentation and firing with occupancy. |
| U6 | **Yuri Tank Bunker `NATBNK` installs, supports and releases a vehicle.** | Approach/radio → reciprocal single-vehicle link/install → actual combat → release/sale/destruction → link cleared and vehicle usable. This differs from U5's infantry occupation model; the previous catalogue omitted the distinction. |
| U7 | **Harrier and Black Eagle complete repeatable airfield sorties.** Both aircraft and provider/pad variants. | Production/idle → takeoff/attack → return/reservation/landing → rearm → next sortie. Include provider loss/capture, contention, replacement orders and aircraft loss. Flight alone cannot complete this loop. |
| U8 | **Launchers manage spawned aircraft or rockets through repeated attacks.** Carrier/Destroyer and V3/Dreadnought/Boomer. | Parent target → fixed spawn pool → child launch/attack → return/reload or missile regeneration → next attack. Include parent/child/target loss and owner changes. Returning aircraft and expendable missiles are variants of the same pool owner. Boris has a different designation/airstrike lifecycle. |
| U9 | **Hovering airborne units move and fight through required transitions.** Rocketeer, Kirov, Nighthawk, Disc, Siege Chopper and active scenario variants. | Jumpjet/altitude behavior → actual attack/carry order → stop/landing where permitted → next action. Chopper landing/deployment/weapon change/resumed flight is a complete named outcome. Disc drain and Nighthawk passengers integrate their effect/cargo owners; locomotion alone cannot certify them. |
| U10 | **Reversible mind control maintains and releases victims.** Yuri Clone/Prime, Psychic Tower and Master Mind. | Acquisition → controller membership/owner transfer → actual order/house/production/visual consumers → capacity/overload and release/controller/victim loss → required restored state. Group controller variants; permanent Dominator effects and deployment pulses differ. Current full Rust acquisition is incomplete, so native evidence must guide the missing owner path. |
| U11 | **Parasites maintain and release their hosts.** Terror Drone infestation and Giant Squid grapple. | Admission → parasite-host relationship → continuing effects → service/escape/detach/death → cleanup. Group the relationship with explicit variant rules; do not merge Temporal or Magnetron merely because they share the special-weapon dispatcher. |

### Distinct abilities that retain their own effect loops

These are separate candidate goals, not one “finish special units” task. Each
includes ordinary movement/combat and presentation consumers required to exercise
the ability. Exact branch coverage remains work for the selected goal.

| Ability | Complete outcome |
|---|---|
| Gattling Tank/Cannon | Fire → stage progression and actual weapon/damage/feedback changes → interruption/retarget/cooldown → subsequent firing. Check unit/building consumers together. |
| Prism support | Support membership/availability → supported shot/damage/beam → target/provider interruption and cleanup. Prism Tank is an attack consumer, not automatically part of the tower network. |
| Tesla charging/attacks | Trooper/coil relationship → charging/overpower and power exceptions → actual shot/bolt → loss/release/recovery. Ordinary Tesla weapons reuse confirmed effect machinery. |
| Sonic and ordinary laser attacks | Each active weapon's fire → wave/beam and real hit behavior → effect/source/target expiry. Shared rendering does not establish one state machine. |
| C4 and bridge charges | Legal approach/action → explosion and actor/bridge consequences → cleanup. Include world and release consumers. |
| Ivan bombs | Attachment/ownership → fuse/defusing → detonation or removal → cleanup/feedback. This is distinct from an immediate explosive attack. |
| Suicide attacks | Firing/detonation and firer removal → collateral damage/credit/cleanup. Verify active unit variants. |
| Desolator radiation | Deployment/firing → radiation-site/target damage → source changes/expiry → continuing world state. Deployment belongs with this effect. |
| Virus, Chaos Drone and active fire/status effects | Each named effect follows creation → persistent damage or changed behavior → ownership/liveness interactions → expiration. These are selectable loops, not one particle-system task. |
| Yuri/Prime deployment pulses | Deploy → area-damage weapon/target rules → stance recovery → next orders. Group pulse variants separately from reversible capture. |
| Chrono movement | Teleport admission → movement/occupancy transition → recovery → usable next action, including affected infantry/miner variants. R1 includes the miner's complete return outcome. |
| Temporal erasure | Attacker/target link → suspension/progress/competing attackers → interruption or erasure → cleanup. |
| Magnetron | Source/victim locomotor handoff → lift/carry → drop/restoration and terrain/damage effects → cleanup. Jumpjet is a dependency, not proof this is an ordinary aircraft. |
| Boris airstrike | Designation → called aircraft → actual attack/departure → source/target/cancellation cleanup and subsequent use. |
| Dog leap | Legal attack → leap/impact → target loss or interruption → recovery/next order, with applicable disguise/detection interactions. |
| Floating Disc | Flight/attack/drain admission → target-class money/power/defense effects → release/source/target loss → dependent capabilities recover. |
| Spy infiltration | Disguise/approach/detection/admission → target-specific result → spy disposition → actual power/economy/information/production/research consumer. Shared Enter/ownership services do not make this engineer capture. |

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

**Keep coupled stages/variants together:** Chronosphere plus linked ChronoWarp;
American plus Tech Airport paradrops. Neither the first Chronosphere click nor
one drop payload/provider can certify the complete combined goal.

Other candidate goals remain Nuclear Missile (`MultiMissile`), Lightning Storm,
Iron Curtain, Force Shield, Psychic Dominator, Genetic Mutator, Spy Plane and
Psychic Reveal. Each owns its effect sequence, targets, termination and actual
world/house/unit consequences. Weather, protection, transformation and permanent
ownership changes have different state and cleanup despite sharing sidebar code.

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
| F8 | **Save a match and resume it through supported entry points.** | UI → content/version validation → prepared world/fixups → commit → rebuild presentation/reset pacing → continued play. Include shell startup/in-game paths, failure/cancel and active feature state. Current same-content restoration does not establish cold-start loading. |
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

## From catalogue to goal prompt

Before dispatch, identify the selected complete result, current gaps and state
owners/consumers that must change together. Use this shape:

> Make **[action or object family]** behave like active retail YR across
> **[explicit related variants]**. Follow **[entry → state/relationship → actual
> result → continuation/release]**. Include **[required consumers and interruption
> cases]**, reusing existing working behavior. Establish native semantics,
> validate the production path and retain evidence. Complete the named scope;
> uncovered required behavior keeps it open.

GI/GGI deployment is a suitable joint goal, but it is not the complete life of
either unit. “Make the GI work exactly like retail” also includes ordinary
movement/combat/damage, relevant host and special-effect interactions, and
lifecycle. Whole-object acceptance spans entries to verify the combination.

One owner follows evidence, implementation, integration and independent review.
PRs can be smaller while preserving a coherent production path. Promote broken
required prerequisites; do not defer them because another row names them.

Broad parity goals enumerate active types and mode/map/campaign variants,
including reachable civilian, preplaced, reinforcement, elite and special-
acquisition cases. Bounded goals state coverage instead of implying family
parity. Tests based only on current Rust cannot establish retail equivalence.

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
