# Phase 3 Map/Spatial Closure Handoff

**Checkpoint:** 2026-08-27, after active-retail Giant Squid lifecycle/state-machine verification

**Worktree:** `C:\Users\enok\Documents\ra2-rust-game-phase3-map-spatial-close`

**Branch:** `feature/phase3-map-spatial-close`

**HEAD before this handoff update:** `b6811604 docs(ship): verify active giant squid lifecycle`

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

The current mechanism remains GSI-04.03 ShipLocomotion bridge-Z destination, braking, terminal, guard, and its promoted Giant Squid attachment prerequisite. The authoritative artifacts are:

- `docs/research/PHASE3_SHIP_BRIDGE_Z_ADJUSTMENT_GHIDRA_REPORT.md`
- `docs/plans/2026-08-27-phase3-ship-bridge-z-adjustment-design.md`

Research commit `b6811604` adds Section 17, which supersedes the stale temporal/CLEG identity, null-cell rejection, bounded attachment-only design, and unconditional Naval-detach death. It verifies the stock SQD manager construction; `LimboLaunch`; target `+0x698` 20-frame Parasite refire lock; exact `CanAttach` gates and reciprocal link order; victim-driven Foot AI updates; `Paralyzes=32767`; states 0–4 and 70/40 update cadence; wake/splash RNG and geometry; culling, XP, delayed sinking; Sonic/suppression/healing releases; adjacent Naval re-entry with `3*ROF=297`; placement-failure death; pointer expiry; persistence; and OpenTS negative evidence.

No Ship/SQD Rust implementation has started. The design is still `OPEN` and is not implementation-ready because it still proposes the superseded bounded approximation. Its next revision must promote the full verified SQD core and resolve the architecture-facing details exposed by that evidence, especially:

- exact native-table trigonometry and f32 rocking-frame ownership/order;
- real attached `AnimObject` handling for SQDG, wakes, splashes, owner-relative XYZ, listener cleanup, and delayed Reports;
- two-stage splash allocation so list-index RNG is consumed only after successful allocation;
- manager/backlink/refire-lock/destination-delay state, native post-load timer reset, digest coverage, and snapshot-version change;
- exact limbo fallback, detach/re-entry, culling, delayed sinking, and all active release routes;
- bounded upstream admission evidence for Chronosphere and IsLocomotor release callers, rather than residual or unconditional approximations.

Fresh critic 7's earlier blockers remain historical inputs to the revised design: the first destination-delay write belongs to the victim's later update rather than projectile detonation; attachment and release must be gated by real manager/link state; and the exact timer predicate is `CdTimer::remaining(frame) != 0`, including paused-start behavior. The bridge-Z core itself had no new finding: transactional command/internal/direct/scatter entry, separate Ship/Drive Z, structural destination authority, native 3D distance/saturation, strict terminal cell-plus-Z retry, geometry rejection without mutation, cancel/getter behavior, and save/hash coverage remain required.

Any unresolved, approximate, missing, or residual behavior keeps this row open. Remaining Phase 3 rows, the phase-wide reverse audit, and the single final full library suite have not begun.

## Working-tree and process state

Preserve these unrelated pre-existing untracked reports; this task did not create or modify them:

- `docs/research/PHASE3_HOUSE_UPDATE_AI_ACTIVATION_GHIDRA_REPORT.md`
- `docs/research/PHASE3_MAIN_TICK_MODAL_SCHEDULER_SEAM_GHIDRA_REPORT.md`
- `docs/research/PHASE3_UNIT_DEPLOY_HOUSE_FLAGS_GHIDRA_REPORT.md`

Before this handoff update, the branch was 74 commits ahead of `origin/main`. No merge, rebase, or cherry-pick is in progress. All delegated researchers and critics are complete. The process check returned no `cargo` or `rustc` processes, so this worktree owns no running build.

No Cargo validation was run after the recorded Spark tests because subsequent work was research/design only.

## Exact next safe action

Resume in this same branch/worktree and reconcile git/worktree/process state first. Rewrite the Ship design against report Section 17 before editing Rust. Close the exact native trig/facing path, animation allocation/ownership ordering, and bounded Chronosphere/IsLocomotor release admissions with live evidence. Commit the revised design, then submit the requirement, Section 17 evidence, prior findings, design diff, and output to a genuinely fresh read-only critic. Repair its largest finding and use another fresh critic, rechecking prior fixes, until the design passes with zero findings. Only then appoint one builder for the Ship/SQD mechanism. Reserve the single `cargo test -p vera20k --lib` run for final Phase 3 closure.
