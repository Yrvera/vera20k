# Phase 3 Drive/Ship slope-transition lifecycle — active retail Ghidra report

**Address(es):** `0x004B0500` (Drive Process), `0x0069FC10` (Ship Process), `0x004AFF60` / `0x0069F670` (Draw_Matrix consumers)

**Investigation Mode:** exhaustive-slice

**Claimed Scope:** active-retail Drive/Ship containing-cell slope cache, transition timer, scheduling, rendering, persistence, and Rust handoff

**Non-Scope:** general locomotion/pathfinding/track geometry, impact-driven body rocking, wake/dust, and unrelated voxel composition

**Confidence:** High

**Active in YR:** Yes — conditional on the active locomotor being Drive or Ship; stock activation is proven from retail `rulesmd.ini`

**Verdict:** **VERIFIED / CLOSED for implementation.** Active retail `gamemd.exe` has one shared slope-transition mechanism, implemented independently but instruction-for-instruction equivalently by `DriveLocomotionClass` and `ShipLocomotionClass`. It samples the owner's **current containing cell at the beginning of each eligible locomotor `Process` call**, before movement or track processing. A changed byte-valued `CellClass+0x11C` slope starts a three-frame, global-frame-based render interpolation from the previously cached slope to the newly sampled slope. Successful unlimbo and a narrow tunnel/piggyback restoration path snap the cache immediately and disable interpolation. No native RNG participates.

Current Rust does not run this lifecycle for production entities because `GameEntity::new` initializes `rocking: None`; its manually enabled implementation also samples after movement and decrements a mutable counter, which differs from the native pre-movement/global-frame contract. The smallest exact implementation contract and discriminating tests are specified below. No load-bearing question remains open.

## Investigation contract

- **Mode:** `exhaustive-slice`
- **Scope:** the active-retail Drive/Ship voxel slope-transition state, its live callers, sampling and render scheduling, eligibility/exclusions, persistence, RNG involvement, and the smallest Rust-facing closure.
- **Non-scope:** general Drive/Ship pathfinding, speed, track geometry, body rocking from impacts, wake/dust effects, and general voxel matrix composition except where needed to order or consume the slope transition.
- **Binary:** the active Ghidra program `gamemd.exe` in project `testProsjekt`.
- **Data authority:** retail `ini/rulesmd.ini` for stock locomotor selection.
- **Candidate-only inputs:** existing slope/Drive/Ship reports and current Rust. Conflicting candidate claims were rechecked against the active binary rather than inherited.
- **Confidence:** high for every implementation-bearing claim. The only indeterminate native value is an unused/preserved timer middle dword; it has no effect on this lifecycle.

## 1. Overview

Whenever a stock Drive- or Ship-locomotor voxel unit crosses between cells whose `CellClass+0x11C` slope bytes differ, its body tilt renders at old slope, one-third blend, two-thirds blend, then new slope. Because sampling occurs before movement, a cell boundary crossed later in the same `Process` call is noticed at the next eligible `Process` entry. This is checked every eligible update even while the unit is stationary, but state is rewritten only on a slope change.

The trigger is ordinary and recurrent: every ramp entry, exit, or change of ramp orientation by an eligible unit. The current Rust omission is therefore player-visible on every such crossing, not a rare save/load or expert-only edge case. It affects presentation orientation, not native passability or movement speed.

## 2. Class Layout / Key Offsets

### Exact active classes

| Class | ILocomotion vtable | RTTI complete-object locator | RTTI type descriptor | RTTI name | CLSID |
|---|---:|---:|---:|---|---|
| Drive | `0x007E7EB0` | `0x007FFDE8` | `0x00820248` | `.?AVDriveLocomotionClass@@` | `{4A582741-9839-11D1-B709-00A024DDAFD1}` |
| Ship | `0x007F2D8C` | `0x008093A0` | `0x0083F880` | `.?AVShipLocomotionClass@@` | `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` |

The class identity is therefore exact rather than inferred from method similarity. Ship has a full counterpart: constructor, force-slope wrapper/helper, `Process`, `Draw_Matrix`, timer fields, and persistence are all present and equivalent to Drive.

### Live owner/caller chain

`FootClass::AI @ 0x004DA530` is the active per-object caller. At `0x004DA877` it dispatches ILocomotion slot `+0x40` (`Process`) through the active locomotor held at `FootClass+0x674`. The call is behind the ordinary active-Foot gates visible in this function: a non-null locomotor plus object-state tests at `FootClass+0x3CD`, `+0x8D`, `+0x81`, and the `+0x2A8` / type `+0x692` pair. This report does not assign speculative semantic names to those bytes.

The behavioral eligibility rule is exact and small:

> Run the slope lifecycle when a normally processed `FootClass` object's **currently active** ILocomotion object has the Drive or Ship vtable.

In stock retail data those owners are `UnitClass` objects. The retail `rulesmd.ini` census selects Drive for 52 vehicle types and Ship for 13:

- **Drive (52):** `MTNK`, `FV`, `MGTK`, `SREF`, `BFRT`, `AMCV`, `TNKD`, `HOWI`, `DRON`, `HTK`, `HTNK`, `V3`, `APOC`, `SMCV`, `HORV`, `HARV`, `UTNK`, `TTNK`, `DTRUCK`, `LTNK`, `YTNK`, `TELE`, `MIND`, `CAOS`, `PCV`, `SMIN`, `TRUCKA`, `TRUCKB`, `BUS`, `CIVP`, `DDBX`, `PICK`, `CAR`, `WINI`, `XCOMET`, `PROPA`, `CONA`, `COP`, `EUROC`, `LIMO`, `STANG`, `SUVB`, `SUVW`, `TAXI`, `YCAB`, `JEEP`, `BCAB`, `PTRUCK`, `DOLY`, `CBLC`, `FTRK`, `AMBU`.
- **Ship (13):** `DEST`, `DLPH`, `AEGIS`, `CARRIER`, `HYD`, `SUB`, `SQD`, `DRED`, `BSUB`, `VLAD`, `CRUISE`, `TUG`, `CDEST`.

Commented alternate `Locomotor=` lines do not change this census. A modded `UnitClass` with either exact CLSID is eligible by the same runtime rule.

### Explicit exclusions

The relevant ILocomotion slots show that only Drive and Ship override both force-slope slots and own the interpolating `Draw_Matrix`:

| Class | vtable | Draw `+0x24` | Process `+0x40` | Force_New_Slope `+0x50` | Set_Slope `+0x7C` |
|---|---:|---:|---:|---:|---:|
| base Locomotion | `0x007EADF4` | `0x0055A730` | `0x0055AC60` | `0x0055AC20` | `0x0055ACE0` |
| Drive | `0x007E7EB0` | `0x004AFF60` | `0x004B0500` | `0x004B04D0` | `0x004AFB40` |
| DropPod | `0x007E8278` | `0x0055A730` | `0x004B5B70` | `0x0055AC20` | `0x0055ACE0` |
| Fly | `0x007E89F4` | `0x004CF610` | `0x004CCB40` | `0x0055AC20` | `0x0055ACE0` |
| Hover | `0x007EACFC` | `0x00513F40` | `0x00514310` | `0x0055AC20` | `0x0055ACE0` |
| Jumpjet | `0x007ECD68` | `0x0054DCC0` | `0x0054AEC0` | `0x0055AC20` | `0x0055ACE0` |
| Mech | `0x007EDB6C` | `0x0055A730` | `0x005B0060` | `0x0055AC20` | `0x0055ACE0` |
| Rocket | `0x007F0B1C` | `0x00663470` | `0x006622C0` | `0x0055AC20` | `0x0055ACE0` |
| Ship | `0x007F2D8C` | `0x0069F670` | `0x0069FC10` | `0x0069FBE0` | `0x0069F250` |
| Teleport | `0x007F5000` | `0x0055A730` | `0x007192F0` | `0x0055AC20` | `0x0055ACE0` |
| Tunnel | `0x007F5A24` | `0x00729B40` | `0x00728E30` | `0x0055AC20` | `0x0055ACE0` |
| Walk | `0x007F69F8` | `0x0055A730` | `0x0075AC80` | `0x0055AC20` | `0x0055ACE0` |

Every class name in this exclusion table was independently bound by the required RTTI walk (`vtable-4` → COL → `COL+0x0C` TypeDescriptor → mangled name), not by a display label:

| Class | COL | TypeDescriptor / mangled-name identity |
|---|---:|---|
| base Locomotion | `0x008032A0` | `0x00820228`, `.?AVLocomotionClass@@` |
| DropPod | `0x00800040` | `0x008202F8`, `.?AVDropPodLocomotionClass@@` |
| Fly | `0x00800658` | `0x008223E8`, `.?AVFlyLocomotionClass@@` |
| Hover | `0x00803228` | `0x008254B8`, `.?AVHoverLocomotionClass@@` |
| Jumpjet | `0x00804C88` | `0x00829648`, `.?AVJumpjetLocomotionClass@@` |
| Mech | `0x00805C28` | `0x0082C228`, `.?AVMechLocomotionClass@@` |
| Rocket | `0x00808908` | `0x008399C8`, `.?AVRocketLocomotionClass@@` |
| Teleport | `0x0080C178` | `0x00844538`, `.?AVTeleportLocomotionClass@@` |
| Tunnel | `0x0080CAE8` | `0x00844AA0`, `.?AVTunnelLocomotionClass@@` |
| Walk | `0x0080D240` | `0x00847BF0`, `.?AVWalkLocomotionClass@@` |

`0x0055AC20` is a no-op and `0x0055ACE0` is a no-op `RET 8`. Consequently aircraft (`Fly`, `Jumpjet`, `Rocket`), Hover, Walk, Tunnel, and the dormant/non-stock-selected Mech and DropPod classes do not participate in this mechanism.

“Train” is not a separate locomotor class. `IsTrain=` is a type flag consumed by Drive behavior, and retail has no active `IsTrain=yes` assignment. A modded train that uses Drive remains eligible; excluding it merely because `IsTrain` is set would be incorrect.

`LocomotionClass::ForEach_SetSlopeIndex @ 0x004E1570` is not part of the proved active chain: it has no direct code caller and is not reached by the verified `FootClass::AI`, unlimbo, or render routes. It is a dormant bulk helper, not an alternate active scheduler.

### State layout and constructor defaults

The Drive constructor is `0x004AF540`; Ship is `0x0069EC50`. Their slope-related writes are structurally identical. The complete locomotor object uses a persistence size of `0x70`; ILocomotion method `this` points four bytes into that object. Both coordinate systems are shown to avoid the common four-byte offset error.

| Role | object-base offset | ILocomotion-`this` offset | constructor state | behavior |
|---|---:|---:|---|---|
| cached/current slope | `+0x1C` | `+0x18` | `0` | latest sampled `Cell+0x11C` byte |
| previous slope | `+0x20` | `+0x1C` | `0` | cached slope immediately before last change |
| timer start frame | `+0x24` | `+0x20` | `g_CurrentFrameCounter` | absolute binary frame of transition start |
| timer middle dword | `+0x28` | `+0x24` | **not initialized here** | never read by `Start`, `Remaining`, change detection, or draw interpolation |
| timer duration | `+0x2C` | `+0x28` | `0` | set to `3` on a sampled change |
| transition total/gate | `+0x30` | `+0x2C` | `0` | set to `3` on a sampled change; zero means snap/stable |

`LocomotionClass::Constructor @ 0x0055A6C0` initializes only the common base through object `+0x14`; it does not define the slope timer's middle dword. The immediate helpers at `0x004AFB40` and `0x0069F250` copy an indeterminate stack-local dword into object `+0x28` while setting all behavior-bearing fields deterministically. This is an opaque/padding-like preserved value, not native entropy or RNG and not a Rust compatibility requirement.

Constructor slope zero is not the live placed-object initial visual state. Successful unlimbo immediately replaces it with the containing cell slope as described next.

## 3. Core Logic

### Initialization and immediate synchronization

`FootClass::Unlimbo @ 0x004D7170` calls the active locomotor's slot `+0x50` at `0x004D71A9` after `TechnoClass::Unlimbo` succeeds. Drive wrapper `0x004B04D0` and Ship wrapper `0x0069FBE0` both:

1. query the linked owner's current containing cell through owner vtable slot `+0x1BC`;
2. load the byte at `CellClass+0x11C`;
3. dispatch ILocomotion slot `+0x7C` with that byte.

Drive helper `0x004AFB40` and Ship helper `0x0069F250` then set `previous = current = sampled_slope`, set timer start to the current global frame, set duration and total to zero, and therefore produce an immediate stable render. There is no flat-to-ramp spawn blend.

An exact indirect-call census found only three ILocomotion `+0x7C` dispatches:

- `0x004B04ED`, inside the Drive force-slope wrapper;
- `0x0069FBFD`, inside the Ship force-slope wrapper;
- `0x00742BE3`, inside `TechnoClass::Set_Destination @ 0x00741970`.

The third call is not a general “movement started” reset. Its enclosing branch identifies an active Tunnel locomotor under piggyback, requires the ground-layer case, performs the Tunnel/Drive piggyback conversion/restoration, and then snaps the restored active locomotor to the owner's current containing-cell slope. This narrow restoration path must not be generalized into every move command.

### Exact per-tick sampling and scheduling

### Drive

`DriveLocomotionClass::Process @ 0x004B0500` executes the slope prologue before any movement/track branch:

- `0x004B050B..0x004B0510`: obtain the owner's current containing cell;
- `0x004B051B`: load `CellClass+0x11C`;
- `0x004B0523..0x004B0528`: compare with cached/current slope;
- on change, `0x004B052A`: copy current to previous;
- `0x004B0533`: store sampled slope as current;
- `0x004B052D..0x004B0554`: start/copy a duration-3 timer;
- `0x004B0557`: store transition total `3`.

Only after this prologue does the routine inspect/process track state (`0x004B0576` onward), invoke the no-track movement path (`0x004B0647`), the active movement path (`0x004B0A79`), and final track processing (`0x004B0AAA`).

### Ship

`ShipLocomotionClass::Process @ 0x0069FC10` has the same slope prologue at `0x0069FC1B..0x0069FC67`, before its first track decision/call at `0x0069FC86`, no-track movement call at `0x0069FCEE`, and subsequent active movement/track work.

### Consequences

- Sampling is **pre-movement and pre-track**.
- Sampling is unconditional on movement intent within an eligible `Process` entry, so a stationary unit is checked too.
- Only the owner's **current containing cell** is sampled. There is no previous-cell query and no next/head-to/destination-cell look-ahead.
- “Previous slope” is a cached byte, not a previous cell reference. “Next slope” means the newly sampled current-cell byte after it differs, not a path preview.
- If movement later in the same call crosses a cell boundary, the new cell's slope is observed on the next eligible `Process` entry.
- Equal slope leaves all transition fields unchanged. There is no stable-cell decrement or expiry callback in `Process`.
- A change during an in-progress transition uses the prior **target/current slope byte** as the new `previous`; it does not sample or bake the visually interpolated matrix.

The byte at locomotor object `+0x62` (ILocomotion `this+0x5E`) is not a slope countdown. Its `Process` use follows `FacingClass::Is_Rotating @ 0x004C9480` and records owner-facing/turn completion state. Treating it as slope lifecycle state is a candidate-report error.

### Timer and render consumption

`CDTimerClass::Start @ 0x0046B640` writes the current global frame to timer `+0` and the requested duration to timer `+8`; it deliberately does not touch the middle dword at `+4`.

`CDTimerClass::Remaining @ 0x004B4D70` behaves as follows:

- if start frame is `-1`, return the stored duration;
- otherwise compute signed `elapsed = g_CurrentFrameCounter - start_frame`;
- return `duration - elapsed` while signed `elapsed < duration`, else return `0`.

No RNG and no mutable per-tick decrement are involved.

Drive `Draw_Matrix @ 0x004AFF60` and Ship `Draw_Matrix @ 0x0069F670` read the same state. If total is zero they use `t = 1`. Otherwise they call `Remaining` and compute:

```text
t = (total - remaining) / total
```

When `t < 1`, both call `VXL_InterpolatedFacing @ 0x00755A40` with `previous` as the source slope, `current` as the destination slope, and `t`; that routine uses the voxel slope quaternion/matrix table and SLERP. Unit voxel rendering reaches active-locomotor `Draw_Matrix` at `UnitClass::DrawVoxelBody` call `0x0073B5CB` and `UnitClass::Draw_Body_And_Turret` call `0x0073C87A`.

For a transition detected in binary frame `F`, the exact visible phases are:

| global frame | Remaining | interpolation `t` | result |
|---:|---:|---:|---|
| `F` | `3` | `0/3` | previous/old slope |
| `F+1` | `2` | `1/3` | one-third blend |
| `F+2` | `1` | `2/3` | two-thirds blend |
| `F+3` and later | `0` | `1` | current/new slope |

Multiple draws in one global frame are read-only and see the same phase. Expiry does not zero the stored total; total can remain `3` indefinitely while `Remaining` returns zero and draw uses the stable current slope. A later change overwrites start/duration/total. The immediate helper instead sets total zero, which also draws stable current slope.

This draw path is player-visible for voxel-rendered bodies. The locomotor state exists independently of whether a particular modded unit supplies a voxel body, but a non-voxel body has no voxel matrix on which to display the SLERP.

### Persistence and load behavior

| Class | persistence vtable | Load | Save | persisted object size |
|---|---:|---:|---:|---:|
| Drive | `0x007E7F7C` | `0x004AF780` | `0x004AF800` | `0x70` (`0x004B4CF0`) |
| Ship | `0x007F2E58` | `0x0069EE90` | `0x0069EF10` | `0x70` (`0x006A42A0`) |

The shared save helper `0x0055AA60` writes the class-reported `0x70` raw bytes before class-specific piggyback data; shared load helper `0x0055AAC0` restores the raw block and participates in pointer swizzling. Class Load restores the correct vtables and class-specific piggyback state. Therefore current slope, previous slope, timer start, duration, total, and the opaque middle dword all survive native save/load. Load does not resample terrain or snap the slope cache. The next eligible `Process` restarts only if the restored cached current slope differs from the then-current containing cell.

Rust need not serialize the unused indeterminate middle dword, but it must serialize/hash enough defined state to resume the same global-frame phase: previous slope, current slope, transition start frame, and total/duration (or a formally equivalent representation proven against load timing).

### Native RNG audit

No RNG call occurs in either constructor slope initialization, force-slope wrapper/helper, `Process` slope prologue, `CDTimerClass::Start`, `CDTimerClass::Remaining`, either `Draw_Matrix` slope branch, or `VXL_InterpolatedFacing`. The indeterminate timer middle dword is never consumed and is not a random source. Dust, wake, combat rocking, and other calls elsewhere in Drive/Ship behavior are outside this mechanism.

## 4. INI Keys

There is no slope-transition duration, enable, or interpolation INI key. The duration `3`, containing-cell byte source, and class restriction are hard-coded. Activation follows the installed `Locomotor=` CLSID.

| Key | Type | Retail value/default in target set | Effect on this slice | Active in YR? | Evidence / confidence |
|---|---|---|---|---|---|
| `Locomotor` | CLSID string | every one of the 65 stock sections listed in §2 explicitly selects Drive or Ship | installs the only two classes that override the slope lifecycle slots | Yes | retail `ini/rulesmd.ini` census + RTTI/vtable walk; HIGH |
| `IsTrain` | boolean type flag | no active `IsTrain=yes` assignment in retail `rulesmd.ini` | consumed inside Drive behavior but does not exclude or replace the Drive slope lifecycle | No stock activation; conditional for mods | retail INI search + Drive class identity; HIGH for this slice |

The absent/invalid `Locomotor=` fallback is not exercised by any stock section in this target set and is not used to generalize eligibility: runtime active-vtable identity is authoritative.

## 5. Integration Points

| Stage | Entry/callsite | Exact order and condition | Output/consumer | Confidence / Active in YR |
|---|---|---|---|---|
| construction | Drive `0x004AF540`, Ship `0x0069EC50` | before placement; defined behavior fields start flat/inactive | later successful unlimbo replaces the live cache | HIGH / Yes |
| placement | `FootClass::Unlimbo @ 0x004D7170`, call `0x004D71A9` | after successful `TechnoClass::Unlimbo` | Drive/Ship force wrapper snaps to containing cell | HIGH / Yes |
| active tick | `FootClass::AI @ 0x004DA530`, dispatch `0x004DA877` | after normal Foot gates; before locomotor-owned movement | Drive/Ship `Process` slope prologue | HIGH / Yes |
| movement/track | Drive `0x004B0576+`; Ship `0x0069FC86+` | strictly after the slope prologue | may change position; change is observed at next eligible Process | HIGH / Yes |
| narrow restore | `TechnoClass::Set_Destination @ 0x00741970`, call `0x00742BE3` | only in proved Tunnel/piggyback ground restoration branch | immediate active-locomotor slope snap | HIGH / Conditional |
| voxel render | Unit callsites `0x0073B5CB`, `0x0073C87A` | reads active locomotor matrix after sim state exists | Drive/Ship Draw_Matrix and SLERP | HIGH / Yes for voxel bodies |
| save/load | Drive/Ship class Load/Save plus `0x0055AA60` / `0x0055AAC0` | raw class state persists; no load-time resample | next Process compares restored cache | HIGH / Yes |

No direct or indirect active scheduler found in the bounded census supersedes these integration points. The bulk helper `0x004E1570` is dormant relative to this proved chain.

## 6. Current Rust Implementation Status

The audited Rust state is commit `0755e46e` in `feature/phase3-map-spatial-close`.

| Rust surface | Current behavior | Native mismatch |
|---|---|---|
| `src/sim/game_entity.rs:797,1170` (`GameEntity::new`) | initializes `rocking: None` | production Drive/Ship objects have no slope-transition state; tests manually attach it |
| `src/sim/components.rs:1082` (`RockingState`) | stores `prev_slope`, `curr_slope`, mutable `transition_ticks_remaining` together with body rocking | defined native slope lifecycle is a global-frame timer, not a decrementing counter |
| `src/sim/rocking/rocking_system.rs:165,187` | processes every entity with `rocking: Some`; samples terrain, then decrements remaining on equal slope | class eligibility is not Drive/Ship-specific; native stable `Process` does not mutate transition state |
| `src/sim/world/mod.rs:6768` phase 2.5 | calls `rocking::tick` after all movement | native samples at each Drive/Ship `Process` entry before movement/track work |
| `src/app/presentation/instances/units.rs:146,174` | maps remaining `3,2,1` to phases `0,1,2`; falls back to terrain when component absent | phase fractions happen to match manually seeded tests, but production omission and scheduling do not |
| `src/sim/world/world_hash.rs:1496,1505` | hashes optional `RockingState` and remaining counter | hashing exists, but the wrong/absent lifecycle state is hashed |
| serde/snapshot | `GameEntity`/`RockingState` derive serialization | shape is persistable, but a schema bump is required if fields change and a production component must actually exist |

The existing rocking tests prove the local countdown and renderer phase mapping, but their production helper explicitly sets `rocking = Some(RockingState::default())` because spawn does not. They therefore do not prove the live production path or native scheduling.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Drive class identity/vtable | verified | COL `0x007FFDE8`, TD `0x00820248`, vtable `0x007E7EB0` | none |
| Ship class identity/vtable | verified | COL `0x008093A0`, TD `0x0083F880`, vtable `0x007F2D8C` | none |
| Drive constructor/defaults | verified | `0x004AF540`, base `0x0055A6C0` | none |
| Ship constructor/defaults | verified | `0x0069EC50`, base `0x0055A6C0` | none |
| Drive Process slope prologue | verified | `0x004B0500`, writes `0x004B052A..0x004B0557` | none |
| Ship Process slope prologue | verified | `0x0069FC10`, writes `0x0069FC1B..0x0069FC67` | none |
| movement/track relative order | verified | Drive `0x004B0576+`; Ship `0x0069FC86+` | none |
| active tick owner/caller | verified | `FootClass::AI @ 0x004DA530`, call `0x004DA877` | none |
| successful-unlimbo initialization | verified | `0x004D7170`, call `0x004D71A9`, wrappers/helpers | none |
| Drive immediate helper | verified | wrapper `0x004B04D0`, helper `0x004AFB40` | none |
| Ship immediate helper | verified | wrapper `0x0069FBE0`, helper `0x0069F250` | none |
| Set_Destination restoration branch | verified | `0x00741970`, indirect call `0x00742BE3` | none |
| timer Start/Remaining semantics | verified | `0x0046B640`, `0x004B4D70` | none |
| Drive/Ship Draw_Matrix | verified | `0x004AFF60`, `0x0069F670` | none |
| slope matrix interpolation | verified | `VXL_InterpolatedFacing @ 0x00755A40` | none |
| Unit voxel draw integration | verified | calls `0x0073B5CB`, `0x0073C87A` | none within scoped matrix consumption |
| locomotor exclusion matrix | verified | complete relevant vtable slot reads and target bodies in §2 | none |
| dormant bulk helper | verified | `0x004E1570`, no direct code caller; absent from active chains | none |
| retail Drive/Ship activation set | verified | `ini/rulesmd.ini` explicit CLSIDs | none |
| Drive persistence | verified | `0x004AF780`, `0x004AF800`, size `0x004B4CF0` | none |
| Ship persistence | verified | `0x0069EE90`, `0x0069EF10`, size `0x006A42A0` | none |
| shared raw persistence helpers | verified | `0x0055AA60`, `0x0055AAC0` | none |
| RNG absence in bounded chain | verified | full bounded callee/body audit | none |
| Rust production reachability | verified | `GameEntity::new`, `rocking: None`; production tests inject state | implementation mismatch remains |
| Rust scheduling/phase/hash | verified | `rocking_system.rs`, `world/mod.rs`, `units.rs`, `world_hash.rs` | implementation mismatch remains |

All planned and discovered in-scope areas are `verified`; there are no `touched-not-exhausted`, `not-touched`, `deferred`, or `conflict-needs-resolution` rows for the claimed slice.

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] Q01 — What live method schedules slope sampling? → Drive/Ship Process.` (evidence: `0x004B0500`, `0x0069FC10`)
- `[RESOLVED] Q02 — What owner calls those methods? → FootClass::AI dispatches the active locomotor Process.` (evidence: `0x004DA877` in `0x004DA530`)
- `[RESOLVED] Q03 — Which active classes are eligible? → only active Drive and Ship vtables.` (evidence: `0x007E7EB0`, `0x007F2D8C`, vtable matrix §2)
- `[RESOLVED] Q04 — Does Ship have a real counterpart? → yes, with independent equivalent constructor, Process, Draw, helpers, and persistence.` (evidence: `0x0069EC50`, `0x0069FC10`, `0x0069F670`, `0x0069F250`)
- `[RESOLVED] Q05 — What are constructor defaults? → current/previous zero, start current frame, duration/total zero.` (evidence: `0x004AF540`, `0x0069EC50`)
- `[RESOLVED] Q06 — Is every timer dword initialized and meaningful? → no; middle dword is indeterminate and unread by this lifecycle.` (evidence: constructors, `0x004AFB40`, `0x0046B640`, `0x004B4D70`)
- `[RESOLVED] Q07 — What establishes the initial live cache? → successful Foot unlimbo snaps both slopes to the containing-cell byte.` (evidence: `0x004D71A9`, `0x004B04D0`, `0x0069FBE0`)
- `[RESOLVED] Q08 — Does spawn blend from flat? → no; force helpers set total zero.` (evidence: `0x004AFB40`, `0x0069F250`)
- `[RESOLVED] Q09 — Which cell is sampled? → the owner's current containing cell only.` (evidence: both Process prologues and force wrappers)
- `[RESOLVED] Q10 — Is a previous or next/head-to cell queried? → no; previous is a cached byte and no look-ahead slope load exists.` (evidence: `0x004B0500`, `0x0069FC10`)
- `[RESOLVED] Q11 — Is sampling before movement/track work? → yes, at Process entry.` (evidence: Drive `0x004B050B` before `0x004B0576`; Ship `0x0069FC1B` before `0x0069FC86`)
- `[RESOLVED] Q12 — What does an equal sample do? → no transition field write and no decrement.` (evidence: compare branches in both Process methods)
- `[RESOLVED] Q13 — What does a changed sample do? → current→previous, sample→current, timer duration/total=3.` (evidence: Drive `0x004B052A..0x004B0557`; Ship `0x0069FC1B..0x0069FC67`)
- `[RESOLVED] Q14 — How is phase advanced? → signed global-frame Remaining arithmetic, not mutable ticking.` (evidence: `0x0046B640`, `0x004B4D70`)
- `[RESOLVED] Q15 — What are exact draw phases? → 0, 1/3, 2/3, then stable current.` (evidence: `0x004AFF60`, `0x0069F670`)
- `[RESOLVED] Q16 — Does render mutate phase? → no; all timer and slope reads are read-only.` (evidence: both Draw_Matrix bodies)
- `[RESOLVED] Q17 — Is locomotor object +0x62 slope state? → no, its writer follows FacingClass::Is_Rotating.` (evidence: Process use of `0x004C9480`)
- `[RESOLVED] Q18 — Are Fly/Jumpjet/Rocket/Hover/Walk/Tunnel/Teleport/Mech/DropPod included? → no; none override both slope slots and the interpolating draw path.` (evidence: vtable matrix §2)
- `[RESOLVED] Q19 — Is a train automatically excluded? → no; it is a Drive type flag, not another locomotor class.` (evidence: retail `IsTrain` census and Drive vtable)
- `[RESOLVED] Q20 — Is the bulk ForEach helper active here? → no direct code caller and no connection to the proved AI/unlimbo/draw chain.` (evidence: `0x004E1570` caller/xref check)
- `[RESOLVED] Q21 — Does every move start snap slope? → no; the extra call is confined to a Tunnel/piggyback ground restoration branch.` (evidence: `0x00742BE3` within `0x00741970`)
- `[RESOLVED] Q22 — Is transition state saved/loaded? → yes, inside the raw 0x70-byte locomotor block.` (evidence: class Load/Save/size methods and `0x0055AA60` / `0x0055AAC0`)
- `[RESOLVED] Q23 — Does load resample terrain? → no; the next eligible Process performs the ordinary compare.` (evidence: Load bodies plus Process prologues)
- `[RESOLVED] Q24 — Does native RNG participate? → no.` (evidence: bounded constructor/writer/timer/draw/SLERP callee audit)
- `[RESOLVED] Q25 — Is current Rust production-reachable? → no; constructor leaves rocking absent and tests inject it.` (evidence: `src/sim/game_entity.rs`, `src/sim/rocking/rocking_tests.rs`)
- `[RESOLVED] Q26 — Does current Rust schedule sampling natively? → no; it samples in a global post-movement pass.` (evidence: `src/sim/world/mod.rs`, `src/sim/rocking/rocking_system.rs`)

No entry is deferred and no load-bearing question remains.

## 9. Visual/UI Composition Ledger

The visual scope is deliberately the slope matrix layer, not full Unit art composition. The transition changes the body transform applied to the already selected voxel/HVA body; it does not select an alternate image/frame or palette.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | Unit voxel body calls at `0x0073B5CB` / `0x0073C87A` | Unit voxel rendering dispatches active locomotor slot `+0x24` | already selected data-driven VXL/HVA body | existing Unit world anchor; unchanged by this slice | existing voxel palette/conversion; unchanged | yes for voxel-bodied eligible Unit | body matrix request |
| 2 | Drive `Draw_Matrix @ 0x004AFF60` or Ship `0x0069F670` | active locomotor vtable identity; total zero or timer-derived phase | no alternate asset/frame selected | same anchor | no palette change | yes | compose slope transform |
| 3 | `VXL_InterpolatedFacing @ 0x00755A40` | only while computed `t < 1`; source=previous, destination=current | native slope quaternion/matrix table | matrix-space, not screen rect | not applicable | conditional during frames F..F+2 | quaternion SLERP |
| 4 | caller continues normal voxel body draw | returned matrix | same body VXL/HVA | normal projected body bounds | normal voxel conversion | yes | rasterized visible result |

| Asset / data | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| unit-selected VXL/HVA body | data-driven | yes | yes for voxel-bodied eligible units | content | no | no | no | no | Unit callsites `0x0073B5CB`, `0x0073C87A` |
| native slope quaternion/matrix table | in-process data | used as transform | yes through body orientation | content transform | no | no | yes for interpolated frames | no | `VXL_InterpolatedFacing @ 0x00755A40` |
| alternate slope sprite/frame | no | no | no | no | no | no | no | yes | Draw_Matrix bodies select a transform, not an image |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| only active Drive/Ship own sampling | vtable/COL matrix; `0x004DA877` | mismatch: generic `rocking: Some` gate and production absence | `game_entity.rs`, `movement/locomotor.rs`, movement Process-entry seam | attach/drive slope state from `active_kind()` Drive/Ship | production Drive and Ship work without manual state; excluded classes do not | do not gate on effective/installed kind or generic voxel/body-rocking presence |
| successful unlimbo snaps cache | `0x004D71A9`, wrappers/helpers | missing | `world_spawn.rs` successful reveal/unlimbo path | previous=current=final cell slope, total=0 | spawn directly on ramp renders stable ramp | do not blend constructor zero to placement slope |
| Process samples before movement/track | `0x004B050B` / `0x0069FC1B` before movement addresses | mismatch: `rocking::tick` is after movement | `movement_tick.rs` Drive/Ship Process-entry path; remove slope work from global post-move stage | detect boundary one eligible Process after crossing | cross A→B during F, detect at next entry only | do not retain an after-movement global slope pass |
| timer is frame-derived, stable Process is no-write | `0x0046B640`, `0x004B4D70`, Draw bodies | mismatch: mutable remaining decrement | `components.rs`, `rocking_system.rs`, presentation snapshot/renderer | retain start/total and derive 0,1/3,2/3,stable from binary frame | exact four-frame phase ledger plus duplicate-render immutability | do not decrement on equal slope or clear total on expiry |
| restoration snap is narrow | `0x00742BE3` branch context | unchecked/missing exact slope side effect | piggyback/Tunnel restoration path | snap only when equivalent branch restores active Drive | Teleport/Tunnel restoration fixture | do not reset slope for every Set_Destination/move command |
| state persists and is deterministic | class Load/Save and raw `0x70` block | partial: serde/hash exist for current wrong fields | `snapshot.rs`, `world_hash.rs`, component serde | persist/hash defined previous/current/start/total fields and bump schema | mid-transition save/load resumes same phase/hash | do not serialize native unused middle dword or recompute cache on load |
| slope lifecycle consumes no RNG | bounded callee audit | current slope update itself consumes none | Process-entry slope helper/tests | keep RNG state unchanged | RNG before/after initialize/change/render equal | do not share wake/dust RNG calls with slope update |

### Smallest exact implementation contract

This is a behavior contract, not a prescribed refactor. Reusing `RockingState` is the smallest source change if slope eligibility is gated separately from body-rocking eligibility.

1. **Defined slope state:** represent `previous_slope`, `current_slope`, signed/native-frame `start_frame`, and `total = 0 or 3`. The duration is `3` whenever total is `3`. Do not reproduce the native unused middle dword.
2. **Eligibility:** run slope sampling only when the entity is a `Foot`-equivalent with `locomotor.active_kind()` equal to `Drive` or `Ship`. Do not gate on `effective_kind()`: a temporarily active Drive piggyback is natively a Drive object and is eligible. Voxel-body presentation determines visibility, not sim ownership.
3. **Successful unlimbo:** after placement succeeds and before ordinary active ticking, sample the final containing-cell slope and set `previous = current = slope`, `start_frame = binary_frame`, `total = 0`.
4. **Process entry:** at the start of each eligible Drive/Ship object update, before tube/forced-track/ordinary movement can change position, sample the current containing-cell slope. On difference, assign `previous = current`, `current = sample`, `start_frame = binary_frame`, `total = 3`. On equality, do nothing.
5. **Movement ordering:** a boundary crossed by movement in frame `F` must not be detected by an after-movement global pass in `F`; it is detected at the next eligible Drive/Ship process entry.
6. **Immediate restoration:** if Rust exercises the Tunnel/piggyback-to-Drive restoration equivalent of `0x00742BE3`, snap to the current cell with total zero after restoration. Do not perform this reset for every move command.
7. **Render:** derive remaining/phase from the saved transition start and the current binary frame, yielding exactly phases `0`, `1/3`, `2/3`, then stable. Drawing must not mutate state. A stable elapsed transition need not be cleared.
8. **Persistence/determinism:** serialize the defined state, bump the snapshot schema for any layout change, and hash every defined field/presence bit. A mid-transition save/load at a restored frame must resume the same phase.
9. **Exclusions:** do not give this lifecycle to Fly/aircraft, Jumpjet, Rocket, Hover, Walk, Tunnel, Teleport, Mech, DropPod, buildings, or infantry merely because they can render or carry a body-rocking component. A modded `IsTrain` Drive unit remains included.
10. **RNG:** consume no `SimRng` values and introduce no RNG-dependent initialization or phase choice.

The current post-movement all-`RockingState` pass can remain for body rocking only, but slope sampling must move to the active Drive/Ship `Process` entry seam. If one component continues to hold both concerns, its body-rocking update and slope-transition update must use independent eligibility and scheduling.

### Discriminating acceptance tests

1. **Production Drive unlimbo on ramp:** spawn a real Drive `Unit` onto nonzero slope without manual component injection. Assert previous=current=cell slope and total zero; first render is stable, not a flat-to-ramp transition.
2. **Production Ship counterpart:** perform the same test with a real Ship type and prove identical state/phase behavior.
3. **Pre-movement boundary timing:** begin an eligible Drive unit on slope A and make its movement cross into slope B during frame `F`. Assert no A→B transition is started by an after-movement pass in `F`; assert the next eligible process entry starts it at phase 0.
4. **Exact four-frame render ledger:** after detection at `F`, assert old at `F`, `1/3` at `F+1`, `2/3` at `F+2`, and stable new at `F+3`.
5. **Stationary discovery and stable no-write:** externally place/reconcile an eligible unit so its containing slope differs, with no movement target; its next process starts a transition. Repeated equal-slope process calls do not mutate start/total.
6. **Mid-transition retarget:** change A→B at `F`, then B→C before expiry. Assert the new transition is B→C, not interpolated(A,B)→C, and restarts at phase 0.
7. **Class exclusion matrix:** identical terrain changes for Walk, Hover, Fly, Jumpjet, Rocket, Teleport, and Tunnel do not create slope state or consume slope RNG. Include a Drive unit with `IsTrain=yes` and assert it remains eligible.
8. **Active-vs-effective piggyback:** an installed Teleport unit temporarily driven by active Drive samples as Drive; ordinary restoration follows its specific snap contract rather than a generic move-start reset.
9. **Save/load:** save at phase `1/3`, load with the same restored binary frame, and assert render phase/state hash continuity. Also assert subsequent equal-slope process does not reset start.
10. **No RNG consumption:** compare `SimRng` state before/after initialization, stable processing, changed-slope processing, and render-state extraction.
11. **Production-path discrimination:** remove every manual `rocking = Some(...)` assignment from the fixture setup for the slope test; reach the mechanism exclusively through normal spawn/unlimbo and active locomotor resolution.
12. **Multiple render reads:** extract/render the same frame twice and prove identical phase with no sim/hash mutation.

### Stale Docs / Follow-up Docs

Live evidence corrects these candidate interpretations:

- `Process` does **not** invoke `Set_Slope`/slot `+0x7C` after starting the timer and therefore does not immediately cancel every transition.
- locomotor object `+0x62` is facing/turn bookkeeping, not a three-tick slope countdown.
- slope is sampled before movement, not after movement.
- there is no next-cell/head-to slope lookup.
- constructor zero is not the placed-unit initial cache; successful `FootClass::Unlimbo` snaps to the occupied cell.
- the `TechnoClass::Set_Destination` `+0x7C` call is a narrow Tunnel/piggyback restoration branch, not every movement start.
- Ship is not absent or materially different; it owns an exact active counterpart.

Existing research documents remain useful as discovery candidates, but these corrected facts should be authoritative for implementation of this bounded lifecycle. Exact replacement wording for the known stale claims is: **“Drive and Ship sample their current containing-cell slope at Process entry, start a global-frame duration-3 render timer on change, and do not call the immediate slope helper from Process; successful unlimbo and the narrow Tunnel/piggyback restoration branch snap instead.”**

### Exhaustion gate: adversarial checks

1. **Could a hidden call at the end of `Process` snap/cancel the timer?** No. The complete ILocomotion `+0x7C` indirect-call census has only the two wrappers and the narrow Set_Destination restoration call; neither `Process` body dispatches it.
2. **Could Ship merely inherit Drive behavior without persistent state?** No. Ship has its own constructor, state writes, Process prologue, Draw_Matrix consumer, Load/Save, and exact vtable overrides.
3. **Could the slope byte come from the destination or track head-to cell?** No. Both prologues obtain the linked owner's current containing cell before any track branch and perform no destination-cell slope load.
4. **Could the three-frame phase be decremented elsewhere between Process and Draw?** No. Draw calls `CDTimerClass::Remaining`, which derives from the global frame; stable Process makes no timer write and no writer to a countdown field exists.
5. **Could generic voxel or body-rocking eligibility imply all locomotors receive slope interpolation?** No. The class vtable matrix isolates the override to Drive and Ship, while all named exclusions inherit no-op force-slope slots.

### Exhaustion gate: cold spot checks

1. **Initialization cold spot:** following `FootClass::Unlimbo` changed the initial-state conclusion. Constructor zero does not cause a first-tick flat-to-ramp blend because successful unlimbo calls the class override and snaps both cached slopes to the containing cell.
2. **Persistence cold spot:** following the secondary persistence vtables and shared raw-block helpers proved that slope/timer phase is serialized rather than recomputed on load. This makes snapshot state and hash coverage part of the Rust acceptance contract.

### Exhaustion gate: zero-add pass

After resolving the lifecycle, a final bounded search rechecked:

- all indirect calls at ILocomotion slots `+0x50` and `+0x7C`;
- the full Drive and Ship `Process` entry ordering around every `Cell+0x11C` load;
- all locomotor vtable implementations of Draw, Process, force-slope, and set-slope slots;
- the `FootClass::AI`, successful-unlimbo, Tunnel/piggyback restoration, and Unit voxel draw chains;
- the Drive/Ship persistence vtables, class sizes, and shared raw save/load helpers;
- constructor and helper assembly for every defined and indeterminate slope timer dword;
- RNG callees in the bounded writer/timer/render chain; and
- current Rust spawn, movement ordering, component update, render phase, serialization, and hash surfaces.

The pass added no new active writer, eligible class, random input, alternate sampled cell, decrement path, or load-time resample. The slice remains closed.

## 11. Ghidra Annotation Candidates

No Ghidra metadata was changed during this investigation. Suggested future annotations, after independent review:

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x004B050B`, `0x0069FC1B` | class method context only | pre-movement containing-cell slope sample prologues | comment | instruction order in both Process bodies | worker-report-only |
| `0x004D71A9` | Foot unlimbo indirect dispatch | successful-unlimbo immediate Drive/Ship slope synchronization | comment | receiver/slot/body trace | worker-report-only |
| `0x00742BE3` | indirect slot call | narrow Tunnel/piggyback restoration slope snap; not generic move start | comment | enclosing branch receiver/condition trace | worker-report-only |
| object `+0x1C/+0x20/+0x24/+0x2C/+0x30` | incomplete/ambiguous field metadata | current slope / previous slope / timer start / duration / transition total | struct comments | constructor, Process, timer, Draw, persistence bodies | worker-report-only |
| object `+0x28` | ambiguous timer member | opaque unused timer-middle dword | struct comment | constructor/helper assembly plus Start/Remaining reads | worker-report-only |
| object `+0x62` | potentially stale slope interpretation | facing/turn bookkeeping; not slope countdown | correction comment | Process call to `0x004C9480` and branch writes | worker-report-only |

## Sources

### Active binary evidence

- `DriveLocomotionClass` constructor `0x004AF540`, force helper `0x004AFB40`, Draw `0x004AFF60`, force wrapper `0x004B04D0`, Process `0x004B0500`, Load `0x004AF780`, Save `0x004AF800`.
- `ShipLocomotionClass` constructor `0x0069EC50`, force helper `0x0069F250`, Draw `0x0069F670`, force wrapper `0x0069FBE0`, Process `0x0069FC10`, Load `0x0069EE90`, Save `0x0069EF10`.
- `LocomotionClass::Constructor @ 0x0055A6C0`.
- `FootClass::Unlimbo @ 0x004D7170`; `FootClass::AI @ 0x004DA530`.
- `TechnoClass::Set_Destination @ 0x00741970`, call at `0x00742BE3`.
- `CDTimerClass::Start @ 0x0046B640`; `CDTimerClass::Remaining @ 0x004B4D70`.
- `VXL_InterpolatedFacing @ 0x00755A40`.
- Unit voxel matrix call sites `0x0073B5CB` and `0x0073C87A`.
- shared persistence helpers `0x0055AA60` and `0x0055AAC0`; size methods `0x004B4CF0` and `0x006A42A0`.
- dormant bulk helper `LocomotionClass::ForEach_SetSlopeIndex @ 0x004E1570`.

### Retail data

- `ini/rulesmd.ini`: uncommented `Locomotor=` selections for stock `VehicleTypes`/unit sections.

### Current Rust inspected

- `src/sim/components.rs`
- `src/sim/game_entity.rs`
- `src/sim/rocking/rocking_system.rs`
- `src/sim/rocking/rocking_tests.rs`
- `src/sim/world/mod.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/movement/locomotor.rs`
- `src/sim/movement/movement_tick.rs`
- `src/app/presentation/instances/units.rs`
- `src/sim/world/world_hash.rs`
- `src/sim/snapshot.rs`

### Candidate research consulted and live-reconciled

- `docs/research/VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`
- `docs/research/VXL_INTERPOLATED_FACING_AND_SLOPE_TRANSITION_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_MOVEMENT_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md`
- `docs/research/FORCE_NEW_SLOPE_CALLERS_GHIDRA_REPORT.md`
- `docs/research/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/VXL_DRAW_MATRIX_GHIDRA_REPORT.md`
- `docs/research/ILOCOMOTION_COM_PROTOCOL_SPEC.md`
