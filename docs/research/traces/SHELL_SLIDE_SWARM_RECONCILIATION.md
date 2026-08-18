# Trace-Swarm Reconciliation — Shell First-Paint Slide Parity

**Date:** 2026-05-30
**Slots:** 5/5 COMPLETE (0 failed). Reports: SHELL_SLIDE_SDBTNANM_FRAME_SCHEDULE_TRACE.md, SHELL_SLIDE_TICK_SCHEDULE_FORMULA_TRACE.md, SHELL_SLIDE_CONTROL_ENUM_GROUP_SPLIT_TRACE.md, SHELL_SLIDE_COMPLETION_TEXTREVEAL_SOUND_TRACE.md, SHELL_SLIDE_TRIGGER_ENTRY_EDGE_ONESHOT_TRACE.md
**Parent verification:** re-decompiled `FUN_006071E0`, `FUN_00608CD0`, `FUN_0060A180`, `FUN_00608260`; re-read Rust `src/app_shell_transition.rs` and `src/app_skirmish_shell_render.rs:280-331`.

## Headline: a shared mis-model inflated 3 of the numeric FAILs

All three numeric slots (1, 2, 3) assumed **Rust `slot_count` == native `N_A`** (the animated-control count). It does not. Verified from the binary:

- `N_A` (native Group-A column count) = controls where `(GetWindowLongA(ctrl,GWL_STYLE) & 0xB) == 0xB` (BS_OWNERDRAW **button**) AND `FUN_00608CD0`==true AND `IsWindowVisible`. Source: `FUN_0060A180 @ 0x0060A180`. **Statics are excluded** (not buttons) and the **Back button is excluded** (it is `FUN_00609730`/Group-B, drawn by the separate `cStack_175` block).
- Rust `slot_count` = animated buttons **including Back** (Skirmish=3: Start/ChooseMap/Back).
- Native schedule total = `N_A + 9`; Rust total = `slot_count + 8`. These are equal iff `slot_count == N_A + 1`.

So the "+8 vs +9" is **not a uniform off-by-one bug** — it is two equivalent parameterizations that agree when there is exactly one Group-B/Back button folded into Rust's count. This must be evaluated per dialog, which the slots did not do.

## Verified native mechanism (authoritative, from this pass)

- `FUN_006071E0` builds schedule `new[(N_A+3)*4]`, fills indices `0..N_A-1` = `1..N_A`, sets `[N_A]=0`, `[N_A+1]=N_A+3`, `[N_A+2]=0`; max entry `= N_A+3`; `iStack_bc = max+6 = N_A+9`; the draw loop runs `N_A+9` iterations, `Sleep(0x1E)=30ms` each (verified `0x00607646..0x00607F11`).
- **Group A column** (the `local_14c` loop): per-control SDBTNANM frame, before=1 / base=5 / after=10 on slide-in (`iStack_13c=5`). Matches Rust `GROUP_A_IN`.
- **Group B column** (the `local_14c < iStack_148` loop, base=`iStack_114=11`): iterates **zero controls** for these dialogs → `base=11` is computed but never drawn.
- **Back / Group-B control** (`cStack_175` block, fires when `FUN_0060A250` found ≥1): drawn with `iStack_13c=5` — i.e. **Group-A frame params (base=5), not base=11**.
- **Completion:** slide-in path (`cVar14`-set, the DL=1 entry from `FUN_00608260`) drains the display chain and `SendMessage(parent, 0x4EC)`; the common proc broadcasts `0x4EE` to qualifying child statics → character-by-character text reveal. The DL=0 deferred-paint path sends `0x4ED` (no reveal).
- **Eligibility / count by dialog:** `FUN_00608CD0` for `0x102` returns true for **6 ids** (`0x694,0x468,0x6ec,0x5aa,0x5a8,0x617`), but only the BS_OWNERDRAW buttons among them feed `N_A`; the statics are revealed via the `0x4EC→0x4EE` text path, not the button column.

## Verdict reconciliation

### CONFIRMED — real disparities (survive parent re-verification)

1. **Text-reveal missing (slot 4).** Native fires `0x4EC→0x4EE` at slide end → skirmish right-panel statics reveal character-by-character; Rust renders them full-text instantly and has no `0x4EC/0x4EE`/reveal state. Corroborated by `SKIRMISH_FUN_006071E0_..._REPORT.md` and the `0x00607F95` send. Player-visible every skirmish/menu entry. **HIGH.**
2. **Campaign `0x94` slide NOT-IMPLEMENTED (slot 5).** Native dialog `0x94` (Main_Game case 8) is allow-listed and slides on first paint; Rust uses an egui overlay with no `ShellSlideKind::Campaign`. Every New-Campaign navigation. **HIGH (conditional frequency).**
3. **Movies `0x101`/`0x129` and Choose-Map `0x6B` slides NOT-IMPLEMENTED (slot 5).** Allow-listed native dialogs reachable in standard offline YR; Rust egui overlays, no slide. **MEDIUM (per-navigation).**

### NEEDS FOLLOW-UP — unresolved, do NOT action as stated

4. **Schedule total-ticks parity.** Not a uniform off-by-one. Equal when `slot_count == N_A+1`. Likely **matches** for skirmish (if native `N_A`=2 buttons → 11 == Rust 3+8=11); likely **off-by-one** for main menu (if `N_A`=6 with no Back fold → native 15 vs Rust 14). UNRESOLVED pending per-dialog owner-draw-button `N_A` counts (requires control-id→style typing from the dialog templates, not done this pass).

### FALSE POSITIVES — caught in reconciliation, do NOT fix

5. **"Skirmish animates 6, Rust 3" (slot 3) — OVERCOUNT.** `FUN_00608CD0`=6 includes statics; `FUN_0060A180` filters to owner-draw buttons (`style&0xB==0xB`). The 4 statics are not in the button column (they get text-reveal instead). The animated button count is ~3 (Start, ChooseMap, Back), matching Rust.
6. **"Back uses base=11 vs native base=5" (slot 3) — FALSE.** Rust draws all three right-panel buttons (incl. Back) via `ButtonGroup::A` (base=5); native draws Back via the `cStack_175` block at `base=5`. Match. Rust's `GROUP_B_IN`(base=11) is dead code in both — no visible disparity.

## Cross-trace pattern

Slots 1, 2, 3 inherited one wrong assumption (`slot_count == N_A`). Their precise numeric FAILs (N+8 vs N+9, "6 vs 3", "3 vs 4", Back ramp) are all downstream of it and were demoted. The surviving disparities (text-reveal, missing campaign/movies/choosemap slides) come from slots 4 and 5 and are independent of that error.

## Recommended follow-ups (user's call)

- **Settle the schedule/count parity properly:** enumerate, per allow-listed dialog Rust models (`0xE2`/`0x100`/`0x102`), which control ids are BS_OWNERDRAW buttons (`style&0xB==0xB`) vs statics, to compute native `N_A` and verify `slot_count == N_A+1`. Fix `total_ticks_for` only if a real per-dialog mismatch is confirmed (main menu is the prime suspect).
- **Implement the static text-reveal** (`0x4EC→0x4EE` equivalent) as a post-slide completion event on the right-panel statics — confirmed real and currently missing.
- **Decide scope for the missing legs** (campaign `0x94`, movies `0x101`/`0x129`, choose-map `0x6B`): these need their shell dialogs to exist in Rust before they can slide; track via `/gap-scan` rather than a slide-only fix.

## Pad-PASS / contract audit

- Slot 1 returned PASS:16/FAIL:1/UNCHECKED:0 — the all-but-one-PASS pattern was the tell; spot-check found its "all 12 constants match" PASS is technically true but missed that `base=11` is never applied (caught here).
- Slots 1–5 all honored the ≤150-line + report-path + tally contract; no read-only Ghidra violations observed.
