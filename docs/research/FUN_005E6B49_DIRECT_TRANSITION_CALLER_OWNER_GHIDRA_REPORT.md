# FUN_005E6B49 Direct Transition Caller Owner - Ghidra Research Report

**Date:** 2026-05-27  
**Target:** `FUN_005E6B49_DIRECT_TRANSITION_CALLER_OWNER`  
**Address(es):** `0x005E6B49 -> FUN_00608260`, raw callback `0x005E6920..0x005E7041`, modal wrapper `FUN_005E68A0`, Skirmish handler `FUN_006ACEE0`, nested random-map helper `FUN_005E8590`, comparison caller `0x00612690`.  
**Scope:** recover the owner and semantics of the direct transition caller at `0x005E6B49`, determine active-YR liveness, and decide whether it explains the standard Single Player `0x100` Skirmish `0x579` route.  
**Non-scope:** full `FUN_006071E0` frame schedule, full random-map generator, full Choose Map visual composition, Rust implementation, Ghidra mutation.

## Summary

`0x005E6B49` is inside the raw WndProc-style callback `0x005E6920..0x005E7041` used by the standard offline Skirmish **Choose Map** dialog `0x6B`, not the main menu, not the Single Player shell `0x100`, and not dialog proc `0x0052D640`.

The exact path is:

`Skirmish 0x102 Choose Map button 0x5AA -> FUN_006ACEE0 -> hide 0x102 -> FUN_005E68A0 creates Choose Map dialog 0x6B -> Choose Map Create Random Map button 0x583 -> hide chooser -> FUN_005E8590 nested random-map setup -> if result != 1, return -1 -> 0x005E6B47/0x005E6B49 calls FUN_00608260 on the chooser HWND -> ShowWindow(chooser, 5)`.

So this caller is relevant to a real player-visible shell page transition, but it is a **different shell/menu path**: it re-shows the Choose Map modal after nested Create Random Map cancel/failure. It does not explain the standard Single Player `0x100` Skirmish `0x579 -> 0x0B -> 0x102` route.

Compared with `0x00612690`, `0x005E6B49` is simpler and better owned: it is a direct call inside a recovered Choose Map modal callback branch. `0x00612690` remains a conditional child-subclass paint/state path in dispatcher `0x00610CA0..0x006128FE`, gated by per-control `+0x1FC == 1`.

## Working Notes Gate

- **Target question:** is `0x005E6B49` relevant to standard Single Player `0x100` Skirmish `0x579`, or is it a different shell/menu path?
- **Non-goals:** do not redo `FUN_006071E0` frame timing, random-map setup controls, or `0x00612690` ownership except contrast.
- **Evidence needed to mark COMPLETE:** owner boundary, route into the owner, gates before `0x005E6B49`, active-YR proof, contrast against `0x0052D640` and `0x00612690`, Rust-facing handoff.
- **Stop conditions:** stop once the caller is tied to one active route and the Single Player relevance verdict is explicit.

## Load-Bearing Verified Facts

1. `0x005E6B49` is inside raw callback bytes `0x005E6920..0x005E7041`, installed by `FUN_005E68A0` when it creates dialog resource `0x6B`.
   - **Evidence:** `FUN_005E68A0` decompile calls `FUN_00775700(param_1, &LAB_005E6920, 0)` and stores the HWND in `DAT_00AC0D40`; raw callback has stdcall exits `RET 0x10` including `0x005E6B60` and `0x005E7041`.
   - **Active in YR:** Yes. This is the standard offline Skirmish Choose Map modal reached from dialog `0x102`.

2. The branch containing `0x005E6B49` is the Choose Map `WM_COMMAND` handler for control `0x583` (`Create Random Map`) when the nested setup helper returns `-1`.
   - **Evidence:** raw callback decodes `WM_COMMAND 0x111` at `0x005E69B7`; compares command `0x583` at `0x005E69D3..0x005E69D8`; calls `FUN_00608070` and `ShowWindow(hwnd, 0)` at `0x005E69FD..0x005E6A0B`; calls `FUN_005E8590` at `0x005E6A11`; compares `EBX` with `-1` at `0x005E6A18`; jumps to `0x005E6B47`.
   - **Active in YR:** Conditional. It requires the player to click Create Random Map in the live Choose Map dialog.

3. `0x005E6B49` calls `FUN_00608260` on the Choose Map dialog HWND, then shows that same chooser with `ShowWindow(hwnd, 5)`.
   - **Evidence:** `0x005E6B47 MOV ECX, ESI`; `0x005E6B49 CALL 0x00608260`; `0x005E6B4E PUSH 5`; `0x005E6B50 PUSH ESI`; `0x005E6B51 CALL [0x007E1498]`.
   - **Active in YR:** Conditional. It fires only when `FUN_005E8590` returns `-1`; `FUN_00608260` still has its own shell-record gates (`+0xC1 != 0`, `+0xB4 == 1`, visible HWND).

4. `FUN_005E8590` returns `-1` for any nested random-map setup result other than exactly `1`.
   - **Evidence:** `FUN_005E8590` decompile; assembly `0x005E85C1 CALL 0x00595BC0`, `0x005E85C6 CMP EAX, 1`, `0x005E85CB OR EAX, 0xFFFFFFFF`, `0x005E85CE RET`.
   - **Active in YR:** Conditional. This covers nested setup cancel/failure.

5. Direct calls to `FUN_00608260` in the retail `.text` are exactly `0x005E6B49` and `0x00612690`; `0x005E6B49` is not in the Single Player dialog proc.
   - **Evidence:** raw PE call-relative scan for target `0x00608260` found `['0x5e6b49', '0x612690']`; prior `0x0052D640..0x0052D785` disassembly contains no `FUN_00608260` call.
   - **Active in YR:** Yes for the binary call inventory; per-call liveness is conditional by route/gates.

## Relationship To Single Player `0x100` Skirmish `0x579`

`0x005E6B49` is **not relevant** to the standard Single Player `0x100` Skirmish `0x579` route.

The verified Single Player route remains:

`main menu 0x683 -> result 1 -> FUN_0060D380(1) -> dialog 0x100 -> button 0x579 -> result 0x0B -> g_GameMode = 5 -> offline Skirmish dialog 0x102`.

The `0x579` result write is in `0x0052D640`, while `0x005E6B49` is in the later Skirmish Choose Map modal path after `0x102` is already active and the player clicks `0x5AA` then `0x583`.

## Contrast With `0x00612690`

| Caller | Owner | Trigger type | Route relevance |
|---|---|---|---|
| `0x005E6B49` | Choose Map `0x6B` callback `0x005E6920..0x005E7041` | Explicit branch: `0x583` Create Random Map, nested setup returns `-1`, re-show chooser | Not `0x100 -> 0x579`; active later in Skirmish Choose Map |
| `0x00612690` | Shell child subclass dispatcher `0x00610CA0..0x006128FE` | Paint/state branch: per-control record `+0x1FC == 1`, writes `2`, calls helper, writes `3` on success | Plausible shell-control transition, but not proven for `0x579` |

## Implementation Handoff

| Verified behavior | Evidence | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|---|
| `0x005E6B49` is a Choose Map Create Random Map cancel/failure re-show transition, not Single Player Skirmish route. | `0x005E69D3..0x005E6B51`; `0x005E85C1..0x005E85CE`; prior `0x0052D640` negative | Keep Single Player `0x579 -> 0x0B` independent from this caller. | `src/app.rs`, `src/ui/single_player_shell/state.rs`, `src/app_shell_transition.rs` | Clicking Single Player Skirmish can route to `0x102` without requiring `0x005E6B49` semantics. | `single_player_skirmish_does_not_use_choose_map_random_transition` | Do not cite `0x005E6B49` as proof for main-menu or Single Player transition parity. |
| Native Choose Map Create Random Map cancel/failure hides the chooser, calls nested setup, then calls `FUN_00608260` and `ShowWindow(hwnd,5)` to re-show the chooser. | `0x005E69FD..0x005E6B51` plus `FUN_00608260` decompile | Current Rust has Choose Map modal and a `create_random_map` helper, but no nested random-map setup/result lifecycle in the app path. | `src/app.rs`, `src/ui/skirmish_shell/state/choose_map.rs`, `src/app_skirmish_shell_render.rs` | Press Create Random Map, cancel nested setup: chooser remains/returns visible with committed parent selection unchanged, using a transition only on the chooser re-show path. | `choose_map_create_random_map_cancel_reshows_chooser_after_transition` | Do not close the parent Choose Map modal or commit `RandMap.Sed` on nested cancel/failure. |
| `0x005E6B49` and `0x00612690` are the only direct `FUN_00608260` callers, with different ownership models. | raw PE call-relative scan; `SHELL_TRANSITION_CALLER_00612690_OWNER_GHIDRA_REPORT.md` | Keep transition taxonomy split: explicit Choose Map re-show vs generic shell child paint/state. | research docs, future shell transition model | A transition implementation can attach to the right route only after its owning caller is verified. | `shell_transition_callers_remain_route_scoped` | Do not globalize `FUN_00608260` as "every shell button click". |

## Negative Facts / Do Not Do

- Do not claim `0x005E6B49` is part of the standard Single Player `0x100` Skirmish `0x579` route. Evidence: it is reached through `0x102 -> 0x5AA -> 0x6B -> 0x583`, not `0x0052D640`.
- Do not treat `0x005E6B49` as the initial Skirmish `0x102` entry animation. Evidence: parent `0x102` is already active before `0x5AA` opens Choose Map.
- Do not use `0x005E6B49` to justify a main-menu-to-Skirmish bridge. Evidence: callback resource is Choose Map `0x6B`; main menu `0xE2` and Single Player `0x100` are separate routes.
- Do not collapse `0x005E6B49` and `0x00612690`. Evidence: first is explicit `0x583` branch; second is dispatcher paint/state branch gated by `record+0x1FC`.
- Do not say nested random-map setup cancel has no UI transition. Evidence: `0x005E6B49` calls the direct transition helper before `ShowWindow(hwnd,5)` on the non-accept path.

## Remaining Uncertainty

- Whether `FUN_00608260` always succeeds on this `0x005E6B49` path at runtime depends on the chooser shell record gates (`+0xC1`, `+0xB4`, visibility). Static evidence proves the call; exact framebuffer output still needs runtime capture.
- The full nested random-map setup UI and exact cancel/failure return provenance are covered by sibling random-map reports, not re-expanded here.
- This does not close the independent question of whether `0x00612690` fires during retail Single Player `0x100` Skirmish `0x579`.

## Stale-Doc Replacement Wording

For `docs/research/SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`, replace wording that says:

> `0x005E6B49` is likely the post-Load-Game-success continuation.

with:

> `0x005E6B49` is inside the standard Skirmish Choose Map dialog `0x6B` callback (`0x005E6920..0x005E7041`). It belongs to the `0x583` Create Random Map branch: after hiding the chooser and calling nested helper `FUN_005E8590`, a non-accept result (`-1`) reaches `0x005E6B47`, calls `FUN_00608260` on the chooser HWND, then calls `ShowWindow(hwnd, 5)` to re-show the chooser. This is not a main-menu, Single Player `0x100`, or Load/Save success transition.

For docs that say "`0x005E6B49` is initial Skirmish entry", use:

> `0x005E6B49` is not initial Skirmish entry. It is a later Choose Map `0x6B` / Create Random Map `0x583` re-show path after Skirmish setup `0x102` is already active.

## Sources

- Fresh read-only Ghidra: `FUN_005E68A0`, `FUN_006ACEE0`, `FUN_005E7160`, `FUN_005E7BF0`, `FUN_005E8590`, `FUN_00608070`, `FUN_00608260`.
- Fresh read-only Ghidra assembly context: `0x005E6B49`, `0x005E69B7..0x005E6B51`, `0x005E85C1..0x005E85CE`.
- Raw PE disassembly via local retail `gamemd.exe` and Capstone: callback bytes `0x005E6920..0x005E7041`.
- Raw PE call-relative scan: direct `CALL 0x00608260` sites are `0x005E6B49` and `0x00612690`.
- Prior docs reconciled: `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`, `SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`, `SHELL_TRANSITION_ON_MAIN_MENU_CLICK_GHIDRA_REPORT.md`, `SHELL_TRANSITION_CALLER_00612690_OWNER_GHIDRA_REPORT.md`, `SHELL_MENU_TRANSITION_SYSTEM_MODEL_SYNTHESIS.md`, `skirmish-ui/SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_CREATE_RANDOM_MAP_0X583_ACCEPT_CANCEL_STATE_MACHINE_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/app.rs`, `src/ui/skirmish_shell/state/choose_map.rs`, `src/app_skirmish_shell_render.rs`, `src/app_shell_transition.rs`, `src/ui/single_player_shell/state.rs`.

## Status

COMPLETE. The owner and semantics of `0x005E6B49` are recovered for the scoped question. It is a live conditional Choose Map / Create Random Map re-show transition path, not the standard Single Player `0x100` Skirmish `0x579` route.
