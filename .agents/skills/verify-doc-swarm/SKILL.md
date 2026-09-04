---
name: verify-doc-swarm
description: "Coordinate research-document audits and reconcile contradictions. Default is read-only detection; --fix or an explicit correction request authorizes edits to assigned documents. Workers always keep Ghidra read-only."
---

# Coordinate research audits

Use [shared coordination](../_shared/swarm.md) and
[verify-doc](../verify-doc/SKILL.md). For authorized corrections, workers use
[audit](../audit/SKILL.md). Keep the distinction between document correctness,
coverage, and gameplay priority throughout reconciliation.

Resolve an explicit list, `--area <area>`, or `--all` into concrete research
documents. With no target, choose a bounded wave from current implementation
dependencies, unresolved findings, and stale evidence. Use the research index for
discovery; use filesystem enumeration for a corpus-wide request because search
rankings do not enumerate the corpus. Deduplicate targets and explain selection.
Proceed within the authorized scope without a ceremonial confirmation.

Give each worker one document and an explicit mode:

- **Detect:** inspect and report; no document or shared-log writes.
- **Fix:** authorized by `--fix` or the user's request to correct documents. Each
  worker exclusively owns its assigned document in the task-owned checkout and
  corrects evidence-supported claims with inline citations. No sibling edits.

Use the verification method's coverage and GREEN/YELLOW/RED meanings. A clean
sample cannot certify a whole document; an exhaustive request requires the complete
declared claim surface. If foundational uncertainty makes an isolated correction
misleading, leave that correction unapplied and identify the necessary research.

Read the returned findings and, in fix mode, the diffs. Independently verify
consequential corrections and the evidence supporting broad clean verdicts. Resolve
conflicts from the binary rather than voting across reports. Check dependent wording
and surface shared error patterns without inventing their historical cause. Route
any repair back to its owner, or take ownership after that worker stops.

Reconcile sibling contradictions, current Rust implications, outstanding evidence,
and safe correction facts. Keep unassigned siblings unchanged. Return a verdict
and coverage per document, decisive findings or corrections, parent checks,
remaining blockers, and exact edited paths.

Additional modifiers:

- `--dry-run`: show scope, mode, and proposed assignments only.
- `--refresh-index`: refresh candidate discovery from current sources.
- `--patch-plan`: add a non-editing correction order and required evidence.
- `--sync-ghidra-labels`: authorize root-only, serial metadata synchronization
  after all readers stop, under the
  [binary workflow](../../../docs/research/ghidra-workflow.md).
  A read-only request or `--no-sync-ghidra-labels` disables it.

Do not create hand-maintained coverage indexes or shared worker logs. An explicitly
requested saved report or log is written once by the parent. This workflow does not
implement gameplay or publish changes.
