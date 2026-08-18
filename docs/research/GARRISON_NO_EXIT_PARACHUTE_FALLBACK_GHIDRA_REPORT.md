# Garrison No-Exit Parachute Fallback -- Ghidra Research Report

**Address(es):** `0x00457DE0` (`BuildingClass::SellBuilding`), `0x004585C0` (`SpawnUnitsWithParachute`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the `CanBeOccupied` garrison branch where `SellBuilding` cannot find any usable ground exit coordinate and calls `SpawnUnitsWithParachute(0)`.
**Non-Scope:** normal successful edge ejection, infantry scatter internals after successful edge ejection, generic transport unload, ordinary aircraft paradrop payloads, tank bunker exit.
**Confidence:** High for the null-argument branch, occupant order, no-parachute/no-RNG/no-mission result, and Rust-facing delta.
**Active in YR:** Yes. The branch is reached from live YR sell/destruction callers when a `CanBeOccupied` building has occupants and all four exit-edge scans fail.

## 0. Target / Non-Goals / Completion Gate

**Target question:** When `BuildingClass::SellBuilding @ 0x00457DE0` cannot find a ground exit cell for `CanBeOccupied` garrison occupants and calls `SpawnUnitsWithParachute(0)`, do occupants parachute or die, in what order, under which owner/mission/landing state, and what RNG is consumed?

**Non-goals:** Do not re-study normal edge ejection or infantry scatter except to prove this fallback is separate. Do not study generic transport unloading, aircraft paradrop carrier formation, or tank bunker occupant release.

**Evidence needed to mark COMPLETE:** live Ghidra decompile plus assembly for the `SellBuilding -> SpawnUnitsWithParachute(0)` call site; live Ghidra decompile plus assembly for `SpawnUnitsWithParachute @ 0x004585C0`; caller/liveness proof for standard YR sell/destruction paths; current Rust surface scan; final open-question log with no open items.

**Stop conditions:** stop once the null-argument branch is drained and a cold pass over `0x00457DE0`/`0x004585C0` adds no new in-scope questions. Do not follow the non-null parachute branch beyond proving it is not used by this fallback.

## 1. Overview

The name `SpawnUnitsWithParachute` is misleading for the no-exit garrison fallback. `SellBuilding` calls it with argument `0`; the helper tests that argument and takes a null branch that reverse-iterates the building's garrison vector and calls each occupant's vtable `+0xF8` destroy/remove virtual. Active in YR: Yes, because `SellBuilding` is called by player sell and by `ReceiveDamage` destruction case 4 for `CanBeOccupied` buildings.

No occupant is unlimboed, no `PARACH` anim is created, no falling/parachute state is attached, no scatter/mission is queued, no owner transfer happens in this fallback, and no RNG is consumed. The existing Rust behavior that kills occupants when no edge cell is available is directionally correct for this exact no-exit case, though its comment saying "no parachute fallback exists yet" is stale.

## 2. Class Layout / Key Offsets

| Offset / field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass+0x684` | garrison `DynamicVectorClass` object used for cleanup after either branch | `0x004585C0` decompile; assembly `0x00458704..0x00458722` | Yes |
| `BuildingClass+0x688` | garrison occupant pointer array | `0x004585C0`; null branch loads `[ESI+0x688]` then `[EAX+EDI*4]` | Yes |
| `BuildingClass+0x68C` | vector capacity/metadata passed back to vector reset | `0x004585C0`; assembly `0x0045870A`, `0x0045871D..0x00458722` | Yes |
| `BuildingClass+0x694` | occupant count; loop starts at count and decrements before use | `0x004585C0`; assembly `0x004585D9..0x004585E2` | Yes |
| `BuildingClass+0x69C` | garrison fire index; reset again at helper entry | `0x004585C0`; assembly `0x004585CF` | Yes |
| helper arg at stack `[ESP+0x2C]` after prologue | non-null branch selector; `SellBuilding` passes `0` | `0x00458180..0x00458184`, `0x004585C5..0x004585DF` | Yes for null branch in this fallback |

## 3. Core Logic

### 3.1 Fallback entry from `SellBuilding`

In `BuildingClass::SellBuilding`, after four deterministic exit-edge scan attempts, the final branch checks the foundation height byte. If no candidate coordinate has been accepted and the scan is exhausted, it calls `SpawnUnitsWithParachute(0)` and returns before the normal unlimbo/scatter/mission loop. Active in YR: Yes.

Assembly context:

- `0x00458027 JGE 0x00458140` exits the final scan when the last edge coordinate range is exhausted.
- `0x00458140 MOV AL, byte ptr [ESP+0x44]`; `0x00458144 TEST AL,AL`; `0x00458146 JZ 0x00458180` selects the no-candidate fallback when the saved foundation-height byte is zero/exhausted.
- `0x00458180 PUSH 0x0`; `0x00458182 MOV ECX,ESI`; `0x00458184 CALL 0x004585C0`; `0x00458190 RET 0x8`.

The only argument pushed by this fallback is literal zero. Therefore, all parachute/falling behavior in the non-null branch of `0x004585C0` is not active for this garrison no-exit fallback.

### 3.2 Null-argument branch in `SpawnUnitsWithParachute @ 0x004585C0`

The helper begins by resetting `Building+0x69C` to zero and loading the occupant count. It then branches on the helper argument:

1. `EBP = [ESP+0x2C]` receives the helper argument.
2. `TEST EBP,EBP`.
3. If `EBP == 0`, load count from `[ESI+0x694]`, decrement to `count - 1`, and loop down to `0`.
4. For each index, load the occupant pointer from `[Building+0x688 + index*4]`.
5. Call the occupant vtable `+0xF8`.
6. After the loop, reset/clear the garrison vector and call `FUN_0070F6E0` on the building's current cell/list result.

Assembly context:

- `0x004585CD TEST EBP,EBP`; `0x004585DF JNZ 0x00458601` proves the non-null branch is skipped for `SpawnUnitsWithParachute(0)`.
- Null branch: `0x004585D9 MOV EDI,[ESI+0x694]`; `0x004585E1 DEC EDI`; `0x004585E2 JS 0x00458704`; `0x004585E8 MOV EAX,[ESI+0x688]`; `0x004585EE MOV ECX,[EAX+EDI*4]`; `0x004585F3 CALL [EDX+0xF8]`; `0x004585F9 DEC EDI`; `0x004585FA JNS 0x004585E8`.
- Cleanup: `0x00458704 MOV EDX,[ESI+0x684]`; `0x00458718 CALL [EDX+0xC]`; `0x0045871D PUSH 0`; `0x0045871F PUSH EBX`; `0x00458722 CALL [EAX+0x8]`; `0x00458729 CALL [EDX+0x1BC]`; `0x00458732 CALL 0x0070F6E0`.

Material result: occupants are destroyed/removed in reverse occupant-vector order. They do not parachute. Active in YR: Yes.

### 3.3 Non-null branch is not the scoped fallback

The non-null branch of `0x004585C0` creates an `AnimClass` with constructor flags including `0x600`, writes `anim+0x100 = -200`, calls the non-null argument object's vtable `+0xE0` with the occupant, then calls the occupant vtable `+0xF8`. This is why older docs could associate the helper with parachute visuals. Active in YR: Conditional for callers that pass a non-null object; not active for `SellBuilding`'s no-exit fallback because the call site pushes `0`.

Important negative split: the scoped fallback is not the ordinary aircraft paradrop path described in `PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`. That real parachute path uses `ObjectClass::Unlimbo @ 0x005F5940`, object falling state, and attached `PARACH`; none of those calls occur on the null branch of `0x004585C0`.

### 3.4 Owner, landing state, mission state, and RNG

Because the null branch only calls each occupant's destroy/remove virtual and then clears the building vector:

- Owner: no owner field is read or written by the null branch. The occupant is removed before any owner-visible landing or mission state can be established. Active in YR: Yes.
- Landing/parachute state: no `Unlimbo`, no `ObjectClass::Unlimbo @ 0x005F5940`, no `Object+0x8D` falling setup, no `PARACH` anim, and no landing mission transition. Active in YR: Yes as a negative fact.
- Mission state: no occupant vtable `+0x174` scatter call and no occupant vtable `+0x1E8` mission queue call occur on this branch. Active in YR: Yes.
- RNG: no calls to `Random::RandomRanged @ 0x0065C7E0`, `Random::Next`, `RateTimer::Current`, or modulo-like selection occur in the null branch. Active in YR: Yes.

## 4. INI Keys

| Key | Stock YR value / source | Role in this slice | Binary reader/use | Active in YR |
|---|---|---|---|---|
| `CanBeOccupied` | many stock civilian buildings set `yes` in `rules.ini`/`rulesmd.ini` | enables the destruction caller gate and makes the garrison occupant vector relevant | destruction gate reads `BuildingType+0x157B` in `0x00442230` | Yes |
| `ParachuteMaxFallRate` | `-3` | not used by this fallback; relevant only to real falling objects with attached chute | read to `Rules+0x7B8`, used by `ObjectClass::AI @ 0x005F3E70` per prior parachute report | No for this fallback |
| `NoParachuteMaxFallRate` | `-100` | not used by this fallback | read to `Rules+0x7BC`, used by no-chute falling branch per prior parachute report | No for this fallback |
| `Parachute` | `PARACH` | not used by this fallback | read to `Rules+0xBBC`, used by `ObjectClass::Unlimbo @ 0x005F5940` per prior parachute report | No for this fallback |
| `BombParachute` | `PARABOMB` | not used by this fallback | read to `Rules+0xBB8` per prior parachute report | No for this fallback |
| `ChuteSound` | `ParachuteDrop` | not played by this fallback | read to `Rules+0x71C`; ordinary drop sound path per prior parachute report | No for this fallback |

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Player sell | `BuildingClass::Sell @ 0x00449C30` calls `BuildingClass::SellBuilding` when occupant count is positive | live decompile `0x00449C30`, call to `BuildingClass__SellBuilding(param_1)` after vtable `+0x408` count check | Yes |
| Building destruction | `BuildingClass::ReceiveDamage @ 0x00442230` case 4 calls `SellBuilding` when `Type+0x157B CanBeOccupied` is true | live decompile `0x00442230` case 4 | Yes |
| No-exit fallback | `SellBuilding` calls `0x004585C0` with literal `0` and returns | assembly `0x00458180..0x00458190` | Conditional on no accepted ground exit cell |
| Normal edge ejection | bypassed when fallback fires; no unlimbo/scatter loop after the fallback call | `0x00458190 RET 0x8`; normal loop starts at `0x00458060` only when a coordinate exists | Yes |
| Real parachute descent | separate path using `ObjectClass::Unlimbo @ 0x005F5940` and `PARACH`; not reached here | prior `PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`; absence of those calls in null branch | No for this fallback |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Match status | Evidence |
|---|---|---|---|
| `src/sim/production/production_sell.rs::eject_garrison_passengers_at_edges` | if no edge cell is selected for an occupant, sets health to 0, `dying = true`, and clears passenger role | mostly matches final outcome for this exact no-exit branch | codegraph context plus Rust scan around lines 243..283 |
| `src/sim/production/production_sell.rs` comment at no-edge branch | says "no parachute fallback exists in this helper yet" | stale/misleading | binary has named helper, but `SpawnUnitsWithParachute(0)` kills/removes rather than parachuting |
| `src/sim/movement/parachute_descent.rs` / `src/app_chute_anim.rs` | real parachute state/render exists for paradrops | should not be invoked for this fallback | binary null branch has no Unlimbo/falling/chute calls |
| `src/sim/production/production_sell.rs::eject_destruction_garrison` | routes destroyed garrison through same edge helper and no-edge death | aligned for fallback result | code scan and live destruction caller |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SellBuilding @ 0x00457DE0` no-exit call site | verified | decompile plus assembly `0x00458180..0x00458190` | none |
| `SpawnUnitsWithParachute @ 0x004585C0` null branch | verified | decompile plus assembly `0x004585CD..0x004585FC` | none |
| `SpawnUnitsWithParachute @ 0x004585C0` non-null branch | touched-not-exhausted | decompile `0x00458601..0x004586FE` | out-of-scope callers/visual semantics |
| Player sell caller | verified | live decompile `0x00449C30` | none for fallback liveness |
| Destruction caller | verified | live decompile `0x00442230` | none for `CanBeOccupied` gate |
| Real paradrop chute state | deferred | prior parachute report | out-of-scope; only used for negative comparison |
| Current Rust no-edge branch | verified | `production_sell.rs` scan and codegraph context | no patch performed |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Does `SellBuilding` pass a non-null parachute object to the helper? -> No; the no-exit fallback executes `PUSH 0` before `CALL 0x004585C0`.` (evidence: `0x00458180..0x00458184`)
- `[RESOLVED] OQ-02 -- What does `0x004585C0` do when the argument is null? -> It reverse-iterates `[Building+0x688]` from `count-1` to `0` and calls each occupant vtable `+0xF8`.` (evidence: `0x004585CD..0x004585FC`)
- `[RESOLVED] OQ-03 -- Are occupants unlimboed in the fallback? -> No; no vtable `+0xD8` Unlimbo call exists in the null branch.` (evidence: `0x004585CD..0x004585FC`)
- `[RESOLVED] OQ-04 -- Do occupants get attached `PARACH`/falling state? -> No; the null branch does not call `ObjectClass::Unlimbo @ 0x005F5940` or create an `AnimClass`.` (evidence: `0x004585CD..0x004585FC`; prior parachute report)
- `[RESOLVED] OQ-05 -- Is any scatter or mission queued? -> No; no vtable `+0x174` or `+0x1E8` call occurs in the null branch.` (evidence: `0x004585CD..0x004585FC`)
- `[RESOLVED] OQ-06 -- Is RNG consumed? -> No; the null branch has no `RandomRanged`, `Random::Next`, or timer-derived direction selection call.` (evidence: `0x004585CD..0x004585FC`)
- `[RESOLVED] OQ-07 -- Is the branch active in YR? -> Yes when a live `CanBeOccupied` sell/destruction garrison has occupants and no accepted edge exit cell.` (evidence: `0x00449C30`; `0x00442230`; `0x00457DE0`; `CanBeOccupied` INI entries)
- `[RESOLVED] OQ-08 -- What happens to the building garrison vector after removal? -> The helper calls vector cleanup/reset and then `FUN_0070F6E0` on the building cell/list result.` (evidence: `0x00458704..0x00458732`)
- `[RESOLVED] OQ-09 -- Does owner matter to the fallback result? -> No owner read/write appears in the null branch; occupants are removed before landing/mission state.` (evidence: `0x004585CD..0x004585FC`)
- `[DEFERRED] OQ-10 -- Exact semantics of the non-null `SpawnUnitsWithParachute` branch.` (category: out-of-scope; reason: `SellBuilding` no-exit fallback passes null; next-step-if-pursued: separate caller inventory for `0x004585C0` non-null callers)
- `[DEFERRED] OQ-11 -- Exact internals of occupant vtable `+0xF8` for each occupant subclass.` (category: requires-different-system-context; reason: prior garrison reports already classify it as destroy/remove and this slice only needs the call/order; next-step-if-pursued: vtable binding audit for InfantryClass destroy/remove)
- `[DEFERRED] OQ-12 -- Runtime reproduction map that blocks all four edge scans in retail.` (category: needs-runtime-debugger; reason: binary branch is proven, but a retail fixture/log was not created in this read-only slice; next-step-if-pursued: construct a dense blocked-edge map and log occupant removal order)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `SellBuilding` no-exit fallback calls `SpawnUnitsWithParachute(0)`, whose null branch destroys/removes occupants in reverse vector order. | `0x00458180..0x00458184`; `0x004585D9..0x004585FA` | mostly matched; Rust kills no-edge occupants but comment is stale | `src/sim/production/production_sell.rs::eject_garrison_passengers_at_edges` | Preserve kill/remove result for no accepted edge exit; document that the named helper's null branch is a death/removal fallback, not parachuting. | Fully block all edge exit cells for a 3-occupant garrison and assert occupants are removed/marked dying in LIFO order with zero successful ejections. Proposed test: `garrison_no_exit_spawnunitswithparachute_zero_kills_lifo` | Do not add parachute descent for `SpawnUnitsWithParachute(0)`. |
| No Unlimbo/scatter/mission/RNG happens on the null branch. | `0x004585CD..0x004585FC` has only `+0xF8` calls before vector cleanup | Rust should avoid fallback scatter/parachute RNG; current no-edge path does not scatter | `production_sell.rs`, future mission/scatter queue surface | Keep no-edge fallback separate from successful edge ejection; do not consume `RandomRanged(0,4)` or queue mission `0xF` for killed occupants. | With blocked exits and deterministic RNG state, assert RNG state and mission queues are unchanged after no-exit fallback while occupants die. Proposed test: `garrison_no_exit_no_scatter_mission_or_rng` | Do not reuse the successful ejection scatter handoff for occupants that never unlimbo. |
| Real parachute descent (`PARACH`, falling state, landing mission) is absent from this fallback despite the helper name. | null branch `0x004585CD..0x004585FC`; real parachute path is `0x005F5940` per prior report | Rust should not invoke `begin_parachute_descent` or render chute for this fallback | `src/sim/movement/parachute_descent.rs`, `src/app_chute_anim.rs`, `production_sell.rs` | Ensure no entity gains `parachute_state`/chute anim when a sold/destroyed garrison has no ground exit. | Block all exits and assert all occupant entities either removed or dying with `parachute_state == None`. Proposed test: `garrison_no_exit_does_not_attach_parachute_state` | The function name is not a behavior spec; the literal zero argument selects the non-parachute branch. |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/gap-scans/2026-05-04-disparity-scan-garrison.md:37` should replace: "falls back to `SpawnUnitsWithParachute()` if no exit cell is available. Only kills the infantry if Unlimbo on the chosen cell fails." with: "if no exit cell is available, calls `SpawnUnitsWithParachute(0)`. In the null-argument branch, gamemd reverse-iterates the garrison vector and calls each occupant's destroy/remove virtual; occupants do not parachute. A separate kill also happens if a chosen exit coordinate exists but an individual `Unlimbo` fails."
- `C:/Users/enok/Documents/ra2-rust-game/docs/gap-scans/2026-05-04b-disparity-scan-garrison.md:102` should replace: "falls to `SpawnUnitsWithParachute` if all four edges fail" with: "falls to `SpawnUnitsWithParachute(0)` if all four edges fail; that null-argument branch removes/destroys occupants rather than spawning parachuted infantry."
- `C:/Users/enok/Documents/ra2-rust-game-docs/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md:48` should replace: "If all edge searches fail before a candidate is found, it calls `SpawnUnitsWithParachute(0)` and returns." with: "If all edge searches fail before a candidate is found, it calls `SpawnUnitsWithParachute(0)` and returns; `0x004585C0` with a null argument destroys/removes occupants in reverse vector order and does not create parachute/falling state."
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md:64` should replace: "If no exit coordinate can be found before the loop, the helper calls `SpawnUnitsWithParachute(0)` and returns rather than killing all occupants in-place." with: "If no exit coordinate can be found before the loop, the helper calls `SpawnUnitsWithParachute(0)` and returns; live Ghidra shows the null-argument branch kills/removes all garrison occupants in reverse vector order rather than parachuting them."
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_sell.rs:278` comment should replace: "All cells blocked; no parachute fallback exists in this helper yet." with: "All edge exits blocked; gamemd calls `SpawnUnitsWithParachute(0)`, whose null branch removes/kills occupants instead of parachuting them."

## 10. Negative Facts / Do Not Do

- Do not implement `SpawnUnitsWithParachute(0)` as a parachute landing fallback. Active in YR: Yes. Evidence: `0x00458180 PUSH 0`; `0x004585CD TEST EBP,EBP`; null branch `0x004585E8..0x004585F3` calls occupant `+0xF8`.
- Do not call `begin_parachute_descent`, create `PARACH`, use `ParachuteMaxFallRate`, or play `ChuteSound` for this no-exit fallback. Active in YR: Yes as a negative fact. Evidence: no `ObjectClass::Unlimbo`/`AnimClass` calls in `0x004585CD..0x004585FC`.
- Do not queue scatter mission `0xF` or movement mission `2` for occupants on this branch. Active in YR: Yes. Evidence: null branch contains no vtable `+0x174` or `+0x1E8`.
- Do not consume scatter RNG or any fallback placement RNG for this branch. Active in YR: Yes. Evidence: null branch contains no RNG/timer calls.
- Do not treat the non-null `SpawnUnitsWithParachute` branch as evidence for sold/destroyed `CanBeOccupied` no-exit garrisons. Active in YR: No for this fallback. Evidence: `SellBuilding` pushes literal `0`.

## 11. Remaining Uncertainty

None for the scoped no-exit fallback. Deferred items are outside this target: non-null `SpawnUnitsWithParachute` callers, subclass-specific internals of vtable `+0xF8`, and a retail runtime fixture for the blocked-edge condition.

## Sources

- Live Ghidra decompile: `BuildingClass::SellBuilding @ 0x00457DE0`; `SpawnUnitsWithParachute @ 0x004585C0`; `BuildingClass::Sell @ 0x00449C30`; `BuildingClass::ReceiveDamage @ 0x00442230`.
- Live Ghidra assembly: `0x00458180..0x00458190`; `0x004585CD..0x004585FC`; `0x00458704..0x00458732`.
- Prior docs referenced: `GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`; `GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`; `PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`; `PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`; `SCATTER_ALL_CALLERS_GHIDRA_REPORT.md`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`.
- Rust scanned: `src/sim/production/production_sell.rs`; `src/sim/movement/parachute_descent.rs`; `src/app_chute_anim.rs`; `src/sim/aircraft/drop_payload.rs`.

## Status

COMPLETE.
