---
name: write-plan
description: >
  Write or revise a VERA20k implementation plan when explicitly requested.
  Describe coherent changes, dependencies and acceptance checks; use review-plan
  for assessment without edits.
---

# Write Plan

Follow [ENGINE.md](../../../ENGINE.md). Use the intended outcome and available
design; a separate brainstorm document is not required. Reconcile assumptions
with current source, consumers and relevant recent changes.

Save the requested plan, normally
`docs/plans/YYYY-MM-DD-<topic>-plan.md` in the task's worktree. Include:

- Outcome, scope, completion condition and supporting evidence.
- Coherent tasks with file/symbol anchors, required deltas, dependencies,
  affected consumers and production integration.
- Necessary foundations/migrations, obsolete logic, unresolved questions and
  acceptance scenarios with setup, action and expected result.

Plan complete mechanisms, leaving the executor freedom to adjust boundaries as
evidence develops. Short signatures or pseudocode can clarify decisions; full
implementations and test bodies usually become stale before execution. Use
ENGINE's validation tiers rather than a ritual for each row.

A plan-only request ends with the plan. Already-authorized execution continues
without another approval gate. For read-only assessment, use
[review-plan](../review-plan/SKILL.md).
