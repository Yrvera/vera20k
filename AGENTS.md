# VERA20k Agent Instructions

**Read `ENGINE.md` completely before any work.** It is the shared project contract —
delivery bar, evidence rules, native-to-Rust translation, architecture boundaries,
coordinate frames, INI authority, reverse-engineering discipline, Tiberian Sun legacy,
System Map rules, change management, parallel-session coordination, git, and cargo. This
file adds only what is specific to autonomous runs.

## Autonomous runs

- **Own one slice at a time.** Finish it and hand off. Do not select another system, start
  another feature, or expand into a new dependency chain because it looks adjacent.
- Promote a prerequisite when it is necessary to locate, implement, or validate the
  current slice, or when a narrower patch would create duplicate authority, a temporary
  adapter, architectural drift, or predictable rework. Close the smallest coherent
  foundational capability—not merely the smallest patch—needed by the parent mechanism.
  A separable foundation is its own prerequisite slice and feature branch/PR, merged
  before its consumer. Do not absorb that system's unrelated backlog; record it as residuals.
- Newly discovered expert-only or scheduler-tail differences become recorded residuals
  unless they are required for correctness, determinism, architecture, or the ordinary
  player-visible loop being closed.
- Every sim-behavior commit names its gamemd source or labels the rule UNCHECKED — the
  full rule is in `ENGINE.md` (Evidence); it applies doubly to mid-run bug fixes.
- Proceed without an approval pause unless ambiguity would materially change the result.
  Resolve discoverable questions from current evidence, choose the smallest reversible
  in-scope assumption, record it, and continue.
- **Verify with the test tiers in `ENGINE.md`:** scoped `--lib` module tests while working;
  the full suite exactly once, before the PR is declared ready for `main`. Do not rerun
  the full suite per slice or per iteration, and do not add extra certification matrices
  beyond what the goal explicitly asks for. **Every `cargo test` MUST carry `--lib`** — a bare
  `cargo test -p vera20k` (with or without `--no-fail-fast`) also compiles and links 13
  unrelated side binaries and is never the right command; the full-suite form is
  `cargo test -p vera20k --lib`. Check `Get-Process cargo,rustc` before starting any
  cargo command; if another session owns Cargo, wait.
- **All implementation work uses a short-lived `feature/<topic>` branch from current `main`.**
  Never commit or push directly to `main`. A sole owner may use this checkout; concurrent tasks
  must use separate worktrees and branches. Continue an existing task-owned branch rather than
  splitting one task across branches. PRs target `main`; do not recreate a long-lived `dev`
  branch. Push/open a PR only when the user or goal authorizes publication.
- **Commit incrementally during implementation.** After each coherent, evidence-backed slice
  passes its focused `--lib` validation, create a descriptive commit before starting the next
  slice. Keep commits reviewable and buildable where practical; do not commit every edit or
  temporary WIP, and do not defer a multi-slice task into one giant end-of-run commit. When
  slices overlap in shared files, use exact hunk staging or finish the coupled dependency as one
  clearly named commit. The full `--lib` suite still runs exactly once at the end, and pushing
  remains authorization-gated by the rule above.
- **Bookkeeping caps.** The state file is ONE page, replaced each update, never appended —
  history lives in git, not the state file. Update the System Map only when verified work
  changed a mapped connection. Produce capture/evidence bundles only for pixel- or
  frame-parity work, not for ordinary behavior fixes. Time spent on bookkeeping an
  ordinary slice should be minutes, not a rival of the implementation itself.
- **Before any pause:** leave the current feature branch stable with no merge in progress,
  release Cargo and leave no build running, and write a handoff recording branch and commit SHAs, literal
  validation output, changed files, remaining residuals, and the exact next safe action.
  Do not select a new owner after a handoff.
- Reconcile actual state before mutating anything — git status, branches, worktrees, recent
  commits, running processes. Actual state wins over journals and prose. Never modify
  another task's files, branch, worktree, dirty checkout, or running process.

## Communication

- **Read for intent, not literal wording** — resolve ambiguity toward retail-convincing
  stock skirmish. Exact INI keys, offsets, addresses, and parity numbers stay literal.
- **The user's observed symptom is evidence; a proposed cause or fix is a hypothesis.**
  When an observation contradicts the analysis, investigate — don't defend the analysis.
- **Verdict first**, short first answers, plain language, severity always with a
  trigger-frequency clause.

## GitHub issues

Short and human-written — `## Player-visible problem` then `## Current Rust mismatch`.
No binary addresses, research dumps, or checklists unless asked.
