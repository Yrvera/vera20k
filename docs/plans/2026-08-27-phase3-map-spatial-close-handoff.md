# Phase 3 Map/Spatial Closure Handoff

**Stopped:** 2026-08-27, at user request

**Worktree:** `C:\Users\enok\Documents\ra2-rust-game-phase3-map-spatial-close`

**Branch:** `feature/phase3-map-spatial-close`

**HEAD before this handoff update:** `49e8aa06 docs(map): finalize Phase 3 handoff`

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

They are included in branch history through `49e8aa06`. No Rust implementation for this mechanism has started. The design remains `REVISED / OPEN` and is not implementation-ready.

Fresh read-only critic 5 completed with `BLOCK`. Its largest finding is that the report/design misstate `TimerStruct::IsActive @ 0x004DE770`: live disassembly shows `start == -1` skips the elapsed-time calculation and returns `duration != 0`. Thus `(-1, positive duration)` is active. Current Rust `CdTimer::remaining` already has the matching sentinel behavior, but the research transcript, formula, design prose, and acceptance test are wrong. This finding has not been repaired or independently reverified by the parent.

Critic 5 also kept the row open for:

- omitted stock Giant Squid damage, culling, detach, and Naval attacker removal/death behavior;
- an unsupported extra `dying` gate in the proposed exact `CanAttach` contract;
- stock-active Chronosphere source-area selection, attachment release, manager timer, and later warp ownership;
- stock-active IsLocomotor admission and `PerformDeploy` release;
- stale design prose claiming the timer is the only new state.

The critic rechecked the core bridge-Z requirements without finding a new gap: transactional command/internal/direct/scatter entry, separate Ship/Drive Z, structural destination authority, native 3D distance and fixed saturation, strict terminal cell-plus-stored-Z retry, geometry rejection without mutation, cancel/getter behavior, and destination/retry save/hash coverage.

Any unresolved, approximate, missing, or residual attachment/release behavior keeps this row open. Remaining Phase 3 rows and the phase-wide reverse audit have not begun.

## Working-tree and process state

Preserve the three unrelated pre-existing untracked reports; this task did not create or modify them:

- `docs/research/PHASE3_HOUSE_UPDATE_AI_ACTIVATION_GHIDRA_REPORT.md`
- `docs/research/PHASE3_MAIN_TICK_MODAL_SCHEDULER_SEAM_GHIDRA_REPORT.md`
- `docs/research/PHASE3_UNIT_DEPLOY_HOUSE_FLAGS_GHIDRA_REPORT.md`

At stop, the branch was 69 commits ahead of `origin/main` before this handoff update. No merge, rebase, or cherry-pick is in progress. All three delegated agents are completed. The process check returned no `cargo` or `rustc` processes, so this worktree owns no running build.

No Cargo validation was run after the recorded Spark tests because the subsequent work was research/design only.

## Exact next safe action

Resume in this same branch/worktree and reconcile actual git/worktree/process state first. Independently verify `0x004DE770` in the active `gamemd.exe`, then correct only the timer-sentinel finding in the existing Ship research report and design, run `git diff --check`, and commit that coherent documentation repair. Submit the revised requirement, native evidence, diff, and output to a genuinely fresh read-only critic that rechecks prior fixes. Continue one-largest-finding-at-a-time with a new critic per repair. Do not start the Ship Rust builder until the design has no open prerequisites and receives a zero-finding PASS. Reserve the single full `cargo test -p vera20k --lib` for final Phase 3 closure.
