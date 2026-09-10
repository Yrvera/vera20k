# Proposed gameplay goals for VERA20k

This is a proposed list of work boundaries: what we can ask to work like retail,
what belongs together, and how we would recognize a complete result. It is not a
completion audit, a mandatory sequence, or a replacement status tracker. Existing
working behavior should be verified and reused, not implemented again.

The target remains active Yuri's Revenge `gamemd.exe` behavior under the
[project contract](../../ENGINE.md), with the intentional 20,000-unit/30-player
scale exception. “Retail” here means Yuri's Revenge, including applicable stock
mode and scenario overrides. The catalogue does not authorize implementation or
change the project contract.

## Where to draw the boundary

Use **shared behavior goals, distinctive ability goals, and whole-object
acceptance** together.

- Group objects when they use the same mechanism: ordinary infantry movement,
  refinery docking, or the three TIBTRE variants. Implement shared authority once
  and verify the affected consumers.
- Give a distinctive mechanism its own goal: GI deployment, Terror Drone
  infestation, or Magnetron lifting. Similar names or a shared base class do not
  establish identical behavior.
- “Make the GI work exactly like retail” is a valid whole-object goal. It must
  check how all its shared and special behaviors combine. Passing a generic
  infantry test does not complete the GI.
- Large families below contain explicitly named child goals. Choose a coherent
  child when assigning work; completing one child does not complete the family.
  Avoid either “all infantry” in one enormous task or one independent infantry
  engine per unit.

The practical boundary is **a complete action or object lifecycle that can be
exercised in the game**. Parsing a field, writing a manager, and drawing an effect
are implementation steps within that boundary. They can be separate reviewable
commits, but the gameplay goal stays open until its required consumers work.

| User-facing goal | What belongs in it | How to divide delivery without losing the goal |
|---|---|---|
| **GI works like retail** | Obtain/select it; move, stop and queue orders; acquire and attack targets; deploy/undeploy; apply applicable passenger/garrison interactions; take damage, react, gain veterancy and die; correct art, voices and UI throughout. Include ownership, special-effect and persistence interactions that apply to the GI. | Reuse shared infantry/combat work, add GI deployment, then verify the complete GI and the affected shared consumers. A newly working deployment button alone is not closure. |
| **Miners work like retail** | War Miner and Chrono Miner each complete acquisition, harvesting, return, refinery admission, cargo transfer, release and the next trip, plus their own movement/combat behavior and interruptions. Slave Miner includes deployment, slaves and its distinct economy. | War Miner, Chrono Miner and Slave Miner are named child goals. Share resource, cargo, refinery and house mechanisms where the native behavior supports it. Do not force all three into one miner state machine. |
| **TIBTRE works like retail** | TIBTRE01–03 load correctly; art/timing drives spawning into eligible cells; the new resource is visible and harvestable; lighting, occupancy, immunity and saved state remain coherent. | Group the three variants. Include resource and miner integration; ordinary trees and the legacy veinhole are different scopes. |
| **Main menu works like retail** | Startup, appearance, input, sound, every retained destination, back/cancel and exit. Each destination must enter its actual flow. | Shell presentation/navigation can land first. Skirmish, campaign, settings and other destination owners can deliver separately; missing backends keep the whole menu goal open. Explicitly settle any changed online-service scope. |

## What completion means

For each selected goal, establish the full active behavior and affected types from
native bodies/callers, retail data and actual scenarios. Record the exact binary,
rules/mode/map inputs and coverage. INI names are useful discovery clues; section
presence, registration, or a plausible screenshot alone is not runtime proof.

Follow the normal player, AI or scenario trigger through authoritative production
ticks to its gameplay result, rendering/audio/UI feedback and stable resulting
state. Cover the relevant alternatives: repetition, queued/replaced/cancelled
orders, blocked admission, source/target loss, ownership change, and cleanup.
Check affected save/restore, deterministic ordering, RNG, hashes and replay state
as part of the mechanism. Required integration is part of the goal even when it
crosses an old phase boundary.

The checks in the tables are **starting scenarios**, not exhaustive parity proofs.
Expand them into a coverage set for the selected goal. Distinguish native behavior
established, Rust regression tested, and parity demonstrated; retain the evidence
and limits. Unknown required behavior or an outstanding required discrepancy
keeps an “exactly like retail” goal open. Use a fresh independent critic for
substantial changes, as required by ENGINE.md.

One owner follows each coherent mechanism through evidence, implementation,
integration and review. Refactor the responsible code when that work needs it;
there is no prerequisite project to refactor the whole engine or install new
analysis tools.

## Catalogue

Identifiers are references, not an execution order. A row is either a bounded
goal or a family with named children. Every row includes its own production
integration and presentation; the shared presentation rows do not defer those
obligations to a later phase.

### P — Screens and player interaction

| ID | Goal and boundary | Starting acceptance scenario |
|---|---|---|
| P1 | **Open the game, use the main menu, and exit.** Startup, localization/fonts, focus/window behavior, shell transitions and real destinations. | Start the application, use each retained button, return/cancel, change focus, and exit cleanly. Destination flows below remain dependencies of full menu completion. |
| P2 | **Change settings and keep them.** Controls/hotkeys, display, audio and gameplay options; shared shell/in-game settings authority. | Apply, cancel and reset changes; use them in a match; restart and check persistence. |
| P3 | **Configure and start a stock skirmish.** Map catalogue/preview, factions, houses, teams, colors, difficulty, mode/options and loading. | Launch selected options into the actual correct starting state, leave, then launch another map without leaked state. |
| P4 | **Generate a map and play it.** Generator options and RNG, preview, saved/generated map identity and match launch. | The preview and played map agree; starts, resources, movement and construction are usable. |
| P5 | **Control the battlefield view and selection.** Scrolling, camera bookmarks, selection/bandbox/type/group rules, health/pips and cursor feedback. | Select and inspect different object types across terrain and visibility changes, then issue real orders. Additional VERA zoom behavior needs a separate explicit contract; it is not implicit retail parity. |
| P6 | **Issue orders and control match pacing.** Move, attack, stop, scatter, guard and applicable planning/waypoint interactions; command admission and feedback; pause/resume and game-speed controls where the session permits them. | Units obey queued/replaced commands through later ticks; markers and acknowledgements match the outcome. Pause, resume and change speed during interacting actions; timers, input and presentation follow the session's native pacing policy. |
| P7 | **Use the sidebar and radar.** Cameos/tabs, queues, progress/hold/cancel, power display, minimap information/navigation and action modes. | Build/place, repair, sell, deploy and set applicable rally behavior through the real UI; radar responds to actual power/visibility state. Shares economy authority with E goals. |
| P8 | **Hear gameplay and play media.** Named children: positional sound/unit voices/EVA arbitration; music transitions; movies/briefing speech/subtitles/credits. | Competing announcements and positional events play correctly; media stays synchronized, skips/returns correctly and honors settings. Each consuming feature also verifies its own feedback. |
| P9 | **Finish or leave a match.** Victory/defeat, surrender, restart, quit/cancel, scores/statistics/results and teardown. | Reach an outcome through real play, check results, then rematch or return to the menu without retained session state. |

### W — Playable world and resources

| ID | Goal and boundary | Starting acceptance scenario |
|---|---|---|
| W1 | **Load retail content into a correct playable map.** Asset/archive/loose-file precedence, localization/theater assets, rules/art/AI and applicable mode/map overrides; ground, ramps, cliffs, shores, bounds and starts. | Load representative stock theaters and overridden scenarios; displayed terrain, click positions, occupancy and actual traversal agree. Trace incorrect values back through the real loader. |
| W2 | **Ordinary trees, rocks and scenery work.** Terrain identity, occupied cells, drawing, targeting and applicable damage/crush/fire/destruction. | Interact with representative destructible and immune scenery; removal updates the map, movement and presentation together. |
| W3 | **Ore and gem fields work.** Resource-cell quantity/type, growth/spread, appearance, depletion and applicable damage. Cargo and credit conversion are owned by E goals. | Harvest, exhaust and revisit a field; growth and remaining quantities agree with native behavior and what miners can consume. |
| W4 | **TIBTRE01–03 work.** Animated resource-spawning terrain, including loading/art timing, eligible cells, light and terrain lifecycle. | A normally loaded TIBTRE produces visible ore that a normally controlled miner harvests; check all three variants and relevant blocked/full surroundings. |
| W5 | **Bridges work through damage and repair.** Named children: high-bridge and low-bridge lifecycles; topology, traversal, occupants, collapse and engineer/repair-hut interactions. | Cross and fight around a bridge, collapse it with occupants present, repair it, then traverse it again; paths, targeting and visuals agree. |
| W6 | **Walls and active gates work.** Buildable overlays, connections, placement, traversal, damage and removal; establish active YR gate use before including legacy behavior. | Build or load adjoining segments, obstruct a route, destroy/open the applicable obstacle and use the changed route. |
| W7 | **Exploration, radar information and shared vision work.** Per-viewer sight/shroud, allied/observer policy and information changes. Ability owners supply Gap/SpySat/reveal effects. | Scout, leave, return and change information sources; tactical display, radar, selection and targeting reveal only the information they should. |
| W8 | **Crates produce complete outcomes.** Stock spawn/regeneration, encounter/pickup, every active reward class and downstream effects. | Pick up crates through ordinary movement; verify reward, feedback, removal and later economy/combat consequences. Ordinary skirmish scope, not optional polish. |
| W9 | **The battlefield composes correctly.** Shared TMP/SHP/voxel projection, sort/depth/masks, palette/remap, elevation, shadows, lighting, smudges and transient effects. | Compare real mixed scenes at different elevations and overlaps, including damaged objects and effects, using appropriate retail captures/GPU output. Each object goal still owns its visible correctness. |

### E — Harvesting, economy and the base

| ID | Goal and boundary | Starting acceptance scenario |
|---|---|---|
| E1 | **Money, cargo value and storage remain correct.** Income/spending order, transactions, displayed credits, ore/gem conversion, storage and applicable loss. | Income, repair and production compete in the same ticks; depletion, cancellation and storage/provider loss leave correct authoritative and displayed balances. |
| E2 | **War Miner works like retail.** Complete harvesting/refinery cycle plus its ordinary vehicle and combat behavior. | Produce/select a miner, harvest ore and gems, return/unload, repeat; replace orders, block the refinery, deplete the field and destroy/capture relevant actors. |
| E3 | **Chrono Miner works like retail.** Complete economy loop plus its distinct return/teleport and recovery behavior. | Repeat harvesting with near/far/blocked/lost refinery cases; compare occupancy, timing, cargo and the next trip. Reuse confirmed shared economy mechanisms. |
| E4 | **Slave Miner and slaves work like retail.** Mobile/deployed forms, worker creation, assignments, harvesting, income, replacement/release and full manager lifecycle. | Deploy, work, relocate and lose/control-change the miner or slaves; verify workers, credits, ownership and cleanup throughout. |
| E5 | **Refineries admit, unload and release miners correctly.** Shared radio/reservation/docking authority, cargo transfer and factory-created refinery/miner relationships where applicable. | Several miners contend for a dock; interrupt admission/unloading and lose or transfer the refinery; every affected miner continues correctly. E2/E3 verify their full cycles. |
| E6 | **Build eligible units and structures.** Tech tree/prerequisites/build limits, factory lifecycle, primary/parallel queues, costs/timing, hold/cancel and completed-product ownership. | Queue from the sidebar, change funding/power/providers and cancel/restart; produce actual eligible objects. Enumerate faction/mode and captured-factory differences. |
| E7 | **Place and activate buildings.** Foundations, adjacency, legal/illegal placement, buildup, occupancy, activation and applicable upgrades/slots. | Place a completed building through the UI, finish it and use its function; held/unplaced/destroyed buildings must not act as active map structures. |
| E8 | **Factory products enter the world and follow orders.** Exit reservation, blocked-exit recovery, placement, rally and first mission. | Block/unblock exits, queue multiple products and remove/capture the factory; products enter exactly once and continue through subsequent ticks. |
| E9 | **MCVs and construction yards work through deployment.** Deployment admission, transformation, house/build authority and applicable reverse transition. | Deploy through normal orders, build from the resulting yard, interrupt/fail/retry in relevant terrain and lifecycle cases. Validate movement continuation as well as the first deploy attempt. |
| E10 | **Repair and sell the base correctly.** Named children: player-paid building repair; selling/refunds/teardown and applicable survivors. | Repair while spending elsewhere; sell/cancel where supported and destroy/capture a target mid-action; money, occupancy, capabilities and UI update together. |
| E11 | **Power providers and consumers work together.** Generation, demand, low-power/blackout transitions, radar and all active dependent consumers. | Add/lose/capture providers while producing, defending or charging powers; consumers stop/resume with correct timing and feedback. |
| E12 | **Special economic buildings deliver their benefits.** Separate named goals: Grinder; Cloning Vat; Bio Reactor; Ore Purifier; Industrial Plant. | Exercise each through its real admission/production/income consumer, then sell/destroy/capture/disable it and verify all benefits, passengers/products and registrations. |
| E13 | **Tech buildings deliver their benefits.** Separate named goals: Oil Derrick; Secret Lab; Tech Hospital; Machine Shop. Tech Airport drop is S8. | Capture and use each benefit, then change ownership/liveness. Hospital/Machine Shop grant passive house benefits; do not substitute an enter-to-heal dock. |
| E14 | **Robot Tanks respond to their controller.** Robot Control Center and applicable power/provider dependencies, activation/deactivation and recovery. | Produce and command Robot Tanks; change controller/power availability during movement/combat and verify resumed behavior. Whole Robot Tank acceptance also covers its movement/combat. |

### U — Shared unit behavior and services

| ID | Goal and boundary | Starting acceptance scenario |
|---|---|---|
| U1 | **Ground units move and arrive correctly.** Walking, driving and amphibious children: admission, paths, facing/speed, occupancy, traffic, retries, scatter and crushing. | A GI, tank and applicable amphibious infantry obey orders across obstacles and congestion; arrival, replacement and crush consequences remain correct on later ticks. |
| U2 | **Ships and hover units navigate correctly.** Water/shore/land permissions, movement, occupancy, collision and appropriate destruction. | Move naval units and hover transports around shores and blockages; verify each supported terrain transition, including Robot Tank consumers where applicable. |
| U3 | **Ordinary combat resolves complete attacks.** Target acquisition/legality and attack/guard/hunt/attack-move behavior; weapon selection, facing, range, fire gates, ROF/burst/ammo, projectile flight/collision and hits. Named child scenarios cover direct, ballistic, homing, anti-air and air-to-air attacks. | Issue attacks and allow auto-acquisition; move/lose targets and replace orders before and during firing. Compare the resolved damage and presentation, not only projectile creation. |
| U4 | **Damage, survival and destruction work.** Armor/Verses/immunities/healing, fear/prone, experience/elite changes, attribution, deaths, applicable survivors, debris, crash/sink and environment damage. | Damage/promote/kill representative infantry, vehicles, ships, aircraft and buildings; verify downstream ownership, space, scores and effect cleanup. |
| U5 | **Cloak, detection and disguise work.** Named children: cloaking/sensors; Mirage disguise; Spy disguise. Share per-viewer information and targeting authority with W7. | Observe and attack with eligible/ineligible detectors across exposure and ownership changes; displayed information and permitted attacks agree. Spy infiltration itself is A16. |
| U6 | **Engineers capture and repair their targets.** Capture/repair children, approach/entry admission, engineer disposition and all ownership consumers. | Order each faction's engineer against eligible/ineligible targets, including damage, blocked access and target loss; verify transferred capabilities and map state. Bridge repair integrates W5. |
| U7 | **Transports load and unload passengers.** Capacity/admission, cargo identity, movement, unloading placement, interruption, transporter destruction and applicable survivors. | Load different passengers into land/sea/air transport examples, relocate, unload in constrained space and lose the transporter mid-operation. |
| U8 | **Service facilities repair/rearm and release units.** Service admission/reservations, costs/timers, repair/rearm effects and departure; only active YR services. | Contending units enter service, complete it and return to play; interrupt or lose the provider. Shared docking authority is implemented once with relevant consumer checks. |
| U9 | **Airfield aircraft complete sorties.** Production, airfield reservation, takeoff/flight/attack, return/approach/landing, rearm and repeat. | Harrier and Black Eagle complete repeated sorties; lose or capture the airfield, contend for space and interrupt their orders. |
| U10 | **Hovering airborne units operate correctly.** Shared Jumpjet movement with separate acceptance for Rocketeer, Kirov, Nighthawk, Floating Disc and Siege Chopper; relevant stock scenario variants too. | Move/attack/load/deploy where applicable; verify altitude, landing permissions, target pursuit, source loss and shadows. Shared locomotion does not imply shared weapons or abilities. |

Aircraft acceptance explicitly includes each active air-to-air, strafing and
bombing behavior in its actual consumer: U9/U10 for normal aircraft, A18 for
spawned aircraft and A20 for called airstrikes. A flight-only demonstration or
one successful missile sortie cannot close those distinct attack modes.

### A — Distinctive abilities

Each named child in this section is independently selectable. Its whole-object
acceptance also includes the shared U/E/W behavior that the object actually uses.

| ID | Goal and boundary | Starting acceptance scenario |
|---|---|---|
| A1 | **GI deploys and undeploys correctly.** Stance transition, weapon/art changes, command and combat interactions. | Deploy, fight, change orders and undeploy; include blocked/interrupted transitions and relevant damage/elite states. |
| A2 | **Guardian GI deploys and undeploys correctly.** Its own restrictions, weapons and targeting with shared stance machinery where justified. | Exercise deployed/undeployed attacks and movement admission against applicable ground/air targets. |
| A3 | **Siege Chopper deploys and resumes flight correctly.** Air/ground transition, weapon switch, occupancy and recovery. | Land/deploy/fire, receive replacement orders and take off; interrupt at each transition. |
| A4 | **Passengers fight correctly from their host.** Separate goals: civilian garrisons; Battle Bunker; Battle Fortress open-topped combat; IFV passenger-dependent weapons. | Enter, fire, change passengers and lose the host; verify range, attribution, ownership, weapon/art changes and passenger disposition for each host type. |
| A5 | **Prism support and attacks work.** Network support, firing ownership/timing and affected tower/tank attack consumers. | Supporting towers engage, change availability and lose targets/providers; validate beams and resolved damage. Do not assume the tank participates in the tower network. |
| A6 | **Tesla weapons and troop charging work.** Ordinary Tesla attacks plus the distinct Tesla Coil charging/overpower interaction. | Fire Tesla units/coils and change assisting troops or power during an engagement; compare firing and electric effects. |
| A7 | **Gattling weapons advance and reset correctly.** Weapon stages, timing, target/order interruptions and applicable unit/building consumers. | Sustain fire, interrupt, switch targets and resume; stage, damage and feedback agree. |
| A8 | **Sonic and laser attacks work.** Separate goals for active sonic-wave and laser mechanisms; their actual collision/damage/lifetime consumers. | Fire representative weapons across relevant targets/terrain and lose the source/target during the effect. Disc-specific behavior remains A22. |
| A9 | **Demolition actions work.** Separate goals: C4/bridge charges; Ivan attachment/fuse/defusing; active suicide attacks. | Use Tanya/SEAL, Ivan and suicide-unit examples through targeting, detonation and collateral effects; change target/owner/liveness while pending. |
| A10 | **Persistent damage and behavior effects work.** Separate goals: Desolator radiation; Virus effects; Chaos Drone berserk; other active fire/status mechanisms. Own source-to-region/target lifecycle and expiry, using shared damage authority. | Create the effect; units enter/stay/leave or change ownership; lose the source and wait for expiry. Verify immunities, behavior, damage and visible residue. |
| A11 | **Terror Drones infest and leave hosts correctly.** Attack admission, infestation, ongoing effects, removal and source/host lifecycle. | Infest eligible/ineligible targets, service or destroy the host and exercise ownership changes; no orphaned controller/effect remains. |
| A12 | **Giant Squids grab and release correctly.** Parasite/host interaction, ongoing attack and applicable escape/removal. | Grab a ship, interrupt or remove either actor and verify resulting movement, damage and cleanup. |
| A13 | **Mind control and release work.** Shared ownership/capture authority; named acceptance for Yuri Clone, Yuri Prime, Master Mind and Psychic Tower, including applicable overload and building-capture interactions. | Capture and release through each active route, change owner or destroy controllers/victims, and verify every affected house/production/command consumer. Deployment pulses are A23; Psychic Dominator is S6. |
| A14 | **Chrono units teleport and recover correctly.** Chrono Legionnaire movement and affected Chrono Miner/special infantry consumers. | Teleport, replace orders and lose an actor during recovery; occupancy, timing and subsequent actions remain correct. Prove availability of special variants in stock content. |
| A15 | **Temporal attacks erase targets correctly.** Admission, temporal progress, interruption and source/target cleanup. | Start, interrupt, resume and complete erasure with relevant competing effects; verify target state, visuals and final removal. |
| A16 | **Spies infiltrate and deliver each reward.** Approach/admission, target-class effects, spy disposition and actual downstream production/research/economy/information consumers. | Infiltrate each active target class and verify the reward in actual use, then destroy/capture affected providers. Disguise/detection integrates U5. |
| A17 | **Magnetrons lift, carry and drop correctly.** Target eligibility, locomotor handoff, suspended behavior, height/rendering, drop consequences and cleanup. | Lift/drop eligible victims around terrain and bridges; remove or transfer source/target while suspended. |
| A18 | **Spawned aircraft complete their service/attack lifecycle.** Carrier/Hornet and Destroyer/ASW children; creation, target handoff, attack, return/replacement and manager teardown. | Repeated attacks with aircraft/source/target losses; no duplicate, stranded or ownerless spawned actors. |
| A19 | **Launched rockets complete attacks.** V3, Dreadnought and Boomer children; launch/flight/interception where applicable, impact and launcher/spawn lifecycle. | Launch repeatedly against changing targets and remove launchers or missiles mid-flight; compare impact and cleanup. |
| A20 | **Boris calls and completes airstrikes.** Designation, called aircraft, arrival/attack/departure and cancellation/liveness rules. | Call a strike through normal targeting, then change source/target state before and during aircraft arrival. |
| A21 | **Dogs leap, hit and recover correctly.** Leap attack, target legality, impact, interruption and return to ordinary infantry behavior. | Attack moving/ineligible targets and lose the target mid-action; compare position, damage, animation and next orders. |
| A22 | **Floating Discs attack and drain correctly.** Disc laser and distinct target-class drain effects, including affected money/power/building consumers and release. | Drain each active target class, interrupt/change ownership/destroy either actor and verify every dependent effect clears or persists as native behavior requires. |
| A23 | **Yuri deployment pulses work.** Yuri Clone and Yuri Prime children: deployment, area-damage weapon/target rules, stance recovery and feedback. This is a distinct damage action from mind control. | Deploy near mixed targets, resolve the area hit and follow the unit through recovery and subsequent orders; interrupt or remove actors at relevant points. |

### S — Strategic powers and information structures

Each launched power includes provider availability, charge/readiness, targeting,
launch, effect completion and feedback. Implement that shared lifecycle once and
test each power's exceptions, including applicable provider loss/capture, low
power, repeated use, cancellation and persistence.

| ID | Complete goal | Starting acceptance scenario |
|---|---|---|
| S1 | **Nuclear Missile works.** The active `MultiMissile` route, launch/descent/impact, damage and effects. | Charge and launch from normal UI; verify target-area results and relevant provider/target changes. EMPulse-named nuke data does not imply a separate active EMP superweapon. |
| S2 | **Lightning Storm works.** Targeting, storm scheduling/weather, damage, world/presentation effects and termination. | Start a storm, observe its full duration and effects on actual objects, then continue the match after it ends. |
| S3 | **Iron Curtain works.** Eligible targets, application, duration, damage interaction and release. | Apply to mixed eligible/ineligible targets and fight during/after protection. |
| S4 | **Force Shield works.** Its own targeting, protection and house/power consequences. | Protect a base area, change affected structures and continue through all resulting state transitions. |
| S5 | **Chronosphere and ChronoWarp work together.** Both linked clicks, selection, destination legality, transport and aftermath. | Select a mixed area, choose valid/invalid destinations, cancel where permitted and verify all affected objects after transfer. |
| S6 | **Psychic Dominator works.** Full effect sequence, damage/capture rules and resulting ownership lifecycle. | Use on mixed targets, verify survivors/ownership and subsequent orders, including controller/provider transitions. |
| S7 | **Genetic Mutator works.** Target qualification, transformations, resulting actors and cleanup. | Use on mixed infantry states/types and verify the resulting actors and ownership through later play. |
| S8 | **Paradrops work.** American and Tech Airport variants are separate acceptance children sharing justified infrastructure. | Obtain/charge the power, drop at the chosen area and command surviving passengers; exercise aircraft loss and constrained landing areas. |
| S9 | **Spy Plane reconnaissance works.** Charge/targeting, aircraft approach/overflight/departure and actual information effects. | Reconnoiter a hidden area and verify each viewer's information and aircraft lifecycle. |
| S10 | **Psychic Reveal works.** Targeting, information effect and its actual duration/lifecycle rules. | Reveal an area and verify targeting/radar/tactical consumers throughout the effect. |
| S11 | **Gap Generators, Spy Satellites and Psychic Sensors work.** Three separately selectable structure goals; their active information/jamming/warning benefits and power/ownership lifecycle. | Gain/lose each provider and exercise the corresponding visibility, radar or enemy-order warning effect. Share information authority with W7. |

### G — Opponents, scripted scenarios and campaigns

| ID | Goal and boundary | Starting acceptance scenario |
|---|---|---|
| G1 | **A skirmish AI develops and maintains its base.** Difficulty/IQ/cadence, economy/spending, factories, placement/rebuilding and defense. Named child outcomes can target economy, build decisions and defense while exercising actual matches. | Start representative faction/difficulty/mode matches, disrupt income/production/base structures and observe recovery and spending under real gameplay. |
| G2 | **AI teams form and complete their missions.** Taskforces/recruitment, membership/ownership, formations, script steps/branching, AITrigger selection and team dissolution. | A team recruits actual units, follows its script, loses/replaces members and finishes or aborts correctly; reuse one team/script authority with scenarios. |
| G3 | **Stock scripted scenarios execute correctly.** Map flags/tags/variables, event predicates, ordered actions, reinforcements, objectives, cinematics and outcome. Include convoy/follower and special mission behavior where active scenarios establish it. | Play a scenario through interacting triggers and save/load mid-script; verify actual units, events and outcome, not only an isolated VM action. |
| G4 | **Campaigns launch and progress correctly.** Selection/difficulty/briefing, mission launch, objectives/outcome, next-mission/carryover and persistent progress. | Start each supported campaign route, complete/fail/retry missions and resume progress after restart; integrate G3, P8 and P9. |

### N — Persistence and multiplayer

| ID | Goal and boundary | Starting acceptance scenario |
|---|---|---|
| N1 | **Save and resume a match through the UI.** Slots/metadata, shell and in-game loading, format/version policy, failure/cancel handling and full state restoration. | Save during interacting movement/combat/economy/scripts, restart/load and continue. Every feature owns persistence of its state; this row owns the usable complete flow. |
| N2 | **Record and replay a match.** Usable entry point, command timing, replay identity/compatibility, deterministic progression and actionable divergence diagnostics. | Record ordinary play, restart and play it back; compare saved native/Rust evidence within its stated coverage. Deterministic agreement alone does not prove retail behavior. |
| N3 | **Host, join, play, observe and leave a LAN match.** Named children: host/join through a first completed match; negotiate/transfer content and launch it; play through network delay/stalls; communicate/change alliances/observe; handle disconnect/timeout/recovery or abort. Includes lobby/modes/readiness, content/options/seed agreement and synchronized commands. | Real peers complete each child flow in an actual match, including results and teardown. Keep one protocol owner; a local simulation test or working lobby alone does not complete the family. |
| N4 | **Find and join an online game.** Service/session/account/chat/discovery and actual multiplayer handoff. | Requires a separate explicit decision on supported service or replacement and any intended departures from retail. Implementing LAN alone does not complete online or that menu destination. |

## Roster coverage without duplicating the engine

The family list must be paired with a whole-object acceptance pass when assigning
an object goal. For example: GI → U1/U3/U4 plus A1 and applicable host/visibility
interactions; Chrono Miner → E1/E3/E5 plus U1/A14; TIBTRE → W1/W3/W4/W9 with a
real E2/E3 consumer. These are starting dependency examples, not exhaustive sets.

At the start of a family goal, enumerate its affected active types using the
retail registries, fields, callers and stock mode/map/campaign overrides. Include
civilian, preplaced, reinforcement, elite and special-acquisition variants where
reachable; the build menu is not the whole roster. Keep this bounded coverage
with the goal's evidence, rather than establishing another permanent global
completion ledger. A type's registration is necessary context, not proof that
every section/behavior is exercised in stock play.

Ordinary infantry, tanks, ships, buildings and aircraft need their complete
object checks even when they introduce no unique ability. They do not each need
an independently implemented engine mechanism. Conversely, two objects sharing
movement do not automatically share weapons, passengers, deployment or ownership
rules. Any active behavior found outside these families expands the appropriate
goal before it can be called exhaustive.

## Standing engine obligations and excluded legacy paths

Coordinates/numeric behavior, scheduler/timer/RNG order, identity and reference
expiry, authoritative state, ownership, command admission, serialization/hash and
scale constraints apply to every affected goal. Performance at the intentional
scale target must remain part of relevant validation. Rendering/audio/input
correctness is validated through real consumers. A missing shared prerequisite
can be delivered first, but its consumer goal remains open until integrated.

Do not turn this into a new “finish all infrastructure first” phase. Existing
engine obligations and focused diagnostic/native-comparison tools support the
selected behavior. They do not need to be rewritten to adopt this catalogue.

Retain the existing active-YR proof gate for legacy TS fog, veins/veinhole,
Firestorm fences, mech/drop-pod/subterranean paths, inactive mission slots,
legacy transports, and inactive strategic-power variants. Keep required data and
enum round trips even when a runtime mechanism is excluded. Ion Cannon section
presence does not make it an active registered power; shared live nuke machinery
must not be excluded because some keys or relatives have legacy names. Editor-
only and cheat behavior is not silently added to the gameplay target.

## Coverage against the existing phase inventory

This is a family-level crosswalk to the
[336-row implementation inventory](2026-07-30-clean-slate-system-implementation-order.md),
including its added stock-mechanism notes. It preserves coverage references; it
is not a fresh finding that every old row is active or correct, nor a new ordering.

| Existing area | Catalogue home |
|---|---|
| Phase 0 — core contracts | Standing obligations; P6/P9 and N1–N3 for full command/session/save/replay consumers. |
| Phases 1–2 — assets, rules and map construction | W1/W9 and the data-to-production path of every selected goal; P1/P8 for shell/media consumers. |
| Phase 3 — world and terrain | W1–W9, E1–E5 and bridge/wall consumers. |
| Phase 4 — instances, shell and battlefield | P1/P3/P5/P9, W1/W2/W9 and object lifecycle checks throughout E/U/A. |
| Phase 5 — interaction and movement | P5–P7, U1/U2/U5, W7/W9. |
| Phase 6 — combat, effects and audio | U3–U5, A goals, W5/W9, P8. |
| Phase 7 — harvesting and economy | E1–E5, W3/W4, P8. Harvester return is part of the Harvest loop, not a new live Return mission. |
| Phase 8 — base production, power and radar | E6–E11, W6, P7. |
| Phase 9 — capture, transport, service and locomotion | U2/U5–U10, E7, A3/A14/A17–A19; ownership consumers across goals. |
| Phase 10 — AI, teams, triggers and outcomes | G1–G4, P9 and scenario/save consumers. |
| Phase 11 — special units and faction mechanics | A1–A23, E4/E12, S11, with their shared U/W/P consumers. |
| Phase 12 — strategic actions | S1–S11 and each affected world/unit/house/UI consumer. |
| Phase 13 — save/load and multiplayer | N1–N3, P9; replay explicitly has N2. |
| Phase 14 — shell, random maps, crates, campaign and online | P1–P4/P8, W8, G3/G4, N4. Late placement in the old inventory does not make crates optional. |
| Phase 15 — inactive/conditional mechanisms | Active-YR proof gate above; preserve compatible data without inventing live behavior. |
| Added stock mechanisms | Spy infiltration A16; neutral-building income/unlocks E13; Robot control E14; dog leap A21; Boris strike A20; Tesla charging A6; Disc drains A22. |
| Other named consumers | Industrial Plant E12; survivor creation U4/E10/U7; suicide attacks A9; Psychic Sensor S11; Siege Chopper A3/U10. |

## How to start using this

Choose one named outcome at a time, establish its current gaps, and keep ownership
through completion. **“Make the GI work exactly like retail” is a reasonable first
object goal**: it exposes shared movement/combat/stance integration while giving
us one visible unit to assess. It is a proposed starting point, not a claim that
the GI is currently the largest gap or authorization to interrupt existing work.

**“TIBTRE01–03 work through ore generation and actual harvesting”** is a useful
terrain/economy goal. **“War Miner completes its entire retail lifecycle”** is a
larger economy/object goal. They overlap in shared consumers, so assign changes
to those owners coherently instead of having concurrent tasks rewrite the same
resource/miner authority.

Keep the existing phase inventory as a reference for omitted responsibilities and
dependencies. Future task titles can name the player result, with smaller PRs
that preserve a working production path. The important change is sustained
ownership and demonstrated integration; simply renaming phases as features would
not prevent the same failures.

## Basis and limits of this proposal

Prepared against main commit `ed8f4837910be9329505c3dfc2fc074d9c1f3106` on
2026-09-11, using ENGINE.md, the existing inventory, current source and existing
local retail data. No game execution or exhaustive native audit was performed
for this catalogue, and it assigns no completion percentages.

Examples grounded in the current code include
[terrain art inputs](../../src/rules/terrain_asset_catalog.rs),
[terrain spawning](../../src/sim/terrain_spawn.rs),
[production](../../src/sim/production),
[team scripts](../../src/sim/team_script_vm.rs),
[scenario triggers](../../src/sim/trigger_runtime.rs), and
[main-menu destinations](../../src/ui/main_menu_dialogs.rs).
TIBTRE art timing feeding simulation is a concrete reason its goal must cross
loading, simulation and presentation.

Local retail inputs were read from the main checkout's existing `ini/` directory,
including `rulesmd.ini`, `artmd.ini` and the mode/scenario catalogues. Resolve that
machine-local directory through `git worktree list`; it is not copied into this
document worktree. These inputs establish names and data relationships here,
not exact executable behavior. Selected implementation goals still need their
own native evidence and comparisons.
