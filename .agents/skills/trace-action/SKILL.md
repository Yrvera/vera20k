---
name: trace-action
description: "Compare one concrete action's production Rust path with active gamemd.exe. Reports divergences and unchecked stages; no fixes."
---

# Trace an action

Set concrete retail inputs, initial state and observable result. Follow the production
chain through trigger, computation, state, consumers and visual/audio output.
Inspect relevant lifecycle entry points: a working helper can hide missing map-spawn,
capture, destruction or exit hooks.

Apply [ENGINE.md](../../../ENGINE.md) and the
[binary workflow](../../../docs/research/ghidra-workflow.md). At each boundary
compare concrete native/Rust values or transitions, including timing, units, rounding,
RNG, operation order and pre-/post-update reads.

- **PASS:** agreement for the stated inputs and boundary.
- **FAIL:** demonstrated difference.
- **NOT IMPLEMENTED:** active native behavior lacks a production equivalent.
- **UNCHECKED:** either side or its comparison remains unresolved.

Distinguish calculated values, executed checks and native-derived evidence. Matching
examples do not prove mechanism equivalence. Explicit exhaustive coverage cannot
be reduced to a happy-path sample.

Report the pipeline, earliest divergence, downstream effects and unchecked stages,
ranked by trigger frequency and impact. Keep source unchanged. Ghidra sync is opt-in
under the binary workflow; workers report candidates only. Already-authorized
implementation may continue in the parent task from these findings.
