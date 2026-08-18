# Shell First-Paint Slide — Trigger / Entry-Edge One-Shot Parity Trace

**Date:** 2026-05-30
**Scope:** WHEN the slide (re)starts, cancels, and which dialog legs are covered — Rust vs gamemd. Frame indices, schedule length, control enum, and completion side-effects are adjacent findings only.
**Verdict summary:** Entry-edge restart and one-shot semantics PASS. Campaign 0x94 slide is NOT-IMPLEMENTED. Dialogs 0x101, 0x129, and 0x6B (mid-session) are NOT-IMPLEMENTED. WOL/network allow-list members are UNCHECKED.

---

## 1. Native mechanism (verified from binary)

**Source:** `FUN_00610ca0` (subclass wndproc, `0x00610CA0`), `FUN_00608260` (`0x00608260`), `FUN_0060c540` (`0x0060C540`). All verified via live Ghidra decompile in the authoritative prior session (`SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`).

### 1.1 One-shot trigger per dialog lifetime

On a dialog's **first** `WM_PAINT`, `FUN_00610ca0` sets the per-dialog owner-draw record field `+0x1FC` from `0 → 1`. The same paint's unwind sees `+0x1FC == 1`, calls `FUN_00608260` (the slide), and on success sets `+0x1FC = 3`. Field `+0x1FC` never returns to 0 while the dialog lives. Because every navigation (back-button, re-entry) calls `FUN_00622650` (CreateDialogIndirectParamA) to **create a fresh dialog**, the new dialog's record starts at `+0x1FC = 0`, so the slide fires again on its first paint.

**Result:** one slide per dialog-lifetime; re-creation on navigation = fresh slide every time the player navigates to a shell.

### 1.2 Eligibility gate — allow-list in `FUN_0060c540`

`FUN_00608260` requires record `+0xB4 == 1` and `+0xC1 != 0`. These are set exclusively by `FUN_0060c540`, which is called from the shared shell init (`FUN_00622820` → `FUN_00622b50`). `FUN_0060c540` sets the markers **only when the dialog's resource ID (`record+0x70`) is in a hardcoded allow-list**.

Full allow-list extracted from `FUN_0060c540` decompile (live, this session):

| Dialog ID | Decimal | YR role | Reachable in standard offline YR? |
|---:|---:|---|---|
| `0xE2` | 226 | Main menu | Yes |
| `0x100` | 256 | Single-player shell | Yes |
| `0x102` | 258 | Offline skirmish setup | Yes |
| `0x94` | 148 | Campaign selector | Yes (via Single Player → New Campaign) |
| `0x6B` | 107 | Choose Map modal (inside 0x102) | Yes (skirmish → Choose Map) |
| `0x101` | 257 | Movies & Credits sub-panel | Yes (via Main Menu → Movies) |
| `0x129` | 297 | Movies picker | Yes (within 0x101 flow) |
| `0xB6` | 182 | WOL/network | UNCHECKED — WOL path |
| `0xA3` | 163 | WOL/network | UNCHECKED |
| `0x73` | 115 | Unknown shell | UNCHECKED |
| `0xD8` | 216 | Unknown shell | UNCHECKED |
| `0xBBB` | 3003 | Unknown shell | UNCHECKED |
| `0xF5` | 245 | Unknown shell | UNCHECKED |
| `0x105` | 261 | Unknown shell | UNCHECKED |
| `0xD5` | 213 | Unknown shell | UNCHECKED |
| `0x2B5` | 693 | Unknown shell | UNCHECKED |
| `0xB7` | 183 | WOL/network | UNCHECKED |
| `0x2B4` | 692 | Unknown shell | UNCHECKED |
| `0x10B` | 267 | Unknown shell | UNCHECKED |
| `0xD7` | 215 | Unknown shell | UNCHECKED |
| `0x108` | 264 | Unknown shell | UNCHECKED |
| `0xBC` | 188 | Unknown shell | UNCHECKED |
| `0xBD` | 189 | Unknown shell | UNCHECKED |
| `0xBB` | 187 | Unknown shell | UNCHECKED |
| `0xB5` | 181 | Unknown shell | UNCHECKED |
| `0xBBA` | 3002 | Unknown shell | UNCHECKED |
| `0xFF` | 255 | Unknown shell | UNCHECKED |
| `0xEA` | 234 | Unknown shell | UNCHECKED |
| `0xB8` | 184 | WOL/network | UNCHECKED |
| `0xD6` | 214 | Unknown shell | UNCHECKED |
| `0x103` | 259 | Unknown shell | UNCHECKED |
| `0xD4` | 212 | Unknown shell | UNCHECKED |
| `0x125` | 293 | Unknown shell | UNCHECKED |
| `0x122` | 290 | Unknown shell | UNCHECKED |
| `0x112` | 274 | Unknown shell | UNCHECKED |
| `0xE7` | 231 | Unknown shell | UNCHECKED |
| `0x116` | 278 | Unknown shell | UNCHECKED |
| `0xC2` | 194 | Unknown shell | UNCHECKED |
| `0xC9` | 201 | Unknown shell | UNCHECKED |
| `0x113` | 275 | Unknown shell | UNCHECKED |
| `0xFE` | 254 | Unknown shell | UNCHECKED |
| `0x10F` | 271 | Unknown shell | UNCHECKED |
| `0x11D` | 285 | Unknown shell | UNCHECKED |
| `0x11C` | 284 | Unknown shell | UNCHECKED |
| `0x117` | 279 | Unknown shell | UNCHECKED |
| `0x114` | 276 | Unknown shell | UNCHECKED |
| `0xE6` | 230 | Unknown shell | UNCHECKED |
| `0xF3` | 243 | Unknown shell | UNCHECKED |
| `0xF4` | 244 | Unknown shell | UNCHECKED |
| `0x2BC` | 700 | Unknown shell | UNCHECKED |
| `0x10E` | 270 | Unknown shell | UNCHECKED |
| `0xFB` | 251 | Unknown shell | UNCHECKED |
| `0x10C` | 268 | Unknown shell | UNCHECKED (excluded first in chain) |

The allow-list is the conjunction-negation chain in `FUN_0060c540`; a dialog NOT in the list skips the `piVar3[0x2d]=1` write and does not slide.

---

## 2. Rust mechanism (verified from source)

**Files:** `src/app_shell_transition.rs`, `src/app.rs`.

### 2.1 Trigger function

`update_shell_first_paint_slide_trigger` (called once per frame at `src/app.rs:2458`) runs:

```rust
let target = current_shell_slide_target(state);
if target == state.shell_slide_active_shell { return; }
state.shell_slide_active_shell = target;
match target {
    Some(kind) => start_shell_first_paint_slide(state, kind.slot_count()),
    None => state.shell_first_paint_slide = None,
}
```

Edge detection: only fires when `target != shell_slide_active_shell`. One-shot per shell showing: once the wave starts, `shell_slide_active_shell` holds the current kind, so no re-trigger mid-stay.

### 2.2 Shell selector

`current_shell_slide_target` maps showing state to `ShellSlideKind`:

| Condition | Returns |
|---|---|
| `screen != MainMenu` | `None` |
| `main_menu_show_native_skirmish_shell` or `dev_skirmish_shell_enabled` | `Some(Skirmish)` |
| `main_menu_show_single_player_shell` | `Some(SinglePlayer)` |
| `!main_menu_shell_failed && !main_menu_show_skirmish_setup` | `Some(MainMenu)` |
| else | `None` |

`ShellSlideKind` has three variants: `MainMenu`, `SinglePlayer`, `Skirmish`. No `Campaign` variant exists.

### 2.3 Wave cancellation

All navigation helpers (`close_native_skirmish_shell`, `start_skirmish_session`, `start_selected_skirmish`, `return_from_skirmish_to_single_player_shell`) explicitly set `state.shell_first_paint_slide = None` before changing shell flags. This pre-clears stale waves so the trigger sees a clean edge.

---

## 3. Question-by-question verdicts

### (a) Entry-edge restart semantics — PASS

**Native:** fresh dialog creation on every navigation → `+0x1FC` always starts `0` → slide fires on first paint → one-shot per dialog lifetime. Back-navigation always re-creates the dialog.

**Rust:** `shell_slide_active_shell` holds the *kind* of the current dialog. On navigation, the kind changes (e.g. `SinglePlayer → MainMenu` when Back is pressed) → `update_shell_first_paint_slide_trigger` detects the edge and starts a fresh wave. This is structurally equivalent: each time the player enters a shell, a fresh wave begins; no re-trigger occurs mid-stay.

**PASS** — semantics match for the implemented shells (MainMenu, SinglePlayer, Skirmish). The mechanism differs (edge on enum change vs. first WM_PAINT) but produces identical observable output: one slide per entry.

### (b) Campaign dialog 0x94 — NOT-IMPLEMENTED

**Native:** `Main_Game` case 8 opens dialog `0x94` (asm `0x0052DF4D MOV ECX,0x94`; `0x0052DF65 CALL FUN_00622650`). `0x94` is in the allow-list → slides on first paint. This is reachable via: Main Menu → Single Player shell → New Campaign button.

**Rust:** `SinglePlayerShellAction::NewCampaign` sets `state.campaign_select = Some(...)` — an egui overlay, not a native shell dialog. `main_menu_show_single_player_shell` remains `true` during campaign select. `current_shell_slide_target` returns `Some(SinglePlayer)` unchanged → no edge detected → no new slide starts. `ShellSlideKind` has no `Campaign` variant. The campaign selector slide is **NOT-IMPLEMENTED**.

Player-visible effect: entering the campaign selector dialog does not animate controls sliding in. In native YR, the campaign selector (difficulty/side picker) controls slide into position on entry.

### (c) Other standard-YR allow-listed dialogs not in Rust — NOT-IMPLEMENTED

**Dialog 0x101 — Movies & Credits sub-panel:**
- Native: `Main_Game` case 4 (`0x0052DD93`) opens dialog `0x101` (proc thunk `0x0052D790`). In the allow-list → slides on first paint. Reachable via Main Menu → Movies & Credits.
- Rust: `MainMenuShellAction::MoviesAndCredits` opens `state.movies_credits_dialog` (egui overlay). `current_shell_slide_target` continues returning `Some(MainMenu)` — no change → no slide started for the sub-panel. NOT-IMPLEMENTED.

**Dialog 0x129 — Movies picker:**
- Native: opened from within `0x101` for the Movies sub-action. In the allow-list → slides on first paint.
- Rust: no native dialog 0x129 equivalent; picker would be inside the egui movies overlay. NOT-IMPLEMENTED.

**Dialog 0x6B — Choose Map modal (mid-skirmish-session):**
- Native: `FUN_006ACEE0` (Choose Map button inside 0x102) opens dialog `0x6B`. In the allow-list → slides on first paint. Reachable via Skirmish setup → Choose Map button.
- Rust: `skirmish_shell_state.choose_map_modal` is rendered as an egui modal overlay on top of the skirmish shell. `current_shell_slide_target` continues returning `Some(Skirmish)` (unchanged) → no edge → no slide for the Choose Map modal. NOT-IMPLEMENTED.

**WOL / network / other allow-listed IDs:** Not reachable in standard offline YR skirmish flow. All marked UNCHECKED pending investigation of whether any offline-YR path reaches them.

### (d) egui fallback / skirmish-setup path — PASS

When `main_menu_show_skirmish_setup = true` (egui skirmish setup, not the native skirmish shell), `current_shell_slide_target` reaches the `else { None }` branch and returns `None`. Native: the egui setup path has no corresponding native shell dialog, so no slide fires. **PASS** — Rust correctly suppresses the slide for the egui fallback.

---

## 4. Adjacent findings (scope boundary — not evaluated here)

1. **Frame indices (GROUP_A_IN/GROUP_B_IN constants)** — covered by prior slide-frame-schedule reports, not re-verified here.
2. **Schedule length (`total_ticks_for`)** — formula covered by prior reports.
3. **Control enum (`ShellSlideKind::slot_count`)** — slot counts for each shell dialog are adjacent (frame composition scope).
4. **Completion side-effects** — `shell_first_paint_slide = None` on wave end; native `+0x1FC = 3` is the equivalent terminal state. Not compared here.

---

## 5. Evidence log

- Live Ghidra decompile (this session): `FUN_0060c540` at `0x0060C540` — full allow-list extracted.
- Prior authoritative session: `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md` — `FUN_00610ca0`, `FUN_00608260`, `FUN_0060c540`, Main_Game case 8, dialog 0x94 route.
- Prior session: `MOVIES_CREDITS_DIALOG_PLAYBACK_FUN_005BED40_GHIDRA_REPORT.md` — dialog 0x101 (Movies & Credits), dialog 0x129 (Movies picker), Main_Game case 4 route (`0x0052DD93`).
- Prior session: `SKIRMISH_CHOOSE_MAP_BUTTON_ACTION_0X102_TO_0X6B_TRACE.md` — dialog 0x6B route from skirmish.
- Rust source verified: `src/app_shell_transition.rs` (full file), `src/app.rs` lines 533–638, 1631–1652, 1655–1687, 2455–2458.

---

## 6. Summary table

| Check | Native | Rust | Verdict |
|---|---|---|---|
| One slide per dialog-lifetime (no mid-stay repeat) | `+0x1FC` one-shot | `shell_slide_active_shell` edge gate | PASS |
| Fresh slide on every re-entry / back-navigation | Dialog re-created → `+0x1FC=0` | Kind changes → edge detected | PASS |
| Cancel wave on leaving all shells | n/a (dialog destroyed) | `shell_first_paint_slide = None` | PASS |
| Main menu 0xE2 slides on entry | Allow-listed, first-paint | `ShellSlideKind::MainMenu` | PASS |
| Single-player shell 0x100 slides on entry | Allow-listed, first-paint | `ShellSlideKind::SinglePlayer` | PASS |
| Skirmish setup 0x102 slides on entry | Allow-listed, first-paint | `ShellSlideKind::Skirmish` | PASS |
| Campaign selector 0x94 slides on entry | Allow-listed, first-paint | No `Campaign` variant; egui overlay | NOT-IMPLEMENTED |
| Movies & Credits 0x101 slides on entry | Allow-listed, first-paint | egui overlay, no slide | NOT-IMPLEMENTED |
| Movies picker 0x129 slides on entry | Allow-listed, first-paint | egui overlay, no slide | NOT-IMPLEMENTED |
| Choose Map modal 0x6B slides on entry | Allow-listed, first-paint | egui modal overlay, no slide | NOT-IMPLEMENTED |
| egui skirmish-setup suppresses slide | No native dialog | `current_shell_slide_target → None` | PASS |
| WOL/network allow-list members | Allow-listed in binary | Not implemented (WOL not in scope) | UNCHECKED |
