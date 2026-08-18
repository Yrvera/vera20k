# FUN_0069B7E0 - Session Color Selection Writer (with Collision Avoidance)

## Summary

__thiscall method: writes a color selection into a session object, parallel to
FUN_0069B760 (country writer, task 56) but with collision-avoidance for the
random case. When param_2 is -2 (random):
  - If param_3 == 0: store 0 + -2 sentinel
  - If param_3 != 0: pick a random color index 0-7, check whether it conflicts
    with an existing slot's color (via session slot list at +0x2840/+0x284C) or
    a used-colors array at +0x84; if conflict, retry until a free color is found
Otherwise: store param_2 + -1 (fixed selection).

Then mirrors the result to a secondary pair of fields.

## Address

0x0069B7E0 (verified via decompile_function 0x0069B7E0)

## Active in YR

Yes. In-scope caller is FUN_006ACEE0 (0x006ACEE0, WM_COMMAND dispatcher, task 2).
(Confirmed via get_function_callers 0x0069B7E0)

## Signature / Parameters

void __thiscall FUN_0069b7e0(int param_1, int param_2, char param_3)
  param_1 = this pointer (session object)
  param_2 = color item-data (-2 = random, 0-7 = specific color index)
  param_3 = randomize flag (non-zero = pick random; zero = clear random)

(verified via decompile_function 0x0069B7E0)

## Behavioral Analysis

### Non-random branch (param_2 != -2)

*(param_1 + 0x17C) = param_2;           // color = explicit selection
*(param_1 + 0x180) = 0xFFFFFFFF;        // mode = -1 (fixed)

(verified via decompile_function 0x0069B7E0)

### Random branch, clear (param_2 == -2, param_3 == 0)

*(param_1 + 0x17C) = 0;                  // color = 0 (no random pick)
*(param_1 + 0x180) = 0xFFFFFFFE;         // mode = -2 (random sentinel)

(verified via decompile_function 0x0069B7E0)

### Random branch, pick (param_2 == -2, param_3 != 0)

Retry loop:
  iVar1 = Random__RandomRanged(0, 7);     // candidate color 0-7
  if (iVar1 != -2) {
      // Check existing slots for conflict
      for each slot in session.slot_list[+0x2840..+0x284C]:
          slot_color = slot[+0x57] == -2 && slot[+0x53] == -1 ? -2 : slot[+0x53]
          if slot_color == iVar1: retry
      // Check used-colors array at +0x84
      for i = 0; i <= 7; i++:
          if session[+0x84 + i*4] == iVar1: retry
  }
  // No conflict: accept iVar1

*(param_1 + 0x17C) = iVar1;              // color = accepted candidate
*(param_1 + 0x180) = 0xFFFFFFFE;         // mode = -2 (random sentinel)

(verified via decompile_function 0x0069B7E0)

### Mirror write (all branches)

*(param_1 + 0x15C) = *(param_1 + 0x17C);
*(param_1 + 0x160) = *(param_1 + 0x180);

(verified via decompile_function 0x0069B7E0)

### Session object fields

| Offset | Usage |
|--------|-------|
| +0x17C | Color index (primary) |
| +0x180 | Mode: -2 = random, -1 = fixed |
| +0x15C | Color index (mirror) |
| +0x160 | Mode (mirror) |
| +0x84  | Used-colors array (8 entries x 4 bytes) |
| +0x2840 | Slot pointer array (for conflict check) |
| +0x284C | Slot count |

### Slot color read pattern

For each slot entry, the color is read as:
  if slot[+0x57] == -2 AND slot[+0x53] == -1: color = -2 (random)
  else: color = slot[+0x53]

This is the same -2 random sentinel convention used throughout the dialog.

## Globals Accessed

None directly -- all accesses via this pointer (param_1).

## Callees

Confirmed via get_function_callees 0x0069B7E0:
  Random__RandomRanged (0x0065C7E0) - returns random int in [0, 7]

## Callers (in scope)

  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)

Out-of-scope: FUN_005E00B0, FUN_005E9C00, FUN_005ED5A0,
SimpleWonlineDialogControl__Constructor at 0x007864C0 and 0x00789B60.
(Confirmed via get_function_callers 0x0069B7E0)

## Structural Parallel

FUN_0069B7E0 is the color-selection analogue of FUN_0069B760 (task 56, country
writer). Both share the same three-case branch structure (fixed / clear-random /
pick-random) and mirror pattern. The color writer adds collision avoidance via
the slot list and used-colors array; the country writer has none.

## Out-of-scope refs

  Session object layout (+0x17C, +0x180, +0x15C, +0x160, +0x84, +0x2840, +0x284C)
  -- covered by task 65
  Slot struct fields +0x53 (color index) and +0x57 (random-mode flag) --
  covered by task 65

## Unverified (YELLOW)

  Slot struct offsets +0x53 and +0x57: inferred as color-index and mode fields
  from the -2 sentinel comparison pattern; not independently verified against
  a struct layout decode.
  Used-colors array at +0x84: 8 entries at stride 4 inferred from loop
  condition (i <= 7); full layout deferred to task 65.
