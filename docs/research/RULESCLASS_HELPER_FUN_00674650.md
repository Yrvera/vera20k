---
title: RulesClass helper FUN_00674650 — Advanced Command Bar button-list reader
source_addr: 0x00674650
owner_report: RULESCLASS_GHIDRA_REPORT.md §5 (Master orchestrators)
yr_active_in_stock_game: NO (section not present in any shipping *.ini)
writes_to_rulesclass: NO
verified_from: gamemd.exe live decompilation (Ghidra MCP, 2026-04-24)
---

# RulesClass helper `FUN_00674650` — Advanced Command Bar button-list reader

## Summary

Not what the plan assumed. This is **not** an AI-triggers / ScriptTypes /
TeamTypes / TaskForces reader. It is a very small helper that populates a
global UI button-layout array (`DAT_00B0CB78` + friends) from either
`[AdvancedCommandBar]` or `[MultiplayerAdvancedCommandBar]` depending on the
active game mode. It does **not** write to any RulesClass field.

**Treat as an adjacent helper.** It is called from the RulesClass dispatcher
(`FUN_00668BF0` at step 33) purely because "load the command bar buttons" is
part of the same bulk-INI load pass that reads rules. Future Rust port
should implement this as a UI-subsystem concern, not a `Rules` struct field.

## Signature

```
undefined4 __?__ FUN_00674650(undefined4 param_1, char param_2);
```

- `param_1` — pushed last (first arg on stack). Unused in the function body.
  Ghidra's signature lists it, but the decomp never references `param_1`.
  Likely a stale CCINIClass pointer the caller happened to be holding in
  ESI; harmless.
- `param_2` — pushed first (second arg on stack). The section selector:
  - `0` → read `[AdvancedCommandBar]`
  - non-zero → read `[MultiplayerAdvancedCommandBar]`

Return: `1` on a successful read, `0` if the section was missing.

## Dispatcher call sites (`FUN_00668BF0`)

Two call sites, exactly one of which fires per dispatcher run, gated on the
global `g_GameMode` (held in EAX at the call site):

```
00668f6f  TEST  EAX, EAX            ; EAX = g_GameMode
00668f71  JZ    0x00668f90          ; GameMode == 0 → single-player path
00668f73  CMP   EAX, 0x5
00668f76  JZ    0x00668f90          ; GameMode == 5 → single-player path
00668f78  MOV   EAX, 0x1            ; else EAX = 1
00668f7d  MOV   ECX, EDI
00668f7f  PUSH  EAX                 ; param_2 = 1 (multiplayer section)
00668f80  PUSH  ESI                 ; param_1 = ESI (unused)
00668f81  CALL  0x00674650          ; read [MultiplayerAdvancedCommandBar]
                                    ; dispatcher returns
00668f90  XOR   EAX, EAX            ; EAX = 0
00668f92  MOV   ECX, EDI
00668f94  PUSH  EAX                 ; param_2 = 0 (single-player section)
00668f95  PUSH  ESI                 ; param_1 = ESI (unused)
00668f96  CALL  0x00674650          ; read [AdvancedCommandBar]
```

So gamemodes **0 (Campaign)** and **5** take the single-player
`[AdvancedCommandBar]`; every other gamemode (skirmish/LAN/Internet/etc.)
takes `[MultiplayerAdvancedCommandBar]`. This is the only thing the `char`
arg controls.

## Function body (full decomp)

```c
undefined4 FUN_00674650(undefined4 param_1, char param_2)
{
    int   iVar1;
    char* puVar2;
    char  local_200[512];

    // Section selector
    puVar2 = PTR_s_MultiplayerAdvancedCommandBar_007f0cec;   // "MultiplayerAdvancedCommandBar"
    if (param_2 == '\0') {
        puVar2 = PTR_s_AdvancedCommandBar_007f0ce8;          // "AdvancedCommandBar"
    }

    // 1) Bail if the section doesn't exist in the INI
    if (CCINIClass__Find_Section(puVar2) == 0) {
        return 0;
    }

    // 2) Zero out the 25-slot button array
    iVar1 = 0;
    do {
        FUN_006cfd20(iVar1, 0);          // &DAT_00b0cb78)[iVar1] = 0
        iVar1 = iVar1 + 1;
    } while (iVar1 < 0x19);              // 25 slots total

    // 3) Read the ButtonList key (comma-separated list of command names)
    iVar1 = CCINIClass__ReadString(puVar2,
                                   PTR_s_ButtonList_007f0cf0,  // "ButtonList"
                                   PTR_DAT_007f0cf4,           // default (empty ptr)
                                   local_200,
                                   0x200);                     // 512-byte buffer
    if (iVar1 != 0) {
        // 4) Tokenise on comma, look up each command, store at next slot
        iVar1 = CRT__strtok(local_200, &DAT_00817f70);          // "," separator
        while (iVar1 != 0) {
            iVar1 = FUN_006cfcc0(token);                       // command-name → index
            if (iVar1 != DAT_008427cc) {                       // 0xFFFFFFFF = not-a-command
                FUN_006cfd20(?, iVar1);                        // append to button array
            }
            iVar1 = CRT__strtok(0, &DAT_00817f70);
        }
        // 5) Recompute the command bar's on-screen layout
        FUN_006cfdb0(?);
    }
    return 1;
}
```

(Ghidra's listing drops some of the implicit first-args on the `FUN_006cfd20`
calls; see the raw disassembly for full arg tracking if needed.)

## Helpers invoked

| Addr | Role |
|---|---|
| `0x00526810` | `CCINIClass::Find_Section(name)` — returns section pointer or 0 |
| `0x00528A10` | `CCINIClass::Read_String(section, key, default, buf, len)` |
| `0x007C9CC2` | CRT `strtok` |
| `0x006CFCC0` | **command-name → command-ID lookup** — linear-compares `param_1` against the 11-entry string-pointer table at `0x008427D0..0x008427FC`; returns the 0-based index on match, or `DAT_008427CC` (`0xFFFFFFFF`) on miss. Table size inferred from the loop's `if (0x8427fb < ppuVar6) return SENTINEL;` bound. |
| `0x006CFD20` | **button-slot setter** — `(&DAT_00b0cb78)[slot] = command_id;` |
| `0x006CFDB0` | **command-bar layout recalc** — updates `DAT_00b0cb54`, `_DAT_00b0cc2c` from the new button count; clamps to the screen-edge constraint at `DAT_00b0fc60[2] + *DAT_00b0fc60`. Runs once at the tail. |

## Target globals (none in RulesClass)

- `DAT_00B0CB78` — 25-slot `int[]` of command IDs (the active command-bar layout).
- `DAT_00B0CB54` — derived button count, used by the layout function.
- `_DAT_00B0CC2C` — cached screen X-origin of the command bar.
- Command-name lookup table: pointer array at `0x008427D0`, 11 entries,
  points into the string pool around `0x00842800+` (names not enumerated here
  — out of scope for RulesClass).

## YR-active status — **dormant in stock YR**

- `ini/rulesmd.ini`, `ini/rules.ini`, and all other shipping `ini/*.ini`
  files in the repo contain **no** `[AdvancedCommandBar]` or
  `[MultiplayerAdvancedCommandBar]` section (grep-confirmed).
- `CCINIClass::Find_Section` returns 0 on a missing section, so the function
  exits early (`return 0`) without touching any global.
- The button array at `DAT_00B0CB78` stays whatever the prior game state
  left it (initialised elsewhere during engine startup).

Net effect on a retail YR skirmish: **this function is a no-op.** It is
present for mod compatibility and as a TS-legacy remnant (the TS-era
"advanced command bar" was an optional side-panel extension). Implementing
it in the Rust port is **not** required for parity.

If a future mod needs to configure the command bar via INI, the Rust port
can implement it as a separate UI-layer INI reader — no RulesClass field is
implicated.

## Confidence

HIGH — function is small (~30 lines decompiled), all five callees verified,
every string pointer resolved to its ASCII literal, call-site gating read
directly from x86 disassembly at the dispatcher. The one unknown is the
concrete semantics of the 11 command-name strings in the lookup table —
deliberately not enumerated because it lies outside RulesClass.

## Cross-refs

- RulesClass dispatcher (`FUN_00668BF0`) — calls this at step 33.
- `ScenarioClass::Full_Init` (`0x00686B20`) — also calls this directly at
  `0x0068768F`, confirming it is invoked whenever a game starts, not just
  via the Rules path.
- `InitSideMixFiles` (`0x00534FA0`) — third caller at `0x0053533D`, likely
  the "campaign intro" early-load path.
