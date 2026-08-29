# Phase 3 Map/Spatial Closure Handoff

**Checkpoint:** 2026-08-29, stopped immediately after the sixth Explodes/Temporal design critic failed on active `IsGattling` target behavior

**Worktree:** `C:\Users\enok\Documents\ra2-rust-game-phase3-map-spatial-close`

**Branch:** `feature/phase3-map-spatial-close`

**HEAD before this handoff update:** `9b700b40 docs(sim): verify gamemd Temporal target completion`

**Publish state:** local only; no push, PR, merge, or main-checkout integration was authorized or performed

## Completed work

The promoted Building Action 119/House destruction sweep plus bounded `Explosion=`/`DestroyAnim=` prerequisite closed through independent builder/critic repair cycles. Its implementation range is `3287e425..d6f9aeca`; the final critic passed at `d6f9aeca`. The important closing commits include `dd21ba74` (Building destruction anim construction), `92eb1cc4..0db2bbc7` (native AnimClass shadow/layer behavior), `bcb8ba1f` (Building Start-smudge interleave), and `d6f9aeca` (Wave Building Start-smudge authority).

The next BuildingType `Explodes=yes` row has complete committed evidence through the following local commits:

- `fe7ec733` — Building Explodes retained lifecycle;
- `ccf4d2ed` — Building survivor/absorber Cargo;
- `e61a6438` — independent `CanBeOccupied` Building vector;
- `84546436` — corrected Explodes report occupant ownership;
- `ad5c7536` — occupied-Building Temporal erase; and
- `9b700b40` — all active Temporal target-class completion, SHA256 `68E06233CB7B576D4F8517F2E4AF4BA617D6A8ED8E21C3D7B9B075A4BFAF7CC1`.

No Explodes/Temporal Rust implementation began. No Cargo validation was run for this open row. The phase-wide reverse audit and the single final `cargo test -p vera20k --lib` have not run.

## Open work and blocking finding

The task-owned provisional design is preserved untracked at `docs/plans/2026-08-29-phase3-building-explodes-lifecycle-design.md` (1,174 lines, 244,109 bytes). Its Testing Strategy contains exactly 128 sequential cases. It integrates Building fatal retention, survivor Cargo, the independent occupant vector, and production Temporal delivery/completion for Infantry, Unit, Aircraft, and Building, but it is **not approved**.

Fresh read-only critic 6 returned FAIL:

> Native `TemporalClass::InitiateWarp @ 0x0071AF20` calls `UpdateGattlingStage(1)` on attachment. Retail `YTNK` and `YAGGUN` author `IsGattling=yes`, so stock CLEG/IFV attacks can alter their active weapon-stage state. The design names the call but does not own its parser/runtime state, progression/decay effects, persistence/hash, or YTNK/YAGGUN production tests; current Rust still records gattling-stage weapon selection as residual.

The smallest required correction is a focused active-retail `UpdateGattlingStage(1)` investigation followed by a design revision and a new fresh critic. That investigation was dispatched, then interrupted immediately when the user ordered this run to stop. It produced no report file, code, Cargo output, commit, or Ghidra mutation. The row therefore remains open; do not implement from the current design.

## Files and checkout state

Feature worktree tracked state is clean after this handoff commit except for deliberately preserved untracked files. The task-owned partial file is:

- `docs/plans/2026-08-29-phase3-building-explodes-lifecycle-design.md`

These unrelated pre-existing protected reports remain untracked and were not modified or staged:

- `docs/research/PHASE3_HOUSE_UPDATE_AI_ACTIVATION_GHIDRA_REPORT.md`
- `docs/research/PHASE3_MAIN_TICK_MODAL_SCHEDULER_SEAM_GHIDRA_REPORT.md`
- `docs/research/PHASE3_UNIT_DEPLOY_HOUSE_FLAGS_GHIDRA_REPORT.md`

This task wrote exactly one file into the main checkout, where it remains untracked and preserved:

- `C:\Users\enok\Documents\ra2-rust-game\docs\research\PHASE3_BUILDING_DESTRUCTION_EFFECT_ANIMS_GHIDRA_REPORT.md` — 37,645 bytes, SHA256 `7C7BFB07996BAAE61E3073FC48A21A46937E9BCB7BEF56D3706FBC7DB6C8CBCE`.

No other current main-checkout dirty path intersects `3287e425^..9b700b40` or the task-named Explodes/Temporal artifacts. The main checkout has substantial unrelated pre-existing modified/untracked work; none was changed, staged, deleted, or committed by this wrap-up.

The final process check returned no output for `Get-Process cargo,rustc`; this worktree owns no build. No merge, rebase, or cherry-pick is in progress. No files are staged beyond this completed handoff update.

## Exact next safe action

Remain stopped until the user explicitly resumes this same row. On a later authorized resume, first reconcile branch/worktree/process state, then finish only the focused active-retail `TemporalClass::InitiateWarp -> UpdateGattlingStage(1)` prerequisite for stock `YTNK` and `YAGGUN`. Commit only a fully read-back evidence report, revise the existing 128-case design without discarding prior contracts, and submit the requirement, all native evidence, design diff, critic-6 finding, and validation output to a genuinely fresh read-only critic. Fix its largest finding and repeat with another fresh critic until PASS. Do not begin Rust implementation, another mechanism, the phase-wide audit, or the full library suite before that gate closes.
