# Global: ColorTableBase — 9-Entry Color Table

## Summary

The color table for dialog 0x102 is a 9-entry, stride-0xC (12-byte) array of
`ColorTableEntry` structs. The "base" address referenced in code is `DAT_008B4040`
— the first entry's ownership field at base+0x0C, which Ghidra sees as a separate
symbol. The actual table starts at `0x008B4034` (verified by prior work on task #28:
`FUN_004E43C0` writes to `_DAT_008B4038 + stride*i` patterns).

However, the first codegen-visible address is `0x008B4040`, which is `0x8B4034 + 0xC`
= the ownership (row-index) field of entry 0.

## Confirmed Addresses

| Symbol | Address | Content at startup |
|---|---|---|
| (inferred base) | `0x008B4034` | Entry 0 swatch RGB (DWORD) |
| (label string ptr) | `0x008B4038` | Entry 0 label string pointer |
| `DAT_008B4040` | `0x008B4040` | Entry 0 owner row index |
| Entry 1 start | `0x008B4040` | Entry 1 swatch RGB |

Note: All values are 0 at static load time; the table is populated at runtime by
`FUN_004E43C0` (task #28).
(Confirmed via `read_memory 0x008B4034` — all zeros at static image; table layout
confirmed via `get_xrefs_to 0x008B4040`)

## Entry Layout (stride 0xC = 12 bytes)

From `FUN_004E43C0` (task #28) and `FUN_004E49A0`/`FUN_004E4C20` access patterns
confirmed via `get_xrefs_to 0x008B4040`:

```
ColorTableEntry {
    +0x00  DWORD  swatch_rgb;   // packed BGRA or RGB color; source: DAT_008316A8..0x8316CB
    +0x04  DWORD  flags;        // written as 0xFFFFFFFF by FUN_004E43C0
    +0x08  DWORD  label_ptr;    // StringTable result pointer; written by FUN_004E43C0
    // stride ends at +0x0C; next entry begins here
}
```

Total table size: 9 entries × 12 bytes = 108 bytes, from `0x008B4034` to `0x008B408F`.

The ownership field (row index that owns this color) is in a parallel table at
`DAT_008B4040` stride 3 dwords — i.e., at `0x008B4040 + c*12 + 0` for color index `c`.
This field is written by `FUN_004E49A0` (task #33, color sentinel) and `FUN_004E4C20`
(task #34, color selection).

## Code References

From `get_xrefs_to 0x008B4040`:
- `FUN_004E43C0 @ 0x004E44B3` [DATA], `0x004E44C5` [WRITE] — label loader (task #28)
- `FUN_004E45A0 @ 0x004E46BD` [DATA], `0x004E46CA` [READ] — color population (task #29)
- `FUN_004E4580 @ 0x004E4585` [DATA] — color sentinel helper
- `FUN_004E49A0 @ 0x004E4A43` [DATA], `0x004E4A48` [READ], `0x004E4B47` [DATA] — color sentinel (task #33)
- `FUN_004E4C20 @ 0x004E4C9F` [DATA], `0x004E4CA4` [READ], `0x004E4D67` [DATA] — color selection (task #34)

(Confirmed via `get_xrefs_to 0x008B4040`)

## Active in YR

Yes. All xref consumers are YR skirmish dialog functions.

## Out-of-scope refs

- `DAT_008316A8` — swatch RGB source array; RGB color definitions for the 9 colors;
  not decoded in cell-UI scope
- String IDs 0x1DB–0x1E3 — label strings loaded in task #28; part of `ra2md.csf`

## Unverified (YELLOW)

- `+0x04` field semantics: written as `0xFFFFFFFF` by `FUN_004E43C0` — inferred as
  a flags/sentinel word; exact meaning unknown.
- Whether the swatch is BGRA or RGB: not confirmed by reading the color values.
- Whether entry 0 at `0x8B4034` has the same three-field layout or a different layout:
  all evidence from xrefs points to `0x8B4040` as the base of the first-seen entry,
  suggesting the actual swatch+label data for entry 0 is at `0x8B4034`/`0x8B4038`
  (no direct Ghidra xrefs to those specific addresses were found, so the layout at
  `+0x00` and `+0x04` within entry 0 is inferred from the stride-0xC pattern in
  `FUN_004E43C0`).
