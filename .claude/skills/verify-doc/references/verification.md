# Research verification

Follow [ENGINE.md](../../../../ENGINE.md) and the
[binary workflow](../../../../docs/research/ghidra-workflow.md). A bridge outage is
a tool blocker, not evidence against the document.

Identify distinct claims across prose, tables, pseudocode and handoffs; check
premises before dependent conclusions. Verify binary claims live and read cited
Rust directly. Apply the binary workflow's typed-offset, vtable, initialization
and active-YR checks. Repeated claims may share evidence.

Classify:

- **CONFIRMED:** exact scoped claim supported.
- **WRONG:** contradicted, including tiny numeric/operator/order errors.
- **MISLEADING:** omitted conditions change meaning or applicability.
- **STALE:** reference/annotation outdated while behavior holds.
- **UNCHECKED / UNVERIFIABLE:** work not done / unavailable evidence cannot establish it.

**GREEN** requires all distinct claims in the declared audited scope supported,
with no unresolved material checks. **YELLOW** means caveats or incomplete coverage;
**RED** means foundational errors make the document unsafe for implementation.
A clean sample cannot yield a whole-document GREEN. Exhaustive requests require
complete declared coverage. These verdicts assess documentation, not Rust parity.

For defects, locate/quote the claim, provide the supported replacement fact and
reproducible evidence, and explain consequences. Check dependent wording and relevant
sibling contradictions. Report actual coverage and uncertainty; do not invent
historical causes or equate priority with correctness.
