# High-Res Skirmish Shell Screenshot Parity Implementation Plan

> Execute task-by-task. Do not change the verified `>800` parent-background policy unless new Ghidra/runtime evidence contradicts the current reports.

**Goal:** Add a repeatable 1024x768 dev Skirmish shell verification path that catches high-resolution composition drift before patching visuals.

**Design Doc:** [docs/plans/2026-05-22-high-res-skirmish-shell-screenshot-parity-design.md](2026-05-22-high-res-skirmish-shell-screenshot-parity-design.md)

---

## Grounding Summary

Primary docs:

- `docs/research/skirmish-ui/SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`
- `docs/research/skirmish-ui/SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`

Current Rust surfaces:

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app_skirmish_shell_render.rs`
- `src/render/skirmish_shell_chrome.rs`
- `src/app.rs`

Verified baseline:

- Fresh `>800` Skirmish draws no parent-background SHP. Do not draw, stretch, tile, or reuse `MnScrnLCoopGameSetup.shp` above exact width 800.
- 1024x768 right-panel/chrome/control placement follows the high-res hosting report, with Start/Choose superseded by the newer resize-policy report's owner-draw button snap rects.
- Remaining work is aggregate screenshot parity: whether the composed Rust shell visually lines up at high resolution.

## Key Decisions

- Verify and tighten deterministic role/rect tests before any screenshot harness.
- Keep screenshot capture as a visual-check artifact, not a normal required `cargo test`, because GPU and retail assets are environment-sensitive.
- Use existing `compute_layout()` and `build_skirmish_shell_instances()` rather than adding a second layout path.
- Patch only screenshot-confirmed deltas that are already backed by Ghidra/layout reports.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/ui/skirmish_shell/layout.rs` | Add or tighten 1024x768 high-res rect invariant tests. |
| Modify | `src/app_skirmish_shell_render.rs` | Add or expose deterministic draw-role/instance invariant checks; optionally add capture entry point. |
| Create | `docs/visual-checks/skirmish-shell/` | Store visual-check README and generated screenshot notes/artifacts. |
| Optional create | `tests/skirmish_shell_capture.rs` or `docs/visual-checks/skirmish-shell/*` tool | Produce 1024x768 screenshot from runtime-equivalent render path. |
| Optional modify | `src/render/skirmish_shell_chrome.rs` | Only if capture needs a public helper to build the atlas without duplicating loading logic. |

## Parity-Critical Items

| Item | Source | Verification |
|---|---|---|
| 1024x768 parent/background role contains no parent-background SHP | GT800 reconciliation | Unit test against `parent_background_role` and semantic draw order. |
| 800 remains exact `ParentBackgroundCoopGameSetup800` | GT800 reconciliation | Existing plus retained role test. |
| 1024x768 right-panel top is `(744,84,168,199)` | high-res hosting report | Layout test. |
| 1024x768 tile rect is `(744,283,168,42)` and tile count is `9` | high-res hosting report | Layout test. |
| 1024x768 bottom cap is `(744,661,168,23)` | high-res hosting report | Layout test. |
| 1024x768 Start is `(756,325,156,42)` | resize-policy report, `FUN_0060B000` owner-draw snap | Layout test plus screenshot inspection. |
| 1024x768 Choose Map is `(756,367,156,42)` | resize-policy report, `FUN_0060B000` owner-draw snap | Layout test plus screenshot inspection. |
| 1024x768 preview is `(756,121,144,112)` | high-res hosting report | Layout test plus screenshot inspection. |
| 1024x768 Back is `(756,619,156,42)` | high-res hosting report | Layout test plus screenshot inspection. |
| Lower strip stays large/non-640 and aligns from common shell origin | current tests/high-res docs | Unit test and screenshot inspection. |
| `SDBTM` bottom cap is clipped, not vertically scaled | SDBTM source-clip report via synthesis | Screenshot inspection; code fix only if drift appears. |
| Dropdown content/scrollbar remains inside computed rects | combo/dropdown reports/current state code | Role/rect test if opened-dropdown screenshot is added. |

---

## Tasks

### Task 1: Verify and tighten high-res layout invariant tests

**Why:** The screenshot harness should not be the first place we discover obvious coordinate drift.

**Files:**

- `src/ui/skirmish_shell/layout.rs`

**Steps:**

1. Check the existing focused `compute_layout(1024, 768)` tests and add only missing assertions:
   - `right_panel.top == RectPx::new(744, 84, 168, 199)`
   - `right_panel.tile == RectPx::new(744, 283, 168, 42)`
   - `right_panel.tile_count == 9`
   - `right_panel.bottom == RectPx::new(744, 661, 168, 23)`
   - `start_button == RectPx::new(756, 325, 156, 42)`
   - `choose_map_button == RectPx::new(756, 367, 156, 42)`
   - `map_preview == RectPx::new(756, 121, 144, 112)`
   - `back_button == RectPx::new(756, 619, 156, 42)`
2. Add a no-global-scale assertion for 1280x960 that key control sizes remain native.
3. Do not alter formulas in this task unless the tightened tests expose an existing mismatch.

**Checks:**

```powershell
cargo test -q --lib skirmish_shell::layout
```

### Task 2: Pin high-res draw roles

**Why:** The resolved no-parent-background policy should be a hard invariant.

**Files:**

- `src/app_skirmish_shell_render.rs`

**Steps:**

1. Check the existing 1024x768 semantic draw-order test and add only missing assertions that both parent-background roles are absent.
2. Ensure the 1024x768 semantic draw-order test asserts `LowerSideLwscrnl` is present.
3. Add an exact 800 guard if needed: `ParentBackgroundCoopGameSetup800` must still appear at 800.
4. Keep the log message in `parent_background_role()` informational only; tests must not depend on logs.

**Checks:**

```powershell
cargo test -q --lib app_skirmish_shell_render
```

### Task 3: Add a visual-check workspace folder

**Why:** Generated screenshots and notes need a stable place that is separate from production code.

**Files:**

- Create `docs/visual-checks/skirmish-shell/README.md`

**Steps:**

1. Document the purpose: high-res dev Skirmish shell screenshot capture.
2. Record required state:
   - render size `1024x768`;
   - `RA2_DEV_SKIRMISH_SHELL=1`;
   - selected map and preview availability;
   - whether dropdowns are closed or which combo is open.
3. Record invariant expectations from Tasks 1 and 2.
4. State that `>800` no-parent-background-SHP is verified and not under visual debate.

**Checks:**

```powershell
Test-Path docs/visual-checks/skirmish-shell/README.md
```

### Task 4: Choose the capture mechanism

**Why:** The repo needs one repeatable way to produce screenshots before visual fixes are attempted.

**Preferred option:** an ignored integration test or visual-check tool that renders the dev shell at 1024x768 and writes a PNG.

**Fallback option:** a documented manual run path that starts the app at 1024x768 with `RA2_DEV_SKIRMISH_SHELL=1`, then captures the window with a standard screenshot tool.

**Files, preferred path:**

- `tests/skirmish_shell_capture.rs` if a test can reuse runtime asset/render setup cleanly.
- Or `docs/visual-checks/skirmish-shell/skirmish-shell-capture/` if a small standalone visual tool fits existing `docs/visual-checks` convention better.

**Steps:**

1. Inspect existing `docs/visual-checks/*` tools for the lowest-friction pattern.
2. Prefer using runtime atlas/build-instance code over duplicating draw logic.
3. If offscreen `wgpu` capture is too invasive, document the manual capture fallback in the README and stop before writing speculative GPU plumbing.
4. If a tool/test is added, gate it behind `#[ignore]` or a clear manual command.

**Checks:**

```powershell
cargo test -q --lib skirmish_shell
```

If a capture tool/test is added:

```powershell
cargo test --test skirmish_shell_capture -- --ignored --nocapture
```

### Task 5: Generate the baseline 1024x768 Rust screenshot

**Why:** This establishes the current visual truth before fixes.

**Files:**

- `docs/visual-checks/skirmish-shell/`

**Steps:**

1. Generate a closed-dropdown 1024x768 screenshot.
2. Save the artifact with a date-stamped filename, for example:
   - `docs/visual-checks/skirmish-shell/2026-05-22-rust-1024x768-closed.png`
3. Add or update a markdown note next to it with:
   - command used;
   - render size;
   - shell state;
   - selected map;
   - asset source path;
   - observed visible deltas.
4. Do not patch visuals during this task.

**Checks:**

```powershell
Test-Path docs/visual-checks/skirmish-shell/2026-05-22-rust-1024x768-closed.png
```

If manual capture is used, the check is the presence of the screenshot plus the note file.

### Task 6: Review the baseline screenshot against the ledger

**Why:** Patches should be driven by observed mismatches, not guesses.

**Files:**

- `docs/visual-checks/skirmish-shell/*.md`

**Steps:**

1. Inspect the screenshot for:
   - right-panel top/tile/bottom cap location;
   - lower strip location and width;
   - absence of `MnScrnLCoopGameSetup.shp` above 800;
   - Start/Choose/Back stack;
   - preview and right-panel text placement;
   - `SDBTM` clipping versus scaling;
   - dropdown/scrollbar placement if an open-dropdown shot exists.
2. Record deltas in a short table:
   - visible symptom;
   - expected behavior/source;
   - suspected Rust surface;
   - proposed fix task;
   - whether new RE is needed.
3. If the screenshot is blank or asset-incomplete, fix the capture path before making visual changes.

**Checks:**

No cargo command required. The output is an updated visual note with an explicit delta table.

### Task 7: Patch only confirmed high-res deltas

**Why:** Avoid replacing verified behavior with convenient approximations.

**Files:**

- Determined by Task 6 delta table.

**Allowed fix classes:**

- Rect formula mismatch in `src/ui/skirmish_shell/layout.rs`.
- Draw-order mismatch in `src/app_skirmish_shell_render.rs`.
- Missing or wrong source clipping for right-panel/lower-strip pieces.
- Screenshot capture bug that does not represent runtime state.

**Explicitly disallowed without new evidence:**

- Drawing any parent-background SHP above 800.
- Globally scaling the dialog.
- Moving non-allowlisted slot-table controls into a centered 800x600 group.
- Treating the 800 path and `>800` path as the same "large" path.

**Checks after each patch:**

```powershell
cargo fmt
cargo test -q --lib skirmish_shell
cargo test -q --lib app_skirmish_shell_render
```

Then regenerate the screenshot from Task 5.

### Task 8: Add an opened-dropdown high-res screenshot

**Why:** Dropdown scrolling and scrollbar work is now implemented and is likely to expose high-res clipping mistakes.

**Files:**

- `docs/visual-checks/skirmish-shell/`
- Optional test/tool state setup if capture is automated.

**Steps:**

1. Set up a deterministic shell state with one dropdown open and enough rows to show a scrollbar.
2. Generate a 1024x768 opened-dropdown screenshot.
3. Inspect:
   - dropdown top/index row alignment;
   - 23 px row cadence;
   - 20 px scrollbar reservation;
   - content-width shrink;
   - thumb and arrow placement;
   - hit-test rect correspondence if debug overlays are available.
4. Record deltas separately from the closed-shell composition deltas.

**Checks:**

If automated:

```powershell
cargo test --test skirmish_shell_capture -- --ignored --nocapture
```

If manual, verify artifact and note files exist.

### Task 9: Final verification pass

**Why:** The plan's output should leave a reproducible trail, not just local confidence.

**Steps:**

1. Run focused deterministic checks:

```powershell
cargo fmt
cargo test -q --lib skirmish_shell
cargo test -q --lib app_skirmish_shell_render
```

2. Regenerate the closed 1024x768 screenshot.
3. Regenerate the opened-dropdown 1024x768 screenshot if Task 8 was implemented.
4. Update the visual-check README/note with:
   - final commands;
   - final artifact paths;
   - remaining deltas;
   - whether any remaining delta needs RE, screenshot comparison, or implementation.

## Out of Scope

- Retail screenshot capture automation.
- Full automated pixel diff against retail.
- Default-enabling the dev Skirmish shell.
- Any `sim/` changes.
- Any change to the verified fresh `>800` no-parent-background-SHP behavior.

## Stop Conditions

Stop and reassess if:

- 1024x768 tests imply drawing `MnScrnLCoopGameSetup.shp` above 800.
- The screenshot capture path produces a different composition than runtime.
- Missing retail assets make the capture misleading.
- A visual mismatch is not covered by existing Ghidra/layout reports.
- Unrelated dirty-tree compile failures block verification.

## Expected Final Artifacts

- Deterministic tests pinning 1024x768 layout and no-parent-background roles.
- A documented 1024x768 closed-dropdown Rust screenshot.
- Optionally, a documented 1024x768 opened-dropdown Rust screenshot.
- A visual delta table that names any remaining high-res composition mismatch.
- Focused `cargo test` output for `skirmish_shell` and `app_skirmish_shell_render`.
