# Missing West-Cell Building During State-3 Unload Trace

**Trace slot:** production/miner-refinery rediscovery slot 3  
**Scenario:** `UnitClass::Mission_Deploy_Building` state 3, miner current cell + `(-1,0)` building lookup returns null.  
**Report date:** 2026-05-27  
**Scope:** cargo preservation, no credit/deposit event, no miner-owner fallback, harvest/return handoff, and Rust reservation/contact release. Display-latch handling is noted only because this branch has an explicit recent Rust test; runtime frame count remains slot-4 territory.

## Status

**Overall status:** PARTIAL. Ghidra MCP was available, but `list_instances` returned no running Ghidra instances, so no fresh decompile could be made in this session. The gamemd side below relies on existing verified Ghidra reports in `docs/research/miner/`, plus INI activation evidence. No Rust, INI, or non-trace docs were edited.

## Active-YR Gate

Stock YR activation is verified by INI and prior Ghidra reports:

- `[HARV]` has `Harvester=yes` and `UnloadingClass=HORV` at `ini/rulesmd.ini:8215`, `:8228`, `:8246`.
- `[CMIN]` has `Harvester=yes` and `UnloadingClass=CMON` at `ini/rulesmd.ini:7351`, `:7364`, `:7384`.
- `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` at `ini/rulesmd.ini:11722`, `:11726-11729`, `:12515`, `:12519-12521`.
- Prior verified reports identify this as active stock `Mission_Deploy_Building @ 0x0073D630`, not TS dormant behavior.

Verdict: PASS.

## Pipeline

1. Trigger: miner is in Rust `MinerState::Dock` / `RefineryDockPhase::Unloading`, matching gamemd state-3 unload.
2. Rediscovery lookup: Rust calls `mission_deploy_unload_building`, using miner current cell `(rx - 1, ry)` and first live structure in that occupancy list.
3. Null branch: if lookup returns `None`, Rust calls `abort_missing_unload_building`.
4. State effects: Rust cancels reservation/contact for `reserved_refinery`, clears Rust dock timers, preserves cargo, emits no credits/events, and chooses the next miner state from cargo fullness.
5. Player result: no deposit payoff occurs; the miner leaves the stale unload relationship and resumes the harvest/refinery loop according to cargo state.

## Stage Verdicts

### Stage 1 - West-Cell Null Lookup

Rust: `mission_deploy_unload_building` reads the miner entity, returns `None` at map edge, otherwise checks `(miner.position.rx - 1, miner.position.ry)` and scans the miner's movement-layer occupancy list for the first live `EntityCategory::Structure` (`src/sim/miner/miner_dock_sequence.rs:432-456`).

gamemd: prior verified Ghidra reports show state 3 computes the current unit cell plus `DAT_0089F6A0/2 == (-1,0)`, calls `MapClass::Get_CellClass`, then `Look_up_building_in_cell`; if that returns null it branches around the drain/credit block (`HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md`, section 3.4; `MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`, Key Binary Findings).

Verdict: PASS for the concrete null scenario. First-building ordering when non-null belongs to slot 1/2.

### Stage 2 - Cargo Preservation

Rust: cargo removal only occurs after `Some(unload_building_id)` at `phase_unloading`; the `None` branch calls `abort_missing_unload_building` and returns before `snap.miner.cargo.retain(...)` (`src/sim/miner/miner_dock_sequence.rs:1045-1059`). The focused test asserts one cargo bale remains after the null lookup (`src/sim/miner/miner_tests.rs:4677-4705`).

gamemd: the verified null branch calls optional radio `3`, queues Harvest, and reaches the timer epilogue; `StorageClass::FindFirstNonEmptySlot`, `RemoveAmount`, and credit calls are only reachable in the non-null branch (`HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md`, section 3.4).

Verdict: PASS.

### Stage 3 - No Credits, No Deposit Event, No Miner-Owner Fallback

Rust: credits and `BaleDepositEvent` are emitted only after a non-null west-cell building id (`src/sim/miner/miner_dock_sequence.rs:1068-1101`). The null-lookup test captures unchanged credits and empty `sim.bale_events` (`src/sim/miner/miner_tests.rs:4698-4705`). Because the null branch exits before owner lookup, it cannot fall back to the miner owner.

gamemd: no `HouseClass::Add_Tiberium_Credits` call is reachable when `Look_up_building_in_cell` returns null, so no owner context is used in this branch (`HARV_DESTROYED_REFINERY_UNLOAD_ABORT_GHIDRA_REPORT.md`, sections 1 and 3.4).

Verdict: PASS.

### Stage 4 - Reservation/Contact Cleanup

Rust: `abort_missing_unload_building` calls `dock_reservations.cancel_miner(ref_sid, miner)` (`src/sim/miner/miner_dock_sequence.rs:626-629`). `cancel_miner` releases pad, contact, contact-entered, and waiter state through `release_on_pad` and `release_contact` (`src/sim/miner/miner_dock.rs:124-136`). Rust then clears `reserved_refinery`, `dock_queued`, dock phase/timers, and movement/drive-track fields (`src/sim/miner/miner_dock_sequence.rs:631-653`).

gamemd: the null branch optionally sends radio `3` before queuing `Mission_Harvest`, and does not use the normal state-4 completion path. Rust's reservation/contact maps are internal queue bookkeeping; the important player-facing outcome is avoiding stale refinery queues after the abort.

Verdict: PASS for Rust stale-queue avoidance. Exact equivalence of Rust's internal queue fields to gamemd radio/contact bytes is UNCHECKED.

### Stage 5 - Harvest/Return Handoff

Rust: after cleanup, `dock_abort_state` returns `ReturnToRefinery` if cargo is full, otherwise `SearchOre`, preserving `ForcedReturn` if set (`src/sim/miner/miner_dock_sequence.rs:464-471`, `:650-653`). The focused full-cargo test asserts full cargo and `ReturnToRefinery` after one tick (`src/sim/miner/miner_tests.rs:4707-4738`).

gamemd: the null branch calls `SetMission(Mission_Harvest=0x0A, queued=1)`; on Harvest state 0, full storage immediately enters return state 2, while partial cargo continues through normal ore-search/harvest logic (`HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md`, sections 3.1, 3.2, 3.4).

Verdict: PASS for the player-visible full-cargo return outcome and partial-cargo no-credit/no-cargo-loss outcome. UNCHECKED for exact same-tick mission/substate identity because fresh Ghidra/runtime timing was unavailable.

### Stage 6 - Display Latch in This Branch

Rust: `abort_missing_unload_building` does not clear `entity.display_type_override`; it only clears facing, movement, drive-track, dock timers, reservation/contact, and state (`src/sim/miner/miner_dock_sequence.rs:631-653`). The focused test asserts the override remains set after the null lookup (`src/sim/miner/miner_tests.rs:4740-4768`).

gamemd: prior verified static evidence says the null branch does not write `unit+0x6D1`, and `Queue_Mission`/`Commence` do not clear it (`MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`, Key Binary Findings). Exact rendered-frame count is runtime-only and belongs to slot 4.

Verdict: PASS for "null branch itself does not clear latch"; UNCHECKED for runtime frame count.

## Findings

No player-visible FAIL or NOT-IMPLEMENTED finding was found for this concrete slot.

The implementation is aligned on the recent requirements that matter here:

- Missing west-cell building preserves cargo.
- Missing west-cell building emits no credits or `BaleDepositEvent`.
- Missing west-cell building has no miner-owner fallback path.
- Rust releases `reserved_refinery` contact/queue bookkeeping enough to avoid stale queues.
- The null branch itself does not clear the unload display override.

## Adjacent Findings

- Fresh Ghidra MCP verification could not be performed because no Ghidra instance was connected. Existing verified Ghidra reports were used instead.
- Rust's high-level state names (`ReturnToRefinery` / `SearchOre`) do not literally expose gamemd's queued `Mission_Harvest(0x0A, queued=1)` intermediate state. The sampled full/partial outcomes match the expected visible branch, but exact byte/tick identity remains UNCHECKED without live binary/runtime comparison.
- Non-null first-object ordering, state-4 `Refinery=yes`, radio `0x16`, far-return fallback search, and teleport lifecycle were intentionally not traced in this slot.

## Verdict Tally

PASS: 6 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Sources

- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs:432`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs:626`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs:1045`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock.rs:124`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs:4677`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs:4707`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs:4740`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/miner/HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/miner/HARV_DESTROYED_REFINERY_UNLOAD_ABORT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/miner/MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:7351`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:8215`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:11722`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:12515`
