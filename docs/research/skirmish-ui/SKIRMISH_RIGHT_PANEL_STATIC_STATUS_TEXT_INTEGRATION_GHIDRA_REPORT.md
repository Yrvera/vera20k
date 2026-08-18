# Skirmish Right-Panel Static / Status Text Integration - Ghidra Research Report

**Address(es):** `0x006AE3F0`, `0x00622B50`, `0x006153E0`, `0x0060A5B0`, `0x00602490`, `0x00610CA0` assembly slice, `0x005E2EF0`, `0x005E2F60`, `0x0060B550`, `0x0060C0C0`, `0x006071E0`, `0x006ACEE0`, `0x00621040`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** integration of standard offline Skirmish dialog `0x102` right-panel title/game-type/map static text controls and bottom-left status/help child `0x695`: child/static rect surfaces, string/update sources, colors/layout flags, y/vertical-adjust behavior, invalidation/update timing, and current Rust first-class layout/render/state coverage.
**Non-Scope:** full child matrix, Choose Map `0x6B` internals, player-name edit `0x6A0`, glyph raster internals, full tooltip key inventory for every control, runtime screenshot capture, and Rust implementation.
**Confidence:** High for active paths, rect/update integration, string-source ownership, and current Rust deltas; Medium for final perceived pixel RGB without retail surface capture.
**Active in YR:** Yes for standard offline Skirmish dialog `0x102`; status/help text content is Conditional on hover/status source availability.

## 0. Working Notes Gate

- Target question: Does the active `gamemd.exe` Skirmish `0x102` path integrate right-panel static labels and the `0x695` status/help strip as separate first-class child/static text controls, and does current Rust have matching layout/render/state surfaces?
- Non-goals: Do not re-cover full right-panel chrome, full child matrix, Choose Map `0x6B`, edit control `0x6A0`, or general glyph raster behavior unless a direct contradiction appears.
- Evidence needed to mark COMPLETE: Ghidra decompile plus assembly/call-site evidence for static classification/paint, dynamic `0x4B2` text-copy/update, status child hover write, right-panel and status placement, shell transition invalidation timing, current Rust layout/render/state scan, and stale-doc wording.
- Stop conditions: All scoped integration questions resolved or explicitly deferred; no Ghidra mutations; exactly this report written; implementation handoff contains Rust-facing deltas with test-name proposals.

## 1. Overview

The right-panel title `0x694`, game-type `0x6EC`, and map/scenario label `0x5A8` are separate Static-class child controls reclassified to kind-1 animated shell text. They paint above the right-panel chrome through `OwnerDraw_Static_006153E0` and `FUN_00621040`; their caller `RECT` is both layout and clip, and they are top-anchored because static style flags do not set vertical-center bit `0x04`.

The bottom-left status/help strip `0x695` is a separate visible Static child, not part of the right panel. It is normally blank and is updated by the common parent shell handler on `WM_NCHITTEST`/hover by sending dynamic wide text message `0x4B2` to child `0x695`.

## 2. Verified Findings

| Finding | Active in YR | Evidence |
|---|---|---|
| Offline Skirmish enters dialog `0x102` proc `0x006AE3F0`, and that proc delegates common shell handling before Skirmish-specific paint/commands. | Yes | `FUN_006AE3F0` decompile calls `FUN_00622B50`; prior launcher evidence `0x006AE31C..0x006AE328` creates id `0x102`. |
| The right-panel static labels are accepted by `FUN_00602490` for parent dialog `0x102`: title `0x694`, and `0x6EC/0x5A8`. | Yes | `FUN_00602490` decompile has explicit `iVar5 == 0x102` branches for `0x694` and for `0x6EC || 0x5A8`. |
| Accepted static labels are reclassified to kind `1`, reveal byte clear, reveal count `1`, interval/step/range loaded from helper trio. | Yes | `FUN_0060A5B0` decompile sets `piVar8[0x1c]=1`, `[0x2a]=0`, `[0x20]=1`, then calls `0x00600CA0`, `0x006015E0`, `0x00601D20`. |
| Static paint only draws kind-1 text when the running byte is set; text uses the child rect from `FUN_00775690`, style-derived flags `0x10/0x11/0x12`, normal color `DAT_00AC18A4`, disabled color `DAT_00AC1CB4`, and `FUN_00621040`. | Yes | `OwnerDraw_Static_006153E0` decompile; call-site assembly `0x00615AE8` calls `0x00621040`. |
| Static labels are not vertically centered. `FUN_00621040` adds a y adjustment only when flags include `0x04`; static flags `0x10/0x11/0x12` do not. | Yes | `FUN_00621040` decompile verifies `flags & 4` y-centering; `OwnerDraw_Static_006153E0` style flag selection plus call `0x00615AE8`. |
| Game-type and map/scenario text updates use dynamic `0x4B2` messages to the actual child HWNDs. | Yes | `FUN_005E2EF0` gets child `0x6EC` and sends `0x4B2` with `FUN_007B7140()`; `FUN_005E2F60` gets child `0x5A8` and sends `0x4B2` with `0x00A8B322`. |
| Successful Choose Map refresh calls both right-panel text update helpers before preview/status invalidation completes. | Yes | `FUN_006ACEE0` decompile calls `FUN_005E2EF0()`, `FUN_005E2F60()`, then `FUN_006ACD60()`, rebuilds/refreshes preview state, and calls `InvalidateRect(parent,NULL,0)`. |
| The common subclass thunk at `0x00610CA0` owns dynamic text copy for `0x4B2`; the static owner proc only refreshes cached backing and invalidates after the copy. | Yes | Read-only assembly `0x00611BC1..0x00611C67` copies incoming wide text into record `+0x28`; `0x00612318..0x0061234B` dispatches original message to stored owner proc; `OwnerDraw_Static_006153E0` `0x4B2/0x4B4` branch refreshes/invalidate. |
| If kind-1 text changes while animation byte is set, the thunk kills timer `0`, clears running byte, and sends `0x4EE` to restart reveal. | Conditional: only when a kind-1 static is already running. | Assembly `0x00611C72..0x00611CAF`: kind compare, running-byte test, `KillTimer`, clear `[+0xA8]`, `SendMessageA(hwnd,0x4EE,0,0)`. |
| Shell transition reveal starts through parent `0x4EC` broadcast to child `0x4EE`, not through ordinary common first paint. | Conditional: transition path with nonzero animation mode. | `FUN_00622B50` handles `0x4EC` by `EnumChildWindows(...,FUN_0060AA60,0)`; `OwnerDraw_Static_006153E0` `0x4EE` starts timer/invalidate; `FUN_006071E0` sends `0x4ED` on non-animated path and `0x4EC` on animated path. |
| Status/help child `0x695` is obtained and updated by the common parent hover path, not by Skirmish tick logic. | Yes | `FUN_00622B50` `WM_NCHITTEST` branch assembly `0x00622CCB` pushes `0x695` for `GetDlgItem`; `0x00622E6D..0x00622E83` converts selected text and sends `0x4B2` to the status child. |
| Status/help fallback order is hovered child `0x4E8`, Skirmish parent `0x4E9`, tooltip dispatcher `FUN_006040B0`, then empty wide string `0x00887734`. | Yes / Conditional on hovered control. | `FUN_00622B50` decompile; `FUN_006AE3F0` `0x4E9` branch supplies AI row item strings; prior status report verifies `FUN_006040B0` mapping and no self-tooltip for `0x695`. |
| Status/help child placement is bottom-left, not right-panel anchored: `x=center_x+10`, `y=screen_h-child_h-center_y-1`, preserving resource size. | Yes | `ResizeShellChildControl_0060C0C0` assembly `0x0060C2B6..0x0060C2C9` routes child `0x695` to `FUN_0060B550`; `FUN_0060B550` decompile computes formula and calls `MoveWindow`. |
| INI data is not part of this text integration. | Yes | No scoped function reads INI; sources are RT_DIALOG child IDs, CSF/string-table/runtime buffers, Win32 messages, and owner-draw records. |
| This is live YR shell UI, not TS legacy. | Yes | Standard offline Skirmish path `0x006AE3F0`/dialog `0x102`; no TS-only default-off gate was found in the scoped static/status path. |

## 3. Current Rust Implementation Status

| Surface | Status |
|---|---|
| `src/ui/skirmish_shell/layout.rs` | Current Rust has first-class `SkirmishRightPanelTextRects` with title/game_type/map_label, computed at `layout.rs:417..420`, and tests at `layout.rs:853..876` for 640/800 rects. It does not expose `0x695` status/help strip. |
| `src/app_skirmish_shell_render.rs` | Current Rust renders right-panel title/game-type/map text in `build_shell_text_draws` at `app_skirmish_shell_render.rs:1726..1760`, with `ShellAlign::H_CENTER` and no `V_CENTER`, so the top-anchor/static y behavior is now represented. It renders all shell text after chrome in the same pass (`:2277..2331`), so text sits above right-panel SHP chrome. |
| `src/ui/skirmish_shell/state.rs` | Current Rust has no status/help text state, no hover-to-STT resolver, and no `0x4E8/0x4E9/FUN_006040B0` equivalent. The only mouse-move logic found is control interaction/drag handling. |
| `src/ui/skirmish_shell/mod.rs` | Exports right-panel text rects but no status/help rect or status text state API. |
| `src/render/shell_text.rs` | Existing clipped text primitive matches the `FUN_00621040` caller-rect-as-layout-and-clip contract well enough for these surfaces, but it does not model kind-1 reveal counts/timers itself. |

## 4. Integration Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Right-panel `0x694/0x6EC/0x5A8` are first-class static child text controls above chrome, using child rect, h-center style, top anchoring, yellow normal text, and dynamic `0x4B2` updates. | `FUN_00602490`, `FUN_0060A5B0`, `OwnerDraw_Static_006153E0`; call `0x00615AE8`; `FUN_005E2EF0`, `FUN_005E2F60`; Rust scan `layout.rs:417..420`, renderer `:1726..1760`, pass `:2277..2331`. | mostly present; reveal/restart timing still missing | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, future shell text animation state | Preserve named rects and no `V_CENTER`; add kind-1 reveal only for transition/text-update semantics, not by hiding text on normal first paint. | 800x600 shell draws title `(635,3,162,16)`, game type `(649,167,135,16)`, map `(649,189,135,33)` above right-panel chrome; selecting a new map updates game/map labels and can restart reveal if running. Proposed test: `skirmish_right_panel_static_text_surfaces_update_above_chrome`. | Do not bake these labels into chrome art or render through generic v-centered labels. |
| `0x695` is a separate bottom-left status/help strip, normally blank, populated by hover/status chain and written via `0x4B2`. | `FUN_00622B50` decompile; assembly `0x00622CCB`, `0x00622E6D..0x00622E83`; `FUN_0060B550`; status report rects. | missing | `src/ui/skirmish_shell/layout.rs`, `state.rs`, `mod.rs`, `app_skirmish_shell_render.rs` | Add explicit status/help rect and optional status text state; default blank; render only when non-empty through shell text. | At 640/800/1024, status rects are `(10,459,615,20)`, `(10,579,615,20)`, `(122,663,615,20)`; fresh no-hover shell renders no status text; hovering Start resolves an `STT:*` help string. Proposed test: `skirmish_status_child_0695_rects_and_blank_default_match_gamemd`. | Do not anchor this strip to the right panel or hardcode "Status"/map/game text into it. |
| Status text source order is child `0x4E8`, parent `0x4E9` item-specific text, `FUN_006040B0` `STT:*` fallback, empty fallback. | `FUN_00622B50`, `FUN_006AE3F0` `0x4E9`; prior `FUN_006040B0` status report. | missing | `state.rs` hit-test/hover model and localization bridge | Add hover resolver that uses control IDs and combo item data, with AI row item text overriding generic tooltips. | Hovering an AI row-state combo item yields None/Easy/Normal/Hard-specific status text; hovering unmapped/status strip itself clears to blank. Proposed test: `skirmish_status_child_0695_hover_uses_native_source_order`. | Do not use visible control labels as tooltip fallback. |
| Dynamic `0x4B2` text updates copy string ownership before static refresh/invalidation; if kind-1 reveal is running, update restarts reveal by `0x4EE`. | thunk assembly `0x00611BC1..0x00611CAF`; owner static `0x4B2/0x4B4` branch. | missing for reveal lifecycle; direct render state currently always recomputes text | future `SkirmishShellState` text/reveal fields or render-side animation state | Model text value separately from paint; on text change during active reveal, restart reveal count/timer. | Change selected map while the shell transition reveal is active: old text is replaced atomically, reveal restarts from count `1`; if no transition animation is active, Rust should still show updated text. Proposed test: `skirmish_static_text_4b2_update_restarts_reveal_only_when_running`. | Do not put text-copy semantics in the static paint function alone; do not blank first paint unless transition event is modeled. |

## 5. Negative Facts / Do Not Do

- Do not treat the right-panel labels as baked `SDTP/SDBTNBKGD/SDBTM` pixels. Active in YR: Yes. Evidence: `FUN_00602490` and `FUN_0060A5B0` classify child IDs `0x694/0x6EC/0x5A8`; `OwnerDraw_Static_006153E0` calls `FUN_00621040` at `0x00615AE8`.
- Do not vertically center title/game/map statics. Active in YR: Yes. Evidence: static style flags `0x10/0x11/0x12` and `FUN_00621040` only y-centers on `flags & 0x04`.
- Do not conflate status child `0x695` with the right-panel statics. Active in YR: Yes. Evidence: `0x695` routes through `FUN_0060B550` bottom-left placement, while right-panel labels use right-panel child rects.
- Do not show permanent status/help text by default. Active in YR: Yes. Evidence: `FUN_00622B50` falls back to empty wide string `0x00887734`; prior matrix says resource text is blank.
- Do not implement status text from visible GUI labels. Active in YR: Yes. Evidence: update path tries child `0x4E8`, parent `0x4E9`, then `FUN_006040B0` `STT:*` mapping before empty fallback.
- Do not rely on OwnerDraw static paint as the owner of dynamic `0x4B2` text copy. Active in YR: Yes. Evidence: thunk assembly `0x00611BC1..0x00611C67` copies text before owner-proc dispatch.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline `0x102` proc and common shell delegation | verified | `FUN_006AE3F0`, prior launcher `0x006AE31C..0x006AE328` | none |
| Right-panel static classification | verified | `FUN_00602490`, `FUN_0060A5B0` | none |
| Static paint rect/color/y behavior | verified | `OwnerDraw_Static_006153E0`, `FUN_00621040`, call `0x00615AE8` | screenshot RGB optional |
| Game/map dynamic string update senders | verified | `FUN_005E2EF0`, `FUN_005E2F60`, `FUN_006ACEE0` | exact CSF/map buffer contents outside scope |
| Static text-copy thunk and reveal restart | verified | assembly `0x00611BC1..0x00611CAF`, `0x00612318..0x0061234B` | full non-text thunk behavior outside scope |
| Shell transition reveal timing | verified for scoped relationship | `FUN_00622B50`, `OwnerDraw_Static_006153E0`, `FUN_006071E0`; prior reveal report | aggregate transition screenshot outside scope |
| Status child update and source order | verified | `FUN_00622B50`, `FUN_006AE3F0`, prior `FUN_006040B0` report | full tooltip key table outside scope |
| Status child rect/placement | verified | `FUN_0060C0C0`, `FUN_0060B550`, prior matrix rects | none |
| Current Rust right-panel text surfaces | verified | source scan `layout.rs`, `app_skirmish_shell_render.rs` | reveal lifecycle |
| Current Rust status/help surfaces | verified missing | source scan `layout.rs`, `state.rs`, `mod.rs`, render file | implementation later |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Are title/game/map text separate child/static controls or incidental chrome? -> Separate Static-class children reclassified to kind-1 text controls.` (evidence: `FUN_00602490`, `FUN_0060A5B0`, `0x00615AE8`)
- `[RESOLVED] OQ-02 - Do right-panel statics use caller rect as layout+clip? -> Yes via `FUN_00621040`, same child rect passed from static paint.` (evidence: `OwnerDraw_Static_006153E0`, `FUN_00621040`)
- `[RESOLVED] OQ-03 - Is there a static y adjustment? -> Only if bit `0x04`; right-panel static flags do not set it.` (evidence: `FUN_00621040`, `0x00615AE8`)
- `[RESOLVED] OQ-04 - What writes game-type and map/scenario strings? -> `0x005E2EF0` and `0x005E2F60` send `0x4B2` to `0x6EC` and `0x5A8`.` (evidence: decompiles)
- `[RESOLVED] OQ-05 - Where is dynamic static text copied? -> Common thunk `0x00610CA0` assembly slice copies incoming `0x4B2` text to record `+0x28`.` (evidence: `0x00611BC1..0x00611C67`)
- `[RESOLVED] OQ-06 - What invalidates after dynamic text update? -> Owner static `0x4B2/0x4B4` branch refreshes backing if present and invalidates; thunk may restart reveal first.` (evidence: `OwnerDraw_Static_006153E0`, `0x00611C72..0x00611CAF`)
- `[RESOLVED] OQ-07 - Is `0x695` status tied to right-panel chrome? -> No; bottom-left placement helper and parent hover update path.` (evidence: `FUN_0060B550`, `FUN_00622B50`)
- `[RESOLVED] OQ-08 - What triggers status updates? -> Parent `WM_NCHITTEST` hover path sends `0x4B2` to child `0x695`.` (evidence: `0x00622CCB`, `0x00622E6D..0x00622E83`)
- `[RESOLVED] OQ-09 - Does current Rust have right-panel static text first-class surfaces? -> Yes for layout/render; not reveal lifecycle.` (evidence: `layout.rs:114..118`, `:417..420`, render `:1726..1760`)
- `[RESOLVED] OQ-10 - Does current Rust have status/help strip surfaces? -> No named layout/render/state/API surface found.` (evidence: `layout.rs`, `state.rs`, `mod.rs`, render scan)
- `[DEFERRED] OQ-11 - Exact final screenshot RGB for normal/disabled text.` (category: `needs-runtime-debugger`; reason: binary source colors are verified, but display-format/capture pixels were not sampled; next-step-if-pursued: retail capture of normal/disabled right-panel and status text)
- `[DEFERRED] OQ-12 - Full `FUN_006040B0` tooltip key table for every Skirmish control.` (category: `out-of-scope`; reason: this slot verifies status plumbing/source order, not exhaustive tooltip copy; next-step-if-pursued: dedicated status key inventory report)

## 8. Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md` replacement wording for the "Right-panel static text controls are not represented as first-class layout rects" mismatch: "Current Rust now has first-class `SkirmishRightPanelTextRects` for title, game type, and map/scenario label and renders them through the shell text path; remaining right-panel text gaps are kind-1 reveal/restart timing and screenshot-level color validation."
- `docs/research/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md` replacement wording for current right-panel status: "Current Rust's right-panel title/game/map static rendering exists and is top-anchored; do not describe right-panel text as absent. Status/help child `0x695` remains absent as a first-class layout/render/state surface."
- `docs/research/skirmish-ui/SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md` replacement wording for current color status: "Current Rust now decodes `0x00000C05` as RGB `(5,12,0)` and `SHELL_LABEL_TEXT_RGB` as yellow `(255,255,0)`; keep the binary source-color finding, but do not describe current constants as still channel-swapped or muted unless a later source scan says so."
- `docs/research/skirmish-ui/SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md` remains current for the status/help gap: current Rust still lacks named `0x695` layout/render/state surfaces in this scan.

## Sources

- Ghidra read-only decompile: `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_00622B50 @ 0x00622B50`, `OwnerDraw_Static_006153E0 @ 0x006153E0`, `FUN_0060A5B0 @ 0x0060A5B0`, `FUN_00602490 @ 0x00602490`, `FUN_005E2EF0 @ 0x005E2EF0`, `FUN_005E2F60 @ 0x005E2F60`, `FUN_0060B550 @ 0x0060B550`, `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`, `FUN_006071E0 @ 0x006071E0`, `FUN_006ACEE0 @ 0x006ACEE0`, `FUN_00621040 @ 0x00621040`.
- Ghidra read-only assembly contexts: `0x00622CCB`, `0x00622E6D..0x00622E83`, `0x00615AE8`, `0x00611BC1..0x00611C67`, `0x00611C72..0x00611CAF`, `0x0060C2B6..0x0060C2C9`.
- Prior docs referenced: `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`, `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/mod.rs`, `src/app_skirmish_shell_render.rs`, `src/render/shell_text.rs`.
