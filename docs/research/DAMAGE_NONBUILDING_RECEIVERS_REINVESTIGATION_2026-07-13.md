# Damage Non-Building Receivers Reinvestigation — 2026-07-13

**Scope:** active concrete `ReceiveDamage` wrappers for `FootClass`,
`InfantryClass`, `UnitClass`, and `AircraftClass` in the current Yuri's Revenge
`gamemd.exe`.

**Status:** **PARTIAL**. Wrapper identity, forwarding, result-band dispatch,
membership timing, and direct RNG order are verified. The complete Infantry
death-presentation switch and a few leaf-helper meanings remain partial.

**Non-scope:** `ObjectClass`/`TechnoClass` common damage math and callbacks,
`BuildingClass`, post-mortem deletion, and Rust implementation.

## 1. Verdict

The four concrete wrappers are not interchangeable pass-throughs:

- `FootClass` owns parasite/Sonic pre-processing and team/mission reactions.
- `InfantryClass` owns Cyborg and airborne `InfDeath=9` gates, fear/scatter,
  death presentation, and normally deferred removal.
- `UnitClass` owns the war-factory immunity gate, miner alerts and reactions,
  sinking/crashing, cargo/crew handling, destruction RNG, and normally
  synchronous removal.
- `AircraftClass` owns the fatal UnitLost notification, destruction animation,
  and crash-or-UnInit decision.

All four forward the same seven arguments in the same order:

```text
pDamage, distance, warhead, attacker, ignoreDefenses, arg6, sourceHouse
```

The shared `ObjectClass`/`TechnoClass` transaction is covered by
`DAMAGE_RECEIVER_OBJECT_CALLBACKS_REINVESTIGATION_2026-07-13.md` and
`DAMAGE_RECEIVER_TECHNO_GATES_REINVESTIGATION_2026-07-13.md`; this report starts
at the concrete wrappers.

## 2. Receiver identity — current bytes, not inherited labels

| Class | RTTI TypeDescriptor string | vtable | slot `+0x16C` | Receiver |
|---|---|---:|---:|---:|
| Foot | `.?AVFootClass@@` | `0x007E8C94` | `0x007E8E00` | `0x004D7330` |
| Infantry | `.?AVInfantryClass@@` | `0x007EB058` | `0x007EB1C4` | `0x00517FA0` |
| Unit | `.?AVUnitClass@@` | `0x007F5C70` | `0x007F5DDC` | `0x00737C90` |
| Aircraft | `.?AVAircraftClass@@` | `0x007E22A4` | `0x007E2410` | `0x004165C0` |

Evidence: live `read_memory` of each vtable slot, vtable-minus-four complete
object locator, locator TypeDescriptor pointer, and TypeDescriptor name.

### Required label corrections

- `0x005227F0` is **not** `InfantryClass::ReceiveDamage`. It is referenced by
  Infantry vtable slot `+0xC8`; the receiver slot contains `0x00517FA0`.
- `0x004D6FA0` is inside `FootClass::Mission_AreaGuard`, not a damage receiver.
  The Foot receiver is `0x004D7330`.
- Ghidra has no function boundary at `0x00517FA0`; Infantry findings below are
  from current raw bytes using read-only `disassemble_bytes(dry_run=true)`.

## 3. Result-band overview

`result` below is the state returned by the immediate base wrapper.

| Wrapper | `0` | `1..3` | `4` | `5` |
|---|---|---|---|---|
| Foot | return | team/mission postlude | team callback only if teamed; otherwise return | immediate return |
| Infantry | immediate return | engineer/scatter/fear postlude | Infantry fatal path | immediate return |
| Unit | return | miner alert, crusher response, low-health dock response | Unit fatal path | immediate return |
| Aircraft | return unchanged | return unchanged | Aircraft fatal path | immediate return |

No wrapper remaps the returned state after its class effects.

## 4. `FootClass::ReceiveDamage @ 0x004D7330`

### 4.1 Pre-base order

The following uses the raw incoming `*pDamage`, before Techno/Object transforms:

1. If `warhead != null`, `warhead+0x14B Sonic != 0`, and
   `victim+0x694 ParasiteAttacker != null`:
   - call `WarpAttachClass::Detach @ 0x0062A4A0` through the parasite attacker's
     manager at `attackerFoot+0x69C`;
   - if the damage attacker is non-null, call its virtual `+0x3C8(0)`.
2. If a parasite attacker exists, it is not the damage attacker, and its
   `Type+0xD6C SuppressionThreshold < *pDamage`, re-arm that parasite manager's
   damage timer:
   - `+0x2C = current frame`;
   - `+0x30 = the wrapper's uninitialized local dword`;
   - `+0x34 = *pDamage * 2 - SuppressionThreshold`.
3. If a parasite attacker exists and `*pDamage < 0`, write the same timer start
   and middle dword, set duration `+0x34 = 50`, then detach.
4. Forward all seven arguments to `TechnoClass::ReceiveDamage` through the
   verified Foot/base chain. The direct call is at `0x004D742C`.

The odd timer middle-dword write is literal current-binary behavior; it must not
be normalized to zero without equivalence proof.

### 4.2 Post-base order

- Result `5`: return immediately.
- Any nonzero result with `victim+0x5D4 Team != null`: synchronously call
  `0x006EB380(team, victim, result, attacker)` and return the same result.
- With no Team, only results `1..3` can reach the remaining postlude. If
  MissionControl entry byte `+4` is true and byte `+5` is false, call victim
  virtual `+0x484(0, 1)`.
- Results `0` and `4` have no remaining Foot-only effect.

A later Team check leading to `0x00708080` is dominated by the earlier
`Team != null -> callback -> return` branch and is unreachable from this
receiver. Older prose that presents it as a live Foot "rescue" action is stale.

### 4.3 Membership and RNG

- Foot itself never calls `UnInit` here.
- The parasite detaches and team callback are synchronous.
- No RNG draw occurs in the verified Foot wrapper or its scoped postlude.
- Stock activity is live: Sonic/parasite behavior covers the Dolphin/Giant
  Squid interaction; Team reactions are conditional but active.

## 5. `InfantryClass::ReceiveDamage @ 0x00517FA0`

### 5.1 Pre-Foot gates

1. If instance byte `+0x6DB` is set, raw damage is positive, and
   `ignoreDefenses == false`, replace damage with
   `ftol(*pDamage * warhead+0xF8)` and clamp the result to at least `1`.
   The body dereferences the warhead on this path without a null guard.
2. Then, if `warhead != null`, `warhead+0x120 InfDeath == 9`, and victim virtual
   `+0x1C8()` (height) is positive, overwrite `*pDamage = 0`.
3. Forward all seven arguments to Foot at `0x00518042`.

The second gate is reachable with stock `[Mutate]` and `[MutateExplosion]`, both
of which specify `InfDeath=9` in `ini/rulesmd.ini`. The Cyborg path is
conditional/mod-facing; no stock `Cyborg=yes` reachability was established here.

### 5.2 Results `1..3`: engineer reaction, Scatter, fear

For every nonzero, nonfatal result:

1. If the victim owner is not player-controlled, `InfantryType+0xEC3 Engineer`
   is true, and current action (`vtable+0x184`) is `5` or `0xB`, queue mission
   `0xF`.
2. If attacker is non-null, call victim `vtable+0x174` (Infantry Scatter) with
   the attacker's coordinate and zero/zero flags.
3. Apply fear at `Infantry+0x6D4`:
   - attacker non-null and current fear `< 100`: Fraidycat (`Type+0xEBF`) sets
     `300`; otherwise Fearless (`Type+0xEBC`) or ability `0xD` blocks the write;
     otherwise set exactly `100`;
   - attacker null or current fear `>= 100`: if not Fearless/ability `0xD`, add
     `50`, reduce to `25` above `ConditionRed`, halve again above
     `ConditionYellow`, then clamp at `300`.
4. Return the original `1`, `2`, or `3`.

The `< 100 -> 100` rule includes fear values `1..99`; it is not only a
zero-to-100 initialization.

### 5.3 Result `4`: verified lifecycle spine

The fatal path performs, in order:

- remove the infantry from a linked `SlaveManagerClass` entry when present;
- run the special object-at-`+0x11C` callback when its type gate passes;
- call victim `+0x3B8(attacker)` (the verified UnitLost notification helper);
- call victim `+0x500()`, `+0x3A0()` (`FootClass::StopFiring`), queue missions
  `-1` then `5`, call `MissionClass::Commence @ 0x005B3570`, and call
  `EMPPassengers(attacker) @ 0x00707CB0`;
- select the death action/animation from height/water state, Cyborg/JumpJet and
  NotHuman flags, `InfDeath`, a type-owned animation list, and special attacker
  type gates.

The type-owned animation list is deterministic here; the wrapper does not use
RNG to choose an entry.

### 5.4 Default deferred removal vs exceptional immediate removal

The wrapper initializes a local immediate-cleanup flag to zero. It sets the flag
only when all of these are true:

- `ignoreDefenses != 0`;
- `InfantryType+0xEAC Cyborg != 0`;
- victim byte `+0x8F != 0`.

Final dispatch:

- flag `0`: return `4` without `UnInit`; the infantry remains in the live Logic
  membership while its death action runs, and the later Infantry sequencer calls
  `FootClass::UnInit @ 0x004DE5D0` when the death animation completes;
- flag `1`, `Crashable == false`: call `UnInit` before returning `4`;
- flag `1`, `Crashable == true`: call virtual `+0x3DC(0)`; retain the object if
  that returns true, otherwise call `UnInit`.

Thus "all infantry deaths are deferred" is too broad, but deferred removal is
the ordinary path.

### 5.5 Infantry presentation table — PARTIAL

The raw switch table at `0x00518D58` maps `InfDeath 1..10` to
`0x005185D5`, `0x00518635`, `0x00518647`, `0x0051869D`, `0x005186F7`,
`0x0051875A`, `0x005187C0`, `0x00518826`, `0x005188AE`, and `0x00518B3E`.
The first two issue actions `0xB` and `0xC`; several later entries construct
fixed Rules-owned AnimTypes. Exact semantic names and all allocation-failure
presentation differences were not expanded before the closure checkpoint.

This row is **PARTIAL**, not an implementation-ready animation mapping.

### 5.6 Infantry RNG

- No direct RNG call occurs in `0x00517FA0..0x00518D52`.
- The nonfatal virtual Scatter can conditionally consume its own class RNG draw.
- The exceptional Crashable `+0x3DC` path can consume the crash helper's three
  rocking draws when its internal gates pass.

## 6. `UnitClass::ReceiveDamage @ 0x00737C90`

### 6.1 Pre-Foot immunity gate

When `ignoreDefenses == false`, return `0` without calling Foot if:

- `FootClass::GetDestination(0)` is a Building;
- its `BuildingType+0x16BD WeaponsFactory` flag is set; and
- the Building found in the Unit's current cell is that exact destination.

This protects a Unit while it is still inside its war-factory destination.

### 6.2 Results `1..3`

The Unit postlude preserves the base result and runs these ordered reactions:

1. If result is nonzero, `UnitType+0xE0E Harvester` is true, and owner equals
   the local player, create radar event type `4` at signed cell coordinates; if
   event creation succeeds, play `EVA_OreMinerUnderAttack`.
2. Return without autonomous reaction when Team is non-null, attacker is null,
   Unit byte `+0x418` is set, attacker is allied, or owner is player-controlled.
3. Otherwise call `0x007438F0`. This is a crusher counteraction predicate; when
   true, set attacker as target (`+0x480(attacker,1)`) and queue mission `2`.
4. Otherwise, for `Harvester` or `Weeder`, require virtual `+0x2B8() > 0`,
   health ratio at or below `Rules+0x1700`, and an acceptable type-owned `Dock=`
   entry. Send radio command `2`; on reply `1`, install the target and queue
   mission `7` (enter/dock). The exact semantic name of `+0x2B8` is not asserted
   here.

### 6.3 Result `4`: branch order

1. Tear down reciprocal link `Unit+0x2E4` when it points to a Building.
2. If `UnitType+0xE20 DeathFrames > 0`, initialize `Unit+0x6D8` once, issue
   UnitLost once, write Health=`1` and Alive=`1`, then join common fatal cleanup.
3. Otherwise select delayed sinking only when all are true:
   `Naval`, not `Underwater`, not `Organic`, `Weight >= ShipSinkingWeight`,
   current cell LandType=`2` water, and Unit byte `+0x271 == 0`. This calls
   UnitLost, restores Health/Alive to `1`, sets `Unit+0x3CD IsSinking=1`, stops
   firing, and joins common fatal cleanup.
4. Otherwise issue UnitLost. If height `<= 10`, byte `+0x8F` is true, and the
   current cell is water, construct `Rules+0x94` at the exact coordinate and the
   last `Rules+0xBC4` animation at Z+5, with no RNG. Otherwise call
   `UnitClass::Death_Explosion @ 0x00738680`.

### 6.4 Common fatal cleanup

The common tail performs:

- virtual `+0x124(0)`, which resolves to `TechnoClass::DoCloak(0) @ 0x004D3780`;
- `CargoClearAllInOpenTransport @ 0x007104C0` when type `OpenTopped` is set;
- `EMPPassengers(attacker)` when height is greater than `208`;
- for non-Crashable transports, pop cargo one passenger at a time:
  - successful ejection requires `ignoreDefenses == false`, victim byte
    `+0x8F == 0`, and `CanEnterCell` result `0` or `2`;
  - copy bridge state, place with a facing derived from the transport facing,
    clear passenger link, optionally clear open-topped cross-owner targeting,
    Scatter, restore Team/mission handling, and selection;
  - otherwise call passenger virtual `+0xE0(attacker)` and then passenger
    `+0xF8()` (UnInit);
- optionally create an explicit survivor type from `Unit+0x338`, or a default
  crew survivor when `arg6 == false`, type `Crewed` is set, type `+0x5E0 == 0`,
  and the crew-probability draw passes; place it, give it randomized health,
  Scatter, set mission, selection, and tag transfer;
- conditionally run the carries-crate placement/overlay adjustment path.

The nearby-cell/crate leaf identities are **PARTIAL**, but their position before
the final membership decision is verified.

### 6.5 Final membership decision

- `Crashable == true`: call virtual `+0x3DC(0)`. If it returns true, retain the
  Unit in live Logic membership in the crash path; if false, call UnInit now.
- `Crashable == false` and `IsSinking == true`: retain it for later sinking AI.
- `Crashable == false` and not sinking: call UnInit synchronously.

`+0x3DC` resolves to `0x004DEBB0`. It is not an EMP receiver: when height is
positive it installs the airborne/crash death state, performs kill/tag effects,
sets state bytes, can set rocking, and calls `Detach_From_All_Lists`; that last
helper does **not** unregister the Logic vector. A false return leads to the
explicit UnInit call.

### 6.6 Unit RNG order

`UnitClass::Death_Explosion` has two independent type animation lists:

- if count `Type+0x73C > 0`, consume one `Random::Next` **before allocation**,
  even when `Explodes`/ability `10` later overrides the chosen entry to the last
  list item; then allocate and construct if allocation succeeds;
- if count `Type+0x758 > 0`, consume one `Random::Next` **before allocation**,
  then allocate and construct if possible.

Additional conditional draws:

- default crew eligibility: one `RandomRanged(0, 0x7FFFFFFE)`; explicit survivor
  index skips this draw;
- successfully placed crew: one `RandomRanged(5, crewStrength/2)` for health;
- each passenger/crew Scatter may consume the concrete Scatter implementation's
  draw when its gates reach RNG;
- a successful non-Infantry, non-map-editor `+0x3DC` crash transition consumes
  three rocking `RandomRanged` draws.

## 7. `AircraftClass::ReceiveDamage @ 0x004165C0`

Aircraft forwards all arguments to Foot. Results `0..3` return unchanged,
result `5` returns immediately, and only result `4` has Aircraft-only work.

Fatal order:

1. Call virtual `+0x3B8(0)`, which resolves to `0x004D98C0`. This is the
   UnitLost notification helper: for a human owner, non-Spawned type, and valid
   cell, it creates radar event type `7` and plays `EVA_UnitLost`.
2. If destruction-list count `Type+0x73C > 0`, allocate `0x1C8` bytes first.
   Only if allocation succeeds, consume exactly one `Random::Next`, select
   `Type+0x730[rng % count]`, and construct one AnimClass at the aircraft
   coordinate with flags `0x600`.
3. Call virtual `+0x3DC(attacker)` (`0x004DEBB0`).
4. If `+0x3DC` returns false, call virtual `+0xF8`, which resolves to
   `FootClass::UnInit @ 0x004DE5D0`. If true, return `4` with the aircraft still
   in live Logic membership for its crash/fall continuation.

For an airborne non-Infantry aircraft outside map-editor mode, the successful
crash helper also consumes three rocking `RandomRanged` draws. If height is not
positive, the helper returns false without those draws and the wrapper UnInits.

The Aircraft list ordering differs deliberately from Unit death explosions:
Aircraft allocates before its list draw; Unit consumes each list draw before
allocation.

## 8. Membership timing summary

| Class/path | Logic membership on wrapper return | Mechanism |
|---|---|---|
| Foot only | unchanged | no UnInit in wrapper |
| ordinary Infantry fatal | retained | death sequencer later calls UnInit |
| exceptional Cyborg immediate flag, non-Crashable | removed synchronously | wrapper calls UnInit |
| exceptional Cyborg immediate flag, successful crash | retained | `+0x3DC` true; list detach is not Logic unregister |
| ordinary Unit fatal | removed synchronously | wrapper calls UnInit |
| Unit sinking | retained | `+0x3CD` sinking continuation |
| Unit successful crash | retained | `+0x3DC` true |
| grounded/failed Aircraft crash | removed synchronously | wrapper calls UnInit |
| successful airborne Aircraft crash | retained | `+0x3DC` true |

`FootClass::UnInit -> ObjectClass::UnInit` performs the lifecycle removal.
`Detach_From_All_Lists @ 0x007258D0` is observer/target-reference cleanup and
must not be treated as equivalent Logic-vector unregister.

## 9. Rust-facing handoff

Current Rust applies batched damage with `saturating_sub` and then routes deaths
primarily by `has_animation` in `src/sim/combat/mod.rs:1849-1897` and
`src/sim/combat/mod.rs:998-1029`. That does not express these class wrappers.

| Verified native wrapper behavior | Current Rust evidence | Verdict / touchpoint |
|---|---|---|
| Foot Sonic/parasite detach and suppression timer | no parasite runtime found; Warhead `Sonic` is not represented on the damage path | **DRIFT** — rules + Foot damage transaction |
| Infantry post-Cyborg damage scaling and airborne `InfDeath=9 -> 0` | damage is directly saturated from queued amount | **DRIFT** — `src/sim/combat/mod.rs:1849-1876` |
| Any attacker hit with fear `<100` resets it to `100` unless blocked | Rust sets 100 only when fear is exactly 0, then returns unchanged for `1..99` | **DRIFT** — `src/sim/infantry.rs:63-73` |
| Nonfatal Infantry calls Scatter toward attacker before fear | no on-hit Scatter dispatch found in combat | **DRIFT** — combat-to-movement handoff |
| Infantry ordinary death deferred; exceptional immediate/crash paths are flag-driven | Rust uses `has_animation` as the main discriminator | **DRIFT/UNCHECKED** — `src/sim/combat/mod.rs:998-1029` |
| Unit inside exact WeaponsFactory destination cell ignores ordinary damage | no pre-damage factory/destination gate in the inspected combat path | **DRIFT** — combat preflight + docking/contact state |
| Human Harvester radar/EVA, AI crusher response, low-health Dock response | no equivalent class postlude found | **DRIFT** — combat result callbacks / miner mission routing |
| Ordinary non-Crashable Unit ejects passable cargo; only failed/forbidden ejections die | Rust explicitly kills all non-garrison transport riders | **DRIFT** — `src/sim/combat/mod.rs:907-951` |
| Unit sinking/crash/DeathFrames/crew/crate branches and exact RNG order | no single equivalent receiver transaction; some parsed flags exist (`harvester`, `crashable`, `crewed`, `open_topped`) | **DRIFT/UNCHECKED** — combat lifecycle and scenario RNG |
| Aircraft UnitLost event and retained crash state | no equivalent damage-receiver crash transaction found in combat/aircraft paths inspected | **DRIFT/UNCHECKED** — combat + aircraft mission/lifecycle |

Implementation should first model a class-aware post-base result transaction;
patching individual visuals into the current `has_animation` branch would not
preserve state, ordering, membership, or RNG.

## 10. Do-not-do list

1. Do not use `0x005227F0` or `0x004D6FA0` as receiver anchors.
2. Do not re-run Object/Techno math inside each wrapper; each concrete wrapper
   forwards once to the common core.
3. Do not implement the dominated `0x00708080` Foot branch as a live rescue
   reaction.
4. Do not equate `Detach_From_All_Lists` with Logic-vector UnInit/removal.
5. Do not classify every result `4` as immediate removal: ordinary Infantry,
   sinking Units, and successful Unit/Aircraft crashes are retained.
6. Do not classify every Infantry result `4` as deferred: the verified
   Cyborg/immediate flag can lead to inline UnInit.
7. Do not choose death lifetime from render asset presence (`has_animation`).
8. Do not move the Unit destruction-list RNG draws after allocation; do not move
   the Aircraft destruction-list draw before allocation.
9. Do not simplify native fear `<100` to a `fear==0` initialization.
10. Do not silently treat the partial Infantry death-presentation table as a
    complete InfDeath mapping.

## 11. Open questions and closure

- **[DEFERRED / bounded-cost]** Exact semantic names and complete observable
  output for every Infantry `InfDeath 1..10` switch arm and allocation failure.
  Next step: a dedicated Infantry death-presentation investigation anchored at
  `0x00518D58`, not a broad damage-core pass.
- **[DEFERRED / leaf helper]** Exact crate/nearby-cell helper output inside the
  Unit fatal tail. Its ordering before final UnInit is verified.
- **[DEFERRED / indirect RNG]** Exhaustive per-class Scatter early-return and RNG
  table for every possible ejected passenger. This report records the virtual
  calls and direct receiver draws, but does not duplicate the Scatter system.
- **[DEFERRED / reachability]** Stock-map reachability of several Cyborg,
  Crashable-Infantry, laser-fence, and TS-legacy type gates.

No open question changes the receiver addresses, argument forwarding,
result-band routing, ordinary membership timing, or direct RNG ordering above.

## 12. Evidence ledger

Live Ghidra, current `/gamemd.exe`, read-only:

- `read_memory` of vtables `0x007E8C94`, `0x007EB058`, `0x007F5C70`,
  `0x007E22A4`, their complete object locators and TypeDescriptors;
- `decompile_function 0x004D7330`, `0x00737C90`, `0x004165C0`;
- `disassemble_bytes 0x00517FA0..0x00518D52, dry_run=true` and raw switch table
  `read_memory 0x00518D58`;
- helper decompiles: `0x004D3780`, `0x004D5660`, `0x004D98C0`, `0x004DE5D0`,
  `0x004DEBB0`, `0x0050B730`, `0x005B3570`, `0x006B0A20`, `0x00738680`,
  `0x007438F0`, `0x007258D0`;
- string bytes at `0x00824784` = `EVA_OreMinerUnderAttack` and verified
  `EVA_UnitLost` use inside `0x004D98C0`.

Local corroboration read, not substituted for live bytes:

- `docs/research/DAMAGE_RECEIVER_OBJECT_CALLBACKS_REINVESTIGATION_2026-07-13.md`
- `docs/research/DAMAGE_RECEIVER_TECHNO_GATES_REINVESTIGATION_2026-07-13.md`
- `docs/research/PARASITE_CLASS_GHIDRA_REPORT.md`
- `docs/research/TARGETDEATH_INFANTRY_DEATH_SEQUENCE_DEFERRED_REMOVAL_RESWARM_20260528.md`
- `docs/research/TARGETDEATH_UNITCLASS_VEHICLE_DEATH_ACTIVE_VECTOR_TIMING_RESWARM_20260528.md`
- `ini/rules.ini`, `ini/rulesmd.ini`
- current Rust files cited in section 9.

No Ghidra labels, functions, comments, or types were modified.
