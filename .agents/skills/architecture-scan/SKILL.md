---
name: architecture-scan
description: "Project-scoped, read-only VERA20k review of repository organization, module ownership, dependency direction, API visibility, naming, cohesion, change locality, test placement, and Cargo target boundaries. Use only when explicitly invoked as $architecture-scan to assess where existing Rust code belongs or whether current repository boundaries are healthy. Reports evidence-backed findings and options; never edits, auto-refactors, performs gameplay parity research, or imposes generic Rust layout preferences."
---

# Architecture Scan

Review the current repository architecture without editing it. Read `ENGINE.md`
completely before scanning. A scan samples current structure; it does not certify
that the repository is well organized.

Judge architecture by whether ownership is predictable, dependencies flow in the
declared direction, APIs protect invariants, and ordinary changes remain reasonably
local. Do not judge it by a preferred folder tree, crate count, nesting depth, or
one-type-per-file rule.

Treat all bundled material as evidence lenses, not implementation requirements.
Its imperatives govern what this read-only review must inspect and prove; they do
not authorize moves, renames, API changes, crate splits, or refactors. Only a later,
explicitly authorized task may change code, with `ENGINE.md` remaining authoritative.

Apply this priority when recommendations compete:

1. explicit `ENGINE.md` boundaries and parity/determinism safety;
2. state and behavior ownership plus dependency direction;
3. API visibility, invariant enforcement, and lifecycle seams;
4. cohesion, change locality, testability, and discoverability;
5. naming consistency and aesthetic preference.

## Inputs

Preserve these forms:

- `$architecture-scan` — review the current package, beginning with `Cargo.toml`,
  `src/lib.rs`, `src/main.rs`, and the top-level module roots.
- `$architecture-scan <path>` — review one repository-relative file, module, or
  directory in its surrounding architecture.
- `--changed [--base <revision>]` — review structural effects of changed Rust and
  manifest files; default the base to `main`.
- `--focus <name>` — repeat as needed. Names are `boundaries`, `ownership`, `api`,
  `layout`, `naming`, `tests`, and `packages`.

Without `--focus`, use `boundaries,ownership,api,layout`. Include `packages` when
manifests or targets are in scope and `tests` when test placement affects the
question. Resolve paths under the repository root and record HEAD, dirty state,
active worktrees, target, focus, and exclusions. Do not assess another task's
half-finished move as settled architecture; identify the active migration and
inspect both its old and new seams.

## Resources

Read [the architecture review lenses](references/review-lenses.md) completely.
They contain project-aware questions plus the primary Rust sources behind them.
No candidate collector is bundled: imports, visibility, macros, and module paths
must be resolved from the live source rather than treated as a reliable regex graph.

## Workflow

1. **Establish the declared shape.** Read the relevant Cargo manifests,
   `src/lib.rs`, `src/main.rs`, module roots, and their `//!` contracts. Identify
   package targets, layer rules, public facades, and any in-progress migration.
2. **Build a factual boundary map.** For each responsibility in scope, record its
   current owner, mutable state, public or crate-visible seam, direct callers,
   upstream inputs, and downstream dependencies. Keep this map small enough to
   explain the question; do not create a permanent architecture ledger.
3. **Trace real access.** Read definitions, imports, re-exports, constructors,
   mutation sites, trait implementations, macro expansions when relevant, tests,
   and representative callers. Distinguish production, test-only, tool-only,
   generated, and presentation paths.
4. **Apply only relevant lenses.** Check explicit project boundaries first, then
   ownership, dependency direction, API surface, cohesion, placement, naming,
   tests, and package boundaries as selected. A count or lexical match is only a
   route to inspect.
5. **Prove concrete cost.** Confirm either an explicit project-contract violation
   or a bounded effect: duplicated authority, a wrong-way dependency, bypassable
   invariants, split lifecycle, unwanted API coupling, recurring cross-module
   change fanout, an untestable seam, or materially misleading placement/name.
   Use focused git history only when claiming recurring churn; do not infer design
   merely from co-change counts.
6. **Check Rust indirection contextually.** Resolve the consumers and implementors
   of a trait and the generated surface of a macro. More traits, generics, facades,
   or macros are not inherently better architecture. Recommend them only when an
   observed variation or boundary benefits.
7. **Bound the direction.** State the smallest safe improvement direction and its
   compatibility, parity, determinism, ordering, persistence, build, and migration
   risks. Do not turn the scan into a design spec or implementation plan.
8. **Report and stop.** Never move files, change visibility, add wrappers, split a
   crate, run automatic fixes, or chain into another skill.

Use `cargo metadata --no-deps` only when manifest topology is ambiguous. Before
any Cargo command, check for active `cargo` or `rustc` processes and follow the
repository's Cargo coordination rules. Do not run Clippy or compile merely to
produce an architecture opinion.

## Confirmation gate

A candidate becomes a finding only when the report can state:

- exact source locations and representative consumers;
- the current responsibility owner and dependency/API boundary;
- the violated project rule or concrete engineering/player consequence;
- the triggering kind of change or runtime path and expected frequency;
- why the proposed direction improves that boundary without speculative layers.

File length, folder depth, `pub`, re-exports, prefixes, traits, macros, many
binaries, or a single large crate are not findings by themselves. Generic Rust
taste is never a project defect.

Use these categories:

- **BOUNDARY** — declared dependency direction or layer ownership is violated;
- **OWNERSHIP** — authoritative state or behavior has competing/bypass owners;
- **API** — visibility or a facade exposes/bypasses more than its contract needs;
- **COHESION** — unrelated responsibilities create verified change amplification;
- **PLACEMENT** — location materially obscures ownership or dependency direction;
- **NAMING** — terminology causes demonstrated ambiguity at a real seam;
- **PACKAGE** — package, crate, binary, feature, or dependency topology is at issue;
- **TESTABILITY** — placement or visibility prevents the right boundary from being tested.

Assign severity from verified impact and always state trigger/change frequency:

- **CRITICAL** — explicit hard boundary violation, duplicated authoritative owner,
  or structural path that threatens deterministic/player-visible correctness;
- **WARNING** — confirmed coupling, invariant bypass, change amplification, or
  test barrier with a recurring or bounded maintenance effect;
- **INFO** — a demonstrated low-risk discoverability or API improvement.

## Report

Lead with a one-sentence verdict, then use:

### Coverage

- Target, HEAD, dirty/worktree state, focuses, and files or edges inspected.
- Explicit contracts and tools used, skipped, or incomplete.

### Current architecture map

| Responsibility | Current owner | Public seam | Inputs / consumers | Evidence |
|---|---|---|---|---|

### Confirmed findings

| Severity | Confidence | Category | File:line | Evidence | Trigger / frequency | Effect | Improvement direction |
|---|---|---|---|---|---|---|---|

If none were confirmed, say: `No confirmed architecture findings in the sampled
scope; this is not certification.`

### Unresolved candidates

List only candidates whose missing caller, owner, history, or boundary evidence
could materially change the verdict. State the exact next check.

### Supported boundaries

Record a few important boundaries that current evidence shows are healthy so a
later cleanup does not erase them accidentally.

### Limits

State excluded areas, unresolved macro/trait paths, uninspected history, and other
unknowns. End after reporting; implementation begins only under a separate request.

## Non-goals

- Do not perform a gameplay parity audit, broad Rust correctness/safety scan,
  performance profile, feature design, design-document review, or implementation.
- Do not produce an architecture score, ideal directory tree, parity ledger, or
  repository-wide cleanup backlog.
- Do not prescribe traits everywhere, one type per file, shallow/deep nesting,
  arbitrary line limits, universal `pub(crate)`, or a workspace split by size.
- Do not replace working project boundaries solely to resemble another engine or
  a generic "clean architecture" diagram.
