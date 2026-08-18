# Building Damage Receiver and Destruction Lifecycle Reinvestigation

Date: 2026-07-13  
Mode: `/re-investigate` exhaustive-slice  
Status: **PARTIAL - receiver/destruction/lifecycle control flow closed; bounded helper semantics remain UNKNOWN**  
Parity target: active Yuri's Revenge `gamemd.exe`

## Verdict

`BuildingClass::ReceiveDamage @ 0x00442230` and `BuildingClass::DestructionEffects @ 0x004415F0` are now re-identified from raw RTTI/vtable bytes and traced through their active destruction lifecycle. The important lifecycle result is counterintuitive but binary-verified:

- an ordinary lethal building hit sets destruction duration `8` and calls virtual `+0xF8` (`ObjectClass::UnInit`) synchronously inside `ReceiveDamage`;
- a building whose current mission is `0x13` (`Selling`) or whose type has `Explodes=yes` gets duration `0`, remains alive/registered at the return from `ReceiveDamage`, and removes itself in its own later `BuildingClass::Update`;
- that deferred path calls `SpawnSurvivors` twice: once in `DestructionEffects` and again during the removing update. The cargo list is emptied by the first call, but the crew budget and per-foundation smudge loop are local and therefore run again.

The stale claim that Building vtable `+0x4EC` is `Limbo` is false. `+0x4EC` is `DestructionEffects @ 0x004415F0`; `+0xD4` is `BuildingClass::Limbo @ 0x00445880`; `+0xF8` is `ObjectClass::UnInit @ 0x005F65F0`.

This report remains **PARTIAL** only because the semantic role and downstream effects of called helper `0x0048DED0` are still `UNKNOWN`, and generic `Scatter`, `AnimClass`, `ParticleSystemClass`, and smudge-constructor internals were intentionally not reopened. The `CausesDelayKill` formula and result-5 production are the Task 2C boundary and are recorded here without re-decoding.

## Target, non-goals, evidence bar, and stop conditions

**Target question:** What exact prechecks, result bands, ordered destruction effects, RNG consumption, passenger/garrison behavior, destruction delay, lifecycle owner, and same-tick scheduler consequences apply when an active YR building receives damage?

**Non-goals:** re-derive the generic Object/Techno damage arithmetic; decode `CausesDelayKill`; identify every generic animation/particle/scatter helper side effect; implement Rust; mutate Ghidra; run Cargo; audit sell behavior except where it is directly reused by lethal building damage.

**Evidence required for a closed assigned slice:** raw identity proof for both target functions; exact forwarding signature; every result branch and building-specific precheck; ordered fatal and destruction-effect ledgers; allocation-gated RNG order; immediate versus deferred removal proof; active-vector mutation consequence; Rust handoff.

**Stop conditions:** stop at generic helper internals, Task 2C formula ownership, and unrelated death paths. Preserve unresolved helper roles as `UNKNOWN` rather than infer names from stale labels.

## Preflight and evidence discipline

- The research-index brief for system `damage`, query `Building ReceiveDamage destruction lifecycle`, and anchors `0x00442230` / `0x004415F0` validated successfully before binary work.
- The output path did not exist at preflight. No same-file conflict was present.
- Static read-only Ghidra was connected to project `testProsjekt-12.1.2-test`, program `/gamemd.exe`, retail path `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, image base `0x00400000`, x86 little-endian 32-bit.
- No debugger, screen/UI automation, Ghidra rename/comment/save operation, Cargo command, Rust edit, or external mod source was used.
- Local Ghidra labels were treated as hints only. Load-bearing identities below come from RTTI bytes, vtable bytes, function bodies, and call/argument flow.

### Fresh binary calls used in this pass

- `get_current_program_info(program="/gamemd.exe")`.
- `get_function_by_address`, `decompile_function`, and `disassemble_function`: `0x00442230`, `0x004415F0`, `0x0043FB20`, `0x00445880`, `0x005F65F0`, `0x0055AFB0`, `0x0055BAE0`.
- `read_memory`: `0x007E3EB8`, `0x007FC360`, `0x00818D60`, `0x007E4028`, `0x007E43A8`, `0x007E3F90`, `0x007E3FB4`, `0x00442C18`.

Each binary-derived statement below cites the actual fresh call or inspected instruction range inline.

## 1. Raw identity and dispatch proof

The BuildingClass primary vtable begins at `0x007E3EBC`. Its Complete Object Locator pointer is the preceding dword at `0x007E3EB8`. The COL points through `0x007FC360+0x0C` to TypeDescriptor `0x00818D60`, whose name bytes are `.?AVBuildingClass@@`. This is direct raw RTTI evidence, not a Ghidra label assumption (`read_memory(0x007E3EB8,1268)`, `read_memory(0x007FC360,20)`, `read_memory(0x00818D60,40)`).

| Building vtable slot | Raw bytes | Target | Verified role |
|---|---:|---:|---|
| `+0x16C` | `30 22 44 00` | `0x00442230` | `BuildingClass::ReceiveDamage` |
| `+0x4EC` | `F0 15 44 00` | `0x004415F0` | `BuildingClass::DestructionEffects` |
| `+0xD4` | `80 58 44 00` | `0x00445880` | `BuildingClass::Limbo` |
| `+0xF8` | `F0 65 5F 00` | `0x005F65F0` | `ObjectClass::UnInit` |

Evidence: `read_memory(0x007E4028,4)`, `read_memory(0x007E43A8,4)`, `read_memory(0x007E3F90,4)`, and `read_memory(0x007E3FB4,4)`, followed by fresh body decompile/disassembly at each target.

`BuildingClass::ReceiveDamage` ends with `RET 0x1C`, proving seven explicit 32-bit arguments. At `0x0044240E..0x00442425` it forwards all seven to `TechnoClass::ReceiveDamage @ 0x00701900` in this established order:

```text
(damage_ptr, distance, warhead, attacker,
 ignore_defenses, arg6, source_house)
```

On fatal result 4, `0x00442651..0x00442665` pushes `0`, `attacker`, `ignore_defenses`, and the saved foundation list before calling Building vtable `+0x4EC`. Thus the verified explicit `DestructionEffects` boundary is:

```text
DestructionEffects(0, attacker, ignore_defenses, saved_foundation)
```

Evidence: fresh `disassemble_function(0x00442230)` and `decompile_function(0x00442230)`. `DestructionEffects` has `RET 0x10`, consistent with four explicit arguments (`disassemble_function(0x004415F0)`).

## 2. ReceiveDamage ordered state machine

### 2.1 Entry prechecks and snapshots

The building wrapper performs this order before interpreting the generic receiver result:

1. If `attacker == this` and the attacker's type byte `+0xCA0` (`DamageSelf`) is false, return result `0` without entering the generic receiver. (`decompile_function(0x00442230)`, entry branch.)
2. Save the pre-hit health-ratio-derived state and current building frame used by later transition logic.
3. If attacker is non-null and victim virtual `+0x80` is false, write the current frame to owner `+0x54D8`, attacker type `+0x30` to owner `+0x54DC`, then call `0x00708080`. This happens before the generic receiver call. (`decompile_function(0x00442230)`.)
4. Snapshot the foundation list and the current contact vector (`Building+0xE8`) through `0x0065AD30`. These snapshots are then used by fatal cleanup, so later mutations cannot change that iteration set. (`decompile_function(0x00442230)`.)
5. If type `+0x16BF` (`LaserFence`) is true and `ignore_defenses == 0`, return `0`. This path exists in the binary but no stock `LaserFence=` assignment was found in the merged repo INIs, so it is dormant in standard stock YR data.
6. If type `+0x16B6` (`BridgeRepairHut`) and ObjectType `+0x233` (`Immune`) are both true, return `0`, independent of `ignore_defenses`. `CABHUT` actively has `BridgeRepairHut=yes` in stock (`ini/rulesmd.ini:16348`); the effective stock `Immune` value for this exact conjunction was not re-resolved in this slice.
7. If current health is nonzero, forward all seven arguments to `TechnoClass::ReceiveDamage @ 0x00701900`. If current health is already zero, skip that call but still enter the wrapper's post-result/alive logic. (`0x0044240E..0x00442425`.)

### 2.2 Result dispatch

The raw jump table at `0x00442C18` contains `0x004426AC`, `0x004426C8`, `0x004424A2`, `0x0044247D` for results 2, 3, 4, and 5 respectively (`read_memory(0x00442C18,16)`). Result 0/1 use the common postlude.

| Generic result | Building-specific action | Then |
|---:|---|---|
| `0` | No damage reaction branch. | Common alive/frame postlude. |
| `1` | No distinct building-only branch. | Common alive/frame postlude. |
| `2` | If `Building+0x30C` particle system exists, multiply its float at `+0xE8` by `1.5`; then fall through result 3. | Damage sound fallback, Sparky, common postlude. |
| `3` | If type `DamageSound` at `+0x538` is `-1`, play global `Rules+0x714` `BuildingDamageSound` at building coordinates. | Sparky, common postlude. |
| `4` | Execute the fatal chain in section 2.3. | Destruction effects, duration/removal branch, common postlude. |
| `5` | No building destruction chain. | Temporary-vector cleanup and common postlude. |

Result `5` is the wrapper boundary for the generic delayed-kill/PostMortem behavior; it must not be treated as an alias of result `4`. The `CausesDelayKill` predicate, `Building+0x6DF` interaction, and exact formula are intentionally owned by Task 2C and remain outside this report.

For results 2 and 3, the wrapper then runs the Sparky foundation loop described in section 3. `DamageSound == -1` is a global fallback condition, not a claim that every damaged building always uses the global sound (`decompile_function(0x00442230)`).

### 2.3 Fatal result-4 chain before DestructionEffects

The exact fatal wrapper order is:

1. Remove the linked object at `Building+0x2E4` from the saved contact snapshot, then call `UndockUnit @ 0x004593A0` for that link. This prevents processing the same link again in the contact loop.
2. If the capture-manager pointer at `+0x2BC` exists, call its `FreeAll` path.
3. If `Building+0x2AC` exists, call `0x0070FEE0(1)`.
4. Iterate the remaining saved contacts in snapshot order.
5. For each contact, calculate 3-D distance. If distance is `< 0x100` leptons **or** the contact type has `Helipad` at `+0x16CB`, damage it for `contact.Type.Strength * 10` with `Rules+0xFA8` as warhead, null attacker, `ignore_defenses=1`, `arg6=1`, and null source house.
6. Otherwise send radio message `0x17` through contact vtable `+0x278`, then clear contact field `+0x500`.
7. Free the temporary snapshot vector.
8. If type `+0x157B` (`CanBeOccupied`) is true, call `SellBuilding @ 0x00457DE0(0,0)`. This ejects the garrison vector in reverse/LIFO order. An occupant whose `Unlimbo` fails is uninitialized. The building garrison container at `+0x684` is cleared.
9. If light-source pointer `Building+0x614` exists, call `0x00554A80(0)` to tear it down.
10. Call virtual `+0x4EC`, now proven `DestructionEffects(0, attacker, ignore_defenses, saved_foundation)`.
11. Apply the duration/removal split in section 6.

Evidence: fresh `decompile_function(0x00442230)` plus `disassemble_function(0x00442230)`, especially `0x0044247D..0x004426AB` and the `+0x4EC` call at `0x00442651..0x00442665`.

The garrison vector handled by `CanBeOccupied -> SellBuilding` is distinct from the inherited Cargo/passenger container processed later by `SpawnSurvivors`. A Rust model may share storage internally only if it positively preserves those two native mechanisms, their different gates, and their ordering.

### 2.4 Common postlude

After the result-specific branch:

- if the native alive byte `Building+0x90` is false, the wrapper returns result `4` regardless of the local generic result;
- if attacker is non-null and result is nonzero, it runs under-attack notification gates, records attacker type at `Building+0x53C`, and runs retaliation logic; one conditional branch consumes `Random::Next` at `0x00442A73`;
- for any nonzero result, it recomputes the health threshold using `Rules+0x1700` with a `<=` comparison, updates cached byte `Building+0x6E6`, and switches existing building animation slots between damaged and normal type entries around `Type+0xF5C` / `Type+0xF4C`;
- if the building frame changed, it sets dirty byte `+0x80` and repeats the threshold/animation-state reconciliation;
- otherwise it returns the local result.

Evidence: fresh `decompile_function(0x00442230)` and corresponding late-body disassembly.

## 3. Sparky foundation animation RNG

The result-2/3 Sparky loop is gated by Warhead byte `+0x14A`. For every saved foundation cell it draws `Ranged(0, foundation_height + foundation_width + 5)` and branches as follows (`decompile_function(0x00442230)` and late-body disassembly):

| First draw | Allocation/selection behavior |
|---:|---|
| `1..5` | Allocate first. On successful allocation only, draw `Ranged(1,3)` and use `Rules+0xB78[0]`. |
| `6..8` | Allocate first. On successful allocation only, draw `Ranged(1,3)` and use `Rules+0xB78[1]`. |
| `9` | Allocate; no second ranged draw; use `Rules+0xB78[2]` and constructor parameter `1`. |
| other | Spawn nothing. |

Successful animations are constructed with flags `0x600` and attached to the building with `AnimClass::SetOwnerObject @ 0x00424B50`. Allocation failure suppresses the second draw in the `1..8` bands. That allocation-gated RNG behavior is part of the parity contract.

## 4. DestructionEffects exact ordered ledger

Fresh `decompile_function(0x004415F0)` and `disassemble_function(0x004415F0)` establish this order:

1. **Clear damage fires.** Iterate the eight pointers at `Building+0x5C8..+0x5E4`; call each non-null animation's virtual `+0xF8`, then null the slot.
2. **Auxiliary/radar cleanup.** Clean `Building+0x210`; when type byte `+0x16A4` is set, update the owner's radar-related mask/state.
3. **Special type recalculation.** Type byte `+0x16C7` gates another state/recalculation path. Its broader semantic name was not needed and is not guessed here.
4. **Laser-fence post disconnect.** Type byte `+0x16BE` (`LaserFencePost`) calls the wall/fence connection update with argument `1`.
5. **Reveal-to-all side effect.** Type byte `+0x5EE` (`RevealToAll`) and a non-human owner call virtual `+0x48C(0,0,1,gPlayer)`.
6. **Death-sound fallback.** If the type's `DieSound` list/count at `+0x520` is empty, play global `Rules+0x6E8` `BuildingDieSound` at the building coordinate.
7. **Center destruction smudge/debris.** For a foundation at least `2x2`, snap the building `Location` to the cell center. If width is `>2`, consume and discard `Ranged(0,width-2)`; if height is `>2`, consume and discard `Ranged(0,height-2)`. Then `Ranged(0,99) < 50` calls the burn/debris helper with `(100,1)`; otherwise the smoke/crater helper with `(100,1)`. The discarded width/height values do **not** move the coordinate.
8. **Per-foundation destruction animations.** For every saved foundation cell, use the type list at `+0x730` / count `+0x73C`. Coordinate scattering calls `0x0049F420(0x40,0)` first. Allocate an animation; only on success draw `Ranged(0,3)`, then consume `Random::Next % count` to select the list entry; construct with flags `0x600`.
9. **`Explodes` burst.** If type byte `+0xD15` (`Explodes`) is true, inspect four hardcoded offsets and their overlay eligibility byte `+0x2B0`. For every eligible offset, allocate first; only on success draw `Ranged(1,3)`, choose through helper `0x00427CB0`, and construct with flags `0x600`.
10. **Storage spill.** While total building storage at `+0x33C` is at least `1.0`, choose the first nonempty resource slot. Remove exactly `1.0` from the building storage and exactly `1.0` from the owner's aggregate mirror at `Owner+0x2FC`; these are two accounting writes for one resource unit, not two spilled units. Draw `Ranged(0x100,0x300)`, call `0x0049F420` for direction, resolve the cell, and call `PlaceTiberium(slot,1)`. Placement success is ignored. The loop leaves any fractional amount below `1.0` stored. Evidence includes assembly `0x00441B30..0x00441B7E`.
11. **Unknown helper.** If the type-cost / `Rules+0x5C8` quotient is positive, call `0x0048DED0`. Its semantic role and downstream effects remain **UNKNOWN**; no label is asserted.
12. **Set destruction timer.** Read current mission through virtual `+0x184`. If mission is `0x13` (`Selling`) **or** type `Explodes` is true, write current frame to `+0x528`, auxiliary timer state to `+0x52C`, duration `0` to `+0x530`, and reset the timer fields at `+0x100..+0x10C`. Otherwise perform the same writes but set duration `8`. Evidence: disassembly `0x00441C39..0x00441CAC`; Rust's mission table independently maps ID 19/`0x13` to `Selling` at `src/sim/mission/mod.rs:57,115`.
13. **Main death animation.** If the type list at `+0x758` is nonempty, consume `Random::Next` before entry selection, select modulo count, then allocate/construct with flags `0x600`. A null selected type skips allocation. If allocation fails while type field `+0xDF0` is nonzero, the native body appears to dereference the null result at `+0xD4`; the engine evidently assumes allocation success on that path. A custom name copy is bounded to `0x20` bytes.
14. **Destruction particle system.** Scan candidate types at `+0x798` / count `+0x7A4` in reverse order and retain candidates whose byte `+0x2B4` is zero. Only if `Building+0x320` is null, a candidate exists, and virtual `+0x1C8 > -10`, allocate first; on success draw `Ranged(0,count-1)`, construct at `Location + offset`, and store the pointer at `+0x320`.
15. **Commit dead state.** Write health `0`. If the third explicit argument (`ignore_defenses`) is nonzero, write `Building+0x6E0 = 1`. Within the audited slice, `+0x6E0` suppresses survivor/crew release; calling it an inherent “Iron Curtain killed” flag is over-specific. Evidence: disassembly `0x00441EFC..0x00441F12` reads the third stack argument.
16. **Passenger and survivor tail.** Call `SpawnSurvivors` with `Building+0x540`, then call `EMPPassengers(attacker)`. Evidence: disassembly `0x00441F12..0x00441F27`.

`Type+0xD15` is `Explodes`; it is not a reactor/radiation flag. `0x004415F0` is `DestructionEffects`; it is not Limbo. Both corrections materially change the lifecycle interpretation.

## 5. SpawnSurvivors, cargo, crew, and smudges

### 5.1 Survivor budget

`SpawnSurvivors` starts a random denominator at `2`; a non-null argument corresponding to `Building+0x540` changes it to `1`; `Building+0x6E3` adds `6`.

The crew budget comes from virtual `+0x2D0 -> 0x00451330`:

- return `0` if `Building+0x6E0` is set or type `+0xCCD` (`Crewed`) is false;
- select the owner-side divisor from `Rules+0x14F8`, `+0x14FC`, or `+0x1500`;
- double that divisor when `Building+0x6E3` is set;
- calculate the type-cost-derived count and clamp it to `1..5`.

Evidence: fresh receiver/destruction call trace plus decompile of the directly reached survivor helpers during this pass.

### 5.2 Cargo/passenger branch

If type `+0x16AE` (`UnitAbsorb`) or `+0x16AF` (`InfantryAbsorb`) is set and inherited Cargo `+0x114` is nonempty, loop the cargo passengers.

- If `Building+0x6E0 == 0`, attempt `Unlimbo`; on success update/clear passenger state, scatter it, and invoke its AI continuation.
- If `+0x6E0 != 0`, or if `Unlimbo` fails, call passenger virtual `+0xE0` then `+0xF8`.
- `Scatter` may consume further conditional RNG; its generic internals are outside this slice.

This cargo path is separate from the earlier `CanBeOccupied -> SellBuilding` garrison path.

### 5.3 Crew branch and exact RNG boundary

For each saved foundation cell, while the owner is not defeated and crew budget remains:

1. Draw `Ranged(0, denominator)`; spawn a crew candidate only when the result is exactly `1`.
2. Resolve survivor type through virtual `+0x30C -> 0x0044EB10`. When `+0x6E3 == 0`, this consumes `Ranged(0,99)`; a result below 25 with type `+0xEB8 == 7` selects `Rules+0xF70`, otherwise it calls `Crew_Type @ 0x00707D20`. `Crew_Type` may consume another `Ranged(0,99)` and selects `Rules+0xF6C` below 15.
3. Type resolution happens **before** infantry allocation, so those random draws remain consumed on allocation failure.
4. Allocate/construct infantry and attempt `Unlimbo`. Only a successful placement decrements the local budget and draws `Ranged(5, survivor_type.Strength)` for health; then apply scatter/mission/target state.
5. Independently of whether crew spawned, run the per-cell smudge branch after `CellClass::CheckCellPassability`: draw `Ranged(0,99)`, call `0x0049F420(0x80,0)` (one `Random::Next` direction draw), snap the coordinate, then choose/place the helper result.

Therefore survivor creation cannot be replaced by “always spawn one infantry.” Allocation, passability, side/type selection, budget, and placement success all affect both visible output and RNG position.

## 6. Destruction duration, lifecycle owner, and same-tick membership

### 6.1 Immediate ordinary lethal path

After `DestructionEffects` returns, `ReceiveDamage` compares the current timer state (`0x0044266B..0x004426A7`):

- ordinary non-Selling, `Explodes=no` destruction has duration `8` and elapsed `0`, so remaining duration is positive;
- that positive branch calls virtual `+0xF8` synchronously, which resolves to `ObjectClass::UnInit @ 0x005F65F0`;
- it then calls `0x00441F60` for foundation/cell/tactical refresh work and reaches the common postlude.

This is the normal immediate building-removal path. The duration value `8` does **not** mean “stay alive for eight ticks” in this branch.

### 6.2 Deferred Selling/Explodes path

For mission `Selling` or `Explodes=yes`, duration is `0`. At the same comparison, elapsed is already `>= duration`, so `ReceiveDamage` skips `UnInit`. Health is zero, but alive byte `+0x90`, limbo state, and LogicClass membership remain unchanged at return.

Fresh `decompile_function(0x0043FB20)` proves the later owner:

1. The dead building reaches its own `BuildingClass::Update` through LogicClass.
2. The zero-health branch clears all eight damage-fire slots again.
3. Its `+0x528/+0x530` timer is expired immediately.
4. It calls virtual `+0xD4 -> BuildingClass::Limbo @ 0x00445880`.
5. It calls `SpawnSurvivors` again.
6. It calls virtual `+0xF8 -> ObjectClass::UnInit`.
7. It calls `0x00441F60` and returns.

If the building's scheduler position is later than the damaging object's position, removal can occur later in the same tick. If it already ran, it remains until the next tick.

### 6.3 Double SpawnSurvivors is real

The deferred path calls `SpawnSurvivors` once at the end of `DestructionEffects` and once from `BuildingClass::Update`. `GetSurvivorCount @ 0x00451330` computes a fresh local budget and does not consume a persistent “already spawned” field. `BuildingClass::Limbo` does not set `+0x6E0` or `+0x6E3`.

Consequences:

- inherited cargo is emptied by the first call, so it does not eject twice;
- crew chance/budget runs again and can create additional crew;
- the per-foundation passability/smudge loop runs again even when crew is suppressed;
- if `+0x6E0` is set, crew is suppressed in both calls, but the per-cell smudge work still repeats.

This is an exact-mechanism finding, not an endorsement of the apparent duplication.

### 6.4 UnInit, Limbo, and active-vector mutation

Fresh `decompile_function(0x005F65F0)` proves `ObjectClass::UnInit` order: defuse; for Foot objects call `EMPPassengers(0)`; detach from lists; call virtual `+0xD4`; clear alive byte `+0x90`; append to the pending-delete list at `0x00B0F69C`. Physical storage deletion is deferred.

`BuildingClass::Limbo @ 0x00445880` reaches `ObjectClass::Conceal @ 0x005F4D30`, which calls active-vector remover `0x0055BAE0`, and sets limbo byte `+0x81` near the end. Fresh `decompile_function(0x0055BAE0)` shows order-preserving left compaction and synchronous membership-byte clearing.

Fresh `decompile_function(0x0055AFB0)` shows LogicClass iterating forward over the live vector, re-reading its current count, and invoking each element's virtual update at `+0x5C`. Therefore:

- a target removed before its scheduled position does not update that tick;
- removing an element at or before the current loop index shifts successors left, while the loop still increments, so the shifted successor can be skipped;
- a deferred building removes itself from its own update, so its immediately shifted successor is skipped in that LogicClass pass;
- an ordinary immediate building removal occurs inside the damaging object's receiver call, and its exact same-tick effect depends on the target's index relative to the current updater.

No Rust implementation can batch all destroyed buildings at a later phase and claim exact scheduler parity without reproducing these list mutations and their same-tick visibility.

## 7. RNG and allocation ledger

| Site | Native draw order | Allocation-gated? |
|---|---|---|
| Sparky result 2/3 | Per cell first range; bands 1..8 allocate, then second `Ranged(1,3)` only on success; band 9 has no second draw. | Yes |
| Center destruction smudge | Width draw only if `W>2`; height draw only if `H>2`; then `Ranged(0,99)`. Width/height values discarded. | No allocation at this level |
| Per-cell destruction anim | Direction `Random::Next`; allocate; on success `Ranged(0,3)` then selection `Random::Next % count`. | Yes after direction |
| `Explodes` four-offset burst | Per eligible offset allocate; on success `Ranged(1,3)` then helper selection. | Yes |
| Storage spill | Per whole unit `Ranged(0x100,0x300)`, then direction `Random::Next`; `PlaceTiberium` can have its own placement-gated draw. | Placement does not restore draws |
| Main death anim | `Random::Next` before list selection and allocation. | Selection draw is not gated |
| Particle system | Allocate first; on success `Ranged(0,count-1)`. | Yes |
| Crew survivor | Chance draw; type-selection draw(s) before allocation; health draw only after successful `Unlimbo`. | Partly |
| Per-cell survivor smudge | Passability gate; `Ranged(0,99)`; direction `Random::Next`; helper placement. | Passability-gated |

Generic `Scatter`, animation constructors, particle constructors, and smudge helper internals may consume additional draws after the stated call boundaries. They remain bounded `UNCHECKED` here.

## 8. Active YR reachability

| Mechanism | Stock activity verdict | Evidence |
|---|---|---|
| Ordinary duration-8 lethal path | **ACTIVE** | Most stock buildings omit/disable `Explodes`; normal combat destruction reaches it. |
| `Explodes` duration-0 deferred path | **ACTIVE** | Stock buildings with `Explodes=yes` include GAYARD, NAYARD, YAYARD, CAOILD, NANRCT, YAPPPT, CAMISC01, CAMISC02, and AMMOCRAT (`ini/rulesmd.ini` merged assignments; examples `11867`, `12655`, `12761`, `13405`, `13947`, `14892`, `14946`, `14968`, `22297`). |
| Selling duration-0 branch | **ACTIVE** | Mission ID `0x13` is active `Selling` (`src/sim/mission/mod.rs:57,115`). |
| CanBeOccupied garrison ejection | **ACTIVE** | Many stock civilian/bunker types have `CanBeOccupied=yes`, including YAPPPT and NABNKR. |
| Crewed survivor branch | **ACTIVE** | Many stock production/power/tech buildings have `Crewed=yes`. |
| Cargo absorb branch | **ACTIVE where configured** | Stock YAPOWR has `InfantryAbsorb=yes` (`ini/rulesmd.ini:13156`). |
| BridgeRepairHut gate | **ACTIVE type path** | CABHUT has `BridgeRepairHut=yes` (`ini/rulesmd.ini:16348`); exact default `Immune` conjunction remains unchecked here. |
| LaserFence precheck/post path | **DORMANT in stock data** | No stock `LaserFence=yes` assignment was found in repo INIs; retained legacy code must not be confused with active low-bridge movement. |

## 9. Corrections and stale-claim audit

| Stale/misleading claim | Correct replacement | Evidence |
|---|---|---|
| Building vtable `+0x4EC` is Limbo. | `+0x4EC = DestructionEffects @ 0x004415F0`; `+0xD4 = Limbo @ 0x00445880`. | Raw vtable reads at `0x007E43A8` / `0x007E3F90`, both bodies decompiled. |
| `Type+0xD15` is a reactor/radiation switch. | `+0xD15 = Explodes`; it gates the four-offset burst and duration-0 lifecycle. | Fresh `0x004415F0` body plus stock `Explodes=` liveness. |
| Mission `0x13` here is Repair/unload. | Building destruction timer reads current mission `0x13 = Selling`. | `0x00441C39..0x00441CAC`; Rust mission table and audited mission research. |
| `Building+0x6E0` inherently means “killed by Iron Curtain.” | In this path it is written directly from nonzero `ignore_defenses`, then suppresses survivor/crew release. Broader semantic identity is not proven here. | `0x00441EFC..0x00441F12`. |
| Center-smudge width/height draws select an interior coordinate. | The conditional draws are discarded; the coordinate is the snapped building Location. | Fresh `0x004415F0` decompile and assembly. |
| Storage performs two one-unit spills per loop. | One unit is removed from building storage and mirrored in owner aggregate accounting; one placement attempt follows. | `0x00441B30..0x00441B7E`. |
| Duration `8` means deferred removal; duration `0` means immediate. | The branch is the opposite at ReceiveDamage return: duration 8 calls UnInit immediately; duration 0 skips it and defers to own Update. | `0x0044266B..0x004426A7` plus `0x0043FB20`. |
| Deferred destruction spawns survivors once. | It calls `SpawnSurvivors` twice; cargo empties once, but crew/smudge local work repeats. | Tail of `0x004415F0` plus zero-health branch of `0x0043FB20`. |
| `0x00441F60` can safely be named solely as Place_OccupyMap. | It performs foundation/cell/tactical refresh work; a narrower semantic label is not established in this slice. | Callsites/body context only; name intentionally bounded. |
| Result 5 is just fatal result 4 with a delay. | Result 5 bypasses the building destruction chain in this wrapper; its production/formula is the Task 2C boundary. | Jump table and branches in `0x00442230`. |

Documents containing these older phrasings include `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md` and the 2026-05-28 lifecycle swarm summary. This report is the correction fact bundle; those files were not edited because this task authorizes one output only.

## 10. Current Rust disparities

The Rust tree was read as it existed on 2026-07-13. Other sessions had unrelated dirty changes; none were modified here.

| Severity | Current Rust surface | Exact disparity |
|---|---|---|
| **CRITICAL** | `src/sim/combat/mod.rs:824-1031`, `src/sim/world/mod.rs:2399-2424` | Deaths are collected/batched, every dead object is unregistered in a later world phase, and nonanimated objects are uninitialized there. Native building removal occurs inside the receiver for ordinary deaths or inside the target's own Logic update for Selling/Explodes, with live compacting-vector consequences. |
| **CRITICAL** | `src/sim/world/mod.rs:2479-2494` | Rust uninitializes ordinary structures before later crew/garrison ejection. Native fatal wrapper ejects CanBeOccupied garrison before DestructionEffects/UnInit, and DestructionEffects runs Cargo/crew before the immediate UnInit; deferred paths run SpawnSurvivors twice. |
| **HIGH** | `src/sim/production/production_sell.rs:193-240` | Destruction survivors are hardcoded to one side-dependent infantry at deterministic perimeter positions. Native uses budget/divisor, per-foundation chance, type-selection RNG, allocation/Unlimbo gates, post-placement health RNG, and can run twice. The comment “always eject at least one” is false. |
| **HIGH** | `src/sim/combat/mod.rs:905-956` | One shared passenger vector is branched into CanBeOccupied garrison or transport death. Native has distinct CanBeOccupied garrison/SellBuilding storage and inherited Cargo with UnitAbsorb/InfantryAbsorb gates. |
| **HIGH** | `src/sim/combat/mod.rs:969-993` | Rust emits a rectangular `width * height` survivor-smudge request set. Native uses the exact saved foundation list and repeats it only on the deferred lifecycle. |
| **HIGH** | `src/sim/combat/smudge_dispatch.rs:222-281` | Rust always consumes two dimension draws for every `>=2x2` building. Native consumes width draw only for `W>2` and height draw only for `H>2`; Rust also centers at `(rx,ry)+128`, while native uses snapped building Location. |
| **HIGH** | `src/sim/combat/smudge_dispatch.rs:284-345` | `PathGrid::is_walkable`, offset/snap math, and helper selection are not proven equivalent to native `CellClass::CheckCellPassability` plus `0x0049F420(0x80,0)`. |
| **HIGH** | `src/sim/combat/mod.rs:883-886` | Rust plays only per-type `die_sound`; native has a global `BuildingDieSound` fallback when the type list is empty. No equivalent global `BuildingDamageSound` fallback was located in the current receiver path. |
| **HIGH** | `src/sim/combat/mod.rs:998-1031` | `has_animation` decides dying animation versus immediate uninit. Native building lifecycle instead uses mission `Selling` and type `Explodes` for the duration/removal split. |
| **HIGH** | `src/rules/object_type.rs:321,1001` | `Explodes` parses, but the audited combat destruction orchestration does not use it to select native duration-0 own-Update removal or four-offset destruction bursts. |
| **HIGH** | `src/app_building_anim.rs:74-200` | Damage fires are driven app-side by BTree/entity scan, `f32` health ratio, and a separate `anim_rng`; native owns them in Building update/receiver/DestructionEffects under Logic order and scenario RNG semantics. This also cannot exist in headless sim parity. |
| **MEDIUM/HIGH** | `src/app_building_anim.rs:291-299` | Threshold selection still has an unresolved raw `+0x157B` TODO and always falls back to `ConditionYellow`. |
| **HIGH** | `src/sim/combat/mod.rs:824-1031` | No equivalent fatal contact snapshot/range-or-Helipad damage/radio-clear loop was found around building death. |
| **HIGH** | audited death pipeline | No equivalent ordered light teardown, storage/owner-mirror spill, `Explodes` burst, main type death animation, reverse candidate particle selection, or unknown `0x0048DED0` call boundary was located. |
| **MEDIUM** | `src/sim/world/mod.rs:2534-2587` | Destruction smudges are skipped when optional grids are unbound and requests are still cleared. Native destruction effects are not optional based on app binding. |
| **MATCHING PRIMITIVE, WRONG ORCHESTRATION** | `src/sim/world/mod.rs:1270-1329` | Rust centralizes `uninit` and end-of-tick pending deletion, which can support native ownership. The caller phase/order and live Logic-vector consequences are not native. |

Parser coverage already present includes `Crewed`, per-type `DieSound`, `Explodes`, `CanBeOccupied`, `BridgeRepairHut`, and `DestroyParticleSystems` (`src/rules/object_type.rs:254,267,321,559,574,795`). Having the key parsed does not prove the destruction mechanism consumes it correctly.

## 11. Rust implementation handoff

This is a research handoff, not authorization to implement.

1. Model a building-specific receiver continuation that can run fatal contact/garrison/light/destruction effects synchronously at the generic result-4 boundary.
2. Preserve result 5 as a separate Task 2C outcome; do not route it into result-4 destruction.
3. Represent the destruction timer fields and split lifecycle exactly: ordinary duration 8 -> immediate `uninit`; Selling/Explodes duration 0 -> remain registered until own building update.
4. Make LogicScheduler removal compact in native order and expose same-tick mutation. Entity storage may remain Rust-owned and physically delete later.
5. Separate CanBeOccupied garrison from Cargo/passenger behavior at the mechanism boundary, even if a shared storage component is retained internally.
6. Implement one ordered `DestructionEffects` transaction with a single scenario RNG cursor and allocation-gated draws. Do not emit independent post-combat queues that reorder survivor, smudge, animation, particle, storage, and lifecycle effects.
7. Preserve the verified double `SpawnSurvivors` on deferred destruction unless new active-binary evidence proves an upstream gate not observed here.
8. Keep `0x0048DED0` as an explicit unresolved hook/blocker. Do not omit it silently and do not assign a speculative name.

### Required acceptance scenarios

| Scenario | Required observation |
|---|---|
| Ordinary `Explodes=no`, non-Selling building killed before its Logic turn | DestructionEffects executes in receiver; virtual UnInit unregisters immediately; target does not later update; physical storage free remains deferred. |
| Ordinary building killed after its Logic turn | Immediate unregister still occurs; compacting-vector index effect on the current updater's successor matches native. |
| `Explodes=yes` building killed before its Logic turn | ReceiveDamage returns with zero health and live membership; own later Update removes it; SpawnSurvivors/smudge local loop runs twice. |
| `Explodes=yes` building killed after its Logic turn | It remains registered until next tick; then own Update removes it and can skip the shifted successor. |
| Selling building killed | Same duration-0 deferred lifecycle as Explodes regardless of Explodes flag. |
| CanBeOccupied building with multiple occupants | Occupants exit LIFO through SellBuilding before DestructionEffects; failed Unlimbo occupant is uninitialized; Cargo branch remains distinct. |
| Crewed building with allocation failure | Type-selection draws remain consumed; health draw and budget decrement do not occur without successful Unlimbo. |
| Foundation `2x2`, `3x2`, `2x3`, `3x3` | Center-smudge dimension draw counts are respectively `0`, `1`, `1`, `2` before the 50/50 draw. |
| Storage `2.75` units | Exactly two placement attempts; building and owner aggregate each decrement twice by `1.0`; `0.75` remains. |
| Missing per-type damage/death sound | Global BuildingDamageSound/BuildingDieSound fallback fires at the verified result/destruction stage. |

## 12. Coverage matrix

| Requested surface | Coverage | Result |
|---|---|---|
| Raw function identity | RTTI + vtable bytes + body | **VERIFIED** |
| Seven-argument forwarding | RET size + push flow | **VERIFIED** |
| Building-only prechecks | Entry/body branches | **VERIFIED** |
| Results 0..5 | Raw jump table + branches | **VERIFIED**, result-5 production delegated to Task 2C |
| Fatal pre-DestructionEffects order | Decompile + disassembly | **VERIFIED** |
| DestructionEffects ordered ledger | Full body, call order, key assembly ranges | **VERIFIED** except helper `0x0048DED0` semantics |
| RNG/allocation gates | Caller-level draws and allocation branches | **VERIFIED at caller boundary**; generic helper internals `UNCHECKED` |
| Garrison vs Cargo | Both call paths | **VERIFIED** |
| Crew budget/type/health/smudge boundary | Direct helpers/calls | **VERIFIED** |
| Immediate/deferred lifecycle | ReceiveDamage + Update + Limbo + UnInit | **VERIFIED** |
| Same-tick membership | Logic loop + compacting remover | **VERIFIED** |
| Stock activity | Repo INIs + mission enum | **VERIFIED for listed keys**; CABHUT `Immune` conjunction `UNCHECKED` |
| Rust disparity handoff | Direct source reads | **VERIFIED against current tree**, not a gamemd parity test |

## 13. Negative facts / do not do

- Do not use the current Ghidra label alone to identify a BuildingClass slot; the raw RTTI/vtable/body proof is authoritative.
- Do not call `+0x4EC` Limbo. It is DestructionEffects. Limbo is `+0xD4`.
- Do not treat result 5 as ordinary fatal destruction.
- Do not treat duration 8 as an eight-tick corpse delay.
- Do not make all building deaths immediate: Selling and Explodes remain live until own Update.
- Do not collapse CanBeOccupied garrison and Cargo absorb passengers into one behavior branch.
- Do not hardcode one survivor or guarantee a survivor from every Crewed building.
- Do not consume both center dimension draws for every `>=2x2` foundation.
- Do not use the discarded center draws as coordinate offsets.
- Do not decrement storage twice per spawned ore unit; the second decrement is the owner aggregate mirror.
- Do not rename `+0x6E0` as an intrinsic Iron Curtain fact from this evidence alone.
- Do not silently omit the second deferred `SpawnSurvivors` call because it looks accidental.
- Do not call Rust-vs-Rust deterministic tests parity certification; acceptance needs active-gamemd-derived traces or exhaustive proof.

## 14. Remaining uncertainty

1. **`0x0048DED0`: UNKNOWN.** Its call predicate is proven, but its semantic role, state writes, audio/visual output, and RNG behavior are not. This is the only unresolved call inside the central DestructionEffects ledger.
2. **Generic helper internals: UNCHECKED.** `Scatter`, `AnimClass` construction/AI, `ParticleSystemClass` construction/AI, `PlaceTiberium`, and smudge helper internals may add downstream effects or RNG draws beyond the caller boundaries recorded here.
3. **Task 2C boundary: intentionally excluded.** The exact `CausesDelayKill` / PostMortem formula and `Building+0x6DF` writer/reader closure are not decoded here.
4. **CABHUT immunity conjunction: UNCHECKED.** `BridgeRepairHut=yes` is active stock data; this pass did not resolve CABHUT's effective merged ObjectType `Immune` value at runtime.
5. **Allocation-failure crash-looking path:** main death-animation allocation failure with nonzero `Type+0xDF0` appears to dereference null. No live fault injection was used; document the assembly behavior, not a claim about retail allocator failure frequency.

These bounded unknowns prevent a whole-function `COMPLETE` label, but they do not weaken the verified receiver result map, destruction ordering around the unknown call, immediate/deferred lifecycle split, double-survivor call, or same-tick membership findings.

## Sources

- Fresh read-only Ghidra calls listed in Preflight, especially `decompile_function` / `disassemble_function` at `0x00442230`, `0x004415F0`, `0x0043FB20`, `0x00445880`, `0x005F65F0`, `0x0055AFB0`, and `0x0055BAE0`.
- Raw identity reads: `0x007E3EB8`, `0x007FC360`, `0x00818D60`, `0x007E4028`, `0x007E43A8`, `0x007E3F90`, `0x007E3FB4`, and jump table `0x00442C18`.
- Existing binary-audited context: `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`, `BUILDING_DAMAGEFIRE_SLOT_CLEAR_DESTROY_LIFECYCLE_GHIDRA_REPORT.md`, `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`, and the current damage receiver reinvestigation set.
- Stock authority: `ini/rules.ini`, `ini/rulesmd.ini`; YR `rulesmd.ini` overlay takes priority.
- Rust source reads: `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/sim/production/production_sell.rs`, `src/sim/combat/smudge_dispatch.rs`, `src/app_building_anim.rs`, `src/rules/object_type.rs`, `src/sim/mission/mod.rs`.

## Status

**PARTIAL.** The assigned Building receiver, DestructionEffects order, duration/removal ownership, double-survivor behavior, and LogicClass same-tick consequences are verified. `0x0048DED0` and explicitly bounded generic helper internals remain unresolved; Task 2C owns the delayed-kill formula.
