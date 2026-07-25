# Exact Main-Menu 0xE2 RGB565 Presentation Implementation Plan

**Date:** 2026-07-25
**Design:**
`docs/plans/2026-07-25-exact-shell-e2-rgb565-presentation-design.md`
**Checkpoint:** `SLICE-20260725-GSI-03-01-E2-RGB565-PRESENTATION`
**Status:** SELF-APPROVED FOR IMPLEMENTATION
**Feature branch:** `feature/exact-shell-e2-rgb565-presentation-20260725-2250`
**Validated base:** `5e8b4eaf2c141913b6d6d9a2e984f3602c84407e`

## Grounding Summary

### Native executable

- Active `gamemd.exe` constructs the relevant DirectDraw surface through the
  function entry at `0x004BA770`, derives channel losses/shifts from runtime
  masks, supports both RGB565 and RGB555, and uses a 16-bit shell surface.
- The current retail `DDrawCompat-gamemd.log` selects `D3DDDIFMT_R5G6B5` for
  plain, primary, and system-memory surfaces.
- The guarded presentation has exact 32/64/32 channel cardinalities across all
  three sealed source frames.

### Differential

- Applying `L5[channel >> 3]` to red/blue and `L6[channel >> 2]` to green makes
  all 63,506 physical pixels in the three opaque button crops exact.
- The title remains red and therefore keeps the encompassing 800x600
  steady-paint matrix row at DRIFT.
- The guard-proven transform is tied to exact guard/source hashes. It is not a
  universal DirectDraw expansion claim.

### Rust production route

- `App::redraw` owns the acquired `wgpu::SurfaceTexture`.
- `render_main_menu_shell` is the normal `0xE2` wrapper.
- The pre-implementation caller recheck found that first-paint transition code
  still called `render_main_menu_shell` with an offscreen view; this plan must
  split that caller explicitly to `render_main_menu_shell_to_target`.
- `ShellRenderTarget` already abstracts borrowed offscreen color/depth views.
- egui renders after the shell wrapper, and frame readback copies the acquired
  swapchain texture after egui.
- `GpuContext` currently requests `RENDER_ATTACHMENT | COPY_SRC` surface usage.
- `DirectDrawPixelFormat` currently lives in `render/tactical_compat.rs` even
  though the shell presenter now needs the same native format description.

## Decisions

| Decision | Confidence | Source |
|---|---:|---|
| Preserve runtime-derived loss/shift structure and define RGB565/RGB555 known fixtures | High | live decompile at `0x004BA770`, current tactical code |
| Use the guard-observed 5/6-bit codebooks for the enrolled presentation profile | High for this guard | three sealed executable frames |
| Apply the effect after complete shell composition | High for opaque steady crop | exact 63,506-pixel differential and native final surface |
| Scope the presenter to the normal `0xE2` wrapper | High | direct Rust caller trace |
| Sample encoded bytes through a compatible non-sRGB view | High | WGPU format/view rules and byte-domain requirement |
| Render to non-sRGB intermediate then copy bytes to the sRGB swapchain texture | High, pending production capture | WGPU copy-compatible format pair |
| Keep first-paint transition on its current lower-level target path | High | Rust caller trace; transition evidence not yet comparable |
| Preserve egui, loading, tactical, and other shell paths | High | user scope and impact review |

## Open Questions Retained as Residuals

- Whether native quantizes between individual alpha/blend operations.
- Whether Bink's decoded/copy surface needs an earlier packed-domain boundary.
- Why the title's final lowercase `u` uses native red 25 rather than red 0.
- Whether another enrolled renderer expands 5/6-bit channels differently.
- Exact hover/press/modal/fade/transition behavior after packed presentation.

None changes the opaque steady implementation seam. Each remains unverified
and must not be promoted by this checkpoint.

## File Map

### Add

- `src/render/native_surface_format.rs`
- `src/render/shell_surface_present.rs`
- `src/render/shell_surface_present.wgsl`
- `tools/shell_certification/presentation_profile.py`
- `tools/shell_certification/tests/test_presentation_profile.py`
- `docs/plans/2026-07-25-exact-shell-e2-rgb565-presentation-design.md`
- `docs/plans/2026-07-25-exact-shell-e2-rgb565-presentation-plan.md`

### Modify

- `src/render/mod.rs`
- `src/render/tactical_compat.rs`
- `src/render/gpu.rs`
- `src/app.rs`
- `src/app_main_menu_shell_render.rs`
- `src/app_shell_transition.rs`
- `tools/shell_certification/cli.py`
- `tools/shell_certification/README.md`
- `docs/research/DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS_GHIDRA_REPORT.md`
- `docs/research/DIRECTDRAW_LIVE_PIXEL_FORMAT_RUNTIME_SAMPLE_GHIDRA_REPORT.md`
- `docs/research/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`
- `docs/research/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md`
- `system_map/topology.v2.json` only if the existing schema accepts one truthful
  affected surface/edge without inventing status.

### Explicitly untouched

- `src/assets/asset_manager.rs`
- `src/render/sidebar_chrome.rs`
- `src/app_sidebar_build.rs`
- Oracle guard and source evidence
- Ghidra labels/comments
- matrix completion fields
- loading/tactical/single-player/skirmish render routes

## Interfaces

### Shared native surface format

`src/render/native_surface_format.rs`:

```rust
//! Native DirectDraw pixel-format semantics shared by compatibility renderers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectDrawPixelFormat {
    pub red_loss: u32,
    pub red_shift: u32,
    pub green_loss: u32,
    pub green_shift: u32,
    pub blue_loss: u32,
    pub blue_shift: u32,
    pub destination_bytes_per_pixel: u8,
}

pub const RGB565: DirectDrawPixelFormat = DirectDrawPixelFormat {
    red_loss: 3,
    red_shift: 11,
    green_loss: 2,
    green_shift: 5,
    blue_loss: 3,
    blue_shift: 0,
    destination_bytes_per_pixel: 2,
};

pub const RGB555: DirectDrawPixelFormat = DirectDrawPixelFormat {
    red_loss: 3,
    red_shift: 10,
    green_loss: 3,
    green_shift: 5,
    blue_loss: 3,
    blue_shift: 0,
    destination_bytes_per_pixel: 2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSurfacePresentationProfile {
    pub format: DirectDrawPixelFormat,
    pub five_bit: [u8; 32],
    pub six_bit: [u8; 64],
}

impl NativeSurfacePresentationProfile {
    pub fn quantize_rgba8(self, rgba: [u8; 4]) -> [u8; 4];
    pub(crate) fn shader_words(self) -> [u32; 96];
}

pub const ACTIVE_RETAIL_RGB565_PRESENTATION:
    NativeSurfacePresentationProfile = /* exact guard-derived arrays */;
```

The profile method indexes channels using the format's losses and preserves
alpha. The six-bit table is present because the active format is RGB565;
RGB555 unit fixtures use the five-bit table for every color channel.

`src/render/tactical_compat.rs` removes the local struct definition and
preserves the public path:

```rust
pub use super::native_surface_format::DirectDrawPixelFormat;
```

Its existing `pack_rgb` behavior remains unchanged.

### Shell presenter

`src/render/shell_surface_present.rs`:

```rust
//! Encoded-byte RGB565 presentation for the stock main-menu shell.

pub(crate) struct ShellSurfacePresenter {
    source_texture: wgpu::Texture,
    source_render_view: wgpu::TextureView,
    source_encoded_view: wgpu::TextureView,
    presented_texture: wgpu::Texture,
    presented_view: wgpu::TextureView,
    profile_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    surface_format: wgpu::TextureFormat,
    encoded_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}

impl ShellSurfacePresenter {
    pub(crate) fn new(gpu: &GpuContext) -> anyhow::Result<Self>;
    pub(crate) fn source_render_view(&self) -> wgpu::TextureView;
    pub(crate) fn resize(&mut self, gpu: &GpuContext);
    pub(crate) fn encode_present(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        destination: &wgpu::Texture,
    );
}
```

Construction requires an sRGB surface format whose
`remove_srgb_suffix()` differs only by the compatible unorm view. The source
texture uses:

```text
format = surface sRGB
usage = RENDER_ATTACHMENT | TEXTURE_BINDING
view_formats = [encoded unorm]
```

The presented texture uses:

```text
format = encoded unorm
usage = RENDER_ATTACHMENT | COPY_SRC
```

The source render view explicitly uses the sRGB format. The source sampling
view explicitly uses the encoded unorm format. `encode_present` renders one
fullscreen triangle to `presented_view`, then calls
`copy_texture_to_texture` for the exact configured extent into the acquired
surface texture.

The profile buffer contains 96 `u32` words: the 32-entry five-bit table followed
by the 64-entry six-bit table. It is a read-only storage binding so WGSL indexes
exact codebook values without transfer-function ambiguity.

### Shader

`src/render/shell_surface_present.wgsl` uses no sampler:

```wgsl
@group(0) @binding(0) var source: texture_2d<f32>;

@group(0) @binding(1) var<storage, read> codebook: array<u32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32)
    -> @builtin(position) vec4f {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    return vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let encoded = textureLoad(source, vec2i(position.xy), 0);
    let bytes = vec4u(round(encoded * 255.0));
    let r = codebook[bytes.r >> 3u];
    let g = codebook[32u + (bytes.g >> 2u)];
    let b = codebook[bytes.b >> 3u];
    return vec4f(vec4u(r, g, b, bytes.a)) / 255.0;
}
```

The production capture is the required proof that the normalized write into the
unorm intermediate produces the exact intended bytes on the active adapter.

### App ownership and render wrapper

Add to `AppState`:

```rust
pub(crate) shell_surface_presenter:
    crate::render::shell_surface_present::ShellSurfacePresenter,
```

Construct it immediately after `GpuContext`, batch/depth setup, and before the
final `AppState` literal. Recreate its size-dependent resources in
`resize_surface_for_window_size` after `gpu.resize`.

Change only the normal wrapper signature:

```rust
pub(crate) fn render_main_menu_shell(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    destination: &wgpu::Texture,
) -> Result<MainMenuShellRenderResult>
```

Implementation:

1. clone `state.shell_surface_presenter.source_render_view()`;
2. clone the ordinary depth view;
3. call `render_main_menu_shell_to_target` with those views;
4. when the result is `Rendered`, call `encode_present` into `destination`;
5. when it is `Fallback`, skip the copy;
6. return the original result.

At the app callsite, pass `&output.texture` instead of `&view`. The acquired
ordinary `view` remains unchanged for egui, all other render routes, and
frame-readback ordering.

### Profile evidence tool

`tools/shell_certification/presentation_profile.py`:

```python
PROFILE_SCHEMA_VERSION = "vera20k.shell-presentation-profile.v1"

def derive_channel_codebooks(
    frame: bytes, width: int, height: int
) -> tuple[tuple[int, ...], tuple[int, ...], tuple[int, ...], tuple[int, ...]]:
    """Return sorted B, G, R, A value sets after exact BGRA length validation."""

def derive_presentation_profile(
    guard_path: Path, oracle_runs: Path
) -> dict[str, object]:
    """Validate the sealed guard and every guarded source frame, then report."""

def write_presentation_profile(
    guard_path: Path, oracle_runs: Path, output: Path
) -> dict[str, object]:
    """Write canonical JSON once; reject existing output or link targets."""
```

The guarded integration path:

1. calls the existing exact-hash `validate_guard`;
2. requires exactly three source entries;
3. resolves each source only beneath
   `oracle_runs / source["run_id"] / source["surface_blob"]`;
4. rejects links, traversal, non-files, changed-during-read content, wrong byte
   length, or pixel SHA mismatch;
5. derives BGRA value sets and requires alpha `(255,)`;
6. requires all sources to have the same B/G/R value sets;
7. requires cardinalities 32/64/32;
8. requires B and R tables to be identical;
9. records the guard SHA, environment identity hash, exact source run/frame
   hashes, surface format, tables, and cardinalities;
10. never edits the guard or source runs and never overwrites output.

Add CLI:

```text
python -m tools.shell_certification derive-presentation-profile
  --guard <sealed-guard>
  --oracle-runs <oracle-runs-root>
  --output <new-json>
```

Tool errors exit 2 through the existing CLI error path.

## Tasks

### Task 1: Correct verified-stale research

Edit only the four named documents:

1. In `DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS_GHIDRA_REPORT.md`, change every
   constructor-entry claim from `0x004BA900` to `0x004BA770` and explicitly
   record `0x004BA900` as an interior address.
2. In `DIRECTDRAW_LIVE_PIXEL_FORMAT_RUNTIME_SAMPLE_GHIDRA_REPORT.md`, change
   the current log name to `DDrawCompat-gamemd.log` and replace the stale
   `game.exe` process claim with the observed `gamemd.exe`; keep the unavailable
   direct in-process global sample marked unavailable.
3. In `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`, replace the claim that 16-bit
   packing is not a parity constraint with the locally verified RGB565
   final-pixel constraint, scoped to this enrolled presentation.
4. In `PIXEL_FX_SPARKLES_GHIDRA_REPORT.md`, replace "RGB565 still open/no Rust
   analogue" with the current log/native-format evidence and preserve the
   general runtime-derived requirement.

Then run the research-index reindex/validation path only after checking that no
other session owns it. Validate the affected shell/pixel-format topics and
inspect literal errors.

Stop condition: corrected claims are internally consistent, current addresses
and filenames are literal, and index validation reports no stale link caused by
these edits.

### Task 2: Add the shared native format/profile module

1. Add `native_surface_format.rs` with the interfaces above.
2. Move only the format type out of `tactical_compat.rs`; keep packing and
   tactical behavior in place.
3. Re-export the type from `tactical_compat` for downstream compatibility.
4. Add exhaustive 0..255 tests:
   - RGB565 indices are `channel >> {3,2,3}`;
   - RGB555 indices are `channel >> 3`;
   - known native masks/shifts pack expected fixture pixels;
   - profile expansion exactly equals the observed 32/64-entry arrays;
   - alpha is preserved;
   - all 256 input values land in the expected cardinality.

Stop condition: no consumer needs tactical ownership to describe a native
surface, and the current `pack_rgb` public behavior is unchanged.

### Task 3: Add machine-generated presentation-profile evidence

1. Implement the pure value-set helper.
2. Implement guarded source resolution and immutable stable reads using the
   existing shell-certification safety helpers where available.
3. Add CLI parsing and one-shot canonical JSON output.
4. Add synthetic tests for:
   - exact 32/64/32 and alpha-255 success;
   - wrong frame length;
   - non-255 alpha;
   - mismatched source codebooks;
   - wrong cardinality;
   - B/R disagreement;
   - path escape/link rejection;
   - existing output rejection.
5. Document the command and its evidence limits.

Stop condition: the tool can independently reproduce the exact profile from
all guarded source frames without touching Oracle evidence.

### Task 4: Implement the byte-safe shell presenter

1. Add the renderer module and WGSL.
2. Validate the selected surface format is an sRGB/unorm-compatible pair.
3. Create source/presented textures and exact explicit views.
4. Upload the Rust profile tables into the read-only shader buffer.
5. Use `textureLoad`, integer table indices, opaque overwrite, and no sampler.
6. Encode the fullscreen render followed by a whole-extent texture copy.
7. Recreate all size-dependent texture/view/bind-group resources on resize;
   preserve the pipeline/profile buffer when their formats are unchanged.
8. Add debug assertions or stored dimensions so an encode with stale size
   cannot silently copy a partial frame.

Stop condition: the presenter owns a complete encoded-byte path and exposes no
global postprocess hook.

### Task 5: Integrate only the normal `0xE2` wrapper

1. Add the presenter to `AppState`.
2. Construct it during app initialization.
3. Resize it next to the depth texture.
4. Add `COPY_DST` to surface configuration after checking
   `SurfaceCapabilities.usages`.
5. Change the wrapper/callsite to pass `&output.texture`.
6. Copy only on `Rendered`; preserve the existing egui fallback on `Fallback`.
7. Change the main-menu first-paint dispatcher from the wrapper to the existing
   `render_main_menu_shell_to_target` function with its current color/depth
   bundle; leave all other transition behavior unchanged.
8. Confirm no single-player, skirmish, loading, tactical, or egui callsite
   imports the presenter.

Stop condition: the normal main-menu production route is quantized before egui
and readback, while first-paint/offscreen transition and neighboring routes are
structurally unchanged.

### Task 6: Static review and non-Cargo validation

While the external Cargo/dev lease remains active:

1. format only edited Rust files with `rustfmt --edition 2024` if direct
   `rustfmt` does not invoke Cargo;
2. run Python shell-certification unit tests;
3. run research-index and System Map validation if their mutation lanes are
   free;
4. inspect `git diff --check`, `git diff --stat`, and the complete patch;
5. verify the worktree has no unrelated paths;
6. update the root crash-safe goal journal with branch SHA, exact evidence
   level, leases, and remaining validation.

Stop condition: the feature diff is coherent and reviewable without claiming a
Rust build result.

### Task 7: Cargo/build validation after explicit release

Only after the current HUD owner explicitly releases Cargo/dev:

1. recheck every worktree, branch, process, and current dev SHA;
2. reconcile current dev into the feature branch without touching dirty root
   files;
3. check `Get-Process cargo,rustc,gamemd,vera20k`;
4. run focused Rust tests serially:
   - native surface format;
   - tactical compatibility;
   - shell presenter construction/integration tests;
   - main-menu shell render tests;
   - frame readback tests;
5. report literal `test result:` lines;
6. run `cargo check -q`;
7. build the production binary once;
8. smoke non-shell startup, first-paint transition, loading startup, and first
   tactical frame with retail assets.

Stop condition: no build/test regression and no neighboring route is visibly
altered by the shell-only seam.

### Task 8: Native-vs-Rust differential

After a fresh Oracle/capture lease check:

1. generate a new profile JSON from the sealed guard's three sources;
2. run two independent hidden Rust captures at 800x600 with neutral cursor;
3. verify capture manifests, config/executable hashes, frame sizes, and
   quiescence;
4. require the two logical Rust frame hashes to agree;
5. compare each capture with the existing guard-aware comparator;
6. require exact `MATCH` for Single Player, Options, and Exit at zero shift;
7. retain title as literal `DRIFT` with counts/hashes;
8. inspect the whole frame and Bink area for newly introduced drift;
9. preserve new evidence in unique run directories.

Any guard, identity, display, cursor, process, capture-health, quiescence, or
unrelated-input failure aborts the operation, cleans up only owned processes,
preserves diagnostics, and returns to safe work.

Stop condition: both independent production captures reproduce the three exact
button-region matches. Without this, the checkpoint remains UNVERIFIED.

### Task 9: Review, commit, integrate, and hand off

1. Review the actual diff and literal evidence again.
2. Update only justified GSI-03.01/GSI-02.13 System Map surfaces/edges; otherwise
   record that no truthful schema-valid map change was available.
3. Keep the full steady matrix row DRIFT due the title.
4. Commit coherent milestones on the feature branch.
5. When dev and Cargo are unowned, merge locally into dev; never push.
6. Validate the merged production route and record literal dev/feature SHAs.
7. Release Cargo/dev/Oracle leases and ensure no owned process or merge remains.
8. Update the state journal with exact evidence, residuals, and the next
   unowned checkpoint.
9. Select the next checkpoint by the contract's route priority.

Stop condition: the feature is merged and production-validated, or it is left
as a coherent committed feature with an honest `UNVERIFIED` handoff if an
external lease/capability remains.

## Player-Experience Regression Matrix for Changed Surfaces

The checkpoint directly certifies only the first row's three opaque button
regions. Every other row is a regression revisit, not a parity promotion:

| Route/state | Required result |
|---|---|
| 800x600 `0xE2` steady default | three button crops exact; title DRIFT retained |
| `0xE2` hover/press/release/outside-release | input/state unchanged; no new artifact |
| `0xE2` quit modal/fade | route still works; packed blend remains UNVERIFIED |
| `0xE2` first-paint slide | bypass path unchanged |
| `0x100` entry/back | presentation path untouched |
| `0x102` entry/back/validation | presentation path untouched |
| loading startup | presentation path untouched |
| first tactical frame | presentation path untouched |
| egui fallback and egui overlay | paints after presenter as before |
| resize/reconfigure | no crop, stretch, stale copy, or validation error |
| hidden frame readback | observes final swapchain bytes after shell and egui |

## Risks and Countermeasures

- **Borrow conflict around `AppState`:** clone the WGPU source view first, end
  that field borrow, call the whole-state renderer, then borrow the presenter
  for final encoding.
- **Unavailable `COPY_DST`:** validate `SurfaceCapabilities.usages` before
  configuration and fail with context.
- **Unsupported view pair:** accept only the explicit sRGB formats whose
  `remove_srgb_suffix()` provides a different compatible unorm format.
- **Shader rounding:** capture final swapchain bytes; unit math is not parity
  proof.
- **Fallback copies stale source:** never call `encode_present` on `Fallback`.
- **Unverified state regression:** exercise every neighboring state reached by
  the changed wrapper and keep its parity status honest.
- **Concurrent changes:** no Cargo/dev until explicit release; recheck actual
  paths and SHAs before integration.
- **Hand-edited certification:** profile JSON and comparator reports are
  machine-generated; the full matrix row remains red.

## Self-Review Verdict

Approved for implementation on the feature branch. The plan is grounded in an
exact guarded red comparison, names the actual production owner and return
path, has a byte-safe sRGB strategy, preserves shared route ownership, and
defines a native executable closing test. It does not overclaim title, Bink,
transition, alpha, other-screen, other-resolution, input, or audio parity.

The strongest remaining objection is that a final postpass cannot prove native
packed-domain blend order. That objection does not invalidate the opaque steady
button comparison, but it prevents certification of modal/fade/alpha states and
is retained explicitly.
