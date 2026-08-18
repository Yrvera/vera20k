# Refinery Sold/Destroyed Mid-Unload Runtime Effects - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x0073CEC0`, `0x00737430`, `0x00442230`, `0x00449C30`, `0x004593A0`, `0x005B35E0`, `0x005B3570`, `0x0047C520`, `0x0073E5E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR stock `HARV`/`CMIN` when a stock refinery is sold or destroyed while the miner is returning, linked/on-pad, or in the unit-side state-3 dock-unload loop. Covered surfaces are cargo preservation, credit cutoff, contact cleanup, dock release/abort, mission fallback, and unloading display override state.  
**Non-Scope:** healthy unload cadence, healthy two-miner handoff, slave miner/Yuri deploy-miner behavior, exact render-frame capture, and broad radio protocol outside this refinery-loss slice.  
**Confidence:** High for static binary branch/order, cargo/credit/contact/display-flag gates, and current Rust source comparison; Medium for exact visible stale `HORV`/`CMON` frame count because that needs runtime capture.  
**Active in YR:** Yes. Stock `[HARV]` and `[CMIN]` have `Harvester=yes`, `Dock=NAREFN,GAREFN`, `Storage=40/20`, and `UnloadingClass=HORV/CMON`; `[NAREFN]` and `[GAREFN]` have `Refinery=yes`, `DockUnload=yes`, and `NumberOfDocks=1`.

## 1. Overview

There are two live refinery-loss paths, and they must not be collapsed. If a reciprocal `building+0x2E4` link exists, building sell/death first call `BuildingClass::UndockUnit`, which stops/redirects the miner, clears both `+0x2E4` links, sends radio `3`, and does not touch storage or credits.

The standard stock zero-link state-3 unload path instead rediscovers the refinery from the miner cell plus the west-neighbor lookup. If the cell lookup returns null, the unit optionally sends radio `3`, queues `Mission_Harvest`, and exits before all storage drain and credit calls. Static code does not clear `unit+0x6D1` on that null-refinery branch; radio `0x17` and normal state 4 are proven clear paths.

## 2. Class Layout / Key Offsets

| Offset / field | Owner | Meaning in this slice | Evidence |
|---|---|---|---|
| `+0x2E4` | building/unit | reciprocal dock-link pointer for linked interrupt path | `0x004593A0`, sell/death callers |
| `+0x6D1` | unit byte | unloading display flag consumed by `DrawExtras` | set `0x0073DFDA`, clear `0x0073E1F6`, radio clear `0x00737AC9` |
| `+0x6C4` / `+0x1B1*4` | unit | current UnitType pointer, temporarily swapped to `UnloadingClass` for draw | `0x0073CEC0` |
| UnitType `+0x6B8` | unit type | resolved `UnloadingClass` pointer | `DrawExtras @ 0x0073CEC0` |
| UnitType `+0xE0E` | unit type | `Harvester=yes` gate | `Mission_Deploy_Building`, `DrawExtras`, INI |
| UnitType `+0xE0F` | unit type | TS `Weeder` branch; not stock HARV/CMIN | `0x0073E6F1`, INI absence |
| Unit `+0x33C` | unit | harvester storage drained only in found-refinery branch | `0x0073E355+` storage calls |
| Unit `+0xF8` | unit | dump-rate accumulator | gate at `0x0073E355..0x0073E36F` |
| Mission `+0xAC/+0xB4/+0xB8/+0xBC` | mission object | current mission, queued mission, queued flag, substate | `0x005B35E0`, `0x005B3570` |
| `CellClass+0xE4` | cell | linked object list scanned by building lookup | `0x0047C520` |

## 3. Core Logic

### 3.1 Linked sell/destroy path

`BuildingClass::Sell @ 0x00449C30` checks `building+0x2E4` in sell state 0. If non-null, it calls `BuildingClass::UndockUnit @ 0x004593A0` before the later sell/removal work. Evidence: `0x0044AAA4` loads `+0x2E4`, `0x0044AAAC` branches around the helper on null, and `0x0044AAB0` calls `0x004593A0`.

`BuildingClass::ReceiveDamage @ 0x00442230` result case 4 does the same for destruction. It removes the linked unit from the local contact list, then calls `UndockUnit` before capture cleanup, chrono deploy cleanup, death effects, and removal/death-animation work. Evidence: linked-list handling around `0x004424A8` and call at `0x004424EA`.

`UndockUnit` behavior is exact:

1. Read the unit from `building+0x2E4`.
2. Return if null.
3. If `WhatAmI()==1`/drive unit path, stop its locomotor.
4. Head the locomotor toward track `0x47` using building coords plus `(-0x80,+0x80,0)`.
5. Set speed to double `1.0`.
6. Clear `unit+0x2E4`.
7. Clear `building+0x2E4`.
8. Send radio command `3` from the building.

The helper has no `StorageClass::RemoveAmount`, `StorageClass::GetAmount`, `HouseClass::Add_Tiberium_Credits`, or `unit+0x6D1` write. Evidence: decompile of `0x004593A0`; link clear and radio at `0x00459450`, `0x0045945C`, `0x00459462`.

### 3.2 Zero-link state-3 refinery missing branch

`UnitClass::Mission_Deploy_Building @ 0x0073D630` state 3 recomputes the refinery lookup cell each deposit pass:

1. Get unit cell (`0x0073E2C8`).
2. Add `g_refinery_unload_adjacent_lookup_dx/dy` (`0x0073E2D5`, `0x0073E2DC`; verified stock offset is west-neighbor).
3. Get the `CellClass`.
4. Call `Look_up_building_in_cell @ 0x0047C520` (`0x0073E306`).
5. If non-null, jump to the dump/credit branch (`0x0073E30F -> 0x0073E355`).

If the lookup is null, the branch is:

1. `PathType::Has_Valid_Steps` (`0x0073E313`).
2. If true, send radio `3` through unit vtable `+0x274` (`0x0073E31E..0x0073E322`).
3. Queue mission `0x0A`/Harvest with immediate flag `1` (`0x0073E32A..0x0073E330`).
4. Enter mission timer epilogue (`0x0073E338+`).

The dump-rate accumulator and all cargo/credit work start only at the non-null branch (`0x0073E355+`). Therefore the null-refinery state-3 branch cannot award credits and cannot remove cargo.

### 3.3 Lookup semantics

`Look_up_building_in_cell @ 0x0047C520` scans `CellClass+0xE4` object links and returns the first object whose vtable `+0x2C` reports `WhatAmI()==6`. It does not inspect health or a dying flag itself. Stock liveness comes from sell/destruction object removal or linked interrupt order; Rust should continue treating `dying` and zero-health refineries as invalid before deposit.

### 3.4 Mission fallback after abort

`MissionClass::Queue_Mission @ 0x005B35E0` writes queued mission fields (`+0xB4`, queued flag) and can call `Commence`; it does not write `+0x6D1`. `MissionClass::Commence @ 0x005B3570` moves queued mission to current mission, resets substate/timers, and clears the queued flag; it also does not write `+0x6D1`.

The queued mission is Harvest (`0x0A`). On the next `UnitClass::Mission_Harvest @ 0x0073E5E0` state-0 pass, non-Weeder full storage calls vtable `+0x2B4`, compares against `1.0`, writes substate `2`, and returns before ore search. Evidence: `0x0073E700`, `0x0073E706`, `0x0073E714`. Full cargo therefore returns to refinery selection; partial cargo can resume normal ore-search/harvest logic.

### 3.5 Display override and stale visual risk

`UnitClass::DrawExtras @ 0x0073CEC0` gates the unloading body swap on:

1. current UnitType `Harvester=yes` (`+0xE0E != 0`);
2. `unit+0x6D1 != 0`;
3. current UnitType has `UnloadingClass` pointer (`+0x6B8 != 0`).

It then temporarily writes `unit+0x6C4 = UnitType+0x6B8`, draws using the override type, and restores the original type. There is no mission/substate gate in the swap.

Clear paths verified in this slice:

- normal state 4 clear: `0x0073E1F6` writes `unit+0x6D1 = 0`;
- radio `0x17` clear: `UnitClass::Receive_Radio @ 0x00737430`, case `0x17`, writes `unit+0x6D1 = 0` at `0x00737AC9` when Harvester/Weeder and flag set.

Not a clear path:

- zero-link state-3 null-refinery branch (`0x0073E306..0x0073E350`);
- `Queue_Mission`;
- `Commence`;
- `UndockUnit`.

So the exact stale `HORV`/`CMON` render count after a zero-link null-refinery abort is runtime-only. Static evidence proves no immediate clear in that branch, but not how many frames are actually presented before another live path clears or redraw state changes.

## 4. INI Keys

| Key | Stock YR value | Effect | Active in YR |
|---|---|---|---|
| `[HARV] Harvester` | `yes` | reaches harvester mission/render gates | Yes |
| `[HARV] Dock` | `NAREFN,GAREFN` | refinery candidates | Yes |
| `[HARV] Storage` | `40` | full cargo threshold and storage source | Yes |
| `[HARV] UnloadingClass` | `HORV` | unload visual type | Yes |
| `[CMIN] Harvester` | `yes` | reaches same harvester mission/render gates | Yes |
| `[CMIN] Dock` | `NAREFN,GAREFN` | refinery candidates | Yes |
| `[CMIN] Storage` | `20` | full cargo threshold and storage source | Yes |
| `[CMIN] UnloadingClass` | `CMON` | unload visual type | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | refinery identity | Yes |
| `[GAREFN]/[NAREFN] DockUnload` | `yes` | building handoff to unload mission | Yes |
| `[GAREFN]/[NAREFN] NumberOfDocks` | `1` | stock single dock capacity | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | zero-link unload FSM and null-refinery abort | decompile + assembly context | Yes |
| `Look_up_building_in_cell @ 0x0047C520` | building lookup from cell object list | decompile | Yes |
| `BuildingClass::Sell @ 0x00449C30` | linked interrupt before sell/removal | `0x0044AAA4..0x0044AAB0` | Yes |
| `BuildingClass::ReceiveDamage @ 0x00442230` | linked interrupt in death result case | `0x004424A8..0x004424EA` | Yes |
| `BuildingClass::UndockUnit @ 0x004593A0` | conditional linked dock cleanup | decompile | Conditional |
| `UnitClass::Receive_Radio @ 0x00737430` | radio `0x17` display clear; radio `3` generic contact break path | decompile | Yes |
| `UnitClass::DrawExtras @ 0x0073CEC0` | `UnloadingClass` visual swap | decompile | Yes |
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | full-cargo fallback into return/refinery selection | `0x0073E700..0x0073E720` | Yes |

## 6. Current Rust Implementation Status

Current Rust source scan only, no edits:

- `src/sim/miner/miner_dock_sequence.rs::resolve_refinery_cells` returns `None` for missing, `dying`, or zero-health refineries.
- `src/sim/miner/miner_dock_sequence.rs::abort_invalid_refinery` cancels dock reservation, clears reserved refinery/queue/unload timers/exit cache, preserves cargo, and routes full miners to `ReturnToRefinery`.
- `src/sim/miner/miner_dock_sequence.rs::interrupt_refinery_docked_miners` approximates linked `UndockUnit`: cancel contact/on-pad state, clear dock fields, preserve cargo, and only start the force-track shape for on-pad miners.
- `src/sim/miner/miner_dock_sequence.rs::phase_unloading` credits the refinery owner only after `handle_dock_sequence` has resolved a live refinery; current invalid-refinery dispatch aborts before slot drain.
- `src/sim/production/production_sell.rs::sell_building` calls `interrupt_refinery_docked_miners` before removing a sold refinery.
- Tests already present include `full_miner_losing_dying_refinery_keeps_returning` and `dying_refinery_aborts_unload_without_credit_or_stuck_visual`.

Rust intentionally clears `display_type_override` immediately on invalid-refinery abort. Static gamemd evidence does not prove that for the zero-link null-refinery branch; keep the immediate clear as a practical visual behavior only if runtime capture later confirms no visible stale override, or document it as an intentional divergence.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Linked sell caller | verified | `0x0044AAA4..0x0044AAB0` | none |
| Linked death caller | verified | `0x004424A8..0x004424EA` | exact live same-frame death-animation ordering |
| `UndockUnit` link cleanup/no credit | verified | `0x004593A0`, `0x00459450..0x00459462` | none |
| Zero-link state-3 null-refinery branch | verified | `0x0073E306..0x0073E350` | runtime stale visual count |
| Storage/credit cutoff | verified | null branch skips `0x0073E355+` drain block | none |
| `Look_up_building_in_cell` semantics | verified | `0x0047C520` | no health/dying check by helper |
| `Queue_Mission`/`Commence` writes | verified | `0x005B35E0`, `0x005B3570` | none for `+0x6D1` |
| `DrawExtras` unloading gate | verified | `0x0073CEC0` | runtime presentation count |
| Radio `0x17` display clear | verified | `0x00737AC9` | exact recipient list for every sell/death variation |
| Mission Harvest full-cargo fallback | verified | `0x0073E700..0x0073E720` | none |
| Current Rust surfaces | touched-not-exhausted | source scan | run focused tests in implementation pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this path active in standard YR? -> Yes for stock HARV/CMIN and stock GAREFN/NAREFN.` (evidence: `rulesmd.ini`; `0x0073D630`)
- `[RESOLVED] OQ-02 - Does linked sell call cleanup before removal? -> Yes, `+0x2E4` guard then `UndockUnit`.` (evidence: `0x0044AAA4..0x0044AAB0`)
- `[RESOLVED] OQ-03 - Does linked destruction call cleanup before teardown? -> Yes in damage result case 4 when `+0x2E4 != 0`.` (evidence: `0x004424A8..0x004424EA`)
- `[RESOLVED] OQ-04 - Does `UndockUnit` drain/credit cargo? -> No.` (evidence: decompile `0x004593A0`)
- `[RESOLVED] OQ-05 - Does `UndockUnit` clear `+0x6D1`? -> No static write in the helper.` (evidence: decompile `0x004593A0`)
- `[RESOLVED] OQ-06 - Does zero-link state-3 null-refinery branch drain or credit? -> No; it branches before the storage/credit block.` (evidence: `0x0073E306..0x0073E355`)
- `[RESOLVED] OQ-07 - What mission follows null-refinery abort? -> optional radio `3`, then queued Harvest `0x0A` with immediate flag `1`.` (evidence: `0x0073E313..0x0073E330`)
- `[RESOLVED] OQ-08 - Does Queue/Commence clear display flag? -> No `+0x6D1` write in either function.` (evidence: `0x005B35E0`, `0x005B3570`)
- `[RESOLVED] OQ-09 - What clears the unload display flag? -> normal state 4 and radio `0x17` are verified clear paths.` (evidence: `0x0073E1F6`, `0x00737AC9`)
- `[RESOLVED] OQ-10 - What gates drawing HORV/CMON? -> Harvester type, `+0x6D1`, and `UnloadingClass`; no mission/substate gate.` (evidence: `0x0073CEC0`)
- `[RESOLVED] OQ-11 - Does full cargo after abort return to refinery selection? -> Yes, Harvest state 0 full check writes substate 2 before ore search.` (evidence: `0x0073E700..0x0073E720`)
- `[RESOLVED] OQ-12 - Does Rust still have tests for dying refinery fallback/no-credit/no-stuck visual? -> Yes.` (evidence: `src/sim/miner/miner_tests.rs`)
- `[DEFERRED] OQ-13 - Exact rendered stale `HORV`/`CMON` frame count after zero-link null-refinery abort.` (category: `needs-runtime-debugger`; reason: static code proves no immediate clear, but not presented-frame count; next-step-if-pursued: runtime trace/capture sell and kill during state-3 unload)
- `[DEFERRED] OQ-14 - Exact same-frame order between combat death animation removal and next miner mission dispatch.` (category: `needs-runtime-debugger`; reason: static caller order is verified, frame interleaving is not; next-step-if-pursued: non-breaking trace `ReceiveDamage`, `UndockUnit`, `Mission_Deploy_Building`)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset/type | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `UnitClass::DrawExtras @ 0x0073CEC0` | `Type+0xE0E`, `unit+0x6D1`, `Type+0x6B8` | `HORV` or `CMON` | normal unit draw anchor | normal unit draw path | while flag remains set | unloading body override |
| 2 | `UnitClass::DrawExtras @ 0x0073CEC0` | original type saved/restored around draw | `HARV` or `CMIN` | same | same | always after draw | restore gameplay type |

| Asset/type | Loaded | Drawn | Visible in target | Role | Evidence |
|---|---|---|---|---|---|
| `HARV` | yes | yes when no `HORV` override | normal miner | content | rules/art INI; draw gate |
| `HORV` | yes | yes while `HARV +0x6D1` gate passes | possible stale after null abort | content override | `UnloadingClass=HORV`; `0x0073CEC0` |
| `CMIN` | yes | yes when no `CMON` override | normal miner | content | rules/art INI; draw gate |
| `CMON` | yes | yes while `CMIN +0x6D1` gate passes | possible stale after null abort | content override | `UnloadingClass=CMON`; `0x0073CEC0` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Zero-link state-3 missing refinery preserves cargo, awards no credits, optionally sends radio `3`, then queues Harvest. | `0x0073E306..0x0073E350` | mostly aligned; tests present | `miner_dock_sequence::handle_dock_sequence`, `resolve_refinery_cells`, `abort_invalid_refinery`, `phase_unloading` | Abort before slot drain whenever refinery is missing/dying/zero-health; preserve cargo and do not credit. | `cmin_mid_unload_refinery_sold_preserves_cargo_no_credit_returns_to_refinery` | Do not credit the removed refinery owner or clear cargo after failed lookup. |
| Linked sell/destroy cleanup clears dock/contact state through `UndockUnit` shape, not healthy completion/promotion. | `0x0044AAA4..0x0044AAB0`, `0x004424A8..0x004424EA`, `0x004593A0` | partial model present | `interrupt_refinery_docked_miners`, `RefineryDockContacts`, sell and combat/C4 destruction hooks | Cancel on-pad/contact/waiting state before refinery removal; preserve cargo; do not run healthy handoff on a removed refinery. | `cmin_linked_refinery_destroy_undocks_preserves_cargo_clears_contact` | Do not promote a waiting miner into a refinery being sold/destroyed. |
| Static binary does not clear `+0x6D1` on zero-link null-refinery abort; radio `0x17` and state 4 do clear it. | `0x0073E306..0x0073E350`, `0x00737AC9`, `0x0073E1F6`, `0x0073CEC0` | Rust clears `display_type_override` immediately on invalid-refinery abort | `display_type_override`, `abort_invalid_refinery`, render selection | Treat immediate visual clear as runtime-needing, not statically proven. Keep current Rust test if runtime capture validates it; otherwise adjust to stale-frame behavior. | `cmin_refinery_loss_unloading_visual_runtime_frame_count` | Do not write docs saying the null-refinery branch itself clears `HORV`/`CMON`. |

## 11. Negative Facts / Do Not Do

- Do not award credits or drain storage after the state-3 lookup returns null.
- Do not use mere entity existence as refinery liveness in Rust; `dying` and zero-health must abort before deposit.
- Do not send full miners to ore search after refinery loss; full Harvest state 0 goes back to return/refinery selection.
- Do not run healthy two-miner handoff/promotion for a refinery being sold or destroyed.
- Do not model this loss branch with radio `0x07`, radio `0x15`, radio `0x19`, normal `ReleaseDockedHarvester`, or healthy state-4 Force_Track effects.
- Do not claim static gamemd clears `+0x6D1` in the zero-link null-refinery branch.

## 12. Remaining Uncertainty

- Exact number of rendered stale `HORV`/`CMON` frames after zero-link null-refinery abort remains runtime-only.
- Exact same-frame ordering between combat death animation/cell-list removal and the miner's next mission tick remains runtime-only.
- Death-result close-contact damage side effects were recognized but not expanded because this slot is bounded to refinery-loss abort effects.

## 13. Stale Docs / Follow-Up Docs

- Replace any stale wording saying "state-3 missing refinery clears the unload visual" with: "Static binary evidence shows the zero-link state-3 null-refinery branch does not clear `unit+0x6D1`; normal state 4 and radio `0x17` are verified clear paths. Exact stale `HORV`/`CMON` frame count after null-refinery abort requires runtime capture."
- Replace broad "destroyed refinery calls `UndockUnit`" wording with: "`UndockUnit` is called by sell/death only when `building+0x2E4 != 0`; stock zero-link DockUnload uses the unit-side null-refinery branch when the adjacent building lookup fails."
- Keep current Rust no-credit/cargo-preservation tests, but do not cite them as proof of the native visual clear.

## Sources

- Ghidra read-only decompiled: `UnitClass__Mission_Deploy_Building @ 0x0073D630`, `UnitClass__DrawExtras @ 0x0073CEC0`, `UnitClass__Receive_Radio @ 0x00737430`, `BuildingClass__ReceiveDamage @ 0x00442230`, `BuildingClass__Sell @ 0x00449C30`, `BuildingClass__UndockUnit @ 0x004593A0`, `MissionClass__Queue_Mission @ 0x005B35E0`, `MissionClass__Commence @ 0x005B3570`, `Look_up_building_in_cell @ 0x0047C520`, `UnitClass__Mission_Harvest @ 0x0073E5E0`.
- Ghidra read-only assembly context: `0x0073E2C8`, `0x0073E306`, `0x0073E30D`, `0x0073E313`, `0x0073E31E`, `0x0073E330`, `0x0073E355`, `0x0073E1F6`, `0x0073DFDA`, `0x0044AAA4`, `0x0044AAB0`, `0x0044AB5A`, `0x0044AB68`, `0x004424A8`, `0x004424EA`, `0x0044259D`, `0x004425AA`, `0x00459450`, `0x0045945C`, `0x00459462`, `0x00737AC9`, `0x0073E700`, `0x0073E714`, `0x0073EB7E`, `0x0073EC50`.
- Prior docs checked: `miner/REFINERY_DESTROYED_OR_SOLD_MID_UNLOAD_CONTACTS_DISPLAY_CREDITS_GHIDRA_REPORT.md`, `miner/MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`, `miner/HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md`, `miner/HARV_DESTROYED_REFINERY_UNLOAD_ABORT_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_tests.rs`, `src/sim/production/production_sell.rs`, `src/sim/combat/mod.rs`, `src/sim/world/world_orders.rs`.

**Status:** COMPLETE
