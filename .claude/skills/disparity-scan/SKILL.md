---
name: disparity-scan
description: "Compare current Rust with active YR behavior across a system or through one concrete action's production path. Separate demonstrated differences from unresolved candidates."
---

# Compare Rust with native behavior

Use [ENGINE.md](../../../ENGINE.md) and the
[binary reference](../../../docs/research/ghidra-workflow.md). Match the method to the
request: inventory a system's behavior or trace concrete retail inputs through an action.

Inspect current production Rust and consumers for every comparison. Research and
missing symbols do not prove missing behavior. Verify consequential, uncertain,
stale or conflicting native claims against the binary and retail data.

For an action, follow trigger, computation, state changes, consumers and visual/audio
output. Check lifecycle entry points that may bypass a working helper, including
map-spawn, capture, destruction and exit. Compare concrete values or transitions,
including timing, units, rounding, RNG, operation order and pre-/post-update reads.

Distinguish:

- Demonstrated gaps: established active-YR behavior differs from current Rust.
- Research-derived candidates: the native premise still needs proof.
- Unresolved Rust state or comparisons.
- Matches limited to the checked inputs/boundary.
- Inactive/TS behavior excluded with gate/default evidence.

Matching examples do not prove mechanism equivalence. Separate calculations, executed
Rust checks and native-derived evidence. Explicit exhaustive requests require complete
declared coverage; report missing or uninspected behavior.

Explain the earliest divergence, downstream effects, evidence and Rust locations.
Group shared causes/prerequisites and rank frequency, impact and determinism risk.
A missing prerequisite leaves dependent gaps blocked, not correctly absent.

Choose delegation and presentation to fit the task; save reports when requested or
useful. Comparison alone leaves source and research inputs unchanged. Already-authorized
implementation can continue from these findings. Ghidra authority follows the binary reference.
