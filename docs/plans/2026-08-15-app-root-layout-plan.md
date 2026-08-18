# App Root Layout Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Establish one literal `src/app/` module root by moving the app facade to `src/app/mod.rs` and placing the private startup-splash implementation in the new `app::frontend` domain without changing runtime behavior.

**Architecture:** `vera20k::app` remains the public app facade and `src/lib.rs` continues to declare only `pub mod app;` for that facade. This foundation changes filesystem/module resolution only: existing root orchestrators remain direct app children, the splash moves to a shallow frontend domain, and deterministic simulation remains independent of app.

**Design Doc:** `docs/plans/2026-08-15-single-app-tree-design.md`

---

## Grounding Summary

- The research index has no module-layout research corpus for this topic; the change introduces no native behavior and needs no gamemd.exe verification.
- Current `src/app.rs` declares the existing `src/app/{frame,handler,initialize,in_game,shell_*,state}.rs` children.
- The only path-relative module owned by `src/app.rs` but stored outside `src/app/` is `#[path = "app_startup_splash.rs"] mod app_startup_splash;`.
- Moving `src/app.rs` to `src/app/mod.rs` leaves ordinary `mod frame;`, `mod handler;`, and sibling declarations resolving to the same existing files.
- `src/app_startup_splash.rs` contains five embedded tests and has consumers only in `src/app/{initialize,state,frame}.rs`.
- `src/asset_tools/palette_production.rs` carries three source-provenance references to the old splash path; those references must move with the implementation, and their current line anchors are already stale.
- Four present-tense comments still identify `app.rs` as the live facade path (`Cargo.toml`, `src/main.rs`, `src/app_dev_overlay.rs`, and `src/app_skirmish_shell_render.rs`); historical “Extracted from app.rs” comments are not live-path claims and do not require churn in this slice.
- `src/lib.rs` already exposes `pub mod app;`; Rust resolves that declaration from `src/app/mod.rs` when `src/app.rs` is absent, so no library-root edit is required.
- A current import scan finds no `crate::app` dependency below `src/sim/`; the plan preserves that boundary.
- Shared `skirmish_launch` contracts remain outside app because sim imports them.
- No INI keys, assets, snapshot fields, replay formats, hashes, RNG streams, or tick/render ordering are changed.
- The closest repository pattern is the existing directory-backed modules `src/app_render/mod.rs`, `src/app_instances/mod.rs`, and `src/app_tactical_capture/mod.rs`.

## Key Technical Decisions

- **Use `src/app/mod.rs` as the sole app module root:** This produces the requested literal one-directory layout while keeping the public Rust path `vera20k::app` unchanged. — **Confidence: high**
  - **Source:** Rust module layout; current `src/lib.rs`; design decision in `docs/plans/2026-08-15-single-app-tree-design.md`.
- **Move the splash directly to `app::frontend::startup_splash`:** A temporary `#[path = "../app_startup_splash.rs"]` would preserve root clutter and require a second move. — **Confidence: high**
  - **Source:** current `src/app.rs:68-69`; current consumers in `src/app/{initialize,state,frame}.rs`.
- **Keep `frontend` public but scope the splash to the app subtree:** Later formerly-public frontend modules can live under `app::frontend`; declaring the splash `pub(super)` preserves the current private module’s effective visibility to `crate::app` and its descendants without exposing it to unrelated crate-root modules. — **Confidence: high**
  - **Source:** current `pub mod app;` facade, private `mod app_startup_splash;` declaration, and Rust restricted-visibility rules.
- **Do not alter function bodies or item visibility:** Module relocation and API-policy cleanup must remain independently reviewable. — **Confidence: high**
  - **Source:** approved design and AGENTS.md incremental-change policy.
- **Do not rustfmt `src/app/mod.rs` or `src/app/frontend/mod.rs`:** Project policy forbids rustfmt on module roots; format their small declaration blocks manually. — **Confidence: high**
  - **Source:** project workflow instructions.

## Open Questions

### Resolved During Planning

- **Should `src/app.rs` remain beside `src/app/`?** No. The explicit target is one literal app directory, so the facade becomes `src/app/mod.rs`.
- **Should the splash stay temporarily at the source root?** No. Its current path attribute would become relative to `src/app/`; moving it directly to `frontend/startup_splash.rs` avoids an interim parent-path dependency.
- **Should shared `skirmish_*` contracts move under app?** No. Simulation imports `skirmish_launch`, so those contracts are neutral lower-layer inputs rather than app-owned modules.
- **Should this branch also move a shell renderer?** No. This branch establishes only the module-root foundation; the first complete frontend subtree receives its own branch and plan.

### Deferred to Implementation

None. Compilation and tests are verification gates, not unresolved design choices.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Move | `src/app.rs` → `src/app/mod.rs` | Public app facade and broad domain declarations |
| Create | `src/app/frontend/mod.rs` | Frontend domain root |
| Move | `src/app_startup_splash.rs` → `src/app/frontend/startup_splash.rs` | Process-start splash composition, presentation, and its five tests |
| Modify | `src/app/initialize.rs` | Import and construct the splash from its new module path |
| Modify | `src/app/state.rs` | Store the splash presentation using its new module path |
| Modify | `src/app/frame.rs` | Render the splash through its new module path |
| Modify | `src/asset_tools/palette_production.rs` | Retarget startup-splash source provenance to the new path and current line anchors |
| Modify | `Cargo.toml` | Name the app facade rather than the removed `app.rs` path in the dependency comment |
| Modify | `src/main.rs` | Name the app facade rather than the removed `app.rs` path in the module header |
| Modify | `src/app_dev_overlay.rs` | Name the app facade rather than the removed `app.rs` path in the module header |
| Modify | `src/app_skirmish_shell_render.rs` | Name the app facade rather than the removed `app.rs` path in the module header |
| Local design artifact | `docs/plans/2026-08-15-single-app-tree-design.md` | Approved initiative design; intentionally ignored by Git |
| Local plan artifact | `docs/plans/2026-08-15-app-root-layout-plan.md` | Executable foundation plan; intentionally ignored by Git |

## Interface Changes

- `vera20k::app` and `vera20k::app::App` remain unchanged.
- `src/lib.rs` remains unchanged and continues to declare `pub mod app;`.
- The private path `crate::app::app_startup_splash` is removed.
- The replacement path is `crate::app::frontend::startup_splash`, with the splash child declared `pub(super)` so its effective visibility remains inside the app subtree.
- No public structs, functions, traits, command formats, snapshot schemas, or runtime contracts change.

## Risk Areas

- Rust must not see both `src/app.rs` and `src/app/mod.rs`; the move must remove the old file atomically.
- The old `#[path = "app_startup_splash.rs"]` declaration must be removed rather than adjusted to a parent path.
- All three splash consumer modules must import `frontend::startup_splash`; retaining one `app_startup_splash` reference will fail compilation.
- Source-provenance strings and present-tense facade comments must not retain removed paths after the physical move.
- The five embedded splash tests must remain byte-for-byte with the moved implementation.
- No recursive formatting command may touch `src/app/mod.rs` or `src/app/frontend/mod.rs`.

## Player-Experience Critical Items

Representative scenario: launch the graphical application into the ordinary shell, display/dismiss the process-start splash, and continue into a stock offline skirmish.

| Task # | Class | Item | Why it matters | Verification |
|--------|-------|------|----------------|--------------|
| Task 1 | COMPOUNDING | Exactly one app module root | Duplicate roots can split type identity or tests | Filesystem checks and library compilation |
| Task 1 | MILESTONE-BLOCKING | Splash build/render/dismiss code remains unchanged | This is the first visible application frame | Five moved unit tests plus unchanged-body diff review |
| Task 1 | COMPOUNDING | All splash consumers and source references use one new path | A stale consumer breaks the build; stale provenance becomes uncheckable | `rg` old-path scan and staged diff review |
| Task 1 | MILESTONE-BLOCKING | No sim, RNG, timing, render-order, persistence, or state changes | A structural move must not change an ordinary match | Scope diff, no sim diff, full `--lib` suite |
| Task 1 | EXACTIFICATION-RESIDUAL | Existing render/app type coupling remains | It is compile-time debt, not active player drift | Record only; no interface redesign in this branch |

---

## Tasks

### Task 1: Atomically establish the directory-backed app facade

**Why:** The facade move, splash relocation, consumer rewiring, and old-path cleanup form one ownership cone. Applying them in one patch keeps the branch buildable at the only task boundary.

**Files:**
- Move: `src/app.rs` → `src/app/mod.rs`
- Create: `src/app/frontend/mod.rs`
- Move: `src/app_startup_splash.rs` → `src/app/frontend/startup_splash.rs`
- Modify: `src/app/initialize.rs:3-12,129-150`
- Modify: `src/app/state.rs:7-18,459`
- Modify: `src/app/frame.rs:6-9,54-76`
- Modify: `src/asset_tools/palette_production.rs:316-324,795-799`
- Modify: `Cargo.toml:118`
- Modify: `src/main.rs:4`
- Modify: `src/app_dev_overlay.rs:4`
- Modify: `src/app_skirmish_shell_render.rs:4`

**Pattern:** Follow the repository's directory-backed module pattern. Preserve every function body, test assertion, runtime call order, and effective visibility; only module resolution and source-path prose change.

**Step 1: Reconcile the checkout before mutation**

Run:

```powershell
git branch --show-current
git status --short
git worktree list
git rev-parse HEAD
git rev-parse main
git rev-parse origin/main
git log --oneline -5
Get-Process cargo,rustc -ErrorAction SilentlyContinue
```

Expected:

- the branch is `feature/app-root-layout`;
- the worktree is clean and no merge is active;
- `HEAD`, `main`, and `origin/main` resolve to the same commit;
- this checkout is the sole owner of `feature/app-root-layout`;
- no Cargo or rustc process is active.

If any expectation differs, stop and reconcile with the `sync` skill before editing. Do not mutate another worktree or interrupt another task's Cargo process.

**Step 2: Apply the complete ownership-cone move in one patch**

Use one `apply_patch` transaction containing all changes below. Do not leave the branch between partial move states.

1. Move `src/app.rs` to `src/app/mod.rs` and replace:

   ```rust
   #[path = "app_startup_splash.rs"]
   mod app_startup_splash;
   ```

   with:

   ```rust
   pub mod frontend;
   ```

   Keep the existing `frame`, `handler`, `initialize`, `in_game`, `shell_*`, and `state` declarations in their current order.

2. Create `src/app/frontend/mod.rs` with exactly:

   ```rust
   //! Front-end shell, loading, and capture orchestration.

   pub(super) mod startup_splash;
   ```

   The restricted visibility preserves the current private module's effective scope: `initialize`, `state`, and `frame` can use it, while unrelated crate-root modules cannot.

3. Move `src/app_startup_splash.rs` to `src/app/frontend/startup_splash.rs` without changing any implementation line, constant, comment, provenance statement, or test assertion.

4. In `src/app/initialize.rs`, `src/app/state.rs`, and `src/app/frame.rs`, replace the imported `app_startup_splash` module with `frontend::startup_splash`, then replace every qualified `app_startup_splash::` use with `startup_splash::`. Do not change arguments, field order, state layout, presentation timing, or control flow.

5. In `src/asset_tools/palette_production.rs`, update the two production-binding sites to:

   ```rust
   "app/frontend/startup_splash.rs:219";
   ```

   Update the ledger header to `src/app/frontend/startup_splash.rs`, its constant anchor to `:32-34`, and its palette-decoder anchor to `:219`. Do not change the binding keys, palette, alpha policy, or lookup behavior.

6. Replace only the four present-tense facade-path comments:

   - `Cargo.toml:118`: `# anyhow for application-level error propagation (main.rs, app facade)`
   - `src/main.rs:4`: `//! This file should stay minimal (~50 lines). All application logic lives in the app facade and its modules.`
   - `src/app_dev_overlay.rs:4`: `//! The app facade snapshots state into DevOverlayInfo, draws, and`
   - `src/app_skirmish_shell_render.rs:4`: ``//! `GameScreen::MainMenu` branch in the app facade small.``

   Leave historical “Extracted from app.rs” comments unchanged; they describe provenance rather than a live filesystem path.

Do not run rustfmt on either `mod.rs`. No other file or body is in scope.

**Step 3: Verify the filesystem and removed paths**

Run:

```powershell
Test-Path src/app.rs
Test-Path src/app/mod.rs
Test-Path src/app_startup_splash.rs
Test-Path src/app/frontend/mod.rs
Test-Path src/app/frontend/startup_splash.rs
rg --crlf -n '^pub mod app;$' src/lib.rs
rg -n 'app_startup_splash|src/app_startup_splash\.rs' Cargo.toml src --glob '*.rs'
rg -n 'All logic lives in app\.rs|Caller \(app\.rs\)|branch in `app\.rs`|main\.rs, app\.rs' Cargo.toml src --glob '*.rs'
rg -n 'crate::app(::|\b)' src/sim --glob '*.rs'
```

Expected:

```text
False
True
False
True
True
one src/lib.rs match for pub mod app;
no removed-splash-path matches
no present-tense removed-facade matches
no sim-to-app matches
```

Historical “Extracted from app.rs” comments may remain and are not matched by the targeted live-path scan.

**Step 4: Stage the exact scope and inspect a rename-aware diff**

Stage before using diff gates so the new destination files are included:

```powershell
git add -A -- Cargo.toml src/app.rs src/app/mod.rs src/app_startup_splash.rs src/app/frontend/mod.rs src/app/frontend/startup_splash.rs src/app/initialize.rs src/app/state.rs src/app/frame.rs src/asset_tools/palette_production.rs src/main.rs src/app_dev_overlay.rs src/app_skirmish_shell_render.rs
git status --short
git diff --quiet
git diff --cached --check
git diff --cached --find-renames --name-status
git diff --cached --stat
git diff --cached
```

Expected:

- no unstaged diff remains;
- Git recognizes two renames, one new frontend module root, and the listed comment/import/provenance modifications;
- `git diff --cached --check` reports no errors;
- the moved splash implementation and its five tests have no content changes;
- no `src/sim/`, behavior, snapshot, replay, RNG, or ignored planning file appears in the staged diff.

If rename detection presents a delete/add pair but the staged content is correct, verify equivalence with `git diff --cached --find-renames=50%`; do not add compatibility files or duplicate declarations.

**Step 5: Recheck Cargo ownership**

Run:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue
```

Expected: no process owned by another task. If Cargo or rustc is active, wait rather than starting another build.

**Step 6: Run the focused moved-module tests**

Run:

```powershell
cargo test -p vera20k --lib app::frontend::startup_splash::tests
```

Expected: all five moved startup-splash tests pass, and the complete library compiles. Report the literal `test result:` line.

**Step 7: Run the branch's full suite exactly once**

Recheck Cargo ownership, then run:

```powershell
cargo test -p vera20k --lib
```

Expected: the full library suite passes. This is the branch's one PR-readiness run. If it fails, diagnose with focused `--lib` filters; do not repeatedly use the full suite as a debugging loop.

**Step 8: Commit the coherent slice**

Re-run `git diff --cached --check`, inspect `git diff --cached --stat`, and commit:

```text
app: consolidate facade under src/app
```

Then run:

```powershell
git status --short
git log -1 --oneline
```

Expected: the feature worktree is clean and the new commit is at `HEAD`. Do not push or open a PR without explicit user authorization.

## Sources & References

- **Design doc:** `docs/plans/2026-08-15-single-app-tree-design.md`
- **Project architecture:** `ENGINE.md`; `AGENTS.md`
- **Current facade:** `src/app.rs`
- **Current library declaration:** `src/lib.rs`
- **Current splash owner:** `src/app_startup_splash.rs`
- **Current splash consumers:** `src/app/initialize.rs`; `src/app/state.rs`; `src/app/frame.rs`
- **Source-path provenance:** `src/asset_tools/palette_production.rs`
- **Present-tense facade references:** `Cargo.toml`; `src/main.rs`; `src/app_dev_overlay.rs`; `src/app_skirmish_shell_render.rs`
- **Existing directory-module patterns:** `src/app_render/mod.rs`; `src/app_instances/mod.rs`; `src/app_tactical_capture/mod.rs`
- **Recent architecture commits:** `a5367c54` (merged app-sim boundary), `2f9d1f4e` (orchestrator facade cleanup), `32f302ed` (front-end shell orchestration)
- **Ghidra reports:** none required; no native behavior is introduced or reimplemented
- **gamemd.exe addresses:** none required
- **INI keys:** none
