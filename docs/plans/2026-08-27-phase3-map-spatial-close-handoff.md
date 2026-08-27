# Phase 3 Map/Spatial Closure Handoff

**Checkpoint:** 2026-08-27, after fresh Ship design critic 7

**Worktree:** `C:\Users\enok\Documents\ra2-rust-game-phase3-map-spatial-close`

**Branch:** `feature/phase3-map-spatial-close`

**HEAD before this handoff update:** `84e106e2 docs(map): correct Ship timer design contract`

**Publish state:** local only; no push or PR authority was granted

## Closed work

The bounded GSI-04.03 Spark shared-CellClass-dummy mechanism is closed. Its final certification is `6463f2c2`; supporting implementation, repair, provenance, and critic-led commits are in branch history through that commit. The final recorded focused validation was:

```text
gsi_04_03: 69 passed; 0 failed
Spark focused groups: 45 passed; 0 failed
snapshot v111 rejection: 1 passed; 0 failed
```

No phase-wide/full `cargo test -p vera20k --lib` was run; Phase 3 remains open.

## Open work at stop

The current mechanism is GSI-04.03 ShipLocomotion bridge-Z destination, braking, terminal, and guard behavior. The committed research and design drafts are:

- `docs/research/PHASE3_SHIP_BRIDGE_Z_ADJUSTMENT_GHIDRA_REPORT.md`
- `docs/plans/2026-08-27-phase3-ship-bridge-z-adjustment-design.md`

They are included in branch history through `84e106e2`. No Rust implementation for this mechanism has started. The design remains `OPEN` and is not implementation-ready.

Critic 5's timer-sentinel finding is closed. Live evidence and the authoritative report were corrected in `37571c99`; the design was corrected in `84e106e2`. The exact predicate is `CdTimer::remaining(frame) != 0`: `start == -1` returns `duration != 0`. Native `FootClass::Constructor` also stores raw `(current frame,0)`, not `(-1,0)`. Fresh evidence critic 6 returned `PASS`, confirming all 16 scoped claims.

Fresh full-design critic 7 returned `BLOCK`. Its single largest finding is that the proposed bounded reciprocal-link/timer subset omits the ordinary stock Naval+Organic Giant Squid combat/lifecycle core. Every successful stock `SquidGrab`/`SquidGrabE` can reach native state 0–4 update, attacker limbo/lifecycle, periodic rookie/elite damage, `Culling=yes` kill behavior, and Naval detach removal/death. A link-plus-continuously-refreshed-timer approximation would leave wrong health, attacker existence, detach timing, future combat, and persistence.

Critic 7 also kept the row open for:

- an unsupported extra `dying` gate in the proposed exact `CanAttach` contract;
- stock-active Chronosphere source-area selection, attachment release, manager timer, and later warp ownership;
- stock-active IsLocomotor admission and `PerformDeploy` release;
- stale/incomplete state and snapshot claims that omit the full SQD manager/state-machine and Chronosphere timer state.

The critic rechecked the core bridge-Z requirements without finding a new gap: transactional command/internal/direct/scatter entry, separate Ship/Drive Z, structural destination authority, native 3D distance and fixed saturation, strict terminal cell-plus-stored-Z retry, geometry rejection without mutation, cancel/getter behavior, and destination/retry save/hash coverage.

Any unresolved, approximate, missing, or residual attachment/release behavior keeps this row open. Remaining Phase 3 rows and the phase-wide reverse audit have not begun.

## Working-tree and process state

Preserve the three unrelated pre-existing untracked reports; this task did not create or modify them:

- `docs/research/PHASE3_HOUSE_UPDATE_AI_ACTIVATION_GHIDRA_REPORT.md`
- `docs/research/PHASE3_MAIN_TICK_MODAL_SCHEDULER_SEAM_GHIDRA_REPORT.md`
- `docs/research/PHASE3_UNIT_DEPLOY_HOUSE_FLAGS_GHIDRA_REPORT.md`

Before this handoff update, the branch was 72 commits ahead of `origin/main`. No merge, rebase, or cherry-pick is in progress. All delegated critics are completed. The process check returned no `cargo` or `rustc` processes, so this worktree owns no running build.

No Cargo validation was run after the recorded Spark tests because the subsequent work was research/design only.

## Exact next safe action

Resume in this same branch/worktree and reconcile actual git/worktree/process state first. Use an exhaustive-slice re-investigation to extend the existing Ship report with the complete active Naval+Organic SQD core rooted at `0x006297F0`, including `LimboLaunch`, manager timers/state, every state 0–4 branch and ordering, rookie/elite damage, culling thresholds/kill path, attacker lifecycle, and Naval detach removal/death. Prove retail activation and every evidence-backed exclusion; do not write Rust during that research pass. Then revise the design/state/snapshot/test contract and submit the whole design plus all prior findings to genuinely fresh critic 8. Continue one-largest-finding-at-a-time with a new critic per repair. Do not start the Ship Rust builder until the design has no open prerequisites and receives a zero-finding PASS. Reserve the single full `cargo test -p vera20k --lib` for final Phase 3 closure.
