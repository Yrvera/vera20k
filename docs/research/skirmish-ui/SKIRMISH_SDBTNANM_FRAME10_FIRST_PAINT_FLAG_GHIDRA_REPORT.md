# Skirmish SDBTNANM Frame-10 First-Paint Flag - Ghidra Report

**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0072E450`, `0x00608440`, `0x00623340`, `0x0060CF00`, `0x0060C540`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact initialization and first-paint value of the state byte that decides whether `RightPanel__Draw` emits repeated `SDBTNANM.SHP` frame `10` overlays for standard offline Skirmish dialog `0x102`.  
**Non-Scope:** all other right-panel chrome, child controls, WOL visual semantics, runtime screenshot capture, and broad writer sweeps outside the checked first-paint path.  
**Confidence:** High for standard offline `0x102` first paint; Medium-high for global writer inventory because indirect/runtime watchpoints were not taken.  
**Active in YR:** Yes for the offline Skirmish read path and first-paint negative result; Conditional for the overlay body in WOL-family dialogs where the gate byte is set.

## 1. Overview

The standard offline Skirmish launcher reaches dialog resource `0x102` with proc `0x006AE3F0`. On `WM_PAINT`, the common shell path calls `WM_PAINT_Handler`, which reads one byte from the dialog record at data offset `+0xD4` and passes the inverted boolean into `RightPanel__Draw`.

For a fresh offline Skirmish `0x102` first paint, that byte is still zero from record initialization. Zero becomes `param_3 = 1`, and `RightPanel__Draw` skips the `SDBTNANM.SHP` frame-10 loop. Rust currently forces the overlay active, so it diverges from gamemd first paint.

## 2. Key Offsets

| Field | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| Dialog data gate byte | data `+0xD4` | Nonzero enables frame-10 overlay by making caller pass `param_3 = 0`; zero skips overlay. | `0x00621FEC` loads `[EBX+0xD4]`; `0x00621FF6` `SETZ`; `0x00621FFE` calls `0x0072E450`. | Yes, read during standard offline `0x102` paint. |
| Hash/root record alias | root `+0xD8` | Same physical byte when addressing starts at the HWND-keyed hash record rather than data pointer. | `FUN_00608440 @ 0x00608440` adds `0x4`, then writes `[EAX+0xD4] = 1`, equivalent to root `+0xD8`. | Conditional; active only where setter is called. |
| Separate nearby bytes | data `+0xD5..+0xD8` | Other shell flags such as sidebar/minimap/radar helpers; not this gate. | `WM_PAINT_Handler` later checks data `+0xD5`, `+0xD6`, and root `+0xDB`. | Yes, but out of this gate's scope. |

## 3. Core Logic

Reader in `WM_PAINT_Handler @ 0x00621E90`:

```text
gate = *(byte *)(dialog_data + 0xD4)
param_3 = (gate == 0)
RightPanel__Draw(..., param_3)
```

Assembly spot-check:

- `0x00621FEC`: `MOV DL, byte ptr [EBX + 0xD4]`
- `0x00621FF4`: `TEST DL, DL`
- `0x00621FF6`: `SETZ AL`
- `0x00621FF9`: `PUSH EAX`
- `0x00621FFE`: `CALL 0x0072E450`

Draw gate in `RightPanel__Draw @ 0x0072E450`:

```text
if param_3 == 0:
    for each right-panel tile row:
        CC_Draw_Shape(g_SDBTNANM_SHP, frame 10, ...)
```

Assembly spot-check:

- `0x0072E5E2`: loads the caller flag byte from stack.
- `0x0072E5E6`: tests the flag.
- `0x0072E5E8`: jumps over the overlay block when nonzero.
- `0x0072E635`: pushes frame literal `0xA` before the `CC_Draw_Shape` call at `0x0072E63A`.

**Active in YR:** Yes. This exact read and draw branch is on the standard offline `0x102` `WM_PAINT` path; the overlay body itself is Conditional and only runs when the gate byte is nonzero.

## 4. Initialization And First Paint

`FUN_00623340 @ 0x00623340` zero-fills the dialog record before setting unrelated defaults:

- `0x00623344`: loads `ECX = 0x80`
- `0x00623349`: clears `EAX`
- `0x0062334D`: repeats `STOSD`

The decompile shows a `0x80` dword clear, then writes defaults at dword offsets `0x1A`, `0x19`, `0x0F`, `0x10`, `0x17`, and `0x24`. It does not write the gate byte after the clear.

The standard offline Skirmish path reaches the dialog as:

- `0x006AE317`: calls `FUN_0072CF40`
- `0x006AE31C`: loads proc `0x006AE3F0`
- `0x006AE321`: loads dialog id `0x102`
- `0x006AE328`: calls `FUN_00622650`

The `0x102` initialization helpers checked in this slice do not set data `+0xD4`:

- `FUN_0060CF00 @ 0x0060CF00` handles dialog `0x102` parent background/convert fields (`+0x1E`, `+0x39`, `+0x3A`) only.
- `FUN_0060C540 @ 0x0060C540` includes dialog `0x102`, writes paint mode `piVar3[0x2D] = 1`, and writes data byte `+0xC1`; not data `+0xD4`.
- `FUN_006AE3F0 @ 0x006AE3F0` delegates common paint first, then runs Skirmish preview/start-position paint work; it contains no call to the frame-10 setter.

**Active in YR:** Yes. These functions are on standard offline Skirmish `0x102` creation and first-paint dispatch.

## 5. Setter Inventory

`FUN_00608440 @ 0x00608440` is a live setter for the same byte. It walks the `DAT_00AC1B00` HWND hash table and writes `1` to the gate byte:

- `0x0060848C`: `ADD EAX, 0x4`
- `0x00608493`: `MOV byte ptr [EAX + 0xD4], 0x1`

Direct xrefs to `0x00608440` in this Ghidra image:

| Call site | Evidence | Active in YR |
|---|---|---|
| `0x0078B808` | `MOV ECX, EBP`; `CALL 0x00608440`; prior report maps surrounding function to WOL dialog `0x113`. | Conditional, online/WOL path. |
| `0x0078BF87` | `MOV ECX, EBP`; `CALL 0x00608440`; prior report maps surrounding function to WOL dialog `0x113`. | Conditional, online/WOL path. |
| `0x00792DA6` | `MOV ECX, ESI`; `CALL 0x00608440`; prior report maps surrounding function to WOL custom-match refresh. | Conditional, online/WOL path. |
| `0x00793407` | `MOV ECX, EBP`; `CALL 0x00608440`; prior report maps surrounding function to WOL verify-connections refresh. | Conditional, online/WOL path. |

`get_function_xrefs 0x006084A0` returned no direct references in this image. No direct setter call appears in the standard offline Skirmish `0x102` launcher/proc/init/paint functions checked above.

**Active in YR:** Conditional. The setter is live in YR online/WOL UI paths, but not in standard offline Skirmish first paint.

## 6. Rust Implementation Status

Current Rust forces the overlay active:

- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:461` defines `right_panel_frame10_overlay_active`.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:464` returns `true`.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:544` uses that helper to emit overlay sprites.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs:100` loads optional `SDBTNANM.SHP` frame `10`.

**Parity result:** Rust forcing `true` diverges from gamemd standard offline Skirmish first paint. For standard offline `0x102` first paint, gamemd gate byte is zero and the overlay loop is skipped.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish launch to dialog `0x102` | verified | `0x006AE317..0x006AE328` | none |
| `0x006AE3F0` common paint delegation | verified | decompile calls `FUN_00622B50` before Skirmish-specific `WM_PAINT` work | none |
| `0x00622B50` `WM_PAINT` common dispatch | verified | decompile message `0x0F` path calls `WM_PAINT_Handler` | none |
| Gate byte read/inversion | verified | `0x00621FEC..0x00621FFE` | none |
| `RightPanel__Draw` overlay condition and frame | verified | `0x0072E5E2..0x0072E65C`, frame literal at `0x0072E635` | none |
| Initial gate value | verified | `FUN_00623340`, `0x00623344..0x0062334D` zero-fill | no runtime watchpoint, but static init is explicit |
| Direct setter inventory | verified for direct xrefs | `get_function_xrefs 0x00608440` returned four WOL-family call sites | indirect/runtime writers outside first-paint path not globally swept |
| Offline `0x102` pre-paint writers | verified-with-bounds | decompiled `0x0060CF00`, `0x0060C540`, `0x006AE3F0`, `0x00622B50` | no full binary byte-pattern write sweep |
| Rust first-paint overlay behavior | verified | source lines listed in section 6 | implementation change out of scope |

## 8. Open Questions - Final State

- [RESOLVED] Q1 - Which byte controls frame-10 overlay in the common right panel? Dialog data `+0xD4` / root `+0xD8`; evidence `0x00621FEC`, `0x00608493`.
- [RESOLVED] Q2 - Does zero mean draw or skip? Zero skips; evidence `0x00621FF6 SETZ` plus `RightPanel__Draw` branch at `0x0072E5E8`.
- [RESOLVED] Q3 - What is standard offline `0x102` first-paint value? Zero; evidence `FUN_00623340` zero-fill and no same-byte write in checked `0x102` init path.
- [RESOLVED] Q4 - Does gamemd first paint emit `SDBTNANM.SHP` frame 10 in offline Skirmish? No; evidence `0x006AE317..0x006AE328`, `0x00621FEC..0x00621FFE`, `0x0072E5E2..0x0072E65C`.
- [RESOLVED] Q5 - Does Rust forcing active match gamemd first paint? No; evidence Rust `right_panel_frame10_overlay_active` returns `true` at `src/app_skirmish_shell_render.rs:461-464`.
- [DEFERRED] Q6 - Are there indirect or runtime-only writers to the same byte outside this first-paint slice? Category: bounded-cost-too-high. A hardware watchpoint or full data-flow sweep would be needed for a global claim; it is not needed to answer standard offline `0x102` first paint.

## Sources

- Ghidra decompile / assembly context: `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0072E450`, `0x00608440`, `0x00623340`, `0x0060CF00`, `0x0060C540`.
- Ghidra xrefs: `get_function_xrefs 0x00608440`, `get_function_xrefs 0x006084A0`.
- Prior docs read: `SKIRMISH_SHELL_CHROME_800X600_TRACE.md`, `SKIRMISH_RIGHT_PANEL_SDBTNANM_FRAME10_STATE_FLAG_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_BACKGROUND_OVERLAY_PLACEMENT_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_RIGHT_PANEL_SHELL_ASSET_PALETTE_SELECTION_GHIDRA_REPORT.md`.
- Rust source checked: `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs`.
