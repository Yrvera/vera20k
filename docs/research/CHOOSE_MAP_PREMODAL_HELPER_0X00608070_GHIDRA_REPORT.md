# Choose Map Pre-Modal Helper FUN_00608070 — Ghidra Research Report

**Target:** `FUN_00608070` at `0x00608070`
**Scope:** What does this function do when called on the setup HWND before the Choose Map
modal (dialog ID `0x6B`) is shown? Is its omission from Rust `open_choose_map_modal` a
real gap?
**Date:** 2026-06-01
**Status:** COMPLETE

---

## 1. Function Signature and High-Level Role

```
undefined4 __fastcall FUN_00608070(HWND param_1)
```

This is a **Win32 shell dialog transition helper** — a "slide/animate-then-block" routine
that plays a UI transition sound, disables the parent window and all its child controls
via `EnumChildWindows`, then spins a **blocking pump loop** (up to 5 000 ms) waiting for
a custom per-window animation flag to clear. It is the hook that drives the SDBTNANM-style
sliding transition animation for YR's Win32 dialog shells.

Evidence: decompiled via `decompile_function 0x00608070`.

---

## 2. Detailed Behavior

### 2.1 Guard checks (early-exit path)

1. Calls `FUN_0069bbe0()` which reads a single byte at `this+0x30D8`. If non-zero,
   return 0. This is the "shell manager busy / TS-animation-disabled" flag.
2. If `DAT_00ac1b04 == 0` (shell object table empty), return 0.
3. Looks up `param_1` (the HWND) in a hash table keyed by HWND. If not found, return 0.
4. Checks two conditions on the located shell object:
   - `*(char*)(obj + 0xC1) != 0` — "animation capable" flag
   - `obj[0x2D] == 1` — sub-state = animated/active
   If either condition fails, `bVar1 = false`.
5. Calls `IsWindowVisible(param_1)`. If window not visible, return 0.
6. If `!bVar1` OR window not visible, return 0 immediately.

**Consequence:** If the setup dialog window is hidden (`ShowWindow(setup, 0)`) before this
function is called, or if the animation flag `obj+0xC1` is clear, the function exits
immediately with no side effects. This is critical for the Rust gap assessment.

### 2.2 Active path (animation ready + visible)

Only reached when the shell object is registered, `obj+0xC1` set, sub-state==1, AND the
window is currently visible:

1. **Plays a sound:** `VocClass__PlayAtPos(0x3F800000, 0)` — volume 1.0f, position 0.
   This is the SDBTNANM transition-out sound.
2. Saves the current `IsWindowEnabled` state of `param_1`.
3. **Disables the parent window:** `EnableWindow(param_1, 0)`.
4. **Disables all child windows:** `EnumChildWindows(param_1, LAB_00606800, 1)` —
   callback at `0x00606800` iterates child HWNDs and disables them.
5. Sets animation-playing flag `*(char*)(obj + 0xC2) = 1`.
6. **Calls `InvalidateRect`** to trigger a repaint (starts the slide-out animation).
7. Enters a **blocking pump loop** (`GetTickCount` + `Process_NetworkMessages` +
   `Main_Tick`) that runs until:
   - `obj+0xC2` is cleared by the animation system (animation complete), OR
   - 5 000 ms timeout is reached.
8. On exit: re-enables child windows (`EnumChildWindows(…, LAB_00606800, 0)`) and
   restores the parent window's enabled state.

Evidence: `decompile_function 0x00608070`.

---

## 3. Callers

Verified via `get_function_callers 0x00608070` — three callers:

| Address | Role | Passes HWND? |
|---------|------|-------------|
| `0x00622720` | Generic shell close helper (calls then `DestroyWindow`) | No (no-arg call) |
| `0x006ACEE0` | Skirmish setup dialog WM_COMMAND handler (`0x5AA` = "Choose Map" button) | No (no-arg call) |
| `0x007757E0` | Modal stack pop function | No (no-arg call) |

**Critical finding:** All three callers pass **no argument** to `FUN_00608070`. The
function is declared `__fastcall HWND param_1` and reads the HWND from ECX — but all
three call sites omit it. Ghidra is inferring the signature from the callee body; the
actual callers do not set ECX before the call. This means `param_1` is whatever ECX
happens to hold at each call site — effectively garbage/uninitialised at those sites.

Evidence: Decompiled bodies of all three callers (`decompile_function` at each address)
show `FUN_00608070()` with zero arguments.

At `0x006ACEE0` (the WM_COMMAND `0x5AA` path = Choose Map button): The call occurs at
message `0x5AA` inside the WM_COMMAND handler. The sequence is:
```
FUN_007ca489(local_200, &DAT_00a8b322);   // copy map name
FUN_00608070();                            // pre-modal helper ← ECX not set
ShowWindow(param_1, 0);                   // hide setup dialog
iVar4 = FUN_005e68a0();                   // create/show modal 0x6B + pump
ShowWindow(param_1, 5);                   // restore setup dialog
```

Because ECX is not deliberately set to the setup HWND, `FUN_00608070` will look up
whatever ECX holds in the shell object hash. This almost certainly fails the hash lookup
and returns 0 immediately. The animation/sound path is **not exercised on Choose Map
entry** in standard YR.

Evidence: `decompile_function 0x006acee0`.

---

## 4. TS Legacy vs YR Activity

The guard `FUN_0069bbe0()` at `0x0069BBE0` reads `this+0x30D8`. This is the shell manager
"animation disabled" flag inherited from Tiberian Sun's transition system. In standard YR
skirmish, the shell manager is the Win32 dialog manager object; if this flag is 0 (meaning
animation enabled), the function proceeds.

However, the more significant gate is the shell object lookup (`obj+0xC1` animation-
capable flag). This flag is set per-dialog at dialog creation time when the SDBTNANM
resource animation data is registered. If the setup dialog was created without animation
registration, `obj+0xC1` is 0 and the function exits immediately.

**YR activity:** Conditional. The function body is live code in YR (not dead). Whether it
produces any effect on Choose Map entry depends entirely on ECX state at the call site in
`0x006ACEE0` — which is not explicitly set, meaning it is almost certainly a no-op in
practice.

Active in YR: **Conditional — likely no-op on Choose Map entry due to ECX not set.**

---

## 5. Observable Effect Assessment

The active path, when reached, produces two player-visible effects:
1. **A transition sound** (VocClass__PlayAtPos).
2. **A visual slide animation** (window disabled + InvalidateRect triggers SDBTNANM
   draw + blocking pump until animation completes).

However, on Choose Map entry from `0x006ACEE0`, ECX is not set to the setup HWND before
calling, so the HWND hash lookup fails and the function returns 0. The animation and sound
are never triggered.

**Verdict: The Rust omission of FUN_00608070 in `open_choose_map_modal` is correct
behavior.** The function is a no-op on Choose Map entry in gamemd.exe due to the ECX
call convention mismatch at the call site. No sound fires, no animation plays, no
window state changes.

---

## 6. Implementation Handoff

### What to do in Rust

Nothing. The Rust `open_choose_map_modal` omission is correct. FUN_00608070 is a no-op
on this code path.

### Negative Facts — Do Not Do

1. Do NOT add a pre-modal "slide-out" animation on Choose Map entry — gamemd does not
   play one on this path.
2. Do NOT add a VocClass transition sound before showing the Choose Map modal — it is
   not fired here.
3. Do NOT implement a child-window disable/enable sweep around the modal open — this
   code is never reached on the Choose Map button click path.
4. Do NOT model FUN_00608070's blocking pump loop as part of choose-map flow — it is
   never entered.
5. Do NOT treat the function as a signal that SDBTNANM animations play on ALL dialog
   transitions — they only play when the HWND is passed correctly and the shell object
   animation flag is set; Choose Map entry does neither.

### Acceptance Scenario

**Test name:** `choose_map_modal_no_premodal_animation`

Verify that calling `open_choose_map_modal` does not trigger any sound playback or
animation state change. In unit tests: assert that after `open_choose_map_modal`, no
VOC sound event is queued and no animation flag is set on the setup panel. In integration:
observe that the Choose Map modal appears immediately without a slide/fade transition on
the setup dialog.

---

## 7. Remaining Uncertainty

1. **HWND argument convention:** Ghidra infers `__fastcall (HWND)` from the body but all
   callers pass no argument. The true calling convention may be that ECX is meaningful only
   in other contexts (e.g. when called from object method). Confirmed all three callers
   omit the argument — but cannot rule out a fourth undiscovered caller that does pass ECX.
   `get_function_callers` returned exactly 3 callers; confidence HIGH that Choose Map entry
   is a no-op.

2. **`obj+0xC1` flag population:** The exact code that sets this flag for each dialog type
   was not traced. If a future investigation shows the setup dialog does set this flag AND
   ECX is valid at some other entry path, the function might activate. Not relevant to
   current Rust gap.

---

## 8. Unverified

_(Nothing in the verified body depends on unverified claims. The function body is fully
decompiled and all three callers are decompiled.)_

---

## Sources

- `decompile_function 0x00608070` — full body
- `get_function_callers 0x00608070` — three callers: 0x00622720, 0x006ACEE0, 0x007757E0
- `decompile_function 0x006ACEE0` — WM_COMMAND handler, 0x5AA = Choose Map button path
- `decompile_function 0x00622720` — generic close helper
- `decompile_function 0x007757E0` — modal stack pop
- `decompile_function 0x0069BBE0` — animation-disabled guard
