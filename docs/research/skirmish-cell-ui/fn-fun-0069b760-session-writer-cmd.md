# FUN_0069B760 - Session Country/Side Selection Writer

## Summary

__thiscall method: writes a country/side selection into a session object. When
param_2 is -2 (random):
  - If param_3 == 0: stores 0 as country + -2 (random sentinel) as mode
  - If param_3 != 0: picks a random country index 0-9 via Random__RandomRanged
    and stores it + -2 (random sentinel)
Otherwise: stores param_2 as country + -1 (specific/fixed selection).

After either branch, mirrors the result into a second pair of fields.

Used by the WM_COMMAND dispatcher to commit a country change from the UI into
the session object.

## Address

0x0069B760 (verified via decompile_function 0x0069B760)

## Active in YR

Yes. In-scope caller is FUN_006ACEE0 (0x006ACEE0, WM_COMMAND dispatcher, task 2).
(Confirmed via get_function_callers 0x0069B760)

## Signature / Parameters

void __thiscall FUN_0069b760(int param_1, int param_2, char param_3)
  param_1 = this pointer (session object)
  param_2 = country item-data (-2 = random, 0-9 = specific country)
  param_3 = randomize flag (non-zero = pick random; zero = clear random)

(verified via decompile_function 0x0069B760)

## Behavioral Analysis

```c
if (param_2 == -2) {
    if (param_3 == 0) {
        *(param_1 + 0x184) = 0;            // country = 0 (no random pick)
        *(param_1 + 0x188) = 0xFFFFFFFE;   // mode = -2 (random sentinel)
    } else {
        uVar1 = Random__RandomRanged(0, 9); // pick random 0-9
        *(param_1 + 0x184) = uVar1;         // country = random result
        *(param_1 + 0x188) = 0xFFFFFFFE;   // mode = -2 (random sentinel)
    }
} else {
    *(param_1 + 0x184) = param_2;           // country = explicit selection
    *(param_1 + 0x188) = 0xFFFFFFFF;        // mode = -1 (fixed)
}
// mirror to secondary fields
*(param_1 + 0x174) = *(param_1 + 0x184);
*(param_1 + 0x178) = *(param_1 + 0x188);
```

(verified via decompile_function 0x0069B760)

### Session object fields

| Offset | Usage |
|--------|-------|
| +0x184 | Country/side index (primary) |
| +0x188 | Mode sentinel: -2 = random, -1 = fixed |
| +0x174 | Country/side index (mirror copy) |
| +0x178 | Mode sentinel (mirror copy) |

The mirror at +0x174/+0x178 is written unconditionally after the branch;
likely used by a different subsystem reading the session (e.g., the
random-assignment phase or the session serializer).

### Sentinel conventions

  -2 (0xFFFFFFFE): random country selection -- actual country stored at +0x184
  -1 (0xFFFFFFFF): fixed/specific country -- param_2 stored at +0x184

These match the -2 random sentinel used throughout the dialog (color combos,
country combos, start-position combos).

## Globals Accessed

None directly -- all accesses are via this pointer (param_1).

## Callees

Confirmed via get_function_callees 0x0069B760:
  Random__RandomRanged (0x0065C7E0) - returns random int in [min, max]

## Callers (in scope)

  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)

Out-of-scope: FUN_005E00B0, FUN_005E9B60,
SimpleWonlineDialogControl__Constructor at 0x007864C0 and 0x00789B60.
(Confirmed via get_function_callers 0x0069B760)

## Out-of-scope refs

  Session object layout at +0x174, +0x178, +0x184, +0x188 -- full struct decode
  covered by task 65 (decode-struct-sessionclass-slots-slice)
  Random__RandomRanged (0x0065C7E0) -- out of cell-UI scope

## Unverified (YELLOW)

  Mirror fields at +0x174/+0x178: the function writes these identically to
  +0x184/+0x188 after every branch; the consumer of the mirror copy is not
  traced in this task.
  Session object layout: offsets +0x174, +0x178, +0x184, +0x188 named from
  decompile; full struct confirmed in task 65.
