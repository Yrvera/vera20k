# Exact Main-Menu 0xE2 RGB565 Presentation Design

**Date:** 2026-07-25
**Checkpoint:** `SLICE-20260725-GSI-03-01-E2-RGB565-PRESENTATION`
**Canonical systems:** `GSI-03.01`, with bounded dependencies on `GSI-02.13`
and `GSI-15.10`
**Status:** SELF-APPROVED
**Branch:** `feature/exact-shell-e2-rgb565-presentation-20260725-2250`
**Worktree:**
`C:/Users/enok/Documents/ra2-rust-game-exact-shell-e2-rgb565-presentation-20260725-2250`
**Validated base:** `5e8b4eaf2c141913b6d6d9a2e984f3602c84407e`

## Goal

Make the production Rust main-menu dialog `0xE2` present its steady opaque
owner-draw button pixels through the same observed 16-bit RGB565 codebook as
the guarded active retail executable, without quantizing unrelated egui,
tactical, loading, or other shell routes.

The decisive checkpoint is deliberately smaller than the complete shell goal:
at 800x600, with the neutral cursor outside the comparison regions and no modal,
fade, or first-paint transition active, the Single Player, Options, and Exit
button crops must byte-match the guarded retail frame after the guard-proven
logical-to-physical point expansion. The title crop is retained as a separate
red residual and is not hidden by this checkpoint.

## Why This Checkpoint Is First

The current Rust production route renders the correct opaque steady button
composition, but its final 8-bit channel values retain colors that active retail
loses when it renders to a 16-bit RGB565 DirectDraw surface. This is persistent,
full-frame-visible drift shared by every main-menu owner-draw button and
therefore outranks isolated title or animation polish.

The verified comparison is decisive:

- guarded native source: sealed 1920x1080 BGRA frame, point-presented from an
  800x600 logical surface;
- Rust source: production 800x600 logical BGRA readback from commit `784cd7b9`;
- observed retail channel codebooks:
  - five-bit: `0, 8, 16, 25, 33, 41, 49, 58, 66, 74, 82, 90, 99, 107, 115,
    123, 132, 140, 148, 156, 164, 173, 181, 189, 197, 206, 214, 222, 230,
    238, 247, 255`;
  - six-bit: `0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 45, 49, 53, 57, 61,
    65, 69, 73, 77, 81, 85, 89, 93, 97, 101, 105, 109, 113, 117, 121, 125,
    129, 133, 138, 142, 146, 150, 154, 158, 162, 166, 170, 174, 178, 182,
    186, 190, 194, 198, 202, 206, 210, 214, 219, 223, 227, 231, 235, 239,
    243, 247, 251, 255`;
- encoded transform:
  `B'=L5[B>>3], G'=L6[G>>2], R'=L5[R>>3], A'=A`;
- exact pixels after the transform:
  - Single Player: `21,075 / 21,075`;
  - Options: `21,356 / 21,356`;
  - Exit: `21,075 / 21,075`;
  - combined: `63,506 / 63,506`;
- ordinary nearest full-range expansion reaches only `61,765 / 63,506`
  (`97.2585%`), so the observed codebook, not an assumed expansion formula, is
  required for this environment.

This codebook is evidence for the enrolled active retail/AMD/DDrawCompat/DXGI
presentation path. It is not claimed as a universal gamemd algorithm.

## Scope

This design owns:

- a shared Rust-native description of native DirectDraw channel losses/shifts;
- an executable-evidence-derived RGB565 presentation profile;
- a shell-only offscreen source and presentation pass;
- the wrapper used by the steady production `0xE2` render path;
- resize/recreation and fail-closed capability handling;
- machine-generated profile evidence and focused regression tests;
- the four directly disproven research statements named in the implementation
  plan.

This design does not own:

- title geometry or its final lowercase-`u` color residual;
- Bink decode internals, Bink alpha/compositing, or movie phase;
- first-paint slide frames or screen-to-screen transition surfaces;
- modal/fade/disabled/translucent packed-domain blend equivalence;
- dialog `0x100`, dialog `0x102`, loading, tactical, or egui presentation;
- 640x480 or 1024x768 certification;
- random-map generator internals;
- the concurrently owned in-game HUD/sidebar prerequisite;
- Oracle guard mutation, Ghidra mutation, or sealed evidence replacement.

## Evidence

### Guarded executable observation

The enrolled shell guard identifies an active Yuri's Revenge `gamemd.exe`
session with:

- logical size 800x600;
- physical capture size 1920x1080;
- content rectangle `[240, 0, 1680, 1080)`;
- point scale 9/5;
- logical sampling:
  `sx=floor((2*(x-240)+1)*5/18)`,
  `sy=floor((2*y+1)*5/18)`;
- BGRA8 full-range G22/P709 capture, HDR off;
- cursor captured as presented;
- three sealed source runs with identical 32-blue, 64-green, 32-red codebooks
  and alpha 255 throughout.

The source frame used for the current red differential is:

`69a15fd903831ea6e82f56b0d717eb80d27e626af92121116c9274e75239b0f1`.

### Live native path

Current live decompilation of the active `gamemd.exe` establishes:

- `DSurface` construction begins at `0x004BA770`;
- it copies the DirectDraw pixel-format descriptor and derives channel shifts
  and losses from the runtime masks;
- the RGB565 branch produces red `(shift 11, loss 3)`, green `(shift 5, loss 2)`,
  and blue `(shift 0, loss 3)`;
- the RGB555 branch is distinct and remains supported by the native code;
- video-mode setup passes 16 bits per pixel;
- the native color-packing helper consumes the derived globals rather than a
  hard-coded surface format;
- main-menu `0xE2` paints a two-byte BSurface and blits it to the alternate
  surface;
- the Bink shell path also targets a DirectDraw surface before the final
  presentation.

The current retail `DDrawCompat-gamemd.log` independently records
`D3DDDIFMT_R5G6B5` for plain, primary, and system-memory surfaces. The retail
DDrawCompat configuration requests the application's 16-bit depth and point
display filtering.

### Production Rust route

The current Rust route:

1. acquires the swapchain texture and its sRGB view;
2. calls `render_main_menu_shell`;
3. the wrapper calls `render_main_menu_shell_to_target`;
4. the shell paints directly into the supplied target;
5. egui may paint afterward;
6. frame readback copies the actual swapchain texture;
7. the queue submits and the frame presents.

The pre-implementation caller recheck found that the first-paint transition
dispatcher still called `render_main_menu_shell` with its own target view. The
implementation must split that caller to
`render_main_menu_shell_to_target` before changing the normal wrapper's
destination type. That explicit split keeps first-paint transition behavior
outside this checkpoint.

## Composition and Interaction Ledger

### Top-level owner and return

- Native dialog owner: main-menu shell dialog `0xE2`.
- Rust owner: the app frame route calling
  `app_main_menu_shell_render::render_main_menu_shell`.
- Return/handoff: encoded commands return to the app frame; egui remains
  eligible to render afterward; frame readback then observes the actual
  swapchain bytes.

### Draw order

The steady Rust owner currently submits:

1. parent background `MNSCRNS.SHP` or `MNSCRNL.SHP`, frame 0, with
   `SHELL.PAL`;
2. Bink panel `Ra2ts_s.bik` or `Ra2ts_l.bik`;
3. shell chrome including `SDTP`, `SDBTNBKGD`, `SDBTM`, and
   `LWSCRNS.SHP`/`LWSCRNL.SHP`;
4. owner-draw button art from `SDBTNANM.SHP`;
5. `GAME.FNT` yellow button text;
6. quit-validation modal, when active;
7. `MOUSE.SHA` cursor frame 0;
8. quit fade, when active.

The presenter runs once after this complete shell draw and before any egui
overlay. It does not reorder individual shell submissions.

### Relevant frames and states

- `SDBTNANM.SHP` frames 0 through 16 remain loaded from retail assets.
- The current steady default button frame is 2.
- Hover uses frame 3.
- Pressed uses frame 4.
- Other ramp/state frames remain driven by existing shell state.
- This checkpoint's native comparison observes the default steady opaque state;
  hover, press, release, outside-release, disabled, fade, and first-paint states
  remain matrix rows requiring later native differential evidence.

### Geometry, clip, anchors, and crops

- Logical surface: 800x600.
- Native content is point-expanded into `[240, 0, 1680, 1080)`.
- Title crop: physical `(1383,16)-(1676,48)`, logical
  `(635,9)-(798,27)`.
- Single Player crop: physical `(1399,358)-(1680,433)`, logical
  `(644,199)-(800,241)`.
- Options crop: physical `(1399,660)-(1680,736)`, logical
  `(644,367)-(800,409)`.
- Exit crop: physical `(1399,963)-(1680,1038)`, logical
  `(644,535)-(800,577)`.
- Neutral cursor: logical `(400,300)`, outside all comparison crops.
- Existing renderer clipping, right-edge anchors, depth values, and scissor
  behavior are unchanged.

### Palette and color conversion

- Asset decoding continues to use normal retail archive precedence.
- Existing palette parsing and SHP atlas construction are unchanged.
- The new pass models only the final observed native surface presentation:
  encoded source bytes are indexed by RGB565 losses, then mapped through the
  guard-derived 5-bit/6-bit codebooks.
- Alpha is preserved byte-for-byte. This is sufficient for the checkpoint's
  opaque final pixels, not proof of packed-domain alpha or blend equivalence.

### Text, cursor, focus, and input

- Text remains `GAME.FNT` with existing CSF strings and existing placement.
- Cursor remains `MOUSE.SHA` frame 0 with its current hotspot and shell depth.
- Button enable flags, hit tests, mouse capture, hover, press/release,
  outside-release behavior, focus, caret, keyboard default/cancel routing, and
  state writes are unchanged.
- No new input path, focus owner, or cursor policy is introduced.

### Audio and state transitions

- Existing music selection and playback continue unchanged.
- Existing UI hover/click/validation sound triggers and ordering continue
  unchanged.
- No transition timer, invalidation rule, repaint cadence, navigation state
  write, or downstream screen handoff changes in this checkpoint.
- First-paint slide rendering deliberately bypasses the presenter until it is
  separately measured and certified.

## Architecture Context

The renderer currently assumes that every main-menu layer can draw directly
into a caller-supplied `TextureView`. `ShellRenderTarget` carries borrowed color
and depth views, which already allows offscreen rendering without changing
individual shell painters. The steady wrapper is the narrow ownership seam:
transition code reaches the lower-level target function directly, while the
normal production route reaches the wrapper.

The swapchain normally uses an sRGB texture format and currently advertises
`RENDER_ATTACHMENT | COPY_SRC`. The byte-domain transformation cannot be
implemented correctly by returning normalized colors from a shader into an sRGB
swapchain target: automatic transfer encoding would alter the intended bytes.
Likewise, sampling an sRGB view would decode the stored bytes to linear values.

WGPU permits compatible sRGB/unorm texture views and byte-preserving
texture-to-texture copies between the corresponding format pair. The design
therefore uses:

1. an offscreen shell source texture in the surface's sRGB format, created with
   the compatible non-sRGB format in `view_formats`;
2. its sRGB view for the existing shell render pipelines;
3. its non-sRGB view for `textureLoad`, so the shader observes the stored bytes
   without sRGB decoding;
4. a non-sRGB intermediate presentation texture;
5. a fullscreen presenter that applies the observed codebooks in encoded-byte
   space;
6. a texture-to-texture copy from the compatible non-sRGB intermediate into
   the actual sRGB swapchain texture;
7. normal egui rendering and frame readback afterward.

The surface adds `COPY_DST` usage. Initialization fails with a clear error when
the selected adapter/surface cannot provide the required format or usage. There
is no silent fallback to the old unquantized path for an otherwise active
stock-shell presentation.

The compatible copy requires the destination `wgpu::Texture`, not merely a
`TextureView`. The steady wrapper therefore accepts the acquired swapchain
texture (and creates or receives its ordinary sRGB view as needed), while
`render_main_menu_shell_to_target` retains its current borrowed-view interface.
The wrapper clones the presenter's offscreen render view before borrowing the
whole `AppState`, renders through the lower-level owner, releases that borrow,
then asks the presenter to encode the quantization pass and copy into the
destination texture. It performs the copy only when the lower-level owner
returns `Rendered`; `Fallback` leaves the destination available for the
existing egui fallback.

## Impact Analysis

### Files and responsibilities

- `src/render/native_surface_format.rs`
  owns the reusable Rust-native DirectDraw shift/loss description, known RGB565
  and RGB555 formats, presentation index helpers, and presentation codebook
  logic.
- `src/render/tactical_compat.rs`
  re-exports the shared format and retains the existing packing helper so
  tactical callers keep a stable API and unchanged packing behavior.
- `src/render/shell_surface_present.rs` and
  `src/render/shell_surface_present.wgsl`
  own the GPU resources, encoded-byte presentation pass, compatible copy, and
  resize recreation.
- `src/render/gpu.rs`
  requests the required surface `COPY_DST` usage and checks support.
- `src/app_main_menu_shell_render.rs`
  changes only the steady wrapper: it requests the presenter's offscreen target,
  renders through the existing lower-level function, and finalizes the copy
  into the acquired swapchain texture. The lower-level transition-facing
  function keeps its current `TextureView` contract.
- `src/app_shell_transition.rs`
  changes its main-menu first-paint branch to call the existing lower-level
  target function explicitly, preserving the transition's unquantized
  offscreen target while the normal wrapper gains swapchain-texture ownership.
- `src/app.rs`
  owns construction and resize lifecycle for the presenter as part of
  `AppState` and passes the acquired `output.texture` to the steady wrapper.
- `src/render/mod.rs`
  exposes the two new renderer modules.
- `tools/shell_certification/**`
  derives a versioned presentation profile from sealed current evidence and
  reports its hashes and channel coverage without modifying the guard.
- four research documents receive narrowly verified corrections.
- `system_map/topology.v2.json` changes only if an existing schema-valid
  GSI-03.01/GSI-02.13 surface or edge can truthfully represent this verified
  connection.

### Shared primitive neighbors

Moving `DirectDrawPixelFormat` affects tactical compatibility tests even though
the shell presenter is the new consumer. Those tests must remain exhaustive for
both RGB565 and RGB555. The new presenter resource touches app initialization
and resize, so main-menu steady render, shell first-paint transition, non-shell
egui rendering, loading, and tactical startup all require smoke validation
after the Cargo/dev lane is released.

The shader and copy are shell-scoped. They must not alter:

- the swapchain format chosen for the rest of the application;
- frame-readback ordering or byte interpretation;
- egui pipeline setup;
- tactical render targets;
- loading-screen presentation;
- offscreen first-paint transition targets.

### Concurrency

The current unrelated HUD owner has the sole Cargo/dev integration lease and
owns narrow SIDENC/sidebar/GCLOCK paths. This feature owns none of those paths.
All implementation, formatting, static inspection, research validation, and
tool tests that do not invoke Cargo stay in the unique feature worktree. Cargo,
dev merge, game launch, and guarded input/capture wait for explicit lease
release.

## Alternatives Considered

### Per-asset palette conversion

Rejected. It would happen before text, Bink, cursor, and blending, cannot
represent final-surface ordering, and would duplicate presentation semantics
across asset types.

### Quantize the whole application swapchain

Rejected. It would incorrectly quantize egui, loading, and tactical frames that
have not been proven to share this shell checkpoint's active native path.

### Direct fullscreen shader into the sRGB swapchain

Rejected. Automatic sRGB encoding would corrupt the desired encoded bytes.

### CPU 16-bit retained compositor

Deferred. It could eventually model native packed-domain blend order, but it is
too broad for the decisive opaque steady comparison and would take ownership of
more shell composition than current evidence requires.

### Chosen approach

Use the shell-only offscreen encoded-byte presenter and compatible byte copy.
It is the smallest reversible seam that matches the observed whole-surface
effect, preserves current composition and interaction ownership, and does not
broaden into unrelated routes.

## Failure and Lifecycle Behavior

- Presenter construction validates format compatibility, non-sRGB counterpart,
  texture usages, and dimensions.
- Unsupported requirements produce an application error with the selected
  format and missing capability; there is no invented art or egui fallback.
- Zero-sized window events retain the existing clamped minimum-size behavior.
- Resize recreates source, intermediate, and any associated bind-group/view
  resources at the configured surface dimensions.
- If shell asset/movie rendering already returns its existing fallback result,
  the wrapper skips the presentation copy, propagates that result, and lets the
  existing egui fallback paint the untouched swapchain destination.
- Device loss and surface reconfiguration continue through the current app
  recovery path; presenter resources are recreated alongside other
  surface-sized resources.

## Verification

### Static and unit checks

- Exhaustive 0..255 channel tests for RGB565 and RGB555 pack/index behavior.
- Exact expected codebook tests for the guard-derived profile.
- Shader/pipeline construction and resize tests where the existing renderer
  test harness supports them.
- Machine-generated profile tests with synthetic sealed-frame fixtures:
  correct 32/64/32 cardinalities, alpha policy, identical-source enforcement,
  source-hash recording, and fail-closed rejection.
- Existing tactical compatibility tests remain unchanged in meaning.
- Existing shell render/readback/certification tests remain green.

### Production differential

After explicit Cargo/dev/Oracle lease release:

1. run focused tests serially, then `cargo check -q` and the production build;
2. capture two independent hidden production Rust steady `0xE2` frames with the
   neutral cursor and retail assets;
3. verify the two Rust logical frame hashes agree;
4. run the existing guard-aware comparator against the same native source;
5. require exact zero-shift matches for all three opaque button regions;
6. require the title region to remain explicitly reported as DRIFT unless
   separately fixed and proven;
7. revisit first-paint transition, egui/non-shell startup, loading startup, and
   tactical startup for regressions;
8. validate any affected System Map entries and update the crash-safe state
   journal with literal results.

The matrix row `visual-state:800x600:main-menu-0xe2:steady-paint` remains DRIFT
while the title is red. Region matches do not promote the whole row to VERIFIED.

## Residuals After This Checkpoint

- Title: moving only yellow title text one logical pixel right raises the native
  match to `9,281 / 9,376`; the final 95 physical pixels (29 logical pixels)
  belong to the lowercase `u`, whose native value is `(25,255,255)` versus the
  transformed Rust `(0,255,255)`. The cause is still unverified.
- Bink phase and packed-surface conversion remain unverified.
- Alpha, translucency, modal, fade, and disabled-control packed-domain behavior
  remain unverified.
- First-paint and screen-to-screen transition frames remain unverified.
- Hover, press, release, outside-release, focus, keyboard, cursor-state, audio,
  and repaint-cadence rows remain unverified.
- Dialogs `0x100` and `0x102`, map browser, loading branches, and tactical
  handoff remain unverified.
- 640x480 and 1024x768 remain unverified.

## Adversarial Self-Review

### Why should this be approved?

The design is bounded by a guarded native differential that changes
`63,506 / 63,506` relevant pixels from red to exact while leaving a known title
residual visible. It follows the complete production wrapper rather than a
helper, preserves existing shell draw/input/audio ownership, uses the native
runtime-derived pixel-format model already needed by tactical compatibility,
and scopes the presentation effect to the one proven shell route.

It also avoids three false proofs: it does not call a Rust-vs-Rust hash parity,
does not treat the observed codebook as universal gamemd behavior, and does not
promote the full steady-paint matrix row while the title remains different.

### What evidence could still make the scoped UI wrong?

1. **Packed-domain order:** Native may quantize before a blend that Rust
   performs before the postpass. This would invalidate modal, fade, alpha, or
   translucent states, so none are certified here.
2. **Environment dependence:** A different enrolled native renderer could use
   another expansion codebook. The profile is therefore tied to exact guard and
   source hashes and must be regenerated from current evidence rather than
   labeled universal.
3. **sRGB byte corruption:** A sampled or rendered sRGB view could change byte
   values. The non-sRGB sampling view, non-sRGB intermediate, compatible byte
   copy, and post-copy readback are mandatory and must be validated by the
   production capture.
4. **Route leakage:** A global swapchain pass could alter egui/loading/tactical
   output. The lower-level transition target remains untouched and only the
   steady `0xE2` wrapper owns this presenter.
5. **Resource lifecycle:** Stale-size textures could crop or stretch after
   resize. Presenter resources must be recreated atomically with the depth and
   surface configuration.
6. **Bink/title ambiguity:** Button success does not prove title or movie
   correctness. Both stay independently red/unverified.
7. **Concurrent integration:** The unrelated HUD slice owns Cargo/dev. This
   feature remains no-Cargo/no-dev until explicit release and will rebase or
   merge current dev only after checking the actual state and owned paths.

All load-bearing objections are either addressed by the architecture or
retained as explicit residuals. The design is therefore self-approved for this
bounded checkpoint, not for complete shell certification.
