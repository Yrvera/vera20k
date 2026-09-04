---
name: review-plan
description: >
  Review a VERA20k design or implementation plan for evidence, architectural fit,
  missing consumers and adequate production validation. Report readiness; revise
  the document only when corrections are requested.
---

# Review Design or Plan

Use the supplied document or conversation context, not automatically the newest file.
Follow [ENGINE.md](../../../ENGINE.md). Inspect original evidence and challenge
omitted mechanisms independently of the author's packet.

Check consequential source assumptions, native behavior/applicability, ownership and
lifecycle handoffs, dependencies and whether acceptance scenarios expose failure
through production. For visuals, check parent composition and active frames/flags,
not merely a draw helper. Compare the same operation on both sides: initialization
and per-tick rules may differ; a missing INI key may use a native default.

Report readiness, sourced findings and coverage gaps, separating uncertain questions
and design preferences from defects. Judge substance rather than headings, line-number
tolerances or full-code templates. Review-only requests leave files unchanged;
authorized document corrections do not authorize implementing the feature.
