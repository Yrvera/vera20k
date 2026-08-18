# GSI-04.03B Merged-Rules Overlay-Registry Authority Implementation Plan

Status: **APPROVED — REVIEW-PLAN READY**

> **For Codex:** Execute this plan task-by-task in a dedicated feature
> worktree. Review the owned diff before the guarded no-commit merge.

**Goal:** Make one successfully composed rules source construct the production
`RuleSet`, `OverlayTypeRegistry`, and overlay-atlas inputs.

**Architecture:** The app-init helper remains the sole owner of base/YR/mode/map
composition. It returns a transient private pair containing the parsed
`RuleSet` and its exact input `IniFile`; the match loader consumes that pair and
removes its independent raw-rules reload. This is source-routing parity only,
not certification of stateful native multi-pass `ReadINI` equivalence.

**Design Doc:**
`docs/plans/2026-07-24-gsi-04-03b-merged-rules-overlay-registry-authority-design.md`

---

## Grounding Summary

- Native `ScenarioClass::Full_Init @ 0x00686B20` invokes the outer
  reset/main-rules path `0x006686C0`, then sends the active map to the same
  inner rules/type reader `0x00668BF0` before play.
- Existing type records therefore receive map-side values through the same
  runtime authority.
- Current Rust composes base `rules.ini`, optional `rulesmd.ini`, selected mode,
  and bounded map value overrides in `load_rules_ini`, then discards the final
  `IniFile`.
- `load_map_from_initial` later reloads raw `rulesmd.ini` or `rules.ini` for
  `OverlayTypeRegistry` and the atlas builder, splitting source authority.
- `RuleSet::from_ini` already stamps `IniFile::content_hash()` as
  `source_ini_hash`, so no protected rules-file change is needed.
- `OverlayTypeRegistry::from_ini` reads `[OverlayTypes]` and each named type's
  `Tiberium`, `Land`, wall, strength, radar, and presentation flags from its
  supplied rules source.
- Stock `MountMoras.map` resolves from `expandmd01.mix`, is 103,241 bytes, has
  no overlay registry/type sections, and changes `GAYARD.TechLevel` from 4 to
  11. It is a stock map-pass/no-op fixture, not an overlay-delta oracle.
- A retail-backed synthetic `[GASAND] Tiberium=yes` map override is the
  non-vacuous routing oracle: production raw ID 0 is `GASAND` and false.
- Full native selected-mode rules application/order remains `UNCHECKED`; the
  implementation preserves current Rust ordering.
- Native stateful per-type reread effects and map-side new-type allocation
  (`XEB2/EB2` map-only `SpazWH`) remain explicit separate residuals.

## Key Technical Decisions

- Return `LoadedRules { rules, merged_ini }` through private construction and
  `into_parts()` — **Confidence: high**
  - **Source:** approved design; current `app_init_helpers.rs`; caller audit.
- Factor the current order into one private composer returning the merged INI
  and applied map-key count — **Confidence: high**
  - **Source:** current loader and `IniFile::merge_rules_overrides`.
- Preserve the startup `load_rules_ini` wrapper — **Confidence: high**
  - **Source:** exhaustive caller search finds `app.rs` startup plus the match
    caller only.
- Treat the retail flag flip as routing evidence only — **Confidence: high**
  - **Source:** `OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` documents stateful
    `Tiberium=yes` side effects not represented by one final parse.
- Keep `app_skirmish.rs`, `src/map/overlay_types.rs`, and all `src/rules/*`
  read-only — **Confidence: high**
  - **Source:** interface/caller audit and protected damage-authority ownership.

## Open Questions

### Resolved During Planning

- Does a stock selected map supply an overlay-type delta? No demonstrated stock
  fixture does. Raw `MountMoras.map` has no overlay sections.
- Is `SpazWH` an overlay classification input? No. It is a map-only warhead and
  belongs to the separate map-type-allocation residual.
- Is current full mode merge native-proven? No. Preserve it as current Rust
  behavior and keep native equivalence `UNCHECKED`.

### Deferred to Separate Contracts

- Stateful per-type multi-pass `ReadINI` persistence and forced Land/Armor
  writes.
- Map-side allocation of new TypeClass and color records.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/app_init_helpers.rs` | Own pair type, composition, archive loader, compatibility wrapper, and focused tests. |
| Modify | `src/app_init.rs` | Consume the pair once and route the retained INI to both production registry builders. |
| Modify (ignored canonical doc, outside feature commit) | `docs/research/ENGINE_STATE_OVERVIEW.md` | Replace the stale “no map rules merge” statement while preserving allocation/reread residuals. |

No new module is warranted. `app_init_helpers.rs` already exceeds the style
guideline because it is a cohesive extracted app-init helper plus test suite;
this bounded change does not create a new responsibility or justify an
unrelated split.

## Interface Changes

- Add crate-private `LoadedRules`.
- Add crate-private `load_rules_with_merged_ini`.
- Keep the existing crate-private `load_rules_ini` signature unchanged.
- No public crate API, simulation state, snapshot, hash format, or dependency
  changes.

## Risk Areas

- A caller could discard the merged source: private construction,
  `into_parts()`, the production callsite cutover, and hash tests pin the
  intended boundary.
- Refactoring could alter optional-patch or log behavior: preserve all current
  failure branches and applied-map-key logging.
- A test could compare against a merged baseline and pass vacuously: the retail
  delta test must intentionally reproduce the old production raw selection.
- A final composed INI is not a full native reread model: tests and comments
  must not claim Land/Armor or arbitrary-map parity.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Base/YR/mode/map order | A changed layer order changes rules and downstream gameplay. | Four-layer synthetic regression; existing order tests. |
| 1 | Pair source identity | Divergent sources recreate the production defect. | Exact `source_ini_hash == content_hash` assertions. |
| 2 | Existing registry identity | Map value routing must not replace bounded registry lists. | Exact ID 0/name `GASAND`; stable `TIB01` ID. |
| 3 | Production raw reload removal | Any remaining alternate source leaves global integration drift. | Source search plus app-init focused tests. |
| 3 | Atlas source routing | Atlas-side registry parsing must see the same source as terrain/sim setup. | Existing argument path retained from `rules_ini`. |

---

## Tasks

### Task 0: Create and register the owned feature worktree

**Why:** Isolate the implementation from `dev` and the protected dirty
damage-authority worktree before any Rust edit.

**Files:** No tracked file is modified by this task.

**Step 1: Run a fresh preflight**

- Require root `dev` to be Git-clean and still at the exact recorded base SHA.
- Record every branch, worktree, stash, active agent, Cargo/rustc process, and
  dirty path in the operational journal.
- Re-read the protected damage-authority diff and confirm neither owned
  `app_init*.rs` path overlaps it.
- Record writer `/root`, approved contract/design/plan paths and hashes,
  plan revision, owned paths, exact base SHA, protected ownership, and exact
  next action.

**Step 2: Create the unique branch and linked worktree**

```text
$gsiBaseSha = git rev-parse dev
$gsiRunSuffix = Get-Date -Format 'yyyyMMdd-HHmmss'
$gsiBranchName = "feature/gsi-04-03b-merged-rules-authority-$gsiRunSuffix"
$gsiWorktreePath = "<local>/Documents/ra2-rust-game-gsi-04-03b-$gsiRunSuffix"
git branch $gsiBranchName $gsiBaseSha
git worktree add $gsiWorktreePath $gsiBranchName
```

Require the linked worktree HEAD to equal the recorded base SHA and its status
to be clean.

**Step 3: Provision ignored retail INIs**

- Copy the main checkout's verified `ini/` directory into the new worktree.
- Resolve both absolute paths first and require the destination to be inside
  the new worktree.
- Assert the destination is a physical copy, not a reparse point.
- Assert `git status --short --untracked-files=all` remains empty because the
  inputs are ignored; never stage them.
- Record the provisioning method and exact paths in the journal.

### Task 1: Introduce the paired rules construction boundary

**Why:** Establish one construction result before changing callers.

**Files:**

- Modify `src/app_init_helpers.rs` around the current `load_rules_ini`.

**Pattern:** Existing app-layer owned loader; no new subsystem.

**Step 1: Add the private pair and constructor**

```rust
pub(crate) struct LoadedRules {
    rules: RuleSet,
    merged_ini: IniFile,
}

impl LoadedRules {
    fn from_merged_ini(merged_ini: IniFile) -> Result<Self, crate::rules::error::RulesError> {
        let rules = RuleSet::from_ini(&merged_ini)?;
        debug_assert_eq!(rules.source_ini_hash(), merged_ini.content_hash());
        Ok(Self { rules, merged_ini })
    }

    pub(crate) fn into_parts(self) -> (RuleSet, IniFile) {
        (self.rules, self.merged_ini)
    }
}
```

**Step 2: Factor composition without archive access**

```rust
fn compose_rules_layers(
    mut ini: IniFile,
    rulesmd: Option<&IniFile>,
    mode: Option<&IniFile>,
    map: Option<&IniFile>,
) -> (IniFile, usize) {
    if let Some(rulesmd) = rulesmd {
        ini.merge(rulesmd);
    }
    if let Some(mode) = mode {
        ini.merge(mode);
    }
    let applied = map
        .map(|map| ini.merge_rules_overrides(map))
        .unwrap_or(0);
    (ini, applied)
}
```

**Step 3: Add the paired archive loader**

- Preserve required base parsing and `None` on failure.
- Parse optional `rulesmd.ini`; retain the current skip-on-parse-failure
  behavior.
- Preserve patch/mode/map logging.
- Call `compose_rules_layers`, then `LoadedRules::from_merged_ini`.
- Log the loaded object count only after successful pair construction.
- On `RuleSet` error, keep the current warning and return `None`.

**Step 4: Preserve the compatibility wrapper**

```rust
pub(crate) fn load_rules_ini(
    asset_manager: &AssetManager,
    mode_rules_override: Option<&IniFile>,
    map_rules_overrides: Option<&IniFile>,
) -> Option<RuleSet> {
    load_rules_with_merged_ini(
        asset_manager,
        mode_rules_override,
        map_rules_overrides,
    )
    .map(|loaded| loaded.into_parts().0)
}
```

### Task 2: Add non-vacuous source-routing tests

**Why:** Prove the pair boundary, bounded precedence, retail-backed delta, and
stock no-op path independently.

**Files:**

- Modify the existing test module in `src/app_init_helpers.rs`.

**Step 1: Add the four-layer synthetic test**

- Base registers `TIB01` and `Riparius`.
- YR and mode set `Riparius.Image` to 2 then 3.
- Map attempts to replace `[OverlayTypes]` and `[Tiberiums]` but also sets
  existing `Riparius.Image=4` and `TIB01.Tiberium=yes`.
- Pass the composed INI through `LoadedRules::from_merged_ini`.
- Assert applied count 2, `Riparius.image == 4`, ID 0 remains `TIB01`, its flag
  is true, and pair/source hashes match.

**Step 2: Add ignored retail delta test**

- Use `RA2_DIR`, falling back to the documented local retail path, and require
  the resolved directory plus `AssetManager::new` with `expect`; the explicitly
  invoked test must never return early or report green without retail assets.
- Build the old production raw baseline from `rulesmd.ini` when present,
  otherwise `rules.ini`.
- Assert raw ID 0/name `GASAND` and `Tiberium=false`.
- Call `load_rules_with_merged_ini` with `[GASAND]\nTiberium=yes`.
- Assert merged ID 0/name `GASAND`, `Tiberium=true`, and exact pair hash.

**Step 3: Add ignored `MountMoras.map` no-op test**

- Resolve with `get_with_source("MountMoras.map")`; assert source exactly
  `expandmd01.mix` and length 103,241.
- Assert `[GAYARD]` exists and overlay registry sections do not.
- Compare no-map and map paired loads.
- Assert `GAYARD.TechLevel` 4 then 11; hashes each match their own sources and
  differ.
- Build a test-only snapshot containing every registry ID/name and every
  public `OverlayTypeFlags` field; assert the complete no-map and map snapshots
  are equal. This prevents a representative-only assertion from missing a
  changed overlay.

**Step 4: Verify focused tests**

Run serially after checking Cargo ownership:

```text
cargo test -p vera20k app_init_helpers::tests::map_overlay_flag_wins_after_rulesmd_and_mode -- --nocapture
$env:RA2_DIR='<ra2-install>'; cargo test -p vera20k app_init_helpers::tests::retail_rules_plus_map_override_reaches_production_overlay_registry -- --ignored --nocapture
$env:RA2_DIR='<ra2-install>'; cargo test -p vera20k app_init_helpers::tests::retail_mount_moras_applies_rules_and_preserves_overlay_registry -- --ignored --nocapture
```

Record each literal `test result:` line.

### Task 3: Cut over the production match loader

**Why:** Remove the only alternate registry source in the match loop.

**Files:**

- Modify `src/app_init.rs` imports and rules-load block.

**Step 1: Import `load_rules_with_merged_ini`**

- Replace the match loader's `load_rules_ini` import; startup `app.rs` remains
  on the compatibility wrapper.

**Step 2: Consume the pair once**

```rust
let (loaded_rules, rules_ini) = load_rules_with_merged_ini(
    &asset_manager,
    mode_override_ini.as_ref(),
    Some(&map_data.ini),
)
.ok_or_else(|| anyhow::anyhow!("failed to load or validate merged game rules"))?
.into_parts();
let mut rules = Some(loaded_rules);
```

**Step 3: Remove the raw reload**

- Delete the later `rulesmd.ini`/`rules.ini` `get_with_source` block.
- Keep `OverlayTypeRegistry::from_ini(&rules_ini, art_ini.as_ref())`.
- Keep the existing `&rules_ini` argument to
  `build_overlay_atlas_from_map`.

**Step 4: Impact search**

```text
rg -n "load_rules_ini\(|load_rules_with_merged_ini\(|Raw rules INI from|OverlayTypeRegistry::from_ini\(" src
```

Expected: startup alone uses `load_rules_ini`; match init uses the paired
loader; no `Raw rules INI from` block remains.

### Task 4: Validate and commit the owned feature

**Why:** Verify the feature worktree before touching `dev`.

**Files:** Only the two owned Rust files are staged in the feature worktree.

**Step 1: Format only owned Rust files**

```text
rustfmt --edition 2024 src/app_init_helpers.rs src/app_init.rs
```

Inspect the diff for unrelated churn.

**Step 2: Run focused regression tests serially**

```text
cargo test -p vera20k app_init_helpers::tests -- --nocapture
cargo test -p vera20k map::overlay_types::tests -- --nocapture
cargo check -q
```

Also run the two ignored retail tests explicitly after setting `RA2_DIR` to the
verified install path; absence or loader failure is a hard test failure.

**Step 3: Review and commit**

- Confirm `git diff --check`.
- Confirm only `src/app_init_helpers.rs` and `src/app_init.rs` are staged.
- Commit one coherent GSI-04.03B milestone.

### Task 5: Correct canonical docs and guarded-integrate

**Why:** Preserve the corrected mechanism record and validate the combined
production state before creating a merge commit.

**Files:**

- Modify ignored canonical doc `docs/research/ENGINE_STATE_OVERVIEW.md` in the
  primary checkout; do not stage it in the feature commit.

**Step 1: Correct and reindex the canonical doc**

- Replace only the stale statement that Rust never applies map rules values.
- State that current Rust applies bounded existing-section values but still
  lacks map-side new-type allocation and generic stateful multi-pass reread
  equivalence.
- Rebuild the research index and require focused validation with no changed or
  missing documents.
- Append the doc hash and research-index result to the operational journal.

**Step 2: Re-run full integration preflight**

- Require root `dev` clean. Capture its current SHA and compare it with the
  recorded feature base.
- Require no staged, unstaged, or untracked paths and no merge/rebase/cherry-pick
  state in root `dev`.
- Recheck stashes, worktrees, protected dirty paths, Cargo ownership, and
  agent ownership.
- If `dev` advanced, reassess every touched interface, plan assumption,
  neighbor dependency, conflict, and validation requirement. If any assumption
  is stale or a semantic conflict exists, preserve the old branch, create a
  fresh branch/worktree from current validated `dev`, reapply only individually
  reviewed compatible changes, revise/reapprove, and rerun branch validation.
- Use branch containment/reachability checks to prove the feature commit is
  reachable from the owned feature branch and not yet from `dev`; list its
  exact two tracked paths.
- On the clean `dev` baseline, run the same focused tests, both explicitly
  enabled retail tests with `RA2_DIR`, impact search, and literal
  `cargo check -q`. Record every literal result before merging so a
  pre-existing failure cannot be attributed to the feature.
- Record the feature commit SHA, clean-dev baseline SHA, and validation results
  in the journal.

**Step 3: Guarded no-commit integration**

- `git merge --no-ff --no-commit $gsiBranchName`.
- Re-run the focused tests, both ignored retail tests, impact search, and
  literal `cargo check -q` on the combined state.
- Commit the merge only after every required check passes; otherwise abort the
  no-commit merge without altering unrelated work.

**Step 4: Record and clean up**

- Record the merge commit, exact changed paths, literal test-result lines, and
  combined validation outcome in the journal.
- Prove the feature commit is now reachable from `dev`.
- Resolve the recorded copied `ini/` path, require it to remain inside the
  recorded feature worktree, and require it not to be a reparse point. Remove
  that exact physical copy without following any external target, then verify
  the path is absent.
- Require the feature worktree to be clean, merged into `dev`, and unowned by
  every live agent. Remove it with non-force `git worktree remove`; retain the
  feature branch as provenance unless the operator explicitly permits
  deletion.
- Never push.

## Sources & References

- `docs/plans/2026-07-24-gsi-04-03b-merged-rules-overlay-registry-authority-design.md`
- `docs/contracts/2026-07-24-gsi-04-03b-merged-rules-overlay-registry-authority-implementation-contract.md`
- `docs/research/RULESCLASS_GHIDRA_REPORT.md`
- `docs/research/core-services-map/rules-class.md`
- `docs/research/OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MPMODES_MIX_OVERRIDE_PAYLOADS_GHIDRA_REPORT.md`
- `gamemd.exe`: `0x006686C0`, `0x00668BF0`, `0x00686B20`
- INI inputs: `rules.ini`, `rulesmd.ini`, `[OverlayTypes]`,
  `[Tiberiums]`, `[GASAND] Tiberium`, `[GAYARD] TechLevel`
- Related code: `src/app_init_helpers.rs`, `src/app_init.rs`,
  `src/app_skirmish.rs`, `src/map/overlay_types.rs`,
  `src/rules/ini_parser.rs`, `src/rules/ruleset.rs`

## Post-Plan Self-Review

- Design and operator requirements map to Tasks 0–5.
- No placeholder/TODO steps remain.
- No simulation layer is touched.
- The crate-private interface lands before callsite integration.
- Retail tests distinguish raw, merged, and no-op paths.
- Native multi-pass and map-allocation residuals are not hidden.
- Every cited path and current caller was rechecked against `dev`.
- All decisions are high-confidence and source-backed.
- No execution-time question is deferred inside this feature.
