# Garrison Ejected Infantry Scatter Ordering -- Ghidra Research Report

**Address(es):** `0x00457DE0` (`BuildingClass::SellBuilding`), `0x0051D0D0` (`InfantryClass::Scatter`), `0x0065C7E0` (`Random::RandomRanged`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** successful `CanBeOccupied` garrison occupant ejection after `SellBuilding` finds an exit cell and `Unlimbo(exit, 0)` succeeds; exact ordering of direct scatter, scatter RNG, mission `2`, destination set, and later mission `0xF`.
**Non-Scope:** exact foundation-edge exit-cell scan order, no-exit `SpawnUnitsWithParachute(0)` internals, all non-infantry scatter variants, generic transport unload, full mission scheduler execution after queue writes.
**Confidence:** High
**Active in YR:** Yes. Standard YR has many `CanBeOccupied=yes` civilian structures in `ini/rulesmd.ini`, and prior verified caller docs plus fresh Ghidra re-read show sell/destruction/red-HP ejection reach `SellBuilding`.
**Status:** COMPLETE

## 1. Overview

Successful garrison ejection is not "place then immediately choose `next_u32() % 8`." `SellBuilding` unlimbos each occupant at the selected exit coordinate, directly calls the occupant scatter virtual before the later mission `0xF` queue, then continues the occupant loop.

For infantry, the direct virtual resolves to `InfantryClass::Scatter @ 0x0051D0D0`. On the live directional branch reached by the ejection call, Scatter can consume scenario RNG through `RandomRanged(0,4)`, queue mission `2` with argument `0`, and set a destination before `SellBuilding` queues mission `0xF`.

## 2. Target Question, Non-Goals, Completion Evidence, Stop Conditions

**Target question:** After a `CanBeOccupied` garrison occupant is successfully unlimboed by `SellBuilding`, what exact ordering occurs among direct Scatter, scenario RNG, DAT_007EAF7C/table gates, mission `2`, destination write, and later mission `0xF`?

**Non-goals:** Do not redo the foundation-edge exit scan except as setup; do not analyze no-exit fallback beyond confirming it does not enter this successful path; do not generalize this report to unit/non-infantry scatter.

**Evidence needed to mark COMPLETE:** fresh Ghidra decompile plus assembly context for `0x004580F9..0x00458138`; fresh decompile plus assembly context for `InfantryClass::Scatter @ 0x0051D0D0` direct directional branch; fresh decompile of `Random::RandomRanged @ 0x0065C7E0`; Rust scan of `production_sell.rs` showing current `% 8`/direct-move approximation.

**Stop conditions:** stop after confirming direct call order, Scatter pre-RNG gates that affect ejected occupants, RNG primitive/bounds, mission queue order, and Rust handoff. Defer only full scatter caller census and full exit-cell scan because they are outside this slot.

## 3. Class Layout / Key Offsets

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass+0x688` | occupant pointer array | `SellBuilding` decompile loop indexing `field_0x688 + i*4` | Yes |
| `BuildingClass+0x694` | occupant count | `SellBuilding` decompile initializes loop from this field | Yes |
| `BuildingClass+0x69C` | garrison fire index reset at helper entry | `SellBuilding` decompile first write clears `field_0x69c` | Yes |
| `Infantry+0x674` | locomotor pointer, null-asserted then queried by vtable `+0x10` | `0x0051D156..0x0051D16E` and decompile | Yes |
| `Infantry+0x6C0` | infantry type pointer | Scatter reads type gates including `+0xEBF` | Yes |
| `Infantry+0x6C4` | sequence/state index used by prone/table gates | `0x0051D0DD`, `0x0051D1AA..0x0051D1C3` | Yes |
| `InfantryType+0xEBF` | scatter capability gate when force has been downgraded false | `0x0051D190..0x0051D1A4`, `0x0051D212..0x0051D220` | Conditional; direct ejection starts forced |
| `Foot/Techno+0x5D4` | parent passenger/transport link used for optional cleanup after direct scatter | `0x0045811C..0x0045812B`; `FUN_006EA870` decompile | Conditional |
| `g_RulesClass+0x17ED` | `PlayerScatter` rule byte | `0x0051D1C9..0x0051D1D6`; `ini/rulesmd.ini:900 PlayerScatter=no` | Yes, default false |
| `g_Scenario+0x218` | scenario RNG object used by directional Scatter | `0x0051D2AC..0x0051D2BA` | Yes |
| `DAT_007EAF7C` | byte table gate indexed as `index*4` for most sequence states | `0x0051D1BA..0x0051D1C3` | Conditional; affects ejected occupants before RNG |

## 4. Core Logic

### 4.1 `SellBuilding` successful occupant path

The relevant live ordering after the exit coordinate is established:

1. `SellBuilding` increments `g_MapEditorMode` before the occupant loop and decrements it after the loop.
2. It iterates occupants high index to low.
3. For each occupant, it calls occupant `Unlimbo(exit_coord, 0)` through vtable `+0xD8`.
4. If `Unlimbo` returns false, it calls occupant vtable `+0xF8` and skips scatter and mission `0xF` for that occupant.
5. If `Unlimbo` succeeds, and the infantry type has the checked `+0x1EC` byte true, it clears occupant bytes `+0x690` and `+0x691`.
6. It calls occupant vtable `+0x3C8` with `0`.
7. It computes the building center through building vtable `+0x48`, using arguments already arranged on the stack.
8. It calls occupant vtable `+0x174`.
9. Only after that direct scatter call returns, if the success flag at `[ESP+0x40]` is true, it optionally calls `FUN_006EA870(parent, occupant, -1, 0)` when `occupant+0x5D4` is non-null, then queues mission `0xF` by vtable `+0x1E8`.

Fresh assembly evidence:

- `0x004580BD CALL [EDX+0xD8]`, `0x004580C3 TEST AL,AL`, `0x004580C5 JZ 0x00458193`; failed unlimbo reaches the destroy/remove side and does not reach `0x0045810A`.
- `0x004580EF CALL [EDX+0x3C8]` precedes the scatter call.
- `0x004580F9 PUSH 0x1`, `0x004580FF PUSH 0x1`, `0x00458101 PUSH ECX`, `0x00458104 CALL [EAX+0x48]`, `0x00458107 PUSH EAX`, `0x0045810A CALL [EBX+0x174]`.
- `BuildingClass::GetCoords @ 0x00447AC0` ends at `0x00447B11 RET 0x4`, so it consumes only its output pointer; the two pushed `1` flags remain for the later `Scatter` call.
- `InfantryClass::Scatter @ 0x0051D0D0` ends at `0x0051D6ED RET 0xC`, matching one coordinate pointer plus two flag arguments.
- `0x00458110 MOV AL,[ESP+0x40]`, `0x00458114 TEST AL,AL`, `0x00458116 JZ 0x0045819D` gates the later mission `0xF` block.
- `0x0045811C MOV ECX,[EDI+0x5D4]`, `0x00458122 TEST ECX,ECX`, `0x00458124 JZ 0x00458130`, then pushes `0`, `-1`, `EDI` and calls `0x006EA870`; fresh decompile of `0x006EA870` removes/decrements passenger links and clears parent linkage, so this is conditional parent cleanup, not a mission setup.
- `0x00458132 PUSH 0x0`, `0x00458134 PUSH 0xF`, `0x00458138 CALL [EDX+0x1E8]` is after `0x0045810A`.

No Guard `SetMission` call is present in this post-`Unlimbo` handoff. The pre-scatter `vtable+0x3C8(0)` is archive-target clearing according to `TECHNOCLASS_VTABLE_COMPLETE.md` and `TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`; the mission setter in this slice is `vtable+0x1E8`, and the only post-scatter mission setter observed in `SellBuilding` is `+0x1E8(0xF,0)`.

Active in YR: Yes for occupied `CanBeOccupied` garrison sell/destruction/red-HP ejection paths. This slot did not re-prove every caller; it relies on prior verified caller reports and fresh decompile of the shared helper.

### 4.2 `InfantryClass::Scatter` direct directional branch

The `SellBuilding` call passes the building center coordinate and true/true flags. In Scatter, the low byte of the third argument is copied into `BL` at entry and acts as the initial force-like flag for the branch gates.

Pre-RNG gates that matter for ejected occupants:

- If the sequence/state index at `Infantry+0x6C4` is `0x1B..0x1E` and both low-byte flags are true, Scatter first calls vtable `+0x558`; otherwise, player-controlled infantry in those states can return early. Evidence: `0x0051D0DD..0x0051D120`.
- Locomotor pointer `+0x674` is null-asserted, then vtable `+0x10` can clear the force byte (`BL=0`) if the locomotor reports busy/blocked state. Evidence: `0x0051D156..0x0051D172`.
- Mission timer entry byte `+9` and type `+0xEBF` can return before RNG only if force has been downgraded false. Evidence: `0x0051D17B..0x0051D1A4`, `0x0051D212..0x0051D220`.
- The DAT table gate is independent of that force byte: if `Infantry+0x6C4` is neither `-1` nor `0x1F`, Scatter reads `byte [0x007EAF7C + index*4]`; if zero, it returns at `0x0051D6E6` before RNG. Evidence: `0x0051D1AA..0x0051D1C3`.

Directional RNG and direction seed:

- The direct branch compares the incoming coordinate against the null/sentinel coordinate at `DAT_00A8F200..208`; ejected occupants pass a building center coordinate, so they take the non-null directional branch.
- Direction comes from `atan2(target-current)` converted by `Math::ftol`, then `((angle >> 12) + 1) >> 1 & 7`.
- Scatter calls scenario RNG through `g_Scenario+0x218`: `0x0051D2AC MOV EAX,[0x00A8B230]`, `0x0051D2B1 PUSH 0x4`, `0x0051D2B3 PUSH EBP` where `EBP` is zero, `0x0051D2B4 LEA ECX,[EAX+0x218]`, `0x0051D2BA CALL 0x0065C7E0`.
- The start direction is `RandomRanged(0,4) - 2 + (((angle >> 12) + 1) >> 1 & 7)`. Evidence: `0x0051D2BF..0x0051D2D0`.

Mission/destination ordering inside Scatter:

- After candidate-cell scan, if no candidate is found, Scatter returns without mission `2` or destination set. Evidence: `0x0051D694..0x0051D6BA` falls through to return on sentinel.
- If a candidate exists, Scatter queues mission `2` before setting the destination: `0x0051D6BE PUSH 0`, `0x0051D6C0 PUSH 0x2`, `0x0051D6C4 CALL [EDX+0x1E8]`, then `0x0051D6CC..0x0051D6E0` gets the cell and calls vtable `+0x480(..., 1)`.
- Therefore the order on a successful destination is: `Unlimbo` succeeds -> direct Scatter call -> `RandomRanged(0,4)` if gates allow -> Scatter queues mission `2` -> Scatter sets destination -> Scatter returns -> `SellBuilding` queues mission `0xF`.

Active in YR: Yes for infantry occupants ejected from standard YR `CanBeOccupied` garrisons, conditional on Scatter pre-RNG gates and candidate-cell availability.

### 4.3 `Random::RandomRanged @ 0x0065C7E0`

`RandomRanged` takes a random object in `ECX` and two stack bounds. Fresh decompile shows:

- If `low == high`, it returns `low` without a draw.
- If `high < low`, it swaps the bounds.
- It computes inclusive span `high - low + 1`.
- It draws from the object's 250-word XOR-style state unless the object's first byte is nonzero, in which case the draw value is forced to zero.
- It masks/rejects until the value is within the inclusive span, then returns `low + value`.

For ejected infantry directional Scatter the bounds are exactly `0` and `4`, and the RNG object is `g_Scenario+0x218`, not raw `next_u32()`, not `% 8`, and not `RandomRanged(0,7)`.

## 5. INI Keys

| Key | Source/default | Binary use in this slice | Active in YR |
|---|---|---|---|
| `CanBeOccupied=yes` | many civilian structures in `ini/rulesmd.ini`; e.g. first uncommented hit at line 13002 | activates the occupant system and destruction/sell ejection in prior verified docs | Yes |
| `[CombatDamage] PlayerScatter=no` | `ini/rulesmd.ini:900` | read as `g_RulesClass+0x17ED` in Scatter; default false affects non-forced/player-scatter gates | Yes, default false |
| `[IQ] Scatter=2` | `ini/rulesmd.ini:3164` | related to scatter behavior in broader system, but not directly consumed in the successful ejected-occupant ordering proven here | Deferred/out-of-scope |

## 6. Integration Points

| Point | Evidence | Ordering |
|---|---|---|
| `SellBuilding -> occupant vtable +0x3C8(0)` | `0x004580E9..0x004580F3`; vtable docs | archive-target clear after `Unlimbo`, not Guard mission |
| `SellBuilding -> occupant vtable +0x174` | `0x004580F9..0x0045810A` | after `Unlimbo` and archive-target clear, before optional parent cleanup and mission `0xF` |
| `InfantryClass::Scatter -> mission 2` | `0x0051D6BE..0x0051D6C4` | after candidate cell chosen, before destination set |
| `InfantryClass::Scatter -> set destination` | `0x0051D6CC..0x0051D6E0` | after mission `2` queue |
| `SellBuilding -> optional parent cleanup` | `0x0045811C..0x0045812B`; `0x006EA870` decompile | after direct Scatter, before mission `0xF`, only when `occupant+0x5D4` is non-null |
| `SellBuilding -> mission 0xF` | `0x00458132..0x00458138` | after direct Scatter returns |
| `CellClass::Scatter_Objects -> vtable +0x174` | `0x00481670` decompile | corroborates the same scatter virtual slot used by other cell scatter callers |

## 7. Current Rust Implementation Status

`src/sim/production/production_sell.rs:313..335` currently documents the path as an approximation and uses `sim.rng.next_u32() as usize % 8` followed by `movement::issue_direct_move(...)`. This is RED for exact ordering:

- It consumes raw modulo RNG during the ejection helper instead of calling a Scatter-equivalent operation with `RandomRanged(0,4)` after Scatter pre-RNG gates.
- It collapses Scatter's mission `2` queue plus destination write into an immediate movement helper.
- It lacks the later mission `0xF` ordering surface.
- It blocks/chooses destinations from Rust's `occupied_cells`/`used_cells` check, not Scatter's `Can_Enter_Cell`, effective-height, line projection, and sentinel fallback sequence.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SellBuilding` successful occupant ordering | verified | decompile `0x00457DE0`; assembly `0x004580BD..0x00458138` | exact edge scan remains slot-1 scope |
| failed `Unlimbo` no-scatter branch | verified | `0x004580BD..0x004580C5`; decompile branch to vtable `+0xF8` | none for this slot |
| direct scatter argument survival | verified | `0x004580F9..0x0045810A`; `GetCoords RET 0x4` at `0x00447B11`; `Scatter RET 0xC` at `0x0051D6ED` | none |
| DAT_007EAF7C gate as it affects ejected occupants | verified | `0x0051D1AA..0x0051D1C3` | table contents/semantic names are not exhausted |
| direct directional RNG bounds/source | verified | `0x0051D2AC..0x0051D2BA`; `RandomRanged @ 0x0065C7E0` decompile | none |
| Scatter mission `2` before destination | verified | `0x0051D6BE..0x0051D6E0` | exact mission queue data fields out-of-scope |
| later `SellBuilding` mission `0xF` | verified | `0x00458132..0x00458138` | exact mission execution after queue out-of-scope |
| no-exit fallback | deferred | prior report; out-of-scope here | separate no-exit report already exists |
| full Scatter caller census | deferred | `CellClass::Scatter_Objects @ 0x00481670` touched | not needed for garrison ejected occupant ordering |

## 9. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 -- Does successful ejection call Scatter before mission 0xF? -> Yes; `0x0045810A CALL [EBX+0x174]` precedes `0x00458138 CALL [EDX+0x1E8]` with pushed `0xF`.` (evidence: `0x004580F9..0x00458138`)
- `[RESOLVED] OQ-02 -- What arguments does the direct Scatter call receive? -> Building center coordinate plus two true flags; `GetCoords` consumes only its output pointer (`RET 0x4`) and `Scatter` consumes three args (`RET 0xC`).` (evidence: `0x004580F9..0x0045810A`, `0x00447B11`, `0x0051D6ED`)
- `[RESOLVED] OQ-03 -- Can Scatter consume RNG before mission 0xF? -> Yes if pre-RNG gates pass; directional branch calls scenario `RandomRanged(0,4)` before `SellBuilding` queues `0xF`.` (evidence: `0x0051D2AC..0x0051D2BA`, `0x00458132..0x00458138`)
- `[RESOLVED] OQ-04 -- Does DAT_007EAF7C affect ejected occupants? -> Yes conditionally; for sequence/state not `-1` or `0x1F`, zero byte at `DAT_007EAF7C + index*4` returns before RNG.` (evidence: `0x0051D1AA..0x0051D1C3`)
- `[RESOLVED] OQ-05 -- What mission does Scatter queue? -> Mission `2` with second arg `0`, before destination set.` (evidence: `0x0051D6BE..0x0051D6E0`)
- `[RESOLVED] OQ-06 -- Does later mission 0xF still occur after Scatter queued mission 2? -> Yes on the `SellBuilding` success block; `0x00458132..0x00458138` runs after Scatter returns.` (evidence: `0x00458110..0x00458138`)
- `[RESOLVED] OQ-07 -- Does failed Unlimbo consume scatter RNG or queue 0xF? -> No; failure branches to occupant vtable `+0xF8` before direct Scatter.` (evidence: `0x004580BD..0x004580C5`)
- `[RESOLVED] OQ-08 -- What is the RNG primitive? -> `Random::RandomRanged` on `g_Scenario+0x218`, inclusive bounds, not raw modulo.` (evidence: `0x0051D2AC..0x0051D2BA`, `0x0065C7E0` decompile)
- `[RESOLVED] OQ-09 -- Is there Guard mission setup in this handoff? -> No; `vtable+0x3C8(0)` is archive-target clear, optional `FUN_006EA870` is parent passenger cleanup, and the only `SellBuilding` mission setter after direct scatter is `+0x1E8(0xF,0)`.` (evidence: `0x004580E9..0x00458138`; `0x006EA870` decompile; vtable docs)
- `[RESOLVED] OQ-10 -- What is the current Rust mismatch? -> `production_sell.rs` uses `next_u32() % 8` and `issue_direct_move`, so order, RNG primitive, and mission semantics drift.` (evidence: `src/sim/production/production_sell.rs:313..335`)
- `[DEFERRED] OQ-11 -- Exact foundation-edge scan coordinate order?` (category: `out-of-scope`; reason: assigned to slot 1; next-step-if-pursued: use that slot's report)
- `[DEFERRED] OQ-12 -- Full mission queue field writes for vtable `+0x1E8`?` (category: `requires-different-system-context`; reason: this slot proves call order, not MissionClass internals; next-step-if-pursued: mission queue lifecycle investigation)
- `[DEFERRED] OQ-13 -- Full semantic contents of `DAT_007EAF7C`?` (category: `bounded-cost-too-high`; reason: only the gate's effect on ejected occupants is needed here; next-step-if-pursued: standalone infantry sequence scatter-table report)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Successful ejection calls direct Scatter before later mission `0xF`; direct Scatter may queue mission `2` and destination first. | `0x004580F9..0x00458138`; `0x0051D6BE..0x0051D6E0` | mismatch: direct move replaces mission ordering | `src/sim/production/production_sell.rs::eject_garrison_passengers_at_edges`; future infantry scatter/mission surface | Preserve order: unlimbo/place -> Scatter(building center, true, true) -> if destination found queue mission `2` and set destination -> then queue mission `0xF`. | Eject one infantry from a garrison with an open adjacent scatter destination and assert Scatter's mission/destination effect is recorded before the later `0xF` queue. Proposed test: `garrison_ejection_scatter_queues_move_before_0f` | Do not collapse the path to only direct movement or only mission `0xF`. |
| Directional ejected-infantry Scatter uses scenario `RandomRanged(0,4)` jitter, not raw `% 8`. | `0x0051D2AC..0x0051D2BA`; `0x0065C7E0` decompile | mismatch: `sim.rng.next_u32() % 8` at `production_sell.rs:321` | `src/sim/production/production_sell.rs`; `src/sim/rng.rs`; future `sim::movement::scatter` equivalent | Consume RNG only inside Scatter after pre-RNG gates, using inclusive `RandomRanged(0,4)` from scenario RNG and `-2` jitter around computed threat direction. | Fixed RNG seed, same current/target coords: assert one inclusive `0..4` draw and resulting start direction, with no `%8` draw. Proposed test: `garrison_ejection_scatter_uses_random_ranged_0_4_after_gates` | Do not replace `% 8` with `RandomRanged(0,7)`; both bounds and timing are wrong. |
| Scatter pre-RNG gates can return before RNG; DAT_007EAF7C zero returns before RNG even for ejected occupants in non-exempt sequence states. | `0x0051D17B..0x0051D1C3`, `0x0051D212..0x0051D220` | missing/unchecked: Rust always draws after placement | future infantry state/sequence surface plus `production_sell.rs` caller | Make scatter RNG conditional on the same pre-RNG gates; if Scatter returns before RNG, `SellBuilding` still later queues `0xF` if the ejection success flag permits. | Put an ejected infantry into a gated sequence state and assert no scatter RNG draw occurs while mission `0xF` still queues. Proposed test: `garrison_ejection_scatter_table_gate_skips_rng_but_queues_0f` | Do not make ejection placement itself consume scatter RNG. |

## 11. Negative Facts / Do Not Do

- Do not keep `next_u32() % 8` for ejected infantry scatter. Evidence: `0x0051D2AC..0x0051D2BA` calls `RandomRanged(0,4)` on `g_Scenario+0x218`. Active in YR: Yes.
- Do not state that `SellBuilding` merely queues Scatter mission `0xF` with no pre-queue RNG. Evidence: `0x0045810A` direct scatter call precedes `0x00458138` mission `0xF`; for infantry, Scatter can consume RNG. Active in YR: Yes.
- Do not model successful ejection as immediate direct move only. Evidence: Scatter queues mission `2` at `0x0051D6C4` and sets destination at `0x0051D6E0`, then `SellBuilding` queues `0xF` later. Active in YR: Yes.
- Do not insert a Guard `SetMission` between `Unlimbo` and Scatter. Evidence: the observed `+0x3C8(0)` call is archive-target clearing, optional `FUN_006EA870` is parent cleanup, and `+0x1E8(0xF,0)` is the only post-scatter mission setter in `SellBuilding`. Active in YR: No for this slice.
- Do not consume scatter RNG on failed `Unlimbo`. Evidence: failed branch exits toward vtable `+0xF8` before `0x0045810A`. Active in YR: Yes.
- Do not let the DAT_007EAF7C table fall out of the scatter implementation. Evidence: `0x0051D1AA..0x0051D1C3` returns before RNG for table-zero states, including ejected occupants. Active in YR: Conditional.

## 12. Remaining Uncertainty

- Exact semantic names and full contents of `DAT_007EAF7C` remain out-of-scope; only its branch effect is verified.
- Exact mission queue field writes behind vtable `+0x1E8` are out-of-scope; this report verifies call order and mission IDs.
- Exact exit-cell scan coordinate order is intentionally left to slot 1.

## 13. Stale Docs / Follow-Up Wording

- `docs/research/PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`: replace "then queues each successful occupant into Scatter mission `0xF`" with "then directly calls the occupant scatter virtual; for infantry this can consume `RandomRanged(0,4)` and queue mission `2` before `SellBuilding` later queues mission `0xF`."
- `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`: replace "Successful ejection queues Guard and then queues Scatter mission `0xF`" with "Successful ejection clears archive target via occupant vtable `+0x3C8(0)`, calls the occupant scatter virtual `+0x174` with the building-center coordinate and true flags, optionally runs `FUN_006EA870` parent-passenger cleanup if `occupant+0x5D4` is non-null, then queues mission `0xF`; for infantry, direct Scatter may consume `RandomRanged(0,4)`, queue mission `2`, and set a destination first."

## Sources

- Fresh Ghidra decompile: `0x00457DE0`, `0x0051D0D0`, `0x0065C7E0`, `0x00447AC0`, `0x00481670`, `0x006EA870`.
- Fresh Ghidra assembly context: `0x004580BD..0x00458138`, `0x00447B11`, `0x0051D0D0..0x0051D220`, `0x0051D2AC..0x0051D2D0`, `0x0051D6BE..0x0051D6ED`, `0x0065C7E0..0x0065C889`.
- Existing research referenced: `GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`, `GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`, `GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`, `GARRISON_SYSTEM_MODEL_SYNTHESIS.md`, `PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini:900`, `ini/rulesmd.ini:3164`, `ini/rulesmd.ini` `CanBeOccupied=yes` civilian structure entries.
- Rust scan: `src/sim/production/production_sell.rs:243..342`.
