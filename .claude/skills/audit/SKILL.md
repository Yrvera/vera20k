---
name: audit
description: "Audit and correct a named VERA20k research document using active gamemd.exe evidence. Use when corrections are authorized; use verify-doc for read-only review."
---

# Audit and correct research

Read the whole target and apply the
[research verification method](../verify-doc/references/verification.md). Work in
the task-owned checkout; confirm file ownership before editing. Correcting a
document is authorized by the user's request to fix it, including earlier context.
Inspection alone remains read-only.

Establish the claim surface before rewriting so a corrected premise does not
leave contradictory summaries, tables, or implementation handoffs. Correct only
facts supported by evidence gathered for this audit, with a compact reproducible
inline citation beside each changed binary claim. Preserve unrelated correct
material and the document's useful structure.

Mark unsupported load-bearing claims `UNKNOWN` or `UNCHECKED` with the missing
evidence; do not silently delete them or invent replacements. If the document's
foundation requires a new investigation, state that limitation and perform only
corrections that remain valid independently. A few repairs cannot make the whole
document GREEN.

This skill changes the authorized research document, not Rust, INIs, assets, or
sibling documents. Report sibling contradictions separately. Keep Ghidra read-only
unless metadata synchronization is explicitly authorized through the user request
or `--sync-ghidra-labels`; the root/sole agent follows the
[binary workflow](../../../docs/research/ghidra-workflow.md) after all readers stop.
Workers report candidates only. Read-only requests and `--no-sync-ghidra-labels`
disable synchronization.

Review the final diff against the evidence and check dependent wording. Report the
edited document, material corrections, remaining coverage and uncertainty, and any
Ghidra mutations actually applied.
