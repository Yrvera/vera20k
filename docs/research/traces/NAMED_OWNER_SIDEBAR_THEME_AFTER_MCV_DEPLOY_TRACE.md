# Named Owner Sidebar Theme After MCV Deploy Trace

**Scenario:** offline/skirmish-style game, local human owner name `Commander`, selected country/side `Russians`/Soviet or `YuriCountry`/Yuri, deploy starting MCV into the matching Construction Yard.

**Trace date:** 2026-05-23

**Scope:** sidebar chrome/theme selection only. Build unlocks, sell survivors, starter-base helpers, and final pixel screenshot capture are out of scope.

**Verdict:** FAIL. Current Rust chooses Allied sidebar chrome for named Soviet/Yuri skirmish owners when `Commander` is not present in `AppState.house_roster`. gamemd's standard YR skirmish path creates the local `HouseClass` from the selected country, sets `g_PlayerPtr` to that house, and side-dependent sidebar/player presentation reads the house's `CountryTypeClass` side/country fields rather than matching the display name against map roster house sections.

## Pipeline

`Skirmish launch session -> Simulation.houses Commander country/side -> MCV deploy preserves owner -> preferred_local_owner_name returns Commander -> current_sidebar_theme -> sidebar_theme_for_owner -> current_sidebar_chrome -> build_sidebar_chrome_instances/draw_passes -> visible sidebar chrome`

## Entry Points

1. `src/app_skirmish.rs:162` `apply_skirmish_launch_session` applies the launch session, clears simulation houses, populates named launch houses, and spawns slot MCVs.
2. `src/app_skirmish.rs:298` `populate_launch_houses` inserts `HouseState` keyed by `slot.owner_name`, with `country=slot.country.country_name()` and `side_index=slot.country.side_index()`.
3. `src/sim/world/world_spawn.rs:621` `deploy_mcv` despawns the MCV and spawns the construction yard using the same owner string.
4. `src/app_commands.rs:586` `preferred_local_owner_name` returns the current playable owner. After deployment, the structure-count path can return `Commander`.
5. `src/app_sidebar_render.rs:405` `current_sidebar_theme` asks `sidebar_theme_for_owner(state, "Commander")`, then falls back to Allied on `None`.
6. `src/app_sidebar_render.rs:421` `sidebar_theme_for_owner` searches `state.house_roster.houses` by `HouseDefinition.name`.
7. `src/app_sidebar_build.rs:37`, `src/app_render/build_instances.rs:732`, and `src/app_render/draw_passes.rs:478` consume the selected chrome/theme for visible sidebar geometry, ready-text tint, and the sidebar texture pass.

Coverage: the scoped theme path is covered. This trace did not inspect unrelated owner/country paths outside sidebar theme.

## Concrete Values

### Stage 1 - Launch Slot Normalization

Rust input: local human owner name `Commander`, selected `LaunchCountry::Russia` or `LaunchCountry::Yuri`.

Rust computation:

- `LaunchCountry::Russia.country_name()` = `Russians`
- `LaunchCountry::Russia.side_index()` = `1`
- `LaunchCountry::Yuri.country_name()` = `YuriCountry`
- `LaunchCountry::Yuri.side_index()` = `2`

Rust output: `Simulation.houses[Commander] = HouseState { side_index: 1, country: Russians }` for Soviet, or `side_index: 2, country: YuriCountry` for Yuri.

gamemd output: standard skirmish Start creates a local `HouseClass` from committed node data and sets local `g_PlayerPtr`. `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md` lines 22-24 and 39-41 verify node country/color consumption and local player house creation in `ScenarioClass__Create_Houses @ 0x00687F10`, active in offline Skirmish (`g_GameMode == 5`). `COUNTRY_SIDE_TYPE_CLASSES.md` lines 205-219 verify `HouseClass+0x34` stores the selected `CountryTypeClass*`.

Verdict: PASS for the Rust launch state shape versus gamemd's selected-country house identity at the side/country abstraction level. Final binary object layout is different by design.

### Stage 2 - Country To Side Mapping

Rust computation:

- `Russians -> side_index 1`
- `YuriCountry -> side_index 2`

gamemd computation: `COUNTRY_SIDE_TYPE_CLASSES.md` lines 177-184 lists vanilla YR side indices: `Russians` are side index `1` (Nod/Soviet), `YuriCountry` is side index `2` (ThirdSide/Yuri). Lines 170-175 and 343-348 verify the side index is assigned through `[Sides]` registration into `CountryTypeClass+0xBC`.

Verdict: PASS.

### Stage 3 - MCV Deploy Owner Preservation

Rust input: entity owner interned as `Commander`.

Rust computation: `deploy_mcv` reads `entity.owner`, despawns the MCV, resolves that owner back to `owner_str`, and calls `spawn_object_at_height(&yard_type, &owner_str, ...)`.

Rust output: deployed Construction Yard owner string remains `Commander`.

gamemd output: not recomputed in this trace. Prior MCV deploy traces cover deploy geometry and feedback, but this trace did not re-decompile the owner transfer branch for deploy. The sidebar-theme mismatch does not depend on deploy changing the owner; it depends on the named owner that reaches the render path.

Verdict: UNCHECKED.

### Stage 4 - Local Owner Selection After Deploy

Rust input: a deployed Construction Yard owned by `Commander`.

Rust computation: `preferred_local_owner_name` checks selected entities first, then structure counts. If the MCV's selection transfers to the yard, selected entity path returns `Commander`. If selection is absent, the structure-count path can return `Commander` after `has_strict_build_option_for_owner` succeeds for the named owner.

Rust output: `preferred_local_owner_name(state) = Some("Commander")`.

gamemd output: the local player is `g_PlayerPtr`, the created local human `HouseClass`, not a lookup by canonical country section name. `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md` lines 39-41 verify `Create_Houses` sets `g_PlayerPtr` for the local human in active standard Skirmish.

Verdict: PASS for the local-owner identity boundary: both paths identify the named human player/house, not the literal country section name.

### Stage 5 - Theme Selection

Rust input: `owner = "Commander"`.

Rust computation:

1. `current_sidebar_theme` calls `sidebar_theme_for_owner(state, "Commander")`.
2. `sidebar_theme_for_owner` searches `state.house_roster.houses.iter().find(|house| house.name.eq_ignore_ascii_case(owner))`.
3. In skirmish launch, `AppState.house_roster` is the map's parsed `[Houses]` roster. `apply_skirmish_launch_session` populates `Simulation.houses`, but `MapLoadResult` keeps the original `house_roster`; no `Commander` `HouseDefinition` is inserted there.
4. Search returns `None`.
5. `current_sidebar_theme` falls back to `SidebarTheme::Allied`.

Rust output:

- Commander + selected Russia -> `SidebarTheme::Allied`
- Commander + selected Yuri -> `SidebarTheme::Allied`

Expected gamemd output:

- Commander + selected Russia -> Soviet sidebar family
- Commander + selected Yuri -> Yuri/ThirdSide sidebar family

gamemd evidence:

- `COUNTRY_SIDE_TYPE_CLASSES.md` lines 205-219: the created local house stores selected `CountryTypeClass*` at `HouseClass+0x34`.
- `COUNTRY_SIDE_TYPE_CLASSES.md` lines 177-184: `Russians -> side 1`, `YuriCountry -> side 2`.
- `COUNTRY_ICON_SHP_SELECTOR_GHIDRA_REPORT.md` lines 51-59 and 100-110 demonstrate sidebar-adjacent faction presentation reads `HouseClass -> CountryTypeClass` fields (`+0x34`, `+0xB8`, `+0xBC`) rather than display name strings; the observer branch is conditional, but the field relationship is active YR data, not TS legacy.
- `SIDEBAR_SYSTEM_GHIDRA_REPORT.md` lines 1061-1067 verifies side-specific sidebar MIX filename families (`SIDEC%02d.MIX`, `SIDEC%02dMD.MIX`) and lines 367-382 verify the draw path consumes loaded sidebar SHPs for visible chrome. Lines 409-420 identify `SIDE1/2/3.SHP` as the chrome pieces.

Verdict: FAIL.

### Stage 6 - Visible Screen Result

Rust output: `current_sidebar_chrome` asks `SidebarChromeSet::for_theme(SidebarTheme::Allied)`, so the chrome atlas uses the Allied `sidec01.mix` family if present. `build_sidebar_chrome_instances` and draw passes then render Allied sidebar pieces; ready-text tint also uses Allied theme.

gamemd output: for a standard Skirmish local player created as Soviet/Yuri, the visible sidebar should follow the selected side family. This trace did not capture final gamemd and Rust framebuffers, so pixel equality is not computed.

Verdict: FAIL for theme-family selection; UNCHECKED for final pixel equality.

## Failures

1. **Theme lookup uses stale map roster instead of simulation house state.**
   - Rust: `src/app_sidebar_render.rs:425-429` searches `state.house_roster.houses` for literal owner `Commander`; no match means `src/app_sidebar_render.rs:408-410` returns Allied.
   - gamemd: selected country is stored on the local `HouseClass` and side is available through `HouseClass+0x34 -> CountryTypeClass+0xBC`; standard Skirmish `Create_Houses` is active and sets `g_PlayerPtr` for the local human.
   - Player-visible difference: a Soviet or Yuri player named `Commander` sees Allied sidebar chrome/highlight coloring after MCV deploy.

2. **Render consumers inherit the wrong theme.**
   - Rust: `src/app_sidebar_build.rs:37`, `src/app_render/build_instances.rs:732-733`, and `src/app_render/draw_passes.rs:478-502` all consume `current_sidebar_theme/current_sidebar_chrome`.
   - gamemd: the sidebar draws `SIDE1/2/3.SHP` and related pieces from side-specific sidebar assets.
   - Player-visible difference: the whole sidebar family, not just a small label, is wrong.

## Not Implemented

None within this scoped path. Sidebar chrome selection exists, but it reads from the wrong owner metadata source for named skirmish players.

## Timing

The Rust theme is recomputed during render, not during the deploy sim tick. Once `preferred_local_owner_name` returns `Commander`, the wrong Allied fallback applies immediately on the next render. Final gamemd repaint timing after MCV deploy was not recomputed in this trace.

## Adjacent Findings

- The likely fix should resolve theme from `Simulation.houses[owner]` first, using `HouseState.side_index` or `HouseState.country`, and only fall back to `AppState.house_roster` for map/campaign houses.
- Final screenshot/pixel capture is still needed after the fix to prove exact chrome asset, tint, and frame parity.

## Verdict Tally

PASS: 3 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Sources Checked

- `src/app_sidebar_render.rs`
- `src/app_commands.rs`
- `src/app_skirmish.rs`
- `src/skirmish_launch.rs`
- `src/sim/house_state.rs`
- `src/sim/world/world_spawn.rs`
- `src/render/sidebar_chrome.rs`
- `src/app_sidebar_build.rs`
- `src/app_render/build_instances.rs`
- `src/app_render/draw_passes.rs`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`
- `docs/research/COUNTRY_SIDE_TYPE_CLASSES.md`
- `docs/research/COUNTRY_ICON_SHP_SELECTOR_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`

