# Garrison SellBuilding Exit Cell Scan Order -- Ghidra Research Report

**Address(es):** `BuildingClass__SellBuilding @ 0x00457DE0`, `SpawnUnitsWithParachute @ 0x004585C0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact `0x00457E35..0x00458060` exit-cell scan order, accepted-cell predicate, no-exit branch trigger, and reuse of the chosen coordinate for reverse garrison occupant ejection.
**Non-Scope:** Infantry scatter internals after the successful handoff, generic transport unload, sell refund/removal, owner reconciliation, bunker occupants, and non-null `SpawnUnitsWithParachute` callers.
**Confidence:** High for scan order, predicate, coordinate reuse, caller argument split, and no-exit branch conditions.
**Active in YR:** Yes. The function is reached by active standard-YR garrison sell/destruction/red-HP paths for `CanBeOccupied` buildings; standard INI data contains many `CanBeOccupied=yes` civilian structures.

## 0. Target Question / Non-Goals / Completion Gate

**Target question:** What exact cell order does `BuildingClass__SellBuilding @ 0x00457DE0` scan when choosing the ground exit coordinate for `CanBeOccupied` garrison occupants, what predicate accepts a cell, when does the no-exit fallback fire, and how is the chosen coordinate reused for reverse occupant iteration?

**Non-goals:** Do not redo `CanDock`/`CanGarrison` entry gates, ownership transfer timing, captured-civilian sell outcome, normal survivor spawning, infantry scatter internals, or parachute visuals. Do not implement Rust.

**Evidence needed to mark COMPLETE:**

1. Live Ghidra decompile of `BuildingClass__SellBuilding`.
2. Address-level disassembly for `0x00457E35..0x00458060` and adjacent fallback/ejection branches.
3. Caller evidence for active YR sell/destruction/red-HP paths and their two pushed arguments.
4. Current Rust surface scan for the affected helper and tests.
5. Final open-question log with no open in-scope items.

**Stop conditions:** Stop after the scan order, predicate, fallback split, and coordinate reuse are proven. Do not follow `InfantryClass::Scatter` beyond naming the successful `Unlimbo` handoff calls.

## 1. Overview

`SellBuilding` does not choose a fresh exit for each occupant. Active in YR: Yes. It first chooses one candidate cell by deterministic perimeter scans using the first garrison occupant's `Can_Enter_Cell`-style virtual, converts that one cell to a coordinate, then attempts to `Unlimbo` every occupant at the same coordinate in reverse vector order.

The scan is not Rust's current distance-sorted rectangular perimeter. Active in YR: Yes. It is four ordered edge scans with inclusive/off-by-one corner behavior: right edge downward/upward from the southeast outside corner, bottom edge westward from the same corner, top edge eastward from just east of the northwest corner, then left edge southward from just south of the northwest corner.

## 2. Class Layout / Key Offsets

| Offset / field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass+0x520` | `BuildingTypeClass*`; source for foundation width/height helpers | decompile and assembly `0x00457E26`, `0x00457E35` | Yes |
| `BuildingClass+0x684` | garrison vector object; cleanup/reset after ejection/fallback | decompile; assembly `0x004581A4..0x004581BC`; prior no-exit report | Yes |
| `BuildingClass+0x688` | garrison occupant pointer array | decompile; assembly `0x00457E61`, `0x004580A9..0x004580B1` | Yes |
| `BuildingClass+0x694` | garrison occupant count | assembly `0x00458098`; `SpawnUnitsWithParachute` decompile | Yes |
| `BuildingClass+0x69C` | current garrison fire index, reset at function entry | assembly `0x00457DEB`; decompile first write | Yes |
| vtable `+0x1B8` | building map-cell origin accessor used for scan bounds | repeated calls in `0x00457E52`, `0x00457E9D`, `0x00457EF1`, `0x00457F6D`, `0x00457FE4`, `0x0045800B` | Yes |
| first occupant vtable `+0x1AC` | accepted-cell predicate for scan candidates | assembly `0x00457E91..0x00457E99` and repeated loop calls | Yes |
| occupant vtable `+0xD8` | `Unlimbo(coord, 0)` attempt at chosen coordinate | assembly `0x004580B4..0x004580C3` | Yes |
| occupant vtable `+0xF8` | destroy/remove on failed `Unlimbo` or no-exit null fallback | assembly `0x00458193..0x00458197`; `SpawnUnitsWithParachute` decompile | Yes |

## 3. Core Logic

### 3.1 Entry and liveness

`SellBuilding` first resets `Building+0x69C` to zero, then calls vtable `+0x408` for occupant count. Active in YR: Yes. Assembly: `0x00457DEB MOV [ESI+0x69C],0`; `0x00457DF5 CALL [EAX+0x408]`; `0x00457DFB TEST EAX,EAX`; zero count jumps to return.

Direct active callers found in the binary:

| Caller | Arguments pushed before call | Meaning for this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingClass__ReceiveDamage` case 4, `CanBeOccupied` gate | `push 0; push 0` | no-exit calls `SpawnUnitsWithParachute(0)` | `0x0044262B..0x0044263B`; decompile case 4 checks `Type+0x157B` then calls helper | Yes |
| `BuildingClass__Sell` state 1 | `push 1; push 0` | no accepted scan cell uses inside-foundation fallback instead of null helper | `0x0044A5BA..0x0044A5CA`; decompile state 1 occupant-count gate | Yes |
| `BuildingClass__CheckAutoSellOrCivilian` red-HP path | `push 0; push 0` | no-exit calls `SpawnUnitsWithParachute(0)` | `0x00458218..0x00458229`; decompile red-HP call | Yes |
| additional direct caller at `0x0044D89C` | `push 0; push 0` | same no-exit behavior; owner function not needed for this slice | raw disassembly scan around `0x0044D892..0x0044D89C` | Touched, likely active; caller identity deferred |

### 3.2 Accepted-cell predicate

The scan uses the first occupant in `Building+0x688` for all candidate tests. Active in YR: Yes. Assembly `0x00457E61 MOV EDX,[ESI+0x688]`; `0x00457E75 MOV EDI,[EDX]`; `0x00457E7B MOV EBP,[EDI]`.

For every candidate, `SellBuilding` calls `MapClass__Get_CellClass(candidate)` then first occupant vtable `+0x1AC` with pushed arguments `candidate CellClass*`, `-1`, `-1`, `0`, `1`. Active in YR: Yes. Assembly pattern for the first candidate: `0x00457E77 PUSH 1`; `0x00457E79 PUSH 0`; `0x00457E7D PUSH -1`; `0x00457E83 PUSH -1`; `0x00457E86 MOV ECX,0x87F7E8`; `0x00457E8B CALL 0x5657A0`; `0x00457E93 CALL [EBP+0x1AC]`.

The accepted result is zero. Active in YR: Yes. Every loop tests the return and accepts only on `EAX == 0`: first candidate `0x00457E99 TEST EAX,EAX`; `0x00457E9B JE 0x00457EE5`; blocked/nonzero continues scanning.

### 3.3 Exact scan order

Definitions used below:

- `O = (ox, oy)` is the building origin cell returned by vtable `+0x1B8`.
- `W = BuildingTypeClass__GetFoundationWidth()`.
- `H = BuildingTypeClass__GetFoundationHeight(0)`.
- Coordinates are signed 16-bit cell x/y packed in a 32-bit cell struct.
- `CanEnter(cell)` means first occupant `vtable+0x1AC(MapClass::Get_CellClass(cell), -1, -1, 0, 1) == 0`.

Scan order and bounds:

| Order | Edge name | Candidate sequence | Inclusive behavior | Evidence | Active in YR |
|---:|---|---|---|---|---|
| 1 | right/east column | `(ox+W, oy+H)`, then decrement `y` until `(ox+W, oy-1)` | includes southeast and northeast outside corners | init/test `0x00457E5F..0x00457E99`; decrement loop `0x00457E9D..0x00457EE3`; stop compare `candidate_y <= oy-1` at `0x00457EAC..0x00457EB8` | Yes |
| 2 | bottom/south row | `(ox+W, oy+H)`, then decrement `x` until `(ox-1, oy+H)` | re-tests southeast corner; includes southwest outside corner | rebuild start `0x00457EEF..0x00457F18`; decrement loop `0x00457F1C..0x00457F61`; stop compare `candidate_x <= ox-1` at `0x00457F2B..0x00457F36` | Yes |
| 3 | top/north row | first tested `(ox, oy-1)`, then increment `x` through `(ox+W, oy-1)` | skips northwest outside corner; re-tests northeast outside corner | start at `(ox-1,oy-1)` `0x00457F6D..0x00457F87`; pre-test increment `0x00457FAF`; stop compare `candidate_x >= ox+W` at `0x00457F9F..0x00457FAD` | Yes |
| 4 | left/west column | first tested `(ox-1, oy)`, then increment `y` through `(ox-1, oy+H)` | skips northwest outside corner; re-tests southwest outside corner | start at `(ox-1,oy-1)` `0x00457FE4..0x00458003`; pre-test increment `0x0045802D`; stop compare `candidate_y >= oy+H` at `0x0045801A..0x00458027` | Yes |

Material tiny details:

- The southeast, northeast, and southwest outside corners can be tested twice if blocked. Active in YR: Yes. Evidence: scan 1/2 share `(ox+W,oy+H)`, scan 1/3 share `(ox+W,oy-1)`, scan 2/4 share `(ox-1,oy+H)`.
- The northwest outside corner `(ox-1,oy-1)` is not passed to `CanEnter` in the four loops. Active in YR: Yes. Evidence: scans 3 and 4 initialize to `(ox-1,oy-1)` but increment x/y before the `MapClass__Get_CellClass` call.
- There is no randomization, sorting, distance key, per-occupant retry, or occupancy collection in this scan. Active in YR: Yes. Evidence: `0x00457E35..0x00458060` contains only deterministic loop arithmetic, `MapClass__Get_CellClass`, and vtable `+0x1AC`.
- Bounds are not clamped in `SellBuilding`; off-map/invalid candidates are still converted through `MapClass__Get_CellClass` and rejected/accepted by the downstream predicate. Active in YR: Yes. Evidence: no explicit map-bound compare exists in `0x00457E35..0x00458060`.

### 3.4 No-exit branch and sell-specific inside fallback

After the fourth scan exhausts without an accepted candidate, `0x00458027 JGE 0x00458140` enters the no-candidate branch. Active in YR: Yes.

At `0x00458140`, the second stack argument controls what happens next. Active in YR: Yes.

- If the second argument is zero, the helper calls `SpawnUnitsWithParachute(0)` and returns: `0x00458140 MOV AL,[ESP+0x44]`; `0x00458144 TEST AL,AL`; `0x00458146 JE 0x00458180`; `0x00458180 PUSH 0`; `0x00458184 CALL 0x004585C0`; `0x00458190 RET 8`.
- Destruction case 4 and red-HP reconciliation pass this zero argument (`push 0; push 0`), so their no-accepted-cell fallback kills/removes occupants through the null branch. Active in YR: Yes.
- Player sell state 1 passes a nonzero second argument (`push 1; push 0`), so it bypasses `SpawnUnitsWithParachute(0)` and constructs an inside-foundation coordinate `(ox+W-1, oy+H-1)` before jumping to the normal unlimbo loop. Active in YR: Yes. Assembly `0x00458148..0x0045817B` computes the coordinate and jumps to `0x00458060`.

This corrects older wording that treated `SpawnUnitsWithParachute(0)` as the universal no-exit result for all `SellBuilding` callers. It is universal for the zero-second-argument callers, not for the normal player-sell caller.

### 3.5 Chosen coordinate reuse and occupant order

Once a candidate exists, `SellBuilding` converts it once and stores a coordinate before any occupant loop begins. Active in YR: Yes. Assembly `0x00458060..0x0045808D` calls `MapClass__Get_CellClass(chosen_cell)`, then cell vtable `+0x48`, then stores the returned coordinate in stack slots used by all occupants.

The occupant loop is reverse vector order, `count-1` down to `0`. Active in YR: Yes. Assembly `0x00458098 MOV EBP,[ESI+0x694]`; `0x0045809E DEC EBP`; `0x004580A3 JS cleanup`; `0x004580A9 MOV EAX,[ESI+0x688]`; `0x004580B1 MOV EDI,[EAX+EBP*4]`; loop decrement `0x0045819D DEC EBP`; `0x0045819E JNS 0x004580A9`.

Each occupant attempts `Unlimbo(chosen_coord, 0)` at the same coordinate. Active in YR: Yes. Assembly `0x004580AF PUSH 0`; `0x004580B4 LEA EAX,[ESP+0x28]`; `0x004580BD CALL [EDX+0xD8]`; failure branch `0x004580C3 TEST AL,AL`; `0x004580C5 JE 0x00458193`; `0x00458193..0x00458197 CALL [EAX+0xF8]`.

Successful `Unlimbo` handoff point: active in YR, but scatter internals are out-of-scope. The helper clears two infantry bytes when the type flag at occupant type `+0x1EC` is set, calls occupant vtable `+0x3C8` with `0`, calls occupant vtable `+0x174` with a coordinate derived from the building (`0x004580F5..0x0045810A`), then conditionally queues mission `0xF` at `0x00458130..0x00458138` only when the first stack argument is nonzero. All direct caller sites found in this slice pass first argument `0`, so the unconditional handoff to preserve is the successful `Unlimbo -> +0x3C8(0) -> +0x174(building coord,1,1)` sequence; `+0x1E8(0xF,0)` is not proven active for these direct YR callers.

## 4. INI Keys

| INI key | Default/source | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `CanBeOccupied` | many `rules.ini` civilian buildings set `CanBeOccupied=yes`; `rulesmd.ini` patches base data | destruction case 4 gates `SellBuilding` on `BuildingTypeClass+0x157B` | INI grep; `0x0044262B..0x0044263B` | Yes |
| `MaxNumberOccupants` | commonly `10`, some `3`; base/YR rules data | controls occupant vector capacity, not scan order | INI grep; garrison vector count at `+0x694` | Yes |
| `Foundation` | object type data | `W/H` source through foundation width/height helpers | decompile `0x00457E26..0x00457E48`; current Rust uses `foundation_dimensions` | Yes |

## 5. Integration Points

| Integration point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Player sell state 1 | calls `SellBuilding` after occupant-count check with second arg `1` | `0x0044A5BA..0x0044A5CA`; decompile `BuildingClass__Sell` | Yes |
| Destruction case 4 | calls `SellBuilding` only if `Type+0x157B CanBeOccupied` | `0x0044262B..0x0044263B`; decompile `BuildingClass__ReceiveDamage` | Yes |
| Red-HP reconciliation | calls `SellBuilding` before ownership/empty checks when red HP | `0x00458218..0x00458229`; decompile `CheckAutoSellOrCivilian` | Yes |
| No-exit null fallback | calls `SpawnUnitsWithParachute(0)`, whose null branch reverse-removes occupants, no parachutes | `0x00458180..0x00458190`; `SpawnUnitsWithParachute` decompile | Yes for second-arg-zero callers |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Match status | Evidence |
|---|---|---|---|
| `src/sim/production/production_sell.rs::sell_survivor_positions` | builds all outside perimeter cells, skips inside foundation, then sorts by squared distance to southeast interior corner and `(y,x)` | mismatch for garrison exit scan | local scan line 109 |
| `src/sim/production/production_sell.rs::eject_garrison_passengers_at_edges` | computes exit cells once but chooses a fresh unused cell per occupant, avoids occupied/used cells, then directly issues approximate scatter movement | mismatch for coordinate reuse and scan predicate; scatter remains known mismatch | local scan line 243 |
| `src/sim/production/production_sell.rs::eject_garrison_occupants` | snapshots building cargo, calls shared edge helper, clears cargo/fire index | partial | local scan line 351 |
| `src/sim/production/production_sell.rs::eject_destruction_garrison` | routes destruction through same helper | partial; uses Rust helper's wrong scan/reuse semantics | local scan line 412 |
| test `destroyed_garrison_uses_sell_edge_scan_and_lifo_order` | expects different cells for each reverse occupant via `sell_survivor_positions` | stale/mismatch | local scan line 726 |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__SellBuilding @ 0x00457DE0` entry/count gate | verified | decompile; assembly `0x00457DEB..0x00457DFD` | none |
| scan order `0x00457E35..0x00458060` | verified | decompile plus disassembly listed above | none |
| accepted-cell predicate | verified | assembly `0x00457E77..0x00457E99` and repeated loop sites | exact vtable target name/binding can be separately audited, but call contract is proven |
| no-exit branch split | verified | assembly `0x00458027`, `0x00458140..0x00458190`; caller args | none |
| player-sell inside-foundation fallback | verified | assembly `0x0044A5C4..0x0044A5CA`; `0x00458148..0x0045817B` | runtime screenshot/cell fixture not made |
| destruction/red-HP null fallback | verified | assembly `0x00442635..0x0044263B`, `0x00458223..0x00458229`, `0x00458180..0x00458190`; `SpawnUnitsWithParachute` decompile | none for branch result |
| chosen coordinate reuse for reverse occupants | verified | assembly `0x00458060..0x0045819E` | none |
| first-argument gated `Queue_Mission(0xF)` | verified for direct callers passing zero; prior docs stale | assembly `0x00458110..0x00458138`; caller arg scan | a separate xref inventory could identify indirect/unusual callers, if any |
| unidentified direct caller `0x0044D89C` | touched-not-exhausted | raw disassembly scan shows `push 0; push 0; call 0x457DE0` | identify owner function only if future scope needs it |
| Rust comparison | verified | codegraph and local scan of `production_sell.rs` | implementation not performed |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Is this slice active in YR? -> Yes; active sell, destruction, and red-HP callers reach `SellBuilding`, and standard INI has `CanBeOccupied=yes` buildings.` (evidence: `0x0044A5CA`, `0x0044263B`, `0x00458229`, INI grep)
- `[RESOLVED] OQ-02 -- Does the scan use randomization or distance sorting? -> No; scan is deterministic branch/loop arithmetic with no RNG calls.` (evidence: `0x00457E35..0x00458060`)
- `[RESOLVED] OQ-03 -- What validates a candidate cell? -> `MapClass__Get_CellClass(candidate)` followed by first occupant vtable `+0x1AC` with args `-1,-1,0,1`; return zero accepts.` (evidence: `0x00457E77..0x00457E99`)
- `[RESOLVED] OQ-04 -- What is exact edge order? -> right/east column, bottom/south row, top/north row, left/west column, with duplicate SE/NE/SW corners and skipped NW corner.` (evidence: `0x00457E5F..0x00458058`)
- `[RESOLVED] OQ-05 -- Does it choose a new exit per occupant? -> No; one cell is converted to a coordinate before the occupant loop and reused for every occupant.` (evidence: `0x00458060..0x004580BD`)
- `[RESOLVED] OQ-06 -- What is occupant order? -> Reverse vector order, `count-1` to `0`.` (evidence: `0x00458098..0x0045819E`)
- `[RESOLVED] OQ-07 -- What happens if an individual `Unlimbo` fails? -> That occupant's vtable `+0xF8` is called; loop continues to earlier occupants.` (evidence: `0x004580BD..0x004580C5`, `0x00458193..0x0045819E`)
- `[RESOLVED] OQ-08 -- When does `SpawnUnitsWithParachute(0)` fire? -> Only after all scan loops fail and the second stack argument is zero; destruction/red-HP pass zero, player sell does not.` (evidence: `0x00458140..0x00458190`; caller sites)
- `[RESOLVED] OQ-09 -- Does normal player sell share the null no-exit fallback? -> No; its second argument is nonzero and it uses `(ox+W-1,oy+H-1)` inside-foundation fallback.` (evidence: `0x0044A5C4..0x0044A5CA`; `0x00458148..0x0045817B`)
- `[RESOLVED] OQ-10 -- Where is the successful scatter/movement handoff point? -> After successful `Unlimbo`, helper calls occupant `+0x3C8(0)` and `+0x174(building coord,1,1)`; the later `+0x1E8(0xF,0)` is first-argument gated and not active for direct callers found here.` (evidence: `0x004580E9..0x00458138`; caller arg scan)
- `[RESOLVED] OQ-11 -- Does Rust currently match scan/reuse behavior? -> No; it uses `sell_survivor_positions` distance sort and picks fresh unused cells per occupant.` (evidence: `src/sim/production/production_sell.rs` lines 109, 243)
- `[DEFERRED] OQ-12 -- Exact target name/binding of occupant vtable `+0x1AC`.` (category: out-of-scope; reason: call contract and args are sufficient for this slice; next-step-if-pursued: vtable binding audit for InfantryClass movement entry predicates)
- `[DEFERRED] OQ-13 -- Identify the owner function of direct caller `0x0044D89C`.` (category: out-of-scope; reason: it has same `push 0; push 0` argument contract and is not needed for core garrison sell/destruction/red-HP handoff; next-step-if-pursued: caller-function boundary audit around `0x0044D89C`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `SellBuilding` scans a fixed perimeter order: east column from SE to NE, south row from SE to SW, north row from `(ox,oy-1)` to NE, west row from `(ox-1,oy)` to SW. | decompile plus assembly `0x00457E35..0x00458060` | mismatch: `sell_survivor_positions` sorts by distance/y/x | `src/sim/production/production_sell.rs::sell_survivor_positions` or a garrison-specific replacement | Add a garrison exit scan that preserves exact order, duplicate corner tests, skipped NW, and no RNG/sort. | Proposed test: `garrison_sellbuilding_scan_order_matches_gamemd_edges_2x2` | Do not reuse survivor spawn ordering for garrison ejection. |
| One accepted coordinate is selected before the occupant loop and reused for every reverse occupant. | `0x00458060..0x004580BD`; loop `0x00458098..0x0045819E` | mismatch: Rust selects a fresh unused edge cell per occupant | `eject_garrison_passengers_at_edges`; tests around destruction/sell ejection | Choose one coordinate once; attempt every occupant at that same coordinate in LIFO order; failed individual placement kills/removes that occupant. | Proposed test: `garrison_sellbuilding_reuses_single_exit_coord_for_all_lifo_occupants` | Do not avoid stacking by advancing to the next edge cell; gamemd lets `Unlimbo` decide success/failure. |
| No-exit behavior is caller-argument dependent: destruction/red-HP `push 0; push 0` calls `SpawnUnitsWithParachute(0)` null branch; player sell `push 1; push 0` uses inside-foundation fallback `(ox+W-1,oy+H-1)`. | caller assembly `0x00442635..0x0044263B`, `0x0044A5C4..0x0044A5CA`, `0x00458223..0x00458229`; fallback assembly `0x00458140..0x00458190` | mismatch/unchecked: Rust has one no-free-cell behavior for all callers | `eject_garrison_occupants`, `eject_destruction_garrison`, any future red-HP ejection surface | Model caller mode or pass a sell-vs-destruction flag so player sell and destruction/red-HP diverge only on all-scan-failed fallback. | Proposed tests: `garrison_destruction_no_exit_uses_spawnunitswithparachute_zero_removal`; `garrison_player_sell_no_exit_uses_inside_foundation_fallback` | Do not state `SpawnUnitsWithParachute(0)` is universal for player sell. |
| Successful `Unlimbo` handoff is `+0x3C8(0)` then occupant `+0x174(building coord,1,1)`; `Queue_Mission(0xF)` is first-argument gated and not active for direct callers found here. | assembly `0x004580E9..0x00458138`; caller arg scan | mismatch/stale docs: Rust directly issues approximate neighbor movement after placement | `eject_garrison_passengers_at_edges`; future mission/scatter queue surface | Replace direct `%8` movement with the verified post-unlimbo handoff surface; keep `0xF` queue gated unless another live caller with first arg nonzero is proven. | Proposed test: `garrison_sellbuilding_success_calls_scatter_handoff_without_raw_neighbor_rng` | Do not consume Rust RNG during ejection placement. Do not unconditionally queue mission `0xF` from this helper for the direct YR callers verified here. |

## 10. Negative Facts / Do Not Do

- Do not use distance-sorted perimeter cells for garrison ejection. Active in YR: No. Evidence: deterministic assembly loops `0x00457E35..0x00458060`.
- Do not choose a different free edge cell for each occupant. Active in YR: No. Evidence: one coordinate is produced at `0x00458060..0x0045808D` before reverse occupant loop.
- Do not remove duplicate corner tests or add the northwest outside corner unless a separate predicate wrapper proves exact equivalence. Active in YR: No. Evidence: loop bounds above.
- Do not call `SpawnUnitsWithParachute(0)` for every no-scan player-sell case. Active in YR: No for normal sell; evidence player sell passes nonzero second argument and executes `0x00458148..0x0045817B`.
- Do not unconditionally queue mission `0xF` after successful `Unlimbo` for the direct active callers verified here. Active in YR: No for these direct calls; evidence first argument is zero at direct caller sites and `0x00458110..0x00458138` is argument-gated.
- Do not model the scan predicate as Rust occupancy-only checks. Active in YR: No. Evidence: first occupant virtual `+0x1AC` with `CellClass*`, `-1`, `-1`, `0`, `1`.

## 11. Remaining Uncertainty

No material uncertainty remains for the scoped scan order, accepted-cell predicate call contract, coordinate reuse, or caller-dependent no-exit split.

Deferred, out-of-scope items: exact name/binding of occupant vtable `+0x1AC`; identifying the owner function of direct caller `0x0044D89C`; runtime fixture screenshots for the player-sell inside-foundation fallback; full scatter internals.

## 12. Stale Docs / Follow-up Docs

- Replace wording equivalent to "exit search tries south/west/north/east" with: "exit search scans east/right column, south/bottom row, north/top row, then west/left column, with duplicate SE/NE/SW corner tests and no `CanEnter` test for NW outside corner."
- Replace wording equivalent to "each occupant uses the next foundation-edge cell" with: "one accepted exit cell is selected before the loop and converted to one coordinate reused by all occupants in reverse order."
- Replace wording equivalent to "if all four edges fail, `SpawnUnitsWithParachute(0)` always runs" with: "`SpawnUnitsWithParachute(0)` runs for second-argument-zero callers such as destruction/red-HP; normal player sell passes a nonzero second argument and falls back to `(origin+W-1,origin+H-1)`."
- Replace wording equivalent to "successful ejection always queues Scatter mission `0xF`" with: "successful ejection always reaches the immediate post-`Unlimbo` handoff (`+0x3C8(0)` then `+0x174(building coord,1,1)`); `+0x1E8(0xF,0)` is first-argument gated and was not active for direct callers found in this slice."

## 13. Proposed Rust Test Names

- `garrison_sellbuilding_scan_order_matches_gamemd_edges_2x2`
- `garrison_sellbuilding_reuses_single_exit_coord_for_all_lifo_occupants`
- `garrison_destruction_no_exit_uses_spawnunitswithparachute_zero_removal`
- `garrison_player_sell_no_exit_uses_inside_foundation_fallback`
- `garrison_sellbuilding_success_calls_scatter_handoff_without_raw_neighbor_rng`

## Sources

- Live Ghidra read-only decompile: `BuildingClass__SellBuilding`; `SpawnUnitsWithParachute`; `BuildingClass__Sell`; `BuildingClass__ReceiveDamage`; `BuildingClass__CheckAutoSellOrCivilian`.
- Live/read-only address evidence: `0x00457DEB..0x00457DFD`, `0x00457E35..0x00458060`, `0x00458060..0x0045819E`, `0x00458140..0x00458190`, `0x004585C0` decompile.
- Caller disassembly: `0x0044262B..0x0044263B`, `0x0044A5BA..0x0044A5CA`, `0x0044D892..0x0044D89C`, `0x00458218..0x00458229`.
- Prior docs referenced: `GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`, `GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`, `PASSENGER_GARRISON_EJECTION_RNG_CLASSIFICATION_GHIDRA_REPORT.md`, `GARRISON_SYSTEM_GHIDRA_REPORT.md`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`.
- Rust scanned: `src/sim/production/production_sell.rs`.
