# Missing/Destroyed Refinery Mid-Unload Ordering - Ghidra Research Report

**Address(es):** `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x004593A0` (`BuildingClass::UndockUnit`), `0x00442230` (`BuildingClass::ReceiveDamage`), `0x00449C30` (`BuildingClass::Sell`), `0x0073E5E0` (`UnitClass::Mission_Harvest`), `0x005B35E0` / `0x005B3570` (`Queue_Mission` / `Commence`), `0x0073CEC0` (`UnitClass::DrawExtras`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** stock YR CMIN/HARV -> GAREFN/NAREFN when the selected refinery is missing, sold, destroyed, dying, or physically removed while the harvester is returning, docked, or in `Mission_Deploy_Building` state-3 unload.
**Non-Scope:** normal unload cadence, normal stock zero-link post-unload exit, full `Mission_Enter` retry queue behavior, slave miner/Yuri deploy-miner path, runtime debugger frame capture.
**Confidence:** High for static branch/order and cargo/mission/visual gates; Medium for exact same-render-frame visibility without runtime capture.
**Active in YR:** Yes. `[CMIN]` and `[HARV]` have `Harvester=yes`, `Dock=NAREFN,GAREFN`, `UnloadingClass=CMON/HORV`; `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` in `rulesmd.ini`.

## Target Question

When the refinery selected by a stock CMIN/HARV disappears or becomes unusable mid-cycle, what does gamemd do first, what state does the miner enter, is cargo preserved, are radio/contact/dock links cleaned, does the unload visual clear, and is another refinery selected immediately?

## Non-Goals

- Do not re-open normal stock unload timing, accepted dock anchor, or zero-link exit parity.
- Do not study stale `ReleaseDockedHarvester` normal-exit claims except to distinguish interrupt paths.
- Do not implement Rust.

## Evidence Needed To Mark COMPLETE

- Direct decompile of `Mission_Deploy_Building` state 3 missing-building branch.
- Direct decompile of sell/destroy interrupt callers to `UndockUnit`.
- Direct decompile of `UndockUnit` link/contact cleanup.
- Direct decompile of `Mission_Harvest` state 0/state 2 re-entry after `Queue_Mission(10,1)`.
- Direct decompile of the render gate for `UnloadingClass`.

## Stop Conditions

- Stop at stock HARV/CMIN dock-unload behavior; do not expand into slave miners or service docks.
- Stop when binary order answers cargo, mission, contact, visual, and next-refinery selection.
- Record runtime-frame uncertainty instead of mutating Ghidra or creating missing function boundaries.

## Key Binary Findings

| Finding | Active in YR | Evidence | Confidence |
|---|---:|---|---|
| State-3 missing-building branch first recomputes the adjacent refinery lookup cell, calls `Look_up_building_in_cell`, and only enters the abort branch when that returns null. | Yes | `0x0073E306 CALL 0x0047C520`; `0x0073E30D CMP EDI,EBX`; `0x0073E30F JNZ 0x0073E355` | High |
| Abort order is: optional radio `3`, then `Queue_Mission(10,1)`, then mission-timer return. | Yes | `0x0073E313 CALL PathType__Has_Valid_Steps`; `0x0073E31A JZ 0x0073E328`; `0x0073E31E PUSH 0x3`; `0x0073E322 CALL [vtable+0x274]`; `0x0073E32A PUSH 1`; `0x0073E32C PUSH 0xA`; `0x0073E330 CALL [vtable+0x1E8]` | High |
| Cargo is preserved on the missing-building branch because every `StorageClass__RemoveAmount` / credit path is inside the non-null building branch at `0x0073E355+`; the null branch jumps around it. | Yes | `Mission_Deploy_Building @ 0x0073D630` decompile: null branch runs radio/queue only; drain block begins after non-null branch and calls `FindFirstNonEmptySlot`, `RemoveAmount`, `HouseClass__Add_Tiberium_Credits` | High |
| Missing-building abort does not explicitly clear `unit+0x6D1`; `Queue_Mission` and `Commence` also do not clear it. | Yes | No `+0x6D1` write in `0x0073E306..0x0073E350`; `Queue_Mission @ 0x005B35E0` writes queued mission `+0xB4` and `+0xB8`; `Commence @ 0x005B3570` writes current mission `+0xAC`, substate `+0xBC`, timers, `+0xB8`; no `+0x6D1` | High |
| `UnloadingClass` visual is gated by `Harvester=yes`, `unit+0x6D1 != 0`, and `Type+0x6B8 != 0`; there is no current-mission gate in the swap. | Yes | `UnitClass::DrawExtras @ 0x0073CEC0` checks `Type+0xE0E`, `unit+0x6D1`, `Type+0x6B8`, then temporarily swaps type to `UnloadingClass` | High |
| Therefore, a static-code-correct implementation of the null-refinery state-3 branch must preserve cargo and leave the unload visual flag uncleared unless a later path explicitly clears it. | Yes | Combination of `0x0073E306..0x0073E330`, `0x005B35E0`, `0x005B3570`, `0x0073CEC0` | High static / Medium runtime-frame |
| `BuildingClass::Sell` calls `UndockUnit` before the building is removed from the map/sold path if `building+0x2E4` is nonzero. | Conditional: only reciprocal dock link exists | `BuildingClass::Sell @ 0x00449C30` decompile: `if field_0x2E4 != 0 { BuildingClass__UndockUnit(); }` before sell state proceeds to removal/refund branches | High |
| `BuildingClass::ReceiveDamage` death result case calls `UndockUnit` before capture cleanup, chrono deploy cleanup, death effects, and possible occupy-map/death-animation continuation. | Conditional: damage result case 4 and `building+0x2E4 != 0` | `ReceiveDamage @ 0x00442230`, case 4; `0x004424EA CALL 0x004593A0` before later cleanup calls | High |
| `UndockUnit` clears both dock-link fields and then sends radio `3` from the building. | Conditional: `building+0x2E4 != 0` and docked unit locomotor type returns `1` | `0x00459450 MOV [unit+0x2E4],0`; `0x0045945C MOV [building+0x2E4],0`; `0x00459458 PUSH 0x3`; `0x00459462 CALL [building+0x274]` | High |
| `UndockUnit` does not touch harvester storage. | Conditional as above | Full decompile `0x004593A0`: locomotor stop/head-to, speed, link clears, radio; no `StorageClass` calls | High |
| After `Queue_Mission(10,1)` commences, `Mission_Harvest` state 0 immediately selects return-to-refinery only when storage percentage is full (`>= 1.0`). | Yes | `Mission_Harvest @ 0x0073E5E0`; `0x0073E700 CALL [vtable+0x2B4]`, compare to `1.0`, `0x0073E714 MOV [unit+0xBC],2`, return `1` | High |
| If cargo is not full after an interrupted partial/mixed unload, `Mission_Harvest` state 0 does not immediately select another refinery; it resumes ore search/harvest logic and reaches state 2 only through normal full/no-ore branches. | Yes | `Mission_Harvest @ 0x0073E5E0` state 0 falls through from full check into archive/ore scan; state 2 calls `Find_Docking_Bay` | High |
| State 2 selects another refinery by calling the dock-list search (`Find_Docking_Bay`) rather than remembering the removed refinery. | Yes | `Mission_Harvest @ 0x0073E5E0`, state 2 calls vtable `+0x528`; prior verified address `FootClass__Find_Docking_Bay @ 0x004DF040` | High |

## Branch Order by Scenario

### Return / Before Dock Contact

`Mission_Harvest` state 2 is the active return-refinery path. It searches `Dock=` each pass. If the previously intended refinery is sold/destroyed before a durable dock link exists, the next state-2 pass does not preserve that old object as a hard target; it calls the dock search again. If another valid refinery exists, it can be selected through state 2. Cargo is not modified on this search path.

**Active in YR:** Yes, gated by stock `Harvester=yes` and `Dock=NAREFN,GAREFN`.

### Physical Reciprocal Dock Link Exists (`building+0x2E4 != 0`)

Sell and death-result damage are interrupt paths. They call `UndockUnit` from the building side before the building is finally removed/sold/death-processed. `UndockUnit`:

1. Reads the docked unit from `building+0x2E4`.
2. For drive locomotion, stops it, issues a head-to offset using facing `0x47`, sets speed to `1.0`.
3. Clears `unit+0x2E4`.
4. Clears `building+0x2E4`.
5. Sends radio `3` from the building.

Cargo is untouched. This path is not the normal stock zero-link completion path; it is an interrupt path when the reciprocal pointer exists.

**Active in YR:** Conditional. It is live in YR sell/damage code, but stock zero-link CMIN/HARV unload often has no reciprocal `+0x2E4` during the dump itself.

### Zero-Link State-3 Unload and Refinery Lookup Fails

The state-3 branch order is exact:

1. Compute unit-anchor plus `g_refinery_unload_adjacent_lookup_dx/dy`.
2. `MapClass__Get_CellClass`.
3. `Look_up_building_in_cell`.
4. If null, call `PathType__Has_Valid_Steps`.
5. If valid steps, send radio `3` from the unit.
6. Queue mission `10` with immediate flag `1`.
7. Return the normal mission timer plus random `0..2`.

No storage drain, no credit add, no refinery anim slot clear, no `+0x6D1` clear, no `ReleaseDockedHarvester`, no radio `0x07`, and no radio `0x19` occur on this branch.

**Active in YR:** Yes for stock HARV/CMIN state-3 unload when the adjacent refinery lookup cell no longer contains a building.

## Current Rust Implementation Status

Current active Rust has already moved toward this branch shape:

- `src/sim/miner/miner_system.rs`: `handle_return` clears invalid reserved refinery, clears queue state, preserves cargo, and keeps a full miner in `ReturnToRefinery`; `find_nearest_refinery` and `refinery_dock_for_sid` skip `dying` / zero-health refineries.
- `src/sim/miner/miner_dock_sequence.rs`: `resolve_refinery_cells` skips `dying` / zero-health refineries; `abort_invalid_refinery` clears reservation/contact, movement target, visual override, pivot/exit/unload timers, preserves cargo, and returns full cargo to refinery selection. `interrupt_refinery_docked_miners` exists for sell-side docked-miner interruption.
- `src/sim/production/production_sell.rs`: `sell_building` calls `interrupt_refinery_docked_miners` before removing the building entity.
- `src/sim/miner/miner_tests.rs`: current tests include `full_miner_losing_dying_refinery_keeps_returning` and `dying_refinery_aborts_unload_without_credit_or_stuck_visual`.

Rust delta against binary: cargo preservation and no-credit-on-dying-refinery are aligned with the verified branch. Visual cleanup intentionally diverges from the static binary if it clears the override immediately on the missing-building branch; that may be a desired player-visible correction only if runtime capture proves gamemd clears the render flag elsewhere. Static binary evidence says it does not clear `+0x6D1` in this branch.

No tests were run for this report because the assignment allowed writing only this report file and test execution would create build artifacts.

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Mission_Deploy_Building` state-3 missing-building branch | verified | `0x0073E306..0x0073E350` | runtime frame visibility only |
| Cargo preservation on missing branch | verified | no storage calls on null branch; storage calls only non-null branch | none |
| Optional radio `3` before mission queue | verified | `0x0073E313..0x0073E322` | exact radio recipient semantics covered by radio reports |
| `Queue_Mission(10,1)` / `Commence` field writes | verified | `0x005B35E0`, `0x005B3570` | none for branch |
| `+0x6D1` visual flag not cleared by missing branch | verified | `0x0073E306..0x0073E350`, `0x005B35E0`, `0x005B3570` | runtime screenshot/frame capture |
| `DrawExtras` UnloadingClass gate | verified | `0x0073CEC0` | none for gate |
| `BuildingClass::Sell` interrupt call | verified | `0x00449C30` | exact final map-removal frame outside scope |
| `ReceiveDamage` death interrupt call | verified | `0x00442230`, call at `0x004424EA` | exact combat tick interleaving outside scope |
| `UndockUnit` cleanup order | verified | `0x004593A0`, `0x00459450..0x00459462` | none |
| `Mission_Harvest` state 0 full check | verified | `0x0073E700..0x0073E720` | none |
| Current Rust exact parity | touched-not-exhausted | code scan only | run focused tests after implementation session |

## Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the missing-refinery branch live for standard YR HARV/CMIN? -> Yes; stock units/refineries have the required Harvester/DockUnload/Refinery keys.` (evidence: `rulesmd.ini` `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`; `0x0073D630`)
- `[RESOLVED] OQ-2 - Does the null branch drain cargo or award credits? -> No; it queues Harvest and skips storage/credit calls.` (evidence: `0x0073E306..0x0073E330`)
- `[RESOLVED] OQ-3 - Is radio cleanup before or after mission queue? -> Before; optional radio `3` precedes `Queue_Mission(10,1)`.` (evidence: `0x0073E31E..0x0073E330`)
- `[RESOLVED] OQ-4 - Does missing branch clear unload visual flag? -> No static write clears `+0x6D1` on this branch, nor in Queue/Commence.` (evidence: `0x0073E306..0x0073E350`, `0x005B35E0`, `0x005B3570`)
- `[RESOLVED] OQ-5 - What gates the visual swap? -> `Harvester=yes`, `unit+0x6D1`, and `UnloadingClass`; no mission gate.` (evidence: `0x0073CEC0`)
- `[RESOLVED] OQ-6 - Does sell call interrupt cleanup before removal? -> Yes when `building+0x2E4 != 0`.` (evidence: `0x00449C30`)
- `[RESOLVED] OQ-7 - Does damage death call interrupt cleanup? -> Yes in result case 4 when `building+0x2E4 != 0`.` (evidence: `0x004424EA`)
- `[RESOLVED] OQ-8 - Does `UndockUnit` clear both sides? -> Yes, unit then building, then radio `3`.` (evidence: `0x00459450..0x00459462`)
- `[RESOLVED] OQ-9 - Is another refinery immediately selected? -> Only after re-entered Harvest sees full storage and transitions to state 2; partial cargo resumes ore-search path first.` (evidence: `0x0073E700..0x0073E720`, `0x0073E5E0` state 2)
- `[DEFERRED] OQ-10 - Exact render frame count for stale `+0x6D1` after refinery removal.` (category: `needs-runtime-debugger`; reason: static code proves no clear, but frame capture is needed for exact visible duration; next-step-if-pursued: runtime trace destroy/sell during state-3 unload while observing CMIN/HORV render)

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Missing state-3 refinery preserves cargo, skips credits, clears contact/reservation concept, and queues Harvest; full cargo then returns to refinery selection. | `0x0073E306..0x0073E330`; `0x0073E700..0x0073E720` | mostly aligned in active work; tests present, not run | `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs` | Invalid/missing/dying refinery must not drain cargo or credit owner; full miner must not enter ore search before selecting another refinery. | `test_miner_missing_refinery_preserves_cargo_and_returns_to_harvest` | Do not clear cargo or send a full miner to ore search after its chosen refinery disappears. |
| Dying or zero-health refinery must be treated as unavailable even while the entity remains in `EntityStore`. | `ReceiveDamage @ 0x00442230`; `UndockUnit @ 0x004593A0`; current Rust scan | active Rust appears aligned for lookup helpers | `resolve_refinery_cells`, `refinery_dock_for_sid`, `find_nearest_refinery`, dock-reservation cleanup | Reject dying refineries for new selection and abort active unload before crediting. | `dying_refinery_aborts_unload_without_credit_or_stuck_visual` | Do not use mere entity existence as refinery liveness. |
| Static binary does not clear `+0x6D1` on the null-refinery state-3 branch; `DrawExtras` would still satisfy the UnloadingClass gate until another path clears it. | `0x0073E306..0x0073E350`; `0x005B35E0`; `0x005B3570`; `0x0073CEC0` | active Rust clears `display_type_override` on invalid-refinery abort | `abort_invalid_refinery`, visual override handling | Decide deliberately: either match static binary stale visual or require runtime evidence before clearing as parity. | proposed: `test_miner_missing_refinery_static_branch_keeps_or_clears_unloading_visual_after_runtime_verification` | Do not claim gamemd clears the unload visual on missing-refinery abort from static evidence; it does not. |

## Negative Facts / Do Not Do

- Do not call `ReleaseDockedHarvester` or Force_Track `0x47` for the stock zero-link missing-building state-3 branch.
- Do not send radio `0x07` or `0x19`; the verified branch sends only optional radio `3`.
- Do not award credits after `Look_up_building_in_cell` returns null.
- Do not assume another refinery is always selected immediately; partial cargo re-enters Harvest state 0 and may search ore first.
- Do not say the missing-building branch clears `unit+0x6D1` or the UnloadingClass render gate.

## Stale Docs / Follow-Up Wording

- Replace any claim that state-3 missing-refinery abort "clears the unload visual" with: "Static binary evidence shows the null-refinery branch queues Mission_Harvest and skips cargo drain, but does not clear `unit+0x6D1`; `DrawExtras` gates UnloadingClass on `+0x6D1`, so visual cleanup requires a later path or runtime-proven side effect."
- Replace broad "refinery destroyed mid-unload calls UndockUnit" wording with: "`UndockUnit` is called by sell/damage only when `building+0x2E4` is nonzero. Stock zero-link CMIN/HARV state-3 unload uses the unit-side missing-building branch instead."
- Treat `miner/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` and older `HARVESTER_DOCK_UNLOAD*.md` claims about normal exit, `ReleaseDockedHarvester`, radio `0x07`, or radio `0x19` as stale unless reconciled against the newer zero-link reports.

## Remaining Uncertainty

- Exact number of rendered frames for stale UnloadingClass after the missing-building branch needs runtime debugger/screenshot confirmation.
- Exact same-tick ordering between combat death animation placement/removal and the next unit mission dispatch is not proven by this static slice; static evidence proves the branch effects once each function runs.

## Sources

- Ghidra read-only decompile: `0x0073D630`, `0x004593A0`, `0x00449C30`, `0x00442230`, `0x0073E5E0`, `0x005B35E0`, `0x005B3570`, `0x0073CEC0`, `0x005B3A00`.
- Ghidra read-only assembly context: `0x0073E306`, `0x0073E313`, `0x0073E31E`, `0x0073E330`, `0x004424EA`, `0x00459450`, `0x0045945C`, `0x00459462`.
- Prior docs: `miner/traces/MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md`, `miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `miner/BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`, `miner/REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`, `UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md`, `miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_MISSION_HARVEST_STATE2_RETURN_BRANCH_COORDS_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini` `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`.
- Rust scan only: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/production/production_sell.rs`, `src/sim/miner/miner_tests.rs`.
