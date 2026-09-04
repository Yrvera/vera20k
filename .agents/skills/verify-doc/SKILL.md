---
name: verify-doc
description: "Audit gamemd.exe research documents against live evidence; correct them when the user authorizes edits."
---

# Verify research

Read the requested documents and apply [ENGINE.md](../../../ENGINE.md) and the
[binary reference](../../../docs/research/ghidra-workflow.md). Default to read-only
inspection; a request to fix or correct authorizes edits to the named, task-owned
documents without another invocation.

Check distinct factual claims across prose, tables, pseudocode and handoffs.
Verify binary claims live and cited Rust directly. Establish premises before
dependent conclusions. Default to the named scope's complete claim surface;
requested spot-checks may sample but cannot certify the whole document.
A bridge outage is a tool blocker, not evidence against the document.

Report contradicted or misleading claims with exact locations, supported replacement
facts, reproducible evidence and consequences. Distinguish outdated references from
semantic errors, and unresolved checks from demonstrated defects. State actual
coverage against the inspected revision; do not invent historical causes.

When correcting, preserve unrelated valid material, cite changed binary claims inline
and update dependent wording. Mark unsupported load-bearing claims `UNKNOWN`/`UNCHECKED`
with missing evidence instead of deleting or inventing them. Foundational uncertainty
limits which corrections are safe; isolated repairs cannot certify the whole document.
Keep unassigned sibling documents unchanged.

Choose delegation and report format to fit the request; save additional reports only
when requested. Document corrections do not authorize gameplay changes.
Ghidra authority follows the binary reference.
