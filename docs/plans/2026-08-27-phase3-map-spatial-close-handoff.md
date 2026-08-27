# Phase 3 Map/Spatial Closure Handoff

**Stopped:** 2026-08-27, at user request

**Worktree:** `C:\Users\enok\Documents\ra2-rust-game-phase3-map-spatial-close`

**Branch:** `feature/phase3-map-spatial-close`

**Documentation checkpoint:** `546dcd86 docs(map): hand off open Ship bridge Z design`

**Pre-handoff research HEAD:** `18280d5e docs(map): normalize Ship bridge Z report`

**Publish state:** local only; no push or PR authority was granted

## Completed slice

The bounded GSI-04.03 Spark shared-CellClass-dummy mechanism is closed. Its final certification is `6463f2c2`; supporting implementation, repair, provenance, and critic-led commits are in branch history through that commit. The final recorded focused validation was:

```text
gsi_04_03: 69 passed; 0 failed
Spark focused groups: 45 passed; 0 failed
snapshot v111 rejection: 1 passed; 0 failed
```

No phase-wide/full `cargo test -p vera20k --lib` was run; Phase 3 is not closed.

## Open slice at stop

The current bounded mechanism is ShipLocomotion bridge-Z destination/braking behavior. Native research and design drafts are present in:

- `docs/research/PHASE3_SHIP_BRIDGE_Z_ADJUSTMENT_GHIDRA_REPORT.md` (modified)
- `docs/plans/2026-08-27-phase3-ship-bridge-z-adjustment-design.md` (untracked)

No Rust implementation for this mechanism has started. The design is `REVISED / OPEN` and must not be treated as implementation-ready.

Fresh design critic 4 rejected the earlier approximation. The repaired documents now require exact reciprocal Giant Squid attachment state, exact `CanAttach`, the real interval between attach and the first timer write, victim-Foot-AI-tail refresh, actual detach transactions, and native `start == -1` timer semantics. They preserve the previously rechecked Ship destination-Z, transactional scatter, 3D braking distance, invalid-geometry rejection, terminal stored-Z retry, snapshot, and hash requirements.

Two stock-active prerequisites remain explicitly open: exact Chronosphere attachment-release admission and exact IsLocomotor/PerformDeploy attachment-release admission. The attempted fresh critic 5 audit was interrupted before verdict when the user asked to stop. Any missing, approximate, or residual attachment/release behavior keeps GSI-04.03 open.

## Working-tree ownership

Preserve the three unrelated pre-existing untracked reports; this task did not create or modify them:

- `docs/research/PHASE3_HOUSE_UPDATE_AI_ACTIVATION_GHIDRA_REPORT.md`
- `docs/research/PHASE3_MAIN_TICK_MODAL_SCHEDULER_SEAM_GHIDRA_REPORT.md`
- `docs/research/PHASE3_UNIT_DEPLOY_HOUSE_FLAGS_GHIDRA_REPORT.md`

At pause, `git diff --check` passed for the Ship report/design drafts. `Get-Process cargo,rustc` returned no processes, so this worktree owns no running Cargo build. No merge, rebase, or cherry-pick is in progress.

## Exact next safe action

Resume on this same branch/worktree. Reconcile `git status`, worktrees, recent commits, and Cargo ownership first. Give the repaired Ship report/design plus critic-4 findings to a genuinely fresh read-only critic. Close the single largest active prerequisite it identifies, update the documents, and repeat with another fresh critic. Do not begin the Ship Rust builder until the design has zero open prerequisites and receives a zero-finding PASS. Continue focused `--lib` validation only; reserve the one full `cargo test -p vera20k --lib` for final Phase-3 closure.
