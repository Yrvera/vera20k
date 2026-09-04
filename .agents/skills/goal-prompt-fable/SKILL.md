---
name: goal-prompt-fable
description: >
  Preserve the named goal-prompt-fable invocation for users who want a concise,
  outcome-focused VERA20k goal prompt. Uses the shared goal-prompt contract without
  model-specific assumptions or changing session settings. Never launches the goal.
---

# Concise Goal Prompt

Apply [goal-prompt](../goal-prompt/SKILL.md), emphasizing the outcome, its reason,
comparison bar and completion condition. Keep the prompt as short as its substance
allows; honor the user's word limit rather than a fixed profile quota.

This name remains available for existing invocations. It does not select Fable,
override the current model or imply a preferred reasoning effort. Preserve explicit
model settings from the request; otherwise leave settings unchanged.

Give the executor room to choose design and method. Keep the evidence, production
validation and independent review needed for the task; brevity does not lower the
bar or replace completion with “no gap is worth another round.” Do not launch,
schedule or publish anything while composing the prompt.

[Profiler example](references/profiler-example.md) illustrates concise acceptance
properties without prescribing files, libraries or a model.
