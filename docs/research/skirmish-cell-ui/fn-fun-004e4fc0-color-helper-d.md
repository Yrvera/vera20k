# FUN_004E4FC0 — Start-Position Availability Mask Setter

## Summary

Writes availability masks into the start-position table at `DAT_008B3F38`
(stride-3-DWORD array base) based on a threshold parameter. For entries at
indices `0..param_1-1`, writes `0xFFFFFFFF` (unavailable/claimed sentinel).
For entries at indices `param_1..8`, writes `0xFFFFFFFE` (available sentinel).
Used to restrict the available start-position choices to those at index
≥ `param_1` when the map has fewer than 9 valid start positions.

## Address

`0x004E4FC0` (verified via `decompile_function 0x004E4FC0`)

## Active in YR

**Yes.** In-scope callers include `FUN_006ACEE0` (0x006ACEE0, WM_COMMAND dispatcher,
YR-active anchor) and `FUN_006AE6E0` (0x006AE6E0, dialog init, YR-active anchor).
(Callers confirmed via `get_function_callers 0x004E4FC0`)

## Signature / Parameters

```c
void __fastcall FUN_004e4fc0(int param_1)
// param_1 = first valid start-position index (entries 0..param_1-1 are unavailable)
```

(verified via `decompile_function 0x004E4FC0`)

## Behavioral Analysis

### Full decompile

```c
void __fastcall FUN_004e4fc0(int param_1)
{
    undefined4 *puVar1;
    int iVar2;

    // Part 1: mark entries 0..param_1-1 as unavailable (0xFFFFFFFF)
    if (0 < param_1) {
        puVar1 = &DAT_008b3f38;   // table base (ownership fields, stride 3 DWORDs)
        iVar2 = param_1;
        do {
            *puVar1 = 0xffffffff;
            puVar1 += 3;
            iVar2--;
        } while (iVar2 != 0);
    }

    // Part 2: mark entries param_1..8 as available (0xFFFFFFFE)
    if (param_1 < 9) {
        puVar1 = &DAT_008b3f38 + param_1 * 3;  // start at entry param_1
        do {
            *puVar1 = 0xfffffffe;
            puVar1 += 3;
        } while ((int)puVar1 < 0x8b3fa4);
    }
}
```

(verified via `decompile_function 0x004E4FC0`)

### Sentinel values

| Value | Meaning |
|---|---|
| `0xFFFFFFFF` | Entry is unavailable (index below current map start-count) |
| `0xFFFFFFFE` | Entry is available for selection |

These are written to the ownership/availability field at `[entry * 3 + 0]` in the
start-position table (i.e., `DAT_008B3F38 + entry * 12`). This field is at offset
+8 relative to the struct base at `0x8B3F30` (see `FUN_004E4F50`, task #37).

When a player selects start position `c`, a row-index value (0–7) is written to
`[c * 3 + 0]` — overwriting the `0xFFFFFFFE` available sentinel with the actual
claiming row. `0xFFFFFFFF` means the entry is off-limits due to map topology.

### Relationship to map start-count

`param_1` is the map's minimum valid start-position index. Maps with N valid start
positions (counted from index 0) use `param_1 = 0`; maps where start positions begin
at a higher index use `param_1 > 0`. In practice the map start-count returned by
`FUN_005E6520` (task #59, mislabeled `CDFileClass__Constructor` at `005E3D10`) drives
this value.

## Globals Accessed

| Global | Address | Access | Role |
|---|---|---|---|
| `DAT_008B3F38` | `0x8B3F38` | WRITE | Start-position ownership/availability table base |

(confirmed via `decompile_function 0x004E4FC0`)

## Callers

In-scope YR-active callers:
- `FUN_006ACEE0` @ `0x006ACEE0` — WM_COMMAND dispatcher (anchor task #2)
- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init (anchor task #1)

Out-of-scope caller: `CDFileClass__Constructor @ 005E3D10` (mislabeled; actual role
is map start-count helper, task #59).

(confirmed via `get_function_callers 0x004E4FC0`)

## Callees

None — pure table write.

## Out-of-scope refs

- `DAT_008B3F38` table full layout — covered by task #37 (`FUN_004E4F50`)

## Unverified (YELLOW)

- `param_1` semantics as "first valid start-position index": inferred from the split
  point between `0xFFFFFFFF` (unavailable) and `0xFFFFFFFE` (available) writes; not
  verified against the call sites in `FUN_006AE6E0` or `FUN_006ACEE0`.
- Whether `0xFFFFFFFE` is specifically an "available" sentinel vs. a general
  "not yet claimed" marker: distinguished from `0xFFFFFFFF` (hard-unavailable) only
  by value; the consumer of this field is in `FUN_004E5310` or `FUN_004E53D0` (tasks
  #39/#40), not verified here.
