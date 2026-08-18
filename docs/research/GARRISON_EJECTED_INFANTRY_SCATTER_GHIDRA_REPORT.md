# Garrison Ejected Infantry Scatter -- Ghidra Research Report

**Address(es):** `0x00457DE0` (`BuildingClass::SellBuilding`), `0x0051D0D0` (`InfantryClass::Scatter`), `0x0065C7E0` (`Random::RandomRanged`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** queued/direct scatter behavior for infantry ejected from `CanBeOccupied` garrisons by sell or destruction paths.
**Non-Scope:** generic transport unload, tank bunker exit, civilian ownership transfer, and full `Find_Nearby_Passable_Cell` internals.
**Confidence:** High for call order, RNG primitive/bounds, and Rust-facing mismatch; Medium for exact mission name of `0xF` beyond existing mission-enum docs.
**Active in YR:** Yes. `CanBeOccupied=yes` civilian garrisons exist in `rulesmd.ini`; destruction case 4 gates on `BuildingTypeClass+0x157B` and calls `SellBuilding`.

## 0. Target / Non-Goals / Completion Gate

**Target question:** Exact `InfantryClass::Scatter` behavior after `CanBeOccupied` garrison sell/destruction ejection: RNG timing, queued mission/order, later scatter reads, direction/jitter, and early-outs affecting visible ejected occupants.

**Non-goals:** Do not re-open elite weapon fallback, bunkerable defaults, CanDock/CanGarrison entry gates, or generic transport unload except to prove separation.

**Evidence needed to mark COMPLETE:** Live Ghidra decompile plus assembly context for the `SellBuilding -> InfantryClass::Scatter -> Queue_Mission` ordering, `RandomRanged` bounds, destruction/sell liveness, and current Rust delta.

**Stop conditions:** Stop after all scoped open questions are resolved/deferred and after a cold re-read of `0x00457DE0` and `0x0051D0D0` adds no new in-scope questions.

## 1. Overview

The older "queue Scatter later, no RNG before queue" wording is incomplete for standard YR. For each successfully unlimboed garrison occupant, `BuildingClass::SellBuilding` calls the occupant scatter virtual (`vtable+0x174`) immediately, using the building center coordinate and two true flags left on the stack, then queues mission `0xF`.

For infantry, that immediate `InfantryClass::Scatter` call can consume scenario RNG before the `0xF` mission queue call. Directional infantry scatter uses `RandomRanged(0,4)`, subtracts 2, adds that jitter to the computed 8-way facing, scans up to 8 directions, queues mission `2` on success, and sets the destination.

## 2. Key Offsets / Data

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass+0x688` | garrison occupant pointer vector | `0x00457DE0` decompile, occupant load before loop | Yes |
| `BuildingClass+0x694` | occupant count, loop starts at count and decrements before use | `0x00457DE0`, assembly around `0x00458098..0x004580B1` from prior docs and live decompile | Yes |
| `BuildingClass+0x69C` | garrison fire index reset at ejection entry | first write in `0x00457DE0` | Yes |
| `Infantry+0x6C4` (`param_1[0x1B1]`) | infantry sequence/state index used by prone and per-sequence scatter table gates | `0x0051D0D0` decompile; assembly entry reads `[ESI+0x6C4]` at `0x0051D0DD` | Yes |
| `Infantry+0x674` (`param_1[0x19D]`) | locomotor pointer; null asserts, vtable `+0x10` busy check can downgrade force | `0x0051D0D0` decompile | Yes |
| `InfantryType+0xEBF` | type gate checked when not forced / PlayerScatter path | `0x0051D0D0` decompile | Yes |
| `InfantryType+0x67C` | speed type passed to null-coordinate nearby-cell helper | `0x0051D0D0` decompile, null-coordinate branch | Conditional; not the sell directional branch |
| `g_RulesClass+0x17ED` | `PlayerScatter` rules byte | `0x0051D0D0` decompile; `rulesmd.ini:900 PlayerScatter=no` | Yes, default false |
| `g_Scenario+0x218` | scenario RNG object used by `RandomRanged` | assembly `0x0051D2AC..0x0051D2BA`, `0x0051D36D..0x0051D385` | Yes |
| `g_DirectionOffsets` | 8-way cell offsets used during directional scan | `0x0051D487` loop in decompile | Yes |

## 3. Core Logic

### 3.1 Ejection ordering in `BuildingClass::SellBuilding @ 0x00457DE0`

For each occupant after a successful exit coordinate is found:

1. `Unlimbo(exit_coord, 0)` is attempted.
2. Failed `Unlimbo` calls occupant vtable `+0xF8` and skips scatter/mission for that occupant.
3. If the occupant's type byte at `type+0x1EC` is set, bytes `occupant+0x691` and `occupant+0x690` are cleared.
4. The occupant receives virtual `+0x3C8(0)`.
5. The building center coordinate is obtained through building vtable `+0x48`.
6. The occupant scatter virtual `+0x174` is called before the mission `0xF` queue.
7. If `occupant+0x5D4` is non-null, `FUN_006EA870(occupant, -1, 0)` runs.
8. The occupant then receives vtable `+0x1E8(0xF, 0)`.

Assembly context proves the critical call order:

- `0x004580F9 PUSH 0x1`, `0x004580FF PUSH 0x1`, `0x00458101 PUSH ECX`, `0x00458104 CALL [EAX+0x48]`, `0x00458107 PUSH EAX`, `0x0045810A CALL [EBX+0x174]`.
- Building `GetCoords @ 0x00447AC0` returns with `RET 0x4`, so the two `0x1` stack values survive and are consumed by `InfantryClass::Scatter`'s `RET 0xC`. This makes the call effectively `Scatter(building_center, 1, 1)` for the ejected occupant.
- `0x00458132 PUSH 0x0`, `0x00458134 PUSH 0xF`, `0x00458138 CALL [EDX+0x1E8]` happens after the scatter virtual call.

**Material correction:** `SellBuilding` can consume scatter RNG before queuing mission `0xF`, because the direct scatter virtual call precedes `0x00458138`.

Active in YR: Yes. Player sell reaches `SellBuilding` in `BuildingClass::Sell @ 0x00449C30` when occupant count is positive. Destruction reaches it from `BuildingClass::ReceiveDamage @ 0x00442230` case 4 when `BuildingTypeClass+0x157B CanBeOccupied` is true.

### 3.2 `InfantryClass::Scatter @ 0x0051D0D0`

The direct ejection call is the directional scatter branch because the first argument is the building center coordinate, not the null coordinate sentinel. For that branch:

1. It computes a facing from the passed coordinate and the infantry's current lepton coordinates.
2. It calls scenario `RandomRanged(0,4)`.
3. It computes `direction = (((atan2_result >> 12) + 1) >> 1) & 7`.
4. It computes `start_dir = direction + roll - 2`.
5. The candidate loop applies `& 7` while scanning up to 8 directions through `g_DirectionOffsets`.
6. Each candidate must be in playfield, pass infantry `Can_Enter_Cell`, and pass the height/snap check using `FUN_006D6410`.
7. If a destination survives, `InfantryClass::Scatter` queues mission `2` with flag `0`, then sets destination through vtable `+0x480(..., 1)`.

Assembly context:

- `0x0051D2AC MOV EAX,[0x00A8B230]`, `0x0051D2B1 PUSH 0x4`, `0x0051D2B3 PUSH EBP`, `0x0051D2B4 LEA ECX,[EAX+0x218]`, `0x0051D2BA CALL 0x0065C7E0`, then `0x0051D2CC LEA EDX,[EAX+ECX-0x2]`.
- `0x0051D36D MOV EDX,[0x00A8B230]`, `0x0051D373 PUSH 0x4`, `0x0051D379 PUSH EBP`, `0x0051D37C LEA ECX,[EDX+0x218]`, `0x0051D385 CALL 0x0065C7E0`, then `0x0051D38C LEA EAX,[EDI+EAX-0x2]`.
- `0x0051D6BE PUSH 0x0`, `0x0051D6C0 PUSH 0x2`, `0x0051D6C4 CALL [EDX+0x1E8]`; `0x0051D6D0 PUSH 0x1`, `0x0051D6DD PUSH EAX`, `0x0051D6E0 CALL [EDI+0x480]`.

Active in YR: Yes. `CellClass::Scatter_Objects @ 0x00481670` also dispatches occupants through vtable `+0x174`, and `SellBuilding` uses the same virtual slot directly.

### 3.3 RNG primitive and bounds

`Random::RandomRanged @ 0x0065C7E0` sorts bounds if high < low, returns immediately without a draw for equal bounds, otherwise draws from the 250-word scenario RNG state with rejection masking and returns an inclusive value in `[low, high]`.

For ejected infantry directional scatter, both live scatter RNG sites push low `0` via `EBP` and high `4`, so the roll is inclusive `0..4`. The visible jitter is `roll - 2`, i.e. `-2, -1, 0, +1, +2`.

Active in YR: Yes. The RNG source is `g_Scenario+0x218`, not raw `Random::Next`, not `% 8`, and not `RandomRanged(0,7)`.

### 3.4 Early-outs visible for ejected occupants

These can prevent the direct scatter from producing a move or from consuming RNG:

- Failed `Unlimbo(exit, 0)` in `SellBuilding` calls vtable `+0xF8`; no scatter virtual call and no `0xF` queue for that occupant. Active in YR: Yes.
- If no exit coordinate is found before the occupant loop, `SellBuilding` calls `SpawnUnitsWithParachute(0)` and returns; this report does not cover that fallback's landing behavior. Active in YR: Yes, conditional on blocked edge search.
- Player-controlled infantry in sequence `0x1B..0x1E` returns if not in the forced+param4 path; the ejection call passes forced/param4 truthy, so it instead calls vtable `+0x558` and continues unless later gates stop it. Active in YR: Conditional.
- Null locomotor asserts; a busy locomotor vtable `+0x10` result downgrades the force byte to false, allowing later non-forced gates to return before RNG. Active in YR: Conditional.
- If mission timer byte `+9` is false and force has been downgraded false, Scatter returns before RNG. Active in YR: Conditional.
- If `InfantryType+0xEBF` is false, `Infantry+0x2B4` is non-null, and force is false, Scatter returns before RNG. Active in YR: Conditional.
- If `Infantry+0x6C4` is neither `-1` nor `0x1F` and `DAT_007EAF7C[index*4]` is false, Scatter returns before RNG even for the direct path. Active in YR: Conditional.
- If no candidate cell survives after the RNG/facing scan, Scatter returns without queuing mission `2` or setting destination; `SellBuilding` still proceeds to the later `0xF` queue. Active in YR: Yes.

## 4. INI Keys

| Key | Default / sample | Role | Active in YR |
|---|---|---|---|
| `CanBeOccupied` | many `rulesmd.ini` civilian structures set `yes`; destruction gate reads `BuildingType+0x157B` | enables the destruction path that calls `SellBuilding` | Yes |
| `[CombatDamage] PlayerScatter` | `rulesmd.ini:900 PlayerScatter=no` | scatter eligibility gate when the call is not forced; ejection direct call starts forced | Yes, default false |
| `[IQ] Scatter` | `rulesmd.ini:3164 Scatter=2` | dispatch eligibility in `CellClass::Scatter_Objects`; not the direct `SellBuilding` virtual call | Conditional/out-of-scope for ejection direct call |

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Player sell state | Calls `BuildingClass::SellBuilding` when occupant count is positive | `0x00449C30` decompile | Yes |
| Building destruction | Case 4 calls `SellBuilding` when `Type+0x157B CanBeOccupied` is true | `0x00442230` decompile | Yes |
| Direct scatter call | `SellBuilding` calls occupant vtable `+0x174` before queuing `0xF` | assembly `0x004580F9..0x0045810A`, `0x00458132..0x00458138` | Yes |
| Scatter destination | Successful directional infantry scatter queues mission `2`, then sets destination | assembly `0x0051D6BE..0x0051D6E0` | Yes |
| Generic transport unload | Separate passenger list/helper in sell state; not the `+0x688/+0x694` garrison vector path | `0x00449C30` decompile, prior passenger report | Conditional and separate |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Match status | Evidence |
|---|---|---|---|
| `src/sim/production/production_sell.rs::eject_garrison_passengers_at_edges` | places occupants, then approximates scatter with `sim.rng.next_u32() as usize % 8` and `movement::issue_direct_move` | mismatch | Rust scan; binary uses immediate class scatter with `RandomRanged(0,4)` directional jitter and early-outs |
| `src/sim/production/production_sell.rs::eject_destruction_garrison` | routes destruction through same helper as sell | mostly matches path sharing | Rust scan; binary destruction calls `SellBuilding` |
| future mission/scatter surface | no verified class-specific `InfantryClass::Scatter` mission behavior wired for ejected occupants | missing | binary direct scatter plus mission `2`/`0xF` ordering |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::SellBuilding @ 0x00457DE0` ejection call order | verified | live decompile plus assembly `0x004580F9..0x00458138` | exact edge-search pseudocode not re-expanded |
| `InfantryClass::Scatter @ 0x0051D0D0` direct directional branch | verified | live decompile plus assembly `0x0051D2BA`, `0x0051D385`, `0x0051D6C4` | exact `DAT_007EAF7C` table contents not dumped |
| `Random::RandomRanged @ 0x0065C7E0` primitive | verified | live decompile | none for `0..4` inclusive bound |
| Player sell caller | verified | live decompile `0x00449C30` | none |
| Destruction caller | verified | live decompile `0x00442230` | none |
| Generic transport unload | deferred | separate helper/list in `0x00449C30` | out-of-scope generic unload report |
| Parachute fallback | touched-not-exhausted | `0x00457DE0` calls `SpawnUnitsWithParachute(0)` | fallback landing/order requires separate report |
| Current Rust garrison ejection | verified | `production_sell.rs` scan | implementation patch not performed |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Does `SellBuilding` call scatter directly before queuing mission `0xF`? -> Yes, occupant vtable `+0x174` at `0x0045810A` runs before queue call `0x00458138`.` (evidence: `0x004580F9..0x00458138`)
- `[RESOLVED] OQ-02 -- Does successful ejection consume RNG before mission `0xF` queue? -> It can; if `InfantryClass::Scatter` passes pre-RNG gates, `RandomRanged(0,4)` runs before `0x00458138`.` (evidence: `0x0045810A`, `0x0051D2BA`, `0x0051D385`)
- `[RESOLVED] OQ-03 -- Which mission/order is queued? -> On successful directional scatter, `InfantryClass::Scatter` queues mission `2` first, then `SellBuilding` queues mission `0xF` after returning.` (evidence: `0x0051D6BE..0x0051D6C4`; `0x00458132..0x00458138`)
- `[RESOLVED] OQ-04 -- Is the direct ejection scatter null-coordinate or directional? -> Directional; the first arg is building center from vtable `+0x48`, not the null coord sentinel.` (evidence: `0x00458104..0x0045810A`; `0x00447AC0 RET 0x4`)
- `[RESOLVED] OQ-05 -- What RNG primitive and bounds are used? -> Scenario `RandomRanged(0,4)` inclusive, jitter `roll - 2`.` (evidence: `0x0051D2AC..0x0051D2CC`; `0x0051D36D..0x0051D38C`; `0x0065C7E0`)
- `[RESOLVED] OQ-06 -- Does the scoped path use raw `% 8`? -> No raw `% 8` appears in `SellBuilding` or `InfantryClass::Scatter`; eight-way behavior is candidate scanning after five-way jitter.` (evidence: live decompile `0x00457DE0`, `0x0051D0D0`)
- `[RESOLVED] OQ-07 -- Are sell and destruction active YR paths? -> Yes; sell state and destruction case both reach `SellBuilding` for occupied/CanBeOccupied cases.` (evidence: `0x00449C30`; `0x00442230`; `rulesmd.ini` CanBeOccupied entries)
- `[RESOLVED] OQ-08 -- What early-outs can make ejected occupants visibly not move? -> failed Unlimbo, pre-RNG scatter gates after force downgrade/table checks, and no surviving candidate cell after RNG scan.` (evidence: `0x00457DE0`; `0x0051D0D0`)
- `[DEFERRED] OQ-09 -- Exact `SpawnUnitsWithParachute(0)` fallback behavior.` (category: `out-of-scope`; reason: parachute fallback is a separate landing/order helper; next-step-if-pursued: targeted parachute fallback investigation)
- `[DEFERRED] OQ-10 -- Exact contents/meaning of every `DAT_007EAF7C` table entry.` (category: `bounded-cost-too-high`; reason: only the table gate's existence matters for this ejected-infantry slice; next-step-if-pursued: mission scatter table decode)
- `[DEFERRED] OQ-11 -- Generic transport unload mapping.` (category: `out-of-scope`; reason: parent explicitly scoped it out except separation proof; next-step-if-pursued: `GENERIC_TRANSPORT_MANUAL_UNLOAD_MAPPING`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Successful garrison ejection calls infantry Scatter immediately before mission `0xF`; Scatter can draw `RandomRanged(0,4)` before the `0xF` queue. | `0x0045810A` before `0x00458138`; `0x0051D2BA` / `0x0051D385` | mismatch: Rust does one immediate raw `% 8` direction and direct move | `src/sim/production/production_sell.rs::eject_garrison_passengers_at_edges`, future infantry scatter system | Replace `% 8` direct adjacent move with class-specific directional Scatter semantics from building center, preserving RNG timing before the later `0xF` mission queue. | Sell a one-occupant `CanBeOccupied` building with fixed RNG and assert the first consumed random value is `RandomRanged(0,4)` in the scatter call before the later mission `0xF` queue. Proposed test: `garrison_ejection_scatter_consumes_randomranged_0_4_before_queue_0f` | Do not implement "no RNG until queued Scatter"; live Ghidra disproves that wording. |
| Directional infantry scatter queues mission `2` on destination success, then `SellBuilding` queues `0xF` after return. | `0x0051D6BE..0x0051D6C4`; `0x00458132..0x00458138` | missing mission-order surface | future mission queue plus `production_sell.rs` | Preserve observable queue/order side effects: direct Scatter may install a move/destination, then ejection code issues mission `0xF`. | With an open adjacent destination, assert ejection records/executes `Move(2)` from Scatter before `0xF` is queued. Proposed test: `garrison_ejection_scatter_move_then_queue_0f_order` | Do not collapse this to only queued `0xF` or only direct move. |
| Direct ejection Scatter has pre-RNG gates and post-RNG no-destination returns; failed `Unlimbo` never scatters or queues `0xF`. | `0x004580BD..0x0045810A`; `0x0051D0D0` gates; `0x0051D694..0x0051D6E6` | Rust always consumes `% 8` after placement and kills on no chosen edge cell | `production_sell.rs`, future scatter destination/passability code | Apply scatter RNG only after successful unlimbo and after Scatter pre-RNG gates; if Scatter finds no destination, still let `SellBuilding` queue `0xF`. | Arrange blocked neighboring cells after a successful exit and assert RNG may be consumed but no direct move destination is installed while `0xF` still queues. Proposed test: `garrison_ejection_scatter_no_destination_still_queues_0f` | Do not use Rust's occupied-cell direct filter as a substitute for `Can_Enter_Cell`/height scan. |

### Negative Facts / Do Not Do

- Do not keep `next_u32() % 8` for ejected infantry scatter. Evidence: live `InfantryClass::Scatter` uses scenario `RandomRanged(0,4)` and scans 8 directions afterward. Active in YR: Yes.
- Do not keep the old "SellBuilding does not consume RNG before queuing scatter" claim. Evidence: `0x0045810A` direct scatter call precedes `0x00458138` mission `0xF` queue. Active in YR: Yes.
- Do not model ejection as "queue `0xF` only." Evidence: direct scatter can queue mission `2` and set destination before `SellBuilding` queues `0xF`. Active in YR: Yes.
- Do not apply this garrison evidence to generic transport unload. Evidence: `0x00449C30` has a separate passenger helper/list before the garrison vector path. Active in YR: Conditional and separate.
- Do not make failed `Unlimbo` consume scatter RNG. Evidence: failed `Unlimbo` branches to vtable `+0xF8` before `0x0045810A`. Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`: replace "Gamemd `BuildingClass::SellBuilding` does not shuffle foundation cells and does not pick an immediate `0..7` direction with raw RNG. It finds one exit coordinate by deterministic foundation-edge scans, unlimbos occupants in reverse occupant-vector order, then queues each successful occupant into Scatter mission `0xF`." with "Gamemd `BuildingClass::SellBuilding` does not shuffle foundation cells and does not pick an immediate raw `% 8` direction. After successful `Unlimbo`, it directly calls the occupant scatter virtual `+0x174` with the building-center coordinate and true flags; for infantry this can consume `RandomRanged(0,4)` before `SellBuilding` later queues mission `0xF`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`: replace "Successful ejection queues Guard then Scatter mission `0xF`; the helper does not draw raw `% 8` scatter direction." with "Successful ejection calls the occupant scatter virtual before the later mission `0xF` queue. For infantry this is directional `InfantryClass::Scatter` using `RandomRanged(0,4)-2`, not raw `% 8`; if the scatter finds a destination, it queues mission `2` before `SellBuilding` queues `0xF`."

## Sources

- Live Ghidra decompile: `0x00457DE0`, `0x00449C30`, `0x00442230`, `0x0051D0D0`, `0x0065C7E0`, `0x00447AC0`, `0x00481670`.
- Live Ghidra assembly context: `0x0045810A`, `0x00458138`, `0x0051D2BA`, `0x0051D385`, `0x0051D6C4`, `0x0051D6E0`, `0x00447B10`.
- Prior docs checked: `PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`, `GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`, `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`, `SCATTER_BUMP_CRUSH_RNG_CLASSIFICATION_GHIDRA_REPORT.md`, `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/production/production_sell.rs`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`.
