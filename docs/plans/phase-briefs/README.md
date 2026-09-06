# Phase briefs

One brief per phase of
[`2026-07-30-clean-slate-system-implementation-order.md`](../2026-07-30-clean-slate-system-implementation-order.md).
A goal session reads the brief for its phase **before** the plan, the registry
or the research index. The brief is the handover between sessions working the
same phase; the plan is only the dependency order.

Briefs are written when a phase is first targeted and updated by the session
that changes the phase's state (a merged PR, a corrected claim, a new residual).
Closed phases keep their brief and link the closure record under
`docs/gap-scans/`. Phases closed before briefs existed (Phase 7) have only
their closure record; phases never targeted have no brief yet.

## Template

Copy the sections in this order. Keep each section current, not cumulative:
replace stale rows, do not append a diary.

```markdown
# Phase N brief — <phase title>

Rows <first>–<last> of the plan. State: OPEN | IN PROGRESS | CLOSED <date>.

## Rows

| Row | GSI | Rust owner(s) | Native anchor(s) | Registry | Notes |

One line per row. Owners are current file paths (check they exist). Anchors are
symbol + address with the research doc that established them. Registry is
`native_evidence / rust_implementation / parity` from `registry.v2.json`.

## Loops

Which `topology.v2.json` loops the rows sit in, and which stage each row owns.
The loop is what a session traces; rows alone do not order the work.

## Prior work

Merged PRs that moved these rows, newest first, one line each. Rows closed by
evidence rather than code, with the evidence.

## Corrections

Claims in research, System Map or code comments that a session must not
trust, with the evidence that corrected them. Sessions add to this list; they
never silently fix the source doc without also recording the correction here.

## Inherited residuals

Residuals recorded by other phases that land on these rows (trigger, effect,
frequency, where labelled).

## Coverage

What the global parity harness fixture and the retail oracles exercise for
these rows, so a moved hash pin is read correctly.

## Open queue

Mechanisms found DRIFT/MISSING and not yet merged, with owner branch and review
state. Empty when the phase is closed or not yet scanned.

## Start here

The first concrete action for a fresh session.
```

## Goal modes

Two modes run against a brief. The goal prompt names the mode; the brief does
not prescribe one.

**Exhaustive phase close.** Every row in the phase is an ownership hypothesis.
Scan each row's mechanisms from the binary, build every DRIFT/MISSING mechanism
with a builder and an independent read-only critic, merge one coherent
mechanism (or its prerequisite foundation) per PR,
run a phase-wide reverse audit, and close only when no omission or regression
remains and `cargo test -p vera20k --lib` passes. Parity stays
`UNCHECKED`/`DRIFT` in the registry until a gamemd-derived executable
comparison demonstrates it; closure is by evidence, not by claim.

**Bounded slice.** Select one ordinary-stock end-to-end loop from the brief's
loop list, trace it in runtime stage order, find the first player-visible or
determinism-relevant divergence, and close the smallest coherent prerequisite
capability (not merely the smallest patch). Deliver a separable foundation
first with its own evidence, validation and review. Rerun the parent loop;
close it only if its end-to-end check passes, otherwise record residuals in
the brief and stop after the handoff.

Both modes update the brief and the System Map registry for touched, verified
rows (`python -m tools.system_map check`).
