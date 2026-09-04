# Independent review

Use this when a task calls for a critic or a substantial change needs independent review.
The reviewer is read-only and should not have built the change. Give it the requirement,
relevant evidence, current diff and actual validation output, plus repository access.
It may inspect original sources and Ghidra, question the scope and identify omitted
mechanisms; the builder's summary is not the boundary of inquiry.

Review what could fail: wrong evidence, missing production callers, duplicate authority,
order/RNG/lifecycle/persistence errors, misleading tests and architectural complexity.
Follow the claim to its consumers before reporting a defect. Distinguish confirmed
defects from questions and design preferences. Every finding needs a trigger,
consequence and source; uncertainty is not a finding of fact.

The owner resolves confirmed issues in the selected mechanism and its necessary
foundations. Adjacent findings become explicit follow-up work, not automatic scope
expansion. Evidence may change the plan or reveal that the mechanism is not closed.
A critic cannot grant permission to publish or edit another task's work.

Report findings first, then reviewed scope and important gaps. A scoped pass does
not certify the whole system. Recheck fixes and affected prior conclusions. Use a fresh
critic for significant revisions or when the goal requires one; no ritual review loop
is needed for a trivial wording edit.