# Null State-3 UnloadingClass Display Latch Trace

**Scenario:** `UnitClass::Mission_Deploy_Building` state 3 has an active `UnloadingClass` / display latch, but the west-cell building lookup at miner current cell + `(-1,0)` returns null.
**Scope:** One mechanic only: whether the state-3 null branch clears or preserves the unload display latch, and how current Rust compares.
**Status:** PARTIAL: Ghidra MCP had no running instance available for fresh decompile. This report uses existing verified Ghidra reports and current Rust source scan only.
**Runtime frame count:** UNCHECKED. Static code proves branch writes, but not the exact number of presented stale `HORV`/`CMON` frames.

## Pipeline

1. Trigger: miner is already in stock deploy/unload state 3 with the unload-active display flag set.
2. Lookup: state 3 computes miner cell + west offset and calls the cell building lookup.
3. Null branch: lookup returns null, so stock queues Harvest instead of entering the storage/credit block.
4. Display state: compare native `unit+0x6D1` and Rust's equivalent fields after the branch.
5. Render: compare whether the visible `UnloadingClass` override can still be selected.

## Stage Verdicts

### Stage 1 - Active YR applicability

Input: stock `HARV`/`CMIN` unloading at stock `GAREFN`/`NAREFN`.

gamemd: Active in standard YR. Existing Ghidra reports verify `[HARV]`/`[CMIN]` have `Harvester=yes`, `Dock=NAREFN,GAREFN`, `UnloadingClass=HORV/CMON`; stock refineries have `Refinery=yes`, `DockUnload=yes`, and `NumberOfDocks=1`.

Rust: Current miner dock code models stock refinery unload through `RefineryDockPhase::Unloading`.

Verdict: PASS.

### Stage 2 - State-3 null lookup branch

Input: state 3 at a dump-gate crossing, west-cell building lookup returns null.

gamemd: Verified static branch at `0x0073E306..0x0073E350`: call `Look_up_building_in_cell`, take null branch, optionally send radio `3`, then `Queue_Mission(10,1)`. Storage drain and credits start only in the non-null branch at `0x0073E355+`.

Rust: `phase_unloading` calls `mission_deploy_unload_building`; when it returns `None`, Rust calls `abort_missing_unload_building` and returns before cargo removal, credits, or `BaleDepositEvent`.

Rust evidence: `src/sim/miner/miner_dock_sequence.rs:1045-1049`.

Verdict: PASS for branch shape relevant to this slot.

### Stage 3 - Native latch clear/preserve

Input: native `unit+0x6D1 == 1` before the null branch.

gamemd: Existing verified reports find no `+0x6D1` write in `0x0073E306..0x0073E350`; `Queue_Mission @ 0x005B35E0` and `Commence @ 0x005B3570` also do not write `+0x6D1`. Verified clear paths are normal state 4 (`0x0073E1F6`) and radio `0x17` (`0x00737AC9`), neither of which is this branch.

Output: native `unit+0x6D1` is preserved by the state-3 null branch.

Verdict: PASS for gamemd behavior established from existing verified Ghidra reports.

### Stage 4 - Rust latch clear/preserve

Input: Rust miner in `RefineryDockPhase::Unloading`, with `miner.unload_active == true` and `display_type_override == Some(HORV/CMON)` before null lookup.

Rust: `abort_missing_unload_building` does not clear `entity.display_type_override`, but it calls `clear_unload_cluster`; `clear_unload_cluster` sets `miner.unload_active = false`. The `Miner` field is explicitly documented as `Unit+0x6D1 unload-active latch`.

Rust evidence:

- `src/sim/miner/mod.rs:304-306` documents `unload_active` as `Unit+0x6D1`.
- `src/sim/miner/miner_dock_sequence.rs:626-646` calls `clear_unload_cluster` in the null branch cleanup.
- `src/sim/miner/miner_dock_sequence.rs:122-130` clears `unload_active`.
- `src/sim/miner/miner_tests.rs:4740-4768` asserts only `display_type_override` preservation, not `unload_active` preservation.

Output: Rust preserves the render override field, but clears the byte-equivalent unload-active latch.

Verdict: FAIL. Mechanism/byte-state parity differs from gamemd's preserved `unit+0x6D1`.

### Stage 5 - Render-visible override after null branch

Input: state after null branch.

gamemd: `UnitClass::DrawExtras @ 0x0073CEC0` gates the `UnloadingClass` swap on `Harvester=yes`, `unit+0x6D1 != 0`, and `Type+0x6B8 != 0`; no mission/substate gate is present. Because the null branch does not clear `+0x6D1`, static evidence says `HORV`/`CMON` remains render-eligible until a later clear path.

Rust: rendering uses `entity.display_type_override` instead of `miner.unload_active`; `abort_missing_unload_building` does not clear `display_type_override`, so the visible override can remain after the null branch.

Rust evidence:

- `src/app_instances/units.rs:167-173` renders `display_type_override` when present.
- `src/sim/miner/miner_dock_sequence.rs:626-654` does not clear `display_type_override`.
- `src/sim/miner/miner_tests.rs:4740-4768` pins `display_type_override` preservation.

Verdict: UNCHECKED for exact player-visible equality because no runtime capture proves the number of actually presented stale frames in gamemd. Static branch eligibility is aligned, but PASS would require computed runtime frame equality.

## Findings

### FAIL - Rust clears the byte-equivalent unload latch on the null branch

Current Rust calls `clear_unload_cluster` from `abort_missing_unload_building`, and that clears `miner.unload_active`. Since `miner.unload_active` is documented as the `Unit+0x6D1` unload-active latch, this is a mechanism drift from gamemd's state-3 null branch, which preserves `+0x6D1`.

Player visibility: Medium. The current renderer keys visible `HORV`/`CMON` off `display_type_override`, which is preserved, so the immediate screen may still match the stale visual. The underlying sim byte/state equivalent does not.

Frequency: Low-to-medium in normal play. It requires refinery loss or lookup failure during active state-3 unloading, but it is reachable in standard YR.

### UNCHECKED - Exact stale `HORV`/`CMON` presented-frame count

Static gamemd evidence proves the null branch itself does not clear `+0x6D1`, and `DrawExtras` has no mission gate. It does not prove how many frames are actually presented after the abort before a later path, redraw ordering, or command transition changes the visible result.

## Adjacent Findings

- Cargo preservation, no credits/events, reserved-refinery cleanup, state-4 rediscovery, radio `0x16`, far-return fallback, and teleport lifecycle were not traced in this slot.
- Existing tests now include a specific `state3_null_lookup_does_not_clear_unload_display_latch` assertion for `display_type_override`, but not for `miner.unload_active`.

## Verdict Tally

PASS: 3
FAIL: 1
UNCHECKED: 1
NOT-IMPLEMENTED: 0

## Sources

- Existing verified Ghidra reports:
  - `docs/research/miner/HARV_UNLOADING_CLASS_DISPLAY_TIMING_GHIDRA_REPORT.md`
  - `docs/research/miner/MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`
  - `docs/research/miner/REFINERY_DESTROYED_OR_SOLD_MID_UNLOAD_CONTACTS_DISPLAY_CREDITS_GHIDRA_REPORT.md`
  - `docs/research/miner/REFINERY_SOLD_DESTROYED_MID_UNLOAD_RUNTIME_EFFECTS_GHIDRA_REPORT.md`
- Current Rust source scan:
  - `src/sim/miner/mod.rs`
  - `src/sim/miner/miner_dock_sequence.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/app_instances/units.rs`
- Ghidra MCP status: `list_instances` returned no running Ghidra instances, so no fresh live read-only decompile was possible in this slot.

