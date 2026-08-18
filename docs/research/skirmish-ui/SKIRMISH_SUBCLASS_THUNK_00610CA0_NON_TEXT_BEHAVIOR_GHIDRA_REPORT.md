# Skirmish Subclass Thunk 0x00610CA0 Non-Text Behavior - Ghidra Research Report

**Address(es):** `0x00610CA0` common shell subclass thunk; visible owner-proc consumers `0x006153E0`, `0x00612B70`, `0x006163A0`, `0x00617250`; Skirmish dialog proc `0x006AE3F0`; common shell proc `0x00622B50`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Non-text behavior of common shell subclass thunk `0x00610CA0` only where it affects standard offline Skirmish dialog `0x102` layout, paint, control cache invalidation, and visual state handoff.  
**Non-Scope:** Re-resolving `0x4B2`/`0x4B4` text copy plumbing, BitFont internals, full shell-wide tooltip/help semantics, full combo dropdown rendering, listbox/edit/scrollbar behavior, and runtime screenshot validation.  
**Confidence:** High for thunk dispatch, destroy cleanup, static cache invalidation, and owner-proc visual-state responsibilities; Medium for shell-global invalidation aggregation because its globals are read from assembly only and not runtime-observed.  
**Active in YR:** Yes for standard offline Skirmish dialog `0x102`, reached by `FUN_006AE3F0 -> FUN_00622B50 -> FUN_0060F9A0` installing `0x00610CA0` on parent and child shell controls.

## 1. Overview

`0x00610CA0` is the common subclass WndProc installed on shell controls. For non-text Skirmish behavior, its player-visible role is not to draw controls directly; it maintains shared per-HWND records, suppresses selected Win32 defaults, handles lifetime cleanup, and routes messages into class-specific owner procs that perform actual button, checkbox, combo, static, and parent paint invalidation.

The Rust implication is mostly negative: do not emulate a Win32 subclass chain in `sim/` or as a general UI framework. Rust needs direct state hooks for the observable state changes that the chain represents: pressed buttons, checked checkboxes, combo selected value/dropdown state, text-animation invalidation, selected-map preview texture invalidation, and teardown of cached preview/control surfaces when their inputs change.

## 2. Class Layout / Key Offsets

| Field / global | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `DAT_00AC18C0` | HWND -> owner proc table; `0x006153E0`, `0x00612B70`, `0x006163A0`, `0x00617250` are class consumers | Yes | Setup `0x0060F9A0`; thunk lookup `0x00610D0B..0x00610D4B`; owner call `0x00612318..0x0061234B` |
| `DAT_00AC1B48` | HWND -> previous WndProc table for fallback and cleanup | Yes | Setup `0x0060F9A0`; cleanup removal `0x006123D6..0x006123F0` |
| `DAT_00AC1B00` | HWND -> shared owner-draw record; record body starts at entry `+4` | Yes | Setup `0x0060F9A0`; thunk lookup `0x0061121F..0x00611289` |
| record `+0x10` | cached `BSurface` pointer used by owner-proc paint paths | Yes | Static destroy/free `0x006123BB..0x006123D1`; static proc `WM_PAINT`, `WM_MOVE`, `WM_SIZE`, `0x47` |
| record `+0x14`, `+0x18`, `+0x24`, `+0x30`, `+0x50` | shell message-settable slots; used as owner-proc/custom-message state, not ordinary Skirmish gameplay | Conditional | Thunk cases `0x49C`, `0x4AA`, `0x49A`, `0x4D1`, `0x4EB` |
| record `+0x38` | focus/activation latch toggled by `WM_SETFOCUS`/`WM_KILLFOCUS`-like messages | Conditional | Thunk `0x006118BF..0x00611907` |
| record `+0x1FC` | recursion/paint dispatch guard used around `CallWindowProcA` and descendant invalidation | Yes | Thunk `0x0061208F..0x00612104` |
| record `+0x204` | next pointer in `DAT_00AC1B00` hash chain | Yes | Setup and lookup paths; cleanup `0x0061243C..0x00612473` |
| `DAT_00AC48B4` | live cached surface count | Yes | Increment in owner procs on `BSurface` allocation; decrement at thunk cleanup `0x006123CB..0x006123D1` |

## 3. Core Logic

### 3.1 Installation And YR Activity

Active in YR: Yes. `FUN_00622B50` handles shell `WM_INITDIALOG` and, on the standard dialog-init path with a non-null parameter, enumerates child windows through `FUN_0060F9A0`, calls `FUN_0060F9A0` on the parent too, then later enumerates setup classifiers. `FUN_0060F9A0` installs `0x00610CA0` with `SetWindowLongA(hwnd, GWL_WNDPROC, 0x610CA0)` for recognized shell classes, stores the owner proc in `DAT_00AC18C0`, stores the previous proc in `DAT_00AC1B48`, creates the `DAT_00AC1B00` record, and sends `0x497`.

Evidence: `FUN_00622B50 @ 0x00622B50`; `FUN_0060F9A0 @ 0x0060F9A0`; direct setup push of `0x610CA0` documented in the prior text-thunk report. Skirmish dialog `0x102` reaches this path via `FUN_006AE3F0`, whose first action is common-shell delegation.

### 3.2 Thunk Early Suppression And Message Routing

Active in YR: Yes. The thunk immediately returns `1` for message `0x20` (`WM_SETCURSOR`), so standard cursor handling is suppressed before owner-proc dispatch. It also calls a helper at `0x00778030`/`0x00778120` before normal routing; this was not expanded because it is shell-global input infrastructure and no Skirmish `0x102` paint/control-specific effect was found in this slice.

Active in YR: Yes. The thunk treats `WM_ERASEBKGND (0x14)` as handled by setting return `1` and exiting through the common return path. This prevents Win32 background erase from clearing over the shell's cached/backbuffer paint model.

Evidence: thunk disassembly `0x00610CB0..0x00610CC5` for early `0x20`; `0x006118AD..0x006118BA` for `0x14`.

### 3.3 Shared Record Lookup And Owner Proc Dispatch

Active in YR: Yes. Every normal path looks up the owner proc from `DAT_00AC18C0`, the previous WndProc from `DAT_00AC1B48`, and the shared owner-draw record from `DAT_00AC1B00`. If the record and owner proc exist, the thunk calls the owner proc through `CallWindowProcA(ownerProc, hwnd, msg, wParam, lParam)` after shared pre-processing.

Evidence: owner-proc lookup `0x00610D0B..0x00610D4B`; shared record lookup `0x0061121F..0x00611289`; owner-proc dispatch `0x00612318..0x0061234B`.

### 3.4 Non-Text Custom Messages

Active in YR: Conditional. The thunk has non-text custom cases that set or query record fields and then return through the common path:

| Message | Behavior | Active in YR | Evidence |
|---|---|---|---|
| `0x49A` | swaps record `+0x24` with `lParam`, returns old value | Conditional | `0x00611969..0x00611982` |
| `0x49C` | swaps record `+0x14` with `lParam`, returns old value | Conditional | `0x0061192D..0x00611946` |
| `0x49D` | writes record `+0x20 = lParam`; if prior `+0x0C` HWND exists, mirrors `+0x20` into that other record | Conditional | `0x0061136B..0x006113F7` |
| `0x49E` | applies cached values at record `+0x1E8..+0x1F4` to a passed HWND through USER32-style setters | Conditional | `0x00611314..0x00611366` |
| `0x49F` | captures current window metrics into record `+0x1E8..+0x1F4` | Conditional | `0x006112B2..0x0061130F` |
| `0x4AA` | swaps record `+0x18` with `lParam`, returns old value | Conditional | `0x0061194B..0x00611964` |
| `0x4B3` / `0x4B5` | get wide/narrow text from shared record into caller buffer; not visual by itself | Yes/Conditional | `0x00611AD4..0x00611B1F`, `0x00611A8D..0x00611ACF`; Skirmish start handoff uses `0x4B3` on control `0x6A0` |
| `0x4CE` | returns whether record `+0x2C` is zero, and suppresses owner-proc dispatch | Conditional | `0x00611A6A..0x00611A88` |
| `0x4D1` | swaps record `+0x30` with `wParam`, returns old value, suppresses owner-proc dispatch | Conditional | `0x00611B9B..0x00611BBC` |
| `0x4EB` | writes record `+0x50 = lParam` | Conditional | `0x00611B24..0x00611B36` |

These cases are not layout formulas. For standard Skirmish, they matter only if a visible owner proc later reads the affected fields or if current Rust needs the same user-visible selected/checked/pressed result.

### 3.5 Static Move/Size/Destroy Cache Invalidation

Active in YR: Yes. The common thunk routes `WM_MOVE (0x03)`, `WM_SIZE (0x05)`, and `0x47` to `OwnerDraw_Static_006153E0` for static controls. The static owner proc frees cached surface record `+0x10`, sets it to null, invalidates the child with `erase=FALSE`, and returns. On `WM_DESTROY (0x02)`, the static proc frees the same cache, frees auxiliary owned data if the `+0x7C` ownership byte is set, kills timer `0` for kind-1 animated text if running and for kind-4 animation, destroys movie handles, kills timer `0x65`, and falls through to previous-proc dispatch.

Evidence: `OwnerDraw_Static_006153E0 @ 0x006153E0`, branch `param_2 == 0x47`, `param_2 == 3 || param_2 == 5`, and `param_2 == 2`.

Rust interpretation: resize/reposition should invalidate/rebuild direct render artifacts derived from layout and selected map preview; Rust does not need a per-HWND `BSurface`, but must not keep stale preview/control geometry after render size or selected map changes.

### 3.6 Thunk Destroy Cleanup

Active in YR: Yes. After owner-proc dispatch, the thunk has a `0x82` cleanup branch. It frees record `+0x10` if still present, decrements `DAT_00AC48B4`, removes the HWND from `DAT_00AC1B48`, constructs a blank local record and removes the HWND from `DAT_00AC1B00`, removes it from `DAT_00AC18C0`, frees any `GWL_USERDATA (-0x15)` payload, sets that slot to zero, and calls a final window cleanup helper.

Evidence: `0x0061234F..0x006124B8`, especially cache free/decrement `0x006123BB..0x006123D1`, previous-proc removal `0x006123D6..0x006123F0`, record removal `0x0061243C..0x00612473`, owner-proc removal `0x00612478..0x00612487`, user-data reset `0x0061248C..0x006124AE`.

Rust interpretation: lifetime cleanup belongs in app/UI resource ownership. It must not be modeled as simulation state and must not leak stale selected-map preview textures or shell atlas references across shell exit/re-entry.

### 3.7 Parent/Descendant Invalidation Aggregation

Active in YR: Yes, but shell-global. The thunk maintains a temporary HWND list around message `0x4A9` and selected paint/input messages, invalidates intersecting descendants, and guards recursion with record `+0x1FC`. This is used by the shell to coordinate invalidation of the parent cached surface and children after changes. In standard Skirmish, the player-visible equivalent is that parent paint and child owner-proc paints are refreshed in the same shell frame rather than waiting for an unrelated repaint.

Evidence: `0x00611407..0x0061160B` for `0x4A9` list update/invalidation; `0x00611EAD..0x0061230E` for intersecting-child invalidation and recursion guard; `FUN_00622B50` `WM_PAINT` calls `WM_PAINT_Handler` then validates parent.

Rust interpretation: direct redraw each frame already covers much of this. State-changing actions must still invalidate/recreate cached resources keyed by content, such as selected preview textures.

### 3.8 Visual Control State Hooks

Active in YR: Yes. Button, checkbox, combo, and static visuals are owner-proc state machines reached through the thunk, not implemented inside the thunk itself.

- Buttons: `OwnerDraw_Button_00612B70` toggles visual pressed/timer state, plays click sound on mouse down/double-click, snapshots/restores backing surface, and paints based on the button state bit. Evidence: `0x00612B70`, especially `WM_TIMER`, mouse `0x201/0x203`, and `WM_PAINT`.
- Checkboxes: `OwnerDraw_Checkbox_006163A0` stores checked state at record `+0xE8`, toggles only when the click is inside the first `18x18` pixels, invalidates, plays click sound, and sends parent `WM_COMMAND` with checked state in the high word. It handles `BM_GETCHECK (0xF0)` and `BM_SETCHECK (0xF1)`. Evidence: `0x006163A0`, click branch and `0xF0/0xF1`.
- Combos: `OwnerDraw_ComboBox_00617250` uses previous WndProc for native combo storage, creates/destroys custom dropdown windows, sends itself `0x4B2`/`0x4B4` when selection changes, and invalidates the combo. Evidence: `0x00617250`, selection-change branch `0x14E`, dropdown open/close `0x14F`, paint `0x0F`.
- Statics: `OwnerDraw_Static_006153E0` owns cached background surface and animation timers; its non-text invalidation is described above. Evidence: `0x006153E0`.

Rust interpretation: the state hooks are needed at the UI state level, not as Win32 messages. Current Rust has `pressed_owner_draw_button` and selected preview caching, but it does not yet model Skirmish checkbox visual state, combo dropdown windows, combo selected-label rendering, or static text reveal timers.

## 4. INI Keys

No INI keys control this thunk slice. The behavior is shell HWND/message/owner-draw state driven. Active in YR: Yes, because it is reached from standard Skirmish dialog creation, not from optional INI gates.

## 5. Integration Points

| Integration point | Active in YR | Evidence | Rust relevance |
|---|---|---|---|
| `FUN_006AE3F0` delegates first to `FUN_00622B50` | Yes | `0x006AE3F0` decompile | Skirmish-specific preview paint happens after common shell paint |
| `FUN_00622B50` `WM_INITDIALOG` installs thunk on children and parent | Yes | `0x00622F2C..0x00623071` decompile | Rust can initialize direct shell state without HWND subclassing |
| `FUN_00622B50` `WM_PAINT` calls common `WM_PAINT_Handler`, validates parent, then returns `0` for Skirmish-specific preview paint | Yes | `0x00622C4F`; prior common-parent report | Parent/background draw order belongs in render path |
| `FUN_006AE3F0` Skirmish `WM_PAINT` draws preview/start positions after common paint when `DAT_00AC1154 != 0` | Yes | `0x006AE3F0`; `DrawStartPositions @ 0x00640710` prior reports | Preview should be parent-level render content, not child static render |
| `OwnerDraw_*` callbacks receive routed messages from thunk | Yes | thunk dispatch `0x00612318..0x0061234B` | Direct Rust state changes should mirror observable owner-proc results |

## 6. Current Rust Implementation Status

Current Rust has no Win32 subclass emulation and should not add one. The implemented surfaces are direct shell state and direct render passes:

- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:37` stores Skirmish shell state, including `pressed_owner_draw_button` at line `44`.
- `C:/Users/enok/Documents/ra2-rust-game/src/app.rs:563` sets `pressed_owner_draw_button` on mouse down and `C:/Users/enok/Documents/ra2-rust-game/src/app.rs:574` consumes it on mouse up.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:570` draws the three owner-draw buttons from `pressed_owner_draw_button`.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:765` caches preview textures by selected map index and rebuilds when the selected map changes.
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:177` currently implements color combos as immediate cycling, not as a dropdown window with native combo state.

Missing or unchecked against this report:

- Checkbox visual/control state for Skirmish options is not represented in `SkirmishShellState`.
- Combo dropdown open/close and selected-label paint are not represented; current `SelectColor` cycles on click.
- Static text reveal/timer state for Skirmish labels is not represented.
- Teardown/re-entry cache cleanup for shell preview/chrome resources is not proven by a targeted test.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00610CA0` function boundary | verified | read-only disassembly from `0x00610CA0`; prior report notes no Ghidra function boundary | none; no mutating function creation performed |
| Setup route into thunk | verified | `0x0060F9A0`, `0x00622B50`, `0x006AE3F0` | none |
| Owner-proc and shared-record lookup | verified | `0x00610D0B..0x00610D4B`, `0x0061121F..0x00611289` | none |
| Non-text custom set/get cases | verified | `0x006112B2..0x00611BBC` | semantic names for some fields remain out-of-scope |
| Text copy cases `0x4B2/0x4B4` | verified-by-prior | prior static-text report | not re-covered |
| Static move/size/cache invalidation | verified | `OwnerDraw_Static_006153E0` `0x47`, `WM_MOVE`, `WM_SIZE` branches | none for static owner proc |
| Thunk destroy cleanup | verified | `0x0061234F..0x006124B8` | none |
| Parent/descendant invalidation aggregation | touched-not-exhausted | `0x00611407..0x0061160B`, `0x00611EAD..0x0061230E` | exact shell-global helper names and runtime order under every nested invalidation |
| Button visual state route | verified | `OwnerDraw_Button_00612B70` | exact asset/pixel details covered by prior button reports |
| Checkbox visual state route | verified | `OwnerDraw_Checkbox_006163A0` | which Skirmish checkboxes set left/right variant flags via `0x4E5/0x4E6` |
| Combo visual/dropdown route | verified | `OwnerDraw_ComboBox_00617250` | full dropdown row pixel behavior covered by separate reports |
| Rust direct-state surface scan | verified | `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | implementation remains future work |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is `0x00610CA0` active in standard offline Skirmish `0x102`? -> Yes; `FUN_00622B50` installs it through `FUN_0060F9A0`, and `FUN_006AE3F0` delegates to the common shell proc.` (evidence: `0x00622B50`, `0x0060F9A0`, `0x006AE3F0`)
- `[RESOLVED] OQ2 - Does the thunk draw Skirmish controls itself? -> No; it pre-processes, maintains records, and dispatches to owner procs for visible drawing.` (evidence: owner-proc call `0x00612318..0x0061234B`)
- `[RESOLVED] OQ3 - Does `WM_ERASEBKGND` matter visually? -> Yes; the thunk returns handled for `0x14`, preserving shell backbuffer/cached paint behavior.` (evidence: `0x006118AD..0x006118BA`)
- `[RESOLVED] OQ4 - Where are static cached backing surfaces invalidated on move/size? -> Static owner proc frees record `+0x10` on `0x47`, `WM_MOVE`, and `WM_SIZE`, then invalidates child.` (evidence: `OwnerDraw_Static_006153E0 @ 0x006153E0`)
- `[RESOLVED] OQ5 - Where are shared control records cleaned up? -> Thunk `0x82` branch frees cache, removes previous/owner/shared records, resets user data, and calls cleanup helper.` (evidence: `0x0061234F..0x006124B8`)
- `[RESOLVED] OQ6 - Are Skirmish button visuals a thunk state or owner-proc state? -> Owner-proc state; the thunk routes messages, while `OwnerDraw_Button_00612B70` paints and handles click/timer state.` (evidence: `0x00612B70`; dispatch `0x00612318..0x0061234B`)
- `[RESOLVED] OQ7 - Are Skirmish checkbox visuals a thunk state or owner-proc state? -> Owner-proc state; checkbox proc handles checked state, invalidation, click sound, and parent `WM_COMMAND`.` (evidence: `0x006163A0`)
- `[RESOLVED] OQ8 - Are Skirmish combo visuals a thunk state or owner-proc state? -> Owner-proc/native combo hybrid; combo proc uses previous WndProc for native storage, custom dropdown windows, self `0x4B2/0x4B4`, and invalidation.` (evidence: `0x00617250`)
- `[RESOLVED] OQ9 - Does this require Rust `sim/` hooks? -> No; all findings are shell UI/render state and must stay outside `sim/`.` (evidence: project architecture rule; Rust scan paths in Section 6)
- `[RESOLVED] OQ10 - Does current Rust already have a direct button state hook? -> Yes; `pressed_owner_draw_button` is set on mouse down and rendered by `push_button_30`.` (evidence: `src/ui/skirmish_shell/state.rs:44`, `src/app.rs:563`, `src/app_skirmish_shell_render.rs:570`)
- `[RESOLVED] OQ11 - Does current Rust model combo dropdown behavior? -> No; color combos cycle immediately on `SelectColor`.` (evidence: `src/ui/skirmish_shell/state.rs:177`)
- `[RESOLVED] OQ12 - Does current Rust model checkbox visual state? -> No Skirmish checkbox state fields exist in `SkirmishShellState`.` (evidence: `src/ui/skirmish_shell/state.rs:37`)
- `[DEFERRED] OQ13 - Exact helper semantics at `0x00778030`/`0x00778120` before thunk routing.` (category: out-of-scope; reason: no Skirmish `0x102` layout/paint/control invalidation effect found in this slice; next-step-if-pursued: shell-global input/cursor investigation)
- `[DEFERRED] OQ14 - Exact runtime names for shell-global invalidation aggregation globals `DAT_00AC1CC8..DAT_00AC1DD0`.` (category: bounded-cost-too-high; reason: assembly confirms invalidation behavior but full naming requires a separate shell-global pass; next-step-if-pursued: trace writers/readers of `DAT_00AC1CC8`, `DAT_00AC1DCC`, `DAT_00AC1DD0`)
- `[DEFERRED] OQ15 - Which Skirmish checkboxes use `0x4E5/0x4E6` left/right art variant flags in normal setup.` (category: requires-different-system-context; reason: checkbox visual proc verified, but setup writers are outside this thunk slot; next-step-if-pursued: focused checkbox owner-draw setup pass)

Deferred items do not block the claimed slice: the report verifies which thunk/owner-proc behaviors must be represented for standard Skirmish visuals and which details should not become Rust architecture.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Static/control cached backing surfaces are invalidated on move/size/custom reset and freed on destroy; selected preview repaint is parent-owned, not child-static-owned | Static `0x47`/`WM_MOVE`/`WM_SIZE` in `0x006153E0`; thunk destroy `0x0061234F..0x006124B8`; Skirmish paint `0x006AE3F0` | partial: preview texture cache is keyed by map index, but no explicit shell teardown/re-entry test | `src/app_skirmish_shell_render.rs`, `AppState` shell resource lifecycle | Keep cache ownership in app/render state; rebuild preview resources when selected map/render geometry changes and clear shell-only resources on shell exit/re-entry | Enter Skirmish, render Dustbowl preview, choose/cycle map, resize 800x600 -> 1024x768 -> 800x600, exit and re-enter; no stale old-map preview or old-geometry artifact appears | Do not model Win32 `BSurface` caches or invalidation lists in `sim/`; proposed test name: `skirmish_shell_preview_cache_rebuilds_on_map_resize_and_reentry` |
| Button visuals are owner-proc state: click down selects down assets, click release activates only if released over same button, and non-text thunk only routes/suppresses background erase | Thunk dispatch `0x00612318..0x0061234B`; button proc `0x00612B70`; thunk `WM_ERASEBKGND` `0x006118AD..0x006118BA` | mostly present for three buttons: `pressed_owner_draw_button` state and render selection exist | `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | Preserve direct pressed-state hook; add regression test that drag-off cancels action but still releases visual state | Mouse down on Start Game, move/release over Choose Map: neither Start nor Choose fires, and all buttons render unpressed on next frame | Do not dispatch synthetic Win32 messages to reproduce this; proposed test name: `skirmish_owner_draw_button_release_must_match_pressed_control` |
| Combo and checkbox visible state belongs in direct UI state; combo selection invalidates/redraws selected label/dropdown, checkbox clicks only toggle inside the first `18x18` box and send parent option state | Combo proc `0x00617250`; checkbox proc `0x006163A0` click and `BM_GETCHECK/BM_SETCHECK`; thunk route `0x00612318..0x0061234B` | missing: Skirmish combos currently cycle colors immediately; checkbox visual state is not represented in `SkirmishShellState` | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, input handling in `src/app.rs` | Add UI state for checkbox checked values and combo dropdown/open/selection rendering; update labels/swatch visuals immediately on state change | Click Short Game checkbox text outside the `18x18` box should not toggle; click inside toggles and redraws checked art; opening a color combo shows dropdown rows and selecting a row updates the swatch/label without stale cached text | Do not hide this behind the common thunk text buffer; proposed test name: `skirmish_checkbox_and_combo_state_update_redraws_without_subclass_emulation` |

### Negative Facts / Do Not Do

- Do not emulate `0x00610CA0` as a general Win32 subclass layer in Rust. Evidence: visible drawing is in owner procs (`0x006153E0`, `0x00612B70`, `0x006163A0`, `0x00617250`), while the thunk mostly routes and maintains HWND records.
- Do not put shell-control state in `sim/`. Evidence: all verified paths are USER32 HWND/message/render cache paths, active before game launch in dialog `0x102`.
- Do not let `WM_ERASEBKGND`-style clearing wipe the Skirmish shell between child paints. Evidence: thunk returns handled for `0x14` at `0x006118AD..0x006118BA`.
- Do not implement the map preview as the child static `0x468` painting its own content. Evidence: `FUN_006AE3F0` parent `WM_PAINT` calls preview/start-position drawing after common paint; prior static report confirms `0x468` is an anchor/placeholder.
- Do not treat `0x4B3`/`0x4B5` as text setters. Evidence: thunk copies record text out into caller buffers at `0x00611AD4..0x00611B1F` and `0x00611A8D..0x00611ACF`; setters are the already documented `0x4B2`/`0x4B4`.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`: replace the final-state `OQ5` wording with: "`[RESOLVED] OQ5 - The common subclass thunk at `0x00610CA0` copies dynamic `0x4B2` text at `0x00611BC1..0x00611C63`; non-text thunk behavior affecting Skirmish layout/paint is covered separately by `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`.`"
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`: replace "Later paints reuse that backing surface unless message `0x47`, `WM_SIZE`, `WM_MOVE`, or destroy cleanup resets it" with: "Later paints reuse that backing surface; `OwnerDraw_Static_006153E0` resets it on `0x47`, `WM_MOVE`, and `WM_SIZE`, while common thunk `0x00610CA0` also frees any remaining record `+0x10` surface during `0x82` cleanup."

### Remaining Uncertainty

- Exact helper semantics at `0x00778030`/`0x00778120` remain unresolved because they are shell-global input/cursor infrastructure and no Skirmish `0x102` layout/paint/control invalidation effect was found in this slice.
- Exact names and runtime ordering for shell-global invalidation aggregation globals `DAT_00AC1CC8..DAT_00AC1DD0` remain partial; assembly confirms intersecting-descendant invalidation, but a shell-global writer/reader pass would be needed for complete naming.
- Which standard Skirmish checkboxes set checkbox-art variant flags via `0x4E5/0x4E6` remains outside this thunk slice.

## Sources

- Ghidra read-only disassembly: `0x00610CA0..0x006124B8`.
- Ghidra read-only decompile: `FUN_0060F9A0 @ 0x0060F9A0`, `OwnerDraw_Static_006153E0 @ 0x006153E0`, `OwnerDraw_Button_00612B70 @ 0x00612B70`, `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `FUN_00622B50 @ 0x00622B50`, `FUN_006AE3F0 @ 0x006AE3F0`.
- Prior docs checked: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/SHELL_SUBCLASS_THUNK_00610CA0_TEXT_UPDATE_PLUMBING_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`.
- INI files checked: none; this slice is shell HWND/message/render-cache behavior, not INI-driven.
