# Damage Concrete Receiver and Lifecycle Reconciliation — 2026-07-13

**Scope:** bounded Task 2S reconciliation of the finalized non-Building,
Building, and PostMortem investigations.  
**Binary target:** active retail Yuri's Revenge `gamemd.exe` 1.001.  
**Mode:** documentation-only reconciliation; no new broad investigation, no
Ghidra mutation, no Rust edit, and no Cargo run.  
**Status:** **PARTIAL — implementation handoff is structurally complete, but
Task 2 is not authority-ready.**

## 1. Verdict

The active concrete receiver chain is now identified and ordered for `Foot`,
`Infantry`, `Unit`, `Aircraft`, `Building`, and Building PostMortem. The new
reports agree on the class entries, seven-argument forwarding boundary,
result-band ownership, and the main live-membership transitions. The canonical
model is:

1. a class wrapper may transform or reject the raw packet before the common
   Techno/Object transaction;
2. the common transaction returns result `0..5` after synchronous callbacks;
3. the concrete wrapper performs class-specific reactions, effects, RNG, and
   lifecycle work before returning the same result; and
4. result `4` does **not** imply one universal removal policy, while result `5`
   is a distinct PostMortem state, not delayed result `4`.

That model is verified, but the parity gate remains closed. The unresolved
Infantry presentation/leaf helpers, Building helper `0x0048DED0` and generic
effect-helper internals, and raw persisted `Building+0x52C` state are all inside
player-visible, RNG-visible, or byte-visible authority scope. The Task 2 plan
requires every authority-critical wrapper, RNG draw, PostMortem field/formula,
and lifecycle transition to be resolved; otherwise the affected class remains
shadow-only before Task 14
(`2026-07-13-damage-authoritative-cutover-plan.md:724-736`). No child `PARTIAL`
finding is upgraded here.

Citation shorthand used below:

- **2A** — `DAMAGE_NONBUILDING_RECEIVERS_REINVESTIGATION_2026-07-13.md`
- **2B** — `DAMAGE_BUILDING_RECEIVER_REINVESTIGATION_2026-07-13.md`
- **2C** — `DAMAGE_POSTMORTEM_REINVESTIGATION_2026-07-13.md`

## 2. Canonical receiver identity and dispatch boundary

| Receiver | Raw authority | Active entry | Verified boundary |
|---|---|---:|---|
| Foot | RTTI `.?AVFootClass@@`, vtable `0x007E8C94`, slot `+0x16C` | `0x004D7330` | raw pre-gates, then seven-argument common receiver call |
| Infantry | RTTI `.?AVInfantryClass@@`, vtable `0x007EB058`, slot `+0x16C` | `0x00517FA0` | Infantry pre-gates → Foot → Infantry result postlude |
| Unit | RTTI `.?AVUnitClass@@`, vtable `0x007F5C70`, slot `+0x16C` | `0x00737C90` | factory/destination immunity → Foot → Unit result postlude |
| Aircraft | RTTI `.?AVAircraftClass@@`, vtable `0x007E22A4`, slot `+0x16C` | `0x004165C0` | Foot → Aircraft fatal continuation |
| Building | RTTI `.?AVBuildingClass@@`, vtable `0x007E3EBC`, slot `+0x16C` | `0x00442230` | Building prechecks/snapshots → Techno → Building result postlude |
| Building PostMortem | Techno receiver result-4 continuation | `0x00701E71..0x00701F72` | qualifying Building death → timer/latch update → life/HP restore → result `5` |

The four non-Building identities are raw-vtable/RTTI findings
(2A:38-57). The Building identity and corrected slots are independently pinned
by raw RTTI and vtable bytes: `+0x16C = 0x00442230`,
`+0x4EC = DestructionEffects @ 0x004415F0`,
`+0xD4 = Limbo @ 0x00445880`, and
`+0xF8 = ObjectClass::UnInit @ 0x005F65F0` (2B:46-72).

Every concrete entry forwards the same explicit packet, in this order:

```text
pDamage, distance, warhead, attacker/source object,
ignoreDefenses, arg6, sourceHouse
```

The non-Building wrappers establish the order directly (2A:27-36), and the
Building `RET 0x1C` plus push flow independently confirms it (2B:59-72). The
semantic name of `arg6` is not upgraded beyond the child evidence.

## 3. Result and membership matrix

| Receiver/path | Result `0` | Results `1..3` | Result `4` | Result `5` | Membership on return |
|---|---|---|---|---|---|
| Foot | return | Team callback or mission reaction | Team callback only when teamed | immediate return | unchanged by Foot |
| Infantry | return | engineer/Scatter/fear postlude | class fatal sequence | immediate return | ordinary fatal retained; exceptional Cyborg path may remove inline or retain a successful crash |
| Unit | return | miner/crusher/dock reactions | class fatal sequence | immediate return | ordinary fatal removed inline; sinking or successful crash retained |
| Aircraft | return | return unchanged | UnitLost → destruction anim → crash/UnInit | immediate return | successful airborne crash retained; failed/grounded crash removed inline |
| Building | common alive/frame postlude | result-2/3 damage effects plus common postlude | fatal contact/garrison/light/destruction transaction | temporary snapshot cleanup only | ordinary lethal removed inline; Selling/Explodes lethal retained at zero HP until own Update; result 5 retained alive at HP 1 |

The non-Building result map and its no-remap rule are verified in 2A:59-70;
the membership distinctions are consolidated in 2A:340-356. The Building jump
table and result actions are verified in 2B:88-103, its immediate/deferred split
in 2B:210-264, and result-5 bypass in 2C:412-424.

## 4. Ordered concrete class contracts

### 4.1 Foot

Before calling the common receiver, Foot uses the **raw** incoming damage:

1. Sonic plus an attached parasite detaches the parasite and may notify the
   damage attacker.
2. A different parasite attacker whose suppression threshold is below the raw
   damage receives a timer start at the current frame, the wrapper's
   undominated local dword in the middle field, and duration
   `damage * 2 - threshold`.
3. Negative raw damage with a parasite writes the same start/middle fields,
   sets duration `50`, and detaches.
4. Foot then forwards the packet once to the common receiver.

This exact order, including the raw/undominated middle-dword provenance, is
verified in 2A:74-95. It must not be normalized to a convenient constant.

After the common result:

1. result `5` returns immediately;
2. any nonzero result with a Team synchronously calls the Team damage callback
   and returns the same result;
3. without a Team, only results `1..3` can reach the mission-control reaction;
4. results `0` and unteamed `4` have no remaining Foot-only effect.

The later apparent Team branch to `0x00708080` is dominated and unreachable in
this receiver (2A:97-109). Foot performs no direct RNG draw and never calls
`UnInit` in this wrapper (2A:111-117).

### 4.2 Infantry

Pre-Foot order:

1. for a set instance byte, positive raw damage, and
   `ignoreDefenses == false`, multiply by the warhead factor, truncate, and clamp
   to at least `1`;
2. a non-null warhead with `InfDeath == 9` and positive victim height then
   overwrites the damage with `0`;
3. forward once to Foot.

The order and stock `InfDeath=9` liveness are verified in 2A:121-133.

For results `1..3`, Infantry performs the engineer mission reaction, calls
Scatter toward a non-null attacker, and then applies fear. With an attacker and
current fear `<100`, Fraidycat writes `300`, Fearless/ability `0xD` blocks the
write, and every other case writes exactly `100`; this includes old values
`1..99`. The alternate path adds `50`, scales it at the red/yellow condition
gates, and clamps at `300` (2A:135-154).

For result `4`, the verified spine is:

1. detach any SlaveManager relationship and run the gated `+0x11C` callback;
2. issue UnitLost, stop/clear fire and mission state, queue `-1` then `5`,
   commence the mission, and EMP passengers;
3. select death presentation from height/water, Cyborg/JumpJet/NotHuman,
   `InfDeath`, type animation data, and special attacker gates;
4. normally return while still in live Logic membership for the death
   sequencer;
5. only the verified Cyborg immediate flag can instead call inline `UnInit`, or
   retain the Infantry when its crash helper succeeds.

The fatal spine and membership split are verified in 2A:156-192. The exact
semantic mapping and allocation-failure output of all ten `InfDeath` switch
arms is still **PARTIAL** (2A:194-203). No direct RNG occurs in the scoped
Infantry body, but virtual Scatter and the exceptional crash helper can consume
conditional draws (2A:205-210).

### 4.3 Unit

Before Foot, ordinary defenses reject the hit when the Unit's exact destination
is a WeaponsFactory Building and the Building in its current cell is that same
destination (2A:214-222).

For results `1..3`, the ordered postlude is:

1. local-player Harvester radar event and `EVA_OreMinerUnderAttack`;
2. early return for Team, null attacker, dock-entered byte, ally, or
   player-controlled owner;
3. otherwise the crusher counteraction may install the attacker and queue attack;
4. otherwise a Harvester/Weeder low-health Dock entry may radio, install the
   target, and queue Enter/Dock.

The exact semantic name of virtual `+0x2B8` in the Dock gate remains unasserted
(2A:224-239).

For result `4`, the wrapper tears down the reciprocal Building link, chooses
DeathFrames, sinking, water visuals, or `Death_Explosion`, then runs common
fatal cleanup. That cleanup decloaks, clears open-topped firing, conditionally
EMPs passengers, ejects each passable cargo object for non-Crashable transports
or kills only failed/forbidden ejections, handles crew/survivor and crate work,
and finally chooses crash/sinking retention or inline `UnInit`
(2A:241-291).

Direct Unit RNG order is load-bearing:

- each of the two destruction lists consumes its selection draw **before**
  allocation;
- the default crew chance draw is skipped by an explicit survivor;
- crew health draws only after successful placement;
- Scatter and successful crash helpers add conditional draws.

The exact ledger is in 2A:293-311. Nearby-cell/crate leaf output and exhaustive
per-class Scatter RNG remain **PARTIAL** (2A:277-278, 400-412).

### 4.4 Aircraft

Only result `4` has Aircraft-only work:

1. issue UnitLost;
2. if the destruction list is nonempty, allocate first;
3. only after successful allocation, consume one selection draw and construct
   the animation;
4. call the crash helper;
5. retain on successful crash, otherwise call `UnInit` inline.

This ordering and membership split are verified in 2A:313-338. Aircraft's
allocation-before-draw order is deliberately the opposite of Unit's
draw-before-allocation order and may not be unified.

### 4.5 Building receiver

The Building wrapper performs, in order, self-damage and type gates, attacker
bookkeeping, foundation/contact snapshots, LaserFence and BridgeRepairHut/Immune
gates, and then the common receiver when current Health is nonzero
(2B:76-87).

Result dispatch is:

- `0/1`: common alive/frame postlude;
- `2`: scale the existing particle field, then fall through result `3`;
- `3`: use the global `BuildingDamageSound` fallback when the type field is
  `-1`, then run Sparky per saved foundation cell;
- `4`: run the fatal chain below;
- `5`: free/unwind the temporary snapshot only, skipping result-4 destruction
  and the normal surviving postlude.

The jump-table mapping is verified in 2B:88-103 and the result-5 behavior in
2C:412-424.

Fatal result-4 order before `DestructionEffects`:

1. remove/undock the reciprocal linked object;
2. release capture-manager and `+0x2AC` state;
3. walk the saved contact snapshot;
4. contacts within `<0x100` leptons or with Helipad receive forced damage;
   other contacts receive radio `0x17` and have `+0x500` cleared;
5. free the contact snapshot;
6. eject `CanBeOccupied` garrison in reverse/LIFO order through SellBuilding;
7. tear down the light source;
8. call `DestructionEffects(0, attacker, ignoreDefenses, savedFoundation)`;
9. apply the verified duration/removal split.

The exact predicates, forced-call packet, and order are in 2B:105-123.
CanBeOccupied garrison and inherited Cargo are distinct mechanisms.

`DestructionEffects @ 0x004415F0` then performs this ordered ledger:

1. clear all eight damage-fire pointers;
2. auxiliary/radar cleanup;
3. the gated special-type recalculation;
4. LaserFencePost disconnect;
5. conditional reveal-to-all;
6. global `BuildingDieSound` fallback when the type list is empty;
7. center destruction smudge/debris, with width draw only for `W>2`, height draw
   only for `H>2`, then the `0..99` choice;
8. per-foundation destruction animations: coordinate draw, allocate, then on
   success draw `0..3` and list selection;
9. allocation-gated four-offset `Explodes` burst;
10. spill one whole stored resource unit per loop while mirroring one owner
    accounting decrement, leaving fractions below `1.0`;
11. conditionally call **UNKNOWN helper `0x0048DED0`**;
12. write the timer triple with duration `0` for Selling/Explodes and `8`
    otherwise;
13. select the main death animation before allocation;
14. reverse-scan and allocation-gate the destruction particle system;
15. commit Health `0` and write survivor-suppression byte `+0x6E0` from
    nonzero `ignoreDefenses`;
16. call `SpawnSurvivors`, then `EMPPassengers(attacker)`.

The complete caller-level ledger is verified in 2B:150-171. Its exact RNG and
allocation boundaries are summarized in 2B:266-280. Helper `0x0048DED0` and the
generic constructor/helper internals are not silently included in that closure.

Building lifecycle then splits:

- ordinary non-Selling, `Explodes=no`: duration `8` makes the receiver call
  `UnInit` synchronously; the Building is concealed/removed from live Logic
  membership and queued for later physical deletion before the wrapper returns;
- Selling or `Explodes=yes`: duration `0` skips inline `UnInit`; the Building
  returns at Health `0` but remains alive and registered until its own Update,
  which calls Limbo, calls `SpawnSurvivors` a **second** time, calls `UnInit`,
  and refreshes foundation/tactical state;
- therefore removal may happen later in the same tick or the next tick according
  to the Building's live-vector position.

This counterintuitive split is verified in 2B:210-249. `UnInit`, Limbo, the
compacting live-vector remove, and successor-skip consequence are traced in
2B:251-264.

## 5. PostMortem exact behavioral contract

### 5.1 Eligibility and ordered transition

PostMortem requires all five gates, in order:

1. the common Object result is exactly `4`;
2. receiver `WhatAmI()` is Building (`6`);
3. warhead is non-null;
4. `warhead+0x130 CausesDelayKill` is true;
5. `BuildingType+0x1551 EligibleForDelayKill` is true.

There is no second `ignoreDefenses` gate in this block and incoming damage does
not enter the delay formula (2C:111-137). The lethal Object transaction has
already executed before this test.

On qualification, the receiver computes a candidate duration, arms a new timer
or replaces the existing timer only when the candidate is strictly shorter,
then universally writes life byte `1`, signed Health `1`, and returns result
`5` (2C:139-150, 262-292, 393-410). Equal/longer hits preserve all timer metadata,
but every qualifying lethal repeat still re-runs the preceding exact-zero
callbacks before the two restoration writes.

### 5.2 Exact duration operation

Let `B = DelayKillFrames` as signed i32, `A = DelayKillAtMax` as binary32,
`C = CellSpread` as binary32, and `d` be caller-supplied signed lepton distance.
The receiver executes:

```text
den_i32 = low_i32(Math__ftol_x87(C)) << 8          // wrapping i32 shift
slope_ext80 = ext80(A) * ext80(B) - ext80(B)
delay_ext80 = ext80(B) + ext80(d) * (slope_ext80 / ext80(den_i32))
duration_i32 = low_i32(Math__ftol_x87(delay_ext80))
```

It preserves x87 stack order, signed i32 inputs, binary32 source bits,
truncate-toward-zero conversion, and has no denominator-zero guard or clamp
(2C:152-219).

Stock `OilExplosionWH` uses `CellSpread=4`, `DelayKillFrames=5`, and
`DelayKillAtMax=7.0`, yielding
`trunc_toward_zero(5 + 30*d/1024)`. Receiver-side checkpoints include
`d=34 -> 5`, `d=35 -> 6`, `d=256 -> 12`, `d=512 -> 20`, `d=768 -> 27`,
`d=1023 -> 34`, and `d=1024 -> 35` (2C:221-258).

### 5.3 Shared fields, cancellation, persistence, and expiry

| Field | Canonical role | Authority status |
|---:|---|---|
| `+0x528` | signed start frame | verified |
| `+0x52C` | opaque raw middle dword | **UNKNOWN value; exact-byte blocker** |
| `+0x530` | signed duration | verified |
| `+0x540` | retained source-object pointer | verified |
| `+0x6DF` | shared pending latch | verified |

PostMortem and infantry C4/Ivan share the latch, timer triple, and source field;
they are not independent timers. PostMortem does not overwrite `+0x540`, while
an existing infantry-planted source survives a strictly shorter PostMortem
replacement (2C:294-320, 322-331).

`+0x52C` is read from undominated stack storage in the PostMortem arm, is not
initialized by the Building constructor, is raw-save persisted, is not read by
expiry, and is omitted from the Building checksum. IronCurtain cancellation
repeats an uninitialized-local write to the same field. Static evidence proves
that provenance but does not supply one deterministic semantic value
(2C:332-353, 470-491). It must remain explicitly quarantined; no seed, facing,
padding, or zero value may be invented.

Building IronCurtain/ForceShield entry clears `+0x6DF` and `+0x540`, rewrites
the timer triple with duration `0`, and then delegates. Healing does not clear
the latch; expiry instead damages the Building by its then-current Health
(2C:426-461).

Regular Building expiry is owned by `BuildingClass::Update` and synchronously
re-enters the seven-argument Building receiver with:

```text
pDamage = current signed Health
distance = 0
warhead = C4Warhead
attacker/source object = +0x540
ignoreDefenses = 1
arg6 = 0
sourceHouse = 0
```

The regular branch does not clear the latch first and may retry if the receiver
unexpectedly leaves the Building alive. BridgeRepairHut instead dispatches the
bridge-specific 5x5 destruction branch and clears latch/source
(2C:493-558).

## 6. Cross-class lifecycle and RNG handoff

### 6.1 Live membership

| Path | Native transition owner | Timing consequence |
|---|---|---|
| Foot result | none in Foot | membership unchanged |
| ordinary Infantry fatal | Infantry death sequencer → Foot UnInit | retained across wrapper return |
| exceptional Infantry immediate/crash | Infantry wrapper | remove inline or retain successful crash |
| ordinary Unit fatal | Unit wrapper → UnInit | compacting removal inside receiver |
| Unit sinking/crash | later sinking/crash owner | retained |
| Aircraft failed/grounded crash | Aircraft wrapper → UnInit | compacting removal inside receiver |
| Aircraft successful crash | crash continuation | retained |
| ordinary Building fatal | Building receiver → UnInit | compacting removal inside receiver |
| Selling/Explodes Building fatal | own Building Update → Limbo/UnInit | retained at return; same-tick or next-tick removal |
| Building PostMortem | no removal; life/HP restored | retained alive at HP 1 until expiry call |

These rows are direct consolidations of 2A:340-356, 2B:210-264, and
2C:393-424. `Detach_From_All_Lists` is observer/reference cleanup, not Logic
unregistration (2A:287-291; 2C:382-391).

### 6.2 Direct RNG ordering

| Owner | Required ordering |
|---|---|
| Foot | no direct draw in scoped wrapper |
| Infantry | no direct body draw; Scatter and crash helper conditionally draw |
| Unit | destruction-list selections before allocation; crew chance/type before allocation; health only after placement |
| Aircraft | allocate before destruction-list selection draw |
| Building Sparky | per-cell first range; second range only after successful allocation in bands 1..8 |
| Building destruction | preserve the center conditional dimension draws, per-cell direction/allocation/selection sequence, Explodes allocation gates, storage draws, main-animation pre-allocation selection, particle allocation gate, crew and smudge gates |
| PostMortem | no PostMortem-block RNG is established; Task 3 still owns producer-side RNG and call timing |

The non-Building rows come from 2A:111-117, 205-210, 293-311, and 323-338.
The complete Building caller-boundary ledger is 2B:266-280. Generic helper
internals remain unresolved and therefore prevent an exhaustive whole-path RNG
claim.

## 7. Contradiction and supersession ledger

| Superseded claim | Canonical replacement | Evidence |
|---|---|---|
| `0x005227F0` is Infantry ReceiveDamage; `0x004D6FA0` is Foot ReceiveDamage. | Infantry is `0x00517FA0`; Foot is `0x004D7330`. | raw vtable slots and body roles, 2A:38-57 |
| Every Infantry death is deferred and current Rust therefore matches. | Ordinary death is deferred, but the verified Cyborg immediate flag can UnInit inline or retain a crash; current Rust is not a class/flag-equivalent receiver. | old `TARGETDEATH_INFANTRY_DEATH_SEQUENCE_DEFERRED_REMOVAL_RESWARM_20260528.md:10-20`; 2A:173-192, 358-379 |
| Building vtable `+0x4EC` is Limbo at `0x00445880`, so every lethal hit conceals synchronously. | `+0x4EC` is DestructionEffects `0x004415F0`; `+0xD4` is Limbo `0x00445880`. Removal then splits by duration/path. | old `TARGETDEATH_BUILDINGCLASS_DESTRUCTION_REMOVAL_OWNER_RESWARM_20260528.md:50-82`; raw bytes, 2B:46-72 |
| Buildings always leave the live vector inside ReceiveDamage and only final UnInit is deferred. | Ordinary duration-8 deaths UnInit inline; Selling/Explodes duration-0 deaths remain registered until own Update. | old Building report:99-145; 2B:210-264 |
| Rust needs no per-Building deferred destruction owner and post-combat survivor ejection is equivalent. | Selling/Explodes requires a real own-Update lifecycle, including second SpawnSurvivors and live-vector timing. | old Building report:230-239, 256-289; 2B:222-249 |
| Duration `8` means an eight-tick corpse delay and duration `0` is immediate. | In this receiver branch, duration `8` causes inline UnInit; duration `0` defers removal to own Update. | 2B:295-308 |
| 2B's result-table shorthand sends result `5` through a "common postlude." | The focused 2C branch trace is canonical: result `5` only unwinds/frees the temporary snapshot and returns, skipping both result-4 destruction and the normal surviving post-damage tail. | 2B:88-103; 2C:412-424 |
| `+0x52C` may be a facing or seed. | It receives undominated stack data, persists raw, is not an expiry input, and is omitted from checksum. Semantic identity remains UNKNOWN. | old `WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md:248-258`; 2C:332-353, 470-491 |
| BuildingType `+0x1551` is SelfHealing. | It is `EligibleForDelayKill`, default false. | old `RECEIVE_DAMAGE_GHIDRA_REPORT.md:157-165,372-374`; string/reader proof, 2C:95-109 |
| The Oil Derrick is the delayed-kill target. | `CAOILD` is a producer; stock eligible targets are `CAMISC01`, `CAMISC02`, and `AMMOCRAT`. | old Warhead report:266-270; 2C:560-612 |
| IronCurtain merely postpones the pending C4/delay hit. | Building IronCurtain/ForceShield entry cancels the shared pending state before delegation. | 2C:426-447 |
| The delay is a generic float `distance/CellSpread` interpolation. | It truncates binary32 CellSpread before the wrapping `<<8` and preserves the specified x87 operation order. | 2C:170-219 |

No unresolved contradiction remains between 2A, 2B, and 2C themselves. The one
result-5 wording conflict is resolved in favor of 2C's narrower, dedicated
branch trace. The remaining boundaries are complementary: 2A owns non-Building
wrappers, 2B owns Building result dispatch and destruction lifecycle, and 2C
owns the result-5 producer and pending-state consumer.

## 8. Current Rust contract

### 8.1 Focused source scan

The reconciliation ran focused `rg` and direct reads over the current tree. The
important current shape is:

- `src/sim/combat/damage/mod.rs:142-158` represents only results through
  `Dead`; `PostMortem` is absent. `receive_damage` is currently referenced only
  by its own unit tests, not the live combat path
  (`src/sim/combat/damage/receive.rs:34-138`).
- live combat still subtracts unsigned Health with `saturating_sub`, batches
  dead IDs, and runs death effects later
  (`src/sim/combat/mod.rs:1849-1909`; area death subtraction at
  `src/sim/combat/mod.rs:1060-1097`).
- current death handling chooses deferred presentation mainly from
  `has_animation`, kills non-garrison transport riders, and queues
  structure/voxel uninit (`src/sim/combat/mod.rs:824-1031`).
- `LogicVector` already provides insertion order and compacting removal, and
  `Simulation::for_each_live_object` exposes live-length iteration
  (`src/sim/world/logic_vector.rs:1-45`;
  `src/sim/world/mod.rs:992-1021`). `Simulation::uninit` already centralizes
  conceal-before-pending-delete (`src/sim/world/mod.rs:1266-1329`), but combat
  invokes unregister/uninit only after the batched damage phase
  (`src/sim/world/mod.rs:2384-2424`).
- Infantry fear still returns unchanged for attacker hits at old fear `1..99`
  (`src/sim/infantry.rs:48-82`), contrary to the native reset-to-100 rule.
- `WarheadType` has no `CausesDelayKill`, signed `DelayKillFrames`, or raw
  binary32 `DelayKillAtMax` fields/parser, and `ObjectType` has no
  `EligibleForDelayKill` field/parser
  (`src/rules/warhead_type.rs:25-119,121-203`;
  `src/rules/object_type.rs:240-335,940-1020,1090-1130`).
- Building C4 is an independent `u64` tick/attacker marker, retries through
  IronCurtain, and is hashed directly
  (`src/sim/components.rs:1041-1060`;
  `src/sim/game_entity.rs:443-456`;
  `src/sim/world/world_orders.rs:434-640`;
  `src/sim/world/world_hash.rs:621-625`).
- destruction survivors are hardcoded to one side infantry, center smudges
  always consume both dimension draws for every `>=2x2` footprint, and center
  on anchor-cell `+128`
  (`src/sim/production/production_sell.rs:193-241`;
  `src/sim/combat/smudge_dispatch.rs:222-281`).
- `GameEntity` is serialized inside the versioned snapshot; the current version
  is `25` (`src/sim/game_entity.rs:189-194`;
  `src/sim/snapshot.rs:66-90,131-170`).

### 8.2 Required Rust-native ownership

The implementation must remain Rust-native while preserving the verified
native transaction:

1. **Rules/data owner:** add exact PostMortem rule/type fields and defaults,
   preserving binary32 inputs and signed i32 frames. Do not route them through
   the existing lossy percent/fixed-point representations.
2. **Receiver result owner:** extend the damage result to `0..5`, keep the common
   Object/Techno transaction synchronous, and dispatch one class-aware wrapper
   continuation. Do not duplicate common arithmetic per class.
3. **Lifecycle owner:** retain `EntityStore` for storage and `LogicVector` for
   order, but call conceal/uninit at the verified wrapper position. Retained
   Infantry/sinking/crash/PostMortem paths must stay in live order; inline Unit,
   Aircraft, and ordinary Building removal must compact it immediately.
4. **Building update owner:** implement the zero-health Selling/Explodes removal
   continuation and the positive-health shared pending-latch expiry at their
   verified own-Update positions. These are not post-combat queues.
5. **Cargo/garrison owner:** preserve CanBeOccupied garrison, Building inherited
   Cargo, and Unit transport cargo as distinct gated mechanisms even if Rust
   reuses internal storage.
6. **Effect/RNG owner:** execute class effects through one ordered scenario-RNG
   transaction. Allocation gates and same-pass registrations/removals must be
   committed where native does so; app/render presence cannot decide gameplay
   lifetime.
7. **Pending-state owner:** merge OilExplosion PostMortem and infantry C4/Ivan
   into one signed-i32 latch/timer/source mechanism, cancel it at Building
   IronCurtain entry, serialize semantic state, and hash native-equivalent
   remaining duration/latch/source identity.
8. **Raw-byte policy owner:** keep `+0x52C` explicitly unresolved. Authority
   requires a documented oracle/schema exception or a verified byte policy; a
   normal Rust field initialized to zero is not evidence of parity.
9. **Snapshot owner:** coordinate the new authoritative fields and result state
   with the single snapshot-version/rebaseline owner; do not insert fields
   without version and hash review.

### 8.3 Minimum acceptance scenarios after blockers close

- `test_infantry_old_fear_1_to_99_resets_to_100_before_return`
- `test_infantry_cyborg_immediate_flag_can_uninit_inline`
- `test_unit_transport_ejects_passable_rider_and_kills_only_failed_ejection`
- `test_unit_death_list_draw_precedes_allocation`
- `test_aircraft_death_list_draw_requires_successful_allocation`
- `test_inline_unit_uninit_compacts_logic_and_skips_shifted_successor`
- `test_building_explodes_zero_duration_survives_receiver_until_own_update`
- `test_building_deferred_path_runs_spawn_survivors_twice`
- `test_building_center_smudge_dimension_draw_matrix_2x2_3x2_2x3_3x3`
- `test_postmortem_d34_d35_boundary_returns_5_with_hp_one`
- `test_postmortem_equal_or_longer_hit_keeps_timer_but_repeats_callbacks`
- `test_postmortem_strictly_shorter_preserves_existing_c4_source`
- `test_building_iron_curtain_cancels_shared_pending_latch`
- `test_postmortem_expiry_reenters_receiver_with_current_hp_and_c4_packet`

These are regression/contract tests until their expected traces are derived from
active `gamemd.exe`; hand-computed values alone do not certify parity.

## 9. Exact unresolved helpers, fields, and reachability

### 9.1 2A blockers retained

1. Exact semantic names, animation/effect output, and allocation-failure behavior
   for every Infantry `InfDeath 1..10` switch arm are **PARTIAL**.
2. Unit crate/nearby-cell helper output is **PARTIAL**, though its position before
   final membership is verified.
3. Exhaustive per-class Scatter early-return/RNG behavior is **DEFERRED**.
4. Stock reachability of several Cyborg, Crashable-Infantry, laser-fence, and
   TS-legacy type gates is **DEFERRED**.

Source: 2A:194-210, 277-278, 400-415.

### 9.2 2B blockers retained

1. Helper `0x0048DED0` has a verified call predicate and order but **UNKNOWN**
   semantic role, state writes, output, and RNG behavior.
2. Downstream effects/RNG inside generic Scatter, AnimClass,
   ParticleSystemClass, PlaceTiberium, and smudge helpers remain **UNCHECKED**.
3. CABHUT's effective merged `Immune` value for the BridgeRepairHut conjunction
   remains **UNCHECKED**.
4. The apparent main-death-animation allocation-failure null dereference is an
   assembly observation, not a verified runtime failure-frequency claim.

Source: 2B:399-407. These gaps do not invalidate the surrounding order, but they
do prevent an exhaustive authoritative effects/RNG contract.

### 9.3 2C blocker retained

`Building+0x52C` remains raw persisted, process-history-dependent UNKNOWN state.
Its writer provenance, persistence, expiry non-use, and checksum omission are
verified; its exact value is not. This is an exact-byte authority blocker, not a
request to infer a nicer semantic field (2C:28-45, 332-353, 470-491, 762-793).

## 10. Task 3 fixture boundary

Task 2 supplies the receiver-side expected state once a concrete packet reaches
the wrapper. It does **not** supply an end-to-end producer fixture. Task 3 must
provide, without approximation:

1. `0x00489280` target collection order, filters, layers, deduplication, fixed
   record lifetime, and the exact seven receiver arguments;
2. native Cartesian world-lepton coordinate conversion and the final signed
   per-target distance for ground, air, and Building targets;
3. normal projectile/effect insertion and detonation scheduler position,
   including same-frame appended-bullet and delayed-impact cases;
4. death-weapon, radiation, and lightning argument provenance, RNG owner,
   tick position, recursion, and any persistent provenance state;
5. stock `OilExplosionWH` calls against `CAMISC01`, `CAMISC02`, and `AMMOCRAT`,
   proving actual `ignoreDefenses`, distance, target order, and frame position
   before comparing the receiver checkpoints.

This boundary is explicit in the Task 3 plan
(`2026-07-13-damage-authoritative-cutover-plan.md:761-839`) and in
2C:595-612. All three eligible stock targets are also `Insignificant=yes`, so an
invented ordinary call with guessed `ignoreDefenses` is not an acceptable
fixture.

## 11. Negative facts / do not do

- Do not use `0x005227F0` or `0x004D6FA0` as active concrete receiver anchors.
- Do not call Building `+0x4EC` Limbo; it is DestructionEffects.
- Do not map every result `4` to immediate deletion or every Infantry result `4`
  to deferred deletion.
- Do not equate observer/reference detach with Logic-vector removal.
- Do not choose death lifetime from `has_animation` or any render asset.
- Do not batch inline Unit/Aircraft/ordinary-Building removal after the damage
  pass and claim scheduler equivalence.
- Do not collapse CanBeOccupied garrison, Building Cargo, and Unit transport
  cargo into one death branch.
- Do not move Unit destruction selection after allocation or Aircraft selection
  before allocation.
- Do not omit the second deferred Building `SpawnSurvivors` call because it
  appears accidental.
- Do not guarantee one Building survivor or consume both center dimension draws
  for every `>=2x2` footprint.
- Do not route result `5` into Building result-4 destruction.
- Do not model C4 and OilExplosion delay as independent timers.
- Do not clear/extend the PostMortem timer on equal or longer repeats.
- Do not normalize `+0x52C` to zero or name it seed/facing/padding.
- Do not promote stock receiver checkpoints to end-to-end parity until Task 3,
  G2, and the retail Oracle supply native producer traces.

## 12. Gate and authority verdict

| Boundary | Verdict | Reason |
|---|---|---|
| Task 2 behavioral map | **PARTIAL but implementation-usable for verified rows** | class dispatch, result bands, main lifecycle, and PostMortem behavior are reconciled; bounded helper/presentation gaps remain |
| Task 2 authority | **FAILED / BLOCKED** | 2A presentation/helpers, 2B `0x0048DED0` and generic helpers, and 2C raw `+0x52C` violate the plan's no-authority-critical-UNKNOWN acceptance |
| G1 receiver evidence | **NOT PASSED by Task 2S** | affected class routes must remain shadow-only until the named gaps are closed or explicitly bounded by an approved non-authoritative policy |
| G2 projectile timing | **OPEN / FAILED independently** | Task 2 proves receiver behavior only; the separate implemented projectile-impact scheduler and exact adapter are still required |
| Task 3 fixture handoff | **REQUIRED** | producer arguments, signed distances, collection order, RNG provenance, and tick position are outside Task 2 |
| G3 retail Oracle | **REQUIRED** | active retail traces remain necessary for authoritative ordered writes/calls/RNG/membership and the `+0x52C` policy |

The correct next use of this document is as the canonical Task 2 handoff and
blocker list. It does not authorize the damage cutover, Task 14 authority work,
or a claim that G2 has passed.

## 13. Evidence closure

This reconciliation read 2A, 2B, and 2C in full, plus the Task 2 acceptance and
the directly conflicting prior Building, Infantry, and DelayKill sections. No
fresh Ghidra call was needed because the new reports' raw-vtable, instruction,
and INI evidence resolve the contradictions without ambiguity. No claim above
upgrades a child `PARTIAL` finding, and no report or source file other than this
sole Task 2S output was modified.
