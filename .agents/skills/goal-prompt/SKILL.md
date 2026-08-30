---
name: goal-prompt
description: >
  Compose or revise a goal prompt for a long-running autonomous VERA20k
  program (Claude Code or Codex), or write the continuation header that lets a
  fresh session take over a goal mid-flight. Use when the user asks to write,
  redesign, tighten, or review a goal prompt, wants to launch a multi-slice
  autonomous program, or wants an in-progress goal handed to another session.
  Produces paste-ready prompt text; never launches the program itself.
---

# Goal Prompt — Autonomous Program Launcher

A goal prompt is the one document a long-running session re-reads when its
context compacts, its plans drift, or a fresh session takes over. Every
sentence either constrains the program or wastes the executor's attention —
write it like a contract, not a briefing.

Two modes. Decide first, because they produce different documents:

- **New goal** — the program does not exist yet. Produce the full prompt.
- **Continuation** — a program is mid-flight (often started in another tool,
  e.g. Codex). Produce a continuation header + the governing prompt. Skipping
  the header makes the new session re-trace, re-derive, or fork the branch.

## The reference is yours to supply

An autonomous loop is good at closing the gap between what exists and the
reference you gave it. It cannot decide what the reference should be. Name
the reference explicitly in every goal prompt: for sim behavior it is gamemd
and the delivery bar (ENGINE.md); for refactors it is behavior-preservation
proven by hashes and bytes; for tooling it is the named user workflow. A
prompt whose bar is a vibe ("clean", "idiomatic", "coherent") authorizes
unbounded work — the program will happily consume weeks matching a reference
nobody chose.

A valid bar passes three tests: **named** (a specific thing, not a
category), **fetchable** (the critic can read it, run it, decompile it, or
diff against it), and **comparable** (the work and the bar can sit side by
side under one judgment). Everything else in the prompt is scaffolding —
the loop only produces quality if the thing it compares against is real.
If the user hasn't named a bar, propose two or three concrete candidates
and stop for their pick before writing the prompt; never invent the
reference silently, because the executor's critics will then invent their
own.

## New-goal template

Six parts, in this order. Keep the whole prompt near one page; the executor's
fresh-critic loop is the catch-all for cases the prompt doesn't anticipate —
do not try to pre-decide everything.

1. **Outcome and closure model.** State what exists when this is done, then
   choose the closure model explicitly. A ranked program orders work by
   player-visibility × frequency first, determinism risk second, and
   maintenance cost last; name the tiers that may end as bounded residuals.
   An exhaustive parity program instead requires every discovered in-scope
   mechanism to close and treats every residual as open. Never silently
   soften an exhaustive goal into a ranked one.
2. **Ground rules.** Read AGENTS.md and ENGINE.md completely. Reconcile git,
   worktrees, running processes, and in-progress work before editing — and
   again before any slice that moves files or splits state containers; defer
   those slices while other branches are in flight.
3. **Trace → reviewable inventory → ratchet.** Begin with bounded, read-only
   discovery of the real production architecture; convert findings into a
   finite, reviewable starting inventory. Plan rows, research documents,
   current owners, and secondary references are scope hypotheses, not proof
   of the denominator. The inventory is a coverage hypothesis, not a parity
   verdict: before implementing each transaction, verify its material
   contract against the named reference. Let later primary evidence reopen or
   split an entry. Group work into the smallest dependency-coherent,
   independently testable transactions; do not mechanically equate a
   transaction with one plan row. Immediately after the trace, install the
   cheap enforcement that stops drift while work proceeds (dependency guards
   with current violations allowlisted, a golden, a schema check — whatever
   ratchets this domain).
4. **Execution protocol.** One coherent slice at a time. Builder implements;
   smallest relevant `--lib` filter; a fresh read-only critic gets the
   requirement, evidence, diff, and actual validation output — **not the
   builder's reasoning** (the builder remembers why every decision felt
   right; the critic must judge the artifact, not the narrative). Builders
   never grade themselves. The critic passes a slice only when its diff and
   non-interactive validation **reproduce the contract** without unsupported
   approximations — a defined pass condition, not a review ritual. On
   BLOCK: close the largest meaningful gap and resubmit to a fresh critic
   until pass; re-verify the previous round's fixes before judging anything
   new; commit. When publication is authorized, **publish per tier**: open or
   update the PR when a tier completes — correctness fixes never wait on
   refactor slices. For parity work, every mechanism records `LIVE` evidence
   (the decisive native body and representative callsites were checked),
   `DOC` evidence (an exact named verified-report section already contains
   those checks), or `UNCHECKED`; labels alone are never evidence.
   Verification is production-observable ("it compiled" and "it plays" are
   different checkpoints — pin the one that matters). For new, ambiguous,
   contradicted, label-dependent, or load-bearing parity claims, require the
   critic to reopen the decisive native body and representative callsites.
   A `DOC` claim may be reused only when its cited section contains that
   evidence.
5. **Structure rules.** A trait only where two real implementations already
   exist in production. Fix dependency direction by moving code to the lower
   layer, never by inverting through an interface. Headless, replay, and
   tests run the real simulation through the same concrete types. No
   speculative abstractions, giant relocations, mass formatting, or
   unrelated cleanup. (Each clause exists because its generic form has been
   exploited: "test seam", "dependency inversion", and "repeated behavior"
   all re-admit one-implementation traits.)
6. **Done-clause.** Apply the selected closure model: every exhaustive item
   closed, or every ranked item fixed, proven intentional, or recorded in an
   explicitly deferrable tier. Require the ratchet green, one final full
   `cargo test -p vera20k --lib`, and a whole-program reverse audit for
   unassigned behavior, cross-transaction gaps, and regressions. Publish only
   as authorized by repository policy; otherwise leave coherent commits and
   an exact handoff.

## Controls for broad implementation programs

Carry these controls into a goal when it spans multiple mechanisms or shared
engine state:

- Use one dependency-coherent transaction per implementation task,
  `feature/*` branch, and PR. Start from current `origin/main`; never begin
  new work from a merged feature branch. Keep a small inseparable prerequisite
  with its consumer, but give a broad shared prerequisite its own transaction.
- Before changing shared constructors, lifecycle, placement, production,
  pathing, schemas, snapshots, or deterministic state, define focused
  cross-system canaries. On failure, compare with `origin/main`, reproduce a
  representative case serially, and only then form a systemic theory.
- Keep one reviewable work inventory containing mechanism, evidence,
  exclusions, owner, transaction, task/branch/PR, critic verdict, validation,
  residuals, and status. Derive status from current code, Git history, named
  checks, and machine-generated evidence; a prose tracker is never parity
  proof. Report current-mechanism, current-PR, and whole-goal status
  separately.
- Treat a diff that outgrows its title or crosses an independently reviewable
  subsystem boundary as a scope alarm. Split or retitle before continuing.
  Keep red work draft, never weaken tests or lifecycle gates for green output,
  and report local validation and CI as separate readiness gates. Absent CI is
  absent, not passed.

## Two continuation models — choose before writing

A program outliving one session continues in one of two ways; pick
deliberately, because they produce different prompts:

- **Idempotent re-entry** (prefer when available). Write the prompt as a
  convergent pass, not a linear program: "Preserve what already matches,
  replace what is wrong, and implement what is missing" over an external
  frozen ledger (a phase/plan document). Every invocation re-derives the
  frontier by checking each item's disposition against the bar — no
  memory, no handoff, no stale done-inventory to trust. When a run dies
  (usage limit, crash), fire the same prompt again. This works exactly
  when a fresh executor can determine what is done by inspecting the
  artifact against the bar; it is the proven pattern of the Codex phase
  prompts, which converged over repeated runs with zero handoff text.
- **Continuation header** (below). Needed when the truth lives in session
  state rather than in the work: mid-slice on a shared branch, unreviewed
  inherited commits, a tool switch, an ownership question. The header
  transfers exactly the facts the world cannot re-derive.

The test: if re-running from scratch would waste nothing but a little
re-checking, write idempotently and skip the header.

## Continuation header template

Prepend this to the governing prompt, filled in concretely:

> **Continuation.** This goal is mid-flight. The approved design/ledger is at
> `<doc path>` (tracked under `docs/plans/`, so it is present in every
> worktree and clone). Do not
> re-trace or re-derive — adopt it. Completed on `<branch>`: <item → SHA
> list>. Verify no other session or tool owns the checkout/branch before
> touching it (<how to check>; if owned, stop and report). First actions:
> <e.g. critic-review the unreviewed commits, publish the completed tier,
> install the ratchet, resume at item N>.

The header exists because three things go wrong without it: the new session
re-runs the expensive trace; it cannot find the design doc (main-checkout
gitignore); it forks a second branch and orphans the finished work. Each
line of the header closes one of those.

## Failure modes this template encodes against

Observed, not hypothetical — check a draft against each:

- **Unbounded endpoint**: "until coherent/idiomatic" has no stop; ranked
  tiers with residuals do.
- **No merge cadence**: "ready for review" only in the done-clause parks
  weeks of unmerged commits while every parallel branch drifts against them.
- **Reconcile-once**: worktree checks only at start miss branches created
  after launch; high-churn slices need a re-check and a deferral clause.
- **Escape-hatch clauses**: every justification list ("...or a test
  boundary, or dependency inversion") will be read as authorization by a
  compliant executor. Gate on facts ("two implementations exist"), not
  purposes.
- **Prompt bloat**: a clause added "just in case" competes with the load-
  bearing ones. The critic loop handles the unanticipated; the prompt only
  needs the rules the critic can't infer.
- **Self-graded verification**: any bar the builder can attest to itself
  ("tests added", "looks correct") will be attested. Bars are literal test
  output, hashes, bytes, or a fresh critic's verdict.

## Worked example

Read [references/engine-boundaries-example.md](references/engine-boundaries-example.md)
for the real prompt pair that ran the 2026-08 engine-domain-boundaries
program: the revised new-goal prompt, the continuation header that moved it
from Codex to Claude Code without re-tracing, and one paragraph on what each
revision fixed. Use it as the calibration for length and specificity.
