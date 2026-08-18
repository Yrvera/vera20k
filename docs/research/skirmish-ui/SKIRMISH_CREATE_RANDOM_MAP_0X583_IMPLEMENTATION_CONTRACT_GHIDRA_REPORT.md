# Skirmish Create Random Map 0x583 Implementation Contract - Ghidra Research Report

**Address(es):** `FUN_005e8590`, `FUN_00595bc0`, `FUN_00597730`, `FUN_00597a10`, `FUN_00598960`, `ScenarioClass__Read_Scenario`, `FUN_005e7160`, `FUN_0069a980`, `FUN_0069adf0`, `FUN_0069acd0`, `FUN_0069ad80`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Rust implementation contract for Choose Map command `0x583` only: command click, accepted random setup, `RandMap.Sed`/`RandMap.img` side effects, sentinel update/append, ordinary accept, and later `.SED` launch branch.  
**Non-Scope:** random terrain/noise formulas, random map dialog visual layout, generic RMG internals, online exchange behavior, and screenshot-exact `RandMap.img` pixels.  
**Confidence:** High for the UI command and launch handoff; Medium for exact runtime UX of malformed external `.SED` or missing/corrupt `RandMap.img` because static branches are bounded but not runtime-captured. Updated 2026-05-23 after current Rust re-scan.  
**Active in YR:** Conditional. This is live in standard YR offline Skirmish when the player clicks Create Random Map in Choose Map `0x6B` and the random-map dialog returns accepted result `1`; the launch branch is live when the selected filename suffix is `.SED`.

## Working Notes Gate

- Target question: What exactly must Rust implement for command `0x583` so Create Random Map is not a no-op and hands off to launch like native YR?
- Non-goals: Do not investigate terrain/noise formulas, full random-map dialog visuals, generic RMG internals, or ordinary map selection beyond the `0x583` handoff boundary.
- Evidence needed to mark COMPLETE: command branch, accepted-result gate, file names, min/max, official flag, selected token identity, preview side effect, and `.SED` launch branch.
- Stop conditions: Stop once the Rust-facing command contract is implementable; defer generator formulas, malformed-file UX, and preview pixel exactness.

Prior state row: **Partial/high-confidence reports exist; proceed to gaps + verification only.** This report reconciles the prior random-map setup, writer, preview-loader, and launch reports, and spot-checks only the handoff-critical binary boundaries. Fresh Ghidra note: several raw address decompiles still return "Function not found" in the read-only project, so this update uses fresh symbol-name decompiles where present, fresh assembly contexts for the missing-boundary ranges, and prior decompile-backed reports for unchanged broad bodies.

## Summary

Create Random Map `0x583` is a live Choose Map modal command, not a row highlight and not a log-only action. Native YR hides/suspends the chooser, calls `FUN_005e8590`, and aborts without side effects unless the random-map dialog returns exactly `1`. On accepted setup, it saves the global random seed/options object to `RandMap.Sed`, replaces the chooser preview wrapper from `RandMap.img`, update-or-appends exactly one official synthetic scenario record for `RandMap.Sed` with min players `2` and max players `4`, then re-enters normal selected-record and Use Map accept semantics.

Launch later still sees the selected filename `RandMap.Sed`. `ScenarioClass__Read_Scenario` detects the `.SED` suffix, loads `[RandomMap]` seed/options through `FUN_00597a10`, calls `FUN_00598960(0,0)` only if that load succeeds, and then copies the original `.SED` filename back to `ScenarioClass+0x125C`.

## Verified Findings

### 1. Command `0x583` is a real Choose Map branch

Active in YR: Yes / Conditional on the player clicking the Create Random Map button in dialog `0x6B`.  
Evidence: assembly at `005e69d3` subtracts `0x583` and jumps to `005e69fd`; `005e6a11` calls `FUN_005e8590`; `005e6a18..005e6a1f` compares the result with `-1` and skips accept work on failure.

Rust-facing implication: [src/app.rs](src/app.rs) must not leave `ChooseMapModalButton::CreateRandomMap0x583` as a log-only branch.

### 2. Accepted random setup is gated by exact return value `1`

Active in YR: Yes / Conditional on the random-map dialog completing successfully.  
Evidence: `FUN_005e8590` decompile calls `FUN_00595bc0`; if the result is not `1`, it returns `-1`. Assembly `005e85c1 CALL 0x00595bc0`, `005e85c6 CMP EAX,0x1`, `005e85cb OR EAX,0xffffffff`, `005e85ce RET`.

Rust-facing implication: clicking `0x583` must enter a setup/blocked state; Rust must not create, select, or commit `RandMap.Sed` merely on button press. Canceled setup preserves the previous selection.

### 3. Accepted setup saves seed/options to `RandMap.Sed`

Active in YR: Yes / Conditional on accepted setup.  
Evidence: assembly `005e85d1 PUSH 0x82bc30` (`RandMap.Sed`), `005e85d6 MOV ECX,0xabdfd8`, `005e85db MOV byte ptr [0x008316d4],0x1`, `005e85e2 CALL 0x00597730`. `FUN_00597730` dispatches vtable `+0x8` when passed a non-null filename.

Rust-facing implication: Rust needs a random-map setup/seed-options model and writer/launch-carried state. A display-only sentinel is insufficient.

### 4. Accepted setup replaces preview source from `RandMap.img`

Active in YR: Yes / Conditional on accepted setup; drawable result is conditional on image load success.  
Evidence: `FUN_005e8590` destroys existing `DAT_00AC1154`, constructs a new wrapper, pushes `0x829abc` (`RandMap.img`), stores the wrapper at `DAT_00AC1154`, then calls `0x00641db0`. `FUN_00595bc0` writes `RandMap.img` during random-map dialog teardown only when a generated preview wrapper exists.

Rust-facing implication: `RandMap.img` is a UI preview product, not gameplay terrain. Rust should clear stale concrete-map preview when accepted random setup occurs and should not decode `[PreviewPack]` from `RandMap.Sed`. Current Rust now has a sentinel preview branch and 1/3-plane PCX support, but the accepted setup lifecycle that writes/refreshes `RandMap.img` is still missing.

### 5. Native update-or-appends one sentinel record

Active in YR: Yes / Conditional on accepted setup.  
Evidence: `FUN_005e8590` scans `DAT_00A8B8CC[0..DAT_00A8B8D8)`, calls `FUN_0069adf0`, and updates the existing record when `record+0x58 == "RandMap.Sed"`. `FUN_0069adf0` decompile compares `param_1 + 0x58` with `s_RandMap_Sed_0082bc30`.

Rust-facing implication: [src/skirmish_scenarios.rs](src/skirmish_scenarios.rs) correctly has an upsert shape; implementation should call it only after accepted setup and must not append duplicates.

### 6. New sentinel fields are file `RandMap.Sed`, official `1`, min `2`, max `4`

Active in YR: Yes / Conditional on accepted setup when no existing sentinel is found.  
Evidence: constructor context `005e866e PUSH 0x4`, `005e8670 PUSH 0x2`, `005e8672 PUSH 0x0`, `005e8674 PUSH 0x1`, `005e8677 PUSH 0xabe050`, `005e867c PUSH 0x82bc30`, `005e8683 CALL 0x0069a980`. `FUN_0069a980` stores filename at `+0x58`, official byte at `+0x17C`, min at `+0x180`, and max at `+0x184`.

Rust-facing implication: current Rust has the identity and capacity correct: `RandMap.Sed`, `official=true`, min `2`, max `4`. Older setup-path docs saying those fields are missing are stale.

### 7. Existing sentinel display and digest are updated

Active in YR: Yes / Conditional on accepted setup when a sentinel already exists.  
Evidence: found-record path calls `FUN_0069acd0(&DAT_00ABE050)` for display/name, then `FUN_005e84d0`, then `FUN_0069ad80` for digest/source. `FUN_0069acd0` copies up to `0x2C`; `FUN_0069ad80` copies up to `0x20` and null-terminates at `+0x17B`.

Rust-facing implication: display update exists in Rust; digest/source metadata is not modeled. This is not needed for first non-noop behavior unless another shell surface consumes it, but the model should leave room for it.

### 8. Accepted `0x583` commits through ordinary selection/accept semantics

Active in YR: Yes / Conditional on accepted setup.  
Evidence: post-`FUN_005e8590` branch reaches normal list/selection helpers and calls `FUN_005e7160`. `FUN_005e7160` reads listbox `0x553` with `LB_GETCURSEL` (`0x188`) and `LB_GETITEMDATA` (`0x199`), finds the scenario-record index in `DAT_00A8B8CC`, reads mode listbox `0x6EB`, writes selected globals, and returns `1`.

Rust-facing implication: successful Create Random Map should select/commit the `RandMap.Sed` scenario record through the same state used by Use Map. Do not represent random map as `None`, `auto`, or a negative index.

### 9. Launch branch is suffix `.SED`, not a special `RandMap.Sed` equality

Active in YR: Yes / Conditional on any selected filename ending in `.SED`; standard Create Random Map uses `RandMap.Sed`.  
Evidence: `ScenarioClass__Read_Scenario` copies the selected filename locally, compares the final suffix against `.SED`, stores `ScenarioClass+0x34BD`, and branches away from normal INI loading when random.

Rust-facing implication: launch should have an early `.sed` branch before normal map lookup. `RandMap.Sed` is the native sentinel, but the branch predicate is suffix-based.

### 10. Launch loads seed/options before generation and retains `.SED` identity

Active in YR: Yes / Conditional on `.SED` suffix.  
Evidence: assembly `00684961` reads `ScenarioClass+0x34BD`; `0068496f MOV ECX,0xabdfd8`, `00684975 CALL 0x00597a10`; if true, `00684980 PUSH 0`, `00684982 PUSH 0`, `00684984 MOV ECX,0xabdfd8`, `00684989 CALL 0x00598960`; `00684995..006849bf` copies the original local filename back into `ScenarioClass+0x125C`.

Rust-facing implication: Rust should not require or invent a generated `.map` filename. Generated map state is in memory; selected identity remains `RandMap.Sed`.

### 11. Launch generation is not preview generation

Active in YR: Yes / Conditional.  
Evidence: launch calls `FUN_00598960(0,0)` after `FUN_00597a10`; random-map dialog preview paths call `FUN_00598960(1, hwnd)` and `GenerateTerrainPreview`. `FUN_00598960` repeatedly checks the preview flag before repainting.

Rust-facing implication: `RandMap.img` preview lifecycle and gameplay generation must be separate. A UI preview image must not become authoritative terrain.

### 12. Mode admission comes from MPModes random flag

Active in YR: Yes / Conditional by selected mode.  
Evidence: local `ini/mpmodesmd.ini` has fifth field `true` for `[Battle] 1=...standard,true` and `[FreeForAll] 2=...standard,true`; Team Game, Megawealth, Duel, Meat Grind, Naval War, Unholy Alliance, and Cooperative entries scanned here are `false`. Prior mode reports bind this field to random-map admission.

Rust-facing implication: Rust's `mode.random_maps_allowed` gate is the right admission surface.

## Current Rust Status

| Surface | Status | Evidence |
|---|---|---|
| Button recognition | Present but still log-only | [src/app.rs](src/app.rs) logs "random map generation is not implemented yet" in `CreateRandomMap0x583` |
| Modal helper for sentinel creation | Present but not wired to the app button | [src/ui/skirmish_shell/state.rs](src/ui/skirmish_shell/state.rs) has `ChooseMapModalState::create_random_map`, but `src/app.rs` does not call it |
| Sentinel identity | Present | [src/skirmish_scenarios.rs](src/skirmish_scenarios.rs) defines `RANDMAP_SED = "RandMap.Sed"` |
| Sentinel min/max/official | Present | `random_map_sentinel` sets min `2`, max `4`, `official=true` |
| Upsert one sentinel | Present | `upsert_random_map_sentinel` updates existing `RandomMapSentinel` before append |
| Mode random flag | Present | `record_matches_mode` admits sentinel only when `mode.random_maps_allowed` |
| App-level accepted setup | Missing | no random-map setup state/dialog branch; command logs only |
| Seed/options model and `.SED` writer/reader | Missing | no `[RandomMap]` model found in Rust scan |
| `RandMap.img` preview source | Partial | [src/app_skirmish_shell_render/preview.rs](src/app_skirmish_shell_render/preview.rs) detects the sentinel and reads runtime `RandMap.img`; no accepted setup lifecycle exists to generate/clear/select it |
| `RandMap.img` 3-plane direct RGB decode | Present | [src/assets/pcx_file.rs](src/assets/pcx_file.rs) supports one-plane paletted and three-plane direct RGB PCX data |
| `.SED` launch generation | Missing | [src/app_init.rs](src/app_init.rs) routes selected map names through normal `load_map_by_name_or_path_with_assets` |

## Implementation Handoff

### Required Deltas

1. Replace the log-only `CreateRandomMap0x583` branch with a real command state.
   - Active in YR: Yes / Conditional.
   - Required effect: button opens/enters a random-map setup flow or a visible explicit blocked state; no silent no-op.
   - Acceptance test: clicking Create Random Map no longer only logs; it either reaches setup or displays an intentional unavailable state.
   - Proposed test: `choose_map_create_random_map_is_not_log_only`.

2. Gate all side effects on accepted setup result.
   - Active in YR: Yes / Conditional.
   - Required effect: cancel/failure preserves previous selected mode/map, preview, and launch token.
   - Acceptance test: canceling random setup leaves the previous concrete map committed.
   - Proposed test: `choose_map_create_random_map_cancel_preserves_previous_selection`.

3. Persist a random-map seed/options object as `RandMap.Sed` semantics.
   - Active in YR: Yes / Conditional.
   - Required effect: Rust has a `[RandomMap]` setup model carried to launch; exact terrain formulas can remain blocked behind generator work.
   - Acceptance test: accepted setup creates a launch-visible `RandMap.Sed` selection and stored seed/options, not only a display row.
   - Proposed test: `choose_map_create_random_map_accept_persists_randommap_seed_options`.

4. Use the existing sentinel upsert with native fields.
   - Active in YR: Yes / Conditional.
   - Required effect: one synthetic record, file `RandMap.Sed`, official `true`, min `2`, max `4`; display may update on repeated accepted setup.
   - Acceptance test: accepting Create Random Map twice leaves one sentinel row and updates its display/setup metadata.
   - Proposed test: `choose_map_create_random_map_accept_upserts_single_native_sentinel`.

5. Commit accepted setup through normal Choose Map selected-record state.
   - Active in YR: Yes / Conditional.
   - Required effect: modal closes like Use Map and launch settings carry `selected_map_file = "RandMap.Sed"`.
   - Acceptance test: after accepted setup, Start does not try the previously highlighted concrete map.
   - Proposed test: `choose_map_create_random_map_accept_commits_randmap_sed`.

6. Add a `.sed` launch branch before normal map lookup.
   - Active in YR: Yes / Conditional.
   - Required effect: selected `.SED` loads random seed/options and routes to generator or an explicit generator-not-implemented launch error; it must not report `RandMap.Sed` as an ordinary missing map.
   - Acceptance test: starting after accepted random setup reaches the random branch.
   - Proposed test: `skirmish_launch_sed_branch_preempts_normal_map_lookup`.

7. Keep preview image separate from gameplay generation.
   - Active in YR: Yes / Conditional.
   - Required effect: accepted setup may use/generated-cache `RandMap.img` for UI preview; launch generation must not consume it as terrain.
   - Acceptance test: gameplay launch can proceed from seed/options even if preview texture is absent, or fails with a generator blocker rather than a preview decode error.
   - Proposed test: `randmap_preview_is_not_gameplay_data`.

### Non-Deltas

- Do not change `sim/` layering for this UI command; random map setup and launch generation are app/map-load concerns.
- Do not implement terrain/noise formulas as part of the `0x583` UI command contract.
- Do not change the already-correct sentinel filename/min/max/official fields unless adding digest/source metadata.
- Do not add `RandMap.Sed` to permanent loose-map scanning.

### Blockers

- No Rust random-map seed/options model or `.SED` writer/reader exists yet.
- No Rust gameplay random map generator entry exists for `FUN_00598960(0,0)` parity.
- Exact runtime UX for malformed external `.SED` and corrupt/missing `RandMap.img` remains unverified.

### Acceptance Tests

- `choose_map_create_random_map_cancel_preserves_previous_selection`: cancel/failure path leaves previous selected map and preview untouched.
- `choose_map_create_random_map_accept_upserts_single_native_sentinel`: accepted path creates/updates one `RandMap.Sed` record, official `true`, min `2`, max `4`.
- `choose_map_create_random_map_accept_commits_randmap_sed`: accepted path closes chooser and launch settings use `selected_map_file == "RandMap.Sed"`.
- `choose_map_create_random_map_is_not_log_only`: command produces setup/blocked UI state, not just an info log.
- `skirmish_launch_sed_branch_preempts_normal_map_lookup`: `RandMap.Sed` does not route through ordinary concrete map loading.
- `randmap_preview_is_not_gameplay_data`: missing preview image does not erase stored seed/options or masquerade as generated terrain.

## Negative Facts / Do Not Do

- Do not commit on mere button click. Active in YR: No; side effects occur only after `FUN_00595bc0` returns `1`.
- Do not create/update `RandMap.Sed` on random-map dialog cancel. Active in YR: No; `FUN_005e8590` returns `-1`.
- Do not append duplicate random sentinel rows. Active in YR: No; native scans and updates by filename first.
- Do not set native-created sentinel `Official=false`. Active in YR: No; constructor receives official `1`.
- Do not set random sentinel min/max from map INI. Active in YR: No; constructor receives hardcoded `2` and `4`.
- Do not parse `RandMap.Sed` as ordinary map INI or `[PreviewPack]`. Active in YR: No; launch uses `.SED` seed reader/generator and preview uses `RandMap.img`.
- Do not replace the selected filename with `RandMap.Map` or any generated loose map name. Active in YR: No; launch copies the original `.SED` token back.
- Do not use `RandMap.img` as gameplay terrain. Active in YR: No; it is generated preview image data.

## Open Questions - Final State

- `[RESOLVED] OQ-01 - Is 0x583 live in standard YR? -> Yes, conditionally on the Choose Map button click.` Evidence: `005e69d3..005e6a11`.
- `[RESOLVED] OQ-02 - What gates side effects? -> random-map dialog result must equal 1.` Evidence: `005e85c1..005e85ce`.
- `[RESOLVED] OQ-03 - What file stores setup? -> RandMap.Sed.` Evidence: `005e85d1`, `FUN_00597730`.
- `[RESOLVED] OQ-04 - What preview file is used? -> RandMap.img.` Evidence: `005e861a..005e8626`, `FUN_00595bc0`.
- `[RESOLVED] OQ-05 - What are new sentinel min/max/official values? -> min 2, max 4, official 1.` Evidence: `005e866e..005e8683`, `FUN_0069a980`.
- `[RESOLVED] OQ-06 - What selected token reaches launch? -> record filename RandMap.Sed.` Evidence: ordinary selected-record accept and `.SED` launch copy-back.
- `[RESOLVED] OQ-07 - What launch branch fires? -> suffix .SED, then seed load and `FUN_00598960(0,0)` on success.` Evidence: `ScenarioClass__Read_Scenario`, `00684961..00684995`.
- `[RESOLVED] OQ-08 - Is current Rust sentinel field state still stale? -> No for file/min/max/official; yes for seed/options, digest/source, preview, and launch branch.` Evidence: Rust scan.
- `[DEFERRED] OQ-09 - Exact terrain/noise formulas.` Category: out-of-scope.
- `[DEFERRED] OQ-10 - Malformed external .SED UX.` Category: needs-runtime-debugger.
- `[DEFERRED] OQ-11 - Corrupt/missing RandMap.img screenshot behavior.` Category: needs-runtime-debugger.

## Stale Docs / Follow-up Docs

- Path: `docs/research/traces/SKIRMISH_CHOOSE_MAP_BUTTON_ACTION_0X102_TO_0X6B_TRACE.md`
  - Replace broad "Create Random Map is log-only/no-op" Rust status with: "STALE as of 2026-05-23 current Rust. The app branch is still log-only, but the lower-level shell model now has `RandomMapSentinel`, mode-gated filtering, native min/max/official sentinel metadata, `RandMap.img` preview detection, and 1/3-plane PCX decode support. Remaining implementation gap is the accepted setup flow that writes seed/options, invokes the upsert/commit path, clears/reloads preview at the right time, and routes `.SED` launch before normal map loading."
- Path: `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`
  - Replace current-Rust statements that the sentinel has no min/max players and that Rust's PCX decoder only supports one-plane paletted PCX with: "STALE as of 2026-05-23 current Rust. `SkirmishScenarioRecord::random_map_sentinel` now sets `RandMap.Sed`, `official=true`, min `2`, max `4`, and `PcxFile` supports both one-plane paletted and three-plane direct RGB PCX data. Rust still lacks `[RandomMap]` seed/options parsing, launch-time generation, generated start waypoints, and the app-level accepted Create Random Map flow."

## Sources

- Fresh read-only Ghidra spot-checks: `batch_string_anchor_report("RandMap")`; decompile of `FUN_005e68a0`, `FUN_005e8590`, `FUN_00595bc0`, `FUN_00596300`, `FUN_00594b50`, `FUN_005a1fb0`, `FUN_007b05c0`; assembly context for `005e69d3`, `005e6a11`, `005e6a18`, `0068465c`, `00684694`, `006846be`, `00684961`, `00684975`, `00684989`, `00684990`; string read at `0x0083DA88 == ".SED"`. Prior decompile-backed reports cover unchanged broader bodies whose raw function boundaries were unavailable in this read-only pass.
- Prior reports reconciled: `SKIRMISH_CREATE_RANDOM_MAP_0X583_BROAD_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md`, `SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`, `SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_IMG_PREVIEW_LOADER_00641DB0_GHIDRA_REPORT.md`, `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`.
- Current Rust scan: `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_scenarios.rs`, `src/app_init.rs`, `src/app_list_maps.rs`, `src/app_transitions.rs`, `src/app_skirmish_shell_render.rs`.
- INI checked: `ini/mpmodesmd.ini`.
