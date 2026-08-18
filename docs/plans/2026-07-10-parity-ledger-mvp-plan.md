# Machine-Derived Parity Ledger MVP Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Build a tracked, deterministic parity-obligation ledger whose status is derived from
current source, git, named checks, and machine-generated evidence rather than hand-maintained
roadmap claims.

**Architecture:** A standard-library Python package imports six ignored local Markdown sources into
three tracked, canonical JSON bundles: source locks, obligations, and evidence declarations. A
strict reducer derives independent source, assignment, implementation, regression, oracle, parity,
and queue axes, while generated JSON/Markdown projections remain disposable under `target/`.

**Design Doc:** `docs/plans/2026-07-10-parity-ledger-mvp-design.md`

---

## Grounding Summary

- The research-index brief for this tooling topic returned zero directly matching documents and
  unrelated implementation handoffs. The earlier global validation remains invalid (2,770 docs,
  11 checksum mismatches, 20 displayed missing links, 887 stale/unknown), so the index is not a
  status authority.
- No gamemd behavior, binary field, INI key, or retail asset is implemented by this plan. Live
  Ghidra verification and INI inspection are therefore not applicable to the ledger tool itself.
- Direct source enumeration establishes the bootstrap inventory: 32 core checklist obligations,
  17 scheduler checklist obligations, 139 miner findings, and 89 shell findings: 277 active rows.
- The miner roadmap's `140` claim is arithmetically inconsistent with its own ranges
  (`19 + 33 + 67 + 20 = 139`). Treat it as a visible nonfatal source diagnostic.
- Seven active findings are literally unassigned before excluded roadmap Status prose:
  `miner:L7`, `miner:L34`, `miner:L35`, `miner:L43`, `miner:M32`, `shell:H1`, and `shell:H19`.
- Shell `L3` and `L25` are merge tombstones; `L28` is a proven-non-gap tombstone. They are source
  dispositions, not active obligations, preserving the shell total of 89.
- Roadmaps contain hierarchical/secondary mentions. The schema must preserve one optional primary
  assignment plus typed related mentions rather than treating every textual ID as a primary owner.
- Current tracked precedent is `tools/bridge_oracle/` plus `src/bin/bridge-oracle-compare.rs`, whose
  missing-field result is UNCHECKED. Its process exit cannot certify PASS because FAIL counts still
  return success.
- The private sibling paths `vera20k-oracle:tools/oracle_harness/` and `vera20k-oracle:tools/dxgi_capture/` belonged to another session during the MVP. They are
  read-only inspiration and must not be imported, edited, or assumed present by the ledger.
- Local Python is 3.13.7. CI will explicitly install Python 3.13 with `actions/setup-python@v6`
  rather than relying on runner defaults.
- No Rust or simulation file changes are planned, so Cargo, snapshot, hash, tick-order, and fixed-
  point concerns are out of scope.

## Key Technical Decisions

- **Tracked normalized corpus, ignored prose inputs:** clean CI validates tracked JSON; developer
  imports additionally verify ignored source hashes. — **Confidence: high**
  - **Source:** approved design; `.gitignore:24-25`
- **One primary plus typed related assignments:** quick-win, parent, research-gate, deferred, and
  historical-partial mentions remain visible without manufacturing duplicate ownership.
  — **Confidence: high**
  - **Source:** miner roadmap W1/W2/W12; shell roadmap WS/QW/research/deferred zones
- **277 active bootstrap obligations:** core 32, scheduler 17, miner 139, shell 89.
  — **Confidence: high**
  - **Source:** direct record enumeration from the six bootstrap documents
- **Seven initial unassigned rows:** do not infer `miner:M32` from the prose alias “S-voices.”
  — **Confidence: high**
  - **Source:** miner scan:454; miner roadmap:14,174-177; shell scan H1/H19
- **Tombstones are dispositions, not obligations:** shell L3/L25/L28 do not inflate active count.
  — **Confidence: high**
  - **Source:** shell scan:25-30,740-748
- **Strict runtime decoding is executable authority:** JSON Schema files are review contracts;
  standard-library validators reject unknown/missing fields, duplicate keys, unsafe paths, and
  forbidden result/status keys. — **Confidence: high**
  - **Source:** approved design; existing local oracle-harness pattern inspected read-only
- **No machine result, no PASS/VERIFIED:** test declarations yield DECLARED; hashes prove identity;
  v1 cannot emit regression PASS/FAIL or parity VERIFIED. — **Confidence: high**
  - **Source:** AGENTS.md parity-certification rules; approved amendment
- **Current-anchor loss is renderable:** a disappeared well-formed anchor becomes STALE_MAPPING;
  malformed evidence remains fatal. — **Confidence: high**
  - **Source:** approved amendment
- **Source-lock-last commit marker:** obligations/evidence share a corpus digest; write them before
  replacing the source lock so interrupted Windows replacements are detectable.
  — **Confidence: high**
  - **Source:** approved amendment; Windows multi-file replacement constraint
- **CI mode ignores ignored-local files even if present:** CI output cannot depend on accidental
  checkout contents. — **Confidence: high**
  - **Source:** approved amendment
- **Independent tool package:** do not depend on private `vera20k-oracle:tools/oracle_harness/`; a future adapter
  can consume its sealed artifacts without coupling this MVP to parallel work.
  — **Confidence: high**
  - **Source:** current `git status`; approved scope boundary

No low-confidence implementation decision remains. `/review-plan` is still useful as an optional
fresh-anchor check, but no item is blocked on binary research.

## Open Questions

### Resolved During Planning

- **How should overlapping roadmap mentions be represented?** One optional primary assignment plus
  sorted typed related mentions. Multiple unresolved primaries fail validation.
- **Is the miner total 140?** No. Actual active finding headings total 139; the approximate prose
  claim is diagnostic only.
- **Is M32 mapped by “S-voices”?** No. Literal-ID import leaves `miner:M32` UNASSIGNED.
- **Do shell L3/L25/L28 become rows?** No. They become merge/retired-non-gap dispositions.
- **Can artifact hashes or declared exhaustive coverage verify parity?** No. They establish identity
  or intent only; machine-interpreted results are required.
- **Can a missing current anchor abort rendering?** No. It derives STALE_MAPPING. Unsafe or malformed
  declarations remain fatal.
- **Can CI inspect ignored sources when available?** No. `--ci` treats all `ignored-local` sources as
  unavailable by policy.

### Deferred Beyond This MVP

- **Executed regression receipts:** a later schema can ingest machine-produced named-test results
  and unlock regression PASS/FAIL.
- **Oracle comparator receipts:** a later adapter can validate sealed gamemd/Rust comparisons,
  two-run reference agreement, and exhaustive proof before enabling VERIFIED.
- **Native closure enumeration:** static/dynamic function and data closure is required before the
  top-level coverage state can leave BOOTSTRAP_PROVISIONAL.
- **Dashboard/nightly integration:** HTML, SQLite, logger, pixel capture, and nightly ratchets remain
  separate approved designs.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `tools/parity_ledger/.gitignore` | Keep Python bytecode/cache files out of git status |
| Create | `tools/parity_ledger/__init__.py` | Package/importer version constants |
| Create | `tools/parity_ledger/__main__.py` | `python -m tools.parity_ledger` entry point |
| Create | `tools/parity_ledger/errors.py` | Stable exit/failure codes and sorted diagnostics |
| Create | `tools/parity_ledger/jsonio.py` | Strict JSON, canonical bytes, hashing, safe paths, atomic writes |
| Create | `tools/parity_ledger/model.py` | Frozen enums/dataclasses for sources, assignments, evidence, rows |
| Create | `tools/parity_ledger/schema.py` | Exact-key runtime decoders and JSON-schema parity checks |
| Create | `tools/parity_ledger/source_sets.py` | Six-source bootstrap configuration and section contracts |
| Create | `tools/parity_ledger/importers/common.py` | Heading/section/ID/multiline primitives |
| Create | `tools/parity_ledger/importers/checklist.py` | Core and scheduler checklist adapters |
| Create | `tools/parity_ledger/importers/miner.py` | Miner finding and assignment-role adapters |
| Create | `tools/parity_ledger/importers/shell.py` | Shell finding, disposition, and assignment-role adapters |
| Create | `tools/parity_ledger/importers/__init__.py` | Adapter exports |
| Create | `tools/parity_ledger/corpus.py` | Import orchestration, corpus digest, loading, cross-record validation |
| Create | `tools/parity_ledger/workspace.py` | Repository discovery, git ancestry, path/test/git-candidate facts |
| Create | `tools/parity_ledger/graph.py` | Relation/dependency resolution and deterministic cycle reporting |
| Create | `tools/parity_ledger/evidence.py` | Closed evidence-check evaluation without arbitrary commands |
| Create | `tools/parity_ledger/reducer.py` | Independent-axis and queue/parity derivation |
| Create | `tools/parity_ledger/renderer.py` | Canonical ledger JSON and Markdown projection |
| Create | `tools/parity_ledger/cli.py` | `import`, `check`, and `render` command handling |
| Create | `tools/parity_ledger/tests/` | Standard-library unit and repository-corpus tests |
| Create | `parity/schemas/source-lock.v1.schema.json` | Portable source-lock contract |
| Create | `parity/schemas/obligation-set.v1.schema.json` | Portable obligation/disposition contract |
| Create | `parity/schemas/evidence-set.v1.schema.json` | Portable declaration-only evidence contract |
| Create | `parity/schemas/ledger.v1.schema.json` | Portable generated-report contract |
| Generate | `parity/sources/bootstrap.json` | Six source hashes and source-lock commit marker |
| Generate | `parity/obligations/bootstrap.json` | Canonical 277-row active obligation corpus plus dispositions |
| Generate | `parity/evidence/bootstrap.json` | Machine-derived git/test declaration candidates; no results |
| Modify | `.github/workflows/rust.yml` | Add independent Python 3.13 ledger validation job |

The implementation must not edit `.gitignore`, `vera20k-oracle:tools/oracle_harness/`, `vera20k-oracle:tools/dxgi_capture/`, any
Rust file, or any ignored source document.

## Interface Changes

No existing Rust or application interface changes.

New CLI contract:

```text
python -m tools.parity_ledger import --source-set bootstrap
python -m tools.parity_ledger check [--ci | --require-sources]
python -m tools.parity_ledger render [--output PATH]
```

Stable process exits:

```text
0   command completed; ordinary DRIFT/UNCHECKED/UNVERIFIED rows are valid output
2   invalid command-line arguments
10  tracked schema/corpus validation failed
11  --require-sources found a missing or stale required source
12  repository or git inspection failed
70  unexpected internal failure
```

New JSON contracts all use integer `schema_version: 1`, strict UTF-8, exact keys, canonical `/`
paths, lowercase SHA-256, sorted semantic arrays, and no timestamps or absolute paths.

## Risk Areas

- **Parser false positives:** `object-AI S5 slice` is not `miner:S5`; shell WS-5 contains a stale
  M35 cross-reference; ranges/front matter are not assignments.
- **Parser false negatives:** titles span physical lines and checklist obligations wrap across
  indented continuation lines.
- **Assignment hierarchy:** shell quick wins overlap parent workstreams; miner W1 has partial-history
  mentions whose remaining primary owner is W2.
- **Ignored source availability:** import works only in the developer workspace; CI must consume the
  tracked normalized corpus.
- **Git history depth:** evidence candidate validation needs full ancestry in CI, so checkout must use
  `fetch-depth: 0`.
- **Multi-file replacement:** Windows cannot atomically replace three files as one transaction;
  shared digest plus source-lock-last ordering detects partial writes.
- **Evidence overclaim:** hashes, commit ancestry, current paths, and declared tests must never become
  parity proof.
- **Parallel workspace work:** current `.gitignore`, oracle harness, and DXGI capture changes are
  unrelated user work and must remain untouched.

## Parity-Critical Items

Although this is tooling, its classifications govern future exact-parity claims.

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 2 | Forbidden result/status inputs | Hand-edited completion would recreate roadmap rot | Schema tests reject every forbidden key recursively |
| 3-5 | Exact source record boundaries | Missing one finding silently removes a parity obligation | Count 32/17/139/89 and assert all seven omissions |
| 4 | Miner assignment roles | W1 partials, W12 gates, and S5 lexical collision can mis-own work | Pin M1/M5→W2, M30→W9, S5/S12→W11 |
| 5 | Shell dispositions/roles | Merged/non-gap IDs and WS/QW overlap can inflate or duplicate backlog | Pin 89 active rows, three dispositions, H1/H19 unassigned |
| 6 | Source fingerprints and corpus digest | Stale or interrupted imports must not retain green state | Raw-byte hashes, source-lock-last write, digest mismatch test |
| 7 | Git/test evidence semantics | A commit subject or declared test is not proof the mechanism matches gamemd | Candidate/LANDED/DECLARED only; never PASS/VERIFIED |
| 8 | Reducer precedence | Wrong precedence can hide current DRIFT as UNCHECKED or VERIFIED | Table-driven cross-product tests |
| 9 | Canonical rendering | Machine and review outputs must be reproducible across Windows/Linux | Byte-identical repeated render with no timestamps/absolute paths |
| 10 | CI source policy | Accidental local docs on a runner must not change verdicts | `check --ci` forces ignored-local sources to UNAVAILABLE |

---

## Tasks

### Task 1: Establish strict JSON, hashing, path, diagnostic, and atomic-write primitives

**Why:** Every later schema, import, and render depends on deterministic bytes and fail-closed input
handling; this foundation must exist before any parser writes tracked data.

**Files:**
- Create: `tools/parity_ledger/.gitignore`
- Create: `tools/parity_ledger/__init__.py`
- Create: `tools/parity_ledger/errors.py`
- Create: `tools/parity_ledger/jsonio.py`
- Create: `tools/parity_ledger/tests/test_jsonio.py`
- Create: `tools/parity_ledger/tests/test_errors.py`

**Pattern:** New tracked tool pattern. Independently mirror the useful strict/canonical semantics of
the untracked oracle harness, but do not import from it.

**Step 1: Add package constants and local ignore rules**

```python
# tools/parity_ledger/__init__.py
"""Machine-derived VERA20k parity obligation ledger."""

IMPORTER_NAME = "vera20k-parity-ledger"
IMPORTER_VERSION = 1
SCHEMA_VERSION = 1
```

```gitignore
# tools/parity_ledger/.gitignore
__pycache__/
*.py[cod]
```

**Step 2: Define stable errors and diagnostics**

```python
# tools/parity_ledger/errors.py
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, IntEnum


class ExitCode(IntEnum):
    OK = 0
    INVALID_ARGUMENT = 2
    VALIDATION_FAILED = 10
    REQUIRED_SOURCE_FAILED = 11
    WORKSPACE_FAILED = 12
    INTERNAL_ERROR = 70


class FailureCode(str, Enum):
    UNSUPPORTED_SCHEMA = "UNSUPPORTED_SCHEMA"
    SCHEMA_INVALID = "SCHEMA_INVALID"
    NONCANONICAL_JSON = "NONCANONICAL_JSON"
    UNSAFE_PATH = "UNSAFE_PATH"
    SOURCE_MALFORMED = "SOURCE_MALFORMED"
    SOURCE_UNAVAILABLE = "SOURCE_UNAVAILABLE"
    SOURCE_STALE = "SOURCE_STALE"
    DUPLICATE_OBLIGATION = "DUPLICATE_OBLIGATION"
    DUPLICATE_ASSIGNMENT = "DUPLICATE_ASSIGNMENT"
    UNRESOLVED_RELATION = "UNRESOLVED_RELATION"
    UNRESOLVED_DEPENDENCY = "UNRESOLVED_DEPENDENCY"
    DEPENDENCY_CYCLE = "DEPENDENCY_CYCLE"
    EVIDENCE_INVALID = "EVIDENCE_INVALID"
    CURRENT_ANCHOR_MISSING = "CURRENT_ANCHOR_MISSING"
    GIT_FAILED = "GIT_FAILED"
    CORPUS_DIGEST_MISMATCH = "CORPUS_DIGEST_MISMATCH"
    OUTPUT_IO_FAILED = "OUTPUT_IO_FAILED"
    INTERNAL_ERROR = "INTERNAL_ERROR"


@dataclass(frozen=True, order=True)
class Diagnostic:
    code: str
    source_path: str = ""
    record_id: str = ""
    field: str = ""
    message: str = ""
    fatal: bool = False

    def to_document(self) -> dict[str, object]:
        return {
            "code": self.code,
            "fatal": self.fatal,
            "field": self.field,
            "message": self.message,
            "record_id": self.record_id,
            "source_path": self.source_path,
        }


class LedgerError(Exception):
    def __init__(self, exit_code: ExitCode, diagnostics: list[Diagnostic]) -> None:
        super().__init__(diagnostics[0].message if diagnostics else exit_code.name)
        self.exit_code = exit_code
        self.diagnostics = tuple(sorted(diagnostics))
```

**Step 3: Implement strict JSON and canonical bytes**

`jsonio.py` must contain these exact public functions:

```python
def load_json_strict(data: str | bytes) -> object
def canonical_json_bytes(value: object) -> bytes
def sha256_bytes(data: bytes) -> str
def sha256_file(path: Path) -> str
def validate_relative_path(value: str, *, field: str = "path") -> str
def atomic_write_bytes(path: Path, payload: bytes) -> None
```

Use duplicate-key rejection through `object_pairs_hook`, reject `NaN`/infinity through
`parse_constant`, recursively accept only JSON null/bool/int/finite-float/string/list/dict, and
serialize exactly:

```python
encoded = json.dumps(
    value,
    ensure_ascii=False,
    allow_nan=False,
    sort_keys=True,
    separators=(",", ":"),
).encode("utf-8")
return encoded + b"\n"
```

`validate_relative_path` rejects absolute paths, drive prefixes, backslashes, empty/`.`/`..`
segments, colons/alternate streams, control characters, trailing spaces/periods, segments over 255
characters, and Win32 device stems `CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`.

`atomic_write_bytes` creates the parent, writes an exclusive temporary sibling, flushes and fsyncs,
then calls `os.replace`; its `finally` block removes an uncommitted temporary file.

**Step 4: Add focused tests**

Tests must assert:

```python
self.assertEqual(
    canonical_json_bytes({"z": 2, "a": "Yuri Ø"}),
    '{"a":"Yuri Ø","z":2}\n'.encode("utf-8"),
)
```

Also assert duplicate keys, nonfinite values, unpaired surrogates, unsafe paths, and short writes are
rejected; streamed file SHA-256 equals `hashlib.sha256(payload).hexdigest()`; diagnostics sort
independently of discovery order; and failed replacement leaves the destination unchanged.

**Step 5: Verify**

Run:

```powershell
python -m unittest tools.parity_ledger.tests.test_jsonio tools.parity_ledger.tests.test_errors -v
```

Expected: all tests report `OK`; no files outside `tools/parity_ledger/` change.

### Task 2: Define strict v1 models, runtime decoders, and portable schema documents

**Why:** Interfaces must be fixed before importers or reducers consume them, and the approved
amendment must be represented explicitly rather than hidden in parser heuristics.

**Files:**
- Create: `tools/parity_ledger/model.py`
- Create: `tools/parity_ledger/schema.py`
- Create: `tools/parity_ledger/tests/test_schema.py`
- Create: `tools/parity_ledger/tests/test_model.py`
- Create: `parity/schemas/source-lock.v1.schema.json`
- Create: `parity/schemas/obligation-set.v1.schema.json`
- Create: `parity/schemas/evidence-set.v1.schema.json`
- Create: `parity/schemas/ledger.v1.schema.json`

**Pattern:** Frozen dataclasses internally; exact-key decoding before construction; JSON Schema
documents stay reviewable while runtime validators are executable authority.

**Step 1: Define closed enums**

`model.py` defines string enums for:

```python
class Tracking(str, Enum): TRACKED = "tracked"; IGNORED_LOCAL = "ignored-local"
class AssignmentRole(str, Enum):
    PRIMARY = "primary"; PARENT = "parent"; QUICK_WIN = "quick_win"
    RESEARCH_GATE = "research_gate"; DEFERRED = "deferred"
    HISTORICAL_PARTIAL = "historical_partial"
class DispositionKind(str, Enum): MERGED = "merged"; RETIRED_NON_GAP = "retired_non_gap"
class SourceState(str, Enum): CURRENT = "CURRENT"; STALE = "STALE"; UNAVAILABLE = "UNAVAILABLE"
class AssignmentState(str, Enum):
    ASSIGNED = "ASSIGNED"; UNASSIGNED = "UNASSIGNED"
    ALIASED = "ALIASED"; SUPERSEDED = "SUPERSEDED"
class ImplementationState(str, Enum):
    NONE = "NONE"; CANDIDATE = "CANDIDATE"; LANDED = "LANDED"
    STALE_MAPPING = "STALE_MAPPING"
class RegressionState(str, Enum): NONE = "NONE"; DECLARED = "DECLARED"; PASS = "PASS"; FAIL = "FAIL"
class OracleState(str, Enum): NONE = "NONE"; INCOMPLETE = "INCOMPLETE"; SAMPLED = "SAMPLED"; EXHAUSTIVE = "EXHAUSTIVE"
class ParityVerdict(str, Enum): DRIFT = "DRIFT"; UNCHECKED = "UNCHECKED"; UNVERIFIED = "UNVERIFIED"; VERIFIED = "VERIFIED"
class QueueState(str, Enum):
    NOT_ACTIONABLE = "NOT_ACTIONABLE"; NEEDS_SOURCE_REFRESH = "NEEDS_SOURCE_REFRESH"
    NEEDS_ASSIGNMENT = "NEEDS_ASSIGNMENT"; DEPENDENCY_BLOCKED = "DEPENDENCY_BLOCKED"
    NEEDS_RESEARCH = "NEEDS_RESEARCH"; NEEDS_IMPLEMENTATION = "NEEDS_IMPLEMENTATION"
    NEEDS_REGRESSION = "NEEDS_REGRESSION"; NEEDS_ORACLE = "NEEDS_ORACLE"
    NO_ACTION = "NO_ACTION"
```

Keep PASS, FAIL, EXHAUSTIVE, and VERIFIED in the interface but add reducer tests proving they are
unreachable from all v1 declaration-only inputs.

**Step 2: Define frozen dataclasses and exact document shapes**

Define frozen classes with tuple collections and `to_document()` methods:

```python
SourceRef(path, local_id, source_key, sha256, tracking, importer, importer_version)
SourceFileLock(source_id, system, role, path, sha256, tracking, adapter, declared_count)
AssignmentMention(workstream, role, source_path)
Assignment(primary, related)
SourceClaims(severity, activation, player_frequency, determinism_impact)
Disposition(source_id, kind, targets, source)
Obligation(id, system, kind, title, source, source_claims, assignment,
           dependencies, relations, rust_anchors)
ArtifactRef(path, sha256)
Provenance(executable_sha256, tool, tool_version, scenario, activation_proof,
           commit, reference_runs)
Coverage(mode, domain)
EvidenceCheck(type, commit, path, test_name, left_trace, right_trace)
EvidenceDeclaration(id, obligations, kind, artifact, provenance, coverage, check)
WorkspaceFacts(source_states, implementation_facts, regression_facts, evidence_facts)
ReducedRow(obligation, source_state, assignment_state, implementation_state,
           regression_state, oracle_state, parity_verdict, queue_state, diagnostics)
LedgerReport(corpus_digest, coverage_state, counts, rows, dispositions, diagnostics)
```

All optional JSON fields are present as `null`; every array is sorted before serialization. The
three tracked bundle documents have exact top-level keys:

```text
source lock:    schema_version, source_set, importer, importer_version, corpus_digest, sources
obligation set: schema_version, source_set, corpus_digest, obligations, dispositions, diagnostics
evidence set:   schema_version, source_set, corpus_digest, evidence
ledger report:  schema_version, corpus_digest, coverage_state, counts, rows, dispositions, diagnostics
```

**Step 3: Implement strict decoders**

`schema.py` supplies `_expect_object`, `_expect_list`, `_expect_str`, `_expect_int`, `_expect_bool`,
`_expect_enum`, `_expect_sha256`, `_expect_nullable`, and one decoder per tracked document. It must:

- require exact nested keys;
- reject bool where int is expected;
- require lowercase 64-character SHA-256;
- validate every stored path with `validate_relative_path`;
- require namespaced obligation IDs matching their `system` prefix;
- recursively reject input keys exactly named `status`, `done`, `complete`, `landed`, `pass`, or
  `verified` case-insensitively;
- require sorted unique obligation IDs, assignments, dependencies, relations, anchors, evidence
  links, dispositions, and reference runs;
- enforce evidence check combinations: `git_ancestor` requires commit only; `path_exists` requires
  path only; `test_declared` requires path+test; `artifact_hash` requires artifact path+hash;
  `bridge_trace` requires both traces plus full provenance but remains declaration-only.

**Step 4: Create matching JSON Schema Draft 2020-12 documents**

Each document uses `additionalProperties: false`, exact required arrays, `schema_version: {"const":
1}`, safe relative-path patterns, SHA-256 patterns, and the enums above. `schema.py` exposes
`assert_schema_document_parity()` comparing top-level properties/required fields, enum values, and
version constants with runtime contracts.

**Step 5: Add schema/model tests**

Cover unknown/missing nested fields, forbidden keys at every nesting level, bool/int distinction,
unsafe paths, uppercase/short hashes, namespace mismatch, unsorted arrays, unsupported versions,
schema/runtime parity, disposition target shape, one-primary-plus-related assignments, and all
evidence check combinations.

**Step 6: Verify**

```powershell
python -m unittest tools.parity_ledger.tests.test_schema tools.parity_ledger.tests.test_model -v
```

Expected: all tests `OK`; the four schema documents are canonical JSON.

### Task 3: Implement common section parsing and core/scheduler checklist adapters

**Why:** Checklist sources have deterministic bounded sections and provide the simplest real import
path for validating IDs, multiline text, checkbox-status exclusion, and source hashing.

**Files:**
- Create: `tools/parity_ledger/source_sets.py`
- Create: `tools/parity_ledger/importers/__init__.py`
- Create: `tools/parity_ledger/importers/common.py`
- Create: `tools/parity_ledger/importers/checklist.py`
- Create: `tools/parity_ledger/tests/test_import_checklist.py`
- Create: `tools/parity_ledger/tests/fixtures/core-checklist.md`
- Create: `tools/parity_ledger/tests/fixtures/scheduler-checklist.md`

**Pattern:** Pure source-specific adapters from raw bytes to typed obligations plus diagnostics.

**Step 1: Declare the six bootstrap sources**

`source_sets.py` defines an immutable `BOOTSTRAP_SOURCES` tuple with source ID, system, role, path,
adapter, and tracking for:

```text
core-todo             core       inventory   docs/plans/2026-05-29-core-engine-substrate-todo.md
scheduler-roadmap     scheduler  inventory   docs/plans/2026-05-28-foundational-scheduler-roadmap-todo.md
miner-scan            miner      inventory   docs/gap-scans/2026-07-02-disparity-scan-miner.md
miner-roadmap         miner      assignment  docs/plans/2026-07-02-miner-parity-roadmap.md
shell-scan            shell      inventory   docs/gap-scans/2026-07-06-disparity-scan-shell-ui.md
shell-roadmap         shell      assignment  docs/plans/2026-07-06-shell-ui-parity-roadmap.md
```

All six use `ignored-local` tracking.

**Step 2: Add exact parser primitives**

`common.py` defines:

```python
H2_RE = re.compile(r"^## (?!#)(?P<title>.+?)\s*$", re.MULTILINE)
H3_RE = re.compile(r"^### (?!#)(?P<title>.+?)\s*$", re.MULTILINE)
CHECK_RE = re.compile(r"^- \[(?P<mark>[ xX])\] (?P<lead>.+)$")
ID_RE = re.compile(r"(?<![A-Z0-9_])(?P<family>[GHMLS])(?P<num>[1-9]\d*)(?![A-Z0-9_])")
RANGE_RE = re.compile(
    r"(?<![A-Z0-9_])([GHMLS])([1-9]\d*)\s*(?:-|–|—|\.\.)\s*\1([1-9]\d*)"
    r"(?![A-Z0-9_])"
)
RUST_PATH_RE = re.compile(r"\b(?:src|tests)/[A-Za-z0-9_./-]+\.rs\b")
```

Add `strict_text(raw)` that hashes raw bytes before `raw.decode("utf-8", errors="strict")`,
`bounded_section(text, start_heading, end_heading)` that requires each anchor exactly once and in
order, `fold_markdown(text)` using `re.sub(r"[ \t\r\n]+", " ", text).strip()`, fenced-code
exclusion, range expansion before scalar-ID extraction, and stable diagnostic sorting.

**Step 3: Generate deterministic checklist IDs**

For each column-zero checkbox plus indented continuation lines, ignore the mark, strip a scheduler
suffix beginning at `— **done`, fold whitespace, and compute:

```python
payload = "\0".join((namespace, " / ".join(heading_path), title)).encode("utf-8")
suffix = hashlib.sha256(payload).hexdigest()[:16]
obligation_id = f"{namespace}:{slugify(heading_path[-1])}:{suffix}"
```

Line numbers, checkbox marks, and status prose never enter identity.

**Step 4: Bound the real sections exactly**

- Core: `## Big Missing Core Systems` through before `## Suggested Next Work`; current H3 owns each
  item. Expect eight groups with four rows each.
- Scheduler: parse only `## Contract Stack To Create`, `## Implementation Roadmap`, and
  `## Open Follow-Up Research`. Expect 7, 5, and 5 rows respectively. Nested bullets are detail.

Assign each checklist obligation primarily to its namespaced heading workstream and leave source
claims as `unknown` except kind (`core_obligation`, `contract`, `implementation`, or `research`).

**Step 5: Add tests**

Fixture tests prove multiline folding, `[x]`/`[ ]` identity equality, heading/text changes creating
new IDs, nested bullets excluded, source/status/do-not-do sections excluded, source raw-byte hash
changes despite stable semantic ID, and exact real-source counts 32 and 17 when the local docs are
available.

**Step 6: Verify**

```powershell
python -m unittest tools.parity_ledger.tests.test_import_checklist -v
```

Expected: all fixture tests `OK`; real-source probe reports core=32 and scheduler=17.

### Task 4: Implement miner finding and hierarchical assignment adapters

**Why:** Miner contains the largest subtle parser surface: four record shapes, an incorrect declared
total, partial/historical mentions, research gates, a false `S5` lexical collision, and five literal
assignment omissions.

**Files:**
- Create: `tools/parity_ledger/importers/miner.py`
- Create: `tools/parity_ledger/tests/test_import_miner.py`
- Create: `tools/parity_ledger/tests/fixtures/miner-scan.md`
- Create: `tools/parity_ledger/tests/fixtures/miner-roadmap.md`

**Pattern:** Source-specific bounded state machine; no global find-all regex over the whole document.

**Step 1: Parse the four confirmed regions**

Bound these H2 regions exactly:

```text
Confirmed gaps — HIGH severity -> Confirmed gaps — MEDIUM severity
Confirmed gaps — MEDIUM severity -> Confirmed gaps — LOW severity
Confirmed gaps — LOW severity -> Slave miner & OREGATH additions
Slave miner & OREGATH additions -> Needs verification
```

Recognize record starts with `^(?:- )?\*\*([GMLS]\d+)\.`. Collect physical lines through the next
record or region end. For G/M and bold-title S records, take the title through the first closing
`**`; for `- **L7.** paragraph` and `- **S9.** paragraph`, take the folded first list-item paragraph.
Extract only literal existing Rust paths into `rust_anchors` and derive severity from G/M/L family or
the current HIGH/MEDIUM/LOW slave subheading.

Require exact contiguous ranges:

```python
assert_ids("G", 1, 19)
assert_ids("M", 1, 33)
assert_ids("L", 1, 67)
assert_ids("S", 1, 20)
```

The active miner result is exactly 139. Record the roadmap's exact `140` claim as a nonfatal
`DECLARED_COUNT_MISMATCH` diagnostic.

**Step 2: Parse assignment mentions only in the approved region**

Read `## W0` through the end of `## Deferred`, stopping before `## Suggested sequence`; never inspect
`## Status`. Classify mentions:

- W0-W11 normal scope: primary candidate.
- W12 G/M/L/S mentions: `research_gate` related mention only.
- Deferred section: primary `miner:DEFERRED` plus `deferred` relation.
- `object-AI S5 slice`: ignore that `S5` token; it is a scheduler slice label.
- `S12 ... NOT in W1 — deferred to W11`: primary `miner:W11`.
- W1 M1/M5 `half advanced`: `historical_partial`; W2 is primary.
- W9 M30 remains primary; W12 M30 is a research gate.

If more than one primary remains after these typed rules, emit fatal DUPLICATE_ASSIGNMENT.

**Step 3: Pin the known literal results**

```python
expected_unassigned = {"miner:L7", "miner:L34", "miner:L35", "miner:L43", "miner:M32"}
assert assignment["miner:M1"].primary == "miner:W2"
assert assignment["miner:M5"].primary == "miner:W2"
assert assignment["miner:M30"].primary == "miner:W9"
assert assignment["miner:S5"].primary == "miner:W11"
assert assignment["miner:S12"].primary == "miner:W11"
```

Do not invent `S-voices -> M32`; the literal ID does not appear before Status.

**Step 4: Add tests and verify**

```powershell
python -m unittest tools.parity_ledger.tests.test_import_miner -v
```

Expected: 19 G + 33 M + 67 L + 20 S = 139; exactly the five IDs above are unassigned; W12 and W1
secondary mentions remain visible but never replace primaries.

### Task 5: Implement shell finding, disposition, and hierarchical assignment adapters

**Why:** Shell uses 89 active headings while retaining three historical IDs, and its quick-win/
workstream/research/deferred layers overlap heavily.

**Files:**
- Create: `tools/parity_ledger/importers/shell.py`
- Create: `tools/parity_ledger/tests/test_import_shell.py`
- Create: `tools/parity_ledger/tests/fixtures/shell-scan.md`
- Create: `tools/parity_ledger/tests/fixtures/shell-roadmap.md`

**Pattern:** Bounded source adapter with explicit disposition and mention-role output.

**Step 1: Parse active findings**

Bound HIGH→MEDIUM→LOW→Needs Verification. Recognize standalone bold titles with
`^\*\*([HML]\d+)\.` and fold through the closing `**`. Require:

```python
assert_ids("H", 1, 19)
assert_ids("M", 1, 39)
assert len([row for row in rows if row.local_id.startswith("L")]) == 31
assert len(rows) == 89
```

Do not require contiguous L1-L34 because three retained IDs are not active headings.

**Step 2: Emit dispositions**

Generate exactly:

```python
Disposition("shell:L3", MERGED, ("shell:L2", "shell:M13"), source_ref)
Disposition("shell:L25", MERGED, ("shell:L24",), source_ref)
Disposition("shell:L28", RETIRED_NON_GAP, (), source_ref)
```

Validate that none of the three source IDs also exists in active obligations.

**Step 3: Parse assignment roles**

- WS-1..WS-13 `Scope (closes)` paragraphs produce primary workstream candidates.
- QW-1..QW-9 headers produce `quick_win` mentions and override the broader WS candidate as primary;
  retain the WS as `parent`.
- `with WS-X` and `folded in ID` occurrences are related, never primary.
- Research-first table rows produce `research_gate` mentions only.
- The exact `Deferred (blocked, not scheduled)` paragraph overlays `deferred` on H10, M29, L9, M1,
  L27, and L1 without changing an existing primary.
- Ignore front matter, suggested-wave prose, and generic ID ranges as assignments.
- Treat the WS-5 M35 reference as a stale secondary mention and preserve WS-8 as M35's primary.

**Step 4: Pin results and verify**

```python
assert unassigned == {"shell:H1", "shell:H19"}
assert assignment["shell:M35"].primary == "shell:WS-8"
assert all(disposition.source_id not in active_ids for disposition in dispositions)
```

Run:

```powershell
python -m unittest tools.parity_ledger.tests.test_import_shell -v
```

Expected: 19 H + 39 M + 31 L = 89; three dispositions; H1/H19 unassigned; parent/QW overlaps are
related rather than duplicate primaries.

### Task 6: Orchestrate bootstrap import, source locks, corpus digests, and CLI import

**Why:** The adapters must merge into one validated corpus and write all tracked records without
leaving an apparently current partial import.

**Files:**
- Create: `tools/parity_ledger/corpus.py`
- Create: `tools/parity_ledger/cli.py`
- Create: `tools/parity_ledger/__main__.py`
- Create: `tools/parity_ledger/tests/test_corpus.py`
- Create: `tools/parity_ledger/tests/test_cli_import.py`

**Pattern:** Validate everything in memory, canonicalize, then commit generated files with one digest
marker.

**Step 1: Add corpus assembly**

`corpus.py` exposes:

```python
def import_source_set(repo: Path, source_set: str) -> ImportBundle
def corpus_digest(obligation_document: dict[str, object], evidence_payload: list[object]) -> str
def validate_cross_records(bundle: ImportBundle) -> list[Diagnostic]
def write_import(repo: Path, bundle: ImportBundle) -> None
def load_tracked_corpus(repo: Path) -> Corpus
```

Import order is source configuration order, while serialized obligations sort by ID, dispositions by
source ID, assignments by `(role, workstream, source_path)`, and diagnostics by their dataclass order.

Compute the digest from a canonical document containing only `source_set`, `obligations`,
`dispositions`, and `evidence`; inject that digest into all three tracked documents afterward.

**Step 2: Enforce source-lock-last replacement**

Canonicalize and validate all three documents first. Write temporary siblings for all three. Replace
`parity/obligations/bootstrap.json`, then `parity/evidence/bootstrap.json`, and finally
`parity/sources/bootstrap.json`. `load_tracked_corpus` recomputes the digest and fails with
CORPUS_DIGEST_MISMATCH if any payload and marker disagree.

**Step 3: Wire only the import command**

`__main__.py` calls `cli.main()`. `cli.py` creates argparse subcommands but Task 6 enables only:

```text
import --source-set bootstrap
```

Unknown source sets exit 2. Source/validation/IO failures print one canonical JSON diagnostic object
to stderr and use exit 10/11/70 as appropriate. Ordinary count mismatch diagnostics remain in the
tracked obligation document and exit 0.

**Step 4: Test transaction and determinism**

- Import twice into a temporary repository and assert all three files are byte-identical.
- Inject failure before the source-lock replace and assert `load_tracked_corpus` detects the mismatch.
- Change one ignored source and assert the next import changes its raw SHA and corpus digest.
- Assert import never writes outside `parity/sources`, `parity/obligations`, and `parity/evidence`.

**Step 5: Verify**

```powershell
python -m unittest tools.parity_ledger.tests.test_corpus tools.parity_ledger.tests.test_cli_import -v
```

Expected: all tests `OK`; repository tracked corpus has not yet been generated in this task.

### Task 7: Add current workspace, git, test-declaration, graph, and evidence facts

**Why:** Current code and history must inform implementation/regression axes, while never becoming
parity proof or executing arbitrary manifest commands.

**Files:**
- Create: `tools/parity_ledger/workspace.py`
- Create: `tools/parity_ledger/graph.py`
- Create: `tools/parity_ledger/evidence.py`
- Create: `tools/parity_ledger/tests/test_workspace.py`
- Create: `tools/parity_ledger/tests/test_graph.py`
- Create: `tools/parity_ledger/tests/test_evidence.py`

**Pattern:** Read-only repository facts; subprocess argument arrays only; closed evidence check enum.

**Step 1: Discover and constrain the repository**

`find_repo_root(start)` walks parents until both `Cargo.toml` and `.git` exist. Every current path is
resolved and checked to remain under that root. Git runs through:

```python
def run_git(repo: Path, args: tuple[str, ...]) -> str:
    completed = subprocess.run(
        ("git", "-C", str(repo), *args),
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="strict",
    )
```

Nonzero exit becomes GIT_FAILED; no shell string is accepted.

**Step 2: Derive git evidence candidates conservatively**

Read `git log --format=%H%x09%s HEAD`, then changed paths with `git diff-tree --no-commit-id
--name-only -r COMMIT`. Extract literal finding IDs from subjects. Link a commit only when:

1. the local ID matches an active obligation;
2. either that ID is system-unique or changed paths intersect the obligation's source-cited
   `rust_anchors`;
3. exactly one obligation remains after that filter.

Ambiguous matches produce a nonfatal diagnostic and no evidence. Store commit ancestry and current
changed-path declarations, never a result/status.

Scan added patch lines for `#[test]` followed by `fn NAME` and confirm the same declaration remains in
the current file with a multiline regex. A current declaration yields regression DECLARED only.

**Step 3: Evaluate closed evidence checks**

- `git_ancestor`: `git merge-base --is-ancestor COMMIT HEAD`; ancestor supports implementation
  evidence, nonancestor does not.
- `path_exists`: present path supports CANDIDATE; missing path yields STALE_MAPPING.
- `test_declared`: current exact file/function declaration supports DECLARED; missing declaration
  yields STALE_MAPPING.
- `artifact_hash`: matching bytes prove artifact identity only and leave oracle INCOMPLETE.
- `bridge_trace`: validate paths/provenance only; v1 leaves oracle INCOMPLETE regardless of process
  exit or declared coverage.

No check can produce regression PASS/FAIL, oracle EXHAUSTIVE, or parity VERIFIED.

**Step 4: Validate relations/dependencies**

`graph.py` rejects unresolved/self dependencies, duplicate relation edges, active/disposition ID
collisions, and cycles. DFS visits sorted IDs and reports the canonical cycle beginning at its
lexicographically smallest member.

**Step 5: Add tests and verify**

Use injected fake git runners for unit tests. Cover ancestor/nonancestor/error, ambiguous commit ID,
anchor intersection, added/current test declarations, missing anchors, artifact hash mismatch,
bridge exit-zero-with-failure remaining incomplete, disposition target resolution, and stable cycle
paths.

```powershell
python -m unittest tools.parity_ledger.tests.test_workspace tools.parity_ledger.tests.test_graph tools.parity_ledger.tests.test_evidence -v
```

Expected: all tests `OK`; no external command other than mocked/read-only git is executed.

### Task 8: Implement conservative reducer precedence and status-axis tests

**Why:** This is the ledger's central correctness boundary; one precedence mistake can convert a
known drift into a false completion claim.

**Files:**
- Create: `tools/parity_ledger/reducer.py`
- Create: `tools/parity_ledger/tests/test_reducer.py`

**Pattern:** Pure functions from validated obligations plus workspace facts to immutable reduced rows.

**Step 1: Implement each axis independently**

Use these exact rules:

```text
source:
  --ci + ignored-local -> UNAVAILABLE
  present hash match -> CURRENT
  present hash mismatch -> STALE
  absent -> UNAVAILABLE

assignment:
  disposition/alias target -> ALIASED or SUPERSEDED
  no primary -> UNASSIGNED
  one resolved primary -> ASSIGNED
  multiple unresolved primaries -> validation failure before reduction

implementation:
  no git/path declaration -> NONE
  candidate anchor without validated ancestor -> CANDIDATE
  ancestor commit plus all declared current anchors -> LANDED
  any once-declared current anchor missing -> STALE_MAPPING

regression:
  no named test -> NONE
  current named test -> DECLARED
  PASS/FAIL unreachable in v1

oracle:
  no oracle declaration -> NONE
  any v1 declaration -> INCOMPLETE or SAMPLED
  EXHAUSTIVE unreachable in v1
```

**Step 2: Implement parity precedence**

```python
def derive_parity(obligation: Obligation, facts: RowFacts) -> ParityVerdict:
    if facts.source_state is SourceState.STALE:
        return ParityVerdict.UNCHECKED
    if obligation.kind == "parity_gap" and facts.implementation_state not in {
        ImplementationState.LANDED,
        ImplementationState.STALE_MAPPING,
    }:
        return ParityVerdict.DRIFT
    if facts.implementation_state is ImplementationState.STALE_MAPPING:
        return ParityVerdict.UNCHECKED
    if facts.implementation_state is ImplementationState.LANDED:
        if facts.oracle_state is OracleState.INCOMPLETE and facts.oracle_attempted:
            return ParityVerdict.UNCHECKED
        return ParityVerdict.UNVERIFIED
    return ParityVerdict.UNCHECKED
```

UNAVAILABLE does not erase a locked imported gap's DRIFT claim in CI.

**Step 3: Implement queue precedence**

```text
disposition/nonactionable -> NOT_ACTIONABLE
stale source -> NEEDS_SOURCE_REFRESH
unassigned -> NEEDS_ASSIGNMENT
unresolved dependency -> DEPENDENCY_BLOCKED
research/activation gap -> NEEDS_RESEARCH
DRIFT or reserved regression failure -> NEEDS_IMPLEMENTATION
LANDED without executed regression -> NEEDS_REGRESSION
LANDED without qualifying oracle -> NEEDS_ORACLE
reserved VERIFIED -> NO_ACTION
```

Deferral is a sort hint stored in related assignments; it never replaces verdict or queue state.

**Step 4: Add exhaustive table tests**

Table-drive source availability/staleness, assignment states, every implementation state, named-test
presence, oracle declaration, dependency state, gap versus core kind, and deferral. Explicitly assert
that no v1 input combination returns PASS, FAIL, EXHAUSTIVE, or VERIFIED.

**Step 5: Verify**

```powershell
python -m unittest tools.parity_ledger.tests.test_reducer -v
```

Expected: all precedence cases `OK`.

### Task 9: Add deterministic rendering, check/render CLI, bootstrap generation, and acceptance tests

**Why:** Integrate the complete tool, create the first tracked corpus, and prove it reports the real
bootstrap inventory rather than roadmap prose.

**Files:**
- Create: `tools/parity_ledger/renderer.py`
- Create: `tools/parity_ledger/tests/test_renderer.py`
- Create: `tools/parity_ledger/tests/test_cli.py`
- Create: `tools/parity_ledger/tests/test_tracked_corpus.py`
- Modify: `tools/parity_ledger/cli.py`
- Generate: `parity/sources/bootstrap.json`
- Generate: `parity/obligations/bootstrap.json`
- Generate: `parity/evidence/bootstrap.json`

**Pattern:** Canonical JSON is machine authority; Markdown is a deterministic human projection.

**Step 1: Render canonical ledger JSON**

Emit exact top-level fields:

```json
{
  "schema_version": 1,
  "corpus_digest": "lowercase-64-character-sha256",
  "coverage_state": "BOOTSTRAP_PROVISIONAL",
  "counts": {},
  "rows": [],
  "dispositions": [],
  "diagnostics": []
}
```

Populate counts independently for every source, assignment, implementation, regression, oracle,
parity, and queue enum. Sort rows by queue rank, controlled player-frequency rank, determinism rank,
research readiness, then ID; `unknown` sorts last. Serialize with `canonical_json_bytes`.

**Step 2: Render deterministic Markdown**

Write `summary.md` with sections in this order:

```text
Coverage State
Inventory Counts
Unassigned Obligations
Stale Sources and Mappings
Parity Verdicts
Next Queue
Source Diagnostics
Dispositions
```

Every section sorts IDs. Include an explicit sentence that BOOTSTRAP_PROVISIONAL is not a certified
completion percentage. Do not include timestamps, absolute paths, environment values, or a percent
complete.

**Step 3: Wire check and render commands**

- `check --require-sources`: validate all tracked corpus data, rehash all six ignored sources, and
  exit 11 on absent/stale source.
- `check --ci`: ignore the filesystem presence of every ignored-local source, set them UNAVAILABLE,
  validate tracked corpus/digest/current git facts, and exit 0 for ordinary open rows.
- Reject simultaneous `--ci` and `--require-sources` through argparse exit 2.
- `render --output PATH`: run normal check, reduce, and atomically write `ledger.json` and
  `summary.md` under the requested repository-contained output path. Never modify tracked corpus.

**Step 4: Generate and inspect the bootstrap corpus**

Run:

```powershell
python -m tools.parity_ledger import --source-set bootstrap
python -m tools.parity_ledger check --require-sources
python -m tools.parity_ledger render --output target/parity-ledger
```

Inspect generated diffs. Required results:

```text
core=32
scheduler=17
miner=139
shell=89
active_total=277
unassigned={miner:L7, miner:L34, miner:L35, miner:L43, miner:M32, shell:H1, shell:H19}
dispositions={shell:L3, shell:L25, shell:L28}
coverage_state=BOOTSTRAP_PROVISIONAL
verified=0
regression_pass=0
regression_fail=0
```

Re-run import and assert `git diff -- parity/` is unchanged.

**Step 5: Add repository-corpus tests**

CI-safe tests read only tracked JSON and assert the totals, seven unassigned IDs, three disposition
IDs, shared digest, canonical bytes, no forbidden keys, zero VERIFIED/PASS/FAIL, and no project
completion percentage. Source-dependent tests run only when explicitly invoked through
`--require-sources`, not during clean CI unit discovery.

**Step 6: Verify**

```powershell
python -m unittest discover -s tools/parity_ledger/tests -p "test_*.py" -v
python -m tools.parity_ledger check --require-sources
python -m tools.parity_ledger render --output target/parity-ledger
python -m tools.parity_ledger check --ci
```

Expected: all tests `OK`; all commands exit 0; tracked corpus is stable after the second import.

### Task 10: Add isolated CI validation and run the final Python-only verification

**Why:** Make corpus/schema rot fail pull requests without changing the existing Rust matrix or
requiring ignored docs, retail assets, Ghidra, Cargo, or oracle capture infrastructure.

**Files:**
- Modify: `.github/workflows/rust.yml`

**Pattern:** Independent lightweight job; existing Windows/Linux Rust job remains byte-for-byte
unchanged except for YAML indentation needed to add a sibling job.

**Step 1: Re-read current workspace state before editing**

Run:

```powershell
git status --short --branch
git diff -- .github/workflows/rust.yml .gitignore
```

Stop if another session modified `.github/workflows/rust.yml`. Never edit or revert `.gitignore`,
`vera20k-oracle:tools/oracle_harness/`, or `vera20k-oracle:tools/dxgi_capture/`.

**Step 2: Add the CI job**

Add this sibling under `jobs:` before the existing `test` job:

```yaml
  parity-ledger:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: actions/setup-python@v6
        with:
          python-version: "3.13"
      - name: Test parity ledger
        run: python -m unittest discover -s tools/parity_ledger/tests -p "test_*.py" -v
      - name: Check parity ledger corpus
        run: python -m tools.parity_ledger check --ci
```

Full history is required for ancestor checks. No pip install/cache step is allowed because the MVP
is standard-library-only.

**Step 3: Run final verification**

```powershell
python --version
python -m unittest discover -s tools/parity_ledger/tests -p "test_*.py" -v
python -m tools.parity_ledger import --source-set bootstrap
python -m tools.parity_ledger check --require-sources
python -m tools.parity_ledger render --output target/parity-ledger
python -m tools.parity_ledger check --ci
python -m tools.parity_ledger import --source-set bootstrap
git diff --check
git status --short -- parity tools/parity_ledger .github/workflows/rust.yml
```

Expected:

- Python reports 3.13.x.
- Every unittest command ends `OK`.
- Both check modes exit 0 with BOOTSTRAP_PROVISIONAL coverage.
- The second import produces no diff in `parity/`.
- `git diff --check` is silent.
- Scoped status lists only intended ledger/CI changes.
- Existing user changes remain present and untouched.

Do not run Cargo: no Rust surface changed.

## Sources & References

- **Design doc:** `docs/plans/2026-07-10-parity-ledger-mvp-design.md`
- **Convergence strategy:** `docs/plans/2026-07-05-parity-convergence-strategy.md:34-49,51-79,95-98`
- **Core inventory:** `docs/plans/2026-05-29-core-engine-substrate-todo.md:36-137`
- **Scheduler inventory:** `docs/plans/2026-05-28-foundational-scheduler-roadmap-todo.md:28-105,126-132`
- **Miner scan:** `docs/gap-scans/2026-07-02-disparity-scan-miner.md:20-32,54-694`
- **Miner roadmap:** `docs/plans/2026-07-02-miner-parity-roadmap.md:3-146`
- **Shell scan:** `docs/gap-scans/2026-07-06-disparity-scan-shell-ui.md:22-32,198-748`
- **Shell roadmap:** `docs/plans/2026-07-06-shell-ui-parity-roadmap.md:56-413`
- **Bridge evidence model:** `tools/bridge_oracle/schema.md:1-58`
- **Bridge comparator:** `src/bin/bridge-oracle-compare.rs:70-100`
- **Rust regression harness:** `src/sim/world/global_parity_harness_tests.rs:1-11,59-84`
- **Replay regression model:** `src/sim/replay.rs:16-19`
- **Research-index metadata limitation:** `tools/research_index/research_index/metadata.py:145-174`
- **Current CI:** `.github/workflows/rust.yml:1-18`
- **Official Python action:** [actions/setup-python](https://github.com/actions/setup-python)
- **Official checkout action:** [actions/checkout](https://github.com/actions/checkout)
- **Ghidra/gamemd evidence:** not applicable; this plan changes tooling only
- **INI/assets:** not applicable; this plan reads no game constants or retail assets

## Post-Plan Self-Review

### Implementation hardening decisions

The implementation review tightened several plan details without expanding MVP scope:

- Scheduler status stripping accepts both `— **done` and `. **done` suffixes.
- Shell LOW import asserts the exact retained ID set, validates the source disposition sentence and
  disposition targets, and emits an `UNNUMBERED_CONFIRMED_ITEMS` diagnostic for folded prose gaps.
- Miner and shell Rust anchors are imported syntactically; current existence is evaluated later so a
  disappeared anchor remains visible as `STALE_MAPPING`.
- Unscoped finding IDs in commit subjects are ignored because roadmap slice labels collide with
  inventory IDs. Only an exact `system: ID` subject plus changed-path/anchor intersection can emit
  Git or test evidence and become `LANDED`; lowercase suffixes such as `S4b` are not scalar IDs.
- Every cited Rust anchor has a regenerated `implementation_anchor` declaration. Git evidence also
  rechecks that a historically changed anchor remains a current file tracked in `HEAD`, so omitting
  path evidence, deleting an anchor, or recreating it as an untracked file cannot preserve `LANDED`.
- Regression declarations carry their originating commit, require the matching scoped association
  to be an ancestor-backed `LANDED` fact, and are rechecked against both the added patch and the full
  Rust file at that commit. Commented or string-literal `#[test]` text is excluded.
- Git ancestry treats exit 1 as a valid non-ancestor result and exit 2 or greater as failure.
- The corpus digest covers source locks, importer identity/version, obligations, dispositions,
  diagnostics, and evidence. All output siblings are staged before commit; the source lock remains
  the final commit marker.
- Bootstrap adapters assert exact region IDs, heading/order structure, declared counts, workstream
  mappings, and dispositions; stale roadmap references remain explicit diagnostics rather than
  silently becoming ownership.
- Runtime decoders and portable schemas use the same closed enums, exact count axes, safe relative
  path contract, unique arrays, nonfatal persisted diagnostics, and same-system provenance rules.
- `check --require-sources` performs a fresh semantic import and compares the complete corpus digest,
  so parser-code drift cannot leave normalized tracked data looking current.
- Default unit discovery uses tracked fixtures and tracked corpus only. Ignored-source enumeration is
  exercised explicitly by import and `check --require-sources`.
- Render output is restricted to a real, non-symlinked `target/` subtree; it cannot overlap the
  tracked corpus or escape through a symlink or junction.
- Re-import determinism is checked with file hashes, including files that are still untracked.

1. **Spec coverage:** all approved design and amendment requirements map to Tasks 1-10.
2. **Vagueness scan:** no unresolved markers or generic deferred work remains; deferred items are
   explicitly out of MVP scope.
3. **Architecture:** no sim/render/assets/gameplay boundary is touched.
4. **Interface ordering:** primitives and schemas precede adapters, corpus, evidence, reducer,
   renderer, data generation, and CI.
5. **Risk coverage:** each parser collision, digest, status-overclaim, and CI-source risk has a named
   test.
6. **Self-containment:** every task names files, interfaces, algorithms, exact source bounds,
   assertions, and verification commands.
7. **Sim compliance:** not applicable; no `src/sim/` file changes.
8. **Grounding:** direct source/docs/current-code evidence is cited; Ghidra and INI are explicitly not
   applicable.
9. **Confidence:** every decision is high-confidence and source-grounded.
10. **Deferred questions:** result receipts, native closure, logger/pixels/nightlies are explicitly
    outside this MVP.
11. **Parity-critical items:** classification, inventory, evidence, and reproducibility details are
    enumerated before tasks.

Recommended execution mode after approval: **batch execution in this session**, Tasks 1-3, 4-6,
7-9, then Task 10, with a verification/status checkpoint after each batch. The path count is high,
but the tool is cohesive and parallel implementation would risk shared-schema/import conflicts.
