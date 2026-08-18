# HARV Destroyed Refinery Unload Abort - Ghidra Research Report

**Address(es):** `0x004593A0` (`BuildingClass::UndockUnit`), `0x00442230`
(`BuildingClass::ReceiveDamage`), `0x00449C30` (`BuildingClass::Sell`),
`0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x0073E5E0`
(`UnitClass::Mission_Harvest`), `0x0047C520` (`Look_up_building_in_cell`),
`0x00451E40` (`BuildingClass::ClearAnimSlot`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** stock YR `HARV -> NAREFN/GAREFN` behavior when the selected
refinery is sold or destroyed while the War Miner is approaching, pivoting,
unloading, or in a conditional reciprocal `+0x2E4` physical link.
**Non-Scope:** Yuri slave miner, modded refineries with non-stock animation
lifetime, live frame capture, full production/sell refund flow, and all
possible radio command senders outside this dock/refinery path.
**Confidence:** High for static binary path liveness, caller order, offsets,
and no-credit-after-missing-building behavior. Medium for exact rendered
`UnloadingClass` reversion frame because the render consumer of the unload flag
was not re-decompiled in this slice.
**Active in YR:** Yes. `[HARV]` has `Harvester=yes`, `Dock=NAREFN,GAREFN`,
`Storage=40`, and `UnloadingClass=HORV` in `rulesmd.ini`; `NAREFN/GAREFN` are
stock `Refinery=yes`/`DockUnload=yes` refineries.

## 0. Working Notes

**Target question:** Does stock YR allow a War Miner to keep depositing cargo
into a refinery after that refinery is sold/destroyed, and what links/visual
state are cleared?

**Non-goals:** Do not re-prove normal cargo-empty exit, queue takeover timing,
or chrono-specific locomotor details except where they affect this abort.

**Evidence needed to mark COMPLETE:**

- Verify sell and damage paths call `UndockUnit` when `building+0x2E4` is live.
- Verify `UndockUnit` clears both `+0x2E4` pointers and does not call any credit
  or storage-drain function.
- Verify the stock zero-link unit-side unload branch re-finds the refinery by
  cell lookup and skips the drain/credit block when lookup fails.
- Verify the fallback mission after missing refinery returns cargo-bearing
  harvesters to the harvest/refinery-selection path rather than an ore-drain
  credit path.
- Compare current Rust surfaces for dying/missing refinery handling and
  `display_type_override` cleanup.

**Stop conditions:** Stop once the sell/damage caller order, state-3 missing
building branch, and current Rust deltas are resolved with evidence; defer live
render-frame proof and broader animation-system consumers.

## 1. Overview

Stock YR does not credit additional War Miner cargo after the refinery is no
longer a live building at the dock lookup cell. The unit-side unload loop
drains cargo only after `Look_up_building_in_cell()` returns a building in state
3; if that lookup returns null, it optionally sends radio `3`, sets
`Mission_Harvest` (`0x0A`, queued), and reaches the timer epilogue without
calling `StorageClass::RemoveAmount` or `HouseClass::Add_Tiberium_Credits`.

When a reciprocal `building+0x2E4` link is actually live, sell and destruction
call `BuildingClass::UndockUnit` first. That helper ejects the miner through the
drive-locomotor interrupt path, clears both link pointers, and sends radio `3`;
it contains no storage or credit calls.

## 2. Class Layout / Key Offsets

| Offset / field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Building/Unit `+0x2E4` (`[0xB9]`) | reciprocal dock-link pointer when present | `0x004593A0`, `0x004424EA`, `0x0044AAB0` | Conditional; not normal stock zero-link completion |
| Unit `+0xBC` (`[0x2F]`) | `Mission_Deploy_Building` substate | `0x0073D630` state checks/writes | Yes |
| Unit `+0xF8` (`[0x3E]`) | dump-rate accumulator | `0x0073E355..0x0073E374`; reset after credit | Yes |
| Unit byte `+0x6D1` | unload-active/first-entry flag | set `0x0073DFDA`, clear `0x0073E1F6` | Yes |
| Unit `+0x33C` | harvester `StorageClass` | state-3 drain calls | Yes |
| UnitType `+0xE0E` | `Harvester=yes` | `0x0073D678`, `0x0073E013`, INI | Yes for HARV |
| UnitType `+0xE0F` | `Weeder=yes` | alternate TS branch in state 3 | No for stock HARV |
| BuildingType `+0x16BB` | `Refinery=yes` | state 3/4 animation gates | Yes for GAREFN/NAREFN |
| Building `+0x584` | anim slot 10 (`SpecialAnim`) pointer | state-3 clear/check | Yes if slot exists |
| Building `+0x57C` | anim slot 8 (`ProductionAnim`) pointer | state-4 wait guard | Usually null for stock refineries |
| `CellClass+0xE4` | object list scanned for buildings | `Look_up_building_in_cell @ 0x0047C520` | Yes |

## 3. Core Logic

### 3.1 Physically linked / reciprocal `+0x2E4` interrupt

`BuildingClass::ReceiveDamage @ 0x00442230` calls `TechnoClass::ReceiveDamage`
and switches on the result. In result case `4`, if `building+0x2E4 != 0`, it
removes that pointer from the local destination/contact list and calls
`BuildingClass::UndockUnit @ 0x004593A0` before later destruction/unlimbo work.

`BuildingClass::Sell @ 0x00449C30` does the same early in sell state `0`: if
`building+0x2E4 != 0`, it calls `BuildingClass::UndockUnit` before sell state
advances to removal/refund work.

`UndockUnit`:

1. Reads `building+0x2E4` into the unit pointer.
2. Returns immediately if null.
3. Checks unit vtable `+0x2C == 1` (DriveLocomotion-class path).
4. Calls locomotor vtable `+0x58`.
5. Calls building `GetCoords`, then locomotor vtable `+0x70` with track `0x47`,
   `x - 0x80`, `y + 0x80`, and unchanged z.
6. Sets unit speed multiplier to double `1.0` via vtable `+0x544`.
7. Clears `unit+0x2E4 = 0`.
8. Clears `building+0x2E4 = 0`.
9. Sends radio command `3` via building vtable `+0x274`.

There is no call to `StorageClass::RemoveAmount`, `StorageClass::GetAmount`, or
`HouseClass::Add_Tiberium_Credits` in `UndockUnit`. So a real linked interrupt
ejects and preserves undumped cargo.

### 3.2 Approaching / no reciprocal link yet

For a full HARV that loses its selected refinery before docking,
`UnitClass::Mission_Harvest @ 0x0073E5E0` case `0` checks storage percentage
before ore search. If non-Weeder and storage percentage is `>= 1.0`, it writes
substate `2` and returns. State `2` then searches for a refinery/docking bay via
the vtable `+0x528` paths. This means a full miner remains in the
return/refinery-selection loop rather than resuming ore search just because the
old refinery disappeared.

### 3.3 Pivoting / first unload-state entry

`UnitClass::Mission_Deploy_Building @ 0x0073D630` is the active unit-side unload
mission. On the stock zero-link path (`unit+0x2E4 == 0`), the harvester branch
uses a RateTimer gate, sets `unit+0x6D1`, and in first-entry state work attempts
to find the refinery at current cell plus the hardcoded west-neighbor lookup.
If no building is found during this first-entry anim setup, it simply skips
slot-7 animation setup and still enters state `3`; the next state-3 pass is the
load-bearing abort branch.

### 3.4 Unloading / state-3 missing-building abort

State `3` re-finds the refinery every deposit pass:

1. Get unit cell.
2. Add the hardcoded `(-1,0)` refinery lookup offset.
3. `MapClass::Get_CellClass`.
4. `Look_up_building_in_cell`.

`Look_up_building_in_cell @ 0x0047C520` scans `CellClass+0xE4` and returns the
first object whose vtable `+0x2C` reports type `6` (BuildingClass). If there is
no such object, it returns null.

Only the non-null branch reaches the dump gate and credit block:

- `HarvesterDumpRate * 900.0 <= unit+0xF8`.
- possible particle/slot-10 anim.
- `StorageClass::FindFirstNonEmptySlot`.
- `StorageClass::GetAmount`.
- `StorageClass::RemoveAmount`.
- `HouseClass::Add_Tiberium_Credits` for base amount and optional purifier
  bonus.

If the building lookup is null, the function takes the abort branch instead:

- calls `PathType::Has_Valid_Steps`;
- if true, sends radio command `3`;
- calls `SetMission(10, 1)` (`Mission_Harvest`, queued);
- reaches the timer epilogue.

No storage or credit call is reachable on that null-building branch.

### 3.5 Visible unloading class cleanup

Static evidence for the exact render switch is indirect in this slice:
`UnloadingClass=HORV` is a verified HARV type key, and prior HARV docs identify
it as the dock-unload visual form. The binary path here proves the unload path
is exited on missing-building abort: reciprocal-link interrupts clear both
`+0x2E4` links and radio-clear; zero-link state-3 abort leaves the drain loop by
setting `Mission_Harvest`. Normal stock state 4 later clears `unit+0x6D1`.

Therefore Rust must not leave `display_type_override=HORV` after an abort. The
exact frame where stock render reverts was not re-verified by decompiling the
render consumer of `+0x6D1` / `UnloadingClass` in this slice.

## 4. INI Keys

| INI key | Stock value | Effect | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[HARV] Harvester` | `yes` | selects harvester mission/unload path | Yes |
| `rulesmd.ini:[HARV] Dock` | `NAREFN,GAREFN` | refinery candidates | Yes |
| `rulesmd.ini:[HARV] Storage` | `40` | full cargo threshold and carried slots | Yes |
| `rulesmd.ini:[HARV] UnloadingClass` | `HORV` | unload visual type | Yes |
| `rulesmd.ini:[HORV] Dock` | `NAREFN,GAREFN` | unload form retains dock compatibility | Yes as type data |
| `rulesmd.ini:[GAREFN] Refinery` | `yes` | refinery gate in state 3/4 | Yes |
| `rulesmd.ini:[GAREFN] DockUnload` | `yes` | building radio handoff to unload mission | Yes |
| `rulesmd.ini:[NAREFN] Refinery` | `yes` | same as GAREFN | Yes |
| `rulesmd.ini:[NAREFN] DockUnload` | `yes` | same as GAREFN | Yes |
| `[General] HarvesterDumpRate` | `0.016` | dump gate: `0.016 * 900.0 = 14.4` frames | Yes |
| `[General] PurifierBonus` | `.25` | optional owner bonus after slot drain | Yes |

## 5. Integration Points

| Function / surface | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::ReceiveDamage @ 0x00442230` | destruction case calls `UndockUnit` if linked | decompile case `4` | Yes |
| `BuildingClass::Sell @ 0x00449C30` | sell state `0` calls `UndockUnit` if linked | decompile | Yes |
| `BuildingClass::UndockUnit @ 0x004593A0` | linked interrupt/eject helper | decompile | Conditional |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | active harvester unload FSM and missing-building abort | decompile | Yes |
| `Look_up_building_in_cell @ 0x0047C520` | validates that a live BuildingClass object is in the dock lookup cell | decompile | Yes |
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | full-cargo check routes to refinery return before ore search | decompile | Yes |
| `BuildingClass::ClearAnimSlot @ 0x00451E40` | clears/kills anim slot(s); relevant to normal cleanup, not called by `UndockUnit` | decompile | Yes |

## 6. Current Rust Implementation Status

Current Rust has already moved beyond the older
`MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md` mismatch in several places:

- `src/sim/miner/miner_dock_sequence.rs:317` rejects a refinery if the entity
  is missing, `dying`, or `health.current == 0`.
- `src/sim/miner/miner_dock_sequence.rs:351` / `:361` choose
  `ReturnToRefinery` for full cargo aborts, not `SearchOre`.
- `src/sim/miner/miner_dock_sequence.rs:400` implements an
  `interrupt_refinery_docked_miners` helper for sell/removal, including
  reservation cancel and display override cleanup.
- `src/sim/miner/miner_dock_sequence.rs:471` clears `display_type_override`,
  movement/facing/track state, dock queue, exit cell, cooldown, and unload timer
  on invalid-refinery abort.
- `src/sim/miner/miner_system.rs:118` treats dying entities as not alive for
  dock reservation cleanup.
- `src/sim/miner/miner_system.rs:304` checks full cargo before ore search.
- `src/sim/miner/miner_tests.rs:4287` and `:4348` cover full cargo dying
  refinery fallback and unload abort without credit/stuck visual.

Remaining Rust-facing risk: combat destruction paths should trigger the same
abort before any later miner tick can credit a dying/refinery corpse. Current
combat code sometimes removes structures immediately (`src/sim/combat/mod.rs:980`)
and C4 can mark structures `dying` (`src/sim/world/world_orders.rs:792`), so both
missing and `dying` forms must stay covered.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required trace path | verified | `miner/traces/MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md` | none |
| `ReceiveDamage` destruction caller order | verified | `0x00442230` case `4` calls `UndockUnit` when `+0x2E4 != 0` | exact post-call unlimbo frame out of scope |
| `Sell` linked caller order | verified | `0x00449C30` state `0`, `+0x2E4` check before sell removal work | none |
| `UndockUnit` link clear | verified | `0x004593A0`: clears unit and building `[0xB9]` | none |
| `UndockUnit` no-credit fact | verified | decompile has no storage/credit calls | none |
| State-3 missing-building abort | verified | `0x0073D630`, null `Look_up_building_in_cell` branch | none |
| Credit path requires building pointer | verified | `0x0073D630`, drain block under `this_00 != 0` | none |
| `Look_up_building_in_cell` semantics | verified | `0x0047C520` scans `CellClass+0xE4`, `WhatAmI()==6` | does not itself check health/dying |
| Full-cargo fallback before ore search | verified | `0x0073E5E0` case `0`, storage `>= 1.0` -> substate `2` | none |
| Render consumer of unload visual | touched-not-exhausted | HARV docs + `+0x6D1` mission evidence | exact frame/render field requires separate visual investigation |
| Runtime same-frame combat removal | touched-not-exhausted | `ReceiveDamage` static path; Rust combat scan | live debugger/frame capture not performed |
| Rust current abort code | verified by source scan | `miner_dock_sequence.rs:317`, `:471`; tests `:4287`, `:4348` | run focused tests in implementation pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this exhaustive or coverage-map? -> Exhaustive-slice for HARV/refinery loss abort, with render-frame proof explicitly non-scope.` (evidence: user target and bounded function set)
- `[RESOLVED] OQ-02 - Does sell call the interrupt helper before removal? -> Yes, `BuildingClass::Sell` checks `+0x2E4` and calls `UndockUnit` in sell state 0.` (evidence: `0x00449C30`)
- `[RESOLVED] OQ-03 - Does destruction call the interrupt helper before destruction teardown? -> Yes, `ReceiveDamage` result case 4 calls `UndockUnit` when `+0x2E4 != 0`.` (evidence: `0x00442230`)
- `[RESOLVED] OQ-04 - What links are cleared by the interrupt helper? -> Unit and building `+0x2E4` are both zeroed.` (evidence: `0x004593A0`)
- `[RESOLVED] OQ-05 - Does the interrupt helper credit cargo? -> No storage or credit calls exist in the body.` (evidence: `0x004593A0`)
- `[RESOLVED] OQ-06 - Where does the unit-side unload loop validate the refinery? -> State 3 calls `Look_up_building_in_cell` from the west-neighbor cell before the dump gate.` (evidence: `0x0073D630`, `0x0047C520`)
- `[RESOLVED] OQ-07 - Can the missing-building branch reach `RemoveAmount`? -> No, it sends optional radio `3`, sets mission `10`, and exits through timer epilogue.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-08 - Is a full miner sent to ore search after refinery loss? -> Not initially; Mission_Harvest state 0 routes full storage to return substate 2 before ore search.` (evidence: `0x0073E5E0`)
- `[RESOLVED] OQ-09 - Is the weeder branch relevant to stock HARV? -> No; `UnitType+0xE0F` is the TS Weeder path and stock HARV does not set it.` (evidence: `0x0073D630`, INI)
- `[RESOLVED] OQ-10 - Does `Look_up_building_in_cell` filter by health? -> No; it only scans object list for `WhatAmI()==6`.` (evidence: `0x0047C520`)
- `[RESOLVED] OQ-11 - What should Rust treat as invalid? -> Missing, `dying`, or zero-health refinery must abort before drain because stock credit path requires a live map building and destruction/sell interrupt exits the dock loop.` (evidence: binary paths + Rust scan)
- `[RESOLVED] OQ-12 - Is current Rust still matching the old stale trace's SearchOre fallback? -> No; current code uses full-cargo return fallback and abort cleanup.` (evidence: `src/sim/miner/miner_dock_sequence.rs:351`, `src/sim/miner/miner_system.rs:304`)
- `[RESOLVED] OQ-13 - Does current Rust clear HORV override on invalid refinery? -> Yes in the invalid-refinery abort helper and interrupt helper.` (evidence: `src/sim/miner/miner_dock_sequence.rs:400`, `:471`)
- `[DEFERRED] OQ-14 - What exact stock render frame switches HORV back to HARV on abort?` (category: `requires-different-system-context`; reason: needs render/unload-class consumer trace or live capture; next-step-if-pursued: trace the draw path that consumes `UnloadingClass`/`+0x6D1`)
- `[DEFERRED] OQ-15 - Exact same-tick ordering after combat kill with building death animation?` (category: `needs-runtime-debugger`; reason: static caller proves interrupt/no-credit branches, but not a frame capture; next-step-if-pursued: trace `ReceiveDamage -> UndockUnit -> unlimbo/cell-list removal` live)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Linked sell/destroy calls `UndockUnit`, clears both `+0x2E4`, no credits | `0x00442230`, `0x00449C30`, `0x004593A0` | mostly implemented for sell; combat path depends on invalid lookup/cleanup | `interrupt_refinery_docked_miners`, combat/C4 destruction hooks, dock reservations | abort linked/on-pad miners before removing/suppressing refinery | `war_miner_linked_refinery_sell_undocks_preserves_cargo_clears_contact` | Do not let a removed or dying refinery stay occupied until death anim ends |
| State-3 missing-building branch cannot drain cargo | `0x0073D630`, `0x0047C520` | current `resolve_refinery_cells` guards `dying`/zero HP | `resolve_refinery_cells`, `phase_unloading` | validate refinery before phase dispatch and return before slot drain | `war_miner_unloading_refinery_destroyed_no_credit_cargo_preserved` | Do not fall back to owner credits when `ref_sid` is gone |
| Full cargo after refinery loss re-enters return/refinery selection | `0x0073E5E0` | current abort state matches | `dock_abort_state`, `handle_search_ore`, `handle_return` | full HARV should choose another live compatible refinery, not ore | `war_miner_full_lost_refinery_selects_next_refinery_not_ore` | Do not clear cargo or target ore while full except stale target cleanup |
| Unload visual must end on abort | `0x0073D630` exits unload loop; normal state 4 clears `+0x6D1`; HARV `UnloadingClass=HORV` | current code clears `display_type_override` on abort/depart | `display_type_override`, dock abort/depart helpers | clear HORV immediately when aborting invalid refinery | `war_miner_dying_refinery_abort_clears_horv_override` | Do not wait for normal deposit cooldown after an abort |
| Contact/radio cleanup uses command `3`, not `0x07`/`0x19` | `0x004593A0`, `0x0073D630` | Rust has explicit reservation cancel/release | `RefineryDockContacts` | release contacts/on-pad state when aborting or exiting | `war_miner_refinery_abort_releases_waiting_queue` | Do not invent DOCKING_COMPLETE/LEAVE_DOCK for this path |

Concrete Rust test-name proposals:

- `war_miner_unloading_refinery_destroyed_no_credit_cargo_preserved`
- `war_miner_unloading_refinery_sold_clears_horv_and_contact`
- `war_miner_full_lost_refinery_selects_next_refinery_not_ore`
- `war_miner_linked_refinery_sell_undocks_preserves_cargo_clears_contact`
- `war_miner_dying_refinery_abort_releases_waiter_without_crediting`

## 10. Negative Facts / Do Not Do

- Do not award any cargo slot after the refinery lookup is null or the Rust
  refinery entity is `dying`/zero-health.
- Do not send full cargo miners to ore search just because their reserved
  refinery disappeared.
- Do not leave `display_type_override=HORV` waiting for normal deposit cooldown
  after a sell/destroy abort.
- Do not model this path with radio `0x07` or `0x19`; verified senders here use
  radio `3`.
- Do not use `ReleaseDockedHarvester` as the stock normal completion model; this
  report only relies on `UndockUnit` for interrupt and state 3/4 for zero-link.

## 11. Remaining Uncertainty

- Exact render-frame reversion for `UnloadingClass=HORV` was not re-proven from
  the renderer/draw consumer; this report only proves the unload mission exits.
- Exact same-frame object-list removal after combat `ReceiveDamage` was not
  captured live. Static evidence still proves no legal credit path once lookup
  is null and proves linked interrupts clear the dock link before destruction
  teardown.
- `Look_up_building_in_cell` itself does not check health, so Rust should keep
  the conservative `dying`/zero-health invalidation even if some stock internals
  remove dead buildings from the cell list earlier.

## 12. Stale Docs / Follow-up Docs

Replace wording in `miner/traces/MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md`
that describes current Rust as still setting `SearchOre` or retaining
`display_type_override` on invalid refinery with:

> Superseded by current Rust scan and
> `HARV_DESTROYED_REFINERY_UNLOAD_ABORT_GHIDRA_REPORT.md`: invalid/missing
> refineries now route full miners back to `ReturnToRefinery`, reject
> `dying`/zero-health refinery entities before unloading, and clear dock
> queue/exit/unload visual state on abort. Keep the binary behavior claim:
> stock state-3 missing-building abort preserves cargo and credits nothing.

Replace any claim that destroyed-but-still-present refineries may be valid
deposit targets in Rust with:

> A refinery entity that is `dying` or at zero health is invalid for miner dock
> resolution; abort before `phase_unloading` can drain a slot.

## Sources

- Ghidra decompiled: `0x004593A0`, `0x00442230`, `0x00449C30`,
  `0x0073D630`, `0x0073E5E0`, `0x0047C520`, `0x00451E40`.
- Required docs read:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`
- INI checked:
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/art.ini`
- Rust scanned:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/production/production_sell.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/mod.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_orders.rs`
