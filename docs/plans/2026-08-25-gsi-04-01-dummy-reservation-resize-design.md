# GSI-04.01 dummy reservation reconstruction design

## Status

Approved for the narrow G1 implementation slice. This does not close
GSI-04.01; the accepted disparity report retains G2-G4, C1-C6, and explicit
cross-row residuals.

## Requirement and evidence

Active YR `MapClass::Resize @ 0x00565C10` reconstructs the one fixed fallback
CellClass at `0x00ABDC50` through `CellClass::Constructor @ 0x0047BBF0` on every
established active Resize. `CellClass::Constructor @ 0x0047BC50` clears
`CellClass+0xDC`. `MouseClass::Load @ 0x005BDF70` reaches Resize at
`0x005BE150`, after candidate restoration and only on the accepted in-scenario
load path.

Rust already reconstructs the process-owned `SharedCellDummy` at that accepted
commit seam, but models `+0xDC` separately as
`CellReservationGrid::dummy_mask`. Snapshot deserialization restores the split
mask and the accepted Resize currently leaves it intact. The exact correction
is to clear that dummy-only mask at the existing successful-Resize seam.

The active native writer/stream contract for real-cell `+0xDC` values remains
unverified. This design therefore does not clear, rebuild, or reinterpret the
real reservation map.

## Player-experience and determinism ledger

- Trigger: a reservation write resolving to an invalid or unallocated cell,
  followed by an accepted in-scenario Load Game.
- Required state change: the shared dummy's modeled constructor fields and its
  split reservation mask become zero at the same commit seam.
- Stable identity: the existing `SharedCellDummy` handle remains the same Arc;
  no process-global handle replacement is allowed.
- Real cells: existing `CellReservationGrid::masks` are left untouched as a
  narrow scope guarantee, not claimed as proven native save/load parity.
- Transactionality: rejected/fallible candidate preparation must not mutate the
  live shared dummy or clear a candidate's reservation state.
- Hashing: because `dummy_mask` already participates in `state_hash`, accepted
  reconstruction must remove the stale bit from subsequent deterministic hash
  state. No RNG cursor, tick, identity allocator, or scheduler state changes.
- New-game and RMG paths: they already create default reservation authority or
  operate before a Simulation reservation grid exists. Their existing
  process-dummy reconstruction remains unchanged.

## Options considered

### 1. Coordinate both modeled pieces at the Simulation Resize seam — chosen

Add a dummy-only reconstruction method to `CellReservationGrid`. Rename the
Simulation seam so its name covers the complete modeled CellClass dummy, make
it mutable, and have it reconstruct the shared handle plus clear the split
dummy reservation mask. The accepted persistence commit calls that one seam.

This is the smallest change that exactly closes the verified mismatch and keeps
the transaction boundary explicit.

### 2. Move `dummy_mask` into `SharedCellDummy`

This is more structurally literal, but it changes serialization, hashing,
cloning, process ownership, RMG interaction, and every reservation writer. The
full `+0xDC` lifecycle is still unverified, so this would front-load broad
architecture before its native contract is known.

### 3. Clear the whole reservation grid on Resize

Rejected. Only dummy reconstruction is proven. Clearing real masks would turn
an unknown native stream contract into an invented behavior and could alter AI
placement after load.

## Implementation shape

1. Add `CellReservationGrid::reconstruct_dummy_for_map_resize(&mut self)` that
   assigns zero only to `dummy_mask`.
2. Rename
   `Simulation::reconstruct_shared_cell_dummy_for_map_resize(&self)` to
   `reconstruct_cellclass_dummy_for_map_resize(&mut self)` and call both
   `SharedCellDummy::reconstruct_for_map_resize` and the new reservation method.
3. Update the accepted `PreparedLoad::into_parts` seam to borrow its owned
   Simulation mutably and call the renamed method.
4. Update direct tests/callers to the complete seam name. Do not change the
   lower-level process-only reconstruction used by ordinary new-map, RMG, or
   headless bootstrap.

## Acceptance tests

- Focused reservation test: seed one real-cell bit and one dummy bit, invoke the
  dummy reconstruction method, and prove the real bit remains while the dummy
  becomes zero.
- Accepted-load/Simulation seam test: seed dirty shared-dummy fields plus real
  and dummy reservation bits; after the complete reconstruction, prove stable
  dummy identity, zero modeled dummy fields, zero dummy reservation, unchanged
  real reservation, and a state-hash change caused by removal of stale dummy
  authority.
- Preserve the raw snapshot round-trip test that expects `dummy_mask` to decode
  before accepted commit. Serialization itself is not Resize and must not
  silently clear state.
- Focused validation only, every Cargo invocation with `--lib`; do not run the
  phase-wide full suite for this slice.

## Adversarial self-review and approval

The most dangerous shortcut is treating the constructor clear as proof that
all real reservation state resets. It is not: the current native report proves
the dummy constructor effect but leaves the real writer/save-load lifecycle
open. The chosen seam therefore changes one proven bit of state only.

The largest likely future rework is moving the split mask into the process-owned
dummy after C1 proves its writers. Coordinating the two existing owners now is
reversible and does not block that later migration. Duplicate multiplayer/RMG
Resize events do not require duplicate Simulation clears because no live
reservation grid exists on those Rust setup paths, and clearing zero is
idempotent.

Approved because it closes the verified accepted-load divergence without
claiming or implementing the unresolved real-cell lifecycle.
