# Hidden Tactical Radar Capture Implementation Plan

> Status: provisional-base formal review `APPROVE`, no remaining P0/P1; still
> provisional until final-dev re-anchor. Re-anchor paths, symbols, tests, and
> line references against the exact released `dev` before execution. The active
> exact-shell task currently owns Cargo and `dev` and is validating shared
> launch/capture seams.

**Goal:** Add a hidden, no-input, self-terminating production checkpoint that
drives a fixed Soviet or Yuri Battle session through real MCV deployment,
power/refinery/radar production and placement, live radar authority, ordinary
tactical rendering, and final BGRA8 swapchain capture.

**Parent:** Suspended `GSI-13.23` ordinary radar/minimap presentation. This plan
implements only the smallest autonomous-validation prerequisite. It does not
correct radar assets, geometry, transitions, or minimap composition.

**Architecture:** Keep the existing shell-capture v2 contract sealed. Route a
separate strict tactical-capture v1 session through the same application,
accepted startup, simulation commands, tactical renderer, and final swapchain.
Share only narrow launch/hidden-window/readback lifecycle code. The scripted
core is a pure observation/action state machine; an app adapter owns startup,
command scheduling, exact stepping, post-render evidence, and immutable output.

**Truth boundary:** A `VALID` capture proves that one pinned Rust production
route completed. Same-profile byte-identical repeats are a regression ratchet,
not native pixel or timing parity. Known pre-parent faction-radar mismatch stays
`DRIFT` outside the tactical validator.

## Evidence and current anchors

- Approved design:
  `docs/plans/2026-07-26-hidden-tactical-radar-capture-design.md`.
- Current observed coordination base:
  `dev@d4882018dee558003e3d85835386c8f057b4c6d0`; this is not an execution
  base until the shell owner releases and the plan is re-anchored.
- System Map:
  `GSI-13.23`, `LOOP-008-REVEAL-RADAR`,
  `LOOP-012-POWER-OUTAGE-RECOVERY`.
- Verified native research:
  - `docs/research/SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`
  - `docs/research/RADAR_TRANSITION_CLOSE_OPEN_ASSET_LIFECYCLE_GHIDRA_REPORT.md`
  - `docs/research/SOVIET_RADAR_LEFT_PANEL_SHP_SELECTORS_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/RADAR_VIEWPORT_RECT_CAMERA_WINDOW_OVERLAY_GHIDRA_REPORT.md`
- Retail fixture:
  - archive `multimd.mix`, length `31,264,268`, SHA-256
    `FF4138BA95F7EFD8BDED14342FC9082B99C47E43C25AB18236E4EEA141B488E9`;
  - entry `Fight.MAP`, MIX ID `0x9306F050`, length `91,254`, SHA-256
    `D751DCE7CD3611077E9228C33235F39C71681FFF6AC08CA1F716D963AD6CE070`;
  - Battle descriptor ID `1`; zero-based `ScenIndex=12` is catalog provenance,
    not a launch-session field;
  - local start `Position(0)`, AI start `Position(1)`.
- Current Rust anchors to re-check:
  - launch: `src/main.rs`, `src/app_shell_capture.rs`;
  - accepted startup: `src/skirmish_launch.rs`, `src/match_bootstrap.rs`,
    `src/app_loading.rs`, `src/app_transitions.rs`;
  - hidden lifecycle/render/readback: `src/app.rs`,
    `src/render/frame_readback.rs`, `src/render/gpu.rs`,
    `src/render/egui_integration.rs`;
  - fixed runtime: `src/app_sim_tick.rs`;
  - commands: `src/app_commands.rs`, `src/sim/command.rs`,
    `src/sim/world/mod.rs`, `src/sim/world/world_commands.rs`;
  - production/placement: `src/sim/production/production_queue.rs`,
    `src/sim/production/production_placement.rs`;
  - radar/render: `src/app_building_anim.rs`,
    `src/app_sidebar_render.rs`, `src/app_render/**`,
    `src/render/radar_anim.rs`, `src/render/sidebar_chrome.rs`.
- Current-head shell re-anchor at `d4882018`:
  - shell schema is `vera20k.shell-capture.v2`;
  - `render_frame` now carries opaque main-menu entry/title receipts and commits
    them only after `output.present()`;
  - collapsed 0xE2 route invalidation is owned by
    `app_shell_transition::invalidate_main_menu_dialog_instance`;
  - tactical integration must leave both receipt lanes byte-for-byte
    behaviorally unchanged and must not generalize their present-commit logic.

## Grounding and confidence

- **HIGH:** accepted Battle classifier inputs, retail archive/map identity,
  country/side/structure identities, two-attempt MCV facing behavior,
  30-tick `BuildingUp` gates, production/placement authority, 45 Hz integer
  tick convention, and current Allied radar-animation construction seam are
  directly grounded in current source and retail INIs.
- **HIGH:** `unit_count=0` is a stock-supported setup value and removes
  combat-capable starting units while retaining both houses and MCVs.
- **HIGH code-derived:** at 800x600 the app UI scale is `0.5`, the scaled
  sidebar width is `84`, and the map-load reset cursor is `(358,300)`.
- **MEDIUM pending production run:** the chosen ordered placement radius, final
  BGRA8 surface variant, actual adapter identity, actual window scale factor,
  final egui `pixels_per_point`, and Soviet free-harvester clearance. Record
  these from the hidden production route and require same-profile repeats to
  agree exactly; do not guess or silently relax them.
- **DECLARED PROVENANCE:** the `Fight.MAP` entry SHA is independently audited
  but not recomputed by tactical v1. Runtime evidence is the pinned whole
  archive hash plus the actual carried source/entry ID/payload length.
- **DEFERRED:** native pixel reference, visible pacing/input, AI one-attempt MCV
  residual, ordinary combat, and wider maps/resolutions. None are implied by a
  green checkpoint.

## Hard execution preconditions

1. Receive the exact-shell owner's explicit Cargo and `dev` release plus exact
   merged `dev` SHA.
2. Reconcile all worktrees/branches, root and feature diffs, live tasks,
   Cargo/rustc/game processes, Git operation state, and Ghidra identity.
3. Coordinate with the mechanism-block System Map tooling owner; incorporate
   any already-reviewed merge before branching, or keep its claimed paths
   disjoint and serialize later integration.
4. Run:

   ```powershell
   python -m tools.system_map check --require-sources
   ```

   Require exit `0`; record literal counts/diagnostics.
5. Re-read every current Rust anchor above and `git log -n 12 -- <path>` for
   shared seams. Repair this plan if exact-shell changed an assumption.
6. Repeat the changed-anchor portion of `/review-plan` against final `dev`;
   repair any new load-bearing finding. The provisional-base review is
   `APPROVE` after repair, with no remaining P0/P1.
7. Claim only the bounded tactical prerequisite plus parent `GSI-13.23`, global
   Cargo, and `dev` integration. Create one uniquely named
   `feature/hidden-tactical-radar-capture-20260726-*` worktree from exact current
   `dev`. Keep root `dev` integration-only and preserve the user's `README.md`.

## Task 1: Seal the tactical profile and launch boundary

**Files**

- Create `src/app_launch.rs`.
- Create `src/app_tactical_capture/mod.rs`.
- Create bounded `src/app_tactical_capture/profile.rs`,
  `script.rs`, `placement.rs`, `evidence.rs`, `manifest.rs`, and `session.rs`.
- Create `src/app_tactical_capture_contract.v1.json`.
- Modify `src/lib.rs`.
- Modify `src/main.rs`.
- Tests beside the new Rust modules.

**Red tests**

- No arguments still select interactive launch.
- Every currently accepted shell argv parses to the same
  `app_shell_capture::AppLaunchMode` and request values.
- Every current shell error case retains its strict rejection.
- Tactical accepts only:

  ```text
  --tactical-capture radar-online-v1
  --profile <absolute regular non-link JSON path>
  --contract <absolute regular non-link JSON path>
  --output <absolute nonexistent child-output path>
  ```

- Tactical rejects mixed shell/tactical flags, duplicate flags, unknown flags,
  missing values, non-UTF-8 option names, relative paths, links/reparse points,
  existing output, unsupported checkpoint, malformed JSON, duplicate JSON
  keys, nonfinite values, booleans where integers are expected, unknown schema
  keys, and unsupported dimensions.
- Tactical environment validation rejects every known `RA2_*` override through
  one versioned contract consumed by both Rust and Python. The initial exact
  denylist is:
  `RA2_QUICKPLAY`, `RA2_DEV_SKIRMISH_SHELL`,
  `RA2_DEBUG_SPAWN_UNITS`, `RA2_DISABLE_LAT`, `RA2_ENABLE_LAT`,
  `RA2_DEBUG_CAMEO_PALETTES`, `RA2_DEBUG_BRIDGE_RENDER_BUCKETS`,
  `RA2_FORCE_TIB3_TO_TIB01`, `RA2_TIB_ID_OFFSET`,
  `RA2_FORCE_TIB_IMAGE`, `RA2_DEBUG_MOUSE_CURSOR_SHEET`,
  `RA2_NORMAL_COUNT`, `RA2_NORMALS`, `RA2_QUEUE_FRAME_MS`, and `RA2_DIR`.

**Implementation**

- `app_launch::AppLaunchMode` owns only top-level routing:
  `Interactive`, `ShellCapture`, `TacticalCapture`.
- For no args and all non-tactical argv, delegate the complete argv unchanged
  to the existing sealed shell parser, then convert its result. Do not copy or
  relax shell parsing.
- `TacticalCaptureProfile` and nested serde types use
  `#[serde(deny_unknown_fields)]`, exact schema/checkpoint strings, bounded
  integers, and explicit fixed values.
- Track two profiles under
  `tools/tactical_certification/profiles/`:
  - Soviet local Russia / Yuri Easy AI;
  - Yuri local Yuri / Russia Easy AI.
- Both profiles pin:
  800x600, seed `0x12345678`, Battle ID `1`, `Fight.MAP`,
  stored speed `1`, credits `10000`, unit count `0`, tech level `10`, default
  AI difficulty `0`, every one of the 13 boolean `SkirmishLaunchOptions`
  fields at its named stock value, input delay `2`, Gold local at start 0, Red
  Easy AI at start 1, and no teams/random fields. `ScenIndex=12` is declared
  catalog provenance only and is never copied into persistence or the launch
  session.
- Pin local names exactly: `VERA-SOVIET` for the Russia-local profile and
  `VERA-YURI` for the Yuri-local profile.
- Both profiles pin `placement_radius=16`, `warm_frames=16`, per-stage tick
  caps `(48,640,48,2048,48,1024,48,96,18)`, matching per-stage wall caps
  `(15,90,15,270,15,140,15,20,10)` seconds,
  `overall_tick_cap=4096`, `post_l0_timeout_seconds=600`,
  `child_timeout_seconds=720`, and absolute allowed tactical timeout `900`.
- Do not serialize inactive `RA2MD.INI` slot mirrors that the explicit launch
  path does not consume. Add a test that arbitrary persisted skirmish values
  cannot alter the constructed session or Rust-L0 receipt.
- Pin capture-affecting runtime configuration: `upscale=false`, internal render
  and output extent `800x600`, `extra_animations=true`, app UI scale `0.5`,
  post-load cursor `(358,300)`, required one-frame `CursorId::Default`,
  accepted BGRA8 surface format, input delay, and every pixel-affecting option
  currently read by the app.
- Reapply and validate the profile cursor only after map transition, because
  `apply_map_load_result` resets it. Keep it inside the non-scroll tactical
  interior and require the one-frame `CursorId::Default`; do not hide the cursor
  or override cursor rendering.
- Assert the derivation in tests:
  `auto_detect_ui_scale(800,600) == 0.5`, stock sidebar width `168 * 0.5 == 84`,
  and tactical-interior center `((800 - 84) / 2, 600 / 2) == (358,300)`.
- Pin and hash before/after the child the actually selected external pixel
  inputs: `C:\Windows\Fonts\verdana.ttf` (length `243,304`, SHA-256
  `6A8481FE107EE547893C018B13DBA291C2020BEC3DE5DA6525D9AC09F6BC2105`)
  and `src/sidebar/sidebar_layout.ron` (length `721`, SHA-256
  `27FE2405990000468B1D6B9F4316D8B6104D72C82BB3386A9942332BA323316C`).
  The child records the selected font identity and resolved layout; fallback is
  `INVALID`, not an unrecorded pixel change.
- Record why `unit_count=0` is intentional and stock-valid: it isolates the
  long radar producer chain from unrelated attack waves while retaining a real
  opponent house/MCV and all local production authority. Add a fixture test
  proving no combat-capable opponent entity is seeded and the match does not
  end before local radar readiness.
- Keep absolute RA2 root out of tracked profiles; resolve it from the validated
  local `config.toml` and record it at runtime.
- Embed the versioned JSON contract in Rust with `include_str!`; have Python
  validate, hash, and pass that exact repository file through `--contract`.
  Rust reads the external file and requires byte-for-byte equality with the
  embedded bytes, so a stale executable cannot silently use a different
  denylist. The wrapper hashes the file before and after the child run. A Python
  test scans Rust/Python source for `RA2_*` names and fails if a discovered
  variable is missing from the contract.

**Focused gate**

```powershell
cargo test -p vera20k --lib app_launch::tests -- --nocapture
cargo test -p vera20k --lib app_tactical_capture::profile_tests -- --nocapture
cargo test -p vera20k --lib app_shell_capture::tests -- --nocapture
```

Record literal `test result:` lines.

## Task 2: Add exact one-step production runtime mode

**Files**

- Modify `src/app_sim_tick.rs`.
- Focused tests in that module.

**Red tests**

- Three repeated exact requests at stored speed 1 each advance:
  - simulation tick by exactly `1`;
  - `total_sim_ms` by exactly `22`;
  - `binary_frame` to exactly
    `floor(total_sim_ms * 15 / 1000)`.
- The same holds across every configured game-speed bucket and arbitrary
  nonzero prior accumulator remainders.
- Exact mode clears the accumulator before and after execution.
- Missing/mismatched accepted L0, absent simulation, zero-step, or multi-step
  results fail closed.
- Existing ordinary fixed-scheduler vector tests remain unchanged.

**Implementation**

- Factor an internal runtime advance mode:
  wall-clock scheduling versus exact one fixed step.
- Bypass speed scaling/scheduler only for the exact mode; execute one iteration
  of the existing fixed-step body.
- Keep the accepted-startup gate and every current post-tick app consumer in
  one common body. Use `SIM_TICK_MS=22` for presentation consumers.
- Leave the existing developer debug-frame-step path unchanged in this
  prerequisite. Its accumulator bug is a recorded residual; changing that
  separate feature is not required for the radar oracle.
- Return an app-local receipt containing before/after tick, total simulation
  milliseconds, binary frame, and cleared accumulator so tactical capture can
  fail without guessing.

**Focused gate**

```powershell
cargo test -p vera20k --lib app_sim_tick::tests -- --nocapture
```

## Task 3: Record command scheduling and implement the pure script

**Files**

- Modify `src/app_commands.rs`.
- Extend `src/app_tactical_capture/script.rs` and `session.rs`.
- Tests beside both modules.

**Red tests**

- A new recorded scheduler returns the actual execute tick
  `current_tick + live input_delay_ticks`.
- Existing `schedule_command(...)->()` behavior/callers are unchanged.
- The pure state machine yields at most one action while no pending command
  exists.
- Pending commands are completed only by observed downstream conditions after
  their execute tick, not merely by being queued.
- First MCV deploy may yield either:
  - exact expected active yard with MCV gone; or
  - same MCV alive at rules-derived deploy facing, allowing one second deploy.
- Any other first result or failed second deploy is terminal.
- Queue completion requires the exact target in live queue or ready set.
- Placement completion requires exact active owner/type/cell and ready entry
  consumed.
- Yard and each placed building remain in an explicit construction-completion
  stage until the same entity is active with `building_up.is_none()`.
- Before each queue action, the exact next target must be present and enabled in
  the strict live build-option view; power/radar authority is not a substitute
  for prerequisite eligibility.
- Stage/tick/wall budgets are strict and profile-owned.
- Resolved current-production rates are power `11`, Soviet/Yuri refinery `37`,
  and radar `18`; schedule-to-ready is `3 + 53 * rate` at input delay 2.
  Tests derive these from merged rules/factory inputs, not hand-inject them
  into production.
- The current-production expected ledger is:
  yard active `33`, power ready/active `619/650`, refinery ready/active
  `2614/2645`, radar ready/active `3602/3633`, radar Online `3699`, second
  readiness tuple `3700`, and capture after 16 warm frames `3716`.
  Any deviation is diagnostic `INVALID`, not a reason to silently extend a
  stage.

**Implementation**

- Add `try_schedule_command(...)->Option<u64>` containing the existing
  scheduling body; keep `schedule_command` as a thin unit-returning wrapper.
- `TacticalObservation` is an owned, renderer-free snapshot.
- `TacticalAction` is one of:
  deploy MCV, queue exact type, place exact type/cell, capture, complete, fail.
- `PendingCommand` records action ID, scheduled/execute ticks, owner, payload
  identity, expected result condition, and resolved result.
- The command ledger labels results as observed conditions because the current
  command phase does not expose raw `apply_command` return values.
- Sequence construction-completion observations between every placement and
  next queue. This is load-bearing: current aggregation can count
  `BuildingUp`, while production tech/prerequisite gates reject it.

**Focused gate**

```powershell
cargo test -p vera20k --lib app_commands::tests -- --nocapture
cargo test -p vera20k --lib app_tactical_capture::script_tests -- --nocapture
cargo test -p vera20k --lib sim::deploy_tests::deploy_mcv_waits_for_target_building_deploy_facing -- --nocapture
```

## Task 4: Add deterministic real-placement search

**Files**

- Extend `src/app_tactical_capture/placement.rs` and `script.rs`.
- No `src/sim/**` production edit.

**Red tests**

- Radius zero visits center once.
- Each radius visits top/bottom with X ascending, then left/right interiors
  with Y ascending.
- Bounds clipping occurs before integer conversion.
- No candidate is duplicated.
- First valid `placement_preview_for_owner` result wins.
- No valid candidate inside profile radius fails without fallback.
- The selected cell is stable and appears in the command/result ledger.
- On pinned `Fight.MAP`, one production fixture reaches valid power, refinery,
  and radar cells within radius 16 for each profile.
- Soviet fixture includes the real `NAREFN` free `HARV`, records its stable ID
  and live cell at radar placement, and still selects the same radar cell on
  repeats.

**Implementation**

- Resolve the live local yard from the MCV rules `DeploysInto` target.
- Anchor one documented square-ring iterator to that yard.
- Bound using live `PathGrid::width/height`, never `511` or map-specific
  constants.
- Search only after the exact target appears in
  `ready_buildings_for_owner`.
- Validate only through `placement_preview_for_owner`; place only through
  scheduled `Command::PlaceReadyBuilding`.
- Record every chosen cell. Never reserve or move the Soviet harvester merely
  to stabilize placement; current live occupancy is part of the fixture.

**Focused gate**

```powershell
cargo test -p vera20k --lib app_tactical_capture::placement_tests -- --nocapture
cargo test -p vera20k --lib sim::production::production_placement_tests -- --nocapture
```

## Task 5: Enter through accepted fixed Battle startup

**Files**

- Modify `src/assets/asset_manager.rs`.
- Modify `src/app_list_maps.rs`.
- Modify `src/app_init.rs`.
- Modify `src/app_transitions.rs`.
- Extend `src/app_tactical_capture/profile.rs`, `session.rs`, and `evidence.rs`.
- Narrow integration in `src/app.rs`.
- No direct map seeding or simulation insertion.

**Red tests**

- Profile maps exactly to a classifier-valid `SkirmishLaunchSession`.
- Controlled clock returns only the profile seed and
  `MatchSeedSource::Controlled`.
- Any random map/country/color/start, descriptor drift, wrong slot count,
  inactive slot leak, wrong options, or wrong seed classification rejects.
- Preflight rejects cwd and retail-root loose `Fight.MAP`.
- Preflight requires canonical RA2 root, `multimd.mix` length/hash, source
  chain, MIX ID, and entry length.
- Post-load re-read reports the same archive/source/entry facts.
- The exact map resolution consumed by parsing is carried through
  `MapLoadInitial` and `MapLoadResult`, then pinned in `AppState`; a later
  lookup cannot masquerade as evidence for a different loaded source.
- The first tactical hook sees accepted startup/L0 at tick 0 before any
  automatic simulation advance.

**Implementation**

- Construct the full launch session from the validated profile.
- Add a borrowed read-only asset-resolution accessor that returns the bytes plus
  the actual first-match archive name/source chain, lookup entry ID, and payload
  length. Preserve existing lookup order and `get*` behavior.
- Add an owned observational map-source enum for loose, MIX, generated, and
  legacy/fallback sources. Make the map loader return the parsed `MapFile` with
  the exact source it actually consumed, carry that source through both load
  phases, and pin it at `apply_map_load_result`.
- Require `AcceptedExplicitFixedBattle`.
- Use a capture-local controlled `MatchSeedClock`.
- Enter only through `LoadingRequest::accepted_skirmish` and the ordinary
  loading pump.
- Before the first exact step, verify:
  startup/receipt correlation, seed/source, map/source, local
  owner/country/side/color/start, AI, options, input delay, tick/time/frame
  zero, and required loaded resources.
- Because the selected name already includes `.MAP` and loose shadows are
  rejected, require the carried source and post-load re-resolution to agree
  exactly without adding a second MIX parser. Do not merely recompute the
  profile's expected MIX ID or perform an unrelated later lookup and call it
  observed load evidence.
- Do not use quickplay, direct `apply_map_load_result`, direct entity spawn,
  owner override, cash/prerequisite grant, or forced power/radar.

**Focused gate**

```powershell
cargo test -p vera20k --lib match_bootstrap::tests -- --nocapture
cargo test -p vera20k --lib app_loading::tests -- --nocapture
cargo test -p vera20k --lib app_tactical_capture::startup_tests -- --nocapture
```

## Task 6: Expose actual radar and render provenance without correcting it

**Files**

- Modify `src/render/sidebar_chrome.rs`.
- Modify `src/render/gpu.rs`.
- Modify `src/render/egui_integration.rs`.
- Modify `src/app_transitions.rs`.
- Modify `src/app_render/mod.rs` and narrowly
  `src/app_render/build_instances.rs`.
- Extend `src/app_tactical_capture/evidence.rs` and `session.rs`.
- Focused tests near each seam.

**Red tests**

- Sidebar atlas resolution reports requested theme and actual resolved
  theme/source asset names without changing fallback behavior.
- Radar animation provenance is set atomically with the actual atlas used.
- Current pre-parent Soviet/Yuri startup honestly records Allied animation
  provenance rather than inferring live theme.
- Render output records:
  `SidebarView` present, minimap instance count, viewport-rectangle count, and
  radar-animation instance count.
- Spawn-pick rendering still discards the richer output with no behavior change.
- No UnitAtlas page IDs/counts/dimensions enter the tactical manifest.
- `GpuContext` retains the exact `wgpu::AdapterInfo` returned by the selected
  adapter: name, vendor, device, device type, driver, driver info, and backend.
- Egui evidence reports actual window scale factor, final
  `pixels_per_point`, and the font file actually selected without changing any
  pixel behavior.

**Implementation**

- Add a small immutable identity to `SidebarChromeAtlas`/resolution that reuses
  the existing build inputs rather than duplicating asset-name constants. It
  records requested theme, actual atlas theme, parent archive, radar/palette/
  background logical names, and each actual resolved source archive, including
  fallback.
- Add one app-owned radar-animation provenance field populated at the current
  construction seam.
- Return a small `GameRenderOutput` containing the existing optional
  `SidebarView` plus tactical evidence counts derived from the exact instance
  vectors that were uploaded/drawn.
- Retain immutable `wgpu::AdapterInfo` in `GpuContext` rather than logging and
  discarding it. Expose a borrowed capture-only observation; do not let the
  tactical session select an adapter or backend.
- Make `load_system_font` return a small immutable selected-font identity and
  expose the actual egui `pixels_per_point` already produced by frame rendering.
  Do not change font choice, fallback, DPI handling, or rendering.
- Do not select a different faction atlas, alter insets, or change pixels in
  this prerequisite. Call the field “current Rust radar animation source,” not
  native-correct identity. The mismatch is the parent work.

**Focused gate**

```powershell
cargo test -p vera20k --lib render::sidebar_chrome::tests -- --nocapture
cargo test -p vera20k --lib render::gpu::tests -- --nocapture
cargo test -p vera20k --lib render::egui_integration::tests -- --nocapture
cargo test -p vera20k --lib app_render::tests -- --nocapture
cargo test -p vera20k --lib app_tactical_capture::render_evidence_tests -- --nocapture
```

## Task 7: Wire hidden lifecycle, readiness, readback, and immutable child output

**Files**

- Create or adapt narrow private capture dispatch in `src/app_capture.rs`
  only if the re-anchored shell code makes it smaller/safer.
- Modify `src/app.rs`.
- Extend `src/app_tactical_capture/session.rs`, `evidence.rs`, and
  `manifest.rs`.
- Reuse `src/render/frame_readback.rs` without semantic change.

**Red tests**

- Shell pre-render readiness remains at its existing hook and schema/output is
  unchanged.
- Tactical lifecycle:
  hidden + inactive 800x600 window, every lifecycle event recorded, loading
  pumped, no simulation advance before L0, one script action before exact step,
  one exact step, normal tactical render, post-render readiness, optional one
  readback.
- Initialization requires `Window::is_visible() == Some(false)` and
  `Window::has_focus() == false`. `Focused(true)` or any keyboard, mouse,
  touch, gesture, IME, drag/drop, or other real input event fails immediately;
  bounded resize, redraw, occlusion, focus-false, and close lifecycle events
  remain separately classified.
- A trace test asserts the exact final-frame order:
  `game render -> configured upscale pass (absent in v1) -> egui ->
  tactical post-render/readiness -> encode copy from final output texture ->
  submit -> present -> map readback -> fingerprint revalidation ->
  transactional bundle publish`.
- Readiness rejects each missing/wrong field independently:
  screen/loading/spawn-pick, startup/receipt, seed/map/owner/AI/options/delay,
  resources, active structures, ready placement, power, active radar,
  `AppState.has_radar`, non-Online animation, missing minimap/viewport/radar
  instances, wrong live theme, pause/modal/targeting/debug overlay, non-neutral
  cursor, or match end.
- Structures must be active with `building_up.is_none()`.
- Bind yard, power, refinery, and radar to the exact IDs observed from command
  consequences, with expected owner/type/cell, `!dying`, no second ambiguous
  match, and `building_up.is_none()`.
- Before queueing radar, require the real strict build-option path to expose
  `NARADR`/`NAPSIS`; this positively exercises Soviet explicit `NAREFN` and
  Yuri generic `PROC -> YAREFN` rather than inferring the prerequisite.
- Two consecutive post-render condition tuples and bounded warm frames are
  required; warm-up world pixels/state hashes need not remain frozen.
- The consecutive readiness tuple excludes tick, total time, binary frame,
  state hash, AI/world positions, and other values expected to advance. The
  before-copy/after-readback fingerprint includes them because exact stepping
  is frozen across that synchronous interval.
- Final fingerprint is identical immediately before encode and immediately
  after readback completion.
- Timeout, surface/device/readback failure, unexpected close, or write failure
  exits only this child and retains permitted diagnostics.
- Successful child output has exactly `capture.json` and `frame.bgra`; a failed
  child may have only an exclusive `capture.json` failure diagnostic and no
  frame.
- Fault injection at frame write, manifest write, each file sync, directory
  sync, and final rename proves no final `frame.bgra` is visible on any
  pre-publication failure. A simulated crash may leave only a private sibling
  staging directory; the final output path stays absent and is `INVALID`.

**Implementation**

- Preserve shell session semantics; use exhaustive variant dispatch only for
  truly shared window/failure/wake/readback operations.
- Tactical pre-step hook schedules/observes at most one action and calls the
  exact-step helper only in `InGame`.
- Tactical post-render hook receives `GameRenderOutput`, builds a condition
  tuple, and requests a single final readback.
- Final fingerprint includes tick/time/binary frame/state hash, structures,
  command ledger, power/radar authority, sidebar values/theme, actual radar
  provenance, aperture/geometry, and render evidence.
- Record merged-rules hash plus each target's resolved cost, build-time
  multiplier, prerequisite result, factory count, power ratio, current
  production rate, spent credits, actual wallet, radar frame count, Soviet
  harvester identity/cell, and all placement cells.
- Compute radar-opening budget from the live frame count and 64 ms cadence.
  The current 33-frame Allied source needs 96 pumped frames from opening start;
  its first 30 overlap radar `BuildingUp`, leaving 66 expected after active.
- Revalidate the fingerprint after mapped BGRA bytes are available. Create both
  artifacts exclusively inside one private sibling staging directory, flush
  and sync both files, sync that directory, then atomically rename the complete
  directory to the required nonexistent output path and self-exit. Never write
  either artifact directly into the final path.
- On handled pre-publication failure, best-effort remove only the exact owned
  staging directory and optionally publish a separately staged failure-only
  manifest. On process loss, leave staging for wrapper diagnostics; it can
  never validate as child output.
- Manifest schema is exactly `vera20k.tactical-capture.v1`; status is
  `COMPLETE`/`FAILED`, validation later is `VALID`/`INVALID`, and native
  comparator/parity certification are explicitly `NONE`.
- Describe minimap/viewport/radar values as emitted instance counts, not proof
  that their pixels are native-correct.
- Add route-invariance tests proving interactive and shell visibility, input
  routing, redraw pump, pre-render hook, and completion behavior are unchanged
  when tactical mode is absent.
- Add a present-order regression proving the current main-menu entry and title
  receipts still commit exactly once after `output.present()`, while tactical
  in-game frames create neither receipt and cannot reorder that shell lifecycle.

**Focused gate**

```powershell
cargo test -p vera20k --lib app_tactical_capture::lifecycle_tests -- --nocapture
cargo test -p vera20k --lib app_shell_capture::tests -- --nocapture
cargo test -p vera20k --lib render::frame_readback::tests -- --nocapture
```

## Task 8: Build the isolated tactical-certification wrapper

**Files**

- Create `tools/tactical_certification/__init__.py`.
- Create `tools/tactical_certification/__main__.py`.
- Create `tools/tactical_certification/core.py`.
- Create `tools/tactical_certification/profile.py`.
- Create `tools/tactical_certification/orchestrator.py`.
- Create `tools/tactical_certification/cli.py`.
- Create `tools/tactical_certification/README.md`.
- Create profile JSON files and
  `tools/tactical_certification/tests/**`.
- Do not edit/import-refactor `tools/shell_certification/**`.

**Red tests**

- Strict JSON duplicate/nonfinite/type/key validation.
- Profile/contract/config/executable/archive/font/layout are absolute regular
  non-link files and remain byte/size/identity stable before/after.
- Every existing ancestor of working/output paths is checked for Windows
  reparse points; working/output parents are non-link/non-junction directories.
- `config.toml` resolves the exact canonical retail root.
- Archive length/hash and loose-shadow rejection match the profile.
- Child launch uses argument list, `shell=False`, fixed cwd,
  `stdin=DEVNULL`, and stdout/stderr redirected to temporary regular files
  outside the child artifact directory.
- Timeout kills only the exact child PID and drains stdout/stderr.
- Whole-child timeout is 720 seconds for v1, with a hard schema maximum of 900;
  child post-L0 state separately enforces 600 seconds and 4,096 ticks. Do not
  inherit shell certification's shorter limits.
- Nonzero exit, launch failure, timeout/kill/drain failure, missing/extra/link
  success artifacts, any failure-frame artifact, mutation, or invalid manifest
  is `INVALID` exit `2`.
- The final child output is accepted only after a complete-directory publish.
  Any private staging sibling or absent final path is retained as failure
  diagnostics and never interpreted as a capture.
- Wrapper writes retained profile copy, stdout, stderr, validation, and run
  report exclusively with flush/fsync.
- Same-profile repeat validation compares only declared stable fields and exact
  BGRA bytes; this includes adapter/surface identity, window and egui scale,
  selected font, layout, and contract identities. Host timestamps, run paths,
  PIDs, and durations are excluded.
- No result uses `MATCH` or `DRIFT`.

**Implementation**

- Reimplement only the small safety primitives needed by this package; do not
  make shell v2 depend on tactical code.
- Use Python stdlib `hashlib`, `tomllib`, `subprocess`, and safe filesystem
  checks.
- Follow the sealed shell wrapper's exact timeout pattern: wait on the
  `Popen`; on timeout kill only that still-live child object; bounded wait
  again; then close/read the regular stdout/stderr files. Never use pipes,
  `taskkill`, process-name/group termination, descendant traversal, or an
  inherited handle whose drain can block after the child is gone.
- Do not implement encrypted MIX parsing. Hash exact `multimd.mix` before/after
  and validate the child's production source/MIX-ID/payload-length facts.
- Hash the exact external contract, selected font, and sidebar-layout file
  before and after the child. Pass the contract with `--contract`; require the
  child manifest to confirm byte equality with its embedded contract.
- CLI:
  - `validate-profile`
  - `capture`
  - `validate`
  - `validate-repeat`
- All success/failure reports are immutable JSON with explicit evidence
  limitations.

**Focused gate**

```powershell
python -m unittest discover -s tools/tactical_certification/tests -v
python -m tools.tactical_certification validate-profile --profile <soviet-profile>
python -m tools.tactical_certification validate-profile --profile <yuri-profile>
```

## Task 9: Adversarial review and feature validation

1. Format only edited Rust files with edition 2024; inspect the diff and remove
   unrelated churn.
2. Run focused tests serially under the one Cargo lease, recording every literal
   `test result:`:

   ```powershell
   cargo test -p vera20k --lib app_tactical_capture -- --nocapture
   cargo test -p vera20k --lib app_launch -- --nocapture
   cargo test -p vera20k --lib app_shell_capture -- --nocapture
   cargo test -p vera20k --lib app_sim_tick::tests -- --nocapture
   cargo test -p vera20k --lib render::frame_readback -- --nocapture
   cargo test -p vera20k --lib render::sidebar_chrome -- --nocapture
   cargo test -p vera20k --lib render::gpu -- --nocapture
   cargo test -p vera20k --lib render::egui_integration -- --nocapture
   python -m unittest discover -s tools/tactical_certification/tests -v
   cargo check -q -p vera20k
   cargo build --bin vera20k
   ```

3. Run two fresh hidden Soviet captures and two fresh hidden Yuri captures. No
   desktop focus, OS input, Oracle mutation, or native process is allowed.
4. Require child exit `0`, exact artifact inventory, `COMPLETE` child manifest,
   `VALID` wrapper validation, fixed startup/command/placement/readiness ledger,
   zero focus/input violations, exact environment identities, process cleanup,
   and byte-identical same-profile BGRA plus stable fields.
5. If the first real run stalls, diagnose end-to-end. Do not inject state or
   weaken readiness. Suspend only for the earliest genuinely required
   correctness/architecture prerequisite; otherwise record expert-only or
   scheduler-tail differences as residuals.
6. Dispatch independent code/evidence review. Ask:
   “Why should this be approved, and what evidence could still make it wrong?”
   Repair every P0/P1 and rerun affected gates.
7. Re-run `python -m tools.system_map check --require-sources`. Do not edit the
   mechanism tooling owner's claimed files or add a false completion/oracle
   claim.

## Task 10: Coherent commit, guarded integration, and parent resume

1. Commit coherent reviewed milestones on the feature branch; record each SHA.
2. Re-audit exact current `dev`, claims, processes, worktrees, root `README.md`,
   and operation state.
3. If `dev` advanced, incorporate exact current `dev` into the feature branch,
   resolve only owned paths, and rerun all affected tests plus both profile
   repeat gates.
4. Acquire the `dev` integration lease, no-ff merge locally, and verify the
   merge tree matches the reviewed feature tree. Never push.
5. Post-merge, rerun:

   ```powershell
   cargo test -p vera20k --lib app_tactical_capture -- --nocapture
   cargo test -p vera20k --lib app_shell_capture -- --nocapture
   cargo test -p vera20k --lib render::gpu -- --nocapture
   cargo test -p vera20k --lib render::egui_integration -- --nocapture
   python -m unittest discover -s tools/tactical_certification/tests -v
   cargo check -q -p vera20k
   cargo build --bin vera20k
   ```

   Then run one fresh repeated Soviet pair and one fresh repeated Yuri pair from
   merged `dev`.
6. Record exact branch/commit/merge SHAs, changed files, literal validation,
   evidence paths/hashes, review verdict, residuals, and the exact next safe
   action in the operational journal.
7. Release Cargo and `dev` explicitly with no build/game process or Git
   operation active.
8. Resume suspended parent `GSI-13.23` from the validated merged `dev`. Re-read
   current radar research/code, correct the earliest load-bearing ordinary
   faction radar divergence, and rerun the complete tactical profiles. Do not
   select another owner before the parent is resumed.

## Planned residuals

- Native pixels, bytes, transition cadence, audio, wall-clock pacing,
  visible-window compositor behavior, input feel, and full-map/resolution
  coverage remain `UNVERIFIED`.
- The tactical exact-step schedule is diagnostic; it does not certify normal
  render pacing or stutter.
- `SIM_TICK_MS=22` is the engine's deterministic integer convention for 45 Hz,
  not an exact fractional `1000/45` wall-clock duration.
- UnitAtlas packing order is process-nondeterministic internally. For this radar
  checkpoint, matching UV/page pairs make it expected to be pixel-transparent;
  page identities are excluded. Any observed BGRA instability escalates it to a
  blocker. Otherwise it remains a documented paging/determinism residual.
- Current Allied radar-animation provenance for Soviet/Yuri is expected known
  `DRIFT` before the parent fix; a `VALID` capture must not relabel it.
- One two-player map, one start arrangement, one backend/config, and two faction
  profiles do not certify every ordinary radar case.
- Current non-wall factory timing ignores `BuildSpeed=.7` and placed buildings
  use a hardcoded 30-tick buildup. The checkpoint records those current-code
  dependencies; it does not certify them as native.
- Ordinary placement of Yuri `YAREFN` currently creates no slave bindings even
  though stock data has `Enslaves=SLAV`; the checkpoint records this gameplay
  drift and does not synthesize slaves.
- Easy AI currently marks its lone MCV deployed after one facing-only attempt.
  With stock-valid `unit_count=0` it cannot attack or obstruct this oracle, but
  the AI lifecycle defect remains outside this prerequisite.
