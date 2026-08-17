---
name: rust-scan
description: "Project-scoped, read-only VERA20k Rust review for confirmed determinism/desync, authoritative-state and lifecycle, architecture, correctness/safety, and measured hot-path risks. Use for /rust-scan [path], changed-Rust reviews, Rust anti-pattern audits, determinism reviews, or code-health scans, including macro-expanded and trait-dispatched behavior when relevant. Defaults to src/sim/; reports evidence and never edits or auto-fixes."
---

# Rust Scan

Review Rust code without editing it. Treat `ENGINE.md` as authoritative and read it
completely before scanning. A scan samples risk; it never certifies parity, correctness,
or cleanliness.

Treat the bundled references as review lenses, not implementation requirements. Their
imperatives govern what this read-only scan must inspect, prove, classify, and report;
they do not prescribe production rewrites or grant permission to change code. A heuristic
or failed lexical check is not a finding until it passes the confirmation gate. Only a
later, explicitly authorized implementation task may change code, and `ENGINE.md` remains
authoritative.

Apply this priority when recommendations compete:

1. verified `gamemd.exe` behavior and player-visible parity;
2. deterministic authority, ordering, lifecycle, persistence, and RNG;
3. required architecture boundaries and 20,000-unit scale;
4. measured performance within an explicit budget;
5. API polish, idiom, and style.

Do not report less-idiomatic Rust merely because a cleaner abstraction exists. Confirm a
concrete parity, correctness, safety, scale, or maintenance effect first.

## Inputs

Preserve these forms:

- `/rust-scan` — scan `src/sim/`.
- `/rust-scan <path>` or `/rust-scan scan <path>` — scan one repository-relative
  Rust file or directory.
- `--changed [--base <revision>]` — restrict the target to changed Rust files;
  default the base to `main`.
- `--focus <name>` — repeat as needed. Names are `determinism`,
  `architecture`, `state`, `safety`, `performance`, `ownership`, and
  `structure`.
- `--include-tests` — include named and probable inline-test candidates.
- `--clippy` — run the scoped Clippy wrapper after static review.

Without `--focus`, use `determinism,architecture,state,safety,performance` for
targets intersecting `src/sim/` and `safety,structure` elsewhere.
Ownership/API-shape review is opt-in; it must not crowd out parity or correctness work.
Use `/architecture-scan` instead for repository-wide module placement, dependency
direction, public-surface, naming, cohesion, or Cargo-boundary review. The
`architecture` focus here remains limited to hard boundaries that affect the Rust
target being scanned, especially authoritative simulation isolation.

Resolve the target under the repository root, accept only `.rs` files, respect
ignore rules, and reject paths outside the repository. Record the current HEAD,
dirty state, target, selected focus, and file count. Never modify code, apply
Clippy fixes, create scan ledgers, or update Ghidra.

## Resources

- Always use [general review rules](references/general-review.md).
- For `src/sim/` or any code feeding authoritative simulation, also use
  [simulation risk rules](references/sim-risks.md).
- Use [the candidate collector](scripts/collect_candidates.py) for deterministic
  lexical discovery. Its output is evidence to inspect, never a verdict.
- Use [the Clippy wrapper](scripts/run_clippy.ps1) only when `--clippy` was
  requested.

Read the confirmation gates plus only the reference sections needed for the selected
focus and surfaced rule IDs. Some high-value checks are contextual and intentionally have
no repository-wide regex.

## Workflow

The numbered order is binding only where a step names a precondition: collect
candidates before verifying them, confirm no active `cargo`/`rustc` process
before Clippy, deduplicate and classify last. Otherwise treat the steps as
required checks, not a script — plan the pass yourself.

1. **Establish scope.** Inspect `src/lib.rs`, `src/sim/mod.rs`, the target module
   headers, and relevant callers. For simulation work, locate the current
   `World::advance_tick` / master-frame spine and its `SPINE REGION` comments
   from live code; do not trust a hardcoded filename or line.
2. **Collect candidates.** Run a summary first:

       python .claude/skills/rust-scan/scripts/collect_candidates.py --target <path> --profile auto --format summary

   Add `--changed --base <revision>`, `--include-tests`, and repeated
   `--focus <name>` as requested. Rerun with `--format jsonl --rule <RULE_ID>`
   only for rules worth inspecting. Never silently truncate a large rule. Narrow
   by changed files or an owning module when practical; otherwise mark that rule
   `PARTIAL` with its candidate count and do not infer cleanliness from examples.
3. **Verify context and Rust indirection.** Read the complete containing item and enough callers to
   establish compiled production reachability, resolved types and ranges,
   ownership, authority, and test/setup/presentation boundaries. Search results,
   comments, names, and lint text alone are not findings. When relevant behavior is
   generated by a declarative/procedural macro or derive, inspect the definition or
   expansion that actually compiles. At trait calls, resolve the reachable concrete impl,
   default or blanket method, associated types, and dispatch boundary. Macro or trait use
   alone is never an anti-pattern.
4. **Apply project semantics.** For outcome-bearing simulation code, verify
   scheduler order, equal-key tie-breakers, RNG stream and draw order, lifecycle
   ownership, same-tick visibility, serialization/hash/snapshot coverage,
   coordinate frames, and nearby gamemd provenance where applicable.
5. **Verify performance claims in two stages.** First prove a live call chain and
   multiplicity or identify the item in a representative optimized-build capture. Then
   inspect allocation, growth, copying, data access, and algorithmic cost inside that hot
   item. `Vec::new()` alone does not allocate; capacity-sensitive `push`, `insert`,
   `extend`, `reserve`, string append, and map insertion may allocate only at the actual
   call-site capacity. For budget, cache, bandwidth, or contention claims, record the
   scenario, entity count, target hardware, sampling window, subsystem time in
   milliseconds, explicit budget, and relevant counters. Without that evidence, report a
   profiling candidate rather than a confirmed performance finding. If no capture or
   budget was supplied or already exists, do not create profiling infrastructure or
   broaden the scan: confirm only bounded algorithmic multiplicity visible in code,
   state the exact measurement needed under unresolved candidates, and stop. Recommend
   reusable scratch only after proving reentrancy, clearing, ordering, snapshot/hash
   exclusion, and memory-retention safety.
6. **Run Clippy when requested.** First confirm no `cargo` or `rustc` process is
   active. Run the wrapper once and never kill it mid-build. Preserve all output
   and the exit code; for a file scan, filter diagnostics to that file only
   after Clippy exits. An unrelated build failure means `Clippy incomplete`,
   not a target finding.
7. **Deduplicate and classify.** Merge overlapping candidates by root cause.
   Keep unresolved items explicitly `CANDIDATE/UNCHECKED`. For large scopes,
   bounded read-only delegation is allowed when host policy permits, but the
   root agent must reread every reported finding and reconcile duplicates.

For authoritative determinism findings, prefer a fix-direction test that exposes the first
divergent tick: twin simulations, varied insertion or worker order, replay through the live
command path, or save/load continuation as appropriate. Rust-vs-Rust agreement is regression
evidence, not `gamemd.exe` parity evidence.

## Classification

A candidate becomes a confirmed finding only when the report can state:

- the compiled, reachable code and authority boundary;
- the concrete behavior, state, safety, or cost affected;
- the trigger and expected frequency;
- the evidence supporting the claim and what remains unknown.

Use severity from verified impact, never from the scan category:

- **CRITICAL** — confirmed production desync/canonical-state corruption,
  outcome-changing correctness fault, or forbidden architecture dependency.
- **WARNING** — confirmed safety, error-boundary, hot-path, ownership, or
  maintainability risk with a bounded player or engineering effect.
- **INFO** — low-risk improvement with demonstrated benefit.

Test-only, comment-only, setup-only, presentation-only, lookup-only, dormant, and
diagnostic cases must be labeled as such. Exact-only drift remains separate from
ordinary-play impact. Do not cap or promote severity merely because Clippy found
the issue.

## Report

Lead with a one-sentence verdict, then use:

### Coverage

- Target, HEAD, dirty state, profiles, files scanned.
- Checks completed, skipped, or incomplete, including Clippy status.
- For performance work, the release scenario, measurements, hardware, and budget—or an
  explicit statement that the result remains an unmeasured profiling candidate.

### Confirmed findings

| Severity | Confidence | Rule | File:line | Evidence | Trigger / frequency | Effect | Fix direction |
|---|---|---|---|---|---|---|---|

If none were confirmed, say: `No confirmed findings in the sampled checks; this
is not certification.`

### Unresolved candidates

List every candidate that remains unresolved; never drop one silently. For those
whose missing type, caller, authority, or runtime evidence could materially
change the verdict, state the exact next check. Group the rest by rule with a
count and a one-line reason they were left unresolved.

### Notable rejected candidates

Briefly record high-signal false positives that would otherwise be rediscovered,
such as test-only code or membership-only `HashMap` use.

### Limits

State excluded files, candidate truncation, unrun tools, and remaining unknowns.

End after reporting. Do not auto-fix findings; implementation is a separate,
explicitly authorized task.
