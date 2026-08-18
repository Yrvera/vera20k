# Shell Dialog Lifetime Transition and Reveal Parity Implementation Plan

> **For Codex:** Execute this plan task-by-task. Tasks 1 and 2 are a mandatory
> evidence and review gate. Do not edit Rust until both tasks pass.

**Goal:** Replace screen-edge shell animation inference with explicit dialog
lifetime entry/close transitions, exact known control schedules, modal parent
preservation, paint-driven static reveals, and gamemd-derived verification for
the rendered `0xE2`, `0x100`, `0x102`, and `0x6B` shell surfaces.

**Architecture:** `ui::shell::slide` owns immutable, render-agnostic dialog
specifications and frame arithmetic. A new pure app-layer
`ShellTransitionController<PendingShellAction>` owns `Idle`, `Entering`, and
`Closing` lifetime states. The app creates and closes lifetimes explicitly,
renderers consume an immutable transition frame keyed by native control ID, and
the app advances state only after the composed frame is submitted and presented.
No `sim/` file or simulation ordering is touched.

**Design Doc:**
`docs/plans/2026-07-18-shell-dialog-lifetime-transition-design.md`

---

## Execution Gate

This plan intentionally has two stages because seven pixel/mechanism qualifiers
are not yet proven strongly enough to encode:

1. the relative `EnumChildWindows` order of Choose Map controls `0x6C5` and
   `0x583` (the count, special Cancel timing, and total duration are already
   closed);
2. exact BITFONT Path-A integer tint and native wide-character arithmetic;
3. exact SDMPBTN/SDWRNTMP per-phase frame selection and draw order under the
   corrected `DL` direction semantics; and
4. the identity of the close-start sound plus the exact timeout/error branch;
5. the non-`0x102` kind-1 static membership/default tuples;
6. the exact effect of redundant or externally caused static repaints; and
7. native Escape routing plus the nonanimated transition composition boundary.

Task 1 produces the missing active-`gamemd.exe` evidence. Task 2 turns that
evidence into an implementation contract, inserts the proven literals into the
evidence-dependent tasks in this document, and runs `/review-plan`. Tasks 3–17
may begin only after the review is green. If Task 1 contradicts the approved
design, stop and revise the design before touching Rust.

This is not a discretionary pause. Inventing any of these values would violate
the project's exact-mechanism and no-small-disparity rules.

---

## Grounding Summary

- The common shell subclass starts SHOW on the first `WM_PAINT` of each new
  allowlisted dialog lifetime. The rendered in-scope IDs are `0xE2`, `0x100`,
  `0x102`, and `0x6B`.
- Generic teardown `0x00622720` and modal pop `0x007757E0` deliberately pass a
  valid visible dialog to close helper `0x00608070`; CLOSE is an active standard
  YR path, not a Choose Map-only or dead path.
- SHOW uses SDBTNANM hold `10`, ramp `10,9,8,7,6,5`, settle `1`, and completion
  `0x4EC`. CLOSE uses hold `1`, ramp `5,6,7,8,9,10`, settle `10`, and completion
  `0x4ED` with no reveal.
- The native loop renders exactly `N_A + 9` frames. Main Menu has `N_A=5` and
  14 frames; Single Player has `N_A=3` and 12; Skirmish has `N_A=2` and 11.
- Every rendered loop frame is followed by `Sleep(30)`, including the terminal
  frame. Completion therefore becomes eligible 30 ms after the terminal paint,
  without drawing an extra frame. Ideal sleep budgets are 420 ms, 360 ms, and
  330 ms for Main Menu, Single Player, and Skirmish/Choose Map respectively.
- Exit/Back/Cancel are found by the native special-control predicate, share the
  final regular schedule value, and use the same Group-A frame family. They do
  not add a slot and do not use the `16..11` frame family.
- Cross-document evidence closes Choose Map at `N_A=2`, Cancel schedule value
  `2`, and 11 total frames. It does not yet prove whether Use Map or Create
  Random Map owns schedule value `1`.
- Opening Choose Map hides and preserves the existing `0x102` parent. Accept or
  Cancel closes `0x6B`, then uncovers that same parent without replaying `0x102`
  entry.
- Skirmish static controls `0x694`, `0x6EC`, and `0x5A8` begin blank and start
  kind-1 reveal only from entry completion. A timer invalidates; a successful
  paint draws the current count and increments afterward.
- Current Rust has stale `6/4/3` counts, enumeration-index scheduling, SHOW-only
  state, screen-edge triggering, pre-render advancement, a timer-driven wipe,
  no Path-A leading-edge gradient, static SDMPBTN, no SDWRNTMP, and no
  `ShellButtonSlideSound` parser.
- The currently invented `+80` shift on ordinary `0x102` SDTP is contradicted by
  the classifier evidence: the shift belongs to a separate `+0xDB` group, and
  `+0xDB` is false for all four in-scope dialogs.
- `[AudioVisual] GUIMoveInSound=MenuSlideIn` is parsed and active at SHOW start.
  `ShellButtonSlideSound=` is stock-empty but is a verified SHOW-completion
  configuration hook and is not parsed. `GUIMoveOutSound=MenuSlideOut` exists,
  but name similarity is not evidence that it is the close-start cue.
- The shared worktree is very dirty. No relevant committed restructuring landed
  after the design, but `app.rs`, every shell renderer, `ruleset.rs`, and several
  exact edit regions contain uncommitted work from other efforts.
- The repo has no gamemd-derived shell-frame corpus. Existing unit tests and Rust
  screenshots are regression checks only and cannot certify parity.

## Key Technical Decisions

- **Explicit lifetime events replace screen edges.** A screen flag is not an
  entry trigger; creation arms one lifetime and its matching first
  paint-equivalent attempt calls `begin_entry`. A real visible teardown calls
  `request_close`. **Confidence: high.**
  - **Source:** `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`,
    `gh-3.md`, approved design.
- **Control schedules use native control IDs and one-based entry ticks.** Layout
  iteration order cannot decide animation order. **Confidence: high for
  `0xE2/0x100/0x102`; medium for the two regular `0x6B` controls until Task 1.**
  - **Source:** `gh-19.md`, control identity modules, `0x6B` resource/resize
    reports.
- **The wave owns “current frame painted” state.** Tick zero is immediately
  paintable; the next tick becomes eligible no sooner than 30 ms after a
  successful presentation. A late frame advances only once. The terminal paint
  also observes its final 30 ms delay before completion, but produces no extra
  frame. **Confidence: high.**
  - **Source:** native blocking-loop cadence and current catch-up defect in
    `slide.rs`.
- **Pending route actions are owned semantic values, never closures.** The
  already-resolved Skirmish session is stored so close transaction/RNG work is
  never repeated after animation. **Confidence: high.**
  - **Source:** approved design and current `OfflineSkirmishRuntime` ordering.
- **The existing `DialogController` remains input/modal-stack authority.** The
  new field is named `shell_transition`; the two controllers are not merged.
  **Confidence: high.**
  - **Source:** `src/ui/shell/controller.rs` and current `AppState`.
- **Renderer construction is immutable; state advances after presentation.** A
  `ShellPaintReport` records which transition/reveal state was actually drawn,
  and the app applies receipts only after queue submit and `present()`.
  **Confidence: high.**
  - **Source:** static reveal evidence and current app render boundary.
- **Missing transition art is explicit and non-certifying.** A successfully
  presented fallback advances the same cadence so routes do not hang, but marks
  the frame `UnverifiedFallback`; a failed render does not advance or consume a
  pending action. **Confidence: high.**
  - **Source:** approved failure contract.
- **Path A gets its own wide-text/tint helper.** Sidebar Path B is not reused;
  `BitFont` only exposes the u16 glyph/measurement primitives Path A needs.
  **Confidence: high for separation; arithmetic is blocked on Task 1.**
  - **Source:** `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` and current shared font code.
- **`ShellButtonSlideSound` is parsed now; `GUIMoveOutSound` is gated.** Empty
  values become `None`, and completion sound is emitted before `0x4EC` reveal
  effects. **Confidence: high for completion hook, low for close cue until
  Task 1.**
  - **Source:** `gh-22.md`, stock `rules.ini`/`rulesmd.ini`.
- **Owner-draw click audio stays separate from slide audio.** The existing
  mouse-down `GUIMainButtonSound` and paint-transition `GenericClick` paths run
  before the parent command requests close; route handlers do not replay or
  relabel them as slide sounds. **Confidence: high.**
  - **Source:** owner-draw click reports and approved design.
- **Scoped `+80` SDTP movement is removed.** Descriptor support for a future
  verified radar-open group may exist, but none of the four target dialogs
  activates it. **Confidence: high.**
  - **Source:** `gh-21.md`, `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS...`,
    `SINGLE_PLAYER_TO_SKIRMISH...`.

Low-confidence decisions above are mandatory `/review-plan` targets in Task 2.

## Open Questions

### Resolved During Planning

- **Are close waves real on ordinary shell teardown?** Yes. Generic close and
  modal pop pass valid HWNDs to `0x00608070`.
- **Do Exit/Back/Cancel extend the wave?** No. One special control shares schedule
  value `N_A` and uses the Group-A frame family.
- **What are the known surface totals?** `0xE2=14`, `0x100=12`, `0x102=11`.
- **What is the Choose Map count/total?** `N_A=2`, special Cancel at value `2`,
  total `11` frames.
- **Does revealing `0x102` after Choose Map replay entry?** No; it is the same
  preserved parent lifetime.
- **Should ordinary `0x102` SDTP move right by 80 pixels during its wave?** No;
  the relevant optional group flag is false.

### Blocking Evidence Questions Owned by Task 1

- Which of `0x6C5` and `0x583` receives entry value `1` versus `2`?
- Which `0x6B`, `0xE2`, and `0x100` static children qualify as kind 1, and what
  interval/step/range/sound values do their classifiers assign?
- What exact integer operations, widths, signed divisions, clamps, packed-color
  conversions, and u16 indexing does BITFONT Path A use?
- Under corrected SHOW=`DL=1` / CLOSE=`DL=0`, what exact SDMPBTN and SDWRNTMP
  frames, anchors, rectangles, and draw order occur at every tick?
- What sound ID reaches the close-start `VocClass` call, and what exact branch
  fires on timeout/error?
- Which nonanimated controls, movie/background layer, hover state, and cursor are
  present in each transition phase?
- Do native `0x100` and `0x102` keyboard paths synthesize close on Escape, or
  should the current direct Rust close be removed?

No evidence-dependent Rust task may execute while any item above lacks a literal
answer in the Task 1 report and Task 2 implementation contract.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `docs/research/skirmish-ui/SHELL_DIALOG_LIFETIME_TRANSITION_EVIDENCE_GATES_GHIDRA_REPORT.md` | One corrected, cited active-binary report closing the seven blocking questions. |
| Create | `docs/research/skirmish-ui/SHELL_DIALOG_LIFETIME_TRANSITION_IMPLEMENTATION_CONTRACT.md` | Literal Rust-facing tables, algorithms, route gates, and acceptance vectors produced after Task 1. |
| Modify | `docs/plans/2026-07-18-shell-dialog-lifetime-transition-plan.md` | Task 2 inserts evidence-dependent literals and records green review. |
| Modify | `src/ui/shell/slide.rs` | Native dialog specs, control-ID schedules, direction-aware frames, exact presentation cadence. |
| Modify | `src/ui/shell/controller.rs` | Preserve/restore `0x102` beneath the independent `0x6B` modal lifetime. |
| Create | `src/app_shell_transition/controller.rs` | Pure `Idle/Entering/Closing` controller and presentation receipt state machine. |
| Create | `src/app_shell_transition/pending.rs` | Owned semantic route actions and Choose Map close payloads. |
| Modify | `src/app_shell_transition.rs` | Public app transition API, renderer/report glue, completion effect dispatch. |
| Modify | `src/app.rs` | Explicit lifetime calls, route deferral, input gate, render/presentation receipts, startup/return entry paths. |
| Modify | `src/app_main_menu_shell_render.rs` | Immutable transition-frame parameter and control-ID frame lookup. |
| Modify | `src/app_single_player_shell_render.rs` | Immutable transition-frame parameter and control-ID frame lookup. |
| Modify | `src/app_skirmish_shell_render.rs` | `0x102/0x6B` transition composition, receipt metadata, remove invented SDTP shift. |
| Modify | `src/app_skirmish_shell_render/modals.rs` | Choose Map button transition frames and modal-only composition receipts. |
| Modify | `src/app_skirmish_shell_render/chrome.rs` | Exact optional chrome frame lookup with no silent clamp. |
| Modify | `src/app_skirmish_shell_render/text.rs` | Blank-waiting reveal draw states and successful-paint control mask. |
| Modify | `src/app_skirmish_shell_render/draw_order.rs` | Task 1-proven optional chrome/text/control composition order. |
| Modify | `src/render/shell_paint.rs` | Exact requested SDBTNANM frame handling; explicit fallback instead of clamp-down. |
| Modify | `src/render/skirmish_shell_chrome.rs` | Load all verified SDMPBTN/SDWRNTMP frames and validate dimensions/counts. |
| Modify | `src/ui/skirmish_shell/static_reveal.rs` | Waiting/running/completed state, timer-invalidates and paint-advances behavior. |
| Modify | `src/ui/skirmish_shell/state/player_name.rs` | Keep the three `0x102` reveal fields; narrowly replace start/advance helpers with timer/presentation APIs. |
| Modify | `src/ui/skirmish_shell/state/choose_map.rs` | Store any Task 1-verified `0x6B` reveal state with the modal lifetime. |
| Modify if qualified by Task 1 | `src/ui/main_menu_shell/state.rs` | Store `0xE2` reveal state only if the active binary proves qualifying children. |
| Modify if qualified by Task 1 | `src/ui/single_player_shell/state.rs` | Store `0x100` reveal state only if the active binary proves qualifying children. |
| Create | `src/render/shell_text_reveal.rs` | BITFONT Path-A u16 reveal window and native packed-color tint arithmetic. |
| Modify | `src/render/shell_text.rs` | Route kind-1 text through the new Path-A helper. |
| Modify | `src/render/bit_font.rs` | Expose u16 glyph/width emission primitives without changing sidebar Path B. |
| Modify | `src/rules/ruleset.rs` | Parse `ShellButtonSlideSound`; parse close cue only if Task 1 proves the key binding. |
| Modify | `src/render/screenshot.rs` | Copy the already-composed swapchain texture for debug corpus capture. |
| Modify | `src/render/mod.rs` | Export capture and Path-A reveal modules. |
| Create | `src/render/shell_frame_compare.rs` | Shared manifest, packed-pixel comparison, and diff implementation used by the binary and integration test. |
| Create | `src/bin/shell-frame-compare.rs` | Exact dimension/pixel comparator and diff-image writer. |
| Modify | `Cargo.toml` | Register `shell-frame-compare` because `autobins=false`. |
| Create | `tests/shell_transition_pixel_oracle.rs` | Ignored, explicit-failure native corpus check. |
| Create | `docs/visual-checks/shell-transitions/README.md` | Corpus schema, provenance, capture commands, and verdict rules. |

## Interface Changes

- `ShellSlideSpec` changes from `{ dialog_id, slot_count }` to a literal
  control-ID schedule plus frame count, optional chrome flags, and reveal child
  IDs.
- `ShellFrameWave` gains both SHOW and CLOSE constructors and a presentation-
  gated API with a final post-terminal delay. Renderer-facing zero-based `slot`
  methods are removed.
- New `ShellSurface`, `ShellWaveDirection`, `ShellTransitionFrame`,
  `ShellTransitionFrameId`, `ShellTransitionController<A>`,
  `PendingShellAction`, `ShellCompletion<A>`, `ShellPaintFidelity`, and
  `ShellPaintReport` types are app/UI-only.
- `AppState.pending_shell_entry` arms a newly created lifetime until its matching
  first paint; armed state blocks input but emits no sound or wave yet.
- `AppState.shell_transition_redraw_deadline` lets `about_to_wait` preserve the
  native 30 ms sleep without duplicate swapchain presentation or a busy loop.
- Controller completion carries exact/fallback fidelity. Task 2 adds the
  Task 1-proven close-timeout outcome and ownership semantics before code begins.
- Main Menu, Single Player, and Skirmish renderer entry points gain an immutable
  `Option<ShellTransitionFrame>` argument and return presentation metadata.
- `GeneralRules` gains `shell_button_slide_sound: Option<String>`. A close-sound
  field is added only if Task 1 proves which INI key is consumed.
- `StaticReveal` changes from timer-advanced `Option<Reveal>` output to explicit
  `RevealDraw::{Hidden, Window, Full}` plus `poll_timer` and
  `record_presented` methods.
- `BitFont` gains u16-based glyph lookup/emission helpers. Existing `build_text`,
  sidebar text, and Path-B APIs retain their signatures.
- `render::screenshot` gains a composed-frame copy API; its current single-batch
  debug function can remain as a wrapper or be retired only after all callers
  are confirmed absent.

## Risk Areas

- **Dirty worktree overlap:** Every code task starts with `git diff -- <paths>`
  and re-reads current files. Do not normalize, overwrite, or revert unrelated
  work. Stop if the exact edit region changed since this plan was reviewed.
- **`app.rs` blast radius:** Startup, Mission Result return, loading, quickplay,
  quit, input, and render submission share this file. Route changes land in
  narrow steps with focused manual checks after each route family.
- **RNG/persistence ordering:** Start and Back already consume cooperative close
  transactions. The resolved owned result is captured before close and never
  recomputed.
- **Terminal off-by-one:** Native renders exactly `N_A+9` loop iterations. The
  current code appears capable of painting `tick == total_ticks`; new tests pin
  tick zero through `total_frames-1` only, then wait the terminal 30 ms without
  drawing another frame.
- **Creation versus first paint:** Arming a lifetime cannot play audio, advance a
  frame, or permit input. Only its matching paint-equivalent render starts entry.
- **Irreversible pre-close work:** Start/Back preflight Busy/source/spec/armed
  state before cooperative/RNG work; request errors return owned payloads.
- **Timeout drift:** The close timeout branch remains a hard gate and is encoded
  in both controller and app effects; it cannot degrade to log-and-clear.
- **Fallback action loss:** Current renderer fallback clears the wave. New logic
  distinguishes a presented nonparity fallback from a failed frame and retains
  pending actions in both cases until the cadence completes.
- **Hidden parent mutation:** `0x102` reveal timers may invalidate while `0x6B`
  is open, but parent reveal counts cannot advance without a parent paint receipt.
- **Shared text renderer:** Path-A work must not alter Path-B sidebar tinting,
  ordinary steady text, missing-glyph behavior, or wrapping without a named test.
- **Quit ordering:** Exit click opens `0x120`; only confirmed OK closes `0xE2`.
  Persistence precedes teardown, and the graceful cascade begins after `0xE2`
  close completion.
- **External close:** `WindowEvent::CloseRequested` is an abnormal OS route and
  remains a direct shutdown/persist path; it must not fabricate a shell close wave.
- **Unimplemented destinations:** Campaign, Movies, Load, Options, and Random Map
  Generator destination work stays outside this plan. Their placeholder routes
  cannot borrow another surface's lifetime spec.
- **Capture credibility:** OS screenshots with DPI scaling or compositor color
  conversion are not accepted as exact native fixtures. Task 1 must name the
  native surface capture boundary and pixel format.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---:|---|---|---|
| 1–2 | Blocking evidence literals | A guessed control order, tint operation, frame, cue, or timeout is direct drift | Cited live decompile/disassembly/runtime evidence plus green `/review-plan` |
| 3 | `N_A`, control entry values, frame totals | One extra control or loop tick is a visible 30 ms/frame disparity | Exhaustive table tests for all surfaces/directions |
| 3 | SHOW/CLOSE frame families | Reversed or wrong terminal frames change every transition | Per-control sequence tests against `gh-21` and Task 1 vectors |
| 3–4 | Presentation cadence | Catch-up, pre-render advance, or skipping the terminal sleep shortens native timing | 29/30 ms, ideal 420/360/330 ms budgets, long-stall, duplicate/stale-paint, failed-paint tests |
| 4, 9 | Close timeout/error | Dropping or prematurely executing an action on timeout changes route/state | Task 1 boundary vectors and exact controller/app outcome tests |
| 4–5 | Exact-once pending action | Double route/RNG/persistence corrupts state and may launch twice | Controller and route-event order tests |
| 9 | Creation versus first paint | Starting at creation plays audio/blocks timing before native `WM_PAINT` | Armed-without-paint and first-paint-once tests |
| 7–9 | Control-ID renderer mapping | Layout enumeration currently delays Exit/Back by one tick | Renderer unit tests keyed by resource ID |
| 9–12 | Source retained until close terminal | Early destination exposure differs for every route | Route lifecycle tests and frame captures |
| 11 | Start/Back transaction order | Re-running cooperative resolution consumes different RNG/state | Fake transaction counter plus existing launch tests |
| 11 | Confirmed Exit order | Quit fade cannot begin while `0xE2` close is still drawing | Ordered effect test and manual route trace |
| 12 | `0x6B` parent preservation | Recreating `0x102` loses selection/focus/reveal state and replays entry | Parent identity/state test and capture |
| 13 | Waiting/running/completed reveal states | Full initial text or timer-side increment changes every reveal frame | Pure state tests and presentation receipts |
| 13–14 | UTF-16 target/counting | Non-BMP/code-unit boundaries can shift cutoff and duration | Task 1 native vectors plus u16 fixtures |
| 14 | Path-A gradient/pixel packing | Range 8 is visibly tinted, not a hard wipe | Packed-color byte vectors and native pixel corpus |
| 15 | SDMPBTN/SDWRNTMP frames/order | Current `0x102` is static/missing; every entry/close is wrong | Asset checks, per-tick role tests, native frame corpus |
| 15 | Remove ordinary `+80` SDTP shift | Current Rust moves a group whose native flag is false | Draw-instance rect tests at 640/800/wide |
| 6, 9 | Sound order | Completion hook precedes `0x4EC`; close cue cannot be inferred | Pure ordered effect sink and configured-key parser tests |
| 10–12 | Click sound before close | Moving/dropping `GUIMainButtonSound` or `GenericClick` changes immediate feedback | Ordered input/paint/command/close event tests |
| 16 | Pixel oracle provenance | Rust-vs-Rust goldens cannot establish gamemd parity | gamemd hash, asset/rules hashes, exact surface dumps, comparator result |

---

## Tasks

### Task 1: Close the active-binary evidence gates

**Why:** Four implementation literals and three route/composition qualifiers are
not proved strongly enough to encode. This task removes that uncertainty before
any Rust mutation.

**Files:**

- Create:
  `docs/research/skirmish-ui/SHELL_DIALOG_LIFETIME_TRANSITION_EVIDENCE_GATES_GHIDRA_REPORT.md`
- Read and reconcile every source listed in **Sources & References**.

**Pattern:** Follow `/re-investigate`: two-pass claim enumeration, live binary
verification, inline evidence-call citations, verified facts separated from
inference, and a Rust implementation handoff. Do not edit Rust, INI, Ghidra
labels, or existing reports in this task.

**Step 1: Revalidate the evidence corpus and exact anchors**

Run the research index validation/map/handoff for `shell slide`, `0x6B`,
`BITFONT Path A`, and `static reveal`. Confirm the newer July 6 correction files
supersede the older direction/message and invalid-HWND claims. Record broken or
stale links in the new report.

Prefer the research-index MCP tools. If they are unavailable, run the exact CLI
fallbacks serially:

```powershell
python tools/research_index/brief.py --system shell "dialog lifetime SHOW CLOSE first paint" --anchor 0x006071E0 --anchor 0x00608070 --anchor 0x00608260
python tools/research_index/handoff.py --system shell "0x6B EnumChildWindows schedule optional chrome static reveal"
python tools/research_index/brief.py --system shell "BITFONT Path A kind-1 static reveal" --anchor 0x00434CD0 --anchor 0x00621040
python tools/research_index/map.py --system shell "dialog transition 0xE2 0x100 0x102 0x6B"
```

**Step 2: Pin Choose Map's two regular schedule values**

At `FUN_006071E0`, `FUN_0060A180`, `FUN_0060A250`, the `0x6B` creation path
`0x005E68A0`, and the raw RT_DIALOG template:

1. prove the actual `EnumChildWindows` order/Z-order seen by the Group-A callback;
2. record whether `0x6C5` or `0x583` owns schedule value `1`;
3. confirm the other owns value `2`;
4. reconfirm `0x5C0` uses the special block at value `N_A=2` with Group-A frame
   constants; and
5. walk the schedule max to 11 rendered iterations.

The report must print one literal `0x6B` table: control ID, predicate/class,
entry value, pre/ramp/post frames for SHOW and CLOSE, and total frames.

**Step 3: Pin kind-1 reveal membership/defaults**

Decompile/disassemble the `FUN_00602490` classifier and the value helpers
`0x00600CA0`, `0x006015E0`, and `0x00601D20` for parents `0xE2`, `0x100`,
`0x102`, and `0x6B`. Record every qualifying child ID and the literal initial
running byte, count, interval, step, range, and sound value. Confirm whether
`0x6B` title `0x694` is the only qualifying child and whether `0xE2/0x100`
have none. Trace `SetTimer`/invalidation through the kind-1 `WM_PAINT` handler
and state whether a redundant or externally caused paint advances count, which
record flag gates that advancement, and exactly when the flag is cleared.

**Step 4: Decode BITFONT Path A exactly**

For `FUN_00434CD0` and wrapper `FUN_00621040`:

1. map the u16/wide-text loop, line wrapping, clipping, whitespace/control
   handling, missing-glyph fallback, and reveal index advancement;
2. transcribe the leading-edge tint calculation instruction by instruction,
   including operand widths, signedness, division rounding, clamps, sentinel
   branches, selected-unit highlight initialization, and range-zero behavior;
3. bind the runtime display loss/shift globals to the active stock pixel format;
4. derive machine-checkable vectors for ASCII, spaces, CR/LF, a missing glyph,
   `count=1`, `count=len`, `range=8`, completed `count=len+8`, and at least one
   valid surrogate pair; and
5. explicitly state which Path-B operations are absent from Path A.

**Step 5: Re-audit optional chrome under corrected direction semantics**

At the optional SDMPBTN/SDWRNTMP blocks of `FUN_006071E0`, with SHOW=`DL=1`
and CLOSE=`DL=0`, record for every delta:

- asset/frame index;
- schedule anchor;
- held-before and held-after behavior;
- destination rectangle and any client conversion;
- conditional base-frame-1 underdraw;
- order relative to background, SDTP, SDBTNANM controls, text, movie, hover, and
  cursor; and
- the exact scope of the `+0x50` branch.

Reconfirm dialog flags: `0xE2=0000`, `0x100=0000`, `0x102` enables D9/DA only,
and `0x6B` enables D9 only.

**Step 6: Bind close sound and timeout/error behavior**

Trace the close helper's `VocClass` argument back to its rules field or constant.
Do not infer from `GUIMoveOutSound` spelling. Record the exact global/record gates,
visibility/enabled behavior, message-pump termination, tick/time source, 5000 ms
comparison semantics, timeout result, and whether any completion/pending action
still occurs on the error branch.

**Step 7: Close composition and Escape qualifiers**

Use static evidence plus a native surface/runtime capture where static proof is
insufficient to record:

- whether movie/background, ordinary owner-draw controls, labels, hover/pressed
  states, and software cursor are drawn during each phase; and
- whether native `0x100`/`0x102` Escape produces a real close command.

Name the exact native framebuffer/surface capture boundary, dimensions, pitch,
pixel format, DPI assumptions, and artifact extraction procedure. A desktop
screenshot with unknown scaling is insufficient.

**Step 8: Validate the report**

Rebuild the index after creating the report, then run validation. Prefer
`research_reindex()` and `research_validate(system="shell", topic="dialog
lifetime transition")`; CLI fallback:

```powershell
python tools/research_index/index.py
python tools/research_index/validate.py --system shell "dialog lifetime transition"
```

The report is complete only when all seven blocking questions have literal
answers, every binary fact cites the actual tool call/address range, stale
contradictions are named, and the implementation handoff contains test vectors
rather than prose-only formulas.

**Verification:** No Cargo command. `research_validate` must report valid, and
the report must contain no `UNCHECKED` item that feeds Tasks 3–9 or 13–16.

### Task 2: Produce the implementation contract and re-review this plan

**Why:** The binary report must be converted into a dumb-executor-safe contract
before code work begins.

**Files:**

- Create:
  `docs/research/skirmish-ui/SHELL_DIALOG_LIFETIME_TRANSITION_IMPLEMENTATION_CONTRACT.md`
- Modify:
  `docs/plans/2026-07-18-shell-dialog-lifetime-transition-plan.md`

**Pattern:** Use `/implementation-contract` on the Task 1 report and current Rust,
then `/review-plan` on this plan. This is the only task authorized to revise the
evidence-dependent literals while preserving the approved architecture.

**Step 1: Write the contract**

The contract must contain:

- literal schedules for all four dialogs;
- exact frame count and terminal boundary;
- exact Path-A pseudocode plus input/output byte vectors;
- exact SDMPBTN/SDWRNTMP per-tick tables and draw order;
- close cue field/ID and timeout/error transition;
- reveal child/default table;
- exact timer/invalidation/redundant-paint state transitions;
- Escape route results; and
- a delta table mapping each fact to current Rust path/symbol.

**Step 2: Literalize the blocked plan sections**

Insert the Task 1 values directly into:

- Task 3's `CHOOSE_MAP_CONTROLS` schedule and tests;
- Task 3's per-surface reveal lists and optional-group flags;
- Tasks 5, 7, 8, and 13's per-surface reveal-receipt/state/renderer wiring for
  every verified child, including `0x6B` and any Main/SP qualifiers;
- Tasks 4 and 9's timeout clock, ownership, error outcome, and app effect;
- Task 6's close-sound parser/hook decision;
- Task 8's `0x6B` renderer schedule;
- Tasks 7–9 and 15–16's exact movie/background/control/text/hover/cursor
  composition and deterministic capture-state requirements;
- Task 13's per-surface reveal table;
- Task 14's `path_a_color` implementation and exact vector assertions; and
- Task 15's optional-chrome frame functions/draw-order assertions.

Do not leave references such as “use the report's value” in those code tasks.
The revised task must show the actual constants and code.

**Step 3: Run plan review**

Run `/review-plan docs/plans/2026-07-18-shell-dialog-lifetime-transition-plan.md`.
Resolve every finding. Record the review date, verdict, Task 1 report path, and
contract path immediately below this task.

**Hard pass condition:** Review verdict GREEN, no evidence-dependent question
remains open, and the plan contains literal code/test vectors for all four
surfaces. Otherwise stop before Task 3.

### Task 3: Replace count-only slide data with exact control schedules and presentation cadence

**Why:** Every renderer and lifecycle task depends on a correct, pure frame
contract. This task fixes known counts, off-by-one behavior, CLOSE direction, and
late-frame catch-up before app wiring.

**Files:**

- Modify: `src/ui/shell/slide.rs`

**Pattern:** Existing render-agnostic `slide.rs`; new control-ID table pattern.
It continues to depend only on `DialogId` and `std`.

**Step 1: Define native surface/spec types**

Use this interface (Task 2 adds the literal `0x6B` array before execution):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellSurface {
    MainMenu0xE2,
    SinglePlayer0x100,
    Skirmish0x102,
    ChooseMap0x6B,
}

impl ShellSurface {
    pub(crate) const fn dialog_id(self) -> DialogId {
        DialogId(match self {
            Self::MainMenu0xE2 => 0x00E2,
            Self::SinglePlayer0x100 => 0x0100,
            Self::Skirmish0x102 => 0x0102,
            Self::ChooseMap0x6B => 0x006B,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScheduledControlKind {
    RegularA,
    SpecialUsingAFrames,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControlWaveSchedule {
    pub control_id: u16,
    pub entry_tick: u32,
    pub kind: ScheduledControlKind,
}

#[derive(Debug)]
pub(crate) struct ShellSlideSpec {
    pub surface: ShellSurface,
    pub regular_count: u32,
    pub total_frames: u32,
    pub controls: &'static [ControlWaveSchedule],
    pub reveal_controls: &'static [u16],
    pub optional_groups: ShellOptionalGroups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellOptionalGroups {
    pub d9: bool,
    pub da: bool,
    pub db: bool,
    pub dc: bool,
}
```

Do not wire the `16..11` family to Exit/Back/Cancel. Keep the current
renderer-facing `ButtonGroup`, `slot_count_for`, `new_first_paint_slide`,
`ShellFrameWave: Clone`, `advance`, `is_complete`, and
`sdbtnanm_frame(slot, ButtonGroup)` only as clearly marked compatibility shims so
Task 3 still compiles with unmigrated callers. Add a read-only
`legacy_current_transition_frame()` adapter for Tasks 7–8 wrappers. Implement
every shim by delegating to the new control schedule/wave state; no new
production code may call them. Task 9 removes the entire compatibility block
after every caller has migrated.

**Step 2: Add literal known schedules**

```rust
const MAIN_MENU_CONTROLS: &[ControlWaveSchedule] = &[
    ControlWaveSchedule { control_id: 0x0683, entry_tick: 1, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x0684, entry_tick: 2, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x0578, entry_tick: 3, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x0686, entry_tick: 4, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x055C, entry_tick: 5, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x03EE, entry_tick: 5, kind: ScheduledControlKind::SpecialUsingAFrames },
];

const SINGLE_PLAYER_CONTROLS: &[ControlWaveSchedule] = &[
    ControlWaveSchedule { control_id: 0x0688, entry_tick: 1, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x0689, entry_tick: 2, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x0579, entry_tick: 3, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x0686, entry_tick: 3, kind: ScheduledControlKind::SpecialUsingAFrames },
];

const SKIRMISH_CONTROLS: &[ControlWaveSchedule] = &[
    ControlWaveSchedule { control_id: 0x0617, entry_tick: 1, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x05AA, entry_tick: 2, kind: ScheduledControlKind::RegularA },
    ControlWaveSchedule { control_id: 0x05C0, entry_tick: 2, kind: ScheduledControlKind::SpecialUsingAFrames },
];
```

Add `CHOOSE_MAP_CONTROLS` exactly as literalized by Task 2. Define four specs
with totals `14`, `12`, `11`, and `11`; assert at test time that
`total_frames == regular_count + 9` and that every entry tick is in
`1..=regular_count`. Task 2 inserts each literal `reveal_controls` list. Set
optional groups to `0000`, `0000`, `1100`, and `1000` for `0xE2`, `0x100`,
`0x102`, and `0x6B`; assert `db == false` for every in-scope surface.

**Step 3: Define immutable renderer frames**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellWaveDirection {
    Enter,
    Close,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellTransitionFrame {
    pub surface: ShellSurface,
    pub direction: ShellWaveDirection,
    pub tick: u32,
    spec: &'static ShellSlideSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellTransitionFrameId {
    pub surface: ShellSurface,
    pub direction: ShellWaveDirection,
    pub tick: u32,
}

impl ShellTransitionFrame {
    pub(crate) const fn id(self) -> ShellTransitionFrameId {
        ShellTransitionFrameId {
            surface: self.surface,
            direction: self.direction,
            tick: self.tick,
        }
    }

    pub(crate) fn sdbtnanm_frame(self, control_id: u16) -> Option<usize> {
        let entry = self.spec.controls.iter().find(|entry| entry.control_id == control_id)?;
        let delta = self.tick as i32 - entry.entry_tick as i32;
        let (before, base, step, after) = match self.direction {
            ShellWaveDirection::Enter => (10, 10, -1, 1),
            ShellWaveDirection::Close => (1, 5, 1, 10),
        };
        let frame = if delta < 0 {
            before
        } else if delta < 6 {
            base + delta * step
        } else {
            after
        };
        Some(frame as usize)
    }
}
```

The special kind remains in the spec for native identity and future draw-order
checks, but uses the same arithmetic in this scope.

**Step 4: Make wave advancement and completion presentation-gated**

`ShellFrameWave` stores the spec, direction, current tick, whether the current
tick has been presented, and the earliest next tick time. Required behavior:

```rust
pub(crate) enum WavePresentation {
    RepeatedFrame,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WavePresentationError {
    MismatchedFrame,
}

pub(crate) enum ShellWavePoll {
    Frame(ShellTransitionFrame),
    WaitingUntil(Instant),
    CompletionDue,
}

impl ShellFrameWave {
    pub(crate) fn new(
        spec: &'static ShellSlideSpec,
        direction: ShellWaveDirection,
        now: Instant,
    ) -> Self;

    pub(crate) fn poll(&mut self, now: Instant) -> ShellWavePoll;

    pub(crate) fn record_presented(
        &mut self,
        frame: ShellTransitionFrameId,
        now: Instant,
    ) -> Result<WavePresentation, WavePresentationError>;
}
```

`poll(now)` advances from tick `t` to `t+1` only when tick `t` was presented,
`now >= next_tick_at`, and `t+1 < total_frames`. It advances at most once and
marks the new tick unpresented. Before that deadline it returns
`WaitingUntil(next_tick_at)` rather than asking the renderer to present the same
frame again; the already-presented surface remains visible during native sleep.
`record_presented(frame, now)` accepts only the exact current
surface/direction/tick, accepts that tick only once, and sets
`next_tick_at = now + 30 ms`.

After presenting `total_frames-1`, `poll(now)` continues returning that terminal
sleep as `WaitingUntil`. At the deadline it returns `CompletionDue` without
creating or rendering `tick == total_frames`. A stale or mismatched receipt is
an error and cannot advance the clock.

**Step 5: Add exhaustive tests**

Tests must assert:

- exact control tables and totals for all four surfaces;
- Exit/Back/Cancel share the last regular tick and use Group-A frames;
- SHOW sequence `10,10..5,1` and CLOSE `1,5..10,10` for every entry value;
- tick zero and terminal tick are included exactly once;
- 29 ms does not advance; 30 ms does;
- a one-second stall advances one tick only;
- a second `poll()` call at the same instant cannot advance an unpresented tick;
- an early poll returns `WaitingUntil` and causes no duplicate presentation;
- repeated presentation of the same tick is a no-op; and
- a stale or wrong-surface/direction/tick receipt is rejected;
- terminal presentation does not complete at 29 ms, completes at 30 ms, and
  never creates an extra frame;
- ideal uninterrupted completion is 420 ms for Main, 360 ms for SP, and 330 ms
  for `0x102`/`0x6B`; and
- all frames are within the retail SDBTNANM `0..=16` range.

**Verify:**

```powershell
cargo test -p vera20k ui::shell::slide::tests -- --nocapture
```

Expected literal result line: `test result: ok` with zero failed tests.

### Task 4: Add the pure lifetime transition controller

**Why:** Navigation must retain source state through close, reject overlap, and
emit pending actions exactly once independently of GPU/AppState complexity.

**Files:**

- Create: `src/app_shell_transition/controller.rs`
- Modify: `src/app_shell_transition.rs`

**Pattern:** New app-layer generic controller; it consumes pure wave data from
`ui::shell::slide` and owns no renderer, audio player, window, or simulation.

**Step 1: Define phases and completion values**

```rust
#[derive(Debug)]
pub(crate) enum ShellTransitionPhase<A> {
    Idle,
    Entering {
        surface: ShellSurface,
        wave: ShellFrameWave,
    },
    Closing {
        surface: ShellSurface,
        wave: ShellFrameWave,
        pending_action: A,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellPaintFidelity {
    ExactPath,
    UnverifiedFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellTransitionFidelity {
    ExactPath,
    UnverifiedFallbackSeen,
}

#[derive(Debug)]
pub(crate) enum ShellCompletion<A> {
    Entry0x4Ec {
        surface: ShellSurface,
        fidelity: ShellTransitionFidelity,
    },
    Close0x4Ed {
        surface: ShellSurface,
        action: A,
        fidelity: ShellTransitionFidelity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ShellTransitionRequestError {
    #[error("a shell transition is already active")]
    Busy,
    #[error("a newly created shell lifetime is still waiting for first paint")]
    EntryArmed,
    #[error("requested close source {requested:?} does not match {active:?}")]
    SourceMismatch {
        requested: ShellSurface,
        active: Option<ShellSurface>,
    },
    #[error("no verified slide specification exists for {0:?}")]
    MissingSpec(ShellSurface),
}

#[derive(Debug)]
pub(crate) enum ShellTransitionPoll<A> {
    Idle,
    Frame(ShellTransitionFrame),
    WaitingUntil(Instant),
    Complete(ShellCompletion<A>),
}
```

**Step 2: Implement the controller API**

```rust
pub(crate) struct ShellTransitionController<A> {
    phase: ShellTransitionPhase<A>,
    unverified_fallback_seen: bool,
}

impl<A> ShellTransitionController<A> {
    pub(crate) fn new() -> Self;
    pub(crate) fn is_active(&self) -> bool;
    pub(crate) fn blocks_input(&self) -> bool;
    pub(crate) fn surface(&self) -> Option<ShellSurface>;
    pub(crate) fn fidelity(&self) -> ShellTransitionFidelity;
    pub(crate) fn begin_entry(
        &mut self,
        surface: ShellSurface,
        now: Instant,
    ) -> Result<(), ShellTransitionRequestError>;
    pub(crate) fn request_close(
        &mut self,
        surface: ShellSurface,
        action: A,
        now: Instant,
    ) -> Result<(), ShellTransitionRequestError>;
    pub(crate) fn poll(&mut self, now: Instant) -> ShellTransitionPoll<A>;
    pub(crate) fn record_presented(
        &mut self,
        frame: ShellTransitionFrameId,
        now: Instant,
        fidelity: ShellPaintFidelity,
    ) -> Result<WavePresentation, WavePresentationError>;
}
```

`begin_entry` and `request_close` accept only `Idle`; Task 9 validates the
requested source against the active dialog before calling the pure controller.
`request_close` moves the requested source and owned action into `Closing`.
`record_presented` validates the exact frame ID and updates
`unverified_fallback_seen`, but advances both fidelity types at the same native
cadence. It never completes immediately.

Every accepted `begin_entry`/`request_close` resets fidelity to `ExactPath`.
The first presented fallback changes it permanently to
`UnverifiedFallbackSeen` for that lifetime transition. The getter exposes the
active status, and the terminal `ShellCompletion` carries the final status so a
subsequent entry cannot erase the result before reporting.

`poll` delegates to the wave. Only `CompletionDue`, 30 ms after the terminal
presentation, moves the phase to `Idle` and returns `Entry0x4Ec` or moves the
owned action into `Close0x4Ed`. While a frame's sleep is pending, it returns
`WaitingUntil` and retains the already-presented source surface without another
render/present.

Task 2 must insert the Task 1-proven close-timeout fields and literal poll branch
into this controller before Task 4 executes: exact clock source, start point,
comparison boundary, phase after timeout, fidelity reporting, and ownership or
execution of `pending_action`. The corresponding `ShellTransitionPoll` timeout
variant must carry every value the app needs; do not reduce it to a log-and-idle
shortcut. If the native branch leaves the close pending, the Rust branch must do
the same.

A renderer failure calls neither `record_presented` nor another completion API,
so the frame and action remain intact for retry.

**Step 3: Add controller tests with a non-Clone action**

Use a test action containing `Box<u32>` to prove the controller does not clone.
Cover:

- `Idle -> Entering -> Idle` and input blocking;
- `Idle -> Closing -> Idle` with source retained;
- exact-once action move after the terminal frame;
- repeated/overlapping requests return `Busy` without replacing the action;
- failed/skipped presentation leaves tick/action unchanged;
- stale or mismatched receipts return `MismatchedFrame` and leave state intact;
- pre-deadline polls return `WaitingUntil` and produce no duplicate receipt;
- an explicit fallback advances but sets the unverified flag;
- accepted starts reset fidelity and completions retain the final fidelity;
- entry yields only `Entry0x4Ec`;
- close yields only `Close0x4Ed`; and
- 11-frame `0x6B` close still owns its action after the 11th accepted receipt and
  returns it only when the final 30 ms delay expires; and
- the Task 1 timeout boundary at one unit before/equal/one unit after, including
  exact pending-action ownership and source-visibility behavior.

**Verify:**

```powershell
cargo test -p vera20k app_shell_transition::controller::tests -- --nocapture
```

Expected: `test result: ok`, zero failed.

### Task 5: Define owned pending actions and ordered completion effects

**Why:** Route semantics and payload ownership must be explicit before AppState
handlers start deferring work.

**Files:**

- Create: `src/app_shell_transition/pending.rs`
- Modify: `src/app_shell_transition.rs`

**Pattern:** App-specific semantic enum above the generic controller. No closure,
trait object, borrowed state, GPU handle, or window handle is stored.

**Step 1: Define action payloads**

```rust
use crate::skirmish_launch::SkirmishLaunchSession;
use crate::ui::skirmish_shell::ChooseMapSelection;

#[derive(Debug)]
pub(crate) enum ChooseMapCloseAction {
    Accept(ChooseMapSelection),
    Cancel,
}

#[derive(Debug)]
pub(crate) enum PendingShellAction {
    OpenSinglePlayer,
    OpenMainMenu,
    OpenSkirmish,
    ReturnToSinglePlayer,
    CloseSkirmishToMainMenu,
    BeginSkirmishLoading(SkirmishLaunchSession),
    FinishChooseMap(ChooseMapCloseAction),
    BeginQuitCascade,
}

#[derive(Debug)]
pub(crate) struct ShellCloseRequestFailure {
    pub error: ShellTransitionRequestError,
    pub action: PendingShellAction,
}
```

`BeginSkirmishLoading` owns the result of the one pre-close
`close_shell_transaction` call. `FinishChooseMap::Accept` owns the modal's
noncommitted selection value. `ShellCloseRequestFailure` prevents an unexpected
Busy/spec/source failure from silently dropping an already-owned payload.

**Step 2: Define renderer/presentation reports**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RevealPaintMask(u64);

impl RevealPaintMask {
    pub(crate) const fn for_spec_index(index: usize) -> Self {
        assert!(index < 64);
        Self(1_u64 << index)
    }
    pub(crate) const fn union(self, other: Self) -> Self { Self(self.0 | other.0) }
    pub(crate) const fn contains(self, other: Self) -> bool { self.0 & other.0 != 0 }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellTransitionPaintReceipt {
    pub frame: ShellTransitionFrameId,
    pub fidelity: ShellPaintFidelity,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellPaintReport {
    pub surface: ShellSurface,
    pub transition: Option<ShellTransitionPaintReceipt>,
    pub reveal_paints: RevealPaintMask,
}
```

The report describes content encoded into the composed frame. It does not mutate
state. The app applies it only after presentation in Task 9. Reveal bits are
indices into that surface's verified `ShellSlideSpec.reveal_controls` list, so
Task 1 can add non-Skirmish qualifiers without changing the receipt shape. Spec
validation rejects more than 64 reveal controls.

**Step 3: Pin ordered entry/close effects**

Add a pure helper that maps `ShellCompletion` to an ordered small fixed array or
test-only event sink. Required order:

- entry: `PlayShellButtonSlideSound`, then `Broadcast0x4Ec`, then start only the
  spec's reveal children;
- close: `Broadcast0x4Ed`, then execute the pending semantic action; no completion
  sound and no reveal.

Use an enum-backed test sink; do not test ordering through Rodio.

**Verify:**

```powershell
cargo test -p vera20k app_shell_transition::pending::tests -- --nocapture
```

### Task 6: Parse and expose verified transition sound hooks

**Why:** The stock-empty completion hook is still a real INI-driven mechanism.
The close cue is added only if Task 1 binds it.

**Files:**

- Modify: `src/rules/ruleset.rs`
- Modify: `src/app.rs`

**Pattern:** Existing `gui_move_in_sound` parser and
`App::play_shell_ui_sound_by_id` helper. Do not edit `audio/events.rs` or
`audio/sfx.rs`.

**Step 1: Add `shell_button_slide_sound`**

Add to `GeneralRules`, `Default`, and `from_ini`:

```rust
/// SHOW-completion cue from [AudioVisual] ShellButtonSlideSound.
/// Stock data is empty; a trimmed empty value is represented as None.
pub shell_button_slide_sound: Option<String>,
```

```rust
shell_button_slide_sound: audio_visual
    .and_then(|section| section.get("ShellButtonSlideSound"))
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(str::to_string),
```

**Step 2: Apply Task 1's close-cue conclusion**

Task 2 must have inserted one literal branch here:

- if Task 1 proves an INI key, add the correspondingly named `Option<String>`
  field/parser with the same trim/empty semantics and expose it through a close-
  start playback helper for Task 9;
- if Task 1 proves a different constant/field or no cue, encode that exact result
  and leave `GUIMoveOutSound` unparsed/unwired.

No branch based only on the key name is permitted.

**Step 3: Add app playback helpers**

Add `play_shell_slide_completion_sound` beside
`play_shell_slide_in_sound`. It clones only the short optional ID to avoid
aliasing `state.rules` while borrowing the player. Call it from entry completion
before reveal effects, not from the renderer. If Task 1 proves a close cue, add
the correspondingly named helper here; Task 9 calls it only after a close request
is accepted and before close tick zero.

**Step 4: Add parser/order tests**

Tests cover nonempty custom value, whitespace trimming, empty stock value to
`None`, independence from `GUIMoveInSound`, and the Task 1 close-cue result.

**Verify:**

```powershell
cargo test -p vera20k rules::ruleset::tests::shell_transition_sound -- --nocapture
```

### Task 7: Convert Main Menu and Single Player renderers to control-ID frames

**Why:** These two renderers currently enumerate six/four buttons and make
Exit/Back one tick late.

**Files:**

- Modify: `src/app_main_menu_shell_render.rs`
- Modify: `src/app_single_player_shell_render.rs`
- Modify: `src/render/shell_paint.rs`

**Pattern:** Existing owner-draw paint lists; frame overrides become an explicit
input instead of reading `AppState.shell_first_paint_slide`.

**Step 1: Add migration-safe renderer entry points**

Add an internal `*_with_transition` entry point for each renderer that accepts:

```rust
transition: Option<ShellTransitionFrame>
```

and returns the existing steady/fallback result plus a `ShellPaintReport` when
the matching surface was composed. Until Task 9 migrates `app.rs`, retain the
old entry point as a compatibility wrapper that adapts the legacy wave through
the Task 3 shim and calls the new function. The new function must not read global
transition state. Task 9 switches the caller, deletes the wrapper, and removes
the last global wave reads.

**Step 2: Resolve frames by resource ID**

Main Menu:

```rust
let wave_frame = transition
    .filter(|frame| frame.surface == ShellSurface::MainMenu0xE2)
    .and_then(|frame| frame.sdbtnanm_frame(button.id.resource_id()));
```

Single Player uses the same pattern with `SinglePlayer0x100`. Unknown controls
receive no override; a scheduled control that lacks a decoded requested frame is
an explicit transition fallback, not a preceding-frame substitute.

**Step 3: Remove silent frame clamping**

In `shell_paint.rs`, replace the “walk down to an available frame” behavior with
an exact lookup result. The caller returns a named missing-frame fallback reason
containing surface, control ID, and requested frame. Steady-state frame selection
is unchanged.

**Step 4: Add pure mapping tests**

Assert every Main/SP resource ID maps to the Task 3 tick, Exit shares Options at
tick 5, Back shares Skirmish at tick 3, SHOW/CLOSE frame values are exact, and
button enumeration order changes do not alter results.

Task 2 inserts any Task 1-proven transition-phase deltas for Main/SP
movie/background, steady controls, text, hover/pressed state, and cursor into
Steps 1–4 as literal composition code and fixture assertions. “Keep current
composition” is permitted only when Task 1 positively proves it.

**Verify serially:**

```powershell
cargo test -p vera20k app_main_menu_shell_render -- --nocapture
cargo test -p vera20k app_single_player_shell_render -- --nocapture
```

### Task 8: Convert Skirmish and Choose Map renderers to surface-specific frames

**Why:** `0x102` Back is late, `0x6B` has no independent wave, and modal-only
composition must identify the correct lifetime.

**Files:**

- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/app_skirmish_shell_render/modals.rs`
- Modify: `src/app_skirmish_shell_render/chrome.rs`

**Pattern:** Existing modal early-return remains the parent-suppression boundary.
Only its transition input and report identity change.

**Step 1: Pass immutable frames into the shared builder**

Add a migration-safe builder entry point taking `Option<ShellTransitionFrame>`;
keep the current `Option<&ShellFrameWave>` wrapper only until Task 9 switches
`app.rs`, then delete it. For ordinary setup, resolve Start `0x617`, Choose Map
`0x5AA`, and Back `0x5C0` by control ID. Back must share entry value 2 and the
same Group-A frame family.

**Step 2: Apply the Task 2 literal `0x6B` schedule**

The modal builder resolves Use Map `0x6C5`, Create Random Map `0x583`, and Cancel
`0x5C0` using the exact Task 1 order inserted into this task. Cancel uses entry
value 2 and Group-A frames. The modal early-return emits a `ChooseMap0x6B`
paint report and no parent reveal bits.

**Step 3: Remove the invented ordinary SDTP shift**

Delete the condition that offsets `right_panel_top_sdtp` by 80 whenever a wave
is active at width >=800. Add rect tests proving no shift at 640, 800, and 1024
for both directions on all four target surfaces.

Task 15 adds any separate optional group that Task 1 proves; it must not reapply
the shift to ordinary SDTP.

**Step 4: Remove exact-frame clamping**

Make `push_right_panel_button_wave` return a missing-frame error/fallback reason
when the exact atlas frame is absent. Do not decrement the requested index.

**Step 5: Add mapping/composition tests**

Cover exact `0x102` and `0x6B` control mappings, modal-only parent suppression,
no parent reveal paint mask, no ordinary +80 shift, and explicit missing-frame
fallback.

Task 2 inserts every Task 1-proven transition-phase delta for ordinary controls,
preview/flags, text, hover/pressed state, movie/background, and cursor as literal
builder ordering plus deterministic fixture assertions. Task 16's manifest uses
the same state axes.

**Verify:**

```powershell
cargo test -p vera20k app_skirmish_shell_render -- --nocapture
```

### Task 9: Replace screen-edge inference with explicit AppState lifetime and presentation wiring

**Why:** The controller cannot become authoritative while old screen-edge fields
can independently start, cancel, or advance a wave.

**Files:**

- Modify: `src/app.rs`
- Modify: `src/app_shell_transition.rs`

**Pattern:** App orchestration only. Renderers remain state-immutable with respect
to transition/reveal clocks.

**Step 1: Replace old fields**

Remove `shell_first_paint_slide` and `shell_slide_active_shell`. Add:

```rust
pub(crate) shell_transition:
    crate::app_shell_transition::ShellTransitionController<
        crate::app_shell_transition::PendingShellAction,
    >,
```

Initialize it with `new()`. Keep `shell_controller: DialogController` unchanged.
Add `pending_shell_entry: Option<ShellSurface>` beside the controller. It marks a
newly created native-equivalent lifetime whose first paint has not yet begun;
it is not a fourth transition phase.
Add `shell_transition_redraw_deadline: Option<Instant>` for the event-loop wait;
the lifecycle poll sets/clears it and no renderer owns it.

**Step 2: Remove edge-trigger APIs**

Delete `current_shell_slide_target`, `update_shell_first_paint_slide_trigger`,
`start_shell_first_paint_slide`, and `render_shell_first_paint_slide`. Remove the
Task 3 and Tasks 7–8 compatibility shims now that this task migrates every caller.
Add narrow helpers:

```rust
pub(crate) fn arm_shell_entry(
    state: &mut AppState,
    surface: ShellSurface,
) -> Result<(), ShellTransitionRequestError>;
pub(crate) fn begin_armed_shell_entry_on_first_paint(
    state: &mut AppState,
    surface: ShellSurface,
    now: Instant,
) -> Result<(), ShellTransitionRequestError>;
pub(crate) fn can_request_shell_close(
    state: &AppState,
    surface: ShellSurface,
) -> Result<(), ShellTransitionRequestError>;
pub(crate) fn request_shell_close(
    state: &mut AppState,
    surface: ShellSurface,
    action: PendingShellAction,
) -> Result<(), ShellCloseRequestFailure>;

pub(crate) enum ShellRenderGate {
    Render(Option<ShellTransitionFrame>),
    WaitUntil(Instant),
}

pub(crate) fn poll_shell_transition_lifecycle(
    state: &mut AppState,
    now: Instant,
) -> ShellRenderGate;
pub(crate) fn commit_presented_shell_frame(
    state: &mut AppState,
    report: ShellPaintReport,
    now: Instant,
);
```

Lifetime creation calls only `arm_shell_entry`; it does not start a clock or
sound. After swapchain acquisition, the matching surface's first
paint-equivalent render attempt calls `begin_armed_shell_entry_on_first_paint`,
which consumes the marker, begins tick zero, and starts `GUIMoveInSound` exactly
once. If no paint attempt occurs, entry remains armed and silent. Uncovering a
preserved parent never arms it. Both helpers reject overlap/mismatched markers
instead of overwriting them; route tests treat such a result as an invariant
failure and never expose an unanimated destination silently.

`can_request_shell_close` verifies Idle state, a known spec, no armed entry, and
that `surface` is the current in-scope close source (accounting for a verified
non-transition confirmation modal that the same command will dismiss).
`request_shell_close` repeats those checks, returns the owned action inside
`ShellCloseRequestFailure` on error, and never silently discards it.
After the controller accepts the close, start only the Task 1-proven close cue
and timeout clock, in their contract order, before close tick zero is rendered.

**Step 3: Poll completion, select the resulting route, then start first paint**

Before acquiring a swapchain texture, poll the active controller once. For
`WaitingUntil`, store the deadline for `about_to_wait` and return from
`render_frame` without acquiring, encoding, or presenting a duplicate frame. If
the terminal 30 ms deadline is due, dispatch the completion effects/action so
render dispatch observes the resulting source/destination state. A destination
created by that action is only armed.

Acquire the surface, then select the renderer for the now-current surface. If
that surface matches the armed marker, consume it with
`begin_armed_shell_entry_on_first_paint` and poll once to obtain entry tick zero.
Otherwise use the active frame returned by the first poll. Pass exactly that
immutable frame only to its matching renderer and collect one `ShellPaintReport`
from the composed shell/fallback path. Permit at most one completion/timeout
dispatch per redraw; the second poll may return a newly armed entry frame but may
not execute another terminal event.

Update `about_to_wait`: while `ShellRenderGate::WaitUntil(deadline)` is active,
set `ControlFlow::WaitUntil(deadline)` and do not call `request_redraw()` early.
At/after the deadline request one redraw. Armed entry and unpresented frame states
request redraw immediately; nontransition screens retain their existing loop.
Resize/expose/CloseRequested events remain serviceable, but an early transition
RedrawRequested must also take the no-present wait branch.

Handle the Task 2-literalized timeout poll variant here with the exact Task 1
source visibility, action ownership/execution, error signal, and redraw behavior.
It must not fall through to ordinary `0x4ED` completion unless the binary report
proves that outcome.

**Step 4: Apply receipts after presentation**

Keep the report in a local variable. Submit the command encoder, call
`output.present()`, then call `commit_presented_shell_frame`. If surface
acquisition/rendering returns early with an error, no receipt is applied.

For an exact shell render, report the exact `ShellTransitionFrameId` with
`ExactPath`. If the normal fallback was actually drawn and presented, report the
same ID with `UnverifiedFallback`; log the missing dependency once per
transition. Reject a report whose surface/direction/tick differs from the
controller. Never clear the controller merely because a renderer requested
fallback.

**Step 5: Cover every explicit entry origin**

Call `arm_shell_entry` for:

- initial Main Menu creation after initialization;
- initial development Skirmish shell creation when that mode is selected;
- Main Menu created after Mission Result dismissal;
- Main Menu created by `return_to_main_menu`; and
- destinations created by pending actions in Tasks 10–12.

Skip Main Menu entry when `RA2_QUICKPLAY` starts in Loading. A revealed existing
parent is not a creation and must not call this helper.

**Step 6: Move native shell input gating before egui consumption**

For keyboard, mouse button, cursor move, wheel, and text input targeting a native
shell, check `pending_shell_entry.is_some() ||
shell_transition.blocks_input()` before `egui.on_window_event` or shell hit
testing. Resize/redraw and `CloseRequested` remain serviceable.
`blocks_shell_input` remains true during the quit cascade as well.

**Step 7: Add render/lifecycle glue tests**

Use pure report/controller tests to prove renderer error retains state, presented
fallback advances without certification, stale receipts do not advance,
completion remains blocked for 29 ms after the terminal presentation and runs at
30 ms, early redraws perform no acquire/present, creation without paint emits no
sound/wave and blocks shell input, the matching first paint starts both exactly
once, and an entry completion plays the configured hook before reveal.

**Verify:**

```powershell
cargo test -p vera20k app_shell_transition -- --nocapture
cargo check -q -p vera20k
```

### Task 10: Migrate ordinary Main Menu and Single Player routes

**Why:** Main↔SP and SP→Skirmish currently mutate visibility immediately, exposing
the destination before the source close wave.

**Files:**

- Modify: `src/app.rs`
- Modify: `src/app_shell_transition/pending.rs`

**Pattern:** Existing route helpers split into “prepare/request close” and
“execute completed action” halves.

**Step 1: Add one pending-action executor**

`execute_pending_shell_action(state, action)` is the only function allowed to
retire a closing lifetime and expose/create the next one. For:

- `OpenSinglePlayer`: retire `0xE2`, set SP flags/state, refresh Load state, make
  input controller active on `0x100`, then arm `0x100` entry;
- `OpenMainMenu`: retire `0x100`, clear SP pressed/hover state, activate `0xE2`,
  then arm `0xE2` entry;
- `OpenSkirmish`: retire `0x100`, preserve the verified return-to-SP flag, ensure
  chrome/cooperative selection, activate `0x102`, then arm `0x102` entry.

The destination is never visible before `Close0x4Ed` supplies the action.

**Step 2: Change action handlers to request close**

- Main Menu Single Player requests close of `0xE2` with `OpenSinglePlayer`.
- Single Player Main Menu requests close of `0x100` with `OpenMainMenu`.
- Single Player Skirmish requests close of `0x100` with `OpenSkirmish`.

Preserve the existing enabled owner-draw input order: mouse-down
`GUIMainButtonSound`, any verified released-to-pressed paint
`GenericClick`/pressed art, then the parent command handler. Drain those already
queued sounds before accepting close. Do not play a second click from the route
handler and do not substitute `ShellButtonSlideSound` or the close cue.

Clear pressed/capture UI state as part of command resolution, but retain source
layout/content/lifetime state required to paint close.

**Step 3: Apply Task 1 Escape result**

Task 2 inserts the literal native behavior. Route Escape through the same pending
action only if Task 1 proves it synthesizes the matching close command. Otherwise
remove the current direct close and encode the verified no-op/dismissal result.
No keyboard path may bypass the transition controller.

**Step 4: Add route tests**

With a pure fake route/effect sink, assert source stays active for every close
frame, destination entry begins after close completion, repeated clicks are
ignored, and each route creates one destination lifetime. The ordered event
trace must place `GUIMainButtonSound` (and `GenericClick` when its paint edge is
present) before command resolution and close-start audio/tick zero, with no
duplicate click event.

**Verify:**

```powershell
cargo test -p vera20k app_shell_transition::pending::tests -- --nocapture
cargo test -p vera20k app_shell_transition -- --nocapture
```

### Task 11: Migrate Skirmish Start, Back, and confirmed Exit without reordering transactions

**Why:** These routes carry launch payload, RNG, persistence, and quit-order
stakes beyond simple visibility.

**Files:**

- Modify: `src/app.rs`
- Modify: `src/app_shell_transition/pending.rs`

**Pattern:** Preserve the current cooperative close-transaction ordering; defer
only visible teardown/destination execution.

For Start and Back, retain the same owner-draw mouse-down/paint click events
described in Task 10 before validation/preparation and close request. Exit's
initial button click retains that feedback before opening `0x120`; the confirm
control retains its own verified modal click feedback before the confirmed
command. None is replayed at close completion.

**Step 1: Start Game preparation and request**

Keep `launch_session` validation first. On failure, show the validation modal and
do not request close. On success, call `can_request_shell_close(0x102)` before
any cooperative/RNG mutation; Busy, wrong-source, armed-entry, or missing-spec
results leave the runtime untouched. After a successful preflight:

1. call `offline_skirmish_runtime.close_shell_transaction` exactly once;
2. if it fails, retain `0x102` and request redraw;
3. sync legacy settings once;
4. store the returned owned `SkirmishLaunchSession` in
   `BeginSkirmishLoading`; and
5. request close of `0x102` without clearing shell flags/state.

Because input handling is single-threaded and no callback runs between preflight
and request, the final request must succeed. Still handle its `Result`: retain
the returned owned session and report an invariant failure rather than dropping
it. Add a test that an injected failed preflight causes zero transaction/RNG
calls.

On completion, execute the existing verified order:

1. teardown `0x102` state;
2. persist the offline snapshot;
3. call `start_skirmish_session` with the stored session; and
4. enter game window mode/loading.

Never call the close transaction or allocate a second session after the wave.

**Step 2: Back preparation and request**

Preflight `can_request_shell_close(0x102)` first. Then keep
`pack_launch_session_without_start_validation` and the one cooperative close
transaction before animation. Capture the destination in the pending action.
Before requesting close:

- if raw packing fails, log the current warning and still request close;
- if `close_shell_transaction` fails after consuming some randomization work,
  retain those already-consumed draws/state, log the current error, and still
  request close; and
- in either error case, persist only at the same post-close point as success.

This preserves the current cooperative extension's “Back remains usable” error
contract; it must not be changed into the Start path's retain-and-redraw behavior.
At close completion:

- if the parent route is SP, retire `0x102`, persist, create `0x100`, and arm its
  entry;
- for direct/dev Skirmish, retire `0x102`, persist, create `0xE2`, and arm its
  entry; and
- do not execute a direct process exit from a visible native shell.

**Step 3: Confirmed Exit**

The initial Exit button still pushes modal `0x120`; it does not close `0xE2`.
Cancel/Enter/Escape dismissal behavior remains owned by the existing verified
modal path. On actual confirm button activation:

1. preflight close of the underlying in-scope `0xE2`; on failure keep `0x120`
   and perform no persistence/cascade mutation;
2. persist settings before teardown;
3. pop/clear `0x120`;
4. request close of `0xE2` with `BeginQuitCascade`;
5. keep drawing `0xE2` through close; and
6. start the graceful music/voice/fade cascade only after close completion.

The degraded egui fallback has no native source surface and remains an explicit
unverified direct cascade path.

**Step 4: Preserve abnormal window close**

`WindowEvent::CloseRequested` continues to persist/teardown and call
`event_loop.exit()` directly. Add a comment/test boundary so it cannot be
mistaken for the Exit-button route.

**Step 5: Add ordering tests**

Use fake counters/events to assert:

- invalid Start performs zero close requests;
- Busy/source/spec/armed-entry preflight failure performs zero cooperative or RNG
  work and retains the source;
- successful Start validates/transactions once, then closes, then persists,
  then launches once;
- Back consumes its transaction once and persists after retirement;
- Back pack/transaction errors retain consumed work, still close once, and still
  persist only after close completion;
- confirmed Exit persists, closes `0xE2`, then starts cascade; and
- Start, Back, Exit, and confirm click events precede their command/close effects
  exactly once;
- close completion cannot start Skirmish reveals.

Retain and run existing launch/session tests.

**Verify:**

```powershell
cargo test -p vera20k skirmish_shell -- --nocapture
cargo test -p vera20k match_bootstrap -- --nocapture
cargo test -p vera20k app_shell_transition -- --nocapture
```

### Task 12: Give Choose Map its own modal lifetime and preserve the parent

**Why:** `0x6B` is already a separate rendered dialog but currently piggybacks
`0x102` visibility and enters/closes instantly.

**Files:**

- Modify: `src/app.rs`
- Modify: `src/ui/skirmish_shell/state/choose_map.rs`
- Modify: `src/ui/shell/controller.rs`

**Pattern:** Existing `DialogController::push`/`pop` LIFO behavior plus current
owned `ChooseMapSelection` payload.

**Step 1: Open without closing the parent**

On Choose Map activation:

1. clear transient parent gestures/dropdowns/status as today;
2. create `ChooseMapModalState` without mutating committed parent selection;
3. push dialog `0x6B` over `0x102` in `shell_controller`;
4. preserve all `0x102` reveal/selection/preview state;
5. arm `ChooseMap0x6B` entry; its first modal paint starts the wave and
   SHOW-start sound; and
6. let the existing modal-only renderer hide the parent.

Do not request `0x102` close.

**Step 2: Accept/cancel request `0x6B` close**

- Preserve each enabled modal owner-draw control's verified mouse-down/paint
  click feedback before its command is converted to a pending action.
- Use Map calls `accept_selection()` to capture an owned, still-uncommitted
  payload and requests `0x6B` close with `FinishChooseMap(Accept(payload))`.
- Cancel requests `0x6B` close with `FinishChooseMap(Cancel)`.
- Create Random Map retains its current recognized/nonimplemented destination
  behavior; it neither borrows this return path nor fabricates a parent replay.

Keep the modal state present and visible throughout all 11 close frames.
If close preflight/request fails, recover the returned payload, leave the modal
and parent unchanged, and do not commit a selection.

**Step 3: Finish after close completion**

On `FinishChooseMap`:

1. for Accept, call `commit_choose_map_selection` once; if commit fails, log the
   exact reason and preserve the prior parent selection;
2. for Cancel, perform no commit;
3. clear modal pressed/state;
4. pop `0x6B` so focus returns to the existing `0x102` entry;
5. expose that same parent; and
6. do not arm or begin `Skirmish0x102` entry.

Dynamic game/map text restarts occur only when a successful Accept actually
changes those child texts.

**Step 4: Add identity/lifetime tests**

Extend current Choose Map state tests to assert parent values/reveal counts are
unchanged on open and Cancel, accept commits only after close terminal, parent
controller entry is restored, no `0x102` entry sound/wave occurs, and hidden
parent paint receipts are absent. Ordered event tests assert one modal click
event before the `0x6B` close cue/tick zero and no replay after completion.

**Verify:**

```powershell
cargo test -p vera20k ui::skirmish_shell::state::tests::choose_map -- --nocapture
cargo test -p vera20k ui::shell::controller::tests -- --nocapture
cargo test -p vera20k app_shell_transition -- --nocapture
```

### Task 13: Replace the timer-driven wipe with paint-driven native reveal state

**Why:** Current default-full text, scalar counts, timer-side increments, and
lost completed-running state contradict the verified `0x102` mechanism.

**Files:**

- Modify: `src/ui/skirmish_shell/static_reveal.rs`
- Modify: `src/ui/skirmish_shell/state/player_name.rs`
- Modify: `src/ui/skirmish_shell/state/choose_map.rs`
- Modify if Task 1 finds qualifiers: `src/ui/main_menu_shell/state.rs`
- Modify if Task 1 finds qualifiers: `src/ui/single_player_shell/state.rs`
- Modify: `src/app_skirmish_shell_render/text.rs`
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/app_skirmish_shell_render/modals.rs`
- Modify if Task 1 finds qualifiers: `src/app_main_menu_shell_render.rs`
- Modify if Task 1 finds qualifiers: `src/app_single_player_shell_render.rs`
- Modify: `src/app_shell_transition.rs`

**Pattern:** Pure reveal state in UI; immutable draw snapshot; post-present receipt
in app glue.

**Step 1: Define explicit phases/draw values**

```rust
pub(crate) const STATIC_REVEAL_INTERVAL_MS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticRevealPhase {
    Waiting,
    Running {
        count: u32,
        target: u32,
        paint_dirty: bool,
        next_timer_at: Instant,
    },
    Completed {
        target: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealDraw {
    Hidden,
    Window { count: u32, range: u32 },
    Full,
}
```

`Default` is `Waiting`; `draw()` returns `Hidden`. `is_running()` is false only
for Waiting and true for Running/Completed. The Task 2 literal reveal table
supplies each surface's interval/step/range; stock `0x102` is 30/1/8.

**Step 2: Implement start/timer/presentation semantics**

For the verified `0x102` values:

- `start(text, now)` uses `text.encode_utf16().count() + 9`, count `1`, dirty
  true, and next timer `now+30ms`;
- `poll_timer(now)` only marks dirty and resets the next deadline from actual
  `now`, never increments and never catches up;
- `record_presented()` follows Task 2's literal dirty/redundant-paint gate and
  increments only after that exact control was included in the presented frame;
- when the increment reaches target, move to Completed and stop the timer while
  retaining `is_running()==true`;
- Waiting text changes remain Waiting; and
- Running/Completed dynamic text changes restart at count 1.

Apply Task 2's literal membership/defaults and repaint-gate semantics for
`0x6B`, `0xE2`, and `0x100`. Store `0x6B` state in the modal lifetime; if Task 1
proves Main/SP qualifiers, store them in their respective shell state rather
than borrowing the Skirmish fields.

**Step 3: Replace frame-driven advance helpers**

Replace `advance_right_panel_static_reveals` with timer polling. Add a method that
accepts `RevealPaintMask` after presentation and resolves its bits through the
current surface spec's `reveal_controls` order before calling
`record_presented`. While Choose Map hides `0x102`, its parent mask is empty and
counts do not advance.

**Step 4: Emit blank/window/full correctly**

In every Task 1-qualified surface text builder:

- Hidden emits no label instances;
- Window passes count/range to Path A;
- Full uses ordinary full text; and
- the returned `ShellPaintReport` sets a reveal bit only for a Window label
  actually emitted into that surface.

**Step 5: Add exhaustive state tests**

Cover default blank, count-1 first draw, timer-without-paint, paint-without-dirty,
failed/hidden paint, UTF-16 target fixtures, final transition to Completed,
running retained, Waiting update, Running update, Completed update, and close
completion no-op.

**Verify:**

```powershell
cargo test -p vera20k ui::skirmish_shell::static_reveal::tests -- --nocapture
cargo test -p vera20k app_skirmish_shell_render -- --nocapture
```

### Task 14: Implement exact BITFONT Path-A u16 reveal and tinting

**Why:** Range 8 is a native leading-edge color window, not a hard left-to-right
cutoff. Rust scalar iteration and uniform tint cannot produce the same pixels.

**Files:**

- Create: `src/render/shell_text_reveal.rs`
- Modify: `src/render/mod.rs`
- Modify: `src/render/shell_text.rs`
- Modify: `src/render/bit_font.rs`

**Pattern:** New Path-A-specific helper; existing Path B/sidebar code is untouched.

**Step 1: Add narrow u16 font primitives**

Expose crate-private methods for glyph lookup, missing glyph, u16 width, and one
glyph instance emission. Do not change `build_text`, `text_width`,
`wrap_layout`, or sidebar APIs. Add tests proving ASCII steady output remains
byte-for-byte equal before/after the refactor.

**Step 2: Add a Path-A wide layout**

`shell_text_reveal` encodes the input once with `encode_utf16`, applies the Task 1
u16 wrapping/control-character rules, and carries half-open u16 ranges across
wrapped lines. It returns glyph instances plus the number of native reveal units
consumed so one global count/range spans all lines.

**Step 3: Implement Task 2's literal packed-color function**

Task 2 must insert the complete integer body here before execution:

```rust
pub(crate) fn path_a_color(
    base_packed: u16,
    highlight_packed: u16,
    reveal_index: u32,
    count: u32,
    range: u32,
    format: NativeShellPixelFormat,
) -> Option<u16>;
```

The function returns `None` for native-clipped units and the exact packed tint
for visible units. Perform native integer arithmetic before converting packed
channels to `SpriteInstance.tint`; do not interpolate in `f32`. Include the
Task 1 loss/shift/pixel-format binding explicitly.

**Step 4: Route only kind-1 Window draws through Path A**

Ordinary full shell text remains on the existing path. `RevealDraw::Window`
calls the new helper with base shell color, verified selected-unit highlight,
count, and range. Do not reuse sidebar Path B or its color constants.

**Step 5: Add native-vector and regression tests**

Paste every Task 1 vector as literal expected packed values. Cover line wrapping,
spaces, CR/LF, missing glyph, range zero, count boundaries, completed range,
valid surrogate pair behavior, clipping, and both active pixel formats if the
binary path supports both. Assert ordinary `build_text` and sidebar tests are
unchanged.

**Verify:**

```powershell
cargo test -p vera20k render::shell_text_reveal::tests -- --nocapture
cargo test -p vera20k render::shell_text::tests -- --nocapture
cargo test -p vera20k render::bit_font::tests -- --nocapture
```

### Task 15: Add exact SDMPBTN/SDWRNTMP transition chrome and retail guards

**Why:** `0x102` currently draws SDMPBTN steady and omits SDWRNTMP; `0x6B` also
has a verified D9 optional group. Every affected transition frame is visibly
wrong without the exact groups.

**Files:**

- Modify: `src/render/skirmish_shell_chrome.rs`
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/app_skirmish_shell_render/chrome.rs`
- Modify: `src/app_skirmish_shell_render/draw_order.rs`

**Pattern:** Existing atlas label/frame array for SDBTNANM, extended to the two
verified retail SHPs. No asset frame or coordinate is hardcoded outside the Task
1 contract and retail metadata checks.

**Step 1: Load full asset frame arrays**

Add:

```rust
pub sdmpbtn_frames: [Option<SkirmishShellChromeEntry>; 7],
pub sdwrntmp_frames: [Option<SkirmishShellChromeEntry>; 6],
```

Decode SDMPBTN frames `0..=6` and SDWRNTMP `0..=5` with the verified palette.
Keep the steady `sd_map_button` alias bound to SDMPBTN frame 0 if it simplifies
existing callers.

**Step 2: Implement Task 2's literal optional-frame functions**

Task 2 inserts complete functions keyed by `surface`, `direction`, and `tick`
that return exact optional frame/underlay/anchor/visibility data. Enable:

- no optional groups for `0xE2` or `0x100`;
- D9 and DA groups for `0x102` exactly as verified; and
- D9 only for `0x6B`.

Do not infer missing frames or reuse SDBTNANM formulas.

**Step 3: Apply exact composition order and rectangles**

Insert optional roles at the Task 1 draw order and exact rect conversion. Keep
steady `0x102` SDTP frame 1 and SDMPBTN frame 0 behavior unchanged outside a
wave. No in-scope ordinary SDTP receives `+80`.

**Step 4: Make missing assets explicit**

If an enabled optional group requests a missing exact frame, return
`UnverifiedFallback` with asset/frame identity. Do not silently skip, clamp, or
substitute frame 0.

**Step 5: Add retail-dependent validation**

Extend the existing ignored retail test to fail explicitly when deliberately
invoked and assets are absent/malformed. Assert:

- SDBTNANM: 17 frames, every referenced frame `156x42`;
- SDMPBTN: 7 frames, `156x84`; and
- SDWRNTMP: 6 frames, `168x177`.

Add pure per-tick role/frame/order/rect tests for SHOW and CLOSE on `0x102` and
`0x6B`, and negative tests for `0xE2/0x100`.

**Verify:**

```powershell
cargo test -p vera20k app_skirmish_shell_render -- --nocapture
cargo test -p vera20k render::skirmish_shell_chrome::tests -- --nocapture
cargo test -p vera20k retail_shell_transition_assets_match_contract -- --ignored --nocapture
```

### Task 16: Build the composed-frame capture and native pixel-oracle harness

**Why:** Passing Rust tests cannot certify gamemd pixel parity. This task creates
the executable comparison boundary required by the design.

**Files:**

- Modify: `src/render/screenshot.rs`
- Create: `src/render/shell_frame_compare.rs`
- Modify: `src/render/mod.rs`
- Create: `src/bin/shell-frame-compare.rs`
- Modify: `Cargo.toml`
- Create: `tests/shell_transition_pixel_oracle.rs`
- Create: `docs/visual-checks/shell-transitions/README.md`

**Pattern:** Existing `image` dependency and `tests/bink_frame_diff.rs` external
oracle shape, but missing fixtures fail explicitly when the ignored test is
requested.

**Step 1: Capture the actual composed Rust surface**

Add an opt-in `VERA_SHELL_CAPTURE_DIR` path. When enabled, encode a copy from the
already-rendered swapchain texture (surface usage already includes `COPY_SRC`)
into a padded readback buffer before encoder submission. After submit, map and
save the exact logical surface before `present()`.

Capture only the first accepted presentation of a transition/reveal state. File
names contain dialog ID, direction, logical resolution, tick, reveal counts,
and fidelity. Production behavior performs no readback/allocation when the env
var is absent.

**Step 2: Implement one shared comparator and register its thin binary**

Put manifest parsing, provenance validation, packed-pixel comparison, mismatch
reporting, and diff-image generation in
`render::shell_frame_compare`. Both the binary and integration test call this
library module; neither shells out to or duplicates the other.

Add to `Cargo.toml`:

```toml
[[bin]]
name = "shell-frame-compare"
path = "src/bin/shell-frame-compare.rs"
```

The thin binary parses CLI paths and calls the shared module. The module reads a
manifest, loads native/Rust PNGs, checks dimensions, applies the Task 1 native
packed-pixel conversion to Rust output, and reports:

- total/mismatching pixels;
- first `(x,y)` mismatch with native/Rust packed values;
- per-channel maximum delta before packing; and
- optional diff PNG with unchanged pixels transparent.

Exit code is nonzero on missing files, dimension mismatch, metadata mismatch, or
any packed-pixel mismatch.

**Step 3: Define the corpus manifest/provenance**

The README fixes this tree:

```text
docs/visual-checks/shell-transitions/
  manifest.json
  native/<gamemd-sha256>/<dialog>/<direction>/<resolution>/tick-NN.png
  rust/<git-tree-id>/<dialog>/<direction>/<resolution>/tick-NN.png
  diffs/<dialog>/<direction>/<resolution>/tick-NN.png
```

Each manifest row includes gamemd SHA-256, rules/rulesmd SHA-256, relevant MIX
hashes, capture boundary/pitch/pixel format, dialog/control state, movie frame,
cursor position/frame, hover/pressed state, transition tick, reveal counts, and
source dimensions. Use the native surface extraction method proved in Task 1;
do not accept DPI-scaled desktop images.

**Step 4: Add an ignored corpus test**

`tests/shell_transition_pixel_oracle.rs` reads
`VERA_SHELL_ORACLE_MANIFEST`. When explicitly invoked, absent/incomplete
fixtures are test failures. It runs the same comparator library logic for:

- all entry and close ticks of `0xE2`, `0x100`, `0x102`, and `0x6B`;
- 640x480, 800x600, and one verified wide resolution;
- every Task 1-qualified reveal child on every surface, from count 1 through
  completed (including `0x6B` and Main/SP if verified), with control ID and text
  payload in each manifest row; and
- Task 1's composition-state variants.

**Step 5: Record honest verdicts**

Unit/asset tests may be green while the corpus is absent or red. In that case
the implementation status is `UNVERIFIED`, never `VERIFIED`.

**Verify:**

```powershell
cargo test -p vera20k --test shell_transition_pixel_oracle -- --ignored --nocapture
cargo run -p vera20k --bin shell-frame-compare -- --manifest docs/visual-checks/shell-transitions/manifest.json
```

Expected only after native fixtures exist: zero missing rows, zero dimension
mismatches, and zero packed-pixel mismatches.

### Task 17: Final focused, full-suite, route, and parity verification

**Why:** This change spans pure timing, app navigation, renderer composition,
audio configuration, assets, and external pixels. All layers need a named final
check.

**Files:** No new production files. Update the visual-check README only with
actual commands/results and remaining `UNVERIFIED` items.

**Step 1: Check build ownership before Cargo**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

If another session owns Cargo, wait. Do not kill it.

**Step 2: Format only edited Rust files**

Re-read the dirty diffs, then run `rustfmt` only on the Rust files actually
edited by Tasks 3–16 (remove any untouched path from this list first):

```powershell
$shellTransitionRust = @(
    'src/ui/shell/slide.rs',
    'src/ui/shell/controller.rs',
    'src/app_shell_transition.rs',
    'src/app_shell_transition/controller.rs',
    'src/app_shell_transition/pending.rs',
    'src/app.rs',
    'src/app_main_menu_shell_render.rs',
    'src/app_single_player_shell_render.rs',
    'src/app_skirmish_shell_render.rs',
    'src/app_skirmish_shell_render/modals.rs',
    'src/app_skirmish_shell_render/chrome.rs',
    'src/app_skirmish_shell_render/text.rs',
    'src/app_skirmish_shell_render/draw_order.rs',
    'src/render/shell_paint.rs',
    'src/render/skirmish_shell_chrome.rs',
    'src/ui/skirmish_shell/static_reveal.rs',
    'src/ui/skirmish_shell/state/player_name.rs',
    'src/ui/skirmish_shell/state/choose_map.rs',
    'src/ui/main_menu_shell/state.rs',
    'src/ui/single_player_shell/state.rs',
    'src/render/shell_text_reveal.rs',
    'src/render/shell_text.rs',
    'src/render/bit_font.rs',
    'src/rules/ruleset.rs',
    'src/render/screenshot.rs',
    'src/render/shell_frame_compare.rs',
    'src/render/mod.rs',
    'src/bin/shell-frame-compare.rs',
    'tests/shell_transition_pixel_oracle.rs'
)
rustfmt --edition 2024 $shellTransitionRust
```

Inspect `git diff -- <those paths>` afterward. Preserve all unrelated user work;
do not revert another session's edits to make formatting cleaner.

**Step 3: Run focused tests serially**

```powershell
cargo test -p vera20k ui::shell::slide::tests -- --nocapture
cargo test -p vera20k app_shell_transition -- --nocapture
cargo test -p vera20k rules::ruleset::tests::shell_transition_sound -- --nocapture
cargo test -p vera20k app_main_menu_shell_render -- --nocapture
cargo test -p vera20k app_single_player_shell_render -- --nocapture
cargo test -p vera20k app_skirmish_shell_render -- --nocapture
cargo test -p vera20k ui::skirmish_shell::static_reveal::tests -- --nocapture
cargo test -p vera20k render::shell_text_reveal::tests -- --nocapture
cargo test -p vera20k render::bit_font::tests -- --nocapture
cargo test -p vera20k ui::shell::controller::tests -- --nocapture
```

Read and report each literal `test result:` line. Every command must show zero
failed tests.

**Step 4: Run the retail and external oracle checks**

```powershell
cargo test -p vera20k retail_shell_transition_assets_match_contract -- --ignored --nocapture
cargo test -p vera20k --test shell_transition_pixel_oracle -- --ignored --nocapture
```

The second command is allowed to remain blocked only if the final report labels
pixel parity `UNVERIFIED` and names the exact missing native artifacts. It cannot
be silently skipped.

**Step 5: Run the full regression/build gate**

```powershell
cargo test -p vera20k --all-targets -- --nocapture
cargo check -q -p vera20k
```

If failures are from unrelated dirty work, record the exact pre-existing error
and do not repair/revert another session's files.

**Step 6: Walk the visible route matrix in VERA20k**

Run `cargo run --bin vera20k` and verify:

1. startup `0xE2` entry;
2. `0xE2 -> 0x100` close then entry;
3. `0x100 -> 0x102` close then entry;
4. `0x102` validation failure with no close;
5. successful Start close before Loading;
6. `0x102` Back close then `0x100` entry;
7. Choose Map entry, Cancel close, same-parent reveal with no replay;
8. Choose Map entry, Accept close, post-close commit/text restart;
9. Exit click opens `0x120`, Cancel stays; confirmed Exit closes `0xE2` before
   cascade; and
10. window CloseRequested exits directly without a fabricated wave.

At each route, attempt repeated clicks during entry/close and confirm they have
no effect.

**Step 7: Final status**

Report separately:

- functional/regression status;
- mechanism evidence status;
- retail asset validation status; and
- native pixel-oracle status.

No commit, staging, branch, push, or golden rebaseline is part of this plan.

**Verification:** Every serial command above must report its literal
`test result:`/exit status. Functional green does not upgrade pixel parity;
`VERIFIED` is allowed only when the complete gamemd-derived corpus reports zero
packed-pixel mismatches.

---

## Sources & References

### Approved design and current scan

- `docs/plans/2026-07-18-shell-dialog-lifetime-transition-design.md`
- `docs/gap-scans/2026-07-18-disparity-scan-main-menu-skirmish-shell.md`
- `docs/gap-scans/2026-07-06-disparity-scan-shell-ui.md`

### Primary shell transition evidence

- `docs/research/skirmish-ui/SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-3.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-19.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-20.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-21.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-22.md`
- `docs/research/traces/SHELL_SLIDE_CONTROL_ENUM_GROUP_SPLIT_TRACE.md`
- `docs/research/traces/SHELL_SLIDE_TICK_SCHEDULE_FORMULA_TRACE.md`
- `docs/research/traces/SHELL_SLIDE_SDBTNANM_FRAME_SCHEDULE_TRACE.md`
- `docs/research/traces/SHELL_SLIDE_SWARM_RECONCILIATION.md`
- `docs/research/SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`
- `docs/research/RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md`
- `docs/research/SINGLE_PLAYER_TO_SKIRMISH_FUN_006071E0_FLAGS_ASSETS_GHIDRA_REPORT.md`

### Dialog flow and composition evidence

- `docs/research/skirmish-ui/SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_NATIVE_SINGLE_PLAYER_ROUTE_TO_0X102_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_POST_IMPLEMENTATION_GAP_AUDIT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_BROAD_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_0X102_TOP_PREVIEW_CHROME_SDTP_SDMPBTN_GHIDRA_REPORT.md`
- `docs/research/SDMPBTN_SDWRNTMP_RECT_CONSUMERS_GHIDRA_REPORT.md`
- `docs/research/MAIN_MENU_SHELL_TRANSITION_ASSET_SURVEY_2026_05_27.md`

### Static text and BITFONT evidence

- `docs/research/skirmish-ui/SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_STATIC_STATUS_TEXT_INTEGRATION_GHIDRA_REPORT.md`
- `docs/research/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`

### Audio/quit evidence

- `docs/research/QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`
- `docs/research/MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md`
- `docs/research/SHELL_UI_SOUND_PLAYBACK_PLUMBING_GHIDRA_REPORT.md`
- `docs/research/SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/shell-sounds-music-verify.md`

### Known stale/conflicting reports to use only through corrections

- `docs/research/SHELL_TRANSITION_ON_MAIN_MENU_CLICK_GHIDRA_REPORT.md` — stale
  invalid/no-close caller implications corrected by `gh-3` and generic first-paint
  evidence.
- `docs/research/CHOOSE_MAP_PREMODAL_HELPER_0X00608070_GHIDRA_REPORT.md` — stale
  caller-argument conclusion corrected by `gh-3`.
- `docs/research/FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` — useful
  optional-branch navigation, but its direction/message interpretation is
  superseded by `gh-21`; do not copy formulas without Task 1 re-audit.
- `docs/research/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` — high-level Path-A shape
  is useful; its older claim that Skirmish passes zero fade is superseded by the
  dedicated `0x102` static-reveal report.

### INI and retail data

- `ini/rules.ini` `[AudioVisual]`:
  `GUIMainButtonSound=MenuClick`, `GUIMoveOutSound=MenuSlideOut`,
  `GUIMoveInSound=MenuSlideIn`, `ShellButtonSlideSound=`.
- `ini/rulesmd.ini` contains the same YR override values and takes priority.
- Retail assets under
  `<ra2-install>/`:
  SDBTNANM `156x42x17`, SDMPBTN `156x84x7`, SDWRNTMP `168x177x6`,
  SDTP `168x199x2`.

### Current Rust patterns

- `src/ui/shell/slide.rs`
- `src/ui/shell/controller.rs`
- `src/app_shell_transition.rs`
- `src/app.rs`
- `src/app_main_menu_shell_render.rs`
- `src/app_single_player_shell_render.rs`
- `src/app_skirmish_shell_render.rs` and submodules
- `src/render/shell_paint.rs`
- `src/render/skirmish_shell_chrome.rs`
- `src/ui/skirmish_shell/static_reveal.rs`
- `src/ui/skirmish_shell/state/choose_map.rs`
- `src/render/shell_text.rs`
- `src/render/bit_font.rs`
- `src/rules/ruleset.rs`
- `src/render/screenshot.rs`
- `tests/bink_frame_diff.rs`

### Binary anchors

- `FUN_006071E0` — common shell transition loop.
- `FUN_00608260` — SHOW wrapper/start cue path.
- `FUN_00608070` — CLOSE wrapper.
- `FUN_0060C540` / `0x00610CA0` — dialog record and first-paint trigger path.
- `FUN_00622720` — generic dialog teardown.
- `FUN_007757E0` — modal pop/teardown.
- `FUN_0060A180`, `FUN_0060A250`, `FUN_00609730` — control enumeration and
  special-control classification.
- `FUN_00602490`, `FUN_0060A5B0`, `FUN_006153E0` — static kind/reveal path.
- `FUN_00434CD0`, `FUN_00621040` — BITFONT Path A and wrapper.
- `0x005E68A0` / dialog `0x6B` — Choose Map creation/callback lifetime.

---

## Post-Plan Self-Review Checklist

**Review result (2026-07-18): GREEN for the evidence-gated plan.** An independent
read-only pass found no remaining structural contradiction. Task 2 still requires
a second GREEN review after Task 1 literalizes the gated binary values; this
result does not waive that hard pass condition.

- [x] Every approved design requirement maps to Tasks 1–17.
- [x] Tasks 1–2 remain a hard pre-code gate.
- [x] No evidence-dependent literal is guessed.
- [x] Creation only arms entry; the matching first paint starts wave/audio.
- [x] No screen-edge entry authority remains after Task 9.
- [x] Control schedules are keyed by resource ID, not renderer enumeration.
- [x] Exactly `N_A+9` frames are presented; no terminal extra frame exists.
- [x] The terminal frame's final 30 ms delay precedes completion/action.
- [x] Waiting intervals produce no duplicate swapchain presentation.
- [x] Start/Back cooperative transaction and persistence order is preserved.
- [x] Button click/pressed audio remains ordered before close initiation.
- [x] Choose Map retains the same parent and never replays parent entry.
- [x] Reveal count changes only from a matching presented paint receipt.
- [x] Path A is separate from sidebar Path B and uses u16/native packed math.
- [x] Missing exact frames are explicit fallbacks, never silent substitutions.
- [x] Timeout/error behavior is literalized before controller/app implementation.
- [x] Dirty worktree checks precede every implementation edit.
- [x] No `sim/` file is touched.
- [x] No commit/push/branch/staging step is present.
- [x] Rust regression evidence and gamemd parity evidence are reported separately.
