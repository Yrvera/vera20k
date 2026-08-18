# FUN_0069ADF0 - Session RandMap_Sed String Validator

## Summary

Returns true if the string at session object offset +0x58 equals "RandMap_Sed".
Calls FUN_007C8D20 (a widely-used string comparison utility) with the session
field and the literal "RandMap_Sed". Returns (result == 0), i.e., true when
strings match, false when they don't.

Used by the skirmish dialog to validate whether the session object represents
a random-map seed session. Called from the WM_COMMAND dispatcher to gate
certain dialog behavior on this condition.

## Address

0x0069ADF0 (verified via decompile_function 0x0069ADF0)

## Active in YR

Yes. In-scope callers include FUN_006ACEE0 (0x006ACEE0, WM_COMMAND dispatcher,
task 2), FUN_006ADDF0 (0x006ADDF0, row-showhide, task 14), and FUN_006AE6E0
(0x006AE6E0, dialog init, task 1).
(Confirmed via get_function_callers 0x0069ADF0)

## Signature / Parameters

bool __fastcall FUN_0069adf0(int param_1)
  param_1 = pointer to session object (or a sub-struct within it)
  returns: true if session_obj+0x58 == "RandMap_Sed", false otherwise

(verified via decompile_function 0x0069ADF0)

## Behavioral Analysis

```c
iVar1 = FUN_007c8d20(param_1 + 0x58, s_RandMap_Sed_0082bc30);
return iVar1 == 0;
```

(verified via decompile_function 0x0069ADF0)

FUN_007C8D20 at 0x007C8D20 is a general-purpose string comparison utility used
by 100+ functions across the binary (confirmed via get_function_callers 0x007C8D20).
The call pattern (char*, literal-string) and return-value comparison to 0 is
consistent with strcmp semantics. Returns 0 when strings are equal.

The literal "RandMap_Sed" at 0x0082BC30 (s_RandMap_Sed_0082bc30 Ghidra label) is
the identifier for the random map seed entry in the session object.

Session object layout: field at +0x58 is a string field (char array or char*).
At offset +0x58 from the session base, the game stores the "RandMap_Sed" token
when a random-map seed is active.

## Globals Accessed

  s_RandMap_Sed_0082bc30 (0x0082BC30) - Literal string "RandMap_Sed"

## Callees

Confirmed via get_function_callees 0x0069ADF0:
  FUN_007C8D20 (0x007C8D20) - String comparison utility (strcmp-like)

## Callers (in scope)

  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)
  FUN_006ADDF0 (0x006ADDF0) - row show/hide adjuster (task 14)
  FUN_006AE6E0 (0x006AE6E0) - dialog init (task 1)

Out-of-scope callers: FUN_005D63E0, FUN_005E8590, FUN_005ED370, FUN_005ED5A0.
(Confirmed via get_function_callers 0x0069ADF0)

## Out-of-scope refs

  FUN_007C8D20 - string comparison utility; not in cell-UI scope
  Session object +0x58 field - full layout covered by task 65

## Unverified (YELLOW)

  FUN_007C8D20 semantics: inferred as strcmp-like from usage pattern
  (char* + literal, compare to 0); not independently decompiled in this task.
  Session object +0x58 as string field: inferred from the string comparison
  call; full session layout not decoded here (task 65).
