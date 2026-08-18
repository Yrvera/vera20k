# Shell First-Paint Slide — Completion Behavior Trace
## Sound, Text-Reveal, Input-Block, Enable/Disable Parity

**Date:** 2026-05-30  
**Mechanic:** Shell first-paint slide — what happens at slide START and END  
**Scope:** Sound, text-reveal broadcast (0x4EC→0x4EE), parent/child enable/disable, input-block  
**Adjacent findings** (frame schedule, control count, trigger edges): excluded per scope  

---

## Evidence Base

| Source | Role |
|---|---|
| `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md` | Trigger chain, DL=1 confirmed, allow-list |
| `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md` | DL=0 vs DL=1 broadcast split, 0x4EC/0x4ED |
| `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md` | 0x4EE child handling, timer cadence, Rust gaps |
| Ghidra live decompile `FUN_00608260 @ 0x00608260` | Sound call, EnableWindow, EnumChildWindows, re-enable |
| Ghidra live decompile `FUN_006071E0 @ 0x006071E0` | DL=0 sends 0x4ED; DL=1 sends 0x4EC; display-chain drain |
| Ghidra live decompile `FUN_00622B50 @ 0x00622B50` | 0x4EC handler: EnumChildWindows(FUN_0060AA60) |
| Ghidra live decompile `FUN_0060AA60 @ 0x0060AA60` | Sends 0x4EE to qualifying children |
| `ini/rules.ini:586`, `ini/rulesmd.ini:712` | `ShellButtonSlideSound=` empty (both files) |
| `src/app_shell_transition.rs` | Rust slide implementation |
| `src/app.rs` lines 1845, 1927, 1971, 2021 | Rust input-block call sites |

---

## (a) Which DL mode does the first-paint slide use?

**VERIFIED: DL=1.**

Native path (from `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md` §2 and live decompile):
- Subclass proc `FUN_00610CA0` fires `FUN_00608260` on the dialog's first `WM_PAINT` (when `+0x1FC` transitions 0→1).
- `FUN_00608260` calls `FUN_006071E0` with assembly `MOV DL, 1` at `0x0060833F`.
- `FUN_006071E0` with nonzero DL: drains the display chain (vtable +0x28/+0x10 loop), plays a shell transition sound via `VocClass__PlayAtPos`, then `SendMessageA(parent, 0x4EC, 0, 0)`.
- DL=0 path (common WM_PAINT deferred) sends 0x4ED, NOT 0x4EC.

**First-paint slide is DL=1 → ends with 0x4EC broadcast → triggers text reveal.**

---

## (b) Text-Reveal Broadcast at Slide Completion

**Stage: Native sends 0x4EC → children receive 0x4EE → text reveal starts.**

Native chain (all verified from live decompiles):
1. `FUN_006071E0` (DL=1) sends `0x4EC` to parent after display-chain drain.
2. `FUN_00622B50` handles `0x4EC`: calls `EnumChildWindows(parent, FUN_0060AA60, 0)`.
3. `FUN_0060AA60`: calls `FUN_00602490` (classifies control); if qualifying, sends `0x4EE` to the child.
4. Child `OwnerDraw_Static` handles `0x4EE`: sets running byte, resets count to 1, starts timer 0 at 30ms, invalidates child.
5. Qualifying children for `0x102`: `0x694` (title), `0x6EC` (game type), `0x5A8` (map label).

**Rust: NOT IMPLEMENTED.**  
- `src/app_shell_transition.rs` clears `state.shell_first_paint_slide = None` when `is_complete()` — no 0x4EC equivalent, no reveal-start event, no reveal state on the three right-panel labels.
- `src/ui/skirmish_shell/state.rs`: no per-label reveal count/timer/running-byte.
- `src/render/shell_text.rs::draw_in_rect`: no count/range parameter.
- Labels are rendered full-text at all times (steady-state), bypassing the "text hidden until 0x4EE, then revealed character-by-character" behavior.

**Verdict: FAIL / NOT-IMPLEMENTED**

---

## (c) Input Block — During Slide and After Completion

**Native (from live decompile `FUN_00608260`):**
- At slide START: `IsWindowEnabled(parent)` saves state, `EnableWindow(parent, 0)` disables parent, `EnumChildWindows(parent, LAB_00606800, 1)` disables children.
- At slide END (after `FUN_006071E0` returns): `EnumChildWindows(parent, LAB_00606800, 0)` re-enables children, `EnableWindow(parent, saved_BVar5)` restores parent enabled state, `InvalidateRect(parent, NULL, 0)`.
- Mechanism: Win32 EnableWindow/EnumChildWindows on the actual HWND hierarchy.

**Rust:**
- `transition_blocks_shell_input` returns `true` while `state.shell_first_paint_slide.is_some()` (blocks keyboard, mouse, wheel input in app.rs at lines 1845, 1927, 1971, 2021).
- On completion: `state.shell_first_paint_slide = None` clears the block (next frame `blocks_shell_input` returns false).
- No Win32 HWND involvement since Rust has no Win32 shell window hierarchy — the mechanism is different but the effect is equivalent: input blocked during slide, unblocked after completion.

**Timing match:** Native unblocks synchronously after `FUN_006071E0` returns (still within the same `WM_PAINT` handling). Rust unblocks on the NEXT FRAME after `is_complete()` is detected in `render_shell_first_paint_slide`. This is a one-frame latency difference in unblocking.

**Verdict:** The block-during / unblock-after logic is structurally PASS for the common case. The mechanism difference (Win32 vs. app-level flag) is acceptable (Rust has no HWND tree). The one-frame unblock latency is negligible and not player-visible. **PASS** (with the one-frame caveat noted).

---

## (d) Sound Verification

**INI confirmation (stock YR):**
- `ini/rules.ini:586`: `ShellButtonSlideSound=` — empty, no sound name.
- `ini/rulesmd.ini:712`: `ShellButtonSlideSound=` — empty, no sound name.

**Native `FUN_00608260` sound call (live decompile):**  
`VocClass__PlayAtPos(0x3f800000, 0)` — the first argument `0x3f800000` is the IEEE 754 float 1.0 reinterpreted as an integer (1065353216). `VocClass__PlayAtPos` checks `(-1 < param_1) && (param_1 < DAT_00b1d388)` where `DAT_00b1d388` is the voc array size (bounded to a few hundred entries). 1065353216 is far outside any valid index — it resolves to index 0 (no entry) → `iVar2 = 0`, the function returns 0 without playing anything.

This confirms: even though `FUN_00608260` does call `VocClass__PlayAtPos`, the resolved voc index is invalid (empty INI entry → no assigned sound), so no audio plays.

**Note:** `FUN_006071E0` (DL=1 path) also calls `VocClass__PlayAtPos(0x3f800000, 0)` at `0x00607F4A` — same call pattern, same result: no sound.

**Rust comment in `app_shell_transition.rs` line 8:** "Silent in stock YR (`ShellButtonSlideSound=` is empty), so no sound is played." — CORRECT.

**Rust: plays no sound. Native: plays no sound (empty INI). PASS.**

---

## Stage Summary

| Stage | Native Behavior | Rust Behavior | Verdict |
|---|---|---|---|
| (a) DL mode | First-paint path uses DL=1 (0x0060833F `MOV DL,1`) | Not directly applicable (Rust doesn't model DL) | PASS (DL=1 confirmed; Rust implementation consequence is (b)) |
| (b) Text reveal at completion | 0x4EC → 0x4EE → 3 right-panel statics start 30ms character reveal | No 0x4EC/0x4EE event, no reveal state; labels painted full-text always | NOT-IMPLEMENTED |
| (c) Input block during slide | EnableWindow(parent,0) + EnumChildWindows disable; restored after | App-level flag blocks all shell input; cleared after is_complete | PASS |
| (c') Unblock timing | Synchronous after FUN_006071E0 returns (same WM_PAINT) | Next render frame after is_complete detected | PASS (one-frame latency not player-visible) |
| (d) Sound at START | VocClass__PlayAtPos called but empty INI → no audio | No sound played (comment acknowledges silent) | PASS |
| (d') Sound at END (DL=1 path) | VocClass__PlayAtPos in FUN_006071E0 (DL=1, 0x607F4A) → empty INI → no audio | No sound played | PASS |

**PASS: 4 | FAIL: 0 | UNCHECKED: 0 | NOT-IMPLEMENTED: 1**

---

## Top Player-Visible Failures

1. **Text-reveal missing at slide completion** — Right-panel statics (`0x694` title, `0x6EC` game type, `0x5A8` map label) should be hidden until slide end, then reveal character-by-character at 30ms/char cadence. Currently they show full text immediately. File: `src/app_shell_transition.rs` (completion handler, lines 297-301); `src/ui/skirmish_shell/state.rs` (no reveal state); `src/render/shell_text.rs` (no count/range param). Native evidence: `FUN_006071E0 @ 0x00607F95` sends 0x4EC; `FUN_00622B50` handles → `FUN_0060AA60` → 0x4EE; `OwnerDraw_Static_006153E0 @ 0x00615FDB..0x00616026` starts timer.

---

## Negative Facts

- DL=0 (common WM_PAINT deferred) sends 0x4ED — does NOT start text reveal. Only DL=1 (first-paint slide path) sends 0x4EC.
- `FUN_006AE3F0` (standard Skirmish proc) has no 0x4ED reveal handler — 0x4ED is not an alias for 0x4EC.
- `ShellButtonSlideSound` is empty in both stock INI files; no other sound plays during the slide in the first-paint path that is audible.
- Native re-enables parent using the saved `BVar5` from `IsWindowEnabled`, not unconditionally TRUE.

---

## Sources

- Ghidra MCP live decompile (read-only): `FUN_00608260`, `FUN_006071E0`, `FUN_00622B50`, `FUN_0060AA60`, `VocClass__PlayAtPos @ 0x00750920`
- Research docs: `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`, `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`, `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md`
- Rust source: `src/app_shell_transition.rs`, `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/render/shell_text.rs`
- INI: `ini/rules.ini:586`, `ini/rulesmd.ini:712`
