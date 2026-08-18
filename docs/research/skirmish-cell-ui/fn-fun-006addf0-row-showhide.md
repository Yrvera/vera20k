# FUN_006ADDF0 — Row Show/Hide Adjuster on Selected-Map Change

## Summary

Adjusts the number of visible AI player rows in dialog 0x102 whenever the selected
map changes. Compares the old start-count (param_2) and the new start-count
(param_3) — or fetches the current count via `CDFileClass__Constructor` (actually
FUN_005E6520, the map start-count function mislabeled by Ghidra's RTTI labeler) if
param_2 is –1. The difference drives whether rows are revealed (`FUN_006ADF00`) or
hidden (`FUN_006AE080`). Also propagates AlliesAllowed team-sentinel across all 7
rows on every call when the dialog HWND and SelectedMode pointer are non-null.

## Address

`0x006ADDF0` (verified via `decompile_function 0x006ADDF0`)

## Active in YR

**Yes.** Callers confirmed via `get_function_callers 0x006ADDF0`:
- `FUN_006ACEE0` (0x006ACEE0) — WM_COMMAND dispatcher, called from the 0x5AA
  map-selector change path
- `FUN_006AE6E0` (0x006AE6E0) — dialog 0x102 custom init (msg 0x497)

Both callers are YR-active anchor functions. No TS-gating flag.

## Signature / Parameters

```c
void __fastcall FUN_006addf0(
    int param_1,   // dialog HWND (or 0 to skip team-sentinel loop)
    int param_2,   // old map start count, or -1 to fetch current
    int param_3    // new map start count
)
```

## Behavioral Analysis

### Step 1 — Determine row-delta

```c
if (param_2 == -1) {
    local_4 = 8;                            // assume 8 (all rows shown)
} else {
    local_4 = CDFileClass__Constructor();   // = FUN_005E6520: old start count
}
iVar3 = CDFileClass__Constructor();         // = FUN_005E6520: new start count
local_4 = iVar3 - local_4;                 // delta = new - old
```

When `param_2 == -1`, the old count is forced to 8, making `delta = new - 8`
(always ≤ 0 for normal maps with ≤ 8 start positions — hides all excess rows
relative to the true new count). Otherwise both old and new counts are read from
the map start-count function (address 0x005E6520, mislabeled CDFileClass__Constructor
by Ghidra per manifest note for task #59).

(verified via `decompile_function 0x006ADDF0`)

### Step 2 — Optional fast-path for map transitions

```c
if (-1 < param_2 && -1 < param_3 &&
    param_2 < DAT_00A8B8D8 && param_3 < DAT_00A8B8D8)
{
    cVar1 = FUN_0069ADF0();   // session validity check
    cVar2 = FUN_0069ADF0();
    if (cVar1 != '\0' && cVar2 != '\0') {
        FUN_006AE080();               // hide rows
        FUN_006ADF00(iVar3 - 1);      // reveal new_count - 1 rows
    }
}
```

When both old and new counts are valid (non-negative and below `DAT_00A8B8D8`),
and `FUN_0069ADF0()` returns non-zero twice (session is ready), the function
takes a fast-path: hides all rows then reveals exactly `new_count - 1` rows.
`DAT_00A8B8D8` is an upper-bound on valid start-count values (map max players).

### Step 3 — Delta-based show/hide

```c
if (local_4 < 1) {
    if (local_4 < 0) {
        FUN_006AE080();           // count shrank: hide rows
    }
    // delta == 0: no change
} else {
    FUN_006ADF00(local_4);        // count grew: reveal |delta| rows
}
```

`FUN_006ADF00` (task #15) reveals rows; `FUN_006AE080` (task #16) hides rows.
The delta is the magnitude of the change. When `delta == 0`, nothing happens.

### Step 4 — AlliesAllowed team-sentinel propagation

```c
if (param_1 != 0 && DAT_00A8B23C != 0) {
    for (iVar3 = 1; iVar3 < 8; iVar3++) {
        FUN_004E5940();                         // set team row context
        if (DAT_00A8B23C == 0 ||
            *(char *)(DAT_00A8B23C + 0x3C) == '\0') {
            uVar4 = 0xFFFFFFFE;                 // -2: AlliesAllowed false
        } else {
            uVar4 = 3;                          // 3: AlliesAllowed true
        }
        FUN_004E5ED0(uVar4);                    // write team sentinel
    }
}
```

Identical AlliesAllowed loop as in `FUN_006ADC20` (task #3). Runs across all 7
rows (iVar3 = 1..7). `*(char *)(DAT_00A8B23C + 0x3C)` = AlliesAllowed byte on
the SelectedMode object (confirmed via `get_xrefs_to 0x00A8B23C` — consistent
with fn-006adc20-row-enable.md findings).

## Callees

Confirmed via `get_function_callees 0x006ADDF0`:
- `CDFileClass__Constructor @ 0x005E6520` — map start-count function (mislabeled;
  see task #59 for full decode)
- `FUN_004E5940` (0x004E5940) — team row context setter (task #44)
- `FUN_004E5ED0` (0x004E5ED0) — team sentinel writer (task #48)
- `FUN_0069ADF0` (0x0069ADF0) — session validity check (task #54)
- `FUN_006ADF00` (0x006ADF00) — reveal AI rows (task #15)
- `FUN_006AE080` (0x006AE080) — hide AI rows (task #16)

## Out-of-scope refs

- `DAT_00A8B8D8` — map-max-players upper bound; not decoded in this scope
- `FUN_0069ADF0` (0x0069ADF0) — session validity function; task #54 in scope
- `FUN_006ADF00`, `FUN_006AE080` — row reveal/hide; tasks #15, #16 in scope
- `FUN_004E5940`, `FUN_004E5ED0` — team helpers; tasks #44, #48 in scope

## Unverified (YELLOW)

- The exact semantics of calling `FUN_0069ADF0()` twice in the fast-path
  (same function called back-to-back with no visible argument difference in the
  decompilation — Ghidra may have collapsed two distinct calls or the function
  has side-effects producing different results on the second call) is not
  confirmed here. To be resolved by task #54 decoder.
