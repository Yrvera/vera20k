# Skirmish Start Game Handoff Session Packing - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x006AE2C0`, `0x006AE3F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Start Game control `0x617` from the offline Skirmish dialog command handler through `FUN_006AE2C0` exit, including immediate session/global packing required to hand off a standard offline Skirmish launch.  
**Non-Scope:** gameplay spawn placement, scenario load/spawn execution after the shell exits, full country/color/team list population, and multiplayer/network lobby variants except where a branch is explicitly skipped by standard offline Skirmish.  
**Confidence:** High for the verified shell handoff and writes listed below; Medium for human-readable field names where they are inferred from control families and prior reports.  
**Active in YR:** Yes. Evidence: `Main_Game` uses game mode `5` for offline Skirmish per prior `MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md`; `0x006AE2C0` creates dialog `0x102`, proc `0x006AE3F0`, pumps until result `0x617` or `0x5C0`, and `0x006AE3F0` routes `WM_COMMAND` low word to `0x006ACEE0`.

## 1. Overview

Pressing Start Game does not immediately launch from the button click. `0x006ACEE0` disables Start, validates row/player-count/team constraints, asks the selected multiplayer mode/category object to accept the transition, then copies dialog-control state into Skirmish globals and session/node records. Only after that packing does it write the dialog-loop result pointer, allowing `0x006AE2C0` to exit with return value `true`.

**Correction, 2026-05-21:** `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md` rechecked the adjacent start/team control destinations. The validation at `0x006AD16C..0x006AD2A7` is a same-team validation using team controls `0x76D..0x774`, not a start-position collision validation. Start controls are `0x6A3..0x6AB` and write to `DAT_00A8B2DC[slot]` / `DAT_00A8B39C`; team controls are `0x76D..0x774` and write to `DAT_00A8B2FC[slot]` / `DAT_00A8B3A4`.

The handoff is UI/session packing only. No evidence in this slice performs unit placement or MCV spawn logic before `0x006AE2C0` returns.

## 2. Key Offsets / Globals

| Field/global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| dialog result pointer at `GetWindowLongA(hwnd, 8)` | Points to `FUN_006AE2C0` local result; final success writes `0x617` through it | `0x006AE2C0` `SetWindowLongA(...,8,&local_4)`; `0x006AD8C7..0x006AD8D5` writes through saved pointer | Yes - standard offline Skirmish modal loop |
| `DAT_00A8B23C` | selected `MPModes` multiplayer mode/category object, not the scenario/map record | vtable `+0x14` call at `0x006AD2C9..0x006AD2D5`; vtable `+0x28` fallback in `0x004E4170`; selected-object writes in `0x005E7160`; corrected by `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` | Yes |
| `DAT_00A8B250`, `DAT_00A8B254` | selected map/category token and selected scenario index, preserved into launch mirrors | `0x006AD34B..0x006AD369` writes `DAT_00A8B3C4/3C8` | Yes |
| `DAT_00A8B274` | AI/opponent row count where row item data is `0`, `1`, or `2` | `0x006ACFBD..0x006AD052` | Yes |
| `DAT_00A8B27C..A8B2FC` | per-AI arrays copied from active row controls: row kind, country, color, start, team | `0x006AD3C1..0x006AD4E6`; start/team order corrected by `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md` | Yes |
| `DAT_00A8B3C4/3C8` | launch mirror of selected map token/index; index is clamped to `0` if beyond map count | `0x006AD34B..0x006AD36B` | Yes |
| `DAT_00A8B3CC/3D0/3D4` | launch mirrors of top-level sliders/combos `0x529`, `0x511`, `0x50C` | `0x006AD703..0x006AD79E` | Yes |
| `DAT_00A8B3D8..3DC` | launch mirrors of checkboxes `0x54E`, `0x69A`, `0x69D`, `0x693`, `0x696` | `0x006AD7A4..0x006AD889` | Yes |
| `DAT_00A8B260=1`, `DAT_00A8B31D=0`, `DAT_00A8B31F=0`, `DAT_00A8B26C=0` | hard/reset launch-state flags written after option mirrors | `0x006AD88F..0x006AD8A4` | Yes |
| `DAT_00AC1154` | menu preview wrapper destroyed before shell exit | `0x006AD85A`, `0x006AD8AB..0x006AD8BD`; also `0x006AE2C0` teardown | Yes |
| `DAT_00A8DA78/84` | vector/count receiving allocated 0x85-byte local player node record | `0x006AD647..0x006AD6F6` | Yes |

## 3. Core Logic

### Command reachability and return codes

**Verified finding:** `FUN_006AE2C0` initializes local result to `-1`, stores its address at dialog `GWL_USERDATA` offset `8`, and pumps until local result becomes `0x617` or `0x5C0`. It returns `local_4 == 0x617`.  
**Evidence:** `0x006AE2C0` decompile; loop checks `local_4 != 0x617 && local_4 != 0x5C0`; final return compares to `0x617`.  
**Active in YR:** Yes - this is the standard offline Skirmish setup launcher.

**Verified finding:** `FUN_006AE3F0` routes `WM_COMMAND (0x111)` to `FUN_006ACEE0`, passing the command low word and notification high word. Start/Back are processed only when notification is `0`; selection-change notifications are ignored for Start/Back.  
**Evidence:** `0x006AE3F0`; `0x006ACF7B..0x006ACF92` checks command `0x617` and notification; `0x006ACF8C` distinguishes Start from Back.  
**Active in YR:** Yes.

**Verified finding:** successful Start writes `0x617` through the saved result pointer only after all packing and preview cleanup; failed validations re-enable Start and return without writing the result pointer, so the modal loop continues.  
**Evidence:** validation returns at `0x006AD0E0..0x006AD169` and `0x006AD2AD..0x006AD343`; final write at `0x006AD8C7..0x006AD8D5`.  
**Active in YR:** Yes.

### Start validation before packing

**Verified finding:** The Start button is disabled immediately on a `0x617` command with notification `0`. It is explicitly re-enabled only on validation/session-acceptance failures.  
**Evidence:** disable path `0x006ACF92..0x006ACF9E`; re-enable after messages at `0x006AD0CB..0x006AD0DA`, `0x006AD14A..0x006AD159`, `0x006AD298..0x006AD2A7`, and `0x006AD31B..0x006AD32A`.  
**Active in YR:** Yes.

**Verified finding:** AI/opponent count is computed by reading seven row controls `0x50B`, `0x50E`, `0x516`, `0x51A`, `0x51B`, `0x51C`, `0x51D`; rows whose `CB_GETITEMDATA` result is `0`, `1`, or `2` count as active. The count is stored in `DAT_00A8B274`.  
**Evidence:** row loop `0x006ACFBD..0x006AD052`; item messages `0x147` and `0x150`.  
**Active in YR:** Yes.

**Verified finding:** Map capacity check compares selected map capacity from `0x005E6520(DAT_00A8B254)` against `active_ai_rows + 1`. If capacity is smaller, it formats string IDs `0x437/0x438`, shows a modal shell message, re-enables Start, and returns.  
**Evidence:** capacity call `0x006ACFA4..0x006ACFAA`; check/message `0x006AD05B..0x006AD0DA`.  
**Active in YR:** Yes.

**Verified finding:** A second validation rejects games with fewer than two total players (`active_ai_rows + 1 <= 1`), using string IDs `0x43F/0x440`, then re-enables Start and returns.  
**Evidence:** `0x006AD0ED..0x006AD159`.  
**Active in YR:** Yes.

**Corrected verified finding:** If the local player team control family returns an explicit team (`FUN_004E6030(hwnd, 0x76D, -1) >= 0`), the code scans active AI rows and requires at least one active row with a different team value. If every active row has the same team as the local player, it shows string IDs `0x457/0x458`, re-enables Start, and returns. This is not a start-position collision check.  
**Evidence:** explicit-team gate `0x006AD16C..0x006AD17C`; scan/read of `0x76E..0x774` via `FUN_004E5940`/`FUN_004E6030` at `0x006AD184..0x006AD22A`; message `0x006AD240..0x006AD2A7`; correction report `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`.  
**Active in YR:** Yes. The exact visible text belongs to CSF/string-table decoding and was not expanded here.

**Corrected verified finding:** Before session/global packing, the selected `MPModes` mode/category object's vtable `+0x14` is called with a small local result buffer. If it returns false and the produced result code is `0x617`, Start is re-enabled after string ID `0x469` is shown; otherwise the code calls `0x005D5E10` and continues. The concrete `+0x14` methods and rejection cases are resolved in `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`.  
**Evidence:** `0x006AD2BA..0x006AD34B`.  
**Active in YR:** Yes / Conditional by selected mode.

### Values packed from dialog controls

**Verified finding:** Selected map state is mirrored before row packing: `DAT_00A8B3C4 = DAT_00A8B250`; `DAT_00A8B3C8 = DAT_00A8B254`, except if `DAT_00A8B254 >= DAT_00A8B8D8` then `DAT_00A8B3C8 = 0`.  
**Evidence:** `0x006AD34B..0x006AD36B`.  
**Active in YR:** Yes.

**Verified finding:** Control `0x6A0` receives message `0x4B3` with `wParam=0x14` and a local buffer before the local player country is copied through `FUN_004E4170(hwnd, 0x6A1, -1)` into session via `FUN_0069B760(..., value, 1)`.  
**Evidence:** `0x006AD375..0x006AD3BA`; `FUN_0069B760 @ 0x0069B760` writes session `+0x184/+0x188` and mirrors to `+0x174/+0x178`.  
**Active in YR:** Yes.

**Corrected verified finding:** For each active AI row, the code writes five parallel arrays at slot `row+1`: row item data to `DAT_00A8B27C`, country from the `0x510/0x513/0x514/0x51E/0x51F/0x520/0x521` family to `DAT_00A8B29C`, color from the `0x522..0x528` family to `DAT_00A8B2BC`, start from the `0x6A3..0x6AB` family to `DAT_00A8B2DC`, and team from `0x76D..0x774` to `DAT_00A8B2FC`. Inactive rows do not overwrite those arrays in this loop.  
**Evidence:** active-row test and stores `0x006AD3C1..0x006AD4E6`; control-id helpers `0x004E37D0`, `0x004E41D0`, `0x004E4E60`, `0x004E5940`, `0x004E6030`.  
**Active in YR:** Yes. Names for country/color/team/start are inferred from control-family docs and helper behavior; the array writes themselves are verified.

**Verified finding:** A compact eight-row launch table begins at `DAT_00A8B3F0`. For each row, the first field maps row item data to launch type code: `-1 -> 1`, `0 -> 4`, `1 -> 5`, `2 -> 6`, and other values -> `0`; the next two fields are country and color values from dialog control helpers.  
**Evidence:** second row loop `0x006AD4F8..0x006AD5E8`; type-code mapping `0x006AD5B0..0x006AD5DF`.  
**Active in YR:** Yes.

**Verified finding:** Local player color is copied through `FUN_004E4E20(hwnd, 0x6A2, -1)` into session with `FUN_0069B7E0(..., color, 1)`. That helper writes session `+0x17C/+0x180` and mirrors to `+0x15C/+0x160`; when color is `-2`, the helper may randomize to an unused color.  
**Evidence:** `0x006AD600..0x006AD63B`; `FUN_0069B7E0 @ 0x0069B7E0`.  
**Active in YR:** Yes.

**Corrected verified finding:** Local player start/team adjunct values are copied into `DAT_00A8B39C` and `DAT_00A8B3A4`, then into a newly allocated 0x85-byte node record at offsets `+0x5B` and `+0x63`. `DAT_00A8B39C` / `+0x5B` is start; `DAT_00A8B3A4` / `+0x63` is team.  
**Evidence:** globals at `0x006AD63B..0x006AD641`; node writes `0x006AD677..0x006AD69C`.  
**Active in YR:** Yes.

**Verified finding:** The local node record is allocated with size `0x85`, its sub-buffer at `+0x28` is initialized to ten `0xFF` bytes by `0x0053ECB0`, a wide player/name string is copied from `DAT_00A8B380`, and offsets `+0x4B`, `+0x53`, `+0x5B`, `+0x63`, `+0x73` receive `DAT_00A8B3AC`, `DAT_00A8B394`, `DAT_00A8B39C`, `DAT_00A8B3A4`, and `-1`.  
**Evidence:** allocation and writes `0x006AD647..0x006AD69C`; helper `0x0053ECB0`; `0x00735120` converts/copies the string source.  
**Active in YR:** Yes.

**Verified finding:** After node creation, `SessionClass__ProcessRandomAssignments @ 0x0069B8C0` resolves random country/color values in both node records and AI arrays. It uses `Random__RandomRanged(0,9)` for random country and `Random__RandomRanged(0,7)` plus uniqueness checks for random color/start-color fields.  
**Evidence:** call at `0x006AD6F9`; function `0x0069B8C0`; color-use helper `0x0069B600`.  
**Active in YR:** Yes.

**Verified finding:** Trackbar/slider-like controls are read with message `0x400`: `0x529` stores `DAT_00A8B268 = 6 - value` and mirrors to `DAT_00A8EB60`; `0x511` stores `DAT_00A8B25C`; `0x50C` stores `DAT_00A8B270`. All three are then mirrored to `DAT_00A8B3CC/3D0/3D4`.  
**Evidence:** `0x006AD703..0x006AD79E`.  
**Active in YR:** Yes. Prior INI defaults identify `GameSpeed=1`, `Money=10000`, and `UnitCount=10`, but the final values here come from live dialog controls when present.

**Verified finding:** Checkbox controls are read with `BM_GETCHECK (0xF0)` and stored as `checked == 1`: `0x54E -> DAT_00A8B262`, `0x69A -> DAT_00A8B263`, `0x69D -> DAT_00A8B264`, `0x693 -> DAT_00A8B320`, `0x696 -> DAT_00A8B261`. The values are then mirrored to `DAT_00A8B3D8..3DC`.  
**Evidence:** `0x006AD7A4..0x006AD889`.  
**Active in YR:** Yes. Prior INI defaults for this dialog family include `ShortGame=yes`, `FogOfWar=no`, `MCVRedeploys=yes`, etc., but this packing path reads the current controls.

### Preserved/default values

**Verified finding:** `DAT_00A8B3C4` is not read from a control during Start; it preserves the selected map/category global `DAT_00A8B250`. `DAT_00A8B3C8` preserves `DAT_00A8B254` unless the selected index is out of range, in which case it is forced to `0`.  
**Evidence:** `0x006AD34B..0x006AD36B`.  
**Active in YR:** Yes.

**Verified finding:** `DAT_00A8B31F`, `DAT_00A8B31D`, and `DAT_00A8B26C` are forced to `0`, while `DAT_00A8B260` is forced to `1`, independent of dialog controls in this slice.  
**Evidence:** `0x006AD88F..0x006AD8A4`.  
**Active in YR:** Yes.

**Verified finding:** The preview object `DAT_00AC1154` is destroyed before writing the result pointer; `FUN_006AE2C0` also has a cleanup check after the modal loop, so a successful Start path normally clears it in `0x006ACEE0` first and leaves nothing for the outer teardown.  
**Evidence:** `0x006AD85A..0x006AD8BD`; `0x006AE2C0` teardown after the loop.  
**Active in YR:** Yes.

## 4. INI Keys

No INI key is read directly inside the verified Start button packing branch. Relevant defaults for the controls are supplied earlier by dialog/session initialization:

| INI path | Value in YR `rulesmd.ini` | Role in this slice | Active in YR |
|---|---:|---|---|
| `[MultiplayerDialogSettings] Money` | `10000` | Default before `0x511`/money-like control value is read | Yes |
| `[MultiplayerDialogSettings] UnitCount` | `10` | Default before `0x50C` control value is read | Yes |
| `[MultiplayerDialogSettings] GameSpeed` | `1` | Default before `0x529` trackbar value is transformed as `6 - value` | Yes |
| `[MultiplayerDialogSettings] AIPlayers` | `0` | Initial/default AI row count before user row controls are packed | Yes |
| `[MultiplayerDialogSettings] ShortGame` | `yes` | Default for one checkbox-family option before Start reads current controls | Yes |
| `[MultiplayerDialogSettings] FogOfWar` | `no` | TS-legacy fog default is off; if a checkbox maps here, the current control still wins | Conditional - feature default off in standard YR |
| `[MultiplayerDialogSettings] MCVRedeploys` | `yes` | Default for one checkbox-family option before Start reads current controls | Yes |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006AE2C0` -> dialog `0x102` | Creates modal setup and returns true only on `0x617` | `0x006AE2C0` | Yes |
| `FUN_006AE3F0` -> `FUN_006ACEE0` | `WM_COMMAND` dispatch sends low/high command words | `0x006AE3F0` | Yes |
| `0x617` Start validation | Disabled Start, count/capacity/player/start checks, session acceptance | `0x006ACF7B..0x006AD34B` | Yes |
| Selected `MPModes` mode object | vtable `+0x14` accepts/rejects start; `+0x28` can provide default country if control data is out of range | `0x006AD2C9..0x006AD2D5`; `0x004E4170`; `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` | Yes |
| Random assignment processing | Resolves random node/AI country/color before handoff | `0x006AD6F9`, `0x0069B8C0` | Yes |
| Preview teardown | Destroys menu preview object before modal result is written | `0x006AD85A..0x006AD8BD` | Yes |

## 6. Current Rust Implementation Status

Rust currently has a narrower shell handoff. `src/ui/skirmish_shell/state.rs` maps `StartGame0x617` to `SkirmishShellAction::StartGame` and `launch_settings` copies selected map, country, credits, start position, and short-game settings. `src/app.rs` then calls `start_selected_skirmish`.

Missing versus the verified binary slice: row-count validation, map-capacity validation, same-explicit-team rejection, Start-button disable/re-enable semantics, session-object acceptance result, per-row AI arrays, compact launch table, random assignment resolution, checkbox/trackbar mirroring, forced launch-state reset flags, and preview-object teardown before result handoff.

Evidence: `src/ui/skirmish_shell/state.rs:70`, `src/app.rs:547`, `src/app_skirmish.rs:31`, `src/sim/game_options.rs:56`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006AE2C0` modal exit | verified | `0x006AE2C0` | none |
| `FUN_006AE3F0` `WM_COMMAND` dispatch | verified | `0x006AE3F0` | none |
| `FUN_006ACEE0` `0x617` notification gate | verified | `0x006ACF7B..0x006ACF92` | none |
| active AI row count | verified | `0x006ACFBD..0x006AD052` | exact visible row labels out of scope |
| map capacity and min-player validation | verified | `0x006AD05B..0x006AD169` | exact CSF text out of scope |
| same-team validation | verified | `0x006AD16C..0x006AD2A7`; corrected by `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md` | exact player-facing text out of scope |
| selected mode object vtable `+0x14` acceptance | resolved by sibling report | `0x006AD2BA..0x006AD34B`; `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` | none for shell handoff; mode-specific internals are owned by mode reports |
| selected map mirrors | verified | `0x006AD34B..0x006AD36B` | none |
| per-AI arrays | verified | `0x006AD3C1..0x006AD4E6` | full country/team naming belongs to slot 5 |
| compact launch table | verified | `0x006AD4F8..0x006AD5E8` | none for packing shape |
| local player session country/color writes | verified | `0x006AD3A4..0x006AD3BA`, `0x006AD600..0x006AD63B`, `0x0069B760`, `0x0069B7E0` | none |
| 0x85-byte node record | verified | `0x006AD647..0x006AD6F6`, `0x0053ECB0` | downstream consumer out of scope |
| random assignment resolution | verified | `0x006AD6F9`, `0x0069B8C0` | RNG seed/source out of scope |
| top-level sliders and checkboxes | verified | `0x006AD703..0x006AD889`; label mapping resolved by `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md` | runtime gameplay consumers of packed option globals |
| forced launch-state flags | verified | `0x006AD88F..0x006AD8A4` | downstream meaning out of scope |
| preview teardown | verified | `0x006AD85A..0x006AD8BD`, `0x006AE2C0` | none |
| gameplay spawn placement | not-touched | user non-scope | separate scenario-launch/spawn investigation |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does Start `0x617` directly exit the dialog? No; it exits only after validation, session acceptance, packing, preview teardown, and writing the saved result pointer. Evidence: `0x006ACF7B..0x006AD8D5`.

[RESOLVED] OQ-2 - What are the shell return codes? `0x617` means Start success and makes `FUN_006AE2C0` return true; `0x5C0` means Back/cancel and makes it return false. Evidence: `0x006AE2C0`.

[RESOLVED] OQ-3 - Which values come from row controls? Active row kind from `0x50B..0x51D`, country from `0x510/0x513/0x514/0x51E/0x51F/0x520/0x521`, color from `0x522..0x528`, start from `0x6A3..0x6AB`, and team from `0x76D..0x774`. Evidence: `0x006AD3C1..0x006AD5E8`; start/team naming corrected by `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`.

[RESOLVED] OQ-4 - Which values are preserved/defaulted rather than read from controls at Start? `DAT_00A8B250/254` are mirrored from selected-map globals, out-of-range selected index defaults to `0`, and flags `DAT_00A8B31F/31D/26C=0`, `DAT_00A8B260=1` are forced. Evidence: `0x006AD34B..0x006AD36B`, `0x006AD88F..0x006AD8A4`.

[RESOLVED] OQ-5 - Is TS-style fog assumed active? No. The relevant YR INI default is `[MultiplayerDialogSettings] FogOfWar=no`; this slice only observes checkbox packing, not fog runtime behavior. Evidence: `ini/rulesmd.ini` `[MultiplayerDialogSettings]`, `0x006AD7A4..0x006AD889`.

[RESOLVED] OQ-6 - What exact concrete method is vtable `+0x14`, and what rejection cases can it return? It is the selected `MPModes` mode/category object's Start-acceptance method, not a generic session-class method. Battle/ManBattle accept via `0x005D6310`; Siege validates node `+0x6B`; Unholy gates on `DAT_00A8B258`; FreeForAll and Cooperative accept with side effects. Evidence: `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`.

[RESOLVED] OQ-7 - Which exact UI labels correspond to checkboxes `0x54E`, `0x69A`, `0x69D`, `0x693`, `0x696`? `0x54E=GUI:ShortGame`, `0x69A=GUI:SuperWeaponsAllowed`, `0x69D=GUI:BuildOffAlly`, `0x693=GUI:MCVRepacks`, `0x696=GUI:CratesAppear`, with STT tooltip IDs and packed globals mapped in `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`.

[DEFERRED] OQ-8 - How the packed globals/node records are consumed to create houses/units/spawns after shell exit. Category: out-of-scope. Reason: user explicitly excluded gameplay spawn placement beyond shell/session handoff.

## Sources

- Ghidra decompiled/read: `0x006ACEE0`, `0x006AE2C0`, `0x006AE3F0`, `0x004E3320`, `0x004E37D0`, `0x004E41D0`, `0x004E4E20`, `0x004E4E60`, `0x004E5900`, `0x004E5940`, `0x004E6030`, `0x0053ECB0`, `0x0069B600`, `0x0069B760`, `0x0069B7E0`, `0x0069B8C0`, `0x007B66C0`, `0x007B6760`, `0x00735120`.
- Ghidra assembly contexts: `0x006ACF7B..0x006AD8D5`, `0x006AD703..0x006AD889`, `0x006AD8C7..0x006AD8D5`.
- Prior reports used as context, not as ground truth where Ghidra was checked: `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` `[MultiplayerDialogSettings]`.
- Rust status scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish.rs`, `src/sim/game_options.rs`.
