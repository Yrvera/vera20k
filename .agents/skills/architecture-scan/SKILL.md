---
name: architecture-scan
description: "Review VERA20k module ownership, dependency direction, API boundaries, and code placement. Use when explicitly invoked as architecture-scan. Reports findings and improvement options without refactoring."
---

# Architecture scan

Assess the requested repository boundary against `ENGINE.md` and its actual
consumers. The deliverable is an evidence-backed review, not an ideal folder
tree or a refactor. Use the [shared review guidance](../_shared/review.md).

## Scope

- With no target, begin at `Cargo.toml`, `src/lib.rs`, `src/main.rs`, and the
  top-level module contracts.
- A path selects that file or module in its surrounding architecture.
- `--changed [--base <revision>]` selects structural changes; default base: `main`.
- `--focus` may select `boundaries`, `ownership`, `api`, `layout`, `naming`,
  `tests`, or `packages`. Otherwise focus on boundaries, ownership, API, and layout.

Record the checkout/HEAD and inspected scope. Distinguish an active migration
from settled architecture using worktree state and relevant history.

## What to establish

Read the relevant sections of [review lenses](references/review-lenses.md).
Trace responsibilities through their state owners, constructors, mutation paths,
public seams, and representative callers. Resolve re-exports, trait dispatch,
macros, and conditional compilation when they affect the boundary.

Look for competing authority, reversed dependencies, bypassed invariants, split
lifecycle, unnecessary coupling, recurring change fanout, and test barriers.
Support recurring-cost claims with history showing why those files changed
together. Recommend an improvement only when its benefit is concrete.

File size, nesting, `pub`, traits, macros, re-exports, or crate count are clues,
not defects by themselves. A single-implementation trait can be justified;
multiple implementations do not establish that an abstraction is useful.
Preserve verified native semantics when considering a Rust ownership change.

Use `cargo metadata --no-deps` if needed to resolve manifest topology, following
`ENGINE.md` coordination. An architecture review ordinarily needs no compilation.

## Result

Lead with the findings and their consequences. For each, identify the current
owner, affected consumers, source locations, trigger/frequency, and a bounded
improvement direction with migration risks. Include a small ownership map only
when it helps explain the result.

Report unresolved boundary questions and inspected scope. Preserve useful
existing boundaries in the recommendations; do not turn a sampled review into a
repository health score or an unrelated cleanup program.
