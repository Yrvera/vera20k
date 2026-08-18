# Active-YR Crate Authority Design

**Date:** 2026-07-23
**Status:** Approved design
**Implementation status:** Not started
**Chosen approach:** Dedicated `CrateAuthority` with synchronous committed-cell-entry dispatch
**Parity status:** UNVERIFIED; runtime activation is blocked on the evidence and authority gates named below

## Goal

Implement the complete active Yuri's Revenge crate mechanism in Rust-native
architecture while preserving native crate-type indexing, slot and overlay
state, frame timing, scenario-RNG consumption, cell-entry ordering, effects,
drops, presentation, persistence, and same-tick consequences.

The system includes:

- `[CrateRules]`, `[Powerups]`, overlay, unit, and building rule parsing;
- initial multiplayer placement and per-slot regeneration;
- immediate pickup on every verified cell-arrival path;
- exact single-player and multiplayer selection behavior;
- all reachable active-YR effects, including weight-zero map-authored effects;
- verified inert handling for TS-legacy powerup slots;
- unit-death, building-removal, and trigger-created crate ingress;
- simulation-owned sound, animation, EVA, shroud, damage, spawn, and economy
  consequences;
- snapshot, world-hash, replay, and scenario-RNG integration; and
- native-derived executable validation.

This design does not accept a stock-only subset as completion. Weight-zero
powerups remain reachable through map-authored `OverlayData`, and crate drops
and trigger actions are active mechanisms. A missing prerequisite may delay
activation, but it does not remove the behavior from scope.

## Architecture Context

### Current Rust state

Crate gameplay is absent even though the default match option enables it:

- `src/sim/game_options.rs` parses and stores `GameOptions::crates`, defaults it
  to `true`, and includes it in the deterministic session state.
- `src/map/overlay_types.rs` parses `Crate=` into
  `OverlayTypeFlags::crate_type`, which is currently used only for rendering
  treatment. It does not parse `CrateTrigger=`.
- `src/sim/overlay_grid.rs` already owns mutable per-cell overlay ID and
  `OverlayData`, dirty-cell propagation, serialization, and hashing. It is the
  correct owner for the crate pixels and map bytes.
- `src/rules/ruleset.rs` has no crate-rules or fixed powerup table.
- `src/rules/object_type.rs` has no `CrateGoodie`, `CarriesCrate`,
  `CrateBeneath`, or `CrateBeneathIsMoney` state.
- `src/sim/scenario_session.rs` owns seed, options, map identity, launch slots,
  and the synthetic frame clock, but not the native game-mode discriminator or
  `Session::NumPlayers`. An app-local `SessionMode` is not simulation
  authority.
- `src/sim/movement/movement_tick.rs` processes a batch of movers while holding
  the entity store and returns aggregate statistics. Ground transition commits
  occur inside both the ordinary step and drive-track paths.
- `src/sim/movement/teleport_movement.rs`,
  `src/sim/movement/tunnel_movement.rs`, and other special movers commit
  relocation through separate APIs. None currently transfers control to a
  broad world-mutation authority immediately after a committed cell arrival.
- `src/sim/world/mod.rs` runs its current object-AI/movement/combat phases, then
  starts the authoritative factory sweep in Phase 7. The native crate
  regeneration rung belongs after the relevant object work and immediately
  before that first factory step, not after production or ore.
- `src/sim/snapshot.rs` is currently schema version 29. Crate state and the
  missing session discriminators are future-affecting state and require a
  coordinated version increment.

The renderer already reads the live overlay grid. A placed `CRATE` or `WCRATE`
overlay therefore has an existing render path; the gameplay design does not
need a standalone crate entity or render-owned crate state.

### Native ownership translated to Rust

Active `gamemd.exe` keeps a 256-entry crate-slot table on the map authority,
stores the visible crate in `CellClass` overlay bytes, invokes pickup from
per-cell movement processing, and consumes `ScenarioClass::Random`.

The Rust translation is:

```text
merged rules.ini/rulesmd.ini
        |
        +--> CrateRules
        `--> PowerupTable[19]

ScenarioSession --------------------+
  raw game mode / player count      |
  GameOptions::crates               |
  visible native frame              |
                                      v
OverlayGrid <-------------------- CrateAuthority
  overlay id/data                256 ordered slots
  dirty cells                   placement / regen / pickup
                                     |
CommittedCellEntry ------------------+
  ground / ship / teleport / verified special arrivals
                                     |
                                     v
Simulation-level effect adapters
  House / entity state / lifecycle / vision / triggers
  damage / spawning / scheduler / sound / world effects
```

`CrateAuthority` owns crate lifecycle state, but it does not own entities,
houses, movement, visibility, damage, or presentation. The `Simulation`
adapter supplies those authorities synchronously at the verified call points.
This follows the existing separation between entity storage and focused
simulation systems without copying the native C++ inheritance or global
singleton layout.

### Evidence precedence

The primary crate source is
`docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md`, especially its 2026-07-19
sections 9-11. Those sections re-verified the canonical names, live weights,
scenario RNG, selection loop, fixups, effect jump table, money formula,
placement, and regeneration cadence.

The following older claims are explicitly superseded or unresolved:

- The old 18-entry/type-index tables in early crate sections and
  `FUN_00481A00_CRATE_PICKUP_WARP_ARRIVAL_GHIDRA_REPORT.md` are not authority
  for type indices or weights. Only their independently verified caller/timing
  evidence may be reused.
- The old stock-weight total 100 is corrected to 110.
- The old `CrateRegen=1.3` example is corrected by stock INI and the current
  verification to `CrateRegen=3`.
- The old fixed `CrateRegen * 1800` period is corrected to the verified jittered
  range.
- The crate report's picked-slot descriptions originally conflicted. A
  2026-07-23 live Ghidra check resolved the mechanism: clear preserves the
  remaining duration numerically, writes sentinel coordinates and
  `start_frame=-1`, and the regen sweep skips that empty slot. Multiplayer
  performs only its immediate first-free-slot replacement.
- `VETERANCY_SYSTEM_GHIDRA_REPORT.md` contains stale crate index/magnitude
  wording. Its detailed eligibility branches may be used only after a focused
  reconciliation against the current crate dispatch.

Supporting primary sources include:

- `MAPCLASS_GHIDRA_REPORT.md` for the slot table and crate placement bounds;
- `pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
  for the crate FNPC argument row;
- `FRAME_BASIS_ONE_INCREMENT_ONE_LOGIC_STEP_GHIDRA_REPORT.md` and
  `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` for the
  native frame contract;
- `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` for overlay state and rendering
  implications;
- `RULESCLASS_GHIDRA_REPORT.md` and stock `ini/rules.ini` /
  `ini/rulesmd.ini` for rule loading and defaults; and
- effect-specific research documents, which must be selected and audited
  during the research-gate phase rather than inferred from powerup names.

## Impact Analysis

### Expected implementation surfaces

Rules and map metadata:

- New focused rule modules for `CrateRules`, `CrateType`, and the fixed
  19-entry `PowerupTable`.
- `src/rules/ruleset.rs` and `src/rules/mod.rs` for ownership and merged-INI
  construction.
- `src/rules/object_type.rs` for `CrateGoodie`, `CarriesCrate`,
  `CrateBeneath`, and `CrateBeneathIsMoney`.
- `src/map/overlay_types.rs` for `CrateTrigger=`.

Simulation authority:

- A new `src/sim/crates/` subsystem split by state, placement, pickup, and
  effects rather than one growing file.
- `src/sim/world/mod.rs` plus a focused world cell-entry adapter for
  bootstrap, immediate pickup, regeneration, drops, and effect integration.
- `src/sim/scenario_session.rs` and launch descriptors for the raw game-mode
  discriminator and authoritative multiplayer player count.
- `src/sim/find_nearby_cell.rs` only where its contract can reproduce the
  verified crate caller row.
- House, visibility, damage, lifecycle, trigger, production/superweapon, and
  presentation modules used by individual effect handlers.

Movement:

- `src/sim/movement/movement_tick.rs` and
  `src/sim/movement/movement_step.rs` for a resumable committed-cell barrier.
- Each separately implemented active arrival path, including
  `teleport_movement.rs` and every other movement family proven by the caller
  census.
- Movement tests that currently assume one opaque batch call.

Persistence and determinism:

- `src/sim/world/world_hash.rs`, `src/sim/snapshot.rs`, replay fixtures, and
  scenario-RNG tracing tests.
- Map initialization in `src/app_init.rs` after launch/session application and
  live overlay-grid construction.

### Dependency and behavioral risks

- **Borrow and control-flow risk:** pickup can mutate or delete the collector,
  iterate other entities, spawn objects, mutate houses, and consume RNG.
  Calling it while a locomotor retains `&mut GameEntity` is unsound
  architecturally and encourages deferred behavior.
- **Ordering risk:** one mover may cross more than one cell in a native frame.
  Pickup must occur after each committed transition and before any later
  movement or RNG operation, not after the mover or movement phase finishes.
- **Scheduler risk:** Unit and animation effects can create live objects.
  Their insertion and same-pass eligibility must use the authoritative object
  lifecycle/scheduler mechanism rather than `EntityStore` iteration alone.
- **Clock risk:** Rust commits `binary_frame` late, which is the correct
  placement shape, but still derives it as a synthetic 15 Hz clock. Static
  evidence proves native timers count `g_CurrentFrameCounter` increments; it
  does not prove the current wall-rate derivation is equivalent in every mode.
- **Effect-authority risk:** current health, modifier, cloak, veterancy,
  superweapon, shroud, trigger, and object-order authorities are incomplete or
  under active migration. A crate handler must not create a second truth.
- **Snapshot risk:** slot state, raw session mode/count, and any new entity or
  house state alter bincode layout and lockstep hash inputs.
- **Map-state risk:** slot coordinates and overlay bytes form one invariant.
  Repairing a mismatch by silently placing, clearing, or rolling RNG would
  create nondeterministic recovery behavior.
- **Mod-compatibility risk:** clamping negative weights, normalizing unknown
  values, or using file order may look defensive but diverges from the native
  parser and branch behavior.
- **Concurrent-work risk:** mission, entity-state, snapshot, movement, and
  world files contain a broad uncommitted authority migration at design time.
  Implementation must re-read ownership and preserve that work rather than
  applying this design against stale line numbers.

### Migration boundary

Crate parsing and dormant state scaffolding may be implemented incrementally,
but the live `GameOptions::crates` behavior must not flip until:

1. every blocking research question is closed;
2. every active effect and ingress path has its required authority;
3. the immediate committed-cell seam covers the verified caller set;
4. snapshot/hash/replay changes land in one coordinated migration; and
5. native-derived execution checks exist for RNG and ordering.

There must be no release state that claims crate parity while silently omitting
map-authored effects, drops, triggers, or exact timing.

## Chosen Approach

Use a dedicated serialized and hashed `CrateAuthority` plus a synchronous
committed-cell-entry barrier owned by the Simulation host.

The movement driver advances one object only until the next externally visible
barrier. When a cell transition commits, it returns control to `Simulation`.
The host releases the entity borrow, runs native-ordered per-cell actions,
including crate pickup, and then reacquires the object if it still exists and
is eligible to continue. The same object may yield multiple times in one
native movement call.

This approach is selected because it:

- gives the 256 slots and regeneration scan one explicit owner;
- keeps visible crate bytes in the existing `OverlayGrid`;
- preserves same-call-stack pickup and scenario-RNG order;
- permits effects to use broad world authorities safely;
- gives drive, walk, ship, teleport, and other verified arrivals one semantic
  integration point without injecting crate services into locomotors;
- survives the current migration toward per-object native scheduling; and
- avoids an event queue whose later drain would be known DRIFT.

## Tiny-Detail Ledger

### Activation, rules, and canonical identity

- Stock YR enables `Crates=yes`; it gates multiplayer regeneration and
  immediate replacement. The Rust default is already `true`, so ignoring the
  mechanism is ordinary-skirmish drift. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.6, 10; ini: rules.ini
  MultiplayerDialogSettings Crates=]`
- Native single-player is raw game mode 0; multiplayer-only branches must use
  authoritative session mode rather than infer mode from house count, lobby
  UI, or map name. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.6, 9.6]`
- Initial random count is
  `clamp(max(CrateMinimum, Session::NumPlayers), CrateMaximum)`.
  `Session::NumPlayers` is an explicit runtime value, not the current number
  of Rust houses. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 8.3, 9.6]`
- The canonical indices are exactly: Money 0, Unit 1, HealBase 2, Cloak 3,
  Explosion 4, Napalm 5, Squad 6, Darkness 7, Reveal 8, Armor 9, Speed 10,
  Firepower 11, ICBM 12, Invulnerability 13, Veteran 14, IonStorm 15, Gas 16,
  Tiberium 17, Pod 18. File order is irrelevant. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.1]`
- Stock merged-INI weights in that order are
  `[20,20,10,0,0,0,0,0,10,10,10,10,0,0,20,0,0,0,0]`, sum 110. Neither the
  compile-time sum 144 nor the stale total 100 may be hardcoded. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.2; ini: rulesmd.ini Powerups]`
- `[Powerups]` is four-field data: weight, animation, over-water byte, and
  native numeric magnitude. `<none>`, omitted fields, percent syntax, signed
  parsing, and malformed-value behavior must match the verified Rules loader;
  convenient Rust normalization is not assumed. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.1-9.2; UNKNOWN - parser edge cases
  need focused RulesClass verification]`
- Stock crate rules are `CrateMaximum=255`, `CrateMinimum=1`,
  `CrateRadius=3.0`, `CrateRegen=3`, `SilverCrate=HealBase`,
  `SoloCrateMoney=5000`, `UnitCrateType=none`, `WoodCrate=Money`,
  `WaterCrate=Money`, `CrateImg=CRATE`, `WoodCrateImg=CRATE`,
  `WaterCrateImg=WCRATE`, and `FreeMCV=yes`. `[ini: rules.ini CrateRules]`
- `Crate=`, `CrateTrigger=`, `CrateGoodie=`, `CarriesCrate=`,
  `CrateBeneath=`, and `CrateBeneathIsMoney=` are independent native flags.
  None may be derived from names or categories. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 2.2-2.4, 8.1-8.2, 10.1]`

### Slot and overlay state

- The native table has exactly 256 ordered 16-byte slots. Runtime scans are
  ascending and live; Rust must not replace this with unordered storage or
  silently resize it for the 30-player target. `[doc:
  MAPCLASS_GHIDRA_REPORT.md Crate Slot Table; doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.6]`
- The slot contains frame/timer state and raw cell coordinates with a sentinel.
  The active meaning of the fourth word and exact initialization writes must be
  verified before finalizing the Rust representation. `[doc:
  MAPCLASS_GHIDRA_REPORT.md CrateSlot structure; UNKNOWN - fourth word needs
  focused verification]`
- Pickup clear preserves the remaining duration, writes sentinel coordinates
  and `start_frame=-1`, and becomes ineligible for the regen scan while empty.
  Multiplayer then calls first-free random placement once; the cleared slot or
  an earlier free slot may be reused. There is no delayed second replacement.
  `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.6-3.7 and verification
  ledger; live Ghidra 2026-07-23: 0x004a1750, 0x0056c020, 0x0056bbe0,
  0x0056bd40]`
- Overlay ID and raw `OverlayData` remain the canonical visible/map bytes.
  Slot state must not replace or normalize those bytes. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 2.2, 9.4; Rust:
  src/sim/overlay_grid.rs]`
- `OverlayData < 19` selects that exact type without a weighted-selection RNG
  draw. Any other byte takes the random path. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.4]`
- The bootstrap relationship between map-authored crate overlays and runtime
  slots is not yet closed. The implementation may not assume that every
  authored crate consumes a random-regeneration slot or that none does.
  `[UNKNOWN - blocking map/bootstrap RE]`
- Native removal scans the slot array for matching coordinates. If duplicates
  or inconsistencies are possible, first-match behavior and later scan effects
  must be preserved; a coordinate hash map must not change that result.
  `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md section 3.7; exact duplicate behavior
  UNKNOWN]`

### RNG and numeric behavior

- Every crate random operation consumes the synchronized
  `ScenarioClass::Random` stream. No crate-local, main, mapgen, cosmetic, or
  seeded helper RNG is permitted. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.3]`
- Random selection draws inclusively from `1..=sum` and picks the first
  canonical entry whose cumulative weight is at least the roll. Boundary rolls
  1, 20, 21, and 110 are load-bearing. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.3]`
- A random type roll is already consumed before multiplayer guard downgrades;
  remapping to Money must not refund or replace that draw. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.3-9.4]`
- Random placement makes X then Y inclusive ranged draws on every attempt, for
  up to 1000 attempts. Failed attempts consume both draws. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.6, 10.4]`
- Only a successful placement consumes the additional jitter draw
  `RandomRanged(0, 0x7ffffffe)`. FNPC itself must not accidentally consume
  scenario RNG for this caller. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 8.4, 9.6; doc:
  pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md]`
- Timer conversion follows the verified x87 expression
  `CrateRegen * (1800 - rand01 * 1350)` and native `ftol` behavior. Stock
  `CrateRegen=3` yields 1350..5400 frame values. A fixed upper period, average,
  seconds conversion, or unproved floating/fixed approximation is DRIFT.
  `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 8.4, 9.6]`
- Simulation code may not use `f32`/`f64`. The replacement must be an
  exhaustive or interval-proven integer/fixed/native-bit formulation over the
  full relevant RNG domain, including conversion boundaries. `[project:
  AGENTS.md Simulation math; doc: CRATE_SYSTEM_GHIDRA_REPORT.md section 8.4]`
- Multiplayer Money consumes
  `RandomRanged(data, data + 900)` inclusively. Single-player Money grants
  `SoloCrateMoney` and consumes no amount roll. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.5]`
- Tiberium, Explosion, Unit, and every other random effect must preserve exact
  draw count, range, retry behavior, and interleaving with spawned-object
  construction. Existing high-level summaries are not sufficient contracts
  for all handlers. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.3, 9.5,
  10.4; UNKNOWN - effect-specific contracts required]`

### Placement and timing

- Random placement uses the native MapClass playfield frame: left and top are
  1; width and height are `SizeW + SizeH - 1`. This is not automatically the
  Rust `LocalSize`, canonical array extent, or FNPC diamond lens. One stock-map
  coordinate fixture must prove the translation. `[doc:
  MAPCLASS_GHIDRA_REPORT.md Playfield Bounds; doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.6]`
- A water source cell selects SpeedType 5 and `WaterCrateImg`; other cells
  select SpeedType 1 and the wood/normal crate image. The exact image choice
  per entry path remains rule-driven. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.6]`
- The crate FNPC call uses zone `-1`, MovementZone 0, required land/height `-1`,
  bridge-aware false, 1x1, reject-overlay false, height check 0, occupancy
  safety 0, allow bridge 1, and final occupancy 0. The source speed type is 5
  on water and 1 otherwise. `[doc:
  pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md
  map/start/crate/wall row]`
- Final placement separately requires the verified overlay/cell conditions.
  The exact relationship between FNPC's accepted result, existing overlay
  rejection, and crate overlay write must be preserved. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.3, 9.6; exact final predicate needs
  focused audit]`
- Pickup is synchronous at the committed cell-arrival point. It is not a
  movement-phase tail operation. Multiple crossings in one native movement
  call must observe removal, replacement, effects, and RNG before the next
  crossing. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 5, 10.3]`
- Teleport arrival invokes the same pickup mechanism after the arrival
  placement at its verified call point. Older type/signature claims in the
  warp report are stale and must not leak into the handler. `[doc:
  FUN_00481A00_CRATE_PICKUP_WARP_ARRIVAL_GHIDRA_REPORT.md caller evidence;
  doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 5, 9]`
- Regeneration runs on the native per-tick rung after object AI and before
  Factory/House AI, scanning slots in ascending live order. In current Rust it
  belongs immediately before the first authoritative factory step, after all
  preceding object-equivalent work. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.6, 10.3; doc:
  MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md]`
- Crate timers count visible native frame-counter increments and observe the
  pre-increment value during the update. They must not count 45 Hz Rust ticks
  or milliseconds. The current late-commit placement is useful, but the
  synthetic 15 Hz derivation is not yet proven as the native frame authority
  in every target mode. `[doc:
  FRAME_BASIS_ONE_INCREMENT_ONE_LOGIC_STEP_GHIDRA_REPORT.md; doc:
  MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md; Rust:
  src/sim/scenario_session.rs]`

### Pickup gates, remaps, and core order

- Exact picker eligibility, observer/civilian exclusion, and receiver family
  must be re-verified. Current reports disagree between a Civilian-house test
  and observer/spectator HouseType wording. `[UNKNOWN - blocking pickup-gate
  RE]`
- `CrateTrigger` fires cell action `0x31`, but its order relative to type
  selection, overlay removal, replacement, effects, and presentation requires
  a verified caller transaction. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 3.4; UNKNOWN - trigger-order RE]`
- In multiplayer, Unit downgrades to Money when credits are greater than 50;
  Cloak when the picker is already cloaked; Armor, Speed, or Firepower when
  the corresponding multiplier is not exactly native 1.0; and Veteran when
  already elite. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md section 9.4]`
- A water cell forces Money unless the selected powerup's third-field
  over-water byte is set. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.4]`
- FreeMCV forces Unit only when the house has no factory, credits are greater
  than 1500, owns zero MCVs, and `FreeMCV` is enabled. Each predicate must use
  its native authority rather than a convenient category count. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 9.4]`
- Squad index 6 unconditionally remaps to Money after the verified
  remove/replace portion. Pod index 18 skips the effect switch. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.4, 9.8]`
- Invulnerability 13 and IonStorm 15 use the shared animation-only tail.
  Squad, Invulnerability, IonStorm, and Pod must not gain invented gameplay
  effects. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.5, 9.8]`
- The verified core pickup order performs overlay removal and immediate
  multiplayer replacement before the chosen effect. Exact slot writes,
  trigger position, animation, sound, and EVA sub-order remain gates; the
  implementation cannot rearrange known stages for convenience. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.4, 9.4, 10.3]`
- An effect may kill, delete, displace, or otherwise invalidate the collector.
  Movement must reacquire by stable ID after the transaction and stop or resume
  from the resulting authoritative state. `[mechanism consequence of
  synchronous pickup; effect-specific confirmation required]`

### Effects, iteration, and presentation

- All 19 canonical slots require explicit dispatch behavior. Weight-zero
  Cloak, Explosion, Napalm, Darkness, ICBM, Gas, and Tiberium remain reachable
  from authored `OverlayData`. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.5, 9.7-9.8]`
- Radius effects use strict 3-D distance `< CrateRadius` with native
  square-root/`ftol` behavior and native active-object iteration order.
  Cell-only, 2-D, `<=`, BTreeMap-order, or squared-distance substitutions
  require positive equivalence proof. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.5, 10.5; doc:
  VETERANCY_SYSTEM_GHIDRA_REPORT.md eligibility details, pending reconciliation]`
- Armor, Speed, and Firepower multiply exact per-instance native values rather
  than mutate type data or approximate bonuses. The exact entity-state owner
  must be authoritative before activation. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.4-9.5]`
- Veterancy promotion must preserve exact candidate gates, tier transitions,
  iteration order, presentation, and already-elite behavior. The stale crate
  index/magnitude prose in the current veterancy report must first be
  corrected. `[doc: VETERANCY_SYSTEM_GHIDRA_REPORT.md; doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 9.1, 9.4-9.5]`
- Unit spawning must preserve `CrateGoodie` filtering, fixed
  `UnitCrateType`, FreeMCV selection, retry/RNG behavior, ownership, facing,
  reveal/unlimbo, occupancy, scheduler registration, and same-pass lifecycle.
  `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.5, 9.4, 10.4; UNKNOWN -
  complete Unit handler contract required]`
- HealBase, Cloak, Darkness, Reveal, ICBM, Explosion, Napalm, Gas, and
  Tiberium must call their real simulation authorities with their exact
  branch/write order. High-level effect names are not implementation
  contracts. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md section 9.5; UNKNOWN -
  per-handler audits/contracts required]`
- Animation type, sound, EVA speech, creation coordinates, start frame,
  palette/layer, and relative order are gameplay-visible outputs. They remain
  INI/asset-driven and need handler-specific evidence; missing animation
  `<none>` is not replaced with a default. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.5, 4.2, 9.5; UNKNOWN -
  presentation-tail audit required]`
- Spawned entities and animations must enter the authoritative live scheduler
  in native order. A snapshot of IDs taken before pickup may suppress native
  same-pass behavior and therefore cannot be assumed equivalent. `[doc:
  SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md; handler-specific
  activation needs verification]`

### Drop and trigger ingress

- `CarriesCrate=yes` unit death and
  `CrateBeneath=yes` building removal are active ingress paths, not optional
  polish. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md sections 8.1-8.2]`
- `CrateBeneathIsMoney=yes` writes predetermined Money data; the random form
  writes a raw value that reaches the random path. Exact values and placement
  timing belong to the building un-place transaction. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 8.1]`
- `CarriesCrate` uses its verified nearby-passable search path on death. The
  two scenario flag gates at the land/water branches are live mechanisms, but
  their stock startup values are not yet verified. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md section 8.2; UNKNOWN - blocking flag-default
  audit]`
- Trigger action `0x6C` drops a crate at a waypoint and must enter through the
  same placement/overlay authority without fabricating a timer or slot policy.
  The current trigger runtime lacks this action. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 8.1, 10.6]`
- External cell action `0x31` from `CrateTrigger` requires synchronous trigger
  mutation. A deferred trigger queue is acceptable only if direct binary
  evidence proves the same visibility and order. `[doc:
  CRATE_SYSTEM_GHIDRA_REPORT.md sections 3.4, 10.6]`

### Persistence and failure edges

- Slots, raw session mode/count, overlay bytes, future-affecting effect state,
  and the scenario RNG cursor are serialized and folded into the Rust world
  hash in a documented fixed order. `[Rust: src/sim/snapshot.rs,
  src/sim/world/world_hash.rs; project: AGENTS.md]`
- Old snapshots lacking crate authority are rejected through the version
  boundary; they are not loaded with default empty slots or inferred mode.
  `[Rust: src/sim/snapshot.rs current version policy]`
- Placement exhaustion after 1000 attempts is a deterministic no-placement
  result with all attempted RNG draws retained, not an exception or retry on
  the next helper call. `[doc: CRATE_SYSTEM_GHIDRA_REPORT.md section 9.6]`
- Zero/negative total weight, malformed rules, missing overlay IDs, and
  inconsistent authored slot state require verified native behavior. Until
  that is known, activation is blocked rather than silently clamped, repaired,
  or rerolled. `[UNKNOWN - malformed/modded-input RE]`
- Rust-to-Rust hashes and hand-computed fixtures are regression ratchets only.
  A parity claim requires a named gamemd/retail-derived executable check or an
  exhaustive proof over the relevant input domain. `[project: AGENTS.md Parity
  certification and status]`

## Design

### Components

#### Crate rules and powerup table

The rules layer owns:

- `CrateType`, a canonical raw-index vocabulary for the 19 verified entries;
- `PowerupEntry`, carrying native-width weight, optional animation identity,
  the exact over-water byte, and exact numeric magnitude representation;
- `PowerupTable`, a fixed 19-entry array populated by the internal canonical
  name list, never INI iteration order; and
- `CrateRules`, carrying all verified crate-rule values and asset references.

Raw `OverlayData` remains `u8` until the pickup branch decides whether it is a
predetermined type. Unknown or out-of-range bytes are not normalized into an
enum.

Weights remain native signed-width values unless the RulesClass parser audit
positively proves a different interpretation. Native double inputs use the
project's native-bit/fixed-point support; gameplay does not perform ordinary
floating-point simulation math.

Base `rules.ini` values are loaded first and `rulesmd.ini` patches them through
the existing merged-INI path. Missing-field and malformed-field behavior must
come from RulesClass evidence, not a newly invented strict parser policy.

#### CrateAuthority

`CrateAuthority` is embedded in `Simulation` and contains exactly 256 ordered
`CrateSlot` values. It has no RNG and no copied overlay grid.

Each slot will preserve:

- the native cell-coordinate sentinel and coordinate widths;
- timer start and duration with native signed/wrapping semantics;
- any verified fourth-word state; and
- every distinct state required by placement, pickup clearing, pause, expiry,
  and load.

The final field representation is intentionally blocked on the slot audit. An
`Option<Cell>` plus countdown is not accepted merely because it is idiomatic;
it is accepted only if it preserves every verified native state and transition.

Slot scans use the fixed array directly in ascending order. A cell-to-slot
cache is unnecessary and could change duplicate/first-match behavior. If later
profiling justifies a cache, it is derived, non-serialized, non-hashed, and
must not decide behavior.

`OverlayGrid` remains the crate's cell-byte authority. The invariant is that an
occupied runtime slot refers to the exact crate overlay/data expected by the
native state. Map-authored crates follow the separately verified bootstrap
policy rather than being forced into this invariant prematurely.

#### Session crate inputs

`ScenarioDescriptor` and `ScenarioSession` gain the raw native game-mode value
and authoritative `Session::NumPlayers` value supplied by the accepted launch
flow before tick 0. They are serialized and hashed.

The raw mode is retained even when convenience queries such as
`is_single_player()` are exposed. This prevents an enum fallback from
collapsing an unknown native mode into skirmish or multiplayer behavior.

Player count is not recomputed from:

- `HouseState` length;
- active/non-defeated houses;
- start-waypoint count;
- UI row count; or
- `GameOptions::ai_players`.

The existing app-local `SessionMode` may be refactored to convert from this
shared raw value, but `sim/` does not depend on the app module.

#### Committed-cell movement barrier

Movement exposes a control result meaning:

> Entity N has committed its position and occupancy from cell A to cell B, and
> no later cell transition or post-arrival action has executed yet.

The world driver:

1. advances the current mover until this barrier or completion;
2. drops every mutable entity borrow;
3. invokes `Simulation::process_committed_cell_entry`;
4. checks whether the entity remains live, on-map, and eligible;
5. resumes from its persisted locomotor state when appropriate; and
6. preserves the native live-object traversal semantics.

This is a control transfer, not a queued event. The barrier itself is ephemeral
and is neither serialized nor hashed.

Ground movement, drive-track movement, ship movement, teleport arrival, and
every other verified active caller adapt to this one contract. A locomotor does
not receive crate rules, houses, fog, damage, or presentation services.

The existing `pathfinding::cell_entry` module remains the pre-entry
passability/occupancy decision owner. The new committed-cell authority is
post-commit and should use a distinct name to avoid conflating the two stages.

#### World cell-entry authority

A focused world module owns `process_committed_cell_entry`. It invokes
native-ordered per-cell mechanisms synchronously. Crates are one consumer; gate
contacts, radio cleanup, triggers, and future verified per-cell operations may
share the authority only after their relative order is proven.

The module does not expose a generic unordered callback registry. Native order
is explicit code and tests. An unresolved relative order is a research gate,
not a plugin-order default.

#### Placement service

The crate placement service:

- locates the first native-free slot in ascending order;
- performs at most 1000 X/Y scenario-RNG attempts;
- translates the native MapClass random rectangle into the Rust coordinate
  frame;
- applies the water/land speed and asset branch;
- invokes the exact crate FNPC contract;
- checks the verified final overlay/cell predicate;
- writes overlay ID and `OverlayData`;
- initializes the selected slot; and
- consumes and converts the jitter draw only after success.

The service accepts a raw content byte so random crates, Money-forced building
drops, and authored/trigger variants preserve their distinct data.

Random placement, regeneration replacement, immediate pickup replacement, and
explicit drop ingress call the same verified primitive only where native does.
They do not share a convenience wrapper that erases different slot or timer
semantics.

#### Pickup transaction

The Simulation-level pickup adapter performs one ordered transaction. Known
stages are:

1. resolve the committed destination cell and crate overlay;
2. apply the verified picker/session gate;
3. run the verified `CrateTrigger` stage;
4. choose predetermined or random canonical type;
5. apply mode, water, anti-stack, and FreeMCV fixups in binary order;
6. perform the exact slot and overlay removal;
7. perform immediate multiplayer replacement when enabled;
8. run the selected gameplay handler or verified inert path;
9. emit animation, sound, EVA, dirty/radar, and other presentation effects in
   their verified relative order; and
10. return control to the movement host without assuming the collector
    survived.

Only the bold ordering constraints already verified by the binary may be fixed
before the remaining audit. In particular, the trigger and presentation
positions above are placeholders to be replaced by the verified ordering, not
claims that those numbered positions are already proven.

The transaction is synchronous but not artificially atomic. If native commits
an earlier write before a later branch exits or kills the collector, Rust keeps
that earlier write.

#### Effect executor

The crate subsystem owns the canonical 19-way dispatch, while each handler
uses the real state authority:

| Type family | Required authority |
|---|---|
| Money | House credit transaction and economy statistics |
| Unit / FreeMCV | Type filtering, lifecycle, occupancy, reveal, and live scheduler |
| HealBase | Signed health/repair authority and native owned-building iteration |
| Cloak | Canonical cloak state and reveal/conceal consequences |
| Darkness / Reveal | Player visibility, shroud, radar, and allied propagation as verified |
| Armor / Speed / Firepower | Exact per-instance modifier state |
| Veteran | Exact experience/rank and promotion authority |
| Explosion / Napalm / Gas | Damage, warhead, animation/particle, and lifecycle ordering |
| ICBM | Verified one-shot superweapon/launch authority |
| Tiberium | Overlay/ore placement and scenario-RNG ordering |
| Squad / Invulnerability / IonStorm / Pod | Explicit verified remap, animation-only, or skip behavior |

The crate module does not implement substitute health, cloak, modifier,
veterancy, superweapon, or trigger fields. If the real authority is absent,
that handler and therefore the complete crate activation remain blocked.

Radius handlers consume an authoritative live-object-order view rather than
iterating `EntityStore` as a proxy. House-owned scans and spawned-object
insertion likewise preserve their verified scheduler order.

#### Drop and trigger adapters

Crate creation has explicit entry adapters for:

- unit-death `CarriesCrate`;
- building un-place `CrateBeneath` and
  `CrateBeneathIsMoney`;
- trigger action `0x6C` at a waypoint; and
- any additional verified scenario/map bootstrap entry.

These adapters run at their owning lifecycle or trigger transaction point.
They pass the exact raw content and placement mode into `CrateAuthority`; they
do not manufacture a player pickup or invoke random multiplayer placement
unless the native path does so.

#### Presentation adapter

Crate gameplay remains entirely under `sim/`. It emits simulation-owned:

- sound events;
- world-effect/animation requests;
- EVA/voice events;
- radar/visibility invalidation; and
- other verified presentation state.

Render and audio layers consume those events. No effect handler imports
`render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.

Animation and sound identities remain parsed rule references and resolved
retail assets. No stock animation name, frame count, sound, or visual offset is
hardcoded in the handler.

### Interfaces / Contracts

#### Rule-load contract

- Build all 19 entries by canonical internal name.
- Preserve native missing and malformed semantics after verification.
- Resolve referenced type/animation/overlay/sound identities through existing
  registries.
- Include all behavior-affecting parsed crate rules in the rules hash.
- Do not enable live crate startup if a required reference or parser behavior
  remains unresolved.

#### Bootstrap contract

Crate bootstrap occurs only after:

1. the accepted launch has supplied raw mode, `Session::NumPlayers`, options,
   and scenario seed;
2. rules and all crate references are resolved;
3. the authoritative overlay grid contains the map-authored overlay bytes;
4. map-authored crate-to-slot behavior has run in its verified order; and
5. all initial placement RNG consumers that native runs earlier are complete.

It occurs before the first gameplay tick. It places the verified initial count
in native call order and leaves all failed-attempt RNG consumption intact.

#### Frame contract

`CrateAuthority` receives the current visible native frame. It does not read
`tick_ms`, `total_sim_ms`, `session.tick`, wall time, or render time.

Slot timers use native signed subtraction, sentinel, wrap, start, and remaining
semantics once the slot audit closes. Regeneration observes the pre-increment
frame for the current tick.

Correct late visibility is necessary but not sufficient: crate activation
waits until the native-frame source/rate contract is accepted for the target
session modes.

#### RNG contract

Every crate entry point receives a mutable reference to the one scenario RNG
owned by `Simulation`. No helper seeds, clones, snapshots, rewinds, forks, or
precomputes the stream.

Tests may wrap the RNG with an instrumentation layer that records:

- caller stage;
- inclusive bounds;
- returned value; and
- stream state before/after.

Instrumentation must not change the production algorithm.

#### Mutation contract

All broad mutations occur after the movement borrow is released. The
Simulation adapter supplies disjoint authorities or a narrowly scoped context;
it does not defer the work to solve borrowing.

An effect that requests lifecycle mutation uses the canonical lifecycle APIs
and scheduler. It does not directly erase `EntityStore` entries, rebuild
occupancy, or create IDs outside the normal authority.

#### Persistence contract

One coordinated snapshot-version increment includes:

- all authoritative slot words/states;
- raw session mode and `Session::NumPlayers`;
- any new entity/house rule state introduced by crate prerequisites; and
- any additional future-affecting trigger, presentation, or effect state.

No new authoritative field uses a serde default to fabricate an old snapshot.
Load rejects the old version under the existing policy. Rebuilt caches validate
slot/overlay invariants without consuming RNG or mutating gameplay state.

The world hash folds future-affecting values in a fixed documented order.
Overlay bytes and scenario RNG remain hashed by their existing owners; crate
state is not double-counted through derived caches.

### Data Flow

#### Match initialization

```text
accepted launch
  -> raw mode + Session::NumPlayers + options + seed
  -> merged crate rules and canonical PowerupTable
  -> map overlays become live OverlayGrid
  -> verified authored-crate/slot bootstrap
  -> initial-count calculation
  -> ordered PlaceCrate calls using Scenario RNG
  -> tick 0 begins
```

The exact authored-versus-random ordering is a blocking research item. The
implementation must not use this diagram to assume an answer.

#### Cell-entry pickup

```text
locomotor commits cell + occupancy
  -> returns CommittedCellEntry barrier
  -> entity borrow is released
  -> world per-cell authority reads destination cell
  -> crate pickup transaction runs synchronously
       selection/fixups
       removal
       immediate MP replacement
       handler
       presentation
  -> world reacquires stable entity ID
  -> same mover resumes or stops
```

No later mover, later cell crossing, factory, ore driver, or unrelated
scenario-RNG consumer can run between the native pickup stages.

#### Regeneration

At the native late scheduler rung:

1. test authoritative multiplayer mode and `GameOptions::crates`;
2. scan all 256 slots in ascending order against the visible frame;
3. for each due slot, perform exact clear/replacement semantics immediately;
4. allow mutations to affect the remainder of the same live scan exactly as
   native does; and
5. continue into the first Factory step, then House tail, with the advanced
   scenario RNG and updated overlays visible.

The scan does not first collect due indices into a vector unless exhaustive
proof shows that snapshotting is equivalent under slot reuse.

#### Death, building, and trigger drops

The owning lifecycle/trigger path calls the matching crate adapter before or
after its other writes exactly as verified. FNPC, raw content data, slot/timer
creation, overlay writes, dirty state, and failure behavior remain specific to
that ingress.

### Error Handling

Runtime crate transactions are deterministic state transitions, not
best-effort operations. Expected native failures such as placement exhaustion,
no eligible unit type, missing optional animation, or a guard downgrade are
represented as verified outcomes and preserve their RNG/state effects.

The system must not:

- catch a failed placement and reroll with different bounds;
- repair slot/overlay disagreement by consuming RNG;
- clamp a rule value without native evidence;
- substitute a stock asset or effect when a configured reference is absent;
- skip a handler and continue claiming parity; or
- partially enable crates because stock random weights do not reach the
  missing path.

Malformed mod behavior that lacks evidence is a research gate. During
development, live activation remains off rather than inventing semantics.
Once all gates close, ordinary stock runtime must not fail because an optional
animation is `<none>` or because 1000 placement attempts find no cell.

Snapshot version, map hash, rules hash, and malformed serialized state follow
the existing structured snapshot errors. An impossible crate-state invariant
detected at load returns an explicit load error; it does not silently reset
slots.

### Testing Strategy

#### Rule and identity tests

- Parse stock merged INI and assert the complete canonical index/name table.
- Assert the stock weight array and total 110.
- Assert every stock data magnitude, over-water byte, animation reference, and
  `<none>` case.
- Assert all `[CrateRules]` values and asset names.
- Assert base-plus-`rulesmd` override behavior and exact missing-key defaults.
- Assert all six per-type flags independently.
- Add malformed/signed/percent parser tests only after their native behavior is
  verified.

#### Slot and frame tests

- Constructor/clear/place/pause/expiry state matrix using distinct sentinel
  words.
- Ascending first-free and first-coordinate-match tests.
- Live-scan reuse fixture where a clear/place mutates a slot at or before the
  current scan index.
- Frame wrap, `-1` sentinel, exact due boundary, and same-frame start/check
  cases.
- A native-frame-source test proving crate code never reads Rust tick or
  milliseconds.
- Snapshot round-trip and old-version rejection for every slot word.

#### RNG tests

- Inclusive selection boundaries 1, 20, 21, and 110.
- Predetermined `OverlayData` proving no selection draw.
- Guard downgrade proving the selection draw remains consumed.
- Placement traces for first-attempt success, N failures then success, and all
  1000 failures.
- X-before-Y order and jitter-only-on-success.
- Multiplayer and single-player Money draw counts.
- Per-handler traces for all random branches and retry loops.
- Mathematical exhaustive/interval proof of the fixed/native-bit jitter
  conversion, with native captures at every conversion boundary class.

#### Movement and ordering tests

- One object crossing one crate cell.
- One fast object crossing multiple cells/crates in one native frame.
- Two objects contending for the same crate in live scheduler order.
- Teleport arrival and every other verified caller family.
- Pickup effect deleting/killing the collector before movement resumes.
- Immediate replacement visible to a later same-frame mover.
- Pickup RNG before later movement, combat, factory, and ore RNG consumers.
- Regeneration before Factory/House and after the relevant object work.
- No end-of-phase crate-event queue and no duplicate pickup after resume.

#### Placement and coordinate tests

- At least one retail map walked from `[Map] Size=` through native playfield X/Y
  draw, Rust coordinate translation, FNPC, and final overlay cell.
- Land and water speed/image branches.
- Exact FNPC argument fixture.
- Existing-overlay, boundary, invalid-cell, bridge, and no-passable-cell cases.
- Initial count across minimum, player-count, maximum, and zero/invalid
  boundaries after parser behavior is verified.

#### Effect tests

- A branch/write/RNG/presentation fixture for every one of the 19 canonical
  indices, including explicit no-gameplay assertions for inert entries.
- Map-authored tests for every weight-zero but active handler.
- Strict radius boundary just below, exactly at, and just above
  `CrateRadius`.
- Native live-object order with several eligible/ineligible objects.
- Exact state-bit or native-value assertions for cloak, modifiers, veterancy,
  health, shroud, damage, superweapon, and ore.
- Unit-spawn lifecycle, occupancy, ownership, reveal, scheduler insertion, and
  same-pass behavior.
- Sound, animation, EVA, and radar/dirty ordering fixtures from retail assets.

#### Drop and trigger tests

- `CarriesCrate` land and water deaths with every verified flag combination.
- Building `CrateBeneath` random and Money-forced raw data.
- Trigger action `0x6C` at valid, invalid, and missing waypoints.
- `CrateTrigger` action `0x31` with synchronous state mutation that affects a
  later pickup stage where native permits it.
- Drop failure preserving exact overlay, slot, lifecycle, and RNG state.

#### Persistence and parity evidence

- Per-field world-hash perturbation for every crate and session field.
- Snapshot round-trip with authored, random, paused, due, and empty slots.
- Deterministic replay equality with scripted placement and pickup.
- A scenario-RNG trace comparator against an instrumented active-YR run.
- Native-derived result captures for selection, jitter, replacement, each
  effect, and tick ordering.

Rust-vs-Rust tests remain regression evidence. Status may become `VERIFIED`
only when the named native checks or exhaustive proofs cover the claimed
mechanism and input domain.

### Research and authority gates before live activation

The implementation plan must begin by closing or consuming reviewed contracts
for:

1. the remaining unresolved slot fourth word and exact placement field
   writes (picked-slot clear/pause/regen semantics were closed on 2026-07-23);
2. map-authored crate-to-slot bootstrap;
3. raw picker observer/civilian gates and full cell-arrival caller census;
4. `CrateTrigger` transaction ordering and trigger action `0x31`;
5. `CarriesCrate` scenario flag defaults and land/water branches;
6. raw game-mode and `Session::NumPlayers` launch authority;
7. native frame-counter source/rate for the target session modes;
8. final random-placement predicate and map coordinate translation;
9. all active effect handlers and their presentation tails;
10. native active-object/house order and same-pass spawned-object behavior;
11. exact RulesClass parsing edges, including invalid weight totals; and
12. lifecycle, entity-state, trigger, superweapon, vision, and presentation
    authorities needed by those handlers.

An implementation plan may split these into research, substrate, and activation
waves. It may not mark the crate system complete after only the eight
positive-weight stock outcomes.

### Stop condition

The crate authority is complete only when:

- all twelve research/authority gates are closed;
- canonical parsing and every per-type flag are live;
- the fixed 256-slot state, authored bootstrap, initial placement,
  regeneration, and all ingress paths match verified behavior;
- every verified cell-arrival caller dispatches synchronously;
- all 19 type slots have their exact active, remapped, animation-only, or
  skipped result;
- scenario RNG count/order, native-frame timing, object iteration, lifecycle,
  presentation, persistence, and hashing pass their named checks;
- no crate-specific movement coupling or phase-end pickup queue exists;
- the broad pre-existing mission/movement work is preserved; and
- the parity status remains `UNVERIFIED` until native-derived certification
  evidence exists.

## Architectural Decisions

### Patterns followed

- **Rust-native owner, native semantics:** a focused authority replaces the
  native MapClass-owned array without copying MapClass inheritance or globals.
- **Existing overlay authority:** visible cell bytes stay in `OverlayGrid`.
- **Simulation-layer mutation:** gameplay effects stay below render, UI,
  sidebar, audio, and net.
- **Synchronous ordered transactions:** immediate control transfer preserves
  same-tick writes and RNG.
- **Authoritative lifecycle services:** spawned/deleted objects use the shared
  lifecycle and scheduler instead of crate-local storage.
- **Fixed deterministic storage:** the native 256-slot scan remains explicit.
- **Versioned state:** future-affecting fields are serialized and hashed once
  the live authority flips.
- **Evidence-ranked activation:** unresolved details block activation rather
  than becoming convenient defaults.

### Deliberate deviations from the current Rust shape

- The opaque batch movement API must become resumable at committed-cell
  barriers. This is a deliberate architectural change because the current API
  cannot express native same-call-stack per-cell effects safely.
- Simulation session data gains raw game mode and player count instead of
  relying on the current app-local mode helper.
- Crate regeneration gets an explicit native scheduler rung immediately before
  Factory rather than being grouped loosely with ore because both use scenario
  RNG.
- Exact powerup numeric state uses native-bit/fixed representations instead of
  ordinary floats, even though the binary computes parts of the mechanism in
  x87.

### Explicit technical debt and coordination

- The current synthetic frame-rate model remains a broader timing dependency.
  The crate subsystem will consume a native-frame interface but will not
  install local timing hacks.
- The current global live-object scheduler migration determines spawned-unit
  same-pass behavior. Crates will not add a private scheduler.
- Several effects depend on authority work outside the crate subsystem.
  Those dependencies are named gates rather than shadow fields.
- The current mission/movement worktree is broad and uncommitted. The later
  implementation plan must rebase its file inventory on the then-current
  architecture and coordinate the snapshot version.

No permanent parity drift is accepted by these debts; they delay the live
authority flip.

## Alternatives Considered

### Inject crate context into every locomotor

Pass crate rules, slots, overlays, houses, visibility, effects, lifecycle, and
scenario RNG directly through drive, walk, ship, teleport, and special
movement functions.

Rejected because it couples movement to economy, fog, combat, spawning,
triggers, and presentation; creates a large mutable-borrow surface; and still
requires a mechanism to release the mover borrow for broad effects. It can
preserve timing in principle, but it fights the existing architecture and
would likely be replaced by the generic per-cell authority.

### Queue cell entries and resolve them after movement

Have locomotors append ordered events and drain them at the end of Phase 1 or
Phase 2.

Rejected as known DRIFT. The crate and RNG remain visible too long, a fast
mover can cross subsequent cells before its first pickup, a second object can
observe stale state, teleport consequences occur late, and lethal effects
cannot stop the collector at the native point. No exhaustive equivalence proof
exists.

### Represent every crate as a GameEntity

Store crates in `EntityStore` and use ordinary entity collision/lifecycle.

Rejected because active `gamemd.exe` uses cell overlay bytes plus a separate
slot table, not a standalone crate object. A GameEntity would perturb stable
IDs, occupancy, live scheduler order, same-pass behavior, snapshots, hashes,
and deletion semantics.

### Use OverlayGrid alone with a timer map

Treat every crate overlay as its own timer and omit the native slot array.

Rejected because slot capacity, first-free order, live regeneration scans,
picked-slot behavior, and map-authored association are load-bearing native
mechanisms. A coordinate timer map has no proven byte/state equivalence and
would conceal the central unresolved slot contradiction.
