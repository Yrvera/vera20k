# Shell Substrate Slice 5b - In-Game Options Modal Design

Status: approved design. Plan/design only; no Rust implementation in this pass.
Date: 2026-06-02

## Goal

Implement the active-YR in-game Options shell dialog path as Rust-native UI/app code while preserving the verified `gamemd.exe` semantics for `0xBBB` / `0xF5` chrome, control behavior, persistence, and modal pump behavior.

## Architecture Context

The current Rust shell substrate already has the high-level modal identity but not the Options dialog surface.

- `src/ui/shell/modal.rs` has `ModalKind::InGameOptions`, `template_id(true)=0x0BBB`, `template_id(false)=0x00F5`, and own-proc result tests for result `1` vs `2`.
- `src/ui/shell/controller.rs` owns stack/keyboard/button routing for the message-box modal family, but it is button-oriented and does not model Options trackbars or checkboxes.
- `src/app_sim_tick.rs` owns app-layer fixed-step simulation through `advance_fixed_simulation`; it has no modal-pump session-mode gate yet.
- `src/render/skirmish_shell_chrome.rs` already loads several shell assets used by skirmish and message-box modals, including `SDBTNANM`, `MNBTTN`, checkbox PCXs, and trackbar PCXs. It does not load the verified active Options type-2 `SIDEBTTN.SHP` role through `SIDEBAR.PAL`.
- `src/app_skirmish_shell_render.rs` and `src/ui/skirmish_shell/*` contain proven checkbox and trackbar geometry/input patterns, but those modules are skirmish-specific and must not become the owner of `0xBBB` / `0xF5`.
- `src/app.rs` still uses an egui in-game pause menu and an egui main-menu Options placeholder. Neither is the native in-game Options path.
- `src/util/ini_writer.rs` provides a reusable low-level INI updater; current application persistence only writes `[Audio] ScoreVolume` through `src/audio/music.rs`.

The main architecture rule is unchanged: `sim/` must not depend on shell, UI, render, audio, sidebar, or network. The Options modal pump decision belongs in the app layer and may call the existing fixed-step entry point; `World::advance_tick` must remain unchanged.

## Impact Analysis

Primary affected surfaces:

- `ui/shell`: add an Options-specific state/layout/control model next to the existing modal substrate.
- `render`: add a type-2 Options button asset role for `SIDEBTTN.SHP` / `SIDEBAR.PAL`; reuse trackbar/checkbox primitives only where verified.
- `app`: replace or bypass the egui in-game pause/options path when opening native Options; route pointer/key input to the Options state; close on native results.
- `app_sim_tick`: add a pure modal pump action decision and service wrapper, reusing `advance_fixed_simulation` only for the verified advance branch.
- `util` / app persistence: add whole-options `RA2MD.INI` write support, not just single-key audio persistence.

Risk areas:

- Treating `0xBBB` as another skirmish `0x102` screen would reproduce the stale assumption that active Options buttons use `SDBTNANM`; this is wrong.
- Treating `0xF5` as a derived copy of `0xBBB` would lose its wider sliders and shell-only controls.
- Confusing message-box result codes with the Options own-proc convention would invert persistence.
- Pushing session-mode or modal state into `sim/` would violate layering and determinism boundaries.
- Implementing active `0x52C` / `0x52D` as plain close buttons without carrying the `g_GameState` 4/6 transition request would leave a visible parity gap. The downstream sound/keyboard dialogs are not fully scoped by the current reports and need either a follow-up trace or an explicit deferred blocker.

## Chosen Approach

Approach A: a dedicated Options shell surface.

This design adds an Options-specific UI model and renderer while reusing low-level primitives where the research proves the same native callback family. It deliberately avoids both extremes:

- It does not build a broad generic Win32 dialog engine before the project needs one.
- It does not fold Options into the skirmish `0x102` module, because the active button art, control set, and anchoring rules differ.

Rust-native ownership:

- `ui/shell/options*` owns dialog-local control state, native IDs, template-specific layout, input, and native result production.
- The app layer owns when the dialog opens, when result `1` applies/writes, when result `2` skips, and when fixed sim is advanced behind the modal.
- Render owns asset decoding and sprite emission. It consumes the Options layout/state but does not own persistence or sim decisions.
- `sim/` remains unaware of modal/session UI.

## Tiny-Detail Ledger

Each item is a design constraint.

- L1: Active byte equality selects the template: `0x00A8E9A0 == 1` means `0xBBB`, all other values mean `0xF5`. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L2: `0xF5` is a distinct resource: 148-DLU sliders, shell-only `0x50F`, `0x51A`, `0x71C`, and no `0x52C` / `0x52D`. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L3: DLU conversion uses MS Sans Serif 8pt base metrics, baseX `6`, baseY `13`. Source: `DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`.
- L4: Active `0xBBB` buttons `0x52C`, `0x52D`, and `0x686` use owner-draw type 2, `SIDEBTTN.SHP` through `SIDEBAR.PAL`, frames `0` released, `1` pressed, `2` timer/highlight. Source: `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`.
- L5: `0xF5` Back `0x686` uses owner-draw type 1, `SDBTNANM.SHP`, frames `2` released, `4` pressed, `3` timer/highlight. Source: `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`.
- L6: `MNBTTN.SHP`, `MAINBTTN.PAL`, and `bue_*` / `bde_*` PCX button pieces are not used for scoped Options buttons. Source: `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`.
- L7: Trackbars and checkboxes use common owner-draw callback families and assets: `trakgrip.pcx`, `trofl/trofm/trofr`, `cue_i.pcx`, `cce_i.pcx`. Source: `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.
- L8: Checkbox label clicks must not toggle; only the 18x18 icon hit toggles. Source: `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.
- L9: `0x52C` and `0x52D` set result `1` only for `WM_COMMAND` hiword `0` and active byte `== 1`; they also set `g_GameState` 4 and 6 respectively. `0x686` sets result `1` unconditionally. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L10: Result `1` applies controls then writes `RA2MD.INI`; result `2` skips both. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L11: `0x529` GameSpeed and `0x52A` ScrollRate use visual/internal inversion `6 - pos`; `0x52B` VisualDetails and `0x50F` Difficulty are direct `0..2`. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L12: Active network GameSpeed changes queue command `0x0D` when the queue permits and do not immediately overwrite the local field; offline/inactive stores directly. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L13: `0x601` writes Options `+0x1E` UnitActionLines, `0x604` writes `+0x1F` ShowHidden, `0x602` writes `+0x20` ToolTips. ToolTips also updates the manager only when active and manager exists. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L14: `0x51A` ScrollCoasting is resource-present in `0xF5` but is not read or written by `0x004E1FE0` / `0x004E1DE0`. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L15: `FUN_00623120` calls `Process_NetworkMessages` first. Offline modes `0` and `5`, or blocker globals, run network-service-only and skip `Main_Tick`. LAN/WOL modes `3` and `4` can call `Main_Tick` only when blockers and `DAT_00ABCD58` permit. Source: `MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`.
- L16: Offline in-game Options freezes world/frame advancement while the dialog remains message-responsive. Source: `MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`.
- L17: `DAT_00ABCD58` is a `Main_Tick` reentrancy byte, not a user pause flag. Source: `MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`.
- L18: Init applies runtime visibility/enabled gates: hide `0x529` / `0x714` / `0x671` when `g_GameMode == 0 && 0x00A8EDDC == 0`, hide the same trio when `0x00A8B538 != 0`, and enable active `0x52D` from `FUN_00407000()`. Source: `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`.
- L19: Trackbar input must preserve the common callback's native gates: the top pixels do not start interaction, thumb hits use the 12 px interval, outside-thumb hits remap, and step zero normalizes to one. Source: `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.
- L20: Keyboard input must route through the registered dialog path before any Rust global ESC/pause path. Exact active-Options Enter/Escape result translation is not proven by the current Options reports and must be implemented only if verified or marked `UNCHECKED`. Source: `MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`, `VALIDATION_MODAL_0X005D3490_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md` for the generic dialog-route mechanism.

## Design

### Components

`ui/shell/options.rs`

- Defines `OptionsTemplate`, `OptionsControlId`, `OptionsDialogState`, `OptionsValues`, `OptionsResult`, and dialog-local interaction state.
- Knows native control IDs, runtime visible/enabled state, and result production.
- Accepts app-supplied gate inputs for the verified init conditions instead of hardcoding app globals inside UI code.
- Does not write files, play audio, advance sim, or invoke render.

`ui/shell/options_layout.rs`

- Stores separate parsed DLU control tables for `0xBBB` and `0xF5`.
- Converts to pixel rects via `ui/shell/geom.rs`.
- Applies verified Options resize/anchoring helper behavior where required by the chrome report.

`app_ingame_options.rs` or an equivalent app module

- Opens the dialog from in-game input.
- Builds initial `OptionsValues` from current app configuration.
- On result `1`, applies values and writes `RA2MD.INI`.
- On result `2`, closes without apply/write.
- Converts active `0x52C` / `0x52D` results into pending next-dialog transitions. If sound/keyboard dialog behavior remains unresearched at implementation time, this must stay a named deferred parity blocker rather than being silently treated as Back.

`render/skirmish_shell_chrome.rs` or a renamed shared shell chrome atlas

- Adds `SIDEBTTN.SHP` frames `0..=2` decoded with `SIDEBAR.PAL`.
- Keeps `SDBTNANM` for shell `0xF5` Back and existing right-panel shell controls.
- Keeps `MNBTTN` only for mode-2 message-box modals.

Options render emitter

- Draws the full-shell Options control set using the Options layout/state.
- Uses type-2 button art for active `0xBBB`.
- Uses type-1 Back art for `0xF5`.
- Uses existing trackbar/checkbox primitive helpers after they are generalized out of skirmish-specific names or called through a small shared utility.

`app_sim_tick.rs`

- Adds `SessionMode` plus a pure pump decision returning an action, not just a boolean.
- Distinguishes message+network-service-only, message+advance-fixed-sim, and message-only reentrancy/deferred cases.
- Adds a wrapper service path that calls the existing `advance_fixed_simulation` only for the advance action.

Persistence module

- Adds whole-options write support for `RA2MD.INI`.
- Uses `util::ini_writer` for low-level update mechanics.
- Does not reuse the ScoreVolume-only audio helper as the owner of general Options persistence.

### Interfaces / Contracts

Options state contract:

- Initialization receives `OptionsValues` plus template selection.
- Initialization applies verified visible/enabled gates for GameSpeed controls and active Sound.
- Input mutates only dialog-local values.
- Hidden or disabled controls never produce commands or value changes.
- Closing returns `OptionsResult::Persist`, `OptionsResult::GameEndedNoPersist`, or a pending next-dialog request paired with `Persist`.

Render contract:

- Renderer receives immutable layout/state plus atlas entries.
- Renderer is responsible for asset/frame selection, text placement, and control z-order.
- Renderer never changes Options values.

Pump contract:

- App layer decides the pump action before touching sim.
- Every action services shell/dialog input and repaint first.
- Offline modes `0` and `5`, or the blocker globals, take network-service-only and leave `World.tick` unchanged.
- LAN/WOL modes `3` and `4` advance through the existing fixed sim path only when blockers are clear and the reentrancy byte is false.
- Reentrancy skip is message-only: it must not explicitly call the network service after refusing `Main_Tick`.
- Legacy/unknown modes remain explicit deferred/no-advance cases until researched.

Persistence contract:

- Result `1` applies before write.
- Result `2` performs neither apply nor write.
- Full Options object write is the target, not changed-key write only.

### Data Flow

1. In-game input opens `OptionsDialogState` for `0xBBB`.
2. The app snapshots current option values into dialog-local state.
3. The render path emits the active shell dialog over the frozen last battlefield frame for offline modes.
4. Input updates local state and live labels.
5. On `0x686`, result `1` applies and writes.
6. On `0x52C` / `0x52D`, result `1` applies/writes and records the verified state transition request.
7. If the pump reports game end, the app produces result `2` and skips persistence.

### Error Handling

- Missing `SIDEBTTN.SHP` or `SIDEBAR.PAL` should fail the native Options render path with a visible/logged asset error, not silently substitute `SDBTNANM` or PCX pieces.
- Missing `RA2MD.INI` should create it through the INI writer path, matching the current single-key writer behavior where practical.
- Unsupported downstream `0x52C` / `0x52D` dialogs must be a named blocked/deferred state, not a silent no-op.

### Testing Strategy

- Unit tests for `0xBBB` vs `0xF5` template selection and exact control sets.
- Layout tests for the parsed DLU rects, including `0xF5` slider width `148`.
- State/init tests for hidden GameSpeed controls and disabled active Sound gates.
- Render asset-selection tests: active `0xBBB` buttons select `SIDEBTTN` frames `0/1/2`; `0xF5` Back selects `SDBTNANM` frames `2/4/3`.
- Trackbar tests for `6 - pos` inversion on `0x529` and `0x52A`.
- Trackbar input tests for y-gate rejection, 12 px thumb-drag start, outside-thumb remap, and step normalization.
- Checkbox tests for icon-only toggle and `BM_GETCHECK == 1` byte writes.
- Keyboard tests that Options consumes/reroutes ESC before the current global pause/egui path; exact Enter/Escape result mapping remains `UNCHECKED` unless separately verified.
- Result tests for `1` apply/write and `2` no apply/no write.
- Persistence tests for all touched and pass-through `WriteToINI` key groups.
- Pump tests for offline network-service-only with `World.tick` delta `0`, LAN/WOL advance only with no blockers/reentrancy, reentrancy message-only/no network-service call, and no `sim/` dependency.
- Focused screenshot/manual stop gate for active Options chrome before merge.

## Architectural Decisions

- Dedicated Options module instead of generic engine: lower blast radius and clearer parity ownership.
- Shared primitives only after proof: checkbox/trackbar helpers can be reused because the callback family is verified; active button art cannot be reused from skirmish.
- App-layer pump: preserves the native authority split and project layering.
- Whole-options persistence: matches native `WriteToINI`, avoids a piecemeal "only changed values" shortcut.

## Alternatives Considered

Approach B, extending skirmish shell state/render, was rejected because it invites the exact drift the new reports corrected: active `0xBBB` buttons are not skirmish type-1 `SDBTNANM` buttons.

Approach C, a broad generic shell dialog engine, was rejected for this slice because it would increase blast radius before the project needs a complete Win32-dialog abstraction.

## Sources

- `docs/research/OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`
- `docs/research/OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`
- `docs/research/MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`
- `docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`
- `docs/research/GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_UI_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/skirmish-ui/SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`
