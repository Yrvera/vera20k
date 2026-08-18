# Mission Authority Foundation Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the reduced Mission shadow with one native-width, mechanism-exact
Mission foundation while preserving current gameplay behind named compatibility
operations and activating no new production Queue, Commence, Override, Restore, or
ReadyToCommence caller.

**Architecture:** `sim/mission/` owns raw Mission state, exact common transitions,
category leaf policy, and pure readiness predicates. A focused Simulation-level
adapter owns entity lookup and synchronous Target/NavCom effects; current command
and legacy-FSM behavior stays behind an enumerable compatibility boundary until
its two authority crosswalks are verified. Every new future-affecting field is
entity-owned, serialized, and folded into the deterministic hash in one coordinated
schema cutover.

**Design Doc:** `docs/plans/2026-07-23-mission-authority-foundation-design.md`

**Implementation Contract:**
`docs/contracts/2026-07-21-mission-state-verb-readiness-authority-implementation-contract.md`

**Execution status:** Bounded foundation implemented and validated in the
worktree; production exact authority remains dormant and Task 11's concrete
setters remain blocked.

**Post-review correction:** The generic Target/NavCom setter sketches in Task 11
are not implementation authority. Live binary review proved that base
`Set_Target @ 0x006FCDB0` clears a separate `+0x50C` byte even on same identity,
different-target paths and receiver-specific Target leaves have additional
effects, and mode-one destination installation is a multi-stage
passability/locomotor/radio/docking mechanism. The worktree therefore uses an
unavailable production provider and recording-only tests for wrapper order.
Do not activate or substitute the Task 11 sketches until those complete setters
and their missing state are implemented.

**Post-execution Queue correction:** `MissionClass::Queue` receives the full
dword packet forwarded by derived wrappers but tests only its low byte when
deciding whether to run Ready/Commence. Every `commence_now != 0` shorthand
below is therefore superseded by `(commence_now as u8) != 0`; `0x100` does not
self-promote, while `0x101` does.

---

## Grounding Summary

- The approved design selects an in-place exact substrate plus sealed legacy
  compatibility adapters; it explicitly rejects both a parallel shadow and an
  immediate production authority flip.
- Current `MissionCom` stores reduced `MissionType`/`Option<MissionType>` selectors,
  an eight-bit substate, a shared unsigned timer, and a refresh counter. Exact
  gamemd state instead uses three signed dword selectors, a raw B8 byte, three
  independent dword counters/state values, and a signed dispatch timer.
- Live Ghidra rechecks confirmed base Assign `0x005B2FD0`, Queue `0x005B35E0`,
  Commence `0x005B3570`, Override `0x005B3650`, and Restore `0x005B36B0`, including
  their guards, preservation sets, and synchronous Queue-to-Ready-to-Commence order.
- Live Ghidra rechecks also confirmed the Aircraft gate at `0x0041BB30`, Aircraft
  Commence hook at `0x0041B870`, and Foot/Techno concrete wrapper order at
  `0x004D8F40`, `0x004D8F80`, `0x007013A0`, and `0x007013E0`.
- Native `+0xCC` is an indeterminate stack-written raw-save residue, has no active
  gameplay reader, and is omitted from Mission checksum behavior. Rust gameplay
  state must omit it and must not describe it as preserved or initialized.
- Unit Ready `0x00744270` and Infantry Ready `0x00521B60` are fully researched.
  The Infantry Doing permission table has exactly 42 records; `-1` is the only
  bypass and `0..=41` is the authoritative indexed domain.
- The six active stock locomotor predicates are distinct: Drive, Ship, Hover,
  Walk, Teleport, and Jumpjet. No generic `is_moving` proxy is exact.
- `ObjectType` already follows merged base-then-`*md` parsing. It parses `Factory=`
  but not the independent `WeaponsFactory=` byte used by Unit readiness; stock
  naval yards prove the two properties cannot be inferred from each other.
- `EntityStore` and `OccupancyGrid` already preserve deterministic/native-style
  order. Readiness must read Radio slot 0 and walk the selected occupancy list in
  its stored order; it must not sort or allocate.
- Only four production areas currently write Mission state: retask, bunker link,
  Unit-host projection, and tick-tail projection. Existing exact-looking Queue,
  Commence, Override, Restore, and Ready helpers have zero live production callers.
- The current generic `MissionTimer` is also used by aircraft docking, building
  gates, and miners. It must remain unchanged; Mission dispatch gets a separate
  signed primitive.
- The three production `GameEntity::new` calls are in `world_spawn.rs`, where
  `session.binary_frame` is available. Frame-zero construction is acceptable only
  through explicitly test-named helpers.
- Snapshot version is 28 at planning time. The schema cutover may claim 29 only
  after an execution-time ownership check proves no competing version change.
- The only dirty tracked file is `src/sim/world/techno_ai.rs`, SHA-256
  `41370B1AC27A73D5F4964C459D0D0C9DEE9E475D3623F7B1484A88FB626BFDC6`,
  with `1497` insertions and `2` deletions. Its additions are test-only but directly
  read and write the reduced Mission fields, so execution has a hard ownership gate.
- The player-action-to-native-verb and legacy-FSM-to-native-owner crosswalks remain
  open. Therefore this slice ends `PARTIAL/UNVERIFIED` even when every planned test
  passes.

## Key Technical Decisions

- **Store raw Mission selectors as `MissionId(i32)` and retain `MissionType` only
  as the known vocabulary.** Unknown dword values and `-1` survive snapshot/hash
  without normalization. — **Confidence: high**
  - **Source:** `FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md`;
    `AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md`; live verb recheck.
- **Add `MissionDispatchTimer`; do not alter `MissionTimer`.** This prevents a
  Mission fix from changing docking, gates, or miner timing. — **Confidence: high**
  - **Source:** current Rust caller census; timer-family research; live Mission
    constructor and verb bodies.
- **Make Mission fields inaccessible outside `sim/mission/` and route all current
  behavior through named compatibility operations before the schema cutover.**
  This gives one writer barrier without activating unverified native mappings.
  — **Confidence: high**
  - **Source:** approved design and current four-area production writer census.
- **Keep exact authority caller count at zero.** Tests may call it; production
  commands, FSMs, and global tick code may not. — **Confidence: high**
  - **Source:** closed native caller reports and the two open crosswalks.
- **Represent suspended combat intent as `Option<TargetKind>`, not a clone of
  `AttackTarget`.** Native archives target identity; Rust cooldown/burst/fire
  progress belongs to the active attack runtime and is preserved when setting the
  same identity. — **Confidence: medium**
  - **Source:** Techno wrapper field evidence plus current `AttackTarget` ownership.
  - **Review requirement:** `/review-plan` must recheck the same-target setter rule
    before implementation starts.
- **Keep `NavigationState::suspended_nav_com` as the NavCom archive and use one
  mode-one Mission destination setter.** Concrete wrappers call it in verified
  order, but no live wrapper may be activated until every `NavTargetRef` coordinate
  provider is exact. — **Confidence: medium**
  - **Source:** Foot wrapper bodies and current `navcom.rs`.
  - **Review requirement:** `/review-plan` must verify the generic reference setter
    does not claim unimplemented locomotor side effects.
- **Store raw category latches with the entity, but keep Unit `+0x6D1` owned by
  the existing `Miner::unload_active` field.** Duplicating that byte would create
  two mutable truths. The Object falling byte is Object-wide, not Infantry-only.
  — **Confidence: high**
  - **Source:** readiness writer lifecycle reports and current miner state.
- **Represent locomotor-ready inputs with six explicit raw variants and raw IEEE
  bits for native double comparisons.** Missing producer-backed input yields
  `Unavailable`, never a guessed boolean. — **Confidence: medium**
  - **Source:** six `ILocomotion+0x80` bodies.
  - **Review requirement:** `/review-plan` must verify where the optional raw input
    lives and that no current movement proxy silently fills it.
- **Use a borrowed, freshly constructed readiness view after Queue mutation.**
  Queue releases its mutable entity borrow, reads current world state, then
  reacquires the entity only for conditional leaf Commence. — **Confidence: high**
  - **Source:** base Queue body and approved Rust ownership design.
- **Encode the full 42-entry Doing table and reject out-of-domain Rust state as
  unavailable/invariant failure.** There is no native clamp. — **Confidence: high**
  - **Source:** live `0x00523D00` decompile and `0x007EAF7C` 168-byte read.
- **Parse `WeaponsFactory=` independently through the normal merged INI path.**
  Runtime Airstrike-manager presence remains explicit false-by-default state and
  is never inferred from Boris keys, type, altitude, or a warhead property.
  — **Confidence: high**
  - **Source:** stock INI sections and Aircraft gate report.
- **Perform one coordinated state/hash/snapshot cutover and pre-read the snapshot
  version before deserializing the new body.** Old layouts must return
  `VersionMismatch`, not a misleading body decode error. — **Confidence: high**
  - **Source:** approved design and current `snapshot.rs` loader behavior.
- **Omit native `+0xCC` completely.** No initialized Rust surrogate, checksum fold,
  serializer field, or parity claim is permitted. — **Confidence: high**
  - **Source:** complete B8/CC census plus live Assign/Commence stack-write recheck.

## Open Questions

### Resolved During Planning

- **Can the shared `MissionTimer` be converted in place?** No. Aircraft docking,
  building gates, and six miner timers depend on its current unsigned/saturating
  behavior; Mission dispatch receives a distinct signed timer.
- **Is Selling an Assign guard?** No. Assign only blocks Deliberate(28) to Guard(5);
  Selling is allowed. Queue and Override separately block when current is Selling.
- **Does Queue readiness run after a redundant or None request?** Yes, when the
  low byte of native `char commence_now` is nonzero and the outer guard passes.
  Aircraft forwards the full dword, but a value such as `0x100` is false in the
  base Queue body.
- **Does Override consume queued Mission state?** No. It saves queued when present
  but preserves the queued slot and timing/state fields.
- **How large is the Infantry Doing table?** Exactly 42 records, indices `0..=41`;
  Doing `-1` bypasses it.
- **Can `WeaponsFactory` be inferred from `Factory=UnitType`?** No. `GAYARD`,
  `NAYARD`, and `YAYARD` carry `WeaponsFactory=yes`.
- **What construction frame is available in production?** All three production
  spawns can pass `Simulation.session.binary_frame`.
- **Should exact authority be wired into current commands now?** No. Both required
  crosswalks remain open and the accepted rollout explicitly preserves current
  gameplay through compatibility methods.

### Deferred Beyond This Foundation

- **Which current player action maps to Assign, Queue, Commence, Override, Restore,
  or a different native owner?** The player-action crosswalk must be researched and
  reviewed before replacing compatibility calls.
- **Which current miner, Aircraft, docking, attack, movement, and idle projections
  correspond to native handler/owner writes?** The legacy-FSM owner crosswalk must
  close before projection removal.
- **How should raw native saves reproduce `+0xCC` residue?** Runtime residue fixtures
  are required. Gameplay state remains safe and initialized by omitting the word.
- **What creates and owns an Aircraft Airstrike manager?** The full Boris
  Airstrike-team/recharge system is a separate investigation and implementation.
- **Where do every raw readiness latch and all six locomotor inputs get written in
  current Rust?** Only proven owner-local writers may activate them. Unavailable
  input keeps exact Ready dormant.
- **What are the complete current Rust equivalents of native Set_Target and
  Set_Destination for every pointer/reference kind?** The foundation preserves
  packet identity and wrapper order; live concrete callers wait for the remaining
  provider crosswalk.
- **How should non-stock Fly, Rocket, Mech, Tunnel, DropPod, and Parachute
  locomotors answer this Mission virtual?** Stock Unit/Infantry bindings do not use
  them. Modded live activation requires separate evidence.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/sim/mission/state.rs` | `MissionId`, final private `MissionCom`, raw getters, construction, and sealed state mutations |
| Create | `src/sim/mission/compatibility.rs` | Named adapters preserving current retask/projection behavior |
| Create | `src/sim/mission/leaf.rs` | Category-specific raw Mission latches and narrow writer/test fixture APIs |
| Create | `src/sim/mission/readiness.rs` | Borrowed views, exact Unit/Infantry/Aircraft/Building predicates, Doing table, signed cell conversion |
| Create | `src/sim/mission/authority.rs` | Category dispatch, Aircraft policy, synchronous Queue flow, concrete Override/Restore transactions |
| Create | `src/sim/movement/locomotor_ready.rs` | Six explicit `ILocomotion+0x80` predicate variants without simulation floats |
| Create | `tools/check_mission_authority_census.py` | Reproducible production writer/caller/compatibility allowlist check |
| Modify | `src/sim/mission/mod.rs` | Keep `MissionType`; export the focused Mission modules and raw ID |
| Modify | `src/sim/mission/timer.rs` | Add signed `MissionDispatchTimer`; leave `MissionTimer` unchanged |
| Modify | `src/sim/mission/verb.rs` | Replace reduced verbs/readiness snapshot with private exact common transitions |
| Modify | `src/sim/mission/retask.rs` | Route current command behavior only through compatibility operations |
| Modify | `src/sim/game_entity.rs` | Attach final Mission/leaf/archive/object-falling state and frame-aware construction |
| Modify | `src/sim/components.rs` | Keep NavCom archive owner and expose only the narrow archive/getter surface needed by Mission authority |
| Modify | `src/sim/movement/mod.rs` | Register/re-export crate-private locomotor readiness types |
| Modify | `src/sim/movement/locomotor.rs` | Carry optional producer-backed exact readiness state; initialize it unavailable |
| Modify | `src/sim/movement/navcom.rs` | Add the sealed mode-one Mission destination operation without command validation |
| Modify | `src/sim/movement/locomotor_tests.rs`, `src/sim/movement/teleport_movement.rs` | Initialize independent `WeaponsFactory` in direct `ObjectType` fixtures |
| Modify | `src/sim/combat/mod.rs` | Add concrete `TargetKind` installation preserving runtime on identical target identity |
| Modify | `src/rules/object_type.rs` | Parse and store independent `WeaponsFactory=` |
| Modify | `src/sim/docking/bunker_link.rs` | Replace live reduced Assign calls with named compatibility calls |
| Modify | `src/sim/world/techno_ai.rs` | Minimal production compatibility routing and mechanical test accessor migration after ownership gate |
| Modify | `src/sim/world/mod.rs` | Replace tail direct writes with the named compatibility projection |
| Modify | `src/sim/world/world_spawn.rs` | Pass `session.binary_frame` to the production constructor |
| Modify | `src/sim/world/world_hash.rs` | Fold every new raw selector, timer, latch, archive, and locomotor-ready input in fixed order |
| Modify | `src/sim/snapshot.rs` | Coordinated version bump, header-first rejection, and round-trip tests |
| Modify | `src/sim/world/mission_authoritative_tests.rs` | Use private-state fixtures and cover final state/hash behavior |
| Modify | `src/sim/world/slice6_retask_tests.rs` | Ratchet compatibility behavior after private-state migration |
| Modify | affected test modules listed in Appendix A | Rename direct frame-free construction and use read-only Mission accessors |
| Modify | affected locomotor fixture modules listed in Appendix B | Initialize the new optional exact locomotor-ready input explicitly |
| Modify | `docs/contracts/2026-07-21-mission-state-verb-readiness-authority-implementation-contract.md` | Record implemented foundation evidence without changing `PARTIAL/UNVERIFIED` authority status |

No module in this map may import `render`, `ui`, `sidebar`, `audio`, or `net`.
The new focused modules are expected to stay below roughly 600 lines each;
readiness and entity transaction logic remain split rather than growing
`verb.rs` or `world/mod.rs`.

## Interface Changes

### Raw Mission vocabulary

```rust
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
    serde::Serialize, serde::Deserialize,
)]
pub struct MissionId(i32);

impl MissionId {
    pub const NONE: Self = Self(-1);

    pub const fn from_raw(raw: i32) -> Self;
    pub const fn raw(self) -> i32;
    pub const fn from_known(known: MissionType) -> Self;
    pub fn known(self) -> Option<MissionType>;
    pub fn dispatch_index(self) -> Option<usize>;
}
```

`MissionType` remains the known 32-entry enum. No `From<MissionId> for
MissionType` conversion is provided because it would have to normalize unknown
values.

### Final Mission state

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MissionCom {
    current: MissionId,
    suspended: MissionId,
    queued: MissionId,
    movement_bypass_latch: u8,
    handler_state: u32,
    mission_start_frame: u32,
    ai_counter: u32,
    dispatch_timer: MissionDispatchTimer,
}
```

Production code receives read-only getters. Mutation is limited to private common
transitions, narrow verified-host methods, or the compatibility module.
`MissionCom` has no gameplay `Default` or frame-free constructor.

### Compatibility surface

```rust
pub(crate) fn legacy_full_retask(
    state: &mut MissionCom,
    requested: MissionType,
    now: u32,
);
pub(crate) fn legacy_current_only_retask(
    state: &mut MissionCom,
    requested: MissionType,
);
pub(crate) fn legacy_unit_host_projection(
    state: &mut MissionCom,
    current: MissionType,
    substate: u8,
);
pub(crate) fn legacy_tick_tail_projection(
    state: &mut MissionCom,
    current: MissionType,
    substate: u8,
);
```

These functions preserve current Rust results; they do not invoke an exact native
verb or readiness function.

### Exact authority surface

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverridePacket {
    pub mission: MissionId,
    pub combat_target: Option<TargetKind>,
    pub destination: Option<NavTargetRef>,
}

impl Simulation {
    pub(crate) fn mission_assign_exact(
        &mut self,
        receiver: u64,
        requested: MissionId,
        now: u32,
    ) -> Result<(), MissionAuthorityError>;

    pub(crate) fn mission_queue_exact(
        &mut self,
        receiver: u64,
        requested: MissionId,
        commence_now: i32,
        now: u32,
        readiness: &impl ReadyInputProvider,
    ) -> Result<(), MissionAuthorityError>;

    pub(crate) fn mission_commence_exact(
        &mut self,
        receiver: u64,
        now: u32,
    ) -> Result<bool, MissionAuthorityError>;

    pub(crate) fn mission_override_exact(
        &mut self,
        receiver: u64,
        packet: OverridePacket,
    ) -> Result<(), MissionAuthorityError>;

    pub(crate) fn mission_restore_exact(
        &mut self,
        receiver: u64,
    ) -> Result<bool, MissionAuthorityError>;
}
```

These methods remain crate-private and have zero production callers at the stop
condition. Missing receiver is detected before any write. Missing exact readiness
input is reported explicitly and cannot be converted to `true` or `false`.

### Construction

```rust
pub fn new_at_frame(
    /* existing constructor arguments */,
    construction_frame: u32,
) -> Self;

#[cfg(test)]
pub(crate) fn new_at_frame_zero_for_test(
    /* existing constructor arguments */
) -> Self;
```

The current frame-free `GameEntity::new` is removed. `test_default` remains
available but calls the explicitly frame-zero helper internally.

### Rules schema

```rust
pub struct ObjectType {
    // existing fields
    pub factory: Option<FactoryType>,
    pub weapons_factory: bool,
    // existing fields
}
```

`weapons_factory` defaults to false when absent and is independent of `factory`
and `naval`.

## Sim Checklist

- [ ] No gameplay decision introduces `f32` or `f64`; native double predicates
  operate on raw `u64` IEEE bits.
- [ ] Every new future-affecting field is serialized and folded into the state
  hash in an explicit fixed order.
- [ ] No `sim/` module depends on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- [ ] Tick ordering is unchanged because no exact authority caller becomes live;
  compatibility operations stay at the current host/tail callsites.
- [ ] Queue, Ready, and Commence execute synchronously in one call stack in tests
  and future verified callers; there is no global promotion drain.
- [ ] `EntityStore` iteration remains stable-ID ordered where used; Unit factory
  fallback preserves `OccupancyGrid` list order and performs no sorting.
- [ ] Readiness and verb operations allocate nothing and consume no RNG.
- [ ] Snapshot version ownership and deterministic-golden ownership are checked
  before the schema cutover.
- [ ] `MissionTimer` behavior and all unrelated timer consumers remain unchanged.
- [ ] Exact authority production caller count is zero at completion.

## Risk Areas

| Risk | Failure mode | Required defense |
|---|---|---|
| Dirty `techno_ai.rs` collision | Private-field migration overwrites another session's test work | Verify file hash and diff shape; wait for the owner or make only reviewed mechanical accessor edits |
| Raw ID truncation | Unknown missions or `-1` become a known byte/None | Store/hash/serialize `i32`; use checked known conversion only |
| Timer-family contamination | Miner/dock/gate cadence changes | Add a separate type; assert no existing `MissionTimer` callsite changes |
| Compatibility drift | Current commands change before crosswalk closure | Characterization tests before and after rerouting; compatibility never calls exact authority |
| Stale readiness | Queue writes a new slot but Ready reads old data | Drop mutable borrow, build view after mutation, then conditionally reacquire for Commence |
| Aircraft partial write | Gate blocks Mission but archives Target/NavCom | Run the gate before every archive or base call |
| Concrete setter order | Pursuit/destination intent differs | Trace archive/base/set order, including the base-guard-declined path |
| Duplicate Unit `+0x6D1` | Miner and Mission leaf disagree | Read existing `Miner::unload_active`; do not add a second field |
| Object falling misownership | Infantry-only field misses DropIn/Unlimbo semantics | Store the raw byte at `GameEntity` Object scope |
| Locomotor approximation | Moving proxy changes readiness | Six explicit raw variants; absent producer state is unavailable |
| Doing out-of-range access | Rust clamps or silently allows invalid state | `-1` bypass; `0..=41` table; every other value returns invariant-unavailable |
| Factory misclassification | Naval-yard exit behaves as non-factory | Parse `WeaponsFactory=` independently and test yard sections |
| Snapshot body-first load | Old save reports decode corruption instead of version mismatch | Decode/validate `GameSnapshotHeader` before full body |
| Hash omission | Lockstep misses future divergence | Per-field perturbation test for state, leaf, archives, and locomotor input |
| Accidental activation | Exact API changes live behavior | Static census with explicit zero-caller rule and no global drain |
| False parity claim | Rust-only tests are treated as gamemd certification | Keep status `PARTIAL/UNVERIFIED`; cite binary evidence separately |

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Signed raw Mission IDs and `-1` | Full dwords affect guards, dispatch, save, and hash | Raw/known round-trip tests plus constructor/verb Ghidra anchors |
| 1 | Signed dispatch arithmetic | High-bit frames, negative delay, and `-1` differ from unsigned/saturating math | Boundary/wrap matrix against the documented native formula |
| 2 | Compatibility behavior | Prevents an unverified early authority flip | Before/after command, bunker, host, and tail regression tests |
| 3 | Independent `WeaponsFactory` | Unit Ready covers naval yards as well as land war factories | Base/md stock INI fixtures |
| 4 | Raw leaf defaults and ownership | Wrong latch scope/default changes immediate readiness | Constructor/category truth table and writer-lifecycle reports |
| 5 | Assign fields and guard | Selling must pass; suspended must survive; exact reset cluster matters | Exhaustive field sentinel matrix against `0x005B2FD0` |
| 5 | Queue predicate and continuation | None/redundant nonzero calls still run Ready/Commence | Complete Queue matrix against `0x005B35E0` |
| 5 | Commence/Override/Restore preservation | Queue, B8, handler, and timing have different ownership per verb | Fieldwise no-op/success matrices |
| 6 | Construction frame and state schema | A frame-zero live constructor shifts dispatch timing | Frame-37 constructor and production spawn tests |
| 6 | Snapshot/hash order | Omission or normalization causes undetected desync | Per-field hash perturbation, round-trip, and old-version rejection |
| 8 | Six locomotor mechanisms | One generic moving flag is known DRIFT | Per-family truth tables at six verified function anchors |
| 9 | Unit branch order | Earlier gates suppress later world queries; exceptions are mission-specific | Branch-precedence tests and query-count spies |
| 9 | Signed cell conversion | Negative coordinates and `+1` Y anchor are exact | Concrete signed fixtures using native shift formula |
| 9 | Infantry Doing table | One entry changes animation-to-mission transition timing | Full 42-entry sweep plus `-1` and invalid-domain fixtures |
| 10 | Aircraft protected set | Full dword classification and manager presence gate all leaf effects | Protected/unknown/None/manager matrix |
| 10 | Aircraft Commence hook | Action latch clears even with an empty queue except old raw `0x1E` | Direct and Queue-owned promotion tests |
| 10 | Same-stack Queue flow | Deferred promotion is one tick late and changes ordering | Ordered trace Ready then conditional leaf Commence |
| 11 | Target/NavCom transaction order | Archive and setter ordering preserves interrupted intent | Exact Building/Foot/Aircraft trace tests |
| 11 | Base-guard-declined continuation | Concrete setters still run after a non-Aircraft wrapper begins | Deliberate-to-Guard wrapper test |
| 11 | Restore success gate | Failed Restore must not resurrect saved references | Success/failure and saved-slot-persistence tests |
| 12 | Two B8 producer sequences | Refinery completion writes B8 after Queue; Jumpjet performs a second Ready/Commence after its B8 write | Inert ordered-call tests against both verified producer bodies |
| 12 | Building latch consumption | Set and successful consume may occur twice around Techno AI in one object update | Inert consume-helper tests; no global-phase activation |
| 13 | Zero live exact callers | Unverified commands/FSMs must not change behavior | Static census and reviewed `rg` output |

---

## Tasks

### Task 0: Establish the Execution Ownership Gate

**Why:** The final schema necessarily touches a dirty test file and the global
snapshot/hash boundaries; execution is unsafe until those ownership facts are
stable.

**Files:** Read-only preflight over the repository.

**Pattern:** Existing project Cargo/snapshot coordination rules.

**Step 1: Verify repository baseline**

Run:

```powershell
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status --short
git diff --numstat -- src/sim/world/techno_ai.rs
Get-FileHash src/sim/world/techno_ai.rs -Algorithm SHA256
rg -n "const SNAPSHOT_VERSION: u32 = " src/sim/snapshot.rs
rg -n "GameEntity::new\(" src --glob "*.rs"
```

Expected planning baseline:

```text
branch: dev
HEAD: 928644d44fee61d0bb8d3214a4f9e0eb7390bc4e
dirty tracked paths: src/sim/world/techno_ai.rs only
techno_ai diff: 1497 insertions, 2 deletions
techno_ai SHA-256: 41370B1AC27A73D5F4964C459D0D0C9DEE9E475D3623F7B1484A88FB626BFDC6
snapshot version: 28
direct GameEntity::new callsites: 76
```

The exact HEAD may advance before execution. If it does, re-read every Mission,
GameEntity, hash, snapshot, rules, combat-target, and NavCom file named in this
plan and update the baseline. Do not force the old SHA or version onto newer work.

**Step 2: Resolve `techno_ai.rs` ownership**

Proceed only under one of these conditions:

1. its owner has landed the work and the file is clean; or
2. its owner explicitly permits the minimal constructor/getter/test-fixture
   migration and the pre-edit diff is saved for comparison.

Do not stage, discard, restore, or rewrite the existing test additions. Before and
after the required mechanical edits, compare:

```powershell
git diff -- src/sim/world/techno_ai.rs
```

The only accepted changes from this plan are import adjustments, exact constructor
renames, Mission getters, test fixture methods, and the three production
compatibility calls.

**Step 3: Check build and rebaseline ownership**

Run:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
rg -n "SNAPSHOT_VERSION|golden|SNAPSHOT" src/sim/snapshot.rs src/sim/world/world_hash.rs
```

If another session owns Cargo, snapshot versioning, or golden updates, wait. Do
not stop its processes and do not select another snapshot number independently.

**Step 4: Record the stop condition**

Execution may continue when the dirty-file owner, snapshot version, hash/golden
owner, and Cargo owner are known. This task makes no files changes and runs no
build.

---

### Task 1: Add Lossless Mission IDs and a Dedicated Dispatch Timer

**Why:** Every later transition depends on full-width selectors and exact signed
timing; adding them first is behavior-neutral.

**Files:**

- Create: `src/sim/mission/state.rs`
- Modify: `src/sim/mission/mod.rs`
- Modify: `src/sim/mission/timer.rs`

**Pattern:** Small serialized value types beside the existing `MissionType`; new
timer family adjacent to, not replacing, `MissionTimer`.

**Step 1: Define `MissionId`**

Implement the final shape:

```rust
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
    serde::Serialize, serde::Deserialize,
)]
pub struct MissionId(i32);

impl MissionId {
    pub const NONE: Self = Self(-1);

    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> i32 {
        self.0
    }

    #[inline]
    pub const fn from_known(known: MissionType) -> Self {
        if matches!(known, MissionType::None) {
            Self::NONE
        } else {
            Self(known as i32)
        }
    }

    pub fn known(self) -> Option<MissionType> {
        let raw = self.0;
        if !(0..MISSION_COUNT as i32).contains(&raw) {
            return None;
        }
        MissionType::from_id(raw as u8)
    }

    pub fn dispatch_index(self) -> Option<usize> {
        let raw = self.0;
        (0..MISSION_COUNT as i32)
            .contains(&raw)
            .then_some(raw as usize)
    }
}
```

Re-export `MissionId` from `mission/mod.rs`. Do not add a lossy conversion that
turns unknown values into `MissionType::None`.

**Step 2: Add `MissionDispatchTimer` without editing `MissionTimer` semantics**

```rust
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash,
    serde::Serialize, serde::Deserialize,
)]
pub struct MissionDispatchTimer {
    start_frame: i32,
    delay: i32,
}

impl MissionDispatchTimer {
    pub const fn at_frame(frame: u32) -> Self {
        Self {
            start_frame: frame as i32,
            delay: 0,
        }
    }

    pub const fn from_raw(start_frame: i32, delay: i32) -> Self {
        Self { start_frame, delay }
    }

    pub const fn start_frame(self) -> i32 {
        self.start_frame
    }

    pub const fn delay(self) -> i32 {
        self.delay
    }

    pub fn due(self, now: u32) -> bool {
        self.start_frame == -1
            || (now as i32).wrapping_sub(self.start_frame) >= self.delay
    }

    pub fn remaining_if_pending(self, now: u32) -> Option<i32> {
        if self.due(now) {
            return None;
        }
        let elapsed = (now as i32).wrapping_sub(self.start_frame);
        Some(self.delay.wrapping_sub(elapsed))
    }
}
```

Keep raw write/reset helpers private to `sim/mission/`; they will be added with
`MissionCom`. Do not change `MissionTimer::due`, `remaining`, `defer`, or any
existing caller.

**Step 3: Add unit tests**

Cover:

```rust
#[test]
fn unknown_mission_id_round_trips_without_normalization() {
    let id = MissionId::from_raw(0x1234_5678);
    assert_eq!(id.raw(), 0x1234_5678);
    assert_eq!(id.known(), None);
    assert_eq!(id.dispatch_index(), None);
}

#[test]
fn none_is_signed_minus_one_not_enum_ff() {
    assert_eq!(MissionId::NONE.raw(), -1);
    assert_eq!(MissionId::from_known(MissionType::None), MissionId::NONE);
}

#[test]
fn signed_dispatch_matrix_matches_native_formula() {
    assert!(MissionDispatchTimer::from_raw(-1, 99).due(0));
    assert!(MissionDispatchTimer::from_raw(10, -1).due(10));
    assert!(!MissionDispatchTimer::from_raw(10, 5).due(14));
    assert!(MissionDispatchTimer::from_raw(10, 5).due(15));
    assert!(!MissionDispatchTimer::from_raw(i32::MAX - 1, 4)
        .due((i32::MIN + 1) as u32));
    assert!(MissionDispatchTimer::from_raw(i32::MAX - 1, 3)
        .due((i32::MIN + 1) as u32));
}
```

Also test every known ID `0..=31`, high-bit `now`, exact boundary, negative delay,
and `remaining_if_pending` wrapping behavior.

**Step 4: Verify**

Before Cargo, repeat the process check from Task 0. Then run serially:

```powershell
cargo test -p vera20k mission_id -- --nocapture
cargo test -p vera20k signed_dispatch -- --nocapture
```

Expected literal result for each command: `test result: ok`.

---

### Task 2: Put Current Gameplay Behind Named Compatibility Operations

**Why:** Exact state cannot become safe authority while current commands and
projections directly mutate its fields.

**Files:**

- Create: `src/sim/mission/compatibility.rs`
- Modify: `src/sim/mission/mod.rs`
- Modify: `src/sim/mission/retask.rs`
- Modify: `src/sim/docking/bunker_link.rs`
- Modify: `src/sim/world/techno_ai.rs`
- Modify: `src/sim/world/mod.rs`
- Modify: current retask/bunker/host/tail tests

**Pattern:** Semantic compatibility boundary, analogous to other migration
adapters; no generic field setter.

**Step 1: Characterize current behavior before rerouting**

Add or retain assertions proving:

- full reduced retask writes current, clears queued and suspended, zeroes substate,
  and resets the current timer at `now`, but preserves the refresh counter;
- current-only retask changes only current;
- Unit-host projection increments the refresh counter, writes current, and writes
  substate;
- tick-tail projection does the same;
- bunker install/release retain their current Guard/Move outcomes.

Run the focused current tests and record their literal `test result:` lines before
changing callsites.

**Step 2: Add four explicit compatibility functions**

Initially implement them over the reduced storage; Task 6 rewrites only their
internals for the final schema:

```rust
pub(crate) fn legacy_full_retask(
    state: &mut MissionCom,
    requested: MissionType,
    now: u32,
) {
    state.current = requested;
    state.queued = None;
    state.suspended = None;
    state.substate = 0;
    state.timer.reset(now);
}

pub(crate) fn legacy_current_only_retask(
    state: &mut MissionCom,
    requested: MissionType,
) {
    state.current = requested;
}

pub(crate) fn legacy_unit_host_projection(
    state: &mut MissionCom,
    current: MissionType,
    substate: u8,
) {
    state.tick_counter = state.tick_counter.wrapping_add(1);
    state.current = current;
    state.substate = substate;
}

pub(crate) fn legacy_tick_tail_projection(
    state: &mut MissionCom,
    current: MissionType,
    substate: u8,
) {
    state.tick_counter = state.tick_counter.wrapping_add(1);
    state.current = current;
    state.substate = substate;
}
```

The deliberate duplication of the two projection entry points makes their later
owner crosswalk enumerable. Neither may delegate to exact Assign, Queue,
Commence, Override, Restore, Ready, Target, NavCom, or Aircraft policy.

**Step 3: Reroute every live writer**

- `retask.rs::assign_mission_with_teardown` calls `legacy_full_retask`.
- `retask.rs::assign_mission_keep_fields` calls
  `legacy_current_only_retask`.
- Both bunker writes call `legacy_full_retask`.
- `techno_ai.rs` production host write calls `legacy_unit_host_projection`.
- `world/mod.rs` tail refresh calls `legacy_tick_tail_projection`.

Remove the live call to the old `verb::assign_mission`. Do not change teardown,
host position, iteration order, or tick timing.

**Step 4: Verify compatibility and caller boundary**

Run serially:

```powershell
cargo test -p vera20k slice6_retask -- --nocapture
cargo test -p vera20k bunker_link -- --nocapture
cargo test -p vera20k techno_ai -- --nocapture
cargo test -p vera20k mission_authoritative -- --nocapture
rg -n "assign_mission\(" src --glob "*.rs"
rg -n "legacy_(full_retask|current_only_retask|unit_host_projection|tick_tail_projection)" src --glob "*.rs"
```

Expected: the same focused behavior passes; production compatibility callsites
match the four reviewed areas; no production Queue/Commence/Override/Restore/Ready
caller exists.

---

### Task 3: Parse Independent `WeaponsFactory=` Rules State

**Why:** Unit readiness reads a native type byte that is independent from the
existing production factory classification.

**Files:**

- Modify: `src/rules/object_type.rs`
- Modify: `src/sim/movement/locomotor_tests.rs`
- Modify: `src/sim/movement/teleport_movement.rs` test fixture

**Pattern:** Existing `IniSection::get_bool` fields in `ObjectType::from_ini`.

**Step 1: Add the field next to `factory`**

```rust
/// Native BuildingType `WeaponsFactory=` classification used by Unit
/// ReadyToCommence. Independent from `Factory=` and `Naval=`.
pub weapons_factory: bool,
```

**Step 2: Parse through the existing merged section**

```rust
factory: section.get("Factory").and_then(FactoryType::from_ini),
weapons_factory: section.get_bool("WeaponsFactory").unwrap_or(false),
cloning: section.get_bool("Cloning").unwrap_or(false),
```

Do not infer it from type ID, `FactoryType`, `Naval`, exit coordinates, or
foundation.

Update the two direct `ObjectType { ... }` test fixtures found by:

```powershell
rg -n "ObjectType \{" src --glob "*.rs"
```

Set `weapons_factory: false` explicitly. Parser-created fixtures need no
mechanical change.

**Step 3: Add parser and stock fixtures**

Tests must cover:

```text
missing key -> false
WeaponsFactory=no -> false
WeaponsFactory=yes -> true
Factory=UnitType without WeaponsFactory -> false
WeaponsFactory=yes without Factory=UnitType -> true
```

Load merged stock rules and assert:

```text
GAWEAP, NAWEAP, GAYARD, NAYARD -> true
YAWEAP, YAYARD -> true after rulesmd overlay
GAPILE and a non-factory building -> false
```

This explicitly tests naval yards rather than only land factories.

**Step 4: Verify**

```powershell
cargo test -p vera20k weapons_factory -- --nocapture
```

Expected literal result: `test result: ok`.

---

### Task 4: Define Category Leaf State and Raw Locomotor-Readiness Inputs

**Why:** Exact readiness and Aircraft policy require category-specific bytes;
representing them now avoids a generic busy flag.

**Files:**

- Create: `src/sim/mission/leaf.rs`
- Create: `src/sim/movement/locomotor_ready.rs`
- Modify: `src/sim/mission/mod.rs`
- Modify: `src/sim/movement/mod.rs`

**Pattern:** Entity-owned serialized state with private raw fields and narrow
category operations. This is a new focused pattern; it must not become a general
byte bag.

**Step 1: Define sealed category state**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum MissionLeafState {
    Unit(UnitMissionLeaf),
    Infantry(InfantryMissionLeaf),
    Aircraft(AircraftMissionLeaf),
    Building(BuildingMissionLeaf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct UnitMissionLeaf {
    deploy_begin_active: u8,
    deploy_reverse_active: u8,
    tracker_byte_18: u8,
    tracker_byte_19: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct InfantryMissionLeaf {
    firing_sequence_latch: u8,
    doing: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct AircraftMissionLeaf {
    action_latch: u8,
    transition_ready_latch: u8,
    airstrike_manager_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct BuildingMissionLeaf {
    ready_latch: u8,
}
```

Category defaults are:

```text
Unit: all four bytes 0
Infantry: firing 0, Doing -1
Aircraft: action 0, transition-ready 1, manager-present false
Building: ready 0
```

The Unit unload-active input is not duplicated here. Readiness reads
`entity.miner.as_ref().is_some_and(|miner| miner.unload_active)`.
The Object falling byte is attached at Object scope in Task 6.

Add read-only views and narrowly named verified writer methods. Methods whose
current Rust owner is not exact remain unused in production. Under `#[cfg(test)]`,
provide explicit raw fixture constructors rather than making fields public.

**Step 2: Define six raw locomotor variants**

```rust
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash,
    serde::Serialize, serde::Deserialize,
)]
pub(crate) enum LocomotorReadyState {
    Drive {
        turning_active: bool,
        slot_moving: bool,
        head_to_nonnull: bool,
        owner_speed: i32,
    },
    Ship {
        turning_active: bool,
        slot_moving: bool,
        head_to_nonnull: bool,
        owner_speed: i32,
    },
    Hover {
        slot_moving: bool,
        speed_bits: u64,
    },
    Walk {
        moving_byte: u8,
        applied_speed_bits: u64,
        destination_nonnull: bool,
    },
    Teleport {
        state: u8,
    },
    Jumpjet {
        state: i32,
    },
}
```

Task 4 defines and tests the raw type only. Task 6 attaches
`mission_ready_state: Option<LocomotorReadyState>` to `LocomotorState` inside the
coordinated schema cutover. This keeps Task 4 independently compilable and avoids
forcing every test struct literal through an intermediate layout.

**Step 3: Add construction tests**

Assert the four category defaults, no cross-category accessor, and lossless
serde/hash round trips for every raw locomotor variant. The constructor-default
test belongs to Task 6 after the field is attached.

**Step 4: Verify**

```powershell
cargo test -p vera20k mission_leaf -- --nocapture
cargo test -p vera20k locomotor_ready_state -- --nocapture
```

Expected literal result for each: `test result: ok`.

---

### Task 5: Replace Reduced Verbs with Exact Private Common Transitions

**Why:** The five common transitions are the central mechanism contract; keeping
them pure and private permits exhaustive verification before entity wiring.

**Files:**

- Modify: `src/sim/mission/verb.rs`
- Modify: `src/sim/mission/state.rs`
- Modify: `src/sim/mission/mod.rs`

**Pattern:** Pure deterministic functions over private state; no clock lookup,
allocation, RNG, world access, or invented status value.

**Step 1: Define internal continuation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QueueContinuation {
    OuterGuardBlocked,
    Continue,
}
```

The enum distinguishes the whole-function guard from allowed execution. It does
not say whether the queue write predicate fired.

**Step 2: Implement exact base transitions**

Use raw IDs:

```rust
const ATTACK: MissionId = MissionId::from_raw(1);
const GUARD: MissionId = MissionId::from_raw(5);
const SELLING: MissionId = MissionId::from_raw(19);
const DELIBERATE: MissionId = MissionId::from_raw(28);

pub(super) fn assign_base(
    state: &mut MissionCom,
    requested: MissionId,
    now: u32,
) {
    if state.current() == DELIBERATE && requested == GUARD {
        return;
    }
    state.assign_transition(requested, now);
}

pub(super) fn queue_base(
    state: &mut MissionCom,
    requested: MissionId,
) -> QueueContinuation {
    let current = state.current();
    if (current == DELIBERATE && requested == GUARD) || current == SELLING {
        return QueueContinuation::OuterGuardBlocked;
    }
    if requested != MissionId::NONE
        && !(current == requested
            && (state.queued() == requested || state.queued() == MissionId::NONE))
    {
        state.write_queue_and_clear_b8(requested);
    }
    QueueContinuation::Continue
}

pub(super) fn commence_base(state: &mut MissionCom, now: u32) -> bool {
    if state.queued() == MissionId::NONE {
        return false;
    }
    state.promote_queue(now);
    true
}

pub(super) fn override_base(state: &mut MissionCom, requested: MissionId) {
    let current = state.current();
    if (current == DELIBERATE && requested == GUARD) || current == SELLING {
        return;
    }
    state.override_transition(requested);
}

pub(super) fn restore_base(state: &mut MissionCom) -> bool {
    if state.suspended() == MissionId::NONE {
        return false;
    }
    state.restore_transition();
    true
}
```

The sealed state mutations must implement exactly:

- Assign: current=requested; queued=None; B8=0; handler state=0;
  mission-start=now; AI counter=0; dispatch start=now; delay=0; suspended
  preserved.
- Commence success: current=queued; queued=None; B8=0; handler state=0;
  mission-start=now; AI counter=0; dispatch start=now; delay=0; suspended
  preserved.
- Override allowed: suspended=queued when queued exists, otherwise old current;
  current=requested; queued preserved; B8=0; handler/timing/start/AI preserved.
- Restore success: current=suspended; suspended=None; B8=0; queued and every
  handler/timing/start/AI field preserved.
- Empty Commence/Restore and guarded transitions are fieldwise no-ops.
- `+0xCC` has no representation or write.

Assign and Override return `()`. Do not invent success booleans.

**Step 3: Add exhaustive field-sentinel tests**

Build a fixture in which every field has a distinct nonzero value. Test:

- all Assign guard combinations, including Selling allowed and suspended
  preserved;
- complete Queue write predicate over requested None, current equal/unequal, queue
  None/equal/different, flag-independent base behavior, and both outer guards;
- Commence empty byte-for-byte equality and success reset/preservation;
- Override queue-present and queue-absent source selection, guards, and all
  preserved fields;
- Restore empty byte-for-byte equality and success preservation;
- unknown raw IDs are compared/preserved as raw dwords.

Use one helper that compares every `MissionCom` field explicitly; do not rely only
on a final equality assertion when the test is documenting a preservation set.

**Step 4: Verify**

```powershell
cargo test -p vera20k mission_assign_base -- --nocapture
cargo test -p vera20k mission_queue_base -- --nocapture
cargo test -p vera20k mission_commence_base -- --nocapture
cargo test -p vera20k mission_override_base -- --nocapture
cargo test -p vera20k mission_restore_base -- --nocapture
```

Expected literal result for every command: `test result: ok`.

---

### Task 6: Perform the Atomic Final State, Hash, and Snapshot Cutover

**Why:** Private raw state, entity construction, hash order, and serialized layout
are one compatibility boundary. Splitting the persistence migration would leave
an intermediate build that accepts or hashes the wrong schema.

**Files:**

- Modify: `src/sim/mission/state.rs`
- Modify: `src/sim/mission/compatibility.rs`
- Modify: `src/sim/game_entity.rs`
- Modify: `src/sim/world/world_spawn.rs`
- Modify: `src/sim/world/world_hash.rs`
- Modify: `src/sim/snapshot.rs`
- Modify: `src/sim/movement/locomotor.rs`

**Pattern:** Explicit entity-owned state and fixed-order manual hash folds already
used by `world_hash.rs`; versioned whole-world snapshot.

**Step 1: Recheck schema ownership**

Immediately before editing:

```powershell
rg -n "const SNAPSHOT_VERSION: u32 = 28" src/sim/snapshot.rs
git status --short
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

If version is no longer 28, do not choose 29. Inspect the intervening schema
change, update this task's final version, and coordinate with its owner.

**Step 2: Install final `MissionCom`**

Move the reduced struct out of `mod.rs` and implement the final private fields in
`state.rs`. Use:

```rust
impl MissionCom {
    pub(crate) const fn at_frame(frame: u32) -> Self {
        Self {
            current: MissionId::NONE,
            suspended: MissionId::NONE,
            queued: MissionId::NONE,
            movement_bypass_latch: 0,
            handler_state: 0,
            mission_start_frame: 0,
            ai_counter: 0,
            dispatch_timer: MissionDispatchTimer::at_frame(frame),
        }
    }

    pub const fn current(&self) -> MissionId { self.current }
    pub const fn suspended(&self) -> MissionId { self.suspended }
    pub const fn queued(&self) -> MissionId { self.queued }
    pub const fn movement_bypass_latch(&self) -> u8 {
        self.movement_bypass_latch
    }
    pub const fn handler_state(&self) -> u32 { self.handler_state }
    pub const fn mission_start_frame(&self) -> u32 {
        self.mission_start_frame
    }
    pub const fn ai_counter(&self) -> u32 { self.ai_counter }
    pub const fn dispatch_timer(&self) -> MissionDispatchTimer {
        self.dispatch_timer
    }

    pub fn effective(&self) -> MissionId {
        if self.current == MissionId::NONE {
            self.queued
        } else {
            self.current
        }
    }
}
```

No gameplay `Default`, `idle`, public fields, public arbitrary setter, or
`#[serde(default)]` is allowed.

**Step 3: Rewrite compatibility internals without changing callers**

Map old behavior deliberately:

- `legacy_full_retask`: current=requested, queued/suspended=None,
  handler_state=0, dispatch timer=(now,0); preserve B8, mission-start, and
  AI counter.
- `legacy_current_only_retask`: current only.
- both projection functions: AI counter wrapping increment, current write, and
  `handler_state=u32::from(substate)`; preserve all other final fields.

These are compatibility semantics, not native verb semantics.

**Step 4: Attach final entity state**

Add:

```rust
pub mission: MissionCom,
pub(crate) mission_leaf: MissionLeafState,
pub(crate) suspended_attack_target: Option<TargetKind>,
pub(crate) object_is_falling_down: u8,
```

All four fields serialize without fallback defaults. Initialize leaf state from
`EntityCategory`, archive target to None, and falling byte to zero.

Replace `GameEntity::new` with `new_at_frame(..., construction_frame)`. Construct
Mission state using `MissionCom::at_frame(construction_frame)`.

Add an explicitly named frame-zero test constructor and make `test_default` call
it. Do not change the 291 existing `test_default` callsites.

Update the three `world_spawn.rs` production calls to pass
`self.session.binary_frame`. No production path may call the frame-zero helper.

Attach the raw locomotor input in the same schema boundary:

```rust
pub mission_ready_state: Option<LocomotorReadyState>,
```

Every production `LocomotorState` constructor initializes it to `None`; `None`
means exact input unavailable, not stopped. Do not derive it from `phase`,
`movement_target`, throttle, current Rust fixed-point speed, or altitude. Serialize
it without an old-layout fallback because the header-first version gate rejects
older bodies.

**Step 5: Fold every new value in fixed order**

Use raw scalar values, not enum normalization:

```text
MissionCom:
  current.raw
  suspended.raw
  queued.raw
  B8
  handler_state
  mission_start_frame
  ai_counter
  dispatch_start
  dispatch_delay

Entity Mission auxiliaries:
  leaf category tag
  leaf raw fields in declaration order
  suspended target presence + TargetKind
  object_is_falling_down

Locomotor:
  mission-ready-state presence
  variant tag
  variant raw fields in declaration order
```

Do not fold `+0xCC`. Do not rely on `MissionType` casts. Preserve existing
EntityStore order and existing RNG fold order.

Add per-field perturbation tests that start from two identical simulations,
change exactly one field through a test fixture API, and assert unequal hashes.
Include unknown Mission ID, each leaf field, each suspended target variant,
falling byte, and every locomotor-ready variant field.

**Step 6: Bump and harden the snapshot boundary**

If the precheck still proves 28 ownership, change to 29 and add one precise
version comment. Load must reject the header before the final body:

```rust
pub fn load(bytes: &[u8]) -> Result<GameSnapshot, SnapshotError> {
    let header: GameSnapshotHeader = bincode::deserialize(bytes)?;
    if header.version != SNAPSHOT_VERSION {
        return Err(SnapshotError::VersionMismatch {
            expected: SNAPSHOT_VERSION,
            found: header.version,
        });
    }
    Ok(bincode::deserialize(bytes)?)
}
```

Keep `read_header` behavior consistent. Add:

- final-schema round-trip with unknown raw Mission IDs and every archive/leaf
  variant;
- a serialized header-only old-version blob whose body cannot decode, proving
  `load` returns `VersionMismatch` before attempting the body;
- current-version malformed body returning `DeserializeFailed`;
- frame-37 entity round-trip preserving dispatch start 37.

**Step 7: Verify the production build boundary**

This task may precede mechanical test migration, so first run:

```powershell
cargo check -q -p vera20k --lib
```

Expected: exit code 0. Then run the snapshot/hash-focused tests after Task 7 has
migrated test field access. Record literal `test result:` lines rather than
inferring success from command completion.

---

### Task 7: Migrate Test Construction and Private-State Access Without Losing Dirty Work

**Why:** Private authority is only real when tests also stop reaching into fields,
but the migration must preserve the pre-existing `techno_ai.rs` work.

**Files:**

- Modify: `src/sim/game_entity.rs` tests
- Modify: `src/sim/world/mission_authoritative_tests.rs`
- Modify: `src/sim/world/slice6_retask_tests.rs`
- Modify: `src/sim/world/techno_ai.rs` tests under Task 0 ownership
- Modify: direct-constructor files in Appendix A
- Modify: other tests found by the final field-access census

**Pattern:** `#[cfg(test)]` fixture builder plus production read-only getters.

**Step 1: Add one explicit test fixture API**

Keep it unavailable in production:

```rust
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct MissionTestFixture {
    pub current: MissionId,
    pub suspended: MissionId,
    pub queued: MissionId,
    pub movement_bypass_latch: u8,
    pub handler_state: u32,
    pub mission_start_frame: u32,
    pub ai_counter: u32,
    pub dispatch_timer: MissionDispatchTimer,
}

#[cfg(test)]
impl MissionCom {
    pub(crate) fn apply_test_fixture(&mut self, fixture: MissionTestFixture) {
        self.current = fixture.current;
        self.suspended = fixture.suspended;
        self.queued = fixture.queued;
        self.movement_bypass_latch = fixture.movement_bypass_latch;
        self.handler_state = fixture.handler_state;
        self.mission_start_frame = fixture.mission_start_frame;
        self.ai_counter = fixture.ai_counter;
        self.dispatch_timer = fixture.dispatch_timer;
    }
}
```

Provide equivalent category-checked leaf and locomotor test fixture operations.
Tests read through normal getters.

**Step 2: Rename direct frame-free construction**

- Production: only `world_spawn.rs` uses `new_at_frame`.
- Tests: direct calls use `new_at_frame_zero_for_test`.
- `GameEntity::test_default` internally uses frame zero and its 291 callers stay
  unchanged.

After edits:

```powershell
rg -n "GameEntity::new\(" src --glob "*.rs"
rg -n "new_at_frame_zero_for_test" src --glob "*.rs"
rg -n "new_at_frame\(" src/sim/world/world_spawn.rs
```

Expected: zero old constructor calls, test helper calls only in test code, and
exactly three production frame-aware spawn calls.

**Step 3: Replace direct Mission field access**

Search:

```powershell
rg -n "\.mission\.(current|queued|suspended|substate|timer|tick_counter)" src --glob "*.rs"
```

Replace reads with `current().known()`, raw getters, or compatibility-specific
read helpers. Replace writes with `MissionTestFixture` in tests and reviewed
semantic operations in production. Do not add `pub(crate)` fields to silence the
compiler.

Update direct `LocomotorState { ... }` test literals found by:

```powershell
rg -n "LocomotorState \{" src --glob "*.rs"
```

Add `mission_ready_state: None` unless the fixture is explicitly testing one of
the six raw variants. Do not add a serde fallback or derive from another field.

**Step 4: Prove the dirty diff was preserved**

Compare the full `techno_ai.rs` diff to the Task 0 capture. The existing 1497/2
work must remain, with only the enumerated mechanical migrations added. If
behavioral test code changes beyond API adaptation, stop and return it to the
owner for review.

**Step 5: Verify**

Run serially:

```powershell
cargo test -p vera20k game_entity -- --nocapture
cargo test -p vera20k mission_authoritative -- --nocapture
cargo test -p vera20k slice6_retask -- --nocapture
cargo test -p vera20k snapshot -- --nocapture
cargo test -p vera20k world_hash -- --nocapture
```

Expected literal result for each: `test result: ok`.

---

### Task 8: Implement the Six Exact Locomotor Ready Predicates

**Why:** Unit and Infantry readiness call one virtual slot whose six active
implementations have different mechanisms.

**Files:**

- Modify: `src/sim/movement/locomotor_ready.rs`

**Pattern:** Pure raw-state decision functions; no conversion through simulation
float or generic movement state.

**Step 1: Implement native zero/ordered-positive helpers**

```rust
const F64_SIGN: u64 = 1 << 63;
const F64_EXPONENT: u64 = 0x7ff0_0000_0000_0000;
const F64_FRACTION: u64 = 0x000f_ffff_ffff_ffff;
const F64_MAGNITUDE: u64 = !F64_SIGN;

fn native_double_is_nan(bits: u64) -> bool {
    bits & F64_EXPONENT == F64_EXPONENT && bits & F64_FRACTION != 0
}

fn native_double_ordered_not_zero(bits: u64) -> bool {
    bits & F64_MAGNITUDE != 0 && !native_double_is_nan(bits)
}

fn native_double_ordered_gt_zero(bits: u64) -> bool {
    let magnitude = bits & F64_MAGNITUDE;
    let is_negative = bits & F64_SIGN != 0;
    !is_negative && magnitude != 0 && !native_double_is_nan(bits)
}
```

This treats `+0.0` and `-0.0` as zero and every NaN as unordered for both
locomotors. Hover accepts finite or infinite nonzero values of either sign;
Walk accepts only ordered positive values. Positive subnormal and positive
infinity are therefore greater than zero, matching the native x87 comparisons
without introducing `f64`.

**Step 2: Implement the six mechanisms**

```rust
impl LocomotorReadyState {
    pub(crate) fn is_moving_now(self) -> bool {
        match self {
            Self::Drive {
                turning_active,
                slot_moving,
                head_to_nonnull,
                owner_speed,
            } => {
                turning_active
                    || (slot_moving && head_to_nonnull && owner_speed > 0)
            }
            Self::Ship {
                turning_active,
                slot_moving,
                head_to_nonnull,
                owner_speed,
            } => {
                turning_active
                    || (slot_moving && head_to_nonnull && owner_speed > 0)
            }
            Self::Hover { slot_moving, speed_bits } => {
                slot_moving && native_double_ordered_not_zero(speed_bits)
            }
            Self::Walk {
                moving_byte,
                applied_speed_bits,
                destination_nonnull,
            } => {
                moving_byte != 0
                    && native_double_ordered_gt_zero(applied_speed_bits)
                    && destination_nonnull
            }
            Self::Teleport { state } => state == 1,
            Self::Jumpjet { state } => state != 0 && state != 2,
        }
    }
}
```

Do not merge Drive and Ship into a stored generic variant even though their final
boolean formula matches; their slot inputs and producers remain distinct.

**Step 3: Add per-family truth tables**

Cover every conjunct independently and:

- Drive/Ship turning true with every other input false;
- signed owner speed `-1`, `0`, `1`;
- Hover `+0`, `-0`, positive, negative, infinity, and NaN raw bits;
- Walk `+0`, `-0`, positive subnormal, negative, positive infinity, and NaN;
- Teleport state 0,1,2,255;
- Jumpjet state -1,0,1,2,3.

Also assert no test constructs or calls an `f64`.

**Step 4: Verify**

```powershell
cargo test -p vera20k locomotor_ready -- --nocapture
```

Expected literal result: `test result: ok`.

---

### Task 9: Implement Exact Readiness as Pure Borrowed Views

**Why:** Readiness must preserve category-specific branch order and report missing
authoritative inputs instead of substituting current Rust approximations.

**Files:**

- Modify: `src/sim/mission/readiness.rs`
- Modify: `src/sim/mission/mod.rs`
- Read/integrate: `src/sim/occupancy.rs`, `src/sim/radio/`, rules lookup surfaces

**Pattern:** Borrowed read-only query views; no cached entity snapshot, allocation,
mutation, or RNG.

**Step 1: Define explicit availability and category views**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ReadyUnavailable {
    #[error("exact locomotor readiness input is unavailable")]
    Locomotor,
    #[error("exact signed object height is unavailable")]
    SignedHeight,
    #[error("Infantry Doing value is outside the verified writer domain: {0}")]
    InvalidDoing(i32),
    #[error("required entity/type/occupancy lookup is unavailable")]
    WorldLookup,
}

pub(crate) type ReadyResult = Result<bool, ReadyUnavailable>;
```

Use category-specific structs rather than optional fields in one coarse struct.
Each view carries only the already-borrowed/scalar input needed by its branch.
Synthetic tests can construct complete views. The production view builder returns
an error if exact signed height, locomotor state, Doing invariant, contact/type
lookup, or occupancy list cannot be supplied.

**Step 2: Encode the Doing table exactly**

```rust
const INFANTRY_READY_BY_DOING: [bool; 42] = [
    true,  true,  true,  true,  true,  false, true,  false,
    true,  true,  true,  false, false, false, false, false,
    true,  true,  true,  true,  false, false, true,  true,
    true,  true,  true,  false, true,  true,  true,  false,
    false, true,  false, false, false, true,  true,  true,
    true,  true,
];

fn infantry_doing_allows(doing: i32) -> ReadyResult {
    if doing == -1 {
        return Ok(true);
    }
    let index = usize::try_from(doing)
        .map_err(|_| ReadyUnavailable::InvalidDoing(doing))?;
    INFANTRY_READY_BY_DOING
        .get(index)
        .copied()
        .ok_or(ReadyUnavailable::InvalidDoing(doing))
}
```

Do not clamp, modulo, treat every negative as idle, or read adjacent bytes.

**Step 3: Implement Unit readiness in native branch order**

Use these exact stages:

```text
1. current raw mission 6 or 21 -> false
2. Unit deploy-begin byte nonzero -> false
3. Unit deploy-reverse byte nonzero -> false
4. existing Miner unload_active (+0x6D1) -> false
5. unless queued is Enter(7), obtain exact locomotor result; if moving,
   signed height >= 0, effective mission is not Guard(5),
   effective mission is not Attack(1) with no target, and B8 is zero -> false
6. either tracker byte nonzero -> false
7. Radio slot 0 is a Building with WeaponsFactory and queued is neither
   Move(2) nor Enter(7) -> false
8. only when slot 0 is empty, walk the selected cell occupancy list in stored
   order; the native building-under result with WeaponsFactory, equal low-16-bit
   signed X cell, and wrapping low-16-bit signed Y delta +1 -> false
9. true
```

Signed cell conversion is:

```rust
fn native_lepton_cell(value: i32) -> i16 {
    (value.wrapping_add((value >> 31) & 0xff) >> 8) as i16
}
```

The cast preserves native `MOV word` truncation before the `TEST AX` and
`CMP CX,1` comparisons. Do not use `div_euclid`, full-width cell comparison,
unsigned casts, footprint membership, any-contact membership, land-factory
helpers, sorted stable IDs, or a fresh vector.

Instrument test-only lookup counters to prove an earlier false branch does not
query later contact/occupancy inputs.

**Step 4: Implement Infantry readiness in native branch order**

```text
1. current raw mission 6 or 21 -> false
2. firing-sequence latch nonzero -> false
3. Object falling byte nonzero -> false
4. obtain exact locomotor result
5. resolve effective mission as current unless current is None(-1), then queued
6. moving and effective not Guard(5)/Hunt(15):
     effective not Attack(1) -> false
     Attack with nonnull target -> false
7. Doing -1 -> true; otherwise return exact table byte
```

Only the initial Sticky/Rescue gate reads raw current. The moving exceptions call
the Mission `+0x184` getter, which falls back from current None(-1) to queued.

**Step 5: Implement Aircraft and Building readiness**

```rust
fn aircraft_ready(view: AircraftReadyView<'_>) -> bool {
    let current = view.mission.current();
    if current == MissionId::from_raw(6) || current == MissionId::from_raw(21) {
        return false;
    }
    if current != MissionId::from_raw(0x1e) && view.action_latch != 0 {
        return false;
    }
    view.transition_ready_latch != 0
}

fn building_ready(view: BuildingReadyView) -> bool {
    view.ready_latch != 0
}
```

Base/Radio/Foot-only views return true. Do not infer Aircraft readiness from
altitude or Building readiness from construction completion.

**Step 6: Add complete tests**

Required groups:

- Unit branch precedence, every raw byte, queued Enter bypass, signed height,
  Guard/Attack-null/B8 exceptions, both tracker bytes, Radio slot 0, Move/Enter
  exceptions, wrong contact category, exact factory fallback, and signed negative
  coordinate fixtures.
- Infantry excluded missions, firing, falling, all moving exceptions, Doing `-1`,
  all 42 table entries, and invalid `-2`, `42`, and `i32::MAX`.
- Aircraft current/action/transition matrix including raw `0x1E`.
- Building latch zero/nonzero.
- A complete view leaves RNG state unchanged and performs no mutation.

Rust tests ratchet the documented mechanism but do not certify gamemd parity.

**Step 7: Verify**

```powershell
cargo test -p vera20k unit_ready_to_commence -- --nocapture
cargo test -p vera20k infantry_ready_to_commence -- --nocapture
cargo test -p vera20k aircraft_ready_to_commence -- --nocapture
cargo test -p vera20k building_ready_to_commence -- --nocapture
```

Expected literal result for every command: `test result: ok`.

---

### Task 10: Implement Category-Aware Assign, Queue, and Commence Authority

**Why:** Base transitions become useful only through receiver-family policy and
Queue's synchronous leaf dispatch.

**Files:**

- Modify: `src/sim/mission/authority.rs`
- Modify: `src/sim/mission/leaf.rs`
- Modify: `src/sim/world/mod.rs` only for module compilation, not a tick call
- Add focused authority tests

**Pattern:** Simulation-level lookup/borrow orchestration; leaf policy is
Rust-native enum dispatch preserving native virtual order.

**Step 1: Define error and Aircraft policy**

```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum MissionAuthorityError {
    #[error("Mission receiver {0} does not exist")]
    MissingReceiver(u64),
    #[error(transparent)]
    Readiness(#[from] ReadyUnavailable),
}

const AIRCRAFT_PROTECTED: [MissionId; 5] = [
    MissionId::from_raw(4),
    MissionId::from_raw(0x1a),
    MissionId::from_raw(0x1b),
    MissionId::from_raw(0x1e),
    MissionId::from_raw(0x1f),
];

fn aircraft_allows(
    current: MissionId,
    requested: MissionId,
    manager_present: bool,
) -> bool {
    manager_present
        || !AIRCRAFT_PROTECTED.contains(&current)
        || AIRCRAFT_PROTECTED.contains(&requested)
}
```

Classification is full `i32`; unknown and None are outside the set.

**Step 2: Implement Assign**

1. Resolve receiver; missing means no writes and an error.
2. If Aircraft, run the gate before base Assign.
3. If blocked, return success with no writes.
4. Otherwise call `assign_base`.

Do not return a native transition boolean.

**Step 3: Implement leaf Commence**

```rust
fn commence_leaf(entity: &mut GameEntity, now: u32) -> bool {
    if let MissionLeafState::Aircraft(leaf) = &mut entity.mission_leaf {
        let old_current = entity.mission.current();
        if old_current != MissionId::from_raw(0x1e) {
            leaf.clear_action_latch_for_commence();
        }
    }
    verb::commence_base(&mut entity.mission, now)
}
```

The Aircraft hook runs before the base call and therefore runs with an empty queue.
Other categories call base directly.

**Step 4: Implement synchronous Queue**

Use this borrow sequence:

```text
1. Resolve receiver before any write.
2. Apply the Aircraft gate read-only; a blocked leaf returns without any write.
3. Copy MissionCom and run queue_base on the copy. If its outer guard blocks,
   return without readiness. When `(commence_now & 0xff) != 0`, ask the provider to validate
   exactly the non-Mission inputs the previewed post-Queue branch can read; an
   unavailable result exits before real Queue mutation. Queued Enter therefore
   does not require a locomotor input that native skips.
4. Call queue_base on the real state under a short mutable entity borrow.
5. If `(commence_now & 0xff) == 0`, return.
6. End the mutable borrow.
7. Build a fresh readiness view from the just-mutated Mission state and current
   world inputs. This is a new read, not the preview values from Step 3.
8. If Ready is false, return with queued state intact.
9. Reacquire receiver and call category leaf Commence immediately.
```

The nonzero-low-byte flag path runs for requested None and redundant requests.
A zero low byte never self-promotes, even if a higher byte is nonzero. Do not
cache the readiness view before the Queue mutation and do
not enqueue a later command. The preview is a pure copy and never becomes
authoritative. Provider validation must not retain the readiness boolean; it
proves branch-specific availability only, preventing a Rust integration error
from leaving a partially mutated queue.

**Step 5: Add ordered tests**

Cover:

- Aircraft protected current with manager absent blocks Assign and Queue;
- manager present and protected requested IDs pass;
- unknown and None requests are blocked under protected/no-manager state;
- Queue outer guards skip readiness;
- unavailable readiness input leaves every receiver field unchanged;
- Queue None/redundant with nonzero flag calls readiness exactly once;
- Queue flag zero never calls readiness or Commence;
- Ready false preserves queued state;
- Ready true calls leaf Commence in the same method;
- Aircraft Queue-owned Commence clears action latch except old `0x1E`;
- Aircraft empty-queue direct Commence still performs the latch hook;
- all operations preserve all three RNG logical states.

Use a test provider with call counters; test-only vectors may record order, while
production code allocates nothing.

**Step 6: Confirm zero production callers**

```powershell
rg -n "mission_(assign|queue|commence)_exact" src --glob "*.rs"
```

Expected: definitions and tests only.

**Step 7: Verify**

```powershell
cargo test -p vera20k mission_authority_assign -- --nocapture
cargo test -p vera20k mission_authority_queue -- --nocapture
cargo test -p vera20k aircraft_mission_authority -- --nocapture
```

Expected literal result for every command: `test result: ok`.

---

### Task 11: Implement Ordered Concrete Override and Restore Transactions

**Why:** Concrete Techno/Foot behavior archives and reinstalls Target/NavCom around
the common transition; a bare Mission pop is known DRIFT.

> **Review correction / current stop condition:** Do not implement the generic
> setter code below. It is retained as the original planning sketch, but live
> review falsified its completeness. Production must remain on
> `UnavailableConcreteMissionEffects` until base and category Target setters,
> mode-one destination installation, the base Target `+0x50C` latch, and all
> same-call-stack side effects have an exact Rust representation. The recording
> provider may test wrapper order only; it is not parity evidence.

**Files:**

- Modify: `src/sim/mission/authority.rs`
- Modify: `src/sim/combat/mod.rs`
- Modify: `src/sim/components.rs`
- Modify: `src/sim/movement/navcom.rs`
- Modify: `src/sim/game_entity.rs`
- Add focused transaction tests

**Pattern:** Entity-level semantic setters and category-specific wrapper order,
not player-command validation.

**Step 1: Add concrete target identity setter**

```rust
impl AttackTarget {
    pub(crate) fn from_kind(target: TargetKind) -> Self {
        match target {
            TargetKind::Entity(id) => Self::new(id),
            TargetKind::Cell(rx, ry) => Self::for_cell(rx, ry),
        }
    }
}

fn install_mission_target(
    entity: &mut GameEntity,
    requested: Option<TargetKind>,
) {
    if entity.attack_target.as_ref().map(|active| active.target) == requested {
        return;
    }
    entity.attack_target = requested.map(AttackTarget::from_kind);
}
```

The same-identity early return preserves cooldown, burst, and pending Infantry
fire state. This helper does not run range checks, target existence checks, player
command teardown, or eager reference expiration.

**Step 2: Add the sealed mode-one NavCom setter**

Expose a crate-private Mission-only function:

```rust
pub(crate) fn set_mission_destination_mode_one(
    entity: &mut GameEntity,
    destination: Option<NavTargetRef>,
) {
    entity.navigation.nav_com_aux = None;
    entity.navigation.nav_com = destination;
    entity.navigation.pending_arrival_clear = false;
}
```

Where an existing exact cell/null owner operation is required by the reviewed
NavCom contract, delegate to it without changing call order. Do not guess
coordinates for Entity/Object/Building references. Keep all exact concrete
authority callers dormant until `/review-plan` and the later provider crosswalk
confirm the full setter effects for each reference kind.

**Step 3: Implement Override wrapper order**

After a missing-receiver precheck and Aircraft gate:

```text
Building:
  suspended_attack_target = active target identity
  override_base
  install requested target

Unit / Infantry / Aircraft:
  suspended_nav_com = active nav_com
  suspended_attack_target = active target identity
  override_base
  install requested target
  set requested destination with mode one

Mission/Radio substrate, if represented:
  override_base only
```

Once a non-Aircraft wrapper begins, the concrete setters execute even when
`override_base` is guarded. Override returns `()`.

**Step 4: Implement Restore wrapper order**

```text
1. resolve receiver before writes
2. call restore_base
3. if false, return false with no concrete setter
4. Building/Techno: install saved target
5. Foot-derived: install saved target, then saved NavCom with mode one
6. return true
```

Do not clear either saved slot on success. Aircraft Restore has no protected-set
or manager gate.

**Step 5: Add exact trace/state tests**

Required fixtures:

- base-only receiver;
- Building trace `archive target -> base -> set target`;
- Unit and Infantry trace `archive nav -> archive target -> base -> set target ->
  set nav`;
- allowed Aircraft uses the Foot trace;
- blocked Aircraft produces an empty trace and byte-identical state;
- Deliberate-to-Guard base guard still reaches target/nav setters after wrapper
  entry;
- Restore failure runs no setter;
- Restore success runs target then nav and retains saved slots;
- Building never receives a NavCom write;
- same target identity preserves active `AttackTarget` runtime counters;
- missing receiver causes no mutation anywhere.

**Step 6: Confirm zero production callers**

```powershell
rg -n "mission_(override|restore)_exact" src --glob "*.rs"
```

Expected: definitions and tests only.

**Step 7: Verify**

```powershell
cargo test -p vera20k mission_override_transaction -- --nocapture
cargo test -p vera20k mission_restore_transaction -- --nocapture
```

Expected literal result for each: `test result: ok`.

---

### Task 12: Add Narrow Verified-Host Writer Operations Without Activating Missing Owners

**Why:** The final state needs semantic writer entry points for known native owner
events, but absent current Rust owners must remain visible rather than synthesized.

**Files:**

- Modify: `src/sim/mission/state.rs`
- Modify: `src/sim/mission/leaf.rs`
- Modify: `src/sim/mission/authority.rs`
- Modify: `src/sim/movement/locomotor_ready.rs`
- Add unit tests in those modules

**Pattern:** Narrow event-named methods, no arbitrary field/value API.

**Step 1: Add common host methods**

Provide exactly:

```rust
pub(crate) fn increment_ai_counter(&mut self) {
    self.ai_counter = self.ai_counter.wrapping_add(1);
}

pub(crate) fn write_dispatch_epilogue(
    &mut self,
    start_frame: i32,
    delay: i32,
) {
    self.dispatch_timer =
        MissionDispatchTimer::from_raw(start_frame, delay);
}

pub(crate) fn set_movement_bypass_after_verified_queue(&mut self) {
    self.movement_bypass_latch = 1;
}
```

If a current producer is not proven to be the native owner, leave these methods
without a production caller. Compatibility projection remains separately named.
The B8 setter remains private to Mission authority and is callable only by the
two ordered helpers below; it is not a general public latch setter.

**Step 2: Add category-specific writer methods**

Provide this bounded set:

```rust
pub(crate) fn set_unit_deploy_begin_active(&mut self, raw: u8);
pub(crate) fn set_unit_deploy_reverse_active(&mut self, raw: u8);
pub(crate) fn set_unit_tracker_byte_18(&mut self, raw: u8);
pub(crate) fn set_unit_tracker_byte_19(&mut self, raw: u8);
pub(crate) fn set_infantry_firing_sequence(&mut self, raw: u8);
pub(crate) fn set_aircraft_transition_ready(&mut self, raw: u8);
pub(crate) fn clear_aircraft_action_for_commence(&mut self);
pub(crate) fn set_building_ready_latch(&mut self, raw: u8);
pub(crate) fn set_infantry_doing_verified(&mut self, doing: i32)
    -> Result<(), ReadyUnavailable>;
```

The Doing writer accepts only `-1` or `0..=41`. Leaf methods check their category
and do not silently write another variant.

Do not wire current animations, altitude, construction state, parachute option,
or generic movement phase as proxy writers. `object_is_falling_down` and raw
locomotor state receive test fixture methods now; production owner wiring remains
deferred unless its exact same-tick lifecycle is independently proven during
execution.

**Step 3: Add inert verified owner-sequence helpers**

Add crate-private authority operations with zero production callers:

```rust
pub(crate) fn mission_refinery_completion_exact(
    &mut self,
    receiver: u64,
    now: u32,
) -> Result<(), MissionAuthorityError>;

pub(crate) fn mission_jumpjet_move_to_completion_exact(
    &mut self,
    receiver: u64,
    now: u32,
    readiness: &impl ReadyInputProvider,
) -> Result<(), MissionAuthorityError>;

pub(crate) fn mission_try_consume_building_ready_exact(
    &mut self,
    receiver: u64,
    now: u32,
) -> Result<bool, MissionAuthorityError>;
```

They preserve these verified sequences:

```text
Refinery/deploy completion:
  Queue(Guard, false)
  B8 = 1

Jumpjet Move_To completion:
  Queue(Guard, true)          # owns its first Ready/conditional Commence
  B8 = 1
  Ready                      # second gate observes nonzero B8
  if Ready: Commence         # success clears B8 through base Commence

Building consume point:
  Ready
  if false: return false and preserve latch
  Commence
  if Commence succeeds: clear Building ready latch and return true
  if Commence is empty: preserve latch and return false
```

The future Building owner may invoke the consume helper at its two verified local
positions around Techno AI. This foundation does not insert either call into the
current global tick. The Jumpjet and refinery helpers likewise remain dormant
until their current Rust owner crosswalks close.

**Step 4: Test invalid ownership and ordered helpers**

Assert:

- writing an Aircraft latch through Unit/Infantry/Building state fails loudly in
  test/debug paths;
- invalid Doing does not mutate old state;
- wrapping AI counter goes `u32::MAX -> 0`;
- dispatch epilogue preserves all non-timer fields;
- refinery trace is exactly Queue then B8;
- Jumpjet trace contains the Queue-owned first gate, then B8, then the explicit
  second Ready/conditional Commence;
- a second Jumpjet Commence success clears B8, while a later tracker/factory
  readiness failure leaves it set;
- empty Building Commence leaves its ready latch armed;
- successful Building Commence clears the ready latch;
- no writer allocates or reads RNG.

**Step 5: Confirm dormant ownership**

```powershell
rg -n "mission_(refinery_completion|jumpjet_move_to_completion|try_consume_building_ready)_exact" src --glob "*.rs"
```

Expected: definitions and tests only.

**Step 6: Verify**

```powershell
cargo test -p vera20k mission_verified_host_write -- --nocapture
cargo test -p vera20k mission_leaf_writer -- --nocapture
cargo test -p vera20k mission_b8_owner_sequence -- --nocapture
cargo test -p vera20k building_ready_consume -- --nocapture
```

Expected literal result for every command: `test result: ok`.

---

### Task 13: Enforce the Writer and Caller Census Mechanically

**Why:** The rollout boundary is a correctness property: current compatibility
calls must be enumerable and exact authority must remain dormant.

**Files:**

- Create: `tools/check_mission_authority_census.py`

**Pattern:** Existing repo validation scripts: deterministic read-only check with
explicit allowlists and nonzero exit on drift.

**Step 1: Encode reviewed production callsites**

The script walks `src/**/*.rs`, ignores lines inside comments for token checks,
and verifies:

```python
EXPECTED_COMPATIBILITY = {
    "legacy_full_retask": {
        "src/sim/mission/retask.rs",
        "src/sim/docking/bunker_link.rs",
    },
    "legacy_current_only_retask": {
        "src/sim/mission/retask.rs",
    },
    "legacy_unit_host_projection": {
        "src/sim/world/techno_ai.rs",
    },
    "legacy_tick_tail_projection": {
        "src/sim/world/mod.rs",
    },
}

EXACT_AUTHORITY_TOKENS = {
    "mission_assign_exact",
    "mission_queue_exact",
    "mission_commence_exact",
    "mission_override_exact",
    "mission_restore_exact",
    "mission_refinery_completion_exact",
    "mission_jumpjet_move_to_completion_exact",
    "mission_try_consume_building_ready_exact",
}

ALLOWED_EXACT_FILES = {
    "src/sim/mission/authority.rs",
}
```

Definitions and in-module tests are allowed in `authority.rs`; any token in
another production file fails.

**Step 2: Reject direct writes and a global drain**

Fail on assignment-like production tokens for:

```text
mission.current
mission.suspended
mission.queued
mission.movement_bypass_latch
mission.handler_state
mission.mission_start_frame
mission.ai_counter
mission.dispatch_timer
```

Also fail if exact Commence appears in `advance_tick`, a tail phase, a global
entity loop, or a generic queued-mission drain. Test fixture methods are allowed
only under `#[cfg(test)]`.

The script prints every matched path and line before returning failure so a
reviewer can distinguish a real new owner from an accidental call.

**Step 3: Add a self-test fixture for the checker**

Use temporary in-memory/source strings or a temporary directory under the test
runner to prove:

- each exact token outside the allowlist fails;
- an extra compatibility path fails;
- a direct field assignment fails;
- definitions/tests in the authority file pass;
- the current repository passes.

The checker never edits source.

**Step 4: Verify**

```powershell
python tools/check_mission_authority_census.py
```

Expected: exit code 0 and a concise census listing the four compatibility
families and zero production exact-authority callers.

---

### Task 14: Run Final Serial Verification and Record Honest Status

**Why:** The foundation is complete only when current gameplay still builds/tests,
all new state is deterministic/persistent, and the dormant-authority boundary is
proven.

**Files:**

- Modify:
  `docs/contracts/2026-07-21-mission-state-verb-readiness-authority-implementation-contract.md`
  only after the checks pass
- Inspect all changed files

**Pattern:** Focused tests first, then one serial library test and check; surgical
formatting only.

**Step 1: Format only edited Rust files**

Run `rustfmt --edition 2024` with the explicit edited Rust path list. Do not run
crate-wide `cargo fmt`. Inspect the diff immediately for unrelated churn.

**Step 2: Run static checks**

```powershell
python tools/check_mission_authority_census.py
rg -n "GameEntity::new\(" src --glob "*.rs"
rg -n "\.mission\.(current|queued|suspended|substate|timer|tick_counter)\s*=" src --glob "*.rs"
rg -n "mission_(assign|queue|commence|override|restore)_exact" src --glob "*.rs"
git diff --check
```

Expected:

- no old constructor;
- no direct reduced-field assignment;
- exact authority definitions/tests only;
- no whitespace errors.

**Step 3: Run focused tests serially**

Before Cargo:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

Then, never in parallel:

```powershell
cargo test -p vera20k mission -- --nocapture
cargo test -p vera20k weapons_factory -- --nocapture
cargo test -p vera20k locomotor_ready -- --nocapture
cargo test -p vera20k snapshot -- --nocapture
cargo test -p vera20k world_hash -- --nocapture
cargo test -p vera20k bunker_link -- --nocapture
cargo test -p vera20k techno_ai -- --nocapture
```

Read and record every literal `test result:` line.

**Step 4: Run the final crate gates serially**

```powershell
cargo test -p vera20k --lib
cargo check -q -p vera20k
```

Expected literal library result: `test result: ok`; expected check exit code: 0.
Do not infer success from a quiet or timed-out command.

**Step 5: Inspect the final change boundary**

```powershell
git status --short
git diff --stat
git diff -- src/sim/mission src/sim/game_entity.rs src/sim/world/world_hash.rs src/sim/snapshot.rs
git diff -- src/sim/world/techno_ai.rs
```

Confirm:

- unrelated user changes are preserved;
- no render/UI/audio/net dependency entered sim;
- no `MissionTimer` consumer changed behavior;
- no snapshot or golden work from another session was overwritten;
- no exact authority caller is live;
- no raw `+0xCC` surrogate exists.

**Step 6: Refresh the contract status**

Record:

- final HEAD/worktree evidence;
- exact test commands and literal results;
- snapshot version selected after the ownership check;
- compatibility census;
- zero production exact-authority caller count;
- any narrow decision changed by `/review-plan`.

The verdict remains `PARTIAL/UNVERIFIED`. State precisely that Rust tests are
regression evidence, not gamemd parity certification. Keep the player-action and
legacy-FSM crosswalks, raw-save `+0xCC`, Airstrike manager, and inactive writer
owners in the remaining-blocker section.

The implementation stop condition is reached only when all checks pass and
current gameplay remains behind the same compatibility callsites.

---

## Appendix A: Direct Constructor Migration Set

At planning time, `GameEntity::new` occurs 76 times across these 36 files. Only
the three `world_spawn.rs` calls are production. Re-run the census at execution
time; this list is a review baseline, not permission to overwrite concurrent
changes.

```text
src/app_context_order.rs
src/app_fire_effects.rs
src/app_render_tests.rs
src/app_target_lines.rs
src/sim/ai.rs
src/sim/anim_class.rs
src/sim/animation_tests.rs
src/sim/deploy_tests.rs
src/sim/docking/bunker_install.rs
src/sim/docking/bunker_link.rs
src/sim/game_entity.rs
src/sim/infantry.rs
src/sim/miner/miner_tests.rs
src/sim/movement/movement_tests.rs
src/sim/movement/path_markers.rs
src/sim/movement/prone_speed_tests.rs
src/sim/movement/tube_movement.rs
src/sim/particles/spark_world.rs
src/sim/production/production_sell.rs
src/sim/production/production_tests.rs
src/sim/radar.rs
src/sim/radio/receive.rs
src/sim/rocking/self_destruct.rs
src/sim/selection.rs
src/sim/superweapon/iron_curtain.rs
src/sim/trigger_runtime_tests.rs
src/sim/vision/vision_tests.rs
src/sim/world/bridge_orchestrator.rs
src/sim/world/lifecycle_tests.rs
src/sim/world/techno_ai.rs
src/sim/world/world_commands.rs
src/sim/world/world_hash.rs
src/sim/world/world_orders_bridge_repair_tests.rs
src/sim/world/world_orders_c4_tests.rs
src/sim/world/world_spawn.rs
src/sim/world/world_tests.rs
```

## Appendix B: Direct Locomotor Fixture Migration Set

At planning time, direct `LocomotorState { ... }` literals occur in these files.
Production constructors set `mission_ready_state: None`; ordinary tests do the
same, while exact locomotor-readiness fixtures provide one explicit raw variant.

```text
src/sim/combat/in_range.rs
src/sim/movement/air_movement.rs
src/sim/movement/droppod_movement.rs
src/sim/movement/jumpjet_movement.rs
src/sim/movement/locomotor.rs
src/sim/movement/movement_bridge.rs
src/sim/movement/movement_tests.rs
src/sim/movement/parachute_descent.rs
src/sim/movement/tunnel_movement.rs
src/sim/world/bridge_orchestrator.rs
```

## Sources & References

### Approved specifications

- `docs/plans/2026-07-23-mission-authority-foundation-design.md`
- `docs/contracts/2026-07-21-mission-state-verb-readiness-authority-implementation-contract.md`

### Primary research reports

- `docs/research/MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md`
  — reverified common transition bodies and vtable slots.
- `docs/research/MISSION_ASSIGN_OVERRIDE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md`
  — complete Assign/Override caller and concrete wrapper ordering census.
- `docs/research/MISSION_QUEUE_COMMENCE_RESTORE_ACTIVE_CALLER_AUTHORITY_GHIDRA_REPORT.md`
  — complete Queue/Commence/Restore caller and same-stack ordering census.
- `docs/research/AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY_GHIDRA_REPORT.md`
  — complete five-slot Aircraft policy.
- `docs/research/MISSION_RAW_BYTES_0XB8_0XCC_FULL_CENSUS_GHIDRA_REPORT.md`
  — complete active-YR B8 and CC surface.
- `docs/research/READYTOCOMMENCE_UNIT_INFANTRY_RESIDUAL_CLOSURE_GHIDRA_REPORT.md`
  — complete Unit/Infantry predicates, six locomotors, writer fields, and 42-entry
  Doing table closure.
- `docs/research/READYTOCOMMENCE_AIRCRAFT_BUILDING_WRITER_LIFECYCLES_GHIDRA_REPORT.md`
  — complete Aircraft/Building readiness byte lifecycles.
- `docs/research/TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md`
  — Mission dispatch timing and current Rust host relationship.
- `docs/research/TIMER_PRIMITIVE_FAMILY_COUNTUP_TIMERCLASS_VS_CDTIMERCLASS_GHIDRA_REPORT.md`
  — verified timer-family semantics and separation.
- `docs/research/FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md`
  — constructor, effective Mission, selector layout, and active movement anchor.

### Live gamemd.exe verification used while writing this plan

- `0x005B2FD0` — base Assign.
- `0x005B35E0` — base Queue and synchronous nonzero continuation.
- `0x005B3570` — base Commence.
- `0x005B3650` — base Override.
- `0x005B36B0` — base Restore.
- `0x0041B870` — Aircraft Commence hook; stale local label rejected.
- `0x0041BB30` — Aircraft Override gate over the full dword protected set.
- `0x004D8F40`, `0x004D8F80` — Foot Override/Restore wrapper order.
- `0x007013A0`, `0x007013E0` — Techno Target archive/set wrapper order.
- `0x00744270` — Unit ReadyToCommence.
- `0x00521B60` — Infantry ReadyToCommence.
- `0x00523D00` — exactly 42 Infantry sequence-name entries.
- `0x007EAF7C`, 168-byte read — 42 four-byte Doing property records.
- `0x004AFC20`, `0x0069F330`, `0x00514C80`, `0x0075AB40`,
  `0x004B6610`, `0x0054D0D0` — Drive, Ship, Hover, Walk, Teleport, and
  Jumpjet ready predicates.

These addresses belong in research/planning evidence, not Rust code comments.

### INI evidence

- `ini/rules.ini`: `GAWEAP`, `NAWEAP`, `GAYARD`, `NAYARD` carry
  `WeaponsFactory=yes`.
- `ini/rulesmd.ini`: merged/added `GAWEAP`, `NAWEAP`, `GAYARD`, `NAYARD`,
  `YAWEAP`, `YAYARD` carry `WeaponsFactory=yes`.
- `ini/rulesmd.ini` Boris Airstrike keys:
  `AirstrikeTeam=2`, `EliteAirstrikeTeam=4`, team type `BPLN`,
  `AirstrikeRechargeTime=100`, and `EliteAirstrikeRechargeTime=50`.
  These configure a future manager system and do not imply runtime manager
  presence on an Aircraft.

### Current Rust anchors

- `src/sim/mission/mod.rs` — current vocabulary and reduced state.
- `src/sim/mission/timer.rs` — shared unsigned timer that remains unchanged.
- `src/sim/mission/verb.rs` — reduced transitions to replace.
- `src/sim/mission/retask.rs` — current command compatibility surface.
- `src/sim/game_entity.rs` — entity state, Target, constructor, and mission
  projection.
- `src/sim/components.rs` — NavCom and suspended NavCom.
- `src/sim/movement/locomotor.rs` — active locomotor ownership.
- `src/sim/movement/navcom.rs` — owner destination helpers.
- `src/sim/combat/mod.rs` — `TargetKind` and `AttackTarget`.
- `src/sim/occupancy.rs` — native-style stored cell list order.
- `src/rules/object_type.rs` — merged type parsing.
- `src/sim/world/world_hash.rs` — deterministic fixed-order folds.
- `src/sim/snapshot.rs` — versioned bincode envelope.
- `src/sim/world/world_spawn.rs` — the three production construction sites.
- `src/sim/world/techno_ai.rs` and `src/sim/world/mod.rs` — current host/tail
  projections.

### Git grounding

- Planning branch: `dev`.
- Planning HEAD: `928644d44fee61d0bb8d3214a4f9e0eb7390bc4e`.
- No newer Mission restructuring was present in the path histories at planning
  time; lifecycle/schema commit `95bef99d` remains the latest relevant entity
  boundary, and current Mission host work remains on the existing layout.

---

## Recommended Review Before Execution

Run `/review-plan docs/plans/2026-07-23-mission-authority-foundation-plan.md`.
The review should focus on the three medium-confidence boundaries:

1. `Option<TargetKind>` archive plus same-identity setter behavior;
2. the exact scope of the mode-one generic NavCom setter; and
3. storage/availability of the six raw locomotor-ready inputs.

No implementation should begin until Task 0's dirty-file and snapshot ownership
gates are satisfied.
