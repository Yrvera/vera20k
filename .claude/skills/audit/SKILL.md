---
name: audit
description: "Audit and correct an authorized research document using active gamemd.exe evidence. Inspection-only requests use verify-doc."
---

# Correct research

Apply the [verification method](../verify-doc/references/verification.md) to the
whole target before rewriting dependent claims. Edit only the authorized, task-owned
document; cite each corrected binary claim inline.

Preserve unrelated correct material. Mark unsupported load-bearing claims
`UNKNOWN`/`UNCHECKED` with missing evidence instead of deleting or inventing them.
If foundational uncertainty requires new research, apply only independently valid
corrections and retain the limitation.

Check the diff and dependent summaries/tables/handoffs. Report corrections,
remaining coverage, and the edited path.

Ghidra synchronization is opt-in under the
[binary workflow](../../../docs/research/ghidra-workflow.md), including
`--sync-ghidra-labels`. Workers report candidates only; read-only requests or
`--no-sync-ghidra-labels` disable synchronization.
