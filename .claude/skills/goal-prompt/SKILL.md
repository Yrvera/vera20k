---
name: goal-prompt
description: >
  Compose, revise or review a prompt for an autonomous VERA20k goal, including
  continuation in another Codex or Claude session. Preserves the user's outcome,
  evidence bar, authority and stopping condition while leaving execution choices
  open. Produces paste-ready text; never launches or schedules the goal.
---

# Goal Prompt

Write the prompt requested by the user. Follow [ENGINE.md](../../../ENGINE.md);
refer to the shared contract instead of copying its Git, Cargo or evidence rules.
This skill composes text only: it does not start a goal, task, loop or automation.

A useful prompt gives the executor:

- The intended outcome, why it matters and the actual scope/exclusions.
- A reference it can inspect and compare against: active gamemd and retail data for
  parity; identified behavior-preservation checks for refactors; the user's concrete
  workflow and acceptance properties for tooling.
- Evidence of completion through the real production path.
- Any task-specific authority, budget, model choice and stopping condition.

Use established project intent when the reference is clear. Resolve discoverable
questions; ask only if a missing user decision would materially change the result.
Keep explicit publication authority, model/effort choices and time/token limits exactly
as given. Do not add them when absent. An example's permission is never permission
for the current task. Preserve a supplied invocation such as `/loop` when adapting it.

For substantial goals, describe coherent transactions with independent review using
[review guidance](../_shared/review.md). The critic can inspect original evidence and
challenge scope, design, omitted mechanisms and tests. Leave architecture, decomposition,
tools and research order to the executor except where the task has a real constraint.
Do not append mandatory scan suites or an arbitrary number of critique rounds.

Preserve the requested completion standard. In exhaustive parity work, plan rows are
ownership hypotheses: discovered in-scope mechanisms and unresolved required behavior
remain open. A ranked effort may stop with explicitly allowed deferrals; never silently
convert one form into the other. Completion of a transaction does not stop an authorized
multi-transaction goal. State authorized publication/merge cadence when it matters to
the goal; do not infer permission to publish from a request to write a prompt.

## Continuation

Read the governing prompt, latest user amendments and available current task/Git state.
Add only the information needed to resume: worktree/branch, relevant HEAD and unmerged
work, evidence/artifact locations, review/validation state, remaining scope and next
safe action. See [handoff guidance](../_shared/handoff.md).

Adopt useful existing work rather than restarting discovery, but recheck stale or
contradicted claims and ownership before acting. A historical scope inventory cannot
prevent new primary evidence from exposing missing required behavior. Do not revive
work the user stopped or silently reset the budget or publication boundary.

Output paste-ready text, respecting any requested length. Save or revise a prompt file
only when requested. If reviewing a prompt, explain material changes and preserve its
meaning. [Examples](references/engine-boundaries-example.md) illustrate the shape;
they are not additional requirements.
