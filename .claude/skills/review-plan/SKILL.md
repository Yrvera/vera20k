---
name: review-plan
description: >
  Review a VERA20k implementation plan against current code, native evidence and
  the requested outcome. Finds stale assumptions, missing consumers, incorrect
  behavior and inadequate acceptance checks. Reports findings without implementing.
---

# Review Plan

Review the explicitly named plan; resolve a short filename under `docs/plans/`.
When no path is given, use the plan established in the conversation, or identify
candidates if the target is ambiguous. Do not silently replace a supplied filename
with the newest plan.

Follow [ENGINE.md](../../../ENGINE.md) and
[independent review](../_shared/review.md). Read the plan and the source behind its
consequential claims. You may inspect native evidence and question omitted mechanisms
independently of the plan author's packet.

Focus on what would make execution wrong or incomplete:

- Current definitions, signatures, owners and behavior differ from the assumptions.
- Callers or downstream readers are missing from an interface/state migration.
- Native predicates, numeric semantics, ordering or active-YR applicability are wrong
  or insufficiently established for the proposed change.
- Tasks create duplicate authority, disconnected behavior or an unsafe dependency order.
- Acceptance checks cannot detect the claimed failure or are presented as stronger
  parity evidence than their source and coverage support.
- Scope or completion criteria quietly omit part of the user's requested outcome.

Verify that a suspected defect compares the same operation and reachable scenario
on both sides. An initialization rule and a per-tick rule may intentionally differ.
A missing explicit INI key may use a native default: inspect that default before
calling the key's absence a plan error. Shifted line numbers matter when the intended
symbol cannot be located or the assumption changed, not by an arbitrary tolerance.

Lead with readiness and actionable findings, citing the claim, contradictory evidence,
trigger/consequence and suggested correction. Separate uncertain questions from
confirmed defects; summarize coverage and remaining gaps instead of listing every
passing citation. Do not require fixed headings, confidence tables or complete code.

Review-only requests leave the plan unchanged. Correct it when the user asks for
corrections; neither mode authorizes implementing the planned feature.
