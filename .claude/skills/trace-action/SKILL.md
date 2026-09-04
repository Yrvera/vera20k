---
name: trace-action
description: "Trace one concrete player action or game mechanic through the production Rust path and compare its stages with active gamemd.exe behavior. Reports failures and unchecked boundaries; does not implement fixes."
---

# Trace a production action

Resolve the request into a concrete scenario with retail types, inputs, initial
state, and expected observable result. State reasonable in-scope assumptions.
Use [ENGINE.md](../../../ENGINE.md), relevant research, current Rust, and main-checkout
retail INIs; consult the System Map for navigation when helpful.

Follow the actual production chain from trigger through data, computation, state
changes, consumers, and any visual or audio result. Adapt the stages to the
mechanism. Read the whole loop and relevant lifecycle entry points so a working
helper does not hide a missing map-spawn, capture, destruction, or exit hook.

At each boundary establish inputs, outputs, the owning functions, timing, and
who observes the result next. Track exact conditions, arithmetic, rounding, units,
RNG, state order, and pre-update versus post-update reads when relevant. Verify
load-bearing native claims through the
[binary workflow](../../../docs/research/ghidra-workflow.md); research points to
evidence and does not settle uncertain or conflicting claims. Prove active-YR
reachability and the appropriate retail defaults.

Compare concrete Rust and native values or state transitions:

- **PASS:** the compared behavior agrees for the stated inputs and boundary.
- **FAIL:** decisive evidence demonstrates a difference.
- **NOT IMPLEMENTED:** active native behavior has no current production equivalent.
- **UNCHECKED:** either side or the necessary comparison is unresolved.

A matching example is not proof of mechanism equivalence. Distinguish code reasoning,
executed Rust checks, and retail/binary-derived evidence. Do not present hand-computed
values as machine-derived goldens or claim a runtime result you did not observe.

Trace visual/audio paths through their final consumers: composition, frames,
palette, position, timing, and ordering can diverge after correct sim state.
Investigate contradictory user observations instead of defending a partial trace.

Report the pipeline, earliest demonstrated divergence, consequential downstream
effects, and unchecked stages. Rank findings by trigger frequency and player or
deterministic impact, keeping low-priority exact differences visible. State the
scope actually traced; exhaustive requests cannot be reduced to a happy-path sample.

This skill is analysis-only. Report Ghidra metadata candidates; synchronize only
when expressly authorized under the binary workflow, after all readers stop.
Workers never mutate Ghidra. If gameplay fixes are already authorized, use the
findings to continue the parent implementation task without another approval gate.
