# Slice 5 sub-step 4b (+ 6) kickoff — paste into a fresh session

STATUS: sub-steps **1, 2, 4a DONE + committed on `dev`** (`d355d49` substrate, `b3d3923` 4a).
4a in-game verification is PENDING (see bottom). **START AT 4b.** Package `vera20k`. Parity
bar: indistinguishable from gamemd.exe.

GOAL (4b): on the quit-confirm **OK**, persist settings to **RA2MD.INI BEFORE teardown**, then
reproduce gamemd's graceful quit cascade (music stop → vox-pump wait → fade → exit). 4a currently
just calls `event_loop.exit()` on OK (in `handle_exit_confirm_modal_mouse_up`, `src/app.rs`);
4b replaces that immediate exit with the ordered cascade below. Then sub-step **6**.

SOURCE OF TRUTH — READ FIRST:
- `docs/plans/2026-06-01-shell-substrate-slice5-plan.md` — §4 (quit cascade), §5 (sub-steps), §8.1
  (sub-step 6 deletion set). Authoritative.
- `docs/plans/2026-06-01-shell-substrate-slice5-kickoff.md` — its **RESUME STATE** block has the 4a
  architecture (N-button `paint_modal_shp`, controller hosting, the egui-fallback deviation).
- 4a code to build on: `src/app.rs` (`handle_exit_confirm_modal_mouse_up`, `open_exit_confirm_modal`,
  `exit_confirm_modal_feed`), `src/app_main_menu_shell_render.rs` (`build_exit_confirm_modal_overlay`),
  `src/ui/shell/modal.rs`, `src/render/shell_paint.rs`.

---

## VERIFIED-FROM-BINARY FACTS — re-confirmed this session via Ghidra (cite when you depend on them)

### 4b — settings persist (all CONFIRMED)
- Writer: `OptionsClass__WriteToINI @0x005FAD10`. Filename **`RA2MD.INI`** (uppercase) — pointer
  `0x00826444` pushed into the CCFileClass ctor `0x004739F0` (`PUSH 0x826444` / `CALL 0x004739F0` at
  0x005FAD19/0x005FAD22).
- It writes **exactly four sections, in this code**: `[Options]` (str 0x008254DC), `[Video]`
  (0x00833160), `[Audio]` (0x008330B4), `[Network]` (0x00833060).
- **Port reality:** the engine does not yet model the full Options/Video/Audio/Network settings
  (the old egui path noted "Options write-back is not decoded yet; persist hook is a TODO"). So 4b's
  job is the **ORDER + the file/section contract**, not a full settings dump. Decide with the user:
  write the settings the port DOES track under the correct section names to `RA2MD.INI`, or land a
  clearly-marked persist hook with the right filename/sections that writes the tracked subset — but
  the **persist-strictly-before-teardown** order is the load-bearing parity fact and must hold.

### 4b — teardown cascade order (CONFIRMED in `Main_Game @0x0052D9A0`)
On OK (message-box result 0): case 6 sets state=7 and calls WriteToINI at `0x0052DE2F`
(`MOV ESI,7` at 0x0052DE2A precedes it); on Cancel state=0x12. The next `Main_Game` iteration runs
case 7 teardown **in this order**:
1. `FUN_00720EA0(1)` — music/theme stop **with fade** (0x0052E76B).
2. **Vox pump-wait loop** (0x0052E79C–0x0052E7D3): poll `VoxClass__PumpAndCheckActive @0x007529E0`
   each iteration (per-iter pump `FUN_0048D080`); **exit the instant it returns 0** (voices done —
   the primary, normal exit), bounded by a deadline of baseline `+ 0xBB8` GetRadarTimer ticks.
3. `FUN_00720EA0(0)` — immediate stop (0x0052E7EC).
4. `FUN_004A3C30(0)` — palette **screen fade**, fade param `0x1E` (30), GATED on `DAT_008175b0 == 0`
   (0x0052E805).
5. `XOR AL,AL` / `RET` → returns 0. **NO** `PostQuitMessage` / `ExitProcess` anywhere.

### 4b — vox-pump cap (CONFIRMED)
`GetRadarTimer @0x006C8C40 = timeGetTime() >> 4` (≈16 ms/tick). Deadline = baseline + `0xBB8`
(3000) ticks (`LEA EDI,[ESI+0xBB8]` @0x0052E796) ⇒ ~48 s wall-clock **ceiling**. Encode it as a
**tick budget gated on the voice-active check** (early-exit dominates), NOT a 3000 ms `sleep`.

> Port mapping: reproduce the **observable** cascade with the engine's own systems — audio
> stops, then a short wait for trailing voice/EVA lines (bounded, not blocking forever), then the
> screen fades, then the window closes. The fade and vox systems are RA2-engine concepts; match the
> player-visible result (audio silenced + fade-to-black before exit), not the C++ plumbing. The wait
> must be non-blocking against the frame loop. Likely worth a `/brainstorm` on how the port's audio +
> render-fade map to this before coding — and `/disparity-scan` of what audio/fade facilities exist.

### Sub-step 6 — anchors (CONFIRMED in current code; re-anchor by content, lines drift)
- `src/app_skirmish_shell_render/chrome.rs` **L321-323**: `modal_button_mnbttn_frame_index(pressed)`
  = `if pressed { 1 } else { 0 }` (the latent bug — pressed→1=disabled). Consumer
  `modal_button_mnbttn_entry` **L337-346** (`match … 1=>frame1, 2=>frame2, _=>frame0`).
- `src/app_skirmish_shell_render.rs` **L967+**: test
  `validation_modal_button_uses_mnbttn_normal_and_pressed_frames` asserts
  `modal_button_mnbttn_frame_index(true) == 1`. NOT in the safety net
  (`src/ui/skirmish_shell/state/tests.rs`) — this test WILL change to `== 2`.
- Sub-step 6 work: re-point `push_validation_modal_instances` (`src/app_skirmish_shell_render/modals.rs`)
  to the new N-button `paint_modal_shp`; route validation-modal input through `DialogController`; fix
  the pressed→2 mapping (or delegate to `shell_paint::modal_button_frame_index`, which is already
  0=up/1=disabled/2=pressed); update the render-side test. Apply the plan §8.1 deletion set in
  `app.rs` — **but those line citations have drifted** (4a edited `app.rs`); re-anchor by content first.

---

## USE WORKFLOWS (the user has opted in for this work)

Lean on the Workflow tool for the **fan-out / verification / review** phases; keep implementation
and the cargo build/test pass serial + foreground.
- **4b:** after implementing the cascade, run a small **adversarial review workflow** — independent
  agents check (a) persist strictly precedes teardown, (b) the tick-cap is gated on the voice-active
  check (not a fixed sleep), (c) the order music-stop → wait → fade → exit matches, (d) no
  blocking-the-frame-loop wait.
- **Sub-step 6:** run a **re-anchor workflow** first — parallel agents verify each §8.1 deletion-set
  target + the validation-modal migration points against the (4a-edited) `app.rs`, returning current
  line numbers/signatures — BEFORE editing. Then a review workflow after.
- **Do NOT** bury `cargo build -p vera20k` / `cargo test -p vera20k` inside a workflow — run them as a
  separate bounded **foreground** pass and read the literal `test result:` line.

## CADENCE (hard rules — unchanged)
- **ONE sub-step per cycle.** Sub-step 4b is meaty — consider sub-stepping it (persist hook+order
  first; then the audio-stop + bounded vox-wait + fade cascade).
- After each: `cargo build -p vera20k` then `cargo test -p vera20k` as a separate foreground pass.
- The skirmish safety net `src/ui/skirmish_shell/state/tests.rs` (**87 tests**) stays GREEN +
  UNCHANGED. If a sub-step needs to change it, STOP and re-scope.
- `sim/` never depends on `ui/`/`render/`.
- Build clean is necessary, not sufficient — **STOP for the user's in-game OK before committing**
  each sub-step to `dev`.

## 4a IN-GAME VERIFICATION — STILL PENDING (do early, it's already committed)
Launch → main menu → Exit. Confirm: (1) the box is the retail **PUDLGBGN** panel + yellow text (not
the old gray egui card); (2) OK/Cancel **sink to MNBTTN frame 2** while held; (3) OK quits, Cancel
and ESC stay; (4) body-text placement; and crucially (5) whether gamemd draws **anything in the gap
between OK and Cancel** — the 0x120 template's unpopulated `0x5AF` button at DLU (207,155,83,15),
which 4a does NOT render. If the original shows a blank button there, add it (it's a quick follow-up).
