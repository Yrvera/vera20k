# ButtonFadeEffect Visual Mechanism — Ghidra Report

**Investigation target:** What each per-frame tick of a ButtonFadeEffect does to the screen,
duration in ticks/ms, and easing curve.

**Status:** PARTIAL — visual mechanism class confirmed (SHP-frame toggle), draw site
confirmed, duration confirmed (1000 ms). DynamicVectorClass<ButtonFadeEffect*> global
instance and its iterator/draw caller were not found in static analysis. Exact struct layout
not investigated (slot 1 scope).

---

## 1. Mechanism Class: SHP Frame Toggle (NOT alpha blend)

The ButtonFadeEffect visual is **a binary SHP-frame swap at 1 Hz**, not a smooth alpha ramp.

**No variable-alpha fade was found.** All `AlphaBlendRect` callers in the button/sidebar
system use constant alpha values:
- `AlphaBlendRect(0, 0x80)` — 50% black overlay (disabled-state darkening)
- `AlphaBlendRect(0, 0x9f)` — ~63% overlay (text background tint)
- `AlphaBlendRect(0, 0xaf)` — ~69% overlay (tooltip backgrounds)

None of these are driven by a per-tick counter. No palette rotation or per-pixel color
blend loop was found in any button-draw function.

---

## 2. SHP Asset and Frame Usage

SHP file: `SDBTNANM.SHP` (loaded at global `g_SDBTNANM_SHP`, address resolved via
`CDFileClass__Constructor` calls in `Sidebar_RightPanel_SHP_Loading` @ 0x0072EB50,
verified via `decompile_function 0x0072EB50`).

Frame assignments confirmed in `OwnerDraw_Button_00612B70` (@ 0x00612B70,
verified via `decompile_function 0x00612B70`):

| Frame | Meaning |
|-------|---------|
| 2     | Normal (idle, non-pressed) |
| 3     | Flash / attention state (alternates with frame 2 at 1 Hz) |
| 4     | Pressed |
| 10    | Panel background. PATCHED 2026-05-20: previously stated as "drawn unconditionally" — actually drawn only when `RightPanel__Draw` at `0x0072E450` sees `param_3 == '\0'`. Verified via `decompile_function 0x0072E450`. |

Frames 0–1, 5–9, 11–16 of SDBTNANM are not drawn in any confirmed code path.

---

## 3. Draw Logic (OwnerDraw_Button_00612B70, WM_PAINT path)

From `decompile_function 0x00612B70` (WM_PAINT case, `param_2 == 0xF`):

```c
iVar14 = piVar17[0x2c];  // "fade type" field at byte offset 0xB0 of per-HWND struct
if (iVar14 == 1) {
    piStack_c4 = FUN_0072E2C0();     // sidebar layout rect
    piStack_dc = g_SDBTNANM_SHP;
    local_f0 = 0x2;                  // default: frame 2 (normal)
    if (!pressed) {
        if (*(char *)((int)piVar17 + 0xC5) != '\0')  // flash bool
            local_f0 = 0x3;          // flash frame
    } else {
        local_f0 = 0x4;              // pressed frame
    }
} else if (iVar14 == 2) {
    // uses DAT_00B0F9EC SHP, frames 0/1/2
} else if (iVar14 == 3) {
    // uses DAT_00B0FACC SHP, frames 0/1
}
// Then:
CC_Draw_Shape(piStack_dc, local_f0 /*frame*/, ...);  // draw call
```

The `piVar17[0x2c]` (= `*(int*)(data + 0xB0)`) is the "button draw style" selector. A value of
1 means SDBTNANM-style cameo button. The flash bool at `+0xC5` drives the frame choice.

---

## 4. Duration and Timer Mechanism

**Duration: 1000 ms per toggle** (effectively a 0.5 Hz visual cycle, 1 Hz bool flip).

From `OwnerDraw_Button_00612B70`, message 0x4DC handler (verified via
`decompile_function 0x00612B70`):

```c
case 0x4DC:
    if (param_4 == 1) {               // enable flash
        if (piVar17[0x31] == '\0') {  // not already running
            *(byte*)(piVar17 + 0x31) = 1;
            SetTimer(param_1, 0, 1000, (TIMERPROC)0x0);
        }
    } else {                          // disable flash
        *(byte*)(piVar17 + 0x31) = 0;
        *(byte*)(piVar17 + 0xC5) = 0;
        KillTimer(param_1, 0);
        InvalidateRect(param_1, (RECT*)0x0, 1);
    }
```

WM_TIMER (0x113) handler (same function):

```c
case 0x113:
    // (corrected 2026-05-28: was *(bool*)(piVar17 + 0xC5) ^= true;
    //  binary uses boolean-NOT assignment, not XOR:
    //  verified via decompile_function 0x00612B70 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
    *(bool*)((int)piVar17 + 0xC5) = *(char*)((int)piVar17 + 0xC5) == '\0';
    InvalidateRect(param_1, (RECT*)0x0, 1);
```

**There is no counter, no lerp, no easing curve.** The effect is: frame 2 for 1000 ms,
frame 3 for 1000 ms, repeat indefinitely until 0x4DC(0) is received.

---

## 5. RTTI Container Globals (from parent investigation)

| Address    | Type descriptor string |
|------------|------------------------|
| 0x00820428 | `.?AV?$VectorClass@PAUButtonFadeEffect@@@@` |
| 0x00820460 | `.?AV?$DynamicVectorClass@PAUButtonFadeEffect@@@@` |

Both share the MSVC type_info vtable at 0x007F9594.

**Status: The DynamicVectorClass<ButtonFadeEffect*> global instance and its per-frame
walker were NOT located in this investigation.** The per-button flash is driven by
Windows `SetTimer` per-HWND, not by a central DynamicVector walk. Possible explanations:
(a) ButtonFadeEffect is used for a *different* animation path not reached by the cameo
flash (e.g., a state-transition crossfade triggered by a different event), or
(b) the vector is walked from an unresolvable code segment (function not picked up by
Ghidra's analysis).

---

## 6. Active in Standard YR

**PATCHED 2026-05-20 — verdict reversed.** The previous version of this section claimed
the 1 Hz SDBTNANM frame toggle fires for in-game cameo "attention" state via a
`StripClass__AI` → `SendMessageA(0x4DC)` caller chain. **This is not what the binary does.**

Live audit findings (verify-doc-swarm 2026-05-20):

- `StripClass__AI @ 0x006A8B30` does NOT call `SendMessageA(hwnd, 0x4DC, ...)`. The
  factory-complete branch (case 6, RTTI=AIRCRAFT) calls `VoxClass__PlayEVA`,
  `FUN_00734250`, and `FUN_0069dfc0` (the tab-flash scheduler). Zero `SendMessageA`
  in `StripClass__AI` or `SidebarClass__Action`. Verified via
  `decompile_function 0x006A8B30` and `decompile_function 0x006A7780`.
- `OwnerDraw_Button_00612B70` is registered only as a `WNDPROC` for Windows
  owner-drawn buttons whose style satisfies `(WS & 0xB) == 0xB` (verified via
  `FUN_0060F9A0` decompile). StripClass cameos are drawn directly to a surface via
  `CC_Draw_Shape` — they are not `HWND` buttons and do **not** route through this
  WndProc.
- The six `push 0x4DC` sites in the binary are all in **network-lobby dialog**
  helpers (`FUN_005e2340` / `FUN_005e23a0` referencing `s_D__ra2mdpost_netdlg2`;
  `FUN_007a2750` / `FUN_007a27d0` referencing `s_D__ra2mdpost_wonline`), all using
  `GetDlgItem(hwnd, 0x59f)`. None of the six are in main-menu code; none are in
  in-game sidebar code.

**Corrected verdict:** the SDBTNANM 1 Hz toggle and the `0x4DC` SetTimer mechanism
documented in §3-§4 are active in YR **only for the network-lobby owner-drawn
buttons** (lobby join/host dialog). They are **not** the in-game cameo or tab flash
mechanism.

The actual in-game tab-flash mechanism (per
[SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md](SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md))
uses gadget-local fields `+0x34`/`+0x38`/`+0x3c` driven by `FUN_0069DFC0` /
`FUN_0069DFF0` / `FUN_0069E010`, with a 10-tick period scheduled from
`StripClass::AI` on aircraft-completion or super-weapon-ready. **It is not related
to this `ButtonFadeEffect` family at all.**

The in-game cameo-level flash field `CameoEntry.FlashEndFrame` is dead code (no
non-zero writer exists in YR — per `CAMEO_FLASH_END_FRAME_WRITER_GHIDRA_REPORT.md`),
so there is no `0x4DC`-style cameo flash either.

---

## 7. Confidence

| Claim | Confidence | Source |
|-------|-----------|--------|
| Mechanism is SHP frame toggle, not alpha ramp | HIGH (verified) | `decompile_function 0x00612B70` |
| Duration is 1000 ms per toggle | HIGH (verified) | `decompile_function 0x00612B70` SetTimer call |
| No easing curve — binary flip only | HIGH (verified) | WM_TIMER handler has no counter arithmetic |
| Frame 2 = normal, 3 = flash, 4 = pressed | HIGH (verified) | WM_PAINT draw path |
| SHP file is SDBTNANM.SHP | HIGH (verified) | `decompile_function 0x0072EB50` |
| DynamicVectorClass<ButtonFadeEffect*> unused | MEDIUM (not proven, absence of evidence) | Exhaustive caller search found no iterator |

---

## 8. Unverified / Open Questions

- **Where is `piVar17[0x2c]` (fade type) set?** The write site was not found. It is set
  at button creation or configuration time. The default constructor sets `[0x1a]` and
  other fields but not `[0x2c]`.
- **What is ButtonFadeEffect struct layout?** Slot 1 scope; not investigated here.
- **What does DynamicVectorClass<ButtonFadeEffect*> actually hold?** No iterator found.
  May be a secondary/transition effect system with a different code path.
- **Are SDBTNANM frames 5–9 / 11–16 used anywhere?** Confirmed unused in all found
  draw paths. May be for unused button types or TS-era legacy.

---

*Investigation by subagent slot 4 — re-swarm batch, 2026-05-19.*
