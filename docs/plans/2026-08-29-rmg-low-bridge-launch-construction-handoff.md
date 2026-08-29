# RMG low-bridge launch construction handoff

- Status: current construction review loop complete; fresh read-only critic PASS.
- Branch: `feature/bridge-movement-parity`
- Reviewed HEAD: `d01675d9`
- Publication: not pushed in this wrap-up; no new pull request exists.
- Scope boundary: this handoff closes only the current U2-14/U2-15/U2-17 critic. U2-20,
  U2-21, branch-wide regressions, and adjacent constructor/placement dependencies were not
  started.

## Closed construction rows

- `db1c89eb test: bind mixed RMG construction trace` closes U2-14/U2-15. The fixed
  seed-4 production fixture drives the real bridge-hut owner, neutral-tech owner, stable
  structure append, and final emitter. It pins CABHUT-before-neutral-tech ordinals, exact
  emitted entity indices/types/cells, a location-free discarded `CADROP`, MapGen
  continuation, and deterministic repetition.
- `d01675d9 test: reject invalid generated Techno bindings` closes U2-17. Projection
  validates the complete generated binding table before mutation. Duplicate, missing,
  unexpected, type-only mismatch, and cell-only mismatch cases preserve the Scenario
  cursor, stable-id/occupancy cursors, entity and Logic stores, occupancy generation and
  cells, and raw occupation bytes. Success consumes no second Scenario draw.
- Fresh critic verdict: PASS with no bounded finding after rechecking the requirements,
  cited native evidence, current diff, and prior fixes.

## Focused validation of record

- Mixed production trace exact fixture: 1 passed, 0 failed.
- `map::rmg::pipeline` module: 11 passed, 0 failed.
- `map::rmg::phases::tech_buildings` module: 11 passed, 0 failed.
- Generated-projection exact negative fixture: 1 passed, 0 failed.
- `sim::world::world_spawn::techno_constructor_tests`: 15 passed, 0 failed.

All validation commands used `cargo test -p vera20k --lib` with a focused filter.

## PR gate and residuals

The reserved branch-wide command `cargo test -p vera20k --lib` is red: exit 1; 7,410
passed, 163 failed, 75 ignored. A representative placement failure passes on
`origin/main`, fails at the first implementation commit `8f54e900`, and still fails at
HEAD, so the branch cannot be represented as merge-ready. Some failures may be stale
expectations after intended native RNG changes, while others may be integration defects;
none were rebaselined or suppressed during this wrap-up.

Recorded, unstarted unit-2 residuals remain U2-20 (waterfall terrain negative boundary)
and U2-21 (active/dormant naming cleanup). The broader active-retail bridge goal also
remains open.

## Repo state and next safe action

The worktree is clean, no merge is in progress, and no Cargo or rustc process remains.
`origin/main` is an ancestor of this branch. The remote feature ref is a fast-forward
target and is 40 commits behind local HEAD; the previous same-name PR #169 is already
merged. The next safe publication action is to push this branch and open a new **draft**
PR to `main`. Do not mark it ready or merge it until the branch-wide `--lib` suite passes.
