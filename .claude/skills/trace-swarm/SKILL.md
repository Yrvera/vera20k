---
name: trace-swarm
description: "Coordinate concrete gameplay traces and reconcile failures. No gameplay fixes; Ghidra synchronization requires explicit authorization."
---

# Coordinate traces

Use [shared coordination](../_shared/swarm.md) and
[trace-action](../trace-action/SKILL.md). Select independent scenarios; absent targets,
favor frequent skirmish interactions.

If proposed traces share one missing parent mechanism, investigate it once with a
focused trace or [disparity-scan](../disparity-scan/SKILL.md). Explain the method
change while preserving requested coverage.

Assign each worker its inputs, scope, evidence and unique task-owned
`docs/research/traces/` report; workers may write only those reports.
Independently inspect consequential failures and surprising passes. Consolidate
shared causes and return report links, scoped verdicts, ranked disparities,
unresolved checks and annotation candidates.

Modifiers:

- `--area <area>`: bound selection.
- `--dry-run`: assignments only; no dispatch or mutation.
- `--refresh-index`: refresh discovery.
- `--sync-ghidra-labels`: parent-only serial synchronization after readers stop,
  following the [binary workflow](../../../docs/research/ghidra-workflow.md).
- `--no-sync-ghidra-labels` or read-only requests leave candidates unapplied.
