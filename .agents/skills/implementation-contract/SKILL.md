---
name: implementation-contract
description: >
  Turn existing VERA20k research and current Rust evidence into required changes
  for a bounded active-YR parity gap, with acceptance scenarios. Writes a contract,
  not implementation code.
---

# Implementation Contract

Under [ENGINE.md](../../../ENGINE.md), establish both native and current Rust
behavior for the target scenario, resolving consequential evidence gaps. Distinguish
required semantics from implementation design.

The contract's core is:

| Behavior/trigger | Native evidence | Rust evidence | Delta or open question | Acceptance scenario |
|---|---|---|---|---|

Separate demonstrated mismatches, evidence gaps, missing coverage and stale prose;
include supported matches and their coverage. Name Rust owners, production callers,
necessary foundations and what is ready versus unresolved. Acceptance scenarios
state setup, action, expected result and its native source, distinguishing proposed
checks from executed ones.

For visuals, establish parent composition, active flags and exact frames: absence
from one asset role does not prove absence from the screen.

Save in the task's worktree, normally
`docs/plans/YYYY-MM-DD-<topic>-implementation-contract.md`; honor an explicit path.
Include source revision/references. Do not patch implementation or source research.
Ghidra remains read-only unless `--sync-ghidra-labels` or a direct request authorizes
[Ghidra workflow](../../../docs/research/ghidra-workflow.md) synchronization;
read-only requests and `--no-sync-ghidra-labels` disable it.
