# Passenger / Garrison Ejection RNG Classification -- Ghidra Research Report

**Address(es):** `0x00457DE0` (`BuildingClass::SellBuilding`), `0x00449C30` (`BuildingClass::Sell`), `0x00442230` (`BuildingClass::ReceiveDamage`), `0x0051D0D0` (`InfantryClass::Scatter`), `0x00743A50` (`UnitClass::Scatter`, prior doc)
**Investigation Mode:** exhaustive-slice for current Rust raw RNG uses in passenger/garrison ejection; coverage-map for generic transport unload.
**Claimed Scope:** classify `src/sim/passenger.rs` and `src/sim/production/production_sell.rs` raw `next_u32() % 8` and Fisher-Yates `% (i+1)` ejection usages against verified YR paths.
**Non-Scope:** full generic transport radio/mission unload state machine, all Scatter callers, survivor infantry spawn RNG outside the garrison-occupant paths, and live runtime observation.
**Confidence:** High for garrison sell/destruction ejection and Infantry scatter RNG primitive; Medium for generic passenger unload.
**Active in YR:** Yes. `CanBeOccupied=yes` garrisons, sell mission, and building destruction case 4 call these paths in standard YR.

## 0. Target Question / Non-Goals / Completion Gate

**Target question:** Do sell/destruction passenger/garrison ejection and scatter directions use raw `Random::Next` modulo, `RandomRanged`, or no ejection-time RNG in gamemd/YR?

**Non-goals:** Do not implement Rust. Do not audit every transport mission. Do not classify unrelated survivor/crew spawn lists beyond noting conflicts with current files.

**Evidence needed to mark COMPLETE:** decompile plus assembly/caller evidence for the material garrison ejection path; explicit Rust delta for each raw modulo site in the scoped files; final tests names.

**Stop conditions:** mark PARTIAL if generic passenger unload cannot be tied to an exact binary unload function in this slice.

## 1. Overview

The current Rust raw modulo draws in the garrison ejection paths do not match the verified YR garrison ejection path. Gamemd `BuildingClass::SellBuilding` does not shuffle foundation cells and does not pick an immediate `0..7` direction with raw RNG. It finds one exit coordinate by deterministic foundation-edge scans, unlimbos occupants in reverse occupant-vector order, then directly calls the occupant Scatter virtual. For infantry, that Scatter call can consume `RandomRanged(0,4)`, queue mission `2`, and set a destination before returning to `SellBuilding`.

The actual randomization for the infantry's later scatter is in `InfantryClass::Scatter`, which uses scenario `RandomRanged(0,4)` and applies a `-2` offset to the computed direction. That is not raw `Random::Next % 8`.

## 2. Key Offsets / Data

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass+0x688` | garrison occupant pointer array | `0x00457DE0` decompile, load at loop start | Yes |
| `BuildingClass+0x694` | occupant count | `0x00457DE0`, `0x004581F0` prior docs | Yes |
| `BuildingClass+0x69C` | garrison fire index, reset before ejection | `0x00457DE0` first write | Yes |
| `g_MapEditorMode` | temporary unlimbo side-effect guard | increment before occupant loop, decrement after | Yes |
| `g_Scenario+0x218` | scenario RNG state used by `RandomRanged` | Infantry scatter assembly at `0x0051D2AC`, `0x0051D36D` | Yes |
| `g_DirectionOffsets` | 8 compass offsets | Scatter docs + `InfantryClass::Scatter` decompile | Yes |

## 3. Core Logic

### 3.1 `BuildingClass::SellBuilding @ 0x00457DE0`

Verified facts:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---|---|
| Resets `Building+0x69C` to zero before doing anything else. | decompile first statement in `0x00457DE0` | High | Yes |
| If occupant count is zero, returns without placement or RNG. | `GetOccupantCount` result gate at function start | High | Yes |
| Exit coordinate search is deterministic. It scans foundation edges with `MapClass::Get_CellClass` and occupant `Can_Enter_Cell` vtable `+0x1AC`; no `RandomRanged` or raw RNG call appears in this helper. | decompile `0x00457E35..0x00458060`; assembly loop spot-check around `0x00458060`, `0x00458098` | High | Yes |
| If all edge searches fail before a candidate is found, no-exit behavior depends on the caller's second argument: destruction/red-HP callers pass zero and take `SpawnUnitsWithParachute(0)`'s null remove branch; normal player sell passes nonzero and uses an inside-foundation fallback coordinate. | `GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md` caller/branch audit | High | Yes |
| Occupants iterate from `count - 1` down to `0` (LIFO/high-to-low). | assembly: `00458098 MOV EBP,[ESI+0x694]`, `0045809E DEC EBP`, `004580A3 JS`, `004580B1 MOV EDI,[EAX+EBP*4]` | High | Yes |
| For each occupant, `Unlimbo(coord, 0)` is attempted. If it fails, vtable `+0xF8` is called on the occupant. | assembly: `004580BD CALL [EDX+0xD8]`, `004580C3 TEST AL,AL`; decompile failure branch calls `+0xF8` | High | Yes |
| Successful occupants call direct Scatter based on the building coord after successful unlimbo. The later `+0x1E8(0xF,0)` mission block is first-argument gated and is not active for the direct callers checked by the 2026-05-27 parent spot-check. | assembly: `00458104 CALL [EAX+0x48]`, `0045810A CALL [EBX+0x174]`, gate at `00458110..00458116`; caller pushes in `GARRISON_SELLBUILDING_EXIT_CELL_SCAN_ORDER_GHIDRA_REPORT.md` | High for Scatter; partial for any `0xF` liveness beyond checked direct callers | Yes / Conditional |

Negative finding: no Fisher-Yates, no per-occupant raw `% 8`, and no `RandomRanged` call inside `SellBuilding`.

### 3.2 Callers / Liveness

| Path | Finding | Evidence | Active in YR |
|---|---|---|---|
| Building sell mission | State 1 calls `BuildingClass::SellBuilding(param_1)` when `GetOccupantCount() > 0`. | `0x00449C30` decompile, state 1 after passenger/crew handling | Yes |
| Building destruction | Damage result case 4 calls `SellBuilding` if `Type+0x157B CanBeOccupied` is true. | `0x00442230` decompile case 4 | Yes |
| Ordinary UnitAbsorb/InfantryAbsorb passengers during sell | Mission sell uses `FUN_00473430` and `RateTimer::Current` facing for this separate passenger list, not the `+0x688/+0x694` garrison vector. | `0x00449C30` decompile | Conditional; not the scoped Rust garrison vector path |
| Crew survivor spawn on sell | Uses `RandomRanged(0, exit_count - 1)` for survivor exit cell selection. | `0x00449C30` decompile around survivor loop | Yes, but separate from garrison occupant ejection |

### 3.3 Scatter RNG after ejection

| Finding | Evidence | Confidence | Active in YR |
|---|---|---|---|
| SellBuilding queues Scatter mission `0xF`; immediate ejection helper does not choose the final scatter cell itself. | `0x00458130..0x00458138` assembly and decompile | High | Yes |
| Infantry directional scatter uses scenario RNG through `RandomRanged(0,4)`, not raw next modulo. | assembly: `0051D2AC MOV EAX,[0x00A8B230]`, `0051D2B1 PUSH 0x4`, `0051D2B3 PUSH EBP` (EBP is zero), `0051D2BA CALL 0x0065C7E0`; also `0051D36D MOV EDX,[0x00A8B230]`, `0051D373 PUSH 0x4`, `0051D379 PUSH EBP`, `0051D385 CALL 0x0065C7E0` | High | Yes |
| The random result is applied as `roll - 2` to a computed 8-way direction, producing jitter `-2..+2`, then wrapped by `& 7` during candidate search. | `0x0051D2CC` and `0x0051D38C` decompile/assembly; loop uses `& 7` | High | Yes |
| Unit scatter, for non-infantry contrast, uses `RandomRanged(0,2)` jitter in the prior verified UnitClass scatter report, not raw `% 8`. | `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md` | Medium for this report; prior doc confidence high | Yes |

## 4. INI Keys

| Key | Role | Evidence | Active in YR |
|---|---|---|---|
| `CanBeOccupied` | gates destruction ejection call from `ReceiveDamage` case 4 and garrison eligibility | `0x00442230`, `rulesmd.ini` many civilian buildings | Yes |
| `MaxNumberOccupants` | bounds occupant vector capacity/count; not RNG | garrison docs and INI | Yes |
| `Occupier` | infantry can enter garrison; not ejection RNG | garrison docs and INI | Yes |
| `Passengers`, `SizeLimit` | generic transport cargo; generic unload RNG not completed in this slice | Rust scan and INI | Conditional |
| survivor divisor keys | crew survivor count; separate from garrison occupant RNG | sell report | Yes, but out of scope for raw modulo classification |

## 5. Current Rust Implementation Status

| Rust surface | Current behavior | Classification | Evidence |
|---|---|---|---|
| `src/sim/passenger.rs:563` | manual immediate scatter `next_u32() % 8` after normal unload | YELLOW/RED: unsupported by scoped binary evidence; if modeling gamemd Scatter, it should not be raw `% 8` | Rust scan; no exact generic unload proof in this slice |
| `src/sim/production/production_sell.rs:340` | sell garrison occupants get immediate `next_u32() % 8` direction | RED | `SellBuilding @ 0x00457DE0` queues Scatter; Infantry scatter uses `RandomRanged(0,4)` |
| `src/sim/production/production_sell.rs:418` | destruction garrison Fisher-Yates shuffles foundation cells with raw `% (i+1)` | RED | gamemd destruction for `CanBeOccupied` calls the same `SellBuilding`; no shuffle |
| `src/sim/production/production_sell.rs:488` | destruction garrison occupants get immediate `next_u32() % 8` direction | RED | same as above |
| `src/sim/production/production_sell.rs` sell/destruction placement | Rust sell uses perimeter positions; destruction uses shuffled foundation interior | RED for destruction placement, partial for sell placement details | `0x00457DE0` deterministic edge search and common sell/destruction helper |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::SellBuilding` ejection RNG | verified | decompile `0x00457DE0`; assembly around `0x00458098..0x00458138` | none |
| Sell mission caller | verified | `0x00449C30` state 1 decompile | none for garrison vector |
| Destruction caller | verified | `0x00442230` case 4 decompile | none for `CanBeOccupied` destruction |
| Infantry scatter RNG primitive | verified | `0x0051D2AC..0x0051D2BA`, `0x0051D36D..0x0051D385` | full Infantry scatter report could refine non-ejection branches |
| Unit scatter contrast | touched-not-exhausted | `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md` | not re-decompiled in this slot |
| Generic transport unload in `passenger.rs` | touched-not-exhausted | Rust scan; generic `ExitTransport @ 0x0070FE50` has no useful ejection RNG | find exact radio/mission unload path |
| Crew/survivor spawn RNG | deferred | `0x00449C30` shows `RandomRanged(0, exit_count-1)` | separate survivor spawn classification |

## 7. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Does `SellBuilding` use raw `Random::Next % 8` for occupant scatter? -> No; no raw RNG call appears in the ejection helper. Successful ejection calls the occupant Scatter virtual; for infantry, Scatter may later consume `RandomRanged(0,4)`.` (evidence: `0x00457DE0`, `0x0045810A`, `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-02 -- Does destruction garrison ejection use a separate foundation shuffle? -> No for `CanBeOccupied`; destruction case 4 calls the same `SellBuilding` helper.` (evidence: `0x00442230`)
- `[RESOLVED] OQ-03 -- Is occupant order randomized? -> No; high-to-low index iteration from `count - 1` down to zero.` (evidence: `0x00458098..0x004580B1`)
- `[RESOLVED] OQ-04 -- Which RNG does the later infantry scatter use? -> Scenario `RandomRanged`, not raw modulo.` (evidence: `0x0051D2AC..0x0051D2BA`, `0x0051D36D..0x0051D385`)
- `[RESOLVED] OQ-05 -- What are Infantry scatter bounds? -> `RandomRanged(0,4)` then subtract 2 for five-direction jitter.` (evidence: same assembly plus decompile)
- `[RESOLVED] OQ-06 -- Is `RandomRanged(0,4)` active in YR? -> Yes, `InfantryClass::Scatter` is the active vtable `+0x174` for ejected garrison infantry.` (evidence: scatter docs, garrison docs, `0x0051D0D0`)
- `[RESOLVED] OQ-07 -- What happens if the chosen SellBuilding exit is blocked at unlimbo? -> Occupant `+0xF8` is called, effectively removing/destroying it.` (evidence: `0x004580BD..0x004580C3` plus decompile)
- `[RESOLVED] OQ-08 -- Is there a parachute fallback? -> Yes only if no exit cell can be found before the occupant loop; then `SpawnUnitsWithParachute(0)` and return.` (evidence: `0x00457DE0` decompile)
- `[DEFERRED] OQ-09 -- Which exact gamemd function corresponds to Rust `tick_unloading` manual one-pass unload?` (category: `requires-different-system-context`; reason: generic transport mission/radio unload is broader than the garrison ejection slice; next-step-if-pursued: investigate Unit/Infantry Mission_Enter and transport radio unload.)
- `[DEFERRED] OQ-10 -- Full `InfantryClass::Scatter` branch audit including all early-outs?` (category: `out-of-scope`; reason: enough evidence exists for RNG primitive classification; next-step-if-pursued: standalone Infantry scatter exhaustive-slice.)
- `[DEFERRED] OQ-11 -- Crew survivor spawn parity in current `eject_sell_survivors`?` (category: `out-of-scope`; reason: not a passenger/garrison occupant path; next-step-if-pursued: survivor spawn RNG classification.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Garrison sell/destruction ejection does not draw immediate `% 8` RNG; after successful unlimbo it calls direct Scatter, and Scatter may draw `RandomRanged(0,4)` after its own gates. | `0x00457DE0`, `0x0045810A`, `0x0051D2AC..0x0051D2BA`, `GARRISON_EJECTED_INFANTRY_SCATTER_ORDERING_GHIDRA_REPORT.md` | mismatch | `production_sell.rs::eject_garrison_occupants`, `production_sell.rs::eject_destruction_garrison` | Remove immediate raw direction draw from garrison ejection; model the verified Scatter handoff. Treat any later `0xF` queue as argument-gated until another live nonzero-first-argument caller is proven. | `test_sell_garrison_ejection_does_not_consume_rng_before_scatter` | Do not replace `% 8` with `RandomRanged(0,7)`; that is still the wrong call/order. |
| Destruction of `CanBeOccupied` garrisons calls the same `SellBuilding` ejection helper as sell. | `0x00442230` case 4 | mismatch: Rust destruction shuffles foundation interiors | `production_sell.rs::eject_destruction_garrison` | Use the same placement/ejection contract as sell unless another YR path is proven. | `test_destroyed_garrison_uses_sellbuilding_ejection_order` | Do not keep Fisher-Yates raw `% (i+1)`; no matching binary behavior found. |
| Occupants eject LIFO/high-to-low from `+0x688`, `+0x694`. | `0x00458098..0x004580B1` | Rust mostly matches LIFO in production sell/destruction | both garrison helpers | Preserve reverse occupant order. | `test_garrison_ejection_lifo_order_preserved_without_shuffle` | Do not sort, shuffle, or FIFO occupants. |
| Later infantry scatter uses scenario `RandomRanged(0,4)` jitter, not raw next modulo. | `0x0051D2AC..0x0051D2BA`, `0x0051D36D..0x0051D385` | mismatch where Rust approximates immediate adjacent move | future `sim::movement::scatter` / passenger/garrison callers | Route ejected infantry through a parity Scatter operation using inclusive `RandomRanged(0,4)` and computed threat direction. | `test_ejected_infantry_scatter_uses_random_ranged_0_4_jitter` | Do not use raw `next_u32() % 8` for Scatter start direction. |
| Generic unload raw `% 8` in `passenger.rs` is not verified by this slice. | Rust scan; incomplete binary mapping | unchecked/high risk | `passenger.rs::tick_unloading` | Leave for a targeted generic transport unload investigation before patching broadly. | `test_transport_unload_rng_contract_pending_binary_verification` | Do not assume garrison sell evidence covers all vehicle transport unloads. |

## Negative Facts / Do Not Do

- Do not implement garrison ejection as Fisher-Yates over foundation cells.
- Do not convert raw `% 8` to `RandomRanged(0,7)` and call it done; gamemd queues Scatter and Scatter uses a computed direction plus jitter.
- Do not consume RNG when `SellBuilding` only finds an exit coordinate and unlimbos.
- Do not conflate crew survivors with garrison occupants; sell survivors have their own `RandomRanged(0, exit_count - 1)` path.

## Sources

- Ghidra decompilation: `0x00457DE0`, `0x00449C30`, `0x00442230`, `0x0051D0D0`, `0x0070FE50`.
- Ghidra assembly context: `0x00458060`, `0x00458098..0x00458138`, `0x0051D2AC..0x0051D2BA`, `0x0051D36D..0x0051D385`.
- Prior docs: `GARRISON_SYSTEM_GHIDRA_REPORT.md`, `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`, `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`, `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`, `SCATTER_ALL_CALLERS_GHIDRA_REPORT.md`, `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/passenger.rs`, `src/sim/production/production_sell.rs`.
- INI scan: `ini/rulesmd.ini`, `ini/rules.ini`.
