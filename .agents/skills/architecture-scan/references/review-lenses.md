# Architecture review lenses

These are questions for gathering evidence, not universal coding standards. A
rule word such as "prefer" or "require" describes what the scan must establish
before reporting a finding; it does not authorize a rewrite. `ENGINE.md` owns
project policy and overrides generic Rust guidance.

## Contents

- [Packages and targets](#packages-and-targets)
- [Modules, files, and placement](#modules-files-and-placement)
- [Ownership and dependency direction](#ownership-and-dependency-direction)
- [Visibility and API seams](#visibility-and-api-seams)
- [Cohesion and change locality](#cohesion-and-change-locality)
- [Names and navigation](#names-and-navigation)
- [Traits and macros](#traits-and-macros)
- [Tests and tools](#tests-and-tools)
- [Parity-preserving structural work](#parity-preserving-structural-work)
- [Primary sources](#primary-sources)

## Packages and targets

- Start from Cargo's actual package and target model. One package with a library
  and several binaries is valid. Conventional `src/lib.rs`, `src/main.rs`,
  `src/bin/`, `tests/`, `examples/`, and `benches/` placement improves navigation,
  but compatibility and project-specific target declarations can justify variants.
- Consider a new package/crate only when it creates a useful compilation,
  dependency, reuse, testing, platform, or stable-API boundary. Size alone does
  not justify a workspace split. Account for dependency cycles, duplicated types,
  build cost, feature unification, and migration burden.
- Treat feature flags as additive capability choices where practical. Confirm the
  production surface affected before reporting package or feature topology.

## Modules, files, and placement

- Remember that modules define namespace, privacy, and paths; files store module
  bodies. A file move or split can improve navigation without changing architecture.
- Place code with the responsibility that owns its state, invariants, lifecycle,
  or externally meaningful contract. Justify shared placement through responsibility,
  dependency direction and actual consumers; consumer count alone decides nothing.
- Read the crate root, parent module, `//!` contract, definitions, and callers.
  Folder depth, sibling count, and filename prefixes are discovery signals only.
- Check whether a facade offers one coherent entry point while keeping its
  implementation private. Do not add a facade merely to reduce import length.
- Recognize active migrations from dirty state and recent focused commits. Mixed
  old/new placement during a bounded move is not automatically architectural debt.

## Ownership and dependency direction

- Identify one truthful owner for mutable or authoritative state and for the
  operations that maintain its invariants. Confirm all mutation paths, including
  constructors, commands, callbacks, tests, macros, and trait methods.
- Compare actual imports and calls with `ENGINE.md`'s layer direction. Separate
  production edges from test-only, diagnostic, generated, and tool edges.
- Prefer communication through the owning layer's command, DTO, event, query, or
  narrow method when direct access would bypass ordering or lifecycle. Do not add
  messaging abstractions where a direct lower-level dependency is already valid.
- Flag a "common", "shared", "manager", or "helpers" module only when it has
  become a second owner, a dependency dumping ground, or a material navigation trap.

## Visibility and API seams

- Rust items are private by default and support restricted visibility such as
  `pub(super)`, `pub(in ...)`, and `pub(crate)`. Inspect actual consumers and use
  the narrowest visibility that truthfully serves them; `pub` alone is not a defect.
- Treat `pub use` as an architectural facade: verify that it exposes the intended
  concept without making implementation placement or an internal dependency part
  of the calling contract.
- Distinguish an application crate's test-facing surface from a published stable
  library API. Apply semver-oriented Rust API guidance only where compatibility is
  an actual project requirement.
- Prefer private fields for invariant-bearing types and public fields for honest
  passive data when that is the intended contract. Accessors that merely mirror
  every field do not create encapsulation.
- Trace dependency types in public signatures. A third-party type, concrete
  container, trait bound, or error type can become part of the boundary, but this
  matters only to real consumers.

## Cohesion and change locality

- Ask whether a module has one coherent reason to change and whether common work
  stays near the owning code. Confirm unrelated responsibility or repeated fanout
  before reporting; line count is only a cue.
- Use focused git history when claiming chronic co-change, but inspect why files
  changed together. A one-time migration, generated update, or parity slice is not
  evidence of permanent coupling.
- A large cohesive table, parser, state machine, or test corpus may be easier to
  reason about together. A short file can still be misplaced or split an invariant.
- Prefer the smallest boundary improvement. Moving files without tightening
  ownership, visibility, dependencies, or navigation may add churn with no gain.

## Names and navigation

- Follow Rust casing and Cargo target conventions unless compatibility requires
  otherwise. More importantly, use the repository's verified domain vocabulary.
- A name should let a maintainer predict the responsibility and likely owner.
  Demonstrate ambiguity through conflicting concepts, imports, or callers before
  reporting a rename; unfamiliar or long names alone are not findings.
- Keep conversion and iterator names consistent with established Rust meanings
  (`as_`, `to_`, `into_`; `iter`, `iter_mut`, `into_iter`) when those APIs exist.
- Module documentation should state purpose and important dependencies. It should
  not become a hand-maintained completion ledger or duplicate implementation detail.

## Traits and macros

- Introduce or retain a trait for a real behavioral contract, multiple meaningful
  implementations, a test seam, or an intentional static/dynamic dispatch boundary.
  A single implementation is neither proof of needless abstraction nor proof that
  a trait is useful.
- Resolve implementors, consumers, associated types, blanket/default methods, and
  object use before judging placement. Sealing is appropriate only when external
  implementations are deliberately excluded.
- Place macros with the domain or infrastructure that owns the generated contract.
  Inspect the expansion that actually compiles when it affects visibility,
  dependencies, state ownership, serialization, or tests. Macro use is not itself
  an architecture smell.

## Tests and tools

- Colocated unit tests may exercise private details. `tests/` integration crates
  should exercise the public boundary. Rustdoc examples document and compile API
  use; `examples/` provides runnable programs. Choose placement by the seam tested,
  not by a universal rule.
- Keep binary entry points thin when logic can be owned and tested by the library,
  but do not expose internals publicly merely to satisfy a test.
- Separate tool-only dependencies and code paths from the production game when
  they would otherwise expand runtime boundaries or compile cost. Many declared
  binaries are not automatically evidence that separate packages are needed.

## Parity-preserving structural work

- A structural move is not behavior-neutral merely because the function bodies
  look unchanged. Check feature/`cfg` reachability, caller order, error routing,
  constructors, serialization, registration, tests, and public paths.
- For authoritative simulation, preserve scheduler and tie order, RNG ownership
  and draw count, lifecycle effects, same-tick visibility, snapshot/hash coverage,
  and gamemd provenance. Inspect native/Rust evidence needed to assess the boundary;
  use a focused parity workflow when helpful. Record unrelated behavior questions
  separately without implementing them. Architectural taste cannot settle semantics.
- Do not recommend translating gamemd's C++ class hierarchy literally. Preserve
  verified behavior contracts while using Rust-native modules, ownership, and APIs.

## Primary sources

- [Rust Book: packages, crates, and modules](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
- [Cargo Book: package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)
- [Cargo Book: workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Rust Reference: visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — recommendations,
  explicitly not universal mandates
- [Rust Book: test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
