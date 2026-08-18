# Ground Movement Native Oracle Offline Substrate Implementation Plan

**Date:** 2026-07-22
**Status:** READY FOR IMPLEMENTATION REVIEW; offline/synthetic scope only
**Authority boundary:** private-oracle source and closed tests; no retail execution or parity certification

## Goal

Implement the bounded offline-ready portion of
`docs/contracts/2026-07-22-ground-movement-native-oracle-capture-implementation-contract.md`:
a canonical ground-movement scenario materializer, a new typed movement wire family,
a separate large-slot movement transport, a fail-closed collector, a fakeable two-lane
coordinator, and an exact movement-specific stability comparator.

The implementation must make the evidence substrate testable without launching or
attaching to `gamemd.exe`, sending native input, injecting a DLL, enrolling tools,
mutating runtime evidence, selecting hook addresses, or changing either runtime CLI
stub. A passing implementation proves only that the offline contracts reject malformed
or incomplete evidence; it is not retail evidence and cannot change parity authority.

## Design Authority and Readiness Mapping

The approved implementation contract is the design authority for this plan. Its
headings do not literally use the write-plan template's two required names, but it
contains both required bodies:

- **Architecture Context** maps to contract sections `Fixed Implementation Decisions`
  and `Required Tooling Changes`: additive movement protocol, dedicated 4,096-byte
  transport, distinct collector, versioned scenario, closed facade, and typed two-lane
  orchestration.
- **Impact Analysis** maps to the 20-row `Parity Delta Table`, A-01 through A-25,
  `Known Non-Requirements`, and `Blockers And Follow-Ups`.

Only O-01, O-02, O-04, O-05, O-06, O-10, O-12, O-13, O-14, and O-15 are ready for
this plan. O-03, O-07, O-08, O-09, O-11, O-17, O-18, O-19, and O-20 remain blocked
and are not hidden follow-up wiring.

The local research index returned no evidence or handoff chunks for this new private-
tooling slice. Therefore the frozen contract, the final Checkpoint-E report, and direct
reads of the private source tree are the planning authority. No Ghidra check is needed
for this offline plan because it introduces no new native address, receiver, field, or
hook claim; all hook selection remains explicitly blocked.

## Frozen Planning Baseline

Public repository at the final pre-implementation-plan recheck:

- HEAD `928644d44fee61d0bb8d3214a4f9e0eb7390bc4e`;
- contract SHA-256
  `F134D218E038407BDF104920FC60BC490FC0DA68B3C5B97E092DFABBDD096A46`,
  659 lines;
- Checkpoint-E report SHA-256
  `EE988EB689C55A3C8F8EF30CCEF20DAA1409EB5536AF67025F3632868E0512C6`;
- object-pass/Drive scheduling report SHA-256
  `5A9E6CB3DE67E3637C001A42EC6C7D34FEFD2AEDA097EFD82BBBCB388038C263`;
- bounded host contract SHA-256
  `4D85178F0EF454AA34472537EF8FA33DB501026C6703897BA1D4A91EB990FD63`;
- Foot Mission_Move report SHA-256
  `FFA7307C17E9DB6176287F48889FAF097F41AB73401595DD7492AAAEAF8BB73C`;
- exact GetCurrentSpeed report SHA-256
  `0A728B262FA8358C6FDE931C93216EC5C7378D51EDC1A07BBD38FBFD4E689683`;
- RawTrack reconciliation SHA-256
  `3B94CF7E896B058CA1ECEBAB69CA63D0B736D7C46AD5D35B137FD6934CCCC93E`;
- Phase-1 locomotor-population/precedence report SHA-256
  `CBE8307F6AF27760A151D0A599C5D7400727840E3C6C2195FFA1598E82ADE37D`;
- ground-movement lifecycle/effect-ownership report SHA-256
  `A4E6DF032FE11EE5E2A2D96399624AE5B19418DFF1E3C1BB683C4DD2ECF765FF`;
- path-buffer source SHA-256
  `6C37DBB7504ABA6F9D1BE7DD01E75ECDB3C07A19BBE8EC3E32F9EB4F9B7B318D`;
- repo retail YR `ini/rulesmd.ini` SHA-256
  `3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF`;
- repo base RA2 `ini/rules.ini` SHA-256
  `FD1E95CEA0306EA78049DC81C8CD816E18C28C496872A1FF02EDD50BD082062F`;
- at the final `2026-07-23T01:46:47+02:00` recheck, the sole unrelated working-tree
  modification is `src/sim/world/techno_ai.rs`; implementation must hash it before work
  and prove the same bytes remain afterward.

Private Oracle repository `<local>/Documents/vera20k-oracle`:

- HEAD `7b8689edd2c5a26ec936caaa03d2c7c9bc31523e`;
- companion-owned modification to
  `docs/plans/2026-07-20-stage-13b-primary-certification-plan.md`;
- companion-owned modifications under `tools/input_provenance_lab/` in
  `controller_core.cpp`, `evidence_writer.cpp`, `live_runner.cpp`,
  `observe_flow_core.cpp/.h`, `observe_terminal_core.cpp/.h`,
  `reconciliation.cpp`, and `tests/test_main.cpp`;
- `create-checkpoint` and `run-original` are still bound to `_not_implemented`;
- `status --json` reports `parity_authority=NONE` and the instrumentation,
  original-YR execution, and cross-engine pipelines `BLOCKED`;
- `doctor` remains `PRECHECK_FAILED` with `LOCAL_TOOL_LOCK_INVALID`,
  `NATIVE_CAPTURE_INVALID`, and `NATIVE_CAPTURE_UNENROLLED`.

Re-run both repository status checks before implementation. If any planned private path
has become dirty or acquired another owner, stop and obtain a path-level handoff rather
than merging concurrent changes into this slice.

## Compact Task Contract

| Item | Contract |
|---|---|
| Goal | Build only the offline movement-evidence substrate and its rejection tests. |
| Necessary scope | New scenario/policy schemas and pure materializers; movement-v3 codec; inert movement transport; synthetic collector; fake two-lane coordinator; movement comparator; focused regression docs/tests. |
| Parity constraint | Preserve every raw byte and run-local measurement; compare only explicitly classified deterministic fields; never call offline consistency retail parity. |
| Smallest validation | Strict Python tests, independent golden-vector check, serial locked Cargo tests in external targets, status/capability regression, and a final source/ownership audit. |
| Stop condition | Offline tests and cold review pass while runtime commands remain STUB, pipelines remain BLOCKED, parity authority remains NONE, and no live/native/evidence operation occurred. |

## Architecture Context

The implementation is additive and keeps the current owners:

```text
checkpoint TransitionProofs
        |
        v
pure scenario materializer ----> canonical scenario bytes + SHA-256
        |                                      |
        |                                      +----------------------+
        v                                                             |
typed lane plan                                                       |
  |                                                                   |
  +-- 2 x InstrumentedState lane --> movement transport --> raw --> normalized
  |             (fake only here)          (synthetic)       |          |
  |                                                         +----------+
  +-- 3 x CleanPresentation lane --> existing clean evidence model     |
                (fake only here)                                       |
                                                                        v
                                                       movement cohort comparator
                                                       (consistency, never parity)
```

`OriginalYrWorkflow` remains the sole YR state-machine owner,
`WorkflowController` remains the transition/cleanup/no-retry owner, and `RunBundle`
remains the only mutable-then-sealed artifact owner. The new coordinator composes them;
it does not recreate their responsibilities. The existing legacy/startup codecs,
transports, collectors, comparator, callback shims, hook manifests, registry, and CLI
remain byte- and behavior-compatible.

## Impact Analysis

| Surface | Planned impact | Forbidden impact |
|---|---|---|
| Scenario contracts | Add strict `ground-movement-scenario.v1`, including explicit negative-control declarations, plus pure canonical materialization | No fabricated finalized MTNK scenario, no silent reroll, no mutation of `scenario.v1` |
| Protocol | Add movement version 3 and kinds 12-20 with explicit little-endian codecs | No reinterpretation or byte change to versions 1/2, StateSnapshot v2, or startup v2 |
| Instrument transport | Add a distinct private movement mapping implementation with 4,096/16/4,080 geometry | No callback/export/build-script/manifest hook, no refactor of legacy/startup transports |
| Collector | Add a separate movement decoder/collector with synthetic pagefile-mapping tests | No import from status/capabilities, no target launch, attach, injection, or evidence enrollment |
| Workflow | Add an injected-factory offline coordinator and proof reconciler | No concrete Win32/raw-input backend and no CLI handler wiring |
| Comparison | Add exhaustive field-role classification, two-instrumented exact comparison, three-clean reuse, and raw cadence reporting | No tolerance, lossy normalization, lane substitution, VERA adapter, or parity verdict |
| Artifacts | Validate lane-specific required files/classes on top of `RunBundle` | No second bundle format, repair mode, evidence rewrite, or weakened seal verification |
| Public engine | None | No public Rust/INI/assets/render/sim edits, Cargo, snapshot bump, or movement activation |

## Key Technical Decisions

- **Additive offline substrate only:** private modules and closed tests are added without
  activating either runtime command. **Confidence: high.** Source: approved contract
  O-01/O-02/O-04/O-05/O-06/O-10/O-12/O-13/O-14/O-15 and current registry/CLI reads.
- **Explicit protocol v3 plus dedicated transport:** movement records do not overload
  v1/v2 or either existing transport. **Confidence: high.** Source: contract §§Fixed
  Implementation Decisions/Required Tooling Changes and startup source pattern.
- **Immediate proof validation plus completed-event reconciliation:** checkpoint proof
  corruption stops at the producing edge and is checked again at materialization.
  **Confidence: high.** Source: A-03 and `OriginalYrWorkflow`/`WorkflowController` call
  order.
- **Strict normalized event and lane-identity schemas:** runtime values remain lossless,
  while full executable/map/tool/environment identity is sealed separately from each
  record. **Confidence: high.** Source: Checkpoint E §§7, 10 and A-12/A-22.
- **Table-driven bounded Unit/Foot host DFA:** the decoder proves the verified pilot path
  only through Foot return and does not invent Unit-tail semantics. **Confidence: high.**
  Source: `TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md`.
- **Fake collector boundaries use the existing controller:** instrumented collection
  starts before `identify_window`, clean collection before `select_mtnk`, both stop before
  `exit`. **Confidence: high.** Source: direct current-source check of controller boundary
  validation and standard-move edges. This intentionally makes no live pre-launch claim.

## Open Questions

### Resolved During Planning

- The schema registry must use function-local imports; eager imports create a cycle and
  can contaminate read-only status commands.
- `identify_window` is the earliest legal fake instrumented collector boundary after
  `launch`; live pre-launch mapping preparation cannot use the current binding API.
- Comparator requirements come only from the sealed scenario/policy, never a caller that
  could weaken completeness.
- The public repository baseline is HEAD
  `928644d44fee61d0bb8d3214a4f9e0eb7390bc4e`, with only the companion-owned
  `src/sim/world/techno_ai.rs` dirty at the final planning recheck.

### Deferred Beyond This Plan

- Exact native hook IDs, addresses, receivers, overwrite windows, continuations and hit
  bounds remain O-07/O-08/O-09.
- Live pre-launch transport preparation, injection, enrollment, and runtime CLI ownership
  remain O-03/O-11/O-17/O-18/O-19/O-20.
- Loader-owned `FS:[0x14]` preservation and observer non-perturbation remain separate
  reviewed work.
- Actual retail MTNK state, pixel, cadence and GameSpeed distributions remain unknown
  until an authorized executable oracle exists.

## Risk Areas and Parity-Critical Evidence

| Task | Risk / exact-mechanism item | Required proof |
|---:|---|---|
| 1 | A malformed checkpoint proof could permit later native-input transitions | Mutate every operation-specific proof and prove zero later backend calls |
| 2 | One width, discriminant, reserved byte or IEEE-754 bit interpretation could make Python/Rust disagree | Independent frozen corpus plus exhaustive enum/flag matrices |
| 3–4 | Publishing or acknowledging too early could seal torn or non-durable evidence | Exact release/xchg order; raw and normalized fsync before fenced acknowledgement |
| 4 | Host/Unit/Foot event reorder could make a plausible but false trace look valid | DFA branch corpus and swapped-valid-event rejection |
| 4 | RNG rejection can consume multiple raw words inside one API call | Full logical-state boundaries plus ordered candidate/draw indices |
| 5 | Retry or result-normalization mistakes could send input twice or seal failure as success | Select/issue partial-failure tests and normalized-result gate |
| 5–6 | Lane/identity substitution could compare different executable, map, tool or environment states | Versioned lane identity and all-ten-hash reconciliation |
| 6 | A caller could omit evidence and still obtain a consistency label | Requirements derived from immutable scenario/policy; missing surface is capture-invalid |
| 7 | Parallel work could be overwritten or silently staged | Pre/post hashes, HEAD/status guards, planned-path allowlist, no staging |

These are parity-critical evidence mechanics, but passing their closed tests is not a
parity result. No task touches `sim/`; deterministic state-hash, fixed-point, tick-order,
and `BTreeMap` checks are therefore not applicable to this private tooling plan.

## Non-Negotiable Invariants

1. `tools/oracle_harness/oracle.py` remains the supported facade, but neither runtime
   stub is activated by this slice.
2. `tools/oracle_harness/oracle-system.v1.json`, `oracle_harness/cli.py`,
   `tools/oracle_instrument/build.rs`, callback shims, generated/enrolled manifests,
   native-capture enrollment, Stage-13B, and `input_provenance_lab` are read-only.
3. The protocol stores explicit little-endian fields. Rust struct layout is never wire
   authority.
4. Every record fits one 4,080-byte slot. Zero-length, oversized, fragmented, partial,
   overwritten, or CRC-invalid records fail before acknowledgement.
5. The collector durably appends the exact raw record before writing a normalized event.
   Normalization adds aliases and hashes; it never deletes raw pointers, raw widths,
   rejection candidates, QPC values, or occurrence order.
6. Instrumented and clean lanes use different nominal types and required artifact sets.
   Neither can satisfy the other's requirements.
7. Every leaf in the normalized movement-event and lane-identity schemas is classified
   by one versioned policy as `deterministic`, `run_local`, or `measurement`. Unknown and
   multiply classified leaves invalidate the comparison. Clean evidence remains the
   existing typed `OriginalRunEvidence`/`SequenceRecord.comparable` projection; its
   scenario contract enumerates exact prerequisite, sequence, evidence-kind, and
   comparable-field names. It is not pretended to have a JSON schema that does not exist.
8. Absolute QPC timestamps, raw native pointers, process/thread IDs, and run IDs remain
   sealed evidence but are not required to equal across fresh processes. No tolerance is
   introduced for deterministic fields.
9. Once a workflow operation may deliver input, the existing no-retry/no-rewind rules
   remain in force even in fake tests.
10. No test result in this plan upgrades a native claim to VERIFIED or changes production
    movement authority.

## Exact File Map

All implementation paths below are inside
`<local>/Documents/vera20k-oracle` unless explicitly prefixed `public:`.

### Create

- `tools/oracle_harness/schemas/ground-movement-scenario.v1.schema.json`
- `tools/oracle_harness/schemas/ground-movement-field-roles.v1.schema.json`
- `tools/oracle_harness/schemas/ground-movement-event.v1.schema.json`
- `tools/oracle_harness/schemas/ground-movement-lane-identity.v1.schema.json`
- `tools/oracle_harness/policies/ground-movement-field-roles.v1.json` (create the absent
  `policies/` parent directory as part of Task 6)
- `tools/oracle_harness/oracle_harness/ground_movement_scenario.py`
- `tools/oracle_harness/oracle_harness/movement_wire.py`
- `tools/oracle_harness/oracle_harness/collectors/oracle_movement.py`
- `tools/oracle_harness/oracle_harness/movement_orchestration.py`
- `tools/oracle_harness/oracle_harness/movement_compare.py`
- `tools/oracle_harness/tests/test_ground_movement_scenario.py`
- `tools/oracle_harness/tests/test_movement_wire.py`
- `tools/oracle_harness/tests/test_oracle_movement.py`
- `tools/oracle_harness/tests/test_movement_orchestration.py`
- `tools/oracle_harness/tests/test_movement_compare.py`
- `tools/oracle_protocol/src/movement_v3.rs`
- `tools/oracle_protocol/ground_movement_v3_golden.hex`
- `tools/oracle_protocol/verify_movement_vector.py`
- `tools/oracle_instrument/src/movement_transport.rs`

### Modify

- `tools/oracle_harness/oracle_harness/schema.py` — register the four new strict
  portable contracts and delegate their nested validation.
- `tools/oracle_harness/oracle_harness/collectors/__init__.py` — list the movement
  collector without importing it eagerly.
- `tools/oracle_harness/tests/test_schema.py` — update schema count/parity assertions.
- `tools/oracle_harness/tests/test_system_cli.py` — add the movement collector to the
  forbidden live-import set used by read-only status/capability tests.
- `tools/oracle_protocol/src/lib.rs` — expose `movement_v3`; do not renumber old kinds.
- `tools/oracle_protocol/README.md` — document movement v3 and explicitly state that no
  live producer exists.
- `tools/oracle_instrument/src/lib.rs` — add only a private, uncalled
  `mod movement_transport;`; add no export or callback body.

### Explicitly read-only

- `tools/oracle_harness/oracle_harness/cli.py`
- `tools/oracle_harness/oracle.py`
- `tools/oracle_harness/oracle-system.v1.json`
- `tools/oracle_harness/oracle_harness/system_registry.py`
- `tools/oracle_harness/oracle_harness/system_status.py`
- `tools/oracle_harness/oracle_harness/adapters/original_yr.py`
- `tools/oracle_harness/oracle_harness/controller.py`
- `tools/oracle_harness/oracle_harness/artifacts.py`
- `tools/oracle_harness/oracle_harness/compare.py`
- `tools/oracle_harness/oracle_harness/evidence.py`
- `tools/oracle_harness/oracle_harness/pixels.py`
- `tools/oracle_harness/oracle_harness/collectors/oracle_instrument.py`
- `tools/oracle_harness/oracle_harness/collectors/oracle_startup.py`
- `tools/oracle_protocol/src/snapshot_v2.rs`
- `tools/oracle_protocol/src/startup_v2.rs`
- `tools/oracle_protocol/Cargo.toml`
- `tools/oracle_protocol/Cargo.lock`
- `tools/oracle_instrument/src/transport.rs`
- `tools/oracle_instrument/src/startup_transport.rs`
- `tools/oracle_instrument/src/callback_shim.rs`
- `tools/oracle_instrument/build.rs`
- `tools/oracle_instrument/Cargo.toml`
- `tools/oracle_instrument/Cargo.lock`
- `tools/oracle_protocol/verify_vector.py`
- `tools/oracle_protocol/state_snapshot_v2_golden.hex`
- `tools/oracle_instrument/tests/test_smoke.py`
- `tools/oracle_harness/tests/test_checkpoint_recipe.py`
- `tools/oracle_harness/tests/test_original_yr_adapter.py`
- `tools/oracle_harness/tests/test_controller.py`
- `tools/oracle_harness/tests/test_artifacts.py`
- `tools/oracle_harness/tests/test_compare.py`
- `tools/oracle_harness/tests/test_oracle_startup.py`
- `tools/oracle_harness/tests/test_oracle_instrument.py`
- `tools/oracle_harness/tests/test_system_status.py`
- all `tools/input_provenance_lab/**` and Stage-13B paths
- the public VERA20k engine, INI files, assets, research reports, and contracts

### Pre-implementation ownership guard

Before Task 1, the executing coordinator—not a worker—must run this read-only snapshot.
It uses one deterministic temporary path, refuses to overwrite a retained guard, requires
the exact normalized public/private status baselines, requires every planned private path
to be clean, and fingerprints every known companion-owned or read-only authority file.
This matters because the plan, contract, and research reports are ignored by public Git;
HEAD/status alone cannot detect edits to them.

The final reviewed handoff supplies this plan's exact SHA-256. Before running the block,
set process environment variable `VERA20K_REVIEWED_PLAN_SHA256` to that value; a missing,
malformed, or mismatching value is a hard stop.

```powershell
$publicRoot = '.'
$privateRoot = '<local>/Documents/vera20k-oracle'
$publicDirty = @(
  'src/sim/world/techno_ai.rs'
)
$publicReadOnly = @(
  'docs/plans/2026-07-22-ground-movement-native-oracle-offline-substrate-plan.md',
  'docs/contracts/2026-07-22-ground-movement-native-oracle-capture-implementation-contract.md',
  'docs/research/GROUND_MOVEMENT_EXECUTABLE_NATIVE_ORACLE_CAPTURE_REPORT.md',
  'docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md',
  'docs/research/TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md',
  'docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md',
  'docs/research/FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md',
  'docs/research/DRIVE_RAWTRACK_METADATA_INITIALIZER_RECONCILIATION_GHIDRA_REPORT.md',
  'docs/research/GROUND_PHASE1_LOCOMOTOR_POPULATION_AND_PRECEDENCE_GHIDRA_REPORT.md',
  'docs/research/GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP_GHIDRA_REPORT.md',
  'docs/research/BRIDGE_TRAVERSAL_STATE_GHIDRA_REPORT.md',
  'ini/rulesmd.ini',
  'ini/rules.ini'
)
$privateDirty = @(
  'docs/plans/2026-07-20-stage-13b-primary-certification-plan.md',
  'tools/input_provenance_lab/src/controller_core.cpp',
  'tools/input_provenance_lab/src/evidence_writer.cpp',
  'tools/input_provenance_lab/src/live_runner.cpp',
  'tools/input_provenance_lab/src/observe_flow_core.cpp',
  'tools/input_provenance_lab/src/observe_flow_core.h',
  'tools/input_provenance_lab/src/observe_terminal_core.cpp',
  'tools/input_provenance_lab/src/observe_terminal_core.h',
  'tools/input_provenance_lab/src/reconciliation.cpp',
  'tools/input_provenance_lab/tests/test_main.cpp'
)
$privateReadOnly = @(
  'tools/oracle_harness/oracle_harness/cli.py',
  'tools/oracle_harness/oracle.py',
  'tools/oracle_harness/oracle-system.v1.json',
  'tools/oracle_harness/oracle_harness/system_registry.py',
  'tools/oracle_harness/oracle_harness/system_status.py',
  'tools/oracle_harness/oracle_harness/adapters/original_yr.py',
  'tools/oracle_harness/oracle_harness/controller.py',
  'tools/oracle_harness/oracle_harness/artifacts.py',
  'tools/oracle_harness/oracle_harness/compare.py',
  'tools/oracle_harness/oracle_harness/evidence.py',
  'tools/oracle_harness/oracle_harness/pixels.py',
  'tools/oracle_harness/oracle_harness/collectors/oracle_instrument.py',
  'tools/oracle_harness/oracle_harness/collectors/oracle_startup.py',
  'tools/oracle_protocol/src/snapshot_v2.rs',
  'tools/oracle_protocol/src/startup_v2.rs',
  'tools/oracle_protocol/Cargo.toml',
  'tools/oracle_protocol/Cargo.lock',
  'tools/oracle_protocol/state_snapshot_v2_golden.hex',
  'tools/oracle_protocol/verify_vector.py',
  'tools/oracle_instrument/src/transport.rs',
  'tools/oracle_instrument/src/startup_transport.rs',
  'tools/oracle_instrument/src/callback_shim.rs',
  'tools/oracle_instrument/build.rs',
  'tools/oracle_instrument/Cargo.toml',
  'tools/oracle_instrument/Cargo.lock',
  'tools/oracle_instrument/tests/test_smoke.py',
  'tools/oracle_harness/tests/test_checkpoint_recipe.py',
  'tools/oracle_harness/tests/test_original_yr_adapter.py',
  'tools/oracle_harness/tests/test_controller.py',
  'tools/oracle_harness/tests/test_artifacts.py',
  'tools/oracle_harness/tests/test_compare.py',
  'tools/oracle_harness/tests/test_oracle_startup.py',
  'tools/oracle_harness/tests/test_oracle_instrument.py',
  'tools/oracle_harness/tests/test_system_status.py'
)
$plannedCreate = @(
  'tools/oracle_harness/schemas/ground-movement-scenario.v1.schema.json',
  'tools/oracle_harness/schemas/ground-movement-field-roles.v1.schema.json',
  'tools/oracle_harness/schemas/ground-movement-event.v1.schema.json',
  'tools/oracle_harness/schemas/ground-movement-lane-identity.v1.schema.json',
  'tools/oracle_harness/policies/ground-movement-field-roles.v1.json',
  'tools/oracle_harness/oracle_harness/ground_movement_scenario.py',
  'tools/oracle_harness/oracle_harness/movement_wire.py',
  'tools/oracle_harness/oracle_harness/collectors/oracle_movement.py',
  'tools/oracle_harness/oracle_harness/movement_orchestration.py',
  'tools/oracle_harness/oracle_harness/movement_compare.py',
  'tools/oracle_harness/tests/test_ground_movement_scenario.py',
  'tools/oracle_harness/tests/test_movement_wire.py',
  'tools/oracle_harness/tests/test_oracle_movement.py',
  'tools/oracle_harness/tests/test_movement_orchestration.py',
  'tools/oracle_harness/tests/test_movement_compare.py',
  'tools/oracle_protocol/src/movement_v3.rs',
  'tools/oracle_protocol/ground_movement_v3_golden.hex',
  'tools/oracle_protocol/verify_movement_vector.py',
  'tools/oracle_instrument/src/movement_transport.rs'
)
$plannedModify = @(
  'tools/oracle_harness/oracle_harness/schema.py',
  'tools/oracle_harness/oracle_harness/collectors/__init__.py',
  'tools/oracle_harness/tests/test_schema.py',
  'tools/oracle_harness/tests/test_system_cli.py',
  'tools/oracle_protocol/src/lib.rs',
  'tools/oracle_protocol/README.md',
  'tools/oracle_instrument/src/lib.rs'
)
$plannedPrivate = @($plannedCreate + $plannedModify)
$expectedPublicStatus = @(
  ' M src/sim/world/techno_ai.rs'
)
$expectedPrivateStatus = @(
  ' M docs/plans/2026-07-20-stage-13b-primary-certification-plan.md',
  ' M tools/input_provenance_lab/src/controller_core.cpp',
  ' M tools/input_provenance_lab/src/evidence_writer.cpp',
  ' M tools/input_provenance_lab/src/live_runner.cpp',
  ' M tools/input_provenance_lab/src/observe_flow_core.cpp',
  ' M tools/input_provenance_lab/src/observe_flow_core.h',
  ' M tools/input_provenance_lab/src/observe_terminal_core.cpp',
  ' M tools/input_provenance_lab/src/observe_terminal_core.h',
  ' M tools/input_provenance_lab/src/reconciliation.cpp',
  ' M tools/input_provenance_lab/tests/test_main.cpp'
)
function Get-GuardHashes([string]$root, [string[]]$paths) {
  $result = [ordered]@{}
  foreach ($relative in $paths) {
    $literal = Join-Path $root $relative
    if (-not (Test-Path -LiteralPath $literal -PathType Leaf)) {
      throw "ownership-guard path is missing: $literal"
    }
    $result[$relative] = (Get-FileHash -Algorithm SHA256 -LiteralPath $literal).Hash
  }
  return $result
}
function Assert-ExactStatus([string[]]$actual, [string[]]$expected, [string]$label) {
  $actualNormalized = @($actual | Sort-Object)
  $expectedNormalized = @($expected | Sort-Object)
  if (($actualNormalized | ConvertTo-Json -Compress) -ne
      ($expectedNormalized | ConvertTo-Json -Compress)) {
    throw "$label status differs from the reviewed baseline"
  }
}
$publicHead = git -C $publicRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'cannot read public HEAD' }
$publicStatus = @(git -C $publicRoot status --porcelain=v1 -uall)
if ($LASTEXITCODE -ne 0) { throw 'cannot read public status' }
$privateHead = git -C $privateRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'cannot read private HEAD' }
$privateStatus = @(git -C $privateRoot status --porcelain=v1 -uall)
if ($LASTEXITCODE -ne 0) { throw 'cannot read private status' }
if ($publicHead -ne '928644d44fee61d0bb8d3214a4f9e0eb7390bc4e') {
  throw 'public baseline changed; re-read and re-review the plan'
}
if ($privateHead -ne '7b8689edd2c5a26ec936caaa03d2c7c9bc31523e') {
  throw 'private baseline changed; re-read and re-review the plan'
}
Assert-ExactStatus $publicStatus $expectedPublicStatus 'public'
Assert-ExactStatus $privateStatus $expectedPrivateStatus 'private'
$plannedStatus = @(git -C $privateRoot status --porcelain=v1 -uall -- $plannedPrivate)
if ($LASTEXITCODE -ne 0) { throw 'cannot inspect planned private paths' }
if ($plannedStatus.Count -ne 0) { throw 'a planned private path is already dirty' }
foreach ($relative in $plannedCreate) {
  if (Test-Path -LiteralPath (Join-Path $privateRoot $relative)) {
    throw "planned create path already exists: $relative"
  }
}
$expectedReviewedPlanHash = [Environment]::GetEnvironmentVariable(
  'VERA20K_REVIEWED_PLAN_SHA256', 'Process'
)
if ($expectedReviewedPlanHash -notmatch '^[0-9A-Fa-f]{64}$') {
  throw 'VERA20K_REVIEWED_PLAN_SHA256 is missing or malformed'
}
$expectedPublicAuthorityHashes = [ordered]@{
  'docs/plans/2026-07-22-ground-movement-native-oracle-offline-substrate-plan.md' = $expectedReviewedPlanHash.ToUpperInvariant()
  'docs/contracts/2026-07-22-ground-movement-native-oracle-capture-implementation-contract.md' = 'F134D218E038407BDF104920FC60BC490FC0DA68B3C5B97E092DFABBDD096A46'
  'docs/research/GROUND_MOVEMENT_EXECUTABLE_NATIVE_ORACLE_CAPTURE_REPORT.md' = 'EE988EB689C55A3C8F8EF30CCEF20DAA1409EB5536AF67025F3632868E0512C6'
  'docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md' = '5A9E6CB3DE67E3637C001A42EC6C7D34FEFD2AEDA097EFD82BBBCB388038C263'
  'docs/research/TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md' = '4D85178F0EF454AA34472537EF8FA33DB501026C6703897BA1D4A91EB990FD63'
  'docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md' = 'FFA7307C17E9DB6176287F48889FAF097F41AB73401595DD7492AAAEAF8BB73C'
  'docs/research/FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md' = '0A728B262FA8358C6FDE931C93216EC5C7378D51EDC1A07BBD38FBFD4E689683'
  'docs/research/DRIVE_RAWTRACK_METADATA_INITIALIZER_RECONCILIATION_GHIDRA_REPORT.md' = '3B94CF7E896B058CA1ECEBAB69CA63D0B736D7C46AD5D35B137FD6934CCCC93E'
  'docs/research/GROUND_PHASE1_LOCOMOTOR_POPULATION_AND_PRECEDENCE_GHIDRA_REPORT.md' = 'CBE8307F6AF27760A151D0A599C5D7400727840E3C6C2195FFA1598E82ADE37D'
  'docs/research/GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP_GHIDRA_REPORT.md' = 'A4E6DF032FE11EE5E2A2D96399624AE5B19418DFF1E3C1BB683C4DD2ECF765FF'
  'docs/research/BRIDGE_TRAVERSAL_STATE_GHIDRA_REPORT.md' = '6C37DBB7504ABA6F9D1BE7DD01E75ECDB3C07A19BBE8EC3E32F9EB4F9B7B318D'
  'ini/rulesmd.ini' = '3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF'
  'ini/rules.ini' = 'FD1E95CEA0306EA78049DC81C8CD816E18C28C496872A1FF02EDD50BD082062F'
}
foreach ($entry in $expectedPublicAuthorityHashes.GetEnumerator()) {
  $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (
    Join-Path $publicRoot $entry.Key
  )).Hash
  if ($actual -ne $entry.Value) {
    throw "reviewed public authority hash changed: $($entry.Key)"
  }
}
$ownershipGuardPath = Join-Path ([IO.Path]::GetTempPath()) (
  'vera20k-ground-movement-offline-ownership-guard-v1.json'
)
if (Test-Path -LiteralPath $ownershipGuardPath) {
  throw "retained ownership guard already exists: $ownershipGuardPath"
}
$guard = [ordered]@{
  schema = 'vera20k.ground-movement-offline-ownership-guard.v1'
  public_head = $publicHead
  public_status = @($publicStatus | Sort-Object)
  public_dirty_hashes = Get-GuardHashes $publicRoot $publicDirty
  public_read_only_hashes = Get-GuardHashes $publicRoot $publicReadOnly
  expected_reviewed_plan_sha256 = $expectedReviewedPlanHash.ToUpperInvariant()
  private_head = $privateHead
  private_status = @($privateStatus | Sort-Object)
  private_dirty_hashes = Get-GuardHashes $privateRoot $privateDirty
  private_read_only_hashes = Get-GuardHashes $privateRoot $privateReadOnly
  planned_create = $plannedCreate
  planned_private = $plannedPrivate
}
$guard | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ownershipGuardPath -Encoding UTF8
Write-Host "ownership guard: $ownershipGuardPath"
```

If either repository baseline/status differs before Task 1, any authority hash differs,
any create path exists, or any planned private path is dirty, stop for an ownership
recheck. The temp JSON is only a source-ownership guard, not runtime evidence and not a
RunBundle. Retain it at the exact deterministic path until Task 7 removes it.

## Data Contract Decisions

### Scenario document

`ground-movement-scenario.v1` has exactly these top-level keys, in this logical
contract even though canonical JSON sorts them on disk:

```python
GROUND_MOVEMENT_SCENARIO_FIELDS = (
    "schema_version",
    "scenario_id",
    "source_recipe",
    "retail_identity",
    "checkpoint",
    "fixture",
    "action",
    "attempt_policy",
    "negative_controls",
    "capture_contract",
    "lanes",
    "comparison_policy",
    "timeouts_ms",
    "artifact_classes",
)
```

Every object in the portable JSON schema sets `additionalProperties: false`.
The document records full SHA-256 identities; it never treats a raw native pointer as
cross-run fixture identity. `fixture.capture_local_object_id` is semantic and stable,
and is exactly integer `1` for this sole pilot object, while each run later records its
raw pointer and LogicVector occurrence. The materializer rejects any other local ID.

The lane contract is literal:

```python
LANE_COUNTS = {
    "instrumented-state": 2,
    "clean-presentation": 3,
}
```

The attempt ledger uses contiguous `attempt_index` values starting at zero, records
every candidate outcome, and contains exactly one accepted terminal candidate. A caller
cannot omit an attempt, request an invisible reroll, or replace a candidate after
canonicalization.

`negative_controls` is a nonempty, ordered array. Each strict entry has exactly
`control_id`, `distinguishing_branch`, `companion_scenario_id`,
`companion_scenario_sha256`, `lane`, `expected_branch_taken`, and
`required_artifact_classes`. IDs use the same stable-slug grammar; the companion hash is
a full nonzero lowercase SHA-256; `lane` is literally `clean-presentation`; and
`expected_branch_taken` is literally `false`. Control IDs and companion scenario IDs are
unique, artifact classes are sorted/unique, and a control may not change the primary
attempt ledger or silently reroll its fixture. This offline slice binds the declarations
canonically but does not execute companion native scenarios; later fixture expansion
must supply their separately sealed bundles before claiming that discriminator family.

Every `capture_contract.hook_bindings` member has exact keys `hook_id`,
`static_address`, `runtime_address_relation`, `scope`, and `receiver_relation`.
`receiver_relation` is the strict object `{ "kind": <enum>, "adjustment": <i32> }`:
kind `none` is legal only with scope `global` and adjustment zero; kind
`complete_object` is legal only with scope `object` and adjustment zero; kind
`signed_adjustment` is legal only with scope `object` and a nonzero adjustment. The
runtime relation remains the already required module-relocation relation. Tests use
conspicuous synthetic hook IDs/addresses; no member chooses a real gamemd hook or closes
O-07.

### Field-role policy

The policy has schema `ground-movement-field-roles.v1`, one `policy_id`, explicit
`surfaces = ("movement_event", "lane_identity")`, and three disjoint sorted
JSON-pointer lists per surface: `deterministic`, `run_local`, and `measurement`. It also
has `required_event_kinds` and `required_artifact_classes`. No glob, prefix,
regular-expression, indexed occurrence, or fallback classification is accepted. The
comparator walks every leaf in each finite strict schema, escapes its JSON pointer, and
fails if the pointer is absent or appears in more than one role. Policy paths are
record-relative or lane-document-relative leaf paths, not occurrence-array indices, so
adding a second occurrence cannot evade classification. Fixed arrays that are one
lossless protocol value are normalized atomically: the 1,016-byte RNG state is one
lowercase `logical_state_hex` leaf, and hashes/correlations remain one fixed-width hex
leaf each. Every other scalar or fixed-array member is a separately named leaf. The
scenario binds the policy's full SHA-256 and exact required surfaces; there is no
caller-supplied weaker policy. Clean presentation evidence is deliberately outside this
JSON-pointer policy: the strict scenario's `comparison_policy.clean_requirements`
enumerates exact prerequisite names/evidence kinds and sequence names/evidence kinds/
sorted `comparable_fields`. Task 6 requires every clean record's `comparable` mapping to
have exactly its declared keys, builds `OriginalRunEvidence`, and passes the resulting
finite scenario-derived `StabilityRequirements` to `compare_original_runs`.

### Normalized event and lane-identity documents

`ground-movement-event.v1` has exactly these top-level keys:

```python
GROUND_MOVEMENT_EVENT_FIELDS = (
    "schema_version",
    "run_id",
    "scenario_id",
    "scenario_sha256",
    "lane",
    "sequence",
    "kind",
    "header",
    "payload",
    "raw",
    "aliases",
)
```

The schema uses one `kind`-selected strict payload variant for each protocol record and
sets `additionalProperties: false` at every level. `raw` contains exactly the source
artifact path, byte offset, byte length, and SHA-256. `aliases` is additive and contains
`frame_delta_from_capture_start`, exactly
`raw_original_frame.wrapping_sub(capture_start_original_frame)`, plus an optional
raw-pointer-to-capture-local-ID mapping
only when that pointer appeared in the same record. The only accepted local ID is `1`.
Aliases never replace raw values. The event schema therefore preserves the exact wire
value while giving the policy deterministic frame/object names.

`ground-movement-lane-identity.v1` is written at
`identity/ground-movement-lane-identity.v1.json` with artifact class
`ground_movement_lane_identity`. It has exactly these top-level keys:

```python
GROUND_MOVEMENT_LANE_IDENTITY_FIELDS = (
    "schema_version",
    "run_id",
    "scenario_id",
    "scenario_sha256",
    "lane",
    "repeat_index",
    "created_utc",
    "retail_identity",
    "oracle_identity",
    "environment_identity",
    "process_identity",
    "artifact_policy_sha256",
)
```

The nested strict objects record:

- `retail_identity`: executable canonical path, byte size, SHA-256 and file version;
  map filename, byte size and SHA-256; and `rulesmd.ini`/base `rules.ini` sizes and
  SHA-256 values;
- `oracle_identity`: private Oracle HEAD; a dirty-scope disposition plus sorted path/
  SHA-256 entries; tool-spec, hook-manifest, producer and collector identities; and the
  callback schema/module identity. A clean lane explicitly records that it has no
  movement callback schema and binds its clean tool manifest instead;
- `environment_identity`: sealed environment ID/hash, capture UTC timestamp, QPC
  frequency, and the observed live GameSpeed byte in inclusive range `0..6`;
- `process_identity`: run-local process ID and module base.

The comparator validates this document before any event comparison. The scenario and
lane document must agree exactly on retail hashes, expected policy hash, lane and repeat;
the two instrumented identities must agree on their movement tooling, and the three clean
identities on their clean tooling. Process IDs, module bases and timestamps are retained
run-local values, never grounds for equality.

### Movement protocol identity

Movement records retain `V20O` magic, use protocol version `3`, and allocate kinds
12 through 20 without changing kinds 1 through 11:

| Kind | Record |
|---:|---|
| 12 | `CaptureStart` |
| 13 | `CommandConsumed` |
| 14 | `ObjectSnapshot` |
| 15 | `HostEvent` |
| 16 | `DriveEvent` |
| 17 | `RngEvent` |
| 18 | `CompletedTick` |
| 19 | `CaptureEnd` |
| 20 | `CaptureError` |

Every cross-language discriminant is part of protocol v3 and is frozen below. Rust uses
the stated `#[repr]`; Python accepts only the same integers. No enum has an `Other`
variant.

| Type / width | Exact values |
|---|---|
| `MovementOrigin` / `u32` | `NativeHook=1` |
| `MovementStatus` / `u8` | `Observed=0`, `Accepted=1`, `Completed=2`, `TerminalError=3` |
| `MovementReason` / `u16` | `None=0`, `RecordTooLarge=1`, `RingFull=2`, `SequenceExhausted=3`, `InvalidConsumerCursor=4`, `PublishFailed=5`, `WrongThread=6`, `WrongIdentity=7`, `ProtocolInvalid=8`, `Overflow=9`, `ForcedTeardown=10`, `RecordBudgetExhausted=11` |
| `MovementLane` / `u8` | `InstrumentedState=1`; clean lanes emit no movement wire records |
| `LocomotorKind` / `u8` | `Drive=1` |
| `CommandKind` / `u16` | `MoveToCell=1` |
| `CommandResult` / `u16` | `Consumed=1` |
| `WireBool` / `u8` | `False=0`, `True=1` |
| `TriState` / `u8` | `NotApplicable=0`, `Failed=1`, `Passed=2` |

The record flag word is exactly `COMPLETE=0x0000_0001`; the legal mask is exactly
`0x0000_0001`, and every published record must set that bit. Status/reason combinations
are exhaustive: `CommandConsumed` uses `Accepted/None`; `CaptureEnd` uses
`Completed/None`; `CaptureError` uses `TerminalError` plus exactly one nonzero reason;
all other kinds use `Observed/None`. Any other pair fails. Every movement record uses
origin `NativeHook=1`; the scenario binds that expected origin, and both codec and
collector reject zero or any other value.

Snapshot boundaries are `u16`: `CaptureStartPreCommand=1`,
`CommandDispatchPost=2`, `ObjectPassEnter=3`, `ObjectAiReturn=4`,
`MissionDispatchPre=5`, `MissionDispatchPost=6`, `DriveProcessEnter=7`,
`DriveTrackAttempt=8`, `DriveTrackReturn=9`, `CellCross=10`, `Arrival=11`,
`ObjectPassReturn=12`, and `CompletedTick=13`.

The `ObjectSnapshot` byte at payload offset 3 is the exact presence mask
`ACTIVE_VALID=0x01`, `LIMBO_VALID=0x02`, `LOGIC_VALID=0x04`,
`MISSION_VALID=0x08`, `TARGET_PATH_VALID=0x10`, `FACING_VALID=0x20`,
`DRIVE_VALID=0x40`, and `OCCUPANCY_VALID=0x80`; no other bit is legal. Its host-outcome
word at offset 120 is `TIMER_DUE_VALID=0x001`, `HEALTH_GATE_VALID=0x002`,
`NAVCOM_VALID=0x004`, `IS_MOVING_VALID=0x008`, `ARRIVAL_VALID=0x010`,
`UNIT_BYTES_VALID=0x020`, `PROCESS_GATES_VALID=0x040`,
`DISPATCH_SCRATCH_VALID=0x080`, and `TUBE_PIGGYBACK_VALID=0x100`, with exact known
mask `0x1FF`. The membership word at offset 268 is
`IN_LOGIC=0x1`, `PENDING_DELETE=0x2`, `CELL_LIST_MEMBER=0x4`, and
`OCCUPANCY_PRESENT=0x8`, with exact known mask `0xF`.

`HostEventCode` is `u16` with these exact values:

| Value | Event | Value | Event |
|---:|---|---:|---|
| 1 | `ObjectPassEnter` | 22 | `NullLocomotorInvariant` |
| 2 | `TechnoPreThroughRocking` | 23 | `OnArrival` |
| 3 | `GuardB` | 24 | `MoveRateLookup` |
| 4 | `TechnoRemainingPre` | 25 | `ScenarioRandomRanged` |
| 5 | `MissionDispatchEnter` | 26 | `DispatchWriteStart` |
| 6 | `ObjectAiEnter` | 27 | `DispatchWriteScratch` |
| 7 | `ObjectAiReturn` | 28 | `DispatchWriteDelay` |
| 8 | `DispatchActiveGate` | 29 | `PassiveAcquire` |
| 9 | `DispatchTimerGate` | 30 | `Bomb` |
| 10 | `DispatchHealthGate` | 31 | `SlaveManager` |
| 11 | `UnitMoveRead6E0` | 32 | `CaptureManager` |
| 12 | `UnitMoveClear6D2` | 33 | `GuardE` |
| 13 | `UnitMoveCheckSaved6E0` | 34 | `TechnoLatePost` |
| 14 | `UnitMoveReadCheck6E1` | 35 | `FootPostTechnoGate` |
| 15 | `UnitMoveReadCheck6E2` | 36 | `FootPreProcess` |
| 16 | `QueueMission` | 37 | `FootProcessGate` |
| 17 | `UnitTrackerCheck` | 38 | `LocomotorProcessEnter` |
| 18 | `UnitTrackerRestart` | 39 | `LocomotorProcessReturn` |
| 19 | `FootMissionMoveEnter` | 40 | `FootPostProcessGate` |
| 20 | `NavComGate` | 41 | `FootLaterWork` |
| 21 | `IsMovingCall` | 42 | `FootReturn` |

Value `43` is `QueuedMissionGate`; it is listed separately so the already frozen
1-through-42 values do not move.

`HostGateCode` is `u16`: `None=0`, `GuardBActive=1`, `DispatchActive=2`,
`DispatchTimerDue=3`, `DispatchHealth=4`, `UnitSaved6E0=5`, `UnitByte6E1=6`,
`UnitByte6E2=7`, `NavComPresent=8`, `LocomotorIsMoving=9`, `GuardEActive=10`,
`FootPostTechnoActive=11`, `FootProcessGate0=12`, `FootProcessGate1=13`,
`FootProcessGate2=14`, `FootProcessGate3=15`, `FootProcessGate4=16`, and
`FootPostProcessActive=17`, `UnitTracker=18`, `LocomotorNonNull=19`, and
`QueuedMissionEmpty=20`. A non-gate host event must use gate `None` and tri-state
`NotApplicable`. A gate event must use its one named gate and `Failed` or `Passed`.
`FootProcessGate` additionally requires `member_index=0..4` in order,
`member_total=5`, and the corresponding gate code. Host flags are `u16`:
`HAS_BEFORE=0x01`, `HAS_AFTER=0x02`, `HAS_RESULT=0x04`, `HAS_MEMBER=0x08`,
`HAS_RELATED_POINTER=0x10`, and `HAS_AUX=0x20`, with known mask `0x3F`.
`result_width` is only `0`, `1`, `2`, `4`, or `8`; zero is legal only when
`HAS_RESULT` is clear.

The exact host-event flag matrix is:

| Events | Required flags | Additional constraint |
|---|---:|---|
| `ObjectAiEnter` | `HAS_BEFORE` (`0x01`) | width 0; field offset `0x90`; raw active byte immediately before `ObjectClass::AI` in `before` |
| `ObjectAiReturn` | `HAS_AFTER` (`0x02`) | width 0; field offset `0x90`; raw active byte immediately after `ObjectClass::AI` in `after` |
| `GuardB`, `DispatchActiveGate`, `GuardE`, `FootPostTechnoGate`, `FootPostProcessGate` | `0x05` (`HAS_BEFORE` plus `HAS_RESULT`) | width 1; raw active byte in `before`; named gate; `Failed`/`Passed` |
| `DispatchTimerGate` | `0x07` (`HAS_BEFORE`, `HAS_AFTER`, `HAS_RESULT`) | width 4; raw unsigned Start/Delay-or-remaining dwords zero-extended; gate `DispatchTimerDue` |
| `DispatchHealthGate` | `0x05` (`HAS_BEFORE` plus `HAS_RESULT`) | width 4; raw signed `i32` health sign-extended in `before`; gate `DispatchHealth` |
| `UnitMoveCheckSaved6E0`, `UnitMoveReadCheck6E1`, `UnitMoveReadCheck6E2` | `0x05` (`HAS_BEFORE` plus `HAS_RESULT`) | width 1; raw saved/read byte in `before`; named gate |
| `UnitTrackerCheck` | `0x05` (`HAS_BEFORE` plus `HAS_RESULT`) | width 4; raw unsigned 32-bit tracker return zero-extended in `before`; gate `UnitTracker` |
| `NavComGate` | `0x14` (`HAS_RESULT` plus `HAS_RELATED_POINTER`) | width 4; gate `NavComPresent`; exact raw NavCom pointer retained |
| `IsMovingCall` | `0x05` (`HAS_BEFORE` plus `HAS_RESULT`) | width 4; raw 32-bit COM Boolean in `before`; gate `LocomotorIsMoving`; tri-state is derived strictly from raw zero/nonzero |
| `QueuedMissionGate` | `0x05` (`HAS_BEFORE` plus `HAS_RESULT`) | width 4; raw signed `i32` queued mission sign-extended in `before`; gate `QueuedMissionEmpty` |
| `NullLocomotorInvariant` | `0x14` (`HAS_RESULT` plus `HAS_RELATED_POINTER`) | width 4; gate `LocomotorNonNull`; a failed result invalidates capture |
| `FootProcessGate` members 0, 1, 2, 4 | `0x0D` (`HAS_BEFORE`, `HAS_RESULT`, `HAS_MEMBER`) | widths `4,1,1,1`; member 0 dword and byte members zero-extend; total 5, matching gate code |
| `FootProcessGate` member 3 | `0x2D` (member flags plus `HAS_AUX`) | width 4; raw `+0x2A8` pointer zero-extends in `before`, Type `+0x692` byte zero-extends in `aux` |
| `UnitMoveRead6E0` | `HAS_BEFORE` | width 0; raw loaded byte in `before` |
| `UnitMoveClear6D2`, `DispatchWriteStart`, `DispatchWriteScratch`, `DispatchWriteDelay` | `0x03` (`HAS_BEFORE` plus `HAS_AFTER`) | width 0; exact native field offset plus raw old/new value; Unit byte and all three raw dispatch dwords zero-extend |
| `QueueMission`, `MoveRateLookup`, `ScenarioRandomRanged`, `LocomotorProcessReturn` | `HAS_AUX` | width 0; first three are signed `i32` sign-extended into `aux`; Process return is raw 32-bit COM Boolean zero-extended |
| all remaining host events | `0` | gate `None`, width 0, tri-state `NotApplicable`, all optional fields zero |

No event may set a strict superset of its required flags.

`DriveEventCode` is `u16`: `ProcessEnter=1`, `TrackSelect=2`,
`TrackInitialize=3`, `PathPoint=4`, `TrackAttempt=5`, `PointBudget=6`,
`CoordinateWrite=7`, `FacingWrite=8`, `CellCross=9`, `TrackComplete=10`,
`SameProcessRetry=11`, `Arrival=12`, `ProcessReturn=13`. Drive flags are `u16`:
`HAS_MEMBER=0x01`, `HAS_POINT_BUDGET=0x02`, `HAS_COORDINATE=0x04`,
`HAS_FACING=0x08`, `HAS_RESIDUAL=0x10`, `HAS_RETRY=0x20`,
`HAS_RESULTS=0x40`, and `HAS_PATH_DIRECTION=0x80`, with known mask `0xFF`. Exact
required sets are:

| Event | Required flags |
|---|---:|
| `ProcessEnter` | `0` |
| `TrackSelect`, `TrackInitialize` | `HAS_RESIDUAL` |
| `PathPoint` | `0x85` (`HAS_MEMBER`, `HAS_COORDINATE`, and `HAS_PATH_DIRECTION`) |
| `TrackAttempt` | `0x17` (`HAS_MEMBER`, `HAS_POINT_BUDGET`, `HAS_COORDINATE`, `HAS_RESIDUAL`) |
| `PointBudget` | `HAS_POINT_BUDGET` |
| `CoordinateWrite` | `HAS_COORDINATE` |
| `FacingWrite` | `HAS_FACING` |
| `CellCross` | `0x44` (`HAS_COORDINATE` plus `HAS_RESULTS`) |
| `TrackComplete` | `0x50` (`HAS_RESIDUAL` plus `HAS_RESULTS`) |
| `SameProcessRetry` | `0x60` (`HAS_RETRY` plus `HAS_RESULTS`) |
| `Arrival` | `0x44` (`HAS_COORDINATE` plus `HAS_RESULTS`) |
| `ProcessReturn` | `HAS_RESULTS` |

No event may set a strict superset. Unused fields and result bytes are zero, and
`WireBool` fields are only 0/1. `CellCross`, `TrackComplete`, `SameProcessRetry`, and
`Arrival` may set only their respectively named result byte; `ProcessReturn` records all
four final Boolean values.

`RngSubkind` is `u16`: `StateBoundary=1`, `ApiBegin=2`, `Candidate=3`,
`Accepted=4`, `ApiEnd=5`. `RngBoundary` is `u16`: `CaptureStart=1`,
`BeforeMissionMove=2`, `AfterMissionMove=3`, `BeforeDriveProcess=4`,
`AfterDriveProcess=5`, `CompletedTick=6`. `RngBranch` is `u16`:
`None=0`, `FootMoveJitter=1`. RNG flags are `u32`: `HAS_BOUNDS=0x01`,
`HAS_CANDIDATE=0x02`, `HAS_ACCEPTED=0x04`, `CANDIDATE_REJECTED=0x08`, and
`HAS_OBJECT=0x10`, with known mask `0x1F`. `StateBoundary` requires flags `0`, branch
`None`, and one named boundary. `ApiBegin` requires
`HAS_BOUNDS | HAS_OBJECT`; `Candidate` requires
`HAS_BOUNDS | HAS_CANDIDATE | HAS_OBJECT` and conditionally adds
`CANDIDATE_REJECTED` exactly when that raw draw was rejected; `Accepted` requires
`HAS_BOUNDS | HAS_CANDIDATE | HAS_ACCEPTED | HAS_OBJECT`; `ApiEnd` requires
`HAS_BOUNDS | HAS_ACCEPTED | HAS_OBJECT`. API records use branch `FootMoveJitter`,
boundary zero, and preserve call/draw indices without modulo inference.
Completed-tick flags are `u32`: `OBJECT_PASS_ENTERED=0x1`,
`OBJECT_PASS_RETURNED=0x2`, `MOVEMENT_COMPLETED=0x4`, known mask `0x7`; a successful
`CompletedTick` requires exactly `0x7`. A successful `CaptureEnd` requires terminal-byte
tuple `(completed=1, overflow=0, forced_teardown=0, producer_status=3)`; no other tuple
is successful. Its nine kind counts cover kinds 12 through 20 inclusive and include the
`CaptureEnd` record itself: kind 19 is one, kind 20 is zero, their sum equals
`total_record_count`, and `total_raw_bytes` equals the sum of every exact record length
including `CaptureEnd`, accumulated and encoded as `u64`. `terminal_sequence` equals both the `CaptureEnd` header sequence
and the transport's committed write sequence. Only a stream completed through the clean
transport transition may treat `CaptureEnd` as terminal authority; an invalid or
uncompleted end never yields success.

The 16-byte correlations have one reproducible definition: scenario and manifest
correlations are the first 16 raw bytes of their respective full SHA-256 values; run
correlation is the first 16 bytes of `SHA256(UTF8(run_id))`. Each must be nonzero.
Action/event correlation is the first eight raw bytes of
`SHA256(UTF8(scenario_id) || 0x00 || UTF8(action_id))`, interpreted as one little-endian
`u64`. A zero result invalidates materialization rather than being remapped. It is stable
for the command and all causally related records across all five runs.

The common movement header is exactly 120 bytes:

| Offset | Width | Field |
|---:|---:|---|
| 0 | 4 | `V20O` |
| 4 | 2 | version `3` |
| 6 | 2 | record kind |
| 8 | 4 | total record length |
| 12 | 4 | monotonic producer sequence |
| 16 | 4 | flags; only bit 0 `COMPLETE` is known |
| 20 | 4 | callback/native origin |
| 24 | 2 | nonzero manifest-scoped hook ID |
| 26 | 1 | status |
| 27 | 1 | reserved zero |
| 28 | 4 | static hook address |
| 32 | 4 | runtime hook address |
| 36 | 4 | producer thread ID |
| 40 | 4 | original global frame |
| 44 | 2 | reason |
| 46 | 2 | reserved zero |
| 48 | 8 | raw QPC ticks |
| 56 | 8 | action/event correlation |
| 64 | 16 | run correlation bytes |
| 80 | 16 | scenario correlation bytes |
| 96 | 16 | manifest correlation bytes |
| 112 | 4 | raw callback receiver pointer |
| 116 | 4 | raw complete-object pointer |

Full executable/map/rules/tool/producer/collector/scenario SHA-256 values live in
`CaptureStart`; the 16-byte header correlations are per-record routing keys, not a
replacement for full identities. Unsupported flags/status/reason combinations,
zero required correlations, invalid hook/address/receiver relationships, and nonzero
reserved bytes fail closed. A manifest-declared global/non-object hook uses receiver
relation `none` and requires both pointer fields to be zero. An object hook requires a
nonzero complete-object pointer plus either exact receiver equality or one declared
signed subobject adjustment; the collector, not the codec, validates that relationship.

`RngEvent` is a typed variant: `StateBoundary` carries the complete 1,016-byte logical
Scenario RNG block; `ApiBegin`, `Candidate`, `Accepted`, and `ApiEnd` carry call index,
raw draw index, rejection/acceptance value, bounds, and branch identity. This keeps each
record below 4,080 bytes while preserving every candidate and full-state boundary.
The Foot path is the native 24-dword direction queue at `Foot+0x5E0..+0x63C` with `-1`
as the terminator. `ObjectSnapshot.path_member_count` is the number of active entries
before the first `-1`, inclusive range `0..24`. Its path hash preimage is exactly the
concatenation, in queue order, of those active raw direction values encoded as signed
little-endian `i32`; the terminator and unused tail are excluded. Count zero therefore
uses `SHA256(empty)`. One `DriveEvent::PathPoint` is required for each active member and
carries matching `member_index`, `member_total`, signed `point_index == member_index`,
the exact signed raw direction, and the destination cell X/Y produced by replay; its Z
field is zero. The codec preserves the full signed raw direction domain; the bounded
ordinary-ground collection contract requires the pilot's active directions to be
`0..8` and rejects any other value as a failed fixture precondition, after raw
persistence. The collector reconstructs the same preimage from ordered PathPoint events
and requires its count/hash to equal the snapshot. Retry events likewise use explicit
member identity; neither family is an untyped byte fragment.

The version-3 record lengths are frozen by kind/subkind:

| Record | Payload bytes | Total bytes |
|---|---:|---:|
| `CaptureStart` | 336 | 456 |
| `CommandConsumed` | 32 | 152 |
| `ObjectSnapshot` | 320 | 440 |
| `HostEvent` | 64 | 184 |
| `DriveEvent` | 80 | 200 |
| `RngEvent::StateBoundary` | 1,032 | 1,152 |
| other `RngEvent` subkinds | 48 | 168 |
| `CompletedTick` | 32 | 152 |
| `CaptureEnd` | 64 | 184 |
| `CaptureError` | 32 | 152 |

The hook ID is deliberately opaque at the codec layer. This plan assigns no native
hook names or numeric values; it only requires a nonzero `u16`. The future reviewed hook
manifest and scenario will bind each numeric ID to an exact static/runtime address and
role, and the collector will enforce that binding.

Both fixed ASCII fields use an exact `u8` length prefix: byte 0 is the count, bytes
`1..=count` are printable 7-bit ASCII excluding NUL, and every remaining byte is zero.
The 16-byte type field therefore permits lengths `1..=15`; the 32-byte owner field
permits `1..=31`. Empty, non-ASCII, embedded-NUL, overlong, or nonzero-tail strings fail.

The fixed payloads use these exact offsets. Any unlisted tail is reserved zero.

`CaptureStart` (336 bytes):

| Offset | Width | Field |
|---:|---:|---|
| 0 | 32 | executable SHA-256 |
| 32 | 32 | map SHA-256 |
| 64 | 32 | YR `rulesmd.ini` SHA-256 |
| 96 | 32 | base `rules.ini` SHA-256 |
| 128 | 32 | tool-spec SHA-256 |
| 160 | 32 | hook-manifest SHA-256 |
| 192 | 32 | producer SHA-256 |
| 224 | 32 | collector SHA-256 |
| 256 | 32 | comparison-policy SHA-256 |
| 288 | 32 | scenario SHA-256 |
| 320 | 1 | `MovementLane::InstrumentedState` (`1`) |
| 321 | 1 | observed live GameSpeed byte, `0..6` |
| 322 | 2 | reserved zero |
| 324 | 4 | producer process ID |
| 328 | 8 | QPC frequency |

Every hash, process ID, and QPC frequency is nonzero. The collector compares all ten
hashes and the observed GameSpeed byte to the sealed scenario/lane identity before it
accepts record 2.

`CommandConsumed` (32 bytes):

| Offset | Width | Field |
|---:|---:|---|
| 0 | 8 | capture-local object ID |
| 8 | 4 | raw native object pointer |
| 12 | 4 | LogicVector occurrence |
| 16 | 4 | destination cell X, signed |
| 20 | 4 | destination cell Y, signed |
| 24 | 2 | command kind |
| 26 | 2 | consumption result |
| 28 | 4 | reserved zero |

`ObjectSnapshot` (320 bytes):

| Offset | Width | Field |
|---:|---:|---|
| 0 | 2 | named snapshot boundary |
| 2 | 1 | locomotor kind |
| 3 | 1 | known snapshot-presence flags |
| 4 | 8 | capture-local object ID |
| 12 | 4 | raw native object pointer |
| 16 | 16 | length-prefixed fixed ASCII type ID with zero tail |
| 32 | 32 | length-prefixed fixed ASCII owner ID with zero tail |
| 64 | 16 | LogicVector index/count before and after, four `u32` values |
| 80 | 4 | active, InLimbo, actually-ran, commenced raw bytes |
| 84 | 20 | signed health, current/suspended/queued mission, substate |
| 104 | 16 | MissionTimer and dispatch timer start/duration, four `u32` values |
| 120 | 4 | known host-outcome presence flags |
| 124 | 12 | signed world coordinate X/Y/Z |
| 136 | 8 | raw/target body and turret facing, four `u16` values |
| 144 | 12 | raw NavCom, auxiliary, and suspended-target pointers |
| 156 | 2 | path member count |
| 158 | 2 | reserved zero |
| 160 | 32 | SHA-256 of the ordered active signed-i32 path directions defined above |
| 192 | 12 | Drive destination X/Y/Z, signed |
| 204 | 12 | Drive head-to X/Y/Z, signed |
| 216 | 4 | Drive residual, signed |
| 220 | 8 | target-speed fraction raw IEEE-754 binary64 `u64` bits |
| 228 | 4 | RawTrack selector, signed |
| 232 | 4 | RawTrack cursor, signed |
| 236 | 1 | RawTrack short byte |
| 237 | 3 | reserved zero |
| 240 | 8 | owner current-speed fraction raw IEEE-754 binary64 `u64` bits |
| 248 | 4 | fresh integer speed, signed |
| 252 | 1 | arrival-guard byte |
| 253 | 1 | Tube direction byte |
| 254 | 1 | Tube index byte |
| 255 | 1 | piggyback state byte |
| 256 | 8 | signed occupancy cell X/Y |
| 264 | 4 | raw CellClass list-head pointer |
| 268 | 4 | live-membership/pending-delete presence flags |
| 272 | 4 | dispatch rate in frame-counter counts |
| 276 | 4 | raw dispatch scratch dword, valid only when its presence bit is set |
| 280 | 1 | observed live GameSpeed byte, `0..6` |
| 281 | 3 | reserved zero |
| 284 | 36 | reserved zero |

`HostEvent` (64 bytes) stores event code and gate code at 0/2, object ID at 4,
native field offset at 12, raw before/after `u64` bit containers at 16/24,
result/width/flags at 32..35, member index/total at 36/40, related raw pointer at 44,
raw auxiliary `u64` bits at 48, and eight reserved-zero bytes at 56. Source-width rules
are event-matrix-specific and identical in Rust/Python: unsigned bytes/words/dwords and
pointers are zero-extended; signed `i32` values are sign-extended to 64 bits; native
`i64` values retain all bits. Unused containers are zero. The tri-state result never
replaces the raw tested operand.

`DriveEvent` (80 bytes) stores event/flags at 0/2, object ID at 4, member index/total
at 12/16, selector/cursor/point index at 20/24/28, point budget before/spend/after at
32/36/40, signed coordinate X/Y/Z at 44/48/52, facing at 56, two reserved bytes,
residual before/after at 60/64, retry index at 68, four Boolean result bytes
(`track_complete`, `same_process_retry`, `cell_cross`, `arrival`) at 72, and signed raw
path direction at 76. Selector, cursor, point index, every point-budget value, residual,
coordinate, and path direction are raw two's-complement `i32`; member indices/totals and
retry index are `u32`, and facing is `u16`. The path-direction field is present only for
`PathPoint`, preserves any signed `i32` bit pattern at the codec layer, and is zero for
every other event; Task 4 enforces the normal pilot's `0..8` domain.
For `PathPoint`, coordinates mean signed cell X/Y with Z zero; for Drive track/coordinate/
cell/arrival events they mean signed native world leptons in the source frame named by
the event.

`RngEvent::StateBoundary` stores subkind/boundary at 0/2, API call index at 4, raw draw
count at 8, API call count at 12, then the exact 1,016-byte logical RNG state at 16.
Every other 48-byte RNG payload stores subkind/branch, API call index, raw draw index,
signed lower/upper bounds, raw candidate, signed accepted value, known flags, object ID,
state-boundary sequence, and a final reserved-zero dword.

`CompletedTick` uses eight `u32` words for counter-before, counter-after,
completed-counter, LogicVector count before/after, records-in-tick, known flags, and
reserved zero. `CaptureEnd` stores nine `u32` kind counts, dropped count, terminal
sequence, four terminal Boolean/status bytes, final frame, total record count, total raw
bytes as a little-endian `u64`, with no tail padding. `CaptureError` stores attempted
kind, reserved zero, required length as a little-endian `u64`, slot capacity, write/read
sequences, dropped count, and offending sequence. The payload has no tail padding.
For avoidance of cross-language ambiguity, `CaptureEnd` offsets are counts `0..35`,
dropped `36`, terminal sequence `40`, terminal bytes `44..47`, final frame `48`, total
record count `52`, and total raw bytes `56..63`. `CaptureError` offsets are attempted kind
`0`, reserved `2`, required length `4..11`, slot capacity `12`, write/read `16/20`,
dropped `24`, and offending sequence `28`.

## Task 1: Add Strict Scenario, Lane-Identity, and Checkpoint-Proof Contracts

**Files:**

- Create `tools/oracle_harness/schemas/ground-movement-scenario.v1.schema.json`.
- Create `tools/oracle_harness/schemas/ground-movement-lane-identity.v1.schema.json`.
- Create `tools/oracle_harness/oracle_harness/ground_movement_scenario.py`.
- Modify `tools/oracle_harness/oracle_harness/schema.py`.
- Modify `tools/oracle_harness/tests/test_schema.py`.
- Create `tools/oracle_harness/tests/test_ground_movement_scenario.py`.

### Implementation

1. Add the scenario and lane-identity contracts to `SCHEMA_CONTRACTS`; update the
   expected contract count from 12 to 14. Keep the generic top-level
   exact-key check in `schema.py`, and delegate nested validation to
   `ground_movement_scenario.py`. Imports of those validators inside `schema.py` must be
   function-local. The delegated leaf validators in `ground_movement_scenario.py` never
   call back into `schema.validate_document`; only materialization/composition entry
   points may import that generic validator function-locally. This preserves direct
   module import and prevents the cycle `schema -> ground_movement_scenario -> schema`;
   `status` and `capabilities` must not import any offline movement module. Task 2 adds
   only the normalized-event contract when `movement_wire.py` exists; Task 6 adds the
   field-role contract after both finite policy surfaces exist.
2. The scenario validator must require:
   - `scenario_id` and `action.action_id` in the existing lowercase stable-slug grammar,
     so the NUL-delimited correlation preimage is unambiguous;
   - exact executable, map, `rulesmd.ini`, base `rules.ini`, recipe, checkpoint,
     tool-spec, producer, collector, manifest, and policy hashes;
   - a stock `MTNK`, owner/type/Drive locomotor identity, exact start cell/coordinate,
     facing, rank/crate/health/mission/target/path/RNG preconditions, exact destination,
     and a proved-empty destination;
   - contiguous explicit attempts with exactly one accepted final candidate;
   - one-or-more strict negative-control declarations with unique control/companion IDs,
     a full companion-scenario hash, literal clean lane and branch-not-taken outcome,
     and sorted unique required artifact classes;
   - exactly two instrumented and three clean repeats with distinct lane names and
     lane-specific required artifact classes;
   - strict clean comparison requirements whose prerequisite and sequence names are
     unique, whose evidence kinds use the existing enum vocabulary, and whose per-
     sequence `comparable_fields` are sorted, unique, nonempty, exact keys;
   - required kind/count bounds and pointer-normalization policy;
   - `capture_contract.expected_origin == 1`, exact allowed nonzero hook-ID/address
     bindings with one strict `none`/`complete_object`/signed-adjustment receiver
     relation, a positive `maximum_frame_delta <= 0x7FFF_FFFF`, and a positive maximum-
     record budget no larger than `0xFFFF_FFFE`;
   - per-kind min/max bounds for every kind 12..20, with successful-error count exactly
     zero, CaptureStart/CommandConsumed/CaptureEnd exactly one, each min no greater than
     max, and the sum of all successful maxima no greater than the ordinary-record budget;
   - exact positive workflow timeouts and artifact classes.
3. Reject zero/multiple accepted objects, wrong type/owner/locomotor, a nonempty
   destination, missing full RNG-state hash, missing/duplicate/noncontiguous attempts,
   and any attempt count not equal to the declared policy.
4. Do not create a production scenario JSON. The repository currently lacks a proved
   finalized checkpoint; tests build complete synthetic documents with conspicuous
   fixture hashes. A real file belongs to later approved checkpoint creation.
5. Materialize through one pure result type:

   ```python
   @dataclass(frozen=True)
   class MaterializedGroundMovementScenario:
       canonical_bytes: bytes
       sha256: str

       def decode(self) -> Mapping[str, object]:
           value = load_json_strict(self.canonical_bytes)
           return validate_document("ground-movement-scenario.v1", value)

   def materialize_ground_movement_scenario(
       *,
       recipe: Mapping[str, object],
       checkpoint_proofs: Sequence[Mapping[str, object]],
       authority: Mapping[str, object],
   ) -> MaterializedGroundMovementScenario:
       document = build_ground_movement_document(
           recipe=recipe,
           checkpoint_proofs=checkpoint_proofs,
           authority=authority,
       )
       validated = validate_document("ground-movement-scenario.v1", document)
       encoded = canonical_json_bytes(validated)
       return MaterializedGroundMovementScenario(
           canonical_bytes=encoded,
           sha256=sha256_bytes(encoded),
       )
   ```

   Canonical `bytes` are the immutable authority. `decode()` returns a newly validated
   value, so mutating a caller-owned source mapping or one decoded copy cannot alter the
   materialized scenario or its hash.

6. Add `CheckpointProofLedger` and `CheckpointProofValidatingBackend`. The wrapper owns
   the exact next operation from `CHECKPOINT_EDGES`, delegates one `execute` call to the
   injected backend, validates the returned `TransitionProof` immediately, and only then
   returns it to `OriginalYrWorkflow`. A malformed proof therefore raises inside that
   transition: the controller never emits `transition_completed` for it and never invokes
   a later edge. On the first malformed proof, the ledger permanently latches an
   `invalid_reason`; every subsequent `execute` fails before calling the inner backend,
   even when the controller gives retry-permitted `preflight`, `identify_window`,
   `shell_ready`, or `prove_mtnk` a positive retry allowance. The wrapper's
   `diagnostics()` delegates to the inner backend so existing failure capture remains
   intact. The ledger accepts exactly these detail-key contracts
   (no missing or additional key):

   | Operation | Exact `details` keys and constraints |
   |---|---|
   | `preflight` | `executable_sha256`, `tool_spec_sha256`, `recipe_sha256`; three nonzero lowercase SHA-256 strings |
   | `clone` | `sandbox_path`, `source_manifest_sha256`, `clone_manifest_sha256`; safe path plus two hashes |
   | `launch` | `process_id`, `executable_sha256`, `launch_args_sha256`; positive PID and two hashes |
   | `identify_window` | `process_id`, `window_handle`, `client_geometry_sha256`; positive IDs and one hash |
   | `shell_ready` | `guard_sha256`; one nonzero hash |
   | `configure_battle_america` | `mode`, `house`, `game_speed`, `map_candidates_sha256`, `attempt_index`; exact standard/America values, GameSpeed `0..6`, nonnegative attempt |
   | `start_standard_match` | `map_filename`, `map_sha256`, `rulesmd_sha256`, `rules_sha256`, `attempt_index`, `rng_precondition_sha256`; safe filename, four hashes, matching attempt |
   | `prove_mtnk` | `candidate_ledger_sha256`, `candidate_count`, `accepted_count`, `capture_local_object_id`, `type_id`, `owner_id`, `locomotor`, `logic_occurrence`, `rank`, `crate_status`, `health`, `mission`; complete candidate ledger, `accepted_count=1`, local ID `1`, stock MTNK/owner/Drive identity |
   | `position_camera_and_unit` | `start_coordinate_sha256`, `start_facing`, `destination_cell_sha256`, `destination_empty`, `terrain_sha256`, `path_sha256`, `client_selection_sha256`, `client_destination_sha256`; destination empty must be literal true |
   | `save_checkpoint` | `checkpoint_files_sha256`, `checkpoint_manifest_sha256`; two nonzero hashes |
   | `exit` | `exit_receipt_sha256`; one nonzero hash |
   | `discover_save` | `discovered_files_sha256`, `discovered_file_count`; nonzero hash and positive count |
   | `clean_reload_verify` | `reloaded_manifest_sha256`, `capture_local_object_id`, `preconditions_sha256`, `rng_state_sha256`; local ID `1` and three hashes |
   | `seal` | `final_manifest_sha256`, `evidence_artifacts_sha256`, `evidence_artifact_count`; two hashes and positive count |

   Every proof also requires `precondition == source.value` and
   `postcondition == destination.value` from that exact `CHECKPOINT_EDGES` row, plus at
   least one safe relative evidence path. The ledger cross-reconciles repeated executable,
   recipe, attempt, map/rules, MTNK, destination, checkpoint, reload and final-manifest
   identities instead of merely validating each row in isolation.
7. Materialization still reconciles only completed controller events. Their operation
   order must equal every name in `CHECKPOINT_EDGES`, and their serialized proof documents
   must byte-canonically equal the already accepted ledger entries. A missing, duplicate,
   failed, reordered, or substituted event prevents materialization; this second check is
   defense in depth, not the first proof gate.
8. Canonicalize unordered input mappings but preserve all semantically ordered arrays:
   checkpoint attempts, LogicVector occurrence, path points, proof order, record-kind
   requirements, and artifact-class requirements.

### Focused tests

Add tests named exactly:

- `test_materialization_is_canonical_across_mapping_insertion_order`
- `test_one_byte_identity_change_changes_scenario_hash`
- `test_hidden_or_noncontiguous_attempt_is_rejected`
- `test_negative_controls_are_required_strict_and_canonical`
- `test_zero_or_multiple_mtnk_candidates_are_rejected`
- `test_wrong_owner_type_locomotor_or_destination_is_rejected`
- `test_checkpoint_proof_order_and_completeness_are_required`
- `test_every_checkpoint_edge_rejects_malformed_proof_before_later_backend_call`
- `test_malformed_retryable_proof_latches_before_inner_retry`
- `test_checkpoint_proof_repeated_identities_must_reconcile`
- `test_lane_counts_are_exactly_two_and_three`
- `test_pointer_is_run_local_not_cross_run_fixture_identity`
- `test_lane_identity_requires_complete_tool_environment_and_gamespeed_identity`
- `test_schema_registration_is_lazy_and_direct_import_is_cycle_free`
- `test_portable_schema_documents_match_runtime_contracts`

The checkpoint tests construct `OriginalYrWorkflow.checkpoint_creation` around
`CheckpointProofValidatingBackend`, run it through `WorkflowController` and a fake inner
backend, then feed the memory sink's completed events into the pure materializer. For
each edge, mutate one required detail while leaving the outer proof typed and assert the
controller result fails, that edge's inner call count is one, every later inner call count
is zero, and no later proof/event is accepted as a substitute. The latch test supplies a
positive retry allowance for each of the four retry-permitted checkpoint operations in
separate cases, proves the inner backend was called once, and proves the retry fails from
the latched ledger without an inner call or later edge.

### Task validation

```powershell
python -B -m unittest tools/oracle_harness/tests/test_schema.py tools/oracle_harness/tests/test_checkpoint_recipe.py tools/oracle_harness/tests/test_ground_movement_scenario.py
if ($LASTEXITCODE -ne 0) { throw 'ground movement contract tests failed' }
```

Expected result: all tests pass; no `scenarios/*.json` production artifact is created.

## Task 2: Add Movement Protocol Version 3, the Event Contract, and a Golden Corpus

**Files:**

- Create `tools/oracle_protocol/src/movement_v3.rs`.
- Create `tools/oracle_protocol/ground_movement_v3_golden.hex`.
- Create `tools/oracle_protocol/verify_movement_vector.py`.
- Modify `tools/oracle_protocol/src/lib.rs`.
- Modify `tools/oracle_protocol/README.md`.
- Create `tools/oracle_harness/schemas/ground-movement-event.v1.schema.json`.
- Create `tools/oracle_harness/oracle_harness/movement_wire.py`.
- Modify `tools/oracle_harness/oracle_harness/ground_movement_scenario.py`.
- Modify `tools/oracle_harness/oracle_harness/schema.py`.
- Modify `tools/oracle_harness/tests/test_ground_movement_scenario.py`.
- Modify `tools/oracle_harness/tests/test_schema.py`.
- Create `tools/oracle_harness/tests/test_movement_wire.py`.

### Implementation

1. Add `pub mod movement_v3;` without changing `MAGIC`, `VERSION`, old record enums,
   old lengths, or old encoders/decoders.
2. Implement the header table, all frozen discriminant/flag matrices, and kinds 12-20
   exactly as specified above. Use explicit
   `put_u16/u32/u64/i32/i64` helpers and slices. Encoding begins by zeroing the selected
   record region; decoding checks every reserved byte.
3. Use typed enums with `TryFrom` for record kind, origin, status, reason, lane,
   locomotor, command/result, snapshot boundary, host event/gate, Drive event, RNG
   subkind/boundary/branch, and Boolean/tri-state fields. Use newtypes
   that reject unknown bits for every flag word. Unknown discriminants fail; do not
   preserve them as an `Other` variant.
   Keep `hook_id` as an opaque nonzero `u16`, because selecting the manifest's exact
   hook vocabulary belongs to blocked O-07.
4. Use this public shape:

   ```rust
   pub const VERSION: u16 = 3;
   pub const HEADER_LEN: usize = 120;

   #[derive(Clone, Copy, Debug, Eq, PartialEq)]
   #[repr(u16)]
   pub enum MovementRecordKind {
       CaptureStart = 12,
       CommandConsumed = 13,
       ObjectSnapshot = 14,
       HostEvent = 15,
       DriveEvent = 16,
       RngEvent = 17,
       CompletedTick = 18,
       CaptureEnd = 19,
       CaptureError = 20,
   }

   #[derive(Clone, Copy, Debug, Eq, PartialEq)]
   pub struct MovementRecordContext {
       pub sequence: u32,
       pub origin: MovementOrigin,
       pub hook_id: u16,
       pub static_address: u32,
       pub runtime_address: u32,
       pub thread_id: u32,
       pub original_frame: u32,
       pub qpc_ticks: u64,
       pub correlation: u64,
       pub run_correlation: [u8; 16],
       pub scenario_correlation: [u8; 16],
       pub manifest_correlation: [u8; 16],
       pub raw_receiver: u32,
       pub raw_complete_object_pointer: u32,
       pub status: MovementStatus,
       pub reason: MovementReason,
   }

   pub enum MovementRecord {
       CaptureStart(MovementHeader, CaptureStartPayload),
       CommandConsumed(MovementHeader, CommandConsumedPayload),
       ObjectSnapshot(MovementHeader, ObjectSnapshotPayload),
       HostEvent(MovementHeader, HostEventPayload),
       DriveEvent(MovementHeader, DriveEventPayload),
       RngEvent(MovementHeader, RngEventPayload),
       CompletedTick(MovementHeader, CompletedTickPayload),
       CaptureEnd(MovementHeader, CaptureEndPayload),
       CaptureError(MovementHeader, CaptureErrorPayload),
   }
   ```

5. `CaptureStart` carries the ten full 32-byte hashes in the frozen order for executable,
   map, YR rules, base rules, tool spec, hook manifest, producer, collector, comparison
   policy, and scenario; it also carries lane `1`, observed live GameSpeed `0..6`, two
   reserved-zero bytes, process ID, and QPC frequency. Its payload is exactly 336 bytes.
   All full hashes, process ID, and QPC frequency must be nonzero.
6. `ObjectSnapshot` records the bounded host/Drive state named in Checkpoint E
   sections 7.2-7.3, with exact signed widths, raw bytes, and flags. It records a path
   count/hash but not an inline variable path; `DriveEvent::PathPoint` carries each
   ordered point.
7. `HostEvent` and `DriveEvent` are fixed-width semantic events using the exact required
   flag/event/gate matrices above. Stateless validators reject impossible per-record
   combinations, including a gate result on a non-gate event, a member index outside
   `0..member_total` or a point spend that contradicts before/after budget. Receiver
   scope/relation, cross-record object/correlation identity, pilot direction-domain
   enforcement, and complete path/retry/RNG ordering
   belong only to Task 4's stateful validator.
8. `RngEvent::StateBoundary` includes the existing full 1,016-byte logical-state shape.
   Candidate/accepted events include API call index and raw draw index so the collector
   never infers advances from ring-index subtraction.
9. `CaptureEnd` includes the exact inclusive kind counts/bytes, dropped count, overflow
   flag, terminal sequence, and completion state defined above. `CaptureError` is the only
   typed terminal-error record. A successful end cannot carry a reason; an error must
   carry one.
10. All encoded records must be `1..=4_080` bytes. The codec itself rejects an output
    shape outside that bound even if the caller supplies a larger buffer.
11. Freeze one concatenated golden corpus containing at least one valid instance of
     every record kind and every variable-length RNG subvariant. Rust parses successive
     record lengths and asserts its independently constructed records equal the frozen
     corpus exactly. Separate table-driven tests iterate every frozen origin/status/
     reason/lane/locomotor/command/result/boundary/Host/Drive/RNG discriminant and every
     flag matrix row, so representative golden events cannot leave unused values
     under-specified.
12. `verify_movement_vector.py` independently constructs and decodes the same corpus
    using only `struct`, `pathlib`, and `unittest`. It must not invoke the Rust codec or
    derive expected bytes from Cargo output.
13. Update the README table and current-producer section to state: movement v3 exists,
    no live movement callback/producer calls it, and offline vectors are not retail
    evidence.
14. Independently implement the stateless Python decoder in `movement_wire.py` with
    locally declared version-3 constants and `struct.unpack_from`; it must not load Rust
    code or copy a Rust-generated JSON interpretation. It returns frozen dataclasses and
    checks the same reserved bytes, enum domains, exact lengths, correlations, signed
    widths, path/retry constraints, point-budget relations, and RNG invariants as the
    Rust codec. Its core public shape is:

    ```python
    @dataclass(frozen=True)
    class MovementHeader:
        magic: bytes
        version: int
        kind: int
        record_len: int
        sequence: int
        flags: int
        origin: int
        hook_id: int
        status: int
        static_address: int
        runtime_address: int
        thread_id: int
        original_frame: int
        reason: int
        qpc_ticks: int
        correlation: int
        run_correlation: bytes
        scenario_correlation: bytes
        manifest_correlation: bytes
        raw_receiver: int
        raw_complete_object_pointer: int

    @dataclass(frozen=True)
    class DecodedMovementRecord:
        header: MovementHeader
        payload: Mapping[str, object]

    def decode_movement_record(record: bytes) -> DecodedMovementRecord:
        header = decode_movement_header(record)
        decoder = MOVEMENT_PAYLOAD_DECODERS.get(header.kind)
        if decoder is None:
            raise MovementWireError(f"unsupported movement kind {header.kind}")
        payload = decoder(record, header)
        return DecodedMovementRecord(header=header, payload=payload)
    ```

    Cross-process shared-memory concerns remain Task 4: this task accepts immutable
    `bytes` only and proves byte-level codec agreement against the frozen corpus.
15. Add only `ground-movement-event.v1` to `SCHEMA_CONTRACTS` and update the expected
    contract count from 14 to 15. Keep the generic top-level exact-key check in
    `schema.py`, and delegate exact kind-selected event validation to `movement_wire.py`
    through a function-local import. The delegated leaf validator never calls back into
    `schema.validate_document`; document composition may import the generic validator
    function-locally. Codec import and direct schema import remain cycle-free. The event
    schema requires the exact normalized top-level keys and exact payload variant for
    each kind described in the frozen wire contract. Task 6 owns the field-role schema,
    policy, validator, and final 15-to-16 registry transition.

### Required rejection tests

Rust and the independent Python checker together cover:

- min/max signed and unsigned scalar values;
- exact ASCII length-prefix maxima and malformed/nonzero tails;
- wrong magic/version/kind/length/flags/status/reason;
- every nonzero reserved region;
- all-zero required correlations or full hashes;
- zero hook ID and unknown event/boundary/locomotor/RNG discriminants; an opaque nonzero
  hook ID is codec-valid and only the collector rejects IDs absent from its bound manifest;
- per-record inconsistent path/retry member index and total;
- inconsistent point budget/residual/track state;
- malformed single RNG events and locally impossible raw/API indices; missing,
  duplicated, or reordered candidates and aggregate count mismatches are Task 4 tests;
- trailing bytes and records over 4,080 bytes;
- exact preservation of kinds 1-11 and the 3,416-byte snapshot golden.

Add Python tests named exactly:

- `test_python_decoder_matches_frozen_movement_golden`
- `test_unknown_kind_version_length_flags_and_reserved_bytes_fail`
- `test_normalized_event_schema_is_strict_and_kind_selected`
- `test_movement_event_schema_registration_is_lazy_and_cycle_free`

### Task validation

```powershell
python -B tools/oracle_protocol/verify_vector.py
if ($LASTEXITCODE -ne 0) { throw 'legacy protocol vector failed' }
python -B tools/oracle_protocol/verify_movement_vector.py
if ($LASTEXITCODE -ne 0) { throw 'movement protocol vector failed' }
python -B -m unittest tools/oracle_harness/tests/test_schema.py tools/oracle_harness/tests/test_ground_movement_scenario.py tools/oracle_harness/tests/test_movement_wire.py
if ($LASTEXITCODE -ne 0) { throw 'movement Python codec/schema tests failed' }
rustfmt --edition 2024 --check tools/oracle_protocol/src/movement_v3.rs tools/oracle_protocol/src/lib.rs
if ($LASTEXITCODE -ne 0) { throw 'protocol rustfmt check failed' }
```

Cargo is deferred to the final serial validation task so it is not run concurrently
with instrument tests.

## Task 3: Add an Inert Dedicated Movement Transport

**Files:**

- Create `tools/oracle_instrument/src/movement_transport.rs`.
- Modify `tools/oracle_instrument/src/lib.rs` only to add
  `mod movement_transport;`.

### Implementation

1. Copy no startup semantic fields and refactor neither existing transport. Implement a
   dedicated single-producer/single-consumer ring with these identities:

   ```rust
   pub(crate) const ENVIRONMENT_KEY: &str = "VERA20K_ORACLE_MOVEMENT_MAPPING";
   pub(crate) const MAPPING_PREFIX: &[u8] = b"Local\\VERA20kOracleMovement-";
   pub(crate) const CONTROL_MAGIC: [u8; 4] = *b"V20M";
   pub(crate) const TRANSPORT_VERSION: u16 = 1;
   pub(crate) const CONTROL_HEADER_LEN: usize = 192;
   pub(crate) const SLOT_SIZE: usize = 4_096;
   pub(crate) const SLOT_PREFIX_LEN: usize = 16;
   pub(crate) const SLOT_RECORD_CAPACITY: usize = 4_080;
   pub(crate) const MIN_SLOT_COUNT: u32 = 2;
   pub(crate) const MAX_SLOT_COUNT: u32 = 256;
   ```

2. Use exactly one extra 4,096-byte emergency terminal slot after the configured ring,
   matching the proven startup safety pattern without sharing its mapping or state.
   `total_len = 192 + (slot_count + 1) * 4096`; validate arithmetic before opening a
   view.
3. The 192-byte control header is explicit and zero-filled before publication:

   | Offset | Width | Field |
   |---:|---:|---|
   | 0 | 4 | `V20M` |
   | 4 | 2 | transport version 1 |
   | 6 | 2 | header length 192 |
   | 8 | 4 | total mapping length |
   | 12 | 4 | slot size 4,096 |
   | 16 | 4 | configured ring slot count |
   | 20 | 4 | protocol magic `V20O` |
   | 24 | 2 | protocol version 3 |
   | 26 | 2 | reserved zero |
   | 28 | 4 | producer status |
   | 32 | 4 | producer terminal reason |
   | 36 | 4 | producer process ID |
   | 40 | 4 | producer thread ID |
   | 44 | 4 | reserved zero in transport v1 |
   | 48 | 8 | QPC frequency |
   | 56 | 4 | committed write sequence |
   | 60 | 4 | acknowledged read sequence |
   | 64 | 4 | dropped count |
   | 68 | 4 | terminal count |
   | 72 | 16 | mapping nonce |
   | 88 | 32 | hook-manifest SHA-256 |
   | 120 | 32 | scenario SHA-256 |
   | 152 | 16 | run correlation |
   | 168 | 4 | emergency committed sequence |
   | 172 | 4 | emergency acknowledgement |
   | 176 | 4 | maximum record budget |
   | 180 | 12 | reserved zero |

4. Producer status is a frozen `u32`: `HostInitialized=0`, `Initializing=1`,
   `Ready=2`, `Terminal=3`. Producer terminal reason is the exact `MovementReason`
   integer widened to `u32`. `HostInitialized`, `Initializing`, and `Ready` require reason
   zero. `Terminal/None` is the one clean-completion tuple after a successful
   `CaptureEnd`; `Terminal/<nonzero>` is the failure tuple and requires exactly one
   emergency `CaptureError`. `maximum record budget` comes only from the validated
   scenario, is `1..=0xFFFF_FFFE`, and is immutable after bootstrap. It is the ceiling for
   ordinary records only: their sequences are contiguous `1..=maximum_record_budget`.
   Zero means unpublished. `u32::MAX` can appear only as the sole emergency terminal
   error's successor sequence when the ordinary budget is `0xFFFF_FFFE`; it is never an
   ordinary record or clean `CaptureEnd`. Dropped and terminal counts are saturating
   `u32`; any nonzero dropped count invalidates capture, and a terminal stream requires
   terminal count exactly one.
5. Each slot prefix remains exactly:

   | Offset | Width | Field |
   |---:|---:|---|
   | 0 | 4 | committed sequence, written last |
   | 4 | 4 | record length |
   | 8 | 4 | IEEE CRC-32 of record bytes |
   | 12 | 4 | reserved zero |
   | 16 | up to 4,080 | one complete movement record |

6. Keep transport APIs private and uncalled from `lib.rs`. Freeze enough immutable
   context for the transport itself to encode a lossless `CaptureError`; callers do not
   synthesize terminal records:

   ```rust
   pub(crate) struct MovementTransportConfig {
       pub mapping_nonce: [u8; 16],
       pub executable_sha256: [u8; 32],
       pub map_sha256: [u8; 32],
       pub rulesmd_sha256: [u8; 32],
       pub rules_sha256: [u8; 32],
       pub tool_spec_sha256: [u8; 32],
       pub hook_manifest_sha256: [u8; 32],
       pub producer_sha256: [u8; 32],
       pub collector_sha256: [u8; 32],
       pub comparison_policy_sha256: [u8; 32],
       pub scenario_sha256: [u8; 32],
       pub run_correlation: [u8; 16],
       pub observed_game_speed: u8,
       pub maximum_record_budget: u32,
   }

   pub(crate) struct MovementEventDescriptor {
       pub kind: MovementRecordKind,
       pub origin: MovementOrigin,
       pub hook_id: u16,
       pub static_address: u32,
       pub runtime_address: u32,
       pub original_frame: u32,
       pub correlation: u64,
       pub raw_receiver: u32,
       pub raw_complete_object_pointer: u32,
   }

   pub(crate) struct MovementFailureDescriptor {
       pub event: MovementEventDescriptor,
       pub reason: MovementReason,
       pub required_len: u64,
   }

   pub(crate) unsafe fn bootstrap(
       config: MovementTransportConfig,
       capture_start: MovementEventDescriptor,
   ) -> bool;
   pub(crate) unsafe fn reserve(
       descriptor: MovementEventDescriptor,
       record_len: usize,
   ) -> Result<Reservation, MovementTransportError>;
   pub(crate) unsafe fn prepare(
       reservation: &Reservation,
       record: &[u8],
   ) -> Result<(), MovementTransportError>;
   pub(crate) unsafe fn commit(
       reservation: Reservation,
   ) -> Result<(), MovementTransportError>;
   pub(crate) unsafe fn fail(
       failure: MovementFailureDescriptor,
   ) -> Result<(), MovementTransportError>;
   pub(crate) unsafe fn complete() -> Result<(), MovementTransportError>;
   ```

   The config supplies every CaptureStart full hash, GameSpeed, manifest/scenario
   identity, and routing correlations; each
   event descriptor—including bootstrap's CaptureStart descriptor—supplies attempted
   kind, hook/static/runtime/frame/action identity, and both receiver pointers. The
   failure descriptor binds that context to one nonzero reason and required length. The
   bootstrap descriptor is retained as the fallback for internal failures with no active
   reservation, and each reservation retains its descriptor through prepare/commit. The
   transport captures current thread ID, QPC ticks,
   write/read cursors, dropped count, required length, and offending sequence. Thus
   reservation failure and explicit failure use the same exact error encoder.

   `bootstrap` opens only the host-created named mapping, validates every immutable
   geometry/config field and HostInitialized status, writes Initializing, captures
   process/thread/QPC frequency, encodes and commits CaptureStart as ordinary sequence 1,
   then writes Ready last. Its descriptor must name CaptureStart, the bound global hook,
   and zero receiver pointers. Failure before Ready uses the retained descriptor and the
   same emergency encoder when a valid mapping exists. It creates no mapping, thread,
   heap allocation, file, lock, or wait in the target.

   No public function, `#[no_mangle]` symbol, callback body, build-script export,
   generated-manifest hook ID, environment overlay, or bootstrap call is added.
7. `reserve` rejects length 0 and 4,081 before touching a normal slot. It rejects a
   consumer cursor ahead of the producer, a full ring, a call after committed
   `CaptureEnd`, and a next ordinary sequence above the immutable budget. Every such
   in-session failure attempts exactly one emergency error using the supplied descriptor;
   a full ring never overwrites an unacknowledged normal slot.
8. `prepare` writes a zero committed marker, exact length, CRC, zero reserved word, and
   record bytes; it zeroes unused slot bytes. `commit` matches the existing startup
   publication sequence exactly: `fence(Ordering::Release)`, then a locked interlocked/
   `xchg` write of the slot committed sequence, then the producer's local counter update,
   then a locked interlocked/`xchg` write of the control-header committed sequence. Do
   not describe or replace this with a generic full fence. Before publishing a
   `CaptureEnd`, `commit` decodes its fixed terminal payload and validates the would-be
   inclusive kind counts, record count, checked-`u64` raw-byte total, and terminal sequence against the
   producer's exact counters including that candidate record. A mismatch publishes the
   sole emergency error from the retained descriptor and never commits the invalid
   `CaptureEnd` to the normal ring.
9. Use the existing bit-at-a-time IEEE CRC-32 polynomial/semantics and assert
   `crc32(b"123456789") == 0xCBF43926`. Do not add a crate.
10. Model terminal errors once and always in the dedicated emergency slot. Let
    `last_normal = WRITE_SEQUENCE` and `terminal_sequence = last_normal.checked_add(1)`.
    The error header sequence, emergency slot committed marker,
    `EMERGENCY_SEQUENCE`, payload offending sequence, and—after collector persistence—
    `EMERGENCY_ACK` must all equal that successor. `WRITE_SEQUENCE` remains the last
    ordinary record. On early failure the successor may still be within the ordinary
    budget; on budget exhaustion it is `budget + 1`; at the maximum budget it is
    `u32::MAX`. No ordinary record may use the successor after failure. Publish record
    bytes/CRC, release fence, locked slot marker, locked `EMERGENCY_SEQUENCE`, terminal
    count/reason, and `ProducerStatus::Terminal` last. A second terminal attempt, occupied
    emergency slot, or impossible checked successor is capture-invalid and may only latch
    terminal control state; it must never overwrite prior terminal evidence.
11. `complete()` is the only clean terminal transition. It verifies that the latest
    ordinary committed record is a successful `CaptureEnd`, that its header sequence,
    payload terminal sequence, and `WRITE_SEQUENCE` are equal, that its inclusive
    kind/record/byte counts are self-consistent, and that no emergency/error is present.
    It then publishes terminal count one, reason `None`, and
    `ProducerStatus::Terminal` last using locked control writes. `reserve` is forbidden
    after `CaptureEnd` even before `complete()`; `complete()` without that exact last
    record or a second `complete()` fails closed. If this post-commit transition detects
    corrupted control state, the stream is capture-invalid even if no second terminal
    record can be published; it may not reinterpret the committed end as clean.

### Unit tests

The module's tests use an aligned in-memory mapping fixture and the same production
offset/reservation helpers. Add tests named:

- `movement_layout_is_exact_and_distinct`
- `crc32_matches_ieee_check_vector`
- `maximum_record_round_trips_in_one_slot`
- `zero_and_oversized_records_publish_error_without_normal_advance`
- `commit_marker_is_published_last`
- `consumer_ack_allows_wrap_without_overwrite`
- `full_ring_latches_failure_before_overwrite`
- `early_failure_uses_exact_next_sequence_in_emergency_slot`
- `record_budget_exhaustion_publishes_exact_successor_error`
- `maximum_budget_uses_u32_max_only_for_terminal_error`
- `producer_status_and_terminal_reason_matrix_is_exact`
- `consumer_cursor_ahead_is_terminal`
- `slot_reserved_word_and_unused_tail_are_zero`
- `emergency_terminal_slot_is_single_use`
- `clean_complete_requires_last_committed_capture_end`
- `capture_end_counts_bytes_and_terminal_sequence_include_itself`
- `reserve_after_capture_end_and_second_complete_fail_closed`
- `failure_descriptor_is_preserved_in_capture_error`
- `bootstrap_validates_identity_and_commits_capture_start_before_ready`

Retain and rerun the existing `lib.rs` unconfigured export/callback test unchanged; do
not add a duplicate test. The only `lib.rs` source change is the private module line, and
no test calls the environment/named-mapping bootstrap entry. Module tests exercise the
same `bootstrap_inner` logic against the aligned in-memory mapping fixture.

### Task validation

```powershell
rustfmt --edition 2024 --check tools/oracle_instrument/src/movement_transport.rs tools/oracle_instrument/src/lib.rs
if ($LASTEXITCODE -ne 0) { throw 'instrument rustfmt check failed' }
python -B -m unittest tools/oracle_instrument/tests/test_smoke.py
if ($LASTEXITCODE -ne 0) { throw 'instrument smoke test failed' }
```

Do not run the Syringe smoke executable, create a mapping outside the unit-test process,
or set `VERA20K_ORACLE_MOVEMENT_MAPPING` in the operator environment.

## Task 4: Add Stateful Movement Validation and the Fail-Closed Collector

**Files:**

- Modify `tools/oracle_harness/oracle_harness/movement_wire.py`.
- Create `tools/oracle_harness/oracle_harness/collectors/oracle_movement.py`.
- Modify `tools/oracle_harness/oracle_harness/collectors/__init__.py`.
- Modify `tools/oracle_harness/tests/test_movement_wire.py`.
- Create `tools/oracle_harness/tests/test_oracle_movement.py`.

### Implementation

1. Extend Task 2's independently tested stateless decoder with the cross-process read
   boundary: copy one committed record to immutable `bytes`, and pass only that private
   copy to `struct.unpack_from`. Plain unpacking against the live mapping is forbidden.
2. Add a stateful `MovementSequenceValidator` that owns all cross-record rules:
   `CaptureStart` uniquely first; exactly one matching `CommandConsumed`; object and
   correlation identity stable; normal sequences contiguous; QPC ticks nondecreasing;
   and wrapping frame deltas from CaptureStart nondecreasing within the scenario's
   declared span below `2^31` while every raw frame remains retained;
   indexed path/retry/RNG members complete and ordered; snapshot path count/hash equals
   the reconstructed signed-direction stream; raw/API RNG counts equal their complete
   candidate sequence; required count bounds honored; one `CaptureEnd` uniquely last on
   success; and the sole emergency `CaptureError` is exactly the successor of the last
   normal sequence on failure. Overflow, a normal sequence outside `1..=budget`, a gap,
   duplicate end/error, or forced teardown is terminally invalid.
   Its `OrdinaryMtnkHostDfa` is table-driven and freezes this bounded Unit→Foot trace,
   where `Passed` always means “continue to the next native host segment”:

   ```text
   ObjectPassEnter -> TechnoPreThroughRocking -> GuardB
     GuardB Failed -> FootPostTechnoGate Failed -> FootReturn
     GuardB Passed -> TechnoRemainingPre -> MissionDispatchEnter
       -> ObjectAiEnter -> ObjectAiReturn -> DispatchActiveGate
       DispatchActiveGate Failed -> EARLY_POST
       DispatchActiveGate Passed -> DispatchTimerGate
         DispatchTimerGate Failed -> EARLY_POST
         DispatchTimerGate Passed -> DispatchHealthGate
           DispatchHealthGate Failed -> EARLY_POST
           DispatchHealthGate Passed -> UNIT_MOVE

   UNIT_MOVE:
     UnitMoveRead6E0 -> UnitMoveClear6D2 -> UnitMoveCheckSaved6E0
       failed -> QueueMission -> DISPATCH_WRITES
       passed -> UnitMoveReadCheck6E1
         failed -> QueueMission -> DISPATCH_WRITES
         passed -> UnitMoveReadCheck6E2
           failed -> QueueMission -> DISPATCH_WRITES
           passed -> UnitTrackerCheck
             failed -> UnitTrackerRestart -> FOOT_MOVE
             passed -> FOOT_MOVE

   FOOT_MOVE:
     FootMissionMoveEnter -> NavComGate
       passed (non-null) -> MOVE_TIMER
       failed (null) -> NullLocomotorInvariant Passed -> IsMovingCall
         passed (moving) -> MOVE_TIMER
         failed (stopped) -> QueuedMissionGate
           failed (queued mission present) -> MOVE_TIMER
           passed (queued mission == -1) -> OnArrival -> DISPATCH_WRITES
   MOVE_TIMER:
     MoveRateLookup -> ScenarioRandomRanged -> exact RngEvent API/candidate sequence
       -> DISPATCH_WRITES
   DISPATCH_WRITES:
     DispatchWriteStart -> DispatchWriteScratch -> DispatchWriteDelay -> EARLY_POST

   EARLY_POST:
     PassiveAcquire -> Bomb -> SlaveManager -> CaptureManager -> GuardE
       GuardE Failed -> FootPostTechnoGate Failed -> FootReturn
       GuardE Passed -> TechnoLatePost -> FootPostTechnoGate Passed -> FootPreProcess
         -> FootProcessGate member 0 -> 1 -> 2 -> 3 -> 4
         any gate Failed -> FootLaterWork -> FootReturn
         all Passed -> NullLocomotorInvariant Passed
           -> LocomotorProcessEnter -> LocomotorProcessReturn -> FootPostProcessGate
             Failed -> FootReturn
             Passed -> FootLaterWork -> FootReturn
   ```

   The Unit byte checks preserve load/short-circuit order: saved `+0x6E0` is checked
   after clearing `+0x6D2`; `+0x6E1` and `+0x6E2` are read only if reached. Arrival emits
   no Move-jitter RNG; timer branches emit one API call and one-or-more raw candidates.
   Guard-B/Guard-E early Techno returns must be followed by a failed Foot post-Techno
   gate, never an injected pass. The bounded DFA ends at `FootReturn`; Unit's intervening
   post-Foot tail and delayed Unit active guard are explicitly outside this record family.
   A failed `DispatchActiveGate` also carries inactive state to `GuardE` and the Foot
   post-Techno gate; injected `Passed` results on either later gate are invalid.
   `ObjectAiEnter.before` and `ObjectAiReturn.after` are mandatory raw `+0x90` bytes in
   that exact position, and `DispatchActiveGate.before` must equal the return byte.
3. The validator receives an immutable `MovementCollectionContract` built from the
   scenario. It contains expected `MovementOrigin::NativeHook`, all ten full
   `CaptureStart` hashes, observed GameSpeed, QPC frequency, maximum ordinary-record
   budget, maximum wrapping frame delta, the full scenario/manifest hashes, 16-byte
   routing correlations, allowed
   semantic hook IDs plus exact static/runtime-address and receiver relation, expected
   object/type/owner/locomotor, expected thread policy, and per-kind min/max counts. Each
   synthetic hook binding declares one receiver relation: `none`, `complete_object`, or
   `signed_adjustment(i32)`. `none` requires both raw pointer fields zero;
   `complete_object` requires equal nonzero pointers; `signed_adjustment` requires the
   complete pointer plus the signed adjustment to remain in `1..=u32::MAX` and equal the
   raw receiver. Every nonzero complete-object pointer must equal the command receipt's
   raw object pointer and its capture-local alias. This freezes the validation mechanism
   without hardcoding a real native hook address, receiver, or adjustment: O-07 remains
   blocked.
4. `OracleMovementCollector` owns a fresh mapping whose Python constants exactly match
   Task 3. It accepts only an `instrumented-state` lane and one `RunBundle`; a clean lane
   is a constructor error.
5. Use exact artifact names/classes:

   | Path | Artifact class |
   |---|---|
   | `movement/raw-records.bin` | `movement_raw_records` |
   | `movement/normalized-events.jsonl` | `movement_normalized_events` |
   | `movement/transport-initial.json` | `movement_transport_snapshot` |
   | `movement/transport-final.json` | `movement_transport_snapshot` |

   Both snapshots use schema string `oracle-movement-transport.v1` and exact top-level
   keys `schema_version`, `mapping_name`, `nonce`, `slot_count`, `total_len`, `header`,
   and `collector`. `header` losslessly names every non-reserved control-header field;
   `collector` contains read/emergency acknowledgements plus record count and all nine
   kind counts. Initial state requires HostInitialized, zero producer PID/TID/QPC,
   cursors/counts/errors, and exact host-seeded config identity. Final state requires the
   clean or error terminal matrix, nonzero producer/QPC identity once bootstrap occurred,
   and the same immutable geometry/identity fields.

6. `start()` creates the mapping, opens raw then normalized writers, validates the fully
   host-initialized HostInitialized header/nonce/scenario/manifest/run/budget identity,
   including the required zero producer-owned fields, and
   durably atomically writes `movement/transport-initial.json` through
   `RunBundle.write_json` before setting `_started`, publishing a drain thread, or
   returning. If any step fails, close normalized writer, raw writer, and mapping in
   reverse order; retain the primary failure and do not expose an environment overlay or
   started collector.
7. On each committed slot:
   - read the aligned committed sequence with the same Python pattern as `_read_u32_at`
     in `oracle_startup.py`: `ctypes.c_uint32` aligned load bracketed by
     `FlushProcessWriteBuffers` calls. Do not call it interlocked; Win32 exposes
     `InterlockedCompareExchange` as a compiler intrinsic, not a callable export here;
   - validate length `120..=4080` and the zero slot-prefix reserved word;
   - copy the bytes;
   - reread committed sequence and reject change;
   - verify IEEE CRC-32 before decode;
   - decode and validate protocol/identity/order;
   - append raw bytes with `durable=True` and record offset/length/SHA-256;
   - validate the normalized document against `ground-movement-event.v1`, then append it
     with `durable=True`, referencing that raw extent/hash;
   - only then acknowledge `READ_SEQUENCE` through an aligned `ctypes.c_uint32` store
     bracketed by `FlushProcessWriteBuffers` calls.
   Plain `struct.unpack_from` on the live mapping is forbidden for cursor fields.
8. Normalization must include all raw scalar fields, raw receiver/complete-object/runtime
   addresses, full
   RNG words and indices, every candidate, API/raw counts, semantic enum labels, and
   ordered occurrence. Capture-local aliases are additional keys only.
9. When the control header announces an emergency committed sequence, the collector must
   drain exactly one emergency `CaptureError`, verify its committed marker/length/CRC,
   decode and durably persist raw plus normalized evidence, then acknowledge the emergency
   sequence with the aligned fenced store. The slot marker, record-header sequence,
   payload offending sequence, `EMERGENCY_SEQUENCE`, and final `EMERGENCY_ACK` must all
   equal `WRITE_SEQUENCE + 1`; this is the sole stateful exception to normal-ring
   contiguity. Missing, duplicated, non-error, or second emergency records are
   capture-invalid; the terminal control header alone is not a substitute.
10. `stop()` preserves one primary failure while always completing cleanup. It signals and
    joins the drain thread, performs the final drain, samples and validates the immutable
    header plus terminal/count/completion state, and durably atomically writes
    `movement/transport-final.json` while the mapping is still live and the bundle is
    writable. Then nested `finally` blocks close normalized writer, raw writer, and
    mapping in reverse ownership order and set `_stopped`. Only after all cleanup does it
    return or raise the preserved primary/thread/producer/collector/close failure with
    later cleanup failures retained as diagnostics. A normalized- or raw-writer close
    failure cannot prevent the already durable final snapshot, and no matching final
    object snapshot can substitute for missing intermediate evidence.
11. Keep `collectors/__init__.py` declarative only:

    ```python
    __all__ = ("oracle_instrument", "oracle_movement", "oracle_startup")
    ```

    Do not import the Windows module at package import time.

### Synthetic tests

Use a pagefile mapping owned by the test process and a synthetic producer helper. Do not
spawn, open, or inspect a retail process. Retain Task 2's stateless decoder tests and add
tests named:

- `test_path_retry_and_rng_sequences_are_ordered_and_complete`
- `test_path_hash_preimage_is_exact_signed_direction_queue`
- `test_native_host_dfa_accepts_each_frozen_branch`
- `test_native_host_dfa_rejects_swapped_missing_extra_or_branch_inconsistent_events`
- `test_collector_persists_raw_before_normalized_and_then_acknowledges`
- `test_normalized_append_is_durable_before_acknowledgement`
- `test_raw_mutation_breaks_sealed_bundle_verification`
- `test_crc_failure_does_not_acknowledge`
- `test_slot_change_during_copy_does_not_acknowledge`
- `test_wrong_origin_fails_even_when_every_other_identity_matches`
- `test_each_capture_start_hash_mismatch_fails_independently`
- `test_wrong_run_scenario_manifest_hook_thread_object_or_locomotor_fails`
- `test_wrong_receiver_complete_object_or_adjustment_fails`
- `test_gap_duplicate_extra_hit_overflow_and_forced_teardown_fail`
- `test_emergency_capture_error_is_drained_persisted_and_acknowledged_once`
- `test_emergency_sequence_marker_header_control_payload_and_ack_match`
- `test_missing_start_command_or_end_fails_even_when_final_state_matches`
- `test_required_kind_count_bounds_reject_noise`
- `test_clean_lane_cannot_construct_movement_collector`
- `test_stop_preserves_primary_failure_and_closes_owned_resources`
- `test_start_persists_initial_snapshot_before_thread_or_return`
- `test_start_failure_cleans_partial_resources_without_publishing_started_state`
- `test_final_snapshot_survives_normalized_or_raw_writer_close_failure`
- `test_cleanup_errors_do_not_replace_primary_stop_failure`

### Task validation

```powershell
python -B -m unittest tools/oracle_harness/tests/test_movement_wire.py tools/oracle_harness/tests/test_oracle_movement.py
if ($LASTEXITCODE -ne 0) { throw 'movement decoder/collector tests failed' }
```

Expected result: synthetic mappings and temporary bundles are cleaned by their test
fixtures; no retail process, runtime evidence root, enrollment file, or facade command is
touched.

## Task 5: Add Pure Checkpoint Reconciliation and a Fakeable Two-Lane Coordinator

**Files:**

- Create `tools/oracle_harness/oracle_harness/movement_orchestration.py`.
- Create `tools/oracle_harness/tests/test_movement_orchestration.py`.
- Read only `adapters/original_yr.py`, `controller.py`, and `artifacts.py`.

### Implementation

1. Do not add a second state machine. The coordinator must call
   `OriginalYrWorkflow.checkpoint_creation` for proof materialization tests and
   `OriginalYrWorkflow.standard_move` for each run, execute transitions through
   `WorkflowController`, and create/seal artifacts only through `RunBundle`.
2. Make lane substitution impossible at the type boundary:

   ```python
   @dataclass(frozen=True)
   class InstrumentedStateLane:
       repeat_index: int

   @dataclass(frozen=True)
   class CleanPresentationLane:
       repeat_index: int

   MovementLane = InstrumentedStateLane | CleanPresentationLane

   @dataclass(frozen=True)
   class OfflineMovementCohort:
       canonical_scenario_bytes: bytes
       scenario_sha256: str
       comparison_policy_sha256: str
       instrumented: tuple[InstrumentedRunResult, InstrumentedRunResult]
       clean: tuple[CleanRunResult, CleanRunResult, CleanRunResult]
   ```

3. Accept injected backend-lease, collector, clock, and evidence-finalizer factories.
   The backend factory returns both an `OriginalYrBackend` and its idempotent cleanup
   callback; `OriginalYrBackend` itself has no `close()` method. Register that callback
   immediately through `WorkflowController.add_cleanup` before any transition can run.
   The module must not import `win32`, `win32_raw_input`, `oracle_movement`,
   `oracle_instrument`, `native_dxgi_capture`, `subprocess`, or `ctypes`. Production
   factory absence is intentional and fail-closed.
4. Build the run plan from the validated scenario only. Reject caller-supplied repeat
   counts, lane labels, action identity, destination, timeouts, artifact classes, or
   retries that differ from the scenario.
5. Convert the scenario's exact positive timeout map with
   `{name: milliseconds * 1_000_000 for ...}` and pass the resulting `timeouts_ns` plus
   the scenario's exact zero-retry map to `OriginalYrWorkflow.standard_move`. Reject a
   Boolean, non-integer, nonpositive millisecond value or a key mismatch before bundle
   creation.
6. Create exactly two instrumented bundles followed by exactly three clean bundles.
   Every run gets a fresh `RunBundle`, run ID, backend, collector set, and controller.
   All five bind the same canonical scenario bytes/hash and retail/map/rules identity;
   lane-specific tool manifests remain distinct and recorded.
7. Write the same canonical scenario bytes into every bundle as
   `scenario/ground-movement-scenario.v1.json` with class
   `ground_movement_scenario`. Write and validate the strict lane identity document at
   `identity/ground-movement-lane-identity.v1.json` with class
   `ground_movement_lane_identity` before starting the controller. Open
   `controller/events.jsonl` with class `controller_events` and wrap it in the existing
   `DurableJsonlEventSink`.
8. The instrumented factory must yield exactly one movement binding with
   `start_before="identify_window"` and `stop_before="exit"`. Clean factories yield the
   declared presentation and cadence bindings, each with `start_before="select_mtnk"`
   and `stop_before="exit"`, and no movement collector. These are the earliest feasible
   offline/fake boundaries accepted by the existing controller: `identify_window` is
   immediately after launch, while clean capture begins before both action-bearing
   transitions. They do not solve the future live pre-launch mapping/environment seam;
   that remains blocked and requires a separately reviewed owner. Validate exact binding
   names, boundaries, and artifact classes before transition execution.
9. Construct the controller exactly as shown and use the literal initial state:

   ```python
   controller = WorkflowController(
       run_id=run_id,
       sink=DurableJsonlEventSink(writer),
       clock_ns=clock_ns,
       diagnostics=backend.diagnostics,
   )
   result = controller.run(
       initial_state=YrState.NEW.value,
       transitions=transitions,
       collector_bindings=bindings,
   )
   ```
10. Preserve the existing standard-move transition order and no-retry policy. In
   particular, `select_mtnk` and `issue_move` remain non-retryable; `issue_move` remains
   the no-rewind boundary. The coordinator never retries a failed run or substitutes a
   later run for it.
11. After `run()` returns, close the controller-event writer and capture, rather than
    replace, any close exception. Call
    `controller.normalize_evidence_result(result, writer_close_error=close_error)`.
    `run()` normally returns a failed `WorkflowResult`; it need not raise. Test
    `result.succeeded` only after normalization. A false result forbids evidence
    finalization, bundle sealing, and construction of the next run.
12. If a run fails, stop the whole cohort. Let `WorkflowController` unwind collectors and
   registered cleanups in reverse order. Preserve the primary failure plus cleanup
   diagnostics, do not seal a success bundle, and do not start the next repeat.
13. On success, require lane-specific evidence finalization and required-member checks
    before `RunBundle.seal()`. Return sealed roots and typed lane results; do not write a
    parity verdict.
14. Expose only an offline/programmatic entry point:

    ```python
    def run_offline_movement_cohort(
        *,
        scenario: MaterializedGroundMovementScenario,
        runs_root: Path,
        factories: OfflineMovementFactories,
    ) -> OfflineMovementCohort:
        plan = MovementRunPlan.from_scenario(scenario)
        instrumented = tuple(
            _run_instrumented(plan, repeat_index, runs_root, factories)
            for repeat_index in range(2)
        )
        clean = tuple(
            _run_clean(plan, repeat_index, runs_root, factories)
            for repeat_index in range(3)
        )
        return OfflineMovementCohort(
            canonical_scenario_bytes=scenario.canonical_bytes,
            scenario_sha256=scenario.sha256,
            comparison_policy_sha256=plan.comparison_policy_sha256,
            instrumented=_require_instrumented_pair(instrumented),
            clean=_require_clean_triple(clean),
        )
    ```

    Do not import this entry point from `cli.py` and do not add a command.

### Focused tests

- `test_coordinator_creates_two_instrumented_and_three_clean_fresh_runs`
- `test_all_runs_bind_identical_scenario_and_retail_identity`
- `test_lane_specific_collectors_and_artifacts_cannot_substitute`
- `test_caller_cannot_override_repeat_counts_action_or_retries`
- `test_issue_move_failure_is_never_retried_or_rewound`
- `test_select_mtnk_partial_failure_is_never_retried_and_stops_cohort`
- `test_failure_stops_cohort_before_next_repeat`
- `test_cleanup_runs_in_reverse_order_and_primary_failure_wins`
- `test_failed_run_is_not_sealed_as_success`
- `test_each_success_bundle_has_exact_required_members_and_classes`
- `test_controller_result_is_normalized_before_finalization_or_seal`
- `test_controller_writer_close_failure_blocks_finalization_and_seal`
- `test_backend_lease_cleanup_is_registered_and_runs_without_close_method`
- `test_collector_boundaries_are_exact_and_future_prelaunch_seam_remains_absent`
- `test_coordinator_has_no_live_backend_or_cli_import`

The two no-retry tests independently configure the fake backend to fail after
`select_mtnk` or `issue_move` begins. Each asserts one call to that operation, no retry,
zero later transition calls, zero next-run factory calls, and no sealed success marker.

### Task validation

```powershell
python -B -m unittest tools/oracle_harness/tests/test_original_yr_adapter.py tools/oracle_harness/tests/test_controller.py tools/oracle_harness/tests/test_movement_orchestration.py
if ($LASTEXITCODE -ne 0) { throw 'movement orchestration tests failed' }
```

## Task 6: Add Movement Bundle Validation and the Two-Lane Comparator

**Files:**

- Create `tools/oracle_harness/oracle_harness/movement_compare.py`.
- Create `tools/oracle_harness/tests/test_movement_compare.py`.
- Create `tools/oracle_harness/schemas/ground-movement-field-roles.v1.schema.json`.
- Create `tools/oracle_harness/policies/ground-movement-field-roles.v1.json`, creating
  the currently absent `tools/oracle_harness/policies/` parent directory first.
- Modify `tools/oracle_harness/oracle_harness/ground_movement_scenario.py`.
- Modify `tools/oracle_harness/oracle_harness/schema.py`.
- Modify `tools/oracle_harness/tests/test_ground_movement_scenario.py`.
- Modify `tools/oracle_harness/tests/test_schema.py`.
- Read only `artifacts.py`, `compare.py`, `evidence.py`, and `pixels.py`.

### Implementation

1. Add `ground-movement-field-roles.v1` to `SCHEMA_CONTRACTS` and update the expected
   contract count from 15 to 16. Delegate its strict nested validation through a
   function-local import without introducing an eager schema cycle. Build the canonical
   policy over exactly the finite `movement_event` and `lane_identity` schema surfaces:
   - deterministic includes schema/scenario/lane values used for semantic comparison;
     sequence/kind/length/flags; semantic origin/hook/static-address/status/reason;
     scenario/manifest/action correlations; frame-delta and capture-local aliases; every
     non-pointer gameplay/protocol payload field; all full bound hashes; GameSpeed; and
     both CaptureStart and lane-environment QPC frequency; normalized raw byte offset and
     length are deterministic because record order/length are part of the comparison;
   - run-local includes run ID/correlation, repeat index, process/thread IDs, module base,
     runtime address, both raw receiver/object pointers and every other raw native
     pointer, raw artifact path/hash, creation/capture UTC, and raw
     original frame while its wrapping delta remains deterministic;
   - measurement includes absolute per-record QPC ticks. Present timestamps, cadence
     samples, and derived deltas remain in the separate typed clean evidence rather than
     being invented as leaves of these two schemas.

   The policy file spells out every concrete escaped pointer, with no wildcard, prefix,
   index, or fallback. An exhaustive test expands every event kind plus the lane-identity
   schema and proves each leaf occurs in exactly one role. It rejects any missing,
   duplicate, extra, or occurrence-indexed entry. Clean projections remain typed
   `OriginalRunEvidence`: exact scenario-derived prerequisite/sequence/evidence-kind
   names feed `StabilityRequirements`; each `SequenceRecord.comparable` mapping must have
   exactly its scenario-declared `comparable_fields`, then the unchanged comparator
   compares that mapping literally.
2. Keep `compare_original_runs` unchanged and reuse it for the deterministic/pixel
   projection of exactly three clean runs. Build that projection explicitly; raw QPC and
   cadence fields remain in sealed raw/normalized artifacts and in a separate measurement
   report rather than being passed as deterministic values.
3. Define non-parity outcome vocabulary:

   ```python
   class MovementCohortStatus(str, Enum):
       COHORT_CONSISTENT = "COHORT_CONSISTENT"
       COHORT_UNSTABLE = "COHORT_UNSTABLE"
       CAPTURE_INVALID = "CAPTURE_INVALID"

   @dataclass(frozen=True)
   class MovementCohortComparison:
       status: MovementCohortStatus
       scenario_sha256: str
       instrumented_run_ids: tuple[str, str]
       clean_run_ids: tuple[str, str, str]
       deterministic_differences: tuple[Mapping[str, object], ...]
       clean_comparison: StabilityComparison | None
       measurement_series: tuple[Mapping[str, object], ...]
       retained_run_local: tuple[Mapping[str, object], ...]
       withheld_claims: tuple[str, ...]
       blockers: tuple[str, ...]
   ```

   Do not use `VERIFIED`, `PASS`, `ORACLE_STABLE`, or a production parity status for
   the combined offline result.
4. `verify_movement_bundle_contract` first calls `verify_bundle`, then checks exact
   required paths and artifact classes for that lane. Missing, extra-forbidden,
   case-altered, or class-altered required members make the capture invalid even when
   generic bundle hashing passes. It validates every normalized event against
   `ground-movement-event.v1` and the lane identity against
   `ground-movement-lane-identity.v1` before inspecting values.
5. Require exactly two instrumented run IDs and three clean run IDs, with all five IDs
   pairwise distinct across both lanes. All five must bind the same scenario; executable
   canonical path, file version,
   size/hash; map filename/size/hash; YR/base-rules size/hash; sealed environment ID/hash;
   QPC frequency; observed live GameSpeed; Oracle HEAD/dirty disposition; and artifact-
   policy hash. Validate the
   lane-specific tool/module/callback schema identities described above. Capture UTC,
   process ID and module base are structurally required but remain per-run values.
   Instrumented manifests must match each other; clean manifests must match each other;
   cross-lane manifest equality is not required and cannot be used for substitution.
6. Parse and validate `cohort.canonical_scenario_bytes`, recompute and compare
   `cohort.scenario_sha256`, then read the exact comparison-policy SHA from that validated
   document. Load the supplied policy path only if its canonical SHA equals both the
   scenario value and `cohort.comparison_policy_sha256`. Walk every
   normalized leaf in occurrence order and classify it exactly once. Any new/unclassified
   field is `CAPTURE_INVALID`, not ignored.
7. Compare the two instrumented runs literally after projecting only fields classified
   deterministic. Keep record kind, ordinal, semantic event, field path, both exact
   values, and both raw-artifact references for every difference. A reorder or one-bit
   change makes the cohort unstable.
8. Preserve all run-local values in `retained_run_local` keyed by run/record/path; do
   not collapse raw pointers to aliases or delete original values. The aliases themselves
   remain deterministic only when the policy names them.
9. Preserve every QPC/present/cadence sample in `measurement_series` for all five runs.
   Derive inter-sample deltas with checked integer subtraction and report them per run.
   Never compare natural absolute QPC values for equality, average away samples, or
   invent a timing tolerance.
10. Derive `MovementComparisonRequirements` exclusively from the validated scenario and
   the hash-bound policy: required command, fixture preconditions, exact kind/count
   bounds, lane artifact classes, pixel surfaces, terminal tuple and measurement series.
   There is no caller-supplied requirements object. If an internal helper receives an
   expected requirements value, it must equal the derived value exactly; omission or
   weakening is `CAPTURE_INVALID`.
11. Before any consistency result, require complete command receipt, preconditions,
   required state/order records, pixels for clean lanes, per-kind counts, no overflow,
   one terminal end, and sealed bundle integrity. Missing evidence is
   `CAPTURE_INVALID`, never stable.
    Preserve every canonical negative-control declaration in `withheld_claims`; this
    bounded primary cohort neither executes nor marks a companion control fulfilled.
12. The public API is read-only over sealed bundles:

    ```python
    def compare_movement_cohort(
        cohort: OfflineMovementCohort,
        *,
        field_policy_path: Path,
    ) -> MovementCohortComparison:
        scenario = validate_canonical_cohort_scenario(cohort)
        policy = load_bound_field_policy(
            field_policy_path,
            expected_sha256=scenario["comparison_policy"]["sha256"],
        )
        requirements = MovementComparisonRequirements.from_scenario(scenario, policy)
        verified = verify_complete_cohort(cohort, requirements)
        if verified.blockers:
            return invalid_movement_comparison(cohort, verified.blockers)
        instrumented = compare_instrumented_pair(verified.instrumented, policy)
        clean = compare_clean_triple(verified.clean, requirements.clean)
        return reconcile_movement_comparisons(
            cohort=cohort,
            instrumented=instrumented,
            clean=clean,
            policy=policy,
        )
    ```

    It creates no bundle, repairs no manifest, and writes no verdict file.

### Focused tests

- `test_exact_two_plus_three_complete_cohort_is_consistent_not_verified`
- `test_duplicate_or_wrong_lane_run_is_capture_invalid` (including a cross-lane run-ID
  collision)
- `test_cross_lane_scenario_or_retail_identity_mismatch_is_invalid`
- `test_one_bit_instrumented_state_change_is_unstable`
- `test_instrumented_record_reorder_is_unstable`
- `test_raw_pointer_and_qpc_variation_remain_retained_not_drift`
- `test_unclassified_or_multiply_classified_field_is_invalid`
- `test_field_role_policy_is_disjoint_exhaustive_and_canonical`
- `test_three_clean_runs_still_use_literal_existing_comparator`
- `test_clean_comparable_fields_match_exact_scenario_inventory`
- `test_first_bgra_difference_is_retained`
- `test_all_raw_qpc_and_derived_deltas_are_reported_per_run`
- `test_missing_command_state_pixel_terminal_or_count_is_invalid`
- `test_caller_cannot_weaken_scenario_derived_comparison_requirements`
- `test_scenario_is_revalidated_before_bound_policy_is_loaded`
- `test_wrong_policy_hash_is_rejected_before_event_projection`
- `test_lane_identity_size_filename_tools_environment_or_gamespeed_mismatch_is_invalid`
- `test_required_artifact_class_change_is_invalid`
- `test_declared_negative_controls_remain_explicitly_withheld`
- `test_mutated_removed_unlisted_or_case_changed_member_fails`
- `test_comparator_is_read_only_over_sealed_bundles`

### Task validation

```powershell
python -B -m unittest tools/oracle_harness/tests/test_schema.py tools/oracle_harness/tests/test_ground_movement_scenario.py tools/oracle_harness/tests/test_artifacts.py tools/oracle_harness/tests/test_compare.py tools/oracle_harness/tests/test_movement_compare.py
if ($LASTEXITCODE -ne 0) { throw 'movement comparator tests failed' }
```

## Task 7: Preserve the Closed Facade and Run the Full Offline Validation

**Files:**

- Modify `tools/oracle_harness/tests/test_system_cli.py` only to add
  `oracle_harness.collectors.oracle_movement` to `FORBIDDEN_LIVE_MODULES` and assert
  the new offline modules are not imported by `status` or `capabilities`.
- No production CLI, registry, system-status, tool-lock, enrollment, manifest, or
  evidence file is modified.

### Static boundary assertions

Add/retain tests that prove:

1. `create-checkpoint` and `run-original` still use the exact `_not_implemented`
   handler object.
2. Both commands remain `STUB`; instrumentation, original-YR execution, and
   cross-engine comparison remain `BLOCKED`; `parity_authority` remains `NONE`.
3. Read-only facade commands do not import movement collector, live DXGI, instrument,
   raw-input, shell navigation, or any native module.
4. No source reference from `cli.py`, `oracle-system.v1.json`, `build.rs`, generated
   manifest, callback shim, or callback body points to the movement coordinator,
   transport bootstrap, or collector.
5. The existing legacy/startup transport constants, protocol kinds, golden vectors,
   and collector tests remain unchanged and pass without rebaselining.

### Focused Python validation

Run from the private Oracle root:

```powershell
python -B tools/oracle_protocol/verify_vector.py
if ($LASTEXITCODE -ne 0) { throw 'legacy protocol vector failed' }
python -B tools/oracle_protocol/verify_movement_vector.py
if ($LASTEXITCODE -ne 0) { throw 'movement protocol vector failed' }
python -B -m unittest tools/oracle_instrument/tests/test_smoke.py
if ($LASTEXITCODE -ne 0) { throw 'instrument smoke test failed' }
python -B -m unittest tools/oracle_harness/tests/test_schema.py tools/oracle_harness/tests/test_checkpoint_recipe.py tools/oracle_harness/tests/test_ground_movement_scenario.py tools/oracle_harness/tests/test_movement_wire.py tools/oracle_harness/tests/test_oracle_movement.py tools/oracle_harness/tests/test_oracle_startup.py tools/oracle_harness/tests/test_oracle_instrument.py tools/oracle_harness/tests/test_original_yr_adapter.py tools/oracle_harness/tests/test_controller.py tools/oracle_harness/tests/test_movement_orchestration.py tools/oracle_harness/tests/test_artifacts.py tools/oracle_harness/tests/test_compare.py tools/oracle_harness/tests/test_movement_compare.py tools/oracle_harness/tests/test_system_cli.py tools/oracle_harness/tests/test_system_status.py
if ($LASTEXITCODE -ne 0) { throw 'offline harness regression suite failed' }
```

These tests are closed/synthetic. Do not run `smoke.py`, `navigate-shell`,
`create-checkpoint`, `run-original`, any enrollment command, or a DXGI capture.

### Serial locked Cargo validation

First check for another owner's build:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
  Select-Object ProcessName,Id,CPU
```

If Cargo/Rustc is active and ownership is not known, wait. Otherwise run the two crates
serially, each with a fresh retained external target and with the inherited environment
restored:

```powershell
$priorCargoTarget = [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR', 'Process')
$priorManifestRs = [Environment]::GetEnvironmentVariable('VERA_ORACLE_MANIFEST_RS', 'Process')
try {
  Remove-Item Env:VERA_ORACLE_MANIFEST_RS -ErrorAction SilentlyContinue
  $targetParent = Join-Path $env:LOCALAPPDATA 'VERA20k\ground-movement-offline-checks'
  New-Item -ItemType Directory -Force -Path $targetParent | Out-Null

  $protocolTarget = Join-Path $targetParent ('protocol-' + [Guid]::NewGuid().ToString('N'))
  if (Test-Path -LiteralPath $protocolTarget) { throw 'fresh protocol target already exists' }
  $env:CARGO_TARGET_DIR = $protocolTarget
  cargo test --locked --offline --manifest-path tools/oracle_protocol/Cargo.toml
  if ($LASTEXITCODE -ne 0) { throw 'oracle_protocol cargo test failed' }

  $instrumentTarget = Join-Path $targetParent ('instrument-' + [Guid]::NewGuid().ToString('N'))
  if (Test-Path -LiteralPath $instrumentTarget) { throw 'fresh instrument target already exists' }
  $env:CARGO_TARGET_DIR = $instrumentTarget
  cargo test --locked --offline --manifest-path tools/oracle_instrument/Cargo.toml
  if ($LASTEXITCODE -ne 0) { throw 'oracle_instrument cargo test failed' }
}
finally {
  if ($null -eq $priorCargoTarget) {
    Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
  } else {
    $env:CARGO_TARGET_DIR = $priorCargoTarget
  }
  if ($null -eq $priorManifestRs) {
    Remove-Item Env:VERA_ORACLE_MANIFEST_RS -ErrorAction SilentlyContinue
  } else {
    $env:VERA_ORACLE_MANIFEST_RS = $priorManifestRs
  }
}
```

Read and record each literal `test result:` line. Do not run Cargo commands in parallel,
delete the retained targets, run `cargo clean`, or rebuild/rebaseline any golden.

### Facade and compatibility closeout

```powershell
$publicRoot = '.'
$privateRoot = '<local>/Documents/vera20k-oracle'
$ownershipGuardPath = Join-Path ([IO.Path]::GetTempPath()) (
  'vera20k-ground-movement-offline-ownership-guard-v1.json'
)
if (-not (Test-Path -LiteralPath $ownershipGuardPath -PathType Leaf)) {
  throw "retained ownership guard is missing: $ownershipGuardPath"
}
$statusJson = python -B tools/oracle_harness/oracle.py status --json
if ($LASTEXITCODE -ne 0) { throw 'oracle status command failed' }
$status = $statusJson | ConvertFrom-Json
if ($status.parity_authority -ne 'NONE') { throw 'parity authority changed' }
$pipelineStates = @($status.pipelines | ForEach-Object { $_.state } | Sort-Object -Unique)
if ($pipelineStates.Count -ne 1 -or $pipelineStates[0] -ne 'BLOCKED') {
  throw 'a blocked pipeline changed state'
}

$plannedPrivate = @(
  'tools/oracle_harness/schemas/ground-movement-scenario.v1.schema.json',
  'tools/oracle_harness/schemas/ground-movement-field-roles.v1.schema.json',
  'tools/oracle_harness/schemas/ground-movement-event.v1.schema.json',
  'tools/oracle_harness/schemas/ground-movement-lane-identity.v1.schema.json',
  'tools/oracle_harness/policies/ground-movement-field-roles.v1.json',
  'tools/oracle_harness/oracle_harness/ground_movement_scenario.py',
  'tools/oracle_harness/oracle_harness/movement_wire.py',
  'tools/oracle_harness/oracle_harness/collectors/oracle_movement.py',
  'tools/oracle_harness/oracle_harness/movement_orchestration.py',
  'tools/oracle_harness/oracle_harness/movement_compare.py',
  'tools/oracle_harness/tests/test_ground_movement_scenario.py',
  'tools/oracle_harness/tests/test_movement_wire.py',
  'tools/oracle_harness/tests/test_oracle_movement.py',
  'tools/oracle_harness/tests/test_movement_orchestration.py',
  'tools/oracle_harness/tests/test_movement_compare.py',
  'tools/oracle_protocol/src/movement_v3.rs',
  'tools/oracle_protocol/ground_movement_v3_golden.hex',
  'tools/oracle_protocol/verify_movement_vector.py',
  'tools/oracle_instrument/src/movement_transport.rs',
  'tools/oracle_harness/oracle_harness/schema.py',
  'tools/oracle_harness/oracle_harness/collectors/__init__.py',
  'tools/oracle_harness/tests/test_schema.py',
  'tools/oracle_harness/tests/test_system_cli.py',
  'tools/oracle_protocol/src/lib.rs',
  'tools/oracle_protocol/README.md',
  'tools/oracle_instrument/src/lib.rs'
)
$guard = Get-Content -Raw -LiteralPath $ownershipGuardPath | ConvertFrom-Json
if ($guard.schema -ne 'vera20k.ground-movement-offline-ownership-guard.v1') {
  throw 'wrong ownership guard schema'
}
function Assert-GuardHashes([string]$root, $properties, [string]$label) {
  foreach ($property in $properties.psobject.Properties) {
    $literal = Join-Path $root $property.Name
    if (-not (Test-Path -LiteralPath $literal -PathType Leaf)) {
      throw "$label path disappeared: $($property.Name)"
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $literal).Hash
    if ($actual -ne [string]$property.Value) {
      throw "$label path changed outside plan ownership: $($property.Name)"
    }
  }
}
$publicHeadAfter = git -C $publicRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'cannot re-read public HEAD' }
$publicStatusAfter = @(git -C $publicRoot status --porcelain=v1 -uall)
if ($LASTEXITCODE -ne 0) { throw 'cannot re-read public status' }
if ($publicHeadAfter -ne $guard.public_head) { throw 'public HEAD changed during implementation' }
if ((@($publicStatusAfter | Sort-Object) | ConvertTo-Json -Compress) -ne
    (@($guard.public_status) | ConvertTo-Json -Compress)) {
  throw 'public working-tree scope changed during implementation'
}
Assert-GuardHashes $publicRoot $guard.public_dirty_hashes 'public companion-owned'
Assert-GuardHashes $publicRoot $guard.public_read_only_hashes 'public authority/read-only'

$privateHeadAfter = git -C $privateRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'cannot re-read private HEAD' }
if ($privateHeadAfter -ne $guard.private_head) { throw 'private HEAD changed during implementation' }
Assert-GuardHashes $privateRoot $guard.private_dirty_hashes 'private companion-owned'
Assert-GuardHashes $privateRoot $guard.private_read_only_hashes 'private read-only'
$privateReadOnly = @($guard.private_read_only_hashes.psobject.Properties.Name)
$readOnlyDiff = @(git -C $privateRoot diff HEAD -- $privateReadOnly)
if ($LASTEXITCODE -ne 0) { throw 'private read-only diff audit failed' }
if ($readOnlyDiff.Count -ne 0) { throw 'a clean private read-only path changed' }

$privateStatusAfter = @(git -C $privateRoot status --porcelain=v1 -uall)
if ($LASTEXITCODE -ne 0) { throw 'cannot re-read private status' }
$privateDirty = @($guard.private_dirty_hashes.psobject.Properties.Name)
$initialPrivateStatus = @($guard.private_status)
foreach ($path in $privateDirty) {
  $before = @($initialPrivateStatus | Where-Object { $_.Length -ge 4 -and $_.Substring(3) -eq $path })
  $after = @($privateStatusAfter | Where-Object { $_.Length -ge 4 -and $_.Substring(3) -eq $path })
  if (($before | ConvertTo-Json -Compress) -ne ($after | ConvertTo-Json -Compress)) {
    throw "companion-owned git status changed: $path"
  }
}
$guardPlanned = @($guard.planned_private)
if ((@($plannedPrivate | Sort-Object) | ConvertTo-Json -Compress) -ne
    (@($guardPlanned | Sort-Object) | ConvertTo-Json -Compress)) {
  throw 'planned private allowlist differs from retained guard'
}
$plannedCreate = @($guard.planned_create)
if ($plannedCreate.Count -ne 19 -or
    @($plannedCreate | Sort-Object -Unique).Count -ne 19 -or
    @($plannedCreate | Where-Object { $plannedPrivate -notcontains $_ }).Count -ne 0) {
  throw 'planned create set is not a subset of the retained allowlist'
}
$allowed = @{}; @($privateDirty + $plannedPrivate) | ForEach-Object { $allowed[$_] = $true }
foreach ($line in $privateStatusAfter) {
  if ($line.Length -lt 4) { throw "malformed git status line: $line" }
  $code = $line.Substring(0, 2)
  $path = $line.Substring(3)
  if (-not $allowed.ContainsKey($path)) { throw "unplanned private path changed: $path" }
  if ($privateDirty -notcontains $path -and $code -ne ' M' -and $code -ne '??') {
    throw "planned path was staged or has unexpected status ${code}: $path"
  }
}
git -C $privateRoot diff --check -- $plannedPrivate
if ($LASTEXITCODE -ne 0) { throw 'planned-path whitespace check failed' }
$strictUtf8 = [Text.UTF8Encoding]::new($false, $true)
foreach ($relative in $plannedCreate) {
  $literal = Join-Path $privateRoot $relative
  if (-not (Test-Path -LiteralPath $literal -PathType Leaf)) {
    throw "planned implementation path is missing: $relative"
  }
  $bytes = [IO.File]::ReadAllBytes($literal)
  if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and
      $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
    throw "UTF-8 BOM is forbidden: $relative"
  }
  try { $text = $strictUtf8.GetString($bytes) }
  catch { throw "strict UTF-8 decode failed: $relative" }
  if ($text.Contains("`0") -or $text.Contains([char]0xFFFD)) {
    throw "NUL or replacement character found: $relative"
  }
  if ($text.Contains("`r")) { throw "non-LF line ending found: $relative" }
  if ($bytes.Length -eq 0 -or $bytes[$bytes.Length - 1] -ne 0x0A) {
    throw "final LF is required: $relative"
  }
  if ($text -match '(?m)[ \t]+$') { throw "trailing whitespace found: $relative" }
  if ($text -match '(?m)^(<<<<<<<|=======|>>>>>>>)') {
    throw "conflict marker found: $relative"
  }
}
Remove-Item -LiteralPath $ownershipGuardPath
```

The read-only `git diff HEAD --` output must be empty, every pre-existing dirty-file hash
must match its preflight value, and every remaining private status line must be in the
explicit companion/planned allowlist. The public HEAD/status, companion hash, plan,
contract, Checkpoint-E report, and load-bearing host/path reports must be byte-for-byte
unchanged. Direct strict-text validation covers untracked planned files that
`git diff --check` cannot see. No staging or commit is part of this plan.

## Acceptance Matrix

| Contract acceptance | Plan task(s) | Evidence produced by implementation |
|---|---:|---|
| A-01 canonical scenario | 1 | canonical-bytes/hash tests |
| A-02 no hidden reroll | 1 | explicit-attempt and candidate rejection tests |
| A-03 proof completeness | 1, 5 | permanently latched per-edge validator plus completed-event reconciliation |
| A-04 exact vectors | 2 | Rust + independent Python frozen corpus |
| A-05 strict protocol rejection | 2, 4 | codec/decoder invalid corpus plus swapped-host-event tests |
| A-06 ordered semantic sequences | 2, 4 | host/Unit/Foot DFA plus path/retry/RNG validator tests |
| A-07 geometry | 3, 4 | Rust/Python constant/layout assertions |
| A-08 maximum record | 3 | exact 4,080 and 4,081 transport boundary tests |
| A-09 ring safety | 3, 4 | commit/ack/wrap/full/budget-successor/emergency/cursor tests |
| A-10 compatibility | 2, 3, 7 | old goldens/transports unchanged and green |
| A-11 raw-before-normalized | 4 | durable raw/offset/hash/ack order test |
| A-12 stream integrity | 4 | expected-origin, ten-hash, receiver/identity/order/CRC/terminal rejection tests |
| A-13 required counts | 1, 4 | scenario count policy and extra-hit rejection |
| A-14 no lossy normalization | 4, 6 | raw pointer/RNG/QPC retention assertions |
| A-15 lane separation | 5, 6 | nominal lane and required-artifact tests |
| A-16 no retry after delivery | 5 | select-MTNK and issue-move partial-failure tests |
| A-17 cleanup/failure priority | 4, 5 | reverse cleanup and primary-error tests |
| A-18 facade remains closed | 7 | capabilities/status/import tests |
| A-19 instrumented stability | 6 | literal deterministic projection/diff tests |
| A-20 clean stability/cadence | 6 | existing comparator plus raw measurement series |
| A-21 completeness before verdict | 4, 6 | scenario-derived non-weakenable capture-invalid matrix |
| A-22 cohort identity | 5, 6 | five-run versioned lane-identity reconciliation |
| A-23 immutable bundle | 4, 6 | required-class plus generic seal mutation tests |
| A-24 workflow regression | 1, 5, 7 | existing and movement-specific no-retry tests |
| A-25 sealed substrate regression | 2, 3, 7 | old vector/callback/transport/startup and legacy collector tests |

## Explicit Non-Claims and Deferred Work

- No final MTNK checkpoint or production scenario is created.
- Negative controls are canonically declared and explicitly withheld; no companion
  native control scenario is executed or claimed complete.
- No native command-consumption owner, hook ID, static address, overwrite window,
  receiver, continuation, or hit bound is selected.
- No movement callback, exported DLL symbol, producer callsite, generated hook manifest,
  or live environment overlay is installed.
- The SyringeEx post-return `FS:[0x14]` mutation is not repaired or downgraded.
- No private runtime tool lock, DXGI binary, enrollment record, runtime evidence,
  sandbox, checkpoint, or runtime evidence/artifact store is created, replaced, sealed,
  cleaned, or deleted. Closed tests do create, seal, verify, and remove synthetic
  temporary `RunBundle` fixtures; those are conspicuously non-retail test artifacts.
- No native input, launch, attach, injection, debugger, screenshot, or capture occurs.
- No synthetic corrupt-speed producer is added.
- No AMCV/Walk/Hover/Ship/Teleport/Tube/miner/lifecycle fixture is claimed.
- No VERA20k adapter or production movement cutover is implemented.
- No INI behavior is introduced. `rulesmd.ini` and base `rules.ini` are opaque hashed
  scenario identities in this tooling slice; their gameplay contents are not parsed or
  reinterpreted here.
- `COHORT_CONSISTENT` means only that complete synthetic or later supplied retail
  artifacts agree under the named policy. It never by itself means gamemd parity.

## Sources & References

- **Approved design authority:**
  `docs/contracts/2026-07-22-ground-movement-native-oracle-capture-implementation-contract.md`,
  SHA-256 `F134D218E038407BDF104920FC60BC490FC0DA68B3C5B97E092DFABBDD096A46`.
- **Checkpoint-E readiness/source map:**
  `docs/research/GROUND_MOVEMENT_EXECUTABLE_NATIVE_ORACLE_CAPTURE_REPORT.md`, SHA-256
  `EE988EB689C55A3C8F8EF30CCEF20DAA1409EB5536AF67025F3632868E0512C6`.
- **Inherited host ordering:**
  `docs/research/TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md` and
  `docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`. This plan records their
  already-verified bounded event order; it selects no new address or hook.
- **Inherited movement prerequisites:**
  `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`,
  `docs/research/FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md`,
  `docs/research/DRIVE_RAWTRACK_METADATA_INITIALIZER_RECONCILIATION_GHIDRA_REPORT.md`,
  `docs/research/GROUND_PHASE1_LOCOMOTOR_POPULATION_AND_PRECEDENCE_GHIDRA_REPORT.md`, and
  `docs/research/GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP_GHIDRA_REPORT.md`.
- **Native Foot path-buffer shape used by the wire preimage:**
  `docs/research/BRIDGE_TRAVERSAL_STATE_GHIDRA_REPORT.md` §4.1 (24 signed dwords at
  `Foot+0x5E0..+0x63C`, `-1` terminator). The new SHA-256 preimage is a tooling-format
  decision over those verified raw direction values, not a claim that gamemd stores a
  hash.
- **Private source patterns at HEAD
  `7b8689edd2c5a26ec936caaa03d2c7c9bc31523e`:**
  `vera20k-oracle:tools/oracle_harness/oracle_harness/adapters/original_yr.py` for
  edges/proofs/no-retry rules;
  `vera20k-oracle:tools/oracle_harness/oracle_harness/controller.py` for collector
  lifetimes, cleanup and evidence normalization;
  `vera20k-oracle:tools/oracle_harness/oracle_harness/artifacts.py` for durable writers
  and sealed bundles; `vera20k-oracle:tools/oracle_harness/oracle_harness/compare.py` for
  the three-clean-run literal comparator;
  `vera20k-oracle:tools/oracle_harness/oracle_harness/collectors/oracle_startup.py` and
  `vera20k-oracle:tools/oracle_instrument/src/startup_transport.rs` for large-slot
  collection, aligned cursor access and commit-last publication;
  `vera20k-oracle:tools/oracle_protocol/src/startup_v2.rs` for explicit codec style; and
  `vera20k-oracle:tools/oracle_instrument/build.rs` plus
  `vera20k-oracle:tools/oracle_instrument/src/lib.rs` for the unconfigured
  instrumentation boundary.
- **Retail INI authority:** repo `ini/rulesmd.ini` and `ini/rules.ini` are bound by full
  hashes in the scenario. This offline tooling plan parses no gameplay key and introduces
  no hardcoded INI-derived behavior.
- **Planning git snapshots:** public HEAD
  `928644d44fee61d0bb8d3214a4f9e0eb7390bc4e`; private HEAD above. Both are rechecked by
  the ownership guard before any implementation.

## Implementation Stop and Handoff

After Tasks 1-7 pass:

1. freeze the private implementation diff and record hashes of every created/modified
   planned file;
2. have an independent reviewer audit protocol widths/reserved bytes, transport
   publication order, collector raw-before-normalized behavior, lane separation,
   comparator exhaustiveness, and the no-live-wiring boundary;
3. re-run the final status/read-only-path audit after review corrections; and
4. stop. Do not proceed to runtime hook research, enrollment, checkpoint creation, or
   retail capture under this plan.

The next separately planned work remains the bounded read-only hook-selection report for
`CommandConsumed` plus the minimum MTNK host/Drive/cell/arrival event set, followed by a
separate design for the loader-owned `FS:[0x14]` preservation boundary. Neither is
authorized by completion of this offline implementation.
