---
name: write-plan
description: >
  Write or revise a VERA20k implementation plan when explicitly requested.
  Ground it in the intended outcome, available design and current source; describe
  coherent changes, dependencies and acceptance checks without prewriting the code.
  Use review-plan for a read-only assessment of an existing plan.
---

# Write Plan

Turn the requested outcome or design into an executable plan under
[ENGINE.md](../../../ENGINE.md). Use the named design or the one established in
conversation. A separate brainstorm document is not a prerequisite: establish the
missing design context directly when the request supplies enough intent.

Read current owners, consumers and relevant recent changes before assigning work.
Plans and research may describe an older checkout. Reconcile changed assumptions
against source and the evidence standard; carry unresolved questions honestly instead
of supplying invented signatures, constants or native behavior.

Save the requested plan, normally at `docs/plans/YYYY-MM-DD-<topic>-plan.md` in the
task's worktree. Include what the executor needs:

- Outcome, scope, governing design/evidence and completion condition.
- The proposed ownership and integration changes, with actual file/symbol anchors.
- Coherent implementation tasks, each naming its purpose, required behavioral delta,
  dependencies, affected consumers and concrete acceptance scenario.
- Migration or prerequisite work needed to deliver the production path, plus what
  existing logic becomes obsolete.
- Remaining uncertainties, evidence needed to resolve them and explicit exclusions.

Organize tasks around complete mechanisms and sensible dependency boundaries.
Production integration belongs with the behavior it delivers; do not plan a long
sequence of disconnected helpers. Let the executor revise task boundaries when
current evidence requires it, preserving the user's outcome and constraints.

Use signatures, pseudocode or short examples only where they clarify a consequential
decision. Full implementations, test bodies and repeated excerpts from the design
usually go stale before execution. Specify test setup, action and expected result,
and distinguish regression coverage from parity evidence. Select validation using
ENGINE's tiers rather than adding a test/commit ritual to every plan row.

Before delivering, check that the tasks cover the requested outcome, source references
resolve, dependencies make sense and production validation can expose the suspected
failure. A missing template heading is not itself a defect.

A plan-only request ends with the plan. When the user already authorized execution,
continue within that authority; writing the plan does not create a new approval gate.
For assessment without edits, use [review-plan](../review-plan/SKILL.md).
