# STT:SkirmishComboColor — Color Combo Tooltip String

## Summary

Static ASCII string at `0x00835434` in the `gamemd.exe` data segment.
Key: `STT:SkirmishComboColor`. Used as the tooltip identifier for all 9 color
combo controls in the Skirmish dialog (0x102). The sole consumer is the global
tooltip dispatcher `FUN_006040B0`, which returns a pointer to this string
whenever a color combo control is hovered and tooltip lookup is requested.

## Address

`0x00835434` (verified via `read_memory 0x00835434`)

## Active in YR

**Yes.** Referenced from `FUN_006040B0` (0x006040B0), the tooltip dispatcher
called from the shared WM_NOTIFY handler of all YR dialogs. Active whenever
the Skirmish dialog (0x102) is open and the player hovers over a color combo.

(confirmed via `get_xrefs_to 0x00835434`)

## String Content

```
STT:SkirmishComboColor
```

22 bytes of ASCII text + null terminator (byte 0x00) = 23 bytes total.
Stored with 1-byte padding to 2-byte alignment (24 bytes on-disk).
Immediately followed at `0x0083544C` by `STT:SkirmishButtonBack`.

(verified via `read_memory 0x00835434` length=64)

```
hex:  5354543a536b69726d697368436f6d626f436f6c6f720000
text: S T T : S k i r m i s h C o m b o C o l o r \0 \0
```

## Cross-Reference Analysis

| From address | Function | Type | Role |
|---|---|---|---|
| 0x006045A3 | FUN_006040B0 | DATA | `MOV EAX, 0x00835434` return value for color combo controls |

Sole xref is the tooltip dispatcher. The instruction at 0x006045A3 is:
```
b8 34 54 83 00   MOV EAX, 0x00835434
5e               POP ESI
59               POP ECX
c3               RET
```
This is the terminal return path in `FUN_006040B0` when dialog ID is `0x102`
and control ID matches any of the 9 color combos (0x6A2, 0x522..0x528).

(verified via `read_memory 0x006045A3` length=8)

## Consumer: FUN_006040B0 Dispatch Logic

The tooltip dispatcher for dialog 0x102 maps the following control IDs to
this string (from the decoded tooltip dispatcher doc, task #51):

| Control ID | Slot |
|---|---|
| 0x6A2 | Slot 0 (player row) |
| 0x522 | Slot 1 |
| 0x523 | Slot 2 |
| 0x524 | Slot 3 |
| 0x525 | Slot 4 |
| 0x526 | Slot 5 |
| 0x527 | Slot 6 |
| 0x528 | Slot 7 |

9 color combos total map to a single shared tooltip string. There is no
per-slot variation in the tooltip text.

## Globals Referenced

None — static string embedded in the binary data segment.

## TS-filter

No TS-only gate. `FUN_006040B0` is the live YR tooltip dispatcher.
**TS-legacy score: 0.0.**

## Unverified (YELLOW)

- Actual tooltip display text: the `STT:SkirmishComboColor` key refers to an
  entry in `GDlgSupp.csf`. The localized display string (e.g., "Select a color
  for this player") was not read from the CSF file in this session. Only the
  key name is verified from binary.
- Padding byte: the second null at offset +23 (0x00835447) is consistent with
  2-byte alignment padding; the exact alignment requirement was not independently
  confirmed from surrounding string storage patterns.
