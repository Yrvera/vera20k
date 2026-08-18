# LSLoadMessage Skirmish Loading Text Split - Ghidra Report

**Address(es):** `ScenarioClass__Read_Scenario @ 0x00684620`, `ScenarioClass__Read_Scenario_INI @ 0x00686730`, `ScenarioClass__Full_Init @ 0x00686B20`, `FUN_00643AE0`, `FUN_00643720`
**Investigation Mode:** exhaustive-slice for `LSLoadMessage` / briefing metadata visibility boundary during standard offline Skirmish loading.
**Confidence:** High for the Skirmish negative boundary; Medium for exact campaign loading-text composition because the campaign UI was intentionally not decoded.
**Active in YR:** Yes for the negative Skirmish result (`g_GameMode == 5`); Conditional for campaign/single-player metadata reads (`g_GameMode == 0`).

## Target Question

Determine whether `LSLoadMessage`, `LSLoadBriefing`, `[Briefing]`, or scenario/map loading text is visible in standard offline Skirmish loading, and contrast only enough with campaign/single-player to establish the Skirmish do-not-do boundary.

## Non-goals

- Do not investigate the full campaign loading UI or exact campaign text layout.
- Do not re-open the already-settled `WM_PAINT` mode-2 composition beyond using its no-text boundary.
- Do not decode exact progress surface text/status strings; prior progress-surface reports own that.
- Do not edit Rust, INI files, or other docs.

## Evidence Needed To Mark COMPLETE

- Prove the active standard offline Skirmish mode value and scenario-load path.
- Prove where `LSLoadMessage` / `LSLoadBriefing` / campaign briefing metadata is read.
- Prove whether those reads are reachable under standard offline Skirmish.
- Prove the Skirmish loading setup uses the non-campaign progress path, not campaign briefing metadata.
- Compare current Rust only enough to produce implementation handoff / negative facts.

## Stop Conditions

- Stop after `ScenarioClass__Read_Scenario`, `ScenarioClass__Read_Scenario_INI`, `ScenarioClass__Full_Init`, and progress manager setup prove the boundary.
- Stop before full campaign loading screen drawing or exact progress-surface text/status.
- Stop if the only remaining uncertainty requires a runtime HWND trace and does not affect the `LSLoadMessage` Skirmish negative.

## Verified Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| Standard offline Skirmish is the non-campaign load branch, documented as `g_GameMode == 5`, and reaches `ScenarioClass__Read_Scenario @ 0x00684620` then ordinary maps call `ScenarioClass__Read_Scenario_INI @ 0x00686730` and `ScenarioClass__Full_Init @ 0x00686B20`. | Existing standard Skirmish reports; Ghidra xrefs `0x00683D21 -> 0x00684620`, `0x006849C9 -> 0x00686730`, `0x00686845 -> 0x00686B20`. | Yes |
| `LSLoadMessage` and `LSLoadBriefing` are read only inside the `g_GameMode == 0` campaign/single-player branch of `ScenarioClass__Full_Init`. | Decompile `0x00686B20`; xrefs to strings `LSLoadMessage @ 0x0083DC28` occur only at `0x00687005`, `0x0068702D`, `0x0068705D`; xrefs to `LSLoadBriefing @ 0x0083DC18` occur only at `0x00687072`, `0x00687098`, `0x006870CC`. The same block is under the `if (g_GameMode == 0)` branch. | Conditional: campaign/single-player only |
| Campaign metadata source is `MISSIONMD.INI` section data, not the normal Skirmish map `[Briefing]` section. | `ScenarioClass__Full_Init @ 0x00686B20` opens `MISSIONMD.INI @ 0x00839724` under `g_GameMode == 0`, reads section `ScenarioClass+0x125C` keys `Briefing`, `UIName`, `LSLoadMessage`, `LSLoadBriefing`, location and background keys. Repo `ini/missionmd.ini:11..23` contains `[SOV01UMD.MAP] Briefing=`, `UIName=`, `LSLoadMessage=`, `LSLoadBriefing=`, and `LS*Bkgd*` keys. | Conditional: campaign/single-player only |
| `Briefing` string xrefs split into campaign metadata / save-writing / unrelated shell strings; no `Briefing` xref was found on a standard Skirmish loading draw path. | String xrefs for `Briefing @ 0x00839718`: `ScenarioClass__Full_Init` campaign reads at `0x00686DF1/0x00686E19/0x00686E49`, `FUN_0068AD70` writes `Briefing` when saving/exporting scenario data, and shell `STT` strings are separate UI labels. | No for standard offline Skirmish loading |
| Skirmish loading setup selects the non-campaign progress surface, not campaign LS briefing assets. | `ScenarioClass__Read_Scenario @ 0x006847E1..0x00684800`: `g_GameMode == 0` selects `SPLDBR.SHP`; otherwise selects `PROGBARM.SHP`. Standard Skirmish is `g_GameMode == 5`, so it takes `PROGBARM.SHP`. | Yes for Skirmish |
| The already-verified mode-2 loading background branch contains no map name/status/briefing text. | `LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md` verifies `WM_PAINT_Handler @ 0x00621E90` mode `2` draws one `PUDLGBG*` SHP frame and blits, with no text/progress overlay in that branch. | Conditional for mode-2 branch; negative text fact applies when branch is active |
| Current Rust adds Skirmish loading text not proven in native Skirmish: `"Mission deployment"`, `"Loading..."`, `Map: {map_name}`, and explanatory status. | `src/ui/main_menu.rs::draw_loading_screen`; `src/app.rs` passes `GameScreen::Loading { map_name }` into it. | Rust only |

## Active in YR Labels

- `LSLoadMessage` / `LSLoadBriefing` reader: Active in YR: Conditional, `g_GameMode == 0` only.
- Standard offline Skirmish loading: Active in YR: Yes, `g_GameMode == 5`, non-campaign branch.
- Skirmish visibility of campaign LS strings: Active in YR: No, no reader or draw consumer reached in the standard Skirmish branch.
- Progress surface: Active in YR: Yes, but exact text/status on that surface remains out of this slot.

## Implementation Handoff

| Verified behavior | Evidence | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|
| Standard offline Skirmish does not read or display `LSLoadMessage` / `LSLoadBriefing`; those keys are campaign-only metadata. | `0x00686B20` branch on `g_GameMode == 0`; xrefs to `0x0083DC28/0x0083DC18` only in that branch. | Do not parse/render LS keys for Skirmish loading. | Loading UI / scenario metadata boundary. | Start a stock Skirmish map that contains or is modified to contain `LSLoadMessage`; native-style Skirmish loading still does not show that text. | `skirmish_loading_ignores_lsloadmessage_metadata` | Medium if future campaign loading support incorrectly reuses Skirmish surface. |
| Standard offline Skirmish loading should not show map name or explanatory egui status text. | Negative branch evidence above plus mode-2 no-text report; Rust scan shows map-name/status text in `draw_loading_screen`. | Remove or gate Skirmish map-name/status text from parity loading surface. | `src/ui/main_menu.rs`, `src/app.rs`, future native loading renderer. | Start Skirmish and first/loading frames show native background/progress art without `Map: ...` or explanatory sentence. | `skirmish_loading_does_not_render_map_name_or_egui_status_text` | High for screenshot parity. |
| Non-campaign Skirmish uses `PROGBARM.SHP`, while campaign may use `SPLDBR.SHP` plus LS metadata/backgrounds. | `0x006847E1..0x00684800`; campaign reader in `0x00686B20`. | Keep campaign loading-text support separate from Skirmish progress rendering. | Loading asset resolver / scenario mode dispatch. | Skirmish loads `PROGBARM.SHP`; campaign path may later consume `MISSIONMD.INI` LS keys but Skirmish never does. | `loading_assets_split_skirmish_progbarm_from_campaign_ls_metadata` | Medium if a shared "scenario loading message" abstraction leaks campaign strings into Skirmish. |

## Negative Facts / Do Not Do

- Do not display `LSLoadMessage`, `LSLoadBriefing`, `Briefing=`, or `UIName=` on the standard offline Skirmish loading screen. Evidence: `0x00686B20` reads them only under `g_GameMode == 0`; Skirmish is `g_GameMode == 5`.
- Do not treat repo map `[Briefing]` parsing as a Skirmish loading-screen text source. Native Skirmish loading does not route it into the loading surface in the verified path.
- Do not use `SPLDBR.SHP` or campaign `LS640/LS800*` background keys for standard Skirmish loading. Evidence: `0x006847E1..0x00684800` selects `PROGBARM.SHP` for non-campaign.
- Do not preserve Rust's current `Map: {map_name}` / explanatory egui loading text for Skirmish parity. Evidence: no native Skirmish reader/draw path found for those text surfaces.
- Do not collapse campaign and Skirmish loading into one "scenario loading message" feature; the binary keeps the `MISSIONMD.INI` LS metadata behind the campaign mode branch.

## Remaining Uncertainty

- Exact campaign loading text layout, font, wrapping, and background use remain intentionally unverified.
- Exact progress-surface status text, if any, remains owned by the loading-progress-surface swarm; this report only proves `LSLoadMessage` / briefing / map-name text do not feed standard Skirmish loading.
- A runtime HWND trace could further classify every direct-draw vs child-window progress repaint, but it is not needed for the LS negative boundary.

## Stale Docs / Suggested Wording

- `C:/Users/enok/Documents/ra2-rust-game-docs/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md`: replace broad wording like "Various scenario sub-INI reads (Briefing, UIName, LSLoadMessage, etc.)" with "Campaign-only `MISSIONMD.INI` metadata reads under `g_GameMode == 0` include `Briefing`, `UIName`, `LSLoadMessage`, `LSLoadBriefing`, and LS background/location keys; standard offline Skirmish (`g_GameMode == 5`) skips this block."

## Sources

- Ghidra read-only: `ScenarioClass__Read_Scenario @ 0x00684620`, `ScenarioClass__Read_Scenario_INI @ 0x00686730`, `ScenarioClass__Full_Init @ 0x00686B20`, `FUN_00643AE0`, `FUN_00643720`, `FUN_00643670`, `FUN_0068AD70`.
- Ghidra xrefs/strings: `LSLoadMessage @ 0x0083DC28`, `LSLoadBriefing @ 0x0083DC18`, `Briefing @ 0x00839718`, `MISSIONMD.INI @ 0x00839724`, `SPLDBR.SHP @ 0x0083DA40`, `PROGBARM.SHP @ 0x0083DA30`.
- Repo INI evidence: `C:/Users/enok/Documents/ra2-rust-game/ini/missionmd.ini:11`.
- Existing docs: `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`, `LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md`, `SKIRMISH_START_TO_LOADING_SCREEN_ACTIVATION_GHIDRA_REPORT.md`, `SCENARIO_INIT_DEEP_DIVE.md`.
- Current Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/ui/main_menu.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`.

**Status:** COMPLETE.
