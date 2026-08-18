# FUN_004E5D60 - Team Combo Population Dispatcher (All-Rows Loop)

## Summary

Iterates all 8 player row slots and populates each row team combo by dispatching
to one of two helpers based on game mode and slot state. This is the team-combo
equivalent of FUN_004E4820 (color, task 31) and shares the identical spectator/
closed dispatch condition.

  Normal path (FUN_004E5B60): full team list population
  Sentinel path (FUN_004E5CB0): single sentinel entry (closed/spectator state)

Called from both dialog init and the WM_COMMAND dispatcher.

## Address

0x004E5D60 (verified via decompile_function 0x004E5D60)

## Active in YR

Yes. In-scope callers:
  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)
  FUN_006AE6E0 (0x006AE6E0) - dialog init (task 1)

Out-of-scope: CDFileClass__Constructor (mislabeled) at 0x005E3D10.
(Confirmed via get_function_callers 0x004E5D60)

## Signature / Parameters

void FUN_004e5d60(void)

No parameters. (verified via decompile_function 0x004E5D60)

## Behavioral Analysis

### Loop body

iVar1 = 0;
do {
    if ((g_GameMode == 3 || g_GameMode == 4) &&
        ((&DAT_00a8da90)[iVar1] == DAT_00ac11b4 ||
         *(int *)((&DAT_00a8da90)[iVar1] + 0x6b) == -1))
    {
        FUN_004e5cb0();   // sentinel: closed/spectator state
    }
    else {
        FUN_004e5b60();   // normal: full team list
    }
    iVar1++;
} while (iVar1 < 8);

(verified via decompile_function 0x004E5D60)

### Dispatch condition

Identical condition to FUN_004E4820 (color) and FUN_004E5480 (start-pos):
  - Outer gate: g_GameMode == 3 || g_GameMode == 4 (spectator/observer mode)
  - Inner gate: slot pointer == DAT_00AC11B4 (absent slot) OR
                slot[+0x6B] == -1 (slot closed/inactive)

In offline skirmish the outer gate is false; FUN_004E5B60 always runs.

## Globals Accessed

  g_GameMode   (symbolic)   - Mode gate: 3/4 = spectator/observer
  DAT_00A8DA90 (0x00A8DA90) - Player-slot pointer array, 8 entries
  DAT_00AC11B4 (0x00AC11B4) - Null/absent slot sentinel

## Callees

Confirmed via get_function_callees 0x004E5D60:
  FUN_004E5B60 (0x004E5B60) - Normal team combo population
  FUN_004E5CB0 (0x004E5CB0) - Team sentinel loader (closed/spectator)

## Callers (in scope)

  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)
  FUN_006AE6E0 (0x006AE6E0) - dialog init (task 1)

(Confirmed via get_function_callers 0x004E5D60)

## Structural Parallel

FUN_004E5D60 is the team-combo counterpart to:
  FUN_004E4820 (color, task 31) -- same loop structure, different helper pair
  FUN_004E4820 calls FUN_004E4770/FUN_004E45A0 (color helpers)
  FUN_004E5D60 calls FUN_004E5CB0/FUN_004E5B60 (team helpers)

## Out-of-scope refs

  FUN_004E5B60 (0x004E5B60) - normal team population; out of current scope
  FUN_004E5CB0 (0x004E5CB0) - team sentinel loader; out of current scope

## TS-filter

Primary callers are YR dialog functions. No TS-only gate. TS-legacy score: 0.0.

## Unverified (YELLOW)

  FUN_004E5CB0 as team sentinel loader: inferred from spectator/closed dispatch
  pattern mirroring FUN_004E4770 (color sentinel); not independently decompiled.
  FUN_004E5B60 as normal team population: inferred from else-branch dispatch
  mirroring FUN_004E45A0 (color population); not independently decompiled.
