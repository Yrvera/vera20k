# Skirmish Shell Verified-Assets Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not commit unless the user explicitly asks for commits in the execution session.

**Goal:** Keep the dedicated Skirmish shell scaffolding, but make the visible implementation evidence-gated: only assets and draw behavior verified as active in offline YR Skirmish dialog `0x102` may render by default; unknown backgrounds stay blank or debug-only.

**Architecture:** UI/render/app work only. `ui/skirmish_shell` remains render-agnostic, `render/skirmish_shell_chrome` owns retail shell asset loading and atlas entries, and app-level code chooses whether the shell is visible. No `sim/` changes are part of this plan.

**Design Doc:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`

**Supersedes:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-plan.md` for asset/rendering policy. The old plan allowed plausible shell assets as substitutions; this plan forbids that for default-visible rendering.

---

## Grounding Summary

- The old design doc has valid architecture context and impact analysis, but its chosen approach assumed the shell could become the normal visible path before every visual asset path was proven.
- `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md` corrects that: active right-panel geometry and owner-draw controls are verified, but several background candidates remain generic shell evidence rather than offline Skirmish proof.
- Live Ghidra MCP is not available in this session; `list_instances` reports no running instances. This plan uses existing verified Ghidra reports and explicitly defers fresh binary-only questions.
- Active YR evidence is high for dialog `0x102`, right-panel anchoring, Back button placement, `SDBTNANM/SDBTNBKGD/SDTP/SDBTM` dimensions, `bue_*30` and `bde_*30` button pieces, flag PCX item-data mapping, and `STARTBUT.SHP` start markers.
- Evidence is insufficient for rendering `MNSCRNL.SHP`, `MNSCRNS.SHP`, `MnScrnLCustomizeBattle.shp`, `dbak6440.pcx`, or `dlgsys*.pcx` as the default offline Skirmish background.
- Owner-draw PCX controls use embedded PCX palettes. `sidebar.pal` is not valid evidence for buttons or flags.
- `mmpb.shp` is a verified map-preview/player-marker asset, not a generic preview backing. It must not be used as a placeholder background for child `0x468`.
- The current Rust implementation already has useful scaffolding, but the default visible path and asset substitutions must be corrected before further parity work.
- Existing repo pattern remains sidebar-style layering: pure layout/state in UI modules, asset atlas in render modules, and sprite instance wiring in app-layer modules.
- No INI key drives the shell background/layout. INI relevance is limited to player-facing setup values such as `[Countries]`, `[Sides]`, `[Colors]`, and `[MultiplayerDialogSettings]`.

## Key Technical Decisions

- **Default visibility is gated off until verified rendering is sufficient.** The previous egui Skirmish setup remains the normal visible screen unless an explicit dev flag enables the shell. **Confidence:** high
  - **Source:** active render path reinvestigation report; current implementation mismatch list.
- **Default shell renderer must never silently substitute unverified backgrounds.** Unknown backgrounds render as transparent/blank, or as clearly labeled debug overlays only when the dev shell flag is enabled. **Confidence:** high
  - **Source:** `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`.
- **Verified active assets are allowlisted by role.** Right-panel SHPs, button PCX pieces, flag PCXs, and `STARTBUT.SHP` may be loaded/rendered by default shell code; generic shell backgrounds may be loaded only as research candidates. **Confidence:** high
  - **Source:** owner-draw callback reports and viewport follow-up report.
- **PCX owner-draw assets use embedded palettes.** Do not use `sidebar.pal` as a first-choice palette for shell PCX controls. **Confidence:** high
  - **Source:** owner-draw callbacks follow-up section on PCX conversion.
- **Fresh Ghidra remains required before claiming full background parity.** This plan can gate/fix the current renderer, but cannot promote a background candidate to verified without a live trace or retail screenshot proof. **Confidence:** high
  - **Source:** no running Ghidra instances in this session; active render path report open questions.

## Open Questions

### Resolved During Planning

- Is the current visible replacement good enough to remain default? No. It renders unverified background/preview/text assumptions.
- Can layout tests and shell scaffolding stay? Yes. The layout/state split is useful and mostly aligned with verified geometry.
- Should unverified assets be deleted? No. Keep them available as research candidates, but do not render them in the default visible path.

### Deferred To Live Ghidra Or Screenshot Verification

- Exact `0x0072CF40` background/palette resource names and how `DAT_00B0FCDC` / `DAT_00B0FCE0` are consumed by shell paint.
- Exact common shell background paint order in `0x00622B50`.
- Exact text/font behavior in `0x00621040`.
- Exact `mmpb.shp` ordering relative to map preview and `STARTBUT.SHP` in offline Skirmish.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app.rs` | Restore egui MainMenu as default; enable shell only through explicit dev flag |
| Modify | `src/app_skirmish_shell_render.rs` | Render only allowlisted verified shell surfaces by default; debug-only overlays for unknowns |
| Modify | `src/render/skirmish_shell_chrome.rs` | Split verified assets from research candidates; remove `sidebar.pal` dependence for PCX controls |
| Modify | `src/ui/skirmish_shell/state.rs` | Keep launch bridge and hit testing; no render-specific assumptions |
| Modify | `src/ui/skirmish_shell/layout.rs` | Keep and extend verified geometry tests if needed |
| Optional modify | `src/app_types.rs` | Add explicit dev-shell flag only if `AppState` owns the flag there |
| Optional modify | config-loading file used by `GameConfig` | Add shell dev flag only if repo has an established config surface; otherwise use env var |

## Interface Changes

- Preserve the existing explicit app-level gate for the shell renderer:
  - Existing gate: environment variable `RA2_DEV_SKIRMISH_SHELL`.
  - Default value remains off/false.
  - Do not add a second shell gate such as `VERA20K_SKIRMISH_SHELL`.
- Do not move `SkirmishSettings`.
- Do not add any `sim/` dependency or gameplay contract.

## Sim Checklist

This plan does not touch `src/sim/`.

- [x] No fixed-point or floating-point sim math changes.
- [x] No deterministic state hash changes.
- [x] No dependency from `sim/` to UI/render/sidebar/audio/net.
- [x] No tick ordering impact.
- [x] No `EntityStore` iteration impact.

## Risk Areas

- The working tree is dirty. Do not revert unrelated files. If unrelated dirty code breaks `cargo check`, report exact errors and stop for direction.
- Gating must not remove shell modules from compilation; future tests should still catch layout/asset regressions.
- Shell asset loading should be skipped or lazy when the shell dev flag is off, otherwise the default egui menu can still pay startup cost or log confusing asset warnings.
- Debug overlays must be impossible to confuse with verified parity output. They should require the explicit dev shell flag.
- The prior `cargo fmt` touched unrelated `sim/` files in the earlier session. Do not run broad formatting unless necessary.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Default visible screen returns to egui | Prevents players from seeing unverified shell pixels as parity work | Run app and confirm egui setup screen appears without env flag |
| Task 2 | Shell dev flag is explicit and default-off | Keeps scaffolding testable without replacing the trusted menu | Run app with and without flag |
| Task 3 | Verified-vs-research asset split | Prevents generic shell assets from being treated as Skirmish evidence | Unit/assertion checks and code inspection |
| Task 4 | Embedded PCX palette path | Button/flag colors must come from the PCX evidence, not sidebar palette guesses | `cargo test skirmish_shell -- --nocapture`; visual dev-shell check |
| Task 5 | Preview does not use `mmpb.shp` as backing | Avoids wrong marker/backdrop semantics in child `0x468` | Code inspection and dev-shell screenshot |
| Task 6 | Full flag PCX mapping | Country rows visibly differ in the original shell | Unit test mapping for item data values |

---

## Tasks

### Task 1: Verify egui Skirmish Setup Is Default MainMenu

**Why:** The current shell replacement is visibly unverified; default behavior must return to the known functional setup screen before further parity iteration.

**Files:**
- Modify: `src/app.rs`

**Pattern:** Narrow app-level screen routing. Leave shell modules compiled.

**Step 1: Locate the `GameScreen::MainMenu` render branch**

Find the branch that currently calls:

```rust
crate::app_skirmish_shell_render::render_skirmish_shell(...)
```

Confirm default behavior already uses the egui `main_menu::draw_main_menu_with_maps` flow when `state.dev_skirmish_shell_enabled` is false. Preserve the existing `MenuAction` handling that updates `state.skirmish_settings`, chooses a map, enters `GameScreen::Loading`, or exits.

**Step 2: Preserve the existing shell gate**

Confirm the existing helper remains the single shell gate:

```rust
fn skirmish_shell_dev_enabled() -> bool {
    std::env::var(DEV_SKIRMISH_SHELL_ENV)
        .ok()
        .is_some_and(|value| {
            let value = value.trim();
            !value.is_empty()
                && value != "0"
                && !value.eq_ignore_ascii_case("false")
                && !value.eq_ignore_ascii_case("off")
                && !value.eq_ignore_ascii_case("no")
        })
}
```

`DEV_SKIRMISH_SHELL_ENV` should remain:

```rust
const DEV_SKIRMISH_SHELL_ENV: &str = "RA2_DEV_SKIRMISH_SHELL";
```

Do not introduce `VERA20K_SKIRMISH_SHELL`.

**Step 3: Branch explicitly**

The `MainMenu` render branch should be structurally:

```rust
GameScreen::MainMenu => {
    if skirmish_shell_dev_enabled() {
        crate::app_skirmish_shell_render::render_skirmish_shell(state, &mut encoder, &view)?;
    } else {
        // existing egui menu path
    }
}
```

Do not route input to shell when the dev flag is off.

**Step 4: Verify**

Run:

```powershell
cargo check
```

Expected: shell modules still compile; default MainMenu uses egui code.

### Task 2: Gate Shell Input And Startup Asset Loading

**Why:** A hidden shell should not steal clicks or load/log shell assets during normal startup.

**Files:**
- Modify: `src/app.rs`
- Optional modify: `src/app_types.rs` if shell asset ownership lives there

**Pattern:** Same predicate as Task 1; one source of truth for shell-enabled state.

**Step 1: Verify MainMenu mouse input is gated**

Find the `WindowEvent::MouseInput` branch for `GameScreen::MainMenu`. It should only call `handle_skirmish_shell_click` when `state.dev_skirmish_shell_enabled` is true.

When the flag is off, let the existing egui input path handle the menu.

**Step 2: Verify startup shell chrome atlas creation is gated**

Only build `skirmish_shell_chrome` when the dev flag is enabled. The shape should remain:

```rust
let skirmish_shell_chrome = if skirmish_shell_dev_enabled() {
    startup_asset_manager.as_ref().and_then(|assets| {
        crate::render::skirmish_shell_chrome::build_skirmish_shell_chrome_atlas(
            &gpu,
            &batch_renderer,
            assets,
        )
    })
} else {
    None
};
```

Keep `skirmish_shell_state` initialized even when the flag is off so tests and future code can compile cleanly.

**Step 3: Verify**

Run:

```powershell
cargo check
```

Then run without the flag:

```powershell
cargo run --bin vera20k
```

Expected: egui Skirmish setup appears; no shell-specific asset warnings appear during normal startup.

### Task 3: Split Verified Assets From Research Candidates

**Why:** The renderer must encode the evidence boundary so future work cannot accidentally render plausible but unverified backgrounds.

**Files:**
- Modify: `src/render/skirmish_shell_chrome.rs`

**Pattern:** Asset-role naming, similar to sidebar chrome entries, but with explicit evidence status.

**Step 1: Add separate fields for verified and research-only assets**

Use role names that prevent accidental substitution:

```rust
pub struct SkirmishShellChromeAtlas {
    pub right_panel_top: Option<SkirmishShellChromeEntry>,
    pub right_panel_tile: Option<SkirmishShellChromeEntry>,
    pub right_panel_bottom: Option<SkirmishShellChromeEntry>,
    pub back_button_anim: Option<SkirmishShellChromeEntry>,
    pub map_button_panel: Option<SkirmishShellChromeEntry>,
    pub start_marker: Option<SkirmishShellChromeEntry>,
    pub button_up_left_30: Option<SkirmishShellChromeEntry>,
    pub button_up_mid_30: Option<SkirmishShellChromeEntry>,
    pub button_up_right_30: Option<SkirmishShellChromeEntry>,
    pub button_down_left_30: Option<SkirmishShellChromeEntry>,
    pub button_down_mid_30: Option<SkirmishShellChromeEntry>,
    pub button_down_right_30: Option<SkirmishShellChromeEntry>,
    pub flags: Vec<(String, SkirmishShellChromeEntry)>,
    pub research_candidates: Vec<(String, SkirmishShellChromeEntry)>,
}
```

If renaming all existing fields is too broad for this task, keep old field names but add `research_candidates` and stop exposing `background_large`, `background_small`, or `preview_marker` to default instance building.

**Step 2: Define allowlisted verified asset names**

Verified default-render assets:

- `SDTP.SHP`
- `SDBTNBKGD.SHP`
- `SDBTM.SHP`
- `SDBTNANM.SHP`
- `SDMPBTN.SHP` only as right-panel/map-button chrome, not preview backing
- `STARTBUT.SHP`
- `bue_li30.pcx`
- `bue_mi30.pcx`
- `bue_ri30.pcx`
- `bde_li30.pcx`
- `bde_mi30.pcx`
- `bde_ri30.pcx`
- `usai.pcx`
- `japi.pcx`
- `frai.pcx`
- `geri.pcx`
- `gbri.pcx`
- `djbi.pcx`
- `arbi.pcx`
- `lati.pcx`
- `rusi.pcx`
- `yrii.pcx`
- `obsi.pcx`
- `rani.pcx`

Research-only candidates:

- `MNSCRNL.SHP`
- `MNSCRNS.SHP`
- `MnScrnLCustomizeBattle.shp`
- `dbak6440.pcx`
- `dlgsysa.pcx`
- `dlgsysi.pcx`
- `mmpb.shp`

**Step 3: Do not fail atlas construction because research candidates are absent**

Missing verified button PCXs should log a warning when the dev shell is enabled. Missing research candidates should not warn unless a verbose/debug asset logging path already exists.

**Step 4: Verify**

Run:

```powershell
cargo check
cargo test skirmish_shell -- --nocapture
```

Expected: atlas compiles; shell tests still pass.

### Task 4: Remove Sidebar Palette From PCX Owner-Draw Path

**Why:** Verified owner-draw PCX controls use embedded palettes; `sidebar.pal` is an unsupported color source for shell buttons/flags.

**Files:**
- Modify: `src/render/skirmish_shell_chrome.rs`
- Modify: `src/assets/pcx_file.rs` only if parser API needs a clearer embedded-palette method

**Pattern:** Asset parser owns PCX palette extraction; render atlas consumes RGBA.

**Step 1: Keep SHP palette handling separate from PCX handling**

SHP decoding may still require a palette until fresh Ghidra proves the exact shell SHP palette. PCX decoding must use:

```rust
let pcx = PcxFile::parse(bytes)?;
let rgba = pcx.to_rgba(transparent_index);
```

Do not pass `sidebar.pal`, `SHELL.PAL`, or `DIALOG.PAL` into PCX conversion.

**Step 2: Remove `sidebar.pal` as first fallback for shell SHPs**

If SHP entries still need a palette, prefer a clearly named unresolved helper:

```rust
fn shell_shp_palette(assets: &AssetManager) -> Option<Palette> {
    assets
        .get_ref("SDBTNANM.PAL")
        .or_else(|| assets.get_ref("SHELL.PAL"))
        .or_else(|| assets.get_ref("DIALOG.PAL"))
        .and_then(|bytes| Palette::from_bytes(bytes).ok())
}
```

Add a comment that exact SHP palette binding remains open pending live Ghidra. Do not use this palette for PCX assets.

**Step 3: Verify**

Run:

```powershell
cargo check
```

Expected: no shell PCX code references `sidebar.pal`.

### Task 5: Make Unknown Backgrounds Blank Or Debug-Only

**Why:** The default shell render must not present unverified art as parity output.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/render/skirmish_shell_chrome.rs` if research candidate accessors are needed

**Pattern:** Default renderer draws only verified surfaces; debug renderer may draw candidates behind an explicit dev flag.

**Step 1: Remove default drawing of unverified background candidates**

In `build_skirmish_shell_instances`, do not draw:

- `MNSCRNL.SHP`
- `MNSCRNS.SHP`
- `MnScrnLCustomizeBattle.shp`
- `dbak6440.pcx`
- `dlgsysa.pcx`
- `dlgsysi.pcx`

Default behavior should clear the frame and draw only verified right-panel/control assets.

**Step 2: Add an optional debug background mode only under the shell dev flag**

If a debug background is useful, require a second explicit variable that follows the existing shell gate name:

```rust
RA2_DEV_SKIRMISH_SHELL_DEBUG_BG=1
```

Debug background drawing must be visibly annotated through logging:

```rust
log::warn!("Rendering unverified Skirmish shell background candidate {name}; debug only");
```

Do not enable this mode by default.

**Step 3: Verify**

Run:

```powershell
cargo check
```

Run with `RA2_DEV_SKIRMISH_SHELL=1` and confirm the shell does not show a guessed full background unless `RA2_DEV_SKIRMISH_SHELL_DEBUG_BG=1` is also set.

### Task 6: Correct Flag Asset Mapping

**Why:** The original shell maps side/item data to exact PCX names; collapsing countries to faction families loses visible icons.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Optional modify: `src/ui/skirmish_shell/state.rs` if state needs an item-data representation

**Pattern:** Pure mapping function with unit tests.

**Step 1: Replace country-family mapping with explicit PCX mapping**

Add a mapping function using the verified item-data table:

```rust
fn flag_pcx_for_side_item_data(item_data: i32) -> Option<&'static str> {
    match item_data {
        -3 => Some("obsi.pcx"),
        -2 => Some("rani.pcx"),
        0 => Some("usai.pcx"),
        1 => Some("japi.pcx"),
        2 => Some("frai.pcx"),
        3 => Some("geri.pcx"),
        4 => Some("gbri.pcx"),
        5 => Some("djbi.pcx"),
        6 => Some("arbi.pcx"),
        7 => Some("lati.pcx"),
        8 => Some("rusi.pcx"),
        9 => Some("yrii.pcx"),
        _ => None,
    }
}
```

Current shell state stores `SkirmishCountry`, not original combo item data. Add this explicit temporary conversion next to the renderer and use it before calling `flag_pcx_for_side_item_data`:

```rust
fn side_item_data_for_country(country: SkirmishCountry) -> i32 {
    match country {
        SkirmishCountry::America => 0,
        SkirmishCountry::Korea => 1,
        SkirmishCountry::France => 2,
        SkirmishCountry::Germany => 3,
        SkirmishCountry::GreatBritain => 4,
        SkirmishCountry::Libya => 5,
        SkirmishCountry::Iraq => 6,
        SkirmishCountry::Cuba => 7,
        SkirmishCountry::Russia => 8,
        SkirmishCountry::Yuri => 9,
    }
}
```

This mapping follows the verified owner-draw item-data to PCX table while the shell state still reuses the existing `SkirmishCountry` enum.

**Step 2: Missing flags render blank**

If `flag_pcx_for_side_item_data` returns `None` or the atlas lacks the PCX, draw nothing for that flag static. Do not fall back to observer except when item data is explicitly `-3`.

**Step 3: Add tests**

Add tests for:

- `-3 -> obsi.pcx`
- `-2 -> rani.pcx`
- `0 -> usai.pcx`
- `1 -> japi.pcx`
- `2 -> frai.pcx`
- `3 -> geri.pcx`
- `4 -> gbri.pcx`
- `5 -> djbi.pcx`
- `6 -> arbi.pcx`
- `7 -> lati.pcx`
- `8 -> rusi.pcx`
- `9 -> yrii.pcx`
- `SkirmishCountry::Korea -> 1 -> japi.pcx`
- `SkirmishCountry::GreatBritain -> 4 -> gbri.pcx`
- `SkirmishCountry::Cuba -> 7 -> lati.pcx`
- unknown value returns `None`

**Step 4: Verify**

Run:

```powershell
cargo test skirmish_shell -- --nocapture
cargo check
```

### Task 7: Correct Map Preview Placeholder Behavior

**Why:** `mmpb.shp` and `SDMPBTN.SHP` are not verified preview backings. The preview area must not lie about parity.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`

**Pattern:** Render nothing for unknown content; render verified markers only when enough data exists.

**Step 1: Stop fitting `mmpb.shp` or `SDMPBTN.SHP` into `layout.map_preview`**

Remove default code equivalent to:

```rust
atlas.preview_marker.or(atlas.sd_map_button)
```

as a preview backing.

**Step 2: Draw a debug-only preview rectangle when shell dev flag is enabled**

Use a simple solid color or no draw. If a debug rectangle is drawn, log once:

```rust
log::warn!("Map preview backing is debug-only; active Skirmish preview draw path is unresolved");
```

Do not include this in default egui menu mode.

**Step 3: Keep `STARTBUT.SHP` available for future marker rendering**

Do not draw `STARTBUT.SHP` until the map preview projection uses scenario visible-map bounds and child `0x468` final coordinates. If a simple proof marker is needed, put it behind the dev shell flag and label it in code as non-parity.

**Step 4: Verify**

Run:

```powershell
cargo check
```

Expected: no default code uses `mmpb.shp` as a map preview backing.

### Task 8: Add Evidence Comments And Tests Around The Asset Policy

**Why:** The policy must be hard to regress when future sessions resume the shell work.

**Files:**
- Modify: `src/render/skirmish_shell_chrome.rs`
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/ui/skirmish_shell/layout.rs` only if test names need updating

**Pattern:** Short comments that explain why, not what.

**Step 1: Add one module-level policy comment in `skirmish_shell_chrome.rs`**

Use a concise comment like:

```rust
//! Shell chrome loading for verified offline Skirmish dialog `0x102` assets.
//!
//! Assets without direct active-Skirmish evidence are kept as research
//! candidates and must not be rendered by the default shell path.
```

**Step 2: Add one render-path guard comment in `app_skirmish_shell_render.rs`**

Place it before background/preview drawing:

```rust
// Unknown shell backgrounds stay blank here. Rendering plausible candidates
// made the first parity pass misleading; promote an asset only after live
// Ghidra or screenshot evidence ties it to offline Skirmish dialog 0x102.
```

**Step 3: Add a focused test for verified/research classification**

If classification is represented as functions or arrays, add a unit test that asserts:

- `STARTBUT.SHP` is verified.
- `bue_li30.pcx` is verified.
- `mmpb.shp` is research-only.
- `MNSCRNL.SHP` is research-only.
- `sidebar.pal` is not a PCX owner-draw palette source.

**Step 4: Verify**

Run:

```powershell
cargo test skirmish_shell -- --nocapture
cargo check
```

### Task 9: Manual Smoke Verification

**Why:** The core user-visible fix is default screen behavior, not just compilation.

**Files:**
- No source changes unless verification finds a bug

**Step 1: Run without dev flag**

```powershell
cargo run --bin vera20k
```

Expected: previous egui Skirmish setup screen is visible.

**Step 2: Run with dev flag**

```powershell
$env:RA2_DEV_SKIRMISH_SHELL='1'
cargo run --bin vera20k
```

Expected: shell scaffolding is visible, but unknown backgrounds are blank/debug-only and no unverified preview backing is drawn.

**Step 3: Clear flag after verification**

```powershell
Remove-Item Env:\RA2_DEV_SKIRMISH_SHELL
Remove-Item Env:\RA2_DEV_SKIRMISH_SHELL_DEBUG_BG -ErrorAction SilentlyContinue
```

**Step 4: Report dirty-worktree boundaries**

Run:

```powershell
git status --short
```

Expected: no `sim/` files were modified by this plan. If `sim/` files are dirty from unrelated prior work, report them as pre-existing/unrelated rather than touching them.

## Sources & References

- **Design doc:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`
- **Superseded plan:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-plan.md`
- **Correction report:** `docs/research/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`
- **Ghidra reports:**
  - `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- **Open live Ghidra targets:** `0x0072CF40`, `0x00622B50`, `0x0060CF00`, `0x00612B70`, `0x00621040`, `0x00640710`, `0x00640A40`
- **Related code:**
  - `src/app.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/render/skirmish_shell_chrome.rs`
  - `src/assets/pcx_file.rs`
  - `src/ui/skirmish_shell/layout.rs`
  - `src/ui/skirmish_shell/state.rs`
- **INI references:**
  - `ini/rulesmd.ini [Countries]`
  - `ini/rulesmd.ini [Sides]`
  - `ini/rulesmd.ini [Colors]`
  - `ini/rulesmd.ini [MultiplayerDialogSettings]`

## Post-Plan Self-Review

- Spec coverage: the plan addresses the user’s corrected rule directly and keeps the old architecture boundaries.
- Placeholder scan: no placeholder tasks remain.
- Architecture check: UI, render, and app responsibilities stay separated.
- Interface ordering: shell gate and asset policy land before render behavior changes.
- Risk coverage: default visibility, asset substitutions, palette misuse, preview misuse, and dirty worktree risk are explicit.
- Self-containment: each task names files, concrete changes, and verification commands.
- Sim compliance: no task touches `sim/`.
- Grounding coverage: design, prior plan, Ghidra reports, current Rust, and INI references are cited.
- Confidence tagging: key decisions include confidence and sources.
- Deferred questions: live Ghidra-only findings are separated from implementation tasks.
- Parity-critical items: default visibility, asset evidence, palette, preview, and flags are listed.
