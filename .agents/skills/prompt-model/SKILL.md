---
name: prompt-model
description: >
  Draft or assess a prompt or skill for a capable coding agent, preserving the
  user's intent while removing redundant or over-prescribed instructions. Use for
  prompt-model draft/audit, adapting an older prompt, or model-specific prompting
  questions. Reports edits or produces text; does not silently rewrite files or
  change model settings.
---

# Prompt Model

For `draft`, produce or adapt the requested prompt. For `audit`, assess the named
prompt or skill and report consequential improvements; edit files only when asked.
For autonomous VERA20k goals, use [goal-prompt](../goal-prompt/SKILL.md).

Begin with the task's actual outcome, necessary context, constraints, reference and
completion condition. Preserve explicit scope, tool authority, publication permission,
model settings, budgets and output requirements. A concise prompt can still specify
a demanding, exhaustive result.

Remove text that does not improve a decision: duplicated project rules, stale examples,
fixed question or alternative counts, ceremonial approval/review loops and mandatory
step sequences for work that needs judgment. Keep exact procedures where order affects
correctness, such as an authorized Ghidra metadata save/readback operation.

Do not delete evidence standards, meaningful validation or independent review merely
because a model is thought to self-check well. Give reviewers access to original sources
and freedom to discover omissions. Distinguish confirmed defects from uncertain leads
without filtering away relevant mechanisms in advance.

Model-specific behavior, capabilities and available settings change. If the request
needs those facts, check the current tool/environment capabilities and official provider
guidance, cite the source and date, and distinguish documentation from local experience.
Otherwise write a model-neutral prompt; a missing model name is not a reason to stop.
Do not retain performance folklore, guessed effort ladders or claims that one model
always needs the opposite delegation policy from another. Change runtime settings only
with explicit authority, through the appropriate tool rather than prompt prose.

Deliver the requested text or a short assessment with the proposed changes and their
purpose. Use a concrete example only when it clarifies the output; it must not smuggle
in permissions, expand scope or contradict the governing project contract.
