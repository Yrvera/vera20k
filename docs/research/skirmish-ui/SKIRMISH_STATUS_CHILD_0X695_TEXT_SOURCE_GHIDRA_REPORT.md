# Skirmish Status Child 0x695 Text Source - Ghidra Research Report

**Address(es):** `0x006AE3F0`, `0x00622B50`, `0x006040B0`, `0x0060B550`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard offline Yuri's Revenge Skirmish dialog `0x102` child static `0x695`: text source, update path, final rect, normal visibility/blank state, and Rust handoff.
**Non-Scope:** Full dialog child matrix, Choose Map `0x6B` status child behavior, online host/guest status strips, every shell dialog using id `0x695`, and runtime screenshot validation.
**Confidence:** High for active path, hover update source, final rect formula, and Rust delta; Medium for "only active 0x102 update site" because this pass bounded the standard offline Skirmish proc path rather than proving whole-binary absence.
**Active in YR:** Yes, for standard offline Skirmish dialog `0x102` reached through `0x006AE2C0`.

## 0. Working Notes Gate

- Target question: What is offline Skirmish dialog `0x102` child `0x695`, where does its text come from, when is it updated, where is it anchored, and what should Rust do?
- Non-goals: Do not re-audit all 72 children, do not re-cover right-panel labels `0x6EC/0x5A8`, do not implement Rust, do not mutate Ghidra.
- Evidence needed to mark COMPLETE: active `0x102` entry path, `0x695` resize/anchor proof, `0x695` owner/static classification, hover/status update path into `0x4B2`, tooltip key dispatcher proof, current Rust surface scan, and TS-vs-YR liveness check.
- Stop conditions: Stop when all scoped open questions are resolved or explicitly deferred, no new scoped callees appear in the final pass, and the report has at least one implementation handoff.

## 1. Overview

Child `0x695` is the bottom-left shell help/status strip for the standard offline Skirmish setup dialog. It is a visible static child with blank initial/resource text, but the common shell parent handler updates it on cursor hit-testing by asking the hovered child/control for status text and then sending dynamic text message `0x4B2` to `0x695`.

Player-visible result: the strip exists at the bottom-left of the full-screen shell and is normally blank when no hovered control produces help text. Moving over Skirmish controls can populate it with localized `STT:*` help text or item-specific combo text.

## 2. Class Layout / Key Offsets

| Field / value | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Dialog id `0x102` | Standard offline Skirmish setup dialog | `0x006AE31C..0x006AE328` loads proc `0x006AE3F0`, id `0x102`, calls `0x00622650` | Yes |
| Child id `0x695` | Bottom-left status/help static | RT_DIALOG matrix report row 42; `0x00622CCB` `GetDlgItem(parent,0x695)` | Yes |
| Static record `+0x70` / `piVar[0x1c]` | Owner-draw static mode; value `1` means text/reveal static | `FUN_0060A5B0` sets `piVar[0x1c]=1` when `FUN_00602490` accepts `0x695` | Yes, conditional on normal shell branch |
| Static record `+0x28` / `piVar[10]` | Heap-owned wide text pointer consumed during paint | Prior static-thunk report, assembly `0x00611BC1..0x00611C67` | Yes |
| Empty wide string `0x00887734` | Fallback text when no hover/status text is available | `FUN_007B7140`; `0x00622E40` pushes `0x00887734` | Yes |

## 3. Core Logic

### 3.1 Active Offline Skirmish Entry

Active in YR: Yes. `FUN_006AE2C0` is the standard offline Skirmish launcher. Its assembly at `0x006AE31C..0x006AE328` sets the dialog procedure to `0x006AE3F0`, sets dialog id `0x102`, passes a zero init param, and calls the shell dialog creation helper `0x00622650`. The main loop then runs until command `0x617` Start Game or `0x5C0` Back.

### 3.2 Parent Handler Status Update

Active in YR: Yes. `FUN_006AE3F0` calls `FUN_00622B50` before Skirmish-specific message handling (`0x006AE40A`). In `FUN_00622B50`, message `0x84` (`WM_NCHITTEST`) is the scoped status update path:

1. Get child `0x695` from the parent (`0x00622CCB..0x00622CD9`).
2. Convert screen cursor coordinates to parent client coordinates and find the child under the cursor with `ChildWindowFromPointEx(parent, point, 1)` (`0x00622CED..0x00622D2B`).
3. Send custom message `0x4E8` to the hovered child and pass its result through the child/parent status chain (`0x00622D4B..0x00622D5E`).
4. If that produces non-empty text, use it.
5. Otherwise, send parent message `0x4E9` with `{hovered_hwnd, -1}` so the Skirmish proc can synthesize item-specific text (`0x00622DB0..0x00622DE6`).
6. If still empty, call `FUN_006040B0(parent, hovered_child)` to map the dialog id and child id to an `STT:*` key (`0x00622E1D..0x00622E38`).
7. If no key exists, use empty string `0x00887734` (`0x00622E40`).
8. Convert the selected string holder to a wide pointer with `FUN_007B7140` and send `SendMessageA(status_hwnd, 0x4B2, 0, wide_text)` (`0x00622E6D..0x00622E83`).

### 3.3 Skirmish-Specific Parent `0x4E9` Sources

Active in YR: Yes. `FUN_006AE3F0` handles `0x4E9` after the common parent handler delegates to it. It clears a temporary wide string (`FUN_007B6880(0)`). For AI row-state combo controls `0x50B`, `0x50E`, `0x516`, `0x51A`, `0x51B`, `0x51C`, and `0x51D`, it resolves the hovered/current combo item and writes one of four localized strings:

| Item data | Source string id in `Skirmish.cpp` load path | Meaning from prior row-state report | Evidence | Active in YR |
|---:|---|---|---|---|
| `-1` | `0x87` | `STT:PlayerNone` | `FUN_006AE3F0`, `0x006AE4A2..0x006AE4B8`; prior AI-row report | Yes |
| `2` | `0x89` | `STT:PlayerDumbAI` | `FUN_006AE3F0`, `0x006AE4BA..0x006AE4CE`; prior AI-row report | Yes |
| `1` | `0x8B` | `STT:PlayerSmartAI` | `FUN_006AE3F0`, `0x006AE4D0..0x006AE4E4`; prior AI-row report | Yes |
| `0` | `0x8D` | `STT:PlayerGeniusAI` | `FUN_006AE3F0`, `0x006AE4E6..0x006AE4FA`; prior AI-row report | Yes |

For other scoped combo/list families, `FUN_006AE3F0` attempts item-specific text through combo/list helper functions (`FUN_004E3830`, `FUN_004E4230`, `FUN_004E4EC0`, and related getters). This report did not expand those helper families because the target is the status child plumbing, not every combo item label source.

### 3.4 Fallback Tooltip Key Dispatcher

Active in YR: Yes. `FUN_006040B0(parent, hovered_child)` reads the parent dialog id from the owner-draw record and switches on dialog `0x102`. For standard Skirmish it maps controls to `STT:Skirmish*` keys for player edit, AI/player combos, flags, side/color/start/team combos, trackbars, buttons, checkboxes, preview, and right-panel labels.

Negative scoped point: `0x695` itself is not mapped in the `0x102` branch. Hovering the status strip itself falls through to null and then the empty string fallback. Evidence: full decompile of `FUN_006040B0`; no `0x695` case in the `iVar4 == 0x102` branch. Active in YR: Yes.

### 3.5 Text Copy and Paint Refresh

Active in YR: Yes. The actual `0x4B2` text ownership is the shared subclass thunk, not `OwnerDraw_Static_006153E0`. Prior static-thunk report verifies `0x00611BC1..0x00611C67` copies incoming wide text into the owner-draw record, frees old text when needed, and resets text state. `OwnerDraw_Static_006153E0` then handles `0x4B2/0x4B4` by refreshing its cached backing surface and invalidating if the backing surface exists. Evidence: prior static-thunk report plus decompile of `OwnerDraw_Static_006153E0 @ 0x006153E0`.

### 3.6 Anchor and Final Rect

Active in YR: Yes. `ResizeShellChildControl_0060C0C0` gets each child id and, when parent dialog id is in the allowlist and child id is `0x695`, calls `FUN_0060B550` (`0x0060C2B6..0x0060C2C5`). `FUN_0060B550` preserves the child size and computes:

- normal shell branch: `x = center_x + 10`, `y = screen_h - child_h - center_y - 1`
- alternate/scenario branch: same formula with zero center offsets

For standard offline Skirmish normal shell, the verified final rects are:

| Screen | Final rect | Evidence | Active in YR |
|---|---:|---|---|
| `640x480` | `(10,459,615,20)` | Matrix report row 42; `FUN_0060B550` | Yes |
| `800x600` | `(10,579,615,20)` | Matrix report row 42; `FUN_0060B550` | Yes |
| `1024x768` | `(122,663,615,20)` | Matrix report row 42; `FUN_0060B550` | Yes |

### 3.7 Visibility and Normal Blank State

Active in YR: Yes. The child is visible in RT_DIALOG `0x102` according to the complete child matrix report, and `FUN_00602490` classifies `0x695` as a text/reveal static for allowlisted shell dialogs when `FUN_0069BBE0()` is false. The initial/resource text is blank per the matrix report's resource row ("status/tooltip blank"). The active parent handler then replaces it with hover/status text or the empty wide string fallback.

Normal player-visible implication: on first paint with no hover-derived help text, the strip is visible but text-blank; it should not show a hardcoded "Status", "Help", map name, or game mode. It becomes non-blank only when the cursor is over a control that yields a status string through `0x4E8`, `0x4E9`, or `FUN_006040B0`.

## 4. INI Keys

No INI keys directly control child `0x695` text source, visibility, anchoring, or fallback content in this slice. Tooltip/status strings are CSF/string-table keys (`STT:*`) and shell resource/layout behavior, not `rules.ini`/`art.ini` data.

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006AE2C0` | Offline Skirmish launcher and modal loop | decompile; assembly `0x006AE31C..0x006AE328` | Yes |
| `FUN_006AE3F0` | Skirmish dialog proc; delegates common handling and synthesizes `0x4E9` item text | decompile `0x006AE3F0` | Yes |
| `FUN_00622B50` | Common shell parent handler; performs hover hit-test and writes status text to `0x695` | decompile; assembly `0x00622CCB..0x00622E83` | Yes |
| `FUN_006040B0` | Dialog/control id to `STT:*` key dispatcher | decompile `0x006040B0` | Yes |
| `FUN_0060B550` | Bottom-left status child placement helper | decompile `0x0060B550` | Yes |
| `0x00610CA0` thunk | Dynamic `0x4B2` text copy into owner-draw record | prior static-thunk report, assembly `0x00611BC1..0x00611C67` | Yes |

## 6. Current Rust Implementation Status

Rust currently has no named layout field or render/state path for Skirmish status child `0x695`.

- `src/ui/skirmish_shell/layout.rs`: `SkirmishShellLayout` exposes right-panel text, buttons, preview, labels, rows, color combos, flags, trackbars, and checkboxes, but no status/help strip field.
- `src/app_skirmish_shell_render.rs::build_shell_text_draws`: renders button labels, column labels, right-panel title/game type/map, player name, checkboxes, trackbar values, combo face labels, and dropdown text. It does not render a bottom-left status strip.
- `src/render/shell_text.rs`: provides the text primitive needed to draw such a strip; no status-specific state/source is present.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish creates dialog `0x102` with proc `0x006AE3F0` | verified | `0x006AE31C..0x006AE328` | none |
| Parent common handler is active for Skirmish proc | verified | `FUN_006AE3F0` calls `FUN_00622B50` at `0x006AE40A` | none |
| `0x695` bottom-left resize route | verified | `ResizeShellChildControl_0060C0C0`, `0x0060C2B6..0x0060C2C5`; `FUN_0060B550` | none |
| Final rects at 640/800/1024 | verified-by-prior-plus-spot-check | matrix report row 42; `FUN_0060B550` | none |
| Initial/resource blank status | verified-by-prior | matrix report row 42 says "status/tooltip blank"; RT_DIALOG resource inventory | runtime screenshot optional |
| `0x695` static text/reveal classification | verified | `FUN_00602490`; `FUN_0060A5B0` | none |
| Hover hit-test update into `0x695` | verified | `FUN_00622B50`, `0x00622CCB..0x00622E83` | none |
| Skirmish-specific `0x4E9` AI combo item text | verified | `FUN_006AE3F0`, `0x006AE4A2..0x006AE4FA`; prior AI-row report | broader combo/list helper families out of scope |
| Fallback tooltip keys for standard Skirmish controls | verified | `FUN_006040B0` | none for scoped controls |
| Absence of `0x695` self-tooltip mapping | verified | `FUN_006040B0` has no `0x695` case in dialog `0x102` branch | none |
| Whole-binary non-`0x102` uses of id `0x695` | deferred | byte-pattern scan found other dialogs/procs | out of scope for standard offline Skirmish |
| Current Rust status | verified | source scan of `layout.rs`, `app_skirmish_shell_render.rs`, `shell_text.rs` | implementation later |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Is dialog 0x102 active in standard offline YR Skirmish? -> Yes, launcher `0x006AE2C0` creates id 0x102 with proc 0x006AE3F0.` (evidence: `0x006AE31C..0x006AE328`)
- `[RESOLVED] OQ-002 - Does the Skirmish proc run the common parent status handler? -> Yes, it calls `FUN_00622B50` before Skirmish-specific handling.` (evidence: `0x006AE40A`)
- `[RESOLVED] OQ-003 - Where is child 0x695 obtained for status updates? -> `FUN_00622B50` calls `GetDlgItem(parent,0x695)` in its `WM_NCHITTEST` branch.` (evidence: `0x00622CCB..0x00622CD9`)
- `[RESOLVED] OQ-004 - What event updates status text? -> Parent `WM_NCHITTEST`/cursor hit-testing, not an explicit Skirmish tick.` (evidence: `FUN_00622B50`, `0x00622B53`, `0x00622CCB..0x00622E83`)
- `[RESOLVED] OQ-005 - What is the final write into 0x695? -> `SendMessageA(status_hwnd,0x4B2,0,wide_text)`.` (evidence: `0x00622E6D..0x00622E83`)
- `[RESOLVED] OQ-006 - What text source is tried first? -> The hovered child receives custom `0x4E8`, then the parent/child chain can provide status text.` (evidence: `0x00622D4B..0x00622D5E`, `FUN_00603F00`)
- `[RESOLVED] OQ-007 - What Skirmish-specific source exists? -> Parent `0x4E9` handles AI row-state combos and selected item values into localized strings.` (evidence: `FUN_006AE3F0`, `0x006AE4A2..0x006AE4FA`)
- `[RESOLVED] OQ-008 - What fallback source exists? -> `FUN_006040B0` maps dialog/control ids to `STT:*` keys, loaded through `StringTable__LoadString`.` (evidence: `0x00622E1D..0x00622E38`, `FUN_006040B0`, `StringTable__LoadString @ 0x00734E60`)
- `[RESOLVED] OQ-009 - What happens if no source text exists? -> Empty wide string `0x00887734` is sent to `0x695`.` (evidence: `0x00622E40`, `FUN_007B7140`)
- `[RESOLVED] OQ-010 - Does `0x695` itself have a tooltip mapping? -> No, no `0x695` case exists in the `0x102` branch of `FUN_006040B0`.` (evidence: decompile `0x006040B0`)
- `[RESOLVED] OQ-011 - What is the anchor formula? -> bottom-left helper preserves size and places `x=center_x+10`, `y=screen_h-child_h-center_y-1` in the normal shell branch.` (evidence: `FUN_0060B550`)
- `[RESOLVED] OQ-012 - Is `0x695` visible? -> Yes, resource inventory says visible and resize matrix includes it; owner-draw static classification also accepts it.` (evidence: matrix report row 42; `FUN_00602490`)
- `[RESOLVED] OQ-013 - Is it initially blank in normal YR? -> Yes by resource/matrix evidence; runtime path later sends empty fallback when no hover text exists.` (evidence: matrix report row 42; `0x00622E40..0x00622E83`)
- `[RESOLVED] OQ-014 - Is this TS legacy or live YR? -> Live YR shell UI; no TS-only gate found on the standard offline Skirmish path.` (evidence: `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`)
- `[RESOLVED] OQ-015 - Are INI keys involved? -> No scoped INI read found or expected; sources are resource, CSF/string table, and shell control ids.` (evidence: `FUN_006040B0`, `StringTable__LoadString`)
- `[RESOLVED] OQ-016 - Does current Rust expose the status rect? -> No named field or render path for `0x695` exists in scanned surfaces.` (evidence: `layout.rs`, `app_skirmish_shell_render.rs`, `shell_text.rs` source scan)
- `[DEFERRED] OQ-017 - Do non-0x102 shell dialogs have different `0x695` update semantics?` (category: out-of-scope; reason: byte-pattern scan found other `0x695` users in other dialog procs, but this target is standard offline Skirmish `0x102`; next-step-if-pursued: investigate Choose Map or online host/guest status strips separately)
- `[DEFERRED] OQ-018 - What exact pixels appear in a runtime capture for no-hover and hover cases?` (category: needs-runtime-debugger; reason: binary static evidence proves text source and rect, but screenshot capture was not part of this read-only Ghidra slot; next-step-if-pursued: run retail YR with cursor over Start/blank area and compare strip)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Child `0x695` is a visible bottom-left status/help static with final rect `(10,459,615,20)` at 640x480, `(10,579,615,20)` at 800x600, and `(122,663,615,20)` at 1024x768 | `FUN_0060B550`; matrix report row 42 | missing named layout field | `src/ui/skirmish_shell/layout.rs` | Add an explicit status/help strip rect using the bottom-left helper formula and preserved `615x20` size | Deterministic layout tests assert all three final rects and that the strip is not right-panel anchored | Do not derive it from right-panel text rects or center it with the main 800-wide shell controls; proposed test `skirmish_status_child_0695_bottom_left_rects_match_gamemd` |
| Status text is blank until hover/status source produces text; when no source exists the parent sends empty wide string to `0x695` | `FUN_00622B50`, `0x00622E40..0x00622E83`; `FUN_006040B0` lacks a `0x695` case | missing state/render behavior | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Add optional status/help text state and render it only when non-empty; default should draw no text | Fresh Skirmish shell with no hovered control has an empty status string; hovering status strip itself remains empty | Do not hardcode "Status", "Help", map name, or game type into the strip; proposed test `skirmish_status_child_0695_defaults_blank_and_self_hover_blank` |
| Hovered controls resolve status text through child `0x4E8`, Skirmish parent `0x4E9`, then `FUN_006040B0` `STT:*` fallback | `FUN_00622B50`, `FUN_006AE3F0`, `FUN_006040B0` | missing hover/status resolver | `src/ui/skirmish_shell/state.rs`, input/hit-test code near `SkirmishShellAction`, render text path | Populate status/help text from hovered control ids using the verified `STT:Skirmish*` mapping, with AI-row item-specific strings taking precedence when hovered over AI combo rows | Hovering Start Game yields localized `STT:SkirmishButtonStartGame`; hovering AI row-state Easy/Normal/Hard items yields the item-specific player AI tooltip text | Do not use visible GUI labels as tooltip text; use `STT:*`/CSF keys and item-specific override order; proposed test `skirmish_status_child_0695_hover_uses_stt_mapping_and_ai_item_overrides` |

### Negative Facts / Do Not Do

- Do not render a permanent label in `0x695`. Active in YR: Yes. Evidence: matrix report says initial/status resource text is blank; `FUN_00622B50` sends empty `0x00887734` when no source text exists.
- Do not treat `0x695` as a right-panel static like `0x6EC` or `0x5A8`. Active in YR: Yes. Evidence: `ResizeShellChildControl_0060C0C0` routes `0x695` to `FUN_0060B550`, not `FUN_0060B1D0`.
- Do not copy visible GUI labels into the status strip as the fallback. Active in YR: Yes. Evidence: fallback mapping uses `STT:*` keys from `FUN_006040B0`, and item-specific text can override generic control tooltip text via `0x4E9`.
- Do not put the only `0x4B2` text-copy behavior in `OwnerDraw_Static_006153E0`. Active in YR: Yes. Evidence: prior static-thunk report shows the common subclass thunk owns the text copy before the owner static proc refreshes backing/invalidation.

### Stale Docs / Follow-up Docs

No stale-doc replacement wording found in this slice. The existing matrix wording "status/tooltip blank" is consistent with the newly verified update path.

## Sources

- Ghidra decompile: `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_00622B50`, `FUN_006040B0`, `FUN_0060B550`, `ResizeShellChildControl_0060C0C0`, `FUN_00602490`, `FUN_0060A5B0`, `OwnerDraw_Static_006153E0`, `StringTable__LoadString @ 0x00734E60`, `FUN_007B6880`, `FUN_007B7140`.
- Ghidra assembly contexts: `0x006AE31C..0x006AE328`, `0x00622CCB..0x00622E83`, `0x0060C2B6..0x0060C2C5`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`.
- Rust scan: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/render/shell_text.rs`.
