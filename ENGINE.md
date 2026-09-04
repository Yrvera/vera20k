# VERA20k — Project contract

A Rust replacement for Yuri's Revenge `gamemd.exe`: **gamemd-native semantics,
Rust-native architecture**. The intentional scale exception is **20,000 units,
30 players**; replace native storage limits while preserving deterministic behavior.

This contract governs Codex and Claude. Use engineering judgment; skills are optional
specialized help. Specific user instructions override workflow defaults.

## Intent and autonomy

The user often thinks aloud. Follow intent, keep exact technical values literal,
and treat proposed causes as hypotheses. Investigate contradictory observations.
Push back briefly with evidence; respect informed decisions.

Complete authorized work without routine approval. Resolve discoverable questions;
ask only for missing authority or consequential, undiscoverable preferences.
Research/review alone does not authorize implementation. Preserve scope amendments
and stop instructions across continuations.

Be brief, plain and result-first. Explain severity through trigger frequency.
GitHub issues use “Player-visible problem” and “Current Rust mismatch”.

## Exactness and evidence

Establish native behavior before sim changes from bodies and active callers,
directly or through cited research. Recheck uncertain, conflicting or consequential
claims against the binary/retail data. Research, labels and other engines can be
wrong. Confirm active-YR reachability; never invent offsets, identities or behavior.

Priority follows player visibility and frequency; it does not establish equivalence.
Behavior/output differences remain **DRIFT** until equivalence is demonstrated.
Missing or unproven required behavior keeps an exhaustive task open.

Distinguish and cite:

- **Native behavior established:** body/caller/data evidence.
- **Rust regression tested:** named implementation checks.
- **Parity demonstrated:** gamemd-derived executable comparison or exhaustive proof,
  bounded by stated coverage. A sample cannot certify the whole mechanism.

Parity goldens come from native execution/emulation, capture or retail bytes,
not hand calculations or prior Rust. Avoid unqualified “VERIFIED”/“complete”.

Each cohesive gamemd-derived Rust behavior carries nearby native identity/address
and source; sim-behavior commits cite their evidence. Unproven internal rules say
“VERA-internal, gamemd equivalent UNCHECKED”; the label does not prove equivalence.
Read the [Ghidra reference](docs/research/ghidra-workflow.md) before binary work.

## Architecture and delivery

Choose boundaries and abstractions by responsibility and consumers, not line/type
counts or C++ structure. Preserve state authority, lifecycle, scheduler/RNG order,
timers, same-tick effects, persistence and exact numeric semantics. Document and
validate floating-point use where native behavior requires it. Storage order and
active-object order are distinct.

`sim/` never depends on `render/`, `ui/`, `sidebar/`, `audio/` or `net/`.
App code orchestrates without owning duplicate gameplay. Current module contracts
and `advance_tick` phases describe the architecture. Name coordinate frames/units;
consult the [coordinate reference](docs/research/coordinate-reference-frames.md).

One owner follows a complete mechanism through evidence, implementation, production
integration and review. Inspect the surrounding loop/consumers; validate the actual
production path, using runtime reproduction when needed. Reassess worsening fixes.
Design/plan artifacts are optional; implementation authority includes design choices.

Promote coherent prerequisites when a smaller patch creates broken behavior, duplicate
authority or predictable rework. A separable foundation has its own PR, merged before
its consumer. Record adjacent findings without absorbing their backlog. Residuals name
trigger, effect, frequency and downstream risk; deferring required loop or
determinism/authority/lifecycle work cannot close that loop.

Delegate independent work with clear ownership. Substantial/risky changes need a fresh
read-only [critic](.agents/skills/_shared/review.md) free to inspect original evidence
and challenge scope/design. Resolve confirmed findings, reject false positives with
evidence. Continue authorized multi-mechanism goals through coherent transactions.
Keep a concise [checkpoint](.agents/skills/_shared/handoff.md) for sustained work.

## Git and validation

Check actual Git/worktree/process state before mutating. Never alter another task's
files, refs or processes; untouched-file failures require causal investigation.

Start `feature/<topic>` from fetched `origin/main`; isolate owned/dirty checkouts.
Continue task-owned branches and commit validated increments. Never commit/push
directly to `main`. Publication requires user/goal authority; PRs target `main`.
Integrate promptly when authorized. Owners resolve conflicts and revalidate.
Preserve unique/local data; use `sync` for complex cleanup.

- Working Rust: `cargo check -p vera20k` as needed; focused
  `cargo test -p vera20k --lib <module_path>::`.
- Rust PR readiness: one full `cargo test -p vera20k --lib` for the final candidate;
  repeat only if later changes/failures invalidate it.
- Docs/skills: validate content, links/examples and tooling; no Cargo suite.
- Every `cargo test` uses `--lib`; report literal `test result:` output.

Before Cargo: `Get-Process cargo,rustc -ErrorAction SilentlyContinue`. Wait for other
owners; never compete or kill a compile. Confirm fresh-worktree config/assets.
Format edited leaf files only (`rustfmt --edition 2024 <file>`), never crate-wide
or recursive `mod.rs`. Coordinate snapshot versions/rebaselines; exclude others' WIP.

## Knowledge and guidance

Use source and `research-index`; ranked results are not exhaustive. Verify index
worktree provenance. Tracked research/plans belong in the task checkout; requested
research documents need no accompanying code. Avoid unsolicited reports or permanent
completion ledgers. Update [System Map](docs/system-map/) only for touched, verified
connections, then `python -m tools.system_map check --require-sources`.

Resolve `<main-checkout>` with `git worktree list`; its `ini/`, config, index cache
and `LOCAL.md` are machine-local. Read retail data before selecting constants.
YR loads standalone `RULESMD.INI`/`ARTMD.INI`/`AIMD.INI`, then applicable language,
mode and map overrides—no underlying RA2 INI merge. Use `asset`/`asset-browser`;
a successful parse or plausible render is not correctness proof.

Check compatibility before dependency changes; document non-obvious decisions near
their owner. Edit skills in `.agents/skills/`; generate Claude copies with
`python tools/skill_sync.py --write`, verify with `--check`. Keep conditional detail
where needed; remove superseded rules.