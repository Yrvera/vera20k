# PR Clean-Checkout Stabilization Design

## Goal

Make the frozen cumulative `dev` tree build and test from a public clean
checkout without weakening the ordinary-skirmish production path, shipping
retail INIs, hiding deterministic drift, or starting another parity owner.

## Architecture Context

`AssetManager` owns production access to the installed RA2/YR archive stack.
It resolves named files in archive-priority order and already supplies
`MPModesMD.ini` and each mode override to `src/skirmish_modes.rs`. The current
production loader nevertheless embeds the ignored loose
`ini/mpmodesmd.ini` as a compile-time fallback. The app also calls a second
embedded-stock fallback when startup assets are absent. This bypasses the
production authority boundary and makes a clean checkout uncompilable.

Default unit tests have a second, separate coupling to the ignored loose
`ini/` tree. Seven `include_str!` call sites require retail files at compile
time, and eighteen tests read loose retail INIs at runtime. Some of those are
true retail-corpus certification checks; others are behavioral tests that only
need a narrow rules profile. GitHub Actions provisions no retail files, and
the repository intentionally ignores `/ini/`.

The deterministic global replay ratchet is independent of the INI problem.
Record/replay equality and the absolute RNG stream tuple pass, but the first
historical schema probe and final world hash do not match their committed
constants. That baseline must be traced to its first causal commit before
either behavior or constants change.

Relevant verified MPModes evidence:

- The active YR loader reads the roster from `MPModesMD.ini` and constructs
  rows from category data. [doc:
  `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md` §9]
- Constructor defaults are `AlliesAllowed=true` and `MustAlly=false`; an
  available per-mode override may replace them. `MustAlly` is cleared when
  allies are disabled. [doc:
  `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES_GHIDRA_REPORT.md` §§4,9]
- Stock exposes nine rows and no Siege row, but this is retail data, not a
  reason to compile a private loose INI into the executable. [same report §9]

## Impact Analysis

Production changes are confined to the shell data-loading boundary:

- `src/skirmish_modes.rs`
- the narrow startup call in `src/app.rs`

Hermetic-test repairs may touch only tests and purpose-built fixtures in:

- `src/rules/ini_value.rs`
- `src/rules/object_type.rs`
- `src/rules/ruleset.rs`
- `src/rules/warhead_type.rs`
- `src/app_cursor.rs`
- `src/map/rmg/{pipeline,tech_catalog,tiles}.rs`
- `src/sim/production/production_placement_tests.rs`
- `src/sim/miner/outbound_drive_tests.rs`
- `tests/fixtures/ini/`
- existing or new ignored retail-certification tests

Portable-path cleanup is confined to:

- `src/app_init_helpers.rs` test-only retail helper
- `tools/rmg_oracle/harness.py`
- `tools/shell_certification/README.md`
- `tools/shell_certification/tests/test_title_differential.py`
- the one newly tracked RGB565 design document containing a worktree path

No simulation production code, tick order, RNG routing, snapshot schema,
rendering, capture lifecycle, or gameplay rule value changes as part of the
INI/path repair. Replay-baseline action remains gated on provenance evidence.

## Chosen Approach

### 1. Keep production asset-authoritative and fail closed

Remove every production compile-time dependency on loose `ini/` files.
`skirmish_modes_from_assets` will load the roster only through
`AssetManager`. Missing, invalid, or empty `MPModesMD.ini` will return an
explicit load error; the app will log it and retain an empty mode roster.
Startup without an `AssetManager` will also retain an empty roster. The app
already marks the native shell failed when startup assets or main-menu chrome
are unavailable, and production consumers use checked `mode_by_id` lookups
rather than indexing the roster.

Per-mode override absence will use the verified native constructor defaults,
not filename-specific hardcoded stock values. A present override is parsed and
applied through the existing `[MultiplayerDialogSettings]` path. This follows
native behavior while ensuring the installed retail/mod archive remains the
authority.

### 2. Keep behavioral tests green with narrow synthetic INI fixtures

Tests whose subject is a parser, command, cursor, RMG phase, placement
handoff, or miner loop will receive only the sections and keys required by
that contract. Larger profiles, particularly the outbound-miner production
loop, will live in clearly labelled tracked files under
`tests/fixtures/ini/`; small parser cases remain inline.

The fixtures are synthetic test inputs, not substitutes for retail data.
They may use verified stock-shaped values when the assertion specifically
tests a stock contract, but they will not copy complete retail files or become
a production fallback.

True corpus/source checks will remain available as explicit ignored
retail-certification tests loaded at runtime through `AssetManager` and
`RA2_DIR`. Every player-visible or deterministic behavioral assertion that can
run against a narrow fixture stays in the default suite. Only the claim
“these values are present in the installed retail corpus” requires retail
assets.

### 3. Make local evidence paths explicit configuration

Replace user-profile and worktree literals with documented environment
variables or placeholders:

- `RA2_DIR` for an installed game directory
- `VERA20K_GAMEMD_EXE` for the RMG Unicorn harness
- `VERA20K_SHELL_GUARD`
- `VERA20K_ORACLE_RUNS`
- `VERA20K_SHELL_CAPTURE`

Tests that use local sealed evidence remain explicitly skipped unless all
inputs are configured. Documentation examples use placeholders or environment
variables. The five newly tracked design documents stay in the PR because
they are implementation provenance; the personal worktree path in the RGB565
design is removed.

### 4. Adjudicate the replay ratchet before changing it

Run the exact replay test at the parent and child of the suspected first
behavior commit, then narrow further if needed. Record:

- the first commit where each historical probe/final hash changes;
- record-versus-replay equality;
- the absolute RNG stream tuple;
- which hashed entity/component state changed;
- why that state change is intended or unintended.

If the change is unintended, fix the behavior and retain the constants. If it
is the intended result of a verified production-loop correction, update the
affected historical/final constants together, with a source comment naming
the causal change and literal old/new hashes. Do not change the stream tuple,
skip the test, or bump `SNAPSHOT_VERSION` for a behavior-only ratchet.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — Stock and modded mode rows must come from the installed
  archive in production; a compiled loose-file fallback can silently show
  stale modes or wrong override behavior. [current `AssetManager` path;
  MPModes reports above]
- `MILESTONE-BLOCKING` — Team Game, Free For All, and other mode ally/team
  behavior must continue to receive per-mode override values. Missing
  overrides use constructor defaults; no filename magic is introduced.
  [doc: override payload report §§4,9]
- `COMPOUNDING` — Default CI must still execute the outbound miner, placement,
  cursor, RMG, and parser behavioral paths. Simply ignoring all retail-coupled
  tests would make later integration drift easier to merge.
- `COMPOUNDING` — The replay ratchet must continue to detect deterministic
  world-state changes. A copied current hash without provenance could bless a
  real scheduler, navigation, lifecycle, or persistence defect.
- `COMPOUNDING` — Synthetic INI fixtures must never enter production data flow
  or be described as retail parity evidence.
- `EXACTIFICATION-RESIDUAL` — Default public CI cannot prove that a private
  retail installation contains each expected stock value. That source check
  remains an explicit ignored, asset-enabled certification gate. Player effect
  is none when production reads the real archive; risk is bounded by retaining
  the local retail gate.
- `EXACTIFICATION-RESIDUAL` — No native desktop capture, Oracle input, pixel
  certification, or shell-route certification is added to this PR hygiene
  slice.

## Design

### Components

- `SkirmishModeLoadError`: a small app/UI data-load error identifying missing,
  invalid, or empty roster data.
- Existing `AssetManager`: unchanged production authority.
- Narrow synthetic fixture files: test-only, purpose-labelled, and excluded
  from production modules except under `#[cfg(test)]`.
- Existing ignored retail tests, expanded only where needed to retain source
  assertions.

### Interfaces / Contracts

- `skirmish_modes_from_assets(&AssetManager)` returns an explicit result rather
  than a silently substituted roster.
- `parse_mpmodes_ini_with_overrides` keeps native default construction and
  applies any supplied override; it does not infer values from an override
  filename.
- Default tests have no dependency on `/ini/`, `RA2_DIR`, a user profile, or a
  retail install.
- Retail-certification tests require `RA2_DIR`, are marked ignored, and fail
  clearly when deliberately invoked without the configured asset root.

### Data Flow

Production:

`GameConfig RA2 path → AssetManager archive priority → MPModesMD.ini → roster
parse → referenced override lookup → native defaults plus override values →
AppState skirmish_modes`

Default tests:

`tracked narrow fixture/inline string → same parser or production-loop helper →
behavior assertion`

Retail certification:

`RA2_DIR → AssetManager → real merged retail data → source/profile assertion`

### Error Handling

- A missing/invalid/empty roster is logged once at startup and leaves no
  selectable modes; it never substitutes guessed stock data.
- Missing/invalid optional mode overrides are logged and leave verified native
  constructor defaults.
- Local evidence tools name the missing environment variable or file.
- Replay mismatch remains a hard test failure until provenance is resolved.

### Testing Strategy

1. Focused tests for each converted fixture consumer.
2. The exact global replay ratchet.
3. Full `cargo test` and `cargo check`.
4. Headless asset-enabled MPModes and converted retail-certification tests
   against the configured local install.
5. Existing Python suites for parity ledger, System Map, research index, shell
   certification, exact-shell matrix, and tactical certification.
6. A fresh detached clean checkout with no root `ini/` directory: `cargo
   check` and full `cargo test`.
7. Path and secret scans, `git diff --check`, and a final process/lease audit.

## Architectural Decisions

The design follows the existing `AssetManager` archive-lookup pattern and the
project rule “Rust-native structure, gamemd-native semantics.” It removes a
second authority instead of inventing a new loader. Test fixtures remain below
production authority and exercise the same parsers and loops.

No gameplay behavior is intentionally simplified. The only default-suite
change is replacing private input provenance with hermetic inputs; source
fidelity remains separately executable. No simulation hash is accepted until
its causal behavior has been proven.

## Alternatives Considered

### Track the retail INIs

Rejected. It would publish proprietary corpus data, conflict with the current
ignore policy, and turn one installation's loose extraction into production
authority.

### Provision private retail INIs through CI secrets

Rejected as the primary solution. Fork pull requests cannot access secrets,
hosted runners would remain operationally fragile, and compilation would still
depend on private data. It may be added later as a non-required certification
environment, not as the public build gate.

### Embed a reduced “stock” roster in production

Rejected. It would compile a second authority into the executable, drift from
mods/retail assets, and repeat the hardcoding problem in a smaller form.

### Mark every retail-coupled test ignored

Rejected. It would make CI green by dropping deterministic and player-visible
loop coverage. Only retail-source assertions may be ignored; behavioral tests
receive hermetic fixtures.

## Adversarial Approval

Why should this be approved? It restores a clean public build by removing
private input assumptions while strengthening the intended production
authority. It keeps normal-play behavior, deterministic loop tests, and honest
retail-only certification separate instead of conflating them.

What evidence could still make it wrong?

- If the current `AssetManager` cannot resolve `MPModesMD.ini` or its overrides
  from the configured retail stack, removing the fallback would expose a
  production regression. A headless retail smoke test is therefore a blocking
  gate.
- If any production consumer indexes an empty roster, fail-closed startup could
  panic. The current source audit found only checked production lookups; this
  is rechecked after implementation.
- If a synthetic fixture omits a state that the production-loop test actually
  consumes, the test may become weaker. Each converted test must retain its
  literal assertions, and the retail profile comparison must prove the
  consumed fields.
- If replay provenance does not isolate one intentional behavior change,
  rebaselining is forbidden.

These objections have concrete executable gates and do not require a scope
expansion. The design is self-approved under the autonomous goal contract for
the bounded PR-stabilization slice.
