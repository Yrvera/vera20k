# Machine-Derived Parity Ledger MVP Design

## Goal

Build a tracked, deterministic parity-obligation ledger whose status is derived from current
source, git, named checks, and machine-generated evidence rather than hand-maintained roadmap
claims.

## Architecture Context

VERA20k currently has three different classes of project evidence:

1. Current Rust, git history, tests, and CI are tracked and executable. They are the authority for
   whether a change is present and whether Rust regression checks pass.
2. The roadmap, gap-scan, and research corpus is locally valuable but intentionally ignored by git
   (`.gitignore:24-25`). It can enumerate obligations and provide research evidence, but it cannot
   serve as a durable status database.
3. Small tracked oracle infrastructure exists, notably `src/bin/bridge-oracle-compare.rs` and
   `tools/bridge_oracle/`. Its PASS/FAIL/UNCHECKED model is useful, but the checked-in sample files
   are diagnostic fixtures rather than captured gamemd proof.

The research index remains a navigation aid only. A live `research_validate` preflight on
2026-07-10 checked 2,770 documents and returned invalid: 11 checksum mismatches, 20 reported
missing links at the configured output limit, and 887 stale/unknown documents. Its current metadata
logic also infers `verified` from source kind or filename
(`tools/research_index/research_index/metadata.py:145-174`), and its default indexed roots omit gap
scans. Neither behavior is sufficient for a parity-status authority.

The current global parity harness and replay fixtures are Rust regression ratchets. Their own
documentation says they compare replayed Rust hashes with committed Rust baselines
(`src/sim/world/global_parity_harness_tests.rs:1-11`, `src/sim/replay.rs:16-19`). They are valuable
regression evidence, but they cannot certify gamemd parity.

The existing GitHub workflow runs Cargo check and tests on pull requests only
(`.github/workflows/rust.yml:1-18`). There is no ledger consistency check or nightly oracle ratchet.

## Impact Analysis

### New tracked surfaces

- `parity/schemas/`: versioned obligation, source-lock, evidence, and generated-row schemas.
- `parity/sources/`: generated source fingerprints and importer metadata.
- `parity/obligations/`: generated normalized obligations for the bootstrap source set.
- `parity/evidence/`: typed evidence declarations and artifact provenance. These records never
  contain a hand-authored result or completion status.
- `tools/parity_ledger/`: a Python standard-library importer, validator, reducer, renderer, and
  focused test suite.
- `.github/workflows/rust.yml`: lightweight Python unit-test and ledger-consistency steps.

`parity/` and `tools/parity_ledger/` are not ignored, so this design does not change `.gitignore`.

### Existing surfaces read by the tool

- Core TODO and scheduler roadmap:
  - `docs/plans/2026-05-29-core-engine-substrate-todo.md`
  - `docs/plans/2026-05-28-foundational-scheduler-roadmap-todo.md`
- Miner inventory and assignment source:
  - `docs/gap-scans/2026-07-02-disparity-scan-miner.md`
  - `docs/plans/2026-07-02-miner-parity-roadmap.md`
- Shell inventory and assignment source:
  - `docs/gap-scans/2026-07-06-disparity-scan-shell-ui.md`
  - `docs/plans/2026-07-06-shell-ui-parity-roadmap.md`
- Current Rust paths, test declarations, git ancestry, and optional evidence artifacts.

### Blast radius

- No gameplay, rendering, input, simulation, snapshot, world-hash, or asset behavior changes.
- No Cargo dependency or profile changes.
- No Ghidra project mutation, runtime injection, retail-binary launch, logger DLL, pixel capture, or
  nightly automation.
- Generated dashboard files live only under `target/parity-ledger/` and are disposable.
- The importer may rewrite only its own generated records under `parity/sources/` and
  `parity/obligations/`.

### Primary risks

- Ignored local source documents are unavailable in a clean CI checkout.
- A naive parser could silently miss multiline findings or trust incorrect document totals.
- A single `done` field would conflate implementation presence, regression health, and gamemd proof.
- Existing Rust test names do not uniformly encode finding IDs or evidence provenance.
- A sampled oracle can be accidentally promoted to exhaustive proof.
- The existing bridge comparator prints FAIL counts but returns success after comparison
  (`src/bin/bridge-oracle-compare.rs:70-100`); exit status alone is therefore not valid evidence.

The design addresses these risks with tracked normalized imports, exact-key validation, independent
status axes, conservative verdict derivation, source fingerprints, and deterministic regeneration.

## Chosen Approach

Use a tracked JSON obligation/evidence corpus with a standard-library Python importer and reducer.

The ignored Markdown corpus is an import/provenance source, not the durable ledger. The import step
extracts actual records, fingerprints the complete source files, and writes normalized tracked JSON
without status fields. A clean CI checkout validates those normalized records without requiring the
ignored source documents. A developer checkout can additionally re-open the source documents and
require their fingerprints to match.

The reducer never trusts prose completion labels, checkbox state, commit-message claims, file names,
or a command's zero exit status as parity proof. It derives a multi-axis row from the evidence that
is actually available at the current HEAD.

### Approved Planning Amendment (2026-07-10)

Direct source enumeration performed before implementation found that the roadmap formats do not
support one flat assignment per finding. A finding can have a primary workstream plus related
quick-win, parent, research-gate, deferred, or historical-partial mentions. The approved v1 model
therefore stores one optional `primary` assignment plus a deterministic list of typed `related`
mentions. Only multiple unresolved `primary` assignments are fatal.

The same enumeration corrected the bootstrap inventory to 277 active obligations: 32 core, 17
scheduler, 139 miner, and 89 shell. `miner:M32` is also literally unassigned before the miner
roadmap's excluded Status section, so the required initial unmapped set has seven members rather
than six. Shell `L3` and `L25` are merge tombstones and `L28` is a proven-non-gap tombstone; none is
an active obligation.

The MVP deliberately leaves machine-result states unreachable when it has no machine result:
`test_declared` can derive only `DECLARED`, artifact hashing proves integrity only, and neither
`regression PASS` nor parity `VERIFIED` can be produced until a later typed execution receipt or
machine-interpreted comparator/proof is added. Missing current anchors render `STALE_MAPPING`
instead of making the whole corpus invalid. Multi-file imports use a shared corpus digest and write
the source-lock commit marker last. `check --ci` always treats `ignored-local` sources as unavailable
even when they happen to exist on a runner, making CI results independent of checkout accidents.

## Tiny-Detail Ledger

The implementation must preserve all of these constraints:

- **Namespaced identity:** finding IDs are stored as `miner:G1`, `miner:L7`, `shell:H1`, etc.; bare
  IDs never cross system boundaries. [source: miner and shell gap-scan ID families]
- **Core identity:** checkbox obligations without a stable local ID receive a deterministic ID from
  the source namespace, heading path, normalized obligation text, and a short content hash. Text
  changes create an explicit old/new difference instead of silently retaining identity.
- **Actual-record counting:** importers enumerate finding records; document header totals and range
  prose are diagnostics only. The miner roadmap's `140` claim, for example, is not an input to the
  generated count. [source: miner roadmap:3-6]
- **Mapping separation:** obligation discovery and roadmap assignment are separate passes. An absent
  assignment produces `UNASSIGNED`; it never drops the obligation.
- **Known omission coverage:** `miner:L7`, `miner:L34`, `miner:L35`, `miner:L43`, `miner:M32`,
  `shell:H1`, and `shell:H19` must exist in the bootstrap output even though their literal IDs are
  absent from the roadmaps' pre-Status primary mappings. [source: miner scan:454,484-485,546-549,
  565-566; miner roadmap:14,174-177; shell scan:203-211,348-355]
- **Assignment roles:** each finding has zero or one resolved `primary` workstream plus sorted typed
  `related` mentions (`parent`, `quick_win`, `research_gate`, `deferred`, or
  `historical_partial`). Multiple related mentions are valid; multiple unresolved primaries are a
  fatal error.
- **Shell tombstones:** `shell:L3` merges into `shell:M13` and `shell:L2`; `shell:L25` merges into
  `shell:L24`; `shell:L28` is retired as a proven non-gap. These are source dispositions, not active
  obligations. [source: shell scan:25-30,740-748]
- **No hand status:** obligation and evidence inputs reject `status`, `done`, `complete`, `landed`,
  `pass`, and `verified` result fields. Roadmap checkbox marks and status sections are not imported.
- **Source provenance:** every imported record carries source path, local ID or deterministic source
  key, whole-file SHA-256, importer name/version, and tracking mode (`tracked` or `ignored-local`).
- **Unavailable versus stale:** missing ignored sources in CI are `SOURCE_UNAVAILABLE`; a present
  source with a mismatched fingerprint is `SOURCE_STALE`. They are never conflated.
- **Current git ancestry:** a commit can support implementation presence only if it is an ancestor
  of HEAD. A subject mentioning a finding ID is not sufficient by itself.
- **Current anchors:** declared Rust paths, symbols, tests, and artifacts are checked against HEAD.
  A syntactically valid declaration whose current anchor disappeared produces `STALE_MAPPING` and a
  visible row diagnostic; malformed or unsafe declarations remain fatal schema errors.
- **Regression is not parity:** Rust-only tests and replay hashes can produce regression states, but
  their highest parity verdict is `UNVERIFIED`. [source: AGENTS.md parity certification rules]
- **Verified gate:** `VERIFIED` requires a machine-interpreted gamemd/retail comparison or exhaustive
  proof over the declared domain. The MVP defines the state but cannot emit it because it does not
  ingest an executed comparator/proof receipt.
- **Sampled coverage:** sampled vectors and traces retain `SAMPLED` coverage and cannot promote a row
  to `VERIFIED` in this MVP.
- **Activation proof:** gamemd evidence must identify the executable hash, scenario/mode, active-YR
  trigger, relevant inputs, tool/schema version, and artifact hash. Missing activation proof is
  `UNCHECKED`.
- **Reference self-agreement:** match-level evidence must carry typed `reference_runs` proving two
  agreeing gamemd runs for the compared ticks; disagreement makes the reference undefined rather
  than a Rust failure. V1 declarations without such a machine receipt remain incomplete.
- **Missing fields:** absent required evidence is `UNCHECKED`, following the bridge schema's existing
  rule (`tools/bridge_oracle/schema.md:17-21`).
- **Bridge exit status:** the reducer may not treat the current bridge comparator's zero process exit
  as PASS. Because the MVP does not ingest a typed comparator receipt, bridge declarations remain
  incomplete regardless of process exit.
- **Dependencies:** dependency IDs must resolve and the graph must be acyclic. Blocking is derived
  from unresolved prerequisite rows; it is not a manually editable status.
- **Deferral semantics:** user deferrals affect queue order only. They never alter parity verdict.
- **Invalid research index:** index validation failures can mark linked research navigation stale,
  but can neither remove obligations nor promote/demote executable evidence.
- **Canonical bytes:** generated JSON sorts records and object keys, uses UTF-8 and `\n`, contains no
  wall-clock timestamp or machine-specific absolute path, and ends with one newline.
- **Strict decoding:** invalid UTF-8 or malformed finding structure is fatal; the importer must not
  replace bytes and risk changing stable IDs silently.
- **Atomic output:** all generated payloads are validated in memory and written to temporary
  siblings first. Obligations/evidence are replaced before the source-lock commit marker; a shared
  `corpus_digest` lets `check` detect any interrupted multi-file replacement on Windows.
- **Coverage honesty:** the dashboard is `BOOTSTRAP_PROVISIONAL` until native function/data closure
  enumeration is imported. It cannot display a project-wide completion percentage as certified.
- **Scope boundary:** no runtime capture, logger, Ghidra closure, pixel oracle, nightly workflow, or
  behavioral fix is part of this MVP.

## Design

### Components

#### 1. CLI package

`python -m tools.parity_ledger` exposes three commands:

- `import --source-set bootstrap`
  - Reads the six configured local Markdown sources.
  - Extracts obligations and assignments with source-specific adapters.
  - Validates the complete import in memory.
  - Atomically writes `parity/sources/bootstrap.json` and normalized files under
    `parity/obligations/`.
- `check [--require-sources] [--ci]`
  - Validates schema versions, exact keys, identities, mappings, relations, dependency graph, source
    locks, git ancestry, current anchors, and evidence provenance.
  - `--require-sources` makes missing or stale ignored sources fatal for a developer checkout.
  - `--ci` accepts unavailable ignored sources but still reports `BOOTSTRAP_PROVISIONAL`; every
    tracked invariant remains fatal.
- `render [--output target/parity-ledger]`
  - Runs `check` first.
  - Derives canonical rows and writes `ledger.json` plus `summary.md`.
  - Never mutates tracked inputs.

The CLI uses `argparse`, `dataclasses`, `hashlib`, `json`, `pathlib`, `re`, `subprocess`, and
`unittest` only. It does not execute raw shell strings from a manifest.

#### 2. Source adapters

Each adapter owns one format rather than one universal Markdown regex:

- Core adapter: heading path plus checkbox/bullet obligation extraction; checkbox state ignored.
- Miner scan adapter: `G`, `M`, `L`, and `S` record extraction, including multiline titles.
- Miner roadmap adapter: bounded W0-through-Deferred mention extraction; the later prose Status
  section is excluded. W12 mentions are research gates, `half advanced` W1 mentions are historical
  partials, and `object-AI S5 slice` is not parsed as the slave-miner `S5` finding.
- Shell scan adapter: `H`, `M`, and active `L` record extraction plus generated dispositions for the
  merged `L3`/`L25` and retired-non-gap `L28` tombstones.
- Shell roadmap adapter: workstream `Scope (closes)`, quick-win, research-first, and deferred mention
  extraction with explicit roles. Quick wins can be the primary leaf assignment while the broader
  workstream remains related. Mere mention elsewhere in the file does not count as assignment.
- Scheduler adapter: contract/implementation/research obligations; checkbox state ignored.

Every adapter returns typed intermediate records plus diagnostics: duplicate IDs, declared-count
differences, malformed headings, and unmapped findings.

#### 3. Tracked record schemas

All records use `schema_version: 1` and reject unknown keys.

`parity/sources/bootstrap.json`, `parity/obligations/bootstrap.json`, and
`parity/evidence/bootstrap.json` carry the same `corpus_digest`, computed from the complete canonical
semantic payload: schema/source-set identity, importer identity/version, source locks, obligations,
dispositions, diagnostics, and evidence. The source-lock document is the commit marker and is
replaced last. A digest mismatch is fatal even when ignored sources are unavailable.

An obligation contains assertions only:

```json
{
  "schema_version": 1,
  "id": "miner:L7",
  "system": "miner",
  "kind": "parity_gap",
  "title": "First-cell harvest gate uses the wrong rate source",
  "source": {
    "path": "docs/gap-scans/2026-07-02-disparity-scan-miner.md",
    "local_id": "L7",
    "sha256": "...",
    "tracking": "ignored-local",
    "importer": "miner-scan",
    "importer_version": 1
  },
  "source_claims": {
    "severity": "low",
    "activation": "active_or_conditional",
    "player_frequency": "unknown",
    "determinism_impact": "sim_critical"
  },
  "assignment": {
    "primary": null,
    "related": []
  },
  "dependencies": [],
  "relations": [],
  "rust_anchors": []
}
```

`source_claims` remain attributed claims; they are not derived status.

The obligation-set document also has a `dispositions` array for source IDs that are not active
obligations. Each disposition contains `source_id`, `kind` (`merged` or `retired_non_gap`), sorted
`targets`, and source provenance. This represents shell L3/L25/L28 without inflating the active
obligation count.

An evidence declaration identifies how a claim could be checked. It contains no result:

```json
{
  "schema_version": 1,
  "id": "evidence:miner-L7-vector",
  "obligations": ["miner:L7"],
  "kind": "gamemd_vector",
  "artifact": {"path": "parity/evidence/artifacts/...", "sha256": "..."},
  "provenance": {
    "executable_sha256": "...",
    "tool": "...",
    "tool_version": "...",
    "scenario": "...",
    "activation_proof": "...",
    "reference_runs": []
  },
  "coverage": {"mode": "sampled", "domain": "..."},
  "check": {"type": "artifact_hash"}
}
```

Only a fixed enum of check types is accepted. The MVP validates `artifact_hash`, `git_ancestor`,
`path_exists`, `test_declared`, and the structure/provenance of bridge trace declarations. It does
not execute arbitrary commands, interpret an oracle result, or claim an unexecuted test passed.
`reference_runs` is present in the provenance contract for later machine receipts, but declarations
alone cannot satisfy it.

#### 4. Reducer

For each obligation the reducer derives independent fields:

- `source_state`: `CURRENT`, `STALE`, or `UNAVAILABLE`.
- `assignment_state`: `ASSIGNED`, `UNASSIGNED`, `ALIASED`, `SUPERSEDED`, or invalid duplicate
  primary; related assignment mentions never create a duplicate-primary failure.
- `implementation_state`: `NONE`, `CANDIDATE`, `LANDED`, or `STALE_MAPPING`.
- `regression_state`: `NONE`, `DECLARED`, `PASS`, or `FAIL`; `PASS`/`FAIL` are reserved and
  unreachable in this MVP because it has no typed execution receipt.
- `oracle_state`: `NONE`, `INCOMPLETE`, `SAMPLED`, or `EXHAUSTIVE`.
- `parity_verdict`: `DRIFT`, `UNCHECKED`, `UNVERIFIED`, or `VERIFIED`; `VERIFIED` is reserved and
  unreachable in this MVP.
- `queue_state`: derived next action such as `NEEDS_ASSIGNMENT`, `NEEDS_RESEARCH`,
  `NEEDS_IMPLEMENTATION`, `NEEDS_REGRESSION`, `NEEDS_ORACLE`, or `DEPENDENCY_BLOCKED`.

The reducer is conservative:

- A still-current confirmed gap with no implementation evidence remains `DRIFT`.
- Current implementation evidence without an executable gamemd oracle becomes `UNVERIFIED`.
- Incomplete provenance or ambiguous source state becomes `UNCHECKED` where it prevents a stronger
  judgment.
- `VERIFIED` is impossible in v1 from source existence, commit messages, checkboxes, Rust-only
  tests, artifact hashes, declared coverage, or sampled evidence.

The top-level report exposes counts for every independent axis and sets
`coverage_state = BOOTSTRAP_PROVISIONAL`.

#### 5. Renderer

The JSON renderer is the canonical machine output. The Markdown renderer is a projection sorted by:

1. queue blocking/unassignment,
2. controlled source-claimed player frequency (`high`, `medium`, `low`, or `unknown`),
3. source-claimed determinism impact,
4. research/evidence readiness,
5. namespaced ID.

Importers set a claim to `unknown` rather than inferring it from unconstrained prose.

The summary includes a dedicated unmapped section, source-staleness diagnostics, and the reason no
row is `VERIFIED`. It never substitutes one aggregate `percent complete` number for the status axes.

### Interfaces / Contracts

- Repository root is discovered from the nearest parent containing `Cargo.toml` and `.git`; no
  absolute workspace path is serialized.
- All stored paths use `/` separators and are repository-relative.
- Importers are pure functions from UTF-8 text plus source metadata to typed records and diagnostics.
- Reducer input is the validated tracked corpus plus read-only current workspace/git facts.
- Renderer input is only reduced rows; it cannot see prose documents or invent status.
- Manifest check definitions are a closed enum, never shell command strings.
- Schema-version mismatch is fatal and requires an explicit migration.

### Data Flow

```text
ignored local docs ----> source-specific importers ----> validated normalized obligations
                              |                                      |
                              +---- source SHA-256 locks ------------+

tracked obligations + tracked evidence + current git/code/test declarations
                              |
                              v
                       deterministic reducer
                              |
                              +----> target/parity-ledger/ledger.json
                              +----> target/parity-ledger/summary.md
```

CI runs the Python unit tests and `check --ci` before or alongside the existing Cargo jobs. It does
not regenerate tracked imports or inspect `ignored-local` source files, even if they unexpectedly
exist on the runner, and does not run external capture tools.

### Error Handling

Fatal validation errors return a nonzero exit and print the exact record ID, field, and source path:

- unsupported schema version or unknown key,
- invalid/nonnamespaced ID,
- duplicate obligation or primary assignment,
- unresolved alias/dependency or dependency cycle,
- malformed source record,
- evidence with missing required provenance,
- forbidden result/status field,
- noncanonical or nondeterministic generated output.

A well-formed evidence declaration whose once-current path/test/artifact no longer exists is a
nonfatal `STALE_MAPPING` row diagnostic. Unsafe paths, invalid hashes, and malformed evidence remain
fatal.

Source absence is mode-sensitive:

- `check --require-sources`: absent or fingerprint-mismatched source is fatal.
- `check --ci`: every `ignored-local` source is deterministically treated as unavailable; a tracked
  record inconsistency remains fatal.

Ordinary open work is never a process error. `DRIFT`, `UNVERIFIED`, `UNCHECKED`, unassigned work,
and unmet dependencies are ledger contents, not validator failures.

### Testing Strategy

#### Unit fixtures

- Namespaces prevent cross-system `L7` collisions.
- Multiline miner/shell findings preserve exact ID/title association.
- Checkbox state and roadmap status prose are ignored.
- Duplicate unresolved primary mappings fail; typed parent/quick-win/research/deferred/historical
  mentions remain sorted related assignments.
- Missing assignment preserves the obligation as `UNASSIGNED`.
- Forbidden status/result keys fail validation.
- Changed source bytes produce `STALE`; absent ignored source produces `UNAVAILABLE`.
- Git evidence from a non-ancestor commit cannot become `LANDED`.
- Missing current Rust paths/tests produce `STALE_MAPPING` without invalidating the corpus.
- Dependency cycles fail with the cycle path.
- Rust-only evidence never produces `VERIFIED`.
- Sampled gamemd vectors never produce exhaustive proof.
- Missing executable hash or activation proof produces `UNCHECKED`.
- Artifact hashes and declared exhaustive coverage cannot produce `VERIFIED` without a
  machine-interpreted result receipt.
- Regression `PASS`/`FAIL` and parity `VERIFIED` remain unreachable in the MVP.
- Bridge comparator exit zero with nonzero FAIL/UNCHECKED counts is not accepted as PASS.
- Two renders of the same inputs are byte-identical and timestamp-free.

#### Repository acceptance tests

- The bootstrap import contains 277 active obligations: 32 core, 17 scheduler, 139 miner, and 89
  shell.
- `miner:L7`, `miner:L34`, `miner:L35`, `miner:L43`, `miner:M32`, `shell:H1`, and `shell:H19` are
  present and unassigned.
- `shell:L3`, `shell:L25`, and `shell:L28` exist only as generated dispositions, not active
  obligations.
- Roadmap omissions appear in the unmapped section instead of disappearing.
- Source-declared totals are compared with computed totals and reported when inconsistent.
- The generated report is `BOOTSTRAP_PROVISIONAL`.
- No initial Rust-only evidence row is reported `VERIFIED`.
- Multiple typed related workstreams do not trigger duplicate-primary failure.
- `check --ci` succeeds without ignored docs after the normalized corpus is tracked.
- `check --require-sources` succeeds in the full developer workspace at the source fingerprints used
  for the import.

#### Verification commands for the implementation slice

```powershell
python -m unittest discover -s tools/parity_ledger/tests
python -m tools.parity_ledger import --source-set bootstrap
python -m tools.parity_ledger check --require-sources
python -m tools.parity_ledger render --output target/parity-ledger
python -m tools.parity_ledger check --ci
git diff --check
```

The implementation will inspect generated diffs and run existing Cargo checks only if it touches an
existing Rust surface. The selected Python design should not require a Cargo build.

## Architectural Decisions

- **Tracked normalized imports, not tracked prose:** preserves CI reproducibility without changing
  the repository's intentional docs-ignore policy.
- **Multi-axis status, not a `done` enum:** prevents code presence or Rust regression health from
  masquerading as parity proof.
- **Python standard library:** keeps backlog validation fast, cross-platform, dependency-free, and
  independent of the large game build.
- **Source-specific adapters:** avoids a permissive universal regex silently losing different
  Markdown record shapes.
- **Primary plus related assignment roles:** preserves roadmap hierarchy and research/deferred
  context without manufacturing duplicate ownership.
- **Strict schemas and unknown-key rejection:** makes format evolution explicit.
- **No arbitrary manifest commands:** prevents the evidence corpus from becoming a CI command
  execution surface.
- **No committed generated dashboard:** canonical tracked inputs stay reviewable; disposable views
  are always regenerated.
- **Provisional closure:** the MVP refuses to imply full-project coverage before Ghidra/static and
  runtime coverage enumeration exists.
- **No premature machine-result states:** `PASS`, `FAIL`, `EXHAUSTIVE`, and `VERIFIED` stay reserved
  until a later typed execution/comparator receipt can justify them.

No runtime architecture, gameplay semantics, or simulation ownership boundary is changed.

## Alternatives Considered

### Typed Rust ledger binary

This could reuse `serde`, `serde_json`, `toml`, and `anyhow`, and would provide strong compile-time
types. It was rejected for the MVP because a lightweight backlog check would become coupled to the
large Cargo package and its platform dependencies, slowing import/CI iteration without improving
evidence quality.

### Research-index extension

This was rejected as the durable authority. The index is currently invalid, intentionally ignored,
does not index the needed gap scans by default, and infers verification-like metadata from filenames
and source kinds. A later read-only adapter may use a valid index for navigation hints, but index
status can never drive parity verdicts.

### Hand-maintained TOML/Markdown tracker

This was rejected because it recreates the exact failure mode being solved: humans would edit
completion fields independently of code, git, tests, and oracle evidence.

### Generate directly from ignored docs on every CI run

This was rejected because clean CI checkouts do not contain the ignored corpus. Tracking normalized,
fingerprinted imports keeps the check reproducible while allowing developer workspaces to detect
source drift.
