---
name: review-plan
description: >
  Review an implementation plan for correctness before execution.
  Verifies codebase assumptions (struct fields, signatures, line numbers),
  binary claims (gamemd.exe addresses, behavior), and internal consistency.
  Catches false positives before reporting. Usage: '/review-plan plan-file'
  or '/review-plan' (auto-detects latest plan doc).
---

# Review Plan — Pre-Execution Correctness Audit

Review the implementation plan: **$ARGUMENTS**

If `$ARGUMENTS` is empty or just a filename, look for the most recent
`docs/plans/*-plan.md` file. If none exists, tell the user there's nothing
to review.

**Purpose:** Catch plan errors BEFORE execution — wrong assumptions, stale
line numbers, incorrect binary claims, missing edge cases. Every finding
must survive a self-verification step before being reported.

**Delivery framing.** Review against the retail-convincing ordinary-skirmish
bar in `AGENTS.md`. Verify consequential binary claims and preserve honest
DRIFT/UNCHECKED labels, but do not reject a plan solely because Rust internals
or an expert-only edge differ. Block when the plan guesses about common
behavior, leaves the representative production loop wrong, or risks
determinism, authority, lifecycle, RNG, persistence, commands, or shared
architecture. Record bounded exactification residuals separately.

Do NOT implement any code. Do NOT modify the plan. Report findings only.

---

## Phase 0 — Extract Plan Grounding

Before extracting generic claims, pull out the plan's grounding artifacts
(produced by `/write-plan`). These drive verification priority.

### 0.1 Grounding Summary

Locate the **Grounding Summary** section. Confirm it's present and
non-empty. A missing or empty summary is itself a finding — the plan
was written without the grounding phase.

### 0.2 Confidence-Tagged Decisions

Locate the **Key Technical Decisions** section. For each decision,
extract:
- The decision itself
- **Confidence:** high | medium | low
- **Source:** (docs/research/... | Ghidra FUN_... | repo pattern ... | inferred)

Build a verification priority list:
1. **low-confidence decisions** — verify first, most likely to be wrong
2. **medium-confidence decisions** — verify second
3. **high-confidence decisions** — verify last, spot-check only

A decision tagged high-confidence but citing `inferred` is suspicious —
promote it to medium for verification purposes.

### 0.3 Sources & References Inventory

Locate the **Sources & References** section. Extract:
- Every `docs/research/*.md` filename cited
- Every gamemd.exe address listed (FUN_..., 0x...)
- Every INI key referenced (`[Section]` Key=)
- Every Rust file path listed

These entries get verified in Phase 2.5 — missing sources invalidate
decisions that depend on them.

### 0.4 Deferred Questions

Locate **Open Questions → Deferred to Implementation**. For each entry,
ask: is this genuinely un-resolvable at planning time, or was it punted?
Flag any question that could plausibly be answered by reading the code
or decompiling — those are planning failures disguised as deferrals.

---

## Phase 1 — Extract Claims

Read the full plan document and extract verifiable factual claims that drive
the implementation, representative production test, or risk classification
into three categories. Do not spend the review proving decorative background
prose.

### A. Codebase claims
- Struct/enum definitions and their fields
- Function signatures (name, parameters, return type)
- Line numbers and file locations ("lines 543-574", "after the NEIGHBORS constant")
- Constants and their values
- Existing behavior ("the old function does X")
- What callers exist and whether they need changes

### B. Binary claims
- Addresses ("AStar_main_loop at 0x00429a90")
- Behavioral descriptions ("matches binary inline check at 0x00429e38")
- Thresholds, formulas, decision trees attributed to gamemd.exe
- Struct field offsets in the binary

### C. Logical claims
- "No callers need changes" — is this actually true?
- "Tests check properties not exact sequences" — do they?
- "Same practical effect" — is it really?
- Risk assessments ("no test breakage expected")

**Output:** A numbered list of claims to verify. Group by category. Don't
verify yet — just extract.

---

## Phase 2 — Verify Codebase Claims

For each codebase claim from Phase 1A, read the actual file and check:

1. **Struct/enum fields** — do they match? Any fields missing from or added
   to the plan's description?
2. **Function signatures** — exact parameter types and return type?
3. **Line numbers** — are they still accurate? (Files shift as edits accumulate.)
   Tolerance: +/- 10 lines is fine. Off by 50+ is a problem.
4. **Constants** — correct names, types, and values?
5. **Existing behavior** — read the actual function. Does it do what the plan
   says it does?
6. **Caller analysis** — grep for all callers. Are there callers the plan missed?

**For each claim, record:** CONFIRMED / STALE / WRONG / SHIFTED (with actual value).

Do NOT skip this phase. Plans written against a moving codebase accumulate
drift. Line numbers are the most common casualty.

**Ordering:** Verify codebase claims that back low-confidence decisions
(from Phase 0.2) first. A low-confidence decision citing `src/sim/foo.rs`
gets checked before a high-confidence one.

---

## Phase 2.5 — Verify Source Citations

Check every item extracted in Phase 0.3:

1. **docs/research/*.md** — does the file exist? Read it and confirm
   it actually covers what the plan claims it covers. A citation to a
   report that doesn't say what the plan says is a fabricated source.
2. **gamemd.exe addresses** — decompile addresses that support a
   milestone-blocking, compounding, authority, lifecycle, RNG, persistence, or
   architecture decision. For residual/background citations, verify existence
   and flag uncertainty without expanding the review.
3. **INI keys** — grep `ini/rulesmd.ini` and `ini/artmd.ini` for each
   cited `[Section]` Key. Missing keys are plan errors.
4. **Rust file paths** — do the files exist? If line ranges are cited,
   are they within the file's actual length?

**For each item, record:** EXISTS / MISSING / MISMATCHED.

**Any MISSING source invalidates every decision that cited it.** Surface
those decisions explicitly in the report.

---

## Phase 3 — Verify Binary Claims

For each implementation-changing binary claim from Phase 1B, decompile via
Ghidra MCP and check. Exactification-residual claims may remain explicitly
unverified when they do not affect the selected plan.
**Prioritize claims that back low-confidence or medium-confidence
decisions from Phase 0.2.**

1. **Does the function exist at the claimed address?** Decompile it.
2. **Does the behavior match the plan's description?** Read the actual
   decompiled code — don't trust summaries.
3. **Are thresholds/operators correct?** Check exact comparisons:
   - `>` vs `>=` (the `CMP + JG` vs `CMP + JGE` distinction)
   - `< 2` vs `<= 2` vs `>= 2` (off-by-one in threshold claims)
   - signed vs unsigned arithmetic
4. **Are there prerequisite checks the plan omits?** The binary often has
   flag checks before the "interesting" logic. Missing a prerequisite
   means the plan's function fires in cases the binary wouldn't (or
   vice versa).
5. **Is the code active in YR?** Trace callers to confirm this isn't
   dormant TS legacy code.

**Ghidra pitfalls to watch for:**
- `param_1` as `int` → direct byte offsets. `param_1` as `int *` → multiply
  index by 4 for byte offset.
- `DAT_` globals may need `read_memory` to verify actual values.
- Decompiler may fold or reorder conditions — compare semantics, not syntax.

**For each claim, record:** CONFIRMED / WRONG (with what the binary actually
does) / PARTIALLY CORRECT (with the nuance).

---

## Phase 4 — Verify Logical Claims

For each logical claim from Phase 1C:

1. **"No callers need changes"** — grep for every call site of every function
   the plan modifies. Verify the claim exhaustively.
2. **"Tests won't break"** — read the actual test assertions. Are they really
   checking properties (start, end, length) or exact sequences? A test that
   asserts `path[2] == (5, 3)` WILL break if tiebreakers change.
3. **"Same practical effect"** — construct concrete scenarios with real values
   and trace both the old and new logic. Do they preserve the same mechanism,
   consumed bytes, downstream values, pixels, audio, UI, and timing?
4. **Risk assessments** — for each "no breakage expected" claim, ask: what's
   the simplest scenario that WOULD break? If you can construct one easily,
   the risk is higher than claimed.

---

## Phase 5 — Self-Verification (MANDATORY)

**This is the most important phase.** Before reporting ANY finding as a
discrepancy, re-verify it from the primary source.

For each potential discrepancy found in Phases 2-4:

1. **Re-read the source.** Not your notes — the actual file or decompilation.
2. **Ask: "Am I comparing the right things?"** A common false positive is
   comparing two operations that serve different purposes (e.g., a start-height
   init threshold vs a per-cell routing threshold).
3. **Ask: "Does this actually fire in practice?"** A discrepancy that only
   triggers with pathological inputs (values that never occur in real RA2 data)
   is not worth reporting as a real issue.
4. **Ask: "Is this an existing discrepancy or one introduced by the plan?"**
   If the plan preserves an existing architectural difference (e.g., caller
   provides start_layer vs binary computes it internally), require proof that
   the difference is byte/pixel equivalent before treating it as acceptable.
5. **Construct a concrete scenario** where the discrepancy produces a different
   result. If you can't construct one with real RA2 values, downgrade it.

**Classification after self-verification:**
- **CONFIRMED ISSUE** — re-verified from source, concrete scenario constructed
- **FALSE POSITIVE** — initially looked like a discrepancy but isn't on
  re-examination. State why.
- **THEORETICAL** — real logical difference but cannot construct a scenario
  with standard RA2 data. Note what data would trigger it.

---

## Phase 6 — Report

### Structure

```
## Plan Review: [plan filename]

### Grounding Audit
- Grounding Summary present? yes/no
- Deferred questions that could have been resolved: [list, if any]

### Confidence Audit
[Table of each Key Technical Decision:]
| Decision | Plan confidence | Verified as | Source check |
|----------|-----------------|-------------|--------------|
| ... | low | CONFIRMED / WRONG / INFERRED | docs/research/XYZ.md EXISTS |

### Source Citations
[Table of items from Phase 2.5:]
| Citation | Status | Notes |
|----------|--------|-------|
| docs/research/XYZ.md | EXISTS | covers claim |
| FUN_00abcdef | MISMATCHED | address is a different function |
| [General] Key= | MISSING | not present in rulesmd.ini |

### Codebase Accuracy
[Table of shifted line numbers, stale references, wrong field names — if any]

### Binary Fidelity
[Table of confirmed vs wrong behavioral claims — if any]

### Confirmed Issues
[For each CONFIRMED ISSUE from Phase 5:]
- What the plan claims
- What the source actually shows (with address / file:line)
- Concrete scenario demonstrating the difference
- Suggested fix (one sentence)

### False Positives Caught
[For each FALSE POSITIVE from Phase 5:]
- What initially looked wrong
- Why it's actually fine (one sentence)

### Theoretical Concerns
[For each THEORETICAL from Phase 5:]
- The logical difference
- Why it doesn't fire with real data
- What would trigger it (for awareness)

### Verdict
[One of:]
- READY — no confirmed issues, plan can be executed as-is
- FIX FIRST — N confirmed issues that should be addressed before execution
- NEEDS REWORK — fundamental assumptions are wrong
- GROUNDING GAP — plan is missing Grounding Summary, has MISSING source
  citations, or has low-confidence decisions with no verifying evidence.
  Send back to `/write-plan` to ground properly before execution.
```

### Rules

- **Lead with confirmed issues.** Don't bury them under tables of passing checks.
- **Include false positives caught.** This builds confidence that the review
  was thorough — "we looked at X and it's fine" is valuable signal.
- **No match tables.** Don't list every claim that checked out. Only report
  deviations and interesting non-deviations (things that looked wrong but
  aren't).
- **Concrete scenarios for every confirmed issue.** "This might differ" is
  not a finding. "Tank at height 2 on bridge cell with ground_level=0 would
  route to ground list instead of bridge list" is a finding.
- **One-sentence suggested fixes.** Don't write implementation code — just
  point the direction.

---

## Anti-Patterns in Plan Reviews

Avoid these common review failures:

- **Threshold telephone** — claiming a threshold is wrong because you compared
  two different operations that use different thresholds for different reasons
- **Architecture-vs-fidelity confusion** — flagging that "the plan does X
  differently from the binary" when X is an intentional architectural choice
  (e.g., caller-provided layer vs internally-computed layer)
- **Ghidra misread** — forgetting to check param_1 type, misinterpreting
  `JG` vs `JGE`, not accounting for sign extension
- **Stale-doc amplification** — treating a research doc claim as ground truth
  without verifying in the binary, then flagging a "discrepancy" that's
  actually a doc error
- **Missing the forest** — spending all review effort on helper functions
  while ignoring the main loop's correctness
- **Over-reporting** — listing 10 "potential issues" that are really 1
  confirmed + 9 theoretical. This wastes the user's time and erodes trust.
