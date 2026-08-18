# FUN_004E4F50 - Start-Position Label Pointer Table Initializer

## Summary

Initializes the start-position label pointer table at 0x008B3F30. Writes 9
pointer values (one per start-position slot, 0-8) pointing to single-character
ASCII string labels stored in a static data region at 0x00822BA4..0x00822BC4.
Then clears 9 ownership fields in the same table to 0xFFFFFFFF (-1 = unclaimed).

Despite the task label color-helper-c, this function operates on the
start-position label table (at 0x8B3F30), not the color table (at 0x8B4040).
The naming follows the manifest sequential labeling of init helpers called from
the same dialog init path.

Called only during dialog initialization (FUN_006AE6E0, task 1).

## Address

0x004E4F50 (verified via decompile_function 0x004E4F50)

## Active in YR

Yes. Single caller is FUN_006AE6E0 (0x006AE6E0, dialog init, YR-active).
(Confirmed via get_function_callers 0x004E4F50)

## Signature / Parameters

void FUN_004e4f50(void)

No callees -- pure data initialization.
(Confirmed via get_function_callees 0x004E4F50, which returned no callees.)

## Behavioral Analysis

### String pointer assignments

Each of the 9 assignment lines stores a pointer at table_base + i*12:

  DAT_008b3f30 = address of DAT_00822bc4   (slot 0 label)
  DAT_008b3f3c = address of DAT_00822bc0   (slot 1 label)
  DAT_008b3f48 = address of DAT_00822bbc   (slot 2 label)
  DAT_008b3f54 = address of DAT_00822bb8   (slot 3 label)
  DAT_008b3f60 = address of DAT_00822bb4   (slot 4 label)
  DAT_008b3f6c = address of DAT_00822bb0   (slot 5 label)
  DAT_008b3f78 = address of DAT_00822bac   (slot 6 label)
  DAT_008b3f84 = address of DAT_00822ba8   (slot 7 label)
  DAT_008b3f90 = address of DAT_00822ba4   (slot 8 label)

read_memory 0x00822BA4 (40 bytes) shows values 0x30, 0x38, 0x37, 0x36, 0x35,
0x34, 0x33, 0x32, 0x31 at 4-byte intervals -- ASCII 0, 8, 7, 6, 5, 4, 3, 2, 1
stored at descending addresses. Combined with the assignment list: slot 0
receives address 0x822BC4 (value 0x30 = ASCII 0), down to slot 8 receives
address 0x822BA4 (value 0x38 = ASCII 8), giving slot labels 0-8.

(Verified via decompile_function 0x004E4F50 and read_memory 0x00822BA4)

### Ownership clear loop

Loop starting at DAT_008b3f38 (table_base + 8 = ownership field of entry 0):
  do { *puVar1 = 0xffffffff; puVar1 += 3; } while (puVar1 < 0x8b3fa4)

Bounds: 0x8B3F38 to 0x8B3FA4 exclusive, stride 12 bytes.
Iterations: (0x8B3FA4 - 0x8B3F38) / 12 = 0x6C / 12 = 9.

Clears the ownership field at table_base + i*12 + 8 for all 9 entries.

(Verified via decompile_function 0x004E4F50)

### Table layout at 0x8B3F30

Each start-position entry is 3 ints (12 bytes):
  [i*12 + 0] = string label pointer (written by this function)
  [i*12 + 4] = unknown (not written here)
  [i*12 + 8] = row ownership field (cleared to -1 by loop)

9 entries x 12 bytes = 108 bytes total (0x8B3F30..0x8B3F9B).

The table at 0x008B3F30 confirmed all-zero at static analysis time via
read_memory 0x008B3F30 (48 bytes, all 0x00 -- runtime-populated).

## Globals Accessed

Table at 0x8B3F30 confirmed via read_memory 0x008B3F30; string data at
0x822BA4 confirmed via read_memory 0x00822BA4.

  DAT_008B3F30 (0x008B3F30) - Start-position label table, 9 entries, stride 12
  String labels (0x00822BA4..0x00822BC4) - ASCII single-char label strings

## Callees

None. (Confirmed via get_function_callees 0x004E4F50)

## Callers

  FUN_006AE6E0 (0x006AE6E0) -- dialog init (task 1)
  (Confirmed via get_function_callers 0x004E4F50)

## Out-of-scope refs

None -- no callees, single caller is the dialog init anchor.

## Unverified (YELLOW)

- The exact semantic meaning of [i*12 + 4] in each entry (the int between
  the label pointer and the ownership field) is unknown -- not written here.
  May be a selection index or a display attribute.
- String labels at 0x00822BA4..0x00822BC4: byte pattern from read_memory
  shows ASCII 0-8 at 4-byte intervals in descending address order; the
  mapping to slot label i = string i is inferred from the assignment-to-address
  ordering. Not individually verified by reading each string address.
