# FUN_004E5940 - Per-Row Team Combo Control-ID Lookup

## Summary

Pure lookup function: maps a 0-based row index (0-7) to the Win32 dialog control
ID for the team combo in that row of dialog 0x102. Row 0 -> 0x76D, rows 1-7 ->
0x76E..0x774. All 8 IDs form a contiguous range. No side effects.

Called by all five anchor functions for dialog 0x102 plus the team-enable helper
(FUN_006ACD60, task 17) -- it is the canonical source of team combo control IDs
for the dialog.

## Address

0x004E5940 (verified via decompile_function 0x004E5940)

## Active in YR

Yes. In-scope callers include:
  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)
  FUN_006ADC20 (0x006ADC20) - row enable state machine (task 3)
  FUN_006ACD60 (0x006ACD60) - team-enable helper (task 17)
  FUN_006ADDF0 (0x006ADDF0) - row show/hide adjuster (task 14)
  FUN_006ADF00 (0x006ADF00) - reveal-AI-rows (task 15)
  FUN_006AE080 (0x006AE080) - hide-AI-rows (task 16)
  FUN_006AE6E0 (0x006AE6E0) - dialog init (task 1)
  FUN_006ACE80 (0x006ACE80) - team-enable helper b (in scope)

Out-of-scope callers: CDFileClass__Constructor (mislabeled), FUN_005DBF50,
FUN_005E9E00, FUN_005E9E70, FUN_005EA3E0, FUN_005EAE40, FUN_005EB7B0,
FUN_005EBA00, FUN_005EBC00, FUN_005EC9F0, FUN_005ED5A0, FUN_005EDDE0,
FUN_005EE3A0, FUN_005EE510, FUN_0077ED00.

(Confirmed via get_function_callers 0x004E5940)

## Signature / Parameters

int __fastcall FUN_004e5940(int param_1)
  param_1 = 0-based row index (0-7)
  returns: Win32 dialog control ID for the team combo in that row

No callees -- pure computation.
(Confirmed via get_function_callees 0x004E5940, which returned no callees.)

## Row-Index to Control-ID Mapping

(verified via decompile_function 0x004E5940)

  Row 0 -> 0x76D   (human player slot)
  Row 1 -> 0x76E
  Row 2 -> 0x76F
  Row 3 -> 0x770
  Row 4 -> 0x771
  Row 5 -> 0x772
  Row 6 -> 0x773
  Row 7 -> 0x774
  other -> 0x5FF  (out-of-range: formula (-(uint)(param_1 != 7) & 0xFFFFF88B) + 0x774;
                   for any input != 7: -(uint)(1) = 0xFFFFFFFF, & 0xFFFFF88B = 0xFFFFF88B,
                   + 0x774 = 0x100000000 + 0x5FF, truncated to 32 bits = 0x5FF)

The IDs form a contiguous range 0x76D..0x774 (8 entries, stride 1).
The final return is via the formula `(-(uint)(param_1 != 7) & 0xFFFFF88B) + 0x774`
which produces 0x774 for param_1==7 and 0x5FF for any other input.

## Cross-References

Control IDs returned by this function appear in the team-enable helper
FUN_006ACD60 (task 17) and the WM_COMMAND dispatcher FUN_006ACEE0 (task 2)
for CBN_SELCHANGE on team combos.

## Callers (in scope)

  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)
  FUN_006ADC20 (0x006ADC20) - row enable state machine (task 3)
  FUN_006ACD60 (0x006ACD60) - team-enable helper (task 17)
  FUN_006ADDF0 (0x006ADDF0) - row show/hide adjuster (task 14)
  FUN_006ADF00 (0x006ADF00) - reveal-AI-rows (task 15)
  FUN_006AE080 (0x006AE080) - hide-AI-rows (task 16)
  FUN_006AE6E0 (0x006AE6E0) - dialog init (task 1)
  FUN_006ACE80 (0x006ACE80) - in-scope team helper

Out-of-scope callers: 15 additional functions in wider game code.

(Confirmed via get_function_callers 0x004E5940)

## Out-of-scope refs

None -- pure lookup, no callees.

## TS-filter

All primary in-scope callers are YR offline Skirmish dialog functions.
No TS-only gate. TS-legacy score: 0.0.

## Unverified (YELLOW)

None. All claims including the out-of-range formula result are verified by step-by-step
bit arithmetic from `decompile_function 0x004E5940`. The function is only called with
valid inputs 0–7 in all in-scope callers; the 0x5FF out-of-range result is never
exercised in a normal YR skirmish.
