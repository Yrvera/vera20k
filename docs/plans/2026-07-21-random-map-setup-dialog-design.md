# Random-Map Setup Dialog (0x105) Design

## Goal

Implement the "Create Random Map" settings dialog (gamemd dialog `0x105`, command
`0x583`) pixel-matched onto the existing Choose Map (`0x6B`) modal frame, wiring its OK
path to write `RandMap.Sed`, upsert the sentinel, and commit the selection so the
already-working `.SED` launch generator runs — the full RA2 create-random-map experience,
minus the in-box terrain preview image (a named, deferred follow-up).

## Scope decision

**Approach B (chosen):** faithful interactive dialog + exact state machine + faithful
narrow Randomize + write/commit/launch. The `0x468` preview box stays **empty** in v1;
Generate still runs, gates OK, and shows the "Working" overlay — it just draws no terrain.

**Named parity gap (user-accepted):** native draws a downscaled terrain image + start
markers into `0x468` (`DrawStartPositions` / `GenerateTerrainPreview`). v1 omits the image.
Follow-up: rasterize the generated `MapFile` into the box per the
`GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS` report. This is a **phase**, not a cut —
the box, the Generate action, and the OK-gate are all present; only the pixels inside the
box are deferred.

## Architecture Context

The skirmish shell is **custom wgpu sprite batches, not egui**. The `0x6B` Choose Map
modal is the reuse template:

- **State:** `ChooseMapModalState` (`src/ui/skirmish_shell/state/choose_map.rs:21`),
  held as `Option<…>` on `SkirmishShellState.choose_map_modal` (opened `app.rs:1177`,
  cleared `app.rs:1203`).
- **Geometry:** `compute_choose_map_modal_layout` (`src/ui/skirmish_shell/layout.rs:559`)
  builds a `ChooseMapModalLayout` of pixel rects from DLU rects via `geom::dlu_rect`
  (`src/ui/shell/geom.rs:77`) — `RectPx(mul_div_round(x,6,4), mul_div_round(y,13,8), …)`,
  round-half-up (`DLU_BASE_X=6`, `DLU_BASE_Y=13`). Frame constants at `layout.rs:36`
  (`CHOOSE_MAP_MODAL_W=533`, `_H=369`).
- **Draw:** `push_choose_map_modal_instances` (`src/app_skirmish_shell_render/modals.rs:102`)
  emits `SpriteInstance`s: background SHP `MnScrnLCustomizeBattle.shp` (only at 800-wide),
  a full-screen `SHELL_MODAL_BG_RGB` fill + dark bevel, the two listboxes, the three
  right-column SDBTNANM buttons (frame 2 idle / 4 pressed), and the preview outline. Text
  is a parallel `ShellTextDraw` pass.
- **Command dispatch:** no central WndProc table — a press-must-match-release gesture.
  Mouse-down records `modal.pressed_button` (`app.rs:1291`); mouse-up fires only if
  press==release (`app.rs:1321`) and matches the `ChooseMapModalButton` enum
  (`layout.rs:159`: `UseMap0x6c5`, `Cancel0x5c0`, `CreateRandomMap0x583`) at
  `app.rs:1330`. **`CreateRandomMap0x583` is a `log::info!` stub at `app.rs:1337`.**
- **Commit + launch seam (already wired):** UseMap → `commit_choose_map_selection`
  (`app.rs:1208`) sets `selected_map_file` and clears the preview texture. At match start,
  `selected_map_file` → `map_load` → **`is_seed_selection` gate** (`app_init.rs:343`;
  `is_seed_selection` at `map/rmg/mod.rs:182` — true for any `*.SED`) → build `RmgOptions`
  → `options.apply_sed(ini)` from the seed file (`app_init.rs:355`) → resolve theater +
  `ResolvedTheaterInputs::from_theater` + `TheaterTileBlocks::build` →
  `build::generate_map(…)` (`app_init.rs:391`). **This whole path already works and is
  verified in-game this session.**
- **Bridge helpers (exist, unwired):** `upsert_random_map_sentinel`
  (`src/skirmish_scenarios.rs:221`; sets file `RandMap.Sed`, official=true, min 2, max 4)
  and `ChooseMapModalState::create_random_map` (`choose_map.rs:149`, calls the upsert) have
  **no production caller** — they dead-end exactly at the `app.rs:1337` stub.
- **Options model (done):** `RmgOptions` (`src/map/rmg/options.rs`) carries all 16 fields;
  `default()` = the native constructor defaults, `normalize()` = the `0x005975E0` clamps,
  `apply_sed`/`to_sed_bytes` = the `[RandomMap]` `.SED` read/write. `seed_u16(-1)` guards
  to `0`.

## Impact Analysis

**New surfaces**
- `RandomMapSetupModalState` — a sibling `Option<…>` on the shell state, mirroring
  `ChooseMapModalState` (holds the working `RmgOptions`, the enable/disable flags, the
  pressed control, the open combo, and `previous_selection` for cancel-restore).
- `0x105` layout function next to `compute_choose_map_modal_layout` — reuses the shared
  frame + right column, adds the left-column control rects from the ledger.
- Left-column draw in `modals.rs` — reuses `controls.rs` combo/trackbar/button primitives.
- Accept path: `0x583` arm opens the dialog; the dialog's OK writes `RandMap.Sed`, calls
  `upsert_random_map_sentinel` + selection-commit.

**Touched**
- `app.rs` `CreateRandomMap0x583` arm (open dialog instead of logging); new mouse
  down/up handlers for the setup modal (siblings of the choose-map handlers).
- `app_skirmish_shell_render.rs` dispatch (draw the setup modal when open).

**Untouched (already done)** — `RmgOptions`, `build::generate_map`, the `.SED` launch
branch, the sentinel record type, PCX decode. Backend risk ≈ 0.

**Blast radius** — the setup modal is a new optional state parallel to the choose-map
modal; when open it suppresses choose-map input the same way choose-map suppresses the
board. No `sim/` changes, no determinism/state-hash impact (UI-only until launch, and
launch already regenerates deterministically from the seed).

## Tiny-Detail Ledger (parity constraint set — carried to /write-plan)

### Geometry & composition — `[doc: …0X105_LAYOUT_GEOMETRY §3, §9]`
- Frame: `DLGTEMPLATEEX` 533×369 DLU, 8pt MS Sans Serif — **identical to `0x6B`**. Reuse
  frame + `MnScrnLCustomizeBattle.shp` background + `SHELL_MODAL_BG_RGB` fill + bevel +
  right column + preview outline. **No new asset.** DLU→px via the existing `dlu_rect`.
- Left-column labels, x=74 cx≈93: Environment (y40 cy14), TimeOfDay (y64 cy14),
  Theater (y90 cy12), MapSize (y114 cy12), Resources (y138 cy12), Players (y162 cy14).
- Left-column controls, x=179 cx=150: map-type/environment combo `0x405` (y41 cy103),
  time combo `0x3EA` (y65 cy101), theater combo `0x407` (y90 cy103), size combo `0x406`
  (y114 cy103), resources combo `0x408` (y138 cy103), **players trackbar `0x3EB`**
  (y163 cy13, `msctls_trackbar32` — a slider, NOT a spin).
- Seed edit `0x3FB` @ (279,287,50,12) — `WS_DISABLED`, style `0x48002000` (no
  `WS_VISIBLE`): display-only, populated by display-sync set-text. `[open: is it drawn at
  all in the standard flow? style lacks WS_VISIBLE — confirm during planning whether to
  render the seed field or leave it hidden.]`
- Randomize `0x621` @ (74,257,83,15) `GUI:SurpriseMe`; Generate `0x620` @ (246,257,83,15)
  `GUI:PreviewMap`.
- Right column (shared w/ `0x6B`): UseMap `0x6C5` @ (422,122,108,23) `GUI:UseMap`;
  LoadMap `0x6C2` @ (422,149) `GUI:LoadMap`; SaveMap `0x6C3` @ (422,176) `GUI:SaveMap`;
  DeleteMap `0x6C4` @ (422,203) `GUI:DeleteMap`; Cancel `0x5C0` @ (423,346,108,23)
  `GUI:Cancel`.
- Preview box `0x468` @ (430,23,96,69). Title `0x694` @ (422,1,108,10) `GUI:GenerateMap`.
  Bottom blank `0x695` @ (2,355,303,12).
- Hidden progress: static `0x638` @ (74,219,150,11) `GUI:WorkingPleaseWait`; button
  `0x639` @ (229,217,100,21) — shown only during Generate.
- Captions resolve through the port's CSF loader (build-time), same as other shell text.

### Defaults & clamps — `[doc: …CONTROLS_OPTIONS §5.1, §5.3]` (already in `RmgOptions`)
- Constructor defaults = `RmgOptions::default()`: theater 0, map_type 1, resources 1,
  ruggedness 0, time 1, water 0, players 2, tiberium 0→clamp 1, tib-layout 0, veg 0,
  urban 0, width 0, height 0, accessibility 0, region 0, seed −1, description CSF `0xF5E`.
- Clamps = `normalize()`: resources/time/width/height 0..3; map_type 0..4; percents
  0..100; players 2..8; tiberium 1..100; seed 0..0xFFFF. Theater is never clamped.

### State machine — `[doc: …CONTROLS_OPTIONS §5.2, §6]` (the observable behavior)
- **On open (WM_INITDIALOG `0x497`):** if `seed == -1` → `seed = RandomRanged(0,0xFFFF)`
  and mark dirty (the port's `default()` seed is −1; **must randomize on open** or launch
  uses seed 0); run display-sync; **OK `0x6C5` starts DISABLED**; **Save `0x6C3` starts
  DISABLED**; Load `0x6C2` / Delete `0x6C4` enabled only if saved seeds exist (empty →
  disabled); Generate `0x620` enabled (native gates on `g_IsMapEditor==0`; the port has no
  map editor → always enabled).
- **Any option change → dirty → OK disabled** until the next Generate.
- **Generate `0x620`:** sync fields → **disable ALL interactive controls incl. Cancel** →
  show `0x638` "Working Please Wait" → run generation → re-enable controls → copy seed
  snapshot → invalidate/paint. **Result: OK becomes enabled.** (v1: generation "marks
  generated" + flashes the overlay; the box stays empty. The preview follow-up upgrades
  this to run `build::generate_map`, cache the `MapFile`, and rasterize into `0x468`.)
- **Randomize `0x621`:** sync → randomize the narrow subset (below) → **destroy the
  preview / clear the generated flag** → **disable OK `0x6C5` and Save `0x6C3`** →
  invalidate.
- **OK `0x6C5`:** sync; if a preview/generated state exists accept immediately, else force
  a Generate first; write **result 1**.
- **Cancel `0x5C0`:** write **result 2**; no commit.

### Randomize subset — `[doc: …CONTROLS_OPTIONS §6.1–6.2]` (narrow, not "randomize all")
- `theater = (RandomRanged(0,100) > 0x31)` → **only 0 or 1** (temperate or snow).
- `map_type = RandomRanged(1,4)`.
- `time = RandomRanged(0,3)`; `resources = RandomRanged(0,3)`;
  `width = height = RandomRanged(0,3)` (one draw, both fields).
- Derived from map_type via `0x00597260`: water amount, ruggedness, urban presence,
  accessibility, region size, **tiberium = resources × 20 (`0x14`)**, tiberium layout,
  vegetation, seed. Vegetation: min & max each clamped 0..100; if `max < min` set
  `min = max`; then `RandomRanged(min,max)`.
- `seed = RandomRanged(0,0xFFFF)`; description = CSF `0xF5E`; then `normalize()`.
- **`RandomRanged(min,max)` is inclusive** on both ends — the port's implementation must
  match (e.g. map_type ∈ {1,2,3,4}). The exact RNG *instance/stream* for this UI-time
  randomizer is a low-priority parity detail (the map itself is decided by the generator
  seed RNG, already done); use a documented instance and inclusive semantics.

### Commit & launch — `[doc: …0X583_IMPLEMENTATION_CONTRACT §2–10]`
- `0x583` **opens the modal**; side effects happen **only if the dialog result == 1**.
- **Result 1:** write the working `RmgOptions` (normalized) to `RandMap.Sed`
  (`to_sed_bytes`) at the path the `.SED` loader resolves `selected_map_file` from
  (`[confirm the write/read path in app_init/app_loading during planning]`); upsert
  **exactly one** sentinel (`upsert_random_map_sentinel`: file `RandMap.Sed`,
  official=true, min 2, max 4); commit selection so `selected_map_file == "RandMap.Sed"`
  via the same path `commit_choose_map_selection` uses. (v1 defers refreshing the
  `RandMap.img` preview source — that is the preview follow-up.)
- **Cancel / result ≠ 1:** restore the previous choose-map selection and preview
  untouched; no `RandMap.Sed`, no sentinel change.
- **Launch:** the existing `.SED` branch (`app_init.rs:343`) regenerates from the seed —
  **no launch-side change**, provided `RandMap.Sed` is written where the loader reads it.
  (Native double-generates too: preview at dialog-time, gameplay at launch, both from the
  same seed — deterministic, so identical output.)

## Design

### Components

1. **`RandomMapSetupModalState`** (`src/ui/skirmish_shell/state/random_map_setup.rs`, new)
   - Fields: `options: RmgOptions` (the working copy), `generated: bool` (preview/OK gate),
     `open_combo: Option<SetupCombo>`, `pressed_button: Option<RandomMapSetupControl>`,
     `saved_seeds_available: bool`, `previous_selection: Option<ChooseMapSelection>` (for
     cancel-restore), plus scroll models for the open combo (reuse `ScrollModel`).
   - Methods: `open(previous_selection)` (seed-randomize-if-−1 + init enable flags),
     `set_option(field, value)` (clamps via `normalize`, clears `generated`),
     `randomize()` (the §6.1 subset + `0x00597260` derived table), `generate()` (mark
     generated; preview follow-up runs the generator), `accept()` → `AcceptResult` /
     `cancel()`.
2. **`RandomMapSetupControl`** enum — variants named with the gamemd ids (`MapType0x405`,
   `Theater0x407`, `Size0x406`, `Resources0x408`, `Time0x3ea`, `Players0x3eb`,
   `Randomize0x621`, `Generate0x620`, `Ok0x6c5`, `Cancel0x5c0`, `Load0x6c2`, `Save0x6c3`,
   `Delete0x6c4`), plus a hit-test `random_map_setup_control_at`.
3. **`RandomMapSetupLayout`** + `compute_random_map_setup_layout` (in `layout.rs`) — reuses
   the shared frame/right-column geometry from the choose-map layout; adds the left-column
   rects from the ledger. A resource-geometry test mirrors
   `row_combo_rects_match_800x600_resource_geometry`.
4. **`push_random_map_setup_modal_instances`** (`modals.rs`) — same background + fill +
   bevel + right column + preview outline as choose-map; draws the five combos, the players
   trackbar, Randomize/Generate, the disabled seed field, and (when generating) the
   `0x638` overlay. Reuses `controls.rs` primitives.
5. **The `0x00597260` derived-field table + `RandomRanged`** — a UI-time randomizer
   distinct from `RmgRng`. Small standalone module (`random_map_setup` submodule fn), fully
   testable, no engine deps.
6. **`0x583` wiring** in `app.rs` — the arm opens the modal with the current selection as
   `previous_selection`; new `handle_random_map_setup_modal_mouse_down/up` mirror the
   choose-map handlers; on `accept()==Committed`, run write-`.SED` + upsert + commit.

### Interfaces / Contracts

- `RandomMapSetupModalState::accept() -> AcceptResult { Committed(RmgOptions) | NeedsGenerate }`
  — `Committed` only when `generated` (or after forcing a generate); the app layer then does
  the file write + upsert + commit. Cancel returns nothing and the app restores
  `previous_selection`.
- The write step calls `options.normalize()` then `to_sed_bytes()`; the upsert reuses
  `upsert_random_map_sentinel`; the commit reuses the `commit_choose_map_selection` path so
  the sentinel's `RandMap.Sed` file name flows to `selected_map_file` untouched.
- Layout contract: `compute_random_map_setup_layout` returns pixel rects only; all DLU
  values live in named constants beside the choose-map ones.

### Data Flow

`0x583` click → open `RandomMapSetupModalState` (seed randomized, OK disabled) → user edits
combos/slider (each edit clamps + clears `generated`, disabling OK) → Generate (disable all
+ overlay → mark generated → enable OK) → OK (`accept()` → `Committed`) → app writes
`RandMap.Sed`, `upsert_random_map_sentinel`, commit `selected_map_file="RandMap.Sed"`, close
modal → match start → existing `.SED` launch branch regenerates deterministically from the
seed. Cancel at any point → restore previous selection, no side effects.

### Error Handling

- `.SED` write failure: surface as a shell log + keep the modal open (do not commit a
  selection the launch path can't read). `anyhow` at the app layer.
- Missing theater/tile data at launch is already handled by the existing `.SED` branch;
  unchanged.
- The `generated` gate prevents committing an un-generated setup (native forces a Generate
  in OK — the port mirrors this).

### Testing Strategy

Pure-model tests (no engine spin-up), the contract's proposed names:
- `random_map_setup_opens_disabled_ok_and_randomizes_unset_seed` (seed −1 → 0..0xFFFF; OK
  disabled on open).
- `random_map_setup_option_change_disables_ok_until_generate`.
- `random_map_setup_generate_enables_ok_and_disables_controls` (incl. Cancel during run).
- `random_map_randomize_subset_matches_native` (theater ∈ {0,1}, map_type ∈ {1..4}, derived
  tiberium = resources×20, vegetation min/max rule) — deterministic with a seeded
  `RandomRanged`.
- `choose_map_create_random_map_cancel_preserves_previous_selection`.
- `choose_map_create_random_map_accept_upserts_single_native_sentinel` (one row,
  official=true, min 2, max 4, on repeat accept).
- `choose_map_create_random_map_accept_commits_randmap_sed`
  (`selected_map_file == "RandMap.Sed"`).
- `random_map_setup_layout_rects_match_800x600_resource_geometry`.
- An end-to-end manual `/run` check: open the dialog in the shell, generate, OK, start —
  confirm a map generates (already proven for the launch path).

## Architectural Decisions

- **Follows** the existing shell-modal pattern exactly (sibling `Option<…>` state,
  DLU-rect layout via `geom::dlu_rect`, custom sprite-batch draw, press==release command
  gesture). No new UI paradigm; no egui.
- **Reuses** the `0x6B` frame/background/right column/preview outline verbatim — the RE
  proved they are the same 533×369 template — so the dialog cannot visually drift from the
  chooser.
- **Reuses** the disk-round-trip launch path (`.SED` on disk → `apply_sed` →
  `generate_map`) with **zero launch-side changes**, matching native's own
  write-`RandMap.Sed`-then-reload behavior. (Alternative: carry `RmgOptions` in-session and
  hook the launch — rejected: more code, diverges from native's file side effect, and loses
  the future saved-seed Load/Save/Delete reuse.)
- **Deferred (named gaps, not silent cuts):** (1) the terrain preview image in `0x468`
  (needs the `GENERATETERRAINPREVIEW` color/dimension spec) — the user-accepted phase; (2)
  the saved-seed Load/Save/Delete file-browser UX (`0x6C2/0x6C3/0x6C4`) — the buttons are
  **drawn** for pixel-match but start disabled exactly as native's empty-saved-seed state
  (Save disabled on init; Load/Delete disabled when no saved seeds), matching native
  observable behavior; the file-browser flow is a separate feature per the contract.

## Alternatives Considered

- **Approach A (full incl. live preview)** — everything in B plus rasterizing the generated
  `MapFile` into `0x468` now. Rejected for v1 (user choice) to land a faithful playable loop
  first; kept as the immediate follow-up.
- **Approach C (minimal commit path)** — OK writes+commits with no Generate step or preview.
  Rejected: drifts from native's observable OK-disabled-until-Generate state machine.
- **egui dialog** — rejected up front: the shell is pixel-parity custom-draw; egui would
  visibly diverge from every other shell dialog. (This was the original design fork; the
  user chose pixel-match and the `0x105` layout RE that unblocks it.)
- **In-session options carry (no `.SED` write)** — rejected as above.
