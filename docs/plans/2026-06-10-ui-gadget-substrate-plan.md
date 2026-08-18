# UI Gadget Substrate — Implementation Plan (A0, A1, A4, A5, D-B3, R1)

> **For Claude:** Execute this plan task-by-task. Each task is self-contained. DOC-ONLY artifact:
> the code blocks here are the target source; do not assume they are already in `src/`.
>
> **EXECUTION TARGET:** the worktree `<local>/Documents/ra2-uigadget-worktree`
> (branch `ui-gadget-substrate` @ baseline commit `7b79a186`). ALL file paths below are relative
> to that worktree; ALL verify commands run **in the worktree**, e.g.
> `cd <local>/Documents/ra2-uigadget-worktree && cargo test -p vera20k <filter>`.
> Do NOT edit the main repo checkout (it carries another session's uncommitted sim changes).
> All commits land on branch `ui-gadget-substrate`.

**Goal:** Stand up the Framework-A gadget substrate (`ui::gadget`: retained list = hit priority =
draw order, sticky capture, fire-on-RELEASE button machine per study §5 G-clauses), flip the
sidebar buttons onto it (tabs/repair/sell + the new strip-scroll pair), add the two missing
player-visible services (shared 1000 ms tooltip service; chat/system message TextLabel surface),
fix the exit-confirm Esc controller bypass (D-B3), and delete the dead `in_game_hud.rs` (R1).

**Architecture:** New render-agnostic `src/ui/gadget/` module beside the shipped `ui::shell`
(study §6.1), driven by a flat app-layer driver (`app_gadget_input.rs`, pattern:
`app_sidebar_gadgets.rs`). The gadget tick is **deterministic and wall-clock-free** — all inputs
arrive in a `GadgetInput` snapshot. Only the tooltip service consumes time, behind an injectable
`now_ms: u64` (tests pass synthetic clocks). New `ui/tooltips.rs` + `ui/messages.rs` models are
pure (text measurement injected); app drivers (`app_tooltips.rs`, `app_messages.rs`) own wall
clock, region sync, sounds, and render hand-off. `sim/` logic is untouched (two `cfg(test)`
fixture literals gain a field — see Sim Checklist) — sim-affecting clicks keep flowing
through the existing `app_commands` seam (study O14).

**Design Doc:** `docs/research/GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
(§5 behavior contract, §6 replacement boundary, §8 slices) + the three grounding lanes in
`docs/research/substrate/worknotes/gadget-dialog-20260610/`
(`plan-grounding-contract.md`, `plan-grounding-rust.md`, `plan-grounding-ini.md`).

---

## Grounding Summary

- **Contract (study §5 + plan-grounding-contract §1):** the full Input tick is pinned at
  implementer pseudocode level — G5 fresh-list reset, G6 coordinate source (event coords iff
  key low byte 1/2), G8 flag assembly (held bits on idle ticks ONLY), G9 modifier word
  (broadcast tier only, hardwired 0 for sticky/keyboard tiers), G10 tier precedence
  (sticky > keyboard > broadcast, exclusive), G11/G12 draw cadence, G13 `ID|0x8000`
  (`|0x4000` iff right-release AND mask has 0x10), G14 hit-test (HALF-OPEN, smallest-area,
  signed `<=` tie-break so later-in-list wins, seed = 786,432 px²), G15 mask-first filter with
  sticky/keyboard bypasses, G17 capture (0x11 acquire iff sticky / 0x44 holder-only release),
  G22 button machine as a 7-row state table (silent press forces `*key=0` return 1;
  inside-test uses LIVE mouse; drag-off via masked-0 sticky re-dispatch), G23 hold-repeat =
  mask property only.
- **A1 identities (VERIFIED-LIVE, contract lane §2.1, decompile 0x006A5310):** tabs IDs
  0xCB..0xCE **Kind 2 (latch-ON)**, IsOn driven externally; repair 0x65 / sell 0x66 **Kind 1
  (flip)**; strip scroll 0xC9 (+page) / 0xC8 (−page) **Kind 0, mask 0x55** (no held bits ⇒ no
  hold-repeat, one page per click). Mask 5 on tabs/repair/sell = left-only.
- **A4 (contract lane §3):** 1000 ms delay / 10000 ms duration now VERIFIED-LIVE (hardcoded,
  no INI); INCLUSIVE-both-edges rect test, first-registered-wins; every mouse-move restarts the
  delay + hides; any of 6 button messages kills; auto-hide re-arm after show; cameo hover zeroes
  the delay (override hook). Repair/sell tips = direct CSF keys `TXT_REPAIR_MODE`/`TXT_SELL_MODE`;
  tab/scroll tips = numeric CSF ids whose **label names are unknown** (deferred).
- **A5 (contract lane §4):** 14 slots × 162 wchars; retail Init x=tacticalX+3, y=tacticalY,
  6 visible, LineHeight 19 px hardcoded, MaxWidth=tacticalW−14; prefix+":"+fitted text (budget
  −8 px, ≤111 chars/line), evict head, tail insert, restack, silent wrap recursion with prefix
  re-included; insert sound = `[AudioVisual] IncomingMessage`; labels mask 0 w=h=1 ⇒ can never
  consume clicks; drawn from the message list's OWN list, never the Buttons walk.
- **Rust anchors (plan-grounding-rust, all verified @ 7b79a186):** sidebar fires on mouse-DOWN
  only (`app_input.rs:39-43`, `:227-238`); **no click sound and no press visual exist today —
  nothing to preserve in the flip**; `sidebar::Rect::contains` is INCLUSIVE (mod.rs:61-63) vs
  shell `RectPx::contains` HALF-OPEN (geom.rs:34-36) — A0 mirrors the RectPx convention;
  strip-scroll buttons DO NOT EXIST (wheel-only, `try_sidebar_scroll`); 5-frame visual table
  `frame_select` (gadget_flash.rs:112-123) already maps pressed frames 3/4 — only a transient
  press-bit source is missing; D-B3 sites verified (`app.rs:2106-2112`, `:1950-1955`,
  `controller.rs:192-194`; `ensure_active` reset_to-clobbers — comment at app.rs:1935-1936 is
  misleading, nothing is pushed over 0xE2 today); R1 zero callers confirmed.
- **INI (plan-grounding-ini):** parse gaps feeding this plan: `GUITabSound` (:645→MenuTab),
  `IncomingMessage` (:683→MessageText), `MessageDelay` (:758→.6 min), `UIName=` (nowhere parsed;
  `object_type.rs` has only `Name=`/`Cost=`). Sound events exist in soundmd.ini; playback path
  (`play_shell_ui_sound_by_id` → `SfxPlayer::play_sound`) already works.
- **Still unknown after grounding** → Deferred Open Questions: CSF numeric-id→label mapping,
  chat-timeout formula, suppression byte 0x00A8F7D8, tooltip box placement math + colors,
  R-UP/R-DN frame count, typewriter gating, TextLabel +0x28 color consumer, SW UIName parse.

## Key Technical Decisions

- **Vec-backed retained list with per-list stable handles; one list = one `ListId`; retained
  order = hit priority = draw order.** Mirrors study §6.1 exactly; lists are tiny (≤ ~70
  gadgets), linear handle lookup is fine and keeps iteration deterministic. —
  **Confidence: high.** Source: study §6.1, G1/G20/O7.
- **`FocusState` removal closure clears hover too (no Mouse_Leave on a dead handle).** The
  deliberate Rust-side closure of the G7 stale-pointer hazard; observable behavior unchanged
  (gamemd never visibly exercises the hazard). — **Confidence: high.** Source: study §6.1/G24;
  contract lane §1.8.
- **Gadget tick never reads wall-clock.** All timing-free; G23 repeat rate = tick call rate by
  contract. ONLY `ui/tooltips.rs` consumes time via injected `now_ms`. — **Confidence: high.**
  Source: plan requirement; study O13/S5.
- **Tick cadence: one event-tick per mouse press/release edge + one idle tick per render
  frame.** gamemd ticks once per gameplay tick with a queued event; we tick synchronously on the
  edge (event coords = live coords, equivalent — no queue lag exists) plus a per-frame idle tick
  for the masked-0 re-dispatch (drag-off pop). Observable deltas: hover/pop response is
  per-frame instead of per-game-tick (faster, ≤1 frame), and a held-mask gadget would repeat at
  frame rate — **no A1 gadget has held bits in its mask**, so no observable repeat drift ships.
  — **Confidence: medium — flagged for /review-plan.** Source: study O2/G23; rust lane §6.3.
- **Scroll page size = visible cameo rows (`layout.side2_tile_count`).** gamemd pages by
  `(strip px height)/0x32` = visible rows; our adaptive layout generalizes the same quantity. —
  **Confidence: medium.** Source: contract lane §2.2.
- **Scroll-button placement: pair centered in the side3 strip (down-left, up-right).** Retail
  anchors at ScrollX / ScrollX+ScrollWidth do not exist in the adaptive RON layout (R11 geometry
  policy is OUT of scope — study §6.4); this is an explicit interim position. —
  **Confidence: LOW — flagged for /review-plan + R11 follow-up.** Source: ini lane §5;
  study §6.4.
- **R-UP.SHP / R-DN.SHP assumed 5-frame convention.** Frame count UNVERIFIED; loader degrades
  gracefully (missing frames → frame-0 fallback, same as repair/sell). —
  **Confidence: LOW — flagged.** Source: ini lane §5.
- **`GUITabSound` wired to tab select.** Key→consumer mapping is name-inferred (MenuTab), not
  binary-verified; one Ghidra spot-check required before final parity sign-off (Parity table). —
  **Confidence: LOW — flagged.** Source: ini lane §1.
- **Cameo tooltip text = `"{name}\n${cost}"` interim format; SW cameos = name only.** gamemd
  formats via CSF#0xC6E (label unknown); name+cost args are verified, the format string is not.
  — **Confidence: LOW — flagged.** Source: contract lane §3.5.
- **Tab/scroll tooltips register with EMPTY text ⇒ no tip shows** (gamemd: NULL text ⇒ Show
  fails ⇒ no tip — observably identical to "not yet mapped"). Most-conservative reading until
  the CSF numeric-id mapping pass. — **Confidence: high (as a conservative stand-in).**
  Source: contract lane §3.5 + UNK §7.2.
- **Tooltip timers are poll-per-frame against injected `now_ms`** instead of SetTimer; at any
  frame rate ≥ ~30 fps the show/hide edges land within one frame of the wall-clock deadline —
  same observable behavior as the OS-timer pump. — **Confidence: high.** Source: contract lane
  §3.3; study O13.
- **Tooltip box visuals: darken-strip fill (existing Ready-strip primitive) + yellow GAME.FNT
  text at cursor+offset, clamped.** gamemd's box fill/border colors and ShowAt placement math
  are undecoded (UNK §7.10) — interim visual, named constants, fidelity-check follow-up. —
  **Confidence: LOW — flagged.** Source: ini lane §4; contract lane UNK 10.
- **Message deadlines in PAUSE-ADJUSTED ms: a pure `PauseAwareClock` subtracts accumulated
  pause spans from the wall clock; `manage()` is additionally skipped while paused.** gamemd
  computes message `now` from a pause-aware 16 ms composite timer that FREEZES during pause
  (contract §4.2 step 8 / §4.3) — a row's remaining lifetime must survive a pause intact, so
  the clock itself stops; merely skipping `manage()` would let wall-time deadlines expire the
  instant the game unpauses (visual freeze alone is NOT the contract). Beacon 225 ticks
  ≈ 3600 ms; chat default = `MessageDelay` minutes → ms. — **Confidence: medium-high.**
  Source: contract lane §4.2 step 8 / §4.3 + UNK 1.
- **MissionAnnouncement reroutes through the message list (egui banner deleted), keeping the
  current 4 s duration.** gamemd map/trigger text IS an Add_Message caller (verified census);
  the exact retail timeout for trigger text is unknown → keep 4 s as a named constant. —
  **Confidence: medium.** Source: contract lane §4.5; rust lane §5.4.
- **D-B3 = true `push(0x120)` on open + controller-routed Esc/OK/Cancel that `pop()`s**,
  mirroring the validation-modal pattern (the only existing `pop()` caller). —
  **Confidence: high.** Source: rust lane §3; controller.rs:77-111.
- **`i32` pixel coords at the gadget boundary** (round from the f32 cursor once, at the
  driver). Matches the native integer hit math and the shell RectPx convention. —
  **Confidence: high.** Source: study G14; rust lane §8.

## Open Questions

### Resolved During Planning

- *Which rect convention for A0* — HALF-OPEN, mirroring shell `RectPx` (geom.rs:34-36), NOT
  `sidebar::Rect` (inclusive). Source: study G14; rust lane §2.1/§8.
- *Do sidebar clicks have a sound/press visual to preserve?* — No. Verified none exist
  (`apply_sidebar_action` plays nothing; repair/sell pass state=0 always). Source: rust lane §1.2/§2.4.
- *Are the strip-scroll buttons a conversion or new?* — NEW; no buttons exist (wheel-only).
  They wire to `sidebar_scroll_rows`/`max_scroll_rows`. Source: rust lane §1.3.
- *Does scroll-release IsPressed need the SidebarClass::AI direct clear?* — No; mask 0x55
  includes release bits so the G22 machine itself clears IsPressed on release; the native AI
  write is redundant belt-and-braces. Source: contract lane §2.1-2.2 + G22 row 5.
- *Where does the 0x120 stack really sit today?* — `ensure_active` reset_to-clobbers to depth 1;
  nothing is pushed over 0xE2 (comment is misleading) ⇒ D-B3 must change BOTH open (push) and
  close (pop). Source: rust lane §3.3.
- *Right-click on tabs* — today right-press selects a tab (legacy hit_test ignores
  `right_click` for tabs); gamemd mask 5 ignores right entirely. The flip intentionally fixes
  this (parity-positive change, noted in Risk Areas). Source: sidebar/mod.rs:384-388; contract
  lane §2.1.

### Deferred to Implementation / Follow-up (do NOT invent values)

- **CSF numeric-id → label mapping** for 0x13CD/0x13D3/0x13DB/0x13DD/0x13DF/0x13E1/0xC6E/0xC6C
  /0x13F4/0x29E — tab/scroll tooltip text + retail cameo tooltip format blocked on one mapping
  pass (CSF dump or Ghidra string-table walk). Until then: empty text (no tip) / interim format.
- **Chat-message timeout formula** (DAT_00A8D748 ← DAT_00A8B394; `MessageDelay` binding
  untraced). Beacon literal 225 ticks IS verified.
- **Suppression byte 0x00A8F7D8** (paused ⇒ instant tooltips + tactical coord readout) — not
  implemented; needs writer decompile (0x00537EFA).
- **Tooltip box placement math** (ShowAt 0x00478BA0 undecoded) + fill/border colors — interim
  cursor-offset + darken-fill shipped; fidelity-check follow-up.
- **R-UP/R-DN SHP frame count** — read the SHP headers from retail `sidec01/02.mix` during A1
  verification; loader tolerates either way.
- **Typewriter reveal + per-char `MessageCharTyped`** (UNK §7.8) — out of A5 scope (prompt-fixed);
  offline non-silent messages in gamemd DO typewriter-reveal — listed as known post-A5 gap.
- **TextLabel +0x28 color field** — carried per message in our model; gamemd draw uses the
  scheme index; exact scheme→message mapping deferred (white interim for system messages).
- **SW UIName parse** (superweapon sections) — A4 ships SW tips as `display_name`; localized SW
  UIName needs a `superweapon_type.rs` field (follow-up).
- **`fit_chars` (FUN_00433F50) exact boundary semantics** — word-break back-up implemented from
  the doc-level description; byte-exact boundary cases unverified.
- **GUITabSound consumer mapping** — one Ghidra spot-check on the sidebar Action-ID consumer.
- **Shell floating-tooltip visual** — gamemd draws shell tips as floating boxes; the current
  Rust bottom status-line placement is kept; A4 only fixes the timing (delay/kill/duration).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/ui/gadget/mod.rs` | module decls, event-flag constants, `GadgetHandle`/`ListId`/`GadgetRect` |
| Create | `src/ui/gadget/focus.rs` | `FocusState` (sticky/keyboard/hover/current_list), removal closure, G25 |
| Create | `src/ui/gadget/list.rs` | `Gadget`, `GadgetBehavior`, `GadgetSpec`, `GadgetList` (retained order), keyboard-focus protocol |
| Create | `src/ui/gadget/button.rs` | sticky_process (G17), base/control action (G16/G13), toggle machine (G22/G23) |
| Create | `src/ui/gadget/tick.rs` | `GadgetInput`/`TickOutput`, hit_test (G14), clicked_on (G15), `tick` (G5-G13) |
| Create | `src/app_gadget_input.rs` | in-game gadget driver: build/sync 8 sidebar buttons, event+idle ticks, fired-ID application, pressed-visual publish |
| Create | `src/ui/tooltips.rs` | pure tooltip service model (delay/duration/inclusive rects/kill) |
| Create | `src/app_tooltips.rs` | tooltip driver: wall clock, region sync (sidebar + main menu), box instance builder |
| Create | `src/ui/messages.rs` | pure message-list model (14 slots, wrap, evict, expiry, restack) |
| Create | `src/app_messages.rs` | message driver: system-message entry, insert sound, manage cadence, text instances |
| Modify | `src/ui/mod.rs` | declare `gadget`, `tooltips`, `messages`; (R1) drop `in_game_hud` |
| Modify | `src/lib.rs` | declare the three new flat app modules |
| Modify | `src/app_input.rs` | route mouse edges through the gadget driver first; `apply_sidebar_action`/modifier helpers → pub(crate) |
| Modify | `src/sidebar/mod.rs` | retire tab/repair/sell probes from `hit_test`; add `scroll_button_rects` |
| Modify | `src/sidebar/gadget_flash.rs` | pressed-bit fields + frame accessors (frames 3/4 reachable) |
| Modify | `src/render/sidebar_chrome.rs` | load R-UP/R-DN into the atlas |
| Modify | `src/app_sidebar_build.rs` | draw the scroll pair (5-frame select) |
| Modify | `src/app.rs` | AppState fields + ctor; tooltip event feeds; per-frame updates; D-B3 routing; banner removal |
| Modify | `src/app_sim_tick.rs` | idle gadget tick hook; banner deadline removal |
| Modify | `src/app_transitions.rs` | (A5 T3) drop the two `mission_announcement*` reset lines |
| Modify | `src/ui/mission_status.rs` | (A5 T3) delete `draw_mission_banner` (keep `draw_mission_result_screen`) |
| Modify | `src/app_render/mod.rs` + `src/app_render/draw_passes.rs` | message/tooltip pooled buffers + draw order (chat → tooltip → cursor) |
| Modify | `src/app_main_menu_shell_render.rs` | tooltip emission gated by the service |
| Modify | `src/rules/ruleset.rs` | parse `GUITabSound`, `IncomingMessage`, `MessageDelay` |
| Modify | `src/rules/object_type.rs` | parse `UIName=` |
| Modify | `src/sim/movement/locomotor_tests.rs` | (A4 T1) test-fixture-only: `ui_name: None,` in the full-field `ObjectType` literal |
| Modify | `src/sim/movement/teleport_movement.rs` | (A4 T1) `cfg(test)`-fixture-only: `ui_name: None,` in the full-field `ObjectType` literal |
| Modify | `src/ui/shell/controller.rs` | (tests only) push-over/pop LIFO test |
| Delete | `src/ui/in_game_hud.rs` | R1 dead code |

No file is expected to exceed ~600 lines of non-test code; `ui/gadget/tick.rs` and
`ui/gadget/button.rs` carry large `#[cfg(test)]` blocks (allowed: test-heavy).

## Interface Changes

- **New public (crate) API:** `ui::gadget::{tick, GadgetList, FocusState, GadgetInput,
  TickOutput, GadgetSpec, GadgetRect, GadgetHandle, ListId, ToggleKind, flag constants}`;
  `ui::tooltips::{TooltipService, TipRegion, TipRect, ActiveTip}`;
  `ui::messages::{MessageList, PauseAwareClock}`. Nothing outside this plan consumes them yet
  except the new drivers.
- **`sidebar::hit_test` narrows**: no longer returns `SelectTab`/`ToggleRepairMode`/
  `ToggleSellMode` (substrate-owned). Sole caller `handle_sidebar_mouse_input`
  (app_input.rs:227-238) — verified single call site.
- **`build_sidebar_chrome_instances_for_layout` gains a `scroll_frames: [u8; 2]` param** —
  verified single caller (`build_sidebar_chrome_instances`, app_sidebar_build.rs:40).
- **`SidebarGadgetState` gains pressed-bit fields** — consumed by `frame_select` accessors;
  all existing accessor call sites keep compiling (signatures unchanged).
- **`app_input::apply_sidebar_action` and `is_shift_held`/`is_ctrl_held`/`is_alt_held` become
  `pub(crate)`** for the driver.
- **`main_menu_paint_labels`**: `hovered_button` param becomes `_hovered_button` (tooltip text
  now comes from the service).
- **AppState fields added:** `in_game_gadgets`, `tooltips`, `tooltip_epoch`, `message_list`,
  `message_clock`. Fields removed (A5 T3): `mission_announcement`,
  `mission_announcement_deadline`.

## Sim Checklist

This plan is **UI-layer only — no sim behavior/logic change**:

- [x] No sim behavior/logic change. Exactly two `#[cfg(test)]` fixture literals gain a
  `ui_name: None,` line when Task 16 adds the `ObjectType` field
  (`src/sim/movement/locomotor_tests.rs:12` and `src/sim/movement/teleport_movement.rs:387` —
  both full-field literals, neither uses `..Default`; verified the only two `ObjectType { .. }`
  literal sites in the crate). No non-test line under `src/sim/` is created, modified, or
  deleted by any task.
- [x] No new dependency from `sim/` to ui/render/sidebar/audio/net (none touched).
- [x] Tick ordering in `World::advance_tick` unaffected; no state-hash change; no
  `SNAPSHOT_VERSION` change.
- [x] Player commands continue to enter the sim exclusively through the existing
  `app_commands::*` seam (study O14): the gadget driver maps fired IDs onto the SAME
  `SidebarAction` handling that exists today.
- [x] No `f32`/`f64` enters game logic — the new f32 use is render/UI-side only; the gadget
  core itself is integer-only.

## Risk Areas

- **Every sidebar click of every match** changes dispatch path (fire-on-DOWN → silent press /
  fire-on-RELEASE). Mitigations: clause-level unit tests (A0), per-action manual checklist
  (A1 gate), legacy fall-through preserved for cameos/dev buttons (out-of-scope A2 surfaces).
- **Intentional behavior changes shipped by A1** (all gamemd-correcting, listed for the gate
  check): release-inside firing + drag-off cancel; right-click no longer selects tabs; NEW
  scroll buttons; tab-select click sound (was silent).
- **87-test net** (`ui/skirmish_shell/state/tests.rs`) must stay green — touched by nothing
  here, verified at every slice gate.
- **Pooled-buffer names** (`message_text`, `tooltip_fill`, `tooltip_text`) assume
  `pool.upload` creates buffers on first use like the existing 9 sidebar buffers; if the pool
  requires pre-registration, the A4/A5 render tasks fail visibly at the gate (fix = register
  alongside "sidebar_text").
- **Banner removal (A5 T3)** deletes `mission_announcement` fields — compile errors surface
  every consumer (app_sim_tick.rs:169-174/846-850, app_transitions.rs:166-167, app.rs:2925-2927
  + AppState/ctor); the task enumerates all of them.
- **Parallel sessions:** the worktree is isolated at 7b79a186; if `cargo check` fails in files
  this plan does not touch, STOP — do not fix unrelated code. The ONE scheduled exception:
  Task 16 (the `ObjectType` field) intentionally triggers missing-field errors in the two
  sim `cfg(test)` fixtures named in the File Map — those two `ui_name: None,` additions are
  plan-owned edits, not "unrelated code."

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| T5/T7 | G22 silent press + fire-on-RELEASE-inside + drag-off cancel | Fires on EVERY sidebar click in every match — the #1 DRIFT (study §4.2 D-A1) | State-table unit tests vs contract lane §1.7; in-game press-drag-off-release leaves mode unchanged |
| T6/T7 | G14 half-open rects, smallest-area, signed `<=` tie-break, 786,432 px² seed | Edge-pixel clicks + overlap ordering (study D-A3); boundary behavior differs from BOTH legacy Rust conventions | Boundary unit tests (x+w pixel misses; equal-area later wins; >seed never wins) |
| T7 | G8 held bits idle-only / G6 event-coord source / G10 tier exclusivity | Input-feel substrate for all later slices (A2/A3/A6 build on it) | Unit tests per clause (flag assembly truth table) |
| T12 | Tabs Kind 2 latch-ON; repair/sell Kind 1 flip; scroll mask 0x55 ⇒ one page per click, NO hold-repeat | Exact gamemd Kind/mask identities (VERIFIED-LIVE 0x006A5310) | Unit tests + manual: hold scroll button → exactly one page |
| T12 | Right-release on scroll fires `ID\|0xC000`, consumer masks `~0x4000` ⇒ right-click scrolls identically | gamemd-verified consumer behavior (contract §2.1) | Unit test + manual right-click scroll |
| T9/T12 | Pressed visual = frames 3/4 while held-inside, popped on drag-off | Player sees the press; today repair/sell never show 3/4 | Manual: hold over button shows pressed frame; drag off pops it |
| T13 | Right-click no longer activates tabs | gamemd mask 5 = left-only; current Rust drift fixed | Manual right-click on tab does nothing |
| T14 | Tab click sound = GUITabSound (MenuTab) | gamemd plays a sound on tab select; mapping name-inferred — **needs one Ghidra spot-check of the 0x80CB..CE consumer** | Ghidra: decompile SidebarClass::AI tab arm for the Voc index before sign-off |
| T17 | 1000 ms delay / 10000 ms duration / move-restarts / kill-on-any-button / INCLUSIVE edges | Continuous in normal play (study D-A2/D-B2); inclusive-vs-half-open is a deliberate 1-px observable | Unit tests incl. the boundary pixel that tips accept but gadgets reject |
| T18 | Tab/scroll tips show NOTHING until CSF mapping lands | Conservative stand-in — must not invent text | Deferred list + manual (no tip on tabs) |
| T19 | Tooltip box visuals/placement are INTERIM | gamemd box undecoded (UNK 10) | Fidelity-check follow-up vs gamemd screenshot |
| T20 | Shell tooltip now waits 1000 ms (was immediate) | Every menu hover (study D-B2) | Manual: hover < 1 s shows nothing; kill on click |
| T23 | 14-slot cap, evict-head, tail insert, 19 px restack, silent wrap with prefix re-included | Every system message; wrap visible on long text | Unit tests per step of contract §4.2 |
| T24 | Insert sound = IncomingMessage (MessageText); wrapped lines silent | Audible on every message | Unit test (outcome flag) + manual |
| T24 | Mission text moves from egui banner to gamemd-shaped top-left list | Visual change, parity-positive (TriggerAction__Execute is an Add_Message caller) | Manual: trigger map text renders as list lines |
| T26 | Esc on exit-confirm pops the controller stack (LIFO, focus restore) | Internal consistency + keyboard parity direction (D-B3) | Controller unit test + manual Esc |

---

## Tasks

Slice order and rationale: **A0** (pure substrate, no player-visible change, maximal test
surface) → **A1** (flips the top player-visible DRIFT) → **A4 → A5** (additive GAP fills) →
**D-B3 → R1** (cleanups). Each slice ends with a full `cargo test -p vera20k` gate.
NOTE: the worktree build cache is being warmed by a baseline `cargo check`; the first verify
command may take longer.

### Task 1 (A0 T1): `ui::gadget` skeleton — constants, handles, half-open rect

**Why:** Every later file imports these types; the half-open rect is the G14 foundation.

**Files:**
- Create: `src/ui/gadget/mod.rs`
- Modify: `src/ui/mod.rs`

**Pattern:** module layout mirrors `src/ui/shell/mod.rs` (render-agnostic ui submodule).

**Step 1: declare the module.** In `src/ui/mod.rs`, the module list currently reads:

```rust
pub mod client_theme;
pub mod game_screen;
pub mod in_game_hud;
pub mod main_menu;
```

Insert `pub mod gadget;` between `game_screen` and `in_game_hud`:

```rust
pub mod client_theme;
pub mod game_screen;
pub mod gadget;
pub mod in_game_hud;
pub mod main_menu;
```

**Step 2: create `src/ui/gadget/mod.rs`:**

```rust
//! Framework-A gadget substrate core (study §6.1): retained gadget lists with
//! gamemd-native dispatch semantics — retained order = hit priority = draw
//! order, sticky capture, fire-on-release button machine.
//!
//! Pure and deterministic: NO wall-clock reads anywhere in this module tree;
//! every tick input arrives in a `tick::GadgetInput` snapshot built by the app
//! driver. Clause IDs (G1..G25) cite the behavior contract in
//! GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md §5.
//!
//! ## Dependency rules
//! - ui/ module: std only — no render/, assets/, sidebar/, audio/, net/.

pub mod button;
pub mod focus;
pub mod list;
pub mod tick;

/// Queued left-mouse-down event (G8 source code).
pub const KEY_LMB_DOWN: u16 = 0x001;
/// Queued right-mouse-down event.
pub const KEY_RMB_DOWN: u16 = 0x002;
/// Queued left-mouse-up event (low byte 1 ⇒ event-coordinate source, G6).
pub const KEY_LMB_UP: u16 = 0x801;
/// Queued right-mouse-up event (low byte 2 ⇒ event-coordinate source, G6).
pub const KEY_RMB_UP: u16 = 0x802;

/// Event-flag bits assembled per tick (G8).
pub const FLAG_LEFT_PRESS: u16 = 0x0001;
pub const FLAG_LEFT_HELD: u16 = 0x0002;
pub const FLAG_LEFT_RELEASE: u16 = 0x0004;
pub const FLAG_LEFT_UP: u16 = 0x0008;
pub const FLAG_RIGHT_PRESS: u16 = 0x0010;
pub const FLAG_RIGHT_HELD: u16 = 0x0020;
pub const FLAG_RIGHT_RELEASE: u16 = 0x0040;
pub const FLAG_RIGHT_UP: u16 = 0x0080;
/// Queued non-mouse event yields exactly this flag (G8); doubles as the
/// keyboard-focus mask bit a focused gadget carries (G18).
pub const FLAG_KEYBOARD: u16 = 0x0100;

/// Press bits — the sticky-capture acquire test (G17).
pub const PRESS_BITS: u16 = FLAG_LEFT_PRESS | FLAG_RIGHT_PRESS; // 0x11
/// Release bits — capture release + the G22 strip test.
pub const RELEASE_BITS: u16 = FLAG_LEFT_RELEASE | FLAG_RIGHT_RELEASE; // 0x44

/// Result protocol (G13): a fired control posts `id | RESULT_BUTTON`.
pub const RESULT_BUTTON: u16 = 0x8000;
/// Extra OR'd marker iff a right-release fired AND the mask includes
/// `FLAG_RIGHT_PRESS` (G13).
pub const RESULT_RIGHT: u16 = 0x4000;

/// Hit-test best-area seed: the fixed 1024×768 constants, NOT live resolution
/// (G14). A gadget with area > 786,432 px² can never win a hit-test.
pub const HIT_SEED_AREA: i32 = 1024 * 768;

/// Ctor rule (G4): a sticky gadget always ORs press+left bits into its mask.
pub const STICKY_CTOR_MASK: u16 = 0x0005;

/// Stable per-list gadget identity. Never reused within a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GadgetHandle(pub u32);

/// Caller-assigned list identity; `FocusState.current_list` compares these for
/// the G5 fresh-list reset. The app owns uniqueness (one in-game list today).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListId(pub u32);

/// Integer pixel rect with HALF-OPEN containment (G14): left/top in,
/// right/bottom out — the same convention as `ui::shell::geom::RectPx` and the
/// native unsigned-compare filter; deliberately NOT `sidebar::Rect` (inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GadgetRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl GadgetRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// Half-open containment via the unsigned-compare trick: a negative delta
    /// wraps to a huge u32 and rejects, so no explicit lower-bound test is
    /// needed (G14/G15). Zero width or height never contains anything.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        (px.wrapping_sub(self.x) as u32) < self.w as u32
            && (py.wrapping_sub(self.y) as u32) < self.h as u32
    }

    /// Signed pixel area (G14 does signed i32 math).
    pub fn area(&self) -> i32 {
        self.w * self.h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_half_open_edges() {
        let r = GadgetRect::new(10, 20, 5, 4);
        assert!(r.contains(10, 20), "left/top edge IN");
        assert!(r.contains(14, 23), "last interior pixel IN");
        assert!(!r.contains(15, 20), "right edge OUT (half-open)");
        assert!(!r.contains(10, 24), "bottom edge OUT (half-open)");
        assert!(!r.contains(9, 20), "negative delta rejects via unsigned wrap");
        assert!(!r.contains(10, 19));
    }

    #[test]
    fn zero_size_rect_contains_nothing() {
        let r = GadgetRect::new(0, 0, 0, 0);
        assert!(!r.contains(0, 0));
    }

    #[test]
    fn seed_constant_value() {
        assert_eq!(HIT_SEED_AREA, 786_432, "1024x768 .rdata seed (G14)");
    }
}
```

**Step 3: Verify.** Run in the worktree:
`cargo test -p vera20k ui::gadget -- --nocapture` → the 3 tests pass.

**Step 4: Commit.** `ui: A0 T1 — ui::gadget skeleton (event-flag constants, half-open GadgetRect, handles)`

### Task 2 (A0 T2): `FocusState` — the four globals as one struct

**Why:** Every dispatch path mutates capture/focus/hover; defined before the list so the
list's focus-aware removal can take it (G24 + the study §6.1 hover closure).

**Files:**
- Create: `src/ui/gadget/focus.rs`

**Step 1: create `src/ui/gadget/focus.rs`:**

```rust
//! FocusState — the four native dispatch focus globals (sticky capture,
//! keyboard focus, hover, current list — study G3/G7/G17/G18) as ONE value
//! owned by the app driver (study §6.1). Removal/destruction clears hover too
//! — deliberately closing the G7 stale-pointer hazard; no leave-notification
//! fires for a dead handle.

use super::{GadgetHandle, ListId};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FocusState {
    /// Mouse-capture holder (G17): re-dispatched every tick, even masked-0.
    pub sticky: Option<GadgetHandle>,
    /// Keyboard-focus holder (G18): receives `FLAG_KEYBOARD` events,
    /// bypassing the bounds test.
    pub keyboard: Option<GadgetHandle>,
    /// Hover holder (G7): updated by the tick's pre-dispatch hit-test only.
    pub hovered: Option<GadgetHandle>,
    /// The list last ticked; a mismatch triggers the G5 fresh-list reset.
    pub current_list: Option<ListId>,
}

impl FocusState {
    pub fn new() -> Self {
        Self::default()
    }

    /// G24 + study §6.1 closure: a removed/destroyed gadget releases capture,
    /// keyboard focus AND hover when it holds them. Called by every
    /// `GadgetList` removal path.
    pub fn on_removed(&mut self, handle: GadgetHandle) {
        if self.sticky == Some(handle) {
            self.sticky = None;
        }
        if self.keyboard == Some(handle) {
            self.keyboard = None;
        }
        if self.hovered == Some(handle) {
            self.hovered = None;
        }
    }

    /// G25 — clear the attached list: forget the current list so the next tick
    /// takes the G5 reset path (the sanctioned page-swap mechanism).
    pub fn clear_attached_list(&mut self) {
        self.current_list = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_removed_clears_only_matching_slots() {
        let a = GadgetHandle(1);
        let b = GadgetHandle(2);
        let mut f = FocusState {
            sticky: Some(a),
            keyboard: Some(b),
            hovered: Some(a),
            current_list: Some(ListId(7)),
        };
        f.on_removed(a);
        assert_eq!(f.sticky, None, "capture released (G24)");
        assert_eq!(f.keyboard, Some(b), "other holder untouched");
        assert_eq!(f.hovered, None, "hover cleared too (study §6.1 closure)");
        assert_eq!(f.current_list, Some(ListId(7)), "list identity untouched");
    }

    #[test]
    fn clear_attached_list_only_clears_list() {
        let a = GadgetHandle(1);
        let mut f = FocusState {
            sticky: Some(a),
            keyboard: None,
            hovered: Some(a),
            current_list: Some(ListId(1)),
        };
        f.clear_attached_list();
        assert_eq!(f.current_list, None, "G25 zeroes only current_list");
        assert_eq!(f.sticky, Some(a));
        assert_eq!(f.hovered, Some(a));
    }
}
```

**Step 2: Verify.** `cargo test -p vera20k ui::gadget::focus` → 2 tests pass.

**Step 3: Commit.** `ui: A0 T2 — FocusState (sticky/keyboard/hover/current-list; removal closure)`

### Task 3 (A0 T3): `GadgetList` — retained order, focus-aware mutation, keyboard-focus protocol

**Why:** The retained list IS the contract (one order for hit + draw, G1/G20/O7); its mutation
set must clear focus on removal (G3/G18/G24).

**Files:**
- Create: `src/ui/gadget/list.rs`

**Step 1: create `src/ui/gadget/list.rs`:**

```rust
//! Retained gadget list: insertion order = hit-test priority = draw order
//! (G1/G14/G20/O7). Vec-backed with stable per-list handles; gadgets belong to
//! exactly one list by ownership (the G2 "every insert self-removes first"
//! invariant is structural in Rust — specs are values, handles are per-list).

use super::focus::FocusState;
use super::{FLAG_KEYBOARD, GadgetHandle, GadgetRect, ListId, STICKY_CTOR_MASK};

/// Toggle kind (G22 row 5): 0 = no on-state, 1 = flip, 2 = latch-ON only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleKind {
    #[default]
    Plain,
    Flip,
    LatchOn,
}

/// Per-button toggle state — the G22 pressed/on/kind triple (verified native
/// field identities cited in the plan Sources section).
#[derive(Debug, Clone, Copy, Default)]
pub struct ButtonState {
    pub is_pressed: bool,
    pub is_on: bool,
    pub kind: ToggleKind,
}

/// Which Action implementation a gadget runs (the 3 live behavior shapes the
/// A0/A1 scope needs; cameo/click-region behaviors arrive with A2/A3).
#[derive(Debug, Clone, Copy)]
pub enum GadgetBehavior {
    /// Base action (G16): consume any masked flags.
    Plain,
    /// Control action (G13): post `id|0x8000` then base.
    Control,
    /// Toggle-button action (G22): the silent-press / fire-on-release machine.
    Button(ButtonState),
}

/// One retained gadget.
#[derive(Debug, Clone)]
pub struct Gadget {
    pub handle: GadgetHandle,
    pub rect: GadgetRect,
    /// Event mask (G15 filters raw flags by this FIRST). Bit 0x100 doubles as
    /// the keyboard-focus marker (G18).
    pub flags: u16,
    /// Result-protocol id; 0 posts nothing (G13).
    pub id: u16,
    pub is_sticky: bool,
    pub is_disabled: bool,
    /// Local dirty byte (G19): set by redraw-flag setters, cleared by draw.
    pub is_to_redraw: bool,
    pub behavior: GadgetBehavior,
}

/// Construction spec (the ctor argument set, G4).
#[derive(Debug, Clone, Copy)]
pub struct GadgetSpec {
    pub rect: GadgetRect,
    pub flags: u16,
    pub id: u16,
    pub sticky: bool,
    pub disabled: bool,
    pub behavior: GadgetBehavior,
}

impl GadgetSpec {
    /// G4 — base ctor: geometry + mask + sticky; `sticky ⇒ Flags |= 0x05`;
    /// everything else zeroed.
    pub fn new(rect: GadgetRect, flags: u16, sticky: bool) -> Self {
        let flags = if sticky { flags | STICKY_CTOR_MASK } else { flags };
        Self {
            rect,
            flags,
            id: 0,
            sticky,
            disabled: false,
            behavior: GadgetBehavior::Plain,
        }
    }

    /// Toggle-button ctor defaults (G4): mask 5 (left press+release), sticky.
    /// Callers override the mask afterwards (`with_flags`) for the 0x55
    /// scroll pair.
    pub fn button(rect: GadgetRect, id: u16, kind: ToggleKind) -> Self {
        let mut spec = Self::new(rect, 0x0005, true);
        spec.id = id;
        spec.behavior = GadgetBehavior::Button(ButtonState {
            is_pressed: false,
            is_on: false,
            kind,
        });
        spec
    }

    /// Replace the event mask AFTER ctor defaults (the native sidebar init
    /// writes the 0x55 scroll mask over the ctor's 5 — contract lane §2.1).
    pub fn with_flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self
    }
}

#[derive(Debug)]
pub struct GadgetList {
    id: ListId,
    next_handle: u32,
    gadgets: Vec<Gadget>,
}

impl GadgetList {
    pub fn new(id: ListId) -> Self {
        Self {
            id,
            next_handle: 1,
            gadgets: Vec::new(),
        }
    }

    pub fn list_id(&self) -> ListId {
        self.id
    }

    pub fn len(&self) -> usize {
        self.gadgets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gadgets.is_empty()
    }

    fn alloc(&mut self, spec: GadgetSpec) -> Gadget {
        let handle = GadgetHandle(self.next_handle);
        self.next_handle += 1;
        Gadget {
            handle,
            rect: spec.rect,
            flags: spec.flags,
            id: spec.id,
            is_sticky: spec.sticky,
            is_disabled: spec.disabled,
            is_to_redraw: false,
            behavior: spec.behavior,
        }
    }

    /// G2 — Add_Tail: append (the registration path, O7).
    pub fn add_tail(&mut self, spec: GadgetSpec) -> GadgetHandle {
        let g = self.alloc(spec);
        let h = g.handle;
        self.gadgets.push(g);
        h
    }

    /// G2 — Add_Head: prepend.
    pub fn add_head(&mut self, spec: GadgetSpec) -> GadgetHandle {
        let g = self.alloc(spec);
        let h = g.handle;
        self.gadgets.insert(0, g);
        h
    }

    /// G2 — Add(after): insert immediately after an existing gadget.
    /// Returns None when `after` is not in this list.
    pub fn add_after(&mut self, after: GadgetHandle, spec: GadgetSpec) -> Option<GadgetHandle> {
        let pos = self.gadgets.iter().position(|g| g.handle == after)?;
        let g = self.alloc(spec);
        let h = g.handle;
        self.gadgets.insert(pos + 1, g);
        Some(h)
    }

    /// G3 — Remove: neighbor repair is implicit in Vec removal; clears every
    /// focus slot pointing at the dying gadget (G18/G24 + hover closure).
    pub fn remove(&mut self, handle: GadgetHandle, focus: &mut FocusState) -> bool {
        let Some(pos) = self.gadgets.iter().position(|g| g.handle == handle) else {
            return false;
        };
        self.gadgets.remove(pos);
        focus.on_removed(handle);
        true
    }

    /// Extract_Gadget(id): remove the first gadget carrying a control id.
    pub fn extract_by_id(&mut self, id: u16, focus: &mut FocusState) -> Option<GadgetHandle> {
        let handle = self.gadgets.iter().find(|g| g.id == id)?.handle;
        self.remove(handle, focus);
        Some(handle)
    }

    /// Delete_List: destroy every gadget, clearing focus slots per gadget (G24).
    pub fn clear(&mut self, focus: &mut FocusState) {
        for g in self.gadgets.drain(..) {
            focus.on_removed(g.handle);
        }
    }

    pub fn get(&self, handle: GadgetHandle) -> Option<&Gadget> {
        self.gadgets.iter().find(|g| g.handle == handle)
    }

    pub fn get_mut(&mut self, handle: GadgetHandle) -> Option<&mut Gadget> {
        self.gadgets.iter_mut().find(|g| g.handle == handle)
    }

    /// Handle at a retained-order index (tick walk helper; index < len).
    pub(crate) fn handle_at(&self, idx: usize) -> GadgetHandle {
        self.gadgets[idx].handle
    }

    /// Head→tail iteration in retained order.
    pub fn iter(&self) -> impl Iterator<Item = &Gadget> {
        self.gadgets.iter()
    }
}

/// G18 — focus acquire: steal keyboard focus. Old holder is dirtied and loses its
/// 0x100 mask bit; the new holder gains it.
pub fn set_focus(list: &mut GadgetList, focus: &mut FocusState, handle: GadgetHandle) {
    if let Some(old) = focus.keyboard.take() {
        if let Some(g) = list.get_mut(old) {
            g.is_to_redraw = true;
            g.flags &= !FLAG_KEYBOARD;
        }
    }
    if let Some(g) = list.get_mut(handle) {
        g.flags |= FLAG_KEYBOARD;
        focus.keyboard = Some(handle);
    }
}

/// G18 — focus clear: self-conditional (only the holder clears itself).
pub fn clear_focus(list: &mut GadgetList, focus: &mut FocusState, handle: GadgetHandle) {
    if focus.keyboard == Some(handle) {
        if let Some(g) = list.get_mut(handle) {
            g.is_to_redraw = true;
            g.flags &= !FLAG_KEYBOARD;
        }
        focus.keyboard = None;
    }
}

/// G18/G19 — enable/disable: set the gate, dirty unconditionally, force the
/// G18 focus clear.
pub fn set_enabled(
    list: &mut GadgetList,
    focus: &mut FocusState,
    handle: GadgetHandle,
    enabled: bool,
) {
    clear_focus(list, focus, handle);
    if let Some(g) = list.get_mut(handle) {
        g.is_disabled = !enabled;
        g.is_to_redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> GadgetRect {
        GadgetRect::new(0, 0, 10, 10)
    }

    #[test]
    fn ctor_defaults_g4() {
        let plain = GadgetSpec::new(rect(), 0x40, false);
        assert_eq!(plain.flags, 0x40, "non-sticky keeps the given mask");
        let sticky = GadgetSpec::new(rect(), 0x40, true);
        assert_eq!(sticky.flags, 0x45, "sticky ORs 0x05 into the mask (G4)");
        let btn = GadgetSpec::button(rect(), 0x65, ToggleKind::Flip);
        assert_eq!(btn.flags, 0x0005, "button ctor mask 5 (G4)");
        assert!(btn.sticky, "button ctor sticky (G4)");
        let scroll = GadgetSpec::button(rect(), 0xC9, ToggleKind::Plain).with_flags(0x55);
        assert_eq!(scroll.flags, 0x55, "sidebar init overrides the scroll mask");
    }

    #[test]
    fn retained_order_add_tail_head_after() {
        let mut f = FocusState::new();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::new(rect(), 0, false));
        let b = l.add_tail(GadgetSpec::new(rect(), 0, false));
        let c = l.add_head(GadgetSpec::new(rect(), 0, false));
        let d = l.add_after(a, GadgetSpec::new(rect(), 0, false)).unwrap();
        let order: Vec<GadgetHandle> = l.iter().map(|g| g.handle).collect();
        assert_eq!(order, vec![c, a, d, b], "head, then a, after-a, tail");
        assert!(l.remove(a, &mut f));
        assert!(!l.remove(a, &mut f), "double remove rejected");
        let order: Vec<GadgetHandle> = l.iter().map(|g| g.handle).collect();
        assert_eq!(order, vec![c, d, b]);
    }

    #[test]
    fn add_after_missing_returns_none() {
        let mut l = GadgetList::new(ListId(1));
        assert!(
            l.add_after(GadgetHandle(99), GadgetSpec::new(rect(), 0, false))
                .is_none()
        );
    }

    #[test]
    fn remove_clears_focus_slots_g24() {
        let mut f = FocusState::new();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::new(rect(), 0x05, true));
        f.sticky = Some(a);
        f.hovered = Some(a);
        set_focus(&mut l, &mut f, a);
        assert_eq!(f.keyboard, Some(a));
        l.remove(a, &mut f);
        assert_eq!(f.sticky, None);
        assert_eq!(f.keyboard, None);
        assert_eq!(f.hovered, None, "hover cleared — no Leave on a dead handle");
    }

    #[test]
    fn set_focus_steals_and_moves_keyboard_bit_g18() {
        let mut f = FocusState::new();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::new(rect(), 0, false));
        let b = l.add_tail(GadgetSpec::new(rect(), 0, false));
        set_focus(&mut l, &mut f, a);
        assert_eq!(l.get(a).unwrap().flags & FLAG_KEYBOARD, FLAG_KEYBOARD);
        // Steal.
        l.get_mut(a).unwrap().is_to_redraw = false;
        set_focus(&mut l, &mut f, b);
        assert_eq!(f.keyboard, Some(b));
        let ga = l.get(a).unwrap();
        assert_eq!(ga.flags & FLAG_KEYBOARD, 0, "old holder loses the 0x100 bit");
        assert!(ga.is_to_redraw, "old holder redrawn");
        assert_eq!(l.get(b).unwrap().flags & FLAG_KEYBOARD, FLAG_KEYBOARD);
        // The G18 focus clear is self-conditional.
        clear_focus(&mut l, &mut f, a);
        assert_eq!(f.keyboard, Some(b), "non-holder clear is a no-op");
        clear_focus(&mut l, &mut f, b);
        assert_eq!(f.keyboard, None);
    }

    #[test]
    fn disable_forces_clear_focus_and_dirty_g18_g19() {
        let mut f = FocusState::new();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::new(rect(), 0, false));
        set_focus(&mut l, &mut f, a);
        l.get_mut(a).unwrap().is_to_redraw = false;
        set_enabled(&mut l, &mut f, a, false);
        assert_eq!(f.keyboard, None);
        let g = l.get(a).unwrap();
        assert!(g.is_disabled);
        assert!(g.is_to_redraw, "Enable/Disable dirty unconditionally");
    }

    #[test]
    fn extract_by_id_and_clear() {
        let mut f = FocusState::new();
        let mut l = GadgetList::new(ListId(1));
        let mut spec = GadgetSpec::new(rect(), 0, false);
        spec.id = 0x65;
        let a = l.add_tail(spec);
        l.add_tail(GadgetSpec::new(rect(), 0, false));
        assert_eq!(l.extract_by_id(0x65, &mut f), Some(a));
        assert_eq!(l.extract_by_id(0x65, &mut f), None);
        f.sticky = l.iter().next().map(|g| g.handle);
        l.clear(&mut f);
        assert!(l.is_empty());
        assert_eq!(f.sticky, None, "clear releases per-gadget focus slots");
    }
}
```

**Step 2: Verify.** `cargo test -p vera20k ui::gadget::list` → 7 tests pass.

**Step 3: Commit.** `ui: A0 T3 — GadgetList retained order + focus-aware remove/clear + keyboard-focus protocol`

### Task 4 (A0 T4): action chain — sticky capture, base/control action (G17/G16/G13)

**Why:** The G22 machine (next task) tail-calls these; G13's result protocol is what the A1
driver consumes.

**Files:**
- Create: `src/ui/gadget/button.rs`

**Step 1: create `src/ui/gadget/button.rs`** (the toggle machine body lands in Task 5; this
task creates the file with everything EXCEPT `toggle_action`, plus a temporary `Button`
routing into `control_action` that Task 5 replaces):

```rust
//! Per-gadget Action implementations: sticky capture protocol (G17), base
//! consume (G16), control result posting (G13), and the toggle-button
//! machine (G22/G23). Pure functions over `Gadget` + `FocusState`; the tick
//! (tick.rs) routes into `dispatch_action` after G15 filtering.

use super::focus::FocusState;
use super::list::{Gadget, GadgetBehavior};
use super::{FLAG_RIGHT_PRESS, FLAG_RIGHT_RELEASE, PRESS_BITS, RELEASE_BITS, RESULT_BUTTON, RESULT_RIGHT};

/// G17 — sticky capture protocol: press bits acquire capture iff the gadget is sticky;
/// release bits release holder-only (an acquire+release in one call both run).
pub(crate) fn sticky_process(g: &Gadget, masked: u16, focus: &mut FocusState) {
    if g.is_sticky && (masked & PRESS_BITS) != 0 {
        focus.sticky = Some(g.handle);
    } else if focus.sticky != Some(g.handle) {
        return;
    }
    if (masked & RELEASE_BITS) != 0 {
        focus.sticky = None;
    }
}

/// G16 — base Action: masked-0 consumes nothing; anything else dirties the
/// gadget, runs the capture protocol, and consumes.
pub(crate) fn base_action(g: &mut Gadget, masked: u16, focus: &mut FocusState) -> u32 {
    if masked == 0 {
        return 0;
    }
    g.is_to_redraw = true;
    sticky_process(g, masked, focus);
    1
}

/// G13 — control action: post `id|0x8000` (`|0x4000` iff a right-release
/// fired AND the gadget's mask includes right-press), then chain to base.
/// (The live in-game population has no peer links — peer callbacks are not
/// modeled.)
pub(crate) fn control_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    focus: &mut FocusState,
) -> u32 {
    if masked != 0 {
        *key = if g.id == 0 { 0 } else { g.id | RESULT_BUTTON };
        if (masked & FLAG_RIGHT_RELEASE) != 0 && (g.flags & FLAG_RIGHT_PRESS) != 0 {
            *key = g.id | RESULT_BUTTON | RESULT_RIGHT;
        }
    }
    base_action(g, masked, focus)
}

/// Behavior router called by `clicked_on` after G15 masking. `live` is the
/// LIVE cursor position (G22's inside-test source — never the event coords).
pub(crate) fn dispatch_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    live: (i32, i32),
    focus: &mut FocusState,
) -> u32 {
    match g.behavior {
        GadgetBehavior::Plain => base_action(g, masked, focus),
        GadgetBehavior::Control => control_action(g, masked, key, focus),
        GadgetBehavior::Button(_) => toggle_action(g, masked, key, live, focus),
    }
}

/// Placeholder routing until Task 5 lands the G22 machine.
fn toggle_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    _live: (i32, i32),
    focus: &mut FocusState,
) -> u32 {
    control_action(g, masked, key, focus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::gadget::list::{GadgetList, GadgetSpec};
    use crate::ui::gadget::{GadgetRect, ListId};

    fn one_gadget(spec: GadgetSpec) -> (GadgetList, crate::ui::gadget::GadgetHandle) {
        let mut l = GadgetList::new(ListId(1));
        let h = l.add_tail(spec);
        (l, h)
    }

    #[test]
    fn sticky_acquire_and_holder_only_release_g17() {
        let mut f = FocusState::new();
        let (mut l, a) = one_gadget(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true));
        let (mut l2, b) = one_gadget(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true));
        // Press acquires.
        sticky_process(l.get(a).unwrap(), 0x01, &mut f);
        assert_eq!(f.sticky, Some(a));
        // A non-holder's release does NOT release.
        sticky_process(l2.get(b).unwrap(), 0x04, &mut f);
        assert_eq!(f.sticky, Some(a), "holder-only release");
        // Holder's release releases.
        sticky_process(l.get(a).unwrap(), 0x04, &mut f);
        assert_eq!(f.sticky, None);
        // Press+release in one call acquires then releases.
        sticky_process(l.get(a).unwrap(), 0x05, &mut f);
        assert_eq!(f.sticky, None);
        // Non-sticky gadget never acquires.
        let (l3, c) = one_gadget(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, false));
        sticky_process(l3.get(c).unwrap(), 0x01, &mut f);
        assert_eq!(f.sticky, None);
        let _ = (l2.len(), c);
    }

    #[test]
    fn base_action_g16() {
        let mut f = FocusState::new();
        let (mut l, a) = one_gadget(GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0xFF, true));
        let g = l.get_mut(a).unwrap();
        assert_eq!(base_action(g, 0, &mut f), 0, "masked-0 consumes nothing");
        assert!(!g.is_to_redraw);
        assert_eq!(base_action(g, 0x08, &mut f), 1, "any masked bits consume");
        assert!(g.is_to_redraw);
    }

    #[test]
    fn control_action_result_protocol_g13() {
        let mut f = FocusState::new();
        // Mask includes right press (0x10) → right-release posts |0x4000.
        let mut spec = GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x55, true);
        spec.id = 0xC9;
        spec.behavior = GadgetBehavior::Control;
        let (mut l, a) = one_gadget(spec);
        let mut key: u16 = 0;
        let g = l.get_mut(a).unwrap();
        assert_eq!(control_action(g, 0x04, &mut key, &mut f), 1);
        assert_eq!(key, 0xC9 | 0x8000, "left release posts ID|0x8000");
        key = 0;
        assert_eq!(control_action(g, 0x40, &mut key, &mut f), 1);
        assert_eq!(key, 0xC9 | 0xC000, "right release + masked 0x10 posts ID|0xC000");
        // Mask WITHOUT right press: right-release does not add 0x4000.
        let mut spec2 = GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true);
        spec2.id = 0x65;
        spec2.behavior = GadgetBehavior::Control;
        let (mut l2, b) = one_gadget(spec2);
        key = 0;
        assert_eq!(control_action(l2.get_mut(b).unwrap(), 0x40, &mut key, &mut f), 1);
        assert_eq!(key, 0x65 | 0x8000);
        // id 0 posts 0 on the plain branch.
        let mut spec3 = GadgetSpec::new(GadgetRect::new(0, 0, 4, 4), 0x45, true);
        spec3.behavior = GadgetBehavior::Control;
        let (mut l3, c) = one_gadget(spec3);
        key = 0x1234;
        assert_eq!(control_action(l3.get_mut(c).unwrap(), 0x04, &mut key, &mut f), 1);
        assert_eq!(key, 0, "ID==0 posts 0");
        // masked-0 leaves the key untouched and consumes nothing.
        key = 0x1234;
        assert_eq!(control_action(l3.get_mut(c).unwrap(), 0, &mut key, &mut f), 0);
        assert_eq!(key, 0x1234);
    }
}
```

**Step 2: Verify.** `cargo test -p vera20k ui::gadget::button` → 3 tests pass.

**Step 3: Commit.** `ui: A0 T4 — base/control action + sticky capture (G13/G16/G17)`

### Task 5 (A0 T5): the G22 toggle machine + G23

**Why:** The fire-on-RELEASE machine is the top player-visible DRIFT fix; encoded as the
7-row state table from contract lane §1.7.

**Files:**
- Modify: `src/ui/gadget/button.rs`

**Step 1: replace the placeholder.** Delete this block from `src/ui/gadget/button.rs`:

```rust
/// Placeholder routing until Task 5 lands the G22 machine.
fn toggle_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    _live: (i32, i32),
    focus: &mut FocusState,
) -> u32 {
    control_action(g, masked, key, focus)
}
```

and insert in its place:

```rust
/// G22 — the toggle-button machine as the verified 7-row state table.
/// Preliminaries run on EVERY call in this order: (1) inside-test against the
/// LIVE cursor (half-open), (2) masked-0 hover-track (reachable only as the
/// sticky holder, G15), (3) capture acquire/release. Then the press / release
/// rows. Hold-repeat (G23) is purely the mask property: held bits fall
/// through to the tail control_action every tick — no timer, no delay.
fn toggle_action(
    g: &mut Gadget,
    masked: u16,
    key: &mut u16,
    live: (i32, i32),
    focus: &mut FocusState,
) -> u32 {
    let GadgetBehavior::Button(mut b) = g.behavior else {
        // dispatch_action only routes Button behaviors here.
        return 0;
    };
    // Step 1 — LIVE mouse inside-test, never the queued event coords.
    let inside = g.rect.contains(live.0, live.1);
    // Step 2 — rows 2/3: masked-0 sticky re-dispatch pops/restores is_pressed
    // (this is what cancels on drag-off and re-arms on drag-back).
    if masked == 0 {
        if inside && !b.is_pressed {
            b.is_pressed = true;
            g.is_to_redraw = true;
        } else if !inside && b.is_pressed {
            b.is_pressed = false;
            g.is_to_redraw = true;
        }
    }
    // Step 3 — capture protocol BEFORE the branch rows.
    sticky_process(g, masked, focus);

    // Row 1 — press: silent consume. Press bits are stripped from the tail
    // call (no ID posts unless other bits remain), then the key is FORCED to
    // 0 and the event is consumed.
    if (masked & PRESS_BITS) != 0 {
        b.is_pressed = true;
        g.is_to_redraw = true;
        g.behavior = GadgetBehavior::Button(b);
        control_action(g, masked & !PRESS_BITS, key, focus);
        *key = 0;
        return 1;
    }

    let mut tail_flags = masked;
    if (masked & RELEASE_BITS) != 0 {
        if b.is_pressed {
            // Rows 5/6 — release while pressed: toggle per Kind iff the LIVE
            // cursor is inside; release bits are KEPT so the tail posts
            // ID|0x8000 (G13). Release-outside still fires — reachable only
            // in the no-intervening-idle-tick boundary case (row 2 would have
            // popped is_pressed first).
            if inside {
                match b.kind {
                    ToggleKind::Flip => b.is_on = !b.is_on,
                    ToggleKind::LatchOn => b.is_on = true,
                    ToggleKind::Plain => {}
                }
            }
            b.is_pressed = false;
            g.is_to_redraw = true;
        } else {
            // Row 4 — release while NOT pressed (the drag-off cancel
            // outcome): strip the release bits; the tail fires nothing
            // unless other masked bits remain.
            tail_flags &= !RELEASE_BITS;
        }
    }
    g.behavior = GadgetBehavior::Button(b);
    // Row 7 — held bits (when masked in) reach the tail every tick (G23).
    control_action(g, tail_flags, key, focus)
}
```

Also extend the `use super::...` line at the top of the file to include `ToggleKind`:

```rust
use super::list::{Gadget, GadgetBehavior, ToggleKind};
```

(replacing the existing `use super::list::{Gadget, GadgetBehavior};`).

**Step 2: append G22/G23 tests** inside the existing `#[cfg(test)] mod tests` block of
`button.rs` (after `control_action_result_protocol_g13`):

```rust
    fn button(id: u16, kind: ToggleKind, flags: u16) -> (GadgetList, crate::ui::gadget::GadgetHandle) {
        let spec = GadgetSpec::button(GadgetRect::new(0, 0, 10, 10), id, kind).with_flags(flags);
        one_gadget(spec)
    }

    const INSIDE: (i32, i32) = (5, 5);
    const OUTSIDE: (i32, i32) = (50, 50);

    #[test]
    fn g22_row1_silent_press_captures_and_consumes() {
        let mut f = FocusState::new();
        let (mut l, a) = button(0x65, ToggleKind::Flip, 0x05);
        let mut key: u16 = 0x001;
        let r = dispatch_action(l.get_mut(a).unwrap(), 0x01, &mut key, INSIDE, &mut f);
        assert_eq!(r, 1, "press consumed");
        assert_eq!(key, 0, "silent press forces *key = 0");
        assert_eq!(f.sticky, Some(a), "capture acquired");
        let GadgetBehavior::Button(b) = l.get(a).unwrap().behavior else { panic!() };
        assert!(b.is_pressed);
        assert!(!b.is_on, "press never toggles");
    }

    #[test]
    fn g22_rows2_3_masked0_hover_tracking() {
        let mut f = FocusState::new();
        let (mut l, a) = button(0x65, ToggleKind::Flip, 0x05);
        let mut key: u16 = 0;
        dispatch_action(l.get_mut(a).unwrap(), 0x01, &mut key, INSIDE, &mut f);
        // Drag off: masked-0 re-dispatch with cursor outside pops is_pressed.
        dispatch_action(l.get_mut(a).unwrap(), 0, &mut key, OUTSIDE, &mut f);
        let GadgetBehavior::Button(b) = l.get(a).unwrap().behavior else { panic!() };
        assert!(!b.is_pressed, "row 2: pop-out");
        // Drag back: pops back in.
        dispatch_action(l.get_mut(a).unwrap(), 0, &mut key, INSIDE, &mut f);
        let GadgetBehavior::Button(b) = l.get(a).unwrap().behavior else { panic!() };
        assert!(b.is_pressed, "row 3: pop back in");
    }

    #[test]
    fn g22_row5_release_inside_fires_and_toggles_by_kind() {
        for (kind, expect_on_after_1, expect_on_after_2) in [
            (ToggleKind::Flip, true, false),
            (ToggleKind::LatchOn, true, true),
            (ToggleKind::Plain, false, false),
        ] {
            let mut f = FocusState::new();
            let (mut l, a) = button(0xCB, kind, 0x05);
            for (click, expect_on) in [(1, expect_on_after_1), (2, expect_on_after_2)] {
                let mut key: u16 = 0;
                dispatch_action(l.get_mut(a).unwrap(), 0x01, &mut key, INSIDE, &mut f);
                key = 0x801;
                let r = dispatch_action(l.get_mut(a).unwrap(), 0x04, &mut key, INSIDE, &mut f);
                assert_eq!(r, 1);
                assert_eq!(key, 0xCB | 0x8000, "fire on release-inside (click {click})");
                assert_eq!(f.sticky, None, "capture released");
                let GadgetBehavior::Button(b) = l.get(a).unwrap().behavior else { panic!() };
                assert!(!b.is_pressed);
                assert_eq!(b.is_on, expect_on, "kind {kind:?} click {click}");
            }
        }
    }

    #[test]
    fn g22_row4_drag_off_cancels() {
        let mut f = FocusState::new();
        let (mut l, a) = button(0x66, ToggleKind::Flip, 0x05);
        let mut key: u16 = 0;
        dispatch_action(l.get_mut(a).unwrap(), 0x01, &mut key, INSIDE, &mut f);
        // Intervening idle tick with the cursor outside pops is_pressed (row 2).
        dispatch_action(l.get_mut(a).unwrap(), 0, &mut key, OUTSIDE, &mut f);
        // Release (sticky re-dispatch): row 4 strips the release bits.
        key = 0x801;
        let r = dispatch_action(l.get_mut(a).unwrap(), 0x04, &mut key, OUTSIDE, &mut f);
        assert_eq!(r, 0, "nothing fires");
        assert_eq!(key, 0x801, "key untouched — no result posted");
        assert_eq!(f.sticky, None, "capture still released by step 3");
        let GadgetBehavior::Button(b) = l.get(a).unwrap().behavior else { panic!() };
        assert!(!b.is_on, "drag-off cancelled the toggle");
    }

    #[test]
    fn g22_row6_release_outside_no_idle_tick_still_fires() {
        // Boundary case: press then release with NO intervening masked-0 tick.
        let mut f = FocusState::new();
        let (mut l, a) = button(0x65, ToggleKind::Flip, 0x05);
        let mut key: u16 = 0;
        dispatch_action(l.get_mut(a).unwrap(), 0x01, &mut key, INSIDE, &mut f);
        key = 0x801;
        let r = dispatch_action(l.get_mut(a).unwrap(), 0x04, &mut key, OUTSIDE, &mut f);
        assert_eq!(r, 1, "release bits NOT stripped — still fires");
        assert_eq!(key, 0x65 | 0x8000);
        let GadgetBehavior::Button(b) = l.get(a).unwrap().behavior else { panic!() };
        assert!(!b.is_on, "but no toggle (cursor outside)");
    }

    #[test]
    fn g23_hold_repeat_is_mask_property_only() {
        // Mask WITH a held bit (0x2): every held tick posts the ID again.
        let mut f = FocusState::new();
        let (mut l, a) = button(0x77, ToggleKind::Plain, 0x05 | 0x02);
        let mut key: u16 = 0;
        dispatch_action(l.get_mut(a).unwrap(), 0x01, &mut key, INSIDE, &mut f);
        for _ in 0..3 {
            key = 0;
            let r = dispatch_action(l.get_mut(a).unwrap(), 0x02, &mut key, INSIDE, &mut f);
            assert_eq!(r, 1);
            assert_eq!(key, 0x77 | 0x8000, "held bit repeats the ID every tick");
        }
        // Mask WITHOUT held bits (the 0x55 scroll mask): held ticks mask to 0.
        let mut f2 = FocusState::new();
        let (mut l2, b) = button(0xC9, ToggleKind::Plain, 0x55);
        let mut key2: u16 = 0;
        dispatch_action(l2.get_mut(b).unwrap(), 0x01, &mut key2, INSIDE, &mut f2);
        key2 = 0;
        let masked = 0x02u16 & 0x55; // what clicked_on would mask
        assert_eq!(masked, 0, "0x55 has no held bits ⇒ masked-0 re-dispatch only");
        let r = dispatch_action(l2.get_mut(b).unwrap(), masked, &mut key2, INSIDE, &mut f2);
        assert_eq!(r, 0);
        assert_eq!(key2, 0, "no repeat for the scroll mask");
    }

    #[test]
    fn g22_right_release_on_scroll_mask_posts_c000() {
        let mut f = FocusState::new();
        let (mut l, a) = button(0xC9, ToggleKind::Plain, 0x55);
        let mut key: u16 = 0;
        dispatch_action(l.get_mut(a).unwrap(), 0x10, &mut key, INSIDE, &mut f);
        assert_eq!(key, 0, "right press also silent");
        key = 0x802;
        dispatch_action(l.get_mut(a).unwrap(), 0x40, &mut key, INSIDE, &mut f);
        assert_eq!(key, 0xC9 | 0xC000, "right-release posts ID|0xC000 (mask has 0x10)");
    }
```

**Step 3: Verify.** `cargo test -p vera20k ui::gadget::button` → 10 tests pass.

**Step 4: Commit.** `ui: A0 T5 — toggle button machine (G22 state table, G23 mask-property repeat)`

### Task 6 (A0 T6): tick input/output types, hit-test (G14), per-gadget filter (G15)

**Why:** Interfaces before the tick; G14's tie-break and seed are parity-critical boundary
behavior.

**Files:**
- Create: `src/ui/gadget/tick.rs`

**Step 1: create `src/ui/gadget/tick.rs`** (the `tick` fn itself lands in Task 7):

```rust
//! The per-tick dispatch authority: hit-test (G14), per-gadget filter (G15),
//! and the three-tier event tick (G5-G13). Deterministic: all inputs arrive in
//! `GadgetInput`; NO wall-clock access.

use super::button::dispatch_action;
use super::focus::FocusState;
use super::list::GadgetList;
use super::{
    FLAG_KEYBOARD, FLAG_LEFT_HELD, FLAG_LEFT_PRESS, FLAG_LEFT_RELEASE, FLAG_LEFT_UP,
    FLAG_RIGHT_HELD, FLAG_RIGHT_PRESS, FLAG_RIGHT_RELEASE, FLAG_RIGHT_UP, GadgetHandle,
    HIT_SEED_AREA, KEY_LMB_DOWN, KEY_LMB_UP, KEY_RMB_DOWN, KEY_RMB_UP,
};

/// One tick's input snapshot, built by the app driver.
#[derive(Debug, Clone, Copy, Default)]
pub struct GadgetInput {
    /// Queued event: 0 = idle tick; KEY_* mouse codes; any other non-zero
    /// value = keyboard event (G8).
    pub queued_key: u16,
    /// Coordinates latched when the event was queued (G6: used iff the key's
    /// low byte is 1 or 2).
    pub event_x: i32,
    pub event_y: i32,
    /// Live cursor position (G6 idle/keyboard source; G22 inside-test source).
    pub mouse_x: i32,
    pub mouse_y: i32,
    /// Live button state (G8 held bits — idle ticks only).
    pub left_held: bool,
    pub right_held: bool,
    /// Modifier keys, polled fresh (G9).
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// One emitted paint (G11/G12/G19 cadence record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawCmd {
    pub handle: GadgetHandle,
    pub forced: bool,
}

/// One Action dispatch (post-G15-masking) — test/observability record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchRecord {
    pub handle: GadgetHandle,
    pub masked_flags: u16,
    /// G9 modifier word: SHIFT=1 | CTRL=2 | ALT=4 (0 outside the broadcast tier).
    pub modifier: u8,
}

/// Tick results. Owned by the driver and reused across ticks (buffers are
/// cleared, not reallocated — no per-frame allocation in the input path).
#[derive(Debug, Default)]
pub struct TickOutput {
    pub draws: Vec<DrawCmd>,
    pub dispatches: Vec<DispatchRecord>,
    /// Hover transition this tick (G7): old gadget left / new gadget entered.
    pub hover_left: Option<GadgetHandle>,
    pub hover_entered: Option<GadgetHandle>,
    /// The broadcast-walk consumer (None for sticky/keyboard-tier ticks).
    pub consumed_by: Option<GadgetHandle>,
}

impl TickOutput {
    pub fn clear(&mut self) {
        self.draws.clear();
        self.dispatches.clear();
        self.hover_left = None;
        self.hover_entered = None;
        self.consumed_by = None;
    }
}

/// G14 — hit test: forward walk, skip disabled, HALF-OPEN rects, smallest
/// area wins with a signed `<=` tie-break (equal area ⇒ the LATER gadget
/// wins), seeded with the fixed 786,432 px² constant — a gadget larger than
/// the seed can never win.
pub fn hit_test(list: &GadgetList, mx: i32, my: i32) -> Option<GadgetHandle> {
    let mut best: Option<GadgetHandle> = None;
    let mut best_area: i32 = HIT_SEED_AREA;
    for g in list.iter() {
        if g.is_disabled || !g.rect.contains(mx, my) {
            continue;
        }
        let area = g.rect.area();
        if area <= best_area {
            best = Some(g.handle);
            best_area = area;
        }
    }
    best
}

/// G15 — clicked-on filter: mask FIRST, then dispatch iff the gadget is the sticky
/// holder (even masked-0), OR masked flags contain the keyboard bit (bounds
/// bypassed), OR masked flags are non-zero AND the point is inside the
/// half-open rect. Returns the Action result (non-zero = consumed).
#[allow(clippy::too_many_arguments)]
pub(crate) fn clicked_on(
    list: &mut GadgetList,
    handle: GadgetHandle,
    key: &mut u16,
    raw_flags: u16,
    x: i32,
    y: i32,
    modifier: u8,
    live: (i32, i32),
    focus: &mut FocusState,
    out: &mut TickOutput,
) -> u32 {
    let is_sticky_holder = focus.sticky == Some(handle);
    let Some(g) = list.get_mut(handle) else {
        return 0;
    };
    let masked = raw_flags & g.flags;
    if !is_sticky_holder
        && (masked & FLAG_KEYBOARD) == 0
        && (masked == 0 || !g.rect.contains(x, y))
    {
        return 0;
    }
    out.dispatches.push(DispatchRecord {
        handle,
        masked_flags: masked,
        modifier,
    });
    dispatch_action(g, masked, key, live, focus)
}

/// G19 — draw(forced): paints iff forced or dirty, then clears the dirty
/// byte; the paint is emitted as a `DrawCmd`.
pub(crate) fn draw_one(
    list: &mut GadgetList,
    handle: GadgetHandle,
    forced: bool,
    out: &mut TickOutput,
) {
    if let Some(g) = list.get_mut(handle) {
        if forced || g.is_to_redraw {
            g.is_to_redraw = false;
            out.draws.push(DrawCmd { handle, forced });
        }
    }
}
```

**Step 2: add the hit-test / filter tests** at the bottom of `tick.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::gadget::list::{GadgetBehavior, GadgetSpec};
    use crate::ui::gadget::{GadgetRect, ListId};

    fn spec(rect: GadgetRect, flags: u16) -> GadgetSpec {
        GadgetSpec::new(rect, flags, false)
    }

    #[test]
    fn g14_smallest_area_wins() {
        let mut l = GadgetList::new(ListId(1));
        let big = l.add_tail(spec(GadgetRect::new(0, 0, 100, 100), 0xFF));
        let small = l.add_tail(spec(GadgetRect::new(10, 10, 20, 20), 0xFF));
        assert_eq!(hit_test(&l, 15, 15), Some(small), "smaller wins inside both");
        assert_eq!(hit_test(&l, 90, 90), Some(big), "only the big one contains");
        assert_eq!(hit_test(&l, 200, 200), None);
    }

    #[test]
    fn g14_equal_area_later_wins() {
        let mut l = GadgetList::new(ListId(1));
        let _first = l.add_tail(spec(GadgetRect::new(0, 0, 20, 20), 0xFF));
        let second = l.add_tail(spec(GadgetRect::new(0, 0, 20, 20), 0xFF));
        assert_eq!(
            hit_test(&l, 5, 5),
            Some(second),
            "signed <= tie-break: later-in-list wins on equal area"
        );
    }

    #[test]
    fn g14_disabled_invisible_and_seed_caps_area() {
        let mut l = GadgetList::new(ListId(1));
        let mut d = spec(GadgetRect::new(0, 0, 10, 10), 0xFF);
        d.disabled = true;
        l.add_tail(d);
        assert_eq!(hit_test(&l, 5, 5), None, "disabled gadgets are invisible");
        // Area 1024*768 ties the seed (signed <=) and CAN win; one px more cannot.
        let mut l2 = GadgetList::new(ListId(2));
        let exact = l2.add_tail(spec(GadgetRect::new(0, 0, 1024, 768), 0xFF));
        assert_eq!(hit_test(&l2, 5, 5), Some(exact), "area == seed ties via <=");
        let mut l3 = GadgetList::new(ListId(3));
        l3.add_tail(spec(GadgetRect::new(0, 0, 1024, 769), 0xFF));
        assert_eq!(hit_test(&l3, 5, 5), None, "area > 786,432 can never win");
    }

    #[test]
    fn g14_half_open_boundary() {
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(spec(GadgetRect::new(10, 10, 5, 5), 0xFF));
        assert_eq!(hit_test(&l, 10, 10), Some(a));
        assert_eq!(hit_test(&l, 14, 14), Some(a));
        assert_eq!(hit_test(&l, 15, 10), None, "right edge out");
        assert_eq!(hit_test(&l, 10, 15), None, "bottom edge out");
    }

    #[test]
    fn g15_mask_first_filter() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let mut s = spec(GadgetRect::new(0, 0, 10, 10), 0x05);
        s.id = 0x65;
        s.behavior = GadgetBehavior::Control;
        let a = l.add_tail(s);
        let mut key: u16 = 0;
        // Raw flags 0x40 (right release) mask to 0 against 0x05 → early-out
        // even though the point is inside.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, 0x40, 5, 5, 0, (5, 5), &mut f, &mut out),
            0
        );
        assert!(out.dispatches.is_empty(), "filtered before dispatch");
        // Masked non-zero but outside the rect → early-out.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, 0x01, 50, 50, 0, (50, 50), &mut f, &mut out),
            0
        );
        // Masked non-zero and inside → dispatches.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, 0x01, 5, 5, 0, (5, 5), &mut f, &mut out),
            1
        );
        assert_eq!(out.dispatches.len(), 1);
        assert_eq!(out.dispatches[0].masked_flags, 0x01);
    }

    #[test]
    fn g15_sticky_holder_bypasses_even_masked_0() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::button(
            GadgetRect::new(0, 0, 10, 10),
            0x65,
            crate::ui::gadget::list::ToggleKind::Flip,
        ));
        f.sticky = Some(a);
        let mut key: u16 = 0;
        // Raw held flags mask to 0 against mask 5; the holder is dispatched
        // anyway (masked-0 hover-track path).
        clicked_on(&mut l, a, &mut key, 0x82, 50, 50, 0, (50, 50), &mut f, &mut out);
        assert_eq!(out.dispatches.len(), 1);
        assert_eq!(out.dispatches[0].masked_flags, 0);
    }

    #[test]
    fn g15_keyboard_flag_bypasses_bounds() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let mut s = spec(GadgetRect::new(0, 0, 10, 10), 0x05 | FLAG_KEYBOARD);
        s.id = 0x42;
        s.behavior = GadgetBehavior::Control;
        let a = l.add_tail(s);
        let mut key: u16 = 0;
        // Keyboard flag, point far outside → still dispatches.
        assert_eq!(
            clicked_on(&mut l, a, &mut key, FLAG_KEYBOARD, 500, 500, 0, (500, 500), &mut f, &mut out),
            1
        );
        assert_eq!(key, 0x42 | 0x8000);
    }

    #[test]
    fn draw_one_dirty_gate_g19() {
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(spec(GadgetRect::new(0, 0, 10, 10), 0));
        draw_one(&mut l, a, false, &mut out);
        assert!(out.draws.is_empty(), "clean + unforced = no paint");
        l.get_mut(a).unwrap().is_to_redraw = true;
        draw_one(&mut l, a, false, &mut out);
        assert_eq!(out.draws.len(), 1);
        assert!(!l.get(a).unwrap().is_to_redraw, "paint clears the dirty byte");
        draw_one(&mut l, a, true, &mut out);
        assert_eq!(out.draws.len(), 2, "forced always paints");
    }
}
```

**Step 3: Verify.** `cargo test -p vera20k ui::gadget::tick` → 8 tests pass.

**Step 4: Commit.** `ui: A0 T6 — hit-test (G14) + per-gadget filter (G15) + tick input/output types`

### Task 7 (A0 T7): the event-flag tick (G5–G13) + clause integration tests

**Why:** Completes the A0 substrate: fresh-list reset, coordinate source, flag assembly,
modifier word, tier precedence, draw cadence, result protocol — each with a direct test.

**Files:**
- Modify: `src/ui/gadget/tick.rs`

**Step 1: insert the `tick` function** in `src/ui/gadget/tick.rs`, immediately after the
`draw_one` function and before the `#[cfg(test)]` block:

```rust
/// One Input tick on a list (the G5-G13 dispatch authority). Returns the
/// 16-bit key, possibly rewritten to `ID|0x8000[|0x4000]` by a fired control
/// (G13) or forced to 0 by a silent press (G22 row 1).
pub fn tick(
    list: &mut GadgetList,
    focus: &mut FocusState,
    input: &GadgetInput,
    out: &mut TickOutput,
) -> u16 {
    out.clear();

    // G5 — fresh-list reset: a different list than last tick clears capture
    // and keyboard focus and force-draws every gadget this tick.
    let list_changed = focus.current_list != Some(list.list_id());
    if list_changed {
        focus.sticky = None;
        focus.keyboard = None;
        focus.current_list = Some(list.list_id());
    }

    let mut key: u16 = input.queued_key;

    // G6 — coordinate source: mouse-button events (low byte 1/2 — covers
    // 0x001/0x002/0x801/0x802) use the latched event coords; keyboard events
    // and idle ticks use the live cursor.
    let (x, y) = if matches!(key & 0xFF, 1 | 2) {
        (input.event_x, input.event_y)
    } else {
        (input.mouse_x, input.mouse_y)
    };

    // G7 — hover transitions run BEFORE dispatch, every tick.
    let hit = hit_test(list, x, y);
    if hit != focus.hovered {
        out.hover_left = focus.hovered;
        out.hover_entered = hit;
        focus.hovered = hit;
    }

    // G8 — flag assembly: event bits from the queued key; held/up bits ONLY
    // on idle ticks; a queued non-mouse event yields exactly FLAG_KEYBOARD.
    let mut flags: u16 = match key {
        0 => 0,
        KEY_LMB_DOWN => FLAG_LEFT_PRESS,
        KEY_RMB_DOWN => FLAG_RIGHT_PRESS,
        KEY_LMB_UP => FLAG_LEFT_RELEASE,
        KEY_RMB_UP => FLAG_RIGHT_RELEASE,
        _ => 0,
    };
    if key == 0 {
        flags |= if input.left_held { FLAG_LEFT_HELD } else { FLAG_LEFT_UP };
        flags |= if input.right_held { FLAG_RIGHT_HELD } else { FLAG_RIGHT_UP };
    } else if flags == 0 {
        flags = FLAG_KEYBOARD;
    }

    // G9 — modifier word, polled fresh; passed ONLY to the broadcast walk
    // (hardwired 0 for the sticky and keyboard tiers).
    let modifier: u8 =
        u8::from(input.shift) | (u8::from(input.ctrl) << 1) | (u8::from(input.alt) << 2);

    let live = (input.mouse_x, input.mouse_y);

    // G10 tier 1 — sticky capture: exclusive; dispatched even masked-0.
    if let Some(handle) = focus.sticky {
        if list.get(handle).is_some() {
            draw_one(list, handle, false, out); // G11 pre-draw
            clicked_on(list, handle, &mut key, flags, x, y, 0, live, focus, out);
            // G11 post-draw re-reads the capture slot: a gadget that released
            // capture this call still gets its post-draw.
            let post = focus.sticky.unwrap_or(handle);
            draw_one(list, post, false, out);
            return key;
        }
        // Unreachable by construction (removal clears focus); never dispatch
        // into a missing slot.
        focus.sticky = None;
    }

    // G10 tier 2 — keyboard focus: only for keyboard-flag ticks.
    if let Some(handle) = focus.keyboard {
        if (flags & FLAG_KEYBOARD) != 0 && list.get(handle).is_some() {
            draw_one(list, handle, false, out);
            clicked_on(list, handle, &mut key, flags, x, y, 0, live, focus, out);
            let post = focus.keyboard.unwrap_or(handle);
            draw_one(list, post, false, out);
            return key;
        }
    }

    // G12 tier 3 — broadcast walk head→tail: every visited gadget is drawn
    // (forced on a fresh list) BEFORE dispatch; disabled gadgets are drawn
    // but never dispatched; the first consumer gets one extra draw and stops
    // the walk — later gadgets get NEITHER call this tick.
    for i in 0..list.len() {
        let handle = list.handle_at(i);
        draw_one(list, handle, list_changed, out);
        let disabled = list.get(handle).is_none_or(|g| g.is_disabled);
        if !disabled
            && clicked_on(list, handle, &mut key, flags, x, y, modifier, live, focus, out) != 0
        {
            draw_one(list, handle, false, out);
            out.consumed_by = Some(handle);
            break;
        }
    }
    key
}
```

**Step 2: append the tick integration tests** inside the existing `#[cfg(test)] mod tests`
block of `tick.rs`:

```rust
    fn btn(rect: GadgetRect, id: u16) -> GadgetSpec {
        GadgetSpec::button(rect, id, crate::ui::gadget::list::ToggleKind::Flip)
    }

    fn idle(mx: i32, my: i32) -> GadgetInput {
        GadgetInput {
            mouse_x: mx,
            mouse_y: my,
            ..Default::default()
        }
    }

    fn event(key: u16, ex: i32, ey: i32, held_left: bool) -> GadgetInput {
        GadgetInput {
            queued_key: key,
            event_x: ex,
            event_y: ey,
            mouse_x: ex,
            mouse_y: ey,
            left_held: held_left,
            ..Default::default()
        }
    }

    #[test]
    fn g5_fresh_list_reset_clears_capture_and_force_draws() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l1 = GadgetList::new(ListId(1));
        let a = l1.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x65));
        // Press captures on list 1.
        tick(&mut l1, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        assert_eq!(f.sticky, Some(a));
        // Ticking a DIFFERENT list resets capture + keyboard, force-draws all.
        let mut l2 = GadgetList::new(ListId(2));
        l2.add_tail(GadgetSpec::new(GadgetRect::new(0, 0, 5, 5), 0, false));
        l2.add_tail(GadgetSpec::new(GadgetRect::new(0, 0, 5, 5), 0, false));
        tick(&mut l2, &mut f, &idle(100, 100), &mut out);
        assert_eq!(f.sticky, None, "G5 nulls capture");
        assert_eq!(f.current_list, Some(ListId(2)));
        assert_eq!(out.draws.len(), 2, "fresh list force-draws every gadget");
        assert!(out.draws.iter().all(|d| d.forced));
    }

    #[test]
    fn g6_event_coords_for_mouse_keys_live_for_idle() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x65));
        // Event coords inside the rect, live mouse far away: the press (low
        // byte 1) must hit-test/dispatch at the EVENT coords.
        let mut input = event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true);
        input.mouse_x = 500;
        input.mouse_y = 500;
        tick(&mut l, &mut f, &input, &mut out);
        assert_eq!(f.sticky, Some(a), "dispatched at event coords");
        assert_eq!(f.hovered, Some(a), "hover hit-tested at event coords too");
        // Idle tick uses the live cursor: far away ⇒ hover leaves.
        let mut f2 = FocusState::new();
        f2.current_list = Some(ListId(1));
        f2.hovered = Some(a);
        tick(&mut l, &mut f2, &idle(500, 500), &mut out);
        assert_eq!(f2.hovered, None);
        assert_eq!(out.hover_left, Some(a));
    }

    #[test]
    fn g7_hover_enter_leave_and_removal_closure() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(GadgetSpec::new(GadgetRect::new(0, 0, 10, 10), 0, false));
        tick(&mut l, &mut f, &idle(5, 5), &mut out);
        assert_eq!(out.hover_entered, Some(a));
        assert_eq!(f.hovered, Some(a));
        // Removing the hovered gadget clears hover; the NEXT tick reports no
        // hover_left for the dead handle (study §6.1 G7-closure).
        l.remove(a, &mut f);
        assert_eq!(f.hovered, None);
        tick(&mut l, &mut f, &idle(5, 5), &mut out);
        assert_eq!(out.hover_left, None, "no Leave fires for a dead handle");
        assert_eq!(out.hover_entered, None);
    }

    #[test]
    fn g8_flag_assembly_held_bits_idle_only() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        // Mask everything so the dispatch record shows the assembled flags.
        let mut s = GadgetSpec::new(GadgetRect::new(0, 0, 10, 10), 0x01FF, false);
        s.behavior = GadgetBehavior::Control;
        s.id = 0x11;
        l.add_tail(s);
        // Idle tick, left held, right up → 0x2 | 0x80.
        let mut input = idle(5, 5);
        input.left_held = true;
        tick(&mut l, &mut f, &input, &mut out);
        assert_eq!(out.dispatches[0].masked_flags, FLAG_LEFT_HELD | FLAG_RIGHT_UP);
        // Press event tick with left ALSO held: event bit only, NO held bits.
        let input = event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true);
        tick(&mut l, &mut f, &input, &mut out);
        assert_eq!(out.dispatches[0].masked_flags, FLAG_LEFT_PRESS, "never both");
        // Queued non-mouse key → exactly FLAG_KEYBOARD.
        let input = event(0x1C, 5, 5, false);
        tick(&mut l, &mut f, &input, &mut out);
        assert_eq!(out.dispatches[0].masked_flags, FLAG_KEYBOARD);
    }

    #[test]
    fn g9_modifier_word_broadcast_only() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x65));
        let mut input = event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true);
        input.shift = true;
        input.alt = true;
        // Broadcast-tier dispatch carries the modifier word.
        tick(&mut l, &mut f, &input, &mut out);
        assert_eq!(out.dispatches[0].modifier, 0b101, "SHIFT=1 | ALT=4");
        assert_eq!(f.sticky, Some(a));
        // Sticky-tier re-dispatch hardwires 0.
        let mut input2 = idle(5, 5);
        input2.left_held = true;
        input2.shift = true;
        tick(&mut l, &mut f, &input2, &mut out);
        assert_eq!(out.dispatches[0].modifier, 0, "sticky tier modifier = 0");
    }

    #[test]
    fn g10_tier_precedence_sticky_exclusive() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x65));
        let b = l.add_tail(btn(GadgetRect::new(20, 0, 10, 10), 0x66));
        // Capture a.
        tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        assert_eq!(f.sticky, Some(a));
        // A press over b while a holds capture goes to a ONLY (tier 1 is
        // exclusive); b is never dispatched.
        tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 25, 5, true), &mut out);
        assert_eq!(out.dispatches.len(), 1);
        assert_eq!(out.dispatches[0].handle, a);
        assert_eq!(out.consumed_by, None, "no broadcast walk ran");
        let _ = b;
    }

    #[test]
    fn g10_keyboard_tier_and_g13_result() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let mut s = GadgetSpec::new(GadgetRect::new(0, 0, 10, 10), 0x05, false);
        s.id = 0x42;
        s.behavior = GadgetBehavior::Control;
        let a = l.add_tail(s);
        crate::ui::gadget::list::set_focus(&mut l, &mut f, a);
        f.current_list = Some(ListId(1));
        // Keyboard event with the cursor far away: routed to the focus
        // holder, bounds bypassed, result = ID|0x8000.
        let result = tick(&mut l, &mut f, &event(0x1C, 500, 500, false), &mut out);
        assert_eq!(result, 0x42 | 0x8000);
        assert_eq!(out.dispatches[0].handle, a);
        // A MOUSE event does not enter the keyboard tier (falls to broadcast,
        // misses the rect, returns the raw key).
        let result = tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 500, 500, false), &mut out);
        assert_eq!(result, crate::ui::gadget::KEY_LMB_DOWN);
    }

    #[test]
    fn g12_walk_stops_at_consumer_draw_cadence() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let a = l.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x65));
        let b = l.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x66)); // same rect, later
        let c = l.add_tail(btn(GadgetRect::new(40, 0, 10, 10), 0x67));
        // Prime current_list so this is NOT a fresh tick.
        tick(&mut l, &mut f, &idle(100, 100), &mut out);
        // Press inside a+b: the walk visits a (clicked_on consumes — a is
        // FIRST in walk order; note hit-test priority would pick b, but the
        // broadcast walk dispatches in LIST order and a consumes first).
        tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        assert_eq!(out.consumed_by, Some(a), "walk order, first consumer stops");
        // a got visited-draw + consumer-draw (both dirty-gated); c never
        // visited after the break: dispatch list has exactly one entry.
        assert_eq!(out.dispatches.len(), 1);
        let _ = (b, c);
    }

    #[test]
    fn g22_end_to_end_click_fires_on_release_only() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let _a = l.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x65));
        // Press: consumed, returns 0 (silent).
        let r = tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        assert_eq!(r, 0, "silent press");
        // Idle held tick (sticky re-dispatch, masked-0): nothing fires.
        let mut held = idle(5, 5);
        held.left_held = true;
        let r = tick(&mut l, &mut f, &held, &mut out);
        assert_eq!(r, 0);
        // Release inside: fires ID|0x8000.
        let r = tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_UP, 5, 5, false), &mut out);
        assert_eq!(r, 0x65 | 0x8000, "fire on release-inside");
        assert_eq!(f.sticky, None);
    }

    #[test]
    fn g22_end_to_end_drag_off_cancels() {
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let _a = l.add_tail(btn(GadgetRect::new(0, 0, 10, 10), 0x65));
        tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        // Drag off (idle tick, cursor outside).
        let mut held = idle(50, 50);
        held.left_held = true;
        tick(&mut l, &mut f, &held, &mut out);
        // Release outside: nothing fires.
        let mut up = event(crate::ui::gadget::KEY_LMB_UP, 50, 50, false);
        up.mouse_x = 50;
        up.mouse_y = 50;
        let r = tick(&mut l, &mut f, &up, &mut out);
        assert_eq!(r, crate::ui::gadget::KEY_LMB_UP, "no result posted — cancelled");
        assert_eq!(f.sticky, None, "capture released");
    }
```

**Step 3: Verify.** `cargo test -p vera20k ui::gadget` → all gadget tests pass
(3 + 2 + 7 + 10 + 18 across the module tree).

**Step 4: Commit.** `ui: A0 T7 — event-flag tick (G5-G13 dispatch, hover, draw cadence) + clause tests`

### Task 8 (A0 gate): full-suite gate

**Why:** A0 is pure substrate; nothing else may have moved.

**Step 1:** Run `cargo test -p vera20k` in the worktree.
Expected: ALL tests green, including the 87-test skirmish net
(`ui::skirmish_shell::state::tests`). Read the literal `test result:` lines before reporting.

**Step 2:** `cargo clippy -p vera20k` — fix any new warnings in `src/ui/gadget/` only.

**Step 3:** Commit only if fixups were needed:
`test(ui): A0 gate — full suite green on gadget substrate core`

### Task 9 (A1 T1): pressed-bit visuals in `SidebarGadgetState`

**Why:** The G22 silent press needs a visible pressed frame (3/4); today repair/sell pass
`state=0` unconditionally and tabs only flash. The gadget driver (Task 12) publishes these
bits after every tick.

**Files:**
- Modify: `src/sidebar/gadget_flash.rs`

**Step 1: add the fields.** In `src/sidebar/gadget_flash.rs`, the struct currently ends:

```rust
    /// Last sim tick the orchestrator processed; used to advance per
    /// sim-tick delta (catch-up safe).
    pub last_sim_tick: u64,
}
```

Replace with:

```rust
    /// Last sim tick the orchestrator processed; used to advance per
    /// sim-tick delta (catch-up safe).
    pub last_sim_tick: u64,

    /// Transient pressed-look bits (study G22): true while the gadget driver
    /// holds the button pressed-inside; popped on drag-off, restored on
    /// drag-back. Published by `app_gadget_input` after every gadget tick.
    pub tab_pressed: [bool; 4],
    pub repair_pressed: bool,
    pub sell_pressed: bool,
    pub scroll_down_pressed: bool,
    pub scroll_up_pressed: bool,
}
```

**Step 2: route the bits into the frame accessors.** Replace the three accessors (currently
`tab_frame` at :166-170, `repair_frame` at :174-176, `sell_frame` at :179-181):

```rust
    /// Frame index for a tab gadget. Caller passes whether this tab is the
    /// currently-active tab (the externally driven latch-ON mirror,
    /// study §2.5 / G22 Kind 2).
    pub fn tab_frame(&self, tab_index: usize, is_active_tab: bool) -> u8 {
        let flash = &self.tab_flashes[tab_index];
        let disabled = self.tab_disabled[tab_index];
        // Pressed-look = flash pulse OR live press-hold (study G22).
        let state = if self.tab_pressed[tab_index] { 1 } else { flash.state };
        frame_select(disabled, is_active_tab, state)
    }

    /// Frame index for the Repair button. Repair has no flash AI — the state
    /// bit is the live press-hold; "stays pressed" comes from `mode_active`.
    pub fn repair_frame(&self) -> u8 {
        frame_select(self.repair_disabled, self.repair_mode_on, u8::from(self.repair_pressed))
    }

    /// Frame index for the Sell button. Same logic as `repair_frame`.
    pub fn sell_frame(&self) -> u8 {
        frame_select(self.sell_disabled, self.sell_mode_on, u8::from(self.sell_pressed))
    }

    /// Frame index for the strip scroll-down (+page) button. No mode bit, no
    /// flash — pressed-look only.
    pub fn scroll_down_frame(&self) -> u8 {
        frame_select(false, false, u8::from(self.scroll_down_pressed))
    }

    /// Frame index for the strip scroll-up (−page) button.
    pub fn scroll_up_frame(&self) -> u8 {
        frame_select(false, false, u8::from(self.scroll_up_pressed))
    }
```

**Step 3: add tests** at the end of the `#[cfg(test)] mod tests` block in the same file:

```rust
    #[test]
    fn pressed_bits_drive_frames_3_and_4() {
        let mut s = SidebarGadgetState::new();
        s.repair_pressed = true;
        assert_eq!(s.repair_frame(), 3, "pressed-idle");
        s.repair_mode_on = true;
        assert_eq!(s.repair_frame(), 4, "pressed-active");
        s.tab_pressed[1] = true;
        assert_eq!(s.tab_frame(1, false), 3);
        assert_eq!(s.tab_frame(1, true), 4);
        s.scroll_down_pressed = true;
        assert_eq!(s.scroll_down_frame(), 3);
        assert_eq!(s.scroll_up_frame(), 0);
    }
```

**Step 4: Verify.** `cargo test -p vera20k gadget_flash` → all (12) tests pass.

**Step 5: Commit.** `ui: A1 T1 — pressed-bit visuals in SidebarGadgetState (frames 3/4 reachable)`

### Task 10 (A1 T2): strip-scroll chrome — R-UP/R-DN atlas, rect helper, draw

**Why:** The scroll buttons are NEW (wheel-only today); they need art + a single rect source
shared by the driver and the renderer (retained order = draw order = hit priority).

**Files:**
- Modify: `src/render/sidebar_chrome.rs`
- Modify: `src/sidebar/mod.rs`
- Modify: `src/app_sidebar_build.rs`

**Step 1: atlas fields.** In `src/render/sidebar_chrome.rs`, the struct currently has:

```rust
    /// Repair button — 5-frame SHP state table (same convention as `tab_frames`).
    pub repair_frames: [Option<SidebarChromeEntry>; 5],
    /// Sell button — 5-frame SHP state table (same convention as `tab_frames`).
    pub sell_frames: [Option<SidebarChromeEntry>; 5],
```

Insert after `sell_frames`:

```rust
    /// Strip scroll-up (R-UP.SHP, −page) — assumed 5-frame convention; the
    /// frame count is unverified (plan deferred item), missing frames fall
    /// back to frame 0 at draw exactly like repair/sell.
    pub scroll_up_frames: [Option<SidebarChromeEntry>; 5],
    /// Strip scroll-down (R-DN.SHP, +page).
    pub scroll_down_frames: [Option<SidebarChromeEntry>; 5],
```

**Step 2: render + register + pack the SHPs.** The chrome-atlas pipeline needs THREE
coordinated edits per new piece (all in the same function): (1) render into entry arrays,
(2) push refs into `all_entries` — the atlas height is the SUM of every registered entry's
height (sidebar_chrome.rs:437-441), so an entry that is blitted but NOT registered would
write past the atlas bounds — and (3) a body-level `blit_entry` pack loop producing the
packed UV array (this is a sequenced loop advancing the shared `y` cursor, NOT a per-field
conversion expression).

(a) **Render.** After the `sell_frame_entries` loop (ends at sidebar_chrome.rs:328 with
`sell_frame_entries[frame] = entry;` + `}`), insert:

```rust
    // Strip-scroll pair. Frame count unverified (deferred) — render_entry
    // returns None for absent frames and the draw falls back to frame 0, so
    // either a 1-frame or 5-frame retail SHP degrades gracefully.
    let mut scroll_up_entries: [Option<RenderedChromeEntry>; 5] = Default::default();
    let mut scroll_down_entries: [Option<RenderedChromeEntry>; 5] = Default::default();
    for frame in 0..5 {
        scroll_up_entries[frame] = render_entry(asset_manager, &mix, "r-up.shp", &palette, frame);
        scroll_down_entries[frame] = render_entry(asset_manager, &mix, "r-dn.shp", &palette, frame);
    }
    if scroll_up_entries[0].is_none() {
        log::warn!("r-up.shp missing in MIX — strip scroll-up button will not render");
    }
    if scroll_down_entries[0].is_none() {
        log::warn!("r-dn.shp missing in MIX — strip scroll-down button will not render");
    }
```

(b) **Register into `all_entries`.** In the "Collect all pieces" block, immediately after
the `sell_frame_entries` push loop (sidebar_chrome.rs:422-426, same shape as the repair
push loop at :417-421), insert:

```rust
    for frame in 0..5 {
        if let Some(ref entry) = scroll_up_entries[frame] {
            all_entries.push(entry);
        }
    }
    for frame in 0..5 {
        if let Some(ref entry) = scroll_down_entries[frame] {
            all_entries.push(entry);
        }
    }
```

(c) **Pack.** Immediately after the `sell_frames_packed` blit loop (sidebar_chrome.rs:520-527,
next to `repair_frames_packed` at :512-519), insert the two pack loops — same
`blit_entry` + `y` advance pattern:

```rust
    let mut scroll_up_frames_packed: [Option<SidebarChromeEntry>; 5] = Default::default();
    for frame in 0..5 {
        if let Some(ref entry) = scroll_up_entries[frame] {
            let uv = blit_entry(&mut rgba, atlas_width, atlas_height, y, entry);
            y += entry.height + CHROME_PADDING;
            scroll_up_frames_packed[frame] = Some(uv);
        }
    }
    let mut scroll_down_frames_packed: [Option<SidebarChromeEntry>; 5] = Default::default();
    for frame in 0..5 {
        if let Some(ref entry) = scroll_down_entries[frame] {
            let uv = blit_entry(&mut rgba, atlas_width, atlas_height, y, entry);
            y += entry.height + CHROME_PADDING;
            scroll_down_frames_packed[frame] = Some(uv);
        }
    }
```

(d) **Assign.** In the `SidebarChromeAtlas { .. }` struct literal at the end of the function
(sidebar_chrome.rs:581), immediately after `sell_frames: sell_frames_packed,` (:601), add:

```rust
        scroll_up_frames: scroll_up_frames_packed,
        scroll_down_frames: scroll_down_frames_packed,
```

**Step 3: rect helper.** In `src/sidebar/mod.rs`, after `compute_layout_with_spec` (ends
line ~333), insert:

```rust
/// Strip-scroll button rects (the R-DN/R-UP pair). gamemd anchors the pair at
/// ScrollX / ScrollX+ScrollWidth in retail strip geometry; our adaptive RON
/// layout (R11 geometry policy — OUT of this plan's scope) has no such
/// anchor, so the interim placement centers the pair inside the side3 strip:
/// scroll-down (+page) on the left, scroll-up (−page) on the right.
/// `None` sizes (atlas missing) collapse to 0×0 — unhittable and undrawn.
pub fn scroll_button_rects(
    layout: &SidebarLayout,
    sidebar_width: f32,
    down_size: Option<[f32; 2]>,
    up_size: Option<[f32; 2]>,
) -> (Rect, Rect) {
    let [dw, dh] = down_size.unwrap_or([0.0, 0.0]);
    let [uw, uh] = up_size.unwrap_or([0.0, 0.0]);
    let x0 = layout.sidebar_x + (sidebar_width - (dw + uw)) * 0.5;
    let y = layout.side3_y + 1.0;
    (
        Rect { x: x0, y, w: dw, h: dh },
        Rect { x: x0 + dw, y, w: uw, h: uh },
    )
}
```

**Step 4: draw the pair.** In `src/app_sidebar_build.rs`:

(a) `build_sidebar_chrome_instances_for_layout` gains a `scroll_frames: [u8; 2]` parameter
([down, up]). Current signature:

```rust
pub fn build_sidebar_chrome_instances_for_layout(
    atlas: &SidebarChromeAtlas,
    spec: SidebarChromeLayoutSpec,
    layout: &SidebarLayout,
    view: &SidebarView,
    tabs: &[SidebarTabButton],
    power_bar_anim: &PowerBarAnimState,
    _screen_size: [f32; 2],
    camera_offset: [f32; 2],
    ui_scale: f32,
) -> Vec<SpriteInstance> {
```

Add `scroll_frames: [u8; 2],` after `power_bar_anim: &PowerBarAnimState,`.

(b) The sole caller (`build_sidebar_chrome_instances`, app_sidebar_build.rs:33-47) passes:

```rust
        &state.power_bar_anim,
```

— add immediately after that argument:

```rust
        [
            state.sidebar_gadget_state.scroll_down_frame(),
            state.sidebar_gadget_state.scroll_up_frame(),
        ],
```

(c) In the function body, after the repair-button block (currently ending at line ~194 with
the closing `}` of `if let Some(repair) = atlas.repair_frames[...]`), insert:

```rust
    // --- Strip-scroll pair (R-DN +page left, R-UP −page right) ---
    // Same 5-frame select + frame-0 fallback convention as repair/sell.
    let (scroll_down_rect, scroll_up_rect) = crate::sidebar::scroll_button_rects(
        layout,
        spec.sidebar_width,
        atlas.scroll_down_frames[0]
            .as_ref()
            .map(|e| [e.pixel_size[0] * s, e.pixel_size[1] * s]),
        atlas.scroll_up_frames[0]
            .as_ref()
            .map(|e| [e.pixel_size[0] * s, e.pixel_size[1] * s]),
    );
    let down_frame = scroll_frames[0] as usize;
    if let Some(e) = atlas.scroll_down_frames[down_frame].or(atlas.scroll_down_frames[0]) {
        push_chrome(&mut inst, e, scroll_down_rect.x, scroll_down_rect.y, btn_depth, camera_offset, s);
    }
    let up_frame = scroll_frames[1] as usize;
    if let Some(e) = atlas.scroll_up_frames[up_frame].or(atlas.scroll_up_frames[0]) {
        push_chrome(&mut inst, e, scroll_up_rect.x, scroll_up_rect.y, btn_depth, camera_offset, s);
    }
```

(If `SidebarChromeEntry` is not `Copy` at the `.or(...)` site, use
`.as_ref().or(atlas.scroll_down_frames[0].as_ref())` and pass `*e` — match whatever the
repair/sell blocks at :172/:184 compile with.)

**Step 5: Verify.** `cargo check -p vera20k` clean; `cargo test -p vera20k sidebar` green.
Run the game (`cargo run -p vera20k`, load any skirmish map): two scroll buttons render in
the side3 strip (or a warn log if R-UP/R-DN are absent from the side MIX — note the result
either way; frame-count question is a deferred item).

**Step 6: Commit.** `render: A1 T2 — strip-scroll chrome (R-UP/R-DN atlas + rects + draw)`

### Task 11 (A1 T3): the in-game gadget driver — state + build/sync

**Why:** Owns the ONE retained list for the in-game surface; rects re-sync from the live
`SidebarView` every tick so the substrate keeps the adaptive geometry as-is (study §6.4).

**Files:**
- Create: `src/app_gadget_input.rs`
- Modify: `src/lib.rs`, `src/app.rs`

**Step 1: declare the module.** In `src/lib.rs`, immediately after the existing line
`pub mod app_sidebar_gadgets;`, add:

```rust
pub mod app_gadget_input;
```

**Step 2: create `src/app_gadget_input.rs`:**

```rust
//! In-game Framework-A gadget driver: owns the retained sidebar button list
//! (study §6.1 `ui::gadget`), builds/synchronizes it from the live
//! `SidebarView`, feeds it mouse-edge events plus one idle tick per frame,
//! applies fired button IDs onto existing app actions, and publishes the
//! transient pressed bits for the 5-frame visuals.
//!
//! Replaces fire-on-mouse-DOWN for tabs / repair / sell (study G22) and adds
//! the strip-scroll pair (mask 0x55 ⇒ no hold-repeat, one page per click, G23).
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use winit::event::MouseButton;

use crate::app::AppState;
use crate::app_sidebar_render::current_sidebar_view;
use crate::sidebar::{self, SidebarAction, SidebarTab, SidebarView};
use crate::ui::gadget::focus::FocusState;
use crate::ui::gadget::list::{GadgetBehavior, GadgetList, GadgetSpec, ToggleKind};
use crate::ui::gadget::tick::{GadgetInput, TickOutput, tick};
use crate::ui::gadget::{
    GadgetHandle, GadgetRect, KEY_LMB_DOWN, KEY_LMB_UP, KEY_RMB_DOWN, KEY_RMB_UP, ListId,
    RESULT_BUTTON, RESULT_RIGHT,
};

/// gamemd sidebar button IDs (study §2.5 live-population table; Kind/mask
/// identities VERIFIED-LIVE — decompile citation in the plan Sources section).
pub(crate) const ID_TAB_BASE: u16 = 0x00CB; // tabs 0xCB..=0xCE, Kind 2 latch-ON
pub(crate) const ID_REPAIR: u16 = 0x0065; // Kind 1 flip
pub(crate) const ID_SELL: u16 = 0x0066; // Kind 1 flip
pub(crate) const ID_SCROLL_DOWN: u16 = 0x00C9; // +1 page, Kind 0
pub(crate) const ID_SCROLL_UP: u16 = 0x00C8; // −1 page, Kind 0
/// Scroll mask: presses + releases for BOTH buttons, no held bits — no
/// hold-repeat (G23); right-release fires `ID|0xC000`, consumer masks it off.
const SCROLL_FLAGS: u16 = 0x0055;
/// The single in-game gadget list (ListId uniqueness is app-owned).
const IN_GAME_LIST: ListId = ListId(1);

/// Stable handles of the 8 sidebar buttons, in retained order.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarButtonHandles {
    pub tabs: [GadgetHandle; 4],
    pub repair: GadgetHandle,
    pub sell: GadgetHandle,
    pub scroll_down: GadgetHandle,
    pub scroll_up: GadgetHandle,
}

/// Persistent driver state on `AppState`.
#[derive(Debug)]
pub(crate) struct InGameGadgets {
    pub list: GadgetList,
    pub focus: FocusState,
    /// Reused tick output (buffers cleared per tick, never reallocated).
    pub out: TickOutput,
    /// Live held record (G8 idle-tick held-bit source — nothing else in the
    /// app tracks left/right held state).
    pub left_held: bool,
    pub right_held: bool,
    pub handles: Option<SidebarButtonHandles>,
}

impl InGameGadgets {
    pub fn new() -> Self {
        Self {
            list: GadgetList::new(IN_GAME_LIST),
            focus: FocusState::new(),
            out: TickOutput::default(),
            left_held: false,
            right_held: false,
            handles: None,
        }
    }
}

fn rect_px(r: sidebar::Rect) -> GadgetRect {
    GadgetRect::new(
        r.x.round() as i32,
        r.y.round() as i32,
        r.w.round() as i32,
        r.h.round() as i32,
    )
}

/// Atlas frame-0 sizes for the scroll pair, ×ui_scale (same convention as the
/// repair/sell view rects — zero size when the atlas is missing).
fn scroll_sizes(state: &AppState) -> (Option<[f32; 2]>, Option<[f32; 2]>) {
    let Some(atlas) = crate::app_sidebar_render::current_sidebar_chrome(state) else {
        return (None, None);
    };
    let sz = |e: Option<&crate::render::sidebar_chrome::SidebarChromeEntry>| {
        e.map(|e| [e.pixel_size[0] * state.ui_scale, e.pixel_size[1] * state.ui_scale])
    };
    (
        sz(atlas.scroll_down_frames[0].as_ref()),
        sz(atlas.scroll_up_frames[0].as_ref()),
    )
}

/// Build the list once (retained order = tabs 0..3, repair, sell, scroll-down,
/// scroll-up; rects are disjoint so relative order is unobservable today, and
/// this pins ONE order for hit priority + draw, study O7/G20), then re-sync
/// every tick: rects from the live view, disabled bits + is_on from app state
/// (the native external latch-on/latch-off equivalent — tabs are externally
/// driven, study §2.1).
fn sync_gadgets(state: &mut AppState, view: &SidebarView) {
    let (down_size, up_size) = scroll_sizes(state);
    let (down_rect, up_rect) = sidebar::scroll_button_rects(
        &view.layout,
        state.sidebar_layout_spec.sidebar_width,
        down_size,
        up_size,
    );
    let tab_rects: Vec<GadgetRect> = view.tabs.iter().map(|t| rect_px(t.rect)).collect();
    let tab_active: Vec<bool> = view.tabs.iter().map(|t| t.active).collect();
    let repair_rect = rect_px(view.repair_button.rect);
    let sell_rect = rect_px(view.sell_button.rect);
    let gs = state.sidebar_gadget_state.clone();

    let gadgets = &mut state.in_game_gadgets;
    if gadgets.handles.is_none() {
        let list = &mut gadgets.list;
        let zero = GadgetRect::new(0, 0, 0, 0);
        let tabs = [0u16, 1, 2, 3].map(|i| {
            list.add_tail(GadgetSpec::button(zero, ID_TAB_BASE + i, ToggleKind::LatchOn))
        });
        let repair = list.add_tail(GadgetSpec::button(zero, ID_REPAIR, ToggleKind::Flip));
        let sell = list.add_tail(GadgetSpec::button(zero, ID_SELL, ToggleKind::Flip));
        let scroll_down = list.add_tail(
            GadgetSpec::button(zero, ID_SCROLL_DOWN, ToggleKind::Plain).with_flags(SCROLL_FLAGS),
        );
        let scroll_up = list.add_tail(
            GadgetSpec::button(zero, ID_SCROLL_UP, ToggleKind::Plain).with_flags(SCROLL_FLAGS),
        );
        gadgets.handles = Some(SidebarButtonHandles {
            tabs,
            repair,
            sell,
            scroll_down,
            scroll_up,
        });
    }
    let handles = gadgets.handles.expect("built above");
    let sync = |list: &mut GadgetList, h: GadgetHandle, rect, disabled, is_on: Option<bool>| {
        if let Some(g) = list.get_mut(h) {
            g.rect = rect;
            g.is_disabled = disabled;
            if let (Some(on), GadgetBehavior::Button(b)) = (is_on, &mut g.behavior) {
                b.is_on = on;
            }
        }
    };
    for i in 0..4 {
        let rect = tab_rects.get(i).copied().unwrap_or(GadgetRect::new(0, 0, 0, 0));
        let active = tab_active.get(i).copied().unwrap_or(false);
        sync(&mut gadgets.list, handles.tabs[i], rect, gs.tab_disabled[i], Some(active));
    }
    sync(&mut gadgets.list, handles.repair, repair_rect, gs.repair_disabled, Some(gs.repair_mode_on));
    sync(&mut gadgets.list, handles.sell, sell_rect, gs.sell_disabled, Some(gs.sell_mode_on));
    sync(&mut gadgets.list, handles.scroll_down, rect_px(down_rect), false, None);
    sync(&mut gadgets.list, handles.scroll_up, rect_px(up_rect), false, None);
}
```

**Step 3: AppState field + ctor.** In `src/app.rs`, the field block currently reads
(app.rs:215-219):

```rust
    pub(crate) power_bar_anim: crate::sidebar::PowerBarAnimState,
    /// Persistent flash + mode state for in-game sidebar gadgets. Ticked from
    /// `app_sidebar_gadgets::update_sidebar_gadget_state` once per sim tick;
    /// read each frame by the sidebar view builder to pick SHP frame indices.
    pub(crate) sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState,
```

Append after the `sidebar_gadget_state` line:

```rust
    /// In-game gadget substrate (study §6.1): retained sidebar button list +
    /// capture/focus state + reusable tick output + the mouse-held record.
    pub(crate) in_game_gadgets: crate::app_gadget_input::InGameGadgets,
```

And in the ctor (app.rs:2569-2570 currently):

```rust
            power_bar_anim: crate::sidebar::PowerBarAnimState::new(),
            sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState::new(),
```

append after the `sidebar_gadget_state` line:

```rust
            in_game_gadgets: crate::app_gadget_input::InGameGadgets::new(),
```

**Step 4: Verify.** `cargo check -p vera20k` clean (the driver compiles; nothing calls it
yet — if `current_sidebar_chrome` or `SidebarChromeEntry` paths differ, fix the imports to
the actual `pub(crate)` items in `app_sidebar_render.rs` / `render/sidebar_chrome.rs`).

**Step 5: Commit.** `app: A1 T3 — in-game gadget list driver state (build/sync sidebar buttons)`

### Task 12 (A1 T4): the authority flip — event ticks, idle tick, fired-ID application

**Why:** This is the player-visible flip: silent press + fire-on-RELEASE-inside + drag-off
cancel for tabs/repair/sell, and one-page-per-click scroll buttons.

**Files:**
- Modify: `src/app_gadget_input.rs`, `src/app_input.rs`, `src/app_sim_tick.rs`

**Step 1: tick entry points.** Append to `src/app_gadget_input.rs`:

```rust
/// Route a mouse press/release edge into the gadget tick. Returns true when
/// the substrate consumed the event — the caller must NOT fall through to the
/// legacy sidebar/minimap/selection paths. A release completing a captured
/// gesture is always consumed (in gamemd the sticky tier is exclusive: no
/// other gadget — including the tactical catcher — ever sees that event).
pub(crate) fn handle_mouse_button_event(
    state: &mut AppState,
    button: MouseButton,
    pressed: bool,
) -> bool {
    // The held record updates on every edge (G8 idle-tick source).
    match button {
        MouseButton::Left => state.in_game_gadgets.left_held = pressed,
        MouseButton::Right => state.in_game_gadgets.right_held = pressed,
        _ => return false,
    }
    let Some(view) = current_sidebar_view(state) else {
        return false;
    };
    sync_gadgets(state, &view);
    let key = match (button, pressed) {
        (MouseButton::Left, true) => KEY_LMB_DOWN,
        (MouseButton::Left, false) => KEY_LMB_UP,
        (MouseButton::Right, true) => KEY_RMB_DOWN,
        (MouseButton::Right, false) => KEY_RMB_UP,
        _ => return false,
    };
    run_tick(state, &view, key)
}

/// Once-per-frame idle tick: drives the masked-0 sticky re-dispatch that pops
/// the pressed visual on drag-off and restores it on drag-back (G22 rows 2/3)
/// and would drive G23 hold-repeat for any future held-mask gadget.
pub(crate) fn idle_tick(state: &mut AppState) {
    let Some(view) = current_sidebar_view(state) else {
        return;
    };
    sync_gadgets(state, &view);
    run_tick(state, &view, 0);
}

fn run_tick(state: &mut AppState, view: &SidebarView, key: u16) -> bool {
    // We tick synchronously on the edge, so event coords == live coords
    // (gamemd latches coords at enqueue; with no queue lag the two sources
    // are identical — G6 still selects per the key's low byte).
    let cx = state.cursor_x.round() as i32;
    let cy = state.cursor_y.round() as i32;
    let input = GadgetInput {
        queued_key: key,
        event_x: cx,
        event_y: cy,
        mouse_x: cx,
        mouse_y: cy,
        left_held: state.in_game_gadgets.left_held,
        right_held: state.in_game_gadgets.right_held,
        shift: crate::app_input::is_shift_held(state),
        ctrl: crate::app_input::is_ctrl_held(state),
        alt: crate::app_input::is_alt_held(state),
    };
    let was_captured = state.in_game_gadgets.focus.sticky.is_some();
    let gadgets = &mut state.in_game_gadgets;
    let result = tick(&mut gadgets.list, &mut gadgets.focus, &input, &mut gadgets.out);
    let consumed_walk = state.in_game_gadgets.out.consumed_by.is_some();
    let fired = (result & RESULT_BUTTON) != 0;
    if fired {
        apply_gadget_result(state, view, result);
    }
    publish_pressed_visuals(state);
    fired || consumed_walk || was_captured
}

/// Map a fired `ID|0x8000[|0x4000]` onto the existing app actions. Consumers
/// mask the right-release marker off (study §2.2: `key & ~0x4000`), so a
/// right-click scrolls identically.
fn apply_gadget_result(state: &mut AppState, view: &SidebarView, result: u16) {
    let id = result & !(RESULT_BUTTON | RESULT_RIGHT);
    match id {
        _ if (ID_TAB_BASE..ID_TAB_BASE + 4).contains(&id) => {
            let tab = SidebarTab::all()[(id - ID_TAB_BASE) as usize];
            crate::app_input::apply_sidebar_action(state, SidebarAction::SelectTab(tab));
        }
        ID_REPAIR => {
            crate::app_input::apply_sidebar_action(state, SidebarAction::ToggleRepairMode);
        }
        ID_SELL => {
            crate::app_input::apply_sidebar_action(state, SidebarAction::ToggleSellMode);
        }
        // One PAGE per click (G23: mask 0x55 has no held bits ⇒ no repeat).
        // Page = visible cameo rows; gamemd computes (strip px height)/50
        // which equals the visible row count.
        ID_SCROLL_DOWN => {
            let page = view.layout.side2_tile_count.max(1);
            state.sidebar_scroll_rows =
                (state.sidebar_scroll_rows + page).min(view.max_scroll_rows);
        }
        ID_SCROLL_UP => {
            let page = view.layout.side2_tile_count.max(1);
            state.sidebar_scroll_rows = state.sidebar_scroll_rows.saturating_sub(page);
        }
        _ => {}
    }
}

/// Publish the transient pressed bits for the 5-frame visuals (frames 3/4).
fn publish_pressed_visuals(state: &mut AppState) {
    let Some(handles) = state.in_game_gadgets.handles else {
        return;
    };
    let pressed = |h: GadgetHandle| {
        state
            .in_game_gadgets
            .list
            .get(h)
            .is_some_and(|g| matches!(g.behavior, GadgetBehavior::Button(b) if b.is_pressed))
    };
    let tabs = handles.tabs.map(pressed);
    let repair = pressed(handles.repair);
    let sell = pressed(handles.sell);
    let down = pressed(handles.scroll_down);
    let up = pressed(handles.scroll_up);
    let gs = &mut state.sidebar_gadget_state;
    gs.tab_pressed = tabs;
    gs.repair_pressed = repair;
    gs.sell_pressed = sell;
    gs.scroll_down_pressed = down;
    gs.scroll_up_pressed = up;
}
```

NOTE: `sync_gadgets` clones `SidebarGadgetState` (`let gs = state.sidebar_gadget_state.clone();`)
to keep borrows disjoint — it is a small Copy-like struct of bools/arrays; if the clone is
unavailable derive stays `Clone` (it already is, gadget_flash.rs:130).

NOTE: the tab arm deliberately plays NO sound yet — the `gui_tab_sound` ruleset field does
not exist at this point in the task order. Task 14 parses `GUITabSound` AND wires the sound
into this arm, so this task's full-suite gate stays green (no forward reference to an
unlanded field).

**Step 2: flip `handle_mouse_input`.** In `src/app_input.rs`, the entry currently reads
(lines 34-43):

```rust
pub(crate) fn handle_mouse_input(
    state: &mut AppState,
    button: MouseButton,
    btn_state: ElementState,
) {
    if btn_state.is_pressed() {
        if handle_sidebar_mouse_input(state, button) {
            return;
        }
    }
```

Replace with:

```rust
pub(crate) fn handle_mouse_input(
    state: &mut AppState,
    button: MouseButton,
    btn_state: ElementState,
) {
    // Gadget substrate first (study G22): tabs/repair/sell/scroll consume
    // their presses silently and fire on RELEASE-inside with drag-off cancel.
    // Consumed events never fall through to minimap/selection. Cameos and the
    // dev/control buttons stay on the legacy press path until slice A2.
    if crate::app_gadget_input::handle_mouse_button_event(state, button, btn_state.is_pressed()) {
        return;
    }
    if btn_state.is_pressed() {
        if handle_sidebar_mouse_input(state, button) {
            return;
        }
    }
```

**Step 3: visibility.** In the same file, change `fn apply_sidebar_action(` to
`pub(crate) fn apply_sidebar_action(` (app_input.rs:240). Locate the modifier helpers
(`is_shift_held` / `is_ctrl_held` / `is_alt_held`, app_input.rs:~795-810) and ensure each is
`pub(crate) fn` (add the qualifier where missing).

**Step 4: idle tick hook.** In `src/app_sim_tick.rs`, the per-frame block currently reads
(lines 216-218):

```rust
    crate::app_building_anim::update_radar_state(state, SIM_TICK_MS as f32);
    crate::app_building_anim::update_power_bar_anim(state);
    crate::app_sidebar_gadgets::update_sidebar_gadget_state(state);
```

Append after the `update_sidebar_gadget_state` line:

```rust
    // Per-frame gadget idle tick (G22 rows 2/3 drag-off/drag-back tracking).
    crate::app_gadget_input::idle_tick(state);
```

**Step 5: Verify.**
- `cargo test -p vera20k` → green.
- Run the game and check the manual list: (a) click tab/repair/sell → mode changes on
  RELEASE, not press; (b) press, drag off, release → NOTHING happens and the pressed frame
  pops out while off; (c) press-hold shows frames 3/4; (d) scroll buttons page once per
  click, holding does NOT repeat, right-click scrolls too; (e) tactical/minimap/cameo clicks
  unchanged (fall-through intact); (f) press a sidebar button, release over the map → no
  unit command is issued.

**Step 6: Commit.** `app: A1 T4 — fire-on-release authority flip (gadget tick on press/release + idle)`

### Task 13 (A1 T5): retire the tab/repair/sell probes from the legacy hit-test

**Why:** Those three surfaces are now substrate-owned; leaving the press-fire probes would
double-handle the press (R7/R8 partial retirement).

**Files:**
- Modify: `src/sidebar/mod.rs`

**Step 1:** In `src/sidebar/mod.rs`, `hit_test` currently begins (lines 379-396):

```rust
pub fn hit_test(view: &SidebarView, x: f32, y: f32, right_click: bool) -> SidebarAction {
    if !view.panel_rect.contains(x, y) {
        return SidebarAction::None;
    }

    for tab in &view.tabs {
        if tab.rect.contains(x, y) {
            return SidebarAction::SelectTab(tab.tab);
        }
    }

    if view.repair_button.rect.contains(x, y) {
        return view.repair_button.action.clone();
    }
    if view.sell_button.rect.contains(x, y) {
        return view.sell_button.action.clone();
    }

    for item in &view.items {
```

Replace with:

```rust
/// Legacy press-path hit-test for the surfaces NOT yet on the gadget
/// substrate (cameos, pause/producer, dev buttons). Tabs, repair, sell and
/// the strip-scroll pair are owned by `app_gadget_input` (fire-on-release,
/// study G22) and are deliberately absent here.
pub fn hit_test(view: &SidebarView, x: f32, y: f32, right_click: bool) -> SidebarAction {
    if !view.panel_rect.contains(x, y) {
        return SidebarAction::None;
    }

    for item in &view.items {
```

(the rest of the function body — items, pause/producer, dev buttons, trailing
`SidebarAction::None` — is unchanged).

**Step 2: delete the two retired-path tests.** The `#[cfg(test)] mod tests` block at the
bottom of the same file asserts the OLD press-path routing for the two surfaces this task
just removed from `hit_test` — they would fail as written:

- `hit_test_routes_repair_button` (sidebar/mod.rs:511-542) — asserts
  `SidebarAction::ToggleRepairMode`;
- `hit_test_routes_sell_button` (sidebar/mod.rs:544-572) — asserts
  `SidebarAction::ToggleSellMode`.

Delete both test functions outright (do NOT rewrite them against `hit_test`: the
substrate-side coverage already exists — the G22 state-table tests in `ui/gadget/button.rs`
plus the Task 12 fired-ID mapping). The third `hit_test` test
(`sidebar/sidebar_view.rs:593`, cancel button → `CancelLastBuild`) exercises a surface that
stays on the legacy path — leave it untouched.

**Step 3: Verify.** `cargo test -p vera20k sidebar` green; `cargo check -p vera20k` clean.
In-game: tabs/repair/sell still work (now via the substrate ONLY); right-click on a tab does
nothing (gamemd-correct — was a drift); cameo left/right clicks unchanged.

**Step 4: Commit.** `ui: A1 T5 — retire tab/repair/sell probes from legacy sidebar hit_test`

### Task 14 (A1 T6): parse `[AudioVisual] GUITabSound` + wire the tab-click sound

**Why:** gamemd plays a sound on tab select (consumer behavior); the key is unparsed today.
Mapping is name-inferred (LOW confidence — Parity table row). Task 12 deliberately left the
tab arm silent (the field did not exist yet); this task parses the key AND wires the
consumer in one gate-green step.

**Files:**
- Modify: `src/rules/ruleset.rs`, `src/app_gadget_input.rs`

**Step 1: field.** In `GeneralRules`, after the existing field pair (ruleset.rs:302-303):

```rust
    /// Sound event for shell checkboxes from [AudioVisual] GUICheckboxSound.
    pub gui_checkbox_sound: Option<String>,
```

insert:

```rust
    /// Sidebar tab click sound from [AudioVisual] GUITabSound (retail
    /// `MenuTab`). The key→tab-click mapping is name-inferred — flagged for a
    /// Ghidra spot-check of the tab-ID consumer before parity sign-off.
    pub gui_tab_sound: Option<String>,
```

**Step 2: parse.** In the `GeneralRules { .. }` construction, after the
`gui_checkbox_sound:` initializer (ruleset.rs:982-986):

```rust
            gui_checkbox_sound: audio_visual
                .and_then(|s| s.get("GUICheckboxSound"))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
```

insert:

```rust
            gui_tab_sound: audio_visual
                .and_then(|s| s.get("GUITabSound"))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
```

(Verbatim mirror of the five adjacent UI-sound parses — same trim/filter-empty `Option`
pattern. No dedicated unit test: there is no minimal-`RuleSet` fixture harness for
`[AudioVisual]` keys in the repo; the field is exercised end-to-end by the Step 3 consumer
and checked at the slice gate runtime step.)

**Step 3: wire the consumer.** In `src/app_gadget_input.rs`, the `apply_gadget_result` tab
arm (landed in Task 12) currently reads:

```rust
        _ if (ID_TAB_BASE..ID_TAB_BASE + 4).contains(&id) => {
            let tab = SidebarTab::all()[(id - ID_TAB_BASE) as usize];
            crate::app_input::apply_sidebar_action(state, SidebarAction::SelectTab(tab));
        }
```

Replace with:

```rust
        _ if (ID_TAB_BASE..ID_TAB_BASE + 4).contains(&id) => {
            let tab = SidebarTab::all()[(id - ID_TAB_BASE) as usize];
            crate::app_input::apply_sidebar_action(state, SidebarAction::SelectTab(tab));
            // [AudioVisual] GUITabSound — name-inferred mapping (LOW
            // confidence): one Ghidra spot-check of the tab-ID consumer is a
            // plan Parity-Critical follow-up.
            let sound = state
                .rules
                .as_ref()
                .and_then(|r| r.general.gui_tab_sound.clone());
            crate::app::App::play_shell_ui_sound_by_id(state, sound.as_deref());
        }
```

**Step 4: Verify.** `cargo check -p vera20k` clean; `cargo test -p vera20k` → green. Run
the game: clicking a tab plays MenuTab (audible) — the in-game sidebar was previously silent.

**Step 5: Commit.** `rules: A1 T6 — parse [AudioVisual] GUITabSound + wire tab click sound`

### Task 15 (A1 gate): full-suite + behavior gate

**Step 1:** `cargo test -p vera20k` → ALL green including the 87-test skirmish net.

**Step 2:** Manual behavior checklist (run the game):
1. Tab/repair/sell fire on RELEASE-inside; drag-off cancels; pressed frames 3/4 show while
   held-inside and pop on drag-off.
2. Scroll buttons: one page per click, no hold-repeat, right-click scrolls, clamped at both
   ends, wheel scrolling still works independently.
3. Tab select resets scroll to row 0; repair⇄sell mutual exclusion intact; arming placement
   clears repair/sell modes.
4. Cameo click/right-click-cancel, pause/producer, dev buttons: identical to pre-flip.
5. Click on sidebar panel background (no button): still falls through (no selection-drag
   regression under the panel — same as before).
6. Tab click plays MenuTab.

**Step 3:** Commit only if fixups were needed:
`test(ui): A1 gate — sidebar flip verified (release-fire, drag-off, scroll pair)`

### Task 16 (A4 T1): parse `UIName=` on ObjectType

**Why:** Cameo tooltips need the localized name; `UIName=` (e.g. `Name:MTNK`) is parsed
nowhere today — `object_type.rs` carries only `Name=` (raw English).

**Files:**
- Modify: `src/rules/object_type.rs`
- Modify (test fixtures only): `src/sim/movement/locomotor_tests.rs`,
  `src/sim/movement/teleport_movement.rs`

**Step 1: field.** The struct currently declares (object_type.rs:152-155):

```rust
    /// Display name (CSF string table key or raw text). None if not specified.
    pub name: Option<String>,
    /// Credit cost to produce this object.
    pub cost: i32,
```

Insert between `name` and `cost`:

```rust
    /// Localized display-name CSF key from `UIName=` (e.g. "Name:MTNK").
    /// None if not specified. Tooltip text resolves this through the CSF
    /// table; `name` (English `Name=`) is the fallback.
    pub ui_name: Option<String>,
```

**Step 2: parse.** The constructor currently assigns (object_type.rs:863-867):

```rust
        Self {
            id: id.to_string(),
            category,
            name: section.get("Name").map(|s| s.to_string()),
            cost: section.get_i32("Cost").unwrap_or(0),
```

Insert after the `name:` line:

```rust
            ui_name: section.get("UIName").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
```

**Step 3: add `ui_name: None,` to the two sim test fixtures.** `cargo check -p vera20k`
flags every full-field `ObjectType { .. }` literal missing the new field. Exactly TWO exist
at baseline, both `#[cfg(test)]` fixtures under `src/sim/` (verified: the only
`ObjectType { .. }` literal sites in the crate; neither uses `..Default`, so both MUST be
edited or the crate does not compile):

- `src/sim/movement/locomotor_tests.rs:12` (`make_obj`) — insert `ui_name: None,`
  immediately after its `name: None,` line;
- `src/sim/movement/teleport_movement.rs:387` (`make_drive_obj`, inside the file's
  `#[cfg(test)]` module; the second helper at :592 builds on it and needs no edit) — insert
  `ui_name: None,` immediately after its `name:` field line.

These are fixture-literal additions only — no sim behavior/logic change (see the amended
Sim Checklist; both files are named in the File Map). This is the plan's ONE scheduled
exception to the Risk Areas rule "if `cargo check` fails in files this plan does not touch,
STOP" — these two failures are expected and plan-owned; any OTHER sim failure still means
STOP.

**Step 4: add a parse test** in the `#[cfg(test)]` module of `object_type.rs` (mirror the
style of the existing section-based tests in that file; if a section-fixture helper exists,
use it — otherwise build an `IniSection` the same way the file's existing tests do):

```rust
    #[test]
    fn parses_ui_name_key() {
        // Reuse the file's existing test fixture pattern for building a
        // section containing: Name=Grizzly Tank, UIName=Name:MTNK, Cost=700.
        // Assert: parsed.ui_name == Some("Name:MTNK".to_string()) and an
        // absent/empty UIName yields None.
    }
```

(Fill the body with the file's actual fixture helper — the existing tests in
`object_type.rs` show the exact construction; assert both the `Some` and `None` cases.)

**Step 5: Verify.** `cargo test -p vera20k object_type` → green, incl. the new test.

**Step 6: Commit.** `rules: A4 T1 — parse UIName= on ObjectType`

### Task 17 (A4 T2): the shared tooltip service model

**Why:** Study S1 — one ToolTipManager-equivalent for both frameworks: 1000 ms delay,
10000 ms duration, INCLUSIVE-both-edges rects, first-registered-wins, move-restarts,
kill-on-any-button, auto-hide re-arm, delay override hook. Pure + injectable clock.

**Files:**
- Create: `src/ui/tooltips.rs`
- Modify: `src/ui/mod.rs`

**Step 1: declare.** In `src/ui/mod.rs`, the list currently ends:

```rust
pub mod single_player_shell;
pub mod skirmish_shell;
```

Append after `skirmish_shell;`:

```rust
pub mod tooltips;
```

**Step 2: create `src/ui/tooltips.rs`:**

```rust
//! Shared tooltip service model (study S1): the native tooltip-manager
//! equivalent consumed by BOTH the in-game sidebar and the front-end shells.
//!
//! Pure and clock-injected: every entry point takes `now_ms: u64`; the app
//! driver (`app_tooltips.rs`) is the only place that reads the wall clock.
//! Timer semantics reproduce the native single-timer state machine: one
//! deadline serves both the show delay and the visible-tip duration; polling
//! once per frame lands edges within a frame of the wall-clock deadline —
//! observably identical to the OS-timer pump at any playable frame rate.
//!
//! ## Dependency rules
//! - ui/ module: std only — no render/, assets/, sidebar/, audio/, net/.

/// Delay before a tip shows (native ctor hardcodes 1000 — NOT INI-driven).
pub const TOOLTIP_DELAY_MS: u64 = 1000;
/// Auto-hide duration once shown (native ctor hardcodes 10000).
pub const TOOLTIP_DURATION_MS: u64 = 10_000;
/// Tip text cap (native buffer is 0x100 wide chars).
pub const TOOLTIP_TEXT_CAP_CHARS: usize = 256;

/// Tip rect with INCLUSIVE-both-edges containment — deliberately different
/// from the gadget half-open rule (study S1): the x+w / y+h pixel row/column
/// still hits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TipRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl TipRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains_inclusive(&self, px: i32, py: i32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// One registered tip region. Text is resolved by the driver at sync time
/// (per frame), which is observably equivalent to the native resolve-at-show
/// (content changes propagate within a frame). EMPTY text reproduces the
/// native "NULL text ⇒ Show fails ⇒ no tip" outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipRegion {
    pub id: u32,
    pub rect: TipRect,
    pub text: String,
}

/// The currently visible tip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTip {
    pub id: u32,
    pub text: String,
    /// Cursor position captured at show (contract lane §3.3 show snapshot).
    pub x: i32,
    pub y: i32,
    pub shown_at_ms: u64,
}

#[derive(Debug, Default)]
pub struct TooltipService {
    enabled: bool,
    regions: Vec<TipRegion>,
    active: Option<ActiveTip>,
    /// The single timer slot: pending show-delay OR visible-tip duration.
    timer_deadline_ms: Option<u64>,
    /// Cameo-style zero-delay override (contract lane §3.4 hover hook).
    delay_override_ms: Option<u64>,
    mouse_x: i32,
    mouse_y: i32,
}

impl TooltipService {
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    fn delay_ms(&self) -> u64 {
        self.delay_override_ms.unwrap_or(TOOLTIP_DELAY_MS)
    }

    /// Register one region; duplicate ids are rejected (native behavior).
    pub fn register(&mut self, region: TipRegion) -> bool {
        if self.regions.iter().any(|r| r.id == region.id) {
            return false;
        }
        self.regions.push(region);
        true
    }

    /// Unregister by id; hides the tip first when it is the visible one.
    pub fn unregister(&mut self, id: u32) -> bool {
        if self.active.as_ref().is_some_and(|a| a.id == id) {
            self.active = None;
        }
        let before = self.regions.len();
        self.regions.retain(|r| r.id != id);
        self.regions.len() != before
    }

    /// Per-frame driver convenience: replace the whole region set (rects move
    /// with the adaptive layout). The visible tip survives iff its id is
    /// still present; a vanished id hides (unregister semantics).
    pub fn sync_regions(&mut self, regions: &[TipRegion]) {
        if let Some(a) = &self.active {
            if !regions.iter().any(|r| r.id == a.id) {
                self.active = None;
            }
        }
        self.regions.clear();
        self.regions.extend_from_slice(regions);
    }

    /// Mouse move: with a non-zero delay, every move RESTARTS the delay timer
    /// and hides a visible tip; with a zero delay-override the next poll
    /// shows immediately (the timer fires at `now`).
    pub fn on_mouse_move(&mut self, x: i32, y: i32, now_ms: u64) {
        if !self.enabled {
            return;
        }
        self.mouse_x = x;
        self.mouse_y = y;
        let delay = self.delay_ms();
        if delay != 0 {
            if self.active.is_some() {
                self.active = None;
            }
            self.timer_deadline_ms = Some(now_ms + delay);
        } else {
            self.timer_deadline_ms = Some(now_ms);
        }
    }

    /// Any mouse button press/release (all 6 native button messages,
    /// including middle): kill the timer + hide.
    pub fn on_button(&mut self, _now_ms: u64) {
        self.timer_deadline_ms = None;
        if self.active.is_some() {
            self.active = None;
        }
    }

    /// The timer pump (call once per frame). Native WM_TIMER semantics: a
    /// firing timer with a visible tip hides it (duration expiry) and arms
    /// nothing; otherwise the cursor is hit-tested against the regions in
    /// REGISTRATION order (first match wins, inclusive edges) and a
    /// non-empty-text match shows + re-arms the duration timer.
    pub fn poll(&mut self, now_ms: u64) {
        if !self.enabled {
            return;
        }
        let Some(deadline) = self.timer_deadline_ms else {
            return;
        };
        if now_ms < deadline {
            return;
        }
        self.timer_deadline_ms = None;
        if self.active.is_some() {
            self.active = None;
            return;
        }
        let hit = self
            .regions
            .iter()
            .find(|r| r.rect.contains_inclusive(self.mouse_x, self.mouse_y));
        if let Some(r) = hit {
            if !r.text.is_empty() {
                let text: String = r.text.chars().take(TOOLTIP_TEXT_CAP_CHARS).collect();
                self.active = Some(ActiveTip {
                    id: r.id,
                    text,
                    x: self.mouse_x,
                    y: self.mouse_y,
                    shown_at_ms: now_ms,
                });
                self.timer_deadline_ms = Some(now_ms + TOOLTIP_DURATION_MS);
            }
        }
    }

    pub fn active(&self) -> Option<&ActiveTip> {
        self.active.as_ref()
    }

    /// Cameo-hover hook (slice A2 consumer; exposed + tested now): zero delay
    /// while highlighted, restored on leave.
    pub fn set_delay_override(&mut self, delay_ms: Option<u64>) {
        self.delay_override_ms = delay_ms;
    }

    /// Enable gate; disabling kills the timer and hides immediately.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.timer_deadline_ms = None;
            self.active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(id: u32, x: i32, y: i32, w: i32, h: i32, text: &str) -> TipRegion {
        TipRegion {
            id,
            rect: TipRect::new(x, y, w, h),
            text: text.to_string(),
        }
    }

    fn service_with(regions: &[TipRegion]) -> TooltipService {
        let mut s = TooltipService::new();
        for r in regions {
            assert!(s.register(r.clone()));
        }
        s
    }

    #[test]
    fn shows_after_exactly_1000_ms_of_stillness() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "tip")]);
        s.on_mouse_move(5, 5, 0);
        s.poll(999);
        assert!(s.active().is_none(), "1 ms early: nothing");
        s.poll(1000);
        let tip = s.active().expect("shown at the deadline");
        assert_eq!(tip.id, 1);
        assert_eq!(tip.shown_at_ms, 1000);
        assert_eq!((tip.x, tip.y), (5, 5), "cursor captured at show");
    }

    #[test]
    fn every_move_restarts_the_delay_and_hides() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "tip")]);
        s.on_mouse_move(5, 5, 0);
        s.on_mouse_move(6, 5, 900);
        s.poll(1000);
        assert!(s.active().is_none(), "restarted at 900 → deadline 1900");
        s.poll(1900);
        assert!(s.active().is_some());
        // A move while visible hides AND re-arms.
        s.on_mouse_move(7, 5, 2000);
        assert!(s.active().is_none(), "moving hides a visible tip");
        s.poll(3000);
        assert!(s.active().is_some(), "re-shown after another full delay");
    }

    #[test]
    fn duration_auto_hide_re_arm() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "tip")]);
        s.on_mouse_move(5, 5, 0);
        s.poll(1000);
        assert!(s.active().is_some());
        s.poll(10_999);
        assert!(s.active().is_some(), "duration is 10000 ms from show");
        s.poll(11_000);
        assert!(s.active().is_none(), "auto-hide at shown+10000");
        s.poll(50_000);
        assert!(s.active().is_none(), "stays hidden until the next move");
    }

    #[test]
    fn any_button_kills_timer_and_tip() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "tip")]);
        s.on_mouse_move(5, 5, 0);
        s.on_button(500);
        s.poll(1000);
        assert!(s.active().is_none(), "pending timer killed");
        s.on_mouse_move(5, 5, 2000);
        s.poll(3000);
        assert!(s.active().is_some());
        s.on_button(3100);
        assert!(s.active().is_none(), "visible tip killed");
    }

    #[test]
    fn inclusive_both_edges_vs_gadget_half_open() {
        // Region {x:10, y:10, w:5, h:5}: the (15,15) corner pixel HITS the
        // tip rect (inclusive) but MISSES a gadget rect of the same numbers
        // (half-open) — the deliberate 1-px observable difference (study S1).
        let mut s = service_with(&[region(1, 10, 10, 5, 5, "tip")]);
        s.on_mouse_move(15, 15, 0);
        s.poll(1000);
        assert!(s.active().is_some(), "inclusive edge hits");
        let gadget_rect = crate::ui::gadget::GadgetRect::new(10, 10, 5, 5);
        assert!(!gadget_rect.contains(15, 15), "same point misses the gadget rect");
        // One past the inclusive edge misses.
        let mut s2 = service_with(&[region(1, 10, 10, 5, 5, "tip")]);
        s2.on_mouse_move(16, 15, 0);
        s2.poll(1000);
        assert!(s2.active().is_none());
    }

    #[test]
    fn first_registered_wins_on_overlap() {
        let mut s = service_with(&[
            region(1, 0, 0, 100, 100, "first"),
            region(2, 0, 0, 10, 10, "second-smaller"),
        ]);
        s.on_mouse_move(5, 5, 0);
        s.poll(1000);
        assert_eq!(
            s.active().unwrap().text,
            "first",
            "registration order, NOT smallest-area (unlike the gadget rule)"
        );
    }

    #[test]
    fn duplicate_register_rejected_unregister_hides() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "tip")]);
        assert!(!s.register(region(1, 50, 50, 5, 5, "dup")), "dup id rejected");
        s.on_mouse_move(5, 5, 0);
        s.poll(1000);
        assert!(s.active().is_some());
        assert!(s.unregister(1));
        assert!(s.active().is_none(), "unregister hides the visible tip");
        assert!(!s.unregister(1));
    }

    #[test]
    fn empty_text_never_shows() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "")]);
        s.on_mouse_move(5, 5, 0);
        s.poll(1000);
        assert!(s.active().is_none(), "NULL/empty text ⇒ Show fails ⇒ no tip");
    }

    #[test]
    fn zero_delay_override_shows_on_next_poll() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "cameo")]);
        s.set_delay_override(Some(0));
        s.on_mouse_move(5, 5, 7000);
        s.poll(7000);
        assert!(s.active().is_some(), "cameo-hover zero delay: immediate");
        s.set_delay_override(None);
        s.on_mouse_move(6, 5, 7100);
        assert!(s.active().is_none());
        s.poll(7500);
        assert!(s.active().is_none(), "restored 1000 ms delay applies again");
        s.poll(8100);
        assert!(s.active().is_some());
    }

    #[test]
    fn sync_regions_keeps_active_iff_id_survives() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "tip")]);
        s.on_mouse_move(5, 5, 0);
        s.poll(1000);
        assert!(s.active().is_some());
        // Same id, moved rect: tip survives.
        s.sync_regions(&[region(1, 100, 100, 10, 10, "tip")]);
        assert!(s.active().is_some());
        // Id gone: hides.
        s.sync_regions(&[region(2, 0, 0, 10, 10, "other")]);
        assert!(s.active().is_none());
    }

    #[test]
    fn disable_kills_and_gates() {
        let mut s = service_with(&[region(1, 0, 0, 10, 10, "tip")]);
        s.on_mouse_move(5, 5, 0);
        s.poll(1000);
        assert!(s.active().is_some());
        s.set_enabled(false);
        assert!(s.active().is_none());
        s.on_mouse_move(5, 5, 2000);
        s.poll(5000);
        assert!(s.active().is_none(), "disabled service ignores input");
    }

    #[test]
    fn text_capped_at_256_chars() {
        let long = "x".repeat(400);
        let mut s = service_with(&[region(1, 0, 0, 10, 10, &long)]);
        s.on_mouse_move(5, 5, 0);
        s.poll(1000);
        assert_eq!(s.active().unwrap().text.chars().count(), 256);
    }
}
```

**Step 3: Verify.** `cargo test -p vera20k ui::tooltips` → 12 tests pass.

**Step 4: Commit.** `ui: A4 T2 — shared tooltip service model (1000 ms delay, inclusive rects, kill-on-press)`

### Task 18 (A4 T3): tooltip driver — clock, event feeds, region sync

**Why:** Wires the model to the app: wall clock lives HERE only; sidebar + main-menu shell
regions sync per frame; mouse events feed the timer.

**Files:**
- Create: `src/app_tooltips.rs`
- Modify: `src/lib.rs`, `src/app.rs`

**Step 1: declare.** In `src/lib.rs`, after `pub mod app_gadget_input;`, add:

```rust
pub mod app_tooltips;
```

**Step 2: create `src/app_tooltips.rs`:**

```rust
//! Tooltip service driver (study S1): the ONLY wall-clock reader for tooltip
//! timing. Feeds cursor moves + button kills into `ui::tooltips`, re-syncs
//! the region set per frame (in-game sidebar buttons + cameos; main-menu
//! shell buttons), and builds the in-game tooltip draw instances.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::app_sidebar_render::current_sidebar_view;
use crate::render::SpriteInstance;
use crate::ui::game_screen::GameScreen;
use crate::ui::tooltips::{TipRect, TipRegion};

/// In-game tip ids mirror the gamemd id space: button ids as-is, cameo slots
/// at 1000+. Shell tips are namespaced above the in-game range (Rust-side
/// convention; gamemd separates them by registration epoch instead).
pub(crate) const CAMEO_TIP_ID_BASE: u32 = 1000;
pub(crate) const SHELL_TIP_NAMESPACE: u32 = 0x0001_0000;

/// Interim cameo tip shape: localized name, newline, $cost. gamemd formats
/// through CSF#0xC6E (label unmapped — plan deferred item); name+cost args
/// are the verified content.
const CAMEO_TIP_COST_PREFIX: &str = "$";

/// Box placement: cursor offset + screen clamp (the native placement math is
/// undecoded — plan deferred item).
pub(crate) const TIP_CURSOR_OFFSET: [i32; 2] = [12, 16];
/// Box padding around the measured text (doc-inherited +4/+3 box metrics).
pub(crate) const TIP_PAD: [f32; 2] = [4.0, 3.0];
/// Line stride for multi-line tips (GAME.FNT cell height).
pub(crate) const TIP_LINE_HEIGHT: f32 = 17.0;
/// Tip text tint (interim: shell yellow; native scheme unverified — deferred).
pub(crate) const TIP_TEXT_RGB: [f32; 3] = [1.0, 1.0, 0.0];

pub(crate) fn now_ms(state: &AppState) -> u64 {
    state.tooltip_epoch.elapsed().as_millis() as u64
}

/// CursorMoved feed (all screens).
pub(crate) fn on_mouse_move(state: &mut AppState) {
    let now = now_ms(state);
    let (x, y) = (state.cursor_x.round() as i32, state.cursor_y.round() as i32);
    state.tooltips.on_mouse_move(x, y, now);
}

/// MouseInput feed — ANY button, press or release, kills tip + timer.
pub(crate) fn on_button_event(state: &mut AppState) {
    let now = now_ms(state);
    state.tooltips.on_button(now);
}

/// Per-frame update: refresh regions for the live surface, then pump the
/// timer. `main_menu_shell_live` is computed by the caller (app.rs owns the
/// shell-activity predicates).
pub(crate) fn update(state: &mut AppState, main_menu_shell_live: bool) {
    let now = now_ms(state);
    if state.screen == GameScreen::InGame {
        sync_in_game_regions(state);
    } else if main_menu_shell_live {
        sync_main_menu_regions(state);
    } else {
        state.tooltips.sync_regions(&[]);
    }
    state.tooltips.poll(now);
}

fn tip_rect(r: crate::sidebar::Rect) -> TipRect {
    TipRect::new(
        r.x.round() as i32,
        r.y.round() as i32,
        r.w.round() as i32,
        r.h.round() as i32,
    )
}

fn csf_text(state: &AppState, key: &str) -> String {
    state
        .csf
        .as_ref()
        .and_then(|csf| csf.get(key))
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}

/// Sidebar regions, mirroring the native registration set (contract lane §3.6):
/// tabs + scroll (EMPTY text until the CSF numeric-id mapping pass — no tip
/// shows, matching the native NULL-text outcome), repair/sell (direct CSF
/// keys), cameos (name + cost, interim format).
fn sync_in_game_regions(state: &mut AppState) {
    let Some(view) = current_sidebar_view(state) else {
        state.tooltips.sync_regions(&[]);
        return;
    };
    let mut regions: Vec<TipRegion> = Vec::with_capacity(8 + view.items.len());
    for (i, tab) in view.tabs.iter().enumerate() {
        regions.push(TipRegion {
            id: crate::app_gadget_input::ID_TAB_BASE as u32 + i as u32,
            rect: tip_rect(tab.rect),
            text: String::new(), // CSF#0x13DB..0x13E1 labels unmapped (deferred)
        });
    }
    regions.push(TipRegion {
        id: crate::app_gadget_input::ID_REPAIR as u32,
        rect: tip_rect(view.repair_button.rect),
        text: csf_text(state, "TXT_REPAIR_MODE"),
    });
    regions.push(TipRegion {
        id: crate::app_gadget_input::ID_SELL as u32,
        rect: tip_rect(view.sell_button.rect),
        text: csf_text(state, "TXT_SELL_MODE"),
    });
    {
        let (down_size, up_size) = {
            let atlas = crate::app_sidebar_render::current_sidebar_chrome(state);
            let sz = |e: Option<&crate::render::sidebar_chrome::SidebarChromeEntry>| {
                e.map(|e| [e.pixel_size[0] * state.ui_scale, e.pixel_size[1] * state.ui_scale])
            };
            match atlas {
                Some(a) => (sz(a.scroll_down_frames[0].as_ref()), sz(a.scroll_up_frames[0].as_ref())),
                None => (None, None),
            }
        };
        let (down_rect, up_rect) = crate::sidebar::scroll_button_rects(
            &view.layout,
            state.sidebar_layout_spec.sidebar_width,
            down_size,
            up_size,
        );
        regions.push(TipRegion {
            id: crate::app_gadget_input::ID_SCROLL_DOWN as u32,
            rect: tip_rect(down_rect),
            text: String::new(), // CSF#0x13D3 unmapped (deferred)
        });
        regions.push(TipRegion {
            id: crate::app_gadget_input::ID_SCROLL_UP as u32,
            rect: tip_rect(up_rect),
            text: String::new(), // CSF#0x13CD unmapped (deferred)
        });
    }
    for (slot, item) in view.items.iter().enumerate() {
        let text = if item.is_superweapon {
            // SW tips: the SW UIName directly (no cost). Localized SW UIName
            // parse is a deferred follow-up; display_name today.
            item.display_name.clone()
        } else {
            let name = state
                .rules
                .as_ref()
                .and_then(|r| r.object(&item.type_id))
                .and_then(|o| o.ui_name.as_deref())
                .and_then(|key| state.csf.as_ref().and_then(|csf| csf.get(key)))
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| item.display_name.clone());
            match item.cost {
                Some(cost) => format!("{name}\n{CAMEO_TIP_COST_PREFIX}{cost}"),
                None => name,
            }
        };
        regions.push(TipRegion {
            id: CAMEO_TIP_ID_BASE + slot as u32,
            rect: tip_rect(item.rect),
            text,
        });
    }
    state.tooltips.sync_regions(&regions);
}

/// Main-menu shell regions: button rects + their STT CSF texts. The render
/// pass shows the active tip in the bottom tooltip line (timing changes,
/// placement stays — the native floating-box visual is a deferred item).
fn sync_main_menu_regions(state: &mut AppState) {
    let layout = crate::ui::main_menu_shell::compute_layout(
        state.gpu.config.width,
        state.gpu.config.height,
    );
    let mut regions: Vec<TipRegion> = Vec::with_capacity(layout.buttons.len());
    for b in &layout.buttons {
        // Already `pub` and re-exported from ui::main_menu_shell (state.rs:111
        // via mod.rs:11-14) — the same helper app_main_menu_shell_render.rs
        // imports for its own emission.
        let key = crate::ui::main_menu_shell::tooltip_csf_key_for_control(b.id);
        regions.push(TipRegion {
            id: SHELL_TIP_NAMESPACE | u32::from(b.id.resource_id()),
            rect: TipRect::new(b.rect.x, b.rect.y, b.rect.w, b.rect.h),
            text: csf_text(state, key),
        });
    }
    state.tooltips.sync_regions(&regions);
}

/// In-game tooltip draw: (fill instances on the darken texture, text
/// instances on the GAME.FNT atlas), drawn between the chat overlay and the
/// software cursor (study O10). Shell tips draw via the shell text path.
pub(crate) fn build_tooltip_instances(state: &AppState) -> (Vec<SpriteInstance>, Vec<SpriteInstance>) {
    let Some(tip) = state.tooltips.active() else {
        return (Vec::new(), Vec::new());
    };
    if (tip.id & SHELL_TIP_NAMESPACE) != 0 || state.screen != GameScreen::InGame {
        return (Vec::new(), Vec::new());
    }
    let font = &state.bit_font;
    let lines: Vec<&str> = tip.text.split('\n').collect();
    let text_w = lines
        .iter()
        .map(|l| font.text_width(l) as f32)
        .fold(0.0_f32, f32::max);
    let box_w = text_w + TIP_PAD[0] * 2.0;
    let box_h = lines.len() as f32 * TIP_LINE_HEIGHT + TIP_PAD[1] * 2.0;
    // Cursor offset, clamped on-screen (placement math deferred).
    let max_x = state.render_width() as f32 - box_w;
    let max_y = state.render_height() as f32 - box_h;
    let bx = ((tip.x + TIP_CURSOR_OFFSET[0]) as f32).clamp(0.0, max_x.max(0.0));
    let by = ((tip.y + TIP_CURSOR_OFFSET[1]) as f32).clamp(0.0, max_y.max(0.0));
    let mut fill = Vec::with_capacity(1);
    if state.bit_font.darken_texture().is_some() {
        fill.push(SpriteInstance {
            position: [bx, by],
            size: [box_w, box_h],
            depth: 0.00021,
            tint: [1.0, 1.0, 1.0],
            alpha: 1.0,
            ..Default::default()
        });
    }
    let mut text = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        text.extend(crate::render::sidebar_text::build_text(
            font,
            line,
            bx + TIP_PAD[0],
            by + TIP_PAD[1] + i as f32 * TIP_LINE_HEIGHT,
            1.0,
            0.00020,
            TIP_TEXT_RGB,
            [0.0, 0.0],
        ));
    }
    (fill, text)
}
```

(`SpriteInstance` import path: match the one `app_sidebar_build.rs` uses; the darken-fill
instance mirrors the Ready-strip pattern at app_sidebar_build.rs:520-543 — copy its exact
uv/Default field handling if `SpriteInstance` requires uv fields for the darken texture.)

**Step 3: AppState fields.** In `src/app.rs`, append after the `in_game_gadgets` field
added in Task 11:

```rust
    /// Shared tooltip service (study S1) — the model is clock-injected; only
    /// `app_tooltips` reads the wall clock.
    pub(crate) tooltips: crate::ui::tooltips::TooltipService,
    /// Epoch for the tooltip/message wall-clock (`now_ms` = elapsed since
    /// app construction).
    pub(crate) tooltip_epoch: Instant,
```

and in the ctor after `in_game_gadgets: ...`:

```rust
            tooltips: crate::ui::tooltips::TooltipService::new(),
            tooltip_epoch: Instant::now(),
```

**Step 4: event feeds.** In `src/app.rs`:

(a) `CursorMoved` arm — after the cursor assignment block (currently lines 2174-2179):

```rust
                state.cursor_x = position.x as f32 * sx;
                state.cursor_y = position.y as f32 * sy;
                // Keep OS cursor hidden whenever the software cursor is active.
                if state.use_software_cursor() {
                    state.window.set_cursor_visible(false);
                }
```

append:

```rust
                // Shared tooltip service: every move restarts the show delay
                // and hides a visible tip (study S1).
                crate::app_tooltips::on_mouse_move(state);
```

(b) `MouseInput` arm — after the cursor-hide block (currently lines 2215-2217):

```rust
                if state.use_software_cursor() {
                    state.window.set_cursor_visible(false);
                }
```

append:

```rust
                // Any button press/release kills a visible tooltip + pending
                // timer (all buttons incl. middle — study S1).
                crate::app_tooltips::on_button_event(state);
```

(c) Per-frame pump — in `render_frame`, after its first line (currently):

```rust
    fn render_frame(state: &mut AppState, event_loop: &ActiveEventLoop) -> Result<()> {
        state.frame_timer.sample(Instant::now());
```

append:

```rust
        let main_menu_shell_live = state.screen == GameScreen::MainMenu
            && !state.main_menu_shell_failed
            && !state.main_menu_show_skirmish_setup
            && !Self::single_player_shell_active(state)
            && !Self::native_skirmish_shell_active(state);
        crate::app_tooltips::update(state, main_menu_shell_live);
```

**Step 5: Verify.** `cargo check -p vera20k` clean; `cargo test -p vera20k` green. (No
visible change yet — render lands next task. NOTE: no visibility edit is needed for
`tooltip_csf_key_for_control` — it is defined `pub fn` in
`src/ui/main_menu_shell/state.rs:111` and re-exported by `src/ui/main_menu_shell/mod.rs:11-14`;
`app_main_menu_shell_render.rs` merely `use`-imports it, which is NOT a re-export path.)

**Step 6: Commit.** `app: A4 T3 — tooltip driver (event feed, sidebar+shell region sync, wall-clock poll)`

### Task 19 (A4 T4): in-game tooltip draw — buffers + pass order

**Why:** Tooltips draw LAST before the cursor (study O10).

**Files:**
- Modify: `src/app_render/mod.rs`, `src/app_render/draw_passes.rs`

**Step 1: uploads.** In `src/app_render/mod.rs`, the sidebar upload block currently ends
(lines 244-245):

```rust
    pool.upload(&state.gpu, "sidebar_cameo_overlay", &sidebar.cameo_overlay);
    pool.upload(&state.gpu, "sidebar_text", &sidebar.text);
```

Append after the `sidebar_text` upload:

```rust
    // A4 in-game tooltip: fill (darken texture) + text (GAME.FNT atlas),
    // drawn after the chat overlay and before the software cursor (O10).
    let (tooltip_fill, tooltip_text) = crate::app_tooltips::build_tooltip_instances(state);
    pool.upload(&state.gpu, "tooltip_fill", &tooltip_fill);
    pool.upload(&state.gpu, "tooltip_text", &tooltip_text);
```

**Step 2: draw order.** In `src/app_render/draw_passes.rs`, the tail of the UI pass
currently reads (lines 516-529):

```rust
    draw_pooled_ui(
        &mut pass,
        &state.batch_renderer,
        pool,
        Some(state.bit_font.atlas()),
        "sidebar_text",
    );
    draw_pooled_ui(
        &mut pass,
        &state.batch_renderer,
        pool,
        current_software_cursor_texture(state),
        "software_cursor",
    );
```

Insert between the two calls:

```rust
    draw_pooled_ui(
        &mut pass,
        &state.batch_renderer,
        pool,
        state.bit_font.darken_texture(),
        "tooltip_fill",
    );
    draw_pooled_ui(
        &mut pass,
        &state.batch_renderer,
        pool,
        Some(state.bit_font.atlas()),
        "tooltip_text",
    );
```

(`darken_texture()` returns `Option<&BatchTexture>` — same shape as the
`sidebar_cameo_overlay` call above it; match that call's texture-argument handling exactly.)

**Step 3: Verify.** Run the game: hover the repair button WITHOUT moving for 1 s → a dark
box with yellow "repair" CSF text appears near the cursor; hover a cameo → name + $cost
after 1 s; moving hides + restarts; any click kills it; after 10 s still it auto-hides;
hovering a tab shows NOTHING (deferred CSF mapping — expected).

**Step 4: Commit.** `render: A4 T4 — in-game tooltip draw (box+text buffers before cursor)`

### Task 20 (A4 T5): main-menu shell tooltip gated by the service

**Why:** Study D-B2 — the 0xE2 tooltip is emitted with zero delay today; gamemd waits
1000 ms and kills on click.

**Files:**
- Modify: `src/app_main_menu_shell_render.rs`

**Step 1:** In `main_menu_paint_labels` (app_main_menu_shell_render.rs:105-164), the
tooltip block currently reads (lines 155-162):

```rust
    if let Some(id) = hovered_button {
        out.push(PaintLabel {
            text: resolve_csf(state, tooltip_csf_key_for_control(id)),
            rect: layout.tooltip_line,
            align: ShellAlign::H_CENTER,
            rgb: SHELL_TEXT_RGB_ENABLED,
        });
    }
```

Replace with:

```rust
    // Tooltip text now comes from the shared service (study S1): it appears
    // only after the 1000 ms hover delay, hides on move, and is killed by any
    // button press — replacing the immediate-on-hover emission (D-B2).
    if let Some(tip) = state
        .tooltips
        .active()
        .filter(|t| (t.id & crate::app_tooltips::SHELL_TIP_NAMESPACE) != 0)
    {
        out.push(PaintLabel {
            text: &tip.text,
            rect: layout.tooltip_line,
            align: ShellAlign::H_CENTER,
            rgb: SHELL_TEXT_RGB_ENABLED,
        });
    }
```

**Step 2:** In the same function's signature, rename the now-unused parameter
`hovered_button: Option<MainMenuControlId>` to `_hovered_button: Option<MainMenuControlId>`
(callers unchanged).

**Step 3: Verify.** `cargo check -p vera20k`; run the game: on the main menu, hovering a
button shows its tooltip line only after ~1 s of stillness; moving between buttons restarts
the delay; clicking hides it. The hover-flash square wave (0x100 shell) is untouched.

**Step 4: Commit.** `app: A4 T5 — main-menu shell tooltip gated by the 1000 ms service`

### Task 21 (A4 gate): full-suite + behavior gate

**Step 1:** `cargo test -p vera20k` → ALL green incl. the 87-test net.

**Step 2:** Manual: in-game repair/sell/cameo tips (delay, move-hide, click-kill, 10 s
auto-hide, inclusive-edge feel on button borders); main-menu delay; no tooltip on tabs/
scroll (deferred); no tooltip anywhere on the skirmish board (out of scope, unchanged).

**Step 3:** Commit only if fixups were needed:
`test(ui): A4 gate — shared tooltip service verified in-game + shell`

### Task 22 (A5 T1): parse `IncomingMessage` + `MessageDelay`

**Why:** The message-insert sound and the chat-message lifetime are the only INI inputs A5
needs (ini lane §2); neither is parsed today.

**Files:**
- Modify: `src/rules/ruleset.rs`

**Step 1: fields.** In `GeneralRules`, after the `gui_tab_sound` field added in Task 14,
insert:

```rust
    /// Message-insert sound from [AudioVisual] IncomingMessage (retail
    /// `MessageText`). Plays on every non-silent message-list insert.
    pub incoming_message_sound: Option<String>,
    /// Chat/system message lifetime in MINUTES from [AudioVisual]
    /// MessageDelay (retail `.6`). The exact native minutes→ticks binding is
    /// untraced (plan deferred item); the driver converts minutes→ms.
    pub message_delay_minutes: f32,
```

**Step 2: parse.** After the `gui_tab_sound:` initializer added in Task 14, insert:

```rust
            incoming_message_sound: audio_visual
                .and_then(|s| s.get("IncomingMessage"))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            message_delay_minutes: audio_visual
                .and_then(|s| s.get_f32("MessageDelay"))
                .unwrap_or(0.6),
```

(`get_f32` is the same accessor the adjacent `DirectRockingCoefficient` parse at
ruleset.rs:997-1000 uses; 0.6 is the retail default at ini/rulesmd.ini:758.)

**Step 3: Verify.** `cargo check -p vera20k` clean. (Same no-fixture-harness note as
Task 14 — values are exercised by the Task 24 consumer + slice gate.)

**Step 4: Commit.** `rules: A5 T1 — parse IncomingMessage + MessageDelay`

### Task 23 (A5 T2): the `MessageList` model

**Why:** The contract §4.1-4.3 surface: 14 slots, prefix+":", px-budget fit with word break,
evict-head, tail insert, 19 px restack, silent wrap recursion, pause-aware expiry.

**Files:**
- Create: `src/ui/messages.rs`
- Modify: `src/ui/mod.rs`

**Step 1: declare.** In `src/ui/mod.rs`, after `pub mod main_menu_shell;`, insert:

```rust
pub mod messages;
```

**Step 2: create `src/ui/messages.rs`:**

```rust
//! Chat/system message list model (study §3.1 / contract §4): a bounded list
//! of text-label rows over the tactical viewport. Pure: text measurement is
//! injected (`measure: &dyn Fn(&str) -> i32`, pixels) so the model never
//! touches the renderer; deadlines are injected `now_ms` values. The app
//! driver owns the clock and MUST feed pause-adjusted time (`PauseAwareClock`
//! below): the native composite timer FREEZES during pause (contract §4.2
//! step 8 / §4.3), so a row's remaining lifetime survives a pause intact.
//!
//! ## Dependency rules
//! - ui/ module: std only — no render/, assets/, sidebar/, audio/, net/.

/// Slot-pool cap (native list walks up to 0xE label slots).
pub const MESSAGE_SLOTS: usize = 14;
/// Per-slot text cap (162 wide chars incl. the terminator).
pub const MESSAGE_SLOT_CHARS: usize = 161;
/// Per-line char cap handed to the fitter (native cap 0x6F — contract §4.2 step 4).
pub const MESSAGE_LINE_FIT_CHARS: usize = 111;
/// Pixel pad subtracted from the width budget (native `budget = MaxWidth −
/// prefix_width − 8`).
pub const MESSAGE_WIDTH_PAD_PX: i32 = 8;
/// Row stride (native Init hardcodes 0x13 = 19 px — NOT the font height).
pub const MESSAGE_LINE_HEIGHT_PX: i32 = 19;
/// Prefix separator (native wide ":" literal).
pub const MESSAGE_PREFIX_SEPARATOR: &str = ":";
/// Retail visible-message cap (Init maxMsg=6; clamped ≤ MESSAGE_SLOTS).
pub const MESSAGE_MAX_VISIBLE_RETAIL: usize = 6;

/// One live message row.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub text: String,
    pub rgb: [f32; 3],
    /// None = never expires (native timeout −1 → deadline 0).
    pub deadline_ms: Option<u64>,
    /// Screen Y, restacked after every insert/expiry.
    pub y: i32,
}

/// Outcome of an `add_message` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddOutcome {
    /// Number of rows added (≥1 when the text wrapped).
    pub added: usize,
    /// True when the caller should play the insert sound (top-level
    /// non-silent adds only; wrapped continuation rows are always silent).
    pub play_sound: bool,
}

#[derive(Debug)]
pub struct MessageList {
    x: i32,
    y: i32,
    max_visible: usize,
    max_width_px: i32,
    messages: Vec<Message>,
}

impl MessageList {
    pub fn new(x: i32, y: i32, max_visible: usize, max_width_px: i32) -> Self {
        Self {
            x,
            y,
            max_visible: max_visible.min(MESSAGE_SLOTS),
            max_width_px,
            messages: Vec::with_capacity(MESSAGE_SLOTS),
        }
    }

    /// Re-anchor on viewport/layout change (the native viewport re-init,
    /// contract §4.1): x = tactical_x + 3, y = tactical_y, width = tactical_w − 14.
    pub fn set_view(&mut self, x: i32, y: i32, max_width_px: i32) {
        self.x = x;
        self.y = y;
        self.max_width_px = max_width_px;
        self.restack();
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    /// Rows in walk order = insertion order = top-to-bottom (G20 analogue on
    /// the list's own walk).
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Contract §4.2: compose prefix+":"+fitted text, evict the head when
    /// full, tail-insert, restack, recurse silently on the remainder.
    pub fn add_message(
        &mut self,
        prefix: Option<&str>,
        text: &str,
        rgb: [f32; 3],
        timeout_ms: Option<u64>,
        silent: bool,
        now_ms: u64,
        measure: &dyn Fn(&str) -> i32,
    ) -> AddOutcome {
        let mut outcome = AddOutcome {
            added: 0,
            play_sound: false,
        };
        self.add_inner(prefix, text, rgb, timeout_ms, silent, now_ms, measure, &mut outcome);
        outcome
    }

    #[allow(clippy::too_many_arguments)]
    fn add_inner(
        &mut self,
        prefix: Option<&str>,
        text: &str,
        rgb: [f32; 3],
        timeout_ms: Option<u64>,
        silent: bool,
        now_ms: u64,
        measure: &dyn Fn(&str) -> i32,
        outcome: &mut AddOutcome,
    ) {
        if text.is_empty() {
            return;
        }
        // Step 2 — compose the prefix.
        let compose_prefix = match prefix {
            Some(p) => format!("{p}{MESSAGE_PREFIX_SEPARATOR}"),
            None => String::new(),
        };
        // Step 3 — pixel budget for the text part.
        let budget = self.max_width_px - measure(&compose_prefix) - MESSAGE_WIDTH_PAD_PX;
        if budget <= 0 {
            return;
        }
        // Step 4 — fit (char cap + px budget, word break).
        let fit_bytes = fit_prefix_bytes(text, budget, MESSAGE_LINE_FIT_CHARS, measure);
        if fit_bytes == 0 {
            return;
        }
        let mut line = compose_prefix;
        line.push_str(&text[..fit_bytes]);
        let line: String = line.chars().take(MESSAGE_SLOT_CHARS).collect();
        // Step 5 — evict the oldest while at the visible cap.
        while self.messages.len() >= self.max_visible {
            self.messages.remove(0);
        }
        // Slot-pool guard (unreachable with retail max_visible ≤ 14).
        if self.messages.len() >= MESSAGE_SLOTS {
            return;
        }
        // Steps 6-12 — tail insert + restack.
        self.messages.push(Message {
            text: line,
            rgb,
            deadline_ms: timeout_ms.map(|t| now_ms + t),
            y: 0,
        });
        self.restack();
        outcome.added += 1;
        if !silent {
            outcome.play_sound = true;
        }
        // Step 13 — wrap recursion: skip control chars after the break, then
        // re-add the remainder with the SAME prefix, always silent.
        if fit_bytes < text.len() {
            let remainder = text[fit_bytes..].trim_start_matches(|c: char| (c as u32) < 0x20);
            if !remainder.is_empty() {
                self.add_inner(prefix, remainder, rgb, timeout_ms, true, now_ms, measure, outcome);
            }
        }
    }

    /// Contract §4.3 — expiry: remove rows whose deadline passed strictly
    /// (`now > deadline`), then restack. Returns true when anything expired.
    /// `now_ms` must come from the pause-adjusted clock (`PauseAwareClock`):
    /// the native composite timer freezes during pause, so deadlines resume
    /// with their remaining lifetime intact. The driver additionally skips
    /// this call entirely while paused.
    pub fn manage(&mut self, now_ms: u64) -> bool {
        let before = self.messages.len();
        self.messages
            .retain(|m| m.deadline_ms.is_none_or(|d| now_ms <= d));
        let expired = self.messages.len() != before;
        if expired {
            self.restack();
        }
        expired
    }

    fn restack(&mut self) {
        let base = self.y;
        for (i, m) in self.messages.iter_mut().enumerate() {
            m.y = base + i as i32 * MESSAGE_LINE_HEIGHT_PX;
        }
    }
}

/// Greedy px-budget fitter with word break (contract lane §4.2 step-4 fitter
/// shape, doc fidelity — exact boundary semantics are a plan deferred item):
/// fits chars while BOTH the char cap and the pixel budget hold; on overflow
/// backs up to just-after the last space when one exists. Returns the fitted
/// BYTE length.
fn fit_prefix_bytes(
    text: &str,
    budget_px: i32,
    max_chars: usize,
    measure: &dyn Fn(&str) -> i32,
) -> usize {
    let mut fitted = 0usize;
    let mut chars = 0usize;
    let mut last_break: Option<usize> = None;
    for (idx, ch) in text.char_indices() {
        let end = idx + ch.len_utf8();
        if chars + 1 > max_chars || measure(&text[..end]) > budget_px {
            return last_break.unwrap_or(idx);
        }
        if ch == ' ' {
            last_break = Some(end);
        }
        fitted = end;
        chars += 1;
    }
    fitted
}

/// Pause-aware message clock (contract §4.2 step 8 / §4.3): the native
/// composite timer FREEZES during pause, so message `now` must stop too —
/// otherwise a deadline that elapses on the wall clock during a pause would
/// expire its row the instant the game unpauses, instead of resuming the
/// remaining lifetime. Pure: wall-clock ms are injected; the app driver feeds
/// pause edges via `set_paused`.
#[derive(Debug, Default)]
pub struct PauseAwareClock {
    /// Sum of all completed pause spans, in wall ms.
    paused_total_ms: u64,
    /// Wall timestamp when the current pause began (None = not paused).
    pause_started_wall_ms: Option<u64>,
}

impl PauseAwareClock {
    /// Feed the live pause flag once per frame; edges are detected here.
    pub fn set_paused(&mut self, paused: bool, wall_ms: u64) {
        match (paused, self.pause_started_wall_ms) {
            (true, None) => self.pause_started_wall_ms = Some(wall_ms),
            (false, Some(started)) => {
                self.paused_total_ms += wall_ms.saturating_sub(started);
                self.pause_started_wall_ms = None;
            }
            _ => {}
        }
    }

    /// Pause-adjusted now: wall ms minus every paused span. Constant (frozen)
    /// for the duration of a pause.
    pub fn now(&self, wall_ms: u64) -> u64 {
        let in_pause = self
            .pause_started_wall_ms
            .map(|s| wall_ms.saturating_sub(s))
            .unwrap_or(0);
        wall_ms.saturating_sub(self.paused_total_ms + in_pause)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 10 px per char monospace fake measure.
    fn mono(s: &str) -> i32 {
        s.chars().count() as i32 * 10
    }

    const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

    fn list(max_visible: usize, width: i32) -> MessageList {
        MessageList::new(3, 0, max_visible, width)
    }

    #[test]
    fn add_composes_prefix_and_restacks_19px() {
        let mut l = list(6, 500);
        let o = l.add_message(Some("Boris"), "attack", WHITE, None, false, 0, &mono);
        assert_eq!(o.added, 1);
        assert!(o.play_sound);
        let o = l.add_message(None, "second", WHITE, None, true, 0, &mono);
        assert!(!o.play_sound, "silent add suppresses the sound");
        let rows = l.messages();
        assert_eq!(rows[0].text, "Boris:attack");
        assert_eq!(rows[1].text, "second");
        assert_eq!(rows[0].y, 0);
        assert_eq!(rows[1].y, MESSAGE_LINE_HEIGHT_PX, "19 px stride");
    }

    #[test]
    fn eviction_drops_oldest_at_cap() {
        let mut l = list(3, 500);
        for i in 0..4 {
            l.add_message(None, &format!("m{i}"), WHITE, None, true, 0, &mono);
        }
        let texts: Vec<&str> = l.messages().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, vec!["m1", "m2", "m3"], "head (oldest) evicted");
        assert_eq!(l.messages()[0].y, 0, "restacked from the top");
    }

    #[test]
    fn wrap_recursion_reincludes_prefix_and_is_silent() {
        // Budget: width 200 − prefix "P:" 20 − pad 8 = 172 px → 17 chars/line.
        let mut l = list(6, 200);
        let text = "aaaa bbbb cccc dddd eeee"; // 24 chars → must wrap
        let o = l.add_message(Some("P"), text, WHITE, None, false, 0, &mono);
        assert!(o.added >= 2, "wrapped into multiple rows");
        assert!(o.play_sound, "ONE sound for the top-level add");
        for row in l.messages() {
            assert!(row.text.starts_with("P:"), "prefix re-included on wraps: {}", row.text);
            assert!(mono(&row.text) <= 200 - MESSAGE_WIDTH_PAD_PX + 20);
        }
        // All input chars survive across the rows (minus break spaces).
        let joined: String = l
            .messages()
            .iter()
            .map(|m| m.text.trim_start_matches("P:"))
            .collect::<Vec<_>>()
            .join(" ");
        let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(normalize(&joined), normalize(text));
    }

    #[test]
    fn word_break_backs_up_to_last_space() {
        // budget 100 px = 10 chars; "hello worlds" breaks after "hello ".
        let fit = fit_prefix_bytes("hello worlds", 100, 111, &mono);
        assert_eq!(&"hello worlds"[..fit], "hello ");
    }

    #[test]
    fn zero_budget_or_unfittable_adds_nothing() {
        let mut l = list(6, 5); // budget 5−0−8 < 0
        let o = l.add_message(None, "text", WHITE, None, false, 0, &mono);
        assert_eq!(o.added, 0);
        assert!(!o.play_sound);
        // First char wider than the budget → fit 0 → nothing.
        let mut l2 = list(6, 17); // budget 17−0−8 = 9 < 10
        let o = l2.add_message(None, "x", WHITE, None, false, 0, &mono);
        assert_eq!(o.added, 0);
    }

    #[test]
    fn manage_expires_strictly_after_deadline() {
        let mut l = list(6, 500);
        l.add_message(None, "temp", WHITE, Some(1000), true, 0, &mono);
        l.add_message(None, "forever", WHITE, None, true, 0, &mono);
        assert!(!l.manage(1000), "now == deadline: kept (strict >)");
        assert!(l.manage(1001), "now > deadline: expired");
        let texts: Vec<&str> = l.messages().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, vec!["forever"], "0/None = never expires");
        assert_eq!(l.messages()[0].y, 0, "restacked after expiry");
    }

    #[test]
    fn pause_freezes_deadline_arithmetic() {
        // Contract §4.2 step 8 / §4.3: the native composite timer FREEZES
        // during pause. Post(timeout 4000) at t=0, pause wall 1000..11000:
        // the row must survive until pause-ADJUSTED now exceeds 4000 (i.e.
        // wall > 14000), NOT expire on unpause.
        let mut l = list(6, 500);
        let mut clock = PauseAwareClock::default();
        l.add_message(None, "temp", WHITE, Some(4000), true, clock.now(0), &mono);
        clock.set_paused(true, 1000);
        assert_eq!(clock.now(5000), 1000, "clock frozen mid-pause");
        clock.set_paused(false, 11000);
        assert_eq!(clock.now(11000), 1000, "10 s pause span subtracted");
        assert!(!l.manage(clock.now(12000)), "wall 12000 → adjusted 2000: kept");
        assert!(!l.manage(clock.now(14000)), "adjusted 4000 == deadline: kept (strict >)");
        assert!(l.manage(clock.now(14001)), "adjusted 4001 > deadline: expired");
    }

    #[test]
    fn set_view_reanchors_rows() {
        let mut l = list(6, 500);
        l.add_message(None, "a", WHITE, None, true, 0, &mono);
        l.add_message(None, "b", WHITE, None, true, 0, &mono);
        l.set_view(10, 100, 400);
        assert_eq!(l.messages()[0].y, 100);
        assert_eq!(l.messages()[1].y, 100 + MESSAGE_LINE_HEIGHT_PX);
        assert_eq!(l.x(), 10);
    }

    #[test]
    fn line_char_cap_111_applies() {
        let mut l = list(14, 100_000);
        let long = "y".repeat(300);
        let o = l.add_message(None, &long, WHITE, None, true, 0, &mono);
        assert!(o.added >= 2);
        assert_eq!(
            l.messages()[0].text.chars().count(),
            MESSAGE_LINE_FIT_CHARS,
            "first row capped at 111 chars"
        );
    }
}
```

**Step 3: Verify.** `cargo test -p vera20k ui::messages` → 9 tests pass.

**Step 4: Commit.** `ui: A5 T2 — MessageList model (14 slots, wrap, evict, expiry)`

### Task 24 (A5 T3): message driver + draw; mission announcements through the list

**Why:** Gives the surface a producer (gamemd's TriggerAction text IS an Add_Message
caller) and the gamemd draw position; retires the egui banner.

**Files:**
- Create: `src/app_messages.rs`
- Modify: `src/lib.rs`, `src/app.rs`, `src/app_sim_tick.rs`, `src/app_transitions.rs`,
  `src/app_render/mod.rs`, `src/app_render/draw_passes.rs`

**Step 1: declare.** In `src/lib.rs`, after `pub mod app_tooltips;`, add:

```rust
pub mod app_messages;
```

**Step 2: create `src/app_messages.rs`:**

```rust
//! Chat/system message surface driver (study §3.1): anchors the
//! `ui::messages::MessageList` to the tactical viewport (x+3 / y / w−14),
//! posts system messages (insert sound = [AudioVisual] IncomingMessage),
//! expires rows per frame against a pause-FROZEN clock (contract §4.2 step 8
//! / §4.3: deadlines resume with remaining lifetime intact after a pause),
//! and builds the text instances drawn between the sidebar text and the
//! tooltip (study O10: chat before tooltip).
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::render::SpriteInstance;
use crate::ui::game_screen::GameScreen;
use crate::ui::messages::MESSAGE_MAX_VISIBLE_RETAIL;

/// Native Init anchors: x = tactical_x + 3, y = tactical_y, w = tactical_w − 14.
const MESSAGE_X_INSET: i32 = 3;
const MESSAGE_WIDTH_INSET: i32 = 14;
/// Interim system-message color (native rows use a color-scheme index whose
/// mapping is a plan deferred item).
const MESSAGE_RGB_SYSTEM: [f32; 3] = [1.0, 1.0, 1.0];
/// Mission/trigger text lifetime — preserves the pre-A5 banner's 4 s
/// (the native trigger-text timeout is untraced; deferred item).
const MISSION_TEXT_TIMEOUT_MS: u64 = 4_000;

/// Pause-adjusted message `now` (contract §4.2 step 8 / §4.3): the wall clock
/// minus every paused span. ALL message deadlines and expiry checks use this
/// clock — never the raw `app_tooltips::now_ms` — so a pause freezes the
/// remaining lifetime of every visible row.
pub(crate) fn message_now_ms(state: &AppState) -> u64 {
    state
        .message_clock
        .now(crate::app_tooltips::now_ms(state))
}

/// Post a system message (mission/trigger text, future house notifications).
pub(crate) fn post_system_message(state: &mut AppState, text: &str) {
    sync_view(state);
    let now = message_now_ms(state);
    let font = &state.bit_font;
    let measure = |s: &str| font.text_width(s) as i32;
    let outcome = state.message_list.add_message(
        None,
        text,
        MESSAGE_RGB_SYSTEM,
        Some(MISSION_TEXT_TIMEOUT_MS),
        false,
        now,
        &measure,
    );
    if outcome.play_sound {
        let sound = state
            .rules
            .as_ref()
            .and_then(|r| r.general.incoming_message_sound.clone());
        crate::app::App::play_shell_ui_sound_by_id(state, sound.as_deref());
    }
}

/// Per-frame: feed the pause edge into the clock, then (unpaused, in-game)
/// re-anchor to the live viewport and expire rows against the FROZEN clock.
/// While paused the clock accumulates the span and `manage` is skipped — both
/// halves are required: skipping alone would let wall-time deadlines expire
/// the instant the game unpauses.
pub(crate) fn update(state: &mut AppState) {
    if state.screen != GameScreen::InGame {
        return;
    }
    let wall = crate::app_tooltips::now_ms(state);
    state.message_clock.set_paused(state.paused, wall);
    if state.paused {
        return;
    }
    sync_view(state);
    let now = message_now_ms(state);
    state.message_list.manage(now);
}

fn sync_view(state: &mut AppState) {
    // Tactical viewport = render area minus the sidebar panel width.
    let tactical_w =
        state.render_width() as i32 - state.sidebar_layout_spec.sidebar_width.round() as i32;
    state.message_list.set_view(
        MESSAGE_X_INSET,
        0,
        (tactical_w - MESSAGE_WIDTH_INSET).max(0),
    );
}

/// Text instances for the "message_text" pooled buffer (GAME.FNT atlas).
pub(crate) fn build_message_text_instances(state: &AppState) -> Vec<SpriteInstance> {
    if state.screen != GameScreen::InGame {
        return Vec::new();
    }
    let font = &state.bit_font;
    let x = state.message_list.x() as f32;
    state
        .message_list
        .messages()
        .iter()
        .flat_map(|m| {
            crate::render::sidebar_text::build_text(
                font,
                &m.text,
                x,
                m.y as f32,
                1.0,
                0.00022,
                m.rgb,
                [0.0, 0.0],
            )
        })
        .collect()
}
```

**Step 3: AppState field.** In `src/app.rs`, append after the `tooltip_epoch` field (Task 18):

```rust
    /// In-game chat/system message surface (study §3.1) — re-anchored to the
    /// tactical viewport per frame by `app_messages`.
    pub(crate) message_list: crate::ui::messages::MessageList,
    /// Pause-adjusted clock for message deadlines (contract §4.2 step 8 /
    /// §4.3: the native composite timer freezes during pause). Fed pause
    /// edges by `app_messages::update`.
    pub(crate) message_clock: crate::ui::messages::PauseAwareClock,
```

ctor (after `tooltip_epoch: Instant::now(),`):

```rust
            message_list: crate::ui::messages::MessageList::new(
                3,
                0,
                crate::ui::messages::MESSAGE_MAX_VISIBLE_RETAIL,
                0,
            ),
            message_clock: crate::ui::messages::PauseAwareClock::default(),
```

(`MESSAGE_MAX_VISIBLE_RETAIL` import in app_messages.rs covers the driver; the ctor uses the
full path. The width re-syncs on first use.)

**Step 4: per-frame hook.** In `render_frame` (app.rs), the Task 18 insertion ends with
`crate::app_tooltips::update(state, main_menu_shell_live);` — append after it:

```rust
        crate::app_messages::update(state);
```

**Step 5: reroute MissionAnnouncement + delete the banner.** Remove the two AppState fields
(app.rs:315-318):

```rust
    /// Transient mission/script announcement shown in-game.
    pub(crate) mission_announcement: Option<String>,
    /// Absolute deadline for clearing the announcement banner.
    pub(crate) mission_announcement_deadline: Option<Instant>,
```

and their ctor initializers; then fix every consumer (compile errors enumerate them):

(a) `src/app_sim_tick.rs:846-850` currently:

```rust
            TriggerEffect::MissionAnnouncement { text } => {
                state.mission_announcement = Some(text.clone());
                state.mission_announcement_deadline =
                    Some(Instant::now() + std::time::Duration::from_secs(4));
            }
```

becomes:

```rust
            TriggerEffect::MissionAnnouncement { text } => {
                // gamemd routes trigger text through the message list
                // (contract lane §4.5: the native trigger-text path is a
                // message-list producer).
                crate::app_messages::post_system_message(state, text);
            }
```

(b) `src/app_sim_tick.rs:169-174` — delete the deadline-expiry block:

```rust
        if let Some(deadline) = state.mission_announcement_deadline {
            if Instant::now() >= deadline {
                state.mission_announcement = None;
                state.mission_announcement_deadline = None;
            }
        }
```

(c) `src/app.rs:2925-2927` — delete the banner draw:

```rust
                if let Some(text) = state.mission_announcement.as_deref() {
                    crate::ui::mission_status::draw_mission_banner(&state.egui.ctx, text);
                }
```

(d) `src/app_transitions.rs:166-167` — delete the two `mission_announcement*` reset lines
(quote them from the file when editing; they null the two fields on screen transitions).

(e) `src/ui/mission_status.rs` — delete the now-unused `draw_mission_banner` function (keep
`draw_mission_result_screen`); remove any stale import.

**Step 6: uploads + draw order.** In `src/app_render/mod.rs`, the Task 19 insertion
currently reads:

```rust
    // A4 in-game tooltip: fill (darken texture) + text (GAME.FNT atlas),
    // drawn after the chat overlay and before the software cursor (O10).
    let (tooltip_fill, tooltip_text) = crate::app_tooltips::build_tooltip_instances(state);
```

Insert BEFORE that comment block:

```rust
    // A5 chat/system message lines (GAME.FNT atlas) — chat draws before the
    // tooltip (O10).
    let message_text = crate::app_messages::build_message_text_instances(state);
    pool.upload(&state.gpu, "message_text", &message_text);
```

In `src/app_render/draw_passes.rs`, the Task 19 insertion starts with the `"tooltip_fill"`
draw call — insert BEFORE it:

```rust
    draw_pooled_ui(
        &mut pass,
        &state.batch_renderer,
        pool,
        Some(state.bit_font.atlas()),
        "message_text",
    );
```

**Step 7: Verify.** `cargo test -p vera20k` green (includes the
`pause_freezes_deadline_arithmetic` model test from Task 23). Run a map with a text trigger
(or call `post_system_message` from a debug hotkey temporarily — remove before commit): the
text appears top-left over the tactical view, white, 19 px rows, wraps when long, plays
MessageText, disappears after 4 s, freezes while paused — AND a message posted ~1 s before
pausing still shows for ~3 s after a long (>4 s) pause ends, instead of vanishing on unpause
(the pause-frozen deadline arithmetic). The egui banner is gone.

**Step 8: Commit.** `app: A5 T3 — message surface driver + draw; mission announcements through the list`

### Task 25 (A5 gate): full-suite + behavior gate

**Step 1:** `cargo test -p vera20k` → ALL green incl. the 87-test net.

**Step 2:** Manual: trigger text renders as message rows (position/wrap/sound/expiry/pause
freeze); messages never block clicks (click a unit under a message row — selection works:
the rows are not in any gadget list and have no hit path, the structural equivalent of the
native mask-0 w=h=1 labels, with the masked early-out itself covered by the A0 G15 tests).

**Step 3:** Commit only if fixups were needed:
`test(ui): A5 gate — message surface verified (insert, wrap, expiry, sound)`

### Task 26 (D-B3): exit-confirm Esc routed through the controller

**Why:** Esc currently bypasses `DialogController` and never pops the 0x120 instance
(app.rs:2106-2112 → close_main_menu_dialogs 1950-1955); open clobbers the stack via
`ensure_active`. Fix = true push on open + controller-routed pop on every close path
(keyboard Esc AND mouse OK/Cancel), mirroring the validation-modal pattern (the only
existing `pop()` caller, app.rs:1339-1359).

**Files:**
- Modify: `src/app.rs`, `src/ui/shell/controller.rs` (tests only)

**Step 1: push on open.** `open_exit_confirm_modal` currently ends (app.rs:1934-1940):

```rust
        Self::ensure_skirmish_shell_chrome(state);
        // Host the modal on the shared shell controller stack (0x120 over the menu's
        // 0xE2) so its OK/Cancel buttons own the press-must-match-release gesture.
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x0120), true);
        state.exit_confirm_modal = Some(modal);
```

Replace with:

```rust
        Self::ensure_skirmish_shell_chrome(state);
        // Host the modal as a TRUE LIFO push over the active shell (D-B3):
        // teardown pops back to it with focus restored. (ensure_active would
        // reset_to-clobber the stack — the prior "0x120 over 0xE2" comment
        // described behavior that never happened.)
        if state.shell_controller.top_id() != Some(crate::ui::shell::descriptor::DialogId(0x0120)) {
            state
                .shell_controller
                .push(crate::ui::shell::descriptor::DialogId(0x0120), true);
        }
        state.exit_confirm_modal = Some(modal);
```

**Step 2: one controller-routed close.** Insert next to `close_main_menu_dialogs`
(app.rs:1949-1955):

```rust
    /// Controller-routed exit-confirm teardown (D-B3): dismiss the modal UI
    /// state, then LIFO-pop its 0x120 instance so focus returns to the shell
    /// beneath. Mirrors `close_validation_modal_from_controller` — every Esc
    /// and mouse close path converges here.
    fn close_exit_confirm_modal_from_controller(state: &mut AppState) {
        state.exit_confirm_modal = None;
        if state.shell_controller.top_id() == Some(crate::ui::shell::descriptor::DialogId(0x0120)) {
            state.shell_controller.pop();
        }
    }

    fn route_exit_confirm_modal_key(state: &mut AppState, key: ShellKey) -> bool {
        if state.exit_confirm_modal.is_none() {
            return false;
        }
        if !state.shell_controller.on_key(key) {
            return false;
        }
        Self::close_exit_confirm_modal_from_controller(state);
        state.window.request_redraw();
        true
    }
```

Also update the doc comment on `close_main_menu_dialogs` from
`/// Close every open main-menu modal dialog (e.g. on ESC).` to
`/// Close the egui-only main-menu dialogs (options/movies/campaign — never on
/// the controller stack). The exit-confirm modal closes through
/// close_exit_confirm_modal_from_controller (D-B3).` — its body is unchanged
(nulling `exit_confirm_modal` stays as a harmless belt-and-braces for the egui fallback
path, which never pushes 0x120).

**Step 3: reroute keyboard Esc.** The bypass currently reads (app.rs:2103-2112):

```rust
                    // A main-menu modal dialog (exit confirm, options, movies,
                    // campaign select) takes ESC first: close it and stay,
                    // never propagating to the shell-close handlers below.
                    if Self::main_menu_dialog_open(state) {
                        if is_escape {
                            Self::close_main_menu_dialogs(state);
                            state.window.request_redraw();
                        }
                        return;
                    }
```

Replace with:

```rust
                    // A main-menu modal dialog (exit confirm, options, movies,
                    // campaign select) takes ESC first: close it and stay,
                    // never propagating to the shell-close handlers below.
                    // The exit-confirm modal routes through the controller
                    // (on_key → LIFO pop, D-B3); the egui-only dialogs are
                    // not on the stack and keep the direct close.
                    if Self::main_menu_dialog_open(state) {
                        if is_escape {
                            if state.exit_confirm_modal.is_some() {
                                if !Self::route_exit_confirm_modal_key(state, ShellKey::Escape) {
                                    // Defensive: on_key only fails with an
                                    // empty route — still close consistently.
                                    Self::close_exit_confirm_modal_from_controller(state);
                                    state.window.request_redraw();
                                }
                            } else {
                                Self::close_main_menu_dialogs(state);
                                state.window.request_redraw();
                            }
                        }
                        return;
                    }
```

(`ShellKey` is already in scope in app.rs — `route_validation_modal_key` uses it.)

**Step 4: mouse paths pop too.** In `handle_exit_confirm_modal_mouse_up`
(app.rs:1702-1727), the two result arms currently read:

```rust
            Some(id) if id == crate::ui::shell::modal::control::OK => {
                Self::persist_settings_on_quit(state);
                state.exit_confirm_modal = None;
                Self::start_quit_cascade(state);
            }
            // Cancel (control 2) -> stay; close the modal.
            Some(id) if id == crate::ui::shell::modal::control::CANCEL => {
                Self::close_main_menu_dialogs(state);
                state.window.request_redraw();
            }
```

Replace with:

```rust
            Some(id) if id == crate::ui::shell::modal::control::OK => {
                Self::persist_settings_on_quit(state);
                Self::close_exit_confirm_modal_from_controller(state);
                Self::start_quit_cascade(state);
            }
            // Cancel (control 2) -> stay; close the modal via the controller
            // pop (D-B3) so mouse and Esc converge on the same teardown.
            Some(id) if id == crate::ui::shell::modal::control::CANCEL => {
                Self::close_exit_confirm_modal_from_controller(state);
                state.window.request_redraw();
            }
```

Additionally, both `handle_exit_confirm_modal_mouse_down` (app.rs:1692-1700) and
`handle_exit_confirm_modal_mouse_up` (app.rs:1702-1709) currently call

```rust
        state
            .shell_controller
            .ensure_active(crate::ui::shell::descriptor::DialogId(0x0120), true);
```

— with the modal now PUSHED at open, `ensure_active` would clobber the stack on any
mismatch. Replace that statement in BOTH functions with the guard:

```rust
        if state.shell_controller.top_id() != Some(crate::ui::shell::descriptor::DialogId(0x0120)) {
            return;
        }
```

**Step 5: controller LIFO test.** In `src/ui/shell/controller.rs`'s `#[cfg(test)]` module
(which already defines `const A: DialogId = DialogId(0x00E2);`), add:

```rust
    #[test]
    fn push_over_base_pops_back_to_it() {
        let modal = DialogId(0x0120);
        let mut c = DialogController::default();
        c.ensure_active(A, false);
        c.push(modal, true);
        assert_eq!(c.top_id(), Some(modal));
        assert_eq!(c.kbd_route(), &[modal], "accepts_keys registers the route");
        assert_eq!(c.pop(), Some(modal));
        assert_eq!(c.top_id(), Some(A), "LIFO pop restores the shell beneath");
        assert!(c.kbd_route().is_empty(), "route pruned on pop");
    }
```

**Step 6: Verify.** `cargo test -p vera20k shell` green (incl. the new test). Manual: main
menu → Exit → Esc cancels (menu stays interactive, hover/press still work); Exit → Cancel
click same; Exit → OK quits with the cascade; pressing other keys while the modal is open
is still swallowed.

**Step 7: Commit.** `ui: D-B3 — exit-confirm Esc routed through DialogController (true push/pop LIFO)`

### Task 27 (D-B3 gate)

**Step 1:** `cargo test -p vera20k` → ALL green incl. the 87-test net.

**Step 2:** Commit only if fixups were needed:
`test(ui): D-B3 gate — controller-routed exit-confirm verified`

### Task 28 (R1): delete the dead `in_game_hud.rs`

**Why:** Zero callers confirmed (study §7 R1; rust lane §7: the only references are the
definition, an internal egui id string, and the mod decl).

**Files:**
- Delete: `src/ui/in_game_hud.rs`
- Modify: `src/ui/mod.rs`

**Step 1:** Delete the file `src/ui/in_game_hud.rs` (210 lines).

**Step 2:** In `src/ui/mod.rs`, remove the line:

```rust
pub mod in_game_hud;
```

**Step 3: Verify.** `cargo check -p vera20k` clean, then `cargo test -p vera20k` → ALL
green (full-suite gate for the R1 slice). Confirm with
`grep -r "in_game_hud" src/` → zero hits.

**Step 4: Commit.** `ui: R1 — delete dead in_game_hud.rs (zero callers)`

### Task 29: end-to-end verification vs gamemd.exe

**Why:** Confirm the shipped slices against the original's observable behavior before
declaring the plan done. No commit — findings go to the user.

**Verify (side-by-side with gamemd.exe where available, otherwise against the study
contract):**
1. **Sidebar click feel (G22):** press-hold any sidebar button in gamemd → pressed art,
   nothing fires; drag off → art pops, release does nothing; release inside → action +
   (tabs) sound. Our build must be indistinguishable on all three gestures.
2. **Scroll pair (G23):** hold a scroll button in gamemd → exactly one page; right-click
   scrolls; identical here. (Also read the R-UP/R-DN SHP frame counts from the retail
   side MIX while at it — closes the deferred frame-count item.)
3. **Tooltips (S1):** stopwatch the gamemd sidebar tooltip delay (~1 s) and auto-hide
   (~10 s); verify move-restart and click-kill; compare our box CONTENT (name/$cost) —
   box ART is a known interim (deferred).
4. **Messages (§3.1):** trigger a map text in gamemd (campaign/skirmish trigger) → top-left
   list line + MessageText sound; compare position, wrap, lifetime.
5. **Esc (D-B3):** gamemd quit-confirm Esc cancels and returns focus to the menu; ours
   pops to the same state.
6. **Ghidra spot-check (LOW-confidence closure):** decompile the SidebarClass::AI tab-arm
   sound call to confirm GUITabSound is the tab-click key (Parity table row, Task 14).
7. Re-run `cargo test -p vera20k` one final time and read the literal `test result:` lines.

## Sources & References

- **Design/study doc:** `docs/research/GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
  (§4.2 gap table, §5 G/O/D/S clauses, §6 boundary design, §7 retire list, §8 slices, §9 YELLOW).
- **Grounding lanes (2026-06-10):**
  `docs/research/substrate/worknotes/gadget-dialog-20260610/plan-grounding-contract.md`
  (G-clause pseudocode, A1 Kind/mask table, tooltip + message contracts, MCP call log),
  `.../plan-grounding-rust.md` (worktree-verified anchors @ 7b79a186),
  `.../plan-grounding-ini.md` (INI keys, sound events, asset gaps).
- **gamemd.exe addresses** (kept HERE, never in Rust code/comments — code cites study clause
  IDs only): GadgetClass::Input 0x004E1640; Hit_Test 0x004E15A0 (seed consts
  0x007F5BE8/0x007F5BF4); Clicked_On 0x004E13F0; Sticky_Process 0x004E1970; base Action
  0x004E1530; ControlClass::Action 0x0048E5A0; ToggleClass::Action 0x00723EC0 / ctor
  0x00723E60 / Turn_On 0x00723EA0 / Turn_Off 0x00723EB0; Set_Focus/Clear_Focus
  0x004E19A0/0x004E19D0; SidebarClass::Init_IO 0x006A5310 (Kind/mask identities — VERIFIED-LIVE
  this study); SidebarClass::AI 0x006A7780 (ID consumption, scroll IsPressed clears
  0x00B0B354/0x00B0B434); button records 0x00B07C48 (tabs), 0x00B0B3A0 (repair 0x65),
  0x00B07DF8 (sell 0x66), 0x00B0B328 (0xC9), 0x00B0B408 (0xC8); ToolTipManager [0x00887368],
  ctor 0x00724000 (1000/10000 ms), ProcessMessage 0x00724200, Register 0x00724580,
  Unregister 0x00724730, Show 0x00724AD0, GetText chain 0x00479050 → 0x006D1800 →
  SidebarClass::GetTooltipText 0x006AC210 → GetCameoTooltip 0x006A92E0;
  MessageListClass Add_Message 0x005D3BA0 / Manage 0x005D4430 / Draw 0x005D49A0 / Add_Edit
  0x005D4210; TextLabelClass ctor 0x0072A440 / Draw_Me 0x0072A4A0; Set_View_Dimensions
  0x004A8960 (message Init args); RulesClass AudioVisual parse site 0x0066A7BF..0x0066A80B
  (+0x6AC IncomingMessage, +0x6C4 MessageCharTyped); beacon timeout literal 0xE1 in
  RadarClass__PlaceBeacon 0x00430BA0.
- **INI keys:** `[AudioVisual]` GUITabSound (rulesmd.ini:645), IncomingMessage (:683),
  MessageCharTyped (:684 — deferred), MessageDelay (:758), GUIMainButtonSound (:643, parsed),
  ScoldSound (:698 — out of scope), GUIBuildSound (:644 — A2); soundmd.ini events
  [MenuTab]:2930, [MessageText]:2959, [TextBleep]:3028; `UIName=` on techno sections
  (e.g. rulesmd.ini:6604).
- **Related code (worktree anchors, all verified this session):** app_input.rs:34-43/210-318,
  sidebar/mod.rs:52-64/127-257/284-333/379-425, sidebar/sidebar_view.rs:111-175,
  sidebar/gadget_flash.rs:112-181, app_sidebar_render.rs:29-169, app_sidebar_build.rs:33-208,
  render/sidebar_chrome.rs:60-100/280-340, ui/shell/controller.rs:1-213, ui/shell/geom.rs:12-37,
  app.rs:215-219/1339-1359/1527-1538/1602-1615/1640-1727/1848-1861/1928-1955/2067-2311/
  2569-2570/2678-2679/2897-2927, app_sim_tick.rs:160-230/839-859, app_render/mod.rs:215-246,
  app_render/draw_passes.rs:495-530, rules/ruleset.rs:191-314/960-1014,
  rules/object_type.rs:147-170/861-870, app_main_menu_shell_render.rs:91-164,
  ui/mission_status.rs:1-30, app_sidebar_gadgets.rs (pattern), render/sidebar_text.rs:28-56,
  render/shell_paint.rs:97-102/328-339.
- **Prior docs (DOC-INHERITED inputs):** `SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`
  (its §5.3 scroll-repeat mechanism is REFUTED by G23),
  `TOOLTIP_TEXT_SOURCE_AND_DELAY_TIMERS_GHIDRA_REPORT.md`,
  `TOOLTIP_GLYPH_RASTER_LINE_WRAPPING_GHIDRA_REPORT.md`, `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`,
  `SELL_REPAIR_TAB_SCROLL_EXACT_GADGET_RECTS_GHIDRA_REPORT.md`,
  `RETAIL_SOVIET_SIDEBAR_SHP_DIMENSIONS_OFFSETS_GHIDRA_REPORT.md`,
  `SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`.
- **Prior commits:** shell substrate slices (geom e1b50ec4, descriptor 21d3341a, controller
  71b9a3de, paint 32f066f0, modal d355d495..54de2fd3) — the pattern this plan's slices mirror.

## Self-review (skill 11-point checklist)

1. **Spec coverage** — every prompt-scoped item maps to tasks: A0→T1-T8 (all named G-clauses
   tested), A1→T9-T15, A4→T16-T21, A5→T22-T25, D-B3→T26-T27, R1→T28; out-of-scope items
   (A2/A3/A6, Slice 4/5b, R11) are explicitly excluded. FIXED during review: added the
   right-release `0xC000`/`~0x4000` scroll test (T5/T12) which the first draft omitted.
2. **Placeholder scan** — no TBD/TODO. One deliberate non-literal step remains and is
   flagged inline as such: T16 step 4's test body says "mirror the adjacent existing
   pattern at the named site" because the exact fixture-helper shape was not read this
   session; it names the precise anchor to copy from. (T10 step 2's atlas wiring was made
   fully literal by the 2026-06-10 plan-review corrections.) Everything else is complete
   code.
3. **Architecture check** — ui/ modules are std-only (measure/clock injected); drivers are
   flat app_* modules; sim untouched (checklist above); no new crate deps.
4. **Interface ordering** — types (T1-T3) before behaviors (T4-T7) before drivers (T11-T12);
   service models (T17/T23) before drivers (T18/T24); parses (T14/T16/T22) before/with their
   consumers.
5. **Risk coverage** — A1 flip has clause tests + a 6-point manual gate; banner removal
   enumerates all consumers; 87-test net checked at every gate.
6. **Self-containment** — every modified site is quoted verbatim from the worktree with
   file:line; new files are full source. FIXED during review: added the
   `ensure_active`→guard replacement in BOTH exit-confirm mouse handlers (the first draft
   only fixed the Esc path, which would have let a mouse gesture clobber the pushed stack).
7. **Sim/ compliance** — no sim behavior/logic change; the only sim-path edits are the two
   `cfg(test)` fixture literals gaining `ui_name: None,` (Task 16, named in the File Map);
   the checklist states it explicitly.
8. **Grounding coverage** — docs (study + 3 lanes), binary addresses (Sources), repo
   patterns (controller tests, orchestrate helper, Ready-strip darken quad, pooled buffers),
   INI keys (4 parses + their rulesmd.ini line numbers) all cited.
9. **Confidence tagging** — every Key Technical Decision carries confidence + source;
   5 LOW-confidence decisions flagged for /review-plan (scroll placement, R-UP/R-DN frames,
   GUITabSound mapping, cameo tip format, tooltip box visuals) + 1 medium cadence decision.
10. **Deferred questions** — 11 items listed with their UNK citations; conservative
    stand-ins (empty tab/scroll tip text, interim formats) are explicit, never silent.
11. **Parity-critical items** — 16 rows, each with task #, player-visible why, and a
    verification path; the LOW-confidence sound mapping has a named Ghidra follow-up.

## Plan-review corrections (2026-06-10)

Pre-execution review per `/review-plan`; all must-fix findings patched in place. Every fix
below was re-verified against the worktree (`<local>/Documents/ra2-uigadget-worktree`
@ 7b79a186) and/or the study + grounding-contract lane before editing — no invented
constants; unknowns stayed in Deferred Open Questions.

- **C-PR1 (Task 13 — retiring the probes broke two existing tests).** `hit_test_routes_repair_button`
  (sidebar/mod.rs:511-542) and `hit_test_routes_sell_button` (sidebar/mod.rs:544-572) assert the
  OLD press-path routing and would fail Task 13's own `cargo test -p vera20k sidebar` gate
  (re-verified in the worktree `#[cfg(test)]` module). **Fix:** new Task 13 Step 2 deletes both
  tests outright (substrate coverage already exists in the G22 state-table tests + the Task 12
  fired-ID mapping); the third `hit_test` test (sidebar_view.rs:593, cancel button →
  `CancelLastBuild`) stays on the legacy path and is explicitly left untouched. Steps renumbered.

- **C-PR2 (Task 16 — `ui_name` field collides with the Sim Checklist / Risk Areas STOP rule).**
  Adding the field forces `ui_name: None,` into the full-field `ObjectType` literals at
  `src/sim/movement/locomotor_tests.rs:12` and `src/sim/movement/teleport_movement.rs:387`
  (re-verified: the only two `ObjectType { .. }` literal sites in the crate; neither uses
  `..Default`; the helper at teleport_movement.rs:592 builds on :387 and needs no edit) — but the
  plan said "no file under src/sim/ is modified" and told the executor to STOP on sim compile
  failures. **Fix:** both files named explicitly in Task 16 Step 3, Task 16's Files list, and the
  File Map (test-fixture-only rows); Sim Checklist amended to "no sim behavior/logic change; two
  cfg(test) fixture literals gain `ui_name: None,`"; Risk Areas parallel-sessions bullet now
  carves out this ONE scheduled exception (any other sim failure still means STOP).

- **C-PR3 (Task 18 Step 5 — wrong anchor for `tooltip_csf_key_for_control`).** The function is NOT
  defined in `app_main_menu_shell_render.rs` — it is already `pub fn` in
  `src/ui/main_menu_shell/state.rs:111` and re-exported via `src/ui/main_menu_shell/mod.rs:11-14`;
  app_main_menu_shell_render.rs only `use`-imports it (:18-21, called at :157), and a private
  `use` is not a re-export, so the instructed visibility edit was unexecutable and the driver's
  `crate::app_main_menu_shell_render::...` call path would not resolve (re-verified by grep in the
  worktree). **Fix:** Step 5 deleted (steps renumbered, a note explains why no visibility edit
  exists); `sync_main_menu_regions` now calls
  `crate::ui::main_menu_shell::tooltip_csf_key_for_control(b.id)`;
  `src/app_main_menu_shell_render.rs` removed from Task 18's Files list (it remains a Task 20
  file).

- **C-PR4 (Task 10 Step 2 — atlas pipeline needs THREE coordinated edits, plan covered one).**
  The chrome atlas sizes itself as the SUM of all `all_entries` heights (sidebar_chrome.rs:437-441),
  so blitting an unregistered entry writes past the atlas bounds; packing is a body-level
  `blit_entry` loop advancing a shared `y` cursor (:512-527), not a per-field conversion
  expression as the plan implied. **Fix:** Step 2 rewritten with full literal code for all
  three sites plus the field assignment: (a) render loop after the sell loop (:328), (b)
  `all_entries` push loops next to the repair/sell pushes (:417-426 pattern), (c) two
  `scroll_*_frames_packed` blit loops next to `repair_frames_packed`/`sell_frames_packed`
  (:512-527), (d) `scroll_up_frames`/`scroll_down_frames` assignments in the struct literal
  (:581, after :601). All line anchors re-verified in the worktree.

- **C-PR5 (File Map omitted two Task 24 files).** Task 24 steps (d)/(e) modify
  `src/app_transitions.rs` (the two `mission_announcement*` reset lines — re-verified present at
  :166-167) and `src/ui/mission_status.rs` (`draw_mission_banner` — re-verified at :6), violating
  the plan's own file-ownership contract. **Fix:** both rows added to the File Map with A5 T3
  responsibilities.

- **C-PR6 (Task 12 read `gui_tab_sound` before Task 14 created it).** The field does not exist at
  baseline 7b79a186 (re-verified: zero grep hits), so Task 12's "cargo test → green" gate was
  unsatisfiable (E0609) and Task 13 inherited the breakage. **Fix:** the sound block is removed
  from Task 12's `apply_gadget_result` (tab arm now only selects the tab; an inline NOTE explains
  the deliberate silence); Task 14 retitled "+ wire the tab-click sound", gains
  `src/app_gadget_input.rs` in its Files list and a new Step 3 that replaces the tab arm with the
  sound-playing version — parse and consumer land in the same gate-green task. Per-task green-gate
  discipline restored.

- **C-PR7 (A5 message deadlines ran on the wall clock through pauses).** Contract §4.2 step 8 /
  §4.3 pin message `now` to a pause-aware 16 ms composite timer that FREEZES during pause; the
  plan computed deadlines from the wall-clock `tooltip_epoch` and merely skipped `manage()` while
  paused — on unpause, any deadline that elapsed during the pause would expire instantly instead
  of resuming its remaining lifetime (player-visible whenever a pause overlaps a visible message).
  **Fix:** new pure `ui::messages::PauseAwareClock` (accumulates pause spans, subtracts them from
  injected wall ms; frozen while paused) + AppState `message_clock` field;
  `app_messages::post_system_message`/`update` now use `message_now_ms` (pause-adjusted) and feed
  pause edges per frame; new model test `pause_freezes_deadline_arithmetic` (post timeout-4000 at
  t=0, pause wall 1000..11000, row survives until adjusted now > 4000); Key Decision bullet
  rewritten to state the clock-freeze requirement (the old "skipping manage reproduces the
  freeze" claim covered only the visual freeze, not the deadline arithmetic); Task 23 module/
  `manage` doc comments, Task 24 driver docs, the Task 24 Step 7 manual check, and the Interface
  Changes lists updated to match.

- **C-PR8 (binary symbols/offsets/FUN_ names in Rust code comments).** Violated
  `feedback_no_engine_refs_in_comments` and the plan's own Sources-section promise ("kept HERE,
  never in Rust code/comments"). **Fix:** all code blocks scrubbed — `FUN_00433F50` →
  "contract lane §4.2 step-4 fitter"; `IsPressed +0x2C / IsOn +0x2D / Kind +0x30` → "the G22
  pressed/on/kind triple (identities cited in plan Sources)"; `+0x228 → +0x22C` → "contract lane
  §3.4 hover hook"; the `+0x2D` tab-mirror comment → "externally driven latch-ON mirror, study
  §2.5 / G22 Kind 2"; `g_StickyFocus`/`g_KeyboardFocus`/`g_HoveredGadget`/`g_CurrentGadgetList`,
  `SidebarClass::Init_IO`, `TriggerAction__Execute`, `ToggleClass`/`GadgetClass::`/
  `ControlClass::Action`, `Sticky_Process`/`Hit_Test`/`Clicked_On`/`Set_Focus`/`Clear_Focus`/
  `Draw_Me`/`Clear_Attached_List`/`Turn_On`/`Turn_Off`/`Mouse_Leave`/`Flag_To_Redraw`/
  `Peer_Callback`/`ToolTipManager`/`ShowAt`/`ShowPos`/`InitSurface`/`Set_View_Dimensions`/
  `fit_chars` and bare `IsPressed`/`IsOn` casings all replaced with study clause IDs,
  grounding-lane § refs, or our own field names (incl. three test assert-message strings). All
  addresses/symbols remain ONLY in the plan's Sources section; plain "gamemd" words without
  symbols/addresses were left (pre-existing precedent in gadget_flash.rs). Prose sections of the
  plan (Grounding Summary, Open Questions, Parity table, Sources) are not code and keep their
  citations.
