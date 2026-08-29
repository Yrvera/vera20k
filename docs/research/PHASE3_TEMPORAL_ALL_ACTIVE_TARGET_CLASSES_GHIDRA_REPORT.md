# Phase 3 Temporal Completion for All Active Target Classes — Active-Retail Ghidra Research Report

**Address(es):** `TemporalClass::Update @ 0x0071A760` (primary); `TemporalClass::InitiateWarp @ 0x0071AF20`; `TemporalClass::CanWarpTarget @ 0x0071AE50`; `ObjectClass::UnInit @ 0x005F65F0`; `DrainDeferredFinalizationQueue @ 0x00725C70`  
**Investigation mode:** exhaustive-slice  
**Claimed scope:** every active-retail completion transaction and prerequisite side effect for a stock Chrono Legionnaire, elite Chrono Legionnaire, or Chrono Legionnaire-gunner IFV targeting any reachable concrete `TechnoClass` leaf: `InfantryClass`, `UnitClass`, `AircraftClass`, or `BuildingClass`  
**Non-scope:** custom/mod-only Temporal producers and type overrides; Map Editor-only behavior; pixel-exact `WARPAWAY` rendering after its deterministic construction boundary  
**Confidence:** HIGH  
**Active in YR:** Yes — ordinary stock `CLEG` combat, elite `CLEG` combat, and the `CLEG` IFV-gunner mode all produce Temporal hits

## 1. Executive Summary

The active target set is exhaustively four concrete runtime leaves. Native RTTI,
`WhatAmI`, the deferred-delete type-cast sequence, retail type registries, and
the 184-map corpus independently agree on `InfantryClass`, `UnitClass`,
`AircraftClass`, and `BuildingClass`. Ships are `UnitClass`; no Terrain, Cell,
Smudge, Bullet, Anim, or other non-Techno object can enter this weapon-target
transaction.

`TemporalClass::Update @ 0x0071A760` has exactly one target-class split at
completion: `BuildingClass` (`WhatAmI == 6`) versus every non-Building Techno.
The Building branch owns the independent `CanBeOccupied` vector and inherited
Cargo prelude. The generic branch owns linked bunker/dock teardown. Both then
liberate slaves when the erased target owns a `SlaveManager`, issue the target
class notification, call `RecordKill(source)`, and call the class's normal
`UnInit`. Normal `UnInit` and `Limbo` perform the extensive class-specific
removal: capture release, nested Cargo destruction, pending ChronoWarp and Team
cleanup, pointer expiration, map/layer/bridge occupancy, house accounting,
building power/factory/base/wall effects, and deferred physical deletion.

This is a production subsystem, not four bespoke deletion helpers. The correct
Rust architecture is one stock weapon-to-manager route, one target-owned update
seam used by all four entity categories, one category-aware completion prelude,
and the existing central lifecycle owner for shared and derived removal. Current
Rust has persistent graph-shaped fields but no production creator, initiation,
or updater; its Temporal detonation is rejected as unsupported. The subsystem
is therefore not implemented, but the all-active-target behavior needed to
implement it now has no open evidence question inside the claimed stock scope.

## 2. Data Structures and Runtime-Class Census

### 2.1 Temporal and target fields

The occupied-Building prerequisite report verifies the complete `0x50`-byte
`TemporalClass` layout. The fields load-bearing for all-target completion are:

| Offset | Type | Purpose | Evidence |
|---:|---|---|---|
| `Temporal+0x24` | `TechnoClass*` | owner/source | constructor, load/save, update |
| `Temporal+0x28` | `TechnoClass*` | current target | initiation, detach, update |
| `Temporal+0x38` | pointer | optional auxiliary link; null on stock CLEG routes | load/save, completion |
| `Temporal+0x3C` | `SuperClass*` | optional Super link; suspended and cleared at completion | `0x0071A760` |
| `Temporal+0x40/+0x44` | `TemporalClass*` | previous/next incoming managers | initiation, detach, load |
| `Temporal+0x48` | signed `i32` | shared warp points remaining on chain head | initiation/update/detach |
| `Temporal+0x4C` | signed `i32` | selected-weapon damage refreshed during progress | `0x0071A760` |
| `Techno+0x270` | flag/state | target's Temporal visual/warped state | initiation/detach |
| `Techno+0x274` | `TemporalClass*` | manager owned by this Techno | `TechnoClass::Init_Managers @ 0x006F3F40` |
| `Techno+0x278` | `TemporalClass*` | incoming chain head | initiation, Techno pointer expiry |
| `Techno+0x2AC` | `BuildingClass*` | pending ChronoWarp/deploy relationship on Foot targets | `FootClass::UnInit @ 0x004DE5D0` |
| `Techno+0x2BC` | `CaptureManagerClass*` | outgoing mind-control captures | initiation and Foot UnInit |
| `Techno+0x2C0` | pointer | incoming mind-controller relationship | capture release/pointer cleanup |
| `Techno+0x2D0` | `SpawnManagerClass*` | outgoing spawned-unit ownership | initiation pre-clean |
| `Techno+0x2D8` | `SlaveManagerClass*` | outgoing slave ownership | completion |
| `Techno+0x2DC` | pointer | slave's owner-manager backlink | Infantry destruction tail |
| `Techno+0x2E4` | `TechnoClass*` | bunker/dock reciprocal link | detonation redirection/completion |
| `Techno+0x114/+0x118` | Cargo head/count | inherited Cargo list | Building prelude and Object UnInit |
| `Building+0x684..+0x698` | vector | independent `CanBeOccupied` occupant vector | Building completion branch |
| `Foot+0x5D4` | `TeamClass*` | Team membership | `FootClass::UnInit` |
| `TechnoType+0xD3A` | bool | `Warpable=` | constructor/parser/eligibility |

### 2.2 Exhaustive concrete target set

| Leaf | Vtable | RTTI type descriptor | `WhatAmI` proof | Completion notification | `UnInit` | `Limbo` |
|---|---:|---:|---|---:|---:|---:|
| `AircraftClass` | `0x007E22A4` | `0x00817B90`, `.?AVAircraftClass@@` | `0x0041C180` returns `2` | Foot region `0x004D98C0` | `0x004DE5D0` | Foot `0x004DB260` |
| `BuildingClass` | `0x007E3EBC` | `0x00818D60`, `.?AVBuildingClass@@` | `0x00459EC0` returns `6` | `0x0044D760` | Object `0x005F65F0` | `0x00445880` |
| `InfantryClass` | `0x007EB058` | `0x00825508`, `.?AVInfantryClass@@` | `0x00523340` returns `0xF` | Foot region `0x004D98C0` | `0x004DE5D0` | `0x0051DF10` |
| `UnitClass` | `0x007F5C70` | `0x00842D80`, `.?AVUnitClass@@` | `0x00746E20` returns `1` | Foot region `0x004D98C0` | `0x004DE5D0` | `0x007440B0` |

`DrainDeferredFinalizationQueue @ 0x00725C70` independently attempts its four
runtime casts against exactly these leaves before calling the scalar deleting
destructor. `TechnoClass` is abstract. Naval ships use `UnitClass`, not a fifth
ship leaf. All other active object families lack Techno weapon-target state and
cannot be passed through `TemporalClass::InitiateWarp` as a legal attack target.

### 2.3 Effective stock `Warpable` census

`TechnoTypeClass::Constructor @ 0x00710AF0` initializes
`TechnoType+0xD3A = true`. `TechnoTypeClass::ReadINI` at
`0x00714F5E..0x00714F79` reads `Warpable` from string `0x00843778` using that
current value as the default.

There are no `Warpable=` assignments in retail `rules.ini`, `rulesmd.ini`, or
the extracted 184-map corpus. Effective YR `rulesmd.ini` declares 65 infantry,
80 vehicle, 12 aircraft, and 403 building types: 560 declared Techno types, all
inheriting `Warpable=yes`. Thus stock data excludes no Techno type through this
flag. Ordinary weapon-fire legality and runtime invulnerability still apply.

The same map corpus contains direct placements in every leaf category:

| Placement section | Maps | Records |
|---|---:|---:|
| `[Infantry]` | 55 | 1,710 |
| `[Units]` | 60 | 1,576 |
| `[Aircraft]` | 2 | 7 |
| `[Structures]` | 175 | 11,992 |

Dynamic production broadens this occurrence set. The corpus has no
`Temporal=`, `Warhead=ChronoBeam`, or `Warpable=` map override.

## 3. Core Logic

### 3.1 Stock producers and delivery

The only stock Temporal weapons are `NeutronRifle`, `NeutronRifleE`, and
`CRNeutronRifle`, all using `ChronoBeam` and `InvisibleMedium` or
`InvisibleLow`. They are the rookie CLEG, elite CLEG, and CLEG IFV-gunner
routes. `TechnoClass::Init_Managers @ 0x006F3F40` allocates one persistent
manager from the owner's primary weapon when its warhead has `Temporal=yes`.
The IFV board/exit instruction regions at `0x00746420` and `0x007464E0`
transfer that same manager passenger-to-vehicle and back and rewrite its owner;
they do not clone it. IFV exit detaches an active target first.

All three projectiles are `Inviso`. A real Bullet is nevertheless created,
submitted to Logic, and detonated on its first Bullet AI visit. The Temporal
special branch in `BulletClass::DetonateAtCoord @ 0x004690B0` initiates the
manager and suppresses ordinary area damage and shrapnel. Its later Inviso
visual-coordinate tail consumes exactly one Scenario RNG draw.

A stock Battle Fortress route is excluded: CLEG has no nonnegative
`OpenTransportWeapon` selection. The compiled open-topped Temporal-distance
branch remains relevant to modded producers but is not a stock producer.

### 3.2 Fire legality, eligibility, and pre-gate mutation

`TemporalClass::CanWarpTarget @ 0x0071AE50` rejects exactly:

- a null target;
- a target whose type has `Warpable=false`;
- a target whose virtual `IsInvulnerable` returns true; or
- a Foot target standing on a Grinder in the same cell.

It does not gate on health, alive state, limbo, ownership, alliance, range,
line of sight, armor, Verses, or continued firing.

`TemporalClass::InitiateWarp @ 0x0071AF20` performs observable work before this
eligibility call. It first destroys the proposed target's outgoing spawned
units through `SpawnManagerClass::Kill_All_Spawns @ 0x006B7100`, frees its
outgoing captured victims through `CaptureManagerClass::FreeAll @ 0x00472140`,
and detaches the source manager's former target. Consequently even a proposed
target later rejected as unwarpable, invulnerable, or on a Grinder has already
lost those outgoing relationships.

Spawn cleanup reverse-scans its controls. Idle/dead-slot children are UnInit;
children in active flight are marked retreating and sent to the manager's
retreat coordinate. Capture `FreeAll` reverse-calls `FreeUnit`. Each victim's
release animation is removed, release sound is played, original ownership is
restored, the victim's Team and outgoing Temporal relationship are cleared,
open-topped passenger targets are reset, and the incoming controller link is
cleared. A human-owned released victim consumes no fate roll; each AI-owned
released victim consumes exactly one `RandomRanged(1,100)` and selects the
native Rules-driven fate. These effects and draws occur even if the new
Temporal attachment fails its later eligibility test.

An already-airborne Aircraft is not a stock legal target. Both stock invisible
projectile types inherit `AA=no, AG=yes` from
`BulletTypeClassConstructorDefaults @ 0x0046BBC0`; `TechnoClass::GetFireError @
0x006FC0B0` rejects a target whose airborne/display-layer state requires AA.
A landed Aircraft is reachable. If it takes off after attachment, the manager
does not recheck altitude and completes through the same generic Foot branch.

### 3.3 Attachment and progression

The first source becomes the target's incoming head and initializes signed
remaining points to `target.Type.Strength * 10`. Additional sources join its
doubly linked manager chain and contribute damage to that one shared budget.
The target remains live, mapped, targetable, and scheduled during erasure.

Only the target's incoming head advances the chain, once on the target's next
normal update. A shot whose Bullet is appended later in the frame therefore
does not decrement synchronously at hit time. Each target visit refreshes every
source manager's contribution from that source's currently selected weapon and
subtracts the sum, using signed integer arithmetic. The native successor walk
has a depth bound of `0x33`. Completion occurs when the head's signed remaining
value is `< 1`.

| Input | Effect on progression |
|---|---|
| target type Strength | initializes `Strength * 10` |
| each chain source's selected Weapon Damage | subtracted once per target visit |
| armor / `ChronoBeam.Verses` | no effect |
| target current health | no effect |
| source ROF / range / LOS / current firing | no effect |
| selected Damage `0` | zero contribution; no minimum decrement |
| target movement or altitude change | no cancellation or formula change |

Changing CLEG/elite/IFV weapon mode affects the next sampled contribution. A
source ownership change does not cancel. A zero-health source or target remains
attached until normal lifecycle reaches pointer expiration; if the target still
receives its update first, completion can win the ordering race.

### 3.4 Detachment and invalidation

`TemporalClass::DetachFromTarget @ 0x0071ABC0` maintains a single shared
remaining value. Removing the sole source clears the target head and warped
state. Removing the head with a successor promotes that successor and assigns
the removed head's remaining value to it; it never adds two budgets. Removing a
non-head only splices links.

Active detach routes are source retarget/idle transitions, the source
cell-cross phase, source or target pointer expiration/`UnInit`, IFV exit, and
the compiled open-topped distance branch. The latter cancels only at strictly
greater than `Rules.OpenToppedWarpDistance * 256`; equality stays attached.
Target movement, range, LOS, stopped firing, alliance, and later eligibility
changes do not detach.

`TemporalClass::PointerExpired @ 0x0071AB60`,
`TechnoClass::PointerExpired @ 0x007077C0`, and
`ObjectClass::UnInit @ 0x005F65F0` repair the graph before physical deletion.
Source death with another attacker promotes the successor and preserves the
shared remaining value; sole-source death cancels the erase. Target expiration
clears all attached source-manager target links and idles their owners.

### 3.5 Common completion prefix

When the signed remaining value becomes `<1`, `TemporalClass::Update @
0x0071A760` executes this prefix:

1. Construct Rules `WarpAway` at the target's current coordinate with delay
   zero, loop one, flags `0x600`, z zero, reverse false.
2. If the source type is Trainable, award explicit Temporal veterancy before
   the class split. Let `T` be target actual cost for the target owner and `S`
   source actual cost for the source owner. Add
   `f32(T) / (f32(S) * Rules.VeteranRatio)` to source veterancy, store through
   native `f32` rounding, and clamp to `Rules.VeteranCap`.
3. Branch on target `WhatAmI == 6`: Building completion or generic Techno
   completion.

The explicit pre-award ignores victim `DontScore` and alliance. The later
ordinary `RecordKill` path can award the source again. Its normal award is
suppressed by victim `DontScore` and reduced to zero for allied victims, but
the explicit Temporal award remains. Tests must preserve this seemingly double
award rather than deduplicate it.

### 3.6 Building completion transaction

The exact Building branch is:

1. If `Building+0x684` occupant-vector count is nonzero, call
   `BuildingClass::SpawnUnitsWithParachute(0)`. Its null-source branch reverse
   walks the independent vector and calls each occupant's `UnInit` source-less;
   no parachute, scatter, or survivor RNG occurs.
2. Outside Map Editor, pop inherited Cargo from `Techno+0x114/+0x118` and call
   each passenger's `UnInit` source-less. The independent vector is already
   empty.
3. If manager `Super+0x3C` exists, call `Suspend(0)` and clear it. It is null on
   the stock CLEG route.
4. If `Building+0x2E4` links a docked/bunkered Unit, call
   `BuildingClass::UndockUnit @ 0x004593A0`. The linked Unit survives: its
   locomotor is stopped, it is headed toward the native offset coordinate at
   speed 1.0, reciprocal links are cleared, and the Building sends radio break.
5. If `Building+0x2D8` owns a SlaveManager, call its `MasterDestroyed` routine
   `0x006B0AE0` with the Temporal source.
6. Call Building notification `0x0044D760(source)`.
7. Call `TechnoClass::RecordKill @ 0x00702D40(source)`.
8. Call Object `UnInit @ 0x005F65F0` through the Building vtable.
9. Mark `target.Owner+0x1FC = 1`.

The notification only acts when source is non-null and the Building owner is
human. It converts the target coordinate to a cell and calls `0x00660B80`,
advancing the eight-entry radar/spacebar navigation ring. It does not create a
Foot-style radar event or play `EVA_UnitLost`.

Building `Limbo @ 0x00445880`, reached from `UnInit`, owns the rest of the
ordinary removal transaction: eight owned animation slots, owner building/type
counts, wall disconnection, base reservations, upgrades, wall/overlay-neighbor
cleanup, full foundation-rim cell occupation, tactical dirties, remaining field
cleanup, house recount/base-center/sidebar changes, and ordinary power/factory/
queue/laser-fence effects. Temporal does not call the normal explosion,
survivor, debris, or rubble transaction.

### 3.7 Shared non-Building completion transaction

For `InfantryClass`, `UnitClass`, and `AircraftClass`, the exact generic branch
is:

1. If `target+0x2E4` is non-null, pass its linked Building to bunker/dock
   teardown `0x00459470` when `WhatAmI == 6`, otherwise pass null. Clear the
   reciprocal relationship.
2. If `target+0x2D8` owns a SlaveManager, call `MasterDestroyed(source,0)`.
3. Call the Foot notification at instruction region `0x004D98C0(source)`.
4. Call `RecordKill(source)`. Infantry and Aircraft bind directly to
   `TechnoClass::RecordKill @ 0x00702D40`; Unit binds wrapper `0x00744720`.
5. Call `FootClass::UnInit @ 0x004DE5D0`.

The Foot notification ignores source. For a human victim owner whose type is
not a spawned-unit type, it obtains the victim's current/occupied cell and asks
`CreateRadarEvent @ 0x0065FA70` for event type 7. Only if event creation
succeeds does it play `EVA_UnitLost`. AI-owned victims and spawned child types
do neither.

The Unit `RecordKill` wrapper conditionally delivers Unit tag events 7, `0x30`,
and `0x1D` before entering the shared Techno routine. The shared routine owns
events 6 and 4 and the non-Unit route for those additional events, then normal
cost/rank/alliance/DontScore veterancy and house-stat accounting. Temporal must
call the virtual binding; replacing it with one base helper loses Unit events.

#### Infantry

Infantry uses Foot `UnInit`, then `InfantryClass::Limbo @ 0x0051DF10`. The leaf
tail stops/resets locomotion and infantry state before Foot Limbo. If the erased
Infantry is itself a slave, its late class destructor finds `Infantry+0x2DC` and
calls `SlaveManagerClass::RemoveSlave @ 0x006B0A20`, reverse-finding the slot,
clearing the backlink and slot, and marking the control record dead. The master
manager remains consistent through the tombstone interval.

#### Unit

Unit uses Foot `UnInit`, then `UnitClass::Limbo @ 0x007440B0`. After successful
Foot Limbo it returns any carried flag link `Unit+0x6CC` to the current cell and
clears the link. A Unit on a bridge uses ordinary Foot/Techno layer and cell
cleanup; Temporal has no bridge-damage or collapse branch.

If a Unit was linked to a bunker when the shot detonated, the detonation region
`0x00469475..0x004694C1` redirects the intended target to the linked Building,
so Building completion erases the bunker and undocks the Unit. The generic
Unit branch is reached when a Unit acquires a Building link after initiation;
`0x00459470` then clears bunker animations, optionally plays
`BunkerWallsDownSound`, constructs health-ratio down animations, sends radio
break, clears reciprocal links, and queues the Building's native mission. No
RNG is consumed.

#### Aircraft

A legally targeted landed Aircraft uses Foot `UnInit` and Foot Limbo directly.
There is no Temporal crash, fall, explosion, debris, or special airborne-death
branch. If it takes off after initiation it is still erased in air through this
same tail; layer and locomotor removal remain Foot/Techno lifecycle work.

### 3.8 Cargo and recursive passenger source attribution

`ObjectClass::UnInit @ 0x005F65F0` calls `FootClass::EMPPassengers @
0x00707CB0` for Cargo-bearing objects. That routine repeatedly removes the
head passenger from a Team, pops it from Cargo, recursively erases its own
passengers, calls `RecordKill(original_arg)`, and calls `UnInit`.

At the top carrier the argument is null, so immediate Unit/Aircraft transport
passengers are source-less. In recursion, children receive their immediate
parent as `RecordKill` source. The order is depth-first and head-first. There is
no scatter, ejection, death visual, or RNG. The Building Temporal branch has
already emptied inherited Cargo source-less before its common `UnInit`, so it
does not enter this recursive path with passengers remaining.

### 3.9 Capture, spawn, slave, Team, owner, and pending-deploy effects

The initiation pre-clean in §3.2, not the completion split, owns outgoing Spawn
and Capture managers. Foot `UnInit @ 0x004DE5D0` defensively calls capture
`FreeAll` again; on a normal Temporal completion it is already empty. It then:

1. resolves a pending `Foot+0x2AC` ChronoWarp/deploy Building relationship by
   calling `BuildingClass__DeployUnit_ChronoWarp @ 0x0070FEE0(this,1)`;
2. removes the Foot from `Team+0x5D4`; and
3. enters Object `UnInit`.

Slave cleanup belongs at completion because the erased target is the slave
master. `SlaveManager MasterDestroyed @ 0x006B0AE0` reverse-scans controls,
clears each slave backlink, and either source-credits/UnInits an in-limbo slave
or changes a live slave to the Temporal source's owner and resets its target and
mission state. It plays `Rules.SlavesFreeSound` once for any live release when
not in Map Editor, then clears the manager owner. No RNG is consumed.

Captures and slaves are distinct. Captured victims revert to their recorded
original owners and can consume AI fate rolls at initiation. Live slaves become
owned by the Temporal source house at completion; in-limbo slaves are killed
with the source. Neither behavior can be modeled as Cargo.

### 3.10 Shared `UnInit`, Limbo, and deferred physical destruction

`ObjectClass::UnInit @ 0x005F65F0` performs, in order:

1. defuse an attached bomb;
2. recursively remove Cargo when the abstract flags select Foot passengers;
3. dispatch pointer-expired cleanup;
4. call virtual `Limbo`;
5. set `IsAlive` false; and
6. append the object to the pending-delete vector.

`TechnoClass::Limbo @ 0x006F6AC0` owns general target/contact, shroud-border,
open-topped passenger target, house `Removed_From_Game`, cell threat/cover,
sound, radio-break, and conceal state. `FootClass::Limbo @ 0x004DB260` owns the
eight surrounding-cell counters, locomotor stop/release, sound stop, sensor
hook, current cell/layer removal, and then Techno Limbo. These common routines,
plus the leaf tails in §§3.6–3.7, are the authoritative owner/map/Logic/removal
transaction; Temporal must invoke, not duplicate, them.

`DrainDeferredFinalizationQueue @ 0x00725C70` runs after Logic/frame advancement.
It stable-removes duplicate occurrences, releases the object, identifies one of
the four concrete leaves, temporarily restores the required object state, and
calls its scalar deleting destructor:

| Leaf | Scalar deleting destructor |
|---|---:|
| Aircraft | `0x0041C210` |
| Building | `0x00459F20` |
| Infantry | `0x00523350` |
| Unit | `0x00746E80` |

Derived destructors remove the leaf global vector, Abstract-ID registrations,
and remaining class resources. They do not perform a second Temporal kill or
map-removal transaction. Native pointers remain valid through the tombstone
interval; Rust's stable-ID deferred-delete model must preserve the equivalent
ordering.

### 3.11 Completion suffix and callback safety

After either class branch, the manager sends its owner to idle, clears target,
auxiliary, and chain links, then sends the owner to idle a second time. The
first call normally detaches the manager; native reentry guards prevent the
shared detach mutation from applying twice, although leaf mission-idle tails
can still observe both calls.

Every external callback above can mutate relationships. Rust must re-resolve
source and target stable IDs after callbacks rather than retaining borrowed
entity references across owner changes, `RecordKill`, pointer cleanup, or
`UnInit`. Manager graph cleanup must remain idempotent under those callbacks.

### 3.12 Persistence, synchronization, and RNG ledger

`TemporalClass::Save @ 0x0071A700` writes the raw `0x50`-byte body after the
Abstract prefix. `Load @ 0x0071A660` restores vtables and swizzles owner,
target, previous, next, and auxiliary pointers; Techno load swizzles owned
manager and incoming-head pointers. Progress and multi-source topology survive
save/load.

Native manager CRC `0x0071A650` adds only the Abstract contribution, but that
routine is not the live multiplayer per-frame consumer. Rust must hash every
persistent manager, backlink, target flag, budget, and topology field used by
its deterministic simulation.

| Event | Scenario RNG consumption |
|---|---:|
| each stock Inviso Temporal detonation | exactly 1 scatter draw after initiation |
| each AI-owned capture victim released by initiation pre-clean | exactly 1 `RandomRanged(1,100)` draw |
| each human-owned capture victim release | 0 |
| attachment, chain insertion, progression, detach | 0 |
| stock `WARPAWAY` construction | 0 additional (`RandomRate`, Bouncer, Meteor are unset) |
| bunker/dock, slave, Cargo, RecordKill, Limbo, deferred delete | 0 |

## 4. INI Keys and Retail Data Authority

| INI field | Stock effective value/role | Native effect |
|---|---|---|
| `ChronoBeam.Temporal` | `yes` | manager creation and exclusive detonation dispatch |
| `NeutronRifle`, `NeutronRifleE`, `CRNeutronRifle` | only stock Temporal weapons | rookie, elite, IFV gunner producers |
| projectile `Inviso` | yes on all three | first Bullet AI detonation; one later scatter draw |
| projectile `AA` / `AG` | inherited `no` / `yes` | rejects already-airborne Aircraft |
| target type `Warpable` | constructor true; no retail override | eligibility |
| target type `Strength` | per target type | initial signed budget `Strength*10` |
| source type `Trainable` | true for stock CLEG and IFV | explicit pre-award |
| `Rules.VeteranRatio`, `VeteranCap` | retail Rules values | explicit and normal XP formulas |
| `Rules.WarpAway` | `WARPAWAY` | completion animation |
| `Rules.SlavesFreeSound` | retail Rules value | one live-slave liberation sound |
| `Rules.OpenToppedWarpDistance` | `7` cells | compiled non-stock open-topped detach threshold |

`WARPAWAY` lacks `RandomRate`, `Bouncer`, and `IsMeteor`, so its stock
construction adds no Scenario RNG. Map Editor checks in Building Cargo, slave
sound, and several Limbo hooks are compiled functionality but excluded from
active skirmish/campaign execution. Custom maps with type or weapon overrides
are outside stock scope; the mounted retail corpus contains none of the relevant
overrides.

## 5. Integration Points

| Stage | Native owner | Required exact responsibility |
|---|---|---|
| Techno construction | `TechnoClass::Init_Managers @ 0x006F3F40` | allocate one persistent manager from primary Temporal warhead |
| IFV board/exit | Unit regions `0x00746420/0x007464E0` | move manager and rewrite owner; detach before exit |
| fire/Bullet | `TechnoClass::Fire_At @ 0x006FDD50`; Bullet Fire/AI | create, submit, and first-visit detonate Inviso Bullet |
| special dispatch | `BulletClass::DetonateAtCoord @ 0x004690B0` | exclusive Temporal initiation; no ordinary AoE/shrapnel; one scatter draw |
| initiate/gate | `0x0071AF20`, `0x0071AE50` | pre-clean, old-target detach, eligibility, chain attach, initial budget |
| target scheduler | each concrete target's normal Techno/leaf update | advance incoming head exactly once per live target visit |
| progress/complete | `TemporalClass::Update @ 0x0071A760` | sample selected damages, decrement, common prefix, class split, suffix |
| shared lifecycle | Foot/Object/Techno/leaf `UnInit` and `Limbo` | captures, Cargo, Team, pointer expiry, map/house/class removal |
| final delete | `DrainDeferredFinalizationQueue @ 0x00725C70` | one stable deferred physical destruction |
| save/load/hash | Temporal and Techno persistence | graph restoration and deterministic Rust hashing |

## 6. Current Rust Implementation Status

### 6.1 Preserve

- `src/sim/game_entity.rs:316-321` already models manager budget, target, and
  previous/next owner identifiers; the entity also carries owned-manager,
  incoming-head, and warped-state fields.
- Snapshot validation around `src/sim/game_entity.rs:1440` verifies the graph,
  and world hashing around `:1742` includes Temporal state. Preserve that
  deterministic representation and extend it only for verified production
  needs.
- The central lifecycle/`uninit_with_rules` owner already concentrates Cargo,
  pointer expiration, class Limbo, alive-state, and pending-delete work. Reuse
  it after the class-aware Temporal prelude.
- `src/sim/docking/bunker_link.rs` already has a clear-only teardown intended
  for Temporal/non-Building removal. Preserve that ownership seam and make its
  native animation/sound/mission effects exact where represented.
- Capture-manager release already models the AI fate draw. Reuse its RNG owner;
  do not add a Temporal-local random implementation.

### 6.2 Replace or implement

- `src/sim/combat/mod.rs:4412-4437` classifies Temporal but returns it as an
  unsupported special detonation. Stock Inviso delivery currently follows
  ordinary immediate AoE semantics. Replace this with the exclusive production
  initiation route and retain only the native Inviso visual draw/tail.
- No production Techno initialization creates the manager, no IFV path moves
  it, no initiation attaches it, and no target update advances it. Implement all
  four through shared system owners; a completion-only helper cannot close the
  route.
- No parsed Techno type field represents `Warpable`. Add the constructor-true,
  INI-overridable field even though all stock data inherits true.
- Existing Temporal graph cleanup promotes a successor by adding removed
  points. Replace it with assignment of the head's shared remaining value.
- The lifecycle sequence currently kills SpawnManager children during generic
  `UnInit`; native Temporal initiation pre-cleans Spawn and Capture before the
  eligibility gate. Move/call those exact side effects from the Temporal
  initiation owner without duplicating the later empty cleanup.
- Add the verified pending ChronoWarp/deploy and Team-removal hooks to the
  central Foot lifecycle if they are absent when implementation begins.
- Completion must dispatch virtual-equivalent notification and `RecordKill`
  behavior, including the Unit wrapper, before calling central lifecycle.

## 7. Coverage Ledger

| Mechanism / branch | Status | Evidence | Active remainder |
|---|---|---|---|
| four-leaf runtime target census | verified | RTTI/vtables/WhatAmI/deferred-delete casts | none |
| effective `Warpable` default/parser/retail census | verified | `0x00710AF0`, `0x00714F5E..79`, rules/maps | none |
| stock producer and map occurrence census | verified | retail rules and 184-map corpus | none |
| already-airborne Aircraft exclusion | verified | projectile defaults, `0x006FC0B0`, Aircraft layer state | none |
| landed Aircraft then airborne-after-attach path | verified | no later eligibility/altitude recheck; generic Foot tail | none |
| initiation Spawn/Capture pre-clean and RNG | verified | `0x0071AF20`, `0x006B7100`, `0x00472140`, `0x00471FF0`, `0x004723B0` | none |
| target-owned progression/formula/timing | verified | `0x0071A760` plus scheduler/detonation chain | none |
| detach/source death/target invalidation | verified | `0x0071AB60`, `0x0071ABC0`, Techno/Object expiry | none |
| common completion prefix/XP | verified | `0x0071A760`, `0x0074FF50` | none |
| Building occupant/Cargo/bunker/slave/removal order | verified | `0x0071A760`, Building lifecycle reports and callees | none |
| Infantry notification/kill/slave-slot/limbo/delete | verified | vtable bindings and leaf destructor | none |
| Unit trigger wrapper/bunker/bridge/flag/limbo/delete | verified | vtable bindings and Unit branches | none |
| Aircraft notification/ground/air/limbo/delete | verified | vtable bindings and fire legality | none |
| Unit/Aircraft recursive Cargo attribution | verified | `0x00707CB0`, Object UnInit | none |
| capture/spawn/slave/Team/pending deploy effects | verified | listed manager and Foot lifecycle callees | none |
| owner/house/power/map/Logic cleanup ownership | verified | Techno/Foot/Building Limbo | none |
| pending-delete and physical finalization | verified | `0x00725C70`, four scalar destructors | none |
| source idle/detach suffix | verified | `0x0071A760` | none |
| save/load/hash boundary | verified | `0x0071A660`, `0x0071A700`, current Rust | none |
| exact RNG ledger | verified | detonation, capture fate, art flags, completion callees | none |
| custom/mod and Map Editor exclusions | verified boundary | mounted retail data and branch gates | none inside stock scope |

### Exhaustion record

The final zero-add pass revisited the completion split, all four vtable bindings,
common/derived `UnInit` and `Limbo`, fire legality, initiation pre-clean,
manager invalidation, and deferred-delete root after the investigation log
drained. It added no new load-bearing mechanism.

Two cold spots materially tightened the report. First, the Aircraft pass proved
that stock projectiles cannot initiate against an already-airborne target but
that a landed target can take off after attachment and is still erased through
the generic Foot tail. Second, the rejected-target pass proved that capture and
spawn cleanup occur before eligibility and that released AI capture victims add
one RNG draw each even when no Temporal link is ultimately attached.

## 8. Open Questions — Final State

- `[RESOLVED] A-01` — Concrete target leaves are exactly Infantry, Unit,
  Aircraft, and Building; ships are Unit and non-Techno objects are unreachable.
- `[RESOLVED] A-02` — All 560 stock-declared Techno types inherit
  `Warpable=yes`; retail rules/maps define no override.
- `[RESOLVED] A-03` — Completion has one Building split and one generic
  non-Building branch, followed by one common manager cleanup suffix.
- `[RESOLVED] A-04` — Building independent occupants drain reverse and
  source-less before inherited Cargo, ordinary kill/UnInit, and Building Limbo.
- `[RESOLVED] A-05` — Non-Building bunker/dock teardown precedes slave release,
  class notification, virtual RecordKill, and Foot UnInit.
- `[RESOLVED] A-06` — Unit uses a RecordKill wrapper for tag events; Infantry
  and Aircraft bind directly to the shared Techno routine.
- `[RESOLVED] A-07` — Immediate Unit/Aircraft Cargo passengers are source-less;
  nested children receive their immediate parent and drain depth-first.
- `[RESOLVED] A-08` — Spawn and Capture managers are destroyed before the new
  target eligibility test; Capture release is the only extra RNG-bearing path.
- `[RESOLVED] A-09` — A destroyed slave master liberates live slaves to the
  Temporal source owner and source-credits in-limbo slaves; a destroyed slave
  removes its master's slot during late Infantry destruction.
- `[RESOLVED] A-10` — Foot completion resolves pending ChronoWarp/deploy and
  Team membership through normal Foot UnInit.
- `[RESOLVED] A-11` — Owner, power, factory, wall, bridge/layer, map, Logic, and
  removal effects are class lifecycle work, not hidden Temporal branches.
- `[RESOLVED] A-12` — Already-airborne Aircraft are rejected by stock fire
  legality; landed Aircraft that later fly still complete through the generic
  tail with no crash/explosion branch.
- `[RESOLVED] A-13` — Explicit Temporal XP precedes and can coexist with normal
  RecordKill XP; `DontScore` and alliance affect only the normal half.
- `[RESOLVED] A-14` — The exact active RNG delta is one Inviso draw per stock
  hit plus one fate roll per AI-owned capture released by the pre-clean.
- `[RESOLVED] A-15` — Physical deletion is deferred, stable, leaf-dispatched,
  and occurs once even if the queue contains duplicate occurrences.
- `[RESOLVED] A-16` — Save/load preserves progress and topology; deterministic
  Rust hashing must retain every simulation field despite native CRC boundaries.

There is no unanswered question affecting stock Temporal target behavior inside
this report's claimed scope.

## 9. Implementation Handoff

### 9.1 Smallest architecture-compatible production route

Implement one `TemporalSystem`-equivalent owner around existing stable entity
IDs and lifecycle APIs:

1. **Production manager ownership:** at Techno creation, inspect the primary
   weapon's warhead and create one detached manager when Temporal. Transfer the
   same manager during CLEG IFV board/exit.
2. **Shared special detonation:** route both immediate Inviso and persistent
   Bullets through one exclusive Temporal action. Perform bunker target
   redirection, initiation pre-clean, old-target detach, exact eligibility,
   chain attachment, and the single native Inviso RNG tail. Do no ordinary
   damage or shrapnel.
3. **Target-owned update:** from the common live Techno update seam reached by
   all four Rust entity categories, advance the incoming head once. Sample
   current selected weapon Damage, update the shared signed budget, and call
   completion below at `<1`.
4. **Class-aware completion prelude:** construct WarpAway and explicit XP, then
   dispatch Building versus non-Building. Preserve vector/Cargo/bunker/slave/
   notification/RecordKill order and the Unit virtual distinction.
5. **Central lifecycle:** call the existing category-aware `uninit_with_rules`
   owner for capture/Cargo/Team/pending deploy/pointer expiry/Limbo/house/map/
   pending-delete work. Add verified missing lifecycle hooks there; do not
   reproduce the lifecycle inside Temporal.
6. **Manager suffix:** re-resolve IDs after callbacks, idle/detach/clear exactly,
   and tolerate pointer-expiration reentry.
7. **Persistence and synchronization:** validate/save/load/hash the full graph,
   budget, target flags, and IFV-transferred owner.

### 9.2 Required completion pseudosequence

```text
complete(head_id):
    target = resolve(head.target_id) or clear_chain_and_idle_sources(); return
    source = resolve(head.owner_id)
    create_warp_away(target.current_coord)
    award_explicit_temporal_xp(source, target) if source.type.trainable

    if target.category == Building:
        reverse_uninit_independent_occupants(source = None)
        drain_building_cargo(source = None)
        suspend_optional_super()
        undock_linked_unit_if_any()
        slave_master_destroyed(source)
        building_temporal_notification(source)
        virtual_record_kill(target, source)
    else:
        clear_linked_bunker_or_dock_if_any()
        slave_master_destroyed(source)
        foot_temporal_notification(target)
        virtual_record_kill(target, source)  // Unit wrapper must dispatch

    central_uninit_with_rules(target)
    if Building: dirty_target_owner()
    re_resolve_manager_and_source()
    native_idle_detach_clear_idle_suffix()
```

### 9.3 Acceptance matrix

| Scenario | Required assertions |
|---|---|
| rookie/elite CLEG vs ordinary Infantry | real shot creates no ordinary damage; next target visit decrements by selected 8/16; WarpAway, Foot radar/EVA gate, RecordKill, Foot/Infantry Limbo, pending delete |
| Infantry `DontScore` and allied target | explicit Temporal XP remains; normal RecordKill XP respectively suppresses or becomes zero; lifecycle still completes |
| ordinary Unit | Foot notification plus Unit wrapper tag events precede Foot UnInit; ordinary layer/cell/flag cleanup occurs |
| Unit on bridge | legal fire/progression/completion uses ordinary Foot bridge/layer cleanup; no bridge damage/collapse side effect |
| Unit bunkered at hit | detonation redirects target to linked Building; Building is erased; Unit is undocked and survives |
| Unit acquires bunker link after initiation | generic Unit completion calls Building bunker teardown before Unit kill/UnInit |
| landed Aircraft | stock AG projectile may attach; generic Foot completion has no crash/fall/explosion path |
| landed Aircraft takes off mid-progress | no detach or altitude recheck; airborne target completes at the same budget tick through Foot cleanup |
| already-airborne Aircraft | stock fire legality rejects before Temporal initiation; no graph/RNG mutation from a shot that is never fired |
| ordinary Building | Building notification/navigation ring, RecordKill, Building Limbo power/factory/base/wall/house effects; no explosion/survivor/rubble |
| occupied Building with Cargo and linked Unit | independent vector reverse source-less, Cargo source-less, Unit undock, slave/notification/kill/UnInit ordering exactly |
| Unit transport with nested Cargo | head-first/depth-first recursion; immediate passenger source null; nested child source immediate parent |
| Aircraft transport with Cargo | same recursive Cargo attribution and Foot cleanup as Unit transport |
| target owns human captures | FreeAll occurs before eligibility; zero fate RNG; owner/team/controller relationships restored/cleared |
| target owns AI captures | exactly one fate draw per released victim, including on an ultimately ineligible Temporal target |
| target owns SpawnManager | pre-gate reverse cleanup; idle children UnInit, live-flight children retreat; same effects on rejected target |
| target is slave master | live slaves change to source owner, in-limbo slaves RecordKill/UnInit with source, one liberation sound, no RNG |
| target is slave Infantry | tombstone remains stable until late destructor removes master slot/backlink exactly once |
| Foot has pending ChronoWarp and Team | Foot UnInit resolves `+0x2AC` and removes `+0x5D4` before Object UnInit |
| two or more Temporal sources | one shared Strength*10 budget, sum current selected damages, successor bound, head death promotes by assignment |
| selected Damage becomes zero | next visit subtracts zero and never forces progress |
| source death/retarget/IFV exit | exact detach and successor behavior; IFV exit returns manager to passenger after detach |
| target normal UnInit wins | pointer expiration clears every incoming link and idles sources before physical destruction |
| completion wins before zero-health UnInit | manager may complete while still target-scheduled; only one removal transaction results |
| save/load mid-chain and mid-IFV ownership | exact budget/topology/owner/target/backlinks/warped flag, next hash, next decrement, and completion tick |
| RNG accounting | one draw per stock detonation plus AI capture rolls; zero additional completion draws |
| duplicate pending-delete entries | finalizer stable-removes duplicates and invokes exactly one correct leaf scalar destructor |

The production-route tests must fire the stock weapon and run the real scheduler;
isolated completion-helper tests are supplementary and cannot satisfy acceptance.

## 10. Adversarial Self-Review

1. **Could a fifth Techno leaf have been hidden by the completion's generic
   branch?** No. RTTI/vtable enumeration and the independent deferred-delete
   cast sequence both close the set at four; ships are Unit.
2. **Could the stock `Warpable` census miss a map-local exception?** No for the
   mounted retail corpus: both base rules and all 184 extracted maps were
   searched; none assigns `Warpable`, `Temporal`, or `ChronoBeam` warheads.
3. **Could generic `RecordKill` erase the need for the explicit XP pre-award?**
   No. The calls are separate and ordered; `DontScore`/alliance distinguish
   their effects and expose any accidental deduplication.
4. **Could Aircraft need a special crash tail?** No. The only completion split
   is Building vs non-Building, and Aircraft binds the generic Foot UnInit/Limbo
   slots. Altitude affects initial fire legality, not later completion.
5. **Could Cargo, bunker occupants, captures, spawns, and slaves be one passenger
   abstraction?** No. They occupy independent fields, have different callback
   timing, ownership attribution, RNG, and survival behavior.
6. **Could the Building helper run normal death/explosion effects later through
   Limbo?** No. Temporal calls RecordKill and UnInit directly; Building Limbo
   handles removal/accounting, not explosion/survivors/rubble.
7. **Could completion safely retain Rust references across callbacks?** No.
   owner changes, RecordKill, manager cleanup, pointer expiry, and UnInit can
   mutate the same graph; stable IDs must be re-resolved.
8. **Could native manager CRC justify omitting Rust hash fields?** No. The native
   routine is not the live frame-sync owner; Rust's deterministic state must
   hash all fields that can change future completion.
9. **Could rejected targets have no observable effect?** No. Spawn/capture
   destruction and AI capture fate rolls occur before `CanWarpTarget`.
10. **Could a Building's occupied vector be handled by generic Cargo cleanup?**
    No. Native completion explicitly drains the independent vector first and in
    reverse, then separately drains inherited Cargo.

## 11. Ghidra Annotation Candidates

No Ghidra metadata was mutated during this read-only investigation.

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x004D98C0` | instruction region without a safe function boundary | comment: Foot Temporal-erasure notification creates radar event 7 and conditionally plays `EVA_UnitLost` | comment | direct vtable binding for Infantry/Unit/Aircraft and disassembly of human/spawned gates | worker-report-only |
| `0x00744720` | existing Unit wrapper | `UnitClass::RecordKill` wrapper comment documenting tag-event prelude | comment | Unit vtable `+0xE0` binding and call into `0x00702D40` | worker-report-only |
| `0x006B0AE0` | generic symbol | `SlaveManagerClass::MasterDestroyed` | rename | reverse slave-control scan, owner transfer/source kill, manager-owner clear | worker-report-only |
| `0x00725C70` | generic symbol | `DrainDeferredFinalizationQueue` | rename | stable duplicate removal, four RTTI casts, leaf scalar-destructor dispatch | worker-report-only |

The instruction region at `0x004D98C0` is a comment candidate only; inventing a
new function boundary would not pass the project's certainty gate.

## Sources

- Active-retail Ghidra program: mounted `gamemd.exe`; read-only decompilation and
  disassembly of `0x0041ADC0`, `0x0041B920`, `0x0041C180`, `0x00445880`,
  `0x0044D760`, `0x004593A0`, `0x00459470`, `0x004690B0`, `0x00469475..C1`,
  `0x0046BBC0`, `0x00471FF0`, `0x00472140`, `0x004723B0`, `0x004D98C0`,
  `0x004DB260`, `0x004DE5D0`, `0x0051DF10`, `0x00523340`, `0x005F65F0`,
  `0x006B0A20`, `0x006B0AE0`, `0x006B7100`, `0x0065FA70`, `0x006F3F40`,
  `0x006FC0B0`, `0x00702D40`, `0x007077C0`, `0x00707CB0`, `0x00710AF0`,
  `0x00714F5E..79`, `0x0071A650`, `0x0071A660`, `0x0071A700`, `0x0071A760`,
  `0x0071AB60`, `0x0071ABC0`, `0x0071AE50`, `0x0071AF20`, `0x00725C70`,
  `0x007440B0`, `0x00744720`, `0x00746E20`, `0x0074FF50`, and the IFV transfer
  regions `0x00746420/0x007464E0`.
- Retail data: mounted `rules.ini`, `rulesmd.ini`, `artmd.ini`, and the extracted
  184-map campaign/skirmish corpus under `target/phase3-retail-census/extract`.
- `docs/research/PHASE3_TEMPORAL_OCCUPIED_BUILDING_ERASE_GHIDRA_REPORT.md`.
- `docs/research/PHASE3_BUILDING_CAN_BE_OCCUPIED_VECTOR_LIFECYCLE_GHIDRA_REPORT.md`.
- `docs/research/PHASE3_BUILDING_SPAWN_SURVIVORS_CARGO_GHIDRA_REPORT.md`.
- `docs/research/PHASE3_BUILDING_EXPLODES_LIFECYCLE_GHIDRA_REPORT.md`.
- Current Rust sources under `src/sim/`, read-only for disparity and handoff.
