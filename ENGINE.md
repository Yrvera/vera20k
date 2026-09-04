# VERA20k — Project contract

VERA20k is a from-scratch Rust replacement for Yuri's Revenge `gamemd.exe`.
**gamemd-native semantics, Rust-native architecture.** The intentional scale exception
is **20,000 units and 30 players**: replace native storage limits while preserving
deterministic behavior and the player experience.

This is the shared contract for Codex and Claude. Use engineering judgment within
these boundaries. Skills provide specialized help; they are not mandatory stages.
Specific user instructions take precedence over repository workflow defaults.

## Working with the user

Read for intent: the user often thinks aloud and uses approximate terminology.
Treat proposed causes and solutions as hypotheses; investigate observations that
contradict your analysis. Keep exact keys, addresses, values and parity claims literal.
Push back briefly with evidence when a proposal is unsound, then respect the decision.

Proceed through authorized work without routine confirmation. Resolve discoverable
questions and choose sound, reversible approaches. Ask only when missing authority
or a material, undiscoverable preference prevents a responsible decision. Research
or review alone does not authorize implementation. Honor stop instructions;
continuation must preserve the latest scope and exclusions.

Lead with the result. Prefer a few plain-language sentences; add detail when useful
or requested. Explain severity through its trigger and frequency in normal play.
GitHub issues use “Player-visible problem” and “Current Rust mismatch”; keep binary
details in supporting evidence unless requested.

## Behavior and evidence

Every implemented behavior must match established active-YR semantics exactly.
Priority follows player visibility and frequency; priority never proves equivalence.
Differences in mechanism, numeric results, ordering or visual/audio output are
**DRIFT** until equivalence is demonstrated. An exhaustive parity task remains open
while required behavior is missing or unproven.

Before changing simulation behavior, establish what gamemd does from native bodies
and active callers, directly or through a cited research document. Research details
are often wrong: recheck uncertain, conflicting or consequential claims against the
binary and retail data. Existing code, Ghidra labels and other engines are leads,
not proof. Do not invent offsets, identities, gates or fallbacks. Confirm active-YR
reachability; inherited Tiberian Sun code is not automatically in scope.

Keep these claims separate and name their evidence:

- **Native behavior established:** cited body/caller/data evidence supports the rule.
- **Rust regression tested:** a named check exercises the implementation.
- **Parity demonstrated:** a gamemd-derived executable comparison or exhaustive proof
  covers the stated behavior and input domain. A sample proves only its coverage.

Parity goldens come from binary execution/emulation, retail bytes or native capture.
Hand calculations and Rust-vs-prior-Rust fixtures are not parity oracles. Do not use
an unqualified “VERIFIED” or “complete” to blur these distinctions.

Put provenance near each cohesive gamemd-derived Rust behavior: native role, verified
function identity/address and source. Sim-behavior commits cite the native source or
research document. Internal rules without a proven equivalent say
“VERA-internal, gamemd equivalent UNCHECKED.” This discloses a gap, not equivalence.
Read the [Ghidra workflow](docs/research/ghidra-workflow.md) before binary work.

## Architecture and implementation freedom

Choose the simplest robust design for the complete mechanism. Refactor boundaries or
introduce abstractions when evidence and consumers justify them; fixed counts of
implementations, files or lines do not decide whether a design is appropriate.

- `sim/` owns deterministic gameplay and never depends on `render/`, `ui/`, `sidebar/`,
  `audio/` or `net/`. App code orchestrates without becoming a second gameplay owner.
- Preserve state authority, lifecycle, scheduler order, RNG consumption, timers,
  same-tick effects and persistence. Storage order and active-object order are distinct.
  Use Rust ownership and interfaces rather than copying C++ inheritance or globals.
- Preserve native numeric semantics: widths, signedness, truncation and operation
  order. Prefer existing fixed/integer types where suitable; document and validate
  floating-point behavior when native semantics require it.
- Name coordinate frames and units at conversions. Read the actual types and
  [coordinate reference](docs/research/coordinate-reference-frames.md); cell positions,
  subcell offsets, height, facing and screen coordinates are not interchangeable.
- Read retail INIs/assets before choosing data-driven constants. Check toolchain and
  architectural compatibility before adding or upgrading dependencies.
- Document module purpose/dependencies and non-obvious decisions near the code.
  Current module headers, owners and `advance_tick` phases describe the architecture.

## Deliver complete mechanisms

Start with the outcome, scope, constraints and evidence of completion. A short
explanation is enough for a small change; write a design/plan when it helps substantial
work. Implementation authorization includes routine design choices.

One owner follows a mechanism across research, code, production integration and review.
Delegate independent work with clear ownership. Inspect the surrounding loop and
consumers before fixing one stage; reassess if a fix worsens the symptom. Compilation
and disconnected unit tests do not prove delivery: validate the actual gameplay,
load or render path, using runtime reproduction when needed.

Promote a necessary foundation when a narrow patch would leave duplicate authority,
broken behavior or predictable rework. Implement its smallest coherent capability;
a separable foundation has its own dependency PR, merged before its consumer.
Record unrelated discoveries without absorbing their backlog. Residuals name their
trigger, effect, frequency and downstream risk. Do not defer a required part of the
selected loop or a determinism/authority/lifecycle defect and call that loop closed.

For substantial or risky changes, use a fresh read-only critic who can inspect original
evidence and challenge scope, design, tests and production reachability. Resolve confirmed
findings; reject false positives with evidence. The builder's packet does not limit the
critic's inquiry. See [review guidance](.agents/skills/_shared/review.md).

An authorized multi-mechanism goal continues through coherent transactions; finishing
one is not an automatic stop. Honor the requested boundary and stopping condition.
Keep only the current checkpoint for sustained work; use the
[handoff guidance](.agents/skills/_shared/handoff.md) when handing off or interrupted.

## Ownership, Git and validation

Reconcile Git status, branches/worktrees, recent commits and relevant processes before
mutating. Never edit, clean, stash or stop another task's work. A failure in an untouched
file does not establish its cause; check consumers, environment and ownership.

Start new work on `feature/<topic>` from freshly fetched `origin/main`. Use an isolated
worktree when the checkout is owned or dirty; continue an existing task-owned branch.
Commit coherent validated increments. Never commit/push directly to `main`. Publication
requires user/goal authorization; PRs target `main`. When authorized, integrate accepted
transactions promptly. The owner resolves conflicts and revalidates affected behavior.
Preserve unique/local-only data during cleanup; use `sync` for complex cleanup.

Use validation appropriate to the change:

- During Rust work: `cargo check -p vera20k` as needed and focused
  `cargo test -p vera20k --lib <module_path>::`.
- Before a Rust PR is ready: one full `cargo test -p vera20k --lib` for the final candidate.
  Repeat only when later changes, conflicts or failures invalidate that result.
- Docs/skills only: check content, links, examples and relevant tooling; no Cargo suite.
- Report literal `test result:` output. Every `cargo test` uses `--lib`.

Before every Cargo command run `Get-Process cargo,rustc -ErrorAction SilentlyContinue`;
wait if another session owns Cargo. Never start competing Cargo or kill an active compile.
Confirm local config/assets in a fresh worktree before attributing failures to code.
Format only edited leaf files (`rustfmt --edition 2024 <file>`), never crate-wide or
through a recursive `mod.rs`. Coordinate snapshot versions and golden rebaselines;
do not absorb another session's unmerged changes into a new baseline.

## Finding knowledge and maintaining guidance

Use current source and `research-index` for focused discovery; ranked search is not an
exhaustive inventory. Research/plans are tracked: read and write them in the task's own
worktree. `<main-checkout>` is the primary checkout from `git worktree list`; its
`ini/`, retail configuration and index cache are machine-local. Consult `LOCAL.md`
there when needed. Verify index provenance before relying on another worktree's results.

YR loads standalone `RULESMD.INI`/`ARTMD.INI`/`AIMD.INI`, then applicable language,
mode and map overrides; it does not merge base RA2 INIs underneath. Use repository
retail data and the `asset` CLI / `asset-browser`. A successful parse or plausible
palette render is not proof of correctness.

Requested research documents are valid deliverables without accompanying code. Avoid
unsolicited reports and permanent hand-maintained completion ledgers; current evidence,
code and named checks determine status. Update [System Map](docs/system-map/) only for
verified connections touched by the task, then run
`python -m tools.system_map check --require-sources`. Check tracking with Git rather
than copying `.gitignore` into prose.

Edit shared skills in `.agents/skills/`; generate Claude copies with
`python tools/skill_sync.py --write` and verify with `python tools/skill_sync.py --check`.
Keep guidance that changes decisions or preserves non-obvious knowledge. Put conditional
detail where it is used; remove superseded rules instead of adding another exception.