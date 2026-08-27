# Phase 3 Ship Bridge-Z and Active SQD Mechanism Design

Date: 2026-08-27

Status: BLOCKED ON EXACT RUNTIME-CRATE PREREQUISITE, THEN FRESH DESIGN CRITIC.
This replaces the rejected bounded attachment design. Successful stock SQD
Attach can synchronously pick up a water crate beneath a factory-delivered
naval victim, while Rust currently owns placement only. The shared runtime
crate mechanism must be evidence-closed, built and independently passed before
this design can be implementation-ready. After that repair, a fresh read-only
critic must return APPROVE with no findings. GSI-04.03 remains OPEN after this
mechanism until the separately owned ChronoWarp and IsLocomotor release
integrations are implemented and independently passed.

Scope:

- Mechanism A: recognized Ship destination installation, bridge-Z braking,
  destination-delay admission, and terminal stored-Z arrival.
- Mechanism B: the complete active-retail Naval+Organic Parasite path used by
  SQD, including construction, LimboLaunch, fire lock, attachment, victim-tail
  grapple states, animation/RNG/audio production, damage/Culling/experience,
  delayed sinking, suppression, Detach/re-entry, lifecycle, persistence, and
  release producers whose parent effects already exist in Rust.
- Blocking prerequisite, separately built and reviewed: exact shared active
  runtime crate placement/slot maintenance, pickup, selection, all retail
  effects and immediate replacement. Placement must use native Overlay
  constructor/Unlimbo/Mark and retain a timer slot without an overlay when Mark
  fails. It must own trigger callback, weighted selection/RNG, guards/water
  downgrade/FreeMCV, removal/replacement, all active stock effects and their
  audiovisual outputs. It must serve ordinary collectors and SQD Attach's Ship
  ForceTrack call through one authority, including effect-driven collector
  death/limbo reentrancy.
- Next mechanisms, explicitly not hidden residuals: Chronosphere/ChronoWarp
  source selection and launch, and IsLocomotor/PerformDeploy. Their exact SQD
  release contracts are known, but their parent effects are absent. Neither
  may call Detach early or approximately.

Primary evidence:

- [doc: PHASE3_SHIP_BRIDGE_Z_ADJUSTMENT_GHIDRA_REPORT.md §§14-22]
- [doc: FOOTCLASS_SET_DESTINATION_GUARD_RECONCILIATION_GHIDRA_REPORT.md]
- [doc: PARASITE_CLASS_GHIDRA_REPORT.md] only where not superseded by the
  primary report
- retail rulesmd.ini sections SQD, SquidGrab, SquidGrabE and ParasitePlus
- retail artmd.ini sections SQDG, WAKE1 and H2O_EXP1/2/3

## Goal

Make stock Ship destination height behavior and its active SQD-owned guard
retail-exact without a synthetic timer writer, partial grapple, generic target
boolean, host trigonometry, immediate cull deletion, or unconditional Squid
death. Preserve every already-correct Drive, Walk, bridge, animation, damage,
veterancy, and lifecycle behavior outside the proven boundaries.

## Architecture Context

Rule parsing flows from rulesmd.ini and artmd.ini into RuleSet. ObjectType
already owns Naval, Trainable, WeaponsFactory, integer Size, weapons and cost;
WeaponType already owns Damage, ROF, Projectile, LimboLaunch and IsSonic;
WarheadType already owns Parasite, Culling, IsLocomotor and Conventional, but
the Conventional provenance comment is wrong (`+0x14D`, not `+0x14B`) and the
distinct native `+0x14B` Sonic field is absent. Weapon `IsSonic` remains the
separate firing/Wave authority. The active SQD inputs still missing are
Warhead Sonic and Paralyzes, Organic, Parasiteable, SuppressionThreshold and
General SplashList.

GameEntity is the persistent Foot/Techno owner. It already holds exact stable
identity, lifecycle/limbo state, health, NativeF32Bits veterancy, typed
locomotor state, NavCom-related movement state, radio contacts, body
FacingClass, RockingState, and manager payloads such as SpawnManager. Entity
references use stable IDs.

Combat's current `AttackTarget` incorrectly co-owns target identity, cooldown,
burst progress and inter-shot delay. Native Techno target assignment clears
target and CurrentBurstIndex while preserving the object-owned rearm timer.
That difference is active when a Dolphin Sonic hit releases SQD, so rearm and
burst progress must become GameEntity-owned before the callback can be exact.

ObjectSubstrate owns EntityStore, AnimStore and the shared LogicVector.
Simulation::for_each_live_object re-reads vector length after every callback,
so a new tail-registered projectile or animation may run later in the same
pass. The entity closure runs parent AI, locomotor Process, movement tails and
object_ai_post_movement_promote_one. The current global rocking::tick runs
after the whole live pass; that placement is wrong for SQD because native
Techno rocking precedes the victim-tail grapple write.

Combat already creates persistent Projectile objects with source ID, concrete
entity target, weapon and warhead payload. The exclusive special-detonation
chain recognizes Parasite but logs it as unsupported. The firing path does not
yet execute LimboLaunch or the Parasite fire-lock writer. Lifecycle mutations
must remain Simulation-owned rather than raw EntityStore edits in pure combat.

Crate simulation currently owns only an approximate initial placement subset and explicitly
omits runtime pickup, weighted/predetermined choice, effects, regeneration and
replacement. Successful Attach calls Ship ForceTrack, which invokes that
shared dispatch synchronously. A stock naval factory can deliver and leave an
idle ship on a pre-existing water crate, so this dependency is not excludable.
The current direct overlay stamp is itself wrong: native construction reaches
Overlay Unlimbo/Mark and ignores failure only at the slot wrapper.

AnimStore and LogicVector already provide native live-tail insertion,
constructor-time delay-zero Middle, the universal constructor first-AI guard,
owner-relative coordinates, serialized state, Report sound events and exact
frame timers. The generic spawn helper allocates its ID internally, so a narrow
reservation surface is still required for SQD's allocation-before-list-RNG
contract; the existing guard already expresses the correct SQD timing.

map::retail_trig already loads and whole-table-verifies the executable's shared
Math::SinFromTable/Math::CosFromTable data. SQD can reuse it with exact x87
phase/store adapters. It must not use the small quarter-facing native_trig
table or host sin/cos.

The damage pipeline already owns armor/Verses damage and NativeF32Bits
veterancy accumulation. Repair Depot, Iron Curtain, teleport movement,
SpawnManager teardown, central UnInit/pointer expiry, Reveal/Unlimbo,
occupancy, radio contacts and sound/effect queues have integration seams.
Positive-health Unit sinking is absent. Native sinking keeps normal visibility,
cell occupation, missions, targeting, fire, reload and later Unit AI active;
only locomotor Process is gated while descent, pitch and wakes continue.

Ship locomotion already has typed ShipLocomotionRuntime.destination and strict
slowdown comparison. movement commands, navcom, scatter, movement_tick,
ResolvedTerrainGrid, PathGrid and native_x87::distance_3d_leptons are the
correct owners for the height half.

## Impact Analysis

| Area | Existing owner | Planned change and risk |
|---|---|---|
| Rule inputs | rules/warhead_type.rs, weapon_type.rs, object_type.rs, ruleset.rs | Retain Warhead Conventional, correct its provenance to `+0x14D`, add distinct Warhead Sonic at `+0x14B`, and add exact defaults/widths for Paralyzes, Organic, Parasiteable, SuppressionThreshold, SplashList, type/global SinkingSound and VoiceSinking. Preserve WeaponType IsSonic as a distinct firing/Wave field. Wrong authority/defaults change stock admission. |
| Persistent state | sim/game_entity.rs, sim/components.rs | Add manager/link, raw timers, fire lock, exact native rocking overrides, sinking flag/prior latch and output-only sink-audio ownership. Gameplay fields affect save/hash/lifecycle; local sound handles do not. |
| Weapon rearm | sim/game_entity.rs, sim/combat/mod.rs, mission/concrete_effects.rs | Move the Techno-owned rearm timer and current burst index out of target identity. Sonic Assign_Target(NULL) clears target/index but preserves the timer; all existing firing/retarget timing must remain exact. |
| Runtime crates (blocking prerequisite) | sim/crates.rs, overlay/cell authority, lifecycle/effects | Correct placement to native constructor/Unlimbo/Mark, including timer-slot-without-overlay failure; complete the shared slot, pickup, selection, effect and immediate-replacement transaction before A+B. ForceTrack must call it synchronously and then re-read collector lifecycle. |
| Parasite/SQD | new sim/parasite_attachment.rs and sim/sqd_grapple.rs | Central Rust-native reciprocal ownership, FSM, visual/RNG, damage/cull and Detach. |
| Fire/projectile | sim/combat, sim/projectile.rs, Simulation inline hook | Add pre-RNG fire lock, post-projectile LimboLaunch, and exact Attach at detonation. |
| Scheduler | sim/world/mod.rs, world/techno_ai.rs, sim/rocking | Move rocking to its per-entity native point; preexisting sinking gates only locomotor Process; SQD runs after locomotion, then same-visit sinking and later Unit continuations run before promotion. |
| Anim | sim/anim_class.rs, world/lifecycle.rs | Add reserve/commit, moving-owner coordinates, SQDG frame/timer and listener operations. |
| Damage/sinking | sim/combat/damage, combat/veterancy.rs, world/techno_ai.rs, util/native_x87.rs | Normal receiver for damage; direct accumulator for cull; pure-integer PC53 health/pitch math; positive-health sinking continuation. |
| Releases | damage receiver, building_dock.rs, superweapon, teleport, grinder/entry, lifecycle | One reciprocal-link Detach at each proven point. Broad selected targets never release. |
| Nearby placement | sim/find_nearby_cell.rs, pathfinding/cell_entry.rs, map/resolved_terrain.rs | Add an SQD Detach query adapter with every native flag and close the existing bridge-aware seed-level residual before reuse. Wrong bridge/query flags kill or misplace SQD. |
| Ship destination | movement/navcom.rs, movement_commands.rs, bump_crush.rs, drive_locomotion.rs, movement_tick.rs | Transactional setter, signed terrain Z, 3D braking, stored-Z terminal predicate. |
| Persistence | snapshot.rs, world_hash.rs | Snapshot 113; reject 112; validate/hash every logical state edge. |

The largest risks are LogicVector timing, moving existing rocking, lifecycle
reentrancy during state-4 damage, exact Anim identity, and Squid Reveal
failure. All are covered by production-path tests.

## Chosen Approach

After the separate runtime-crate prerequisite passes, use two explicit
Rust-native subsystems sharing GameEntity state:

1. Ship destination/height remains locomotor-owned and reads an owner timer.
2. ParasiteAttachment owns one optional attacker manager, one victim backlink
   and every attach/detach lifecycle transaction.
3. SqdGrapple owns the Naval+Organic victim-tail update and its exact state,
   visual, RNG, damage and cull behavior.

LogicVector remains scheduling authority and AnimStore remains visual/audio
object authority. Existing combat, veterancy, lifecycle, placement and
movement helpers are reused through narrow exact adapters. There is no global
Parasite singleton, raw pointer vector, native vtable, or duplicate visual
scheduler.

This approach replaces the bounded timer/link proposal because that proposal
changed normal SQD combat and Detach outcomes. It also avoids a literal native
class port whose unsafe ownership buys no behavior.

The design is self-approved only for independent critic submission under the
user's autonomous instruction. Adversarial self-review identified approximate
trig, wrong live-tail timing, immediate cull deletion, failed re-entry, delayed
Report timing and broad Detach calls as the largest ordinary-skirmish risks;
each is milestone-blocking below.

## Player-Experience Detail Ledger

- MILESTONE-BLOCKING — Ship setter guard order is dying/sinking,
  destination-delay active, warp-out, warp-in. Rejection mutates nothing. A
  paused CdTimer is active whenever raw duration is nonzero. [doc: primary
  §§14-16]
- MILESTONE-BLOCKING — recognized Ship stores exact destination XYZ; target
  structural bridge adds 416 once; braking uses destination-cell ground and
  native 3D distance; terminal clear also needs strict wrapping
  abs(current-Z minus stored-Z) below 208. [doc: primary §§14.1-14.5]
- MILESTONE-BLOCKING — destination delay is first written on the victim's next
  Foot tail after admitted attachment, never at detonation. A command in the
  intervening window remains legal. [doc: primary §17.4]
- MILESTONE-BLOCKING — manager existence comes from the rookie primary
  Parasite warhead; target fire lock is signed current+20 at LimboLaunch;
  attacker is limbo before projectile travel. [doc: primary §17.2]
- MILESTONE-BLOCKING — CanAttach has exactly eight gates: victim non-null,
  non-limbo, native-alive, nonzero health, no backlink, Parasiteable, no
  conflicting installed/contained relation, and for Naval owner a known cell
  must be water. Null cell is admitted. No alliance, Verses, mission, poison
  or owner-house gate exists. [doc: primary §17.3]
- MILESTONE-BLOCKING — Attach resets FSM/visual/listener/attack timer.
  Success writes victim backlink then manager victim, leaves attacker limbo,
  and does not arm destination delay. Failure attempts fallback Reveal and
  removes only on placement failure. Restore is the cached pre-limbo cell
  center at facing zero; it never runs a nearby search. Success first sends
  victim exact XYZ plus selector -1 to the attacker locomotor and ignores the
  HRESULT. [doc: primary §§17.3, 19.5]
- MILESTONE-BLOCKING — every water/null-cell tail reanchors current
  Paralyzes=32767 before the switch. First damage is update 70 and repeats
  every 40 surviving updates; ROF=99 is ignored. [doc: primary §§17.4-17.5]
- MILESTONE-BLOCKING — state 4 uses current slot-0 weapon: 15 for
  rookie/veteran, 40 elite. Culling uses pre-damage status: rookie/veteran red;
  elite red or yellow. [doc: primary §17.7]
- MILESTONE-BLOCKING — cull suppresses ReceiveDamage, awards exact trainable
  experience, Detaches, enters positive-health sinking, then issues
  UnitLost/StopFiring. Sinking commits Z-minus-5 before testing altitude, removes only
  below -400, and on each surviving global frame divisible by four consumes
  two rejection-sampled RNG[-170,170] results (Y then X) plus a second altitude
  query before optional WAKE1 allocation. A cull set in the victim Foot tail
  reaches its first descent later in that same Unit visit. [doc: primary
  §§17.7, 19.4, 20]
- MILESTONE-BLOCKING — an already-sinking Unit updates forward pitch at Techno
  rocking with exact PC53/chop binary32 stores, and Foot audio emits the
  VoiceSinking then type/global SinkingSound edge before the victim tail. A
  fresh tail cull therefore descends immediately but pitch/audio begin next
  visit. [doc: primary §20]
- MILESTONE-BLOCKING — ordinary Naval Detach applies the exact wrapping
  `0x3FFF` facing transform, checks its one adjacent candidate, then only on
  first-passability failure uses the victim-typed/frame-indexed ring fallback.
  It validates the selected cell in native order, reappears SQD at cell center
  with rounded facing, and arms owner delay for 3 times current-primary ROF.
  Only failure removes it. Lost-water and cull overwrite owner delay to zero.
  [doc: primary §§17.9, 19.5]
- MILESTONE-BLOCKING — damage-event Warhead Sonic, not WeaponType IsSonic,
  Detaches before downstream damage. Negative damage arms manager damage timer
  50 then Detaches; qualifying third-party positive damage arms damage*2 minus
  SuppressionThreshold without advancing FSM. [doc: primary §17.8]
- MILESTONE-BLOCKING — release requires the reciprocal installed link.
  Accepted repair, non-Organic Iron Curtain, teleport, grinder/building
  consumption call Detach at their exact parent point. Endpoint expiry uses a
  separate timer/attacker-IC remove-or-re-place transaction, never normal
  Detach. Rejected repair, Organic Iron Curtain and broad targets do not. [doc:
  primary §§17.8-17.10, 21]
- COMPOUNDING — state-2/3 rocking stores one of twenty exact f32 bit patterns;
  negatives are not sign-negated positives. Techno rocking clears first and
  victim-tail SQD writes last. [doc: primary §18.1]
- COMPOUNDING — SQDG counters continue after visual failure. Delay-zero Middle
  is synchronous; manager then writes exact frame/timer and binds victim; a
  same-pass Anim visit cannot overwrite. [doc: primary §18.2]
- COMPOUNDING — each wake consumes two draws before allocation. Splash consumes
  side draw, allocation, then list-index only on successful allocation. Three
  successful splashes plus surviving transition consume thirteen draws.
  [doc: primary §§17.6, 18.2]
- COMPOUNDING — a splash's same-pass visit only clears its first-AI guard; the
  next pass changes delay 2 to 1 and the following pass reaches Middle/Report.
  None of SQDG, WAKE1 or H2O_EXP1/2/3 has RandomRate. [doc: primary §§18.2,
  19.2; ini: artmd.ini]
- COMPOUNDING — manager/counters/Anim/listener, links, fire lock, destination
  delay, exact rocking and sinking serialize/hash. On load, the FSM and links
  survive but both manager timers are deliberately reset to restored-current/0;
  they do not round-trip. Native uninitialized middle timer dwords are never
  modeled. [doc: primary §§17.10, 18.1, 19.3]
- NEXT-MECHANISM BLOCKER — ChronoWarp releases linked Naval parasite during
  fixed source scan before later warp eligibility, after arming damage timer
  500. Current two-click launch is absent. [doc: primary §18.3]
- NEXT-MECHANISM BLOCKER — admitted IsLocomotor PerformDeploy kills target
  spawns, releases actual Naval attachment, then swaps locomotor. Current
  action is unsupported. [doc: primary §18.4]

There is no exactification or unknown-risk residual inside Mechanisms A/B.
The named next mechanisms keep the GSI row open and receive their own complete
design/builder/critic cycles.

## Design

### Components

#### Rule fields

WarheadType retains `conventional: bool` and its existing `Conventional=` parser,
correcting only its provenance from the wrong `+0x14B` comment to native
`+0x14D`. Add the separate `sonic: bool` field at native `+0x14B`, parsed from
`Sonic=` default false, plus signed i32 Paralyzes default 0. ObjectType adds
Organic default false, UnitType Parasiteable default true, and signed
SuppressionThreshold using the verified constructor default plus retail SQD
override 250. ObjectType also adds optional SinkingSound and VoiceSinking;
GeneralRules adds ordered SplashList and the optional `[AudioVisual]`
SinkingSound fallback. Missing/invalid sound names mean None/native -1. Existing
WeaponType::is_sonic remains authoritative only for firing/Wave behavior; it
must never drive attachment release. Parsing tests pin omission, signed values,
retail list/sound values, and mismatched Weapon IsSonic/Warhead Sonic fixtures.

The later IsLocomotor mechanism must also add the warhead locomotor CLSID and
native-double Size representation; it may not reuse current integer Size for
fractional modified data.

#### Persistent state

GameEntity adds:

- parasite_manager: Option<ParasiteManagerState>
- parasite_attacker_id: Option<u64>
- parasite_fire_lock_until: i32, constructor zero
- foot_destination_delay: CdTimer, constructor current/zero
- weapon_rearm: CdTimer, constructor current/zero
- current_burst_index: i32, constructor zero and snapshot-validated nonnegative
- native_rocking_override: NativeRockingOverride with optional raw sideways and
  forward `NativeF32Bits`
- unit_sinking: Option<UnitSinkingState>

ParasiteManagerState owns victim_id, damage_timer, attack_timer, grapple state
0-4, major, subframe, sqdg_anim_id and anim_listener_registered. The attacker
ID is implicit in the GameEntity owner. All map, production and limbo
construction paths call one manager-initialization helper so no spawn route
misses it. Both manager timers initialize current/zero.

There is no separate global manager vector. Deterministic scans use the native
LogicVector order or stable order only when ordering has no consumer.

`weapon_rearm` and `current_burst_index` replace the target-owned
`cooldown_ticks`, `burst_remaining` and `burst_delay_ticks` representation.
The one timer is armed with native GetROF's 3-5-frame mid-burst result or the
ordinary ROF result and continues to expire by binary frame with no target.
CurrentBurstIndex is incremented/reset by the existing fire transaction; the
adapter derives any remaining-shot view only transiently from weapon Burst and
the stored index. `AttackTarget` retains target identity and the target-latched
pending Infantry fire record. Retargets preserve both entity-owned fields.
All current firing, passive-repick, force-fire, target-loss, serialization and
hash paths migrate together; there is no temporary duplicate countdown.

Simulation adds `parasite_world_placement_active`, false during construction/
teardown, set true only after scenario start or snapshot-load registration, and
cleared before victory/defeat/Exit teardown expires objects. It mirrors native
`0x00A8ED5C` solely for PointerExpired's placement permission. Active snapshots
do not serialize/hash it; load explicitly reinstates true after fixups, while
teardown tests drive false before expiry callbacks.

Every Unit owns `UnitSinkingState { active, previous }`; non-Units store None.
The two booleans correspond to positive-health sinking `+0x3CD` and native
prior-state latch `+0x3CE`, so load cannot replay the transition edge.
Wake cadence is derived from global binary frame and has no per-unit counter;
ordinary XYZ is descent/removal progress. The prior latch is serialized for
audiovisual resume but excluded from gameplay hash because it only gates output
events. The state does not set normal dying or zero health until removal.

`NativeRockingOverride` is required because SimFixed cannot represent either
the verified SQD sideways stores or sinking forward-pitch stores. Ordinary
RockingState remains the existing authority until an override is needed. A
pure-integer accessor converts its signed fixed raw numerator through
`X87Chop53::load_i64`, divides by exact `2^16` (`65_536`, the denominator of
the signed I16F16 `SimFixed` raw value), and `store_f32`s toward zero;
no host float conversion occurs. Techno ordinary rocking clears/replaces the
relevant override; victim-tail SQD may replace sideways, while sinking
RockingUpdate owns forward. Presentation prefers each present raw axis and
falls back independently to RockingState, so the two mechanisms compose.
Forward override bits serialize and hash as deterministic simulation/render
state. Local handled-audio identity remains presentation-owned and unhashed.

The shared GameEntity constructor initializes ordinary `RockingState::default()`
exactly when the resolved active-retail `is_voxel` fact is true and stores None
otherwise. The fact is successful body-VXL/HVA admission for the verified
retail asset corpus, and the shared constructor covers map, ordinary production
and limbo production. Snapshot restore preserves the serialized option and
does not rerun asset admission. Sinking never manufactures the payload. Missing
or malformed custom voxel assets are an evidence-backed non-retail exclusion,
not a reason to omit the positive stock path. [doc: primary §22.1]

#### ParasiteAttachment subsystem

new sim/parasite_attachment.rs owns manager construction, signed fire-lock
admission, read-only can_attach, attach, reciprocal validation, Detach,
manager/Anim expiry and the separate PointerExpired transaction.

DetachReason names Ordinary, LostWater, Cull, Sonic, Healing, Repair,
IronCurtain, Teleport, GrinderEntry, ChronoWarp and PerformDeploy. It prevents
callers from sharing wrong pre-writes, re-entry or owner-delay overrides.

Detach accepts attacker ID and expected victim ID, proves both directions, and
returns Reappeared, Removed, TeardownOnly or NoMatchingLink. It never clears an
unrelated timer/link on absent or mismatched ownership.

`pointer_expired(expired_id, parasite_world_placement_active)` is not Detach. Owner expiry
clears only manager owner authority before manager destruction. Animation
expiry clears Anim ID and sets the listener-removal latch only. Inactive-scenario
victim expiry clears manager victim only. Active victim expiry uses native
signed remaining on the manager damage timer and the *attacker's* Iron Curtain
predicate: positive remainder plus attacker IC inactive removes; otherwise it
re-places. The ordinary retail Organic SQD cannot retain legal IC, but the IC
branch is preserved for restored/binary state. [doc: primary §21]

Expiry removal clears the expiring victim backlink, clears manager victim,
then UnInits the attacker without a health-zero prewrite; it does not reset
manager timers, visual, FSM or listener. Expiry re-placement uses a dedicated
helper, not normal Naval Detach: try victim current cell center with copied
bridge/deck Z when Naval CanPlace accepts, otherwise choose one unrotated
facing-octant adjacent cell with no nearby scan; pass rounded current facing to
Unit Unlimbo. A scoped reentrancy guard mirrors global `0x00A8E7AC` and must
decrement on all success/failure exits.

On expiry re-placement success, perform in order: human special-selection
cleanup; team/member reattachment; conditional archive/target/destination
clears when TarCom is unset; EnterIdle; UpdateReveal; fog-border update at
attacker coordinate with Sight-3/Sight+3/0; clear manager victim last. Do not
arm 3*ROF or reset FSM/Anim/listener/victim rocking/timer. Failure writes
attacker health zero, UnInits, then clears manager victim. The victim backlink
is not explicitly cleared on re-placement because the victim is already in its
expiry transaction.

#### SqdGrapple subsystem

new sim/sqd_grapple.rs owns only managers whose owner is Naval and Organic.
tick_victim_tail follows victim backlink, proves manager reciprocity, resolves
the attacker's current slot-0 weapon/warhead, tests victim cell, writes
destination delay, then executes states 0-4. Other Parasite manager types do
not silently receive SQD cadence.

The state switch is total. Snapshot rejects outside 0-4; impossible runtime
state performs native immediate Detach without invented transition.

#### Exact math

Add a narrow adapter around installed retail_trig::TrigTable:

- sqd_rocking_bits returns all twenty verified raw results;
- sqd_facing_sin_cos applies signed-i16 current facing minus 16383, the
  verified 2*pi/65536 constant, x87 chop/table-index path, and retail reads;
- rotate_sqd_offset truncates each rotated result toward zero.

Facing authority is body_facing.current(binary_frame), otherwise facing shifted
into the 16-bit domain. Tests pin every rocking result and representative/wrap
facing rotations. Missing retail tables use the existing startup parity
failure, never host trig fallback.

#### Anim integration

AnimStore remains owner. Add narrow reserve_runtime_anim and
commit_runtime_anim operations so raw allocation precedes SplashList selection;
add set_sqdg_frame for exact frame and 128 timer;
set_anim_owner_object_if_changed; manager-listener expiry notification; and
SQDG destroy/reset.

A test-injected failed reservation preserves native mandatory RNG and consumes
no splash-list draw. Production allocation retains Rust failure semantics.

SQD call sites preserve the existing universal constructor first_ai_guard:

- SQDG/WAKE1 delay zero calls Middle synchronously;
- SQDG manager writes frame/timer afterward, then owner-binds;
- a later same-pass ordinary visit only clears the guard;
- splash delay two reaches 2-to-1 on the next pass and Middle/Report on the
  following pass.

Correct shared Anim owner coordinates to use exact moving owner X/Y/subcell and
exact world Z. Do not add an SQD-local coarse position workaround. Ordinary
ship wakes in world_effects remain separate; SQD wake/splash use AnimStore.

#### Scheduler

Split rocking::tick into tick_one plus a wrapper retained for tests. Remove the
production global post-pass call. Each entity invokes tick_one at the proven
Techno AI point before Mission/locomotor work. The existing
`GameEntity::rocking.is_some()` check remains Rust's active-retail projection
of native successful body-VXL/HVA load admission. Shared construction provides
the positive path for every verified stock voxel type; non-voxel SQD/DLPH
remains None. `tick_one` returns before every normal, ship-rocking, or sinking
branch when it is false. It must not manufacture a rocking payload merely
because `unit_sinking.active` is true. For a preexisting sinking Unit which
passes that projection, the exact sinking pitch branch
replaces normal spring rocking: read raw forward
binary32, compare absolute magnitude to `0x3F490FDB`, and if strictly below,
derive the current-facing octant; phases 1..5 add binary64
`0x3F847AE147AE147B`, phases 0/6/7 subtract it. Use
`X87Chop53::{load_f32,load_f64,add,sub,store_f32}` so the persistent store is
PC53/chop binary32. Equality/larger magnitude freezes without clamp. Sideways
angle and both velocities remain untouched. Active-retail pitch is finite;
NaN/corrupt restored rocking is rejected at snapshot validation rather than
approximated.

After locomotor/movement but before SqdGrapple, process the Foot sinking audio
edge. If current sinking differs from `UnitSinkingState.previous_flag`, entering
sinking emits optional Type VoiceSinking at current XYZ without a persistent
handle, then Type SinkingSound or General fallback through a presentation-owned
handled channel. Leaving sinking releases that channel only when ordinary move
sound is inactive. Update the prior latch in all cases, including absent,
disabled, inaudible or failed sound. The edge consumes no scenario RNG. Because
this precedes the victim tail, a newly culled Unit does not emit or pitch until
its next visit.

After locomotor Process, movement, teleport/piggyback/entry and lifecycle
drain, invoke SqdGrapple::tick_victim_tail. Anim/projectile slots never run Foot
tail. This preserves next-visit first write when impact occurs after a victim
slot and leaves the SQD rocking write after the earlier Techno rocking pass.

For Units, run `tick_unit_sinking_continuation_one` immediately after that Foot
tail. A cull which sets sinking in this very tail therefore descends in the same
visit, matching UnitClass's return from FootClass::AI. A preexisting sinking
flag gates only the locomotor Process earlier in the visit; Techno AI, Mission
dispatch, fog/visibility, rocking, sound edges, Foot tail and later Unit target/
fire/facing/reload/harvest work continue. Only the `< -400` removal returns
early. Then run `object_ai_post_movement_promote_one` at its existing point.

#### Fire and Attach flow

Parasite fire admission tests signed target lock before target mutation, shot
accounting or RNG. After ordinary projectile creation succeeds, a narrow
Simulation-owned post-fire transaction calls the production
`techno_limbo_with_rules` seam on the attacker, writes target lock current+20,
reinitializes/registers the projectile with the same payload and launches it.
It must not use test-only `Simulation::conceal`: Techno/Foot Limbo owns sensor,
radio, deposited-footprint, occupation and redraw cleanup. Projectile allocation
failure leaves attacker active and changes no attachment state.

At detonation, the Parasite special branch requires live source manager and
ProjectileTarget::Entity and calls one `attach` transaction. `attach` first
resets state/major/subframe, destroys the persistent Anim, unregisters its
listener, and resets the attack timer to current/zero; only then does it call
the read-only `can_attach` predicate. It returns without area damage or a
destination-delay write in every outcome. Rejected Attach reads the attacker's
cached pre-limbo occupied cell, resolves its centered exact ground coordinate,
and calls Simulation Unlimbo with facing zero. It does not zone-check, call
CanPlaceAtTarget, or search. Unlimbo applies its normal null/scenario/on-map,
`CanEnterCell(candidate,-1,-1,0,0)==0`, and occupation-mark gates. Failure calls
UnInit without a health-zero prewrite. Success performs ReReveal; fog-border
update at actual attacker coordinate with Sight-3/Sight+2; conditional target
and destination clears when TarCom is unset; and EnterIdle. No link is installed.

Successful Attach instead snapshots victim exact world XYZ and calls a narrow
new adapter in typed locomotor ownership that represents native attach slot
`+0x70` with selector -1 and that coordinate. Ship ForceTrack writes selector
-1/index zero, replaces destination XYZ/validity, resolves the victim cell and
calls the shared runtime-crate pickup authority synchronously. A stock naval
factory can leave an idle produced ship on a pre-existing water crate, so this
call may select and execute any active crate effect even though the SQD
collector is in limbo. ForceTrack must re-read collector alive/limbo after the
effect: the ordinary surviving attached SQD clears destination XYZ/validity
while retaining selector/index and stale head-to/speed; a dead collector keeps
the requested destination. [doc: primary §22.2]

The separately reviewed crate prerequisite must establish effect-specific
collector lifetime/reentrancy and Attach continuation before this paragraph is
final. No ForceTrack-local no-op, partial effect subset, or overlay clear is
allowed. The native outer result already proves that when dispatch/trigger
kills or UnInits the limboed SQD, ForceTrack retains requested destination and
validity and Attach still writes victim backlink followed by manager victim;
the crate contract must define the exact inactive-object representation and
avoid an invalid Rust borrow. No Reveal/Unlimbo occurs on the ordinary
successful survivor path.

#### Damage, cull and sinking

State 4 spawns/plans all three splashes first and snapshots current weapon,
warhead, rank and victim status.

Non-cull enters existing ReceiveDamage with limboed SQD source. If victim
dies/uninits, clean FSM/Anim/listener and consume no survivor RNG. If it
survives, choose state 2/3, prime once and emit wakes.

Cull calls one shared `native_health_status` helper over signed current health,
signed Strength and raw IEEE-754 bits for the already-retained native
ConditionRed/Yellow fields. No simulation-path floating-point operation is
permitted. Reuse the pure-integer `util::native_x87::X87Chop53` authority:
`load_i32(health)`, `load_i32(strength)`, `div`, `load_f64(threshold bits)`, and
`compare` reproduce the finite PC53/chop quotient and FCOMP ordering without
host math. Wrap that finite kernel in a narrow `NativeHealthQuotient` enum so a
zero Strength yields signed infinity for nonzero health and unordered for zero
health; compare those logical values without fabricating a host float. Active
retail thresholds are finite normals. Extend the same utility with a pure-
integer `load_i64` only for exact SimFixed-to-native-rocking conversion rather
than creating another floating-point emulator.

The helper performs the native red-or-unordered test, positive-health gate,
fresh red-or-unordered test, and fresh yellow-or-unordered test in exact order.
Rule loading may retain `f64` for parser compatibility but hands only `.to_bits()`
into simulation; no host divide/compare occurs in a gameplay tick. It never uses
`condition_*_x1000`, rational cross-products as the final status, threshold
clamping, or band reordering. Retail thresholds are exact 0.25/0.5; adversarial
modified CRT decimal-parser midpoint behavior is an evidence-backed non-stock
exclusion for this row. [doc: primary §19.1; ENGINE fixed-point rule]

Cull does not call ReceiveDamage. When Trainable, call the lower-level
NativeF32Bits veterancy accumulation with victim cost as points, attacker cost,
VeteranRatio and VeteranCap. Do not route through ordinary alliance/DontScore/
victim-rank kill multipliers absent from the verified cull body.

Then execute the native order exactly: Detach(Cull); set the saved victim's
UnitSinkingState.active=true without changing its prior latch; issue UnitLost;
call StopFiring; clear
visual/FSM/listener; overwrite the reappeared attacker delay to zero; return
without survivor RNG. No callback may observe UnitLost before the sinking flag.

Unit sinking is not a whole-object early return. At Foot locomotor Process, a
preexisting sinking flag skips only that physical locomotor call; a flag set by
the SQD victim tail is too late to gate that visit's completed movement but
enters the following continuation immediately. First commit exact Z-minus-5
through the ordinary marked coordinate setter, preserving remove/re-add cell
occupation and on-map state, then query altitude. If altitude is strictly below
-400, UnInit immediately and emit no wake; exactly -400 survives. On a surviving frame with
`binary_frame & 3 == 0`, call inclusive `RandomRanged(-170,170)` for direct Y,
then again for direct X, query altitude again, and form
`(X+second, Y+first, Z-altitude)`. All ranged rejection draws and the second
altitude query precede Anim allocation. Allocation success constructs Rules
Wake with delay 0, loops 1, flags 0x600, z-adjust 0 and reverse false; failure
keeps RNG consumption. There is no rotation, retained Anim pointer, sound, or
per-unit cadence state in the wake spawn itself; sinking transition audio is
the separate Foot edge above. A surviving sinking Unit remains alive, targetable,
visible under ordinary house/cloak/fog rules, registered in its normal cell and
display layer, mission-processing, and eligible for target/fire/facing/reload/
harvest work. Do not clear occupation, hide it from radar, pause missions,
suppress fire, or skip later Unit AI. Pitch and audio run at the scheduler points
above: a fresh cull descends immediately, then next visit performs the first
pitch step and emits VoiceSinking followed by type/global SinkingSound before
the tail. [doc: primary §§19.4, 20]

#### Detach and release producers

Ordinary Naval Detach snapshots victim position and the low 16 bits `F` of
current body facing, then computes:

```text
o = ((((F >> 12) + 1) >> 1) & 7)
F2 = wrapping_u16(if o <= 2 { F + 0x3FFF } else { F - 0x3FFF })
d = ((((F2 >> 12) + 1) >> 1) & 7)
face8 = ((F2 + 0x80) >> 8) & 0xFF
```

The one first candidate is victim current cell plus direction `d`, using the
canonical N,NE,E,SE,S,SW,W,NW offsets. Preserve the exact single-value boundary
anomalies proven in primary §19.5; do not replace `0x3FFF` with a quarter-turn.
Resolve the candidate and call GetZoneID with attacker MovementZone and
OnBridge=false, then CheckCellPassability with attacker SpeedType, both mask
flags false, that zone, attacker MovementZone, level=-1, bridgeArg=true.

Only when this first passability call fails, invoke a narrow exact
`find_sqd_detach_fallback(victim, frame)` adapter. It ignores the failed adjacent
cell and starts at the victim current cell, using victim SpeedType (4 normalized
to 1), MovementZone, `GetZoneID(victim_cell, MovementZone, victim.OnBridge)`,
and `bridge_aware_zone=victim.OnBridge`. Construct `NearbyQuery` explicitly with
footprint 1x1, height-aware playfield bounds enabled, required level `-1`/None,
`allow_bridge_cells=true`, caller height-difference check disabled,
`check_occupancy=false`, both passability occupancy-mask filters false, and the
literal zero target that selects frame modulo. Before reuse, fix and test the
shared helper's recorded bridge-aware seed-level residual so a bridge seed adds
the native four-level rise wherever that seed level is consumed; do not waive
it merely because older free-unit callers were non-bridge.

The scan caps radius at `min(map_width+map_height,32)`, visits each ring as
interleaved top/bottom west to east then left/right north to south, retains at
most 24 passable candidates, and applies the native first-qualifying-ring stop.
Partition coordinate-round-tripping candidates before indirect candidates,
prefer the direct list, and select `list[frame % len]`; consume no RNG.

After either first-candidate success or fallback selection, reject NullCell,
resolve centered X/Y plus exact ground Z, reject NullCoord, then call the Naval
CanPlaceAtTarget predicate and finally Simulation Unlimbo with `face8`. For
Naval SQD, CanPlace rejects missing/high-flying victim or a building in the
victim current cell and skips non-Naval land/overlay/bridge-range gates; it does
not inspect the candidate. Unlimbo independently requires
`CanEnterCell(candidate,-1,-1,0,0)==0` and successful occupation marking.

On successful Unlimbo, preserve native mutation order: the nested Unlimbo reveal
and occupation commit; human-owner special-selection clear when applicable;
team/member reattachment via the existing exact helper when team is non-null;
if TarCom is null, clear archive target, assigned target, and destination;
EnterIdle; explicit `TechnoClass__UpdateReveal(0,0,0,0,0)`; attacker Foot delay
`start=current,duration=3*current-primary ROF`; then manager FSM/Anim/listener
reset. Null cell/coord, CanPlace, or Unlimbo failure instead writes attacker
health zero and UnInits through lifecycle before that same manager reset.
Both outcomes then zero victim rocking, write victim destination delay
current/0, clear victim backlink, and clear manager victim, in that order.
LostWater and Cull finally overwrite owner duration to zero.

Wire exact existing parents:

- known non-water tail: LostWater before FSM;
- damage receiver: event Warhead Sonic first performs reciprocal Detach, then
  for a non-null source performs exact active-Dolphin Assign_Target(NULL), then
  re-reads the link before negative-heal or positive-suppression handling and
  only afterward enters downstream HP/damage work. Assign_Target always clears
  passive acquisition before its same-null short circuit. With an old target it
  clears target and `current_burst_index` but preserves the running
  `weapon_rearm`; active DLPH has no AirstrikeTeam, SpawnManager, fire-particle
  or Infantry-animation side effect. Sonic+negative damage therefore Detaches
  once and does not arm the 50-frame manager timer. Without Sonic, negative
  damage arms timer 50 then Healing Detach; positive third-party raw damage
  above the strict threshold arms only the suppression timer;
- funded RepairResponse::Roger before heal/cost; no detach for unfunded/full;
- non-Organic Iron Curtain before protection; Organic follows existing branch;
- teleport release before relocation;
- grinder/building consumption at the proven acceptance point;
- endpoint PointerExpired runs before generic references break, using the exact
  owner/victim/Anim and scenario-active branches above;
- SQDG Anim expiry clears manager visual/listener latch without victim Detach.

ChronoWarp and PerformDeploy may have private contract tests but unsupported
dispatch does not call them. Their later parent builders integrate them.

#### Ship destination and height

One read-only recognized-Ship preflight in navcom rejects, in native order:

1. dying or positive-health sinking;
2. foot_destination_delay.remaining(frame) != 0;
3. warp-out active;
4. warp-in active.

AttackTarget cooldown is never consulted. Every high-level Move, AttackMove,
RepairAtDepot, defensive, layered/direct, teleporter fallback, scatter and
internal cell setter preflights before mutation. Rejection preserves all order,
target, dock, navigation, movement, facing, occupation and manager state.

Valid cell target construction uses native signed coordinate-to-cell
conversion, centered X/Y, exact signed ground_height_leptons, and one +416 only
for structural bridge. Missing/out-of-grid terrain or slope above 20 rejects
before mutation. Resolved entity coordinate preserves incoming Z plus
conditional structural +416. Null clears.

Success atomically commits NavCom, Ship destination XYZ, movement/path and
facing. Direct same-cell and scatter use the same transaction; scatter threads
ResolvedTerrainGrid through every caller.

Ship Process uses exact current X/Y/subcell; current Z is exact_z_leptons else
signed terrain/slope plus +416 only for current on_bridge. Destination cell
comes only from Ship destination X/Y; destination ground recomputes at exact
X/Y and structural bridge adds 416. native_x87::distance_3d_leptons returns
i32; 0..32767 converts exactly and 32768+ saturates to SimFixed::MAX, preserving
strict distance below slowdown.

Terminal arrival retires executed track head, then requires NavCom cell
equality and strict wrapping abs(current Z minus stored destination Z) below
208. Clear only on both; otherwise preserve and defer retry. Remove the two
generic water-mover/current-cell bridge-Z blocks.

### Interfaces and Contracts

- can_attach is read-only.
- attach installs both links or restores/removes limboed attacker; it never
  writes victim destination delay.
- Detach requires a reciprocal pair and owns explicit release callers;
  PointerExpired alone owns endpoint-specific asymmetric clears/removal/re-entry.
- tick_victim_tail consumes one victim LogicVector turn and may synchronously
  damage, cull, Detach, spawn Anim or queue UnInit.
- Anim reservation is single-use.
- Ship setter failure leaves before/after entity state equal.
- No generic resolved target is attachment authority.
- Manager/link state is rejected rather than repaired on load.

### Data Flow

    fire admission
      -> exact target fire-lock gate
      -> create projectile
      -> Simulation techno_limbo_with_rules attacker
      -> target lock = current + 20
      -> projectile reinitialize/register/launch
      -> later impact
      -> manager FSM/Anim/listener/attack-timer reset
      -> exact CanAttach
      -> failure: fallback Reveal or remove attacker
      -> success: victim backlink then manager victim
      -> no destination-delay write

    victim LogicVector slot
      -> parent Techno AI and ordinary rocking
      -> mission/destination/locomotor/movement/entry
      -> SQD victim tail before post-movement promotion
      -> prove reciprocal Naval+Organic link
      -> non-water: LostWater Detach
      -> water/null: destination-delay refresh
      -> state 0..4 and final rocking/Anim ownership
      -> Unit sinking continuation, including same-visit cull start
      -> surviving Unit target/fire/facing/reload/harvest continuations
      -> next LogicVector slot

    state 4
      -> three splash draw/reserve/list/construct sequences
      -> pre-damage cull decision
      -> cull: exact XP, Detach, sinking, UnitLost, StopFiring
      -> normal: ReceiveDamage
      -> dead: cleanup, no retry RNG
      -> survivor: direction draw, prime, six wake draws

    victim pointer expiry before generic reference break
      -> scenario inactive: manager victim clear only
      -> active + positive suppression timer + attacker IC inactive
         -> victim backlink clear, manager victim clear, attacker UnInit
      -> otherwise bespoke current-cell/adjacent re-placement
         -> success cleanup/reveal/fog, manager victim clear last
         -> failure health zero, UnInit, manager victim clear
      -> never normal Detach, 3*ROF, FSM/Anim reset or nearby search

    command/direct/scatter
      -> read-only Ship guard
      -> validate exact target coordinate
      -> atomic NavCom + Ship destination + movement/facing
      -> destination-cell 3D braking
      -> terminal cell plus stored-Z predicate

### Error Handling

- Missing exact retail trig never falls back.
- Missing SQDG/WAKE/Splash modified data suppresses the relevant visual while
  state and mandatory RNG continue. Empty SplashList fails retail validation
  because native has no safe zero-count branch.
- Missing/mismatched links never cause broad clear.
- Pointer expiry is dispatched while the expiring endpoint/Anim facts are still
  readable and never falls through to normal Detach.
- State-4 rechecks victim after synchronous damage before survivor RNG.
- ForceTrack re-reads the SQD collector after synchronous crate effects; exact
  continuation after effect-driven death/removal comes from the independently
  passed crate prerequisite and may not retain a stale mutable reference.
- Failed Squid Reveal removes through lifecycle, never raw store deletion.
- Sinking never becomes a generic AI/visibility/occupation early return; only
  its `< -400` removal branch stops the current Unit visit.
- Invalid Ship geometry returns false before mutation; corrupt restored tick
  state remains total but cannot falsely arrive.

### Testing Strategy

Focused work uses cargo test -p vera20k --lib plus a filter. The full --lib
suite runs exactly once only after all Phase 3 mechanisms and reverse audit.

Rules/construction:

- retail SQD manager from rookie primary across map, production and limbo spawn;
  secondary-only Parasite/building controls;
- exact Paralyzes/Organic/Parasiteable/SuppressionThreshold/SplashList and
  Warhead Sonic, type/global SinkingSound and VoiceSinking parsing; Weapon
  IsSonic remains separate;
- frame-123 Foot raw destination delay (123,0), fire lock zero, empty backlink,
  no-manager versus empty-manager distinction.
- map, ordinary-production and limbo-production stock voxel naval Units start
  with `Some(default rocking)` before any impulse; non-voxel DLPH/SQD starts
  None; snapshot restore preserves both. Retail VXL/HVA audit remains zero-fail.

Fire/attach:

- signed lock deadline and wrap, checked before RNG/shot mutation;
- projectile exists before Techno/Foot Limbo; allocation failure leaves source active;
- successful launch uses full Techno/Foot Limbo sensor/radio/occupancy order;
- complete CanAttach matrix including Naval null-cell exception and no extra
  alliance/Verses/poison/mission gates;
- first-link-wins; rejected Attach cached-cell center, facing zero, exact
  Unlimbo/fog/idle success tail versus no-health-write UnInit failure;
  rejected admission still proves pre-admission FSM/Anim/listener/attack-timer
  reset/destruction;
  successful Attach locomotor selector -1/index zero, requested XYZ, shared
  synchronous crate-dispatch call and post-effect alive/limbo reread; ordinary
  survivor clears destination/validity while retaining stale head-to/speed;
  no timer write and no nearby restore search;
- prerequisite production fixture places WCRATE first, exits a naval Unit onto
  it with no rally move, proves idle movement leaves the overlay, then SQD
  Attach executes the exact selected crate effect. Cover every effect class
  that can kill, limbo, move or retarget the collector and assert the final
  Attach/link result established by the crate contract, including killed SQD
  retaining requested destination/validity and still receiving both links.
- occupied-cell random placement passes the no-occupancy finder but fails
  Overlay Mark on Unit occupation bit 0x20, retains the native timer slot and
  leaves no cell overlay; Rust must not direct-stamp a crate under that Unit.

Scheduler/FSM:

- impact after victim slot first writes on next visit; intervening Ship command
  succeeds;
- every update 1..110 pins state/major/subframe/frame, damage at 70 and 110;
- state 0 continues after failed SQDG; invalid snapshot rejects;
- all twenty rocking bits and negative asymmetry; ordinary rocking before SQD.
- cull same-visit descent but next-visit pitch/audio; locomotor-only sinking
  gate and continued Unit fire/AI order.

Anim/RNG/audio:

- SQDG Middle then manager frame/timer then owner bind, same-pass no-overwrite;
- all eight wake reservation masks consume six mandatory draws;
- all eight splash masks consume list draws only for successes;
- exact B=-128,0,128, A=+64/-64 rotations/truncation;
- delay-2 same-pass guard clear, next-pass 2-to-1, following-pass
  Middle/Report and exact list-to-sound;
- survivor state-4 thirteen draws; cull/death skips seven; no RandomRate draws.
- ordinary-rocking fallback conversion at raw I16F16 negative, zero, positive,
  and minimum/maximum values, plus independent sideways/forward override
  composition; a sinking Unit with `rocking == None` proves failed native
  admission leaves pitch and overrides unchanged;
- sinking pitch raw-bit threshold/direction/overshoot/freeze vectors; start edge
  VoiceSinking-before-type/global SinkingSound, absent/inaudible latch update,
  clear-edge move-sound guard, and no scenario RNG.

Damage/cull/sinking:

- 15/40 normal damage through receiver;
- exact native-health-status branch matrix: just below/equal/above red/yellow,
  health <= 0, zero/negative Strength, unordered, and one crafted quotient
  where host nearest-even differs from PC53 toward-zero; assert the tick path
  invokes no floating-point operation;
- cull suppresses damage/ordinary kill modifiers, exact Trainable XP, Detach,
  StopFiring and sinking;
- sinking final descent, strict -400 boundary, global-frame cadence,
  rejection-sampled Y/X order, second-altitude coordinate, allocation failure,
  delay-zero guard, same-visit cull start, locomotor-only freeze, continued
  mission/fire/reload/harvest/visibility/occupation, pitch/latch transition,
  and final UnInit;
- normal death cleanup/no retry RNG and survivor chooser/wakes.

Detach/releases:

- all facing ranges plus eight single-value boundaries select the verified
  adjacent cell and exact `face8`; valid return and stock 3*ROF=297;
- first candidate pass/fail split; victim-typed radius/ring order, 24-cap,
  direct partition and `frame % len` fallback with no RNG; bridge-seed four-level
  rise plus every explicit NearbyQuery flag;
- zone/passability argument order; NullCell/NullCoord, Naval CanPlace,
  CanEnter, mark and Unlimbo failures remove with health-zero prewrite;
- LostWater/Cull owner-delay zero overrides;
- victim delay/rocking/link cleanup;
- Sonic, negative heal, positive suppression strictness/source exemption;
- mismatched Weapon IsSonic/Warhead Sonic controls prove release follows only
  `EntityDamageEvent.warhead_ref`;
- rookie Dolphin Sonic preserves an active ROF timer while clearing target,
  passive flag and burst index; elite post-shot-1 release preserves the native
  3-5-frame wait while resetting burst; same-null clears only passive state;
  third-target, null-source and Sonic+negative controls;
- ordered trace proves `Detach complete < source Assign_Target(NULL) < link
  reread/suppression/heal < HP mutation`, with no second Detach or 50-frame arm;
- funded repair only; non-Organic Iron Curtain only; teleport, grinder and
  nonreciprocal/broad target no clear;
- pointer expiry owner-only, inactive victim-only, Anim-latch-only, active
  timer/attacker-IC removal and both bespoke re-placement outcomes; assert no
  3*ROF/FSM/Anim/victim-timer normal-Detach side effects and exact fog
  Sight-3/Sight+3.

Ship:

- preserve every prior-approved structural/current/destination, signed
  level/slope, 100/427/500, 129/128, 32767/32768, exact-Z, invalid install,
  retarget/cancel/getter, direct/scatter/same-cell and terminal boundary test;
- active SQD timer blocks every command path without mutation; ordinary weapon
  cooldown and Walk/Drive controls remain admitted;
- one end-to-end detonation-to-tail writer/consumer test.

Persistence/hash:

- snapshot 113 round-trips no/empty/active manager in states 0-4, Anim/listener,
  serialized timer words, links, lock, destination delay, weapon rearm/current
  burst index, rocking, sinking,
  Ship destination/retry; post-load rehydration resets both manager timers to
  restored-current/0 while preserving every other state field; version 112
  rejects;
- reject self/one-sided/duplicate/missing links, non-limbo active attacker, bad
  FSM/counters, SQDG/listener mismatch and inconsistent sinking/Logic state;
- sink active/prior latch and forward raw pitch round-trip; prior audio latch is
  snapshot-only while pitch is hashed; local sound handles never serialize/hash;
- one hash discriminator per gameplay-logical field; no native middle-dword
  garbage.

Production scenarios:

- retail SQD versus destroyer through fire, flight, attachment, update 70,
  damage, ordinary Detach and re-entry;
- retail factory-produced destroyer parked on a pre-existing WCRATE followed by
  SQD Attach exercises shared runtime pickup and post-effect reentrancy;
- elite SQD yellow cull, exact XP and delayed sinking wakes;
- culled live ship remains occupied/visible and can fire while its locomotor is
  frozen; first descent is same victim visit and final removal is below -400;
- command between impact and first tail succeeds, attached commands reject,
  post-Detach commands resume;
- save/load in states 0-4 proves the intentional timer restart and its later
  damage/Anim schedule; sinking load preserves later RNG/removal behavior and
  hashes the restored logical state.

## Architectural Decisions

- Follow optional manager payload and stable-ID patterns, not native pointers.
- Follow shared live LogicVector, not a new SQD scheduler.
- Use AnimStore/Report, not WorldEffect, for SQD visuals/audio.
- Reuse verified retail_trig and NativeF32Bits, not host math or embedded table.
- Move rocking to per-object order because ordering is gameplay.
- Use one two-axis raw rocking override/accessor for SQD sideways and sinking
  forward stores rather than widening all physical RockingState fields.
- Reuse lower-level veterancy accumulation while bypassing ordinary kill gates
  absent from Culling.
- Model positive-health sinking explicitly.
- Make rearm and CurrentBurstIndex Techno-owned rather than target-owned, so
  Assign_Target(NULL) cannot grant a free shot.
- Reuse one exact shared runtime crate authority; ForceTrack is a caller, not a
  second crate implementation.
- Keep ChronoWarp/IsLocomotor as named next mechanisms; this cannot close row.

No deliberate approximation is accepted in Mechanisms A/B. They remain open
until the crate prerequisite supplies the unresolved effect/lifetime boundary
and a fresh critic verifies the repaired complete design.

## Alternatives Considered

### Bounded reciprocal link and timer

Rejected: it omits routine SQD FSM, damage, Culling, visuals, RNG, sinking and
re-entry.

### Synthetic destination-delay writer

Rejected: detonation-time/cooldown writes reject a legal intervening command
and arm against failed attachment.

### Literal ParasiteClass port

Rejected: pointer vectors/vtables add unsafe lifetime coupling without behavior
not already owned by stable IDs, LogicVector and AnimStore.

### Early ChronoWarp/IsLocomotor release hooks

Rejected: detach followed by unsupported/rejected parent action is a false
stock-visible side effect. Each parent requires its own complete cycle.

### Chosen staged closure

First evidence-close, design, build and independently pass the shared runtime
crate mechanism. Feed its exact ForceTrack effect/lifetime contract back into
this document. Then obtain a zero-finding fresh design approval and build/pass
A+B as one coupled writer-consumer mechanism while keeping the row open. Then
close ChronoWarp and IsLocomotor with their own designs, builders and fresh
critics before GSI-04.03 or Phase 3 can pass.

## Review Gate

Only after the separately built runtime-crate prerequisite passes and its exact
Attach continuation is incorporated, a fresh critic who did not research/write
this design receives the GSI requirement, zero-residual bar, primary report
§§14-22, retail activation, the crate evidence/contract/diff/output, this
design, current no-code diff, and all prior findings/fixes.

Any uncited active behavior, inaccurate file/API, missing lifecycle/RNG edge,
approximate math, false release, test gap or prior Ship regression returns
REVISE/BLOCK. After repair, a different fresh critic rechecks the whole design
and prior fixes. Only zero-finding APPROVE authorizes the A+B builder.
