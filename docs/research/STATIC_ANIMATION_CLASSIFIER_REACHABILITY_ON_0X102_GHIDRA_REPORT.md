# Static Animation Classifier Reachability on Dialog 0x102 - Ghidra Report

**Address(es):** `0x0060A5B0`, `0x00603240`, `0x006033F0`, `0x00603870`, `0x00608CD0`, `0x006153E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** whether the static-animation classifier/attach infrastructure previously noted for main-menu `(dialog 0xE2, ctrl 0x71C)` is reachable or relevant for standard offline Skirmish dialog `0x102`, and whether any `0x102` Static control uses the kind-4 SHP animation setup.  
**Non-Scope:** exact Skirmish text animation cadence, flag-image asset provenance, combo/button rendering, and unrelated shell dialogs.  
**Confidence:** High for the negative kind-4 finding; Medium for non-kind-4 Skirmish static classification details not fully expanded here.  
**Active in YR:** Conditional - the setup infrastructure is active in YR shell dialogs, including `0x102`; the kind-4 SHP branch is not active for `0x102`.

## 1. Overview

The Skirmish dialog `0x102` does run through the shared owner-draw/static setup passes after shell initialization. The previously studied kind-4 SHP attach branch is therefore reachable as infrastructure, but its internal predicates do not include dialog `0x102`, and no Skirmish Static control is armed as a kind-4 SHP animation.

Comparison point: main-menu `(0xE2, 0x71C)` is in the `0x71C` kind-4 predicate family; Skirmish `0x102` is not. Skirmish uses the same owner-draw static procedure and some adjacent classifier families for text/static layout, but not the `0x71C` kind-4 SHP attach path.

## 2. Dialog 0x102 Static Controls

Verified from the `RT_DIALOG` resource in retail `gamemd.exe` (`RT_DIALOG` type `5`, resource id `0x102`, language `0x409`, DIALOGEX, 72 controls):

| Control id | Class | Rect | Style / ex-style | Title | Active in YR |
|---|---|---:|---|---|---|
| `0x694` | Static | `(425,1,108,10)` | `0x50020001 / 0` | `GUI:SkirmishGame` | Yes; PE dialog resource |
| `0x699` | Static | `(201,176,60,10)` | `0x50000000 / 0` | `GUI:GameSpeed` | Yes; PE dialog resource |
| `0x69B` | Static | `(201,193,60,10)` | `0x50000000 / 0` | `GUI:Credits` | Yes; PE dialog resource |
| `0x69C` | Static | `(201,210,60,10)` | `0x50000000 / 0` | `GUI:UnitCount` | Yes; PE dialog resource |
| `0x6DA..0x6E1` | Static | flag column | `0x50000005 / 0x20` | empty | Yes; PE dialog resource |
| `0x6EC` | Static | `(432,103,90,10)` | `0x50000201 / 0` | `GUI:None` | Yes; PE dialog resource |
| `0x695` | Static | `(2,355,410,12)` | `0x50000200 / 0` | `GUI:Blank` | Yes; PE dialog resource |
| `0x468` | Static | `(429,23,96,69)` | `0x50000004 / 0x20` | empty | Yes; PE dialog resource |
| `0x5A8` | Static | `(432,116,90,20)` | `0x50000001 / 0` | `GUI:None` | Yes; PE dialog resource |
| `0x796,0x791,0x792,0x793,0x794` | Static | column headers | `0x50020000 / 0` | header CSF keys | Yes; PE dialog resource |

No Static control with id `0x71C` exists in dialog `0x102`. Active in YR: Yes for absence, because this comes from the `RT_DIALOG 0x102` template.

## 3. Core Logic

### 3.1 Shared setup is live for Skirmish

`FUN_00622820` and `FUN_00622B50` enumerate shell child windows and install owner-draw procedures with `FUN_0060F9A0`, then enumerate children through `FUN_0060A330` and `FUN_0060A5B0`.

Evidence:

- `FUN_00622820 @ 0x00622820` calls `EnumChildWindows(param_1, FUN_0060F9A0, 0)`, then `EnumChildWindows(param_1, FUN_0060A330, 0)`, then `EnumChildWindows(param_1, FUN_0060A5B0, 0)`.
- The same function explicitly treats dialog id `0x102` as a shell/game-setup dialog by writing shell flags at record offsets `+0xD5` and `+0xD6`.
- `FUN_00622B50 @ 0x00622B50` has the same post-init enumeration of `FUN_0060A330` and `FUN_0060A5B0`.

Active in YR: Yes. This is the live shell owner-draw setup path used by shell dialogs, including `0x102`.

### 3.2 OwnerDraw Static proc is live, but does not by itself arm kind-4

`FUN_0060F9A0 @ 0x0060F9A0` maps the class string `"Static"` to `OwnerDraw_Static_006153E0 @ 0x006153E0`, stores the old WndProc, creates/fetches the per-control record, then sends `0x497`.

In `OwnerDraw_Static_006153E0`, message `0x497` initializes the static record with `record[0x1C] = 0` (kind reset), `record[0x2B] = 0x0C`, and `record[0x3B] = DAT_00AC18A4`. It does not set kind `4` or attach SHP data. Kind-4 SHP animation later requires `record[0x1C] == 4` and `record[0x1E] != 0`.

Active in YR: Yes for subclass/install/init. Active in YR for Skirmish kind-4 arming: No; no kind-4 write occurs in this function.

### 3.3 The kind-4 attach pass is live, but excludes dialog 0x102

`FUN_0060A5B0 @ 0x0060A5B0` is the live setup pass that can set:

- kind `1` when `FUN_00602490` matches;
- kind `2` when `FUN_00602B90` matches;
- kind `4` in the branch beginning around `0x0060A7CF..0x0060A987`.

The kind-4 branch's `0x71C` dialog-id set is:

`0x94, 0xD8, 0xF5, 0xE2, 0xD5, 0x101, 0x129, 0xD7, 0xBB, 0x100, 0xD6, 0x125, 0x122, 0x112, 0xE7, 0x116, 0x11D, 0x11C, 0xFE, 0x10F, 0x117, 0x114, 0xE6, 0xF3, 0xF4, 0x2BC, 0x10E, 0x108, 0xBC6, 0xBC7`.

It then requires `GetDlgCtrlID(child) == 0x71C` before setting `record[0x1C] = 4`. The same function also sets kind `4` for `(0x94, 0x6EA/0x6EC/0x6EB)`, `(0x103, 0x72B)`, `(0xBC7, 0x72B)`, and `(0xC4, 0x7A9)`.

Dialog `0x102` is not present in that kind-4 dialog-id set. No Skirmish Static id is `0x71C`, and the only overlapping id in the explicit kind-4 alternatives is `0x6EC`, but that alternative is gated by dialog `0x94`, not `0x102`.

Active in YR: Conditional. `FUN_0060A5B0` is active in YR; its kind-4 branch is active only for the listed dialog/control pairs. For standard Skirmish `0x102`, Active in YR: No.

### 3.4 The classifier functions agree with the attach branch

`FUN_00603240`, `FUN_006033F0`, `FUN_006035F0`, and `FUN_00603870` all use the same `0x71C` dialog-id family and omit `0x102`.

Specific findings:

- `FUN_00603240 @ 0x00603240` returns `1` for the `0x71C` family but not for `(0x102, any Skirmish Static)`. Active in YR: Yes for the function; No for `0x102` kind-4 timing.
- `FUN_006033F0 @ 0x006033F0` returns `100` for the `0x71C` family, for `(0x94, 0x6EA/0x6EC/0x6EB)`, for `(0x103/0xBC7, 0x72B)`, and `0x32` for `(0xC4, 0x7A9)`. It has no `(0x102, 0x6EC)` case. Active in YR: Yes for `OwnerDraw_Static_006153E0` case `0x4D3`; No for Skirmish `0x102`.
- `FUN_006035F0 @ 0x006035F0` selects filename/loading helpers for the same kind-4 targets and omits `0x102`. Active in YR: Yes for kind-4 targets; No for `0x102`.
- `FUN_00603870 @ 0x00603870` returns a SHP pointer for the same kind-4 targets and omits `0x102`. Active in YR: Yes for kind-4 targets; No for `0x102`.

### 3.5 `FUN_00608CD0` is relevant to 0x102, but only for layout/anchoring

`FUN_00608CD0 @ 0x00608CD0` does include dialog `0x102`, but its Skirmish cases are resize/layout predicates, not kind-4 SHP attach predicates:

- broad shell-title case: `(dialog 0x102, ctrl 0x694)` returns true;
- map preview case: `(dialog 0x102, ctrl 0x468)` returns true;
- explicit Skirmish case: `(dialog 0x102, ctrl 0x6EC/0x5AA/0x5A8/0x617)` returns true.

`ResizeShellChildControl_0060C0C0 @ 0x0060C0C0` calls `FUN_00608CD0` and then `FUN_0060B1D0` / related layout helpers. `FUN_0060A330` also calls it to set per-control layout group state (`record[0x2C] = 1` or `2` depending shell mode). No SHP pointer is attached by this path.

Active in YR: Yes for Skirmish layout. Active in YR for kind-4 SHP animation: No.

### 3.6 Message-driven kind-4 start does not rescue 0x102

`OwnerDraw_Static_006153E0` case `0x4D3` starts a kind-4 SHP animation only if:

1. `record[0x1E] != 0` (SHP data pointer attached);
2. `record[0x1C] == 4`;
3. `record[0x2A] == 0` (not already running).

The binary has `push 0x4D3` send sites at `0x004713C5`, `0x0052EE1E`, `0x0052EE58`, `0x0052EE98`, `0x0052F384`, `0x006C93C2`, and `0x00792DCF`. The visible control-id contexts target campaign/score animation controls such as `0x6EA`, `0x6EC`, `0x6EB`, and `0x72B`, not Skirmish dialog `0x102`. Since `FUN_0060A5B0` never sets Skirmish records to kind `4`, a `0x4D3` sent to a Skirmish Static would still early-out.

Active in YR: Yes for other dialogs. For standard offline Skirmish `0x102`, Active in YR: No.

## 4. INI Keys

No INI keys were found or needed for this reachability slice. The relevant evidence is in `gamemd.exe` shell resource templates and owner-draw/static setup code. Active in YR: Not applicable.

## 5. Integration Points

| Function / area | Role | Active in YR | Evidence |
|---|---|---|---|
| `FUN_00622820` | Shell dialog owner-draw setup and post-init child enumeration | Yes | decompile: calls `FUN_0060F9A0`, `FUN_0060A330`, `FUN_0060A5B0`; includes dialog `0x102` shell flags |
| `FUN_00622B50` | Shared shell WndProc; `WM_INITDIALOG` path enumerates owner-draw children | Yes | decompile: `EnumChildWindows(... FUN_0060F9A0 ...)`, then `FUN_0060A330`, `FUN_0060A5B0` |
| `FUN_0060F9A0` | Class-to-ownerdraw installer; `"Static"` -> `0x006153E0` | Yes | decompile and xref to `OwnerDraw_Static_006153E0` at `0x0060FDCC` |
| `FUN_0060A5B0` | Static animation/image attach classification | Yes, but kind-4 no for `0x102` | decompile of kind-4 branch excludes `0x102` |
| `OwnerDraw_Static_006153E0` | Static WndProc paint/timer/message handling | Yes | installed by `FUN_0060F9A0`; `0x4D3` requires kind `4` and SHP pointer |
| `FUN_00608CD0` | Layout/resize classifier | Yes for `0x102` | decompile includes `(0x102, 0x694/0x468/0x6EC/0x5AA/0x5A8/0x617)` |

## 6. Current Rust Implementation Status

Not scanned in this slot. The user scope prohibited modifying repo files, and this investigation is a binary reachability slice only.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RT_DIALOG 0x102` Static inventory | verified | PE resource parse of retail `gamemd.exe`, dialog id `0x102`, language `0x409` | none for this slice |
| `FUN_0060F9A0` Static ownerdraw install | verified | decompile `0x0060F9A0`, `"Static"` -> `OwnerDraw_Static_006153E0` | none |
| `FUN_0060A5B0` reachability | verified | xrefs from `FUN_00622820` and `FUN_00622B50`; decompile shows live setup pass | none |
| `FUN_0060A5B0` kind-4 branch on `0x102` | verified | decompile `0x0060A5B0`; kind-4 dialog-id set excludes `0x102` | none |
| `FUN_00603240` | verified | decompile `0x00603240`; `0x71C` family omits `0x102` | none |
| `FUN_006033F0` | verified | decompile `0x006033F0`; no `(0x102, 0x6EC)` or other Skirmish Static case | none |
| `FUN_006035F0` | verified | decompile `0x006035F0`; SHP filename/helper selection omits `0x102` | none |
| `FUN_00603870` | verified | decompile `0x00603870`; SHP pointer cases omit `0x102` | none |
| `FUN_00608CD0` | verified | decompile `0x00608CD0`; includes `0x102` layout cases only | none |
| `OwnerDraw_Static_006153E0` kind-4 start/paint preconditions | verified | decompile `0x006153E0`; `0x4D3` requires `record[0x1C] == 4` and `record[0x1E] != 0` | none |
| `0x4D3` send sites | touched-not-exhausted | byte pattern `68 D3 04 00 00` and assembly contexts | full caller semantics of every non-Skirmish send site out of scope |
| non-kind-4 Skirmish text/static animation details | deferred | `FUN_00602490`, `FUN_00602B90` touched | exact cadence/assets are a different target |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Is the `0x0060A338..0x0060AAxx` attach/setup region reachable? Yes; the relevant live function is `FUN_0060A5B0`, referenced as an `EnumChildWindows` callback from `FUN_00622820 @ 0x00622B33` and `FUN_00622B50 @ 0x0062307B`. Active in YR: Yes.

[RESOLVED] OQ2 - Does the live attach/setup pass set kind `4` for any dialog `0x102` Static? No; `FUN_0060A5B0` kind-4 predicates exclude `0x102`. Active in YR for Skirmish kind-4: No.

[RESOLVED] OQ3 - Does `FUN_00603240` / `FUN_006033F0` / `FUN_00603870` recognize `0x102` the way they recognize main-menu `0xE2,0x71C`? No; the `0x71C` family omits `0x102`, and the explicit alternate kind-4 controls are not Skirmish `0x102`. Active in YR for Skirmish kind-4: No.

[RESOLVED] OQ4 - Is `FUN_00608CD0` relevant to Skirmish `0x102`? Yes, but for layout/resize classification only, not SHP attach. Active in YR: Yes for layout; No for kind-4 SHP setup.

[RESOLVED] OQ5 - Does `OwnerDraw_Static_006153E0` independently start SHP animation for Skirmish statics? No; its `0x4D3` path early-outs unless the setup pass has already attached SHP data and set kind `4`. Active in YR for Skirmish kind-4: No.

[DEFERRED] OQ6 - Exact Skirmish non-kind-4 static animation cadence and flag-image assets. Category: out-of-scope. `FUN_00602490` and `FUN_00602B90` show Skirmish static classification, but this slot is only the kind-4 SHP reachability question.

## Sources

- Ghidra decompile: `FUN_00622820 @ 0x00622820`
- Ghidra decompile: `FUN_00622B50 @ 0x00622B50`
- Ghidra decompile: `FUN_0060F9A0 @ 0x0060F9A0`
- Ghidra decompile: `FUN_0060A330 @ 0x0060A330`
- Ghidra decompile: `FUN_0060A5B0 @ 0x0060A5B0`
- Ghidra decompile: `FUN_00602490 @ 0x00602490`
- Ghidra decompile: `FUN_00602B90 @ 0x00602B90`
- Ghidra decompile: `FUN_00603240 @ 0x00603240`
- Ghidra decompile: `FUN_006033F0 @ 0x006033F0`
- Ghidra decompile: `FUN_006035F0 @ 0x006035F0`
- Ghidra decompile: `FUN_00603870 @ 0x00603870`
- Ghidra decompile: `FUN_00608CD0 @ 0x00608CD0`
- Ghidra decompile: `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`
- Ghidra decompile: `OwnerDraw_Static_006153E0 @ 0x006153E0`
- Ghidra decompile: `FUN_006040B0 @ 0x006040B0`
- Ghidra xrefs: `OwnerDraw_Static_006153E0` referenced from `FUN_0060F9A0 @ 0x0060FDCC`
- Ghidra xrefs: `FUN_0060A5B0` referenced from `FUN_00622820 @ 0x00622B33`, `FUN_00622B50 @ 0x0062307B`
- Byte search: `C7 46 70 04 00 00 00` -> `0x0060A928`, `0x0060A949`, `0x0060A96A`, `0x0060A987`; no `C7 40/47/45 70 04 00 00 00` variants found
- Byte search: `68 D3 04 00 00` -> seven `0x4D3` send sites; visible contexts target non-Skirmish animation controls
- PE resource parse: retail `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, `RT_DIALOG 0x102`, language `0x409`
- Prior docs cross-checked: `STATIC_0X71C_RUNTIME_VISIBILITY_GHIDRA_REPORT.md`, `STATIC_0x71C_VISIBILITY_TRACE_GHIDRA_REPORT.md`, `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
