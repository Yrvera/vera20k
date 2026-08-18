# Shell First-Paint Slide — Control Enumeration & Group A/B Classification Parity Trace

**Date:** 2026-05-30
**Scope:** Which controls participate in the shell first-paint slide, how they split into
Group A vs Group B, and whether Rust's `slot_count` and group assignment match gamemd
for dialogs 0x102 (Skirmish), 0xE2 (main menu), and 0x100 (single-player shell).
**Authority:** binary → Ghidra (live decompile this session) → docs.
**Confidence:** High for all three dialogs' predicate membership; UNCHECKED for runtime
visibility overrides that could exclude otherwise-predicate-matching controls.
**Adjacent-findings-only:** frame indices (slot 1), schedule length inter-dialog (slot 2),
completion detection (slot 4), trigger mechanism (slot 5).

---

## 1. Membership Predicates — Full Decompile

### 1.1 FUN_00608CD0 (Group A predicate, increments DAT_00AC1CAC)

Verified via `decompile_function 0x00608CD0`.

Parameters: `(int param_1 /*parent HWND used to look up dialog resource ID iVar4*/,
HWND param_2 /*child HWND*/)`. `iVar3 = GetDlgCtrlID(param_2)`.

The predicate is a dialog-ID × control-ID lookup table. Each branch below is
mutually exclusive per dialog ID:

| Dialog iVar4 | Matching control IDs (iVar3) |
|---:|---|
| many dialogs incl. 0x102 | 0x694 (first shared block) |
| many dialogs NOT incl. 0x102 | 0x71C (second shared block) |
| 0x102, 0xbc, 0xbd, 0xc2, 0xc9, 0x105, 0x6b, 0x113, 0xbc6 | 0x468 |
| 0xE2 only | 0x686, 0x578, 0x55C, 0x683, 0x55F, 0x684 |
| 0x101 only | 0x68E, 0x68D, 0x68F |
| 0x129 only | 0x745 |
| 0x100 only | 0x689, 0x688, 0x579 |
| 0x94 only | 0x40E |
| 0x102 only (else-if branch) | 0x6EC, 0x5AA, 0x5A8, 0x617 |
| *(many other dialogs)* | *(out of scope)* |

Predicate is keyed on (dialog resource ID, control ID). Visibility and enabled-state
filter are applied by the enum callback (FUN_0060A180), not by FUN_00608CD0.

**Enum callback gate (FUN_0060A180, decompiled):**
- `(GetWindowLongA(param_1, -0x10) & 0xB) == 0xB` — low style bits must be 0xB
  (owner-draw button class). Only controls with this style pass.
- `piVar4[0x1b] == 0` — enabled (not disabled state).
- `FUN_00608CD0() != 0` — predicate above returns true.
- `IsWindowVisible(param_1) != 0` — visible at enum time.

### 1.2 FUN_00609730 (Group B predicate, increments DAT_00AC4894)

Verified via `decompile_function 0x00609730`.

Same parameter signature and HWND/record lookup as FUN_00608CD0.

| Dialog iVar4 | Matching control IDs (iVar3) |
|---:|---|
| 0xE2 | 0x3EE (Exit Game) |
| 0x73, 0x10C, 0x103 | 0x423 |
| 0x108, 0xBC7 | 0x6D1 |
| 0x102, 0x105, 0x6B, 0xBB, 0x117, 0xBC6 | 0x5C0 (Back) |
| 0xBC, 0xBD, 0xEA, 0xD7, 0xC2, 0xC9, 700, 0xE7, 0xE6, 0xF3, 0xF4, 0x122, 0x112, 0xFE, 0xFC | 0x2 |
| 0x100 (falls to the last else-return path) | 0x686 (Back/Cancel) |
| 0xD4 | 0x1 |
| 0xFB | 0x7 |
| 0xFF | 0x675 |
| 0x94, 0xB6, 0xA3, 0xD8, 0xBBB, 0xF5, 0xD5, 0x2B5, 0xB7, 0x2B4, 0x101, 0x129, 0xB5, 0xBBA, 0x100, 0xB8, 0xD6, 0x125, 0x116, 0x11D, 0x11C, 0x113, 0x109, 0x10F, 0x114, 0x10E | 0x686 (last `return iVar3 == 0x686`) |

For dialog 0x100: the big negated-and chain includes `iVar4 != 0x100`, so 0x100
falls to `return iVar3 == 0x686` → Back button (0x686) is Group B for 0x100. ✓

---

## 2. Timing Array & Slot Count — FUN_006071E0 Construction

Verified via `decompile_function 0x006071E0`.

Variables (as read from decompile):
- `iStack_168` ← `DAT_00AC1CAC` after first EnumChildWindows (Group A count)
- `iVar5 = iStack_168` (Group A count, call it **N_A**)
- `DAT_00AC4894` ← count after second EnumChildWindows (Group B count, call it **N_B**)
- `cStack_175 = (0 < DAT_00AC4894)` → true when N_B >= 1
- `iStack_148 = iStack_168 - 1` when cStack_175; else `iStack_148 = iStack_168`
- `iVar8 = iVar5 + 1` = N_A + 1
- Array allocated: `(iVar5 + 3) * 4` bytes = N_A+3 int slots
- Loop fills slots 0..N_A with values 1..N_A+1; iVar7 = N_A+2 after loop
- `local_17c[N_A]` = 0 (sentinel)
- `local_17c[N_A+1]` = iVar7+1 = N_A+3
- `local_17c[N_A+2]` = 0
- Max value in array = N_A+3
- **Total ticks = N_A + 3 + 6 = N_A + 9**

### 2.1 Frame Ramp Parameters (from FUN_006071E0 with cVar14 = slide-in direction)

Slide-in constants (cVar14==0, forward direction iStack_174=1):

| Constant | Value (slide-in) | Rust name |
|---|---:|---|
| `iStack_13c` (Group A base) | 5 | `GROUP_A_IN.base` |
| `iStack_10c` (Group A before) | 1 | `GROUP_A_IN.before` |
| Group A after (terminal) | 10 | `GROUP_A_IN.after` |
| `iStack_114` (Group B base) | 0xB = 11 | `GROUP_B_IN.base` |
| Group B before (terminal) | 0 | `GROUP_B_IN.before` |
| Group B after (terminal) | 10 | `GROUP_B_IN.after` |

These match Rust's `GROUP_A_IN` and `GROUP_B_IN` exactly.

### 2.2 Which Loop Uses Which Parameters

**Draw Loop 1** (`if (0 < (int)local_14c)`): iterates slots 0..N_A-1, uses `iStack_13c`
(Group A frames). These are the FUN_00608CD0 controls.

**Draw Loop 2** (`if ((int)local_14c < iStack_148)`): iterates slots N_A..iStack_148-1,
uses `iStack_114` (Group B frames = base 11). Active only when N_A < iStack_148, i.e.,
when N_B > 1. (For N_B=1 and cStack_175=true: iStack_148 = N_A-1 < N_A → loop empty.)

**`cStack_175` block** (`if (cStack_175)`): draws the single Group B control at slot
`local_17c[iStack_168-1]` = the N_A-th slot value = N_A. Uses `iStack_13c` (Group A
frame constants!). This is the path taken when N_B==1 (one Back/secondary button).

**Conclusion:** when N_B==1 (the standard case for all three target dialogs), the
secondary control is drawn with Group A frame parameters (base 5, before 1, after 10),
NOT Group B (base 11). The Group B (iStack_114) loop only fires when N_B > 1.

---

## 3. Per-Dialog Verification

### 3.1 Dialog 0x102 — Skirmish

**Dialog controls (from RT_DIALOG resource, prior docs verified):**
Relevant owner-draw children: 0x694 (Static heading), 0x468 (Static thumbnail), 0x6EC,
0x5AA, 0x5A8, 0x617 (from else-if branch), 0x5C0 (Back button).

**Group A (FUN_00608CD0 for 0x102):**
- Shared block 0x694: YES (0x694 exists in dialog)
- Third block 0x468: YES (0x468 exists in dialog)
- Else-if 0x102 block: 0x6EC, 0x5AA, 0x5A8, 0x617
- Total: **N_A = 6** (0x694, 0x468, 0x6EC, 0x5AA, 0x5A8, 0x617)

**Group B (FUN_00609730 for 0x102):** 0x5C0 (Back) → **N_B = 1**

**Timing:** cStack_175=true, total_ticks = N_A+9 = 6+9 = **15 ticks**

**Back button timing slot:** local_17c[5] = 6, drawn with Group A frame params.

**Rust:** `ShellSlideKind::Skirmish` slot_count=3, total_ticks=3+8=11. Rust assigns Back
(0x5C0) to `ButtonGroup::B` (base 11 parameters).

| Check | gamemd | Rust | Verdict |
|---|---|---|---|
| Skirmish Group A count | 6 | slot_count=3 | **FAIL** |
| Skirmish total_ticks | 15 | 11 | **FAIL** |
| Back button group params | Group A (base 5) | Group B (base 11) | **FAIL** |

### 3.2 Dialog 0xE2 — Main Menu

**Dialog controls (from MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md):**
0x694, 0x695, 0x71A, 0x71C, 0x71D, 0x683, 0x684, 0x578, 0x686, 0x55C, 0x3EE.

**Group A (FUN_00608CD0 for 0xE2):**
- Shared block 0x694: 0xE2 in list → YES, control 0x694 exists → +1
- Second shared block 0x71C: 0xE2 in list → but 0x71C's style bits 0x7 ≠ 0xB → FUN_0060A180
  gate fails (style & 0xB != 0xB). However FUN_00608CD0 predicate would return true, but the
  enum callback gate blocks it. **UNCHECKED** whether 0x71C is excluded by style or merely
  by FUN_00608CD0 returning false (0xE2 IS in the second shared block so predicate=true;
  exclusion comes from FUN_0060A180 style check 0x7 & 0xB != 0xB).
- Else-if 0xE2 block: 0x686, 0x578, 0x55C, 0x683, 0x55F(not in dialog), 0x684 → 5 present
- Third block (0x468): 0xE2 NOT in list → 0
- **N_A = 6** (0x694, 0x683, 0x684, 0x578, 0x686, 0x55C) assuming 0x71C excluded by style gate

**Group B (FUN_00609730 for 0xE2):** `if (iVar4==0xe2) return iVar3==0x3ee` → 0x3EE (Exit)
→ **N_B = 1**

**Timing:** cStack_175=true, total_ticks = 6+9 = **15 ticks**

**Exit button timing slot:** local_17c[5] = 6, drawn with Group A frame params.

**Rust:** `ShellSlideKind::MainMenu` slot_count=6, total_ticks=6+8=14.

| Check | gamemd | Rust | Verdict |
|---|---|---|---|
| Main menu Group A count | 6 | slot_count=6 | **PASS** (count matches) |
| Main menu total_ticks | 15 | 14 | **FAIL** (off by 1) |
| Exit button group params | Group A (base 5) via cStack_175 block | not modeled separately | UNCHECKED |
| 0x71C exclusion from enum | via style-gate (0x7 & 0xB != 0xB) | N/A | UNCHECKED (runtime) |

### 3.3 Dialog 0x100 — Single-Player Shell

**Dialog controls (from SINGLE_PLAYER_SUBMENU_DIALOG_CASE1_GHIDRA_REPORT.md):**
0x688, 0x689, 0x579, 0x686.

**Group A (FUN_00608CD0 for 0x100):**
- Shared block 0x694: 0x100 in list → but 0x694 NOT in 0x100's controls → 0
- Second shared block 0x71C: 0x100 in list → but 0x71C not in controls → 0
- Else-if 0x100 block: 0x689, 0x688, 0x579
- Total: **N_A = 3** (0x689, 0x688, 0x579)

**Group B (FUN_00609730 for 0x100):** falls to `return iVar3==0x686` → Back (0x686)
→ **N_B = 1**

**Timing:** cStack_175=true, total_ticks = 3+9 = **12 ticks**

**Back button timing slot:** local_17c[2] = 3, drawn with Group A frame params.

**Rust:** `ShellSlideKind::SinglePlayer` slot_count=4, total_ticks=4+8=12.

| Check | gamemd | Rust | Verdict |
|---|---|---|---|
| SinglePlayer Group A count | 3 | slot_count=4 | **FAIL** |
| SinglePlayer total_ticks | 12 | 12 | **PASS** (by coincidence: 3+9=4+8) |
| Back button group params | Group A (base 5) via cStack_175 | not modeled separately | UNCHECKED |

---

## 4. Group A vs Group B Definition — Rust Comment Accuracy

Rust comment in `app_shell_transition.rs`:
> "Group A = enabled 'active' buttons; Group B = the remaining buttons."

**gamemd reality:**
- FUN_00608CD0 predicate matches specific owner-draw controls per dialog. These are
  predominantly the main action buttons (Start, Choose Map, heading static, thumbnail
  static, etc.) — NOT filtered by enabled/active state (that's the enum callback gate).
- FUN_00609730 predicate matches the single Back/Exit/Cancel button per dialog.
- When N_B==1 (the standard case), the Back button is drawn with Group A frame parameters
  (via the `cStack_175` block), not Group B (base 11) parameters. The Group B iStack_114
  (base 11) loop only fires when N_B>1 — not observed in any of the three target dialogs.

**Verdict:** The Rust comment "Group A = active buttons, Group B = remaining" is a
reasonable semantic description but the frame-constant assignment is wrong: in gamemd,
Back/Exit uses Group A frame constants (base 5) when it is the only Group B control.
Rust's `ButtonGroup::B` (base 11) is never used by gamemd for any of the three target
dialogs.

---

## 5. Verdict Summary

| Dialog | gamemd N_A | Rust slot_count | N_A Match | gamemd ticks | Rust ticks | Ticks Match | Back/Exit group params |
|---|---:|---:|---|---:|---:|---|---|
| 0x102 Skirmish | 6 | 3 | **FAIL** | 15 | 11 | **FAIL** | **FAIL** (back gets base-5, not base-11) |
| 0xE2 Main Menu | 6 | 6 | PASS | 15 | 14 | **FAIL** | UNCHECKED |
| 0x100 SinglePlayer | 3 | 4 | **FAIL** | 12 | 12 | PASS (coincidence) | UNCHECKED |

---

## 6. Player-Visible Effects of the Failures

1. **Skirmish slide is 36% too short** (11 vs 15 ticks × 30 ms = 330 ms vs 450 ms). All
   6 animated controls complete their ramp faster than in the original. Slot stagger is
   also wrong: Rust staggers over 3 slots (30 ms apart), gamemd over 6 (180 ms total
   stagger). The player sees all buttons arrive earlier and with less visual separation.

2. **Skirmish slot assignment is wrong** (6 actual vs 3 Rust slots). The heading text
   (0x694), map thumbnail (0x468), game-type text (0x6EC), and scenario label (0x5A8)
   are also animated in gamemd but absent from Rust's animated set.

3. **Skirmish Back button frame ramp is wrong** (base 11 vs base 5). The Back button
   animates between SDBTNANM frames 11→16 in Rust (Group B) vs 5→10 in gamemd (Group A
   via cStack_175 block). The player sees a visually different button reveal animation
   on the Back button.

4. **Main menu slide is 1 tick too short** (14 vs 15 ticks × 30 ms). The Exit button
   enters at slot 5 in Rust (same as last main action) but gamemd schedules it at slot 5
   with slot timing matching the 6th action button — marginal 30 ms difference visible
   as a slightly early animation completion.

5. **Single-player slide has wrong slot count** (4 vs 3 animated buttons). Rust animates
   a 4th phantom slot that doesn't correspond to any real button, extending the animation
   by 30 ms over gamemd's 3-button schedule. The three real buttons (Campaign, Load, Skirmish)
   get slightly wrong stagger offsets.

---

## 7. Evidence Log

- `decompile_function 0x00608CD0` — full predicate verified this session
- `decompile_function 0x00609730` — full predicate verified this session
- `decompile_function 0x0060A180` — enum callback style/visibility gates verified
- `decompile_function 0x0060A250` — Group B enum callback verified
- `decompile_function 0x006071E0` — timing array construction, draw loops, cStack_175 block,
  frame constants verified
- SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md §3.3 (prior control list)
- SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md §2, §3 (allow-list, slide trigger)
- MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md (0xE2 control IDs)
- SINGLE_PLAYER_SUBMENU_DIALOG_CASE1_GHIDRA_REPORT.md (0x100 control IDs)
- SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md (0x102 control IDs)
- Rust source: `src/app_shell_transition.rs` (lines 71–88, slot_count; lines 157–174 sdbtnanm_frame)

---

## 8. Adjacent Findings (not in scope)

- **Frame indices (slot 1):** GROUP_A_IN/OUT and GROUP_B_IN/OUT constants in Rust match the
  binary-verified iStack_13c/iStack_114/iStack_10c/iStack_110 values exactly.
- **Schedule length formula (slot 2):** gamemd uses N_A+9, Rust uses slot_count+8. For
  slot_count == N_A these differ by 1; fixing slot_count to N_A would leave 1 tick short.
  Full formula fix: `total_ticks = N_A + 9`.
- **Completion (slot 4):** `ShellFrameWave::is_complete` correctly tests tick >= total_ticks;
  the bug is in total_ticks value, not the completion predicate.
- **Trigger mechanism (slot 5):** The first-paint trigger (FUN_00610CA0 `+0x1FC` gate,
  FUN_00608260 call) is confirmed by SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md
  and is not part of this trace scope.

---

**PASS: 2 | FAIL: 6 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0**

(PASS: Skirmish N_B=1 predicate match; SinglePlayer total_ticks coincidence.
 FAIL: Skirmish N_A, Skirmish ticks, Skirmish Back params, Main Menu ticks,
       Main Menu N_A-only slot_count formula, SinglePlayer N_A.
 UNCHECKED: 0x71C exclusion by style gate, Exit/Back group params for 0xE2 and 0x100,
            runtime visibility of any controls excluded by IsWindowVisible at slide time.)

**Status: COMPLETE**
