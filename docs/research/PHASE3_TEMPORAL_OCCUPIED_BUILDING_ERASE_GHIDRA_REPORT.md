# Phase 3 Temporal Occupied-Building Erase — Active-Retail Ghidra Research Report

**Address(es):** `TemporalClass::Update @ 0x0071A760` (primary); `TemporalClass::InitiateWarp @ 0x0071AF20`; `BulletClass::DetonateAtCoord @ 0x004690B0`; `TechnoClass::Init_Managers @ 0x006F3F40`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the active-retail production route from a stock Temporal weapon shot to manager attachment, target-owned per-tick progression, interruption, and completion when the target is an occupied `BuildingClass`, including exact independent occupant-vector, inherited Cargo, kill, and `UnInit` ordering  
**Non-Scope:** full Temporal behavior for every non-Building target class; WarpAway pixel composition; custom/mod Temporal weapons beyond the compiled branches needed to state evidence-backed exclusions  
**Confidence:** HIGH  
**Active in YR:** Yes — continuously available through the stock Chrono Legionnaire (`CLEG`), conditionally through a Chrono Legionnaire gunner in an IFV, and present in stock campaign data

## 1. Overview

Retail Temporal erasure is not damage-over-time and is not advanced by a global
manager loop. A firing Techno owns one persistent `TemporalClass`; a successful
Temporal projectile detonation attaches that manager to the target's incoming
chain, and the target advances the chain once per target update by subtracting
the attackers' currently selected weapon damage from a signed warp-point
budget initialized to `target.Type.Strength * 10`.

When that target is an occupied Building and the budget becomes less than one,
`TemporalClass::Update @ 0x0071A760` erases the Building's independent occupant
vector first, source-less and in reverse order, then erases inherited Cargo,
records the Building removal against the Temporal source, and calls Building
`UnInit`. This report closes the production prerequisite identified by the
Phase 3 critic: the tail ordering was already verified, but current Rust has no
reachable producer or updater for it.

The bounded prerequisite is **READY for implementation**. No active unknown,
unverified behavior, or residual remains in the claimed occupied-Building route.

## 2. Class Layout / Key Offsets

### 2.1 `TemporalClass` (`0x50` bytes)

| Offset | Type | Verified purpose | Evidence |
|---:|---|---|---|
| `+0x24` | `TechnoClass*` | owner/source; the Techno which owns this one persistent manager | constructor `0x0071A4E0`; save/load `0x0071A660`/`0x0071A700` |
| `+0x28` | `TechnoClass*` | current target; null while detached | initiation `0x0071AF20`; detach `0x0071ABC0` |
| `+0x38` | pointer | optional auxiliary link; zero on stock `CLEG` route; swizzled on load | constructor/load |
| `+0x3C` | `SuperClass*` | optional auxiliary SuperWeapon link; completion calls `Suspend(0)` when non-null; zero on stock route | completion block `0x0071A760`; constructor/load |
| `+0x40` | `TemporalClass*` | previous incoming manager; null for head | initiation/detach; load swizzle |
| `+0x44` | `TemporalClass*` | next incoming manager | initiation/detach; load swizzle |
| `+0x48` | signed `i32` | warp points remaining | initiation/update/detach |
| `+0x4C` | signed `i32` | current owner weapon damage cache, refreshed during progression | manager init `0x006F3F40`; update/sum chain `0x0071A760` |

### 2.2 Relevant owner, target, Bullet, weapon, and Building fields

| Class / offset | Type | Verified purpose | Evidence |
|---|---|---|---|
| `TechnoClass+0x270` | byte | target is being temporally warped | initiation/detach |
| `TechnoClass+0x274` | `TemporalClass*` | manager owned by this Techno | `TechnoClass::Init_Managers @ 0x006F3F40` |
| `TechnoClass+0x278` | `TemporalClass*` | head of managers targeting this Techno | initiation/detach/update |
| `BulletClass+0x6C` | signed damage | shot damage stored on Bullet | fire/detonate chain |
| `BulletClass+0xAC` | `BulletTypeClass*` | projectile type | detonation `Inviso` tail |
| `BulletClass+0xB0` | `TechnoClass*` | source/firer | detonation Temporal branch |
| `BulletClass+0x10C` | target pointer | intended target | detonation Temporal branch |
| `BulletClass+0x128` | `WarheadTypeClass*` | warhead | special-effect dispatch |
| `WeaponTypeClass+0xA4` | signed damage | damage re-read from the selected weapon every update | `TemporalClass::Update @ 0x0071A760` |
| `WarheadTypeClass+0x15A` | bool | `Temporal=yes` | detonation cascade; manager allocation gate |
| `TechnoTypeClass+0xD3A` | bool | `Warpable=` target eligibility | `CanWarpTarget @ 0x0071AE50` |
| `TechnoTypeClass+0xCD5` | bool | `IsGattling`; initiation updates gattling stage, not immunity | `InitiateWarp @ 0x0071AF20` |
| `BuildingClass+0x684..+0x698` | dynamic vector | independent `CanBeOccupied` occupant vector | occupied-vector report; completion `0x0071A760` |
| `BuildingClass+0x69C` | index/cursor | occupant fire cursor; belongs to vector mechanism | occupied-vector report |
| inherited `TechnoClass+0x114/+0x118` | Cargo head/count | inherited Cargo list, separate from occupant vector | Cargo report; completion `0x0071A760` |

The Bullet source/target offsets above supersede stale notes which interchange
`+0xB0` and `+0x10C`.

## 3. Core Logic

### 3.1 Production manager creation

`TechnoClass::Init_Managers @ 0x006F3F40` inspects weapon slot zero. If that
weapon exists and its warhead has `Temporal` at `+0x15A`, it allocates exactly
`0x50` bytes, runs `TemporalClass::Constructor @ 0x0071A4E0`, stores the pointer
at owner `+0x274`, and caches weapon damage at manager `+0x4C`. The constructor
has no other caller. Infantry, Unit, Aircraft, and Building construction paths
reach `Init_Managers`; Building ownership is not excluded.

Thus a stock Chrono Legionnaire owns its manager from Techno initialization,
before it fires. Creation on hit or a free-standing global manager object would
be non-native.

### 3.2 Projectile delivery and exact special dispatch

`TechnoClass::Fire_At @ 0x006FDD50` constructs a real Bullet even for an
`Inviso` projectile. `BulletClass::Fire @ 0x00468670` reveals/unlimbos it and
appends it to the live Logic vector. `BulletClass::AI @ 0x004666E0` resolves a
same-cell `Inviso` shot at its target coordinate and detonates it on the
Bullet's first AI visit.

`BulletClass::DetonateAtCoord @ 0x004690B0` selects one exclusive special
branch. Temporal is after MindControl, Ivan, Electric, and Parasite in that
else-if cascade. The Temporal branch requires both source `+0xB0` and target
`+0x10C`; it calls the source's manager `InitiateWarp`. Because this is an
exclusive special branch, it does **not** execute ordinary area damage or
shrapnel.

After initiation, the common visual tail still runs. For an `Inviso` projectile
it calls the coordinate-scatter helper at `0x0049F420` with `(coord, 0x20, 0)`,
consuming exactly one Scenario RNG byte/draw even when the Temporal warhead has
no `AnimList`. Manager attachment and progression consume no RNG.

### 3.3 Eligibility and attachment

`TemporalClass::CanWarpTarget @ 0x0071AE50` returns false only when:

- the target pointer is null;
- target type `Warpable` at `+0xD3A` is false;
- target virtual `IsInvulnerable` at vtable `+0x160` is true; or
- a Foot target is on a Grinder in the same cell.

There is no health, alive, limbo, ownership, house, range, line-of-sight, or
continued-firing eligibility gate.

`TemporalClass::InitiateWarp @ 0x0071AF20` has observable pre-gate effects. It
destroys the target's Spawn and capture managers before target eligibility is
tested; it also detaches this source manager's previous target before testing
the new target. Attachment proceeds only if the source itself has no incoming
Temporal head at owner `+0x278`.

For a first source:

1. write the manager to target `+0x278`;
2. initialize `warp_points = target.Type.Strength * 10` using signed integer
   arithmetic;
3. perform the first-head radar/EVA/under-attack notification work.

For a later source, insert immediately after the head. A later manager does not
copy or independently initialize the head's warp points. In both cases, set
target `+0x270 = 1`, redraw it, detach the target's own active outgoing
Temporal manager, and leave the target live, mapped, and scheduled. For a
Building, initiation also sets its owner-state dirty flags and starts cloaking.
`IsGattling` causes `UpdateGattlingStage(1)`; it is not Temporal immunity.

### 3.4 Scheduler placement and first decrement

The only progression owner is the warped target's normal leaf update. That
update reads target `+0x278` and calls the incoming head's virtual update. There
is no global Temporal-manager advancement loop; the global Temporal vector is
an ownership/save family.

For a Building, `BuildingClass::Update @ 0x0043FB20` advances Temporal after
sound, damage-fire, and facing maintenance, but before chrono/shimmer/building
animation, common Techno/mission work, health/death processing, delayed fire,
repair, and power work.

`LogicClass::PerTickUpdate @ 0x0055AFB0` re-reads the live vector count and item
while visiting it. A Bullet appended at the tail by a source can therefore run
later in the same pass. The target, however, already occupied a Logic-vector
slot before that newly appended Bullet, so its current-frame target update has
already happened. Initiation occurs in the firing frame and the first manager
decrement occurs on the target's next normal visit: exactly one target update
later.

### 3.5 Per-tick formula and multi-source chain

At every target update, each participating manager asks its current owner for
the currently selected weapon index, resolves that weapon, reads
`WeaponTypeClass+0xA4`, and refreshes manager `+0x4C`. The head applies its own
damage plus the recursively summed damage of successors:

```text
initial_remaining = target_type_strength * 10

for each target update while attached:
    head_damage  = selected_weapon_damage(head.owner)
    chain_damage = sum(selected_weapon_damage(successor.owner), bounded depth)
    remaining    = wrapping_signed_i32_sub(remaining, head_damage + chain_damage)
    if remaining < 1:
        complete_temporal_erase()
```

The chain recursion is entered only when a successor exists and the recursion
depth is below `0x33`. The called successor contributes its own current damage;
the head contribution remains separate. There is no armor/Verses multiplier,
health dependency, ROF gate, range or LOS recheck, continued-fire requirement,
or zero clamp.

For one source the duration is `ceil(Strength * 10 / current weapon Damage)`
target updates. Stock rookie `CLEG` contributes 8 per update, elite contributes
16, and the IFV gunner weapon contributes 5. A weapon-selection change affects
the next sample.

### 3.6 Detach, pointer expiry, and interruption

`TemporalClass::DetachFromTarget @ 0x0071ABC0` preserves a single shared
remaining budget:

- removing the sole head clears target `+0x278`, target `+0x270`, Building
  cloaking/owner dirties, and the removed manager;
- removing a head with a successor promotes the successor, sets its previous
  pointer null, and **copies** the removed head's remaining points into the new
  head;
- removing a non-head splices previous and next without changing the head
  budget.

The promoted value is assignment, not addition.

`TemporalClass::PointerExpired @ 0x0071AB60` detaches when its owner expires; if
its target expires it clears target/chain/auxiliary pointers and sends the owner
to idle. `TechnoClass::PointerExpired @ 0x007077C0` clears an exact incoming head
early and forwards expired pointers to its owned manager. `ObjectClass::UnInit
@ 0x005F65F0` dispatches pointer expiry before limbo/delisting and deferred
deletion. The Temporal destructor removes the manager from the global ownership
vector but does not repair an attached chain, so lifecycle pointer expiry is
load-bearing.

Active cancellation routes are:

- source retargets the manager;
- source calls the common enter-idle helper;
- a Foot source crosses a cell boundary while its movement phase is `2`
  (`FUN_006F5090`); subcell movement alone does not cancel;
- IFV gunner unload transfers the manager back to the passenger and detaches an
  active target first;
- source or target `UnInit`/pointer expiry;
- an open-topped source exceeds `Rules.OpenToppedWarpDistance * 256` in integer
  3D Euclidean distance; equality remains attached.

There is no generic later eligibility recheck. A zero-health source or target
can remain attached until its lifecycle reaches `UnInit`; if the target still
receives its update, progression can complete. Target movement and source
ownership change do not cancel. Source death promotes a successor and copies
the shared remaining budget.

### 3.7 IFV ownership transfer

The Unit gunner board/exit callbacks are instruction regions beginning at
`0x00746420` and `0x007464E0`; the active Ghidra program has no safe function
boundary to rename at those addresses.

Boarding transfers the same manager pointer from passenger `+0x274` to vehicle
`+0x274`, rewrites manager owner `+0x24` to the vehicle, and clears passenger
`+0x274`. It selects the gunner slot first and then applies passenger `IFVMode`.
For `CLEG`, `IFVMode=10` selects IFV `Weapon11=CRNeutronRifle`. Exiting performs
the reverse transfer and owner rewrite; if the manager is attached, it detaches
before transfer. The firing timer/burst preservation surrounding weapon-mode
selection is independent of the Temporal graph but must not be regressed.

### 3.8 Occupied-Building completion transaction

When signed remaining points become less than one,
`TemporalClass::Update @ 0x0071A760` performs:

1. allocate the rules `WarpAway` animation at target coordinates with delay
   zero, loop one, flags `0x600`, z zero, reverse false;
2. if the Building's independent occupant vector count is nonzero, call
   `BuildingClass::SpawnUnitsWithParachute(0)`. The null-source branch walks the
   vector in reverse, calls `UnInit` source-less on every occupant, emits no
   parachute animation and no kill attribution, consumes no Scenario RNG, and
   clears the vector;
3. unless Map Editor mode is active, pop inherited Cargo and call source-less
   `UnInit` on every Cargo object separately;
4. if manager `+0x3C` is non-null, call its `SuperClass::Suspend(0)` and clear it;
5. undock Building `+0x2E4` and clean its slave manager when present;
6. call the Building removal-notification virtual with the Temporal source; for
   the retail Building implementation at `0x0044D760`, emit the radar event only
   when the source is non-null and the Building house is human;
7. call `TechnoClass::RecordKill(source)` for the Building itself;
8. call Building `UnInit`;
9. dirty the Building owner's state at House `+0x1FC`;
10. send the source to idle inside the target-present block, clear manager
    target/link fields, then send the source to idle a second time.

For infantry source, vtable `+0x484` resolves to
`InfantryClass::Enter_Idle_Mode @ 0x0051CBA0`, which reaches
`FootClass::Enter_Idle_Mode @ 0x004D82B0`; for IFV source it resolves to
`UnitClass::Enter_Idle_Mode @ 0x00738970`. The first call's common helper
detaches. A Foot re-entry guard prevents the shared base body from applying
twice in the same frame, while each leaf still performs its mission-selection
tail.

The required order is therefore:

```text
independent occupant vector (reverse, source-less)
    before inherited Cargo (source-less)
    before Building removal notification / Building kill credit
    before Building UnInit
```

Only the Building itself is credited to the Temporal source. This is the exact
production join with the Phase 3 `CanBeOccupied` vector mechanism.

### 3.9 Save, load, checksum, and RNG

`TemporalClass::Save @ 0x0071A700` delegates Abstract save and writes the raw
`0x50`-byte body. `TemporalClass::Load @ 0x0071A660` restores vtables and
swizzles owner, target, previous, next, and auxiliary pointers at
`+0x24/+0x28/+0x40/+0x44/+0x38/+0x3C`. Techno load also swizzles its owned
manager `+0x274` and incoming head `+0x278`. Active progress and graph topology
survive save/load.

Temporal's virtual at `0x0071A650` delegates the Abstract CRC contribution and
does not add its link/progress body. The verified retail ComputeCRC family is
not the live multiplayer per-frame synchronization consumer. A deterministic
Rust engine must nevertheless hash all manager and target-backlink state; that
is an engine-level equivalent, not a reason to omit native-persistent fields.

RNG ledger for the claimed route:

- one Scenario RNG draw for each stock `Inviso` detonation visual coordinate,
  after `InitiateWarp`;
- zero RNG in manager attachment, progression, detach, and occupant/Cargo erase;
- stock `WARPAWAY` has no `RandomRate`, `Bouncer`, or `IsMeteor`, so its
  constructor adds no Scenario RNG draw. Its `Report=ChronoLegionKill` sound is
  synchronous presentation state.

## 4. INI Keys

### 4.1 Active stock binding

| Type / key | Retail value | Effect in claimed route | Evidence |
|---|---|---|---|
| `CLEG.Primary` | `NeutronRifle` | rookie source owns manager and contributes 8/update | `rulesmd.ini` |
| `CLEG.ElitePrimary` | `NeutronRifleE` | elite source contributes 16/update | `rulesmd.ini` |
| `CLEG.IFVMode` | `10` | IFV gunner selects weapon slot 11 | `rulesmd.ini`; Unit callbacks |
| `NeutronRifle.Damage` | `8` | rookie manager decrement | `rulesmd.ini` |
| `NeutronRifleE.Damage` | `16` | elite manager decrement | `rulesmd.ini` |
| `CRNeutronRifle.Damage` | `5` | IFV gunner manager decrement | `rulesmd.ini` |
| all three `.ROF` | `120` | firing cadence only; not manager cadence | `rulesmd.ini`; update formula |
| all three `.Warhead` | `ChronoBeam` | selects Temporal special branch | `rulesmd.ini` |
| all three projectiles | `InvisibleMedium` / `InvisibleLow` | all stock routes use `Inviso`; first Bullet AI detonation and one visual scatter draw | `rulesmd.ini` |
| `ChronoBeam.Temporal` | `yes` | manager allocation and detonation dispatch | `rulesmd.ini` |
| `ChronoBeam.Verses` | ten `100%` entries then `0%` | does not scale Temporal decrement | `rulesmd.ini`; native formula |
| `General.WarpAway` | `WARPAWAY` | completion animation | `rulesmd.ini` / `artmd.ini` |
| `General.OpenToppedWarpDistance` | `7` | compiled open-topped cancellation threshold, in cells before `*256` | `rulesmd.ini`; update branch |
| target type `.Warpable` | default true unless disabled | eligibility | parser/type field; `CanWarpTarget` |
| target type `.Strength` | per type | initializes signed budget `Strength*10` | `InitiateWarp` |

`WARPAWAY` is translucent, ground-layer, flat, `Rate=300`, and
`Report=ChronoLegionKill`; it does not set `RandomRate`, `Bouncer`, or
`IsMeteor`.

### 4.2 Retail activation and exclusions

Only three effective retail weapon definitions bind `Warhead=ChronoBeam`:
`NeutronRifle`, `NeutronRifleE`, and `CRNeutronRifle`. Across the mounted
184-map retail corpus there are zero map-local `Temporal=` assignments and zero
map-local `ChronoBeam` warhead overrides.

`CLEG` is ordinarily buildable by the standard Allied skirmish progression at
TechLevel 10 with `GAPILE` plus `TECH`, including the Smithsonian prerequisite
override. It also appears in five stock map files and nine lines: six
taskforce/member rows in `c2s01md`, `c2s03md`, and `sov07tmd` totaling twelve
members, `CASLAB.SecretInfantry=CLEG` in `c2s03md`, and map-local `CLEG`
sections in `sov02smd` and `sov03umd`.

The IFV route is stock-active because `FV.Gunner=yes` and weapon slot 11 is
`CRNeutronRifle`. A Battle Fortress is not a stock producer: `CLEG` has no
`OpenTransportWeapon` override, whose default is `-1`, and the open-topped
selector requires a nonnegative passenger index. The compiled
`OpenToppedWarpDistance` branch remains relevant to modded Temporal sources but
is unreachable for stock `CLEG` in `BFRT`. IFV gunner mode is not the
open-topped route.

No stock `CanBeOccupied` Building type is also `IsGattling`; the gattling-stage
side effect is therefore excluded from this exact occupied-Building target.
Map Editor Cargo suppression is excluded from ordinary retail play.

`ParasiteClass`/WarpAttach visual manager code at `0x006297F0` belongs to
warhead `Parasite` at `+0x159`, not Temporal at `+0x15A`; `ChronoBeam` therefore
does not inherit SQDG, Wake, rubble, or that mechanism's RNG. Likewise
`TechnoClass::UpdateTemporalVisual @ 0x0070E5A0` is an invulnerability/
Iron-Curtain/Force-Shield visual phase, not Temporal-manager progression.

## 5. Integration Points

| Stage | Native owner / address | Exact responsibility |
|---|---|---|
| Techno construction | `TechnoClass::Init_Managers @ 0x006F3F40` | create the single owner manager from primary weapon's Temporal warhead |
| IFV board/exit | Unit instruction regions `0x00746420` / `0x007464E0` | transfer identical manager and rewrite owner; detach on exit |
| fire | `TechnoClass::Fire_At @ 0x006FDD50` | create even an `Inviso` Bullet |
| logic insertion | `BulletClass::Fire @ 0x00468670` | submit Bullet to live Logic vector |
| first Bullet update | `BulletClass::AI @ 0x004666E0` | same-cell `Inviso` impact on first AI visit |
| detonation | `BulletClass::DetonateAtCoord @ 0x004690B0` | exclusive Temporal initiation, no ordinary damage/shrapnel, then one `Inviso` scatter draw |
| attachment | `TemporalClass::InitiateWarp @ 0x0071AF20` | eligibility, target incoming chain, budget, flags, notifications |
| target update | `BuildingClass::Update @ 0x0043FB20` | advance incoming head after damage-fire/facing and before mission/death tail |
| progression/completion | `TemporalClass::Update @ 0x0071A760` | refresh selected damages, subtract budget, complete erase |
| interruption | `DetachFromTarget @ 0x0071ABC0`; `PointerExpired @ 0x0071AB60` | graph repair and lifecycle cancellation |
| lifecycle expiry | `TechnoClass::PointerExpired @ 0x007077C0`; `ObjectClass::UnInit @ 0x005F65F0` | expire cross-object links before deletion |
| persistence | `TemporalClass::Load/Save @ 0x0071A660/0x0071A700` | preserve manager body and swizzled graph |

## 6. Current Rust Implementation Status

### 6.1 Preserve

- `src/sim/game_entity.rs:316-321` already represents manager remaining points,
  target, previous, and next by stable owner ID; the target backlink and warped
  byte already exist on `GameEntity`.
- `src/sim/snapshot.rs:1440-1600` validates manager graph/backlink invariants,
  and `src/sim/world/world_hash.rs:1742` hashes Temporal state.
- The mixed live-object scheduler already commits a fired projectile into the
  current pass, providing the substrate for native firing-frame initiation and
  next-target-visit first decrement.
- The immediate `Inviso` branch already owns one effect-coordinate scatter
  draw; preserve that draw and fire/effect tail after adding exclusive Temporal
  dispatch.

### 6.2 Replace what is wrong

- `src/sim/capture_manager.rs:405-456` uses Temporal detach as cleanup, but head
  promotion at line 441 adds the removed points to the successor. Native copies
  the removed head's remaining points.
- `src/sim/combat/mod.rs:7072-7330` classifies all three stock Temporal
  projectiles as immediate because they are `Inviso`, then applies ordinary AoE
  damage before the visual scatter. Native initiates Temporal, skips ordinary
  AoE and shrapnel, and still performs the one scatter draw afterward.
- `src/sim/combat/mod.rs:4412-4437` recognizes Temporal only in the persistent
  detonation special classifier and returns it as unsupported. That branch does
  not cover stock `CLEG`; both immediate and persistent delivery must share one
  Temporal action.

### 6.3 Implement what is missing

- No production spawn/type-init path creates a `TemporalManagerState` from the
  owner's primary Temporal weapon.
- No production hit path calls an initiation transaction or creates/repairs the
  incoming manager chain.
- No target-owned update advances that chain. The Building arm in
  `src/sim/world/techno_ai.rs:442-473` has the exact insertion seam immediately
  after `update_building_damage_fire` and before common mission/C4 work.
- Passenger transactions in `src/sim/passenger.rs:620-640`, `905-925`,
  `1198`, and `1337` change `weapon_override` but do not transfer manager
  ownership or detach the active graph on exit.
- The source/target pointer-expiry cleanup is not a complete production
  Temporal lifecycle.
- The planned Phase 3 independent occupant-vector implementation must be called
  by real Temporal completion before inherited Cargo recursion and Building
  `UnInit`; an isolated test helper does not close this prerequisite.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| manager construction and owner identity | verified | `0x006F3F40`, `0x0071A4E0` | none |
| stock `CLEG`/elite/IFV producer bindings | verified | `rulesmd.ini`; Unit callbacks | none |
| BFRT/open-topped stock exclusion | verified | `OpenTransportWeapon=-1` default; selector and distance branch | none inside claimed route |
| Bullet creation/submission/first AI timing | verified | `0x006FDD50`, `0x00468670`, `0x004666E0`, `0x0055AFB0` | none |
| Temporal special cascade and `Inviso` RNG tail | verified | `0x004690B0`, `0x0049F420` | none |
| eligibility and pre-gate side effects | verified | `0x0071AE50`, `0x0071AF20` | none |
| incoming chain insertion and shared budget | verified | `0x0071AF20`, `0x0071ABC0` | none |
| target-owned Building scheduler seam | verified | `0x0043FB20`, `0x0055AFB0` | none |
| decrement formula, selected weapon refresh, depth bound | verified | `0x0071A760` and recursive damage helper | none |
| source death, target expiry, retarget, movement, IFV unload, range cancellation | verified | `0x0071AB60`, `0x0071ABC0`, `0x006F5090`, Unit callbacks, update distance branch | none |
| occupied-vector reverse source-less erase | verified | `0x0071A760`; occupant-vector report §7 | none |
| inherited Cargo then Building kill/UnInit order | verified | `0x0071A760`; Cargo and occupant-vector reports | none |
| WarpAway construction and route RNG | verified | `0x0071A760`; `artmd.ini` | pixel composition outside claimed scope |
| save/load pointer restoration | verified | `0x0071A660`, `0x0071A700`; Techno load | none |
| native checksum consumer boundary | verified | `0x0071A650`; multiplayer sync research index | none |
| active map/rules census and exclusions | verified | mounted `rulesmd.ini`, `artmd.ini`, 184-map corpus | none |
| current Rust production reachability | verified | Rust sources cited in §6 | implementation absent |
| non-Building completion branches | deferred | bounded prerequisite is occupied Building | follow-up only if a later row claims all Temporal target classes |
| WarpAway pixel/frame composition | deferred | no Phase 3 logic dependency | visual parity investigation if requested |

### Exhaustion record

The zero-add pass re-decompiled the producer, detonation, eligibility,
initiation, target update, detach, pointer-expiry, completion, save, load, and
Logic-scheduler roots after the question log drained. It added no new
load-bearing question.

Two cold-spot passes were also performed. Re-reading `TemporalClass::Update @
0x0071A760` corrected the completion ledger to distinguish reverse source-less
occupant-vector erasure from inherited Cargo and to retain the two source-idle
calls. Re-reading `BulletClass::DetonateAtCoord @ 0x004690B0` corrected the
current-Rust diagnosis: all stock Chrono weapons are `Inviso`, so the active
gap is primarily the immediate delivery branch, whose visual RNG tail remains
after initiation and whose ordinary AoE/shrapnel must be suppressed.

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] T-01 — What creates an active Temporal manager? → Techno construction inspects primary weapon slot zero and allocates one manager when its warhead is Temporal; hit-time allocation is non-native.` (evidence: `0x006F3F40`, `0x0071A4E0`)
- `[RESOLVED] T-02 — Does a stock invisible Chrono shot bypass Bullet lifecycle? → No; it creates and submits a Bullet, whose first AI visit detonates it.` (evidence: `0x006FDD50`, `0x00468670`, `0x004666E0`)
- `[RESOLVED] T-03 — Does Temporal also apply ordinary damage or shrapnel? → No; it is one exclusive detonation branch.` (evidence: `0x004690B0`)
- `[RESOLVED] T-04 — What exact target gates apply? → non-null, Warpable, not virtual-invulnerable, and not a Foot on a same-cell Grinder; there is no general health/range/LOS gate.` (evidence: `0x0071AE50`)
- `[RESOLVED] T-05 — Are Spawn/capture cleanup effects conditional on eligibility? → No; they occur before the target eligibility test.` (evidence: `0x0071AF20`)
- `[RESOLVED] T-06 — Is the target limboed while erasure progresses? → No; it remains live, mapped, and target-scheduled with its warped flag set.` (evidence: `0x0071AF20`, `0x0043FB20`)
- `[RESOLVED] T-07 — Who advances the manager? → only the warped target's leaf update via its incoming head; the global manager vector does not schedule progress.` (evidence: `0x0043FB20`, `0x0071A760`)
- `[RESOLVED] T-08 — When is the first decrement relative to firing? → on the target's next normal update, because the newly appended Bullet detonates after the target's existing slot was visited.` (evidence: `0x0055AFB0`, fire/Bullet chain)
- `[RESOLVED] T-09 — What is the exact formula? → initialize Strength*10, subtract each chain owner's currently selected weapon Damage once per target update using signed integer arithmetic, complete at <1.` (evidence: `0x0071AF20`, `0x0071A760`)
- `[RESOLVED] T-10 — Do armor, Verses, ROF, health, range, LOS, or continued firing affect progress? → No.` (evidence: `0x0071A760`)
- `[RESOLVED] T-11 — How are later attackers represented? → inserted after the head; they contribute selected weapon damage but do not receive an independent budget.` (evidence: `0x0071AF20`, chain-sum helper)
- `[RESOLVED] T-12 — What happens when the head disappears? → promote successor and copy the removed head's remaining budget; do not add budgets.` (evidence: `0x0071ABC0`)
- `[RESOLVED] T-13 — Does source death necessarily cancel the whole erase? → sole source detaches; with successors, lifecycle pointer expiry promotes one and preserves the shared remaining budget.` (evidence: `0x0071AB60`, `0x0071ABC0`, `0x005F65F0`)
- `[RESOLVED] T-14 — Does a zero-health target stop progress before deletion? → not by a manager eligibility recheck; it can complete if the target still receives its update before UnInit.` (evidence: `0x0071AE50`, `0x0071A760`, `0x005F65F0`)
- `[RESOLVED] T-15 — Does target movement cancel? → No; only the listed source movement/cell-phase, idle/retarget, IFV exit, pointer expiry, and open-topped distance paths detach.` (evidence: detach caller census)
- `[RESOLVED] T-16 — How does IFV gunner mode preserve manager ownership? → pointer transfer passenger→vehicle on board and vehicle→passenger on exit, with owner rewrite and active detach before exit.` (evidence: `0x00746420`, `0x007464E0` instruction regions)
- `[RESOLVED] T-17 — Can stock CLEG fire Temporal from a Battle Fortress? → No; no nonnegative OpenTransportWeapon index exists for CLEG. The compiled distance branch is mod-conditional, not a stock producer route.` (evidence: `rulesmd.ini`; selector branch)
- `[RESOLVED] T-18 — What happens to occupied Building contents on completion? → reverse source-less independent-vector UnInit, then separate source-less inherited Cargo UnInit, then Building notification/kill/UnInit.` (evidence: `0x0071A760`; three Phase 3 Building reports)
- `[RESOLVED] T-19 — Are occupants credited as source kills? → No; only the Building receives source attribution.` (evidence: completion call order at `0x0071A760`)
- `[RESOLVED] T-20 — Is IsGattling an immunity? → No; initiation advances its gattling stage. No stock CanBeOccupied Building is IsGattling.` (evidence: `0x0071AF20`; retail census)
- `[RESOLVED] T-21 — How much Scenario RNG does the stock route consume? → exactly one draw in the Inviso visual-coordinate tail per detonation; zero in attachment/progression/vector/Cargo erase and zero extra from stock WARPAWAY construction.` (evidence: `0x004690B0`, `0x0049F420`, `artmd.ini`)
- `[RESOLVED] T-22 — Does save/load preserve an in-progress multi-source chain? → Yes; manager body, owner/target/previous/next/auxiliary pointers, and Techno owner/head pointers are serialized and swizzled.` (evidence: `0x0071A660`, `0x0071A700`; Techno load)
- `[RESOLVED] T-23 — Does native manager CRC omission authorize Rust hash omission? → No; retail's Abstract-only manager CRC is not the live MP frame consumer, while Rust must hash deterministic simulation state.` (evidence: `0x0071A650`; project sync research index)
- `[RESOLVED] T-24 — What current production Rust path reaches occupied-Building erase? → None; stock shots enter immediate ordinary AoE, persistent Temporal returns unsupported, and no producer/updater exists.` (evidence: current Rust §6)
- `[RESOLVED] T-25 — Could a helper-only completion test close the Phase 3 row? → No; acceptance must fire the real stock weapon through production delivery, attachment, target-owned updates, and completion.` (evidence: critic requirement; native integration chain)
- `[RESOLVED] T-26 — Is Parasite/WarpAttach visual code part of Chrono erasure? → No; it is gated by adjacent Parasite warhead state, not Temporal.` (evidence: `0x006297F0`; warhead offsets)
- `[RESOLVED] T-27 — Does UpdateTemporalVisual advance the erasure manager? → No; it owns invulnerability/Iron-Curtain/Force-Shield visual timing.` (evidence: `0x0070E5A0`)
- `[DEFERRED] T-28 — What are the complete completion transactions for every non-Building target subclass?` (category: `out-of-scope`; reason: `not required to make the Phase 3 occupied-Building vector route production-reachable`; next-step-if-pursued: `run a separate exhaustive Temporal target-class completion investigation`)
- `[DEFERRED] T-29 — What is pixel-exact WARPAWAY draw composition?` (category: `out-of-scope`; reason: `the Phase 3 blocker is deterministic simulation reachability and lifecycle order`; next-step-if-pursued: `trace animation palette/layer/frame rendering separately`)

### Adversarial reader test

The questions “what if the source dies,” “what if the target reaches zero
health,” “what if two sources are chained and the head exits,” “what if the
game is saved mid-erasure,” “what if `CLEG` boards or leaves an IFV,” “what if
the source moves out of range,” and “does a zero-damage selection stall without
special-casing” are resolved by T-10 through T-22. The zero-damage case follows
the exact signed subtraction formula: a zero contribution changes no points
that update; no synthetic minimum is applied.

## 9. Visual/UI Composition Ledger

This report does not claim pixel/frame composition. The only visual surface
needed for deterministic behavior is the verified event/RNG boundary:

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `BulletClass::DetonateAtCoord @ 0x004690B0`, scatter `0x0049F420` | projectile `Inviso=yes` | no required `AnimList` | target coordinate scattered within native `0x20` argument | outside scope | yes | consumes one Scenario RNG draw after initiation |
| 2 | `TemporalClass::Update @ 0x0071A760` | completion at remaining `<1` | `WARPAWAY` | target coordinate | outside scope | yes | completion presentation and `ChronoLegionKill` report |

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `WARPAWAY` | yes | yes on completion | yes | outside claimed pixel scope | no | world animation | yes | no | rules/art binding and `0x0071A760` |
| ChronoBeam warhead `AnimList` | none in stock route | no | no | n/a | no | no | no | yes | `rulesmd.ini`; visual tail still scatters coordinate |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| one persistent manager created from primary Temporal weapon | `0x006F3F40`, `0x0071A4E0` | missing | Techno spawn/type initialization; `GameEntity::temporal_manager` | create detached manager at production object initialization and cache/derive active weapon damage as required | spawned stock `CLEG` has detached manager before firing; ordinary infantry does not | do not allocate only on hit or only in tests |
| IFV transfers the same manager and rewrites owner | Unit regions `0x00746420/0x007464E0` | missing | `src/sim/passenger.rs` board/exit transactions | move manager passenger↔vehicle atomically; detach before exit; preserve existing gunner weapon/timer semantics | CLEG boards IFV, fires `CRNeutronRifle`, exits during active erase, graph cancels and manager returns | do not clone manager or treat IFV as open-topped |
| stock Inviso shot initiates Temporal and skips damage | `0x004690B0` | wrong immediate AoE; persistent unsupported | `src/sim/combat/mod.rs` immediate and persistent detonation ownership | add one shared Temporal special action; immediate stock route initiates before effects, emits no ordinary AoE/shrapnel, retains one scatter draw and fire tail | stock rookie CLEG fires at healthy occupied Building: health unchanged at hit, manager attached, RNG advances once | do not wire only persistent projectile path; do not suppress visual RNG |
| eligibility and pre-gate cleanup are ordered | `0x0071AE50`, `0x0071AF20` | missing | world-side Temporal initiation transaction | destroy target spawn/capture managers and detach old target before eligibility; apply exact gate and source-incoming exclusion | failed unwarpable/invulnerable target still observes pre-gate cleanup/old-target detach | do not reorder cleanup behind eligibility |
| first update occurs next target visit | `0x0055AFB0`, `0x0043FB20` | missing | mixed live scheduler and `techno_ai` Structure arm | attach during Bullet visit; advance target head immediately after Building damage-fire on next target visit | firing-frame snapshot shows full `Strength*10`; next Building visit subtracts 8/16/5 | do not create global manager loop or decrement synchronously on hit |
| exact shared signed budget and chain damage | `0x0071AF20`, `0x0071A760` | missing | Temporal manager graph/update | sample each owner's selected weapon Damage each target update, bounded successor sum, wrapping signed subtraction, complete `<1` | two attackers sum current damages; weapon-mode change affects next tick; zero damage stalls | do not apply Verses/armor/ROF/range or clamp |
| detach promotes by copying remaining | `0x0071ABC0` | Rust adds values | `src/sim/capture_manager.rs` Temporal cleanup or dedicated module | centralize native graph detach; promotion assigns removed head's remaining points | head source death with successor preserves exactly one remaining value and all backlinks | do not add successor-local points |
| pointer expiry repairs graph before deletion | `0x0071AB60`, `0x007077C0`, `0x005F65F0` | partial cleanup only | object lifecycle/UnInit | expire owner/target links through the shared detach transaction before object removal | source UnInit, target UnInit, and target-first deletion leave no stale IDs and match snapshot invariants | do not rely on manager destructor |
| occupied vector drains before Cargo and Building UnInit | `0x0071A760`; three Phase 3 Building reports | planned tail not production-reachable | new independent occupant-vector owner plus existing Cargo/lifecycle | completion reverse-UnInits vector source-less, clears it, drains Cargo source-less, attributes Building only, then Building UnInit | production stock CLEG attack on occupied Building exercises real shot→manager→updates→completion and asserts trace order | do not use Cargo as vector storage; do not pass source to occupant erase |
| active graph persists and hashes | `0x0071A660/0x0071A700`; Rust snapshot/hash | representation exists; production roundtrip untested | snapshot validation and world hash | preserve manager/target graph fields, progress, and owner transfer across save/load; hash every deterministic field | save/load mid-chain produces identical next-step state/hash and completion frame | do not mimic native Abstract-only CRC by omitting fields |

### Smallest architecture-compatible prerequisite

The smallest real prerequisite is one production-owned Temporal subsystem using
the existing stable-ID entity graph and live-object scheduler:

1. create the manager at normal Techno initialization from primary weapon
   Temporal binding;
2. transfer that manager in existing IFV passenger transactions;
3. expose one world-side initiate/detach/update transaction shared by immediate
   and persistent projectile detonation;
4. call target-head update from the target's existing leaf update, with the
   Building seam after damage-fire and before mission/C4;
5. route completion into the independent occupant-vector owner, then Cargo and
   Building lifecycle in native order;
6. reuse snapshot validation/world hashing and repair the copy-vs-add detach
   mismatch.

It does not require a new global scheduler, a parallel object store, or a
test-only completion helper.

### Required acceptance suite

1. **Production stock route:** rookie `CLEG` fires `NeutronRifle` at an occupied
   Building. The shot uses immediate `Inviso` delivery, performs no ordinary
   damage, consumes one scatter draw, creates the target chain, and decrements
   only on the next Building visit.
2. **Exact duration:** single rookie, elite, and IFV gunner cases complete after
   `ceil(Strength*10/8)`, `/16`, and `/5` target updates respectively.
3. **Occupied completion trace:** reverse vector occupant source-less `UnInit`
   precedes Cargo source-less `UnInit`, Building source notification/kill, and
   Building `UnInit`; only Building is source-credited.
4. **Multi-source:** later source contributes damage, head removal promotes it
   with copied remaining points, and backlinks remain valid.
5. **Interruptions:** retarget, enter-idle, source cell-cross phase 2, source
   UnInit, target UnInit, IFV exit, and open-topped `distance > 7*256` each take
   their exact detach route; equality does not detach.
6. **No recheck:** post-init target movement, source ownership change, range,
   LOS, ROF, and stopped firing do not alter progression.
7. **Eligibility side effects:** invalid/unwarpable/invulnerable attempt proves
   Spawn/capture cleanup and prior-target detach happen before the failed gate.
8. **Zero contribution:** selected Damage zero leaves signed remaining unchanged
   without a minimum decrement.
9. **Persistence:** save/load during a multi-source occupied-Building erase
   preserves progress, chain topology, target backlink/flag, next hash, and
   completion frame.
10. **Retail producer exclusions:** Battle Fortress does not acquire stock CLEG
    Temporal output; IFV does; no map-local Temporal override changes the stock
    route.

### Stale Docs / Follow-up Docs

The following prior claims are superseded for this slice:

- Replace “Temporal managers advance from a global manager update” with “the
  warped target advances its incoming head from its leaf update; the global
  vector owns/saves managers.”
- Replace “a later chain node copies/initializes the head budget on attach” with
  “later nodes have no independent budget; head promotion copies the current
  head remaining points at detach.”
- Replace “Temporal initiation limbos the target” with “the target remains
  live, mapped, and scheduled until completion or normal lifecycle removal.”
- Replace “`IsGattling` is Temporal immunity” with “initiation calls
  `UpdateGattlingStage(1)`.”
- Replace “ChronoBeam uses Parasite/WarpAttach (`0x006297F0`) effects” with
  “that manager is gated by Parasite, not Temporal.”
- Replace “stock CLEG can erase from a Battle Fortress” with “stock CLEG has
  default `OpenTransportWeapon=-1`; IFV gunner mode is the active transport
  producer.”
- Replace swapped Bullet source/target offsets with source `+0xB0`, target
  `+0x10C`.

Follow-up implementation documentation must make test 86 exercise the full
production chain, not call the completion tail directly.

## 11. Ghidra Annotation Candidates

No Ghidra metadata was mutated during this read-only investigation.

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x0071AB60` | `FUN_0071AB60` | `TemporalClass::PointerExpired` | rename | body independently handles expired owner and expired target, repairs links, and idles owner; caller context is pointer-expiry dispatch | worker-report-only |
| `0x00746420` | instruction region without safe function boundary | comment: IFV gunner boarding transfers Temporal manager passenger→vehicle and rewrites owner | comment | pointer moves `+0x274`, manager `+0x24` rewrite, passenger clear, followed by gunner selection | deferred |
| `0x007464E0` | instruction region without safe function boundary | comment: IFV gunner exit detaches active Temporal target then transfers manager vehicle→passenger | comment | detach call plus inverse pointer move/owner rewrite | deferred |

The two Unit regions are comment candidates only; inventing function boundaries
would not pass the project certainty gate.

## Sources

- Active-retail Ghidra program: mounted `gamemd.exe`; read-only decompilation and
  disassembly of `0x0043FB20`, `0x004666E0`, `0x00468670`, `0x004690B0`,
  `0x0049F420`, `0x0051CBA0`, `0x0055AFB0`, `0x005F65F0`, `0x006F3F40`,
  `0x006F5090`, `0x006FDD50`, `0x007077C0`, `0x0070E5A0`, `0x0071A4E0`,
  `0x0071A650`, `0x0071A660`, `0x0071A700`, `0x0071A760`, `0x0071AB60`,
  `0x0071ABC0`, `0x0071AE50`, `0x0071AF20`, `0x00738970`, and Unit instruction
  regions `0x00746420`/`0x007464E0`.
- Retail data: mounted `rulesmd.ini`, `artmd.ini`, and 184-map campaign/skirmish
  corpus.
- `docs/research/PHASE3_BUILDING_CAN_BE_OCCUPIED_VECTOR_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/PHASE3_BUILDING_SPAWN_SURVIVORS_CARGO_GHIDRA_REPORT.md`
- `docs/research/PHASE3_BUILDING_EXPLODES_LIFECYCLE_GHIDRA_REPORT.md`
- Project research index and relevant Temporal/Chrono research documents,
  treated as hypotheses and corrected where live evidence disagreed.
- Current Rust: `src/sim/game_entity.rs`, `src/sim/capture_manager.rs`,
  `src/sim/combat/mod.rs`, `src/sim/passenger.rs`,
  `src/sim/world/techno_ai.rs`, `src/sim/snapshot.rs`, and
  `src/sim/world/world_hash.rs`.
