---
name: design-review
description: >
  Assess a VERA20k design before implementation for evidence, architectural fit,
  omitted behavior and production validation. Reports actionable findings and
  readiness; does not revise the design or implement code unless separately asked.
---

# Design Review

Review the named design, or the design clearly identified by the conversation.
Follow [ENGINE.md](../../../ENGINE.md) and
[independent review](../_shared/review.md). Inspect original sources as needed;
the author's citations and chosen scope do not bound the reviewer's inquiry.

Judge whether the approach can deliver the requested outcome:

- Does current source support the described owners, interfaces and consumers?
- Do native bodies/callers and retail data support consequential parity claims,
  applicability and exclusions? Are unresolved claims clearly identified?
- Does the design preserve state authority and required ordering/lifecycle behavior
  through the whole production path, including dependencies and retiring old logic?
- Are abstractions and migrations justified by real responsibilities and consumers?
- Can the proposed validation distinguish a working mechanism from disconnected
  helpers, inherited wrong goldens or a merely plausible screenshot?

For visual designs, check the parent composition and target flags/frame selection,
not just a cited draw helper. For simulation designs, check the surrounding loop and
consumers that observe changed state. Treat user observations that contradict the
premise as evidence needing resolution.

Report a readiness judgment with actionable findings, their source and consequences,
then important gaps in the review. Explain whether an issue blocks the requested
scope or belongs to a separately named follow-up. Do not soften an exhaustive goal
into a partial milestone or treat a design preference as a proven defect.

A design need not use a particular heading, table or risk vocabulary. Review its
substance. Leave files unchanged for review-only requests; if corrections are also
authorized, make the justified design edits without implementing the feature.
