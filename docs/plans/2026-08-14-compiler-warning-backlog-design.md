# Compiler-Warning Backlog Cleanup Design

## Goal

Make every VERA20k package target compile without project-owned Rust compiler warnings while preserving runtime behavior, deterministic simulation state, and intentional parity/test scaffolding.

## Architecture Context

VERA20k is one Rust package with a public library root and fifteen explicit binaries. `src/lib.rs` publishes the engine layers and app orchestration modules; the official test boundary is the library test target. The live baseline is:

- `cargo check -p vera20k --lib`: 151 warnings across 88 files — 111 `dead_code`, 21 `unused_imports`, 12 `unused_variables`, 5 `private_interfaces`, and 2 `unused_mut`.
- Area distribution: 108 warnings in `src/sim`, 25 in `src/app*`, 13 in `src/map`, and one each in assets, render, rules, sidebar, and UI.
- `cargo test -p vera20k --lib --no-run`: 77 warnings — 51 shared with the library build and 26 test-only.
- `cargo check -p vera20k --all-targets`: additionally finds two stale integration-test API errors (`Simulation::particle_systems` is now an accessor and `RmgTileKeys` gained `ramp_smooth`) plus four integration-test warnings before the all-target scan can complete.

The compiler is reporting several structurally different conditions under the same warning umbrella:

1. Mechanical residue: unused imports, bindings, assignments, and unnecessary mutability.
2. Visibility mismatch: public items expose crate-private parameter or field types.
3. Unit-test-only or debug-only seams that are intentionally unused in a normal library build.
4. Compatibility/shadow entry points retained beside newer authoritative paths.
5. Truly orphaned private helpers and fields.

Existing project precedent is to align configuration and visibility with actual ownership instead of suppressing lints globally: commit `41cc24da` gates a debug/test-only movement re-export, while `d2a8ef47` removes unused imports and tightens public visibility. Item-local `allow(dead_code)` is already used for intentionally retained GPU ownership, test-only convenience APIs, and reference paths with an explanatory comment.

No gamemd.exe mechanism is being designed or changed. Research-index results for generic “dead code” concern native gameplay branches and are not evidence for whether Rust items can be removed; current Rust callers, configuration gates, tests, and history are authoritative for this cleanup.

## Impact Analysis

The cleanup may touch warning sites throughout `src/sim`, `src/app*`, `src/map`, the small remaining library areas, and `tests/`. No module dependency direction changes, public data-format migrations, dependency upgrades, snapshot changes, or gameplay API redesigns are in scope.

Risk areas:

- Removing a compiler-dead item that is retained as a parity shadow, test seam, or imminent authoritative handoff.
- Adding a broad lint suppression that hides future regressions.
- Changing a parameter name or visibility in a way that breaks call sites, integration tests, or public consumers.
- Gating code under `cfg(test)` when debug builds or tool binaries intentionally use it.
- “Using” an otherwise dead field merely to silence the compiler, accidentally changing ordering, RNG consumption, state hashing, serialization, or render composition.
- Fixing unrelated behavior while visiting broad warning-heavy modules.

The two all-target compile errors are narrow test maintenance prerequisites, not production behavior changes. They must be updated to the current public APIs so the package-wide warning gate can finish.

## Chosen Approach

Use a targeted, callsite-proven cleanup in four passes:

1. Apply behavior-neutral mechanical fixes: remove unused imports, remove unnecessary `mut`, remove overwritten assignments, and underscore intentionally unused bindings where their presence documents a callback or tuple contract.
2. Align visibility with actual consumers. Prefer `pub(crate)` or private visibility when every caller is within the crate; widen a private type only if an externally useful public contract is proven.
3. Classify every `dead_code` warning by searching all source, tests, binaries, configuration gates, and recent history:
   - delete only proven obsolete private code;
   - match `cfg(test)`/`cfg(debug_assertions)` to an item whose consumers are confined to those builds;
   - retain intentional compatibility, ownership, or parity seams with the narrowest item-level lint allowance and a reason comment;
   - never add crate-wide or broad module-wide suppression merely to reach zero.
4. Repair the two stale integration-test API uses and clear target-specific test warnings, then repeat the compiler inventory until all targets are clean.

This follows existing repository patterns and preserves the current architecture. It treats the compiler output as a classification queue, not as permission to delete gameplay mechanisms.

## Player-Experience Detail Ledger

- **MILESTONE-BLOCKING — Runtime behavior must remain bit-for-bit unchanged by intent.** No warning fix may add a read/write, call a dormant helper, alter a branch, or change tick-stage order. `[ENGINE.md: Native-to-Rust translation; Architecture boundaries]`
- **MILESTONE-BLOCKING — Deterministic state is untouched.** Simulation RNG consumption, entity iteration order, lifecycle ordering, snapshot fields, serialization, and state hashing must not change. `[ENGINE.md: Evidence; Native-to-Rust translation]`
- **MILESTONE-BLOCKING — Intentional staged mechanisms are not deleted solely because rustc calls them dead.** The backlog includes compatibility and shadow entry points in combat, movement, mission, miner, particles, pathfinding, and world orchestration. Their callers/configuration/history must be traced first. `[compiler: cargo check -p vera20k --lib; code: src/sim]`
- **COMPOUNDING — Future warnings must remain visible.** A crate-wide or broad module-wide `allow(dead_code)`/`allow(unused_*)` would make later abandoned paths accumulate silently and is forbidden. `[code: existing item-local lint attributes; git: 41cc24da, d2a8ef47]`
- **COMPOUNDING — Visibility must reflect ownership.** The five `private_interfaces` warnings currently expose private implementation types through wider APIs; align the public surface without widening simulation internals by default. `[compiler: private_interfaces diagnostics; code: src/sim/combat, src/sim/occupancy, src/ui/shell/slide]`
- **EXACTIFICATION-RESIDUAL — No native parity claim follows from a clean compiler.** Warning-free compilation certifies source hygiene only, not gamemd.exe parity or player-visible equivalence. `[ENGINE.md: Evidence]`
- **UNKNOWN-RISK — Additional target-specific warnings may appear after the two integration-test errors are repaired.** Keep the slice open until `--all-targets` completes cleanly; handle only newly revealed compiler hygiene within the same boundaries. `[compiler: current all-target run stopped on E0615 and E0063]`

## Design

### Components

- **Warning inventory:** rustc JSON diagnostics grouped by lint, target, file, and configuration.
- **Mechanical cleanup:** imports, bindings, assignments, and mutability at their existing owners.
- **API-boundary cleanup:** visibility changes at the five mismatched interfaces.
- **Dead-code classification:** caller/config/history checks for each private item, with deletion, configuration matching, or narrow documented retention.
- **Target repair:** current API updates in `tests/particle_render_integration.rs` and `tests/retail_ini_contracts.rs`, followed by any newly exposed test-only warning sites.

### Interfaces / Contracts

- Public engine APIs remain source-compatible unless an item is proven to have no external consumer and already exposes an unusable private type.
- Simulation behavior, snapshot representation, deterministic hashes, draw ordering, and INI interpretation do not change.
- Lint allowances, where unavoidable, are item-local and explain the non-obvious ownership/test/compatibility reason.
- No dependency versions, Cargo profiles, or crate-wide lint levels change.

### Data Flow

There is no runtime data-flow change. At build time, the process is:

`rustc diagnostic -> classify by owner/config/callers -> smallest source correction -> focused compiler check -> clean all-target check -> one final full --lib test run`.

### Error Handling

- If a supposedly dead item has a live caller under any target/configuration, preserve it and align its gate rather than deleting it.
- If a visibility fix would alter an intentional external API, retain the API and make the smallest defensible type visibility adjustment only after proving the contract.
- If cleanup exposes a behavioral test failure, stop and reassess that edit instead of layering changes.
- Newly revealed non-warning compile failures outside the two known stale integration-test API uses are residuals unless necessary to finish the all-target warning scan; any necessary repair remains test/build maintenance only.

### Testing Strategy

1. While editing, run `cargo check -p vera20k --lib` and focused `cargo test -p vera20k --lib <module_path>::` checks for touched logic-bearing modules. Check for other Cargo/Rust processes before every command.
2. Run `cargo check -p vera20k --all-targets` until it completes with zero VERA20k warnings and zero compile errors.
3. Run the full suite exactly once at the end: `cargo test -p vera20k --lib`. Record the literal `test result:` line and require zero warnings.
4. Format only edited leaf files with `rustfmt --edition 2024`; never run crate-wide formatting or rustfmt on a `mod.rs` file.

## Architectural Decisions

- Follow existing visibility tightening and configuration-gating patterns.
- Prefer deletion for proven orphans, configuration matching for test/debug-only seams, and narrow documented allowances for intentionally retained compatibility/ownership code.
- Do not introduce a new lint framework or warning baseline file; rustc remains the live ledger.
- Do not change gameplay to make a symbol appear used.
- No technical debt is intentionally added. Any retained dead-code allowance records why the item is still part of the architecture.

## Alternatives Considered

### Delete every item rustc calls dead

This produces the smallest release surface, but it confuses “not reached in this build target” with “architecturally obsolete.” It risks deleting parity shadows, compatibility entry points, and test seams across simulation systems. Rejected as too behavior- and rework-prone.

### Add crate- or module-wide lint allowances

This is fast and leaves runtime behavior unchanged, but it hides rather than eliminates the backlog and prevents new warning regressions from being visible. Rejected because warning cleanliness would immediately become unverifiable.

### Targeted classification and cleanup (chosen)

This requires more callsite inspection across 88 files, but it preserves architecture and produces a durable zero-warning baseline without broad suppression. It is the only approach that satisfies both compiler hygiene and the project’s parity/determinism constraints.
