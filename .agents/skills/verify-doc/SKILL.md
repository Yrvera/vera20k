---
name: verify-doc
description: "Audit an existing gamemd.exe research document against the live binary and current cited sources. Reports incorrect, misleading, stale, or unchecked claims without editing the document."
---

# Verify a research document

Resolve and read the target document. Use the research index or focused search
when given a topic rather than a path; ask only if multiple plausible documents
would materially change the requested audit.

Apply the [research verification method](references/verification.md). The default
is a full audit of the named document's distinct factual claims. Repeated claims
can share evidence; narrative, pseudocode, tables, and handoffs must agree. A
requested spot-check may sample, but name omissions and limit the verdict to that
sample. An explicit exhaustive request cannot be completed by sampling.

Inspect recent changes and earlier findings to focus verification. Neither a
recent edit nor a past GREEN verdict requires a new approval pause. Anchor the
report to the actual document revision and distinguish changes occurring during
the audit.

Keep the source document and Ghidra read-only. Return findings in chat unless the
user requested a saved report or audit-log entry. That request can be given before
the audit; no second approval is needed. Do not write a corrected copy.

Lead with the verdict for the audited scope, then wrong or misleading claims,
their exact locations, decisive evidence, and downstream implications. State
coverage and unresolved checks. If the user has also authorized corrections,
continue with [audit](../audit/SKILL.md) using those permissions; do not require
another invocation or a separate task.
