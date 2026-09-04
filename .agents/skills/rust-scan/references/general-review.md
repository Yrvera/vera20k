# General Rust review rules

Inspect relevant rule IDs in context. Lexical hits and lint messages are candidates,
not mandates to rewrite working code.

## Safety and failure behavior

### SAFE-001 — unsafe contract

Inspect reachable unsafe/FFI/raw-memory operations and manual `Send`/`Sync`.
Establish validity, initialization, aliasing, lifetime, provenance, and thread
obligations; a `SAFETY:` comment or Miri pass alone proves neither safety nor unsoundness.

### SAFE-002 — reachable panic or placeholder

Trace panics, indexing, assertions, `todo!`, and `unimplemented!` to production
inputs. Test assertions and proved invariants are different from user-triggered
failures; do not skip reachable placeholders.

### ERR-001 — fallible input handling

Trace `unwrap`/`expect`, discarded results, and defaults to their source.
External data needs an error boundary or prior validation proving the invariant;
`must_use` does not make explicit error dismissal correct.

### ERR-002 — error-layer boundary

Check whether consumers retain needed error information and recovery options.
Crate choice alone is not a defect; a broad fallback hiding malformed retail
input can be.

### SAFE-003 — lint suppression

Inspect new, broad, stale, or unexplained suppressions for the hidden issue.
Documented generated/platform allowances can be legitimate; suppression syntax
does not establish severity.

### SAFE-004 — arithmetic, conversion, and portable width

Resolve ranges, overflow, shifts, division, and casts at input/state boundaries.
Native-width integers and default Rust layout cannot silently define portable
formats. Choose checked, wrapping, saturating, or fallible conversions from semantics.

## Ownership, allocation, and hot-path cost

OWN-001/002 are API lenses when requested or needed for a concrete issue.
OWN-003 also applies to authoritative-state review.

### OWN-001 — borrowed versus consumed API

Suggest `&str`/`&[T]` only when concrete-container semantics are unnecessary.
Owned arguments may intentionally be stored, transferred, normalized, or dropped.

### OWN-002 — string identity and indirection

Runtime INI/map/player identifiers cannot become `&'static str`. Interning or
`Arc<str>` needs actual identity, cloning, and ownership benefits; ordinary
owned strings may fit.

### OWN-003 — shared interior mutability

Trace `Rc<RefCell<T>>` aliasing, reentrancy, borrow-panics, and lifecycle.
It is not automatically nondeterministic; non-hashed thread-local scratch differs
from authoritative shared state.

### PERF-001 — allocation on a proved hot path

Establish a reachable hot path and multiplicity or optimized-build measurement.
`Vec::new()` does not allocate; growth depends on capacity. Scratch reuse requires
correct clearing, reentrancy, retention, order, and authoritative-state exclusion.

### PERF-002 — clone cost

Resolve the type and hot caller: an ID copy or `Arc` increment differs from
cloning buffers. Per-file clone counts cannot establish cost.

### PERF-003 — measured cache and data-layout cost

Cache/bandwidth/budget claims need representative release measurements, hardware,
entity count, sampling window, time, and relevant counters. Do not infer layout
problems from AoS/SoA syntax; preserve storage consumers and ordering.

### PERF-004 — spatial-query and pathfinding scaling

Check nested scans, query multiplicity, path recomputation, frontier growth, and
index rebuild/freshness. Batching or slicing must preserve tie order, same-tick
visibility, and deterministic commits.

### PERF-005 — observability cost and evidence

Inspect per-entity tracing, formatting, and counters on hot paths. Measurements
stay outside canonical decisions. Missing instrumentation is a finding only when
it blocks an explicit measurement/regression requirement.

## Macros and trait dispatch

Resolve generated behavior and concrete/default/blanket implementations where they
affect evaluation order, allocation, unsafe, `cfg`, or canonical state. Unresolved
dispatch remains unchecked; abstraction use alone is not a defect.

## Structure and modern Rust

### STRUCT-001 — module documentation

Check purpose/dependency documentation, accounting for generated files, inner
attributes, and included test modules before reporting absence.

### STRUCT-002 — file growth and cohesion

Growth prompts inspection of ownership and change locality. Cohesive tables,
state machines, or tests need no arbitrary line-count split.

### MODERN-001 — lazy initialization

Distinguish lazy initialization from explicit one-time setup and synchronization
needs. Inspect direct dependencies, not merely transitive lockfile entries.

### STYLE-001 — optional idioms

Style suggestions are optional. Iterator or arithmetic rewrites must preserve
evaluation order, eagerness, overflow, zero behavior, and ownership.

## Clippy interpretation

Use the requested bundled wrapper. Inspect diagnostics in source/caller context;
severity follows consequence, not lint group. Avoid wholesale `restriction` or
`nursery` activation.
