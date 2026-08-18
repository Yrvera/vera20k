# Stock Skirmish Loading Presentation Design

## Goal

Make the ordinary 800×600 stock Skirmish loading transition visibly closer to
retail by preserving PROGBARM shading and presenting the verified 3%, 90%, and
100% selected-map boundaries before tactical takeover, while ensuring `.SED`
random-map loads use the native halved progress domain rather than inheriting
selected-map percentages unchanged.

## Architecture Context

`AppState.loading_session` is the sole app-level owner from accepted Start
until `app_transitions::apply_map_load_result` installs the tactical state.
`NativeLoadingScreenState` owns the selected country art, player color,
progress state, and session-local GPU atlas. `app_init` emits load-phase
milestones through `LoadingProgressSink`; `RenderingProgressSink` already
performs synchronous full-screen presentations for advancing values.

The selected loop is `LOOP-001-SKIRMISH-LAUNCH`; this slice owns only its
`GSI-03.09` loading stage and does not create simulation or launch authority.

## Impact Analysis

Changed production surfaces:

- `src/render/loading_screen_chrome.rs`: decode `PROGBARM.SHP` using a copy of
  `MPLS.PAL` whose indices 16–31 contain the selected player's existing
  `HouseColorRamps` band.
- `src/app_loading.rs`: pass that ramp at atlas creation, keep the 0% LS
  composition off-screen, include 3% in the first confirmed display blit, and
  route 100% through the existing synchronous presenter. Classify `.SED`
  sessions once and convert raw random-map callbacks with integer halving,
  including terminal raw 200 → effective 100.
- `src/app_init.rs`: preserve the verified theater-finalization 25% boundary
  before 30%, and emit the verified 90% beacon-art boundary between 86% and 93%.
- Bounded prerequisite `src/rules/house_colors.rs`: correct the shared native
  schedule ownership before consuming it here—sine/S uses 50°→90°; cosine/V
  uses 20°→90° with the shade-0 π/16 override.

No simulation, map parsing, startup acceptance, RNG, input, audio, country
selection, side-icon mapping, tactical rendering, or shared palette authority
changes. The atlas remains loading-session-local.

## Chosen Approach

Use the existing Rust-native loading owners and presentation sink:

1. Copy the chosen 16-color ramp into the progress palette before decoding
   frame 0. Draw the resulting atlas entry without a uniform tint.
2. Compose the native loading frame, advance it to 3%, and let the outer render
   submit/present that combined result once. Do not visibly present a separate
   zero-progress frame.
3. Emit the verified 25% theater-finalization boundary. Keep the native dynamic
   internal 13–24 steps as a residual because Rust's monolithic theater loader
   exposes no equivalent iteration boundary; do not fake every integer.
4. Emit 90% at the already named end-load beacon-art boundary.
5. On successful load completion, advance to 100% and present before returning
   `LoadingPump::Finished`.
6. At the internal 0% compose state, emit only the country background. Add
   backing, clipped bar, and side icon for the first confirmed 3% display.
7. Derive one immutable progress cadence from the selected filename. Normal
   maps retain raw values; `.SED` seed loads use `raw / 2` and raw 200 as the
   terminal request. Do not invent the unresolved generator-specific sequence.

This is preferred over a new render path because the current synchronous sink
already owns duplicate suppression, surface acquisition, submission, and
presentation. It is preferred over canned frames because the real loader
continues to own progress.

## Player-Experience Detail Ledger

- **COMPOUNDING:** uniform-tint PROGBARM appears on every ordinary load and
  erases the retail asset's shading. Fixed in this slice.
- **COMPOUNDING:** every load skips the first and final visible progress
  boundaries. Fixed in this slice.
- **EXACTIFICATION-RESIDUAL:** Rust `f64` ramp construction and current
  truncation approximate native lookup-table/x87 behavior. The schedule
  ownership is corrected, but the exact channel-byte bound is not claimed
  because live constructor `ftol` rounding remains UNCHECKED.
- **UNCHECKED:** native x87 rounding control at the 3% bar width is not yet
  runtime-pinned. At a 326-pixel frame it may distinguish 9 from 10 pixels, so
  the timing/value is verified but the exact 3% pixel width is not claimed.
- **UNKNOWN-RISK:** no native/Rust loading-frame capture pair exists. The
  production composition and lifecycle are testable non-interactively, but
  final visual judgment remains unverified.
- **MILESTONE-BLOCKING CANDIDATE, EVIDENCE-BLOCKED:** assigned-player markers
  and two text layers are absent. Their remaining placement/content facts are
  not guessed in this slice.
- **EXACTIFICATION-RESIDUAL:** larger-than-art centering is outside the normal
  fixed 800×600 start and remains unchanged.
- **MILESTONE-RESIDUAL:** `.SED` loads now use the verified halved value domain,
  but Rust does not yet emit the full native generator-specific raw sequence.
- **DEVELOPMENT-ONLY RESIDUAL:** `RA2_QUICKPLAY=<seed>.sed` can override a
  non-SED request after session construction, so that diagnostic-only route can
  retain selected-map cadence. Ordinary UI-launched stock random maps preserve
  the `.SED` filename and are covered by this slice.

## Design

### Components

- `LoadingScreenAtlas`: unchanged ownership; its progress entry is decoded
  with the selected ramp.
- `NativeLoadingScreenState`: retains backing color and color index; the
  separate flat `bar_rgb` field is removed because the atlas now contains the
  correct per-pixel colors. It also retains an immutable selected-map versus
  random-map progress cadence derived at session construction.
- `present_native_loading`: remains the single synchronous loading presenter.
- `RenderingProgressSink`: remains the single later-milestone owner.

### Interfaces / Contracts

- `build_loading_screen_atlas(..., progress_ramp)` requires exactly 16 opaque
  palette colors and copies them into indices 16–31.
- `build_native_loading_instances` returns background only for the internal 0%
  compose state; for an advancing positive milestone it adds backing, clipped
  frame-0 span, and the current side icon in existing vector order.
- The outer first loading render owns the 3% display. Advancing 100% is stateful
  only if presentation is attempted through the synchronous presenter. A
  surface failure is logged and does not turn a successful map load into a
  failure, matching the existing later-milestone policy.
- Every raw callback passes through the session cadence before the monotonic
  gate. Selected maps are identity-mapped; `.SED` loads truncate `raw / 2` and
  finish with raw 200.

### Data Flow

```text
accepted local color priority
→ [Colors] entry index
→ RuleSet.house_color_ramps[index]
→ loading atlas palette indices 16..31
→ decoded PROGBARM frame 0
→ clipped instance at current percent
→ synchronous surface present
→ loader continues / tactical handoff
```

### Error Handling

Missing mandatory loading art or palette remains fail-closed during first
renderer setup. Missing side icon remains non-fatal. A synchronous repaint
surface error logs a warning and allows loading to continue, preserving the
current policy.

### Testing Strategy

- Pure palette regression: progress palette indices 16 and 31 equal the
  selected ramp endpoints while unrelated indices remain unchanged.
- Composition regression: the internal 0% state produces background only;
  positive progress produces the established row stack and clips frame 0.
- Milestone regression: the current production ledger contains 90 between 86
  and 93, contains theater finalization 25 before 30, and monotonic duplicate
  suppression remains intact.
- Map-kind regression: `.map` selects identity cadence, case-insensitive `.SED`
  selects halving, and 3/90/200 map to 1/45/100 for random maps.
- One focused Rust test command for loading modules, then one
  `cargo check -q -p vera20k`.
- No desktop focus, injected input, native launch, or PR/push.

## Architectural Decisions

The design follows the existing app-level loading-session owner, asset decode
pipeline, `HouseColorRamps`, and synchronous presentation pattern. It adds no
duplicate progress manager and does not move gameplay state across layers.

The feature is retail-convincing, not pixel-certified. No new technical debt is
introduced; known marker/text evidence gaps remain explicit.

## Alternatives Considered

- **Full first-render closure with markers and text:** rejected for this slice
  because final marker placement and localized runtime text are unresolved.
- **Uniform tint with a better single color:** rejected because it still
  destroys the verified 16-shade asset structure.
- **Smooth/interpolated progress or canned timed frames:** rejected because
  retail uses discrete real load callbacks and duplicate suppression.
- **New offscreen/capture-specific renderer:** rejected because it would not
  fix the production player path.

## Autonomous Approval

Approved for implementation.

Why it should be approved: every change attaches to an existing production
owner, fixes an every-load visible divergence with current binary-identified
research, preserves the real loader, and can be regression-tested without
desktop control.

Strongest rejection case: a separate 3% surface acquisition could duplicate the
outer frame, the wrong palette could be remapped, or `.SED` could silently reuse
selected-map percentages. The finalized first-render audit resolves the first
issue by putting the initial effective value into the outer frame's single
confirmed display blit; only the terminal value reuses the mid-load presenter.
A palette-band test protects the 16–31 contract, and the filename-kind cadence
test protects the random-map boundary.

Evidence that could still make it wrong: the active research audit could
disprove the standard PROGBARM ColorScheme path or the first/terminal callback
ordering. Integration must stop and this design must be repaired if such a
contradiction arrives before merge.
