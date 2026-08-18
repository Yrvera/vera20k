# Reduce_Tiberium Full Removal Side Effects and Return Value - Ghidra Research Report

**Address(es):** `0x00480A80` primary, `0x0073D450` standard harvester caller  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `CellClass::Reduce_Tiberium` return value, amount bounds/signedness, `OverlayData` semantics, full/partial removal writes, dirtying, growth/spread queue calls, and active standard YR `CMIN`/`HARV` caller path.  
**Non-Scope:** complete tiberium queue architecture, exact runtime direction-table values, `PlaceTiberium`, selected-unit cargo pips, refinery unload, chrono return movement, and non-harvester damage callers beyond caller identity.  
**Confidence:** High for the claimed slice; Medium for Rust deltas that depend on later architecture ownership.  
**Active in YR:** Yes. Standard YR `CMIN` has `Harvester=yes`, `Storage=20`, `PipScale=Tiberium`, and `Teleporter=yes` in `ini/rulesmd.ini:7351-7396`; `UnitClass__Harvest_Ore_Tick @ 0x0073D450` gates on `UnitType+0xE0E Harvester`, land type 5, and calls `CellClass__Reduce_Tiberium @ 0x00480A80`.

## 0. Working Notes Gate

Target question: What exact behavior does `CellClass::Reduce_Tiberium @ 0x00480A80` expose to standard YR harvester full and partial ore removal?

Non-goals: Do not re-investigate outbound chrono movement, close/far refinery return, stock refinery unload, selected pips, or the full RA2/YR tiberium queue architecture.

Evidence needed to mark COMPLETE: decompile plus assembly context for `Reduce_Tiberium`; decompile plus assembly/caller context for `UnitClass__Harvest_Ore_Tick`; decompile of side-effect callees; INI proof that stock `CMIN` activates the harvester path; current Rust surface scan.

Stop conditions: Stop after the return value, signedness/bounds, write order, side-effect call order, density-11 branch, neighbor reseed loop shape, and standard harvester liveness are proven or explicitly deferred.

## 1. Overview

`CellClass::Reduce_Tiberium` removes ore/gem density levels from the current cell. Partial removal subtracts the requested amount from `CellClass+0x11E OverlayData` and returns the requested amount. Full removal clears `OverlayTypeIndex`, zeroes `OverlayData`, recalculates terrain attributes, marks radar/tactical dirty, clears this cell's spread-bitmap entries for all tiberium types, then reseeds valid neighbors into the removed cell's tiberium type's spread queue.

For a standard empty Allied Chrono Miner harvesting Riparius from `OverlayData=11`, `Harvest_Ore_Tick` requests 20 density levels, `Reduce_Tiberium` takes the full-removal path, and the return value is 11. The miner then adds exactly `11.0` Riparius storage, worth `11 * 25 = 275` credits when deposited.

## 2. Class Layout / Key Offsets

| Offset / Global | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `CellClass+0x44` | `int` | `OverlayTypeIndex`; input to `OverlayToTiberiumIndex`; full removal writes `-1` | decompile `0x00480A80`; assembly `00480B63`, `00480BCE` | Yes |
| `CellClass+0x11E` | `byte` | `OverlayData`; tiberium density byte; partial subtract; full removal writes `0` | decompile `0x00480A80`; assembly `00480B91`, `00480BA6`, `00480BBD`, `00480BD5` | Yes |
| `CellClass+0x24/+0x26` | `short, short` | map coord passed to radar and queue reseed | assembly `00480BE1-00480BEA`, `00480BFD-00480C10` | Yes |
| `TiberiumClass+0xF8` | `byte*` | spread-queue bitmap pointer checked before neighbor enqueue | decompile `0x00480A80`; `AddToSpreadQueue @ 0x00722AF0` | Yes, conditional on spread queue use |
| `TiberiumClass+0xF0/+0xF4/+0xFC` | queue fields | spread queue count, heap, entries | decompile `0x00722AF0` | Yes, conditional on `CanSpreadTiberium` |
| `TiberiumClass+0x10C/+0x110/+0x114/+0x118` | queue fields | growth queue count, heap, bitmap, entries | decompile `0x007235A0` | Yes, but density-11 call is a net no-op for value 11 |
| `g_TiberiumClass_Array @ 0x00B0F4EC` | pointer array | resolves tiberium type by index | assembly `00480B88-00480B8E`; decompile `0x005FDD20` | Yes |
| `g_TiberiumClass_Array_Count @ 0x00B0F4F8` | int | loop bound for all-type bitmap clearing | decompile `0x00722AB0`; ADDRESS_MAP | Yes |

## 3. Core Logic

### Entry guards and signedness

The stack argument is loaded into `EBX`; the guard is signed:

- `00480B73: MOV EBX,dword ptr [ESP + 0x50]`
- `00480B77: TEST EBX,EBX`
- `00480B79: JLE 0x00480CA0`
- `00480B7F: CMP EAX,-0x1`
- `00480B82: JZ 0x00480CA0`

Verified behavior:

- `amount <= 0` as a signed integer returns 0 before any ore mutation.
- `OverlayToTiberiumIndex == -1` returns 0 before any ore mutation.
- Negative `amount` does not act as a huge unsigned reduction because the signed `JLE` guard catches it.

Active in YR: Yes. `UnitClass__Harvest_Ore_Tick` passes a normal positive `ftol(Storage - GetTotalAmount())` value on the standard harvester path.

### Density-11 detour

When `OverlayData == 11`, `Reduce_Tiberium` calls `TiberiumClass__AddToGrowthQueue(&cell->MapCoord)` before it reads the current density for reduction:

- `00480B91: MOV AL,byte ptr [ESI + 0x11e]`
- `00480B97: CMP AL,0xb`
- `00480B99: JNZ 0x00480BA6`
- `00480B9B: LEA EDX,[ESI + 0x24]`
- `00480B9E: MOV ECX,EBP`
- `00480BA0: PUSH EDX`
- `00480BA1: CALL 0x007235A0`

`AddToGrowthQueue @ 0x007235A0` immediately looks the cell back up and only queues if `*(byte *)(cell + 0x11E) < 0xB`. Since `Reduce_Tiberium` calls it before decrementing or clearing the cell, an `OverlayData=11` cell fails this internal guard. Net effect for `OverlayData=11`: the call is made, but no growth queue entry is inserted.

Active in YR: Yes, the call branch is active for max-density ore, but its enqueue effect is blocked by the callee's own guard for the standard `OverlayData=11` case.

### Partial removal

After the density-11 detour, the function reads the current density byte:

- `00480BA6: MOV AL,byte ptr [ESI + 0x11e]`
- `00480BAC: MOV ECX,EAX`
- `00480BAE: AND ECX,0xff`
- `00480BB4: LEA EDX,[ECX + 0x1]`
- `00480BB7: CMP EDX,EBX`
- `00480BB9: JLE 0x00480BC8`
- `00480BBB: SUB AL,BL`
- `00480BBD: MOV byte ptr [ESI + 0x11e],AL`
- `00480BC3: JMP 0x00480C67`

The branch condition is equivalent to: if `current + 1 > amount`, partial removal. It subtracts the low byte of `amount` from `OverlayData` and returns `amount`. For ordinary harvester requests, `amount` is positive and small enough that this byte subtraction is ordinary density subtraction.

Important off-by-one: an `OverlayData=11` cell full-removes only when `amount >= 12`, but the full-removal return value is `current` (11), not `current + 1` (12).

Active in YR: Yes. Standard harvesters can take this path whenever empty capacity is smaller than `current + 1`.

### Full removal

When `amount >= current + 1`, the function sets the return register source to `current` and clears the overlay before recalculation:

- `00480BC8: MOV EBX,ECX` sets return value to current `OverlayData`
- `00480BCE: MOV dword ptr [ESI + 0x44],0xffffffff`
- `00480BD5: MOV byte ptr [ESI + 0x11e],0x0`
- `00480BDC: CALL 0x0047D2B0` (`CellClass__RecalcAttributes`)
- `00480BE1: ADD ESI,0x24`
- `00480BE9: PUSH ESI`
- `00480BEA: CALL 0x006551C0` (`RadarClass__MarkTerrainDirty`)
- `00480BF1: CALL 0x00722AB0` (`TiberiumClass__ClearSpreadBitmaps_AllTypes`)

After full-removal side effects and neighbor reseeding, both partial and full paths fall through to tactical dirtying and return:

- `00480C89: CALL 0x006D2790` (`TacticalClass__DirtyScreenRect`)
- `00480C94: MOV EAX,EBX`
- `00480C9D: RET 0x4`

Active in YR: Yes. Standard `CMIN` with empty storage requests 20; on `OverlayData=11`, `20 >= 12`, so this path runs.

### Neighbor reseed loop

Full removal performs an ordered 8-neighbor loop after clearing spread bitmaps:

- `00480BF6: XOR EDI,EDI`
- `00480BFA: AND EAX,0x7`
- `00480BFD: MOV CX,word ptr [EAX*0x4 + 0x89f688]`
- `00480C05: MOV DX,word ptr [EAX*0x4 + 0x89f68a]`
- `00480C0D: ADD CX,word ptr [ESI]`
- `00480C10: ADD DX,word ptr [ESI + 0x2]`
- `00480C37: CALL 0x00568300` in-bounds check
- `00480C44: CALL 0x0042B1C0` linear cell index
- `00480C49: MOV EDX,dword ptr [EBP + 0xf8]`
- `00480C4F: CMP byte ptr [EAX + EDX*0x1],0x0`
- `00480C53: JNZ 0x00480C61`
- `00480C59: MOV ECX,EBP`
- `00480C5C: CALL 0x00722AF0`
- `00480C61: INC EDI`
- `00480C62: CMP EDI,0x8`
- `00480C65: JL 0x00480BF8`

The loop order is direction index `0..7`. The exact runtime dx/dy table values are deferred because `0x0089F688` is a runtime-initialized direction table, but the call shape, order, and argument ownership are verified.

Active in YR: Yes for full removal. The actual neighbor admission is conditional inside `AddToSpreadQueue` on `CanSpreadTiberium`, spread bitmap state, and the scenario/map spread gates.

## 4. INI Keys

| Key | Stock YR value | Effect in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `[CMIN] Harvester` | `yes` | Enables standard harvester branch at `UnitType+0xE0E` | `ini/rulesmd.ini:7364`; `0x0073D4A4-0x0073D4AC` | Yes |
| `[CMIN] Storage` | `20` | Empty CMIN requests 20 density levels | `ini/rulesmd.ini:7374`; `0x0073D556-0x0073D5A1` | Yes |
| `[CMIN] PipScale` | `Tiberium` | Visible cargo pips exist, but formula is out of scope | `ini/rulesmd.ini:7372` | Yes |
| `[CMIN] Teleporter` | `yes` | Identifies Chrono Miner; irrelevant to `Reduce_Tiberium` | `ini/rulesmd.ini:7396` | Yes, not used by this helper |
| `[Riparius] Value` | `25` | 11 returned units later deposit as 275 credits | `ini/rulesmd.ini:30388-30396`; storage add at `0x0073D5AE-0x0073D5B9` | Yes |
| `[Riparius] Growth/Spread` | `2200/2200` | Queue timing outside this report; queue add paths are live with stock data | `ini/rulesmd.ini:30393-30396` | Yes, deeper queue timing out of scope |
| `[General] TiberiumGrows/Spreads` | `yes/yes` | Scenario/map gates allow normal ore growth/spread unless overridden | `ini/rulesmd.ini:43-45`; `CanGrow/CanSpread` decompile | Conditional, default yes |

## 5. Integration Points

### Live standard harvester caller

`UnitClass__Harvest_Ore_Tick @ 0x0073D450` is the active standard harvester path. It:

1. Gets the cell at the unit coordinate.
2. Returns early if the unit has `Unit+0x5A4` set.
3. Requires `UnitType+0xE0E Harvester != 0`.
4. Requires virtual call `vtable+0x2B4` result below 1.0.
5. Requires current cell `LandType == 5` (tiberium).
6. Branches away for `UnitType+0xE0F Weeder`; stock `CMIN` does not set this.
7. Gets tiberium type, computes `Storage - StorageClass::GetTotalAmount`, converts via `Math__ftol`, calls `Reduce_Tiberium`, and if the return value is positive calls `StorageClass__AddAmount((float)returned, tibType)`.

Assembly context:

- Harvester flag gate: `0073D4A4: MOV AL,byte ptr [EDX + 0xe0e]`, `0073D4AA: TEST AL,AL`, `0073D4AC: JZ 0x0073D5FE`
- Land type gate: `0073D4CD: CMP dword ptr [EBP + 0xec],0x5`, `0073D4D4: JNZ 0x0073D5FE`
- Weeder split: `0073D4E0: MOV AL,byte ptr [ECX + 0xe0f]`, `0073D4E8: TEST AL,AL`, `0073D4EA: JZ 0x0073D541`
- Reduce call: `0073D599: CALL 0x007C5F00` (`Math__ftol`), `0073D59E: PUSH EAX`, `0073D59F: MOV ECX,EBP`, `0073D5A1: CALL 0x00480A80`
- Storage add: `0073D5A6: TEST EAX,EAX`, `0073D5AC: JLE 0x0073D623`, `0073D5AE: FILD dword ptr [ESP + 0x10]`, `0073D5B9: CALL 0x006C9690`

Active in YR: Yes. `ini/rulesmd.ini` proves stock `CMIN` has the required harvester/storage data; no TS-only gate is required for this path.

### Other callers

`get_function_callers 0x00480A80` returned:

- `AnimClass__Middle @ 0x00424CE0`
- `AnimClass__Start @ 0x00424F00`
- `Apply_area_damage @ 0x00489280`
- `BuildingClass__ExtendWallInDirection @ 0x00452DC0`
- `FUN_00522E70 @ 0x00522E70`
- `MapClass__ReduceTiberiumInRadius @ 0x0057B790`
- `UnitClass__Harvest_Ore_Tick @ 0x0073D450`

Active in YR: Mixed/conditional. This report only proves the standard harvester caller is active for the target scenario. The existence of the other callers is verified, but their full liveness and gates are out of scope.

## 6. Current Rust Implementation Status

Rust currently has three separate surfaces that matter for a future fix:

| Surface | Current behavior | Delta vs verified binary |
|---|---|---|
| `src/sim/production/production_queue.rs:132-172` | Seeds live map ore resource nodes from overlays, using `richness = entry.frame.min(11) + 1`; `OverlayData=11` becomes 12 levels | Mismatch for `Reduce_Tiberium` harvest semantics; binary full removal returns 11 for `OverlayData=11` |
| `src/sim/miner/miner_system.rs:804-855` | `extract_bales_max` extracts `remaining / base` bales, clears resource node and overlay when empty | Mismatch: returns 12 bales from an overlay-seeded max ore cell; lacks authoritative side-effect bundle |
| `src/sim/overlay_grid.rs:92-99`, `184-260` | `clear_overlay` pushes dirty cell; app later recalculates passability/terrain metadata | Directionally similar to RecalcAttributes, but not an atomic `Reduce_Tiberium` boundary |
| `src/sim/ore_growth.rs:1-15`, `156-170`, `292-337` | RA1-style scan/reservoir growth and random-start spread | Not the RA2/YR per-type queue model used by `ClearSpreadBitmaps_AllTypes` and `AddToSpreadQueue` |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass__Reduce_Tiberium @ 0x00480A80` guards | verified | decompile; assembly `00480B73-00480B82` | none for amount/sign guard |
| partial-removal branch | verified | decompile; assembly `00480BA6-00480BC3` | high-amount low-byte edge not relevant to standard harvesters |
| full-removal return value | verified | decompile; assembly `00480BC8`, `00480C94` | none |
| overlay write order | verified | assembly `00480BCE`, `00480BD5`, `00480BDC` | none |
| `RecalcAttributes` call | verified | decompile `0x0047D2B0`; assembly `00480BDC` | exact downstream zone invalidation timing outside helper not expanded |
| radar dirty call | verified | decompile `0x006551C0`; assembly `00480BE1-00480BEA` | renderer consumption timing out of scope |
| tactical dirty call | verified | assembly `00480C67-00480C8F` | exact dirty rect geometry not re-derived |
| all-type spread bitmap clear | verified | decompile `0x00722AB0`; assembly call `00480BF1` | none for single-cell entry clearing |
| neighbor reseed loop call order | verified | assembly `00480BF6-00480C65` | exact runtime dx/dy table values deferred |
| `AddToSpreadQueue` internal gates | touched-not-exhausted | decompile `0x00722AF0`, `CanSpreadTiberium @ 0x00483690` | complete queue architecture owned by slot 2/3 |
| density-11 growth call net effect | verified | assembly `00480B91-00480BA1`; decompile `0x007235A0` | none for `OverlayData=11` net no-op |
| standard `CMIN` liveness | verified | `ini/rulesmd.ini:7351-7396`; `0x0073D450` decompile/assembly | none for target scenario |
| non-harvester callers | touched-not-exhausted | caller table from Ghidra | full liveness/gates out of scope |
| current Rust extraction delta | verified from source scan | `miner_system.rs:804-855`, `production_queue.rs:132-172` | future implementation design |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Does standard CMIN reach Reduce_Tiberium? -> Yes; CMIN has Harvester=yes/Storage=20 and `Harvest_Ore_Tick` calls `0x00480A80` after harvester and land-type gates.` (evidence: `ini/rulesmd.ini:7364,7374`; assembly `0073D4A4-0073D5A1`)
- `[RESOLVED] OQ2 - Is amount signed or unsigned for bounds? -> Signed for guard and branch; `amount <= 0` returns 0, and partial/full compares `current+1 <= amount` using signed conditional branch.` (evidence: assembly `00480B77-00480B79`, `00480BB7-00480BB9`)
- `[RESOLVED] OQ3 - What does full removal return? -> It returns pre-removal `OverlayData`, not `OverlayData+1`; `OverlayData=11` returns 11.` (evidence: assembly `00480BC8`, `00480C94`)
- `[RESOLVED] OQ4 - What does partial removal return? -> It subtracts amount from the density byte and returns amount.` (evidence: assembly `00480BBB-00480BC3`, decompile `0x00480A80`)
- `[RESOLVED] OQ5 - Are overlay writes before RecalcAttributes? -> Yes; `OverlayTypeIndex=-1`, then `OverlayData=0`, then `RecalcAttributes`.` (evidence: assembly `00480BCE-00480BDC`)
- `[RESOLVED] OQ6 - Does full removal radar-dirty the cell? -> Yes; it passes `&CellClass+0x24` to `RadarClass__MarkTerrainDirty`, which dedups/appends and sets dirty flag `+0x14D9=1`.` (evidence: assembly `00480BE1-00480BEA`; decompile `0x006551C0`)
- `[RESOLVED] OQ7 - Does full removal tactical-dirty the screen? -> Yes; both mutation paths fall through to `TacticalClass__DirtyScreenRect` before return.` (evidence: assembly `00480C67-00480C8F`)
- `[RESOLVED] OQ8 - Does full removal clear spread bitmaps for all types or one type? -> All tiberium types, but only the removed cell's bitmap entry.` (evidence: decompile `0x00722AB0`)
- `[RESOLVED] OQ9 - Which queue receives neighbor reseed? -> The removed cell's tiberium type only; `ECX=EBP` before `AddToSpreadQueue`.` (evidence: assembly `00480C59-00480C5C`)
- `[RESOLVED] OQ10 - What is the density-11 branch effect? -> It calls `AddToGrowthQueue`, but the callee's `< 11` guard sees the still-unchanged value 11, so no entry is inserted.` (evidence: assembly `00480B91-00480BA1`; decompile `0x007235A0`)
- `[RESOLVED] OQ11 - Is `OverlayToTiberiumIndex` fallback `-1` or `0` for a flagged-but-unmatched overlay? -> It returns 0 after logging, so Reduce_Tiberium would treat such an overlay as index 0 instead of bailing.` (evidence: decompile `0x005FDD20`)
- `[RESOLVED] OQ12 - Does Rust currently use this helper-equivalent for miner harvest? -> No; miner harvest uses `extract_bales_max` over `resource_nodes`, not the existing combat/smudge `reduce_tiberium` helper.` (evidence: `src/sim/miner/miner_system.rs:520-540`, `804-855`)
- `[DEFERRED] OQ13 - Exact runtime dx/dy values in `g_DirectionOffsets @ 0x0089F688`.` (category: bounded-cost-too-high; reason: table is runtime-initialized and exact values are not required to prove the implementation-critical call order/argument ownership in this slot; next-step-if-pursued: trace writes/xrefs to the direction table initializer)
- `[DEFERRED] OQ14 - Complete `AddToSpreadQueue` admission semantics and serialization.` (category: requires-different-system-context; reason: assigned to queue-state swarm slot; next-step-if-pursued: investigate TiberiumClass growth/spread queue state and save/load)
- `[DEFERRED] OQ15 - Exact selected cargo pip formula after 11/20 storage.` (category: out-of-scope; reason: UI pip rendering not part of Reduce_Tiberium helper; next-step-if-pursued: trace selected-unit PipScale=Tiberium draw path)

## 9. Negative Facts / Do Not Do

- Do not model full-removal harvested amount as `OverlayData + 1`. Evidence: full-removal path sets `EBX=current OverlayData` at `00480BC8` and returns `EAX=EBX` at `00480C94`; `OverlayData=11` returns 11.
- Do not enqueue the removed `OverlayData=11` cell into the growth queue as a net effect of the density-11 branch. Evidence: `0x00480BA1` calls `AddToGrowthQueue`, but `0x007235A0` admits only if the looked-up cell has `OverlayData < 11`; the call happens before mutation.
- Do not clear only the current tiberium type's spread bitmap entry on full removal. Evidence: `ClearSpreadBitmaps_AllTypes @ 0x00722AB0` loops over `g_TiberiumClass_Array_Count`.
- Do not reseed all tiberium types' spread queues. Evidence: neighbor loop calls `AddToSpreadQueue` with `ECX=EBP`, where `EBP` is the tiberium pointer resolved from the removed overlay at `00480B88-00480B8E`.
- Do not defer terrain/passability invalidation until after another sim system can observe stale tiberium if implementing a shared reduction API. Evidence: gamemd writes overlay fields before `RecalcAttributes` in the same `Reduce_Tiberium` call (`00480BCE-00480BDC`).

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Full removal returns pre-removal `OverlayData`; for `OverlayData=11` and amount 20, return is 11 and storage add is 11.0 Riparius. Active in YR: Yes. | `0x00480A80` decompile; assembly `00480BC8`, `00480C94`; harvester caller `0073D599-0073D5B9`; `ini/rulesmd.ini:7351-7396`, `30388-30396` | Rust overlay seeding turns frame 11 into 12 harvestable bales and `extract_bales_max` returns 12 | `src/sim/production/production_queue.rs:132-172`; `src/sim/miner/miner_system.rs:804-855` | Harvest cargo/payment semantics for real overlay-backed ore must use gamemd reduction result, not `frame+1` for max-density harvest | Standard empty CMIN harvests one Riparius cell with `OverlayData=11`; after one extraction, cargo count is 11 and carried value is 275 | Test name: `cmin_overlaydata_11_full_removal_returns_11_bales`; risk: breaking overlay rendering by changing visual frame instead of extraction semantics |
| Full removal is one atomic side-effect bundle: clear overlay type/data, recalc attributes, radar dirty, clear all-type bitmap entry, reseed neighbors for current tib type, tactical dirty. Active in YR: Yes for full removal from harvester. | assembly `00480BCE-00480C8F`; decompiles `0x0047D2B0`, `0x006551C0`, `0x00722AB0`, `0x00722AF0` | Rust removes resource node and clears overlay, but queue reseed is absent and terrain recalculation is a later dirty-cell process | `src/sim/miner/miner_system.rs:838-842`; `src/sim/overlay_grid.rs:92-99`, `184-260`; `src/sim/ore_growth.rs` or replacement queue module | Create one authoritative sim-facing reduction path that publishes terrain/overlay dirty effects before later sim decisions and records/dispatches the queue reseed event | Deplete a tiberium cell and immediately issue a path/passability query plus inspect the spread-queue observable; no stale tiberium terrain and valid neighbor reseed exists | Test name: `reduce_tiberium_full_removal_recalc_and_reseeds_neighbors`; risk: bolting queue reseed onto miner only and missing warhead/anim callers |
| Partial removal subtracts amount from `OverlayData` and returns amount when `amount < current+1`; amount guard is signed and returns 0 for `amount <= 0`. Active in YR: Yes when harvester empty capacity is smaller than cell density. | assembly `00480B77-00480B79`, `00480BB7-00480BBD`; decompile `0x00480A80` | Rust partial extraction uses resource-node levels and overlay data update `(remaining_after / base) - 1`; exact helper semantics are not centralized | `src/sim/miner/miner_system.rs:819-852`; existing combat/smudge reduction helper should be audited before reuse | A shared helper should preserve binary branch behavior for zero/negative-equivalent, partial, and full removal cases | Call helper on representative overlay states: amount 0 returns 0/no mutation; amount 3 on `OverlayData=5` returns 3 and leaves `OverlayData=2`; amount 20 on 11 returns 11 and clears | Test name: `reduce_tiberium_partial_and_zero_amount_match_gamemd`; risk: using unsigned saturation that makes negative/interpreted values destructive |

## 11. Stale Docs / Follow-up Docs

- `docs/contracts/2026-05-23-chrono-miner-reduce-tiberium-implementation-contract.md`: replace "density mapping, 11-vs-12 bale behavior" with "full-removal harvest amount is the pre-removal `OverlayData` byte; for `OverlayData=11`, `Reduce_Tiberium(20)` returns 11, not 12. Do not globally reinterpret the visual overlay frame until the queue/overlay architecture is designed."
- `docs/research/CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`: no contradiction found. This report narrows and strengthens the harvester-active evidence and Rust handoff wording.

## Sources

- Ghidra MCP `decompile_function 0x00480A80`
- Ghidra MCP `get_assembly_context 0x00480A80`, `0x00480BEA`, `0x0073D599`
- Ghidra MCP `get_function_callers 0x00480A80`
- Ghidra MCP `get_function_callees 0x00480A80`
- Ghidra MCP `decompile_function 0x0073D450`, `0x00722AB0`, `0x00722AF0`, `0x007235A0`, `0x005FDD20`, `0x006551C0`, `0x0047D2B0`, `0x00483620`, `0x00483690`
- `ini/rulesmd.ini`
- `src/sim/miner/miner_system.rs`
- `src/sim/production/production_queue.rs`
- `src/sim/overlay_grid.rs`
- `src/sim/ore_growth.rs`
- `docs/research/CELLCLASS_REDUCE_TIBERIUM_FUN_00480A80_GHIDRA_REPORT.md`
- `docs/research/traces/CMIN_HARVEST_DENSITY_CARGO_REDUCE_TIBERIUM_TRACE.md`
