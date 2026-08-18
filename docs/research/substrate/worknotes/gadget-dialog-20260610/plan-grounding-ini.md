# Plan grounding — INI / asset / audio dependencies (lane R4)

**Date:** 2026-06-10
**Plan scope:** A0/A1/A4/A5 + D-B3 + R1 (ui::gadget substrate).
**Sources:** `ini/` in MAIN repo `.` (canonical); all `src/` file:line anchors verified against the WORKTREE `<local>/Documents/ra2-uigadget-worktree` (branch `ui-gadget-substrate` @ 7b79a186). No Ghidra used; binary claims cite existing docs.

---

## 1. Click/UI sound keys — `[AudioVisual]`, ini/rulesmd.ini (section spans lines 595–781)

| Key | rulesmd.ini line | Retail value | Parsed in Rust? | Rust field / anchor |
|---|---|---|---|---|
| `GUIMainButtonSound` | 643 | `MenuClick` | YES | `GeneralRules.gui_main_button_sound` (worktree `src/rules/ruleset.rs:298-299`, parse `:972-976`) |
| `GUIBuildSound` | 644 | `MenuClick` | **NO** | — |
| `GUITabSound` | 645 | `MenuTab` | **NO** | — |
| `GUIOpenSound` | 646 | `MenuACBOpen` | **NO** | — |
| `GUICloseSound` | 647 | `MenuACBClose` | **NO** | — |
| `GUIMoveOutSound` | 648 | `MenuSlideOut` | **NO** | — |
| `GUIMoveInSound` | 649 | `MenuSlideIn` | **NO** | — |
| `GUIComboOpenSound` | 650 | `MenuACBOpen` | YES | `gui_combo_open_sound` (`ruleset.rs:304-305`, `:987-991`) |
| `GUIComboCloseSound` | 651 | `MenuACBClose` | YES | `gui_combo_close_sound` (`ruleset.rs:306-307`, `:992-996`) |
| `GUICheckboxSound` | 652 | `MenuClick` | YES | `gui_checkbox_sound` (`ruleset.rs:302-303`, `:982-986`) |
| `IncomingMessage` | 683 | `MessageText` | **NO** | — (A5 insert sound) |
| `MessageCharTyped` | 684 | `TextBleep` | **NO** | — (A5 typing effect) |
| `ScoldSound` | 698 | `MenuScold` | **NO** | — (zero hits for "scold" in worktree src) |
| `GenericClick` | 703 | `MenuClick` | YES | `generic_click_sound` (`ruleset.rs:300-301`, `:977-981`) |
| `ShellButtonSlideSound` | 712 | *(empty)* | **NO** (hardcoded knowledge) | `src/app_shell_transition.rs:9` comments "empty ⇒ no sound" without reading the key |

All parse sites use the same trim/filter-empty `Option<String>` pattern; struct is `GeneralRules` (`ruleset.rs:193`).

**Sound event entries exist in ini/soundmd.ini** (so playback resolves once keys are parsed): `[MenuClick]`:2926, `[MenuTab]`:2930, `[MenuACBOpen]`:2938, `[MenuACBClose]`:2942, `[MenuSlideIn]`:2946, `[MenuSlideOut]`:2954, `[MessageText]`:2959 (`Sounds=umessage`, `Volume=60`), `[MenuScold]`:2987 (`Sounds=umenscol`, `Volume=40`), `[TextBleep]`:3028 (`Sounds=utext`, `Control=interrupt`, `Limit=1`, `Priority=high`, `Volume=60`).

**Playback path (already working for shells):**
- `App::play_shell_ui_sound_by_id` (worktree `src/app.rs:1848-1861`) → `sfx.play_sound(sound_id, &state.sound_registry, assets, &state.audio_indices)`.
- `SoundRegistry::from_ini` / `get` — `src/rules/sound_ini.rs:56` / `:165`.
- Shell sound-enum→GeneralRules-field mapping: `skirmish_shell_ui_sound_id`, `app.rs:1816-1834` (only 4 variants today: GuiCheckboxSound, GenericClick, GuiComboOpenSound, GuiComboCloseSound).

**In-game sidebar today plays NO click sound at all.** Sidebar actions fire on mouse-DOWN: `handle_mouse_input` `src/app_input.rs:39-43` → `handle_sidebar_mouse_input` `:227-238` → `apply_sidebar_action` `:240+` — no sound emission anywhere in that path. This matches the A1 flip target (fire-on-RELEASE will be the new authority; sounds are downstream Action-ID consumer behavior — study `gadget-family.md:470-474`: "No sound is played inside the gadget layer for sidebar buttons; click sounds come from downstream Action-ID consumers").

**ASSUMPTION flag for the plan (not binary-verified this session):** which exact key fires on tab click (`GUITabSound`) vs cameo click (`GUIBuildSound`) vs invalid click (`ScoldSound`) is inferred from key names + comments, not traced in gamemd. One Ghidra spot-check on the sidebar Action-ID consumer is needed before wiring sounds in A1, or land the parse fields and leave wiring behind a verified-mapping task.

---

## 2. Chat/system message constants (A5)

- `MessageDelay=.6` — ini/rulesmd.ini:758, inside `[AudioVisual]` (next section header `[CrateRules]` at :782). Comment: "time duration of multiplayer messages displayed over map". Units: minutes (gamemd converts minutes→frames; conversion factor to verify at implementation — standard is 900 frames/min at normal speed). **NOT parsed** anywhere in worktree src.
- **No message-color INI key exists.** `[AudioVisual]` color keys are only LineTrailColorOverride/ChronoBeamColor/MagnaBeamColor/LaserTargetColor/IronCurtainColor/BerserkColor/ForceShieldColor/LocalRadarColor (lines 600-628, 776). Chat line color comes from the sending house's color; system messages use a fixed color — both binary-side constants, no INI dependency.
- 14-slot cap, heap label per message, wrap, sound on insert: binary constants per study `gadget-family.md` §3.10 (`MessageListClass::Add_Message` 0x005D3BA0, 0xE slots, `VocClass__PlayAtPos` on insert) — the only INI dependencies are `IncomingMessage`, `MessageCharTyped`, `MessageDelay` above.
- `[AudioVisual] SpeakDelay=2` (:770) is EVA advice cadence — NOT part of A5.

---

## 3. Tooltip text data (A4)

**CSF (localized strings):**
- Parser: `src/assets/csf_file.rs` — `CsfFile::from_bytes` (:52), `get(&str) -> Option<&str>` (:118), `entries()` (:135).
- Load: `load_csf` `src/app_init.rs:147-160` (tries `ra2md.csf`, `ra2.csf`, `stringtablemd.csf`, `stringtable.csf`), stored at `MapLoadResult.csf` (`app_init.rs:118`) → `AppState.csf`.
- Lookup helpers the tooltip service can mirror: `App::csf_label` (`src/app.rs:860-867`, key→text with English fallback), `localized_status_help_text` (`app.rs:1213-1220`).

**Object display name + cost (cameo tooltips):**
- `UIName=` is **NOT parsed** by any rules struct. `object_type.rs` parses only `Name=` (plain English, `:866`) and `Cost=` (`:867`). `superweapon_type.rs` has UIName only in doc-comment/test text — no field. Retail rulesmd.ini has `UIName=Name:MTNK` style keys on every techno (e.g. :6604).
- Current display-name chain: `BuildOption.display_name = obj.name | obj.id` (`src/sim/production/production_tech.rs:85`) → `resolve_csf_name` (`src/app_sidebar_render.rs:465-472`) tries the *whole string* as a CSF key — misses for English `Name=` values, so sidebar shows English INI names, not CSF-localized text. **For gamemd tooltip parity (localized UIName + cost) a `ui_name: Option<String>` parse on ObjectType (and SW types) is a plan task.**
- Cameo slot data structure the tooltip service consumes: `SidebarItem` (`src/sidebar/mod.rs:156-176`) — `rect`, `type_id: String`, `display_name`, `cost: Option<i32>`, `is_superweapon`, `super_weapon_section`. Built per frame by `current_sidebar_view` (`src/app_sidebar_render.rs:29`) → `build_sidebar_view_with_spec` (`src/sidebar/sidebar_view.rs`, items grid `:177-210`). Tab/repair/sell rects: `SidebarTabButton` (`mod.rs:186-195`), `SidebarToggleButton` (`mod.rs:201-206`).

**Static button tooltip text (gamemd, doc-sourced):** `TOOLTIP_TEXT_SOURCE_AND_DELAY_TIMERS_GHIDRA_REPORT.md` — delay 1000 ms / duration 10000 ms / TTIP timer; registered descriptor hit-tests **inclusive** right+bottom edges (claim 7 — matches A4's deliberate inclusive-both-edges rule); repair = `TXT_REPAIR_MODE`, sell = `TXT_SELL_MODE` (direct CSF label keys); scroll up ID 200→string 0x13CD, scroll down 0xC9→0x13D3, tabs 0xCB..0xCE→0x13DB/0x13DD/0x13DF/0x13E1, power ID 999. **UNKNOWN:** the CSF *label names* behind numeric IDs 0x13CD/0x13D3/0x13DB.. (our CsfFile looks up by label, not numeric ID) — needs one mapping pass (Ghidra string-table or CSF dump) before tab/scroll tooltips can show retail text.
- Cameo hover sets delay to 0 (immediate tooltip) by save/zero/restore of the manager delay — same doc, claim 2.

**Shell tooltips today (the "no delay" behavior A4 replaces):** descriptor field `tooltip_key: Option<&'static str>` (`src/ui/shell/descriptor.rs:92-93`); main-menu bottom status line built unconditionally from hover (`src/app_main_menu_shell_render.rs:142-158`); skirmish shell status help `app.rs:1222-1240`; hover tracking `src/ui/shell/controller.rs:41`. No timer anywhere.

---

## 4. Fonts / art for A5 labels + A4 tooltip box

- `GAME.FNT` parsed by `src/assets/fnt_file.rs` (`fonT` magic only — `FonT` is dormant TS-legacy per BITFONT report), loaded at `src/app_init.rs:1027-1029` (`asset_manager.get_ref("GAME.FNT")`), stored `MapLoadResult.fnt_file` (`:120`).
- `BitFont` (atlas + measure + wrap state machine): `src/render/bit_font.rs`; constructed `src/app.rs:2333` (`fallback_5x7`) then `:2399` (`from_fnt`), rebuilt on transition `src/app_transitions.rs:104`. Owned by `AppState.bit_font`. Constants match the binary: CHAR_SPACING=1, TAB_WIDTH=64, CELL_HEIGHT=17, BITMAP_ROWS=16, missing-glyph 0xB0 + XOR 0x5555.
- Upper wrappers: `src/render/shell_text.rs` (Path A — align bit-flags 0x01/0x02/0x04, clip) and `src/render/sidebar_text.rs` (Path B — single-line + fade).
- **gamemd in-game tooltip box uses the same global GAME.FNT BitFont** for sizing AND draw (`TOOLTIP_GLYPH_RASTER_LINE_WRAPPING_GHIDRA_REPORT.md:12` — `BitFont__MeasureText` sizing, +4/+3 padding, remeasure at region_width−4, per-pixel clip, no shadow pass, line height 17). Existing `BitFont::wrap_layout` is the right substrate; no tooltip SHP asset exists or is needed (box is filled rect + border).
- **A5 TextLabel font: UNKNOWN.** The study worknotes do not name the font used by `TextLabelClass::Draw_Me`; no `EditFont`/secondary FNT reference found in the study or BITFONT report. Likely the same `g_GAME_FNT` global, but verify with one decompile of the TextLabel draw before A5 draw-cadence work.
- gamemd font authority: one global `g_GAME_FNT @ 0x0089C4D0` for shell, sidebar, and tooltip text (`BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` §summary; FonT path dormant).

---

## 5. SHP frame indices for sidebar button states

**Current Rust contract (matches study G-clauses' 5-frame convention):**
- `frame_select(disabled, mode_active, state)` → 0=idle, 1=mode/tab-active, 2=disabled, 3=pressed-idle, 4=pressed-active: `src/sidebar/gadget_flash.rs:112-123` (mirrors SBGadgetClass::Draw 0x0069DEB0 per file comment).
- Per-gadget accessors: `tab_frame` `:166-170`, `repair_frame` `:174-176`, `sell_frame` `:179-181`. **Gap for A1:** repair/sell pass `state=0` unconditionally — there is NO transient mouse-down pressed visual today (frames 3/4 unreachable for repair/sell; tabs reach 3/4 only via flash AI). The G22 silent-press contract requires the substrate to drive the `state` bit during press-hold.
- Atlas storage: `SidebarChromeAtlas.tab_frames[[Option;5];4]` (`src/render/sidebar_chrome.rs:84`), `repair_frames` (`:88`), `sell_frames` (`:90`); loaded from `tab00..tab03.shp`, `repair.shp`, `sell.shp` frames 0..5 (`sidebar_chrome.rs:293-329`), warn + fallback-to-frame-0 on missing.
- View populates `frame_index`: `src/sidebar/sidebar_view.rs:139` (tabs), `:169` (repair), `:174` (sell). Draw consumes: `src/app_sidebar_build.rs:136-138` (tabs), `:171-172` (sell), `:183-184` (repair).

**Strip-scroll buttons (A1's 2 new gadgets) do not exist in Rust at all.** Scrolling is mouse-wheel-only: `try_sidebar_scroll` `src/app_input.rs:210-225`, state `AppState.sidebar_scroll_rows` (`src/app.rs:314`), reset on tab switch (`app_input.rs:245`). Retail assets/IDs (doc-sourced):
- SHPs `R-UP.SHP` / `R-DN.SHP`, loaded by `SidebarClass__LoadSHPs` alongside SELL/REPAIR/TAB00..03 (`RETAIL_SOVIET_SIDEBAR_SHP_DIMENSIONS_OFFSETS_GHIDRA_REPORT.md:29`). NOT loaded by `sidebar_chrome.rs` today → A1 asset task.
- IDs/positions: scroll up = 0xC8 at `ScrollX + ScrollWidth`; scroll down = 0xC9 at `ScrollX` (`SELL_REPAIR_TAB_SCROLL_EXACT_GADGET_RECTS_GHIDRA_REPORT.md:77, :232` — note this corrected an older doc that had the two reversed).
- Flags mask 0x55 (no held bits 0x2/0x20) ⇒ NO hold-repeat, one page per click, fire-on-release (study G23/A1 row, `GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md:366, :470`).
- **UNKNOWN:** R-UP/R-DN frame count (assume the same 5-frame convention; read SHP header from retail `sidec01.mix`/`sidec02.mix` during A1 before wiring `frame_select`).

---

## Unparsed-key task list (feeds the plan)

Required by in-scope slices:
1. `GUITabSound` ([AudioVisual]:645 → `MenuTab`) — A1 tab click. New `GeneralRules` field + shell-sound-enum variant or direct lookup.
2. `ScoldSound` (:698 → `MenuScold`) — A1 invalid-click feedback (consumer mapping needs one binary spot-check).
3. `GUIBuildSound` (:644 → `MenuClick`) — cameo click; parse can land with A1 even though cameo gadget conversion is A2.
4. `IncomingMessage` (:683 → `MessageText`) — A5 message-insert sound.
5. `MessageCharTyped` (:684 → `TextBleep`) — A5 typing effect (only if compose/typing is in slice scope).
6. `MessageDelay` (:758 → `.6` minutes) — A5 message lifetime; verify minutes→frames factor.
7. `UIName=` on ObjectType (and SW sections) — A4 localized tooltip names; today display names are English `Name=` text, CSF resolution effectively bypassed for technos.

Adjacent, NOT required by A0/A1/A4/A5 (list for completeness, don't wire): `GUIOpenSound`/`GUICloseSound` (:646-647), `GUIMoveInSound`/`GUIMoveOutSound` (:648-649), `ShellButtonSlideSound` (:712, empty in retail; currently a hardcoded comment instead of a parse).

Asset tasks (not INI): load `R-UP.SHP`/`R-DN.SHP` into `SidebarChromeAtlas` (+ verify frame count); CSF label names for numeric tooltip string IDs 0x13CD/0x13D3/0x13DB/0x13DD/0x13DF/0x13E1; verify TextLabel draw font (assumed GAME.FNT).
