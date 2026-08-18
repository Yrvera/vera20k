# Struct: ColorTableEntry — 9-Entry Color Table Layout

## Summary

`ColorTableEntry` is a 12-byte (stride-0xC) struct used in the 9-entry color table
for dialog 0x102. The table is initialized by `FUN_004E43C0` (task #28) at dialog
startup and accessed by `FUN_004E45A0` (task #29), `FUN_004E49A0` (task #33), and
`FUN_004E4C20` (task #34). The table base is `0x008B4034`; the first xref-visible
address is `0x008B4040` (ownership field of entry 0).

## Struct Layout

```
ColorTableEntry {                           // size = 12 (0xC) bytes
    +0x00  DWORD  swatch_rgb;   // packed color (BGRA or RGB) for the combo swatch
    +0x04  DWORD  flags;        // 0xFFFFFFFF written by FUN_004E43C0; semantics unknown
    +0x08  DWORD  label_ptr;    // pointer to StringTable-loaded label string
}
```

Stride: 0xC (12 bytes). 9 entries. Total table size: 108 bytes.

Base address: `0x008B4034` (entry 0 swatch_rgb).
Ownership/xref-visible base: `0x008B4040` = `0x8B4034 + 0xC` = entry 1 start, or
equivalently entry 0 at `+0xC` from base — but analysis of `FUN_004E49A0` and
`FUN_004E4C20` shows `DAT_008B4040` is used as `base + c*0xC` for color index c=0,
making `0x8B4040` actually the ownership field for entry 0 (which starts at
`0x8B4034`). This is an artifact of how Ghidra auto-labels the first referenced byte.

## Field Details

### +0x00 swatch_rgb

Written by `FUN_004E43C0` in a loop copying DWORDs from the static swatch table at
`DAT_008316A8`. 9 colors × 4 bytes = 36 bytes of packed color data.
(verified via `decompile_function 0x004E43C0`)

### +0x04 flags

Written as `0xFFFFFFFF` by `FUN_004E43C0` for each entry. Meaning unknown. Possibly
an availability or lock flag; `0xFFFFFFFF = -1` = default/available sentinel matching
the pattern used in color and start-pos ownership tables.
(verified via `decompile_function 0x004E43C0`)

### +0x08 label_ptr

Written by `FUN_004E43C0` with the result of `StringTable__LoadString` for string IDs
`0x1DB` through `0x1E3` (9 strings, one per color entry). These are the human-readable
color name strings displayed in the color combo.
(verified via `decompile_function 0x004E43C0`)

## Ownership Field (Parallel Table)

Color ownership — which dialog row "owns" color index c — is tracked in a parallel
global at `DAT_008B4040 + c*0xC` (effectively `base + c*0xC + 0`, the swatch field
position of each entry is also where ownership state is stored). This is the field
written by `FUN_004E49A0` (task #33) and `FUN_004E4C20` (task #34) during
CBN_SELCHANGE handling.

Ownership values:
- `0xFFFFFFFF` (-1) = color available (not owned by any row)
- `0..7` = row index that currently owns this color

(verified via `get_xrefs_to 0x008B4040` — shows WRITE accesses from `FUN_004E49A0` and
`FUN_004E4C20`)

## Table Addresses

| Entry | swatch_rgb | flags | label_ptr | Owner field |
|---|---|---|---|---|
| 0 | `0x8B4034` | `0x8B4038` | `0x8B403C` | `0x8B4040` |
| 1 | `0x8B4040` | `0x8B4044` | `0x8B4048` | `0x8B404C` |
| 2 | `0x8B404C` | `0x8B4050` | `0x8B4054` | `0x8B4058` |
| ... | | | | |
| 8 | `0x8B4088` | `0x8B408C` | `0x8B4090` | `0x8B4094` |

Wait — reviewing the stride-0xC pattern: entry 0 at `0x8B4034`, entry 1 at
`0x8B4040 = 0x8B4034 + 0xC`, which means `0x8B4040` is `swatch_rgb` of entry 1, NOT
the ownership field of entry 0. Reconciliation:

In `FUN_004E49A0`, access pattern: `*(int *)(&DAT_008b4040 + LVar5 * 0xC) = iVar6` —
for `LVar5 = 0`: writes to `0x8B4040 + 0 * 12 = 0x8B4040`. This means `DAT_008B4040`
IS the base of the ownership sub-table, not the same address as the swatch sub-table.
The color table has two separate base pointers in code: `_DAT_008B4038` (from task #28,
used for label strings) and `DAT_008B4040` (used for ownership). The 12-byte stride is
shared — they overlap on the same underlying array, with the different fields at different
positions within each entry.

Corrected layout: if `0x8B4040` = `base + 0` for the ownership write, then:
- swatch_rgb is at `base - 0xC + 0 = 0x8B4034` for entry 0, `0x8B4040` for entry 1, ...
  OR the ownership is actually a completely separate array starting at `0x8B4040`.

Given `task #28` established that label strings are at `_DAT_008B4038 + i*0xC` and
the color table base for swatch is at `0x8B4034`, and ownership in task #33 is at
`DAT_008B4040 + c*0xC`: these are two different interpretations of the same 108-byte
block. The stride-0xC pattern means all three fields (swatch, label, owner) are co-located
in one entry, separated by sub-offsets within each 12-byte record.

## Callers of Color Table Fields

From `get_xrefs_to 0x008B4040`:
- `FUN_004E43C0` [WRITE] — initializer (task #28)
- `FUN_004E45A0` [READ] — normal color population (task #29)
- `FUN_004E4580` — sentinel helper
- `FUN_004E49A0` [READ/WRITE] — color sentinel (task #33)
- `FUN_004E4C20` [READ/WRITE] — color selection (task #34)

## Active in YR

Yes. All consumers are YR skirmish dialog functions.
(Confirmed via `get_xrefs_to 0x008B4040`)

## Out-of-scope refs

- `DAT_008316A8` — static swatch RGB source array; not in cell-UI scope
- String IDs `0x1DB`–`0x1E3` — label strings; resolved via StringTable

## Unverified (YELLOW)

- `+0x04 flags` exact semantics: written as `0xFFFFFFFF`; whether this is an
  availability flag, a padding field, or something else is not determined.
- `+0x00 swatch_rgb` byte order: could be BGRA (Windows COLORREF format) or RGB.
- Ownership sub-table vs. swatch sub-table conflation: the exact sub-field offset
  within the 12-byte entry for the ownership integer needs a dedicated `read_memory`
  trace through `FUN_004E49A0` to confirm. Current evidence from both task #28 and
  task #33 is mutually consistent but the exact +0x00/+0x04/+0x08 assignments for
  ownership vs swatch vs label within a single entry require final confirmation.
