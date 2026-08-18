# FUN_004E43C0 — Color Table Label and Swatch Initializer

## Summary

One-time initializer called from dialog init (`FUN_006AE6E0`). Loads 9 color-label
strings (string IDs 0x1DB–0x1E3) from `GDlgSupp_cpp` string table into a global
array at stride-12 positions starting at `_DAT_008B4038`. Then copies 9 swatch RGB
values from the read-only data block `DAT_008316A8..0x8316CB` into the parallel
array slots at `DAT_008B4040`-based positions (offset −4 from each string pointer
slot), writing `0xFFFFFFFF` as a terminator/flags word at each slot's `+0x0` offset.
Sole caller is `FUN_006AE6E0`.

## Address

`0x004E43C0` (verified via `decompile_function 0x004E43C0`)

## Active in YR

**Yes.** Sole caller is `FUN_006AE6E0` (0x006AE6E0, dialog init, YR-active anchor).
(Callers confirmed via `get_function_callers 0x004E43C0`)

## Signature / Parameters

```c
void FUN_004e43c0(void)
// no parameters, no return value
```

(verified via `decompile_function 0x004E43C0`)

## Behavioral Analysis

### Step 1 — Load 9 color label strings

```c
_DAT_008b4038 = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1db);
_DAT_008b4044 = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1dc);
_DAT_008b4050 = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1dd);
_DAT_008b405c = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1de);
_DAT_008b4068 = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1df);
_DAT_008b4074 = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1e0);
_DAT_008b4080 = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1e1);
_DAT_008b408c = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1e2);
_DAT_008b4098 = StringTable__LoadString(s_D__ra2mdpost_GDlgSupp_cpp_008229c0, 0x1e3);
```

String IDs 0x1DB–0x1E3 = 9 consecutive entries, one per color slot. Each is loaded
from the `GDlgSupp_cpp` string table (same source used for country names, dialog
labels, etc.). The label string pointer is stored at every 12th byte (stride 0xC)
starting at `0x008B4038`, i.e.: 0x8B4038, 0x8B4044, 0x8B4050, ..., 0x8B4098.
(verified via `decompile_function 0x004E43C0`)

### Step 2 — Copy swatch RGB data loop

```c
puVar3 = &DAT_008b4040;  // array base at 0x8B4040
puVar2 = &DAT_008316a8;  // source RGB data at 0x8316A8
do {
    uVar1 = *puVar2;
    puVar2++;
    puVar3[-1] = uVar1;          // write RGB at stride slot − 4 bytes
    *puVar3 = 0xffffffff;        // write 0xFFFFFFFF at slot + 0
    puVar3 += 3;                 // advance by 3 DWORDs = 12 bytes
} while ((int)puVar2 < 0x8316cc);
```

The source range `0x8316A8..0x8316CB` is 36 bytes = 9 × 4-byte entries (one DWORD
per color slot). Each DWORD is a packed RGB swatch. The destination layout places
the swatch DWORD at `base + index*12 - 4` and `0xFFFFFFFF` at `base + index*12`.

Concretely, the color table entry at index `i` occupies bytes at:
- `0x8B4038 + i*0xC`: label string pointer (set in Step 1)
- `0x8B4038 + i*0xC - 4` = `0x8B4034 + i*0xC`: swatch RGB DWORD (set in Step 2)
- `0x8B4038 + i*0xC - 0` = `0x8B4038 + i*0xC`: 0xFFFFFFFF flags/terminator word

Since Ghidra uses `_DAT_008b4038` (with overlap-warning prefix `_`) for the string
pointer slot and `DAT_008b4040` as the loop base, the effective array base for the
struct is `0x8B4034` (swatch at `+0`, flags at `+4`, string ptr at `+8`), stride 0xC.
(verified via `decompile_function 0x004E43C0`)

### Color table entry struct (inferred, stride 0xC)

| Offset | Size | Content |
|---|---|---|
| +0x0 | 4 | Swatch RGB DWORD (from `0x8316A8` source) |
| +0x4 | 4 | 0xFFFFFFFF (flags/terminator written by loop) |
| +0x8 | 4 | Label string pointer (`StringTable__LoadString` result) |

Array base: `0x008B4034`. 9 entries (i = 0..8).

This struct layout is the subject of task #66 (`decode-struct-colortableentry`).

## String ID → Color Label Mapping

| String ID | Array index | Global slot |
|---|---|---|
| 0x1DB | 0 | `_DAT_008B4038` |
| 0x1DC | 1 | `_DAT_008B4044` |
| 0x1DD | 2 | `_DAT_008B4050` |
| 0x1DE | 3 | `_DAT_008B405C` |
| 0x1DF | 4 | `_DAT_008B4068` |
| 0x1E0 | 5 | `_DAT_008B4074` |
| 0x1E1 | 6 | `_DAT_008B4080` |
| 0x1E2 | 7 | `_DAT_008B408C` |
| 0x1E3 | 8 | `_DAT_008B4098` |

The actual color names (e.g. "Yellow", "Red", "Blue", ...) are in the string table
at the given IDs; they are not resolved by this function.

## Globals Accessed

| Global | Address | Access | Role |
|---|---|---|---|
| `_DAT_008B4038`..`_DAT_008B4098` | `0x8B4038` + stride | WRITE | Color label string pointers |
| `DAT_008B4040` | `0x8B4040` | WRITE | Swatch/flags array base (loop pointer) |
| `DAT_008316A8` | `0x8316A8` | READ | Source swatch RGB data (9 × 4 bytes) |
| `s_D__ra2mdpost_GDlgSupp_cpp_008229c0` | `0x8229C0` | READ | String table source label |

(confirmed via `decompile_function 0x004E43C0`)

## Callers

- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init (anchor task #1, sole caller)

(confirmed via `get_function_callers 0x004E43C0`)

## Callees

- `StringTable__LoadString` (9 calls) — loads a string by ID from the named table
- No other callees (confirmed: no in-scope helpers)

## Out-of-scope refs

- `StringTable__LoadString` — string table system; not in cell-UI scope
- `DAT_008316A8` — read-only swatch RGB source data; analyzed under task #66
- Struct layout at `0x8B4034` — covered by task #66 (`decode-struct-colortableentry`)
- String IDs 0x1DB–0x1E3 — covered by task #63 (`decode-string-stt-skirmishcombocolor`)

## Unverified (YELLOW)

- Actual color names for string IDs 0x1DB–0x1E3: not read from the string table in
  this session. Expected to be the 9 YR player colors (e.g. Yellow, Red, Blue, etc.)
  based on context; unverified.
- Swatch RGB values at `0x8316A8..0x8316CB`: not read via `read_memory` in this
  session. Content is inferred as 9 × 4-byte packed RGB values from the loop stride.
- `0xFFFFFFFF` at `puVar3[0]`: written unconditionally in the loop; inferred as a
  flags/sentinel field for the color-entry struct. Actual purpose not verified.
