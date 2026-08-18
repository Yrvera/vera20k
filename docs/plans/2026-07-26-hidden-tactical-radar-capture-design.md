# Hidden Tactical Radar Capture Design

## Status

Design approved for the smallest prerequisite slice needed by the suspended
`GSI-13.23` ordinary-radar presentation owner. The 2026-07-26 formal review
returned `FIX FIRST`; this revision closes its evidence-integrity findings.

This document authorizes planning, not implementation. Production work must wait
until the active exact-shell task explicitly releases the global Cargo and dev
integration leases, after which the prerequisite must start from reconciled
current `dev` in its own feature worktree.

## Goal

Add one self-terminating, hidden, no-input production checkpoint that can
autonomously drive and capture the ordinary stock skirmish path:

```text
accepted fixed Battle startup
  -> retail map load
  -> local MCV deploy
  -> real power-plant production and placement
  -> real refinery production and placement
  -> real radar-building production and placement
  -> live power/radar authority
  -> radar opening reaches Online
  -> ordinary sidebar/minimap composition
  -> final swapchain readback
```

The initial profiles are Soviet and Yuri at 800x600 on one pinned stock map.
This prerequisite exists to make the parent radar correction autonomously
verifiable without focusing the desktop or injecting operating-system input.

It is not a native pixel-parity oracle and must not be described as one.

## Why A Prerequisite Is Required

The current `--shell-capture` contract is deliberately sealed to the main-menu
`0xE2` checkpoint. It rejects quickplay, accepts only the main-menu screen,
captures one shell frame, and exits. Generalizing its manifest or readiness
rules would weaken an already useful evidence boundary.

`RA2_QUICKPLAY` is also unsuitable. It follows the generic `"auto"` load path,
can choose a cwd-shadowed or fallback map, uses a wall-clock seed, has heuristic
owner selection, does not produce the accepted-startup Rust-L0 receipt, and does
not self-capture or self-exit.

Finally, an ordinary new stock skirmish starts without operational radar.
Capturing the first tactical frame can exercise loading and the offline sidebar,
but cannot expose the selected defect: the live Soviet/Yuri radar chrome,
aperture, minimap, and transition state. The checkpoint therefore has to drive
the real acquisition loop far enough to make radar operational.

## Architecture Context

The existing production path already provides the load and gameplay mechanisms
this checkpoint needs:

- `SkirmishLaunchSession` plus `classify_startup_session` can describe and admit
  an explicit fixed stock Battle session.
- `MatchSeedClock` supports a controlled seed.
- `LoadingRequest::accepted_skirmish`, the loading pump, and
  `apply_map_load_result` own the real loading and Rust-L0 handoff.
- `app_commands::schedule_command` queues ordinary deterministic
  `CommandEnvelope`s into the simulation.
- `Command::DeployMcv`, `Command::QueueProduction`, and
  `Command::PlaceReadyBuilding` exercise the real deployment, prerequisites,
  cash, build-time, placement, lifecycle, power, and radar producers.
- `production::placement_preview_for_owner` is the existing placement authority
  and can validate a deterministic ordered cell search.
- `app_render::render_game` is the real tactical composition path.
- `PendingBgra8Readback` already copies the final BGRA8 swapchain, removes GPU
  row padding, and fails on malformed mappings without changing channels.

The capture remains an app/tooling concern. It must not add a render dependency
to `sim/`, alter normal skirmish state ownership, or introduce capture-only game
objects.

System Map v2 identifies `GSI-13.23` as the radar/minimap renderer consumed by
both `LOOP-008-REVEAL-RADAR` and
`LOOP-012-POWER-OUTAGE-RECOVERY`. The proposed checkpoint supplies neither
loop's missing native differential and does not certify either loop complete.
It is a narrower production-route prerequisite that makes the ordinary
power/radar/sidebar handoff autonomously observable before those broader loops
are revisited.

## Impact Analysis

Expected production impact is intentionally app-local:

- normal interactive startup, loading, simulation pacing, input, and rendering
  must remain byte-for-byte behaviorally unchanged when tactical capture is not
  requested;
- shell capture must retain its current accepted arguments, schema, hidden
  lifecycle, readiness, artifacts, and failure behavior;
- accepted skirmish startup and fixed simulation APIs are consumed, not given
  tactical-specific state;
- the only shared render primitive is final BGRA8 swapchain readback;
- deterministic simulation commands and state remain owned by `sim/`; the
  capture session observes and schedules them from the app layer;
- the new wrapper/profile/schema is isolated under
  `tools/tactical_certification/`.

The highest-blast-radius seams are `src/main.rs` and `src/app.rs`, because they
own launch dispatch, window visibility/input, loading-to-game transitions,
frame advancement, presentation, failure exit, and the existing shell capture.
They require explicit normal-launch and shell-capture regression tests.

The fixed-step seam is the determinism risk. It must neither duplicate the
production post-tick consumers nor leak its accumulator/timing behavior into
ordinary play. A missed or repeated tick invalidates the command ledger and
frame comparison.

The tool boundary is the evidence-integrity risk. A mutable output directory,
fallback asset/map resolution, inherited quickplay/debug state, broad process
termination, or permissive artifact set could produce convincing-looking but
untrustworthy output.

No simulation snapshot/version, state hash, network command, rules parser,
retail INI, map parser, renderer content, sidebar/radar behavior, or public
library API is expected to change. Discovery that one of those must change
suspends this prerequisite for renewed design review.

## Current Parent Divergence

The parent owner remains suspended while this prerequisite is built.

Current Rust evidence shows:

- map-load initialization selects Allied radar animation/inset data before the
  live local-house theme is pinned;
- sidebar chrome later resolves from the live owner, so the producer and
  consumer can disagree;
- minimap content uses a generic fixed `13,0,140x120` rectangle;
- verified active-YR research separates ordinary Soviet/Yuri radar background,
  aperture, right-panel, and transition responsibilities;
- the minimap is withheld until `RadarAnimPhase::Online`.

The checkpoint does not fix any of those differences. It only creates the
production oracle required to correct and regress them safely.

## Chosen Approach

### Separate tactical capture boundary

Add a distinct `TacticalCaptureSession` and
`vera20k.tactical-capture.v1` evidence bundle.

Share only:

- a small top-level launch/capture dispatch boundary;
- hidden-window lifecycle primitives;
- final-surface `PendingBgra8Readback`;
- generic failure propagation and event-loop exit.

Keep `ShellCaptureSession`, `vera20k.shell-capture.v2`, its exact key set, and
`tools/shell_certification/` behavior unchanged.

Tactical launch receives the validated profile, external contract, and
nonexistent final output paths explicitly. Rust embeds the same versioned
contract with `include_str!`, reads the wrapper-supplied external contract, and
requires byte-for-byte equality before startup. This prevents a stale executable
from silently enforcing a different environment denylist than the wrapper.

### Profile-owned scenario

The certification profile/tool, not production Rust constants, owns the fixture:

- canonical stock map identity, source archive path and digest, MIX entry ID,
  and expected entry-payload length and digest;
- explicit Battle mode descriptor;
- controlled match seed;
- fixed local and AI countries, colors, start positions, teams, difficulty, and
  options;
- fixed input-delay ticks and expected accepted-startup correlation fields;
- 800x600 render surface;
- neutral software cursor;
- the code-derived app UI scale `0.5`, post-load neutral cursor `(358,300)`,
  and one-frame `CursorId::Default`;
- the selected system-font file and sidebar-layout file identities;
- local side-specific build targets:
  - Soviet profile: `NAPOWR`, `NAREFN`, then `NARADR`;
  - Yuri profile: `YAPOWR`, `YAREFN`, then `NAPSIS`;
- bounded per-stage simulation-tick and wall-clock failure budgets.

The first implementation profile uses the verified retail fixture:

- logical name `Fight.MAP`;
- canonical source `multimd.mix`, MIX ID `0x9306F050`;
- source archive length `31,264,268` bytes and SHA-256
  `FF4138BA95F7EFD8BDED14342FC9082B99C47E43C25AB18236E4EEA141B488E9`;
- entry payload length `91,254` bytes and SHA-256
  `D751DCE7CD3611077E9228C33235F39C71681FFF6AC08CA1F716D963AD6CE070`;
- `NEWURBAN`, map size `81x52`, local size `75x42`;
- exactly two start waypoints, local `Position(0)` and AI `Position(1)`;
- Battle descriptor ID `1`; zero-based `ScenIndex=12` is declared catalog
  provenance only and is never applied as a launch-session or persistence field.

The wrapper must verify the exact top-level retail archive length and SHA-256
before and after the child run. The child must reject cwd and retail-root loose
shadows, then require the ordinary loader's carried source provenance and a
post-load `AssetManager` re-resolution to agree that the exact parsed bytes
came from `multimd.mix`, with actual lookup ID `0x9306F050` and payload length
`91,254`. The pinned archive digest uniquely seals the bytes consumed through
that entry, while the independently audited entry digest is labeled declared
fixture provenance rather than a runtime-computed hash. Together these checks
close the time-of-check/time-of-use and shadow boundaries without adding a
second encrypted-MIX implementation, new Python crypto dependency,
capture-only map loader, or Rust hashing dependency. A same-named cwd or
loose-install file is an error, not a fallback.

The launch request explicitly owns every field consumed by
`SkirmishLaunchSession`, including all 18 `SkirmishLaunchOptions` values:

- `starting_credits=10000`, `unit_count=0`, `tech_level=10`,
  `game_speed=1`, `default_ai_difficulty=0`;
- `short_game`, `bases`, `bridges_destroyable`, `super_weapons`,
  `build_off_ally`, `crates`, `mcv_redeploy`, `shroud`,
  `tiberium_grows`, and `ally_change_allowed` enabled;
- `fog_of_war`, `multi_engineer`, and `harvester_truce` disabled.

It also owns local/AI country, color, start, and team fields because current
Rust does not hydrate those identities from `[MultiPlayer]`. Persisted
`RA2MD.INI` rows that are not consumed by the explicit accepted-startup path do
not appear in the certification profile merely to create apparent coverage.
Tests instead prove that arbitrary persisted skirmish fields cannot change the
explicit launch session or Rust-L0 receipt.

Use two concrete classifier-valid profiles, both with controlled seed
`0x12345678`, Gold local player at `Position(0)`, Red Easy AI at
`Position(1)`, no teams, no random country/color/start fields, and exactly one
active opponent:

- Soviet: local `LaunchCountry::Russia`, player name `VERA-SOVIET`; AI
  `LaunchCountry::Yuri`;
- Yuri: local `LaunchCountry::Yuri`, player name `VERA-YURI`; AI
  `LaunchCountry::Russia`.

The names above are the unique nonempty profile-owned player names. Starts and
teams remain explicit launch-session fields; inactive persisted slot triples
are not part of this profile.

`unit_count=0` is a stock-supported skirmish slider value
(`MinUnitCount=0`). It is deliberate diagnostic isolation, not a production
code simplification: the real opponent house and its MCV remain, while no
unrelated starting combat wave can destroy or block the 3,000+ tick local
producer chain. The fixture still exercises accepted Battle startup, both
houses, MCV deployment, ordinary production, cash, prerequisites, placement,
power, radar, AI scheduling, and final tactical composition. It does not claim
to certify ordinary combat or the AI's separately known one-attempt MCV deploy
residual.

The v1 profiles seal current-production budgets rather than using an unbounded
“eventually” condition:

| Stage | Current expected ticks | Strict tick cap | Strict wall cap |
|---|---:|---:|---:|
| L0 through active yard, including two deploy attempts | 33 | 48 | 15 s |
| Queue power through ready | 586 | 640 | 90 s |
| Place power through construction complete | 31 | 48 | 15 s |
| Queue refinery through ready | 1,964 | 2,048 | 270 s |
| Place refinery through construction complete | 31 | 48 | 15 s |
| Queue radar through ready | 957 | 1,024 | 140 s |
| Place radar through construction complete | 31 | 48 | 15 s |
| Active radar through animation Online | 66 expected | 96 | 20 s |
| Second readiness tuple plus 16 warm frames | 17 | 18 | 10 s |

The expected complete ledger reaches capture at simulation tick `3,716`; the
strict overall post-L0 cap is `4,096` ticks. The wrapper uses a `600 s`
post-L0 wall cap, `720 s` whole-child timeout, and absolute tactical maximum
`900 s`. Tick caps are authoritative; wall caps are provisional host-safety
bounds and must be checked during profile enrollment.

The app-derived UI scale and cursor are not enrollment guesses:
`auto_detect_ui_scale(800,600)` is `0.5`; the 168-pixel stock sidebar becomes
84 pixels wide; and `apply_map_load_result` resets the tactical-interior center
to `(358,300)`. The operating-system window scale factor and egui
`pixels_per_point` are separate pixel inputs. The child records their actual
values and same-profile validation requires exact agreement.

The wrapper pins the selected current font and layout inputs before and after
each child run. The initial machine profile selects
`C:\Windows\Fonts\verdana.ttf`, length `243,304`, SHA-256
`6A8481FE107EE547893C018B13DBA291C2020BEC3DE5DA6525D9AC09F6BC2105`,
and `src/sidebar/sidebar_layout.ron`, length `721`, SHA-256
`27FE2405990000468B1D6B9F4316D8B6104D72C82BB3386A9942332BA323316C`.
If production selects a different font or layout fallback, the capture is
invalid rather than silently comparable.

Factory expectations are derived from the current authoritative 54-step
cadence (`schedule-to-ready = input_delay + 1 + 53 * rate`) and the live merged
rules. The manifest records the merged-rules hash, target cost, per-type
multiplier, prerequisites, power state, matching factory count, resolved rate,
spent credits, and actual wallet. Do not describe the current hardcoded
30-tick buildup or non-wall `BuildSpeed` behavior as proven native parity.

Placement search has a v1 maximum square-ring radius of `16` cells and is
accepted only after a production fixture proves all three cells on pinned
`Fight.MAP`. Soviet `NAREFN` creates a real `HARV`; the radar search must use
the live placement authority on the ready tick, record that harvester's stable
ID/cell, and require repeat runs to choose the same radar cell. Yuri `YAREFN`
currently creates no slaves through ordinary placement; that existing gameplay
drift is recorded rather than synthesized inside this oracle.

### Accepted startup only

Build an explicit `SkirmishLaunchSession` from the request and require
`classify_startup_session` to return `AcceptedExplicitFixedBattle`.

Use a capture-local `MatchSeedClock` that returns the requested seed with
`MatchSeedSource::Controlled`, then enter through
`LoadingRequest::accepted_skirmish`.

After load, fail closed unless all of the following match the request:

- accepted startup and Rust-L0 receipt;
- canonical map identity and digest;
- match seed and seed source;
- configured and live input-delay ticks;
- local owner, side/country, slot, color, and start position;
- `GameScreen::InGame`;
- no loading session, spawn picker, modal, pause overlay, or dev overlay;
- required simulation, rules, map, terrain, palette, sidebar, minimap, cursor,
  sprite-atlas, and unit-atlas resources.

Never call lower-level map seeding directly. Never insert or mutate entities,
override the local owner, grant prerequisites, grant cash, force power, or force
radar state.

### Deterministic production script

The tactical session owns a small condition-driven state machine:

1. `AwaitRustL0`
2. `DeployMcvAttemptOne`
3. `AwaitMcvTurnOrConstructionYard`
4. `DeployMcvAttemptTwoIfNeeded`
5. `AwaitConstructionYard`
6. `AwaitYardConstructionCompleteAndPowerBuildOption`
7. `QueuePower`
8. `AwaitPowerReady`
9. `PlacePower`
10. `AwaitPowerConstructionCompleteAndAuthority`
11. `AwaitRefineryBuildOption`
12. `QueueRefinery`
13. `AwaitRefineryReady`
14. `PlaceRefinery`
15. `AwaitRefineryConstructionCompleteAndRadarBuildOption`
16. `QueueRadar`
17. `AwaitRadarReady`
18. `PlaceRadar`
19. `AwaitRadarConstructionComplete`
20. `AwaitRadarOnline`
21. `WarmStableFrames`
22. `CaptureFinalSurface`
23. `Complete` or `Failed`

Each action is an ordinary command scheduled for the local owner. The state
machine advances only after reading the real downstream state produced by prior
ticks. It records command owner, execute tick, payload identity, result
condition, resolved entity IDs, and chosen placement cells.

The app scheduling adapter records the actual execute tick while preserving the
existing unit-returning scheduling helper. A pending action is not considered
accepted merely because it was queued: after its execute tick, the state machine
requires the command-specific downstream condition (turned MCV or yard, exact
queue/ready entry, or exact placed active building with the ready entry
consumed). These are recorded as observed result conditions; the current
simulation command phase does not expose raw `apply_command` return values.

Stock startup gives the MCV facing `64`, while the target construction yard can
require another `DeployFacing`. The first legitimate `DeployMcv` command may
therefore only turn the live MCV. After its execute tick, accept either a live
yard or the same live MCV at the rules-derived target facing. In the latter
case, schedule exactly one second `DeployMcv` command. After that execute tick,
require the MCV to be gone and exactly the expected active yard to exist; never
loop or mutate facing directly.

Deployment and placement initially attach a 30-tick `BuildingUp` lifecycle.
Current power/radar aggregation can observe such a structure earlier than the
production prerequisite gate will accept it. Therefore every placed structure
must remain in an explicit construction-completion stage until the same
owner/type/entity is alive, active, and has `building_up.is_none()`. Before
queueing the next target, positively require the exact strict build option to
exist and be enabled. Power/radar authority alone is never used as proof that a
production prerequisite is ready.

For placement, inspect cells in one documented stable order around the live
construction yard and accept the first cell for which
`placement_preview_for_owner` returns a valid preview. Schedule
`PlaceReadyBuilding`; do not call the lower-level placement mutator.

The AI remains active because the checkpoint is a stock skirmish route. Any AI
interference that makes the named fixed fixture nondeterministic is a failed
profile to diagnose, not permission to disable normal simulation globally or
inject state.

### Fixed-step hidden pumping

Wall-clock-driven hidden rendering can advance a variable number of fixed ticks
per frame and can make visual animation/readiness nondeterministic. The
checkpoint therefore needs a capture-local fixed-step driver around the existing
`advance_in_game_runtime` production path.

For each scripted tactical frame:

- clear any accumulated wall-clock simulation remainder;
- bypass the wall-clock speed scheduler and execute exactly one iteration of
  the existing fixed-step body at the current production rate of `45 Hz`
  (`SIM_TICK_MS=22`);
- advance all normal post-tick app consumers;
- render one production frame;
- require simulation tick `+1` and `total_sim_ms +22`, and record the resulting
  binary frame as `floor(total_sim_ms * 15 / 1000)` rather than incorrectly
  requiring it to increment on every simulation tick.

The implementation must prove with focused tests that this mode advances
exactly one simulation tick per pumped frame across every configured game-speed
value and arbitrary pre-existing accumulator remainders. The existing
debug-frame-step flag cannot simply be assumed to do so: at stored speed `1`
its `22 ms` request scales to `30 ms`, leaving `8 ms`, then `16 ms`, and
eventually producing two steps on the third request. Factor one internal
exact-step mode through the existing fixed-step body and common
runtime/post-tick consumers rather than duplicating simulation behavior or
exactifying the ordinary scheduler.

The fixed-step schedule is a deterministic diagnostic schedule. It does not
certify retail wall-clock pacing or visible-window presentation smoothness.

### Readiness and final capture

The capture is ready only after the real production loop reports:

- the expected local construction yard, power plant, and radar building are
  live and owned by the local house;
- the expected faction refinery is live and owned by the local house, satisfying
  the stock `PROC`/explicit refinery prerequisite through the real rules path;
- aggregate power is sufficient and the house is not in low power;
- `has_radar` is true through the ordinary radar authority;
- `RadarAnimPhase::Online`;
- live sidebar theme matches the local house;
- the currently observable radar/chrome asset identities and geometry are
  recorded;
- minimap content has a valid aperture rectangle;
- the ordinary tactical render reports minimap, viewport rectangle, and radar
  chrome emission;
- the same complete readiness snapshot holds for two consecutive post-render
  evaluations, followed by a bounded number of additional stable frames.

The prerequisite must not require the side-specific radar identity to be
*correct*, because selecting the wrong Allied radar data is the parent drift it
exists to expose. Before the parent fix, a complete capture is `VALID`
production evidence while the separately evaluated radar result remains known
`DRIFT`. The tactical tool exposes only `VALID`/`INVALID`; it never emits
`MATCH` or `DRIFT` without a native comparator. The parent adds the verified
side-specific identity assertion and changes the pixels; the checkpoint then
guards that correction.

Because `RadarAnimState` currently carries no construction provenance, add the
smallest app-layer observational identity set atomically at the existing radar
animation construction seam. The capture records that actual identity; it must
not infer it from the later live sidebar theme, because doing so would conceal
the known Allied-source drift. This metadata has no simulation or render
authority. The immutable source identity is built from actual atlas resolution:
requested and resolved themes, parent archive, radar/palette/background logical
names, and each actual source archive including fallback. It is described as
the **current Rust radar animation source**, never as a native-correct identity.

Readiness that depends on render scratch state must be evaluated after
`app_render::render_game`, while the readback is encoded only after all
world/sidebar/egui composition has reached the final swapchain texture.

Revalidate the same state immediately before writing the bundle. Capture one
final surface; do not synchronously read back every warm-up frame.

### Evidence bundle

The final child output directory may contain only `capture.json` and
`frame.bgra`. A successful child publishes exactly both; a handled failure may
publish only an exclusive `capture.json` failure diagnostic and never a partial
frame. The child must not write either final artifact directly into the output
directory. It creates and fully syncs both files inside one exclusive private
sibling staging directory, syncs that directory, and atomically renames the
whole directory to the required nonexistent output path. A pre-publication
failure best-effort removes only its exact owned staging directory, then may
publish a separately staged failure-only bundle. A crash may leave an
unpublished staging directory, but the final output path remains absent and the
wrapper reports `INVALID`; staged bytes are never accepted as a capture.

Its immutable manifest plus the wrapper-owned immutable run report form the
evidence bundle.
The child records live production identities and state; the wrapper records
independently hashed filesystem inputs. Together they must include at least:

- schema/profile/checkpoint identities;
- executable, config, profile, embedded/external contract, canonical retail
  archive, selected font, sidebar layout, and frame hashes;
- the verified map-entry provenance digest plus the child's actual production
  source archive, MIX ID, and payload length;
- full graphics adapter identity (name, vendor, device, device type, driver,
  driver info, backend), surface format, render extent, app UI scale, actual
  window scale factor, egui `pixels_per_point`, and pixel-affecting options;
- full fixed launch ledger and accepted-startup/Rust-L0 identities;
- controlled seed and source;
- command/tick/placement state-machine ledger;
- final simulation tick, binary frame, and deterministic state hash;
- local owner/country/theme;
- built construction-yard/power/refinery/radar IDs and types, plus power/radar
  authority state;
- radar phase, chrome/radar asset identities, and minimap aperture rectangle;
- final BGRA dimensions, row stride, byte length, and SHA-256;
- child exit status, bounded durations, and explicit evidence limitations.

The wrapper should reuse the shell tool's safety patterns without sharing its
schema: exclusive new run directory, executable/config validation,
child-PID-only timeout cleanup, retained stdout/stderr, unexpected-artifact
rejection, and no shell-mediated process launch. The sequential shell artifact
writer is not reusable for this bundle because it does not provide the
transaction above.

Two fresh runs of the same profile must be byte-identical on the same validated
hardware/config before the checkpoint is accepted as a Rust regression ratchet.
Their adapter, surface, window-scale, egui-scale, font, layout, and contract
identities must also agree exactly. Later parent comparisons use the accepted
pre-parent run report as the expected environment identity.
The stable comparator must not include UnitAtlas page numbers, UV placements,
or other internal packing identities: this radar prerequisite needs final
pixels and radar/sidebar authority, not paging certification. Any observed
process-to-process framebuffer instability remains a blocker to diagnose; the
known non-total atlas packing order is otherwise recorded as a separate
determinism residual for the pending paging owner.

## Fail-Closed Rules

Reject the run before or during execution when:

- the output directory exists or is not an absolute new directory;
- dimensions or checkpoint/profile identity are unsupported;
- `Window::is_visible()` is not exactly `Some(false)`, `Window::has_focus()` is
  true, a `Focused(true)` event occurs, or any keyboard, mouse, touch, gesture,
  IME, drag/drop, or other real input event is received; bounded resize,
  occlusion, redraw, focus-false, and close lifecycle events are recorded
  separately rather than treated as user input;
- `RA2_QUICKPLAY`, developer shell/spawns, LAT/tiberium/image overrides, or
  another state/pixel-affecting debug override is active;
- the map resolves outside the canonical retail source or its digest differs;
- persisted `RA2MD.INI` values leak into a supposedly fixed launch field;
- startup classification or Rust-L0 receipt differs;
- local owner, side, position, seed, mode, options, or resources differ;
- a scripted command is rejected, duplicated, scheduled for the wrong owner, or
  misses its condition;
- the first MCV deploy neither creates a yard nor leaves the same MCV at the
  rules-derived deploy facing, or the bounded second deploy does not create the
  yard;
- placement has no valid deterministic candidate;
- a stage exceeds its tick or wall-clock budget;
- match end, modal UI, pause, fallback shell, device loss, or surface failure
  occurs;
- power/radar never becomes operational;
- final render state differs between readiness and bundle write;
- embedded contract bytes differ from the exact wrapper-validated external
  contract, or the executable/config/profile/archive/font/layout identity
  changes during the run;
- an unexpected file appears in the run directory.

On every failure, self-exit and preserve a diagnostic manifest plus child
stdout/stderr where the wrapper contract permits it. Do not fall back to a
different map, side, seed, building, placement, checkpoint, or capture surface.

## Production Touchpoints

Expected narrow implementation surface:

- `src/main.rs`
  - parse/dispatch/finish tactical capture without changing normal launch.
- new `src/app_tactical_capture/`
  - `mod.rs` owns the narrow public app boundary;
  - bounded `profile.rs`, `script.rs`, `placement.rs`, `evidence.rs`,
    `manifest.rs`, and `session.rs` modules own validation, controlled launch,
    deterministic actions, readiness, transactional publication, and lifecycle
    without creating a multi-thousand-line app module.
- `src/app.rs`
  - store the mutually exclusive capture mode, request a hidden window, suppress
    input, pump fixed tactical frames, expose pre/post-render hooks, complete
    final readback, and self-exit.
- `src/lib.rs`
  - export the new app module.
- `src/app_shell_capture.rs`
  - only if its top-level parser boundary must move behind a neutral launch enum;
    shell request/manifest/readiness semantics remain unchanged.
- `src/app_sim_tick.rs`
  - only if a narrow tested exact-one-tick capture advance is needed to avoid
    duplicating the production runtime/post-tick path.
- `src/assets/asset_manager.rs`
  - add one borrowed, read-only actual-resolution view exposing the selected
    archive/source chain, lookup entry ID, payload length, and bytes without
    changing lookup order or existing getters.
- `src/app_list_maps.rs`, `src/app_init.rs`, and `src/app_transitions.rs`
  - carry the observational source actually consumed by map parsing through
    both loading phases into `AppState`; do not substitute a later lookup.
- `src/render/frame_readback.rs`
  - no functional change expected; neutral documentation naming only if needed.
- `src/render/gpu.rs`
  - retain immutable `wgpu::AdapterInfo` in `GpuContext` instead of logging and
    discarding it.
- `src/render/egui_integration.rs`
  - expose the actual selected font identity and final `pixels_per_point`
    observationally without changing font selection or rendering.
- new `tools/tactical_certification/**`
  - sealed profiles, immutable child runner, validation, repeat comparison, and
    focused Python tests.
- focused ignored design/plan/evidence documents and the operational journal.

No planned changes belong in `src/sim/**`, `src/rules/**`, `src/map/**`, radar
behavior, power behavior, sidebar/radar rendering, or UnitAtlas production code.
If those are required merely to make the checkpoint pass, stop and classify the
new dependency rather than hiding it inside the prerequisite.

## Player-Experience Ledger

Covered by the prerequisite:

- a genuine fixed stock Battle session reaches the real tactical screen;
- a real local MCV deploys;
- the local player builds and places the faction's real power and radar
  structures, including the stock refinery prerequisite, through normal
  command/production/placement authority;
- live house identity reaches sidebar selection;
- live aggregate power enables radar;
- the opening transition reaches Online;
- terrain, units, sidebar, radar chrome, minimap, cursor, and overlays compose in
  the final production swapchain;
- Soviet and Yuri profiles can be captured without desktop control;
- the process fails or completes autonomously and leaves immutable evidence.

Explicitly not covered:

- exact gamemd pixels, bytes, frame number, transition cadence, or audio;
- visible-window compositor behavior or input responsiveness;
- player-driven clicks and placement gestures;
- wall-clock pacing or absence of stutter;
- radar power-loss closing/reopening;
- every map, start position, resolution, graphics backend, or faction;
- rare/expert-only radar and scheduler tails;
- full UnitAtlas paging unless a separate named profile proves a visible page
  greater than zero was consumed.

Those remain `DRIFT`, `UNCHECKED`, or `UNVERIFIED` as appropriate. They do not
become exact merely because two Rust captures match.

## Alternatives Considered

### Extend `ShellCaptureSession`

This would reduce the number of top-level hooks, but it would mix tactical
startup, simulation commands, and state diagnostics into a sealed main-menu
schema with an exact shell-only key set. Rejected because it weakens existing
evidence and creates overlap with active shell work.

### Separate tactical session sharing final readback

Chosen. It preserves the shell contract, exercises the real app/loading/sim/render
route, keeps diagnostic authority in the app/tooling layers, and can grow only
through explicit named tactical profiles.

### Offscreen or test-only renderer

Useful for unit tests of packing, transforms, and invariants, but not acceptable
as the parent oracle. It bypasses the winit surface, production initialization,
loading transitions, swapchain, hidden-window pump, and full app composition.
Rejected as the completion gate.

### Preplaced fixture or forced radar state

A synthetic map with preplaced structures, direct entity insertion, extra cash,
or a forced radar flag would be faster. It would bypass the producer chain whose
handoffs the parent needs to trust. Rejected for the ordinary-radar checkpoint.

## Validation Strategy

Before feature implementation:

- reconcile exact current `dev`, every worktree/branch, live tasks, claims,
  Cargo/rustc ownership, and Ghidra identity;
- run source-required System Map validation;
- create one unique feature branch/worktree for this prerequisite;
- write and adversarially review an executable implementation plan.

Feature validation, serial under the global Cargo lease:

1. focused Rust unit tests for request/profile validation, exact one-tick pumping,
   state-machine transitions, ordered placement search, readiness, focus/input
   rejection, environment provenance, fail-closed cases, transactional
   publication fault injection, and immutable manifest validation;
2. existing shell-capture tests to prove the sealed contract did not change;
3. focused Python tactical-wrapper tests;
4. `cargo check -q -p vera20k`;
5. `cargo build --bin vera20k`;
6. two fresh hidden Soviet runs;
7. two fresh hidden Yuri runs;
8. assert each pair has identical manifest-stable state and frame bytes while
   excluding only explicitly nondeterministic host timestamps/paths;
9. inspect literal child exit status, capture classification, state ledger,
   process cleanup, and artifact set;
10. independent code/evidence review with no unresolved P0/P1 finding.

After guarded merge, rerun the focused tests, check/build, and one fresh pair per
profile from merged `dev`. Then release Cargo/dev explicitly and resume the
parent radar correction from that validated `dev`.

The parent correction must rerun the complete tactical profiles and compare
named radar/minimap regions as regression evidence. A changed frame is expected
when correcting known drift; approval depends on the verified asset/geometry
contract and production state ledger, not on preserving the old Rust pixels.

## Approval Review

### Why should this be approved?

- It unlocks autonomous validation of a persistent, ordinary, faction-visible
  radar defect without desktop takeover.
- It drives the real accepted startup, deployment, production, placement, power,
  radar, and final-render path instead of injecting the desired state.
- It keeps capture mechanics outside deterministic simulation ownership.
- It preserves the sealed shell checkpoint and shares only the proven final
  readback primitive.
- It is bounded to two named stock profiles and one parent oracle.

### What evidence could still make it wrong?

- The scripted stock fixture may expose an existing load/production/placement
  blocker before radar becomes online.
- The current runtime advance path may not support exact one-tick pumping without
  a carefully factored app-layer mode.
- AI behavior or asynchronous asset readiness may make the chosen profile
  nondeterministic.
- A final Rust frame cannot decide native pixel correctness without a separately
  valid native reference.
- One map/resolution may hide errors visible elsewhere.

### Resolution of load-bearing objections

- Do not weaken readiness or inject state if the producer loop stalls; report the
  earliest real blocker and suspend only for the smallest required correction.
- Prove exact-one-tick pumping with focused tests and recorded tick deltas before
  trusting repeated pixels.
- Pin every launch input and fail on AI/profile nondeterminism rather than
  silently changing the scenario.
- Classify the output as production-path regression evidence, never exact parity.
- Keep broader map/resolution/transition coverage as honest residuals after this
  named checkpoint is green.

With those constraints, the design is approved for implementation planning once
the active shell task releases Cargo and dev integration.
