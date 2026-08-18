# SKIRMISH_BTN_MINS_PLUS_USE_SITE - Ghidra Research Report

**Address(es):** `0x0083FDB8`, `0x0083FDC8`, executable use-site block `0x006B1B30..0x006B1CFA`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `FUN_006AE6E0 @ 0x006AE6E0`, `FUN_006ACEE0 @ 0x006ACEE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Resolve whether `BTN-MINS.SHP` and `BTN-PLUS.SHP` have a static table/use-site in the Skirmish shell or adjacent owner-draw shell code, whether they are active in standard offline YR Skirmish, and what controls/actions they would correspond to if live.  
**Non-Scope:** Global proof of every non-Skirmish screen that might instantiate `SliderClass`; retail asset search beyond prior archive-probe evidence; implementing or changing Rust.  
**Confidence:** High for static use-site and standard offline Skirmish non-use; Medium for global `SliderClass` reachability outside this scoped shell path.  
**Active in YR:** Conditional overall. The generic `SliderClass` use-site exists in `gamemd.exe`, but standard offline YR Skirmish dialog `0x102` does not use these SHPs; it uses Win32 `msctls_trackbar32` controls painted by `OwnerDraw_Trackbar_0061D950`.

## 1. Overview

`BTN-MINS.SHP` and `BTN-PLUS.SHP` are not just inert strings. Ghidra's normal xref recovery misses them because the containing executable block is not defined as a function, but byte-pattern search finds direct immediate loads of both string addresses inside the generic `SliderClass` constructor path.

Active in YR: No for standard offline Skirmish dialog `0x102`. Evidence: Skirmish initializes and applies controls `0x529`, `0x511`, and `0x50C` through Win32 trackbar messages and `OwnerDraw_Trackbar_0061D950`, whose paint path uses `trakgrip.pcx` and `trof*.pcx`, not these SHPs.

## 2. Static Strings and Use-Site

| Item | Evidence | Finding | Active in YR |
|---|---|---|---|
| `BTN-MINS.SHP` string | `search_strings` at `0x0083FDB8`; `get_bulk_xrefs` returns no Ghidra xrefs | The string is present in the `Skirmish.cpp` string cluster, but not cross-referenced by Ghidra's recovered xref model | Conditional: data exists; not active by itself |
| `BTN-PLUS.SHP` string | `search_strings` at `0x0083FDC8`; `get_bulk_xrefs` returns no Ghidra xrefs | Same status as minus string | Conditional: data exists; not active by itself |
| direct use-site | byte-pattern search for little-endian `0x0083FDC8` finds `0x006B1B90`; search for `0x0083FDB8` finds `0x006B1BCA` | The executable constructor block loads the plus string first, then the minus string | Conditional: active only when this generic slider constructor path is instantiated |
| static table question | `inspect_memory_content @ 0x0083FD70` and immediate-load sites at `0x006B1B90/0x006B1BCA` | No pointer table was found for these two strings in this slice; the material use-site is direct code immediates, not a data table | Conditional |

Important correction to prior docs: prior reports were right that normal Ghidra xrefs were absent, but byte-pattern search resolves an executable use-site. The unresolved part is not "no use-site"; it is "use-site exists in generic `SliderClass`, not in offline Skirmish's live Win32 trackbar shell path."

## 3. Generic SliderClass Use-Site

The executable block beginning around `0x006B1B30` behaves as a `SliderClass` constructor. Ghidra has no function object there, but the block writes the `SliderClass` vtable pointer `0x007ED21C`, calls the gauge/control base constructor, and allocates two `0x60`-byte `ShapeButtonClass` children when the constructor is not passed external button objects.

| Detail | Evidence | Behavior | Active in YR |
|---|---|---|---|
| plus button construction | `0x006B1B90` loads `0x0083FDC8`; `ShapeButtonClass__Constructor @ 0x0069DD30` consumes loaded shape pointer and dimensions | Plus button is stored at slider field `+0x3C` | Conditional: only for generic `SliderClass` instances using built-in shape buttons |
| minus button construction | `0x006B1BCA` loads `0x0083FDB8`; same constructor path | Minus button is stored at slider field `+0x40` | Conditional |
| child registration | same block calls each child through virtual slots `+0x84`, `+0x0C`, and `+0x48` after construction | The plus/minus buttons are attached/refreshed as child gadgets of the slider | Conditional |
| destructor cleanup | `SliderClass` destructor-like function at `0x006B1D00` deletes pointers at `+0x3C` and `+0x40` before calling `GadgetClass__Constructor`/base cleanup | The two SHP-backed child buttons are real owned objects, not transient locals | Conditional |

The constructor skips allocation/loading of these two shape buttons when its "external/supplied button" condition is set; in that case the field at `+0x44` is set and the `BTN-*` load block is bypassed.

Active in YR: Conditional. This is real executable code in `gamemd.exe`, but this slot found no evidence that the standard offline Skirmish dialog instantiates this generic gadget slider.

## 4. If Live: Controls and Actions

| Control/action | Evidence | Meaning | Active in YR |
|---|---|---|---|
| `BTN-PLUS.SHP` child | Event-routing method at `0x006B2160` compares event source against field `+0x3C` | If the event source is the plus child and the event mask includes bit `0x04`, the slider calls virtual slot `+0xB0` with argument `0` | Conditional: generic slider only, not Skirmish `0x102` |
| `BTN-MINS.SHP` child | Same method compares event source against field `+0x40` | If the event source is the minus child and the event mask includes bit `0x04`, the slider calls virtual slot `+0xB0` with argument `1` | Conditional |
| one-step change | vtable `0x007ED21C + 0xB0` points to block `0x006B2040` | Argument `0` increments the current gauge/slider value by one; argument `1` decrements it by one; both route through the clamp/set path | Conditional |
| page/track click | vtable `+0xAC` points to block `0x006B2000`; mouse handler `FUN_006B1F50` calls it when the pointer is before/after the thumb | Track-area clicks adjust by the stored larger step/page amount at field `+0x48`, separate from the SHP buttons' one-step action | Conditional |
| clamp/set | `FUN_004E25A0 @ 0x004E25A0` clamps value to `0..param_1[0x0C]` and invalidates via virtual `+0x48` when changed | Plus/minus cannot push the value outside the slider's range | Conditional |

So if this slider class is live in a screen, `BTN-PLUS.SHP` is the one-step increment button and `BTN-MINS.SHP` is the one-step decrement button.

## 5. Standard Offline Skirmish Non-Use

Standard offline Skirmish's three sliders are not `SliderClass` gadgets. They are Win32 common-control trackbars in dialog `0x102`, routed by the owner-draw framework to `OwnerDraw_Trackbar_0061D950`.

| Skirmish control | Evidence | Live art/action path | Active in YR |
|---|---|---|---|
| `0x529` game speed | `FUN_006AE6E0` sends range `0x406` and position `0x405`; `FUN_006ACEE0` reads `TB_GETPOS`-style message `0x400` and stores `6 - pos` | `OwnerDraw_Trackbar_0061D950`; uses `trakgrip.pcx` and optional `trofl/trofm/trofr.pcx` numeric plaque | Yes |
| `0x511` credits | `FUN_006AE6E0` sends range from Rules `+0x1480/+0x1488`, position from `DAT_00A8B25C`, step via `0x4AB` | Same PCX owner-draw trackbar callback | Yes |
| `0x50C` unit count | `FUN_006AE6E0` sends range from Rules `+0x1490/+0x1498`, position from `DAT_00A8B270` | Same PCX owner-draw trackbar callback | Yes |
| owner-draw trackbar art | `OwnerDraw_Trackbar_0061D950` calls `FUN_006BA140("trakgrip.pcx")`, `FUN_006BA140("trofl.pcx")`, `FUN_006BA140("trofm.pcx")`, and `FUN_006BA140("trofr.pcx")` | No reference to `BTN-MINS.SHP` or `BTN-PLUS.SHP` in the live Skirmish trackbar paint path | Yes for PCX path; No for `BTN-*` |

Active in YR: Yes for the PCX trackbar controls; No for `BTN-MINS.SHP` / `BTN-PLUS.SHP` in standard offline Skirmish. Evidence: `Main_Game @ 0x0052D9A0` reaches the offline Skirmish modal path for `g_GameMode == 5` per `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`, and that active path initializes/reads the three Win32 trackbars above.

## 6. Asset Availability

The prior archive probe in `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md` did not resolve either `BTN-MINS.SHP` or `BTN-PLUS.SHP` in the configured retail RA2/YR install. This is supporting evidence only; the primary standard-Skirmish non-use proof is the live dialog/control path.

Active in YR: No for standard offline Skirmish. Evidence: active Skirmish trackbar art is PCX-based; prior archive probe also reports both `BTN-*` assets missing from the configured retail stack.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BTN-MINS.SHP` / `BTN-PLUS.SHP` string addresses | verified | `search_strings`, `0x0083FDB8`, `0x0083FDC8` | none |
| direct use-site search | verified | byte-pattern hits at `0x006B1B90`, `0x006B1BCA` | none |
| static pointer table hypothesis | verified-negative | `get_bulk_xrefs` empty; memory around `0x0083FD70` has strings, not recovered pointer table | none for this slice |
| generic slider constructor use-site | verified | executable block `0x006B1B30..0x006B1CFA`, vtable write `0x007ED21C`, `ShapeButtonClass__Constructor @ 0x0069DD30` | no global instantiation inventory outside scope |
| plus/minus event action | verified | event method `0x006B2160`; vtable `+0xB0 -> 0x006B2040`; clamp setter `FUN_004E25A0` | none |
| standard offline Skirmish trackbars | verified | `FUN_006AE6E0`, `FUN_006ACEE0`, `OwnerDraw_Trackbar_0061D950`; prior `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md` | none |
| global non-Skirmish `SliderClass` reachability | deferred | scope excludes all other shell/game screens | separate broad gadget inventory if needed |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Do the strings have a use-site despite missing Ghidra xrefs? Yes. Byte-pattern search finds direct immediate loads at `0x006B1B90` and `0x006B1BCA` inside an executable `SliderClass` constructor block. Active in YR: Conditional.  
[RESOLVED] OQ2 - Is the use-site a static table? No table was found in this slice; the material use-site is direct code immediates. Active in YR: Conditional.  
[RESOLVED] OQ3 - Are the strings used by standard offline Skirmish trackbars? No. Active in YR: No for `BTN-*` in Skirmish; evidence is the `0x006AE6E0`/`0x006ACEE0` -> `OwnerDraw_Trackbar_0061D950` PCX path.  
[RESOLVED] OQ4 - What would the buttons do if live? `BTN-PLUS.SHP` increments the slider by one; `BTN-MINS.SHP` decrements by one; both clamp through `FUN_004E25A0`. Active in YR: Conditional.  
[DEFERRED] OQ5 - Which non-Skirmish screens instantiate this generic `SliderClass` path? Category: out-of-scope; this slot was limited to Skirmish shell/adjacent owner-draw shell code.

## Sources

- Ghidra `search_strings`: `BTN-MINS.SHP @ 0x0083FDB8`, `BTN-PLUS.SHP @ 0x0083FDC8`.
- Ghidra `get_bulk_xrefs`: no recovered xrefs to `0x0083FDB8` / `0x0083FDC8`.
- Ghidra byte-pattern search: `0x0083FDC8` immediate at `0x006B1B90`; `0x0083FDB8` immediate at `0x006B1BCA`.
- Ghidra memory/decode: executable block `0x006B1B30..0x006B1CFA`; vtable pointer `0x007ED21C`.
- Ghidra decompile: `ShapeButtonClass__Constructor @ 0x0069DD30`.
- Ghidra decompile: `FUN_004E25A0 @ 0x004E25A0`.
- Ghidra decompile: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`.
- Ghidra decompile: `FUN_006AE6E0 @ 0x006AE6E0`.
- Ghidra decompile: `FUN_006ACEE0 @ 0x006ACEE0`.
- Prior docs: `SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`.
