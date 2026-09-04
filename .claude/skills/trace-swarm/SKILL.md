---
name: trace-swarm
description: "Coordinate independent traces of concrete gameplay interactions, then reconcile production failures and unchecked boundaries. Workers keep Ghidra read-only. Does not implement gameplay fixes; metadata sync requires explicit authorization."
---

# Coordinate production traces

Use [shared coordination](../_shared/swarm.md) and
[trace-action](../trace-action/SKILL.md).

Choose independent concrete scenarios from the request, current code, prior
findings, and research. With no targets, select a bounded wave favoring frequent
ordinary-skirmish interactions. Explain the selection and proceed. A single
scenario can use one investigator; do not pad the work to fill slots.

Check that proposed traces have meaningful production paths to compare. If a broad
surface shares one missing parent mechanism, investigate that prerequisite once
with a focused trace or [disparity-scan](../disparity-scan/SKILL.md) instead of
dispatching duplicate findings. Honor explicitly requested trace coverage and
explain any change in method.

Give each worker its scenario, inputs, scope, relevant evidence, and a unique report
path under task-owned `docs/research/traces/`. Workers write only their reports
and keep source, research inputs, and Ghidra unchanged. Require the actual pipeline,
decisive comparisons, PASS/FAIL/NOT IMPLEMENTED/UNCHECKED results, and omissions.

Read the reports and independently inspect consequential failures, surprising
passes, and shared assumptions. Verify that a PASS actually compares both sides
and applies only to the stated boundary. Reconcile contradictions and consolidate
shared causes before ranking findings. A clean set of sampled traces cannot certify
the containing systems.

Return report links, scope and result per scenario, earliest shared causes,
ranked disparities with trigger frequency, unresolved checks, and annotation
candidates. Preserve exact differences separately from implementation priority.
Continue only as needed to finish the user's requested trace set.

Modifiers:

- `--area <area>` bounds target selection.
- `--dry-run` reports the planned assignments without dispatch or mutation.
- `--refresh-index` refreshes discovery from current sources.
- `--sync-ghidra-labels` authorizes the parent to synchronize certain metadata
  serially after every reader stops, following the
  [binary workflow](../../../docs/research/ghidra-workflow.md).
- `--no-sync-ghidra-labels` or a read-only request leaves candidates unapplied.

Do not create shared claims logs or hand-maintained trace indexes, and do not
implement gameplay fixes under this workflow.
