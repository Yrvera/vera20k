# Simulation risk rules

Apply to authoritative simulation and its inputs. Resolve production reachability,
types, guards, owners, and consumers before treating a match as a defect.
ENGINE owns policy; these IDs guide inspection.

## Determinism and ordering

### DET-001 — host floating point

Trace floating-point values into gameplay/canonical state. Presentation and
reference-test arithmetic are different. Establish native precision, evaluation
order, conversions, rounding, and supported-build consistency; fixed-point
replacement also needs semantic evidence.

### DET-002 — unordered collections

Membership/keyed lookup alone is deterministic. Trace exposed iteration,
serialization, selection, reductions, and RNG effects. Sorted keys may still
violate native scheduler order. `Hash`/`DefaultHasher` is not a stable
cross-toolchain persistence or canonical-digest contract.

### DET-003 — external entropy, clock, environment, or filesystem order

Trace entropy, clocks, environment, locale, and directory order into authoritative
decisions. Logging without feedback is not desync; canonical input selection
requires deterministic enumeration.

### DET-004 — casts, ranges, and native-width arithmetic

Prove ranges before judging casts. Negative signed-to-unsigned conversion can
break consumers; modulo cannot recover truncated bits. Distinguish cast truncation
from arithmetic overflow and preserve native wrap/clamp/round behavior.

### DET-005 — conditional production behavior

Inspect OS/architecture/feature/debug gates, including macro expansions.
Report differences affecting supported peers' production state, serialization,
phase logic, commands, or RNG; test-only helpers are not such differences.

### DET-006 — concurrency, atomics, and uninitialized storage

Race-free need not mean deterministic. Inspect reductions, equal-key selection,
`find_any`, completion-order buffers, and commit ordering against native semantics.
Atomics are not inherently nondeterministic. For `MaybeUninit`, prove initialization
before each read. Pure/read-only parallel work is allowed.

### DET-007 — scheduler, sort, heap, and tie order

Trace equal-key selection and ordering into effects. Stable-ID order may differ
from native insertion order; stable sorting cannot repair nondeterministic input.
Non-associative reductions and outcome-bearing ties need the correct deterministic order.

### DET-008 — RNG ownership and draw order

Verify stream ownership, caller order, draw count, rejected draws, cloning, and
same-tick commits. A deterministic generator with the wrong draw order still diverges.

## Authoritative state and lifecycle

### STATE-001 — serialization, hash, and snapshot coverage

For changed authoritative fields, check initialization, serialization, manual
hashing, snapshot compatibility, and continuation. `derive(Serialize)` does not
prove hash coverage. Skipped/derived state must rebuild deterministically without
losing future behavior.

### STATE-002 — lifecycle and authority ownership

Trace insertion/removal, ownership, limbo, scheduler membership, occupancy, radio
links, and reservations through lifecycle effects. Direct mutation is wrong when
it bypasses required registration, hashing, ordering, or same-tick consequences.

### STATE-003 — tick-spine and phase changes

Read the actual `SPINE REGION` comments and surrounding loop at command ingress,
combat/projectile resolution, deletion, and frame commit. Ordinary lints do not
establish phase order or same-tick visibility.

### STATE-004 — command and replay path

Compare live commands, AI intentions, delayed work, and replay: tick assignment,
ordering, rejection, duplicates, missing targets, and RNG. Alternate replay mutation
paths can hide divergence; another engine does not establish YR command timing.

## Architecture

### ARCH-001 — forbidden dependency

Check ENGINE's production simulation boundary through imports, aliases, re-exports,
and indirect edges. Separate genuine test/tool paths; an apparently harmless
production dependency still violates the boundary.

### ARCH-002 — presentation ownership leaked into simulation

Trace GPU/UI/audio handles and visual state to their owners and feedback.
Hash exclusion alone proves neither correct placement nor absence of effects
on future simulation.

## Coordinates and provenance

### COORD-001 — frame, unit, sign, and rounding boundary

Establish frames/units and shift/divide/round/clamp/sign behavior for coordinates,
height, facing, and foundation anchors. Check a concrete boundary fixture;
`256` or bit-shift syntax alone proves no bug.

### PROV-001 — gamemd-derived behavior provenance

Establish derivation before requiring nearby native provenance. Use the Ghidra
workflow's unknown-owner fallback where needed; Rust architecture glue requires
no invented native identity.

## Validation direction

Choose checks exposing the first divergent tick or state: live commands, varied
ordering, replay, or uninterrupted versus save/load continuation. Broaden build/
platform/worker matrices only when the risk crosses those boundaries.
Rust-vs-Rust agreement is regression evidence, not gamemd parity.
