# WWOnline & Network/LAN Button Dispatch — Main_Game Cases 2 and 3

**Date:** 2026-05-19  
**Scope:** Document the dispatch entry for cases 2 (WWOnline) and 3 (Network/LAN) in
`Main_Game @ 0x0052D9A0`. Covers: dialog/proc creation, button control IDs, CSF keys,
TS-legacy gating, and return-code mapping.  
**Read-only:** No Ghidra annotations modified.

Cross-ref docs used (not re-investigated):
- `MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md` — case-index overview
- `SINGLE_PLAYER_SUBMENU_DIALOG_CASE1_GHIDRA_REPORT.md` — case 1 structural template
- `MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md` — case 4 structural template
- `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md` — case 6 structural template

---

## Verification calls (all read-only)

| Claim | Ghidra call used |
|---|---|
| Main_Game switch body | `decompile_function 0x0052D9A0` |
| Main menu dialog proc | `decompile_function 0x00531F60` (confirmed Ghidra label `MainMenuDialog0xE2_Proc_00531F60`) |
| `FUN_0053F1F0` setter stub | `decompile_function 0x0053F1F0` |
| WOL orchestrator | `decompile_function 0x0077B2A0` |
| WOL login dialog construction | `decompile_function 0x00789B60` (labelled `SimpleWonlineDialogControl__Constructor` by Ghidra) |
| Network/LAN dialog runner | `decompile_function 0x005DC350` |
| Network/LAN dialog proc address | traced from `FUN_00775700(g_hWnd, &LAB_005DDBD0, 0)` in `FUN_005DC350` |
| `FUN_00622650` wrapper | `decompile_function 0x00622650` |
| `FUN_00775700` wrapper | `decompile_function 0x00775700` |
| CSF key strings | `search_strings "STT:MainButton"`, `search_strings "STT:WOLLogin"`, `search_strings "STT:WOL"` |

---

## 1. How Cases 2 and 3 Are Triggered

`FUN_00531CC0 @ 0x00531CC0` runs the main menu dialog (dialog template ID `0xE2`
via `CreateDialogIndirectParamA`, called through `FUN_00622650(0)`). Its dialog proc
is `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`.

The dialog proc maps `WM_COMMAND (0x111)` messages to numeric return codes written
into the GWLP_USERDATA slot. Confirmed button-ID-to-return-code mapping:

| Button control ID (hex) | Return code | Case | CSF key (from `search_strings`) |
|---|---|---|---|
| `0x684` (1668) | 2 | WWOnline | `STT:MainButtonWWOnline` (@ `0x0083576C`) |
| `0x578` (1400) | 3 | Network  | `STT:MainButtonNetwork` (@ `0x00835754`) |
| `0x683` (1667) | 1 | Single Player | `STT:MainButtonSinglePlayer` (@ `0x00835784`) |
| `0x686` (1670) | 4 | Movies | `STT:MainButtonMovies` (@ `0x0083573C`) |
| `0x55C` (1372) | 5 | Options | `STT:MainButtonOptions` (@ `0x00835724`) |
| `0x3EE` (1006) | 6 | Exit | `STT:MainButtonExitGamemd` (@ `0x00835708`) |

Verified via `decompile_function 0x00531F60` — the proc's `if/else if` chain on
`param_3 & 0xffff` explicitly encodes all six mappings.

---

## 2. Case 2 — WWOnline Button (return code 2)

### Switch body (from `decompile_function 0x0052D9A0`)

```c
case 2:
    iVar11 = 0x10;
    g_GameMode = 4;
    FUN_0053F1F0();     // DAT_00828140 = 0  (called with no explicit arg; param is 0)
    break;
```

`FUN_0053F1F0 @ 0x0053F1F0` is a 6-byte fastcall stub: `DAT_00828140 = param_1; return;`
(verified via `decompile_function 0x0053F1F0`). Called here with implicit param 0.

### What dialog is actually opened

Case 2 does **not** open a dialog directly. It sets `g_GameMode = 4` and `iVar11 = 0x10`,
then `break`s. The loop iterates; because `g_GameMode == 4`, the pre-switch routing
hardcodes `iVar11 = 0x10` again. Case `0x10` (`g_GameMode == 4` branch) calls:

```c
FUN_0077B2A0()   // WOL orchestrator @ 0x0077B2A0
```

`FUN_0077B2A0` (verified via `decompile_function 0x0077B2A0`) is the WOL session
manager. It runs an internal state machine on `DAT_00a8b244` (initial value 0).

When `DAT_00a8b244 == 0`, it calls `SimpleWonlineDialogControl__Constructor @ 0x00789B60`
(Ghidra label confirmed via `get_function_callers "SimpleWonlineDialogControl__Constructor"`
+ `decompile_function 0x00789B60`).

### WOL Entry Dialog — `SimpleWonlineDialogControl__Constructor @ 0x00789B60`

This function creates the WOL **login dialog** via:

```c
iVar8 = (**(code **)(*DAT_00ac116c + 0x1c))(DAT_00ac116c, aiStack_160);
```

`DAT_00ac116c` is the `IWOLAppSite` COM object pointer. Vtable slot `+0x1c` is
`IWOLAppSite::CreateDialog`. This is **not** `CreateDialogIndirectParamA`; it is a
WOL SDK COM call that creates an internal WOL dialog by type ID.

The dialog type is encoded in `aiStack_160`:

```c
aiStack_160[0] = 0x29;   // dialog type = 41 decimal (WOL login screen)
aiStack_160[1] = 1;
aiStack_160[2] = DAT_00b77dc4;   // sub-mode: 2 = lobby, 6 = default initial login
```

`DAT_00b77dc4` defaults to 6 on first entry (set just before the COM call when
`DAT_00b77dc4 < 2`). Value 6 routes to the standard WOL login screen.

#### Immediate controls on the WOL login entry screen

Verified via CSF key scan (`search_strings "STT:WOLLogin"`) — login dialog controls:

| CSF key | Role |
|---|---|
| `STT:WOLLoginNickname` (@ `0x008342C0`) | Nickname text field label |
| `STT:WOLLoginPassword` (@ `0x008342A8`) | Password text field label |
| `STT:WOLLoginLogin` (@ `0x0083427C`) | Login / connect button |
| `STT:WOLLoginRemember` (@ `0x00834290`) | Remember password checkbox |
| `STT:WOLLoginBack` (@ `0x00834240`) | Back / cancel button |
| `STT:WOLLoginForget` (@ `0x00834254`) | Forget saved credentials button |
| `STT:WOLNewAccount` (@ `0x00834268`) | New account creation button |
| `STT:WOLManageAccount` (@ `0x00834228`) | Manage account button |

The WOL login dialog also exposes a `GetDlgItem(hWnd, 0x540)` focus target
(control ID `0x540` = 1344), confirmed at three locations in the decompiled login
construction code — this is the Nickname edit control.

#### Dialog proc

The WOL login screen has no direct Ghidra-recoverable proc address because it is
created inside the WOL SDK COM object (`IWOLAppSite::CreateDialog`), not via
`CreateDialogIndirectParamA`. The dialog window messages are handled internally by
the WOL SDK; `FUN_0077B2A0` pumps an outer loop polling `DAT_00a8b244` state
transitions.

The outer orchestrator `FUN_0077B2A0` itself serves as the effective "proc" from
the game's perspective — it runs `do { … } while (true)` and returns:

| Return value | Meaning |
|---|---|
| `1` | Game launched (proceed to multiplayer session) |
| `0` | User backed out → `g_GameMode = 0`, return to main menu |
| `-1` (0xFFFFFFFF) | Process exit requested → `return 0` from `Main_Game` |
| `2` | Routes case `0xb` (Tiberian Dawn campaign; TS legacy — see §4) |
| `-2` / `0` (state 6) | Backend disconnect / error → `g_GameMode = 0`, main menu |

---

## 3. Case 3 — Network/LAN Button (return code 3)

### Switch body (from `decompile_function 0x0052D9A0`)

```c
case 3:
    iVar11 = 0x10;
    g_GameMode = 3;
    FUN_0053F1F0();     // DAT_00828140 = 0
    break;
```

Same setter stub, same `iVar11 = 0x10` routing. Case `0x10` (`g_GameMode == 3` branch)
calls:

```c
FUN_005DB680()   // @ 0x005DB680 — initialise IPX/network layer
FUN_005DC350()   // @ 0x005DC350 — LAN lobby dialog runner (if FUN_005DB680 != 0)
```

`FUN_005DB680` (verified via `decompile_function 0x005DB680`) is a thin init wrapper:
calls `FUN_00540A80()` to check network availability; returns 0 (no network) or 1
(network ready). If it returns 0 the LAN path bails and returns to main menu.

### LAN Entry Dialog — `FUN_005DC350 @ 0x005DC350`

`FUN_005DC350` (verified via `decompile_function 0x005DC350`) opens the LAN lobby
dialog via:

```c
pHVar3 = FUN_00775700(g_hWnd, &LAB_005DDBD0, 0);
FUN_00622820();
ShowWindow(pHVar3, 1);
```

`FUN_00775700 @ 0x00775700` (verified via `decompile_function 0x00775700`) is a
`CreateDialogIndirectParamA` wrapper. Signature:

```c
HWND FUN_00775700(HINSTANCE param_1, undefined4 param_2/*dialog-type-id*/,
                  HWND param_3/*parent*/, DLGPROC param_4, int param_5/*show-immediately*/);
```

Here called as `FUN_00775700(g_hWnd, &LAB_005DDBD0, 0)` — where the third positional
argument is the DLGPROC. The internal `CreateDialogIndirectParamA` receives
`param_4 = &LAB_005DDBD0 @ 0x005DDBD0`.

**Dialog proc: `LAB_005DDBD0 @ 0x005DDBD0`** — Ghidra reports no function boundary at
this address (it falls within the body of `FUN_005DD8A0`, which ends at `0x005DDBC3`,
and `0x005DDBD0` is a jump target label in the next function gap). This indicates the
dialog proc entry point is at an unlabelled boundary; the function Ghidra did not
auto-detect. It acts as a standard Windows dialog procedure for the LAN lobby window.

`FUN_005DC350` drives the LAN dialog with an internal event loop, polling
`DAT_00ac025c` for control notifications.

#### LAN entry dialog control IDs (from `decompile_function 0x005DC350`)

| Control ID (hex) | Role |
|---|---|
| `0x5C0` / `0x2` (WM_CLOSE/quit) | Cancel / Exit LAN lobby |
| `0xBB` (187) | Back / close without game |
| `0xBC` (188) | Host game button |
| `0xBD` (189) | Join game button |
| `0x541` (1345) | Secondary action button (enabled/disabled by game-found state) |
| `0x588` (1416) | "Find game" / join network game trigger button |
| `0x6CC` (1740) | Player-name entry commit (from LAN join name field) |
| `0x6CD` (1741) | Join game confirmed |
| `0x6CE` (1742) | Rejoin / reconnect action |

#### CSF keys used in the LAN dialog runner (`decompile_function 0x005DC350`)

The runner pulls strings from `D:\ra2mdpost\netdlg2.cpp` source table
(`s_D__ra2mdpost_netdlg2_cpp_00831288`). Numeric string IDs referenced:

| StringTable ID (decimal) | Usage context |
|---|---|
| `0x57E` (1406) | "Timing out LAN join request" message |
| `0x683` (1667) | No nickname entered error |
| `0x68D` (1677) | Duplicate name on LAN error |
| `0x748` (1864) | Need at least 2 players error |
| `0x754` (1876) | Player not ready warning |
| `0x767` (1895) | Too many players error |
| `0x79A` (1946) | Missing scenario / map error |
| `0x7AC` (1964) | AI player configuration error |

The dialog proc label `LAB_005DDBD0` (within `FUN_005DC350`'s broader function
region) is the DLGPROC registered with Windows. Notification routing from the dialog
to `FUN_005DC350`'s main loop is via `DAT_00ac025c` (a global message-ID register
written by the dialog proc).

---

## 4. TS-Legacy Gating

### Case 2 — WWOnline

**YR-live in code, but operationally dead.** The code path is fully reachable in stock YR:
there is no `SpecialFlags` gate, no `OptionsClass` boolean that defaults off, and no
conditional skip. The WOL SDK is linked into gamemd.exe and the COM object
(`IWOLAppSite`) is initialised on the WOL entry path.

However, Westwood Online servers were **shut down in 2004**. The `IWOLAppSite::CreateDialog`
call will succeed, present the login screen, and then fail to connect to any server.
This is a service availability issue, not a code gate. From the engine's perspective
the code is live; from a player's perspective the path is a dead end.

No `SpecialFlags` or `OptionsClass` field gates this path — confirmed by absence of
any such check in `FUN_0077B2A0` or the pre-case-2 routing in `decompile_function 0x0052D9A0`.

`FUN_0077B2A0` return code `2` routes `iVar11 = 0xb`, which sets `g_GameMode = 5`
(Tiberian Dawn). Case `0xb` then falls through to case `0x10` with GameMode 5, and
`FUN_006AE2C0()` is called (TS campaign launcher — TS-legacy dead code in YR).
This sub-path is reachable only if the WOL backend returns `uVar4 == 2`,
which requires a live WOL session — unreachable in 2026 YR.

### Case 3 — Network/LAN

**Fully live in YR.** No SpecialFlags gate. The only precondition is `FUN_00540A80()`
returning non-zero (a network adapter/IPX driver check). IPX support was removed from
Windows Vista+; on Vista and later, `FUN_005DB680` will return 0 and the LAN path
will silently bail back to main menu without opening any dialog.

No TS-only sub-paths were found in `FUN_005DC350` that are gated off in standard YR.
The function was written for RA2/YR LAN IPX multiplayer; the entire function is active
in YR but the IPX transport layer it depends on is OS-disabled post-XP.

---

## 5. Return-Code Mapping Back to Main_Game Loop

### Case 2 path (WOL, g_GameMode=4)

`FUN_0077B2A0` returns into case `0x10`'s `g_GameMode == 4` branch. The `uVar4`
return value is dispatched:

| `FUN_0077B2A0` return | Effect in case `0x10` | Player-visible |
|---|---|---|
| `1` | `bVar1 = false; _DAT_00825c84 = 2` → loop does NOT go back to `LAB_0052dc40`; proceeds to scenario launch | WOL game starts |
| `0` or `0xFFFFFFFE` | `g_GameMode = 0; iVar11 = 0x12` + reload INTRO music | Return to main menu (0xE2 dialog) |
| `0xFFFFFFFF` | `FUN_00720EA0(1)` + `return 0` from `Main_Game` | Process exits |
| `2` | `g_GameMode = 0; iVar11 = 0xb` → case `0xb` sets `g_GameMode=5` | TS campaign (dead in 2026) |

### Case 3 path (LAN, g_GameMode=3)

`FUN_005DC350` returns 0 (cancel/fail) or 1 (game ready). Within case `0x10`:

| `FUN_005DC350` return | Effect | Player-visible |
|---|---|---|
| `0` (returned by `FUN_005DB680` or `FUN_005DC350`) | `g_GameMode = 0; iVar11 = 0x12` | Return to main menu |
| `1` (game joined/hosted) | `goto switchD_0052e356_caseD_1` — proceeds to scenario launch | LAN game starts |

For both cases the "return to main menu" path reaches `iVar11 = 0x12`, which on the
next iteration calls `FUN_00531CC0()` and displays dialog `0xE2` (the main menu with
the six buttons). There is no separate "back" dialog; the main menu is simply re-shown.

---

## Summary of Key Addresses

| Symbol | Address | Verified via |
|---|---|---|
| `Main_Game` | `0x0052D9A0` | `decompile_function` |
| `MainMenuDialog0xE2_Proc_00531F60` | `0x00531F60` | `get_function_by_address` + `decompile_function` |
| `FUN_0053F1F0` (DAT_00828140 setter) | `0x0053F1F0` | `decompile_function` |
| WOL orchestrator (`FUN_0077B2A0`) | `0x0077B2A0` | `decompile_function` |
| WOL login constructor (`SimpleWonlineDialogControl__Constructor`) | `0x00789B60` | Ghidra label + `decompile_function` |
| LAN init (`FUN_005DB680`) | `0x005DB680` | `decompile_function` |
| LAN lobby runner (`FUN_005DC350`) | `0x005DC350` | `decompile_function` |
| LAN dialog proc entry point | `0x005DDBD0` | traced from `FUN_00775700` call in `FUN_005DC350` |
| `CreateDialogIndirectParamA` LAN wrapper | `0x00775700` | `decompile_function` |
| `CreateDialogIndirectParamA` shell wrapper | `0x00622650` | `decompile_function` |
| Main-menu button `0x684` (WWOnline) | CSF `STT:MainButtonWWOnline` @ `0x0083576C` | `search_strings` + dialog proc decompile |
| Main-menu button `0x578` (Network) | CSF `STT:MainButtonNetwork` @ `0x00835754` | `search_strings` + dialog proc decompile |
