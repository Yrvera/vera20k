# GSI-07.15 Miner Far-Return Drive Command Design

Date: 2026-07-25  
Parent owner: GSI-07.15 level-zero scan/move production loop  
Prerequisite scope: stock HARV state-2 far-return Drive command  
Input contract:
`docs/contracts/2026-07-25-gsi-07-15-miner-far-return-drive-command-implementation-contract.md`

## Goal

Make a stock non-teleporter HARV physically drive from a verified
`HarvesterTooFarDistance` fallback to its already selected QueueingCell staging
cell with the merged retail Drive profile. Preserve the verified target search,
mission state, owner gate, and command ordering. The verified shared
mission-delay RNG/scheduler tail is recorded as a separate residual rather than
being approximated in this prerequisite.

The change must close the earliest load-bearing divergence exposed by the
suspended parent's real harvest/return/dock/exit loop. It must not widen into a
general miner-return rewrite.

## Evidence And Architecture Fit

Active YR's HARV state-2 far-return branch passes the valid fallback
`CellClass*` through the normal unit destination chain:

`TechnoClass::Set_Destination @ 0x00741970`
→ `FootClass::Set_Destination_Internal @ 0x004D94B0`
→ `DriveLocomotionClass::Set_Destination @ 0x004AFD40`.

Ore and far-return destinations share that chain. Drive's active speed ramp at
`0x004B0F20` reads the type-owned `Accelerates`, acceleration, deceleration, and
slowdown inputs. For stock HARV, merged retail rules resolve
`Accelerates=true`, `AccelerationFactor=0.03`,
`DeaccelerationFactor=0.002`, and `SlowdownDistance=500`.

Rust already has the correct authority boundary in
`src/sim/miner/miner_system.rs`: `issue_outbound_ore_move` resolves merged
`MoveInfo`, terrain cost, movement zone, crusher input, NavCom, Drive runtime,
and the exact movement profile. Its name is narrower than its mechanism, but
it does not own the caller's existing-destination gate.
`try_issue_standard_far_return_drive` instead calls metadata-free
`issue_move_if_idle`, so the current fraction remains zero forever for an
accelerating HARV.

The verified state-2 owner check is also misplaced in Rust. Native checks the
existing `Foot/Unit+0x5A4` destination before refinery selection. Rust's
outbound ore caller has the corresponding NavCom plus transitional
`MovementTarget` predicate, but `handle_return` does not. A helper-local check
would be too late because `handle_return` may already store
`reserved_refinery`.

This design keeps the Rust-native ownership split:

- the miner mission chooses and owns the state-2 staging destination;
- one miner-specific Drive command issuer translates merged type/world
  authority into the generic movement command;
- the movement/Drive systems own path and speed evolution after issuance.

No simulation state, serialization field, cross-layer dependency, or new
authority is needed.

## Tiny-Detail Ledger

| Detail | Required treatment |
|---|---|
| Stock unit | HARV only for the new call site; existing CMIN outbound behavior remains unchanged |
| Destination | Keep the already resolved passable QueueingCell fallback |
| Mission state | Remain `ReturnToRefinery`, the Rust representation of native case 2 |
| Owner gate | Add a War-only NavCom/`MovementTarget` early return in `handle_return` before refinery selection; the shared issuer has no such gate |
| Rules | Resolve current merged object rules; do not hardcode the stock profile |
| Terrain | Preserve effective terrain, `SpeedType`, and terrain-cost lookup |
| Zone | Preserve movement-zone legality |
| Crusher | Preserve current crusher eligibility input |
| Locomotor | Preserve the full helper's Drive/Teleport selection and CMIN rollback |
| Runtime state | Require immediate NavCom, final goal, and Drive destination; native later `head_to` timing remains UNVERIFIED |
| Speed ramp | Stamp acceleration, deceleration, and slowdown before the next Drive tick |
| Ordering | Keep far-return state write and handled return in the current tick |
| Failure result | Do not branch on the Rust A* boolean; native failure/retry timing is still UNKNOWN |
| RNG | The destination/Drive handoff itself does not draw, but the enclosing native state-2 dispatch reaches `Scenario+0x218.RandomRanged(0,2)` and returns base delay plus jitter. This design makes no full-dispatch RNG claim. |
| Neighbor callers | Close-HARV dock movement and refused CMIN staging remain explicit follow-ups |
| Proof level | Two bounded mechanism/result checks; no broad Drive, path, byte, or pixel certification |

## Alternatives

### A. Direction-Neutral Miner Drive Issuer, Far HARV Only

Rename `issue_outbound_ore_move` to `issue_stock_miner_drive_move`, update its
existing outbound caller, and use it for the far-HARV staging destination.
Ignore its boolean at the new call site so the current mission-state/tick
behavior is unchanged. Add the verified stock-HARV existing-destination gate
at the top of `handle_return`, before any refinery selection.

Advantages:

- matches the verified shared native destination chain;
- reuses the already production-tested authority path without duplication;
- closes exactly the failing parent dependency;
- leaves uncertain path-failure timing untouched;
- keeps CMIN outbound rollback behavior in one implementation.

Risk:

- the helper name must not imply it is universally correct for every future
  miner movement caller. The name therefore describes stock-miner Drive command
  ownership, not all miner mission movement.

### B. Call The Existing Ore-Named Helper Directly

This has the same runtime behavior but leaves false architectural documentation:
far-return movement would depend on an ore-only name. That invites later
duplication and obscures the verified shared native mechanism.

Rejected.

### C. Replace All Three Metadata-Free Return/Dock Callers

This would also change close-HARV movement toward an accepted dock and CMIN
refused-contact staging. Both are active risks, but neither control loop has the
required red-first production oracle or a closed native failure/timing contract
in this prerequisite.

Rejected as broader than the smallest parent dependency.

### D. Make The Far Wrapper Return The Issuance Boolean

This could cause same-tick Rust fallthrough when A* rejects a command. The
available native evidence proves the destination handoff but not the later
path-failure/retry lifecycle. Treating the Rust boolean as native authority
would introduce unverified control-flow drift.

Rejected pending a dedicated trace.

## Chosen Design

Use alternative A.

In `src/sim/miner/miner_system.rs`:

1. Rename `issue_outbound_ore_move` to
   `issue_stock_miner_drive_move`.
2. Update `handle_move_to_ore` to call the renamed helper with no behavior
   change.
3. In `handle_return`, immediately after the teleport-state guard, return for
   `MinerKind::War` when the entity already has a NavCom or transitional
   `MovementTarget`. This must precede `reserved_refinery` selection.
4. In `try_issue_standard_far_return_drive`, call the renamed helper with the
   already selected staging cell.
5. Deliberately ignore the returned boolean, then perform the existing
   `ReturnToRefinery` state write and return `true`.
6. Leave `issue_move_if_idle` and its two remaining return/dock callers
   unchanged.

The helper body is not redesigned. Its current rule resolution, target bounds,
locomotor selection, CMIN rollback, terrain/zone/crusher authority, generic
command call, and movement-profile stamping remain byte-for-byte unchanged
except for its identifier and purpose comment.

## Production Oracle

Add
`production_stock_harv_far_return_drive_uses_rule_profile` to
`src/sim/miner/outbound_drive_tests.rs`, reusing the merged-retail fixture.

The test must:

1. Load and merge retail base plus YR rules and art INIs.
2. Spawn stock GAREFN and stock HARV through production lifecycle APIs.
3. Block the refinery foundation in a real `PathGrid`.
4. Put a full HARV farther than the stock five-cell threshold in
   `ReturnToRefinery`.
5. Install Clear terrain, Track cost, and movement-zone authority.
6. Advance the production miner tick until it issues the far staging command.
7. Assert the selected target is the stock QueueingCell-derived passable cell,
   not the accepted dock cell.
8. Assert state, NavCom, `MovementTarget.final_goal`, Drive destination, merged speed,
   acceleration `0.03`, deceleration `0.002`, and slowdown `500`.
9. Advance one normal movement tick and assert exact current fraction `0.03`
   and positive current speed.
10. Advance a bounded number of production ticks and assert physical departure.
11. Make no RNG-parity assertion across the full production dispatch; the
    verified state-2 mission-delay tail remains outside this prerequisite.

The new oracle must inspect those fields directly. It must not reuse
`assert_command_state`, because that existing outbound regression helper also
requires an immediate path shape and `Drive.head_to == destination`; neither is
verified native `Set_Destination` timing for this parity slice.

The test must fail before the production change specifically because the
far-return command has zero acceleration metadata and cannot depart.

Add
`production_stock_harv_far_return_preserves_existing_navcom_owner` in the same
module:

1. Build the same merged-retail far-refinery world.
2. Issue a normal HARV outbound command through production.
3. Remove only its `MovementTarget`, preserving the production-created NavCom
   and Drive destination/runtime, then set the full miner to
   `ReturnToRefinery` with no reserved refinery.
4. Snapshot state, target/cargo, scoped miner timers, NavCom, Drive runtime,
   reservation, radio/dock facts, and sound events.
5. Advance one production tick.
6. Require the snapshot to remain unchanged, including
   `reserved_refinery=None`; no staging movement or contact may appear.

This negative test distinguishes the native top-of-state owner gate from a
helper-local check, which would run only after Rust had already selected and
stored a refinery.

## Impact And Neighbor Review

Direct behavior changes are limited to stock HARV state-2 return ownership and
far-return issuance: an existing owner now suppresses refinery selection, while
an idle far HARV receives the merged Drive profile. The renamed helper's
existing outbound users must produce identical command state, so all seven
outbound Drive production tests remain mandatory.

The suspended parent's six production tests are rerun only after this
prerequisite is validated, committed, merged into `dev`, and replayed into the
parent worktree. Relevant Drive ramp, miner docking/exit, and CMIN piggyback
neighbors are part of the regression gate.

Explicit residuals:

- close-HARV accepted-dock movement still uses metadata-free issuance;
- CMIN refused-contact staging still uses metadata-free issuance;
- native path-failure/retry timing remains UNKNOWN;
- native `Set_Destination` stores only Drive destination, so Rust's immediate
  `head_to` handoff remains UNVERIFIED and is not a parity assertion here;
- both the existing-destination and valid far-destination native state-2 exits
  reach `Scenario+0x218.RandomRanged(0,2)` and return current mission base delay
  plus jitter; Rust scheduler ownership for that delay remains UNVERIFIED and
  must be closed as its own mechanism slice;
- broader pathfinding, track, collision, render, audio, byte, and pixel parity
  remain UNVERIFIED.

## Approval Question

Why should this be approved?

The failing production trace, current Rust dataflow, merged retail rule values,
and verified native destination/Drive chain all point to the same earliest
divergence. The design restores the verified owner gate before selection, then
reuses the existing tested Rust authority boundary at the parent-required
far-return call site.

What evidence could still make it wrong?

- evidence that native far-return bypasses type-owned Drive speed fields;
- evidence that far-return uses a different locomotor command than ore
  destination;
- evidence that the Rust issue-result boolean must alter same-tick mission
  control flow;
- evidence that the selected staging target is not the stock QueueingCell
  fallback;
- evidence that state 2 may select a refinery despite a non-null native NavCom.

The first, second, and fourth are contradicted by the cited active-YR evidence.
The fifth is contradicted by the state-2 entry gate at `Foot/Unit+0x5A4`. The
third is unresolved, so the design explicitly preserves current timing and does
not claim parity for failure handling.

## Acceptance Gate

- exact red-before/green-after positive profile and negative owner-gate
  production oracles;
- all seven existing outbound Drive production tests;
- suspended parent's six production tests after replay;
- relevant Drive speed and CMIN piggyback neighbors;
- one serial `cargo check -q`;
- branch and post-merge validation with literal result lines recorded;
- no push.
