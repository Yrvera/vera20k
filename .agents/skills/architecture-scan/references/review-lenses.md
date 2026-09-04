# Architecture review lenses

Use relevant sections to establish concrete consequences; these are review lenses,
not additional implementation policy.

## Packages and targets

Resolve actual Cargo targets and feature interactions. Multiple binaries or a
large crate do not justify a workspace split. A proposed package boundary needs a
real compilation, dependency, reuse, or API benefit without cycles or duplicated authority.

## Modules, files, and placement

Read module contracts and consumers. Place responsibility with its state,
invariants, and lifecycle; shared placement needs a semantic reason, not a consumer
quota. Mixed old/new locations during an owned migration are not settled debt.

## Ownership and dependency direction

Trace constructors and all relevant mutation paths, including macros and trait
methods. Check ENGINE's layer direction. Direct access is wrong when it bypasses
authority, ordering, or lifecycle; a messaging abstraction is not automatically better.

## Visibility and API seams

Check whether fields, re-exports, and signature types expose implementation details
or bypass invariants for actual consumers. Public passive data can be appropriate;
mirrored getters do not create encapsulation. Apply semver concerns only to real
compatibility contracts.

## Cohesion and change locality

Support recurring fanout with history explaining why files changed together.
A migration or generated update is not chronic coupling. Large cohesive tables,
state machines, and tests may belong together; show a benefit before moving them.

## Names and navigation

Use verified domain vocabulary. Demonstrate conflicting concepts or misleading
ownership at an actual seam before recommending a rename. Length, unfamiliarity,
and stylistic preference alone are insufficient.

## Traits and macros

Resolve implementations, consumers, defaults, blanket methods, and generated code.
Implementation count alone establishes neither usefulness nor needless abstraction.
Inspect expansions affecting visibility, dependencies, serialization, and ownership.

## Tests and tools

Choose placement by the tested boundary. Avoid exposing internals merely for tests.
Check whether tool-only dependencies unnecessarily enter production or increase
build cost; binary count alone proves neither.

## Parity-preserving structural work

Unchanged bodies can still change behavior through `cfg`, caller order, constructors,
error routing, serialization, or registration. Inspect native/Rust evidence needed
to establish preserved lifecycle, RNG, scheduling, persistence, and same-tick effects.
Architectural taste cannot settle semantics.

## Primary sources

- [Rust modules](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
- [Cargo layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Rust visibility](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
- [Rust API guidance](https://rust-lang.github.io/api-guidelines/)
- [Rust test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
