---
name: verify-doc-swarm
description: "Coordinate research-document audits. Default is detection; --fix or explicit correction authority permits assigned-document edits. Workers keep Ghidra read-only."
---

# Coordinate document audits

Use [shared coordination](../_shared/swarm.md),
[verify-doc](../verify-doc/SKILL.md), and [audit](../audit/SKILL.md) for corrections.

Resolve a document list, `--area <area>`, or `--all`. With no target, choose a
bounded wave from implementation dependencies and stale evidence. Enumerate the
filesystem for corpus-wide coverage; ranked search is insufficient.

Assign one document per worker:

- **Detect:** inspect and return findings; no writes.
- **Fix:** `--fix` or explicit correction authority permits only the assigned
  task-owned document. Correct established facts with inline citations.

Independently check consequential corrections and broad clean verdicts. Reconcile
sibling contradictions, dependent wording and missing evidence. Return repairs to
the owner or take ownership after it stops; leave unassigned siblings unchanged.
Report each document's verdict, coverage, corrections, parent checks and blockers.
The parent writes requested summary reports/logs.

Modifiers:

- `--dry-run`: scope/mode/assignments only.
- `--refresh-index`: refresh discovery.
- `--patch-plan`: non-editing correction order/evidence needs.
- `--sync-ghidra-labels`: parent-only synchronization after readers stop, under the
  [binary workflow](../../../docs/research/ghidra-workflow.md).
  Read-only requests or `--no-sync-ghidra-labels` disable it.

No gameplay implementation or publication.
