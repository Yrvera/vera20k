# Process Startup Splash Design

## Goal

Restore the active Yuri's Revenge process-start `GLS*MD` splash before the first
main-menu frame without coupling it to scenario loading.

## Architecture Context

`App::initialize` in `src/app.rs` already creates the window, GPU, batch renderer,
shell RGB565 presentation boundary, startup `AssetManager`, CSF, and `GAME.FNT`
before it builds menu chrome and enumerates maps. That is the earliest Rust point
where all verified native splash inputs exist together. `App::render_frame` also
has a dormant `startup_splash_until` branch, but the field is initialized to
`None`, never armed, and currently routes to a generic egui loading panel.

The selected-map loading owner is `src/app_loading.rs` and is deliberately outside
this design. The process-start owner will be a private submodule of `app.rs`, so it
does not require editing the currently active `src/lib.rs` or match-loading files.

Primary evidence is
`docs/research/PROCESS_STARTUP_GLS_SPLASH_005312A0_GHIDRA_REPORT.md`.

## Impact Analysis

- `src/app.rs`
  - declares the private startup module;
  - holds the startup presentation object instead of a bare deadline;
  - presents once during initialization, then holds only any remaining deadline;
  - suppresses gameplay/menu input while the splash is active.
- `src/app_startup_splash.rs` (new)
  - owns retail asset selection, CPU composition, GPU texture, presentation, and
    deadline state.

No INI, simulation, map loading, scenario-loading compositor, main-menu chrome,
or save/replay state changes. The only runtime risk is acquiring/presenting a
surface before `AppState` construction; failure is recoverable by retaining an
unarmed presentation for the first normal redraw.

## Chosen Approach

Compose one complete native-size RGBA frame on the CPU from retail SHP/PAL/FNT/CSF
data, upload it as one nearest-filtered batch texture, and present it through the
existing shell RGB565 boundary.

This keeps exact coordinate and draw-order logic in a pure, testable compositor,
uses existing asset parsers and presentation infrastructure, and avoids introducing
egui DPI/layout behavior into a native-pixel screen. The completed texture remains
owned by a small startup presentation state until its deadline expires.

The first presentation is attempted immediately after startup CSF and `GAME.FNT`
load, before later menu/map initialization. The deadline is anchored only after a
successful present. Initialization work consumes that interval; the normal render
loop holds the frame only for any remaining time.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — exact asset selector: width `== 640` uses
  `GLSSMD.SHP`; every other width uses `GLSLMD.SHP`.
  [GHIDRA `0x00531369..0x00531381`]
- `MILESTONE-BLOCKING` — `GLSMD.PAL` uses per-channel byte `<< 2`, not VGA
  rescaling. [GHIDRA `0x005312EC..0x0053132F`]
- `MILESTONE-BLOCKING` — SHP frame 0 is centered at native size with signed
  truncation toward zero; no stretching. [GHIDRA `0x005313C6..0x0053140E`]
- `MILESTONE-BLOCKING` — draw order is background, copyright, brand,
  `GUI:LoadingEx`, trademark top, trademark bottom.
  [GHIDRA `0x00531413..0x005315D2`]
- `MILESTONE-BLOCKING` — `Loading...` starts at `(10,10)` using `GAME.FNT`;
  bottom lines use `height-40` and `height-20` with retail 17px line height.
  [GHIDRA `0x005314F6..0x005315D2`]
- `MILESTONE-BLOCKING` — the completed frame is visibly presented before later
  startup work and held to a timestamp-plus-5000ms boundary.
  [GHIDRA `0x005315D7..0x005315FD`, `Init_Game @ 0x0052BA60`]
- `COMPOUNDING` — process-start state is separate from selected-map loading, so
  startup timing cannot mutate match-loading milestones.
  [doc: `PROCESS_STARTUP_GLS_SPLASH_005312A0_GHIDRA_REPORT.md` §6]
- `EXACTIFICATION-RESIDUAL` — the installed EA archive lacks the 640-specific
  `GLSSMD.SHP`; its filename/selector are verified but its retail metadata is not.
  Trigger: launching at exactly 640px with an install lacking that asset. Effect:
  black background with verified text. Ordinary 800x600 startup is unaffected.
- `EXACTIFICATION-RESIDUAL` — native display-chain callbacks include a 50ms sleep.
  Rust uses its existing synchronous shell presentation commit and anchors the
  five-second deadline after that commit; it does not add a blocking 50ms sleep.
  This is non-compounding and not perceptible against the five-second hold.

## Design

### Components

`StartupSplashPresentation`

- owns the complete `BatchTexture`;
- owns its single full-client sprite instance buffer;
- owns `Option<Instant>` deadline;
- becomes active immediately and records the deadline only after first present.

Pure compositor helpers

- select the SHP filename from logical width;
- parse `GLSMD.PAL`, SHP frame 0, and `GAME.FNT`;
- center and clipped-blit the frame onto an opaque black client canvas;
- resolve CSF values with retail English fallbacks;
- measure and rasterize the five text layers.

Presenter

- clears the existing shell source surface;
- draws one native-pixel quad through the UI camera;
- invokes `ShellSurfacePresenter::encode_present`;
- submits and calls swapchain `present`.

### Interfaces / Contracts

- `build(...) -> Result<StartupSplashPresentation>` requires retail assets, parsed
  FNT, client dimensions, GPU, and batch renderer.
- Missing SHP or palette is not fatal: composition remains black plus text.
- Missing/unparseable `GAME.FNT` means the startup splash is omitted with a warning;
  the degraded application remains usable.
- `present(...) -> Result<()>` has no menu or loading-session side effects.
- `mark_presented(now)` is idempotent and anchors `now + 5000ms` only once.

### Data Flow

```text
AssetManager + CSF + FNT
  -> pure CPU compositor
  -> nearest-filtered full-client BatchTexture
  -> shell RGB565 source/presentation boundary
  -> swapchain present
  -> five-second deadline
  -> remaining App::initialize work
  -> redraw hold if deadline remains
  -> first main-menu frame
```

### Error Handling

- Asset parse failures are logged and reduce to the smallest safe fallback.
- Initial surface acquisition failure does not abort initialization; the same
  presentation is retried by `render_frame`, and the deadline starts on that first
  successful commit.
- Surface errors during the hold propagate through the existing render error path.

### Testing Strategy

- pure tests for exact 640/non-640 asset selection;
- pure tests for signed centered placement and clipping;
- synthetic SHP/PAL/FNT-independent pixel-blit tests;
- deadline tests proving it is unarmed before present, starts once, and expires;
- scoped `--lib` tests for the new module;
- `cargo check` after implementation;
- user performs the real visual runtime check, as requested.

## Architectural Decisions

- Follow the existing startup `AssetManager`, batch texture, UI camera, and shell
  presentation patterns.
- Use a dedicated process-start module instead of reusing `app_loading`.
- Compose one immutable texture because the native screen is static; rebuilding
  text/SHP every redraw would add complexity without behavior value.
- Keep the exact-640 missing-asset case honest rather than inventing or scaling a
  replacement.

## Alternatives Considered

1. **Reuse the generic egui loading panel.** Rejected: wrong artwork, font,
   coordinates, DPI behavior, palette, and ownership.
2. **Add layers to the scenario-loading compositor.** Rejected: creates lifecycle
   coupling and risks match-loading progress regressions.
3. **Embed a converted PNG.** Rejected: ignores retail archive overrides,
   localization, palette conversion, and the verified SHP/FNT authority.

## Approval Record

The user approved the bounded design after receiving the exact implementation
scope: dedicated retail asset/text composition, pre-menu presentation, five-second
hold, generic placeholder retirement, and strict separation from scenario loading.

## Runtime Repair Addendum

The first runtime launch exposed a WGPU validation contract that compilation and
pure compositor tests could not exercise: `Batch Pipeline (Overlay Passthrough)`
declares a `Depth32Float` attachment even though it compares `Always` and does not
write depth. The startup splash render pass omitted that attachment and therefore
panicked on every normal launch before its first command buffer could submit.

The approved repair follows the existing main-menu and skirmish-shell pattern:
thread the already-created application depth view into the splash presenter and
attach it to the composition pass. This changes no splash assets, pixels, timing,
surface-presentation ordering, INI authority, or selected-map loading behavior.
Allocating a transient depth texture per splash frame and adding a second
depthless batch pipeline were rejected as unnecessary duplication and renderer
surface-area growth, respectively.

Repair validation adds a real process launch held beyond the five-second splash
deadline, because a compile-only check cannot detect render-pass compatibility.
