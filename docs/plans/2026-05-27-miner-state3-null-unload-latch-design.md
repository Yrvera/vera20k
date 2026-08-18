# Miner State-3 Null Unload Latch Design

## Goal

Preserve the active gamemd `Unit+0x6D1` unload-active latch on the `Mission_Deploy_Building` state-3 null west-cell refinery lookup branch while retaining Rust cleanup that prevents stale dock/contact queues.

## Architecture Context

The scoped flow lives in `src/sim/miner/miner_dock_sequence.rs`. `handle_dock_sequence` drives `RefineryDockPhase::Unloading`; `phase_unloading` calls `mission_deploy_unload_building`, which rechecks the miner's current cell plus the west-cell offset. When that lookup returns `None`, Rust calls `abort_missing_unload_building` before cargo drain, credit award, or `BaleDepositEvent` emission.

Rust currently has two separate state carriers relevant to this branch:

- `miner.unload_active`, documented in `src/sim/miner/mod.rs` as the `Unit+0x6D1` unload-active latch.
- `entity.display_type_override`, consumed by `src/app_instances/units.rs` to render `UnloadingClass` art such as `HORV` or `CMON`.

The present null-lookup branch preserves `display_type_override` but calls `clear_unload_cluster`, which clears `miner.unload_active`. That is byte/mechanism drift from the verified gamemd branch. The same branch must still clear Rust dock bookkeeping through `RefineryDockContacts::cancel_miner` so stale contacts, pad occupancy, and waiter state do not block future miner/refinery interaction.

The `sim/` boundary remains intact: all state changes stay in miner/dock simulation code. Rendering continues to consume simulation state from above; no render, UI, audio, or net dependency is introduced into `sim/`.

## Impact Analysis

Primary implementation surface after approval:

- `src/sim/miner/miner_dock_sequence.rs`
  - Split unload timer/accumulator cleanup from latch cleanup.
  - Update `abort_missing_unload_building` to preserve `miner.unload_active`.
  - Keep normal state-4 and invalid-refinery clear paths unchanged unless tests reveal an accidental branch conflation.
- `src/sim/miner/miner_tests.rs`
  - Extend the null west-cell lookup test to assert `miner.unload_active` remains true.
  - Preserve tests for no credits/events, cargo preservation, queue/contact release, and state-4 clear behavior.

Risk areas:

- Accidentally preserving unload timer/accumulator fields along with the latch could allow stale unload loops. Only the `Unit+0x6D1`-equivalent latch is meant to survive this branch.
- Accidentally moving the normal state-4 clear would create a much larger parity regression. State 4 remains the verified normal clear path.
- Treating this branch as a generic invalid-refinery abort could conflate separate gamemd paths. The west-cell null branch has a specific verified ordering and latch behavior.

## Chosen Approach

Use a narrow split-cleanup helper: preserve `miner.unload_active` only in the state-3 null west-cell lookup abort, while clearing the rest of the unload timer/cluster bookkeeping and all Rust dock/contact reservation state.

Concretely, after implementation approval:

1. Keep `clear_unload_cluster` as the full clear used by the verified normal state-4 completion path and any already-verified clear path represented in Rust. Do not use this wording to invent new latch clears.
2. Add or extract a helper that clears unload accumulator/timer cluster fields but does not clear `miner.unload_active`.
3. Have `abort_missing_unload_building` call the preserve-latch helper instead of the full clear.
4. Leave `entity.display_type_override` untouched in this branch.
5. Continue calling `cancel_miner`, clearing `reserved_refinery`, `dock_queued`, `dock_phase`, pivot/facing/movement/exit caches, and choosing `dock_abort_state`.

This is preferred over a boolean `clear_unload_cluster(preserve_latch)` because the branch distinction is itself parity-critical. A named preserve-latch helper makes future mistakes easier to spot.

## Tiny-Detail Ledger

- Active YR gate: stock `[HARV]` and `[CMIN]` have `Harvester=yes`, `Dock=NAREFN,GAREFN`, and `UnloadingClass=HORV/CMON`; stock `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`. Source: `docs/research/traces/MISSING_WEST_CELL_BUILDING_STATE3_TRACE.md:14-19`.
- State 3 recomputes the refinery from miner current cell plus the west offset `(-1,0)`, calls `Look_up_building_in_cell`, and branches around drain/credit work when null. Source: `docs/research/miner/HARV_FULL_CARGO_MISSING_REFINERY_FALLBACK_GHIDRA_REPORT.md:103-131`.
- Null branch order is `PathType::Has_Valid_Steps`, optional radio `3`, `Queue_Mission(0x0A, 1)`, then mission timer epilogue. Source: `docs/research/miner/MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md:74-87`.
- Null branch skips storage drain, credit award, refinery animation slot clearing, `ReleaseDockedHarvester`, radio `0x07`, and radio `0x19`. Source: `docs/research/miner/MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md:74-87,140-146`.
- Null branch does not clear `Unit+0x6D1`; `Queue_Mission` and `Commence` also do not clear it. Source: `docs/research/traces/NULL_STATE3_UNLOADINGCLASS_DISPLAY_LATCH_TRACE.md:40-65`.
- `UnitClass::DrawExtras` gates `UnloadingClass` on `Harvester=yes`, `Unit+0x6D1 != 0`, and non-null `UnloadingClass`; it has no current mission/substate gate. Source: `docs/research/miner/UNLOAD_VISUAL_STALE_FRAME_AFTER_REFINERY_LOSS_GHIDRA_REPORT.md:72-80`.
- Verified clear paths for `Unit+0x6D1` are normal state 4 and radio `0x17`; radio `3` is not a clear path. Source: `docs/research/miner/UNLOAD_VISUAL_STALE_FRAME_AFTER_REFINERY_LOSS_GHIDRA_REPORT.md:45-63`.
- Full cargo after the abort re-enters Harvest and returns to refinery selection before ore scan; partial cargo can resume ore search/harvest logic. Source: `docs/research/miner/REFINERY_SOLD_DESTROYED_MID_UNLOAD_RUNTIME_EFFECTS_GHIDRA_REPORT.md:75-79`.
- Rust must preserve cargo, emit no credits or deposit events, and avoid miner-owner fallback on this null branch. Source: `docs/research/traces/MISSING_WEST_CELL_BUILDING_STATE3_TRACE.md:41-55`.
- Rust must still release reservation/contact/pad/waiter bookkeeping enough to avoid stale queues, even though these maps are Rust-side structure rather than byte-identical gamemd fields. Source: `docs/research/traces/MISSING_WEST_CELL_BUILDING_STATE3_TRACE.md:57-63`.
- Exact stale `HORV`/`CMON` rendered frame count after null abort remains runtime-UNCHECKED. Static evidence proves no immediate clear in the branch, not the number of presented frames. Source: `docs/research/miner/UNLOAD_VISUAL_STALE_FRAME_AFTER_REFINERY_LOSS_GHIDRA_REPORT.md:107-119`.

## Design

### Components

- `abort_missing_unload_building`: owns the state-3 null west-cell lookup abort. It should preserve the `Unit+0x6D1` equivalent latch while clearing Rust contact/reservation and movement bookkeeping.
- Full unload clear helper: remains the normal state-4/radio-clear-equivalent cleanup path that clears `miner.unload_active`.
- Preserve-latch timer cleanup helper: clears accumulator, timer-fired marker, cluster start/scratch/duration/repeat, and legacy unload timer fields as needed, but deliberately leaves `miner.unload_active` unchanged.
- `RefineryDockContacts`: remains the Rust-side contact cleanup owner via `cancel_miner`.

### Interfaces / Contracts

The branch contract for `abort_missing_unload_building` is:

- Preserve `miner.unload_active`.
- Preserve `entity.display_type_override`.
- Preserve all cargo.
- Emit no credits and no `BaleDepositEvent`.
- Clear `reserved_refinery`, `dock_queued`, pivot/facing/movement/drive-track/exit/cache state, and dock contact state for the removed/missing refinery.
- Route full cargo to `ReturnToRefinery` and partial cargo through the existing abort-state rule. Existing Rust also preserves `ForcedReturn` when cargo remains; that is Rust behavior to avoid regressing unless later gamemd evidence says otherwise, not a separately verified parity claim from this null-branch slice.

The normal state-4 completion contract is unchanged:

- Clear `miner.unload_active`.
- Clear `display_type_override`.
- Release contact/pad state.
- Hand back to harvest/search scheduling through the existing `phase_departing` path.

### Data Flow

1. `phase_unloading` reaches a dump-gate crossing.
2. Cargo still has a slot to drain, so Rust attempts `mission_deploy_unload_building`.
3. If lookup returns `None`, Rust enters `abort_missing_unload_building`.
4. `cancel_miner` removes the miner from Rust dock/contact/pad/waiter state.
5. Entity movement/pivot targets are cleared, but `display_type_override` remains.
6. Miner dock fields are reset, but `unload_active` remains true.
7. The miner leaves `Dock` via `dock_abort_state` without cargo drain or credit/event emission.

### Error Handling

No new fallible API is required. Missing entities remain handled through existing `Option` checks. The implementation should avoid panics in the abort path because this branch is specifically for missing/invalid world state.

### Testing Strategy

Focused tests after implementation approval:

- Extend `state3_null_lookup_does_not_clear_unload_display_latch` to assert `miner.unload_active == true` as well as `display_type_override == Some(HORV/CMON)`.
- Keep `missing_west_cell_building_does_not_credit_or_emit_deposit_event` asserting cargo preservation and no credits/events.
- Keep or add assertion that `reserved_refinery == None`, `dock_queued == false`, and `dock_reservations.is_occupied(refinery) == false` after the null abort.
- Guard normal state-4 completion: `Departing` must still clear `miner.unload_active` and `display_type_override`.
- Guard invalid/dying refinery behavior separately so it does not silently inherit the state-3 null latch rule unless that branch is proven to be the same null west-cell lookup case.

Verification run after implementation approval:

- Run the focused miner tests that cover state-3 null lookup and state-4 departing.
- Then run the narrow miner test module or focused `cargo test` target used by this repo for `src/sim/miner/miner_tests.rs`.
- Run `cargo check -q` only after checking for active cargo/rustc processes, per project instructions.

## Architectural Decisions

- Keep this in `sim/miner`; no render/UI/audio dependencies are introduced.
- Model the native byte-equivalent latch in `miner.unload_active`, as the current code already documents.
- Do not key the implementation on exact stale rendered frame count. Static evidence is sufficient to preserve the latch; runtime capture is only needed to prove how many presented stale frames stock YR shows.
- Do not move or weaken state-4 clearing. State 4 remains the verified normal clear path.
- Do not touch radio `0x16`, far-return fallback search, teleport lifecycle, or normal dock admission behavior.

## Alternatives Considered

### Boolean parameter on `clear_unload_cluster`

`clear_unload_cluster(snap, preserve_latch)` would be a smaller edit, but the boolean hides the parity-critical branch distinction at call sites. A named helper is clearer and safer.

### Rich abort-kind enum

A `DockAbortKind` enum could encode state-3 null lookup, invalid refinery, linked interrupt, and normal completion behavior. That may be useful later, but it is broader than this approved patch and risks unrelated churn.

### Preserve all unload cluster fields

Leaving accumulator/timer state intact would preserve more internal bytes, but Rust's timer cluster drives future unload behavior. Static gamemd evidence only requires preserving `Unit+0x6D1` on this branch; preserving the whole active unload cluster risks stale drain behavior and blocked queues.
