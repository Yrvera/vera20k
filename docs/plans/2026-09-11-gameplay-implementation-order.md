# Suggested order for future implementation work

Use this ordered list to choose future goals from the [full catalogue](2026-09-11-gameplay-goal-catalog.md).
It covers gameplay, movement, presentation and application flows. VERA20k is already
playable: these are priorities for remaining work, not a claim that each item is missing.
The order is a planning recommendation based on the catalogue's shared-work findings
and everyday match use, not a measured defect/severity ranking or estimated schedule.

Start near the top and select the first meaningful demonstrated gap. Inspect current
source and existing evidence, preserve working behavior, and compose its concrete goal
using the catalogue. Do not start a new refactor merely to visit every row. A serious
current problem, required dependency or explicit user priority moves its work forward;
for example a broken Chrono return belongs with the resource goal, and a LAN-focused
request need not wait for campaign or every special weapon.

Earlier placement favors ordinary match interactions and broad shared consumers.
Visual composition, AI and save are deliberately interleaved with gameplay. Later
placement of specialized abilities or application flows is not permission to defer
one needed by the current goal. Settings, sound, rendering, persistence, cleanup and
determinism remain obligations of each affected implementation from the start.

These are ordered selection entries, not mandatory separate goal sessions or phases.
Nearby entries are not automatically one implementation group. Combine them when the
catalogue/source establishes shared research and changes; a large authorized goal can
span several entries and PRs. Repeated references mark different portions of a broad
scope or shared consumer, not duplicate implementations. Finishing an early portion
cannot close the full referenced family; later entries reuse supported work.

| Order | Work to select | Catalogue coverage | Intended result / grouping boundary |
|---|---|---|---|
| 1 | Ground orders and movement | [C1](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships), [U1](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships), [M1](2026-09-11-gameplay-goal-catalog.md#movement-and-locomotion-implementation-groups), [M2](2026-09-11-gameplay-goal-catalog.md#movement-and-locomotion-implementation-groups) | Selection/orders through driving, ship tracks, walking, blockage and usable arrival; group shared fixes. |
| 2 | Standard resource economy | [R1](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | TIBTRE/ore/gems through War and Chrono harvesting, refinery income and the next trip; include required teleport work. |
| 3 | Purchases and delivery | [B1](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Shared factory lifecycle through usable units or placed buildings, including affected variants and queue continuation. |
| 4 | MCV deployment and recovery | [B2](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Stock variants through usable bases and applicable reverse conversion. |
| 5 | Power and provider-dependent behavior | [B5](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Sources, occupants and capability shutdown/restoration through actual consumers. |
| 6 | Ordinary combat | [U2](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Order/targeting through firing, impact, damage, death and subsequent action; select shared changed branches. |
| 7 | Vision and reveal publication | [W4](2026-09-11-gameplay-goal-catalog.md#world-interaction-and-information), [S10](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Per-viewer knowledge and complete Psychic Reveal behavior; coordinate shared publication while retaining source-specific lifetime and admission. |
| 8 | World depth and draw order | [V4](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups) | Correct mixed terrain, bridge, building, sprite and voxel occlusion in motion. |
| 9 | World lighting and palettes | [V1](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups) | Source/profile changes through actual lit terrain, actors and animations. |
| 10 | Voxel appearance and shadows | [V2](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups), [V3](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups) | Facing/parts/slopes through body composition; remaining shadow shape, placement and darkening across the selected full scope. Combine shared work, preserving distinct geometry producers. |
| 11 | GI and Guardian GI deployment | [U3](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Both sustained-stance variants through firing, interruption and resumed movement. |
| 12 | AI base decisions and recovery | [F4](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Production/deployment/placement decisions through a functioning base; retain the rest of full-opponent scope when requested. |
| 13 | AI Team execution | [F4](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Recruitment, scripts and real combat effects through replenishment; integrate with existing base decisions. |
| 14 | Save and resume | [F8](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Group quickload/panel restoration; include cold-start content preparation for the complete supported loading flow. |
| 15 | Building repair | [B3](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Spending/healing through interruption and continued building operation. |
| 16 | Building sale | [B4](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Dependents, refund and capability changes through usable released units and ground. |
| 17 | Engineer capture and building benefits | [B6](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Capture through actual benefit and later revocation, preserving distinct benefit owners. |
| 18 | Unit repair/service visits | [B7](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Admission, contention and spending through release and the next order or mining trip. |
| 19 | Bridge traversal, collapse and repair | [W1](2026-09-11-gameplay-goal-catalog.md#world-interaction-and-information) | Connected topology, occupancy, engineer and visible-world lifecycle. |
| 20 | Scenery lifecycle | [W2](2026-09-11-gameplay-goal-catalog.md#world-interaction-and-information) | Loading/occupation through active damage/removal and spatial/visual cleanup. |
| 21 | Walls and active gates | [W3](2026-09-11-gameplay-goal-catalog.md#world-interaction-and-information) | Placement and connected obstacle behavior through passage changes and removal. |
| 22 | Cargo release and retry across routes | [U4](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships), [U5](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships), [S8](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Shared departure work across vehicles, landed aircraft, garrisons and paradrops; route-specific rules stay explicit. |
| 23 | Transport boarding, carrying and passenger-dependent weapons | [U4](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Complete mobile cargo behavior; distinguish IFV host weapons from Battle Fortress passenger firing. |
| 24 | Infantry garrisons and Battle Bunkers | [U5](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Occupation through firing, ownership changes and usable evacuation. |
| 25 | Tank Bunkers | [U6](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | The separate reciprocal vehicle link through installation, fighting and release. |
| 26 | Slave Miner workforce | [R2](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Shared resource consumers plus the distinct worker, deployment, relocation and liberation lifecycle. |
| 27 | Grinder admission and settlement | [B8](2026-09-11-gameplay-goal-catalog.md#resources-and-the-base) | Consumption and value through interruption/cleanup and continued facility use. |
| 28 | Hover movement | [M3](2026-09-11-gameplay-goal-catalog.md#movement-and-locomotion-implementation-groups), [U1](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Steering, height and permitted terrain transitions through usable arrival. |
| 29 | Shared air motion | [M4](2026-09-11-gameplay-goal-catalog.md#movement-and-locomotion-implementation-groups) | Changed Fly/Jumpjet motion through real cell-list and altitude transitions; carry required consumer work with it. |
| 30 | Airfield sorties | [U7](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Harrier/Black Eagle through attack, return, rearm and repeated sorties. |
| 31 | Jumpjet unit transitions and required abilities | [U9](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Select remaining landing/deployment/cargo/attack outcomes by actual shared owner; whole-unit scope includes its abilities. |
| 32 | Spawn pools and launched rockets | [U8](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships), [M6](2026-09-11-gameplay-goal-catalog.md#movement-and-locomotion-implementation-groups) | Shared launcher pools and relevant child flight through real impact/return, regeneration and the next attack. |
| 33 | American and Tech Airport paradrops | [S8](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Shared handler variants through actual landed passengers and aircraft cleanup. |
| 34 | Remaining Chrono movement and recovery | [M5](2026-09-11-gameplay-goal-catalog.md#movement-and-locomotion-implementation-groups), [X14](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Complete the selected infantry/unit variants and locomotor restoration; reuse miner work already done. |
| 35 | Concealment, detection and affected composition | [W5](2026-09-11-gameplay-goal-catalog.md#world-interaction-and-information), [V5](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups) | Simulation admission/viewer knowledge through real targeting and visible cloak behavior; retain other modified-pixel cases. |
| 36 | Reversible mind control | [U10](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Controller variants through acquisition, real ownership consumers, capacity/overload and release. |
| 37 | Parasite-host behavior | [U11](2026-09-11-gameplay-goal-catalog.md#units-passengers-and-continuing-combat-relationships) | Drone/Squid relationship variants through continuing effects, interruption and cleanup. |
| 38 | Gattling progression | [X1](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Tank/Cannon stage changes through actual firing and cooldown. |
| 39 | Prism support | [X2](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | The support network through supported shots and provider/target loss. |
| 40 | Tesla charging and attacks | [X3](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Trooper/coil variants through power exceptions, actual attack and recovery. |
| 41 | Ordinary laser attacks | [X5](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Shared applicable beam behavior through hit effects and cleanup. |
| 42 | Sonic attacks | [X4](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Wave creation through actual hits and source/target expiry. |
| 43 | C4 and bridge charges | [X6](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Legal placement/action through explosion, world consequences and cleanup. |
| 44 | Ivan bombs | [X7](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Attachment, fuse/defusing and detonation through cleanup. |
| 45 | Suicide attacks | [X8](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Firers and collateral damage through attribution and removal. |
| 46 | Desolator radiation | [X9](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Deployment/firing through radiation effects, expiry and continuing world state. |
| 47 | Virus effects | [X10](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Persistent attack consequences through affected actors and expiry. |
| 48 | Chaos Drone | [X11](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Behavior changes through eligibility, interruption and recovery. |
| 49 | Other active fire/status effects | [X12](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Select the named reachable effect and its actual consumers; no inferred all-effects mega-task. |
| 50 | Yuri deployment pulses | [X13](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Shared pulse variants through actual area damage and stance recovery. |
| 51 | Temporal erasure | [X15](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Links and progress through competing attackers, cancellation or erasure. |
| 52 | Magnetron lift and release | [X16](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Locomotor handoff through lift/carry/drop, restoration and damage. |
| 53 | Boris airstrikes | [X17](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Designation through actual aircraft attack/departure and repeated use. |
| 54 | Dog leap | [X18](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Legal attack through impact, interruption and recovery. |
| 55 | Floating Disc attacks and drain | [X19](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Required flight/target admission through money/power/defense effects and release. |
| 56 | Spy infiltration | [X20](2026-09-11-gameplay-goal-catalog.md#distinct-abilities-that-retain-their-own-effect-loops) | Disguise/admission through target-specific result and actual consumer. |
| 57 | Psychic Sensor intent warnings | [W6](2026-09-11-gameplay-goal-catalog.md#world-interaction-and-information) | Actual enemy-order eligibility through warning update and removal. |
| 58 | Crates and rewards | [W7](2026-09-11-gameplay-goal-catalog.md#world-interaction-and-information) | Pickup through concrete rewards, removal and subsequent effects. |
| 59 | Iron Curtain and Force Shield | [S3](2026-09-11-gameplay-goal-catalog.md#strategic-powers), [S4](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Joint protection work with distinct target selection and blackout/recovery rules. |
| 60 | Chronosphere and ChronoWarp | [S5](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Both linked clicks through transfer and aftermath; reuse proven movement work. |
| 61 | Nuclear Missile | [S1](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Launch/descent through impact, damage and world cleanup. |
| 62 | Lightning Storm | [S2](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Scheduling through real strikes, termination and continued play. |
| 63 | Psychic Dominator | [S6](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Effect sequence through permanent ownership/damage and continuing victim behavior. |
| 64 | Genetic Mutator | [S7](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Eligibility through transformation, usable replacements and source cleanup. |
| 65 | Spy Plane | [S9](2026-09-11-gameplay-goal-catalog.md#strategic-powers) | Targeting through flight, information effect and departure. |
| 66 | Remaining modified-pixel composition | [V5](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups) | Translucency/distortion and other named native blitter cases not completed with cloak work. |
| 67 | Combat-light composition | [V6](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups) | Live sources through overlapping scene edits and expiry. |
| 68 | Searchlights | [V7](2026-09-11-gameplay-goal-catalog.md#lighting-and-rendering-implementation-groups) | Active source/registration through submitted beam output and cleanup. |
| 69 | Options across launcher and in-game screens | [F3](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Shared profile/persistence and actual consumers, with each apply/cancel policy. |
| 70 | Shell navigation and complete skirmish flow | [F3](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows), [F1](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Menus/setup/loading through playable match, results and clean restart/exit. |
| 71 | Generated maps and shared launch handoff | [F2](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows), [F1](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Accepted generation/seed identity through the actual playable map; check fixed maps when changing shared launch. |
| 72 | Authored scenario execution | [F5](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Events and Team consumers through real consequences, objectives and saved continuation. |
| 73 | Briefing and movie playback | [F7](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Media/speech/subtitles through skip/finish and correct destination. |
| 74 | Campaign progression | [F6](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Launch, mission outcomes, carryover and resumed progress with required scenario/media work. |
| 75 | Record and replay | [F9](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Actual initialized match and timed commands through completion or actionable divergence. |
| 76 | LAN multiplayer | [F10](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | Host/join through synchronized play, observation, outcome and disconnect handling. |
| 77 | Chosen online service | [F11](2026-09-11-gameplay-goal-catalog.md#opponents-and-complete-application-flows) | After an explicit service/support decision, actual multiplayer handoff and return. |

The original catalogue remains the scope/evidence authority. This index adds no
implementation, publication or scheduling authority and no completion ledger. Its
order can be revised as actual matches expose more valuable shared work. There is
no requirement to finish every earlier item before selecting a later complete goal.
