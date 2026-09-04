---
name: write-plan
description: >
  Write or revise a requested VERA20k implementation plan or behavior contract.
  Ground changes in current source and evidence, with dependencies and acceptance
  scenarios. Does not require a preceding brainstorm or contract stage.
---

# Write Plan

Follow [ENGINE.md](../../../ENGINE.md). Reconcile the intended outcome and available
design with current owners, consumers and recent changes. Plan coherent mechanisms:
required deltas, file/symbol anchors, dependencies, production integration, obsolete
logic and acceptance scenarios with setup, action and expected result. Leave design
choices open where evidence does not constrain them; use code snippets only to
clarify consequential decisions.

When a behavior contract is requested, emphasize the comparison instead of task
breakdown: behavior/trigger, native evidence, current Rust evidence, required delta
or unresolved question, and acceptance scenario. Distinguish demonstrated mismatches,
supported matches, evidence gaps and missing coverage. Name necessary foundations,
source revision and what is ready versus unresolved; proposed checks are not results.

Honor the requested output. Saved artifacts normally use task-owned
`docs/plans/YYYY-MM-DD-<topic>-plan.md` or
`docs/plans/YYYY-MM-DD-<topic>-implementation-contract.md`.
A plan/contract-only request does not authorize implementation or source-research
patches. Already-authorized execution continues without another approval gate.
