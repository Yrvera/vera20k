# Shell Transition Caller 00612690 Owner - Ghidra Research Report

**Date:** 2026-05-27
**Address(es):** `0x00612690`, raw owner `0x00610CA0..0x006128FE`, installer `FUN_0060F9A0 @ 0x0060F9A0`, direct helper `FUN_00608260`
**Investigation Mode:** exhaustive-slice for the `0x00612690` owner/message-path question; route-pixel capture remains out of scope.
**Claimed Scope:** recover the owner boundary around `0x00612690`, identify how it is installed, describe the message/state gates that reach `FUN_00608260`, and determine whether the Single Player shell `0x100` Skirmish button/action can plausibly reach it.
**Non-Scope:** full semantics of every `0x00610CA0` message branch, full pixel composition of `FUN_006071E0`, live debugger proof of a retail click, and unrelated `0x005E6B49` caller semantics.
**Confidence:** High for owner boundary, subclass installer, and local state gates. Medium for Single Player click reachability because static evidence proves subclass plausibility but not a runtime `0x579` click hitting the `+0x1FC == 1` transition branch.
**Active in YR:** Conditional. The owner/subclass machinery is active in standard YR shell dialogs; the `0x00612690` call itself is active only when the subclass dispatcher's paint/state gates are satisfied.

## 0. Investigation Contract

**Target question:** What owns call site `0x00612690 -> FUN_00608260`, through which message/state path does it run, and can it plausibly be reached by Single Player dialog `0x100` Skirmish button `0x579`?

**Non-goals:** Do not re-prove the settled `0x683 -> 1 -> 0x100 -> 0x579 -> 0x0B -> 0x102` route except where it bears on this caller. Do not implement Rust. Do not mutate Ghidra. Do not claim frame/pixel parity.

**Evidence needed to mark COMPLETE:** raw or Ghidra evidence for a containing boundary, installer evidence tying the boundary to shell controls, local assembly gates around `0x00612690`, and a route comparison against dialog proc `0x0052D640`.

**Stop conditions:** stop once the owner/message path is bounded and every remaining question is either resolved or explicitly deferred; stop immediately if the only way forward requires mutating Ghidra or runtime debugger capture.

## 1. Overview

`0x00612690` is not inside `OwnerDraw_Button_00612B70` and is not inside dialog proc `0x0052D640`. It is inside an unlabelled shell child subclass dispatcher at raw PE range `0x00610CA0..0x006128FE`. That dispatcher is installed onto shell child windows by `FUN_0060F9A0` via `SetWindowLongA(hwnd, GWL_WNDPROC=-4, 0x00610CA0)`.

The call to `FUN_00608260` is a conditional paint/state transition path: after the dispatcher's paint-related bookkeeping, if the per-control shell record state at `+0x1FC` is `1`, the code writes `2`, tests a `GetWindowLongA(hwnd, 4)` gate, calls `FUN_00608260(parent_hwnd)`, and on success writes `+0x1FC = 3`.

For Single Player shell `0x100`, the Skirmish button `0x579` is a `Button` child with style `0x5000000B`, so it is plausibly subclassed by this machinery and can reach `0x00610CA0` for its child-window messages. However, the actual Skirmish action/result write is still direct in `0x0052D640` and contains no call to `FUN_00608260`. Therefore this caller is not proven to be the native `0x579 -> 0x102` transition owner.

## 2. Owner Boundary And Installation

| Finding | Active in YR | Evidence | Confidence |
|---|---|---|---|
| Raw PE disassembly recovers a complete function-like boundary at `0x00610CA0..0x006128FE`. It starts with `SUB ESP,0x36C`, saves `EBX/EBP/ESI/EDI`, reads four stdcall arguments from `[ESP+0x380..0x38C]`, and has `RET 0x10` epilogues at `0x006128ED` and `0x006128FE`. | Yes, as installed shell child subclass dispatcher. | raw PE disassembly `0x00610CA0`, `0x006128E1..0x006128FE`; Ghidra has no function object at `0x00612690`. | High |
| `FUN_0060F9A0` installs this dispatcher on shell child controls. | Yes; standard shell owner-draw setup uses it. | `FUN_0060F9A0` decompile: `SetWindowLongA(param_1, -4, 0x610ca0)`. | High |
| `FUN_0060F9A0` stores the selected per-control owner-draw proc separately from the dispatcher. For `Button` class with `(style & 0x0B) == 0x0B`, it selects `OwnerDraw_Button_00612B70`, then still installs `0x00610CA0` as the window proc. | Yes for shell button children with that style. | `FUN_0060F9A0` decompile and raw assembly `0x0060FE58..0x0060FF0D`. | High |
| The dispatcher uses two hash tables: one mapping HWND to selected owner-draw proc (`DAT_00AC18C0` family), and one mapping HWND to previous WndProc (`DAT_00AC1B48` family). It also creates/uses a larger per-HWND shell record in `DAT_00AC1B00`. | Yes for subclassed shell controls. | `FUN_0060F9A0` decompile `LAB_0060FFDF..LAB_0061028C`; raw dispatcher lookups at `0x00610D01..0x00610D88` and `0x00612048..0x0061208F`. | High |

## 3. Local `0x00612690` State Path

The local path is bounded by the following gates:

1. `0x006125B4`: loads `EDI = [ESP+0x68]`; if null, exits toward cleanup.
2. `0x006125C0`: requires byte `[ESP+0x1F] != 0`. This byte is set during the dispatcher's `WM_PAINT (0x0F)` handling at `0x00611137..0x0061114D`, so this path is paint-phase, not ordinary `WM_COMMAND`.
3. `0x006125D9`: calls `GetWindowLongA(hwnd, 4)`. If nonzero and `DAT_00AC48DC > 1`, it writes `record+0x1FC = 2`.
4. `0x006125F5`: decrements `DAT_00AC48DC`; if the result is nonzero, exits. The transition branch waits for the final counted paint/control pass.
5. `0x00612606..0x00612621`: requires `record+0x20 == 0`, local state at `[ESP+0x4C] >= 1`, and `FUN_00774070()` returning false.
6. `0x00612642`: compares `record+0x1FC` against `1`. Only exact state `1` enters the direct transition call; any other state branches to the non-transition repaint path at `0x006127C1`.
7. `0x0061266F`: writes `record+0x1FC = 2`.
8. `0x00612679..0x00612687`: calls `GetWindowLongA(hwnd, 4)` again and skips if false.
9. `0x00612689..0x00612697`: moves the parent/main HWND from `[ESP+0x380]` into `ECX`, calls `FUN_00608260`, and tests `AL`.
10. `0x00612699`: on success, writes `record+0x1FC = 3`.

**Active in YR:** Conditional. The dispatcher and records are active shell UI code, but this exact transition branch requires the paint/state gates above. Evidence: raw PE disassembly `0x006125B4..0x00612699`; `FUN_0060F9A0` installer.

After the call site, success/failure re-enters display-surface bookkeeping. The success-side path copies through `DAT_00887308` and `DAT_00887310`, flushes both surfaces, enumerates children through callback `0x00622470`, and sends child message `0x4AF` in the loop at `0x00612777..0x006127A9`.

**Active in YR:** Conditional, same paint/state path. Evidence: raw PE disassembly `0x006126A3..0x006127BC`.

## 4. Single Player `0x100` / Skirmish `0x579` Reachability

| Question | Answer | Active in YR | Evidence |
|---|---|---|---|
| Can dialog `0x100` controls plausibly reach the `0x00610CA0` dispatcher? | Yes, conditionally. Dialog `0x100` is a standard shell dialog; its `Button` children use style `0x5000000B`, and the installer classifies `Button` with `(style & 0x0B) == 0x0B` as `OwnerDraw_Button_00612B70` while installing dispatcher `0x00610CA0`. | Yes for shell child messages after setup. | prior `SKIRMISH_FUN_0060D380_SINGLE_PLAYER_0X100_TO_0X0B_GHIDRA_REPORT.md` resource table; `FUN_0060F9A0` decompile; raw `0x0060FE78..0x0060FF0D`. |
| Is `0x00612690` the direct owner of `0x579 -> 0x0B`? | No. The route result is written directly by dialog proc `0x0052D640`; that proc contains no call to `FUN_00608260`. | Yes for the direct result write; no for direct transition ownership. | prior raw PE disassembly `0x0052D6F1..0x0052D720`; prior report explicitly verified no call to `0x00608260` in `0x0052D640..0x0052D785`. |
| Does static evidence prove a Skirmish click hits `0x00612690` before `0x102`? | No. Static evidence proves the button can be subclassed and painted through `0x00610CA0`, but not that the activation click sets `record+0x1FC == 1` and drains the paint count before the `0x0B` route destroys/leaves dialog `0x100`. | Unchecked/conditional. | local gates `0x006125B4..0x00612699`; direct result owner `0x0052D640`. |

Practical interpretation: `0x00612690` is a generic shell child paint/state transition owner. It is a plausible shell-control transition participant for `0x100`, but it is not proven as the native Single Player `Skirmish` action transition trigger.

## 5. Current Rust Implementation Status

Current Rust has already implemented the intermediate Single Player shell route identity:

| Surface | Status | Evidence |
|---|---|---|
| Main menu Single Player opens the intermediate shell. | Implemented. | `src/app.rs:536`, `src/app.rs:1633` |
| Dialog `0x100` control identities and return codes. | Implemented for `0x688 -> 8`, `0x689 -> 9`, `0x579 -> 0x0B`, `0x686 -> 0x12`. | `src/ui/single_player_shell/state.rs:8`, `src/ui/single_player_shell/state.rs:41` |
| Single Player `Skirmish` action enters Skirmish shell directly. | Implemented, but no transition. | `src/app.rs:556`, `src/app.rs:1603` |
| Existing bridge compositor. | Still named and scoped as main-menu-to-Skirmish bridge/DRIFT; not wired to Single Player `0x579`. | `src/app_shell_transition.rs:1`, `src/app_shell_transition.rs:84` |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Ghidra function object at `0x00612690` | verified absent | `decompile_function(0x00612690)` returned no function | none; raw boundary used |
| Raw owner boundary `0x00610CA0..0x006128FE` | verified | raw PE prologue/epilogues and stack argument use | optional future Ghidra create_function in write-enabled session |
| Installer `FUN_0060F9A0` | verified | Ghidra decompile, `SetWindowLongA(..., 0x610ca0)` | none for this scope |
| Button classifier to `OwnerDraw_Button_00612B70` | verified | `0x0060FE58..0x0060FF0D`; `FUN_0060F9A0` decompile | none |
| Local `0x00612690` gates | verified | raw PE `0x006125B4..0x00612699` | runtime values for a specific click |
| Direct `0x579 -> 0x0B` route | verified by prior doc, not duplicated | `0x0052D6F1..0x0052D720` from prior report | none |
| `0x579` click reaches `0x00612690` | touched-not-exhausted | subclass plausibility plus direct result negative | live trace or write-enabled function creation/call graph |
| Rust single-player shell route | verified | Codegraph + `rg`; files listed above | transition behavior remains design choice/DRIFT |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - What function contains 0x00612690? -> Raw owner is the unlabelled stdcall dispatcher at 0x00610CA0..0x006128FE.` (evidence: raw PE disassembly prologue/epilogues)
- `[RESOLVED] OQ-02 - How is that dispatcher entered? -> FUN_0060F9A0 installs it as child WndProc with SetWindowLongA(hwnd, -4, 0x610ca0).` (evidence: `FUN_0060F9A0`)
- `[RESOLVED] OQ-03 - Is it OwnerDraw_Button_00612B70 itself? -> No; OwnerDraw_Button_00612B70 is a selected callback stored by the installer, while 0x00610CA0 is the installed dispatcher.` (evidence: `0x0060FE82`, `0x0060FF05`)
- `[RESOLVED] OQ-04 - What message phase reaches 0x00612690? -> Paint-phase path requiring [ESP+0x1F] set during WM_PAINT handling, plus non-null shell record.` (evidence: `0x00611137..0x0061114D`, `0x006125B4..0x006125C6`)
- `[RESOLVED] OQ-05 - What state value gates the helper call? -> record+0x1FC must equal 1; the path writes 2 before the call and 3 after successful FUN_00608260.` (evidence: `0x00612642`, `0x0061266F`, `0x00612699`)
- `[RESOLVED] OQ-06 - Does 0x0052D640 need FUN_00608260 to write 0x0B? -> No; it writes 0x0B directly for command 0x579.` (evidence: prior report `0x0052D6F1..0x0052D720`)
- `[RESOLVED] OQ-07 - Can dialog 0x100 Skirmish button plausibly be subclassed into this dispatcher? -> Yes, as a Button with style 0x5000000B under standard shell setup.` (evidence: prior resource table; `FUN_0060F9A0`)
- `[DEFERRED] OQ-08 - Does a retail 0x579 click actually hit 0x00612690 before dialog result 0x0B completes?` (category: `needs-runtime-debugger`; reason: static evidence does not reveal live `+0x1FC` and paint-count values for this exact click; next-step-if-pursued: breakpoint/log `0x00612690` during retail Single Player -> Skirmish click)
- `[DEFERRED] OQ-09 - Which caller/message first sets record+0x1FC to 1 for the 0x579 control in the target scenario?` (category: `bounded-cost-too-high`; reason: the broad dispatcher has many message paths; next-step-if-pursued: write-enabled Ghidra function creation plus focused dataflow on `record+0x1FC`)
- `[RESOLVED] OQ-10 - Is this TS-only legacy? -> No, it is standard shell subclass and owner-draw machinery used by active YR shell dialogs; exact branch is conditional, not TS-gated.` (evidence: `FUN_0060F9A0`, dialog `0x100` route docs)

## 8. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `FUN_0060F9A0` | standard shell child setup | none directly | child HWND client rect | initializes shell colors/cache | yes | subclass/record install |
| 2 | `0x00610CA0` dispatcher | installed as WndProc `-4` | selected callback depends on class | child HWND | dispatcher-level surface bookkeeping | conditional | message owner |
| 3 | `OwnerDraw_Button_00612B70` | Button class, `(style & 0x0B)==0x0B` | button PCX/SDBTNANM paths per record type | child client rect | shell draw palette globals | yes for `0x579` button paint | button paint callback |
| 4 | `0x00612690 -> FUN_00608260` | paint path, `record+0x1FC == 1`, `GetWindowLong(hwnd,4) != 0` | transition SHPs inside `FUN_006071E0` | parent HWND, child schedule | display surfaces `DAT_00887308/10` | conditional/unproven for `0x579` click | transition helper trigger |
| 5 | `0x0052D640 WM_COMMAND 0x579` | command id low word `0x579` | none | parent dialog result pointer | none | yes | route result owner |

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| `0x00612690` is a conditional subclass paint/state transition path, not the command-result owner. | raw `0x00610CA0..0x006128FE`; prior `0x0052D640` report | Rust has direct `Skirmish` action entry and no single-player transition. | `src/app.rs`, `src/app_shell_transition.rs` | If adding a transition on `0x579`, label it bridge/DRIFT unless runtime proves this exact branch fires. | Clicking Skirmish can animate visually, but docs/UI flags must not claim native `FUN_00608260` parity. | `single_player_skirmish_transition_remains_bridge_until_native_owner_verified` | Do not cite `0x00612690` as proven native `0x579` transition trigger. |
| Dialog `0x100` `0x579` route result remains direct `0x0B`. | prior `0x0052D6F1..0x0052D720`; current Rust `state.rs` | Rust matches return code. | `src/ui/single_player_shell/state.rs` | Preserve `Skirmish0x579 -> Some(0x0B)` even if adding visual transition. | Unit test asserts `Skirmish0x579` action returns `0x0B`. | `single_player_skirmish_control_returns_0x0b_during_transition` | Do not replace route identity with a transition-state enum only. |
| Current main-menu Single Player route already opens `0x100`; the old main-menu-to-Skirmish bridge is no longer the native route surface. | `src/app.rs:536`, `src/app.rs:1633` | Bridge module still named main-menu-to-Skirmish and not wired to `0x579`. | `src/app_shell_transition.rs`, render glue | A future smooth bridge should source from Single Player shell and complete to `0x102`, not from main menu. | Main-menu Single Player shows `0x100`; only `0x579` can start a visual bridge to Skirmish. | `main_menu_single_player_does_not_start_skirmish_bridge` | Do not resurrect direct main-menu-to-Skirmish transition. |
| Subclass dispatcher is per-child HWND and paint-count/state driven. | raw `0x006125B4..0x006127BC` | Rust whole-screen bridge composites source/destination targets, not child-scheduled records. | `src/render/shell_transition_pass.rs`, future shell transition model | Exact parity would require child/control schedule and state bytes, not only a full-screen shader. | A parity implementation can replay child order/timing and keep direct route result separate. | `shell_transition_bridge_does_not_emit_native_reveal_events` | Do not collapse generic whole-screen bridge with native child dispatcher semantics. |

## 10. Negative Facts / Do Not Do

- Do not treat `0x00612690` as a direct `WM_COMMAND 0x579` handler. Evidence: direct command route is in `0x0052D640`; local `0x00612690` path is gated by paint/state bytes.
- Do not say `OwnerDraw_Button_00612B70` owns the helper call. Evidence: `FUN_0060F9A0` stores `OwnerDraw_Button_00612B70` as callback but installs `0x00610CA0` as WndProc.
- Do not trigger native static reveal/events from ordinary route result `0x0B` unless the exact nonzero transition branch is proven. Evidence: `FUN_00608260` is conditional and separate from `0x0052D640`.
- Do not use the old main-menu-to-Skirmish bridge as parity evidence. Evidence: current native route is `0x683 -> 1 -> 0x100 -> 0x579 -> 0x0B`.
- Do not invent a visible extra transition control. Evidence: dialog `0x100` resource already has visible Skirmish button `0x579`; proc-only branches are separate.

## Stale Docs / Replacement Wording

Replace wording that says "`0x00612690` is an unidentified owner-draw state machine just below `OwnerDraw_Button_00612B70`" with:

> `0x00612690` is inside the unlabelled shell child subclass dispatcher at raw range `0x00610CA0..0x006128FE`, installed by `FUN_0060F9A0` with `SetWindowLongA(hwnd, GWL_WNDPROC=-4, 0x00610CA0)`. The dispatcher may call `FUN_00608260` only on its paint/state path when the per-control record `+0x1FC` equals `1`; it writes `2` before the call and `3` on success. This is not the direct `0x579 -> 0x0B` command-result owner.

## Sources

- Ghidra read-only: `FUN_0060F9A0`, `OwnerDraw_Button_00612B70`, failed decompile of `0x00612690` confirming no Ghidra function object.
- Ghidra assembly context: `0x00612640..0x00612699`.
- Raw PE disassembly via Capstone over retail `gamemd.exe`: `0x00610CA0..0x006128FE`, `0x0060FDC0..0x0060FF18`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_FUN_0060D380_SINGLE_PLAYER_0X100_TO_0X0B_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_NATIVE_SINGLE_PLAYER_ROUTE_TO_0X102_RECHECK_GHIDRA_REPORT.md`, `docs/research/SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`, `docs/research/SHELL_TRANSITION_ON_MAIN_MENU_CLICK_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/app.rs`, `src/ui/single_player_shell/state.rs`, `src/app_shell_transition.rs`.
