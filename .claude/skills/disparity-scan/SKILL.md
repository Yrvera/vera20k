---
name: disparity-scan
description: "Compare a named system's current Rust implementation with active YR evidence. Reports verified gaps separately from doc-derived candidates and ranks impact independently of exact parity."
---

# Find disparities

Bound the scan to the requested system, scenario, or file. Use
[ENGINE.md](../../../ENGINE.md), the research index, relevant reports, and recent
code to discover candidate behaviors. Read cited evidence directly; the index and
research prose do not establish parity.

Inspect the current production Rust for every reported comparison, including
callers, parsers, ownership, lifecycle hooks, and downstream consumers. A missing
symbol or an old report does not prove missing behavior. Read retail INIs from the
main checkout and use their active YR loading semantics.

Classify the evidence before ranking the gap:

- **Verified gap:** active-YR behavior is established and current Rust is missing,
  partial, or different. Cite both sides and keep the exact verdict as DRIFT.
- **Doc-derived candidate:** Rust differs from an unverified research claim.
  State the specific missing native proof; this is not implementation-ready.
- **Rust state unknown:** the current production equivalent was not established.
- **Inactive / TS legacy:** exclude from active-YR gaps only with gate and default
  evidence.
- **Match:** limit the claim to the checked boundary and evidence strength.

Use the [binary workflow](../../../docs/research/ghidra-workflow.md) for decisive
live checks when evidence is stale, ambiguous, conflicting, or consequential to
implementation. State, ordering, RNG, determinism, lifecycle, ownership, and exact
output claims require adequate active-binary support. Choose the smallest check
that resolves the uncertainty. Unresolved evidence remains a candidate, not a
confirmed gap.

Cover the requested scope, including relevant visuals, audio, INI behavior, and
boundary cases. A broad scan may identify gaps without exhaustive binary research;
name omissions. An explicitly exhaustive request requires its complete declared
coverage. Do not infer a parity percentage from a scan.

Group findings by mechanism and prerequisite, then rank trigger frequency, player
impact, compounding effects, determinism, and unblock value. A missing prerequisite
does not make dependent behavior correctly absent. Preserve blocked gaps and
low-priority drift with their evidence states.

Save a dated report in task-owned `docs/research/` unless the user chose another
location. Include scope, verified gaps, candidates, evidence, current Rust
locations, prerequisites, and remaining checks. Report meaningful false positives
and stale-doc corrections when they prevent repeated mistakes.

Keep source and research inputs unchanged. Ghidra is read-only unless synchronization
is explicitly authorized under the binary workflow; workers report candidates only.
Finish with the report and strongest conclusions. If implementation is already
authorized, continue the parent task from these findings without demanding a
separate invocation.
