# Simulation risk rules

Use these rules for `src/sim/` and for parsers, bootstrap code, commands, or
other code that feeds authoritative simulation. `ENGINE.md` wins if this file
ever conflicts with it.

Interpret these as evidence lenses for a requested review, not as independent
implementation policy. They identify questions the scan must resolve; verified
`gamemd.exe` evidence, `ENGINE.md`, and the explicitly authorized task govern any
later code change.

## Contents

- [Confirmation gate](#confirmation-gate)
- [Determinism and ordering](#determinism-and-ordering)
- [Authoritative state and lifecycle](#authoritative-state-and-lifecycle)
- [Architecture](#architecture)
- [Coordinates and provenance](#coordinates-and-provenance)
- [Validation direction](#validation-direction)

## Confirmation gate

Treat every lexical hit as a candidate. Before assigning severity:

1. Exclude comments, strings, test-only items, setup-only paths, diagnostics, and
   presentation-only data where the rule does not apply.
2. Resolve source and destination types, guards, callers, and the owning state.
3. Establish whether the path can affect commands, RNG draws, ordering,
   canonical state, serialization, hashing, persistence, or same-tick reads.
4. State the player trigger and frequency. A rare deterministic-state risk may
   still be critical, but must not be described as common.
5. For behavior changes, identify the gamemd evidence or label the equivalent
   `UNCHECKED`. Never invent the native correction.

## Determinism and ordering

### DET-001 — host floating point

Candidate signal: `f32`/`f64` types, literals, arithmetic, transcendental
functions, or fixed-to-float conversions in the simulation authority cone.

Confirm whether the value can affect gameplay or canonical state. Comments,
reference-only tests, and hash-excluded presentation state are not desync
findings, though presentation state living in `sim/` may be ARCH-002. Establish
native numeric semantics before judging the representation. For authoritative
floating point, inspect precision, evaluation order, conversions, rounding,
overflow, and supported-build consistency. A fixed-point replacement also needs
evidence that it preserves the required native behavior.

### DET-002 — unordered collections

Candidate signal: `HashMap`, `HashSet`, unordered collection, or upstream data
with unspecified order.

Membership and keyed lookup alone are deterministic. Confirm a risk only when
iteration, serialization, reduction, selection, command generation, RNG
consumption, insertion order, or a downstream sort exposes unordered order.
`BTreeMap` is not a universal fix: key order can itself drift from gamemd's
insertion/scheduler order. Storage and active-object scheduling have distinct
contracts; establish both from current owners and native evidence. Standard
`Hash`/`DefaultHasher` output is not a stable save, replay, network, or canonical
digest contract across toolchains and platforms; require an explicitly specified
canonical encoding and digest where stability matters.

### DET-003 — external entropy, clock, environment, or filesystem order

Candidate signal: `rand::rng`, `thread_rng`, `rand::random`, `OsRng`,
`getrandom`, entropy seeding, `SystemTime::now`, `Instant::now`, environment
reads, locale, or directory enumeration.

Confirm whether the result enters authoritative decisions. App-owned logging,
debug assertions, and evidence sinks are not desyncs when they cannot feed back
into simulation. Filesystem iteration must be explicitly sorted before it
selects canonical input.

### DET-004 — casts, ranges, and native-width arithmetic

Candidate signal: `as` conversion to integer types, especially narrowing,
signed/unsigned, float/integer, or `usize`/`isize` conversions.

Resolve the actual source type and prove its range. Casting a negative `i32`
to `usize` produces a large unsigned value, which may violate the consumer's
range. A modulo after a narrowing cast does not recover truncated bits. Cover
all relevant widths. Separate arithmetic overflow from cast truncation.
Do not automatically suggest clamp, saturate, or `TryFrom`: first establish
gamemd's wrap/truncate/clamp behavior and the supported-platform contract.

### DET-005 — conditional production behavior

Candidate signal: `cfg`, `cfg_attr`, `cfg!`, target OS/architecture/pointer
width, feature, or debug-only branches.

Test-only helpers are normally fine. Confirm a risk when configuration changes
production structs, serialization, phase logic, RNG use, commands, or canonical
results between supported peers/builds. Check aliases and macro-expanded gates
when a direct hit is ambiguous.

### DET-006 — concurrency, atomics, and uninitialized storage

Candidate signal: spawned work, Rayon/parallel iterators, async tasks, atomics,
`MaybeUninit`, or unsafe initialization.

Parallelism is allowed when work is pure/read-only or commits in a proved
deterministic order without changing RNG or same-tick visibility. Atomics,
including `Relaxed`, are not inherently nondeterministic; confirm that
inter-thread timing can affect authoritative state. For `MaybeUninit`, inspect
initialization coverage and each `assume_init*`/raw read rather than flagging
the type itself. Explicitly inspect Rayon `reduce*`, `sum`, `product`, equal-key
min/max, `find_any`, `par_bridge`, channel/completion-order collection, and
parallel mutation buffers. Race-free output is not necessarily deterministic.
Outcome-bearing work requires schedule-independent unique ordering keys and a
single deterministic commit in the verified authoritative order; thread index or
completion order is not such a key.

### DET-007 — scheduler, sort, heap, and tie order

Candidate signal: `sort*`, `BinaryHeap`, priority queues, entity snapshots,
stable-ID walks, insertion/removal, or equal-key comparators.

Trace the order into effects. Require a total, deterministic tie-breaker when
equal elements can change damage, lifecycle, commands, or RNG. Do not replace
native/live-object insertion order with stable-ID or sorted-key order merely
because it is deterministic. Also inspect equal-key min/max/find selection and
non-associative reductions: stable sorting cannot repair a nondeterministic input
order, and a deterministic key can still be the wrong native order.

### DET-008 — RNG ownership and draw order

Candidate signal: `SimRng`, `main_rng`, `scenario_rng`, `mapgen_rng`, stream
cloning, helper draws, or branch-dependent draws.

Verify the owning stream, caller order, draw count, rejected-draw behavior, and
same-tick state commit. New direct stream access outside an owning/routing seam
requires evidence. A deterministic PRNG algorithm can still desync when draw
order differs.

## Authoritative state and lifecycle

### STATE-001 — serialization, hash, and snapshot coverage

For every added or changed authoritative field in `Simulation`, `GameEntity`,
RNG, scheduler, production, house, trigger, or mission state, verify:

- serialization/deserialization or a justified `serde(skip)`;
- inclusion in the canonical state hash, or an explicit proof that it cannot
  feed future simulation;
- snapshot-version and compatibility impact;
- initialization/default and save/load/replay/hash mutation tests.

A `serde(skip)` field is high risk if its value can influence a later decision.
Do not infer coverage from `derive(Serialize)` alone when hashing is manual.
Classify state as authoritative, deterministically derived, or presentation-only.
Derived state must rebuild deterministically and should remain diagnosable by a
canonical dump/hash when it can expose a divergence.

### STATE-002 — lifecycle and authority ownership

Inspect direct entity-store insertion/removal, pending-delete writes, scheduler
or LogicVector edits, owner changes, reveal/conceal, limbo/unlimbo, occupancy,
radio links, and reservations. Confirm that the owning lifecycle/helper performs
side effects in native order. A direct mutation is not automatically wrong, but
must not bypass registration, hashing, occupancy, or same-tick consequences.

### STATE-003 — tick-spine and phase changes

Elevate changes around command ingress, master-frame phases, live-object
snapshots, combat/projectile resolution, pending deletion, and frame commit.
Read the current `SPINE REGION` comments and relevant closed loop. Verify
ordering and same-tick visibility even if ordinary lint checks pass.

### STATE-004 — command and replay path

Inspect player/network commands, AI intentions, delayed work, and replay records
as ordered data entering the live simulation path. Verify tick assignment,
ordering, duplicate and missing-target behavior, rejection semantics, RNG effects,
and same-tick visibility. Replay-only callbacks or alternate mutation paths are
candidates because they can hide divergence. Do not import another engine's
command delay, rollback window, or phase boundary; `gamemd.exe` evidence owns the
exact contract.

## Architecture

### ARCH-001 — forbidden dependency

Production `sim/` must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or
`net/`. Search direct imports, fully qualified paths, grouped imports, aliases,
re-exports, and indirect module edges. Separate genuine test-only references.
A confirmed production edge is critical even when it currently appears harmless.

### ARCH-002 — presentation ownership leaked into simulation

Candidate signal: screen/pixel coordinates, sprites, textures, GPU/UI types,
audio handles, or render-only animation state stored under `sim/`.

Confirm ownership and feedback direction. Hash-excluded presentation data is
not automatically a determinism fault, but it may violate the headless/replay
boundary or let presentation state feed authoritative logic.

## Coordinates and provenance

### COORD-001 — frame, unit, sign, and rounding boundary

For cell/lepton/screen, foundation-anchor, facing-byte, height, and isometric
conversions, require named source and destination frames/units plus exact
shift/divide/round/clamp/sign semantics. Walk one concrete boundary fixture.
Literal `256` or bit shifts are candidates, not proof of a bug.

### PROV-001 — gamemd-derived behavior provenance

When simulation semantics are derived in any way from gamemd, verify the nearby
provenance comment naming the mechanism and verified identity/address, using the
Ghidra workflow's unknown-owner fallback when necessary. Absence is a finding only after establishing derivation;
pure Rust architecture glue does not need invented provenance.

## Validation direction

Match the proposed validation to the confirmed risk rather than adding a generic
certification matrix:

- arithmetic, parser, lifecycle, and command boundary tests for local contracts;
- twin simulations comparing canonical state at every tick, including varied
  insertion or worker order when ordering is at risk;
- uninterrupted versus save/load/continue for snapshot coverage;
- repeated replay through the live command path for command/RNG/order risks;
- the first divergent tick plus canonical human-readable state dumps for diagnosis.

Cross-toolchain, architecture, optimization, or worker-count runs are appropriate
only when the finding crosses those boundaries. Rust-vs-Rust agreement is a
regression check; it does not verify parity with `gamemd.exe`.
