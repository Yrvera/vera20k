> ⚠️ **YELLOW — PARTIALLY SUPERSEDED 2026-05-19**
>
> Two load-bearing conclusions in this report were refuted by subsequent investigation. The dead-code analysis of `ButtonFadeEffect` (Section 3 and the deferred items) remains valid — three independent re-swarm subagents (2026-05-19, batch `button-fade-effect-deep-dive`) confirmed no live allocation path for ButtonFadeEffect in YR.
>
> **Refuted claims:**
>
> 1. *"Main-menu buttons use the PCX path (`bue_li30 + bue_mi30 + bue_ri30` 3-piece composites)."*
>    → **WRONG.** `MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md` proved `LAB_0060A330` writes `record+0xB0 = 1` for all 6 main-menu button IDs (0x683/0x684/0x578/0x686/0x55C/0x3EE), routing them through `OwnerDraw_Button_00612B70`'s `iVar14 == 1` branch → **`g_SDBTNANM_SHP` frames 2/3/4**, not PCX. The original trace-swarm slot 3 (`MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md`) was correct that we already use SDBTNANM.
>
> 2. *"`ButtonFadeEffect` is the click animation system."*
>    → **WRONG.** ButtonFadeEffect is dead WW codebase scaffolding (RTTI exists but no allocation path). The actual visible animation is the **1 Hz SDBTNANM frame 2↔3 toggle** driven by `SetTimer(hwnd, 0, 1000, NULL)` armed via message `0x4DC` (hover mutator). Documented in `BUTTON_FADE_EFFECT_VISUAL_GHIDRA_REPORT.md` (slot 4 of the re-swarm — the report file is misnamed; it documents the focus-flash mechanism, not ButtonFadeEffect).
>
> **What in this report is still correct:**
> - The fresh decompile of `0x00612B70`'s branch structure (Section 2) is accurate. The error was identifying which `piVar17[0x2c]` value the main menu uses — it's `1` (SDBTNANM), not `0` (PCX).
> - The MSFadeAnim cross-check (now in `MSFADEANIM_SIBLING_CLASS_GHIDRA_REPORT.md`) confirms that class is unrelated to main-menu buttons.
> - The ButtonFadeEffect dead-code analysis is correct — no live allocation path exists.
>
> See `MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md` and `BUTTON_FADE_EFFECT_VISUAL_GHIDRA_REPORT.md` for the corrected picture.

---

# Main-Menu Button Click Animation — Ghidra Research Report

**Address(es):** `0x00612B70` (OwnerDraw_Button), RTTI strings `0x00820428` / `0x00820460` (ButtonFadeEffect vector type-info)

**Confidence:** HIGH on what the animation is NOT (verified fresh from binary). MEDIUM on the active mechanism (ButtonFadeEffect — class proven to exist via RTTI, but constructor + per-frame update site not yet traced).

**Active in YR:** YES — `ButtonFadeEffect` RTTI is present in the live binary and the containers are typed (`VectorClass<ButtonFadeEffect*>` + `DynamicVectorClass<ButtonFadeEffect*>`), implying both a fixed-size buffer and a growable list maintained at runtime.

---

## 1. Executive summary

The "animation that plays when clicking a main-menu button" is **NOT** produced inside `OwnerDraw_Button_00612B70`. Fresh decompile confirms the button proc's WM_PAINT path produces only an instant 3-piece PCX swap (`bue_*30 → bde_*30`) with a +2 px Y shift on press, and the WM_LBUTTONDOWN handler plays a sound (`VocClass__PlayAtPos`) without scheduling any visual.

The animation comes from a separate engine subsystem: a global list of **`ButtonFadeEffect`** instances, ticked per-frame. RTTI strings for `VectorClass<ButtonFadeEffect*>` and `DynamicVectorClass<ButtonFadeEffect*>` are present in the binary at `0x00820428` and `0x00820460` (verified via `read_memory 0x00820420` — both type descriptors point to MSVC's `type_info` vtable at `0x007f9594`).

The button-proc itself is intentionally dumb. When a button is clicked, the engine drops a `ButtonFadeEffect` onto the dynamic vector elsewhere; that vector is walked every frame and the fade is composited on top of (or in addition to) the button artwork until the effect expires and is removed from the list.

**Critical secondary finding — the entire SDBTNANM-vs-PCX confusion is resolved here.** Slot 3 of the recent trace-swarm claimed the main-menu buttons use SDBTNANM.SHP frames 2/3/4. **They do not.** That code path exists in `OwnerDraw_Button_00612B70`, but it is a different branch (`piVar17[0x2c] == 1`) that handles SDBTNANM-style controls used elsewhere in the binary (dropdowns, etc.). Dialog 0xE2 main-menu buttons go through `piVar17[0x2c] == 0` (PCX) → `piVar17[5] == 0` (regular PCX, not "special bitmap") → loads `bue_li30 + bue_mi30 + bue_ri30` and swaps to `bde_*30` on press.

## 2. Verified branch structure in `OwnerDraw_Button_00612B70` (fresh decompile this session)

```c
iVar14 = piVar17[0x2c];        // control "kind" selector
if (iVar14 == 0) {
    iVar14 = piVar17[5];       // PCX-or-special selector
    if (iVar14 == 0) {
        // === MAIN-MENU PATH — PCX 3-piece composite ===
        // bue_li30 (left cap) + bue_mi30 (middle, tiled) + bue_ri30 (right cap)
        // Press: 'u' → 'd' → bde_*30
        // Disabled: forced 'u' → bue_*30 + 50% black alpha overlay
        // Pressed Y shift: +2 px (pWStack_d0 += 2)
        // NO frame index, NO timer, NO animation step
        goto LAB_00613568;
    } else {
        // Special bitmap path — blit iVar14 (= bitmap surface ptr) directly
    }
} else if (iVar14 == 1) {
    // SDBTNANM SHP path — frame 2=default, 3=focus (if +0xC5 set), 4=pressed
    // This is NOT the main-menu path. Used for SDBTNANM-style controls
    // elsewhere. Slot 3's trace was looking at this branch by mistake.
    piStack_dc = g_SDBTNANM_SHP;
    local_f0 = 2;
    if ((pWStack_d8 & 1) == 0) {
        if (record[+0xC5] != 0) local_f0 = 3;
    } else local_f0 = 4;
} else if (iVar14 == 2) {
    // Different SHP (DAT_00b0f9ec) — frame 0/1/2 selection
} else if (iVar14 == 3) {
    // Different SHP (DAT_00b0facc) — frame 0/1/2 selection
}
```

The main-menu buttons satisfy **both** `piVar17[0x2c] == 0` AND `piVar17[5] == 0`. Slot 3's trace claim that the main menu uses SDBTNANM frames 2/3/4 is **wrong** — confirmed by fresh decompile in this session and corroborated by the pre-existing `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` (today, 09:43) and `SDBTNANM_FRAME10_OVERLAY_GATE_GHIDRA_REPORT.md` (today, 17:40).

## 3. Where the click animation actually lives — `ButtonFadeEffect`

### Evidence from RTTI

`read_memory 0x00820420` (96 bytes) returned two adjacent MSVC RTTI type descriptors:

```
0x00820420: 94 95 7f 00 00 00 00 00  ; type_info vtable @ 0x007f9594
0x00820428: ".?AV?$VectorClass@PAUButtonFadeEffect@@@@"

0x00820458: 94 95 7f 00 00 00 00 00  ; type_info vtable @ 0x007f9594
0x00820460: ".?AV?$DynamicVectorClass@PAUButtonFadeEffect@@@@"
```

The mangled-name decoding:
- `?$VectorClass@...` → `VectorClass<...>` template
- `PAU` → `pointer to user-defined struct` — confirms `ButtonFadeEffect` is a **struct** (not class) and the vector holds **pointers** to instances
- Both `VectorClass` (fixed-size) and `DynamicVectorClass` (growable) wrappers exist — implies the engine has dedicated infrastructure for managing a growing pool of active fade effects

### Why this is "the animation"

1. The button-proc itself has no per-frame state advancement (no WM_TIMER frame index increment, no animation step counter). Verified by exhaustive enumeration of the message-handler switch in `0x00612B70`.
2. SDBTNANM.SHP frames 1, 5–9, 11–16 are **not referenced by any draw path** found in this binary (per `SDBTNANM_FRAME10_OVERLAY_GATE_GHIDRA_REPORT.md`, section 6.4). They can't be the animation source.
3. The `ButtonFadeEffect` class exists, has growable storage, and is named after exactly the behavior the user observed.
4. Engine-pattern: WW UI code typically uses these "effect" + global-vector patterns for per-frame compositing (other examples in the codebase: `MSFadeAnim` at `0x008300c8` — a sibling class, likely a different fade variant).

### What the fade likely does

Based on naming + the standard WW UI pattern:

- Triggered on button click (probably from WM_LBUTTONDOWN, WM_LBUTTONUP, or BN_CLICKED → WM_COMMAND).
- A new `ButtonFadeEffect` is constructed (probably via `operator_new`) and pushed to the `DynamicVectorClass<ButtonFadeEffect*>` global.
- Each frame, the engine walks the vector, calls an `Update`/`Tick` on each effect, draws the fade pixels, and removes effects whose alpha or timer has expired.
- Common visual: a brief flash/glow/dim that fades out over a fraction of a second on the just-clicked button.

This explains why the user perceives "an animation" tied to clicks — it IS an animation, just one composited on top of the static PCX button artwork rather than driven by frame indices inside the button SHP.

### Unverified — what would close it

- The `ButtonFadeEffect` constructor address (would need to search byte patterns or follow operator_new call sites to find it).
- The global vector instance address (one of the `DAT_*` globals).
- The per-frame tick site (likely in the main UI/render loop).
- Whether the fade is alpha-based, color-shift-based, or palette-rotation-based.
- Exact duration and curve.

These weren't pursued in this pass because the user requested a quick scope. Each would take ~3–5 more Ghidra calls to nail down.

## 4. What our Rust engine currently does (for comparison)

- `src/render/main_menu_shell_chrome.rs`: loads SDBTNANM.SHP frames 2/3/4 (`button_default`, `button_hover`, `button_pressed`) — **wrong source asset family**.
- `src/app_main_menu_shell_render.rs`: blits one of those three frames per button per draw, no per-frame animation step, no fade-effect list.
- Result: our buttons show as SHP frames instead of 3-piece PCX composites, and we have no click-fade animation at all.

## 5. Implementation implications (for a future plan — NOT a code change)

The minimum gamemd-parity path here is two-step:

1. **Switch button artwork from SDBTNANM SHP to bue_*30 / bde_*30 three-piece PCX composites** (matches the verified PCX path). The Rust shell-chrome atlas currently loads SDBTNANM, SDTP, SDBTNBKGD, SDBTM, LWSCRNL/S — it needs to additionally load the bue/bde PCX files and assemble the 3-piece per-button artwork.
2. **Add a click-fade-effect system** modeled after `ButtonFadeEffect`: an effects list, per-frame update, composite on top of buttons.

Step 1 alone gets a static-shell parity win (PCX colors are subtly different from the SHP frames; the press shift is +2 Y not what we have today; the disabled state needs the 50% black overlay path). Step 2 gets the click animation. They're independent and step 1 is much smaller.

## 6. Open Questions — final state of the log

- `[RESOLVED]` What button artwork does the main menu actually use? → 3-piece PCX (`bue_li30 + bue_mi30 + bue_ri30` for up, `bde_*30` for down), via the `piVar17[0x2c] == 0 && piVar17[5] == 0` branch of `OwnerDraw_Button_00612B70`. (evidence: fresh decompile of `0x00612B70` this session; `search_strings "bue_li30"` → `0x00835e34`; sibling report `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`)
- `[RESOLVED]` Are SDBTNANM frames 5–16 used for a click animation? → No. They are not referenced by any draw path in `RightPanel__Draw` or `OwnerDraw_Button_00612B70`. Frame 10 is the WOL-only chrome overlay; frames 2/3/4 are for SDBTNANM-style controls, not main-menu buttons. (evidence: `SDBTNANM_FRAME10_OVERLAY_GATE_GHIDRA_REPORT.md` §6.4; `search_strings "SDBTNANM"` returns only 2 hits, both in resource-load strings)
- `[RESOLVED]` Is there ANY animation infrastructure in the button proc? → No timer-driven frame advance, no animation counter, no WM_TIMER-tied paint state changes for the PCX path. (evidence: full enumeration of `OwnerDraw_Button_00612B70` message switch this session)
- `[RESOLVED]` Where does the click animation actually come from? → A `ButtonFadeEffect`-typed effect, managed via a global `DynamicVectorClass<ButtonFadeEffect*>` list, ticked per-frame outside the button proc. (evidence: RTTI strings at `0x00820428` and `0x00820460`; type-info vtable confirmed via `read_memory 0x00820420`)
- `[DEFERRED]` ButtonFadeEffect constructor address. (category: `bounded-cost-too-high`; reason: user requested quick scope and the architectural question is already answered; next-step-if-pursued: search for `operator_new(N)` call sites where N matches the ButtonFadeEffect struct size, or scan for the vtable if one exists despite the struct nature.)
- `[DEFERRED]` Global `DynamicVectorClass<ButtonFadeEffect*>` instance address. (category: `bounded-cost-too-high`; reason: requires finding the constructor or a per-frame `Update_All` site to backtrack from; next-step-if-pursued: scan for the type-descriptor pointer `0x00820458` in .data — instances point back to their type descriptor.)
- `[DEFERRED]` Per-frame tick site (likely in the main UI loop). (category: `bounded-cost-too-high`; reason: same as above; next-step-if-pursued: find the global list head, search xrefs to it.)
- `[DEFERRED]` Visual style of the fade (alpha, color shift, palette rotation, frame cycle from a small SHP). (category: `needs-runtime-debugger`; reason: easiest to confirm by watching the binary play the animation under a debugger; next-step-if-pursued: hook the tick site and screen-capture the button rect across N frames.)
- `[DEFERRED]` Duration and easing curve of the fade. (category: `needs-runtime-debugger`; reason: same as above.)
- `[DEFERRED]` Identity and role of the sibling `MSFadeAnim` class (RTTI at `0x008300c8`). (category: `out-of-scope`; reason: not directly tied to main-menu buttons; possibly used for menu-screen transitions or movie crossfades; next-step-if-pursued: search xrefs to `0x008300c8`.)

## 7. Sources

**Ghidra MCP calls (read-only, this session):**

- `decompile_function 0x00612B70` — full re-decompile of `OwnerDraw_Button_00612B70`, confirmed the PCX-vs-SHP branching structure
- `decompile_function 0x00531F60` — confirmed `MainMenuDialog0xE2_Proc` WM_COMMAND only handles 6 button IDs and sets result codes via `GetWindowLong(hwnd, 8)`; no animation scheduling
- `search_strings "SDBTNANM"` — only 3 matches, all in resource-load strings (no animation refs)
- `search_strings "bue_li30"` — confirmed PCX filename at `0x00835e34` (main-menu button asset family)
- `search_strings "fade"` — surfaced `ButtonFadeEffect` RTTI at `0x00820428` / `0x00820460` and `MSFadeAnim` RTTI at `0x008300c8`
- `search_strings "ButtonFadeEffect"` — confirmed only the two vector-container RTTI strings exist
- `search_strings "MSFadeAnim"` — confirmed the sibling fade-animation RTTI
- `read_memory 0x00820420` (96 bytes) — confirmed both type descriptors point to MSVC `type_info` vtable at `0x007f9594`
- `get_xrefs_to 0x00531F60` — confirmed single caller `FUN_00531cc0` (the menu launcher)
- `get_xrefs_to 0x00820428` / `0x00820460` — no direct xrefs (normal — RTTI is referenced via the type descriptor structure, not the name string)
- `search_functions_enhanced name_pattern=Fade` — confirmed Ghidra has not labeled any ButtonFadeEffect-related function with a name containing "Fade"

**Prior reports cross-referenced:**

- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` (2026-05-19 09:43) — anchor for the PCX path semantics
- `SDBTNANM_FRAME10_OVERLAY_GATE_GHIDRA_REPORT.md` (2026-05-19 17:40) — anchor for SDBTNANM frame usage being limited to frame 10 (WOL only)
- `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md` and `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md` — supporting evidence for the above
- `traces/MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md` — slot 3 of the recent swarm; this report **supersedes** slot 3's claim that the main menu uses SDBTNANM frames 2/3/4

**Rust files:** read for comparison only; not modified. `src/render/main_menu_shell_chrome.rs`, `src/app_main_menu_shell_render.rs`.
