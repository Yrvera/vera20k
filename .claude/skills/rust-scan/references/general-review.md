# General Rust review rules

Use these rules for every target. Confirm context before reporting; token matches
are candidates only.

Interpret these as evidence lenses for a requested review, not as an independent
coding standard. Words such as "inspect," "require," and "prefer" describe the
proof needed to classify a candidate or evaluate a fix direction; they neither
authorize nor require a production rewrite.

## Contents

- [Safety and failure behavior](#safety-and-failure-behavior)
- [Ownership, allocation, and hot-path cost](#ownership-allocation-and-hot-path-cost)
- [Macros and trait dispatch](#macros-and-trait-dispatch)
- [Structure and modern Rust](#structure-and-modern-rust)
- [Clippy interpretation](#clippy-interpretation)

## Safety and failure behavior

### SAFE-001 — unsafe contract

Inspect `unsafe` blocks/functions/traits/impls, FFI blocks, raw pointer
operations, `transmute`, `from_raw_parts`, `set_len`, unchecked indexing,
`assume_init*`, and manual `Send`/`Sync`. Inventory every reachable unsafe site;
do not sample them. Require a nearby `SAFETY:` rationale that proves the relevant
alignment, validity, initialization, aliasing, lifetime, allocation-provenance,
and thread-safety obligations. Comment presence alone is not proof. Check
`# Safety` docs for public unsafe functions and edition-2024 unsafe
attributes/extern blocks. `unsafe_op_in_unsafe_fn` is a lint requirement, not
evidence that an operation is unsound. Targeted Miri on pure, unsafe-heavy tests
can add evidence when supported, but a pass is not a proof and FFI/GPU paths may
be outside its model.

### SAFE-002 — reachable panic or placeholder

Inspect production `panic!`, `assert!`/`assert_eq!`, `todo!`,
`unimplemented!`, `unreachable!`, indexing, and other panic paths. Accept a
proved internal invariant and test-only assertions. Never skip reachable
`todo!` or `unimplemented!`. Classify by the input and player trigger, not by
macro name.

### ERR-001 — fallible input handling

For `unwrap`/`expect`, trace the value source. External/user/retail data, file
I/O, decoded assets, network input, and configurable INI/map values require a
real error boundary unless validation proves the invariant first. Test code and
documented post-validation invariants are normally acceptable. Also inspect
ignored `Result`, `let _ =`, `.ok()`, and fallback/defaulting when they can erase
a production failure; `Result` being `must_use` does not make an explicit discard
correct.

### ERR-002 — error-layer boundary

Inspect whether consumers receive the error information and recovery boundary
they need. Follow existing typed-domain and application-propagation conventions;
a crate choice alone is not a defect. Also inspect discarded errors (`.ok()`,
ignored results, broad fallback/default) when they hide malformed production input.

### SAFE-003 — lint suppression

Do not flag every `allow`. Review new, broad, stale, or unexplained
suppressions. Prefer `expect(lint, reason = "...")` for a local condition that
should disappear, while retaining documented persistent/generated/platform
allows when appropriate. Severity comes from the hidden issue.

### SAFE-004 — arithmetic, conversion, and portable width

At parser, serialization, indexing, coordinate, and simulation boundaries,
inspect narrowing or signedness-changing `as`, unchecked shifts/division, and
ordinary arithmetic whose overflow behavior matters. `usize`/`isize` and default
Rust layout are target-dependent; do not let them silently define portable save,
network, replay, or canonical-state formats. Choose `checked_*`, `wrapping_*`,
`saturating_*`, `overflowing_*`, or `TryFrom` only after establishing the required
domain or native behavior. A cast is not wrong by syntax alone.

## Ownership, allocation, and hot-path cost

Apply API-shape rules OWN-001 and OWN-002 only when the user selects the
`ownership` focus or a concrete finding requires ownership analysis. They are
not default performance findings. OWN-003 remains a default state check in
authoritative simulation because aliasing and borrow-panics can cross lifecycle
and tick boundaries.

### OWN-001 — borrowed versus consumed API

`&String` and `&Vec<T>` are candidates for `&str` and `&[T]` only when callers
need no concrete-container semantics. An owned parameter may intentionally be
stored, transferred, normalized, or dropped. Resolve use before suggesting a
signature change.

### OWN-002 — string identity and indirection

Do not recommend `&'static str` for runtime INI, map, mod, or player-provided
identifiers. Use the project's `InternedId`/newtype when repeated identity or
hot cloning justifies interning; otherwise ordinary owned strings may be right.
`Arc<str>`/`Box<str>` are options only when immutable ownership and lost spare
capacity fit the API.

### OWN-003 — shared interior mutability

`Rc<RefCell<T>>` in authoritative simulation is an ownership and reentrancy
candidate, not automatic nondeterminism. Inspect aliasing, borrow-panic
reachability, lifecycle ownership, and whether the value crosses tick or hash
boundaries. A documented thread-local `RefCell` used only for non-hashed
scratch is a different pattern and must not be reported under this rule.

### PERF-001 — allocation on a proved hot path

Use two stages. First trace from the current tick/frame spine and state the
multiplicity, or identify the item in a representative optimized-build capture.
Then inspect allocations inside that hot item. `Vec::new()` does not allocate;
growth, `with_capacity`, `vec!`, `collect::<Vec<_>>()`, `to_vec`,
strings/formatting, boxes, and cloning owned buffers may. Capacity-sensitive
`push`, `insert`, `extend`, `reserve`, string append, and map insertion require
call-site capacity evidence before claiming an allocation. Confirm the resolved
type and branch frequency. Suggest scratch reuse only after checking reentrancy,
clearing, retained memory, ordering, and authoritative-state exclusion.

### PERF-002 — clone cost

Never use a per-file clone-count threshold. Resolve the cloned type: a Copy-like
ID or `Arc` bump differs from a `String`/`Vec`/map clone. One clone inside a
20,000-entity loop can matter more than many setup clones. Report performance
only with a hot call path or measurement; report ownership confusion only when
the API makes it concrete.

### PERF-003 — measured cache and data-layout cost

Require a proved hot phase plus traversal order, fields touched, working-set size,
indirection, and cache/bandwidth evidence before recommending a layout change.
Report the representative release scenario, target hardware, sampling window,
milliseconds against an explicit subsystem budget, and relevant counters. Never
infer “AoS bad,” require an ECS/SoA rewrite, or replace the current storage owner
from syntax alone. Storage changes must preserve its consumers and ordering contract.

### PERF-004 — spatial-query and pathfinding scaling

In proved hot simulation items, inspect per-entity global scans, nested entity
loops, repeated nearest/overlap queries, full path recomputation, frontier growth,
index rebuild frequency and freshness, and query backlog. Record entity count,
candidate count per query, and update frequency. Broadphases, batching, tiles, or
sliced work are only fix directions after preserving verified tie order, same-tick
visibility, index rebuild semantics, and deterministic commit order.

### PERF-005 — observability cost and evidence

Stable phase spans and counters for tick duration, active entities, allocations,
spatial-query count, path backlog, and job wait/commit time can make a performance
claim reproducible. Inspect dynamic trace strings, formatting, and per-entity
events in hot loops. Instrumentation must remain outside canonical state, and
wall-clock values must never govern authoritative work. Missing instrumentation is
not itself a finding unless it blocks an explicit budget or regression contract.

## Macros and trait dispatch

Do not report declarative macros, procedural macros, derives, generics, trait
objects, or blanket impls merely because they obscure a text search.

- When a selected-focus behavior is generated, inspect the macro definition or
  compiler expansion that actually builds. Check argument evaluation count/order,
  hidden allocation or iteration, generated unsafe, `cfg` branches, and generated
  serialization/hash/state behavior as applicable.
- At a trait call, resolve the reachable concrete impl, default or blanket method,
  associated types/constants, and dispatch boundary. Review an `unsafe trait` or
  manual `Send`/`Sync` under SAFE-001. Treat dynamic dispatch as a performance issue
  only on a proved hot path with demonstrated cost.
- If expansion or monomorphization cannot be established, keep the result
  `CANDIDATE/UNCHECKED`; do not guess from the call-site name.

## Structure and modern Rust

### STRUCT-001 — module documentation

Module files should open with a `//!` purpose/dependency comment. Account for
shebangs, inner attributes, generated files, and intentionally included test
modules before reporting.

### STRUCT-002 — file growth and cohesion

Use continued growth as a cue to inspect cohesion and change locality.
Cohesive tables, state machines, and tests may belong together. Report the
concrete ownership or maintenance cost, not a crossed line-count threshold.

### MODERN-001 — lazy initialization

Map by semantics: lazy static initialization normally maps to `LazyLock`;
one-time explicit initialization to `OnceLock`; unsynchronized variants to an
appropriate `std::cell` type. Inspect direct crate use only, not transitive
lockfile entries.

### STYLE-001 — optional idioms

Generic rewrites such as iterator chains, `matches!`, let chains,
`derive(Default)`, `From`/`Into`, `is_multiple_of`, and `div_ceil` are INFO-only
and opt-in. Prove evaluation order, eagerness, overflow/zero behavior, ownership,
and domain meaning. Do not rewrite clear order-sensitive game logic merely to
look modern; rely on selected Clippy groups for routine style discovery.

## Clippy interpretation

Run only through `scripts/run_clippy.ps1` when requested. A diagnostic is still
a candidate: read its span and caller. Do not suppress compile errors or confuse
an unrelated build failure with a target issue. Severity follows verified
impact, not the Clippy group. Prefer the default correctness/suspicious/perf
groups plus targeted unsafe-contract lints. Do not enable `restriction` or
`nursery` wholesale; cherry-pick a lint only when its contract matches the scan.
