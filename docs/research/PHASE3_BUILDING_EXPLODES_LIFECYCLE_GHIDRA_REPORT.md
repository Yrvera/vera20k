# Phase 3 Building `Explodes=yes` Burst and Retained-Update Lifecycle — Ghidra Report

**Date:** 2026-08-29  
**Program:** active retail Yuri's Revenge 1.001 `gamemd.exe` (`x86`, little-endian, image base `0x00400000`)  
**Primary addresses:** `BuildingClass::DestructionEffects @ 0x004415F0`, `BuildingClass::Update @ 0x0043FB20`  
**Supporting addresses:** `BuildingClass::ReceiveDamage @ 0x00442230`, `TechnoClass::ReceiveDamage @ 0x00701900`, `BuildingClass::SpawnSurvivors @ 0x00442D90`, `BuildingClass::Limbo @ 0x00445880`, `ObjectClass::Limbo @ 0x005F4D30`, `ObjectClass::UnInit @ 0x005F65F0`, `BuildingClass::Place_OccupyMap @ 0x00441F60`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `LogicClass::UnregisterObject @ 0x0055BAE0`  
**Investigation mode:** exhaustive slice (`re-investigate`), research only  
**Rust baseline:** `d6f9aecade79b7d155f1eb163db9c9ff1d77fba3`  
**Status:** **VERIFIED — implementation-ready**  
**Confidence:** High for field identity, complete active consumer inventory, burst coordinates/gates/RNG/constructor row, zero-duration retention, cleanup ordering, repeated survivor consequences, scheduler mutation, active-retail reach, persistence/checksum behavior, and the current Rust delta.

## 1. Scope and completion boundary

This report treats BuildingType `Explodes=yes` as an ownership hypothesis and closes every active way that the same native `TechnoTypeClass+0xD15` byte changes a Building's fatal-destruction transaction:

1. the four-offset overlay-conditioned `FIRE3` burst inside `BuildingClass::DestructionEffects`;
2. the shared `TechnoClass::ReceiveDamage` fatal gate that destroys carried passengers and admits the death-weapon helper before Building destruction effects;
3. the `Selling OR Explodes` zero-duration timer write in `DestructionEffects`;
4. the resulting retained-live Health-zero Building, its own later `BuildingClass::Update`, and the `Limbo -> second SpawnSurvivors -> UnInit -> Place_OccupyMap` cleanup sequence.

The slice includes the exact survivor/crew/smudge RNG consequences that are repeated only because `Explodes` retains the Building for its own Update. It also establishes the active-stock exclusion for the overlay-conditioned `FIRE3` burst.

This report does not reopen the already researched `Explosion=`/`DestroyAnim=` constructors, stored-resource spill, destruction particle-system internals, wall/power teardown internals, generic death-weapon detonation internals, generic Anim AI, or UnitClass death animation. Their ordering at this boundary is recorded where it matters, but their internal parity remains owned by their existing rows.

No load-bearing question remains `UNKNOWN`, `UNCHECKED`, approximate, or deferred.

## 2. Verdict

The native field is unequivocally `Explodes`, not a reactor/radiation flag. `TechnoTypeClass+0xD15` defaults false, is read from the `Explodes=` key using the current field as the default, and has exactly eight instruction accesses program-wide. For Buildings, three runtime reads matter: generic fatal passenger/death-weapon admission, the four-cell destruction burst, and the zero-duration cleanup timer.

The mission value paired with `Explodes` in the timer OR is raw `0x13 = Selling`, not repair. Native writes a zero-duration timer for `Selling OR Explodes`; ordinary nonselling, non-Explodes Buildings receive duration 8 and are immediately uninitialized by the fatal receiver. A duration-zero Building is deliberately returned alive and represented with Health zero, still in Logic and still occupying the map. Its own later Building Update runs the ordinary prefix, detects the expired zero timer, then executes:

`BuildingClass::Limbo -> BuildingClass::SpawnSurvivors (second invocation) -> ObjectClass::UnInit -> BuildingClass::Place_OccupyMap -> return`.

The second `SpawnSurvivors` is not a harmless duplicate. The first invocation has already run at the end of `DestructionEffects`. For a Type-Explodes Building, generic fatal processing has already destroyed all cargo, so both survivor calls see empty cargo. Each call nevertheless recomputes a fresh local crew budget. If that budget is nonzero, each independently walks the entire ordered foundation, rolls for crew, and produces the passable-cell debris/smudge branch and all of its Scenario RNG. Thus an ordinary, non-`ignore_defenses` fatal hit on a side-eligible `NANRCT` gets two independent crew/smudge passes. A forced C4-style `ignore_defenses` fatal packet sets the persistent survivor-suppression latch first, so both calls skip the foundation walk. The other nine effective stock BuildingTypes have no crew budget and native emits no per-foundation survivor smudge pass at all.

Current Rust preserves the parsed Building `explodes` field and death-weapon admission and already implements the separately owned Building destruction animations. It does not preserve the duration-zero live Building, does not allow the Building's own Update to remove it, purges cargo under the wrong gate, ejects `YAPPPT` garrison occupants instead of destroying them, spawns Building crew once after UnInit, and emits one survivor-smudge foundation pass for every destroyed Building regardless of native crew eligibility.

The native `FIRE3` burst is compiled and exact but unreachable in the mounted active retail corpus: 250 active OverlayTypes, zero OverlayType `Explodes=` assignments in canonical rules or any of 184 extracted maps, therefore zero eligible overlay types and zero eligible overlay cells. Active-stock implementation must consume zero RNG and construct zero burst Anims. Adding mod-only overlay `Explodes` support is not required to close this active-retail row, but any future implementation must use the exact row below.

## 3. Evidence method and authorities

- Queried the project research index first for `Explodes`, `+0xD15`, Building destruction, survivor, damage-receiver, and scheduler evidence.
- Freshly decompiled and inspected assembly for every load-bearing native function listed above.
- Repeated a full-program instruction operand census for `0xD15`: eight matches across 1,163,302 decoded instructions, not truncated.
- Read raw burst-table bytes at `0x00818CB8`: `00 00 00 00 02 00 00 00 04 00 00 00 06 00 00 00`.
- Verified parser string identity (`"Explodes" @ 0x0083355C`), defaults, overlay parser, mission string table, vtable callbacks, save/load, and checksum paths.
- Parsed canonical `ini/rulesmd.ini` (SHA-256 `3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF`) and every one of the 184 mounted extracted retail maps under `target/phase3-retail-census/extract`.
- Inspected the Rust tree directly at baseline HEAD; no claim relies on stale prose alone.
- Performed adversarial contradiction and cold zero-add passes after the first synthesis.

The live Ghidra instance remained connected to project `testProsjekt`, program `/gamemd.exe`, throughout the final cold pass. No Ghidra metadata was changed.

## 4. Mandatory stale-document corrections

### 4.1 `TechnoTypeClass+0xD15` is `Explodes`

The fresh program-wide census is:

| Address | Function | Access | Meaning |
|---:|---|---|---|
| `0x00441A31` | `BuildingClass::DestructionEffects` | read | four-cell overlay burst gate |
| `0x00441C4E` | `BuildingClass::DestructionEffects` | read | zero-duration timer OR arm |
| `0x0070258F` | `TechnoClass::ReceiveDamage` | read | fatal cargo/death-weapon admission |
| `0x007114A8` | `TechnoTypeClass::Constructor` | write | default false |
| `0x007122BE` | `TechnoTypeClass::ReadINI` | read | current value passed as `ReadBool` default |
| `0x007122D2` | `TechnoTypeClass::ReadINI` | write | store `Explodes=` result |
| `0x00717705` | type checksum helper | read | `CRCEngine::AddBool` |
| `0x007386C3` | `UnitClass::Death_Explosion` | read | Unit-only consumer; excluded for Buildings |

There are no additional `+0xD15` readers or writers. Earlier prose that labels this byte radiation, reactor, or a Nuclear Reactor special is wrong.

### 4.2 Raw mission `0x13` is `Selling`

The native mission-name pointer table entry for index 19 points at string `"Selling" @ 0x00816DC4` through table slot `0x00816CF8`. Rust independently declares `MissionType::Selling = 19` in `src/rules/mission_data.rs`.

Therefore the branch at `0x00441C43..0x00441C56` is exactly:

`if currentMission == Selling || Type.Explodes { duration = 0 } else { duration = 8 }`.

Any label of mission `0x13` as repair is wrong.

## 5. Field default, parser, layering, and checksum

`TechnoTypeClass::Constructor` writes zero to `+0xD15`. `TechnoTypeClass::ReadINI @ 0x00712170` loads the current byte, passes it with the section name and `"Explodes"` to `CCINIClass::ReadBool`, and stores the returned byte. This has two consequences:

- a malformed or missing later-layer value preserves the current field;
- ordinary Rules/mode/map processing is sequential field update, not parent-type inheritance.

There is no other BuildingType inheritance mechanism for this field. A later map section that omits `Explodes` leaves the prior Rules value intact; an authored `Explodes=no` clears it.

Rust matches the active data semantics:

- `src/rules/object_type.rs:463` stores `explodes: bool`;
- `src/rules/object_type.rs:1457` reads `Explodes` with default false;
- `src/rules/ini_parser.rs:411` overlays only authored keys, so a map section that omits the key preserves the existing exact key/value.

The native type checksum helper at `0x007171A0` feeds `+0xD15` to `CRCEngine::AddBool`. Rust's effective RuleSet/config hash must continue to distinguish this value.

## 6. Exact four-offset `FIRE3` burst

### 6.1 Offset table and coordinate helper

The gate at `0x00441A31` is `Type+0xD15 != 0`. The loop reads four dwords from `0x00818CB8` in literal order and terminates when the pointer reaches `0x00818CC8`:

| Iteration | Raw direction | Initialized direction delta | Result |
|---:|---:|---:|---|
| 0 | `0` | `(0,-256)` | one cell north |
| 1 | `2` | `(256,0)` | one cell east |
| 2 | `4` | `(0,256)` | one cell south |
| 3 | `6` | `(-256,0)` | one cell west |

The direction table is initialized at `0x0049F3A0..0x0049F413`. `FUN_0049F550 @ 0x0049F550` applies the chosen delta without RNG:

- `targetX = baseX + dx`
- `targetY = baseY + dy`
- `targetZ = groundHeight(targetXY) + (baseZ - groundHeight(baseXY))`

Thus the burst preserves the Building's height above the ground across a one-cell orthogonal ring. Order is **N, E, S, W**.

### 6.2 Cell resolution and overlay eligibility

For each target coordinate, `MapClass::Get_CellClass_At_Coord @ 0x00565730` converts X/Y to cell coordinates with the native sign-biased `/256` truncation, forms `index = y*512+x`, and accepts only `0 <= index < MapClass+0x140` with a real Cell pointer. Failure returns the shared dummy Cell.

The caller deliberately resolves the cell twice:

1. first lookup: require `Cell+0x44 != -1` (a real overlay type index);
2. second lookup: index `g_OverlayTypeClass_Array[Cell+0x44]` and require byte `OverlayTypeClass+0x2B0 != 0`.

`OverlayTypeClass+0x2B0` is its own `Explodes` field, not a generic eligibility flag. Its constructor defaults false. `OverlayTypeClass::ReadINI @ 0x005FE770` reads `Explodes=` with the current byte as default and stores it at `+0x2B0`.

A no-overlay target, a target routed to the dummy Cell, or a nonexploding overlay skips allocation and consumes no RNG.

### 6.3 Allocation, RNG, lookup, and constructor order

For an eligible overlay cell:

1. allocate exactly `0x1C8` bytes;
2. allocation failure skips the rest of this cell with **no Scenario RNG draw**;
3. call Scenario `RandomRanged(1,3)` exactly once;
4. call `AnimTypeClass::FindByIndex @ 0x00427CB0` with literal string pointer `0x00818E00 = "FIRE3"`;
5. use the returned AnimType registry index to fetch `g_AnimTypes_Array[index]`;
6. construct `AnimClass(type=FIRE3, coord=target, delay=Ranged+3, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

The delay is therefore 4, 5, or 6. `FindByIndex` performs a case-insensitive name scan in AnimType declaration order and returns the array index or `-1`; it is not a random selector. Active retail declares `FIRE3`.

There is no call to `AnimClass::SetOwnerObject` after construction. The constructor initializes the owner-object link to null, and no instruction between the constructor call at `0x00441B1A` and the loop advance at `0x00441B1F` changes it. Layer is the AnimType's art-owned layer.

Both `ini/art.ini` and `ini/artmd.ini` author the same active row:

```ini
[FIRE3]
Layer=ground
Damage=.003
LoopCount=5
Rate=450
UseNormalLight=yes
Translucency=50
```

There is no `RandomRate`, bounce, meteor, or other constructor-time random key. Because caller delay is nonzero, construction does not synchronously enter `Middle/Start`. The burst consumes exactly one Scenario `RandomRanged(1,3)` per eligible allocated cell and no other Scenario draw at this boundary. Later ordinary FIRE3 AI/damage behavior is owned by the generic Anim subsystem.

### 6.4 Active-retail exclusion

The exhaustive installed-data scan found:

- 250 canonical `OverlayTypes`;
- zero canonical OverlayType sections with `Explodes=`;
- zero map-local OverlayType sections with `Explodes=` across all 184 extracted maps.

Therefore the active eligible OverlayType set is empty, and the active eligible overlay-cell set is necessarily empty regardless of OverlayPack contents. For active retail, every Type-Explodes Building skips all four burst allocations, consumes zero burst RNG, and creates zero `FIRE3` Anims.

This is an evidence-backed active-stock exclusion, not proof that the compiled arm is dead for mods.

## 7. Generic fatal `Explodes` coupling: cargo before death weapon

`TechnoClass::ReceiveDamage @ 0x00701900` reads `Type+0xD15` at `0x0070258F`. Admission to the shared fatal block is true when any of these hold:

- Type `Explodes=yes`;
- active veteran/elite Explodes ability; or
- the object's current selected weapon has `Suicide=yes`.

For a Type-Explodes Building the first arm is sufficient. At `0x00702603` the helper loops while cargo head `Techno+0x118` is non-null:

1. inspect the current head;
2. if it belongs to a Team, call `TeamClass::Remove_Member`;
3. pop the head through `FUN_00473430 @ 0x00473430` (stable head pop, decrement count, clear popped `next`);
4. recursively call `FootClass::EMPPassengers @ 0x00707CB0` on the popped Foot;
5. call the popped object's virtual `+0xE0` damage/death callback;
6. call its virtual `+0xF8` `UnInit`;
7. repeat until cargo is empty.

Only after cargo is empty does `FUN_0070D690` run the shared death-weapon helper. Building-specific `DestructionEffects` follows later through the Building receiver wrapper. Thus, for Type-Explodes Buildings:

- every carried passenger is synchronously destroyed before death-weapon detonation;
- every passenger is gone before the first `BuildingClass::SpawnSurvivors`;
- neither the first nor second survivor call can eject that cargo.

The Explodes/Suicide/ability admission and the cargo-before-death-weapon order are owned by this field boundary. Weapon selection, warhead detonation, radial damage, and their internal RNG are the existing generic death-weapon mechanism and remain outside this report.

Rust's `death_weapon_aoe` gate in `src/sim/combat/mod.rs:1622` matches the effective Type/veterancy/Suicide admission. Rust's `BeforeDeathEffects` hook in `src/sim/world/mod.rs:1990`, however, purges cargo for every fatal Unit/Structure regardless of that gate and special-cases occupied/absorber Buildings to eject their cargo. For the active Type-Explodes set, ordinary cargo purge happens to match; `YAPPPT` (`CanBeOccupied=yes`) is a direct opposite result and must destroy, not eject, its occupants.

## 8. Timer writes and `Selling OR Explodes` precedence

After the burst, stored-resource spill, cost callback, and other destruction work, `DestructionEffects` queries current mission through Building vtable `+0x184`:

- `0x00441C39..0x00441C46`: if mission is raw `0x13` (`Selling`), take the zero arm without needing to read Explodes;
- otherwise `0x00441C48..0x00441C56`: read `Type+0xD15`; true also takes the zero arm;
- only when both are false does the duration-8 arm run.

Both arms write the shared timer at Building `+0x528/+0x52C/+0x530`:

| Field | Duration-8 arm | Selling/Explodes zero arm |
|---:|---|---|
| `+0x528` | `g_CurrentFrameCounter` | `g_CurrentFrameCounter` |
| `+0x52C` | current stack scratch dword | same |
| `+0x530` | `8` | `0` |

The zero arm additionally resets the Building animation timer quartet:

| Field | Value |
|---:|---:|
| `+0x100` | `g_CurrentFrameCounter` |
| `+0x104` | same current stack scratch dword |
| `+0x108` | `0` |
| `+0x10C` | `0` |

The stack scratch at `[ESP+0x44]` is overwritten by the burst coordinate local. When the burst runs, the last direction is west and its Y equals the original Building Y; that dword becomes the value stored at `+0x52C/+0x104`. No expiry or checksum reader consumes those opaque scratch fields. They raw-persist only and may be omitted from Rust if no native-visible reader is introduced.

The `+0x100/+0x108/+0x10C` reset is behavior-visible during the retained Building's Update. `BuildingClass::UpdateAnimation @ 0x004509D0` runs before the Health-zero cleanup, asks the timer for remaining time, and advances the current animation frame only when remaining is zero **and** `+0x10C != 0`. The zero arm therefore prevents that frame advance; the later inactive/selling branch can set Building `+0x6DD` when the animation state is absent or `+0x10C==0`. A Rust retained-update path must not advance a Building animation from this reset timer before removal.

## 9. Fatal `BuildingClass::ReceiveDamage`: why duration zero retains the object

The fatal Building wrapper calls virtual `+0x4EC` (`BuildingClass::DestructionEffects`) at `0x00442665`. Assembly `0x00442651..0x00442665` proves the four explicit arguments are `(0, attacker, ignore_defenses, saved_foundation)`. Near the end of `DestructionEffects`, a nonzero `ignore_defenses` argument writes `Building+0x6E0 = 1` at `0x00441F0B` before the first survivor call. The latch is constructor-false and has no clearing writer during this fatal lifetime. It therefore suppresses both the first and retained-Update survivor budgets. Rust C4 expiry already submits `ignore_defenses=true`; ordinary weapon damage submits false. The wrapper then evaluates the shared timer:

```text
start = Building+0x528
duration = Building+0x530
if start != -1:
    elapsed = currentFrame - start
    if elapsed >= duration: return from the fatal branch
    remaining = duration - elapsed
if remaining > 0:
    virtual +0xF8 UnInit
    BuildingClass::Place_OccupyMap
return
```

Assembly anchors are `0x0044266B..0x004426A7`. At the same frame for duration 0, `elapsed=0` and `elapsed >= duration` is true, so the function jumps directly to its common return. For duration 8, remaining is positive and the wrapper immediately calls `UnInit` and `Place_OccupyMap`.

At the return from a Type-Explodes fatal hit, the Building has:

| Fact | Native state |
|---|---|
| Health | `0` (`Building+0x6C`) |
| Object alive | true (`+0x90` not yet cleared) |
| InLimbo | false (`+0x81` not yet set) |
| Logic membership | true (`+0x98` not yet cleared) |
| map/foundation occupancy | retained |
| owner/house membership | retained |
| cargo | empty from the generic fatal gate |
| first `SpawnSurvivors` | already complete |
| death weapon and destruction effects | already complete; must not repeat |

This is not a deferred copy or a presentation corpse. It is the same live Building object, still scheduled.

## 10. The retained Building's own `Update`

The Health-zero test is not the entry of `BuildingClass::Update @ 0x0043FB20`. A retained Type-Explodes Building receives a real ordinary Building visit first. Before the test at `0x00440072`, native can execute:

1. active/mission-dependent looping-sound maintenance and `BuildingClass::UpdateGapAndSpecialEffects`;
2. damage-ratio evaluation and damage-fire creation/removal;
3. turret/attachment/temporal and Building animation-slot maintenance;
4. `BuildingClass::UpdateAnimation @ 0x004509D0`, including the zeroed timer behavior established above;
5. Building mission/Techno AI through `MissionClass::TechnoClass::AI_Update`;
6. the post-AI `IsAlive` check;
7. gattling/cache and current-animation-state maintenance;
8. only then the current Health read.

The retention mechanism therefore requires the Building to remain in the ordinary Logic scheduler and traverse the implemented ordinary Building-update prefix. It is not equivalent to a detached end-of-frame cleanup queue. Internal parity of each generic prefix subsystem remains in its own row, but skipping an already implemented prefix for this retained Building would be a new Explodes-specific ordering regression.

At Health zero, `0x00440076..0x00440099` clears the same eight damage-fire Anim owner slots again. The timer predicate at `0x0044009B..0x004400BB` is the same normalized-remaining calculation used by the fatal wrapper. For duration zero it is expired immediately. The exact cleanup call sequence is:

1. `0x004400C5`: virtual `+0xD4` -> most-derived `BuildingClass::Limbo @ 0x00445880`;
2. `0x004400D4`: `BuildingClass::SpawnSurvivors @ 0x00442D90` (second invocation);
3. `0x004400DD`: virtual `+0xF8` -> `ObjectClass::UnInit @ 0x005F65F0`;
4. `0x004400E5`: `BuildingClass::Place_OccupyMap @ 0x00441F60`;
5. immediate return at `0x004400F1`.

No death weapon, `DestructionEffects`, Building death sound, `Explosion=`, `DestroyAnim=`, storage spill, or destruction particle selection repeats in this Update.

## 11. Limbo, second survivor pass, UnInit, and `Place_OccupyMap`

### 11.1 First virtual `Limbo`

`BuildingClass::Limbo @ 0x00445880` runs Building-specific removal before delegating through the Techno/Object chain. Its active work includes damage-fire cleanup, Building/House count and power/special-vector maintenance, walls/reservations/foundation-border state, owner recount/base-center work, and the common object removal.

`ObjectClass::Limbo @ 0x005F4D30` checks that the object is active and not already in limbo, then synchronously:

- deselects it;
- calls virtual Destroy with argument 1;
- calls Mark/Remove;
- removes it from display/layer and owned sound state;
- unregisters it from the Logic vector through `LogicClass::UnregisterObject`;
- dirties its representation;
- sets `InLimbo=true`.

It does **not** clear `ObjectClass+0x90 IsAlive`. Therefore the second survivor call runs with the Building in limbo and out of Logic/map occupancy, but still alive and not yet UnInit.

This placement is load-bearing: the first survivor pass occurs before Limbo while Building occupancy remains; the second occurs after Limbo. Crew unlimbo attempts and `CellClass::CheckCellPassability` can consequently see different cell availability. The second pass cannot be modeled by copying the first pass's already-computed results.

### 11.2 `ObjectClass::UnInit`

After the second survivor pass, `ObjectClass::UnInit @ 0x005F65F0` executes:

1. defuse an attached bomb if present;
2. for a Foot object, recursively process any remaining passengers;
3. dispatch direct pointer-expiry/reference cleanup;
4. call virtual `+0xD4` Limbo again (an early-out because `InLimbo` is already true);
5. clear `IsAlive` at `+0x90`;
6. append the object to the deferred finalization/deletion vector.

The double virtual Limbo call is intentional but only the first performs removal.

### 11.3 Post-UnInit `BuildingClass::Place_OccupyMap`

`BuildingClass::Place_OccupyMap @ 0x00441F60` is called **after** UnInit in both the ordinary immediate-removal arm and the retained own-Update arm. It consumes the still-resident Building allocation and type/foundation data. When its foundation-data gate succeeds, it walks the sentinel-terminated foundation, performs the `FUN_00486E70` cell operation, writes cell `+0x44=0xEF` and `+0x40=0`, recalculates attributes and orphaned/nearby zone graphs, detaches target references/restores missions, writes the origin cell `+0x40` type pointer, dirties the tactical rectangle, and runs the final cell helper. Its wall-height-adjust arm is gated by BuildingType bytes `+0x1767/+0x1769`.

Those cell semantics belong to the existing placement/occupancy mechanism; this report fixes only the non-negotiable ordering: first Limbo, second SpawnSurvivors, UnInit, then `Place_OccupyMap`. Rust must invoke its equivalent post-UnInit cell transaction once, not before the second survivor pass and not twice.

## 12. Exact repeated `BuildingClass::SpawnSurvivors`

### 12.1 Invocation sites and cargo result

The first call is at `0x00441F1B`, after `DestructionEffects` sets Health to zero and before `FootClass::EMPPassengers` on the Building. The second is at `0x004400D4`, after the retained Building's first Limbo and before UnInit.

`SpawnSurvivors` has its own cargo-pop arm for absorber/carry-capable types, but Type Explodes already emptied cargo in generic `TechnoClass::ReceiveDamage`. Both invocations therefore enter with cargo count/head zero. There is no cargo ejection, placement, passenger mission assignment, or cargo RNG in either pass for a Type-Explodes fatal Building.

This is why `YAPPPT`'s occupied-building status does not rescue its occupants: the Explodes fatal gate annihilates them before Building destruction effects.

### 12.2 Fresh crew budget on every invocation

Each call initializes a new local denominator and new local budget. No decrement from the first call is stored back to the Building.

Denominator:

```text
denominator = 2
if Building+0x540 != 0: denominator = 1
if Building+0x6E3 != 0: denominator += 6
```

Budget comes from Building vtable `+0x2D0`, resolved to `FUN_00451330`:

```text
if Building+0x6E0 != 0: 0
if Type+0xCCD Crewed == false: 0
side = Owner+0x1E8
divisor = Rules+[0x14F8,0x14FC,0x1500][side 0,1,2]
other side or divisor 0: 0
if Building+0x6E3 != 0: divisor *= 2
adjustedCost = Type virtual +0xB8(Owner,0)
budget = clamp(adjustedCost / divisor, 1, 5)
```

The three instance inputs are now closed rather than left as anonymous flags:

- `+0x540` is the retained source-object pointer shared with the Building C4/PostMortem timer. The Building constructor writes null at `0x0043B789`; Infantry C4 planting writes the planter at `0x0051A5FD`; pointer expiry clears an exact match at `0x0044E904..0x0044E910`; Iron Curtain/Force Shield and the consumed bridge-hut branch also clear it. Both survivor call sites push this field. A direct conventional fatal hit normally has null and therefore base denominator 2. A conventional fatal hit while a hostile C4 is already armed retains the planter, uses base denominator 1, and may target that source in the survivor postlude. C4 expiry itself uses `ignore_defenses=true`, sets `+0x6E0`, and therefore suppresses the entire budget before the denominator can affect output.
- `+0x6E0` is the fatal-call survivor-suppression latch at this boundary: constructor false; `DestructionEffects` copies nonzero `ignore_defenses` into true at `0x00441F0B`; `FUN_00451330` returns zero when it is true. No clear occurs before either survivor call. This is why forced C4 destruction emits neither crew nor survivor-foundation smudges from either invocation.
- `+0x6E3` is a sticky successful-owner-change latch. A fresh full-program operand census found the constructor false write at `0x0043B96C`, the only runtime true writer in `BuildingClass::ChangeOwner @ 0x00448260` at `0x00448723`, the three survivor-related reads at `0x00442DD7`, `0x0044EB13`, and `0x00451395`, one adjacent repair/power read, and the checksum read at `0x00454431`; no runtime clear writer exists. A real transfer to a different owner sets it after the ownership transaction. It adds 6 to the chance denominator, doubles the cost divisor, and suppresses the otherwise unconditional 25% ConYard Engineer-type roll.

Therefore the second Explodes pass recomputes the same initial 1..5 ceiling from then-current inputs. It is not limited by how many crew the first call successfully spawned. The three latches/pointers do not change between the first and second calls in this fatal path.

### 12.3 Ordered per-foundation crew transaction

The entire foundation walk is skipped when the **initial** budget is zero. Otherwise the sentinel-terminated vtable `+0x108` foundation list is visited in native order. For each cell:

1. if owner byte `+0x1F6` is clear, current budget is positive, and `RandomRanged(0,denominator)==1`, attempt a crew spawn;
2. resolve the crew type through Building vtable `+0x30C -> FUN_0044EB10`;
3. allocate `0x6F0` and construct Infantry if the type is non-null;
4. try the native per-cell Infantry placement/unlimbo;
5. only on successful unlimbo, decrement the local budget and draw `RandomRanged(5, InfantryType+0xA0)` for survivor Health;
6. assign the native owner/source-dependent mission/target postlude.

`FUN_0044EB10` has its own exact RNG:

- when Building `+0x6E3==0`, it **always** draws `RandomRanged(0,99)` on an admitted crew attempt; if the draw is below 25 and BuildingType `+0xEB8==7`, it returns Rules `+0xF70` (Engineer);
- otherwise it calls `FUN_00707D20`.

`FUN_00707D20` first requires `Crewed`, chooses Rules `+0xF78/+0xF7C/+0xF80` (Allied/Soviet/Third crew) from owner side, and falls back to Rules `+0xF6C` (Technician) for other sides. When owner-type `+0xBC==-1`, Technician is returned without a draw. Otherwise a virtual Building predicate at `+0x2AC` gates one more `RandomRanged(0,99)`; a value below 15 returns Technician, otherwise the side crew remains.

Allocation or placement failure retains the already-consumed chance/type draws but skips budget decrement and Health RNG.

The postlude uses the same `+0x540` source passed by both call sites. With a null or allied source, an AI-controlled owner assigns raw mission `0x0F` (`Hunt`) and a human-controlled owner assigns raw `2` (`Move`). With a hostile source, native assigns raw mission `1` (`Attack`) and sets that source object as target. The active way to obtain a non-null source without suppressing the budget is to plant C4 and then kill the Building conventionally before the forced C4 expiry packet.

Before placement, a separate conditional copies one native bookkeeping flag: when Building `+0x6E9` is true and the chosen InfantryType has `Nominal=yes` (`Type+0xC9E`), native writes survivor `+0x6D9 = 1`. Fresh tracing resolves `+0x6E9` as set by `BuildingClass::Init_Managers` when the BuildingType primary SHP getter returns non-null; `+0x6D9` prevents the resulting nominal Infantry from incrementing the normal House type count. This branch is evidence-backed **inactive for the active Explodes set**: the only Crewed type is unarmed, nonoccupiable `NANRCT`, so its weapon-equipped 15% Technician arm is false; stock side crew `E1/E2/INIT` and `ENGINEER` all have `Nominal=no`; neutral-side `NANRCT` has zero crew budget. Rust need not add general `Nominal` survivor bookkeeping to close this active-retail row, but it must not claim that conditional compiled behavior is absent.

### 12.4 Ordered per-foundation debris/smudge transaction

For every foundation cell in an entered walk—whether a crew was attempted, failed, succeeded, owner is defeated, or the budget has since fallen to zero—native then calls `CellClass::CheckCellPassability`. A true result performs:

1. `RandomRanged(0,99)`;
2. `FUN_0049F420(radius=0x80,snapFlag=0)`, which consumes exactly one raw Scenario `Random::Next` for direction;
3. snap the returned X/Y to the resolved cell center;
4. if the first roll is below 50, call `SpawnDebris(100,0)`; otherwise call `Debris_Smoke(100,0)`;
5. the selected helper enumerates placeable burn/crater SmudgeTypes and size preferences;
6. when at least one candidate exists and the `0xB0` Smudge allocation succeeds, consume `RandomRanged(0,candidateCount-1)` and construct that Smudge.

Neither helper reduces tiberium in this Building survivor call. Candidate-vector memory failures can suppress later selection; no selection draw occurs without a candidate and successful Smudge allocation.

The active draw order for each entered cell is therefore:

`optional crew-chance -> optional 25% type roll -> optional 15% fallback roll -> optional Health roll after successful unlimbo -> smudge 0..99 -> raw direction Next -> optional SmudgeType selection`.

The first and second calls each repeat that complete stream independently, with the passability difference caused by intervening Limbo.

### 12.5 Active Explodes consequences

Only `NANRCT` among the ten effective active BuildingTypes explicitly has `Crewed=yes`. The 22 preplaced instances split into 15 `Neutral` (Civilian side, so `FUN_00451330` returns zero), five Soviet-side (`Africans:1`, `Arabs:2`, `Confederation:1`, campaign `BadGuy2:1`), and two Third-side (`YuriCountry:2`). There are no map-local `NANRCT` overrides for `Crewed`, weapon, occupancy, image, or Explodes.

For the seven side-eligible preplaced instances with stock Cost 1000 and no owner cost multiplier, an uncaptured Building has initial budget 4 on Soviet divisor 250 and budget 1 on Third divisor 750. A later capture sets sticky `+0x6E3`; the new side's divisor is doubled and the chance denominator becomes 8 when `+0x540` is null (7 when it is non-null). Clamp-to-one means a valid side still retains at least one budget. When `+0x6E0` is clear, each call independently walks the foundation; when it is set, both calls skip it. The owner-defeated byte suppresses crew attempts but does not suppress the smudge work after an already admitted nonzero-budget walk.

The other nine effective types default/hard-set `Crewed=no`. Their initial budget is zero, so native performs **no** survivor foundation walk and emits **no** per-foundation survivor debris/smudge from either call. The earlier large-building center branch is separate and still runs once when its own geometry gate admits it.

## 13. Logic scheduler mutation and exact timing

`LogicClass::PerTickUpdate @ 0x0055AFB0` reaches the active Logic vector loop at `0x0055B5FF..0x0055B619`:

```text
index = 0
while index < liveCount:
    object = liveData[index]
    object->virtual +0x5C Update()
    liveCount = vector.count        // re-read after callback
    index++
```

`LogicClass::UnregisterObject @ 0x0055BAE0` is a stable erase: find the exact object, shift every later pointer left by one, decrement count, and clear object `+0x98`.

Consequences for a fatal Explodes Building:

- If fatal damage happens before the scheduler reaches the Building's current Logic index, the retained Building can receive its own cleanup Update later in the **same** Logic pass.
- If fatal damage happens after the Building's index has already run, cleanup waits until the next Logic pass.
- If damage is delivered outside this vector loop before the pass, the Building is available for that pass normally.
- The cleanup Update's first Limbo unregisters the Building synchronously.
- Stable erase shifts its immediate successor into the just-visited index. The outer loop then increments, so that successor is skipped for this pass.

This compact-vector successor skip is part of native determinism. A snapshot of Logic IDs, an end-of-tick despawn list, or swap-remove produces a different callback order.

Paused simulation performs no Logic callback and advances no Scenario frame, so a retained Building remains represented until simulation resumes. This is ordinary scheduler behavior, not another Explodes timer arm.

## 14. Save, load, checksum, and replay state

Active save chaining is `BuildingClass::Save @ 0x00454190 -> TechnoClass::Save -> RadioClass::Save -> AbstractClass::Save @ 0x00410320`. `AbstractClass::Save` writes the live receiver pointer as a four-byte swizzle token, then writes the entire virtual-size receiver block. The raw fields at `+0x528/+0x52C/+0x530`, `+0x100..+0x10C`, `+0x540`, `+0x6E0`, `+0x6E3`, and `+0x6E9`, plus Health, `IsAlive`, `InLimbo`, and Logic membership, therefore persist in a save made during the retained window.

`AbstractClass::Load @ 0x00410380` reads the saved token, registers the reconstructed object with the Swizzle manager, raw-reads the receiver block while preserving the live slot at `+0x1C`, and lets derived Load methods swizzle pointer fields. `BuildingClass::Load` explicitly registers `+0x540` for pointer swizzling at `0x00453F56`. A saved Type-Explodes Building with duration zero resumes as the same Health-zero, alive, in-Logic pending cleanup object, with the same suppression/owner-change/source inputs, and follows the restored Logic order.

`BuildingClass::Save_ChecksumFields @ 0x00454260` normalizes timer `+0x528/+0x530` to remaining duration and feeds that dword to `CRCEngine::AddInt32`; it does not hash raw start or `+0x52C`. An Explodes duration-zero timer contributes exactly 0. It hashes the `+0x540` referent identity when non-null, directly adds bools `+0x6E0` and `+0x6E3`, and does not add `+0x6E9`. The parent Techno/Object checksum covers the object's other represented state, while the type checksum covers the Explodes bool.

Current Rust already serializes `GameEntity`, independently serializes/rebuilds Logic order, and hashes:

- exact Logic order (`src/sim/world/world_hash.rs:543`);
- lifecycle `object_alive/in_limbo/cell_marked` (`:1246..1248`);
- Health (`:1309..1310`);
- passenger-role/cargo state (`:1625` and following);
- the existing `PendingC4Detonation.source_entity_id`, including serialization and hashing.

The implementation needs one persisted, hashed pending Building fatal-cleanup state (or an exactly equivalent authoritative state) so a Health-zero represented Building neither reruns death effects nor disappears on load. It also needs a persistent per-Building owner-changed latch set at the authoritative `change_owner` chokepoint, because capture can precede death by arbitrarily many frames. The fatal pending state must preserve the `ignore_defenses` survivor-suppression result and retain/reuse the existing C4 source identity through both survivor calls. The expected schema change is `SNAPSHOT_VERSION 121 -> 122`. Hash the future-affecting pending phase/normalized remaining duration, owner-change latch, suppression latch, and retained source identity; do not hash the behavior-opaque `+0x52C/+0x104` scratch values. A save/load round trip in the retained window must preserve Logic position and cleanup timing exactly.

## 15. Full active-retail data census

The scanner parsed all 403 values in canonical `[BuildingTypes]`, all 250 values in `[OverlayTypes]`, every type section, and all 11,992 `[Structures]` rows in the 184 mounted extracted retail maps. Map type sections were applied as later Rules passes for authored `Explodes=` overrides.

### 15.1 Effective BuildingTypes and instances

Nine canonical BuildingTypes author `Explodes=yes`. `xeb2.map` adds one map-local BuildingType, producing ten effective active types and 1,794 preplaced instances across 148 maps.

| Effective type | Source | Crewed | Cargo special | Death weapon selection | Instances |
|---|---|---|---|---|---:|
| `AMMOCRAT` | canonical yes | default false | none | explicit `BarrelExplosion` | 17 |
| `CAMISC01` | canonical yes | default false | none | explicit `BarrelExplosion` | 89 |
| `CAMISC02` | canonical yes | default false | none | explicit `BarrelExplosion` | 92 |
| `CAOILD` | canonical yes | default false | none | explicit `OilExplosion` | 1,554 |
| `GAYARD` | canonical yes | explicit no | none | shared Rules default | 2 |
| `INGRNLMP` | `xeb2.map` yes | explicit no | none | map-local `specialrad` | 8 |
| `NANRCT` | canonical yes | explicit yes | none | explicit `NukePayload` | 22 |
| `NAYARD` | canonical yes | explicit no | none | shared Rules default | 3 |
| `YAPPPT` | canonical yes | default false | `CanBeOccupied=yes` | explicit `BlimpBombEffect` | 1 |
| `YAYARD` | canonical yes | explicit no | none | shared Rules default | 6 |
| **Total** |  |  |  |  | **1,794** |

Only `NANRCT` reaches a positive native crew budget from active type data. `YAPPPT` is the active proof that Type Explodes takes priority over garrison ejection.

### 15.2 Every map-local `Explodes=` assignment

The entire 184-map corpus contains exactly four assignments:

| Map | Section | Value | Effect |
|---|---|---|---|
| `all04dmd.map` | `CAMISC01` | yes | repeats canonical yes; no preplaced instance in this map |
| `all04dmd.map` | `CAMISC02` | yes | repeats canonical yes; one instance |
| `sov01umd.map` | `DNOAA` | no | AircraftType, not BuildingType |
| `xeb2.map` | `INGRNLMP` | yes | introduces the tenth effective BuildingType; eight instances |

No other map changes the Building field. No canonical or map OverlayType authors `Explodes=`.

### 15.3 Affected-map ledger

This is the complete nonzero per-map count ledger. A bare type/count means the map contains only that effective type; mixed rows spell out the full type breakdown.

```text
2peaks 4 CAOILD
all01umd 4 (NANRCT:2,CAMISC01:1,CAMISC02:1)
all02umd 3 CAOILD
all03umd 3 (CAOILD:2,CAMISC01:1)
all04dmd 6 (CAOILD:5,CAMISC02:1)
all05umd 3 (GAYARD:1,YAYARD:2)
all06umd 12 (CAMISC02:4,CAOILD:6,CAMISC01:1,YAYARD:1)
all07smd 4 CAOILD
arena33forever 18 CAOILD
austintx 12 CAOILD
c1a01md 10 (CAOILD:4,NANRCT:1,CAMISC01:3,CAMISC02:2)
c1a02md 4 CAOILD
c1a03md 5 (CAOILD:3,CAMISC01:1,CAMISC02:1)
c2s01md 11 (NAYARD:1,GAYARD:1,CAOILD:3,CAMISC01:3,CAMISC02:3)
c2s02md 16 (CAOILD:10,CAMISC02:2,CAMISC01:4)
c2s03md 7 (NANRCT:2,CAOILD:3,CAMISC01:1,CAMISC02:1)
c3y01md 7 (CAOILD:3,CAMISC01:2,CAMISC02:2)
c3y02md 11 (CAOILD:5,CAMISC02:2,CAMISC01:1,NANRCT:1,NAYARD:2)
c3y03md 14 (CAOILD:6,CAMISC02:6,CAMISC01:2)
c4w01md 7 (YAYARD:2,CAOILD:5)
deathvalleygirl 4 CAOILD
deathvalleygirlmw 20 CAOILD
doubletrouble 4 CAOILD
downtown 4 CAOILD
dryheat 4 CAOILD
dryheatmw 20 CAOILD
eastvsbest 6 CAOILD
facedown 2 CAOILD
fourcorners 5 CAOILD
fourcornersmw 21 CAOILD
frstbite 2 CAOILD
groundze 4 NANRCT
groundzemw 24 (NANRCT:4,CAOILD:20)
hidvally 4 CAOILD
hillbtwn 4 CAOILD
isleofoades 4 CAOILD
manhatta 3 CAOILD
monumentvalley 6 CAOILD
monumentvalleymw 30 CAOILD
nowimps 10 CAOILD
offensedefense 4 CAOILD
ottersrevenge 4 CAOILD
pcofdune 4 CAOILD
rushhr 4 CAOILD
saharami 6 CAOILD
saharamimw 14 CAOILD
sedonapass 6 CAOILD
sedonapassmw 30 CAOILD
sov01umd 5 (YAPPPT:1,CAMISC02:1,CAMISC01:1,NANRCT:2)
sov02smd 63 (CAMISC02:21,CAMISC01:25,AMMOCRAT:17)
sov03umd 8 (CAOILD:4,CAMISC02:4)
sov04dmd 5 (CAOILD:3,NANRCT:2)
sov05umd 9 (YAYARD:1,CAOILD:8)
sov07tmd 1 NANRCT
triplecrossed 6 CAOILD
xamazon01 4 CAOILD
xarena 8 CAOILD
xbarrel 23 (CAOILD:12,CAMISC02:6,CAMISC01:5)
xbayopigs 26 CAOILD
xbermuda 1 NANRCT
xbreak 9 CAOILD
xcarville 16 CAOILD
xdeadman 12 CAOILD
xdeath 21 CAOILD
xdisaster 8 CAOILD
xdustbowl 7 (CAOILD:4,CAMISC01:1,CAMISC02:2)
xdustbowlmw 23 (CAOILD:20,CAMISC01:1,CAMISC02:2)
xeb1 6 CAOILD
xeb1mw 22 CAOILD
xeb2 10 (CAMISC01:1,CAMISC02:1,INGRNLMP:8)
xeb3 4 CAOILD
xeb5 7 (CAOILD:5,NANRCT:2)
xgoldst 17 CAOILD
xgrinder 12 CAOILD
xhailmary 12 CAOILD
xhills 4 CAOILD
xinvasion 16 CAOILD
xkaliforn 12 CAOILD
xkiller 6 CAOILD
xlostlake 16 CAOILD
xmp01du 4 CAOILD
xmp01t4 4 CAOILD
xmp05du 2 CAOILD
xmp05mw 18 CAOILD
xmp05t4 2 CAOILD
xmp06mw 29 (CAMISC01:6,CAMISC02:5,CAOILD:18)
xmp06t2 11 (CAMISC01:6,CAMISC02:5)
xmp08mw 8 CAOILD
xmp10s4 8 CAOILD
xmp12s4 6 CAOILD
xmp13du 3 (CAMISC02:2,CAMISC01:1)
xmp13mw 17 (CAMISC02:2,CAMISC01:1,CAOILD:14)
xmp13s4 3 (CAMISC02:2,CAMISC01:1)
xmp14mw 14 CAOILD
xmp14t2 2 CAOILD
xmp15mw 32 CAOILD
xmp16mw 31 CAOILD
xmp16s4 4 CAOILD
xmp17mw 25 CAOILD
xmp18du 18 (CAMISC01:6,CAMISC02:4,CAOILD:8)
xmp18s3 11 (CAMISC01:2,CAOILD:7,CAMISC02:2)
xmp19t4 4 CAOILD
xmp20mw 37 (CAOILD:36,CAMISC01:1)
xmp20t6 13 (CAOILD:12,CAMISC01:1)
xmp21s2 8 CAOILD
xmp22mw 40 CAOILD
xmp22s8 12 CAOILD
xmp23mw 27 CAOILD
xmp24du 4 CAOILD
xmp24t2 6 CAOILD
xmp25mw 37 (CAOILD:36,CAMISC01:1)
xmp26s6 4 CAOILD
xmp27mw 34 CAOILD
xmp28u4 30 (CAOILD:23,CAMISC01:4,CAMISC02:3)
xmp29mw 17 CAOILD
xmp29u2 3 CAOILD
xmp30mw 26 (CAMISC01:1,CAMISC02:1,CAOILD:24)
xmp30s6 2 (CAMISC01:1,CAMISC02:1)
xmp31s2 16 CAOILD
xmp32mw 33 CAOILD
xmp33u4 10 (CAMISC02:3,CAMISC01:3,CAOILD:4)
xnewhghts 10 CAOILD
xnorest 32 CAOILD
xoceansid 8 CAOILD
xpacific 8 CAOILD
xpacificmw 24 CAOILD
xpotomac 12 CAOILD
xrockets 12 CAOILD
xroulette 28 CAOILD
xround 12 CAOILD
xseaofiso 14 CAOILD
xshrapnel 4 CAOILD
xtanyas 16 CAOILD
xterritor 14 CAOILD
xtn01mw 12 CAOILD
xtn01t2 2 CAOILD
xtn02mw 28 CAOILD
xtn02s4 4 CAOILD
xtn03mw 32 CAOILD
xtn03t8 8 CAOILD
xtn04mw 12 CAOILD
xtn04t2 4 CAOILD
xtower 4 CAOILD
xtowermw 20 CAOILD
xtsunami 8 CAOILD
xvalley 8 CAOILD
xvalleymw 24 CAOILD
xyuriplot 6 CAOILD
```

The remaining 36 maps contain zero preplaced effective Type-Explodes Buildings. The per-type totals above and this ledger independently sum to 1,794.

## 16. Current Rust comparison at `d6f9aeca`

### 16.1 Preserve what already matches

| Native requirement | Current Rust evidence | Verdict |
|---|---|---|
| BuildingType `Explodes` default false and parse | `src/rules/object_type.rs:463,1457` | match |
| later authored map key overrides; omitted key preserves | `src/rules/ini_parser.rs:411..425` | match |
| veteran/elite Explodes plus current Suicide gate | `src/sim/combat/mod.rs:1622..1652` | match |
| explicit/current/default death-weapon selection | same `death_weapon_aoe` helper | match for owned selection boundary |
| raw mission 19 is Selling | `src/rules/mission_data.rs:81,142` | match |
| Building `Explosion=`/`DestroyAnim=` scheduler constructors | `BuildingDestructionAnimPlan` and `commit_building_destruction_anims` | preserve; separate row already implemented |
| Building animation start-smudge interleave | `building_explosion_start_smudge` and scheduler callback | preserve |
| Scenario-owned RNG and smudge dispatcher | existing combat/world hooks | reusable |
| active survivor divisor keys/defaults | `src/rules/ruleset.rs:750..759,2093..2095` | match and reusable |
| retained C4 source identity | `PendingC4Detonation.source_entity_id` plus snapshot/hash support | represented; retain through the new fatal window |
| independent Health/lifecycle/Logic-order hash inputs | current `GameEntity`, LogicVector, and world hash | reusable |

### 16.2 Replace what is wrong

1. `FatalLifecycleStage::BeforeDeathEffects` in `src/sim/world/mod.rs:1990` purges passengers for every fatal Unit/Structure. Native passenger annihilation is admitted only by effective Explodes/Suicide/ability.
2. The same hook ejects occupied/absorber Building cargo. For Type Explodes, including the active `YAPPPT`, native destroys it before Building death effects.
3. `append_building_smudge_requests` in `src/sim/combat/mod.rs:1174` always appends one request for every foundation cell. Native enters that foundation pass only when initial `SpawnSurvivors` crew budget is nonzero. Nine of ten active Explodes BuildingTypes must emit none.
4. `eject_destruction_survivors` in `src/sim/production/production_sell.rs:201` always tries one side crew from one position. Native uses cost/divisor budget 1..5, ordered per-cell chance/type/placement/Health RNG, and can spawn multiple.
5. Rust dispatches that survivor event after combat/world cleanup. Native first-pass crew happens before Limbo/UnInit; Type Explodes repeats the full opportunity after Limbo and before UnInit.
6. `FatalLifecycleStage::AfterDeathEffects` in `src/sim/world/mod.rs:2041` immediately uninitializes every fatal Structure. Native duration-zero Explodes/Selling Buildings remain alive, in Logic, and on the map until their own Update.
7. The authoritative Rust `change_owner_impl` at `src/sim/world/mod.rs:5167` returns early on same-owner requests and performs the real transfer, but stores no sticky Building owner-changed latch. Native sets `+0x6E3` after every successful distinct-owner Building transfer and later consumes/hashes it.

### 16.3 Implement what is missing

- the `Selling OR Explodes` zero-duration pending-cleanup state;
- animation-timer reset semantics needed before the retained own Update;
- retained Health-zero, alive, nonlimbo, in-Logic/map state after fatal return;
- ordinary Building Update prefix followed by the exact Health-zero cleanup transaction;
- second `SpawnSurvivors` after first Limbo and before UnInit;
- compact live-Logic removal timing and successor skip;
- persisted/hashable pending phase with snapshot schema bump;
- exact first/second crew and survivor-smudge mechanics, including `ignore_defenses` suppression, retained C4-source denominator/targeting, and sticky owner-change effects.

`OverlayTypeFlags` in `src/rules/overlay_types.rs` currently has no `Explodes` member and Rust has no four-cell FIRE3 burst. That is a real compiled-mod disparity but not an active-retail gap because the effective active overlay set is empty. Do not expand this row merely to add unused mod support.

## 17. Evidence-backed exclusions and adjacent boundaries

| Candidate mechanism | Classification | Evidence-backed boundary |
|---|---|---|
| `UnitClass::Death_Explosion @ 0x00738680` `+0xD15` read | excluded | most-derived Unit-only consumer, never Building fatal destruction |
| overlay-conditioned FIRE3 burst in active stock | excluded occurrence | zero eligible active overlay types/cells; zero RNG/Anim result |
| later FIRE3 AI, `.003` damage, rate/loops | adjacent | generic Anim subsystem after a hypothetical mod burst |
| generic death-weapon detonation internals | adjacent | Explodes owns admission and cargo-before-call order, not warhead internals |
| `Explosion=`/`DestroyAnim=` | adjacent, preserve | separately verified and implemented; must execute once only |
| large-building center debris/smudge branch | adjacent, preserve | occurs once before per-foundation explosion work; not repeated by second SpawnSurvivors |
| stored-resource spill | adjacent | active branch between burst and timer; independent of `Explodes` except ordering |
| destruction cost/threshold callback | adjacent | independent call before timer |
| destruction particle-system selection | adjacent | independent after timer/DestroyAnim, before Health zero |
| Building wall/power/count/owner callbacks | adjacent internals | Limbo ordering is owned here; individual subsystem semantics remain mapped elsewhere |
| Building trigger/tag callbacks | adjacent | no Explodes-specific additional callback discovered; ordinary death/Limbo callback order remains |
| Selling mission producers/state machine | adjacent | raw ID and timer OR are owned here; how Selling is assigned is not |
| C4 plant/timer mechanics | adjacent, preserve | this row consumes the already represented `+0x540` source and `ignore_defenses` expiry result; planting, shortening, and bridge-hut collapse stay with C4 |
| `+0x6E3` repair/power consumer | adjacent | the sticky latch storage/writer/hash and survivor reads are owned here; its separate repair/power effect remains outside |
| `+0x6E9 && Nominal` survivor count marker | excluded active occurrence | compiled direct crew consequence, but active Explodes crew cannot select a Nominal InfantryType; no schema expansion required for this row |
| raw timer scratch `+0x52C/+0x104` | excluded Rust authority | raw-persisted but no expiry/checksum/behavior reader found |

No excluded mechanism is used to excuse an active ordinary-retail result.

## 18. Rust implementation handoff

### 18.1 Required transaction

The smallest exact active-retail implementation is:

1. Reuse the current effective Explodes/Suicide/ability predicate for native fatal admission.
2. Only when admitted, destroy cargo recursively in stable head order before the death weapon; never route Type-Explodes Building cargo through garrison ejection.
3. Run the existing death weapon once.
4. Run Building destruction effects once, preserving current center-smudge and `Explosion=`/`DestroyAnim=` ordering.
5. Replace the unconditional foundation-smudge list with an exact first `SpawnSurvivors` transaction. Cargo is already empty; compute crew budget from side divisor, sticky owner-change latch, and `ignore_defenses` suppression; then interleave crew and smudge draws per foundation only when initial budget is nonzero. Reuse the retained C4 source for denominator and survivor mission/target postlude.
6. At the authoritative Building owner-transfer chokepoint, set a persistent owner-changed latch only after a real distinct-owner transfer; never clear it during the Building lifetime and include it in snapshot/hash state.
7. For current mission Selling **or** Type Explodes, leave the Building Health zero but alive, nonlimbo, in occupancy and Logic; store pending duration-zero cleanup, the survivor-suppression result, and reset the Building animation timer equivalent. Skip `AfterDeathEffects` UnInit for this object.
8. When the live Logic scheduler reaches the Building, run the ordinary implemented Building Update prefix. Do not advance the reset Building animation timer.
9. At the Health-zero timer check, clear damage-fire slots, call Building Limbo, execute a fresh second exact `SpawnSurvivors` with the same suppression/owner-change/source inputs, call UnInit, run the existing post-UnInit Building cell/placement transaction, and return.
10. Stable-erase the Building from the live Logic vector through Limbo so the native successor-skip behavior occurs.
11. Serialize and hash the pending phase/remaining duration, owner-change latch, survivor-suppression latch, and retained source identity; bump snapshot version to 122. Preserve the current explosion animation objects, smudges, survivor entities, cargo deaths, lifecycle facts, occupancy, and Logic order in the hash.

The compiled four-cell FIRE3 burst may remain unimplemented for this active-retail row, but its absence must be protected by a census regression asserting the eligible overlay set remains empty. If implemented, use the exact N/E/S/W, allocation-before-RNG, fixed FIRE3 constructor row in section 6.

### 18.2 Exact acceptance tests

1. **Field/parser/default:** missing `Explodes` is false; later missing/malformed value preserves current; later yes/no overrides; raw mission 19 resolves Selling.
2. **Consumer inventory fixture:** document/test guard keeps the native `+0xD15` consumer census at eight and classifies the Unit-only arm outside the Building path.
3. **Active census golden:** canonical counts remain 403 BuildingTypes and 250 OverlayTypes; ten effective Explodes BuildingTypes, 1,794 structures, 148 affected maps; zero exploding OverlayTypes and zero map overlay assignments.
4. **Active burst zero-add:** fatal active `CAOILD`, `NANRCT`, and `YAPPPT` beside arbitrary stock overlays consume zero burst RNG and spawn zero FIRE3 Anims.
5. **Optional synthetic burst:** with a synthetic OverlayType `Explodes=yes`, visit N/E/S/W in order, preserve relative Z, reject dummy/no-overlay/nonexploding cells without allocation/RNG, allocate before `Ranged(1,3)`, construct FIRE3 delay 4..6 with loop 1/flags `0x600`/owner none/ground layer, and consume no constructor RNG.
6. **Cargo/death-weapon order:** a Type-Explodes Building with nested passengers destroys cargo depth-first/head-stably before its death weapon and before first survivor work. No passenger can be ejected by either survivor pass.
7. **`YAPPPT` regression:** occupied active `YAPPPT` destroys all occupants; it does not invoke garrison exit selection/ejection.
8. **Immediate post-fatal state:** after Type-Explodes fatal ReceiveDamage returns, assert Health=0, alive=true, in_limbo=false, Logic membership/position unchanged, occupancy/map membership retained, death effects marked complete, and cargo empty.
9. **Non-Explodes control:** a nonselling, non-Explodes Building follows duration 8 and immediate UnInit/placement transaction; it never reaches a retained own Update.
10. **Selling precedence:** a non-Explodes Building already on Selling takes the same duration-zero retained path; Explodes need not be read for the result. A Building both Selling and Explodes still performs one cleanup only.
11. **No effect replay:** retained own Update must not repeat death weapon, death sound, DestructionEffects, center smudge, `Explosion=`, `DestroyAnim=`, storage, or particle creation.
12. **Update-prefix ordering:** a retained Building reaches ordinary implemented sound/effect/animation/mission prefix before Health-zero cleanup, but its reset animation timer does not advance the Building frame.
13. **Two survivor calls:** trace first before Limbo and second after Limbo/before UnInit. Assert second recomputes initial budget rather than using first-pass remainder.
14. **Active `NANRCT` owner census/budget:** protect 22 instances as 15 Civilian-side budget-zero, five Soviet budget-four, and two Third-side budget-one at the initial owner-change-false state. Assert no authored active override changes the result.
15. **`ignore_defenses` suppression:** fatal forced C4 damage sets the suppression latch before the first call. Both calls execute zero foundation cells and consume zero survivor RNG/smudge RNG even for side-eligible `NANRCT`; a conventional control leaves the latch clear.
16. **Sticky capture latch:** a same-owner request leaves it unchanged; a real Building transfer sets it once and later transfers never clear it. With source null, chance denominator changes 2 -> 8 and the cost divisor doubles in both survivor calls; snapshot/hash changes and round-trips.
17. **Retained C4 source:** arm C4, then kill `NANRCT` conventionally before expiry. Both calls use denominator 1 (or 7 after capture); a successfully placed survivor treats a hostile retained source as Attack target. Null/allied source gives human Move or AI Hunt exactly. Pointer expiry before death returns to the null-source branch.
18. **`NANRCT` exact RNG:** with controlled owner side/divisors/foundation/passability, assert two independent per-cell chance/type/Health/smudge sequences and the exact draw order. Include allocation and placement failures, captured/uncaptured variants, and owner-defeated crew suppression without smudge suppression.
19. **Crewless smudge exclusion:** active `CAOILD`, yards, `AMMOCRAT`, `CAMISC01/02`, `YAPPPT`, and `INGRNLMP` execute zero survivor foundation cells and consume zero survivor-smudge RNG in both calls.
20. **Before-Limbo vs after-Limbo passability:** make one occupied foundation cell fail in the first pass and become passable after Limbo; only the second pass may consume that cell's smudge draws/spawn there.
21. **Cleanup callbacks:** assert `Building Limbo -> second SpawnSurvivors -> UnInit (second Limbo early-out) -> post-UnInit Place_OccupyMap`, with IsAlive clearing only in UnInit.
22. **Scheduler before-turn:** fatal damage before the victim's current Logic index permits same-pass cleanup.
23. **Scheduler after-turn:** fatal damage after the victim's visit defers cleanup to the next Logic pass.
24. **Compact successor skip:** victim self-removal stable-shifts the successor; outer increment skips exactly that successor for the current pass, without swap-remove reordering.
25. **Pause/resume:** pausing between fatal return and own Update preserves the represented Building without frame/timer drift.
26. **Save/load:** snapshot in the retained window, reload, and prove identical Logic index, pending phase, owner-change/suppression/source inputs, RNG, lifecycle/occupancy state, later cleanup order, survivor results, and final hash.
27. **Schema/hash:** assert snapshot version 122; toggling pending cleanup, owner-change, suppression, or retained source identity changes the Rust state hash; raw opaque scratch is not introduced as authority; after cleanup the pending phase is absent.
28. **Regression suite:** rerun the existing Building `Explosion=`/`DestroyAnim=` constructor, delay/Start-smudge, death-weapon, C4, garrison, ownership, lifecycle, occupancy, snapshot, and world-hash focused tests. No already-correct constructor order or owner/layer fact may change.

## 19. Open Questions Log

| Question | Resolution |
|---|---|
| Is `TechnoType+0xD15` radiation/reactor state? | **RESOLVED:** no; exact parser string/default/checksum and complete consumer census prove `Explodes`. |
| Is raw mission `0x13` repair? | **RESOLVED:** no; mission table and Rust enum prove Selling. |
| Is the burst table random or diagonal? | **RESOLVED:** literal `{0,2,4,6}` -> N,E,S,W one-cell orthogonal ring. |
| Does coordinate resolution clamp to map? | **RESOLVED:** no; sign-biased cell conversion plus linear bounds, otherwise shared DummyCell. |
| What does OverlayType `+0x2B0` mean? | **RESOLVED:** its own parsed/default-false `Explodes` bool. |
| Is allocation before or after burst RNG? | **RESOLVED:** before; allocation failure consumes no draw. |
| Is FIRE3 selected randomly? | **RESOLVED:** no; literal name lookup in AnimType declaration order. |
| Exact FIRE3 constructor row/layer/owner? | **RESOLVED:** delay 4..6, loop 1, flags `0x600`, zAdjust/reverse 0, art ground layer, owner null; no SetOwnerObject. |
| Can FIRE3 constructor consume more RNG? | **RESOLVED:** not active art; no RandomRate and nonzero delay. |
| Is the burst active in stock maps? | **RESOLVED:** no; zero eligible OverlayTypes/cells in the complete corpus. |
| Does Type Explodes eject or kill Building cargo? | **RESOLVED:** kill recursively before death weapon and Building effects. |
| Does `YAPPPT` use garrison ejection? | **RESOLVED:** no; Explodes fatal cargo annihilation occurs first. |
| Does duration zero mean no timer or delayed removal? | **RESOLVED:** timer exists with duration zero; fatal wrapper treats it expired and returns without UnInit, leaving own Update to remove. |
| State at fatal return? | **RESOLVED:** Health zero, alive, nonlimbo, in Logic/map/occupancy, effects already complete. |
| Does own Update start at cleanup? | **RESOLVED:** no; ordinary Building prefix and AI run first. |
| Exact cleanup callback order? | **RESOLVED:** Limbo, second SpawnSurvivors, UnInit, Place_OccupyMap, return. |
| Does second SpawnSurvivors see cargo? | **RESOLVED:** no; generic fatal Explodes gate emptied it before the first call. |
| Is second crew budget the first remainder? | **RESOLVED:** no; fresh local recomputation. |
| What is `+0x6E0` and can it differ between calls? | **RESOLVED:** constructor-false survivor-suppression latch set from nonzero fatal `ignore_defenses` before call one; it remains true through call two. |
| What is `+0x540` in the survivor denominator/postlude? | **RESOLVED:** swizzled/checksummed retained C4/PostMortem source object; normally null, planter when C4 is armed, cleared on exact pointer expiry/IC/consumed hut. |
| What is `+0x6E3` and who writes it? | **RESOLVED:** sticky successful Building owner-change latch; constructor false, `ChangeOwner` is the only runtime true writer, no clear; persisted and directly checksummed. |
| Does the 15% Technician predicate remain anonymous? | **RESOLVED:** no; Building vtable `+0x2AC -> 0x00458DB0` is weapon-equipped (`IsOccupied OR base weapon-present`). Active `NANRCT` is false. |
| Does the `Nominal` survivor bookkeeping change active Explodes output? | **RESOLVED/excluded:** compiled `+0x6E9 && Type.Nominal` sets survivor no-House-count `+0x6D9`, but active Explodes crew cannot reach a Nominal type. |
| Are foundation smudges unconditional? | **RESOLVED:** no; initial nonzero crew budget gates the entire foundation walk. |
| Is the second pass behaviorally identical? | **RESOLVED:** no; first is pre-Limbo, second post-Limbo, so passability/placement can differ. |
| How does self-removal affect the Logic successor? | **RESOLVED:** stable erase plus outer increment skips the shifted successor that pass. |
| Does the retained window survive save/load/hash? | **RESOLVED:** raw native block persists; native CRC adds normalized remaining zero; Rust must persist/hash its future-equivalent phase. |
| Is there an equivalent hidden Rust path? | **RESOLVED:** no; active Rust immediately UnInits and has no pending own-Update cleanup. |

All questions are closed. There is no residual load-bearing uncertainty.

## 20. Adversarial contradiction checks

1. **Stale field-name attack:** searched all `0xD15` operands, not just known functions. No radiation/reactor writer or reader exists.
2. **Mission-label attack:** derived index 19 from the native mission pointer table and cross-checked Rust raw enum, rather than trusting a decompiler label.
3. **Burst-table attack:** raw-read the 16 bytes and separately verified the runtime direction initializer and coordinate helper.
4. **Random Anim attack:** decompiled `AnimTypeClass::FindByIndex`; it returns a deterministic registry index and consumes no RNG.
5. **Owner attack:** inspected the post-constructor instruction interval; no owner setter occurs. Constructor initializes null.
6. **Hidden overlay reach attack:** scanned all canonical OverlayType sections and all map-local type sections, not only observed OverlayPack cells. The eligible set is empty by type authority.
7. **Timer inversion attack:** inspected assembly in both fatal ReceiveDamage and own Update. Duration 8 triggers immediate UnInit in the fatal wrapper; duration 0 skips it and is removed by own Update.
8. **Second-pass cargo attack:** traced generic fatal cargo destruction before Building DestructionEffects, then both survivor call sites. Cargo is empty at both.
9. **Smudge duplication attack:** decompiled the outer `if budget != 0`; crewless Buildings skip the whole pass, contrary to current Rust.
10. **Pass equivalence attack:** verified first call precedes Limbo and second follows it; cell passability and Infantry unlimbo can differ.
11. **Scheduler snapshot attack:** inspected the live count re-read after every callback and stable vector erase. The successor skip is native, not an inferred queue behavior.
12. **Persistence attack:** inspected raw base Save/Load and the derived Building checksum normalization; pending state is save-visible even though duration hashes as zero.
13. **Anonymous-flag attack:** repeated full-program operand censuses for `+0x540`, `+0x6E0`, and `+0x6E3`; this closed the C4 source, ignore-defenses suppression, and sticky owner-change semantics instead of treating them as arbitrary survivor modifiers.
14. **Active-owner attack:** classified all 22 preplaced `NANRCT` owners and their Country sides. Fifteen Neutral instances have zero budget; only five Soviet and two Third-side instances enter the crew walk before later capture/defeat/damage-state gates.
15. **Technician/Nominal attack:** resolved Building vtable `+0x2AC`, the `+0x6E9` writer, `Nominal +0xC9E`, and survivor `+0x6D9`; active `NANRCT` cannot reach the compiled nominal-count branch.

No contradiction survived.

## 21. Coverage ledger

| Required surface | Evidence | Result |
|---|---|---|
| field identity/default/parser/current-default layering | constructor, ReadINI, string xref, Rust parser/merge | closed |
| complete `+0xD15` consumer inventory | full-program operand scan, 8/8 | closed |
| burst offsets/order/coordinates/Z | raw table, direction init, `FUN_0049F550` | closed |
| cell conversion/dummy/overlay gate | `MapClass::Get_CellClass_At_Coord`, caller assembly | closed |
| OverlayType `+0x2B0` identity/default/parser | ctor/ReadINI/census | closed |
| allocation and RNG gate/order | burst assembly | closed |
| FIRE3 lookup/constructor/layer/owner | helper, ctor call row, active art | closed |
| burst downstream RNG | active art plus nonzero delay | closed |
| active overlay types/cells | canonical + 184-map census | closed: zero |
| generic fatal cargo and death-weapon order | `TechnoClass::ReceiveDamage`, cargo helpers | closed |
| timer writes and OR precedence | DestructionEffects assembly | closed |
| fatal return lifecycle/map state | Building ReceiveDamage plus absence of removal calls | closed |
| own Update prefix and timer predicate | Building Update/UpdateAnimation | closed |
| cleanup vtable resolutions/order | Building vtable/functions/assembly | closed |
| first/second SpawnSurvivors cargo/crew/smudge | both call sites and helper callees | closed |
| `+0x540/+0x6E0/+0x6E3` identities/writers/readers | full operand censuses, C4/ChangeOwner/CRC call sites | closed |
| survivor source mission/target postlude | SpawnSurvivors assembly plus mission table | closed |
| weapon-equipped/Technician/Nominal conditional | vtable `+0x2AC`, type/instance flags, active census | closed; active Nominal result excluded |
| all survivor RNG relevant to repeat | chance/type/health/direction/smudge-selection helpers | closed |
| live Logic timing and compact mutation | PerTickUpdate + UnregisterObject | closed |
| before/after turn, pause | scheduler control flow | closed |
| save/load/checksum | Building/Base persistence, `+0x540` swizzle, CRC | closed |
| active BuildingTypes/map instances/owner sides | 403 types, 184 maps, 11,992 rows; 22 `NANRCT` owner census | closed |
| current Rust direct match/disparity | HEAD `d6f9aeca` source scan | closed |
| evidence-backed exclusions | binary/data/Rust boundary table | closed |
| exact Rust acceptance/schema/hash | handoff above | closed |

## 22. Cold spot checks and zero-add pass

The final cold pass repeated the following independently of the first synthesis:

- live Ghidra connection and active `/gamemd.exe` target;
- full `0xD15` instruction census: still exactly eight, scan not truncated;
- full `+0x540`, `+0x6E0`, and `+0x6E3` operand censuses and their only real Building writers/clearers/readers;
- raw `0x00818CB8` table: still `{0,2,4,6}`;
- decompile/assembly of the burst, timer writes, fatal-return predicate, own-Update cleanup, generic fatal cargo loop, SpawnSurvivors, crew helpers, source postlude, smudge helpers, ChangeOwner, live Logic loop, stable erase, Limbo, UnInit, Place_OccupyMap, parser, lookup, Save/Load, and checksum;
- canonical/map census totals, every authored `Explodes=` occurrence, all 22 `NANRCT` owners/Country sides, and relevant map-local `NANRCT` overrides;
- Rust parser, survivor-divisor keys, retained C4 source, owner-change chokepoint, death-weapon gate, fatal hooks, Building smudge plan, crew ejection, lifecycle, snapshot version, and hash inputs.

The zero-add pass found no additional active `Explodes` consumer, no active overlay-burst reach, no third survivor invocation, no second death-weapon/destruction-effects invocation, no hidden SetOwnerObject, no alternate cleanup callback order, no additional map override, and no active route from Explodes crew selection into the compiled Nominal bookkeeping branch. The slice is ready for a builder/critic loop without further reverse-engineering prerequisites.
