# Phase 3 Ship Bridge-Z Adjustment Design

**Date:** 2026-08-27

**Status:** REVISED / OPEN — critic 5 returned `BLOCK`; its timer-sentinel finding was corrected in research commit `37571c99` and passed fresh evidence critic 6. Awaiting fresh full-design critic 7. Stock-active Chronosphere and IsLocomotor release integration remains a named prerequisite blocker; no implementation-readiness or parity `PASS` is claimed.

**Scope:** Phase 3 GSI-04.03, the stock Ship locomotor destination, bridge-Z braking distance, destination-delay guard, and terminal stored-Z consumer.

**Evidence:** `docs/research/PHASE3_SHIP_BRIDGE_Z_ADJUSTMENT_GHIDRA_REPORT.md`, including Sections 12-16. Section 15 supersedes every earlier ordinary-rearm or no-stock-Ship claim; Section 16 supersedes the earlier timer-sentinel and constructor-raw-state claims.

## Goal

Make recognized Ship locomotion match the active retail mechanism:

- keep immutable `BRIDGE_Z_OFFSET = 416`;
- store the native Ship immediate destination XYZ;
- derive braking target Z from the immediate destination's structural cell and calculate native signed 3D distance;
- preserve strict slowdown comparison and fixed-point range;
- make every Ship cell destination install transactional, including direct scatter;
- preserve/retry terminal navigation unless cell and stored-Z arrival both pass;
- model the active Foot destination-delay predicate on the owner through the exact stock `SQD -> ParasitePlus -> admitted attachment -> victim Foot-AI-tail Paralyzes=32767` lifecycle;
- remove generic non-Ship water-mover bridge-Z copies;
- preserve Drive, Walk, flat-water, and `OnBridge` behavior.

Do not add a mutable bridge global or a second cached target-Z. The only new state is the native owner destination-delay timer; it is neither target Z nor global state.

## Architecture Fit

The smallest existing ownership seams are:

- `LocomotorKind::Ship` is recognized-class ownership.
- `ShipLocomotionRuntime.destination: Option<DriveCoord>` is the immediate native destination triple. Its X/Y lifetime already matches retarget/cancel; its cell-target Z is currently coarse and must be corrected.
- `drive_locomotion.rs` owns Ship speed fraction and strict `distance < slowdown_distance`.
- `navcom.rs` owns owner NavCom, Ship setter/getter/null, and terminal navigation state.
- `movement_commands.rs` owns layered/direct install entry points.
- `PathCell::has_structural_bridge()` is the `Cell.Flags & 0x100` projection. Walkability, bridgehead/ramp deck facts, and current-cell bridge state are not substitutes.
- `ResolvedTerrainGrid` owns exact signed terrain level/slope for setter construction; `PathGrid` owns the runtime structural projection used by braking.
- `util::native_x87::distance_3d_leptons` is the existing exact `CoordStruct::Distance3D @ 0x0041C380` port.
- `GameEntity` is the persistent Foot owner. A `CdTimer` there survives attack-target replacement and command mutation, matching `Foot+0x6A0/+0x6A8`.
- the existing projectile special-detonation dispatch supplies the source and object-target boundary for exact Parasite `CanAttach` admission. It is an attachment seam only: it must not arm the victim timer.
- the existing per-object scheduler bracket after locomotor Process and before `object_ai_post_movement_promote_one` is the architecture-correct Foot-AI tail for the first write and each refresh.
- central pointer-expiry/uninit and the concrete damage, repair, and Iron Curtain producers own exact detach calls. A resolved special target never substitutes for an actual reciprocal attachment.

The flow is:

```text
command/direct/scatter request
  -> read-only recognized-Ship preflight at current binary frame
  -> validate and construct exact destination XYZ
  -> one commit of NavCom + Ship destination + movement/facing

movement tick
  -> recognized Ship reads immediate destination
  -> destination structural cell + signed current/destination Z
  -> native distance_3d_leptons -> i32
  -> comparison-preserving I16F16 adapter
  -> existing strict slowdown transition

terminal track
  -> retire executed head
  -> target-cell equality AND abs(owner Z - stored destination Z) < 208
  -> clear, otherwise preserve destination/NavCom and defer repath

Parasite prerequisite
  -> detonation validates source manager + exact CanAttach gates
  -> successful transaction installs attacker.manager.victim_id <-> victim.parasite_attacker_id
  -> detonation leaves the victim raw timer pair unchanged; the ordinary zero-duration case stays inactive through the pre-victim-tail interval
  -> next victim Foot-AI tail resolves attacker's live slot-0 warhead
  -> water/missing-cell: re-anchor victim timer; known non-water: exact detach
  -> release/lifecycle producer -> one reciprocal detach-and-zero helper
```

## Impact Analysis

### Production and test files

| File | Planned responsibility |
|---|---|
| `src/rules/warhead_type.rs` | Parse and retain signed `Paralyzes=` as `i32`, default zero, and `Sonic=` as its own bool at native `+0x14B`. Do not alias it to the currently misdocumented neighboring field. Add omission, signed, 32767, and Sonic tests. |
| `src/rules/object_type.rs` | Parse `Organic=` default false and `Parasiteable=` for the bounded recognized-Ship UnitType path with the live UnitType default true. `naval` already exists. Keep non-Ship Parasite targets on the unsupported path rather than inventing other registry defaults. |
| `src/sim/game_entity.rs` | Add `parasite_manager: Option<ParasiteManagerState>`, victim `parasite_attacker_id: Option<u64>`, and `foot_destination_delay: CdTimer`. Defaults are no manager/no link; native Foot construction must explicitly install raw timer `start=current creation frame,duration=0` rather than use `CdTimer::default()`. |
| new `src/sim/parasite_attachment.rs`, plus `src/sim/mod.rs` | Own `ParasiteManagerState { victim_id }`, read-only `can_attach`, transactional `attach`, reciprocal `detach`, consistency validation, and one victim-tail update. It owns no animation/damage/placement approximation. |
| `src/sim/world/world_spawn.rs` | Construct manager presence exactly for non-building objects whose rookie primary weapon resolves to `Parasite=yes`; manager presence persists independently of whether it has a victim. |
| `src/sim/combat/mod.rs` | In the Parasite branch, require live source manager and `ProjectileTarget::Entity`, evaluate exact admission using entities/rules/interner/resolved terrain, then install links and return through the unsupported tail with the timer untouched. Wire Sonic and negative-damage detach before the Foot receiver mutates HP. |
| `src/sim/projectile.rs` | Carry no new mutable attachment state; preserve source ID, concrete entity target, and special-action priority. Tests pin that null/cell/dummy targets cannot become attachment authority. |
| `src/sim/world/mod.rs` | Call `tick_parasite_victim_tail_one` inside the live-object closure after mission/destination and all locomotor work, before `object_ai_post_movement_promote_one`; this gives a frame-N attach its first write on the victim's frame-N+1 visit. |
| `src/sim/world/lifecycle.rs` | Before pointer expiry breaks either endpoint, call the reciprocal detach helper. Validate loaded links after deserialization and reject malformed one-sided/duplicate/self links. |
| `src/sim/docking/building_dock.rs` | On an accepted funded service-repair mutation, detach an actually attached victim before applying heal. No insufficient-funds/no-heal clear. |
| `src/sim/superweapon/{iron_curtain,invulnerability}.rs` | On the stock non-Organic Iron Curtain receiver path, detach an actual Parasite link before invulnerability. Organic targets follow their existing damage branch and do not clear through this call. ForceShield/ordinary target selection alone is not detach authority. |
| `src/sim/movement/navcom.rs` | Own recognized-Ship preflight, exact Ship coordinate construction, getter/null behavior, transactional internal cell install, and the Ship terminal cell-plus-Z arrival predicate. Leave Drive coordinate construction unchanged. |
| `src/sim/movement/movement_commands.rs` | Preflight layered/direct/teleporter-facing entries before any mutation. Give direct moves resolved terrain; on recognized-Ship success install NavCom and exact Ship destination in the same transaction, including same-cell success. |
| `src/sim/movement/bump_crush.rs` | Pass resolved terrain through `scatter_blocker` to the direct recognized-Ship setter. |
| `src/sim/movement/movement_tick.rs` | Dispatch recognized Ship to the pure 3D helper; delete both generic water-mover bridge-Z additions; pass the exact current owner Z facts. |
| `src/sim/movement/drive_locomotion.rs` | Add signed coord-to-cell, exact Ship current/destination world-Z, native `i32` distance, and saturating boundary adapter beside existing Ship speed logic. |
| `src/sim/movement/movement_occupancy.rs`, `tube_movement.rs`, `src/sim/docking/bunker_install.rs` | Thread already-owned `ResolvedTerrainGrid` through active scatter/direct callers. |
| direct callers under `src/sim/combat`, `miner`, `passenger`, `production_sell`, and `world` | Pass optional terrain to the changed direct API; stock non-Ship behavior remains unchanged. |
| `src/sim/world/world_commands.rs` | Run the same read-only Ship preflight before Move/AttackMove/RepairAtDepot or defensive invalid-combination mutation. No command may erase the guard before it is observed. |
| `src/sim/snapshot.rs` | Bump 112 -> 113, reject 112, round-trip manager presence/victim link, victim back-link, raw timer, destination, and pending retry, and reject malformed reciprocal links. |
| `src/sim/world/world_hash.rs` | Hash manager presence/victim ID, victim attacker ID, and raw timer start/duration in stable entity order. Existing destination/navigation membership remains. |
| `src/sim/movement/movement_tests.rs`, `src/sim/world/world_tests.rs`, `src/sim/combat/combat_tests.rs`, focused parasite/snapshot tests | Add the acceptance matrix below. |
| `src/sim/movement/movement_bridge.rs` | Preserve `BRIDGE_Z_OFFSET=416`; correct provenance and delete stale generic planar-add expectations. |

`src/sim/components.rs`, pathfinding algorithms, bridge topology/traversal, `OnBridge`, occupancy layer selection, rendering, RNG, and `util::native_x87.rs` are unchanged authorities.

### Documentation corrections

| File | Correction |
|---|---|
| `docs/research/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` | Setter guards are IsCrashing, Foot destination-delay remaining, IsWarpingOut, IsBeingWarped. `0x004DE770` is not ordinary weapon rearm. |
| `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` | Same guard correction; `+0xC94=IsTrain`; structural destination, not current-cell/walkable ownership. |
| `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` | Correct stale `+0x380` setter name. |
| `docs/research/FOOTCLASS_VTABLE_COMPLETE.md` and any protocol doc calling `0x004DE770` IsInRearmTimer/IsFiring/IsInGarrison | Rename literally to Foot destination-delay/paralysis remaining predicate. |
| docs that name `0x006297F0` as a Temporal update or `0x00719400` as Teleport InitiateWarp | Correct the stale identities: `0x006297F0` is Naval+Organic Parasite/SQD update; `0x00719400` is a mid-body split in the `0x007192F0` Teleport state machine. True chrono-erase does not write/clear this timer. |
| `docs/research/BRIDGE_BSS_RUNTIME_CONSTANT_SWEEP_GHIDRA_REPORT.md` | Record fixed initialized 104/416 values and active writer/reader evidence. |

## Behavior Ledger

| Priority | Mechanism | Required behavior |
|---|---|---|
| MILESTONE-BLOCKING | Ownership | Only recognized Ship uses this bridge-Z calculation. Non-Ship water movers remain neutral 2D. |
| MILESTONE-BLOCKING | Destination authority | Active `ShipLocomotionRuntime.destination.x/y` decides the structural cell; disagreeing `final_goal/path` never overrides it. |
| MILESTONE-BLOCKING | Stored Z | Successful Ship cell install stores exact cell-center signed ground/slope Z +416 iff structural. |
| MILESTONE-BLOCKING | Braking | Current and target signed 3D coordinates feed native `distance_3d_leptons`; strict `<` owns slowdown. |
| MILESTONE-BLOCKING | Destination-delay guard | A positive owner Foot timer rejects recognized-Ship installs before all mutation. Ordinary weapon cooldown does not. |
| MILESTONE-BLOCKING | Attachment admission | Parasite detonation installs reciprocal links only after every native CanAttach gate; failure is a full no-op. It never arms the timer. |
| MILESTONE-BLOCKING | Active writer/order | A legal stock SQD attachment keeps the timer inactive until the victim's next Foot-AI tail, then writes/refreshes 32767 every qualifying tail after destination and locomotor work. |
| MILESTONE-BLOCKING | Detach | Only an actual reciprocal attachment may clear; water loss, active Sonic/heal/repair/IC, and endpoint lifecycle use one exact clear transaction. |
| MILESTONE-BLOCKING | Scatter | Stock `UnitClass::Scatter` equivalent reaches the exact recognized-Ship setter; stale/no-prior destination cases are correct. |
| MILESTONE-BLOCKING | Terminal arrival | Clear only for target-cell equality plus strict `abs(ownerZ-storedZ)<208`; otherwise preserve/retry. |
| MILESTONE-BLOCKING | Invalid geometry | Missing terrain, out-of-grid target, or unsupported slope rejects a new recognized-Ship install without mutation. |
| COMPOUNDING | Fixed range | Native distance remains `i32`; exact through 32767 and saturate from 32768 only at the `SimFixed` boundary. |
| SUPPORTING | Save/determinism | Manager presence, reciprocal links, timer, exact destination Z, and retry state round-trip/hash; malformed links and v112 reject under v113. |
| SUPPORTING | Exclusions | No Passive, IsTrain, selector>=0x40, mutable global, second target-Z, or ordinary-rearm substitution. |

## Chosen Design

### 1. Exact bounded Parasite attachment prerequisite

This prerequisite models only the persistent state and scheduler edges required to make the Ship setter's active stock guard exact. It does not model grapple animations, periodic damage, culling damage, attached-attacker placement, or generic Terror Drone attack cadence.

#### 1.1 Persistent representation and construction

Add these serde/hash-visible fields:

```text
ParasiteManagerState { victim_id: Option<u64> }
GameEntity.parasite_manager: Option<ParasiteManagerState> // attacker +0x69C
GameEntity.parasite_attacker_id: Option<u64>               // victim +0x694
GameEntity.foot_destination_delay: CdTimer                 // victim +0x6A0/+0x6A8
```

`parasite_manager` is present from object construction exactly for a non-building object whose rookie primary weapon resolves to a `Parasite=yes` warhead. It remains present with `victim_id=None` before attachment and after detach. Do not derive manager existence from the detonating warhead: a modded secondary Parasite projectile without a primary-created manager must fail as native does. Every new Foot owner explicitly starts `foot_destination_delay` as `CdTimer::started(current_creation_frame, 0)`, matching native constructor stores `0x004D33F6..0x004D3402`. Do not use `CdTimer::default()` / raw `(-1,0)`: it is predicate-inactive but byte/state-distinct once serialized and hashed.

Add `ObjectType.organic` (default false), `ObjectType.parasiteable` for UnitType/`Vehicle` with the live default true, `WarheadType.paralyzes: i32`, and the separate native `WarheadType.sonic` boolean. This bounded prerequisite handles only a recognized Ship victim; non-Ship Parasite targets retain the existing explicit unsupported result, so no unverified Infantry/Aircraft/Building default is invented. Existing `ObjectType.naval`, lifecycle, health, installed `BunkerLink`, type/veterancy weapon selection, and resolved terrain supply the other facts. `ImmuneToPoison`, houses, mission, and Verses are deliberately absent from admission because native does not read them.

#### 1.2 Exact transactional admission

`parasite_attachment::can_attach` takes immutable source/victim/type/terrain facts. It requires, in order:

1. source exists and has a manager;
2. target is `ProjectileTarget::Entity`, resolves, and owns recognized `LocomotorKind::Ship`; non-Ship targets remain explicitly unsupported by this bounded slice;
3. victim is not in limbo, `object_alive`, non-dying, and health is nonzero;
4. victim has no `parasite_attacker_id`;
5. victim type is `parasiteable`;
6. victim has no installed bunker link (`bunker_link.installed_in().is_none()`; an approach intent is not the native `+0x2E4` link);
7. when source type is Naval, victim has an in-grid resolved-terrain cell and that cell's canonical water-set fact is true.

The detonation branch reads all gates before mutation. Success commits `source.parasite_manager.victim_id=Some(victim_id)` and `victim.parasite_attacker_id=Some(source_id)` as one transaction, then keeps the existing unsupported tail for animation/damage behavior. It does **not** touch the timer. Failure preserves both entities, mission/order/navigation/attack state, and timer byte-for-byte. Two Squids resolving against one victim in order produce one winner; the second observes the installed back-link and cannot replace or refresh it.

#### 1.3 Victim-tail first write and refresh

Call `tick_parasite_victim_tail_one(sim, victim_id, rules, resolved_terrain)` after that object's mission/destination and locomotor work and before `object_ai_post_movement_promote_one`. The helper first validates both reciprocal IDs and manager presence. For the stock Naval+Organic manager owner it resolves that attacker's **current live slot-0 weapon** using existing type/veterancy selection, then the weapon's warhead and signed `paralyzes`.

- missing native cell analogue or a canonical water-set cell: `victim.foot_destination_delay.start(current_frame, paralyzes)`;
- known non-water cell: call the detach helper instead;
- non-Naval+Organic manager: leave its generic ROF/damage behavior unsupported and do not invent a timer write in this bounded slice.

This ordering is load-bearing. A frame-N projectile detonation installs links after the ordinary stock victim has already visited the live-object loop and leaves its raw timer pair unchanged. For the ordinary newly constructed/inactive victim, duration remains zero (normally raw `(creation_frame,0)`), so a Ship destination request in that interval is admitted. The first 32767 write occurs only at the victim's frame-N+1 tail, after its destination/locomotor work; every later qualifying tail re-anchors it. `SquidGrab ROF=99` is irrelevant to this path.

#### 1.4 Exact detach transaction and active callers

`detach_parasite(entities, attacker_id, victim_id, current_frame)` requires the reciprocal pair, then clears manager `victim_id`, clears victim `parasite_attacker_id`, and writes victim `CdTimer::started(current_frame, 0)`. It is idempotent only for an already-absent pair; it must not clear an unrelated timer or link from a broad target selection.

Wire currently reachable Rust producers at their native ordering point:

- known non-water victim at the Foot-AI tail;
- `Sonic=yes` or negative incoming damage in the Foot damage receiver, before HP mutation;
- an accepted/funded service-depot repair step, before its heal;
- non-Organic Iron Curtain application, before invulnerability;
- pointer expiry/uninit of attacker or victim, before generic references break.

Native also detaches an actual Naval attachment in Chronosphere selection and after the full IsLocomotor/`PerformDeploy` admission. Both are stock-active against eligible Ship victims while their Rust upstream effects are absent. Therefore this design and GSI row remain **OPEN** on two named prerequisites:

- Chronosphere must first own its native source-area object enumeration and eligibility order; only an enumerated object with an actual attacker link, Naval attacker type, and live attacker manager receives the manager-delay-500 then detach call before the later warp decisions.
- IsLocomotor must first reproduce Bullet's gates before `PerformDeploy`: non-null source; source current-target mismatch and no existing locomotor target; source not already locomotor-affected; target object/type validation; target not invulnerable or already locomotor-affected; Unit/Aircraft class; and incoming damage meeting the target-type threshold. Only then may `PerformDeploy` detach an actual Naval attacker link.

Until those exact admission surfaces are designed/implemented, neither `SpecialDetonationAction::Locomotor` nor a superweapon/chrono target may call `detach_parasite`. This is a blocker, not a harmless residual. Teleport primary locomotion and Grinder entry are evidence-excluded for stock recognized Ships. True chrono-erase does not write or clear this timer.

The destination-delay predicate is the existing raw `CdTimer::remaining(frame) != 0` behavior. Live `0x004DE770` loads duration before testing the sentinel; `start == -1` skips elapsed calculation and tests that raw duration directly:

```text
if start == -1:
    active = duration != 0
else:
    elapsed = current_frame.wrapping_sub(start)
    active = elapsed < duration
          && duration.wrapping_sub(elapsed) != 0
```

The running comparison is signed and exact expiry is inactive. Thus raw `(-1,0)` is inactive, while `(-1,32767)` and `(-1,-7)` are active. No command, attack retarget, or ordinary cooldown tick clears it. Only the exact refresh/detach writers above mutate it.

### 2. Transactional recognized-Ship preflight

Define one read-only predicate beside the Ship setter, taking `&GameEntity` and current signed binary frame. Non-Ship returns admitted immediately. Recognized Ship checks in native order:

1. `dying` (ordinary stock Ship sinking/crashing authority);
2. `foot_destination_delay.remaining(frame) != 0`;
3. `teleport_state.warp_out_active()`;
4. `teleport_state.warp_in_active()`.

Do not include `attack_target.cooldown_ticks`. A normally rearming Ship remains movable, and Walk/Drive behavior is untouched.

Call the predicate before any mutation in:

- high-level Move, AttackMove, RepairAtDepot and defensive movement command branches;
- `issue_move_command_with_layered`;
- `issue_direct_move`, including same-cell success;
- teleporter fallback;
- `navcom::set_destination_internal_cell`.

Rejection preserves mission/dispatch timer, order intent, complete attack target/cooldown/provenance, NavCom/aux/queue/pending retry, Ship destination/head/path/speed, movement target, facing, dock/C4/capture/passenger/bunker/radio state, and occupation.

### 3. Destination construction and transaction

Convert signed destination coordinates to cells exactly:

```rust
fn native_coord_to_cell(coord: i32) -> i32 {
    coord.wrapping_add((coord >> 31) & 0xff) >> 8
}
```

`-1..=-255 -> 0`, `-256 -> -1`, `255 -> 0`, `256 -> 1`. Checked conversion/bounds follows; no unsigned wrap.

For a cell target, construct the entire Ship coordinate before borrowing mutably:

- X/Y = cell center (`cell * 256 + 128`);
- Z = `ground_height_leptons(level, slope_type, X, Y)`;
- add 416 exactly once iff the resolved cell has structural bridge;
- walkable-only bridgeheads/ramps add zero.

For an already resolved entity/object coordinate, preserve incoming exact Z and add 416 iff its X/Y structural cell requires it. Current production does not install Ship entity NavTargets, so this is an adapter/control contract, not a new pursuit pipeline.

Missing terrain, invalid cell, or unsupported slope `>20` returns false before mutation. No zero-Z successful fallback and no dummy cell. Null/cancel remains the native clear path and does not install geometry.

On success commit NavCom, complete Ship destination, movement target/path, and facing together. Direct recognized-Ship movement uses this same transaction; non-Ship direct movement keeps its current API semantics. Same-cell recognized-Ship success must still run the exact setter transaction so stale/no-prior destination state cannot survive a claimed successful install.

### 4. Scatter ownership

Native `UnitClass::Scatter @ 0x00743A50` reaches `TechnoClass::Set_Destination -> FootClass::Set_Destination_Internal -> locomotor +0x44`; recognized Ship resolves to `0x0069F450`.

Thread already-owned `ResolvedTerrainGrid` through every active `scatter_blocker` and `issue_direct_move` caller. A recognized-Ship scatter:

- preflights before any old-state clear;
- validates exact adjacent target geometry;
- atomically replaces stale prior NavCom/destination or creates the first one;
- stores structural +416 where required;
- rejects missing/out-of-grid/slope-error without changing old state.

### 5. Current and destination world Z

Current X/Y:

```text
x = rx*256 + sub_x.to_num::<i32>()
y = ry*256 + sub_y.to_num::<i32>()
```

Current Z precedence:

1. `Position.exact_z_leptons`, already absolute;
2. otherwise exact signed terrain/slope height at current X/Y;
3. add 416 iff existing owner `on_bridge` is true.

`Level=0xFF`, slope zero yields -103 leptons by signed-level/native truncation convention. Do not use unsigned `Position.z * 104` when terrain is valid.

For braking, active `ship.destination.x/y` is the only structural cell authority. Recompute target ground at those exact X/Y from the destination `PathCell`, add 416 iff `has_structural_bridge()`, and ignore stored destination Z for braking. Stored Z remains authoritative for terminal arrival.

If `ship.destination=None`, preserve neutral `final_goal -> path.last() -> current` 2D behavior. If a pre-existing corrupt active destination lacks grid/terrain authority, the tick helper remains total: use the same immediate X/Y with neutral planar Z and never borrow structure from final goal. Successful production installs cannot create this state.

### 6. Native distance and fixed boundary

Call `distance_3d_leptons([current_x,current_y,current_z], [dest_x,dest_y,target_z])` and retain the nonnegative `i32`.

At the existing `SimFixed=I16F16` comparison boundary:

- `0..=32767`: exact integer conversion;
- `32768..=i32::MAX`: `SimFixed::MAX`;
- never panic, wrap, or clamp the slowdown threshold.

This preserves native `d < s` for every representable nonnegative slowdown:

- `d=32767, s=32767`: equality false;
- `d=32767, s>32767` such as `SimFixed::MAX`: true;
- `d=32768` saturates MAX; against MAX equality false;
- every longer distance is likewise non-braking for any representable threshold.

Required approximation discriminator: native `[0,0,0] -> [129,0,0]` returns 128; slowdown 129 brakes, slowdown 128 does not. Do not use integer sqrt, host sqrt, or planar+416.

### 7. Terminal stored-Z arrival

At track terminal:

1. retire executed head/selector;
2. require owner NavCom;
3. derive target cell from NavCom X/Y with signed native conversion;
4. require owner current cell equality;
5. derive exact owner current world Z;
6. `delta = owner_z.wrapping_sub(stored_destination.z)`;
7. native wrapping abs and strict `< 2 * 104 = 208`;
8. clear destination/NavCom only on pass.

On failure preserve NavCom and stored destination, set/generalize `pending_arrival_clear`, and let the existing next-process deferred seam rebuild from the cell target. Do not call the current null/stop helper before a successful refreshed setter. A missing or non-cell NavCom cannot prove arrival and preserves/retries defensively.

### 8. Scheduler integration and duplicate removal

Keep movement tick order. Drive uses its existing 2D distance. Recognized Ship uses the helper and existing Ship speed update. Other locomotors retain generic behavior.

Delete both `movement_zone.is_water_mover()` plus current-cell bridge-Z blocks. Movement zone and current bridge walkability are not Ship ownership or destination structure.

### 9. Evidence-backed exclusions

- Positive forced selectors `0x43..0x47` require the land Tank Bunker reciprocal link; no stock Ship reaches selector `>=0x40`. The other relevant callers pass `-1`.
- `Passive` is stock false for all Ship-CLSID types.
- the second global reader's `TechnoType+0xC94` gate is `IsTrain`; no stock Ship enables it.
- ordinary weapon rearm at Techno `+0x2EC/+0x2F4` is not setter guard 2.
- Terror Drone cannot target water and its `3*ROF` post-detach value is not a Ship-owner path.
- true chrono-erase/Chrono Legionnaire does not read or write this timer at all; any prior stock-zero claim was a stale-function-label error.
- Magnetron `IsLocomotor` and Chronosphere are stock-active detach callers for an **actual** Squid attachment, not nonzero sources. Their missing exact Rust upstream admission is a named OPEN blocker, not an exclusion.
- Teleport primary locomotion and Grinder entry cannot be reached by stock recognized Ships; endpoint pointer-expiry cleanup still prevents dangling links generally.

### 10. Snapshot and hash

Set snapshot version 113 and reject 112. Version 113 contains:

- attacker manager presence and `victim_id`;
- victim `parasite_attacker_id`;
- new owner `foot_destination_delay` raw signed start/duration;
- existing exact `ShipLocomotionRuntime.destination.z`;
- existing navigation/pending-retry state.

Hash every new field in stable entity order. Changing only manager presence/link, victim back-link, timer start/duration, destination Z, or pending retry must change the world hash. Load finalization validates that every active manager link has one matching victim back-link, every victim back-link names one manager that points back, endpoints differ and exist, and no victim has two managers. Reject malformed v113 snapshots; never silently repair them. Identical fixtures remain deterministic. This is a payload/hash-membership change.

## Acceptance Tests

| Test | Exact assertion |
|---|---|
| Destination/current disagreement | Current structural + destination flat adds no target 416; current flat + destination structural adds 416. |
| Immediate/final disagreement | Active immediate destination decides structure in both directions; `final_goal=None` remains valid. |
| Structural/walkable | Structural adds 416; merely walkable bridgehead/ramp does not. |
| 100/427/500 | Planar 100 -> 100; signed Z 416 -> native 427; slowdown 500 brakes. |
| 129/128 | Native distance is 128; slowdown 129 brakes and 128 equality does not. |
| Fixed boundary | 32767 exact/equality false; 32767 vs a threshold >32767 true; 32768 -> MAX/MAX equality false; long-map no panic/wrap. |
| Signed coordinates/levels | Native cell conversion boundaries; `Level=0xFF -> -103`; nonzero slopes sample exact X/Y. |
| Exact-Z precedence/layer | `exact_z_leptons` wins; `on_bridge` adds 416 once; flat/under-bridge remains ground/water. |
| Invalid install | Missing terrain, out-of-grid, slope 21 reject with full before/after equality. Corrupt pre-existing tick state is total and does not clear as arrived. |
| Stored cell/entity Z | Cell ground/slope + structural bump; entity incoming Z preserved plus conditional bump; Drive control byte-for-byte unchanged. |
| Retarget/cancel/getter | Successful retarget replaces XYZ; rejected retarget preserves it; null clears; getter returns exact triple. |
| Direct scatter stale/no-prior | Recognized Ship direct scatter creates exact NavCom/destination when absent and replaces stale state; structural case stores +416. |
| Direct same-cell | Successful recognized-Ship same-cell request runs setter transaction; rejected request preserves stale state. |
| Terminal Z | delta 0/207 clear; 208/416 preserve and defer repath; on-deck match clears; cell mismatch, missing/non-cell NavCom preserve. |
| Manager construction | Rookie primary Parasite creates an empty manager on non-building SQD/DRON; secondary-only Parasite and buildings do not. Empty manager round-trips distinctly from no manager. |
| Exact CanAttach matrix | Null source, missing manager, non-entity target, limbo/dead/dying/health-zero victim, existing link, `Parasiteable=no`, installed bunker, and Naval missing/non-water cell all preserve state; approach-only bunker control admits. No alliance/mission/ImmuneToPoison gate is added. |
| Two-Squid race | First admitted projectile installs the reciprocal pair; the second fails without replacing links or touching timer. |
| Detonation-to-tail order | Admitted frame-N detonation installs links without changing the timer; an ordinary raw `(creation_frame,0)` victim remains zero-duration and admits a Ship destination install before its next visit; frame-N+1 victim tail runs after destination/locomotor and first writes 32767. |
| Active SQD refresh | Every later water-cell victim tail re-anchors start=current and duration=32767 using the attacker's live slot-0 warhead; ROF 99 and ordinary cooldown do not gate it. |
| Timer construction | A Foot created at frame 123 stores raw `(123,0)`, is inactive, and round-trips/hashes that exact pair; it does not normalize to `(-1,0)`. |
| Timer predicate | `(-1,0)` is inactive; `(-1,32767)` and `(-1,-7)` are active; current start/32767 is active; exact signed expiry is inactive; wrapping-frame cases match `CdTimer::remaining`. |
| Exact detach controls | Known non-water, Sonic, negative damage, accepted funded repair, non-Organic Iron Curtain, attacker uninit, and victim uninit clear both links and set victim duration zero. Unattached, Organic IC, failed repair, true chrono-erase, and broad resolved targets do not clear. |
| OPEN upstream releases | Chronosphere and IsLocomotor tests remain blocking until their exact upstream admission exists; once present, only an actual Naval reciprocal attachment detaches at the cited point. |
| Guarded Move/AttackMove | Positive owner timer rejects before mission/order/attack/NavCom/destination/movement mutation; timer survives the attempt. |
| Retarget/pursuit/target loss | Positive owner timer rejects rearming retarget->pursuit and still rejects after attack target clear/death followed by Move. |
| Ordinary rearm controls | Ship with only `attack_target.cooldown_ticks>0` remains admitted; target-cleared Move is admitted; Walk with the same cooldown remains unchanged. |
| Other guards | `dying`, warp-out, warp-in reject in native order; zero/inactive controls accept. |
| Non-Ship water/Drive | Non-Ship water mover and Drive keep prior 2D result and command behavior. |
| Save/load/hash | v113 round-trips empty manager, reciprocal active attachment, raw timer, exact structural destination Z, and pending retry; v112 and malformed reciprocal links reject; each changed field changes hash. |

No test may treat a seeded positive timer as proof of the active writer; both combat-writer and movement-consumer integration must be exercised.

## Risks and Prohibited Shortcuts

- Do not call the field rearm, firing, garrison, locomotor-speed, or target cooldown.
- Do not use `AttackTarget.cooldown_ticks` for setter guard 2.
- Do not write `+0x6A8` at projectile detonation. Detonation installs links; the first write is the next victim Foot-AI tail after destination/locomotor.
- Do not call `0x006297F0` Temporal or clear this timer from true chrono-erase.
- Do not replace reciprocal attachment with a resolved target, boolean latch, frozen detonation-time `Paralyzes`, or weapon-ROF timer.
- Do not clear for Chronosphere/IsLocomotor until their exact stock-active upstream admission exists. These are OPEN blockers.
- Do not expand into full grapple damage/animation/culling/placement, but ensure any endpoint death or future terminal producer enters the exact detach helper.
- Do not omit the active stock writer, snapshot/hash membership, or target-loss persistence.
- Do not use `isqrt`, host floating point, planar+416, current cell, final goal, movement zone, or walkability as the Ship structural calculation.
- Do not accept invalid Ship geometry with Z zero.
- Do not clear NavCom/destination unconditionally at terminal.
- Do not alter Drive destination semantics, `OnBridge`, physical Ship elevation, path layers, rendering, bridge traversal, or RNG.

## Alternatives

### Reuse ordinary attack cooldown

Rejected. Native setter reads Foot `+0x6A0/+0x6A8`; ordinary weapon cadence is Techno `+0x2EC/+0x2F4`. Rust's target-owned cooldown is lost on target mutation and has the wrong writers.

### Add only a synthetic positive guard fixture

Rejected. Retail Squid/ParasitePlus actively writes a Ship victim only after a successful reciprocal attachment and a later victim tail. A fixture or detonation-time write cannot prove the production path.

### Implement the full Parasite/WarpAttach system

Rejected for this bounded slice. Exact guard reachability requires manager presence, reciprocal attachment, victim-tail refresh, and detach, but not grapple animations, periodic damage, culling damage, or attached-attacker placement. Those excluded effects may not bypass endpoint cleanup when later implemented.

### Chosen: bounded reciprocal attachment + owner timer + transactional Ship setter + pure Ship distance/arrival helpers

This is the smallest architecture-aware fit that makes the stock guard's attach/write/clear lifecycle exact. The design remains OPEN because the stock-active Chronosphere and IsLocomotor upstream release admissions are not yet owned in Rust.

## Review Gate

This revision is **OPEN** and does not claim implementation readiness or `PASS`. Fresh read-only critic 7 must receive the requirement, live native evidence, retail data, current Rust state, the complete diff through the timer repair, critic-5 findings, critic-6 timer PASS, and every earlier finding. It must recheck the corrected sentinel and native `(current frame,0)` construction plus all prior fixes. The row cannot close until the exact stock-active Chronosphere and IsLocomotor release admissions are promoted or implemented and then reviewed with the attachment, Ship destination, braking, scatter, arrival, snapshot, and hash work. Any incorrect admission, writer ordering, detach caller, transactional caller, invalid-geometry mutation, terminal clear mismatch, snapshot/hash omission, or prior-regression keeps the design and GSI row open.
