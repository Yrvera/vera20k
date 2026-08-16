# Worked example — the engine-domain-boundaries program (2026-08)

The prompt pair below ran a real 14-item architecture program: started by a
Codex session (F01–F04 partial), continued by a Claude Code session that
completed F04–F09 in a weekend, with every slice critic-gated and the tier
published as a PR. Use it to calibrate length and specificity — the whole
governing prompt is under a page.

## What each revision fixed (against the original draft)

The original draft's endpoint was "refactor domain-by-domain until ownership,
dependencies, state boundaries, module layout, and APIs are coherent and
idiomatic" — an unbounded bar that made all fourteen items mandatory. Four
edits fixed what execution later proved mattered:

1. Ranked scope + residual tiers → the program could stop after the
   valuable tiers (it did: F10+ deferred as maintenance).
2. Guard-first ratchet → the dependency guards landed before cleanup, so
   the architecture could only improve while work proceeded. The guards also
   caught a scanner blind spot via independent review — the ratchet itself
   needs adversarial checking once.
3. Publish-per-tier → PR opened at tier 1 instead of parking ~40 commits;
   (residual lesson: the PR still grew as later tiers stacked on the same
   branch — a stricter variant would branch per tier).
4. Tightened trait clause → the draft's justification list ("real
   test/headless boundary, dependency inversion, repeated behavior") was
   replaced by the two-implementations fact-gate, because the headless path
   was deliberately served by concrete shared construction, not a trait.

## The governing prompt (as run)

> Establish a source-backed overview of the VERA20k engine, then close
> ownership, dependency, and API defects in ranked order. Coherence is the
> direction, not the finish line: rank the ledger by player-visibility ×
> frequency first, determinism risk second, maintenance cost last; pure
> relocations and cosmetic regrouping are the lowest tier and may end as
> recorded residuals.
>
> Read AGENTS.md and ENGINE.md completely. Reconcile Git, worktrees, running
> processes, and in-progress work before editing, and again before any slice
> that moves files or splits state containers; defer those slices while
> other branches are in flight.
>
> First trace the real production architecture: app orchestration,
> simulation, rules/assets, map loading, rendering, input, audio,
> persistence, replay, and tests. Identify authoritative state owners,
> mutation paths, forbidden dependencies, duplicate authority, oversized
> orchestrators, misplaced modules, dead compatibility paths, and unclear
> APIs. Convert these findings into a finite cleanup ledger; freeze its
> scope; later discoveries become residuals unless required for correctness.
> Immediately after the trace, install source-level dependency guards with
> current violations allowlisted, before any cleanup slice.
>
> Complete the ledger one item at a time. Preserve correct behavior. For
> simulation behavior changes, establish the smallest retail contract from
> verified gamemd.exe callsites and retail data, or label it UNCHECKED.
>
> For each batch: define the boundary, have a builder implement it, run
> minimum focused `--lib` tests, then give a fresh read-only critic the
> requirement, evidence, diff, and validation output. Builders cannot grade
> themselves. Close the critic's largest meaningful gap and repeat with a
> fresh critic until passed. Commit each coherent slice, and open or update
> the feature PR after each completed tier — correctness fixes never wait on
> refactor slices to publish.
>
> Use a trait only where two real implementations already exist in
> production code; otherwise concrete types, enums, and functions. Fix
> dependency direction by moving code to the lower layer, never by inverting
> through an interface. Headless, replay, and tests run the real simulation
> through the same concrete types. No speculative one-implementation traits,
> giant relocations, mass formatting, or unrelated cleanup.
>
> Keep simulation authoritative and independent of app/render/UI/audio.
> Narrow visibility, remove duplicate state ownership, and decompose large
> state containers incrementally; a decomposition slice must prove per-tick
> hashes, RNG cursors, and snapshot bytes identical.
>
> Done means every ledger item is fixed, proven intentional, or recorded as
> a bounded residual; the guards are green; focused checks pass; one final
> `cargo test -p vera20k --lib` passes; and all completed tiers are
> published for review, not parked on the branch.

## The continuation header (as run)

> **Continuation.** This goal is mid-flight. The approved design and frozen
> ledger (F01–F14) are in
> `docs/plans/2026-08-15-engine-domain-boundaries-design.md` in the main
> checkout (resolve via `git worktree list`; the doc is gitignored and
> absent from worktrees). Do not re-trace or re-derive the ledger — adopt
> it. Completed on `feature/rust-scan-v2`: F01 (`8a413592`, `2321ee13`), F02
> (`c5c9c41a`), F03 (`7a9e9c01`), and the first bullet of F04 (`4bc0cf90`).
> Verify a Codex session no longer owns the main checkout before touching
> that branch; if it does, stop and report. First actions: run the
> fresh-critic review of the unreviewed commits above, publish the completed
> tier as a PR, install the dependency guards, then resume at the remaining
> F04 bullets in frozen order.

## External corroboration — the Gauntlet Loop postmortem (2026-07)

The viral "Gauntlet Loop" (Matt Shumer's Claude-of-Duty run) is the same
builder/fresh-critic/biggest-gap shape, and its published postmortem
independently confirms two rules worth carrying into goal prompts:

- **Sequential single-owner beats parallel fan-out for coupled concerns.**
  The repo's own numbers: three rounds of six parallel agents gained +0.46
  on its quality score while frame-ruining defects rose; one sequential
  pass with a single owner per coupled system gained +1.00 and cut defects
  66 → 26. This is the ledger's "one compiling owner per commit" rule,
  proven at someone else's expense — parallelize only independently
  checkable pieces.
- **Regressions go first.** The strongest variant re-verifies last round's
  fixes before judging anything new ("the hunter's verdicts go first in the
  next round"). Worth a clause in any goal prompt whose critics iterate:
  a fix that quietly regressed must not survive because attention moved on.

Its cautionary findings also transfer: a vibes bar ("utterly wowed") was
never met — scores plateaued at 5/10 and the run was stopped by hand; and
with no existing reference the critic "invents its own arbitrary standard
and approves work against criteria you never chose." Measurable bars and a
named reference are not optional decoration; they are what makes the loop
terminate and point somewhere.

## What the continuation achieved (why the header earns its lines)

The taking-over session, on its first turn: resolved the main checkout,
adopted the ledger without re-tracing, ran the documented Codex-ownership
check (found the goal thread paused), then launched four parallel fresh
critics on the inherited commits before writing any code. One critic
BLOCKed F03 with a real finding (tautological freeze-gate tests) that the
original builder's own tests had certified green — the strongest argument
recorded so far for both the fresh-critic rule and for critic-reviewing
inherited work instead of trusting it.
