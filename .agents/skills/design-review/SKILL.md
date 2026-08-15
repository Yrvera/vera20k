---
name: design-review
description: >
  Review a brainstorm/design doc before implementation. Audits whether behavior
  claims are grounded in cited research docs, INI/assets, source, or explicit
  UNKNOWN/UNCHECKED status; checks that the chosen approach covers its
  player-experience ledger without architecture drift. Usage: '/design-review design-doc-path'
  or 'review this brainstorm design doc'.
---

# Design Review - Pre-Implementation Readiness Audit

Review the design doc: **$ARGUMENTS**

If `$ARGUMENTS` is empty, use the most recent `docs/plans/*-design.md`. If none
exists, ask for a design doc path.

## Hard Gate

- Do not implement code.
- Do not edit Rust.
- Do not revise the design doc unless explicitly asked.
- Findings first, ordered by severity.
- Review against the retail-convincing ordinary-skirmish bar in `AGENTS.md`.
  Preserve exact truth labels, but do not block implementation solely for a
  bounded, documented expert-only residual.

## Verdicts

- `APPROVE`: implementation-ready; remaining unknowns are named, bounded, and not
  blocking the chosen change.
- `REVISE`: design needs citation, wording, approach, or test-plan fixes before
  implementation.
- `BLOCK`: design contradicts load-bearing evidence, guesses about consequential
  behavior, leaves the representative loop broken, or creates
  determinism/authority/lifecycle/architecture debt.

## Review Workflow

### 1. Load Grounding

Read:

1. `AGENTS.md`.
2. The design doc.
3. Every cited `docs/research/...` file section needed to verify its claims.
4. Relevant Rust files named by the design.

For parity-sensitive topics, request a compact `research_brief` from the
research-index MCP first. If that capability is unavailable, run the CLI fallback
from the repo root:

```text
python tools/research_index/brief.py --system <system> "<topic>" --limit 8
```

Use synthesis docs only as maps. Cite underlying Ghidra reports, traces, INI keys,
or source files for verdicts.

### 2. Extract Claims

Extract only claims that matter to implementation readiness:

- Active-YR gates and INI/default applicability.
- Branch order, predicates, field reads/writes, clear/set paths, side effects, and
  skipped side effects.
- Rust architecture claims: modules, fields, callers, state flow, boundaries.
- Chosen approach claims.
- Test-plan claims.
- Unknowns/deferred items.

### 3. Verify Evidence

For each parity claim:

- `CONFIRMED`: cited source says this.
- `MISMATCHED`: cited source says something else.
- `UNCITED`: no usable citation.
- `STALE`: source is superseded or contradicted by newer evidence.
- `UNCHECKED`: design correctly marks this as not yet known.

Rules:

- An uncited parity claim is a finding.
- A synthesis citation alone is not enough for a mechanism claim.
- Runtime-only facts must not be inferred from static decompilation.
- "Rare", "visually masked", "internal-only", and "probably equivalent" never
  prove exact equivalence. They may support delivery deferral only when trigger,
  ordinary-play frequency, player effect, compounding, and downstream risk are
  documented.

### 4. Ledger Coverage

Check the player-experience ledger:

- Every milestone-blocking and compounding item has an owner and production test.
- Exactification residuals name trigger, frequency, player effect, and downstream
  risk.
- Unknown-risk items are investigated enough to determine whether they block.

### 5. Architecture And Tests

Verify:

- `sim/` does not gain render/ui/audio/net dependency.
- The design fits existing module boundaries.
- It does not broaden scope into unrelated systems.
- Tests cover the representative production path, negative side effects,
  neighboring common paths, and deterministic state that could affect outcomes.

## Output Format

```text
Verdict: APPROVE | REVISE | BLOCK

Findings
- [P1] ...
- [P2] ...

Open Questions
- ...

Implementation Readiness
- ...
```

If there are no findings, say so explicitly and still list residual risks or test
gaps.
