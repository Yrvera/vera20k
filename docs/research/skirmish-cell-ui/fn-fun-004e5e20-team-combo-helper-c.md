# FUN_004E5E20 - Per-Slot Team Sentinel Dispatcher

## Summary

Per-slot helper called during dialog init. Checks whether a specific slot (param_2
row index) is in spectator/observer mode with an absent AND closed slot. If so,
passes sentinel value 4 to FUN_004E5ED0 (team sentinel writer, task 48); otherwise
passes -1 (0xFFFFFFFF). param_1 is also passed to FUN_004E5ED0 but not used by
this function directly.

Note: the dispatch condition here requires BOTH inner conditions joined with AND
(slot == absent AND slot+0x6B == -1), unlike FUN_004E5D60 which uses OR.

## Address

0x004E5E20 (verified via decompile_function 0x004E5E20)

## Active in YR

Yes. In-scope caller is FUN_006AE6E0 (0x006AE6E0, dialog init, task 1).
(Confirmed via get_function_callers 0x004E5E20)

## Signature / Parameters

void __fastcall FUN_004e5e20(undefined4 param_1, int param_2)
  param_1 = passed through to FUN_004E5ED0 (likely dialog HWND)
  param_2 = 0-based row/slot index

(verified via decompile_function 0x004E5E20)

## Behavioral Analysis

```c
uVar1 = 0xFFFFFFFF;   // default: -1 (normal/no-sentinel)
if ((g_GameMode == 3 || g_GameMode == 4) &&
    (&DAT_00A8DA90)[param_2] == DAT_00AC11B4 &&
    *(int *)((&DAT_00A8DA90)[param_2] + 0x6B) == -1)
{
    uVar1 = 4;   // sentinel: spectator + absent + closed
}
FUN_004e5ed0(uVar1);
```

(verified via decompile_function 0x004E5E20)

### Dispatch condition vs. FUN_004E5D60

FUN_004E5D60 (task 46) uses OR between the two inner slot conditions:
  slot == absent  OR  slot+0x6B == -1

FUN_004E5E20 uses AND:
  slot == absent  AND  slot+0x6B == -1

This is a stricter condition. In offline skirmish the outer g_GameMode gate
is false regardless, so uVar1 = -1 always in practice.

### Sentinel value 4

The value 4 passed to FUN_004E5ED0 when the full condition fires. Likely a
team index meaning no-team or observer in the team combo. FUN_004E5ED0 (task 48)
decodes the meaning.

## Globals Accessed

  g_GameMode   (symbolic)   - Mode gate: 3/4 = spectator/observer
  DAT_00A8DA90 (0x00A8DA90) - Player-slot pointer array
  DAT_00AC11B4 (0x00AC11B4) - Null/absent slot sentinel

## Callees

Confirmed via get_function_callees 0x004E5E20:
  FUN_004E5ED0 (0x004E5ED0) - Team sentinel writer (task 48)

## Callers (in scope)

  FUN_006AE6E0 (0x006AE6E0) - dialog init (task 1)

(Confirmed via get_function_callers 0x004E5E20)

## Out-of-scope refs

  FUN_004E5ED0 (0x004E5ED0) - team sentinel writer; decoded in task 48

## Unverified (YELLOW)

  Sentinel value 4: meaning inferred as no-team/observer team index from
  context; not verified by reading FUN_004E5ED0 in this task.
  param_1 role: Ghidra types it as undefined4, passed through to FUN_004E5ED0;
  likely dialog HWND but not confirmed from this function alone.
