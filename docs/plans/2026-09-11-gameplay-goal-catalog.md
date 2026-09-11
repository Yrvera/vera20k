# Gameplay implementation guide: order, scope and goal prompts

Use this page to **choose the next work and define its goal**. It combines the
suggested implementation order with the catalogue's complete coverage and
boundaries. The [evidence companion](2026-09-11-gameplay-boundary-evidence.md)
remains supporting research to consult for the selected work.

## How to use this guide

1. Choose an entry from the [suggested order](#suggested-order), or start with a
   current match problem or your own priority. Check current code and behavior
   before treating an entry as unfinished.
2. Read its **Selection** and **Coverage and completion** together. Decide whether
   the goal is a full object/family, a named loop, or a bounded fix. Where a family
   spans entries, the links stay on this page; completing one selection does not
   close the broader family.
3. Use the entry's evidence links as starting leads. Compose the prompt with the
   [scope guidance](#start-with-the-requested-outcome) and
   [goal brief](#a-concise-goal-brief) below, stating the intended outcome and scope.
   An entry number or coverage identifier alone is not a goal prompt.

The game is already playable. This order favors everyday match interactions and
shared work; it is a recommendation, not a measured defect ranking, schedule or
completion checklist. Select the first meaningful demonstrated gap. Serious
current problems, required dependencies and user priorities can move any entry
forward. A broken Chrono return belongs with the resource goal; a LAN request
need not wait for every special weapon or campaign feature.

The 77 selections are not mandatory separate sessions or phases. Nearby entries
can be combined when their actual owners support shared work; a large authorized
goal can span several entries and PRs. The 83 coverage identifiers (R1, U4, etc.)
retain their scope, including when several selections refer to one family.
Reuse working behavior rather than refactoring merely to visit every entry.
Movement, presentation, AI and save are interleaved; any required integration
belongs to the selected goal regardless of where its other entry appears.

[ENGINE.md](../../ENGINE.md) governs evidence, architecture, scale, validation and
delivery. Rendering, audio, input, persistence, determinism and cleanup remain
obligations of every affected loop. This document grants no implementation,
publication or scheduling authority. The source observations remain dated to
the original inspection; this consolidation makes no new parity or status claims.

## Suggested order

Each link opens the selection and its scope **on this page**. Numbers preserve
the suggested order; coverage identifiers preserve the catalogue references.

| Order | Work to select | Coverage |
|---|---|---|
| 1 | [Ground orders and movement](#goal-1) | C1, U1, M1, M2 |
| 2 | [Standard resource economy](#goal-2) | R1 |
| 3 | [Purchases and delivery](#goal-3) | B1 |
| 4 | [MCV deployment and recovery](#goal-4) | B2 |
| 5 | [Power and provider-dependent behavior](#goal-5) | B5 |
| 6 | [Ordinary combat](#goal-6) | U2 |
| 7 | [Vision and reveal publication](#goal-7) | W4, S10 |
| 8 | [World depth and draw order](#goal-8) | V4 |
| 9 | [World lighting and palettes](#goal-9) | V1 |
| 10 | [Voxel appearance and shadows](#goal-10) | V2, V3 |
| 11 | [GI and Guardian GI deployment](#goal-11) | U3 |
| 12 | [AI base decisions and recovery](#goal-12) | F4 |
| 13 | [AI Team execution](#goal-13) | F4 |
| 14 | [Save and resume](#goal-14) | F8 |
| 15 | [Building repair](#goal-15) | B3 |
| 16 | [Building sale](#goal-16) | B4 |
| 17 | [Engineer capture and building benefits](#goal-17) | B6 |
| 18 | [Unit repair/service visits](#goal-18) | B7 |
| 19 | [Bridge traversal, collapse and repair](#goal-19) | W1 |
| 20 | [Scenery lifecycle](#goal-20) | W2 |
| 21 | [Walls and active gates](#goal-21) | W3 |
| 22 | [Cargo release and retry across routes](#goal-22) | U4, U5, S8 |
| 23 | [Transport boarding, carrying and passenger-dependent weapons](#goal-23) | U4 |
| 24 | [Infantry garrisons and Battle Bunkers](#goal-24) | U5 |
| 25 | [Tank Bunkers](#goal-25) | U6 |
| 26 | [Slave Miner workforce](#goal-26) | R2 |
| 27 | [Grinder admission and settlement](#goal-27) | B8 |
| 28 | [Hover movement](#goal-28) | M3, U1 |
| 29 | [Shared air motion](#goal-29) | M4 |
| 30 | [Airfield sorties](#goal-30) | U7 |
| 31 | [Jumpjet unit transitions and required abilities](#goal-31) | U9 |
| 32 | [Spawn pools and launched rockets](#goal-32) | U8, M6 |
| 33 | [American and Tech Airport paradrops](#goal-33) | S8 |
| 34 | [Remaining Chrono movement and recovery](#goal-34) | M5, X14 |
| 35 | [Concealment, detection and affected composition](#goal-35) | W5, V5 |
| 36 | [Reversible mind control](#goal-36) | U10 |
| 37 | [Parasite-host behavior](#goal-37) | U11 |
| 38 | [Gattling progression](#goal-38) | X1 |
| 39 | [Prism support](#goal-39) | X2 |
| 40 | [Tesla charging and attacks](#goal-40) | X3 |
| 41 | [Ordinary laser attacks](#goal-41) | X5 |
| 42 | [Sonic attacks](#goal-42) | X4 |
| 43 | [C4 and bridge charges](#goal-43) | X6 |
| 44 | [Ivan bombs](#goal-44) | X7 |
| 45 | [Suicide attacks](#goal-45) | X8 |
| 46 | [Desolator radiation](#goal-46) | X9 |
| 47 | [Virus effects](#goal-47) | X10 |
| 48 | [Chaos Drone](#goal-48) | X11 |
| 49 | [Other active fire/status effects](#goal-49) | X12 |
| 50 | [Yuri deployment pulses](#goal-50) | X13 |
| 51 | [Temporal erasure](#goal-51) | X15 |
| 52 | [Magnetron lift and release](#goal-52) | X16 |
| 53 | [Boris airstrikes](#goal-53) | X17 |
| 54 | [Dog leap](#goal-54) | X18 |
| 55 | [Floating Disc attacks and drain](#goal-55) | X19 |
| 56 | [Spy infiltration](#goal-56) | X20 |
| 57 | [Psychic Sensor intent warnings](#goal-57) | W6 |
| 58 | [Crates and rewards](#goal-58) | W7 |
| 59 | [Iron Curtain and Force Shield](#goal-59) | S3, S4 |
| 60 | [Chronosphere and ChronoWarp](#goal-60) | S5 |
| 61 | [Nuclear Missile](#goal-61) | S1 |
| 62 | [Lightning Storm](#goal-62) | S2 |
| 63 | [Psychic Dominator](#goal-63) | S6 |
| 64 | [Genetic Mutator](#goal-64) | S7 |
| 65 | [Spy Plane](#goal-65) | S9 |
| 66 | [Remaining modified-pixel composition](#goal-66) | V5 |
| 67 | [Combat-light composition](#goal-67) | V6 |
| 68 | [Searchlights](#goal-68) | V7 |
| 69 | [Options across launcher and in-game screens](#goal-69) | F3 |
| 70 | [Shell navigation and complete skirmish flow](#goal-70) | F3, F1 |
| 71 | [Generated maps and shared launch handoff](#goal-71) | F2, F1 |
| 72 | [Authored scenario execution](#goal-72) | F5 |
| 73 | [Briefing and movie playback](#goal-73) | F7 |
| 74 | [Campaign progression](#goal-74) | F6 |
| 75 | [Record and replay](#goal-75) | F9 |
| 76 | [LAN multiplayer](#goal-76) | F10 |
| 77 | [Chosen online service](#goal-77) | F11 |

## Ordered goal scopes

**Selection** names the portion to consider now. **Coverage and completion**
preserves the full referenced scope; use only the portion authorized by the
chosen goal. Variants listed here are starting coverage, not an exhaustive
active-retail census. Full-family goals include reachable civilian, preplaced,
reinforcement, elite, special-acquisition and mode/map/campaign variants
where they affect the named scope. Consult the [shared-work guidance](#choose-a-porting-goal-before-selecting-coverage)
when combining entries, and the [family notes](#family-scope-notes) for
additional movement, rendering and strategic-power obligations.

<a id="goal-1"></a>

### 1. Ground orders and movement

**Selection:** Selection/orders through driving, ship tracks, walking, blockage and usable arrival; group shared fixes.

**Coverage and completion:**

<a id="c1"></a>

**C1** — **Battlefield controls reach actual gameplay.** Scrolling/bookmarks, single/bandbox/type/control-group selection, hotkeys, queued/planned waypoints, radar navigation and permitted pause/resume/speed changes.

View/select → cursor/order admission → visible/audible feedback → actual execution → replacement/cancellation. Reach U1/U2 or the actual selected action. A marker or directly injected simulation command cannot certify the player interaction.

<a id="u1"></a>

**U1** — **Units obey movement/replacement orders through arrival.** Walking, driving, ship and hover variants.

Command → mission/destination/installed locomotor → path/turn/traffic/occupancy/crush → arrival → actual next action. Include stop, blocked recovery, replacement and death. Select the affected locomotor set explicitly; sharing this contract does not make all locomotors identical. Ability-specific movement and sorties stay integrated with their parent actions.

Related selections: [28](#goal-28). These cover portions or shared consumers of the same scope.

<a id="m1"></a>

**M1** — Drive and Ship track movement

Shared track admission/transition and lepton advancement justify joint changes across land/water variants. Cover order → route/turn → cell transition/occupancy → arrival/replacement and blocked recovery. Preserve separate path/runtime fields and ship admission rules.

<a id="m2"></a>

**M2** — Walking and infantry spatial movement

Shared ground path/crossing machinery with Walk-specific stepping and blocked behavior. Cover real orders, subcell occupancy, bridges, recovery, arrival and next action; include stance/prone consumers where affected. This does not automatically include every infantry weapon.

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo); [E13 Movement implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e13-movement-implementation-evidence).

**Shared scope notes:** [movement lifecycle and output rules](#movement-and-locomotion-implementation-groups).

<a id="goal-2"></a>

### 2. Standard resource economy

**Selection:** TIBTRE/ore/gems through War and Chrono harvesting, refinery income and the next trip; include required teleport work.

**Coverage and completion:**

<a id="r1"></a>

**R1** — **Resource supply becomes spendable income through standard miners.** Map ore/gems, TIBTRE01–03, growth/spread, War/Chrono Miner, refineries, cargo/value/storage and Ore Purifier.

Supply → authoritative resource cell → visible/searchable resource → mining/cargo → return/dock → income → release → next trip. Preserve different return paths and source-placement versus existing-field-growth rules. Include relevant orders/combat interruptions and concurrent-spending checks. Existing spending systems consume the wallet; this is not ownership of all economy code.

**Starting evidence:** [E1 Resources and mining](2026-09-11-gameplay-boundary-evidence.md#e1-resources-and-mining).

<a id="goal-3"></a>

### 3. Purchases and delivery

**Selection:** Shared factory lifecycle through usable units or placed buildings, including affected variants and queue continuation.

**Coverage and completion:**

<a id="b1"></a>

**B1** — **A purchase becomes a usable product and the queue continues.** Infantry, vehicles, ships, aircraft, structures/walls; faction/factory variants; prerequisites, build limits, prices, active building upgrades/slots, Industrial Plant and Cloning Vat consumers.

Sidebar/AI order → eligible queue → factory-held identity → charging/hold/cancel → completion → exit or placement/buildup/activation → next product. Blocked exits, provider loss, upgrade admission/effects, cloned-product delivery and completed-object disposition belong here. Building placement and mobile delivery are terminal variants of one purchase lifecycle.

**Starting evidence:** [E3 Purchases and base lifecycle](2026-09-11-gameplay-boundary-evidence.md#e3-purchases-and-base-lifecycle).

<a id="goal-4"></a>

### 4. MCV deployment and recovery

**Selection:** Stock variants through usable bases and applicable reverse conversion.

**Coverage and completion:**

<a id="b2"></a>

**B2** — **MCVs establish and relocate usable construction bases.** Stock MCV/Construction Yard variants and applicable reverse conversion.

Command → stop/turn/admission → successful target creation → source removal → selection/build authority → reverse conversion and usable mobile unit. Coordinate sale-completion machinery with B4. Slave Miner stays with R2's workforce lifecycle.

**Starting evidence:** [E3 Purchases and base lifecycle](2026-09-11-gameplay-boundary-evidence.md#e3-purchases-and-base-lifecycle).

<a id="goal-5"></a>

### 5. Power and provider-dependent behavior

**Selection:** Sources, occupants and capability shutdown/restoration through actual consumers.

**Coverage and completion:**

<a id="b5"></a>

**B5** — **Power/provider changes disable and restore the right capabilities.** Plants, Bio Reactor occupants, radar, production, defenses, Robot Control Center/Robot Tanks and strategic availability.

Gain/change/lose provider → output/demand/provider state → actual shutdown → feedback → restoration and resumed use. Bio Reactor passenger admission/release must reach power. Establish Robot's specific provider relationship; generic positive house power is not a sufficient specification. Tesla charging and Disc drain include their B5 integration.

**Starting evidence:** [E4 Power capture and benefits](2026-09-11-gameplay-boundary-evidence.md#e4-power-capture-and-benefits).

<a id="goal-6"></a>

### 6. Ordinary combat

**Selection:** Order/targeting through firing, impact, damage, death and subsequent action; select shared changed branches.

**Coverage and completion:**

<a id="u2"></a>

**U2** — **An ordinary engagement resolves from order to aftermath.** Common infantry/vehicle/building/naval/air cases; direct, ballistic, homing and applicable anti-air/air-to-air/strafe/bombing attacks.

Approach/acquire → weapon/range/facing/fire gates → burst/projectile/impact → armor/damage/fear/prone/veterancy/death/attribution → next order/target. Include debris and affected terrain/resource consequences. Reuse shared combat authority; representative units are coverage cases, not duplicate implementations.

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo).

<a id="goal-7"></a>

### 7. Vision and reveal publication

**Selection:** Per-viewer knowledge and complete Psychic Reveal behavior; coordinate shared publication while retaining source-specific lifetime and admission.

**Coverage and completion:**

<a id="w4"></a>

**W4** — **Exploration and shroud sources produce correct per-viewer knowledge.**

Scout/source activation → reconciliation → tactical/radar/selection/targeting → source loss/ownership/alliance/observer changes. Gap/SpySat are explicit source/latch cases; strategic reveals include their activation/effect lifecycle. Powered radar and revealed terrain are different facts.

<a id="s10"></a>

**S10** — Psychic Reveal

Targeting → actual per-viewer knowledge effect → required duration/termination.

**Starting evidence:** [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information); [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-8"></a>

### 8. World depth and draw order

**Selection:** Correct mixed terrain, bridge, building, sprite and voxel occlusion in motion.

**Coverage and completion:**

<a id="v4"></a>

**V4** — Mixed terrain/SHP/VXL depth and draw order

Shared tactical draw plan, ground-parent ordering, lowering, native Z policies and final pass submission. Group terrain, bridge, building, infantry, vehicle and upper-layer consumers affected by an ordering change. Compare occlusion during motion and bridge crossings, with parts/effects attached to the correct parent. This does not require rewriting each asset decoder.

**Starting evidence:** [E14 Rendering implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).

**Shared scope notes:** [rendering output and comparison rules](#lighting-and-rendering-implementation-groups).

<a id="goal-9"></a>

### 9. World lighting and palettes

**Selection:** Source/profile changes through actual lit terrain, actors and animations.

**Coverage and completion:**

<a id="v1"></a>

**V1** — Scenario/cell lighting and palette-lit world appearance

Follow authoritative lighting sources/profile → derived cell grid → per-drawer palette/brightness choice → actual terrain, building, infantry, vehicle and animation output. Shared grid or palette-conversion changes should cover affected consumers together. Grid lifetime and palette arithmetic can be separate increments; preserve cell/ColorScheme/animation differences. Compare the same scene before/after source/profile changes and affected restore paths. Aircraft altitude brightness remains a targeted investigation.

**Starting evidence:** [E14 Rendering implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).

**Shared scope notes:** [rendering output and comparison rules](#lighting-and-rendering-implementation-groups).

<a id="goal-10"></a>

### 10. Voxel appearance and shadows

**Selection:** Facing/parts/slopes through body composition; remaining shadow shape, placement and darkening across the selected full scope. Combine shared work, preserving distinct geometry producers.

**Coverage and completion:**

<a id="v2"></a>

**V2** — Voxel bodies, parts and slope appearance

Common native preparation/raster, CPU/GPU paths, atlas keys and visible slope-transition cache. Group affected body/turret/barrel, facing and slope variants through their in-game composition. Preserve model/HVA/VPL responsibilities; previews are not production proof. A cache hit or one flat-facing image cannot establish moving/ramp appearance.

<a id="v3"></a>

**V3** — Shadow shape, placement and destination darkening

Follow shadow producer → cached/masked geometry → projected position/depth admission → darkened scene. Common destination blending changes need affected SHP/VXL/terrain-shadow checks; their geometry producers are distinct. Ordinary Ground-band, uncloaked Drive units with flat single-section voxel shadows have a narrower established path than slopes, multiple sections or aircraft. Compare overlap, motion and applicable altitude/ramp cases; do not infer every shadow uses the same generator.

**Starting evidence:** [E14 Rendering implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).

**Shared scope notes:** [rendering output and comparison rules](#lighting-and-rendering-implementation-groups).

<a id="goal-11"></a>

### 11. GI and Guardian GI deployment

**Selection:** Both sustained-stance variants through firing, interruption and resumed movement.

**Coverage and completion:**

<a id="u3"></a>

**U3** — **GI and Guardian GI fight through deployment and recovery.** Both sustained deployed-infantry variants.

Deploy input → stance/animation/movement gate → actual deployed targeting/firing → undeploy/reorder/damage/death. Integrate ordinary movement/combat and applicable host interactions. Desolator radiation and Yuri pulses remain with their effects; Siege Chopper with U9.

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo).

<a id="goal-12"></a>

### 12. AI base decisions and recovery

**Selection:** Production/deployment/placement decisions through a functioning base; retain the rest of full-opponent scope when requested.

This selection focuses on base decisions. A complete F4 opponent also requires [Team execution](#goal-13).

**Coverage and completion:**

<a id="f4"></a>

**F4** — **An AI opponent builds, fights, defends and recovers.**

House decisions → ordinary production/deploy/place/attack consumers → resulting state → future decisions. Connected internal loops: base economy/placement/rebuilding and team selection/recruitment/script execution/replenishment. Faction/difficulty are variants. A loaded registry or advancing cursor cannot establish an actual attack/defense. Base decisions and Team execution are distinct implementation focuses within this full outcome.

Related selections: [13](#goal-13). These cover portions or shared consumers of the same scope.

**Starting evidence:** [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-13"></a>

### 13. AI Team execution

**Selection:** Recruitment, scripts and real combat effects through replenishment; integrate with existing base decisions.

Select recruitment, script effects and replenishment here; integrate with base decisions when the goal is a complete opponent.

**Coverage and completion:**

- [F4: An AI opponent builds, fights, defends and recovers.](#f4) — full scope in [entry 12](#goal-12).

**Starting evidence:** [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-14"></a>

### 14. Save and resume

**Selection:** Group quickload/panel restoration; include cold-start content preparation for the complete supported loading flow.

**Coverage and completion:**

<a id="f8"></a>

**F8** — **Save a match and resume it through supported entry points.**

UI → content/version validation → prepared world/fixups → commit → rebuild presentation/reset pacing → continued play. Include shell startup/in-game paths, failure/cancel and active feature state. Group quickload/panel restoration work; cold-start content preparation is an additional focus. Restoring within an existing content context cannot by itself establish cold-start loading.

**Starting evidence:** [E10 Persistence networking and presentation](2026-09-11-gameplay-boundary-evidence.md#e10-persistence-networking-and-presentation); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-15"></a>

### 15. Building repair

**Selection:** Spending/healing through interruption and continued building operation.

**Coverage and completion:**

<a id="b3"></a>

**B3** — **Buildings repair correctly while the match continues.** Repair toggle, funding, HP/damage states and power effects.

Order → scheduled debit/healing → presentation/power update → completion/cancel/destruction. Include competing production/depot spending where ordering matters. The building remains the same live object; this differs from sale and unit service.

**Starting evidence:** [E3 Purchases and base lifecycle](2026-09-11-gameplay-boundary-evidence.md#e3-purchases-and-base-lifecycle).

<a id="goal-16"></a>

### 16. Building sale

**Selection:** Dependents, refund and capability changes through usable released units and ground.

**Coverage and completion:**

<a id="b4"></a>

**B4** — **Selling releases a building, its dependents and its value correctly.** Ordinary, occupied, docked, upgraded and capability-providing variants where active.

Sell → animation/state → crew/passenger/refinery/bunker release → removal → refund and house/production/power changes → usable ground and released units. Check the resulting world, not only a refund formula. B2 owns reverse-MCV acceptance when this machinery is involved.

**Starting evidence:** [E3 Purchases and base lifecycle](2026-09-11-gameplay-boundary-evidence.md#e3-purchases-and-base-lifecycle).

<a id="goal-17"></a>

### 17. Engineer capture and building benefits

**Selection:** Capture through actual benefit and later revocation, preserving distinct benefit owners.

**Coverage and completion:**

<a id="b6"></a>

**B6** — **Engineers capture buildings, use their benefits and lose them correctly.** Base/factory structures, Oil Derrick, Secret Lab, Tech Hospital/Machine Shop and other active tech variants.

Approach/admission → old-owner cleanup → transfer/engineer disposition → actual benefit → later loss/revocation. Each benefit retains its mechanism owner: production, periodic income, unlocks or passive healing. Hospitals/Machine Shops are not repair-depot visits. Engineer bridge-hut behavior integrates W1.

**Starting evidence:** [E4 Power capture and benefits](2026-09-11-gameplay-boundary-evidence.md#e4-power-capture-and-benefits).

<a id="goal-18"></a>

### 18. Unit repair/service visits

**Selection:** Admission, contention and spending through release and the next order or mining trip.

**Coverage and completion:**

<a id="b7"></a>

**B7** — **A unit visits a service facility and returns to use.** Active repair-depot/service variants, including affected miners.

Enter/radio/reservation → service/debit/healing → release → next order or resumed mining. Include contention, interruption and provider loss. Reuse docking primitives while retaining refinery unload and airfield sorties as their distinct complete loops.

**Starting evidence:** [E4 Power capture and benefits](2026-09-11-gameplay-boundary-evidence.md#e4-power-capture-and-benefits).

<a id="goal-19"></a>

### 19. Bridge traversal, collapse and repair

**Selection:** Connected topology, occupancy, engineer and visible-world lifecycle.

**Coverage and completion:**

<a id="w1"></a>

**W1** — **Bridges remain coherent through traversal, damage, collapse and repair.**

High/low/orientation variants share the world lifecycle. Include occupants/on-under passage, combat, engineer/hut admission, topology/zone/overlay/radar refresh and traversal after repair. Repair animation is insufficient without restored passage.

**Starting evidence:** [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information).

<a id="goal-20"></a>

### 20. Scenery lifecycle

**Selection:** Loading/occupation through active damage/removal and spatial/visual cleanup.

**Coverage and completion:**

<a id="w2"></a>

**W2** — **Scenery affects the world throughout its lifetime.**

Trees/rocks share terrain lifecycle with type/theater/immune/damage variants: load → occupation/appearance → applicable interaction → removal/spatial cleanup. TIBTRE integrates this terrain owner while spawning is R1. Do not assume every tree burns or can be crushed.

**Starting evidence:** [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information).

<a id="goal-21"></a>

### 21. Walls and active gates

**Selection:** Placement and connected obstacle behavior through passage changes and removal.

**Coverage and completion:**

<a id="w3"></a>

**W3** — **Walls and active gates alter passage correctly.**

Connections, placement/load, opening/obstruction, damage/removal and navigation/appearance stay together. B1 owns purchased wall delivery; this loop owns the obstacle. Prove active YR gate variants before importing legacy fence behavior.

**Starting evidence:** [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information).

<a id="goal-22"></a>

### 22. Cargo release and retry across routes

**Selection:** Shared departure work across vehicles, landed aircraft, garrisons and paradrops; route-specific rules stay explicit.

The shared change is cargo-head removal, recorded size and failed-release restoration across vehicle, landed-aircraft, garrison and paradrop callers. Cover successful release and retry. Preserve route geometry, cadence, parachute attachment and weapon-reset differences. Full transport combat, garrison combat and strategic-power lifecycles are additional scopes; NATBNK uses a separate reciprocal link.

**Coverage and completion:**

- [U4: Mobile transports and passengers work through carrying, fighting and release. Land/sea/air transports, IFV and Battle Fortress.](#u4) — full scope in [entry 23](#goal-23).

- [U5: Infantry occupy, fight from and leave buildings. Civilian garrisons and Soviet Battle Bunker `NABNKR`.](#u5) — full scope in [entry 24](#goal-24).

- [S8: American and Tech Airport paradrops](#s8) — full scope in [entry 33](#goal-33).

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo); [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-23"></a>

### 23. Transport boarding, carrying and passenger-dependent weapons

**Selection:** Complete mobile cargo behavior; distinguish IFV host weapons from Battle Fortress passenger firing.

**Coverage and completion:**

<a id="u4"></a>

**U4** — **Mobile transports and passengers work through carrying, fighting and release.** Land/sea/air transports, IFV and Battle Fortress.

Admission → cargo membership/concealment → movement and applicable firing → unload placement/retry or host destruction → usable passengers. IFV selects the host's weapon; Battle Fortress passenger firing retains its own responsibility. Air-transport landing/exit is a required variant. Use the cargo-departure group across U5/S8 when changing that shared operation; keep host-weapon and passenger-firing work explicit.

Related selections: [22](#goal-22). These cover portions or shared consumers of the same scope.

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-24"></a>

### 24. Infantry garrisons and Battle Bunkers

**Selection:** Occupation through firing, ownership changes and usable evacuation.

**Coverage and completion:**

<a id="u5"></a>

**U5** — **Infantry occupy, fight from and leave buildings.** Civilian garrisons and Soviet Battle Bunker `NABNKR`.

Admission → occupant/ownership state → occupant firing/credit and occupied art → voluntary/forced evacuation, sale or destruction → released actors/building state. Keep presentation and firing with occupancy.

Related selections: [22](#goal-22). These cover portions or shared consumers of the same scope.

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo).

<a id="goal-25"></a>

### 25. Tank Bunkers

**Selection:** The separate reciprocal vehicle link through installation, fighting and release.

**Coverage and completion:**

<a id="u6"></a>

**U6** — **Yuri Tank Bunker `NATBNK` installs, supports and releases a vehicle.**

Approach/radio → reciprocal single-vehicle link/install → actual combat → release/sale/destruction → link cleared and vehicle usable. This differs from U5's infantry occupation model.

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo).

<a id="goal-26"></a>

### 26. Slave Miner workforce

**Selection:** Shared resource consumers plus the distinct worker, deployment, relocation and liberation lifecycle.

**Coverage and completion:**

<a id="r2"></a>

**R2** — **Slave Miner and its workforce sustain harvesting through relocation and loss.** Mobile/deployed master, slaves, assignment, harvesting, replacement/liberation and ownership.

Share R1's resource/payout contracts; own master/worker state, return/deposit, deployment and cleanup here. Verify resources produced by R1 remain usable by slaves. Worker deposit does not use the standard refinery queue.

**Starting evidence:** [E2 Slave economy](2026-09-11-gameplay-boundary-evidence.md#e2-slave-economy).

<a id="goal-27"></a>

### 27. Grinder admission and settlement

**Selection:** Consumption and value through interruption/cleanup and continued facility use.

**Coverage and completion:**

<a id="b8"></a>

**B8** — **A Grinder admits and consumes a unit with correct settlement.** Eligible units, ownership and interrupted entry.

Approach/admission → consumption → value/house effects → cleanup and continued facility use. Shared credits or Enter machinery do not turn this into building repair or sale. Remaining exact admission/settlement rules require a targeted native trace.

**Starting evidence:** [E4 Power capture and benefits](2026-09-11-gameplay-boundary-evidence.md#e4-power-capture-and-benefits).

<a id="goal-28"></a>

### 28. Hover movement

**Selection:** Steering, height and permitted terrain transitions through usable arrival.

Select Hover-specific work from U1 here; reuse the ground-order contracts established in entry 1.

**Coverage and completion:**

<a id="m3"></a>

**M3** — Hover motion

Steering, throttle and vertical motion form a distinct focus using ground movement consumers. Cover permitted land/water transitions, acceleration/turning/height, stop/replacement and usable arrival. Check other locomotors when changing their shared ground infrastructure.

- [U1: Units obey movement/replacement orders through arrival. Walking, driving, ship and hover variants.](#u1) — full scope in [entry 1](#goal-1).

**Starting evidence:** [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo); [E13 Movement implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e13-movement-implementation-evidence).

**Shared scope notes:** [movement lifecycle and output rules](#movement-and-locomotion-implementation-groups).

<a id="goal-29"></a>

### 29. Shared air motion

**Selection:** Changed Fly/Jumpjet motion through real cell-list and altitude transitions; carry required consumer work with it.

**Coverage and completion:**

<a id="m4"></a>

**M4** — Shared Fly/Jumpjet air motion and branch-specific transitions

Joint common air-motion/cell-list changes must exercise both; Fly and Jumpjet retain different speed/altitude/landing decisions. Airfield return/rearm (U7), Jumpjet deployment (U9) and cargo (U4) are explicit additional focuses required for their complete parent outcomes. Common flight work alone does not certify complete aircraft behavior.

**Starting evidence:** [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups); [E13 Movement implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e13-movement-implementation-evidence).

**Shared scope notes:** [movement lifecycle and output rules](#movement-and-locomotion-implementation-groups).

<a id="goal-30"></a>

### 30. Airfield sorties

**Selection:** Harrier/Black Eagle through attack, return, rearm and repeated sorties.

**Coverage and completion:**

<a id="u7"></a>

**U7** — **Harrier and Black Eagle complete repeatable airfield sorties.** Both aircraft and provider/pad variants.

Production/idle → takeoff/attack → return/reservation/landing → rearm → next sortie. Include provider loss/capture, contention, replacement orders and aircraft loss. Flight alone cannot complete this loop.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects).

<a id="goal-31"></a>

### 31. Jumpjet unit transitions and required abilities

**Selection:** Select remaining landing/deployment/cargo/attack outcomes by actual shared owner; whole-unit scope includes its abilities.

**Coverage and completion:**

<a id="u9"></a>

**U9** — **Hovering airborne units move and fight through required transitions.** Rocketeer, Kirov, Nighthawk, Disc, Siege Chopper and active scenario variants.

Jumpjet/altitude behavior → actual attack/carry order → stop/landing where permitted → next action. Chopper landing/deployment/weapon change/resumed flight is a complete named outcome. Disc drain and Nighthawk passengers integrate their effect/cargo owners; locomotion alone cannot certify them. Split shared flight work from those additional effect/cargo/deployment focuses when selecting a bounded goal.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-32"></a>

### 32. Spawn pools and launched rockets

**Selection:** Shared launcher pools and relevant child flight through real impact/return, regeneration and the next attack.

**Coverage and completion:**

<a id="u8"></a>

**U8** — **Launchers manage spawned aircraft or rockets through repeated attacks.** Carrier/Destroyer and V3/Dreadnought/Boomer.

Parent target → fixed spawn pool → child launch/attack → return/reload or missile regeneration → next attack. Include parent/child/target loss and owner changes. Returning aircraft and expendable missiles are variants of the same pool owner. Boris has a different designation/airstrike lifecycle.

<a id="m6"></a>

**M6** — Launcher-rocket flight and impact

Rocket phases/payload are separate from ordinary air ticking. Follow parent launch → movement phases → impact/damage → child cleanup → parent regeneration/next attack (U8). A completed trajectory flag is not a completed attack.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects); [E13 Movement implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e13-movement-implementation-evidence).

**Shared scope notes:** [movement lifecycle and output rules](#movement-and-locomotion-implementation-groups).

<a id="goal-33"></a>

### 33. American and Tech Airport paradrops

**Selection:** Shared handler variants through actual landed passengers and aircraft cleanup.

**Coverage and completion:**

<a id="s8"></a>

**S8** — American and Tech Airport paradrops

Provider/payload variants → aircraft/drop → actual landed passengers and aircraft cleanup. Check both variants.

Related selections: [22](#goal-22). These cover portions or shared consumers of the same scope.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-34"></a>

### 34. Remaining Chrono movement and recovery

**Selection:** Complete the selected infantry/unit variants and locomotor restoration; reuse miner work already done.

**Coverage and completion:**

<a id="m5"></a>

**M5** — Teleport movement and locomotor recovery

Follow destination admission → relocation/occupancy/visual state → recovery/restored locomotor → next action. Include Chrono Miner return and ordinary player-movement consumers where changed. Active Teleport and temporary locomotor overrides remain distinct branches; Chronosphere's full two-click effect is not automatically completed by this movement work.

<a id="x14"></a>

**X14** — Chrono movement

Teleport admission → movement/occupancy transition → recovery → usable next action, including affected infantry/miner variants. R1 includes the miner's complete return outcome.

**Starting evidence:** [E1 Resources and mining](2026-09-11-gameplay-boundary-evidence.md#e1-resources-and-mining); [E5 Movement stance and cargo](2026-09-11-gameplay-boundary-evidence.md#e5-movement-stance-and-cargo); [E13 Movement implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e13-movement-implementation-evidence).

General teleport/infantry recovery needs a targeted native-owner lookup;
the miner and installed-movement references are not its full contract.

**Shared scope notes:** [movement lifecycle and output rules](#movement-and-locomotion-implementation-groups).

<a id="goal-35"></a>

### 35. Concealment, detection and affected composition

**Selection:** Simulation admission/viewer knowledge through real targeting and visible cloak behavior; retain other modified-pixel cases.

Select cloak/detection admission and affected visible composition here. Other V5 pixel effects remain selectable in [entry 66](#goal-66).

**Coverage and completion:**

<a id="w5"></a>

**W5** — **Concealment and detection govern observer knowledge and attacks.**

Cloak/disguise exposure or sensor/provider movement → counted detection/resident reevaluation → targeting/picking/rendering → owner/limbo/expiry cleanup. Include Spy/Mirage/naval variants; share W4 without collapsing distinct concealment rules.

<a id="v5"></a>

**V5** — Translucency, cloak and other modified-pixel composition

Select the actual native blitter/effect family and affected production consumers; follow admission/strength → source/destination pixel operation → depth/output → recovery. Current opaque palette conversion deliberately routes alpha/FX pixels elsewhere, so ordinary opaque palette parity cannot close these effects. Establish which effects share implementation before expanding to all cloaking, translucency or distortion.

Related selections: [66](#goal-66). These cover portions or shared consumers of the same scope.

**Starting evidence:** [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups); [E14 Rendering implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).

**Shared scope notes:** [rendering output and comparison rules](#lighting-and-rendering-implementation-groups).

<a id="goal-36"></a>

### 36. Reversible mind control

**Selection:** Controller variants through acquisition, real ownership consumers, capacity/overload and release.

**Coverage and completion:**

<a id="u10"></a>

**U10** — **Reversible mind control maintains and releases victims.** Yuri Clone/Prime, Psychic Tower and Master Mind.

Acquisition → controller membership/owner transfer → actual order/house/production/visual consumers → capacity/overload and release/controller/victim loss → required restored state. Group controller variants; permanent Dominator effects and deployment pulses differ. Inspect acquisition and release callers, not only manager fields.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects).

<a id="goal-37"></a>

### 37. Parasite-host behavior

**Selection:** Drone/Squid relationship variants through continuing effects, interruption and cleanup.

**Coverage and completion:**

<a id="u11"></a>

**U11** — **Parasites maintain and release their hosts.** Terror Drone infestation and Giant Squid grapple.

Admission → parasite-host relationship → continuing effects → service/escape/detach/death → cleanup. Group the relationship with explicit variant rules; do not merge Temporal or Magnetron merely because they share the special-weapon dispatcher.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects).

<a id="goal-38"></a>

### 38. Gattling progression

**Selection:** Tank/Cannon stage changes through actual firing and cooldown.

**Coverage and completion:**

<a id="x1"></a>

**X1** — Gattling Tank/Cannon

Fire → stage progression and actual weapon/damage/feedback changes → interruption/retarget/cooldown → subsequent firing. Check unit/building consumers together.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-39"></a>

### 39. Prism support

**Selection:** The support network through supported shots and provider/target loss.

**Coverage and completion:**

<a id="x2"></a>

**X2** — Prism support

Support membership/availability → supported shot/damage/beam → target/provider interruption and cleanup. Prism Tank is an attack consumer, not automatically part of the tower network.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-40"></a>

### 40. Tesla charging and attacks

**Selection:** Trooper/coil variants through power exceptions, actual attack and recovery.

**Coverage and completion:**

<a id="x3"></a>

**X3** — Tesla charging/attacks

Trooper/coil relationship → charging/overpower and power exceptions → actual shot/bolt → loss/release/recovery. Ordinary Tesla weapons reuse confirmed effect machinery.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-41"></a>

### 41. Ordinary laser attacks

**Selection:** Shared applicable beam behavior through hit effects and cleanup.

**Coverage and completion:**

<a id="x5"></a>

**X5** — Ordinary laser attacks

Fire → beam and actual hit behavior → effect/source/target expiry. Disc-specific attack/drain is X19.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-42"></a>

### 42. Sonic attacks

**Selection:** Wave creation through actual hits and source/target expiry.

**Coverage and completion:**

<a id="x4"></a>

**X4** — Sonic attacks

Fire → wave and actual hit behavior → effect/source/target expiry. Shared drawing code does not merge this with lasers.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-43"></a>

### 43. C4 and bridge charges

**Selection:** Legal placement/action through explosion, world consequences and cleanup.

**Coverage and completion:**

<a id="x6"></a>

**X6** — C4 and bridge charges

Legal approach/action → explosion and actor/bridge consequences → cleanup. Include world and release consumers.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-44"></a>

### 44. Ivan bombs

**Selection:** Attachment, fuse/defusing and detonation through cleanup.

**Coverage and completion:**

<a id="x7"></a>

**X7** — Ivan bombs

Attachment/ownership → fuse/defusing → detonation or removal → cleanup/feedback. This is distinct from an immediate explosive attack.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-45"></a>

### 45. Suicide attacks

**Selection:** Firers and collateral damage through attribution and removal.

**Coverage and completion:**

<a id="x8"></a>

**X8** — Suicide attacks

Firing/detonation and firer removal → collateral damage/credit/cleanup. Verify active unit variants.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-46"></a>

### 46. Desolator radiation

**Selection:** Deployment/firing through radiation effects, expiry and continuing world state.

**Coverage and completion:**

<a id="x9"></a>

**X9** — Desolator radiation

Deployment/firing → radiation-site/target damage → source changes/expiry → continuing world state. Deployment belongs with this effect.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-47"></a>

### 47. Virus effects

**Selection:** Persistent attack consequences through affected actors and expiry.

**Coverage and completion:**

<a id="x10"></a>

**X10** — Virus effects

Source attack → persistent damage/effect lifecycle → affected actors/ownership → expiry/cleanup. Include actual damage and feedback.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-48"></a>

### 48. Chaos Drone

**Selection:** Behavior changes through eligibility, interruption and recovery.

**Coverage and completion:**

<a id="x11"></a>

**X11** — Chaos Drone

Effect admission → changed behavior → ownership/liveness interactions → expiration and resumed behavior.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-49"></a>

### 49. Other active fire/status effects

**Selection:** Select the named reachable effect and its actual consumers; no inferred all-effects mega-task.

**Coverage and completion:**

<a id="x12"></a>

**X12** — Other active fire/status effects

Select the named active effect and its consumers; follow creation → continuing damage/behavior → expiration. This is a coverage family, not a task to implement every particle system.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-50"></a>

### 50. Yuri deployment pulses

**Selection:** Shared pulse variants through actual area damage and stance recovery.

**Coverage and completion:**

<a id="x13"></a>

**X13** — Yuri/Prime deployment pulses

Deploy → area-damage weapon/target rules → stance recovery → next orders. Group pulse variants separately from reversible capture.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-51"></a>

### 51. Temporal erasure

**Selection:** Links and progress through competing attackers, cancellation or erasure.

**Coverage and completion:**

<a id="x15"></a>

**X15** — Temporal erasure

Attacker/target link → suspension/progress/competing attackers → interruption or erasure → cleanup.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects).

<a id="goal-52"></a>

### 52. Magnetron lift and release

**Selection:** Locomotor handoff through lift/carry/drop, restoration and damage.

**Coverage and completion:**

<a id="x16"></a>

**X16** — Magnetron

Source/victim locomotor handoff → lift/carry → drop/restoration and terrain/damage effects → cleanup. Jumpjet is a dependency, not proof this is an ordinary aircraft.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects).

<a id="goal-53"></a>

### 53. Boris airstrikes

**Selection:** Designation through actual aircraft attack/departure and repeated use.

**Coverage and completion:**

<a id="x17"></a>

**X17** — Boris airstrike

Designation → called aircraft → actual attack/departure → source/target/cancellation cleanup and subsequent use.

**Starting evidence:** [E6 Sorties pools and attached effects](2026-09-11-gameplay-boundary-evidence.md#e6-sorties-pools-and-attached-effects).

<a id="goal-54"></a>

### 54. Dog leap

**Selection:** Legal attack through impact, interruption and recovery.

**Coverage and completion:**

<a id="x18"></a>

**X18** — Dog leap

Legal attack → leap/impact → target loss or interruption → recovery/next order, with applicable disguise/detection interactions.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-55"></a>

### 55. Floating Disc attacks and drain

**Selection:** Required flight/target admission through money/power/defense effects and release.

**Coverage and completion:**

<a id="x19"></a>

**X19** — Floating Disc

Flight/attack/drain admission → target-class money/power/defense effects → release/source/target loss → dependent capabilities recover.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-56"></a>

### 56. Spy infiltration

**Selection:** Disguise/admission through target-specific result and actual consumer.

**Coverage and completion:**

<a id="x20"></a>

**X20** — Spy infiltration

Disguise/approach/detection/admission → target-specific result → spy disposition → actual power/economy/information/production/research consumer. Shared Enter/ownership services do not make this engineer capture.

**Starting research:** use the [effect-owner lookup](#effect-owner-lookup) for this named ability.

<a id="goal-57"></a>

### 57. Psychic Sensor intent warnings

**Selection:** Actual enemy-order eligibility through warning update and removal.

**Coverage and completion:**

<a id="w6"></a>

**W6** — **Psychic Sensors show the correct enemy-intent warnings.**

Provider/enemy-order eligibility → warning lines → order/source changes and cleanup. Establish its own radius/rules; this is not merely radar dots or cloak detection.

**Starting evidence:** [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information).

<a id="goal-58"></a>

### 58. Crates and rewards

**Selection:** Pickup through concrete rewards, removal and subsequent effects.

**Coverage and completion:**

<a id="w7"></a>

**W7** — **Crates yield complete rewards.**

Spawn/regeneration → ordinary movement pickup → effect/feedback/removal → later unit/economy consequences. Active reward classes are variants; include ordinary stock skirmish.

**Starting evidence:** [E7 World and information](2026-09-11-gameplay-boundary-evidence.md#e7-world-and-information).

<a id="goal-59"></a>

### 59. Iron Curtain and Force Shield

**Selection:** Joint protection work with distinct target selection and blackout/recovery rules.

**Coverage and completion:**

<a id="s3"></a>

**S3** — Iron Curtain

Eligible target application → protection/damage interactions → expiry and subsequent use.

<a id="s4"></a>

**S4** — Force Shield

Application → protection and applicable house/power consequences → recovery.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-60"></a>

### 60. Chronosphere and ChronoWarp

**Selection:** Both linked clicks through transfer and aftermath; reuse proven movement work.

**Coverage and completion:**

<a id="s5"></a>

**S5** — Chronosphere plus ChronoWarp

Both linked clicks → destination admission → affected-object transfer and aftermath. The first click cannot close the action.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-61"></a>

### 61. Nuclear Missile

**Selection:** Launch/descent through impact, damage and world cleanup.

**Coverage and completion:**

<a id="s1"></a>

**S1** — Nuclear Missile (`MultiMissile`)

Launch/descent/impact → damage and world effects → cleanup.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-62"></a>

### 62. Lightning Storm

**Selection:** Scheduling through real strikes, termination and continued play.

**Coverage and completion:**

<a id="s2"></a>

**S2** — Lightning Storm

Activation → storm scheduling and actual strikes → termination and continued play.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-63"></a>

### 63. Psychic Dominator

**Selection:** Effect sequence through permanent ownership/damage and continuing victim behavior.

**Coverage and completion:**

<a id="s6"></a>

**S6** — Psychic Dominator

Effect sequence → actual damage/ownership result → continuing victim/world behavior.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-64"></a>

### 64. Genetic Mutator

**Selection:** Eligibility through transformation, usable replacements and source cleanup.

**Coverage and completion:**

<a id="s7"></a>

**S7** — Genetic Mutator

Eligibility → transformation → usable resulting actors and source cleanup.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-65"></a>

### 65. Spy Plane

**Selection:** Targeting through flight, information effect and departure.

**Coverage and completion:**

<a id="s9"></a>

**S9** — Spy Plane

Targeting → approach/overflight and actual information effect → departure/cleanup.

**Starting evidence:** [E8 Strategic powers](2026-09-11-gameplay-boundary-evidence.md#e8-strategic-powers).

**Shared scope notes:** [power lifecycle and variant rules](#strategic-powers).

<a id="goal-66"></a>

### 66. Remaining modified-pixel composition

**Selection:** Translucency/distortion and other named native blitter cases not completed with cloak work.

Select the V5 translucency, distortion or other native blitter cases still required after the cloak work; reuse its supported composition changes.

**Coverage and completion:**

- [V5: Translucency, cloak and other modified-pixel composition](#v5) — full scope in [entry 35](#goal-35).

**Starting evidence:** [E14 Rendering implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).

**Shared scope notes:** [rendering output and comparison rules](#lighting-and-rendering-implementation-groups).

<a id="goal-67"></a>

### 67. Combat-light composition

**Selection:** Live sources through overlapping scene edits and expiry.

**Coverage and completion:**

<a id="v6"></a>

**V6** — Combat-light screen composition

Shared combat-light preparation, scene snapshot and RGB565 mask editing through real effect sources, movement and expiry. Validate overlapping lights and the resulting scene, not only a generated mask. This screen-composition path is distinct from V1's cell-light grid. Source lifecycle changes remain integrated with their weapon/particle consumers.

**Starting evidence:** [E14 Rendering implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).

**Shared scope notes:** [rendering output and comparison rules](#lighting-and-rendering-implementation-groups).

<a id="goal-68"></a>

### 68. Searchlights

**Selection:** Active source/registration through submitted beam output and cleanup.

**Coverage and completion:**

<a id="v7"></a>

**V7** — Searchlight/spotlight projection and visible beam

Follow the active source/child-light lifecycle into beam or mask generation and submitted output. Existing renderer support alone does not prove live delivery: the inspected instance builder supplies an empty spotlight vector. Trace registration/admission before implementation scope is fixed. Do not merge this with combat flashes solely because both are called lights.

**Starting evidence:** [E14 Rendering implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).

**Shared scope notes:** [rendering output and comparison rules](#lighting-and-rendering-implementation-groups).

<a id="goal-69"></a>

### 69. Options across launcher and in-game screens

**Selection:** Shared profile/persistence and actual consumers, with each apply/cancel policy.

Select shared options profile, persistence and actual consumers across launcher and in-game screens, preserving their apply/cancel/preview policies. F3 also includes [shell navigation](#goal-70).

**Coverage and completion:**

<a id="f3"></a>

**F3** — **Navigate the shell and retain settings.**

Menu/dialog → correct child route/settings operation → apply/cancel/back/focus → handoff or exit/persistence. Shell navigation is bounded; campaign/network mechanics are destination goals. “All destinations work” requires combined integration. Include controls, display/audio/gameplay options and restart.

Related selections: [70](#goal-70). These cover portions or shared consumers of the same scope.

**Starting evidence:** [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-70"></a>

### 70. Shell navigation and complete skirmish flow

**Selection:** Menus/setup/loading through playable match, results and clean restart/exit.

Select shell routing and the complete fixed-map skirmish flow here. Reuse options work from entry 69; destination backends retain their own acceptance.

**Coverage and completion:**

- [F3: Navigate the shell and retain settings.](#f3) — full scope in [entry 69](#goal-69).

<a id="f1"></a>

**F1** — **Configure, launch, play and finish a skirmish.**

Shell choices → actual map/rules/mode/assets/starts → loading → playable match → outcome/surrender/scores/statistics/results/restart/exit → clean next session. Include launch failure/cancel paths. Faction/map/difficulty are variants. Exercise content precedence, theaters, ramps/cliffs/shores/bounds and mode/map overrides through actual world construction and play.

Related selections: [71](#goal-71). These cover portions or shared consumers of the same scope.

**Starting evidence:** [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-71"></a>

### 71. Generated maps and shared launch handoff

**Selection:** Accepted generation/seed identity through the actual playable map; check fixed maps when changing shared launch.

Select generation/seed semantics and accepted-map handoff here. Check the fixed-map consumer when changing shared launch; a preview alone does not establish the playable map.

**Coverage and completion:**

<a id="f2"></a>

**F2** — **Generate a map and play that same map.**

Options/seed → generator/preview → matching launch → movement/construction and cleanup. Share F1's launch contract; generation has its own RNG/content identity.

- [F1: Configure, launch, play and finish a skirmish.](#f1) — full scope in [entry 70](#goal-70).

**Starting evidence:** [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows); [E12 Further implementation groups](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups).

<a id="goal-72"></a>

### 72. Authored scenario execution

**Selection:** Events and Team consumers through real consequences, objectives and saved continuation.

**Coverage and completion:**

<a id="f5"></a>

**F5** — **Authored scenario events produce their full consequences.**

Trigger/Tag/variables/latches → ordered action → real units/teams/reinforcements/camera/messages/objectives → next event/outcome and saved continuation. Share Team/command machinery with F4; scenario triggers have different conditions and persistent state. Include active convoy/special-mission cases when established.

**Starting evidence:** [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows).

<a id="goal-73"></a>

### 73. Briefing and movie playback

**Selection:** Media/speech/subtitles through skip/finish and correct destination.

**Coverage and completion:**

<a id="f7"></a>

**F7** — **Movies and briefing media play and return correctly.**

Start → synchronized video/speech/subtitles → skip/finish → correct screen/mission. Formats and destinations are variants; reusable media machinery stays shared.

**Starting evidence:** [E10 Persistence networking and presentation](2026-09-11-gameplay-boundary-evidence.md#e10-persistence-networking-and-presentation).

<a id="goal-74"></a>

### 74. Campaign progression

**Selection:** Launch, mission outcomes, carryover and resumed progress with required scenario/media work.

**Coverage and completion:**

<a id="f6"></a>

**F6** — **Campaigns launch, advance and resume progress.**

Campaign/mission/difficulty/briefing → actual scenario → outcome → next mission/carryover → persisted progression. Integrate F5 scripts and F7 media. Campaign progression is not a skirmish setting variant.

**Starting evidence:** [E9 AI scenarios and session flows](2026-09-11-gameplay-boundary-evidence.md#e9-ai-scenarios-and-session-flows).

<a id="goal-75"></a>

### 75. Record and replay

**Selection:** Actual initialized match and timed commands through completion or actionable divergence.

**Coverage and completion:**

<a id="f9"></a>

**F9** — **Record and replay a match.**

Identity/scenario/seed → scenario initialization → timed commands → completion/stop or actionable divergence. Shared deterministic commands do not make replay a save restore or LAN session.

**Starting evidence:** [E10 Persistence networking and presentation](2026-09-11-gameplay-boundary-evidence.md#e10-persistence-networking-and-presentation).

<a id="goal-76"></a>

### 76. LAN multiplayer

**Selection:** Host/join through synchronized play, observation, outcome and disconnect handling.

**Coverage and completion:**

<a id="f10"></a>

**F10** — **Host, join, play, observe and leave LAN matches.**

Discovery/lobby/content/seed/options/transfer → launch → synchronized real peers → communications/alliances/observers → result/disconnect/recovery-or-abort. Select complete children such as host-to-finished-match or transferred-map-to-play. Staging/network internals alone do not establish playable LAN.

**Starting evidence:** [E10 Persistence networking and presentation](2026-09-11-gameplay-boundary-evidence.md#e10-persistence-networking-and-presentation).

<a id="goal-77"></a>

### 77. Chosen online service

**Selection:** After an explicit service/support decision, actual multiplayer handoff and return.

**Coverage and completion:**

<a id="f11"></a>

**F11** — **Use a chosen online service to enter and leave actual matches.**

Explicit service/support decision → session/account/chat/discovery → actual multiplayer handoff → return/disconnect. Reuse F10 match authority; settle service policy before a retail-equivalent online claim.

**Starting evidence:** [E10 Persistence networking and presentation](2026-09-11-gameplay-boundary-evidence.md#e10-persistence-networking-and-presentation).

## Choose a porting goal before selecting coverage

The purpose is to port active retail behavior into VERA20k through research,
implementation, integration and demonstrated comparison. Distinguish three levels:

- **Porting goal:** the user-visible capability or family whose retail behavior
  the owner must complete. It can span multiple PRs and resumptions.
- **Coverage item:** a behavior, variant or interaction that the goal must account
  for. The coverage identifiers name these items, not separate sessions.
- **Implementation increment:** a coherent, reviewable change within the goal.
  Finishing it does not close the remaining goal.

Group work by implementation leverage: shared behavior research, state authority,
production callers and validation fixtures that a single change can improve together.
The player-visible result remains the acceptance bar, but a common gameplay theme
alone does not establish an efficient implementation group. Neither does sharing
one dispatcher, wallet or generic helper.

### Implementation groups supported by the inspected source

These are starting proposals for selecting goals, not a task queue. They describe
shared work at the inspected baseline, not how much remains or measured effort
savings. The [implementation evidence](2026-09-11-gameplay-boundary-evidence.md#e11-implementation-grouping)
records the source paths and limits. Preserve existing working behavior.

| Work to consider together | Why it can save repeated work; where to stop |
|---|---|
| War and Chrono Miner harvesting, return and unloading (R1) | Both enter the same Harvest/state-machine and refinery sequence. Research/fix their common decisions once, cover drive/teleport differences and the next trip. Include resource supply changes needed for the selected result. |
| TIBTRE placement, ore/gem cell mutation and growth (within R1) | Shared live resource authority joins producer and consumer. Combine when the change touches that authority; exercise real harvesting. A TIBTRE-only admission fix need not reopen every correct refinery branch. |
| GI and Guardian GI deployed combat (U3) | Same sustained-stance predicate and mission/weapon consumers. Cover both variants through actual fire and recovery. Other deploy effects need affected-branch checks, not automatic full radiation/pulse implementation. |
| Factory queue lifecycle across product categories (B1) | Enqueue/cancel/held identity/completion share an owner; mobile delivery and building/wall placement are terminal consumers. Include affected delivery variants. MCV conversion and sale do not become queue work merely because they involve buildings. |
| Spawn-pool launchers (U8) | Carrier/Destroyer and V3/Dreadnought/Boomer use the same manager and child-slot lifecycle. Cover returning aircraft and expendable missile branches through repeated attacks. Child flight changes may require a narrower additional implementation focus; Boris uses another owner. |
| American and Tech Airport paradrops (S8) | Both dispatch to one launch handler with payload/provider branches and the same carrier construction. Complete both variants through usable landed passengers. Spy Plane is not proven to be another variant of this handler. |
| Iron Curtain and Force Shield protection (S3–S4) | Shared invulnerability state/application and damage consumers make joint protection work a strong candidate. Keep their different recipient selection, infantry handling and Force Shield blackout/recovery explicit. Shared protection does not imply identical launch semantics. |
| Stock MCV conversion variants (B2) | Shared replacement/transfer/removal flow supports working across stock variants. Include sale machinery where reverse conversion actually uses it; it does not imply a full purchase/repair/sale goal. |

### Further cross-row groups and broad rows to subdivide

The [second-pass evidence](2026-09-11-gameplay-boundary-evidence.md#e12-further-implementation-groups)
adds the following implementation boundaries. “Together” means the named shared
behavior through its real callers; it does not absorb every behavior of each row.

| Implementation group | Coverage to combine and work to keep distinct |
|---|---|
| Cargo departure and failed-release recovery | U4/U5/S8 cross at cargo-head removal, recorded size and retry restoration. Handle affected vehicle, landed-aircraft, garrison and paradrop routes together, through successful release and retry. Keep route geometry, cadence, parachute attachment and weapon-reset differences explicit. This does not require all garrison combat or the entire strategic-power lifecycle. |
| Passenger-dependent weapon behavior | Within U4, boarding/release feeds IFV host-weapon selection and OpenTopped registration. Shared membership changes require both consumers. Full IFV gunner behavior and passenger-owned firing are distinct implementation focuses; a cargo list alone proves neither. U5 occupied-building weapon substitution also needs checks when shared selection changes. |
| Jumpjet movement across active users | Within U9, shared altitude/acceleration/landing decisions justify variant work together. Nighthawk cargo, Disc drain and Siege Chopper deployed combat add their own state and acceptance; treat them as explicit additional focuses instead of silently including them all in a locomotion fix. Whole-unit goals still include each required ability. |
| Per-viewer reveal publication | W4/S10 meet at real Psychic Reveal → vision publication. Changes to viewer propagation or knowledge state should include this strategic source alongside affected ordinary sources. Temporary reveal, whole-map exploration, Gap and cloak detection retain different producers and lifetime rules; do not equate them. |
| Options profile and its consumers across screens | Within F3, launcher and in-game options share the retail options profile and persistence. Group profile/consumer changes across both screens, with each apply/cancel/preview policy. Menu navigation/routing is a separate focus from settings behavior. |
| Existing-context save restoration | Within F8, quickload and the in-game load panel converge on preparation and commit. Group failure isolation, world replacement and output rebuilding across those entries. Cold-start content preparation/loading is a distinct additional focus required for full F8. |
| Fixed/generated-map launch handoff | F1/F2 share the accepted-map loading handoff. Group changes to that handoff with both map sources; generator algorithms/seed semantics and match outcome handling are separate work. A generated preview is not the gameplay map acceptance test. |
| AI base decisions and Team execution | F4 contains separate production/deployment/placement decisions and Team script/effect execution. Select the changed owner and its real consumers; integrate both when completing a whole AI opponent. Do not assume either implementation will cheaply finish the other. |

For example, “port passenger release and recovery across the live cargo routes”
is a coherent cross-row goal; “port all transports, garrisons and paradrops” is a
larger explicit scope. Similarly, a whole AI or save-system request can retain one
goal while using the distinct implementation focuses above internally. These
subdivisions are not a return to disconnected infrastructure phases: each selected
focus must deliver its named behavior through the production path.

### Conditional combinations and useful separations

- **Standard and Slave Miner:** combine resource extraction or payout changes
  where both use the changed contract; check both consumers. Slave assignment,
  relocation, replacement and liberation have a separate worker lifecycle (R2).
  Porting that entire lifecycle is not a cheap extra inferred from shared ore.
- **Strategic powers:** work on common availability/charge/launch support once
  when needed and validate affected real powers. S3–S4 and S8 have stronger effect
  implementation overlap. Storm scheduling, mutation and reveal use distinct
  owners; S1/S5/S6/S9 need further effect-path investigation before claiming an
  efficient grouping. “All strategic powers” remains a valid explicitly requested
  goal, but is not the default inferred from shared sidebar machinery.
- **Tank Bunker and special abilities:** the cargo-departure group above does
  not include NATBNK's reciprocal vehicle link. Inspect changed admission,
  targeting, damage or release owners before combining additional full effects;
  sharing Enter or an effect dispatcher is insufficient.
- **Repair, sale, service and capture:** generic money/ownership services support
  different transactions. Combine a demonstrated shared defect and its consumer
  checks, not every complete transaction merely because it changes house state.
- **Scenarios, campaigns, media, replay and multiplayer:** no new full-scope
  efficiency grouping is established here. The more specific shell, save and AI
  findings above do not imply these other backends are cheap additions.

Before composing a goal, identify what shared research/change will be done once,
which additional variants need mostly branch-specific work, and which would add
an independent lifecycle or substantial investigation. No numeric effort estimate
is required; uncertainty stays explicit. Include an additional variant when the
shared work provides a concrete reason, not just because its file is nearby.

Do not stop at a shared helper: the selected goal must reach real retail behavior
through its consumers and continuation/cleanup. Separate implementation increments
are fine within that goal. A regression check of an adjacent consumer does not
claim its full parity, and discovered required work cannot be relabeled adjacent.
An explicit full-family request retains its entire completion obligation even when
the implementation is best divided internally.

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
For a proposed grouping, name the shared research or implementation that makes
the included variants worth doing together.
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
> consumers. After each coherent increment, obtain fresh independent criticism
> of requirements, original evidence, the complete diff and validation; correct
> confirmed findings and repeat review until passed. Maintain affected evidence
> and documentation after independent confirmation, respecting annotation authority.
> Complete the named scope with a final audit for omissions and cross-mechanism
> gaps; a helper, passing sample or merged PR
> alone does not close it. Preserve any remaining in-scope work for continuation.

Carry the current goal-prompt skill's standing PR-creation preference into final
implementation prompts unless narrowed by the user; merge needs its own authority.
Preserve established publication authority, budgets and workflow settings. Leave
design/decomposition/tools open except for real constraints. The scope
examples in this guide also require that skill's independent correction loop and final
whole-scope acceptance when expanded into implementation prompts.
If implementation is authorized, preparation alone does not complete the task.

### Load the relevant evidence, not the whole guide

Each numbered entry links directly to the relevant companion section. These are
dated starting leads, not a fixed reading list or permission to skip newly found
consumers. Follow current source and native references as the action requires.
The phase inventory and migration history are optional coverage aids.

## How relationships determine scope

| Relationship found | Consequence for a goal |
|---|---|
| Same state machine or transaction, with different data/branches | Usually include as coverage of one goal and investigate together, with explicit variant checks. War/Chrono Miner, GI/Guardian GI deployment and different factory products are examples. |
| Direct producer and consumer | Include the integration needed for the named result. TIBTRE creates resource cells that growth and miners consume; a spawning-only test does not complete that resource loop. |
| A relationship that persists during an action | Own establishment, operation and release together: passenger/transport, controller/victims, launcher/spawn pool, aircraft/airfield. |
| Shared service, but different state and termination | Keep each complete action, and check affected consumers when changing the service. Sharing credits does not merge repair and sale; sharing ownership transfer does not merge engineers and mind control. |
| Same button, class name, visual effect or theme | Insufficient reason to group. MCV, GI, Desolator and Slave Miner all deploy, but their resulting lifecycles differ. |

**A goal can cross catalogue rows.** These entries identify behavior coverage
and important relationships; they are not walls around source directories.
A family can be requested whole or explicitly narrowed, and a large goal may take several
PRs. Neither fact permits a required consumer to be deferred while claiming the
named goal complete.

Do not merge every transitive dependency into one giant task. Stop expanding when
the selected action reaches its required continuing state through an established
consumer contract. For example, a purchase must yield a usable tank that can
receive its first order; it need not reimplement every tank weapon. If that first
order exposes a broken required handoff, include the repair in the purchase goal.

## Family scope notes

These notes apply to the linked coverage wherever it appears in the order.

### Resources and the base

Coverage: [R1](#r1), [R2](#r2), [B1](#b1), [B2](#b2), [B3](#b3), [B4](#b4), [B5](#b5), [B6](#b6), [B7](#b7), [B8](#b8).

Coverage references are identifiers, separate from the numbered selection order. Each coverage entry includes the normal
entry, actual gameplay result, feedback and continuation or cleanup. Variants
named here are starting coverage; exhaustive goals require a full active census.
Include reachable civilian, preplaced, reinforcement, elite, special-acquisition
and mode/map/campaign variants where they affect the named scope.

#### Example: resources and miners

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

### Units, passengers and continuing combat relationships

Coverage: [C1](#c1), [U1](#u1), [U2](#u2), [U3](#u3), [U4](#u4), [U5](#u5), [U6](#u6), [U7](#u7), [U8](#u8), [U9](#u9), [U10](#u10), [U11](#u11).

#### Distinct abilities that retain their own effect loops

Coverage: [X1](#x1), [X2](#x2), [X3](#x3), [X4](#x4), [X5](#x5), [X6](#x6), [X7](#x7), [X8](#x8), [X9](#x9), [X10](#x10), [X11](#x11), [X12](#x12), [X13](#x13), [X14](#x14), [X15](#x15), [X16](#x16), [X17](#x17), [X18](#x18), [X19](#x19), [X20](#x20).

These are effect coverage items. Include the relevant ones when porting a unit or
family; a specifically requested ability can also be a bounded goal. Distinct
effects do not mandate separate sessions. Each includes ordinary movement/combat
and presentation consumers required to exercise it. Exact branch coverage remains
work for the selected goal.

<a id="effect-owner-lookup"></a>

**Effect-owner lookup.** For X abilities without a specific companion section,
start with [weapon selection](../../src/sim/combat/combat_weapon.rs),
[effect dispatch](../../src/sim/combat/world_receiver.rs) and their real callers,
then trace the active native owner. The companion does not establish a full
contract for each effect. Use E4/E5/E7 for affected power, input and world
consumers, and preserve their actual admission and lifetime rules.


### World interaction and information

Coverage: [W1](#w1), [W2](#w2), [W3](#w3), [W4](#w4), [W5](#w5), [W6](#w6), [W7](#w7).



### Strategic powers

Coverage: [S1](#s1), [S2](#s2), [S3](#s3), [S4](#s4), [S5](#s5), [S6](#s6), [S7](#s7), [S8](#s8), [S9](#s9), [S10](#s10).

S1–S10 describe effect coverage, not a predetermined session grouping. Prefer
shared implementation work where established above: notably S3–S4 protection and
both S8 variants. Include granting/revocation, charge, targeting and recovery
needed by the selected powers. If the user explicitly requests all strategic
powers, keep all ten in scope across the necessary implementation increments.

Weather, protection, transformation and permanent ownership changes have different
state and cleanup. Keep their evidence and implementation responsibilities explicit
inside the selected goal. S5's stages and S8's variants remain coupled
coverage; neither effect differences nor row identifiers prescribe session boundaries.

Provider loss/capture, relevant power changes, targeting cancellation, repeated
use and save/restore belong to each selected power. Shared-framework changes
require affected-consumer checks. An unregistered INI section or an EMPulse-named
key used by the live nuke is not evidence of another active strategic power.

### Opponents and complete application flows

Coverage: [F1](#f1), [F2](#f2), [F3](#f3), [F4](#f4), [F5](#f5), [F6](#f6), [F7](#f7), [F8](#f8), [F9](#f9), [F10](#f10), [F11](#f11).



### Movement and locomotion implementation groups

Coverage: [M1](#m1), [M2](#m2), [M3](#m3), [M4](#m4), [M5](#m5), [M6](#m6).

These references refine U1/U7/U8/U9/X14 and relevant cargo/combat consumers; they
are not additional copies of those systems. Start with
[E13 Movement evidence](2026-09-11-gameplay-boundary-evidence.md#e13-movement-implementation-evidence).
Common order admission, destination replacement, locomotor install/restore and
per-object scheduling remain integrated with whichever groups a change affects.
Do not create a disconnected “movement infrastructure first” completion gate.

Installed-locomotor lifetime and order teardown can justify a cross-group goal
when the same change affects several classes. Name the affected branches and
validate their real transitions; do not infer every locomotor shares the same end
gate. Active-YR reachability is required for fallback/dormant classes. Movement
state also feeds visible facing, height, slope and shadows: use V2/V3/V4 for those
handoffs when affected, without duplicating simulation authority in rendering.

### Lighting and rendering implementation groups

Coverage: [V1](#v1), [V2](#v2), [V3](#v3), [V4](#v4), [V5](#v5), [V6](#v6), [V7](#v7).

These groups make visual work selectable in its own right, while remaining part
of any gameplay goal that changes the same output. Start with
[E14 Rendering evidence](2026-09-11-gameplay-boundary-evidence.md#e14-rendering-implementation-evidence).
They are shared-work proposals, not a claim that all remaining parity gaps are known.

Changes crossing these groups stay coherent: for example V2's body mask can affect
V3, and V4's depth ordering can affect V3/V5. Include that handoff and affected
output without silently absorbing every independent producer. Palette changes must
respect indexed/remap data and destination color arithmetic; body geometry, cell
illumination and shadow darkening are different responsibilities.

Use saved retail comparisons and relevant native fixtures with their stated
coverage. Validate the production GPU output through captures/readbacks where
needed; CPU math and diagnostic asset renders alone do not establish the scene.
For scale-sensitive changes, check the affected workload under ENGINE's scale
contract. Extra zoom/filtering behavior needs an explicit VERA target and must not
silently change the retail-size comparison bar. This catalogue pass ran no game,
GPU experiment or performance benchmark.

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
> evidence. After each coherent increment, obtain fresh independent review of
> original evidence, the complete diff and validation; correct findings and repeat
> review until passed. Update affected evidence after confirmation and finish with
> a whole-scope omission/regression audit. This does not certify the entire infantry
> roster or either unit's unrelated abilities. Preserve outstanding in-scope behavior
> for continuation. Apply the goal-prompt skill's standing PR-creation preference,
> preserving any narrower instructions and separate merge authority.

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
open again. Reassess after a completed goal and actual match feedback. The
[suggested order](#suggested-order) is an adaptable
selection guide, not a “finish all resources, then all units” gate.

**Evidence limit:** this examination used main
`ed8f4837910be9329505c3dfc2fc074d9c1f3106`. No game run or fresh native execution
was performed for this revision. Current paths were read directly; existing
native reports were used with their limits and stale current-status claims
excluded. The companion evidence distinguishes supported relationships from
remaining uncertainty. This is a better-grounded planning catalogue, not an
exhaustive retail-parity certification.
