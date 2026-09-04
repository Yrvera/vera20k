---
name: review-plan
description: >
  Review a VERA20k implementation plan against current code, native evidence and
  the requested outcome. Find stale assumptions, missing consumers and inadequate
  acceptance checks without implementing.
---

# Review Plan

Use the supplied plan; resolve short filenames under `docs/plans/`. Otherwise use
conversation context, not automatically the newest file. Follow
[ENGINE.md](../../../ENGINE.md) and [independent review](../_shared/review.md),
with freedom to inspect original evidence and challenge omitted mechanisms.

Check consequential source assumptions, native behavior/applicability, downstream
consumers, dependency order and whether acceptance scenarios expose the claimed
failure through production. Compare the same operation on both sides: initialization
and per-tick rules may differ. A missing INI key may use a native default. Shifted
line numbers matter when the intended symbol or assumption changed.

Report readiness, sourced findings and coverage gaps, separating uncertain questions
from defects. Do not require full code or fixed headings. Review-only requests leave
the plan unchanged; requested corrections permit plan edits, not implementation.
