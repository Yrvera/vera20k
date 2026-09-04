---
name: implementation-contract
description: >
  Distill existing VERA20k research and current Rust evidence into the exact
  behavior changes needed to close a bounded active-YR parity gap, with evidence,
  unresolved questions and acceptance scenarios. Produces a contract, not Rust code.
---

# Implementation Contract

Establish what must change for the named parity gap under
[ENGINE.md](../../../ENGINE.md). The contract separates required semantics from
implementation design. It is useful when research exists but its concrete Rust
consequences remain unclear; it is not a mandatory stage before implementation.

Read the underlying native evidence and current Rust owners/consumers. Reuse supported
research, resolving consequential conflicts or gaps with bounded investigation.
For each claimed mismatch, establish both behaviors and their relationship in the
target scenario. Existing labels, synthesis summaries and TODOs do not prove either side.

The central comparison should state:

| Behavior and trigger | Native evidence | Current Rust evidence | Required delta or unresolved question | Acceptance scenario |
|---|---|---|---|---|

Use as many rows as the actual mechanism needs. Distinguish a demonstrated mismatch,
an evidence gap, missing regression coverage and stale documentation. Absence of proof
is not proof of a particular bug. Record matches when supported, stating the domain
covered rather than requiring an artificial change to every row.

Name the likely Rust owner, production callers and necessary foundations. Preserve
native semantics without prescribing C++ layout or premature Rust APIs. For acceptance,
state setup, action, expected result and the native source of that expectation;
distinguish the proposed check from one actually executed.

For UI/render behavior, establish the relevant parent composition, active flags,
asset roles and exact frames. A loaded asset need not be drawn, and absence from one
role does not prove absence from the screen. For gameplay, include state handoffs and
readers that make a local difference observable or deterministic.

Conclude with what is ready to implement, what needs evidence, and any excluded or
deferred behavior with its justification. Priority may order work; it does not erase
a proven difference or close an exhaustive request.

Save the requested artifact, normally
`docs/plans/YYYY-MM-DD-<topic>-implementation-contract.md` in the task's worktree;
honor an explicitly requested output path.
Identify the source revision and supporting references so a later executor can reconcile
it with current code. Do not patch Rust, INIs, assets or source research as a side effect.

Ghidra analysis is read-only by default. `--sync-ghidra-labels` or a direct request may
authorize metadata synchronization under the
[Ghidra workflow](../../../docs/research/ghidra-workflow.md); a read-only request or
`--no-sync-ghidra-labels` disables it. Uncertain identities remain candidates.
