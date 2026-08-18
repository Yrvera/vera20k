# Loading First-Renderer Composition System Model Synthesis

**Date:** 2026-07-27  
**Scope:** standard offline selected-map Skirmish, from the final launch/start assignment through the first confirmed displayed loading frame.  
**Included:** LS background, selected preview, black start indicators, assigned `mmpb` markers, four post-marker text layers, and the raw-3 display handoff.  
**Non-scope:** campaign loading, setup-dialog `STARTBUT.SHP`, random-map preview internals, exact DirectDraw pixels, and exact HWND-versus-direct repaint liveness.  
**Output type:** model-synthesis.  
**Overall status:** **IMPLEMENTATION_SAFE** for the selected-map composition mechanism and logical layout; final native/Rust pixel parity remains **UNCHECKED**.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safety |
|---|---|---|---|---|---|
| `0x00552D60` composes manager `+0x60` offscreen and does not present it | `LOADING_FIRST_RENDERER_CORRECTED_COMPOSITION_DATA_READINESS_GHIDRA_REPORT.md` §§Entry-to-Return, First Confirmed Display; parent cold check `0x00552D60`, `0x00554400` | confirmed | high | yes, mode 5 | IMPLEMENTATION_SAFE |
| Native parses waypoints, creates houses, and assigns starts before the compositor | corrected composition report §Data Readiness; marker report §§8,10; parent cold check `0x00687550..0x00687588` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Top-level order is LS background → preview/markers → four text layers → return | corrected composition report §Entry-to-Return; marker report §4.1; text report §§1,3 | confirmed | high | yes | IMPLEMENTATION_SAFE |
| First confirmed selected-map display is the completed composition plus raw progress `3` | corrected composition report §First Confirmed Display; parent cold check `0x00687588..0x00687594`, `0x00643CF0..0x00643D55`, `0x00554400` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Marker helper receives preview wrapper in `ECX` and loading destination on the stack | marker report §3; parent cold check `0x0055367B..0x00553692` | confirmed | high | conditional on preview source | IMPLEMENTATION_SAFE |
| Marker region is `(x,y,w,h)` and uses equality-selected 800/1024 branches | marker report §§1,7; parent cold check `0x00640CE2..0x00640D48`, width words `0x007F5BE0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Marker iteration counts valid `0..7`, then visits numeric prefix `0..N-1` | marker report §5 | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Marker projection uses signed cell centers, isometric projection, aspect fit, two-stage `1,000,000` normalization, truncation toward zero, `(-3,-2)`, and surface clipping | marker report §§6,7; parent decompile/assembly cold check `0x00640A40` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Assignment is start-index → house-index; explicit selections precede human-first/AI-second automatic assignment | marker report §8; `0x005D6BE0`, `0x005EE9D0`, `0x005EE6F0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Each `mmpb` marker uses the assigned house's scheme convert, not the local progress color | marker report §9 | confirmed | high | conditional on assignment/convert | IMPLEMENTATION_SAFE |
| Four text layers are country, uppercased special unit, `LoadBrief:*`, and `GUI:LoadingEx` | text report §§1,3-5; parent callsite cold checks `0x005539DF`, `0x00553D01`, `0x00554022`, `0x005540A8` | confirmed | high | yes, mode 5 | IMPLEMENTATION_SAFE |
| Battle-family mode predicate is false; Cooperative is true and moves only the 800-base briefing Y | text report §§2,3,5; parent cold checks `0x005C0E40`, `0x005C4EF0` | confirmed | high | conditional by selected mode | IMPLEMENTATION_SAFE |
| Text uses `g_GAME_FNT`; selected layers have black alpha `0x9F` backing; the special-unit line is black without that backing | text report §§3,6; helpers `0x005541C0`, `0x00554280` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Exact glyph/blend/final-frame pixel equivalence | no retail runtime oracle | unknown | low | yes | UNSAFE_FOR_PARITY_CLAIM |
| Standard Skirmish always uses one specific HWND/direct repaint branch | no complete live HWND trace | unknown | low | conditional | UNSAFE_FOR_IMPLEMENTATION AS AN EXACT BRANCH CLAIM |

## Current Model

1. Launch/session resolution has already selected a mode, country, colors, and requested starts.
2. Native scenario initialization parses the selected map's waypoints and preview, constructs houses/color schemes, computes projected playfield bounds, applies explicit starts, and completes alternate or automatic assignment.
3. `0x00552D60` allocates/clears a hidden loading surface and draws the country LS frame.
4. `0x00640A40` decorates the selected preview with black `4x4` start indicators, aspect-fits it into the exact width-selected temporary region, overlays assigned-house-colored retail `mmpb.shp` frame 0 markers, and copies the completed region into the hidden loading surface.
5. The compositor draws localized country name, uppercase special-unit name, localized `LoadBrief:*`, and localized `GUI:LoadingEx`, with the verified logical rectangles, alignment, color-scheme identity, and alpha-backing rules.
6. The compositor returns without presenting. The immediately following advancing raw-3 callback copies the completed hidden surface, repaints progress, and synchronously blits it through the display chain. This is the first confirmed selected-map displayed frame.

## Implementation-Safe Facts

- The Rust-native owner should be a loading-presentation snapshot finalized before the first frame, not simulation state and not torn-down shell widgets.
- Selected-map preview/start/assignment data must exist before the first submitted frame. Current Rust instead submits at `3` before `InitialMapSelection`.
- Text does not require map parsing: local country, selected mode, CSF, font, and loading-side scheme are sufficient.
- Marker data does require the selected preview, original waypoint coordinates/projected bounds, and a start-index ownership/color mapping.
- Preserve player-slot order and waypoint/start-index order as separate domains.
- Preserve three draw phases so the separate font atlas can sit between preview/markers and progress: loading atlas background/preview/markers → font text → loading atlas progress/backing/icon.
- `mmpb.shp` remains retail runtime data. Do not hardcode its pixel matrix.
- Selected-map raw `3` and random-map raw `3 / 2 = 1` remain separate presentation contracts.

## Delivery Classification

- **MILESTONE-BLOCKING:** selected-map preview and assigned markers are absent on every ordinary selected-map Skirmish load.
- **MILESTONE-BLOCKING:** all four native mode-5 text layers are absent; the loading surface is visibly incomplete each match.
- **COMPOUNDING:** current `MmpbRegionRect` field semantics and `>=` width selector contradict the verified native contract and would visibly misplace the region once consumed.
- **EXACTIFICATION-RESIDUAL:** native sparse-hole prefix behavior on malformed/custom maps, provided the trigger and deviation remain recorded.
- **EXACTIFICATION-RESIDUAL:** equality fallback behavior at non-retail widths if a deliberate modern-width policy is chosen and truth remains labelled DRIFT.
- **EXACTIFICATION-RESIDUAL:** exact HWND/direct repaint selection, dwell, glyph raster, pixel-format quantization, alpha pixels, and complete-frame pixel parity.

## Doc-Patch-Ready Facts

- The old direct palette chain is wrong. Correct sequence: setup/build `0x00552CC0 -> 0x0072B530`, then later composition/access `0x00552D60 -> 0x0072B500`.
- `0x00552D60` does not display a separate pre-3 frame.
- The marker helper's wrapper owns the preview source; the stack argument is the loading destination.
- Marker tuples are `(x,y,w,h)`, not `(origin_x,size_x,size_y,origin_y)`.
- `LSLoadMessage` and mission briefing fields remain campaign-only, but mode 5 has a separate four-layer loading-text pipeline.

## Stale Or Superseded Claims

- `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`: stale pre-3 visibility, false palette call chain, incomplete data-readiness/text model.
- `skirmish-ui/SKIRMISH_MMPB_ASSIGNED_PLAYER_MARKER_CONTEXT_GHIDRA_REPORT.md`: stale source/destination ownership, `1000` interpretation, prefix iteration, projected-extent meaning, and assignment coverage.
- `LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md`: stale broad “no Skirmish loading text” conclusion.
- `src/render/loading_screen_chrome.rs`: current marker tuple names/comments and `>=` selector are contradicted before production consumption.

## Cross-Doc Conflicts

No unresolved load-bearing conflict remains among the three 2026-07-27 re-swarm reports. The parent cold checks corroborated their shared composition order and corrected the older documents above. The following are residual unknowns, not conflicts:

- exact selected-mode alternate assignment (`vtable +0x84`) internals outside the explicit/ordinary assignment branch;
- exact live HWND/direct repaint branch for every configuration;
- complete native/Rust pixel equivalence.

## Needs Re-Investigation

None for the bounded selected-map implementation. Re-investigate only if implementation expands to:

- in-game/generated random-map preview/marker composition;
- exact non-retail-width behavior as a parity target;
- executable full-frame pixel certification;
- every alternate selected-mode assignment implementation.

## Do-Not-Implement Notes

- Do not delay native selected-map marker data until after the first displayed Rust frame.
- Do not render markers from `[Map] LocalSize`, zip players to waypoints, or tint every marker with the local progress color.
- Do not use `STARTBUT.SHP`, circles, or hardcoded pixels in place of retail `mmpb.shp`.
- Do not float-collapse the signed staged projection or center the 12×12 marker by `(-6,-6)`.
- Do not source these four text layers from `LSLoadMessage`, `LSLoadBriefing`, map `[Briefing]`, or hardcoded English.
- Do not call the synchronous loading blit native `Present`/`Flip`.
- Do not claim pixel parity from formula/unit tests alone.

## Current Rust Surface

- `src/app.rs`: selected scenario records, CSF/font ownership, and launch-to-loading handoff.
- `src/app_loading.rs`: `LoadingRequest`, `NativeLoadingScreenState`, first-frame timing, instance construction, synchronous milestone repaint.
- `src/render/loading_screen_chrome.rs`: LS/PROGBARM atlas, marker constants, future retail `mmpb` entry.
- `src/skirmish_scenarios.rs`: selected preview, multiplayer waypoints, and preview-source bounds.
- `src/skirmish_launch.rs`: resolved local/opponent country, color, mode, and requested starts.
- `src/render/shell_text.rs` / `src/render/bit_font.rs`: existing GAME.FNT measurement, wrapping, alignment, scissor, and glyph instance paths.

## Source Ledger

- `LOADING_FIRST_RENDERER_CORRECTED_COMPOSITION_DATA_READINESS_GHIDRA_REPORT.md`
- `LOADING_MMPB_EXACT_MARKER_ASSIGNMENT_COMPOSITION_GHIDRA_REPORT.md`
- `LOADING_POST_MARKER_TEXT_MODE5_CONTENT_LAYOUT_GHIDRA_REPORT.md`
- `LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md`
- `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`
- `LOADING_PROGRESSCLASS_VALUE_MAX_MAPPING_GHIDRA_REPORT.md`
- `AUDIT_LOG.md` entries dated 2026-07-27 for the three superseded documents
- Parent read-only cold checks: `0x00687550..0x00687594`, `0x0055367B..0x00553692`, `0x00640CE2..0x00640DA0`, `0x005539DF`, `0x00553D01`, `0x00554022`, `0x005540A8`, `0x00554100/150/1C0/280`, `0x005C0E40`, `0x005C4EF0`, `0x00554400`
- Retail `langmd.mix` CSF and retail `mmpb.shp` metadata as cited by the focused reports
- Current Rust direct reads at the surfaces listed above
