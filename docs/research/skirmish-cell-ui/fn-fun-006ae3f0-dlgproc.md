# FUN_006AE3F0 — Dialog 0x102 DlgProc

## Summary

`FUN_006AE3F0` is the `DLGPROC` callback for dialog 0x102 (offline Skirmish setup). It is registered via `CreateDialogIndirectParamA` called from the launcher `FUN_006AE2C0` — Ghidra shows no direct callers because the Windows message loop dispatches it indirectly. The function routes three observable message types: message 0x497 (custom init, dispatches to `FUN_006AE6E0`), WM_PAINT (0x0F, draws start-position markers on the mini-map), and WM_COMMAND (0x111, dispatches cell-change notifications to `FUN_006ACEE0`). Message 0x4E9 (WM_NOTIFY/tooltip hover) dispatches per-cell tooltip text using a two-level approach: AI-type combos get string-table text directly; country/color/start-pos cell tooltips are dispatched via per-cell helpers (FUN_004e3830, FUN_004e4170, etc.). The shared dialog handler `FUN_00622B50` is always called first and handles WM_INITDIALOG (0x110), WM_DESTROY (0x02), background brush (WM_CTLCOLOR 0x132..0x138), WM_DRAWITEM (0x2B), WM_MOUSEMOVE (0x84/tooltip hover), and WM_SETFONT.

## Active in YR

**Yes.** `FUN_006AE3F0` address is referenced at 0x006AE31C in `FUN_006AE2C0` as `[DATA]` (confirmed via `get_xrefs_to 0x006AE3F0`), and `FUN_006AE2C0` passes it as `param_2` (a `DLGPROC`) to `FUN_00622650` which calls `CreateDialogIndirectParamA`. `FUN_006AE2C0` is the standard offline Skirmish launcher, reachable from the YR main menu. No TS-only guard present in either function.

## Decompilation excerpt (verified via `decompile_function 0x006AE3F0`)

```c
int FUN_006ae3f0(HWND param_1, uint param_2, uint param_3, undefined4 *param_4)
{
    // Always check shared dialog handler first
    int iVar2 = FUN_00622b50(param_3, param_4);  // shared dialog handler
    if (iVar2 != 0) return iVar2;

    if (param_2 < 0x498) {
        if (param_2 == 0x497) {
            // Custom init message: populate all row cells
            return FUN_006ae6e0(param_4);
        }
        if (param_2 == 0x0F) {
            // WM_PAINT: draw start-position markers on mini-map (if map loaded)
            if (DAT_00ac1154 != 0) {
                GetDlgItem(param_1, 0x468);  // mini-map control
                cVar1 = FUN_006067a0();      // check if rendering busy
                if (cVar1 == '\0') {
                    DrawStartPositions(param_1);  // paint start markers
                }
            }
            ValidateRect(param_1, NULL);
            return 0;
        }
        if (param_2 == 0x111) {
            // WM_COMMAND: route cell-change notifications
            FUN_006acee0(param_4, param_3 >> 0x10);  // dispatch with control ID
            return 1;
        }
    }
    else if (param_2 == 0x4e9) {
        // WM_NOTIFY (tooltip hover) — per-cell tooltip dispatch
        FUN_007b6880(0);  // clear tooltip first
        if (((HWND)*param_4 != NULL) && (param_4[1] != -1)) {
            iVar2 = GetDlgCtrlID((HWND)*param_4);
            if (AI_type_combo(iVar2)) {
                // AI-type combos (0x50B, 0x50E, 0x516, 0x51A, 0x51B, 0x51C, 0x51D):
                // get hovered item-data, map to string table entry
                wParam = param_4[1];  // hovered item index
                if (wParam == 0xFFFFFFFF) {
                    wParam = SendDlgItemMessageA(param_1, iVar2, 0x147, 0, 0); // CB_GETCURSEL
                }
                LVar4 = SendDlgItemMessageA(param_1, iVar2, 0x150, wParam, 0); // CB_GETITEMDATA
                if (LVar4 == -1) { FUN_007b6880(StringTable(0x87)); return 1; }
                if (LVar4 ==  2) { FUN_007b6880(StringTable(0x89)); return 1; }
                if (LVar4 ==  1) { FUN_007b6880(StringTable(0x8b)); return 1; }
                if (LVar4 ==  0) { FUN_007b6880(StringTable(0x8d)); return 1; }
            }
            else {
                // Country / color / start-pos / team cells: delegate to per-cell helpers
                iVar2 = FUN_004e3830();      // country combo hit-test
                if (iVar2 == -1) {
                    iVar2 = FUN_004e4230();  // color combo hit-test
                    if (iVar2 != -1) {
                        FUN_004e4e20(param_4[1]);
                        FUN_007b6880(FUN_004e42a0()); return 1;
                    }
                    iVar2 = FUN_004e4ec0();  // start-pos combo hit-test
                    if (iVar2 != -1) {
                        FUN_004e5900(param_4[1]);
                        FUN_007b6880(FUN_004e4f30()); return 1;
                    }
                } else {
                    iVar2 = FUN_004e4170(param_4[1]); // country tooltip content
                    if (iVar2 != -1) {
                        FUN_007b6880(FUN_004e38a0()); return 1;
                    }
                }
            }
        }
        return 1;
    }
    return 0;
}
```

## Behavioral analysis

### Message routing table

| Message value | Name | Handler | Observable output |
|---------------|------|---------|-------------------|
| 0x497 | Custom dialog init | `FUN_006AE6E0` | Populates all 8 row cells from session data |
| 0x0F | WM_PAINT | Inline + `DrawStartPositions` | Draws start-pos markers on mini-map (control 0x468) if map loaded (`DAT_00AC1154 != 0`) |
| 0x111 | WM_COMMAND | `FUN_006ACEE0` | Routes cell-change CBN_SELCHANGE and button clicks (Start 0x617, Back 0x5C0) |
| 0x4E9 | WM_NOTIFY (tooltip) | Per-cell helpers | Sets tooltip text for hovered cell |
| Any | — | `FUN_00622B50` (first) | Shared frame handler: WM_INITDIALOG, WM_DESTROY, WM_MOUSEMOVE (0x84), WM_CTLCOLOR, WM_DRAWITEM, WM_SETFONT |

### Shared handler first-pass (FUN_00622B50)

`FUN_00622B50` (verified via `decompile_function 0x00622B50`) handles the generic dialog frame messages: WM_INITDIALOG (0x110), WM_DESTROY (0x02), WM_MOUSEMOVE/tooltip setup (0x84), WM_CTLCOLORBTN..WM_CTLCOLORSTATIC (0x132-0x138 → returns stock brush `GetStockObject(4)`), WM_DRAWITEM (0x2B → `FUN_006213A0`), and message 0x4EC (EnumChildWindows cleanup). When it returns non-zero, `FUN_006AE3F0` returns that value immediately without further message processing.

The WM_MOUSEMOVE (0x84) path inside `FUN_00622B50` does the tooltip position calculation and sends `WM_NOTIFY(0x4E9)` back to the dialog to fill in tooltip text, which then re-enters `FUN_006AE3F0`.

### WM_PAINT path

The start-positions paint only fires when `DAT_00AC1154 != 0` (random map resource loaded). The mini-map control is at dialog item ID 0x468. `FUN_006067A0` is a "rendering busy" check — if rendering is in progress, skip the draw. `DrawStartPositions` at 0x00640710 is called to paint the numbered start-position indicators.

### WM_COMMAND path

`param_3 >> 0x10` extracts the notification code from the wParam high word. This is passed as the second argument to `FUN_006ACEE0` alongside `param_4` (lParam = control HWND). The dispatcher in `FUN_006ACEE0` handles CBN_SELCHANGE for all cell types and BN_CLICKED for Start/Back buttons.

### WM_NOTIFY tooltip dispatch — AI-type combos

For the 7 AI-type combo controls (IDs 0x50B, 0x50E, 0x516, 0x51A, 0x51B, 0x51C, 0x51D), the tooltip text is determined by hovered item's `CB_GETITEMDATA` value:

| Item-data | String table index | Tooltip text |
|-----------|-------------------|--------------|
| -1 | 0x87 | "Closed" tooltip |
| 2  | 0x89 | "Computer" (Easy) tooltip |
| 1  | 0x8B | "Computer" (Medium) tooltip |
| 0  | 0x8D | "Human" tooltip |

The hovered item index comes from `param_4[1]`; if -1, `CB_GETCURSEL` (0x147) is used as fallback.

### WM_NOTIFY tooltip dispatch — other cells

Country, color, start-pos, and team combos use a hit-test chain:
1. `FUN_004e3830` — country combo hit-test (returns slot index or -1)
2. `FUN_004e4230` — color combo hit-test (returns slot index or -1)
3. `FUN_004e4ec0` — start-pos combo hit-test (returns slot index or -1)

If a hit is found, dedicated helpers provide the tooltip string:
- Country: `FUN_004e4170(param_4[1])` → `FUN_004e38a0()` provides string
- Color: `FUN_004e4e20(param_4[1])` → `FUN_004e42a0()` provides string
- Start-pos: `FUN_004e5900(param_4[1])` → `FUN_004e4f30()` provides string

### DlgProc registration (no direct Ghidra callers — by design)

`FUN_006AE3F0` has no direct Ghidra callers because it is passed as a `DLGPROC` function pointer at 0x006AE31C inside `FUN_006AE2C0` (confirmed via `get_xrefs_to 0x006AE3F0`). `FUN_006AE2C0` passes it to `FUN_00622650(0, FUN_006AE3F0, ...)` which calls `CreateDialogIndirectParamA(hInstance, lpTemplate, g_hWnd, param_2, lParam)`. The OS dispatches it indirectly via the message loop.

## Struct field accesses

| Pointer | Offset | Unit | Usage | Frame |
|---------|--------|------|-------|-------|
| `param_4` | `[0]` (HWND) | — | Control that triggered WM_NOTIFY hover | Win32 NMHDR |
| `param_4` | `[1]` (int) | — | Hovered item index (-1 = current selection) | Win32 NMHDR |
| `FUN_00622B50` internal | `param_4[0x30]` | byte | "needs redraw" flag on dialog object | internal dialog struct |
| `FUN_00622B50` internal | `param_4[0xBE]` | byte | "dirty" flag triggering FUN_006071E0 | internal dialog struct |
| `FUN_00622B50` internal | `param_4[0x6C]` | dword | Dialog timestamp/sequence field | internal dialog struct |

## Globals referenced

| Global | Address | Role in this function |
|--------|---------|----------------------|
| `DAT_00AC1154` | 0x00AC1154 | Random-map resource pointer — gates WM_PAINT start-pos draw |
| `DAT_00A8ED8C` | 0x00A8ED8C | Dialog open-count (incremented WM_INITDIALOG, decremented WM_DESTROY inside `FUN_00622B50`) |
| `g_hWnd` | named | Main window handle (used by `FUN_00622650`) |

## Callers

None in Ghidra (DlgProc registered via `CreateDialogIndirectParamA` in `FUN_006AE2C0`). Registration confirmed at 0x006AE31C via `get_xrefs_to 0x006AE3F0`.

## Callees (summary)

| Address | Name | Role in this function |
|---------|------|-----------------------|
| 0x00622B50 | FUN_00622b50 | Shared dialog frame handler (always first) |
| 0x006AE6E0 | FUN_006ae6e0 | Custom init handler (msg 0x497) |
| 0x006ACEE0 | FUN_006acee0 | WM_COMMAND dispatcher |
| 0x00640710 | DrawStartPositions | Draws start-pos markers on mini-map |
| 0x006067A0 | FUN_006067a0 | Rendering-busy check (gates paint) |
| 0x007B6880 | FUN_007b6880 | Set tooltip text |
| 0x004E3830 | FUN_004e3830 | Country combo hit-test |
| 0x004E38A0 | FUN_004e38a0 | Country tooltip string provider |
| 0x004E4170 | FUN_004e4170 | Country tooltip content resolver |
| 0x004E4230 | FUN_004e4230 | Color combo hit-test |
| 0x004E42A0 | FUN_004e42a0 | Color tooltip string provider |
| 0x004E4E20 | FUN_004e4e20 | Color tooltip handler |
| 0x004E4EC0 | FUN_004e4ec0 | Start-pos combo hit-test |
| 0x004E4F30 | FUN_004e4f30 | Start-pos tooltip string provider |
| 0x004E5900 | FUN_004e5900 | Start-pos tooltip handler |

## Out-of-scope refs

- `FUN_006213A0` @ 0x006213A0 — WM_DRAWITEM handler for dialog controls (shared, out of cell scope)
- `FUN_006071E0` @ 0x006071E0 — dirty-flag callback triggered in WM_PAINT path inside shared handler
- `DrawStartPositions` @ 0x00640710 — mini-map start-pos rendering; out of cell-combo scope
- `FUN_006040B0` @ 0x006040B0 — tooltip dispatcher referenced inside `FUN_00622B50` WM_MOUSEMOVE path; in scope as task #51

## TS-filter

Dialog 0x102 is YR-introduced. All message handlers trace to the offline Skirmish flow. No TS-only gates found in the decompilation. **TS-legacy score: 0.0.**

## Unverified claims (YELLOW)

- String table indices 0x87, 0x89, 0x8B, 0x8D for AI-type tooltip items — confirmed as `StringTable__LoadString` calls with those indices in the decompile, but string content not verified by reading the string table directly.
- `FUN_00622B50` field accesses (`param_4[0x30]`, `param_4[0xBE]`, `param_4[0x6C]`) — observed in the shared handler decompile but semantic names not independently confirmed.
- `DAT_00A8ED8C` dialog open-count semantics — inferred from the WM_INITDIALOG increment and WM_DESTROY decrement pattern inside `FUN_00622B50`; not independently verified.
