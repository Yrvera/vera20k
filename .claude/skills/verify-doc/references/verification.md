# Research-document verification

Follow [ENGINE.md](../../../../ENGINE.md) and read the
[binary workflow](../../../../docs/research/ghidra-workflow.md) before binary work.
Establish the active program before evaluating claims. An unavailable bridge is a
tool blocker, not evidence that a document is wrong; use `ghidra-up` when appropriate.

Identify the distinct factual claims within the requested scope: function and
data identity, field layout, formulas, conditions, timing, ordering, defaults,
INI mappings, YR activation, and current Rust implications. Include precise
details in tables and pseudocode as well as prose. Check premise claims before
conclusions that depend on them. Use enough working notes to account for coverage,
without creating a permanent claim ledger.

Verify binary claims from live bodies, callsites, bytes, and data reads as the
claim requires. Match each citation to what it proves. In particular, check
pointer-scaled offsets, signedness, inclusive bounds, initialization, concrete
virtual dispatch, and active-YR gates under retail defaults. Use the binary
workflow's full vtable ownership and slot proof. Read cited Rust directly; research
prose cannot establish its current implementation. Read retail INIs from the main
checkout and apply YR loading rules.

Classify findings:

- **CONFIRMED:** the inspected evidence supports the exact scoped claim.
- **WRONG:** evidence contradicts it, including a one-bit, one-tick, or operator error.
- **MISLEADING:** literal text omits conditions that change its meaning or applicability.
- **STALE:** a reference or annotation is outdated while the behavior still holds.
- **UNCHECKED / UNVERIFIABLE:** distinguish work not done from a claim that cannot be
  established with the available evidence.

Renaming or reanalyzing unchanged binary bytes cannot move actual vtable slots,
field offsets, or instructions. Do not invent historical causes for a mismatch.
State a root cause only when evidence supports it.

Use **GREEN** only when all distinct claims in the declared audited scope are
supported with no unresolved material checks. **YELLOW** means listed caveats or
incomplete coverage; **RED** means foundational errors make the document unsafe
as an implementation basis. A clean sample may be described as such, never as a
global GREEN. These statuses assess documentation, not executable Rust parity.

For each defect, quote or locate the original claim, show the exact supported
replacement fact and reproducible evidence, and explain its consequence. Check
dependent sections and relevant sibling contradictions without silently expanding
the audit. Separate uncertainty from demonstrated errors and completeness from
priority. Record actual coverage rather than estimating percentages.
