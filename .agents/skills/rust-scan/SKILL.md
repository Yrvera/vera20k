---
name: rust-scan
description: "Review VERA20k Rust for determinism, state/lifecycle, correctness, safety, and performance risks. Supports paths, changed-code reviews, and focused scans; defaults to src/sim/. Reports findings without editing."
---

# Rust scan

Review the requested Rust against `ENGINE.md`. Use the
[shared review guidance](../_shared/review.md); report engineering risk separately
from exact gamemd parity. A scan does not certify either.

## Scope and tools

- No target selects `src/sim/`; a path or `scan <path>` selects a Rust file/module.
- `--changed [--base <revision>]` selects changed Rust; default base: `main`.
- Repeat `--focus` for `determinism`, `architecture`, `state`, `safety`,
  `performance`, `ownership`, or `structure`.
- Default focuses are determinism, architecture, state, safety, and performance
  for simulation; safety and structure elsewhere. API/style review is optional.
- `--include-tests` includes test candidates. `--clippy` requests the bundled
  Clippy wrapper; it never applies fixes.

Use [general lenses](references/general-review.md) and, for code feeding
authoritative simulation, [simulation lenses](references/sim-risks.md). Read the
sections relevant to the scope and candidate rule IDs.

For broad lexical discovery, run from the repository root:

```text
python .agents/skills/rust-scan/scripts/collect_candidates.py --target <path> --profile auto --format summary
```

The collector supports the scope flags above; `--format jsonl --rule <RULE_ID>`
returns details. Its matches are candidates, not findings or complete coverage.
A direct source review is enough for a small target. Account for uninspected
candidates or truncated output rather than inferring cleanliness from examples.

## Review focus

Read containing items and callers to establish production reachability, types,
ranges, authority, and test/setup/presentation boundaries. Resolve relevant macro
expansions and concrete trait implementations; syntax and lint wording alone do
not prove a defect. Use `architecture-scan` for broader module/package placement.

For simulation, follow the live `advance_tick` / master-frame spine and
its `SPINE REGION` comments. Check scheduler/tie order, RNG stream and draw order,
lifecycle effects, same-tick visibility, snapshot/hash coverage, coordinates,
and gamemd provenance where the change can affect them.

Performance findings need a reachable hot path and multiplicity or a
representative optimized-build measurement. Resolve actual allocation/copy
behavior: `Vec::new()` alone does not allocate; growth depends on capacity.
Cache, bandwidth, contention, and budget claims need corresponding measurements.
Without them, retain a profiling candidate and describe the missing evidence.
Do not create profiling infrastructure as a side effect of a review.

When Clippy is requested, invoke this skill's `scripts/run_clippy.ps1` with the
working directory inside the repo. It checks Cargo ownership and uses the
library target. Preserve the exit code and diagnostics; an unrelated build
failure makes Clippy incomplete, not the scanned code defective.

## Result

Report findings with source locations, evidence, consequence, trigger/frequency,
and fix direction. Separate unresolved candidates and notable rejected matches;
large unresolved groups can be summarized by rule and count. State checkout/HEAD,
scope, tools run, and missing evidence. Do not silently discard findings for being
low priority or because a concrete risk was discovered outside a regex rule.

For determinism fixes, suggest a check that exposes the first divergent tick,
using live commands, varied ordering, replay, or save/load continuation as
appropriate. Rust-vs-Rust agreement is regression evidence, not gamemd parity.
