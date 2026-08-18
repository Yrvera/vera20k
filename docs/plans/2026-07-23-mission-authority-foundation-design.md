# Mission Authority Foundation Design

**Date:** 2026-07-23  
**Status:** Approved design  
**Implementation status:** Not started  
**Chosen rollout:** In-place exact substrate with sealed legacy compatibility adapters  
**Evidence contract:** `docs/contracts/2026-07-21-mission-state-verb-readiness-authority-implementation-contract.md`

## Goal

Replace the reduced Mission shadow representation with one native-width,
mechanism-exact Mission foundation while preserving current gameplay through an
explicit compatibility boundary and activating no new production
Queue/Commence/Override/Restore callers.

This foundation includes:

- lossless common Mission state and signed dispatch timing;
- the exact five common verbs;
- concrete Target/NavCom Override and Restore transactions;
- the Aircraft five-slot leaf policy;
- exact read-only ReadyToCommence predicates and their raw inputs;
- independent `WeaponsFactory=` rule state;
- deterministic hashing and snapshot persistence; and
- a mechanically enumerable migration boundary around current commands and
  legacy FSM projections.

It does not perform the full Mission authority flip. The current player-command
to native-action crosswalk and the legacy-FSM to native-owner crosswalk remain
prerequisites for that later change. Native `+0xCC` save compatibility is also
separately blocked on runtime residue fixtures.

The scoped mechanisms and accepted caller families have been checked as active
or conditionally active Yuri's Revenge paths. No accepted caller in the closed
ledgers was classified as dormant TS-only behavior.

## Architecture Context

The current Rust Mission layer is a partial shadow:

- `src/sim/mission/mod.rs` defines a known `MissionType` vocabulary but stores a
  reduced `MissionCom` with `MissionType`/`Option<MissionType>` selectors,
  `substate: u8`, one unsigned timer, and a refresh counter.
- `src/sim/mission/timer.rs` uses unsigned wrapping comparison and saturating
  remaining-time arithmetic. Native Mission dispatch uses signed dword
  arithmetic and a `-1` start sentinel.
- `src/sim/mission/verb.rs` implements reduced transitions. Assign clears the
  suspended slot, Selling is treated as an Assign guard, Queue lacks the
  `commence_now` chain, Override consumes the queue and resets timing, Restore
  resets state/timing, and readiness is reduced to category plus
  `is_driving`.
- `src/sim/mission/retask.rs` routes several live player commands through the
  reduced Assign helper. Four combat command families write only
  `mission.current`.
- `src/sim/game_entity.rs::derived_mission`,
  `src/sim/world/techno_ai.rs`, and `src/sim/world/mod.rs` project legacy miner,
  aircraft, docking, attack, movement, order-intent, and idle state back into
  Mission fields. These writers can clobber a correct verb result.
- `GameEntity` already owns the active combat Target. `NavigationState` owns
  NavCom and an unused suspended NavCom, but there is no suspended combat
  Target.
- `GameEntity` has Radio contacts with stable slot order. That supplies the
  required Unit readiness slot-0 identity, but the independent
  `WeaponsFactory=` type byte is not parsed.
- Aircraft gameplay is currently represented by a separate legacy FSM and has
  neither the exact Mission leaf latches nor runtime Airstrike-manager presence.
- `src/sim/world/world_hash.rs` and `src/sim/snapshot.rs` already cover the
  reduced Mission component, Navigation state, and Target state. Every new
  future-affecting field therefore crosses a lockstep and save-format boundary.

The design follows existing project architecture:

- `EntityStore` remains the deterministic `BTreeMap` owner of entity storage.
- `GameEntity` owns state with the same lifetime as the entity.
- `sim/mission/` owns common state and pure Mission rules.
- A Simulation/entity-level adapter owns cross-component Target/NavCom effects
  and read-only world lookups.
- The simulation remains independent from render, UI, sidebar, audio, and net.
- No ECS, native C++ inheritance tree, vtable replica, raw pointer store, global
  mutable singleton, floating-point simulation decision, or global promotion
  drain is introduced.

### Dependency flow

```text
current commands / legacy FSMs
        |
        v
sealed legacy compatibility adapters
        |
        v
private MissionCom state

future verified native owners
        |
        v
Simulation MissionAuthority
        |----> leaf gate / concrete Target+NavCom transaction
        |----> read-only ReadyAuthorityView
        `----> private common verb primitives

rules.ini + rulesmd.ini
        |
        v
ObjectType.weapons_factory
        |
        v
Unit readiness view

MissionCom + target archives + leaf state
        |
        +----> fixed-order world hash
        `----> versioned snapshot
```

### Primary research basis

The design uses the following current binary reports as primary evidence:

- `MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md`
- `MISSION_ASSIGN_OVERRIDE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md`
- `MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md`
- `AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md`
- `MISSION_RAW_BYTES_0XB8_0XCC_FULL_CENSUS_GHIDRA_REPORT.md`
- `READYTOCOMMENCE_UNIT_INFANTRY_RESIDUAL_CLOSURE_GHIDRA_REPORT.md`
- `READYTOCOMMENCE_AIRCRAFT_BUILDING_WRITER_LIFECYCLES_GHIDRA_REPORT.md`
- `TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md`
- `TIMER_PRIMITIVE_FAMILY_COUNTUP_TIMERCLASS_VS_CDTIMERCLASS_GHIDRA_REPORT.md`
- `FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md`

The implementation contract reconciles these sources and records superseded
claims. This design does not treat older partial reports, Ghidra labels, or
current Rust tests as parity authority.

## Impact Analysis

### Expected implementation surfaces

- `src/sim/mission/mod.rs`: vocabulary exports and removal of the reduced state
  definition.
- New focused Mission modules, expected as:
  - `src/sim/mission/state.rs`
  - `src/sim/mission/leaf.rs`
  - `src/sim/mission/readiness.rs`
- `src/sim/mission/timer.rs`: Mission-specific signed timing.
- `src/sim/mission/verb.rs`: exact private common transitions.
- `src/sim/mission/retask.rs`: explicit legacy compatibility routing.
- `src/sim/game_entity.rs`: frame-aware Mission construction, category-specific
  leaf state, suspended Target, and read-only Mission access.
- `src/sim/components.rs`: existing Navigation archive remains authoritative for
  suspended NavCom.
- A focused Simulation-level Mission authority module, rather than adding more
  gameplay ownership to `world/mod.rs`.
- `src/sim/movement/navcom.rs`: reviewed destination setter surface used by
  concrete Mission transactions.
- `src/rules/object_type.rs`: independent `WeaponsFactory=` parse and storage.
- `src/sim/world/world_hash.rs`: fixed-order folds for every new gameplay field.
- `src/sim/snapshot.rs`: one coordinated version increment and round-trip tests.
- Existing Mission, retask, world-command, entity, navigation, Aircraft, rule
  parser, hash, and snapshot tests.

Making Mission fields private will require mechanical migration of current
production and test reads. `src/sim/world/techno_ai.rs` currently contains
pre-existing uncommitted test-only work. It is not part of this design.
Implementation must re-check ownership before touching that file, preserve the
uncommitted work, and either wait for it to land or coordinate only the minimum
accessor migration required for compilation.

### Behavioral and compatibility risks

- Changing state widths or sentinel representation can silently alter mission
  comparisons if any caller truncates to `u8` or normalizes an unknown value.
- A frame-free constructor would produce a wrong dispatch start frame.
- An exact verb called from an unproven current command could change live
  gameplay before the command crosswalk is complete.
- A later Unit-host or tick-tail projection could overwrite a correct verb in
  the same tick.
- A stale readiness cache could miss Queue's just-written slot or same-object
  latch changes.
- Snapshot layout changes require one coordinated version increment. Other
  sessions must not perform a competing rebaseline at the same time.
- Hash omission of one new latch, archive, or manager-presence value could cause
  undetected lockstep divergence.
- Deriving `WeaponsFactory` from current land-factory logic would misclassify
  naval yards.
- Treating configured Airstrike keys, aircraft type, altitude, or the warhead
  `Airstrike=` key as runtime manager presence would change the Aircraft gate.

### Behavior boundary for this slice

Current commands and legacy FSM projections keep their present behavior through
named compatibility operations. Exact Queue, Commence, Override, Restore,
readiness, B8 producers, Building consume points, and Aircraft leaf operations
are testable but gain no new live production caller. This restriction is a
missing-prerequisite boundary, not acceptance of parity drift: the two
crosswalks must close before those callsites can be activated safely.

## Chosen Approach

Use one in-place, native-width `MissionCom` and seal all mutation behind:

1. exact Mission authority;
2. narrowly named verified host writes; or
3. temporary legacy compatibility adapters.

This is preferred over a parallel shadow because it establishes the final
state, hash, and snapshot representation once. It is preferred over an
immediate flip because current command and FSM mappings are not yet proven.

The exact common primitives remain private to `sim/mission/`. Production
systems reach them through a Simulation/entity-level authority adapter that
preserves receiver-family behavior and synchronous ordering. Compatibility
methods cannot call Queue, Commence, Override, Restore, readiness, or concrete
Target/NavCom transactions.

## Tiny-Detail Ledger

- Current, suspended, and queued are independent signed dwords; `-1` is none,
  and full-width values must not be reduced to a byte. `[doc:
  FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md; doc:
  AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md]`
- B8 is a raw byte, not a bool or universal ready flag. It bypasses one Unit
  movement rejection and has exactly two nonzero producer families. `[doc:
  MISSION_RAW_BYTES_0XB8_0XCC_FULL_CENSUS_GHIDRA_REPORT.md]`
- Handler state is a full dword. Assign and successful Commence reset it;
  Override and Restore preserve it. `[doc:
  MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md
  §§1-5]`
- Mission-start frame, per-mission AI counter, dispatch start, and dispatch
  delay are separate fields with separate writers. `[doc:
  FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md; doc:
  TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md]`
- Live construction initializes all three selectors to none, B8/handler
  state/mission-start/AI-counter/delay to zero, and dispatch start to the
  construction frame. `[doc:
  FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md; doc:
  TIMER_PRIMITIVE_FAMILY_COUNTUP_TIMERCLASS_VS_CDTIMERCLASS_GHIDRA_REPORT.md]`
- Dispatch due arithmetic uses wrapping signed subtraction and signed
  comparison; unsigned comparison and saturating remaining are DRIFT. `[doc:
  TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md; doc:
  TIMER_PRIMITIVE_FAMILY_COUNTUP_TIMERCLASS_VS_CDTIMERCLASS_GHIDRA_REPORT.md]`
- Effective mission is current unless current is none, then queued. No other
  idle or busy normalization replaces that lookup. `[doc:
  FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md]`
- Assign has only the Deliberate-to-Guard whole-function guard. Selling is
  allowed; suspended is preserved; queue/B8 and the exact reset fields change.
  `[doc:
  MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md
  §1]`
- Queue blocks Deliberate-to-Guard and any request while current is Selling.
  Its exact redundancy predicate controls only the queue write and B8 clear.
  `[doc:
  MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md
  §2]`
- Queue with nonzero `commence_now` calls leaf Ready and then conditional leaf
  Commence synchronously, even after a redundant or none request. Flag zero
  never self-promotes. `[doc:
  MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md
  §4.1]`
- `Queue(None, false)` is a complete no-op; `Queue(None, true)` may promote an
  already queued mission. `[doc:
  MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md
  §§4.1, 12]`
- Empty Commence is a fieldwise no-op and false. Success promotes once,
  preserves suspended, and resets the exact common fields. `[doc:
  MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md
  §3]`
- Override uses Queue's two guards, saves queued when present or old current
  otherwise, preserves the queued slot and all handler/timing fields, and
  clears B8 only on mutation. `[doc:
  MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md
  §4]`
- Empty Restore is a fieldwise no-op and false. Success restores current,
  clears suspended, clears B8, and preserves queue/handler/timing. `[doc:
  MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md
  §5]`
- Building Override order is old Target archive, base Override, new Target.
  Foot order is old NavCom archive, old Target archive, base Override, new
  Target, new NavCom. `[doc:
  MISSION_ASSIGN_OVERRIDE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md]`
- Once a non-Aircraft concrete Override is entered, a base Mission guard does
  not suppress the later Target/NavCom setters. `[doc:
  MISSION_ASSIGN_OVERRIDE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md]`
- Restore runs base first and installs saved Target then saved NavCom only on
  success. Success does not explicitly clear either saved slot. `[doc:
  MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md
  §4.2]`
- Aircraft Assign, Queue, and Override share one pre-parent gate over the exact
  protected set. The classifier uses the full dword; a null manager does not
  turn none or an unknown value into a wildcard. `[doc:
  AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md]`
- A blocked Aircraft leaf call performs no archive, base transition, Target
  setter, or NavCom setter. Restore has no Aircraft filter. `[doc:
  AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md]`
- Aircraft Commence reads old current before promotion and clears its action
  latch for every old current except raw `0x1E`. Queue-owned promotion must
  dispatch through this hook. `[doc:
  AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md]`
- Unit Ready preserves the verified branch order across excluded missions,
  three raw bytes, queued Enter, concrete locomotor result, signed height,
  effective mission/Target/B8, two tracker bytes, Radio slot 0, and exact
  fallback geometry. `[doc:
  READYTOCOMMENCE_UNIT_INFANTRY_RESIDUAL_CLOSURE_GHIDRA_REPORT.md §§3-4]`
- Drive, Ship, Hover, Walk, Teleport, and Jumpjet use different locomotor
  predicates. One generic moving boolean is not mechanism-equivalent. `[doc:
  READYTOCOMMENCE_UNIT_INFANTRY_RESIDUAL_CLOSURE_GHIDRA_REPORT.md §2]`
- Infantry Ready reads current mission, firing latch, falling/settle latch,
  concrete locomotor state, Target exception, and raw Doing permission in
  order. Only Doing `-1` bypasses the table; there is no generalized clamp.
  `[doc:
  READYTOCOMMENCE_UNIT_INFANTRY_RESIDUAL_CLOSURE_GHIDRA_REPORT.md §§5-7]`
- Aircraft Ready uses current raw mission plus action and transition-ready
  latches. Building Ready uses its reusable animation/mission-ready latch.
  Neither is an altitude or permanent-construction proxy. `[doc:
  READYTOCOMMENCE_AIRCRAFT_BUILDING_WRITER_LIFECYCLES_GHIDRA_REPORT.md]`
- Building may set and consume its ready latch in one object update, with two
  consume points around Techno AI. A global tail pass is one tick late. `[doc:
  READYTOCOMMENCE_AIRCRAFT_BUILDING_WRITER_LIFECYCLES_GHIDRA_REPORT.md]`
- Unit readiness reads Radio contact slot 0 specifically and the independent
  `WeaponsFactory=yes` byte. Naval yards carry that byte; land-only
  `Factory=UnitType` inference is DRIFT. `[doc:
  READYTOCOMMENCE_UNIT_INFANTRY_RESIDUAL_CLOSURE_GHIDRA_REPORT.md §4; ini:
  rules.ini/rulesmd.ini WeaponsFactory=]`
- Verbs and readiness consume no RNG. Queue and concrete wrappers expose
  ordered same-call-stack effects, not deferred commands. `[doc:
  MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md;
  doc: MISSION_ASSIGN_OVERRIDE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md]`
- Native callers own promotion locally across object update, lifecycle,
  locomotor, mission, Radio, load, event/team, production, and per-cell paths.
  No generic end-of-tick promotion drain exists. `[doc:
  MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md
  §§6-8]`
- Native `+0xCC` has no active gameplay reader, is omitted from the native
  Mission checksum, and exposes stack-dependent residue through raw save.
  Gameplay must not guess a constant or use uninitialized Rust memory. `[doc:
  MISSION_RAW_BYTES_0XB8_0XCC_FULL_CENSUS_GHIDRA_REPORT.md]`
- Rust lockstep state must hash every new future-affecting gameplay field even
  when the native class checksum omits a field. A Rust hash is regression
  evidence, not gamemd parity certification. `[doc:
  MISSION_RAW_BYTES_0XB8_0XCC_FULL_CENSUS_GHIDRA_REPORT.md; AGENTS.md
  Parity certification and status]`

## Design

### Components

#### MissionId

`MissionId` is a transparent raw signed-dword value:

```text
MissionId(i32)
```

It owns the `-1` none sentinel and centralized conversion to and from the known
`MissionType` vocabulary. It must:

- preserve every input bit;
- compare protected Aircraft missions as full `i32` values;
- expose a checked known-mission query without normalizing unknown values;
- provide the known dispatch index only for values `0..=31`; and
- keep sentinel handling out of callsites.

The internal Mission state stores `MissionId`, not
`Option<MissionType>`. `MissionType` remains useful for typed known constants,
INI mission tables, and dispatch selection.

#### MissionCom

`MissionCom` becomes the single in-place common Mission state:

```text
MissionCom
|- current: MissionId
|- suspended: MissionId
|- queued: MissionId
|- movement_bypass_latch: u8
|- handler_state: u32
|- mission_start_frame: u32
|- ai_counter: u32
`- dispatch_timer
   |- start_frame: i32
   `- delay: i32
```

All fields are private. Read-only getters expose raw values and checked known
values. No general production setter exists.

`MissionCom::at_frame(frame)` is the only live constructor. It initializes:

- current, suspended, and queued to none;
- movement-bypass latch, handler state, mission-start frame, AI counter, and
  delay to zero; and
- dispatch start to the current binary frame, preserving the frame's raw low
  32 bits.

Generic `Default` must not provide gameplay construction semantics. Test
fixtures may use an explicitly named frame-zero constructor or builder.

Native `+0xCC` is deliberately absent from gameplay state. Assign and Commence
will implement every verified gameplay-visible write but cannot claim raw
native-save parity for that scratch word.

#### MissionDispatchTimer

Mission timing remains a dedicated primitive rather than changing unrelated
timers:

```text
if start == -1:
    due = true
else:
    elapsed = wrapping_i32(now_bits - start)
    due = elapsed >= delay using signed comparison
    if not due:
        remaining = wrapping_i32(delay - elapsed)
```

`now` may enter as the simulation's unsigned binary-frame storage, but it is
reinterpreted as the same 32 raw bits before native signed operations. No
saturation, floating point, wall clock, or decrement-per-tick model is allowed.

#### Mission leaf state

Category-specific raw leaf state is stored with the entity and initialized by
category:

- Unit: three independent readiness inputs and two independent tracker bytes,
  all initially zero.
- Infantry: firing-sequence latch and falling/settle latch initially zero, plus
  access to the raw Doing value owned by the Infantry sequence authority.
- Aircraft: action latch initially zero, transition-ready latch initially one,
  and runtime Airstrike-manager presence initially false while no manager
  exists.
- Building: reusable animation/mission-ready latch initially zero.

The representation may be an enum keyed by `EntityCategory` or an equivalently
sealed category bundle. It must prevent a caller from treating all categories
as one generic busy flag.

Raw fields remain private. Narrow writer operations represent verified writes.
If the matching Rust owner or same-tick placement does not yet exist, the
operation remains unused; no proxy field or animation state synthesizes it.

#### Suspended concrete state

`GameEntity` gains a suspended combat Target alongside the active Target.
`NavigationState::suspended_nav_com` remains the suspended NavCom owner.

The saved values are not moved into `MissionCom`: the common base transition
does not own Techno/Foot concrete fields. Successful Restore does not clear
either saved value.

#### MissionAuthority

A focused Simulation/entity-level adapter owns:

- receiver-family dispatch;
- Aircraft pre-parent gates;
- synchronous Queue/Ready/Commence flow;
- concrete Target/NavCom archive and setter order;
- construction of a current read-only readiness view; and
- missing-receiver transaction boundaries.

This adapter is Rust-native enum/context dispatch, not a C++ inheritance or
vtable replica. The private common transition helpers remain inside
`sim/mission/` and cannot be called directly by arbitrary production systems.

#### ReadyAuthorityView

`ReadyAuthorityView` borrows current authoritative inputs:

- Mission state and category-specific latches;
- active concrete locomotor state;
- signed entity height and position;
- active combat Target;
- Radio slot 0 identity;
- entity/type lookup for the contacted Building;
- occupancy/type lookup for the exact no-contact factory cell; and
- parsed rules data.

It is constructed after Queue's possible mutation, never cached on the entity,
and never used as a mutation command. It allocates nothing and consumes no RNG.

#### Legacy compatibility adapters

Temporary compatibility operations preserve current behavior for:

- player-command retasking with current dock teardown;
- current-only combat retasking;
- bunker install/release Mission writes;
- Unit-host `derived_mission` projection;
- tick-tail `derived_mission` projection; and
- any current reduced host/timer writes not yet mapped to a verified native
  owner.

Their names include `legacy` or `compatibility`, and their APIs expose only the
specific current operation. They cannot invoke exact Queue, Commence, Override,
Restore, readiness, Target/NavCom transactions, or Aircraft leaf logic.

Every compatibility callsite is a deferred migration item, not proof of native
authority.

#### Rules data

`ObjectType` gains an independent `weapons_factory` value parsed through the
existing base-INI then `*md` overlay path. Tests cover present, absent, and
naval-yard sections.

Runtime Airstrike-manager presence is not inferred from type, altitude,
paradrop state, owner building, stock-only constants, or warhead `Airstrike=`.
The future Airstrike manager system will own creation and association. Parsing
and implementing the full Airstrike team/recharge system is deferred with that
missing prerequisite; tests may construct explicit manager-present and
manager-absent leaf fixtures.

### Interfaces / Contracts

#### Private common transitions

Conceptual internal interfaces:

```text
assign_base(state, requested, now)
queue_base(state, requested) -> QueueContinuation
commence_base(state, now) -> bool
override_base(state, requested)
restore_base(state) -> bool
```

`QueueContinuation` is an internal orchestration result, not gameplay state. It
distinguishes:

- an outer guard, which skips Ready and Commence; from
- a valid continuation, which may or may not have written the queue.

This is necessary because a redundant or none request still reaches readiness
when `commence_now` is nonzero.

Assign and Override do not expose invented native success booleans. Tests judge
their fieldwise results. Concrete Override deliberately does not branch on a
base-mutation result.

#### Exact production-facing verb surface

The entity authority exposes the complete semantic packet:

```text
OverridePacket
|- mission: MissionId
|- combat_target: concrete nullable Target value
`- destination: concrete nullable NavCom value
```

The public exact operations are category-aware:

```text
assign(receiver, mission, now)
queue(receiver, mission, commence_now, now, read_context)
commence(receiver, now)
override(receiver, packet)
restore(receiver)
```

`commence_now` preserves native nonzero/zero semantics. It must not be reduced
to a separate deferred request.

These APIs exist for tests and future verified callers. This foundation adds no
new live production caller.

#### Concrete transaction contracts

Override:

```text
Mission/base:
    base Override

Building:
    save old Target
    base Override
    set new Target

Unit/Infantry:
    save old NavCom
    save old Target
    base Override
    set new Target
    set new NavCom using the reviewed native setter mode

Aircraft:
    apply leaf gate
    if blocked: no writes
    if allowed: run Unit/Infantry sequence
```

After a non-Aircraft wrapper begins, later setters run even if the common base
guard leaves Mission state unchanged.

Restore:

```text
base Restore
if false:
    stop; no Target/NavCom setter
if true:
    set saved Target
    if Foot-derived:
        set saved NavCom using the reviewed native setter mode
```

Saved Target and NavCom remain stored after successful Restore.

#### Aircraft leaf contracts

Assign, Queue, and Override share:

```text
protected = {4, 0x1A, 0x1B, 0x1E, 0x1F}

if current is protected
   and no runtime Airstrike manager exists
   and requested is not protected:
    block the whole leaf call
else:
    call the parent behavior with the original full-width values
```

Aircraft Commence:

```text
old_current = current
if old_current != 0x1E:
    clear aircraft action latch
return base Commence
```

The hook runs even when base Commence later finds no queued mission. Aircraft
Restore uses the inherited Foot transaction and has no leaf filter.

#### Verified host-write interfaces

Narrow operations cover established state-owner events:

- increment the per-mission AI counter with wrapping dword arithmetic;
- write dispatch start and delay at the handler epilogue;
- reset the exact Assign/Commence cluster through the verb only; and
- write or clear a category latch only at a verified owner.

No operation accepts an arbitrary field name/value pair.

#### Compatibility interfaces

Compatibility methods preserve current Rust behavior exactly, including the
current distinctions between full reduced retask, current-only combat retask,
host projection, and tail projection. They are intentionally not named after
native verbs unless the callsite crosswalk later proves that mapping.

Private fields provide the first writer barrier. A reviewed `rg` census over
compatibility methods provides the second: every production callsite must be
listed in the later authority-flip crosswalk.

### Data Flow

#### Construction

1. The gameplay spawn owner supplies the current binary frame to
   `GameEntity` construction.
2. `MissionCom::at_frame` creates exact common state.
3. Category construction creates the correct leaf-latch defaults.
4. The entity enters `EntityStore`; insertion does not reinitialize Mission
   state, so load/reinsert paths cannot overwrite saved values.

Tests pass an explicit frame, normally zero. A hidden frame-free fallback is
not permitted.

#### Current compatibility tick

1. Existing command/FSM logic computes its current result exactly as before.
2. It calls a specifically named compatibility operation.
3. The compatibility operation changes only the fields owned by that old path.
4. Hashing observes the resulting single Mission state.

There is no parallel Mission shadow. The state is final-format, but the
compatibility writer remains classified as unverified until replaced.

#### Exact Queue flow

When a future verified caller is activated:

1. Resolve the receiver; missing receiver produces no write.
2. Apply the Aircraft gate when applicable.
3. Run common Queue's outer guards and possible queue/B8 mutation.
4. If the outer guard blocked or `commence_now` is zero, return synchronously.
5. Release the mutable entity borrow.
6. Construct a fresh read-only readiness view from the just-written state and
   current world/rules inputs.
7. Evaluate the exact receiver leaf.
8. If ready, reacquire the receiver and invoke leaf Commence immediately.

No simulation phase or external system can interleave between these steps. The
temporary borrow release solves Rust aliasing without creating a stale cached
snapshot or delaying promotion.

#### Exact Override flow

1. Resolve receiver and classify family.
2. For Aircraft, evaluate the leaf gate before any archive or setter.
3. Archive concrete state in family-specific order.
4. Run base Override.
5. Run concrete Target/NavCom setters even if the base guard declined.
6. Return synchronously with no RNG or deferred command.

#### Exact Restore flow

1. Resolve receiver.
2. Run base Restore.
3. On false, return with all concrete state untouched.
4. On true, install saved Target and then saved NavCom for Foot-derived
   receivers.
5. Preserve both saved slots.

#### Readiness flow

Readiness selects the concrete category leaf and evaluates each branch in the
binary order. Unit contact logic reads slot 0, not contact membership. The
no-contact fallback checks the exact factory anchor relation, not whole
footprint occupancy. Concrete locomotor dispatch calls the matching family
query rather than a common movement proxy.

#### Persistence and hashing

One coordinated implementation-time snapshot-version increment covers:

- raw current, suspended, and queued IDs;
- B8, handler state, mission-start frame, AI counter, dispatch start, and
  delay;
- suspended Target and suspended NavCom;
- all readiness latches; and
- runtime Airstrike-manager presence.

Authoritative Mission and leaf fields do not use serde defaults to fabricate
old state. Older snapshot versions are rejected through the existing version
boundary.

The Rust world hash folds the same future-affecting values in a documented
fixed order. `+0xCC` is absent from both Rust gameplay state and the parity
claim.

### Error Handling

Common verb primitives are deterministic and infallible. None and unknown
Mission IDs are valid raw inputs and are preserved according to the native
branches rather than rejected, clamped, or normalized.

The Simulation-level adapter may report a missing receiver. That check happens
before the transaction, so no partial archive or mutation is possible.

Target and NavCom packet values are not rejected merely because the referenced
object cannot currently be resolved. Native reference expiration has its own
owner and ordering. A future live caller must integrate with that authority
rather than invent eager validation in the Mission layer.

If a live readiness view cannot prove the required current contact,
locomotor, type, or occupancy semantics, that caller remains inactive. It must
not substitute a convenient default. Synthetic tests construct complete views.

Snapshot version mismatch or malformed state follows existing snapshot error
propagation. No compatibility deserializer guesses a construction frame or
leaf-latch state.

### Testing Strategy

#### Common state and timer

- Frame-37 constructor fixture covering every initialized field.
- Independent-field preservation fixture with distinct sentinel values.
- Signed dispatch matrix over start `-1`, zero and negative delay, exact due
  boundary, high-bit values, and frame wrap.
- Effective-mission raw and known-value cases.

#### Verb matrices

- Exhaustive relevant Assign guards and fieldwise preserved/reset values.
- Queue outer guards and complete write predicate.
- Queue none/redundant cases with flag zero, Ready false, and Ready true.
- Commence empty fieldwise no-op and success reset matrix.
- Override queued-present/absent cases with distinct preserved-field sentinels.
- Restore empty/success matrices.
- Snapshot all simulation RNG logical states before and after each operation.

#### Ordered concrete transactions

- Base, Building, Unit, Infantry, and Aircraft Override traces.
- Base-guard-declined concrete setter continuation.
- Completely blocked Aircraft transaction with zero effects.
- Restore success Target-then-NavCom order.
- Restore failure with no concrete setters.
- Saved-slot persistence after success.
- Aircraft old-current Commence action-latch exception, including Queue-owned
  immediate promotion.

#### Readiness

- Unit branch-precedence tests for every excluded mission, raw byte, queued
  Enter bypass, height, effective mission/Target/B8 case, tracker byte, Radio
  slot-0 factory, Move/Enter exception, and exact no-contact geometry.
- Separate Drive, Ship, Hover, Walk, Teleport, and Jumpjet truth tables.
- Infantry firing, falling/settle, moving mission exceptions, null-target
  Attack, Doing `-1`, and represented Doing permission sweep.
- Aircraft current/action/transition truth table.
- Building raw-latch truth table.
- Zero mutation, zero allocation on the hot path where measurable, and zero RNG
  consumption.

#### Rules, persistence, and migration

- Base and `rulesmd` overlay parsing of `WeaponsFactory=`.
- Stock land-war-factory and naval-yard fixtures plus absent-key behavior.
- Per-field hash perturbation.
- Snapshot round-trip and old-version rejection.
- Deterministic replay equality.
- Existing command, bunker, Unit-host, and tail-projection regression tests.
- Reviewed production census of exact-authority and compatibility method calls.
- A guard test or validation script confirming no generic promotion drain.

Rust tests ratchet this implementation contract. They do not by themselves
certify gamemd parity. A verified parity claim still requires a named
gamemd-derived executable check or exhaustive proof over the relevant input
space.

### Stop condition

The foundation slice is complete only when:

- the final-format state, timer, verbs, concrete transactions, Aircraft leaf,
  readiness predicates, rule field, hash, and snapshot format exist;
- all direct production Mission field writes have moved behind reviewed exact,
  verified-host, or compatibility operations;
- current gameplay regression tests still pass;
- no new Queue/Commence/Override/Restore production caller is active;
- the pre-existing `techno_ai.rs` test work has been preserved; and
- status remains explicitly `PARTIAL/UNVERIFIED`, with both authority-flip
  crosswalks still open.

## Architectural Decisions

### Decisions followed

- **Rust-native structure, gamemd-native semantics:** enum/context dispatch
  replaces C++ inheritance while preserving leaf selection and call order.
- **Entity-owned state:** common and leaf state share entity lifetime; no
  sidecar cleanup authority is introduced.
- **Private authoritative fields:** behavior flows through semantic operations,
  not arbitrary writes.
- **Synchronous local authority:** Queue/Ready/Commence and concrete wrappers
  execute in one call stack.
- **Borrowed readiness view:** current world state is read without a stale
  duplicate cache.
- **One persistence migration:** final-format state is hashed and serialized
  now rather than migrating a shadow twice.
- **Explicit compatibility debt:** unproven current mappings remain visible and
  enumerable instead of being mislabeled as native verbs.

### Deliberate deviations and debt

- Current legacy projections remain temporarily active. This is not native
  parity; it is required because their true handler/owner mappings are not yet
  closed.
- Exact verbs remain mostly inactive in production. This is required because
  current player commands are not yet proven to map to those verbs.
- Native scratch `+0xCC` is omitted. Native raw-save compatibility remains
  blocked on captured runtime residue fixtures.
- Full Airstrike manager creation and `AirstrikeTeam*` configuration semantics
  remain with a future Airstrike-system investigation/implementation. The
  foundation stores only the runtime presence fact required by the verified
  Aircraft gate.
- Raw readiness writer operations may initially have no production caller where
  the matching owner or same-tick point does not yet exist. No approximation is
  substituted.

### Required follow-ups before authority flip

1. Produce and review the current player-action crosswalk for Move, Stop,
   Attack, ForceAttack, ForceAttackCell, AttackMove, RepairAtDepot,
   EnterTransport, PlantC4, CaptureBuilding, EnterBunker, and bunker
   install/release.
2. Produce and review the legacy-FSM owner crosswalk for miner, Aircraft,
   docking, attack, movement, order-intent, idle Guard, Unit-host projection,
   and tick-tail projection.
3. Replace compatibility callsites individually with their proven native owner
   operations.
4. Activate receiver-proven obstruction/damage Override, the three Restore
   families, and owner-local Queue/Commence points only at their verified
   same-call-stack positions.
5. Delete the compatibility layer only after the writer census reaches zero
   and same-tick clobber tests pass.

## Alternatives Considered

### Parallel exact shadow state

Add a second exact Mission state beside the current reduced component and
compare them until cutover.

Rejected because it creates two mutable truths, requires another hash/snapshot
migration, complicates same-tick comparison, and makes ownership less clear.
It reduces short-term mechanical changes but delays the foundational authority
benefit the user selected.

### Immediate production authority flip

Replace current commands and all `derived_mission` projections with exact
verbs and native caller adapters in one change.

Rejected because two load-bearing crosswalks are still missing. It could map
combat commands to the wrong verb, double-promote queued missions, activate
absent subsystem behavior, or let a legacy projection clobber a same-tick
transition. Those are parity failures, not acceptable implementation
trade-offs.

### Keep the reduced public state and patch verb functions only

Retain public fields, `Option<MissionType>`, unsigned timing, coarse readiness,
and current projections while correcting the obvious verb branches.

Rejected because it cannot preserve full-width values, native signed timing,
independent fields, concrete wrapper order, or one-owner write discipline. It
would encode known DRIFT behind superficially improved APIs.
